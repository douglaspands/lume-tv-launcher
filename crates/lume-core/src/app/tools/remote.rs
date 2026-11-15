//! The Remote control tool (`remote-mapping`): every button pressed, with its name, code and
//! repeat (task 4.1, D10).
//!
//! While it is open every raw keycode comes here before the key mapping, and none of them runs
//! its action: the tool only says what lume would do. Back is the way out, but only pressed twice
//! within 2 s, so a single Back can be tested too. Home never comes here as a key of the tool: it
//! does what it always does.

use super::common::{self, draw_panel, panel_heading, panel_row, PANEL_ROW_PITCH};
use super::{ToolKey, ToolKind};
use crate::app::page::LEFT;
use crate::app::painter::Painter;
use crate::app::{Key, LumeApp, Screen};
use crate::i18n::{Arg, StringId, Translator};
use crate::keys::{self, KeyAction};
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{StateFlags, THEME};
use std::collections::VecDeque;

/// How many keys the history keeps.
pub const HISTORY_LEN: usize = 8;
/// Two Back presses closer than this close the tool.
pub const DOUBLE_BACK_MS: u64 = 2000;
/// The same code again within this time is a repeat of a held button: Android repeats a held key
/// after about 500 ms, and then every 50 ms or so (an IR remote, every 100 ms or so).
pub const REPEAT_GAP_MS: u64 = 700;

/// The panels: the last key at the left (heading and five rows), the history at the right
/// (heading and eight rows). The rows start 72 below the top, as the statistics panel.
const ROWS_TOP: f32 = 72.0;
const LAST_PANEL_HEIGHT: f32 = 280.0;
const HISTORY_PANEL_HEIGHT: f32 = 400.0;
/// The hint under the last key panel.
const HINT_GAP: f32 = 24.0;
const HINT_HEIGHT: f32 = 24.0;

/// The button pressed last, with its repeats while it is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Press {
    code: i32,
    /// When it was pressed.
    at: u64,
    /// How many times it repeated, and when the first and the last repeat came.
    repeats: u32,
    first_repeat_at: u64,
    last_at: u64,
}

impl Press {
    /// The average time between repeats, in ms. The wait before the first repeat (the hold delay)
    /// counts only while it is the only interval there is.
    fn average_interval(&self) -> Option<u64> {
        match self.repeats {
            0 => None,
            1 => Some(self.last_at - self.at),
            n => Some((self.last_at - self.first_repeat_at) / (n as u64 - 1)),
        }
    }
}

/// A line of the history: a button and how many times it repeated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    code: i32,
    repeats: u32,
}

/// The tool's state, alive only while it is open.
#[derive(Debug, Default)]
pub struct RemoteTool {
    last: Option<Press>,
    /// The newest first, at most [`HISTORY_LEN`].
    history: VecDeque<Entry>,
    /// When Back was pressed, while a second Back would close the tool.
    back_at: Option<u64>,
}

impl RemoteTool {
    pub(in crate::app) fn new(_now_ms: u64) -> RemoteTool {
        RemoteTool::default()
    }

    /// Records a code: a repeat of the button pressed last, or a new press.
    fn record(&mut self, code: i32, now_ms: u64) {
        if let Some(last) = self.last.as_mut() {
            if last.code == code && now_ms.saturating_sub(last.last_at) <= REPEAT_GAP_MS {
                last.repeats += 1;
                if last.repeats == 1 {
                    last.first_repeat_at = now_ms;
                }
                last.last_at = now_ms;
                if let Some(entry) = self.history.front_mut() {
                    entry.repeats = last.repeats;
                }
                return;
            }
        }
        self.last = Some(Press {
            code,
            at: now_ms,
            repeats: 0,
            first_repeat_at: now_ms,
            last_at: now_ms,
        });
        self.history.push_front(Entry { code, repeats: 0 });
        self.history.truncate(HISTORY_LEN);
    }

    /// Whether a Back now would be the second one.
    fn back_pending(&self, now_ms: u64) -> bool {
        self.back_at
            .is_some_and(|at| now_ms.saturating_sub(at) < DOUBLE_BACK_MS)
    }
}

