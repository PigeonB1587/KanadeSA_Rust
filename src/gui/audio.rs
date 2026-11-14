//! Shared UI feedback. Never called for hover or scrolling gestures.
use macroquad::audio::{Sound, load_sound, play_sound_once};
use std::cell::RefCell;

thread_local! {
    static CLICK: RefCell<Option<Sound>> = const { RefCell::new(None) };
}

pub async fn init() {
    match load_sound("audio/ui/button.ogg").await {
        Ok(sound) => CLICK.with(|slot| *slot.borrow_mut() = Some(sound)),
        Err(error) => tracing::warn!("Failed to load UI click sound: {error}"),
    }
}

pub fn click() {
    CLICK.with(|slot| {
        if let Some(sound) = slot.borrow().as_ref() {
            play_sound_once(sound);
        }
    });
}
