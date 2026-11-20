//! The launcher itself: screens, navigation and drawing.
//!
//! [`LumeApp`] holds the state and is driven by three calls from the platform shell:
//! [`LumeApp::handle_key`] for remote control keys, [`LumeApp::draw`] to fill a display list,
//! and [`LumeApp::next_deadline`] to know when something will change without any input
//! (an animation, the clock), so the shell can sleep otherwise.

mod apps;
mod confirm;
mod context_menu;
#[cfg(test)]
mod custom_tests;
mod dock;
mod dock_move;
mod editor;
mod home;
mod keymap;
mod page;
mod painter;
pub mod qr;
mod receive;
mod settings;
mod splash;
#[cfg(test)]
mod tests;
pub mod tools;
mod topbar;

/// Fails when a layout measure is off the 4-unit ruler (see "Régua de layout" in the
/// `design-system` spec).
#[cfg(test)]
fn assert_on_ruler(measures: &[(&str, f32)]) {
    for (name, value) in measures {
        assert!(
            value % 4.0 == 0.0,
            "{name} = {value} is off the 4-unit ruler"
        );
    }
}

pub use context_menu::{ContextMenu, MenuAction};
pub use painter::Painter;

use crate::app_icons::{AppIcons, Art};
use crate::catalog::Catalog;
use crate::clock::WallClock;
use crate::home::config::HomeConfig;
use crate::i18n::Translator;
use crate::render::Rect;
use crate::theme::{AccentColor, AccentPalette, ComputedStyle, Theme, THEME};
use crate::ui::animation::Animations;
use crate::ui::focus::{Direction, Focus, FocusId, Row};
use crate::ui::scroll;
use crate::usage::UsageHistory;

/// The largest an app tile (focused) and a dock icon (at the peak of the Magnification) are drawn,
/// in canvas units: the sizes the icon texture keeps their pictures at (see
/// [`crate::app_icons::AtlasLayout`]).
pub(crate) fn image_peaks() -> (f32, f32) {
    let focused = THEME
        .resolve(&THEME.tile, crate::theme::StateFlags::FOCUS)
        .scale;
    (apps::TILE_WIDTH * focused, dock::BASE * dock::PEAK)
}

/// The size of a dock icon at rest, in canvas units.
#[cfg(test)]
pub(crate) fn dock_rest() -> f32 {
    dock::BASE
}

/// Numbers of the focusable elements. Each screen has its own range.
pub mod ids {
    use crate::ui::focus::FocusId;

    pub const TAB_HOME: FocusId = 1;
    pub const TAB_APPS: FocusId = 2;
    pub const GEAR: FocusId = 3;
    pub const INPUTS: FocusId = 4;
    pub const DOCK_ALL_APPS: FocusId = 998;
    pub const FAVORITES_ADD: FocusId = 999;
    pub const FAVORITE_BASE: FocusId = 1000;
    pub const GRID_BASE: FocusId = 2000;
    pub const GRID_SETTINGS: FocusId = 2999;
    pub const SETTINGS_BASE: FocusId = 3000;
    /// The items of the context menu (only their focus animations use these).
    pub const MENU_BASE: FocusId = 5000;
    /// The items of the Tools menu, one per tool.
    pub const TOOLS_BASE: FocusId = 6000;
    /// The items inside the tools: 100 per tool, in the menu order (`ToolKind::first_id`).
    pub const TOOL_BASE: FocusId = 6100;
    /// The buttons of the flash warning (only their focus animations use these).
    pub const WARNING_START: FocusId = 6900;
    pub const WARNING_CANCEL: FocusId = 6901;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home = 0,
    Apps = 1,
}

pub use crate::keys::Key;

/// What the platform shell has to do for the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    OpenApp {
        package: String,
        activity: String,
    },
    OpenAndroidSettings,
    OpenInputs,
    /// A specific TV or Android settings screen.
    OpenTvSetting {
        setting: crate::keys::SettingShortcut,
    },
    /// The app's page in the Android settings.
    OpenAppInfo {
        package: String,
    },
    /// The system uninstaller for the app (it asks the user; the removal arrives as a package change).
    Uninstall {
        package: String,
    },
    /// The home configuration changed and must be written to disk.
    SaveConfig,
    /// The usage history changed and must be written to disk.
    SaveUsage,
    /// The key mapping changed and must be written to disk.
    SaveKeys,
    /// Keep the screen on (no screen saver, no sleep) while the Tools are open, or stop.
    KeepScreenOn(bool),
    /// Plays or stops a sound of the audio tool.
    Audio(tools::AudioCommand),
    /// Starts a test on the platform's `lume-test` thread (only one runs at a time).
    StartTest(tools::TestKind),
    /// Cancels the running test, if any, and deletes what it left behind.
    CancelTest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main(Tab),
    /// The lume settings (opened with the gear).
    Settings,
    About,
    /// The TV's technical sheet (opened from the settings).
    TvInfo,
    /// Editing the home screen.
    Editor,
    /// Remote control key mapping.
    KeyMap,
    /// Choosing the app a key will open (opened from the key mapping).
    KeyMapPickApp,
    /// Choosing the settings screen a key will open (opened from the key mapping).
    KeyMapPickSetting,
    /// The Tools menu (opened from the settings).
    Tools,
    /// A tool (opened from the Tools menu).
    Tool(tools::ToolKind),
    /// Receiving a picture from the phone (opened from the editor's phone card).
    ReceiveImage,
    /// The picture from the phone in full screen, to apply or not (opened by an upload).
    ConfirmImage,
}

/// What is animated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Anim {
    /// How focused an element is, 0.0 to 1.0.
    Focus(FocusId),
    ScrollY,
    WallpaperScrollX,
    /// The place (fractional while it moves) of the dock icon that is magnified.
    DockPosition,
    /// How much the dock magnifies, 0.0 (the focus is elsewhere) to 1.0.
    DockAmplitude,
    /// How visible the content of the tab is while it fades in after a tab change, 0.0 to 1.0.
    TabFade,
    /// Vertical slide/fade transition for `Slide` tabless mode (250 ms), 0.0 to 1.0.
    SlideTransition,
    /// How visible the context menu is while it opens or closes, 0.0 to 1.0.
    MenuFade,
    /// What is left of a dock icon's slide to its new place, in units (by its place in the dock).
    DockShift(usize),
    /// What is left of a dock icon's change of size on that slide.
    DockGrow(usize),
    /// A favorite that left the dock, shrinking where it was: 1.0 to 0.0.
    DockGhost,
    /// How much room the neighbors of a favorite being moved leave for its arrows, 0.0 to 1.0.
    DockRoom,
    /// How far the reveal of the interface after the splash has got, 0.0 to 1.0.
    Reveal,
    /// How visible the splash logo is while it fades in, 0.0 to 1.0.
    SplashLogo,
    /// How visible an icon that arrived with the interface on screen is, 0.0 to 1.0: the tile cell
    /// `slot`, or the dock cell with `dock`.
    IconIn {
        dock: bool,
        slot: usize,
    },
}

