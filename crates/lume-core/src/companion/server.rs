//! A minimal HTTP/1.1 server in `std::net`: one thread blocked on `accept`, one connection at a time,
//! `Connection: close`. Each function of the page has its own path, with its routes under it; what
//! belongs to the session (`/ping`) or is shared (`/lib/`) stays outside:
//!
//! | Route | Answer |
//! |---|---|
//! | `GET /wallpaper/` | the wallpaper page in the language of `Accept-Language` |
//! | `POST /wallpaper/upload?c=<code>` | a QOI body of 1920x1080; JSON `{"ok": true}` or `{"ok": false, "error": "<kind>"}` |
//! | `POST /ping?c=<code>&idle=<s>` | the page is open and visible, `idle` seconds after its last use; `{"ok": true, "tv": "<verdict>"}` |
//! | `GET /lib/heic.js`, `GET /lib/heic.wasm` | the HEIC decoder, asked only when the browser cannot open a photo |
//! | `GET /`, `GET /wallpaper` | 302 to `/wallpaper/`, keeping the query (`?c=`) |
//! | anything else | 404 |
//!
//! Everything that arrives is untrusted: headers up to 8 KB, 10 s per read or write, a body only
//! with `Content-Length` and at most [`MAX_BODY`] (refused before it is read), the picture checked
//! and decoded in full before it is accepted. While a picture arrives, a second upload gets "busy".
//! An accepted picture does not stop the server: the phone may send another framing. What the TV did
//! with it (the [`Verdict`]) goes back to the page in the answer to its pings; once the picture is
//! applied, uploads are refused and only the pings are answered, until the session stops the server.
//! To stop, a flag is set and a local connection wakes the `accept`; the listener goes with the thread.

use super::{notify, Throttle, MAX_WRONG_CODES};
use crate::wallpaper::custom::decode_qoi;
use crate::wallpaper::Image;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The largest body accepted: a 1920x1080 QOI in the worst case (4 bytes a pixel, 8.3 MB) and its
/// header and end, rounded up to 9 MB.
pub const MAX_BODY: usize = 9 * 1024 * 1024;
/// The largest request head (request line and headers).
pub const MAX_HEAD: usize = 8 * 1024;
/// Read and write timeout of a connection.
pub const IO_TIMEOUT: Duration = Duration::from_secs(10);
/// The least time between two progress notices (4 a second).
pub const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// The path of the wallpaper function.
pub const WALLPAPER_PATH: &str = "/wallpaper/";

/// Builds the page for an `Accept-Language` header and active accent color.
pub type PageBuilder = fn(&str, crate::theme::AccentColor) -> String;
/// Reads a shared file of the page (`heic.js`, `heic.wasm`).
pub type LibReader = fn(&str) -> Option<crate::memory::Buffer>;

/// The shared files the page may ask for, and their types.
const LIBRARY: &[(&str, &str)] = &[
    ("heic.js", "text/javascript; charset=utf-8"),
    ("heic.wasm", "application/wasm"),
];

/// Why a picture was refused (the server keeps waiting for another).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    /// The body is over [`MAX_BODY`].
    TooBig,
    /// Not QOI, not 1920x1080, cut short or inconsistent.
    Unreadable,
}

impl Reject {
    fn kind(self) -> &'static str {
        match self {
            Reject::TooBig => "too_big",
            Reject::Unreadable => "read",
        }
    }
}

/// News from the server for the screen.
#[derive(Debug)]
pub enum Event {
    /// The page was loaded with the right code.
    PageLoaded,
    /// The page is open and visible; its last use was `idle` seconds ago.
    Ping { idle: u64 },
    /// A picture is arriving: the percentage received (at most 4 a second).
    Receiving(u8),
    /// The picture arrived and is being checked.
    Verifying,
    /// The picture was refused.
    Rejected(Reject),
    /// A valid picture (the server keeps listening).
    Accepted(Image),
    /// 5 wrong codes: the server stops.
    TooManyAttempts,
}

