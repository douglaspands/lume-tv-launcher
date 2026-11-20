//! The top bar: logo, tabs, inputs and settings buttons and clock.

use super::painter::{fade, Painter};
use super::{ids, Tab};
use crate::clock::WallClock;
use crate::display::CANVAS_WIDTH;
use crate::home::config::TabsAlign;
use crate::icons;
use crate::logo;
use crate::render::Rect;
use crate::theme::{vars, StateFlags};
use crate::ui::focus::{Focusable, Row};

/// Left and right margin of the whole interface.
pub const MARGIN: f32 = 64.0;

const LOGO: Rect = Rect::new(MARGIN, 24.0, 40.0, 40.0);
/// The wordmark's ink starts 16 after the logo: the two read as one lockup.
const WORDMARK_X: f32 = 118.0;
/// The wordmark and the clock share one size and one baseline: the bar's two ends.
const WORDMARK_BASELINE: f32 = 56.0;
/// The tabs are smaller (their underline marks them), with the text on the bar's axis (y=44).
const TAB_BASELINE: f32 = 52.0;
/// Where the first tab's text starts (48 after the wordmark), and the space between tab texts.
const TABS_X: f32 = 236.0;
const TAB_GAP: f32 = 32.0;
const CLOCK_RIGHT: f32 = 1216.0;
/// Between the gear and the clock. The buttons follow the clock's width, so with the glyph's own
/// margin the ink gap is always the one between the two buttons.
const CLOCK_GAP: f32 = 24.0;
/// Round buttons, centered on the bar's axis (y=44), with a gap between them.
const BUTTON: f32 = 48.0;
const BUTTON_GAP: f32 = 16.0;
/// The button glyphs. A line drawing reads larger than the solid logo, so the drawing is about
/// 80% of the logo's height (30 to 32 of 40), with room above and below; the icon font's stroke
/// gives it the stem of the tab text (see `LUCIDE_STROKE` in `scripts/subset-fonts.py`).
const GLYPH: f32 = 36.0;

/// Where the tabs and the buttons are, from the width of the tab labels and of the clock.
pub struct BarLayout {
    /// Left edge and width of the label of each tab.
    pub text: [(f32, f32); 2],
    pub inputs: Rect,
    pub gear: Rect,
}

impl BarLayout {
    pub fn new(
        p: &mut Painter,
        home_label: &str,
        apps_label: &str,
        clock: &WallClock,
        align: TabsAlign,
    ) -> BarLayout {
        let style = p.style(|t| &t.tab, StateFlags::ACTIVE);
        let home_width = p.measure(home_label, &style);
        let apps_width = p.measure(apps_label, &style);
        let clock_style = Painter::text_style(&p.style(|t| &t.clock, StateFlags::NONE));
        let clock_width = p.text.measure(&clock.text(), &clock_style);
        let gear = Rect::new(
            CLOCK_RIGHT - clock_width - CLOCK_GAP - BUTTON,
            44.0 - BUTTON / 2.0,
            BUTTON,
            BUTTON,
        );
        let inputs = Rect::new(gear.x - BUTTON_GAP - BUTTON, gear.y, BUTTON, BUTTON);
        let home_x = match align {
            TabsAlign::Left => TABS_X,
            TabsAlign::Center => {
                let total = home_width + TAB_GAP + apps_width;
                let ideal = (CANVAS_WIDTH - total) / 2.0;
                let min_x = TABS_X;
                let max_x = (inputs.x - TAB_GAP - total).max(min_x);
                ideal.clamp(min_x, max_x)
            }
        };
        BarLayout {
            text: [
                (home_x, home_width),
                (home_x + home_width + TAB_GAP, apps_width),
            ],
            inputs,
            gear,
        }
    }

    /// The focus rectangle of a tab: its text with a little room around it.
    pub fn rect(&self, tab: Tab) -> Rect {
        let (x, width) = self.text[tab as usize];
        Rect::new(x - 8.0, 28.0, width + 16.0, 32.0)
    }
}

/// The focus row of the top bar. With visible tabs, the active tab is where the focus lands from
/// below; without tabs, the focus lands on the first available button (`INPUTS` or `GEAR`).
pub fn row(tabs: &BarLayout, active: Tab, inputs: bool, tabs_visible: bool) -> Row {
    let mut items = Vec::with_capacity(4);
    if tabs_visible {
        items.push(Focusable::new(ids::TAB_HOME, tabs.rect(Tab::Home)));
        items.push(Focusable::new(ids::TAB_APPS, tabs.rect(Tab::Apps)));
    }
    if inputs {
        items.push(Focusable::new(ids::INPUTS, tabs.inputs));
    }
    items.push(Focusable::new(ids::GEAR, tabs.gear));
    let entry_up = if tabs_visible {
        match active {
            Tab::Home => ids::TAB_HOME,
            Tab::Apps => ids::TAB_APPS,
        }
    } else if inputs {
        ids::INPUTS
    } else {
        ids::GEAR
    };
    Row::new(items).with_entry_up(entry_up)
}

