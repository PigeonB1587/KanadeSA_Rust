//! Adapted from phira/prpr/src/ui/billboard.rs (GPL-3.0).
//! Fixed-color, non-modal messages in logical pixels, above all GUI layers.
//! See THIRD_PARTY_NOTICES.md. No scene/UI framework or texture dependency.
use macroquad::prelude::*;
use std::{cell::RefCell, collections::VecDeque};

const MOTION_TIME: f64 = 0.8;
const MAX_MESSAGES: usize = 4;
const PADDING: f32 = 12.0;
const ICON_WIDTH: f32 = 38.0;
const FONT_SIZE: u16 = 13;
const LINE_HEIGHT: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Warn,
    Success,
    Error,
}
impl Kind {
    fn color(self) -> Color {
        match self {
            Self::Info => Color::new(0.16, 0.71, 0.96, 1.0),
            Self::Warn => Color::new(1.0, 0.66, 0.15, 1.0),
            Self::Success => Color::new(0.4, 0.73, 0.42, 1.0),
            Self::Error => Color::new(0.96, 0.26, 0.21, 1.0),
        }
    }
    fn duration(self) -> f64 {
        match self {
            Self::Info | Self::Success => 3.0,
            Self::Warn => 4.0,
            Self::Error => 6.0,
        }
    }
}

struct Message {
    content: String,
    kind: Kind,
    time: f64,
    position: f32,
}

#[derive(Default)]
struct Billboard {
    messages: VecDeque<Message>,
    // Keep a small history as per-frame errors must not reappear every frame
    // or continually reset the message's fade-out timer.
    recent: VecDeque<(Kind, String, f64)>,
    last_time: Option<f64>,
}
thread_local! {
    static BILLBOARD: RefCell<Billboard> = RefCell::new(Billboard::default());
}

pub fn show(content: impl Into<String>, kind: Kind) {
    let content = content.into();
    if content.trim().is_empty() {
        return;
    }
    let now = get_time();
    BILLBOARD.with(|board| {
        let mut board = board.borrow_mut();
        board.recent.retain(|(_, _, time)| now - time < 10.0);
        let cooldown = if kind == Kind::Error { 10.0 } else { 0.8 };
        if board
            .recent
            .iter()
            .any(|(k, text, time)| *k == kind && text == &content && now - time < cooldown)
        {
            return;
        }
        if kind == Kind::Error {
            tracing::warn!("{content}");
        }
        if board.recent.len() >= 32 {
            board.recent.pop_front();
        }
        board.recent.push_back((kind, content.clone(), now));
        if board.messages.len() >= MAX_MESSAGES {
            board.messages.pop_front();
        }
        let position = board.messages.len() as f32 * 64.0;
        board.messages.push_back(Message {
            content,
            kind,
            time: now,
            position,
        });
    });
}

pub fn info(content: impl Into<String>) {
    show(content, Kind::Info);
}
pub fn warn(content: impl Into<String>) {
    show(content, Kind::Warn);
}
pub fn success(content: impl Into<String>) {
    show(content, Kind::Success);
}
pub fn error(content: impl Into<String>) {
    show(content, Kind::Error);
}

