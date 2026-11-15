//! Giving memory back to the system as soon as it is not in use.
//!
//! A wallpaper picture is 6.2 MB of RGB that lives a moment (until the GPU has it) or a while (the
//! Confirm screen). Allocated on the heap, it stays in the process after it is freed: the allocator
//! keeps the pages for the next allocation, and some (Scudo on the AIWA, Android 14) keep them
//! however they are asked. So large buffers ([`Buffer`]) are mapped on their own (`mmap`) and
//! unmapped when dropped: their memory goes back to the system at once, whatever the allocator.
//! [`release_freed`] still asks the allocator for the small freed pages around them.

use std::ops::{Deref, DerefMut};

/// Buffers from this size up get their own mapping.
pub const MAP_MIN: usize = 512 * 1024;

/// A byte buffer whose memory goes back to the system when it is dropped (when it is large).
pub struct Buffer {
    inner: Inner,
}

enum Inner {
    Heap(Vec<u8>),
    /// An anonymous mapping and the bytes of it in use.
    Mapped(memmap2::MmapMut, usize),
}

impl Buffer {
    /// `len` zero bytes: mapped from [`MAP_MIN`] up, on the heap below (or when mapping fails).
    pub fn zeroed(len: usize) -> Buffer {
        if len >= MAP_MIN {
            match memmap2::MmapMut::map_anon(len) {
                Ok(map) => {
                    return Buffer {
                        inner: Inner::Mapped(map, len),
                    }
                }
                Err(e) => log::warn!("memory: cannot map {len} bytes ({e}), on the heap instead"),
            }
        }
        Buffer {
            inner: Inner::Heap(vec![0; len]),
        }
    }

    /// A buffer with these bytes (copied into a mapping when large).
    pub fn copy_from(bytes: &[u8]) -> Buffer {
        let mut buffer = Buffer::zeroed(bytes.len());
        buffer.copy_from_slice(bytes);
        buffer
    }

    /// Reads a whole file; `expected` refuses a file of another size before reading it.
    pub fn read_file(path: &std::path::Path, expected: Option<usize>) -> std::io::Result<Buffer> {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let len = file.metadata()?.len() as usize;
        if expected.is_some_and(|e| e != len) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected size",
            ));
        }
        let mut buffer = Buffer::zeroed(len);
        file.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Keeps only the first `len` bytes.
    pub fn truncate(&mut self, len: usize) {
        match &mut self.inner {
            Inner::Heap(v) => v.truncate(len),
            Inner::Mapped(_, used) => *used = (*used).min(len),
        }
    }

    /// Whether the buffer has its own mapping.
    pub fn is_mapped(&self) -> bool {
        matches!(self.inner, Inner::Mapped(..))
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(v: Vec<u8>) -> Buffer {
        Buffer {
            inner: Inner::Heap(v),
        }
    }
}

impl FromIterator<u8> for Buffer {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Buffer {
        let bytes: Vec<u8> = iter.into_iter().collect();
        if bytes.len() >= MAP_MIN {
            Buffer::copy_from(&bytes)
        } else {
            Buffer::from(bytes)
        }
    }
}

impl Deref for Buffer {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match &self.inner {
            Inner::Heap(v) => v,
            Inner::Mapped(map, used) => &map[..*used],
        }
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        match &mut self.inner {
            Inner::Heap(v) => v,
            Inner::Mapped(map, used) => &mut map[..*used],
        }
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        self
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Buffer {
        Buffer::copy_from(self)
    }
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Buffer) -> bool {
        **self == **other
    }
}

impl Eq for Buffer {}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Buffer({} bytes, mapped: {})",
            self.len(),
            self.is_mapped()
        )
    }
}

/// Returns the freed pages of the heap to the system (what is left there after a big buffer).
/// Cheap enough to call after a big picture is dropped; never in a frame.
pub fn release_freed() {
    #[cfg(target_os = "android")]
    {
        // bionic's <malloc.h>: M_PURGE_ALL (API 34, every size class) and M_PURGE (API 28).
        const M_PURGE: std::ffi::c_int = -101;
        const M_PURGE_ALL: std::ffi::c_int = -104;
        let started = std::time::Instant::now();
        // SAFETY: `mallopt` only reads its two integers; an option the system does not know
        // returns 0 and changes nothing.
        extern "C" {
            fn mallopt(param: std::ffi::c_int, value: std::ffi::c_int) -> std::ffi::c_int;
        }
        let (all, some) = unsafe {
            let all = mallopt(M_PURGE_ALL, 0);
            (all, if all == 0 { mallopt(M_PURGE, 0) } else { 1 })
        };
        log::info!(
            "memory: freed pages released (purge all {all}, purge {some}) in {} ms",
            started.elapsed().as_millis()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_buffers_are_mapped_and_small_ones_are_not() {
        let big = Buffer::zeroed(1920 * 1080 * 3);
        assert_eq!(big.len(), 1920 * 1080 * 3);
        assert!(big.iter().all(|&b| b == 0));
        assert!(big.is_mapped());
        assert!(!Buffer::zeroed(1000).is_mapped());
    }

    #[test]
    fn a_buffer_reads_writes_clones_and_truncates_like_a_vec() {
        let mut big = Buffer::zeroed(MAP_MIN + 3);
        big[MAP_MIN] = 7;
        let copy = big.clone();
        assert_eq!(copy, big);
        assert_eq!(copy[MAP_MIN], 7);
        big.truncate(10);
        assert_eq!(big.len(), 10);
        assert_ne!(copy, big);
        assert_eq!(Buffer::from(vec![1, 2, 3]), Buffer::copy_from(&[1, 2, 3]));
    }

    #[test]
    fn a_file_is_read_whole_and_its_size_checked() {
        let path = std::env::temp_dir().join(format!("lume-buffer-{}", std::process::id()));
        let bytes: Vec<u8> = (0..MAP_MIN + 10).map(|i| i as u8).collect();
        std::fs::write(&path, &bytes).unwrap();
        let read = Buffer::read_file(&path, None).unwrap();
        assert!(read.is_mapped() && *read == bytes[..]);
        assert!(Buffer::read_file(&path, Some(5)).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