/// The fade of the content when the tab changes (`--tab-fade`).
const TAB_FADE: crate::theme::Transition = crate::theme::vars::TAB_FADE;
/// Vertical transition for `Slide` tabless mode (`--duration-medium1`, 250 ms,
/// `--easing-standard-decelerate`).
const SLIDE_TRANSITION: crate::theme::Transition = crate::theme::Transition {
    duration_ms: crate::theme::vars::DURATION_MEDIUM1.duration_ms,
    curve: crate::theme::vars::FOCUS_TIME.curve,
};
/// Smooth dock ↔ grid transition for `Peek` tabless mode (`--duration-medium2`, 300 ms,
/// `--easing-standard`): eases out of rest instead of snapping on the first frame.
const PEEK_TRANSITION: crate::theme::Transition = crate::theme::Transition {
    duration_ms: crate::theme::vars::DURATION_MEDIUM2.duration_ms,
    curve: crate::theme::Curve::CubicBezier(0.2, 0.0, 0.0, 1.0),
};
/// Vertical travel of the content during `Anim::SlideTransition`, in logical units.
const SLIDE_OFFSET: f32 = 48.0;
/// Scroll distance over which the wallpaper dims progressively in `Peek` mode.
const PEEK_DIM_DISTANCE: f32 = 400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingFocus {
    /// The first element below the top bar.
    FirstContent,
    Id(FocusId),
}

/// Where the user was when they opened another app, to put them back there when they return.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ReturnPoint {
    screen: Screen,
    focus: Option<FocusId>,
    scroll_y: f32,
}

pub struct LumeApp {
    pub catalog: Catalog,
    /// Where each app's icon is in the icon texture.
    pub icons: AppIcons,
    pub usage: UsageHistory,
    pub config: HomeConfig,
    /// Resolved theme for the active accent color (`effective_accent()`).
    active_theme: Theme,
    /// Transient accent color being previewed in the Home screen editor.
    preview_accent: Option<AccentColor>,
    pub keys: crate::keys::KeyMap,
    pub translator: Translator,
    pub clock: WallClock,
    screen: Screen,
    /// Whether the system has a TV inputs panel: the top bar shows its button only then.
    pub inputs_available: bool,
    editor: editor::EditorState,
    pub keymap: keymap::KeyMapState,
    /// The context menu, while it is open or fading out.
    context_menu: Option<ContextMenu>,
    /// The place in the Apps grid of the focused tile, as drawn in the last frame.
    grid_position: Option<usize>,
    /// The favorite being moved along the dock ("Move" in its context menu).
    dock_move: Option<dock_move::DockMove>,
    /// The dock icons as drawn in the last frame.
    dock_drawn: Vec<dock::DrawnIcon>,
    /// Where the icons were before the favorites changed: they slide from there at the next frame.
    dock_from: Option<Vec<dock::DrawnIcon>>,
    /// A favorite that left the dock, fading where it was.
    dock_ghost: Option<dock::DrawnIcon>,
    about_scroll: f32,
    /// What the platform read for the TV information screen, and how far that screen can scroll.
    device_info: Option<crate::device_info::DeviceInfo>,
    tv_max_scroll: f32,
    /// The last TV statistics read while the settings were on screen, and the CPU use from them.
    stats: Option<crate::stats::Stats>,
    cpu_use: Option<f32>,
    /// When the statistics are due again (`None`: right away).
    stats_at: Option<u64>,
    focus: Focus,
    anim: Animations<Anim>,
    scroll_y: f32,
    /// Where the content of the frame starts in the display list (after the wallpaper).
    content_start: usize,
    /// Direction of the active `SlideTransition` (`1.0` sliding into Apps, `-1.0` back to Home).
    slide_dir: f32,
    /// The dock icon the focus is on, when it is in the dock.
    dock_focus: Option<usize>,
    /// The last dock icon that had the focus: its name stays in the bubble while it fades out.
    dock_last: usize,
    /// The focused element whose position the scrolling was last adjusted for.
    revealed: Option<FocusId>,
    /// The element whose focus animation was last started.
    tracked_focus: Option<FocusId>,
    /// Where the focus goes at the next layout (after a screen change).
    pending_focus: Option<PendingFocus>,
    toast: Option<(String, u64)>,
    /// Set when an app was opened; used by the next Home press, dropped by any other key.
    return_point: Option<ReturnPoint>,
    /// Whether lume is on screen. A Home press while it is can only mean "go to the start".
    foreground: bool,
    /// lume just came back to the screen and has not drawn a frame yet.
    wake_pending: bool,
    /// When the first frame after coming back was drawn.
    foreground_since: Option<u64>,
    /// The splash, while it is on screen or the interface is being revealed.
    phase: splash::Phase,
    /// lume has come to the screen before (so the next time is not a cold start).
    woke_before: bool,
    /// The interface became ready since the platform last asked.
    interface_ready: bool,
    /// Where the top bar starts in the display list, on the main screens.
    bar_start: Option<usize>,
    /// The state of the open tool: exists only while a tool is on screen.
    tool: Option<Box<tools::ToolState>>,
    /// The Receive image screen, while it is open (its server lives with it).
    receive: Option<Box<receive::ReceiveState>>,
    /// After Apply: the session that only tells the phone's page, until its server stops.
    lingering: Option<crate::companion::Session>,
    /// The picture the Confirm screen shows, while it is open (and Apply keeping it).
    confirmation: Option<confirm::Confirmation>,
    /// Whether the platform was asked to keep the screen on (the Tools are open).
    keep_screen_on: bool,
    now_ms: u64,
}

/// How long after coming back to the screen a Home press still counts as the one that brought
/// lume back (the system may deliver it after the window is already there).
const RETURN_WINDOW_MS: u64 = 2000;

