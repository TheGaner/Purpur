// SPDX-License-Identifier: AGPL-3.0-or-later
mod builtin;
mod command;
mod native;
mod network;
mod plugins;
mod protocol;

// Limit console command ingress and work per tick. Player command and
// suggestion queues will have separate capacities when login is implemented.
const MAX_CONSOLE_COMMANDS_QUEUED: usize = 1024;
const MAX_CONSOLE_COMMANDS_PER_TICK: usize = 128;

use std::{
    env,
    io::{self, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bind = String::from("127.0.0.1:25565");
    let mut motd = String::from("Purpur-Horizon in Rust (experimental)");
    let mut plugins_dir = PathBuf::from("plugins");
    let mut data_dir = PathBuf::from("data");
    let mut max_connections: usize = 64;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => bind = args.next().ok_or("missing --bind value")?,
            "--motd" => motd = args.next().ok_or("missing --motd value")?,
            "--data" => data_dir = PathBuf::from(args.next().ok_or("missing --data value")?),
            "--max-connections" => {
                max_connections = args
                    .next()
                    .ok_or("missing --max-connections value")?
                    .parse()?;
                if !(1..=1024).contains(&max_connections) {
                    return Err("--max-connections must be between 1 and 1024".into());
                }
            }
            "--plugins" => {
                plugins_dir = PathBuf::from(args.next().ok_or("missing --plugins value")?)
            }
            "--help" => {
                println!("purpur-horizon --bind HOST:PORT --motd TEXT --plugins DIR --data DIR --max-connections N\nPrototype: server-list status/ping only; no playable world yet.");
                return Ok(());
            }
            _ => return Err(format!("unrecognized flag {arg}").into()),
        }
    }
    let listener = TcpListener::bind(&bind)?;
    println!(
        "Purpur-Horizon standalone server at {} (MC 26.3 protocol {})",
        listener.local_addr()?,
        protocol::VERSION
    );
    println!("WARNING: gameplay login and Purpur parity NOT implemented");
    // Status players=0 and MOTD cannot change while the prototype is running.
    // Preframe once, then share immutable bytes among concurrent connections.
    // Dynamic player counts will later use a versioned, refreshed snapshot.
    let status_reply = Arc::new(make_status_response(&motd));
    let mut plugins = plugins::Plugins::load(&plugins_dir);
    println!("{}", plugins.info());
    let mut commands = command::Registry::builtins();
    builtin::register(&mut commands);
    plugins.register_commands(&mut commands);
    // Always-on native services. Refuse corrupt persisted state rather than
    // silently resetting access rules or player homes/warps.
    let mut permissions =
        builtin::permissions::NativePermissions::open(data_dir.join("permissions.json"))?;
    let mut utilities = builtin::utilities::NativeUtilities::open(data_dir.join("utilities.json"))?;
    println!("Native Rust permissions and utilities: enabled");
    let started = Instant::now();
    let (tx, rx) = mpsc::sync_channel(MAX_CONSOLE_COMMANDS_QUEUED);
    thread::spawn(move || {
        let mut s = String::new();
        loop {
            s.clear();
            if io::stdin().read_line(&mut s).unwrap_or(0) == 0 {
                break;
            }
            if tx.send(s.trim().to_string()).is_err() {
                break;
            }
        }
    });
    let slots = network::Slots::new(max_connections);
    // Blocking accept runs independently from the 20Hz game tick. No 50ms
    // queueing latency for status/login connections, and no accept budget can
    // monopolize a simulation tick. Use bounded admission before spawning.
    let _accept_thread = thread::Builder::new()
        .name("horizon-accept".into())
        .spawn({
            let status_reply = Arc::clone(&status_reply);
            move || loop {
                match listener.accept() {
                    Ok((socket, _)) => {
                        let Some(permit) = slots.enter() else {
                            drop(socket);
                            continue;
                        };
                        let reply = Arc::clone(&status_reply);
                        if let Err(error) = thread::Builder::new()
                            .name("horizon-client-handshake".into())
                            .spawn(move || {
                                let _permit = permit;
                                if let Err(error) = serve(socket, &reply) {
                                    eprintln!("client: {error}");
                                }
                            })
                        {
                            eprintln!("could not spawn handshake worker: {error}");
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        eprintln!("connection accept loop stopped: {e}");
                        break;
                    }
                }
            }
        })?;
    let mut next = Instant::now();
    let mut tick = 0u64;
    loop {
        for cmd in rx.try_iter().take(MAX_CONSOLE_COMMANDS_PER_TICK) {
            let plugins_info = plugins.info();
            let snapshot = command::Snapshot {
                version: "Purpur-Horizon 0.1.0 (command prototype)",
                plugins: &plugins_info,
                tick,
                uptime: started.elapsed(),
            };
            match commands.execute(&cmd, &command::Source::Console, &permissions, &snapshot) {
                Ok(reply) => {
                    for line in reply.lines {
                        println!("{line}");
                    }
                    match reply.effect {
                        command::Effect::None => {}
                        command::Effect::Stop => return Ok(()),
                        command::Effect::Core { command, args } => {
                            match builtin::dispatch(
                                &command,
                                &args,
                                &mut permissions,
                                &mut utilities,
                            ) {
                                Ok(message) => println!("{message}"),
                                Err(error) => eprintln!("[native] {error}"),
                            }
                        }
                        command::Effect::Plugin {
                            owner,
                            command,
                            args,
                        } => {
                            plugins.execute_command(&owner, &command, &args, tick);
                        }
                    }
                }
                Err(command::Error::Empty) => {}
                Err(error) => eprintln!("[command] {error}"),
            }
        }
        plugins.event(2, tick, "");
        tick += 1;
        next += Duration::from_millis(50);
        if let Some(delay) = next.checked_duration_since(Instant::now()) {
            thread::sleep(delay)
        } else {
            next = Instant::now();
        }
    }
}
/// Complete status packet (length VarInt + packet ID + JSON payload) built
/// once per startup. Login replies stay dynamic and framed independently.
fn make_status_response(motd: &str) -> Vec<u8> {
    let status = format!(
        r#"{{"version":{{"name":"Horizon-Purpur 26.3 prototype","protocol":{}}},"players":{{"max":0,"online":0}},"description":{{"text":{}}}}}"#,
        protocol::VERSION,
        protocol::json(motd)
    );
    let mut payload = Vec::with_capacity(status.len() + 8);
    protocol::varint_write(0, &mut payload);
    protocol::write_string(&status, &mut payload);
    let mut framed = Vec::with_capacity(payload.len() + 5);
    protocol::varint_write(payload.len() as i32, &mut framed);
    framed.extend_from_slice(&payload);
    framed
}
fn serve(mut socket: TcpStream, status_reply: &[u8]) -> io::Result<()> {
    use protocol::*;
    // One total admission window shared by all preauth protocol states.
    let deadline = Instant::now() + Duration::from_secs(3);
    socket.set_write_timeout(Some(Duration::from_secs(3)))?;
    // 255-char hostname (worst-case 3-byte UTF-8), packet ID, version,
    // port and next-state VarInts fit under 1024 bytes.
    let hs = frame_read_limited(
        &mut network::DeadlineReader::new(&mut socket, deadline),
        1024,
    )?;
    let intention = network::parse_intention(&hs)?;
    match intention.state {
        network::State::Status => {
            let req =
                frame_read_limited(&mut network::DeadlineReader::new(&mut socket, deadline), 1)?;
            if req != [0] {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad status request",
                ));
            }
            socket.write_all(status_reply)?;
            // The optional ping is independent of the status response. Do
            // not occupy a scarce handshake worker for the entire 3s budget
            // after already answering a status-only peer. Give genuine
            // status/Ping clients a separate bounded 1s opportunity to ping.
            let ping_deadline = deadline.min(Instant::now() + Duration::from_secs(1));
            if let Ok(ping) = frame_read_limited(
                &mut network::DeadlineReader::new(&mut socket, ping_deadline),
                9,
            ) {
                if ping.len() == 9 && ping[0] == 1 {
                    frame_write(&mut socket, &ping)?;
                }
            }
        }
        network::State::Login | network::State::Transfer => {
            // Reject incompatible protocol versions before waiting for a login
            // hello, preventing an avoidable timeout on malformed old clients.
            if intention.version != VERSION {
                frame_write(
                    &mut socket,
                    &network::login_disconnect(&format!(
                        "Requires Horizon 26.3 (protocol {VERSION})"
                    )),
                )?;
                return Ok(());
            }
            // Parse the real protocol-777 login hello, including client-provided
            // UUID, but do NOT accept, authenticate, or store that identity yet.
            // Login hello ID + 16-char name + 16-byte UUID.
            let hello_packet =
                frame_read_limited(&mut network::DeadlineReader::new(&mut socket, deadline), 64)?;
            let hello = network::parse_login_hello(&hello_packet);
            let reason = if hello.is_err() {
                "Invalid login hello for Minecraft 26.3".to_owned()
            } else {
                "Horizon-Purpur Rust: authentication and gameplay not implemented yet".into()
            };
            frame_write(&mut socket, &network::login_disconnect(&reason))?;
        }
    }
    let _ = socket.flush();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_console_work_never_drains_all_pending_commands_in_one_tick() {
        let (sender, receiver) = mpsc::sync_channel::<String>(MAX_CONSOLE_COMMANDS_QUEUED);
        for n in 0..200 {
            sender.try_send(n.to_string()).unwrap();
        }
        let first_tick: Vec<_> = receiver
            .try_iter()
            .take(MAX_CONSOLE_COMMANDS_PER_TICK)
            .collect();
        assert_eq!(first_tick.len(), MAX_CONSOLE_COMMANDS_PER_TICK);
        assert_eq!(first_tick[0], "0");
        let remaining: Vec<_> = receiver.try_iter().collect();
        assert_eq!(remaining.len(), 72);
        assert_eq!(remaining[0], "128");
    }
    #[test]
    fn bounded_console_queue_rejects_excess_without_growth() {
        let (sender, _receiver) = mpsc::sync_channel::<u8>(2);
        sender.try_send(1).unwrap();
        sender.try_send(2).unwrap();
        assert!(matches!(
            sender.try_send(3),
            Err(mpsc::TrySendError::Full(3))
        ));
    }
    #[test]
    fn preframed_status_matches_protocol_framing() {
        let framed = make_status_response(r#"Horizon "optimized""#);
        let mut reader = &framed[..];
        let packet = protocol::frame_read_limited(&mut reader, 1024).unwrap();
        assert!(reader.is_empty());
        let mut c = protocol::Cursor::new(&packet);
        assert_eq!(c.varint().unwrap(), 0);
        let json = c.text(1024).unwrap();
        assert!(json.contains(r#"Horizon \"optimized\""#));
        assert_eq!(c.remaining(), 0);
    }
    #[test]
    fn oversized_preauth_handshake_rejected_on_wire_without_payload() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, &make_status_response("hello"))
                .unwrap_err()
                .kind()
        });
        let mut client = TcpStream::connect(address).unwrap();
        let mut length = vec![];
        protocol::varint_write(1024 * 1024, &mut length);
        client.write_all(&length).unwrap();
        assert_eq!(handle.join().unwrap(), io::ErrorKind::InvalidData);
    }
    #[test]
    fn mismatched_protocol_rejected_without_login_hello() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let job = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, &make_status_response("test")).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut hs = vec![0];
        protocol::varint_write(776, &mut hs);
        protocol::write_string("localhost", &mut hs);
        hs.extend([0x63, 0xdd, 2]);
        protocol::frame_write(&mut client, &hs).unwrap();
        let reply = protocol::frame_read_limited(&mut client, 256).unwrap();
        let mut c = protocol::Cursor::new(&reply);
        assert_eq!(c.varint().unwrap(), 0);
        assert!(c.text(256).unwrap().contains("protocol 777"));
        job.join().unwrap();
    }
    #[test]
    fn status_only_peer_releases_worker_without_three_second_ping_hold() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let start = Instant::now();
        let job = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, &make_status_response("test")).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut handshake = vec![0];
        protocol::varint_write(777, &mut handshake);
        protocol::write_string("localhost", &mut handshake);
        handshake.extend([0x63, 0xdd, 1]);
        protocol::frame_write(&mut client, &handshake).unwrap();
        protocol::frame_write(&mut client, &[0]).unwrap();
        let response = protocol::frame_read_limited(&mut client, 1024).unwrap();
        let mut c = protocol::Cursor::new(&response);
        assert_eq!(c.varint().unwrap(), 0);
        assert!(c.text(1024).unwrap().contains("protocol"));
        // Deliberately keep the connection open and never send Ping.
        job.join().unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(1700),
            "status-only peer occupied a worker too long: {:?}",
            start.elapsed()
        );
    }
    #[test]
    fn delayed_status_ping_within_grace_echoes_payload() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let job = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, &make_status_response("test")).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut hs = vec![0];
        protocol::varint_write(777, &mut hs);
        protocol::write_string("localhost", &mut hs);
        hs.extend([0x63, 0xdd, 1]);
        protocol::frame_write(&mut client, &hs).unwrap();
        protocol::frame_write(&mut client, &[0]).unwrap();
        let _ = protocol::frame_read_limited(&mut client, 1024).unwrap();
        thread::sleep(Duration::from_millis(180));
        let mut ping = vec![1];
        ping.extend((-1234i64).to_be_bytes());
        protocol::frame_write(&mut client, &ping).unwrap();
        assert_eq!(protocol::frame_read_limited(&mut client, 9).unwrap(), ping);
        job.join().unwrap();
    }
    #[test]
    fn status_ping() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let job = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, &make_status_response("Horizon \"only\"")).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        let mut hs = vec![0];
        protocol::varint_write(777, &mut hs);
        protocol::write_string("localhost", &mut hs);
        hs.extend([0x63, 0xdd, 1]);
        protocol::frame_write(&mut client, &hs).unwrap();
        protocol::frame_write(&mut client, &[0]).unwrap();
        let status = protocol::frame_read(&mut client).unwrap();
        let mut c = protocol::Cursor::new(&status);
        assert_eq!(c.varint().unwrap(), 0);
        assert!(c.text(10000).unwrap().contains(r#"Horizon \"only\""#));
        let mut ping = vec![1];
        ping.extend(123i64.to_be_bytes());
        protocol::frame_write(&mut client, &ping).unwrap();
        assert_eq!(protocol::frame_read(&mut client).unwrap(), ping);
        job.join().unwrap();
    }
    #[test]
    fn login_not_yet_supported() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let job = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, &make_status_response("test")).unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        let mut hs = vec![0];
        protocol::varint_write(777, &mut hs);
        protocol::write_string("localhost", &mut hs);
        hs.extend([0, 0, 2]);
        protocol::frame_write(&mut client, &hs).unwrap();
        let mut login = vec![0];
        protocol::write_string("TestUser", &mut login);
        login.extend([0; 16]);
        protocol::frame_write(&mut client, &login).unwrap();
        let reply = protocol::frame_read(&mut client).unwrap();
        let mut c = protocol::Cursor::new(&reply);
        assert_eq!(c.varint().unwrap(), 0);
        assert!(c.text(200).unwrap().contains("not implemented"));
        job.join().unwrap();
    }
}