/// What the TV did with the last picture received, which the page reads in the answer to its pings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Verdict {
    /// No picture yet.
    None = 0,
    /// A picture is on the Confirm screen.
    Pending = 1,
    /// Apply: it is the wallpaper (the server only answers pings until it stops).
    Applied = 2,
    /// Cancel: the TV did not use it.
    Declined = 3,
}

impl Verdict {
    fn from_u8(value: u8) -> Verdict {
        match value {
            1 => Verdict::Pending,
            2 => Verdict::Applied,
            3 => Verdict::Declined,
            _ => Verdict::None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Verdict::None => "none",
            Verdict::Pending => "pending",
            Verdict::Applied => "applied",
            Verdict::Declined => "declined",
        }
    }
}

/// A running server.
pub struct Server {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    verdict: Arc<AtomicU8>,
    thread: Option<JoinHandle<()>>,
    events: Receiver<Event>,
}

/// What the server serves: the page and the shared files.
#[derive(Clone, Copy)]
pub struct Content {
    pub page: PageBuilder,
    pub accent: crate::theme::AccentColor,
    pub library: LibReader,
}

/// Starts listening on `ip`, on `preferred_port` or, when it is taken, on a port the system chooses.
pub fn start(
    ip: Ipv4Addr,
    preferred_port: u16,
    code: String,
    content: Content,
) -> io::Result<Server> {
    let listener = TcpListener::bind((ip, preferred_port)).or_else(|e| {
        log::info!("companion: port {preferred_port} unavailable ({e}), the system chooses one");
        TcpListener::bind((ip, 0))
    })?;
    let address = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let verdict = Arc::new(AtomicU8::new(Verdict::None as u8));
    let (sender, events) = channel();
    let context = Context {
        code,
        content,
        events: sender,
        stop: stop.clone(),
        verdict: verdict.clone(),
        wrong_codes: 0,
    };
    let thread = std::thread::Builder::new()
        .name("lume-companion".into())
        .spawn(move || serve(listener, context))?;
    Ok(Server {
        address,
        stop,
        verdict,
        thread: Some(thread),
        events,
    })
}

impl Server {
    pub fn port(&self) -> u16 {
        self.address.port()
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Tells the page, at its next ping, what the TV did with the picture.
    pub fn set_verdict(&self, verdict: Verdict) {
        self.verdict.store(verdict as u8, Ordering::Release);
    }

    pub fn verdict(&self) -> Verdict {
        Verdict::from_u8(self.verdict.load(Ordering::Acquire))
    }

    /// Asks the thread to stop (it ends after the request it is serving) without waiting for it.
    pub fn stop(&mut self) {
        if self.stop.swap(true, Ordering::AcqRel) {
            return;
        }
        // Wakes the blocked `accept`; the thread sees the flag and drops the listener.
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_millis(500));
    }

