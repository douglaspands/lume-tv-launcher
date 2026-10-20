//! The dock: the favorites as a floating bar of square icons at the bottom of the Home screen.
//!
//! The focused icon grows (Magnification), its neighbors grow less, and the icons next to them
//! are pushed aside, like a desktop dock. Everything that decides the sizes lives here as
//! constants; the theme only styles the bar (`.dock`) and the name bubble (`.dock-label`).

use super::painter::{fade, Painter};
use super::{ids, Anim, LumeApp};
use crate::render::Rect;
use crate::text::{Align, TextStyle};
use crate::theme::{vars, Color, Paint, StateFlags, THEME};

/// Side of an icon at rest, in logical units.
pub const BASE: f32 = 64.0;
/// Space between two icons at rest.
pub const GAP: f32 = 14.0;
/// Space between the bar's edge and the icons.
pub const PADDING: f32 = 16.0;
/// Scale of the focused icon.
pub const PEAK: f32 = 1.6;
/// How much of the growth (`PEAK - 1`) an icon has, by its distance in places from the focus:
/// 0, 1, 2 and 3 or more. In between, the value is interpolated, so a moving focus is smooth.
pub const FALLOFF: [f32; 4] = [1.0, 0.45, 0.15, 0.0];
/// How long the Magnification takes to follow the focus, a little slower than the theme's focus
/// transition for a softer movement between icons.
const MOVE: crate::theme::Transition = crate::theme::Transition {
    duration_ms: 210.0,
    curve: crate::theme::Curve::EaseOut,
};
/// Corner radius of an icon over its side.
const RADIUS_RATIO: f32 = 0.22;
/// Space between the top of the focused icon and the name bubble.
const LABEL_GAP: f32 = 12.0;
const LABEL_HEIGHT: f32 = 40.0;

/// The growth of an icon `distance` places away from the focus (1.0 at the focus, 0.0 far away).
pub fn falloff(distance: f32) -> f32 {
    let d = distance.abs();
    let last = FALLOFF.len() - 1;
    if d >= last as f32 {
        return FALLOFF[last];
    }
    let (low, high) = (d.floor() as usize, d.ceil() as usize);
    FALLOFF[low] + (FALLOFF[high] - FALLOFF[low]) * (d - low as f32)
}

/// The scale of the icon at `index` for a focus at place `position` (fractional while it moves)
/// with `amplitude` between 0.0 (the focus is not in the dock) and 1.0.
pub fn scale(index: usize, position: f32, amplitude: f32) -> f32 {
    1.0 + (PEAK - 1.0) * amplitude.clamp(0.0, 1.0) * falloff(index as f32 - position)
}

/// Where the bar and the icons are.
#[derive(Clone, Debug, PartialEq)]
pub struct DockLayout {
    pub bar: Rect,
    pub items: Vec<Rect>,
}

/// Lays out `count` icons centered on `center_x`, with the bottom of the bar at `bottom`. The
/// bar is as wide as its icons need and as tall as an icon at rest; icons grow upward from the
/// bottom, and the ones beside a grown icon are pushed away from it.
pub fn layout(
    count: usize,
    position: f32,
    amplitude: f32,
    center_x: f32,
    bottom: f32,
) -> DockLayout {
    let sizes: Vec<f32> = (0..count)
        .map(|i| BASE * scale(i, position, amplitude))
        .collect();
    let gaps = count.saturating_sub(1) as f32 * GAP;
    let inner = sizes.iter().sum::<f32>() + gaps;
    let bar = Rect::new(
        center_x - (inner + 2.0 * PADDING) / 2.0,
        bottom - (BASE + 2.0 * PADDING),
        inner + 2.0 * PADDING,
        BASE + 2.0 * PADDING,
    );
    let mut x = bar.x + PADDING;
    let items = sizes
        .iter()
        .map(|&size| {
            let rect = Rect::new(x, bar.bottom() - PADDING - size, size, size);
            x += size + GAP;
            rect
        })
        .collect();
    DockLayout { bar, items }
}

/// Where the name bubble is anchored while the focus moves: the icon rectangle at the fractional
/// `position`, blended between the two icons the focus is between, so the bubble glides with the
/// Magnification instead of jumping to the new icon. Also returns the icon whose name to show,
/// the nearest one.
pub fn label_anchor(items: &[Rect], position: f32) -> Option<(Rect, usize)> {
    let last = items.len().checked_sub(1)?;
    let position = position.clamp(0.0, last as f32);
    let (low, high) = (position.floor() as usize, position.ceil() as usize);
    let t = position - low as f32;
    let (a, b) = (items[low], items[high]);
    let lerp = |from: f32, to: f32| from + (to - from) * t;
    Some((
        Rect::new(
            lerp(a.x, b.x),
            lerp(a.y, b.y),
            lerp(a.w, b.w),
            lerp(a.h, b.h),
        ),
        position.round() as usize,
    ))
}

