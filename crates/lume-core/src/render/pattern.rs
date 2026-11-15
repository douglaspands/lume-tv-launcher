//! Test patterns of the Tools, drawn in device pixels outside the logical canvas (see D6 in the
//! `ferramentas` design): no scaling and no smoothing, so a 1 px line is one pixel of the panel.
//!
//! The renderer runs these operations right after clearing the frame and before the batches of the
//! display list, so the text of a `.toast` drawn on the canvas lands on top of the pattern.

use crate::theme::Color;

/// A rectangle in device pixels of the render buffer, origin at the top left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PixelRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    /// The GL scissor box of the rectangle in a buffer `height` pixels tall: GL counts rows from
    /// the bottom.
    pub const fn scissor(&self, height: i32) -> [i32; 4] {
        [self.x, height - self.y - self.h, self.w, self.h]
    }
}

/// One drawing operation of a test pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternOp {
    /// Clears the whole buffer with the color (`glClear`).
    Fill(Color),
    /// A solid rectangle (`glScissor` + `glClear`): lines of 1 px are rectangles 1 px thick.
    Rect(PixelRect, Color),
    /// A gradient with a color at each corner (top left, top right, bottom right, bottom left),
    /// dithered so it shows no bands. The quad is two triangles, so it is exact when the colors
    /// change along one axis only (the ramps).
    Gradient(PixelRect, [Color; 4]),
    /// A line 1 px wide, not smoothed, between the centers of two pixels (`GL_LINES`): for the
    /// slanted lines, which a rectangle cannot draw.
    Line {
        from: (i32, i32),
        to: (i32, i32),
        color: Color,
    },
    /// A circle 1 px wide, not smoothed (`GL_LINE_LOOP` of short segments), centered on a pixel.
    Circle {
        center: (i32, i32),
        radius: i32,
        color: Color,
    },
}

/// The points of the loop that draws a circle of `radius` px: one segment every 4 px of the
/// circumference (at least 32), so it stays round without costing a vertex per pixel.
pub fn circle_segments(radius: i32) -> usize {
    let circumference = 2.0 * std::f32::consts::PI * radius.max(1) as f32;
    ((circumference / 4.0).ceil() as usize).clamp(32, 2048)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scissor_counts_rows_from_the_bottom() {
        // A line on the first row at the top is the last row for GL.
        assert_eq!(
            PixelRect::new(0, 0, 1920, 1).scissor(1080),
            [0, 1079, 1920, 1]
        );
        assert_eq!(PixelRect::new(10, 1079, 1, 1).scissor(1080), [10, 0, 1, 1]);
        assert_eq!(
            PixelRect::new(5, 100, 64, 64).scissor(720),
            [5, 556, 64, 64]
        );
    }

    #[test]
    fn a_circle_has_enough_segments_to_look_round() {
        assert_eq!(circle_segments(1), 32);
        // A 324 px radius (30% of 1080) gets a segment every 4 px or less.
        let n = circle_segments(324);
        assert!(2.0 * std::f32::consts::PI * 324.0 / n as f32 <= 4.0);
        assert!(circle_segments(100_000) <= 2048);
    }
}
