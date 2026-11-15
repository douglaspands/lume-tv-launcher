//! The Screen tool (`display-diagnostics`): diagnostic patterns, image retention cleaning and the
//! stuck pixel fixer.
//!
//! The patterns are drawn in device pixels with [`PatternOp`]s (D6): the frame is cleared with the
//! color of a solid pattern, 1 px lines are scissored clears, and the ramps are one dithered quad.
//! Only the modes that animate by nature ask for continuous frames (D7): the color cycle of the
//! cleaning while it fades from one color to the next, and the stuck pixel fixer.

use super::common::{
    self, draw_panel, format_remaining, panel_heading, wrapped_text, FlashWarning, Hint,
    WarningAnswer,
};
use super::{ToolEvent, ToolKey, ToolKind};
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp, PendingFocus};
use crate::i18n::{Arg, StringId, Translator};
use crate::render::{PatternOp, PixelRect, Rect};
use crate::theme::{Color, StateFlags, THEME};
use crate::ui::animation::FRAME_MS;
use crate::ui::focus::FocusId;
use crate::ui::style::mix_color;

/// The description panel at the right, as tall as the one of the Tools menu.
pub(super) const DESC_PANEL_HEIGHT: f32 = super::menu::MENU_PANEL_HEIGHT;
/// Where the description starts below the panel's inner top (under the heading).
pub(super) const DESC_TOP: f32 = 48.0;

/// The side of the stuck pixel window, in device pixels.
pub const WINDOW_SIZE: i32 = 64;
/// How far an arrow moves the window, in device pixels: a press, and a key held down.
pub const WINDOW_STEP: i32 = 16;
pub const WINDOW_STEP_HELD: i32 = 8;
/// The same arrow again within this time is a key held down (its repeats).
const HELD_MS: u64 = 250;

/// The cleaning's durations and the stuck pixel fixer's, in minutes.
pub const CLEAN_MINUTES: [u64; 3] = [10, 30, 60];
pub const FIX_MINUTES: [u64; 3] = [5, 10, 20];
/// How long each color of the "Colors" cleaning stays, and how long it takes to fade into the
/// next: at most one change every 3 s, never a flash (`diagnostic-tools`).
pub const CLEAN_HOLD_MS: u64 = 2000;
pub const CLEAN_FADE_MS: u64 = 1000;
const CLEAN_COLORS: [Color; 5] = [Color::WHITE, RED, GREEN, BLUE, BLACK];
/// The colors the stuck pixel window goes through, one per frame.
const FIX_COLORS: [Color; 5] = [RED, GREEN, BLUE, Color::WHITE, BLACK];

pub(super) const BLACK: Color = Color::rgb(0, 0, 0);
const RED: Color = Color::rgb(255, 0, 0);
const GREEN: Color = Color::rgb(0, 255, 0);
const BLUE: Color = Color::rgb(0, 0, 255);
/// Grays of 5%, 20% and 50% of white.
const GRAY_5: Color = Color::rgb(13, 13, 13);
const GRAY_20: Color = Color::rgb(51, 51, 51);
pub(super) const GRAY_50: Color = Color::rgb(128, 128, 128);
/// The grid has this many cells from top to bottom (square cells: 60 px at 1080, 40 at 720).
const GRID_ROWS: i32 = 18;

/// The focus ids of the tool, from [`ToolKind::first_id`].
const MODE_DIAGNOSTIC: FocusId = 0;
const MODE_CLEAN: FocusId = 1;
const MODE_FIX: FocusId = 2;
const CLEAN_DURATION: FocusId = 10;
const CLEAN_PATTERN: FocusId = 11;
const CLEAN_START: FocusId = 12;
const FIX_DURATION: FocusId = 20;
const FIX_PLACE: FocusId = 21;

fn id(offset: FocusId) -> FocusId {
    ToolKind::Display.first_id() + offset
}

/// The diagnostic patterns, in the order of the spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Black,
    White,
    Red,
    Green,
    Blue,
    Gray5,
    Gray20,
    Gray50,
    GrayRamp,
    RedRamp,
    GreenRamp,
    BlueRamp,
    Grid,
}

impl Pattern {
    pub const ALL: [Pattern; 13] = [
        Pattern::Black,
        Pattern::White,
        Pattern::Red,
        Pattern::Green,
        Pattern::Blue,
        Pattern::Gray5,
        Pattern::Gray20,
        Pattern::Gray50,
        Pattern::GrayRamp,
        Pattern::RedRamp,
        Pattern::GreenRamp,
        Pattern::BlueRamp,
        Pattern::Grid,
    ];

