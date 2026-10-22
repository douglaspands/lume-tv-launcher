//! The Apps tab: every installed app in a grid.

use super::context_menu::Origin;
use super::painter::Painter;
use super::topbar::{self, BarLayout, TopBar};
use super::{ids, Effect, LumeApp, Tab};
use crate::i18n::StringId;
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{vars, Paint, StateFlags, THEME};
use crate::ui::focus::{Focusable, Row};

const COLUMNS: usize = 6;
const WIDTH: f32 = 1280.0 - 2.0 * topbar::MARGIN;
const GAP: f32 = 24.0;
pub(super) const TILE_WIDTH: f32 = (WIDTH - (COLUMNS as f32 - 1.0) * GAP) / COLUMNS as f32;
const TILE_HEIGHT: f32 = TILE_WIDTH * 9.0 / 16.0;
/// Distance between the tops of two grid rows: the tile and its name below it.
const ROW_PITCH: f32 = 148.0;
const TITLE_BASELINE: f32 = 116.0;
const GRID_TOP: f32 = 144.0;
const BOTTOM_PADDING: f32 = 40.0;
const CLIP_TOP: f32 = 76.0;
const VIEW_TOP: f32 = 88.0;

/// One cell of the grid.
pub(super) struct Cell {
    pub id: u32,
    pub package: Option<String>,
    rect: Rect,
    label: String,
    color: crate::theme::Color,
}

/// The bottom of the grid's view.
pub(super) fn view_bottom(p: &Painter) -> f32 {
    p.display.visible_rect().bottom()
}

impl LumeApp {
    pub(super) fn grid_cells(&self) -> Vec<Cell> {
        let mut cells: Vec<Cell> = self
            .catalog
            .apps()
            .iter()
            .enumerate()
            .map(|(i, app)| Cell {
                id: ids::GRID_BASE + i as u32,
                package: Some(app.package.clone()),
                rect: Rect::default(),
                label: app.label.clone(),
                color: app.color,
            })
            .collect();
        // lume's own entry, with its logo, opening lume's Settings screen.
        cells.push(Cell {
            id: ids::GRID_SETTINGS,
            package: None,
            rect: Rect::default(),
            label: "Lume".into(),
            color: vars::PRIMARY,
        });
        // lume is one more app of the grid: it takes its place in the alphabetical order.
        cells.sort_by_cached_key(|c| crate::catalog::normalize::sort_key(&c.label));
        for (index, cell) in cells.iter_mut().enumerate() {
            let (row, column) = (index / COLUMNS, index % COLUMNS);
            cell.rect = Rect::new(
                topbar::MARGIN + column as f32 * (TILE_WIDTH + GAP),
                GRID_TOP + row as f32 * ROW_PITCH,
                TILE_WIDTH,
                TILE_HEIGHT,
            );
        }
        cells
    }

    pub(super) fn draw_apps(&mut self, p: &mut Painter) {
        let cells = self.grid_cells();
        let tabs = BarLayout::new(
            p,
            &self.translator.get(StringId::TabHome),
            &self.translator.get(StringId::TabApps),
            &self.clock,
        );

        let mut rows = vec![topbar::row(&tabs, Tab::Apps, self.inputs_available)];
        for (index, line) in cells.chunks(COLUMNS).enumerate() {
            let mut row = Row::new(line.iter().map(|c| Focusable::new(c.id, c.rect)).collect());
            if index == 0 {
                // Coming down from the top bar the focus starts at the first tile.
                row = row.with_entry_down(line[0].id);
            }
            rows.push(row);
        }
        self.update_focus(rows);
        let focused = self.focus.current_id();
        self.grid_position = focused.and_then(|id| cells.iter().position(|c| c.id == id));

        let content_bottom = cells
            .last()
            .map_or(GRID_TOP, |c| c.rect.bottom() + BOTTOM_PADDING + 40.0);
        let in_grid = focused.is_some_and(|id| id >= ids::GRID_BASE);
        let rect = focused.and_then(|id| cells.iter().find(|c| c.id == id).map(|c| c.rect));
        let view_height = (p.display.visible_rect().bottom() - VIEW_TOP).max(1.0);
        // Keep the label under the focused tile visible too.
        let reveal_rect = rect.map(|r| Rect::new(r.x, r.y, r.w, r.h + 40.0));
        self.reveal_vertical(
            if in_grid { reveal_rect } else { None },
            content_bottom,
            VIEW_TOP,
            view_height,
        );
        if focused.is_some_and(|id| id < ids::GRID_BASE) {
            self.scroll_to(0.0);
        }
        let scroll_y = self.current_scroll_y();
        // The menu goes next to the tile as it looks with the focus: grown, with its ring around it.
        let focused = p.focus_style(&THEME.tile, 1.0);
        let ring = focused
            .outline
            .map_or(0.0, |o| o.width + focused.outline_offset);
        for cell in &cells {
            let drawn = cell
                .rect
                .translate(0.0, -scroll_y)
                .scaled_about_center(focused.scale)
                .grow(ring);
            self.anchor_context_menu(Origin::Grid(cell.id), drawn);
        }

        let visible = p.display.visible_rect();

        p.list.push_clip(Rect::new(
            visible.x,
            CLIP_TOP,
            visible.w,
            visible.bottom() - CLIP_TOP,
        ));
        let title_style = p.style(&THEME.section_title, StateFlags::NONE);
        let title = self.translator.get(StringId::AppsTitle);
        p.text.draw(
            p.list,
            &title,
            topbar::MARGIN,
            TITLE_BASELINE - scroll_y,
            &Painter::text_style(&title_style),
            title_style.color,
        );
        let tiles: Vec<_> = cells
            .iter()
            .map(|cell| {
                let rect = cell.rect.translate(0.0, -scroll_y);
                let mut style = p.focus_style(&THEME.tile, self.focus_progress(cell.id));
                style.background = Some(Paint::Solid(cell.color));
                (cell, rect, style)
            })
            .collect();
        // Boxes, then images, then text: one draw call for each kind (see `draw_tile_image`).
        for (cell, rect, style) in &tiles {
            if self.tile_has_image(cell.package.as_deref(), false) {
                // The image pass draws the tile color with the image, under one edge, and the
                // outline runs under that edge until the image is opaque: the image is clipped
                // half a ramp in and fades over one more, and the outline has its own half ramp,
                // so two ramps (pixels of the buffer, in canvas units).
                let mut style = *style;
                style.background = None;
                let under = 2.0 * vars::EDGE_SOFTNESS / p.display.scale;
                p.list.draw_box_under(*rect, &style, under);
            } else {
                p.draw_box(*rect, style);
            }
        }
        for (cell, rect, style) in &tiles {
            self.draw_tile_image(p, cell.package.as_deref(), *rect, style, false);
        }
        for (cell, rect, style) in &tiles {
            if cell.package.is_none() {
                // lume's own tile: the whole tile is the logo color, with the L centered. The mark's
                // square has the tile's color, so only the L shows and the tile stays a rectangle.
                let tile = rect.scaled_about_center(style.scale);
                let side = tile.h * 0.72;
                let mark = Rect::new(
                    tile.x + (tile.w - side) / 2.0,
                    tile.y + (tile.h - side) / 2.0,
                    side,
                    side,
                );
                crate::logo::draw_mark(p.list, mark, vars::PRIMARY, vars::BG);
            } else {
                self.draw_tile_name(p, cell.package.as_deref(), &cell.label, *rect, style, false);
            }
            let label_style = p.focus_style(&THEME.tile_label, self.focus_progress(cell.id));
            p.text(
                &cell.label,
                Rect::new(rect.x, rect.bottom() + 8.0, rect.w, 24.0),
                Align::Center,
                &label_style,
            );
        }
        p.list.pop_clip();

        self.fade_tab_content(p);
        let progress = |id: u32| self.focus_progress(id);
        TopBar {
            tabs: &tabs,
            active: Tab::Apps,
            home_label: &self.translator.get(StringId::TabHome),
            apps_label: &self.translator.get(StringId::TabApps),
            clock: &self.clock,
            inputs: self.inputs_available,
            focus: &progress,
        }
        .draw(p);
    }

