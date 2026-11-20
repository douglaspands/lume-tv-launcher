//! The Receive image screen: the QR code and the address the phone opens, the code, the steps and
//! where the reception is. The companion server lives as long as this screen and the Confirm screen
//! that follows it.

use super::editor::{ButtonKind, PHONE_CARD};
use super::page::{self, LEFT};
use super::painter::Painter;
use super::{Effect, LumeApp, PendingFocus, Screen};
use crate::companion::server::Reject;
use crate::companion::{Session, Status};
use crate::i18n::{Arg, StringId};
use crate::icons;
use crate::render::Rect;
use crate::text::TextStyle;
use crate::theme::{vars, AccentColor, Color, Paint, StateFlags};
use crate::ui::focus::{FocusId, Focusable, Row};

pub const CANCEL: FocusId = 9000;
pub const NEW_CODE: FocusId = 9001;

const TOP: f32 = page::PLAIN_CONTENT_TOP;
/// The white square of the QR code.
const QR_SIDE: f32 = 320.0;
/// The panel at the right of the QR code.
const PANEL_X: f32 = 432.0;
const PANEL_RIGHT: f32 = 1216.0;
const PANEL_PAD_X: f32 = 32.0;
const PANEL_PAD_Y: f32 = 24.0;
const STEP_PITCH: f32 = 32.0;
const BUTTON_HEIGHT: f32 = 48.0;
const BUTTON_GAP: f32 = 24.0;
const BUTTON_MIN_WIDTH: f32 = 160.0;
/// Space between the letters of the code.
const CODE_TRACKING: f32 = 8.0;

/// The screen's state: the session (the server, its code and address) and the QR of its address.
pub(super) struct ReceiveState {
    pub(super) session: Session,
    qr: Option<(String, super::qr::Modules)>,
}

impl ReceiveState {
    fn new(now_ms: u64, accent: AccentColor) -> ReceiveState {
        ReceiveState {
            session: Session::open_with_accent(now_ms, accent),
            qr: None,
        }
    }

    /// The QR of the session's address, encoded again only when the address changes.
    fn qr(&mut self) -> Option<&super::qr::Modules> {
        let url = self.session.qr_url()?;
        if self.qr.as_ref().map(|(u, _)| u) != Some(&url) {
            let modules = super::qr::encode(&url)?;
            self.qr = Some((url, modules));
        }
        self.qr.as_ref().map(|(_, m)| m)
    }
}

impl LumeApp {
    /// Opens the Receive image screen: a new code and, with a network, the server.
    pub(super) fn open_receive(&mut self) {
        self.context_menu = None;
        self.receive = Some(Box::new(ReceiveState::new(
            self.now_ms,
            self.effective_accent(),
        )));
        self.screen = Screen::ReceiveImage;
        self.scroll_to_top();
        self.pending_focus = Some(PendingFocus::Id(CANCEL));
        self.focus.set_rows(vec![]);
    }

    /// Leaves the Receive image screen for the editor, with the focus on the phone card; the server
    /// stops.
    pub(super) fn close_receive(&mut self) -> Vec<Effect> {
        self.end_receive();
        self.back_to_editor_card()
    }

    /// Ends the phone flow (Home, Back): the server stops and a received picture is let go.
    pub(super) fn end_receive(&mut self) {
        if let Some(mut receive) = self.receive.take() {
            receive.session.close();
        }
        self.lingering = None;
        self.end_confirmation();
    }

    /// The editor again, with the focus on the phone card.
    pub(super) fn back_to_editor_card(&mut self) -> Vec<Effect> {
        let from_home = self.editor.from_home;
        self.open_editor(from_home);
        self.pending_focus = Some(PendingFocus::Id(PHONE_CARD));
        vec![]
    }

