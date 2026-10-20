//! The lume logo mark, drawn from signed-distance rounded rectangles.
//!
//! A rounded square holds an "L" made of two bars and a dot. All proportions are
//! fractions of the square's side, measured from the design file.

use crate::render::{DisplayList, Rect};
use crate::theme::{Color, Paint};

/// Corner radius of the square.
const CORNER: f32 = 0.30;
/// The L's vertical bar.
const STEM_X: f32 = 0.3125;
const STEM_WIDTH: f32 = 0.128;
const STEM_TOP: f32 = 0.205;
/// Bottom edge of both bars of the L.
const BOTTOM: f32 = 0.705;
/// The L's horizontal bar.
const FOOT_RIGHT: f32 = 0.708;
const FOOT_TOP: f32 = 0.577;
/// The dot.
const DOT_X: f32 = 0.646;
const DOT_Y: f32 = 0.372;
const DOT_RADIUS: f32 = 0.0744;

/// Draws the logo mark into the square `rect`: a `square` colored tile with the L and
/// the dot in `glyph` color.
pub fn draw_mark(list: &mut DisplayList, rect: Rect, square: Color, glyph: Color) {
    let size = rect.w.min(rect.h);
    let at = |fx: f32, fy: f32| (rect.x + fx * size, rect.y + fy * size);
    list.fill(
        Rect::new(rect.x, rect.y, size, size),
        CORNER * size,
        Paint::Solid(square),
    );

    let paint = Paint::Solid(glyph);
    let thickness = STEM_WIDTH * size;
    // Vertical bar of the L: rounded at the top.
    let (x, y) = at(STEM_X, STEM_TOP);
    list.fill(
        Rect::new(x, y, thickness, (BOTTOM - STEM_TOP) * size),
        thickness / 2.0,
        paint,
    );
    // Horizontal bar: rounded at both ends, which rounds the L's outer corner too.
    let foot_height = (BOTTOM - FOOT_TOP) * size;
    let (x, y) = at(STEM_X, FOOT_TOP);
    list.fill(
        Rect::new(x, y, (FOOT_RIGHT - STEM_X) * size, foot_height),
        foot_height / 2.0,
        paint,
    );
    // The dot.
    let (cx, cy) = at(DOT_X, DOT_Y);
    let r = DOT_RADIUS * size;
    list.fill(Rect::new(cx - r, cy - r, 2.0 * r, 2.0 * r), r, paint);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Cmd, RectKind};

    #[test]
    fn the_mark_is_four_shapes_scaled_with_its_size() {
        let mut small = DisplayList::new();
        draw_mark(
            &mut small,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Color::rgb(77, 179, 255),
            Color::rgb(15, 23, 30),
        );
        let mut large = DisplayList::new();
        draw_mark(
            &mut large,
            Rect::new(0.0, 0.0, 168.0, 168.0),
            Color::rgb(77, 179, 255),
            Color::rgb(15, 23, 30),
        );
        assert_eq!(small.len(), 4);
        let radius = |list: &DisplayList| match list.cmds()[0] {
            Cmd::Rect(r) => {
                assert!(matches!(r.kind, RectKind::Fill(_)));
                r.radius
            }
            _ => panic!(),
        };
        // The topbar logo: a 40 px square with 12 px corners.
        assert!((radius(&small) - 12.0).abs() < 1e-4);
        assert!((radius(&large) - 50.4).abs() < 1e-3);
    }

    #[test]
    fn a_non_square_rect_draws_the_largest_square_in_its_corner() {
        let mut list = DisplayList::new();
        draw_mark(
            &mut list,
            Rect::new(10.0, 10.0, 100.0, 40.0),
            Color::WHITE,
            Color::TRANSPARENT,
        );
        let Cmd::Rect(square) = list.cmds()[0] else {
            panic!()
        };
        assert_eq!((square.rect.w, square.rect.h), (40.0, 40.0));
    }
}
