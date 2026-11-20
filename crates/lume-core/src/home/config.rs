//! The user's home screen setup: the favorites (`config.v1`).

use crate::catalog::known::KnownApp;
use crate::catalog::Catalog;
use crate::storage::{self, Kv, StorageError};
use crate::theme::AccentColor;
use crate::wallpaper::custom::Applied;
use crate::wallpaper::WallpaperId;
use std::path::Path;

const VERSION: u32 = 1;

/// Most favorites the home screen can hold.
pub const MAX_FAVORITES: usize = 12;
/// Favorites chosen automatically on first use.
pub const FIRST_USE_FAVORITES: usize = 5;

/// Horizontal alignment of the top bar tabs when visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabsAlign {
    #[default]
    Left,
    Center,
}

impl TabsAlign {
    pub const fn as_str(&self) -> &'static str {
        match self {
            TabsAlign::Left => "left",
            TabsAlign::Center => "center",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "left" => Some(TabsAlign::Left),
            "center" => Some(TabsAlign::Center),
            _ => None,
        }
    }
}

/// How the user reaches the apps grid when top bar tabs are hidden.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TablessMode {
    #[default]
    Slide,
    Peek,
    Dock,
}

impl TablessMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            TablessMode::Slide => "slide",
            TablessMode::Peek => "peek",
            TablessMode::Dock => "dock",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "slide" => Some(TablessMode::Slide),
            "peek" => Some(TablessMode::Peek),
            "dock" => Some(TablessMode::Dock),
            _ => None,
        }
    }
}

/// Visual shape of favorite items on the Home dock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DockIconShape {
    /// Circular icons (default).
    #[default]
    Round,
    /// Rounded square icons.
    Square,
    /// Rectangular 16:9 TV banners (reusing the Apps grid banner atlas).
    Banner,
}

impl DockIconShape {
    pub const fn as_str(&self) -> &'static str {
        match self {
            DockIconShape::Round => "round",
            DockIconShape::Square => "square",
            DockIconShape::Banner => "banner",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "round" => Some(DockIconShape::Round),
            "square" => Some(DockIconShape::Square),
            "banner" => Some(DockIconShape::Banner),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeConfig {
    /// Favorite apps, left to right.
    pub favorites: Vec<String>,
    /// Shape of favorite items on the dock (`round`, `square`, or `banner` 16:9).
    pub dock_shape: DockIconShape,
    /// Active wallpaper choice.
    pub wallpaper: WallpaperId,
    /// "Minha imagem", when there is one: its framing and the revision of its result. The source
    /// itself is a file (`files/wallpaper/custom.qoi`).
    pub custom: Option<Applied>,
    /// Active accent color preset (`--primary` and Splash T7 background).
    pub accent_color: AccentColor,
    /// Whether the top bar shows the Home / Apps tabs.
    pub tabs_visible: bool,
    /// Horizontal alignment of the top bar tabs when `tabs_visible` is `true`.
    pub tabs_align: TabsAlign,
    /// How the user opens the apps grid when `tabs_visible` is `false`.
    pub tabless_mode: TablessMode,
    /// Position of the fixed "All apps" slot in the Dock when `tabless_mode == TablessMode::Dock`.
    /// `None` places it at the end (`favorites.len()`). Always clamped to `0..=favorites.len()`.
    pub all_apps_dock_index: Option<usize>,
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
            favorites: installed.into_iter().take(FIRST_USE_FAVORITES).collect(),
            dock_shape: DockIconShape::Round,
            wallpaper: WallpaperId::Aurora,
            custom: None,
            accent_color: AccentColor::Blue,
            tabs_visible: true,
            tabs_align: TabsAlign::Left,
            tabless_mode: TablessMode::Slide,
            all_apps_dock_index: None,
        }
    }

    // ---- editing ------------------------------------------------------------------------

    /// Whether the Dock currently includes the extra fixed "All apps" slot.
    pub fn has_dock_all_apps(&self) -> bool {
        !self.tabs_visible && self.tabless_mode == TablessMode::Dock
    }

    /// Effective slot index (`0..=favorites.len()`) of the "All apps" slot in the Dock.
    pub fn effective_all_apps_dock_index(&self) -> usize {
        self.all_apps_dock_index
            .map_or(self.favorites.len(), |idx| idx.min(self.favorites.len()))
    }

