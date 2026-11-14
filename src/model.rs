//! Portable save data. Catalog order and graphics/runtime pointers never enter a save.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_TRACKS: usize = 9;

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn control_track_is_fixed_by_developer_preset() {
        for action in Action::ALL {
            let data = AnimationData::preset(action);
            assert_eq!(
                data.control_track,
                if action == Action::Emote { 1 } else { 0 }
            );
            assert!(data.control_track < data.tracks.len());
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    #[default]
    Idle,
    Run,
    CreepIdle,
    CreepWalk,
    Roll,
    RollBack,
    Umbrella,
    Dive,
    Emote,
}

impl Action {
    pub const ALL: [Self; 9] = [
        Self::Idle,
        Self::Run,
        Self::CreepIdle,
        Self::CreepWalk,
        Self::Roll,
        Self::RollBack,
        Self::Umbrella,
        Self::Dive,
        Self::Emote,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Run => "Run",
            Self::CreepIdle => "CreepIdle",
            Self::CreepWalk => "CreepWalk",
            Self::Roll => "Roll",
            Self::RollBack => "RollBack",
            Self::Umbrella => "Umbrella",
            Self::Dive => "Dive",
            Self::Emote => "Emote",
        }
    }
    /// Embedded Unity presets; no runtime _animation.json dependency.
    pub fn control_track(self) -> usize {
        match self {
            Self::Emote => 1,
            _ => 0,
        }
    }

    pub fn tokens(self) -> &'static [&'static str] {
        match self {
            Self::Idle => &["idle/idle", "aim/*"],
            Self::Run => &["run/run", "aim/*"],
            Self::CreepIdle => &["creeping/idle_creeping", "aim/*"],
            Self::CreepWalk => &["creeping/creeping", "aim/*"],
            Self::Roll => &["rollover/rollover", "aim/*"],
            Self::RollBack => &["rollover/rollover_backwards", "aim/*"],
            Self::Umbrella => &["parachute/parachute"],
            Self::Dive => &["parachute/fall"],
            Self::Emote => &["idle/idle_emotes", "emotes/*"],
        }
    }
    pub fn weapons_visible(self) -> bool {
        !matches!(self, Self::Umbrella | Self::Dive | Self::Emote)
    }
    pub fn umbrella_visible(self) -> bool {
        self == Self::Umbrella
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrackData {
    pub token: String,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
    /// Current clip is persisted so initial→loop and alternate fire restore exactly.
    pub current_clip: Option<String>,
}
impl Default for TrackData {
    fn default() -> Self {
        Self {
            token: String::new(),
            time: 0.0,
            speed: 1.0,
            looping: true,
            current_clip: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnimationData {
    pub action: Action,
    pub tracks: Vec<TrackData>,
    pub control_track: usize,
    pub paused: bool,
    pub pet_animation: Option<PetAnimationData>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PetAnimationData {
    pub clip: String,
    pub time: f32,
}
impl Default for AnimationData {
    fn default() -> Self {
        Self::preset(Action::Idle)
    }
}
impl AnimationData {
    pub fn preset(action: Action) -> Self {
        Self {
            action,
            tracks: action
                .tokens()
                .iter()
                .map(|token| TrackData {
                    token: (*token).into(),
                    ..Default::default()
                })
                .collect(),
            control_track: action.control_track(),
            paused: false,
            pet_animation: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EquipmentData {
    /// Actual booth selections; editing the loadout never selects these slots.
    pub weapon: Option<String>,
    pub gun_skins: BTreeMap<String, String>,
    pub umbrella: Option<String>,
    pub pet: Option<String>,
    pub emote: Option<String>,
    /// Absent in older saves; the catalog migrates their existing selections.
    pub loadout: Option<LoadoutData>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadoutData {
    pub melee: Option<String>,
    pub emote: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterData {
    pub id: String,
    pub animal: String,
    pub outfits: [String; 5],
    pub equipment: EquipmentData,
    pub level: u32,
    pub animation: AnimationData,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceFrame {
    #[default]
    Character,
    World,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Placement {
    pub character_id: String,
    /// Unity world units; Y points up, ground depth is position[1].
    pub position: [f32; 2],
    pub scale: f32,
    /// Absolute world Y of the character foot, independent of perspective scale.
    pub height: f32,
    pub face_right: bool,
    pub pet_face_right: bool,
    pub pet_reference: ReferenceFrame,
    /// Character frame: offset from the container, scaled by its perspective.
    /// World frame: absolute world coordinates, independently draggable.
    pub pet_position: [f32; 2],
    pub rotation: f32,
    pub target_angle: f32,
    pub color: [f32; 4],
    pub shadow_alpha: f32,
}
impl Default for Placement {
    fn default() -> Self {
        Self {
            character_id: String::new(),
            position: [0.0, -3.0],
            scale: 1.0,
            height: -3.0,
            face_right: true,
            pet_face_right: true,
            pet_reference: ReferenceFrame::Character,
            pet_position: [-1.9, -0.32],
            rotation: 0.0,
            target_angle: 0.0,
            color: [1.0; 4],
            shadow_alpha: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Watermark {
    Hidden,
    Left,
    #[default]
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterFile {
    pub format_version: u32,
    pub revision: String,
    pub characters: Vec<CharacterData>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneFile {
    pub format_version: u32,
    pub revision: String,
    pub background: String,
    pub watermark: Watermark,
    pub transparent: bool,
    pub selected_character: Option<String>,
    pub placements: Vec<Placement>,
}

pub fn validate_pair(characters: &CharacterFile, scene: &SceneFile) -> Result<(), String> {
    if characters.format_version != FORMAT_VERSION || scene.format_version != FORMAT_VERSION {
        return Err("Unsupported save format version".into());
    }
    if characters.revision != scene.revision {
        return Err("The two JSON files belong to different saves".into());
    }
    let mut ids = HashSet::new();
    for character in &characters.characters {
        if character.id.is_empty() || !ids.insert(character.id.as_str()) {
            return Err("Empty or duplicate character ID".into());
        }
        let animation = &character.animation;
        if animation
            .pet_animation
            .as_ref()
            .is_some_and(|pet| pet.clip.is_empty() || !pet.time.is_finite() || pet.time < 0.0)
        {
            return Err(format!("Invalid pet animation for {}", character.id));
        }
        if animation.tracks.is_empty()
            || animation.tracks.len() > MAX_TRACKS
            || animation.control_track >= animation.tracks.len()
        {
            return Err(format!("Invalid animation tracks for {}", character.id));
        }
        for track in &animation.tracks {
            if !track.time.is_finite()
                || track.time < 0.0
                || !track.speed.is_finite()
                || !(0.0..=8.0).contains(&track.speed)
                || track.token.is_empty()
            {
                return Err(format!("Invalid animation property for {}", character.id));
            }
        }
    }
    let mut placed = HashSet::new();
    for p in &scene.placements {
        if !ids.contains(p.character_id.as_str()) || !placed.insert(p.character_id.as_str()) {
            return Err("Missing or duplicate character reference in scene".into());
        }
        if p.position
            .iter()
            .chain(&p.pet_position)
            .chain(&p.color)
            .chain([
                &p.scale,
                &p.height,
                &p.rotation,
                &p.target_angle,
                &p.shadow_alpha,
            ])
            .any(|v| !v.is_finite())
            || !(0.05..=5.0).contains(&p.scale)
            || !(0.0..=1.0).contains(&p.shadow_alpha)
            || p.color.iter().any(|v| !(0.0..=1.0).contains(v))
        {
            return Err("Invalid scene transform or color".into());
        }
    }
    if scene
        .selected_character
        .as_deref()
        .is_some_and(|id| !placed.contains(id))
    {
        return Err("Selected character is absent from scene".into());
    }
    Ok(())
}
