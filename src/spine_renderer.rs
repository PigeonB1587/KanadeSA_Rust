//! Low-level renderer that draws `rusty_spine` skeletons with macroquad/miniquad.
//!
//! macroquad does not support the vertex attributes (dark color) or the blend modes that Spine
//! requires, so this module issues raw miniquad draw calls through
//! [`macroquad::window::get_internal_gl`]. Textures are still created with macroquad's
//! [`Texture2D`] so macroquad keeps track of their lifetime.

use glam::Mat4;
use macroquad::miniquad::{
    Bindings, BlendFactor, BlendState, BlendValue, BufferId, BufferSource, BufferType, BufferUsage,
    CullFace, Equation, FilterMode, FrontFaceOrder, PassAction, Pipeline, PipelineParams,
    PrimitiveType, RenderingBackend, ShaderSource, TextureId, TextureWrap, UniformsSource,
    VertexAttribute, VertexFormat,
};
use macroquad::prelude::{ImageFormat, Texture2D, get_internal_gl};
use rusty_spine::BlendMode;
use rusty_spine::atlas::{AtlasFilter, AtlasFormat, AtlasWrap};
use rusty_spine::controller::SkeletonCombinedRenderable;

use macroquad::miniquad as mq;

/// Vertex format expected by the Spine shaders.
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    dark_color: [f32; 4],
}

#[repr(C)]
struct Uniforms {
    world: Mat4,
    view: Mat4,
}

mod shader {
    use macroquad::miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const VERTEX: &str = r#"
        #version 100
        attribute vec2 position;
        attribute vec2 uv;
        attribute vec4 color;
        attribute vec4 dark_color;

        uniform mat4 world;
        uniform mat4 view;

        varying lowp vec2 f_texcoord;
        varying lowp vec4 f_color;
        varying lowp vec4 f_dark_color;

        void main() {
            gl_Position = view * world * vec4(position, 0.0, 1.0);
            f_texcoord = uv;
            f_color = color;
            f_dark_color = dark_color;
        }
    "#;

    pub const FRAGMENT: &str = r#"
        #version 100
        varying lowp vec2 f_texcoord;
        varying lowp vec4 f_color;
        varying lowp vec4 f_dark_color;

        uniform sampler2D tex;

        void main() {
            lowp vec4 tex_color = texture2D(tex, f_texcoord);
            gl_FragColor = vec4(
                ((tex_color.a - 1.0) * f_dark_color.a + 1.0 - tex_color.rgb) * f_dark_color.rgb + tex_color.rgb * f_color.rgb,
                tex_color.a * f_color.a
            );
        }
    "#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("world", UniformType::Mat4),
                    UniformDesc::new("view", UniformType::Mat4),
                ],
            },
        }
    }
}

/// CPU-side description of a texture stored on a Spine atlas page.
///
/// The actual GPU texture is loaded lazily the first time a renderable needs it. This keeps
/// memory usage sane for characters whose atlas contains many pages (the bundled character has
/// ten 4096x4096 pages).
#[derive(Debug)]
struct SpineTexture {
    path: String,
    filter: FilterMode,
    wrap_x: TextureWrap,
    wrap_y: TextureWrap,
    texture: Option<Texture2D>,
    failed: bool,
}

impl SpineTexture {
    fn texture(&mut self) -> Option<&Texture2D> {
        if self.texture.is_none() && !self.failed {
            match std::fs::read(&self.path) {
                Ok(bytes) => {
                    let texture = Texture2D::from_file_with_format(&bytes, Some(ImageFormat::Png));
                    texture.set_filter(self.filter);

                    // macroquad does not expose wrap settings, so set them through miniquad.
                    let raw_id = texture.raw_miniquad_id();
                    {
                        let gl = unsafe { get_internal_gl() };
                        gl.quad_context
                            .texture_set_wrap(raw_id, self.wrap_x, self.wrap_y);
                    }

                    self.texture = Some(texture);
                }
                Err(err) => {
                    tracing::error!("Failed to read Spine texture '{}': {err}", self.path);
                    self.failed = true;
                }
            }
        }
        self.texture.as_ref()
    }
}