    pub fn name(self, t: &Translator) -> String {
        let id = match self {
            Pattern::Black => StringId::ToolDisplayBlack,
            Pattern::White => StringId::ToolDisplayWhite,
            Pattern::Red => StringId::ToolDisplayRed,
            Pattern::Green => StringId::ToolDisplayGreen,
            Pattern::Blue => StringId::ToolDisplayBlue,
            Pattern::Gray5 => return t.format(StringId::ToolDisplayGray, &[Arg::Int(5)]),
            Pattern::Gray20 => return t.format(StringId::ToolDisplayGray, &[Arg::Int(20)]),
            Pattern::Gray50 => return t.format(StringId::ToolDisplayGray, &[Arg::Int(50)]),
            Pattern::GrayRamp => StringId::ToolDisplayGrayRamp,
            Pattern::RedRamp => StringId::ToolDisplayRedRamp,
            Pattern::GreenRamp => StringId::ToolDisplayGreenRamp,
            Pattern::BlueRamp => StringId::ToolDisplayBlueRamp,
            Pattern::Grid => StringId::ToolDisplayGrid,
        };
        t.get(id).into_owned()
    }

    /// The operations that draw the pattern on a buffer of `width` x `height` device pixels.
    pub fn ops(self, width: i32, height: i32) -> Vec<PatternOp> {
        let ramp = |to: Color| {
            vec![PatternOp::Gradient(
                PixelRect::new(0, 0, width, height),
                [BLACK, to, to, BLACK],
            )]
        };
        match self {
            Pattern::Black => vec![PatternOp::Fill(BLACK)],
            Pattern::White => vec![PatternOp::Fill(Color::WHITE)],
            Pattern::Red => vec![PatternOp::Fill(RED)],
            Pattern::Green => vec![PatternOp::Fill(GREEN)],
            Pattern::Blue => vec![PatternOp::Fill(BLUE)],
            Pattern::Gray5 => vec![PatternOp::Fill(GRAY_5)],
            Pattern::Gray20 => vec![PatternOp::Fill(GRAY_20)],
            Pattern::Gray50 => vec![PatternOp::Fill(GRAY_50)],
            Pattern::GrayRamp => ramp(Color::WHITE),
            Pattern::RedRamp => ramp(RED),
            Pattern::GreenRamp => ramp(GREEN),
            Pattern::BlueRamp => ramp(BLUE),
            Pattern::Grid => grid(width, height),
        }
    }
}

/// White lines of 1 device pixel on black, with square cells, and lines on the last column and row
/// too, so the four edges of the panel are drawn.
fn grid(width: i32, height: i32) -> Vec<PatternOp> {
    let step = (height / GRID_ROWS).max(2);
    let mut ops = vec![PatternOp::Fill(BLACK)];
    let columns = (0..width).step_by(step as usize).chain([width - 1]);
    for x in columns {
        ops.push(vline(x, 0, height, Color::WHITE));
    }
    let rows = (0..height).step_by(step as usize).chain([height - 1]);
    for y in rows {
        ops.push(hline(0, y, width, Color::WHITE));
    }
    ops
}

/// A horizontal line 1 device pixel thick.
pub(super) fn hline(x: i32, y: i32, width: i32, color: Color) -> PatternOp {
    PatternOp::Rect(PixelRect::new(x, y, width, 1), color)
}

/// A vertical line 1 device pixel thick.
pub(super) fn vline(x: i32, y: i32, height: i32, color: Color) -> PatternOp {
    PatternOp::Rect(PixelRect::new(x, y, 1, height), color)
}

/// The outline of `rect`, 1 device pixel thick, inside it.
pub(super) fn outline(rect: PixelRect, color: Color) -> [PatternOp; 4] {
    let PixelRect { x, y, w, h } = rect;
    [
        hline(x, y, w, color),
        hline(x, y + h - 1, w, color),
        vline(x, y, h, color),
        vline(x + w - 1, y, h, color),
    ]
}

/// The color of the "Colors" cleaning `elapsed` ms after it started: each color stays
/// [`CLEAN_HOLD_MS`], then fades into the next one in [`CLEAN_FADE_MS`].
pub fn cleaning_color(elapsed: u64) -> Color {
    let period = CLEAN_HOLD_MS + CLEAN_FADE_MS;
    let index = (elapsed / period) as usize;
    let into = elapsed % period;
    let color = CLEAN_COLORS[index % CLEAN_COLORS.len()];
    if into < CLEAN_HOLD_MS {
        return color;
    }
    let next = CLEAN_COLORS[(index + 1) % CLEAN_COLORS.len()];
    mix_color(
        color,
        next,
        (into - CLEAN_HOLD_MS) as f32 / CLEAN_FADE_MS as f32,
    )
}

/// What the tool shows.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// The three modes, with the description of the focused one.
    Modes,
    /// A diagnostic pattern, by its index in [`Pattern::ALL`].
    Diagnostic(usize),
    /// The choice of the cleaning's duration and pattern.
    CleanSetup,
    Cleaning {
        started: u64,
        ends: u64,
    },
    /// The choice of the fixer's duration.
    FixSetup,
    /// The window is moved over the stuck pixel.
    FixPlace,
    /// The flash warning, over the placed window.
    FixWarning(FlashWarning),
    Fixing {
        ends: u64,
    },
}

