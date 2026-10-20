//! The home screen editor: the favorites.

use super::page::{self, LEFT, WIDTH};
use super::painter::{fade, Painter};
use super::{Effect, LumeApp, PendingFocus, Screen, Tab};
use crate::home::config::{HomeConfig, MAX_FAVORITES};
use crate::i18n::{Arg, PluralId, StringId};
use crate::icons;
use crate::render::Rect;
use crate::text::{Align, TextStyle};
use crate::theme::{vars, Color, Paint, StateFlags, THEME};
use crate::ui::focus::{FocusId, Focusable, Row};
use std::collections::HashMap;

pub const DONE: FocusId = 4000;
pub const RESTORE: FocusId = 4001;
pub(super) const FAVORITE_BUTTON_BASE: FocusId = 4400;
pub(super) const ADD_APP_BASE: FocusId = 4600;
pub const CONFIRM_RESTORE: FocusId = 4900;
pub const CONFIRM_CANCEL: FocusId = 4901;

const CHIP_HEIGHT: f32 = 42.0;
const CARD_WIDTH: f32 = 168.0;
const CARD_NAME_HEIGHT: f32 = 56.0;
const CARD_CONTROLS_HEIGHT: f32 = 60.0;
const CARD_GAP: f32 = 18.0;
const TOP: f32 = 162.0;
const BOTTOM_PADDING: f32 = 120.0;

#[derive(Clone, Debug, Default)]
pub(super) struct EditorState {
    /// The "Restore defaults" confirmation is open.
    pub confirm: bool,
}

