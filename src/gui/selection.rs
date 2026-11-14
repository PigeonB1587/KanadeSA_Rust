//! Virtualized mouse-driven catalog. Only visible cells request textures.
use crate::character::Character;
use macroquad::prelude::*;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

fn normalize(text: &str, japanese: bool) -> String {
    text.nfkc()
        .map(|c| {
            if japanese && ('\u{30a1}'..='\u{30f6}').contains(&c) {
                char::from_u32(c as u32 - 0x60).unwrap()
            } else {
                c
            }
        })
        .collect::<String>()
        .to_lowercase()
}

const ACCENT: Color = Color::new(0.925, 0.604, 0.361, 1.0);
const CACHE_LIMIT: usize = 128;

#[derive(Default)]
pub struct Selection {
    pub pointer_blocked: bool,
    tab: usize,
    scroll: [f32; 6],
    cache: HashMap<String, (Option<Texture2D>, u64)>,
    tick: u64,
    dragging: bool,
    content_drag: crate::drag_scroll::DragScroll,
    query: String,
    search_focused: bool,
    capture_keyboard: bool,
    last_language: String,
    native_input: super::input::NativeInput,
}

fn matches_query(name: &str, id: &str, query: &str) -> bool {
    query
        .split_whitespace()
        .all(|word| name.to_lowercase().contains(word) || id.to_lowercase().contains(word))
}

#[cfg(test)]
mod tests {
    use super::{matches_query, normalize};
    use std::collections::HashMap;
    #[test]
    fn japanese_search_matches_unity_kana_and_width_rules() {
        assert_eq!(normalize("ＦＯＸ ｷﾂﾈ", true), normalize("fox きつね", true));
        assert!(matches_query(
            &normalize("スーパーキツネ", true),
            "CharFox",
            &normalize("ｷﾂﾈ", true)
        ));
        assert!(matches_query("超级狐狸", "CharFox", "狐狸"));
    }
    #[test]
    fn search_matches_names_ids_and_multiple_words() {
        assert!(matches_query(
            "Super Yellow Fox",
            "CharFox_Yellow",
            "yellow fox"
        ));
        assert!(matches_query(
            "Super Yellow Fox",
            "CharFox_Yellow",
            "charfox"
        ));
        assert!(matches_query("None", "Neck_Null", "none"));
        assert!(matches_query("Super Fox", "CharFox", "  "));
        assert!(!matches_query("Super Fox", "CharFox", "blue"));
    }
    #[test]
    fn localized_search_uses_translation_and_keeps_id_lookup() {
        let languages: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(include_str!("../../configs/_languages.json")).unwrap();
        let chinese = &languages["Item.CharFox"]["schinese"];
        assert!(matches_query(chinese, "charfox", "狐狸"));
        assert!(!matches_query(chinese, "charfox", "super"));
        assert!(matches_query(chinese, "charfox", "charfox"));
    }
}

