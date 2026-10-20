//! The Apps tab: every installed app in a grid.

use super::painter::Painter;
use super::topbar::{self, TabLayout, TopBar};
use super::{ids, Effect, LumeApp, Tab};
use crate::i18n::StringId;
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{vars, Paint, StateFlags, THEME};
use crate::ui::focus::{Focusable, Row};

const COLUMNS: usize = 5;
const WIDTH: f32 = 1280.0 - 2.0 * topbar::MARGIN;
const GAP: f32 = 24.0;
const TILE_WIDTH: f32 = (WIDTH - (COLUMNS as f32 - 1.0) * GAP) / COLUMNS as f32;
const TILE_HEIGHT: f32 = TILE_WIDTH * 9.0 / 16.0;
/// Distance between the tops of two grid rows: the tile and its name below it.
const ROW_PITCH: f32 = 170.5;
const TITLE_BASELINE: f32 = 119.0;
const GRID_TOP: f32 = 143.0;
const BOTTOM_PADDING: f32 = 40.0;
const CLIP_TOP: f32 = 76.0;
const VIEW_TOP: f32 = 88.0;

/// One cell of the grid.
struct Cell {
    id: u32,
    package: Option<String>,
    rect: Rect,
    label: String,
    color: crate::theme::Color,
}

impl LumeApp {
    fn grid_cells(&self) -> Vec<Cell> {
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
        cells.push(Cell {
            id: ids::GRID_SETTINGS,
            package: None,
            rect: Rect::default(),
            label: self.translator.get(StringId::AppsSettingsTile).into_owned(),
            color: vars::TILE_SETTINGS,
        });
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
        let tabs = TabLayout::new(
            p,
            &self.translator.get(StringId::TabHome),
            &self.translator.get(StringId::TabApps),
        );

        let mut rows = vec![topbar::row(&tabs, Tab::Apps)];
        for (index, line) in cells.chunks(COLUMNS).enumerate() {
            let mut row = Row::new(line.iter().map(|c| Focusable::new(c.id, c.rect)).collect());
            if index == 0 {
                // Coming down from the top bar the focus starts at the first tile.
                row = row.with_entry_down(line[0].id);
            }
            rows.push(row);
        }
        self.update_focus(rows);

        let content_bottom = cells
            .last()
            .map_or(GRID_TOP, |c| c.rect.bottom() + BOTTOM_PADDING + 40.0);
        let focused = self.focus.current_id();
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

        let visible = p.display.visible_rect();
        p.list.fill(
            visible,
            0.0,
            THEME.app.base.background.unwrap_or(Paint::Solid(vars::BG)),
        );

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
        for (_, rect, style) in &tiles {
            p.draw_box(*rect, style);
        }
        for (cell, rect, style) in &tiles {
            self.draw_tile_image(p, cell.package.as_deref(), *rect, style, false);
        }
        let label_style = p.style(&THEME.tile_label, StateFlags::NONE);
        for (cell, rect, style) in &tiles {
            self.draw_tile_name(p, cell.package.as_deref(), &cell.label, *rect, style, false);
            p.text(
                &cell.label,
                Rect::new(rect.x, rect.bottom() + 8.0, rect.w, 24.0),
                Align::Center,
                &label_style,
            );
        }
        p.list.pop_clip();

        let progress = |id: u32| self.focus_progress(id);
        TopBar {
            tabs: &tabs,
            active: Tab::Apps,
            home_label: &self.translator.get(StringId::TabHome),
            apps_label: &self.translator.get(StringId::TabApps),
            clock: &self.clock,
            focus: &progress,
        }
        .draw(p);
    }

    pub(super) fn activate_apps(&mut self, id: u32) -> Vec<Effect> {
        if id == ids::GRID_SETTINGS {
            return vec![Effect::OpenAndroidSettings];
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
