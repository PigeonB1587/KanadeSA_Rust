//! Preview-only perspective reference plane, anchored to the character's feet.
//! All dimensions are logical pixels; this layer never participates in scene capture.
use super::{layout, widgets::paint};
use macroquad::prelude::*;

#[derive(Default)]
pub struct CoordinateGrid {
    extent: f32,
    velocity: f32,
}

struct Projection {
    origin: Vec2,
    width: f32,
    depth: f32,
    height: f32,
}

impl Projection {
    fn point(&self, x: f32, y: f32, z: f32) -> Vec2 {
        // A shallow ground plane with level horizontal lines. Its depth and
        // upright reference stay small while the horizontal extent expands.
        let perspective = 1.0 / (1.0 + z * 0.16);
        self.origin
            + vec2(
                x * self.width + z * self.depth * 0.5,
                -y * self.height - z * self.depth,
            ) * perspective
    }
}

fn theme(dark: Color, day: Color) -> Color {
    let t = paint::light_amount();
    Color::new(
        dark.r + (day.r - dark.r) * t,
        dark.g + (day.g - dark.g) * t,
        dark.b + (day.b - dark.b) * t,
        dark.a + (day.a - dark.a) * t,
    )
}

fn alpha(mut color: Color, amount: f32) -> Color {
    color.a *= amount;
    color
}

fn stroke(a: Vec2, b: Vec2, width: f32, color: Color) {
    draw_line(a.x, a.y, b.x, b.y, width, color);
}

/// Fade toward the horizon without cutting off the horizontal screen-wide span.
fn feather(z: f32) -> f32 {
    1.0 - ((z + 0.33) / 1.5).clamp(0.0, 1.0) * 0.80
}

fn ground_line(projection: &Projection, a: Vec2, b: Vec2, width: f32, color: Color) {
    for i in 0..24 {
        let start = a.lerp(b, i as f32 / 24.0);
        let end = a.lerp(b, (i + 1) as f32 / 24.0);
        let mid = (start + end) * 0.5;
        stroke(
            projection.point(start.x, 0.0, start.y),
            projection.point(end.x, 0.0, end.y),
            width,
            alpha(color, feather(mid.y)),
        );
    }
}

fn arrow(a: Vec2, b: Vec2, color: Color) {
    stroke(a, b, 1.25, color);
    let direction = (b - a).normalize_or_zero();
    let normal = vec2(-direction.y, direction.x);
    stroke(b, b - direction * 6.0 + normal * 3.0, 1.0, color);
    stroke(b, b - direction * 6.0 - normal * 3.0, 1.0, color);
}

fn label(text: &str, point: Vec2, font: &Font, color: Color) {
    let size = measure_text(text, Some(font), 10, 1.0);
    draw_text_ex(
        text,
        point.x - size.width * 0.5,
        point.y - size.height * 0.5 + size.offset_y,
        TextParams {
            font: Some(font),
            font_size: 10,
            color,
            ..Default::default()
        },
    );
}