/// The name of an Android keycode as in `KeyEvent` (`KEYCODE_DPAD_DOWN`), for the codes a TV
/// remote or a keyboard may send.
pub fn android_name(code: i32) -> Option<&'static str> {
    const LETTERS: [&str; 26] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    const FUNCTIONS: [&str; 12] = [
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    ];
    Some(match code {
        3 => "HOME",
        4 => "BACK",
        5 => "CALL",
        6 => "ENDCALL",
        7..=16 => DIGITS[(code - 7) as usize],
        17 => "STAR",
        18 => "POUND",
        19 => "DPAD_UP",
        20 => "DPAD_DOWN",
        21 => "DPAD_LEFT",
        22 => "DPAD_RIGHT",
        23 => "DPAD_CENTER",
        24 => "VOLUME_UP",
        25 => "VOLUME_DOWN",
        26 => "POWER",
        27 => "CAMERA",
        28 => "CLEAR",
        29..=54 => LETTERS[(code - 29) as usize],
        55 => "COMMA",
        56 => "PERIOD",
        61 => "TAB",
        62 => "SPACE",
        66 => "ENTER",
        67 => "DEL",
        82 => "MENU",
        83 => "NOTIFICATION",
        84 => "SEARCH",
        85 => "MEDIA_PLAY_PAUSE",
        86 => "MEDIA_STOP",
        87 => "MEDIA_NEXT",
        88 => "MEDIA_PREVIOUS",
        89 => "MEDIA_REWIND",
        90 => "MEDIA_FAST_FORWARD",
        91 => "MUTE",
        92 => "PAGE_UP",
        93 => "PAGE_DOWN",
        96 => "BUTTON_A",
        97 => "BUTTON_B",
        99 => "BUTTON_X",
        100 => "BUTTON_Y",
        108 => "BUTTON_START",
        109 => "BUTTON_SELECT",
        111 => "ESCAPE",
        122 => "MOVE_HOME",
        123 => "MOVE_END",
        126 => "MEDIA_PLAY",
        127 => "MEDIA_PAUSE",
        128 => "MEDIA_CLOSE",
        129 => "MEDIA_EJECT",
        130 => "MEDIA_RECORD",
        131..=142 => FUNCTIONS[(code - 131) as usize],
        160 => "NUMPAD_ENTER",
        164 => "VOLUME_MUTE",
        165 => "INFO",
        166 => "CHANNEL_UP",
        167 => "CHANNEL_DOWN",
        168 => "ZOOM_IN",
        169 => "ZOOM_OUT",
        170 => "TV",
        171 => "WINDOW",
        172 => "GUIDE",
        173 => "DVR",
        174 => "BOOKMARK",
        175 => "CAPTIONS",
        176 => "SETTINGS",
        177 => "TV_POWER",
        178 => "TV_INPUT",
        179 => "STB_POWER",
        180 => "STB_INPUT",
        181 => "AVR_POWER",
        182 => "AVR_INPUT",
        183 => "PROG_RED",
        184 => "PROG_GREEN",
        185 => "PROG_YELLOW",
        186 => "PROG_BLUE",
        187 => "APP_SWITCH",
        206 => "3D_MODE",
        219 => "ASSIST",
        220 => "BRIGHTNESS_DOWN",
        221 => "BRIGHTNESS_UP",
        222 => "MEDIA_AUDIO_TRACK",
        223 => "SLEEP",
        224 => "WAKEUP",
        225 => "PAIRING",
        226 => "LAST_CHANNEL",
        229 => "MEDIA_TOP_MENU",
        231 => "VOICE_ASSIST",
        243 => "TV_INPUT_HDMI_1",
        244 => "TV_INPUT_HDMI_2",
        245 => "TV_INPUT_HDMI_3",
        246 => "TV_INPUT_HDMI_4",
        272 => "MEDIA_SKIP_FORWARD",
        273 => "MEDIA_SKIP_BACKWARD",
        274 => "MEDIA_STEP_FORWARD",
        275 => "MEDIA_STEP_BACKWARD",
        276 => "SOFT_SLEEP",
        _ => return None,
    })
}

