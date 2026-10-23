// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Rust permission service (not copied LuckPerms source).
//! The evaluator is intentionally a subset, NOT LuckPerms behavior parity.
//! Changes are persisted before publication. Undefined permissions deny commands.
use crate::command::Permissions;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_SIZE: u64 = 2 * 1024 * 1024;
const MAX_CACHE: usize = 8192;
const MAX_SUBJECTS: usize = 128;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Exclusive lifetime lock on a persistent JSON database. The .lock file
/// remains after shutdown, but the OS releases the advisory lock on close.
/// This supports ONE writer per file, not concurrent multi-server updates.
pub(super) struct StoreLock {
    _handle: File,
}
pub(super) fn lock_store(path: &Path) -> io::Result<StoreLock> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("no store parent"))?;
    fs::create_dir_all(parent)?;
    let lock_path = path.with_extension("json.lock");
    let handle = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    handle.try_lock().map_err(|e| {
        io::Error::new(
            io::ErrorKind::WouldBlock,
            format!("store already open by another writer (or locking unsupported): {e}"),
        )
    })?;
    Ok(StoreLock { _handle: handle })
}

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
    _store_lock: StoreLock,
    data: Saved,
    cache: Mutex<HashMap<Key, Cached>>,
    revision: u64,
    poisoned: bool,
}
// Store identities in disjoint namespaces so player UUIDs/names never
// resolve as group subjects, even if a remote client supplies "group:admin".
// Console-only legacy name grammar. No caller-supplied string is proof of a
// Minecraft player's authenticated identity; future play must use UUIDs.
fn canonical_name(input: &str) -> Option<String> {
    (valid_id(input) && !input.contains(':')).then(|| input.to_ascii_lowercase())
}
fn typed_subject(input: &str, default_namespace: &str) -> Option<String> {
    let (namespace, raw) = if let Some(name) = input.strip_prefix("user:") {
        ("user", name)
    } else if let Some(name) = input.strip_prefix("group:") {
        ("group", name)
    } else {
        (default_namespace, input)
    };
    let name = canonical_name(raw)?;
    let subject = format!("{namespace}:{name}");
    valid_subject_key(&subject).then_some(subject)
}
fn config_subject(subject: &str) -> Option<String> {
    typed_subject(subject, "user")
}
fn user_subject(subject: &str) -> Option<String> {
    // Explicitly refuse a group principal in a player check; this prevents
    // a future untrusted login name from being interpreted as a group grant.
    let subject = typed_subject(subject, "user")?;
    subject.starts_with("user:").then_some(subject)
}
fn group_subject(group: &str) -> Option<String> {
    let group = typed_subject(group, "group")?;
    group.starts_with("group:").then_some(group)
}
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_:.-".contains(&b))
}
fn valid_subject_key(subject: &str) -> bool {
    subject
        .strip_prefix("user:")
        .or_else(|| subject.strip_prefix("group:"))
        .is_some_and(|name| valid_id(name) && !name.contains(':'))
}
fn valid_group_link(subject: &str) -> bool {
    subject
        .strip_prefix("group:")
        .is_some_and(|name| valid_id(name) && !name.contains(':'))
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
/// An error after rename means disk may contain the new revision even
/// though directory durability could not be confirmed.
pub(super) struct PersistError {
    pub(super) cause: io::Error,
    pub(super) replaced: bool,
}
pub(super) fn atomic_json<T: Serialize>(file: &Path, value: &T) -> Result<(), PersistError> {
    atomic_json_with_sync(file, value, |parent| {
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        #[cfg(not(unix))]
        let _ = parent;
        Ok(())
    })
}
pub(super) fn atomic_json_with_sync<T: Serialize>(
    file: &Path,
    value: &T,
    sync_dir: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), PersistError> {
    let mut replaced = false;
    let mut temp_path: Option<PathBuf> = None;
    let outcome = (|| -> io::Result<()> {
        let parent = file
            .parent()
            .ok_or_else(|| io::Error::other("no data parent"))?;
        fs::create_dir_all(parent)?;
        // Exactly the same limit enforced on open.
        let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
        if bytes.len() as u64 > MAX_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "serialized database would exceed 2 MiB",
            ));
        }
        let name = file
            .file_name()
            .ok_or_else(|| io::Error::other("invalid store filename"))?;
        let mut tmp_name = std::ffi::OsString::from(".");
        tmp_name.push(name);
        tmp_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let temp = parent.join(tmp_name);
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        temp_path = Some(temp.clone());
        out.write_all(&bytes)?;
        out.sync_all()?;
        drop(out);
        fs::rename(&temp, file)?;
        replaced = true;
        // An error here is fundamentally different from one before rename.
        // Never allow stale live data to overwrite the newly renamed state.
        sync_dir(parent)?;
        Ok(())
    })();
    if !replaced {
        if let Some(temp) = temp_path {
            let _ = fs::remove_file(temp);
        }
    }
    outcome.map_err(|cause| PersistError { cause, replaced })
}
impl NativePermissions {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = path.as_ref().to_owned();
        // Lock BEFORE reading, or two processes can both load one stale snapshot.
        let store_lock = lock_store(&file)?;
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
        // Old JSON v1 data could contain mixed-case player/group keys. Do not
        // silently merge two legacy identities with potentially opposite grants.
        let data = Self::canonicalize(data)?;
        Self::validate(&data)?;
        Ok(Self {
            file,
            _store_lock: store_lock,
            data,
            cache: Mutex::new(HashMap::new()),
            revision: 0,
            poisoned: false,
        })
    }
    fn canonicalize(mut data: Saved) -> io::Result<Saved> {
        let mut grants = BTreeMap::new();
        for (subject, mut rules) in data.grants {
            let key = subject.to_ascii_lowercase();
            if !valid_id(&key) || grants.contains_key(&key) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ambiguous or invalid mixed-case permission subject",
                ));
            }
            for rule in &mut rules {
                rule.node.make_ascii_lowercase();
            }
            grants.insert(key, rules);
        }
        let mut parents = BTreeMap::new();
        for (subject, groups) in data.parents {
            let key = subject.to_ascii_lowercase();
            if !valid_id(&key) || parents.contains_key(&key) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ambiguous or invalid mixed-case permission parent",
                ));
            }
            let mut normalized = Vec::new();
            for group in groups {
                let group = group.to_ascii_lowercase();
                if !normalized.contains(&group) {
                    normalized.push(group);
                }
            }
            parents.insert(key, normalized);
        }
        data.grants = grants;
        data.parents = parents;
        Ok(data)
    }
    fn validate(data: &Saved) -> io::Result<()> {
        if data.version != 1 || data.grants.len() > 10_000 || data.parents.len() > 10_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid permission database version/size",
            ));
        }
        for (principal, entries) in &data.grants {
            if !valid_subject_key(principal)
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
            if !valid_subject_key(principal)
                || groups.len() > MAX_SUBJECTS
                || groups.iter().any(|g| !valid_group_link(g))
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
        self.commit_with(next, atomic_json)
    }
    fn commit_with(
        &mut self,
        next: Saved,
        save: impl FnOnce(&Path, &Saved) -> Result<(), PersistError>,
    ) -> io::Result<()> {
        if self.poisoned {
            return Err(io::Error::other(
                "permission store requires restart after ambiguous write",
            ));
        }
        Self::validate(&next)?;
        match save(&self.file, &next) {
            Ok(()) => {
                self.data = next;
                self.revision = self.revision.wrapping_add(1);
                self.cache.lock().unwrap().clear();
                Ok(())
            }
            Err(failure) => {
                if failure.replaced {
                    self.poisoned = true;
                    self.cache.lock().unwrap().clear();
                }
                Err(failure.cause)
            }
        }
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
        let subject = config_subject(principal).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid subject namespace")
        })?;
        let mut next = self.data.clone();
        let entries = next.grants.entry(subject).or_default();
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
        let subject = config_subject(principal)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid parent subject"))?;
        let link = group_subject(group).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid group namespace")
        })?;
        let mut next = self.data.clone();
        let groups = next.parents.entry(subject).or_default();
        if !groups.contains(&link) {
            groups.push(link);
        }
        self.commit(next)
    }
    pub fn remove_parent(&mut self, principal: &str, group: &str) -> io::Result<()> {
        let subject = config_subject(principal)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid parent subject"))?;
        let link = group_subject(group).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid group namespace")
        })?;
        let mut next = self.data.clone();
        if let Some(groups) = next.parents.get_mut(&subject) {
            groups.retain(|g| g != &link);
        }
        self.commit(next)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn resolve(&self, principal: &str, node: &str, world: Option<&str>, now: u64) -> Decision {
        // If rename succeeded but fsync failed, any cached or in-memory
        // authorizations may disagree with the snapshot on disk.
        if self.poisoned {
            return Decision::Undefined;
        }
        // The persisted mutation API bounds names to 128 bytes, but a
        // lookup previously accepted arbitrary strings into the 8192-entry
        // cache. Reject invalid inputs before allocating/copying a cache key.
        if !valid_node(node) || world.is_some_and(|w| !valid_id(w)) {
            return Decision::Undefined;
        }
        let Some(user) = user_subject(principal) else {
            return Decision::Undefined;
        };
        let key = Key {
            principal: user,
            node: node.to_ascii_lowercase(),
            world: world.map(str::to_owned),
        };
        if let Some(cached) = self.cache.lock().unwrap().get(&key) {
            if now >= cached.calculated_at && cached.valid_until.is_none_or(|expires| now < expires)
            {
                return cached.decision;
            }
        }
        let mut subjects = VecDeque::from([(key.principal.clone(), 0usize)]);
        let mut visited = HashSet::new();
        let mut best: Option<((usize, usize, usize), Decision)> = None;
        let mut next_expiry: Option<u64> = None;
        while let Some((subject, depth)) = subjects.pop_front() {
            if !visited.insert(subject.clone()) {
                continue;
            }
            // A permission answer from only part of the reachable group
            // graph is not authoritative. A bounded evaluator must deny.
            if visited.len() > MAX_SUBJECTS {
                return Decision::Undefined;
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
        drop(db);
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
    fn malformed_lookup_inputs_fail_closed_before_cache_allocation() {
        let (path, mut db) = db();
        db.set("alex", "test.read", Decision::Allow, None, None)
            .unwrap();
        let huge = "x".repeat(256 * 1024);
        for (node, world) in [
            (huge.as_str(), None),
            ("test.read", Some(huge.as_str())),
            ("", None),
            ("test.invalid!", None),
            ("test.read", Some("bad world")),
            ("test.read", Some("")),
        ] {
            assert_eq!(db.resolve("alex", node, world, 100), Decision::Undefined);
            assert!(
                db.cache.lock().unwrap().is_empty(),
                "invalid lookup was cached"
            );
        }
        assert_eq!(db.resolve("alex", "test.read", None, 100), Decision::Allow);
        assert_eq!(db.cache.lock().unwrap().len(), 1);
        drop(db);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn maximum_valid_lookup_length_remains_supported() {
        let (path, mut db) = db();
        let node = "n".repeat(128);
        let world = "w".repeat(128);
        db.set("alex", &node, Decision::Allow, Some(&world), None)
            .unwrap();
        assert_eq!(
            db.resolve("alex", &node, Some(&world), 100),
            Decision::Allow
        );
        assert_eq!(db.resolve("alex", &node, None, 100), Decision::Undefined);
        drop(db);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
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
    fn second_store_writer_is_rejected_until_first_drops() {
        let (path, mut first) = db();
        first
            .set("alex", "test.node", Decision::Allow, None, None)
            .unwrap();
        let error = match NativePermissions::open(&path) {
            Ok(_) => panic!("concurrent store must not open"),
            Err(e) => e,
        };
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        drop(first);
        let second = NativePermissions::open(&path).unwrap();
        assert_eq!(
            second.resolve("alex", "test.node", None, 10),
            Decision::Allow
        );
        drop(second);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn oversize_write_preserves_previous_db() {
        let (path, mut db) = db();
        db.set("alex", "test.read", Decision::Allow, None, None)
            .unwrap();
        let original = fs::read(&path).unwrap();
        let result = atomic_json(&path, &"x".repeat((MAX_SIZE + 1) as usize));
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        drop(db);
        let reopened = NativePermissions::open(&path).unwrap();
        assert_eq!(
            reopened.resolve("alex", "test.read", None, 10),
            Decision::Allow
        );
        drop(reopened);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn post_rename_sync_failure_poison_denies_and_survives_restart() {
        let (path, mut db) = db();
        db.set("alex", "test.access", Decision::Allow, None, None)
            .unwrap();
        assert_eq!(
            db.resolve("alex", "test.access", None, 100),
            Decision::Allow
        );
        let mut next = db.data.clone();
        next.grants
            .entry("user:alex".into())
            .or_default()
            .push(Rule {
                node: "test.replaced".into(),
                decision: Decision::Deny,
                world: None,
                expires_at: None,
            });
        let error = db
            .commit_with(next, |file, value| {
                atomic_json_with_sync(file, value, |_| {
                    Err(io::Error::other("injected directory sync failure"))
                })
            })
            .unwrap_err();
        assert!(error.to_string().contains("injected directory"));
        assert!(db.poisoned);
        // Invalidate a warm permission cache and deny everything immediately.
        assert_eq!(
            db.resolve("alex", "test.access", None, 100),
            Decision::Undefined
        );
        let disk = fs::read(&path).unwrap();
        assert!(db
            .set("alex", "test.other", Decision::Allow, None, None)
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), disk);
        drop(db);
        let fresh = NativePermissions::open(&path).unwrap();
        assert_eq!(
            fresh.resolve("alex", "test.access", None, 100),
            Decision::Allow
        );
        assert_eq!(
            fresh.resolve("alex", "test.replaced", None, 100),
            Decision::Deny
        );
        drop(fresh);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn pre_rename_error_preserves_live_store_and_allows_retry() {
        let (path, mut db) = db();
        let next = db.data.clone();
        let error = db
            .commit_with(next, |_, _| {
                Err(PersistError {
                    cause: io::Error::other("injected pre-rename failure"),
                    replaced: false,
                })
            })
            .unwrap_err();
        assert!(error.to_string().contains("pre-rename"));
        assert!(!db.poisoned);
        db.set("alex", "test.allowed", Decision::Allow, None, None)
            .unwrap();
        assert_eq!(
            db.resolve("alex", "test.allowed", None, 10),
            Decision::Allow
        );
        drop(db);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn subject_case_is_consistent_for_direct_and_group_access() {
        let (path, mut db) = db();
        db.set("Alex", "Test.Use", Decision::Deny, None, None)
            .unwrap();
        assert_eq!(db.resolve("aLeX", "test.use", None, 10), Decision::Deny);
        db.set("group:STAFF", "test.staff", Decision::Allow, None, None)
            .unwrap();
        db.add_parent("ALEX", "StAfF").unwrap();
        assert_eq!(db.resolve("alex", "test.staff", None, 10), Decision::Allow);
        db.remove_parent("AlEx", "STAFF").unwrap();
        assert_eq!(
            db.resolve("alex", "test.staff", None, 10),
            Decision::Undefined
        );
        let disk = fs::read_to_string(&path).unwrap();
        assert!(disk.contains("user:alex") && disk.contains("group:staff"));
        assert!(!disk.contains("user:Alex") && !disk.contains("group:STAFF"));
        drop(db);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn historical_casefold_json_loads_without_losing_permissions() {
        let (path, original) = db();
        drop(original);
        fs::write(&path, r#"{"version":1,"grants":{"user:Alex":[{"node":"TEST.USE","decision":"deny"}],"group:STAFF":[{"node":"test.staff","decision":"allow"}]},"parents":{"user:Alex":["group:STAFF"]}}"#).unwrap();
        let mut db = NativePermissions::open(&path).unwrap();
        assert_eq!(db.resolve("aLEX", "test.use", None, 10), Decision::Deny);
        assert_eq!(db.resolve("alex", "test.staff", None, 10), Decision::Allow);
        db.set("aLeX", "test.extra", Decision::Allow, None, None)
            .unwrap();
        drop(db);
        let disk = fs::read_to_string(&path).unwrap();
        assert!(disk.contains("user:alex") && disk.contains("group:staff"));
        assert!(!disk.contains("user:Alex") && !disk.contains("group:STAFF"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn conflicting_historical_case_aliases_fail_closed() {
        let (path, original) = db();
        drop(original);
        fs::write(&path,r#"{"version":1,"grants":{"user:Alex":[{"node":"test.use","decision":"allow"}],"user:alex":[{"node":"test.use","decision":"deny"}]}}"#).unwrap();
        let error = NativePermissions::open(&path).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("ambiguous"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn saturated_group_graph_fails_closed_and_recovers_after_parent_removed() {
        let (path, mut db) = db();
        db.set("alex", "test.privileged", Decision::Allow, None, None)
            .unwrap();
        // 127 groups + user root = 128 visited subjects, exactly at the cap.
        let mut snapshot = db.data.clone();
        snapshot.parents.insert(
            "user:alex".into(),
            (0..127).map(|i| format!("group:g{i:03}")).collect(),
        );
        db.commit(snapshot).unwrap();
        assert_eq!(
            db.resolve("alex", "test.privileged", None, 10),
            Decision::Allow
        );
        // The 128th direct parent is schema-valid, but the reachable graph
        // has 129 distinct subjects. Never grant based on a partial traversal.
        db.add_parent("alex", "g127").unwrap();
        assert_eq!(
            db.resolve("alex", "test.privileged", None, 10),
            Decision::Undefined
        );
        assert_eq!(
            db.resolve("alex", "test.privileged", None, 11),
            Decision::Undefined
        );
        db.remove_parent("alex", "g127").unwrap();
        assert_eq!(
            db.resolve("alex", "test.privileged", None, 10),
            Decision::Allow
        );
        drop(db);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn malformed_parent_namespace_and_subject_keys_fail_closed() {
        let (path, original) = db();
        drop(original);
        let cases = [
            r#"{"version":1,"grants":{"user:admin":[{"node":"test.root","decision":"allow"}]},"parents":{"user:alex":["user:admin"]}}"#,
            r#"{"version":1,"grants":{"group:":[{"node":"test.root","decision":"allow"}]}}"#,
            r#"{"version":1,"grants":{"alex":[{"node":"test.root","decision":"allow"}]}}"#,
            r#"{"version":1,"parents":{"user:":["group:staff"]}}"#,
            r#"{"version":1,"grants":{"user:user:alice":[{"node":"test.use","decision":"allow"}]}}"#,
            r#"{"version":1,"parents":{"user:alex":["group:group:staff"]}}"#,
        ];
        for json in cases {
            fs::write(&path, json).unwrap();
            let error = match NativePermissions::open(&path) {
                Ok(_) => panic!("invalid subject/parent accepted: {json}"),
                Err(error) => error,
            };
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn typed_principal_names_agree_with_bare_names_and_reject_spoofed_groups() {
        let (path, mut db) = db();
        db.set("user:Alex", "test.direct", Decision::Allow, None, None)
            .unwrap();
        assert_eq!(db.resolve("alex", "test.direct", None, 10), Decision::Allow);
        assert_eq!(
            db.resolve("user:ALEx", "test.direct", None, 10),
            Decision::Allow
        );
        db.set("group:Staff", "test.group", Decision::Allow, None, None)
            .unwrap();
        db.add_parent("user:Alex", "group:staff").unwrap();
        assert_eq!(db.resolve("Alex", "test.group", None, 10), Decision::Allow);
        assert_eq!(
            db.resolve("group:staff", "test.group", None, 10),
            Decision::Undefined
        );
        db.remove_parent("alex", "group:Staff").unwrap();
        assert_eq!(
            db.resolve("alex", "test.group", None, 10),
            Decision::Undefined
        );
        assert!(db
            .set("user:user:alex", "test.direct", Decision::Allow, None, None)
            .is_err());
        assert!(db.add_parent("alex", "user:staff").is_err());
        assert!(db.add_parent("alex", "group:group:staff").is_err());
        assert!(db.remove_parent("alex", "user:staff").is_err());
        drop(db);
        let reopened = NativePermissions::open(&path).unwrap();
        assert_eq!(
            reopened.resolve("user:alex", "test.direct", None, 10),
            Decision::Allow
        );
        drop(reopened);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }
    #[test]
    fn invalid_input_and_corruption_fail_closed() {
        let (path, mut db) = db();
        assert!(db.set("alex", "", Decision::Allow, None, None).is_err());
        assert!(db.set("alex", "a", Decision::Undefined, None, None).is_ok());
        fs::write(&path, "{broken").unwrap();
        drop(db);
        assert!(NativePermissions::open(&path).is_err());
        let _ = fs::remove_file(path);
    }
}
