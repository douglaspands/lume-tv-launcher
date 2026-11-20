//! The phone companion: while the Receive image or the Confirm screen is open, a minimal HTTP server
//! on the local network serves the Lume page to the phone, where the picture is framed, and receives
//! the result from it (QOI of 1920x1080).
//!
//! Only listens, never connects out. The server lives with the two screens: Apply, leaving, the
//! background, 5 wrong codes or the time running out for good switch it off, and then no port of
//! Lume stays open. The time only runs without editing: the page tells the TV, every 30 s while it
//! is visible, how long ago it was last used.

pub mod page;
pub mod server;

use crate::wallpaper::Image;
use server::{Event, Reject, Server, Verdict};
use std::net::Ipv4Addr;
use std::sync::OnceLock;
use std::time::Instant;

/// The port tried first; a busy one makes the system choose (the QR carries the port either way).
pub const PREFERRED_PORT: u16 = 8642;
/// How long the server waits after the last edit (or the opening).
pub const TIMEOUT_MS: u64 = 5 * 60 * 1000;
/// A ping counts as editing when the page was used less than this many seconds before it.
pub const EDIT_IDLE_S: u64 = 120;
/// How long after the last edit the screen says the phone is connected (two pings missed).
pub const CONNECTED_MS: u64 = 75_000;
/// The countdown shows only in the last minute.
pub const COUNTDOWN_MS: u64 = 60_000;
/// New codes made by themselves when the time runs out without editing; then "Time is up".
pub const MAX_RENEWALS: u32 = 3;
/// How often the screen looks for a network while there is none.
pub const NETWORK_RETRY_MS: u64 = 3000;
/// Wrong codes that switch the server off.
pub const MAX_WRONG_CODES: u32 = 5;
/// How long the server stays up after Apply, only to tell the page (its pings come every 2 s then).
pub const APPLIED_LINGER_MS: u64 = 10_000;
/// Characters of the access code.
pub const CODE_LEN: usize = 4;
/// The code's alphabet: no 0/O nor 1/I/L, 31 symbols.
pub const CODE_ALPHABET: &[u8; 31] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/// Lists the interfaces with an IPv4 address (name and address).
pub type InterfaceProvider = fn() -> Vec<(String, Ipv4Addr)>;

static INTERFACES: OnceLock<InterfaceProvider> = OnceLock::new();
static NOTIFIER: OnceLock<fn()> = OnceLock::new();
static LIBRARY: OnceLock<server::LibReader> = OnceLock::new();

/// Sets how the platform lists its interfaces (Android: `SIOCGIFCONF`). Without one there is no network.
pub fn set_interface_provider(provider: InterfaceProvider) {
    let _ = INTERFACES.set(provider);
}

/// Sets what the platform does when the server has news for the screen (Android: wake the loop).
pub fn set_notifier(notifier: fn()) {
    let _ = NOTIFIER.set(notifier);
}

/// Sets how the platform reads the page's shared files (Android: `assets/companion/` of the APK).
/// Without one, desktop builds read the repository's `assets/companion/heic/`.
pub fn set_library_reader(reader: server::LibReader) {
    let _ = LIBRARY.set(reader);
}

fn interfaces() -> Vec<(String, Ipv4Addr)> {
    INTERFACES.get().map(|list| list()).unwrap_or_default()
}

/// The page's shared files: the platform's reader, or the repository on the desktop.
fn read_library(name: &str) -> Option<crate::memory::Buffer> {
    if let Some(reader) = LIBRARY.get() {
        return reader(name);
    }
    #[cfg(not(target_os = "android"))]
    {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/companion/heic");
        crate::memory::Buffer::read_file(&dir.join(name), None).ok()
    }
    #[cfg(target_os = "android")]
    None
}

fn content(accent: crate::theme::AccentColor) -> server::Content {
    server::Content {
        page: page::render,
        accent,
        library: read_library,
    }
}

pub(crate) fn notify() {
    if let Some(notify) = NOTIFIER.get() {
        notify();
    }
}

