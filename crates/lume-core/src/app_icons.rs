//! What the app tiles show: one RGBA texture divided into fixed 16:9 cells, one cell per app.
//!
//! A cell holds the app's TV banner, which fills the whole tile, or, when the app has none, its
//! icon centered on a transparent background, so the tile color shows through. The platform
//! fetches the pixels and uploads them to the cell reserved here. Nothing is kept on the CPU
//! side, so everything goes away with the GPU context when lume goes dormant, and the cells are
//! filled again at the next wake.

use crate::render::Rect;
use std::collections::HashMap;

/// Size of one cell, in pixels: the 16:9 of a TV banner (320x180) and of a tile.
pub const CELL_WIDTH: u32 = 256;
pub const CELL_HEIGHT: u32 = 144;
/// Size of the texture, in pixels.
pub const ATLAS_WIDTH: u32 = 1024;
pub const ATLAS_HEIGHT: u32 = 2048;
const COLUMNS: u32 = ATLAS_WIDTH / CELL_WIDTH;
const ROWS: u32 = ATLAS_HEIGHT / CELL_HEIGHT;
const CAPACITY: usize = (COLUMNS * ROWS) as usize;

/// What a cell holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// The app's TV banner, already with the tile's rounded corners. It covers the tile.
    Banner,
    /// The app's icon, centered, over transparency.
    Icon,
}

#[derive(Default)]
pub struct AppIcons {
    slots: HashMap<String, (usize, Art)>,
}

impl AppIcons {
    /// Whether the app has a cell (or has no room for one).
    pub fn has(&self, package: &str) -> bool {
        self.slots.contains_key(package)
    }

    /// Whether there is still a free cell.
    pub fn has_room(&self) -> bool {
        self.slots.len() < CAPACITY
    }

    /// Reserves the cell of `package` and returns its pixel region `[x, y, width, height]`.
    /// `None` when the texture is full.
    pub fn reserve(&mut self, package: &str, art: Art) -> Option<[u32; 4]> {
        if let Some(&(slot, _)) = self.slots.get(package) {
            self.slots.insert(package.to_string(), (slot, art));
            return Some(Self::region(slot));
        }
        if !self.has_room() {
            return None;
        }
        let slot = self.slots.len();
        self.slots.insert(package.to_string(), (slot, art));
        Some(Self::region(slot))
    }

    /// What the cell of `package` holds.
    pub fn art(&self, package: &str) -> Option<Art> {
        self.slots.get(package).map(|&(_, art)| art)
    }

    /// Where the cell is in the texture, in normalized coordinates.
    pub fn uv(&self, package: &str) -> Option<Rect> {
        let [x, y, w, h] = Self::region(self.slots.get(package)?.0);
        // Half a pixel in, so linear filtering does not read the neighboring cell.
        let (width, height) = (ATLAS_WIDTH as f32, ATLAS_HEIGHT as f32);
        Some(Rect::new(
            (x as f32 + 0.5) / width,
            (y as f32 + 0.5) / height,
            (w as f32 - 1.0) / width,
            (h as f32 - 1.0) / height,
        ))
    }

    /// Forgets every cell: the texture is gone.
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    fn region(slot: usize) -> [u32; 4] {
        let (column, row) = (slot as u32 % COLUMNS, slot as u32 / COLUMNS);
        [
            column * CELL_WIDTH,
            row * CELL_HEIGHT,
            CELL_WIDTH,
            CELL_HEIGHT,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: [u32; 2] = [CELL_WIDTH, CELL_HEIGHT];

    #[test]
    fn each_app_gets_its_own_cell_and_keeps_it() {
        let mut icons = AppIcons::default();
        let first = icons.reserve("a", Art::Banner).unwrap();
        let second = icons.reserve("b", Art::Icon).unwrap();
        assert_eq!(first, [0, 0, CELL[0], CELL[1]]);
        assert_eq!(second, [CELL_WIDTH, 0, CELL[0], CELL[1]]);
        assert_eq!(icons.reserve("a", Art::Banner), Some(first));
        assert!(icons.has("a") && !icons.has("c"));
        assert_eq!(icons.art("a"), Some(Art::Banner));
        assert_eq!(icons.art("b"), Some(Art::Icon));
    }

    #[test]
    fn cells_wrap_to_the_next_row() {
        let mut icons = AppIcons::default();
        for i in 0..COLUMNS {
            icons.reserve(&format!("app{i}"), Art::Icon);
        }
        assert_eq!(
            icons.reserve("next", Art::Icon),
            Some([0, CELL_HEIGHT, CELL[0], CELL[1]])
        );
    }

    #[test]
    fn a_full_texture_refuses_new_apps() {
        let mut icons = AppIcons::default();
        for i in 0..CAPACITY {
            assert!(icons.reserve(&format!("app{i}"), Art::Icon).is_some());
        }
        assert!(icons.reserve("one more", Art::Icon).is_none());
        assert!(!icons.has_room());
    }

    #[test]
    fn every_cell_fits_in_the_texture() {
        let mut icons = AppIcons::default();
        for i in 0..CAPACITY {
            let [x, y, w, h] = icons.reserve(&format!("app{i}"), Art::Icon).unwrap();
            assert!(x + w <= ATLAS_WIDTH && y + h <= ATLAS_HEIGHT);
        }
    }

    #[test]
    fn uv_stays_inside_the_cell() {
        let mut icons = AppIcons::default();
        icons.reserve("a", Art::Banner);
        let uv = icons.uv("a").unwrap();
        assert!(uv.x > 0.0 && uv.right() < CELL_WIDTH as f32 / ATLAS_WIDTH as f32);
        assert!(uv.y > 0.0 && uv.bottom() < CELL_HEIGHT as f32 / ATLAS_HEIGHT as f32);
        assert!(icons.uv("missing").is_none());
    }

    #[test]
    fn clear_forgets_everything() {
        let mut icons = AppIcons::default();
        icons.reserve("a", Art::Icon);
        icons.clear();
        assert!(!icons.has("a"));
        assert_eq!(
            icons.reserve("b", Art::Icon),
            Some([0, 0, CELL[0], CELL[1]])
        );
    }
}
