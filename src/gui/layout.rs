//! Stepped UI scale and shared layout geometry.
use macroquad::prelude::*;

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
    if size.x < 620.0 {
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
        let width = if compact {
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
            Rect::new(left, top, (panel_x - left - 12.0).max(1.0), height),
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
    Vec2::from(mouse_position()) / scale
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
