// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native utilities service inspired by common Essentials-style functionality.
//! Pure data layer only: world validation, teleport, economy and events are not implemented.
use super::permissions::{atomic_json, lock_store, StoreLock};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

const MAX_SIZE: u64 = 2 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Position {
    pub fn new(world: &str, x: f64, y: f64, z: f64) -> io::Result<Self> {
        if world.is_empty()
            || world.len() > 128
            || !world
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_:.-".contains(&b))
            || ![x, y, z].iter().all(|v| v.is_finite())
            || x.abs() > 30_000_000.0
            || z.abs() > 30_000_000.0
            || y.abs() > 4096.0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid world/coordinates",
            ));
        }
        Ok(Self {
            world: world.into(),
            x,
            y,
            z,
        })
    }
}
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    #[serde(default)]
    homes: BTreeMap<String, BTreeMap<String, Position>>,
    #[serde(default)]
    warps: BTreeMap<String, Position>,
}
pub struct NativeUtilities {
    file: PathBuf,
    _store_lock: StoreLock,
    data: Saved,
}
fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}
fn normalize(name: &str) -> io::Result<String> {
    let name = name.to_ascii_lowercase();
    if valid_name(&name) {
        Ok(name)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid home/warp name",
        ))
    }
}
impl NativeUtilities {
    pub fn open(file: impl AsRef<Path>) -> io::Result<Self> {
        let file = file.as_ref().to_owned();
        let store_lock = lock_store(&file)?;
        let data: Saved = if file.exists() {
            let input = fs::File::open(&file)?;
            if input.metadata()?.len() > MAX_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "utility data too large",
                ));
            }
            let mut buf = Vec::new();
            input.take(MAX_SIZE + 1).read_to_end(&mut buf)?;
            serde_json::from_slice(&buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        } else {
            Saved {
                version: 1,
                ..Saved::default()
            }
        };
        Self::validate(&data)?;
        Ok(Self {
            file,
            _store_lock: store_lock,
            data,
        })
    }
    fn validate(data: &Saved) -> io::Result<()> {
        let mut count = 0;
        if data.version != 1 || data.homes.len() > 10_000 || data.warps.len() > 4096 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported utility data version/count",
            ));
        }
        for (user, homes) in &data.homes {
            if !valid_name(user) || homes.len() > 32 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid utility user/home count",
                ));
            }
            for (name, pos) in homes {
                if !valid_name(name) || Position::new(&pos.world, pos.x, pos.y, pos.z).is_err() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid home location",
                    ));
                }
                count += 1;
            }
        }
        for (name, pos) in &data.warps {
            if !valid_name(name) || Position::new(&pos.world, pos.x, pos.y, pos.z).is_err() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid warp location",
                ));
            }
        }
        if count > 100_000 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "too many homes"));
        }
        Ok(())
    }
    fn commit(&mut self, next: Saved) -> io::Result<()> {
        Self::validate(&next)?;
        atomic_json(&self.file, &next)?;
        self.data = next;
        Ok(())
    }
    pub fn set_warp(&mut self, name: &str, pos: Position) -> io::Result<()> {
        let name = normalize(name)?;
        let mut next = self.data.clone();
        next.warps.insert(name, pos);
        self.commit(next)
    }
    pub fn delete_warp(&mut self, name: &str) -> io::Result<bool> {
        let mut next = self.data.clone();
        let removed = next.warps.remove(&normalize(name)?).is_some();
        if removed {
            self.commit(next)?;
        }
        Ok(removed)
    }
    pub fn warp(&self, name: &str) -> Option<&Position> {
        self.data.warps.get(&name.to_ascii_lowercase())
    }
    pub fn list_warps(&self) -> Vec<&str> {
        self.data.warps.keys().map(String::as_str).collect()
    }
    pub fn set_home(&mut self, user: &str, name: &str, pos: Position) -> io::Result<()> {
        let user = normalize(user)?;
        let name = normalize(name)?;
        let mut next = self.data.clone();
        next.homes.entry(user).or_default().insert(name, pos);
        self.commit(next)
    }
    pub fn home(&self, user: &str, name: &str) -> Option<&Position> {
        self.data
            .homes
            .get(&user.to_ascii_lowercase())?
            .get(&name.to_ascii_lowercase())
    }
    pub fn list_homes(&self, user: &str) -> Vec<&str> {
        self.data
            .homes
            .get(&user.to_ascii_lowercase())
            .map(|homes| homes.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }
    pub fn delete_home(&mut self, user: &str, name: &str) -> io::Result<bool> {
        let user = normalize(user)?;
        let name = normalize(name)?;
        let mut next = self.data.clone();
        let removed = next
            .homes
            .get_mut(&user)
            .is_some_and(|homes| homes.remove(&name).is_some());
        if removed {
            self.commit(next)?;
        }
        Ok(removed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    fn service() -> (PathBuf, NativeUtilities) {
        let p = std::env::temp_dir().join(format!(
            "purpur-horizon-utility-{}-{}.json",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let m = NativeUtilities::open(&p).unwrap();
        (p, m)
    }
    #[test]
    fn native_warp_lifecycle_and_restart() {
        let (path, mut m) = service();
        let loc = Position::new("minecraft:overworld", 12.0, 64.0, -12.5).unwrap();
        m.set_warp("Spawn", loc.clone()).unwrap();
        assert_eq!(m.warp("SPAWN"), Some(&loc));
        assert_eq!(m.list_warps(), vec!["spawn"]);
        drop(m);
        let mut fresh = NativeUtilities::open(&path).unwrap();
        assert_eq!(fresh.warp("spawn"), Some(&loc));
        assert!(fresh.delete_warp("spawn").unwrap());
        assert!(fresh.warp("spawn").is_none());
        let _ = fs::remove_file(path);
    }
    #[test]
    fn per_player_homes_and_delete() {
        let (path, mut m) = service();
        let loc = Position::new("world", 1.0, 65.0, 2.0).unwrap();
        m.set_home("Alex", "base", loc.clone()).unwrap();
        assert_eq!(m.home("alex", "base"), Some(&loc));
        assert!(m.home("steve", "base").is_none());
        assert_eq!(m.list_homes("alex"), vec!["base"]);
        assert!(m.delete_home("alex", "base").unwrap());
        assert!(m.list_homes("alex").is_empty());
        let _ = fs::remove_file(path);
    }
    #[test]
    fn utilities_are_single_writer_and_persist_across_restart() {
        let (path, mut first) = service();
        let location = Position::new("world", 1.0, 65.0, 2.0).unwrap();
        first.set_warp("spawn", location.clone()).unwrap();
        assert_eq!(
            NativeUtilities::open(&path).err().unwrap().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(first);
        let second = NativeUtilities::open(&path).unwrap();
        assert_eq!(second.warp("spawn"), Some(&location));
        drop(second);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn rejects_invalid_coords_and_corrupted_files() {
        assert!(Position::new("world", f64::NAN, 64.0, 0.0).is_err());
        assert!(Position::new("world", 0.0, 64.0, 30_000_001.0).is_err());
        let (path, mut m) = service();
        assert!(m
            .set_warp("../bad", Position::new("world", 0.0, 0.0, 0.0).unwrap())
            .is_err());
        fs::write(
            &path,
            r#"{"version":1,"warps":{"spawn":{"world":"world","x":1e400,"y":0,"z":0}}}"#,
        )
        .unwrap();
        drop(m);
        assert!(NativeUtilities::open(&path).is_err());
        let _ = fs::remove_file(path);
    }
}
