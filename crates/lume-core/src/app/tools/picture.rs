//! The Picture tool (`display-diagnostics`): overscan and geometry, and motion.
//!
//! Both are drawn in device pixels like the Screen tool's patterns (D6). The overscan pattern is
//! static: a frame for its hint and none after. The motion test asks for continuous frames while it
//! is open (D7) and counts the intervals between frames of 33 ms or more, the ruler of
//! `docs/device-notes.md`.

use super::common::Hint;
use super::display::{draw_description, hline, outline, vline, BLACK, GRAY_50};
use super::{ToolEvent, ToolKey, ToolKind};
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp, PendingFocus};
use crate::i18n::{Arg, StringId};
use crate::render::{PatternOp, PixelRect, Rect};
use crate::text::Align;
use crate::theme::{vars, Color, StateFlags, THEME};
use crate::ui::animation::FRAME_MS;
use crate::ui::focus::FocusId;

/// The overscan marks, as a share of the width and height from each edge.
const MARK_NEAR: f32 = 0.025;
const MARK_FAR: f32 = 0.05;
/// The color of the 2.5% marks (yellow) and of the 5% ones (blue), named in the description.
const MARK_NEAR_COLOR: Color = Color::rgb(255, 212, 0);
const MARK_FAR_COLOR: Color = Color::rgb(77, 179, 255);
/// The radius of the center circle, as a share of the height; the square around it has the
/// circle's diameter as its side.
const CIRCLE: f32 = 0.3;

/// The motion speeds, in screen widths per second: slow, medium and fast.
pub const SPEEDS: [f32; 3] = [0.125, 0.25, 0.5];
const MEDIUM: usize = 1;
/// The moving block's side, as a share of the height, and where its top is.
const BLOCK: f32 = 0.125;
const BLOCK_TOP: f32 = 0.25;
/// A frame that took this long or more is a dropped frame.
pub const DROPPED_MS: u64 = 33;
/// Where the moving text and the status line are, in canvas units.
const MOTION_TEXT_TOP: f32 = 432.0;
const MOTION_TEXT_HEIGHT: f32 = 64.0;
const STATUS_TOP: f32 = 32.0;
const STATUS_HEIGHT: f32 = 32.0;
const STATUS_LEFT: f32 = 32.0;

/// The focus ids of the tool, from [`ToolKind::first_id`].
const MODE_OVERSCAN: FocusId = 0;
const MODE_MOTION: FocusId = 1;

fn id(offset: FocusId) -> FocusId {
    ToolKind::Picture.first_id() + offset
}

/// The motion test's state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Motion {
    /// Index in [`SPEEDS`].
    speed: usize,
    /// Where the block was (device pixels traveled) at `since`, when the speed last changed.
    origin: f64,
    since: u64,
    /// When the last frame was drawn, and how many intervals were of 33 ms or more.
    last_frame: Option<u64>,
    dropped: u32,
}

impl Motion {
    fn new(now_ms: u64) -> Motion {
        Motion {
            speed: MEDIUM,
            origin: 0.0,
            since: now_ms,
            last_frame: None,
            dropped: 0,
        }
    }

    /// Device pixels traveled at `now_ms` on a screen `width` wide: constant speed, whatever the
    /// frames.
    fn traveled(&self, now_ms: u64, width: i32) -> f64 {
        let speed = f64::from(SPEEDS[self.speed]) * f64::from(width);
        self.origin + speed * now_ms.saturating_sub(self.since) as f64 / 1000.0
    }

    /// Another speed from now on, without a jump.
    fn set_speed(&mut self, speed: usize, now_ms: u64, width: i32) {
        self.origin = self.traveled(now_ms, width);
        self.since = now_ms;
        self.speed = speed;
    }

