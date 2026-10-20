//! What the app tiles and the dock show: one RGBA texture with two areas. The top has fixed 16:9
//! cells, one per app, for the tiles. The strip below it has square 128x128 cells for the dock's
//! icons, which only the favorites use.
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
/// Height of the area of the 16:9 cells, in pixels.
const GRID_HEIGHT: u32 = 2048;
/// Size of one dock cell, in pixels: a square icon.
pub const DOCK_CELL: u32 = 128;
/// Dock cells per row and rows (16 cells, more than the 12 favorites the dock can hold).
const DOCK_COLUMNS: u32 = 8;
const DOCK_ROWS: u32 = 2;
const DOCK_CAPACITY: usize = (DOCK_COLUMNS * DOCK_ROWS) as usize;
// The dock holds more icons than the 12 favorites there can be, and the strip is below every
// 16:9 cell.
const _: () = assert!(DOCK_CAPACITY >= crate::home::config::MAX_FAVORITES);
const _: () = assert!(ROWS * CELL_HEIGHT <= GRID_HEIGHT);
/// Size of the texture, in pixels.
pub const ATLAS_WIDTH: u32 = 1024;
pub const ATLAS_HEIGHT: u32 = GRID_HEIGHT + DOCK_ROWS * DOCK_CELL;
const COLUMNS: u32 = ATLAS_WIDTH / CELL_WIDTH;
const ROWS: u32 = GRID_HEIGHT / CELL_HEIGHT;
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
    /// Who holds each dock cell.
    dock: [Option<String>; DOCK_CAPACITY],
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

    /// The cell of `package`, by number.
    pub fn slot(&self, package: &str) -> Option<usize> {
        self.slots.get(package).map(|&(slot, _)| slot)
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

    /// Whether the app has a dock cell.
    pub fn has_dock(&self, package: &str) -> bool {
        self.dock_slot(package).is_some()
    }

    /// Reserves a dock cell for `package` (the same one if it already has it) and returns its
    /// pixel region `[x, y, width, height]`. `None` when all the dock cells are taken.
    pub fn reserve_dock(&mut self, package: &str) -> Option<[u32; 4]> {
        let slot = match self.dock_slot(package) {
            Some(slot) => slot,
            None => {
                let free = self.dock.iter().position(Option::is_none)?;
                self.dock[free] = Some(package.to_string());
                free
            }
        };
        Some(Self::dock_region(slot))
    }

    /// Gives the dock cell of `package` back, for the next favorite to use.
    pub fn release_dock(&mut self, package: &str) {
        if let Some(slot) = self.dock_slot(package) {
            self.dock[slot] = None;
        }
    }

    /// Gives back the dock cell of every app that is not in `keep`.
    pub fn release_dock_except(&mut self, keep: &[String]) {
        for holder in &mut self.dock {
            if holder.as_ref().is_some_and(|p| !keep.contains(p)) {
                *holder = None;
            }
        }
    }

    /// Where the dock cell of `package` is in the texture, in normalized coordinates.
    pub fn dock_uv(&self, package: &str) -> Option<Rect> {
        let [x, y, w, h] = Self::dock_region(self.dock_slot(package)?);
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
        self.dock = Default::default();
    }

    /// The dock cell of `package`, by number.
    pub fn dock_slot(&self, package: &str) -> Option<usize> {
        self.dock
            .iter()
            .position(|holder| holder.as_deref() == Some(package))
    }

    fn dock_region(slot: usize) -> [u32; 4] {
        let (column, row) = (slot as u32 % DOCK_COLUMNS, slot as u32 / DOCK_COLUMNS);
        [
            column * DOCK_CELL,
            GRID_HEIGHT + row * DOCK_CELL,
            DOCK_CELL,
            DOCK_CELL,
        ]
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
    fn the_atlas_grew_by_the_dock_strip() {
        assert_eq!((ATLAS_WIDTH, ATLAS_HEIGHT), (1024, 2304));
    }

    #[test]
    fn dock_cells_are_reserved_found_and_released() {
        let mut icons = AppIcons::default();
        let a = icons.reserve_dock("a").unwrap();
        let b = icons.reserve_dock("b").unwrap();
        assert_eq!(a, [0, GRID_HEIGHT, DOCK_CELL, DOCK_CELL]);
        assert_eq!(b, [DOCK_CELL, GRID_HEIGHT, DOCK_CELL, DOCK_CELL]);
        assert_eq!(icons.reserve_dock("a"), Some(a), "the same cell again");
        assert!(icons.has_dock("a") && !icons.has_dock("c"));
        // A released cell goes to the next app.
        icons.release_dock("a");
        assert!(!icons.has_dock("a") && icons.dock_uv("a").is_none());
        assert_eq!(icons.reserve_dock("c"), Some(a));
        // The grid cells are separate from the dock cells.
        assert!(!icons.has("a") && icons.reserve("a", Art::Icon).is_some());
    }

    #[test]
    fn the_dock_holds_at_least_twelve_icons_and_then_refuses() {
        let mut icons = AppIcons::default();
        for i in 0..DOCK_CAPACITY {
            assert!(icons.reserve_dock(&format!("app{i}")).is_some());
        }
        assert!(icons.reserve_dock("one more").is_none());
        icons.release_dock("app3");
        assert!(icons.reserve_dock("one more").is_some());
    }

    #[test]
    fn dock_cells_fit_the_texture_and_never_overlap_each_other_or_the_grid() {
        let mut icons = AppIcons::default();
        let mut regions = Vec::new();
        for i in 0..DOCK_CAPACITY {
            let [x, y, w, h] = icons.reserve_dock(&format!("app{i}")).unwrap();
            assert!(x + w <= ATLAS_WIDTH && y + h <= ATLAS_HEIGHT && y >= GRID_HEIGHT);
            regions.push((x, y, w, h));
        }
        for (i, a) in regions.iter().enumerate() {
            for b in &regions[i + 1..] {
                assert!(
                    a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1
                );
            }
        }
    }

    #[test]
    fn dock_uv_stays_inside_its_cell_without_bleeding_into_the_neighbors() {
        let mut icons = AppIcons::default();
        for i in 0..DOCK_CAPACITY {
            icons.reserve_dock(&format!("app{i}"));
        }
        for i in 0..DOCK_CAPACITY {
            let package = format!("app{i}");
            let [x, y, w, h] = AppIcons::dock_region(icons.dock_slot(&package).unwrap());
            let uv = icons.dock_uv(&package).unwrap();
            let (aw, ah) = (ATLAS_WIDTH as f32, ATLAS_HEIGHT as f32);
            // Half a texel in on every side: linear filtering stays inside the cell.
            assert!(uv.x * aw >= x as f32 + 0.49 && uv.right() * aw <= (x + w) as f32 - 0.49);
            assert!(uv.y * ah >= y as f32 + 0.49 && uv.bottom() * ah <= (y + h) as f32 - 0.49);
        }
    }

    #[test]
    fn clear_forgets_everything() {
        let mut icons = AppIcons::default();
        icons.reserve("a", Art::Icon);
        icons.reserve_dock("a");
        icons.clear();
        assert!(!icons.has("a") && !icons.has_dock("a"));
        assert_eq!(
            icons.reserve("b", Art::Icon),
            Some([0, 0, CELL[0], CELL[1]])
        );
    }
}