/// The tool's state, alive only while it is open.
#[derive(Debug)]
pub struct DisplayTool {
    phase: Phase,
    clean_minutes: usize,
    clean_colors: bool,
    fix_minutes: usize,
    /// The top left corner of the stuck pixel window, in device pixels; set the first time it is
    /// placed (in the middle) and kept while the tool is open.
    window: Option<(i32, i32)>,
    /// The size of the render buffer, from the last frame.
    screen: (i32, i32),
    hint: Hint,
    /// The window's color, by its index in the fixer's colors: one step per frame drawn.
    fix_step: usize,
    /// When the last frame was drawn, to step the fixer's colors once per frame.
    last_frame: u64,
    /// The last arrow and when, to tell a key held down.
    last_arrow: Option<(Key, u64)>,
}

impl DisplayTool {
    pub(in crate::app) fn new(_now_ms: u64) -> DisplayTool {
        DisplayTool {
            phase: Phase::Modes,
            clean_minutes: 0,
            clean_colors: false,
            fix_minutes: 0,
            window: None,
            screen: (1920, 1080),
            hint: Hint::default(),
            fix_step: 0,
            last_frame: 0,
            last_arrow: None,
        }
    }

    /// Whether the phase covers the whole screen with a pattern at `now_ms`. A timed mode whose time
    /// is over is not: the frame that finds it over draws the tool screen.
    fn fullscreen(&self, now_ms: u64) -> bool {
        match self.phase {
            Phase::Modes | Phase::CleanSetup | Phase::FixSetup => false,
            Phase::Cleaning { ends, .. } | Phase::Fixing { ends } => now_ms < ends,
            Phase::Diagnostic(_) | Phase::FixPlace | Phase::FixWarning(_) => true,
        }
    }

    /// The middle of the screen for the window, or where it was left.
    fn window_or_center(&mut self) -> (i32, i32) {
        let (width, height) = self.screen;
        *self
            .window
            .get_or_insert(((width - WINDOW_SIZE) / 2, (height - WINDOW_SIZE) / 2))
    }

    /// Moves the window by an arrow, within the screen.
    fn move_window(&mut self, key: Key, now_ms: u64) {
        let held = self
            .last_arrow
            .is_some_and(|(last, at)| last == key && now_ms.saturating_sub(at) < HELD_MS);
        self.last_arrow = Some((key, now_ms));
        let step = if held { WINDOW_STEP_HELD } else { WINDOW_STEP };
        let (dx, dy) = match key {
            Key::Left => (-step, 0),
            Key::Right => (step, 0),
            Key::Up => (0, -step),
            Key::Down => (0, step),
            _ => (0, 0),
        };
        let (x, y) = self.window_or_center();
        let (width, height) = self.screen;
        self.window = Some((
            (x + dx).clamp(0, (width - WINDOW_SIZE).max(0)),
            (y + dy).clamp(0, (height - WINDOW_SIZE).max(0)),
        ));
    }
}

impl LumeApp {
    fn display_state(&mut self) -> Option<&mut DisplayTool> {
        match self.tool.as_deref_mut() {
            Some(super::ToolState::Display(s)) => Some(s),
            _ => None,
        }
    }

    fn display_ref(&self) -> Option<&DisplayTool> {
        match self.tool.as_deref() {
            Some(super::ToolState::Display(s)) => Some(s),
            _ => None,
        }
    }

    /// Opening the tool (its state was just created): effects for the platform.
    pub(in crate::app) fn display_open(&mut self) -> Vec<Effect> {
        vec![]
    }

    pub(in crate::app) fn draw_display(&mut self, p: &mut Painter) {
        let now = self.now_ms;
        let t = self.translator;
        let Some(s) = self.display_state() else {
            return;
        };
        let (width, height) = (p.display.width as i32, p.display.height as i32);
        s.screen = (width, height);
        let new_frame = now != s.last_frame;
        s.last_frame = now;

        // A timed mode that is over goes back to its choices, with a message.
        let mut done = None;
        match s.phase {
            Phase::Cleaning { ends, .. } if now >= ends => {
                s.phase = Phase::CleanSetup;
                done = Some((CLEAN_START, StringId::ToolDisplayCleanDone));
            }
            Phase::Fixing { ends } if now >= ends => {
                s.phase = Phase::FixSetup;
                done = Some((FIX_PLACE, StringId::ToolDisplayFixDone));
            }
            _ => {}
        }
        if let Some((item, message)) = done {
            s.hint.hide();
            self.pending_focus = Some(PendingFocus::Id(id(item)));
            self.show_toast(t.get(message).into_owned());
            self.focus.set_rows(vec![]);
        }

        let Some(s) = self.display_state() else {
            return;
        };
        match s.phase {
            Phase::Modes | Phase::CleanSetup | Phase::FixSetup => {
                let phase = s.phase;
                self.draw_display_choices(p, phase);
                return;
            }
            Phase::Diagnostic(index) => {
                for op in Pattern::ALL[index].ops(width, height) {
                    p.list.pattern(op);
                }
            }
            Phase::Cleaning { started, .. } => {
                let color = if s.clean_colors {
                    cleaning_color(now.saturating_sub(started))
                } else {
                    Color::WHITE
                };
                p.list.pattern(PatternOp::Fill(color));
            }
            Phase::FixPlace | Phase::FixWarning(_) | Phase::Fixing { .. } => {
                let (x, y) = s.window_or_center();
                let color = match s.phase {
                    Phase::Fixing { .. } => {
                        if new_frame {
                            s.fix_step = (s.fix_step + 1) % FIX_COLORS.len();
                        }
                        FIX_COLORS[s.fix_step]
                    }
                    _ => Color::WHITE,
                };
                p.list.pattern(PatternOp::Fill(BLACK));
                p.list.pattern(PatternOp::Rect(
                    PixelRect::new(x, y, WINDOW_SIZE, WINDOW_SIZE),
                    color,
                ));
            }
        }
        s.hint.draw(p, now);
        if let Phase::FixWarning(warning) = s.phase {
            warning.draw(p, &t);
        }
    }

