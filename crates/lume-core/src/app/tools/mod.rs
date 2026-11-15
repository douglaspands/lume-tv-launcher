//! The Tools: a menu in the lume settings with five diagnostic tools (`diagnostic-tools` spec).
//!
//! Everything here exists only while it is used (D1 in the `ferramentas` design): the state of the
//! open tool lives in [`LumeApp::tool`] and is dropped when the tool closes; the heavy resources
//! (audio stream, test thread, temporary file) live on the platform side and are asked
//! for with an [`Effect`] (D2).
//!
//! Every tool lives in its own file, with hooks on [`LumeApp`] (`draw_<tool>` and `<tool>_key`
//! always; `<tool>_open`, `<tool>_deadline`, `<tool>_event` and `<tool>_fullscreen` only when the tool
//! needs them), which this module dispatches to; its state is reached with [`LumeApp::tool_ref`] and
//! [`LumeApp::tool_mut`].

pub mod audio;
pub(super) mod common;
pub mod display;
mod menu;
pub mod picture;
pub mod remote;
pub mod storage;

use super::{ids, Effect, Key, LumeApp, PendingFocus, Screen};
use crate::i18n::StringId;
use crate::ui::focus::FocusId;

/// The tools, in the order of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ToolKind {
    /// "Tela": test patterns, image retention cleaning and the stuck pixel fixer.
    Display,
    /// "Imagem": overscan and geometry, and motion.
    Picture,
    /// "Controle remoto": every button pressed, with its code and repeat.
    Remote,
    /// "Áudio": channels, sweep and audio/video sync.
    Audio,
    /// "Armazenamento": sequential write and read speed.
    Storage,
}

impl ToolKind {
    pub const ALL: [ToolKind; 5] = [
        ToolKind::Display,
        ToolKind::Picture,
        ToolKind::Remote,
        ToolKind::Audio,
        ToolKind::Storage,
    ];

    pub fn name(self) -> StringId {
        match self {
            ToolKind::Display => StringId::ToolDisplay,
            ToolKind::Picture => StringId::ToolPicture,
            ToolKind::Remote => StringId::ToolRemote,
            ToolKind::Audio => StringId::ToolAudio,
            ToolKind::Storage => StringId::ToolStorage,
        }
    }

    pub fn description(self) -> StringId {
        match self {
            ToolKind::Display => StringId::ToolDisplayDesc,
            ToolKind::Picture => StringId::ToolPictureDesc,
            ToolKind::Remote => StringId::ToolRemoteDesc,
            ToolKind::Audio => StringId::ToolAudioDesc,
            ToolKind::Storage => StringId::ToolStorageDesc,
        }
    }

    /// The position in [`ToolKind::ALL`] (the variants are declared in the same order).
    fn index(self) -> usize {
        self as usize
    }

    /// The menu item of the tool.
    pub(super) fn menu_id(self) -> FocusId {
        ids::TOOLS_BASE + self.index() as FocusId
    }

    pub(super) fn from_menu_id(id: FocusId) -> Option<ToolKind> {
        ToolKind::ALL.into_iter().find(|k| k.menu_id() == id)
    }

    /// The first focus id of the tool's own range (`ids::TOOL_BASE` + 100 per tool): each tool
    /// numbers its items from here, up to 99 of them.
    pub fn first_id(self) -> FocusId {
        ids::TOOL_BASE + 100 * self.index() as FocusId
    }
}

/// Which audio output a tone goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    Left,
    Right,
    Both,
}

/// What the audio tool asks the platform to play (D2). The platform opens the stream on the first
/// command and closes it on [`AudioCommand::Stop`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioCommand {
    /// A steady sine tone.
    Tone { channel: Channel, hz: u32 },
    /// The logarithmic sweep from 20 Hz to 20 kHz, in both channels.
    Sweep,
    /// A 1 kHz beep of 50 ms every `period_ms`, for the audio/video sync test.
    SyncBeep { period_ms: u32 },
    /// Closes the stream.
    Stop,
}

/// A test that runs on the platform's `lume-test` thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TestKind {
    /// The storage write and read test (`storage-diagnostics`).
    Storage,
}

/// What the platform tells the open tool.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolEvent {
    /// Progress or result of the test on the `lume-test` thread.
    Test(TestUpdate),
    /// The first audio frame of a sync beep left the speakers at this time (`now_ms` clock, D8).
    AudioStarted { at_ms: u64 },
    /// The audio stream could not be opened ("Áudio indisponível").
    AudioUnavailable,
    /// The answer to a periodic reading asked with [`LumeApp::tool_reading_deadline`].
    Reading(ToolReading),
}

/// Progress of a test, by test.
#[derive(Clone, Debug, PartialEq)]
pub enum TestUpdate {
    Storage(storage::StorageUpdate),
}

/// A periodic reading the platform makes for a tool (only with the tool on screen and the
/// interface at rest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadingKind {
    /// Free and total space of the data partition.
    Storage,
}

