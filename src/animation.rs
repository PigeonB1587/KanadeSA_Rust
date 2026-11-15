//! Multi-track action resolution and deterministic restore/seek.
use crate::{
    equipment::{Catalog, Emote, Weapon},
    model::{AnimationData, EquipmentData, MAX_TRACKS},
};
use rusty_spine::{Physics, controller::SkeletonController};

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTrack {
    pub clip: Option<String>,
    pub initial: Option<String>,
    pub alternate: Option<String>,
    pub interval: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Action;
    use rusty_spine::{AnimationStateData, Atlas, SkeletonBinary};
    use std::sync::Arc;

    fn controller() -> SkeletonController {
        let atlas = Arc::new(Atlas::new_from_file(crate::assets::CHARACTER_ATLAS).unwrap());
        let data = Arc::new(
            SkeletonBinary::new(atlas)
                .read_skeleton_data_file(crate::assets::CHARACTER_SKELETON)
                .unwrap(),
        );
        SkeletonController::new(data.clone(), Arc::new(AnimationStateData::new(data)))
    }

    #[test]
    fn weapon_changes_preserve_base_track_and_invalid_edits_leave_runtime_intact() {
        let mut controller = controller();
        let catalog = Catalog::load().unwrap();
        let mut equipment = EquipmentData::default();
        let mut data = AnimationData::preset(Action::Run);
        let mut runtime = AnimationRuntime::default();
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, true)
            .unwrap();
        runtime.update(&mut controller, &mut data, 0.37);
        let base = data.tracks[0].clone();
        equipment.weapon = Some(
            catalog
                .weapons
                .iter()
                .find(|w| w.spine_aim_anim_key.is_some())
                .unwrap()
                .id
                .clone(),
        );
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, false)
            .unwrap();
        runtime.update(&mut controller, &mut data, 0.0);
        assert_eq!(data.tracks[0], base);
        assert_eq!(data.tracks[1].time, 0.0);
        assert!(data.tracks[1].current_clip.is_some());
        let before = data.clone();
        data.tracks[0].token = "missing/animation".into();
        assert!(
            runtime
                .configure(&mut controller, &data, &catalog, &equipment, false)
                .is_err()
        );
        let mut invalid_restore = before.clone();
        invalid_restore.tracks[0].current_clip = Some("idle/idle".into());
        assert!(
            runtime
                .configure(
                    &mut controller,
                    &invalid_restore,
                    &catalog,
                    &equipment,
                    true
                )
                .is_err()
        );
        assert_eq!(
            controller
                .animation_state
                .get_current(0)
                .unwrap()
                .track_time(),
            base.time
        );
        data = before;
        equipment.weapon = None;
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, false)
            .unwrap();
        runtime.update(&mut controller, &mut data, 0.0);
        assert_eq!(data.tracks[0], base);
        assert_eq!(data.tracks[1].time, 0.0);
        assert!(data.tracks[1].current_clip.is_none());
    }

    #[test]
    fn initial_transition_seek_pause_speed_loop_and_restore_are_deterministic() {
        let mut controller = controller();
        let catalog = Catalog::load().unwrap();
        let equipment = EquipmentData::default();
        let mut data = AnimationData::default();
        data.tracks.truncate(1);
        data.tracks[0].token = "misc/backpack_open".into();
        data.tracks[0].speed = 2.0;
        data.tracks[0].looping = false;
        let mut runtime = AnimationRuntime::default();
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, true)
            .unwrap();
        let duration = runtime.duration(&controller, 0);
        runtime.update(&mut controller, &mut data, duration * 0.25);
        assert_eq!(
            data.tracks[0].current_clip.as_deref(),
            Some("misc/backpack_open")
        );
        let saved = data.clone();
        runtime.update(&mut controller, &mut data, duration);
        data = saved;
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, true)
            .unwrap();
        data.paused = true;
        runtime.seek(&mut controller, &mut data, duration * 0.75);
        runtime.update(&mut controller, &mut data, 1.0);
        assert_eq!(
            data.tracks[0].current_clip.as_deref(),
            Some("misc/backpack_open")
        );
        assert!((data.tracks[0].time - duration * 0.75).abs() < 0.0001);
        data.paused = false;
        runtime.update(&mut controller, &mut data, duration * 0.25);
        assert_eq!(
            data.tracks[0].current_clip.as_deref(),
            Some("misc/backpack_loop")
        );
        assert!((data.tracks[0].time - duration * 0.25).abs() < 0.0001);
        assert!(!controller.animation_state.get_current(0).unwrap().looping());
        let saved = data.clone();
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, true)
            .unwrap();
        runtime.update(&mut controller, &mut data, 0.0);
        assert_eq!(data, saved);
        // Explicit emote paths must not inherit the selected emote's initial clip.
        let emote = catalog
            .emotes
            .iter()
            .find(|e| e.emote_spine_key_initial.is_some())
            .unwrap();
        assert!(
            resolve(&emote.emote_spine_key, None, Some(emote))
                .initial
                .is_none()
        );
        assert_eq!(
            resolve("emotes/*", None, Some(emote)).initial,
            emote.emote_spine_key_initial
        );
    }

    #[test]
    fn alternate_fire_keeps_frame_remainder_and_respects_pause_and_loop_toggle() {
        let mut controller = controller();
        let catalog = Catalog::load().unwrap();
        let weapon = catalog
            .weapons
            .iter()
            .find(|w| w.spine_atk_alternate_anim_key.is_some())
            .unwrap();
        let equipment = EquipmentData {
            weapon: Some(weapon.id.clone()),
            ..Default::default()
        };
        let mut data = AnimationData::preset(Action::Run);
        data.tracks[1].token = "attacks/*".into();
        let mut runtime = AnimationRuntime::default();
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, true)
            .unwrap();
        let first = weapon.spine_atk_anim_key.as_ref().unwrap();
        let second = weapon.spine_atk_alternate_anim_key.as_ref().unwrap();
        let period_a = controller
            .skeleton
            .data()
            .find_animation(first)
            .unwrap()
            .duration()
            + weapon.shot_interval;
        let period_b = controller
            .skeleton
            .data()
            .find_animation(second)
            .unwrap()
            .duration()
            + weapon.shot_interval;
        let remainder = period_b * 0.3;
        runtime.update(
            &mut controller,
            &mut data,
            (period_a + period_b) * 3.0 + period_a + remainder,
        );
        assert_eq!(data.tracks[1].current_clip.as_ref(), Some(second));
        assert!((data.tracks[1].time - remainder).abs() < 0.0001);
        data.paused = true;
        let saved = data.clone();
        runtime
            .configure(&mut controller, &data, &catalog, &equipment, true)
            .unwrap();
        runtime.update(&mut controller, &mut data, 8.0);
        assert_eq!(data, saved);
        data.paused = false;
        data.tracks[1].looping = false;
        runtime.update(&mut controller, &mut data, period_b * 2.0);
        assert_eq!(data.tracks[1].current_clip.as_ref(), Some(second));
        data.tracks[1].looping = true;
        runtime.update(&mut controller, &mut data, period_a * 0.1);
        assert_eq!(data.tracks[1].current_clip.as_ref(), Some(second));
        assert!((data.tracks[1].time - remainder - period_a * 0.1).abs() < 0.0001);
    }

    #[test]
    fn all_presets_weapon_tokens_and_emote_initials_resolve_on_actual_skeleton() {
        let atlas = Arc::new(Atlas::new_from_file(crate::assets::CHARACTER_ATLAS).unwrap());
        let data = Arc::new(
            SkeletonBinary::new(atlas)
                .read_skeleton_data_file(crate::assets::CHARACTER_SKELETON)
                .unwrap(),
        );
        let mut controller =
            SkeletonController::new(data.clone(), Arc::new(AnimationStateData::new(data)));
        let catalog = Catalog::load().unwrap();
        let mut e = EquipmentData::default();
        let mut runtime = AnimationRuntime::default();
        for action in Action::ALL {
            runtime
                .configure(
                    &mut controller,
                    &AnimationData::preset(action),
                    &catalog,
                    &e,
                    true,
                )
                .unwrap();
        }
        let mut failures = vec![];
        for weapon in &catalog.weapons {
            e.weapon = Some(weapon.id.clone());
            for token in ["aim/*", "attacks/*", "reload/*"] {
                let mut a = AnimationData::default();
                a.tracks[1].token = token.into();
                if let Err(error) = runtime.configure(&mut controller, &a, &catalog, &e, true) {
                    failures.push(format!("{}: {error}", weapon.id));
                }
            }
        }
        for emote in &catalog.emotes {
            e.emote = Some(emote.id.clone());
            if let Err(error) = runtime.configure(
                &mut controller,
                &AnimationData::preset(Action::Emote),
                &catalog,
                &e,
                true,
            ) {
                failures.push(format!("{}: {error}", emote.id));
            }
        }
        assert!(
            failures.is_empty(),
            "{} animation failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
    #[test]
    fn appearance_reapply_preserves_tracks_and_seek_restores_pose() {
        let atlas = Arc::new(Atlas::new_from_file(crate::assets::CHARACTER_ATLAS).unwrap());
        let data = Arc::new(
            SkeletonBinary::new(atlas)
                .read_skeleton_data_file(crate::assets::CHARACTER_SKELETON)
                .unwrap(),
        );
        let mut controller =
            SkeletonController::new(data.clone(), Arc::new(AnimationStateData::new(data)));
        let catalog = Catalog::load().unwrap();
        let e = EquipmentData::default();
        let mut a = AnimationData::preset(Action::Run);
        let mut runtime = AnimationRuntime::default();
        runtime
            .configure(&mut controller, &a, &catalog, &e, true)
            .unwrap();
        runtime.update(&mut controller, &mut a, 0.35);
        let time = controller
            .animation_state
            .get_current(0)
            .unwrap()
            .track_time();
        runtime
            .configure(&mut controller, &a, &catalog, &e, false)
            .unwrap();
        assert_eq!(
            controller
                .animation_state
                .get_current(0)
                .unwrap()
                .track_time(),
            time
        );
        a.paused = true;
        runtime.update(&mut controller, &mut a, 1.0);
        assert_eq!(
            controller
                .animation_state
                .get_current(0)
                .unwrap()
                .track_time(),
            time
        );
        runtime.seek(&mut controller, &mut a, 0.1);
        assert!(
            (controller
                .animation_state
                .get_current(0)
                .unwrap()
                .track_time()
                - 0.1)
                .abs()
                < 0.0001
        );
        assert!(
            controller
                .skeleton
                .bones()
                .all(|b| b.world_x().is_finite() && b.world_y().is_finite())
        );
    }
}

fn resolve(token: &str, weapon: Option<&Weapon>, emote: Option<&Emote>) -> ResolvedTrack {
    let clip = match token {
        "aim/*" => weapon.and_then(|w| w.spine_aim_anim_key.clone()),
        "attacks/*" => weapon.and_then(|w| w.spine_atk_anim_key.clone()),
        "reload/*" => weapon.and_then(|w| w.reload_spine_anim_key.clone()),
        "emotes/*" => emote.map(|e| e.emote_spine_key.clone()),
        "misc/backpack_open" => Some("misc/backpack_loop".into()),
        "misc/open_crate" => Some("misc/open_crate_loop".into()),
        "" => None,
        _ => Some(token.into()),
    };
    let initial = match token {
        "misc/backpack_open" | "misc/open_crate" => Some(token.into()),
        "emotes/*" => emote.and_then(|e| e.emote_spine_key_initial.clone()),
        _ => None,
    };
    let alternate = if clip
        .as_deref()
        .is_some_and(|s| s.starts_with("attacks/fire"))
    {
        weapon.and_then(|w| w.spine_atk_alternate_anim_key.clone())
    } else {
        None
    };
    let interval = if alternate.is_some() {
        weapon.map_or(0.0, |w| w.shot_interval)
    } else {
        0.0
    };
    ResolvedTrack {
        clip,
        initial,
        alternate,
        interval,
    }
}

#[derive(Default)]
pub struct AnimationRuntime {
    resolved: Vec<ResolvedTrack>,
}
impl AnimationRuntime {
    pub fn resolve(
        data: &AnimationData,
        catalog: &Catalog,
        equipment: &EquipmentData,
    ) -> Vec<ResolvedTrack> {
        let emote = catalog.emote(equipment);
        data.tracks
            .iter()
            .enumerate()
            .take(MAX_TRACKS)
            .map(|(i, t)| {
                if t.token == "run/run_emotes"
                    && data
                        .tracks
                        .get(i + 1)
                        .is_some_and(|next| next.token.starts_with("emotes/"))
                {
                    if let Some(walk) = emote.and_then(|e| e.emote_spine_key_walk.as_deref()) {
                        return resolve(walk, catalog.weapon(equipment), emote);
                    }
                }
                resolve(&t.token, catalog.weapon(equipment), emote)
            })
            .collect()
    }
    pub fn configure(
        &mut self,
        controller: &mut SkeletonController,
        data: &AnimationData,
        catalog: &Catalog,
        equipment: &EquipmentData,
        force: bool,
    ) -> Result<(), String> {
        let resolved = Self::resolve(data, catalog, equipment);
        if !force && self.resolved == resolved {
            return Ok(());
        }
        // Validate every explicit clip before clearing any existing track.
        for (r, track) in resolved.iter().zip(&data.tracks) {
            for clip in [&r.clip, &r.initial, &r.alternate].into_iter().flatten() {
                if controller.skeleton.data().find_animation(clip).is_none() {
                    return Err(format!("Unknown animation {clip}"));
                }
            }
            if force
                && track.current_clip.as_ref().is_some_and(|clip| {
                    [&r.clip, &r.initial, &r.alternate]
                        .into_iter()
                        .all(|resolved| resolved.as_ref() != Some(clip))
                })
            {
                return Err(format!("Saved clip does not match track {}", track.token));
            }
        }
        if force {
            controller.animation_state.clear_tracks();
        } else {
            for i in resolved.len()..self.resolved.len() {
                controller.animation_state.clear_track(i);
            }
        }
        controller.skeleton.set_to_setup_pose();
        for (i, (r, t)) in resolved.iter().zip(&data.tracks).enumerate() {
            if !force && self.resolved.get(i) == Some(r) {
                continue;
            }
            controller.animation_state.clear_track(i);
            let Some(clip) = &r.clip else { continue };
            let restored = t.current_clip.as_ref().filter(|name| {
                force
                    && (Some(*name) == r.clip.as_ref()
                        || Some(*name) == r.initial.as_ref()
                        || Some(*name) == r.alternate.as_ref())
            });
            let first = restored.or(r.initial.as_ref()).unwrap_or(clip);
            let initial = r.initial.as_ref() == Some(first);
            let alternate = r.alternate.is_some();
            {
                let mut entry = controller
                    .animation_state
                    .set_animation_by_name(i, first, t.looping && !initial && !alternate)
                    .map_err(|e| format!("{e:?}"))?;
                entry.set_track_time(if force { t.time } else { 0.0 });
                entry.set_timescale(t.speed);
                entry.set_mix_duration(0.0);
            }
        }
        self.resolved = resolved;
        self.apply(controller);
        Ok(())
    }
    pub fn update(
        &mut self,
        controller: &mut SkeletonController,
        data: &mut AnimationData,
        dt: f32,
    ) {
        let delta = if data.paused { 0.0 } else { dt };
        for (i, t) in data.tracks.iter().enumerate() {
            if let Some(mut entry) = controller.animation_state.track_at_index_mut(i) {
                entry.set_timescale(t.speed);
                if self.resolved.get(i).is_some_and(|r| {
                    r.alternate.is_none() && r.initial.as_deref() != Some(entry.animation().name())
                }) {
                    entry.set_looping(t.looping);
                }
            }
        }
        controller.animation_state.update(delta);
        for (i, r) in self.resolved.iter().enumerate() {
            let Some(clip) = &r.clip else { continue };
            // Resolve initial→loop ourselves so a large frame, pause or seek
            // cannot advance a queued entry using Spine's previous-frame clock.
            if delta > 0.0 && data.tracks[i].speed > 0.0 {
                let remaining = controller.animation_state.get_current(i).and_then(|entry| {
                    let animation = entry.animation();
                    (r.initial.as_deref() == Some(animation.name())
                        && entry.track_time() >= animation.duration())
                    .then_some(entry.track_time() - animation.duration())
                });
                if let Some(time) = remaining {
                    let mut entry = controller
                        .animation_state
                        .set_animation_by_name(i, clip, data.tracks[i].looping)
                        .expect("validated animation");
                    entry.set_mix_duration(0.0);
                    entry.set_timescale(data.tracks[i].speed);
                    entry.set_track_time(time);
                }
            }
            let Some(alternate) = &r.alternate else {
                continue;
            };
            if delta > 0.0 && data.tracks[i].looping && data.tracks[i].speed > 0.0 {
                let switch = controller.animation_state.get_current(i).and_then(|entry| {
                    let animation = entry.animation();
                    let first = animation.name();
                    let mut time = entry.track_time();
                    let period = animation.duration() + r.interval;
                    if time < period {
                        return None;
                    }
                    let other = if first == clip { alternate } else { clip };
                    let other_period =
                        controller.skeleton.data().find_animation(other)?.duration() + r.interval;
                    let cycle = period + other_period;
                    if cycle <= 0.0 {
                        return None;
                    }
                    time %= cycle;
                    if time >= period {
                        Some((other.clone(), time - period))
                    } else {
                        Some((first.to_owned(), time))
                    }
                });
                if let Some((next, time)) = switch {
                    if let Ok(mut entry) = controller
                        .animation_state
                        .set_animation_by_name(i, &next, false)
                    {
                        entry.set_mix_duration(0.0);
                        entry.set_timescale(data.tracks[i].speed);
                        entry.set_track_time(time);
                    }
                }
            }
        }
        controller.animation_state.apply(&mut controller.skeleton);
        controller.skeleton.update(delta);
        for (i, t) in data.tracks.iter_mut().enumerate() {
            if let Some(entry) = controller.animation_state.get_current(i) {
                t.time = entry.track_time();
                t.current_clip = Some(entry.animation().name().into());
            } else {
                t.time = 0.0;
                t.current_clip = None;
            }
        }
    }
    pub fn seek(&self, controller: &mut SkeletonController, data: &mut AnimationData, time: f32) {
        let i = data.control_track;
        if let Some(mut entry) = controller.animation_state.track_at_index_mut(i) {
            entry.set_track_time(time.max(0.0));
            entry.set_animation_last(time.max(0.0));
            if let Some(t) = data.tracks.get_mut(i) {
                t.time = time.max(0.0);
            }
        }
        self.apply(controller);
    }
    fn apply(&self, controller: &mut SkeletonController) {
        controller.skeleton.set_bones_to_setup_pose();
        controller.animation_state.apply(&mut controller.skeleton);
        controller.skeleton.update_world_transform(Physics::Reset);
    }
    pub fn duration(&self, controller: &SkeletonController, track: usize) -> f32 {
        controller
            .animation_state
            .get_current(track)
            .map_or(0.0, |t| t.animation().duration())
    }
}
