//! Editor-only settings, autosaved independently of the paired scene documents.
use serde::{Deserialize, Serialize};
use std::io::Write;

pub const FILE: &str = "editor_settings.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub vsync: bool,
    pub light: bool,
    pub language: String,
    pub last_project: Option<String>,
    pub inspector_visible: bool,
    pub inspector_tab: usize,
    pub photo_longest: u32,
    pub gif_longest: u32,
    pub gif_fps: u32,
    pub gif_colors: u32,
    pub show_shadows: bool,
    pub show_pets: bool,
    pub show_gems: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            vsync: true,
            light: false,
            language: "english".into(),
            last_project: None,
            inspector_visible: true,
            inspector_tab: 0,
            photo_longest: 1920,
            gif_longest: 720,
            gif_fps: 20,
            gif_colors: 128,
            show_shadows: true,
            show_pets: true,
            show_gems: true,
        }
    }
}
impl Preferences {
    pub fn load() -> Self {
        let bytes = match std::fs::read(FILE) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Read the old theme/VSync file only until the root file exists.
                match std::fs::read("saves/preferences.json") {
                    Ok(bytes) => bytes,
                    Err(_) => return Self::default(),
                }
            }
            Err(error) => {
                tracing::warn!("Cannot read editor settings: {error}");
                return Self::default();
            }
        };
        match serde_json::from_slice::<Self>(&bytes) {
            Ok(mut preferences) => {
                preferences.normalize();
                preferences
            }
            Err(error) => {
                tracing::warn!("Cannot parse editor settings: {error}");
                Self::default()
            }
        }
    }
    fn normalize(&mut self) {
        if self.language.trim().is_empty() {
            self.language = "english".into();
        }
        if self
            .last_project
            .as_deref()
            .is_some_and(|id| !crate::project::valid_id(id))
        {
            self.last_project = None;
        }
        self.inspector_tab = self.inspector_tab.min(2);
        if ![1280, 1920, 2560].contains(&self.photo_longest) {
            self.photo_longest = 1920;
        }
        if ![480, 720, 1080].contains(&self.gif_longest) {
            self.gif_longest = 720;
        }
        if ![10, 20, 25, 50].contains(&self.gif_fps) {
            self.gif_fps = 20;
        }
        if ![64, 128, 256].contains(&self.gif_colors) {
            self.gif_colors = 128;
        }
    }
    pub fn save(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut file =
            std::fs::File::create(format!("{FILE}.pending")).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(format!("{FILE}.pending"), FILE).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_preferences_gain_editor_defaults() {
        let preferences: Preferences =
            serde_json::from_str(r#"{"vsync":false,"light":true}"#).unwrap();
        assert!(!preferences.vsync);
        assert!(preferences.light);
        assert_eq!(preferences.language, "english");
        assert_eq!(preferences.photo_longest, 1920);
        assert_eq!(preferences.gif_fps, 20);
        assert_eq!(preferences.last_project, None);
    }

    #[test]
    fn editor_settings_round_trip_without_scene_or_character_data() {
        let preferences = Preferences {
            language: "schinese".into(),
            last_project: Some("project-1".into()),
            gif_fps: 50,
            show_pets: false,
            ..Default::default()
        };
        let value = serde_json::to_value(&preferences).unwrap();
        assert!(value.get("characters").is_none());
        assert!(value.get("placements").is_none());
        assert!(value.get("background").is_none());
        assert!(value.get("canvas_ratio").is_none());
        assert!(value.get("save_directory").is_none());
        assert_eq!(
            serde_json::from_value::<Preferences>(value).unwrap(),
            preferences
        );
    }

    #[test]
    fn invalid_editor_values_fall_back_to_supported_choices() {
        let mut preferences = Preferences {
            photo_longest: 0,
            gif_longest: u32::MAX,
            gif_fps: 999,
            gif_colors: 0,
            inspector_tab: usize::MAX,
            last_project: Some("../outside".into()),
            language: "".into(),
            ..Default::default()
        };
        preferences.normalize();
        assert_eq!(preferences.photo_longest, 1920);
        assert_eq!(preferences.gif_longest, 720);
        assert_eq!(preferences.gif_fps, 20);
        assert_eq!(preferences.gif_colors, 128);
        assert_eq!(preferences.inspector_tab, 2);
        assert_eq!(preferences.last_project, None);
        assert_eq!(preferences.language, "english");
    }
}
