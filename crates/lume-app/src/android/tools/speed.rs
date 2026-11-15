//! The internet speed test against speed.cloudflare.com (D3 in the `ferramentas` design), on the
//! `lume-test` thread: the latency (median of 5 empty requests), the download (up to 25 MB or
//! 10 s) and the upload (up to 10 MB or 10 s), over HTTPS with `ureq` and `rustls`.
//!
//! One 64 KB buffer is read into and sent from; the connections close when the test ends, fails
//! or is cancelled (the agent and its pool are dropped with the function). The thread looks at the
//! cancellation between blocks, a few milliseconds apart while data flows.

use super::TestContext;
use lume_core::app::tools::network::{
    mbps, median, SpeedFailure, SpeedPhase, SpeedUpdate, BLOCK_BYTES, CONNECT_MS,
    DOWNLOAD_MAX_BYTES, LATENCY_PINGS, PHASE_MS, PROGRESS_MS, UPLOAD_MAX_BYTES,
};
use lume_core::app::tools::{TestUpdate, ToolEvent};
use std::io::{self, Read};
use std::time::{Duration, Instant};

const DOWN_URL: &str = "https://speed.cloudflare.com/__down?bytes=";
const UP_URL: &str = "https://speed.cloudflare.com/__up";

/// Why a phase stopped early.
enum Stop {
    /// The user left: nothing more is sent.
    Cancelled,
    Failed(SpeedFailure),
}

impl From<ureq::Error> for Stop {
    fn from(error: ureq::Error) -> Stop {
        log::info!("speed test: {error}");
        Stop::Failed(failure(&error))
    }
}

/// What the screen says about an error: the service unreachable, the service failing, or a
/// timeout.
fn failure(error: &ureq::Error) -> SpeedFailure {
    match error {
        ureq::Error::Timeout(_) | ureq::Error::BodyStalled => SpeedFailure::Timeout,
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => SpeedFailure::NoInternet,
        ureq::Error::Io(e) => io_failure(e),
        _ => SpeedFailure::Server,
    }
}

fn io_failure(error: &io::Error) -> SpeedFailure {
    if let Some(inner) = error
        .get_ref()
        .and_then(|e| e.downcast_ref::<ureq::Error>())
    {
        return failure(inner);
    }
    match error.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => SpeedFailure::Timeout,
        io::ErrorKind::ConnectionRefused
        | io::ErrorKind::NetworkUnreachable
        | io::ErrorKind::HostUnreachable
        | io::ErrorKind::NetworkDown
        | io::ErrorKind::AddrNotAvailable => SpeedFailure::NoInternet,
        _ => SpeedFailure::Server,
    }
}

/// Sends an update; a cancelled test stops at the next check.
fn send(context: &TestContext, update: SpeedUpdate) -> Result<(), Stop> {
    if context.send(ToolEvent::Test(TestUpdate::Speed(update))) {
        Ok(())
    } else {
        Err(Stop::Cancelled)
    }
}

fn check(context: &TestContext) -> Result<(), Stop> {
    if context.cancelled() {
        Err(Stop::Cancelled)
    } else {
        Ok(())
    }
}

fn millis(since: Instant) -> f32 {
    since.elapsed().as_secs_f32() * 1000.0
}

/// 5 s to resolve, connect and get the first byte of an answer; 15 s for a whole call, a backstop
/// for a stalled transfer (each phase ends itself at 10 s).
fn agent() -> ureq::Agent {
    let connect = Some(Duration::from_millis(CONNECT_MS));
    ureq::Agent::config_builder()
        .timeout_resolve(connect)
        .timeout_connect(connect)
        .timeout_send_request(connect)
        .timeout_recv_response(connect)
        .timeout_global(Some(Duration::from_millis(PHASE_MS + CONNECT_MS)))
        .build()
        .into()
}

pub fn run(context: &TestContext) {
    let agent = agent();
    let mut block = vec![0u8; BLOCK_BYTES];
    let result = latency(context, &agent)
        .and_then(|_| download(context, &agent, &mut block))
        .and_then(|_| upload(context, &agent, &block));
    if let Err(Stop::Failed(failure)) = result {
        if !context.cancelled() {
            // Nothing else to do if nobody listens any more.
            let _ = send(context, SpeedUpdate::Failed(failure));
        }
    }
}

/// The median time to an answer of 5 empty downloads. The first one also opens the connection
/// (DNS, TCP and TLS), which the median leaves out.
fn latency(context: &TestContext, agent: &ureq::Agent) -> Result<(), Stop> {
    let start = Instant::now();
    let url = format!("{DOWN_URL}0");
    let mut samples = Vec::with_capacity(LATENCY_PINGS);
    for ping in 0..LATENCY_PINGS {
        check(context)?;
        if start.elapsed() >= Duration::from_millis(PHASE_MS) {
            return Err(Stop::Failed(SpeedFailure::Timeout));
        }
        let asked = Instant::now();
        let mut response = agent.get(&url).call()?;
        samples.push(millis(asked));
        // Read the (empty) body to the end, so the connection is kept for the next request.
        io::copy(&mut response.body_mut().as_reader(), &mut io::sink())
            .map_err(|e| Stop::Failed(io_failure(&e)))?;
        send(
            context,
            SpeedUpdate::Progress {
                phase: SpeedPhase::Latency,
                value: median(&samples),
                fraction: (ping + 1) as f32 / LATENCY_PINGS as f32,
            },
        )?;
    }
    let value = median(&samples).unwrap_or_default();
    send(
        context,
        SpeedUpdate::Result {
            phase: SpeedPhase::Latency,
            value,
        },
    )
}

