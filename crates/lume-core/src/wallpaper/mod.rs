//! The wallpaper: the single place the renderer gets it from.
//!
//! A catalog of 10 high definition wallpapers in raw 24-bit RGB (1920x1080) enlarged by the GPU's
//! linear filter, with sharp stars drawn as points on top when present.

use crate::i18n::StringId;

/// An RGB image the renderer uploads as the background texture, and its stars.
#[derive(Debug)]
pub struct Wallpaper {
    pub width: u32,
    pub height: u32,
    /// Deflate-compressed `width * height * 3` bytes (R, G, B), top row first.
    pub compressed_rgb: &'static [u8],
    /// Stars to draw as crisp points over the image: `x`, `y` in a 1920x1080 picture and a
    /// brightness from 0 to 255.
    pub stars: &'static [(u16, u16, u8)],
}

impl Wallpaper {
    /// Decompresses the 24-bit raw RGB buffer (`width * height * 3` bytes).
    pub fn decompress_rgb(&self) -> Vec<u8> {
        miniz_oxide::inflate::decompress_to_vec(self.compressed_rgb)
            .expect("corrupt compressed wallpaper")
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

/// Identifiers for the 10 wallpapers in the catalog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WallpaperId {
    #[default]
    Aurora,
    MistyForest,
    MidnightOcean,
    ObsidianCanyon,
    SilkWave,
    PrismaticTopo,
    MirrorLake,
    BlackDunes,
    GlacialMist,
    StardustEther,
}

impl WallpaperId {
    /// All 10 wallpapers in catalog display order.
    pub const ALL: [WallpaperId; 10] = [
        WallpaperId::Aurora,
        WallpaperId::MistyForest,
        WallpaperId::MidnightOcean,
        WallpaperId::ObsidianCanyon,
        WallpaperId::SilkWave,
        WallpaperId::PrismaticTopo,
        WallpaperId::MirrorLake,
        WallpaperId::BlackDunes,
        WallpaperId::GlacialMist,
        WallpaperId::StardustEther,
    ];

    /// The 0-based index of this wallpaper in the catalog (0 to 9).
    pub const fn index(&self) -> usize {
        match self {
            WallpaperId::Aurora => 0,
            WallpaperId::MistyForest => 1,
            WallpaperId::MidnightOcean => 2,
            WallpaperId::ObsidianCanyon => 3,
            WallpaperId::SilkWave => 4,
            WallpaperId::PrismaticTopo => 5,
            WallpaperId::MirrorLake => 6,
            WallpaperId::BlackDunes => 7,
            WallpaperId::GlacialMist => 8,
            WallpaperId::StardustEther => 9,
        }
    }

