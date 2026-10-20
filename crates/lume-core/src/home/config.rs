//! The user's home screen setup: sections, carousel and favorites (`config.v1`).

use crate::catalog::known::KnownApp;
use crate::catalog::Catalog;
use crate::storage::{self, Kv, StorageError};
use std::path::Path;

const VERSION: u32 = 1;

/// Most favorites the home screen can hold.
pub const MAX_FAVORITES: usize = 12;
/// Favorites chosen automatically on first use.
pub const FIRST_USE_FAVORITES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Carousel,
    Favorites,
}

impl Section {
    fn key(self) -> &'static str {
        match self {
            Section::Carousel => "carousel",
            Section::Favorites => "favorites",
        }
    }

    fn from_key(key: &str) -> Option<Section> {
        [Section::Carousel, Section::Favorites]
            .into_iter()
            .find(|s| s.key() == key)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionSetting {
    pub section: Section,
    pub visible: bool,
}

/// How often the carousel advances to the next slide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interval {
    Seconds3,
    Seconds5,
    Seconds10,
    Seconds30,
    Minute1,
}

impl Interval {
    pub const ALL: [Interval; 5] = [
        Interval::Seconds3,
        Interval::Seconds5,
        Interval::Seconds10,
        Interval::Seconds30,
        Interval::Minute1,
    ];
    pub const DEFAULT: Interval = Interval::Seconds5;

    pub const fn seconds(self) -> u32 {
        match self {
            Interval::Seconds3 => 3,
            Interval::Seconds5 => 5,
            Interval::Seconds10 => 10,
            Interval::Seconds30 => 30,
            Interval::Minute1 => 60,
        }
    }

