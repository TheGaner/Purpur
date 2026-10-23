// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{command, native};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
pub struct Plugins {
    #[cfg(unix)]
    rust: Vec<native::Plugin>,
    java: Option<Child>,
}
impl Plugins {
    pub fn load(base: &Path) -> Self {
        #[cfg(unix)]
        let mut rust = vec![];
        #[cfg(unix)]
        if let Ok(files) = fs::read_dir(base.join("rust")) {
            for entry in files.flatten() {
                if entry.path().extension().is_some_and(|s| s == "so") {
                    match native::Plugin::load(&entry.path()) {
                        Ok(p) => {
                            println!("[rust] loaded {}", p.name);
                            rust.push(p)
                        }
                        Err(e) => eprintln!("[rust] plugin failed: {e}"),
                    }
                }
            }
        }
        let jar_dir = base.join("java");
        let has_jar = fs::read_dir(&jar_dir)
            .map(|iter| {
                iter.flatten()
                    .any(|e| e.path().extension().is_some_and(|s| s == "jar"))
            })
            .unwrap_or(false);
        let java = if has_jar {
            // Embed the Java loader in the Rust binary so deployment does not
            // depend on the build machine, Horizon checkout or source directory.
            let runtime = base.join(".runtime");
            let source = runtime.join("HorizonJavaBridge.java");
            let prepared = fs::create_dir_all(&runtime)
                .and_then(|()| fs::write(&source, include_str!("../java/HorizonJavaBridge.java")));
            if let Err(ref error) = prepared {
                eprintln!("[java] cannot prepare embedded bridge: {error}");
            }
            let mut command = Command::new("java");
            command
                .arg("--source")
                .arg("21")
                .arg(source)
                .arg(&jar_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            if prepared.is_ok() {
                match command.spawn() {
                    Ok(child) => Some(child),
                    Err(e) => {
                        eprintln!("[java] JVM bridge unavailable: {e}");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let mut p = Self {
            #[cfg(unix)]
            rust,
            java,
        };
        p.event(1, 0, "");
        p
    }
    pub fn register_commands(&self, registry: &mut command::Registry) {
        #[cfg(unix)]
        for plugin in &self.rust {
            if let Some(manifest) = plugin.declared_commands() {
                for line in manifest.lines().take(64) {
                    let parts: Vec<_> = line.split('|').collect();
                    if parts.len() != 3 {
                        eprintln!("[rust] invalid command declaration in {}", plugin.name);
                        continue;
                    }
                    let aliases = if parts[2].is_empty() {
                        vec![]
                    } else {
                        parts[2].split(',').map(str::to_owned).collect()
                    };
                    let spec = command::Command {
                        name: parts[0].to_owned(),
                        aliases,
                        permission: parts[1].to_owned(),
                        description: format!("Plugin command from {}", plugin.name),
                        parameters: vec![],
                        handler: command::Handler::Plugin {
                            owner: plugin.name.clone(),
                        },
                    };
                    if let Err(error) = registry.register(spec) {
                        eprintln!("[rust] {}: {error}", plugin.name);
                    }
                }
            }
        }
        #[cfg(not(unix))]
        let _ = registry;
    }
    pub fn execute_command(&mut self, owner: &str, command: &str, args: &str, tick: u64) {
        #[cfg(unix)]
        {
            if let Some(plugin) = self.rust.iter().find(|p| p.name == owner) {
                let text = if args.is_empty() {
                    command.to_string()
                } else {
                    format!("{command} {args}")
                };
                plugin.event(3, tick, &text);
                return;
            }
        }
        eprintln!("[command] no loaded plugin owns {owner}:{command}");
    }
    pub fn event(&mut self, code: u32, tick: u64, payload: &str) {
        #[cfg(unix)]
        for plugin in &self.rust {
            plugin.event(code, tick, payload)
        }
        if let Some(child) = &mut self.java {
            if let Some(stdin) = &mut child.stdin {
                let value = payload.replace(['\n', '\r', '\t'], " ");
                if code == 3 {
                    let _ = writeln!(stdin, "COMMAND\t{value}");
                }
            }
        }
    }
    pub fn info(&self) -> String {
        #[cfg(unix)]
        let count = self.rust.len();
        #[cfg(not(unix))]
        let count = 0;
        format!(
            "Rust plugins: {count}, Java bridge: {}",
            if self.java.is_some() { "active" } else { "off" }
        )
    }
}
impl Drop for Plugins {
    fn drop(&mut self) {
        self.event(4, 0, "");
        if let Some(mut child) = self.java.take() {
            // EOF is the normal, non-blocking shutdown request for the bridge.
            // A plugin can hang in onDisable; never hang the Rust server's Drop.
            drop(child.stdin.take());
            let deadline = Instant::now() + Duration::from_millis(1500);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Err(error) => {
                        eprintln!("[java] bridge wait failed: {error}");
                        break;
                    }
                    Ok(None) if Instant::now() >= deadline => {
                        eprintln!("[java] bridge shutdown timed out; terminating child");
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(15)),
                }
            }
        }
    }
}
