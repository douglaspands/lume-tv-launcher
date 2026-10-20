//! The Home tab: the carousel and the favorites row.

use super::painter::{fade, Painter};
use super::topbar::{self, TabLayout, TopBar};
use super::{ids, slide_effect, Anim, Effect, LumeApp, Screen, Tab};
use crate::home::config::Section;
use crate::home::slides::{build_slides, Slide};
use crate::i18n::StringId;
use crate::icons;
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{vars, Color, Gradient, GradientStop, Paint, StateFlags, THEME};
use crate::ui::focus::{Focusable, Row};
use crate::ui::scroll;

/// Space above the first section, below the top bar.
const CONTENT_TOP: f32 = topbar::HEIGHT;
const SECTION_GAP: f32 = 36.0;
const HERO_HEIGHT: f32 = 340.0;
const WIDTH: f32 = 1280.0 - 2.0 * topbar::MARGIN;
const TILE_GAP: f32 = 16.0;
const TILES_VISIBLE: f32 = 5.0;
/// Tiles are 16:9.
const TILE_WIDTH: f32 = (WIDTH - (TILES_VISIBLE - 1.0) * TILE_GAP) / TILES_VISIBLE;
const TILE_HEIGHT: f32 = TILE_WIDTH * 9.0 / 16.0;
/// Height of a section's title line and the space under it.
const TITLE_BASELINE: f32 = 19.0;
const TILES_OFFSET: f32 = 39.0;
const HERO_PADDING: f32 = 48.0;
/// The rows below the top bar are clipped at this height so scrolled content never overlaps it.
const CLIP_TOP: f32 = 76.0;

struct HomeGeometry {
    hero: Option<Rect>,
    hero_button: Rect,
    title: Option<Rect>,
    tiles: Vec<Rect>,
    add_tile: Option<Rect>,
    content_bottom: f32,
}

impl LumeApp {
    fn home_geometry(&self, p: &mut Painter, slide: Option<&Slide>) -> HomeGeometry {
        let mut y = CONTENT_TOP;
        let mut geometry = HomeGeometry {
            hero: None,
            hero_button: Rect::default(),
            title: None,
            tiles: Vec::new(),
            add_tile: None,
            content_bottom: y,
        };
        let sections: Vec<Section> = self.config.visible_sections().collect();
        for section in sections {
            match section {
                Section::Carousel => {
                    let hero = Rect::new(topbar::MARGIN, y, WIDTH, HERO_HEIGHT);
                    let style = p.style(&THEME.button, StateFlags::NONE);
                    let label = slide.map_or("", |s| s.button.as_str());
                    let width =
                        p.measure(label, &style) + style.padding[1] + style.padding[3] + 26.0;
                    geometry.hero_button = Rect::new(
                        hero.x + HERO_PADDING,
                        hero.y + 219.0,
                        width,
                        style.height.unwrap_or(48.0),
                    );
                    geometry.hero = Some(hero);
                    y += HERO_HEIGHT + SECTION_GAP;
                }
                Section::Favorites => {
                    geometry.title = Some(Rect::new(topbar::MARGIN, y, WIDTH, 23.6));
                    let top = y + TILES_OFFSET;
                    if self.config.favorites.is_empty() {
                        geometry.add_tile =
                            Some(Rect::new(topbar::MARGIN, top, TILE_WIDTH, TILE_HEIGHT));
                    } else {
                        for index in 0..self.config.favorites.len() {
                            let x = topbar::MARGIN + index as f32 * (TILE_WIDTH + TILE_GAP);
                            geometry
                                .tiles
                                .push(Rect::new(x, top, TILE_WIDTH, TILE_HEIGHT));
                        }
                    }
                    y = top + TILE_HEIGHT + SECTION_GAP;
                }
            }
        }
        geometry.content_bottom = y;
        geometry
    }

    /// The rows the focus moves through: the top bar, then one row per visible section.
    fn home_rows(&self, geometry: &HomeGeometry, tabs: &TabLayout) -> Vec<Row> {
        let mut rows = vec![topbar::row(tabs, Tab::Home)];
        for section in self.config.visible_sections() {
            match section {
                Section::Carousel => {
                    rows.push(Row::new(vec![Focusable::new(
                        ids::HERO_BUTTON,
                        geometry.hero_button,
                    )]));
                }
                Section::Favorites => {
                    let items: Vec<Focusable> = if let Some(add) = geometry.add_tile {
                        vec![Focusable::new(ids::FAVORITES_ADD, add)]
                    } else {
                        geometry
                            .tiles
                            .iter()
                            .enumerate()
                            .map(|(i, rect)| Focusable::new(ids::FAVORITE_BASE + i as u32, *rect))
                            .collect()
                    };
                    rows.push(Row::new(items));
                }
            }
        }
        rows
    }

