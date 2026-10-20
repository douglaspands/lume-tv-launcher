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
fn the_app_starts_on_the_carousel_button() {
    let rig = Rig::new();
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
}

#[test]
fn down_goes_from_the_carousel_to_the_favorites_and_right_moves_along_them() {
    let mut rig = Rig::new();
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
    rig.press_all(&[Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 2));
    rig.press(Key::Up);
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
}

#[test]
fn moving_the_focus_animates_the_old_and_new_elements() {
    let mut rig = Rig::new();
    rig.advance(1000);
    assert_eq!(
        rig.app.focus_progress(ids::HERO_BUTTON),
        1.0,
        "the first element is fully focused"
    );
    rig.app.handle_key(Key::Down, rig.now);
    rig.advance(10);
    rig.advance(60);
    let (old, new) = (
        rig.app.focus_progress(ids::HERO_BUTTON),
        rig.app.focus_progress(ids::FAVORITE_BASE),
    );
    assert!(
        old < 1.0 && old > 0.0 && new > 0.0 && new < 1.0,
        "mid-animation: {old} {new}"
    );
    rig.advance(1000);
    assert_eq!(rig.app.focus_progress(ids::HERO_BUTTON), 0.0);
    assert_eq!(rig.app.focus_progress(ids::FAVORITE_BASE), 1.0);
    assert!(
        rig.app.next_deadline(rig.now).is_some(),
        "the clock still ticks"
    );
}

fn open_apps(rig: &mut Rig) {
    // From the carousel: up to the top bar, right to "Apps", OK.
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
    rig.press_all(&[Key::Down, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 1));
    let effects = rig.press(Key::Back);
    assert!(effects.is_empty(), "lume stays open");
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(
        rig.focused(),
        Some(ids::HERO_BUTTON),
        "the focus returns to the carousel"
    );
    assert_eq!(rig.app.scroll_y, 0.0);
    // A second Back changes nothing.
    assert!(rig.press(Key::Back).is_empty());
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
}

#[test]
fn back_in_the_apps_tab_returns_to_home() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
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
        Some(ids::HERO_BUTTON),
        "the first element of the content"
    );
    // Pressing Home again on the home tab keeps everything where it is.
    rig.press_all(&[Key::Down, Key::Right]);
    rig.press(Key::Home);
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
}

#[test]
fn the_apps_grid_is_alphabetical_and_ends_with_the_settings_tile() {
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
    let cells = rig
        .app
        .focus
        .rows()
        .iter()
        .skip(1)
        .map(|r| r.items.len())
        .collect::<Vec<_>>();
    assert_eq!(
        cells,
        [5, 3],
        "5 columns, and the settings tile makes the 8th item"
    );
    assert_eq!(
        rig.app
            .focus
            .rows()
            .last()
            .unwrap()
            .items
            .last()
            .unwrap()
            .id,
        ids::GRID_SETTINGS
    );
}

#[test]
fn the_grid_does_not_wrap_at_the_end_of_a_row() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press_all(&[Key::Right; 4]);
    assert_eq!(rig.focused(), Some(ids::GRID_BASE + 4));
    rig.press(Key::Right);
    assert_eq!(
        rig.focused(),
        Some(ids::GRID_BASE + 4),
        "stays on the 5th column"
    );
}

#[test]
fn down_into_a_short_last_row_lands_on_its_last_item() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press_all(&[Key::Right; 3]);
    assert_eq!(rig.focused(), Some(ids::GRID_BASE + 3));
    rig.press(Key::Down);
    // The last row has Spotify... YouTube and "Settings": the nearest to column 4 is the last one.
    assert_eq!(rig.focused(), Some(ids::GRID_SETTINGS));
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
fn ok_on_the_settings_tile_opens_the_android_settings() {
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Down);
    rig.press_all(&[Key::Down, Key::Right, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::GRID_SETTINGS));
    assert_eq!(rig.press(Key::Ok), [Effect::OpenAndroidSettings]);
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
    // Focus the favorites so the carousel can rotate; with one slide per app it does.
    rig.press(Key::Down);
    rig.advance(5000);
    let deadline = rig.app.next_deadline(rig.now).unwrap();
    assert!(deadline <= rig.now + 60_000);
}

