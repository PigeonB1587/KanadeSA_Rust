//! Touch-sized booth controls, a scrollable inspector and the retained Preview layer.
use super::drag_scroll::DragScroll;
use super::layout::{BoothLayout, CONTROL_HEIGHT, GAP, MARGIN, TOPBAR_HEIGHT};
use super::notifications;
use super::widgets::paint::{draw_line, draw_rectangle, draw_text_ex, draw_texture_ex};
use crate::{
    booth::{Booth, canvas_to_world, perspective, world_to_canvas},
    model::{ReferenceFrame, Watermark},
};
use macroquad::prelude::*;

const ACCENT: Color = Color::new(0.925, 0.604, 0.361, 1.0);
const SURFACE: Color = Color::new(0.078, 0.082, 0.11, 0.97);
const CONTROL: Color = Color::new(0.12, 0.125, 0.16, 1.0);
const PANEL_PADDING: f32 = super::layout::INSPECTOR_PADDING;
const INSPECTOR_CONTROL_HEIGHT: f32 = 36.0;
const ROW_STEP: f32 = INSPECTOR_CONTROL_HEIGHT + PANEL_PADDING;
const RATIOS: [(&str, u32, u32); 8] = [
    ("16:9", 16, 9),
    ("16:10", 16, 10),
    ("4:3", 4, 3),
    ("3:2", 3, 2),
    ("1:1", 1, 1),
    ("4:5", 4, 5),
    ("3:4", 3, 4),
    ("9:16", 9, 16),
];

pub struct BoothUi {
    recorder: crate::recorder::Recorder,
    photo_requested: bool,
    photo_size: usize,
    gif_size: usize,
    gif_fps: usize,
    gif_quality: usize,
    pub preview: bool,
    ratio: usize,
    ratio_open: bool,
    ratio_transition: super::widgets::Transition,
    ratio_scroll: f32,
    ratio_drag: DragScroll,
    tab: usize,
    inspector: bool,
    scroll: f32,
    inspector_heights: [f32; 3],
    scroll_drag: DragScroll,
    actor_drag: Option<(usize, bool, Vec2)>,
    slider: Option<String>,
    slider_start: Option<(Vec2, f32)>,
    content_tap: Option<Vec2>,
    modal_this_frame: bool,
    color_picker: super::color_picker::ColorPicker,
    scene_picker: bool,
    scene_transition: super::widgets::Transition,
    preset_transition: super::widgets::Transition,
    scene_gallery: super::scenes::SceneGallery,
    projects: super::projects::Projects,
    project_font: Option<Font>,
    clip_picker: bool,
    clip_scroll: f32,
    clip_drag: DragScroll,
    equipment: super::equipment::EquipmentPicker,
}
impl Default for BoothUi {
    fn default() -> Self {
        Self {
            recorder: Default::default(),
            photo_requested: false,
            photo_size: 1,
            gif_size: 1,
            gif_fps: 1,
            gif_quality: 1,
            preview: false,
            ratio: 0,
            ratio_open: false,
            ratio_transition: super::widgets::Transition::default(),
            ratio_scroll: 0.0,
            ratio_drag: DragScroll::default(),
            tab: 0,
            inspector: true,
            scroll: 0.0,
            inspector_heights: [0.0; 3],
            scroll_drag: DragScroll::default(),
            actor_drag: None,
            slider: None,
            slider_start: None,
            content_tap: None,
            modal_this_frame: false,
            color_picker: super::color_picker::ColorPicker::default(),
            scene_picker: false,
            scene_transition: super::widgets::Transition::default(),
            preset_transition: super::widgets::Transition::default(),
            scene_gallery: super::scenes::SceneGallery::default(),
            projects: super::projects::Projects::default(),
            project_font: None,
            clip_picker: false,
            clip_scroll: 0.0,
            clip_drag: DragScroll::default(),
            equipment: super::equipment::EquipmentPicker::default(),
        }
    }
}

pub fn label(text: &str, x: f32, y: f32, size: u16, font: &Font, color: Color, width: f32) {
    fitted_label(text, x, y, size, font, color, width, false);
}

pub fn selected_label(text: &str, x: f32, y: f32, size: u16, font: &Font, width: f32) {
    fitted_label(text, x, y, size, font, WHITE, width, true);
}

