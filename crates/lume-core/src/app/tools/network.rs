//! The Network tool (`network-diagnostics`): how the TV is connected (type, address, gateway), the
//! Wi-Fi signal and an internet speed test that runs only when the user asks.
//!
//! The platform reads the connection every 2 s while the tool is on screen ([`NetworkReading`],
//! D4 and D5 in the `ferramentas` design) and runs the speed test on its `lume-test` thread
//! ([`SpeedUpdate`], D3). This file holds the state machine, the drawing and the pure parts the
//! platform uses (the `/proc/net/route` parser, the limits of the test, the median).

use super::common::{
    draw_panel, panel_bar, panel_heading, panel_rect, panel_row, wrapped_text, COLUMN_WIDTH,
    PANEL_PAD_TOP, PANEL_ROW_HEIGHT, PANEL_ROW_PITCH,
};
use super::{TestKind, ToolEvent, ToolKey, ToolKind, ToolReading, ToolState};
use crate::app::page::LEFT;
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp};
use crate::device_info::Link;
use crate::i18n::{StringId, Translator};
use crate::render::Rect;
use crate::theme::{vars, StateFlags, THEME};
use std::net::Ipv4Addr;

/// How often the connection is read while the tool is on screen.
pub const READING_MS: u64 = 2000;

/// The limits of the speed test (D3): the download stops at 25 MB or 10 s, the upload at 10 MB or
/// 10 s, whichever comes first.
pub const DOWNLOAD_MAX_BYTES: u64 = 25_000_000;
pub const UPLOAD_MAX_BYTES: u64 = 10_000_000;
pub const PHASE_MS: u64 = 10_000;
/// How long a connection may take to open (or a response to start) before the test fails.
pub const CONNECT_MS: u64 = 5_000;
/// The latency is the median of this many small requests.
pub const LATENCY_PINGS: usize = 5;
/// The buffer the download reads into and the upload sends from (reused, never grown).
pub const BLOCK_BYTES: usize = 64 * 1024;
/// The least time between two progress updates (the screen shows at most 4 per second).
pub const PROGRESS_MS: u64 = 250;

/// The phases of the speed test, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpeedPhase {
    /// The median time of [`LATENCY_PINGS`] empty requests, in ms.
    Latency,
    /// Mbps received.
    Download,
    /// Mbps sent.
    Upload,
}

impl SpeedPhase {
    pub const ALL: [SpeedPhase; 3] = [
        SpeedPhase::Latency,
        SpeedPhase::Download,
        SpeedPhase::Upload,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// Why the speed test stopped without a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpeedFailure {
    /// The service could not be reached: no internet, or the name does not resolve.
    NoInternet,
    /// The service answered with an error, or the secure connection failed.
    Server,
    /// Nothing came back in time.
    Timeout,
}

/// Progress or result of the speed test, sent by the platform's `lume-test` thread.
#[derive(Clone, Debug, PartialEq)]
pub enum SpeedUpdate {
    /// This platform cannot run the test (the desktop viewer).
    Unavailable,
    /// A phase is running: its partial value (ms or Mbps), when there is one yet, and how far it
    /// is (0 to 1, by bytes or by time).
    Progress {
        phase: SpeedPhase,
        value: Option<f32>,
        fraction: f32,
    },
    /// The final value of a phase. The upload's ends the test.
    Result { phase: SpeedPhase, value: f32 },
    /// The test stopped; it can be started again.
    Failed(SpeedFailure),
}

/// The Wi-Fi connection as the bridge's `wifi-signal` command tells it (D4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WifiSignal {
    /// Signal strength, in dBm.
    pub rssi: i32,
    /// Link speed, in Mbps, when known.
    pub link_mbps: Option<u32>,
    /// Channel frequency, in MHz, when known.
    pub freq_mhz: Option<u32>,
}

/// A periodic reading of the connection, made by the platform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkReading {
    /// How the TV reaches the network and its IPv4 address (cable preferred, as `device-stats`);
    /// `None` without any interface with an IPv4 address.
    pub connection: Option<(Link, Ipv4Addr)>,
    /// The default gateway; `None` when the system does not expose it.
    pub gateway: Option<Ipv4Addr>,
    /// The Wi-Fi signal; only shown when the connection is Wi-Fi.
    pub wifi: Option<WifiSignal>,
}

/// The default gateway in the text of `/proc/net/route` (D5): the gateway of the up route to
/// 0.0.0.0/0 with the lowest metric. The addresses are hex in the kernel's byte order, which is
/// little-endian on every Android ABI lume ships.
pub fn parse_default_gateway(route: &str) -> Option<Ipv4Addr> {
    const RTF_UP: u32 = 0x1;
    const RTF_GATEWAY: u32 = 0x2;
    let hex = |field: &str| u32::from_str_radix(field, 16).ok();
    route
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 8 {
                return None;
            }
            let destination = hex(fields[1])?;
            let gateway = hex(fields[2])?;
            let flags = hex(fields[3])?;
            let metric: u32 = fields[6].parse().ok()?;
            let mask = hex(fields[7])?;
            let default = destination == 0 && mask == 0;
            let usable = flags & RTF_UP != 0 && flags & RTF_GATEWAY != 0 && gateway != 0;
            (default && usable).then_some((metric, Ipv4Addr::from(gateway.to_le_bytes())))
        })
        .min_by_key(|(metric, _)| *metric)
        .map(|(_, gateway)| gateway)
}

