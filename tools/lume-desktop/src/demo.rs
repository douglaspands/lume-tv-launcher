//! The demonstration data the harness runs with: apps of the design mockup and a fixed clock.

use lume_core::app::{Key, LumeApp};
use lume_core::catalog::color::ICON_SIZE;
use lume_core::catalog::{Catalog, Discovered};
use lume_core::clock::WallClock;
use lume_core::home::config::HomeConfig;
use lume_core::i18n::Translator;
use lume_core::usage::UsageHistory;

/// 2026-10-19 21:45:00 UTC: the time shown in the design mockups.
pub const MOCKUP_TIME: u64 = 1_792_446_300;

/// A solid-color icon, so the catalog derives that color for apps lume does not know.
fn icon(rgb: u32) -> Vec<u32> {
    vec![0xFF00_0000 | rgb; ICON_SIZE * ICON_SIZE]
}

fn app(package: &str, label: &str, rgb: Option<u32>) -> Discovered {
    Discovered {
        package: package.to_string(),
        label: label.to_string(),
        activity: format!("{package}.Main"),
        leanback: true,
        system: false,
        icon: rgb.map(icon),
    }
}

/// The apps of the "Todos os apps" mockup (the known ones get their colors from the theme).
pub fn mockup_apps() -> Vec<Discovered> {
    vec![
        app("com.netflix.ninja", "Netflix", None),
        app("com.google.android.youtube.tv", "YouTube", None),
        app("com.amazon.amazonvideo.livingroom", "Prime Video", None),
        app("com.disney.disneyplus", "Disney+", None),
        app("com.wbd.stream", "Max", None),
        app("com.spotify.tv.android", "Spotify", None),
        app("com.globo.globotv", "Globoplay", None),
        app("com.android.vending", "Play Store", None),
        app("com.android.chrome", "Chrome", None),
        app("com.demo.games", "Jogos", Some(0x6B3FA5)),
        app("com.demo.gallery", "Galeria", Some(0xA3446E)),
        app("com.demo.music", "Música", Some(0xB3601A)),
        app("com.demo.cast", "Cast", Some(0x27757C)),
        app("com.demo.files", "Arquivos", Some(0x3E4A5C)),
    ]
}

pub struct Setup {
    pub locale: Option<String>,
    pub pseudo_locale: bool,
    pub is_24h: bool,
    /// The first N apps of [`mockup_apps`] as favorites, instead of the first-use ones.
    pub favorites: Option<usize>,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            locale: None,
            pseudo_locale: false,
            is_24h: true,
            favorites: None,
        }
    }
}

/// A launcher with the mockup apps, as it looks on a first run.
pub fn app_with(setup: &Setup) -> LumeApp {
    let catalog = Catalog::from_discovered(&mockup_apps(), "app.lume.tv");
    let mut usage = UsageHistory::new();
    usage.mark_baseline(catalog.apps().iter().map(|a| a.package.as_str()));
    let mut config = HomeConfig::first_use(&catalog);
    if let Some(count) = setup.favorites {
        config.favorites = mockup_apps()
            .into_iter()
            .take(count)
            .map(|app| app.package)
            .collect();
    }
    let preferred: Vec<&str> = setup.locale.as_deref().into_iter().collect();
    let mut translator = Translator::new(&preferred);
    if setup.pseudo_locale {
        translator = translator.pseudo();
    }
    let clock = WallClock {
        unix_secs: MOCKUP_TIME,
        utc_offset_secs: 0,
        is_24h: setup.is_24h,
    };
    let mut app = LumeApp::new(catalog, usage, config, translator, clock);
    app.inputs_available = true;
    app
}

/// Feeds the settings panel the numbers of a made-up TV, as the platform would every 2 seconds.
pub fn feed_stats(app: &mut LumeApp, now_ms: u64) {
    use lume_core::stats::{CpuReading, Stats};
    if app.stats_deadline(now_ms).is_some_and(|at| at <= now_ms) {
        app.apply_stats(
            Stats {
                memory_total_kb: 2_000_000,
                memory_available_kb: 820_000,
                disk_total: 12_000_000_000,
                disk_free: 7_300_000_000,
                cpu: CpuReading::Freq {
                    current: 3_600_000,
                    max: 8_000_000,
                },
            },
            now_ms,
        );
    }
}

/// Parses names such as `down`, `right` and `ok`.
pub fn parse_keys(text: &str) -> Result<Vec<Key>, String> {
    text.split(',')
        .filter(|name| !name.trim().is_empty())
        .map(|name| match name.trim().to_ascii_lowercase().as_str() {
            "up" => Ok(Key::Up),
            "down" => Ok(Key::Down),
            "left" => Ok(Key::Left),
            "right" => Ok(Key::Right),
            "ok" | "enter" => Ok(Key::Ok),
            "back" => Ok(Key::Back),
            "home" => Ok(Key::Home),
            "menu" => Ok(Key::Menu),
            other => Err(format!(
                "unknown key '{other}' (use up, down, left, right, ok, back, home, menu)"
            )),
        })
        .collect()
}
