//! The synthesizer of the audio tool (D2 in the `ferramentas` design): pure functions and a small
//! state machine that fill interleaved stereo `f32` buffers. The platform's audio callback only
//! calls [`Synth::fill`], so everything that decides what is heard is tested here, without a
//! sound card.
//!
//! Nothing here allocates or locks: [`Synth::fill`] runs on the real-time audio thread.

use crate::app::tools::{AudioCommand, Channel};
use std::f64::consts::TAU;

/// The tone of the channel test (`audio-diagnostics`, "Tons por canal").
pub const TONE_HZ: u32 = 440;
/// The fade in and out of every sound, so it never clicks.
pub const RAMP_MS: u32 = 20;
/// The sweep: 20 Hz to 20 kHz on a logarithmic scale, in 30 s.
pub const SWEEP_FROM_HZ: f64 = 20.0;
pub const SWEEP_TO_HZ: f64 = 20_000.0;
pub const SWEEP_MS: u32 = 30_000;
/// The beep of the sync test: 1 kHz for 50 ms.
pub const BEEP_HZ: f64 = 1000.0;
pub const BEEP_MS: u32 = 50;
/// The fade of a beep. Shorter than [`RAMP_MS`]: a 50 ms beep with 20 ms ramps would have no
/// clear onset to match with the square; 2 ms is still long enough not to click.
pub const BEEP_RAMP_MS: u32 = 2;
/// The peak amplitude: 0.25 is -12.04 dBFS, under the -12 dBFS of the spec.
pub const PEAK: f32 = 0.25;

/// The frequency of the sweep `elapsed_ms` after it started (held at the ends).
pub fn sweep_hz(elapsed_ms: f64) -> f64 {
    let t = (elapsed_ms / SWEEP_MS as f64).clamp(0.0, 1.0);
    SWEEP_FROM_HZ * (SWEEP_TO_HZ / SWEEP_FROM_HZ).powf(t)
}

/// A linear fade in over `ramp` frames from frame 0 and a fade out over the last `ramp` frames
/// of `length` (when the sound has an end): the gain at `frame`, from 0 to 1.
pub fn envelope(frame: u64, length: Option<u64>, ramp: u64) -> f32 {
    let ramp = ramp.max(1);
    let attack = (frame as f32 / ramp as f32).min(1.0);
    let release = match length {
        Some(length) => (length.saturating_sub(frame) as f32 / ramp as f32).min(1.0),
        None => 1.0,
    };
    attack.min(release)
}

/// Packs a command in a `u32`, so the loop can hand it to the audio thread through an atomic.
pub fn pack(command: AudioCommand) -> u32 {
    match command {
        AudioCommand::Stop => 0,
        AudioCommand::Tone { channel, hz } => {
            let kind = match channel {
                Channel::Left => 1,
                Channel::Right => 2,
                Channel::Both => 3,
            };
            kind << 24 | (hz & 0xFF_FFFF)
        }
        AudioCommand::Sweep => 4 << 24,
        AudioCommand::SyncBeep { period_ms } => 5 << 24 | (period_ms & 0xFF_FFFF),
    }
}

/// The command packed by [`pack`].
pub fn unpack(packed: u32) -> AudioCommand {
    let value = packed & 0xFF_FFFF;
    let tone = |channel| AudioCommand::Tone { channel, hz: value };
    match packed >> 24 {
        1 => tone(Channel::Left),
        2 => tone(Channel::Right),
        3 => tone(Channel::Both),
        4 => AudioCommand::Sweep,
        5 => AudioCommand::SyncBeep { period_ms: value },
        _ => AudioCommand::Stop,
    }
}

/// A sound that is playing.
#[derive(Clone, Copy, Debug)]
struct Voice {
    command: AudioCommand,
    /// Frames since it started.
    frame: u64,
    /// The position in the wave's cycle, from 0 to 1.
    phase: f64,
    /// Frames left of the fade out, once it was asked to stop.
    release: Option<u64>,
}

/// Plays one sound at a time; a new one fades the current one out first.
#[derive(Clone, Debug)]
pub struct Synth {
    rate: u32,
    voice: Option<Voice>,
    /// What plays once the current voice faded out (`None`: silence).
    next: Option<AudioCommand>,
}

impl Synth {
    pub fn new(rate: u32) -> Synth {
        Synth {
            rate: rate.max(1),
            voice: None,
            next: None,
        }
    }

    fn frames(&self, ms: u32) -> u64 {
        self.rate as u64 * ms as u64 / 1000
    }

    /// Asks for a sound ([`AudioCommand::Stop`]: silence). What plays fades out first.
    pub fn play(&mut self, command: AudioCommand) {
        let next = (command != AudioCommand::Stop).then_some(command);
        let ramp = self.frames(RAMP_MS);
        match self.voice.as_mut() {
            Some(voice) => {
                voice.release.get_or_insert(ramp);
                self.next = next;
            }
            None => self.voice = next.map(Voice::new),
        }
    }

