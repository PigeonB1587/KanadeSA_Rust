mod character;
mod gui;
mod logging;
mod spine_renderer;
mod version;
mod wardrobe;

use character::{CHARACTERS, Character};
use gui::{background, drag_scroll, layout as gui_scale, preview as ui, selection, settings};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "KanadeSA_Rust - Spine".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    simulate_mouse_with_touch(true);
    // Keep the guard alive so queued file logs are flushed on normal exit.
    let _log_guard = logging::init();
    tracing::info!("Starting KanadeSA_Rust");
    gui::audio::init().await;
    let font_regular = load_ttf_font("fonts/Rubik/Rubik-Regular.ttf")
        .await
        .expect("failed to load Rubik Regular font");
    let font_bold = load_ttf_font("fonts/Rubik/Rubik-Bold.ttf")
        .await
        .expect("failed to load Rubik Bold font");

    let mut character = Character::load(&CHARACTERS[0]).unwrap_or_else(|err| {
        panic!("failed to load character: {err}");
    });
    let multilingual = load_ttf_font("fonts/SourceHanSans/Regular.otf")
        .await
        .expect("Failed to load multilingual font");
    let multilingual_bold = load_ttf_font("fonts/SourceHanSans/Bold.otf")
        .await
        .expect("Failed to load multilingual bold font");
    tracing::info!(
        "Loaded {}; skin: {}; animation: {}",
        character.name(),
        character.skin_name(),
        character.animation_name()
    );

    let mut fps_started = std::time::Instant::now();
    let mut selection = selection::Selection::default();
    let background = background::Background::new();
    let mut info = ui::Info::default();
    let mut settings = settings::Settings::default();
    let mut fps_frames = 0u32;

    loop {
        clear_background(Color::from_rgba(24, 24, 32, 255));
        background.draw();

        character.update(get_frame_time());
        character.draw();

        gui_scale::begin();
        let cjk = matches!(
            character.wardrobe().language.as_str(),
            "schinese" | "tchinese" | "japanese" | "koreana"
        );
        let regular = if cjk { &multilingual } else { &font_regular };
        let bold = if cjk { &multilingual_bold } else { &font_bold };
        ui::draw(&character, regular, bold);
        let settings_blocked = settings.blocks_input();
        if settings_blocked {
            selection.release_search();
        }
        selection.pointer_blocked = settings_blocked || info.blocks_pointer();
        selection.draw(&mut character, regular, bold).await;
        info.draw(&character, regular, bold, settings_blocked);
        settings.draw(&mut character, &multilingual, &font_bold);
        set_default_camera();

        if !settings_blocked && !settings.blocks_input() && !selection.captures_keyboard() {
            if is_key_pressed(KeyCode::Left) {
                character.cycle_animation(-1);
            }
            if is_key_pressed(KeyCode::Right) {
                character.cycle_animation(1);
            }
            if is_key_pressed(KeyCode::Tab) {
                character.select_wardrobe_category();
            }
            if is_key_pressed(KeyCode::Q) {
                character.cycle_outfit(-1);
            }
            if is_key_pressed(KeyCode::E) {
                character.cycle_outfit(1);
            }
            if is_key_pressed(KeyCode::Backspace) {
                character.clear_outfit();
            }
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
        }

        next_frame().await;

        fps_frames += 1;
        let elapsed = fps_started.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            tracing::info!(target: "kanade_sa::performance", "FPS {:.1} | frame {:.2} ms",
                f64::from(fps_frames) / elapsed, elapsed * 1000.0 / f64::from(fps_frames));
            fps_started = std::time::Instant::now();
            fps_frames = 0;
        }
    }
    tracing::info!("Shutting down");
}
