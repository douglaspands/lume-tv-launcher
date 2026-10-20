//! The lume settings screen and the About screen.

use super::page::{self, LEFT, WIDTH};
use super::painter::Painter;
use super::{ids, Effect, LumeApp, PendingFocus, Screen, Tab};
use crate::i18n::{Arg, StringId};
use crate::icons;
use crate::licenses;
use crate::render::Rect;
use crate::stats;
use crate::text::{Align, TextStyle};
use crate::theme::Paint;
use crate::theme::{vars, StateFlags, THEME};
use crate::ui::focus::{Focusable, Row};

/// Where the source code lives, shown on the About screen.
pub const SOURCE_URL: &str = "https://github.com/lume-launcher/lume";
/// The version of the app, from the Cargo package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The settings screen is split in two equal columns with a 48 px gutter: items and statistics.
const COLUMN_WIDTH: f32 = (WIDTH - 48.0) / 2.0;
const ITEM_WIDTH: f32 = COLUMN_WIDTH;
const ITEMS_TOP: f32 = page::CONTENT_TOP;
const ITEM_PITCH: f32 = 88.0;
const ABOUT_TEXT_TOP: f32 = 190.0;
const ABOUT_VIEW_TOP: f32 = 150.0;
const ABOUT_STEP: f32 = 96.0;
const PANEL_X: f32 = LEFT + COLUMN_WIDTH + 48.0;
const PANEL_WIDTH: f32 = COLUMN_WIDTH;
const PANEL_HEIGHT: f32 = 312.0;

pub(super) const ITEM_HOME_SCREEN: u32 = ids::SETTINGS_BASE;
pub(super) const ITEM_ABOUT: u32 = ids::SETTINGS_BASE + 2;

