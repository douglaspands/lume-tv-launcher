//! Behavior of the launcher as a whole, driven by keys and frames like the platform shell does.

use super::*;
use crate::catalog::Discovered;
use crate::display::Display;
use crate::render::DisplayList;
use crate::text::TextEngine;

const NETFLIX: &str = "com.netflix.ninja";
const YOUTUBE: &str = "com.google.android.youtube.tv";
const PRIME: &str = "com.amazon.amazonvideo.livingroom";
const SPOTIFY: &str = "com.spotify.tv.android";
const DISNEY: &str = "com.disney.disneyplus";
const NOW: u64 = 1_792_446_300;

pub(super) fn discovered(package: &str, label: &str) -> Discovered {
    Discovered {
        package: package.to_string(),
        label: label.to_string(),
        activity: format!("{package}.Main"),
        leanback: true,
        system: false,
        icon: None,
    }
}

fn catalog(apps: &[(&str, &str)]) -> Catalog {
    let entries: Vec<Discovered> = apps.iter().map(|(p, l)| discovered(p, l)).collect();
    Catalog::from_discovered(&entries, "app.lume.tv")
}

fn standard_catalog() -> Catalog {
    catalog(&[
        (NETFLIX, "Netflix"),
        (YOUTUBE, "YouTube"),
        (PRIME, "Prime Video"),
        (SPOTIFY, "Spotify"),
        (DISNEY, "Disney+"),
        ("com.demo.chrome", "Chrome"),
        ("com.demo.games", "Jogos"),
    ])
}

pub(super) struct Rig {
    pub app: LumeApp,
    text: TextEngine,
    display: Display,
    pub now: u64,
}

impl Rig {
    pub fn new() -> Rig {
        Rig::with_catalog(standard_catalog())
    }

    pub fn with_catalog(catalog: Catalog) -> Rig {
        let mut usage = UsageHistory::new();
        usage.mark_baseline(catalog.apps().iter().map(|a| a.package.as_str()));
        let config = HomeConfig::first_use(&catalog);
        let clock = WallClock {
            unix_secs: NOW,
            utc_offset_secs: 0,
            is_24h: true,
        };
        let display = Display::new(1280, 720);
        let mut rig = Rig {
            app: LumeApp::new(catalog, usage, config, Translator::new(&["pt-BR"]), clock),
            text: TextEngine::new(display.scale),
            display,
            now: 0,
        };
        rig.frame();
        rig
    }

    /// Draws one frame and returns the display list.
    pub fn frame(&mut self) -> DisplayList {
        let mut list = DisplayList::new();
        for _ in 0..4 {
            list.clear();
            self.text.begin_frame();
            let mut painter = Painter {
                list: &mut list,
                text: &mut self.text,
                display: self.display,
            };
            self.app.draw(&mut painter, self.now);
            if !self.text.end_frame() {
                break;
            }
        }
        list
    }

    /// Presses a key, advances 100 ms and draws.
    pub fn press(&mut self, key: Key) -> Vec<Effect> {
        let effects = self.app.handle_key(key, self.now);
        self.now += 100;
        self.frame();
        effects
    }

    pub fn press_all(&mut self, keys: &[Key]) {
        for key in keys {
            self.press(*key);
        }
    }

    /// Moves the clock forward by `ms`, drawing a frame at the end.
    pub fn advance(&mut self, ms: u64) {
        self.now += ms;
        self.frame();
    }

    pub fn focused(&self) -> Option<FocusId> {
        self.app.focused()
    }
}

#[test]
fn the_app_starts_on_the_first_favorite() {
    let rig = Rig::new();
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
}

#[test]
fn right_moves_along_the_favorites_and_down_does_nothing() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 2));
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 2));
    rig.press(Key::Up);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME));
}

#[test]
fn moving_the_focus_animates_the_old_and_new_elements() {
    let mut rig = Rig::new();
    rig.advance(1000);
    assert_eq!(
        rig.app.focus_progress(ids::FAVORITE_BASE),
        1.0,
        "the first element is fully focused"
    );
    rig.app.handle_key(Key::Right, rig.now);
    rig.advance(10);
    rig.advance(60);
    let (old, new) = (
        rig.app.focus_progress(ids::FAVORITE_BASE),
        rig.app.focus_progress(ids::FAVORITE_BASE + 1),
    );
    assert!(
        old < 1.0 && old > 0.0 && new > 0.0 && new < 1.0,
        "mid-animation: {old} {new}"
    );
    rig.advance(1000);
    assert_eq!(rig.app.focus_progress(ids::FAVORITE_BASE), 0.0);
    assert_eq!(rig.app.focus_progress(ids::FAVORITE_BASE + 1), 1.0);
    assert!(
        rig.app.next_deadline(rig.now).is_some(),
        "the clock still ticks"
    );
}

fn open_apps(rig: &mut Rig) {
    // From the favorites: up to the top bar, right to "Apps", OK.
    rig.press_all(&[Key::Up]);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME));
    rig.press_all(&[Key::Right, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
}

#[test]
fn up_from_the_first_row_goes_to_the_active_tab_and_right_then_ok_opens_apps() {
    let mut rig = Rig::new();
    rig.press(Key::Up);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME), "the active tab");
    rig.press(Key::Right);
    assert_eq!(rig.focused(), Some(ids::TAB_APPS));
    rig.press(Key::Ok);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(
        rig.focused(),
        Some(ids::TAB_APPS),
        "the focus stays on the tab"
    );
    // Down enters the grid.
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::GRID_BASE));
}

#[test]
fn back_on_the_home_root_scrolls_to_the_top_without_closing() {
    let mut rig = Rig::new();
    rig.press(Key::Right);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 1));
    let effects = rig.press(Key::Back);
    assert!(effects.is_empty(), "lume stays open");
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE),
        "the focus returns to the first favorite"
    );
    assert_eq!(rig.app.scroll_y, 0.0);
    // A second Back changes nothing.
    assert!(rig.press(Key::Back).is_empty());
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn back_in_the_apps_tab_returns_to_home() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn the_home_button_returns_to_the_start_of_the_home_tab() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press_all(&[Key::Down, Key::Right, Key::Right]);
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE),
        "the first element of the content"
    );
    // Pressing Home again on the home tab keeps everything where it is.
    rig.press_all(&[Key::Down, Key::Right]);
    rig.press(Key::Home);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn the_apps_grid_is_alphabetical_with_lume_among_the_apps() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    let labels: Vec<&str> = rig
        .app
        .catalog
        .apps()
        .iter()
        .map(|a| a.label.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            "Chrome",
            "Disney+",
            "Jogos",
            "Netflix",
            "Prime Video",
            "Spotify",
            "YouTube"
        ]
    );
    let rows = rig.app.focus.rows();
    let ids_in_order: Vec<u32> = rows
        .iter()
        .skip(1)
        .flat_map(|r| r.items.iter().map(|i| i.id))
        .collect();
    assert_eq!(ids_in_order.len(), 8, "the seven apps and lume");
    assert_eq!(
        ids_in_order[3],
        ids::GRID_SETTINGS,
        "lume comes between Jogos and Netflix"
    );
    let cells = rows
        .iter()
        .skip(1)
        .map(|r| r.items.len())
        .collect::<Vec<_>>();
    assert_eq!(cells, [6, 2], "6 columns");
}

#[test]
fn the_grid_does_not_wrap_at_the_end_of_a_row() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press_all(&[Key::Right; 5]);
    // Chrome, Disney+, Jogos, Lume, Netflix, Prime Video: the 6th is the 5th app (index 4).
    assert_eq!(rig.focused(), Some(ids::GRID_BASE + 4));
    rig.press(Key::Right);
    assert_eq!(
        rig.focused(),
        Some(ids::GRID_BASE + 4),
        "stays on the 6th column"
    );
}

#[test]
fn down_into_a_short_last_row_lands_on_its_last_item() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press_all(&[Key::Right; 4]);
    // Column 5 of the first row is Netflix (index 3, as lume sits before it).
    assert_eq!(rig.focused(), Some(ids::GRID_BASE + 3));
    rig.press(Key::Down);
    // The last row has Spotify and YouTube: the nearest to column 5 is the last one.
    assert_eq!(rig.focused(), Some(ids::GRID_BASE + 6));
}

#[test]
fn up_from_the_first_grid_row_goes_to_the_top_bar() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press_all(&[Key::Down, Key::Right, Key::Right, Key::Up]);
    assert_eq!(rig.focused(), Some(ids::TAB_APPS), "the active tab");
}

#[test]
fn ok_on_a_tile_opens_the_app_and_records_it() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Down, Key::Right, Key::Right]);
    let effects = rig.press(Key::Ok);
    assert_eq!(
        effects,
        [
            Effect::OpenApp {
                package: PRIME.into(),
                activity: format!("{PRIME}.Main")
            },
            Effect::SaveUsage
        ]
    );
    assert_eq!(rig.app.usage.opens_in_last_7_days(PRIME, NOW), 1);
    assert_eq!(rig.app.usage.last_opened(PRIME), Some(NOW));
}

#[test]
fn ok_on_the_lume_tile_opens_the_lume_settings() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press_all(&[Key::Right, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::GRID_SETTINGS));
    assert_eq!(rig.press(Key::Ok), []);
    assert_eq!(rig.app.screen(), &Screen::Settings);
}

#[test]
fn opening_an_app_that_is_gone_shows_a_message_and_does_not_crash() {
    let mut rig = Rig::new();
    rig.press(Key::Down);
    // The app disappears from the catalog between drawing and the key press.
    rig.app.catalog.apply(&[], &[NETFLIX.to_string()]);
    let effects = rig.app.handle_key(Key::Ok, rig.now);
    assert!(effects.is_empty());
    assert!(rig
        .app
        .toast
        .as_ref()
        .is_some_and(|(text, _)| text == "App não encontrado"));
    rig.advance(3000);
    assert!(rig.app.toast.is_none(), "the message goes away by itself");
}

#[test]
fn the_clock_asks_for_a_frame_when_the_minute_turns() {
    let mut rig = Rig::new();
    rig.advance(1000);
    let deadline = rig.app.next_deadline(rig.now).expect("a deadline");
    assert!(deadline > rig.now && deadline <= rig.now + 60_000);
    // 21:45:00 -> the minute turns after exactly 60 seconds from the clock's own start.
    assert_eq!(rig.app.clock.millis_to_next_minute(), 60_000);
}

#[test]
fn an_idle_app_without_a_visible_clock_change_only_wakes_for_the_clock() {
    let mut rig = Rig::new();
    rig.press(Key::Down);
    rig.advance(5000);
    let deadline = rig.app.next_deadline(rig.now).unwrap();
    assert!(deadline <= rig.now + 60_000);
}

#[test]
fn an_empty_favorites_row_shows_a_single_add_tile() {
    let mut rig = Rig::new();
    rig.app.config.favorites.clear();
    rig.advance(0);
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::FAVORITES_ADD));
    rig.press(Key::Right);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITES_ADD),
        "it is the only tile"
    );
}

#[test]
fn removing_an_installed_app_updates_the_screens() {
    let mut rig = Rig::new();
    rig.app.catalog.apply(&[], &[YOUTUBE.to_string()]);
    rig.app.config.remove_uninstalled(&rig.app.catalog);
    rig.advance(100);
    assert_eq!(rig.app.config.favorites, [NETFLIX, PRIME, SPOTIFY, DISNEY]);
}

// ---- settings and the home screen editor ---------------------------------------------------

use super::editor::{
    ADD_APP_BASE, CONFIRM_CANCEL, CONFIRM_RESTORE, DONE, FAVORITE_BUTTON_BASE, RESTORE,
    ROUND_FAVORITES,
};

fn open_settings(rig: &mut Rig) {
    // The Apps tab, then the "Lume" tile.
    open_apps(rig);
    rig.press(Key::Down);
    press_on(rig, ids::GRID_SETTINGS);
    assert_eq!(rig.app.screen(), &Screen::Settings);
}

fn open_editor(rig: &mut Rig) {
    open_settings(rig);
    assert_eq!(
        rig.focused(),
        Some(ids::SETTINGS_BASE),
        "the first item has the focus"
    );
    rig.press(Key::Ok);
    assert_eq!(rig.app.screen(), &Screen::Editor);
}

