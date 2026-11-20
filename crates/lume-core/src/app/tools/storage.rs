//! The Storage tool (`storage-diagnostics`): sequential write and read speed (task 7.1, D9).
//!
//! The test itself runs on the platform's `lume-test` thread (it writes [`TEST_MB`] MB in 1 MB
//! blocks, syncs, drops the file from the page cache and reads it back); this side only starts or
//! cancels it and shows what it reports. The platform deletes the temporary file when the test
//! ends or is cancelled, and when the tool opens (a run that was killed may have left it).

use super::common::{
    self, draw_panel, panel_bar, panel_heading, panel_row, wrapped_text, PANEL_ROW_PITCH,
};
use super::{TestKind, TestUpdate, ToolEvent, ToolKey, ToolKind, ToolReading};
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp};
use crate::i18n::{Arg, StringId};
use crate::render::Rect;
use crate::stats;
use crate::theme::StateFlags;

/// The size of the test file, written and read in 1 MB blocks.
pub const TEST_MB: u32 = 64;
/// The size of a block: the only memory the test uses for data.
pub const BLOCK_BYTES: usize = 1024 * 1024;
/// Below this much free space the test does not start.
pub const MIN_FREE_BYTES: u64 = 256 * 1024 * 1024;

/// The free and total bytes of the data partition (where the test file goes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

impl StorageSpace {
    /// Whether there is room for the test.
    pub fn enough(&self) -> bool {
        self.free_bytes >= MIN_FREE_BYTES
    }
}

/// Progress or result of the storage test, sent by the platform's `lume-test` thread.
#[derive(Clone, Debug, PartialEq)]
pub enum StorageUpdate {
    /// The space before the test starts (the first update of every run).
    Space(StorageSpace),
    /// Less than [`MIN_FREE_BYTES`] free: the test did not start.
    NoSpace(StorageSpace),
    /// `mb` of [`TEST_MB`] written (the last one counts once the data is on the storage).
    Writing { mb: u32 },
    /// `mb` of [`TEST_MB`] read back; the write speed is known.
    Reading { mb: u32, write_mb_s: f32 },
    /// The test ended, and its file is gone.
    Done { write_mb_s: f32, read_mb_s: f32 },
    /// Writing or reading failed (the file is deleted all the same).
    Failed,
}

/// Where the test is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Not run yet.
    Idle,
    /// Not started for lack of space.
    Refused,
    Writing {
        mb: u32,
    },
    Reading {
        mb: u32,
        write_mb_s: f32,
    },
    Done {
        write_mb_s: f32,
        read_mb_s: f32,
    },
    /// Stopped by Back or by OK on "Cancel".
    Cancelled,
    Failed,
}

impl Phase {
    pub fn running(self) -> bool {
        matches!(self, Phase::Writing { .. } | Phase::Reading { .. })
    }
}

/// The tool's state, alive only while it is open.
#[derive(Debug)]
pub struct StorageTool {
    /// `None` until the platform tells it.
    pub space: Option<StorageSpace>,
    /// Whether the space was read since the tool opened (the reading may fail: it is not
    /// repeated then).
    space_read: bool,
    pub phase: Phase,
}

impl StorageTool {
    pub(in crate::app) fn new(_now_ms: u64) -> StorageTool {
        StorageTool {
            space: None,
            space_read: false,
            phase: Phase::Idle,
        }
    }

    /// Takes an update of the test. Progress that comes after the test was cancelled is dropped.
    fn update(&mut self, update: StorageUpdate) {
        match update {
            StorageUpdate::Space(space) => self.space = Some(space),
            StorageUpdate::NoSpace(space) => {
                self.space = Some(space);
                self.phase = Phase::Refused;
            }
            _ if !self.phase.running() => {}
            StorageUpdate::Writing { mb } => self.phase = Phase::Writing { mb },
            StorageUpdate::Reading { mb, write_mb_s } => {
                self.phase = Phase::Reading { mb, write_mb_s }
            }
            StorageUpdate::Done {
                write_mb_s,
                read_mb_s,
            } => {
                self.phase = Phase::Done {
                    write_mb_s,
                    read_mb_s,
                }
            }
            StorageUpdate::Failed => self.phase = Phase::Failed,
        }
    }
}

/// The only item: "Test", or "Cancel test" while it runs.
fn item_id() -> crate::ui::focus::FocusId {
    ToolKind::Storage.first_id()
}

