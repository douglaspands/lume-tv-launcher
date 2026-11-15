//! The QR code of the Receive image screen: drawn as rectangles in the display list (no texture),
//! each module a whole number of render-buffer pixels with no edge ramp, centered in a white square.

use crate::display::Display;
use crate::render::{DisplayList, Rect};
use crate::theme::Color;
use qrcodegen::{QrCode, QrCodeEcc};

/// Modules of the quiet zone around the code (the standard asks for 4).
pub const QUIET_ZONE: usize = 4;

/// The modules of a QR code: `size` x `size`, dark ones `true`, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modules {
    pub size: usize,
    pub dark: Vec<bool>,
}

/// Encodes `text` with error correction M and the smallest version that holds it.
pub fn encode(text: &str) -> Option<Modules> {
    let code = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let size = code.size() as usize;
    let dark = (0..size * size)
        .map(|i| code.get_module((i % size) as i32, (i / size) as i32))
        .collect();
    Some(Modules { size, dark })
}

/// Where the modules go inside `square` (logical units): `(left, top, module)` in render-buffer pixels,
/// the module as large as a whole pixel count allows with the quiet zone around.
pub fn placement(display: &Display, square: Rect, size: usize) -> (f32, f32, u32) {
    let side = (square.w.min(square.h) * display.scale).floor();
    let module = ((side / (size + 2 * QUIET_ZONE) as f32).floor() as u32).max(1);
    let code = (module as usize * size) as f32;
    let left = (display.offset_x + square.x * display.scale + (side - code) / 2.0).round();
    let top = (display.offset_y + square.y * display.scale + (side - code) / 2.0).round();
    (left, top, module)
}

/// Draws the dark modules in `color` over the white square already drawn at `square`. A row's
/// neighboring dark modules are one rectangle.
pub fn draw(
    list: &mut DisplayList,
    display: &Display,
    square: Rect,
    modules: &Modules,
    color: Color,
) {
    let (left, top, module) = placement(display, square, modules.size);
    let to_logical = |x: f32, y: f32| {
        (
            (x - display.offset_x) / display.scale,
            (y - display.offset_y) / display.scale,
        )
    };
    let m = module as f32;
    for row in 0..modules.size {
        let mut column = 0;
        while column < modules.size {
            if !modules.dark[row * modules.size + column] {
                column += 1;
                continue;
            }
            let start = column;
            while column < modules.size && modules.dark[row * modules.size + column] {
                column += 1;
            }
            let (x, y) = to_logical(left + start as f32 * m, top + row as f32 * m);
            let w = (column - start) as f32 * m / display.scale;
            list.fill_crisp(Rect::new(x, y, w, m / display.scale), color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_are_whole_pixels_with_the_quiet_zone_at_720p_and_1080p() {
        let modules = encode("http://192.168.100.200:8642/?c=ABCD").unwrap();
        for (width, height) in [(1280, 720), (1920, 1080)] {
            let display = Display::new(width, height);
            let square = Rect::new(64.0, 152.0, 320.0, 320.0);
            let (left, top, module) = placement(&display, square, modules.size);
            let side = 320.0 * display.scale;
            let x0 = display.offset_x + 64.0 * display.scale;
            let y0 = display.offset_y + 152.0 * display.scale;
            assert_eq!(left.fract(), 0.0);
            assert_eq!(top.fract(), 0.0);
            let quiet = (left - x0)
                .min(top - y0)
                .min(x0 + side - (left + (module as usize * modules.size) as f32));
            assert!(
                quiet >= (QUIET_ZONE as u32 * module) as f32,
                "{width}: {quiet}"
            );
            assert!(module >= 4, "{width}: {module} px a module");
        }
    }
}