/// Puts the focus on `id` (navigation itself is tested separately) and presses OK.
fn press_on(rig: &mut Rig, id: FocusId) -> Vec<Effect> {
    focus_on(rig, id);
    rig.press(Key::Ok)
}

fn focus_on(rig: &mut Rig, id: FocusId) {
    assert!(rig.app.focus.focus(id), "{id} cannot take the focus");
    rig.advance(0);
}

#[test]
fn the_lume_settings_start_on_the_first_item_and_list_three() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE));
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE + 1));
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE + 2));
    assert!(
        !rig.app.focus.move_focus(Direction::Down),
        "three items only"
    );
}

fn open_tv_info(rig: &mut Rig) {
    open_settings(rig);
    rig.press_all(&[Key::Down, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::TvInfo);
}

#[test]
fn the_tv_information_waits_for_the_platform_and_is_read_each_time_it_opens() {
    let mut rig = Rig::new();
    open_tv_info(&mut rig);
    assert!(rig.app.wants_device_info(), "asked on opening");
    rig.app
        .apply_device_info(crate::device_info::DeviceInfo::default());
    assert!(!rig.app.wants_device_info(), "read once while it stays");
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Settings);
    assert!(!rig.app.wants_device_info());
    rig.press(Key::Ok);
    assert!(rig.app.wants_device_info(), "read again on the next visit");
    rig.app.went_to_background();
    assert!(!rig.app.wants_device_info(), "not in the background");
}

#[test]
fn the_tv_information_scrolls_within_the_sheet_and_back_returns_to_its_item() {
    let mut rig = Rig::new();
    open_tv_info(&mut rig);
    rig.app
        .apply_device_info(crate::device_info::DeviceInfo::default());
    rig.advance(0);
    assert!(
        rig.app.tv_max_scroll > 0.0,
        "the sheet is taller than the screen"
    );
    rig.press(Key::Up);
    assert_eq!(rig.app.scroll_y, 0.0, "no scrolling above the top");
    rig.press(Key::Down);
    assert!(rig.app.scroll_y > 0.0);
    for _ in 0..50 {
        rig.press(Key::Down);
    }
    assert_eq!(
        rig.app.scroll_y, rig.app.tv_max_scroll,
        "no scrolling past the end"
    );
    rig.advance(1000);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Settings);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE + 1));
    assert_eq!(rig.app.scroll_y, 0.0);
}

#[test]
fn back_from_the_settings_returns_to_the_lume_tile() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), Some(ids::GRID_SETTINGS));
}

#[test]
fn the_gear_opens_the_android_settings() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    assert_eq!(rig.press(Key::Ok), [Effect::OpenAndroidSettings]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
}

#[test]
fn the_inputs_button_sits_left_of_the_gear_and_opens_the_inputs() {
    let mut rig = Rig::new();
    rig.app.inputs_available = true;
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::INPUTS));
    assert_eq!(rig.press(Key::Ok), [Effect::OpenInputs]);
    rig.press(Key::Right);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    rig.press(Key::Left);
    assert_eq!(rig.focused(), Some(ids::INPUTS));
}

#[test]
fn without_an_inputs_panel_the_button_is_not_there() {
    let mut rig = Rig::new();
    rig.app.inputs_available = false;
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    rig.advance(1000);
    assert!(!rig.app.focus.focus(ids::INPUTS));
}

#[test]
fn the_about_screen_scrolls_and_goes_back_to_the_settings() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    rig.press_all(&[Key::Down, Key::Down, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::About);
    assert_eq!(rig.app.about_scroll, 0.0);
    rig.press(Key::Down);
    assert!(rig.app.about_scroll > 0.0, "the license list scrolls");
    rig.press_all(&[Key::Up, Key::Up]);
    assert_eq!(rig.app.about_scroll, 0.0);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Settings);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE + 2));
}

#[test]
fn the_editor_opens_from_the_settings_and_done_returns_to_the_home_screen() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    assert_eq!(
        rig.focused(),
        Some(ROUND_FAVORITES),
        "the round favorites switch has the focus"
    );
    focus_on(&mut rig, DONE);
    let effects = rig.press(Key::Ok);
    assert_eq!(effects, [Effect::SaveConfig]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn back_in_the_editor_returns_to_the_settings_and_then_to_the_lume_tile() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let effects = rig.press(Key::Back);
    assert_eq!(effects, [Effect::SaveConfig], "the changes are kept");
    assert_eq!(rig.app.screen(), &Screen::Settings);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE));
    // The way out retraces the way in: the "Lume" tile of the Apps tab.
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), Some(ids::GRID_SETTINGS));
}

#[test]
fn back_in_the_editor_opened_from_the_home_screen_goes_home() {
    let mut rig = Rig::new();
    rig.app.config.favorites.clear();
    rig.press(Key::Home);
    rig.press(Key::Ok);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert_eq!(rig.press(Key::Back), [Effect::SaveConfig]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
}

#[test]
fn the_add_favorites_tile_opens_the_editor() {
    let mut rig = Rig::new();
    rig.app.config.favorites.clear();
    rig.advance(0);
    rig.press_all(&[Key::Down, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::Editor);
}

#[test]
fn a_favorite_moves_left_and_the_focus_follows_it() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    // Spotify is in position 4 (index 3): its "move left" button.
    let left = FAVORITE_BUTTON_BASE + 3 * 3;
    assert_eq!(press_on(&mut rig, left), [Effect::SaveConfig]);
    assert_eq!(
        rig.app.config.favorites,
        [NETFLIX, YOUTUBE, SPOTIFY, PRIME, DISNEY]
    );
    assert_eq!(
        rig.focused(),
        Some(FAVORITE_BUTTON_BASE + 2 * 3),
        "on the same button of the moved card"
    );
}

#[test]
fn a_favorite_moves_right_and_the_edges_are_disabled() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let enabled = |rig: &Rig, id: FocusId| {
        rig.app
            .focus
            .rows()
            .iter()
            .any(|r| r.items.iter().any(|i| i.id == id && i.enabled))
    };
    assert!(
        !enabled(&rig, FAVORITE_BUTTON_BASE),
        "the first cannot move left"
    );
    assert!(
        !enabled(&rig, FAVORITE_BUTTON_BASE + 4 * 3 + 1),
        "the last cannot move right"
    );
    let right = FAVORITE_BUTTON_BASE + 1;
    assert_eq!(press_on(&mut rig, right), [Effect::SaveConfig]);
    assert_eq!(rig.app.config.favorites[..2], [YOUTUBE, NETFLIX]);
}

#[test]
fn a_favorite_can_be_removed() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let remove = FAVORITE_BUTTON_BASE + 2;
    assert_eq!(press_on(&mut rig, remove), [Effect::SaveConfig]);
    assert_eq!(rig.app.config.favorites, [YOUTUBE, PRIME, SPOTIFY, DISNEY]);
    // The removed app is offered in "Add app" again.
    assert!(rig.app.addable_apps().iter().any(|(p, _)| p == NETFLIX));
}

#[test]
fn adding_a_favorite_appends_it_and_takes_it_off_the_add_list() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let addable = rig.app.addable_apps();
    let chrome = addable
        .iter()
        .position(|(p, _)| p == "com.demo.chrome")
        .unwrap() as u32;
    assert_eq!(
        press_on(&mut rig, ADD_APP_BASE + chrome),
        [Effect::SaveConfig]
    );
    assert_eq!(rig.app.config.favorites.last().unwrap(), "com.demo.chrome");
    assert!(!rig
        .app
        .addable_apps()
        .iter()
        .any(|(p, _)| p == "com.demo.chrome"));
}

#[test]
fn the_twelve_favorites_limit_disables_adding() {
    let catalog = many_apps(20);
    let mut rig = Rig::with_catalog(catalog.clone());
    rig.app.config.favorites = catalog
        .apps()
        .iter()
        .take(12)
        .map(|a| a.package.clone())
        .collect();
    open_editor(&mut rig);
    let focusable_adds = rig
        .app
        .focus
        .rows()
        .iter()
        .flat_map(|r| r.items.iter())
        .filter(|i| (ADD_APP_BASE..CONFIRM_RESTORE).contains(&i.id))
        .count();
    assert_eq!(focusable_adds, 0, "no \"Add app\" chip can take the focus");
    assert!(rig.app.config.favorites_full());
    assert!(!rig.app.config.add_favorite("com.demo.app19"));
}

#[test]
fn restoring_the_defaults_asks_first() {
    let mut rig = Rig::new();
    rig.app.config.remove_favorite(0);
    open_editor(&mut rig);
    assert_eq!(press_on(&mut rig, RESTORE), []);
    assert!(rig.app.editor.confirm, "the dialog is open");
    assert_eq!(
        rig.focused(),
        Some(CONFIRM_CANCEL),
        "the safe choice has the focus"
    );
    assert_eq!(rig.app.config.favorites.len(), 4, "nothing changed yet");
    // Only the dialog's buttons can take the focus.
    assert!(rig.app.focus.rows().iter().all(|r| r
        .items
        .iter()
        .all(|i| i.id == CONFIRM_RESTORE || i.id == CONFIRM_CANCEL)));
    rig.press(Key::Left);
    assert_eq!(rig.focused(), Some(CONFIRM_RESTORE));
    assert_eq!(rig.press(Key::Ok), [Effect::SaveConfig]);
    assert!(!rig.app.editor.confirm);
    assert_eq!(rig.app.config, HomeConfig::first_use(&rig.app.catalog));
}

#[test]
fn cancelling_the_restore_changes_nothing() {
    let mut rig = Rig::new();
    rig.app.config.remove_favorite(0);
    let before = rig.app.config.clone();
    open_editor(&mut rig);
    press_on(&mut rig, RESTORE);
    assert_eq!(rig.press(Key::Ok), [], "the focus starts on Cancel");
    assert_eq!(rig.app.config, before);
    assert!(!rig.app.editor.confirm);
    assert_eq!(rig.focused(), Some(RESTORE));
    // Back also closes the dialog without leaving the editor.
    press_on(&mut rig, RESTORE);
    assert_eq!(rig.press(Key::Back), []);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert!(!rig.app.editor.confirm);
}

#[test]
fn restoring_keeps_the_usage_history() {
    let mut rig = Rig::new();
    rig.app.usage.record_open(NETFLIX, NOW);
    open_editor(&mut rig);
    press_on(&mut rig, RESTORE);
    rig.press_all(&[Key::Left, Key::Ok]);
    assert_eq!(rig.app.usage.opens_in_last_7_days(NETFLIX, NOW), 1);
}

#[test]
fn the_editor_scrolls_to_keep_the_focus_visible() {
    let catalog = many_apps(30);
    let mut rig = Rig::with_catalog(catalog.clone());
    rig.app.config.favorites = catalog
        .apps()
        .iter()
        .take(12)
        .map(|a| a.package.clone())
        .collect();
    open_editor(&mut rig);
    assert_eq!(rig.app.scroll_y, 0.0);
    focus_on(&mut rig, DONE);
    rig.advance(1000);
    assert!(
        rig.app.scroll_y > 100.0,
        "scrolled to the bottom button: {}",
        rig.app.scroll_y
    );
    let rect = rig.app.focus.current().unwrap().rect;
    assert!(
        rect.bottom() - rig.app.scroll_y <= 720.0,
        "the focused button is on screen"
    );
    // Back to the first favorite: the page returns to the top.
    focus_on(&mut rig, FAVORITE_BUTTON_BASE + 1);
    rig.advance(1000);
    assert_eq!(rig.app.scroll_y, 0.0);
}

#[test]
fn texts_in_the_editor_follow_the_language() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let list = rig.frame();
    // The screen has plenty of text; a smoke check that it draws without panicking in English too.
    assert!(list.len() > 50);
    rig.app.translator = Translator::new(&["en"]);
    assert!(rig.frame().len() > 50);
    assert_eq!(
        rig.app.translator.get(crate::i18n::StringId::EditorDone),
        "Done"
    );
}