    /// A frame drawn at `now_ms`: counts it when it came late.
    fn frame(&mut self, now_ms: u64) {
        if let Some(last) = self.last_frame {
            if now_ms > last && now_ms - last >= DROPPED_MS {
                self.dropped += 1;
            }
        }
        self.last_frame = Some(now_ms);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Modes,
    Overscan,
    Motion(Motion),
}

/// The tool's state, alive only while it is open.
#[derive(Debug)]
pub struct PictureTool {
    phase: Phase,
    hint: Hint,
    /// The width of the render buffer, from the last frame.
    width: i32,
}

impl PictureTool {
    pub(in crate::app) fn new(_now_ms: u64) -> PictureTool {
        PictureTool {
            phase: Phase::Modes,
            hint: Hint::default(),
            width: 1920,
        }
    }
}

/// The overscan and geometry pattern on a buffer of `width` x `height` device pixels: a 1 px border
/// on the exact edge, the 2.5% and 5% marks, the diagonals, and a circle in a square in the middle.
pub fn overscan_ops(width: i32, height: i32) -> Vec<PatternOp> {
    let mut ops = vec![PatternOp::Fill(BLACK)];
    let inset = |share: f32| {
        let (dx, dy) = (
            (width as f32 * share).round() as i32,
            (height as f32 * share).round() as i32,
        );
        PixelRect::new(dx, dy, width - 2 * dx, height - 2 * dy)
    };
    ops.push(PatternOp::Line {
        from: (0, 0),
        to: (width - 1, height - 1),
        color: GRAY_50,
    });
    ops.push(PatternOp::Line {
        from: (width - 1, 0),
        to: (0, height - 1),
        color: GRAY_50,
    });
    ops.extend(outline(inset(MARK_FAR), MARK_FAR_COLOR));
    ops.extend(outline(inset(MARK_NEAR), MARK_NEAR_COLOR));
    ops.extend(outline(PixelRect::new(0, 0, width, height), Color::WHITE));
    // The center lines, so the middle of the panel is plain to see.
    let (cx, cy) = (width / 2, height / 2);
    ops.push(hline(0, cy, width, GRAY_50));
    ops.push(vline(cx, 0, height, GRAY_50));
    let radius = (height as f32 * CIRCLE).round() as i32;
    ops.extend(outline(
        PixelRect::new(cx - radius, cy - radius, 2 * radius + 1, 2 * radius + 1),
        Color::WHITE,
    ));
    ops.push(PatternOp::Circle {
        center: (cx, cy),
        radius,
        color: Color::WHITE,
    });
    ops
}

impl LumeApp {
    fn picture_state(&mut self) -> Option<&mut PictureTool> {
        match self.tool.as_deref_mut() {
            Some(super::ToolState::Picture(s)) => Some(s),
            _ => None,
        }
    }

    fn picture_ref(&self) -> Option<&PictureTool> {
        match self.tool.as_deref() {
            Some(super::ToolState::Picture(s)) => Some(s),
            _ => None,
        }
    }

    /// Opening the tool (its state was just created): effects for the platform.
    pub(in crate::app) fn picture_open(&mut self) -> Vec<Effect> {
        vec![]
    }

    pub(in crate::app) fn draw_picture(&mut self, p: &mut Painter) {
        let now = self.now_ms;
        let t = self.translator;
        let Some(s) = self.picture_state() else {
            return;
        };
        let (width, height) = (p.display.width as i32, p.display.height as i32);
        s.width = width;
        match &mut s.phase {
            Phase::Modes => {
                let items: Vec<(FocusId, String)> = [
                    (MODE_OVERSCAN, StringId::ToolPictureOverscan),
                    (MODE_MOTION, StringId::ToolPictureMotion),
                ]
                .into_iter()
                .map(|(item, name)| (id(item), t.get(name).into_owned()))
                .collect();
                self.draw_tool_items(p, &items, 0.0);
                let motion = self.focus.current_id() == Some(id(MODE_MOTION));
                let (name, description) = if motion {
                    (StringId::ToolPictureMotion, StringId::ToolPictureMotionDesc)
                } else {
                    (
                        StringId::ToolPictureOverscan,
                        StringId::ToolPictureOverscanDesc,
                    )
                };
                // The descriptions have "%%" for a percent sign: `format` makes it one.
                draw_description(p, &t.get(name), &t.format(description, &[]));
                self.draw_tool_header(p, ToolKind::Picture);
                return;
            }
            Phase::Overscan => {
                for op in overscan_ops(width, height) {
                    p.list.pattern(op);
                }
            }
            Phase::Motion(motion) => {
                motion.frame(now);
                let traveled = motion.traveled(now, width);
                let block = (height as f32 * BLOCK).round() as i32;
                let span = f64::from(width + block);
                let x = traveled.rem_euclid(span) as i32 - block;
                p.list.pattern(PatternOp::Fill(BLACK));
                p.list.pattern(PatternOp::Rect(
                    PixelRect::new(x, (height as f32 * BLOCK_TOP).round() as i32, block, block),
                    Color::WHITE,
                ));

                // The text travels at the same speed, half a screen ahead of the block.
                let mut style = p.style(&THEME.page_title, StateFlags::NONE);
                style.color = Color::WHITE;
                let sample = t.get(StringId::ToolPictureMotionSample);
                let scale = f64::from(p.display.scale);
                let text_w = f64::from(p.measure(&sample, &style)) * scale;
                let span = f64::from(width) + text_w;
                let x_dev = (traveled + f64::from(width) / 2.0).rem_euclid(span) - text_w;
                let x = ((x_dev - f64::from(p.display.offset_x)) / scale) as f32;
                p.text(
                    &sample,
                    Rect::new(x, MOTION_TEXT_TOP, text_w as f32 + 8.0, MOTION_TEXT_HEIGHT),
                    Align::Start,
                    &style,
                );

                let speed = t.get(match motion.speed {
                    0 => StringId::ToolPictureSpeedSlow,
                    1 => StringId::ToolPictureSpeedMedium,
                    _ => StringId::ToolPictureSpeedFast,
                });
                let status = t.format(
                    StringId::ToolPictureMotionStatus,
                    &[Arg::Str(&speed), Arg::Int(i64::from(motion.dropped))],
                );
                let mut status_style = p.style(&THEME.breadcrumb, StateFlags::NONE);
                status_style.color = vars::TEXT;
                let visible = p.display.visible_rect();
                p.text(
                    &status,
                    Rect::new(
                        visible.x + STATUS_LEFT,
                        visible.y + STATUS_TOP,
                        visible.w,
                        STATUS_HEIGHT,
                    ),
                    Align::Start,
                    &status_style,
                );
            }
        }
        s.hint.draw(p, now);
    }

