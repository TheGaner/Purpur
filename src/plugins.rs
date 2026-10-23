use crate::native;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
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
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("java/HorizonJavaBridge.java");
            match Command::new("java")
                .arg("--source")
                .arg("21")
                .arg(source)
                .arg(&jar_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
            {
                Ok(child) => Some(child),
                Err(e) => {
                    eprintln!("[java] JVM bridge unavailable: {e}");
                    None
                }
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
            if let Some(stdin) = &mut child.stdin {
                let _ = writeln!(stdin, "STOP");
            }
            let _ = child.wait();
        }
    }
}
