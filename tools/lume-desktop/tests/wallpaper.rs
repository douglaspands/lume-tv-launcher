//! The wallpaper as the real OpenGL ES 2.0 backend draws it. Tests are skipped when no EGL
//! implementation is available.

use lume_core::app::Painter;
use lume_desktop::screenshot;
use std::path::PathBuf;

fn wallpaper_frame(width: u32, height: u32) -> Option<Vec<u8>> {
    let result = screenshot::render(width, height, |scene| {
        let mut painter = Painter {
            list: scene.list,
            text: scene.text,
            display: *scene.display,
        };
        painter.wallpaper(false);
    });
    match result {
        Ok(pixels) => Some(pixels),
        Err(e) if e.contains("EGL") || e.contains("GL context") => {
            eprintln!("skipped: {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn decode(path: &PathBuf) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buffer).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgb);
    (info.width, info.height, buffer)
}

fn luma(rgba: &[u8], width: u32, x: u32, y: u32) -> f32 {
    let i = ((y * width + x) * 4) as usize;
    0.2126 * f32::from(rgba[i]) + 0.7152 * f32::from(rgba[i + 1]) + 0.0722 * f32::from(rgba[i + 2])
}

#[test]
fn at_1080p_it_matches_the_project_reference_within_2_in_255() {
    let Some(pixels) = wallpaper_frame(1920, 1080) else {
        return;
    };
    let reference = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mockup/wallpaper-aurora-lume@1080p.png");
    let (w, h, rgb) = decode(&reference);
    assert_eq!((w, h), (1920, 1080));
    let mut total = 0u64;
    for p in 0..(1920 * 1080) as usize {
        for c in 0..3 {
            total += u64::from(pixels[p * 4 + c].abs_diff(rgb[p * 3 + c]));
        }
    }
    let mean = total as f64 / (1920.0 * 1080.0 * 3.0);
    assert!(mean <= 2.0, "mean absolute difference {mean:.2}/255");
}

/// The bright stars that have no other star close by, so their surroundings are plain sky.
fn lonely_bright_stars() -> Vec<(u16, u16)> {
    let stars = lume_core::wallpaper::current().stars;
    stars
        .iter()
        .filter(|&&(x, y, k)| {
            k > 204
                && stars.iter().all(|&(ox, oy, _)| {
                    (ox, oy) == (x, y) || (ox.abs_diff(x) > 8 || oy.abs_diff(y) > 8)
                })
        })
        .map(|&(x, y, _)| (x, y))
        .collect()
}

#[test]
fn stars_are_sharp_points_of_the_render_buffer_at_720p_and_1080p() {
    for (w, h) in [(1280u32, 720u32), (1920, 1080)] {
        let Some(pixels) = wallpaper_frame(w, h) else {
            return;
        };
        let stars = lonely_bright_stars();
        assert!(stars.len() > 10, "{} lonely bright stars", stars.len());
        let (mut sharp, mut checked) = (0, 0);
        for (sx, sy) in stars.into_iter().take(40) {
            let (x, y) = (
                (f32::from(sx) * w as f32 / 1920.0).round() as u32,
                (f32::from(sy) * h as f32 / 1080.0).round() as u32,
            );
            if x + 6 >= w || y + 6 >= h {
                continue;
            }
            checked += 1;
            let point = luma(&pixels, w, x, y);
            // Two pixels away, on both sides of the point, the sky is back: no soft halo.
            let around = luma(&pixels, w, x + 4, y).max(luma(&pixels, w, x, y + 4));
            if point - around > 50.0 {
                sharp += 1;
            }
        }
        assert!(checked > 10);
        assert!(
            sharp * 10 >= checked * 9,
            "{w}x{h}: only {sharp} of {checked} stars are sharp"
        );
    }
}

/// The dimmed wallpaper of the text screens, rendered as it is and without its dither.
fn dimmed_with_and_without_dither(width: u32, height: u32) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut session = match screenshot::Session::new(width, height) {
        Ok(session) => session,
        Err(e) if e.contains("EGL") || e.contains("GL context") => {
            eprintln!("skipped: {e}");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    session.build(|scene| {
        let mut painter = Painter {
            list: scene.list,
            text: scene.text,
            display: *scene.display,
        };
        painter.wallpaper(true);
    });
    let dithered = session.render();
    session.set_wallpaper_dither(false);
    let plain = session.render();
    session.finish();
    Some((dithered, plain))
}

#[test]
fn the_dither_breaks_the_bands_without_changing_the_average_color() {
    for (width, height) in [(1280, 720), (1920, 1080)] {
        let Some((dithered, plain)) = dimmed_with_and_without_dither(width, height) else {
            return;
        };
        assert_ne!(
            dithered, plain,
            "{width}x{height}: the wallpaper is dithered"
        );
        let mut worst = 0.0f64;
        for top in (0..height - 15).step_by(16) {
            for left in (0..width - 15).step_by(16) {
                for c in 0..3 {
                    let (mut a, mut b) = (0u32, 0u32);
                    for y in top..top + 16 {
                        for x in left..left + 16 {
                            let i = ((y * width + x) * 4) as usize + c;
                            a += u32::from(dithered[i]);
                            b += u32::from(plain[i]);
                        }
                    }
                    worst = worst.max((f64::from(a) - f64::from(b)).abs() / 256.0);
                }
            }
        }
        assert!(
            worst <= 1.0,
            "{width}x{height}: a 16x16 region changed {worst:.2}/255 on average"
        );
    }
}
