use super::widgets::{
    self,
    paint::{draw_rectangle, draw_text_ex},
};
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
    pub booth_mode: bool,
    open: bool,
    transition: super::widgets::Transition,
    dropdown: bool,
    dropdown_transition: super::widgets::Transition,
    scroll: f32,
    content_drag: crate::drag_scroll::DragScroll,
    modal_this_frame: bool,
}
impl Settings {
    pub fn is_visible(&self) -> bool {
        self.open || self.transition.visible() || self.modal_this_frame
    }
    pub fn blocks_input(&self) -> bool {
        self.is_visible()
            || (is_mouse_button_pressed(MouseButton::Left)
                && self.button().contains(gui_scale::mouse()))
    }
    fn button(&self) -> Rect {
        Rect::new(
            gui_scale::size().x
                - if gui_scale::compact() && !self.booth_mode {
                    166.0
                } else {
                    94.0
                },
            10.0,
            82.0,
            32.0,
        )
    }
    fn toggle(&mut self) {
        self.open = !self.open;
        self.dropdown = false;
        self.content_drag.cancel();
    }
    pub fn draw(
        &mut self,
        character: &mut Character,
        font: &Font,
        bold: &Font,
        input_blocked: bool,
    ) {
        let size = gui_scale::size();
        self.modal_this_frame = self.open || self.transition.visible();
        let mouse = if input_blocked {
            vec2(-10000.0, -10000.0)
        } else {
            gui_scale::mouse()
        };
        let click = !input_blocked && is_mouse_button_pressed(MouseButton::Left);
        let button = self.button();
        if click && button.contains(mouse) {
            super::audio::click();
            self.toggle();
        }
        self.transition.update(self.open);
        draw_rectangle(
            button.x,
            button.y,
            button.w,
            button.h,
            Color::from_rgba(30, 31, 40, 255),
        );
        let text = measure_text("SETTINGS", Some(bold), 13, 1.0);
        label(
            "SETTINGS",
            button.x + (button.w - text.width) * 0.5,
            button.y + (button.h - text.height) * 0.5 + text.offset_y,
            13,
            bold,
            if self.open { ORANGE } else { LIGHTGRAY },
        );
        if !self.open && !self.transition.visible() {
            self.dropdown_transition.update(false);
            return;
        }
        self.transition.backdrop();
        let width = (size.x - 32.0).min(440.0);
        let height = (size.y - 80.0).clamp(1.0, 430.0);
        let x = (size.x - width) * 0.5;
        let panel = self.transition.panel(
            self.open,
            Rect::new(x, 56.0 + (size.y - 80.0 - height) * 0.5, width, height),
        );
        let y = panel.y;
        let mouse = self.transition.mouse(self.open, mouse);
        let _opacity = self.transition.scope();
        draw_rectangle(x, y, width, height, widgets::SURFACE);
        let close = widgets::popup_header(panel, "Settings", font, bold);
        let child_active = self.dropdown || self.dropdown_transition.visible();
        let close_parent =
            self.open && widgets::dismiss_layer(panel, close, button, mouse, !child_active);
        if close_parent {
            self.toggle();
        }
        let mouse = if self.open { mouse } else { widgets::BLOCKED };
        label("Language", x + 20.0, y + 65.0, 13, font, GRAY);
        let field = Rect::new(x + 20.0, y + 76.0, width - 40.0, 34.0);
        draw_rectangle(
            field.x,
            field.y,
            field.w,
            field.h,
            Color::from_rgba(36, 37, 47, 255),
        );
        let current = LANGUAGES
            .iter()
            .find(|(key, _)| *key == character.wardrobe().language)
            .map(|(_, name)| *name)
            .unwrap_or(&character.wardrobe().language);
        label(current, field.x + 10.0, field.y + 23.0, 14, font, WHITE);
        label(
            "v",
            field.x + field.w - 22.0,
            field.y + 23.0,
            13,
            font,
            GRAY,
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
            label(line, x + 20.0, y + 145.0 + i as f32 * 24.0, 12, font, GRAY);
        }
        self.dropdown_transition.update(self.dropdown);
        if self.dropdown || self.dropdown_transition.visible() {
            let dropdown_panel = self.dropdown_transition.panel(
                self.dropdown,
                Rect::new(
                    field.x,
                    field.y + field.h + 4.0,
                    field.w,
                    (y + height - field.y - field.h - 16.0).max(32.0),
                ),
            );
            let list = Rect::new(
                dropdown_panel.x,
                dropdown_panel.y,
                dropdown_panel.w,
                dropdown_panel.h,
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
            let mouse = self
                .dropdown_transition
                .mouse(self.dropdown && self.open, mouse);
            if self.dropdown
                && widgets::dismiss_except(
                    dropdown_panel,
                    Rect::new(-10000.0, -10000.0, 0.0, 0.0),
                    field,
                    mouse,
                )
            {
                self.dropdown = false;
            }
            let mouse = if self.dropdown {
                mouse
            } else {
                widgets::BLOCKED
            };
            let tapped =
                widgets::scroll(list, mouse, &mut self.scroll, max, &mut self.content_drag);
            let _opacity = self.dropdown_transition.scope();
            draw_rectangle(
                dropdown_panel.x,
                dropdown_panel.y,
                dropdown_panel.w,
                dropdown_panel.h,
                Color::from_rgba(36, 37, 47, 255),
            );
            let mut chosen = None;
            super::layout::clip(list);
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
                        Color::from_rgba(53, 44, 41, 255),
                    );
                }
                label(
                    name,
                    row.x + 10.0,
                    row.y + 21.0,
                    14,
                    font,
                    if *key == character.wardrobe().language {
                        ORANGE
                    } else {
                        WHITE
                    },
                );
                if self.open
                    && self.dropdown
                    && tapped
                    && list.contains(mouse)
                    && row.contains(mouse)
                {
                    chosen = Some((*key).to_owned());
                }
            }
            super::layout::end_clip();
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
