//! The Tools menu: the six tools at the left, the description of the focused one in a `.panel` at
//! the right. The six items fit the canvas; the list would scroll if more came (it keeps the focused
//! item whole).

use super::common::{self, draw_panel, item_rect, panel_heading, wrapped_text, LIST_VIEW_TOP};
use super::ToolKind;
use crate::app::painter::Painter;
use crate::app::{settings, Effect, LumeApp, PendingFocus, Screen};
use crate::i18n::StringId;
use crate::theme::StateFlags;

/// The description panel, as tall as the statistics panel of the settings.
pub(in crate::app) const MENU_PANEL_HEIGHT: f32 = 312.0;
/// Room left under the last item when the list scrolls to it.
const SCROLL_MARGIN: f32 = 28.0;

impl LumeApp {
    pub(in crate::app) fn open_tools(&mut self) {
        self.screen = Screen::Tools;
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(ToolKind::Display.menu_id()));
        self.focus.set_rows(vec![]);
    }

    pub(in crate::app) fn back_from_tools(&mut self) {
        self.screen = Screen::Settings;
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(settings::ITEM_TOOLS));
    }

    pub(in crate::app) fn activate_tools(&mut self, id: u32) -> Vec<Effect> {
        match ToolKind::from_menu_id(id) {
            Some(kind) => self.open_tool(kind),
            None => vec![],
        }
    }

    pub(in crate::app) fn draw_tools(&mut self, p: &mut Painter) {
        let t = self.translator;
        let items: Vec<(u32, String)> = ToolKind::ALL
            .iter()
            .map(|k| (k.menu_id(), t.get(k.name()).into_owned()))
            .collect();
        let scroll = self.current_scroll_y();
        let rects = self.draw_tool_items(p, &items, scroll);

        // Keep the focused item whole on the canvas.
        let focused = self.focus.current_id();
        let index = ToolKind::ALL
            .iter()
            .position(|k| Some(k.menu_id()) == focused);
        let bottom = p.display.visible_rect().bottom();
        let content = rects.last().map_or(0.0, |r| r.bottom()) + SCROLL_MARGIN;
        self.reveal_vertical(
            index.map(item_rect),
            content,
            LIST_VIEW_TOP,
            bottom - LIST_VIEW_TOP,
        );

        // The description of the focused tool, in a panel that does not scroll.
        let kind = index.map_or(ToolKind::Display, |i| ToolKind::ALL[i]);
        let inner = draw_panel(p, common::panel_rect(MENU_PANEL_HEIGHT));
        panel_heading(p, inner, &t.get(kind.name()));
        let body = p.style(|t| &t.breadcrumb, StateFlags::NONE);
        let text = crate::render::Rect::new(inner.x, inner.y + 48.0, inner.w, body.line_height);
        wrapped_text(p, &t.get(kind.description()), text, body.line_height, &body);

        crate::app::page::draw_header(
            p,
            &[
                &t.get(StringId::SettingsTitle),
                &t.get(StringId::SettingsTools),
            ],
            &t.get(StringId::SettingsTools),
            None,
            0.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("MENU_PANEL_HEIGHT", MENU_PANEL_HEIGHT),
            ("SCROLL_MARGIN", SCROLL_MARGIN),
        ]);
    }

    #[test]
    fn the_items_start_36_below_the_title_baseline_and_all_fit() {
        // As the editor: a secondary screen without subtitle starts its content at y=152.
        assert_eq!(item_rect(0).y, 152.0);
        let last = item_rect(ToolKind::ALL.len() - 1);
        assert_eq!(last.bottom(), 664.0);
    }
}
