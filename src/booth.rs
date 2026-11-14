//! Booth coordinates, background sprites, independently sorted actors and capture.
use crate::{
    character::Character,
    model::{CharacterFile, FORMAT_VERSION, SceneFile, Watermark},
};
use glam::Mat4;
use macroquad::prelude::*;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub const UNITY_SCALE: f32 = 5.0 / 67.5;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Background {
    pub img: String,
    // Retained for import/reference validation; the picker currently displays names.
    #[allow(dead_code)]
    pub thumb: String,
    pub color: [f32; 3],
    #[serde(default)]
    pub go_solid_on_record: bool,
    #[serde(default)]
    pub transparency_option: bool,
    pub horizon_y: f32,
    pub horizon_mult: f32,
    pub horizon_bot_start: f32,
    #[serde(default)]
    pub layers: Vec<Layer>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub img: String,
    pub offset_x: f32,
    pub offset_y: f32,
    #[serde(default)]
    pub z_offset: f32,
}
#[derive(Clone, Deserialize, Debug)]
pub struct SpriteMeta {
    pub size: [u32; 2],
    pub pixels_per_unit: f32,
    pub pivot: [f32; 2],
    pub filter_mode: i32,
}
#[derive(Deserialize)]
struct SpriteFile {
    format_version: u32,
    sprites: HashMap<String, SpriteMeta>,
}

pub fn resource_path(path: &str) -> String {
    format!("{}.png", path.to_lowercase())
}
pub fn perspective(background: &Background, y: f32) -> f32 {
    let start = (background.horizon_bot_start * 135.0 - 67.5) * UNITY_SCALE;
    let end = background.horizon_y * UNITY_SCALE;
    if (end - start).abs() < 0.00001 {
        return 1.0;
    }
    let t = ((y - start) / (end - start)).clamp(0.0, 1.0);
    (1.0 - background.horizon_mult * 0.85 * t).max(0.15)
}
/// Rectangle in a top-left pixel canvas from Unity's bottom-left sprite pivot.
pub fn sprite_rect(meta: &SpriteMeta, position: [f32; 2], scale: [f32; 2], size: Vec2) -> Rect {
    let pixels = size.y / (2.0 * camera_half_height(size));
    let width = meta.size[0] as f32 / meta.pixels_per_unit * scale[0].abs() * pixels;
    let height = meta.size[1] as f32 / meta.pixels_per_unit * scale[1].abs() * pixels;
    Rect::new(
        size.x * 0.5 + position[0] * pixels - width * meta.pivot[0],
        size.y * 0.5 - position[1] * pixels - height * (1.0 - meta.pivot[1]),
        width,
        height,
    )
}
// Match Unity's CameraOrthographicMatchWidth without stretching scene geometry.
fn camera_half_height(size: Vec2) -> f32 {
    5.0 * ((16.0 / 9.0) / (size.x / size.y)).min(1.0)
}
pub fn world_to_canvas(position: [f32; 2], size: Vec2) -> Vec2 {
    let pixels = size.y / (2.0 * camera_half_height(size));
    vec2(
        size.x * 0.5 + position[0] * pixels,
        size.y * 0.5 - position[1] * pixels,
    )
}
pub fn canvas_to_world(point: Vec2, size: Vec2) -> [f32; 2] {
    let units = 2.0 * camera_half_height(size) / size.y;
    [
        (point.x - size.x * 0.5) * units,
        (size.y * 0.5 - point.y) * units,
    ]
}