#[test]
fn round_favorites_toggles_with_ok_and_emits_save_config() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    assert_eq!(rig.focused(), Some(ROUND_FAVORITES));
    assert!(!rig.app.config.round_favorites);

    let effects = rig.press(Key::Ok);
    assert_eq!(effects, [Effect::SaveConfig]);
    assert!(rig.app.config.round_favorites);

    let effects = rig.press(Key::Ok);
    assert_eq!(effects, [Effect::SaveConfig]);
    assert!(!rig.app.config.round_favorites);
}

#[test]
fn restore_defaults_resets_round_favorites_to_false() {
    let mut rig = Rig::new();
    rig.app.config.round_favorites = true;
    open_editor(&mut rig);
    press_on(&mut rig, RESTORE);
    press_on(&mut rig, CONFIRM_RESTORE);
    assert!(!rig.app.config.round_favorites);
}

#[test]
fn editor_fits_six_cards_per_line_with_no_horizontal_overflow() {
    let mut rig = Rig::new();
    let catalog = many_apps(12);
    rig.app.catalog = catalog.clone();
    rig.app.config.favorites = catalog
        .apps()
        .iter()
        .take(12)
        .map(|a| a.package.clone())
        .collect();
    open_editor(&mut rig);
    let rows = rig.app.focus.rows();
    // Row 0 is ROUND_FAVORITES (1 item)
    assert_eq!(rows[0].items.len(), 1);
    assert_eq!(rows[0].items[0].id, ROUND_FAVORITES);
    // Row 1 is the first line of 6 cards * 3 buttons = 18 buttons
    assert_eq!(rows[1].items.len(), 18);
    // Row 2 is the second line of 6 cards * 3 buttons = 18 buttons
    assert_eq!(rows[2].items.len(), 18);
}

fn images(list: &DisplayList) -> usize {
    list.cmds()
        .iter()
        .filter(|c| matches!(c, crate::render::Cmd::Image(_)))
        .count()
}

fn glyphs(list: &DisplayList) -> usize {
    list.cmds()
        .iter()
        .filter(|c| matches!(c, crate::render::Cmd::Glyph(_)))
        .count()
}

#[test]
fn a_grid_tile_shows_the_icon_instead_of_its_name() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    let before = rig.frame();
    assert_eq!(images(&before), 0, "no icon is loaded yet");

    rig.app
        .icons
        .reserve(NETFLIX, crate::app_icons::Art::Icon)
        .unwrap();
    let after = rig.frame();
    assert_eq!(images(&after), 1);
    // The name inside the tile is gone; the one printed under it stays.
    assert!(glyphs(&after) < glyphs(&before));
}

#[test]
fn a_banner_in_the_grid_leaves_only_the_name_under_the_tile() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    let before = rig.frame();
    rig.app
        .icons
        .reserve(NETFLIX, crate::app_icons::Art::Banner)
        .unwrap();
    let after = rig.frame();
    assert_eq!(images(&after), 1);
    assert!(glyphs(&after) < glyphs(&before));
}

fn many_apps(count: usize) -> Catalog {
    let entries: Vec<Discovered> = (0..count)
        .map(|i| discovered(&format!("com.demo.app{i:02}"), &format!("App {i:02}")))
        .collect();
    Catalog::from_discovered(&entries, "app.lume.tv")
}

#[test]
fn an_idle_apps_grid_taller_than_the_screen_asks_for_no_frames() {
    let mut rig = Rig::with_catalog(many_apps(30));
    open_apps(&mut rig);
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::GRID_BASE));
    for step in 0..120 {
        rig.advance(100);
        let deadline = rig.app.next_deadline(rig.now);
        // The focus animation of the first second is normal.
        assert!(
            step < 10 || deadline.is_none_or(|at| at > rig.now + 1_000),
            "step {step}: still animating, next deadline {deadline:?} at {}",
            rig.now
        );
    }
}

#[test]
fn the_apps_grid_is_drawn_in_few_batches_however_many_tiles_it_has() {
    // Tiles are drawn in passes (boxes, images, text): alternating the programs tile by tile
    // made one draw call per kind for every tile, which a TV GPU takes a toll on.
    let mut few = Rig::with_catalog(many_apps(5));
    let mut many = Rig::with_catalog(many_apps(15));
    for rig in [&mut few, &mut many] {
        open_apps(rig);
        for i in 0..rig.app.catalog.apps().len() {
            let package = rig.app.catalog.apps()[i].package.clone();
            let art = if i % 2 == 0 {
                crate::app_icons::Art::Banner
            } else {
                crate::app_icons::Art::Icon
            };
            rig.app.icons.reserve(&package, art).unwrap();
        }
    }
    let batches = |rig: &mut Rig| crate::render::Frame::build(&rig.frame()).batches.len();
    let (small, large) = (batches(&mut few), batches(&mut many));
    assert_eq!(small, large, "ten more tiles must not add draw calls");
    assert!(large <= 16, "{large} draw calls");
}

// ---- top bar focus ------------------------------------------------------------------------------

/// The blue underlines of the tabs: left edge and opacity of each.
fn underlines(list: &DisplayList) -> Vec<(f32, f32)> {
    list.cmds()
        .iter()
        .filter_map(|cmd| match cmd {
            crate::render::Cmd::Rect(r)
                if r.rect.y == 64.0
                    && r.rect.w > 10.0
                    && r.rect.h == crate::theme::vars::TAB_UNDERLINE
                    && matches!(r.kind, crate::render::RectKind::Fill(_)) =>
            {
                Some((r.rect.x, r.opacity))
            }
            _ => None,
        })
        .collect()
}

/// The rings drawn inside the top bar's tab row (tabs and gear).
fn top_bar_rings(list: &DisplayList) -> Vec<crate::render::RectCmd> {
    list.cmds()
        .iter()
        .filter_map(|cmd| match cmd {
            crate::render::Cmd::Rect(r)
                if r.rect.bottom() <= 88.0
                    && matches!(r.kind, crate::render::RectKind::Ring { .. }) =>
            {
                Some(*r)
            }
            _ => None,
        })
        .collect()
}

fn underline_alpha(list: &DisplayList) -> Vec<(f32, u8)> {
    list.cmds()
        .iter()
        .filter_map(|cmd| match cmd {
            crate::render::Cmd::Rect(r)
                if r.rect.y == 64.0
                    && r.rect.w > 10.0
                    && r.rect.h == crate::theme::vars::TAB_UNDERLINE =>
            {
                match r.kind {
                    crate::render::RectKind::Fill(crate::theme::Paint::Solid(c)) => {
                        Some((r.rect.x, c.a))
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_focused_tab_has_no_ring() {
    let mut rig = Rig::new();
    rig.press(Key::Up);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME));
    rig.advance(1000);
    assert!(top_bar_rings(&rig.frame()).is_empty());
    rig.press(Key::Right);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::TAB_APPS));
    assert!(top_bar_rings(&rig.frame()).is_empty());
}

#[test]
fn the_underline_is_under_the_active_tab_when_the_focus_is_in_the_content() {
    let mut rig = Rig::new();
    rig.advance(1000);
    let list = rig.frame();
    let lines = underline_alpha(&list);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].1, 255);
    let home = rig.app_tab_text_x(Tab::Home);
    assert!((lines[0].0 - (home - 2.0)).abs() < 0.5);
    assert_eq!(underlines(&list).len(), 1);
}

#[test]
fn the_underline_follows_the_focus_on_the_tabs_and_the_screen_changes_with_it() {
    let mut rig = Rig::new();
    rig.press(Key::Up);
    rig.advance(1000);
    let lines = underline_alpha(&rig.frame());
    assert_eq!(lines.len(), 1, "on the active tab: {lines:?}");
    let home_x = rig.app_tab_text_x(Tab::Home) - 2.0;
    assert!((lines[0].0 - home_x).abs() < 0.5);

    // Focus on the other tab: the underline moves there and the screen changes at once, no OK.
    rig.press(Key::Right);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::TAB_APPS));
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    let lines = underline_alpha(&rig.frame());
    assert_eq!(lines.len(), 1, "{lines:?}");
    let apps_x = rig.app_tab_text_x(Tab::Apps) - 2.0;
    assert!((lines[0].0 - apps_x).abs() < 0.5, "{lines:?}");

    // The gear: the underline stays under the tab that is showing.
    rig.press(Key::Right);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    let lines = underline_alpha(&rig.frame());
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!((lines[0].0 - apps_x).abs() < 0.5);

    // Back to the first tab: it shows Home again, and the underline follows.
    rig.press_all(&[Key::Left, Key::Left]);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME));
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    let lines = underline_alpha(&rig.frame());
    assert_eq!(lines.len(), 1);
    assert!((lines[0].0 - home_x).abs() < 0.5);

    // Down from a tab enters the content of the tab that is showing.
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn the_underline_moves_with_a_transition() {
    let mut rig = Rig::new();
    rig.press(Key::Up);
    rig.advance(1000);
    rig.app.handle_key(Key::Right, rig.now);
    rig.advance(10);
    rig.advance(60);
    let lines = underline_alpha(&rig.frame());
    assert_eq!(lines.len(), 2, "both fade during the move: {lines:?}");
    assert!(lines.iter().all(|(_, a)| *a > 0 && *a < 255), "{lines:?}");
}

#[test]
fn the_focused_gear_has_a_blue_ring() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    rig.advance(1000);
    let rings = top_bar_rings(&rig.frame());
    assert_eq!(rings.len(), 1, "{rings:?}");
    let crate::render::RectKind::Ring { color, .. } = rings[0].kind else {
        unreachable!()
    };
    assert_eq!(
        (color.r, color.g, color.b),
        (
            crate::theme::vars::ACCENT.r,
            crate::theme::vars::ACCENT.g,
            crate::theme::vars::ACCENT.b
        )
    );
}

impl Rig {
    /// Where the label of `tab` starts in the top bar.
    fn app_tab_text_x(&mut self, tab: Tab) -> f32 {
        let tabs = topbar::BarLayout::new(
            &mut Painter {
                list: &mut DisplayList::new(),
                text: &mut self.text,
                display: self.display,
            },
            &self.app.translator.get(crate::i18n::StringId::TabHome),
            &self.app.translator.get(crate::i18n::StringId::TabApps),
            &self.app.clock,
        );
        tabs.text[tab as usize].0
    }
}

#[test]
fn down_from_the_favorites_goes_to_done_and_restore_is_to_its_left() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    assert_eq!(rig.focused(), Some(ROUND_FAVORITES));
    // Down goes to the cards.
    rig.press(Key::Down);
    assert!(
        rig.focused()
            .is_some_and(|id| (FAVORITE_BUTTON_BASE..ADD_APP_BASE).contains(&id)),
        "the cards come after round favorites"
    );
    // Down from the cards goes to the "Add app" chips.
    rig.press(Key::Down);
    assert!(
        rig.focused().is_some_and(|id| id >= ADD_APP_BASE),
        "the chips come after cards"
    );
    rig.press_all(&[Key::Down, Key::Down]);
    assert_eq!(rig.focused(), Some(DONE));
    // "Restore defaults" is at the opposite side: Left from "Done", and back with Right.
    rig.press(Key::Left);
    assert_eq!(rig.focused(), Some(RESTORE));
    let restore_x = rig.app.focus.current().unwrap().rect.x;
    rig.press(Key::Right);
    assert_eq!(rig.focused(), Some(DONE));
    let done = rig.app.focus.current().unwrap().rect;
    assert!(restore_x < 100.0, "at the left margin");
    assert!(done.right() > 1100.0, "at the right margin");
    // Up from "Done" returns to the "Add app" chips.
    rig.press(Key::Up);
    assert!(rig.focused().is_some_and(|id| id >= ADD_APP_BASE));
    // The first card cannot move left: that button takes no focus.
    assert!(!rig.app.focus.focus(FAVORITE_BUTTON_BASE));
}