    /// The string identifier stored in configuration (`config.v1`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            WallpaperId::Aurora => "aurora",
            WallpaperId::MistyForest => "misty_forest",
            WallpaperId::MidnightOcean => "midnight_ocean",
            WallpaperId::ObsidianCanyon => "obsidian_canyon",
            WallpaperId::SilkWave => "silk_wave",
            WallpaperId::PrismaticTopo => "prismatic_topo",
            WallpaperId::MirrorLake => "mirror_lake",
            WallpaperId::BlackDunes => "black_dunes",
            WallpaperId::GlacialMist => "glacial_mist",
            WallpaperId::StardustEther => "stardust_ether",
        }
    }

    /// Parses a string identifier from configuration.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<WallpaperId> {
        match s {
            "aurora" => Some(WallpaperId::Aurora),
            "misty_forest" => Some(WallpaperId::MistyForest),
            "midnight_ocean" => Some(WallpaperId::MidnightOcean),
            "obsidian_canyon" => Some(WallpaperId::ObsidianCanyon),
            "silk_wave" => Some(WallpaperId::SilkWave),
            "prismatic_topo" => Some(WallpaperId::PrismaticTopo),
            "mirror_lake" => Some(WallpaperId::MirrorLake),
            "black_dunes" => Some(WallpaperId::BlackDunes),
            "glacial_mist" => Some(WallpaperId::GlacialMist),
            "stardust_ether" => Some(WallpaperId::StardustEther),
            _ => None,
        }
    }

    /// The localized name string ID.
    pub const fn title_string_id(&self) -> StringId {
        match self {
            WallpaperId::Aurora => StringId::WallpaperAurora,
            WallpaperId::MistyForest => StringId::WallpaperMistyForest,
            WallpaperId::MidnightOcean => StringId::WallpaperMidnightOcean,
            WallpaperId::ObsidianCanyon => StringId::WallpaperObsidianCanyon,
            WallpaperId::SilkWave => StringId::WallpaperSilkWave,
            WallpaperId::PrismaticTopo => StringId::WallpaperPrismaticTopo,
            WallpaperId::MirrorLake => StringId::WallpaperMirrorLake,
            WallpaperId::BlackDunes => StringId::WallpaperBlackDunes,
            WallpaperId::GlacialMist => StringId::WallpaperGlacialMist,
            WallpaperId::StardustEther => StringId::WallpaperStardustEther,
        }
    }

    /// UV rectangle (x, y, w, h) in the 640x144 thumbnail atlas.
    pub fn thumb_uv(self) -> crate::render::Rect {
        let i = match self {
            WallpaperId::Aurora => 0,
            WallpaperId::MistyForest => 1,
            WallpaperId::MidnightOcean => 2,
            WallpaperId::ObsidianCanyon => 3,
            WallpaperId::SilkWave => 4,
            WallpaperId::PrismaticTopo => 5,
            WallpaperId::MirrorLake => 6,
            WallpaperId::BlackDunes => 7,
            WallpaperId::GlacialMist => 8,
            WallpaperId::StardustEther => 9,
        };
        let col = (i % 5) as f32;
        let row = (i / 5) as f32;
        crate::render::Rect::new(
            col * 128.0 / 640.0,
            row * 72.0 / 144.0,
            128.0 / 640.0,
            72.0 / 144.0,
        )
    }

    /// The wallpaper metadata, RGB buffer and stars.
    pub fn wallpaper(&self) -> &'static Wallpaper {
        match self {
            WallpaperId::Aurora => &AURORA,
            WallpaperId::MistyForest => &MISTY_FOREST,
            WallpaperId::MidnightOcean => &MIDNIGHT_OCEAN,
            WallpaperId::ObsidianCanyon => &OBSIDIAN_CANYON,
            WallpaperId::SilkWave => &SILK_WAVE,
            WallpaperId::PrismaticTopo => &PRISMATIC_TOPO,
            WallpaperId::MirrorLake => &MIRROR_LAKE,
            WallpaperId::BlackDunes => &BLACK_DUNES,
            WallpaperId::GlacialMist => &GLACIAL_MIST,
            WallpaperId::StardustEther => &STARDUST_ETHER,
        }
    }
}

static AURORA: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_aurora_1920x1080.deflate"
    )),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_aurora_stars.rs")),
};

static MISTY_FOREST: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_misty_forest_1920x1080.deflate"
    )),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_misty_forest_stars.rs")),
};

static MIDNIGHT_OCEAN: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_midnight_ocean_1920x1080.deflate"
    )),
    stars: include!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_midnight_ocean_stars.rs"
    )),
};

static OBSIDIAN_CANYON: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_obsidian_canyon_1920x1080.deflate"
    )),
    stars: include!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_obsidian_canyon_stars.rs"
    )),
};

static SILK_WAVE: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_silk_wave_1920x1080.deflate"
    )),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_silk_wave_stars.rs")),
};

static PRISMATIC_TOPO: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_prismatic_topo_1920x1080.deflate"
    )),
    stars: include!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_prismatic_topo_stars.rs"
    )),
};

static MIRROR_LAKE: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_mirror_lake_1920x1080.deflate"
    )),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_mirror_lake_stars.rs")),
};

static BLACK_DUNES: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_black_dunes_1920x1080.deflate"
    )),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_black_dunes_stars.rs")),
};

static GLACIAL_MIST: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_glacial_mist_1920x1080.deflate"
    )),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_glacial_mist_stars.rs")),
};

static STARDUST_ETHER: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    compressed_rgb: include_bytes!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_stardust_ether_1920x1080.deflate"
    )),
    stars: include!(concat!(
        env!("OUT_DIR"),
        "/wallpaper_stardust_ether_stars.rs"
    )),
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
    fn all_ten_wallpapers_have_declared_size() {
        for id in WallpaperId::ALL {
            let w = id.wallpaper();
            assert_eq!((w.width, w.height), (1920, 1080));
            assert_eq!(w.decompress_rgb().len(), 1920 * 1080 * 3);
        }
    }

    #[test]
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

    #[test]
    fn wallpaper_id_string_round_trip() {
        for id in WallpaperId::ALL {
            let s = id.as_str();
            assert_eq!(WallpaperId::from_str(s), Some(id));
        }
        assert_eq!(WallpaperId::from_str("unknown"), None);
    }
}
