//! Small application preferences, separate from scene/character saves.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub vsync: bool,
    pub light: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            vsync: true,
            light: false,
        }
    }
}
impl Preferences {
    pub fn load() -> Self {
        std::fs::read("saves/preferences.json")
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all("saves").map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write("saves/preferences.json.pending", bytes).map_err(|e| e.to_string())?;
        std::fs::rename("saves/preferences.json.pending", "saves/preferences.json")
            .map_err(|e| e.to_string())
    }
}
