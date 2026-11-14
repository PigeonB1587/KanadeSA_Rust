//! Shared popup transitions, dismissal and scrolling for the immediate-mode GUI.
use macroquad::prelude::*;

pub const DURATION: f64 = 0.28;
pub const SLIDE: f32 = 60.0;
pub const BLOCKED: Vec2 = Vec2::new(-10000.0, -10000.0);
pub const SURFACE: Color = Color::new(0.078, 0.082, 0.11, 0.98);
pub const PREVIEW_SURFACE: Color = SURFACE;
/// The fullscreen Preview backing lets 30% of the booth show through.
pub const PREVIEW_BACKDROP: Color = Color::new(0.13, 0.134, 0.16, 0.70);
pub const HEADER_HEIGHT: f32 = 50.0;

pub fn popup_close_rect(panel: Rect) -> Rect {
    Rect::new(panel.right() - 76.0, panel.y + 8.0, 64.0, 30.0)
}

/// All popup headers use the scene picker's title and filled Close control.
/// Activation belongs to dismiss_layer so every dismissal plays exactly once.
pub fn popup_header(panel: Rect, title: &str, _font: &Font, bold: &Font) -> Rect {
    let close = popup_close_rect(panel);
    paint::draw_text_ex(
        title,
        panel.x + 14.0,
        panel.y + 27.0,
        TextParams {
            font: Some(bold),
            font_size: 18,
            color: WHITE,
            ..Default::default()
        },
    );
    paint::draw_rectangle(
        close.x,
        close.y,
        close.w,
        close.h,
        Color::new(0.12, 0.125, 0.16, 1.0),
    );
    paint::draw_text_ex(
        "Close",
        close.x + 8.0,
        close.y + close.h * 0.5 + 5.0,
        TextParams {
            font: Some(bold),
            font_size: 12,
            color: WHITE,
            ..Default::default()
        },
    );
    close
}

#[derive(Default)]
pub struct Transition {
    target: bool,
    value: f32,
    from: f32,
    started: f64,
}
impl Transition {
    pub fn update(&mut self, open: bool) -> f32 {
        self.update_at(open, get_time())
    }
    fn update_at(&mut self, open: bool, now: f64) -> f32 {
        // Sample the old direction first: reversing midway never jumps.
        self.sample(now);
        if open != self.target {
            self.from = self.value;
            self.started = now;
            self.target = open;
        }
        self.value
    }
    fn sample(&mut self, now: f64) {
        let t = ((now - self.started) / DURATION).clamp(0.0, 1.0) as f32;
        self.value = self.from + (f32::from(self.target) - self.from) * (1.0 - (1.0 - t).powi(3));
    }
    pub fn visible(&self) -> bool {
        self.target || self.value > 0.001
    }
    pub fn panel(&mut self, open: bool, mut rect: Rect) -> Rect {
        let p = self.update(open);
        rect.y += (1.0 - p) * SLIDE;
        rect
    }
    /// The opening click and every closing frame belong to the modal, never its content.
    pub fn mouse(&self, open: bool, mouse: Vec2) -> Vec2 {
        if open && self.target && self.value > 0.001 {
            mouse
        } else {
            BLOCKED
        }
    }
    pub fn scope(&self) -> paint::Opacity {
        paint::Opacity::new(self.value)
    }
    pub fn slide(&self) -> paint::Translation {
        paint::Translation::new(vec2(0.0, (1.0 - self.value) * SLIDE))
    }
    pub fn backdrop(&self) {
        let _opacity = self.scope();
        let size = super::layout::size();
        paint::draw_rectangle(0.0, 0.0, size.x, size.y, Color::new(0.0, 0.0, 0.0, 0.55));
    }
}

pub fn dismiss(panel: Rect, close: Rect, mouse: Vec2) -> bool {
    dismiss_except(panel, close, Rect::new(0.0, 0.0, 0.0, 0.0), mouse)
}

/// Ignore an opener's pointer click without suppressing Escape above that button.
pub fn dismiss_except(panel: Rect, close: Rect, opener: Rect, mouse: Vec2) -> bool {
    dismiss_layer(panel, close, opener, mouse, true)
}

/// Nested popups retain Escape until their exit has finished.
pub fn dismiss_layer(
    panel: Rect,
    close: Rect,
    opener: Rect,
    mouse: Vec2,
    allow_escape: bool,
) -> bool {
    let dismissed = dismissal_requested(
        panel,
        close,
        opener,
        mouse,
        allow_escape && is_key_pressed(KeyCode::Escape),
        is_mouse_button_pressed(MouseButton::Left),
    );
    if dismissed {
        super::audio::click();
    }
    dismissed
}

fn dismissal_requested(
    panel: Rect,
    close: Rect,
    opener: Rect,
    mouse: Vec2,
    escape: bool,
    pressed: bool,
) -> bool {
    mouse.x > -9999.0
        && (escape
            || (pressed
                && !opener.contains(mouse)
                && (close.contains(mouse) || !panel.contains(mouse))))
}

pub fn scroll(
    area: Rect,
    mouse: Vec2,
    offset: &mut f32,
    max: f32,
    drag: &mut super::drag_scroll::DragScroll,
) -> bool {
    *offset = offset.clamp(0.0, max.max(0.0));
    if mouse.x <= -9999.0 {
        drag.cancel();
        return false;
    }
    if area.contains(mouse) {
        *offset = (*offset - mouse_wheel().1 * 36.0).clamp(0.0, max);
    }
    drag.update(
        area,
        mouse,
        is_mouse_button_pressed(MouseButton::Left),
        is_mouse_button_down(MouseButton::Left),
        is_mouse_button_released(MouseButton::Left),
        offset,
        max,
    )
}