impl LumeApp {
    pub(super) fn open_settings(&mut self) {
        self.screen = Screen::Settings;
        self.stats = None;
        self.cpu_use = None;
        self.stats_at = None;
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::FirstContent);
        // The settings screen has no top bar: its first row is the first item.
        self.focus.set_rows(vec![]);
    }

    pub(super) fn draw_settings(&mut self, p: &mut Painter) {
        let t = self.translator;
        let labels = [
            (
                ITEM_HOME_SCREEN,
                t.get(StringId::SettingsHomeScreen).into_owned(),
            ),
            (ITEM_ABOUT, t.get(StringId::SettingsAbout).into_owned()),
        ];
        let rects: Vec<Rect> = (0..labels.len())
            .map(|i| Rect::new(LEFT, ITEMS_TOP + i as f32 * ITEM_PITCH, ITEM_WIDTH, 72.0))
            .collect();
        let rows = labels
            .iter()
            .zip(&rects)
            .map(|((id, _), rect)| Row::new(vec![Focusable::new(*id, *rect)]))
            .collect();
        self.update_focus(rows);

        page::draw_header(
            p,
            &[&t.get(StringId::SettingsTitle)],
            &t.get(StringId::SettingsTitle),
            None,
            0.0,
        );
        for ((id, label), rect) in labels.iter().zip(&rects) {
            let progress = self.focus_progress(*id);
            let style = p.focus_style(&THEME.settings_item, progress);
            p.draw_box(*rect, &style);
            let inner = Rect::new(
                rect.x + style.padding[3],
                rect.y,
                rect.w - style.padding[3] - 70.0,
                rect.h,
            );
            p.text(label, inner, Align::Start, &style);
            let chevron = Rect::new(rect.right() - 28.0 - 28.0, rect.y, 28.0, rect.h);
            p.icon(icons::CHEVRON_RIGHT, chevron, 24.0, style.color);
        }
        self.draw_stats(p);

        // The installed version, at the bottom right.
        let version = t.format(StringId::AboutVersion, &[Arg::Str(VERSION)]);
        let style = p.style(&THEME.breadcrumb, StateFlags::NONE);
        let bottom = p.display.safe_area().bottom();
        let line = Rect::new(LEFT, bottom - 40.0, WIDTH, 24.0);
        p.text(&version, line, Align::End, &style);
    }

    /// The TV statistics card, right of the items: no focus, no animation.
    fn draw_stats(&self, p: &mut Painter) {
        let t = self.translator;
        let comma = t.language().starts_with("pt");
        let panel = Rect::new(PANEL_X, ITEMS_TOP, PANEL_WIDTH, PANEL_HEIGHT);
        let style = p.style(&THEME.panel, StateFlags::NONE);
        p.draw_box(panel, &style);
        let inner = Rect::new(panel.x + 32.0, panel.y + 24.0, panel.w - 64.0, 20.0);
        let heading = TextStyle {
            size: 15.0,
            weight: 800,
            letter_spacing: 1.2,
        };
        p.text_with(
            &t.get(StringId::StatsTitle),
            inner,
            Align::Start,
            &heading,
            vars::TEXT_MUTED,
        );

        let stats = self.stats.as_ref();
        let rows = [
            (
                StringId::StatsMemory,
                stats.map(|s| s.memory_used()),
                stats.map(|s| {
                    format!(
                        "{} / {}",
                        stats::format_size(s.memory_used_bytes(), comma),
                        stats::format_size(s.memory_total_bytes(), comma)
                    )
                }),
            ),
            (
                StringId::StatsDisk,
                stats.map(|s| s.disk_used()),
                stats.map(|s| {
                    format!(
                        "{} / {}",
                        stats::format_size(s.disk_used_bytes(), comma),
                        stats::format_size(s.disk_total, comma)
                    )
                }),
            ),
            (
                StringId::StatsCpu,
                self.cpu_use,
                self.cpu_use.map(|u| format!("{:.0}%", u * 100.0)),
            ),
        ];
        let label = p.style(&THEME.breadcrumb, StateFlags::NONE);
        let mut value_style = label;
        value_style.color = vars::TEXT;
        value_style.font_weight = 700;
        for (index, (id, fraction, value)) in rows.into_iter().enumerate() {
            let top = panel.y + 72.0 + index as f32 * 80.0;
            let line = Rect::new(inner.x, top, inner.w, 24.0);
            p.text(&t.get(id), line, Align::Start, &label);
            // A dash stands for what the system would not tell.
            let value = value.unwrap_or_else(|| "—".to_string());
            p.text(&value, line, Align::End, &value_style);
            let bar = Rect::new(inner.x, top + 36.0, inner.w, 8.0);
            p.list.fill(bar, 4.0, Paint::Solid(vars::CONTROL));
            if let Some(fraction) = fraction {
                let filled = Rect::new(bar.x, bar.y, (bar.w * fraction).max(8.0), bar.h);
                p.list.fill(filled, 4.0, Paint::Solid(vars::PRIMARY));
            }
        }
    }

    pub(super) fn activate_settings(&mut self, id: u32) -> Vec<Effect> {
        match id {
            ITEM_HOME_SCREEN => {
                self.open_editor(false);
                vec![]
            }
            ITEM_ABOUT => {
                self.screen = Screen::About;
                self.about_scroll = 0.0;
                self.focus.set_rows(vec![]);
                vec![]
            }
            _ => vec![],
        }
    }

    pub(super) fn back_from_settings(&mut self) {
        self.screen = Screen::Main(Tab::Apps);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(ids::GRID_SETTINGS));
    }

    /// Scrolls the About text with the up and down keys.
    pub(super) fn scroll_about(&mut self, delta: f32) {
        self.about_scroll = (self.about_scroll + delta * ABOUT_STEP).max(0.0);
    }

    pub(super) fn draw_about(&mut self, p: &mut Painter) {
        let t = self.translator;
        self.update_focus(vec![]);
        let visible = p.display.visible_rect();

        // Build the text, then limit the scrolling to its length.
        let lines = licenses::lines();
        let mut blocks: Vec<(String, f32, bool)> = vec![
            (
                t.format(StringId::AboutVersion, &[Arg::Str(VERSION)]),
                24.0,
                true,
            ),
            (t.get(StringId::AboutLicense).into_owned(), 20.0, false),
            (
                t.format(StringId::AboutSource, &[Arg::Str(SOURCE_URL)]),
                20.0,
                false,
            ),
            (String::new(), 16.0, false),
            (t.get(StringId::AboutThirdParty).into_owned(), 24.0, true),
        ];
        blocks.extend(lines.into_iter().map(|line| (line, 16.0, false)));
        let heights: Vec<f32> = blocks.iter().map(|(_, size, _)| size * 1.9).collect();
        let total: f32 = heights.iter().sum();
        let view = (visible.bottom() - ABOUT_VIEW_TOP).max(1.0);
        let max_scroll = (ABOUT_TEXT_TOP + total + 40.0 - ABOUT_VIEW_TOP - view).max(0.0);
        self.about_scroll = self.about_scroll.min(max_scroll);
        let scroll = self.about_scroll;

        p.list.push_clip(Rect::new(
            visible.x,
            ABOUT_VIEW_TOP,
            visible.w,
            visible.bottom() - ABOUT_VIEW_TOP,
        ));
        let mut y = ABOUT_TEXT_TOP - scroll;
        for ((text, size, heading), height) in blocks.iter().zip(&heights) {
            let mut style = p.style(&THEME.breadcrumb, StateFlags::NONE);
            style.font_size = *size;
            if *heading {
                style.color = vars::TEXT;
                style.font_weight = 700;
            }
            let rect = Rect::new(LEFT, y, WIDTH, *height);
            p.text(text, rect, Align::Start, &style);
            y += height;
        }
        p.list.pop_clip();

        // The scrolling text is clipped below the header, so the header needs no cover.
        page::draw_header(
            p,
            &[
                &t.get(StringId::SettingsTitle),
                &t.get(StringId::SettingsAbout),
            ],
            &t.get(StringId::AboutTitle),
            None,
            0.0,
        );
    }
}