#[test]
fn from_an_empty_home_the_add_favorites_item_opens_the_editor_at_the_favorites() {
    let mut rig = Rig::new();
    rig.app.config.favorites.clear();
    rig.press(Key::Home);
    assert_eq!(rig.focused(), Some(ids::FAVORITES_ADD));
    rig.press(Key::Ok);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    // The focus starts on "Round favorites".
    assert_eq!(rig.focused(), Some(ROUND_FAVORITES));
    // Down goes to the first "Add app" chip.
    rig.press(Key::Down);
    assert!(rig.focused().is_some_and(|id| id >= ADD_APP_BASE));
    // Down through the chip lines: the last one lands on "Done".
    rig.press_all(&[Key::Down, Key::Down, Key::Down]);
    assert_eq!(rig.focused(), Some(DONE));
}

// ---- wallpaper --------------------------------------------------------------------------------

/// A fill of the whole visible area: what the screens used to paint as their background.
fn full_screen_fills(list: &DisplayList, visible: Rect) -> usize {
    list.cmds()
        .iter()
        .filter(|cmd| {
            matches!(cmd, crate::render::Cmd::Rect(r)
                if matches!(r.kind, crate::render::RectKind::Fill(_))
                    && r.rect.w >= visible.w
                    && r.rect.h >= visible.h)
        })
        .count()
}

#[test]
fn every_screen_starts_with_the_wallpaper_and_has_no_flat_background() {
    let visible = Display::new(1280, 720).visible_rect();
    let mut rig = Rig::new();
    let mut lists = vec![("home", rig.frame())];
    open_apps(&mut rig);
    lists.push(("apps", rig.frame()));
    let mut rig = Rig::new();
    open_settings(&mut rig);
    lists.push(("settings", rig.frame()));
    rig.press_all(&[Key::Down, Key::Down, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::About);
    lists.push(("about", rig.frame()));
    let mut rig = Rig::new();
    open_tv_info(&mut rig);
    rig.app
        .apply_device_info(crate::device_info::DeviceInfo::default());
    lists.push(("tv information", rig.frame()));
    let mut rig = Rig::new();
    open_editor(&mut rig);
    lists.push(("editor", rig.frame()));
    for (name, list) in lists {
        let Some(crate::render::Cmd::Wallpaper(quad)) = list.cmds().first() else {
            panic!("{name}: the first command is not the wallpaper");
        };
        assert_eq!(quad.dest, visible, "{name}");
        assert_eq!(
            quad.uv,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            "{name}: 16:9 uses it whole"
        );
        assert_eq!(
            list.cmds()
                .iter()
                .filter(|c| matches!(c, crate::render::Cmd::Wallpaper(_)))
                .count(),
            1,
            "{name}: drawn once"
        );
        // No flat background or dimming layer anywhere: the dimming is in the wallpaper's own draw.
        assert_eq!(
            full_screen_fills(&list, visible),
            0,
            "{name}: no full-screen fill over the wallpaper"
        );
    }
}

#[test]
fn on_other_proportions_the_wallpaper_covers_the_whole_surface_without_stretching() {
    for (w, h) in [(1920, 1200), (2560, 1080), (1024, 768)] {
        let display = Display::new(w, h);
        let mut list = DisplayList::new();
        let mut text = TextEngine::new(display.scale);
        let mut painter = Painter {
            list: &mut list,
            text: &mut text,
            display,
        };
        painter.wallpaper(false);
        let Some(crate::render::Cmd::Wallpaper(quad)) = list.cmds().first() else {
            panic!()
        };
        assert_eq!(quad.dest, display.visible_rect(), "{w}x{h}");
        // The shown part keeps the proportion of the surface: nothing is stretched.
        let image = crate::wallpaper::current();
        let shown = (quad.uv.w * image.width as f32) / (quad.uv.h * image.height as f32);
        assert!(
            (shown - w as f32 / h as f32).abs() < 1e-3,
            "{w}x{h}: {shown}"
        );
        assert!(
            quad.uv.x >= 0.0
                && quad.uv.right() <= 1.0001
                && quad.uv.y >= 0.0
                && quad.uv.bottom() <= 1.0001
        );
    }
}

// ---- contrast over the wallpaper ----------------------------------------------------------------

fn relative_luminance(rgb: [f32; 3]) -> f32 {
    let linear = |c: f32| {
        let c = c / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

fn contrast(text: crate::theme::Color, background_luminance: f32) -> f32 {
    let text = relative_luminance([text.r, text.g, text.b].map(f32::from));
    (text.max(background_luminance) + 0.05) / (text.min(background_luminance) + 0.05)
}

/// The brightest texel of the wallpaper inside `area` (logical units of the 1280x720 canvas),
/// after the layer `dim` (if any) is drawn over it.
fn brightest_behind(area: Rect, dim: Option<crate::theme::Color>) -> f32 {
    let wallpaper = crate::wallpaper::current();
    let (sx, sy) = (
        wallpaper.width as f32 / 1280.0,
        wallpaper.height as f32 / 720.0,
    );
    let mut brightest = 0.0f32;
    for y in (area.y * sy) as usize
        ..((area.bottom() * sy).ceil() as usize).min(wallpaper.height as usize)
    {
        for x in (area.x * sx) as usize
            ..((area.right() * sx).ceil() as usize).min(wallpaper.width as usize)
        {
            let i = (y * wallpaper.width as usize + x) * 3;
            let mut pixel = [0, 1, 2].map(|c| f32::from(wallpaper.rgb[i + c]));
            if let Some(dim) = dim {
                let a = f32::from(dim.a) / 255.0;
                let shade = [dim.r, dim.g, dim.b].map(f32::from);
                for c in 0..3 {
                    pixel[c] = pixel[c] * (1.0 - a) + shade[c] * a;
                }
            }
            brightest = brightest.max(relative_luminance(pixel));
        }
    }
    brightest
}

#[test]
fn text_on_the_dimmed_screens_has_4_5_to_1_over_the_brightest_part_of_the_wallpaper() {
    let everywhere = Rect::new(0.0, 0.0, 1280.0, 720.0);
    let behind = brightest_behind(everywhere, Some(painter::TEXT_SCREEN_DIM));
    // The dimmest text color used on those screens (tile names, breadcrumbs, hints).
    let ratio = contrast(crate::theme::vars::TEXT_MUTED, behind);
    assert!(
        ratio >= 4.5,
        "muted text: {ratio:.2}:1 over luminance {behind:.4}"
    );
    assert!(contrast(crate::theme::vars::TEXT, behind) >= 4.5);
}

#[test]
fn the_top_bar_texts_have_4_5_to_1_over_the_undimmed_wallpaper_on_home() {
    let mut rig = Rig::new();
    let tabs = topbar::BarLayout::new(
        &mut Painter {
            list: &mut DisplayList::new(),
            text: &mut rig.text,
            display: rig.display,
        },
        &rig.app.translator.get(crate::i18n::StringId::TabHome),
        &rig.app.translator.get(crate::i18n::StringId::TabApps),
        &rig.app.clock,
    );
    let (home, apps) = (tabs.rect(Tab::Home), tabs.rect(Tab::Apps));
    let tab_area = Rect::new(home.x, home.y, apps.right() - home.x, home.h);
    let muted = brightest_behind(tab_area, None);
    let ratio = contrast(crate::theme::vars::TEXT_MUTED, muted);
    assert!(
        ratio >= 4.5,
        "the inactive tab: {ratio:.2}:1 over luminance {muted:.4}"
    );
    // The white texts: the wordmark, the active tab, the gear and the clock.
    for (name, area) in [
        ("logo", Rect::new(64.0, 20.0, 120.0, 44.0)),
        ("tabs", tab_area),
        ("gear", Rect::new(1096.0, 20.0, 48.0, 48.0)),
        ("clock", Rect::new(1150.0, 28.0, 70.0, 30.0)),
    ] {
        let ratio = contrast(crate::theme::vars::TEXT, brightest_behind(area, None));
        assert!(ratio >= 4.5, "{name}: {ratio:.2}:1");
    }
}

// ---- the dock -----------------------------------------------------------------------------------

fn rings_in(list: &DisplayList, area: Rect) -> usize {
    list.cmds()
        .iter()
        .filter(|cmd| {
            matches!(cmd, crate::render::Cmd::Rect(r)
                if matches!(r.kind, crate::render::RectKind::Ring { .. }) && area.intersects(&r.rect))
        })
        .count()
}

fn favorites_dock(rig: &mut Rig, count: usize) {
    let packages: Vec<String> = rig
        .app
        .catalog
        .apps()
        .iter()
        .take(count)
        .map(|a| a.package.clone())
        .collect();
    rig.app.config.favorites = packages;
    rig.press(Key::Home);
    rig.advance(1000);
}

#[test]
fn the_dock_shows_every_favorite_at_once_inside_the_safe_area() {
    let mut rig = Rig::with_catalog(many_apps(14));
    favorites_dock(&mut rig, 12);
    let safe = Display::new(1280, 720).safe_area();
    let row = &rig.app.focus.rows()[1];
    assert_eq!(row.items.len(), 12);
    assert!(row
        .items
        .iter()
        .all(|i| i.rect.x >= safe.x && i.rect.right() <= safe.right()));
    // Even the last one with the focus on it, at the peak of the Magnification.
    rig.press_all(&[Key::Right; 11]);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 11));
    let (position, amplitude) = rig.app.dock_motion();
    let layout = dock::layout(12, position, amplitude, 640.0, safe.bottom());
    assert!(layout
        .items
        .iter()
        .all(|r| r.x >= safe.x && r.right() <= safe.right()));
}

#[test]
fn the_magnification_follows_the_focus_with_an_animation_and_no_jump() {
    let mut rig = Rig::new();
    rig.advance(1000);
    let (position, amplitude) = rig.app.dock_motion();
    assert_eq!(
        (position, amplitude),
        (0.0, 1.0),
        "the first icon is magnified"
    );
    rig.press_all(&[Key::Right, Key::Right]);
    rig.advance(1000);
    assert_eq!(rig.app.dock_motion(), (2.0, 1.0));
    // Right: the position slides from 2 to 3 in steps, never leaping.
    rig.app.handle_key(Key::Right, rig.now);
    rig.advance(10);
    let mut last = rig.app.dock_motion().0;
    assert!((2.0..3.0).contains(&last));
    for _ in 0..12 {
        rig.advance(16);
        let now = rig.app.dock_motion().0;
        assert!(now >= last && now - last < 0.5, "{last} -> {now}");
        last = now;
    }
    rig.advance(500);
    assert_eq!(rig.app.dock_motion(), (3.0, 1.0));
}

#[test]
fn with_the_focus_outside_the_dock_every_icon_is_at_its_normal_size() {
    let mut rig = Rig::new();
    rig.advance(1000);
    rig.press(Key::Up);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME));
    let (position, amplitude) = rig.app.dock_motion();
    assert_eq!(amplitude, 0.0);
    let layout = dock::layout(5, position, amplitude, 640.0, 700.0);
    assert!(layout.items.iter().all(|r| r.w == dock::BASE));
    // Coming back magnifies the icon the focus lands on, without sliding from the old place.
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
    rig.advance(1000);
    assert_eq!(rig.app.dock_motion(), (0.0, 1.0));
}

#[test]
fn a_focused_dock_item_has_no_ring_around_it_only_the_bar_has_one() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Right, Key::Right]);
    rig.advance(1000);
    let list = rig.frame();
    let dock_area = Rect::new(0.0, 400.0, 1280.0, 320.0);
    assert_eq!(
        rings_in(&list, dock_area),
        1,
        "the bar's own thin ring, and nothing on the icon"
    );
    // Nothing is drawn above the bar's top besides the name bubble: no outline on the icon.
    assert!(top_bar_rings(&list).is_empty());
}

#[test]
fn the_name_bubble_appears_with_the_focus_and_goes_with_it() {
    let mut rig = Rig::new();
    rig.advance(1000);
    let with_focus = glyphs(&rig.frame());
    rig.press(Key::Up);
    rig.advance(1000);
    let without = glyphs(&rig.frame());
    assert!(
        with_focus > without,
        "the name is drawn only while the focus is in the dock"
    );
    rig.press_all(&[Key::Down, Key::Right]);
    rig.advance(1000);
    assert!(
        glyphs(&rig.frame()) > without,
        "the bubble is back with the focus"
    );
}

