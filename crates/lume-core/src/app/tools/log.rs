//! The Log tool (`log-viewer`): lume's own log, newest first, on a screen of its own, to look for
//! failures on the TV.
//!
//! The platform reads lume's log file ([`crate::logfile`], earlier runs included) on the `lume-test`
//! thread when the tool opens and sends the lines parsed by [`parse`]; this side shows them. The
//! lines stay in English, as lume writes them.

use super::common::{draw_panel, panel_heading};
use super::{TestKind, TestUpdate, ToolEvent, ToolKey, ToolKind};
use crate::app::page::{LEFT, WIDTH};
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp};
use crate::i18n::StringId;
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{vars, StateFlags};

/// How many of the newest lines the platform reads.
pub const MAX_LINES: usize = 500;

/// Where the log's lines start under the panel's heading.
const LINES_TOP: f32 = 40.0;
/// One text line of the log (`--line-label`).
const LINE_PITCH: f32 = 24.0;
/// The gap between two entries.
const ENTRY_GAP: f32 = 8.0;
/// The column of the date, time and level, before the message.
const PREFIX_WIDTH: f32 = 200.0;
/// At most this many lines of one message are shown (a longer one ends in an ellipsis).
const MAX_ENTRY_LINES: usize = 6;

/// One line of the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    /// "11-15 12:24:30", as the log file writes it (without the milliseconds).
    pub time: String,
    /// `V`, `D`, `I`, `W`, `E` or `F`.
    pub level: char,
    pub tag: String,
    pub message: String,
}

impl LogLine {
    /// A warning, an error or a fatal error.
    pub fn is_failure(&self) -> bool {
        matches!(self.level, 'W' | 'E' | 'F')
    }
}

/// What the platform sends: the lines, oldest first, or that the log could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogUpdate {
    Lines(Vec<LogLine>),
    Failed,
}

/// Parses lines in `logcat -v threadtime` shape, as [`crate::logfile`] writes them (`11-15 12:24:18.910  3007  3025 I tag: message`), oldest
/// first. The buffer separators (`--------- beginning of main`) and lines in another shape are
/// skipped.
pub fn parse(text: &str) -> Vec<LogLine> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<LogLine> {
    let mut rest = line.trim_start();
    let mut field = || {
        let (word, tail) = rest.split_once(char::is_whitespace)?;
        rest = tail.trim_start();
        Some(word)
    };
    let date = field()?;
    let time = field()?;
    let _pid = field()?;
    let _tid = field()?;
    let level = field()?;
    let mut chars = level.chars();
    let level = match (chars.next(), chars.next()) {
        (Some(c @ ('V' | 'D' | 'I' | 'W' | 'E' | 'F')), None) => c,
        _ => return None,
    };
    if !date.contains('-') || !time.contains(':') {
        return None;
    }
    let (tag, message) = match rest.split_once(": ") {
        Some((tag, message)) => (tag.trim_end(), message),
        None => (rest.trim_end_matches(':').trim_end(), ""),
    };
    let time = time.split('.').next().unwrap_or(time);
    Some(LogLine {
        time: format!("{date} {time}"),
        level,
        tag: tag.to_string(),
        message: message.to_string(),
    })
}

/// The tool's state, alive only while it is open.
#[derive(Debug, Default)]
pub struct LogTool {
    /// `None` while the log is being read.
    lines: Option<Vec<LogLine>>,
    failed: bool,
    /// Only warnings and errors.
    failures_only: bool,
    /// The entry at the top of the panel, counted from the newest of the shown ones.
    top: usize,
    /// Whether the last frame showed the oldest entry whole: Down stops there.
    at_end: bool,
}

impl LogTool {
    pub(in crate::app) fn new(_now_ms: u64) -> LogTool {
        LogTool::default()
    }

    /// The shown lines, newest first.
    fn shown(&self) -> impl Iterator<Item = &LogLine> {
        let failures_only = self.failures_only;
        self.lines
            .iter()
            .flatten()
            .rev()
            .filter(move |l| !failures_only || l.is_failure())
    }
}

impl LumeApp {
    /// Opening the tool: the platform reads the log.
    pub(in crate::app) fn log_open(&mut self) -> Vec<Effect> {
        vec![Effect::StartTest(TestKind::Log)]
    }