/// The answer to a [`ReadingKind`].
#[derive(Clone, Debug, PartialEq)]
pub enum ToolReading {
    /// `None`: the system would not tell.
    Storage(Option<storage::StorageSpace>),
}

/// The state of the open tool (D1): created when it opens, dropped when it closes.
pub(super) enum ToolState {
    Display(display::DisplayTool),
    Picture(picture::PictureTool),
    Remote(remote::RemoteTool),
    Audio(audio::AudioTool),
    Storage(storage::StorageTool),
}

/// The state of one tool, reached from [`ToolState`] by its type.
pub(super) trait ToolOf {
    fn of(state: &ToolState) -> Option<&Self>;
    fn of_mut(state: &mut ToolState) -> Option<&mut Self>;
}

macro_rules! tool_of {
    ($($variant:ident => $ty:ty),* $(,)?) => {$(
        impl ToolOf for $ty {
            fn of(state: &ToolState) -> Option<&Self> {
                match state {
                    ToolState::$variant(s) => Some(s),
                    _ => None,
                }
            }

            fn of_mut(state: &mut ToolState) -> Option<&mut Self> {
                match state {
                    ToolState::$variant(s) => Some(s),
                    _ => None,
                }
            }
        }
    )*};
}

tool_of!(
    Display => display::DisplayTool,
    Picture => picture::PictureTool,
    Remote => remote::RemoteTool,
    Audio => audio::AudioTool,
    Storage => storage::StorageTool,
);

impl ToolState {
    fn new(kind: ToolKind, now_ms: u64) -> ToolState {
        match kind {
            ToolKind::Display => ToolState::Display(display::DisplayTool::new(now_ms)),
            ToolKind::Picture => ToolState::Picture(picture::PictureTool::new(now_ms)),
            ToolKind::Remote => ToolState::Remote(remote::RemoteTool::new(now_ms)),
            ToolKind::Audio => ToolState::Audio(audio::AudioTool::new(now_ms)),
            ToolKind::Storage => ToolState::Storage(storage::StorageTool::new(now_ms)),
        }
    }

    fn kind(&self) -> ToolKind {
        match self {
            ToolState::Display(_) => ToolKind::Display,
            ToolState::Picture(_) => ToolKind::Picture,
            ToolState::Remote(_) => ToolKind::Remote,
            ToolState::Audio(_) => ToolKind::Audio,
            ToolState::Storage(_) => ToolKind::Storage,
        }
    }

    /// What the platform must release when the tool closes or lume leaves the screen. Asking to
    /// stop what is not running is harmless.
    fn release_effects(&self) -> Vec<Effect> {
        match self.kind() {
            ToolKind::Audio => vec![Effect::Audio(AudioCommand::Stop)],
            ToolKind::Storage => vec![Effect::CancelTest],
            _ => vec![],
        }
    }
}

/// What a tool did with a key.
pub(super) enum ToolKey {
    /// Not handled: the usual behavior of the screen (arrows move the focus, Back closes the tool).
    Default,
    /// Handled, with these effects.
    Handled(Vec<Effect>),
    /// The tool closes (back to the menu), with these effects.
    Exit(Vec<Effect>),
}

impl ToolKey {
    /// The effects of a key a tool handled, closing the tool on [`ToolKey::Exit`]. `None`: not
    /// handled.
    fn apply(self, app: &mut LumeApp, kind: ToolKind) -> Option<Vec<Effect>> {
        match self {
            ToolKey::Default => None,
            ToolKey::Handled(effects) => Some(effects),
            ToolKey::Exit(effects) => {
                app.back_from_tool(kind);
                Some(effects)
            }
        }
    }
}

impl LumeApp {
    /// The open tool's state, if the open tool is `T`.
    pub(super) fn tool_ref<T: ToolOf>(&self) -> Option<&T> {
        self.tool.as_deref().and_then(T::of)
    }

    /// The open tool's state, mutable, if the open tool is `T`.
    pub(super) fn tool_mut<T: ToolOf>(&mut self) -> Option<&mut T> {
        self.tool.as_deref_mut().and_then(T::of_mut)
    }

    /// Whether the screen is the Tools menu or a tool.
    pub(super) fn in_tools(&self) -> bool {
        matches!(self.screen, Screen::Tools | Screen::Tool(_))
    }

    /// Opens a tool from the menu: its state is created now.
    pub(super) fn open_tool(&mut self, kind: ToolKind) -> Vec<Effect> {
        self.screen = Screen::Tool(kind);
        self.tool = Some(Box::new(ToolState::new(kind, self.now_ms)));
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
        self.focus.set_rows(vec![]);
        match kind {
            ToolKind::Storage => self.storage_open(),
            ToolKind::Display | ToolKind::Picture | ToolKind::Remote | ToolKind::Audio => vec![],
        }
    }

    /// Back from a tool to the menu, with the focus on the tool. The state is dropped (and its
    /// resources released) by [`LumeApp::sync_tools`].
    pub(super) fn back_from_tool(&mut self, kind: ToolKind) {
        self.screen = Screen::Tools;
        self.pending_focus = Some(PendingFocus::Id(kind.menu_id()));
        self.revealed = None;
    }