/// The address the server listens on: a private IPv4 (10/8, 172.16/12, 192.168/16), wired (`eth*`)
/// before wireless (`wlan*`) before anything else; loopback never.
pub fn choose_ipv4(list: &[(String, Ipv4Addr)]) -> Option<Ipv4Addr> {
    let rank = |name: &str| {
        if name.starts_with("eth") {
            0
        } else if name.starts_with("wlan") {
            1
        } else {
            2
        }
    };
    list.iter()
        .filter(|(_, ip)| ip.is_private() && !ip.is_loopback())
        .min_by_key(|(name, _)| rank(name))
        .map(|(_, ip)| *ip)
}

/// A new access code from `/dev/urandom`, different from `previous`.
pub fn new_code(previous: Option<&str>) -> String {
    loop {
        let code = random_code();
        if previous != Some(code.as_str()) {
            return code;
        }
    }
}

fn random_code() -> String {
    use std::io::Read;
    let mut code = String::with_capacity(CODE_LEN);
    let mut file = std::fs::File::open("/dev/urandom").ok();
    let mut fallback = fallback_seed();
    while code.len() < CODE_LEN {
        let mut byte = [0u8; 1];
        let ok = file
            .as_mut()
            .is_some_and(|f| f.read_exact(&mut byte).is_ok());
        if !ok {
            // No /dev/urandom (never on Android or Linux): a time-seeded generator, logged.
            if file.take().is_some() || code.is_empty() {
                log::warn!("companion: no /dev/urandom, the code comes from the clock");
            }
            fallback ^= fallback << 13;
            fallback ^= fallback >> 7;
            fallback ^= fallback << 17;
            byte[0] = fallback as u8;
        }
        // 248 = 31 * 8: anything above would favor the first symbols.
        if byte[0] < 248 {
            code.push(CODE_ALPHABET[usize::from(byte[0]) % CODE_ALPHABET.len()] as char);
        }
    }
    code
}

fn fallback_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    nanos | 1
}

/// Where the reception is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// No private IPv4: no server, the screen asks for the same network as the phone.
    NoNetwork,
    /// The server is waiting for the phone (which may be editing: see [`Session::connected`]).
    Waiting,
    /// A picture is arriving: the percentage received.
    Receiving(u8),
    /// The picture arrived and is being checked.
    Verifying,
    /// The last picture was refused (the server is still waiting for another).
    Refused(Reject),
    /// The time ran out after [`MAX_RENEWALS`] new codes: the server is off.
    TimedOut,
    /// 5 wrong codes: the server is off.
    TooManyAttempts,
    /// The port could not be opened.
    PortError,
}

impl Status {
    /// Whether the server is off for good (a new code is needed).
    pub fn is_closed(self) -> bool {
        matches!(
            self,
            Status::TimedOut | Status::TooManyAttempts | Status::PortError
        )
    }
}

/// One stay in the Receive image and Confirm screens: the server, its address and code, and its state.
pub struct Session {
    server: Option<Server>,
    status: Status,
    code: String,
    address: Option<(Ipv4Addr, u16)>,
    /// When the phone was last at work (the page loaded, a ping of a page in use, an upload).
    last_edit_ms: Option<u64>,
    /// When the code runs out: [`TIMEOUT_MS`] after the last edit or the opening.
    deadline_ms: u64,
    /// New codes made by the deadline since the last edit.
    renewals: u32,
    /// When to look for a network again while there is none.
    network_check_ms: u64,
    /// The last picture accepted, not yet taken by the screen.
    image: Option<Image>,
    /// What the screen showed at the last poll: the state, "connected" and the countdown.
    shown: (Status, bool, Option<u64>),
    /// After Apply: when the server stops (it only answers the page's pings until then).
    applied_until: Option<u64>,
    interfaces: InterfaceProvider,
    preferred_port: u16,
    accent: crate::theme::AccentColor,
    /// Listens on this address whatever the interfaces say (tests on loopback).
    bind_override: Option<Ipv4Addr>,
}

impl Session {
    /// Opens the session at `now_ms` with the active accent color: a new code and, with a network,
    /// the server.
    pub fn open_with_accent(now_ms: u64, accent: crate::theme::AccentColor) -> Session {
        Session::open_with_config(now_ms, interfaces, PREFERRED_PORT, accent)
    }

    /// Opens the session at `now_ms` with the default accent color.
    pub fn open(now_ms: u64) -> Session {
        Session::open_with_accent(now_ms, crate::theme::AccentColor::default())
    }