/// A default route (0.0.0.0/0 through a gateway) from the kernel's route dump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefaultRoute {
    pub gateway: Ipv4Addr,
    /// The index of the interface the route goes out of (`RTA_OIF`), 0 when not given.
    pub interface: u32,
    /// The route's metric (`RTA_PRIORITY`): lower wins.
    pub metric: u32,
}

/// The IPv4 default routes in the answer to a netlink `RTM_GETROUTE` dump, from every routing
/// table. Android keeps the default route of each network in a table of its own (for example
/// `default via 192.168.0.1 dev eth0 table 1003`), which `/proc/net/route` (the main table only)
/// never shows. Integers are in the host's byte order, the address in network order.
pub fn parse_route_dump(buf: &[u8]) -> Vec<DefaultRoute> {
    const NLMSG_HEADER: usize = 16;
    const RTMSG: usize = 12;
    const RTM_NEWROUTE: u16 = 24;
    const AF_INET: u8 = 2;
    const RTN_UNICAST: u8 = 1;
    const RTA_OIF: u16 = 4;
    const RTA_GATEWAY: u16 = 5;
    const RTA_PRIORITY: u16 = 6;
    let align = |n: usize| (n + 3) & !3;
    let u16_at = |b: &[u8], i: usize| u16::from_ne_bytes([b[i], b[i + 1]]);
    let u32_at = |b: &[u8], i: usize| u32::from_ne_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let mut routes = Vec::new();
    let mut at = 0;
    while at + NLMSG_HEADER <= buf.len() {
        let len = u32_at(buf, at) as usize;
        if len < NLMSG_HEADER || at + len > buf.len() {
            break;
        }
        let kind = u16_at(buf, at + 4);
        let message = &buf[at..at + len];
        at += align(len);
        if kind != RTM_NEWROUTE || message.len() < NLMSG_HEADER + RTMSG {
            continue;
        }
        let rt = &message[NLMSG_HEADER..];
        let (family, dst_len, route_type) = (rt[0], rt[1], rt[7]);
        if family != AF_INET || dst_len != 0 || route_type != RTN_UNICAST {
            continue;
        }
        let (mut gateway, mut interface, mut metric) = (None, 0, 0);
        let mut attr = NLMSG_HEADER + RTMSG;
        while attr + 4 <= message.len() {
            let attr_len = u16_at(message, attr) as usize;
            if attr_len < 4 || attr + attr_len > message.len() {
                break;
            }
            let data = &message[attr + 4..attr + attr_len];
            match (u16_at(message, attr + 2), data.len()) {
                (RTA_GATEWAY, 4) => {
                    gateway = Some(Ipv4Addr::new(data[0], data[1], data[2], data[3]))
                }
                (RTA_OIF, 4) => interface = u32_at(data, 0),
                (RTA_PRIORITY, 4) => metric = u32_at(data, 0),
                _ => {}
            }
            attr += align(attr_len);
        }
        if let Some(gateway) = gateway.filter(|g| !g.is_unspecified()) {
            routes.push(DefaultRoute {
                gateway,
                interface,
                metric,
            });
        }
    }
    routes
}

/// The gateway of the default route that goes out of an interface `prefer` accepts (the one the
/// connection uses), else of any default route, the lowest metric first.
pub fn pick_gateway(routes: &[DefaultRoute], prefer: impl Fn(u32) -> bool) -> Option<Ipv4Addr> {
    let best = |list: &mut dyn Iterator<Item = &DefaultRoute>| {
        list.min_by_key(|r| r.metric).map(|r| r.gateway)
    };
    best(&mut routes.iter().filter(|r| prefer(r.interface))).or_else(|| best(&mut routes.iter()))
}

/// How good a Wi-Fi signal is, by the ranges of the spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignalQuality {
    /// -55 dBm or more.
    Excellent,
    /// -56 to -67 dBm.
    Good,
    /// -68 to -75 dBm.
    Fair,
    /// Below -75 dBm.
    Weak,
}

impl SignalQuality {
    pub fn of(rssi: i32) -> SignalQuality {
        match rssi {
            -55.. => SignalQuality::Excellent,
            -67..=-56 => SignalQuality::Good,
            -75..=-68 => SignalQuality::Fair,
            _ => SignalQuality::Weak,
        }
    }

    fn name(self) -> StringId {
        match self {
            SignalQuality::Excellent => StringId::ToolNetworkExcellent,
            SignalQuality::Good => StringId::ToolNetworkGood,
            SignalQuality::Fair => StringId::ToolNetworkFair,
            SignalQuality::Weak => StringId::ToolNetworkWeak,
        }
    }
}

/// How full the signal bar is: -90 dBm (or less) empty, -30 dBm (or more) full.
pub fn signal_fraction(rssi: i32) -> f32 {
    ((rssi as f32 + 90.0) / 60.0).clamp(0.0, 1.0)
}

/// The Wi-Fi band of a channel frequency.
pub fn band_label(freq_mhz: u32, comma: bool) -> Option<String> {
    let band = match freq_mhz {
        2400..=2500 => "2.4",
        4900..=5900 => "5",
        5925..=7125 => "6",
        _ => return None,
    };
    let band = if comma {
        band.replace('.', ",")
    } else {
        band.to_string()
    };
    Some(format!("{band} GHz"))
}