/// The panel: heading, free and total space with the used bar, write and read speeds with the
/// progress bar, and up to two lines of status. The rows start 72 below the top, as the
/// statistics panel, 40 apart; the speeds start a pitch lower, below the used bar.
const ROWS_TOP: f32 = 72.0;
const SPEEDS_TOP: f32 = 184.0;
/// The bar under a row, as the memory bar of the statistics panel.
const BAR_OFFSET: f32 = 36.0;
const BAR_HEIGHT: f32 = 8.0;
/// The status text, under the progress bar.
const STATUS_TOP: f32 = 292.0;
const STATUS_PITCH: f32 = 28.0;
const PANEL_HEIGHT: f32 = 376.0;

/// A speed such as `42,3 MB/s`.
fn format_speed(mb_s: f32, comma: bool) -> String {
    let text = format!("{mb_s:.1} MB/s");
    if comma {
        text.replace('.', ",")
    } else {
        text
    }
}

impl LumeApp {
    /// Opening the tool (its state was just created): the platform deletes a test file that a
    /// killed run may have left behind (cancelling a test that does not run only does that).
    pub(in crate::app) fn storage_open(&mut self) -> Vec<Effect> {
        vec![Effect::CancelTest]
    }

    /// The space of the data partition, from a reading (see [`LumeApp::storage_reading_deadline`]).
    pub(in crate::app) fn storage_space(&mut self, space: Option<StorageSpace>) {
        if let Some(state) = self.tool_mut::<StorageTool>() {
            state.space_read = true;
            state.space = space.or(state.space);
        }
    }

    /// The tool wants the space read once when it opens (and not while the test writes).
    pub(in crate::app) fn storage_reading_deadline(&self, now_ms: u64) -> Option<u64> {
        self.tool_ref::<StorageTool>()
            .filter(|state| !state.space_read && !state.phase.running())
            .map(|_| now_ms)
    }

    pub(in crate::app) fn draw_storage(&mut self, p: &mut Painter) {
        let t = self.translator;
        let running = self
            .tool_ref::<StorageTool>()
            .is_some_and(|s| s.phase.running());
        let label = if running {
            StringId::ToolStorageCancel
        } else {
            StringId::ToolStorageTest
        };
        self.draw_tool_items(p, &[(item_id(), t.get(label).into_owned())], 0.0);
        self.draw_tool_header(p, ToolKind::Storage);
        let Some(state) = self.tool_ref::<StorageTool>() else {
            return;
        };
        let comma = !t.language().starts_with("en");
        let dash = || "—".to_string();

        let panel = common::panel_rect(PANEL_HEIGHT);
        let inner = draw_panel(p, panel);
        panel_heading(p, inner, &t.get(StringId::ToolStorageHeading));
        let row = |top: f32, index: usize| {
            Rect::new(
                inner.x,
                panel.y + top + index as f32 * PANEL_ROW_PITCH,
                inner.w,
                common::PANEL_ROW_HEIGHT,
            )
        };
        let bar = |line: Rect| Rect::new(line.x, line.y + BAR_OFFSET, line.w, BAR_HEIGHT);

        let space = state.space;
        let free = space.map_or_else(dash, |s| stats::format_size(s.free_bytes, comma));
        let total = space.map_or_else(dash, |s| stats::format_size(s.total_bytes, comma));
        panel_row(
            p,
            row(ROWS_TOP, 0),
            &t.get(StringId::ToolStorageFree),
            &free,
        );
        panel_row(
            p,
            row(ROWS_TOP, 1),
            &t.get(StringId::ToolStorageTotal),
            &total,
        );
        let used = space
            .filter(|s| s.total_bytes > 0)
            .map(|s| 1.0 - s.free_bytes as f32 / s.total_bytes as f32);
        panel_bar(p, bar(row(ROWS_TOP, 1)), used);

        let (write, read) = match state.phase {
            Phase::Reading { write_mb_s, .. } => (Some(write_mb_s), None),
            Phase::Done {
                write_mb_s,
                read_mb_s,
            } => (Some(write_mb_s), Some(read_mb_s)),
            _ => (None, None),
        };
        let speed = |v: Option<f32>| v.map_or_else(dash, |v| format_speed(v, comma));
        panel_row(
            p,
            row(SPEEDS_TOP, 0),
            &t.get(StringId::ToolStorageWrite),
            &speed(write),
        );
        panel_row(
            p,
            row(SPEEDS_TOP, 1),
            &t.get(StringId::ToolStorageRead),
            &speed(read),
        );

        // Progress: the write is the first half of the bar, the read the second.
        let progress = match state.phase {
            Phase::Writing { mb } => Some(mb as f32 / (2 * TEST_MB) as f32),
            Phase::Reading { mb, .. } => Some((TEST_MB + mb) as f32 / (2 * TEST_MB) as f32),
            Phase::Done { .. } => Some(1.0),
            _ => None,
        };
        panel_bar(p, bar(row(SPEEDS_TOP, 1)), progress);

        let total_mb = Arg::Int(TEST_MB as i64);
        let status = match state.phase {
            Phase::Idle => t.format(StringId::ToolStorageNote, &[total_mb]),
            Phase::Refused => t.format(
                StringId::ToolStorageNoSpace,
                &[Arg::Int((MIN_FREE_BYTES / (1024 * 1024)) as i64)],
            ),
            Phase::Writing { mb } => t.format(
                StringId::ToolStorageWriting,
                &[Arg::Int(mb as i64), total_mb],
            ),
            Phase::Reading { mb, .. } => t.format(
                StringId::ToolStorageReading,
                &[Arg::Int(mb as i64), total_mb],
            ),
            Phase::Done { .. } => t.get(StringId::ToolStorageDone).into_owned(),
            Phase::Cancelled => t.get(StringId::ToolStorageCancelled).into_owned(),
            Phase::Failed => t.get(StringId::ToolStorageFailed).into_owned(),
        };
        let style = p.style(|t| &t.breadcrumb, StateFlags::NONE);
        let area = Rect::new(inner.x, panel.y + STATUS_TOP, inner.w, 2.0 * STATUS_PITCH);
        wrapped_text(p, &status, area, STATUS_PITCH, &style);
    }

