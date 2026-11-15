//! The Audio tool (`audio-diagnostics`): a tone per channel, the frequency sweep and the
//! audio/video sync test (tasks 5.x).
//!
//! The sound itself is made on the platform side (D2): this screen only sends an
//! [`AudioCommand`] when something starts or stops, and the platform opens the audio stream for
//! it and closes it on [`AudioCommand::Stop`]. The samples come from [`synth`], which the
//! platform's audio callback calls.

#[path = "synth.rs"]
pub mod synth;

use super::common::{self, draw_panel, panel_bar, panel_heading, panel_row, wrapped_text, Hint};
use super::{AudioCommand, Channel, ToolEvent, ToolKey, ToolKind};
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp};
use crate::i18n::{Arg, StringId};
use crate::render::{PatternOp, PixelRect, Rect};
use crate::text::Align;
use crate::theme::{Color, StateFlags, THEME};
use crate::ui::focus::FocusId;

/// The panel at the right: the focused item's description and, during the sweep, its frequency.
const PANEL_HEIGHT: f32 = 280.0;
/// Where the description starts under the panel's heading.
const TEXT_TOP: f32 = 48.0;
/// The frequency row of the sweep and its progress bar under it, from the panel's inner top.
const FREQUENCY_TOP: f32 = 176.0;
const BAR_GAP: f32 = 36.0;
const BAR_HEIGHT: f32 = 8.0;
/// The side of the white square of the sync test (1% of a 16:9 screen, under the 5% of the spec).
const SQUARE: f32 = 96.0;

/// How long the white square shows (as long as the beep).
const SQUARE_MS: u64 = synth::BEEP_MS as u64;
/// The sync test repeats once a second.
const SYNC_PERIOD_MS: u32 = 1000;
/// The sync offset: steps of 10 ms within ±300 ms.
const OFFSET_STEP_MS: i32 = 10;
const OFFSET_MAX_MS: i32 = 300;
/// How often the frequency of the sweep is redrawn.
const SWEEP_FRAME_MS: u64 = 250;

/// The items of the screen, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Left,
    Right,
    Both,
    Sweep,
    Sync,
}

impl Item {
    const ALL: [Item; 5] = [Item::Left, Item::Right, Item::Both, Item::Sweep, Item::Sync];

    fn id(self) -> FocusId {
        ToolKind::Audio.first_id() + Item::ALL.iter().position(|i| *i == self).unwrap_or(0) as u32
    }

    fn from_id(id: FocusId) -> Option<Item> {
        Item::ALL.into_iter().find(|i| i.id() == id)
    }

    fn name(self) -> StringId {
        match self {
            Item::Left => StringId::ToolAudioLeft,
            Item::Right => StringId::ToolAudioRight,
            Item::Both => StringId::ToolAudioBoth,
            Item::Sweep => StringId::ToolAudioSweep,
            Item::Sync => StringId::ToolAudioSync,
        }
    }

    fn description(self) -> StringId {
        match self {
            Item::Left => StringId::ToolAudioLeftDesc,
            Item::Right => StringId::ToolAudioRightDesc,
            Item::Both => StringId::ToolAudioBothDesc,
            Item::Sweep => StringId::ToolAudioSweepDesc,
            Item::Sync => StringId::ToolAudioSyncDesc,
        }
    }

    fn channel(self) -> Option<Channel> {
        match self {
            Item::Left => Some(Channel::Left),
            Item::Right => Some(Channel::Right),
            Item::Both => Some(Channel::Both),
            _ => None,
        }
    }
}

/// What plays now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Playing {
    Tone(Channel),
    Sweep { started_ms: u64 },
}

/// The sync test, on screen while it runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SyncTest {
    /// How much later than the beep the square shows (negative: earlier). Only for this run.
    offset_ms: i32,
    /// When the last beep left the speakers (D8), once the platform told.
    beep_at: Option<u64>,
    hint: Hint,
}

/// The tool's state, alive only while it is open.
#[derive(Debug, Default)]
pub struct AudioTool {
    playing: Option<Playing>,
    sync: Option<SyncTest>,
    /// The item whose sound could not start ("Áudio indisponível"), until another one starts.
    unavailable: Option<Item>,
}