/// The code as the tool shows it: `KEYCODE_DPAD_DOWN (20)`, or only the number when it has no
/// known name.
pub fn code_text(code: i32) -> String {
    match android_name(code) {
        Some(name) => format!("KEYCODE_{name} ({code})"),
        None => code.to_string(),
    }
}

/// The name of the button: the navigation keys by what they are on the remote, the others as the
/// key mapping names them ("Favorito", "Tecla 999").
fn key_name(code: i32, t: &Translator) -> String {
    let id = match code {
        keys::KEYCODE_DPAD_UP => StringId::ToolRemoteKeyUp,
        keys::KEYCODE_DPAD_DOWN => StringId::ToolRemoteKeyDown,
        keys::KEYCODE_DPAD_LEFT => StringId::ToolRemoteKeyLeft,
        keys::KEYCODE_DPAD_RIGHT => StringId::ToolRemoteKeyRight,
        keys::KEYCODE_DPAD_CENTER => StringId::ToolRemoteKeyOk,
        keys::KEYCODE_BACK => StringId::ToolRemoteKeyBack,
        keys::KEYCODE_HOME => StringId::ToolRemoteKeyHome,
        keys::KEYCODE_POWER | keys::KEYCODE_TV_POWER => StringId::ToolRemoteKeyPower,
        _ => return keys::keycode_parts(code, t).0,
    };
    t.get(id).into_owned()
}

/// The power and volume keys: the system's, whatever the mapping says.
fn is_system_code(code: i32) -> bool {
    keys::POWER_KEYCODES.contains(&code)
        || matches!(
            code,
            keys::KEYCODE_VOLUME_UP | keys::KEYCODE_VOLUME_DOWN | keys::KEYCODE_VOLUME_MUTE
        )
}

impl LumeApp {
    /// Whether the remote control test is open: then every key goes to lume (also the ones it
    /// does not use), so the test can show them.
    pub fn is_remote_test_open(&self) -> bool {
        self.screen == Screen::Tool(ToolKind::Remote)
    }

    /// What lume does with `code` by the current mapping, in words, without doing it.
    fn remote_action_text(&self, code: i32) -> String {
        let t = self.translator;
        if is_system_code(code) {
            return t.get(StringId::ToolRemoteReserved).into_owned();
        }
        let action = self
            .keys
            .action_for(code)
            .or_else(|| keys::from_android(code).map(KeyAction::Key));
        let opens = |name: &str| t.format(StringId::ToolRemoteOpens, &[Arg::Str(name)]);
        let id = match action {
            None => StringId::ToolRemoteUnused,
            Some(KeyAction::Key(key)) => match key {
                Key::Up => StringId::ToolRemoteActionUp,
                Key::Down => StringId::ToolRemoteActionDown,
                Key::Left => StringId::ToolRemoteActionLeft,
                Key::Right => StringId::ToolRemoteActionRight,
                Key::Ok => StringId::ToolRemoteActionOk,
                Key::Back => StringId::ToolRemoteActionBack,
                Key::Home => StringId::ToolRemoteActionHome,
                Key::Menu => return opens(&t.get(StringId::CommandMenu)),
            },
            Some(KeyAction::Inputs) => return opens(&t.get(StringId::CommandInputs)),
            Some(KeyAction::Setting(setting)) => return opens(&t.get(setting.title_id())),
            Some(KeyAction::App(package)) => {
                let name = self
                    .catalog
                    .get(&package)
                    .map(|app| app.label.clone())
                    .unwrap_or(package);
                return opens(&name);
            }
        };
        t.get(id).into_owned()
    }

