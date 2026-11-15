//! Stepped UI scale and shared layout geometry.
use macroquad::prelude::*;

pub const MARGIN: f32 = 12.0;
pub const GAP: f32 = 4.0;
pub const INSPECTOR_PADDING: f32 = 8.0;
pub const CONTROL_HEIGHT: f32 = 34.0;
pub const TOPBAR_HEIGHT: f32 = 52.0;
pub const TOOLS_Y: f32 = TOPBAR_HEIGHT + 8.0;

/// One geometry source for the booth, its equipment row and the Settings opener.
pub struct BoothLayout {
    pub wrapped: bool,
    pub actions: [Rect; 5],
    pub settings: Rect,
    pub output: [Rect; 2],
    pub equipment: Rect,
    pub ratio: Rect,
    pub stage: Rect,
    pub inspector: Rect,
    pub footer: Rect,
}

impl BoothLayout {
    pub fn new(size: Vec2, inspector: bool) -> Self {
        let wrapped = size.x < 840.0;
        let settings_w = if wrapped { 82.0 } else { 98.0 };
        let mut settings = Rect::new(
            size.x - MARGIN - settings_w,
            9.0,
            settings_w,
            CONTROL_HEIGHT,
        );
        let action_w = if wrapped {
            ((settings.x - MARGIN - GAP * 5.0) / 5.0).max(1.0)
        } else {
            88.0
        };
        let actions = std::array::from_fn(|i| {
            Rect::new(
                MARGIN + i as f32 * (action_w + GAP),
                9.0,
                action_w,
                CONTROL_HEIGHT,
            )
        });
        let equipment = Rect::new(MARGIN, TOOLS_Y, 96.0, CONTROL_HEIGHT);
        let ratio = Rect::new(equipment.right() + MARGIN, TOOLS_Y, 78.0, CONTROL_HEIGHT);
        let mut footer = Rect::new(
            MARGIN,
            size.y - MARGIN - 28.0,
            (size.x - MARGIN * 2.0).max(1.0),
            28.0,
        );
        let stage_top = TOOLS_Y + CONTROL_HEIGHT + MARGIN;
        let content_bottom = footer.y - MARGIN;
        let side = size.x >= 700.0 || (size.x >= 460.0 && size.y < 500.0);
        // Match the eight-pixel gap below the top bar at the window's bottom.
        // In stacked layouts the role footer still needs its own full-width row.
        let panel_bottom = if side { size.y - 8.0 } else { footer.y - GAP };
        let panel = if side {
            let width = (size.x * 0.42).clamp(220.0, 286.0);
            let top = if wrapped { stage_top } else { TOOLS_Y };
            Rect::new(
                size.x - MARGIN - width,
                top,
                width,
                (panel_bottom - top).max(1.0),
            )
        } else {
            let top = (size.y * 0.40).max(stage_top + 64.0);
            Rect::new(MARGIN, top, footer.w, (panel_bottom - top).max(1.0))
        };
        if inspector && side {
            footer.w = (panel.x - MARGIN - footer.x).max(1.0);
        }
        let right = if inspector && side {
            panel.x - MARGIN
        } else {
            size.x - MARGIN
        };
        let bottom = if inspector && !side {
            panel.y - MARGIN
        } else {
            content_bottom
        };
        let stage = Rect::new(
            MARGIN,
            stage_top,
            (right - MARGIN).max(1.0),
            (bottom - stage_top).max(1.0),
        );
        let (output_left, output_right, output_w, output_y) = if wrapped {
            let right = panel.right();
            let left = panel.x.max(ratio.right() + MARGIN);
            let width = ((right - left - GAP) * 0.5).clamp(1.0, 90.0);
            (right - width * 2.0 - GAP, right, width, TOOLS_Y)
        } else {
            // Align all three toolbar controls with the inspector's three tabs.
            let left = panel.x + INSPECTOR_PADDING;
            let width = (panel.w - INSPECTOR_PADDING * 2.0 - GAP * 2.0) / 3.0;
            settings.x = left + (width + GAP) * 2.0;
            settings.w = width;
            (left, left + width * 2.0 + GAP, width, 9.0)
        };
        let output = std::array::from_fn(|i| {
            Rect::new(
                if i == 0 {
                    output_left
                } else {
                    output_right - output_w
                },
                output_y,
                output_w,
                CONTROL_HEIGHT,
            )
        });
        Self {
            wrapped,
            actions,
            settings,
            output,
            equipment,
            ratio,
            stage,
            inspector: panel,
            footer,
        }
    }

    pub fn canvas(&self, aspect: f32) -> Rect {
        let height = self.stage.h.min(self.stage.w / aspect);
        let width = height * aspect;
        Rect::new(
            self.stage.x + (self.stage.w - width) * 0.5,
            self.stage.y + (self.stage.h - height) * 0.5,
            width,
            height,
        )
    }
}

pub fn preview_info_button(size: Vec2) -> Rect {
    Rect::new(size.x - 76.0, 10.0, 60.0, 32.0)
}

/// Shared foot anchor for both the Spine framing and its decorative coordinate plane.
pub fn preview_origin(area: Rect, size: Vec2, compact: bool) -> Vec2 {
    vec2(
        area.x + area.w * 0.5,
        if compact {
            area.y + area.h * 0.90
        } else {
            size.y * 0.80
        },
    )
}