    pub(in crate::app) fn storage_key(&mut self, key: Key) -> ToolKey {
        let focused = self.focus.current_id();
        let Some(state) = self.tool_mut::<StorageTool>() else {
            return ToolKey::Default;
        };
        match key {
            // Back stops a running test and stays; otherwise it leaves (the usual handling).
            Key::Back | Key::Ok if state.phase.running() => {
                if key == Key::Ok && focused != Some(item_id()) {
                    return ToolKey::Default;
                }
                state.phase = Phase::Cancelled;
                ToolKey::Handled(vec![Effect::CancelTest])
            }
            Key::Ok if focused == Some(item_id()) => {
                if state.space.is_some_and(|s| !s.enough()) {
                    state.phase = Phase::Refused;
                    return ToolKey::Handled(vec![]);
                }
                state.phase = Phase::Writing { mb: 0 };
                ToolKey::Handled(vec![Effect::StartTest(TestKind::Storage)])
            }
            _ => ToolKey::Default,
        }
    }

    pub(in crate::app) fn storage_event(&mut self, event: ToolEvent) {
        match event {
            ToolEvent::Reading(ToolReading::Storage(space)) => self.storage_space(space),
            ToolEvent::Test(TestUpdate::Storage(update)) => {
                if let Some(state) = self.tool_mut::<StorageTool>() {
                    state.update(update);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};
    use crate::app::Screen;

    const GB: u64 = 1024 * 1024 * 1024;

    fn opened() -> Rig {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Storage);
        rig
    }

    fn phase(rig: &Rig) -> Phase {
        rig.app
            .tool_ref::<StorageTool>()
            .expect("the storage tool is not open")
            .phase
    }

    fn send(rig: &mut Rig, update: StorageUpdate) {
        let now = rig.now;
        rig.app
            .tool_event(ToolEvent::Test(TestUpdate::Storage(update)), now);
        rig.advance(100);
    }

    fn space(free_gb: u64) -> StorageSpace {
        StorageSpace {
            free_bytes: free_gb * GB,
            total_bytes: 8 * GB,
        }
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("ROWS_TOP", ROWS_TOP),
            ("SPEEDS_TOP", SPEEDS_TOP),
            ("BAR_OFFSET", BAR_OFFSET),
            ("BAR_HEIGHT", BAR_HEIGHT),
            ("STATUS_TOP", STATUS_TOP),
            ("STATUS_PITCH", STATUS_PITCH),
            ("PANEL_HEIGHT", PANEL_HEIGHT),
        ]);
        // Each bar is a whole pitch above the next row, and two status lines fit the panel.
        let used_bar = ROWS_TOP + PANEL_ROW_PITCH + BAR_OFFSET + BAR_HEIGHT;
        assert!(used_bar + 24.0 <= SPEEDS_TOP, "{used_bar}");
        let progress_bar = SPEEDS_TOP + PANEL_ROW_PITCH + BAR_OFFSET + BAR_HEIGHT;
        assert!(progress_bar + 24.0 <= STATUS_TOP, "{progress_bar}");
        const { assert!(STATUS_TOP + 2.0 * STATUS_PITCH + common::PANEL_PAD_TOP <= PANEL_HEIGHT) };
        const { assert!(common::ITEMS_TOP + PANEL_HEIGHT <= 720.0) };
    }