    pub(in crate::app) fn draw_remote(&mut self, p: &mut Painter) {
        self.draw_tool_items(p, &[], 0.0);
        self.draw_tool_header(p, ToolKind::Remote);
        let Some(state) = self.tool_ref::<RemoteTool>() else {
            return;
        };
        let t = self.translator;
        let now = self.now_ms;

        // The last key, at the left.
        let panel = Rect::new(
            LEFT,
            common::ITEMS_TOP,
            common::COLUMN_WIDTH,
            LAST_PANEL_HEIGHT,
        );
        let inner = draw_panel(p, panel);
        let dash = "—".to_string();
        let rows = match state.last {
            Some(last) => {
                panel_heading(p, inner, &t.get(StringId::ToolRemoteLast));
                [
                    (StringId::ToolRemoteKey, key_name(last.code, &t)),
                    (StringId::ToolRemoteCode, code_text(last.code)),
                    (
                        StringId::ToolRemoteAction,
                        self.remote_action_text(last.code),
                    ),
                    (StringId::ToolRemoteRepeats, last.repeats.to_string()),
                    (
                        StringId::ToolRemoteInterval,
                        last.average_interval()
                            .map(|ms| format!("{ms} ms"))
                            .unwrap_or_else(|| dash.clone()),
                    ),
                ]
            }
            None => {
                panel_heading(p, inner, &t.get(StringId::ToolRemoteWaiting));
                [
                    (StringId::ToolRemoteKey, dash.clone()),
                    (StringId::ToolRemoteCode, dash.clone()),
                    (StringId::ToolRemoteAction, dash.clone()),
                    (StringId::ToolRemoteRepeats, dash.clone()),
                    (StringId::ToolRemoteInterval, dash.clone()),
                ]
            }
        };
        for (index, (label, value)) in rows.iter().enumerate() {
            let line = Rect::new(
                inner.x,
                panel.y + ROWS_TOP + index as f32 * PANEL_ROW_PITCH,
                inner.w,
                common::PANEL_ROW_HEIGHT,
            );
            panel_row(p, line, &t.get(*label), value);
        }

        // The way out, under it: after a first Back, that one more closes the tool.
        let hint = if state.back_pending(now) {
            StringId::ToolRemoteBackAgain
        } else {
            StringId::ToolRemoteHint
        };
        let style = p.style(&THEME.breadcrumb, StateFlags::NONE);
        let line = Rect::new(
            LEFT,
            panel.bottom() + HINT_GAP,
            common::COLUMN_WIDTH,
            HINT_HEIGHT,
        );
        p.text(&t.get(hint), line, Align::Start, &style);

        // The history, at the right: the newest at the top.
        let panel = common::panel_rect(HISTORY_PANEL_HEIGHT);
        let inner = draw_panel(p, panel);
        panel_heading(p, inner, &t.get(StringId::ToolRemoteHistory));
        for (index, entry) in state.history.iter().enumerate() {
            let line = Rect::new(
                inner.x,
                panel.y + ROWS_TOP + index as f32 * PANEL_ROW_PITCH,
                inner.w,
                common::PANEL_ROW_HEIGHT,
            );
            let name = if entry.repeats > 0 {
                format!("{} ×{}", key_name(entry.code, &t), entry.repeats + 1)
            } else {
                key_name(entry.code, &t)
            };
            panel_row(p, line, &name, &code_text(entry.code));
        }
    }

    /// A key from the desktop viewer (the TV sends codes, see [`LumeApp::remote_keycode`]): it is
    /// tested as the first code the remote sends for it.
    pub(in crate::app) fn remote_key(&mut self, key: Key) -> ToolKey {
        match key.default_codes().first() {
            Some(&code) => self.remote_keycode(code),
            None => ToolKey::Handled(vec![]),
        }
    }

    /// A frame when a first Back stops waiting for the second one (the hint changes back).
    pub(in crate::app) fn remote_deadline(&self, now_ms: u64) -> Option<u64> {
        let state = self.tool_ref::<RemoteTool>()?;
        state
            .back_at
            .map(|at| at + DOUBLE_BACK_MS)
            .filter(|at| *at > now_ms)
    }

    /// Every raw keycode while this tool is open, before the key mapping (D10): it is recorded and
    /// nothing runs. Back twice within 2 s closes the tool; Home does what it always does.
    pub(in crate::app) fn remote_keycode(&mut self, code: i32) -> ToolKey {
        let now = self.now_ms;
        let is_home = code == keys::KEYCODE_HOME || self.keys.from_android(code) == Some(Key::Home);
        if is_home {
            return ToolKey::Handled(self.handle_key(Key::Home, now));
        }
        let is_back = keys::is_back_code(code) || self.keys.from_android(code) == Some(Key::Back);
        let Some(state) = self.tool_mut::<RemoteTool>() else {
            return ToolKey::Default;
        };
        if is_back && state.back_pending(now) {
            return ToolKey::Exit(vec![]);
        }
        state.back_at = is_back.then_some(now);
        state.record(code, now);
        ToolKey::Handled(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};
    use crate::keys::KeyTarget;

    fn opened() -> Rig {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Remote);
        rig
    }