    pub(super) fn carousel_deadline(&self) -> Option<u64> {
        // Only the home screen shows the carousel, so only it consumes the deadline: anywhere else
        // it would stay overdue and the loop would draw non-stop.
        if self.screen != Screen::Main(Tab::Home)
            || self.slides.len() < 2
            || self.focus.current_id() == Some(ids::HERO_BUTTON)
        {
            return None;
        }
        Some(self.carousel.since_ms + u64::from(self.config.interval.seconds()) * 1000)
    }

    pub(super) fn draw_home(&mut self, p: &mut Painter) {
        // Slides depend on the history and the clock; keep the index inside the list.
        self.slides = build_slides(
            &self.config,
            &self.catalog,
            &self.usage,
            self.clock.unix_secs,
            &self.translator,
        );
        if self.carousel.index >= self.slides.len() {
            self.carousel.index = 0;
            self.carousel.previous = None;
        }
        let hero_focused = self.focus.current_id() == Some(ids::HERO_BUTTON);
        if hero_focused {
            // The rotation pauses while the focus is on the carousel.
            self.carousel.since_ms = self.now_ms;
        } else if self.slides.len() > 1
            && self.now_ms
                >= self.carousel.since_ms + u64::from(self.config.interval.seconds()) * 1000
        {
            self.step_slide(1);
        }

        let slide = self.slides.get(self.carousel.index).cloned();
        let geometry = self.home_geometry(p, slide.as_ref());
        let tabs = TabLayout::new(
            p,
            &self.translator.get(StringId::TabHome),
            &self.translator.get(StringId::TabApps),
        );
        let rows = self.home_rows(&geometry, &tabs);
        self.update_focus(rows);

        // Scroll so the focused element stays visible.
        let focused = self.focus.current_id();
        let rect = focused.and_then(|id| self.home_rect_of(id, &geometry));
        let view_height = (p.display.visible_rect().bottom() - CONTENT_TOP).max(1.0);
        if let Some(id) = focused {
            if (ids::FAVORITE_BASE..ids::GRID_BASE).contains(&id) {
                self.reveal_favorites_x(id - ids::FAVORITE_BASE, &geometry);
            }
        }
        let in_content =
            focused.is_some_and(|id| id != ids::TAB_HOME && id != ids::TAB_APPS && id != ids::GEAR);
        self.reveal_vertical(
            if in_content { rect } else { None },
            geometry.content_bottom,
            CONTENT_TOP,
            view_height,
        );
        // Going up to the top bar brings the page back to the top.
        if focused.is_some_and(|id| id == ids::TAB_HOME || id == ids::TAB_APPS || id == ids::GEAR) {
            self.scroll_to(0.0);
        }

        let scroll_y = self.current_scroll_y();
        let scroll_x = self
            .anim
            .get(Anim::FavoritesScrollX, self.favorites_scroll_x, self.now_ms);

        // Background of the whole surface.
        let visible = p.display.visible_rect();
        p.list.fill(
            visible,
            0.0,
            THEME.app.base.background.unwrap_or(Paint::Solid(vars::BG)),
        );

        // Content, clipped below the top bar.
        p.list.push_clip(Rect::new(
            visible.x,
            CLIP_TOP,
            visible.w,
            visible.bottom() - CLIP_TOP,
        ));
        if let (Some(hero), Some(slide)) = (geometry.hero, &slide) {
            self.draw_hero(
                p,
                hero.translate(0.0, -scroll_y),
                geometry.hero_button.translate(0.0, -scroll_y),
                slide,
            );
        }
        if let Some(title) = geometry.title {
            let style = p.style(&THEME.section_title, StateFlags::NONE);
            let text = self.translator.get(StringId::HomeFavoritesTitle);
            p.text.draw(
                p.list,
                &text,
                title.x,
                title.y - scroll_y + TITLE_BASELINE,
                &Painter::text_style(&style),
                style.color,
            );
        }
        self.draw_favorites(p, &geometry, scroll_x, scroll_y);
        p.list.pop_clip();

        let progress = |id: u32| self.focus_progress(id);
        TopBar {
            tabs: &tabs,
            active: Tab::Home,
            home_label: &self.translator.get(StringId::TabHome),
            apps_label: &self.translator.get(StringId::TabApps),
            clock: &self.clock,
            focus: &progress,
        }
        .draw(p);
    }

