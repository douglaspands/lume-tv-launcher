//! The user's home screen setup: the favorites (`config.v1`).

use crate::catalog::known::KnownApp;
use crate::catalog::Catalog;
use crate::storage::{self, Kv, StorageError};
use crate::wallpaper::custom::Applied;
use crate::wallpaper::WallpaperId;
use std::path::Path;

const VERSION: u32 = 1;

/// Most favorites the home screen can hold.
pub const MAX_FAVORITES: usize = 12;
/// Favorites chosen automatically on first use.
pub const FIRST_USE_FAVORITES: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeConfig {
    /// Favorite apps, left to right.
    pub favorites: Vec<String>,
    /// Whether favorite dock icons are rendered as circles instead of rounded squares.
    pub round_favorites: bool,
    /// Active wallpaper choice.
    pub wallpaper: WallpaperId,
    /// "Minha imagem", when there is one: its framing and the revision of its result. The source
    /// itself is a file (`files/wallpaper/custom.qoi`).
    pub custom: Option<Applied>,
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
            round_favorites: true,
            wallpaper: WallpaperId::Aurora,
            custom: None,
        }
    }

    // ---- editing ------------------------------------------------------------------------

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

    /// Takes apps that are no longer installed out of the favorites, keeping the order of the
    /// rest. Returns `true` when something changed.
    pub fn remove_uninstalled(&mut self, catalog: &Catalog) -> bool {
        let before = self.favorites.len();
        self.favorites.retain(|p| catalog.contains(p));
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
        kv.set(
            "round_favorites",
            if self.round_favorites { "1" } else { "0" },
        );
        kv.set("wallpaper", self.wallpaper.as_str());
        if let Some(custom) = &self.custom {
            kv.set("custom_rev", custom.rev.to_string());
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
        // Round is the default: a file without the key (an older version) gets it too.
        let round_favorites = kv.get("round_favorites").is_none_or(|value| value == "1");
        let custom = Self::custom_from_kv(kv);
        let wallpaper = kv
            .get("wallpaper")
            .and_then(WallpaperId::from_str)
            .filter(|id| *id != WallpaperId::Custom || custom.is_some())
            .unwrap_or(WallpaperId::Aurora);
        Some(HomeConfig {
            favorites,
            round_favorites,
            wallpaper,
            custom,
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
            round_favorites: false,
            wallpaper: WallpaperId::Aurora,
            custom: None,
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
            round_favorites: false,
            wallpaper: WallpaperId::Aurora,
            custom: None,
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
            round_favorites: false,
            wallpaper: WallpaperId::Aurora,
            custom: None,
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
            round_favorites: false,
            wallpaper: WallpaperId::Aurora,
            custom: None,
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
        assert!(text.contains("round_favorites=0"));
        assert!(!text.contains("sections") && !text.contains("carousel"));
    }

    #[test]
    fn an_empty_list_survives_the_round_trip() {
        let config = HomeConfig {
            favorites: vec![],
            round_favorites: false,
            wallpaper: WallpaperId::Aurora,
            custom: None,
        };
        let path = temp("empty");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
    }

    #[test]
    fn round_favorites_round_trips() {
        let config = HomeConfig {
            favorites: vec![NETFLIX.into()],
            round_favorites: true,
            wallpaper: WallpaperId::Aurora,
            custom: None,
        };
        let path = temp("round-favs-true");
        config.save(&path).unwrap();
        assert_eq!(HomeConfig::load(&path), Some(config));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("round_favorites=1"));
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
        assert!(
            config.round_favorites,
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
            round_favorites: false,
            wallpaper: WallpaperId::MirrorLake,
            custom: None,
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
}
