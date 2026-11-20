//! The Home screen: the wallpaper, the top bar, the dock with favorites, and (in tabless modes)
//! the hint for `Slide`, the unified grid peek for `Peek`, or the fixed "All apps" dock item.

use super::context_menu::Origin;
use super::painter::Painter;
use super::topbar::{BarLayout, TopBar};
use super::{apps, dock, ids, topbar, Anim, Effect, LumeApp, Tab};
use crate::display::CANVAS_WIDTH;
use crate::i18n::StringId;
use crate::render::Rect;
use crate::text::{Align, TextStyle};
use crate::theme::vars;
use crate::ui::focus::{Focusable, Row};

/// Bottom of the dock bar in `Slide` mode (`y = 568..664`), leaving room for the hint below it.
pub(super) const DOCK_BOTTOM_SLIDE: f32 = 664.0;
/// Non-focusable `v All apps` hint in `Slide` mode (`y = 676..700`, inside the safe area).
pub(super) const SLIDE_HINT_TOP: f32 = 676.0;
pub(super) const SLIDE_HINT_HEIGHT: f32 = 24.0;
const SLIDE_HINT_ICON: f32 = 16.0;
const SLIDE_HINT_GAP: f32 = 8.0;

/// Bottom of the dock bar in `Peek` mode (`y = 524..620`), exposing ~48 px of the 1st grid row.
pub(super) const DOCK_BOTTOM_PEEK: f32 = 620.0;
/// Scroll distance over which the dock softly fades and recedes in `Peek` mode before reaching
/// the top bar, avoiding an abrupt clip at `apps::CLIP_TOP`.
const PEEK_DOCK_FADE_DISTANCE: f32 = 280.0;
/// Gentle parallax factor for the dock while fading out in `Peek` mode.
const PEEK_DOCK_PARALLAX: f32 = 0.45;

/// Minimum scroll offset when focus enters the grid in `Peek` mode: moves the dock completely
/// above `apps::CLIP_TOP` (`620 - 544 = 76`), placing the title at `y = 112` and row 1 at `y = 128`.
pub(super) const PEEK_GRID_MIN_SCROLL: f32 = 544.0;

impl LumeApp {
    pub(super) fn draw_home(&mut self, p: &mut Painter) {
        if self.is_peek_mode() {
            self.draw_home_peek(p);
            return;
        }

        let items = self.dock_items();
        let bottom = if self.is_slide_mode() {
            DOCK_BOTTOM_SLIDE
        } else {
            Self::dock_bottom(p)
        };
        let center = CANVAS_WIDTH / 2.0;
        let tabs = BarLayout::new(
            p,
            &self.translator.get(StringId::TabHome),
            &self.translator.get(StringId::TabApps),
            &self.clock,
            self.config.tabs_align,
        );

        // The focus moves over the icons at rest, so a key press does not depend on how far the
        // Magnification has got.
        let rest = dock::layout_for(
            items.len(),
            0.0,
            0.0,
            center,
            bottom,
            self.config.dock_shape,
        );
        let bar = topbar::row(
            &tabs,
            Tab::Home,
            self.inputs_available,
            self.config.tabs_visible,
        );
        let mut dock_row = Row::new(
            items
                .iter()
                .zip(&rest.items)
                .map(|(item, rect)| Focusable::new(item.id, *rect))
                .collect(),
        );
        // Coming down from the top bar, from any of its items, the focus lands on the first icon.
        if let Some(first) = items.first() {
            dock_row = dock_row.with_entry_down(first.id);
        }
        self.update_focus(vec![bar, dock_row]);

        let focus = self.dock_focus_index(&items);
        self.track_dock(focus);
        let (position, amplitude) = self.dock_motion();
        let room = self.anim.get(Anim::DockRoom, 0.0, self.now_ms);
        let mut layout = dock::layout_with_room_for(
            items.len(),
            position,
            amplitude,
            center,
            bottom,
            room,
            self.config.dock_shape,
        );
        self.slide_dock(&items, &mut layout);
        self.draw_dock(p, &items, &layout);
        self.draw_dock_ghost(p);
        if room > 0.0 {
            self.draw_move_arrows(p, &layout.items, position.round() as usize, room);
        }
        for (index, rect) in layout.items.iter().enumerate() {
            self.anchor_context_menu(Origin::Dock(index), *rect);
        }
        // The bubble follows the animated position of the focus, so it glides with the icons.
        let anchor_position = if self.dock_focus.is_some() {
            position
        } else {
            self.dock_last as f32
        };
        if let Some((icon, nearest)) = dock::label_anchor(&layout.items, anchor_position) {
            if let Some(item) = items.get(nearest) {
                let label = item.label.clone();
                self.draw_dock_label(p, &label, icon, amplitude);
            }
        }
        if self.is_slide_mode() {
            self.draw_slide_hint(p);
        }

        self.fade_tab_content(p);
        let progress = |id: u32| self.focus_progress(id);
        TopBar {
            tabs: &tabs,
            active: Tab::Home,
            tabs_visible: self.config.tabs_visible,
            home_label: &self.translator.get(StringId::TabHome),
            apps_label: &self.translator.get(StringId::TabApps),
            clock: &self.clock,
            inputs: self.inputs_available,
            focus: &progress,
        }
        .draw(p);
    }

