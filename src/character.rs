//! Spine character loading, animation and placement.
//!
//! Add new characters by dropping their exported files into
//! `spineAssets/characters/<name>/` and appending an entry to [`CHARACTERS`].

use std::sync::Arc;

use glam::{Mat4, Vec3};
use macroquad::prelude::{screen_height, screen_width};
use rusty_spine::controller::SkeletonCombinedRenderable;
use rusty_spine::{
    AnimationStateData, Atlas, Physics, SkeletonBinary, SkeletonData,
    controller::{SkeletonController, SkeletonControllerSettings},
    draw::{ColorSpace, CullDirection},
};

use crate::spine_renderer::{self, SpineRenderer};
use crate::wardrobe::Wardrobe;

/// Static description of a character asset.
pub struct CharacterConfig {
    /// Human readable name shown in the HUD.
    pub name: &'static str,
    /// Path to the `.atlas` file (relative to the working directory).
    pub atlas_path: &'static str,
    /// Path to the `.skel.bytes` (or `.json`) skeleton file.
    pub skeleton_path: &'static str,
    /// Optional skin to apply. The bundled asset's `default` skin is only a placeholder, so a
    /// complete skin must be selected.
    pub skin: Option<&'static str>,
    /// Animation to start with. If missing, a sensible default is picked.
    pub animation: Option<&'static str>,
    /// Fixed world scale. `None` fits the character to [`Self::height_ratio`] of the window.
    pub scale: Option<f32>,
    /// Fraction of the window height used when auto-fitting.
    pub height_ratio: f32,
    /// Distance from the bottom of the window (fraction of window height).
    pub bottom_margin: f32,
    /// Enable backface culling. Some Spine exports require it, some break with it.
    pub backface_culling: bool,
}

/// Registry of all characters available in the project.
pub const CHARACTERS: &[CharacterConfig] = &[CharacterConfig {
    name: "Character 01",
    atlas_path: "spineAssets/characters/character_01/skeleton.atlas",
    skeleton_path: "spineAssets/characters/character_01/skeleton.skel.bytes",
    skin: Some("Fox/-Fox"),
    animation: Some("idle/idle"),
    scale: None,
    // Matches the Unity reference: camera ortho size 5, import scale 0.01, base player scale 0.9.
    height_ratio: 0.42,
    // Unity default character position is (0, -3) with ortho size 5 => 20% above the bottom.
    bottom_margin: 0.20,
    backface_culling: false,
}];

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl Bounds {
    fn height(self) -> f32 {
        (self.max_y - self.min_y).max(1.0)
    }
}

/// A loaded, animating Spine character.
pub struct Character {
    config: &'static CharacterConfig,
    controller: SkeletonController,
    renderer: SpineRenderer,
    bounds: Bounds,
    animation_names: Vec<String>,
    current_animation: usize,
    wardrobe: Wardrobe,
}

impl Character {
    pub fn load(config: &'static CharacterConfig) -> Result<Self, String> {
        spine_renderer::init_texture_callbacks();

        let atlas = Arc::new(
            Atlas::new_from_file(config.atlas_path)
                .map_err(|e| format!("failed to load atlas '{}': {e:?}", config.atlas_path))?,
        );
        let premultiplied_alpha = atlas.pages().any(|page| page.pma());

        let skeleton_data = Arc::new(
            SkeletonBinary::new(atlas.clone())
                .read_skeleton_data_file(config.skeleton_path)
                .map_err(|e| {
                    format!("failed to load skeleton '{}': {e:?}", config.skeleton_path)
                })?,
        );

        if let Some(version) = skeleton_data.version() {
            if !version.starts_with("4.2") {
                tracing::warn!(
                    "'{}' was exported with Spine {version}, but this runtime supports 4.2",
                    config.name
                );
            }
        }

        let mut animation_state_data = AnimationStateData::new(skeleton_data.clone());
        // Same cross-fade duration as the Unity reference project.
        animation_state_data.set_default_mix(0.2);
        let animation_state_data = Arc::new(animation_state_data);
        let mut controller = SkeletonController::new(skeleton_data.clone(), animation_state_data)
            .with_settings(SkeletonControllerSettings {
                premultiplied_alpha,
                cull_direction: CullDirection::CounterClockwise,
                color_space: ColorSpace::SRGB,
            });

        let mut wardrobe = Wardrobe::load(
            skeleton_data.clone(),
            atlas,
            config.skin.unwrap_or("default"),
        )?;
        wardrobe.apply(&mut controller)?;

        let animation_names = usable_animations(&skeleton_data);
        let start_animation = config
            .animation
            .filter(|name| skeleton_data.find_animation(name).is_some())
            .map(str::to_owned)
            .or_else(|| pick_default_animation(&animation_names))
            .ok_or_else(|| format!("skeleton '{}' has no animations", config.name))?;

        controller
            .animation_state
            .set_animation_by_name(0, &start_animation, true)
            .map_err(|e| format!("failed to play animation '{start_animation}': {e:?}"))?;

        // Apply the initial pose so bounds can be measured.
        controller.update(0.0, Physics::Update);

        let renderables = controller.combined_renderables();
        let bounds = bounds_of(&renderables).unwrap_or(Bounds {
            min_x: skeleton_data.x(),
            min_y: skeleton_data.y(),
            max_x: skeleton_data.x() + skeleton_data.width(),
            max_y: skeleton_data.y() + skeleton_data.height(),
        });

        let current_animation = animation_names
            .iter()
            .position(|name| name == &start_animation)
            .unwrap_or(0);

        let renderer = SpineRenderer::new(premultiplied_alpha, config.backface_culling);

        Ok(Self {
            config,
            controller,
            renderer,
            bounds,
            animation_names,
            current_animation,
            wardrobe,
        })
    }

