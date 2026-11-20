//! The Confirm screen: the picture the phone sent, in full screen, as it will be the wallpaper, and
//! a small card at the bottom asking whether to use it (Cancel and Apply).
//!
//! The picture is the wallpaper texture itself: it becomes what "Minha imagem" shows while the
//! screen is open (no second texture). The server keeps listening, so the phone can frame again and
//! send another picture, which replaces the one shown. Apply keeps it off the loop and stops the server.

use super::editor::ButtonKind;
use super::painter::Painter;
use super::{Effect, LumeApp, PendingFocus, Screen};
use crate::i18n::StringId;
use crate::render::Rect;
use crate::text::TextStyle;
use crate::theme::{vars, StateFlags};
use crate::ui::focus::{FocusId, Focusable, Row};
use crate::wallpaper::custom::{self, Applied};
use crate::wallpaper::{Image, WallpaperId};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

pub const CANCEL: FocusId = 9121;
pub const APPLY: FocusId = 9122;

/// The card: 640 wide, centered, its base on the safe area's (720 - 18).
const CARD_WIDTH: f32 = 640.0;
const CARD_BOTTOM: f32 = 702.0;
const PAD_X: f32 = 32.0;
const PAD_Y: f32 = 24.0;
const BUTTON_HEIGHT: f32 = 48.0;
const BUTTON_GAP: f32 = 16.0;
const QUESTION_GAP: f32 = 24.0;
const LINE_HEIGHT: f32 = 24.0;
const BUTTON_MIN_WIDTH: f32 = 128.0;
const BUTTON_PAD_X: f32 = 28.0;

/// The received picture and, once Apply was pressed, the work that keeps it.
pub(super) struct Confirmation {
    pub(super) image: Arc<Image>,
    applying: Option<Receiver<std::io::Result<(Applied, Image)>>>,
}

impl LumeApp {
    /// A picture arrived from the phone: the Confirm screen shows it (or, already open, shows the
    /// new one instead).
    pub(super) fn show_received(&mut self, image: Image) {
        let image = Arc::new(image);
        custom::set_preview(Some(image.clone()));
        self.confirmation = Some(Confirmation {
            image,
            applying: None,
        });
        if self.screen != Screen::ConfirmImage {
            self.context_menu = None;
            self.screen = Screen::ConfirmImage;
            self.scroll_to_top();
            self.pending_focus = Some(PendingFocus::Id(APPLY));
            self.focus.set_rows(vec![]);
        }
    }

    /// Cancel or Back: nothing is applied and the Receive image screen comes back with the same
    /// code, for the phone to send again.
    pub(super) fn cancel_confirm(&mut self) -> Vec<Effect> {
        if let Some(receive) = self.receive.as_ref() {
            receive.session.declined();
        }
        self.end_confirmation();
        self.screen = Screen::ReceiveImage;
        self.pending_focus = Some(PendingFocus::Id(super::receive::CANCEL));
        self.focus.set_rows(vec![]);
        vec![]
    }

    /// The received picture leaves memory and the wallpaper goes back to what the configuration says.
    pub(super) fn end_confirmation(&mut self) {
        if self.confirmation.take().is_some() {
            custom::set_preview(None);
            crate::memory::release_freed();
        }
    }

    /// Whether Apply is keeping the picture off the loop: only Home leaves meanwhile.
    pub fn confirm_busy(&self) -> bool {
        self.confirmation
            .as_ref()
            .is_some_and(|c| c.applying.is_some())
    }

    pub(super) fn activate_confirm(&mut self, id: FocusId) -> Vec<Effect> {
        if self.confirm_busy() {
            return vec![];
        }
        match id {
            CANCEL => self.cancel_confirm(),
            APPLY => {
                self.apply_received();
                vec![]
            }
            _ => vec![],
        }
    }

