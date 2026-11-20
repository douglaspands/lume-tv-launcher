//! The dock: the favorites as a floating bar of square icons at the bottom of the Home screen.
//!
//! The focused icon grows (Magnification), its neighbors grow less, and the icons next to them
//! are pushed aside, like a desktop dock. Everything that decides the sizes lives here as
//! constants; the theme only styles the bar (`.dock`) and the name bubble (`.dock-label`).

use super::painter::{fade, Painter};
use super::{ids, Anim, LumeApp, Screen, Tab};
use crate::home::config::DockIconShape;
use crate::render::Rect;
use crate::text::{Align, TextStyle};
use crate::theme::{vars, Color, Paint, StateFlags};

/// Side of a square/round icon at rest (and height of a banner item at rest), in logical units.
pub const BASE: f32 = 64.0;
/// Width of a 16:9 rectangular banner item at rest (`112 x 64`, on the 4-unit ruler).
pub const BANNER_BASE_WIDTH: f32 = 112.0;
/// Space between two icons at rest.
pub const GAP: f32 = 16.0;
/// Space between the bar's edge and the icons.
pub const PADDING: f32 = 16.0;
/// Maximum width of the dock bar so 13 items (12 favorites + "All apps") stay inside the safe area
/// even while `MOVE_ROOM` is open during reordering.
pub const MAX_BAR_WIDTH: f32 = crate::display::CANVAS_WIDTH - 2.0 * super::topbar::MARGIN;
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
/// Space added on each side of a favorite being moved, where its arrows go.
pub const MOVE_ROOM: f32 = 24.0;
/// Side of the arrows beside a favorite being moved.
const ARROW_SIZE: f32 = 24.0;
/// How long the icons take to slide to their new places when the favorites change, and the
/// neighbors of a favorite being moved to make room for its arrows.
/// (`--dock-reorder`).
const SLIDE: crate::theme::Transition = crate::theme::vars::DOCK_REORDER;
/// Space between the top of the focused icon and the name bubble.
pub(super) const LABEL_GAP: f32 = 12.0;
pub(super) const LABEL_HEIGHT: f32 = 40.0;

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
#[cfg(test)]
pub fn layout(
    count: usize,
    position: f32,
    amplitude: f32,
    center_x: f32,
    bottom: f32,
) -> DockLayout {
    layout_with_room_for(
        count,
        position,
        amplitude,
        center_x,
        bottom,
        0.0,
        DockIconShape::Round,
    )
}

/// Like [`layout`], using the geometry of `shape` (`Round`/`Square` or 16:9 `Banner`).
pub fn layout_for(
    count: usize,
    position: f32,
    amplitude: f32,
    center_x: f32,
    bottom: f32,
    shape: DockIconShape,
) -> DockLayout {
    layout_with_room_for(count, position, amplitude, center_x, bottom, 0.0, shape)
}

/// Like [`layout`], with `room` (0.0 to 1.0) of [`MOVE_ROOM`] added to the two gaps beside the
/// icon at `position`: the space for the arrows while a favorite is being moved.
#[cfg(test)]
pub fn layout_with_room(
    count: usize,
    position: f32,
    amplitude: f32,
    center_x: f32,
    bottom: f32,
    room: f32,
) -> DockLayout {
    layout_with_room_for(
        count,
        position,
        amplitude,
        center_x,
        bottom,
        room,
        DockIconShape::Round,
    )
}