    pub(in crate::app) fn draw_log(&mut self, p: &mut Painter) {
        let t = self.translator;
        // No items: the screen is the log (the focus rows are empty).
        self.draw_tool_items(p, &[], 0.0);
        self.draw_tool_header(p, ToolKind::Log);
        let Some(state) = self.tool_mut::<LogTool>() else {
            return;
        };

        let bottom = p
            .display
            .visible_rect()
            .bottom()
            .min(crate::display::CANVAS_HEIGHT);
        let top = super::common::ITEMS_TOP;
        let panel = Rect::new(LEFT, top, WIDTH, bottom - 36.0 - top);
        let inner = draw_panel(p, panel);
        let filter = t.get(if state.failures_only {
            StringId::ToolLogFailures
        } else {
            StringId::ToolLogAll
        });
        panel_heading(
            p,
            inner,
            &format!("{} · {}", t.get(StringId::ToolLogHeading), filter),
        );
        let muted = p.style(|t| &t.breadcrumb, StateFlags::NONE);
        let hint = t.get(if state.failures_only {
            StringId::ToolLogShowAll
        } else {
            StringId::ToolLogShowFailures
        });
        p.text(
            &hint,
            Rect::new(inner.x, inner.y, inner.w, 20.0),
            Align::End,
            &muted,
        );

        let area = Rect::new(
            inner.x,
            inner.y + LINES_TOP,
            inner.w,
            inner.bottom() - (inner.y + LINES_TOP),
        );
        let empty = match (&state.lines, state.failed) {
            (_, true) => Some(StringId::ToolLogFailed),
            (None, _) => Some(StringId::ToolLogLoading),
            (Some(_), _) if state.shown().next().is_none() => Some(StringId::ToolLogEmpty),
            _ => None,
        };
        if let Some(message) = empty {
            state.at_end = true;
            let line = Rect::new(area.x, area.y, area.w, LINE_PITCH);
            p.text(&t.get(message), line, Align::Start, &muted);
            return;
        }

        let mut text = muted;
        text.color = vars::TEXT;
        let mut failure = text;
        failure.color = vars::DANGER_FG;
        let text_style = Painter::text_style(&text);
        let message_x = area.x + PREFIX_WIDTH;
        let message_w = area.w - PREFIX_WIDTH;

        p.list.push_clip(area);
        let mut y = area.y;
        let mut at_end = true;
        let top_entry = state.top;
        for line in state.shown().skip(top_entry) {
            if y >= area.bottom() {
                at_end = false;
                break;
            }
            let style = if matches!(line.level, 'E' | 'F') {
                &failure
            } else {
                &text
            };
            let prefix = format!("{}  {}", line.time, line.level);
            p.text(
                &prefix,
                Rect::new(area.x, y, PREFIX_WIDTH, LINE_PITCH),
                Align::Start,
                &muted,
            );
            let body = if line.message.is_empty() {
                line.tag.clone()
            } else {
                format!("{}: {}", line.tag, line.message)
            };
            let mut rows = crate::app::editor::wrap(p, &body, &text_style, message_w);
            if rows.len() > MAX_ENTRY_LINES {
                rows.truncate(MAX_ENTRY_LINES);
                if let Some(last) = rows.last_mut() {
                    last.push('…');
                }
            }
            for row in &rows {
                p.text(
                    row,
                    Rect::new(message_x, y, message_w, LINE_PITCH),
                    Align::Start,
                    style,
                );
                y += LINE_PITCH;
            }
            y += ENTRY_GAP;
            if y - ENTRY_GAP > area.bottom() {
                at_end = false;
            }
        }
        p.list.pop_clip();
        if let Some(state) = self.tool_mut::<LogTool>() {
            state.at_end = at_end;
        }
    }

