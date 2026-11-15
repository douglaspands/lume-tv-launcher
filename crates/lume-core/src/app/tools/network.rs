//! The Network tool (`network-diagnostics`): how the TV is connected (type, address, gateway) and,
//! on Wi-Fi, the signal. Everything is local: the tool never opens a connection.
//!
//! The platform reads the connection every 2 s while the tool is on screen ([`NetworkReading`],
//! D4 and D5 in the `ferramentas` design). This file holds the drawing and the pure parts the
//! platform uses (the route parsers and the signal classes).

use super::common::{
    draw_panel, panel_bar, panel_heading, panel_row, COLUMN_WIDTH, ITEMS_TOP, PANEL_PAD_TOP,
    PANEL_ROW_HEIGHT, PANEL_ROW_PITCH, PANEL_WIDTH, PANEL_X,
};
use super::{ToolEvent, ToolKey, ToolKind, ToolReading, ToolState};
use crate::app::page::LEFT;
use crate::app::painter::Painter;
use crate::app::{Effect, Key, LumeApp};
use crate::device_info::Link;
use crate::i18n::{StringId, Translator};
use crate::render::Rect;
use std::net::Ipv4Addr;

/// How often the connection is read while the tool is on screen.
pub const READING_MS: u64 = 2000;

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

/// The tool's state, alive only while it is open.
#[derive(Debug, Default)]
pub struct NetworkTool {
    /// The last reading, `None` until the first one arrives.
    reading: Option<NetworkReading>,
    /// When the last reading arrived (`None`: one is due now).
    read_at: Option<u64>,
}

impl NetworkTool {
    pub(in crate::app) fn new(_now_ms: u64) -> NetworkTool {
        NetworkTool::default()
    }
}

/// The panels: the first row sits this far below the heading; the signal bar this far below the
/// signal row, and the link row this far below the signal row.
const HEADING_TO_ROW: f32 = 40.0;
const SIGNAL_BAR_GAP: f32 = 32.0;
const SIGNAL_LINK_GAP: f32 = 56.0;
const BAR_HEIGHT: f32 = 8.0;
/// The connection panel (left column): heading and three rows.
const CONNECTION_HEIGHT: f32 =
    HEADING_TO_ROW + 2.0 * PANEL_ROW_PITCH + PANEL_ROW_HEIGHT + 2.0 * PANEL_PAD_TOP;
/// The Wi-Fi panel (right column): heading, the signal with its bar, and the link.
const WIFI_HEIGHT: f32 = HEADING_TO_ROW + SIGNAL_LINK_GAP + PANEL_ROW_HEIGHT + 2.0 * PANEL_PAD_TOP;

impl LumeApp {
    /// Opening the tool (its state was just created): the first reading is due at once
    /// ([`LumeApp::network_reading_deadline`]); nothing touches the network.
    pub(in crate::app) fn network_open(&mut self) -> Vec<Effect> {
        vec![]
    }

    fn network_tool(&self) -> Option<&NetworkTool> {
        match self.tool.as_deref() {
            Some(ToolState::Network(tool)) => Some(tool),
            _ => None,
        }
    }

    /// The connection in a panel at the left and, on Wi-Fi, the signal in a panel at the right.
    /// The tool has no action, so nothing takes the focus.
    pub(in crate::app) fn draw_network(&mut self, p: &mut Painter) {
        let t = self.translator;
        self.draw_tool_items(p, &[], 0.0);
        let Some(tool) = self.network_tool() else {
            return;
        };
        let texts = panel_texts(&t, tool);
        draw_connection_panel(p, &t, &texts);
        if let Some(wifi) = &texts.wifi {
            draw_wifi_panel(p, &t, wifi);
        }
        self.draw_tool_header(p, ToolKind::Network);
    }

    pub(in crate::app) fn network_key(&mut self, _key: Key) -> ToolKey {
        ToolKey::Default
    }

    /// The screen changes only with the platform's readings.
    pub(in crate::app) fn network_deadline(&self, _now_ms: u64) -> Option<u64> {
        None
    }

    pub(in crate::app) fn network_fullscreen(&self) -> bool {
        false
    }

    pub(in crate::app) fn network_event(&mut self, event: ToolEvent) {
        let now = self.now_ms;
        if let (ToolEvent::Reading(ToolReading::Network(reading)), Some(ToolState::Network(tool))) =
            (event, self.tool.as_deref_mut())
        {
            tool.reading = Some(reading);
            tool.read_at = Some(now);
        }
    }

    /// When the platform must read the connection again: at once after opening, then every 2 s
    /// with the tool on screen (the shell waits for the interface at rest).
    pub(in crate::app) fn network_reading_deadline(&self, now_ms: u64) -> Option<u64> {
        let tool = self.network_tool()?;
        Some(tool.read_at.map_or(now_ms, |at| at + READING_MS))
    }
}