/// Must be called once, before loading any atlas.
pub fn init_texture_callbacks() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        set_texture_callbacks();
    });
}

fn set_texture_callbacks() {
    rusty_spine::extension::set_create_texture_cb(|page, path| {
        let filter = match page.min_filter() {
            AtlasFilter::Nearest => FilterMode::Nearest,
            _ => FilterMode::Linear,
        };
        if page.mag_filter() != page.min_filter() {
            tracing::warn!(
                "atlas page '{}' uses different min/mag filters, using the min filter",
                page.name()
            );
        }
        let wrap = |wrap: AtlasWrap| match wrap {
            AtlasWrap::Repeat => TextureWrap::Repeat,
            AtlasWrap::MirroredRepeat => TextureWrap::Mirror,
            _ => TextureWrap::Clamp,
        };
        if page.format() != AtlasFormat::RGBA8888 && page.format() != AtlasFormat::RGB888 {
            tracing::warn!(
                "atlas page '{}' uses unsupported format {:?}, assuming PNG RGBA",
                page.name(),
                page.format()
            );
        }
        page.renderer_object().set(SpineTexture {
            path: path.to_owned(),
            filter,
            wrap_x: wrap(page.u_wrap()),
            wrap_y: wrap(page.v_wrap()),
            texture: None,
            failed: false,
        });
    });

    rusty_spine::extension::set_dispose_texture_cb(|page| unsafe {
        page.renderer_object().dispose::<SpineTexture>();
    });
}

fn blend_states(blend_mode: BlendMode, premultiplied_alpha: bool) -> (BlendState, BlendState) {
    match blend_mode {
        BlendMode::Additive => {
            if premultiplied_alpha {
                (
                    BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One),
                    BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One),
                )
            } else {
                (
                    BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    ),
                    BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One),
                )
            }
        }
        BlendMode::Multiply => (
            BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::DestinationColor),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            ),
            BlendState::new(
                Equation::Add,
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            ),
        ),
        BlendMode::Normal => {
            if premultiplied_alpha {
                (
                    BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    ),
                    BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    ),
                )
            } else {
                (
                    BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    ),
                    BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    ),
                )
            }
        }
        BlendMode::Screen => (
            BlendState::new(
                Equation::Add,
                BlendFactor::One,
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            ),
            BlendState::new(
                Equation::Add,
                BlendFactor::OneMinusValue(BlendValue::SourceColor),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            ),
        ),
    }
}

/// GPU resources used to draw one Spine skeleton.
pub struct SpineRenderer {
    /// One pipeline per [`BlendMode`] (indexed by its `as usize` value).
    pipelines: [Pipeline; 4],
    vertex_buffer: BufferId,
    index_buffer: BufferId,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl SpineRenderer {
    pub fn new(premultiplied_alpha: bool, backface_culling: bool) -> Self {
        let gl = unsafe { get_internal_gl() };
        let ctx: &mut dyn RenderingBackend = gl.quad_context;

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: shader::VERTEX,
                    fragment: shader::FRAGMENT,
                },
                shader::meta(),
            )
            .expect("failed to compile Spine shader");

        let cull_face = if backface_culling {
            CullFace::Back
        } else {
            CullFace::Nothing
        };

        let pipelines = std::array::from_fn(|index| {
            let blend_mode = match index {
                0 => BlendMode::Normal,
                1 => BlendMode::Additive,
                2 => BlendMode::Multiply,
                _ => BlendMode::Screen,
            };
            let (color_blend, alpha_blend) = blend_states(blend_mode, premultiplied_alpha);
            let params = PipelineParams {
                cull_face,
                front_face_order: FrontFaceOrder::CounterClockwise,
                color_blend: Some(color_blend),
                alpha_blend: Some(alpha_blend),
                primitive_type: PrimitiveType::Triangles,
                ..Default::default()
            };
            ctx.new_pipeline(
                &[mq::BufferLayout::default()],
                &[
                    VertexAttribute::new("position", VertexFormat::Float2),
                    VertexAttribute::new("uv", VertexFormat::Float2),
                    VertexAttribute::new("color", VertexFormat::Float4),
                    VertexAttribute::new("dark_color", VertexFormat::Float4),
                ],
                shader,
                params,
            )
        });

