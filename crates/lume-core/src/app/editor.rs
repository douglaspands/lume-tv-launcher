//! The home screen editor: the favorites.

use super::page::{self, LEFT, WIDTH};
use super::painter::{fade, Painter};
use super::{Anim, Effect, LumeApp, PendingFocus, Screen, Tab};
use crate::home::config::{DockIconShape, HomeConfig, TablessMode, TabsAlign, MAX_FAVORITES};
use crate::i18n::{Arg, PluralId, StringId};
use crate::icons;
use crate::render::Rect;
use crate::text::{Align, TextStyle};
use crate::theme::{vars, AccentColor, Color, Paint, StateFlags, Theme};
use crate::ui::focus::{FocusId, Focusable, Row};
use crate::ui::style::mix;
use crate::wallpaper::WallpaperId;
use std::collections::HashMap;

pub const RESTORE: FocusId = 4001;
pub const WALLPAPER_BASE: FocusId = 4100;
/// The last mini-card: "+ From phone", or "Minha imagem" once there is one.
pub const PHONE_CARD: FocusId = WALLPAPER_BASE + WallpaperId::ALL.len() as FocusId;
/// The mini-cards: the phone's first, then the catalog.
const CARD_COUNT: usize = WallpaperId::ALL.len() + 1;
/// Focus IDs for the 10 curated accent color swatches (`AccentColor::ALL`).
pub const ACCENT_BASE: FocusId = 4200;
/// Focus IDs for the 2-level conditional navigation selector in the Appearance panel.
pub const NAV_TABS_ON: FocusId = 4300;
pub const NAV_TABS_OFF: FocusId = 4301;
pub const NAV_ALIGN_LEFT: FocusId = 4310;
pub const NAV_ALIGN_CENTER: FocusId = 4311;
pub const NAV_MODE_SLIDE: FocusId = 4320;
pub const NAV_MODE_PEEK: FocusId = 4321;
pub const NAV_MODE_DOCK: FocusId = 4322;
/// Focus IDs for the favorites dock shape selector (`Round` | `Square` | `Banner`).
pub const SHAPE_ROUND: FocusId = 4330;
pub const SHAPE_SQUARE: FocusId = 4331;
pub const SHAPE_BANNER: FocusId = 4332;

fn accent_index(color: AccentColor) -> usize {
    AccentColor::ALL
        .iter()
        .position(|&c| c == color)
        .unwrap_or(0)
}

fn accent_for_id(id: FocusId) -> Option<AccentColor> {
    let end = ACCENT_BASE + AccentColor::ALL.len() as FocusId;
    if (ACCENT_BASE..end).contains(&id) {
        Some(AccentColor::ALL[(id - ACCENT_BASE) as usize])
    } else {
        None
    }
}

/// The place of a mini-card in the row: the phone card is the first, the catalog follows.
fn card_slot(id: FocusId) -> usize {
    if id == PHONE_CARD {
        0
    } else {
        (id - WALLPAPER_BASE) as usize + 1
    }
}

/// The mini-card at a place of the row.
fn slot_card(slot: usize) -> FocusId {
    if slot == 0 {
        PHONE_CARD
    } else {
        WALLPAPER_BASE + slot as FocusId - 1
    }
}
pub(super) const FAVORITE_BUTTON_BASE: FocusId = 4400;
pub(super) const ADD_APP_BASE: FocusId = 4600;
pub const CONFIRM_RESTORE: FocusId = 4900;
pub const CONFIRM_CANCEL: FocusId = 4901;

const CHIP_HEIGHT: f32 = 40.0;
const CHIP_GAP: f32 = 16.0;
/// Space inside the favorites panel, around its content.
const PANEL_PADDING_X: f32 = 32.0;
const PANEL_PADDING_Y: f32 = 24.0;
const CARD_WIDTH: f32 = 168.0;
const CARD_NAME_HEIGHT: f32 = 56.0;
const CARD_CONTROLS_HEIGHT: f32 = 60.0;
const CARD_GAP: f32 = 16.0;
/// Where the content starts: 36 below the title's baseline at 116 (no subtitle).
const TOP: f32 = super::page::PLAIN_CONTENT_TOP;
const ACCENT_ROW_HEIGHT: f32 = 72.0;
const SWATCH_SIZE: f32 = 40.0;
const SWATCH_GAP: f32 = 12.0;
const ROW_HEIGHT: f32 = 64.0;
const EDITOR_ROW_GAP: f32 = 8.0;
const ROW_PADDING_X: f32 = 24.0;
const OPTION_CHIP_GAP: f32 = 8.0;
const ROW_GAP: f32 = 16.0;
const WALLPAPER_CARD_WIDTH: f32 = 178.0;
const WALLPAPER_CARD_HEIGHT: f32 = 100.0;
const WALLPAPER_CARD_GAP: f32 = 16.0;
const HEADING_GAP: f32 = 32.0;
/// The line under the favorites heading that says what the panel does.
const DESCRIPTION_HEIGHT: f32 = 48.0;
const CARDS_TO_CHIPS_GAP: f32 = 20.0;
const BOTTOM_PADDING: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 48.0;
/// How far below the top of the content the first line of cards reaches.
const FIRST_CARDS_DEPTH: f32 = 540.0;
const BUTTON_MIN_WIDTH: f32 = 160.0;
/// Space between the favorites panel and the buttons under it.
const BUTTONS_GAP: f32 = 24.0;

/// The buttons of the design system (see the `design-system` spec).
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) enum ButtonKind {
    /// The action of the screen.
    Primary,
    /// The quiet alternative.
    Secondary,
    /// What cannot be undone.
    Danger,
}

