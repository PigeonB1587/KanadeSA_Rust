//! Straight-alpha sprites composited into an RGBA target need separate alpha
//! blending. Macroquad's default pipeline applies SourceAlpha to alpha as well.
use macroquad::{
    miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams},
    prelude::*,
};

pub fn load() -> Result<Material, String> {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX,
            fragment: FRAGMENT,
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                alpha_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())
}
const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
}"#;
const FRAGMENT: &str = r#"#version 100
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() { gl_FragColor = color * texture2D(Texture, uv); }
"#;