    pub(super) fn activate_receive(&mut self, id: FocusId) -> Vec<Effect> {
        match id {
            CANCEL => self.close_receive(),
            NEW_CODE => {
                let now = self.now_ms;
                if let Some(receive) = self.receive.as_mut() {
                    receive.session.renew(now);
                }
                self.pending_focus = Some(PendingFocus::Id(CANCEL));
                vec![]
            }
            _ => vec![],
        }
    }

    /// Takes the server's news. Returns whether the screen changed. An accepted picture opens the
    /// Confirm screen, or replaces the one it shows.
    pub fn poll_companion(&mut self, now_ms: u64) -> bool {
        let Some(receive) = self.receive.as_mut() else {
            return false;
        };
        let changed = receive.session.poll(now_ms);
        let status = receive.session.status();
        let image = receive.session.take_image();
        if changed && status.is_closed() && self.screen == Screen::ReceiveImage {
            // The server is off for good: the way on is a new code.
            self.pending_focus = Some(PendingFocus::Id(NEW_CODE));
        }
        if let Some(image) = image {
            if !self.confirm_busy() {
                self.show_received(image);
            }
            return true;
        }
        changed
    }

    /// What the QR of the Receive image screen carries, while it shows one (the desktop prints it).
    pub fn companion_url(&self) -> Option<String> {
        self.receive.as_ref().and_then(|r| r.session.qr_url())
    }

    /// Whether the Receive image or the Confirm screen is open (the server may be listening): the
    /// platform asks for its news.
    pub fn companion_active(&self) -> bool {
        self.receive.is_some() || self.lingering.is_some()
    }

    /// When the screen needs a frame without news: its countdown, or its look for a network.
    ///
    /// Counted from the last frame (`self.now_ms`), not from `now_ms`: a deadline counted from the
    /// moment it is asked would slide away at every turn of the loop and the countdown would never
    /// get its frame.
    pub(super) fn receive_deadline(&self, _now_ms: u64) -> Option<u64> {
        self.receive
            .as_ref()
            .filter(|_| self.foreground)
            .and_then(|r| r.session.next_wake_ms(self.now_ms))
    }

    /// lume leaves the screens: the server stops before it goes to the background.
    pub(super) fn receive_to_background(&mut self) {
        if let Some(receive) = self.receive.as_mut() {
            receive.session.close();
        }
        self.lingering = None;
    }

    /// lume is back: the screens get a new code and their server again.
    pub(super) fn receive_to_foreground(&mut self) {
        let now = self.now_ms;
        if let Some(receive) = self.receive.as_mut() {
            if !receive.session.is_listening() {
                receive.session.renew(now);
            }
        }
    }