/// The median of the latency samples (the mean of the two middle ones for an even count).
pub fn median(samples: &[f32]) -> Option<f32> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f32::total_cmp);
    let middle = sorted.len() / 2;
    Some(if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    })
}

/// Megabits per second for `bytes` moved in `ms`.
pub fn mbps(bytes: u64, ms: f32) -> f32 {
    if ms <= 0.0 {
        return 0.0;
    }
    bytes as f32 * 8.0 / (ms * 1000.0)
}

/// A speed for the screen: one decimal below 10 Mbps, none above.
fn format_mbps(value: f32, comma: bool) -> String {
    let number = if value < 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    };
    let number = if comma {
        number.replace('.', ",")
    } else {
        number
    };
    format!("{number} Mbps")
}

fn format_value(phase: SpeedPhase, value: f32, comma: bool) -> String {
    match phase {
        SpeedPhase::Latency => format!("{value:.0} ms"),
        _ => format_mbps(value, comma),
    }
}

/// Where the speed test is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpeedState {
    /// Never started since the tool opened.
    Idle,
    /// A phase is running, with how far it is.
    Running {
        phase: SpeedPhase,
        fraction: f32,
    },
    /// All three values are final.
    Done,
    /// Back (or OK on "Cancelar teste") stopped it.
    Cancelled,
    Failed(SpeedFailure),
    /// The platform cannot run it.
    Unavailable,
    /// OK was pressed without any connection: nothing was started.
    NoConnection,
}

/// The tool's state, alive only while it is open.
#[derive(Debug)]
pub struct NetworkTool {
    /// The last reading, `None` until the first one arrives.
    reading: Option<NetworkReading>,
    /// When the last reading arrived (`None`: one is due now).
    read_at: Option<u64>,
    speed: SpeedState,
    /// The final value of each phase, by [`SpeedPhase::index`].
    results: [Option<f32>; 3],
    /// The partial value of the running phase.
    partial: Option<f32>,
}

impl NetworkTool {
    pub(in crate::app) fn new(_now_ms: u64) -> NetworkTool {
        NetworkTool {
            reading: None,
            read_at: None,
            speed: SpeedState::Idle,
            results: [None; 3],
            partial: None,
        }
    }

    pub fn speed(&self) -> SpeedState {
        self.speed
    }

    fn running(&self) -> bool {
        matches!(self.speed, SpeedState::Running { .. })
    }

    /// Whether the last reading says there is no connection at all (before the first reading the
    /// test may start: the platform tells if it cannot run).
    fn offline(&self) -> bool {
        self.reading
            .as_ref()
            .is_some_and(|r| r.connection.is_none())
    }

    /// OK on the test item: starts the test, or cancels the running one.
    fn toggle(&mut self) -> Vec<Effect> {
        if self.running() {
            return self.cancel();
        }
        if self.offline() {
            self.speed = SpeedState::NoConnection;
            return vec![];
        }
        self.speed = SpeedState::Running {
            phase: SpeedPhase::Latency,
            fraction: 0.0,
        };
        self.results = [None; 3];
        self.partial = None;
        vec![Effect::StartTest(TestKind::Speed)]
    }

    /// Stops the running test. The phases already measured keep their values; the partial value
    /// of the interrupted phase never becomes final.
    fn cancel(&mut self) -> Vec<Effect> {
        self.speed = SpeedState::Cancelled;
        self.partial = None;
        vec![Effect::CancelTest]
    }

    /// Applies the test's progress. Updates of a test that is no longer running (sent before the
    /// thread noticed the cancellation) are dropped.
    fn update(&mut self, update: SpeedUpdate) {
        if !self.running() {
            return;
        }
        match update {
            SpeedUpdate::Unavailable => self.speed = SpeedState::Unavailable,
            SpeedUpdate::Progress {
                phase,
                value,
                fraction,
            } => {
                self.speed = SpeedState::Running {
                    phase,
                    fraction: fraction.clamp(0.0, 1.0),
                };
                self.partial = value;
            }
            SpeedUpdate::Result { phase, value } => {
                self.results[phase.index()] = Some(value);
                self.partial = None;
                self.speed = match phase {
                    SpeedPhase::Latency => SpeedState::Running {
                        phase: SpeedPhase::Download,
                        fraction: 0.0,
                    },
                    SpeedPhase::Download => SpeedState::Running {
                        phase: SpeedPhase::Upload,
                        fraction: 0.0,
                    },
                    SpeedPhase::Upload => SpeedState::Done,
                };
            }
            SpeedUpdate::Failed(failure) => {
                self.speed = SpeedState::Failed(failure);
                self.partial = None;
            }
        }
    }

    /// The value shown for a phase: final, partial while it runs, or a dash.
    fn shown(&self, phase: SpeedPhase) -> Option<f32> {
        match self.speed {
            SpeedState::Running { phase: running, .. } if running == phase => self.partial,
            _ => self.results[phase.index()],
        }
    }

