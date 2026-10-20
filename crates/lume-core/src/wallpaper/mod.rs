//! The wallpaper: the single place the renderer gets it from.
//!
//! It is the generated aurora: a smooth 640x360 RGB base, enlarged by the GPU's linear filter, and
//! the positions of its stars, which the renderer draws as 1 or 2 pixel points on top (so they are
//! as sharp as the interface even on a 4K screen). Both come from `tools/wallpaper/aurora06.py` and
//! are embedded by `build.rs`. A future setting that lets the user pick another wallpaper only has
//! to change what [`current`] returns.

/// An RGB image the renderer uploads as the background texture, and its stars.
#[derive(Debug)]
pub struct Wallpaper {
    pub width: u32,
    pub height: u32,
    /// `width * height` pixels of three bytes (R, G, B), top row first.
    pub rgb: &'static [u8],
    /// Stars to draw as crisp points over the image: `x`, `y` in a 1920x1080 picture and a
    /// brightness from 0 to 255.
    pub stars: &'static [(u16, u16, u8)],
}

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

static CURRENT: Wallpaper = Wallpaper {
    width: WIDTH,
    height: HEIGHT,
    rgb: include_bytes!(concat!(env!("OUT_DIR"), "/wallpaper_640x360.rgb")),
    stars: include!(concat!(env!("OUT_DIR"), "/wallpaper_stars.rs")),
};

/// The wallpaper to draw behind every screen.
pub fn current() -> &'static Wallpaper {
    &CURRENT
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Average luminance (0 to 255) of the rows `rows` of the base image.
    fn luma(rows: std::ops::Range<usize>) -> f32 {
        let rgb = current().rgb;
        let slice = &rgb[rows.start * 640 * 3..rows.end * 640 * 3];
        slice
            .chunks(3)
            .map(|p| 0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]))
            .sum::<f32>()
            / (slice.len() / 3) as f32
    }

    #[test]
    fn the_embedded_image_has_the_declared_size() {
        let w = current();
        assert_eq!((w.width, w.height), (640, 360));
        assert_eq!(w.rgb.len(), 640 * 360 * 3);
    }

    #[test]
    fn the_bottom_is_dark_for_the_dock_and_the_top_for_the_top_bar() {
        assert!(luma(300..360) < 25.0, "bottom: {}", luma(300..360));
        assert!(luma(0..30) < 40.0, "top: {}", luma(0..30));
        assert!(
            luma(120..240) > luma(300..360),
            "the aurora is brighter than the bottom"
        );
    }

    #[test]
    fn there_are_420_stars_inside_the_1080p_picture() {
        let stars = current().stars;
        assert_eq!(stars.len(), 420);
        for &(x, y, k) in stars {
            assert!(x < 1920 && y < 1080 && k > 0);
        }
    }

    #[test]
    fn the_stars_stay_in_the_upper_part_of_the_picture() {
        // The generator keeps them out of the lowest 30%, where the dock sits.
        assert!(current()
            .stars
            .iter()
            .all(|&(_, y, _)| y < (1080.0 * 0.7) as u16));
    }
}
