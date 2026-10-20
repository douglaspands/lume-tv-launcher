//! Drawing helpers shared by all screens: theme styles, text and icons.

use crate::display::Display;
use crate::render::{DisplayList, Rect};
use crate::text::{Align, TextEngine, TextStyle};
use crate::theme::{Color, ComponentTheme, ComputedStyle, StateFlags, THEME};
use crate::ui::style::mix;

/// What a screen draws with for one frame.
pub struct Painter<'a> {
    pub list: &'a mut DisplayList,
    pub text: &'a mut TextEngine,
    pub display: Display,
}

impl Painter<'_> {
    /// The theme style of `component` with the given pseudo-classes.
    pub fn style(&self, component: &ComponentTheme, state: StateFlags) -> ComputedStyle {
        THEME.resolve(component, state)
    }

    /// The style of `component` between its resting look (0.0) and its focused look (1.0).
    pub fn focus_style(&self, component: &ComponentTheme, progress: f32) -> ComputedStyle {
        let rest = THEME.resolve(component, StateFlags::NONE);
        if progress <= 0.0 {
            return rest;
        }
        mix(
            &rest,
            &THEME.resolve(component, StateFlags::FOCUS),
            progress,
        )
    }

    /// Draws a box (shadow, background and outline) as the style describes it.
    pub fn draw_box(&mut self, rect: Rect, style: &ComputedStyle) {
        self.list.draw_box(rect, style);
    }

    /// The text style a computed style asks for.
    pub fn text_style(style: &ComputedStyle) -> TextStyle {
        TextStyle {
            size: style.font_size,
            weight: style.font_weight,
            letter_spacing: style.letter_spacing,
        }
    }

    /// One line of text inside `rect`, ellipsized, aligned and centered vertically on its capitals.
    pub fn text(&mut self, text: &str, rect: Rect, align: Align, style: &ComputedStyle) {
        let text_style = Painter::text_style(style);
        let color = fade(style.color, style.opacity);
        self.text
            .draw_line(self.list, text, rect, align, &text_style, color);
    }

    /// Like [`Painter::text`] with an explicit color and style.
    pub fn text_with(
        &mut self,
        text: &str,
        rect: Rect,
        align: Align,
        style: &TextStyle,
        color: Color,
    ) {
        self.text
            .draw_line(self.list, text, rect, align, style, color);
    }

    /// The width `text` takes in the given style.
    pub fn measure(&mut self, text: &str, style: &ComputedStyle) -> f32 {
        self.text.measure(text, &Painter::text_style(style))
    }

    /// An icon from the icon font, centered in `rect` at `size`.
    pub fn icon(&mut self, icon: char, rect: Rect, size: f32, color: Color) {
        self.text.draw_icon(self.list, icon, rect, size, color);
    }
}

pub fn fade(color: Color, opacity: f32) -> Color {
    Color {
        a: (f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8,
        ..color
    }
}
