// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pre-play protocol stage: bounded handshake/login parsing and connection admission.
//! A valid login hello is NOT authenticated and MUST NOT be admitted as a player.
use crate::protocol::{self, Cursor};
use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    net::{IpAddr, TcpStream},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
/// One absolute pre-authentication deadline across every frame and byte.
/// Ordinary SO_RCVTIMEO resets for each read; slow peers could otherwise
/// occupy one handshake slot indefinitely by sending a byte just in time.
pub struct DeadlineReader<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
}
impl<'a> DeadlineReader<'a> {
    pub fn new(stream: &'a mut TcpStream, deadline: Instant) -> Self {
        Self { stream, deadline }
    }
}
impl io::Read for DeadlineReader<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                "pre-authentication deadline expired",
            ));
        }
        // A nonzero TCP timeout is required by the OS; deadline is checked
        // again before each subsequent read.
        self.stream
            .set_read_timeout(Some(remaining.max(Duration::from_millis(1))))?;
        io::Read::read(self.stream, out)
    }
}
/// Use the same absolute pre-authentication deadline for outbound writes.
/// A fixed SO_SNDTIMEO on its own restarts for each partial write.
pub struct DeadlineWriter<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
}
impl<'a> DeadlineWriter<'a> {
    pub fn new(stream: &'a mut TcpStream, deadline: Instant) -> Self {
        Self { stream, deadline }
    }
}
impl io::Write for DeadlineWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                "pre-authentication write deadline expired",
            ));
        }
        // The OS requires a nonzero timeout. Each additional partial write
        // rechecks the shared monotonic deadline, not a fresh three seconds.
        self.stream
            .set_write_timeout(Some(remaining.max(Duration::from_millis(1))))?;
        io::Write::write(self.stream, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(self.stream)
    }
}
fn bad(detail: &'static str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, detail)
}
pub struct Intention {
    pub version: i32,
    pub state: State,
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    Status,
    Login,
    Transfer,
}
pub fn parse_intention(body: &[u8]) -> io::Result<Intention> {
    let mut cur = Cursor::new(body);
    if cur.varint()? != 0 {
        return Err(bad("unexpected handshake packet"));
    }
    let version = cur.varint()?;
    // Horizon StrN<255> counts Unicode characters, while its bounded
    // UTF-8 encoding permits up to 3x the character count in bytes.
    let host = cur.text(255 * 3)?;
    if host.is_empty() || host.chars().count() > 255 {
        return Err(bad("invalid handshake host"));
    }
    let _port = u16::from_be_bytes(cur.bytes(2)?.try_into().map_err(|_| bad("invalid port"))?);
    let state = match cur.varint()? {
        1 => State::Status,
        2 => State::Login,
        3 => State::Transfer,
        _ => return Err(bad("unsupported next state")),
    };
    if cur.remaining() != 0 {
        return Err(bad("extra handshake bytes"));
    }
    Ok(Intention { version, state })
}
#[derive(Debug, PartialEq, Eq)]
pub struct LoginHello<'a> {
    pub name: &'a str,
    /// Client-declared UUID is NEVER an authenticated identity.
    pub claimed_uuid: [u8; 16],
}
pub fn parse_login_hello(packet: &[u8]) -> io::Result<LoginHello<'_>> {
    let mut cur = Cursor::new(packet);
    if cur.varint()? != 0 {
        return Err(bad("expected serverbound login hello"));
    }
    // 26.3 login ServerHello = name StrN<16> plus fixed 128-bit UUID.
    // Horizon's LAN reference allows printable ASCII (1..=16 bytes).
    let name = cur.text(16)?;
    if name.is_empty() || !name.bytes().all(|b| b > 32 && b < 127) {
        return Err(bad("invalid player name"));
    }
    let claimed_uuid: [u8; 16] = cur
        .bytes(16)?
        .try_into()
        .map_err(|_| bad("invalid login UUID"))?;
    if cur.remaining() != 0 {
        return Err(bad("extra login hello bytes"));
    }
    Ok(LoginHello { name, claimed_uuid })
}
/// Minecraft login ClientboundDisconnect packet ID 0 (uncompressed, pre-auth).
pub fn login_disconnect(text: &str) -> Vec<u8> {
    let mut packet = Vec::new();
    protocol::varint_write(0, &mut packet);
    protocol::write_string(
        &format!(r#"{{"text":{}}}"#, protocol::json(text)),
        &mut packet,
    );
    packet
}
/// RAII admission limits both total preauth workers and per-TCP-source
/// occupancy. Tracking ends with the last live permit; no per-IP history.
pub struct Slots {
    state: Mutex<SlotState>,
    limit: usize,
    per_ip_limit: usize,
}
#[derive(Default)]
struct SlotState {
    used: usize,
    per_ip: HashMap<IpAddr, usize>,
}
pub struct Permit {
    slots: Arc<Slots>,
    ip: IpAddr,
}
fn canonical_peer(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}
impl Slots {
    pub fn new(limit: usize, per_ip_limit: usize) -> Arc<Self> {
        assert!(limit > 0 && per_ip_limit > 0);
        Arc::new(Self {
            state: Mutex::new(SlotState::default()),
            limit,
            per_ip_limit,
        })
    }
    /// Peer IP comes from TcpListener::accept, never a handshake hostname.
    pub fn enter(self: &Arc<Self>, source: IpAddr) -> Option<Permit> {
        let ip = canonical_peer(source);
        let mut state = self.state.lock().unwrap();
        if state.used >= self.limit
            || state.per_ip.get(&ip).copied().unwrap_or_default() >= self.per_ip_limit
        {
            return None;
        }
        state.used += 1;
        *state.per_ip.entry(ip).or_insert(0) += 1;
        Some(Permit {
            slots: self.clone(),
            ip,
        })
    }
    pub fn used(&self) -> usize {
        self.state.lock().unwrap().used
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        let mut state = self.slots.state.lock().unwrap();
        state.used -= 1;
        let peers = state.per_ip.get_mut(&self.ip).expect("permit peer tracked");
        *peers -= 1;
        if *peers == 0 {
            state.per_ip.remove(&self.ip);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn hs(protocol_version: i32, state: i32) -> Vec<u8> {
        let mut out = vec![0];
        protocol::varint_write(protocol_version, &mut out);
        protocol::write_string("localhost", &mut out);
        out.extend(25565u16.to_be_bytes());
        protocol::varint_write(state, &mut out);
        out
    }
    #[test]
    fn parse_handshake_intents_and_reject_extra() {
        assert_eq!(parse_intention(&hs(777, 2)).unwrap().state, State::Login);
        assert_eq!(parse_intention(&hs(777, 3)).unwrap().state, State::Transfer);
        assert_eq!(parse_intention(&hs(777, 1)).unwrap().state, State::Status);
        let mut bad = hs(777, 2);
        bad.push(0);
        assert!(parse_intention(&bad).is_err());
        assert!(parse_intention(&hs(777, 99)).is_err());
        assert!(parse_intention(&[]).is_err());
    }
    #[test]
    fn handshake_host_uses_protocol_character_limit_not_byte_limit() {
        let mut international = vec![0];
        protocol::varint_write(777, &mut international);
        protocol::write_string(&"é".repeat(255), &mut international);
        international.extend(25565u16.to_be_bytes());
        protocol::varint_write(2, &mut international);
        assert_eq!(parse_intention(&international).unwrap().state, State::Login);
        let mut overlong = vec![0];
        protocol::varint_write(777, &mut overlong);
        protocol::write_string(&"x".repeat(256), &mut overlong);
        overlong.extend(25565u16.to_be_bytes());
        protocol::varint_write(2, &mut overlong);
        assert!(parse_intention(&overlong).is_err());
    }
    fn login(name: &str) -> Vec<u8> {
        let mut out = vec![0];
        protocol::write_string(name, &mut out);
        out.extend([13; 16]);
        out
    }
    #[test]
    fn reads_login_profile_without_trusting_uuid() {
        let hello = login("Horizon");
        let parsed = parse_login_hello(&hello).unwrap();
        assert_eq!(parsed.name, "Horizon");
        assert_eq!(parsed.claimed_uuid, [13; 16]);
    }
    #[test]
    fn rejects_malformed_login_names_and_trailing_bytes() {
        assert!(parse_login_hello(&login("")).is_err());
        assert!(parse_login_hello(&login("contains spaces")).is_err());
        assert!(parse_login_hello(&login("☃")).is_err());
        assert!(parse_login_hello(&login("abcdefghijklmnopq")).is_err());
        assert!(parse_login_hello(&login("good")[..8]).is_err());
        let mut extra = login("good");
        extra.push(1);
        assert!(parse_login_hello(&extra).is_err());
        let mut wrong = login("good");
        wrong[0] = 3;
        assert!(parse_login_hello(&wrong).is_err());
    }
    #[test]
    fn send_login_disconnect_is_json_packet_0() {
        let packet = login_disconnect("Use Horizon \"26.3\"");
        let mut c = Cursor::new(&packet);
        assert_eq!(c.varint().unwrap(), 0);
        assert_eq!(c.text(32767).unwrap(), r#"{"text":"Use Horizon \"26.3\""}"#);
        assert_eq!(c.remaining(), 0);
    }
    #[test]
    fn expired_writer_refuses_writable_socket_without_sending() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let mut writer =
            DeadlineWriter::new(&mut server, Instant::now() - Duration::from_millis(1));
        assert_eq!(
            writer.write_all(b"blocked").unwrap_err().kind(),
            ErrorKind::TimedOut
        );
        assert!(matches!(
            peer.read(&mut [0; 16]).unwrap_err().kind(),
            ErrorKind::TimedOut | ErrorKind::WouldBlock
        ));
    }
    #[test]
    fn stalled_peer_cannot_extend_total_write_budget() {
        use std::io::Write;
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        // Keep the receiver alive but never read: a 16 MiB response cannot
        // normally fit into the loopback TCP send buffers.
        let _peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let started = Instant::now();
        let mut writer = DeadlineWriter::new(&mut server, started + Duration::from_millis(250));
        let error = writer.write_all(&vec![0u8; 16 * 1024 * 1024]).unwrap_err();
        assert!(
            matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock),
            "{error}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "blocked writer exceeded reasonable deadline tolerance: {:?}",
            started.elapsed()
        );
    }
    #[test]
    fn unexpired_writer_sends_valid_framed_reply() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut writer = DeadlineWriter::new(&mut server, Instant::now() + Duration::from_secs(1));
        protocol::frame_write(&mut writer, &[1, 42, 77]).unwrap();
        assert_eq!(
            protocol::frame_read_limited(&mut peer, 3).unwrap(),
            vec![1, 42, 77]
        );
    }
    #[test]
    fn expired_deadline_never_reads_available_bytes() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        use std::io::Write;
        client.write_all(&[8]).unwrap();
        let mut read = DeadlineReader::new(&mut server, Instant::now() - Duration::from_millis(1));
        assert_eq!(
            io::Read::read(&mut read, &mut [0]).unwrap_err().kind(),
            ErrorKind::TimedOut
        );
    }
    #[test]
    fn slow_drip_has_global_deadline_not_per_byte_timeout() {
        use std::io::Write;
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let writer = std::thread::spawn(move || {
            // A valid frame len 5, but the body arrives one byte every 250ms.
            client.write_all(&[5]).unwrap();
            for _ in 0..5 {
                std::thread::sleep(Duration::from_millis(250));
                if client.write_all(&[1]).is_err() {
                    break;
                }
            }
        });
        let started = Instant::now();
        let mut reader = DeadlineReader::new(&mut server, started + Duration::from_millis(430));
        let error = protocol::frame_read_limited(&mut reader, 8).unwrap_err();
        assert!(matches!(
            error.kind(),
            ErrorKind::TimedOut | ErrorKind::WouldBlock
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(reader);
        drop(server);
        writer.join().unwrap();
    }
    #[test]
    fn connection_slots_released_on_return() {
        let slots = Slots::new(2, 2);
        let peer: IpAddr = "192.0.2.1".parse().unwrap();
        let a = slots.enter(peer).unwrap();
        let b = slots.enter(peer).unwrap();
        assert!(slots.enter(peer).is_none());
        assert_eq!(slots.used(), 2);
        drop(a);
        assert_eq!(slots.used(), 1);
        let c = slots.enter(peer).unwrap();
        drop(b);
        drop(c);
        assert_eq!(slots.used(), 0);
    }
    #[test]
    fn admission_across_threads_never_exceeds_limit() {
        use std::sync::{Barrier, Mutex};
        use std::thread;
        let slots = Slots::new(4, 2);
        let start = Arc::new(Barrier::new(16));
        let release = Arc::new(Barrier::new(16));
        let held = Arc::new(Mutex::new(Vec::new()));
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let (s, b, e, h) = (slots.clone(), start.clone(), release.clone(), held.clone());
                thread::spawn(move || {
                    b.wait();
                    let peer = "192.0.2.1".parse().unwrap();
                    let permit = s.enter(peer);
                    if permit.is_some() {
                        h.lock().unwrap().push(1);
                    }
                    e.wait();
                    permit
                })
            })
            .collect();
        for h in handles {
            let _ = h.join().unwrap();
        }
        assert!(held.lock().unwrap().len() <= 2);
        assert_eq!(slots.used(), 0);
    }
    #[test]
    fn per_ip_limit_preserves_other_peer_and_recovers() {
        let slots = Slots::new(3, 1);
        let a: IpAddr = "192.0.2.1".parse().unwrap();
        let b: IpAddr = "192.0.2.2".parse().unwrap();
        let c: IpAddr = "2001:db8::3".parse().unwrap();
        let first = slots.enter(a).unwrap();
        assert!(slots.enter(a).is_none());
        let second = slots.enter(b).expect("other source retains capacity");
        let third = slots.enter(c).expect("global slot 3 available");
        assert_eq!(slots.used(), 3);
        assert!(slots.enter("2001:db8::4".parse().unwrap()).is_none());
        drop(first);
        assert!(slots.enter(a).is_some());
        drop(second);
        drop(third);
        assert_eq!(slots.used(), 0);
    }
    #[test]
    fn ipv4_mapped_ipv6_cannot_bypass_per_ip_quota() {
        let slots = Slots::new(3, 1);
        let ipv4: IpAddr = "192.0.2.40".parse().unwrap();
        let mapped: IpAddr = "::ffff:192.0.2.40".parse().unwrap();
        let first = slots.enter(ipv4).unwrap();
        assert!(slots.enter(mapped).is_none());
        drop(first);
        assert!(slots.enter(mapped).is_some());
    }
    #[test]
    fn worker_panic_releases_peer_and_global_permits() {
        let slots = Slots::new(2, 1);
        let peer: IpAddr = "192.0.2.99".parse().unwrap();
        let permit = slots.enter(peer).unwrap();
        let worker = std::thread::spawn(move || {
            let _permit = permit;
            panic!("intentional test of RAII unwind");
        });
        assert!(worker.join().is_err());
        assert_eq!(slots.used(), 0);
        assert!(slots.enter(peer).is_some());
    }
}