    pub fn from_seconds(seconds: u32) -> Option<Interval> {
        Interval::ALL.into_iter().find(|i| i.seconds() == seconds)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeConfig {
    /// Both sections, top to bottom.
    pub sections: Vec<SectionSetting>,
    /// Apps with a slide in the carousel, in slide order.
    pub carousel_apps: Vec<String>,
    pub interval: Interval,
    /// Favorite apps, left to right.
    pub favorites: Vec<String>,
}

impl HomeConfig {
    /// The setup of a first run: the well-known apps that are installed, in the preferred order.
    pub fn first_use(catalog: &Catalog) -> HomeConfig {
        let installed: Vec<String> = KnownApp::DEFAULT_FAVORITES
            .iter()
            .filter_map(|known| {
                known
                    .packages()
                    .iter()
                    .find(|package| catalog.contains(package))
            })
            .map(|package| package.to_string())
            .collect();
        HomeConfig {
            sections: vec![
                SectionSetting {
                    section: Section::Carousel,
                    visible: true,
                },
                SectionSetting {
                    section: Section::Favorites,
                    visible: true,
                },
            ],
            carousel_apps: installed.clone(),
            interval: Interval::DEFAULT,
            favorites: installed.into_iter().take(FIRST_USE_FAVORITES).collect(),
        }
    }

    /// The sections that are shown, in order.
    pub fn visible_sections(&self) -> impl Iterator<Item = Section> + '_ {
        self.sections
            .iter()
            .filter(|s| s.visible)
            .map(|s| s.section)
    }

    pub fn is_visible(&self, section: Section) -> bool {
        self.sections
            .iter()
            .any(|s| s.section == section && s.visible)
    }

    pub fn position(&self, section: Section) -> usize {
        self.sections
            .iter()
            .position(|s| s.section == section)
            .expect("both sections are always present")
    }

    // ---- editing ------------------------------------------------------------------------

    /// Moves a section one place up. Returns `false` when it is already first.
    pub fn move_section_up(&mut self, section: Section) -> bool {
        let at = self.position(section);
        if at == 0 {
            return false;
        }
        self.sections.swap(at, at - 1);
        true
    }

    /// Moves a section one place down. Returns `false` when it is already last.
    pub fn move_section_down(&mut self, section: Section) -> bool {
        let at = self.position(section);
        if at + 1 >= self.sections.len() {
            return false;
        }
        self.sections.swap(at, at + 1);
        true
    }

    pub fn can_move_section_up(&self, section: Section) -> bool {
        self.position(section) > 0
    }

    pub fn can_move_section_down(&self, section: Section) -> bool {
        self.position(section) + 1 < self.sections.len()
    }

    pub fn set_section_visible(&mut self, section: Section, visible: bool) {
        let at = self.position(section);
        self.sections[at].visible = visible;
    }

    /// Adds or removes `package` from the carousel. The last selected app cannot be removed;
    /// returns `false` in that case.
    pub fn toggle_carousel_app(&mut self, package: &str) -> bool {
        match self.carousel_apps.iter().position(|p| p == package) {
            Some(_) if self.carousel_apps.len() == 1 => false,
            Some(at) => {
                self.carousel_apps.remove(at);
                true
            }
            None => {
                self.carousel_apps.push(package.to_string());
                true
            }
        }
    }

    pub fn set_interval(&mut self, interval: Interval) {
        self.interval = interval;
    }

    /// Moves a favorite one place to the left. Returns `false` at the first place.
    pub fn move_favorite_left(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.favorites.len() {
            return false;
        }
        self.favorites.swap(index, index - 1);
        true
    }

    /// Moves a favorite one place to the right. Returns `false` at the last place.
    pub fn move_favorite_right(&mut self, index: usize) -> bool {
        if index + 1 >= self.favorites.len() {
            return false;
        }
        self.favorites.swap(index, index + 1);
        true
    }

    pub fn remove_favorite(&mut self, index: usize) -> bool {
        if index >= self.favorites.len() {
            return false;
        }
        self.favorites.remove(index);
        true
    }

    pub fn favorites_full(&self) -> bool {
        self.favorites.len() >= MAX_FAVORITES
    }

    /// Adds an app at the end of the favorites. Fails when the list is full or already has it.
    pub fn add_favorite(&mut self, package: &str) -> bool {
        if self.favorites_full() || self.favorites.iter().any(|p| p == package) {
            return false;
        }
        self.favorites.push(package.to_string());
        true
    }

    /// Takes apps that are no longer installed out of the favorites and the carousel,
    /// keeping the order of the rest. Returns `true` when something changed.
    pub fn remove_uninstalled(&mut self, catalog: &Catalog) -> bool {
        let before = (self.favorites.len(), self.carousel_apps.len());
        self.favorites.retain(|p| catalog.contains(p));
        self.carousel_apps.retain(|p| catalog.contains(p));
        before != (self.favorites.len(), self.carousel_apps.len())
    }

    // ---- storage ------------------------------------------------------------------------

    fn to_kv(&self) -> Kv {
        let mut kv = Kv::new();
        kv.set_list("sections", self.sections.iter().map(|s| s.section.key()));
        for setting in &self.sections {
            kv.set(
                &format!("section.{}.visible", setting.section.key()),
                if setting.visible { "1" } else { "0" },
            );
        }
        kv.set_list(
            "carousel.apps",
            self.carousel_apps.iter().map(String::as_str),
        );
        kv.set("carousel.interval", self.interval.seconds().to_string());
        kv.set_list("favorites", self.favorites.iter().map(String::as_str));
        kv
    }

    /// Reads a file's values, repairing anything out of range, and `None` when essential
    /// parts are missing.
    fn from_kv(kv: &Kv) -> Option<HomeConfig> {
        let mut order: Vec<Section> = Vec::new();
        for key in kv.list("sections")? {
            let section = Section::from_key(key)?;
            if !order.contains(&section) {
                order.push(section);
            }
        }
        // Both sections always exist; one missing from the file goes to the end.
        for section in [Section::Carousel, Section::Favorites] {
            if !order.contains(&section) {
                order.push(section);
            }
        }
        let sections = order
            .into_iter()
            .map(|section| SectionSetting {
                section,
                visible: kv.get(&format!("section.{}.visible", section.key())) != Some("0"),
            })
            .collect();

        let unique = |items: Vec<&str>| {
            let mut out: Vec<String> = Vec::new();
            for item in items {
                if !item.is_empty() && !out.iter().any(|o| o == item) {
                    out.push(item.to_string());
                }
            }
            out
        };
        let carousel_apps = unique(kv.list("carousel.apps")?);
        let mut favorites = unique(kv.list("favorites")?);
        favorites.truncate(MAX_FAVORITES);
        let interval = kv
            .parse("carousel.interval")
            .and_then(Interval::from_seconds)
            .unwrap_or(Interval::DEFAULT);
        Some(HomeConfig {
            sections,
            carousel_apps,
            interval,
            favorites,
        })
    }

    /// Writes the configuration file (`config.v1`).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        storage::save(path, VERSION, &self.to_kv())
    }