#[test]
fn left_and_right_stop_at_the_ends_up_goes_to_the_active_tab_and_down_does_nothing() {
    let mut rig = Rig::new();
    rig.press(Key::Left);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE),
        "no step left of the first"
    );
    rig.press_all(&[Key::Right; 10]);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 4),
        "no step right of the last"
    );
    rig.press(Key::Down);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 4),
        "nothing below the dock"
    );
    rig.press(Key::Up);
    assert_eq!(rig.focused(), Some(ids::TAB_HOME), "the active tab");
    // From the top bar, down goes to the first icon of the screen that is showing.
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
    // Going to the gear passes over the Apps tab, which shows its grid: down from the gear lands on
    // the grid's first tile.
    rig.press_all(&[Key::Up, Key::Right, Key::Right, Key::Down]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), Some(ids::GRID_BASE), "from the gear too");
}

#[test]
fn ok_on_a_dock_icon_opens_its_app() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Right, Key::Right, Key::Right]);
    let effects = rig.press(Key::Ok);
    assert_eq!(
        effects[0],
        Effect::OpenApp {
            package: SPOTIFY.into(),
            activity: format!("{SPOTIFY}.Main")
        }
    );
}

#[test]
fn the_empty_dock_has_a_single_add_favorites_item_that_opens_the_editor() {
    let mut rig = Rig::new();
    rig.app.config.favorites.clear();
    rig.press(Key::Home);
    rig.advance(1000);
    let row = &rig.app.focus.rows()[1];
    assert_eq!(row.items.len(), 1);
    assert_eq!(row.items[0].id, ids::FAVORITES_ADD);
    assert_eq!(rig.focused(), Some(ids::FAVORITES_ADD));
    rig.press(Key::Right);
    assert_eq!(rig.focused(), Some(ids::FAVORITES_ADD));
    assert_eq!(rig.press(Key::Ok), []);
    assert_eq!(rig.app.screen(), &Screen::Editor);
}

#[test]
fn the_first_frame_is_drawn_with_colors_and_initials_before_any_icon_arrives() {
    let mut rig = Rig::new();
    let before = rig.frame();
    assert_eq!(images(&before), 0, "no icon is loaded yet");
    assert!(
        glyphs(&before) > 5,
        "the initials and the name are already there"
    );
    rig.app.icons.reserve_dock(NETFLIX).unwrap();
    let after = rig.frame();
    assert_eq!(images(&after), 1);
    assert!(
        glyphs(&after) < glyphs(&before),
        "its initial gave way to the icon"
    );
}

#[test]
fn an_idle_home_asks_for_no_frame_but_the_clocks() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Right, Key::Right]);
    rig.advance(1000);
    let now = rig.now;
    let deadline = rig.app.next_deadline(now).expect("the clock");
    assert_eq!(
        deadline,
        now + rig.app.clock.millis_to_next_minute(),
        "only the clock turning"
    );
    // And after a long wait the frame still has the same few things to draw.
    let busy = rig.frame().len();
    rig.advance(30_000);
    assert_eq!(rig.frame().len(), busy);
    assert_eq!(
        rig.app.next_deadline(rig.now),
        Some(rig.now + rig.app.clock.millis_to_next_minute())
    );
}

// ---- coming back from an opened app ----------------------------------------------------------------

/// What happens to lume when an app takes the screen and the user comes back with Home.
fn leave_and_come_back(rig: &mut Rig) {
    rig.app.went_to_background();
    rig.app.came_to_foreground();
    rig.press(Key::Home);
}

fn grid_id_of(rig: &Rig, package: &str) -> FocusId {
    let index = rig
        .app
        .catalog
        .apps()
        .iter()
        .position(|a| a.package == package)
        .unwrap();
    ids::GRID_BASE + index as u32
}

#[test]
fn back_from_an_app_opened_in_the_apps_tab_returns_to_that_tab_and_icon() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    let youtube = grid_id_of(&rig, YOUTUBE);
    focus_on(&mut rig, youtube);
    let effects = rig.press(Key::Ok);
    assert!(matches!(&effects[0], Effect::OpenApp { package, .. } if package == YOUTUBE));
    // The user comes back with the Home button.
    leave_and_come_back(&mut rig);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), Some(youtube));
}

#[test]
fn back_from_an_app_opened_from_the_dock_returns_to_that_icon() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Right, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 3));
    rig.press(Key::Ok);
    leave_and_come_back(&mut rig);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 3),
        "not the first icon"
    );
}

#[test]
fn the_grid_scroll_is_kept_when_coming_back() {
    let mut rig = Rig::with_catalog(many_apps(40));
    open_apps(&mut rig);
    rig.press_all(&[Key::Down; 6]);
    rig.advance(1000);
    let (focus, scroll) = (rig.focused().unwrap(), rig.app.scroll_y);
    assert!(scroll > 0.0, "the grid scrolled");
    rig.press(Key::Ok);
    leave_and_come_back(&mut rig);
    rig.advance(1000);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), Some(focus));
    assert!(
        (rig.app.scroll_y - scroll).abs() < 1.0,
        "{} vs {scroll}",
        rig.app.scroll_y
    );
    let rect = rig.app.focus.current().unwrap().rect;
    assert!(rect.y - rig.app.scroll_y >= 76.0 && rect.bottom() - rig.app.scroll_y <= 720.0);
}

#[test]
fn back_from_the_settings_opened_by_the_lume_tile_returns_to_the_tile() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    focus_on(&mut rig, ids::GRID_SETTINGS);
    rig.press(Key::Ok);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
}

#[test]
fn coming_back_from_the_android_settings_keeps_the_gear_focused() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    assert_eq!(rig.press(Key::Ok), [Effect::OpenAndroidSettings]);
    leave_and_come_back(&mut rig);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), Some(ids::GEAR));
}

#[test]
fn any_other_key_forgets_the_place_so_home_goes_to_the_start() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Ok);
    // lume came back by Back (no Home press): the screen is already as it was, and the user moves.
    rig.press(Key::Right);
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE),
        "the usual Home press"
    );
}

#[test]
fn home_without_an_app_having_been_opened_goes_to_the_start() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press_all(&[Key::Down, Key::Right]);
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn the_place_is_used_once() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Ok);
    leave_and_come_back(&mut rig);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    rig.advance(5000);
    rig.press(Key::Home);
    assert_eq!(
        rig.app.screen(),
        &Screen::Main(Tab::Home),
        "the second press is the usual one"
    );
}

#[test]
fn an_app_that_could_not_be_opened_leaves_nothing_to_go_back_to() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Ok);
    rig.app.app_not_found(rig.now);
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
}

#[test]
fn home_pressed_while_lume_is_on_screen_goes_to_the_start_even_after_opening_an_app() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Ok);
    // lume never left the screen (the app did not open, or the user came back some other way).
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn a_home_press_long_after_coming_back_is_the_usual_one() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Ok);
    rig.app.went_to_background();
    rig.app.came_to_foreground();
    rig.advance(100); // the first frame back
    rig.advance(5000);
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn a_home_press_that_arrives_just_after_the_first_frame_back_still_restores() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    let focus = rig.focused();
    rig.press(Key::Ok);
    rig.app.went_to_background();
    rig.app.came_to_foreground();
    rig.advance(100);
    rig.press(Key::Home);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.focused(), focus);
}

/// The horizontal center of the name bubble above the dock icons, if it is drawn.
fn bubble_center(list: &DisplayList) -> Option<f32> {
    list.cmds().iter().find_map(|cmd| match cmd {
        crate::render::Cmd::Rect(r)
            if r.rect.h == 40.0
                && r.rect.y > 400.0
                && matches!(r.kind, crate::render::RectKind::Fill(_)) =>
        {
            Some(r.rect.center().0)
        }
        _ => None,
    })
}

#[test]
fn the_name_bubble_glides_with_the_focus_between_the_first_and_second_icon() {
    let mut rig = Rig::new();
    rig.advance(1000);
    let start = bubble_center(&rig.frame()).expect("the bubble is drawn");
    rig.app.handle_key(Key::Right, rig.now);
    let mut last = start;
    let mut moved = 0.0f32;
    let mut centers = Vec::new();
    for _ in 0..16 {
        rig.advance(16);
        let center = bubble_center(&rig.frame()).expect("the bubble stays while moving");
        // Frame by frame it only advances a little: no jump to the new icon, no going back.
        assert!(center >= last - 0.01, "went back: {last} -> {center}");
        moved = moved.max(center - last);
        centers.push(center);
        last = center;
    }
    assert!(
        moved < 40.0,
        "the biggest step in one frame was {moved}: {centers:?}"
    );
    assert!(
        last - start > 60.0,
        "it did get to the second icon: {start} -> {last}"
    );
}

#[test]
fn the_first_move_in_the_dock_slides_like_every_other_one() {
    // The position 0 is the value the animations rest at: it must not be forgotten between frames,
    // or the first move would start from the new place and jump instead of sliding.
    let mut rig = Rig::new();
    rig.advance(2000);
    assert_eq!(rig.app.dock_motion(), (0.0, 1.0));
    rig.app.handle_key(Key::Right, rig.now);
    rig.advance(10);
    rig.advance(30);
    let (position, _) = rig.app.dock_motion();
    assert!(position > 0.0 && position < 0.9, "sliding, at {position}");
    rig.advance(1000);
    assert_eq!(rig.app.dock_motion(), (1.0, 1.0));
    // And back to the first icon slides too.
    rig.app.handle_key(Key::Left, rig.now);
    rig.advance(10);
    rig.advance(30);
    let (position, _) = rig.app.dock_motion();
    assert!(
        position > 0.1 && position < 1.0,
        "sliding back, at {position}"
    );
}

#[test]
fn moving_the_focus_to_a_tab_shows_it_without_ok_and_ok_on_it_changes_nothing() {
    let mut rig = Rig::new();
    rig.press(Key::Up);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    rig.press(Key::Right);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps), "no OK needed");
    assert_eq!(
        rig.focused(),
        Some(ids::TAB_APPS),
        "the focus stays on the tab"
    );
    let labels_before = glyphs(&rig.frame());
    assert!(rig.press(Key::Ok).is_empty());
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(glyphs(&rig.frame()), labels_before);
    // And left shows Home again.
    rig.press(Key::Left);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::TAB_HOME));
}

#[test]
fn the_apps_tab_scrolled_back_to_the_top_when_the_focus_arrives_on_it() {
    let mut rig = Rig::with_catalog(many_apps(40));
    open_apps(&mut rig);
    rig.press_all(&[Key::Down; 6]);
    rig.advance(1000);
    assert!(rig.app.scroll_y > 0.0);
    rig.press_all(&[Key::Up; 7]);
    assert_eq!(rig.focused(), Some(ids::TAB_APPS));
    rig.press(Key::Left);
    rig.press(Key::Right);
    rig.advance(1000);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(rig.app.scroll_y, 0.0);
}

