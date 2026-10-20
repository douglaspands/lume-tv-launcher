//! The lume settings screen and the About screen.

use super::page::{self, LEFT, WIDTH};
use super::painter::Painter;
use super::{ids, Effect, LumeApp, PendingFocus, Screen, Tab};
use crate::i18n::{Arg, StringId};
use crate::icons;
use crate::licenses;
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{vars, StateFlags, THEME};
use crate::ui::focus::{Focusable, Row};

/// Where the source code lives, shown on the About screen.
pub const SOURCE_URL: &str = "https://github.com/lume-launcher/lume";
/// The version of the app, from the Cargo package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const ITEM_WIDTH: f32 = 640.0;
const ITEMS_TOP: f32 = 162.0;
const ITEM_PITCH: f32 = 88.0;
const ABOUT_TEXT_TOP: f32 = 170.0;
const ABOUT_VIEW_TOP: f32 = 140.0;
const ABOUT_STEP: f32 = 96.0;

pub(super) const ITEM_HOME_SCREEN: u32 = ids::SETTINGS_BASE;
pub(super) const ITEM_ANDROID: u32 = ids::SETTINGS_BASE + 1;
pub(super) const ITEM_ABOUT: u32 = ids::SETTINGS_BASE + 2;

impl LumeApp {
    pub(super) fn open_settings(&mut self) {
        if let Screen::Main(tab) = self.screen {
            self.settings_return = tab;
        }
        self.screen = Screen::Settings;
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
            (ITEM_ANDROID, t.get(StringId::SettingsAndroid).into_owned()),
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
            p.icon(icons::CHEVRON_RIGHT, chevron, 24.0, vars::TEXT_MUTED);
        }
    }

    pub(super) fn activate_settings(&mut self, id: u32) -> Vec<Effect> {
        match id {
            ITEM_HOME_SCREEN => {
                self.open_editor();
                vec![]
            }
            ITEM_ANDROID => vec![Effect::OpenAndroidSettings],
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
        self.screen = Screen::Main(self.settings_return);
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(match self.settings_return {
            Tab::Home => ids::GEAR,
            Tab::Apps => ids::GEAR,
        }));
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