fn wrap(content: &str, font: &Font, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for ch in content.chars() {
        if ch == '\r' {
            continue;
        }
        if ch == '\n'
            || measure_text(&format!("{line}{ch}"), Some(font), FONT_SIZE, 1.0).width > width
        {
            lines.push(std::mem::take(&mut line));
            if lines.len() == 3 {
                let last = lines.last_mut().unwrap();
                while !last.is_empty()
                    && measure_text(&format!("{last}…"), Some(font), FONT_SIZE, 1.0).width > width
                {
                    last.pop();
                }
                last.push('…');
                return lines;
            }
        }
        if ch != '\n' {
            line.push(ch);
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// Draw once after Settings/Preview/Info; deliberately bypass theme/opacity
/// wrappers, so neither modal fades nor Day/Dark can recolor these messages.
pub fn draw(font: &Font) {
    let size = super::layout::size();
    let now = get_time();
    BILLBOARD.with(|board| {
        let mut board = board.borrow_mut();
        let dt = board.last_time.map_or(0.0, |last| (now - last).max(0.0)) as f32;
        board.last_time = Some(now);
        board
            .messages
            .retain(|msg| now < msg.time + msg.kind.duration() + MOTION_TIME);
        let width = (size.x - PADDING * 2.0).min(420.0).max(1.0);
        let text_width = (width - ICON_WIDTH - PADDING * 2.0).max(1.0);
        let mut target = 0.0;
        let top = super::layout::TOPBAR_HEIGHT + PADDING;
        super::layout::clip(Rect::new(
            0.0,
            top,
            size.x,
            (size.y - top - PADDING).max(1.0),
        ));
        for msg in &mut board.messages {
            let lines = wrap(&msg.content, font, text_width);
            let height = (lines.len() as f32 * LINE_HEIGHT + PADDING * 2.0).max(48.0);
            let elapsed = now - msg.time;
            let exit = ((elapsed - msg.kind.duration()) / MOTION_TIME).clamp(0.0, 1.0) as f32;
            let enter = (elapsed / MOTION_TIME).clamp(0.0, 1.0) as f32;
            let travel = (1.0 - enter).powi(3) + 1.0 - (1.0 - exit).powi(3);
            let alpha = 1.0 - exit;
            let x = size.x - PADDING - width + (width + PADDING) * travel;
            msg.position += (target - msg.position) * (1.0 - 0.5_f32.powf(dt / 0.1));
            let y = top + msg.position;
            if exit == 0.0 {
                target += height + 8.0;
            }
            let mut color = msg.kind.color();
            color.a = alpha;
            draw_rectangle(x, y, width, height, color);
            draw_rectangle(
                x,
                y,
                ICON_WIDTH,
                height,
                Color::new(1.0, 1.0, 1.0, 0.4 * alpha),
            );
            if elapsed < msg.kind.duration() {
                let remaining = 1.0 - (elapsed / msg.kind.duration()) as f32;
                draw_rectangle(
                    x,
                    y + height - 3.0,
                    width * remaining,
                    3.0,
                    Color::new(1.0, 1.0, 1.0, 0.3 * alpha),
                );
            }
            let white = Color::new(1.0, 1.0, 1.0, alpha);
            draw_icon(
                msg.kind,
                vec2(x + ICON_WIDTH * 0.5, y + height * 0.5),
                white,
                font,
            );
            for (i, line) in lines.iter().enumerate() {
                draw_text_ex(
                    line,
                    x + ICON_WIDTH + PADDING,
                    y + PADDING + 13.0 + i as f32 * LINE_HEIGHT,
                    TextParams {
                        font: Some(font),
                        font_size: FONT_SIZE,
                        color: white,
                        ..Default::default()
                    },
                );
            }
        }
        super::layout::end_clip();
    });
}

fn draw_icon(kind: Kind, center: Vec2, color: Color, font: &Font) {
    let (x, y) = (center.x, center.y);
    match kind {
        Kind::Success => {
            draw_line(x - 8.0, y, x - 2.0, y + 6.0, 2.5, color);
            draw_line(x - 2.0, y + 6.0, x + 9.0, y - 6.0, 2.5, color);
        }
        Kind::Error => {
            draw_line(x - 6.0, y - 6.0, x + 6.0, y + 6.0, 2.5, color);
            draw_line(x + 6.0, y - 6.0, x - 6.0, y + 6.0, 2.5, color);
        }
        Kind::Info | Kind::Warn => {
            let symbol = if kind == Kind::Info { "i" } else { "!" };
            let text = measure_text(symbol, Some(font), 23, 1.0);
            draw_text_ex(
                symbol,
                x - text.width * 0.5,
                y - text.height * 0.5 + text.offset_y,
                TextParams {
                    font: Some(font),
                    font_size: 23,
                    color,
                    ..Default::default()
                },
            );
        }
    }
}
