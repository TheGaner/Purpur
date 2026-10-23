// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Rust permission service (not copied LuckPerms source).
//! The evaluator is intentionally a subset, NOT LuckPerms behavior parity.
//! Changes are persisted before publication. Undefined permissions deny commands.
use crate::command::Permissions;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_SIZE: u64 = 2 * 1024 * 1024;
const MAX_CACHE: usize = 8192;
const MAX_SUBJECTS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Allow,
    Deny,
    Undefined,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub node: String,
    pub decision: Decision,
    #[serde(default)]
    pub world: Option<String>,
    #[serde(default)]
    pub expires_at: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    #[serde(default)]
    grants: BTreeMap<String, Vec<Rule>>,
    #[serde(default)]
    parents: BTreeMap<String, Vec<String>>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            grants: BTreeMap::new(),
            parents: BTreeMap::new(),
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    principal: String,
    node: String,
    world: Option<String>,
}
#[derive(Clone)]
struct Cached {
    decision: Decision,
    calculated_at: u64,
    valid_until: Option<u64>,
}
pub struct NativePermissions {
    file: PathBuf,
    data: Saved,
    cache: Mutex<HashMap<Key, Cached>>,
    revision: u64,
}
// Store identities in disjoint namespaces so player UUIDs/names never
// resolve as group subjects, even if a remote client supplies "group:admin".
fn config_subject(subject: &str) -> String {
    if subject.starts_with("group:") || subject.starts_with("user:") {
        subject.to_owned()
    } else {
        format!("user:{subject}")
    }
}
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_:.-".contains(&b))
}
fn valid_node(s: &str) -> bool {
    (s == "*" || valid_id(s) || (s.ends_with(".*") && valid_id(&s[..s.len() - 2])))
        && s.len() <= 128
}
fn match_node(pattern: &str, node: &str) -> Option<usize> {
    if pattern == node {
        Some(1000 + node.len())
    } else if pattern == "*" {
        Some(0)
    } else if let Some(prefix) = pattern.strip_suffix(".*") {
        if node
            .strip_prefix(prefix)
            .is_some_and(|s| s.starts_with('.'))
        {
            Some(prefix.len())
        } else {
            None
        }
    } else {
        None
    }
}
pub(super) fn atomic_json<T: Serialize>(file: &Path, value: &T) -> io::Result<()> {
    let parent = file
        .parent()
        .ok_or_else(|| io::Error::other("no data parent"))?;
    fs::create_dir_all(parent)?;
    let temp = file.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    let mut out = File::create(&temp)?;
    out.write_all(&bytes)?;
    out.sync_all()?;
    fs::rename(temp, file)
}
impl NativePermissions {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = path.as_ref().to_owned();
        let data: Saved = if file.exists() {
            let f = File::open(&file)?;
            if f.metadata()?.len() > MAX_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "permission database too large",
                ));
            }
            let mut contents = Vec::new();
            f.take(MAX_SIZE + 1).read_to_end(&mut contents)?;
            serde_json::from_slice(&contents)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        } else {
            Saved::default()
        };
        Self::validate(&data)?;
        Ok(Self {
            file,
            data,
            cache: Mutex::new(HashMap::new()),
            revision: 0,
        })
    }
    fn validate(data: &Saved) -> io::Result<()> {
        if data.version != 1 || data.grants.len() > 10_000 || data.parents.len() > 10_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid permission database version/size",
            ));
        }
        for (principal, entries) in &data.grants {
            if !valid_id(principal)
                || entries.len() > 2048
                || entries.iter().any(|r| {
                    !valid_node(&r.node)
                        || r.world.as_deref().is_some_and(|w| !valid_id(w))
                        || r.decision == Decision::Undefined
                })
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid permission grant",
                ));
            }
        }
        for (principal, groups) in &data.parents {
            if !valid_id(principal)
                || groups.len() > MAX_SUBJECTS
                || groups.iter().any(|g| !valid_id(g))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid permission group link",
                ));
            }
        }
        Ok(())
    }
    fn commit(&mut self, next: Saved) -> io::Result<()> {
        Self::validate(&next)?;
        atomic_json(&self.file, &next)?;
        self.data = next;
        self.revision = self.revision.wrapping_add(1);
        self.cache.lock().unwrap().clear();
        Ok(())
    }
    /// Persist a principal grant. Undefined removes that exact node/context.
    pub fn set(
        &mut self,
        principal: &str,
        node: &str,
        decision: Decision,
        world: Option<&str>,
        expires_at: Option<u64>,
    ) -> io::Result<()> {
        let normalized_node = node.to_ascii_lowercase();
        if !valid_id(principal)
            || !valid_node(&normalized_node)
            || world.is_some_and(|w| !valid_id(w))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid principal/node/world",
            ));
        }
        let mut next = self.data.clone();
        let entries = next.grants.entry(config_subject(principal)).or_default();
        entries.retain(|r| !(r.node == normalized_node && r.world.as_deref() == world));
        if decision != Decision::Undefined {
            entries.push(Rule {
                node: normalized_node,
                decision,
                world: world.map(str::to_owned),
                expires_at,
            });
        }
        self.commit(next)
    }
    pub fn add_parent(&mut self, principal: &str, group: &str) -> io::Result<()> {
        if !valid_id(principal) || !valid_id(group) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid principal/group",
            ));
        }
        let mut next = self.data.clone();
        let link = format!("group:{group}");
        let groups = next.parents.entry(config_subject(principal)).or_default();
        if !groups.contains(&link) {
            groups.push(link);
        }
        self.commit(next)
    }
    pub fn remove_parent(&mut self, principal: &str, group: &str) -> io::Result<()> {
        let mut next = self.data.clone();
        if let Some(groups) = next.parents.get_mut(&config_subject(principal)) {
            groups.retain(|g| g != &format!("group:{group}"));
        }
        self.commit(next)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn resolve(&self, principal: &str, node: &str, world: Option<&str>, now: u64) -> Decision {
        let key = Key {
            principal: principal.to_owned(),
            node: node.to_ascii_lowercase(),
            world: world.map(str::to_owned),
        };
        if let Some(cached) = self.cache.lock().unwrap().get(&key) {
            if now >= cached.calculated_at && cached.valid_until.is_none_or(|expires| now < expires)
            {
                return cached.decision;
            }
        }
        let mut subjects = VecDeque::from([(format!("user:{principal}"), 0usize)]);
        let mut visited = HashSet::new();
        let mut best: Option<((usize, usize, usize), Decision)> = None;
        let mut next_expiry: Option<u64> = None;
        while let Some((subject, depth)) = subjects.pop_front() {
            if !visited.insert(subject.clone()) || visited.len() > MAX_SUBJECTS {
                continue;
            }
            if let Some(grants) = self.data.grants.get(&subject) {
                for rule in grants {
                    if let Some(exp) = rule.expires_at {
                        if exp <= now {
                            continue;
                        }
                        next_expiry = Some(next_expiry.map_or(exp, |previous| previous.min(exp)));
                    }
                    if rule.world.as_deref().is_some_and(|w| Some(w) != world) {
                        continue;
                    }
                    if let Some(spec) = match_node(&rule.node, &key.node) {
                        // Most specific node, then contextual rule, then direct/closest parent.
                        let rank = (
                            spec,
                            usize::from(rule.world.is_some()),
                            MAX_SUBJECTS.saturating_sub(depth),
                        );
                        if best.as_ref().is_none_or(|(old, d)| {
                            rank > *old
                                || (rank == *old
                                    && *d == Decision::Allow
                                    && rule.decision == Decision::Deny)
                        }) {
                            best = Some((rank, rule.decision));
                        }
                    }
                }
            }
            if let Some(groups) = self.data.parents.get(&subject) {
                for group in groups {
                    if !visited.contains(group) {
                        subjects.push_back((group.clone(), depth + 1));
                    }
                }
            }
        }
        let decision = best.map_or(Decision::Undefined, |(_, d)| d);
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= MAX_CACHE {
            cache.clear();
        }
        cache.insert(
            key,
            Cached {
                decision,
                calculated_at: now,
                valid_until: next_expiry,
            },
        );
        decision
    }
}
impl Permissions for NativePermissions {
    fn allowed(&self, principal: &str, node: &str, context: Option<&str>) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.resolve(principal, node, context, now) == Decision::Allow
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn db() -> (PathBuf, NativePermissions) {
        let filename = std::env::temp_dir().join(format!(
            "purpur-horizon-perm-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let db = NativePermissions::open(&filename).unwrap();
        (filename, db)
    }
    #[test]
    fn default_undefined_denies_and_persists() {
        let (path, mut db) = db();
        assert_eq!(
            db.resolve("alex", "essentials.home", None, 100),
            Decision::Undefined
        );
        db.set("alex", "essentials.home", Decision::Allow, None, None)
            .unwrap();
        assert_eq!(
            db.resolve("alex", "essentials.home", None, 100),
            Decision::Allow
        );
        let fresh = NativePermissions::open(&path).unwrap();
        assert_eq!(
            fresh.resolve("alex", "essentials.home", None, 100),
            Decision::Allow
        );
        let _ = fs::remove_file(path);
    }
    #[test]
    fn group_inheritance_cycle_and_override() {
        let (path, mut db) = db();
        db.set("group:staff", "essentials.*", Decision::Allow, None, None)
            .unwrap();
        db.add_parent("alex", "staff").unwrap();
        db.add_parent("group:staff", "staff").unwrap();
        assert_eq!(
            db.resolve("alex", "essentials.warp", None, 100),
            Decision::Allow
        );
        db.set("alex", "essentials.warp", Decision::Deny, None, None)
            .unwrap();
        assert_eq!(
            db.resolve("alex", "essentials.warp", None, 100),
            Decision::Deny
        );
        let _ = fs::remove_file(path);
    }
    #[test]
    fn contextual_rule_and_expiry_invalidate_cache() {
        let (path, mut db) = db();
        db.set("alex", "essentials.home", Decision::Allow, None, None)
            .unwrap();
        db.set(
            "alex",
            "essentials.home",
            Decision::Deny,
            Some("nether"),
            Some(105),
        )
        .unwrap();
        assert_eq!(
            db.resolve("alex", "essentials.home", Some("nether"), 104),
            Decision::Deny
        );
        assert_eq!(
            db.resolve("alex", "essentials.home", Some("nether"), 105),
            Decision::Allow
        );
        db.set("alex", "essentials.home", Decision::Deny, None, None)
            .unwrap();
        assert_eq!(
            db.resolve("alex", "essentials.home", None, 105),
            Decision::Deny
        );
        let _ = fs::remove_file(path);
    }
    #[test]
    fn persistent_group_remove_and_revisions() {
        let (path, mut db) = db();
        db.set("group:member", "test.foo", Decision::Allow, None, None)
            .unwrap();
        let rev = db.revision();
        db.add_parent("alex", "member").unwrap();
        assert!(db.revision() > rev);
        assert_eq!(db.resolve("alex", "test.foo", None, 100), Decision::Allow);
        db.remove_parent("alex", "member").unwrap();
        assert_eq!(
            db.resolve("alex", "test.foo", None, 100),
            Decision::Undefined
        );
        let _ = fs::remove_file(path);
    }
    #[test]
    fn invalid_input_and_corruption_fail_closed() {
        let (path, mut db) = db();
        assert!(db.set("alex", "", Decision::Allow, None, None).is_err());
        assert!(db.set("alex", "a", Decision::Undefined, None, None).is_ok());
        fs::write(&path, "{broken").unwrap();
        assert!(NativePermissions::open(&path).is_err());
        let _ = fs::remove_file(path);
    }
}
