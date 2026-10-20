//! The splash: the logo fading in on `--bg` (the color of the system's starting window), held while
//! the screen behind it gets the images it shows, and then the reveal of that screen. All of it is
//! drawn by lume: the system's window has only the background color.
//!
//! ```text
//! came_to_foreground --> Splash --(images ready | 600 ms)--> Revealing --(end)--> Normal
//!                              \--(a key other than Home)-----------------------> Normal
//! ```
//!
//! While the splash waits nothing is animated, so the platform keeps loading icons (it only does
//! when the screen is at rest) and no frames are asked for besides the 600 ms deadline.

use super::painter::{fade, Painter};
use super::{apps, Anim, LumeApp, Screen, Tab};
use crate::render::Rect;
use crate::theme::{vars, Transition};
use std::collections::HashSet;

/// The longest the splash waits for the images, from its first frame.
pub(super) const SPLASH_LIMIT_MS: u64 = 600;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Phase {
    Normal,
    Splash(Waiting),
    /// The screen appears (see [`Anim::Reveal`]).
    Revealing,
}

/// The splash while it waits for the images of the screen behind it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Waiting {
    /// When the first splash frame was drawn.
    since: Option<u64>,
    /// lume was just started (the process did not exist): the reveal takes longer.
    cold: bool,
    /// The images of the screen behind that are ready (or failed), and how many it shows.
    ready: usize,
    expected: usize,
    /// The apps whose dock icon or tile image the platform could not give: they count as ready.
    failed_dock: HashSet<String>,
    failed_tiles: HashSet<String>,
    /// The tiles of the Apps grid the splash waits for that have no image yet, for the platform to
    /// load first.
    missing_tiles: Vec<String>,
}

impl Waiting {
    pub(super) fn new(cold: bool) -> Waiting {
        Waiting {
            cold,
            ..Waiting::default()
        }
    }

    /// How much of the screen's images are ready, 0.0 to 1.0 (1.0 when it shows none). Nothing
    /// shows it yet: it is there for a loading indicator, see [`LumeApp::draw_splash_status`].
    pub(super) fn progress(&self) -> f32 {
        if self.expected == 0 {
            1.0
        } else {
            self.ready as f32 / self.expected as f32
        }
    }
}

/// An image the screen behind the splash shows.
enum Image {
    Dock(String),
    Tile(String),
}

impl LumeApp {
    /// lume is on screen again: the splash comes first.
    pub(super) fn start_splash(&mut self) {
        let cold = !self.woke_before;
        self.woke_before = true;
        self.anim.set(Anim::Reveal, 0.0);
        self.phase = Phase::Splash(Waiting::new(cold));
    }

    /// Ends the splash at once, without the reveal (a key was pressed).
    pub(super) fn cut_splash(&mut self) {
        if matches!(self.phase, Phase::Splash(_)) {
            self.phase = Phase::Normal;
            self.anim.set(Anim::Reveal, 1.0);
            self.interface_ready = true;
        }
    }

    /// The platform could not give the dock icon (`dock`) or the tile image of `package`.
    pub fn icon_failed(&mut self, package: &str, dock: bool) {
        if let Phase::Splash(waiting) = &mut self.phase {
            let set = if dock {
                &mut waiting.failed_dock
            } else {
                &mut waiting.failed_tiles
            };
            set.insert(package.to_string());
        }
    }

    /// Whether the splash is on screen, waiting for the images.
    pub fn in_splash(&self) -> bool {
        matches!(self.phase, Phase::Splash(_))
    }

    /// How much of the images the splash waits for are ready (1.0 out of the splash).
    pub fn splash_progress(&self) -> f32 {
        match &self.phase {
            Phase::Splash(waiting) => waiting.progress(),
            _ => 1.0,
        }
    }

    /// Whether the interface is being revealed after the splash.
    pub fn is_revealing(&self) -> bool {
        self.phase == Phase::Revealing
    }

    /// The tiles of the Apps grid the splash still waits for.
    pub fn splash_missing_tiles(&self) -> &[String] {
        match &self.phase {
            Phase::Splash(waiting) => &waiting.missing_tiles,
            _ => &[],
        }
    }

    /// Whether the interface became ready (the reveal ended, or a key cut the splash) since the
    /// last call.
    pub fn take_interface_ready(&mut self) -> bool {
        std::mem::take(&mut self.interface_ready)
    }

    /// When the splash stops waiting if the images are not all there.
    pub fn splash_deadline(&self) -> Option<u64> {
        match &self.phase {
            Phase::Splash(waiting) => waiting.since.map(|since| since + SPLASH_LIMIT_MS),
            _ => None,
        }
    }

    /// How far the reveal has got: 0.0 during the splash, 1.0 once the interface is all there.
    pub(super) fn reveal(&self) -> f32 {
        match self.phase {
            Phase::Normal => 1.0,
            Phase::Splash(_) => 0.0,
            Phase::Revealing => self.anim.get(Anim::Reveal, 1.0, self.now_ms),
        }
    }

    /// Called at the start of a frame: the splash notes its first frame (where the logo starts to
    /// fade in), and the reveal ends.
    pub(super) fn splash_frame_start(&mut self, now_ms: u64) {
        match &mut self.phase {
            Phase::Splash(waiting) if waiting.since.is_none() => {
                waiting.since = Some(now_ms);
                // The value at rest is 1.0, so the start is given (see `Animations::prune`).
                self.anim.animate_to(
                    Anim::SplashLogo,
                    0.0,
                    1.0,
                    now_ms,
                    Some(vars::SPLASH_LOGO_IN),
                );
            }
            Phase::Revealing if !self.anim.is_animating(now_ms) => {
                self.phase = Phase::Normal;
                self.interface_ready = true;
            }
            _ => {}
        }
    }