    /// Whether nothing plays nor is about to: the stream can be closed.
    pub fn is_silent(&self) -> bool {
        self.voice.is_none() && self.next.is_none()
    }

    /// Fills `out` (interleaved left and right samples). Returns the frame of `out` where a sync
    /// beep starts, if one does (the beeps are a second apart, so at most one per buffer).
    pub fn fill(&mut self, out: &mut [f32]) -> Option<usize> {
        let mut beep = None;
        for (index, frame) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let (left, right, starts_beep) = self.next_frame();
            frame[0] = left;
            frame[1] = right;
            if starts_beep && beep.is_none() {
                beep = Some(index);
            }
        }
        beep
    }

    /// One frame: left, right, and whether a beep starts at it.
    fn next_frame(&mut self) -> (f32, f32, bool) {
        let ramp = self.frames(RAMP_MS);
        let beep_frames = self.frames(BEEP_MS);
        let beep_ramp = self.frames(BEEP_RAMP_MS);
        let sweep_frames = self.frames(SWEEP_MS);
        let rate = self.rate as f64;
        let Some(voice) = self.voice.as_mut() else {
            return (0.0, 0.0, false);
        };
        let mut starts_beep = false;
        let (hz, gain, channel) = match voice.command {
            AudioCommand::Tone { channel, hz } => {
                (hz as f64, envelope(voice.frame, None, ramp), channel)
            }
            AudioCommand::Sweep => {
                let elapsed_ms = voice.frame as f64 * 1000.0 / rate;
                let gain = envelope(voice.frame, Some(sweep_frames), ramp);
                (sweep_hz(elapsed_ms), gain, Channel::Both)
            }
            AudioCommand::SyncBeep { period_ms } => {
                let period = self.rate as u64 * period_ms.max(BEEP_MS) as u64 / 1000;
                let in_period = voice.frame % period;
                if in_period == 0 {
                    // Every beep starts at the top of a cycle, the same at every repetition.
                    voice.phase = 0.0;
                    starts_beep = voice.release.is_none();
                }
                let gain = if in_period < beep_frames {
                    envelope(in_period, Some(beep_frames), beep_ramp)
                } else {
                    0.0
                };
                (BEEP_HZ, gain, Channel::Both)
            }
            AudioCommand::Stop => (0.0, 0.0, Channel::Both),
        };
        let release = voice
            .release
            .map_or(1.0, |left| left as f32 / ramp.max(1) as f32);
        let sample = (voice.phase * TAU).sin() as f32 * PEAK * gain * release;
        voice.phase = (voice.phase + hz / rate).fract();
        voice.frame += 1;
        let ended = match voice.release.as_mut() {
            Some(left) => {
                *left = left.saturating_sub(1);
                *left == 0
            }
            None => matches!(voice.command, AudioCommand::Sweep) && voice.frame >= sweep_frames,
        };
        if ended {
            self.voice = self.next.take().map(Voice::new);
        }
        match channel {
            Channel::Left => (sample, 0.0, starts_beep),
            Channel::Right => (0.0, sample, starts_beep),
            Channel::Both => (sample, sample, starts_beep),
        }
    }
}

impl Voice {
    fn new(command: AudioCommand) -> Voice {
        Voice {
            command,
            frame: 0,
            phase: 0.0,
            release: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn render(synth: &mut Synth, ms: u32) -> Vec<f32> {
        let mut out = vec![0.0; (RATE as usize * ms as usize / 1000) * 2];
        synth.fill(&mut out);
        out
    }

    fn peak(samples: impl Iterator<Item = f32>) -> f32 {
        samples.fold(0.0, |m, s| m.max(s.abs()))
    }

    fn left(out: &[f32]) -> impl Iterator<Item = f32> + '_ {
        out.iter().step_by(2).copied()
    }

    fn right(out: &[f32]) -> impl Iterator<Item = f32> + '_ {
        out.iter().skip(1).step_by(2).copied()
    }

    #[test]
    fn no_sound_goes_over_minus_12_dbfs() {
        let limit = 10f32.powf(-12.0 / 20.0);
        for command in [
            AudioCommand::Tone {
                channel: Channel::Both,
                hz: TONE_HZ,
            },
            AudioCommand::Sweep,
            AudioCommand::SyncBeep { period_ms: 1000 },
        ] {
            let mut synth = Synth::new(RATE);
            synth.play(command);
            let out = render(&mut synth, 2000);
            let peak = peak(out.iter().copied());
            assert!(peak <= limit, "{command:?} peaks at {peak}");
            assert!(peak > 0.9 * PEAK, "{command:?} is too quiet: {peak}");
        }
    }