/// How often the TV statistics are read while the settings are on screen.
const STATS_INTERVAL_MS: u64 = 2000;

impl LumeApp {
    pub fn new(
        catalog: Catalog,
        usage: UsageHistory,
        config: HomeConfig,
        translator: Translator,
        clock: WallClock,
    ) -> LumeApp {
        // What "Minha imagem" is, for the renderer, and its thumbnail for the editor.
        let files = crate::wallpaper::custom::files_dir();
        let custom = files.as_ref().and(config.custom);
        crate::wallpaper::custom::set_applied(custom);
        crate::wallpaper::custom::set_thumbnail(
            custom
                .and(files.as_deref())
                .and_then(crate::wallpaper::custom::load_thumbnail),
        );
        let active_theme = Theme::for_accent(config.accent_color);
        LumeApp {
            catalog,
            icons: AppIcons::default(),
            usage,
            config,
            active_theme,
            preview_accent: None,
            keys: crate::keys::KeyMap::default(),
            translator,
            clock,
            screen: Screen::Main(Tab::Home),
            inputs_available: false,
            editor: editor::EditorState::default(),
            keymap: keymap::KeyMapState::default(),
            context_menu: None,
            grid_position: None,
            dock_move: None,
            dock_drawn: Vec::new(),
            dock_from: None,
            dock_ghost: None,
            about_scroll: 0.0,
            device_info: None,
            tv_max_scroll: 0.0,
            stats: None,
            cpu_use: None,
            stats_at: None,
            focus: Focus::new(),
            anim: Animations::new(),
            scroll_y: 0.0,
            content_start: 0,
            slide_dir: 0.0,
            dock_focus: None,
            dock_last: 0,
            revealed: None,
            tracked_focus: None,
            // The app starts with the focus on the first element of the content.
            pending_focus: Some(PendingFocus::FirstContent),
            toast: None,
            return_point: None,
            foreground: true,
            wake_pending: false,
            foreground_since: None,
            phase: splash::Phase::Normal,
            woke_before: false,
            interface_ready: false,
            bar_start: None,
            tool: None,
            receive: None,
            lingering: None,
            confirmation: None,
            keep_screen_on: false,
            now_ms: 0,
        }
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn focused(&self) -> Option<FocusId> {
        self.focus.current_id()
    }

    /// The tab that is showing, when on a main screen.
    pub fn tab(&self) -> Tab {
        match self.screen {
            Screen::Main(tab) => tab,
            // The settings, About and editor screens all come from the Apps tab.
            _ => Tab::Apps,
        }
    }

    pub(super) fn is_slide_mode(&self) -> bool {
        !self.config.tabs_visible
            && self.config.tabless_mode == crate::home::config::TablessMode::Slide
    }

    pub(super) fn is_peek_mode(&self) -> bool {
        !self.config.tabs_visible
            && self.config.tabless_mode == crate::home::config::TablessMode::Peek
    }

    pub(super) fn is_dock_mode(&self) -> bool {
        !self.config.tabs_visible
            && self.config.tabless_mode == crate::home::config::TablessMode::Dock
    }

    fn is_in_dock(&self) -> bool {
        self.focus.current_id().is_some_and(|id| {
            id == ids::FAVORITES_ADD
                || id == ids::DOCK_ALL_APPS
                || (ids::FAVORITE_BASE..ids::GRID_BASE).contains(&id)
        })
    }

    fn is_on_first_grid_row(&self) -> bool {
        let Some(focused) = self.focus.current_id() else {
            return false;
        };
        if focused < ids::GRID_BASE {
            return false;
        }
        self.focus
            .rows()
            .get(1)
            .is_some_and(|row| row.items.iter().any(|item| item.id == focused))
    }

    fn dock_return_focus(&self) -> PendingFocus {
        let items = self.dock_items();
        items
            .get(self.dock_last)
            .or_else(|| items.first())
            .map_or(PendingFocus::FirstContent, |item| PendingFocus::Id(item.id))
    }

    fn slide_to_apps(&mut self) {
        self.screen = Screen::Main(Tab::Apps);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
        self.slide_dir = 1.0;
        self.anim.animate_to(
            Anim::SlideTransition,
            0.0,
            1.0,
            self.now_ms,
            Some(SLIDE_TRANSITION),
        );
    }

    fn slide_to_home(&mut self) {
        self.screen = Screen::Main(Tab::Home);
        self.scroll_to_top();
        self.pending_focus = Some(self.dock_return_focus());
        self.slide_dir = -1.0;
        self.anim.animate_to(
            Anim::SlideTransition,
            0.0,
            1.0,
            self.now_ms,
            Some(SLIDE_TRANSITION),
        );
    }

    /// The accent color currently displayed (live preview in the editor, or the saved one).
    pub fn effective_accent(&self) -> AccentColor {
        self.preview_accent.unwrap_or(self.config.accent_color)
    }

    /// Rebuilds `active_theme` from the current effective accent color.
    pub fn refresh_theme(&mut self) {
        self.active_theme = Theme::for_accent(self.effective_accent());
    }

    /// Sets or clears the live-preview accent color and refreshes `active_theme`.
    pub fn set_preview_accent(&mut self, accent: Option<AccentColor>) {
        if self.preview_accent != accent {
            self.preview_accent = accent;
            self.refresh_theme();
        }
    }

    /// The resolved theme for the active accent color.
    pub fn active_theme(&self) -> &Theme {
        &self.active_theme
    }

    /// The resolved palette for the active accent color.
    pub fn active_accent(&self) -> AccentPalette {
        self.effective_accent().palette()
    }

    /// The transition the theme gives to focus changes.
    fn focus_transition(&self) -> Option<crate::theme::Transition> {
        self.active_theme.tile.base.transition
    }

    fn style(
        &self,
        component: impl FnOnce(&Theme) -> &crate::theme::ComponentTheme,
    ) -> ComputedStyle {
        self.active_theme.resolve(
            component(&self.active_theme),
            crate::theme::StateFlags::NONE,
        )
    }

    /// Moves the focus in `direction` and returns whether it moved.
    fn move_focus(&mut self, direction: Direction) -> bool {
        let moved = self.focus.move_focus(direction);
        if moved {
            self.sync_editor_preview_accent();
        }
        moved
    }

    /// Handles a key press.
    pub fn handle_key(&mut self, key: Key, now_ms: u64) -> Vec<Effect> {
        self.now_ms = now_ms;

        // Listening for a key: Back cancels and the keys the interface needs are refused.
        if self.is_listening_key() {
            return self.listening_key(key);
        }

        // A key ends the splash at once and goes to the interface. Home does not: it is what brings
        // lume back, and it only decides which screen the splash reveals.
        if key != Key::Home {
            self.cut_splash();
        }
        // Only the Home press that brings lume back can use the place where an app was opened.
        if key != Key::Home {
            self.return_point = None;
        }
        let mut effects = self.dispatch_key(key);
        self.note_return_point(&effects);
        self.sync_tools(&mut effects);
        effects
    }

    /// When the effects take the user out of lume, remembers where they were, to bring them back.
    fn note_return_point(&mut self, effects: &[Effect]) {
        if effects.iter().any(|e| {
            matches!(
                e,
                Effect::OpenApp { .. }
                    | Effect::OpenAndroidSettings
                    | Effect::OpenInputs
                    | Effect::OpenTvSetting { .. }
                    | Effect::OpenAppInfo { .. }
                    | Effect::Uninstall { .. }
            )
        }) {
            self.return_point = Some(ReturnPoint {
                screen: self.screen,
                focus: self.focus.current_id(),
                scroll_y: self.scroll_y,
            });
        }
    }

    /// Handles a raw hardware keycode from the remote control.
    pub fn handle_keycode(&mut self, code: i32, now_ms: u64) -> Vec<Effect> {
        self.now_ms = now_ms;

        // 1. Listening for a key: Back cancels, a reserved key is refused, any other is recorded.
        if self.is_listening_key() {
            return self.listening_code(code);
        }
        // The remote control test sees every code before the mapping, and runs none of them.
        if let Some(mut effects) = self.tool_keycode(code) {
            self.sync_tools(&mut effects);
            return effects;
        }

        // 2. Otherwise check custom mapping table:
        let effects = if let Some(action) = self.keys.action_for(code) {
            match action {
                crate::keys::KeyAction::Key(key) => self.handle_key(key, now_ms),
                crate::keys::KeyAction::Inputs => vec![Effect::OpenInputs],
                crate::keys::KeyAction::Setting(s) => match s {
                    crate::keys::SettingShortcut::LumeSettings => {
                        self.open_settings();
                        vec![]
                    }
                    crate::keys::SettingShortcut::LumeEditor => {
                        self.open_editor(false);
                        vec![]
                    }
                    crate::keys::SettingShortcut::LumeTvInfo => {
                        self.open_tv_info();
                        vec![]
                    }
                    crate::keys::SettingShortcut::Android => vec![Effect::OpenAndroidSettings],
                    other => vec![Effect::OpenTvSetting { setting: other }],
                },
                crate::keys::KeyAction::App(pkg) => {
                    if let Some(app) = self.catalog.get(&pkg) {
                        vec![Effect::OpenApp {
                            package: app.package.clone(),
                            activity: app.activity.clone(),
                        }]
                    } else {
                        vec![]
                    }
                }
            }
        } else if let Some(key) = crate::keys::from_android(code) {
            self.handle_key(key, now_ms)
        } else {
            vec![]
        };
        // A shortcut that leaves lume is something to come back from, like OK on a tile.
        let mut effects = effects;
        self.note_return_point(&effects);
        self.sync_tools(&mut effects);
        effects
    }

    fn dispatch_key(&mut self, key: Key) -> Vec<Effect> {
        // A favorite being moved and an open menu take every key. Home ends them and then does
        // what it always does.
        let trapped = if self.dock_move.is_some() {
            Some(self.dock_move_key(key))
        } else if self.menu_is_open() {
            Some(self.menu_key(key))
        } else {
            None
        };
        match trapped {
            Some(effects) if key != Key::Home => effects,
            Some(mut effects) => {
                effects.extend(self.screen_key(key));
                effects
            }
            None => self.screen_key(key),
        }
    }

    fn screen_key(&mut self, key: Key) -> Vec<Effect> {
        // While Apply keeps the picture, only Home leaves the Confirm screen.
        if self.screen == Screen::ConfirmImage && self.confirm_busy() && key != Key::Home {
            return vec![];
        }
        // A tool takes the keys it wants first; Home always does what it always does.
        if let Screen::Tool(kind) = self.screen {
            if key != Key::Home {
                if let Some(effects) = self.tool_key(kind, key) {
                    return effects;
                }
            }
        }
        match key {
            Key::Up | Key::Down if self.screen == Screen::About => {
                self.scroll_about(if key == Key::Up { -1.0 } else { 1.0 });
                vec![]
            }
            Key::Up | Key::Down if self.screen == Screen::TvInfo => {
                self.scroll_tv_info(if key == Key::Up { -1.0 } else { 1.0 });
                vec![]
            }
            Key::Up => {
                if self.screen == Screen::Main(Tab::Apps)
                    && self.is_slide_mode()
                    && self.is_on_first_grid_row()
                {
                    self.slide_to_home();
                } else {
                    self.move_focus(Direction::Up);
                }
                vec![]
            }
            Key::Down => {
                if self.screen == Screen::Main(Tab::Home)
                    && self.is_slide_mode()
                    && self.is_in_dock()
                {
                    self.slide_to_apps();
                } else {
                    self.move_focus(Direction::Down);
                }
                vec![]
            }
            Key::Left => self.horizontal(Direction::Left),
            Key::Right => self.horizontal(Direction::Right),
            Key::Ok => self.activate(),
            Key::Back => self.back(),
            Key::Home => {
                match self.return_point.take() {
                    // Back from the app that was opened: the screen and the focus as they were.
                    Some(point) if self.just_returned() => self.restore(point),
                    // lume was already on screen: Home is the way back to the start.
                    _ => self.go_home(),
                }
                vec![]
            }
            Key::Menu => {
                self.open_context_menu();
                vec![]
            }
        }
    }

    fn horizontal(&mut self, direction: Direction) -> Vec<Effect> {
        self.move_focus(direction);
        // Moving the focus onto a tab shows that tab at once, without OK. The focus stays on it.
        if self.config.tabs_visible && matches!(self.screen, Screen::Main(_)) {
            match self.focus.current_id() {
                Some(ids::TAB_HOME) => self.select_tab(Tab::Home),
                Some(ids::TAB_APPS) => self.select_tab(Tab::Apps),
                _ => {}
            }
        }
        vec![]
    }

    fn activate(&mut self) -> Vec<Effect> {
        let Some(id) = self.focus.current_id() else {
            return vec![];
        };
        match self.screen {
            Screen::Main(_) => {}
            Screen::Settings => return self.activate_settings(id),
            Screen::About | Screen::TvInfo => return vec![],
            Screen::Editor => return self.activate_editor(id),
            Screen::KeyMap => return self.activate_keymap(id),
            Screen::KeyMapPickApp => return self.activate_pick_app(id),
            Screen::KeyMapPickSetting => return self.activate_pick_setting(id),
            Screen::Tools => return self.activate_tools(id),
            Screen::Tool(_) => return vec![],
            Screen::ReceiveImage => return self.activate_receive(id),
            Screen::ConfirmImage => return self.activate_confirm(id),
        }
        match id {
            ids::GEAR => vec![Effect::OpenAndroidSettings],
            ids::INPUTS => vec![Effect::OpenInputs],
            ids::TAB_HOME if self.config.tabs_visible => {
                self.select_tab(Tab::Home);
                vec![]
            }
            ids::TAB_APPS if self.config.tabs_visible => {
                self.select_tab(Tab::Apps);
                vec![]
            }
            _ => match self.screen {
                Screen::Main(Tab::Home) => self.activate_home(id),
                Screen::Main(Tab::Apps) => self.activate_apps(id),
                _ => vec![],
            },
        }
    }

    fn back(&mut self) -> Vec<Effect> {
        match self.screen {
            Screen::Main(Tab::Home) => {
                if self.is_peek_mode()
                    && (self.focused().is_some_and(|id| id >= ids::GRID_BASE)
                        || self.scroll_y > 0.0)
                {
                    self.revealed = None;
                    self.scroll_to_peek(0.0);
                    self.pending_focus = Some(self.dock_return_focus());
                } else {
                    // Back on the home root scrolls to the top and never closes lume.
                    self.scroll_to_top();
                    self.pending_focus = Some(PendingFocus::FirstContent);
                }
                vec![]
            }
            Screen::Main(Tab::Apps) => {
                if self.is_slide_mode() {
                    self.slide_to_home();
                } else if self.is_dock_mode() {
                    self.select_tab(Tab::Home);
                    self.pending_focus = Some(PendingFocus::Id(ids::DOCK_ALL_APPS));
                } else {
                    self.select_tab(Tab::Home);
                    self.pending_focus = Some(PendingFocus::FirstContent);
                }
                vec![]
            }
            Screen::Settings => {
                self.back_from_settings();
                vec![]
            }
            Screen::About => {
                self.screen = Screen::Settings;
                self.pending_focus = Some(PendingFocus::Id(settings::ITEM_ABOUT));
                vec![]
            }
            Screen::Tools => {
                self.back_from_tools();
                vec![]
            }
            Screen::Tool(kind) => {
                self.back_from_tool(kind);
                vec![]
            }
            Screen::TvInfo => {
                self.back_from_tv_info();
                vec![]
            }
            Screen::Editor => self.back_from_editor(),
            Screen::KeyMap => self.back_from_keymap(),
            Screen::KeyMapPickApp => self.back_from_picker(keymap::ADD_APP),
            Screen::KeyMapPickSetting => self.back_from_picker(keymap::ADD_SETTING),
            Screen::ReceiveImage => self.close_receive(),
            Screen::ConfirmImage => self.cancel_confirm(),
        }
    }

    /// The Home button: back to the Home tab, scrolled to the top, focus on its first element.
    pub fn go_home(&mut self) {
        self.end_receive();
        self.editor = editor::EditorState::default();
        self.set_preview_accent(None);
        self.context_menu = None;
        self.end_dock_move_quietly();
        self.screen = Screen::Main(Tab::Home);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
    }

    /// The catalog changed (apps installed, updated or removed). A tile that left the grid passes
    /// the focus to the tile now in its place, or to the last one; a menu whose app is gone closes.
    pub fn catalog_changed(&mut self) {
        self.check_context_menu();
        self.check_dock_move();
        // A favorite that was uninstalled leaves the dock with the same slide as a removal.
        self.begin_dock_transition(None);
        if self.screen == Screen::Main(Tab::Apps)
            || (self.screen == Screen::Main(Tab::Home) && self.is_peek_mode())
        {
            if let Some(position) = self.grid_position {
                if let Some(id) = self.grid_id_at(position) {
                    self.pending_focus = Some(PendingFocus::Id(id));
                }
            }
        }
    }

    /// lume left the screen (another app is in front).
    pub fn went_to_background(&mut self) -> Vec<Effect> {
        self.foreground = false;
        // No port stays open while another app is in front.
        self.receive_to_background();
        self.tools_to_background()
    }

    /// lume is on the screen again.
    pub fn came_to_foreground(&mut self) {
        self.foreground = true;
        self.receive_to_foreground();
        self.wake_pending = true;
        self.foreground_since = None;
        self.start_splash();
    }

    /// Whether a Home press now is the one that brought lume back from another app: it comes while
    /// lume is not on screen yet, or right after it is.
    fn just_returned(&self) -> bool {
        !self.foreground
            || self.wake_pending
            || self
                .foreground_since
                .is_some_and(|since| self.now_ms.saturating_sub(since) < RETURN_WINDOW_MS)
    }

    /// Puts the screen, the focused element and the scrolling back as they were when an app was opened.
    fn restore(&mut self, point: ReturnPoint) {
        self.editor = editor::EditorState::default();
        self.set_preview_accent(None);
        self.context_menu = None;
        self.end_dock_move_quietly();
        self.screen = point.screen;
        self.scroll_y = point.scroll_y;
        self.anim.set(Anim::ScrollY, point.scroll_y);
        self.revealed = None;
        self.pending_focus = point.focus.map(PendingFocus::Id);
    }

    fn select_tab(&mut self, tab: Tab) {
        if self.screen != Screen::Main(tab) {
            self.screen = Screen::Main(tab);
            self.anim
                .animate_to(Anim::TabFade, 0.0, 1.0, self.now_ms, Some(TAB_FADE));
            self.scroll_to_top();
            // The focus stays on the tab that was selected.
        }
    }

    fn scroll_to_top(&mut self) {
        self.scroll_y = 0.0;
        self.anim.set(Anim::ScrollY, 0.0);
        self.revealed = None;
    }

    /// Opens the app `package` and records it in the usage history.
    pub(crate) fn open_app(&mut self, package: &str) -> Vec<Effect> {
        match self.catalog.get(package) {
            Some(app) => {
                let effect = Effect::OpenApp {
                    package: app.package.clone(),
                    activity: app.activity.clone(),
                };
                self.usage.record_open(package, self.clock.unix_secs);
                vec![effect, Effect::SaveUsage]
            }
            None => {
                self.show_toast(
                    self.translator
                        .get(crate::i18n::StringId::ToastAppNotFound)
                        .into_owned(),
                );
                vec![]
            }
        }
    }

    /// The platform could not open an app that was in the catalog.
    pub fn app_not_found(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        // lume never left: there is nothing to go back to.
        self.return_point = None;
        self.show_toast(
            self.translator
                .get(crate::i18n::StringId::ToastAppNotFound)
                .into_owned(),
        );
    }

    /// The message being shown, if any.
    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().map(|(text, _)| text.as_str())
    }

