//! "Minha imagem": the user's own wallpaper.
//!
//! The phone frames the picture and sends the result, already 1920x1080. Apply keeps it as QOI in
//! `files/wallpaper/custom.qoi`, which the system does not clear, with a 128x72 thumbnail for the
//! editor's card next to it; `config.v1` only gets its revision (`custom_rev`). The result is kept
//! in the cache like the catalog's (`wallpaper-custom-<rev>-<version>.rgb`) and decoded again from
//! `files/` when the system clears the cache. The TV keeps neither the original photo nor the framing.
//!
//! What "Minha imagem" shows right now (the applied result, or the picture the Confirm screen shows)
//! is shared with the renderer through a small global state and a generation number that changes
//! with it.

use super::Image;
use crate::memory::Buffer;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// The result, under the app's `files/` directory.
pub const SOURCE_FILE: &str = "wallpaper/custom.qoi";
/// The editor's thumbnail (raw RGB, [`THUMB_WIDTH`] x [`THUMB_HEIGHT`]), next to the result.
pub const THUMB_FILE: &str = "wallpaper/custom-thumb.rgb";
pub const THUMB_WIDTH: u32 = 128;
pub const THUMB_HEIGHT: u32 = 72;
/// The size of the picture the phone sends, kept and cached: the same as the catalog's.
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;

/// "Minha imagem" exists: the revision of the result (it goes up at every Apply, and names the cache).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Applied {
    pub rev: u32,
}

static FILES_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Sets the app's `files/` directory (Android: `internalDataPath`), where the result is kept.
pub fn set_files_dir(dir: PathBuf) {
    let _ = FILES_DIR.set(dir);
}

/// The app's `files/` directory, when the platform set one.
pub fn files_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(dir) = TEST_FILES.with(|dir| dir.borrow().clone()) {
        return Some(dir);
    }
    FILES_DIR.get().cloned()
}