    /// Reads the configuration. `None` means there is nothing usable (first run, or a file that
    /// is corrupt or from an unknown version, which is logged) and the caller starts with
    /// [`HomeConfig::first_use`].
    pub fn load(path: &Path) -> Option<HomeConfig> {
        match storage::load(path, VERSION) {
            Ok(kv) => {
                let config = HomeConfig::from_kv(&kv);
                if config.is_none() {
                    log::warn!("ignoring {}: incomplete configuration", path.display());
                }
                config
            }
            Err(StorageError::Missing) => None,
            Err(e) => {
                log::warn!("ignoring {}: {e}", path.display());
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Discovered;

    fn catalog(packages: &[&str]) -> Catalog {
        let entries: Vec<Discovered> = packages
            .iter()
            .map(|p| Discovered {
                package: p.to_string(),
                label: p.to_string(),
                activity: "Main".into(),
                leanback: true,
                icon: None,
            })
            .collect();
        Catalog::from_discovered(&entries, "app.lume.tv")
    }

    const NETFLIX: &str = "com.netflix.ninja";
    const YOUTUBE: &str = "com.google.android.youtube.tv";
    const SPOTIFY: &str = "com.spotify.tv.android";

    fn sample() -> HomeConfig {
        HomeConfig {
            sections: vec![
                SectionSetting {
                    section: Section::Carousel,
                    visible: true,
                },
                SectionSetting {
                    section: Section::Favorites,
                    visible: true,
                },
            ],
            carousel_apps: vec![NETFLIX.into(), YOUTUBE.into()],
            interval: Interval::Seconds10,
            favorites: vec![NETFLIX.into(), YOUTUBE.into(), SPOTIFY.into()],
        }
    }

    #[test]
    fn first_use_picks_the_installed_known_apps_in_order() {
        // Installed in a different order than the preferred one, plus unknown apps.
        let config = HomeConfig::first_use(&catalog(&[SPOTIFY, "com.example.x", NETFLIX, YOUTUBE]));
        assert_eq!(config.favorites, [NETFLIX, YOUTUBE, SPOTIFY]);
        assert_eq!(config.carousel_apps, [NETFLIX, YOUTUBE, SPOTIFY]);
        assert_eq!(config.interval, Interval::Seconds5);
        assert_eq!(
            config.visible_sections().collect::<Vec<_>>(),
            [Section::Carousel, Section::Favorites]
        );
    }

    #[test]
    fn first_use_limits_favorites_to_five_but_not_the_carousel() {
        let all = [
            NETFLIX,
            YOUTUBE,
            "com.amazon.amazonvideo.livingroom",
            SPOTIFY,
            "com.disney.disneyplus",
            "com.wbd.stream",
            "com.globo.globotv",
        ];
        let config = HomeConfig::first_use(&catalog(&all));
        assert_eq!(
            config.favorites,
            [
                NETFLIX,
                YOUTUBE,
                "com.amazon.amazonvideo.livingroom",
                SPOTIFY,
                "com.disney.disneyplus"
            ]
        );
        assert_eq!(config.carousel_apps.len(), 7);
    }

    #[test]
    fn first_use_with_no_known_apps_is_empty() {
        let config = HomeConfig::first_use(&catalog(&["com.example.x"]));
        assert!(config.favorites.is_empty() && config.carousel_apps.is_empty());
        assert_eq!(config.sections.len(), 2);
    }

    #[test]
    fn first_use_uses_whichever_package_of_a_known_app_is_installed() {
        let config = HomeConfig::first_use(&catalog(&["com.netflix.mediaclient"]));
        assert_eq!(config.favorites, ["com.netflix.mediaclient"]);
    }

    #[test]
    fn moving_sections() {
        let mut config = sample();
        assert!(
            !config.can_move_section_up(Section::Carousel)
                && config.can_move_section_down(Section::Carousel)
        );
        assert!(!config.move_section_up(Section::Carousel));
        assert!(config.move_section_down(Section::Carousel));
        assert_eq!(
            config.visible_sections().collect::<Vec<_>>(),
            [Section::Favorites, Section::Carousel]
        );
        assert!(!config.can_move_section_down(Section::Carousel));
        assert!(!config.move_section_down(Section::Carousel));
        assert!(config.move_section_up(Section::Carousel));
        assert_eq!(config.position(Section::Carousel), 0);
    }

    #[test]
    fn hiding_a_section() {
        let mut config = sample();
        config.set_section_visible(Section::Favorites, false);
        assert!(!config.is_visible(Section::Favorites));
        assert_eq!(
            config.visible_sections().collect::<Vec<_>>(),
            [Section::Carousel]
        );
        // A hidden section keeps its place in the editor.
        assert_eq!(config.position(Section::Favorites), 1);
        config.set_section_visible(Section::Favorites, true);
        assert!(config.is_visible(Section::Favorites));
    }

    #[test]
    fn the_carousel_keeps_at_least_one_app() {
        let mut config = HomeConfig {
            carousel_apps: vec![NETFLIX.into()],
            ..sample()
        };
        assert!(!config.toggle_carousel_app(NETFLIX));
        assert_eq!(config.carousel_apps, [NETFLIX]);
        assert!(config.toggle_carousel_app(YOUTUBE));
        assert!(config.toggle_carousel_app(NETFLIX));
        assert_eq!(config.carousel_apps, [YOUTUBE]);
    }

    #[test]
    fn changing_the_interval() {
        let mut config = sample();
        config.set_interval(Interval::Minute1);
        assert_eq!(config.interval.seconds(), 60);
        assert_eq!(Interval::ALL.map(Interval::seconds), [3, 5, 10, 30, 60]);
        assert_eq!(Interval::from_seconds(10), Some(Interval::Seconds10));
        assert_eq!(Interval::from_seconds(7), None);
    }

    #[test]
    fn reordering_favorites() {
        let mut config = HomeConfig {
            favorites: ["a", "b", "c", "spotify", "e"].map(String::from).to_vec(),
            ..sample()
        };
        // Spotify, in position 4 (index 3), moves left.
        assert!(config.move_favorite_left(3));
        assert_eq!(config.favorites, ["a", "b", "spotify", "c", "e"]);
        assert!(!config.move_favorite_left(0));
        assert!(!config.move_favorite_right(4));
        assert!(config.move_favorite_right(0));
        assert_eq!(config.favorites, ["b", "a", "spotify", "c", "e"]);
        assert!(!config.move_favorite_left(9) && !config.move_favorite_right(9));
    }

    #[test]
    fn adding_and_removing_favorites() {
        let mut config = HomeConfig {
            favorites: vec!["a".into()],
            ..sample()
        };
        assert!(config.add_favorite("chrome"));
        assert_eq!(config.favorites, ["a", "chrome"]);
        assert!(!config.add_favorite("chrome"), "no duplicates");
        assert!(config.remove_favorite(0));
        assert!(!config.remove_favorite(5));
        assert_eq!(config.favorites, ["chrome"]);
    }

    #[test]
    fn the_favorites_limit_is_twelve() {
        let mut config = HomeConfig {
            favorites: (0..11).map(|i| format!("app{i}")).collect(),
            ..sample()
        };
        assert!(!config.favorites_full());
        assert!(config.add_favorite("twelfth"));
        assert!(config.favorites_full());
        assert!(!config.add_favorite("thirteenth"));
        assert_eq!(config.favorites.len(), 12);
    }

    #[test]
    fn uninstalled_apps_leave_favorites_and_carousel_keeping_the_order() {
        let mut config = sample();
        assert!(config.remove_uninstalled(&catalog(&[NETFLIX, SPOTIFY])));
        assert_eq!(config.favorites, [NETFLIX, SPOTIFY]);
        assert_eq!(config.carousel_apps, [NETFLIX]);
        assert!(!config.remove_uninstalled(&catalog(&[NETFLIX, SPOTIFY])));
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lume-config-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("config.v1")
    }

    #[test]
    fn the_configuration_round_trips() {
        let mut config = sample();
        config.move_section_down(Section::Carousel);
        config.set_section_visible(Section::Carousel, false);
        let path = temp("roundtrip");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
    }

    #[test]
    fn an_empty_list_survives_the_round_trip() {
        let config = HomeConfig {
            favorites: vec![],
            carousel_apps: vec![],
            ..sample()
        };
        let path = temp("empty");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
    }

    #[test]
    fn missing_corrupt_or_foreign_files_mean_no_configuration() {
        let path = temp("bad");
        assert_eq!(HomeConfig::load(&path), None);
        std::fs::write(&path, "garbage").unwrap();
        assert_eq!(HomeConfig::load(&path), None);
        std::fs::write(&path, "lume=2\nsections=carousel\n#end\n").unwrap();
        assert_eq!(HomeConfig::load(&path), None);
        std::fs::write(&path, "lume=1\nsections=carousel,favorites\n#end\n").unwrap();
        assert_eq!(HomeConfig::load(&path), None, "the app lists are required");
    }

    #[test]
    fn out_of_range_values_are_repaired() {
        let path = temp("repair");
        let mut kv = Kv::new();
        kv.set("sections", "favorites,unknown");
        kv.set("section.favorites.visible", "0");
        kv.set("carousel.apps", "a,,a,b");
        kv.set("carousel.interval", "7");
        kv.set(
            "favorites",
            (0..20)
                .map(|i| format!("app{i}"))
                .collect::<Vec<_>>()
                .join(","),
        );
        storage::save(&path, 1, &kv).ok();
        // "unknown" is not a section, so the whole file is rejected as corrupt.
        assert_eq!(HomeConfig::load(&path), None);

        kv.set("sections", "favorites");
        storage::save(&path, 1, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        // The missing section is appended; visibility defaults to shown.
        assert_eq!(
            config
                .sections
                .iter()
                .map(|s| s.section)
                .collect::<Vec<_>>(),
            [Section::Favorites, Section::Carousel]
        );
        assert!(!config.is_visible(Section::Favorites) && config.is_visible(Section::Carousel));
        assert_eq!(config.carousel_apps, ["a", "b"]);
        assert_eq!(config.interval, Interval::Seconds5);
        assert_eq!(config.favorites.len(), MAX_FAVORITES);
    }
}
