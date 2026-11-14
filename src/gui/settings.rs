use crate::{character::Character, gui_scale, version};
use macroquad::prelude::*;

pub const LANGUAGES: [(&str, &str); 12] = [
    ("english", "English"),
    ("schinese", "简体中文"),
    ("tchinese", "繁體中文"),
    ("japanese", "日本語"),
    ("koreana", "한국어"),
    ("russian", "Русский"),
    ("german", "Deutsch"),
    ("italian", "Italiano"),
    ("french", "Français"),
    ("spanish", "Español"),
    ("latam", "Español (Latinoamérica)"),
    ("brazilian", "Português (Brasil)"),
];

#[derive(Default)]
pub struct Settings {
    open: bool,
    progress: f32,
    from: f32,
    started: f64,
    dropdown: bool,
    scroll: f32,
    content_drag: crate::drag_scroll::DragScroll,
}
impl Settings {
    pub fn is_visible(&self) -> bool {
        self.open || self.progress > 0.001
    }
    pub fn blocks_input(&self) -> bool {
        self.is_visible() || self.button().contains(gui_scale::mouse())
    }
    fn button(&self) -> Rect {
        Rect::new(
            gui_scale::size().x - if gui_scale::compact() { 166.0 } else { 106.0 },
            10.0,
            82.0,
            32.0,
        )
    }
    fn toggle(&mut self) {
        super::audio::click();
        self.from = self.progress;
        self.started = get_time();
        self.open = !self.open;
        self.dropdown = false;
        self.content_drag.cancel();
    }
    pub fn draw(&mut self, character: &mut Character, font: &Font, bold: &Font) {
        let size = gui_scale::size();
        let mouse = gui_scale::mouse();
        let click = is_mouse_button_pressed(MouseButton::Left);
        let button = self.button();
        if click && button.contains(mouse) {
            self.toggle();
        }
        if self.open && is_key_pressed(KeyCode::Escape) {
            self.toggle();
        }
        let t = ((get_time() - self.started) / 0.28).clamp(0.0, 1.0) as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        self.progress = self.from + ((if self.open { 1.0 } else { 0.0 }) - self.from) * eased;
        draw_rectangle(
            button.x,
            button.y,
            button.w,
            button.h,
            Color::from_rgba(30, 31, 40, 255),
        );
        label(
            "settings",
            button.x + 12.0,
            button.y + 21.0,
            13,
            bold,
            if self.open { ORANGE } else { LIGHTGRAY },
        );
        if self.progress < 0.001 {
            return;
        }
        draw_rectangle(
            0.0,
            0.0,
            size.x,
            size.y,
            Color::new(0.0, 0.0, 0.0, self.progress * 0.55),
        );
        let width = (size.x - 32.0).min(440.0);
        let height = (size.y - 80.0).min(430.0);
        let x = (size.x - width) * 0.5;
        let y = 56.0 + (size.y - 80.0 - height) * 0.5 + (1.0 - self.progress) * 60.0;
        let panel = Rect::new(x, y, width, height);
        let faded = |mut color: Color| {
            color.a *= self.progress;
            color
        };
        draw_rectangle(
            x,
            y,
            width,
            height,
            faded(Color::from_rgba(20, 21, 28, 255)),
        );
        label("Settings", x + 20.0, y + 30.0, 20, bold, faded(WHITE));
        let close = Rect::new(x + width - 70.0, y + 10.0, 54.0, 30.0);
        label("close", close.x, close.y + 20.0, 13, font, faded(GRAY));
        if self.open
            && click
            && (close.contains(mouse) || (!panel.contains(mouse) && !button.contains(mouse)))
        {
            self.toggle();
        }
        let faded = |mut color: Color| {
            color.a *= self.progress;
            color
        };
        label("Language", x + 20.0, y + 65.0, 13, font, faded(GRAY));
        let field = Rect::new(x + 20.0, y + 76.0, width - 40.0, 34.0);
        draw_rectangle(
            field.x,
            field.y,
            field.w,
            field.h,
            faded(Color::from_rgba(36, 37, 47, 255)),
        );
        let current = LANGUAGES
            .iter()
            .find(|(key, _)| *key == character.wardrobe().language)
            .map(|(_, name)| *name)
            .unwrap_or(&character.wardrobe().language);
        label(
            current,
            field.x + 10.0,
            field.y + 23.0,
            14,
            font,
            faded(WHITE),
        );
        label(
            "v",
            field.x + field.w - 22.0,
            field.y + 23.0,
            13,
            font,
            faded(GRAY),
        );
        if self.open && click && field.contains(mouse) {
            super::audio::click();
            self.dropdown = !self.dropdown;
            self.content_drag.cancel();
        }
        for (i, line) in [
            format!("Version: {}", version::VERSION),
            format!("Version ID: {}", version::VERSION_ID),
            format!("Spine asset game version: {}", version::SPINE_GAME_VERSION),
        ]
        .iter()
        .enumerate()
        {
            label(
                line,
                x + 20.0,
                y + 145.0 + i as f32 * 24.0,
                12,
                font,
                faded(GRAY),
            );
        }
        if self.dropdown {
            let list = Rect::new(
                field.x,
                field.y + field.h + 4.0,
                field.w,
                (y + height - field.y - field.h - 16.0).max(32.0),
            );
            let languages: Vec<(&str, &str)> = character
                .wardrobe()
                .available_languages
                .iter()
                .map(|key| {
                    (
                        key.as_str(),
                        LANGUAGES
                            .iter()
                            .find(|(known, _)| known == key)
                            .map(|(_, name)| *name)
                            .unwrap_or(key.as_str()),
                    )
                })
                .collect();
            let row_height = 40.0;
            let max = (languages.len() as f32 * row_height - list.h).max(0.0);
            let tapped = self.content_drag.update(
                list,
                mouse,
                click,
                is_mouse_button_down(MouseButton::Left),
                is_mouse_button_released(MouseButton::Left),
                &mut self.scroll,
                max,
            );
            if list.contains(mouse) {
                self.scroll = (self.scroll - mouse_wheel().1 * 30.0).clamp(0.0, max);
            }
            draw_rectangle(
                list.x,
                list.y,
                list.w,
                list.h,
                faded(Color::from_rgba(36, 37, 47, 255)),
            );
            let mut chosen = None;
            // QuadGl scissor coordinates are top-left framebuffer pixels, not GUI units.
            let sx = screen_width() * screen_dpi_scale() / gui_scale::size().x;
            let sy = screen_height() * screen_dpi_scale() / gui_scale::size().y;
            let clip = (
                (list.x * sx).floor() as i32,
                (list.y * sy).floor() as i32,
                (list.w * sx).ceil() as i32,
                (list.h * sy).ceil() as i32,
            );
            unsafe {
                get_internal_gl().quad_gl.scissor(Some(clip));
            }
            for (i, (key, name)) in languages.iter().enumerate() {
                let row = Rect::new(
                    list.x,
                    list.y + i as f32 * row_height - self.scroll,
                    list.w,
                    row_height,
                );
                if row.y + row.h <= list.y || row.y >= list.y + list.h {
                    continue;
                }
                if row.contains(mouse) {
                    draw_rectangle(
                        row.x,
                        row.y,
                        row.w,
                        row.h,
                        faded(Color::from_rgba(53, 44, 41, 255)),
                    );
                }
                label(
                    name,
                    row.x + 10.0,
                    row.y + 21.0,
                    14,
                    font,
                    faded(if *key == character.wardrobe().language {
                        ORANGE
                    } else {
                        WHITE
                    }),
                );
                if self.open && tapped && row.contains(mouse) {
                    chosen = Some((*key).to_owned());
                }
            }
            unsafe {
                get_internal_gl().quad_gl.scissor(None);
            }
            if let Some(key) = chosen {
                super::audio::click();
                character.set_language(&key);
                self.dropdown = false;
            }
        }
    }
}
fn label(s: &str, x: f32, y: f32, size: u16, font: &Font, color: Color) {
    draw_text_ex(
        s,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size: size,
            color,
            ..Default::default()
        },
    );
}