#[cfg(test)]
thread_local! {
    static TEST_FILES: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Gives this test thread its own `files/` directory (the global one belongs to whoever set it first).
#[cfg(test)]
pub(crate) fn set_test_files_dir(dir: Option<PathBuf>) {
    TEST_FILES.with(|d| *d.borrow_mut() = dir);
}

struct State {
    applied: Option<Applied>,
    /// The picture the Confirm screen shows instead of the applied result.
    preview: Option<Arc<Image>>,
    thumbnail: Option<Arc<Image>>,
}

static STATE: Mutex<State> = Mutex::new(State {
    applied: None,
    preview: None,
    thumbnail: None,
});
/// Tests that touch the shared state of "Minha imagem" take this lock, so they do not run together.
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

static GENERATION: AtomicU64 = AtomicU64::new(1);
static THUMB_GENERATION: AtomicU64 = AtomicU64::new(1);
/// Set when the kept result could not be read: the app turns "Minha imagem" off (see [`take_broken`]).
static BROKEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Changes whenever what [`WallpaperId::Custom`](super::WallpaperId::Custom) shows changes: the
/// renderer keeps the texture while it is the same.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

/// "Minha imagem" in use, or `None`.
pub fn applied() -> Option<Applied> {
    state().applied
}

/// Sets "Minha imagem" in use (after Apply, at start from `config.v1`, `None` after Remove).
pub fn set_applied(applied: Option<Applied>) {
    let mut s = state();
    if s.applied != applied {
        s.applied = applied;
        GENERATION.fetch_add(1, Ordering::AcqRel);
    }
}

/// The picture the Confirm screen shows, which the wallpaper texture shows instead of the applied
/// result while it is set.
pub fn preview() -> Option<Arc<Image>> {
    state().preview.clone()
}

/// Shows a received picture as "Minha imagem" (the Confirm screen), or with `None` goes back to the
/// applied result.
pub fn set_preview(preview: Option<Arc<Image>>) {
    let mut s = state();
    if preview.is_none() && s.preview.is_none() {
        return;
    }
    s.preview = preview;
    GENERATION.fetch_add(1, Ordering::AcqRel);
}

/// The editor's thumbnail (RGB, 128x72) and its generation, when there is one.
pub fn thumbnail() -> Option<(u64, Arc<Image>)> {
    let s = state();
    s.thumbnail
        .clone()
        .map(|t| (THUMB_GENERATION.load(Ordering::Acquire), t))
}

/// Sets the editor's thumbnail.
pub fn set_thumbnail(thumbnail: Option<Image>) {
    state().thumbnail = thumbnail.map(Arc::new);
    THUMB_GENERATION.fetch_add(1, Ordering::AcqRel);
}

/// Whether the kept result turned out unreadable since the last call (the app then falls back to
/// the Aurora).
pub fn take_broken() -> bool {
    BROKEN.swap(false, Ordering::AcqRel)
}

// ---- files ----------------------------------------------------------------------------------

/// Writes `bytes` to `path` through a temporary file and a rename, so a reader never sees half of it.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

/// Encodes an RGB picture as QOI.
pub fn encode_qoi(image: &Image) -> Option<Buffer> {
    let mut out = Buffer::zeroed(qoi::encode_max_len(image.width, image.height, 3u8));
    let len = qoi::encode_to_buf(&mut out, &*image.rgb, image.width, image.height).ok()?;
    out.truncate(len);
    Some(out)
}

/// Decodes a QOI picture of exactly 1920x1080 into RGB (an alpha channel is composed over black).
pub fn decode_qoi(bytes: &[u8]) -> Result<Image, DecodeError> {
    let header = qoi::decode_header(bytes).map_err(|_| DecodeError::Header)?;
    let (w, h) = (header.width, header.height);
    if (w, h) != (WIDTH, HEIGHT) {
        return Err(DecodeError::Size);
    }
    // Decoded straight into buffers of their own: they go back to the system when dropped.
    let mut pixels = Buffer::zeroed(header.n_bytes());
    qoi::decode_to_buf(&mut pixels, bytes).map_err(|_| DecodeError::Data)?;
    let rgb = match header.channels {
        qoi::Channels::Rgb => pixels,
        qoi::Channels::Rgba => {
            let mut rgb = Buffer::zeroed(w as usize * h as usize * 3);
            for (out, p) in rgb
                .as_chunks_mut::<3>()
                .0
                .iter_mut()
                .zip(pixels.as_chunks::<4>().0)
            {
                let a = u16::from(p[3]);
                *out = [0, 1, 2].map(|c| ((u16::from(p[c]) * a + 127) / 255) as u8);
            }
            rgb
        }
    };
    Ok(Image {
        width: w,
        height: h,
        rgb,
    })
}

/// Why a QOI picture was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Not a QOI header.
    Header,
    /// Not 1920x1080 (the phone frames the picture before sending it).
    Size,
    /// The data is cut short or inconsistent with the header.
    Data,
}

/// Keeps the result in `files/`.
pub fn save_result(files: &Path, result: &Image) -> io::Result<()> {
    let bytes = encode_qoi(result)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "cannot encode the picture"))?;
    write_atomic(&files.join(SOURCE_FILE), &bytes)
}

/// Reads the kept result back; `None` (logged) when it is missing, does not decode or is not
/// 1920x1080 (the source kept by a development version of this feature).
pub fn load_kept(files: &Path) -> Option<Image> {
    let path = files.join(SOURCE_FILE);
    let bytes = match Buffer::read_file(&path, None) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::warn!("custom wallpaper: cannot read the picture: {e}");
            return None;
        }
    };
    match decode_qoi(&bytes) {
        Ok(image) => Some(image),
        Err(e) => {
            log::warn!("custom wallpaper: the kept picture is not usable: {e:?}");
            None
        }
    }
}

/// Whether there is a kept result in `files/`.
pub fn has_kept(files: &Path) -> bool {
    files.join(SOURCE_FILE).is_file()
}

