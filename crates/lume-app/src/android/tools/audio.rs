//! The AAudio stream of the audio tool (D2, D8 in the `ferramentas` design): opened on the first
//! command, closed once [`AudioCommand::Stop`] faded the sound out.
//!
//! The loop and the audio thread share only atomics: the loop writes the command, and AAudio's
//! callback hands it to the synthesizer of `lume-core` (which decides every sample), and tells
//! back when the sound fell silent and where each sync beep starts. Nothing in the callback
//! allocates, locks or logs.

use lume_core::app::tools::audio::synth::{self, Synth};
use lume_core::app::tools::{AudioCommand, ToolEvent};
use ndk::audio::{
    AudioCallbackResult, AudioDirection, AudioError, AudioFormat, AudioPerformanceMode,
    AudioSharingMode, AudioStream, AudioStreamBuilder, Clockid,
};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// No beep started yet.
const NO_BEEP: u64 = u64::MAX;

/// What the loop and the audio callback share.
struct Shared {
    /// The packed command ([`synth::pack`]) in the low 32 bits and its generation in the high
    /// ones, so the same command asked twice is seen twice.
    command: AtomicU64,
    /// The last generation the callback applied.
    applied: AtomicU32,
    /// Whether the synthesizer is silent (nothing plays nor fades out): the stream can go.
    silent: AtomicBool,
    /// The stream broke (the output was unplugged, or the system took it).
    failed: AtomicBool,
    /// The frame (counted from the stream's start) where the last sync beep starts...
    beep_frame: AtomicU64,
    /// ...and when it was written, on `CLOCK_MONOTONIC`: the fallback when AAudio has no
    /// timestamp (it adds the buffer's latency, a constant the user's adjustment absorbs).
    beep_written_ns: AtomicI64,
}

/// An open stream.
struct Open {
    stream: AudioStream,
    shared: Arc<Shared>,
    rate: u32,
    /// The last beep reported to the tool.
    reported: u64,
}

/// The audio output, open only while a sound plays.
#[derive(Default)]
pub struct Audio {
    open: Option<Open>,
    generation: u32,
    /// The loop's clock (`now_ms`) and `CLOCK_MONOTONIC` at the same instant, to turn AAudio's
    /// times into the loop's.
    anchor: (u64, i64),
}

impl Audio {
    /// Plays a sound, opening the stream if needed, or fades it out
    /// ([`AudioCommand::Stop`]; [`Audio::poll`] closes the stream once it is silent).
    pub fn run(&mut self, command: AudioCommand, now_ms: u64) -> Vec<ToolEvent> {
        if now_ms > 0 {
            self.anchor = (now_ms, monotonic_ns());
        }
        self.close_if_done();
        if self.open.is_none() {
            if command == AudioCommand::Stop {
                return vec![];
            }
            match open_stream() {
                Ok(open) => self.open = Some(open),
                Err(e) => {
                    log::warn!("cannot open the audio output: {e}");
                    return vec![ToolEvent::AudioUnavailable];
                }
            }
        }
        if let Some(open) = &self.open {
            self.generation = self.generation.wrapping_add(1).max(1);
            let packed = (self.generation as u64) << 32 | synth::pack(command) as u64;
            open.shared.command.store(packed, Ordering::Release);
        }
        vec![]
    }

    /// What happened on the audio thread since the last call: a sync beep that leaves the
    /// speakers (D8), or a stream that broke. Closes the stream once the sound faded out.
    pub fn poll(&mut self) -> Vec<ToolEvent> {
        let mut events = Vec::new();
        if let Some(open) = self.open.as_mut() {
            if open.shared.failed.load(Ordering::Acquire) {
                log::warn!("the audio output stopped working");
                self.close();
                return vec![ToolEvent::AudioUnavailable];
            }
            let beep = open.shared.beep_frame.load(Ordering::Acquire);
            if beep != NO_BEEP && beep != open.reported {
                open.reported = beep;
                let at_ns = beep_time_ns(open, beep);
                let (anchor_ms, anchor_ns) = self.anchor;
                let at_ms = anchor_ms as i64 + (at_ns - anchor_ns).div_euclid(1_000_000);
                events.push(ToolEvent::AudioStarted {
                    at_ms: at_ms.max(0) as u64,
                });
            }
        }
        self.close_if_done();
        events
    }

    /// Closes the stream when the last command was applied and the sound is over.
    fn close_if_done(&mut self) {
        let done = self.open.as_ref().is_some_and(|open| {
            open.shared.applied.load(Ordering::Acquire) == self.generation
                && open.shared.silent.load(Ordering::Acquire)
        });
        if done {
            self.close();
        }
    }

