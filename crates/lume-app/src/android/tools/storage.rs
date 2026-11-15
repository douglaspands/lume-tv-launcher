//! The storage write and read test (D9), on the `lume-test` thread.
//!
//! It writes [`TEST_MB`] MB in 1 MB blocks from a single buffer, syncs the file so the time counts
//! until the storage has the data, drops the file from the page cache (`posix_fadvise` with
//! `POSIX_FADV_DONTNEED`, which works because the synced pages are clean) and reads it back, so
//! the read comes from the flash and not from memory. The file is deleted at the end, on a failure
//! and on a cancel (and by [`cleanup`] when the tool opens, for a run that was killed).

use super::TestContext;
use lume_core::app::tools::storage::{
    StorageSpace, StorageUpdate, BLOCK_BYTES, MIN_FREE_BYTES, TEST_MB,
};
use lume_core::app::tools::{TestUpdate, ToolEvent};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

/// The start of the temporary file's name, in the app's `files/`. Every run has a file of its own
/// (`lume-storage-test.<run>.tmp`): a cancelled run whose thread is still ending deletes its own
/// file, never the one of the run that replaced it.
const TEST_FILE_PREFIX: &str = "lume-storage-test";

/// Numbers the runs, for their file names.
static RUN: AtomicU32 = AtomicU32::new(0);

/// The free and total bytes of the partition that holds `dir`.
pub fn space(dir: &Path) -> Option<StorageSpace> {
    let path = CString::new(dir.as_os_str().as_bytes()).ok()?;
    // SAFETY: `statvfs` fills the zeroed struct or fails.
    let stat = unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path.as_ptr(), &mut stat) != 0 {
            return None;
        }
        stat
    };
    let unit = stat.f_frsize as u64;
    Some(StorageSpace {
        free_bytes: stat.f_bavail as u64 * unit,
        total_bytes: stat.f_blocks as u64 * unit,
    })
}

/// How the run ended, when it did not finish.
enum Stop {
    /// Cancelled, or nobody listens any more: nothing more to say.
    Cancelled,
    Failed(std::io::Error),
}

impl From<std::io::Error> for Stop {
    fn from(e: std::io::Error) -> Stop {
        Stop::Failed(e)
    }
}

fn send(context: &TestContext, update: StorageUpdate) -> Result<(), Stop> {
    if context.send(ToolEvent::Test(TestUpdate::Storage(update))) {
        Ok(())
    } else {
        Err(Stop::Cancelled)
    }
}

fn mb_per_s(seconds: f64) -> f32 {
    (TEST_MB as f64 / seconds.max(1e-6)) as f32
}

pub fn run(context: &TestContext) {
    let Some(space) = space(&context.files) else {
        log::warn!("storage test: statvfs failed");
        let _ = send(context, StorageUpdate::Failed);
        return;
    };
    let run = RUN.fetch_add(1, Ordering::Relaxed);
    let path = context.files.join(format!("{TEST_FILE_PREFIX}.{run}.tmp"));
    let result = if space.free_bytes < MIN_FREE_BYTES {
        send(context, StorageUpdate::NoSpace(space))
    } else {
        send(context, StorageUpdate::Space(space)).and_then(|()| measure(context, &path))
    };
    let _ = std::fs::remove_file(&path);
    if let Err(Stop::Failed(e)) = result {
        log::warn!("storage test failed: {e}");
        let _ = send(context, StorageUpdate::Failed);
    }
}

/// Writes, syncs, drops from the cache and reads back the test file, sending the progress.
fn measure(context: &TestContext, path: &Path) -> Result<(), Stop> {
    // The only data buffer of the test; not zeros, so no layer can skip writing it.
    let mut block: Vec<u8> = (0..BLOCK_BYTES).map(|i| (i % 251) as u8).collect();

    let start = Instant::now();
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    for mb in 1..=TEST_MB {
        if context.cancelled() {
            return Err(Stop::Cancelled);
        }
        file.write_all(&block)?;
        if mb < TEST_MB {
            send(context, StorageUpdate::Writing { mb })?;
        }
    }
    file.sync_all()?;
    let write_mb_s = mb_per_s(start.elapsed().as_secs_f64());
    drop_cache(&file);
    drop(file);
    send(context, StorageUpdate::Writing { mb: TEST_MB })?;

    let start = Instant::now();
    let mut file = File::open(path)?;
    drop_cache(&file);
    for mb in 1..=TEST_MB {
        if context.cancelled() {
            return Err(Stop::Cancelled);
        }
        file.read_exact(&mut block)?;
        send(context, StorageUpdate::Reading { mb, write_mb_s })?;
    }
    let read_mb_s = mb_per_s(start.elapsed().as_secs_f64());
    drop(file);
    // The file goes before the result, so the test is over when the screen says so.
    let _ = std::fs::remove_file(path);
    send(
        context,
        StorageUpdate::Done {
            write_mb_s,
            read_mb_s,
        },
    )
}

/// Asks the kernel to forget the file's cached pages, so the read goes to the storage.
fn drop_cache(file: &File) {
    // SAFETY: a valid open descriptor; the call only gives advice about its pages.
    let result = unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
    if result != 0 {
        log::warn!("storage test: posix_fadvise failed ({result})");
    }
}

/// Deletes the test files that runs left (finished, cancelled or killed). A run still ending keeps
/// writing to its open file, which goes once it is closed.
pub fn cleanup(files: &Path) {
    let Ok(entries) = std::fs::read_dir(files) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(TEST_FILE_PREFIX) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
