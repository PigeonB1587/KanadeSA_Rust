//! Compact project shelf with named creation and explicit save-before-switch.
use super::{booth::control_label, drag_scroll::DragScroll, input::NativeInput, layout, widgets};
use crate::{booth::Booth, project::Project};
use macroquad::prelude::*;
use std::collections::HashMap;
use widgets::paint::{draw_line, draw_rectangle, draw_texture_ex};

const ACCENT: Color = Color::new(0.925, 0.604, 0.361, 1.0);
const CONTROL: Color = Color::new(0.12, 0.125, 0.16, 1.0);
const EDGE: Color = Color::new(0.23, 0.235, 0.27, 1.0);

#[derive(Clone)]
pub enum Action {
    Create(String),
    Open(String),
    Rename(String),
    Import(String),
    Export(String),
}

enum NameInput {
    Create,
    Rename,
    Import,
    Export,
}

#[derive(Default)]
pub struct Projects {
    open: bool,
    transition: widgets::Transition,
    entries: Vec<Project>,
    thumbnails: HashMap<String, Option<Texture2D>>,
    scroll: f32,
    drag: DragScroll,
    input: NativeInput,
    naming: Option<NameInput>,
    pending: Option<Action>,
}

fn button(title: &str, rect: Rect, font: &Font, mouse: Vec2, enabled: bool, accent: bool) -> bool {
    button_with_trigger(
        title,
        rect,
        font,
        mouse,
        enabled,
        accent,
        is_mouse_button_pressed(MouseButton::Left),
    )
}

fn button_with_trigger(
    title: &str,
    rect: Rect,
    font: &Font,
    mouse: Vec2,
    enabled: bool,
    accent: bool,
    trigger: bool,
) -> bool {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, CONTROL);
    if accent {
        draw_rectangle(rect.x, rect.bottom() - 2.0, rect.w, 2.0, ACCENT);
    }
    let size = [12, 11, 10]
        .into_iter()
        .find(|size| measure_text(title, Some(font), *size, 1.0).width <= rect.w - 8.0)
        .unwrap_or(10);
    control_label(
        title,
        rect,
        size,
        font,
        if !enabled {
            GRAY
        } else if accent {
            ACCENT
        } else {
            WHITE
        },
        true,
    );
    let clicked = enabled && rect.contains(mouse) && trigger;
    if clicked {
        super::audio::click();
    }
    clicked
}