    /// Apply: the page is told (and the server stops a little later), and the picture, its thumbnail
    /// and its cache are written off the loop.
    fn apply_received(&mut self) {
        let rev = self.config.custom.map_or(1, |c| c.rev.wrapping_add(1));
        let Some(files) = custom::files_dir() else {
            log::error!("custom wallpaper: no files directory to keep it in");
            return;
        };
        if let Some(receive) = self.receive.as_mut() {
            // The page hears it at its next ping; the server stops 10 s later.
            receive.session.applied(self.now_ms);
        }
        let Some(confirmation) = self.confirmation.as_mut() else {
            return;
        };
        let applied = Applied { rev };
        let (sender, receiver) = channel();
        let image = confirmation.image.clone();
        let spawned = std::thread::Builder::new()
            .name("lume-apply".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let result = custom::apply(
                    &files,
                    crate::wallpaper::cache_dir(),
                    lume_version::VERSION,
                    &image,
                    applied,
                )
                .map(|thumbnail| (applied, thumbnail));
                log::info!(
                    "custom wallpaper applied in {} ms",
                    started.elapsed().as_millis()
                );
                let _ = sender.send(result);
                crate::wallpaper::notify_ready();
            });
        match spawned {
            Ok(_) => confirmation.applying = Some(receiver),
            Err(e) => log::error!("cannot start the apply thread: {e}"),
        }
    }

    /// Takes the end of Apply, when it came: "Minha imagem" becomes the wallpaper and the editor
    /// comes back with the focus on its card. Returns the effects (saving the configuration), or
    /// `None` while it runs.
    pub(super) fn poll_apply(&mut self) -> Option<Vec<Effect>> {
        let confirmation = self.confirmation.as_mut()?;
        let result = confirmation.applying.as_ref()?.try_recv().ok()?;
        confirmation.applying = None;
        match result {
            Ok((applied, thumbnail)) => {
                self.config.custom = Some(applied);
                self.config.wallpaper = WallpaperId::Custom;
                custom::set_thumbnail(Some(thumbnail));
                custom::set_applied(Some(applied));
                self.end_confirmation();
                // The session lives on only to tell the page, then its server stops.
                self.lingering = self.receive.take().map(|r| r.session);
                let mut effects = self.back_to_editor_card();
                effects.push(Effect::SaveConfig);
                Some(effects)
            }
            Err(e) => {
                log::error!("custom wallpaper: cannot apply: {e}");
                Some(vec![])
            }
        }
    }

    /// Waits for Apply to finish (tests).
    #[cfg(test)]
    pub(super) fn wait_apply(&mut self) -> Vec<Effect> {
        for _ in 0..500 {
            if let Some(effects) = self.poll_apply() {
                return effects;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("Apply did not finish");
    }

    /// Opens the Confirm screen with a picture as if the phone had sent it (desktop and tests).
    #[doc(hidden)]
    pub fn confirm_with(&mut self, image: Image) {
        if self.receive.is_none() {
            self.open_receive();
        }
        self.show_received(image);
    }

    /// The card at the bottom: the question at the left, Cancel and Apply at the right.
    pub(super) fn draw_confirm(&mut self, p: &mut Painter) {
        let t = self.translator;
        let applying = self.confirm_busy();
        let card = Rect::new(
            (1280.0 - CARD_WIDTH) / 2.0,
            CARD_BOTTOM - (PAD_Y + BUTTON_HEIGHT + PAD_Y),
            CARD_WIDTH,
            PAD_Y + BUTTON_HEIGHT + PAD_Y,
        );
        let style = p.style(|t| &t.panel, StateFlags::NONE);
        p.draw_box(card, &style);
        let y = card.y + PAD_Y;
        let apply_label = t.get(StringId::ConfirmApply).into_owned();
        let cancel_label = t.get(StringId::EditorCancel).into_owned();
        let apply_w = button_width(p, &apply_label);
        let cancel_w = button_width(p, &cancel_label);
        let apply = Rect::new(card.right() - PAD_X - apply_w, y, apply_w, BUTTON_HEIGHT);
        let cancel = Rect::new(apply.x - BUTTON_GAP - cancel_w, y, cancel_w, BUTTON_HEIGHT);
        self.draw_button(p, cancel, &cancel_label, CANCEL, ButtonKind::Secondary);
        self.draw_button(p, apply, &apply_label, APPLY, ButtonKind::Primary);

        // The question, on one line or two, centered on the buttons.
        let question = if applying {
            t.get(StringId::ConfirmApplying)
        } else {
            t.get(StringId::ConfirmQuestion)
        };
        let question_style = TextStyle::new(18.0, 700);
        let x = card.x + PAD_X;
        let width = (cancel.x - QUESTION_GAP - x).max(0.0);
        let mut lines = super::editor::wrap(p, &question, &question_style, width);
        lines.truncate(2);
        let block = lines.len() as f32 * LINE_HEIGHT;
        let mut baseline = y + (BUTTON_HEIGHT - block) / 2.0 + 18.0;
        for line in &lines {
            let line = p.text.ellipsize(line, &question_style, width).into_owned();
            p.text
                .draw(p.list, &line, x, baseline, &question_style, vars::TEXT);
            baseline += LINE_HEIGHT;
        }
        self.update_focus(vec![Row::new(vec![
            Focusable::new(CANCEL, cancel),
            Focusable::new(APPLY, apply),
        ])
        .with_entry_down(APPLY)
        .with_entry_up(APPLY)]);
    }
}

fn button_width(p: &mut Painter, label: &str) -> f32 {
    let style = p.style(|t| &t.button, StateFlags::NONE);
    (p.measure(label, &style) + 2.0 * BUTTON_PAD_X)
        .max(BUTTON_MIN_WIDTH)
        .ceil()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_on_the_ruler() {
        super::super::assert_on_ruler(&[
            ("CARD_WIDTH", CARD_WIDTH),
            ("PAD_X", PAD_X),
            ("PAD_Y", PAD_Y),
            ("BUTTON_HEIGHT", BUTTON_HEIGHT),
            ("BUTTON_GAP", BUTTON_GAP),
            ("QUESTION_GAP", QUESTION_GAP),
        ]);
        // x 320 to 960, its base 18 above the bottom of 720.
        assert_eq!((1280.0 - CARD_WIDTH) / 2.0, 320.0);
        assert_eq!(CARD_BOTTOM, 720.0 - 18.0);
    }
}
