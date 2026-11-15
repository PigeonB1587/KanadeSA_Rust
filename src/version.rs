//! Application metadata. Displayed only in settings.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const VERSION_ID: u32 = 1;
// Source game release is not recorded in the supplied assets. Not the Spine editor version.
pub const SPINE_GAME_VERSION: &str = "2.x";