    /// Stops and waits for the thread to end (the port is free afterwards).
    pub fn stop_and_wait(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Context {
    code: String,
    content: Content,
    events: Sender<Event>,
    stop: Arc<AtomicBool>,
    verdict: Arc<AtomicU8>,
    wrong_codes: u32,
}

impl Context {
    fn send(&self, event: Event) {
        let _ = self.events.send(event);
        notify();
    }

    fn stopping(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn verdict(&self) -> Verdict {
        Verdict::from_u8(self.verdict.load(Ordering::Acquire))
    }

    fn code_matches(&self, head: &Head) -> bool {
        head.query("c").map(normalize_code).as_deref() == Some(self.code.as_str())
    }

    /// Counts a wrong code: answers it and, at the fifth, stops the server.
    fn wrong_code(&mut self, stream: &mut TcpStream) -> Flow {
        self.wrong_codes += 1;
        if self.wrong_codes >= MAX_WRONG_CODES {
            log::warn!("companion: too many wrong codes");
            refuse_unread(stream, "403 Forbidden", "attempts");
            self.send(Event::TooManyAttempts);
            return Flow::Stop;
        }
        refuse_unread(stream, "403 Forbidden", "code");
        Flow::Continue
    }
}

enum Flow {
    Continue,
    Stop,
}

fn serve(listener: TcpListener, mut context: Context) {
    for stream in listener.incoming() {
        if context.stopping() {
            break;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        match handle(&mut context, &mut stream, &listener) {
            Ok(Flow::Continue) | Err(_) => {}
            Ok(Flow::Stop) => break,
        }
        if context.stopping() {
            break;
        }
    }
    log::info!("companion: server stopped");
}

/// A parsed request head.
struct Head {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    /// Bytes read after the head (the start of the body).
    rest: Vec<u8>,
}

impl Head {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }

    /// The query with its `?`, or nothing.
    fn query_string(&self) -> &str {
        self.target.find('?').map_or("", |at| &self.target[at..])
    }

    fn query(&self, key: &str) -> Option<&str> {
        let query = self.target.split_once('?')?.1;
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }
}

fn read_head(stream: &mut TcpStream) -> io::Result<Option<Head>> {
    let mut data = Vec::with_capacity(1024);
    let mut buf = [0u8; 1024];
    let end = loop {
        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        if data.len() > MAX_HEAD {
            return Ok(None);
        }
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }
        data.extend_from_slice(&buf[..n]);
    };
    if end > MAX_HEAD {
        return Ok(None);
    }
    let Ok(text) = std::str::from_utf8(&data[..end]) else {
        return Ok(None);
    };
    let mut lines = text.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (Some(method), Some(target)) = (first.next(), first.next()) else {
        return Ok(None);
    };
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Ok(Some(Head {
        method: method.to_string(),
        target: target.to_string(),
        headers,
        rest: data[end + 4..].to_vec(),
    }))
}

fn respond_with(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    extra: &str,
    body: &[u8],
) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    respond_with(
        stream,
        status,
        content_type,
        "Cache-Control: no-store\r\n",
        body,
    );
}

fn respond_json(stream: &mut TcpStream, status: &str, error: Option<&str>) {
    let body = match error {
        None => r#"{"ok":true}"#.to_string(),
        Some(kind) => format!(r#"{{"ok":false,"error":"{kind}"}}"#),
    };
    respond(stream, status, "application/json", body.as_bytes());
}

/// Answers an error before the body was read, then reads a little of it so the client gets the
/// answer and not a reset of the connection.
fn refuse_unread(stream: &mut TcpStream, status: &str, kind: &str) {
    respond_json(stream, status, Some(kind));
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let mut sink = [0u8; 16 * 1024];
    for _ in 0..16 {
        if !matches!(stream.read(&mut sink), Ok(n) if n > 0) {
            break;
        }
    }
}

fn handle(
    context: &mut Context,
    stream: &mut TcpStream,
    listener: &TcpListener,
) -> io::Result<Flow> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    if context.stopping() {
        return Ok(Flow::Stop);
    }
    let Some(head) = read_head(stream)? else {
        respond(stream, "400 Bad Request", "text/plain", b"bad request");
        return Ok(Flow::Continue);
    };
    match (head.method.as_str(), head.path()) {
        ("GET", "/" | "/wallpaper") => {
            let location = format!("{WALLPAPER_PATH}{}", head.query_string());
            let extra = format!("Location: {location}\r\nCache-Control: no-store\r\n");
            respond_with(stream, "302 Found", "text/plain", &extra, b"");
            Ok(Flow::Continue)
        }
        ("GET", WALLPAPER_PATH) => {
            if context.code_matches(&head) {
                context.send(Event::PageLoaded);
            }
            let page = (context.content.page)(
                head.header("accept-language").unwrap_or(""),
                context.content.accent,
            );
            respond(
                stream,
                "200 OK",
                "text/html; charset=utf-8",
                page.as_bytes(),
            );
            Ok(Flow::Continue)
        }
        ("POST", "/wallpaper/upload") => upload(context, stream, listener, head),
        ("POST", "/ping") => {
            if !context.code_matches(&head) {
                return Ok(context.wrong_code(stream));
            }
            let idle = head
                .query("idle")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(u64::MAX);
            let body = format!(r#"{{"ok":true,"tv":"{}"}}"#, context.verdict().as_str());
            respond(stream, "200 OK", "application/json", body.as_bytes());
            context.send(Event::Ping { idle });
            Ok(Flow::Continue)
        }
        ("GET", path) if path.starts_with("/lib/") => {
            let name = &path["/lib/".len()..];
            let file = LIBRARY
                .iter()
                .find(|(n, _)| *n == name)
                .and_then(|(n, kind)| Some(((context.content.library)(n)?, *kind)));
            match file {
                Some((bytes, kind)) => respond_with(
                    stream,
                    "200 OK",
                    kind,
                    "Cache-Control: max-age=86400\r\n",
                    &bytes,
                ),
                None => respond(stream, "404 Not Found", "text/plain", b"not found"),
            }
            Ok(Flow::Continue)
        }
        _ => {
            respond(stream, "404 Not Found", "text/plain", b"not found");
            Ok(Flow::Continue)
        }
    }
}

fn normalize_code(code: &str) -> String {
    code.trim().to_ascii_uppercase()
}

fn upload(
    context: &mut Context,
    stream: &mut TcpStream,
    listener: &TcpListener,
    head: Head,
) -> io::Result<Flow> {
    if !context.code_matches(&head) {
        return Ok(context.wrong_code(stream));
    }
    if context.verdict() == Verdict::Applied {
        refuse_unread(stream, "409 Conflict", "applied");
        return Ok(Flow::Continue);
    }
    let chunked = head
        .header("transfer-encoding")
        .is_some_and(|v| !v.eq_ignore_ascii_case("identity"));
    let length = head
        .header("content-length")
        .and_then(|v| v.parse::<usize>().ok());
    let Some(length) = length.filter(|_| !chunked) else {
        refuse_unread(stream, "411 Length Required", "length");
        return Ok(Flow::Continue);
    };
    if length > MAX_BODY {
        context.send(Event::Rejected(Reject::TooBig));
        refuse_unread(stream, "413 Payload Too Large", Reject::TooBig.kind());
        return Ok(Flow::Continue);
    }
    if head
        .header("expect")
        .is_some_and(|v| v.eq_ignore_ascii_case("100-continue"))
    {
        stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
    }
    context.send(Event::Receiving(0));
    let body = match read_body(context, stream, listener, head.rest, length) {
        Ok(Some(body)) => body,
        Ok(None) => return Ok(Flow::Stop),
        Err(e) => {
            log::warn!("companion: the upload was cut: {e}");
            context.send(Event::Rejected(Reject::Unreadable));
            respond_json(stream, "400 Bad Request", Some(Reject::Unreadable.kind()));
            return Ok(Flow::Continue);
        }
    };
    context.send(Event::Verifying);
    match decode_qoi(&body) {
        Ok(image) => {
            drop(body);
            log::info!("companion: picture accepted");
            context
                .verdict
                .store(Verdict::Pending as u8, Ordering::Release);
            respond_json(stream, "200 OK", None);
            context.send(Event::Accepted(image));
            Ok(Flow::Continue)
        }
        Err(e) => {
            log::warn!("companion: picture refused: {e:?}");
            context.send(Event::Rejected(Reject::Unreadable));
            respond_json(
                stream,
                "422 Unprocessable Content",
                Some(Reject::Unreadable.kind()),
            );
            Ok(Flow::Continue)
        }
    }
}

/// Reads `length` bytes of body, sending the progress and answering "busy" to anyone else who tries
/// meanwhile. `None` when the server is being stopped.
fn read_body(
    context: &Context,
    stream: &mut TcpStream,
    listener: &TcpListener,
    start: Vec<u8>,
    length: usize,
) -> io::Result<Option<crate::memory::Buffer>> {
    // The body in a buffer of its own: it goes back to the system as soon as it is decoded.
    let mut body = crate::memory::Buffer::zeroed(length);
    let mut filled = start.len().min(length);
    body[..filled].copy_from_slice(&start[..filled]);
    let mut throttle = Throttle::new(PROGRESS_EVERY);
    throttle.ready(Instant::now());
    listener.set_nonblocking(true)?;
    let result = (|| {
        while filled < length {
            if context.stopping() {
                return Ok(None);
            }
            let want = (length - filled).min(64 * 1024);
            let n = stream.read(&mut body[filled..filled + want])?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "cut short"));
            }
            filled += n;
            if throttle.ready(Instant::now()) {
                let percent = (filled * 100 / length.max(1)) as u8;
                context.send(Event::Receiving(percent.min(99)));
            }
            refuse_busy(listener);
        }
        Ok(Some(std::mem::replace(
            &mut body,
            crate::memory::Buffer::zeroed(0),
        )))
    })();
    listener.set_nonblocking(false)?;
    result
}