fn fitted_label(
    text: &str,
    x: f32,
    y: f32,
    size: u16,
    font: &Font,
    color: Color,
    width: f32,
    fixed_color: bool,
) {
    let mut fitted = text.to_owned();
    while !fitted.is_empty() && measure_text(&fitted, Some(font), size, 1.0).width > width {
        fitted.pop();
    }
    let draw = if fixed_color {
        super::widgets::paint::draw_text_fixed_color
    } else {
        super::widgets::paint::draw_text_ex
    };
    draw(
        &fitted,
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

pub fn control_label(text: &str, r: Rect, size: u16, font: &Font, color: Color, centered: bool) {
    let mut fitted = text.to_owned();
    let available = (r.w - if centered { 8.0 } else { 16.0 }).max(0.0);
    while !fitted.is_empty() && measure_text(&fitted, Some(font), size, 1.0).width > available {
        fitted.pop();
    }
    if fitted.is_empty() {
        return;
    }
    let text = measure_text(&fitted, Some(font), size, 1.0);
    draw_text_ex(
        &fitted,
        if centered {
            r.x + (r.w - text.width) * 0.5
        } else {
            r.x + 8.0
        },
        r.y + (r.h - text.height) * 0.5 + text.offset_y,
        TextParams {
            font: Some(font),
            font_size: size,
            color,
            ..Default::default()
        },
    );
}

fn button_text_color(enabled: bool, active: bool) -> Color {
    if active {
        ACCENT
    } else if enabled {
        WHITE
    } else {
        GRAY
    }
}

fn centered_button(
    title: &str,
    r: Rect,
    size: u16,
    font: &Font,
    mouse: Vec2,
    enabled: bool,
    active: bool,
    fill: Color,
) -> bool {
    let clicked = button_with_fill("", r, font, mouse, enabled, active, fill);
    control_label(
        title,
        r,
        size,
        font,
        button_text_color(enabled, active),
        true,
    );
    clicked
}

fn button(title: &str, r: Rect, font: &Font, mouse: Vec2, enabled: bool, active: bool) -> bool {
    button_with_fill(title, r, font, mouse, enabled, active, CONTROL)
}

fn button_with_fill(
    title: &str,
    r: Rect,
    font: &Font,
    mouse: Vec2,
    enabled: bool,
    active: bool,
    fill: Color,
) -> bool {
    draw_rectangle(
        r.x,
        r.y,
        r.w,
        r.h,
        if active {
            Color::from_rgba(53, 44, 41, 255)
        } else {
            fill
        },
    );
    if active {
        draw_rectangle(r.x, r.y + r.h - 2.0, r.w, 2.0, ACCENT);
    }
    control_label(
        title,
        r,
        12,
        font,
        button_text_color(enabled, active),
        false,
    );
    // Inspector content commits taps on release so vertical scrolling never
    // changes a property. Toolbar and modal headers remain press buttons.
    let trigger = if CONTENT_BUTTONS.with(|value| value.get()) {
        is_mouse_button_released(MouseButton::Left) && CONTENT_TAP.with(|value| value.get())
    } else {
        is_mouse_button_pressed(MouseButton::Left)
    };
    let clicked = enabled && r.contains(mouse) && trigger;
    if clicked {
        super::audio::click();
    }
    clicked
}
thread_local! { static CONTENT_BUTTONS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
thread_local! { static CONTENT_TAP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

impl BoothUi {
    pub fn set_project_font(&mut self, font: Font) {
        self.project_font = Some(font);
    }
    pub fn select_project_ratio(&mut self, booth: &Booth) {
        if let Some(project) = &booth.project {
            self.ratio = RATIOS
                .iter()
                .position(|(name, _, _)| *name == project.canvas_ratio)
                .unwrap_or(0);
        }
        self.actor_drag = None;
        self.slider = None;
        self.slider_start = None;
        self.content_tap = None;
        self.scroll_drag.cancel();
        self.scroll = 0.0;
    }
    pub fn open_projects(&mut self) {
        self.projects.open();
    }
    fn save_project(&mut self, booth: &mut Booth) -> bool {
        if booth.project.is_none() {
            self.projects.open();
            return false;
        }
        let result = booth.save_project(RATIOS[self.ratio].0);
        super::layout::begin();
        let success = self
            .result(result, "Project saved · overview updated")
            .is_some();
        if success {
            self.projects.refresh();
        }
        success
    }
    fn project_action(
        &mut self,
        booth: &mut Booth,
        action: super::projects::Action,
        save_first: bool,
    ) {
        if save_first && !self.save_project(booth) {
            return;
        }
        let switched = matches!(
            &action,
            super::projects::Action::Create(_)
                | super::projects::Action::Open(_)
                | super::projects::Action::Import(_)
        );
        let result = match action {
            super::projects::Action::Create(name) => booth.create_project(&name),
            super::projects::Action::Open(id) => booth.open_project(&id),
            super::projects::Action::Rename(name) => booth.rename_project(&name),
            super::projects::Action::Import(path) => {
                booth.import_project(std::path::Path::new(&path))
            }
            super::projects::Action::Export(path) => {
                let result =
                    booth.export_project(std::path::Path::new(&path), RATIOS[self.ratio].0);
                super::layout::begin();
                match result {
                    Ok(path) => {
                        notifications::success(format!("Project exported: {}", path.display()));
                        self.projects.finished(false);
                    }
                    Err(error) => {
                        notifications::error(error);
                        self.projects.refresh();
                    }
                }
                return;
            }
        };
        super::layout::begin();
        if self
            .result(
                result,
                if switched {
                    "Project ready"
                } else {
                    "Project renamed"
                },
            )
            .is_some()
        {
            if switched {
                self.select_project_ratio(booth);
            }
            self.projects.finished(switched);
        }
    }
    pub fn apply_preferences(
        &mut self,
        booth: &mut Booth,
        preferences: &crate::preferences::Preferences,
    ) {
        self.inspector = preferences.inspector_visible;
        self.tab = preferences.inspector_tab.min(2);
        self.photo_size = [1280, 1920, 2560]
            .iter()
            .position(|value| *value == preferences.photo_longest)
            .unwrap_or(1);
        self.gif_size = [480, 720, 1080]
            .iter()
            .position(|value| *value == preferences.gif_longest)
            .unwrap_or(1);
        self.gif_fps = [10, 20, 25, 50]
            .iter()
            .position(|value| *value == preferences.gif_fps)
            .unwrap_or(1);
        self.gif_quality = [64, 128, 256]
            .iter()
            .position(|value| *value == preferences.gif_colors)
            .unwrap_or(1);
        booth.render_options.shadows = preferences.show_shadows;
        booth.render_options.pets = preferences.show_pets;
        booth.render_options.gems = preferences.show_gems;
        if booth.characters[0]
            .wardrobe()
            .available_languages
            .contains(&preferences.language)
        {
            booth.set_language(preferences.language.clone());
        }
    }

    pub fn capture_preferences(
        &self,
        booth: &Booth,
        preferences: &mut crate::preferences::Preferences,
    ) {
        preferences.inspector_visible = self.inspector;
        preferences.inspector_tab = self.tab;
        preferences.photo_longest = [1280, 1920, 2560][self.photo_size];
        preferences.gif_longest = [480, 720, 1080][self.gif_size];
        preferences.gif_fps = [10, 20, 25, 50][self.gif_fps];
        preferences.gif_colors = [64, 128, 256][self.gif_quality];
        preferences.show_shadows = booth.render_options.shadows;
        preferences.show_pets = booth.render_options.pets;
        preferences.show_gems = booth.render_options.gems;
        preferences.language.clone_from(&booth.language);
        preferences.last_project = booth.project.as_ref().map(|p| p.id.clone());
    }

    pub fn tick_recorder(&mut self) {
        if let Some(result) = self.recorder.tick() {
            match result {
                Ok(path) => notifications::success(format!("GIF saved: {}", path.display())),
                Err(error) if error == "Recording cancelled" => notifications::info(error),
                Err(error) => notifications::error(error),
            }
        }
    }
    pub fn take_photo(&mut self, booth: &mut Booth) {
        let (width, height) = self.output_size([1280, 1920, 2560][self.photo_size]);
        match booth.capture(width, height) {
            Ok(path) => notifications::success(format!("Photo saved: {}", path.display())),
            Err(error) => notifications::error(error),
        }
    }
    fn draw_ratio_picker(&mut self, font: &Font, bold: &Font, mouse: Vec2, blocked: bool) {
        self.ratio_transition.update(self.ratio_open);
        if !self.ratio_open && !self.ratio_transition.visible() {
            return;
        }
        let size = super::layout::size();
        let opener = BoothLayout::new(size, self.inspector).ratio;
        let panel = self.ratio_transition.panel(
            self.ratio_open,
            Rect::new(
                opener.x,
                opener.bottom() + GAP,
                116.0,
                (size.y - opener.bottom() - GAP - MARGIN)
                    .min(RATIOS.len() as f32 * CONTROL_HEIGHT)
                    .max(1.0),
            ),
        );
        let mouse = self.ratio_transition.mouse(
            self.ratio_open,
            if blocked {
                vec2(-10000.0, -10000.0)
            } else {
                mouse
            },
        );
        if super::widgets::dismiss_except(
            panel,
            Rect::new(-10000.0, -10000.0, 0.0, 0.0),
            opener,
            mouse,
        ) {
            self.ratio_open = false;
            self.ratio_drag.cancel();
        }
        let tapped = super::widgets::scroll(
            panel,
            mouse,
            &mut self.ratio_scroll,
            (RATIOS.len() as f32 * CONTROL_HEIGHT - panel.h).max(0.0),
            &mut self.ratio_drag,
        );
        let _opacity = self.ratio_transition.scope();
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, SURFACE);
        super::layout::clip(panel);
        for (i, (title, _, _)) in RATIOS.iter().enumerate() {
            let r = Rect::new(
                panel.x,
                panel.y + i as f32 * CONTROL_HEIGHT - self.ratio_scroll,
                panel.w,
                CONTROL_HEIGHT,
            );
            if r.bottom() <= panel.y || r.y >= panel.bottom() {
                continue;
            }
            let hit = panel.contains(mouse) && r.contains(mouse);
            if i == self.ratio || hit {
                draw_rectangle(r.x, r.y, r.w, r.h, CONTROL);
            }
            control_label(
                title,
                r,
                12,
                bold,
                if i == self.ratio { ACCENT } else { WHITE },
                false,
            );
            if tapped && hit {
                self.ratio = i;
                self.ratio_open = false;
                self.actor_drag = None;
                super::audio::click();
            }
        }
        super::layout::end_clip();
        let _ = font;
    }
    pub fn blocks_input(&self) -> bool {
        self.color_picker.visible()
            || self.ratio_open
            || self.ratio_transition.visible()
            || self.scene_picker
            || self.scene_transition.visible()
            || self.preset_transition.visible()
            || self.clip_picker
            || self.equipment.is_open()
            || self.projects.visible()
            || self.modal_this_frame
    }
    pub fn rects(&self) -> (Rect, Rect) {
        let layout = BoothLayout::new(super::layout::size(), self.inspector);
        let (_, rw, rh) = RATIOS[self.ratio];
        (layout.canvas(rw as f32 / rh as f32), layout.inspector)
    }
    pub fn output_size(&self, longest: u32) -> (u32, u32) {
        let (_, w, h) = RATIOS[self.ratio];
        let max = w.max(h);
        ((longest * w / max).max(1), (longest * h / max).max(1))
    }
    pub fn draw_canvas(&self, booth: &Booth) {
        let (canvas, _) = self.rects();
        draw_rectangle(
            canvas.x - 1.0,
            canvas.y - 1.0,
            canvas.w + 2.0,
            canvas.h + 2.0,
            Color::from_rgba(55, 56, 68, 255),
        );
        draw_texture_ex(
            &booth.target.texture,
            canvas.x,
            canvas.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(canvas.w, canvas.h)),
                flip_y: true,
                ..Default::default()
            },
        );
        if let Some(c) = booth.selected() {
            let point = world_to_canvas(c.placement.position, vec2(canvas.w, canvas.h))
                + vec2(canvas.x, canvas.y);
            draw_line(
                point.x - 8.0,
                point.y + 5.0,
                point.x + 8.0,
                point.y + 5.0,
                2.0,
                ACCENT,
            );
        }
    }
    fn result<T>(&mut self, result: Result<T, String>, success: &str) -> Option<T> {
        match result {
            Ok(value) => {
                notifications::success(success);
                Some(value)
            }
            Err(error) => {
                notifications::error(error);
                None
            }
        }
    }
    pub fn draw(&mut self, booth: &mut Booth, font: &Font, bold: &Font, blocked: bool) {
        self.modal_this_frame = self.scene_picker
            || self.ratio_open
            || self.ratio_transition.visible()
            || self.color_picker.visible()
            || self.scene_transition.visible()
            || self.preset_transition.visible()
            || self.clip_picker
            || self.equipment.is_open()
            || self.projects.visible();
        let size = super::layout::size();
        let raw_mouse = super::layout::mouse();
        let modal = self.blocks_input();
        let mouse = if blocked || modal {
            vec2(-10000.0, -10000.0)
        } else {
            raw_mouse
        };
        let layout = BoothLayout::new(size, self.inspector);
        draw_rectangle(0.0, 0.0, size.x, TOPBAR_HEIGHT, SURFACE);
        super::settings::Settings::draw_button(layout.settings, bold);
        let title_left = layout.ratio.right() + MARGIN;
        let title_right = if layout.wrapped {
            layout.output[0].x
        } else {
            layout.inspector.x
        };
        if title_right - title_left > 80.0 {
            control_label(
                booth
                    .project
                    .as_ref()
                    .map(|project| project.name.as_str())
                    .unwrap_or("Choose a project"),
                Rect::new(
                    title_left,
                    layout.ratio.y,
                    title_right - title_left - GAP,
                    CONTROL_HEIGHT,
                ),
                13,
                self.project_font.as_ref().unwrap_or(font),
                ACCENT,
                false,
            );
        }
        let actions = ["+ Role", "- Role", "Preview", "Scene", "Inspect"];
        let action_font_size = [12, 11, 10]
            .into_iter()
            .find(|size| {
                actions.iter().zip(&layout.actions).all(|(title, rect)| {
                    measure_text(title, Some(bold), *size, 1.0).width <= rect.w - 8.0
                })
            })
            .unwrap_or(10);
        for (i, title) in actions.iter().enumerate() {
            let r = layout.actions[i];
            let enabled = match i {
                1 | 2 => booth.selected.is_some(),
                _ => true,
            };
            let clicked = centered_button(
                title,
                r,
                action_font_size,
                bold,
                mouse,
                enabled,
                i == 4 && self.inspector,
                CONTROL,
            );
            if clicked {
                match i {
                    0 => {
                        let result = booth.add_or_restore();
                        self.result(result, "Independent role added");
                    }
                    1 => booth.remove(),
                    2 => self.preview = true,
                    3 => {
                        self.scene_picker = true;
                        self.scene_gallery.open();
                    }
                    4 => self.inspector = !self.inspector,
                    _ => {}
                }
            }
        }
        let mut deferred = None;
        let footer = layout.footer;
        let role_width = footer.w;
        let total = booth.active_count();
        if total > 0 {
            for (i, title) in ["<", ">"].iter().enumerate() {
                if centered_button(
                    title,
                    Rect::new(footer.x + i as f32 * (32.0 + GAP), footer.y, 32.0, footer.h),
                    12,
                    font,
                    mouse,
                    true,
                    false,
                    CONTROL,
                ) {
                    let current = booth.selected.unwrap_or(0);
                    booth.selected = Some((current + if i == 0 { total - 1 } else { 1 }) % total);
                }
            }
            let selected = booth.selected.unwrap_or(0);
            control_label(
                &format!(
                    "Role {} / {} · {}",
                    selected + 1,
                    total,
                    booth.characters[selected].name()
                ),
                Rect::new(
                    footer.x + 72.0,
                    footer.y,
                    (role_width - 72.0).max(1.0),
                    footer.h,
                ),
                12,
                font,
                WHITE,
                false,
            );
        } else {
            control_label(
                "No roles · tap + Role to add",
                Rect::new(footer.x, footer.y, role_width, footer.h),
                12,
                font,
                GRAY,
                false,
            );
        }
        // Small windows wrap output controls into the toolbar's second row.
        for (i, title) in ["Projects", "Save project"].iter().enumerate() {
            let r = layout.output[i];
            let clicked = centered_button(
                title,
                r,
                if layout.wrapped { 10 } else { 11 },
                bold,
                mouse,
                !self.recorder.busy(),
                false,
                if layout.wrapped { SURFACE } else { CONTROL },
            );
            if clicked {
                deferred = Some(i);
            }
        }
        self.equipment.toolbar(booth, self.rects().0, font, mouse);
        self.handle_drag(booth, mouse, blocked || modal || self.equipment.is_open());
        if self.inspector {
            self.draw_inspector(
                booth,
                font,
                bold,
                if self.equipment.is_open() {
                    vec2(-10000.0, -10000.0)
                } else {
                    mouse
                },
            );
        }
        let ratio_button = layout.ratio;
        if centered_button(
            &format!("{}  v", RATIOS[self.ratio].0),
            ratio_button,
            12,
            bold,
            mouse,
            true,
            false,
            SURFACE,
        ) {
            self.ratio_open = true;
        }
        self.draw_ratio_picker(font, bold, raw_mouse, blocked || self.projects.visible());
        self.color_picker
            .draw(booth, font, bold, blocked || self.projects.visible());
        if self.scene_picker || self.scene_transition.visible() {
            self.draw_scene_picker(
                booth,
                font,
                bold,
                if blocked || self.projects.visible() {
                    vec2(-10000.0, -10000.0)
                } else {
                    raw_mouse
                },
            );
        }
        if self.clip_picker || self.preset_transition.visible() {
            self.draw_clip_picker(
                booth,
                font,
                bold,
                if blocked || self.projects.visible() {
                    vec2(-10000.0, -10000.0)
                } else {
                    raw_mouse
                },
            );
        }
        if let Some(result) = self.equipment.draw(
            booth,
            font,
            bold,
            if blocked || self.projects.visible() {
                vec2(-10000.0, -10000.0)
            } else {
                raw_mouse
            },
        ) {
            self.result(result, "Booth selection updated");
        }
        if let Some(action) = deferred {
            match action {
                0 => {
                    self.projects.open();
                }
                1 => {
                    self.save_project(booth);
                }
                _ => {}
            }
            super::layout::begin();
        }
        if self.projects.visible() {
            if let Some((action, save_first)) = self.projects.draw(
                booth,
                font,
                bold,
                self.project_font.as_ref().unwrap_or(font),
                if blocked {
                    super::widgets::BLOCKED
                } else {
                    raw_mouse
                },
                self.recorder.busy(),
            ) {
                self.project_action(booth, action, save_first);
                super::layout::begin();
            }
        }
        self.modal_this_frame |= self.projects.visible();
        if self.photo_requested {
            self.photo_requested = false;
            self.take_photo(booth);
            super::layout::begin();
        }
    }
    pub fn preview_close(&mut self, font: &Font, blocked: bool) {
        let size = super::layout::size();
        let r = super::layout::preview_back_button(size, super::layout::compact());
        let mouse = if blocked {
            vec2(-10000.0, -10000.0)
        } else {
            super::layout::mouse()
        };
        let clicked = button_with_fill(
            "",
            r,
            font,
            mouse,
            true,
            false,
            Color::from_rgba(30, 31, 40, 255),
        );
        control_label(
            "Back to booth",
            r,
            13,
            font,
            if r.contains(mouse) {
                Color::from_rgba(236, 154, 92, 255)
            } else {
                Color::new(0.90, 0.91, 0.94, 1.0)
            },
            true,
        );
        if clicked || (!blocked && is_key_pressed(KeyCode::Escape)) {
            self.preview = false;
        }
    }
    fn handle_drag(&mut self, booth: &mut Booth, mouse: Vec2, blocked: bool) {
        let (canvas, _) = self.rects();
        let size = vec2(canvas.w, canvas.h);
        if blocked {
            self.actor_drag = None;
            return;
        }
        if is_mouse_button_pressed(MouseButton::Left)
            && canvas.contains(mouse)
            && !super::equipment::EquipmentPicker::bar(canvas).contains(mouse)
        {
            let mut candidates = vec![];
            for (i, c) in booth
                .characters
                .iter()
                .take(booth.active_count())
                .enumerate()
            {
                let scale = perspective(
                    &booth.backgrounds[booth.background],
                    c.placement.position[1],
                );
                let foot = world_to_canvas([c.placement.position[0], c.placement.height], size)
                    + vec2(canvas.x, canvas.y);
                let h = 3.7 * scale * c.placement.scale * size.y / 10.0;
                let bounds = Rect::new(foot.x - h * 0.5, foot.y - h, h, h + 12.0);
                if bounds.contains(mouse) {
                    candidates.push((c.placement.position[1], i, false, c.placement.position));
                }
                if c.has_pet() {
                    let pos = c.pet_position(scale);
                    let foot = world_to_canvas(pos, size) + vec2(canvas.x, canvas.y);
                    let h =
                        1.7 * perspective(&booth.backgrounds[booth.background], pos[1]) * size.y
                            / 10.0;
                    if Rect::new(foot.x - h * 0.6, foot.y - h, h * 1.2, h + 10.0).contains(mouse) {
                        candidates.push((pos[1], i, true, pos));
                    }
                }
            }
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, i, pet, pos)) = candidates.first() {
                booth.selected = Some(*i);
                let world = canvas_to_world(mouse - vec2(canvas.x, canvas.y), size);
                self.actor_drag = Some((*i, *pet, vec2(pos[0] - world[0], pos[1] - world[1])));
            }
        }
        if !is_mouse_button_down(MouseButton::Left) {
            self.actor_drag = None;
            return;
        }
        if let Some((i, pet, offset)) = self.actor_drag {
            let world = canvas_to_world(mouse - vec2(canvas.x, canvas.y), size);
            let x = (world[0] + offset.x).clamp(-8.88, 8.88);
            let y = (world[1] + offset.y).clamp(-5.0, 4.5);
            if let Some(c) = booth.characters.get_mut(i) {
                if pet {
                    let scale = perspective(
                        &booth.backgrounds[booth.background],
                        c.placement.position[1],
                    );
                    c.placement.pet_position = if c.placement.pet_reference == ReferenceFrame::World
                    {
                        [x, y]
                    } else {
                        [
                            (x - c.placement.position[0]) / scale,
                            (y - c.placement.position[1]) / scale,
                        ]
                    };
                } else {
                    let delta = y - c.placement.position[1];
                    c.placement.position = [x, y];
                    c.placement.height += delta;
                }
            }
        }
    }
    fn draw_inspector(&mut self, booth: &mut Booth, font: &Font, bold: &Font, mouse: Vec2) {
        let background = booth.backgrounds[booth.background].clone();
        let (_, panel) = self.rects();
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, SURFACE);
        let inner_width = panel.w - PANEL_PADDING * 2.0;
        let tab_width = (inner_width - GAP * 2.0) / 3.0;
        let titles = ["Transform", "Animation", "Recorder"];
        let font_size = [11, 10, 9]
            .into_iter()
            .find(|size| {
                titles.iter().enumerate().all(|(i, title)| {
                    measure_text(title, Some(font), *size, 1.0).width
                        + if i == 2 { 10.0 } else { 0.0 }
                        <= tab_width - 8.0
                })
            })
            .unwrap_or(9);
        for (i, title) in titles.iter().enumerate() {
            let rect = Rect::new(
                panel.x + PANEL_PADDING + i as f32 * (tab_width + GAP),
                panel.y + PANEL_PADDING,
                tab_width,
                CONTROL_HEIGHT,
            );
            if button("", rect, font, mouse, true, self.tab == i) {
                self.tab = i;
                self.scroll = 0.0;
                self.scroll_drag.cancel();
                self.slider = None;
                self.slider_start = None;
                self.content_tap = None;
            }
            let marker_width = if i == 2 { 10.0 } else { 0.0 };
            let text = measure_text(title, Some(font), font_size, 1.0);
            let start = rect.x + (rect.w - text.width - marker_width) * 0.5;
            if i == 2 {
                super::widgets::paint::draw_circle(start + 3.0, rect.y + rect.h * 0.5, 3.0, RED);
            }
            label(
                title,
                start + if i == 2 { 10.0 } else { 0.0 },
                rect.y + (rect.h - text.height) * 0.5 + text.offset_y,
                font_size,
                font,
                if self.tab == i { ACCENT } else { WHITE },
                rect.w,
            );
        }
        let area = Rect::new(
            panel.x + PANEL_PADDING,
            panel.y + CONTROL_HEIGHT + PANEL_PADDING * 2.0,
            inner_width,
            (panel.h - CONTROL_HEIGHT - PANEL_PADDING * 3.0).max(1.0),
        );
        let max = (self.inspector_heights[self.tab] - area.h).max(0.0);
        self.scroll = self.scroll.clamp(0.0, max);
        let pressed = is_mouse_button_pressed(MouseButton::Left);
        let released = is_mouse_button_released(MouseButton::Left);
        if !area.contains(mouse) && mouse.x < -9999.0 {
            self.content_tap = None;
            self.slider = None;
            self.slider_start = None;
            self.scroll_drag.cancel();
        }
        if pressed && area.contains(mouse) {
            self.content_tap = Some(mouse);
        }
        if self
            .content_tap
            .is_some_and(|start| start.distance(mouse) > 7.0)
        {
            self.content_tap = None;
        }
        let tapped = released && self.content_tap.take().is_some();
        let scroll_gesture = self.slider_start.is_some_and(|(start, _)| {
            let movement = mouse - start;
            movement.y.abs() > 7.0 && movement.y.abs() > movement.x.abs()
        });
        // A vertical gesture that starts on a slider scrolls the panel and
        // restores the value touched on press. Horizontal movement edits it.
        if self.slider.is_none() || scroll_gesture {
            super::widgets::scroll(area, mouse, &mut self.scroll, max, &mut self.scroll_drag);
        }
        super::layout::clip(area);
        let hit_mouse = if self.slider.is_some() || (area.contains(mouse) && (tapped || pressed)) {
            mouse
        } else {
            vec2(-10000.0, -10000.0)
        };
        let x = area.x;
        let mut y = area.y - self.scroll;
        let w = area.w;
        CONTENT_BUTTONS.with(|value| value.set(true));
        CONTENT_TAP.with(|value| value.set(tapped));
        if self.tab == 2 {
            label("PHOTO & GIF", x, y + 19.0, 12, bold, GRAY, w);
            y += 32.0;
            if button(
                "Photo · PNG (F12)",
                Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                bold,
                hit_mouse,
                true,
                false,
            ) {
                self.photo_requested = true;
            }
            y += ROW_STEP;
            let can_record = booth.selected().is_some_and(|c| {
                c.duration() > 0.0 && c.animation.tracks[c.animation.control_track].speed > 0.0
            });
            if button(
                if self.recorder.busy() {
                    "Cancel recording"
                } else {
                    "Record GIF"
                },
                Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                font,
                hit_mouse,
                self.recorder.busy() || can_record,
                self.recorder.busy(),
            ) {
                if self.recorder.busy() {
                    self.recorder.cancel();
                } else {
                    let (width, height) = self.output_size([480, 720, 1080][self.gif_size]);
                    let options = crate::recorder::Options {
                        width,
                        height,
                        fps: [10, 20, 25, 50][self.gif_fps],
                        colors: [64, 128, 256][self.gif_quality],
                        dither: self.gif_quality > 0,
                    };
                    let result = booth.project_directory().and_then(|directory| {
                        self.recorder.start(booth, options, &directory.join("gifs"))
                    });
                    if let Err(error) = result {
                        notifications::error(error);
                    }
                }
            }
            y += 52.0;
            let duration = booth.selected().map_or(0.0, |c| {
                c.duration()
                    / c.animation.tracks[c.animation.control_track]
                        .speed
                        .max(0.00001)
            });
            label(
                &if self.recorder.busy() {
                    self.recorder.progress.clone()
                } else {
                    format!("Duration: {duration:.2}s · selected role control track")
                },
                x,
                y,
                11,
                font,
                GRAY,
                w,
            );
            y += 16.0;
            label(
                "GIF: ffmpeg required · project/gifs/",
                x,
                y,
                11,
                font,
                GRAY,
                w,
            );
            y += 20.0;
            let titles = [
                format!("Photo size: {} px", [1280, 1920, 2560][self.photo_size]),
                format!(
                    "GIF size: {} · {} px",
                    ["Small", "Balanced", "Large"][self.gif_size],
                    [480, 720, 1080][self.gif_size]
                ),
                format!("GIF frame rate: {} fps", [10, 20, 25, 50][self.gif_fps]),
                format!(
                    "GIF quality: {}",
                    ["Small file", "Balanced", "Best"][self.gif_quality]
                ),
                format!(
                    "Shadows: {}",
                    if booth.render_options.shadows {
                        "on"
                    } else {
                        "off"
                    }
                ),
                format!(
                    "Pets: {}",
                    if booth.render_options.pets {
                        "on"
                    } else {
                        "off"
                    }
                ),
                format!(
                    "Level gems: {}",
                    if booth.render_options.gems {
                        "on"
                    } else {
                        "off"
                    }
                ),
            ];
            for (i, title) in titles.iter().enumerate() {
                if button(
                    title,
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    !self.recorder.busy(),
                    false,
                ) {
                    match i {
                        0 => self.photo_size = (self.photo_size + 1) % 3,
                        1 => self.gif_size = (self.gif_size + 1) % 3,
                        2 => self.gif_fps = (self.gif_fps + 1) % 4,
                        3 => self.gif_quality = (self.gif_quality + 1) % 3,
                        4 => booth.render_options.shadows = !booth.render_options.shadows,
                        5 => booth.render_options.pets = !booth.render_options.pets,
                        _ => booth.render_options.gems = !booth.render_options.gems,
                    }
                }
                y += ROW_STEP;
            }
            label("SCENE & RENDER", x, y + 19.0, 12, bold, GRAY, w);
            y += 32.0;
            let name = super::scenes::name(&booth.backgrounds[booth.background]);
            if button(
                &format!("Background: {name}"),
                Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                font,
                hit_mouse,
                true,
                false,
            ) {
                self.scene_picker = true;
                self.scene_gallery.open();
            }
            y += ROW_STEP;
            if button(
                match booth.watermark {
                    Watermark::Hidden => "Watermark: hidden",
                    Watermark::Left => "Watermark: bottom left",
                    Watermark::Right => "Watermark: bottom right",
                },
                Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                font,
                hit_mouse,
                true,
                false,
            ) {
                booth.watermark = match booth.watermark {
                    Watermark::Hidden => Watermark::Right,
                    Watermark::Right => Watermark::Left,
                    Watermark::Left => Watermark::Hidden,
                };
            }
            y += ROW_STEP;
            if button(
                if booth.transparent {
                    "Transparent PNG / GIF: on"
                } else {
                    "Transparent PNG / GIF: off"
                },
                Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                font,
                hit_mouse,
                booth.backgrounds[booth.background].transparency_option,
                booth.transparent,
            ) {
                booth.transparent = !booth.transparent;
            }
            y += 52.0;
            label(
                "Green backdrop supports transparency",
                x,
                y,
                11,
                font,
                GRAY,
                w,
            );
            y += 20.0;
            let (width, height) = self.output_size([1280, 1920, 2560][self.photo_size]);
            label(
                &format!("Photo: {width} × {height} · excludes GUI"),
                x,
                y,
                11,
                font,
                GRAY,
                w,
            );
            y += 28.0;
            label("PROJECT OUTPUT", x, y + 19.0, 12, bold, GRAY, w);
            y += 44.0;
            label(
                &booth
                    .project_directory()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| "Create a project to export".into()),
                x,
                y,
                11,
                font,
                GRAY,
                w,
            );
        } else if let Some(c) = booth.selected_mut() {
            if self.tab == 0 {
                label(c.name(), x, y + 20.0, 14, bold, WHITE, w);
                y += 32.0;
                let tint = Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT);
                if button("Color Tint", tint, font, hit_mouse, true, false) {
                    self.color_picker.open(c.id.clone(), c.placement.color);
                }
                super::widgets::paint::draw_swatch(
                    x + w - 34.0,
                    y + 5.0,
                    26.0,
                    26.0,
                    Color::new(
                        c.placement.color[0],
                        c.placement.color[1],
                        c.placement.color[2],
                        c.placement.color[3],
                    ),
                );
                y += ROW_STEP;
                let p = &mut c.placement;
                slider(
                    "Left / right (scene units)",
                    &mut p.position[0],
                    -8.88,
                    8.88,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                let old_y = p.position[1];
                slider(
                    "Near / far (higher = farther)",
                    &mut p.position[1],
                    -5.0,
                    4.5,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                p.height += p.position[1] - old_y;
                let mut elevation = p.height - p.position[1];
                slider(
                    "Height above ground (0 = grounded)",
                    &mut elevation,
                    -5.0 - p.position[1],
                    8.0 - p.position[1],
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                p.height = p.position[1] + elevation;
                slider(
                    "Character size",
                    &mut p.scale,
                    0.05,
                    3.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                slider(
                    "Character rotation",
                    &mut p.rotation,
                    -180.0,
                    180.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                slider(
                    "Target relative angle",
                    &mut p.target_angle,
                    -180.0,
                    180.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                slider(
                    "Shadow opacity",
                    &mut p.shadow_alpha,
                    0.0,
                    1.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                if button(
                    if p.face_right {
                        "Character mirror: off"
                    } else {
                        "Character mirror: on"
                    },
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    true,
                    !p.face_right,
                ) {
                    p.face_right = !p.face_right;
                }
                y += ROW_STEP;
                if button(
                    if p.pet_face_right {
                        "Pet mirror: off"
                    } else {
                        "Pet mirror: on"
                    },
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    true,
                    !p.pet_face_right,
                ) {
                    p.pet_face_right = !p.pet_face_right;
                }
                y += ROW_STEP;
                let scale = perspective(&background, p.position[1]);
                if button(
                    if p.pet_reference == ReferenceFrame::Character {
                        "Pet frame: character"
                    } else {
                        "Pet frame: world"
                    },
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    true,
                    false,
                ) {
                    p.pet_position = if p.pet_reference == ReferenceFrame::Character {
                        [
                            p.position[0] + p.pet_position[0] * scale,
                            p.position[1] + p.pet_position[1] * scale,
                        ]
                    } else {
                        [
                            (p.pet_position[0] - p.position[0]) / scale,
                            (p.pet_position[1] - p.position[1]) / scale,
                        ]
                    };
                    p.pet_reference = if p.pet_reference == ReferenceFrame::Character {
                        ReferenceFrame::World
                    } else {
                        ReferenceFrame::Character
                    };
                }
                y += ROW_STEP;
                slider(
                    "Pet X",
                    &mut p.pet_position[0],
                    -12.0,
                    12.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                slider(
                    "Pet Y",
                    &mut p.pet_position[1],
                    -8.0,
                    8.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                let mut level = c.level as f32;
                slider(
                    "Level / GEM",
                    &mut level,
                    0.0,
                    999.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                c.level = level.round() as u32;
            } else {
                if button(
                    &format!("Preset: {}  >", c.animation.action.label()),
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    true,
                    false,
                ) {
                    self.clip_picker = true;
                }
                y += ROW_STEP;
                if button(
                    if c.animation.paused { "Play" } else { "Pause" },
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    true,
                    c.animation.paused,
                ) {
                    c.animation.paused = !c.animation.paused;
                }
                y += ROW_STEP;
                let duration = c.duration();
                let mut time = c.animation.tracks[c.animation.control_track].time;
                if duration > 0.0 {
                    time = if c.animation.tracks[c.animation.control_track].looping {
                        time.rem_euclid(duration)
                    } else {
                        time.min(duration)
                    };
                }
                if slider(
                    "Progress (seconds)",
                    &mut time,
                    0.0,
                    duration.max(0.001),
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                ) {
                    c.seek(time);
                }
                let t = &mut c.animation.tracks[c.animation.control_track];
                slider(
                    "Playback speed",
                    &mut t.speed,
                    0.0,
                    4.0,
                    x,
                    &mut y,
                    w,
                    font,
                    hit_mouse,
                    &mut self.slider,
                    &mut self.slider_start,
                );
                if button(
                    if t.looping { "Loop: on" } else { "Loop: off" },
                    Rect::new(x, y, w, INSPECTOR_CONTROL_HEIGHT),
                    font,
                    hit_mouse,
                    true,
                    t.looping,
                ) {
                    t.looping = !t.looping;
                }
                y += ROW_STEP;
            }
        } else {
            label(
                "Add a role to edit its properties",
                x,
                y + 24.0,
                12,
                font,
                GRAY,
                w,
            );
            y += INSPECTOR_CONTROL_HEIGHT;
        }
        self.inspector_heights[self.tab] = (y + self.scroll - area.y + 12.0).max(0.0);
        self.scroll = self
            .scroll
            .clamp(0.0, (self.inspector_heights[self.tab] - area.h).max(0.0));
        super::layout::end_clip();
        CONTENT_BUTTONS.with(|value| value.set(false));
        CONTENT_TAP.with(|value| value.set(false));
        if !is_mouse_button_down(MouseButton::Left) {
            self.slider = None;
            self.slider_start = None;
        }
    }
    fn draw_scene_picker(&mut self, booth: &mut Booth, font: &Font, bold: &Font, mouse: Vec2) {
        let size = super::layout::size();
        let panel = self.scene_transition.panel(
            self.scene_picker,
            super::scenes::SceneGallery::panel(size, booth.backgrounds.len()),
        );
        self.scene_transition.backdrop();
        let mut mouse = self.scene_transition.mouse(self.scene_picker, mouse);
        let _opacity = self.scene_transition.scope();
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, super::widgets::SURFACE);
        let close = super::widgets::popup_header(panel, "Booth scenes", font, bold);
        if super::widgets::dismiss(panel, close, mouse) {
            self.scene_picker = false;
            self.scene_gallery.cancel();
            mouse = super::widgets::BLOCKED;
        }
        if let Some(index) = self.scene_gallery.draw(panel, booth, font, bold, mouse) {
            booth.switch_background(index);
            self.scene_picker = false;
            self.scene_gallery.cancel();
            super::audio::click();
        }
    }
    fn draw_clip_picker(&mut self, booth: &mut Booth, font: &Font, bold: &Font, mouse: Vec2) {
        let Some(c) = booth.selected_mut() else {
            self.clip_picker = false;
            self.preset_transition.update(false);
            return;
        };
        let size = super::layout::size();
        let w = (size.x - 32.0).min(540.0);
        let h = (size.y - 56.0).max(1.0);
        let x = (size.x - w) * 0.5;
        let panel = self
            .preset_transition
            .panel(self.clip_picker, Rect::new(x, 28.0, w, h));
        let y = panel.y;
        self.preset_transition.backdrop();
        let mut mouse = self.preset_transition.mouse(self.clip_picker, mouse);
        let _opacity = self.preset_transition.scope();
        draw_rectangle(x, y, w, h, super::widgets::SURFACE);
        let close = super::widgets::popup_header(panel, "Animation presets", font, bold);
        if super::widgets::dismiss(panel, close, mouse) {
            self.clip_picker = false;
            self.clip_drag.cancel();
            mouse = super::widgets::BLOCKED;
        }
        let names = crate::model::Action::ALL;
        let area = Rect::new(x + 12.0, y + 50.0, w - 24.0, (h - 62.0).max(1.0));
        let max = (names.len() as f32 * 38.0 - area.h).max(0.0);
        let tap =
            super::widgets::scroll(area, mouse, &mut self.clip_scroll, max, &mut self.clip_drag);
        let mut chosen = None;
        super::layout::clip(area);
        for (i, name) in names.iter().enumerate() {
            let r = Rect::new(
                area.x,
                area.y + i as f32 * 38.0 - self.clip_scroll,
                area.w,
                36.0,
            );
            if r.bottom() <= area.y || r.y >= area.bottom() {
                continue;
            }
            draw_rectangle(r.x, r.y, r.w, r.h, CONTROL);
            label(
                name.label(),
                r.x + 8.0,
                r.y + 24.0,
                12,
                font,
                WHITE,
                r.w - 16.0,
            );
            if tap && area.contains(mouse) && r.contains(mouse) {
                chosen = Some(*name);
            }
        }
        super::layout::end_clip();
        if let Some(name) = chosen {
            c.set_action(name);
            self.clip_picker = false;
            super::audio::click();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn slider(
    title: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    x: f32,
    y: &mut f32,
    w: f32,
    font: &Font,
    mouse: Vec2,
    active: &mut Option<String>,
    start: &mut Option<(Vec2, f32)>,
) -> bool {
    let mut changed = false;
    let value_text = format!("{value:.2}");
    let value_width = measure_text(&value_text, Some(font), 11, 1.0).width;
    label(
        title,
        x,
        *y + 15.0,
        11,
        font,
        GRAY,
        (w - value_width - PANEL_PADDING).max(0.0),
    );
    label(
        &value_text,
        x + w - value_width,
        *y + 15.0,
        11,
        font,
        WHITE,
        value_width + 1.0,
    );
    let hit = Rect::new(x, *y + 18.0, w, 26.0);
    let track_x = x + 3.0;
    let track_width = (w - 6.0).max(1.0);
    if is_mouse_button_pressed(MouseButton::Left) && hit.contains(mouse) {
        *active = Some(title.into());
        *start = Some((mouse, *value));
    }
    if active.as_deref() == Some(title) && mouse.x > -9999.0 {
        let scrolling = start.is_some_and(|(point, _)| {
            let movement = mouse - point;
            movement.y.abs() > 7.0 && movement.y.abs() > movement.x.abs()
        });
        *value = if scrolling {
            let original = start.unwrap().1;
            *active = None;
            *start = None;
            original
        } else {
            min + ((mouse.x - track_x) / track_width).clamp(0.0, 1.0) * (max - min)
        };
        changed = true;
    }
    let t = ((*value - min) / (max - min).max(0.00001)).clamp(0.0, 1.0);
    draw_rectangle(track_x, *y + 30.0, track_width, 2.0, CONTROL);
    draw_rectangle(track_x, *y + 30.0, track_width * t, 2.0, ACCENT);
    draw_rectangle(
        track_x + track_width * t - 3.0,
        *y + 24.0,
        6.0,
        14.0,
        ACCENT,
    );
    *y += 48.0;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_and_inspector_fit_small_portrait_landscape_and_desktop_windows() {
        for size in [
            vec2(480.0, 320.0),
            vec2(640.0, 360.0),
            vec2(360.0, 640.0),
            vec2(480.0, 720.0),
            vec2(1280.0, 720.0),
        ] {
            for inspector in [true, false] {
                let layout = BoothLayout::new(size, inspector);
                let panel = layout.inspector;
                for (_, rw, rh) in RATIOS {
                    let aspect = rw as f32 / rh as f32;
                    let canvas = layout.canvas(aspect);
                    if inspector {
                        assert!(!canvas.overlaps(&panel));
                    }
                    assert!(!layout.equipment.overlaps(&canvas));
                    assert!(!layout.ratio.overlaps(&canvas));
                    assert!((canvas.w / canvas.h - aspect).abs() < 0.001);
                    for (rect, max_bottom) in [
                        (canvas, layout.footer.y - MARGIN),
                        (
                            panel,
                            if panel.x > MARGIN {
                                size.y - 8.0
                            } else {
                                layout.footer.y - GAP
                            },
                        ),
                    ] {
                        assert!(rect.w > 0.0 && rect.h > 0.0);
                        assert!(rect.x >= 0.0 && rect.y >= TOPBAR_HEIGHT);
                        assert!(rect.right() <= size.x && rect.bottom() <= max_bottom + 0.001);
                    }
                    if inspector {
                        assert!(!layout.footer.overlaps(&panel));
                    }
                }
            }
        }
    }
}