    fn home_rect_of(&self, id: u32, geometry: &HomeGeometry) -> Option<Rect> {
        match id {
            ids::HERO_BUTTON => geometry.hero,
            ids::FAVORITES_ADD => geometry.add_tile,
            id if id >= ids::FAVORITE_BASE => geometry
                .tiles
                .get((id - ids::FAVORITE_BASE) as usize)
                .copied(),
            _ => None,
        }
    }

    /// Scrolls the favorites row sideways so the focused tile is fully in view.
    fn reveal_favorites_x(&mut self, index: u32, geometry: &HomeGeometry) {
        let Some(tile) = geometry.tiles.get(index as usize) else {
            return;
        };
        let total = geometry
            .tiles
            .last()
            .map_or(0.0, |t| t.right() - topbar::MARGIN);
        let max = scroll::max_offset(total, WIDTH);
        let target = scroll::reveal(
            self.favorites_scroll_x,
            WIDTH,
            tile.x - topbar::MARGIN,
            tile.right() - topbar::MARGIN,
            0.0,
            max,
        );
        if (target - self.favorites_scroll_x).abs() > 0.01 {
            let transition = self.focus_transition();
            self.anim.animate_to(
                Anim::FavoritesScrollX,
                self.favorites_scroll_x,
                target,
                self.now_ms,
                transition,
            );
            self.favorites_scroll_x = target;
        }
    }

    fn draw_favorites(
        &mut self,
        p: &mut Painter,
        geometry: &HomeGeometry,
        scroll_x: f32,
        scroll_y: f32,
    ) {
        if let Some(add) = geometry.add_tile {
            let progress = self.focus_progress(ids::FAVORITES_ADD);
            let mut style = p.focus_style(&THEME.tile, progress);
            style.background = Some(Paint::Solid(vars::SURFACE_ROW));
            style.color = vars::TEXT_MUTED;
            let rect = add.translate(0.0, -scroll_y);
            p.draw_box(rect, &style);
            let label = self.translator.get(StringId::HomeAddFavorites);
            p.text(&label, rect.grow(-12.0), Align::Center, &style);
            return;
        }
        let mut tiles = Vec::new();
        for (index, rect) in geometry.tiles.iter().enumerate() {
            let package = &self.config.favorites[index];
            let Some(app) = self.catalog.get(package) else {
                continue;
            };
            let rect = rect.translate(-scroll_x, -scroll_y);
            let progress = self.focus_progress(ids::FAVORITE_BASE + index as u32);
            let mut style = p.focus_style(&THEME.tile, progress);
            style.background = Some(Paint::Solid(app.color));
            tiles.push((app, rect, style));
        }
        // Boxes, then images, then text: one draw call for each kind (see `draw_tile_image`).
        for (_, rect, style) in &tiles {
            p.draw_box(*rect, style);
        }
        for (app, rect, style) in &tiles {
            self.draw_tile_image(p, Some(&app.package), *rect, style, true);
        }
        for (app, rect, style) in &tiles {
            self.draw_tile_name(p, Some(&app.package), &app.label, *rect, style, true);
        }
    }

    /// Draws the carousel box. While a slide change is fading, the previous slide is under the new one.
    fn draw_hero(&mut self, p: &mut Painter, hero: Rect, button: Rect, slide: &Slide) {
        let fade_progress = self.anim.get(Anim::SlideFade, 1.0, self.now_ms);
        if let (Some(previous), true) = (self.carousel.previous, fade_progress < 1.0) {
            if let Some(old) = self.slides.get(previous).cloned() {
                self.draw_slide(p, hero, button, &old, 1.0, false);
            }
            self.draw_slide(p, hero, button, slide, fade_progress, true);
        } else {
            self.draw_slide(p, hero, button, slide, 1.0, true);
        }
        self.draw_pager(p, hero);
    }

