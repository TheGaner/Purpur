// SPDX-License-Identifier: AGPL-3.0-or-later
mod builtin;
mod command;
mod native;
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
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bind = String::from("127.0.0.1:25565");
    let mut motd = String::from("Purpur-Horizon in Rust (experimental)");
    let mut plugins_dir = PathBuf::from("plugins");
    let mut data_dir = PathBuf::from("data");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => bind = args.next().ok_or("missing --bind value")?,
            "--motd" => motd = args.next().ok_or("missing --motd value")?,
            "--data" => data_dir = PathBuf::from(args.next().ok_or("missing --data value")?),
            "--plugins" => {
                plugins_dir = PathBuf::from(args.next().ok_or("missing --plugins value")?)
            }
            "--help" => {
                println!("purpur-horizon --bind HOST:PORT --motd TEXT --plugins DIR --data DIR\nPrototype: server-list status/ping only; no playable world yet.");
                return Ok(());
            }
            _ => return Err(format!("unrecognized flag {arg}").into()),
        }
    }
    let listener = TcpListener::bind(&bind)?;
    listener.set_nonblocking(true)?;
    println!(
        "Purpur-Horizon standalone server at {} (MC 26.3 protocol {})",
        listener.local_addr()?,
        protocol::VERSION
    );
    println!("WARNING: gameplay login and Purpur parity NOT implemented");
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
    let mut next = Instant::now();
    let mut tick = 0u64;
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                let motd = motd.clone();
                thread::spawn(move || {
                    if let Err(e) = serve(socket, &motd) {
                        eprintln!("client: {e}");
                    }
                });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => eprintln!("accept: {e}"),
        }
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
fn serve(mut socket: TcpStream, motd: &str) -> io::Result<()> {
    use protocol::*;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.set_write_timeout(Some(Duration::from_secs(3)))?;
    let hs = frame_read(&mut socket)?;
    let mut c = Cursor::new(&hs);
    if c.varint()? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "handshake packet id",
        ));
    }
    let version = c.varint()?;
    c.text(255)?;
    c.bytes(2)?;
    let intent = c.varint()?;
    if c.remaining() != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "extra handshake bytes",
        ));
    }
    match intent {
        1 => {
            let req = frame_read(&mut socket)?;
            if req != [0] {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad status request",
                ));
            }
            let status = format!(
                r#"{{"version":{{"name":"Horizon-Purpur 26.3 prototype","protocol":{}}},"players":{{"max":0,"online":0}},"description":{{"text":{}}}}}"#,
                VERSION,
                json(motd)
            );
            let mut pkt = vec![];
            varint_write(0, &mut pkt);
            write_string(&status, &mut pkt);
            frame_write(&mut socket, &pkt)?;
            if let Ok(ping) = frame_read(&mut socket) {
                if ping.len() == 9 && ping[0] == 1 {
                    frame_write(&mut socket, &ping)?;
                }
            }
        }
        2 => {
            let why = if version != VERSION {
                format!("Requires Horizon 26.3 (protocol {VERSION})")
            } else {
                "Standalone Rust gameplay engine not implemented yet".to_string()
            };
            let mut pkt = vec![];
            varint_write(0, &mut pkt);
            write_string(&format!(r#"{{"text":{}}}"#, json(&why)), &mut pkt);
            frame_write(&mut socket, &pkt)?;
        }
        _ => {}
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
    fn status_ping() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let job = thread::spawn(move || {
            let (socket, _) = server.accept().unwrap();
            serve(socket, "Horizon \"only\"").unwrap();
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
            serve(socket, "test").unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        let mut hs = vec![0];
        protocol::varint_write(777, &mut hs);
        protocol::write_string("localhost", &mut hs);
        hs.extend([0, 0, 2]);
        protocol::frame_write(&mut client, &hs).unwrap();
        let reply = protocol::frame_read(&mut client).unwrap();
        let mut c = protocol::Cursor::new(&reply);
        assert_eq!(c.varint().unwrap(), 0);
        assert!(c.text(200).unwrap().contains("not implemented"));
        job.join().unwrap();
    }
}