fn text(s: &str, x: f32, y: f32, font: &Font, color: Color, size: u16) {
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

impl Selection {
    pub fn captures_keyboard(&self) -> bool {
        self.capture_keyboard || self.native_input.active()
    }
    pub fn release_search(&mut self) {
        self.search_focused = false;
        self.capture_keyboard = false;
    }
    pub async fn draw(&mut self, character: &mut Character, font: &Font, bold: &Font) {
        self.tick += 1;
        if let Some(query) = self.native_input.poll() {
            self.query = query.chars().filter(|c| !c.is_control()).take(80).collect();
            self.scroll = [0.0; 6];
            self.content_drag.cancel();
        }
        let size = crate::gui_scale::size();
        let w = size.x;
        let compact = crate::gui_scale::compact();
        let mouse = if self.pointer_blocked || self.native_input.active() {
            vec2(-10000.0, -10000.0)
        } else {
            crate::gui_scale::mouse()
        };
        let click = is_mouse_button_pressed(MouseButton::Left);
        let (_, panel) = crate::gui_scale::regions(size);
        let (x, top, width, bottom) = (panel.x, panel.y, panel.w, panel.y + panel.h);
        // Header animation controls remain available without reaching for the keyboard.
        for (offset, title, direction) in [(58.0, "<", -1), (28.0, ">", 1)] {
            let r = Rect::new(
                w - offset - 16.0,
                if compact { 48.0 } else { 64.0 },
                28.0,
                28.0,
            );
            text(
                title,
                r.x + 7.0,
                r.y + 19.0,
                bold,
                if r.contains(mouse) { ACCENT } else { GRAY },
                14,
            );
            if click && r.contains(mouse) {
                super::audio::click();
                character.cycle_animation(direction);
            }
        }
        if bottom - top < 130.0 {
            return;
        }
        let panel = Rect::new(x, top, width, bottom - top);
        draw_rectangle(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            Color::from_rgba(20, 21, 28, 250),
        );
        let tabs = ["Characters", "Neck", "Clothes", "Glasses", "Hats", "Beards"];
        let tab_w = (width - 24.0) / 3.0;
        for (i, title) in tabs.iter().enumerate() {
            let r = Rect::new(
                x + 12.0 + (i % 3) as f32 * tab_w,
                top + 12.0 + (i / 3) as f32 * 30.0,
                tab_w - 4.0,
                26.0,
            );
            let hover = r.contains(mouse);
            if hover && click {
                super::audio::click();
                self.content_drag.cancel();
                self.tab = i;
                self.scroll[i] = 0.0;
            }
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if self.tab == i {
                    Color::from_rgba(53, 44, 41, 255)
                } else {
                    Color::from_rgba(30, 31, 40, 255)
                },
            );
            if self.tab == i {
                draw_rectangle(r.x, r.y + r.h - 2.0, r.w, 2.0, ACCENT);
            }
            text(
                title,
                r.x + 7.0,
                r.y + 17.0,
                bold,
                if self.tab == i || hover { WHITE } else { GRAY },
                if tab_w < 90.0 { 10 } else { 12 },
            );
        }
        let search = Rect::new(x + 12.0, top + 76.0, width - 24.0, 28.0);
        let reset = Rect::new(search.x + search.w - 28.0, search.y, 28.0, 28.0);
        if click && !self.pointer_blocked {
            if search.contains(mouse) && !reset.contains(mouse) {
                super::audio::click();
                self.native_input.open(&self.query);
            }
            self.search_focused = false;
        }
        self.search_focused = self.native_input.active();
        self.capture_keyboard = self.search_focused;
        let previous = self.query.clone();
        if click && reset.contains(mouse) {
            self.query.clear();
        }
        // The native widget owns editing, composition, clipboard and keyboard handling.
        while get_char_pressed().is_some() {}
        if previous != self.query {
            self.content_drag.cancel();
            self.scroll = [0.0; 6];
        }
        draw_rectangle(
            search.x,
            search.y,
            search.w,
            search.h,
            Color::from_rgba(30, 31, 40, 255),
        );
        if self.search_focused {
            draw_rectangle(search.x, search.y + search.h - 1.0, search.w, 1.0, ACCENT);
        }
        let mut shown = if self.query.is_empty() {
            "Search name or ID...".to_owned()
        } else {
            self.query.clone()
        };
        while !shown.is_empty() && measure_text(&shown, Some(font), 12, 1.0).width > search.w - 42.0
        {
            shown.remove(0);
        }
        text(
            &shown,
            search.x + 8.0,
            search.y + 19.0,
            font,
            if self.query.is_empty() { GRAY } else { WHITE },
            12,
        );
        text("x", reset.x + 9.0, reset.y + 19.0, font, GRAY, 14);
        let clear = Rect::new(x + width - 72.0, top + 110.0, 60.0, 22.0);
        text(
            "Clear all",
            clear.x,
            clear.y + 15.0,
            font,
            if clear.contains(mouse) { ACCENT } else { GRAY },
            11,
        );
        if click && clear.contains(mouse) {
            super::audio::click();
            character.clear_outfit();
        }
        let wardrobe = character.wardrobe();
        if self.last_language != wardrobe.language {
            self.scroll = [0.0; 6];
            self.last_language.clone_from(&wardrobe.language);
        }
        let total = if self.tab == 0 {
            wardrobe.characters.len()
        } else {
            wardrobe.items[self.tab - 1].len()
        };
        let japanese = wardrobe.language == "japanese";
        let query = normalize(&self.query, japanese);
        let filtered: Vec<usize> = (0..total)
            .filter(|&index| {
                let id = if self.tab == 0 {
                    &wardrobe.characters[index].inventory_id
                } else {
                    &wardrobe.items[self.tab - 1][index].inventory_id
                };
                let name = if self.tab != 0 && index == 0 {
                    wardrobe.search_name("None")
                } else {
                    wardrobe.search_name(id)
                };
                matches_query(&normalize(name, japanese), &normalize(id, japanese), &query)
            })
            .collect();
        let count = filtered.len();
        let selected = if self.tab == 0 {
            wardrobe.current_character
        } else {
            wardrobe.selected[self.tab - 1]
        };
        text(
            &format!("{} / {} items", count, total),
            x + 12.0,
            top + 125.0,
            font,
            GRAY,
            11,
        );
        let grid = Rect::new(
            x + 12.0,
            top + 140.0,
            width - 34.0,
            (bottom - top - 172.0).max(1.0),
        );
        let columns = ((grid.w + 8.0) / 82.0).floor().max(2.0) as usize;
        let cell = (grid.w + 8.0) / columns as f32;
        let rows = count.div_ceil(columns);
        let max_scroll = (rows as f32 * cell - grid.h).max(0.0);
        let tapped = if self.pointer_blocked {
            self.content_drag.cancel();
            false
        } else {
            self.content_drag.update(
                grid,
                mouse,
                click,
                is_mouse_button_down(MouseButton::Left),
                is_mouse_button_released(MouseButton::Left),
                &mut self.scroll[self.tab],
                max_scroll,
            )
        };
        if grid.contains(mouse) {
            self.scroll[self.tab] -= mouse_wheel().1 * 38.0;
        }
        let track = Rect::new(x + width - 14.0, grid.y, 4.0, grid.h);
        if click && Rect::new(track.x - 6.0, track.y, 16.0, track.h).contains(mouse) {
            self.dragging = true;
        }
        if !is_mouse_button_down(MouseButton::Left) {
            self.dragging = false;
        }
        let thumb_h = (grid.h * grid.h / (rows as f32 * cell).max(grid.h))
            .max(24.0)
            .min(grid.h);
        if self.dragging && max_scroll > 0.0 {
            self.scroll[self.tab] = ((mouse.y - grid.y - thumb_h * 0.5) / (grid.h - thumb_h))
                .clamp(0.0, 1.0)
                * max_scroll;
        }
        self.scroll[self.tab] = self.scroll[self.tab].clamp(0.0, max_scroll);
        draw_rectangle(
            track.x,
            track.y,
            track.w,
            track.h,
            Color::from_rgba(42, 43, 53, 255),
        );
        draw_rectangle(
            track.x,
            grid.y
                + if max_scroll > 0.0 {
                    self.scroll[self.tab] / max_scroll * (grid.h - thumb_h)
                } else {
                    0.0
                },
            track.w,
            thumb_h,
            ACCENT,
        );
        let first_row = (self.scroll[self.tab] / cell).floor() as usize;
        let last_row = ((self.scroll[self.tab] + grid.h) / cell).ceil() as usize;
        let mut chosen = None;
        let mut hovered_name = None;
        let mut loads = 0;
        if count == 0 {
            text("No matches", grid.x, grid.y + 24.0, font, GRAY, 13);
        }
        for position in first_row * columns..((last_row * columns).min(count)) {
            let index = filtered[position];
            let row = position / columns;
            let r = Rect::new(
                grid.x + (position % columns) as f32 * cell,
                grid.y + row as f32 * cell - self.scroll[self.tab],
                cell - 8.0,
                cell - 8.0,
            );
            let clip_top = r.y.max(grid.y);
            let clip_bottom = (r.y + r.h).min(grid.y + grid.h);
            if clip_bottom <= clip_top {
                continue;
            }
            let hit = Rect::new(r.x, clip_top, r.w, clip_bottom - clip_top).contains(mouse);
            let (icon, id) = if self.tab == 0 {
                let c = &wardrobe.characters[index];
                (&c.icon, &c.inventory_id)
            } else {
                let c = &wardrobe.items[self.tab - 1][index];
                (&c.icon, &c.inventory_id)
            };
            if hit {
                hovered_name = Some(wardrobe.display_name(id).to_owned());
                if tapped {
                    chosen = Some(index);
                }
            }
            let key = format!("icons/{}.png", icon.to_lowercase());
            if !self.cache.contains_key(&key) && loads < 2 {
                let texture = match load_texture(&key).await {
                    Ok(texture) => {
                        texture.set_filter(FilterMode::Linear);
                        Some(texture)
                    }
                    Err(error) => {
                        tracing::warn!("Cannot load icon {key}: {error}");
                        None
                    }
                };
                self.cache.insert(key.clone(), (texture, self.tick));
                loads += 1;
            }
            draw_rectangle(
                r.x,
                clip_top,
                r.w,
                clip_bottom - clip_top,
                if index == selected {
                    ACCENT
                } else if hit {
                    Color::from_rgba(84, 86, 101, 255)
                } else {
                    Color::from_rgba(44, 46, 58, 255)
                },
            );
            if let Some((texture, tick)) = self.cache.get_mut(&key) {
                *tick = self.tick;
                if let Some(texture) = texture {
                    let inset = 3.0;
                    let image = Rect::new(
                        r.x + inset,
                        r.y + inset,
                        r.w - inset * 2.0,
                        r.h - inset * 2.0,
                    );
                    let y1 = image.y.max(grid.y);
                    let y2 = (image.y + image.h).min(grid.y + grid.h);
                    if y2 > y1 {
                        draw_texture_ex(
                            texture,
                            image.x,
                            y1,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(vec2(image.w, y2 - y1)),
                                source: Some(Rect::new(
                                    0.0,
                                    (y1 - image.y) / image.h * texture.height(),
                                    texture.width(),
                                    (y2 - y1) / image.h * texture.height(),
                                )),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
        while self.cache.len() > CACHE_LIMIT {
            let oldest = self
                .cache
                .iter()
                .min_by_key(|(_, (_, tick))| *tick)
                .map(|(key, _)| key.clone())
                .unwrap();
            self.cache.remove(&oldest);
        }
        let name = hovered_name.unwrap_or_else(|| {
            if self.tab == 0 {
                character.name().to_owned()
            } else {
                wardrobe.item_name(self.tab - 1).to_owned()
            }
        });
        let mut name = name;
        while measure_text(&name, Some(font), 12, 1.0).width > width - 24.0 {
            name.pop();
        }
        text(&name, x + 12.0, bottom - 10.0, font, WHITE, 12);
        if let Some(index) = chosen {
            super::audio::click();
            character.choose(
                if self.tab == 0 {
                    None
                } else {
                    Some(self.tab - 1)
                },
                index,
            );
        }
    }
}