impl Projects {
    pub fn visible(&self) -> bool {
        self.open || self.transition.visible() || self.input.active()
    }
    pub fn open(&mut self) {
        self.open = true;
        self.pending = None;
        self.scroll = 0.0;
        self.drag.cancel();
        self.refresh();
    }
    pub fn refresh(&mut self) {
        match crate::project::list() {
            Ok(entries) => self.entries = entries,
            Err(error) => super::notifications::error(format!("Cannot list projects: {error}")),
        }
        self.thumbnails.clear();
    }
    pub fn finished(&mut self, switched: bool) {
        self.pending = None;
        self.scroll = 0.0;
        if switched {
            self.open = false;
        }
        self.refresh();
    }
    fn request(&mut self, action: Action, booth: &Booth) -> Option<Action> {
        if booth.project.is_some() {
            self.pending = Some(action);
            self.scroll = 0.0;
            self.drag.cancel();
            None
        } else {
            Some(action)
        }
    }
    pub fn draw(
        &mut self,
        booth: &Booth,
        font: &Font,
        bold: &Font,
        name_font: &Font,
        mouse: Vec2,
        busy: bool,
    ) -> Option<(Action, bool)> {
        let mut requested = None;
        let was_input_active = self.input.active();
        if let Some(text) = self.input.poll() {
            match self.naming.take() {
                Some(NameInput::Import) => {
                    if !text.trim().is_empty() {
                        requested = self.request(Action::Import(text.trim().into()), booth);
                    }
                }
                Some(NameInput::Export) => {
                    if !text.trim().is_empty() {
                        requested = Some(Action::Export(text.trim().into()));
                    }
                }
                kind => match crate::project::clean_name(&text) {
                    Ok(name) => match kind {
                        Some(NameInput::Create) => {
                            requested = self.request(Action::Create(name), booth)
                        }
                        Some(NameInput::Rename) => requested = Some(Action::Rename(name)),
                        _ => {}
                    },
                    Err(error) => super::notifications::error(error),
                },
            }
        }
        if !self.input.active() {
            self.naming = None;
        }
        if !self.visible() {
            return requested.map(|a| (a, false));
        }
        let size = layout::size();
        let w = (size.x - 24.0).clamp(1.0, 760.0);
        let h = (size.y - 40.0).clamp(1.0, 620.0);
        let panel = self.transition.panel(
            self.open,
            Rect::new((size.x - w) * 0.5, (size.y - h) * 0.5, w, h),
        );
        self.transition.backdrop();
        let mut mouse = self.transition.mouse(
            self.open && !was_input_active && !self.input.active() && requested.is_none(),
            mouse,
        );
        let _opacity = self.transition.scope();
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, widgets::SURFACE);
        let close = if booth.project.is_some() {
            widgets::popup_header(panel, "Projects", font, bold)
        } else {
            control_label(
                "Projects",
                Rect::new(
                    panel.x + 6.0,
                    panel.y + 4.0,
                    (panel.w - 12.0).max(1.0),
                    40.0,
                ),
                18,
                bold,
                WHITE,
                false,
            );
            Rect::new(-10000.0, -10000.0, 0.0, 0.0)
        };
        if booth.project.is_some() && widgets::dismiss(panel, close, mouse) {
            if self.pending.is_some() {
                self.pending = None;
                self.scroll = 0.0;
            } else {
                self.open = false;
            }
            self.drag.cancel();
            mouse = widgets::BLOCKED;
        }
        if self.pending.is_some() {
            let area = Rect::new(
                panel.x + 12.0,
                panel.y + 50.0,
                (panel.w - 24.0).max(1.0),
                (panel.h - 62.0).max(1.0),
            );
            let stacked = area.w < 430.0;
            let button_height = if stacked { 116.0 } else { 36.0 };
            let content = button_height + 96.0;
            let max = (content - area.h).max(0.0);
            let tap = widgets::scroll(area, mouse, &mut self.scroll, max, &mut self.drag);
            layout::clip(area);
            let body = Rect::new(area.x, area.y - self.scroll, area.w, 28.0);
            control_label("Save before leaving?", body, 15, bold, WHITE, false);
            if let Some(project) = &booth.project {
                control_label(
                    &project.name,
                    Rect::new(body.x, body.y + 28.0, body.w, 28.0),
                    13,
                    name_font,
                    ACCENT,
                    false,
                );
            }
            control_label(
                "Exported files will be kept.",
                Rect::new(body.x, body.y + 56.0, body.w, 24.0),
                11,
                font,
                GRAY,
                false,
            );
            let width = if stacked {
                body.w
            } else {
                ((body.w - 16.0) / 3.0).max(1.0)
            };
            let mut choice = None;
            for (i, title) in ["Save & continue", "Discard edits", "Cancel"]
                .iter()
                .enumerate()
            {
                let rect = Rect::new(
                    body.x
                        + if stacked {
                            0.0
                        } else {
                            i as f32 * (width + 8.0)
                        },
                    body.y + 90.0 + if stacked { i as f32 * 40.0 } else { 0.0 },
                    width,
                    36.0,
                );
                if button_with_trigger(
                    title,
                    rect,
                    bold,
                    if area.contains(mouse) {
                        mouse
                    } else {
                        widgets::BLOCKED
                    },
                    !busy || i == 2,
                    i == 0,
                    tap,
                ) {
                    choice = Some(i);
                }
            }
            layout::end_clip();
            if max > 0.0 {
                let height = (area.h * area.h / content).max(8.0).min(area.h);
                draw_rectangle(
                    area.right() - 2.0,
                    area.y + (area.h - height) * self.scroll / max,
                    2.0,
                    height,
                    ACCENT,
                );
            }
            if let Some(i) = choice {
                if i == 2 {
                    self.pending = None;
                    self.scroll = 0.0;
                    return None;
                }
                return self.pending.clone().map(|action| (action, i == 0));
            }
            return None;
        }
        let stacked = panel.w < 600.0;
        let columns = if stacked { 2 } else { 4 };
        let new = Rect::new(
            panel.x + 12.0,
            panel.y + 50.0,
            ((panel.w - 24.0 - (columns - 1) as f32 * 8.0) / columns as f32).max(1.0),
            34.0,
        );
        let action_rect = |index: usize| {
            Rect::new(
                new.x + (index % columns) as f32 * (new.w + 8.0),
                new.y + (index / columns) as f32 * (new.h + 8.0),
                new.w,
                new.h,
            )
        };
        if button("+ New project", new, bold, mouse, !busy, true) {
            self.naming = Some(NameInput::Create);
            self.input
                .open_with("New project", "Name your project (up to 80 characters)", "");
        }
        if button(
            "Rename current",
            action_rect(1),
            font,
            mouse,
            !busy && booth.project.is_some(),
            false,
        ) {
            self.naming = Some(NameInput::Rename);
            self.input.open_with(
                "Rename project",
                "Project name",
                booth
                    .project
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or(""),
            );
        }
        if button("Import project", action_rect(2), font, mouse, !busy, false) {
            self.naming = Some(NameInput::Import);
            self.input.open_with(
                "Import project",
                "Path to a .kanade-project.json file; imports as a new independent project",
                "",
            );
        }
        if button(
            "Export current",
            action_rect(3),
            font,
            mouse,
            !busy && booth.project.is_some(),
            false,
        ) {
            self.naming = Some(NameInput::Export);
            let default = booth
                .project
                .as_ref()
                .map(|project| project.default_export_path().to_string_lossy().into_owned())
                .unwrap_or_default();
            self.input.open_with(
                "Export project",
                "Save current edits and export to this file (existing files are never overwritten)",
                &default,
            );
        }
        let current_y = new.y + if stacked { 80.0 } else { 38.0 };
        if let Some(project) = &booth.project {
            control_label(
                &format!("Current: {}", project.name),
                Rect::new(panel.x + 12.0, current_y, (panel.w - 24.0).max(1.0), 28.0),
                12,
                name_font,
                ACCENT,
                false,
            );
        }
        let area = Rect::new(
            panel.x + 12.0,
            current_y + 34.0,
            (panel.w - 24.0).max(1.0),
            (panel.bottom() - 38.0 - (current_y + 34.0)).max(1.0),
        );
        let cols = (((area.w + 10.0) / 210.0).floor() as usize).clamp(1, 3);
        let cw = ((area.w - 8.0 - (cols - 1) as f32 * 10.0) / cols as f32).max(1.0);
        let ch = cw * 9.0 / 16.0 + 58.0;
        let rows = self.entries.len().div_ceil(cols);
        let content = (rows as f32 * (ch + 10.0) - 10.0).max(0.0);
        let max = (content - area.h).max(0.0);
        let tap = widgets::scroll(area, mouse, &mut self.scroll, max, &mut self.drag);
        let mut selected = None;
        let mut loads = 0;
        layout::clip(area);
        if self.entries.is_empty() {
            control_label(
                "Create a project to start your next scene.",
                Rect::new(area.x, area.y + 20.0, area.w, 48.0),
                13,
                font,
                GRAY,
                true,
            );
        }
        for (index, project) in self.entries.iter().enumerate() {
            let rect = Rect::new(
                area.x + (index % cols) as f32 * (cw + 10.0),
                area.y + (index / cols) as f32 * (ch + 10.0) - self.scroll,
                cw,
                ch,
            );
            if rect.bottom() <= area.y || rect.y >= area.bottom() {
                continue;
            }
            let current = booth.project.as_ref().is_some_and(|p| p.id == project.id);
            let hovered = rect.contains(mouse) && area.contains(mouse);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                if current {
                    ACCENT
                } else if hovered {
                    GRAY
                } else {
                    EDGE
                },
            );
            draw_rectangle(
                rect.x + 1.0,
                rect.y + 1.0,
                rect.w - 2.0,
                rect.h - 2.0,
                CONTROL,
            );
            let image = Rect::new(rect.x + 1.0, rect.y + 1.0, rect.w - 2.0, cw * 9.0 / 16.0);
            if !self.thumbnails.contains_key(&project.id) && loads < 2 {
                let texture = std::fs::read(project.directory().join("thumbnail.png"))
                    .ok()
                    .and_then(|bytes| {
                        Image::from_file_with_format(&bytes, Some(ImageFormat::Png)).ok()
                    })
                    .map(|image| {
                        let texture = Texture2D::from_image(&image);
                        texture.set_filter(FilterMode::Linear);
                        texture
                    });
                self.thumbnails.insert(project.id.clone(), texture);
                loads += 1;
            }
            if let Some(Some(texture)) = self.thumbnails.get(&project.id) {
                let s = (image.w / texture.width()).min(image.h / texture.height());
                let dest = vec2(texture.width(), texture.height()) * s;
                draw_texture_ex(
                    texture,
                    image.x + (image.w - dest.x) * 0.5,
                    image.y + (image.h - dest.y) * 0.5,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(dest),
                        ..Default::default()
                    },
                );
            } else {
                control_label("No overview yet", image, 11, font, GRAY, true);
            }
            control_label(
                &project.name,
                Rect::new(rect.x + 4.0, image.bottom() + 2.0, rect.w - 8.0, 28.0),
                13,
                name_font,
                if current { ACCENT } else { WHITE },
                false,
            );
            let detail = if current {
                format!(
                    "{} roles / {} / CURRENT",
                    project.character_count, project.canvas_ratio
                )
            } else {
                format!(
                    "{} roles / {} / {}",
                    project.character_count,
                    project.canvas_ratio,
                    project.updated_at.get(..10).unwrap_or("")
                )
            };
            control_label(
                &detail,
                Rect::new(rect.x + 4.0, image.bottom() + 30.0, rect.w - 8.0, 22.0),
                10,
                font,
                GRAY,
                false,
            );
            if current {
                draw_rectangle(rect.x, rect.bottom() - 2.0, rect.w, 2.0, ACCENT);
            }
            if tap && hovered && !busy && !current {
                selected = Some(project.id.clone());
            }
        }
        if max > 0.0 {
            let height = (area.h * area.h / content)
                .clamp(12.0, area.h.max(12.0))
                .min(area.h);
            draw_rectangle(
                area.right() - 3.0,
                area.y + (area.h - height) * self.scroll / max,
                2.0,
                height,
                ACCENT,
            );
        }
        layout::end_clip();
        draw_line(
            panel.x + 12.0,
            panel.bottom() - 30.0,
            panel.right() - 12.0,
            panel.bottom() - 30.0,
            1.0,
            EDGE,
        );
        control_label(
            if busy {
                "Recording in progress / switching disabled"
            } else {
                "Project files contain edits / photos & GIFs stay in project folders"
            },
            Rect::new(panel.x + 12.0, panel.bottom() - 28.0, panel.w - 24.0, 24.0),
            10,
            font,
            GRAY,
            false,
        );
        if let Some(id) = selected {
            requested = self.request(Action::Open(id), booth);
        }
        requested.map(|action| (action, false))
    }
}