fn slide_titles(rig: &Rig) -> Vec<String> {
    rig.app.slides.iter().map(|s| s.title.clone()).collect()
}

#[test]
fn the_carousel_shows_one_slide_per_selected_installed_app() {
    let rig = Rig::new();
    assert_eq!(
        slide_titles(&rig),
        ["Netflix", "YouTube", "Prime Video", "Spotify", "Disney+"]
    );
    assert_eq!(rig.app.carousel.index, 0);
}

#[test]
fn the_carousel_rotates_at_the_configured_interval() {
    let mut rig = Rig::new();
    rig.press(Key::Down); // the focus leaves the carousel
    let start = rig.now;
    assert_eq!(rig.app.carousel.index, 0);
    rig.advance(4000);
    assert_eq!(
        rig.app.carousel.index,
        0,
        "not yet: {} ms since the change",
        rig.now - start
    );
    rig.advance(1500);
    assert_eq!(rig.app.carousel.index, 1, "after 5 seconds");
    rig.advance(5100);
    assert_eq!(rig.app.carousel.index, 2);
    // After the last slide it goes back to the first.
    rig.advance(5100);
    rig.advance(5100);
    assert_eq!(rig.app.carousel.index, 4);
    rig.advance(5100);
    assert_eq!(rig.app.carousel.index, 0);
}

#[test]
fn the_rotation_follows_a_changed_interval() {
    let mut rig = Rig::new();
    rig.app.config.interval = crate::home::config::Interval::Seconds10;
    rig.press(Key::Down);
    rig.advance(6000);
    assert_eq!(rig.app.carousel.index, 0);
    rig.advance(5000);
    assert_eq!(rig.app.carousel.index, 1);
}

#[test]
fn the_carousel_pauses_while_it_has_the_focus() {
    let mut rig = Rig::new();
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
    rig.advance(30_000);
    assert_eq!(
        rig.app.carousel.index, 0,
        "the slide does not change on its own"
    );
    assert_eq!(rig.app.carousel_deadline(), None);
    // Leaving the carousel restarts the countdown from that moment.
    rig.press(Key::Down);
    rig.advance(4000);
    assert_eq!(rig.app.carousel.index, 0);
    rig.advance(1500);
    assert_eq!(rig.app.carousel.index, 1);
}

#[test]
fn left_and_right_change_the_slide_by_hand() {
    let mut rig = Rig::new();
    rig.press(Key::Right);
    assert_eq!(rig.app.carousel.index, 1);
    assert_eq!(
        rig.focused(),
        Some(ids::HERO_BUTTON),
        "the focus stays on the button"
    );
    rig.press_all(&[Key::Right, Key::Right]);
    assert_eq!(rig.app.carousel.index, 3);
    rig.press_all(&[Key::Left, Key::Left, Key::Left, Key::Left]);
    assert_eq!(
        rig.app.carousel.index, 4,
        "left from the first slide goes to the last"
    );
}

#[test]
fn a_slide_change_cross_fades() {
    let mut rig = Rig::new();
    rig.press(Key::Right);
    rig.advance(100);
    let fade = rig.app.anim.get(Anim::SlideFade, 1.0, rig.now);
    assert!(fade > 0.0 && fade < 1.0, "fading: {fade}");
    rig.advance(1000);
    assert_eq!(rig.app.anim.get(Anim::SlideFade, 1.0, rig.now), 1.0);
}

#[test]
fn ok_on_the_carousel_button_opens_the_slide_app() {
    let mut rig = Rig::new();
    rig.press_all(&[Key::Right, Key::Right]);
    let effects = rig.press(Key::Ok);
    assert_eq!(
        effects[0],
        Effect::OpenApp {
            package: PRIME.into(),
            activity: format!("{PRIME}.Main")
        }
    );
}

#[test]
fn without_any_selected_app_the_welcome_slide_leads_to_the_apps_tab() {
    let mut rig = Rig::with_catalog(catalog(&[("com.demo.chrome", "Chrome")]));
    assert_eq!(slide_titles(&rig), ["Bem-vindo ao lume"]);
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
    let effects = rig.press(Key::Ok);
    assert!(effects.is_empty());
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
}

