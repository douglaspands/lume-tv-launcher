//! The Home tab: the wallpaper, the top bar and, at the bottom, the dock with the favorites.

use super::context_menu::Origin;
use super::dock;
use super::painter::Painter;
use super::topbar::{self, BarLayout, TopBar};
use super::{ids, Anim, Effect, LumeApp, Tab};
use crate::display::CANVAS_WIDTH;
use crate::i18n::StringId;
use crate::ui::focus::{Focusable, Row};

impl LumeApp {
    pub(super) fn draw_home(&mut self, p: &mut Painter) {
        let items = self.dock_items();
        let bottom = Self::dock_bottom(p);
        let center = CANVAS_WIDTH / 2.0;
        let tabs = BarLayout::new(
            p,
            &self.translator.get(StringId::TabHome),
            &self.translator.get(StringId::TabApps),
            &self.clock,
        );

        // The focus moves over the icons at rest, so a key press does not depend on how far the
        // Magnification has got.
        let rest = dock::layout(items.len(), 0.0, 0.0, center, bottom);
        let bar = topbar::row(&tabs, Tab::Home, self.inputs_available);
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
        let mut layout =
            dock::layout_with_room(items.len(), position, amplitude, center, bottom, room);
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

        self.fade_tab_content(p);
        let progress = |id: u32| self.focus_progress(id);
        TopBar {
            tabs: &tabs,
            active: Tab::Home,
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
            ids::FAVORITES_ADD => {
                self.open_editor(true);
                vec![]
            }
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