    pub(in crate::app) fn picture_key(&mut self, key: Key) -> ToolKey {
        let now = self.now_ms;
        let t = self.translator;
        let focused = self.focus.current_id();
        let Some(s) = self.picture_state() else {
            return ToolKey::Default;
        };
        let mut focus_on = None;
        match &mut s.phase {
            Phase::Modes => match key {
                Key::Ok if focused == Some(id(MODE_OVERSCAN)) => {
                    s.phase = Phase::Overscan;
                    s.hint
                        .show(t.format(StringId::ToolPictureOverscanHint, &[]), now);
                }
                Key::Ok if focused == Some(id(MODE_MOTION)) => {
                    s.phase = Phase::Motion(Motion::new(now));
                }
                _ => return ToolKey::Default,
            },
            Phase::Overscan => {
                if key == Key::Back {
                    s.phase = Phase::Modes;
                    s.hint.hide();
                    focus_on = Some(MODE_OVERSCAN);
                } else {
                    s.hint
                        .show(t.format(StringId::ToolPictureOverscanHint, &[]), now);
                }
            }
            Phase::Motion(motion) => match key {
                Key::Back => {
                    s.phase = Phase::Modes;
                    focus_on = Some(MODE_MOTION);
                }
                Key::Up => {
                    let speed = (motion.speed + 1).min(SPEEDS.len() - 1);
                    motion.set_speed(speed, now, s.width);
                }
                Key::Down => {
                    let speed = motion.speed.saturating_sub(1);
                    motion.set_speed(speed, now, s.width);
                }
                _ => {}
            },
        }
        if let Some(item) = focus_on {
            self.pending_focus = Some(PendingFocus::Id(id(item)));
            self.focus.set_rows(vec![]);
        }
        ToolKey::Handled(vec![])
    }

    pub(in crate::app) fn picture_deadline(&self, _now_ms: u64) -> Option<u64> {
        let s = self.picture_ref()?;
        match s.phase {
            Phase::Motion(_) => Some(self.now_ms + FRAME_MS),
            _ => s.hint.deadline(self.now_ms),
        }
    }

    pub(in crate::app) fn picture_fullscreen(&self) -> bool {
        self.picture_ref().is_some_and(|s| s.phase != Phase::Modes)
    }

    pub(in crate::app) fn picture_event(&mut self, _event: ToolEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};

    fn state(rig: &Rig) -> &PictureTool {
        rig.app.picture_ref().expect("the Picture tool is open")
    }

    fn motion(rig: &Rig) -> Motion {
        match state(rig).phase {
            Phase::Motion(m) => m,
            other => panic!("{other:?}"),
        }
    }

