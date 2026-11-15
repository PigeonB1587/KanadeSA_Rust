//! Installed resources live under assets/; project data keeps its own root.
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

pub const ROOT: &str = "assets";
pub const BOOTH_CONFIG: &str = include_str!("../assets/configs/_boothbg.json");
#[cfg(test)]
pub const LANGUAGES: &str = include_str!("../assets/configs/_languages.json");

pub const CHARACTER_ATLAS: &str = "assets/spine/characters/character_01/skeleton.atlas";
pub const CHARACTER_SKELETON: &str = "assets/spine/characters/character_01/skeleton.skel.bytes";
pub const PET_ATLAS: &str = "assets/spine/pets/skeleton.atlas";
pub const PET_SKELETON: &str = "assets/spine/pets/skeleton.skel.bytes";
pub const PLATFORM_ATLAS: &str = "assets/spine/spring_platform/skeleton.atlas";
pub const PLATFORM_SKELETON: &str = "assets/spine/spring_platform/skeleton.skel.bytes";
pub const UMBRELLA_ATLAS: &str = "assets/spine/umbrellas/skeleton.atlas";
pub const FONT_REGULAR: &str = "assets/fonts/Rubik/Rubik-Regular.ttf";
pub const FONT_BOLD: &str = "assets/fonts/Rubik/Rubik-Bold.ttf";
pub const FONT_MULTILINGUAL: &str = "assets/fonts/SourceHanSans/Regular.otf";
pub const FONT_MULTILINGUAL_BOLD: &str = "assets/fonts/SourceHanSans/Bold.otf";
pub const BUTTON_SOUND: &str = "assets/audio/ui/button.ogg";
pub const FALLBACK_ICON: &str = "assets/icons/ui/inventoryicons/item_rejection_letter.png";

/// Catalog IDs (Booth/..., UI/...) stay unchanged; resolve only at file I/O.
pub fn path(relative: impl AsRef<Path>) -> PathBuf {
    Path::new(ROOT).join(relative)
}

pub fn read_config<T: DeserializeOwned>(name: &str) -> Result<T, String> {
    let path = path(format!("configs/_{name}.json"));
    let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid {}: {e}", path.display()))
}
