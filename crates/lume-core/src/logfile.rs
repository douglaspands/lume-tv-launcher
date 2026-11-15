//! lume's own log file (`log-viewer`): every line lume logs also goes to `files/lume.log`, so the
//! Log tool can show it on the TV. The system's log buffers are too small for that on the test TVs
//! (64 KB on the AIWA, filled by the system in seconds).
//!
//! Rotation keeps the disk use bounded: past [`MAX_BYTES`] the file becomes `lume.log.1` (replacing
//! the old one) and a new one starts, so there are at most two files, [`MAX_BYTES`] each. A line is
//! cut at [`MAX_LINE_BYTES`].
//!
//! The lines use `logcat -v threadtime`'s shape (`11-15 12:24:18.910  3007  3025 I tag: message`),
//! which [`crate::app::tools::log::parse`] reads.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The current file, in the app's `files/`.
pub const FILE: &str = "lume.log";
/// The file before the last rotation.
pub const OLD_FILE: &str = "lume.log.1";
/// The size at which the file rotates.
pub const MAX_BYTES: u64 = 256 * 1024;
/// The longest line kept (a longer one is cut, ending in "…").
pub const MAX_LINE_BYTES: usize = 1024;

/// When a line was logged, in local time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stamp {
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millis: u32,
}

/// One log line in `threadtime` shape, on a single line and at most [`MAX_LINE_BYTES`] long.
pub fn format_line(
    stamp: Stamp,
    pid: i32,
    tid: i32,
    level: char,
    tag: &str,
    message: &str,
) -> String {
    let Stamp {
        month,
        day,
        hour,
        minute,
        second,
        millis,
    } = stamp;
    // A message on several lines (a panic, a backtrace) stays one entry.
    let message = message.replace("\r\n", " | ").replace(['\n', '\r'], " | ");
    let mut line = format!(
        "{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03} {pid:5} {tid:5} {level} {tag}: {message}"
    );
    if line.len() > MAX_LINE_BYTES {
        let mut cut = MAX_LINE_BYTES - '…'.len_utf8();
        while !line.is_char_boundary(cut) {
            cut -= 1;
        }
        line.truncate(cut);
        line.push('…');
    }
    line
}

/// The log file being written.
#[derive(Debug)]
pub struct LogFile {
    dir: PathBuf,
    file: Option<File>,
    size: u64,
}

impl LogFile {
    /// Opens (or creates) `dir/lume.log` to append to it. A file that cannot be opened makes the
    /// writes no-ops (the log still goes to the system).
    pub fn open(dir: &Path) -> LogFile {
        let mut log = LogFile {
            dir: dir.to_path_buf(),
            file: None,
            size: 0,
        };
        log.reopen();
        log
    }

    fn reopen(&mut self) {
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(FILE))
            .ok();
        self.size = self
            .file
            .as_ref()
            .and_then(|f| f.metadata().ok())
            .map_or(0, |m| m.len());
    }

    /// Appends one line (without its newline), rotating first when it would pass [`MAX_BYTES`].
    pub fn append(&mut self, line: &str) {
        let bytes = line.len() as u64 + 1;
        if self.size > 0 && self.size + bytes > MAX_BYTES {
            self.file = None;
            let _ = std::fs::rename(self.dir.join(FILE), self.dir.join(OLD_FILE));
            self.reopen();
        }
        let Some(file) = self.file.as_mut() else {
            return;
        };
        // One write per line: a line is never split between two writers, and it is in the kernel
        // (it survives the process ending) as soon as this returns.
        let mut buffer = Vec::with_capacity(line.len() + 1);
        buffer.extend_from_slice(line.as_bytes());
        buffer.push(b'\n');
        if file.write_all(&buffer).is_ok() {
            self.size += bytes;
        }
    }
}

/// The text of both files, oldest first (what [`crate::app::tools::log::parse`] reads).
pub fn read(dir: &Path) -> String {
    let mut text = String::new();
    for name in [OLD_FILE, FILE] {
        if let Ok(mut file) = File::open(dir.join(name)) {
            let mut bytes = Vec::new();
            if file.read_to_end(&mut bytes).is_ok() {
                text.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tools::log::parse;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lume-logfile-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn stamp() -> Stamp {
        Stamp {
            month: 11,
            day: 15,
            hour: 9,
            minute: 5,
            second: 3,
            millis: 7,
        }
    }

    #[test]
    fn a_line_has_the_threadtime_shape_and_parses_back() {
        let line = format_line(
            stamp(),
            3007,
            3025,
            'E',
            "storage",
            "test failed:\nno space",
        );
        assert_eq!(
            line,
            "11-15 09:05:03.007  3007  3025 E storage: test failed: | no space"
        );
        let parsed = parse(&line);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].time, "11-15 09:05:03");
        assert_eq!(parsed[0].level, 'E');
        assert_eq!(parsed[0].tag, "storage");
        assert_eq!(parsed[0].message, "test failed: | no space");
    }

    #[test]
    fn a_long_line_is_cut_on_a_character() {
        let line = format_line(stamp(), 1, 1, 'I', "lume", &"é".repeat(2000));
        assert!(line.len() <= MAX_LINE_BYTES);
        assert!(line.ends_with('…'));
    }

    #[test]
    fn the_file_rotates_and_never_passes_two_files_of_the_limit() {
        let dir = temp_dir("rotate");
        let mut log = LogFile::open(&dir);
        let line = format_line(stamp(), 1, 1, 'I', "lume", &"x".repeat(900));
        for _ in 0..1000 {
            log.append(&line);
        }
        let size = |name: &str| std::fs::metadata(dir.join(name)).map_or(0, |m| m.len());
        assert!(size(FILE) <= MAX_BYTES);
        assert!(size(OLD_FILE) <= MAX_BYTES);
        assert!(size(OLD_FILE) > MAX_BYTES / 2, "the old file was kept");
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(names.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reading_gives_the_old_file_first_and_survives_a_reopen() {
        let dir = temp_dir("read");
        std::fs::write(dir.join(OLD_FILE), "old\n").unwrap();
        let mut log = LogFile::open(&dir);
        log.append("first");
        drop(log);
        let mut log = LogFile::open(&dir);
        log.append("second");
        assert_eq!(read(&dir), "old\nfirst\nsecond\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
