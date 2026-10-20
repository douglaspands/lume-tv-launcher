//! The home screen editor: sections, carousel and favorites.

use super::page::{self, LEFT, WIDTH};
use super::painter::{fade, Painter};
use super::{Effect, LumeApp, PendingFocus, Screen, Tab};
use crate::catalog::known::KnownApp;
use crate::home::config::{HomeConfig, Interval, Section, MAX_FAVORITES};
use crate::i18n::{Arg, PluralId, StringId};
use crate::icons;
use crate::render::Rect;
use crate::text::{Align, TextStyle};
use crate::theme::{vars, Color, Paint, StateFlags, THEME};
use crate::ui::focus::{FocusId, Focusable, Row};
use std::collections::HashMap;

pub const DONE: FocusId = 4000;
pub const RESTORE: FocusId = 4001;
pub(super) const SECTION_BASE: FocusId = 4100;
pub(super) const INTERVAL_BASE: FocusId = 4200;
pub(super) const CAROUSEL_APP_BASE: FocusId = 4300;
pub(super) const FAVORITE_BUTTON_BASE: FocusId = 4400;
pub(super) const ADD_APP_BASE: FocusId = 4600;
pub const CONFIRM_RESTORE: FocusId = 4900;
pub const CONFIRM_CANCEL: FocusId = 4901;

const ROW_HEIGHT: f32 = 71.0;
const ROW_GAP: f32 = 17.0;
const PANEL_GAP: f32 = 16.0;
const CHIP_HEIGHT: f32 = 42.0;
const CHIP_GAP: f32 = 14.0;
const CARD_WIDTH: f32 = 168.0;
const CARD_NAME_HEIGHT: f32 = 56.0;
const CARD_CONTROLS_HEIGHT: f32 = 60.0;
const CARD_GAP: f32 = 18.0;
const BUTTON_SIZE: f32 = 48.0;
const TOP: f32 = 162.0;
const BOTTOM_PADDING: f32 = 120.0;

/// What a section row's three buttons do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SectionAction {
    Up = 0,
    Down = 1,
    Hide = 2,
}

fn section_index(section: Section) -> u32 {
    match section {
        Section::Carousel => 0,
        Section::Favorites => 1,
    }
}

pub(super) fn section_button(section: Section, action: SectionAction) -> FocusId {
    SECTION_BASE + section_index(section) * 10 + action as u32
}

#[derive(Clone, Debug, Default)]
pub(super) struct EditorState {
    /// The "Restore defaults" confirmation is open.
    pub confirm: bool,
    /// When the "at least one app" hint stops being highlighted.
    pub hint_until_ms: u64,
}

/// An app that can be switched on or off in the carousel.
struct CarouselChoice {
    package: String,
    label: String,
    color: Color,
}