    fn draw_slide(
        &mut self,
        p: &mut Painter,
        hero: Rect,
        button: Rect,
        slide: &Slide,
        opacity: f32,
        interactive: bool,
    ) {
        let mut style = p.style(&THEME.hero, StateFlags::NONE);
        style.background = style.background.map(|paint| tint(paint, slide.color));
        style.opacity = opacity;
        p.draw_box(hero, &style);

        let left = hero.x + HERO_PADDING;
        // Badge: a dot in the app's color and the app's name.
        let badge_style = p.style(&THEME.hero_badge, StateFlags::NONE);
        let text_width = p.measure(&slide.badge, &badge_style);
        let badge_height = badge_style.height.unwrap_or(30.0);
        let badge = Rect::new(
            left,
            hero.y + 73.0,
            14.0 + 10.0 + 8.0 + text_width + 16.0,
            badge_height,
        );
        let mut pill = badge_style;
        pill.opacity = opacity;
        p.draw_box(badge, &pill);
        let dot = Rect::new(badge.x + 14.0, badge.center().1 - 5.0, 10.0, 10.0);
        p.list
            .fill(dot, 5.0, Paint::Solid(fade(brighten(slide.color), opacity)));
        let text_rect = Rect::new(dot.right() + 8.0, badge.y, text_width + 8.0, badge.h);
        p.text(&slide.badge, text_rect, Align::Start, &pill);

        // Title and context line.
        let title_style = p.style(&THEME.hero_title, StateFlags::NONE);
        let title = p
            .text
            .ellipsize(&slide.title, &Painter::text_style(&title_style), 700.0)
            .into_owned();
        p.text.draw(
            p.list,
            &title,
            left,
            hero.y + 159.0,
            &Painter::text_style(&title_style),
            fade(title_style.color, opacity),
        );
        let context_style = p.style(&THEME.hero_subtitle, StateFlags::NONE);
        let context = p
            .text
            .ellipsize(&slide.context, &Painter::text_style(&context_style), 700.0)
            .into_owned();
        p.text.draw(
            p.list,
            &context,
            left,
            hero.y + 195.0,
            &Painter::text_style(&context_style),
            fade(context_style.color, opacity),
        );

        // The button.
        let progress = if interactive {
            self.focus_progress(ids::HERO_BUTTON)
        } else {
            0.0
        };
        let mut button_style = p.focus_style(&THEME.button, progress);
        button_style.opacity = opacity;
        p.draw_box(button, &button_style);
        let icon_rect = Rect::new(button.x + 22.0, button.y, 18.0, button.h);
        p.icon(
            icons::PLAY,
            icon_rect,
            16.0,
            fade(button_style.color, opacity),
        );
        let label_rect = Rect::new(
            button.x + 22.0 + 18.0 + 10.0,
            button.y,
            button.w - 22.0 - 18.0 - 10.0 - 18.0,
            button.h,
        );
        p.text(&slide.button, label_rect, Align::Start, &button_style);
    }

    fn draw_pager(&mut self, p: &mut Painter, hero: Rect) {
        let count = self.slides.len();
        if count < 2 {
            return;
        }
        let dot = p.style(&THEME.pager_dot, StateFlags::NONE);
        let active = p.style(&THEME.pager_dot, StateFlags::ACTIVE);
        let (dot_size, gap) = (dot.height.unwrap_or(10.0), 10.0);
        let active_width = active.width.unwrap_or(28.0);
        let total = (count - 1) as f32 * (dot.width.unwrap_or(10.0) + gap) + active_width;
        let mut x = hero.right() - 28.0 - total;
        let y = hero.bottom() - 24.0 - dot_size;
        for index in 0..count {
            let selected = index == self.carousel.index;
            let style = if selected { &active } else { &dot };
            let width = if selected {
                active_width
            } else {
                dot.width.unwrap_or(10.0)
            };
            let background = style.background.unwrap_or(Paint::Solid(Color::WHITE));
            p.list
                .fill(Rect::new(x, y, width, dot_size), dot_size / 2.0, background);
            x += width + gap;
        }
    }

    pub(super) fn activate_home(&mut self, id: u32) -> Vec<Effect> {
        match id {
            ids::HERO_BUTTON => match self.slides.get(self.carousel.index).cloned() {
                Some(slide) => slide_effect(self, &slide),
                None => vec![],
            },
            ids::FAVORITES_ADD => {
                self.open_editor();
                vec![]
            }
            id if id >= ids::FAVORITE_BASE => {
                let package = self
                    .config
                    .favorites
                    .get((id - ids::FAVORITE_BASE) as usize)
                    .cloned();
                package.map(|p| self.open_app(&p)).unwrap_or_default()
            }
            _ => vec![],
        }
    }
}

/// Tints the hero gradient toward the app's color, keeping it dark.
fn tint(paint: Paint, app_color: Color) -> Paint {
    let Paint::Gradient(gradient) = paint else {
        return paint;
    };
    let stops: Vec<GradientStop> = gradient
        .stops()
        .iter()
        .map(|stop| GradientStop {
            color: crate::ui::style::mix_color(stop.color, app_color, 0.10),
            ..*stop
        })
        .collect();
    Paint::Gradient(Gradient::new(gradient.angle_deg, &stops))
}

/// A lighter shade of the app color for the small dot next to its name.
pub(super) fn brighten(color: Color) -> Color {
    let lift = |c: u8| (f32::from(c) * 1.3 + 20.0).min(255.0) as u8;
    Color {
        r: lift(color.r),
        g: lift(color.g),
        b: lift(color.b),
        a: 255,
    }
}