    /// The apps whose tiles are at least partly in the grid's view (from below the top bar to
    /// `bottom`) with the grid scrolled by `scroll_y`.
    pub(super) fn visible_grid_packages(&self, bottom: f32, scroll_y: f32) -> Vec<String> {
        self.grid_cells()
            .into_iter()
            .filter(|cell| {
                let rect = cell.rect.translate(0.0, -scroll_y);
                rect.bottom() > CLIP_TOP && rect.y < bottom
            })
            .filter_map(|cell| cell.package)
            .collect()
    }

    /// The id of the tile at `position` of the grid, or of the last one when there are fewer.
    pub(super) fn grid_id_at(&self, position: usize) -> Option<u32> {
        let cells = self.grid_cells();
        cells.get(position).or(cells.last()).map(|c| c.id)
    }

    pub(super) fn activate_apps(&mut self, id: u32) -> Vec<Effect> {
        if id == ids::GRID_SETTINGS {
            self.open_settings();
            return vec![];
        }
        let Some(index) = id.checked_sub(ids::GRID_BASE) else {
            return vec![];
        };
        let package = self
            .catalog
            .apps()
            .get(index as usize)
            .map(|a| a.package.clone());
        package.map(|p| self.open_app(&p)).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_on_the_ruler() {
        // The tile height is derived from 16:9 (96.75) and is the one registered exception.
        super::super::assert_on_ruler(&[
            ("WIDTH", WIDTH),
            ("GAP", GAP),
            ("TILE_WIDTH", TILE_WIDTH),
            ("ROW_PITCH", ROW_PITCH),
            ("TITLE_BASELINE", TITLE_BASELINE),
            ("GRID_TOP", GRID_TOP),
            ("BOTTOM_PADDING", BOTTOM_PADDING),
            ("CLIP_TOP", CLIP_TOP),
            ("VIEW_TOP", VIEW_TOP),
        ]);
    }

    #[test]
    fn six_columns_fill_the_content_width_without_a_leftover() {
        let last_right = topbar::MARGIN + (COLUMNS - 1) as f32 * (TILE_WIDTH + GAP) + TILE_WIDTH;
        assert!(
            (last_right - (1280.0 - topbar::MARGIN)).abs() < 0.01,
            "{last_right}"
        );
        assert_eq!(
            COLUMNS as f32 * TILE_WIDTH + (COLUMNS - 1) as f32 * GAP,
            WIDTH
        );
    }

    #[test]
    fn tiles_are_about_172_by_97() {
        assert!((TILE_WIDTH - 172.0).abs() < 0.01, "{TILE_WIDTH}");
        assert!((TILE_HEIGHT - 96.75).abs() < 0.01, "{TILE_HEIGHT}");
        // About 18% smaller than the old 211x119 tiles.
        assert!((1.0 - TILE_WIDTH / 211.0 - 0.18).abs() < 0.02);
    }

    #[test]
    fn a_row_keeps_the_same_strip_for_the_name_as_before() {
        assert_eq!(ROW_PITCH, 148.0);
    }
}