/// Answers every connection waiting at the door that the TV is busy with another picture.
fn refuse_busy(listener: &TcpListener) {
    while let Ok((mut other, _)) = listener.accept() {
        let _ = other.set_nonblocking(false);
        let _ = other.set_read_timeout(Some(Duration::from_secs(2)));
        let _ = other.set_write_timeout(Some(Duration::from_secs(2)));
        refuse_unread(&mut other, "503 Service Unavailable", "busy");
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn page(language: &str, _accent: crate::theme::AccentColor) -> String {
        format!("<html>{language}</html>")
    }

    fn library(name: &str) -> Option<crate::memory::Buffer> {
        (name == "heic.js").then(|| b"// heic".to_vec().into())
    }

    fn start_local(code: &str) -> Server {
        start(
            Ipv4Addr::LOCALHOST,
            0,
            code.to_string(),
            Content {
                page,
                accent: crate::theme::AccentColor::default(),
                library,
            },
        )
        .unwrap()
    }

    /// Sends a raw request and returns the whole answer.
    pub(crate) fn request_to(address: SocketAddr, raw: &[u8]) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(raw).unwrap();
        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer);
        String::from_utf8_lossy(&answer).into_owned()
    }

    fn request(server: &Server, raw: &[u8]) -> String {
        request_to(server.address(), raw)
    }

    pub(crate) fn upload_request(code: &str, body: &[u8]) -> Vec<u8> {
        let mut raw = format!(
            "POST /wallpaper/upload?c={code} HTTP/1.1\r\nHost: tv\r\nContent-Type: image/qoi\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        raw.extend_from_slice(body);
        raw
    }

    pub(crate) fn qoi(width: u32, height: u32) -> Vec<u8> {
        let rgb: Vec<u8> = (0..width * height)
            .flat_map(|i| [(i % 256) as u8, (i / 7 % 256) as u8, 30])
            .collect();
        qoi::encode_to_vec(rgb, width, height).unwrap()
    }

    fn events(server: &Server) -> Vec<Event> {
        server.events().try_iter().collect()
    }

    #[test]
    fn the_page_answers_under_its_path_and_unknown_paths_are_404() {
        let mut server = start_local("ABCD");
        let answer = request(
            &server,
            b"GET /wallpaper/?c=ABCD HTTP/1.1\r\nAccept-Language: es-ES,es;q=0.9\r\n\r\n",
        );
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        assert!(answer.contains("<html>es-ES,es;q=0.9</html>"));
        for path in ["/favicon.ico", "/fotos/", "/upload", "/wallpaper/x"] {
            let answer = request(&server, format!("GET {path} HTTP/1.1\r\n\r\n").as_bytes());
            assert!(answer.starts_with("HTTP/1.1 404"), "{path}: {answer}");
        }
        server.stop_and_wait();
        // Only the page with the right code is the phone at work.
        assert!(matches!(events(&server)[..], [Event::PageLoaded]));
    }

    #[test]
    fn the_root_leads_to_the_wallpaper_keeping_the_code() {
        let mut server = start_local("ABCD");
        let answer = request(&server, b"GET /?c=ABCD HTTP/1.1\r\n\r\n");
        assert!(answer.starts_with("HTTP/1.1 302"), "{answer}");
        assert!(
            answer.contains("Location: /wallpaper/?c=ABCD\r\n"),
            "{answer}"
        );
        let answer = request(&server, b"GET / HTTP/1.1\r\n\r\n");
        assert!(answer.contains("Location: /wallpaper/\r\n"), "{answer}");
        // What the screen shows to type, without the slash.
        let answer = request(&server, b"GET /wallpaper HTTP/1.1\r\n\r\n");
        assert!(answer.contains("Location: /wallpaper/\r\n"), "{answer}");
        server.stop_and_wait();
    }

    #[test]
    fn the_ping_answers_and_tells_how_long_the_page_was_idle() {
        let mut server = start_local("ABCD");
        let answer = request(&server, b"POST /ping?c=abcd&idle=42 HTTP/1.1\r\n\r\n");
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        assert!(answer.ends_with(r#"{"ok":true,"tv":"none"}"#), "{answer}");
        let answer = request(&server, b"POST /ping?c=WXYZ&idle=1 HTTP/1.1\r\n\r\n");
        assert!(answer.contains(r#""error":"code""#), "{answer}");
        server.stop_and_wait();
        assert!(matches!(events(&server)[..], [Event::Ping { idle: 42 }]));
    }

    #[test]
    fn the_ping_carries_the_verdict_and_an_applied_picture_closes_the_uploads() {
        let mut server = start_local("ABCD");
        let ping = b"POST /ping?c=ABCD&idle=0 HTTP/1.1\r\n\r\n";
        let answer = request(&server, &upload_request("ABCD", &qoi(1920, 1080)));
        assert!(answer.contains(r#"{"ok":true}"#), "{answer}");
        assert!(request(&server, ping).ends_with(r#""tv":"pending"}"#));
        server.set_verdict(Verdict::Declined);
        assert!(request(&server, ping).ends_with(r#""tv":"declined"}"#));
        server.set_verdict(Verdict::Applied);
        assert!(request(&server, ping).ends_with(r#""tv":"applied"}"#));
        let answer = request(&server, &upload_request("ABCD", &qoi(64, 64)));
        assert!(answer.starts_with("HTTP/1.1 409"), "{answer}");
        assert!(answer.contains(r#""error":"applied""#), "{answer}");
        server.stop_and_wait();
    }

    #[test]
    fn the_library_is_served_with_its_type() {
        let mut server = start_local("ABCD");
        let answer = request(&server, b"GET /lib/heic.js HTTP/1.1\r\n\r\n");
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        assert!(answer.contains("Content-Type: text/javascript"), "{answer}");
        assert!(answer.contains("Content-Length: 7\r\n"), "{answer}");
        // Known but missing, and unknown.
        for path in ["/lib/heic.wasm", "/lib/other.js", "/lib/../index.html"] {
            let answer = request(&server, format!("GET {path} HTTP/1.1\r\n\r\n").as_bytes());
            assert!(answer.starts_with("HTTP/1.1 404"), "{path}: {answer}");
        }
        server.stop_and_wait();
    }

    #[test]
    fn a_wrong_code_is_refused_and_five_switch_the_server_off() {
        let mut server = start_local("ABCD");
        for _ in 0..3 {
            let answer = request(&server, &upload_request("WXYZ", &qoi(64, 64)));
            assert!(answer.contains(r#""error":"code""#), "{answer}");
        }
        // A ping with a wrong code counts too.
        let answer = request(&server, b"POST /ping?c=WXYZ&idle=0 HTTP/1.1\r\n\r\n");
        assert!(answer.contains(r#""error":"code""#), "{answer}");
        let answer = request(&server, &upload_request("WXYZ", b""));
        assert!(answer.contains(r#""error":"attempts""#), "{answer}");
        let address = server.address();
        server.stop_and_wait();
        assert!(events(&server)
            .iter()
            .any(|e| matches!(e, Event::TooManyAttempts)));
        // The port is free: nobody listens any more.
        assert!(TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_err());
    }

    #[test]
    fn stopping_frees_the_port() {
        let mut server = start_local("ABCD");
        let address = server.address();
        assert!(TcpStream::connect(address).is_ok());
        server.stop_and_wait();
        assert!(TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_err());
    }

    #[test]
    fn a_body_over_the_limit_is_refused_without_being_read() {
        let mut server = start_local("ABCD");
        let raw = format!(
            "POST /wallpaper/upload?c=ABCD HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            40 * 1024 * 1024
        );
        let answer = request(&server, raw.as_bytes());
        assert!(answer.starts_with("HTTP/1.1 413"), "{answer}");
        assert!(answer.contains(r#""error":"too_big""#));
        let chunked =
            b"POST /wallpaper/upload?c=ABCD HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n";
        assert!(request(&server, chunked).contains(r#""error":"length""#));
        let answer = request(&server, b"POST /wallpaper/upload?c=ABCD HTTP/1.1\r\n\r\n");
        assert!(answer.contains(r#""error":"length""#), "{answer}");
        server.stop_and_wait();
    }

    #[test]
    fn a_truncated_or_wrongly_sized_picture_is_refused_and_the_server_keeps_waiting() {
        let mut server = start_local("ABCD");
        let mut body = qoi(1920, 1080);
        body.truncate(body.len() / 2);
        let answer = request(&server, &upload_request("abcd", &body));
        assert!(answer.contains(r#""error":"read""#), "{answer}");
        let answer = request(&server, &upload_request("ABCD", &qoi(1280, 720)));
        assert!(answer.contains(r#""error":"read""#), "{answer}");
        // Still listening: a valid one goes through.
        let answer = request(&server, &upload_request("ABCD", &qoi(1920, 1080)));
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        assert!(answer.contains(r#"{"ok":true}"#));
        server.stop_and_wait();
        let all = events(&server);
        let refused = all
            .iter()
            .filter(|e| matches!(e, Event::Rejected(Reject::Unreadable)))
            .count();
        assert_eq!(refused, 2);
        let accepted = all.iter().find_map(|e| match e {
            Event::Accepted(image) => Some((image.width, image.height)),
            _ => None,
        });
        assert_eq!(accepted, Some((1920, 1080)));
    }

    #[test]
    fn two_uploads_in_a_row_are_both_accepted() {
        let mut server = start_local("ABCD");
        for _ in 0..2 {
            let answer = request(&server, &upload_request("ABCD", &qoi(1920, 1080)));
            assert!(answer.contains(r#"{"ok":true}"#), "{answer}");
        }
        server.stop_and_wait();
        let accepted = events(&server)
            .iter()
            .filter(|e| matches!(e, Event::Accepted(_)))
            .count();
        assert_eq!(accepted, 2);
    }

    #[test]
    fn a_second_upload_meanwhile_hears_that_the_tv_is_busy() {
        let mut server = start_local("ABCD");
        let body = qoi(1920, 1080);
        let head = format!(
            "POST /wallpaper/upload?c=ABCD HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        let mut first = TcpStream::connect(server.address()).unwrap();
        first.write_all(head.as_bytes()).unwrap();
        first.write_all(&body[..body.len() / 2]).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        // The second arrives while the first is half way; the next chunk of the first lets the
        // server answer it.
        let address = server.address();
        let second = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .write_all(&upload_request("ABCD", &qoi(64, 64)))
                .unwrap();
            let mut answer = String::new();
            let _ = stream.read_to_string(&mut answer);
            answer
        });
        std::thread::sleep(Duration::from_millis(200));
        first.write_all(&body[body.len() / 2..]).unwrap();
        let mut answer = String::new();
        let _ = first.read_to_string(&mut answer);
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        let busy = second.join().unwrap();
        assert!(busy.contains(r#""error":"busy""#), "{busy}");
        server.stop_and_wait();
    }

    #[test]
    fn an_upload_in_100_pieces_gives_at_most_4_progress_notices_a_second() {
        let mut server = start_local("ABCD");
        let body = qoi(1920, 1080);
        let head = format!(
            "POST /wallpaper/upload?c=ABCD HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        let started = Instant::now();
        let mut stream = TcpStream::connect(server.address()).unwrap();
        stream.write_all(head.as_bytes()).unwrap();
        for piece in body.chunks(body.len().div_ceil(100)) {
            stream.write_all(piece).unwrap();
            stream.flush().unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut answer = String::new();
        let _ = stream.read_to_string(&mut answer);
        let seconds = started.elapsed().as_secs_f32();
        server.stop_and_wait();
        let progress = events(&server)
            .iter()
            .filter(|e| matches!(e, Event::Receiving(_)))
            .count();
        assert!(progress >= 1);
        // The 0% at the start, then at most 4 a second.
        assert!(
            progress as f32 <= 1.0 + 4.0 * seconds.ceil(),
            "{progress} notices in {seconds} s"
        );
    }
}