/// Scoped alpha applies to *all* primitives (including text and texture tint).
/// Nested dropdowns inherit their parent's fade; dropping the guard restores it.
pub mod paint {
    use macroquad::prelude::*;
    use std::cell::Cell;
    thread_local! { static ALPHA: Cell<f32> = const { Cell::new(1.0) }; }
    thread_local! { static OFFSET: Cell<Vec2> = const { Cell::new(Vec2::ZERO) }; }
    pub fn offset() -> Vec2 {
        OFFSET.with(Cell::get)
    }
    pub struct Translation(Vec2);
    impl Translation {
        pub fn new(offset: Vec2) -> Self {
            Self(OFFSET.with(|value| value.replace(value.get() + offset)))
        }
    }
    impl Drop for Translation {
        fn drop(&mut self) {
            OFFSET.with(|value| value.set(self.0));
        }
    }
    pub struct Opacity(f32);
    impl Opacity {
        pub fn new(alpha: f32) -> Self {
            Self(ALPHA.with(|value| value.replace(value.get() * alpha.clamp(0.0, 1.0))))
        }
    }
    impl Drop for Opacity {
        fn drop(&mut self) {
            ALPHA.with(|value| value.set(self.0));
        }
    }
    fn tint(mut color: Color) -> Color {
        color.a *= ALPHA.with(Cell::get);
        color
    }
    #[cfg(test)]
    pub(super) fn tinted(color: Color) -> Color {
        tint(color)
    }
    pub fn draw_rectangle(x: f32, y: f32, w: f32, h: f32, color: Color) {
        let offset = offset();
        macroquad::shapes::draw_rectangle(x + offset.x, y + offset.y, w, h, tint(color));
    }
    pub fn draw_line(x: f32, y: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        let offset = offset();
        macroquad::shapes::draw_line(
            x + offset.x,
            y + offset.y,
            x2 + offset.x,
            y2 + offset.y,
            thickness,
            tint(color),
        );
    }
    pub fn draw_text_ex(text: &str, x: f32, y: f32, mut params: TextParams) -> TextDimensions {
        params.color = tint(params.color);
        let offset = offset();
        macroquad::text::draw_text_ex(text, x + offset.x, y + offset.y, params)
    }
    pub fn draw_texture_ex(
        texture: &Texture2D,
        x: f32,
        y: f32,
        color: Color,
        params: DrawTextureParams,
    ) {
        let offset = offset();
        macroquad::texture::draw_texture_ex(
            texture,
            x + offset.x,
            y + offset.y,
            tint(color),
            params,
        );
    }
    #[cfg(test)]
    pub(super) fn alpha() -> f32 {
        ALPHA.with(Cell::get)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transition_endpoints_reversal_and_modal_lifetime() {
        let mut t = Transition::default();
        assert!(!t.visible());
        assert_eq!(t.update_at(true, 1.0), 0.0);
        assert!(t.visible());
        assert_eq!(t.mouse(true, Vec2::ZERO), BLOCKED);
        let middle = t.update_at(true, 1.14);
        assert!((middle - 0.875).abs() < 0.0001);
        assert_eq!(t.update_at(false, 1.14), middle);
        assert!(t.visible());
        assert_eq!(t.mouse(false, Vec2::ZERO), BLOCKED);
        let closing = t.update_at(false, 1.21);
        assert!(closing > 0.0 && closing < middle);
        assert_eq!(t.update_at(true, 1.21), closing);
        assert_eq!(t.update_at(true, 1.6), 1.0);
        t.update_at(false, 1.6);
        assert_eq!(t.update_at(false, 2.0), 0.0);
        assert!(!t.visible());
    }
    #[test]
    fn nested_opacity_restores_parent_and_preserves_source_alpha() {
        assert_eq!(paint::alpha(), 1.0);
        {
            let _parent = paint::Opacity::new(0.5);
            assert_eq!(paint::alpha(), 0.5);
            let source = Color::new(0.2, 0.4, 0.8, 0.6);
            assert_eq!(paint::tinted(source), Color::new(0.2, 0.4, 0.8, 0.3));
            {
                let _child = paint::Opacity::new(0.4);
                assert_eq!(paint::alpha(), 0.2);
            }
            assert_eq!(paint::alpha(), 0.5);
        }
        assert_eq!(paint::alpha(), 1.0);
    }

    #[test]
    fn dismissal_keeps_escape_independent_of_pointer_exclusions() {
        let panel = Rect::new(100.0, 100.0, 200.0, 200.0);
        let close = Rect::new(240.0, 100.0, 60.0, 30.0);
        let opener = Rect::new(100.0, 60.0, 200.0, 30.0);
        assert!(!dismissal_requested(
            panel,
            close,
            opener,
            vec2(150.0, 75.0),
            false,
            true
        ));
        assert!(dismissal_requested(
            panel,
            close,
            opener,
            vec2(150.0, 75.0),
            true,
            false
        ));
        assert!(dismissal_requested(
            panel,
            close,
            opener,
            vec2(270.0, 115.0),
            false,
            true
        ));
        assert!(dismissal_requested(
            panel,
            close,
            opener,
            Vec2::ZERO,
            false,
            true
        ));
        assert!(!dismissal_requested(
            panel,
            close,
            opener,
            vec2(150.0, 150.0),
            false,
            true
        ));
        assert!(!dismissal_requested(
            panel, close, opener, BLOCKED, true, true
        ));
    }
}
