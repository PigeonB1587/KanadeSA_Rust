//! Preview header, wardrobe summary, footer and information panel.
use crate::character::Character;
use macroquad::prelude::*;

const TEXT: Color = Color::new(0.90, 0.91, 0.94, 1.0);
const MUTED: Color = Color::new(0.49, 0.52, 0.59, 1.0);
const LINE: Color = Color::new(0.19, 0.20, 0.25, 1.0);
const INFO_SURFACE: Color = Color::new(0.055, 0.059, 0.080, 0.78);
const INFO_MUTED: Color = Color::new(0.65, 0.68, 0.74, 1.0);

fn info_surface(panel: Rect) {
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, INFO_SURFACE);
}

#[derive(Default)]
pub struct Info {
    open: bool,
    scroll: f32,
}

impl Info {
    pub fn blocks_pointer(&self) -> bool {
        if !crate::gui_scale::compact() {
            return false;
        }
        let size = crate::gui_scale::size();
        let mouse = crate::gui_scale::mouse();
        Rect::new(size.x - 76.0, 10.0, 60.0, 32.0).contains(mouse)
            || (self.open
                && Rect::new(
                    16.0,
                    80.0,
                    (size.x - 32.0).min(260.0),
                    (size.y - 120.0).clamp(1.0, 320.0),
                )
                .contains(mouse))
    }
    pub fn draw(&mut self, character: &Character, font: &Font, bold: &Font, input_blocked: bool) {
        if !crate::gui_scale::compact() {
            return;
        }
        let size = crate::gui_scale::size();
        let mouse = crate::gui_scale::mouse();
        let button = Rect::new(size.x - 76.0, 10.0, 60.0, 32.0);
        if !input_blocked && is_mouse_button_pressed(MouseButton::Left) && button.contains(mouse) {
            self.open = !self.open;
            super::audio::click();
        }
        draw_rectangle(
            button.x,
            button.y,
            button.w,
            button.h,
            if self.open {
                Color::from_rgba(53, 44, 41, 255)
            } else {
                Color::from_rgba(30, 31, 40, 255)
            },
        );
        label(
            "info",
            button.x + 16.0,
            button.y + 21.0,
            13,
            bold,
            if self.open || button.contains(mouse) {
                Color::from_rgba(236, 154, 92, 255)
            } else {
                TEXT
            },
            40.0,
        );
        if !self.open {
            return;
        }
        let width = (size.x - 32.0).min(260.0);
        let x = 16.0;
        let y = 80.0;
        let height = (size.y - y - 40.0).clamp(1.0, 320.0);
        if !input_blocked && Rect::new(x, y, width, height).contains(mouse) {
            self.scroll =
                (self.scroll - mouse_wheel().1 * 24.0).clamp(0.0, (320.0 - height).max(0.0));
        }
        info_surface(Rect::new(x, y, width, height));
        let scale = screen_width() * screen_dpi_scale() / size.x;
        unsafe {
            get_internal_gl().quad_gl.scissor(Some((
                (x * scale) as i32,
                (y * scale) as i32,
                (width * scale).ceil() as i32,
                (height * scale).ceil() as i32,
            )));
        }
        let top = y - self.scroll;
        label(
            "SKIN",
            x + 12.0,
            top + 22.0,
            10,
            font,
            INFO_MUTED,
            width - 24.0,
        );
        label(
            character.skin_name(),
            x + 12.0,
            top + 44.0,
            15,
            font,
            TEXT,
            width - 24.0,
        );
        label(
            &format!(
                "ANIMATION   {:03} / {:03}",
                character.animation_index(),
                character.animation_count()
            ),
            x + 12.0,
            top + 68.0,
            10,
            font,
            INFO_MUTED,
            width - 24.0,
        );
        label(
            character.animation_name(),
            x + 12.0,
            top + 90.0,
            15,
            font,
            TEXT,
            width - 24.0,
        );
        draw_line(x, top + 112.0, x + width, top + 112.0, 1.0, LINE);
        draw_wardrobe_content(
            character,
            font,
            bold,
            Rect::new(x, top + 112.0, width, 208.0),
        );
        unsafe {
            get_internal_gl().quad_gl.scissor(None);
        }
    }
}