/// The color of the (only) tab underline in the frame.
fn underline_color(list: &DisplayList) -> crate::theme::Color {
    list.cmds()
        .iter()
        .find_map(|cmd| match cmd {
            crate::render::Cmd::Rect(r)
                if r.rect.y == 64.0
                    && r.rect.w > 10.0
                    && r.rect.h == crate::theme::vars::TAB_UNDERLINE =>
            {
                match r.kind {
                    crate::render::RectKind::Fill(crate::theme::Paint::Solid(c)) => Some(c),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("an underline is drawn")
}

fn rgb(c: crate::theme::Color) -> [u8; 3] {
    [c.r, c.g, c.b]
}

#[test]
fn the_underline_is_blue_while_the_tabs_have_the_focus_and_white_when_it_goes_down() {
    let mut rig = Rig::new();
    rig.advance(1000);
    // The focus is in the dock: the tabs are not being controlled, the underline is white.
    let white = crate::theme::vars::TEXT;
    let blue = crate::theme::vars::ACCENT;
    assert_eq!(rgb(underline_color(&rig.frame())), rgb(white));
    // Up to the tabs: blue.
    rig.press(Key::Up);
    rig.advance(1000);
    assert_eq!(rgb(underline_color(&rig.frame())), rgb(blue));
    // Down again: white.
    rig.press(Key::Down);
    rig.advance(1000);
    assert_eq!(rgb(underline_color(&rig.frame())), rgb(white));
    // The gear belongs to the top bar too.
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    assert_eq!(rgb(underline_color(&rig.frame())), rgb(blue));
}

#[test]
fn the_underline_fades_between_white_and_blue_with_the_focus() {
    let mut rig = Rig::new();
    rig.advance(1000);
    rig.app.handle_key(Key::Up, rig.now);
    rig.advance(10);
    rig.advance(60);
    let c = underline_color(&rig.frame());
    let (white, blue) = (crate::theme::vars::TEXT, crate::theme::vars::ACCENT);
    assert!(
        c.r < white.r && c.r > blue.r,
        "between white and blue: {c:?}"
    );
}

#[test]
fn in_the_apps_grid_the_underline_is_white_too() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.advance(1000);
    assert_eq!(rig.focused(), Some(ids::GRID_BASE));
    assert_eq!(
        rgb(underline_color(&rig.frame())),
        rgb(crate::theme::vars::TEXT)
    );
}

#[test]
fn changing_the_tab_fades_the_new_content_in_and_then_goes_idle() {
    let mut rig = Rig::new();
    rig.press(Key::Up);
    rig.advance(1000);
    rig.app.handle_key(Key::Right, rig.now);
    rig.advance(10);
    let mid = rig.app.anim.get(super::Anim::TabFade, 1.0, rig.now);
    assert!(mid > 0.0 && mid < 1.0, "mid-fade: {mid}");
    assert!(
        rig.app.is_animating(rig.now),
        "frames are asked for while it fades"
    );
    // The content is drawn translucent; the top bar after it is not.
    let list = rig.frame();
    assert!(
        list.cmds()
            .iter()
            .any(|c| matches!(c, crate::render::Cmd::Rect(r) if r.opacity < 1.0)),
        "some content is translucent during the fade"
    );
    rig.advance(1000);
    assert!(
        !rig.app.is_animating(rig.now),
        "nothing animates once it is done"
    );
}

#[test]
fn the_name_under_the_focused_tile_is_white_and_the_others_are_muted() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.advance(1000);
    let focused = rig.app.focus_progress(rig.focused().unwrap());
    assert_eq!(focused, 1.0);
    let mut style = |progress: f32| {
        let p = Painter {
            list: &mut DisplayList::new(),
            text: &mut rig.text,
            display: rig.display,
        };
        p.focus_style(&crate::theme::THEME.tile_label, progress)
            .color
    };
    assert_eq!(style(0.0), crate::theme::vars::TEXT_MUTED);
    assert_eq!(style(1.0), crate::theme::vars::TEXT);
}

// ---- TV statistics panel ---------------------------------------------------------------------

fn sample_stats() -> crate::stats::Stats {
    crate::stats::Stats::parse("2000000\t800000\t12000000000\t7000000000\tstat,100,1000").unwrap()
}

#[test]
fn the_statistics_are_read_only_while_the_settings_are_on_screen() {
    let mut rig = Rig::new();
    assert_eq!(rig.app.stats_deadline(rig.now), None, "the home screen");
    open_apps(&mut rig);
    assert_eq!(rig.app.stats_deadline(rig.now), None, "the apps screen");
    rig.press(Key::Down);
    press_on(&mut rig, ids::GRID_SETTINGS);
    assert_eq!(rig.app.screen(), &Screen::Settings);
    assert_eq!(
        rig.app.stats_deadline(rig.now),
        Some(rig.now),
        "due at once"
    );

    rig.app.apply_stats(sample_stats(), rig.now);
    assert_eq!(rig.app.stats_deadline(rig.now), Some(rig.now + 2000));
    assert_eq!(
        rig.app.cpu_use, None,
        "the first reading has nothing to compare with"
    );

    // Leaving for the About screen, the editor or the background stops the readings.
    rig.press_all(&[Key::Down, Key::Down, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::About);
    assert_eq!(rig.app.stats_deadline(rig.now), None);
    rig.press(Key::Back);
    assert!(rig.app.stats_deadline(rig.now).is_some());
    rig.app.went_to_background();
    assert_eq!(rig.app.stats_deadline(rig.now), None);
}

#[test]
fn the_panel_never_asks_for_frames_by_itself() {
    let mut rig = Rig::new();
    rig.press(Key::Down);
    open_settings(&mut rig);
    rig.app.apply_stats(sample_stats(), rig.now);
    rig.advance(3000);
    // The frame deadline (the clock's minute) is far away: the 2 s reading is the platform's
    // timer, and a reading that changes nothing visible draws nothing.
    let deadline = rig.app.next_deadline(rig.now).unwrap();
    assert!(deadline - rig.now > 2000, "{deadline} vs {}", rig.now);
}

#[test]
fn the_cpu_comes_from_two_readings() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    rig.app.apply_stats(sample_stats(), 0);
    let later =
        crate::stats::Stats::parse("2000000\t800000\t12000000000\t7000000000\tstat,400,2000")
            .unwrap();
    rig.app.apply_stats(later, 2000);
    assert!((rig.app.cpu_use.unwrap() - 0.3).abs() < 1e-6);
}

// ---- the context menu (the Menu button) ------------------------------------------------------

use super::MenuAction;

fn menu_actions(rig: &Rig) -> Vec<MenuAction> {
    let menu = rig.app.context_menu().expect("a menu is open");
    menu.items.iter().map(|i| i.action).collect()
}

fn selected_action(rig: &Rig) -> MenuAction {
    let menu = rig.app.context_menu().expect("a menu is open");
    menu.items[menu.selected].action
}

/// Opens the Apps tab and the menu of the app `package`.
fn open_grid_menu(rig: &mut Rig, package: &str) -> FocusId {
    open_apps(rig);
    let id = grid_id_of(rig, package);
    focus_on(rig, id);
    rig.press(Key::Menu);
    id
}

/// Opens the menu of the favorite at `index` of the dock.
fn open_dock_menu(rig: &mut Rig, index: usize) {
    focus_on(rig, ids::FAVORITE_BASE + index as u32);
    rig.press(Key::Menu);
}

#[test]
fn menu_on_an_app_of_the_grid_opens_with_the_focus_on_the_first_item() {
    let mut rig = Rig::new();
    let chrome = open_grid_menu(&mut rig, "com.demo.chrome");
    assert_eq!(
        menu_actions(&rig),
        [
            MenuAction::AddToHome,
            MenuAction::AppInfo,
            MenuAction::Uninstall
        ]
    );
    assert_eq!(selected_action(&rig), MenuAction::AddToHome);
    assert_eq!(
        rig.focused(),
        Some(chrome),
        "the tile keeps the focus under the menu"
    );
}

#[test]
fn menu_does_nothing_on_the_top_bar_the_lume_tile_add_favorites_and_the_settings() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    focus_on(&mut rig, ids::GEAR);
    assert!(rig.press(Key::Menu).is_empty());
    assert!(rig.app.context_menu().is_none());
    assert_eq!(rig.focused(), Some(ids::GEAR));

    focus_on(&mut rig, ids::GRID_SETTINGS);
    rig.press(Key::Menu);
    assert!(rig.app.context_menu().is_none());

    rig.press(Key::Ok);
    assert_eq!(rig.app.screen(), &Screen::Settings);
    rig.press(Key::Menu);
    assert!(rig.app.context_menu().is_none());
    assert_eq!(rig.app.screen(), &Screen::Settings);

    let mut rig = Rig::new();
    rig.app.config.favorites.clear();
    rig.advance(0);
    focus_on(&mut rig, ids::FAVORITES_ADD);
    rig.press(Key::Menu);
    assert!(rig.app.context_menu().is_none());
}

#[test]
fn back_or_menu_close_the_menu_without_changing_anything() {
    for key in [Key::Back, Key::Menu] {
        let mut rig = Rig::new();
        let chrome = open_grid_menu(&mut rig, "com.demo.chrome");
        let favorites = rig.app.config.favorites.clone();
        assert!(rig.press(key).is_empty());
        assert!(rig.app.context_menu().is_none(), "{key:?}");
        assert_eq!(rig.app.config.favorites, favorites);
        assert_eq!(rig.focused(), Some(chrome));
        assert_eq!(
            rig.app.screen(),
            &Screen::Main(Tab::Apps),
            "Back did not leave the tab"
        );
    }
}

#[test]
fn the_focus_is_trapped_in_the_menu() {
    let mut rig = Rig::new();
    let chrome = open_grid_menu(&mut rig, "com.demo.chrome");
    rig.press_all(&[Key::Down, Key::Down, Key::Down, Key::Down]);
    assert_eq!(
        selected_action(&rig),
        MenuAction::Uninstall,
        "stops at the last item"
    );
    rig.press_all(&[Key::Left, Key::Right]);
    assert!(rig.app.context_menu().is_some());
    assert_eq!(rig.focused(), Some(chrome), "the grid did not move");
    rig.press_all(&[Key::Up, Key::Up, Key::Up, Key::Up]);
    assert_eq!(selected_action(&rig), MenuAction::AddToHome);
    assert_eq!(
        rig.focused(),
        Some(chrome),
        "nor did the top bar take the focus"
    );
}

#[test]
fn the_menu_fades_in_and_out_in_150_ms() {
    let mut rig = Rig::new();
    open_grid_menu(&mut rig, "com.demo.chrome");
    rig.advance(200);
    assert_eq!(rig.app.anim.get(Anim::MenuFade, 1.0, rig.now), 1.0);
    rig.app.handle_key(Key::Back, rig.now);
    rig.advance(50);
    let half = rig.app.anim.get(Anim::MenuFade, 1.0, rig.now);
    assert!(half > 0.0 && half < 1.0, "{half}");
    assert!(rig.app.context_menu.is_some(), "still fading");
    rig.advance(150);
    rig.advance(0);
    assert!(rig.app.context_menu.is_none(), "gone after the fade");
}

#[test]
fn home_closes_the_menu_and_goes_home() {
    let mut rig = Rig::new();
    open_grid_menu(&mut rig, "com.demo.chrome");
    rig.press(Key::Home);
    assert!(rig.app.context_menu().is_none());
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
}

#[test]
fn add_to_home_from_the_grid_appends_the_app_and_saves() {
    let mut rig = Rig::new();
    let chrome = open_grid_menu(&mut rig, "com.demo.chrome");
    let effects = rig.press(Key::Ok);
    assert_eq!(effects, [Effect::SaveConfig]);
    assert_eq!(rig.app.config.favorites.last().unwrap(), "com.demo.chrome");
    assert!(rig.app.context_menu().is_none());
    assert_eq!(rig.focused(), Some(chrome));
    // The next menu on it offers to remove it.
    rig.press(Key::Menu);
    assert_eq!(menu_actions(&rig)[0], MenuAction::RemoveFromHome);
}

#[test]
fn remove_from_home_from_the_grid_takes_the_favorite_out() {
    let mut rig = Rig::new();
    let youtube = open_grid_menu(&mut rig, YOUTUBE);
    assert_eq!(selected_action(&rig), MenuAction::RemoveFromHome);
    assert_eq!(rig.press(Key::Ok), [Effect::SaveConfig]);
    assert!(!rig.app.config.favorites.iter().any(|p| p == YOUTUBE));
    assert_eq!(rig.focused(), Some(youtube));
}

#[test]
fn with_twelve_favorites_add_is_disabled_and_skipped() {
    let apps: Vec<(String, String)> = (0..13)
        .map(|i| (format!("com.demo.app{i:02}"), format!("App {i:02}")))
        .collect();
    let refs: Vec<(&str, &str)> = apps.iter().map(|(p, l)| (p.as_str(), l.as_str())).collect();
    let mut rig = Rig::with_catalog(catalog(&refs));
    rig.app.config.favorites = apps[..12].iter().map(|(p, _)| p.clone()).collect();
    rig.advance(0);
    open_grid_menu(&mut rig, "com.demo.app12");
    let menu = rig.app.context_menu().unwrap();
    assert_eq!(menu.items[0].action, MenuAction::AddToHome);
    assert!(!menu.items[0].enabled);
    assert_eq!(
        selected_action(&rig),
        MenuAction::AppInfo,
        "the focus starts after it"
    );
    rig.press(Key::Up);
    assert_eq!(
        selected_action(&rig),
        MenuAction::AppInfo,
        "it never takes the focus"
    );
}

#[test]
fn a_system_app_cannot_be_uninstalled() {
    let mut entries: Vec<Discovered> = [
        (NETFLIX, "Netflix"),
        ("com.android.tv.settings", "Configurações"),
    ]
    .iter()
    .map(|(p, l)| discovered(p, l))
    .collect();
    entries[1].system = true;
    let mut rig = Rig::with_catalog(Catalog::from_discovered(&entries, "app.lume.tv"));
    open_grid_menu(&mut rig, "com.android.tv.settings");
    assert_eq!(
        menu_actions(&rig),
        [MenuAction::AddToHome, MenuAction::AppInfo]
    );
}

#[test]
fn app_info_and_uninstall_ask_the_platform() {
    let mut rig = Rig::new();
    open_grid_menu(&mut rig, "com.demo.chrome");
    rig.press(Key::Down);
    assert_eq!(
        rig.press(Key::Ok),
        [Effect::OpenAppInfo {
            package: "com.demo.chrome".into()
        }]
    );
    rig.press(Key::Menu);
    rig.press_all(&[Key::Down, Key::Down]);
    assert_eq!(
        rig.press(Key::Ok),
        [Effect::Uninstall {
            package: "com.demo.chrome".into()
        }]
    );
}

#[test]
fn an_uninstalled_app_leaves_the_focus_on_the_tile_in_its_place() {
    // The grid: Chrome, Disney+, Jogos, Lume, Netflix, Prime Video, Spotify, YouTube.
    let mut rig = Rig::new();
    open_grid_menu(&mut rig, DISNEY);
    rig.press(Key::Back);
    assert_eq!(rig.app.grid_position, Some(1));
    rig.app.catalog.apply(&[], &[DISNEY.to_string()]);
    rig.app.config.remove_uninstalled(&rig.app.catalog);
    rig.app.catalog_changed();
    rig.advance(0);
    assert!(
        !rig.app.config.favorites.iter().any(|p| p == DISNEY),
        "out of the dock"
    );
    assert_eq!(rig.app.grid_position, Some(1));
    assert_eq!(
        rig.focused(),
        Some(grid_id_of(&rig, "com.demo.games")),
        "Jogos took its place"
    );
    // The last tile gives the focus to the new last one.
    let youtube = grid_id_of(&rig, YOUTUBE);
    focus_on(&mut rig, youtube);
    assert_eq!(rig.app.grid_position, Some(6));
    rig.app.catalog.apply(&[], &[YOUTUBE.to_string()]);
    rig.app.catalog_changed();
    rig.advance(0);
    assert_eq!(rig.app.grid_position, Some(5));
    assert_eq!(rig.focused(), Some(grid_id_of(&rig, SPOTIFY)));
}

#[test]
fn a_menu_whose_app_is_uninstalled_closes() {
    let mut rig = Rig::new();
    open_grid_menu(&mut rig, "com.demo.chrome");
    rig.app.catalog.apply(&[], &["com.demo.chrome".to_string()]);
    rig.app.catalog_changed();
    assert!(rig.app.context_menu().is_none());
}

#[test]
fn the_dock_menu_has_move_remove_and_app_info() {
    let mut rig = Rig::new();
    open_dock_menu(&mut rig, 0);
    assert_eq!(
        menu_actions(&rig),
        [
            MenuAction::Move,
            MenuAction::RemoveFromHome,
            MenuAction::AppInfo
        ]
    );
}

#[test]
fn only_one_menu_item_is_ever_lit() {
    // The last item of one menu stayed lit in the next one.
    let mut rig = Rig::new();
    let lit = |rig: &Rig| {
        (0..4)
            .filter(|i| rig.app.focus_progress(ids::MENU_BASE + i) > 0.0)
            .count()
    };
    open_grid_menu(&mut rig, "com.demo.chrome");
    rig.press_all(&[Key::Down, Key::Down]);
    rig.advance(300);
    rig.press(Key::Back);
    rig.advance(300);
    rig.press(Key::Menu);
    rig.advance(300);
    assert_eq!(lit(&rig), 1);
    assert_eq!(selected_action(&rig), MenuAction::AddToHome);
    rig.press(Key::Back);
    rig.press(Key::Home);
    rig.advance(300);
    open_dock_menu(&mut rig, 1);
    rig.press_all(&[Key::Down, Key::Down]);
    rig.press(Key::Back);
    rig.advance(300);
    rig.press(Key::Menu);
    rig.advance(300);
    assert_eq!(lit(&rig), 1);
}

/// Opens the menu of the favorite at `index` and starts moving it.
fn start_moving(rig: &mut Rig, index: usize) {
    open_dock_menu(rig, index);
    assert_eq!(selected_action(rig), MenuAction::Move);
    assert!(rig.press(Key::Ok).is_empty(), "nothing is written yet");
    assert!(rig.app.context_menu().is_none());
}

#[test]
fn move_carries_the_favorite_with_the_arrows_and_ok_keeps_the_place() {
    let mut rig = Rig::new();
    let before = rig.app.config.favorites.clone();
    start_moving(&mut rig, 1);
    assert_eq!(rig.app.moving_favorite(), Some(before[1].as_str()));
    assert!(rig.press(Key::Right).is_empty());
    assert!(rig.press(Key::Right).is_empty());
    assert_eq!(rig.app.config.favorites[3], before[1]);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 3),
        "the focus goes with it"
    );
    rig.press_all(&[Key::Up, Key::Down]);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 3),
        "up and down do nothing"
    );
    assert_eq!(rig.press(Key::Ok), [Effect::SaveConfig]);
    assert_eq!(rig.app.moving_favorite(), None);
    let mut expected = before.clone();
    let moved = expected.remove(1);
    expected.insert(3, moved);
    assert_eq!(rig.app.config.favorites, expected);
}