    /// The line under the notice: the phase running or how the test ended.
    fn status(&self) -> Option<StringId> {
        Some(match self.speed {
            SpeedState::Idle => return None,
            SpeedState::Running { phase, .. } => match phase {
                SpeedPhase::Latency => StringId::ToolNetworkMeasuringLatency,
                SpeedPhase::Download => StringId::ToolNetworkMeasuringDownload,
                SpeedPhase::Upload => StringId::ToolNetworkMeasuringUpload,
            },
            SpeedState::Done => StringId::ToolNetworkDone,
            SpeedState::Cancelled => StringId::ToolNetworkCancelled,
            SpeedState::Failed(SpeedFailure::NoInternet) => StringId::ToolNetworkFailedOffline,
            SpeedState::Failed(SpeedFailure::Server) => StringId::ToolNetworkFailedServer,
            SpeedState::Failed(SpeedFailure::Timeout) => StringId::ToolNetworkFailedTimeout,
            SpeedState::Unavailable => StringId::ToolNetworkUnavailable,
            SpeedState::NoConnection => StringId::ToolNetworkNeedsConnection,
        })
    }
}

/// The left column: the notice starts this far below the item, the status this far below the
/// notice, and the progress bar this far below the status.
const NOTICE_GAP: f32 = 24.0;
const TEXT_PITCH: f32 = 24.0;
const STATUS_GAP: f32 = 24.0;
const PROGRESS_GAP: f32 = 36.0;
const BAR_HEIGHT: f32 = 8.0;
/// The panel: the first row of a section sits this far below its heading, the next heading this
/// far below the last row, and the signal bar this far below the signal row.
const HEADING_TO_ROW: f32 = 40.0;
const SECTION_GAP: f32 = 64.0;
const SIGNAL_BAR_GAP: f32 = 32.0;
const SIGNAL_LINK_GAP: f32 = 56.0;

/// The panel's rows, from the top of its inner area, with or without the Wi-Fi section.
struct PanelLayout {
    /// The connection rows (type, address, gateway).
    connection: [f32; 3],
    /// The signal row, its bar and the link row.
    wifi: Option<(f32, f32, f32)>,
    speed_heading: f32,
    speed: [f32; 3],
    /// The panel's full height.
    height: f32,
}

impl PanelLayout {
    fn new(wifi: bool) -> PanelLayout {
        let rows = |top: f32| [top, top + PANEL_ROW_PITCH, top + 2.0 * PANEL_ROW_PITCH];
        let connection = rows(HEADING_TO_ROW);
        let mut last = connection[2];
        let wifi = wifi.then(|| {
            let signal = last + PANEL_ROW_PITCH;
            let link = signal + SIGNAL_LINK_GAP;
            last = link;
            (signal, signal + SIGNAL_BAR_GAP, link)
        });
        let speed_heading = last + SECTION_GAP;
        let speed = rows(speed_heading + HEADING_TO_ROW);
        let height = speed[2] + PANEL_ROW_HEIGHT + 2.0 * PANEL_PAD_TOP;
        PanelLayout {
            connection,
            wifi,
            speed_heading,
            speed,
            height,
        }
    }
}

impl LumeApp {
    /// Opening the tool (its state was just created): effects for the platform. The first reading
    /// is due at once ([`LumeApp::network_reading_deadline`]); nothing touches the network.
    pub(in crate::app) fn network_open(&mut self) -> Vec<Effect> {
        vec![]
    }

    fn network_tool(&self) -> Option<&NetworkTool> {
        match self.tool.as_deref() {
            Some(ToolState::Network(tool)) => Some(tool),
            _ => None,
        }
    }

    fn network_tool_mut(&mut self) -> Option<&mut NetworkTool> {
        match self.tool.as_deref_mut() {
            Some(ToolState::Network(tool)) => Some(tool),
            _ => None,
        }
    }

    pub(in crate::app) fn draw_network(&mut self, p: &mut Painter) {
        let t = self.translator;
        let Some(tool) = self.network_tool() else {
            return;
        };
        let label = if tool.running() {
            StringId::ToolNetworkCancel
        } else {
            StringId::ToolNetworkTest
        };
        let id = ToolKind::Network.first_id();
        let rects = self.draw_tool_items(p, &[(id, t.get(label).into_owned())], 0.0);
        let Some(tool) = self.network_tool() else {
            return;
        };
        draw_notice_and_status(p, &t, tool, rects[0]);
        draw_network_panel(p, &t, tool);
        self.draw_tool_header(p, ToolKind::Network);
    }

    pub(in crate::app) fn network_key(&mut self, key: Key) -> ToolKey {
        let focused = self.focused() == Some(ToolKind::Network.first_id());
        let Some(tool) = self.network_tool_mut() else {
            return ToolKey::Default;
        };
        match key {
            Key::Ok if focused => ToolKey::Handled(tool.toggle()),
            // Back first stops a running test; the next one closes the tool.
            Key::Back if tool.running() => ToolKey::Handled(tool.cancel()),
            _ => ToolKey::Default,
        }
    }

    /// The screen changes only with the platform's events (readings and progress).
    pub(in crate::app) fn network_deadline(&self, _now_ms: u64) -> Option<u64> {
        None
    }

    pub(in crate::app) fn network_fullscreen(&self) -> bool {
        false
    }

    pub(in crate::app) fn network_event(&mut self, event: ToolEvent) {
        let now = self.now_ms;
        let Some(tool) = self.network_tool_mut() else {
            return;
        };
        match event {
            ToolEvent::Reading(ToolReading::Network(reading)) => {
                tool.reading = Some(reading);
                tool.read_at = Some(now);
            }
            ToolEvent::Test(super::TestUpdate::Speed(update)) => tool.update(update),
            _ => {}
        }
    }

