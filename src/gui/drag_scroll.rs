//! Shared mouse/touch drag recognizer. A tap is committed only on release.
use macroquad::prelude::*;

#[derive(Default)]
pub struct DragScroll {
    start: Option<Vec2>,
    previous: Vec2,
    moved: bool,
}
impl DragScroll {
    pub fn cancel(&mut self) {
        self.start = None;
        self.moved = false;
    }
    pub fn update(
        &mut self,
        area: Rect,
        position: Vec2,
        pressed: bool,
        down: bool,
        released: bool,
        offset: &mut f32,
        max: f32,
    ) -> bool {
        if pressed && area.contains(position) {
            self.start = Some(position);
            self.previous = position;
            self.moved = false;
        }
        if let Some(start) = self.start {
            if down || released {
                if position.distance(start) > 7.0 {
                    self.moved = true;
                }
                if self.moved {
                    *offset = (*offset + self.previous.y - position.y).clamp(0.0, max);
                }
                self.previous = position;
            }
            if released {
                let tap = !self.moved && area.contains(position);
                self.cancel();
                return tap;
            }
            if !down {
                self.cancel();
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_does_not_select_and_tap_selects_on_release() {
        let mut drag = DragScroll::default();
        let mut offset = 0.0;
        let area = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(!drag.update(
            area,
            vec2(20.0, 60.0),
            true,
            true,
            false,
            &mut offset,
            200.0
        ));
        assert!(!drag.update(
            area,
            vec2(20.0, 20.0),
            false,
            true,
            false,
            &mut offset,
            200.0
        ));
        assert_eq!(offset, 40.0);
        assert!(!drag.update(
            area,
            vec2(20.0, 20.0),
            false,
            false,
            true,
            &mut offset,
            200.0
        ));
        drag.update(
            area,
            vec2(20.0, 20.0),
            true,
            true,
            false,
            &mut offset,
            200.0,
        );
        assert!(drag.update(
            area,
            vec2(21.0, 21.0),
            false,
            false,
            true,
            &mut offset,
            200.0
        ));
    }
}