impl ButtonKind {
    fn component(self, theme: &Theme) -> &crate::theme::ComponentTheme {
        match self {
            ButtonKind::Primary => &theme.button,
            ButtonKind::Secondary => &theme.button_secondary,
            ButtonKind::Danger => &theme.button_danger,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct EditorState {
    /// The "Restore defaults" confirmation is open.
    pub confirm: bool,
    /// It was opened from the Home screen ("Add favorites"), not from the settings: Back goes
    /// home again instead of to the settings.
    pub from_home: bool,
    /// Current rendered horizontal scroll offset for wallpaper mini-cards.
    pub wallpaper_scroll_x: f32,
    /// Target horizontal scroll offset for wallpaper mini-cards.
    pub target_wallpaper_scroll_x: f32,
    /// The open confirmation is the one of removing "Minha imagem" (not of restoring the defaults).
    pub remove_image: bool,
}

impl LumeApp {
    pub(super) fn open_editor(&mut self, from_home: bool) {
        self.screen = Screen::Editor;
        let wp_idx = self.config.wallpaper.index() as FocusId;
        self.editor = EditorState {
            from_home,
            ..EditorState::default()
        };
        self.set_preview_accent(None);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(WALLPAPER_BASE + wp_idx));
        self.focus.set_rows(vec![]);
    }

    /// Keeps `preview_accent` synchronized with the currently focused swatch in the editor, and
    /// reverts to the saved accent color whenever focus leaves the swatch row or the editor.
    pub(super) fn sync_editor_preview_accent(&mut self) {
        let preview = if self.screen == Screen::Editor && !self.editor.confirm {
            self.focus
                .current_id()
                .and_then(accent_for_id)
                .filter(|&accent| accent != self.config.accent_color)
        } else {
            None
        };
        self.set_preview_accent(preview);
    }

    /// Apps that are not favorites yet, in the catalog's order.
    pub(super) fn addable_apps(&self) -> Vec<(String, String)> {
        self.catalog
            .apps()
            .iter()
            .filter(|a| !self.config.favorites.contains(&a.package))
            .map(|a| (a.package.clone(), a.label.clone()))
            .collect()
    }

    pub(super) fn draw_editor(&mut self, p: &mut Painter) {
        // Two passes: the first only lays out (to know where the focus is and how far to
        // scroll), the second draws with the final scroll position.
        let layout = self.editor_pass(p, false, 0.0);
        self.update_focus(layout.rows);
        let focused = self.focus.current_id();
        let rect = focused.and_then(|id| layout.rects.get(&id).copied());
        let view_height = p.display.visible_rect().bottom().max(720.0);
        self.reveal_vertical(rect, layout.content_height, 0.0, view_height);
        if rect.is_some_and(|r| r.y < TOP + FIRST_CARDS_DEPTH) {
            // The first line of cards brings the whole page back to the top, with its header.
            self.scroll_to(0.0);
        }
        let scroll_y = self.current_scroll_y();
        self.editor_pass(p, true, scroll_y);
    }

    /// Lays out the whole screen. With `draw` it also paints it.
    fn editor_pass(&mut self, p: &mut Painter, draw: bool, scroll_y: f32) -> Layout {
        let mut layout = Layout::default();
        let t = self.translator;
        if draw {
            page::draw_header(
                p,
                &[
                    &t.get(StringId::SettingsTitle),
                    &t.get(StringId::SettingsHomeScreen),
                ],
                &t.get(StringId::SettingsHomeScreen),
                None,
                scroll_y,
            );
        }

        let wp_top = TOP;
        let panel_top = self.wallpaper_section(p, &mut layout, wp_top, scroll_y, draw);
        let y = self.favorites_panel(p, &mut layout, panel_top, scroll_y, draw);

        // "Restore defaults" at the left, on the line under the panel. The "Done" button is
        // removed: all changes persist immediately and Back is the standard Android TV exit.
        let y = y + BUTTONS_GAP;
        let restore_label = t.get(StringId::EditorRestoreDefaults).into_owned();
        let button_style = p.style(|t| &t.button, StateFlags::NONE);
        let width_of = |p: &mut Painter, label: &str| {
            (p.measure(label, &button_style) + 2.0 * 28.0).max(BUTTON_MIN_WIDTH)
        };
        let restore_width = width_of(p, &restore_label);
        let restore = Rect::new(LEFT, y, restore_width, BUTTON_HEIGHT);
        layout.rects.insert(RESTORE, restore);
        layout
            .rows
            .push(Row::new(vec![Focusable::new(RESTORE, restore)]).with_entry_down(RESTORE));
        let view_height = p.display.visible_rect().bottom().max(720.0);
        let restore_scrolled = restore.translate(0.0, -scroll_y);
        let is_visible = |r: Rect| r.bottom() >= -32.0 && r.y <= view_height + 32.0;
        if draw && !self.editor.confirm && is_visible(restore_scrolled) {
            p.list.fill(
                Rect::new(LEFT, y - 12.0 - scroll_y, WIDTH, 2.0),
                1.0,
                Paint::Solid(vars::SURFACE),
            );
            self.draw_button(
                p,
                restore_scrolled,
                &restore_label,
                RESTORE,
                ButtonKind::Secondary,
            );
        }
        layout.content_height = y + BUTTON_HEIGHT + BOTTOM_PADDING;

        if self.editor.confirm {
            // The dialog traps the focus.
            let dialog = self.confirm_dialog(p, draw);
            layout.rows = dialog.0;
            layout.rects.extend(dialog.1);
        }
        layout
    }

    /// The favorites: order, remove, and add.
    fn favorites_panel(
        &mut self,
        p: &mut Painter,
        layout: &mut Layout,
        top: f32,
        scroll_y: f32,
        draw: bool,
    ) -> f32 {
        let t = self.translator;
        let inner_left = LEFT + PANEL_PADDING_X;
        let inner_width = WIDTH - 2.0 * PANEL_PADDING_X;
        let per_line =
            (((inner_width + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor() as usize).max(1);
        let card_height = CARD_NAME_HEIGHT + CARD_CONTROLS_HEIGHT;
        let favorites = self.config.favorites.clone();
        let total_slots = self.config.dock_slot_count();
        let card_lines = total_slots.div_ceil(per_line);

        let heading_y = top + PANEL_PADDING_Y;

        // The heading, then a line saying what the panel is for, then the cards.
        let cards_top = heading_y + HEADING_GAP + DESCRIPTION_HEIGHT;
        let mut y = cards_top;
        if card_lines > 0 {
            y += card_lines as f32 * card_height + (card_lines as f32 - 1.0) * CARD_GAP;
            y += CARDS_TO_CHIPS_GAP;
        }

        let addable = self.addable_apps();
        let full = self.config.favorites_full();
        let add_label = t.get(StringId::EditorAddApp).into_owned();
        let label_style = TextStyle::new(17.0, 500);
        let add_label_width = p.text.measure(&add_label, &label_style);
        let chip_style = TextStyle::new(18.0, 700);
        let widths: Vec<f32> = addable
            .iter()
            .map(|(_, label)| 20.0 + 16.0 + 10.0 + p.text.measure(label, &chip_style) + 20.0)
            .collect();
        let chips_x = inner_left + add_label_width + 16.0;
        let positions = flow(
            &widths,
            chips_x,
            LEFT + WIDTH - PANEL_PADDING_X,
            CHIP_GAP,
            inner_left,
            CHIP_HEIGHT + CHIP_GAP,
            y,
        );
        let chip_lines = if addable.is_empty() {
            1
        } else {
            positions.last().map_or(1, |q| q.line + 1)
        };
        let panel_bottom = y
            + chip_lines as f32 * CHIP_HEIGHT
            + (chip_lines as f32 - 1.0) * CHIP_GAP
            + PANEL_PADDING_Y;
        let panel = Rect::new(LEFT, top, WIDTH, panel_bottom - top);

        let view_height = p.display.visible_rect().bottom().max(720.0);
        let is_visible = |r: Rect| r.bottom() >= -32.0 && r.y <= view_height + 32.0;

        if draw && is_visible(panel.translate(0.0, -scroll_y)) {
            let style = p.style(|t| &t.panel, StateFlags::NONE);
            p.draw_box(panel.translate(0.0, -scroll_y), &style);
            let heading = t.get(StringId::EditorFavoritesOrder).to_uppercase();
            let heading_style = p.style(|t| &t.card_heading, StateFlags::NONE);
            p.text.draw(
                p.list,
                &heading,
                inner_left,
                heading_y + 17.0 - scroll_y,
                &Painter::text_style(&heading_style),
                p.primary_on_dark(),
            );
            let description = t.get(StringId::EditorFavoritesDescription).into_owned();
            let description_style = TextStyle::new(16.0, 500);
            let shown = p
                .text
                .ellipsize(&description, &description_style, inner_width)
                .into_owned();
            p.text.draw(
                p.list,
                &shown,
                inner_left,
                heading_y + HEADING_GAP + 20.0 - scroll_y,
                &description_style,
                vars::TEXT_MUTED,
            );
            let hint = t.get(StringId::EditorUseArrows).into_owned();
            let hint_style = TextStyle::new(16.0, 500);
            let width = p.text.measure(&hint, &hint_style);
            p.text.draw(
                p.list,
                &hint,
                LEFT + WIDTH - PANEL_PADDING_X - width,
                heading_y + 17.0 - scroll_y,
                &hint_style,
                vars::TEXT_MUTED,
            );
        }

        // The cards, a line at a time, each with three buttons under it.
        for line in 0..card_lines {
            let mut items: Vec<Focusable> = Vec::new();
            for column in 0..per_line {
                let slot = line * per_line + column;
                if slot >= total_slots {
                    break;
                }
                let x = inner_left + column as f32 * (CARD_WIDTH + CARD_GAP);
                let card_y = cards_top + line as f32 * (card_height + CARD_GAP);
                let rects: [Rect; 3] = [0.0, 1.0, 2.0].map(|b| {
                    Rect::new(
                        x + 18.0 + b * 50.0,
                        card_y + CARD_NAME_HEIGHT + 6.0,
                        44.0,
                        44.0,
                    )
                });
                let button_ids: [FocusId; 3] =
                    [0, 1, 2].map(|b| FAVORITE_BUTTON_BASE + slot as u32 * 3 + b);
                let last_slot = total_slots - 1;
                let fav_index = self.config.dock_slot_favorite_index(slot);
                let is_all_apps = fav_index.is_none();
                for b in 0..3 {
                    layout.rects.insert(button_ids[b], rects[b]);
                    let disabled = (b == 0 && slot == 0)
                        || (b == 1 && slot == last_slot)
                        || (b == 2 && is_all_apps);
                    items.push(if disabled {
                        Focusable::disabled(button_ids[b], rects[b])
                    } else {
                        Focusable::new(button_ids[b], rects[b])
                    });
                }
                let card_scrolled = Rect::new(x, card_y - scroll_y, CARD_WIDTH, card_height);
                if draw && is_visible(card_scrolled) {
                    if let Some(fav_idx) = fav_index {
                        let package = &favorites[fav_idx];
                        let (label, color) = match self.catalog.get(package) {
                            Some(app) => (app.label.clone(), app.color),
                            None => (package.clone(), vars::TILE_DEFAULT),
                        };
                        self.draw_card(
                            p,
                            card_scrolled,
                            slot + 1,
                            &label,
                            color,
                            rects,
                            button_ids,
                            (slot == 0, slot == last_slot),
                            scroll_y,
                        );
                    } else {
                        self.draw_all_apps_card(
                            p,
                            card_scrolled,
                            slot + 1,
                            rects,
                            button_ids,
                            (slot == 0, slot == last_slot),
                            scroll_y,
                        );
                    }
                }
            }
            layout.row(items);
        }

        // "Add app".
        let mut chip_rows: Vec<Vec<Focusable>> = vec![Vec::new(); chip_lines];
        if draw {
            let first_baseline = y + CHIP_HEIGHT / 2.0 + 6.0 - scroll_y;
            p.text.draw(
                p.list,
                &add_label,
                inner_left,
                first_baseline,
                &label_style,
                vars::TEXT_MUTED,
            );
        }
        for (i, ((_, label), pos)) in addable.iter().zip(&positions).enumerate() {
            let rect = Rect::new(pos.x, pos.y, widths[i], CHIP_HEIGHT);
            let id = ADD_APP_BASE + i as u32;
            layout.rects.insert(id, rect);
            if !full {
                chip_rows[pos.line].push(Focusable::new(id, rect));
            }
            let chip_scrolled = rect.translate(0.0, -scroll_y);
            if draw && is_visible(chip_scrolled) {
                let progress = self.focus_progress(id);
                self.draw_add_chip(p, chip_scrolled, label, progress, full);
            }
        }
        if full && draw && is_visible(panel.translate(0.0, -scroll_y)) {
            let message = t.plural(
                PluralId::EditorLimitReached,
                MAX_FAVORITES as i64,
                &[Arg::Int(MAX_FAVORITES as i64)],
            );
            let style = TextStyle::new(16.0, 500);
            let width = p.text.measure(&message, &style);
            p.text.draw(
                p.list,
                &message,
                LEFT + WIDTH - 32.0 - width,
                panel_bottom - 24.0 - 14.0 - scroll_y,
                &style,
                vars::TEXT_MUTED,
            );
        }
        for items in chip_rows {
            layout.row(items);
        }
        panel_bottom
    }

    // ---- drawing of the pieces --------------------------------------------------------------

    /// Section displaying the wallpapers as 178x100 mini-cards inside a panel with horizontal scrolling.
    fn wallpaper_section(
        &mut self,
        p: &mut Painter,
        layout: &mut Layout,
        top: f32,
        scroll_y: f32,
        draw: bool,
    ) -> f32 {
        let t = self.translator;
        let inner_left = LEFT + PANEL_PADDING_X;
        let inner_width = WIDTH - 2.0 * PANEL_PADDING_X;

        let heading_y = top + PANEL_PADDING_Y;
        let cards_y = heading_y + HEADING_GAP;
        // Under the wallpapers: divider, accent color swatches, 2-level navigation selector, and
        // favorites dock shape selector.
        let div_y = cards_y + WALLPAPER_CARD_HEIGHT + ROW_GAP + 8.0;
        let accent_y = div_y + ROW_GAP;
        let accent_rect = Rect::new(inner_left, accent_y, inner_width, ACCENT_ROW_HEIGHT);
        let tabs_y = accent_y + ACCENT_ROW_HEIGHT + EDITOR_ROW_GAP;
        let tabs_rect = Rect::new(inner_left, tabs_y, inner_width, ROW_HEIGHT);
        let subnav_y = tabs_y + ROW_HEIGHT + EDITOR_ROW_GAP;
        let subnav_rect = Rect::new(inner_left, subnav_y, inner_width, ROW_HEIGHT);
        let shape_y = subnav_y + ROW_HEIGHT + EDITOR_ROW_GAP;
        let shape_rect = Rect::new(inner_left, shape_y, inner_width, ROW_HEIGHT);
        let panel_bottom = shape_y + ROW_HEIGHT + PANEL_PADDING_Y;
        let panel = Rect::new(LEFT, top, WIDTH, panel_bottom - top);

        let focused_id = self.focus.current_id();
        let card_step = WALLPAPER_CARD_WIDTH + WALLPAPER_CARD_GAP;
        let max_scroll =
            (CARD_COUNT as f32 * card_step - WALLPAPER_CARD_GAP - inner_width).max(0.0);

        let focused_wp_index = focused_id.and_then(|fid| {
            if (WALLPAPER_BASE..WALLPAPER_BASE + CARD_COUNT as FocusId).contains(&fid) {
                Some(card_slot(fid))
            } else {
                None
            }
        });

        if let Some(index) = focused_wp_index {
            let card_left = inner_left + index as f32 * card_step;
            let card_right = card_left + WALLPAPER_CARD_WIDTH;

            if card_right - self.editor.target_wallpaper_scroll_x > inner_left + inner_width {
                self.editor.target_wallpaper_scroll_x = card_right - (inner_left + inner_width);
            } else if card_left - self.editor.target_wallpaper_scroll_x < inner_left {
                self.editor.target_wallpaper_scroll_x = card_left - inner_left;
            }
        }
        self.editor.target_wallpaper_scroll_x =
            self.editor.target_wallpaper_scroll_x.clamp(0.0, max_scroll);

        if draw {
            let current_scroll = self.anim.get(
                Anim::WallpaperScrollX,
                self.editor.wallpaper_scroll_x,
                self.now_ms,
            );
            if (current_scroll - self.editor.target_wallpaper_scroll_x).abs() > 0.5 {
                self.anim.animate_to(
                    Anim::WallpaperScrollX,
                    current_scroll,
                    self.editor.target_wallpaper_scroll_x,
                    self.now_ms,
                    Some(vars::FOCUS_TIME),
                );
                self.editor.wallpaper_scroll_x = current_scroll;
            } else {
                self.editor.wallpaper_scroll_x = self.editor.target_wallpaper_scroll_x;
            }
        }

        let view_height = p.display.visible_rect().bottom().max(720.0);
        let is_visible = |r: Rect| r.bottom() >= -32.0 && r.y <= view_height + 32.0;

        if draw && !self.editor.confirm && is_visible(panel.translate(0.0, -scroll_y)) {
            let style = p.style(|t| &t.panel, StateFlags::NONE);
            p.draw_box(panel.translate(0.0, -scroll_y), &style);

            let heading = t.get(StringId::EditorAppearance).to_uppercase();
            let heading_style = p.style(|t| &t.card_heading, StateFlags::NONE);
            p.text.draw(
                p.list,
                &heading,
                inner_left,
                heading_y + 17.0 - scroll_y,
                &Painter::text_style(&heading_style),
                p.primary_on_dark(),
            );

            let active_idx = focused_wp_index.unwrap_or_else(|| {
                card_slot(WALLPAPER_BASE + self.config.wallpaper.index() as FocusId)
            });
            let total = CARD_COUNT;
            let hint = if self.editor.wallpaper_scroll_x <= 4.0 {
                format!("{} / {}  ›", active_idx + 1, total)
            } else if self.editor.wallpaper_scroll_x >= max_scroll - 4.0 {
                format!("‹  {} / {}", active_idx + 1, total)
            } else {
                format!("‹  {} / {}  ›", active_idx + 1, total)
            };
            let hint_style = TextStyle::new(16.0, 700);
            let width = p.text.measure(&hint, &hint_style);
            p.text.draw(
                p.list,
                &hint,
                LEFT + WIDTH - PANEL_PADDING_X - width,
                heading_y + 17.0 - scroll_y,
                &hint_style,
                vars::TEXT_MUTED,
            );
        }

        let mut items = Vec::with_capacity(CARD_COUNT);
        for i in 0..CARD_COUNT {
            let card_id = slot_card(i);
            let card_x = inner_left + i as f32 * card_step;
            let card_rect = Rect::new(card_x, cards_y, WALLPAPER_CARD_WIDTH, WALLPAPER_CARD_HEIGHT);
            layout.rects.insert(card_id, card_rect);
            items.push(Focusable::new(card_id, card_rect));
        }

        let active_wp_id = WALLPAPER_BASE + self.config.wallpaper.index() as FocusId;
        layout
            .rows
            .push(Row::new(items).with_entry_up(active_wp_id));

        // Accent color swatch row (10 circular swatches right-aligned to x = 1160).
        let swatch_total_w = AccentColor::ALL.len() as f32 * SWATCH_SIZE
            + (AccentColor::ALL.len() as f32 - 1.0) * SWATCH_GAP;
        let swatch_start_x = accent_rect.right() - ROW_PADDING_X - swatch_total_w;
        let swatch_y = accent_rect.y + (ACCENT_ROW_HEIGHT - SWATCH_SIZE) / 2.0;
        let mut swatch_items = Vec::with_capacity(AccentColor::ALL.len());
        let mut swatch_rects = [Rect::new(0.0, 0.0, 0.0, 0.0); AccentColor::ALL.len()];
        for (i, slot) in swatch_rects.iter_mut().enumerate() {
            let swatch_id = ACCENT_BASE + i as FocusId;
            let rect = Rect::new(
                swatch_start_x + i as f32 * (SWATCH_SIZE + SWATCH_GAP),
                swatch_y,
                SWATCH_SIZE,
                SWATCH_SIZE,
            );
            *slot = rect;
            layout.rects.insert(swatch_id, rect);
            swatch_items.push(Focusable::new(swatch_id, rect));
        }
        let active_accent_id = ACCENT_BASE + accent_index(self.config.accent_color) as FocusId;
        layout.rows.push(
            Row::new(swatch_items)
                .with_entry_down(active_accent_id)
                .with_entry_up(active_accent_id),
        );

        // Level 1 navigation row: Top bar tabs (Tabs on | Tabs off).
        let tabs_options = [
            (
                NAV_TABS_ON,
                t.get(StringId::EditorTabsOn).into_owned(),
                self.config.tabs_visible,
            ),
            (
                NAV_TABS_OFF,
                t.get(StringId::EditorTabsOff).into_owned(),
                !self.config.tabs_visible,
            ),
        ];
        let tabs_chips = self.layout_option_chips(p, tabs_rect, &tabs_options);
        let active_tabs_id = if self.config.tabs_visible {
            NAV_TABS_ON
        } else {
            NAV_TABS_OFF
        };
        let mut tabs_row_items = Vec::with_capacity(tabs_chips.len());
        for &(id, rect, _) in &tabs_chips {
            layout.rects.insert(id, rect);
            tabs_row_items.push(Focusable::new(id, rect));
        }
        layout.rows.push(
            Row::new(tabs_row_items)
                .with_entry_down(active_tabs_id)
                .with_entry_up(active_tabs_id),
        );

        // Level 2 navigation row: conditional on `tabs_visible`.
        let (subnav_label, subnav_options, active_subnav_id): (
            String,
            Vec<(FocusId, String, bool)>,
            FocusId,
        ) = if self.config.tabs_visible {
            (
                t.get(StringId::EditorTabsPosition).into_owned(),
                vec![
                    (
                        NAV_ALIGN_LEFT,
                        t.get(StringId::EditorTabsLeft).into_owned(),
                        self.config.tabs_align == TabsAlign::Left,
                    ),
                    (
                        NAV_ALIGN_CENTER,
                        t.get(StringId::EditorTabsCenter).into_owned(),
                        self.config.tabs_align == TabsAlign::Center,
                    ),
                ],
                match self.config.tabs_align {
                    TabsAlign::Left => NAV_ALIGN_LEFT,
                    TabsAlign::Center => NAV_ALIGN_CENTER,
                },
            )
        } else {
            (
                t.get(StringId::EditorAppsAccess).into_owned(),
                vec![
                    (
                        NAV_MODE_SLIDE,
                        t.get(StringId::EditorAccessSlide).into_owned(),
                        self.config.tabless_mode == TablessMode::Slide,
                    ),
                    (
                        NAV_MODE_PEEK,
                        t.get(StringId::EditorAccessPeek).into_owned(),
                        self.config.tabless_mode == TablessMode::Peek,
                    ),
                    (
                        NAV_MODE_DOCK,
                        t.get(StringId::EditorAccessDock).into_owned(),
                        self.config.tabless_mode == TablessMode::Dock,
                    ),
                ],
                match self.config.tabless_mode {
                    TablessMode::Slide => NAV_MODE_SLIDE,
                    TablessMode::Peek => NAV_MODE_PEEK,
                    TablessMode::Dock => NAV_MODE_DOCK,
                },
            )
        };
        let subnav_chips = self.layout_option_chips(p, subnav_rect, &subnav_options);
        let mut subnav_row_items = Vec::with_capacity(subnav_chips.len());
        for &(id, rect, _) in &subnav_chips {
            layout.rects.insert(id, rect);
            subnav_row_items.push(Focusable::new(id, rect));
        }
        layout.rows.push(
            Row::new(subnav_row_items)
                .with_entry_down(active_subnav_id)
                .with_entry_up(active_subnav_id),
        );

        let shape_options = [
            (
                SHAPE_ROUND,
                t.get(StringId::EditorShapeRound).into_owned(),
                self.config.dock_shape == DockIconShape::Round,
            ),
            (
                SHAPE_SQUARE,
                t.get(StringId::EditorShapeSquare).into_owned(),
                self.config.dock_shape == DockIconShape::Square,
            ),
            (
                SHAPE_BANNER,
                t.get(StringId::EditorShapeBanner).into_owned(),
                self.config.dock_shape == DockIconShape::Banner,
            ),
        ];
        let shape_chips = self.layout_option_chips(p, shape_rect, &shape_options);
        let active_shape_id = match self.config.dock_shape {
            DockIconShape::Round => SHAPE_ROUND,
            DockIconShape::Square => SHAPE_SQUARE,
            DockIconShape::Banner => SHAPE_BANNER,
        };
        let mut shape_row_items = Vec::with_capacity(shape_chips.len());
        for &(id, rect, _) in &shape_chips {
            layout.rects.insert(id, rect);
            shape_row_items.push(Focusable::new(id, rect));
        }
        layout.rows.push(
            Row::new(shape_row_items)
                .with_entry_down(active_shape_id)
                .with_entry_up(active_shape_id),
        );

        if draw && !self.editor.confirm {
            self.draw_accent_row(
                p,
                accent_rect.translate(0.0, -scroll_y),
                &swatch_rects,
                scroll_y,
            );
            let tabs_label = t.get(StringId::EditorTopbarTabs).into_owned();
            self.draw_option_row(
                p,
                tabs_rect.translate(0.0, -scroll_y),
                &tabs_label,
                &tabs_options,
                &tabs_chips,
                scroll_y,
            );
            self.draw_option_row(
                p,
                subnav_rect.translate(0.0, -scroll_y),
                &subnav_label,
                &subnav_options,
                &subnav_chips,
                scroll_y,
            );
            let shape_label = t.get(StringId::EditorFavoritesShape).into_owned();
            self.draw_option_row(
                p,
                shape_rect.translate(0.0, -scroll_y),
                &shape_label,
                &shape_options,
                &shape_chips,
                scroll_y,
            );
            p.list.fill(
                Rect::new(inner_left, div_y - scroll_y, inner_width, 1.0),
                0.5,
                Paint::Solid(vars::SURFACE_ROW),
            );
            let visible_clip = Rect::new(
                inner_left - 6.0,
                cards_y - 8.0 - scroll_y,
                inner_width + 12.0,
                WALLPAPER_CARD_HEIGHT + 16.0,
            );
            p.list.push_clip(visible_clip);

            for (i, &id) in WallpaperId::ALL.iter().enumerate() {
                let card_id = WALLPAPER_BASE + i as FocusId;
                let card_x = inner_left + card_slot(card_id) as f32 * card_step
                    - self.editor.wallpaper_scroll_x;
                let card_rect = Rect::new(
                    card_x,
                    cards_y - scroll_y,
                    WALLPAPER_CARD_WIDTH,
                    WALLPAPER_CARD_HEIGHT,
                );

                if card_rect.right() >= inner_left && card_rect.x <= inner_left + inner_width {
                    self.draw_wallpaper_card(p, card_rect, id, card_id);
                }
            }
            let phone_rect = Rect::new(
                inner_left + card_slot(PHONE_CARD) as f32 * card_step
                    - self.editor.wallpaper_scroll_x,
                cards_y - scroll_y,
                WALLPAPER_CARD_WIDTH,
                WALLPAPER_CARD_HEIGHT,
            );
            self.anchor_context_menu(
                super::context_menu::Origin::WallpaperCard(PHONE_CARD),
                phone_rect,
            );
            if phone_rect.right() >= inner_left && phone_rect.x <= inner_left + inner_width {
                if self.config.custom.is_some() {
                    self.draw_wallpaper_card(p, phone_rect, WallpaperId::Custom, PHONE_CARD);
                } else {
                    self.draw_phone_card(p, phone_rect);
                }
            }

            p.list.pop_clip();

            if self.editor.wallpaper_scroll_x > 4.0 {
                let left_edge =
                    Rect::new(inner_left, cards_y - scroll_y, 48.0, WALLPAPER_CARD_HEIGHT);
                let edge_scrim = Paint::Gradient(crate::theme::Gradient::new(
                    270.0,
                    &[
                        crate::theme::GradientStop {
                            color: Color {
                                r: 18,
                                g: 28,
                                b: 37,
                                a: 220,
                            },
                            position: 0.0,
                        },
                        crate::theme::GradientStop {
                            color: Color::TRANSPARENT,
                            position: 1.0,
                        },
                    ],
                ));
                p.list.fill(left_edge, vars::SHAPE_MD, edge_scrim);
                let chevron_rect = Rect::new(
                    inner_left + 6.0,
                    cards_y - scroll_y + (WALLPAPER_CARD_HEIGHT - 24.0) / 2.0,
                    24.0,
                    24.0,
                );
                p.icon(icons::CHEVRON_LEFT, chevron_rect, 20.0, vars::TEXT_MUTED);
            }
            if self.editor.wallpaper_scroll_x < max_scroll - 4.0 {
                let right_edge = Rect::new(
                    inner_left + inner_width - 48.0,
                    cards_y - scroll_y,
                    48.0,
                    WALLPAPER_CARD_HEIGHT,
                );
                let edge_scrim = Paint::Gradient(crate::theme::Gradient::new(
                    90.0,
                    &[
                        crate::theme::GradientStop {
                            color: Color {
                                r: 18,
                                g: 28,
                                b: 37,
                                a: 220,
                            },
                            position: 0.0,
                        },
                        crate::theme::GradientStop {
                            color: Color::TRANSPARENT,
                            position: 1.0,
                        },
                    ],
                ));
                p.list.fill(right_edge, vars::SHAPE_MD, edge_scrim);
                let chevron_rect = Rect::new(
                    inner_left + inner_width - 30.0,
                    cards_y - scroll_y + (WALLPAPER_CARD_HEIGHT - 24.0) / 2.0,
                    24.0,
                    24.0,
                );
                p.icon(icons::CHEVRON_RIGHT, chevron_rect, 20.0, vars::TEXT_MUTED);
            }
        }

        panel_bottom + BUTTONS_GAP
    }

    fn draw_wallpaper_card(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        id: WallpaperId,
        card_id: FocusId,
    ) {
        let is_focused = self.focus.current_id() == Some(card_id);
        let is_active = self.config.wallpaper == id;
        // An image from the phone has no thumbnail until Apply wrote one: its color meanwhile.
        if id == WallpaperId::Custom && crate::wallpaper::custom::thumbnail().is_none() {
            p.list
                .fill(rect, vars::SHAPE_MD, Paint::Solid(vars::SURFACE_ROW));
        }

        let target_rect = if is_focused {
            rect.scaled_about_center(1.05)
        } else {
            rect
        };

        // No shadow: the mini-card does not float; the focus is the ring and the scale.
        // Real thumbnail preview!
        p.list
            .wallpaper_thumb(target_rect, id.thumb_uv(), vars::SHAPE_MD);

        // Scrim at the bottom for text legibility
        let scrim_rect = Rect::new(
            target_rect.x,
            target_rect.bottom() - 36.0,
            target_rect.w,
            36.0,
        );
        let scrim_paint = Paint::Gradient(crate::theme::Gradient::new(
            180.0,
            &[
                crate::theme::GradientStop {
                    color: Color::TRANSPARENT,
                    position: 0.0,
                },
                crate::theme::GradientStop {
                    color: Color {
                        r: 6,
                        g: 10,
                        b: 16,
                        a: 220,
                    },
                    position: 1.0,
                },
            ],
        ));
        p.list.fill(scrim_rect, vars::SHAPE_MD, scrim_paint);

        // Outline / focus ring
        if is_focused {
            p.list.ring(target_rect, vars::SHAPE_MD, 3.0, p.primary());
            p.draw_contrast_ring(target_rect, vars::SHAPE_MD, 1.0);
        } else if is_active {
            p.list.ring(target_rect, vars::SHAPE_MD, 2.0, p.primary());
        } else {
            p.list.ring(target_rect, vars::SHAPE_MD, 1.0, vars::CONTROL);
        }

        // Active indicator badge if currently selected
        if is_active {
            let badge_size = 18.0;
            let badge_rect = Rect::new(
                target_rect.right() - badge_size - 8.0,
                target_rect.y + 8.0,
                badge_size,
                badge_size,
            );
            p.list
                .fill(badge_rect, vars::SHAPE_FULL, Paint::Solid(p.primary()));
            let dot_rect = Rect::new(badge_rect.x + 5.0, badge_rect.y + 5.0, 8.0, 8.0);
            p.list
                .fill(dot_rect, vars::SHAPE_FULL, Paint::Solid(p.on_primary()));
        }

        // Name label
        let name = self.translator.get(id.title_string_id());
        let label_style = if is_focused {
            TextStyle::new(16.0, 700)
        } else {
            TextStyle::new(16.0, 500)
        };
        let label_color = if is_focused {
            vars::TEXT
        } else {
            vars::TEXT_MUTED
        };
        let text_y = target_rect.bottom() - 12.0;
        let text_x = target_rect.x + 12.0;
        p.text
            .draw(p.list, &name, text_x, text_y, &label_style, label_color);
    }

    /// The empty phone card: `--surface` with a dashed outline of 2 in `--border`, the phone icon
    /// and "+ From phone".
    fn draw_phone_card(&mut self, p: &mut Painter, rect: Rect) {
        let is_focused = self.focus.current_id() == Some(PHONE_CARD);
        let rect = if is_focused {
            rect.scaled_about_center(1.05)
        } else {
            rect
        };
        p.list
            .fill(rect, vars::SHAPE_MD, Paint::Solid(vars::SURFACE));
        if is_focused {
            p.list.ring(rect, vars::SHAPE_MD, 3.0, p.primary());
            p.draw_contrast_ring(rect, vars::SHAPE_MD, 1.0);
        } else {
            dashed_outline(p, rect, vars::SHAPE_MD, 2.0, vars::BORDER);
        }
        let icon = Rect::new(rect.x + (rect.w - 32.0) / 2.0, rect.y + 16.0, 32.0, 32.0);
        let color = if is_focused {
            vars::TEXT
        } else {
            vars::TEXT_MUTED
        };
        p.icon(icons::PHONE, icon, 32.0, color);
        let label = self.translator.get(StringId::WallpaperFromPhone);
        let style = if is_focused {
            TextStyle::new(16.0, 700)
        } else {
            TextStyle::new(16.0, 500)
        };
        let w = p.text.measure(&label, &style);
        p.text.draw(
            p.list,
            &label,
            rect.x + (rect.w - w) / 2.0,
            rect.bottom() - 12.0,
            &style,
            color,
        );
    }

    fn draw_accent_row(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        swatch_rects: &[Rect; AccentColor::ALL.len()],
        scroll_y: f32,
    ) {
        let style = p.style(|t| &t.editor_row, StateFlags::NONE);
        p.draw_box(rect, &style);

        let t = self.translator;
        let focused_accent = self.focus.current_id().and_then(accent_for_id);
        let shown_accent = focused_accent.unwrap_or(self.config.accent_color);
        let swatch_left = swatch_rects[0].x;
        let label_w = (swatch_left - (rect.x + ROW_PADDING_X) - ROW_PADDING_X).max(120.0);

        let label = t.get(StringId::EditorAccentColor);
        let label_style = Painter::text_style(&style);
        p.text_with(
            &label,
            Rect::new(rect.x + ROW_PADDING_X, rect.y + 12.0, label_w, 24.0),
            Align::Start,
            &label_style,
            vars::TEXT,
        );

        let color_name = t.get(shown_accent.title_string_id());
        let name_style = TextStyle::new(16.0, 700);
        let name_color = if shown_accent == AccentColor::Black {
            vars::TEXT_SOFT
        } else {
            shown_accent.palette().primary
        };
        p.text_with(
            &color_name,
            Rect::new(rect.x + ROW_PADDING_X, rect.y + 38.0, label_w, 22.0),
            Align::Start,
            &name_style,
            name_color,
        );

        for (i, &accent) in AccentColor::ALL.iter().enumerate() {
            let swatch_id = ACCENT_BASE + i as FocusId;
            let base_rect = swatch_rects[i].translate(0.0, -scroll_y);
            let progress = self.focus_progress(swatch_id);
            let drawn = base_rect.scaled_about_center(1.0 + 0.08 * progress);
            let palette = accent.palette();

            p.list
                .fill(drawn, vars::SHAPE_FULL, Paint::Solid(palette.primary));
            if let Some(contrast) = palette.contrast_ring {
                p.list.ring(drawn, vars::SHAPE_FULL, 1.5, contrast);
            }
            if accent == self.config.accent_color {
                p.icon(icons::CHECK, drawn, 20.0, palette.on_primary);
            }
            if progress > 0.0 {
                let ring_rect = drawn.grow(4.0 + 3.0);
                p.list.ring(
                    ring_rect,
                    vars::SHAPE_FULL,
                    3.0,
                    fade(palette.primary, progress),
                );
                if let Some(contrast) = palette.contrast_ring {
                    let outer = ring_rect.grow(super::painter::CONTRAST_RING_WIDTH);
                    p.list.ring(
                        outer,
                        vars::SHAPE_FULL,
                        super::painter::CONTRAST_RING_WIDTH,
                        fade(contrast, progress),
                    );
                }
            }
        }
    }

    /// Computes right-aligned 40 px option chips on the 4-unit ruler inside an `.editor-row`.
    fn layout_option_chips(
        &self,
        p: &mut Painter,
        row_rect: Rect,
        options: &[(FocusId, String, bool)],
    ) -> Vec<(FocusId, Rect, bool)> {
        let chip_style = p.style(|t| &t.chip, StateFlags::NONE);
        let widths: Vec<f32> = options
            .iter()
            .map(|(_, label, _)| {
                let raw = p.measure(label, &chip_style) + 2.0 * 16.0;
                ((raw / 4.0).ceil() * 4.0).max(64.0)
            })
            .collect();
        let total_w: f32 =
            widths.iter().sum::<f32>() + OPTION_CHIP_GAP * options.len().saturating_sub(1) as f32;
        let mut x = row_rect.right() - ROW_PADDING_X - total_w;
        let chip_y = row_rect.y + (row_rect.h - CHIP_HEIGHT) / 2.0;
        let mut placed = Vec::with_capacity(options.len());
        for ((id, _, active), width) in options.iter().zip(widths) {
            placed.push((*id, Rect::new(x, chip_y, width, CHIP_HEIGHT), *active));
            x += width + OPTION_CHIP_GAP;
        }
        placed
    }

    fn draw_option_row(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        label: &str,
        options: &[(FocusId, String, bool)],
        chips: &[(FocusId, Rect, bool)],
        scroll_y: f32,
    ) {
        let style = p.style(|t| &t.editor_row, StateFlags::NONE);
        p.draw_box(rect, &style);

        let chips_left = chips
            .first()
            .map_or(rect.right() - ROW_PADDING_X, |c| c.1.x);
        let label_w = (chips_left - (rect.x + ROW_PADDING_X) - ROW_PADDING_X).max(120.0);
        let text_rect = Rect::new(rect.x + ROW_PADDING_X, rect.y, label_w, rect.h);
        p.text(label, text_rect, Align::Start, &style);

        for ((_, option_label, _), &(id, chip_rect, active)) in options.iter().zip(chips) {
            let progress = self.focus_progress(id);
            self.draw_option_chip(
                p,
                chip_rect.translate(0.0, -scroll_y),
                option_label,
                active,
                progress,
            );
        }
    }

    fn draw_option_chip(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        label: &str,
        active: bool,
        progress: f32,
    ) {
        let base_flags = StateFlags {
            active,
            ..StateFlags::NONE
        };
        let rest = p.style(|t| &t.chip, base_flags);
        let style = if progress > 0.0 {
            let focused = p.style(
                |t| &t.chip,
                StateFlags {
                    focus: true,
                    active,
                    ..StateFlags::NONE
                },
            );
            mix(&rest, &focused, progress)
        } else {
            rest
        };
        p.draw_box(rect, &style);
        p.text(label, rect, Align::Center, &style);
    }

    /// A button of the design system: `kind` picks the component of the theme.
    pub(super) fn draw_button(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        label: &str,
        id: FocusId,
        kind: ButtonKind,
    ) {
        let progress = self.focus_progress(id);
        let style = p.focus_style(|t| kind.component(t), progress);
        p.draw_box(rect, &style);
        p.text(
            label,
            rect.scaled_about_center(style.scale),
            Align::Center,
            &style,
        );
    }

    fn draw_round_button(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        icon: char,
        progress: f32,
        disabled: bool,
        danger: bool,
    ) {
        let state = if disabled {
            StateFlags::DISABLED
        } else {
            StateFlags::NONE
        };
        let mut style = if disabled {
            p.style(|t| &t.icon_button, state)
        } else {
            p.focus_style(|t| &t.icon_button, progress)
        };
        style.border_radius = vars::SHAPE_FULL;
        style.background = Some(Paint::Solid(match (disabled, danger) {
            (true, _) => vars::CONTROL_DISABLED,
            (false, true) => vars::DANGER_BG,
            (false, false) => vars::CONTROL,
        }));
        p.draw_box(rect, &style);
        let color = match (disabled, danger) {
            (true, _) => vars::TEXT_DISABLED,
            (false, true) => vars::DANGER_FG,
            (false, false) => vars::TEXT,
        };
        p.icon(icon, rect, 24.0, color);
    }

    fn draw_add_chip(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        label: &str,
        progress: f32,
        disabled: bool,
    ) {
        let radius = vars::SHAPE_FULL;
        let opacity = if disabled { 0.4 } else { 1.0 };
        p.list
            .fill(rect, radius, Paint::Solid(fade(vars::SURFACE, opacity)));
        p.list.ring(rect, radius, 2.0, fade(vars::CONTROL, opacity));
        p.icon(
            icons::PLUS,
            Rect::new(rect.x + 18.0, rect.y, 16.0, rect.h),
            16.0,
            fade(vars::TEXT, opacity),
        );
        let style = TextStyle::new(18.0, 700);
        let text_rect = Rect::new(
            rect.x + 18.0 + 16.0 + 10.0,
            rect.y,
            rect.w - 18.0 - 16.0 - 10.0 - 16.0,
            rect.h,
        );
        p.text_with(
            label,
            text_rect,
            Align::Start,
            &style,
            fade(vars::TEXT, opacity),
        );
        if !disabled {
            self.draw_focus_ring(p, rect, radius, progress);
        }
    }

    fn draw_focus_ring(&mut self, p: &mut Painter, rect: Rect, radius: f32, progress: f32) {
        if progress <= 0.0 {
            return;
        }
        let ring = rect.grow(3.0);
        p.list.ring(
            ring,
            radius + 3.0,
            3.0,
            Color {
                a: (255.0 * progress) as u8,
                ..p.primary()
            },
        );
        p.draw_contrast_ring(ring, radius + 3.0, progress);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_card(
        &mut self,
        p: &mut Painter,
        card: Rect,
        number: usize,
        label: &str,
        color: Color,
        buttons: [Rect; 3],
        button_ids: [FocusId; 3],
        (first, last): (bool, bool),
        scroll_y: f32,
    ) {
        let name = Rect::new(card.x, card.y, card.w, CARD_NAME_HEIGHT);
        p.list.fill(name, vars::SHAPE_MD, Paint::Solid(color));
        // Square off the bottom of the colored part where it meets the controls.
        p.list.fill(
            Rect::new(card.x, name.bottom() - 12.0, card.w, 12.0),
            0.0,
            Paint::Solid(color),
        );
        let controls = Rect::new(card.x, name.bottom(), card.w, CARD_CONTROLS_HEIGHT);
        p.list
            .fill(controls, vars::SHAPE_MD, Paint::Solid(vars::SURFACE_ROW));
        p.list.fill(
            Rect::new(card.x, controls.y, card.w, 12.0),
            0.0,
            Paint::Solid(vars::SURFACE_ROW),
        );
        let number_style = TextStyle::new(16.0, 500);
        p.text_with(
            &number.to_string(),
            Rect::new(name.x + 14.0, name.y, 20.0, name.h),
            Align::Start,
            &number_style,
            fade(Color::WHITE, 0.7),
        );
        let label_style = TextStyle::new(20.0, 700);
        let label_rect = Rect::new(name.x + 36.0, name.y, name.w - 36.0 - 10.0, name.h);
        p.text_with(label, label_rect, Align::Start, &label_style, Color::WHITE);
        let button_icons = [icons::CHEVRON_LEFT, icons::CHEVRON_RIGHT, icons::CLOSE];
        for b in 0..3 {
            let disabled = (b == 0 && first) || (b == 1 && last);
            let progress = self.focus_progress(button_ids[b]);
            let rect = buttons[b].translate(0.0, -scroll_y);
            self.draw_round_button(p, rect, button_icons[b], progress, disabled, b == 2);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_all_apps_card(
        &mut self,
        p: &mut Painter,
        card: Rect,
        number: usize,
        buttons: [Rect; 3],
        button_ids: [FocusId; 3],
        (first, last): (bool, bool),
        scroll_y: f32,
    ) {
        let name = Rect::new(card.x, card.y, card.w, CARD_NAME_HEIGHT);
        p.list
            .fill(name, vars::SHAPE_MD, Paint::Solid(vars::SURFACE_FOCUS));
        p.list.fill(
            Rect::new(card.x, name.bottom() - 12.0, card.w, 12.0),
            0.0,
            Paint::Solid(vars::SURFACE_FOCUS),
        );
        let controls = Rect::new(card.x, name.bottom(), card.w, CARD_CONTROLS_HEIGHT);
        p.list
            .fill(controls, vars::SHAPE_MD, Paint::Solid(vars::SURFACE_ROW));
        p.list.fill(
            Rect::new(card.x, controls.y, card.w, 12.0),
            0.0,
            Paint::Solid(vars::SURFACE_ROW),
        );

        let number_style = TextStyle::new(16.0, 500);
        p.text_with(
            &number.to_string(),
            Rect::new(name.x + 12.0, name.y + 6.0, 20.0, 22.0),
            Align::Start,
            &number_style,
            fade(vars::TEXT, 0.7),
        );
        let icon_color = if self.effective_accent() == AccentColor::Black {
            vars::TEXT
        } else {
            p.primary()
        };
        p.icon(
            icons::LAYOUT_GRID,
            Rect::new(name.x + 32.0, name.y + 8.0, 18.0, 18.0),
            18.0,
            icon_color,
        );
        let title = self.translator.get(StringId::AppsTitle).into_owned();
        let title_style = TextStyle::new(16.0, 700);
        p.text_with(
            &title,
            Rect::new(name.x + 54.0, name.y + 6.0, name.w - 62.0, 22.0),
            Align::Start,
            &title_style,
            vars::TEXT,
        );
        let hint = self
            .translator
            .get(StringId::EditorFixedInDock)
            .into_owned();
        let hint_style = TextStyle::new(16.0, 500);
        p.text_with(
            &hint,
            Rect::new(name.x + 12.0, name.y + 28.0, name.w - 24.0, 22.0),
            Align::Start,
            &hint_style,
            vars::TEXT_MUTED,
        );

        let button_icons = [icons::CHEVRON_LEFT, icons::CHEVRON_RIGHT, icons::CLOSE];
        for b in 0..3 {
            let disabled = (b == 0 && first) || (b == 1 && last) || b == 2;
            let progress = self.focus_progress(button_ids[b]);
            let rect = buttons[b].translate(0.0, -scroll_y);
            self.draw_round_button(p, rect, button_icons[b], progress, disabled, b == 2);
        }
    }

    /// The "Restore defaults?" dialog; returns its focus rows and rectangles.
    fn confirm_dialog(
        &mut self,
        p: &mut Painter,
        draw: bool,
    ) -> (Vec<Row>, HashMap<FocusId, Rect>) {
        let t = self.translator;
        let visible = p.display.visible_rect();
        let width = 640.0;
        let height = 280.0;
        let dialog = Rect::new(640.0 - width / 2.0, 360.0 - height / 2.0, width, height);
        let (title_id, body_id, confirm_id) = if self.editor.remove_image {
            (
                StringId::EditorRemoveImageTitle,
                StringId::EditorRemoveImageBody,
                StringId::MenuRemoveImage,
            )
        } else {
            (
                StringId::EditorRestoreTitle,
                StringId::EditorRestoreBody,
                StringId::EditorRestoreConfirm,
            )
        };
        let confirm_label = t.get(confirm_id).into_owned();
        let cancel_label = t.get(StringId::EditorCancel).into_owned();
        let button_style = p.style(|t| &t.button, StateFlags::NONE);
        let confirm_w = (p.measure(&confirm_label, &button_style) + 56.0).max(140.0);
        let cancel_w = (p.measure(&cancel_label, &button_style) + 56.0).max(140.0);
        let y = dialog.bottom() - 32.0 - BUTTON_HEIGHT;
        let cancel = Rect::new(dialog.right() - 32.0 - cancel_w, y, cancel_w, BUTTON_HEIGHT);
        let confirm = Rect::new(cancel.x - 16.0 - confirm_w, y, confirm_w, BUTTON_HEIGHT);
        if draw {
            p.list.fill(
                visible,
                0.0,
                Paint::Solid(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 170,
                }),
            );
            p.list.fill(
                dialog.translate(0.0, 12.0).grow(2.0),
                24.0,
                Paint::Solid(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 60,
                }),
            );
            p.list
                .fill(dialog, vars::SHAPE_LG, Paint::Solid(vars::SURFACE_ROW));
            p.list.ring(dialog, vars::SHAPE_LG, 2.0, vars::BORDER);
            let title_style = TextStyle::new(28.0, 800);
            let title = p
                .text
                .ellipsize(&t.get(title_id), &title_style, width - 64.0)
                .into_owned();
            p.text.draw(
                p.list,
                &title,
                dialog.x + 32.0,
                dialog.y + 66.0,
                &title_style,
                vars::TEXT,
            );
            let body_style = TextStyle::new(18.0, 500);
            let body = t.get(body_id).into_owned();
            let mut line_y = dialog.y + 110.0;
            for line in wrap(p, &body, &body_style, width - 64.0) {
                p.text.draw(
                    p.list,
                    &line,
                    dialog.x + 32.0,
                    line_y,
                    &body_style,
                    vars::TEXT_MUTED,
                );
                line_y += vars::LINE_LABEL;
            }
            self.draw_button(
                p,
                confirm,
                &confirm_label,
                CONFIRM_RESTORE,
                ButtonKind::Danger,
            );
            self.draw_button(
                p,
                cancel,
                &cancel_label,
                CONFIRM_CANCEL,
                ButtonKind::Secondary,
            );
        }
        let rows = vec![Row::new(vec![
            Focusable::new(CONFIRM_RESTORE, confirm),
            Focusable::new(CONFIRM_CANCEL, cancel),
        ])];
        let rects = HashMap::from([(CONFIRM_RESTORE, confirm), (CONFIRM_CANCEL, cancel)]);
        (rows, rects)
    }

    // ---- actions ----------------------------------------------------------------------------

    pub(super) fn activate_editor(&mut self, id: FocusId) -> Vec<Effect> {
        let save = vec![Effect::SaveConfig];
        if let Some(accent) = accent_for_id(id) {
            let changed = self.config.accent_color != accent;
            self.config.accent_color = accent;
            self.preview_accent = None;
            self.refresh_theme();
            return if changed { save } else { vec![] };
        }
        match id {
            NAV_TABS_ON => {
                if !self.config.tabs_visible {
                    self.config.tabs_visible = true;
                    return save;
                }
                return vec![];
            }
            NAV_TABS_OFF => {
                if self.config.tabs_visible {
                    self.config.tabs_visible = false;
                    return save;
                }
                return vec![];
            }
            NAV_ALIGN_LEFT => {
                if self.config.tabs_align != TabsAlign::Left {
                    self.config.tabs_align = TabsAlign::Left;
                    return save;
                }
                return vec![];
            }
            NAV_ALIGN_CENTER => {
                if self.config.tabs_align != TabsAlign::Center {
                    self.config.tabs_align = TabsAlign::Center;
                    return save;
                }
                return vec![];
            }
            NAV_MODE_SLIDE => {
                if self.config.tabless_mode != TablessMode::Slide {
                    self.config.tabless_mode = TablessMode::Slide;
                    return save;
                }
                return vec![];
            }
            NAV_MODE_PEEK => {
                if self.config.tabless_mode != TablessMode::Peek {
                    self.config.tabless_mode = TablessMode::Peek;
                    return save;
                }
                return vec![];
            }
            NAV_MODE_DOCK => {
                if self.config.tabless_mode != TablessMode::Dock {
                    self.config.tabless_mode = TablessMode::Dock;
                    return save;
                }
                return vec![];
            }
            SHAPE_ROUND => {
                if self.config.dock_shape != DockIconShape::Round {
                    self.config.dock_shape = DockIconShape::Round;
                    return save;
                }
                return vec![];
            }
            SHAPE_SQUARE => {
                if self.config.dock_shape != DockIconShape::Square {
                    self.config.dock_shape = DockIconShape::Square;
                    return save;
                }
                return vec![];
            }
            SHAPE_BANNER => {
                if self.config.dock_shape != DockIconShape::Banner {
                    self.config.dock_shape = DockIconShape::Banner;
                    return save;
                }
                return vec![];
            }
            RESTORE => {
                self.editor.confirm = true;
                self.editor.remove_image = false;
                self.pending_focus = Some(PendingFocus::Id(CONFIRM_CANCEL));
                return vec![];
            }
            CONFIRM_RESTORE if self.editor.remove_image => {
                self.close_confirm();
                if let Some(files) = crate::wallpaper::custom::files_dir() {
                    self.config
                        .remove_custom(&files, crate::wallpaper::cache_dir());
                }
                return save;
            }
            CONFIRM_RESTORE => {
                // The image from the phone stays: only its use as the wallpaper goes.
                let custom = self.config.custom;
                self.config = HomeConfig::first_use(&self.catalog);
                self.config.custom = custom;
                self.preview_accent = None;
                self.refresh_theme();
                self.editor.confirm = false;
                self.pending_focus = Some(PendingFocus::Id(RESTORE));
                return save;
            }
            CONFIRM_CANCEL => {
                self.close_confirm();
                return vec![];
            }
            PHONE_CARD => {
                if self.config.custom.is_none() {
                    self.open_receive();
                    return vec![];
                }
                if self.config.wallpaper != WallpaperId::Custom {
                    self.config.wallpaper = WallpaperId::Custom;
                    return save;
                }
                // Already the wallpaper: OK opens its menu (use, replace, remove), so it
                // can be changed without knowing the Menu button.
                self.open_context_menu();
                return vec![];
            }
            _ => {}
        }
        if (WALLPAPER_BASE..WALLPAPER_BASE + WallpaperId::ALL.len() as FocusId).contains(&id) {
            let index = (id - WALLPAPER_BASE) as usize;
            let chosen = WallpaperId::ALL[index];
            if self.config.wallpaper != chosen {
                self.config.wallpaper = chosen;
                return save;
            }
            return vec![];
        }
        if (FAVORITE_BUTTON_BASE..ADD_APP_BASE).contains(&id) {
            let offset = id - FAVORITE_BUTTON_BASE;
            let (slot, button) = ((offset / 3) as usize, offset % 3);
            let changed = match button {
                0 => self.config.move_dock_slot_left(slot),
                1 => self.config.move_dock_slot_right(slot),
                _ => self.config.remove_dock_slot(slot),
            };
            if changed {
                let target = match button {
                    0 => FAVORITE_BUTTON_BASE + (slot as u32 - 1) * 3,
                    1 => FAVORITE_BUTTON_BASE + (slot as u32 + 1) * 3 + 1,
                    _ => id,
                };
                self.pending_focus = Some(PendingFocus::Id(target));
                return save;
            }
            return vec![];
        }
        if (ADD_APP_BASE..CONFIRM_RESTORE).contains(&id) {
            let addable = self.addable_apps();
            let Some((package, _)) = addable.get((id - ADD_APP_BASE) as usize) else {
                return vec![];
            };
            if self.config.add_favorite(package) {
                return save;
            }
        }
        vec![]
    }

    /// "Done", and Back at the top level of the editor.
    pub(super) fn finish_editing(&mut self) -> Vec<Effect> {
        self.set_preview_accent(None);
        self.screen = Screen::Main(Tab::Home);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
        vec![Effect::SaveConfig]
    }

    /// Closes the confirmation, the focus back on what opened it.
    fn close_confirm(&mut self) {
        self.editor.confirm = false;
        let back_to = if self.editor.remove_image {
            PHONE_CARD
        } else {
            RESTORE
        };
        self.editor.remove_image = false;
        self.pending_focus = Some(PendingFocus::Id(back_to));
    }

    /// "Remove" of the menu of "Minha imagem": asks first.
    pub(super) fn confirm_remove_image(&mut self) {
        self.editor.confirm = true;
        self.editor.remove_image = true;
        self.pending_focus = Some(PendingFocus::Id(CONFIRM_CANCEL));
    }

    pub(super) fn back_from_editor(&mut self) -> Vec<Effect> {
        if self.editor.confirm {
            self.close_confirm();
            return vec![];
        }
        self.set_preview_accent(None);
        if self.editor.from_home {
            return self.finish_editing();
        }
        // One step back: the Lume settings, on the item that opened this screen. Back from there
        // goes on to the "Lume" tile, so the way out retraces the way in.
        self.screen = Screen::Settings;
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(super::settings::ITEM_HOME_SCREEN));
        vec![Effect::SaveConfig]
    }
}

/// An outline of dashes along the straight sides of a rounded rectangle (the corners stay open).
fn dashed_outline(p: &mut Painter, rect: Rect, radius: f32, width: f32, color: Color) {
    const DASH: f32 = 8.0;
    const GAP: f32 = 6.0;
    let paint = Paint::Solid(color);
    let mut x = rect.x + radius;
    while x < rect.right() - radius {
        let w = DASH.min(rect.right() - radius - x);
        p.list.fill(Rect::new(x, rect.y, w, width), 0.0, paint);
        p.list
            .fill(Rect::new(x, rect.bottom() - width, w, width), 0.0, paint);
        x += DASH + GAP;
    }
    let mut y = rect.y + radius;
    while y < rect.bottom() - radius {
        let h = DASH.min(rect.bottom() - radius - y);
        p.list.fill(Rect::new(rect.x, y, width, h), 0.0, paint);
        p.list
            .fill(Rect::new(rect.right() - width, y, width, h), 0.0, paint);
        y += DASH + GAP;
    }
}

/// The rows and rectangles of the editor, collected while laying it out.
#[derive(Default)]
struct Layout {
    rows: Vec<Row>,
    rects: HashMap<FocusId, Rect>,
    content_height: f32,
}

impl Layout {
    /// Adds a row of focusable elements (empty rows are dropped).
    fn row(&mut self, items: Vec<Focusable>) {
        if !items.is_empty() {
            self.rows.push(Row::new(items));
        }
    }
}

struct Placed {
    x: f32,
    y: f32,
    line: usize,
}

/// Places items of the given `widths` left to right, wrapping at `max_x`. The first line starts
/// at `first_x`; later lines start at `next_x`.
fn flow(
    widths: &[f32],
    first_x: f32,
    max_x: f32,
    gap: f32,
    next_x: f32,
    line_pitch: f32,
    top: f32,
) -> Vec<Placed> {
    let mut placed = Vec::with_capacity(widths.len());
    let (mut x, mut line) = (first_x, 0usize);
    for &width in widths {
        if x + width > max_x && x > if line == 0 { first_x } else { next_x } {
            line += 1;
            x = next_x;
        }
        placed.push(Placed {
            x,
            y: top + line as f32 * line_pitch,
            line,
        });
        x += width + gap;
    }
    placed
}

/// Breaks `text` into lines no wider than `max_width`.
pub(super) fn wrap(p: &mut Painter, text: &str, style: &TextStyle, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if current.is_empty() || p.text.measure(&candidate, style) <= max_width {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_on_the_ruler() {
        super::super::assert_on_ruler(&[
            ("CHIP_HEIGHT", CHIP_HEIGHT),
            ("CHIP_GAP", CHIP_GAP),
            ("PANEL_PADDING_X", PANEL_PADDING_X),
            ("PANEL_PADDING_Y", PANEL_PADDING_Y),
            ("CARD_WIDTH", CARD_WIDTH),
            ("CARD_NAME_HEIGHT", CARD_NAME_HEIGHT),
            ("CARD_CONTROLS_HEIGHT", CARD_CONTROLS_HEIGHT),
            ("CARD_GAP", CARD_GAP),
            ("BOTTOM_PADDING", BOTTOM_PADDING),
            ("BUTTON_HEIGHT", BUTTON_HEIGHT),
            ("FIRST_CARDS_DEPTH", FIRST_CARDS_DEPTH),
            ("BUTTON_MIN_WIDTH", BUTTON_MIN_WIDTH),
            ("BUTTONS_GAP", BUTTONS_GAP),
            ("ACCENT_ROW_HEIGHT", ACCENT_ROW_HEIGHT),
            ("SWATCH_SIZE", SWATCH_SIZE),
            ("SWATCH_GAP", SWATCH_GAP),
            ("ROW_HEIGHT", ROW_HEIGHT),
            ("EDITOR_ROW_GAP", EDITOR_ROW_GAP),
            ("ROW_PADDING_X", ROW_PADDING_X),
            ("OPTION_CHIP_GAP", OPTION_CHIP_GAP),
            ("ROW_GAP", ROW_GAP),
            ("HEADING_GAP", HEADING_GAP),
            ("CARDS_TO_CHIPS_GAP", CARDS_TO_CHIPS_GAP),
            ("TOP", TOP),
        ]);
    }
}