#[test]
fn move_stops_at_the_ends_of_the_dock() {
    let mut rig = Rig::new();
    let before = rig.app.config.favorites.clone();
    start_moving(&mut rig, 0);
    rig.press(Key::Left);
    assert_eq!(rig.app.config.favorites, before);
    rig.press_all(&[Key::Right; 8]);
    assert_eq!(rig.app.config.favorites.last(), Some(&before[0]));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 4));
}

#[test]
fn back_puts_the_moved_favorite_back_where_it_was() {
    let mut rig = Rig::new();
    let before = rig.app.config.favorites.clone();
    start_moving(&mut rig, 2);
    rig.press_all(&[Key::Left, Key::Left]);
    assert_ne!(rig.app.config.favorites, before);
    rig.press(Key::Back);
    assert_eq!(rig.app.config.favorites, before);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 2));
    assert_eq!(rig.app.moving_favorite(), None);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    // OK without moving writes nothing.
    start_moving(&mut rig, 2);
    assert!(rig.press(Key::Ok).is_empty());
}

#[test]
fn home_while_moving_puts_the_favorite_back_and_goes_home() {
    let mut rig = Rig::new();
    let before = rig.app.config.favorites.clone();
    start_moving(&mut rig, 2);
    rig.press(Key::Right);
    rig.press(Key::Home);
    assert_eq!(rig.app.config.favorites, before);
    assert_eq!(rig.app.moving_favorite(), None);
}

#[test]
fn a_single_favorite_has_no_move() {
    let mut rig = Rig::new();
    rig.app.config.favorites = vec![NETFLIX.to_string()];
    rig.advance(0);
    open_dock_menu(&mut rig, 0);
    assert!(!menu_actions(&rig).contains(&MenuAction::Move));
}

#[test]
fn moved_icons_slide_to_their_new_places() {
    let mut rig = Rig::new();
    start_moving(&mut rig, 1);
    rig.advance(400);
    let x_of = |rig: &Rig, package: &str| {
        let icon = rig
            .app
            .dock_drawn
            .iter()
            .find(|d| d.package.as_deref() == Some(package));
        icon.unwrap().rect.center().0
    };
    let moved = rig.app.config.favorites[1].clone();
    let start = x_of(&rig, &moved);
    rig.app.handle_key(Key::Right, rig.now);
    // The first frame draws it where it was; the slide starts from there.
    rig.advance(16);
    assert_eq!(x_of(&rig, &moved), start);
    rig.advance(60);
    let early = x_of(&rig, &moved);
    rig.advance(400);
    let end = x_of(&rig, &moved);
    assert!(
        early > start && early < end,
        "on its way: {start} {early} {end}"
    );
}

#[test]
fn a_removed_favorite_fades_where_it_was_and_the_others_slide_in() {
    let mut rig = Rig::new();
    rig.advance(400);
    let third = rig.app.config.favorites[2].clone();
    let x_before = rig.app.dock_drawn[2].rect.center().0;
    open_dock_menu(&mut rig, 1);
    rig.press_all(&[Key::Down]);
    assert_eq!(selected_action(&rig), MenuAction::RemoveFromHome);
    rig.app.handle_key(Key::Ok, rig.now);
    rig.advance(16);
    rig.advance(60);
    assert!(
        rig.app.dock_ghost.is_some(),
        "the removed icon is still fading"
    );
    let x_early = rig.app.dock_drawn[1].rect.center().0;
    assert_eq!(
        rig.app.dock_drawn[1].package.as_deref(),
        Some(third.as_str())
    );
    rig.advance(400);
    let x_end = rig.app.dock_drawn[1].rect.center().0;
    assert!(
        x_early < x_before && x_early > x_end,
        "{x_before} {x_early} {x_end}"
    );
    rig.advance(0);
    assert!(rig.app.dock_ghost.is_none());
}

#[test]
fn removing_the_only_favorite_leaves_add_favorites_with_the_focus() {
    let mut rig = Rig::new();
    rig.app.config.favorites = vec![NETFLIX.to_string()];
    rig.advance(0);
    open_dock_menu(&mut rig, 0);
    assert_eq!(
        menu_actions(&rig),
        [MenuAction::RemoveFromHome, MenuAction::AppInfo]
    );
    rig.press(Key::Ok);
    assert!(rig.app.config.favorites.is_empty());
    assert_eq!(rig.focused(), Some(ids::FAVORITES_ADD));
}

#[test]
fn the_editor_shows_what_the_menu_changed() {
    let mut rig = Rig::new();
    open_grid_menu(&mut rig, "com.demo.chrome");
    rig.press(Key::Ok);
    rig.press(Key::Down);
    focus_on(&mut rig, ids::GRID_SETTINGS);
    rig.press_all(&[Key::Ok, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::Editor);
    assert!(!rig
        .app
        .addable_apps()
        .iter()
        .any(|(p, _)| p == "com.demo.chrome"));
    rig.press(Key::Menu);
    assert!(rig.app.context_menu().is_none(), "no menu in the editor");
}

#[test]
fn removing_a_favorite_from_the_dock_moves_the_focus_to_its_place() {
    let mut rig = Rig::new();
    let before = rig.app.config.favorites.clone();
    open_dock_menu(&mut rig, 1);
    rig.press(Key::Down);
    assert_eq!(selected_action(&rig), MenuAction::RemoveFromHome);
    assert_eq!(rig.press(Key::Ok), [Effect::SaveConfig]);
    assert_eq!(rig.app.config.favorites.len(), before.len() - 1);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 1),
        "the next one took its place"
    );
    // The last one gives the focus to the one before it.
    open_dock_menu(&mut rig, 3);
    rig.press(Key::Down);
    rig.press(Key::Ok);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 2));
}

#[test]
fn twelve_favorites_with_room_for_the_arrows_fit_the_safe_area() {
    let safe = crate::display::Display::new(1280, 720).safe_area();
    for focus in 0..12 {
        let l = dock::layout_with_room(12, focus as f32, 1.0, 640.0, safe.bottom(), 1.0);
        assert!(
            l.bar.x >= safe.x && l.bar.right() <= safe.right(),
            "focus {focus}: {:?}",
            l.bar
        );
    }
}

// The splash.

use crate::render::{Cmd, RectCmd, RectKind};
use crate::theme::{vars, Paint};