    /// A raw code at `rig.now`, then `after` ms pass.
    fn code(rig: &mut Rig, code: i32, after: u64) -> Vec<crate::app::Effect> {
        let effects = rig.app.handle_keycode(code, rig.now);
        rig.advance(after);
        effects
    }

    fn state(rig: &Rig) -> &RemoteTool {
        rig.app
            .tool_ref::<RemoteTool>()
            .expect("the remote tool is not open")
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("ROWS_TOP", ROWS_TOP),
            ("LAST_PANEL_HEIGHT", LAST_PANEL_HEIGHT),
            ("HISTORY_PANEL_HEIGHT", HISTORY_PANEL_HEIGHT),
            ("HINT_GAP", HINT_GAP),
            ("HINT_HEIGHT", HINT_HEIGHT),
        ]);
        // Five rows fit the last key panel and eight the history, with the bottom margin.
        let last_row = ROWS_TOP + 4.0 * PANEL_ROW_PITCH + common::PANEL_ROW_HEIGHT;
        assert!(last_row + common::PANEL_PAD_TOP <= LAST_PANEL_HEIGHT);
        let last_row = ROWS_TOP + 7.0 * PANEL_ROW_PITCH + common::PANEL_ROW_HEIGHT;
        assert!(last_row + common::PANEL_PAD_TOP <= HISTORY_PANEL_HEIGHT);
        const { assert!(common::ITEMS_TOP + HISTORY_PANEL_HEIGHT <= 720.0) };
    }

    #[test]
    fn a_key_mapped_to_an_app_is_shown_and_does_not_open_it() {
        let mut rig = opened();
        let red = keys::KEYCODE_PROG_RED;
        assert!(rig
            .app
            .keys
            .add_code(&KeyTarget::App("com.demo.chrome".into()), red));
        let effects = code(&mut rig, red, 100);
        assert!(effects.is_empty(), "nothing runs: {effects:?}");
        assert_eq!(rig.app.screen(), &Screen::Tool(ToolKind::Remote));
        let t = rig.app.translator;
        assert_eq!(key_name(red, &t), "Vermelho");
        assert_eq!(code_text(red), "KEYCODE_PROG_RED (183)");
        assert_eq!(rig.app.remote_action_text(red), "Abre Chrome");
    }

    #[test]
    fn arrows_and_ok_move_nothing() {
        let mut rig = opened();
        for c in [
            keys::KEYCODE_DPAD_DOWN,
            keys::KEYCODE_DPAD_UP,
            keys::KEYCODE_DPAD_CENTER,
            keys::KEYCODE_MENU,
        ] {
            assert!(code(&mut rig, c, 1000).is_empty());
            assert_eq!(rig.app.screen(), &Screen::Tool(ToolKind::Remote));
        }
        assert!(!rig.app.menu_is_open(), "Menu opens no context menu");
        assert_eq!(state(&rig).history.len(), 4);
        assert_eq!(
            rig.app.remote_action_text(keys::KEYCODE_MENU),
            "Abre Menu de contexto"
        );
        assert_eq!(code_text(keys::KEYCODE_BOOKMARK), "KEYCODE_BOOKMARK (174)");
    }

    #[test]
    fn an_unknown_code_shows_its_number_and_unused() {
        let mut rig = opened();
        code(&mut rig, 999, 100);
        assert_eq!(state(&rig).last.map(|l| l.code), Some(999));
        assert_eq!(code_text(999), "999");
        assert_eq!(key_name(999, &rig.app.translator), "Tecla 999");
        assert_eq!(rig.app.remote_action_text(999), "Não usada");
        assert_eq!(
            rig.app.remote_action_text(keys::KEYCODE_VOLUME_UP),
            "Reservada ao sistema"
        );
    }

    #[test]
    fn a_held_button_counts_repeats_and_their_average_interval() {
        let mut rig = opened();
        // Pressed, the first repeat 500 ms later, then one every 50 ms: 2 s held.
        code(&mut rig, keys::KEYCODE_DPAD_DOWN, 500);
        for _ in 0..30 {
            code(&mut rig, keys::KEYCODE_DPAD_DOWN, 50);
        }
        let last = state(&rig).last.unwrap();
        assert_eq!(last.repeats, 30);
        assert_eq!(last.average_interval(), Some(50));
        assert_eq!(
            state(&rig).history.iter().copied().collect::<Vec<_>>(),
            vec![Entry {
                code: keys::KEYCODE_DPAD_DOWN,
                repeats: 30
            }],
            "a held button is one line"
        );
        // Let go and pressed again later: a new press.
        rig.advance(1000);
        code(&mut rig, keys::KEYCODE_DPAD_DOWN, 100);
        assert_eq!(state(&rig).last.unwrap().repeats, 0);
        assert_eq!(state(&rig).history.len(), 2);
    }

    #[test]
    fn a_single_repeat_shows_the_hold_delay() {
        let mut rig = opened();
        code(&mut rig, keys::KEYCODE_DPAD_UP, 500);
        code(&mut rig, keys::KEYCODE_DPAD_UP, 100);
        assert_eq!(state(&rig).last.unwrap().average_interval(), Some(500));
    }

    #[test]
    fn a_single_back_is_only_recorded() {
        let mut rig = opened();
        assert!(code(&mut rig, keys::KEYCODE_BACK, 2500).is_empty());
        assert_eq!(rig.app.screen(), &Screen::Tool(ToolKind::Remote));
        code(&mut rig, keys::KEYCODE_BACK, 100);
        assert_eq!(
            rig.app.screen(),
            &Screen::Tool(ToolKind::Remote),
            "more than 2 s after the first"
        );
        assert_eq!(state(&rig).history.len(), 2);
    }

    #[test]
    fn back_twice_within_two_seconds_closes_the_tool() {
        let mut rig = opened();
        code(&mut rig, keys::KEYCODE_BACK, 0);
        assert_eq!(
            rig.app.remote_deadline(rig.now),
            Some(rig.now + DOUBLE_BACK_MS),
            "a frame when the hint changes back"
        );
        rig.advance(1000);
        code(&mut rig, keys::KEYCODE_BACK, 100);
        assert_eq!(rig.app.screen(), &Screen::Tools);
        assert_eq!(rig.focused(), Some(ToolKind::Remote.menu_id()));
        assert!(rig.app.tool.is_none());
    }

    #[test]
    fn another_key_between_two_backs_keeps_the_tool_open() {
        let mut rig = opened();
        code(&mut rig, keys::KEYCODE_BACK, 100);
        code(&mut rig, keys::KEYCODE_DPAD_LEFT, 100);
        code(&mut rig, keys::KEYCODE_BACK, 100);
        assert_eq!(rig.app.screen(), &Screen::Tool(ToolKind::Remote));
    }

    #[test]
    fn home_does_what_it_always_does() {
        let mut rig = opened();
        code(&mut rig, keys::KEYCODE_HOME, 100);
        assert_eq!(rig.app.screen(), &Screen::Main(crate::app::Tab::Home));
        assert!(rig.app.tool.is_none());
    }

    #[test]
    fn the_history_keeps_the_last_eight() {
        let mut rig = opened();
        for c in 0..10 {
            code(&mut rig, keys::KEYCODE_0 + c, 1000);
        }
        let history = &state(&rig).history;
        assert_eq!(history.len(), HISTORY_LEN);
        assert_eq!(history.front().unwrap().code, keys::KEYCODE_9);
        assert_eq!(history.back().unwrap().code, keys::KEYCODE_2);
    }

    #[test]
    fn the_desktop_keys_are_tested_as_their_codes() {
        let mut rig = opened();
        rig.press(Key::Down);
        assert_eq!(
            state(&rig).last.map(|l| l.code),
            Some(keys::KEYCODE_DPAD_DOWN)
        );
        rig.press(Key::Back);
        assert_eq!(rig.app.screen(), &Screen::Tool(ToolKind::Remote));
        rig.press(Key::Back);
        assert_eq!(rig.app.screen(), &Screen::Tools);
    }
}