/// Reads up to 25 MB for up to 10 s, from the first byte of the answer, and drops the bytes.
fn download(context: &TestContext, agent: &ureq::Agent, block: &mut [u8]) -> Result<(), Stop> {
    check(context)?;
    let mut response = agent
        .get(format!("{DOWN_URL}{DOWNLOAD_MAX_BYTES}"))
        .call()?;
    let mut reader = response.body_mut().as_reader();
    let start = Instant::now();
    let mut last = start;
    let mut bytes = 0u64;
    let phase = Duration::from_millis(PHASE_MS);
    let progress = Duration::from_millis(PROGRESS_MS);
    loop {
        check(context)?;
        let read = match reader.read(block) {
            Ok(0) => break,
            Ok(read) => read,
            Err(e) if context.cancelled() => {
                log::info!("speed test cancelled: {e}");
                return Err(Stop::Cancelled);
            }
            Err(e) => {
                log::info!("speed test download: {e}");
                return Err(Stop::Failed(io_failure(&e)));
            }
        };
        bytes += read as u64;
        let elapsed = start.elapsed();
        if bytes >= DOWNLOAD_MAX_BYTES || elapsed >= phase {
            break;
        }
        if last.elapsed() >= progress {
            last = Instant::now();
            let fraction = (bytes as f32 / DOWNLOAD_MAX_BYTES as f32)
                .max(elapsed.as_secs_f32() / phase.as_secs_f32());
            send(
                context,
                SpeedUpdate::Progress {
                    phase: SpeedPhase::Download,
                    value: Some(mbps(bytes, millis(start))),
                    fraction,
                },
            )?;
        }
    }
    let value = mbps(bytes, millis(start));
    // Dropping the reader before its end closes the connection.
    drop(reader);
    send(
        context,
        SpeedUpdate::Result {
            phase: SpeedPhase::Download,
            value,
        },
    )
}

/// The upload body: the 64 KB block over and over, until 10 MB or 10 s, with the progress on the
/// way. Sent chunked, since where it stops is known only at the end.
struct Source<'a> {
    context: &'a TestContext,
    block: &'a [u8],
    sent: u64,
    start: Option<Instant>,
    last: Option<Instant>,
    /// When the last byte was handed to the connection.
    end: Option<Instant>,
}

impl Read for Source<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.context.cancelled() {
            return Err(io::Error::other("cancelled"));
        }
        let now = Instant::now();
        let start = *self.start.get_or_insert(now);
        let elapsed = now - start;
        if self.sent >= UPLOAD_MAX_BYTES || elapsed >= Duration::from_millis(PHASE_MS) {
            self.end.get_or_insert(now);
            return Ok(0);
        }
        if self
            .last
            .is_none_or(|last| now - last >= Duration::from_millis(PROGRESS_MS))
        {
            self.last = Some(now);
            let fraction = (self.sent as f32 / UPLOAD_MAX_BYTES as f32)
                .max(elapsed.as_secs_f32() * 1000.0 / PHASE_MS as f32);
            let value = (self.sent > 0).then(|| mbps(self.sent, millis(start)));
            let update = SpeedUpdate::Progress {
                phase: SpeedPhase::Upload,
                value,
                fraction,
            };
            if !self
                .context
                .send(ToolEvent::Test(TestUpdate::Speed(update)))
            {
                return Err(io::Error::other("cancelled"));
            }
        }
        let left = (UPLOAD_MAX_BYTES - self.sent).min(self.block.len() as u64) as usize;
        let count = left.min(buf.len());
        buf[..count].copy_from_slice(&self.block[..count]);
        self.sent += count as u64;
        Ok(count)
    }
}

fn upload(context: &TestContext, agent: &ureq::Agent, block: &[u8]) -> Result<(), Stop> {
    check(context)?;
    let mut source = Source {
        context,
        block,
        sent: 0,
        start: None,
        last: None,
        end: None,
    };
    let result = agent
        .post(UP_URL)
        .header("Content-Type", "application/octet-stream")
        .send(ureq::SendBody::from_reader(&mut source));
    check(context)?;
    result?;
    let (Some(start), Some(end)) = (source.start, source.end) else {
        return Err(Stop::Failed(SpeedFailure::Server));
    };
    let value = mbps(source.sent, (end - start).as_secs_f32() * 1000.0);
    send(
        context,
        SpeedUpdate::Result {
            phase: SpeedPhase::Upload,
            value,
        },
    )
}
