//! The wallpaper: the single place the renderer gets it from.
//!
//! A catalog of 6 high definition wallpapers (1920x1080, lossless QOI) read from the platform's
//! assets (the APK's `assets/` on Android, the repository on the desktop), not embedded in the
//! library: the APK carries one copy of each. Sharp stars drawn as points on top when present.

pub mod custom;

use crate::i18n::StringId;
use crate::memory::Buffer;
use std::sync::{Arc, OnceLock};

/// An RGB picture, 3 bytes per pixel, row by row. Its pixels are a [`Buffer`]: a full-size picture
/// has its own mapping, which goes back to the system as soon as the picture is dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgb: Buffer,
}

impl Image {
    /// A picture of `width` x `height` filled with black.
    pub fn black(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            rgb: Buffer::zeroed(width as usize * height as usize * 3),
        }
    }
}

/// Reads a wallpaper asset (a file name such as `aurora-1920x1080.qoi`) from the platform.
pub type AssetReader = fn(&str) -> Option<Buffer>;

static ASSET_READER: OnceLock<AssetReader> = OnceLock::new();

/// Sets the platform's asset reader (Android: the `AAssetManager`). Without one, desktop builds
/// read the repository's `assets/wallpaper/` directory.
pub fn set_asset_reader(reader: AssetReader) {
    let _ = ASSET_READER.set(reader);
}

#[cfg(not(target_os = "android"))]
fn default_reader() -> Option<AssetReader> {
    Some(|name| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/wallpaper");
        Buffer::read_file(&dir.join(name), None).ok()
    })
}

#[cfg(target_os = "android")]
fn default_reader() -> Option<AssetReader> {
    None
}

static CACHE_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Sets the directory where the decoded wallpaper in use is kept between wake-ups (Android: the app's
/// cache directory, which the system may clear and does not back up). Without one there is no cache.
pub fn set_cache_dir(dir: std::path::PathBuf) {
    let _ = CACHE_DIR.set(dir);
}

/// The cache directory, when the platform set one.
pub fn cache_dir() -> Option<&'static std::path::Path> {
    CACHE_DIR.get().map(|dir| dir.as_path())
}

static READY_NOTIFIER: OnceLock<fn()> = OnceLock::new();

/// Sets what the platform does when a wallpaper prepared off the frame loop is ready (Android: wake the
/// loop, so it draws the frame that swaps it in).
pub fn set_ready_notifier(notifier: fn()) {
    let _ = READY_NOTIFIER.set(notifier);
}

/// Where the picture of a [`WallpaperId`] comes from.
#[derive(Clone, Copy, Debug)]
pub enum Source {
    /// One of the catalog's, from the APK's assets.
    Catalog(&'static Wallpaper),
    /// "Minha imagem": the Confirm screen's picture, the cache or the result in `files/` (see [`custom`]).
    Custom,
}

impl Source {
    /// The source of `id`.
    pub fn of(id: WallpaperId) -> Source {
        match id {
            WallpaperId::Custom => Source::Custom,
            catalog => Source::Catalog(catalog.wallpaper()),
        }
    }