    /// Moves the focus to `id` directly (for tests of code built on top of the launcher).
    #[doc(hidden)]
    pub fn focus_for_test(&mut self, id: FocusId) -> bool {
        self.focus.focus(id)
    }

    pub(crate) fn show_toast(&mut self, text: String) {
        self.toast = Some((text, self.now_ms + 2500));
    }

    /// When something will change without any key press: the next animation frame, the
    /// clock's minute turning or a message expiring. `None` lets the shell sleep.
    pub fn next_deadline(&self, now_ms: u64) -> Option<u64> {
        // While something was moving at the last frame, the next frame is one frame after that one.
        // It must be a fixed time: counting from `now_ms` it would slide away at every call and the
        // loop would sleep forever without drawing the animation. The frame that finds everything
        // at rest is the last one.
        let mut deadline = self
            .anim
            .is_animating(self.now_ms)
            .then_some(self.now_ms + crate::ui::animation::FRAME_MS);
        let mut consider = |t: u64| deadline = Some(deadline.map_or(t, |d: u64| d.min(t)));
        consider(now_ms + self.clock.millis_to_next_minute());
        if let Some((_, until)) = &self.toast {
            consider(*until);
        }
        if let Some(at) = self.splash_deadline() {
            consider(at);
        }
        if let Some(at) = self.tool_deadline(now_ms) {
            consider(at);
        }
        if let Some(at) = self
            .lingering
            .as_ref()
            .and_then(|s| s.next_wake_ms(self.now_ms))
        {
            consider(at);
        }
        if let Some(at) = self.receive_deadline(now_ms) {
            consider(at);
        }
        deadline
    }