    /// Total number of slots displayed in the Dock (favorites plus optional "All apps" slot).
    pub fn dock_slot_count(&self) -> usize {
        self.favorites.len() + usize::from(self.has_dock_all_apps())
    }

    /// Maps a Dock slot index to its index in `favorites`, or `None` if it is the fixed "All apps"
    /// slot or out of bounds.
    pub fn dock_slot_favorite_index(&self, slot: usize) -> Option<usize> {
        if self.has_dock_all_apps() {
            let all_idx = self.effective_all_apps_dock_index();
            if slot == all_idx || slot > self.favorites.len() {
                None
            } else if slot < all_idx {
                Some(slot)
            } else {
                Some(slot - 1)
            }
        } else {
            (slot < self.favorites.len()).then_some(slot)
        }
    }

    /// Moves a Dock slot (either a favorite or the fixed "All apps" slot) from `from` to `to`.
    /// Returns `false` if either index is out of bounds or `from == to`.
    pub fn move_dock_slot(&mut self, from: usize, to: usize) -> bool {
        let total = self.dock_slot_count();
        if from >= total || to >= total || from == to {
            return false;
        }
        if !self.has_dock_all_apps() {
            let pkg = self.favorites.remove(from);
            self.favorites.insert(to, pkg);
            return true;
        }
        let all_idx = self.effective_all_apps_dock_index();
        if from == all_idx {
            self.all_apps_dock_index = Some(to);
            return true;
        }
        let fav_from = if from < all_idx { from } else { from - 1 };
        let all_after_remove = if from < all_idx { all_idx - 1 } else { all_idx };
        let (fav_to, new_all_idx) = if to <= all_after_remove {
            (to, all_after_remove + 1)
        } else {
            (to - 1, all_after_remove)
        };
        let pkg = self.favorites.remove(fav_from);
        self.favorites.insert(fav_to, pkg);
        self.all_apps_dock_index = Some(new_all_idx);
        true
    }

    /// Moves a Dock slot one place to the left. Returns `false` at the first slot or out of bounds.
    pub fn move_dock_slot_left(&mut self, slot: usize) -> bool {
        if slot == 0 {
            return false;
        }
        self.move_dock_slot(slot, slot - 1)
    }

    /// Moves a Dock slot one place to the right. Returns `false` at the last slot or out of bounds.
    pub fn move_dock_slot_right(&mut self, slot: usize) -> bool {
        self.move_dock_slot(slot, slot + 1)
    }