    /// The modes, or the choices of a mode, at the left, and what the mode does at the right.
    fn draw_display_choices(&mut self, p: &mut Painter, phase: Phase) {
        let t = self.translator;
        let Some(s) = self.display_ref() else {
            return;
        };
        let duration =
            |minutes: u64| t.format(StringId::ToolDisplayDuration, &[Arg::Int(minutes as i64)]);
        let items: Vec<(FocusId, String)> = match phase {
            Phase::CleanSetup => {
                let pattern = t.get(if s.clean_colors {
                    StringId::ToolDisplayColors
                } else {
                    StringId::ToolDisplayWhite
                });
                vec![
                    (id(CLEAN_DURATION), duration(CLEAN_MINUTES[s.clean_minutes])),
                    (
                        id(CLEAN_PATTERN),
                        t.format(StringId::ToolDisplayPattern, &[Arg::Str(&pattern)]),
                    ),
                    (id(CLEAN_START), t.get(StringId::ToolsStart).into_owned()),
                ]
            }
            Phase::FixSetup => vec![
                (id(FIX_DURATION), duration(FIX_MINUTES[s.fix_minutes])),
                (
                    id(FIX_PLACE),
                    t.get(StringId::ToolDisplayPlace).into_owned(),
                ),
            ],
            _ => [
                (MODE_DIAGNOSTIC, StringId::ToolDisplayDiagnostic),
                (MODE_CLEAN, StringId::ToolDisplayClean),
                (MODE_FIX, StringId::ToolDisplayFix),
            ]
            .into_iter()
            .map(|(item, name)| (id(item), t.get(name).into_owned()))
            .collect(),
        };
        self.draw_tool_items(p, &items, 0.0);

        // The mode described: the one of the choices, or the focused one.
        let mode = match phase {
            Phase::CleanSetup => MODE_CLEAN,
            Phase::FixSetup => MODE_FIX,
            _ => self
                .focus
                .current_id()
                .map_or(MODE_DIAGNOSTIC, |f| f.saturating_sub(id(0)).min(MODE_FIX)),
        };
        let (name, description) = match mode {
            MODE_CLEAN => (StringId::ToolDisplayClean, StringId::ToolDisplayCleanDesc),
            MODE_FIX => (StringId::ToolDisplayFix, StringId::ToolDisplayFixDesc),
            _ => (
                StringId::ToolDisplayDiagnostic,
                StringId::ToolDisplayDiagnosticDesc,
            ),
        };
        draw_description(p, &t.get(name), &t.get(description));
        self.draw_tool_header(p, ToolKind::Display);
    }

