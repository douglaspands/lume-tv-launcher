//! What the settings panel shows about the TV: memory, data disk and CPU.
//!
//! The Java bridge answers the `stats` command with five tab-separated fields: total and
//! available memory in KB, total and free disk in bytes, and the CPU reading, which is
//! `stat,<busy>,<total>` (jiffies of /proc/stat), `freq,<current>,<max>` (kHz summed over the
//! cores, for systems that block /proc/stat) or `-`.

/// One CPU reading. A `Busy` one only means something next to an earlier reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuReading {
    Busy { busy: u64, total: u64 },
    Freq { current: u64, max: u64 },
    Unavailable,
}

/// One answer of the `stats` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub memory_total_kb: u64,
    pub memory_available_kb: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub cpu: CpuReading,
}

impl Stats {
    /// Parses the answer; `None` when it is malformed.
    pub fn parse(text: &str) -> Option<Stats> {
        let mut fields = text.trim().split('\t');
        let mut number = || fields.next()?.parse::<u64>().ok();
        let memory_total_kb = number()?;
        let memory_available_kb = number()?;
        let disk_total = number()?;
        let disk_free = number()?;
        let cpu = parse_cpu(fields.next()?);
        if memory_total_kb == 0 || disk_total == 0 {
            return None;
        }
        Some(Stats {
            memory_total_kb,
            memory_available_kb: memory_available_kb.min(memory_total_kb),
            disk_total,
            disk_free: disk_free.min(disk_total),
            cpu,
        })
    }

    /// The fraction (0.0 to 1.0) of the memory in use.
    pub fn memory_used(&self) -> f32 {
        1.0 - self.memory_available_kb as f32 / self.memory_total_kb as f32
    }

    /// The fraction (0.0 to 1.0) of the disk in use.
    pub fn disk_used(&self) -> f32 {
        1.0 - self.disk_free as f32 / self.disk_total as f32
    }

    pub fn memory_used_bytes(&self) -> u64 {
        (self.memory_total_kb - self.memory_available_kb) * 1024
    }

    pub fn memory_total_bytes(&self) -> u64 {
        self.memory_total_kb * 1024
    }

    pub fn disk_used_bytes(&self) -> u64 {
        self.disk_total - self.disk_free
    }
}

fn parse_cpu(text: &str) -> CpuReading {
    let mut parts = text.trim().split(',');
    let kind = parts.next();
    let (Some(a), Some(b)) = (
        parts.next().and_then(|v| v.parse::<u64>().ok()),
        parts.next().and_then(|v| v.parse::<u64>().ok()),
    ) else {
        return CpuReading::Unavailable;
    };
    match kind {
        Some("stat") if b > 0 => CpuReading::Busy { busy: a, total: b },
        Some("freq") if b > 0 => CpuReading::Freq { current: a, max: b },
        _ => CpuReading::Unavailable,
    }
}

/// The CPU use (0.0 to 1.0) at `now`, given the reading before it. The jiffy counters need two
/// readings; the frequency, which is how fast the cores run against their top speed, needs one.
pub fn cpu_use(previous: Option<CpuReading>, now: CpuReading) -> Option<f32> {
    match (previous, now) {
        (
            Some(CpuReading::Busy {
                busy: busy_before,
                total: total_before,
            }),
            CpuReading::Busy { busy, total },
        ) if total > total_before => Some(
            (busy.saturating_sub(busy_before) as f32 / (total - total_before) as f32)
                .clamp(0.0, 1.0),
        ),
        (_, CpuReading::Freq { current, max }) => {
            Some((current as f32 / max as f32).clamp(0.0, 1.0))
        }
        _ => None,
    }
}

/// A size such as `1.2 GB` (or `1,2 GB` with `comma`), from bytes.
pub fn format_size(bytes: u64, comma: bool) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let text = if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / (1024.0 * 1024.0))
    };
    if comma {
        text.replace('.', ",")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_answer_is_read() {
        let s = Stats::parse("2000000\t500000\t10000000000\t4000000000\tstat,300,1000\n").unwrap();
        assert_eq!(s.memory_total_kb, 2_000_000);
        assert!((s.memory_used() - 0.75).abs() < 1e-6);
        assert!((s.disk_used() - 0.6).abs() < 1e-6);
        assert_eq!(
            s.cpu,
            CpuReading::Busy {
                busy: 300,
                total: 1000
            }
        );
    }

    #[test]
    fn without_a_cpu_the_rest_is_kept() {
        let s = Stats::parse("2000000\t500000\t10000000000\t4000000000\t-").unwrap();
        assert_eq!(s.cpu, CpuReading::Unavailable);
        assert_eq!(cpu_use(None, s.cpu), None);
        let s = Stats::parse("2000000\t500000\t10000000000\t4000000000\tstat,x,1").unwrap();
        assert_eq!(s.cpu, CpuReading::Unavailable);
    }

    #[test]
    fn malformed_answers_are_refused() {
        assert_eq!(Stats::parse(""), None);
        assert_eq!(Stats::parse("1\t2\t3"), None);
        assert_eq!(Stats::parse("a\tb\tc\td\t-"), None);
        assert_eq!(Stats::parse("0\t0\t0\t0\t-"), None);
    }

    #[test]
    fn available_cannot_exceed_the_total() {
        let s = Stats::parse("1000\t5000\t100\t900\t-").unwrap();
        assert_eq!(s.memory_used(), 0.0);
        assert_eq!(s.disk_used(), 0.0);
    }

    #[test]
    fn cpu_use_from_two_readings_or_one_frequency() {
        let a = CpuReading::Busy {
            busy: 100,
            total: 1000,
        };
        let b = CpuReading::Busy {
            busy: 400,
            total: 2000,
        };
        assert_eq!(cpu_use(None, a), None);
        assert!((cpu_use(Some(a), b).unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(cpu_use(Some(b), b), None, "no time passed");
        let f = CpuReading::Freq {
            current: 600,
            max: 2400,
        };
        assert!((cpu_use(None, f).unwrap() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn sizes() {
        assert_eq!(format_size(1_610_612_736, false), "1.5 GB");
        assert_eq!(format_size(1_610_612_736, true), "1,5 GB");
        assert_eq!(format_size(300 * 1024 * 1024, false), "300 MB");
    }
}