    /// Called after the screen was laid out and drawn: during the splash its drawing is dropped
    /// (the layout and the focus stay) and the splash checks the images; during the reveal the
    /// drawing comes up and in. Then the logo goes on top.
    ///
    /// `content_end` is where the top bar starts: the bar fades in where it is, the rest rises.
    pub(super) fn splash_frame_end(&mut self, p: &mut Painter, content_end: usize) {
        match self.phase {
            Phase::Normal => return,
            Phase::Splash(_) => {
                // Only the layout of the screen is kept. Drawing it invisibly, to warm up the GPU,
                // made the first frame slower and the reveal no smoother on the TV.
                p.list.truncate(self.content_start);
                // The screen settles behind the logo (a restored focus does not animate): once the
                // logo is in nothing moves, so the platform keeps loading icons and no frames are
                // asked for.
                self.anim.settle(|key| *key == Anim::SplashLogo);
                self.check_splash_images(p);
            }
            Phase::Revealing => {
                let t = self.reveal();
                p.list.translate_range(
                    self.content_start,
                    content_end,
                    vars::SPLASH_RISE * (1.0 - t),
                );
                p.list.fade_from(self.content_start, t);
            }
        }
        let logo_in = self.anim.get(Anim::SplashLogo, 1.0, self.now_ms);
        self.draw_splash(p, logo_in * (1.0 - self.reveal()));
    }

    /// Counts the images of the screen behind that are ready and starts the reveal when they all are,
    /// or when the splash has waited long enough.
    fn check_splash_images(&mut self, p: &Painter) {
        let images = self.splash_images(p);
        let Phase::Splash(waiting) = &mut self.phase else {
            return;
        };
        waiting.expected = images.len();
        waiting.missing_tiles.clear();
        waiting.ready = 0;
        for image in images {
            let ready = match &image {
                Image::Dock(package) => {
                    self.icons.has_dock(package) || waiting.failed_dock.contains(package)
                }
                Image::Tile(package) => {
                    self.icons.has(package) || waiting.failed_tiles.contains(package)
                }
            };
            if ready {
                waiting.ready += 1;
            } else if let Image::Tile(package) = image {
                waiting.missing_tiles.push(package);
            }
        }
        let timed_out = waiting
            .since
            .is_some_and(|since| self.now_ms >= since + SPLASH_LIMIT_MS);
        if waiting.ready == waiting.expected || timed_out {
            let transition: Transition = if waiting.cold {
                vars::SPLASH_REVEAL_COLD
            } else {
                vars::SPLASH_REVEAL_WARM
            };
            self.phase = Phase::Revealing;
            // The value at rest is 1.0, so the start is given (see `Animations::prune`).
            self.anim
                .animate_to(Anim::Reveal, 0.0, 1.0, self.now_ms, Some(transition));
        }
    }

    /// The images the splash waits for, so the user never sees an icon arrive: the dock icons and
    /// the tiles the Apps grid shows (where it was scrolled to, when it is the screen behind; its
    /// first screen otherwise, which is what a move to the Apps tab shows).
    fn splash_images(&self, p: &Painter) -> Vec<Image> {
        let scroll_y = if self.screen == Screen::Main(Tab::Apps) {
            self.scroll_y
        } else {
            0.0
        };
        let dock = self
            .dock_items()
            .into_iter()
            .filter_map(|item| item.package.map(Image::Dock));
        let tiles = self
            .visible_grid_packages(apps::view_bottom(p), scroll_y)
            .into_iter()
            .map(Image::Tile);
        dock.chain(tiles).collect()
    }

    /// Where the splash logo is: centered on the screen, at the theme's size.
    pub(super) fn splash_logo_rect(p: &Painter) -> Rect {
        let visible = p.display.visible_rect();
        let size = vars::SPLASH_LOGO;
        Rect::new(
            p.display.snap(visible.x + (visible.w - size) / 2.0),
            p.display.snap(visible.y + (visible.h - size) / 2.0),
            size,
            size,
        )
    }

    /// The splash at `opacity`: the logo, and under it the band kept for a loading indicator.
    fn draw_splash(&self, p: &mut Painter, opacity: f32) {
        if opacity <= 0.0 {
            return;
        }
        let logo = Self::splash_logo_rect(p);
        crate::logo::draw_mark(
            p.list,
            logo,
            fade(vars::PRIMARY, opacity),
            fade(vars::BG, opacity),
        );
        let visible = p.display.visible_rect();
        let status = Rect::new(
            visible.x,
            logo.bottom() + vars::SPLASH_STATUS_TOP,
            visible.w,
            vars::SPLASH_STATUS_HEIGHT,
        );
        self.draw_splash_status(p, self.splash_progress(), status);
    }

    /// The place for a loading indicator (a bar, dots, "Loading apps…") in `area`, under the logo.
    /// It draws nothing for now: the splash is short. Should one be added, it must only appear
    /// once the wait passes a threshold (about 300 ms), so quick starts do not flash it.
    fn draw_splash_status(&self, _p: &mut Painter, _progress: f32, _area: Rect) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_counts_the_ready_images() {
        let mut waiting = Waiting::new(false);
        assert_eq!(waiting.progress(), 1.0, "a screen without images is ready");
        waiting.expected = 4;
        assert_eq!(waiting.progress(), 0.0);
        waiting.ready = 1;
        assert_eq!(waiting.progress(), 0.25);
        waiting.ready = 4;
        assert_eq!(waiting.progress(), 1.0);
    }
}
