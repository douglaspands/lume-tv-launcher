//! The periodic reading of the network tool: type, address, gateway and Wi-Fi signal (D4, D5 in
//! the `ferramentas` design). Everything is local: nothing here opens a connection.

use crate::android::bridge::Bridge;
use crate::android::device_info;
use lume_core::app::tools::network::{
    parse_default_gateway, parse_route_dump, pick_gateway, NetworkReading,
};
use lume_core::device_info::Link;
use std::net::Ipv4Addr;

/// The kernel's main routing table, read when the netlink dump is refused. Android keeps the
/// default route of each network in a table of its own, so this one rarely has it.
const ROUTE_TABLE: &str = "/proc/net/route";

pub fn read(bridge: &Bridge) -> NetworkReading {
    let connection = device_info::network();
    let link = connection.map(|(link, _)| link);
    let gateway = netlink_gateway(link).or_else(|| {
        std::fs::read_to_string(ROUTE_TABLE)
            .ok()
            .and_then(|table| parse_default_gateway(&table))
    });
    // The signal is asked for only on Wi-Fi: on cable the section is not shown.
    let wifi = match connection {
        Some((Link::WiFi, _)) => bridge.wifi_signal(),
        _ => None,
    };
    NetworkReading {
        connection,
        gateway,
        wifi,
    }
}

/// The default gateway from a netlink route dump of every table (Rust, no Java), preferring the
/// route out of the interface the connection uses (`eth*` on cable, `wlan*` on Wi-Fi).
fn netlink_gateway(link: Option<Link>) -> Option<Ipv4Addr> {
    let dump = route_dump()?;
    let routes = parse_route_dump(&dump);
    let prefix = match link {
        Some(Link::Ethernet) => "eth",
        Some(Link::WiFi) => "wlan",
        _ => "",
    };
    pick_gateway(&routes, |index| {
        !prefix.is_empty() && interface_name(index).is_some_and(|name| name.starts_with(prefix))
    })
}

fn interface_name(index: u32) -> Option<String> {
    let mut buf = [0 as libc::c_char; libc::IF_NAMESIZE];
    // SAFETY: the buffer has IF_NAMESIZE bytes, as `if_indextoname` requires.
    let name = unsafe { libc::if_indextoname(index, buf.as_mut_ptr()) };
    if name.is_null() {
        return None;
    }
    // SAFETY: on success the buffer holds a NUL-terminated name.
    let name = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) };
    Some(name.to_string_lossy().into_owned())
}

/// Asks the kernel for every IPv4 route (`RTM_GETROUTE` with `NLM_F_DUMP`) and returns the raw
/// answer, up to the end of the dump. `None` when the socket is refused or nothing comes in time.
fn route_dump() -> Option<Vec<u8>> {
    const RTM_GETROUTE: u16 = 26;
    const NLMSG_DONE: u16 = 3;
    const NLMSG_ERROR: u16 = 2;
    const NLM_F_REQUEST: u16 = 0x1;
    const NLM_F_DUMP: u16 = 0x300;
    // SAFETY: plain socket call; the descriptor is closed by `Fd` below.
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_RAW | libc::SOCK_CLOEXEC,
            libc::NETLINK_ROUTE,
        )
    };
    if fd < 0 {
        return None;
    }
    struct Fd(libc::c_int);
    impl Drop for Fd {
        fn drop(&mut self) {
            // SAFETY: the descriptor is ours and closed once.
            unsafe { libc::close(self.0) };
        }
    }
    let fd = Fd(fd);
    // Never block the loop for long: the dump answers in microseconds.
    let timeout = libc::timeval {
        tv_sec: 0,
        tv_usec: 200_000,
    };
    // SAFETY: valid descriptor and a timeval of the right size.
    unsafe {
        libc::setsockopt(
            fd.0,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            (&timeout as *const libc::timeval).cast(),
            std::mem::size_of::<libc::timeval>() as libc::socklen_t,
        );
    }
    // nlmsghdr (16 bytes) + rtmsg (12 bytes) asking for AF_INET routes.
    let mut request = Vec::with_capacity(28);
    request.extend(28u32.to_ne_bytes());
    request.extend(RTM_GETROUTE.to_ne_bytes());
    request.extend((NLM_F_REQUEST | NLM_F_DUMP).to_ne_bytes());
    request.extend(1u32.to_ne_bytes());
    request.extend(0u32.to_ne_bytes());
    request.extend([libc::AF_INET as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // SAFETY: the buffer is valid for its length.
    let sent = unsafe { libc::send(fd.0, request.as_ptr().cast(), request.len(), 0) };
    if sent != request.len() as isize {
        return None;
    }
    let mut dump = Vec::new();
    let mut buf = vec![0u8; 16 * 1024];
    // A dump of a TV's routes is a few kilobytes; the cap keeps a broken answer from growing.
    while dump.len() < 256 * 1024 {
        // SAFETY: the buffer is valid for its length.
        let n = unsafe { libc::recv(fd.0, buf.as_mut_ptr().cast(), buf.len(), 0) };
        if n <= 0 {
            return None;
        }
        let chunk = &buf[..n as usize];
        dump.extend_from_slice(chunk);
        // Stop at the end of the dump (or an error), read from the message headers of this chunk.
        let mut at = 0;
        while at + 16 <= chunk.len() {
            let len = u32::from_ne_bytes([chunk[at], chunk[at + 1], chunk[at + 2], chunk[at + 3]])
                as usize;
            let kind = u16::from_ne_bytes([chunk[at + 4], chunk[at + 5]]);
            if kind == NLMSG_DONE {
                return Some(dump);
            }
            if kind == NLMSG_ERROR || len < 16 {
                return None;
            }
            at += (len + 3) & !3;
        }
    }
    None
}
