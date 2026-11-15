//! Scene artwork belongs to the gallery, not the booth's full-size sprite cache.
use super::{booth::control_label, drag_scroll::DragScroll, layout, widgets};
use crate::booth::{Background, Booth, resource_path};
use macroquad::prelude::*;
use std::collections::HashMap;
use widgets::paint::{draw_line, draw_rectangle, draw_texture_ex};

const ACCENT: Color = Color::new(0.925, 0.604, 0.361, 1.0);
const CONTROL: Color = Color::new(0.12, 0.125, 0.16, 1.0);
const EDGE: Color = Color::new(0.23, 0.235, 0.27, 1.0);
const GAP: f32 = 10.0;
const CAPTION: f32 = 34.0;

pub fn name(background: &Background) -> &str {
    if background.transparency_option {
        "Green screen"
    } else {
        background.img.split('/').nth(1).unwrap_or(&background.img)
    }
}

fn columns(width: f32) -> usize {
    (((width + GAP) / 194.0).floor() as usize).clamp(1, 3)
}

fn card_size(width: f32, columns: usize) -> Vec2 {
    let width = (width - GAP * (columns - 1) as f32) / columns as f32;
    vec2(width, (width - 2.0) * 9.0 / 16.0 + CAPTION + 2.0)
}

#[derive(Default)]
pub struct SceneGallery {
    scroll: f32,
    drag: DragScroll,
    thumbnails: HashMap<String, Option<Texture2D>>,
    reveal_selected: bool,
}

impl SceneGallery {
    pub fn open(&mut self) {
        self.drag.cancel();
        self.reveal_selected = true;
    }

    pub fn cancel(&mut self) {
        self.drag.cancel();
    }

    pub fn panel(size: Vec2, count: usize) -> Rect {
        let width = (size.x - 24.0).clamp(1.0, 800.0);
        let inner = (width - 24.0).max(1.0);
        let columns = columns((inner - 8.0).max(1.0));
        let card = card_size((inner - 8.0).max(1.0), columns);
        let rows = count.div_ceil(columns);
        let content = rows as f32 * (card.y + GAP) - GAP;
        let height = (content + 116.0).min((size.y - 40.0).max(1.0));
        Rect::new(
            (size.x - width) * 0.5,
            (size.y - height) * 0.5,
            width,
            height,
        )
    }