impl AudioTool {
    pub(in crate::app) fn new(_now_ms: u64) -> AudioTool {
        AudioTool::default()
    }

    /// The item that is sounding, if any.
    fn sounding(&self) -> Option<Item> {
        if self.sync.is_some() {
            return Some(Item::Sync);
        }
        match self.playing? {
            Playing::Tone(Channel::Left) => Some(Item::Left),
            Playing::Tone(Channel::Right) => Some(Item::Right),
            Playing::Tone(Channel::Both) => Some(Item::Both),
            Playing::Sweep { .. } => Some(Item::Sweep),
        }
    }

    /// The sweep ends by itself after 30 s (the platform closes the stream when the synthesizer
    /// fell silent).
    fn settle(&mut self, now_ms: u64) {
        if let Some(Playing::Sweep { started_ms }) = self.playing {
            if now_ms >= started_ms + synth::SWEEP_MS as u64 {
                self.playing = None;
            }
        }
    }
}

/// The text of the sync offset and what it means.
fn sync_text(t: &crate::i18n::Translator, offset_ms: i32) -> String {
    let ms = Arg::Int(offset_ms.unsigned_abs() as i64);
    match offset_ms {
        0 => t.get(StringId::ToolAudioSyncZero).into_owned(),
        o if o > 0 => t.format(StringId::ToolAudioSyncLate, &[ms]),
        _ => t.format(StringId::ToolAudioSyncEarly, &[ms]),
    }
}

/// Whether the square shows at `now_ms`: for 50 ms, `offset_ms` after each beep (D8). The beeps
/// are a second apart, so one beep's time places them all, also the next one (for a negative
/// offset the square comes before it).
fn square_visible(beep_at: u64, offset_ms: i32, now_ms: u64) -> bool {
    let since = now_ms as i64 - beep_at as i64 - offset_ms as i64;
    (since.rem_euclid(SYNC_PERIOD_MS as i64) as u64) < SQUARE_MS
}

impl LumeApp {
    fn audio_state(&self) -> Option<&AudioTool> {
        match self.tool.as_deref() {
            Some(super::ToolState::Audio(s)) => Some(s),
            _ => None,
        }
    }

    fn audio_state_mut(&mut self) -> Option<&mut AudioTool> {
        match self.tool.as_deref_mut() {
            Some(super::ToolState::Audio(s)) => Some(s),
            _ => None,
        }
    }

    /// Opening the tool: nothing plays, so no stream is opened.
    pub(in crate::app) fn audio_open(&mut self) -> Vec<Effect> {
        vec![]
    }

    pub(in crate::app) fn draw_audio(&mut self, p: &mut Painter) {
        let now = self.now_ms;
        let t = self.translator;
        let Some(state) = self.audio_state_mut() else {
            return;
        };
        state.settle(now);
        if let Some(sync) = &state.sync {
            // Black, and the white square in the middle while it is due, in device pixels.
            p.list.pattern(PatternOp::Fill(Color::rgb(0, 0, 0)));
            if sync
                .beep_at
                .is_some_and(|at| square_visible(at, sync.offset_ms, now))
            {
                let side = (SQUARE * p.display.scale).round() as i32;
                let (w, h) = (p.display.width as i32, p.display.height as i32);
                p.list.pattern(PatternOp::Rect(
                    PixelRect::new((w - side) / 2, (h - side) / 2, side, side),
                    Color::rgb(255, 255, 255),
                ));
            }
            sync.hint.draw(p, now);
            return;
        }
        let sounding = state.sounding();
        let unavailable = state.unavailable;
        let sweep_ms = match state.playing {
            Some(Playing::Sweep { started_ms }) => Some(now.saturating_sub(started_ms)),
            _ => None,
        };

        let items: Vec<(FocusId, String)> = Item::ALL
            .iter()
            .map(|i| (i.id(), t.get(i.name()).into_owned()))
            .collect();
        let rects = self.draw_tool_items(p, &items, 0.0);

        // What each item is doing, at its right: playing, the sweep's frequency, or the failure.
        for (item, rect) in Item::ALL.iter().zip(&rects) {
            let status = if unavailable == Some(*item) {
                t.get(StringId::ToolAudioUnavailable).into_owned()
            } else if sounding == Some(*item) {
                match sweep_ms {
                    Some(ms) => hz_text(&t, ms),
                    None => t.get(StringId::ToolAudioPlaying).into_owned(),
                }
            } else {
                continue;
            };
            let style = p.focus_style(&THEME.settings_item, self.focus_progress(item.id()));
            let inner = Rect::new(
                rect.x + style.padding[3],
                rect.y,
                rect.w - style.padding[3] - style.padding[1],
                rect.h,
            );
            p.text(&status, inner, Align::End, &style);
        }

        // The focused item's description and, during the sweep, the frequency and the progress.
        let focused = self.focused().and_then(Item::from_id).unwrap_or(Item::Left);
        let inner = draw_panel(p, common::panel_rect(PANEL_HEIGHT));
        panel_heading(p, inner, &t.get(focused.name()));
        let body = p.style(&THEME.breadcrumb, StateFlags::NONE);
        let text = Rect::new(inner.x, inner.y + TEXT_TOP, inner.w, body.line_height);
        wrapped_text(
            p,
            &t.get(focused.description()),
            text,
            body.line_height,
            &body,
        );
        if let Some(ms) = sweep_ms {
            let line = Rect::new(
                inner.x,
                inner.y + FREQUENCY_TOP,
                inner.w,
                common::PANEL_ROW_HEIGHT,
            );
            panel_row(
                p,
                line,
                &t.get(StringId::ToolAudioFrequency),
                &hz_text(&t, ms),
            );
            let bar = Rect::new(inner.x, line.y + BAR_GAP, inner.w, BAR_HEIGHT);
            panel_bar(p, bar, Some(ms as f32 / synth::SWEEP_MS as f32));
        }

        self.draw_tool_header(p, ToolKind::Audio);
    }