    /// Keeps the tool state and the "keep the screen on" request in step with the screen, after
    /// anything that may have changed it: leaving a tool releases it, leaving the Tools lets the
    /// screen sleep again.
    pub(super) fn sync_tools(&mut self, effects: &mut Vec<Effect>) {
        let open = match self.screen {
            Screen::Tool(kind) => Some(kind),
            _ => None,
        };
        if self.tool.as_ref().map(|t| t.kind()) != open {
            if let Some(state) = self.tool.take() {
                effects.extend(state.release_effects());
            }
        }
        let in_tools = self.in_tools();
        if in_tools != self.keep_screen_on {
            self.keep_screen_on = in_tools;
            effects.push(Effect::KeepScreenOn(in_tools));
        }
    }

    /// lume left the screen: the running test and the sound stop, and the tool starts over when
    /// lume comes back. Returns what the platform must release.
    pub(super) fn tools_to_background(&mut self) -> Vec<Effect> {
        let Some(state) = self.tool.take() else {
            return vec![];
        };
        let effects = state.release_effects();
        self.tool = Some(Box::new(ToolState::new(state.kind(), self.now_ms)));
        effects
    }

    /// A key on a tool screen. `None`: the screen's usual handling applies.
    pub(super) fn tool_key(&mut self, kind: ToolKind, key: Key) -> Option<Vec<Effect>> {
        let result = match kind {
            ToolKind::Display => self.display_key(key),
            ToolKind::Picture => self.picture_key(key),
            ToolKind::Remote => self.remote_key(key),
            ToolKind::Audio => self.audio_key(key),
            ToolKind::Storage => self.storage_key(key),
        };
        result.apply(self, kind)
    }

    pub(super) fn draw_tool(&mut self, p: &mut super::Painter, kind: ToolKind) {
        match kind {
            ToolKind::Display => self.draw_display(p),
            ToolKind::Picture => self.draw_picture(p),
            ToolKind::Remote => self.draw_remote(p),
            ToolKind::Audio => self.draw_audio(p),
            ToolKind::Storage => self.draw_storage(p),
        }
    }

    /// When the open tool changes by itself: `Some(now)`-like times ask for continuous frames (an
    /// animated mode, D7), later ones for a single frame (a hint fading, a timer ending).
    pub(super) fn tool_deadline(&self, now_ms: u64) -> Option<u64> {
        let Screen::Tool(kind) = self.screen else {
            return None;
        };
        if !self.foreground {
            return None;
        }
        match kind {
            ToolKind::Display => self.display_deadline(now_ms),
            ToolKind::Picture => self.picture_deadline(now_ms),
            ToolKind::Remote => self.remote_deadline(now_ms),
            ToolKind::Audio => self.audio_deadline(now_ms),
            ToolKind::Storage => None,
        }
    }

    /// Whether the open tool covers the whole screen with a pattern (no wallpaper is drawn).
    pub(super) fn tool_fullscreen(&self) -> bool {
        match self.screen {
            Screen::Tool(ToolKind::Display) => self.display_fullscreen(),
            Screen::Tool(ToolKind::Picture) => self.picture_fullscreen(),
            Screen::Tool(ToolKind::Audio) => self.audio_fullscreen(),
            _ => false,
        }
    }

    /// Something the platform tells the open tool. Ignored when no tool, or another one, is open.
    pub fn tool_event(&mut self, event: ToolEvent, now_ms: u64) {
        self.now_ms = self.now_ms.max(now_ms);
        let Screen::Tool(kind) = self.screen else {
            return;
        };
        match kind {
            ToolKind::Audio => self.audio_event(event),
            ToolKind::Storage => self.storage_event(event),
            ToolKind::Display | ToolKind::Picture | ToolKind::Remote => {}
        }
    }

    /// The periodic reading the open tool wants and when, while lume is on screen.
    pub fn tool_reading_deadline(&self, now_ms: u64) -> Option<(ReadingKind, u64)> {
        if !self.foreground {
            return None;
        }
        match self.screen {
            Screen::Tool(ToolKind::Storage) => self
                .storage_reading_deadline(now_ms)
                .map(|at| (ReadingKind::Storage, at)),
            _ => None,
        }
    }

    /// A raw keycode while the remote control test is open: it takes every code before the
    /// mapping (D10). `None` when that tool is not open.
    pub(super) fn tool_keycode(&mut self, code: i32) -> Option<Vec<Effect>> {
        if self.screen != Screen::Tool(ToolKind::Remote) {
            return None;
        }
        let result = self.remote_keycode(code);
        Some(result.apply(self, ToolKind::Remote).unwrap_or_default())
    }

    /// The tool's state, for tests.
    #[cfg(test)]
    pub(super) fn tool_kind(&self) -> Option<ToolKind> {
        self.tool.as_ref().map(|t| t.kind())
    }
}