pub fn preview_back_button(size: Vec2, compact: bool) -> Rect {
    if compact {
        let info = preview_info_button(size);
        Rect::new((info.x - 8.0 - 158.0).max(0.0), info.y, 158.0, info.h)
    } else {
        Rect::new((size.x - 174.0).max(0.0), size.y - 40.0, 158.0, 32.0)
    }
}

/// Clip GUI content in physical framebuffer pixels, including partially visible rows.
pub fn clip(area: Rect) {
    let offset = super::widgets::paint::offset();
    let physical = macroquad::miniquad::window::screen_size();
    let logical = size();
    let (sx, sy) = (physical.0 / logical.x, physical.1 / logical.y);
    unsafe {
        get_internal_gl().quad_gl.scissor(Some((
            ((area.x + offset.x) * sx).floor() as i32,
            ((area.y + offset.y) * sy).floor() as i32,
            (area.w * sx).ceil() as i32,
            (area.h * sy).ceil() as i32,
        )));
    }
}

pub fn end_clip() {
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
}

pub fn dimensions(width: f32, height: f32) -> (f32, f32, f32) {
    let available = (width / 1280.0).min(height / 720.0);
    let scale = [1.0_f32, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0]
        .into_iter()
        .filter(|step| *step <= available)
        .last()
        .unwrap_or(1.0);
    (width / scale, height / scale, scale)
}

pub fn compact() -> bool {
    let size = size();
    size.x < 1000.0 || size.y < 560.0
}

/// Persistent catalog and preview occupy disjoint regions, even on small windows.
pub fn regions(size: Vec2) -> (Rect, Rect) {
    let compact = size.x < 1000.0 || size.y < 560.0;
    let top = if compact { 82.0 } else { 100.0 };
    let bottom = size.y - if compact { 40.0 } else { 70.0 };
    let height = (bottom - top).max(1.0);
    if size.x < 620.0 && !(size.x >= 460.0 && size.y < 560.0) {
        let preview_height = (height * 0.38).min(200.0);
        (
            Rect::new(12.0, top, (size.x - 24.0).max(1.0), preview_height),
            Rect::new(
                12.0,
                top + preview_height + 8.0,
                (size.x - 24.0).max(1.0),
                (height - preview_height - 8.0).max(1.0),
            ),
        )
    } else {
        let width = if size.x < 620.0 {
            (size.x * 0.50).clamp(220.0, 280.0)
        } else if compact {
            (size.x * 0.44).clamp(280.0, 420.0)
        } else {
            (size.x * 0.34).clamp(250.0, 420.0)
        };
        let left = if compact {
            12.0
        } else {
            28.0 + ((size.x - 56.0) * 0.18).clamp(130.0, 205.0) + 24.0
        };
        let panel_x = size.x - width - 16.0;
        (
            Rect::new(left, top, (panel_x - left - 68.0).max(1.0), height),
            Rect::new(panel_x, top, width, height),
        )
    }
}

pub fn size() -> Vec2 {
    let (w, h, _) = dimensions(screen_width(), screen_height());
    vec2(w, h)
}

pub fn mouse() -> Vec2 {
    let (_, _, scale) = dimensions(screen_width(), screen_height());
    Vec2::from(mouse_position()) / scale - super::widgets::paint::offset()
}

pub fn begin() {
    let size = size();
    set_camera(&canvas_camera(size));
}

fn canvas_camera(size: Vec2) -> Camera2D {
    // Camera2D flips Y itself when drawing to the screen (not a render texture).
    // Positive zoom therefore preserves our top-left, Y-down UI coordinates.
    Camera2D {
        target: size * 0.5,
        zoom: vec2(2.0 / size.x, 2.0 / size.y),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_and_catalog_never_overlap() {
        for (w, h) in [
            (480.0, 480.0),
            (480.0, 320.0),
            (640.0, 360.0),
            (600.0, 720.0),
            (800.0, 540.0),
            (960.0, 720.0),
            (1280.0, 720.0),
        ] {
            let (preview, catalog) = regions(vec2(w, h));
            assert!(!preview.overlaps(&catalog));
            assert!(catalog.w > 0.0 && catalog.h > 0.0);
            assert!(catalog.x >= 0.0 && catalog.x + catalog.w <= w);
        }
    }
    #[test]
    fn canvas_origin_is_top_left_and_y_points_down() {
        use macroquad::camera::Camera;
        let matrix = canvas_camera(vec2(1280.0, 720.0)).matrix();
        let top_left = matrix.transform_point3(vec3(0.0, 0.0, 0.0));
        let bottom_right = matrix.transform_point3(vec3(1280.0, 720.0, 0.0));
        assert!((top_left.x + 1.0).abs() < 0.001);
        assert!((top_left.y - 1.0).abs() < 0.001);
        assert!((bottom_right.x - 1.0).abs() < 0.001);
        assert!((bottom_right.y + 1.0).abs() < 0.001);
    }
    #[test]
    fn scale_is_stepped_and_never_shrinks_small_window_controls() {
        for (w, h, expected) in [
            (480.0, 320.0, 1.0),
            (960.0, 540.0, 1.0),
            (1280.0, 720.0, 1.0),
            (1500.0, 850.0, 1.0),
            (1600.0, 900.0, 1.25),
            (1920.0, 1080.0, 1.5),
            (2560.0, 1440.0, 2.0),
        ] {
            assert_eq!(dimensions(w, h).2, expected);
        }
    }
}