    /// Takes what happened off the loop: the phone companion's news, the end of Apply on the Confirm
    /// screen, and a source of "Minha imagem" found unreadable. `Some` (with what to save) when the
    /// screen changed.
    pub fn poll_background(&mut self, now_ms: u64) -> Option<Vec<Effect>> {
        // `self.now_ms` stays the time of the last frame or key: the next frame of an animation is
        // due one frame after it, and moving it here at every turn of the loop would push that
        // deadline away forever (the splash and the focus would stop moving).
        let mut changed = self.poll_companion(now_ms);
        if let Some(session) = self.lingering.as_mut() {
            session.poll(now_ms);
            if !session.is_listening() {
                self.lingering = None;
            }
        }
        let mut effects = Vec::new();
        if let Some(applied) = self.poll_apply() {
            changed = true;
            effects.extend(applied);
        }
        if crate::wallpaper::custom::take_broken() && self.config.custom.is_some() {
            // The source could not be read: the Aurora, and the card goes back to "+ From phone".
            self.config.forget_custom();
            crate::wallpaper::custom::set_applied(None);
            crate::wallpaper::custom::set_thumbnail(None);
            effects.push(Effect::SaveConfig);
            changed = true;
        }
        changed.then_some(effects)
    }

    /// Whether something off the loop may have news for the screen (the platform asks often).
    pub fn waits_for_background(&self) -> bool {
        self.companion_active() || self.confirm_busy()
    }