    pub(super) fn draw_receive(&mut self, p: &mut Painter) {
        let t = self.translator;
        let now = self.now_ms;
        page::draw_header(
            p,
            &[
                &t.get(StringId::SettingsTitle),
                &t.get(StringId::SettingsHomeScreen),
                &t.get(StringId::ReceiveTitle),
            ],
            &t.get(StringId::ReceiveTitle),
            None,
            0.0,
        );
        let Some(mut receive) = self.receive.take() else {
            return;
        };
        let status = receive.session.status();
        let square = Rect::new(LEFT, TOP, QR_SIDE, QR_SIDE);
        let qr = (status != Status::NoNetwork && !status.is_closed())
            .then(|| receive.qr().cloned())
            .flatten();
        match &qr {
            Some(modules) => {
                p.list
                    .fill(square, vars::SHAPE_MD, Paint::Solid(Color::WHITE));
                super::qr::draw(p.list, &p.display, square, modules, Color::rgb(0, 0, 0));
            }
            None => self.draw_qr_placeholder(p, square, status),
        }

        // The panel: the steps, the address and the code, and the state at its base.
        let panel = Rect::new(PANEL_X, TOP, PANEL_RIGHT - PANEL_X, QR_SIDE);
        let style = p.style(|t| &t.panel, StateFlags::NONE);
        p.draw_box(panel, &style);
        let x = panel.x + PANEL_PAD_X;
        let inner = panel.w - 2.0 * PANEL_PAD_X;
        let step_style = TextStyle::new(18.0, 500);
        let mut baseline = panel.y + PANEL_PAD_Y + 18.0;
        for id in [
            StringId::ReceiveStepNetwork,
            StringId::ReceiveStepScan,
            StringId::ReceiveStepSend,
        ] {
            let line = p
                .text
                .ellipsize(&t.get(id), &step_style, inner)
                .into_owned();
            p.text
                .draw(p.list, &line, x, baseline, &step_style, vars::TEXT_MUTED);
            baseline += STEP_PITCH;
        }
        if let (Some(address), true) = (receive.session.address(), qr.is_some()) {
            let label_style = TextStyle::new(16.0, 500);
            let label_y = baseline + 20.0;
            let address_style = TextStyle::new(24.0, 700);
            let code_style = TextStyle::new(40.0, 800);
            let code = receive.session.code().to_string();
            let code_width = code
                .chars()
                .map(|c| p.text.measure(&c.to_string(), &code_style))
                .sum::<f32>()
                + CODE_TRACKING * (code.chars().count().saturating_sub(1)) as f32;
            let code_x = panel.right() - PANEL_PAD_X - code_width.max(96.0);
            let address_width = (code_x - 24.0 - x).max(0.0);
            p.text.draw(
                p.list,
                &t.get(StringId::ReceiveAddress),
                x,
                label_y,
                &label_style,
                vars::TEXT_MUTED,
            );
            let address = p
                .text
                .ellipsize(&address, &address_style, address_width)
                .into_owned();
            p.text.draw(
                p.list,
                &address,
                x,
                label_y + 36.0,
                &address_style,
                vars::TEXT,
            );
            p.text.draw(
                p.list,
                &t.get(StringId::ReceiveCode),
                code_x,
                label_y,
                &label_style,
                vars::TEXT_MUTED,
            );
            let mut cx = code_x;
            for c in code.chars() {
                let letter = c.to_string();
                p.text.draw(
                    p.list,
                    &letter,
                    cx,
                    label_y + 44.0,
                    &code_style,
                    p.primary_on_dark(),
                );
                cx += p.text.measure(&letter, &code_style) + CODE_TRACKING;
            }
        }
        let (text, color) = self.receive_status_text(&receive.session, now);
        let status_style = TextStyle::new(18.0, 700);
        let text = p.text.ellipsize(&text, &status_style, inner).into_owned();
        p.list.fill(
            Rect::new(x, panel.bottom() - 64.0, inner, 1.0),
            0.0,
            Paint::Solid(vars::SURFACE_ROW),
        );
        p.text.draw(
            p.list,
            &text,
            x,
            panel.bottom() - PANEL_PAD_Y - 4.0,
            &status_style,
            color,
        );

        // The buttons: Cancel under the QR code; "Make a new code" once the server is off for good.
        let button_y = TOP + QR_SIDE + BUTTON_GAP;
        let button_style = p.style(|t| &t.button, StateFlags::NONE);
        let cancel_label = t.get(StringId::EditorCancel).into_owned();
        let cancel_w = (p.measure(&cancel_label, &button_style) + 56.0).max(BUTTON_MIN_WIDTH);
        let cancel = Rect::new(LEFT, button_y, cancel_w, BUTTON_HEIGHT);
        let mut items = vec![Focusable::new(CANCEL, cancel)];
        let renew = status.is_closed();
        if renew {
            let label = t.get(StringId::ReceiveNewCode).into_owned();
            let w = (p.measure(&label, &button_style) + 56.0).max(BUTTON_MIN_WIDTH);
            let rect = Rect::new(PANEL_X, button_y, w, BUTTON_HEIGHT);
            items.push(Focusable::new(NEW_CODE, rect));
            self.draw_button(p, rect, &label, NEW_CODE, ButtonKind::Primary);
        }
        self.draw_button(p, cancel, &cancel_label, CANCEL, ButtonKind::Secondary);
        self.receive = Some(receive);
        self.update_focus(vec![Row::new(items)]);
    }