    /// Reads the picture (blocking: off the frame loop). "Minha imagem" without a usable picture
    /// gives the Aurora, as the configuration does with an unknown wallpaper.
    pub fn load(self) -> Option<Arc<Image>> {
        match self {
            Source::Catalog(wallpaper) => wallpaper.load_rgb().map(|rgb| {
                Arc::new(Image {
                    width: wallpaper.width,
                    height: wallpaper.height,
                    rgb,
                })
            }),
            Source::Custom => custom::load_current(
                CACHE_DIR.get().map(|dir| dir.as_path()),
                lume_version::VERSION,
            )
            .or_else(|| Source::Catalog(&AURORA).load()),
        }
    }
}

/// Tells the platform that a wallpaper picture is done off the frame loop (Apply).
pub fn notify_ready() {
    if let Some(notify) = READY_NOTIFIER.get() {
        notify();
    }
}

/// The size of the picture that covers a render buffer of `width` x `height` keeping 16:9 (the
/// wallpaper texture's size).
pub fn cover_size(width: u32, height: u32) -> (u32, u32) {
    if u64::from(width) * 9 >= u64::from(height) * 16 {
        (width, (width * 9).div_ceil(16))
    } else {
        ((height * 16).div_ceil(9), height)
    }
}

/// Reads and decodes the wallpaper on a worker thread; the receiver gets the picture (or `None` when
/// the asset is missing or invalid) and then the platform is notified.
pub fn load_in_background(id: WallpaperId) -> std::sync::mpsc::Receiver<Option<Arc<Image>>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let source = Source::of(id);
    let spawned = std::thread::Builder::new()
        .name("lume-wallpaper".into())
        .spawn(move || {
            let _ = sender.send(source.load());
            if let Some(notify) = READY_NOTIFIER.get() {
                notify();
            }
        });
    if let Err(e) = spawned {
        log::error!("cannot start the wallpaper thread: {e}");
    }
    receiver
}

fn read_asset(name: &str) -> Option<Buffer> {
    let reader = ASSET_READER.get().copied().or_else(default_reader)?;
    reader(name)
}

/// An RGB image the renderer uploads as the background texture, and its stars.
#[derive(Debug)]
pub struct Wallpaper {
    pub width: u32,
    pub height: u32,
    /// The asset's file name (QOI, `width * height` RGB pixels).
    pub asset: &'static str,
    /// Stars to draw as crisp points over the image: `x`, `y` in a 1920x1080 picture and a
    /// brightness from 0 to 255.
    pub stars: &'static [(u16, u16, u8)],
}

impl Wallpaper {
    /// Reads the 24-bit raw RGB buffer (`width * height * 3` bytes), from the cache when it has it and
    /// otherwise by decoding the asset (and then keeping the result in the cache). `None` when the asset
    /// is missing or invalid.
    pub fn load_rgb(&self) -> Option<Buffer> {
        self.load_rgb_cached(
            CACHE_DIR.get().map(|dir| dir.as_path()),
            lume_version::VERSION,
        )
    }

    /// The cache file of this wallpaper: the asset and the version of the build in the name, so a new
    /// version of the app never reads the image of an old one.
    fn cache_file(&self, dir: &std::path::Path, version: &str) -> std::path::PathBuf {
        dir.join(format!("wallpaper-{}-{version}.rgb", self.asset))
    }

    fn load_rgb_cached(&self, cache: Option<&std::path::Path>, version: &str) -> Option<Buffer> {
        let started = std::time::Instant::now();
        let size = (self.width * self.height * 3) as usize;
        let file = cache.map(|dir| self.cache_file(dir, version));
        if let Some(rgb) = file
            .as_ref()
            .and_then(|file| Buffer::read_file(file, Some(size)).ok())
        {
            log::info!(
                "wallpaper {}: from the cache in {} ms",
                self.asset,
                started.elapsed().as_millis()
            );
            return Some(rgb);
        }
        // No cache, or a broken one (cut short by a crash, another size): made again.
        let rgb = self.decode_asset()?;
        log::info!(
            "wallpaper {}: decoded in {} ms",
            self.asset,
            started.elapsed().as_millis()
        );
        if let (Some(dir), Some(file)) = (cache, file) {
            // Only the wallpaper in use stays: the others (and the old versions) are deleted. Written
            // to a temporary name and renamed, so a reader never sees half of a file.
            if let Some(name) = file.file_name() {
                custom::store_cache(dir, &name.to_string_lossy(), &rgb);
            }
        }
        Some(rgb)
    }

    fn decode_asset(&self) -> Option<Buffer> {
        let bytes = read_asset(self.asset)?;
        let header = qoi::decode_header(&*bytes).ok()?;
        if header.channels != qoi::Channels::Rgb {
            return None;
        }
        let mut rgb = Buffer::zeroed(header.n_bytes());
        qoi::decode_to_buf(&mut rgb, &*bytes).ok()?;
        let valid = header.width == self.width
            && header.height == self.height
            && rgb.len() == (self.width * self.height * 3) as usize;
        valid.then_some(rgb)
    }

