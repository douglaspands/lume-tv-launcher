//! The launcher itself: screens, navigation and drawing.
//!
//! [`LumeApp`] holds the state and is driven by three calls from the platform shell:
//! [`LumeApp::handle_key`] for remote control keys, [`LumeApp::draw`] to fill a display list,
//! and [`LumeApp::next_deadline`] to know when something will change without any input
//! (an animation, the clock), so the shell can sleep otherwise.

mod apps;
mod dock;
mod editor;
mod home;
mod page;
mod painter;
mod settings;
#[cfg(test)]
mod tests;
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

pub use painter::Painter;

use crate::app_icons::{AppIcons, Art};
use crate::catalog::Catalog;
use crate::clock::WallClock;
use crate::home::config::HomeConfig;
use crate::i18n::Translator;
use crate::render::Rect;
use crate::theme::{ComputedStyle, THEME};
use crate::ui::animation::Animations;
use crate::ui::focus::{Direction, Focus, FocusId, Row};
use crate::ui::scroll;
use crate::usage::UsageHistory;

/// Numbers of the focusable elements. Each screen has its own range.
pub mod ids {
    use crate::ui::focus::FocusId;

    pub const TAB_HOME: FocusId = 1;
    pub const TAB_APPS: FocusId = 2;
    pub const GEAR: FocusId = 3;
    pub const INPUTS: FocusId = 4;
    pub const FAVORITES_ADD: FocusId = 999;
    pub const FAVORITE_BASE: FocusId = 1000;
    pub const GRID_BASE: FocusId = 2000;
    pub const GRID_SETTINGS: FocusId = 2999;
    pub const SETTINGS_BASE: FocusId = 3000;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home = 0,
    Apps = 1,
}

/// Remote control keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    /// OK, Enter or the center of the D-pad.
    Ok,
    Back,
    Home,
}

/// What the platform shell has to do for the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    OpenApp {
        package: String,
        activity: String,
    },
    OpenAndroidSettings,
    OpenInputs,
    /// The home configuration changed and must be written to disk.
    SaveConfig,
    /// The usage history changed and must be written to disk.
    SaveUsage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main(Tab),
    /// The lume settings (opened with the gear).
    Settings,
    About,
    /// Editing the home screen.
    Editor,
}

/// What is animated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Anim {
    /// How focused an element is, 0.0 to 1.0.
    Focus(FocusId),
    ScrollY,
    /// The place (fractional while it moves) of the dock icon that is magnified.
    DockPosition,
    /// How much the dock magnifies, 0.0 (the focus is elsewhere) to 1.0.
    DockAmplitude,
    /// How visible the content of the tab is while it fades in after a tab change, 0.0 to 1.0.
    TabFade,
}