    /// Removes the favorite at Dock `slot`. Never removes the fixed "All apps" slot.
    pub fn remove_dock_slot(&mut self, slot: usize) -> bool {
        let Some(fav_idx) = self.dock_slot_favorite_index(slot) else {
            return false;
        };
        self.remove_favorite(fav_idx)
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
        if let Some(slot) = self.all_apps_dock_index {
            let adjusted = if slot > index { slot - 1 } else { slot };
            self.all_apps_dock_index = Some(adjusted.min(self.favorites.len()));
        }
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

    /// Takes apps that are no longer installed out of the favorites, keeping the order of the
    /// rest. Returns `true` when something changed.
    pub fn remove_uninstalled(&mut self, catalog: &Catalog) -> bool {
        let before = self.favorites.len();
        if let Some(slot) = self.all_apps_dock_index {
            let removed_before = self
                .favorites
                .iter()
                .take(slot.min(before))
                .filter(|p| !catalog.contains(p))
                .count();
            self.favorites.retain(|p| catalog.contains(p));
            let adjusted = slot
                .saturating_sub(removed_before)
                .min(self.favorites.len());
            self.all_apps_dock_index = Some(adjusted);
        } else {
            self.favorites.retain(|p| catalog.contains(p));
        }
        before != self.favorites.len()
    }

    /// Drops "Minha imagem" when its picture is not in `files/` any more (logged), going back to the
    /// Aurora if it was the active wallpaper. Returns `true` when something changed.
    pub fn reconcile_custom(&mut self, files: &Path) -> bool {
        if self.custom.is_none() || crate::wallpaper::custom::has_kept(files) {
            return false;
        }
        log::warn!("custom wallpaper: the picture is missing, back to the Aurora");
        self.forget_custom();
        true
    }

    /// Removes "Minha imagem": deletes its picture, thumbnail and cached results, forgets its keys and
    /// goes back to the Aurora when it was the active wallpaper. The caller saves the configuration.
    pub fn remove_custom(&mut self, files: &Path, cache: Option<&Path>) {
        crate::wallpaper::custom::remove(files, cache);
        self.forget_custom();
        crate::wallpaper::custom::set_applied(None);
        crate::wallpaper::custom::set_thumbnail(None);
    }

    /// Forgets "Minha imagem" (its files are deleted by the caller); the Aurora when it was active.
    pub fn forget_custom(&mut self) {
        self.custom = None;
        if self.wallpaper == WallpaperId::Custom {
            self.wallpaper = WallpaperId::Aurora;
        }
    }

    // ---- storage ------------------------------------------------------------------------

    fn to_kv(&self) -> Kv {
        let mut kv = Kv::new();
        kv.set_list("favorites", self.favorites.iter().map(String::as_str));
        kv.set("dock_shape", self.dock_shape.as_str());
        kv.set(
            "round_favorites",
            if self.dock_shape == DockIconShape::Round {
                "1"
            } else {
                "0"
            },
        );
        kv.set("wallpaper", self.wallpaper.as_str());
        if let Some(custom) = &self.custom {
            kv.set("custom_rev", custom.rev.to_string());
        }
        kv.set("accent_color", self.accent_color.as_str());
        kv.set("tabs_visible", if self.tabs_visible { "1" } else { "0" });
        kv.set("tabs_align", self.tabs_align.as_str());
        kv.set("tabless_mode", self.tabless_mode.as_str());
        if let Some(index) = self.all_apps_dock_index {
            kv.set(
                "all_apps_dock_index",
                index.min(self.favorites.len()).to_string(),
            );
        }
        kv
    }

    /// "Minha imagem" from its key: `None` without `custom_rev`. The framing keys a development
    /// version wrote (`custom_fit`, `custom_zoom`...) are ignored: the TV keeps the framed result.
    fn custom_from_kv(kv: &Kv) -> Option<Applied> {
        kv.parse::<u32>("custom_rev").map(|rev| Applied { rev })
    }

    /// Reads a file's values, repairing anything out of range, and `None` when the favorites are
    /// missing. Keys from older versions (sections and carousel) are ignored.
    fn from_kv(kv: &Kv) -> Option<HomeConfig> {
        let mut favorites: Vec<String> = Vec::new();
        for item in kv.list("favorites")? {
            if !item.is_empty() && !favorites.iter().any(|f| f == item) {
                favorites.push(item.to_string());
            }
        }
        favorites.truncate(MAX_FAVORITES);
        // `dock_shape` takes precedence; if absent, migrate from legacy `round_favorites`
        // ("0" -> Square, "1" or absent -> Round). Invalid `dock_shape` values fall back to Round.
        let dock_shape = match kv.get("dock_shape") {
            Some(value) => DockIconShape::from_str(value).unwrap_or_default(),
            None => match kv.get("round_favorites") {
                Some("0") => DockIconShape::Square,
                _ => DockIconShape::Round,
            },
        };
        let custom = Self::custom_from_kv(kv);
        let wallpaper = kv
            .get("wallpaper")
            .and_then(WallpaperId::from_str)
            .filter(|id| *id != WallpaperId::Custom || custom.is_some())
            .unwrap_or(WallpaperId::Aurora);
        let accent_color = kv
            .get("accent_color")
            .and_then(AccentColor::from_str)
            .unwrap_or_default();
        // Visible tabs are the default; missing or invalid values fall back to `true`.
        let tabs_visible = kv.get("tabs_visible") != Some("0");
        let tabs_align = kv
            .get("tabs_align")
            .and_then(TabsAlign::from_str)
            .unwrap_or_default();
        let tabless_mode = kv
            .get("tabless_mode")
            .and_then(TablessMode::from_str)
            .unwrap_or_default();
        let all_apps_dock_index = kv
            .parse::<usize>("all_apps_dock_index")
            .map(|idx| idx.min(favorites.len()));
        Some(HomeConfig {
            favorites,
            dock_shape,
            wallpaper,
            custom,
            accent_color,
            tabs_visible,
            tabs_align,
            tabless_mode,
            all_apps_dock_index,
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
                system: false,
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
            favorites: vec![NETFLIX.into(), YOUTUBE.into(), SPOTIFY.into()],
            dock_shape: DockIconShape::Square,
            wallpaper: WallpaperId::Aurora,
            custom: None,
            accent_color: AccentColor::Blue,
            tabs_visible: true,
            tabs_align: TabsAlign::Left,
            tabless_mode: TablessMode::Slide,
            all_apps_dock_index: None,
        }
    }

    #[test]
    fn first_use_picks_the_installed_known_apps_in_order() {
        // Installed in a different order than the preferred one, plus unknown apps.
        let config = HomeConfig::first_use(&catalog(&[SPOTIFY, "com.example.x", NETFLIX, YOUTUBE]));
        assert_eq!(config.favorites, [NETFLIX, YOUTUBE, SPOTIFY]);
    }

    #[test]
    fn first_use_limits_favorites_to_five() {
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
    }

    #[test]
    fn first_use_with_no_known_apps_is_empty() {
        let config = HomeConfig::first_use(&catalog(&["com.example.x"]));
        assert!(config.favorites.is_empty());
    }

    #[test]
    fn first_use_uses_whichever_package_of_a_known_app_is_installed() {
        let config = HomeConfig::first_use(&catalog(&["com.netflix.mediaclient"]));
        assert_eq!(config.favorites, ["com.netflix.mediaclient"]);
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
    fn uninstalled_apps_leave_the_favorites_keeping_the_order() {
        let mut config = sample();
        assert!(config.remove_uninstalled(&catalog(&[NETFLIX, SPOTIFY])));
        assert_eq!(config.favorites, [NETFLIX, SPOTIFY]);
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
        let config = sample();
        let path = temp("roundtrip");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
    }

    #[test]
    fn the_file_only_holds_the_favorites() {
        let path = temp("only-favorites");
        sample().save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("favorites="));
        assert!(text.contains("dock_shape=square"));
        assert!(text.contains("round_favorites=0"));
        assert!(!text.contains("sections") && !text.contains("carousel"));
    }

    #[test]
    fn an_empty_list_survives_the_round_trip() {
        let config = HomeConfig {
            favorites: vec![],
            ..sample()
        };
        let path = temp("empty");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
    }

    #[test]
    fn dock_shape_round_trips_and_writes_legacy_round_favorites() {
        for (shape, expected_round_flag) in [
            (DockIconShape::Round, "1"),
            (DockIconShape::Square, "0"),
            (DockIconShape::Banner, "0"),
        ] {
            assert_eq!(DockIconShape::from_str(shape.as_str()), Some(shape));
            let config = HomeConfig {
                favorites: vec![NETFLIX.into()],
                dock_shape: shape,
                ..sample()
            };
            let path = temp(&format!("dock-shape-{}", shape.as_str()));
            config.save(&path).unwrap();
            assert_eq!(HomeConfig::load(&path), Some(config));
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(text.contains(&format!("dock_shape={}", shape.as_str())));
            assert!(text.contains(&format!("round_favorites={expected_round_flag}")));
        }
        assert_eq!(DockIconShape::from_str("triangle"), None);
    }

    #[test]
    fn legacy_round_favorites_migrates_when_dock_shape_is_absent() {
        // Legacy round_favorites=0 -> Square
        let path_sq = temp("legacy-square");
        let mut kv = Kv::new();
        kv.set("favorites", NETFLIX);
        kv.set("round_favorites", "0");
        storage::save(&path_sq, VERSION, &kv).unwrap();
        assert_eq!(
            HomeConfig::load(&path_sq).unwrap().dock_shape,
            DockIconShape::Square
        );

        // Legacy round_favorites=1 -> Round
        let path_rd = temp("legacy-round");
        kv.set("round_favorites", "1");
        storage::save(&path_rd, VERSION, &kv).unwrap();
        assert_eq!(
            HomeConfig::load(&path_rd).unwrap().dock_shape,
            DockIconShape::Round
        );

        // Neither key -> Round (default)
        let path_none = temp("legacy-missing");
        let mut kv_empty = Kv::new();
        kv_empty.set("favorites", NETFLIX);
        storage::save(&path_none, VERSION, &kv_empty).unwrap();
        assert_eq!(
            HomeConfig::load(&path_none).unwrap().dock_shape,
            DockIconShape::Round
        );

        // Invalid dock_shape -> Round (repaired)
        let path_invalid = temp("invalid-dock-shape");
        let mut kv_bad = Kv::new();
        kv_bad.set("favorites", NETFLIX);
        kv_bad.set("round_favorites", "0");
        kv_bad.set("dock_shape", "hexagon");
        storage::save(&path_invalid, VERSION, &kv_bad).unwrap();
        assert_eq!(
            HomeConfig::load(&path_invalid).unwrap().dock_shape,
            DockIconShape::Round
        );
    }

    #[test]
    fn missing_corrupt_or_foreign_files_mean_no_configuration() {
        let path = temp("bad");
        assert_eq!(HomeConfig::load(&path), None);
        std::fs::write(&path, "garbage").unwrap();
        assert_eq!(HomeConfig::load(&path), None);
        std::fs::write(&path, "lume=2\nfavorites=a\n#end\n").unwrap();
        assert_eq!(HomeConfig::load(&path), None);
        std::fs::write(&path, "lume=1\nsections=carousel,favorites\n#end\n").unwrap();
        assert_eq!(HomeConfig::load(&path), None, "the favorites are required");
    }

    #[test]
    fn a_file_from_the_carousel_version_keeps_the_favorites_in_order() {
        let path = temp("old");
        let mut kv = Kv::new();
        kv.set("sections", "favorites,carousel");
        kv.set("section.favorites.visible", "0");
        kv.set("carousel.apps", "a,b");
        kv.set("carousel.interval", "10");
        kv.set("favorites", "c,b,a");
        storage::save(&path, 1, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.favorites, ["c", "b", "a"]);
        assert_eq!(
            config.dock_shape,
            DockIconShape::Round,
            "a file without the key gets the default"
        );
    }

    #[test]
    fn out_of_range_values_are_repaired() {
        let path = temp("repair");
        let mut kv = Kv::new();
        kv.set(
            "favorites",
            ["a", "", "a"]
                .into_iter()
                .map(String::from)
                .chain((0..20).map(|i| format!("app{i}")))
                .collect::<Vec<_>>()
                .join(","),
        );
        storage::save(&path, 1, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.favorites.len(), MAX_FAVORITES);
        assert_eq!(config.favorites[0], "a");
        assert_eq!(
            config.favorites[1], "app0",
            "empty and repeated names are dropped"
        );
    }

    #[test]
    fn wallpaper_round_trips() {
        let config = HomeConfig {
            favorites: vec![NETFLIX.into()],
            wallpaper: WallpaperId::MirrorLake,
            ..sample()
        };
        let path = temp("wallpaper-roundtrip");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("wallpaper=mirror_lake"));
    }

    #[test]
    fn missing_wallpaper_key_defaults_to_aurora() {
        let path = temp("missing-wallpaper");
        let mut kv = Kv::new();
        kv.set("favorites", "com.netflix.ninja");
        kv.set("round_favorites", "0");
        storage::save(&path, 1, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.wallpaper, WallpaperId::Aurora);
    }

    #[test]
    fn invalid_wallpaper_key_repairs_to_aurora() {
        let path = temp("invalid-wallpaper");
        let mut kv = Kv::new();
        kv.set("favorites", "com.netflix.ninja");
        kv.set("round_favorites", "0");
        kv.set("wallpaper", "non_existent_wallpaper_id");
        storage::save(&path, 1, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.wallpaper, WallpaperId::Aurora);
    }

    #[test]
    fn my_image_round_trips_with_only_its_revision() {
        let mut config = sample();
        config.wallpaper = WallpaperId::Custom;
        config.custom = Some(Applied { rev: 7 });
        let path = temp("custom");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("custom_rev=7"), "{text}");
        assert!(!text.contains("custom_fit"), "{text}");
    }

