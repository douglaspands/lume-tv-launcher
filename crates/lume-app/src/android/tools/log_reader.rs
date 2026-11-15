//! Reads lume's own log for the Log tool (`log-viewer`), on the `lume-test` thread: the two files
//! of [`lume_core::logfile`] (the current one and the one before the last rotation), of which the
//! tool shows the newest [`MAX_LINES`] lines.
//!
//! The system's log (`logcat`) is not used: on the test TVs its buffers hold seconds to minutes,
//! and lume's few lines are gone before anyone opens the tool.

use super::TestContext;
use lume_core::app::tools::log::{parse, LogUpdate, MAX_LINES};
use lume_core::app::tools::{TestUpdate, ToolEvent};
use lume_core::logfile;

pub fn run(context: &TestContext) {
    let mut lines = parse(&logfile::read(&context.files));
    let extra = lines.len().saturating_sub(MAX_LINES);
    lines.drain(..extra);
    if !context.cancelled() {
        context.send(ToolEvent::Test(TestUpdate::Log(LogUpdate::Lines(lines))));
    }
}