    pub(in crate::app) fn display_key(&mut self, key: Key) -> ToolKey {
        let now = self.now_ms;
        let t = self.translator;
        let focused = self.focus.current_id().map(|f| f.wrapping_sub(id(0)));
        let Some(s) = self.display_state() else {
            return ToolKey::Default;
        };
        // Where the focus goes when the tool screen comes back.
        let mut focus_on = None;
        match s.phase {
            Phase::Modes => match (key, focused) {
                (Key::Ok, Some(MODE_DIAGNOSTIC)) => {
                    s.phase = Phase::Diagnostic(0);
                    s.hint.show(Pattern::ALL[0].name(&t), now);
                }
                (Key::Ok, Some(MODE_CLEAN)) => {
                    s.phase = Phase::CleanSetup;
                    focus_on = Some(CLEAN_DURATION);
                }
                (Key::Ok, Some(MODE_FIX)) => {
                    s.phase = Phase::FixSetup;
                    focus_on = Some(FIX_DURATION);
                }
                _ => return ToolKey::Default,
            },
            Phase::Diagnostic(index) => {
                let count = Pattern::ALL.len();
                let index = match key {
                    Key::Back => {
                        s.phase = Phase::Modes;
                        s.hint.hide();
                        focus_on = Some(MODE_DIAGNOSTIC);
                        index
                    }
                    Key::Right => (index + 1) % count,
                    Key::Left => (index + count - 1) % count,
                    _ => index,
                };
                if key != Key::Back {
                    s.phase = Phase::Diagnostic(index);
                    s.hint.show(Pattern::ALL[index].name(&t), now);
                }
            }
            Phase::CleanSetup => match (key, focused) {
                (Key::Back, _) => {
                    s.phase = Phase::Modes;
                    focus_on = Some(MODE_CLEAN);
                }
                (Key::Ok | Key::Right, Some(CLEAN_DURATION)) => {
                    s.clean_minutes = (s.clean_minutes + 1) % CLEAN_MINUTES.len();
                }
                (Key::Left, Some(CLEAN_DURATION)) => {
                    s.clean_minutes =
                        (s.clean_minutes + CLEAN_MINUTES.len() - 1) % CLEAN_MINUTES.len();
                }
                (Key::Ok | Key::Left | Key::Right, Some(CLEAN_PATTERN)) => {
                    s.clean_colors = !s.clean_colors;
                }
                (Key::Ok, Some(CLEAN_START)) => {
                    let ends = now + CLEAN_MINUTES[s.clean_minutes] * 60_000;
                    s.phase = Phase::Cleaning { started: now, ends };
                    s.hint.show(time_left(&t, ends - now), now);
                }
                _ => return ToolKey::Default,
            },
            Phase::Cleaning { ends, .. } => {
                if key == Key::Back {
                    s.phase = Phase::CleanSetup;
                    s.hint.hide();
                    focus_on = Some(CLEAN_START);
                } else {
                    s.hint.show(time_left(&t, ends.saturating_sub(now)), now);
                }
            }
            Phase::FixSetup => match (key, focused) {
                (Key::Back, _) => {
                    s.phase = Phase::Modes;
                    focus_on = Some(MODE_FIX);
                }
                (Key::Ok | Key::Right, Some(FIX_DURATION)) => {
                    s.fix_minutes = (s.fix_minutes + 1) % FIX_MINUTES.len();
                }
                (Key::Left, Some(FIX_DURATION)) => {
                    s.fix_minutes = (s.fix_minutes + FIX_MINUTES.len() - 1) % FIX_MINUTES.len();
                }
                (Key::Ok, Some(FIX_PLACE)) => {
                    s.window_or_center();
                    s.phase = Phase::FixPlace;
                    s.last_arrow = None;
                    s.hint
                        .show(t.get(StringId::ToolDisplayPlaceHint).into_owned(), now);
                }
                _ => return ToolKey::Default,
            },
            Phase::FixPlace => match key {
                Key::Back => {
                    s.phase = Phase::FixSetup;
                    s.hint.hide();
                    focus_on = Some(FIX_PLACE);
                }
                Key::Ok => {
                    s.hint.hide();
                    s.phase = Phase::FixWarning(FlashWarning::new());
                }
                Key::Left | Key::Right | Key::Up | Key::Down => s.move_window(key, now),
                _ => {}
            },
            Phase::FixWarning(mut warning) => match warning.key(key) {
                WarningAnswer::Open => s.phase = Phase::FixWarning(warning),
                WarningAnswer::Start => {
                    let ends = now + FIX_MINUTES[s.fix_minutes] * 60_000;
                    s.phase = Phase::Fixing { ends };
                    s.hint.show(time_left(&t, ends - now), now);
                }
                WarningAnswer::Cancel => {
                    s.phase = Phase::FixSetup;
                    focus_on = Some(FIX_PLACE);
                }
            },
            Phase::Fixing { ends } => {
                if key == Key::Back {
                    // Back to placing the window, where it was.
                    s.phase = Phase::FixPlace;
                    s.last_arrow = None;
                    s.hint
                        .show(t.get(StringId::ToolDisplayPlaceHint).into_owned(), now);
                } else {
                    s.hint.show(time_left(&t, ends.saturating_sub(now)), now);
                }
            }
        }
        if let Some(item) = focus_on {
            self.pending_focus = Some(PendingFocus::Id(id(item)));
            self.focus.set_rows(vec![]);
        }
        ToolKey::Handled(vec![])
    }

    pub(in crate::app) fn display_deadline(&self, _now_ms: u64) -> Option<u64> {
        let s = self.display_ref()?;
        let now = self.now_ms;
        let mode = match s.phase {
            // A frame for every step of the fade, and none while a color stays.
            Phase::Cleaning { started, ends } if s.clean_colors => {
                let period = CLEAN_HOLD_MS + CLEAN_FADE_MS;
                let into = now.saturating_sub(started) % period;
                let next = if into < CLEAN_HOLD_MS {
                    now + (CLEAN_HOLD_MS - into)
                } else {
                    now + FRAME_MS
                };
                Some(next.min(ends))
            }
            Phase::Cleaning { ends, .. } => Some(ends),
            Phase::Fixing { ends } => Some((now + FRAME_MS).min(ends)),
            _ => None,
        };
        [mode, s.hint.deadline(now)].into_iter().flatten().min()
    }

    pub(in crate::app) fn display_fullscreen(&self) -> bool {
        self.display_ref()
            .is_some_and(|s| s.fullscreen(self.now_ms))
    }

    pub(in crate::app) fn display_event(&mut self, _event: ToolEvent) {}
}

/// "Faltam 9:59".
pub(super) fn time_left(t: &Translator, ms: u64) -> String {
    t.format(StringId::ToolsTimeLeft, &[Arg::Str(&format_remaining(ms))])
}