fn label(text: &str, x: f32, y: f32, size: u16, font: &Font, color: Color, width: f32) {
    let mut fitted = text.to_owned();
    if measure_text(&fitted, Some(font), size, 1.0).width > width {
        while !fitted.is_empty()
            && measure_text(&format!("{fitted}..."), Some(font), size, 1.0).width > width
        {
            fitted.pop();
        }
        fitted.push_str("...");
    }
    // Clip even the ellipsis when the window is narrower than one text glyph.
    if width <= 0.0 || measure_text(&fitted, Some(font), size, 1.0).width > width {
        return;
    }
    draw_text_ex(
        &fitted,
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

/// Shared by the desktop summary and compact Info, including selection styling.
fn draw_wardrobe(character: &Character, regular: &Font, bold: &Font, panel: Rect) {
    info_surface(panel);
    draw_wardrobe_content(character, regular, bold, panel);
}

fn draw_wardrobe_content(character: &Character, regular: &Font, bold: &Font, panel: Rect) {
    let x = panel.x + 12.0;
    let y = panel.y + 12.0;
    let width = panel.w - 24.0;
    let wardrobe = character.wardrobe();
    label("WARDROBE", x, y + 10.0, 11, bold, INFO_MUTED, width);
    for (index, category) in crate::wardrobe::CATEGORIES.iter().enumerate() {
        let baseline = y + 36.0 + index as f32 * 32.0;
        let active = index == wardrobe.category;
        if active {
            draw_rectangle(
                panel.x,
                baseline - 14.0,
                2.0,
                29.0,
                Color::from_rgba(236, 154, 92, 255),
            );
        }
        let color = if active { TEXT } else { INFO_MUTED };
        label(
            &format!(
                "{}  {}/{}",
                category,
                wardrobe.selected[index] + 1,
                wardrobe.items[index].len()
            ),
            x,
            baseline - 2.0,
            10,
            regular,
            color,
            width,
        );
        label(
            wardrobe.item_name(index),
            x,
            baseline + 13.0,
            12,
            regular,
            color,
            width,
        );
    }
}

pub fn draw(character: &Character, regular: &Font, bold: &Font) {
    let size = crate::gui_scale::size();
    let w = size.x;
    let h = size.y;
    let margin = 28.0_f32.min(w * 0.04);
    let inner = (w - margin * 2.0).max(0.0);
    let narrow = crate::gui_scale::compact();
    let header_h = if narrow { 104.0 } else { 88.0 };
    let footer_h = if narrow { 54.0 } else { 58.0 };
    let panel = Color::from_rgba(20, 21, 28, 242);
    if crate::gui_scale::compact() {
        draw_rectangle(0.0, 0.0, w, 74.0, panel);
        label(
            "CHARACTER PREVIEW",
            16.0,
            20.0,
            10,
            regular,
            MUTED,
            w - 32.0,
        );
        label(
            character.name(),
            16.0,
            44.0,
            18,
            bold,
            TEXT,
            (w - 198.0).max(1.0),
        );
        label(
            character.animation_name(),
            16.0,
            64.0,
            12,
            regular,
            MUTED,
            (w - 100.0).max(1.0),
        );
        draw_line(16.0, 74.0, w - 16.0, 74.0, 1.0, LINE);
        draw_rectangle(0.0, h - 32.0, w, 32.0, panel);
        label(
            "Click to equip    Q / E: cycle    Esc: quit",
            16.0,
            h - 12.0,
            11,
            regular,
            MUTED,
            w - 32.0,
        );
        return;
    }
    draw_rectangle(0.0, 0.0, w, header_h, panel);
    draw_line(margin, header_h, w - margin, header_h, 1.0, LINE);

    label("CHARACTER PREVIEW", margin, 27.0, 11, regular, MUTED, inner);
    let title_width = if narrow { inner } else { inner * 0.28 };
    label(character.name(), margin, 57.0, 22, bold, TEXT, title_width);

    let (skin_x, animation_x, label_y, value_y, skin_w, animation_w) = if narrow {
        (
            margin,
            margin + inner * 0.5,
            78.0,
            96.0,
            inner * 0.47,
            inner * 0.5,
        )
    } else {
        (
            margin + inner * 0.32,
            margin + inner * 0.61,
            27.0,
            57.0,
            inner * 0.25,
            inner * 0.39,
        )
    };
    label("SKIN", skin_x, label_y, 10, regular, MUTED, skin_w);
    label(
        character.skin_name(),
        skin_x,
        value_y,
        15,
        regular,
        TEXT,
        skin_w,
    );
    let (animation_label_y, animation_value_y) = (label_y, value_y);
    label(
        &format!(
            "ANIMATION   {:03} / {:03}",
            character.animation_index(),
            character.animation_count()
        ),
        animation_x,
        animation_label_y,
        10,
        regular,
        MUTED,
        animation_w,
    );
    label(
        character.animation_name(),
        animation_x,
        animation_value_y,
        15,
        regular,
        TEXT,
        animation_w,
    );

    let card_w = (inner * 0.18).clamp(130.0, 205.0).min(inner);
    let card_x = margin;
    let card_y = header_h + 24.0;
    draw_wardrobe(
        character,
        regular,
        bold,
        Rect::new(card_x - 12.0, card_y - 12.0, card_w + 24.0, 208.0),
    );

    let top = (h - footer_h).max(header_h);
    draw_rectangle(0.0, top, w, footer_h, panel);
    draw_line(margin, top, w - margin, top, 1.0, LINE);
    if narrow {
        label(
            "Tab: category   Q / E: item   Backspace: clear",
            margin,
            top + 47.0,
            9,
            regular,
            MUTED,
            inner,
        );
        label(
            "Left / Right   Animation",
            margin,
            top + 16.0,
            10,
            regular,
            MUTED,
            inner,
        );
        label(
            "Esc   Quit",
            margin,
            top + 31.0,
            10,
            regular,
            MUTED,
            inner,
        );
    } else {
        label(
            "Tab   Category       Q / E   Equip item       Backspace   Clear outfit",
            margin,
            top + 47.0,
            11,
            regular,
            MUTED,
            inner,
        );
        let third = inner / 3.0;
        label(
            "Left / Right   Animation",
            margin,
            top + 29.0,
            12,
            regular,
            MUTED,
            third,
        );
        let quit = "Esc   Quit";
        let quit_w = measure_text(quit, Some(regular), 12, 1.0).width;
        label(
            quit,
            w - margin - quit_w,
            top + 29.0,
            12,
            regular,
            MUTED,
            third,
        );
    }
}