    /// Like [`Session::open`] with the interfaces and port given (tests).
    pub fn open_with(now_ms: u64, interfaces: InterfaceProvider, preferred_port: u16) -> Session {
        Session::open_with_config(
            now_ms,
            interfaces,
            preferred_port,
            crate::theme::AccentColor::default(),
        )
    }

    fn open_with_config(
        now_ms: u64,
        interfaces: InterfaceProvider,
        preferred_port: u16,
        accent: crate::theme::AccentColor,
    ) -> Session {
        let mut session = Session {
            server: None,
            status: Status::NoNetwork,
            code: new_code(None),
            address: None,
            last_edit_ms: None,
            deadline_ms: 0,
            renewals: 0,
            network_check_ms: 0,
            image: None,
            shown: (Status::NoNetwork, false, None),
            applied_until: None,
            interfaces,
            preferred_port,
            accent,
            bind_override: None,
        };
        session.start(now_ms);
        session
    }

    /// A session listening on loopback, for tests.
    #[cfg(test)]
    pub(crate) fn open_on_loopback(now_ms: u64) -> Session {
        let mut session = Session::open_with(now_ms, Vec::new, 0);
        session.bind_override = Some(Ipv4Addr::LOCALHOST);
        session.start(now_ms);
        session
    }

    fn start(&mut self, now_ms: u64) {
        let Some(ip) = self
            .bind_override
            .or_else(|| choose_ipv4(&(self.interfaces)()))
        else {
            self.status = Status::NoNetwork;
            self.network_check_ms = now_ms + NETWORK_RETRY_MS;
            return;
        };
        match server::start(
            ip,
            self.preferred_port,
            self.code.clone(),
            content(self.accent),
        ) {
            Ok(server) => {
                self.address = Some((ip, server.port()));
                log::info!("companion: listening on {ip}:{}", server.port());
                self.server = Some(server);
                self.status = Status::Waiting;
                self.deadline_ms = now_ms + TIMEOUT_MS;
            }
            Err(e) => {
                log::error!("companion: cannot listen on {ip}: {e}");
                self.status = Status::PortError;
            }
        }
    }

    /// The phone is at work: the time starts over and the renewals are forgotten.
    fn edit(&mut self, now_ms: u64) {
        self.last_edit_ms = Some(now_ms);
        self.deadline_ms = now_ms + TIMEOUT_MS;
        self.renewals = 0;
    }

    /// Takes the server's news and checks the clock. Returns `true` when what the screen shows changed.
    pub fn poll(&mut self, now_ms: u64) -> bool {
        let code_before = self.code.clone();
        if self.status == Status::NoNetwork && now_ms >= self.network_check_ms {
            self.start(now_ms);
        }
        let events: Vec<Event> = self
            .server
            .as_ref()
            .map(|s| s.events().try_iter().collect())
            .unwrap_or_default();
        let mut news = false;
        for event in events {
            match event {
                Event::PageLoaded => self.edit(now_ms),
                Event::Ping { idle } => {
                    if idle < EDIT_IDLE_S {
                        self.edit(now_ms);
                    }
                }
                Event::Receiving(percent) => {
                    self.edit(now_ms);
                    self.status = Status::Receiving(percent);
                }
                Event::Verifying => self.status = Status::Verifying,
                Event::Rejected(reason) => {
                    self.edit(now_ms);
                    self.status = Status::Refused(reason);
                }
                Event::Accepted(image) => {
                    self.edit(now_ms);
                    self.image = Some(image);
                    self.status = Status::Waiting;
                    news = true;
                }
                Event::TooManyAttempts => {
                    self.status = Status::TooManyAttempts;
                    self.stop_server();
                }
            }
        }
        if self.applied_until.is_some_and(|until| now_ms >= until) {
            log::info!("companion: the page knows, the server stops");
            self.applied_until = None;
            self.stop_server();
            return false;
        }
        if self.applied_until.is_some() {
            return news;
        }
        if self.server.is_some() && !self.status.is_closed() && now_ms >= self.deadline_ms {
            if self.renewals < MAX_RENEWALS {
                log::info!("companion: time is up, a new code");
                let renewals = self.renewals + 1;
                self.renew(now_ms);
                self.renewals = renewals;
            } else {
                log::info!("companion: time is up for good");
                self.status = Status::TimedOut;
                self.stop_server();
            }
        }
        let shown = (self.status, self.connected(now_ms), self.countdown(now_ms));
        let changed = news || code_before != self.code || shown != self.shown;
        self.shown = shown;
        changed
    }