    /// Up and Down scroll one entry, OK switches between everything and only the failures; Back
    /// leaves (the usual handling).
    pub(in crate::app) fn log_key(&mut self, key: Key) -> ToolKey {
        let Some(state) = self.tool_mut::<LogTool>() else {
            return ToolKey::Default;
        };
        match key {
            Key::Up => state.top = state.top.saturating_sub(1),
            Key::Down if !state.at_end => state.top += 1,
            Key::Ok => {
                state.failures_only = !state.failures_only;
                state.top = 0;
                state.at_end = false;
            }
            Key::Down | Key::Left | Key::Right | Key::Menu => {}
            Key::Back | Key::Home => return ToolKey::Default,
        }
        ToolKey::Handled(vec![])
    }

    pub(in crate::app) fn log_event(&mut self, event: ToolEvent) {
        let ToolEvent::Test(TestUpdate::Log(update)) = event else {
            return;
        };
        if let Some(state) = self.tool_mut::<LogTool>() {
            match update {
                LogUpdate::Lines(lines) => {
                    state.lines = Some(lines);
                    state.failed = false;
                }
                LogUpdate::Failed => state.failed = true,
            }
            state.top = 0;
            state.at_end = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};
    use crate::app::Screen;

    const SAMPLE: &str = "--------- beginning of main
11-15 12:24:18.910  3007  3025 I lume    : window ready
11-15 12:24:19.001  3007  3025 W lume    : cannot open the audio output: busy
11-15 12:24:20.500  3007  3030 E lume    : storage test failed: No space left on device
not a log line
11-15 12:24:21.000  3007  3025 D lume:
";

    fn state(rig: &Rig) -> &LogTool {
        rig.app.tool_ref::<LogTool>().expect("the log tool is open")
    }

    fn opened_with(text: &str) -> Rig {
        let mut rig = Rig::new();
        let effects = open_tool(&mut rig, ToolKind::Log);
        assert!(effects.contains(&Effect::StartTest(TestKind::Log)));
        let now = rig.now;
        rig.app.tool_event(
            ToolEvent::Test(TestUpdate::Log(LogUpdate::Lines(parse(text)))),
            now,
        );
        rig.frame();
        rig
    }

    #[test]
    fn threadtime_lines_are_parsed_and_the_rest_skipped() {
        let lines = parse(SAMPLE);
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[1],
            LogLine {
                time: "11-15 12:24:19".into(),
                level: 'W',
                tag: "lume".into(),
                message: "cannot open the audio output: busy".into(),
            }
        );
        assert_eq!(lines[3].tag, "lume");
        assert_eq!(lines[3].message, "");
    }

    #[test]
    fn the_newest_line_comes_first_and_ok_keeps_only_the_failures() {
        let mut rig = opened_with(SAMPLE);
        let shown: Vec<char> = state(&rig).shown().map(|l| l.level).collect();
        assert_eq!(shown, vec!['D', 'E', 'W', 'I']);

        rig.press(Key::Ok);
        let shown: Vec<char> = state(&rig).shown().map(|l| l.level).collect();
        assert_eq!(shown, vec!['E', 'W']);
        rig.press(Key::Ok);
        assert_eq!(state(&rig).shown().count(), 4);
    }

    #[test]
    fn down_scrolls_until_the_oldest_entry_shows() {
        let many: String = (0..200)
            .map(|i| format!("11-15 12:00:00.000  1  1 I lume    : line {i}\n"))
            .collect();
        let mut rig = opened_with(&many);
        assert!(!state(&rig).at_end);
        for _ in 0..500 {
            rig.press(Key::Down);
            rig.frame();
        }
        let top = state(&rig).top;
        assert!(state(&rig).at_end);
        assert!(top > 150 && top < 200, "stopped at {top}");
        rig.press(Key::Up);
        assert_eq!(state(&rig).top, top - 1);
    }

    #[test]
    fn back_leaves_and_the_lines_go_with_the_tool() {
        let mut rig = opened_with(SAMPLE);
        rig.press(Key::Back);
        assert_eq!(rig.app.screen(), &Screen::Tools);
        rig.frame();
        assert!(rig.app.tool_ref::<LogTool>().is_none());
    }

    #[test]
    fn a_log_that_cannot_be_read_says_so() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Log);
        let now = rig.now;
        rig.app
            .tool_event(ToolEvent::Test(TestUpdate::Log(LogUpdate::Failed)), now);
        rig.frame();
        assert!(state(&rig).failed);
    }
}
