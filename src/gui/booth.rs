//! Touch-sized booth controls, a scrollable inspector and the retained Preview layer.
use super::widgets::paint::{draw_line, draw_rectangle, draw_text_ex, draw_texture_ex};
use super::{drag_scroll::DragScroll, input::NativeInput};
use crate::{
    booth::{Booth, canvas_to_world, perspective, world_to_canvas},
    model::{ReferenceFrame, Watermark},
};
use macroquad::prelude::*;

const ACCENT: Color = Color::new(0.925, 0.604, 0.361, 1.0);
const SURFACE: Color = Color::new(0.078, 0.082, 0.11, 0.97);
const CONTROL: Color = Color::new(0.12, 0.125, 0.16, 1.0);

pub struct BoothUi {
    pub preview: bool,
    pub status: String,
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
    picker_scroll: f32,
    picker_drag: DragScroll,
    native_input: NativeInput,
    input_kind: Option<InputKind>,
    clip_picker: bool,
    clip_scroll: f32,
    clip_drag: DragScroll,
    equipment: super::equipment::EquipmentPicker,
}
enum InputKind {
    SaveDirectory,
}
impl Default for BoothUi {
    fn default() -> Self {
        Self {
            preview: false,
            status: String::new(),
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
            picker_scroll: 0.0,
            picker_drag: DragScroll::default(),
            native_input: NativeInput::default(),
            input_kind: None,
            clip_picker: false,
            clip_scroll: 0.0,
            clip_drag: DragScroll::default(),
            equipment: super::equipment::EquipmentPicker::default(),
        }
    }
}