        let vertex_capacity = 4096;
        let index_capacity = 8192;
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<Vertex>(vertex_capacity),
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<u16>(index_capacity),
        );

        Self {
            pipelines,
            vertex_buffer,
            index_buffer,
            vertex_capacity,
            index_capacity,
        }
    }

    /// Draw all renderables with the given view/world matrices (Spine's y-axis points up).
    pub fn draw(&mut self, view: Mat4, world: Mat4, renderables: &[SkeletonCombinedRenderable]) {
        // Resolve/load textures before borrowing the rendering backend; texture creation uses
        // macroquad's own context.
        let mut texture_ids: Vec<Option<TextureId>> = Vec::with_capacity(renderables.len());
        for renderable in renderables {
            let Some(renderer_object) = renderable.attachment_renderer_object else {
                texture_ids.push(None);
                continue;
            };
            // SAFETY: the pointer is owned by the atlas page and lives as long as the skeleton.
            let texture = unsafe { &mut *(renderer_object as *mut SpineTexture) };
            texture_ids.push(texture.texture().map(Texture2D::raw_miniquad_id));
        }

        let mut gl = unsafe { get_internal_gl() };
        // Make sure any geometry macroquad batched before this point is already on screen.
        gl.flush();

        let (screen_width, screen_height) = mq::window::screen_size();
        let ctx: &mut dyn RenderingBackend = gl.quad_context;

        ctx.begin_default_pass(PassAction::Nothing);
        ctx.apply_viewport(0, 0, screen_width as i32, screen_height as i32);
        ctx.apply_scissor_rect(0, 0, screen_width as i32, screen_height as i32);

        let uniforms = Uniforms { world, view };

        for (renderable, texture_id) in renderables.iter().zip(texture_ids) {
            let Some(texture_id) = texture_id else {
                continue;
            };
            if renderable.indices.is_empty() {
                continue;
            }

            let vertices: Vec<Vertex> = renderable
                .vertices
                .iter()
                .zip(&renderable.uvs)
                .enumerate()
                .map(|(i, (position, uv))| Vertex {
                    position: *position,
                    uv: *uv,
                    color: renderable.colors[i],
                    dark_color: renderable.dark_colors[i],
                })
                .collect();

            self.ensure_capacity(ctx, vertices.len(), renderable.indices.len());

            ctx.buffer_update(self.vertex_buffer, BufferSource::slice(&vertices));
            ctx.buffer_update(self.index_buffer, BufferSource::slice(&renderable.indices));

            ctx.apply_pipeline(&self.pipelines[renderable.blend_mode as usize]);
            ctx.apply_bindings(&Bindings {
                vertex_buffers: vec![self.vertex_buffer],
                index_buffer: self.index_buffer,
                images: vec![texture_id],
            });
            ctx.apply_uniforms(UniformsSource::table(&uniforms));
            ctx.draw(0, renderable.indices.len() as i32, 1);
        }

        ctx.end_render_pass();
    }

    fn ensure_capacity(&mut self, ctx: &mut dyn RenderingBackend, vertices: usize, indices: usize) {
        if vertices > self.vertex_capacity {
            self.vertex_capacity = vertices.next_power_of_two();
            ctx.delete_buffer(self.vertex_buffer);
            self.vertex_buffer = ctx.new_buffer(
                BufferType::VertexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<Vertex>(self.vertex_capacity),
            );
        }
        if indices > self.index_capacity {
            self.index_capacity = indices.next_power_of_two();
            ctx.delete_buffer(self.index_buffer);
            self.index_buffer = ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u16>(self.index_capacity),
            );
        }
    }
}