    pub fn update(&mut self, delta_seconds: f32) {
        self.controller.update(delta_seconds, Physics::Update);
        self.wardrobe.apply_visibility(&mut self.controller);
    }
    pub fn set_language(&mut self, language: &str) {
        self.wardrobe.language = language.to_owned();
    }

    pub fn choose(&mut self, category: Option<usize>, index: usize) {
        let old_character = self.wardrobe.current_character;
        let old_selected = self.wardrobe.selected;
        if let Some(category) = category {
            if index >= self.wardrobe.items[category].len() {
                return;
            }
            self.wardrobe.selected[category] = index;
            self.wardrobe.category = category;
        } else {
            if index >= self.wardrobe.characters.len() {
                return;
            }
            self.wardrobe.current_character = index;
        }
        if let Err(error) = self.wardrobe.apply(&mut self.controller) {
            self.wardrobe.current_character = old_character;
            self.wardrobe.selected = old_selected;
            tracing::warn!("Cannot apply selection: {error}");
        }
    }

    /// Draw the character centered horizontally and standing on the bottom margin.
    pub fn draw(&mut self) {
        let (screen_width, screen_height) = (screen_width(), screen_height());
        let mut scale = self
            .config
            .scale
            .unwrap_or_else(|| screen_height * self.config.height_ratio / self.bounds.height());

        let center_x = (self.bounds.min_x + self.bounds.max_x) * 0.5;
        let (gui_w, gui_h, gui_scale) = crate::gui_scale::dimensions(screen_width, screen_height);
        let (preview, _) = crate::gui_scale::regions(macroquad::prelude::vec2(gui_w, gui_h));
        scale = scale
            .min(preview.h * gui_scale * 0.8 / self.bounds.height())
            .min(preview.w * gui_scale * 0.8 / (self.bounds.max_x - self.bounds.min_x).max(1.0));
        let preview_center = preview.x + preview.w * 0.5;
        let foot_y = if crate::gui_scale::compact() {
            (preview.y + preview.h * 0.90) * gui_scale
        } else {
            screen_height * (1.0 - self.config.bottom_margin)
        };
        let translation = Vec3::new(
            -center_x * scale + preview_center * gui_scale - screen_width * 0.5,
            -self.bounds.min_y * scale + screen_height * 0.5 - foot_y,
            0.0,
        );
        let world = Mat4::from_translation(translation) * Mat4::from_scale(Vec3::splat(scale));
        let view = Mat4::orthographic_rh_gl(
            -screen_width * 0.5,
            screen_width * 0.5,
            -screen_height * 0.5,
            screen_height * 0.5,
            -1.0,
            1.0,
        );

        let renderables = self.controller.combined_renderables();
        self.renderer.draw(view, world, &renderables);
    }

    pub fn cycle_animation(&mut self, direction: isize) {
        if self.animation_names.is_empty() {
            return;
        }
        let len = self.animation_names.len() as isize;
        let next_animation = (self.current_animation as isize + direction).rem_euclid(len) as usize;
        let name = &self.animation_names[next_animation];
        if let Err(err) = replace_animation(&mut self.controller, name) {
            tracing::error!("Failed to play animation '{name}': {err:?}");
        } else {
            self.current_animation = next_animation;
            self.wardrobe.apply_visibility(&mut self.controller);
            tracing::debug!("Animation changed: {name}");
        }
    }

    pub fn name(&self) -> &str {
        self.wardrobe.character_name()
    }

    pub fn skin_name(&self) -> &str {
        &self.wardrobe.characters[self.wardrobe.current_character].spine_skin
    }

    pub fn wardrobe(&self) -> &Wardrobe {
        &self.wardrobe
    }

    pub fn select_wardrobe_category(&mut self) {
        self.wardrobe.category = (self.wardrobe.category + 1) % 5;
    }

    pub fn cycle_outfit(&mut self, direction: isize) {
        let category = self.wardrobe.category;
        let previous = self.wardrobe.selected[category];
        let count = self.wardrobe.items[category].len();
        for offset in 1..=count {
            self.wardrobe.selected[category] = (previous as isize + direction * offset as isize)
                .rem_euclid(count as isize) as usize;
            match self.wardrobe.apply(&mut self.controller) {
                Ok(()) => {
                    tracing::info!(
                        "Equipped {}: {}",
                        crate::wardrobe::CATEGORIES[category],
                        self.wardrobe.item_name(category)
                    );
                    return;
                }
                Err(error) => tracing::warn!(
                    "Skipping incompatible item {}: {error}",
                    self.wardrobe.items[category][self.wardrobe.selected[category]].inventory_id
                ),
            }
        }
        self.wardrobe.selected[category] = previous;
    }

