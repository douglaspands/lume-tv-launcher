//! The platform side of the Tools (D2 in the `ferramentas` design): the window flag that keeps the
//! screen on, the audio stream, the `lume-test` thread and the periodic readings. Nothing here
//! exists until a tool asks for it, and everything goes when the tool closes.

mod audio;
mod log_reader;
mod storage;

use android_activity::{AndroidApp, WindowManagerFlags};
use lume_core::app::tools::{AudioCommand, ReadingKind, TestKind, ToolEvent, ToolReading};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;

/// What a test running on the `lume-test` thread gets: where to send its progress and whether it
/// was cancelled.
pub struct TestContext {
    cancel: Arc<AtomicBool>,
    tx: Sender<ToolEvent>,
    /// The app's `files/` directory (the storage test writes its temporary file there).
    pub files: PathBuf,
}

impl TestContext {
    /// Whether the test must stop now (the user left, or lume went to the background).
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Sends progress or a result to the loop and wakes it. Returns `false` when nobody listens
    /// any more (the test was cancelled): the test should stop.
    pub fn send(&self, event: ToolEvent) -> bool {
        let sent = self.tx.send(event).is_ok();
        super::wake_loop();
        sent && !self.cancelled()
    }
}

/// A test running on the `lume-test` thread.
struct TestRun {
    cancel: Arc<AtomicBool>,
    rx: Receiver<ToolEvent>,
    thread: Option<JoinHandle<()>>,
}

/// The resources of the Tools, alive only while a tool uses them.
pub struct Tools {
    files: PathBuf,
    test: Option<TestRun>,
    audio: audio::Audio,
}

impl Tools {
    pub fn new(files: PathBuf) -> Tools {
        Tools {
            files,
            test: None,
            audio: audio::Audio::default(),
        }
    }

    /// Keeps the screen on (no screen saver, no sleep) while the Tools are open (`KEEP_SCREEN_ON`
    /// on the activity's window; no Java needed).
    pub fn keep_screen_on(app: &AndroidApp, on: bool) {
        let flag = WindowManagerFlags::KEEP_SCREEN_ON;
        if on {
            app.set_window_flags(flag, WindowManagerFlags::empty());
        } else {
            app.set_window_flags(WindowManagerFlags::empty(), flag);
        }
    }

    /// Plays or stops a sound. Returns what the audio tool must be told (the stream could not be
    /// opened, or a beep left the speakers).
    pub fn audio(&mut self, command: AudioCommand, now_ms: u64) -> Vec<ToolEvent> {
        self.audio.run(command, now_ms)
    }

    /// Starts a test on a new `lume-test` thread, cancelling the one that was running.
    pub fn start_test(&mut self, kind: TestKind) {
        self.cancel_test();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let context = TestContext {
            cancel: cancel.clone(),
            tx,
            files: self.files.clone(),
        };
        let thread =
            std::thread::Builder::new()
                .name("lume-test".into())
                .spawn(move || match kind {
                    TestKind::Storage => storage::run(&context),
                    TestKind::Log => log_reader::run(&context),
                });
        match thread {
            Ok(thread) => {
                self.test = Some(TestRun {
                    cancel,
                    rx,
                    thread: Some(thread),
                })
            }
            Err(e) => log::warn!("cannot start the {kind:?} test: {e}"),
        }
    }

    /// Cancels the running test, if any, and removes what a test may have left behind (the
    /// storage test's file, also after a run that was killed). The thread notices within its next
    /// step and ends on its own.
    pub fn cancel_test(&mut self) {
        if let Some(mut run) = self.test.take() {
            run.cancel.store(true, Ordering::SeqCst);
            // Dropping the receiver makes the thread's next send fail; it is not joined here, so
            // the loop never waits on the storage. The thread deletes its own file when it ends.
            drop(run.thread.take());
        }
        storage::cleanup(&self.files);
    }

    /// What the audio stream and the running test sent since the last call. The test is forgotten once its
    /// thread ended.
    pub fn poll(&mut self) -> Vec<ToolEvent> {
        // The audio stream closes here once its sound faded out, and reports the sync beeps.
        let mut events = self.audio.poll();
        let Some(run) = self.test.as_mut() else {
            return events;
        };
        loop {
            match run.rx.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.test = None;
                    break;
                }
            }
        }
        events
    }

    /// Makes a periodic reading for the open tool.
    pub fn read(&mut self, kind: ReadingKind) -> ToolEvent {
        match kind {
            ReadingKind::Storage => {
                ToolEvent::Reading(ToolReading::Storage(storage::space(&self.files)))
            }
        }
    }

    /// lume leaves the screen or the tool closes: the sound and the test stop.
    pub fn release(&mut self) {
        self.audio.run(AudioCommand::Stop, 0);
        self.cancel_test();
    }
}