impl LumeApp {
    pub(super) fn open_editor(&mut self) {
        self.screen = Screen::Editor;
        self.editor = EditorState::default();
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(DONE));
        self.focus.set_rows(vec![]);
    }

    /// The apps offered in "Apps in the carousel": the known streaming apps that are installed,
    /// every favorite, and anything already selected so it can be switched off.
    fn carousel_choices(&self) -> Vec<CarouselChoice> {
        let mut packages: Vec<String> = Vec::new();
        let add = |package: &str, packages: &mut Vec<String>| {
            if self.catalog.contains(package) && !packages.iter().any(|p| p == package) {
                packages.push(package.to_string());
            }
        };
        for known in KnownApp::DEFAULT_FAVORITES {
            for package in known.packages() {
                add(package, &mut packages);
            }
        }
        for package in self
            .config
            .favorites
            .iter()
            .chain(&self.config.carousel_apps)
        {
            add(package, &mut packages);
        }
        packages
            .into_iter()
            .filter_map(|package| {
                let app = self.catalog.get(&package)?;
                Some(CarouselChoice {
                    package: package.clone(),
                    label: app.label.clone(),
                    color: app.color,
                })
            })
            .collect()
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
        let visible = p.display.visible_rect();
        if draw {
            p.list.fill(
                visible,
                0.0,
                THEME.app.base.background.unwrap_or(Paint::Solid(vars::BG)),
            );
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

        let mut y = TOP;
        let sections: Vec<Section> = self.config.sections.iter().map(|s| s.section).collect();
        let last = sections.len() - 1;
        for (index, section) in sections.into_iter().enumerate() {
            y = self.section_block(
                p,
                &mut layout,
                section,
                index == 0,
                index == last,
                y,
                scroll_y,
                draw,
            );
        }

        // "Restore defaults" at the bottom right.
        y += 24.0;
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

    /// One section: its row, and the panel below it.
    #[allow(clippy::too_many_arguments)]
    fn section_block(
        &mut self,
        p: &mut Painter,
        layout: &mut Layout,
        section: Section,
        first: bool,
        last: bool,
        top: f32,
        scroll_y: f32,
        draw: bool,
    ) -> f32 {
        let t = self.translator;
        let row = Rect::new(LEFT, top, WIDTH, ROW_HEIGHT);
        let buttons_y = top + (ROW_HEIGHT - BUTTON_SIZE) / 2.0;
        let button = |i: f32| {
            Rect::new(
                LEFT + 16.0 + i * (BUTTON_SIZE + 8.0),
                buttons_y,
                BUTTON_SIZE,
                BUTTON_SIZE,
            )
        };
        let (up, down, hide) = (button(0.0), button(1.0), button(2.0));
        let (up_id, down_id, hide_id) = (
            section_button(section, SectionAction::Up),
            section_button(section, SectionAction::Down),
            section_button(section, SectionAction::Hide),
        );
        let mut items = vec![
            if first {
                Focusable::disabled(up_id, up)
            } else {
                Focusable::new(up_id, up)
            },
            if last {
                Focusable::disabled(down_id, down)
            } else {
                Focusable::new(down_id, down)
            },
            Focusable::new(hide_id, hide),
        ];
        for (id, rect) in [(up_id, up), (down_id, down), (hide_id, hide)] {
            layout.rects.insert(id, rect);
        }

        // Interval chips of the carousel, at the right end of its row.
        let mut interval_chips: Vec<(Interval, Rect)> = Vec::new();
        if section == Section::Carousel {
            let chip_style = p.style(&THEME.chip, StateFlags::NONE);
            let mut x = LEFT + WIDTH - 16.0;
            for (i, interval) in Interval::ALL.iter().enumerate().rev() {
                let label = self.interval_label(*interval);
                let width = p.measure(&label, &chip_style) + 2.0 * chip_style.padding[1];
                x -= width;
                let rect = Rect::new(x, top + (ROW_HEIGHT - 33.0) / 2.0, width, 33.0);
                interval_chips.push((*interval, rect));
                layout.rects.insert(INTERVAL_BASE + i as u32, rect);
                x -= 8.0;
            }
            interval_chips.reverse();
            for (i, (_, rect)) in interval_chips.iter().enumerate() {
                items.push(Focusable::new(INTERVAL_BASE + i as u32, *rect));
            }
        }
        layout.row(items);

        let visible_section = self.config.is_visible(section);
        let number = self.config.position(section) + 1;
        let (title, summary) = match section {
            Section::Carousel => {
                let count = self.config.carousel_apps.len() as i64;
                (
                    t.get(StringId::EditorSectionCarousel).into_owned(),
                    t.plural(PluralId::EditorCarouselSummary, count, &[Arg::Int(count)]),
                )
            }
            Section::Favorites => {
                let names: Vec<&str> = self
                    .config
                    .favorites
                    .iter()
                    .filter_map(|package| self.catalog.get(package).map(|a| a.label.as_str()))
                    .collect();
                let summary = if names.is_empty() {
                    t.get(StringId::EditorFavoritesNone).into_owned()
                } else {
                    names.join(", ")
                };
                (
                    t.get(StringId::EditorSectionFavorites).into_owned(),
                    summary,
                )
            }
        };
        let hidden = (!visible_section).then(|| t.get(StringId::EditorSectionHidden).into_owned());
        if draw {
            self.draw_section_row(
                p,
                row.translate(0.0, -scroll_y),
                [up, down, hide],
                [up_id, down_id, hide_id],
                (first, last),
                visible_section,
                number,
                &title,
                &summary,
                hidden.as_deref(),
                &interval_chips,
                scroll_y,
                section == Section::Carousel,
            );
        }

        let panel_top = top + ROW_HEIGHT + PANEL_GAP;
        let bottom = match section {
            Section::Carousel => self.carousel_panel(p, layout, panel_top, scroll_y, draw),
            Section::Favorites => self.favorites_panel(p, layout, panel_top, scroll_y, draw),
        };
        bottom + ROW_GAP
    }

    fn interval_label(&self, interval: Interval) -> String {
        let t = self.translator;
        let seconds = interval.seconds();
        if seconds >= 60 {
            t.format(
                StringId::IntervalMinutes,
                &[Arg::Int(i64::from(seconds / 60))],
            )
        } else {
            t.format(StringId::IntervalSeconds, &[Arg::Int(i64::from(seconds))])
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_section_row(
        &mut self,
        p: &mut Painter,
        row: Rect,
        buttons: [Rect; 3],
        ids_: [FocusId; 3],
        (first, last): (bool, bool),
        visible_section: bool,
        number: usize,
        title: &str,
        summary: &str,
        hidden: Option<&str>,
        intervals: &[(Interval, Rect)],
        scroll_y: f32,
        is_carousel: bool,
    ) {
        let mut style = p.style(&THEME.editor_row, StateFlags::NONE);
        let any_focus = ids_.iter().any(|id| self.focus_progress(*id) > 0.0);
        if any_focus {
            style = p.focus_style(&THEME.editor_row, 0.0);
        }
        style.outline = None;
        style.opacity = if visible_section { 1.0 } else { 0.6 };
        p.draw_box(row, &style);

        let icons_for = [
            icons::CHEVRON_UP,
            icons::CHEVRON_DOWN,
            if visible_section {
                icons::EYE
            } else {
                icons::EYE_OFF
            },
        ];
        for (i, rect) in buttons.iter().enumerate() {
            let disabled = (i == 0 && first) || (i == 1 && last);
            let progress = self.focus_progress(ids_[i]);
            self.draw_round_button(
                p,
                rect.translate(0.0, -scroll_y),
                icons_for[i],
                progress,
                disabled,
                false,
            );
        }

        // The position number, then the title and the summary.
        let number_style = TextStyle::new(22.0, 700);
        p.text_with(
            &number.to_string(),
            Rect::new(buttons[2].right() + 14.0, row.y, 36.0, row.h),
            Align::Center,
            &number_style,
            vars::TEXT_MUTED,
        );
        let text_x = buttons[2].right() + 14.0 + 36.0 + 16.0;
        let dot_color = if visible_section {
            Color::WHITE
        } else {
            vars::TEXT_DISABLED
        };
        p.list.fill(
            Rect::new(text_x, row.y + 22.0, 11.0, 11.0),
            5.5,
            Paint::Solid(dot_color),
        );
        let title_x = text_x + 22.0;
        let title_style = TextStyle::new(24.0, 700);
        let max_width = if is_carousel {
            intervals.first().map_or(row.right(), |(_, r)| r.x) - 140.0 - title_x
        } else {
            row.right() - 40.0 - title_x
        };
        let shown_title = match hidden {
            Some(h) => format!("{title} ({})", h.to_lowercase()),
            None => title.to_string(),
        };
        let shown = p
            .text
            .ellipsize(&shown_title, &title_style, max_width)
            .into_owned();
        p.text.draw(
            p.list,
            &shown,
            title_x,
            row.y + 31.0,
            &title_style,
            fade(vars::TEXT, style.opacity),
        );
        let summary_style = TextStyle::new(16.0, 500);
        let shown = p
            .text
            .ellipsize(summary, &summary_style, max_width)
            .into_owned();
        p.text.draw(
            p.list,
            &shown,
            title_x,
            row.y + 55.0,
            &summary_style,
            fade(vars::TEXT_MUTED, style.opacity),
        );

        if is_carousel {
            let label = self
                .translator
                .get(StringId::EditorCarouselInterval)
                .into_owned();
            let label_style = TextStyle::new(16.0, 500);
            let width = p.text.measure(&label, &label_style);
            let first_chip = intervals.first().map_or(row.right(), |(_, r)| r.x);
            p.text.draw(
                p.list,
                &label,
                first_chip - 12.0 - width,
                row.y + 41.0,
                &label_style,
                vars::TEXT_MUTED,
            );
        }
        for (i, (interval, rect)) in intervals.iter().enumerate() {
            let selected = *interval == self.config.interval;
            let progress = self.focus_progress(INTERVAL_BASE + i as u32);
            let label = self.interval_label(*interval);
            self.draw_chip(
                p,
                rect.translate(0.0, -scroll_y),
                &label,
                selected,
                progress,
            );
        }
        let _ = scroll_y;
    }

    /// The panel under the carousel row: which apps rotate in it.
    fn carousel_panel(
        &mut self,
        p: &mut Painter,
        layout: &mut Layout,
        top: f32,
        scroll_y: f32,
        draw: bool,
    ) -> f32 {
        let t = self.translator;
        let choices = self.carousel_choices();
        let label = t.get(StringId::EditorCarouselApps).into_owned();
        let label_style = TextStyle {
            size: 15.0,
            weight: 800,
            letter_spacing: 1.2,
        };
        let label_width = p.text.measure(&label, &label_style);
        let chip_style = TextStyle::new(18.0, 700);

        let widths: Vec<f32> = choices
            .iter()
            .map(|c| {
                20.0 + 10.0 + 10.0 + p.text.measure(&c.label, &chip_style) + 14.0 + 16.0 + 18.0
            })
            .collect();
        let start_x = LEFT + 32.0 + label_width + 24.0;
        let positions = flow(
            &widths,
            start_x,
            LEFT + WIDTH - 32.0,
            CHIP_GAP,
            LEFT + 32.0,
            CHIP_HEIGHT + 12.0,
            top + 24.0,
        );
        let lines = positions.last().map_or(1, |q| q.line + 1);
        let height = 24.0 + lines as f32 * CHIP_HEIGHT + (lines as f32 - 1.0) * 12.0 + 24.0;
        let panel = Rect::new(LEFT, top, WIDTH, height.max(72.0));

        if draw {
            let style = p.style(&THEME.panel, StateFlags::NONE);
            p.draw_box(panel.translate(0.0, -scroll_y), &style);
            p.text.draw(
                p.list,
                &label,
                LEFT + 32.0,
                top + 24.0 + 28.0 - scroll_y,
                &label_style,
                vars::ACCENT,
            );
        }
        let mut row_items: Vec<Vec<Focusable>> = vec![Vec::new(); lines];
        for (i, (choice, pos)) in choices.iter().zip(&positions).enumerate() {
            let rect = Rect::new(pos.x, pos.y, widths[i], CHIP_HEIGHT);
            let id = CAROUSEL_APP_BASE + i as u32;
            layout.rects.insert(id, rect);
            row_items[pos.line].push(Focusable::new(id, rect));
            if draw {
                let selected = self.config.carousel_apps.contains(&choice.package);
                let progress = self.focus_progress(id);
                self.draw_app_chip(
                    p,
                    rect.translate(0.0, -scroll_y),
                    &choice.label,
                    choice.color,
                    selected,
                    progress,
                );
            }
        }
        for items in row_items {
            layout.row(items);
        }
        if draw {
            // The hint that one app has to stay.
            let hint = t.get(StringId::EditorCarouselMinOne).into_owned();
            let hint_style = TextStyle::new(16.0, 500);
            let width = p.text.measure(&hint, &hint_style);
            let flash = self.now_ms < self.editor.hint_until_ms;
            let color = if flash {
                vars::ACCENT
            } else {
                vars::TEXT_MUTED
            };
            p.text.draw(
                p.list,
                &hint,
                panel.right() - 32.0 - width,
                panel.bottom() - 24.0 - 13.0 - scroll_y,
                &hint_style,
                color,
            );
        }
        panel.bottom()
    }

    /// The panel under the favorites row: order, remove, and add.
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
                vars::ACCENT,
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

    fn draw_chip(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        label: &str,
        selected: bool,
        progress: f32,
    ) {
        let state = if selected {
            StateFlags::ACTIVE
        } else {
            StateFlags::NONE
        };
        let rest = p.style(&THEME.chip, state);
        let mut focused = state;
        focused.focus = true;
        let mut style = crate::ui::style::mix(&rest, &p.style(&THEME.chip, focused), progress);
        style.border_radius = rect.h / 2.0;
        p.draw_box(rect, &style);
        p.text(label, rect, Align::Center, &style);
    }

    fn draw_app_chip(
        &mut self,
        p: &mut Painter,
        rect: Rect,
        label: &str,
        color: Color,
        selected: bool,
        progress: f32,
    ) {
        let radius = rect.h / 2.0;
        let background = if selected {
            vars::SURFACE_FOCUS
        } else {
            vars::SURFACE
        };
        p.list.fill(rect, radius, Paint::Solid(background));
        let border = if selected {
            vars::ACCENT
        } else {
            vars::CONTROL
        };
        p.list.ring(rect, radius, 2.0, border);
        let dot = Rect::new(rect.x + 20.0, rect.center().1 - 5.0, 10.0, 10.0);
        p.list
            .fill(dot, 5.0, Paint::Solid(super::home::brighten(color)));
        let style = TextStyle::new(18.0, 700);
        let text_rect = Rect::new(
            dot.right() + 10.0,
            rect.y,
            rect.w - 20.0 - 10.0 - 10.0 - 14.0 - 16.0 - 8.0,
            rect.h,
        );
        p.text_with(label, text_rect, Align::Start, &style, vars::TEXT);
        if selected {
            p.icon(
                icons::CHECK,
                Rect::new(rect.right() - 20.0 - 16.0, rect.y, 16.0, rect.h),
                16.0,
                vars::TEXT,
            );
        }
        self.draw_focus_ring(p, rect, radius, progress);
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
                ..vars::TEXT
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
        for section in [Section::Carousel, Section::Favorites] {
            let (up, down, hide) = (
                section_button(section, SectionAction::Up),
                section_button(section, SectionAction::Down),
                section_button(section, SectionAction::Hide),
            );
            if id == up || id == down || id == hide {
                let moved = if id == up {
                    self.config.move_section_up(section)
                } else if id == down {
                    self.config.move_section_down(section)
                } else {
                    let visible = self.config.is_visible(section);
                    self.config.set_section_visible(section, !visible);
                    true
                };
                if moved {
                    // The focus follows the section to its new row. A section that reached an end
                    // has that button disabled, so the focus goes to the opposite one.
                    let target = if id == down && !self.config.can_move_section_down(section) {
                        up
                    } else if id == up && !self.config.can_move_section_up(section) {
                        down
                    } else {
                        id
                    };
                    self.pending_focus = Some(PendingFocus::Id(target));
                }
                return if moved { save } else { vec![] };
            }
        }
        if (INTERVAL_BASE..INTERVAL_BASE + Interval::ALL.len() as u32).contains(&id) {
            self.config
                .set_interval(Interval::ALL[(id - INTERVAL_BASE) as usize]);
            return save;
        }
        if (CAROUSEL_APP_BASE..FAVORITE_BUTTON_BASE).contains(&id) {
            let choices = self.carousel_choices();
            let Some(choice) = choices.get((id - CAROUSEL_APP_BASE) as usize) else {
                return vec![];
            };
            if self.config.toggle_carousel_app(&choice.package) {
                return save;
            }
            // The last app cannot be removed: highlight the hint.
            self.editor.hint_until_ms = self.now_ms + 1500;
            return vec![];
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