#[test]
fn a_single_slide_never_rotates() {
    let mut rig = Rig::with_catalog(catalog(&[(NETFLIX, "Netflix")]));
    rig.press(Key::Down);
    assert_eq!(rig.app.carousel_deadline(), None);
    rig.advance(60_000);
    assert_eq!(rig.app.carousel.index, 0);
    rig.press_all(&[Key::Up, Key::Right]);
    assert_eq!(rig.app.carousel.index, 0);
}

#[test]
fn the_carousel_deadline_is_reported_to_the_event_loop() {
    let mut rig = Rig::new();
    rig.press(Key::Down);
    let deadline = rig.app.carousel_deadline().unwrap();
    assert_eq!(deadline, rig.app.carousel.since_ms + 5000);
    assert!(rig.app.next_deadline(rig.now).unwrap() <= deadline);
}

#[test]
fn opening_from_the_carousel_updates_the_context_line() {
    let mut rig = Rig::new();
    rig.press(Key::Ok);
    // Another app is used more, so Netflix is not "most used".
    rig.app.usage.record_open(YOUTUBE, NOW);
    rig.app.usage.record_open(YOUTUBE, NOW);
    rig.advance(7200 * 1000);
    rig.app.clock.unix_secs += 7200;
    rig.press(Key::Down);
    rig.press(Key::Home);
    assert_eq!(rig.app.slides[0].context, "Aberto há 2 horas");
    assert_eq!(rig.app.slides[1].context, "Mais usado da semana");
}

#[test]
fn the_favorites_row_scrolls_sideways_to_keep_the_focus_visible() {
    let catalog = many_apps(12);
    let mut rig = Rig::with_catalog(catalog.clone());
    rig.app.config.favorites = catalog.apps().iter().map(|a| a.package.clone()).collect();
    rig.advance(0);
    rig.press(Key::Down);
    assert_eq!(rig.app.favorites_scroll_x, 0.0);
    // Five tiles fit; moving to the sixth scrolls the row.
    rig.press_all(&[Key::Right; 5]);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE + 5));
    let scrolled = rig.app.favorites_scroll_x;
    assert!(scrolled > 0.0, "scrolled by {scrolled}");
    rig.advance(1000);
    // The focused tile is fully inside the 64..1216 band now.
    let tile = rig.app.focus.current().unwrap().rect;
    assert!(
        tile.right() - scrolled <= 1216.5 && tile.x - scrolled >= 63.5,
        "{tile:?} scrolled by {scrolled}"
    );
    // Moving back to the first tile scrolls back.
    rig.press_all(&[Key::Left; 5]);
    assert_eq!(rig.app.favorites_scroll_x, 0.0);
}

#[test]
fn the_favorites_row_ends_at_the_last_favorite() {
    let catalog = many_apps(12);
    let mut rig = Rig::with_catalog(catalog.clone());
    rig.app.config.favorites = catalog.apps().iter().map(|a| a.package.clone()).collect();
    rig.advance(0);
    rig.press(Key::Down);
    rig.press_all(&[Key::Right; 20]);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE + 11),
        "no wrapping at the end"
    );
    let max = 12.0 * (217.6 + 16.0) - 16.0 - 1152.0;
    assert!(
        (rig.app.favorites_scroll_x - max).abs() < 0.5,
        "{} vs {max}",
        rig.app.favorites_scroll_x
    );
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
fn a_hidden_section_is_not_shown_or_focusable() {
    let mut rig = Rig::new();
    rig.app
        .config
        .set_section_visible(crate::home::config::Section::Carousel, false);
    rig.advance(100);
    // The content starts with the favorites row.
    rig.press(Key::Home);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
    rig.app
        .config
        .set_section_visible(crate::home::config::Section::Favorites, false);
    rig.app
        .config
        .set_section_visible(crate::home::config::Section::Carousel, true);
    rig.press(Key::Home);
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
    assert!(rig
        .app
        .focus
        .rows()
        .iter()
        .all(|r| r.items.iter().all(|i| i.id < ids::FAVORITE_BASE)));
}

