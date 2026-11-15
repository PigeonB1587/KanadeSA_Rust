//! Scene tint popup: HSV palette and native numeric RGBA entry.
use super::{
    input::NativeInput,
    widgets::{
        self,
        paint::{draw_rectangle, draw_text_ex},
    },
};
use macroquad::prelude::*;

#[derive(Default)]
pub struct ColorPicker {
    target: Option<String>,
    open: bool,
    transition: widgets::Transition,
    color: [f32; 4],
    hsv: [f32; 3],
    drag: Option<usize>,
    input: NativeInput,
    channel: usize,
    just_opened: bool,
}

fn rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let f = h.fract();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match h as usize {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
fn hsv(c: [f32; 4]) -> [f32; 3] {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    };
    [h / 6.0, if max == 0.0 { 0.0 } else { d / max }, max]
}
impl ColorPicker {
    pub fn visible(&self) -> bool {
        self.open || self.transition.visible() || self.input.active()
    }
    pub fn open(&mut self, id: String, color: [f32; 4]) {
        self.target = Some(id);
        self.color = color;
        self.hsv = hsv(color);
        self.open = true;
        self.just_opened = true;
        self.drag = None;
    }
    pub fn draw(
        &mut self,
        booth: &mut crate::booth::Booth,
        font: &Font,
        bold: &Font,
        blocked: bool,
    ) {
        if let Some(value) = self.input.poll() {
            if let Ok(value) = value.trim().parse::<f32>() {
                if value.is_finite() {
                    self.color[self.channel] = value.clamp(0.0, 255.0) / 255.0;
                    self.hsv = hsv(self.color);
                }
            }
        }
        self.transition.update(self.open);
        if !self.visible() {
            return;
        }
        self.transition.backdrop();
        let size = super::layout::size();
        let w = (size.x - 24.0).min(370.0);
        let h = (size.y - 24.0).min(414.0);
        let panel = self.transition.panel(
            self.open,
            Rect::new((size.x - w) * 0.5, (size.y - h) * 0.5, w, h),
        );
        let mouse = self.transition.mouse(
            self.open,
            if blocked || self.input.active() || self.just_opened {
                vec2(-10000.0, -10000.0)
            } else {
                super::layout::mouse()
            },
        );
        if !self.just_opened
            && !self.input.active()
            && !blocked
            && widgets::dismiss(panel, widgets::popup_close_rect(panel), mouse)
        {
            self.open = false;
            self.drag = None;
        }
        self.just_opened = false;
        let _opacity = self.transition.scope();
        super::shadow::surface(panel, widgets::SURFACE, super::shadow::Elevation::Popup);
        widgets::popup_header(panel, "Color Tint", font, bold);
        let palette = Rect::new(
            panel.x + 14.0,
            panel.y + 54.0,
            w - 64.0,
            (h - 154.0).max(40.0),
        );
        let hue = Rect::new(palette.right() + 12.0, palette.y, 24.0, palette.h);
        let alpha = Rect::new(palette.x, palette.bottom() + 12.0, w - 28.0, 18.0);
        if is_mouse_button_pressed(MouseButton::Left) {
            self.drag = [palette, hue, alpha].iter().position(|r| r.contains(mouse));
        }
        if mouse.x < -9999.0 || !is_mouse_button_down(MouseButton::Left) {
            self.drag = None;
        }
        if let Some(drag) = self.drag {
            match drag {
                0 => {
                    self.hsv[1] = ((mouse.x - palette.x) / palette.w).clamp(0.0, 1.0);
                    self.hsv[2] = (1.0 - (mouse.y - palette.y) / palette.h).clamp(0.0, 1.0);
                }
                1 => self.hsv[0] = ((mouse.y - hue.y) / hue.h).clamp(0.0, 0.99999),
                _ => self.color[3] = ((mouse.x - alpha.x) / alpha.w).clamp(0.0, 1.0),
            }
            let c = rgb(self.hsv[0], self.hsv[1], self.hsv[2]);
            self.color[..3].copy_from_slice(&c);
        }
        for y in 0..48 {
            for x in 0..48 {
                let c = rgb(self.hsv[0], x as f32 / 47.0, 1.0 - y as f32 / 47.0);
                widgets::paint::draw_swatch(
                    palette.x + x as f32 * palette.w / 48.0,
                    palette.y + y as f32 * palette.h / 48.0,
                    palette.w / 48.0 + 0.5,
                    palette.h / 48.0 + 0.5,
                    Color::new(c[0], c[1], c[2], 1.0),
                );
            }
        }
        for y in 0..96 {
            let c = rgb(y as f32 / 96.0, 1.0, 1.0);
            widgets::paint::draw_swatch(
                hue.x,
                hue.y + y as f32 * hue.h / 96.0,
                hue.w,
                hue.h / 96.0 + 0.5,
                Color::new(c[0], c[1], c[2], 1.0),
            );
        }
        for x in 0..64 {
            let shade = if x / 2 % 2 == 0 { 0.35 } else { 0.55 };
            let a = x as f32 / 63.0;
            widgets::paint::draw_swatch(
                alpha.x + x as f32 * alpha.w / 64.0,
                alpha.y,
                alpha.w / 64.0 + 0.5,
                alpha.h,
                Color::new(
                    shade * (1.0 - a) + self.color[0] * a,
                    shade * (1.0 - a) + self.color[1] * a,
                    shade * (1.0 - a) + self.color[2] * a,
                    1.0,
                ),
            );
        }
        for (x, y) in [
            (
                palette.x + self.hsv[1] * palette.w,
                palette.y + (1.0 - self.hsv[2]) * palette.h,
            ),
            (hue.x + hue.w * 0.5, hue.y + self.hsv[0] * hue.h),
            (alpha.x + self.color[3] * alpha.w, alpha.y + alpha.h * 0.5),
        ] {
            draw_rectangle(x - 4.0, y - 4.0, 8.0, 8.0, BLACK);
            draw_rectangle(x - 2.0, y - 2.0, 4.0, 4.0, WHITE);
        }
        let field_w = (w - 34.0) / 4.0;
        for (i, name) in ["R", "G", "B", "A"].iter().enumerate() {
            let r = Rect::new(
                panel.x + 14.0 + i as f32 * (field_w + 2.0),
                alpha.bottom() + 12.0,
                field_w,
                32.0,
            );
            draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(30, 31, 40, 255));
            draw_text_ex(
                &format!("{} {}", name, (self.color[i] * 255.0).round() as u32),
                r.x + 6.0,
                r.y + 21.0,
                TextParams {
                    font: Some(font),
                    font_size: 12,
                    color: WHITE,
                    ..Default::default()
                },
            );
            if is_mouse_button_pressed(MouseButton::Left) && r.contains(mouse) {
                super::audio::click();
                self.channel = i;
                self.input.open_with(
                    "Color Tint",
                    "Enter channel value (0–255)",
                    &format!("{}", (self.color[i] * 255.0).round() as u32),
                );
            }
        }
        if let Some(c) = booth
            .characters
            .iter_mut()
            .find(|c| Some(&c.id) == self.target.as_ref())
        {
            c.placement.color = self.color;
        }
    }
}