    fn stop_server(&mut self) {
        if let Some(mut server) = self.server.take() {
            server.stop();
        }
    }

    /// Switches the server off (leaving the screens, going to the background).
    pub fn close(&mut self) {
        self.applied_until = None;
        self.stop_server();
    }

    /// Apply: the page hears it at its next ping; uploads are refused, and the server stops
    /// [`APPLIED_LINGER_MS`] later (or at once without a server).
    pub fn applied(&mut self, now_ms: u64) {
        match &self.server {
            Some(server) => {
                server.set_verdict(Verdict::Applied);
                self.applied_until = Some(now_ms + APPLIED_LINGER_MS);
            }
            None => self.close(),
        }
    }

    /// Cancel on the Confirm screen: the page hears that the TV did not use the picture.
    pub fn declined(&self) {
        if let Some(server) = &self.server {
            server.set_verdict(Verdict::Declined);
        }
    }

    /// Whether the session is only telling the page that the picture was applied.
    pub fn is_lingering(&self) -> bool {
        self.applied_until.is_some()
    }

    /// A new code and a new server ("Make a new code", or the time running out).
    pub fn renew(&mut self, now_ms: u64) {
        self.stop_server();
        self.code = new_code(Some(&self.code));
        self.address = None;
        self.last_edit_ms = None;
        self.renewals = 0;
        self.start(now_ms);
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    /// Whether the server is listening.
    pub fn is_listening(&self) -> bool {
        self.server.is_some()
    }

    /// Whether the phone was at work in the last [`CONNECTED_MS`] ("Phone connected · editing…").
    pub fn connected(&self, now_ms: u64) -> bool {
        self.is_listening()
            && self
                .last_edit_ms
                .is_some_and(|at| now_ms < at + CONNECTED_MS)
    }

    /// The whole seconds left, only in the last minute and while nobody is editing.
    pub fn countdown(&self, now_ms: u64) -> Option<u64> {
        (self.is_listening()
            && !self.status.is_closed()
            && !self.connected(now_ms)
            && self.deadline_ms.saturating_sub(now_ms) <= COUNTDOWN_MS)
            .then(|| self.deadline_ms.saturating_sub(now_ms).div_ceil(1000))
    }

    /// The address to type, without the code (`192.168.0.12:8642/wallpaper`).
    pub fn address(&self) -> Option<String> {
        self.address.map(|(ip, port)| {
            format!(
                "{ip}:{port}{}",
                server::WALLPAPER_PATH.trim_end_matches('/')
            )
        })
    }

    /// What the QR carries: the page's address with the code.
    pub fn qr_url(&self) -> Option<String> {
        self.address.map(|(ip, port)| {
            format!(
                "http://{ip}:{port}{}?c={}",
                server::WALLPAPER_PATH,
                self.code
            )
        })
    }

    /// The port the server listens on (tests).
    #[cfg(test)]
    pub(crate) fn port(&self) -> Option<u16> {
        self.address.map(|(_, port)| port)
    }

    /// When the screen needs its next frame without any news, counted from `drawn_ms`, the time of
    /// the last frame: the next second of the countdown, the end of "connected", the start of the
    /// last minute or the deadline, or the next look for a network. Absolute moments: asked again at
    /// any later turn of the loop, the answer is the same.
    pub fn next_wake_ms(&self, drawn_ms: u64) -> Option<u64> {
        if let Some(until) = self.applied_until {
            return Some(until);
        }
        if self.status == Status::NoNetwork {
            return Some(self.network_check_ms);
        }
        if self.server.is_none() || self.status.is_closed() {
            return None;
        }
        let mut wake = self.deadline_ms;
        let mut consider = |at: u64| {
            if at > drawn_ms {
                wake = wake.min(at);
            }
        };
        if let Some(edit) = self.last_edit_ms {
            consider(edit + CONNECTED_MS);
        }
        consider(self.deadline_ms.saturating_sub(COUNTDOWN_MS));
        if self.countdown(drawn_ms).is_some() {
            // The countdown changes when the time left crosses a whole second.
            let left = self.deadline_ms.saturating_sub(drawn_ms);
            let step = match left % 1000 {
                0 => 1000.min(left.max(1)),
                part => part,
            };
            consider(drawn_ms + step);
        }
        Some(wake)
    }

    /// The last picture accepted, once.
    pub fn take_image(&mut self) -> Option<Image> {
        self.image.take()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop_server();
    }
}

/// At most one progress notice per period (4 a second): the loop draws no more frames than that.
#[derive(Debug)]
pub struct Throttle {
    every: std::time::Duration,
    last: Option<Instant>,
}

impl Throttle {
    pub fn new(every: std::time::Duration) -> Throttle {
        Throttle { every, last: None }
    }