impl LumeApp {
    pub(super) fn open_editor(&mut self) {
        self.screen = Screen::Editor;
        self.editor = EditorState::default();
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(DONE));
        self.focus.set_rows(vec![]);
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
        if focused == Some(DONE) {
            // The first element brings the whole page back to the top, with its header.
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
                    &t.get(StringId::EditorBreadcrumbEdit),
                ],
                &t.get(StringId::EditorTitle),
                Some(&t.get(StringId::EditorInstruction)),
                scroll_y,
            );
        }

        // "Done" at the top right.
        let done_label = t.get(StringId::EditorDone).into_owned();
        let button_style = p.style(&THEME.button, StateFlags::NONE);
        let done_width =
            p.measure(&done_label, &button_style) + 2.0 * button_style.padding[1].max(28.0);
        let done = Rect::new(
            LEFT + WIDTH - done_width.max(160.0),
            82.0,
            done_width.max(160.0),
            52.0,
        );
        layout.row(vec![Focusable::new(DONE, done)]);
        layout.rects.insert(DONE, done);
        if draw && !self.editor.confirm {
            self.draw_button(p, done.translate(0.0, -scroll_y), &done_label, DONE);
        }

        let y = self.favorites_panel(p, &mut layout, TOP, scroll_y, draw);

        // "Restore defaults" at the bottom right.
        let y = y + 24.0 + 24.0;
        let label = t.get(StringId::EditorRestoreDefaults).into_owned();
        let width = p.measure(&label, &button_style) + 2.0 * 28.0;
        let restore = Rect::new(LEFT + WIDTH - width, y, width, 50.0);
        layout.row(vec![Focusable::new(RESTORE, restore)]);
        layout.rects.insert(RESTORE, restore);
        if draw && !self.editor.confirm {
            p.list.fill(
                Rect::new(LEFT, y - 12.0 - scroll_y, WIDTH, 2.0),
                1.0,
                Paint::Solid(vars::SURFACE),
            );
            self.draw_outlined_button(p, restore.translate(0.0, -scroll_y), &label, RESTORE);
        }
        layout.content_height = y + 50.0 + BOTTOM_PADDING;

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
        let inner_left = LEFT + 32.0;
        let inner_width = WIDTH - 64.0;
        let per_line =
            (((inner_width + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor() as usize).max(1);
        let card_height = CARD_NAME_HEIGHT + CARD_CONTROLS_HEIGHT;
        let favorites = self.config.favorites.clone();
        let card_lines = favorites.len().div_ceil(per_line);

        let heading_y = top + 24.0;
        let cards_top = heading_y + 40.0;
        let mut y = cards_top;
        if card_lines > 0 {
            y += card_lines as f32 * card_height + (card_lines as f32 - 1.0) * CARD_GAP;
        }
        y += 24.0;

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
            LEFT + WIDTH - 32.0,
            12.0,
            inner_left,
            CHIP_HEIGHT + 12.0,
            y,
        );
        let chip_lines = if addable.is_empty() {
            1
        } else {
            positions.last().map_or(1, |q| q.line + 1)
        };
        let panel_bottom =
            y + chip_lines as f32 * CHIP_HEIGHT + (chip_lines as f32 - 1.0) * 12.0 + 24.0;
        let panel = Rect::new(LEFT, top, WIDTH, panel_bottom - top);

        if draw {
            let style = p.style(&THEME.panel, StateFlags::NONE);
            p.draw_box(panel.translate(0.0, -scroll_y), &style);
            let heading = t.get(StringId::EditorFavoritesOrder).into_owned();
            let heading_style = TextStyle {
                size: 15.0,
                weight: 800,
                letter_spacing: 1.2,
            };
            p.text.draw(
                p.list,
                &heading,
                inner_left,
                heading_y + 17.0 - scroll_y,
                &heading_style,
                vars::PRIMARY,
            );
            let hint = t.get(StringId::EditorUseArrows).into_owned();
            let hint_style = TextStyle::new(16.0, 500);
            let width = p.text.measure(&hint, &hint_style);
            p.text.draw(
                p.list,
                &hint,
                LEFT + WIDTH - 32.0 - width,
                heading_y + 17.0 - scroll_y,
                &hint_style,
                vars::TEXT_MUTED,
            );
        }

        // The cards, a line at a time, each with three buttons under it.
        for line in 0..card_lines {
            let mut items: Vec<Focusable> = Vec::new();
            for column in 0..per_line {
                let index = line * per_line + column;
                let Some(package) = favorites.get(index) else {
                    break;
                };
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
                    [0, 1, 2].map(|b| FAVORITE_BUTTON_BASE + index as u32 * 3 + b);
                let last_index = favorites.len() - 1;
                for b in 0..3 {
                    layout.rects.insert(button_ids[b], rects[b]);
                    let disabled = (b == 0 && index == 0) || (b == 1 && index == last_index);
                    items.push(if disabled {
                        Focusable::disabled(button_ids[b], rects[b])
                    } else {
                        Focusable::new(button_ids[b], rects[b])
                    });
                }
                if draw {
                    let (label, color) = match self.catalog.get(package) {
                        Some(app) => (app.label.clone(), app.color),
                        None => (package.clone(), vars::TILE_DEFAULT),
                    };
                    self.draw_card(
                        p,
                        Rect::new(x, card_y - scroll_y, CARD_WIDTH, card_height),
                        index + 1,
                        &label,
                        color,
                        rects,
                        button_ids,
                        (index == 0, index == last_index),
                        scroll_y,
                    );
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
            if draw {
                let progress = self.focus_progress(id);
                self.draw_add_chip(p, rect.translate(0.0, -scroll_y), label, progress, full);
            }
        }
        if full && draw {
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

    fn draw_button(&mut self, p: &mut Painter, rect: Rect, label: &str, id: FocusId) {
        let progress = self.focus_progress(id);
        let style = p.focus_style(&THEME.button, progress);
        p.draw_box(rect, &style);
        p.text(
            label,
            rect.scaled_about_center(style.scale),
            Align::Center,
            &style,
        );
    }

    fn draw_outlined_button(&mut self, p: &mut Painter, rect: Rect, label: &str, id: FocusId) {
        let progress = self.focus_progress(id);
        let rest = p.style(&THEME.button, StateFlags::NONE);
        let mut style =
            crate::ui::style::mix(&rest, &p.style(&THEME.button, StateFlags::FOCUS), progress);
        style.background = Some(Paint::Solid(crate::ui::style::mix_color(
            vars::SURFACE,
            vars::SURFACE_FOCUS,
            progress,
        )));
        style.color = vars::TEXT;
        let scaled = rect.scaled_about_center(style.scale);
        p.list.draw_box(scaled, &style);
        p.list.ring(scaled, style.border_radius, 2.0, vars::BORDER);
        p.text(label, scaled, Align::Center, &style);
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
            p.style(&THEME.icon_button, state)
        } else {
            p.focus_style(&THEME.icon_button, progress)
        };
        style.border_radius = rect.w / 2.0;
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
        let radius = rect.h / 2.0;
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
                ..vars::PRIMARY
            },
        );
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
        p.list.fill(name, 12.0, Paint::Solid(color));
        // Square off the bottom of the colored part where it meets the controls.
        p.list.fill(
            Rect::new(card.x, name.bottom() - 12.0, card.w, 12.0),
            0.0,
            Paint::Solid(color),
        );
        let controls = Rect::new(card.x, name.bottom(), card.w, CARD_CONTROLS_HEIGHT);
        p.list.fill(controls, 12.0, Paint::Solid(vars::SURFACE_ROW));
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
        let confirm_label = t.get(StringId::EditorRestoreConfirm).into_owned();
        let cancel_label = t.get(StringId::EditorCancel).into_owned();
        let button_style = p.style(&THEME.button, StateFlags::NONE);
        let confirm_w = (p.measure(&confirm_label, &button_style) + 56.0).max(140.0);
        let cancel_w = (p.measure(&cancel_label, &button_style) + 56.0).max(140.0);
        let y = dialog.bottom() - 32.0 - 50.0;
        let cancel = Rect::new(dialog.right() - 32.0 - cancel_w, y, cancel_w, 50.0);
        let confirm = Rect::new(cancel.x - 16.0 - confirm_w, y, confirm_w, 50.0);
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
            p.list.fill(dialog, 20.0, Paint::Solid(vars::SURFACE_ROW));
            p.list.ring(dialog, 20.0, 2.0, vars::BORDER);
            let title_style = TextStyle::new(28.0, 800);
            let title = p
                .text
                .ellipsize(
                    &t.get(StringId::EditorRestoreTitle),
                    &title_style,
                    width - 64.0,
                )
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
            let body = t.get(StringId::EditorRestoreBody).into_owned();
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
                line_y += 26.0;
            }
            self.draw_button(p, confirm, &confirm_label, CONFIRM_RESTORE);
            self.draw_outlined_button(p, cancel, &cancel_label, CONFIRM_CANCEL);
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
        match id {
            DONE => return self.finish_editing(),
            RESTORE => {
                self.editor.confirm = true;
                self.pending_focus = Some(PendingFocus::Id(CONFIRM_CANCEL));
                return vec![];
            }
            CONFIRM_RESTORE => {
                self.config = HomeConfig::first_use(&self.catalog);
                self.editor.confirm = false;
                self.pending_focus = Some(PendingFocus::Id(RESTORE));
                return save;
            }
            CONFIRM_CANCEL => {
                self.editor.confirm = false;
                self.pending_focus = Some(PendingFocus::Id(RESTORE));
                return vec![];
            }
            _ => {}
        }
        if (FAVORITE_BUTTON_BASE..ADD_APP_BASE).contains(&id) {
            let offset = id - FAVORITE_BUTTON_BASE;
            let (index, button) = ((offset / 3) as usize, offset % 3);
            let changed = match button {
                0 => self.config.move_favorite_left(index),
                1 => self.config.move_favorite_right(index),
                _ => self.config.remove_favorite(index),
            };
            if changed {
                let target = match button {
                    0 => FAVORITE_BUTTON_BASE + (index as u32 - 1) * 3,
                    1 => FAVORITE_BUTTON_BASE + (index as u32 + 1) * 3 + 1,
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
        self.screen = Screen::Main(Tab::Home);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
        vec![Effect::SaveConfig]
    }

    pub(super) fn back_from_editor(&mut self) -> Vec<Effect> {
        if self.editor.confirm {
            self.editor.confirm = false;
            self.pending_focus = Some(PendingFocus::Id(RESTORE));
            return vec![];
        }
        self.finish_editing()
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
fn wrap(p: &mut Painter, text: &str, style: &TextStyle, max_width: f32) -> Vec<String> {
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
