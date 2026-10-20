//! The top bar: logo, tabs, inputs and settings buttons and clock.

use super::painter::{fade, Painter};
use super::{ids, Tab};
use crate::clock::WallClock;
use crate::icons;
use crate::logo;
use crate::render::Rect;
use crate::text::TextStyle;
use crate::theme::{vars, StateFlags, THEME};
use crate::ui::focus::{Focusable, Row};

/// Left and right margin of the whole interface.
pub const MARGIN: f32 = 64.0;

const LOGO: Rect = Rect::new(MARGIN, 24.0, 40.0, 40.0);
const WORDMARK_X: f32 = 116.0;
const WORDMARK_BASELINE: f32 = 56.0;
const TAB_BASELINE: f32 = 51.0;
/// Where the first tab's text starts, and the space between tab texts.
const TABS_X: f32 = 228.0;
const TAB_GAP: f32 = 36.0;
const GEAR: Rect = Rect::new(1096.0, 20.0, 48.0, 48.0);
/// The inputs button sits one button (48 px) and a gap (8 px) to the left of the gear.
const INPUTS: Rect = Rect::new(1040.0, 20.0, 48.0, 48.0);
const CLOCK_RIGHT: f32 = 1216.0;
const CLOCK_BASELINE: f32 = 52.0;

/// Where the tabs are, from the width of their labels.
pub struct TabLayout {
    /// Left edge and width of the label of each tab.
    pub text: [(f32, f32); 2],
}

impl TabLayout {
    pub fn new(p: &mut Painter, home_label: &str, apps_label: &str) -> TabLayout {
        let style = p.style(&THEME.tab, StateFlags::ACTIVE);
        let home_width = p.measure(home_label, &style);
        let apps_width = p.measure(apps_label, &style);
        TabLayout {
            text: [
                (TABS_X, home_width),
                (TABS_X + home_width + TAB_GAP, apps_width),
            ],
        }
    }

    /// The focus rectangle of a tab: its text with a little room around it.
    pub fn rect(&self, tab: Tab) -> Rect {
        let (x, width) = self.text[tab as usize];
        Rect::new(x - 8.0, 28.0, width + 16.0, 30.0)
    }
}

/// The focus row of the top bar. The active tab is where the focus lands from below.
pub fn row(tabs: &TabLayout, active: Tab, inputs: bool) -> Row {
    let mut items = vec![
        Focusable::new(ids::TAB_HOME, tabs.rect(Tab::Home)),
        Focusable::new(ids::TAB_APPS, tabs.rect(Tab::Apps)),
    ];
    if inputs {
        items.push(Focusable::new(ids::INPUTS, INPUTS));
    }
    items.push(Focusable::new(ids::GEAR, GEAR));
    Row::new(items).with_entry_up(match active {
        Tab::Home => ids::TAB_HOME,
        Tab::Apps => ids::TAB_APPS,
    })
}

pub struct TopBar<'a> {
    pub tabs: &'a TabLayout,
    pub active: Tab,
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
        logo::draw_mark(p.list, LOGO, vars::PRIMARY, vars::BG);
        let wordmark = TextStyle {
            size: 28.0,
            weight: 800,
            letter_spacing: -0.5,
        };
        p.text.draw(
            p.list,
            "Lume",
            WORDMARK_X,
            WORDMARK_BASELINE,
            &wordmark,
            vars::TEXT,
        );

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
            let rest = p.style(&THEME.tab, state);
            let progress = (self.focus)(id);
            let style = if progress > 0.0 {
                let mut focused = state;
                focused.focus = true;
                crate::ui::style::mix(&rest, &p.style(&THEME.tab, focused), progress)
            } else {
                rest
            };
            let (x, width) = self.tabs.text[tab as usize];
            let text_style = Painter::text_style(&style);
            p.text
                .draw(p.list, label, x, TAB_BASELINE, &text_style, style.color);
            let weight = self.underline_weight(tab);
            if weight > 0.0 {
                // Blue while the focus is in the top bar (the tabs are being controlled), white once
                // it goes down to the content.
                let color =
                    crate::ui::style::mix_color(vars::SECONDARY, vars::PRIMARY, self.bar_focus());
                p.list.fill(
                    Rect::new(x - 2.0, 64.0, width + 4.0, vars::TAB_UNDERLINE),
                    vars::TAB_UNDERLINE / 2.0,
                    crate::theme::Paint::Solid(fade(color, weight)),
                );
            }
        }

        if self.inputs {
            let progress = (self.focus)(ids::INPUTS);
            let button = p.focus_style(&THEME.icon_button, progress);
            p.draw_box(INPUTS, &button);
            p.icon(icons::INPUTS, INPUTS, 27.0, button.color);
        }

        let progress = (self.focus)(ids::GEAR);
        let gear = p.focus_style(&THEME.icon_button, progress);
        p.draw_box(GEAR, &gear);
        p.icon(icons::SETTINGS, GEAR, 27.0, gear.color);

        let clock_style = p.style(&THEME.clock, StateFlags::NONE);
        let text_style = Painter::text_style(&clock_style);
        let text = self.clock.text();
        let width = p.text.measure(&text, &text_style);
        p.text.draw(
            p.list,
            &text,
            CLOCK_RIGHT - width,
            CLOCK_BASELINE,
            &text_style,
            clock_style.color,
        );
    }
}
