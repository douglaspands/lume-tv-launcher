//! Drawing helpers shared by all screens: theme styles, text and icons.

use crate::display::Display;
use crate::render::{DisplayList, Rect};
use crate::text::{Align, TextEngine, TextStyle};
use crate::theme::{AccentPalette, Color, ComponentTheme, ComputedStyle, Paint, StateFlags, Theme};
use crate::ui::style::mix;

/// Width of the subtle outer contrast ring drawn around dark-on-dark focused elements (`Black`).
pub const CONTRAST_RING_WIDTH: f32 = 1.5;

/// The dimming of the wallpaper on the text screens: the theme's `--bg` at about 78% (the value
/// was tuned with a contrast measurement, see the tests). It is not a layer: the wallpaper is
/// drawn already scaled by the remaining 22% over the `--bg` the frame is cleared to (see
/// [`Painter::wallpaper`]), which costs nothing extra on a weak GPU.
pub const TEXT_SCREEN_DIM: Color = Color {
    r: 15,
    g: 23,
    b: 30,
    a: 200,
};

/// What a screen draws with for one frame.
pub struct Painter<'a> {
    pub list: &'a mut DisplayList,
    pub text: &'a mut TextEngine,
    pub display: Display,
    pub theme: &'a Theme,
    pub accent: AccentPalette,
}