impl CoordinateGrid {
    pub fn draw(&mut self, open: bool, opacity: f32, font: &Font) {
        // A damped spring gives the entrance a small overshoot. The critically
        // damped exit contracts into the origin during the Preview's own fade.
        let dt = get_frame_time().clamp(0.0, 0.05) / 4.0;
        let (frequency, damping) = if open { (21.0, 0.72) } else { (26.0, 1.0) };
        for _ in 0..4 {
            self.velocity += (frequency * frequency * (f32::from(open) - self.extent)
                - 2.0 * damping * frequency * self.velocity)
                * dt;
            self.extent = (self.extent + self.velocity * dt).max(0.0);
        }
        if opacity <= 0.001 || self.extent <= 0.001 {
            return;
        }

        let size = layout::size();
        let (area, _) = layout::regions(size);
        let origin = layout::preview_origin(area, size, layout::compact());
        // The plane reaches both window edges behind the UI. Keep the character's
        // foot origin, but do not clip the horizontal axis at the catalog rail.
        let grid_area = Rect::new(0.0, area.y, size.x, area.h);
        let expansion = self.extent.min(1.06);
        let depth = (area.h * 0.16)
            .min(72.0)
            .min(((area.bottom() - origin.y - 8.0) * 2.7).max(1.0));
        let cell = 48.0;
        let first_column = -(origin.x * 1.3 / cell).ceil() as i32 - 2;
        let last_column = ((size.x - origin.x) * 1.3 / cell).ceil() as i32 + 2;
        let projection = Projection {
            origin: origin + paint::offset(),
            width: cell * expansion,
            depth,
            height: (area.h * 0.10).clamp(16.0, 48.0),
        };
        // Explicit theme colors bypass semantic surface recoloring. Opacity and
        // translation match the surrounding Preview, including interrupted exits.
        let fade = opacity.clamp(0.0, 1.0);
        let grid = alpha(
            theme(
                Color::new(0.58, 0.66, 0.73, 0.23),
                Color::new(0.25, 0.32, 0.38, 0.27),
            ),
            fade,
        );
        let floor = alpha(
            theme(
                Color::new(0.44, 0.57, 0.67, 0.038),
                Color::new(0.25, 0.33, 0.42, 0.045),
            ),
            fade,
        );
        let warm = alpha(
            theme(
                Color::new(0.925, 0.66, 0.43, 0.66),
                Color::new(0.58, 0.31, 0.14, 0.76),
            ),
            fade,
        );
        let cool = alpha(
            theme(
                Color::new(0.41, 0.70, 0.74, 0.52),
                Color::new(0.14, 0.40, 0.43, 0.65),
            ),
            fade,
        );
        layout::clip(grid_area);

        // Restrained alternating tiles supply volume without an opaque backdrop.
        for row in 0..8 {
            let z0 = -0.33 + row as f32 * 0.165;
            let z1 = z0 + 0.165;
            for col in first_column..last_column {
                let x0 = col as f32;
                let x1 = x0 + 1.0;
                let color = alpha(
                    floor,
                    feather((z0 + z1) * 0.5) * if (row + col) % 2 == 0 { 1.0 } else { 0.42 },
                );
                let a = projection.point(x0, 0.0, z0);
                let b = projection.point(x1, 0.0, z0);
                let c = projection.point(x1, 0.0, z1);
                let d = projection.point(x0, 0.0, z1);
                draw_triangle(a, b, c, color);
                draw_triangle(a, c, d, color);
            }
        }
        for i in first_column..=last_column {
            let x = i as f32;
            ground_line(
                &projection,
                vec2(x, -0.33),
                vec2(x, 0.99),
                1.0,
                alpha(grid, if i % 2 == 0 { 0.80 } else { 0.36 }),
            );
        }
        for i in -2_i32..=6 {
            let z = i as f32 * 0.165;
            ground_line(
                &projection,
                vec2(first_column as f32, z),
                vec2(last_column as f32, z),
                1.0,
                alpha(grid, if i % 2 == 0 { 1.0 } else { 0.45 }),
            );
        }

        let center = projection.origin;
        let offset = paint::offset();
        let left = vec2(offset.x + origin.x * (1.0 - expansion), center.y);
        let right = vec2(
            offset.x + origin.x + (size.x - origin.x) * expansion,
            center.y,
        );
        ground_line(
            &projection,
            vec2(-origin.x / cell, 0.0),
            vec2((size.x - origin.x) / cell, 0.0),
            4.0,
            alpha(warm, 0.08),
        );
        arrow(center, left, alpha(warm, 0.58));
        arrow(center, right, warm);
        arrow(
            projection.point(0.0, 0.0, -0.30),
            projection.point(0.0, 0.0, 1.12),
            cool,
        );
        let up = projection.point(0.0, 1.0, 0.0);
        arrow(center, up, alpha(grid, 1.45));

        // Fine graduations remain short and quiet beside the stronger axes.
        for i in first_column * 2..=last_column * 2 {
            if i != 0 {
                let x = i as f32 * 0.5;
                let half = if i % 2 == 0 { 3.5 } else { 2.0 };
                let p = projection.point(x, 0.0, 0.0);
                stroke(
                    p - vec2(0.0, half),
                    p + vec2(0.0, half),
                    1.0,
                    alpha(warm, 0.64),
                );
            }
        }
        for i in 1..=6 {
            let z = i as f32 * 0.165;
            stroke(
                projection.point(-0.06, 0.0, z),
                projection.point(0.06, 0.0, z),
                1.0,
                cool,
            );
        }
        for i in 1..=5 {
            let p = projection.point(0.0, i as f32 / 6.0, 0.0);
            stroke(
                p - vec2(3.0, 0.0),
                p + vec2(3.0, 0.0),
                1.0,
                alpha(grid, 0.9),
            );
        }

        // Two projected registration rings make the origin read as a footprint.
        for (radius, strength) in [(24.0, 0.75), (38.0, 0.25)] {
            for i in 0..64 {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                let b = (i + 1) as f32 / 64.0 * std::f32::consts::TAU;
                stroke(
                    projection.point(a.cos() * radius / cell, 0.0, a.sin() * 0.12),
                    projection.point(b.cos() * radius / cell, 0.0, b.sin() * 0.12),
                    1.0,
                    alpha(warm, strength),
                );
            }
        }
        draw_circle(center.x, center.y, 4.0, alpha(warm, 0.12));
        draw_circle(center.x, center.y, 1.8, warm);
        label("−X", left + vec2(16.0, -13.0), font, alpha(warm, 0.8));
        label("+X", right + vec2(-16.0, -13.0), font, warm);
        label("Y", up - vec2(0.0, 12.0), font, alpha(grid, 2.2));
        label(
            "Z",
            projection.point(0.0, 0.0, 1.12) - vec2(0.0, 12.0),
            font,
            cool,
        );
        layout::end_clip();
    }
}