    /// When the platform must read the connection again: at once after opening, then every 2 s
    /// with the tool on screen (the shell waits for the interface at rest).
    pub(in crate::app) fn network_reading_deadline(&self, now_ms: u64) -> Option<u64> {
        let tool = self.network_tool()?;
        Some(tool.read_at.map_or(now_ms, |at| at + READING_MS))
    }

    /// The tool's state, for tests.
    #[cfg(test)]
    fn network_state(&self) -> &NetworkTool {
        self.network_tool().expect("the network tool is open")
    }
}

/// Whether numbers use a decimal comma (every language lume ships but English).
fn decimal_comma(t: &Translator) -> bool {
    !t.language().starts_with("en")
}

/// The external service notice under the test item, then the status line and, while a phase
/// runs, its progress bar.
fn draw_notice_and_status(p: &mut Painter, t: &Translator, tool: &NetworkTool, item: Rect) {
    let muted = p.style(&THEME.breadcrumb, StateFlags::NONE);
    let top = item.bottom() + NOTICE_GAP;
    let notice = Rect::new(LEFT, top, COLUMN_WIDTH, TEXT_PITCH);
    let lines = wrapped_text(
        p,
        &t.get(StringId::ToolNetworkNotice),
        notice,
        TEXT_PITCH,
        &muted,
    );
    let Some(status) = tool.status() else {
        return;
    };
    let mut strong = muted;
    strong.color = vars::TEXT;
    strong.font_weight = 700;
    let status_top = top + lines as f32 * TEXT_PITCH + STATUS_GAP;
    let lines = wrapped_text(
        p,
        &t.get(status),
        Rect::new(LEFT, status_top, COLUMN_WIDTH, TEXT_PITCH),
        TEXT_PITCH,
        &strong,
    );
    if let SpeedState::Running { fraction, .. } = tool.speed {
        let bar_top = status_top + (lines.max(1) - 1) as f32 * TEXT_PITCH + PROGRESS_GAP;
        panel_bar(
            p,
            Rect::new(LEFT, bar_top, COLUMN_WIDTH, BAR_HEIGHT),
            Some(fraction),
        );
    }
}

/// A label and its value, as one row of the panel shows them.
type PanelRow = (String, String);

/// What the panel says, apart from where: the connection rows, the Wi-Fi rows (signal and link,
/// only on Wi-Fi) with the signal bar, and the test's rows.
struct PanelTexts {
    connection: [PanelRow; 3],
    wifi: Option<([PanelRow; 2], Option<f32>)>,
    speed: [PanelRow; 3],
}

fn panel_texts(t: &Translator, tool: &NetworkTool) -> PanelTexts {
    let comma = decimal_comma(t);
    let dash = || "—".to_string();
    let label = |id: StringId| t.get(id).into_owned();
    let reading = tool.reading.as_ref();
    let connection = reading.and_then(|r| r.connection);
    let link = match (reading, connection) {
        (None, _) => dash(),
        (Some(_), None) => label(StringId::ToolNetworkNoConnection),
        (Some(_), Some((link, _))) => label(match link {
            Link::WiFi => StringId::TvLinkWifi,
            Link::Ethernet => StringId::TvLinkEthernet,
            Link::Other => StringId::TvLinkOther,
        }),
    };
    let address = connection.map_or_else(dash, |(_, ip)| ip.to_string());
    let gateway = reading
        .and_then(|r| r.gateway)
        .map_or_else(dash, |g| g.to_string());

    let wifi = matches!(connection, Some((Link::WiFi, _))).then(|| {
        let signal = reading.and_then(|r| r.wifi);
        let strength = signal.map_or_else(dash, |s| {
            format!(
                "{} dBm · {}",
                s.rssi,
                t.get(SignalQuality::of(s.rssi).name())
            )
        });
        let speed = signal
            .and_then(|s| s.link_mbps)
            .map(|m| format!("{m} Mbps"));
        let band = signal
            .and_then(|s| s.freq_mhz)
            .and_then(|f| band_label(f, comma));
        let link = match (speed, band) {
            (Some(speed), Some(band)) => format!("{speed} · {band}"),
            (Some(one), None) | (None, Some(one)) => one,
            (None, None) => dash(),
        };
        (
            [
                (label(StringId::ToolNetworkSignal), strength),
                (label(StringId::ToolNetworkLink), link),
            ],
            signal.map(|s| signal_fraction(s.rssi)),
        )
    });

    let value = |phase: SpeedPhase| {
        tool.shown(phase)
            .map_or_else(dash, |v| format_value(phase, v, comma))
    };
    PanelTexts {
        connection: [
            (label(StringId::TvConnection), link),
            (label(StringId::TvIp), address),
            (label(StringId::ToolNetworkGateway), gateway),
        ],
        wifi,
        speed: [
            (
                label(StringId::ToolNetworkLatency),
                value(SpeedPhase::Latency),
            ),
            (
                label(StringId::ToolNetworkDownload),
                value(SpeedPhase::Download),
            ),
            (
                label(StringId::ToolNetworkUpload),
                value(SpeedPhase::Upload),
            ),
        ],
    }
}