    #[test]
    fn an_old_file_without_the_custom_keys_opens_with_the_aurora() {
        let path = temp("old-custom");
        let mut kv = Kv::new();
        kv.set_list("favorites", [NETFLIX]);
        kv.set("wallpaper", "custom");
        storage::save(&path, VERSION, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.wallpaper, WallpaperId::Aurora);
        assert_eq!(config.custom, None);
    }

    #[test]
    fn the_framing_keys_of_a_development_version_are_ignored() {
        let path = temp("custom-range");
        let mut kv = Kv::new();
        kv.set_list("favorites", [NETFLIX]);
        kv.set("wallpaper", "custom");
        kv.set("custom_fit", "tile");
        kv.set("custom_scale", "400");
        kv.set("custom_zoom", "x");
        kv.set("custom_x", "-300");
        kv.set("custom_tile", "mirror");
        kv.set("custom_rev", "2");
        storage::save(&path, VERSION, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        // The framing keys of a development version are ignored: the revision is enough.
        assert_eq!(config.wallpaper, WallpaperId::Custom);
        assert_eq!(config.custom, Some(Applied { rev: 2 }));
    }

    #[test]
    fn a_missing_source_falls_back_to_the_aurora() {
        let files = temp("reconcile").parent().unwrap().to_path_buf();
        let mut config = sample();
        config.wallpaper = WallpaperId::Custom;
        config.custom = Some(Applied { rev: 1 });
        assert!(config.reconcile_custom(&files));
        assert_eq!(
            (config.wallpaper, config.custom),
            (WallpaperId::Aurora, None)
        );
        assert!(!config.reconcile_custom(&files));
    }

    #[test]
    fn removing_my_image_deletes_its_files_and_keys_and_goes_back_to_the_aurora() {
        use crate::wallpaper::{custom, Image};
        let dir = temp("remove").parent().unwrap().to_path_buf();
        let (files, cache) = (dir.join("files"), dir.join("cache"));
        let result = Image::black(custom::WIDTH, custom::HEIGHT);
        let applied = Applied { rev: 1 };
        custom::apply(&files, Some(&cache), "1.0", &result, applied).unwrap();
        let mut config = sample();
        config.wallpaper = WallpaperId::Custom;
        config.custom = Some(applied);
        let path = dir.join("config.v1");
        config.remove_custom(&files, Some(&cache));
        config.save(&path).unwrap();
        assert!(!custom::has_kept(&files));
        assert!(std::fs::read_dir(&cache).unwrap().next().is_none());
        assert_eq!(config.wallpaper, WallpaperId::Aurora);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("custom"), "{text}");
        // Not the active one: the wallpaper stays.
        let mut config = sample();
        config.wallpaper = WallpaperId::MirrorLake;
        config.custom = Some(applied);
        config.remove_custom(&files, Some(&cache));
        assert_eq!(
            (config.wallpaper, config.custom),
            (WallpaperId::MirrorLake, None)
        );
    }