    pub(in crate::app) fn audio_key(&mut self, key: Key) -> ToolKey {
        let now = self.now_ms;
        let t = self.translator;
        let focused = self.focused().and_then(Item::from_id);
        let Some(state) = self.audio_state_mut() else {
            return ToolKey::Default;
        };
        state.settle(now);

        // The sync test takes every key: the arrows move the square, Back ends it.
        if let Some(sync) = state.sync.as_mut() {
            match key {
                Key::Left | Key::Right => {
                    let step = if key == Key::Left {
                        -OFFSET_STEP_MS
                    } else {
                        OFFSET_STEP_MS
                    };
                    sync.offset_ms = (sync.offset_ms + step).clamp(-OFFSET_MAX_MS, OFFSET_MAX_MS);
                    sync.hint.show(sync_text(&t, sync.offset_ms), now);
                }
                Key::Back => {
                    // The offset goes with the test: the next run starts at 0 ms.
                    state.sync = None;
                    state.playing = None;
                    return ToolKey::Handled(vec![Effect::Audio(AudioCommand::Stop)]);
                }
                _ => sync.hint.show(sync_text(&t, sync.offset_ms), now),
            }
            return ToolKey::Handled(vec![]);
        }

        if key != Key::Ok {
            return ToolKey::Default;
        }
        let Some(item) = focused else {
            return ToolKey::Default;
        };
        // OK on what plays stops it; on anything else, starts it (stopping the rest).
        if state.sounding() == Some(item) {
            state.playing = None;
            return ToolKey::Handled(vec![Effect::Audio(AudioCommand::Stop)]);
        }
        state.unavailable = None;
        let command = match item {
            Item::Sweep => {
                state.playing = Some(Playing::Sweep { started_ms: now });
                AudioCommand::Sweep
            }
            Item::Sync => {
                state.playing = None;
                let mut sync = SyncTest::default();
                sync.hint.show(sync_text(&t, 0), now);
                state.sync = Some(sync);
                AudioCommand::SyncBeep {
                    period_ms: SYNC_PERIOD_MS,
                }
            }
            _ => {
                let channel = item.channel().unwrap_or(Channel::Both);
                state.playing = Some(Playing::Tone(channel));
                AudioCommand::Tone {
                    channel,
                    hz: synth::TONE_HZ,
                }
            }
        };
        ToolKey::Handled(vec![Effect::Audio(command)])
    }