#[test]
fn the_favorites_come_first_when_the_sections_are_swapped() {
    let mut rig = Rig::new();
    rig.app
        .config
        .move_section_down(crate::home::config::Section::Carousel);
    rig.press(Key::Home);
    assert_eq!(rig.focused(), Some(ids::FAVORITE_BASE));
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
}

#[test]
fn removing_an_installed_app_updates_the_screens() {
    let mut rig = Rig::new();
    rig.app.catalog.apply(&[], &[YOUTUBE.to_string()]);
    rig.app.config.remove_uninstalled(&rig.app.catalog);
    rig.advance(100);
    assert_eq!(rig.app.config.favorites, [NETFLIX, PRIME, SPOTIFY, DISNEY]);
    assert!(!slide_titles(&rig).contains(&"YouTube".to_string()));
}

// ---- settings and the home screen editor ---------------------------------------------------

use super::editor::{
    section_button, SectionAction, ADD_APP_BASE, CAROUSEL_APP_BASE, CONFIRM_CANCEL,
    CONFIRM_RESTORE, DONE, FAVORITE_BUTTON_BASE, INTERVAL_BASE, RESTORE,
};
use crate::home::config::{Interval, Section};

fn open_settings(rig: &mut Rig) {
    // Up to the top bar, then right to the gear.
    rig.press_all(&[Key::Up, Key::Right, Key::Right]);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    rig.press(Key::Ok);
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
fn the_gear_opens_the_settings_with_the_focus_on_the_first_item() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE + 1));
    rig.press(Key::Down);
    assert_eq!(rig.focused(), Some(ids::SETTINGS_BASE + 2));
    assert!(
        !rig.app.focus.move_focus(Direction::Down),
        "three items only"
    );
}

#[test]
fn back_from_the_settings_returns_to_the_tab_it_came_from() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::GEAR));
    // From the Apps tab.
    let mut rig = Rig::new();
    open_apps(&mut rig);
    rig.press(Key::Right);
    assert_eq!(rig.focused(), Some(ids::GEAR));
    rig.press(Key::Ok);
    rig.press(Key::Back);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
}

#[test]
fn android_settings_open_from_the_settings_screen() {
    let mut rig = Rig::new();
    open_settings(&mut rig);
    rig.press(Key::Down);
    assert_eq!(rig.press(Key::Ok), [Effect::OpenAndroidSettings]);
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
    assert_eq!(rig.focused(), Some(DONE));
    let effects = rig.press(Key::Ok);
    assert_eq!(effects, [Effect::SaveConfig]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Home));
    assert_eq!(rig.focused(), Some(ids::HERO_BUTTON));
}

#[test]
fn back_in_the_editor_works_like_done() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let effects = rig.press(Key::Back);
    assert_eq!(effects, [Effect::SaveConfig]);
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
fn moving_a_section_down_shows_the_favorites_first() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let down = section_button(Section::Carousel, SectionAction::Down);
    assert_eq!(press_on(&mut rig, down), [Effect::SaveConfig]);
    assert_eq!(
        rig.app.config.visible_sections().collect::<Vec<_>>(),
        [Section::Favorites, Section::Carousel]
    );
    // The focus followed the section to its new row; "down" is disabled there, so it is on "up".
    assert_eq!(
        rig.focused(),
        Some(section_button(Section::Carousel, SectionAction::Up))
    );
    rig.press(Key::Back);
    assert_eq!(
        rig.focused(),
        Some(ids::FAVORITE_BASE),
        "the home screen starts with the favorites"
    );
}

#[test]
fn the_edges_have_disabled_buttons() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let first_up = section_button(Section::Carousel, SectionAction::Up);
    let last_down = section_button(Section::Favorites, SectionAction::Down);
    let enabled = |rig: &Rig, id: FocusId| {
        rig.app
            .focus
            .rows()
            .iter()
            .any(|r| r.items.iter().any(|i| i.id == id && i.enabled))
    };
    assert!(!enabled(&rig, first_up), "the first section cannot move up");
    assert!(
        !enabled(&rig, last_down),
        "the last section cannot move down"
    );
    assert!(enabled(
        &rig,
        section_button(Section::Carousel, SectionAction::Down)
    ));
}