pub fn label(text: &str, x: f32, y: f32, size: u16, font: &Font, color: Color, width: f32) {
    let mut fitted = text.to_owned();
    while !fitted.is_empty() && measure_text(&fitted, Some(font), size, 1.0).width > width {
        fitted.pop();
    }
    draw_text_ex(
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
    label(
        title,
        r.x + 8.0,
        r.y + r.h * 0.5 + 5.0,
        12,
        font,
        if active {
            ACCENT
        } else if enabled {
            WHITE
        } else {
            GRAY
        },
        r.w - 16.0,
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

fn regions(size: Vec2, inspector: bool) -> (Rect, Rect) {
    let side = size.x >= 700.0 || (size.x >= 460.0 && size.y < 500.0);
    let panel = if side {
        let width = (size.x * 0.42).clamp(220.0, 286.0);
        let top = if size.x < 840.0 { 100.0 } else { 60.0 };
        Rect::new(
            size.x - width - 12.0,
            top,
            width,
            (size.y - top - 12.0).max(1.0),
        )
    } else {
        let top = (size.y * 0.40).max(166.0);
        Rect::new(
            12.0,
            top,
            (size.x - 24.0).max(1.0),
            (size.y - top - 44.0).max(1.0),
        )
    };
    let canvas = if !inspector {
        Rect::new(
            12.0,
            100.0,
            (size.x - 24.0).max(1.0),
            (size.y - 144.0).max(1.0),
        )
    } else if side {
        Rect::new(
            12.0,
            100.0,
            (panel.x - 24.0).max(1.0),
            (size.y - 144.0).max(1.0),
        )
    } else {
        Rect::new(
            12.0,
            100.0,
            (size.x - 24.0).max(1.0),
            (panel.y - 108.0).max(1.0),
        )
    };
    let aspect = 16.0 / 9.0;
    let fitted = if canvas.w / canvas.h > aspect {
        let w = canvas.h * aspect;
        Rect::new(canvas.x + (canvas.w - w) * 0.5, canvas.y, w, canvas.h)
    } else {
        let h = canvas.w / aspect;
        Rect::new(canvas.x, canvas.y + (canvas.h - h) * 0.5, canvas.w, h)
    };
    (fitted, panel)
}

impl BoothUi {
    pub fn blocks_input(&self) -> bool {
        self.color_picker.visible()
            || self.scene_picker
            || self.scene_transition.visible()
            || self.preset_transition.visible()
            || self.clip_picker
            || self.equipment.is_open()
            || self.native_input.active()
            || self.modal_this_frame
    }
    pub fn rects(&self) -> (Rect, Rect) {
        regions(super::layout::size(), self.inspector)
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
                self.status = success.into();
                Some(value)
            }
            Err(error) => {
                tracing::warn!("{error}");
                self.status = error;
                None
            }
        }
    }
    fn poll_input(&mut self, booth: &mut Booth) {
        if let Some(text) = self.native_input.poll() {
            match self.input_kind.take() {
                Some(InputKind::SaveDirectory) => {
                    if !text.trim().is_empty() {
                        booth.save_directory = std::path::PathBuf::from(text.trim());
                    }
                }
                None => {}
            }
        }
        if !self.native_input.active() {
            self.input_kind = None;
        }
    }
    pub fn draw(&mut self, booth: &mut Booth, font: &Font, bold: &Font, blocked: bool) {
        self.modal_this_frame = self.scene_picker
            || self.color_picker.visible()
            || self.scene_transition.visible()
            || self.preset_transition.visible()
            || self.clip_picker
            || self.equipment.is_open()
            || self.native_input.active();
        self.poll_input(booth);
        let size = super::layout::size();
        let raw_mouse = super::layout::mouse();
        let modal = self.blocks_input();
        let mouse = if blocked || modal {
            vec2(-10000.0, -10000.0)
        } else {
            raw_mouse
        };
        draw_rectangle(0.0, 0.0, size.x, 50.0, SURFACE);
        let narrow = size.x < 840.0;
        let button_w = if narrow { (size.x - 116.0) / 5.0 } else { 92.0 };
        for (i, title) in ["+ Role", "- Role", "Preview", "Scene", "Inspect"]
            .iter()
            .enumerate()
        {
            let r = Rect::new(12.0 + i as f32 * button_w, 9.0, button_w - 4.0, 34.0);
            let enabled = match i {
                1 | 2 => booth.selected.is_some(),
                _ => true,
            };
            if button(title, r, bold, mouse, enabled, i == 4 && self.inspector) {
                match i {
                    0 => {
                        let result = booth.add_or_restore();
                        self.result(result, "Independent role added");
                    }
                    1 => booth.remove(),
                    2 => self.preview = true,
                    3 => {
                        self.scene_picker = true;
                        self.picker_drag.cancel();
                    }
                    4 => self.inspector = !self.inspector,
                    _ => {}
                }
            }
        }
        let mut deferred = None;
        let row_y = size.y - 34.0;
        let total = booth.active_count();
        if total > 0 {
            for (i, title) in ["<", ">"].iter().enumerate() {
                if button(
                    title,
                    Rect::new(12.0 + i as f32 * 36.0, row_y, 32.0, 28.0),
                    font,
                    mouse,
                    true,
                    false,
                ) {
                    let current = booth.selected.unwrap_or(0);
                    booth.selected = Some((current + if i == 0 { total - 1 } else { 1 }) % total);
                }
            }
            let selected = booth.selected.unwrap_or(0);
            label(
                &format!(
                    "Role {} / {} · {}",
                    selected + 1,
                    total,
                    booth.characters[selected].name()
                ),
                92.0,
                row_y + 19.0,
                12,
                font,
                WHITE,
                (size.x * 0.48 - 92.0).max(1.0),
            );
        } else {
            label(
                "No roles · tap + Role to add",
                12.0,
                row_y + 19.0,
                12,
                font,
                GRAY,
                size.x * 0.5,
            );
        }
        // Small windows wrap output controls into the toolbar's second row.
        let right_w = if narrow { 59.0 } else { 86.0 };
        let output_y = if narrow { 56.0 } else { 9.0 };
        let output_right = if narrow {
            size.x - 12.0
        } else {
            size.x - 106.0
        };
        for (i, title) in ["Save", "Load", "Photo"].iter().enumerate() {
            let r = Rect::new(
                output_right - (3 - i) as f32 * right_w,
                output_y,
                right_w - 4.0,
                34.0,
            );
            let clicked = if narrow {
                button(title, r, bold, mouse, true, false)
            } else {
                let clicked = button("", r, bold, mouse, true, false);
                let text = measure_text(title, Some(bold), 12, 1.0);
                draw_text_ex(
                    title,
                    r.x + (r.w - text.width) * 0.5,
                    r.y + (r.h - text.height) * 0.5 + text.offset_y,
                    TextParams {
                        font: Some(bold),
                        font_size: 12,
                        color: LIGHTGRAY,
                        ..Default::default()
                    },
                );
                clicked
            };
            if clicked {
                deferred = Some(i);
            }
        }
        if !self.status.is_empty() {
            label(
                &self.status,
                118.0,
                80.0,
                11,
                font,
                ACCENT,
                (if narrow {
                    output_right - 3.0 * right_w
                } else {
                    self.rects().0.right()
                } - 130.0)
                    .max(1.0),
            );
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
        self.color_picker.draw(booth, font, bold, blocked);
        if self.scene_picker || self.scene_transition.visible() {
            self.draw_scene_picker(
                booth,
                font,
                bold,
                if blocked || self.native_input.active() {
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
                if blocked || self.native_input.active() {
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
            if blocked || self.native_input.active() {
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
                    let r = booth.save();
                    self.result(r, "Saved characters.json + scene.json");
                }
                1 => {
                    let r = booth.load();
                    self.result(r, "Scene restored");
                }
                2 => {
                    let r = booth.capture(1920, 1080);
                    if let Some(path) = self.result(r, "") {
                        self.status = format!("Photo saved: {}", path.display());
                    }
                }
                _ => {}
            }
            super::layout::begin();
        }
    }
    pub fn preview_close(&mut self, font: &Font, blocked: bool) {
        let size = super::layout::size();
        let r = if super::layout::compact() {
            Rect::new((size.x - 242.0).max(0.0), 10.0, 158.0, 32.0)
        } else {
            Rect::new((size.x - 174.0).max(0.0), size.y - 40.0, 158.0, 32.0)
        };
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
        let text = measure_text("Back to booth", Some(font), 13, 1.0);
        label(
            "Back to booth",
            r.x + (r.w - text.width) * 0.5,
            r.y + (r.h - text.height) * 0.5 + text.offset_y,
            13,
            font,
            if r.contains(mouse) {
                Color::from_rgba(236, 154, 92, 255)
            } else {
                Color::new(0.90, 0.91, 0.94, 1.0)
            },
            r.w - 24.0,
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
        for (i, title) in ["Transform", "Animation", "Scene"].iter().enumerate() {
            if button(
                title,
                Rect::new(
                    panel.x + 8.0 + i as f32 * (panel.w - 16.0) / 3.0,
                    panel.y + 8.0,
                    (panel.w - 16.0) / 3.0 - 3.0,
                    34.0,
                ),
                font,
                mouse,
                true,
                self.tab == i,
            ) {
                self.tab = i;
                self.scroll = 0.0;
                self.scroll_drag.cancel();
                self.slider = None;
                self.slider_start = None;
                self.content_tap = None;
            }
        }
        let area = Rect::new(
            panel.x + 8.0,
            panel.y + 50.0,
            panel.w - 16.0,
            (panel.h - 58.0).max(1.0),
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
            label("SCENE & OUTPUT", x, y + 19.0, 12, bold, GRAY, w);
            y += 32.0;
            let name = booth.backgrounds[booth.background]
                .img
                .split('/')
                .nth(1)
                .unwrap_or("Color");
            if button(
                &format!("Background: {name}"),
                Rect::new(x, y, w, 36.0),
                font,
                hit_mouse,
                true,
                false,
            ) {
                self.scene_picker = true;
            }
            y += 46.0;
            if button(
                match booth.watermark {
                    Watermark::Hidden => "Watermark: hidden",
                    Watermark::Left => "Watermark: bottom left",
                    Watermark::Right => "Watermark: bottom right",
                },
                Rect::new(x, y, w, 36.0),
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
            y += 46.0;
            if button(
                if booth.transparent {
                    "Transparent PNG: on"
                } else {
                    "Transparent PNG: off"
                },
                Rect::new(x, y, w, 36.0),
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
            label("Photo: 1920 × 1080 · excludes GUI", x, y, 11, font, GRAY, w);
            y += 28.0;
            if button(
                "Save folder…",
                Rect::new(x, y, w, 36.0),
                font,
                hit_mouse,
                true,
                false,
            ) {
                self.input_kind = Some(InputKind::SaveDirectory);
                self.native_input.open_with(
                    "Save folder",
                    "Folder for characters.json and scene.json",
                    &booth.save_directory.to_string_lossy(),
                );
            }
            y += 58.0;
            label(
                &booth.save_directory.to_string_lossy(),
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
                y += 30.0;
                let tint = Rect::new(x, y, w, 36.0);
                if button("Color Tint", tint, font, hit_mouse, true, false) {
                    self.color_picker.open(c.id.clone(), c.placement.color);
                }
                draw_rectangle(
                    x + w - 34.0,
                    y + 5.0,
                    26.0,
                    26.0,
                    Color::new(c.placement.color[0], c.placement.color[1], c.placement.color[2], c.placement.color[3]),
                );
                y += 48.0;
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
                    Rect::new(x, y, w, 36.0),
                    font,
                    hit_mouse,
                    true,
                    !p.face_right,
                ) {
                    p.face_right = !p.face_right;
                }
                y += 44.0;
                if button(
                    if p.pet_face_right {
                        "Pet mirror: off"
                    } else {
                        "Pet mirror: on"
                    },
                    Rect::new(x, y, w, 36.0),
                    font,
                    hit_mouse,
                    true,
                    !p.pet_face_right,
                ) {
                    p.pet_face_right = !p.pet_face_right;
                }
                y += 44.0;
                let scale = perspective(&background, p.position[1]);
                if button(
                    if p.pet_reference == ReferenceFrame::Character {
                        "Pet frame: character"
                    } else {
                        "Pet frame: world"
                    },
                    Rect::new(x, y, w, 36.0),
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
                y += 44.0;
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
                    Rect::new(x, y, w, 36.0),
                    font,
                    hit_mouse,
                    true,
                    false,
                ) {
                    self.clip_picker = true;
                }
                y += 44.0;
                if button(
                    if c.animation.paused { "Play" } else { "Pause" },
                    Rect::new(x, y, w, 36.0),
                    font,
                    hit_mouse,
                    true,
                    c.animation.paused,
                ) {
                    c.animation.paused = !c.animation.paused;
                }
                y += 44.0;
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
                    Rect::new(x, y, w, 36.0),
                    font,
                    hit_mouse,
                    true,
                    t.looping,
                ) {
                    t.looping = !t.looping;
                }
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
        }
        self.inspector_heights[self.tab] = (y + self.scroll - area.y + 12.0).max(0.0);
        self.scroll = self.scroll.clamp(0.0, (self.inspector_heights[self.tab] - area.h).max(0.0));
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
        let w = (size.x - 32.0).min(420.0);
        let h = (size.y - 56.0).max(1.0);
        let x = (size.x - w) * 0.5;
        let panel = self
            .scene_transition
            .panel(self.scene_picker, Rect::new(x, 28.0, w, h));
        let y = panel.y;
        self.scene_transition.backdrop();
        let mut mouse = self.scene_transition.mouse(self.scene_picker, mouse);
        let _opacity = self.scene_transition.scope();
        draw_rectangle(x, y, w, h, super::widgets::SURFACE);
        let close = super::widgets::popup_header(panel, "Booth scenes", font, bold);
        if super::widgets::dismiss(panel, close, mouse) {
            self.scene_picker = false;
            self.picker_drag.cancel();
            mouse = super::widgets::BLOCKED;
        }
        let area = Rect::new(x + 12.0, y + 50.0, w - 24.0, (h - 62.0).max(1.0));
        let max = (booth.backgrounds.len() as f32 * 42.0 - area.h).max(0.0);
        let tap = super::widgets::scroll(
            area,
            mouse,
            &mut self.picker_scroll,
            max,
            &mut self.picker_drag,
        );
        let mut chosen = None;
        super::layout::clip(area);
        for (i, bg) in booth.backgrounds.iter().enumerate() {
            let r = Rect::new(
                area.x,
                area.y + i as f32 * 42.0 - self.picker_scroll,
                area.w,
                38.0,
            );
            if r.bottom() <= area.y || r.y >= area.bottom() {
                continue;
            }
            let name = if i == 0 {
                "Green screen"
            } else {
                bg.img.split('/').nth(1).unwrap_or(&bg.img)
            };
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if i == booth.background {
                    Color::from_rgba(53, 44, 41, 255)
                } else {
                    CONTROL
                },
            );
            label(
                name,
                r.x + 10.0,
                r.y + 25.0,
                14,
                font,
                if i == booth.background { ACCENT } else { WHITE },
                r.w - 20.0,
            );
            if tap && area.contains(mouse) && r.contains(mouse) {
                chosen = Some(i);
            }
        }
        super::layout::end_clip();
        if let Some(index) = chosen {
            booth.switch_background(index);
            self.scene_picker = false;
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
    label(
        &format!("{title}  {value:.2}"),
        x,
        *y + 15.0,
        11,
        font,
        GRAY,
        w,
    );
    let hit = Rect::new(x, *y + 18.0, w, 26.0);
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
            min + ((mouse.x - x) / w).clamp(0.0, 1.0) * (max - min)
        };
        changed = true;
    }
    let t = ((*value - min) / (max - min).max(0.00001)).clamp(0.0, 1.0);
    draw_rectangle(x, *y + 30.0, w, 2.0, CONTROL);
    draw_rectangle(x, *y + 30.0, w * t, 2.0, ACCENT);
    draw_rectangle(x + w * t - 3.0, *y + 24.0, 6.0, 14.0, ACCENT);
    *y += 49.0;
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
            let (canvas, panel) = regions(size, true);
            assert!(!canvas.overlaps(&panel));
            let equipment = super::super::equipment::EquipmentPicker::bar(canvas);
            assert!(!equipment.overlaps(&canvas));
            assert!(!equipment.overlaps(&panel));
            let (full_canvas, _) = regions(size, false);
            assert!(!equipment.overlaps(&full_canvas));
            assert!((canvas.w / canvas.h - 16.0 / 9.0).abs() < 0.001);
            for rect in [canvas, panel] {
                assert!(rect.w > 0.0 && rect.h > 0.0);
                assert!(rect.x >= 0.0 && rect.y >= 50.0);
                assert!(rect.right() <= size.x && rect.bottom() <= size.y - 40.0);
            }
        }
    }
}