    /// Continuous frames only while the sync square may show; four a second for the sweep's
    /// frequency, and one when it ends.
    pub(in crate::app) fn audio_deadline(&self, _now_ms: u64) -> Option<u64> {
        let state = self.audio_state()?;
        if state.sync.is_some() {
            return Some(self.now_ms + crate::ui::animation::FRAME_MS);
        }
        match state.playing {
            Some(Playing::Sweep { started_ms }) => {
                let elapsed = self.now_ms.saturating_sub(started_ms);
                let next = (elapsed / SWEEP_FRAME_MS + 1) * SWEEP_FRAME_MS;
                Some(started_ms + next.min(synth::SWEEP_MS as u64))
            }
            _ => None,
        }
    }

    /// The sync test is a full-screen pattern: a square over black, without wallpaper.
    pub(in crate::app) fn audio_fullscreen(&self) -> bool {
        self.audio_state().is_some_and(|s| s.sync.is_some())
    }

    pub(in crate::app) fn audio_event(&mut self, event: ToolEvent) {
        let Some(state) = self.audio_state_mut() else {
            return;
        };
        match event {
            ToolEvent::AudioStarted { at_ms } => {
                if let Some(sync) = state.sync.as_mut() {
                    sync.beep_at = Some(at_ms);
                }
            }
            ToolEvent::AudioUnavailable => {
                // The item that tried to play says so; the interface goes on.
                state.unavailable = state.sounding();
                state.playing = None;
                state.sync = None;
            }
            _ => {}
        }
    }
}