    /// Like [`Wallpaper::load_rgb`], for tests.
    pub fn decompress_rgb(&self) -> Buffer {
        self.load_rgb()
            .unwrap_or_else(|| panic!("cannot read the wallpaper asset {}", self.asset))
    }
}

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

pub const THUMBNAILS_WIDTH: u32 = 640;
pub const THUMBNAILS_HEIGHT: u32 = 144;
pub const THUMBNAILS_DEFLATED: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wallpaper_thumbnails_640x144.deflate"
));

/// Decompresses the 640x144 RGBA thumbnail atlas.
pub fn decompress_thumbnails() -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec(THUMBNAILS_DEFLATED).expect("corrupt thumbnails")
}

/// Identifiers for the 6 wallpapers in the catalog, and "Minha imagem" ([`WallpaperId::Custom`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WallpaperId {
    #[default]
    Aurora,
    MountainStars,
    MirrorLake,
    MistyForest,
    GlassFacade,
    SynthwaveGrid,
    /// The user's own image ("Minha imagem"), sent from the phone; not in [`WallpaperId::ALL`].
    Custom,
}

impl WallpaperId {
    /// All 6 wallpapers in catalog display order.
    pub const ALL: [WallpaperId; 6] = [
        WallpaperId::Aurora,
        WallpaperId::MountainStars,
        WallpaperId::MirrorLake,
        WallpaperId::MistyForest,
        WallpaperId::GlassFacade,
        WallpaperId::SynthwaveGrid,
    ];

    /// The 0-based index of this wallpaper in the catalog (0 to 5; 6 for "Minha imagem", whose
    /// thumbnail takes the atlas' free slot after the catalog's).
    pub const fn index(&self) -> usize {
        match self {
            WallpaperId::Aurora => 0,
            WallpaperId::MountainStars => 1,
            WallpaperId::MirrorLake => 2,
            WallpaperId::MistyForest => 3,
            WallpaperId::GlassFacade => 4,
            WallpaperId::SynthwaveGrid => 5,
            WallpaperId::Custom => 6,
        }
    }

    /// The string identifier stored in configuration (`config.v1`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            WallpaperId::Aurora => "aurora",
            WallpaperId::MountainStars => "mountain_stars",
            WallpaperId::MirrorLake => "mirror_lake",
            WallpaperId::MistyForest => "misty_forest",
            WallpaperId::GlassFacade => "glass_facade",
            WallpaperId::SynthwaveGrid => "synthwave_grid",
            WallpaperId::Custom => "custom",
        }
    }

    /// Parses a string identifier from configuration.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<WallpaperId> {
        match s {
            "aurora" => Some(WallpaperId::Aurora),
            "mountain_stars" => Some(WallpaperId::MountainStars),
            "mirror_lake" => Some(WallpaperId::MirrorLake),
            "misty_forest" => Some(WallpaperId::MistyForest),
            "glass_facade" => Some(WallpaperId::GlassFacade),
            "synthwave_grid" => Some(WallpaperId::SynthwaveGrid),
            "custom" => Some(WallpaperId::Custom),
            _ => None,
        }
    }

    /// The localized name string ID.
    pub const fn title_string_id(&self) -> StringId {
        match self {
            WallpaperId::Aurora => StringId::WallpaperAurora,
            WallpaperId::MountainStars => StringId::WallpaperMountainStars,
            WallpaperId::MirrorLake => StringId::WallpaperMirrorLake,
            WallpaperId::MistyForest => StringId::WallpaperMistyForest,
            WallpaperId::GlassFacade => StringId::WallpaperGlassFacade,
            WallpaperId::SynthwaveGrid => StringId::WallpaperSynthwaveGrid,
            WallpaperId::Custom => StringId::WallpaperCustom,
        }
    }

    /// UV rectangle (x, y, w, h) in the 640x144 thumbnail atlas.
    pub fn thumb_uv(self) -> crate::render::Rect {
        let i = self.index();
        let col = (i % 5) as f32;
        let row = (i / 5) as f32;
        crate::render::Rect::new(
            col * 128.0 / 640.0,
            row * 72.0 / 144.0,
            128.0 / 640.0,
            72.0 / 144.0,
        )
    }

    /// The wallpaper metadata, RGB buffer and stars. For [`WallpaperId::Custom`] only the size and the
    /// (absent) stars mean something: its picture comes through [`Source::Custom`].
    pub fn wallpaper(&self) -> &'static Wallpaper {
        match self {
            WallpaperId::Aurora => &AURORA,
            WallpaperId::MountainStars => &MOUNTAIN_STARS,
            WallpaperId::MirrorLake => &MIRROR_LAKE,
            WallpaperId::MistyForest => &MISTY_FOREST,
            WallpaperId::GlassFacade => &GLASS_FACADE,
            WallpaperId::SynthwaveGrid => &SYNTHWAVE_GRID,
            WallpaperId::Custom => &CUSTOM,
        }
    }
}