    #[test]
    fn tabs_align_and_tabless_mode_round_trip() {
        for align in [TabsAlign::Left, TabsAlign::Center] {
            assert_eq!(TabsAlign::from_str(align.as_str()), Some(align));
        }
        assert_eq!(TabsAlign::from_str("right"), None);

        for mode in [TablessMode::Slide, TablessMode::Peek, TablessMode::Dock] {
            assert_eq!(TablessMode::from_str(mode.as_str()), Some(mode));
        }
        assert_eq!(TablessMode::from_str("grid"), None);
    }

    #[test]
    fn personalization_fields_round_trip_in_config_v1() {
        let config = HomeConfig {
            favorites: vec![NETFLIX.into(), YOUTUBE.into(), SPOTIFY.into()],
            dock_shape: DockIconShape::Banner,
            wallpaper: WallpaperId::Aurora,
            custom: None,
            accent_color: AccentColor::Emerald,
            tabs_visible: false,
            tabs_align: TabsAlign::Center,
            tabless_mode: TablessMode::Dock,
            all_apps_dock_index: Some(1),
        };
        let path = temp("personalization-roundtrip");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("dock_shape=banner"), "{text}");
        assert!(text.contains("round_favorites=0"), "{text}");
        assert!(text.contains("accent_color=emerald"), "{text}");
        assert!(text.contains("tabs_visible=0"), "{text}");
        assert!(text.contains("tabs_align=center"), "{text}");
        assert!(text.contains("tabless_mode=dock"), "{text}");
        assert!(text.contains("all_apps_dock_index=1"), "{text}");
    }

    #[test]
    fn old_config_without_new_keys_uses_defaults() {
        let path = temp("backward-compat");
        let mut kv = Kv::new();
        kv.set("favorites", "com.netflix.ninja,com.spotify.tv.android");
        kv.set("round_favorites", "1");
        kv.set("wallpaper", "aurora");
        storage::save(&path, VERSION, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.accent_color, AccentColor::Blue);
        assert!(config.tabs_visible);
        assert_eq!(config.tabs_align, TabsAlign::Left);
        assert_eq!(config.tabless_mode, TablessMode::Slide);
        assert_eq!(config.all_apps_dock_index, None);
    }

    #[test]
    fn invalid_personalization_values_are_repaired() {
        let path = temp("repair-personalization");
        let mut kv = Kv::new();
        kv.set("favorites", "a,b");
        kv.set("accent_color", "neon");
        kv.set("tabs_visible", "maybe");
        kv.set("tabs_align", "right");
        kv.set("tabless_mode", "grid");
        kv.set("all_apps_dock_index", "not_a_number");
        storage::save(&path, VERSION, &kv).unwrap();
        let config = HomeConfig::load(&path).unwrap();
        assert_eq!(config.accent_color, AccentColor::Blue);
        assert!(config.tabs_visible);
        assert_eq!(config.tabs_align, TabsAlign::Left);
        assert_eq!(config.tabless_mode, TablessMode::Slide);
        assert_eq!(config.all_apps_dock_index, None);

        // Out-of-bounds numeric index is clamped to favorites.len().
        let path_clamp = temp("clamp-dock-index");
        kv.set("all_apps_dock_index", "99");
        storage::save(&path_clamp, VERSION, &kv).unwrap();
        let clamped = HomeConfig::load(&path_clamp).unwrap();
        assert_eq!(clamped.all_apps_dock_index, Some(2));
    }

    #[test]
    fn dock_all_apps_extra_slot_reorders_and_cannot_be_removed() {
        let mut config = HomeConfig {
            favorites: (0..MAX_FAVORITES).map(|i| format!("app{i}")).collect(),
            tabs_visible: false,
            tabless_mode: TablessMode::Dock,
            all_apps_dock_index: None,
            ..sample()
        };
        assert!(config.favorites_full());
        assert!(config.has_dock_all_apps());
        assert_eq!(config.dock_slot_count(), MAX_FAVORITES + 1);
        assert_eq!(config.effective_all_apps_dock_index(), MAX_FAVORITES);
        assert_eq!(config.dock_slot_favorite_index(MAX_FAVORITES), None);
        assert!(!config.remove_dock_slot(MAX_FAVORITES));
        assert_eq!(config.favorites.len(), MAX_FAVORITES);

        // Move the fixed "All apps" slot left by one (12 -> 11).
        assert!(config.move_dock_slot_left(MAX_FAVORITES));
        assert_eq!(config.all_apps_dock_index, Some(11));
        assert_eq!(config.dock_slot_favorite_index(11), None);
        assert_eq!(config.dock_slot_favorite_index(12), Some(11));

        // Move favorite at slot 10 right across "All apps" (10 -> 11).
        assert!(config.move_dock_slot_right(10));
        assert_eq!(config.all_apps_dock_index, Some(10));
        assert_eq!(config.favorites[10], "app10");

        // Removing a favorite before "All apps" shifts its index down by 1.
        assert!(config.remove_favorite(0));
        assert_eq!(config.favorites.len(), 11);
        assert_eq!(config.all_apps_dock_index, Some(9));

        // Removing uninstalled apps adjusts and clamps the slot safely.
        let keep = catalog(&["app1", "app11"]);
        assert!(config.remove_uninstalled(&keep));
        assert_eq!(config.favorites, ["app1", "app11"]);
        assert_eq!(config.all_apps_dock_index, Some(1));
        assert!(!config.remove_dock_slot(1));
        assert!(config.remove_dock_slot(0));
        assert_eq!(config.favorites, ["app11"]);
        assert_eq!(config.all_apps_dock_index, Some(0));
    }
}