pub struct Booth {
    pub characters: Vec<Character>,
    pub selected: Option<usize>,
    pub backgrounds: Vec<Background>,
    pub background: usize,
    pub watermark: Watermark,
    pub transparent: bool,
    pub language: String,
    pub save_directory: PathBuf,
    sprites: HashMap<String, (Texture2D, SpriteMeta)>,
    metadata: HashMap<String, SpriteMeta>,
    sprite_material: Material,
    pub target: RenderTarget,
}
#[derive(Clone, Copy)]
enum DrawItem {
    Layer(usize),
    Character(usize),
    Pet(usize),
}
impl Booth {
    pub fn new(character: Character) -> Result<Self, String> {
        let backgrounds: Vec<Background> =
            serde_json::from_str(include_str!("../configs/_boothbg.json"))
                .map_err(|e| e.to_string())?;
        let target = render_target(1280, 720);
        target.texture.set_filter(FilterMode::Linear);
        Ok(Self {
            language: character.wardrobe().language.clone(),
            characters: vec![character],
            selected: Some(0),
            backgrounds,
            background: 1,
            watermark: Watermark::Right,
            transparent: false,
            save_directory: PathBuf::from("saves/default"),
            sprites: HashMap::new(),
            metadata: HashMap::new(),
            sprite_material: crate::sprite_material::load()?,
            target,
        })
    }
    pub fn selected(&self) -> Option<&Character> {
        self.selected.and_then(|i| self.characters.get(i))
    }
    pub fn selected_mut(&mut self) -> Option<&mut Character> {
        self.selected.and_then(|i| self.characters.get_mut(i))
    }
    pub fn draw_preview(&mut self) -> Result<(), String> {
        let Some(index) = self.selected else {
            return Ok(());
        };
        // Used by the rendering verifier; interactive Preview owns a separate entity.
        let path = format!(
            "booth/gems/gem_{}.png",
            (self.characters[index].level / 10).min(99)
        );
        if self.characters[index].gem(1.0).is_some() {
            self.load_sprite(&path)?;
        }
        let sprite = self.sprites.get(&path);
        let gem = self.characters[index].draw(sprite.map(|(_, meta)| meta));
        Self::draw_preview_gem(gem, sprite);
        Ok(())
    }
    pub fn draw_preview_entity(
        &mut self,
        character: &mut Character,
        opacity: f32,
    ) -> Result<(), String> {
        let path = format!("booth/gems/gem_{}.png", (character.level / 10).min(99));
        if character.gem(1.0).is_some() {
            self.load_sprite(&path)?;
        }
        let sprite = self.sprites.get(&path);
        let gem = character.draw_faded(sprite.map(|(_, meta)| meta), opacity);
        Self::draw_preview_gem(gem, sprite);
        Ok(())
    }
    fn draw_preview_gem(gem: Option<(Rect, f32)>, sprite: Option<&(Texture2D, SpriteMeta)>) {
        crate::gui::layout::begin();
        if let (Some((rect, alpha)), Some((texture, _))) = (gem, sprite) {
            crate::gui::widgets::paint::draw_texture_ex(
                texture,
                rect.x,
                rect.y,
                Color::new(1.0, 1.0, 1.0, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(rect.w, rect.h)),
                    ..Default::default()
                },
            );
        }
    }
    pub fn add(&mut self) -> Result<(), String> {
        // The last entity is retained off-scene when empty, preserving shared asset ownership.
        let source = self
            .selected()
            .or_else(|| self.characters.first())
            .ok_or("No character template")?;
        let mut character = source.duplicate()?;
        let offset = self.characters.len() as f32 * 0.65;
        character.placement.position = [offset.min(5.0), -3.0];
        character.placement.height = -3.0;
        character.set_language(&self.language);
        self.characters.push(character);
        self.selected = Some(self.characters.len() - 1);
        Ok(())
    }
    pub fn remove(&mut self) {
        if let Some(i) = self.selected {
            if self.characters.len() == 1 {
                self.selected = None;
            } else {
                self.characters.remove(i);
                self.selected = Some(i.min(self.characters.len() - 1));
            }
        }
    }
    pub fn active_count(&self) -> usize {
        if self.selected.is_none() {
            0
        } else {
            self.characters.len()
        }
    }
    pub fn add_or_restore(&mut self) -> Result<(), String> {
        if self.selected.is_none() {
            // Replace removed state with a fresh independent entity and stable ID.
            let character = self.characters[0].duplicate()?;
            self.characters[0] = character;
            self.selected = Some(0);
            Ok(())
        } else {
            self.add()
        }
    }
    pub fn update(&mut self, dt: f32) {
        if self.selected.is_some() {
            for character in &mut self.characters {
                character.update(dt);
            }
        }
    }
    pub fn set_language(&mut self, language: String) {
        for character in &mut self.characters {
            character.set_language(&language);
        }
        self.language = language;
    }
    pub fn switch_background(&mut self, index: usize) {
        self.background = index.min(self.backgrounds.len() - 1);
        self.transparent &= self.backgrounds[self.background].transparency_option;
        // Keep only common sprites; previous backgrounds can be large.
        self.sprites.retain(|key, _| {
            key.starts_with("booth/gems/")
                || key == "booth/shadow.png"
                || key == "booth/watermark.png"
        });
    }
    fn load_sprite(&mut self, path: &str) -> Result<(), String> {
        if self.sprites.contains_key(path) {
            return Ok(());
        }
        if !self.metadata.contains_key(path) {
            let p = Path::new(path);
            let parent = p.parent().ok_or("Missing sprite parent")?;
            let file: SpriteFile = serde_json::from_slice(
                &std::fs::read(parent.join("sprites.json")).map_err(|e| format!("{path}: {e}"))?,
            )
            .map_err(|e| e.to_string())?;
            if file.format_version != 1 {
                return Err("Unsupported sprite metadata".into());
            }
            for (name, meta) in file.sprites {
                self.metadata
                    .insert(parent.join(name).to_string_lossy().into_owned(), meta);
            }
        }
        let meta = self
            .metadata
            .get(path)
            .ok_or_else(|| format!("Missing metadata for {path}"))?
            .clone();
        let image = Image::from_file_with_format(
            &std::fs::read(path).map_err(|e| format!("{path}: {e}"))?,
            Some(ImageFormat::Png),
        )
        .map_err(|e| e.to_string())?;
        if [image.width as u32, image.height as u32] != meta.size {
            return Err(format!("Sprite dimensions changed: {path}"));
        }
        let texture = Texture2D::from_image(&image);
        texture.set_filter(if meta.filter_mode == 0 {
            FilterMode::Nearest
        } else {
            FilterMode::Linear
        });
        self.sprites.insert(path.into(), (texture, meta));
        Ok(())
    }
    fn sprite(&self, path: &str, position: [f32; 2], scale: [f32; 2], color: Color, size: Vec2) {
        if let Some((texture, meta)) = self.sprites.get(path) {
            let r = sprite_rect(meta, position, scale, size);
            draw_texture_ex(
                texture,
                r.x,
                r.y,
                color,
                DrawTextureParams {
                    dest_size: Some(vec2(r.w, r.h)),
                    flip_x: scale[0] < 0.0,
                    flip_y: scale[1] < 0.0,
                    ..Default::default()
                },
            );
        }
    }
    pub fn render(&mut self, width: u32, height: u32, capture: bool) -> Result<(), String> {
        if self.target.texture.width() as u32 != width
            || self.target.texture.height() as u32 != height
        {
            self.target = render_target(width, height);
            self.target.texture.set_filter(FilterMode::Linear);
        }
        let bg = self.backgrounds[self.background].clone();
        let transparent = capture && self.transparent && bg.transparency_option;
        let hide_background = transparent || (capture && bg.go_solid_on_record);
        let mut paths = vec!["booth/shadow.png".into(), "booth/watermark.png".into()];
        if !hide_background {
            paths.push(resource_path(&bg.img));
            paths.extend(bg.layers.iter().map(|layer| resource_path(&layer.img)));
        }
        for c in &self.characters {
            if c.gem(1.0).is_some() {
                paths.push(format!("booth/gems/gem_{}.png", (c.level / 10).min(99)));
            }
        }
        for path in paths {
            self.load_sprite(&path)?;
        }
        let size = vec2(width as f32, height as f32);
        // With negative zoom, canvas Y is down inside the offscreen framebuffer.
        set_camera(&Camera2D {
            target: size * 0.5,
            zoom: vec2(2.0 / size.x, -2.0 / size.y),
            render_target: Some(self.target.clone()),
            ..Default::default()
        });
        clear_background(if transparent {
            Color::new(0.0, 0.0, 0.0, 0.0)
        } else if hide_background {
            Color::new(bg.color[0], bg.color[1], bg.color[2], 1.0)
        } else {
            BLACK
        });
        gl_use_material(&self.sprite_material);
        if !hide_background {
            self.sprite(
                &resource_path(&bg.img),
                [0.0, 0.0],
                [UNITY_SCALE; 2],
                Color::new(bg.color[0], bg.color[1], bg.color[2], 1.0),
                size,
            );
        }
        let count = self.active_count();
        // Shadows are the Unity Shadow sorting layer, below every default layer.
        for c in self.characters.iter().take(count) {
            let p = &c.placement;
            let scale = perspective(&bg, p.position[1]) * 0.9 * p.scale;
            let center = world_to_canvas(p.position, size);
            if let Some((texture, _)) = self.sprites.get("booth/shadow.png") {
                let extent = vec2(2.59, 0.62) * scale * size.y / (2.0 * camera_half_height(size));
                draw_texture_ex(
                    texture,
                    center.x - extent.x * 0.5,
                    center.y - extent.y * 0.5,
                    Color::new(1.0, 1.0, 1.0, p.shadow_alpha),
                    DrawTextureParams {
                        dest_size: Some(extent),
                        ..Default::default()
                    },
                );
            }
        }
        let mut sorted: Vec<(f32, DrawItem)> = vec![];
        if !hide_background {
            for (i, l) in bg.layers.iter().enumerate() {
                sorted.push(((l.offset_y + l.z_offset) * UNITY_SCALE, DrawItem::Layer(i)));
            }
        }
        for (i, c) in self.characters.iter().take(count).enumerate() {
            sorted.push((c.placement.position[1], DrawItem::Character(i)));
            if c.has_pet() {
                sorted.push((
                    c.pet_position(perspective(&bg, c.placement.position[1]))[1],
                    DrawItem::Pet(i),
                ));
            }
        }
        // Stable ordering matches Unity at equal depth; smaller Z draws last.
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        sorted.reverse();
        let half_height = camera_half_height(size);
        let view = Mat4::orthographic_rh_gl(
            -half_height * size.x / size.y,
            half_height * size.x / size.y,
            -half_height,
            half_height,
            -1.0,
            1.0,
        );
        for (_, item) in sorted {
            match item {
                DrawItem::Layer(i) => {
                    let l = &bg.layers[i];
                    self.sprite(
                        &resource_path(&l.img),
                        [l.offset_x * UNITY_SCALE, l.offset_y * UNITY_SCALE],
                        [UNITY_SCALE; 2],
                        WHITE,
                        size,
                    );
                }
                DrawItem::Character(i) => {
                    let c = &mut self.characters[i];
                    let scale = perspective(&bg, c.placement.position[1]);
                    c.draw_world(view, scale, &self.target);
                    if let Some((pos, scale, alpha)) = c.gem(scale) {
                        let path = format!("booth/gems/gem_{}.png", (c.level / 10).min(99));
                        self.sprite(&path, pos, scale, Color::new(1.0, 1.0, 1.0, alpha), size);
                    }
                }
                DrawItem::Pet(i) => {
                    let c = &mut self.characters[i];
                    let pos = c.pet_position(perspective(&bg, c.placement.position[1]));
                    c.draw_pet(view, pos, perspective(&bg, pos[1]), &self.target);
                }
            }
        }
        if self.watermark != Watermark::Hidden {
            let s = size.y / 1080.0;
            if let Some((texture, _)) = self.sprites.get("booth/watermark.png") {
                let x = if self.watermark == Watermark::Right {
                    size.x - 250.0 * s
                } else {
                    30.0 * s
                };
                draw_texture_ex(
                    texture,
                    x,
                    size.y - 153.0 * s,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(220.0, 128.0) * s),
                        ..Default::default()
                    },
                );
            }
        }
        unsafe {
            get_internal_gl().flush();
        }
        gl_use_default_material();
        set_default_camera();
        Ok(())
    }
    pub fn save(&self) -> Result<(), String> {
        let revision = crate::persistence::new_id();
        let count = self.active_count();
        let characters = CharacterFile {
            format_version: FORMAT_VERSION,
            revision: revision.clone(),
            characters: self
                .characters
                .iter()
                .take(count)
                .map(Character::data)
                .collect(),
        };
        let scene = SceneFile {
            format_version: FORMAT_VERSION,
            revision,
            background: self.backgrounds[self.background].img.clone(),
            watermark: self.watermark,
            transparent: self.transparent,
            selected_character: self.selected().map(|c| c.id.clone()),
            placements: self
                .characters
                .iter()
                .take(count)
                .map(|c| c.placement.clone())
                .collect(),
        };
        crate::persistence::save(&self.save_directory, &characters, &scene)
    }
    pub fn load(&mut self) -> Result<(), String> {
        let (characters, scene) = crate::persistence::load(&self.save_directory)?;
        let background = self
            .backgrounds
            .iter()
            .position(|b| b.img == scene.background)
            .ok_or("Saved background is unavailable")?;
        let template = self.characters.first().ok_or("No character template")?;
        let mut restored = Vec::new();
        for p in scene.placements {
            let data = characters
                .characters
                .iter()
                .find(|c| c.id == p.character_id)
                .ok_or("Missing character reference")?;
            let mut character = template.duplicate()?;
            character.restore(data, p)?;
            character.set_language(&self.language);
            restored.push(character);
        }
        let selected = scene
            .selected_character
            .as_ref()
            .and_then(|id| restored.iter().position(|c| c.id == *id));
        // Stage the complete runtime before replacing the current scene.
        if restored.is_empty() {
            self.selected = None;
            self.characters.truncate(1);
        } else {
            self.characters = restored;
            self.selected = selected.or(Some(0));
        }
        self.switch_background(background);
        self.watermark = scene.watermark;
        self.transparent = scene.transparent && self.backgrounds[background].transparency_option;
        Ok(())
    }
    pub fn capture(&mut self, width: u32, height: u32) -> Result<PathBuf, String> {
        let path =
            PathBuf::from("photos").join(format!("KanadeSA-{}.png", crate::persistence::new_id()));
        self.capture_to(&path, width, height)?;
        Ok(path)
    }
    pub fn capture_to(&mut self, path: &Path, width: u32, height: u32) -> Result<(), String> {
        self.render(width, height, true)?;
        let mut image = self.target.texture.get_texture_data();
        let transparent = self.transparent && self.backgrounds[self.background].transparency_option;
        if transparent {
            unpremultiply(&mut image.bytes);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // macroquad Image::export_png flips GL readback, but panics on I/O errors.
        // Use the encoder directly so failures are reported in the UI.
        let stride = width as usize * 4;
        let mut pixels = vec![0; image.bytes.len()];
        for y in 0..height as usize {
            pixels[y * stride..(y + 1) * stride].copy_from_slice(
                &image.bytes[(height as usize - 1 - y) * stride..(height as usize - y) * stride],
            );
        }
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
pub fn unpremultiply(bytes: &mut [u8]) {
    for pixel in bytes.chunks_exact_mut(4) {
        let a = pixel[3] as u32;
        if a == 0 {
            pixel[..3].fill(0)
        } else {
            for c in &mut pixel[..3] {
                *c = (((*c as u32) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unity_sprites_have_valid_metadata_and_all_references_resolve() {
        let backgrounds: Vec<Background> =
            serde_json::from_str(include_str!("../configs/_boothbg.json")).unwrap();
        assert_eq!(backgrounds.len(), 15);
        for bg in backgrounds {
            for reference in std::iter::once(&bg.img)
                .chain(std::iter::once(&bg.thumb))
                .chain(bg.layers.iter().map(|l| &l.img))
            {
                let path = resource_path(reference);
                let p = Path::new(&path);
                let file: SpriteFile = serde_json::from_slice(
                    &std::fs::read(p.parent().unwrap().join("sprites.json")).unwrap(),
                )
                .unwrap();
                let meta = &file.sprites[p.file_name().unwrap().to_str().unwrap()];
                assert!(meta.pixels_per_unit > 0.0);
                assert!(meta.size.iter().all(|v| *v > 0));
                assert!(p.exists());
            }
            assert!((perspective(&bg, -5.0) - 1.0).abs() < 0.1);
            assert!(perspective(&bg, 5.0) >= 0.15);
        }
    }
    #[test]
    fn pivot_coordinate_depth_and_alpha_conversion() {
        let meta = SpriteMeta {
            size: [80, 40],
            pixels_per_unit: 8.0,
            pivot: [0.0, 0.0],
            filter_mode: 1,
        };
        let size = vec2(1280.0, 720.0);
        let pos = [-2.0, 1.0];
        let r = sprite_rect(&meta, pos, [UNITY_SCALE; 2], size);
        let anchor = world_to_canvas(pos, size);
        assert!((r.x - anchor.x).abs() < 0.001);
        assert!((r.y + r.h - anchor.y).abs() < 0.001);
        let restored = canvas_to_world(anchor, size);
        for i in 0..2 {
            assert!((restored[i] - pos[i]).abs() < 0.001)
        }
        let mut bytes = [64, 32, 16, 128, 5, 6, 7, 0];
        unpremultiply(&mut bytes);
        assert_eq!(bytes, [128, 64, 32, 128, 0, 0, 0, 0]);
    }
}