/// The first letter of a name, for an icon that has not arrived yet.
fn initial(label: &str) -> String {
    label
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

/// Where the name bubble goes: centered above `icon`, `width` wide (shortened to the safe area
/// when the name is too long, which truncates the text) and moved sideways to stay inside `safe`.
pub fn label_rect(icon: Rect, width: f32, safe: Rect) -> Rect {
    let width = width.min(safe.w);
    let x = (icon.center().0 - width / 2.0).clamp(safe.x, (safe.right() - width).max(safe.x));
    Rect::new(x, icon.y - LABEL_GAP - LABEL_HEIGHT, width, LABEL_HEIGHT)
}

/// What one dock icon shows.
pub(super) struct DockItem {
    pub id: u32,
    pub package: Option<String>,
    pub label: String,
    pub color: Color,
}

impl LumeApp {
    /// The icons of the dock in order: the favorites, or the single "Add favorites" item.
    pub(super) fn dock_items(&self) -> Vec<DockItem> {
        if self.config.favorites.is_empty() {
            return vec![DockItem {
                id: ids::FAVORITES_ADD,
                package: None,
                label: self
                    .translator
                    .get(crate::i18n::StringId::HomeAddFavorites)
                    .into_owned(),
                color: vars::SURFACE_ROW,
            }];
        }
        self.config
            .favorites
            .iter()
            .enumerate()
            .filter_map(|(index, package)| {
                let app = self.catalog.get(package)?;
                Some(DockItem {
                    id: ids::FAVORITE_BASE + index as u32,
                    package: Some(app.package.clone()),
                    label: app.label.clone(),
                    color: app.color,
                })
            })
            .collect()
    }

    /// The place (0-based) in the dock of the focused element, if the focus is in the dock.
    pub(super) fn dock_focus_index(&self, items: &[DockItem]) -> Option<usize> {
        let focused = self.focus.current_id()?;
        items.iter().position(|item| item.id == focused)
    }

    /// Where the bar's bottom edge is: just above the bottom of the safe area.
    pub(super) fn dock_bottom(p: &Painter) -> f32 {
        p.display.safe_area().bottom()
    }

    /// Starts the animation of the Magnification toward the current focus. Entering the dock puts
    /// the focus on its icon at once and grows it; moving inside it slides the focus; leaving it
    /// shrinks everything back.
    pub(super) fn track_dock(&mut self, focus: Option<usize>) {
        let transition = Some(MOVE);
        let now = self.now_ms;
        match focus {
            Some(index) => {
                let entering = self.dock_focus.is_none();
                // Where the position was: it must be given as the starting value, because the
                // frame's clean-up forgets values at rest, and the position 0 counts as rest.
                let from = self.dock_last as f32;
                self.dock_focus = Some(index);
                self.dock_last = index;
                if entering {
                    self.anim.set(Anim::DockPosition, index as f32);
                }
                self.anim
                    .animate_to(Anim::DockPosition, from, index as f32, now, transition);
                self.anim
                    .animate_to(Anim::DockAmplitude, 0.0, 1.0, now, transition);
            }
            None => {
                self.dock_focus = None;
                self.anim
                    .animate_to(Anim::DockAmplitude, 0.0, 0.0, now, transition);
            }
        }
    }

    /// The animated position and amplitude of the Magnification now.
    pub(super) fn dock_motion(&self) -> (f32, f32) {
        let position = self.anim.get(Anim::DockPosition, 0.0, self.now_ms);
        let amplitude = self.anim.get(Anim::DockAmplitude, 0.0, self.now_ms);
        (position, amplitude)
    }

    /// Draws the bar, the icons and the name bubble.
    pub(super) fn draw_dock(&mut self, p: &mut Painter, items: &[DockItem], layout: &DockLayout) {
        let bar = p.style(&THEME.dock, StateFlags::NONE);
        p.draw_box(layout.bar, &bar);

        let radius = |rect: &Rect| rect.w * RADIUS_RATIO;
        for (item, rect) in items.iter().zip(&layout.items) {
            p.list.fill(*rect, radius(rect), Paint::Solid(item.color));
        }
        for (item, rect) in items.iter().zip(&layout.items) {
            let uv = item.package.as_deref().and_then(|p| self.icons.dock_uv(p));
            if let Some(uv) = uv {
                p.list.image(*rect, uv, Color::WHITE);
            }
        }
        // While the icon has not arrived, the app's color carries its first letter (or a plus).
        for (item, rect) in items.iter().zip(&layout.items) {
            let has_icon = item
                .package
                .as_deref()
                .is_some_and(|p| self.icons.has_dock(p));
            if has_icon {
                continue;
            }
            if item.package.is_none() {
                p.icon(crate::icons::PLUS, *rect, rect.w * 0.45, vars::TEXT_MUTED);
            } else {
                let style = TextStyle::new((rect.w * 0.45).round(), 800);
                p.text_with(
                    &initial(&item.label),
                    *rect,
                    Align::Center,
                    &style,
                    Color::WHITE,
                );
            }
        }
    }

    /// Draws the bubble with the name of the focused app above its (grown) icon.
    pub(super) fn draw_dock_label(
        &mut self,
        p: &mut Painter,
        label: &str,
        icon: Rect,
        amplitude: f32,
    ) {
        if amplitude <= 0.0 {
            return;
        }
        let style = p.style(&THEME.dock_label, StateFlags::NONE);
        let safe = p.display.safe_area();
        let text_width = p.text.measure(label, &Painter::text_style(&style));
        let padding = style.padding[1] + style.padding[3];
        let rect = label_rect(icon, text_width + padding, safe);
        let mut bubble = style;
        bubble.opacity = amplitude;
        p.draw_box(rect, &bubble);
        // One px of slack on each side: `rect.w - padding` can come out a rounding error short of
        // the measured text, and the line would then be ellipsized although the bubble fits it.
        let inner = Rect::new(
            rect.x + style.padding[3] - 1.0,
            rect.y,
            rect.w - padding + 2.0,
            rect.h,
        );
        let mut text = style;
        text.color = fade(text.color, amplitude);
        p.text(label, inner, Align::Center, &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CENTER: f32 = 640.0;
    const BOTTOM: f32 = 700.0;

    #[test]
    fn the_falloff_table_and_the_scales_of_the_middle_item_of_five() {
        assert_eq!(falloff(0.0), 1.0);
        assert_eq!(falloff(1.0), 0.45);
        assert_eq!(falloff(2.0), 0.15);
        assert_eq!(falloff(3.0), 0.0);
        assert_eq!(falloff(7.0), 0.0);
        assert_eq!(falloff(-1.0), falloff(1.0));
        let scales: Vec<f32> = (0..5).map(|i| scale(i, 2.0, 1.0)).collect();
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(scales[2], 1.6), "{scales:?}");
        assert!(
            close(scales[1], 1.27) && close(scales[3], 1.27),
            "{scales:?}"
        );
        assert!(
            close(scales[0], 1.09) && close(scales[4], 1.09),
            "{scales:?}"
        );
    }

    #[test]
    fn the_scale_is_normal_when_the_focus_is_outside_the_dock() {
        for index in 0..5 {
            assert_eq!(scale(index, 2.0, 0.0), 1.0);
        }
        let layout = layout(5, 2.0, 0.0, CENTER, BOTTOM);
        assert!(layout.items.iter().all(|r| r.w == BASE && r.h == BASE));
    }

    #[test]
    fn the_scale_moves_smoothly_with_the_focus() {
        // Between two places the sizes change a little at a time, never jumping.
        let mut previous: Vec<f32> = (0..5).map(|i| scale(i, 2.0, 1.0)).collect();
        for step in 1..=20 {
            let position = 2.0 + step as f32 / 20.0;
            let now: Vec<f32> = (0..5).map(|i| scale(i, position, 1.0)).collect();
            for (a, b) in previous.iter().zip(&now) {
                assert!((a - b).abs() < 0.06, "{a} -> {b} at {position}");
            }
            previous = now;
        }
        assert!((scale(3, 3.0, 1.0) - 1.6).abs() < 1e-4, "arrived");
    }

    #[test]
    fn the_amplitude_scales_the_growth() {
        let half = scale(2, 2.0, 0.5);
        assert!((half - 1.3).abs() < 1e-4);
        assert_eq!(scale(2, 2.0, 5.0), 1.6, "clamped");
    }

    #[test]
    fn the_layout_is_centered_anchored_at_the_bottom_and_pushes_the_neighbors() {
        let rest = layout(5, 2.0, 0.0, CENTER, BOTTOM);
        let grown = layout(5, 2.0, 1.0, CENTER, BOTTOM);
        for l in [&rest, &grown] {
            assert!((l.bar.center().0 - CENTER).abs() < 1e-3, "centered");
            assert_eq!(l.bar.bottom(), BOTTOM);
            for rect in &l.items {
                assert!(
                    (rect.bottom() - (BOTTOM - PADDING)).abs() < 1e-3,
                    "base anchored"
                );
            }
        }
        // The focused icon grows upward from the same base, and the bar widens.
        assert!(grown.items[2].h > rest.items[2].h);
        assert!(grown.items[2].y < rest.items[2].y);
        assert!(grown.bar.w > rest.bar.w);
        assert_eq!(grown.bar.h, rest.bar.h, "the bar keeps its height");
        // Neighbors are pushed outward and never overlap.
        assert!(grown.items[0].x < rest.items[0].x);
        assert!(grown.items[4].right() > rest.items[4].right());
        for pair in grown.items.windows(2) {
            assert!(pair[0].right() + GAP - 1e-3 <= pair[1].x, "{pair:?}");
        }
    }

    #[test]
    fn twelve_favorites_fit_the_safe_area_even_at_the_peak() {
        let display = crate::display::Display::new(1280, 720);
        let safe = display.safe_area();
        for focus in 0..12 {
            let l = layout(12, focus as f32, 1.0, CENTER, safe.bottom());
            assert!(
                l.bar.x >= safe.x && l.bar.right() <= safe.right(),
                "focus {focus}: {:?}",
                l.bar
            );
            assert!(l.bar.bottom() <= safe.bottom());
            assert!(l
                .items
                .iter()
                .all(|r| r.x >= safe.x && r.right() <= safe.right() && r.y >= safe.y));
        }
    }

    #[test]
    fn one_item_makes_a_small_centered_bar() {
        let l = layout(1, 0.0, 1.0, CENTER, BOTTOM);
        assert_eq!(l.items.len(), 1);
        assert!((l.items[0].center().0 - CENTER).abs() < 1e-3);
        assert!(layout(0, 0.0, 0.0, CENTER, BOTTOM).items.is_empty());
    }

    #[test]
    fn the_bubble_is_centered_above_the_grown_icon() {
        let display = crate::display::Display::new(1280, 720);
        let safe = display.safe_area();
        let l = layout(5, 2.0, 1.0, CENTER, safe.bottom());
        let icon = l.items[2];
        let bubble = label_rect(icon, 120.0, safe);
        assert!((bubble.center().0 - icon.center().0).abs() < 1e-3);
        assert!(bubble.bottom() < icon.y, "above the icon");
        assert!(bubble.y >= safe.y);
    }

    #[test]
    fn the_bubble_stays_inside_the_safe_area_for_an_item_at_the_edge() {
        let display = crate::display::Display::new(1280, 720);
        let safe = display.safe_area();
        for focus in [0usize, 11] {
            let l = layout(12, focus as f32, 1.0, CENTER, safe.bottom());
            let bubble = label_rect(l.items[focus], 400.0, safe);
            assert!(
                bubble.x >= safe.x - 1e-3 && bubble.right() <= safe.right() + 1e-3,
                "{bubble:?}"
            );
        }
    }

    #[test]
    fn a_name_wider_than_the_safe_area_is_cut_to_it() {
        let display = crate::display::Display::new(1280, 720);
        let safe = display.safe_area();
        let l = layout(5, 0.0, 1.0, CENTER, safe.bottom());
        let bubble = label_rect(l.items[0], 5000.0, safe);
        assert_eq!((bubble.x, bubble.w), (safe.x, safe.w));
    }

    #[test]
    fn the_bubble_glides_between_two_icons_instead_of_jumping() {
        let l = layout(5, 1.0, 1.0, CENTER, BOTTOM);
        let (at_one, who) = label_anchor(&l.items, 1.0).unwrap();
        assert_eq!((at_one, who), (l.items[1], 1));
        let (at_two, who) = label_anchor(&l.items, 2.0).unwrap();
        assert_eq!((at_two, who), (l.items[2], 2));
        // In between it is between the two, and the name changes at the half way point.
        let (mut last_x, mut names) = (at_one.center().0, Vec::new());
        for step in 1..=10 {
            let (anchor, who) = label_anchor(&l.items, 1.0 + step as f32 / 10.0).unwrap();
            let x = anchor.center().0;
            assert!(
                x > last_x && x - last_x < 20.0,
                "steps of a few units, not a jump: {last_x} -> {x}"
            );
            last_x = x;
            names.push(who);
        }
        assert_eq!(names, [1, 1, 1, 1, 2, 2, 2, 2, 2, 2]);
        assert!(label_anchor(&[], 0.0).is_none());
        assert_eq!(
            label_anchor(&l.items, 9.0).unwrap().1,
            4,
            "clamped to the last icon"
        );
    }

    #[test]
    fn the_initial_is_the_first_letter_or_digit_in_capitals() {
        assert_eq!(initial("netflix"), "N");
        assert_eq!(initial("  +Disney"), "D");
        assert_eq!(initial("ábaco"), "Á");
        assert_eq!(initial("++"), "");
    }
}