pub struct TopBar<'a> {
    pub tabs: &'a BarLayout,
    pub active: Tab,
    pub tabs_visible: bool,
    pub home_label: &'a str,
    pub apps_label: &'a str,
    pub clock: &'a WallClock,
    /// Whether the inputs button is shown.
    pub inputs: bool,
    /// Focus progress (0.0 to 1.0) of the tabs and the gear, by id.
    pub focus: &'a dyn Fn(u32) -> f32,
}

impl TopBar<'_> {
    /// How much the focus is in the top bar (tabs or gear), 0.0 to 1.0, animated with the focus.
    fn bar_focus(&self) -> f32 {
        [ids::TAB_HOME, ids::TAB_APPS, ids::INPUTS, ids::GEAR]
            .into_iter()
            .map(|id| (self.focus)(id))
            .fold(0.0f32, f32::max)
            .clamp(0.0, 1.0)
    }

    /// How much of the underline `tab` has (0.0 to 1.0). It sits under the focused tab while
    /// the focus is on the tabs, and under the active tab otherwise; the focus animation carries
    /// it from one to the other.
    fn underline_weight(&self, tab: Tab) -> f32 {
        let focus = |tab: Tab| match tab {
            Tab::Home => (self.focus)(ids::TAB_HOME),
            Tab::Apps => (self.focus)(ids::TAB_APPS),
        };
        let other = match tab {
            Tab::Home => Tab::Apps,
            Tab::Apps => Tab::Home,
        };
        let held = if tab == self.active {
            1.0 - focus(other)
        } else {
            0.0
        };
        focus(tab).max(held).clamp(0.0, 1.0)
    }

    pub fn draw(&self, p: &mut Painter) {
        logo::draw_mark(p.list, LOGO, p.primary());
        let wordmark = p.style(|t| &t.brand, StateFlags::NONE);
        p.text.draw(
            p.list,
            "Lume",
            WORDMARK_X,
            WORDMARK_BASELINE,
            &Painter::text_style(&wordmark),
            wordmark.color,
        );

        if self.tabs_visible {
            for (tab, label, id) in [
                (Tab::Home, self.home_label, ids::TAB_HOME),
                (Tab::Apps, self.apps_label, ids::TAB_APPS),
            ] {
                let selected = tab == self.active;
                let state = if selected {
                    StateFlags::ACTIVE
                } else {
                    StateFlags::NONE
                };
                let rest = p.style(|t| &t.tab, state);
                let progress = (self.focus)(id);
                let style = if progress > 0.0 {
                    let mut focused = state;
                    focused.focus = true;
                    crate::ui::style::mix(&rest, &p.style(|t| &t.tab, focused), progress)
                } else {
                    rest
                };
                let (x, width) = self.tabs.text[tab as usize];
                let text_style = Painter::text_style(&style);
                p.text
                    .draw(p.list, label, x, TAB_BASELINE, &text_style, style.color);
                let weight = self.underline_weight(tab);
                if weight > 0.0 {
                    // Primary accent while the focus is in the top bar (the tabs are being
                    // controlled), white once it goes down to the content.
                    let color = crate::ui::style::mix_color(
                        vars::SECONDARY,
                        p.primary_on_dark(),
                        self.bar_focus(),
                    );
                    p.list.fill(
                        Rect::new(x - 2.0, 64.0, width + 4.0, vars::TAB_UNDERLINE),
                        vars::TAB_UNDERLINE / 2.0,
                        crate::theme::Paint::Solid(fade(color, weight)),
                    );
                }
            }
        }

        if self.inputs {
            let progress = (self.focus)(ids::INPUTS);
            let button = p.focus_style(|t| &t.icon_button, progress);
            p.draw_box(self.tabs.inputs, &button);
            p.icon(icons::INPUTS, self.tabs.inputs, GLYPH, button.color);
        }

        let progress = (self.focus)(ids::GEAR);
        let gear = p.focus_style(|t| &t.icon_button, progress);
        p.draw_box(self.tabs.gear, &gear);
        p.icon(icons::SETTINGS, self.tabs.gear, GLYPH, gear.color);

        let clock_style = p.style(|t| &t.clock, StateFlags::NONE);
        let text_style = Painter::text_style(&clock_style);
        let text = self.clock.text();
        let width = p.text.measure(&text, &text_style);
        p.text.draw(
            p.list,
            &text,
            CLOCK_RIGHT - width,
            WORDMARK_BASELINE,
            &text_style,
            clock_style.color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextEngine;

    #[test]
    fn the_layout_is_on_the_ruler() {
        // WORDMARK_X (118) is an optical correction: it puts the wordmark's ink 16 after the logo.
        super::super::assert_on_ruler(&[
            ("MARGIN", MARGIN),
            ("LOGO.y", LOGO.y),
            ("LOGO.w", LOGO.w),
            ("WORDMARK_BASELINE", WORDMARK_BASELINE),
            ("TAB_BASELINE", TAB_BASELINE),
            ("TABS_X", TABS_X),
            ("TAB_GAP", TAB_GAP),
            ("CLOCK_RIGHT", CLOCK_RIGHT),
            ("CLOCK_GAP", CLOCK_GAP),
            ("BUTTON", BUTTON),
            ("BUTTON_GAP", BUTTON_GAP),
            ("GLYPH", GLYPH),
        ]);
    }

    #[test]
    fn the_clock_keeps_the_same_gap_to_the_gear_and_clears_the_tabs_at_any_time() {
        let mut text = TextEngine::new(1.0);
        let mut list = crate::render::DisplayList::new();
        let accent = crate::theme::AccentColor::default();
        let theme = crate::theme::Theme::for_accent(accent);
        let mut p = Painter {
            list: &mut list,
            text: &mut text,
            display: crate::display::Display::new(1280, 720),
            theme: &theme,
            accent: accent.palette(),
        };
        for align in [TabsAlign::Left, TabsAlign::Center] {
            for (home_label, apps_label) in [("Início", "Apps"), ("Home", "Apps")] {
                for is_24h in [true, false] {
                    for minute_of_day in 0..24 * 60 {
                        let clock = WallClock {
                            unix_secs: minute_of_day * 60,
                            utc_offset_secs: 0,
                            is_24h,
                        };
                        let bar = BarLayout::new(&mut p, home_label, apps_label, &clock, align);
                        let style = Painter::text_style(&p.style(|t| &t.clock, StateFlags::NONE));
                        let width = p.text.measure(&clock.text(), &style);
                        assert!((CLOCK_RIGHT - width - bar.gear.right() - CLOCK_GAP).abs() < 1e-3);
                        let (home_x, home_width) = bar.text[0];
                        let (apps_x, apps_width) = bar.text[1];
                        assert!(
                            home_x >= TABS_X - 1e-3,
                            "{align:?} {home_label}: home_x={home_x}"
                        );
                        assert!(
                            bar.inputs.x >= apps_x + apps_width + TAB_GAP - 1e-3,
                            "{align:?} {}: inputs.x={} apps_right={}",
                            clock.text(),
                            bar.inputs.x,
                            apps_x + apps_width
                        );
                        if align == TabsAlign::Center {
                            let center = (home_x + apps_x + apps_width) / 2.0;
                            assert!(
                                (center - CANVAS_WIDTH / 2.0).abs() < 1e-2,
                                "expected tabs centered at 640, got {center} ({home_label}/{apps_label} {})",
                                clock.text()
                            );
                            assert!((apps_x - (home_x + home_width + TAB_GAP)).abs() < 1e-3);
                        } else {
                            assert_eq!(home_x, TABS_X);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn tabless_row_omits_tabs_and_enters_inputs_or_gear() {
        let mut text = TextEngine::new(1.0);
        let mut list = crate::render::DisplayList::new();
        let accent = crate::theme::AccentColor::default();
        let theme = crate::theme::Theme::for_accent(accent);
        let mut p = Painter {
            list: &mut list,
            text: &mut text,
            display: crate::display::Display::new(1280, 720),
            theme: &theme,
            accent: accent.palette(),
        };
        let clock = WallClock {
            unix_secs: 0,
            utc_offset_secs: 0,
            is_24h: true,
        };
        let bar = BarLayout::new(&mut p, "Início", "Apps", &clock, TabsAlign::Left);

        let with_inputs = row(&bar, Tab::Home, true, false);
        let ids_with: Vec<u32> = with_inputs.items.iter().map(|i| i.id).collect();
        assert_eq!(ids_with, vec![ids::INPUTS, ids::GEAR]);
        assert_eq!(with_inputs.entry_up, Some(ids::INPUTS));

        let without_inputs = row(&bar, Tab::Home, false, false);
        let ids_without: Vec<u32> = without_inputs.items.iter().map(|i| i.id).collect();
        assert_eq!(ids_without, vec![ids::GEAR]);
        assert_eq!(without_inputs.entry_up, Some(ids::GEAR));
    }
}
