//! Actual booth choices. The Preview catalog only configures their appearance.
use super::widgets::paint::{draw_rectangle, draw_texture_ex};
use super::{booth::label, drag_scroll::DragScroll, icons::IconCache};
use crate::{booth::Booth, equipment::Entry};
use macroquad::prelude::*;

#[derive(Clone, Copy)]
pub enum Kind {
    Weapon,
    Emote,
}

#[derive(Default)]
pub struct EquipmentPicker {
    open: Option<(Kind, String)>,
    closing: bool,
    transition: super::widgets::Transition,
    just_opened: bool,
    scroll: f32,
    drag: DragScroll,
    icons: IconCache,
}
impl EquipmentPicker {
    pub fn open(&mut self, kind: Kind, character_id: &str) {
        self.open = Some((kind, character_id.to_owned()));
        self.closing = false;
        self.just_opened = true;
        self.scroll = 0.0;
        self.drag.cancel();
    }
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }
    pub fn bar(_canvas: Rect) -> Rect {
        // Dedicated controls row below the main toolbar, outside the stage.
        Rect::new(12.0, 56.0, 96.0, 38.0)
    }

    pub fn toolbar(&mut self, booth: &Booth, canvas: Rect, font: &Font, mouse: Vec2) {
        self.icons.begin_frame();
        let Some(c) = booth.selected() else {
            return;
        };
        let wardrobe = c.wardrobe();
        let catalog = &wardrobe.equipment_catalog;
        let equipment = &wardrobe.equipment;
        let weapon = catalog
            .weapon(equipment)
            .map(|w| catalog.weapon_icon(equipment, w))
            .unwrap_or_else(|| "UI/InventoryIcons/inventory-none".into());
        let emote = catalog
            .emote(equipment)
            .map(|e| e.icon.as_str())
            .unwrap_or("");
        let bar = Self::bar(canvas);
        for (i, (kind, icon, title)) in [
            (Kind::Weapon, weapon.as_str(), "W"),
            (Kind::Emote, emote, "E"),
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(bar.x + i as f32 * 49.0, bar.y, 46.0, bar.h);
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.078, 0.082, 0.11, 0.97));
            if let Some(texture) = self.icons.get(icon, true) {
                draw_texture_ex(
                    texture,
                    r.x + 7.0,
                    r.y + 3.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(32.0, 32.0)),
                        ..Default::default()
                    },
                );
            } else {
                label(title, r.x + 14.0, r.y + 25.0, 12, font, WHITE, 20.0);
            }
            if r.contains(mouse) && is_mouse_button_pressed(MouseButton::Left) {
                self.open(kind, &c.id);
                super::audio::click();
            }
        }
    }

    pub fn draw(
        &mut self,
        booth: &mut Booth,
        font: &Font,
        bold: &Font,
        mouse: Vec2,
    ) -> Option<Result<(), String>> {
        let (kind, id) = self.open.clone()?;
        self.transition.update(!self.closing);
        if self.closing && !self.transition.visible() {
            self.open = None;
            return None;
        }
        let mouse = if std::mem::take(&mut self.just_opened) || self.closing {
            vec2(-10000.0, -10000.0)
        } else {
            self.transition.mouse(true, mouse)
        };
        let Some(c) = booth.characters.iter_mut().find(|c| c.id == id) else {
            self.open = None;
            return None;
        };
        let wardrobe = c.wardrobe();
        let equipment = &wardrobe.equipment;
        let catalog = &wardrobe.equipment_catalog;
        let entries = match kind {
            Kind::Weapon => catalog.booth_weapons(equipment),
            Kind::Emote => catalog
                .emotes
                .iter()
                .map(|e| Entry {
                    id: e.id.clone(),
                    icon: e.icon.clone(),
                    weapon: None,
                    skin: None,
                })
                .collect(),
        };
        let size = super::layout::size();
        let panel = self.transition.panel(
            !self.closing,
            Rect::new(
                (size.x - (size.x - 32.0).min(440.0)) * 0.5,
                28.0,
                (size.x - 32.0).min(440.0),
                (size.y - 56.0).max(1.0),
            ),
        );
        self.transition.backdrop();
        let _opacity = self.transition.scope();
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, super::widgets::SURFACE);
        let close = super::widgets::popup_header(
            panel,
            match kind {
                Kind::Weapon => "Booth weapon",
                Kind::Emote => "Booth emote",
            },
            font,
            bold,
        );
        let mouse = if super::widgets::dismiss(panel, close, mouse) {
            self.closing = true;
            self.drag.cancel();
            super::widgets::BLOCKED
        } else {
            mouse
        };
        let configured = Rect::new(panel.x + 12.0, panel.y + 42.0, panel.w - 24.0, 34.0);
        let configured_id = catalog.configured_emote(equipment).to_owned();
        let configured_name = wardrobe.display_name(&configured_id);
        label(
            &match kind {
                Kind::Weapon => "Configured skins · tap to hold".into(),
                Kind::Emote => format!("Use configured: {configured_name}"),
            },
            configured.x + 6.0,
            configured.y + 23.0,
            12,
            font,
            ORANGE,
            configured.w - 12.0,
        );
        let use_configured = matches!(kind, Kind::Emote)
            && configured.contains(mouse)
            && is_mouse_button_pressed(MouseButton::Left);
        if use_configured {
            self.closing = true;
        }
        let mouse = if self.closing {
            super::widgets::BLOCKED
        } else {
            mouse
        };
        let area = Rect::new(
            panel.x + 12.0,
            panel.y + 84.0,
            panel.w - 24.0,
            (panel.h - 96.0).max(1.0),
        );
        let columns = ((area.w + 8.0) / 82.0).floor().max(2.0) as usize;
        let cell = (area.w + 8.0) / columns as f32;
        let max = (entries.len().div_ceil(columns) as f32 * cell - area.h).max(0.0);
        let tap = super::widgets::scroll(area, mouse, &mut self.scroll, max, &mut self.drag);
        super::layout::clip(area);
        let mut chosen = None;
        for (i, entry) in entries.iter().enumerate() {
            let r = Rect::new(
                area.x + (i % columns) as f32 * cell,
                area.y + (i / columns) as f32 * cell - self.scroll,
                cell - 8.0,
                cell - 8.0,
            );
            if r.bottom() <= area.y || r.y >= area.bottom() {
                continue;
            }
            let selected = match kind {
                Kind::Weapon => equipment.weapon.as_deref().unwrap_or("None") == entry.id,
                Kind::Emote => catalog.emote(equipment).is_some_and(|e| e.id == entry.id),
            };
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if selected {
                    Color::from_rgba(110, 70, 45, 255)
                } else {
                    Color::from_rgba(44, 46, 58, 255)
                },
            );
            if let Some(texture) = self.icons.get(&entry.icon, matches!(kind, Kind::Weapon)) {
                let edge = r.w - 20.0;
                draw_texture_ex(
                    texture,
                    r.x + 10.0,
                    r.y + 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(edge, edge)),
                        ..Default::default()
                    },
                );
            }
            let name_id = if let (Some(w), Some(s)) = (entry.weapon, entry.skin) {
                format!(
                    "{}_{}",
                    catalog.weapons[w].id, catalog.weapons[w].skin_names[s]
                )
            } else {
                entry.id.clone()
            };
            label(
                wardrobe.display_name(&name_id),
                r.x + 3.0,
                r.bottom() - 4.0,
                10,
                font,
                WHITE,
                r.w - 6.0,
            );
            if tap && area.contains(mouse) && r.contains(mouse) {
                chosen = Some(entry.id.clone());
            }
        }
        super::layout::end_clip();
        if let Some(id) = chosen {
            let result = match kind {
                Kind::Weapon => c.use_weapon((id != "None").then_some(id.as_str())),
                Kind::Emote => c.use_emote(&id),
            };
            self.closing = true;
            self.drag.cancel();
            super::audio::click();
            return Some(result);
        }
        use_configured.then(|| c.use_emote(&configured_id))
    }
}