/// The panel at the right: the connection, the Wi-Fi signal (only on Wi-Fi) and the test's values.
fn draw_network_panel(p: &mut Painter, t: &Translator, tool: &NetworkTool) {
    let texts = panel_texts(t, tool);
    let layout = PanelLayout::new(texts.wifi.is_some());
    let inner = draw_panel(p, panel_rect(layout.height));
    let line = |y: f32| Rect::new(inner.x, inner.y + y, inner.w, PANEL_ROW_HEIGHT);

    panel_heading(p, inner, &t.get(StringId::ToolNetworkSectionConnection));
    for ((label, value), y) in texts.connection.iter().zip(layout.connection) {
        panel_row(p, line(y), label, value);
    }
    if let (Some((rows, fraction)), Some((signal_y, bar_y, link_y))) = (&texts.wifi, layout.wifi) {
        panel_row(p, line(signal_y), &rows[0].0, &rows[0].1);
        let bar = Rect::new(inner.x, inner.y + bar_y, inner.w, BAR_HEIGHT);
        panel_bar(p, bar, *fraction);
        panel_row(p, line(link_y), &rows[1].0, &rows[1].1);
    }
    let heading = Rect::new(inner.x, inner.y + layout.speed_heading, inner.w, 20.0);
    panel_heading(p, heading, &t.get(StringId::ToolNetworkSectionSpeed));
    for ((label, value), y) in texts.speed.iter().zip(layout.speed) {
        panel_row(p, line(y), label, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};
    use crate::app::tools::TestUpdate;

    fn speed(update: SpeedUpdate) -> ToolEvent {
        ToolEvent::Test(TestUpdate::Speed(update))
    }

    fn reading(connection: Option<(Link, Ipv4Addr)>) -> ToolEvent {
        ToolEvent::Reading(ToolReading::Network(NetworkReading {
            connection,
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
            wifi: Some(WifiSignal {
                rssi: -60,
                link_mbps: Some(300),
                freq_mhz: Some(5180),
            }),
        }))
    }

    fn wifi() -> Option<(Link, Ipv4Addr)> {
        Some((Link::WiFi, Ipv4Addr::new(192, 168, 1, 20)))
    }

    /// Everything the screen says (item, status and panel), joined, after drawing a frame.
    fn frame_text(rig: &mut Rig) -> String {
        rig.frame();
        let t = rig.app.translator;
        let tool = rig.app.network_state();
        let texts = panel_texts(&t, tool);
        let mut all = vec![t
            .get(if tool.running() {
                StringId::ToolNetworkCancel
            } else {
                StringId::ToolNetworkTest
            })
            .into_owned()];
        all.extend(tool.status().map(|s| t.get(s).into_owned()));
        let rows = texts
            .connection
            .iter()
            .chain(texts.wifi.iter().flat_map(|(rows, _)| rows.iter()))
            .chain(texts.speed.iter());
        all.extend(rows.map(|(label, value)| format!("{label}: {value}")));
        all.join("\n")
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        let with = PanelLayout::new(true);
        let without = PanelLayout::new(false);
        crate::app::assert_on_ruler(&[
            ("NOTICE_GAP", NOTICE_GAP),
            ("TEXT_PITCH", TEXT_PITCH),
            ("STATUS_GAP", STATUS_GAP),
            ("PROGRESS_GAP", PROGRESS_GAP),
            ("BAR_HEIGHT", BAR_HEIGHT),
            ("HEADING_TO_ROW", HEADING_TO_ROW),
            ("SECTION_GAP", SECTION_GAP),
            ("SIGNAL_BAR_GAP", SIGNAL_BAR_GAP),
            ("SIGNAL_LINK_GAP", SIGNAL_LINK_GAP),
            ("panel height with Wi-Fi", with.height),
            ("panel height without Wi-Fi", without.height),
            ("speed heading with Wi-Fi", with.speed_heading),
        ]);
        // The tallest panel stays inside the safe area of the 720 canvas.
        let bottom = panel_rect(with.height).bottom();
        assert!(bottom <= 720.0 - crate::display::SAFE_MARGIN_Y, "{bottom}");
        let (_, bar, link) = with.wifi.unwrap();
        assert!(
            bar + BAR_HEIGHT < link,
            "the bar sits between the signal and the link"
        );
    }

    #[test]
    fn signal_classes_at_the_limits() {
        assert_eq!(SignalQuality::of(-30), SignalQuality::Excellent);
        assert_eq!(SignalQuality::of(-55), SignalQuality::Excellent);
        assert_eq!(SignalQuality::of(-56), SignalQuality::Good);
        assert_eq!(SignalQuality::of(-67), SignalQuality::Good);
        assert_eq!(SignalQuality::of(-68), SignalQuality::Fair);
        assert_eq!(SignalQuality::of(-75), SignalQuality::Fair);
        assert_eq!(SignalQuality::of(-76), SignalQuality::Weak);
        assert_eq!(SignalQuality::of(-100), SignalQuality::Weak);
        assert_eq!(signal_fraction(-95), 0.0);
        assert_eq!(signal_fraction(-60), 0.5);
        assert_eq!(signal_fraction(-20), 1.0);
    }

    #[test]
    fn bands_by_frequency() {
        assert_eq!(band_label(2437, true).as_deref(), Some("2,4 GHz"));
        assert_eq!(band_label(2437, false).as_deref(), Some("2.4 GHz"));
        assert_eq!(band_label(5180, true).as_deref(), Some("5 GHz"));
        assert_eq!(band_label(5955, true).as_deref(), Some("6 GHz"));
        assert_eq!(band_label(0, true), None);
    }

    /// One netlink message: an `RTM_NEWROUTE` (or `kind`) with an rtmsg and the attributes.
    fn route_message(kind: u16, dst_len: u8, attrs: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut body = vec![2, dst_len, 0, 0, 254, 4, 0, 1, 0, 0, 0, 0];
        for (ty, data) in attrs {
            body.extend(((4 + data.len()) as u16).to_ne_bytes());
            body.extend(ty.to_ne_bytes());
            body.extend(data);
            while body.len() % 4 != 0 {
                body.push(0);
            }
        }
        let mut msg = Vec::new();
        msg.extend(((16 + body.len()) as u32).to_ne_bytes());
        msg.extend(kind.to_ne_bytes());
        msg.extend([0u8; 10]);
        msg.extend(body);
        msg
    }

    #[test]
    fn the_gateway_comes_from_the_route_dump_of_every_table() {
        // As on the TCL: a default route without gateway (dummy0), the cable's in table 1003, a
        // /24 route and the end of the dump.
        let mut dump = route_message(24, 0, &[(4, 9u32.to_ne_bytes().to_vec())]);
        dump.extend(route_message(
            24,
            0,
            &[
                (15, 1003u32.to_ne_bytes().to_vec()),
                (5, vec![192, 168, 0, 1]),
                (4, 3u32.to_ne_bytes().to_vec()),
            ],
        ));
        dump.extend(route_message(24, 24, &[(5, vec![10, 0, 0, 1])]));
        dump.extend(route_message(3, 0, &[]));
        let routes = parse_route_dump(&dump);
        assert_eq!(
            routes,
            vec![DefaultRoute {
                gateway: Ipv4Addr::new(192, 168, 0, 1),
                interface: 3,
                metric: 0
            }]
        );
        assert_eq!(
            pick_gateway(&routes, |_| false),
            Some(Ipv4Addr::new(192, 168, 0, 1))
        );
    }

    #[test]
    fn the_gateway_of_the_connection_interface_wins() {
        let route = |last, interface, metric| DefaultRoute {
            gateway: Ipv4Addr::new(192, 168, 0, last),
            interface,
            metric,
        };
        let routes = [route(1, 3, 100), route(254, 5, 0)];
        assert_eq!(
            pick_gateway(&routes, |i| i == 3),
            Some(Ipv4Addr::new(192, 168, 0, 1))
        );
        assert_eq!(
            pick_gateway(&routes, |_| false),
            Some(Ipv4Addr::new(192, 168, 0, 254))
        );
        assert_eq!(pick_gateway(&[], |_| true), None);
        // A truncated dump is not read past its end.
        assert!(parse_route_dump(&[40, 0, 0, 0, 24, 0]).is_empty());
    }

    #[test]
    fn the_default_gateway_comes_from_the_route_table() {
        let table =
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
            wlan0\t0001A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0\n\
            eth0\t00000000\t0102A8C0\t0003\t0\t0\t20\t00000000\t0\t0\t0\n\
            wlan0\t00000000\t0101A8C0\t0003\t0\t0\t10\t00000000\t0\t0\t0\n";
        assert_eq!(
            parse_default_gateway(table),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "the lowest metric wins"
        );
        // Only the header (Android keeps the default route in other tables), a route that is down
        // or without a gateway, garbage: nothing.
        assert_eq!(parse_default_gateway("Iface\tDestination\tGateway\n"), None);
        assert_eq!(
            parse_default_gateway(
                "h\nwlan0\t00000000\t0101A8C0\t0002\t0\t0\t0\t00000000\t0\t0\t0\n"
            ),
            None
        );
        assert_eq!(
            parse_default_gateway(
                "h\nwlan0\t00000000\t00000000\t0001\t0\t0\t0\t00000000\t0\t0\t0\n"
            ),
            None
        );
        assert_eq!(parse_default_gateway(""), None);
        assert_eq!(parse_default_gateway("h\nx y z\n"), None);
    }

    #[test]
    fn median_and_speed() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[30.0, 10.0, 250.0, 12.0, 11.0]), Some(12.0));
        assert_eq!(median(&[10.0, 20.0]), Some(15.0));
        assert_eq!(mbps(1_250_000, 1000.0), 10.0);
        assert_eq!(mbps(1, 0.0), 0.0);
        assert_eq!(format_mbps(9.44, true), "9,4 Mbps");
        assert_eq!(format_mbps(312.4, true), "312 Mbps");
        assert_eq!(format_value(SpeedPhase::Latency, 23.4, true), "23 ms");
    }

    #[test]
    fn opening_reads_at_once_then_every_two_seconds_and_never_starts_the_test() {
        let mut rig = Rig::new();
        let effects = open_tool(&mut rig, ToolKind::Network);
        assert!(!effects.contains(&Effect::StartTest(TestKind::Speed)));
        let now = rig.now;
        assert_eq!(rig.app.network_reading_deadline(now), Some(now));
        rig.app.tool_event(reading(wifi()), now);
        assert_eq!(
            rig.app.network_reading_deadline(now),
            Some(now + READING_MS)
        );
        rig.advance(5000);
        assert!(!rig.app.network_state().running(), "never automatic");
    }

    #[test]
    fn wifi_shows_the_signal_and_cable_hides_it() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.app.tool_event(reading(wifi()), rig.now);
        let text = frame_text(&mut rig);
        for expected in [
            "Wi-Fi",
            "192.168.1.20",
            "192.168.1.1",
            "-60 dBm · Bom",
            "300 Mbps · 5 GHz",
        ] {
            assert!(text.contains(expected), "{expected} in {text}");
        }
        assert!(!text.contains("SSID"));
        let cable = Some((Link::Ethernet, Ipv4Addr::new(10, 0, 0, 2)));
        rig.app.tool_event(reading(cable), rig.now);
        let text = frame_text(&mut rig);
        assert!(text.contains("Cabo Ethernet"), "{text}");
        assert!(!text.contains("dBm"), "no Wi-Fi section on cable");
    }

    #[test]
    fn without_connection_the_test_does_not_start() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.app.tool_event(reading(None), rig.now);
        let effects = rig.press(Key::Ok);
        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(rig.app.network_state().speed(), SpeedState::NoConnection);
        assert!(frame_text(&mut rig).contains("Sem conexão"));
    }

    #[test]
    fn a_full_test_shows_partial_values_then_the_three_results() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.app.tool_event(reading(wifi()), rig.now);
        let effects = rig.press(Key::Ok);
        assert_eq!(effects, vec![Effect::StartTest(TestKind::Speed)]);
        let now = rig.now;
        rig.app.tool_event(
            speed(SpeedUpdate::Progress {
                phase: SpeedPhase::Latency,
                value: Some(21.0),
                fraction: 0.4,
            }),
            now,
        );
        let text = frame_text(&mut rig);
        assert!(
            text.contains("21 ms") && text.contains("Medindo a latência"),
            "{text}"
        );
        rig.app.tool_event(
            speed(SpeedUpdate::Result {
                phase: SpeedPhase::Latency,
                value: 20.0,
            }),
            now,
        );
        rig.app.tool_event(
            speed(SpeedUpdate::Progress {
                phase: SpeedPhase::Download,
                value: Some(55.5),
                fraction: 0.5,
            }),
            now,
        );
        assert_eq!(
            rig.app.network_state().speed(),
            SpeedState::Running {
                phase: SpeedPhase::Download,
                fraction: 0.5
            }
        );
        let text = frame_text(&mut rig);
        assert!(text.contains("20 ms") && text.contains("56 Mbps"), "{text}");
        for (phase, value) in [(SpeedPhase::Download, 94.0), (SpeedPhase::Upload, 8.25)] {
            rig.app
                .tool_event(speed(SpeedUpdate::Result { phase, value }), now);
        }
        assert_eq!(rig.app.network_state().speed(), SpeedState::Done);
        let text = frame_text(&mut rig);
        for expected in ["20 ms", "94 Mbps", "8,2 Mbps", "Teste concluído"] {
            assert!(text.contains(expected), "{expected} in {text}");
        }
    }

    #[test]
    fn back_cancels_without_keeping_the_partial_value_then_closes() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.press(Key::Ok);
        let now = rig.now;
        rig.app.tool_event(
            speed(SpeedUpdate::Progress {
                phase: SpeedPhase::Download,
                value: Some(40.0),
                fraction: 0.2,
            }),
            now,
        );
        let effects = rig.press(Key::Back);
        assert_eq!(effects, vec![Effect::CancelTest]);
        assert_eq!(rig.app.network_state().speed(), SpeedState::Cancelled);
        // A late update from the thread changes nothing.
        rig.app.tool_event(
            speed(SpeedUpdate::Result {
                phase: SpeedPhase::Download,
                value: 41.0,
            }),
            rig.now,
        );
        assert_eq!(rig.app.network_state().shown(SpeedPhase::Download), None);
        let text = frame_text(&mut rig);
        assert!(
            !text.contains("40 Mbps") && text.contains("Teste cancelado"),
            "{text}"
        );
        rig.press(Key::Back);
        assert_eq!(rig.app.tool_kind(), None, "the second Back closes the tool");
    }

    #[test]
    fn ok_on_the_running_test_cancels_it() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.press(Key::Ok);
        assert!(frame_text(&mut rig).contains("Cancelar teste"));
        assert_eq!(rig.press(Key::Ok), vec![Effect::CancelTest]);
        assert_eq!(rig.app.network_state().speed(), SpeedState::Cancelled);
    }

    #[test]
    fn a_failure_shows_a_message_and_the_test_can_be_repeated() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.press(Key::Ok);
        rig.app.tool_event(
            speed(SpeedUpdate::Result {
                phase: SpeedPhase::Latency,
                value: 30.0,
            }),
            rig.now,
        );
        rig.app.tool_event(
            speed(SpeedUpdate::Failed(SpeedFailure::NoInternet)),
            rig.now,
        );
        assert_eq!(
            rig.app.network_state().speed(),
            SpeedState::Failed(SpeedFailure::NoInternet)
        );
        assert!(frame_text(&mut rig).contains("Sem acesso à internet"));
        // Repeating clears the previous values.
        assert_eq!(rig.press(Key::Ok), vec![Effect::StartTest(TestKind::Speed)]);
        assert!(rig.app.network_state().running());
        assert_eq!(rig.app.network_state().shown(SpeedPhase::Latency), None);
    }

    #[test]
    fn the_desktop_answers_unavailable() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.press(Key::Ok);
        rig.app.tool_event(speed(SpeedUpdate::Unavailable), rig.now);
        assert_eq!(rig.app.network_state().speed(), SpeedState::Unavailable);
        assert!(frame_text(&mut rig).contains("indisponível"));
    }
}
