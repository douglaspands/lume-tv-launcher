//! lume's logger: every line goes to the system's log (`android_logger`, tag `lume`) and to
//! `files/lume.log` ([`lume_core::logfile`]), which the Log tool reads.
//!
//! The file opens once the loop knows the `files/` directory ([`open`]); the lines logged before
//! that (the start) wait in memory, at most [`EARLY_LINES`] of them. The audio callback never logs,
//! so the lock here is never taken on the real-time thread.

use log::{Level, LevelFilter, Log, Metadata, Record};
use lume_core::logfile::{format_line, LogFile, Stamp};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

/// What lume logs.
const MAX_LEVEL: LevelFilter = LevelFilter::Info;
/// The lines kept until the file opens.
const EARLY_LINES: usize = 64;

enum Sink {
    /// The file is not open yet.
    Early(Vec<String>),
    Open(LogFile),
}

struct LumeLogger {
    system: android_logger::AndroidLogger,
    file: Mutex<Sink>,
}

static LOGGER: OnceLock<LumeLogger> = OnceLock::new();

/// Installs the logger (once per process; later calls do nothing).
pub fn init() {
    let logger = LOGGER.get_or_init(|| LumeLogger {
        system: android_logger::AndroidLogger::new(
            android_logger::Config::default()
                .with_max_level(MAX_LEVEL)
                .with_tag("lume"),
        ),
        file: Mutex::new(Sink::Early(Vec::new())),
    });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(MAX_LEVEL);
    }
}

/// Opens `files/lume.log` and writes the lines logged so far.
pub fn open(files: &Path) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    let Ok(mut sink) = logger.file.lock() else {
        return;
    };
    if let Sink::Early(lines) = &mut *sink {
        let lines = std::mem::take(lines);
        let mut file = LogFile::open(files);
        for line in &lines {
            file.append(line);
        }
        *sink = Sink::Open(file);
    }
}

impl Log for LumeLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= MAX_LEVEL
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        self.system.log(record);
        let level = match record.level() {
            Level::Error => 'E',
            Level::Warn => 'W',
            Level::Info => 'I',
            Level::Debug => 'D',
            Level::Trace => 'V',
        };
        // The module, without the crate's path: `lume::android::tools::audio` is `audio`.
        let tag = record.target().rsplit("::").next().unwrap_or("lume");
        // SAFETY: plain syscalls without arguments.
        let (pid, tid) = unsafe { (libc::getpid(), libc::gettid()) };
        let line = format_line(now(), pid, tid, level, tag, &record.args().to_string());
        if let Ok(mut sink) = self.file.lock() {
            match &mut *sink {
                Sink::Early(lines) if lines.len() < EARLY_LINES => lines.push(line),
                Sink::Early(_) => {}
                Sink::Open(file) => file.append(&line),
            }
        }
    }

    fn flush(&self) {}
}

/// The local time now.
// `timespec` and `tm` fields are 32 bits on armv7 and 64 on the others.
#[allow(clippy::unnecessary_cast)]
fn now() -> Stamp {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` and `tm` are valid to write into; `localtime_r` is the thread-safe variant.
    unsafe {
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&ts.tv_sec, &mut tm).is_null() {
            return Stamp::default();
        }
        Stamp {
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
            second: tm.tm_sec as u32,
            millis: (ts.tv_nsec / 1_000_000) as u32,
        }
    }
}