    pub fn clear_outfit(&mut self) {
        let previous = self.wardrobe.selected;
        self.wardrobe.selected = [0; 5];
        if let Err(error) = self.wardrobe.apply(&mut self.controller) {
            self.wardrobe.selected = previous;
            tracing::error!("Cannot clear outfit: {error}");
        }
    }

    pub fn animation_name(&self) -> &str {
        &self.animation_names[self.current_animation]
    }

    pub fn animation_count(&self) -> usize {
        self.animation_names.len()
    }

    pub fn animation_index(&self) -> usize {
        self.current_animation + 1
    }
}

/// Replace the whole action, as the Unity controller does when its action changes.
/// Clearing tracks alone leaves the last applied pose on the skeleton. Reset that pose as
/// well, so properties absent from the new animation cannot retain values from the old one.
fn replace_animation(controller: &mut SkeletonController, name: &str) -> Result<(), String> {
    if controller.skeleton.data().find_animation(name).is_none() {
        return Err(format!("animation '{name}' does not exist"));
    }
    controller.animation_state.clear_tracks();
    controller.skeleton.set_to_setup_pose();
    controller
        .animation_state
        .set_animation_by_name(0, name, true)
        .map_err(|err| format!("{err:?}"))?;
    controller.update(0.0, Physics::Reset);
    Ok(())
}

fn bounds_of(renderables: &[SkeletonCombinedRenderable]) -> Option<Bounds> {
    let mut bounds = Bounds {
        min_x: f32::INFINITY,
        min_y: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        max_y: f32::NEG_INFINITY,
    };
    let mut any = false;
    for renderable in renderables {
        for vertex in &renderable.vertices {
            any = true;
            bounds.min_x = bounds.min_x.min(vertex[0]);
            bounds.min_y = bounds.min_y.min(vertex[1]);
            bounds.max_x = bounds.max_x.max(vertex[0]);
            bounds.max_y = bounds.max_y.max(vertex[1]);
        }
    }
    any.then_some(bounds)
}

fn usable_animations(skeleton_data: &SkeletonData) -> Vec<String> {
    let names: Vec<String> = skeleton_data
        .animations()
        .map(|animation| animation.name().to_owned())
        .filter(|name| !is_special_animation(name))
        .collect();
    if names.is_empty() {
        skeleton_data
            .animations()
            .map(|animation| animation.name().to_owned())
            .collect()
    } else {
        names
    }
}

fn is_special_animation(name: &str) -> bool {
    name.starts_with("-marketing exports")
        || matches!(
            name,
            "silhouette" | "T-pose" | "TEST" | "TEST2" | "_ball" | "_export_poses"
        )
}

fn pick_default_animation(names: &[String]) -> Option<String> {
    for preferred in ["idle/idle", "idle/idle_menu", "idle/idle_arms", "idle/talk"] {
        if names.iter().any(|name| name == preferred) {
            return Some(preferred.to_owned());
        }
    }
    names
        .iter()
        .find(|name| name.starts_with("idle/"))
        .or_else(|| names.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone_pose(controller: &SkeletonController) -> Vec<[f32; 7]> {
        controller
            .skeleton
            .bones()
            .map(|bone| {
                [
                    bone.x(),
                    bone.y(),
                    bone.rotation(),
                    bone.scale_x(),
                    bone.scale_y(),
                    bone.shear_x(),
                    bone.shear_y(),
                ]
            })
            .collect()
    }

    #[test]
    fn returning_to_idle_does_not_depend_on_previous_animation() {
        // Exercise the actual exported timelines without creating a graphics context.
        let config = &CHARACTERS[0];
        let atlas = Arc::new(Atlas::new_from_file(config.atlas_path).unwrap());
        let data = Arc::new(
            SkeletonBinary::new(atlas)
                .read_skeleton_data_file(config.skeleton_path)
                .unwrap(),
        );
        let state_data = Arc::new(AnimationStateData::new(data.clone()));
        let mut controller = SkeletonController::new(data.clone(), state_data);
        controller
            .skeleton
            .set_skin_by_name(config.skin.unwrap())
            .unwrap();
        replace_animation(&mut controller, "idle/idle").unwrap();
        let expected = bone_pose(&controller);

        for name in usable_animations(&data) {
            replace_animation(&mut controller, &name).unwrap();
            for _ in 0..30 {
                controller.update(1.0 / 60.0, Physics::Update);
            }
            replace_animation(&mut controller, "idle/idle").unwrap();
            for (index, (actual, expected)) in
                bone_pose(&controller).iter().zip(&expected).enumerate()
            {
                for (actual, expected) in actual.iter().zip(expected) {
                    assert!(
                        (actual - expected).abs() < 0.0001,
                        "bone {index} retained pose after {name}: {actual} != {expected}"
                    );
                }
            }
        }
    }
}