/// The panel at the right with a mode's name and what it can and cannot do.
pub(super) fn draw_description(p: &mut Painter, name: &str, description: &str) {
    let inner = draw_panel(p, common::panel_rect(DESC_PANEL_HEIGHT));
    panel_heading(p, inner, name);
    let body = p.style(&THEME.breadcrumb, StateFlags::NONE);
    let text = Rect::new(inner.x, inner.y + DESC_TOP, inner.w, body.line_height);
    wrapped_text(p, description, text, body.line_height, &body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};

    fn state(rig: &Rig) -> &DisplayTool {
        rig.app.display_ref().expect("the Screen tool is open")
    }

    fn phase(rig: &Rig) -> Phase {
        state(rig).phase
    }

    /// The pattern operations of the last frame.
    fn pattern(rig: &mut Rig) -> Vec<PatternOp> {
        rig.frame().pattern_ops().to_vec()
    }

    fn open_mode(rig: &mut Rig, downs: usize) {
        open_tool(rig, ToolKind::Display);
        for _ in 0..downs {
            rig.press(Key::Down);
        }
        rig.press(Key::Ok);
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("DESC_PANEL_HEIGHT", DESC_PANEL_HEIGHT),
            ("DESC_TOP", DESC_TOP),
            ("WINDOW_SIZE", WINDOW_SIZE as f32),
            ("WINDOW_STEP", WINDOW_STEP as f32),
            ("WINDOW_STEP_HELD", WINDOW_STEP_HELD as f32),
        ]);
    }

    #[test]
    fn the_tool_lists_three_modes() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Display);
        assert_eq!(rig.focused(), Some(id(MODE_DIAGNOSTIC)));
        rig.press_all(&[Key::Down, Key::Down, Key::Down]);
        assert_eq!(rig.focused(), Some(id(MODE_FIX)));
    }

    #[test]
    fn the_patterns_go_in_order_and_around() {
        let mut rig = Rig::new();
        open_mode(&mut rig, 0);
        assert_eq!(phase(&rig), Phase::Diagnostic(0));
        assert!(rig.app.tool_fullscreen());
        assert_eq!(pattern(&mut rig), vec![PatternOp::Fill(BLACK)]);
        rig.press(Key::Right);
        assert_eq!(pattern(&mut rig), vec![PatternOp::Fill(Color::WHITE)]);
        rig.press(Key::Right);
        assert_eq!(pattern(&mut rig), vec![PatternOp::Fill(RED)]);
        assert!(state(&rig).hint.is_visible(rig.now));
        // Around: from the grid (the last) to black, and back.
        for _ in 2..Pattern::ALL.len() - 1 {
            rig.press(Key::Right);
        }
        assert_eq!(phase(&rig), Phase::Diagnostic(12));
        rig.press(Key::Right);
        assert_eq!(phase(&rig), Phase::Diagnostic(0));
        rig.press(Key::Left);
        assert_eq!(phase(&rig), Phase::Diagnostic(12));
    }

    #[test]
    fn the_name_shows_three_seconds_and_then_no_frames() {
        let mut rig = Rig::new();
        open_mode(&mut rig, 0);
        rig.press(Key::Right);
        let shown = rig.now - 100;
        assert_eq!(
            rig.app.display_deadline(rig.now),
            Some(shown + common::HINT_MS)
        );
        rig.advance(3000);
        assert_eq!(rig.app.display_deadline(rig.now), None);
        // Only the clock's minute is left.
        let next = rig.app.next_deadline(rig.now).unwrap();
        assert!(next > rig.now + 1000, "no frames while a pattern stands");
        assert!(!state(&rig).hint.is_visible(rig.now));
    }

    #[test]
    fn back_leaves_the_pattern_with_the_focus_on_its_mode() {
        let mut rig = Rig::new();
        open_mode(&mut rig, 0);
        rig.press(Key::Back);
        assert_eq!(phase(&rig), Phase::Modes);
        assert!(!rig.app.tool_fullscreen());
        assert_eq!(rig.focused(), Some(id(MODE_DIAGNOSTIC)));
        assert_eq!(
            rig.app.screen(),
            &crate::app::Screen::Tool(ToolKind::Display)
        );
    }

    #[test]
    fn the_grid_lines_are_one_pixel_and_reach_every_edge() {
        let ops = grid(1920, 1080);
        let rects: Vec<PixelRect> = ops
            .iter()
            .filter_map(|op| match op {
                PatternOp::Rect(r, _) => Some(*r),
                _ => None,
            })
            .collect();
        assert!(rects.iter().all(|r| r.w == 1 || r.h == 1));
        assert!(rects.contains(&PixelRect::new(1919, 0, 1, 1080)));
        assert!(rects.contains(&PixelRect::new(0, 1079, 1920, 1)));
        assert!(rects.contains(&PixelRect::new(60, 0, 1, 1080)));
        // 32 + 1 columns and 18 + 1 rows.
        assert_eq!(rects.len(), 33 + 19);
    }

    /// Opens the cleaning's choices with "Colors" (or "White") and starts it.
    fn start_cleaning(rig: &mut Rig, colors: bool) {
        open_mode(rig, 1);
        assert_eq!(phase(rig), Phase::CleanSetup);
        assert_eq!(rig.focused(), Some(id(CLEAN_DURATION)));
        rig.press(Key::Down);
        if colors {
            rig.press(Key::Ok);
        }
        rig.press(Key::Down);
        assert_eq!(rig.focused(), Some(id(CLEAN_START)));
        rig.press(Key::Ok);
        assert!(matches!(phase(rig), Phase::Cleaning { .. }));
    }

    #[test]
    fn the_cleaning_starts_only_with_ok_on_start() {
        let mut rig = Rig::new();
        open_mode(&mut rig, 1);
        rig.press(Key::Ok); // 10 -> 30 min
        assert_eq!(state(&rig).clean_minutes, 1);
        assert_eq!(phase(&rig), Phase::CleanSetup);
        assert!(!rig.app.tool_fullscreen());
        rig.press(Key::Back);
        assert_eq!(phase(&rig), Phase::Modes);
        assert_eq!(rig.focused(), Some(id(MODE_CLEAN)));
    }

    #[test]
    fn the_colors_change_slowly_and_gradually() {
        // Sampled every frame for a minute: a new color starts at most once every 3 s, and a
        // frame never moves a channel more than a few levels.
        let mut previous = cleaning_color(0);
        let mut changes = Vec::new();
        let mut held = previous;
        for ms in (0..60_000).step_by(FRAME_MS as usize) {
            let color = cleaning_color(ms);
            let step = [
                color.r.abs_diff(previous.r),
                color.g.abs_diff(previous.g),
                color.b.abs_diff(previous.b),
            ];
            assert!(step.iter().all(|d| *d <= 5), "{ms} ms: {step:?}");
            if CLEAN_COLORS.contains(&color) && color != held {
                changes.push(ms);
                held = color;
            }
            previous = color;
        }
        assert!(changes.windows(2).all(|w| w[1] - w[0] >= 2000));
        // Each color stays 2 s and the fade takes 1 s.
        assert_eq!(cleaning_color(1999), Color::WHITE);
        assert_ne!(cleaning_color(2500), Color::WHITE);
        assert_eq!(cleaning_color(3000), RED);
        assert_eq!(cleaning_color(4999), RED);
        // At most 3 color changes in any second: far below it.
        let per_second = changes.windows(2).filter(|w| w[1] - w[0] < 1000).count();
        assert_eq!(per_second, 0);
    }

    #[test]
    fn the_colors_cleaning_draws_frames_only_while_fading() {
        let mut rig = Rig::new();
        start_cleaning(&mut rig, true);
        let started = rig.now - 100;
        // White holds: the next frame is the end of the hint, then the start of the fade.
        assert_eq!(
            rig.app.display_deadline(rig.now),
            Some(CLEAN_HOLD_MS + started)
        );
        rig.advance(CLEAN_HOLD_MS - 100); // fading to red
        assert_eq!(rig.app.display_deadline(rig.now), Some(rig.now + FRAME_MS));
        rig.advance(1100); // red holds until 5 s
        assert_eq!(rig.now - started, 3100);
        assert_eq!(
            rig.app.display_deadline(rig.now),
            Some(started + CLEAN_HOLD_MS + CLEAN_FADE_MS + CLEAN_HOLD_MS)
        );
    }

    #[test]
    fn the_white_cleaning_is_static() {
        let mut rig = Rig::new();
        start_cleaning(&mut rig, false);
        assert_eq!(pattern(&mut rig), vec![PatternOp::Fill(Color::WHITE)]);
        rig.advance(3000);
        let Phase::Cleaning { ends, .. } = phase(&rig) else {
            panic!("cleaning");
        };
        assert_eq!(rig.app.display_deadline(rig.now), Some(ends));
    }

    #[test]
    fn the_time_left_shows_on_each_key() {
        let mut rig = Rig::new();
        start_cleaning(&mut rig, false);
        rig.advance(5000);
        assert!(!state(&rig).hint.is_visible(rig.now));
        rig.press(Key::Right);
        assert!(state(&rig).hint.is_visible(rig.now));
        assert!(matches!(phase(&rig), Phase::Cleaning { .. }));
        rig.advance(3000);
        assert!(!state(&rig).hint.is_visible(rig.now));
    }

    #[test]
    fn the_cleaning_ends_by_itself_with_a_message() {
        let mut rig = Rig::new();
        start_cleaning(&mut rig, true);
        let Phase::Cleaning { ends, .. } = phase(&rig) else {
            panic!("cleaning");
        };
        assert_eq!(ends - (rig.now - 100), 10 * 60_000);
        rig.advance(ends - rig.now - 1);
        assert!(rig.app.tool_fullscreen());
        rig.advance(1);
        assert_eq!(phase(&rig), Phase::CleanSetup);
        assert!(!rig.app.tool_fullscreen());
        assert_eq!(rig.focused(), Some(id(CLEAN_START)));
        let toast = rig.app.toast.as_ref().map(|(text, _)| text.as_str());
        assert_eq!(toast, Some("Limpeza concluída"));
    }

    #[test]
    fn back_stops_the_cleaning_on_start() {
        let mut rig = Rig::new();
        start_cleaning(&mut rig, true);
        rig.press(Key::Back);
        assert_eq!(phase(&rig), Phase::CleanSetup);
        assert_eq!(rig.focused(), Some(id(CLEAN_START)));
        assert_eq!(rig.app.display_deadline(rig.now), None);
    }

    /// Opens the fixer's choices and the window placement.
    fn place_window(rig: &mut Rig) {
        open_mode(rig, 2);
        assert_eq!(phase(rig), Phase::FixSetup);
        rig.press(Key::Down);
        rig.press(Key::Ok);
        assert_eq!(phase(rig), Phase::FixPlace);
    }

    #[test]
    fn the_window_starts_in_the_middle_and_moves_by_16() {
        let mut rig = Rig::new();
        place_window(&mut rig);
        // The rig's display is 1280x720.
        let middle = ((1280 - 64) / 2, (720 - 64) / 2);
        assert_eq!(state(&rig).window, Some(middle));
        rig.press(Key::Right);
        assert_eq!(state(&rig).window, Some((middle.0 + 16, middle.1)));
        rig.press(Key::Down);
        assert_eq!(state(&rig).window, Some((middle.0 + 16, middle.1 + 16)));
        // A key held down repeats faster than 250 ms: smaller steps.
        let effects = rig.app.handle_key(Key::Down, rig.now + 50);
        assert!(effects.is_empty());
        assert_eq!(state(&rig).window, Some((middle.0 + 16, middle.1 + 24)));
        let ops = pattern(&mut rig);
        assert_eq!(ops[0], PatternOp::Fill(BLACK));
        assert_eq!(
            ops[1],
            PatternOp::Rect(
                PixelRect::new(middle.0 + 16, middle.1 + 24, 64, 64),
                Color::WHITE
            )
        );
    }

    #[test]
    fn the_window_never_leaves_the_screen() {
        let mut rig = Rig::new();
        place_window(&mut rig);
        for _ in 0..100 {
            rig.press(Key::Right);
            rig.press(Key::Up);
        }
        assert_eq!(state(&rig).window, Some((1280 - 64, 0)));
        for _ in 0..100 {
            rig.press(Key::Left);
            rig.press(Key::Down);
        }
        assert_eq!(state(&rig).window, Some((0, 720 - 64)));
    }

    #[test]
    fn the_cycle_starts_only_after_start_on_the_warning() {
        let mut rig = Rig::new();
        place_window(&mut rig);
        rig.press(Key::Right);
        rig.press(Key::Ok);
        assert!(matches!(phase(&rig), Phase::FixWarning(_)));
        // Nothing flashes under the warning: the window stays white, and no frames are asked for.
        rig.advance(5000);
        assert_eq!(rig.app.display_deadline(rig.now), None);
        let ops = pattern(&mut rig);
        assert!(matches!(ops[1], PatternOp::Rect(_, Color::WHITE)));
        // Back refuses: the choices come back.
        rig.press(Key::Back);
        assert_eq!(phase(&rig), Phase::FixSetup);
        assert_eq!(rig.focused(), Some(id(FIX_PLACE)));

        rig.press(Key::Ok);
        rig.press(Key::Ok);
        rig.press(Key::Left); // the focus goes to "Start"
        rig.press(Key::Ok);
        assert!(matches!(phase(&rig), Phase::Fixing { .. }));
        // A color per frame, only in the window, the rest black.
        let mut colors = Vec::new();
        for _ in 0..5 {
            rig.advance(FRAME_MS);
            let ops = pattern(&mut rig);
            assert_eq!(ops.len(), 2);
            assert_eq!(ops[0], PatternOp::Fill(BLACK));
            match ops[1] {
                PatternOp::Rect(r, c) => {
                    assert_eq!((r.w, r.h), (64, 64));
                    colors.push(c);
                }
                other => panic!("{other:?}"),
            }
        }
        colors.sort_by_key(|c| (c.r, c.g, c.b));
        colors.dedup();
        assert_eq!(colors.len(), 5, "red, green, blue, white and black");
        assert_eq!(rig.app.display_deadline(rig.now), Some(rig.now + FRAME_MS));
    }

    #[test]
    fn back_during_the_cycle_places_the_window_where_it_was() {
        let mut rig = Rig::new();
        place_window(&mut rig);
        rig.press_all(&[Key::Left, Key::Left]);
        let placed = state(&rig).window;
        rig.press_all(&[Key::Ok, Key::Right, Key::Ok]);
        assert!(matches!(phase(&rig), Phase::Fixing { .. }));
        rig.press(Key::Back);
        assert_eq!(phase(&rig), Phase::FixPlace);
        assert_eq!(state(&rig).window, placed);
        assert_eq!(
            rig.app.display_deadline(rig.now),
            Some(rig.now - 100 + common::HINT_MS)
        );
    }

    #[test]
    fn the_fixer_ends_by_itself() {
        let mut rig = Rig::new();
        place_window(&mut rig);
        rig.press_all(&[Key::Ok, Key::Right, Key::Ok]);
        let Phase::Fixing { ends } = phase(&rig) else {
            panic!("fixing");
        };
        assert_eq!(ends - (rig.now - 100), 5 * 60_000);
        rig.advance(ends - rig.now);
        assert_eq!(phase(&rig), Phase::FixSetup);
        assert_eq!(rig.app.display_deadline(rig.now), None);
    }
}