    /// Draws the non-focusable `v All apps` hint centered below the dock in `Slide` mode.
    fn draw_slide_hint(&self, p: &mut Painter) {
        let label = self.translator.get(StringId::AppsTitle);
        let style = TextStyle::new(16.0, 500);
        let text_w = p.text.measure(&label, &style);
        let total_w = SLIDE_HINT_ICON + SLIDE_HINT_GAP + text_w;
        let start_x = (CANVAS_WIDTH - total_w) / 2.0;
        let icon_rect = Rect::new(start_x, SLIDE_HINT_TOP, SLIDE_HINT_ICON, SLIDE_HINT_HEIGHT);
        p.icon(
            crate::icons::CHEVRON_DOWN,
            icon_rect,
            SLIDE_HINT_ICON,
            vars::TEXT_MUTED,
        );
        let text_rect = Rect::new(
            start_x + SLIDE_HINT_ICON + SLIDE_HINT_GAP,
            SLIDE_HINT_TOP,
            text_w + 2.0,
            SLIDE_HINT_HEIGHT,
        );
        p.text_with(&label, text_rect, Align::Start, &style, vars::TEXT_MUTED);
    }

    /// Unified Home + Apps screen for `Peek` mode (`tabs_visible = false, tabless_mode = Peek`).
    fn draw_home_peek(&mut self, p: &mut Painter) {
        let items = self.dock_items();
        let cells = self.grid_cells_at(apps::GRID_TOP_PEEK);
        let center = CANVAS_WIDTH / 2.0;
        let tabs = BarLayout::new(
            p,
            &self.translator.get(StringId::TabHome),
            &self.translator.get(StringId::TabApps),
            &self.clock,
            self.config.tabs_align,
        );

        let rest = dock::layout_for(
            items.len(),
            0.0,
            0.0,
            center,
            DOCK_BOTTOM_PEEK,
            self.config.dock_shape,
        );
        let bar = topbar::row(&tabs, Tab::Home, self.inputs_available, false);
        let mut dock_row = Row::new(
            items
                .iter()
                .zip(&rest.items)
                .map(|(item, rect)| Focusable::new(item.id, *rect))
                .collect(),
        );
        if let Some(first) = items.first() {
            dock_row = dock_row.with_entry_down(first.id);
        }
        if let Some(prev) = items.get(self.dock_last).or_else(|| items.first()) {
            dock_row = dock_row.with_entry_up(prev.id);
        }

        let mut rows = vec![bar, dock_row];
        for (index, line) in cells.chunks(apps::COLUMNS).enumerate() {
            let mut row = Row::new(line.iter().map(|c| Focusable::new(c.id, c.rect)).collect());
            if index == 0 {
                row = row.with_entry_down(line[0].id);
            }
            rows.push(row);
        }
        self.update_focus(rows);

        let focused = self.focus.current_id();
        self.grid_position = focused.and_then(|id| cells.iter().position(|c| c.id == id));
        let in_grid = focused.is_some_and(|id| id >= ids::GRID_BASE);
        if in_grid {
            let content_bottom = cells.last().map_or(apps::GRID_TOP_PEEK, |c| {
                c.rect.bottom() + apps::BOTTOM_PADDING + 40.0
            });
            let rect = focused.and_then(|id| cells.iter().find(|c| c.id == id).map(|c| c.rect));
            let view_height = (p.display.visible_rect().bottom() - apps::VIEW_TOP).max(1.0);
            let reveal_rect = rect.map(|r| Rect::new(r.x, r.y, r.w, r.h + 40.0));
            self.reveal_vertical_peek(
                reveal_rect,
                content_bottom,
                apps::VIEW_TOP,
                view_height,
                PEEK_GRID_MIN_SCROLL,
            );
        } else {
            self.revealed = None;
            self.scroll_to_peek(0.0);
        }

        let scroll_y = self.current_scroll_y();
        let focus = self.dock_focus_index(&items);
        self.track_dock(focus);
        let (position, amplitude) = self.dock_motion();
        let room = self.anim.get(Anim::DockRoom, 0.0, self.now_ms);
        let mut layout = dock::layout_with_room_for(
            items.len(),
            position,
            amplitude,
            center,
            DOCK_BOTTOM_PEEK,
            room,
            self.config.dock_shape,
        );
        self.slide_dock(&items, &mut layout);

        // Softly fade, scale and lift the dock with gentle parallax as the grid rises, so it
        // dissolves smoothly instead of being abruptly clipped at the top bar.
        let dock_t = (scroll_y / PEEK_DOCK_FADE_DISTANCE).clamp(0.0, 1.0);
        let dock_alpha = 1.0 - dock_t * dock_t * (3.0 - 2.0 * dock_t);
        let dock_shift_y = -scroll_y * PEEK_DOCK_PARALLAX;
        let dock_scale = 1.0 - 0.06 * dock_t;
        let (bar_cx, bar_cy) = layout.bar.center();
        let transform_dock = |r: Rect| {
            let scaled = if (dock_scale - 1.0).abs() > 1e-4 {
                let (cx, cy) = r.center();
                let w = r.w * dock_scale;
                let h = r.h * dock_scale;
                let ncx = bar_cx + (cx - bar_cx) * dock_scale;
                let ncy = bar_cy + (cy - bar_cy) * dock_scale;
                Rect::new(ncx - w / 2.0, ncy - h / 2.0, w, h)
            } else {
                r
            };
            scaled.translate(0.0, dock_shift_y)
        };

        let mut drawn_layout = layout.clone();
        drawn_layout.bar = transform_dock(drawn_layout.bar);
        for rect in &mut drawn_layout.items {
            *rect = transform_dock(*rect);
        }
        for (index, rect) in drawn_layout.items.iter().enumerate() {
            self.anchor_context_menu(Origin::Dock(index), *rect);
        }

        // Vertical culling: skip the dock once it has fully faded out or scrolled above the top bar.
        if dock_alpha > 0.005 && drawn_layout.bar.bottom() > apps::CLIP_TOP {
            let visible = p.display.visible_rect();
            let clip_dock = scroll_y > 0.0;
            if clip_dock {
                p.list.push_clip(Rect::new(
                    visible.x,
                    apps::CLIP_TOP,
                    visible.w,
                    visible.bottom() - apps::CLIP_TOP,
                ));
            }
            let dock_start = p.list.len();
            self.draw_dock(p, &items, &drawn_layout);
            self.draw_dock_ghost(p);
            if room > 0.0 {
                self.draw_move_arrows(p, &drawn_layout.items, position.round() as usize, room);
            }
            let anchor_position = if self.dock_focus.is_some() {
                position
            } else {
                self.dock_last as f32
            };
            if let Some((icon, nearest)) = dock::label_anchor(&drawn_layout.items, anchor_position)
            {
                if let Some(item) = items.get(nearest) {
                    let label = item.label.clone();
                    self.draw_dock_label(p, &label, icon, amplitude);
                }
            }
            if dock_alpha < 0.999 {
                p.list.fade_from(dock_start, dock_alpha);
            }
            if clip_dock {
                p.list.pop_clip();
            }
        }

        self.draw_grid_clipped(p, &cells, apps::TITLE_BASELINE_PEEK, scroll_y);

        self.fade_tab_content(p);
        let progress = |id: u32| self.focus_progress(id);
        TopBar {
            tabs: &tabs,
            active: Tab::Home,
            tabs_visible: false,
            home_label: &self.translator.get(StringId::TabHome),
            apps_label: &self.translator.get(StringId::TabApps),
            clock: &self.clock,
            inputs: self.inputs_available,
            focus: &progress,
        }
        .draw(p);
    }

