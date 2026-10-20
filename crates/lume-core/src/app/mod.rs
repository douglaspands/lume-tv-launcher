//! The launcher itself: screens, navigation and drawing.
//!
//! [`LumeApp`] holds the state and is driven by three calls from the platform shell:
//! [`LumeApp::handle_key`] for remote control keys, [`LumeApp::draw`] to fill a display list,
//! and [`LumeApp::next_deadline`] to know when something will change without any input
//! (an animation, the carousel, the clock), so the shell can sleep otherwise.

mod apps;
mod editor;
mod home;
mod page;
mod painter;
mod settings;
#[cfg(test)]
mod tests;
mod topbar;

pub use painter::Painter;

use crate::app_icons::{AppIcons, Art};
use crate::catalog::Catalog;
use crate::clock::WallClock;
use crate::home::config::HomeConfig;
use crate::home::slides::{Slide, SlideAction};
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
    pub const HERO_BUTTON: FocusId = 10;
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
    FavoritesScrollX,
    /// Cross-fade between carousel slides, 0.0 to 1.0.
    SlideFade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingFocus {
    /// The first element below the top bar.
    FirstContent,
    Id(FocusId),
}

/// The carousel's runtime state.
#[derive(Clone, Debug, Default)]
struct Carousel {
    index: usize,
    previous: Option<usize>,
    /// When the current slide appeared (milliseconds on the app clock).
    since_ms: u64,
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
    /// The tab to return to when leaving the settings.
    settings_return: Tab,
    editor: editor::EditorState,
    about_scroll: f32,
    focus: Focus,
    anim: Animations<Anim>,
    scroll_y: f32,
    favorites_scroll_x: f32,
    /// The focused element whose position the scrolling was last adjusted for.
    revealed: Option<FocusId>,
    /// The element whose focus animation was last started.
    tracked_focus: Option<FocusId>,
    /// Where the focus goes at the next layout (after a screen change).
    pending_focus: Option<PendingFocus>,
    carousel: Carousel,
    slides: Vec<Slide>,
    toast: Option<(String, u64)>,
    now_ms: u64,
}

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
            settings_return: Tab::Home,
            editor: editor::EditorState::default(),
            about_scroll: 0.0,
            focus: Focus::new(),
            anim: Animations::new(),
            scroll_y: 0.0,
            favorites_scroll_x: 0.0,
            revealed: None,
            tracked_focus: None,
            // The app starts with the focus on the first element of the content.
            pending_focus: Some(PendingFocus::FirstContent),
            carousel: Carousel::default(),
            slides: Vec::new(),
            toast: None,
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
            _ => self.settings_return,
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
                self.go_home();
                vec![]
            }
        }
    }

    fn horizontal(&mut self, direction: Direction) -> Vec<Effect> {
        // With the focus on the carousel's button, left and right change the slide.
        if self.screen == Screen::Main(Tab::Home)
            && self.focus.current_id() == Some(ids::HERO_BUTTON)
        {
            self.step_slide(if direction == Direction::Left { -1 } else { 1 });
            return vec![];
        }
        self.move_focus(direction);
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
            ids::GEAR => {
                self.open_settings();
                vec![]
            }
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

    fn select_tab(&mut self, tab: Tab) {
        if self.screen != Screen::Main(tab) {
            self.screen = Screen::Main(tab);
            self.scroll_to_top();
            // The focus stays on the tab that was selected.
        }
    }

    fn scroll_to_top(&mut self) {
        self.scroll_y = 0.0;
        self.favorites_scroll_x = 0.0;
        self.anim.set(Anim::ScrollY, 0.0);
        self.anim.set(Anim::FavoritesScrollX, 0.0);
        self.revealed = None;
    }

    /// Shows the next (`1`) or previous (`-1`) slide.
    fn step_slide(&mut self, delta: i32) {
        let count = self.slides.len();
        if count < 2 {
            return;
        }
        let next = (self.carousel.index as i32 + delta).rem_euclid(count as i32) as usize;
        self.set_slide(next);
    }

    fn set_slide(&mut self, index: usize) {
        if index == self.carousel.index {
            return;
        }
        self.carousel.previous = Some(self.carousel.index);
        self.carousel.index = index;
        self.carousel.since_ms = self.now_ms;
        let transition = Some(crate::theme::Transition {
            duration_ms: 300.0,
            curve: crate::theme::Curve::EaseInOut,
        });
        self.anim.set(Anim::SlideFade, 0.0);
        self.anim
            .animate_to(Anim::SlideFade, 0.0, 1.0, self.now_ms, transition);
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

    /// When something will change without any key press: the next animation frame, the carousel
    /// advancing, the clock's minute turning or a message expiring. `None` lets the shell sleep.
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
        if let Some(at) = self.carousel_deadline() {
            consider(at);
        }
        deadline
    }

    /// Draws the current screen.
    pub fn draw(&mut self, p: &mut Painter, now_ms: u64) {
        self.now_ms = now_ms;
        if let Some((_, until)) = &self.toast {
            if now_ms >= *until {
                self.toast = None;
            }
        }
        match self.screen {
            Screen::Main(Tab::Home) => self.draw_home(p),
            Screen::Main(Tab::Apps) => self.draw_apps(p),
            Screen::Settings => self.draw_settings(p),
            Screen::About => self.draw_about(p),
            Screen::Editor => self.draw_editor(p),
        }
        self.draw_toast(p);
        self.anim.prune(now_ms, |key| match key {
            Anim::SlideFade => 1.0,
            _ => 0.0,
        });
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

/// What a tap on a slide's button does, for the shell and the tests.
pub(crate) fn slide_effect(app: &mut LumeApp, slide: &Slide) -> Vec<Effect> {
    match &slide.action {
        SlideAction::OpenApp(package) => app.open_app(package),
        SlideAction::ShowApps => {
            app.select_tab(Tab::Apps);
            vec![]
        }
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