/// Like [`layout_with_room`], supporting `DockIconShape::Banner` (16:9 `112×64` at rest) and
/// proportionally scaling item dimensions (preserving 16:9) and gap when needed so up to 13 items
/// + Magnification + `MOVE_ROOM` stay inside [`MAX_BAR_WIDTH`].
pub fn layout_with_room_for(
    count: usize,
    position: f32,
    amplitude: f32,
    center_x: f32,
    bottom: f32,
    room: f32,
    shape: DockIconShape,
) -> DockLayout {
    let scales: Vec<f32> = (0..count).map(|i| scale(i, position, amplitude)).collect();
    let moving = position.round() as usize;
    let scales_sum: f32 = scales.iter().sum();
    let gap_count = count.saturating_sub(1) as f32;
    let move_gaps = (1..count)
        .filter(|&i| i == moving || i == moving + 1)
        .count() as f32;
    let move_room_total = move_gaps * MOVE_ROOM * room;
    let (base_w, base_h, base_gap) = match shape {
        DockIconShape::Banner => {
            let available = (MAX_BAR_WIDTH - 2.0 * PADDING - move_room_total).max(1.0);
            let nominal = BANNER_BASE_WIDTH * scales_sum + gap_count * GAP;
            let shrink = if nominal > available {
                available / nominal
            } else {
                1.0
            };
            (BANNER_BASE_WIDTH * shrink, BASE * shrink, GAP * shrink)
        }
        DockIconShape::Round | DockIconShape::Square => {
            let sizes_sum = BASE * scales_sum;
            let base_gap = if count > 12 && gap_count > 0.0 {
                let full = sizes_sum + 2.0 * PADDING + move_room_total + gap_count * GAP;
                if full > MAX_BAR_WIDTH {
                    ((MAX_BAR_WIDTH - sizes_sum - 2.0 * PADDING - move_room_total) / gap_count)
                        .clamp(8.0, GAP)
                } else {
                    GAP
                }
            } else {
                GAP
            };
            (BASE, BASE, base_gap)
        }
    };
    // The gap before the icon at `i` (from the second icon on).
    let gap = |i: usize| {
        base_gap
            + if i == moving || i == moving + 1 {
                MOVE_ROOM * room
            } else {
                0.0
            }
    };
    let gaps: f32 = (1..count).map(gap).sum();
    let inner = base_w * scales_sum + gaps;
    let bar = Rect::new(
        center_x - (inner + 2.0 * PADDING) / 2.0,
        bottom - (base_h + 2.0 * PADDING),
        inner + 2.0 * PADDING,
        base_h + 2.0 * PADDING,
    );
    let mut x = bar.x + PADDING;
    let items = scales
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            if i > 0 {
                x += gap(i);
            }
            let w = base_w * s;
            let h = base_h * s;
            let rect = Rect::new(x, bar.bottom() - PADDING - h, w, h);
            x += w;
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

/// An icon as it was drawn, to slide it from there when the favorites change.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct DrawnIcon {
    pub id: u32,
    pub package: Option<String>,
    pub color: Color,
    pub rect: Rect,
}

/// What one dock icon shows.
pub(super) struct DockItem {
    pub id: u32,
    pub package: Option<String>,
    pub label: String,
    pub color: Color,
}