static AURORA: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "aurora-1920x1080.qoi",
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_aurora_stars.rs")),
};

static MOUNTAIN_STARS: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "mountain_stars-1920x1080.qoi",
    stars: include!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_mountain_stars_stars.rs"
    )),
};

static MIRROR_LAKE: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "mirror_lake-1920x1080.qoi",
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_mirror_lake_stars.rs")),
};

static MISTY_FOREST: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "misty_forest-1920x1080.qoi",
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_misty_forest_stars.rs")),
};

static GLASS_FACADE: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "glass_facade-1920x1080.qoi",
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_glass_facade_stars.rs")),
};

static SYNTHWAVE_GRID: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "synthwave_grid-1920x1080.qoi",
    stars: include!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_synthwave_grid_stars.rs"
    )),
};

/// "Minha imagem": a 1920x1080 picture without stars, framed from the user's source.
static CUSTOM: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    asset: "custom",
    stars: &[],
};

/// The default wallpaper (Aurora).
pub fn current() -> &'static Wallpaper {
    WallpaperId::Aurora.wallpaper()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Average luminance (0 to 255) of the rows `rows` of a wallpaper, given in a 360-row scale
    /// (the bands the contrast limits were set on), whatever the stored height.
    fn luma(w: &Wallpaper, rows: std::ops::Range<usize>) -> f32 {
        let rgb = w.decompress_rgb();
        let k = w.height as usize / 360;
        let row = w.width as usize * 3;
        let slice = &rgb[rows.start * k * row..rows.end * k * row];
        slice
            .chunks(3)
            .map(|p| 0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]))
            .sum::<f32>()
            / (slice.len() / 3) as f32
    }

    #[test]
    fn all_wallpapers_have_declared_size() {
        for id in WallpaperId::ALL {
            let w = id.wallpaper();
            assert_eq!((w.width, w.height), (1920, 1080));
            assert_eq!(w.decompress_rgb().len(), 1920 * 1080 * 3);
        }
    }

    #[test]
    #[ignore = "the wallpapers no longer carry the top and dock darkening (author's decision, 2026-10-21; see CLAUDE.md); see the wallpapers-full-hd change"]
    fn all_wallpapers_dark_top_and_bottom_for_ui() {
        for id in WallpaperId::ALL {
            let w = id.wallpaper();
            let top = luma(w, 0..30);
            let bottom = luma(w, 300..360);
            let mid = luma(w, 120..240);
            assert!(
                top < 40.0,
                "{}: top luma {top} must be < 40 for top bar contrast",
                id.as_str()
            );
            assert!(
                bottom < 25.0,
                "{}: bottom luma {bottom} must be < 25 for dock contrast",
                id.as_str()
            );
            assert!(
                mid > bottom,
                "{}: mid luma {mid} must be > bottom luma {bottom}",
                id.as_str()
            );
        }
    }

    #[test]
    fn stars_stay_in_the_upper_part_of_the_picture() {
        for id in WallpaperId::ALL {
            let w = id.wallpaper();
            for &(x, y, k) in w.stars {
                assert!(x < 1920 && y < 1080 && k > 0);
                assert!(
                    y < (1080.0 * 0.7) as u16,
                    "{}: star at ({x}, {y}) is in bottom 30%",
                    id.as_str()
                );
            }
        }
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("lume-wallpaper-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cache_files(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn the_cache_misses_then_hits_and_keeps_the_same_image() {
        let dir = temp_dir("hit");
        let w = WallpaperId::Aurora.wallpaper();
        assert!(cache_files(&dir).is_empty());
        let first = w.load_rgb_cached(Some(&dir), "1.0").unwrap();
        assert_eq!(cache_files(&dir).len(), 1, "the decoded image is kept");
        let second = w.load_rgb_cached(Some(&dir), "1.0").unwrap();
        assert!(first == second, "the cache returns the decoded image");
        assert_eq!(first, w.decode_asset().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn choosing_another_wallpaper_or_version_leaves_only_the_active_file() {
        let dir = temp_dir("swap");
        WallpaperId::Aurora
            .wallpaper()
            .load_rgb_cached(Some(&dir), "1.0")
            .unwrap();
        WallpaperId::MirrorLake
            .wallpaper()
            .load_rgb_cached(Some(&dir), "1.0")
            .unwrap();
        assert_eq!(
            cache_files(&dir),
            ["wallpaper-mirror_lake-1920x1080.qoi-1.0.rgb"]
        );
        WallpaperId::MirrorLake
            .wallpaper()
            .load_rgb_cached(Some(&dir), "1.1")
            .unwrap();
        assert_eq!(
            cache_files(&dir),
            ["wallpaper-mirror_lake-1920x1080.qoi-1.1.rgb"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_or_missing_cache_is_made_again_and_no_cache_dir_still_works() {
        let dir = temp_dir("broken");
        let w = WallpaperId::GlassFacade.wallpaper();
        std::fs::write(w.cache_file(&dir, "1.0"), b"cut short").unwrap();
        let rgb = w.load_rgb_cached(Some(&dir), "1.0").unwrap();
        assert_eq!(rgb.len(), 1920 * 1080 * 3);
        assert_eq!(
            std::fs::read(w.cache_file(&dir, "1.0")).unwrap().len(),
            rgb.len()
        );
        // The system cleared the cache: the directory itself is gone.
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            w.load_rgb_cached(Some(&dir), "1.0").unwrap().len(),
            rgb.len()
        );
        assert_eq!(w.load_rgb_cached(None, "1.0").unwrap().len(), rgb.len());
    }

    #[test]
    fn wallpaper_id_string_round_trip() {
        for id in WallpaperId::ALL {
            let s = id.as_str();
            assert_eq!(WallpaperId::from_str(s), Some(id));
        }
        assert_eq!(WallpaperId::from_str("unknown"), None);
    }

    #[test]
    fn custom_has_its_own_identifier_and_the_catalog_keeps_its_six() {
        assert_eq!(WallpaperId::from_str("custom"), Some(WallpaperId::Custom));
        assert_eq!(WallpaperId::Custom.as_str(), "custom");
        assert!(!WallpaperId::ALL.contains(&WallpaperId::Custom));
        let names: Vec<&str> = WallpaperId::ALL.iter().map(|id| id.as_str()).collect();
        assert_eq!(
            names,
            [
                "aurora",
                "mountain_stars",
                "mirror_lake",
                "misty_forest",
                "glass_facade",
                "synthwave_grid"
            ]
        );
        // Its thumbnail goes to the atlas' first free slot.
        let uv = WallpaperId::Custom.thumb_uv();
        assert_eq!((uv.x, uv.y), (128.0 / 640.0, 0.5));
    }
}