/// Whether numbers use a decimal comma (every language lume ships but English).
fn decimal_comma(t: &Translator) -> bool {
    !t.language().starts_with("en")
}

/// A label and its value, as one row of a panel shows them.
type PanelRow = (String, String);

/// What the panels say: the connection rows, and on Wi-Fi the signal and link rows with how full
/// the signal bar is.
struct PanelTexts {
    connection: [PanelRow; 3],
    wifi: Option<([PanelRow; 2], Option<f32>)>,
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

    PanelTexts {
        connection: [
            (label(StringId::TvConnection), link),
            (label(StringId::TvIp), address),
            (label(StringId::ToolNetworkGateway), gateway),
        ],
        wifi,
    }
}

/// The connection (type, address, gateway) in a panel in the left column.
fn draw_connection_panel(p: &mut Painter, t: &Translator, texts: &PanelTexts) {
    let inner = draw_panel(
        p,
        Rect::new(LEFT, ITEMS_TOP, COLUMN_WIDTH, CONNECTION_HEIGHT),
    );
    panel_heading(p, inner, &t.get(StringId::ToolNetworkSectionConnection));
    for (index, (label, value)) in texts.connection.iter().enumerate() {
        let y = inner.y + HEADING_TO_ROW + index as f32 * PANEL_ROW_PITCH;
        panel_row(
            p,
            Rect::new(inner.x, y, inner.w, PANEL_ROW_HEIGHT),
            label,
            value,
        );
    }
}

/// The Wi-Fi signal (with its bar) and link in a panel in the right column.
fn draw_wifi_panel(p: &mut Painter, t: &Translator, wifi: &([PanelRow; 2], Option<f32>)) {
    let (rows, fraction) = wifi;
    let inner = draw_panel(p, Rect::new(PANEL_X, ITEMS_TOP, PANEL_WIDTH, WIFI_HEIGHT));
    panel_heading(p, inner, &t.get(StringId::TvLinkWifi));
    let line = |y: f32| Rect::new(inner.x, inner.y + y, inner.w, PANEL_ROW_HEIGHT);
    panel_row(p, line(HEADING_TO_ROW), &rows[0].0, &rows[0].1);
    let bar = Rect::new(
        inner.x,
        inner.y + HEADING_TO_ROW + SIGNAL_BAR_GAP,
        inner.w,
        BAR_HEIGHT,
    );
    panel_bar(p, bar, *fraction);
    panel_row(
        p,
        line(HEADING_TO_ROW + SIGNAL_LINK_GAP),
        &rows[1].0,
        &rows[1].1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{open_tool, Rig};

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

    /// Everything the panels say, joined, after drawing a frame.
    fn frame_text(rig: &mut Rig) -> String {
        rig.frame();
        let t = rig.app.translator;
        let texts = panel_texts(&t, rig.app.network_tool().expect("open"));
        texts
            .connection
            .iter()
            .chain(texts.wifi.iter().flat_map(|(rows, _)| rows.iter()))
            .map(|(label, value)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("HEADING_TO_ROW", HEADING_TO_ROW),
            ("SIGNAL_BAR_GAP", SIGNAL_BAR_GAP),
            ("SIGNAL_LINK_GAP", SIGNAL_LINK_GAP),
            ("BAR_HEIGHT", BAR_HEIGHT),
            ("CONNECTION_HEIGHT", CONNECTION_HEIGHT),
            ("WIFI_HEIGHT", WIFI_HEIGHT),
        ]);
        // The bar sits between the signal and the link.
        const { assert!(SIGNAL_BAR_GAP + BAR_HEIGHT < SIGNAL_LINK_GAP) };
    }

    #[test]
    fn opening_reads_at_once_then_every_two_seconds_and_opens_no_connection() {
        let mut rig = Rig::new();
        let effects = open_tool(&mut rig, ToolKind::Network);
        assert!(
            effects.iter().all(|e| matches!(e, Effect::KeepScreenOn(_))),
            "no test, no connection: {effects:?}"
        );
        let now = rig.now;
        assert_eq!(rig.app.network_reading_deadline(now), Some(now));
        rig.app.tool_event(reading(wifi()), now);
        assert_eq!(
            rig.app.network_reading_deadline(now),
            Some(now + READING_MS)
        );
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
        assert!(!text.contains("dBm"), "no Wi-Fi panel on cable");
    }

    #[test]
    fn back_closes_the_tool_at_once() {
        let mut rig = Rig::new();
        open_tool(&mut rig, ToolKind::Network);
        rig.press(Key::Back);
        assert_eq!(rig.app.screen(), &crate::app::Screen::Tools);
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
}