    fn close(&mut self) {
        if let Some(open) = self.open.take() {
            if let Err(e) = open.stream.request_stop() {
                log::warn!("cannot stop the audio output: {e}");
            }
            // Dropping the stream closes it (AAudio waits for a running callback).
            drop(open.stream);
        }
    }
}

/// When the beep that starts at `frame` leaves the speakers, on `CLOCK_MONOTONIC`: from AAudio's
/// timestamp (a frame it presented and when), or else when the callback wrote it.
fn beep_time_ns(open: &Open, frame: u64) -> i64 {
    match open.stream.timestamp(Clockid::Monotonic) {
        Ok(ts) if ts.frame_position > 0 => {
            let frames = frame as i64 - ts.frame_position;
            ts.time_nanoseconds + frames * 1_000_000_000 / open.rate.max(1) as i64
        }
        _ => open.shared.beep_written_ns.load(Ordering::Acquire),
    }
}

// `timespec`'s fields are 32 bits on armv7 and 64 on the others.
#[allow(clippy::unnecessary_cast)]
fn monotonic_ns() -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid timespec to write into; CLOCK_MONOTONIC always exists.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as i64 * 1_000_000_000 + ts.tv_nsec as i64
}

/// Opens and starts a stereo float stream in low latency, its callback playing the synthesizer.
fn open_stream() -> Result<Open, AudioError> {
    let shared = Arc::new(Shared {
        command: AtomicU64::new(0),
        applied: AtomicU32::new(0),
        silent: AtomicBool::new(true),
        failed: AtomicBool::new(false),
        beep_frame: AtomicU64::new(NO_BEEP),
        beep_written_ns: AtomicI64::new(0),
    });
    let data = shared.clone();
    let mut synth: Option<Synth> = None;
    let mut applied = 0u32;
    let mut written = 0u64;
    let mut was_silent = true;
    let stream = AudioStreamBuilder::new()?
        .direction(AudioDirection::Output)
        .format(AudioFormat::PCM_Float)
        .channel_count(2)
        .performance_mode(AudioPerformanceMode::LowLatency)
        .sharing_mode(AudioSharingMode::Shared)
        .data_callback(Box::new(move |stream, buffer, frames| {
            let rate = stream.sample_rate().max(1) as u32;
            let synth = synth.get_or_insert_with(|| Synth::new(rate));
            let packed = data.command.load(Ordering::Acquire);
            let generation = (packed >> 32) as u32;
            if generation != applied {
                applied = generation;
                synth.play(synth::unpack(packed as u32));
                // Silent no more before the loop sees the command applied, or it could close
                // the stream under the new sound.
                was_silent = synth.is_silent();
                data.silent.store(was_silent, Ordering::Release);
                data.applied.store(generation, Ordering::Release);
            }
            let frames = frames.max(0) as usize;
            // SAFETY: AAudio hands a buffer of `frames` frames of the stream's format, checked
            // to be two float channels when it opened.
            let out = unsafe { std::slice::from_raw_parts_mut(buffer as *mut f32, frames * 2) };
            if let Some(at) = synth.fill(out) {
                let offset_ns = at as i64 * 1_000_000_000 / rate as i64;
                data.beep_written_ns
                    .store(monotonic_ns() + offset_ns, Ordering::Release);
                data.beep_frame
                    .store(written + at as u64, Ordering::Release);
            }
            written += frames as u64;
            let silent = synth.is_silent();
            if silent != was_silent {
                was_silent = silent;
                data.silent.store(silent, Ordering::Release);
                if silent {
                    // The loop may be asleep: it closes the stream at its next turn.
                    crate::android::wake_loop();
                }
            }
            AudioCallbackResult::Continue
        }))
        .error_callback({
            let shared = shared.clone();
            Box::new(move |_, error| {
                log::warn!("audio output error: {error}");
                shared.failed.store(true, Ordering::Release);
                crate::android::wake_loop();
            })
        })
        .open_stream()?;
    if stream.format() != AudioFormat::PCM_Float || stream.channel_count() != 2 {
        log::warn!(
            "the audio output is {:?} with {} channels, not stereo float",
            stream.format(),
            stream.channel_count()
        );
        return Err(AudioError::InvalidFormat);
    }
    stream.request_start()?;
    let rate = stream.sample_rate().max(1) as u32;
    Ok(Open {
        stream,
        shared,
        rate,
        reported: NO_BEEP,
    })
}