    fn open_motion(rig: &mut Rig) {
        open_tool(rig, ToolKind::Picture);
        rig.press(Key::Down);
        rig.press(Key::Ok);
        assert!(matches!(state(rig).phase, Phase::Motion(_)));
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("MOTION_TEXT_TOP", MOTION_TEXT_TOP),
            ("MOTION_TEXT_HEIGHT", MOTION_TEXT_HEIGHT),
            ("STATUS_TOP", STATUS_TOP),
            ("STATUS_HEIGHT", STATUS_HEIGHT),
            ("STATUS_LEFT", STATUS_LEFT),
        ]);
    }

    #[test]
    fn the_overscan_marks_are_where_the_spec_says() {
        let ops = overscan_ops(1920, 1080);
        let rects: Vec<(PixelRect, Color)> = ops
            .iter()
            .filter_map(|op| match op {
                PatternOp::Rect(r, c) => Some((*r, *c)),
                _ => None,
            })
            .collect();
        // Every rectangle is a line of 1 px.
        assert!(rects.iter().all(|(r, _)| r.w == 1 || r.h == 1));
        // The border on the exact edge.
        assert!(rects.contains(&(PixelRect::new(0, 0, 1920, 1), Color::WHITE)));
        assert!(rects.contains(&(PixelRect::new(1919, 0, 1, 1080), Color::WHITE)));
        assert!(rects.contains(&(PixelRect::new(0, 1079, 1920, 1), Color::WHITE)));
        // 2.5% is 48 px of 1920 and 27 of 1080; 5% is 96 and 54.
        assert!(rects.contains(&(PixelRect::new(48, 27, 1824, 1), MARK_NEAR_COLOR)));
        assert!(rects.contains(&(PixelRect::new(96, 54, 1, 972), MARK_FAR_COLOR)));
        // The circle and the square have the same center and size: round and square on a
        // panel with square pixels.
        assert!(ops.contains(&PatternOp::Circle {
            center: (960, 540),
            radius: 324,
            color: Color::WHITE
        }));
        assert!(rects.contains(&(PixelRect::new(636, 216, 649, 1), Color::WHITE)));
        assert!(rects.contains(&(PixelRect::new(636, 216, 1, 649), Color::WHITE)));
        // Both diagonals.
        assert_eq!(
            ops.iter()
                .filter(|op| matches!(op, PatternOp::Line { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn the_overscan_pattern_is_static_after_its_hint() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Picture);
        rig.press(Key::Ok);
        assert_eq!(state(&rig).phase, Phase::Overscan);
        assert!(rig.app.tool_fullscreen());
        assert!(rig.app.picture_deadline(rig.now).is_some());
        rig.advance(3000);
        assert_eq!(rig.app.picture_deadline(rig.now), None);
        rig.press(Key::Back);
        assert_eq!(state(&rig).phase, Phase::Modes);
        assert_eq!(rig.focused(), Some(id(MODE_OVERSCAN)));
    }

    #[test]
    fn a_frame_of_50_ms_counts_as_dropped() {
        let mut rig = Rig::new();
        open_motion(&mut rig);
        assert_eq!(motion(&rig).dropped, 0);
        for _ in 0..10 {
            rig.advance(16);
        }
        rig.advance(17);
        assert_eq!(motion(&rig).dropped, 0);
        rig.advance(50);
        assert_eq!(motion(&rig).dropped, 1);
        rig.advance(33);
        assert_eq!(motion(&rig).dropped, 2);
        rig.advance(32);
        assert_eq!(motion(&rig).dropped, 2);
    }

    #[test]
    fn up_and_down_change_the_speed_without_a_jump() {
        let mut rig = Rig::new();
        open_motion(&mut rig);
        assert_eq!(motion(&rig).speed, MEDIUM);
        rig.advance(1000);
        let before = motion(&rig).traveled(rig.now, 1280);
        rig.app.handle_key(Key::Up, rig.now);
        assert_eq!(motion(&rig).speed, 2, "fast");
        assert!((motion(&rig).traveled(rig.now, 1280) - before).abs() < 1e-6);
        // Fast: half a width per second.
        let at = motion(&rig).traveled(rig.now + 1000, 1280);
        assert!((at - before - 640.0).abs() < 1e-6);
        rig.press_all(&[Key::Up, Key::Down, Key::Down, Key::Down]);
        assert_eq!(motion(&rig).speed, 0, "slow");
    }

    #[test]
    fn frames_flow_only_while_the_motion_test_is_open() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Picture);
        assert_eq!(rig.app.picture_deadline(rig.now), None);
        rig.press(Key::Down);
        rig.press(Key::Ok);
        assert_eq!(rig.app.picture_deadline(rig.now), Some(rig.now + FRAME_MS));
        rig.advance(10_000);
        assert_eq!(rig.app.picture_deadline(rig.now), Some(rig.now + FRAME_MS));
        rig.press(Key::Back);
        assert_eq!(rig.app.picture_deadline(rig.now), None);
        assert_eq!(rig.focused(), Some(id(MODE_MOTION)));
        assert!(!rig.app.tool_fullscreen());
    }
}