#[test]
fn hiding_a_section_removes_it_from_the_home_screen() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let hide = section_button(Section::Favorites, SectionAction::Hide);
    assert_eq!(press_on(&mut rig, hide), [Effect::SaveConfig]);
    assert!(!rig.app.config.is_visible(Section::Favorites));
    rig.press(Key::Back);
    rig.advance(100);
    assert!(rig
        .app
        .focus
        .rows()
        .iter()
        .all(|r| r.items.iter().all(|i| i.id < ids::FAVORITE_BASE)));
}

#[test]
fn a_chip_changes_the_interval() {
    let mut rig = Rig::new();
    open_editor(&mut rig);
    let ten_seconds = INTERVAL_BASE + 2;
    assert_eq!(press_on(&mut rig, ten_seconds), [Effect::SaveConfig]);
    assert_eq!(rig.app.config.interval, Interval::Seconds10);
}

#[test]
fn the_last_carousel_app_cannot_be_removed() {
    let mut rig = Rig::new();
    rig.app.config.carousel_apps = vec![NETFLIX.to_string()];
    open_editor(&mut rig);
    let netflix_chip = CAROUSEL_APP_BASE; // Netflix is the first known app
    let effects = press_on(&mut rig, netflix_chip);
    assert!(effects.is_empty(), "nothing to save");
    assert_eq!(rig.app.config.carousel_apps, [NETFLIX]);
    assert!(
        rig.app.now_ms < rig.app.editor.hint_until_ms,
        "the hint is highlighted"
    );
    // Adding another app and then removing Netflix works.
    assert_eq!(
        press_on(&mut rig, CAROUSEL_APP_BASE + 1),
        [Effect::SaveConfig]
    );
    assert_eq!(press_on(&mut rig, netflix_chip), [Effect::SaveConfig]);
    assert_eq!(rig.app.config.carousel_apps, [YOUTUBE]);
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
    rig.app.config.interval = Interval::Minute1;
    rig.app.config.remove_favorite(0);
    open_editor(&mut rig);
    assert_eq!(press_on(&mut rig, RESTORE), []);
    assert!(rig.app.editor.confirm, "the dialog is open");
    assert_eq!(
        rig.focused(),
        Some(CONFIRM_CANCEL),
        "the safe choice has the focus"
    );
    assert_eq!(
        rig.app.config.interval,
        Interval::Minute1,
        "nothing changed yet"
    );
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
    rig.app.config.interval = Interval::Minute1;
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
    focus_on(&mut rig, RESTORE);
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
    // Back to the top.
    focus_on(&mut rig, DONE);
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
fn a_favorite_shows_the_icon_and_keeps_its_name() {
    let mut rig = Rig::new();
    let before = rig.frame();
    rig.app
        .icons
        .reserve(NETFLIX, crate::app_icons::Art::Icon)
        .unwrap();
    let after = rig.frame();
    assert_eq!(images(&after), 1);
    assert_eq!(glyphs(&after), glyphs(&before), "the name is still drawn");
}

#[test]
fn a_banner_covers_the_tile_and_replaces_the_name_on_the_home_screen() {
    let mut rig = Rig::new();
    let before = rig.frame();
    rig.app
        .icons
        .reserve(NETFLIX, crate::app_icons::Art::Banner)
        .unwrap();
    let after = rig.frame();
    assert_eq!(images(&after), 1);
    assert!(
        glyphs(&after) < glyphs(&before),
        "the banner has the name in it"
    );
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
fn no_screen_but_home_keeps_the_carousel_timer_running() {
    // The carousel rotates on a timer while the focus is off it; leaving the home screen must stop it.
    let mut rig = Rig::new();
    assert!(
        rig.app.carousel_deadline().is_none(),
        "focus is on the hero"
    );
    rig.press(Key::Down);
    assert!(rig.app.carousel_deadline().is_some(), "rotating on Home");
    rig.press_all(&[Key::Up, Key::Up, Key::Right, Key::Ok]);
    assert_eq!(rig.app.screen(), &Screen::Main(Tab::Apps));
    assert!(
        rig.app.carousel_deadline().is_none(),
        "Apps does not rotate it"
    );
    rig.advance(30_000);
    assert!(rig
        .app
        .next_deadline(rig.now)
        .is_none_or(|at| at > rig.now + 1_000));
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