    #[test]
    fn opening_asks_to_delete_a_file_left_behind_and_starts_nothing() {
        let mut rig = Rig::new();
        let effects = open_tool(&mut rig, ToolKind::Storage);
        assert!(effects.contains(&Effect::CancelTest), "{effects:?}");
        assert!(!effects.contains(&Effect::StartTest(TestKind::Storage)));
        assert_eq!(phase(&rig), Phase::Idle);
        assert_eq!(rig.focused(), Some(item_id()));
    }

    #[test]
    fn the_space_is_read_once_when_the_tool_opens() {
        let mut rig = opened();
        assert_eq!(rig.app.storage_reading_deadline(rig.now), Some(rig.now));
        rig.app.storage_space(None);
        assert_eq!(
            rig.app.storage_reading_deadline(rig.now),
            None,
            "asked once, even when it fails"
        );
        rig.app.storage_space(Some(space(4)));
        assert_eq!(rig.app.storage_reading_deadline(rig.now), None);
    }

    #[test]
    fn a_full_run_reports_both_speeds() {
        let mut rig = opened();
        let effects = rig.press(Key::Ok);
        assert_eq!(effects, vec![Effect::StartTest(TestKind::Storage)]);
        assert!(phase(&rig).running());
        send(&mut rig, StorageUpdate::Space(space(4)));
        send(&mut rig, StorageUpdate::Writing { mb: 32 });
        assert_eq!(phase(&rig), Phase::Writing { mb: 32 });
        send(
            &mut rig,
            StorageUpdate::Reading {
                mb: 10,
                write_mb_s: 20.5,
            },
        );
        assert!(matches!(phase(&rig), Phase::Reading { mb: 10, .. }));
        send(
            &mut rig,
            StorageUpdate::Done {
                write_mb_s: 20.5,
                read_mb_s: 80.0,
            },
        );
        assert_eq!(
            phase(&rig),
            Phase::Done {
                write_mb_s: 20.5,
                read_mb_s: 80.0
            }
        );
        assert_eq!(format_speed(20.5, true), "20,5 MB/s");
        // Once done, Back leaves as usual.
        rig.press(Key::Back);
        assert_eq!(rig.app.screen(), &Screen::Tools);
    }

    #[test]
    fn the_test_is_refused_with_little_free_space() {
        // Known before OK: the test does not start.
        let mut rig = opened();
        rig.app.storage_space(Some(StorageSpace {
            free_bytes: 100 * 1024 * 1024,
            total_bytes: 8 * GB,
        }));
        let effects = rig.press(Key::Ok);
        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(phase(&rig), Phase::Refused);

        // Found by the test itself.
        let mut rig = opened();
        rig.press(Key::Ok);
        send(
            &mut rig,
            StorageUpdate::NoSpace(StorageSpace {
                free_bytes: 200 * 1024 * 1024,
                total_bytes: 8 * GB,
            }),
        );
        assert_eq!(phase(&rig), Phase::Refused);
        assert!(!space(0).enough() && space(1).enough());
    }

    #[test]
    fn back_while_running_cancels_and_stays() {
        let mut rig = opened();
        rig.press(Key::Ok);
        send(&mut rig, StorageUpdate::Writing { mb: 5 });
        let effects = rig.press(Key::Back);
        assert_eq!(effects, vec![Effect::CancelTest]);
        assert_eq!(phase(&rig), Phase::Cancelled);
        assert_eq!(rig.app.screen(), &Screen::Tool(ToolKind::Storage));
        // Progress already on its way is dropped.
        send(&mut rig, StorageUpdate::Writing { mb: 6 });
        assert_eq!(phase(&rig), Phase::Cancelled);
        // A second Back leaves, and the release cancels again (harmless).
        let effects = rig.press(Key::Back);
        assert_eq!(rig.app.screen(), &Screen::Tools);
        assert!(effects.contains(&Effect::CancelTest));
    }

    #[test]
    fn ok_on_the_item_while_running_cancels() {
        let mut rig = opened();
        rig.press(Key::Ok);
        let effects = rig.press(Key::Ok);
        assert_eq!(effects, vec![Effect::CancelTest]);
        assert_eq!(phase(&rig), Phase::Cancelled);
        // And it can run again.
        let effects = rig.press(Key::Ok);
        assert_eq!(effects, vec![Effect::StartTest(TestKind::Storage)]);
    }

    #[test]
    fn a_failure_is_shown() {
        let mut rig = opened();
        rig.press(Key::Ok);
        send(&mut rig, StorageUpdate::Failed);
        assert_eq!(phase(&rig), Phase::Failed);
    }

    #[test]
    fn leaving_with_home_cancels_the_test() {
        let mut rig = opened();
        rig.press(Key::Ok);
        let effects = rig.press(Key::Home);
        assert!(effects.contains(&Effect::CancelTest));
        assert!(rig.app.tool.is_none());
    }
}