    /// Whether the settings are on screen and so the TV statistics are being read.
    pub fn wants_stats(&self) -> bool {
        self.foreground && self.screen == Screen::Settings
    }

    /// When the next statistics reading is due, while the settings are on screen.
    pub fn stats_deadline(&self, now_ms: u64) -> Option<u64> {
        self.wants_stats().then(|| self.stats_at.unwrap_or(now_ms))
    }

    /// A reading failed: ask again after the usual interval.
    pub fn stats_retry(&mut self, now_ms: u64) {
        self.stats_at = Some(now_ms + STATS_INTERVAL_MS);
    }

    /// Takes a statistics reading. The panel is redrawn by the caller.
    pub fn apply_stats(&mut self, stats: crate::stats::Stats, now_ms: u64) {
        self.cpu_use = crate::stats::cpu_use(self.stats.map(|s| s.cpu), stats.cpu);
        self.stats = Some(stats);
        self.stats_at = Some(now_ms + STATS_INTERVAL_MS);
    }

    /// Whether the TV information screen is open and still waits for the platform's reading.
    pub fn wants_device_info(&self) -> bool {
        self.foreground && self.screen == Screen::TvInfo && self.device_info.is_none()
    }

    /// Takes the platform's reading of the TV. The screen is redrawn by the caller.
    pub fn apply_device_info(&mut self, info: crate::device_info::DeviceInfo) {
        self.device_info = Some(info);
    }

    /// Whether an animation is running at `now_ms` (a frame is due every few milliseconds).
    pub fn is_animating(&self, now_ms: u64) -> bool {
        self.anim.is_animating(self.now_ms.max(now_ms))
    }

    /// Whether something is moving that loading an icon would make stutter. The splash logo and the
    /// icons fading in are not: they are faint fades, and icons go on loading during them (in small
    /// batches), or the splash would hold and the icons would come one per fade.
    pub fn is_moving(&self, now_ms: u64) -> bool {
        self.anim
            .is_animating_where(self.now_ms.max(now_ms), |key| {
                !matches!(key, Anim::SplashLogo | Anim::IconIn { .. })
            })
    }

