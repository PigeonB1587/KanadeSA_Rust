mod animation;
mod booth;
mod character;
mod equipment;
mod gui;
mod logging;
mod model;
mod persistence;
mod spine_renderer;
mod sprite_material;
mod verification;
mod version;
mod wardrobe;

use character::{CHARACTERS, Character};
use gui::{background, drag_scroll, layout as gui_scale, preview as ui, selection, settings};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "KanadeSA_Rust".to_owned(),
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

    let character = Character::load(&CHARACTERS[0]).unwrap_or_else(|err| {
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
    let mut booth = booth::Booth::new(character).expect("Cannot create booth");
    let mut booth_ui = gui::booth::BoothUi::default();
    if let Some(directory) = std::env::args()
        .skip_while(|arg| arg != "--verify-booth")
        .nth(1)
    {
        match verification::run(
            &mut booth,
            std::path::Path::new(&directory),
            &font_regular,
            &font_bold,
        )
        .await
        {
            Ok(()) => tracing::info!("Booth rendering verification passed: {directory}"),
            Err(error) => panic!("Booth rendering verification failed: {error}"),
        }
        return;
    }
    if booth.save_directory.join("scene.json").exists() {
        if let Err(error) = booth.load() {
            tracing::warn!("Cannot restore last scene: {error}");
            booth_ui.status = error;
        }
    }

    let mut fps_started = std::time::Instant::now();
    let mut selection = selection::Selection::default();
    let mut preview_character: Option<Character> = None;
    let mut preview_source: Option<String> = None;
    let mut preview_transition = gui::widgets::Transition::default();
    let background = background::Background::new();
    let mut info = ui::Info::default();
    let mut settings = settings::Settings::default();
    let mut fps_frames = 0u32;

    loop {
        clear_background(Color::from_rgba(24, 24, 32, 255));
        background.draw();

        booth.update(get_frame_time());
        if let Err(error) = booth.render(1280, 720, false) {
            booth_ui.status = error;
        }

        gui_scale::begin();
        booth_ui.draw_canvas(&booth);
        let cjk = matches!(
            booth.language.as_str(),
            "schinese" | "tchinese" | "japanese" | "koreana"
        );
        let regular = if cjk { &multilingual } else { &font_regular };
        let bold = if cjk { &multilingual_bold } else { &font_bold };
        settings.booth_mode = !booth_ui.preview;
        let settings_blocked = settings.booth_mode && settings.blocks_input();
        if settings_blocked {
            selection.release_search();
        }
        let preview_opacity = preview_transition.update(booth_ui.preview);
        let preview_visible =
            (booth_ui.preview || preview_transition.visible()) && booth.selected.is_some();
        if preview_visible {
            let source = booth.selected().unwrap();
            if preview_source.as_deref() != Some(source.id.as_str()) {
                match source.preview_copy() {
                    Ok(mut character) => {
                        character.set_action(model::Action::Idle);
                        preview_character = Some(character);
                        preview_source = Some(source.id.clone());
                        selection.begin_preview();
                        info = ui::Info::default();
                    }
                    Err(error) => {
                        booth_ui.status = error;
                        booth_ui.preview = false;
                        next_frame().await;
                        continue;
                    }
                }
            }
            let character = preview_character.as_mut().unwrap();
            character.set_language(&booth.language);
            character.update(get_frame_time());
            let size = gui_scale::size();
            // Keep the complete booth GUI underneath the read-only Preview overlay.
            booth_ui.draw(&mut booth, regular, bold, true);
            settings.booth_mode = true;
            settings.draw(
                &mut booth.characters[booth.selected.unwrap_or(0)],
                &multilingual,
                &font_bold,
                true,
            );
            settings.booth_mode = false;
            let _opacity = preview_transition.scope();
            gui::widgets::paint::draw_rectangle(
                0.0,
                0.0,
                size.x,
                size.y,
                gui::widgets::PREVIEW_BACKDROP,
            );
            let _slide = preview_transition.slide();
            set_default_camera();
            if let Err(error) = booth.draw_preview_entity(character, preview_opacity) {
                booth_ui.status = error;
            }
            gui_scale::begin();
            ui::draw(character, regular, bold);
            let preview_blocked = !booth_ui.preview || preview_opacity < 0.999;
            selection.pointer_blocked =
                preview_blocked || settings_blocked || info.blocks_pointer();
            selection.draw(character, regular, bold).await;
            info.draw(
                character,
                regular,
                bold,
                preview_blocked || settings_blocked || selection.captures_keyboard(),
            );
            booth_ui.preview_close(
                bold,
                preview_blocked
                    || settings_blocked
                    || selection.captures_keyboard()
                    || info.blocks_pointer(),
            );
            if let Err(error) = booth.selected_mut().unwrap().apply_preview(character) {
                booth_ui.status = error;
            }
        } else {
            preview_character = None;
            preview_source = None;
            booth_ui.preview = false;
            booth_ui.draw(&mut booth, regular, bold, settings_blocked);
        }
        let index = booth.selected.unwrap_or(0);
        let old_language = booth.language.clone();
        let modal = if preview_visible {
            !booth_ui.preview
                || preview_opacity < 0.999
                || selection.captures_keyboard()
                || (info.blocks_pointer() && !settings_blocked)
        } else {
            booth_ui.blocks_input()
        };
        if !preview_visible {
            settings.draw(
                &mut booth.characters[index],
                &multilingual,
                &font_bold,
                modal,
            );
        }
        let language = booth.characters[index].wardrobe().language.clone();
        if language != old_language {
            booth.set_language(language);
        }
        set_default_camera();

        if booth_ui.preview
            // Opening Preview takes effect before its entity is created next frame.
            && let Some(character) = preview_character.as_mut()
            && preview_opacity >= 0.999
            && !settings_blocked
            && !settings.blocks_input()
            && !selection.captures_keyboard()
            && !info.blocks_pointer()
        {
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
            if let Err(error) = booth.selected_mut().unwrap().apply_preview(character) {
                booth_ui.status = error;
            }
        } else if !preview_visible
            && !booth_ui.preview
            && !settings_blocked
            && !settings.blocks_input()
            && !booth_ui.blocks_input()
        {
            if is_key_pressed(KeyCode::F12) {
                match booth.capture(1920, 1080) {
                    Ok(path) => booth_ui.status = format!("Photo saved: {}", path.display()),
                    Err(error) => booth_ui.status = error,
                }
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
}