    #[test]
    fn a_tone_plays_only_in_its_channel() {
        let mut synth = Synth::new(RATE);
        synth.play(AudioCommand::Tone {
            channel: Channel::Left,
            hz: TONE_HZ,
        });
        let out = render(&mut synth, 100);
        assert!(peak(left(&out)) > 0.2);
        assert_eq!(peak(right(&out)), 0.0);

        // Switching to the right: the left fades out, then only the right plays.
        synth.play(AudioCommand::Tone {
            channel: Channel::Right,
            hz: TONE_HZ,
        });
        render(&mut synth, RAMP_MS + 1);
        let out = render(&mut synth, 100);
        assert_eq!(peak(left(&out)), 0.0);
        assert!(peak(right(&out)) > 0.2);
    }

    #[test]
    fn sounds_fade_in_and_out_over_20_ms() {
        let mut synth = Synth::new(RATE);
        synth.play(AudioCommand::Tone {
            channel: Channel::Both,
            hz: TONE_HZ,
        });
        // The first 10 ms stay under half the peak; the full level only comes at 20 ms.
        let start = render(&mut synth, 10);
        assert!(peak(left(&start)) <= PEAK * 0.5 + 1e-3);
        let rest = render(&mut synth, 10);
        assert!(peak(left(&rest)) > PEAK * 0.5);
        assert!(peak(left(&rest)) < PEAK);
        let steady = render(&mut synth, 50);
        assert!(peak(left(&steady)) > PEAK * 0.99);

        synth.play(AudioCommand::Stop);
        assert!(!synth.is_silent(), "it fades out first");
        let fade = render(&mut synth, 10);
        assert!(peak(left(&fade)) > PEAK * 0.4, "no sudden stop");
        render(&mut synth, 10);
        assert!(synth.is_silent());
        let after = render(&mut synth, 10);
        assert_eq!(peak(after.iter().copied()), 0.0);
    }

    #[test]
    fn the_envelope_rises_and_falls_linearly() {
        assert_eq!(envelope(0, None, 960), 0.0);
        assert_eq!(envelope(480, None, 960), 0.5);
        assert_eq!(envelope(5000, None, 960), 1.0);
        assert_eq!(envelope(10_000 - 480, Some(10_000), 960), 0.5);
        assert_eq!(envelope(10_000, Some(10_000), 960), 0.0);
    }

    #[test]
    fn the_sweep_is_logarithmic_and_stops_by_itself() {
        assert_eq!(sweep_hz(0.0), 20.0);
        // Halfway in time is the geometric middle: sqrt(20 * 20000) = 632 Hz.
        assert!((sweep_hz(15_000.0) - 632.46).abs() < 0.01);
        assert!((sweep_hz(30_000.0) - 20_000.0).abs() < 1e-6);
        assert!((sweep_hz(60_000.0) - 20_000.0).abs() < 1e-6);

        // Mid-sweep, the wave really is at about 632 Hz: count the zero crossings in 100 ms.
        let mut synth = Synth::new(RATE);
        synth.play(AudioCommand::Sweep);
        render(&mut synth, 14_950);
        let out = render(&mut synth, 100);
        let samples: Vec<f32> = left(&out).collect();
        let crossings = samples
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count();
        let hz = crossings as f64 / 2.0 / 0.1;
        assert!((hz - 632.0).abs() < 15.0, "{hz} Hz");

        render(&mut synth, 15_000);
        assert!(synth.is_silent());
    }

    #[test]
    fn a_beep_lasts_50_ms_every_second_and_its_start_is_reported() {
        let mut synth = Synth::new(RATE);
        synth.play(AudioCommand::SyncBeep { period_ms: 1000 });
        let mut out = vec![0.0; 480 * 2];
        assert_eq!(synth.fill(&mut out), Some(0));
        let mut starts = vec![];
        let mut samples = vec![];
        for buffer in 1..200 {
            if let Some(frame) = synth.fill(&mut out) {
                starts.push(buffer * 480 + frame);
            }
            samples.extend(left(&out));
        }
        assert_eq!(starts, vec![48_000]);
        // The second beep (the one starting at 1 s, frame 47 520 of `samples`) ends 50 ms later,
        // and all is silent until the next one.
        let loud: Vec<usize> = (0..samples.len())
            .filter(|i| samples[*i].abs() > 1e-6)
            .map(|i| i + 480)
            .collect();
        let second: Vec<usize> = loud.into_iter().filter(|f| *f >= 48_000).collect();
        let span = second.last().unwrap() - second.first().unwrap();
        assert!(span.abs_diff(50 * 48) < 4, "the beep spans {span} frames");
    }

    #[test]
    fn commands_survive_the_packing() {
        for command in [
            AudioCommand::Stop,
            AudioCommand::Sweep,
            AudioCommand::SyncBeep { period_ms: 1000 },
            AudioCommand::Tone {
                channel: Channel::Left,
                hz: 440,
            },
            AudioCommand::Tone {
                channel: Channel::Right,
                hz: 20_000,
            },
            AudioCommand::Tone {
                channel: Channel::Both,
                hz: 1,
            },
        ] {
            assert_eq!(unpack(pack(command)), command);
        }
    }
}