    /// The icon of `package` (its dock icon with `dock`) was just uploaded. With the interface on
    /// screen it fades in over the app's color and initial; during the splash nobody sees it come.
    pub fn icon_arrived(&mut self, package: &str, dock: bool, now_ms: u64) {
        if self.phase != splash::Phase::Normal {
            return;
        }
        let slot = if dock {
            self.icons.dock_slot(package)
        } else {
            self.icons.slot(package)
        };
        if let Some(slot) = slot {
            self.anim.animate_to(
                Anim::IconIn { dock, slot },
                0.0,
                1.0,
                now_ms,
                Some(crate::theme::vars::ICON_IN),
            );
        }
    }

    /// How visible the icon of `package` is (1.0 once it has faded in).
    pub(super) fn icon_visibility(&self, package: &str, dock: bool) -> f32 {
        let slot = if dock {
            self.icons.dock_slot(package)
        } else {
            self.icons.slot(package)
        };
        slot.map_or(1.0, |slot| {
            self.anim.get(Anim::IconIn { dock, slot }, 1.0, self.now_ms)
        })
    }

    /// Fades or slides the content drawn so far in this frame (not the top bar, which is drawn
    /// after it) while a tab or `Slide` transition runs, and notes where the top bar starts.
    pub(super) fn fade_tab_content(&mut self, p: &mut painter::Painter) {
        let bar_start = p.list.len();
        self.bar_start = Some(bar_start);
        let visible = self.anim.get(Anim::TabFade, 1.0, self.now_ms);
        if visible < 1.0 {
            p.list.fade_from(self.content_start, visible);
        }
        let slide = self.anim.get(Anim::SlideTransition, 1.0, self.now_ms);
        if slide < 1.0 {
            let dy = self.slide_dir * SLIDE_OFFSET * (1.0 - slide);
            p.list.translate_range(self.content_start, bar_start, dy);
            p.list.fade_from(self.content_start, slide);
        }
    }

    /// Draws the current screen.
    pub fn draw(&mut self, p: &mut Painter, now_ms: u64) {
        self.now_ms = now_ms;
        if self.wake_pending {
            self.wake_pending = false;
            self.foreground_since = Some(now_ms);
        }
        if let Some((_, until)) = &self.toast {
            if now_ms >= *until {
                self.toast = None;
            }
        }
        self.splash_frame_start(now_ms);
        self.draw_splash_background(p);
        // A full-screen pattern of the Tools is drawn on the cleared frame, without wallpaper.
        if !self.tool_fullscreen() {
            // The Confirm screen shows the picture received, which is "Minha imagem" meanwhile, as
            // the wallpaper would be: without darkening.
            let (wallpaper, dim_amount) = match self.screen {
                Screen::ConfirmImage => (crate::wallpaper::WallpaperId::Custom, 0.0),
                Screen::Main(Tab::Home) if self.is_peek_mode() => (
                    self.config.wallpaper,
                    (self.current_scroll_y() / PEEK_DIM_DISTANCE).clamp(0.0, 1.0),
                ),
                Screen::Main(Tab::Home) => (self.config.wallpaper, 0.0),
                _ => (self.config.wallpaper, 1.0),
            };
            p.wallpaper_at(wallpaper, dim_amount, self.wallpaper_reveal());
        }
        self.content_start = p.list.len();
        self.bar_start = None;
        match self.screen {
            Screen::Main(Tab::Home) => self.draw_home(p),
            Screen::Main(Tab::Apps) => self.draw_apps(p),
            Screen::Settings => self.draw_settings(p),
            Screen::About => self.draw_about(p),
            Screen::TvInfo => self.draw_tv_info(p),
            Screen::Editor => self.draw_editor(p),
            Screen::KeyMap => self.draw_keymap(p),
            Screen::KeyMapPickApp => self.draw_pick_app(p),
            Screen::KeyMapPickSetting => self.draw_pick_setting(p),
            Screen::Tools => self.draw_tools(p),
            Screen::Tool(kind) => self.draw_tool(p, kind),
            Screen::ReceiveImage => self.draw_receive(p),
            Screen::ConfirmImage => self.draw_confirm(p),
        }
        self.draw_context_menu(p);
        self.draw_toast(p);
        let content_end = self.bar_start.unwrap_or(p.list.len());
        self.splash_frame_end(p, content_end);
        self.anim.prune(now_ms, |key| match key {
            Anim::TabFade
            | Anim::SlideTransition
            | Anim::MenuFade
            | Anim::Reveal
            | Anim::SplashLogo
            | Anim::IconIn { .. } => 1.0,
            _ => 0.0,
        });
    }

    fn draw_toast(&mut self, p: &mut Painter) {
        let Some((text, _)) = self.toast.clone() else {
            return;
        };
        let style = self.style(|t| &t.toast);
        let width = p.measure(&text, &style) + style.padding[1] + style.padding[3];
        let height = style.font_size * 1.6 + style.padding[0] + style.padding[2];
        let visible = p.display.visible_rect();
        let rect = Rect::new(
            visible.x + (visible.w - width) / 2.0,
            720.0 - 96.0,
            width,
            height,
        );
        p.draw_box(rect, &style);
        p.text(&text, rect, crate::text::Align::Center, &style);
    }

    /// How focused `id` is (0.0 to 1.0) at this moment.
    fn focus_progress(&self, id: FocusId) -> f32 {
        self.anim.get(Anim::Focus(id), 0.0, self.now_ms)
    }

    /// Starts the focus animations for a change of the focused element since the last frame.
    fn track_focus(&mut self) {
        let previous = self.tracked_focus;
        let current = self.focus.current_id();
        if previous == current {
            return;
        }
        self.tracked_focus = current;
        let transition = self.focus_transition();
        if let Some(old) = previous {
            self.anim
                .animate_to(Anim::Focus(old), 1.0, 0.0, self.now_ms, transition);
        }
        if let Some(new) = current {
            self.anim
                .animate_to(Anim::Focus(new), 0.0, 1.0, self.now_ms, transition);
        }
    }

    /// Rebuilds the focus rows from the screen's layout, keeping the focus where it was.
    fn update_focus(&mut self, rows: Vec<Row>) {
        self.focus.set_rows(rows);
        match self.pending_focus.take() {
            Some(PendingFocus::Id(id)) => {
                self.focus.focus(id);
            }
            Some(PendingFocus::FirstContent) => {
                // Only the main screens have a top bar to skip.
                let skip = usize::from(matches!(self.screen, Screen::Main(_)));
                let first = self
                    .focus
                    .rows()
                    .iter()
                    .skip(skip)
                    .find_map(|r| r.items.iter().find(|i| i.enabled).map(|i| i.id));
                if let Some(id) = first {
                    self.focus.focus(id);
                }
            }
            None => {}
        }
        self.sync_editor_preview_accent();
        self.track_focus();
    }

