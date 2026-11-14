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
use crate::{
    animation::AnimationRuntime,
    model::{Action, AnimationData, CharacterData, Placement, ReferenceFrame},
};

struct Assets {
    data: Arc<SkeletonData>,
    state: Arc<AnimationStateData>,
    pet_data: Arc<SkeletonData>,
    pet_state: Arc<AnimationStateData>,
    pet_pma: bool,
}

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
    fn include(&mut self, other: Self) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }
    fn transformed(self, matrix: Mat4) -> Self {
        let mut result = Self {
            min_x: f32::INFINITY,
            min_y: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            max_y: f32::NEG_INFINITY,
        };
        for x in [self.min_x, self.max_x] {
            for y in [self.min_y, self.max_y] {
                let point = matrix.transform_point3(Vec3::new(x, y, 0.0));
                result.include(Self {
                    min_x: point.x,
                    max_x: point.x,
                    min_y: point.y,
                    max_y: point.y,
                });
            }
        }
        result
    }
}

/// Start at 42% of window height and shrink to fit character/pet content.
/// The decorative spring platform is excluded from framing bounds.
fn preview_framing(
    reference: Bounds,
    content: Bounds,
    screen: [f32; 2],
    preview: macroquad::prelude::Rect,
    gui_scale: f32,
    compact: bool,
) -> (Mat4, f32) {
    let center_x = (reference.min_x + reference.max_x) * 0.5;
    let foot = reference.min_y;
    let anchor_x = (preview.x + preview.w * 0.5) * gui_scale;
    let anchor_y = if compact {
        (preview.y + preview.h * 0.90) * gui_scale
    } else {
        screen[1] * 0.80
    };
    let mut scale = screen[1] * 0.42 / (reference.max_y - reference.min_y).max(0.01);
    for (space, extent) in [
        (preview.w * gui_scale * 0.40, center_x - content.min_x),
        (preview.w * gui_scale * 0.40, content.max_x - center_x),
        (
            anchor_y - (preview.y + preview.h * 0.05) * gui_scale,
            content.max_y - foot,
        ),
        (
            (preview.y + preview.h * 0.95) * gui_scale - anchor_y,
            foot - content.min_y,
        ),
    ] {
        if extent > 0.0 {
            scale = scale.min(space.max(0.0) / extent);
        }
    }
    let framing = Mat4::from_translation(Vec3::new(
        anchor_x - screen[0] * 0.5,
        screen[1] * 0.5 - anchor_y,
        0.0,
    )) * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_translation(Vec3::new(-center_x, -foot, 0.0));
    (framing, scale)
}

/// A loaded, animating Spine character.
pub struct Character {
    config: &'static CharacterConfig,
    controller: SkeletonController,
    pet: SkeletonController,
    preview_platform: Option<SkeletonController>,
    preview_scale: Option<f32>,
    renderer: SpineRenderer,
    bounds: Bounds,
    animation_names: Arc<Vec<String>>,
    wardrobe: Wardrobe,
    assets: Arc<Assets>,
    pub id: String,
    pub level: u32,
    pub animation: AnimationData,
    pub placement: Placement,
    animation_runtime: AnimationRuntime,
    pet_signature: String,
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
        let mut controller =
            SkeletonController::new(skeleton_data.clone(), animation_state_data.clone())
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

        let pet_atlas = Arc::new(
            Atlas::new_from_file("spineAssets/pets/skeleton.atlas")
                .map_err(|e| format!("Pet atlas: {e:?}"))?,
        );
        let pet_pma = pet_atlas.pages().any(|p| p.pma());
        let pet_data = Arc::new(
            SkeletonBinary::new(pet_atlas)
                .read_skeleton_data_file("spineAssets/pets/skeleton.skel.bytes")
                .map_err(|e| format!("Pet skeleton: {e:?}"))?,
        );
        let pet_state = Arc::new(AnimationStateData::new(pet_data.clone()));
        let assets = Arc::new(Assets {
            data: skeleton_data,
            state: animation_state_data,
            pet_data,
            pet_state,
            pet_pma,
        });
        let pet = Self::new_pet(&assets);
        let renderer = SpineRenderer::new(premultiplied_alpha, config.backface_culling);
        let id = crate::persistence::new_id();