    /// The QR's square without a QR: no network, or the server is off.
    fn draw_qr_placeholder(&mut self, p: &mut Painter, square: Rect, status: Status) {
        let t = self.translator;
        p.list
            .fill(square, vars::SHAPE_MD, Paint::Solid(vars::SURFACE));
        p.list.ring(square, vars::SHAPE_MD, 2.0, vars::BORDER);
        let icon = Rect::new(
            square.x + (square.w - 48.0) / 2.0,
            square.y + 72.0,
            48.0,
            48.0,
        );
        p.icon(icons::PHONE, icon, 48.0, vars::TEXT_MUTED);
        let message = if status == Status::NoNetwork {
            t.get(StringId::ReceiveNoNetwork).into_owned()
        } else {
            self.receive_status_text_closed(status)
        };
        let style = TextStyle::new(20.0, 700);
        let mut y = square.y + 172.0;
        for line in super::editor::wrap(p, &message, &style, square.w - 48.0) {
            let w = p.text.measure(&line, &style);
            p.text.draw(
                p.list,
                &line,
                square.x + (square.w - w) / 2.0,
                y,
                &style,
                vars::TEXT,
            );
            y += 28.0;
        }
    }

    fn receive_status_text_closed(&self, status: Status) -> String {
        let t = self.translator;
        let id = match status {
            Status::TooManyAttempts => StringId::ReceiveTooMany,
            Status::PortError => StringId::ReceivePortError,
            _ => StringId::ReceiveTimedOut,
        };
        t.get(id).into_owned()
    }

    /// The state line at the base of the panel, and its color.
    pub(super) fn receive_status_text(&self, session: &Session, now_ms: u64) -> (String, Color) {
        let t = self.translator;
        let primary = self.active_accent().primary;
        match session.status() {
            Status::NoNetwork => (
                t.get(StringId::ReceiveNoNetworkHint).into_owned(),
                vars::TEXT_MUTED,
            ),
            Status::Waiting if session.connected(now_ms) => {
                (t.get(StringId::ReceiveConnected).into_owned(), primary)
            }
            Status::Waiting => match session.countdown(now_ms) {
                Some(left) => {
                    let time = super::tools::common::format_remaining(left * 1000);
                    (
                        t.format(StringId::ReceiveExpires, &[Arg::Str(&time)]),
                        vars::TEXT,
                    )
                }
                None => (t.get(StringId::ReceiveWaiting).into_owned(), vars::TEXT),
            },
            Status::Receiving(percent) => (
                t.format(StringId::ReceiveReceiving, &[Arg::Int(i64::from(percent))]),
                primary,
            ),
            Status::Verifying => (t.get(StringId::ReceiveVerifying).into_owned(), primary),
            Status::Refused(reason) => {
                let id = match reason {
                    Reject::TooBig => StringId::CompanionErrBig,
                    Reject::Unreadable => StringId::CompanionErrRead,
                };
                (t.get(id).into_owned(), vars::DANGER_FG)
            }
            status => (self.receive_status_text_closed(status), vars::DANGER_FG),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_layout_is_on_the_ruler() {
        use super::*;
        super::super::assert_on_ruler(&[
            ("TOP", TOP),
            ("QR_SIDE", QR_SIDE),
            ("PANEL_X", PANEL_X),
            ("PANEL_RIGHT", PANEL_RIGHT),
            ("PANEL_PAD_X", PANEL_PAD_X),
            ("PANEL_PAD_Y", PANEL_PAD_Y),
            ("STEP_PITCH", STEP_PITCH),
            ("BUTTON_HEIGHT", BUTTON_HEIGHT),
            ("BUTTON_GAP", BUTTON_GAP),
        ]);
        assert_eq!(
            (LEFT, TOP, LEFT + QR_SIDE, TOP + QR_SIDE),
            (64.0, 152.0, 384.0, 472.0)
        );
    }
}
