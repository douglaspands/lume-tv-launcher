//! The lume logo mark, drawn as a single signed-distance rounded rectangle with the "L" and dot
//! subtracted in the fragment shader so they are truly transparent (cut out).
//!
//! All proportions are fractions of the mark's side, measured from the design file, and read by
//! `build.rs` to generate the companion page's SVG `<mask>`.

use crate::render::{DisplayList, Rect};
use crate::theme::Color;

/// Corner radius of the square.
const CORNER: f32 = 0.30;
/// The L's vertical bar.
#[allow(dead_code)]
const STEM_X: f32 = 0.3125;
#[allow(dead_code)]
const STEM_WIDTH: f32 = 0.128;
#[allow(dead_code)]
const STEM_TOP: f32 = 0.205;
/// Bottom edge of both bars of the L.
#[allow(dead_code)]
const BOTTOM: f32 = 0.705;
/// The L's horizontal bar.
#[allow(dead_code)]
const FOOT_RIGHT: f32 = 0.708;
#[allow(dead_code)]
const FOOT_TOP: f32 = 0.577;
/// The dot.
#[allow(dead_code)]
const DOT_X: f32 = 0.646;
#[allow(dead_code)]
const DOT_Y: f32 = 0.372;
#[allow(dead_code)]
const DOT_RADIUS: f32 = 0.0744;

/// Draws the logo mark into the largest square in the top-left of `rect`: a `square` colored
/// rounded tile with the L and dot truly cut out (transparent) in a single SDF quad.
pub fn draw_mark(list: &mut DisplayList, rect: Rect, square: Color) {
    let size = rect.w.min(rect.h);
    list.logo_mark(
        Rect::new(rect.x, rect.y, size, size),
        CORNER * size,
        size,
        square,
    );
}

/// Draws the L and dot truly cut out (transparent) centered inside an arbitrary rounded `rect`
/// (such as the 16:9 Lume tile in the Apps grid), with the cutout scaled to `mark_size`.
pub fn draw_mark_in_rect(
    list: &mut DisplayList,
    rect: Rect,
    radius: f32,
    mark_size: f32,
    color: Color,
) {
    list.logo_mark(rect, radius, mark_size, color);
}

/// The numbers of the phone page's SVG logo (a 100x100 box), as `build.rs` writes them from these
/// constants: the page's logo is this one.
#[cfg(test)]
pub(crate) fn svg_numbers() -> Vec<String> {
    let k = |v: f32| ((v * 100.0 * 1000.0).round() / 1000.0).to_string();
    let foot = BOTTOM - FOOT_TOP;
    vec![
        k(CORNER),
        k(STEM_X),
        k(STEM_TOP),
        k(STEM_WIDTH),
        k(BOTTOM - STEM_TOP),
        k(STEM_WIDTH / 2.0),
        k(FOOT_TOP),
        k(FOOT_RIGHT - STEM_X),
        k(foot),
        k(foot / 2.0),
        k(DOT_X),
        k(DOT_Y),
        k(DOT_RADIUS),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Cmd, RectKind};

    #[test]
    fn the_mark_is_one_sdf_quad_scaled_with_its_size() {
        let blue = Color::rgb(77, 179, 255);
        let mut small = DisplayList::new();
        draw_mark(&mut small, Rect::new(0.0, 0.0, 40.0, 40.0), blue);
        let mut large = DisplayList::new();
        draw_mark(&mut large, Rect::new(0.0, 0.0, 168.0, 168.0), blue);
        assert_eq!(small.len(), 1);
        assert_eq!(large.len(), 1);
        let inspect = |list: &DisplayList| match list.cmds()[0] {
            Cmd::Rect(r) => match r.kind {
                RectKind::LogoMark { color, mark_size } => {
                    assert_eq!(color, blue);
                    (r.radius, mark_size)
                }
                _ => panic!("expected LogoMark"),
            },
            _ => panic!(),
        };
        // The topbar logo: a 40 px square with 12 px corners.
        let (r_small, s_small) = inspect(&small);
        let (r_large, s_large) = inspect(&large);
        assert!((r_small - 12.0).abs() < 1e-4);
        assert!((s_small - 40.0).abs() < 1e-4);
        assert!((r_large - 50.4).abs() < 1e-3);
        assert!((s_large - 168.0).abs() < 1e-4);
    }

    #[test]
    fn a_non_square_rect_draws_the_largest_square_in_its_corner() {
        let mut list = DisplayList::new();
        draw_mark(&mut list, Rect::new(10.0, 10.0, 100.0, 40.0), Color::WHITE);
        let Cmd::Rect(square) = list.cmds()[0] else {
            panic!()
        };
        assert_eq!((square.rect.w, square.rect.h), (40.0, 40.0));
        assert!(matches!(
            square.kind,
            RectKind::LogoMark { color, mark_size }
                if color == Color::WHITE && (mark_size - 40.0).abs() < 1e-4
        ));
    }

    #[test]
    fn draw_mark_in_rect_supports_wide_tile_with_centered_cutout() {
        let mut list = DisplayList::new();
        let tile = Rect::new(64.0, 144.0, 172.0, 96.75);
        draw_mark_in_rect(&mut list, tile, 12.0, tile.h * 0.72, Color::WHITE);
        assert_eq!(list.len(), 1);
        let Cmd::Rect(cmd) = list.cmds()[0] else {
            panic!()
        };
        assert_eq!(cmd.rect, tile);
        assert_eq!(cmd.radius, 12.0);
        assert!(matches!(
            cmd.kind,
            RectKind::LogoMark { color, mark_size }
                if color == Color::WHITE && (mark_size - tile.h * 0.72).abs() < 1e-4
        ));
    }
}