        let mut character = Self {
            config,
            controller,
            pet,
            preview_platform: None,
            preview_scale: None,
            renderer,
            bounds,
            animation_names: Arc::new(animation_names),
            wardrobe,
            assets,
            id: id.clone(),
            level: 0,
            animation: AnimationData::default(),
            placement: Placement {
                character_id: id,
                ..Default::default()
            },
            animation_runtime: AnimationRuntime::default(),
            pet_signature: String::new(),
        };
        character.configure_animation(true)?;
        Ok(character)
    }

    fn new_pet(assets: &Assets) -> SkeletonController {
        SkeletonController::new(assets.pet_data.clone(), assets.pet_state.clone()).with_settings(
            SkeletonControllerSettings {
                premultiplied_alpha: assets.pet_pma,
                cull_direction: CullDirection::CounterClockwise,
                color_space: ColorSpace::SRGB,
            },
        )
    }

    pub fn duplicate(&self) -> Result<Self, String> {
        let mut wardrobe = self.wardrobe.fork();
        let mut controller =
            SkeletonController::new(self.assets.data.clone(), self.assets.state.clone())
                .with_settings(self.controller.settings.clone());
        wardrobe.apply(&mut controller)?;
        let id = crate::persistence::new_id();
        let mut placement = self.placement.clone();
        placement.character_id = id.clone();
        let mut result = Self {
            config: self.config,
            controller,
            pet: Self::new_pet(&self.assets),
            preview_platform: None,
            preview_scale: None,
            renderer: SpineRenderer::new(
                self.controller.settings.premultiplied_alpha,
                self.config.backface_culling,
            ),
            bounds: self.bounds,
            animation_names: self.animation_names.clone(),
            wardrobe,
            assets: self.assets.clone(),
            id,
            level: self.level,
            animation: self.animation.clone(),
            placement,
            animation_runtime: AnimationRuntime::default(),
            pet_signature: String::new(),
        };
        result.configure_animation(true)?;
        result.update(0.0);
        Ok(result)
    }

    /// Preview shares appearance/resources, never scene transforms or playback.
    pub fn preview_copy(&self) -> Result<Self, String> {
        let mut preview = self.duplicate()?;
        preview.placement = Placement {
            character_id: preview.id.clone(),
            ..Placement::default()
        };
        preview.set_action(crate::model::Action::Idle);
        let atlas = Arc::new(
            Atlas::new_from_file("spineAssets/spring_platform/skeleton.atlas")
                .map_err(|e| format!("Preview platform atlas: {e:?}"))?,
        );
        let pma = atlas.pages().any(|p| p.pma());
        let data = Arc::new(
            SkeletonBinary::new(atlas)
                .read_skeleton_data_file("spineAssets/spring_platform/skeleton.skel.bytes")
                .map_err(|e| format!("Preview platform skeleton: {e:?}"))?,
        );
        let state = Arc::new(AnimationStateData::new(data.clone()));
        let mut platform =
            SkeletonController::new(data, state).with_settings(SkeletonControllerSettings {
                premultiplied_alpha: pma,
                cull_direction: CullDirection::CounterClockwise,
                color_space: ColorSpace::SRGB,
            });
        platform
            .animation_state
            .set_animation_by_name(0, "idle", true)
            .map_err(|e| format!("Preview platform animation: {e:?}"))?;
        platform.update(0.0, Physics::Update);
        preview.preview_platform = Some(platform);
        preview.update(0.0);
        Ok(preview)
    }

    pub fn data(&self) -> CharacterData {
        CharacterData {
            id: self.id.clone(),
            animal: self.wardrobe.characters[self.wardrobe.current_character]
                .inventory_id
                .clone(),
            outfits: std::array::from_fn(|i| {
                self.wardrobe.items[i][self.wardrobe.selected[i]]
                    .inventory_id
                    .clone()
            }),
            equipment: self.wardrobe.equipment.clone(),
            level: self.level,
            animation: self.animation.clone(),
        }
    }

    pub fn restore(&mut self, data: &CharacterData, placement: Placement) -> Result<(), String> {
        self.wardrobe.equipment_catalog.validate(&data.equipment)?;
        let animal = self
            .wardrobe
            .characters
            .iter()
            .position(|c| c.inventory_id == data.animal)
            .ok_or_else(|| format!("Unknown animal {}", data.animal))?;
        let mut selected = [0; 5];
        for (i, id) in data.outfits.iter().enumerate() {
            selected[i] = self.wardrobe.items[i]
                .iter()
                .position(|c| c.inventory_id == *id)
                .ok_or_else(|| format!("Unknown outfit {id}"))?;
        }
        self.wardrobe.current_character = animal;
        self.wardrobe.selected = selected;
        self.wardrobe.equipment = data.equipment.clone();
        self.wardrobe
            .equipment_catalog
            .initialize_loadout(&mut self.wardrobe.equipment);
        self.wardrobe.action = data.animation.action;
        self.wardrobe.apply(&mut self.controller)?;
        self.id = data.id.clone();
        self.level = data.level;
        self.animation = data.animation.clone();
        self.placement = placement;
        self.configure_animation(true)?;
        self.update(0.0);
        Ok(())
    }

    fn configure_animation(&mut self, force: bool) -> Result<(), String> {
        // The developer preset owns this index, including when restoring older saves.
        self.animation.control_track = self.animation.action.control_track();
        // Validate the pet before mutating either animation controller.
        if let Some(pet) = self
            .wardrobe
            .equipment_catalog
            .pet(&self.wardrobe.equipment)
        {
            if self.assets.pet_data.find_skin(&pet.skin).is_none() {
                return Err(format!("Unknown pet skin {}", pet.skin));
            }
        }
        self.animation_runtime.configure(
            &mut self.controller,
            &self.animation,
            &self.wardrobe.equipment_catalog,
            &self.wardrobe.equipment,
            force,
        )?;
        self.wardrobe.action = self.animation.action;
        self.configure_pet(force)?;
        Ok(())
    }

    fn configure_pet(&mut self, force: bool) -> Result<(), String> {
        let catalog = &self.wardrobe.equipment_catalog;
        let Some(pet) = catalog.pet(&self.wardrobe.equipment) else {
            self.pet_signature.clear();
            self.animation.pet_animation = None;
            return Ok(());
        };
        let emote = catalog.emote(&self.wardrobe.equipment);
        let has_emote = self
            .animation
            .tracks
            .iter()
            .any(|t| t.token.starts_with("emotes/"));
        let base = if pet.anim_type == 1 {
            "chicken"
        } else {
            "mini-quadruped"
        };
        let sleep = has_emote
            && emote.is_some_and(|e| matches!(e.id.as_str(), "EmoteNoddingOff" | "EmoteSleep"));
        let clip = if let Some(clip) = emote
            .and_then(|e| e.pet_anim.as_deref())
            .filter(|clip| has_emote && !clip.is_empty())
        {
            clip
        } else if sleep {
            "sleep"
        } else {
            "idle"
        };
        let mut animation = format!("{base}/{clip}");
        if self.assets.pet_data.find_animation(&animation).is_none() {
            animation = format!("{base}/idle");
        }
        if force
            && self
                .animation
                .pet_animation
                .as_ref()
                .is_some_and(|saved| saved.clip != animation)
        {
            return Err("Saved pet animation does not match equipment".into());
        }
        let signature = format!("{}@{animation}", pet.skin);
        if !force && self.pet_signature == signature {
            return Ok(());
        }
        self.pet
            .skeleton
            .set_skin_by_name(&pet.skin)
            .map_err(|e| format!("{e:?}"))?;
        self.pet.skeleton.set_to_setup_pose();
        self.pet.animation_state.clear_tracks();
        let mut entry = self
            .pet
            .animation_state
            .set_animation_by_name(0, &animation, true)
            .map_err(|e| format!("{e:?}"))?;
        if force {
            if let Some(saved) = self
                .animation
                .pet_animation
                .as_ref()
                .filter(|p| p.clip == animation)
            {
                entry.set_track_time(saved.time);
            }
        }
        entry.set_mix_duration(0.0);
        self.pet_signature = signature;
        Ok(())
    }

    pub fn update(&mut self, delta_seconds: f32) {
        if let Some(platform) = &mut self.preview_platform {
            platform.update(delta_seconds, Physics::Update);
        }
        self.animation_runtime
            .update(&mut self.controller, &mut self.animation, delta_seconds);
        if let Some(mut target) = self.controller.skeleton.find_bone_mut("TARGET") {
            let rad = self.placement.target_angle.to_radians()
                * if self.placement.face_right { -1.0 } else { 1.0 };
            target.set_x(rad.cos() * 447.0);
            target.set_y(219.0 + rad.sin() * 447.0);
        }
        let [r, g, b, a] = self.placement.color;
        self.controller
            .skeleton
            .color_mut()
            .set_from_floats(r, g, b, a);
        self.wardrobe.apply_visibility(&mut self.controller);
        self.controller
            .skeleton
            .update_world_transform(Physics::Update);
        self.pet.skeleton.color_mut().set_from_floats(r, g, b, a);
        self.pet.update(
            if self.animation.paused {
                0.0
            } else {
                delta_seconds
            },
            Physics::Update,
        );
        self.animation.pet_animation = if self.has_pet() {
            self.pet
                .animation_state
                .get_current(0)
                .map(|entry| crate::model::PetAnimationData {
                    clip: entry.animation().name().into(),
                    time: entry.track_time(),
                })
        } else {
            None
        };
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

    /// Use the previous Preview size, shrinking only to keep attachments visible.
    pub fn draw(
        &mut self,
        gem_meta: Option<&crate::booth::SpriteMeta>,
    ) -> Option<(macroquad::prelude::Rect, f32)> {
        self.draw_faded(gem_meta, 1.0)
    }

    pub fn draw_faded(
        &mut self,
        gem_meta: Option<&crate::booth::SpriteMeta>,
        opacity: f32,
    ) -> Option<(macroquad::prelude::Rect, f32)> {
        let (screen_width, screen_height) = (screen_width(), screen_height());
        let (gui_w, gui_h, gui_scale) = crate::gui_scale::dimensions(screen_width, screen_height);
        let (preview, _) = crate::gui_scale::regions(macroquad::prelude::vec2(gui_w, gui_h));
        let mut renderables = self.controller.combined_renderables();
        let world = self.world_matrix(1.0);
        let reference = self.bounds.transformed(world);
        let mut bounds = reference;
        // Unity uses SkeletonData scale 0.01 and Transform scale 0.9.
        // Keep the platform in the same framing as the character, at its feet.
        let platform_world = Mat4::from_translation(Vec3::new(
            self.placement.position[0],
            self.placement.height - 0.10 * self.placement.scale,
            0.0,
        )) * Mat4::from_scale(Vec3::splat(0.009 * self.placement.scale));
        let mut platform_renderables = self
            .preview_platform
            .as_mut()
            .map(|p| p.combined_renderables())
            .unwrap_or_default();
        if let Some(current) = bounds_of(&renderables) {
            bounds.include(current.transformed(world));
        }
        let pet_world = self.pet_matrix(self.pet_position(1.0), 1.0);
        let mut pet_renderables = if self.has_pet() {
            self.pet.combined_renderables()
        } else {
            Vec::new()
        };
        if let Some(pet_bounds) = bounds_of(&pet_renderables) {
            bounds.include(pet_bounds.transformed(pet_world));
        }
        let gem = self
            .gem(1.0)
            .zip(gem_meta)
            .map(|((pos, scale, alpha), meta)| {
                let width = meta.size[0] as f32 / meta.pixels_per_unit * scale[0].abs();
                let height = meta.size[1] as f32 / meta.pixels_per_unit * scale[1].abs();
                let gem_bounds = Bounds {
                    min_x: pos[0] - width * meta.pivot[0],
                    max_x: pos[0] + width * (1.0 - meta.pivot[0]),
                    min_y: pos[1] - height * meta.pivot[1],
                    max_y: pos[1] + height * (1.0 - meta.pivot[1]),
                };
                bounds.include(gem_bounds);
                (gem_bounds, alpha)
            });
        // Fade generated vertices without changing the entity's persistent color.
        // Premultiplied-alpha meshes also need their RGB faded.
        for mesh in renderables
            .iter_mut()
            .chain(pet_renderables.iter_mut())
            .chain(platform_renderables.iter_mut())
        {
            for color in &mut mesh.colors {
                color[3] *= opacity;
                if mesh.premultiplied_alpha {
                    for channel in &mut color[..3] {
                        *channel *= opacity;
                    }
                }
            }
        }
        let (framing, scale) = preview_framing(
            reference,
            bounds,
            [screen_width, screen_height],
            preview,
            gui_scale,
            crate::gui_scale::compact(),
        );
        // Frame-rate-independent easing, anchored at the feet so zooming does
        // not make the character slide. Initialize directly on first display.
        let previous_scale = self.preview_scale.unwrap_or(scale);
        let blend = 1.0 - (-2.0_f32 * macroquad::prelude::get_frame_time().min(0.1)).exp();
        let smoothed_scale = previous_scale + (scale - previous_scale) * blend;
        self.preview_scale = Some(smoothed_scale);
        let anchor = framing.transform_point3(Vec3::new(
            (reference.min_x + reference.max_x) * 0.5,
            reference.min_y,
            0.0,
        ));
        let framing = Mat4::from_translation(anchor)
            * Mat4::from_scale(Vec3::splat(smoothed_scale / scale.max(0.000001)))
            * Mat4::from_translation(-anchor)
            * framing;
        let scale = smoothed_scale;
        let offset = crate::gui::widgets::paint::offset();
        let shifted_framing =
            Mat4::from_translation(Vec3::new(offset.x * gui_scale, -offset.y * gui_scale, 0.0))
                * framing;
        let view = Mat4::orthographic_rh_gl(
            -screen_width * 0.5,
            screen_width * 0.5,
            -screen_height * 0.5,
            screen_height * 0.5,
            -1.0,
            1.0,
        );

        let physical = macroquad::miniquad::window::screen_size();
        let sx = physical.0 / screen_width;
        let sy = physical.1 / screen_height;
        let clip = Some([
            ((preview.x + offset.x) * gui_scale * sx) as i32,
            ((preview.y + offset.y) * gui_scale * sy) as i32,
            (preview.w * gui_scale * sx) as i32,
            (preview.h * gui_scale * sy) as i32,
        ]);
        let viewport = [physical.0 as i32, physical.1 as i32];
        self.renderer.draw_to(
            view * shifted_framing,
            platform_world,
            &platform_renderables,
            None,
            viewport,
            clip,
        );
        self.renderer.draw_to(
            view * shifted_framing,
            world,
            &renderables,
            None,
            viewport,
            clip,
        );
        if self.has_pet() {
            self.renderer.draw_to(
                view * shifted_framing,
                pet_world,
                &pet_renderables,
                None,
                viewport,
                clip,
            );
        }
        gem.map(|(b, alpha)| {
            let top_left = framing.transform_point3(Vec3::new(b.min_x, b.max_y, 0.0));
            (
                macroquad::prelude::Rect::new(
                    (top_left.x + screen_width * 0.5) / gui_scale,
                    (screen_height * 0.5 - top_left.y) / gui_scale,
                    (b.max_x - b.min_x) * scale / gui_scale,
                    (b.max_y - b.min_y) * scale / gui_scale,
                ),
                alpha,
            )
        })
    }

    pub fn cycle_animation(&mut self, direction: isize) {
        let i = Action::ALL
            .iter()
            .position(|a| *a == self.animation.action)
            .unwrap_or(0);
        let next = (i as isize + direction).rem_euclid(Action::ALL.len() as isize) as usize;
        self.set_action(Action::ALL[next]);
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
        self.animation.action.label()
    }

    pub fn animation_count(&self) -> usize {
        Action::ALL.len()
    }

    pub fn animation_index(&self) -> usize {
        Action::ALL
            .iter()
            .position(|a| *a == self.animation.action)
            .unwrap_or(0)
            + 1
    }

    pub fn set_action(&mut self, action: Action) {
        let old = self.animation.clone();
        self.animation = AnimationData::preset(action);
        if let Err(error) = self.configure_animation(true) {
            self.animation = old;
            if let Err(rollback) = self.configure_animation(true) {
                tracing::error!("Cannot restore action: {rollback}");
            }
            tracing::warn!("Cannot set action: {error}");
        }
        self.update(0.0);
    }
    pub fn choose_equipment(&mut self, category: usize, index: usize) -> Result<(), String> {
        let previous = self.wardrobe.equipment.clone();
        self.wardrobe
            .equipment_catalog
            .choose(&mut self.wardrobe.equipment, category, index);
        let result = self
            .wardrobe
            .apply(&mut self.controller)
            .and_then(|_| self.configure_animation(false));
        if let Err(error) = result {
            self.wardrobe.equipment = previous;
            self.wardrobe.apply(&mut self.controller)?;
            self.configure_animation(false)?;
            return Err(error);
        }
        self.update(0.0);
        Ok(())
    }
    /// Preview is a separate entity. These selections only drive its try-on pose.
    pub fn preview_category(&mut self, category: usize) {
        use crate::equipment::CategoryKind;
        let catalog = self.wardrobe.equipment_catalog.clone();
        let Some(c) = catalog.categories.get(category) else {
            return;
        };
        let equipment = &mut self.wardrobe.equipment;
        let action = match c.kind {
            CategoryKind::Weapon => {
                let selected = catalog.selected(equipment, category);
                equipment.weapon = c.entries[selected]
                    .weapon
                    .map(|w| catalog.weapons[w].id.clone());
                Action::Idle
            }
            CategoryKind::Emote => {
                equipment.emote = Some(catalog.configured_emote(equipment).to_owned());
                Action::Emote
            }
            CategoryKind::Umbrella => Action::Umbrella,
            CategoryKind::Pet => Action::Idle,
        };
        if let Err(error) = self.wardrobe.apply(&mut self.controller) {
            tracing::warn!("Cannot preview equipment: {error}");
        }
        self.set_action(action);
    }

    /// Copy appearance/configuration while retaining the booth's actual choices and tracks.
    pub fn apply_preview(&mut self, preview: &Character) -> Result<(), String> {
        let before = self.data();
        let mut equipment = preview.wardrobe.equipment.clone();
        equipment.weapon = before.equipment.weapon.clone();
        equipment.emote = before.equipment.emote.clone();
        if self.wardrobe.current_character == preview.wardrobe.current_character
            && self.wardrobe.selected == preview.wardrobe.selected
            && self.wardrobe.equipment == equipment
            && self.level == preview.level
        {
            return Ok(());
        }
        // A held melee slot follows its configured appearance without selecting that slot.
        if self
            .wardrobe
            .equipment_catalog
            .weapon(&before.equipment)
            .is_some_and(|weapon| weapon.weapon_class == "Melee")
        {
            equipment.weapon = self
                .wardrobe
                .equipment_catalog
                .configured_melee(&equipment)
                .map(str::to_owned);
        }
        self.wardrobe.current_character = preview.wardrobe.current_character;
        self.wardrobe.selected = preview.wardrobe.selected;
        self.wardrobe.equipment = equipment;
        self.level = preview.level;
        let result = self
            .wardrobe
            .apply(&mut self.controller)
            .and_then(|_| self.configure_animation(false));
        if let Err(error) = result {
            self.restore(&before, self.placement.clone())?;
            return Err(error);
        }
        self.update(0.0);
        Ok(())
    }

    pub fn use_weapon(&mut self, id: Option<&str>) -> Result<(), String> {
        let catalog = &self.wardrobe.equipment_catalog;
        if id.is_some_and(|id| {
            !catalog
                .booth_weapons(&self.wardrobe.equipment)
                .iter()
                .any(|e| e.id == id)
        }) {
            return Err("Weapon is not in the configured booth loadout".into());
        }
        let before = self.data();
        self.wardrobe.equipment.weapon = id.map(str::to_owned);
        if !self.animation.action.weapons_visible() {
            let paused = self.animation.paused;
            self.animation = AnimationData::preset(Action::Idle);
            self.animation.paused = paused;
        }
        let result = self
            .wardrobe
            .apply(&mut self.controller)
            .and_then(|_| self.configure_animation(false));
        if let Err(error) = result {
            self.restore(&before, self.placement.clone())?;
            return Err(error);
        }
        self.update(0.0);
        Ok(())
    }

    pub fn use_emote(&mut self, id: &str) -> Result<(), String> {
        if !self
            .wardrobe
            .equipment_catalog
            .emotes
            .iter()
            .any(|emote| emote.id == id)
        {
            return Err("Unknown booth emote".into());
        }
        let before = self.data();
        let paused = self.animation.paused;
        self.wardrobe.equipment.emote = Some(id.to_owned());
        self.animation = AnimationData::preset(Action::Emote);
        self.animation.paused = paused;
        if let Err(error) = self.configure_animation(true) {
            self.restore(&before, self.placement.clone())?;
            return Err(error);
        }
        self.update(0.0);
        Ok(())
    }
    pub fn seek(&mut self, time: f32) {
        self.animation_runtime
            .seek(&mut self.controller, &mut self.animation, time);
        self.update(0.0);
    }
    pub fn duration(&self) -> f32 {
        self.animation_runtime
            .duration(&self.controller, self.animation.control_track)
    }

    pub fn world_matrix(&self, perspective: f32) -> Mat4 {
        let p = &self.placement;
        Mat4::from_translation(Vec3::new(p.position[0], p.height, 0.0))
            * Mat4::from_rotation_z(p.rotation.to_radians() * if p.face_right { -1.0 } else { 1.0 })
            * Mat4::from_scale(
                Vec3::new(if p.face_right { 1.0 } else { -1.0 }, 1.0, 1.0)
                    * (0.009 * p.scale * perspective),
            )
    }
    pub fn pet_position(&self, perspective: f32) -> [f32; 2] {
        let p = &self.placement;
        if p.pet_reference == ReferenceFrame::World {
            p.pet_position
        } else {
            [
                p.position[0] + p.pet_position[0] * perspective,
                p.position[1] + p.pet_position[1] * perspective,
            ]
        }
    }
    pub fn has_pet(&self) -> bool {
        !self.pet_signature.is_empty()
    }
    pub fn draw_world(
        &mut self,
        view: Mat4,
        perspective: f32,
        target: &macroquad::prelude::RenderTarget,
    ) {
        let renderables = self.controller.combined_renderables();
        self.renderer.draw_to(
            view,
            self.world_matrix(perspective),
            &renderables,
            Some(target.render_pass.raw_miniquad_id()),
            [
                target.texture.width() as i32,
                target.texture.height() as i32,
            ],
            None,
        );
    }
    fn pet_matrix(&self, position: [f32; 2], perspective: f32) -> Mat4 {
        Mat4::from_translation(Vec3::new(position[0], position[1], 0.0))
            * Mat4::from_scale(
                Vec3::new(
                    if self.placement.pet_face_right {
                        1.0
                    } else {
                        -1.0
                    },
                    1.0,
                    1.0,
                ) * (0.0072 * perspective),
            )
    }
    pub fn draw_pet(
        &mut self,
        view: Mat4,
        position: [f32; 2],
        perspective: f32,
        target: &macroquad::prelude::RenderTarget,
    ) {
        if !self.has_pet() {
            return;
        }
        let world = self.pet_matrix(position, perspective);
        let renderables = self.pet.combined_renderables();
        self.renderer.draw_to(
            view,
            world,
            &renderables,
            Some(target.render_pass.raw_miniquad_id()),
            [
                target.texture.width() as i32,
                target.texture.height() as i32,
            ],
            None,
        );
    }
    /// Position and bone scale for the external level gem (unrotated sibling sprite).
    pub fn gem(&self, perspective: f32) -> Option<([f32; 2], [f32; 2], f32)> {
        if self.animation.action != Action::Emote {
            return None;
        }
        let alpha = self
            .controller
            .skeleton
            .find_slot("gem_template")?
            .color()
            .a;
        if alpha <= 0.0 {
            return None;
        }
        let bone = self.controller.skeleton.find_bone("weapon_free")?;
        let world = self.world_matrix(perspective).transform_point3(Vec3::new(
            bone.world_x(),
            bone.world_y(),
            0.0,
        ));
        Some((
            [world.x, world.y],
            [
                bone.scale_x() * self.placement.scale * perspective,
                bone.scale_y() * self.placement.scale * perspective,
            ],
            alpha,
        ))
    }
}