impl LumeApp {
    /// The icons of the dock in order: the favorites (plus optional fixed "All apps" slot in
    /// `No Dock` mode), or the single "Add favorites" item when there are no favorites yet.
    pub(super) fn dock_items(&self) -> Vec<DockItem> {
        let all_apps = || DockItem {
            id: ids::DOCK_ALL_APPS,
            package: None,
            label: self
                .translator
                .get(crate::i18n::StringId::AppsTitle)
                .into_owned(),
            color: self.active_accent().primary,
        };
        if self.config.favorites.is_empty() {
            let mut items = vec![DockItem {
                id: ids::FAVORITES_ADD,
                package: None,
                label: self
                    .translator
                    .get(crate::i18n::StringId::HomeAddFavorites)
                    .into_owned(),
                color: vars::SURFACE_ROW,
            }];
            if self.config.has_dock_all_apps() {
                items.push(all_apps());
            }
            return items;
        }
        let mut items: Vec<DockItem> = self
            .config
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
            .collect();
        if self.config.has_dock_all_apps() {
            let index = self.config.effective_all_apps_dock_index().min(items.len());
            items.insert(index, all_apps());
        }
        items
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

    /// Whether the dock currently draws 16:9 tile banners instead of square dock icons.
    pub(super) fn dock_uses_banners(&self) -> bool {
        self.config.dock_shape == DockIconShape::Banner
    }

    fn dock_item_uv(&self, package: &str) -> Option<Rect> {
        if self.dock_uses_banners() {
            self.icons.uv(package)
        } else {
            self.icons.dock_uv(package)
        }
    }

    fn dock_item_has_image(&self, package: &str) -> bool {
        if self.dock_uses_banners() {
            self.icons.has(package)
        } else {
            self.icons.has_dock(package)
        }
    }

    fn dock_item_visibility(&self, package: &str) -> f32 {
        self.icon_visibility(package, !self.dock_uses_banners())
    }

    pub(super) fn dock_radius_ratio(&self) -> f32 {
        match self.config.dock_shape {
            DockIconShape::Round => 0.5,
            DockIconShape::Square => 0.22,
            DockIconShape::Banner => vars::SHAPE_MD / BASE,
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
        let bar = p.style(|t| &t.dock, StateFlags::NONE);
        p.draw_box(layout.bar, &bar);

        let radius_ratio = self.dock_radius_ratio();
        let radius = |rect: &Rect| rect.h * radius_ratio;
        let uv = |item: &DockItem| item.package.as_deref().and_then(|p| self.dock_item_uv(p));
        // Precompute optional Banner text placement for `DOCK_ALL_APPS` so the 9 SDF dots stay
        // in the rect pass and the translated "Apps" label stays in the text pass (preserving
        // draw-call batching).
        let all_apps_label = self
            .translator
            .get(crate::i18n::StringId::TabApps)
            .into_owned();
        let mut all_apps_banner_text: Option<(Rect, TextStyle)> = None;

        // An item without an icon is its color; one with an icon is drawn in one piece, the icon
        // over the color, so its edge is antialiased once (a fill under the image left a ring of
        // the color around it). Each kind is one draw call.
        for (item, rect) in items.iter().zip(&layout.items) {
            if uv(item).is_none() {
                p.list.fill(*rect, radius(rect), Paint::Solid(item.color));
                if item.id == ids::DOCK_ALL_APPS {
                    if let Some(ring) = p.accent().contrast_ring {
                        p.list.ring(*rect, radius(rect), 1.5, ring);
                    }
                    let banner = if self.dock_uses_banners() {
                        all_apps_banner_layout(p, *rect, &all_apps_label)
                    } else {
                        None
                    };
                    let grid_x = banner.as_ref().map(|(x, _, _)| *x);
                    if let Some((_, text_rect, style)) = banner {
                        all_apps_banner_text = Some((text_rect, style));
                    }
                    draw_all_apps_dots(p, *rect, grid_x);
                }
            }
        }
        for (item, rect) in items.iter().zip(&layout.items) {
            if let Some(uv) = uv(item) {
                let visible = item
                    .package
                    .as_deref()
                    .map_or(1.0, |p| self.dock_item_visibility(p));
                p.list.image_over(
                    *rect,
                    uv,
                    super::painter::fade(Color::WHITE, visible),
                    radius(rect),
                    item.color,
                );
            }
        }
        // While the icon has not arrived, the app's color carries its first letter (or a plus); while
        // it fades in, the letter fades out under it.
        for (item, rect) in items.iter().zip(&layout.items) {
            let has_icon = item
                .package
                .as_deref()
                .is_some_and(|p| self.dock_item_has_image(p));
            let letter = if has_icon {
                1.0 - item
                    .package
                    .as_deref()
                    .map_or(1.0, |p| self.dock_item_visibility(p))
            } else {
                1.0
            };
            if letter <= 0.0 {
                continue;
            }
            if item.id == ids::DOCK_ALL_APPS {
                if let Some((text_rect, style)) = all_apps_banner_text.as_ref() {
                    p.text_with(
                        &all_apps_label,
                        *text_rect,
                        Align::Start,
                        style,
                        p.on_primary(),
                    );
                }
            } else if item.package.is_none() {
                p.icon(crate::icons::PLUS, *rect, rect.h * 0.45, vars::TEXT_MUTED);
            } else {
                let style = TextStyle::new((rect.h * 0.45).round(), 800);
                p.text_with(
                    &initial(&item.label),
                    *rect,
                    Align::Center,
                    &style,
                    super::painter::fade(Color::WHITE, letter),
                );
            }
        }
    }

    /// The favorites are about to change while the dock is on screen: the icons slide from where
    /// they are drawn now to their new places. `focus`, the place the focus goes to, is taken by
    /// the Magnification at once, so the slide alone carries the movement and no icon changes
    /// size on the way.
    pub(super) fn begin_dock_transition(&mut self, focus: Option<usize>) {
        if self.screen != Screen::Main(Tab::Home) {
            return;
        }
        self.dock_from = Some(self.dock_drawn.clone());
        if let Some(index) = focus {
            self.dock_last = index;
            if self.dock_focus.is_some() {
                self.dock_focus = Some(index);
            }
            self.anim.set(Anim::DockPosition, index as f32);
        }
    }

    /// Starts the slide on the first frame after a change, then moves each icon by what is left of
    /// it. The bar follows its icons. A favorite that left the dock shrinks and fades where it was.
    pub(super) fn slide_dock(&mut self, items: &[DockItem], layout: &mut DockLayout) {
        let now = self.now_ms;
        if let Some(from) = self.dock_from.take() {
            for (index, (item, rect)) in items.iter().zip(&layout.items).enumerate() {
                let old = from.iter().find(|d| {
                    if item.package.is_some() {
                        d.package == item.package
                    } else {
                        item.id == ids::DOCK_ALL_APPS && d.id == ids::DOCK_ALL_APPS
                    }
                });
                let Some(old) = old else { continue };
                let shift = old.rect.center().0 - rect.center().0;
                let grow = old.rect.w - rect.w;
                // From the value as drawn now, even when a slide was still running.
                self.anim.set(Anim::DockShift(index), shift);
                self.anim
                    .animate_to(Anim::DockShift(index), shift, 0.0, now, Some(SLIDE));
                self.anim.set(Anim::DockGrow(index), grow);
                self.anim
                    .animate_to(Anim::DockGrow(index), grow, 0.0, now, Some(SLIDE));
            }
            let gone = from
                .into_iter()
                .find(|d| d.package.is_some() && !items.iter().any(|i| i.package == d.package));
            if let Some(gone) = gone {
                self.dock_ghost = Some(gone);
                self.anim.set(Anim::DockGhost, 1.0);
                self.anim
                    .animate_to(Anim::DockGhost, 1.0, 0.0, now, Some(SLIDE));
            }
        }
        for (index, rect) in layout.items.iter_mut().enumerate() {
            let shift = self.anim.get(Anim::DockShift(index), 0.0, now);
            let grow = self.anim.get(Anim::DockGrow(index), 0.0, now);
            if shift != 0.0 || grow != 0.0 {
                let ratio = if rect.w > 0.0 { rect.h / rect.w } else { 1.0 };
                let w = (rect.w + grow).max(1.0);
                let h = w * ratio;
                let center = rect.center().0 + shift;
                *rect = Rect::new(center - w / 2.0, rect.bottom() - h, w, h);
            }
        }
        if let (Some(first), Some(last)) = (layout.items.first(), layout.items.last()) {
            layout.bar.x = first.x - PADDING;
            layout.bar.w = last.right() + PADDING - layout.bar.x;
        }
        self.dock_drawn = items
            .iter()
            .zip(&layout.items)
            .map(|(item, rect)| DrawnIcon {
                id: item.id,
                package: item.package.clone(),
                color: item.color,
                rect: *rect,
            })
            .collect();
    }

    /// The favorite that left the dock, shrinking and fading where it was.
    pub(super) fn draw_dock_ghost(&mut self, p: &mut Painter) {
        let Some(ghost) = self.dock_ghost.clone() else {
            return;
        };
        let left = self.anim.get(Anim::DockGhost, 0.0, self.now_ms);
        if left <= 0.0 {
            self.dock_ghost = None;
            return;
        }
        let radius_ratio = self.dock_radius_ratio();
        let rect = ghost.rect.scaled_about_center(left);
        let radius = rect.h * radius_ratio;
        match ghost.package.as_deref().and_then(|p| self.dock_item_uv(p)) {
            Some(uv) => p.list.image_over(
                rect,
                uv,
                fade(Color::WHITE, left),
                radius,
                fade(ghost.color, left),
            ),
            None => p
                .list
                .fill(rect, radius, Paint::Solid(fade(ghost.color, left))),
        }
    }

    /// The arrows beside the favorite being moved, in the gaps the neighbors left for them; none
    /// at an end of the dock. `room` is how far the neighbors have moved away (0.0 to 1.0).
    pub(super) fn draw_move_arrows(
        &self,
        p: &mut Painter,
        items: &[Rect],
        index: usize,
        room: f32,
    ) {
        let Some(icon) = items.get(index) else { return };
        let color = fade(vars::TEXT, room);
        let y = icon.center().1 - ARROW_SIZE / 2.0;
        if let Some(before) = index.checked_sub(1).and_then(|i| items.get(i)) {
            let x = (before.right() + icon.x) / 2.0 - ARROW_SIZE / 2.0;
            let rect = Rect::new(x, y, ARROW_SIZE, ARROW_SIZE);
            p.icon(crate::icons::CHEVRON_LEFT, rect, ARROW_SIZE, color);
        }
        if let Some(after) = items.get(index + 1) {
            let x = (icon.right() + after.x) / 2.0 - ARROW_SIZE / 2.0;
            let rect = Rect::new(x, y, ARROW_SIZE, ARROW_SIZE);
            p.icon(crate::icons::CHEVRON_RIGHT, rect, ARROW_SIZE, color);
        }
    }

    /// Opens or closes the room for the arrows beside a favorite being moved.
    pub(super) fn animate_move_room(&mut self, open: bool) {
        let (from, to) = if open { (0.0, 1.0) } else { (1.0, 0.0) };
        self.anim
            .animate_to(Anim::DockRoom, from, to, self.now_ms, Some(SLIDE));
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
        let style = p.style(|t| &t.dock_label, StateFlags::NONE);
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

/// Computes the horizontal placement of the 3×3 dot grid and translated "Apps" label inside a
/// 16:9 `Banner` `DOCK_ALL_APPS` tile, returning `None` if the label does not fit cleanly.
fn all_apps_banner_layout(
    p: &mut Painter,
    rect: Rect,
    label: &str,
) -> Option<(f32, Rect, TextStyle)> {
    let scale = rect.h / BASE;
    let dot = 5.8 * scale;
    let gap = 4.8 * scale;
    let grid_size = 3.0 * dot + 2.0 * gap;
    let label_gap = 10.0 * scale;
    let style = TextStyle::new((17.0 * scale).round().max(10.0), 700);
    let label_w = p.text.measure(label, &style);
    let total_w = grid_size + label_gap + label_w;
    if total_w > rect.w - 16.0 * scale {
        return None;
    }
    let start_x = rect.center().0 - total_w / 2.0;
    let text_rect = Rect::new(
        start_x + grid_size + label_gap,
        rect.y,
        label_w + 4.0,
        rect.h,
    );
    Some((start_x, text_rect, style))
}

/// Draws the 3×3 circular dot grid for `ids::DOCK_ALL_APPS` directly via SDF rects (zero extra
/// texture). When `grid_x` is `None`, the grid is centered horizontally inside `rect`.
fn draw_all_apps_dots(p: &mut Painter, rect: Rect, grid_x: Option<f32>) {
    let scale = rect.h / BASE;
    let dot = 5.8 * scale;
    let gap = 4.8 * scale;
    let grid_size = 3.0 * dot + 2.0 * gap;
    let gx = grid_x.unwrap_or_else(|| rect.center().0 - grid_size / 2.0);
    let gy = rect.center().1 - grid_size / 2.0;
    let dot_radius = dot / 2.0;
    let paint = Paint::Solid(p.on_primary());
    for row in 0..3 {
        for col in 0..3 {
            let x = gx + col as f32 * (dot + gap);
            let y = gy + row as f32 * (dot + gap);
            p.list.fill(Rect::new(x, y, dot, dot), dot_radius, paint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_on_the_ruler() {
        super::super::assert_on_ruler(&[
            ("BASE", BASE),
            ("BANNER_BASE_WIDTH", BANNER_BASE_WIDTH),
            ("GAP", GAP),
            ("PADDING", PADDING),
            ("LABEL_GAP", LABEL_GAP),
            ("LABEL_HEIGHT", LABEL_HEIGHT),
            ("MOVE_ROOM", MOVE_ROOM),
            ("ARROW_SIZE", ARROW_SIZE),
        ]);
    }

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
    fn thirteen_items_with_move_room_fit_the_safe_area_at_any_focus() {
        let display = crate::display::Display::new(1280, 720);
        let safe = display.safe_area();
        for room in [0.0, 1.0] {
            for focus in 0..13 {
                let l = layout_with_room(13, focus as f32, 1.0, CENTER, safe.bottom(), room);
                assert!(
                    l.bar.w <= MAX_BAR_WIDTH + 1e-2,
                    "room {room} focus {focus}: bar width {} > {}",
                    l.bar.w,
                    MAX_BAR_WIDTH
                );
                assert!(
                    l.bar.x >= safe.x - 1e-2 && l.bar.right() <= safe.right() + 1e-2,
                    "room {room} focus {focus}: {:?}",
                    l.bar
                );
                assert!(l.items.iter().all(|r| r.x >= safe.x - 1e-2
                    && r.right() <= safe.right() + 1e-2
                    && r.y >= safe.y));
                for pair in l.items.windows(2) {
                    assert!(
                        pair[0].right() <= pair[1].x + 1e-3,
                        "icons overlap at room {room} focus {focus}: {pair:?}"
                    );
                }
            }
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

    #[test]
    fn thirteen_banner_items_with_move_room_fit_the_safe_area_at_any_focus() {
        let display = crate::display::Display::new(1280, 720);
        let safe = display.safe_area();
        let expected_aspect = BANNER_BASE_WIDTH / BASE;
        // Fewer items keep the exact 112x64 base size at rest.
        let five_rest = layout_for(5, 0.0, 0.0, CENTER, safe.bottom(), DockIconShape::Banner);
        assert!(five_rest
            .items
            .iter()
            .all(|r| (r.w - BANNER_BASE_WIDTH).abs() < 1e-3 && (r.h - BASE).abs() < 1e-3));

        for room in [0.0, 1.0] {
            for focus in 0..13 {
                let l = layout_with_room_for(
                    13,
                    focus as f32,
                    1.0,
                    CENTER,
                    safe.bottom(),
                    room,
                    DockIconShape::Banner,
                );
                assert!(
                    l.bar.w <= MAX_BAR_WIDTH + 1e-2,
                    "banner room {room} focus {focus}: bar width {} > {}",
                    l.bar.w,
                    MAX_BAR_WIDTH
                );
                assert!(
                    l.bar.x >= safe.x - 1e-2 && l.bar.right() <= safe.right() + 1e-2,
                    "banner room {room} focus {focus}: {:?}",
                    l.bar
                );
                for r in &l.items {
                    assert!(r.x >= safe.x - 1e-2 && r.right() <= safe.right() + 1e-2);
                    assert!(r.y >= safe.y);
                    assert!(
                        ((r.w / r.h) - expected_aspect).abs() < 1e-3,
                        "aspect ratio broken: {r:?}"
                    );
                }
                for pair in l.items.windows(2) {
                    assert!(
                        pair[0].right() <= pair[1].x + 1e-3,
                        "banner icons overlap at room {room} focus {focus}: {pair:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn dock_radius_ratio_reflects_dock_shape() {
        let catalog = crate::catalog::Catalog::new("app.lume.tv");
        let mut app = LumeApp::new(
            catalog,
            crate::usage::UsageHistory::new(),
            crate::home::config::HomeConfig {
                favorites: vec!["app1".into()],
                dock_shape: DockIconShape::Square,
                wallpaper: crate::wallpaper::WallpaperId::Aurora,
                custom: None,
                accent_color: crate::theme::AccentColor::Blue,
                tabs_visible: true,
                tabs_align: crate::home::config::TabsAlign::Left,
                tabless_mode: crate::home::config::TablessMode::Slide,
                all_apps_dock_index: None,
            },
            crate::i18n::Translator::new(&["en"]),
            crate::clock::WallClock {
                unix_secs: 0,
                utc_offset_secs: 0,
                is_24h: true,
            },
        );
        assert_eq!(app.dock_radius_ratio(), 0.22);
        app.config.dock_shape = DockIconShape::Round;
        assert_eq!(app.dock_radius_ratio(), 0.5);
        app.config.dock_shape = DockIconShape::Banner;
        assert!((app.dock_radius_ratio() - vars::SHAPE_MD / BASE).abs() < 1e-6);
    }

    #[test]
    fn dock_all_apps_draws_nine_circular_dots_and_black_contrast_ring() {
        use crate::catalog::{Catalog, Discovered};
        use crate::render::{Cmd, DisplayList, RectKind};
        use crate::theme::{AccentColor, THEME};

        let catalog = Catalog::from_discovered(
            &[Discovered {
                package: "app.one".into(),
                label: "One".into(),
                activity: "app.one.Main".into(),
                leanback: true,
                system: false,
                icon: None,
            }],
            "app.lume.tv",
        );
        let display = crate::display::Display::new(1280, 720);
        let mut text = crate::text::TextEngine::new(1.0);

        for (shape, accent, expect_ring, expect_text) in [
            (DockIconShape::Round, AccentColor::Blue, false, false),
            (DockIconShape::Square, AccentColor::Emerald, false, false),
            (DockIconShape::Banner, AccentColor::Blue, false, true),
            (DockIconShape::Banner, AccentColor::Black, true, true),
        ] {
            let mut app = LumeApp::new(
                catalog.clone(),
                crate::usage::UsageHistory::new(),
                crate::home::config::HomeConfig {
                    favorites: vec!["app.one".into()],
                    dock_shape: shape,
                    wallpaper: crate::wallpaper::WallpaperId::Aurora,
                    custom: None,
                    accent_color: accent,
                    tabs_visible: false,
                    tabs_align: crate::home::config::TabsAlign::Left,
                    tabless_mode: crate::home::config::TablessMode::Dock,
                    all_apps_dock_index: None,
                },
                crate::i18n::Translator::new(&["pt-BR"]),
                crate::clock::WallClock {
                    unix_secs: 0,
                    utc_offset_secs: 0,
                    is_24h: true,
                },
            );
            let theme = THEME.with_accent(accent);
            let palette = accent.palette();
            let mut list = DisplayList::new();
            let items = app.dock_items();
            let layout = layout_for(items.len(), 0.0, 0.0, 640.0, 680.0, shape);
            let mut p = Painter {
                list: &mut list,
                text: &mut text,
                display,
                theme: &theme,
                accent: palette,
            };
            app.draw_dock(&mut p, &items, &layout);

            let dot_count = list
                .cmds()
                .iter()
                .filter(|cmd| {
                    matches!(
                        cmd,
                        Cmd::Rect(r)
                            if (r.rect.w - r.rect.h).abs() < 1e-3
                                && (r.radius - r.rect.w / 2.0).abs() < 1e-3
                                && r.rect.w < 12.0
                                && matches!(r.kind, RectKind::Fill(Paint::Solid(c)) if c == palette.on_primary)
                    )
                })
                .count();
            assert_eq!(
                dot_count, 9,
                "{shape:?} {accent:?} should draw 9 circular dots"
            );

            let has_ring = list.cmds().iter().any(|cmd| {
                matches!(
                    cmd,
                    Cmd::Rect(r) if matches!(r.kind, RectKind::Ring { color, .. } if Some(color) == palette.contrast_ring)
                )
            });
            assert_eq!(has_ring, expect_ring, "{shape:?} {accent:?} contrast ring");

            let has_banner_text = list
                .cmds()
                .iter()
                .any(|cmd| matches!(cmd, Cmd::Glyph(g) if g.color == palette.on_primary));
            assert_eq!(
                has_banner_text, expect_text,
                "{shape:?} {accent:?} banner label"
            );
        }
    }
}