    pub(super) fn activate_home(&mut self, id: u32) -> Vec<Effect> {
        match id {
            ids::DOCK_ALL_APPS => {
                self.select_tab(Tab::Apps);
                self.pending_focus = Some(super::PendingFocus::FirstContent);
                vec![]
            }
            ids::FAVORITES_ADD => {
                self.open_editor(true);
                vec![]
            }
            id if id >= ids::GRID_BASE => self.activate_apps(id),
            id if id >= ids::FAVORITE_BASE => {
                let package = self
                    .config
                    .favorites
                    .get((id - ids::FAVORITE_BASE) as usize)
                    .cloned();
                package.map(|p| self.open_app(&p)).unwrap_or_default()
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabless_home_layout_is_on_the_ruler() {
        super::super::assert_on_ruler(&[
            ("DOCK_BOTTOM_SLIDE", DOCK_BOTTOM_SLIDE),
            ("SLIDE_HINT_TOP", SLIDE_HINT_TOP),
            ("SLIDE_HINT_HEIGHT", SLIDE_HINT_HEIGHT),
            ("SLIDE_HINT_ICON", SLIDE_HINT_ICON),
            ("SLIDE_HINT_GAP", SLIDE_HINT_GAP),
            ("DOCK_BOTTOM_PEEK", DOCK_BOTTOM_PEEK),
            ("PEEK_GRID_MIN_SCROLL", PEEK_GRID_MIN_SCROLL),
        ]);
    }
}