/// Replace the whole action, as the Unity controller does when its action changes.
/// Clearing tracks alone leaves the last applied pose on the skeleton. Reset that pose as
/// well, so properties absent from the new animation cannot retain values from the old one.
#[cfg(test)]
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

    #[test]
    fn preview_preserves_previous_size_and_foot_anchor_without_upscaling() {
        let reference = Bounds {
            min_x: -1.0,
            max_x: 1.0,
            min_y: -3.0,
            max_y: 1.0,
        };
        let preview = macroquad::prelude::Rect::new(260.0, 100.0, 560.0, 550.0);
        let (matrix, scale) =
            preview_framing(reference, reference, [1280.0, 720.0], preview, 1.0, false);
        assert!((scale * 4.0 - 720.0 * 0.42).abs() < 0.001);
        let foot = matrix.transform_point3(Vec3::new(0.0, -3.0, 0.0));
        assert!((foot.x + 640.0 - 540.0).abs() < 0.001);
        assert!((360.0 - foot.y - 576.0).abs() < 0.001);
        let content = Bounds {
            max_y: 8.0,
            ..reference
        };
        let (_, with_accessory) =
            preview_framing(reference, content, [1280.0, 720.0], preview, 1.0, false);
        assert!(with_accessory < scale);
    }

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