/// The fade of the content when the tab changes.
const TAB_FADE: crate::theme::Transition = crate::theme::Transition {
    duration_ms: 180.0,
    curve: crate::theme::Curve::EaseOut,
};

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
    pub translator: Translator,
    pub clock: WallClock,
    screen: Screen,
    /// Whether the system has a TV inputs panel: the top bar shows its button only then.
    pub inputs_available: bool,
    editor: editor::EditorState,
    about_scroll: f32,
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
        LumeApp {
            catalog,
            icons: AppIcons::default(),
            usage,
            config,
            translator,
            clock,
            screen: Screen::Main(Tab::Home),
            inputs_available: false,
            editor: editor::EditorState::default(),
            about_scroll: 0.0,
            stats: None,
            cpu_use: None,
            stats_at: None,
            focus: Focus::new(),
            anim: Animations::new(),
            scroll_y: 0.0,
            content_start: 0,
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

    /// The transition the theme gives to focus changes.
    fn focus_transition(&self) -> Option<crate::theme::Transition> {
        THEME.tile.base.transition
    }

    fn style(&self, component: &crate::theme::ComponentTheme) -> ComputedStyle {
        THEME.resolve(component, crate::theme::StateFlags::NONE)
    }

    /// Moves the focus in `direction` and returns whether it moved.
    fn move_focus(&mut self, direction: Direction) -> bool {
        self.focus.move_focus(direction)
    }

    /// Handles a key press.
    pub fn handle_key(&mut self, key: Key, now_ms: u64) -> Vec<Effect> {
        self.now_ms = now_ms;
        // Only the Home press that brings lume back can use the place where an app was opened.
        if key != Key::Home {
            self.return_point = None;
        }
        let effects = self.dispatch_key(key);
        if effects.iter().any(|e| {
            matches!(
                e,
                Effect::OpenApp { .. } | Effect::OpenAndroidSettings | Effect::OpenInputs
            )
        }) {
            self.return_point = Some(ReturnPoint {
                screen: self.screen,
                focus: self.focus.current_id(),
                scroll_y: self.scroll_y,
            });
        }
        effects
    }

    fn dispatch_key(&mut self, key: Key) -> Vec<Effect> {
        match key {
            Key::Up | Key::Down if self.screen == Screen::About => {
                self.scroll_about(if key == Key::Up { -1.0 } else { 1.0 });
                vec![]
            }
            Key::Up => {
                self.move_focus(Direction::Up);
                vec![]
            }
            Key::Down => {
                self.move_focus(Direction::Down);
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
        }
    }

    fn horizontal(&mut self, direction: Direction) -> Vec<Effect> {
        self.move_focus(direction);
        // Moving the focus onto a tab shows that tab at once, without OK. The focus stays on it.
        if matches!(self.screen, Screen::Main(_)) {
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
            Screen::About => return vec![],
            Screen::Editor => return self.activate_editor(id),
        }
        match id {
            ids::GEAR => vec![Effect::OpenAndroidSettings],
            ids::INPUTS => vec![Effect::OpenInputs],
            ids::TAB_HOME => {
                self.select_tab(Tab::Home);
                vec![]
            }
            ids::TAB_APPS => {
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
                // Back on the home root scrolls to the top and never closes lume.
                self.scroll_to_top();
                self.pending_focus = Some(PendingFocus::FirstContent);
                vec![]
            }
            Screen::Main(Tab::Apps) => {
                self.select_tab(Tab::Home);
                self.pending_focus = Some(PendingFocus::FirstContent);
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
            Screen::Editor => self.back_from_editor(),
        }
    }

    /// The Home button: back to the Home tab, scrolled to the top, focus on its first element.
    pub fn go_home(&mut self) {
        self.editor = editor::EditorState::default();
        self.screen = Screen::Main(Tab::Home);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
    }

    /// lume left the screen (another app is in front).
    pub fn went_to_background(&mut self) {
        self.foreground = false;
    }

    /// lume is on the screen again.
    pub fn came_to_foreground(&mut self) {
        self.foreground = true;
        self.wake_pending = true;
        self.foreground_since = None;
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
        deadline
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

    /// Whether an animation is running at `now_ms` (a frame is due every few milliseconds).
    pub fn is_animating(&self, now_ms: u64) -> bool {
        self.anim.is_animating(self.now_ms.max(now_ms))
    }

    /// Fades the content drawn so far in this frame (not the top bar, which is drawn after it)
    /// while the tab that was just selected appears.
    pub(super) fn fade_tab_content(&self, p: &mut painter::Painter) {
        let visible = self.anim.get(Anim::TabFade, 1.0, self.now_ms);
        if visible < 1.0 {
            p.list.fade_from(self.content_start, visible);
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
        p.wallpaper(self.screen != Screen::Main(Tab::Home));
        self.content_start = p.list.len();
        match self.screen {
            Screen::Main(Tab::Home) => self.draw_home(p),
            Screen::Main(Tab::Apps) => self.draw_apps(p),
            Screen::Settings => self.draw_settings(p),
            Screen::About => self.draw_about(p),
            Screen::Editor => self.draw_editor(p),
        }
        self.draw_toast(p);
        self.anim
            .prune(now_ms, |key| if *key == Anim::TabFade { 1.0 } else { 0.0 });
    }

    fn draw_toast(&mut self, p: &mut Painter) {
        let Some((text, _)) = self.toast.clone() else {
            return;
        };
        let style = self.style(&THEME.toast);
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
        if (target - self.scroll_y).abs() < 0.01 {
            return;
        }
        let transition = self.focus_transition();
        self.anim.animate_to(
            Anim::ScrollY,
            self.scroll_y,
            target,
            self.now_ms,
            transition,
        );
        self.scroll_y = target;
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
        let tint = painter::fade(crate::theme::Color::WHITE, style.opacity);
        match art {
            Art::Banner => p.list.image(tile, uv, tint),
            Art::Icon if name_on_tile => {
                p.list.image(tile.translate(0.0, -tile.h * 0.17), uv, tint);
            }
            Art::Icon => p.list.image(tile, uv, tint),
        }
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
        match package.and_then(|package| self.icons.art(package)) {
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
