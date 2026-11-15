//! Virtualized mouse-driven catalog. Only visible cells request textures.
use super::widgets::{
    self,
    paint::{draw_rectangle, draw_text_ex, draw_texture_ex},
};
use crate::character::Character;
use macroquad::prelude::*;
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

#[derive(Default)]
pub struct Selection {
    pub pointer_blocked: bool,
    tab: usize,
    scroll: Vec<f32>,
    category_scroll: f32,
    category_drag: crate::drag_scroll::DragScroll,
    cache: super::icons::IconCache,
    category_cache: super::icons::IconCache,
    sparkle_cache: super::icons::IconCache,
    preview_tab: Option<usize>,
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
    fn draw_categories(&mut self, character: &Character, rail: Rect, mouse: Vec2, font: &Font) {
        use crate::equipment::CategoryKind;
        let wardrobe = character.wardrobe();
        let mut icons: Vec<(String, bool, String)> = [
            ("Animal", "Characters"),
            ("Neck", "Neck"),
            ("Body", "Clothes"),
            ("Eyes", "Glasses"),
            ("Head", "Hats"),
            ("Face", "Beards"),
        ]
        .into_iter()
        .map(|(icon, title)| {
            (
                format!("ui/customizeicons/icon_{}", icon.to_lowercase()),
                false,
                title.into(),
            )
        })
        .collect();
        for category in &wardrobe.equipment_catalog.categories {
            let (icon, white) = match category.kind {
                CategoryKind::Weapon => (
                    category
                        .entries
                        .iter()
                        .find(|entry| !entry.icon.is_empty())
                        .map(|entry| entry.icon.clone())
                        .unwrap_or_default(),
                    true,
                ),
                CategoryKind::Umbrella => ("ui/customizeicons/icon_umbrella".into(), false),
                CategoryKind::Pet => ("ui/customizeicons/icon_pets".into(), false),
                CategoryKind::Emote => ("ui/customizeicons/icon_emote".into(), false),
            };
            icons.push((icon, white, category.label.clone()));
        }
        self.category_cache.begin_frame();
        let mut order: Vec<usize> = (0..icons.len()).collect();
        order.sort_by_key(|&index| icons[index].1);
        let step = 52.0;
        let max = (icons.len() as f32 * step - 6.0 - rail.h).max(0.0);
        let tapped = widgets::scroll(
            rail,
            mouse,
            &mut self.category_scroll,
            max,
            &mut self.category_drag,
        );
        let mut hovered = None;
        super::layout::clip(rail);
        for (position, &i) in order.iter().enumerate() {
            let (icon, white, title) = &icons[i];
            let r = Rect::new(
                rail.x,
                rail.y + position as f32 * step - self.category_scroll,
                rail.w,
                46.0,
            );
            if r.bottom() <= rail.y || r.y >= rail.bottom() {
                continue;
            }
            let hit = rail.contains(mouse) && r.contains(mouse);
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if self.tab == i {
                    Color::from_rgba(53, 44, 41, 255)
                } else {
                    widgets::PREVIEW_SURFACE
                },
            );
            if self.tab == i {
                draw_rectangle(r.x, r.y, 2.0, r.h, ACCENT);
            }
            if let Some(texture) = self.category_cache.get(icon, *white) {
                let scale = if *white {
                    (r.w - 12.0).min(r.h - 10.0) / texture.width().max(texture.height()).max(1.0)
                } else {
                    0.65
                };
                let dest = vec2(texture.width(), texture.height()) * scale;
                draw_texture_ex(
                    texture,
                    r.x + (r.w - dest.x) * 0.5,
                    r.y + (r.h - dest.y) * 0.5,
                    if self.tab == i {
                        ACCENT
                    } else {
                        widgets::paint::icon_color(WHITE)
                    },
                    DrawTextureParams {
                        dest_size: Some(dest),
                        ..Default::default()
                    },
                );
            }
            if hit {
                hovered = Some(title.as_str());
                if tapped && self.tab != i {
                    self.tab = i;
                    self.content_drag.cancel();
                    super::audio::click();
                }
            }
        }
        super::layout::end_clip();
        if let Some(title) = hovered {
            let width = measure_text(title, Some(font), 11, 1.0).width + 16.0;
            let x = (rail.x - width - 6.0).max(0.0);
            let y = mouse.y.min(rail.bottom() - 24.0);
            draw_rectangle(x, y, width, 24.0, widgets::PREVIEW_SURFACE);
            text(title, x + 8.0, y + 16.0, font, WHITE, 11);
        }
    }
    pub fn begin_preview(&mut self) {
        // Opening always starts Idle, even when an equipment category was retained.
        self.preview_tab = Some(self.tab);
        self.category_drag.cancel();
    }
    pub fn captures_keyboard(&self) -> bool {
        self.capture_keyboard || self.native_input.active()
    }
    pub fn release_search(&mut self) {
        self.search_focused = false;
        self.capture_keyboard = false;
    }
    pub async fn draw(&mut self, character: &mut Character, font: &Font, bold: &Font) {
        let modal_was_open = self.native_input.active();
        let categories = 6 + character.wardrobe().equipment_catalog.categories.len();
        self.scroll.resize(categories, 0.0);
        self.tab = self.tab.min(categories - 1);
        self.cache.begin_frame();
        self.sparkle_cache.begin_frame();
        if let Some(query) = self.native_input.poll() {
            self.query = query.chars().filter(|c| !c.is_control()).take(80).collect();
            self.scroll.fill(0.0);
            self.content_drag.cancel();
        }
        let size = crate::gui_scale::size();
        let w = size.x;
        let compact = crate::gui_scale::compact();
        let actual_mouse = if self.pointer_blocked || self.native_input.active() {
            vec2(-10000.0, -10000.0)
        } else {
            crate::gui_scale::mouse()
        };
        let mouse = actual_mouse;
        let click = is_mouse_button_pressed(MouseButton::Left);
        let (_, panel) = crate::gui_scale::regions(size);
        let stacked = panel.x < 32.0;
        let rail = Rect::new(
            if stacked { panel.x } else { panel.x - 56.0 },
            panel.y + panel.h * 0.04,
            48.0,
            panel.h * 0.92,
        );
        let inset = if stacked { 60.0 } else { 0.0 };
        let (x, top, width, bottom) =
            (panel.x + inset, panel.y, panel.w - inset, panel.y + panel.h);
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
        let search_offset = 12.0;
        let summary_offset = 46.0;
        let grid_offset = 72.0;
        let panel = Rect::new(x, top, width, bottom - top);
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, widgets::PREVIEW_SURFACE);
        self.draw_categories(character, rail, mouse, font);
        let search = Rect::new(x + 12.0, top + search_offset, width - 24.0, 28.0);
        let reset = Rect::new(search.x + search.w - 28.0, search.y, 28.0, 28.0);
        if click && !self.pointer_blocked {
            if search.contains(mouse) && !reset.contains(mouse) {
                super::audio::click();
                self.native_input.open(&self.query);
            }
            self.search_focused = false;
        }
        self.search_focused = self.native_input.active();
        self.capture_keyboard = self.search_focused || modal_was_open;
        let previous = self.query.clone();
        if click && reset.contains(mouse) {
            self.query.clear();
        }
        // The native widget owns editing, composition, clipboard and keyboard handling.
        while get_char_pressed().is_some() {}
        if previous != self.query {
            self.content_drag.cancel();
            self.scroll.fill(0.0);
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
        let clear = Rect::new(x + width - 72.0, top + summary_offset, 60.0, 22.0);
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
            self.scroll.fill(0.0);
            self.last_language.clone_from(&wardrobe.language);
        }
        let total = if self.tab == 0 {
            wardrobe.characters.len()
        } else if self.tab < 6 {
            wardrobe.items[self.tab - 1].len()
        } else {
            wardrobe.equipment_catalog.categories[self.tab - 6]
                .entries
                .len()
        };
        let japanese = wardrobe.language == "japanese";
        let query = normalize(&self.query, japanese);
        let filtered: Vec<usize> = (0..total)
            .filter(|&index| {
                let (_, id) = entry(wardrobe, self.tab, index);
                let name = if self.tab > 0 && self.tab < 6 && index == 0 {
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
        } else if self.tab < 6 {
            wardrobe.selected[self.tab - 1]
        } else {
            wardrobe
                .equipment_catalog
                .selected(&wardrobe.equipment, self.tab - 6)
        };
        text(
            &format!("{} / {} items", count, total),
            x + 12.0,
            top + summary_offset + 15.0,
            font,
            GRAY,
            11,
        );
        let grid = Rect::new(
            x + 12.0,
            top + grid_offset,
            width - 34.0,
            (bottom - top - grid_offset - 32.0).max(1.0),
        );
        let columns = ((grid.w + 8.0) / 82.0).floor().max(2.0) as usize;
        let cell = (grid.w + 8.0) / columns as f32;
        let rows = count.div_ceil(columns);
        let max_scroll = (rows as f32 * cell - grid.h).max(0.0);
        let tapped = widgets::scroll(
            grid,
            mouse,
            &mut self.scroll[self.tab],
            max_scroll,
            &mut self.content_drag,
        );
        if mouse.x <= -9999.0 {
            self.dragging = false;
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
        if count == 0 {
            text("No matches", grid.x, grid.y + 24.0, font, GRAY, 13);
        }
        super::layout::clip(grid);
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
            let (icon, id) = entry(wardrobe, self.tab, index);
            if hit {
                hovered_name = Some(wardrobe.display_name(id).to_owned());
                if tapped {
                    chosen = Some(index);
                }
            }
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if index == selected {
                    ACCENT
                } else if hit {
                    Color::from_rgba(84, 86, 101, 255)
                } else {
                    Color::from_rgba(44, 46, 58, 255)
                },
            );
            if let Some(texture) = self.cache.get(icon, false) {
                let inset = 3.0;
                let image = Rect::new(
                    r.x + inset,
                    r.y + inset,
                    r.w - inset * 2.0,
                    r.h - inset * 2.0,
                );
                draw_texture_ex(
                    texture,
                    image.x,
                    image.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(image.w, image.h)),
                        ..Default::default()
                    },
                );
            }
            let sparkle = if self.tab == 0 {
                false
            } else if self.tab < 6 {
                wardrobe.items[self.tab - 1][index].use_cosmetics_anim_track
            } else {
                wardrobe.equipment_catalog.has_sparkle(self.tab - 6, index)
            };
            if sparkle && let Some(texture) = self.sparkle_cache.get("ui/shopsparkle", false) {
                let side = (r.w * 0.23).clamp(12.0, 20.0);
                draw_texture_ex(
                    texture,
                    r.x + 4.0,
                    r.y + 4.0,
                    widgets::paint::icon_color(WHITE),
                    DrawTextureParams {
                        dest_size: Some(vec2(side, side)),
                        ..Default::default()
                    },
                );
            }
        }
        super::layout::end_clip();
        let name = hovered_name.unwrap_or_else(|| {
            if self.tab == 0 {
                character.name().to_owned()
            } else if self.tab < 6 {
                wardrobe.item_name(self.tab - 1).to_owned()
            } else {
                wardrobe
                    .display_name(entry(wardrobe, self.tab, selected).1)
                    .to_owned()
            }
        });
        let mut name = name;
        while measure_text(&name, Some(font), 12, 1.0).width > width - 24.0 {
            name.pop();
        }
        text(&name, x + 12.0, bottom - 10.0, font, WHITE, 12);
        if let Some(index) = chosen {
            super::audio::click();
            if self.tab >= 6 {
                if let Err(error) = character.choose_equipment(self.tab - 6, index) {
                    super::notifications::error(format!("Cannot equip: {error}"));
                }
                character.preview_category(self.tab - 6);
            } else {
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
        if self.preview_tab != Some(self.tab) {
            if self.tab >= 6 {
                character.preview_category(self.tab - 6);
            }
            self.preview_tab = Some(self.tab);
        }
    }
}

fn entry(wardrobe: &crate::wardrobe::Wardrobe, tab: usize, index: usize) -> (&str, &str) {
    if tab == 0 {
        let c = &wardrobe.characters[index];
        (&c.icon, &c.inventory_id)
    } else if tab < 6 {
        let c = &wardrobe.items[tab - 1][index];
        (&c.icon, &c.inventory_id)
    } else {
        let c = &wardrobe.equipment_catalog.categories[tab - 6].entries[index];
        (&c.icon, &c.id)
    }
}