/// lume goes to the background and comes back; the first frame back is drawn.
fn come_back(rig: &mut Rig) -> DisplayList {
    rig.app.went_to_background();
    rig.app.icons.clear();
    rig.app.came_to_foreground();
    rig.frame()
}

/// The splash logo's square in a frame, if it is drawn.
fn splash_logo(list: &DisplayList) -> Option<RectCmd> {
    list.cmds().iter().find_map(|cmd| match cmd {
        Cmd::Rect(r) if r.rect.w == vars::SPLASH_LOGO && r.rect.h == vars::SPLASH_LOGO => Some(*r),
        _ => None,
    })
}

/// The alpha the logo's square is painted with (0 to 255).
fn logo_alpha(list: &DisplayList) -> Option<u8> {
    match splash_logo(list)?.kind {
        RectKind::Fill(Paint::Solid(color)) => Some(color.a),
        _ => None,
    }
}

/// The alpha the wallpaper is drawn with (0 to 255), if it is drawn.
fn wallpaper_alpha(list: &DisplayList) -> Option<u8> {
    list.cmds().iter().find_map(|cmd| match cmd {
        Cmd::Wallpaper(quad) => Some(quad.color.a),
        _ => None,
    })
}

/// The top edge of the dock's background.
fn dock_top(list: &DisplayList) -> f32 {
    list.cmds()
        .iter()
        .find_map(|cmd| match cmd {
            Cmd::Rect(r) if r.radius == 28.0 && matches!(r.kind, RectKind::Fill(_)) => {
                Some(r.rect.y)
            }
            _ => None,
        })
        .expect("the dock is drawn")
}

fn load_dock_icons(rig: &mut Rig) {
    for package in rig.app.config.favorites.clone() {
        rig.app.icons.reserve_dock(&package).unwrap();
    }
}

/// Gives every app of the grid its tile image.
fn load_grid_tiles(rig: &mut Rig) {
    for i in 0..rig.app.catalog.apps().len() {
        let package = rig.app.catalog.apps()[i].package.clone();
        rig.app
            .icons
            .reserve(&package, crate::app_icons::Art::Icon)
            .unwrap();
    }
}

#[test]
fn the_splash_is_only_the_logo_centered_on_the_background() {
    let mut rig = Rig::new();
    let first = come_back(&mut rig);
    assert!(rig.app.in_splash());
    // The logo fades in on the background the system's window already shows.
    assert_eq!(logo_alpha(&first), Some(0));
    rig.now += vars::SPLASH_LOGO_IN.duration_ms as u64 / 2;
    let half = logo_alpha(&rig.frame()).unwrap();
    assert!(half > 0 && half < 255, "{half}");
    rig.now += vars::SPLASH_LOGO_IN.duration_ms as u64;
    let list = rig.frame();
    // The four shapes of the mark and nothing else: no wallpaper, no screen, nothing under the
    // logo in the band kept for a loading indicator.
    assert_eq!(list.len(), 4, "{:?}", list.cmds());
    let logo = splash_logo(&list).expect("the logo");
    assert_eq!(logo.rect.center(), (640.0, 360.0));
    assert_eq!(logo_alpha(&list), Some(255));
    assert!(matches!(logo.kind, RectKind::Fill(Paint::Solid(c)) if c == vars::PRIMARY));
}

#[test]
fn the_splash_holds_while_a_dock_icon_is_missing_and_lets_go_when_it_loads() {
    let mut rig = Rig::new();
    let favorites = rig.app.config.favorites.clone();
    assert!(favorites.len() >= 2);
    come_back(&mut rig);
    assert_eq!(rig.app.splash_progress(), 0.0);
    load_grid_tiles(&mut rig);
    for package in &favorites[1..] {
        rig.app.icons.reserve_dock(package).unwrap();
    }
    rig.advance(50);
    assert!(rig.app.in_splash(), "one icon is missing");
    let partial = rig.app.splash_progress();
    assert!(partial > 0.0 && partial < 1.0, "{partial}");
    rig.app.icons.reserve_dock(&favorites[0]).unwrap();
    rig.advance(10);
    assert!(
        !rig.app.in_splash(),
        "all icons are there: the reveal starts"
    );
    assert_eq!(rig.app.splash_progress(), 1.0);
}

#[test]
fn an_icon_that_failed_counts_as_ready() {
    let mut rig = Rig::new();
    let favorites = rig.app.config.favorites.clone();
    come_back(&mut rig);
    load_grid_tiles(&mut rig);
    for package in &favorites[1..] {
        rig.app.icons.reserve_dock(package).unwrap();
    }
    rig.app.icon_failed(&favorites[0], true);
    rig.advance(10);
    assert!(!rig.app.in_splash());
}

#[test]
fn the_splash_lets_go_after_600_ms_without_asking_for_frames_meanwhile() {
    let mut rig = Rig::new();
    come_back(&mut rig);
    let since = rig.now;
    // Once the logo is in nothing moves while it waits: the next frame is the 600 ms deadline (the
    // clock's minute is further away), so the loop sleeps and keeps loading icons.
    rig.advance(vars::SPLASH_LOGO_IN.duration_ms as u64);
    assert!(!rig.app.is_animating(rig.now));
    assert_eq!(rig.app.next_deadline(rig.now), Some(since + 600));
    rig.advance(599 - vars::SPLASH_LOGO_IN.duration_ms as u64);
    assert!(rig.app.in_splash());
    rig.advance(1);
    assert!(!rig.app.in_splash(), "the wait has a limit");
}

#[test]
fn the_splash_waits_for_the_visible_tiles_of_the_grid_it_returns_to() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    let focus = rig.focused();
    rig.press(Key::Ok);
    come_back(&mut rig);
    rig.press(Key::Home);
    assert!(
        rig.app.in_splash(),
        "the Home press that brings lume back keeps the splash"
    );
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert_eq!(
        rig.focused(),
        focus,
        "the focus is restored behind the splash"
    );
    let missing = rig.app.splash_missing_tiles().to_vec();
    assert_eq!(
        missing.len(),
        rig.app.catalog.apps().len(),
        "all tiles fit the view"
    );
    for package in &missing {
        rig.app
            .icons
            .reserve(package, crate::app_icons::Art::Icon)
            .unwrap();
    }
    rig.advance(10);
    assert!(
        rig.app.in_splash(),
        "the dock icons, for a move to Home, are missing"
    );
    load_dock_icons(&mut rig);
    rig.advance(10);
    assert!(!rig.app.in_splash());
}

#[test]
fn the_home_splash_also_waits_for_the_first_screen_of_the_grid() {
    let mut rig = Rig::new();
    come_back(&mut rig);
    load_dock_icons(&mut rig);
    rig.advance(10);
    // A move to the Apps tab right after the reveal must not show tiles arriving.
    assert!(rig.app.in_splash());
    assert_eq!(
        rig.app.splash_missing_tiles().len(),
        rig.app.catalog.apps().len()
    );
    load_grid_tiles(&mut rig);
    rig.advance(10);
    assert!(!rig.app.in_splash());
}

#[test]
fn a_key_during_the_splash_ends_it_and_is_applied() {
    let mut rig = Rig::new();
    let first = rig.focused();
    come_back(&mut rig);
    rig.press(Key::Right);
    assert!(!rig.app.in_splash());
    assert!(
        rig.app.take_interface_ready(),
        "the interface is ready at once"
    );
    assert_ne!(rig.focused(), first, "the focus moved to the right");
    let list = rig.frame();
    assert_eq!(logo_alpha(&list), None, "no reveal: the logo is gone");
    assert_eq!(wallpaper_alpha(&list), Some(255));
}

#[test]
fn home_during_the_splash_follows_the_usual_rules() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    come_back(&mut rig);
    // Not back from an app opened by lume: Home goes to the start, behind the splash.
    rig.press(Key::Home);
    assert!(rig.app.in_splash());
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
}

#[test]
fn the_reveal_brings_up_the_wallpaper_and_the_content_and_fades_the_logo() {
    let mut rig = Rig::new();
    let rest = dock_top(&rig.frame());
    for (cold, duration) in [
        (true, vars::SPLASH_REVEAL_COLD.duration_ms),
        (false, vars::SPLASH_REVEAL_WARM.duration_ms),
    ] {
        come_back(&mut rig);
        rig.advance(vars::SPLASH_LOGO_IN.duration_ms as u64);
        load_dock_icons(&mut rig);
        load_grid_tiles(&mut rig);
        let start = rig.frame();
        assert!(!rig.app.in_splash(), "cold: {cold}");
        assert_eq!(logo_alpha(&start), Some(255));
        assert_eq!(
            wallpaper_alpha(&start),
            None,
            "intensity 0: only the background"
        );

        rig.now += (duration / 2.0) as u64;
        let middle = rig.frame();
        let t = crate::ui::animation::ease(crate::theme::Curve::EaseOut, 0.5);
        let alpha = |v: f32| (255.0 * v).round() as i32;
        assert!((i32::from(wallpaper_alpha(&middle).unwrap()) - alpha(t)).abs() <= 1);
        assert!((i32::from(logo_alpha(&middle).unwrap()) - alpha(1.0 - t)).abs() <= 1);
        assert!((dock_top(&middle) - rest - 24.0 * (1.0 - t)).abs() < 0.01);
        assert!(!rig.app.take_interface_ready());

        rig.now += duration as u64;
        let end = rig.frame();
        assert_eq!(logo_alpha(&end), None);
        assert_eq!(wallpaper_alpha(&end), Some(255));
        assert_eq!(dock_top(&end), rest);
        assert!(rig.app.take_interface_ready(), "the end of the reveal");
        assert!(!rig.app.is_animating(rig.now));
    }
}

#[test]
fn the_logo_fades_only_where_it_is() {
    let mut rig = Rig::new();
    come_back(&mut rig);
    load_dock_icons(&mut rig);
    rig.frame();
    rig.now += 100;
    let list = rig.frame();
    // Only the logo's four shapes are translucent paint over the whole frame: no full-screen layer.
    let full_screen = list
        .cmds()
        .iter()
        .any(|cmd| matches!(cmd, Cmd::Rect(r) if r.rect.w >= 1280.0 && r.rect.h >= 720.0));
    assert!(!full_screen);
}

/// The alpha of the dock icon images in a frame (dock cells are the square images).
fn dock_image_alphas(list: &DisplayList) -> Vec<u8> {
    list.cmds()
        .iter()
        .filter_map(|cmd| match cmd {
            Cmd::Image(t) if (t.dest.w - t.dest.h).abs() < 0.01 => Some(t.color.a),
            _ => None,
        })
        .collect()
}

#[test]
fn an_icon_that_arrives_with_the_interface_on_screen_fades_in() {
    let mut rig = Rig::new();
    rig.advance(1000);
    let netflix = rig.app.config.favorites[0].clone();
    rig.app.icons.reserve_dock(&netflix).unwrap();
    rig.app.icon_arrived(&netflix, true, rig.now);
    let start = rig.frame();
    assert_eq!(dock_image_alphas(&start), [0]);
    // A faint fade: icons go on loading under it.
    assert!(rig.app.is_animating(rig.now));
    assert!(!rig.app.is_moving(rig.now));

    rig.now += vars::ICON_IN.duration_ms as u64 / 2;
    let half = dock_image_alphas(&rig.frame())[0];
    assert!(half > 0 && half < 255, "{half}");

    rig.now += vars::ICON_IN.duration_ms as u64;
    assert_eq!(dock_image_alphas(&rig.frame()), [255]);
}

#[test]
fn icons_that_arrive_during_the_splash_do_not_fade_in() {
    let mut rig = Rig::new();
    come_back(&mut rig);
    rig.advance(vars::SPLASH_LOGO_IN.duration_ms as u64);
    for package in rig.app.config.favorites.clone() {
        rig.app.icons.reserve_dock(&package).unwrap();
        rig.app.icon_arrived(&package, true, rig.now);
    }
    load_grid_tiles(&mut rig);
    rig.frame();
    // The reveal brings them in with the rest; once it is over they are all there.
    rig.now += 1000;
    let list = rig.frame();
    let alphas = dock_image_alphas(&list);
    assert!(!alphas.is_empty());
    assert!(alphas.iter().all(|a| *a == 255), "{alphas:?}");
}