    /// Whether a notice may go out at `now` (and, if so, counts it).
    pub fn ready(&mut self, now: Instant) -> bool {
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < self.every)
        {
            return false;
        }
        self.last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::server::tests::{qoi, request_to, upload_request};
    use super::*;

    fn list(items: &[(&str, [u8; 4])]) -> Vec<(String, Ipv4Addr)> {
        items
            .iter()
            .map(|(n, ip)| (n.to_string(), Ipv4Addr::from(*ip)))
            .collect()
    }

    #[test]
    fn the_address_is_private_and_wired_before_wireless() {
        assert_eq!(
            choose_ipv4(&list(&[
                ("wlan0", [192, 168, 0, 20]),
                ("eth0", [10, 0, 0, 5]),
                ("lo", [127, 0, 0, 1]),
            ])),
            Some(Ipv4Addr::new(10, 0, 0, 5))
        );
        assert_eq!(
            choose_ipv4(&list(&[
                ("rmnet0", [172, 20, 1, 1]),
                ("wlan0", [192, 168, 0, 20]),
            ])),
            Some(Ipv4Addr::new(192, 168, 0, 20))
        );
        // A public address is not the local network.
        assert_eq!(choose_ipv4(&list(&[("eth0", [8, 8, 8, 8])])), None);
        assert_eq!(choose_ipv4(&list(&[("lo", [127, 0, 0, 1])])), None);
        assert_eq!(
            choose_ipv4(&list(&[("tun0", [172, 16, 3, 4]), ("eth0", [8, 8, 4, 4])])),
            Some(Ipv4Addr::new(172, 16, 3, 4))
        );
        assert_eq!(choose_ipv4(&[]), None);
    }

    #[test]
    fn codes_use_only_the_alphabet_and_two_in_a_row_differ() {
        let mut previous: Option<String> = None;
        for _ in 0..10_000 {
            let code = new_code(previous.as_deref());
            assert_eq!(code.len(), CODE_LEN);
            assert!(code.bytes().all(|b| CODE_ALPHABET.contains(&b)), "{code}");
            assert_ne!(Some(&code), previous.as_ref());
            previous = Some(code);
        }
        assert!(!CODE_ALPHABET.iter().any(|b| b"0O1IL".contains(b)));
    }

    #[test]
    fn the_throttle_lets_four_a_second_through() {
        let mut throttle = Throttle::new(std::time::Duration::from_millis(250));
        let start = Instant::now();
        let passed = (0..1000)
            .filter(|i| throttle.ready(start + std::time::Duration::from_millis(*i)))
            .count();
        assert_eq!(passed, 4);
    }

    fn no_interfaces() -> Vec<(String, Ipv4Addr)> {
        Vec::new()
    }

    #[test]
    fn without_a_network_there_is_no_server_and_it_looks_again_every_3_s() {
        let mut session = Session::open_with(0, no_interfaces, 0);
        assert_eq!(session.status(), Status::NoNetwork);
        assert!(!session.is_listening());
        assert_eq!(session.qr_url(), None);
        assert_eq!(session.next_wake_ms(0), Some(NETWORK_RETRY_MS));
        // Asked again later, it is the same moment: it does not slide away.
        assert_eq!(session.next_wake_ms(1_500), Some(NETWORK_RETRY_MS));
        assert!(!session.poll(NETWORK_RETRY_MS));
        assert_eq!(
            session.next_wake_ms(NETWORK_RETRY_MS),
            Some(2 * NETWORK_RETRY_MS)
        );
    }