/// A frequency of the sweep, `elapsed_ms` after it started, in whole hertz.
fn hz_text(t: &crate::i18n::Translator, elapsed_ms: u64) -> String {
    let hz = synth::sweep_hz(elapsed_ms as f64).round() as i64;
    t.format(StringId::ToolAudioHz, &[Arg::Int(hz)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};

    fn open() -> Rig {
        let mut rig = Rig::new();
        let effects = open_tool(&mut rig, ToolKind::Audio);
        assert!(
            !effects.iter().any(|e| matches!(e, Effect::Audio(_))),
            "no stream while nothing plays"
        );
        rig
    }

    fn sync_offset(rig: &Rig) -> Option<i32> {
        rig.app
            .audio_state()
            .and_then(|s| s.sync.as_ref())
            .map(|s| s.offset_ms)
    }

    fn start_sync(rig: &mut Rig) {
        rig.press_all(&[Key::Down, Key::Down, Key::Down, Key::Down]);
        assert_eq!(rig.focused(), Some(Item::Sync.id()));
        assert_eq!(
            rig.press(Key::Ok),
            vec![Effect::Audio(AudioCommand::SyncBeep { period_ms: 1000 })]
        );
        assert!(rig.app.audio_fullscreen());
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("PANEL_HEIGHT", PANEL_HEIGHT),
            ("TEXT_TOP", TEXT_TOP),
            ("FREQUENCY_TOP", FREQUENCY_TOP),
            ("BAR_GAP", BAR_GAP),
            ("BAR_HEIGHT", BAR_HEIGHT),
            ("SQUARE", SQUARE),
        ]);
        // The square stays under 5% of the screen.
        const { assert!(SQUARE * SQUARE / (1280.0 * 720.0) < 0.05) };
    }

    #[test]
    fn ok_toggles_a_tone_and_another_tone_replaces_it() {
        let mut rig = open();
        assert_eq!(rig.focused(), Some(Item::Left.id()));
        let left = AudioCommand::Tone {
            channel: Channel::Left,
            hz: 440,
        };
        assert_eq!(rig.press(Key::Ok), vec![Effect::Audio(left)]);
        rig.press(Key::Down);
        let right = AudioCommand::Tone {
            channel: Channel::Right,
            hz: 440,
        };
        assert_eq!(rig.press(Key::Ok), vec![Effect::Audio(right)]);
        assert_eq!(rig.press(Key::Ok), vec![Effect::Audio(AudioCommand::Stop)]);
        assert_eq!(rig.app.audio_state().unwrap().playing, None);
        assert_eq!(rig.app.audio_deadline(rig.now), None, "no frames at rest");
    }

    #[test]
    fn the_sweep_shows_its_frequency_and_ends_by_itself() {
        let mut rig = open();
        rig.press_all(&[Key::Down, Key::Down, Key::Down]);
        assert_eq!(rig.press(Key::Ok), vec![Effect::Audio(AudioCommand::Sweep)]);
        let started = rig.now - 100;
        assert_eq!(rig.app.audio_deadline(rig.now), Some(started + 250));
        rig.advance(15_000 - 100);
        // Halfway it plays (and shows) the geometric middle of 20 Hz and 20 kHz.
        assert!(rig.app.audio_state().unwrap().playing.is_some());
        assert_eq!(hz_text(&rig.app.translator, rig.now - started), "632 Hz");
        rig.advance(15_000);
        assert_eq!(rig.app.audio_state().unwrap().playing, None);
        assert_eq!(rig.app.audio_deadline(rig.now), None);
    }

    #[test]
    fn the_sync_offset_moves_in_10_ms_steps_within_300_ms() {
        let mut rig = open();
        start_sync(&mut rig);
        assert_eq!(sync_offset(&rig), Some(0));
        for _ in 0..8 {
            rig.press(Key::Right);
        }
        assert_eq!(sync_offset(&rig), Some(80));
        assert_eq!(
            sync_text(&rig.app.translator, 80),
            "+80 ms: o som está 80 ms atrasado"
        );
        for _ in 0..50 {
            rig.press(Key::Left);
        }
        assert_eq!(sync_offset(&rig), Some(-300));
        assert_eq!(
            sync_text(&rig.app.translator, -300),
            "−300 ms: o som está 300 ms adiantado"
        );
    }

    #[test]
    fn the_offset_is_dropped_when_the_sync_test_ends() {
        let mut rig = open();
        start_sync(&mut rig);
        rig.press_all(&[Key::Right, Key::Right, Key::Right]);
        assert_eq!(sync_offset(&rig), Some(30));
        assert_eq!(
            rig.press(Key::Back),
            vec![Effect::Audio(AudioCommand::Stop)]
        );
        assert!(!rig.app.audio_fullscreen());
        assert_eq!(rig.focused(), Some(Item::Sync.id()));
        rig.press(Key::Ok);
        assert_eq!(sync_offset(&rig), Some(0));

        // Leaving the tool drops its state, offset and all, and stops the sound.
        let effects = rig.press(Key::Back);
        assert!(effects.contains(&Effect::Audio(AudioCommand::Stop)));
        let effects = rig.press(Key::Back);
        assert!(effects.contains(&Effect::Audio(AudioCommand::Stop)));
        assert_eq!(rig.app.tool_kind(), None);
    }

    #[test]
    fn the_square_follows_the_beep_by_the_offset() {
        let mut rig = open();
        start_sync(&mut rig);
        let square = |rig: &mut Rig| {
            rig.frame()
                .pattern_ops()
                .iter()
                .any(|op| matches!(op, PatternOp::Rect(..)))
        };
        assert!(!square(&mut rig), "no square before the first beep");
        assert_eq!(
            rig.app.audio_deadline(rig.now),
            Some(rig.now + crate::ui::animation::FRAME_MS),
            "continuous frames"
        );
        let beep = rig.now + 20;
        rig.app
            .tool_event(ToolEvent::AudioStarted { at_ms: beep }, rig.now);
        rig.now = beep + 10;
        assert!(square(&mut rig));
        rig.now = beep + 60;
        assert!(!square(&mut rig));

        rig.press_all(&[Key::Right; 8]);
        rig.now = beep + 1000 + 10;
        assert!(!square(&mut rig), "80 ms later now");
        rig.now = beep + 1000 + 90;
        assert!(square(&mut rig));
        assert!(square_visible(1000, -100, 1900 + 10), "before the beep");
    }

    #[test]
    fn a_stream_that_fails_says_so_on_its_item() {
        let mut rig = open();
        rig.press(Key::Ok);
        rig.app.tool_event(ToolEvent::AudioUnavailable, rig.now);
        let state = rig.app.audio_state().unwrap();
        assert_eq!(state.playing, None);
        assert_eq!(state.unavailable, Some(Item::Left));
        assert_eq!(
            rig.app.translator.get(StringId::ToolAudioUnavailable),
            "Áudio indisponível"
        );
        // The interface goes on: the focus moves.
        rig.press(Key::Down);
        assert_eq!(rig.focused(), Some(Item::Right.id()));
    }
}