/// Keeps the editor's thumbnail.
pub fn save_thumbnail(files: &Path, thumbnail: &Image) -> io::Result<()> {
    write_atomic(&files.join(THUMB_FILE), &thumbnail.rgb)
}

/// Reads the editor's thumbnail back.
pub fn load_thumbnail(files: &Path) -> Option<Image> {
    let rgb = Buffer::read_file(&files.join(THUMB_FILE), None).ok()?;
    (rgb.len() == (THUMB_WIDTH * THUMB_HEIGHT * 3) as usize).then_some(Image {
        width: THUMB_WIDTH,
        height: THUMB_HEIGHT,
        rgb,
    })
}

/// The cache file of a result.
pub fn cache_name(rev: u32, version: &str) -> String {
    format!("wallpaper-custom-{rev}-{version}.rgb")
}

/// Keeps `rgb` as the cache file `name`, deleting every other wallpaper in the cache (the catalog's
/// and older results): only the one in use stays.
pub(super) fn store_cache(dir: &Path, name: &str, rgb: &[u8]) {
    let file = dir.join(name);
    let _ = std::fs::create_dir_all(dir);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let entry_name = entry.file_name();
            if entry_name.to_string_lossy().starts_with("wallpaper-") && entry.path() != file {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let _ = write_atomic(&file, rgb);
}

/// The 1920x1080 result of `applied`: from the cache when it is there, otherwise decoded again from
/// `files/` (and kept in the cache). `None` when the kept picture is missing or unusable.
pub fn load_result(
    files: &Path,
    cache: Option<&Path>,
    version: &str,
    applied: Applied,
) -> Option<Image> {
    let started = std::time::Instant::now();
    let name = cache_name(applied.rev, version);
    let size = (WIDTH * HEIGHT * 3) as usize;
    if let Some(rgb) = cache.and_then(|dir| Buffer::read_file(&dir.join(&name), Some(size)).ok()) {
        if rgb.len() == size {
            log::info!(
                "custom wallpaper: from the cache in {} ms",
                started.elapsed().as_millis()
            );
            return Some(Image {
                width: WIDTH,
                height: HEIGHT,
                rgb,
            });
        }
    }
    let result = load_kept(files)?;
    log::info!(
        "custom wallpaper: decoded again in {} ms",
        started.elapsed().as_millis()
    );
    if let Some(dir) = cache {
        store_cache(dir, &name, &result.rgb);
    }
    Some(result)
}

/// What [`WallpaperId::Custom`](super::WallpaperId::Custom) shows now: the Confirm screen's picture
/// while it has one, otherwise the applied result. Called off the frame loop.
pub(super) fn load_current(cache: Option<&Path>, version: &str) -> Option<Arc<Image>> {
    if let Some(preview) = preview() {
        return Some(preview);
    }
    let applied = applied()?;
    let files = files_dir()?;
    match load_result(&files, cache, version, applied) {
        Some(image) => Some(Arc::new(image)),
        None => {
            log::error!("custom wallpaper: no usable picture, back to the Aurora");
            BROKEN.store(true, Ordering::Release);
            None
        }
    }
}

/// Applies a received picture: keeps it in `files/`, its thumbnail next to it and the RGB in the
/// cache under `applied`'s revision. Returns the thumbnail.
pub fn apply(
    files: &Path,
    cache: Option<&Path>,
    version: &str,
    result: &Image,
    applied: Applied,
) -> io::Result<Image> {
    save_result(files, result)?;
    let thumbnail = thumbnail_of(result);
    save_thumbnail(files, &thumbnail)?;
    if let Some(dir) = cache {
        store_cache(dir, &cache_name(applied.rev, version), &result.rgb);
    }
    Ok(thumbnail)
}

/// The editor's thumbnail: an area reduction to 128x72 (each thumbnail pixel is the average of the
/// block of the picture it covers).
pub fn thumbnail_of(picture: &Image) -> Image {
    let (sw, sh) = (picture.width as usize, picture.height as usize);
    let (tw, th) = (THUMB_WIDTH as usize, THUMB_HEIGHT as usize);
    let mut thumb = Image::black(THUMB_WIDTH, THUMB_HEIGHT);
    if sw == 0 || sh == 0 {
        return thumb;
    }
    for ty in 0..th {
        let (y0, y1) = (ty * sh / th, ((ty + 1) * sh / th).max(ty * sh / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * sw / tw, ((tx + 1) * sw / tw).max(tx * sw / tw + 1));
            let mut sum = [0u32; 3];
            for y in y0..y1.min(sh) {
                let row = &picture.rgb[(y * sw + x0) * 3..(y * sw + x1.min(sw)) * 3];
                for p in row.as_chunks::<3>().0 {
                    for (total, value) in sum.iter_mut().zip(p) {
                        *total += u32::from(*value);
                    }
                }
            }
            let n = ((y1.min(sh) - y0) * (x1.min(sw) - x0)).max(1) as u32;
            let at = (ty * tw + tx) * 3;
            for (out, total) in thumb.rgb[at..at + 3].iter_mut().zip(sum) {
                *out = ((total + n / 2) / n) as u8;
            }
        }
    }
    thumb
}

/// Removes "Minha imagem": the kept result, the thumbnail and its results in the cache.
pub fn remove(files: &Path, cache: Option<&Path>) {
    let _ = std::fs::remove_file(files.join(SOURCE_FILE));
    let _ = std::fs::remove_file(files.join(THUMB_FILE));
    if let Some(dir) = cache {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("wallpaper-custom-")
                {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lume-custom-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub(crate) fn picture(width: u32, height: u32) -> Image {
        let rgb = (0..width * height)
            .flat_map(|i| [(i % 251) as u8, (i % 13 * 19) as u8, 90])
            .collect::<Vec<u8>>()
            .into();
        Image { width, height, rgb }
    }

    fn files_named(dir: &Path, prefix: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| n.starts_with(prefix))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    #[test]
    fn the_result_is_saved_and_read_back() {
        let files = temp_dir("result");
        let result = picture(WIDTH, HEIGHT);
        save_result(&files, &result).unwrap();
        assert!(has_kept(&files));
        assert_eq!(load_kept(&files), Some(result));
        let _ = std::fs::remove_dir_all(&files);
    }

    #[test]
    fn a_missing_corrupt_or_wrongly_sized_picture_reads_as_none() {
        let files = temp_dir("corrupt");
        assert_eq!(load_kept(&files), None);
        std::fs::create_dir_all(files.join("wallpaper")).unwrap();
        std::fs::write(
            files.join(SOURCE_FILE),
            b"qoif\0\0\x07\x80\0\0\x04\x38\x03\0cut",
        )
        .unwrap();
        assert_eq!(load_kept(&files), None);
        let applied = Applied { rev: 1 };
        assert_eq!(load_result(&files, None, "1.0", applied), None);
        // The source a development version kept (any size up to 4 MP) is not a result.
        let old = picture(1080, 1920);
        std::fs::write(files.join(SOURCE_FILE), encode_qoi(&old).unwrap()).unwrap();
        assert_eq!(load_kept(&files), None);
        let _ = std::fs::remove_dir_all(&files);
    }

    #[test]
    fn a_cleared_cache_is_decoded_again_to_the_same_rgb() {
        let files = temp_dir("rebuild-files");
        let cache = temp_dir("rebuild-cache");
        let result = picture(WIDTH, HEIGHT);
        let applied = Applied { rev: 3 };
        apply(&files, Some(&cache), "1.0", &result, applied).unwrap();
        let first = std::fs::read(cache.join(cache_name(3, "1.0"))).unwrap();
        assert!(first[..] == result.rgb[..]);
        std::fs::remove_dir_all(&cache).unwrap();
        let again = load_result(&files, Some(&cache), "1.0", applied).unwrap();
        assert!(again.rgb[..] == first[..]);
        assert_eq!(files_named(&cache, "wallpaper-"), [cache_name(3, "1.0")]);
        let _ = std::fs::remove_dir_all(&files);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn applying_again_leaves_a_single_custom_cache_file() {
        let files = temp_dir("again-files");
        let cache = temp_dir("again-cache");
        std::fs::write(cache.join("wallpaper-aurora-1920x1080.qoi-1.0.rgb"), b"x").unwrap();
        let result = picture(WIDTH, HEIGHT);
        apply(&files, Some(&cache), "1.0", &result, Applied { rev: 1 }).unwrap();
        apply(&files, Some(&cache), "1.0", &result, Applied { rev: 2 }).unwrap();
        assert_eq!(files_named(&cache, "wallpaper-"), [cache_name(2, "1.0")]);
        assert!(load_thumbnail(&files).is_some());
        let _ = std::fs::remove_dir_all(&files);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn remove_deletes_the_picture_the_thumbnail_and_the_cache() {
        let files = temp_dir("remove-files");
        let cache = temp_dir("remove-cache");
        let result = picture(WIDTH, HEIGHT);
        apply(&files, Some(&cache), "1.0", &result, Applied { rev: 1 }).unwrap();
        remove(&files, Some(&cache));
        assert!(!has_kept(&files));
        assert!(load_thumbnail(&files).is_none());
        assert!(files_named(&cache, "wallpaper-custom-").is_empty());
        let _ = std::fs::remove_dir_all(&files);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn decoding_wants_1920x1080_and_composes_alpha_over_black() {
        let small = qoi::encode_to_vec(vec![0u8; 1280 * 720 * 3], 1280, 720).unwrap();
        assert_eq!(decode_qoi(&small), Err(DecodeError::Size));
        assert_eq!(decode_qoi(b"not a qoi"), Err(DecodeError::Header));
        let rgba: Vec<u8> = (0..WIDTH * HEIGHT)
            .flat_map(|_| [200, 100, 50, 128])
            .collect();
        let half = decode_qoi(&qoi::encode_to_vec(rgba, WIDTH, HEIGHT).unwrap()).unwrap();
        assert_eq!(&half.rgb[..3], &[100, 50, 25]);
        let mut cut = encode_qoi(&picture(WIDTH, HEIGHT)).unwrap();
        cut.truncate(cut.len() / 2);
        assert_eq!(decode_qoi(&cut), Err(DecodeError::Data));
    }

    #[test]
    fn the_thumbnail_averages_the_blocks_it_covers() {
        // Left half black, right half white: the thumbnail splits the same way.
        let mut image = Image::black(WIDTH, HEIGHT);
        let row_len = WIDTH as usize * 3;
        for y in 0..HEIGHT as usize {
            image.rgb[y * row_len + row_len / 2..(y + 1) * row_len].fill(255);
        }
        let thumb = thumbnail_of(&image);
        assert_eq!((thumb.width, thumb.height), (THUMB_WIDTH, THUMB_HEIGHT));
        let at = |x: usize, y: usize| thumb.rgb[(y * THUMB_WIDTH as usize + x) * 3];
        assert_eq!(
            (at(0, 0), at(63, 40), at(64, 40), at(127, 71)),
            (0, 0, 255, 255)
        );
        // A 2x2 checker of 15x15 blocks averages to grey.
        let mut checker = Image::black(WIDTH, HEIGHT);
        for (i, p) in checker.rgb.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            let (x, y) = (i % WIDTH as usize, i / WIDTH as usize);
            if (x + y) % 2 == 0 {
                p.fill(254);
            }
        }
        let thumb = thumbnail_of(&checker);
        assert!(thumb.rgb.iter().all(|&v| (126..=128).contains(&v)));
    }
}