    fn port_is_closed(port: u16) -> bool {
        std::net::TcpStream::connect_timeout(
            &(Ipv4Addr::LOCALHOST, port).into(),
            std::time::Duration::from_millis(300),
        )
        .is_err()
    }

    /// Waits for the thread of a stopped server to drop its listener.
    fn wait_closed(port: u16) -> bool {
        (0..50).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            port_is_closed(port)
        })
    }

    fn address(session: &Session) -> std::net::SocketAddr {
        (Ipv4Addr::LOCALHOST, session.port().unwrap()).into()
    }

    /// Sends a ping and waits for the session to see it, at `now_ms`.
    fn ping(session: &mut Session, idle: u64, now_ms: u64) {
        let raw = format!(
            "POST /ping?c={}&idle={idle} HTTP/1.1\r\n\r\n",
            session.code()
        );
        let answer = request_to(address(session), raw.as_bytes());
        assert!(answer.contains(r#""ok":true"#), "{answer}");
        settle(session, now_ms);
    }

    /// Polls at `now_ms` until the server's events arrived (they come from another thread).
    fn settle(session: &mut Session, now_ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(30));
        session.poll(now_ms);
    }

    #[test]
    fn the_address_and_the_qr_lead_to_the_wallpaper_page() {
        let session = Session::open_on_loopback(0);
        let port = session.port().unwrap();
        assert_eq!(
            session.qr_url(),
            Some(format!(
                "http://127.0.0.1:{port}/wallpaper/?c={}",
                session.code()
            ))
        );
        assert_eq!(
            session.address(),
            Some(format!("127.0.0.1:{port}/wallpaper"))
        );
    }

    #[test]
    fn the_time_runs_out_three_times_with_a_new_code_and_then_for_good() {
        let mut session = Session::open_on_loopback(1_000);
        assert_eq!(session.status(), Status::Waiting);
        let first_port = session.port().unwrap();
        // The countdown shows only in the last minute.
        assert_eq!(session.countdown(1_000), None);
        assert_eq!(
            session.next_wake_ms(1_000),
            Some(1_000 + TIMEOUT_MS - COUNTDOWN_MS)
        );
        let last_minute = 1_000 + TIMEOUT_MS - 45_000;
        assert!(session.poll(last_minute));
        assert_eq!(session.countdown(last_minute), Some(45));
        assert_eq!(session.next_wake_ms(last_minute), Some(last_minute + 1_000));
        let mut codes = vec![session.code().to_string()];
        let mut at = 1_000;
        for renewal in 1..=MAX_RENEWALS {
            at += TIMEOUT_MS;
            session.poll(at - 1);
            assert_eq!(session.countdown(at - 1), Some(1));
            assert!(session.poll(at), "renewal {renewal}");
            assert_eq!(session.status(), Status::Waiting);
            assert!(session.is_listening());
            assert!(!codes.contains(&session.code().to_string()));
            codes.push(session.code().to_string());
        }
        assert!(wait_closed(first_port));
        at += TIMEOUT_MS;
        assert!(session.poll(at));
        assert_eq!(session.status(), Status::TimedOut);
        assert!(!session.is_listening());
        assert_eq!(session.next_wake_ms(at), None);
        // "Make a new code" opens it again, with another code and the renewals forgotten.
        session.renew(at);
        assert_eq!(session.status(), Status::Waiting);
        assert!(!codes.contains(&session.code().to_string()));
        session.close();
    }

    #[test]
    fn ten_minutes_of_editing_do_not_run_the_time_out() {
        let mut session = Session::open_on_loopback(0);
        let code = session.code().to_string();
        let page = format!("GET /wallpaper/?c={code} HTTP/1.1\r\n\r\n");
        request_to(address(&session), page.as_bytes());
        settle(&mut session, 0);
        assert!(session.connected(0));
        // A ping every 30 s, used a few seconds before each.
        for i in 1..=20 {
            ping(&mut session, 5, i * 30_000);
            assert!(session.connected(i * 30_000));
            assert_eq!(session.countdown(i * 30_000), None);
        }
        assert_eq!(session.code(), code);
        assert_eq!(session.status(), Status::Waiting);
        session.close();
    }

    #[test]
    fn a_forgotten_page_runs_out_after_2_and_then_5_minutes() {
        let mut session = Session::open_on_loopback(0);
        let code = session.code().to_string();
        // Pings of a page nobody touches: the idle time grows with them.
        for i in 1..=8u64 {
            ping(&mut session, i * 30, i * 30_000);
        }
        // The last one that counted was at 90 s (idle 90 < 120).
        assert!(!session.connected(90_000 + CONNECTED_MS));
        assert!(session.poll(90_000 + TIMEOUT_MS));
        assert_ne!(session.code(), code);
        session.close();
    }

    #[test]
    fn a_hidden_page_stops_being_connected_after_75_s() {
        let mut session = Session::open_on_loopback(0);
        ping(&mut session, 0, 10_000);
        assert!(session.connected(10_000));
        assert_eq!(session.next_wake_ms(10_000), Some(10_000 + CONNECTED_MS));
        assert!(!session.poll(10_000 + CONNECTED_MS - 1));
        assert!(session.poll(10_000 + CONNECTED_MS));
        assert!(!session.connected(10_000 + CONNECTED_MS));
        session.close();
    }

    #[test]
    fn the_heic_decoder_is_served_from_the_repository_with_its_type_and_size() {
        let session = Session::open_on_loopback(0);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/companion/heic");
        for (name, kind) in [
            ("heic.js", "text/javascript"),
            ("heic.wasm", "application/wasm"),
        ] {
            let size = std::fs::metadata(dir.join(name)).unwrap().len();
            let raw = format!("GET /lib/{name} HTTP/1.1\r\n\r\n");
            let answer = request_to(address(&session), raw.as_bytes());
            let head = &answer[..answer.find("\r\n\r\n").unwrap()];
            assert!(head.starts_with("HTTP/1.1 200 OK"), "{head}");
            assert!(head.contains(&format!("Content-Type: {kind}")), "{head}");
            assert!(
                head.contains(&format!("Content-Length: {size}\r\n")),
                "{head}"
            );
        }
    }

    #[test]
    fn after_apply_the_page_hears_it_and_the_server_stops_10_s_later() {
        let mut session = Session::open_on_loopback(0);
        let port = session.port().unwrap();
        session.applied(1_000);
        assert!(session.is_lingering());
        let raw = format!("POST /ping?c={}&idle=0 HTTP/1.1\r\n\r\n", session.code());
        let answer = request_to(address(&session), raw.as_bytes());
        assert!(answer.ends_with(r#""tv":"applied"}"#), "{answer}");
        assert_eq!(session.next_wake_ms(1_000), Some(1_000 + APPLIED_LINGER_MS));
        // The pings meanwhile do not move the time nor make a new code.
        settle(&mut session, 1_000 + APPLIED_LINGER_MS - 1);
        assert!(session.is_listening());
        session.poll(1_000 + APPLIED_LINGER_MS);
        assert!(!session.is_listening() && !session.is_lingering());
        assert!(wait_closed(port));
    }

    #[test]
    fn the_deadline_does_not_slide_between_turns_of_the_loop() {
        let session = Session::open_on_loopback(0);
        let first = session.next_wake_ms(0);
        for later in [1, 500, 30_000, 200_000] {
            assert_eq!(session.next_wake_ms(later), first, "asked at {later}");
        }
    }

    #[test]
    fn closing_the_session_frees_the_port() {
        let mut session = Session::open_on_loopback(0);
        let port = session.port().unwrap();
        assert!(!port_is_closed(port));
        session.close();
        assert!(wait_closed(port));
        // Dropping does the same.
        let session = Session::open_on_loopback(0);
        let port = session.port().unwrap();
        drop(session);
        assert!(wait_closed(port));
    }

    #[test]
    fn an_accepted_picture_is_handed_over_and_the_server_keeps_listening() {
        let mut session = Session::open_on_loopback(0);
        for round in 0..2 {
            let answer = request_to(
                address(&session),
                &upload_request(session.code(), &qoi(1920, 1080)),
            );
            assert!(answer.contains(r#"{"ok":true}"#), "{answer}");
            let mut image = None;
            for _ in 0..50 {
                session.poll(10);
                image = session.take_image();
                if image.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert_eq!(
                image.map(|i| (i.width, i.height)),
                Some((1920, 1080)),
                "round {round}"
            );
            assert!(session.is_listening());
            assert_eq!(session.status(), Status::Waiting);
            assert!(session.connected(10));
        }
        session.close();
    }
}