    /// Scrolling that keeps the focused rectangle visible. Returns the vertical offset.
    fn reveal_vertical(
        &mut self,
        rect: Option<Rect>,
        content_height: f32,
        view_top: f32,
        view_height: f32,
    ) {
        let focused = self.focus.current_id();
        if self.revealed == focused {
            return;
        }
        self.revealed = focused;
        let Some(rect) = rect else { return };
        let max = scroll::max_offset(content_height - view_top, view_height);
        let target = scroll::reveal(
            self.scroll_y,
            view_height,
            rect.y - view_top,
            rect.bottom() - view_top,
            28.0,
            max,
        );
        self.scroll_to(target);
    }

    fn scroll_to(&mut self, target: f32) {
        let transition = self.focus_transition();
        self.scroll_to_with(target, transition);
    }

    fn scroll_to_peek(&mut self, target: f32) {
        self.scroll_to_with(target, Some(PEEK_TRANSITION));
    }

    fn scroll_to_with(&mut self, target: f32, transition: Option<crate::theme::Transition>) {
        if (target - self.scroll_y).abs() < 0.01 {
            return;
        }
        self.anim.animate_to(
            Anim::ScrollY,
            self.scroll_y,
            target,
            self.now_ms,
            transition,
        );
        self.scroll_y = target;
    }

    /// Vertical reveal for `Peek` mode: when focus is in the grid, ensures the scroll offset is at
    /// least `min_scroll` (hiding the dock behind the top bar), using the 250 ms slide curve when
    /// crossing from the dock into the grid.
    fn reveal_vertical_peek(
        &mut self,
        rect: Option<Rect>,
        content_height: f32,
        view_top: f32,
        view_height: f32,
        min_scroll: f32,
    ) {
        let focused = self.focus.current_id();
        if self.revealed == focused {
            return;
        }
        self.revealed = focused;
        let Some(rect) = rect else { return };
        let entering_grid = self.scroll_y < min_scroll;
        let max = scroll::max_offset(content_height - view_top, view_height).max(min_scroll);
        let current = self.scroll_y.max(min_scroll);
        let target = scroll::reveal(
            current,
            view_height,
            rect.y - view_top,
            rect.bottom() - view_top,
            28.0,
            max,
        )
        .max(min_scroll);
        let transition = if entering_grid {
            Some(PEEK_TRANSITION)
        } else {
            self.focus_transition()
        };
        self.scroll_to_with(target, transition);
    }

    /// The vertical scroll offset to draw with now.
    fn current_scroll_y(&self) -> f32 {
        self.anim.get(Anim::ScrollY, self.scroll_y, self.now_ms)
    }
}

impl LumeApp {
    /// The image of an app tile: its TV banner, which covers the tile, or its icon centered on the
    /// tile color. With `name_on_tile`, an icon moves up to leave room for the name. `rect` is the
    /// tile before `style.scale`.
    ///
    /// Tiles are drawn in passes (boxes, then images, then text) so each pass is one draw call:
    /// alternating the three programs tile by tile took ~3 calls per tile on the TV's GPU.
    ///
    /// A banner, or an icon in the middle of the tile, is drawn in one piece with the tile color
    /// (`style.background`) under it, clipped once by the tile's corners at its final size: the box
    /// pass then leaves the color out (see [`LumeApp::tile_has_image`]), and the tile has a single
    /// antialiased edge.
    pub(super) fn draw_tile_image(
        &self,
        p: &mut painter::Painter,
        package: Option<&str>,
        rect: Rect,
        style: &ComputedStyle,
        name_on_tile: bool,
    ) {
        let tile = rect.scaled_about_center(style.scale);
        let Some(package) = package else { return };
        let (Some(uv), Some(art)) = (self.icons.uv(package), self.icons.art(package)) else {
            return;
        };
        let tint = painter::fade(
            crate::theme::Color::WHITE,
            style.opacity * self.icon_visibility(package, false),
        );
        if art == Art::Icon && name_on_tile {
            // The lifted icon leaves room for the name: the box pass draws the tile color.
            p.list.image(tile.translate(0.0, -tile.h * 0.17), uv, tint);
            return;
        }
        let color = match style.background {
            Some(crate::theme::Paint::Solid(color)) => painter::fade(color, style.opacity),
            _ => crate::theme::Color::TRANSPARENT,
        };
        let radius = style.border_radius * style.scale;
        p.list.image_over(tile, uv, tint, radius, color);
    }

    /// Whether [`draw_tile_image`](Self::draw_tile_image) draws the tile color with the image, so
    /// the box pass must leave it out.
    pub(super) fn tile_has_image(&self, package: Option<&str>, name_on_tile: bool) -> bool {
        package.is_some_and(|package| {
            self.icons.uv(package).is_some()
                && match self.icons.art(package) {
                    Some(Art::Banner) => true,
                    Some(Art::Icon) => !name_on_tile,
                    None => false,
                }
        })
    }

    /// The name inside an app tile: the whole name while there is no image, under a lifted icon
    /// with `name_on_tile`, and nothing on a banner (which has the name in it).
    pub(super) fn draw_tile_name(
        &self,
        p: &mut painter::Painter,
        package: Option<&str>,
        label: &str,
        rect: Rect,
        style: &ComputedStyle,
        name_on_tile: bool,
    ) {
        let tile = rect.scaled_about_center(style.scale);
        let art = package.and_then(|package| self.icons.art(package));
        // While an image fades in, the name it replaces fades out under it.
        let fading = package
            .filter(|_| art.is_some() && !name_on_tile)
            .map_or(1.0, |package| self.icon_visibility(package, false));
        if fading < 1.0 {
            let mut faded = *style;
            faded.color = painter::fade(style.color, 1.0 - fading);
            p.text(label, tile.grow(-12.0), crate::text::Align::Center, &faded);
            return;
        }
        match art {
            None => p.text(label, tile.grow(-12.0), crate::text::Align::Center, style),
            Some(Art::Icon) if name_on_tile => {
                let name = Rect::new(
                    tile.x + 12.0,
                    tile.y + tile.h * 0.60,
                    tile.w - 24.0,
                    tile.h * 0.28,
                );
                p.text(label, name, crate::text::Align::Center, style);
            }
            Some(_) => {}
        }
    }
}