impl Painter<'_> {
    /// The active primary accent color (`--primary`).
    pub fn primary(&self) -> Color {
        self.accent.primary
    }

    /// The text/icon color that sits on top of [`primary`](Self::primary) (`--on-primary`).
    pub fn on_primary(&self) -> Color {
        self.accent.on_primary
    }

    /// Accent color safe for text, underlines and progress bars drawn directly on dark surfaces.
    /// Uses `--on-primary` (`#F1F4F9`) when `--primary` itself is dark (`Black`).
    pub fn primary_on_dark(&self) -> Color {
        if self.accent.contrast_ring.is_some() {
            self.accent.on_primary
        } else {
            self.accent.primary
        }
    }

    /// The resolved runtime palette for the active accent color.
    pub fn accent(&self) -> AccentPalette {
        self.accent
    }
    /// Draws the wallpaper over the whole visible area, cropped to cover it (the image keeps its
    /// proportion). It goes first in every frame, behind everything else.
    ///
    /// `dimmed` darkens it towards `--bg` by [`TEXT_SCREEN_DIM`] without a second full-screen layer
    /// (that layer cost more than half of the frames on the TV): the image and the stars are
    /// drawn at the remaining share, and the frame is cleared to `--bg`, so the premultiplied
    /// blend gives `image * (1 - dim) + bg * dim`.
    pub fn wallpaper(&mut self, dimmed: bool) {
        self.wallpaper_at(
            crate::wallpaper::WallpaperId::default(),
            if dimmed { 1.0 } else { 0.0 },
            1.0,
        );
    }

    /// The wallpaper at `dim_amount` (0.0 vivid to 1.0 dimmed by [`TEXT_SCREEN_DIM`]) and
    /// `intensity` (0.0, only the `--bg` the frame is cleared to, to 1.0, full draw): the splash
    /// reveals it and Peek mode dims it progressively this way, by the color it is drawn with and
    /// not with a layer over it.
    pub fn wallpaper_at(
        &mut self,
        id: crate::wallpaper::WallpaperId,
        dim_amount: f32,
        intensity: f32,
    ) {
        let dim = dim_amount.clamp(0.0, 1.0);
        let max_dim = f32::from(TEXT_SCREEN_DIM.a) / 255.0;
        let keep = (1.0 - max_dim * dim) * intensity.clamp(0.0, 1.0);
        if keep <= 0.0 {
            return;
        }
        let visible = self.display.visible_rect();
        let wallpaper = id.wallpaper();
        let uv = cover_uv(
            wallpaper.width as f32 / wallpaper.height as f32,
            visible.w / visible.h,
        );
        self.list
            .wallpaper_with_id(visible, uv, fade(Color::WHITE, keep), id);
        // The stars are drawn as points of 1 or 2 pixels of the render buffer, over the image, so
        // they stay as sharp as the interface on a 4K screen (which renders at 1080p).
        let pixel = 1.0 / self.display.scale;
        for &(sx, sy, brightness) in wallpaper.stars {
            let (u, v) = (f32::from(sx) / 1920.0, f32::from(sy) / 1080.0);
            if u < uv.x || u >= uv.right() || v < uv.y || v >= uv.bottom() {
                continue;
            }
            let size = if brightness > 204 { 2.0 * pixel } else { pixel };
            let x = self.display.snap(visible.x + (u - uv.x) / uv.w * visible.w);
            let y = self.display.snap(visible.y + (v - uv.y) / uv.h * visible.h);
            self.list.fill(
                Rect::new(x, y, size, size),
                0.0,
                Paint::Solid(Color {
                    r: 200,
                    g: 210,
                    b: 235,
                    a: (f32::from(brightness) * keep).round() as u8,
                }),
            );
        }
    }

    /// The theme style of `component` with the given pseudo-classes, resolved from the active theme.
    pub fn style(
        &self,
        component: impl FnOnce(&Theme) -> &ComponentTheme,
        state: StateFlags,
    ) -> ComputedStyle {
        self.theme.resolve(component(self.theme), state)
    }

    /// The style of `component` between its resting look (0.0) and its focused look (1.0).
    pub fn focus_style(
        &self,
        component: impl FnOnce(&Theme) -> &ComponentTheme,
        progress: f32,
    ) -> ComputedStyle {
        let comp = component(self.theme);
        let rest = self.theme.resolve(comp, StateFlags::NONE);
        if progress <= 0.0 {
            return rest;
        }
        mix(
            &rest,
            &self.theme.resolve(comp, StateFlags::FOCUS),
            progress,
        )
    }

    /// Draws the subtle outer contrast ring around `rect` (with corner `radius`) when the active
    /// accent requires one (`Black`), scaled by `opacity` (0.0..=1.0).
    pub fn draw_contrast_ring(&mut self, rect: Rect, radius: f32, opacity: f32) {
        let Some(ring_color) = self.accent.contrast_ring else {
            return;
        };
        let alpha = opacity.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let outer = rect.grow(CONTRAST_RING_WIDTH);
        self.list.ring(
            outer,
            radius + CONTRAST_RING_WIDTH,
            CONTRAST_RING_WIDTH,
            fade(ring_color, alpha),
        );
    }

    fn maybe_draw_contrast_ring(&mut self, rect: Rect, style: &ComputedStyle) {
        let Some(outline) = style.outline else {
            return;
        };
        if self.accent.contrast_ring.is_none() || outline.width <= 0.0 || outline.color.a == 0 {
            return;
        }
        let primary = self.accent.primary;
        if (outline.color.r, outline.color.g, outline.color.b) != (primary.r, primary.g, primary.b)
        {
            return;
        }
        let scaled = rect.scaled_about_center(style.scale);
        let radius = style.border_radius * style.scale;
        let grow = style.outline_offset + outline.width;
        let alpha = (f32::from(outline.color.a) / 255.0) * style.opacity.clamp(0.0, 1.0);
        self.draw_contrast_ring(scaled.grow(grow), radius + grow, alpha);
    }

    /// Draws a box (shadow, background and outline) as the style describes it.
    pub fn draw_box(&mut self, rect: Rect, style: &ComputedStyle) {
        self.list.draw_box(rect, style);
        self.maybe_draw_contrast_ring(rect, style);
    }

    /// Like [`draw_box`](Self::draw_box), for a box an image is drawn over afterwards.
    pub fn draw_box_under(&mut self, rect: Rect, style: &ComputedStyle, under: f32) {
        self.list.draw_box_under(rect, style, under);
        self.maybe_draw_contrast_ring(rect, style);
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

/// The part of an image (in 0..1 texture coordinates) that covers a rectangle of the given
/// proportions (width over height), centered: the image is cropped, never stretched or letterboxed.
pub fn cover_uv(image_aspect: f32, target_aspect: f32) -> Rect {
    if target_aspect > image_aspect {
        let h = image_aspect / target_aspect;
        Rect::new(0.0, (1.0 - h) / 2.0, 1.0, h)
    } else {
        let w = target_aspect / image_aspect;
        Rect::new((1.0 - w) / 2.0, 0.0, w, 1.0)
    }
}

pub fn fade(color: Color, opacity: f32) -> Color {
    Color {
        a: (f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8,
        ..color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matching_proportion_uses_the_whole_image() {
        assert_eq!(
            cover_uv(16.0 / 9.0, 16.0 / 9.0),
            Rect::new(0.0, 0.0, 1.0, 1.0)
        );
    }

    #[test]
    fn a_wider_target_crops_top_and_bottom_and_a_taller_one_crops_the_sides() {
        let wide = cover_uv(16.0 / 9.0, 21.0 / 9.0);
        assert_eq!((wide.x, wide.w), (0.0, 1.0));
        assert!(
            (wide.h - (16.0 / 21.0)).abs() < 1e-5 && (wide.y * 2.0 + wide.h - 1.0).abs() < 1e-5
        );
        let tall = cover_uv(16.0 / 9.0, 4.0 / 3.0);
        assert_eq!((tall.y, tall.h), (0.0, 1.0));
        assert!((tall.w - (4.0 / 3.0) / (16.0 / 9.0)).abs() < 1e-5);
        assert!((tall.x * 2.0 + tall.w - 1.0).abs() < 1e-5);
    }

    #[test]
    fn black_accent_emits_outer_contrast_ring_only_on_focused_dark_surface_elements() {
        use crate::render::{Cmd, RectKind};
        use crate::theme::AccentColor;

        let rect = Rect::new(100.0, 100.0, 172.0, 97.0);
        let black_ring = AccentColor::Black
            .palette()
            .contrast_ring
            .expect("Black has contrast_ring");

        let count_contrast_rings =
            |accent: AccentColor, comp: fn(&Theme) -> &ComponentTheme, progress: f32| -> usize {
                let theme = Theme::for_accent(accent);
                let mut list = DisplayList::new();
                let mut text = TextEngine::new(1.0);
                let mut p = Painter {
                    list: &mut list,
                    text: &mut text,
                    display: Display::new(1280, 720),
                    theme: &theme,
                    accent: accent.palette(),
                };
                let style = p.focus_style(comp, progress);
                p.draw_box(rect, &style);
                list.cmds()
                    .iter()
                    .filter(|cmd| {
                        matches!(
                            cmd,
                            Cmd::Rect(r)
                                if matches!(
                                    r.kind,
                                    RectKind::Ring { width, color }
                                        if (width - CONTRAST_RING_WIDTH).abs() < 1e-4
                                            && color == black_ring
                                )
                        )
                    })
                    .count()
            };

        // Black emits exactly 1 outer contrast ring when focused on a dark-surface element.
        assert_eq!(
            count_contrast_rings(AccentColor::Black, |t| &t.tile, 1.0),
            1
        );
        assert_eq!(
            count_contrast_rings(AccentColor::Black, |t| &t.button, 1.0),
            1
        );
        // Unfocused elements do not get a contrast ring.
        assert_eq!(
            count_contrast_rings(AccentColor::Black, |t| &t.tile, 0.0),
            0
        );
        // Light-card menu items do not get a dark-surface contrast ring.
        assert_eq!(
            count_contrast_rings(AccentColor::Black, |t| &t.menu_item, 1.0),
            0
        );
        // Other accent colors never emit the contrast ring.
        for accent in AccentColor::ALL {
            if accent != AccentColor::Black {
                assert_eq!(
                    count_contrast_rings(accent, |t| &t.tile, 1.0),
                    0,
                    "{:?}",
                    accent
                );
            }
        }
    }
}