    pub fn draw(
        &mut self,
        panel: Rect,
        booth: &Booth,
        font: &Font,
        bold: &Font,
        mouse: Vec2,
    ) -> Option<usize> {
        let summary = Rect::new(panel.x + 12.0, panel.y + 45.0, panel.w - 24.0, 24.0);
        control_label(
            &format!("{} scenes / Choose your backdrop", booth.backgrounds.len()),
            summary,
            11,
            font,
            GRAY,
            false,
        );
        let area = Rect::new(
            panel.x + 12.0,
            panel.y + 76.0,
            (panel.w - 24.0).max(1.0),
            (panel.h - 116.0).max(1.0),
        );
        let grid_width = (area.w - 8.0).max(1.0);
        let columns = columns(grid_width);
        let card = card_size(grid_width, columns);
        let rows = booth.backgrounds.len().div_ceil(columns);
        let content = (rows as f32 * (card.y + GAP) - GAP).max(0.0);
        let max = (content - area.h).max(0.0);
        if self.reveal_selected {
            let top = (booth.background / columns) as f32 * (card.y + GAP);
            if top < self.scroll {
                self.scroll = top;
            } else if top + card.y > self.scroll + area.h {
                self.scroll = top + card.y - area.h;
            }
            self.reveal_selected = false;
        }
        let tap = widgets::scroll(area, mouse, &mut self.scroll, max, &mut self.drag);
        let mut chosen = None;
        let mut loads = 0;
        layout::clip(area);
        for (index, background) in booth.backgrounds.iter().enumerate() {
            let rect = Rect::new(
                area.x + (index % columns) as f32 * (card.x + GAP),
                area.y + (index / columns) as f32 * (card.y + GAP) - self.scroll,
                card.x,
                card.y,
            );
            if rect.bottom() <= area.y || rect.y >= area.bottom() {
                continue;
            }
            let selected = index == booth.background;
            let hovered = area.contains(mouse) && rect.contains(mouse);
            let border = if selected {
                ACCENT
            } else if hovered {
                GRAY
            } else {
                EDGE
            };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, border);
            draw_rectangle(
                rect.x + 1.0,
                rect.y + 1.0,
                rect.w - 2.0,
                rect.h - 2.0,
                if selected {
                    Color::from_rgba(53, 44, 41, 255)
                } else {
                    CONTROL
                },
            );
            let image = Rect::new(
                rect.x + 1.0,
                rect.y + 1.0,
                rect.w - 2.0,
                rect.h - CAPTION - 2.0,
            );
            let path = resource_path(&background.thumb);
            if !self.thumbnails.contains_key(&path) && loads < 2 {
                // Decode only visible cards, at most two per frame. Failed loads are
                // retained too, so a missing resource never retries every frame.
                let loaded = std::fs::read(crate::assets::path(&path))
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| {
                        Image::from_file_with_format(&bytes, Some(ImageFormat::Png))
                            .map_err(|error| error.to_string())
                    });
                let texture = match loaded {
                    Ok(image) => {
                        let texture = Texture2D::from_image(&image);
                        texture.set_filter(FilterMode::Linear);
                        Some(texture)
                    }
                    Err(error) => {
                        tracing::warn!("Cannot load scene thumbnail {path}: {error}");
                        None
                    }
                };
                self.thumbnails.insert(path.clone(), texture);
                loads += 1;
            }
            if let Some(Some(texture)) = self.thumbnails.get(&path) {
                // Preserve the entire dedicated thumbnail, without cropping or
                // theme recoloring. The neutral green-screen artwork needs RGB tint.
                let scale = (image.w / texture.width()).min(image.h / texture.height());
                let dest = vec2(texture.width(), texture.height()) * scale;
                let tint = if background.transparency_option {
                    Color::new(
                        background.color[0],
                        background.color[1],
                        background.color[2],
                        1.0,
                    )
                } else {
                    WHITE
                };
                draw_texture_ex(
                    texture,
                    image.x + (image.w - dest.x) * 0.5,
                    image.y + (image.h - dest.y) * 0.5,
                    tint,
                    DrawTextureParams {
                        dest_size: Some(dest),
                        ..Default::default()
                    },
                );
            } else {
                control_label("Preview unavailable", image, 11, font, GRAY, true);
            }
            if selected {
                let badge = Rect::new(image.x + 6.0, image.y + 6.0, 56.0, 20.0);
                draw_rectangle(badge.x, badge.y, badge.w, badge.h, widgets::SURFACE);
                control_label("ACTIVE", badge, 10, bold, ACCENT, true);
            }
            let caption = Rect::new(rect.x + 5.0, image.bottom(), rect.w - 32.0, CAPTION);
            control_label(
                name(background),
                caption,
                13,
                bold,
                if selected { ACCENT } else { WHITE },
                false,
            );
            if selected {
                let x = rect.right() - 18.0;
                let y = image.bottom() + CAPTION * 0.5;
                draw_line(x - 4.0, y, x - 1.0, y + 3.0, 1.5, ACCENT);
                draw_line(x - 1.0, y + 3.0, x + 5.0, y - 4.0, 1.5, ACCENT);
                draw_rectangle(rect.x, rect.bottom() - 2.0, rect.w, 2.0, ACCENT);
            }
            if tap && hovered {
                chosen = Some(index);
            }
        }
        if max > 0.0 {
            let track_x = area.right() - 3.0;
            draw_rectangle(track_x, area.y, 2.0, area.h, CONTROL);
            let thumb_height = (area.h * area.h / content).max(18.0).min(area.h);
            let thumb_y = area.y + (area.h - thumb_height) * self.scroll / max;
            draw_rectangle(track_x, thumb_y, 2.0, thumb_height, ACCENT);
        }
        layout::end_clip();
        draw_line(
            panel.x + 12.0,
            panel.bottom() - 32.0,
            panel.right() - 12.0,
            panel.bottom() - 32.0,
            1.0,
            EDGE,
        );
        control_label(
            &format!("Selected / {}", name(&booth.backgrounds[booth.background])),
            Rect::new(panel.x + 12.0, panel.bottom() - 30.0, panel.w - 24.0, 26.0),
            11,
            font,
            GRAY,
            false,
        );
        chosen
    }
}
