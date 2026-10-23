// SPDX-License-Identifier: AGPL-3.0-or-later
//! Standalone Brigadier-inspired command registry foundation.
//! No Java, Bukkit or client-engine dependency. Not full vanilla command parity.
use std::collections::{HashMap, HashSet};
pub mod tree;
use std::time::Duration;
const MAX_LINE: usize = 4096;
const MAX_TOKENS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Empty,
    TooLong,
    InvalidSyntax(String),
    Unknown(String),
    Duplicate(String),
    Denied(String),
    Usage(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty command"),
            Self::TooLong => write!(f, "Command exceeded length or argument limit"),
            Self::InvalidSyntax(s) => write!(f, "Invalid command: {s}"),
            Self::Unknown(s) => write!(f, "Unknown command: {s}"),
            Self::Duplicate(s) => write!(f, "Duplicate command/alias: {s}"),
            Self::Denied(s) => write!(f, "You do not have permission: {s}"),
            Self::Usage(s) => write!(f, "Usage: {s}"),
        }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<String>, Error> {
    if input.len() > MAX_LINE {
        return Err(Error::TooLong);
    }
    let mut out = vec![];
    let mut word = String::new();
    let mut quoted: Option<char> = None;
    let mut escaped = false;
    let mut started = false;
    for ch in input.chars() {
        if escaped {
            word.push(ch);
            escaped = false;
            started = true;
        } else if ch == '\\' {
            escaped = true;
            started = true;
        } else if let Some(quote) = quoted {
            if ch == quote {
                quoted = None
            } else {
                word.push(ch)
            }
        } else if ch == '"' || ch == '\'' {
            quoted = Some(ch);
            started = true;
        } else if ch.is_whitespace() {
            if started {
                out.push(std::mem::take(&mut word));
                started = false;
                if out.len() > MAX_TOKENS {
                    return Err(Error::TooLong);
                }
            }
        } else {
            started = true;
            word.push(ch);
        }
    }
    if escaped {
        return Err(Error::InvalidSyntax("dangling backslash".into()));
    }
    if quoted.is_some() {
        return Err(Error::InvalidSyntax("unterminated quoted argument".into()));
    }
    if started {
        out.push(word)
    }
    if out.len() > MAX_TOKENS {
        return Err(Error::TooLong);
    }
    Ok(out)
}

pub enum Source<'a> {
    Console,
    Player { id: &'a str, world: Option<&'a str> },
}

pub trait Permissions {
    fn allowed(&self, principal: &str, node: &str, context: Option<&str>) -> bool;
}

#[derive(Clone, Debug)]
struct Grant {
    node: String,
    allow: bool,
    world: Option<String>,
}
#[derive(Default)]
pub struct Grants {
    grants: HashMap<String, Vec<Grant>>,
    groups: HashMap<String, Vec<String>>,
}
impl Grants {
    pub fn set(&mut self, principal: &str, node: &str, allow: bool, world: Option<&str>) {
        self.grants
            .entry(principal.to_string())
            .or_default()
            .push(Grant {
                node: node.to_ascii_lowercase(),
                allow,
                world: world.map(str::to_owned),
            });
    }
    pub fn add_group(&mut self, principal: &str, group: &str) {
        let entry = self.groups.entry(principal.to_owned()).or_default();
        if !entry.contains(&group.to_owned()) {
            entry.push(group.to_owned());
        }
    }
}
fn match_node(rule: &str, node: &str) -> Option<usize> {
    if rule == "*" {
        return Some(0);
    }
    if rule == node {
        return Some(1000 + rule.len());
    }
    if let Some(prefix) = rule.strip_suffix(".*") {
        if node.starts_with(prefix) && node.as_bytes().get(prefix.len()) == Some(&b'.') {
            return Some(prefix.len());
        }
    }
    None
}
impl Permissions for Grants {
    fn allowed(&self, principal: &str, node: &str, context: Option<&str>) -> bool {
        if node.is_empty() {
            return true;
        }
        // Most specific matching rule wins. On an equal score explicit deny wins.
        // Direct user rules beat group rules at the same specificity.
        let node = node.to_ascii_lowercase();
        let mut subjects = vec![principal.to_owned()];
        let mut visited = HashSet::new();
        visited.insert(principal.to_owned());
        let mut cursor = 0;
        while cursor < subjects.len() && subjects.len() < 64 {
            let current = subjects[cursor].clone();
            if let Some(groups) = self.groups.get(&current) {
                for group in groups {
                    let group_key = format!("group:{group}");
                    if visited.insert(group_key.clone()) {
                        subjects.push(group_key);
                    }
                }
            }
            cursor += 1;
        }
        if visited.insert("group:default".to_string()) {
            subjects.push("group:default".to_string());
        }
        let mut best: Option<(usize, bool)> = None;
        for (level, subject) in subjects.iter().enumerate() {
            if let Some(grants) = self.grants.get(subject) {
                for grant in grants {
                    if grant.world.as_deref().is_some_and(|w| Some(w) != context) {
                        continue;
                    }
                    if let Some(spec) = match_node(&grant.node, &node) {
                        let score = spec * 1000
                            + if grant.world.is_some() { 100 } else { 0 }
                            + 99usize.saturating_sub(level);
                        match best {
                            None => best = Some((score, grant.allow)),
                            Some((old, old_allow))
                                if score > old || (score == old && old_allow && !grant.allow) =>
                            {
                                best = Some((score, grant.allow))
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        best.is_some_and(|(_, allow)| allow)
    }
}
impl<'a> Source<'a> {
    fn has(&self, node: &str, permissions: &dyn Permissions) -> bool {
        match self {
            Self::Console => true,
            Self::Player { id, world } => permissions.allowed(id, node, *world),
        }
    }
}
#[derive(Clone, Debug)]
pub enum Argument {
    Word,
    Integer { min: i64, max: i64 },
    Greedy,
}
#[derive(Clone, Debug)]
pub struct Parameter {
    pub label: &'static str,
    pub kind: Argument,
    pub optional: bool,
}
#[derive(Clone, Debug)]
pub enum Handler {
    Builtin(Builtin),
    Core,
    Plugin { owner: String },
}
#[derive(Clone, Copy, Debug)]
pub enum Builtin {
    Help,
    Stop,
    Version,
    Plugins,
    Uptime,
    Tps,
    Tree,
    Echo,
}
#[derive(Clone, Debug)]
pub struct Command {
    pub name: String,
    pub aliases: Vec<String>,
    pub permission: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub handler: Handler,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Stop,
    Plugin {
        owner: String,
        command: String,
        args: String,
    },
    Core {
        command: String,
        args: String,
    },
}
pub struct Reply {
    pub lines: Vec<String>,
    pub effect: Effect,
}
pub struct Snapshot<'a> {
    pub version: &'a str,
    pub plugins: &'a str,
    pub tick: u64,
    pub uptime: Duration,
}
#[derive(Default)]
pub struct Registry {
    commands: HashMap<String, Command>,
    routes: HashMap<String, String>,
    revision: u64,
}
fn valid_name(name: &str) -> bool {
    if name == "?" {
        return true;
    }
    !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-' || c == b':'
        })
}
impl Registry {
    pub fn builtins() -> Self {
        let mut reg = Self::default();
        let entries = [
            (
                "help",
                vec!["?"],
                "List permitted server commands",
                Builtin::Help,
            ),
            (
                "stop",
                vec!["quit", "exit"],
                "Shut down the server",
                Builtin::Stop,
            ),
            (
                "version",
                vec!["ver"],
                "Show server version",
                Builtin::Version,
            ),
            (
                "plugins",
                vec!["pl"],
                "List plugin runtime summary",
                Builtin::Plugins,
            ),
            ("uptime", vec![], "Show server uptime", Builtin::Uptime),
            (
                "tps",
                vec![],
                "Show configured tick rate (not measured)",
                Builtin::Tps,
            ),
            (
                "commandtree",
                vec![],
                "Inspect the permission-filtered command graph (not a client packet)",
                Builtin::Tree,
            ),
            (
                "echo",
                vec![],
                "Echo text to the command source",
                Builtin::Echo,
            ),
        ];
        for (name, aliases, description, handler) in entries {
            reg.register(Command {
                name: name.into(),
                aliases: aliases.iter().map(ToString::to_string).collect(),
                permission: format!("horizon.command.{name}"),
                description: description.into(),
                parameters: if name == "echo" {
                    vec![Parameter {
                        label: "text",
                        kind: Argument::Greedy,
                        optional: false,
                    }]
                } else if name == "help" {
                    vec![Parameter {
                        label: "command",
                        kind: Argument::Word,
                        optional: true,
                    }]
                } else {
                    vec![]
                },
                handler: Handler::Builtin(handler),
            })
            .expect("non-overlapping builtin command");
        }
        reg
    }
    pub fn register(&mut self, spec: Command) -> Result<(), Error> {
        // Bounded registry and command tree size; plugin manifests must not
        // allocate an unbounded node graph on player join.
        if self.commands.len() >= 1024 || spec.aliases.len() > 32 || spec.parameters.len() > 64 {
            return Err(Error::TooLong);
        }
        let name = spec.name.to_ascii_lowercase();
        let aliases: Vec<_> = spec
            .aliases
            .iter()
            .map(|a| a.to_ascii_lowercase())
            .collect();
        let mut all = vec![name.clone()];
        all.extend(aliases);
        let unique: HashSet<&str> = all.iter().map(String::as_str).collect();
        if unique.len() != all.len() {
            return Err(Error::Duplicate(name));
        }
        for label in &all {
            if !valid_name(label) {
                return Err(Error::InvalidSyntax(format!(
                    "invalid command name {label}"
                )));
            }
            if self.routes.contains_key(label) {
                return Err(Error::Duplicate(label.clone()));
            }
        }
        if spec.parameters.iter().any(|p| p.optional)
            && spec
                .parameters
                .windows(2)
                .any(|w| w[0].optional && !w[1].optional)
        {
            return Err(Error::InvalidSyntax(
                "required parameter after optional parameter".into(),
            ));
        }
        if spec
            .parameters
            .iter()
            .take(spec.parameters.len().saturating_sub(1))
            .any(|p| matches!(p.kind, Argument::Greedy))
        {
            return Err(Error::InvalidSyntax("greedy parameter must be last".into()));
        }
        for label in all {
            self.routes.insert(label, name.clone());
        }
        self.commands.insert(name, spec);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn unregister_owner(&mut self, owner: &str) -> usize {
        let owned: Vec<_> = self
            .commands
            .iter()
            .filter_map(|(key, cmd)| {
                if matches!(&cmd.handler, Handler::Plugin { owner: id } if id == owner) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        for name in &owned {
            if let Some(spec) = self.commands.remove(name) {
                self.routes.remove(&name.to_lowercase());
                for alias in spec.aliases {
                    self.routes.remove(&alias.to_lowercase());
                }
            }
        }
        if !owned.is_empty() {
            self.revision = self.revision.wrapping_add(1);
        }
        owned.len()
    }
    /// Permission-filtered deterministic command graph. This intentionally
    /// does not perform wire serialization or cache potentially stale grants.
    pub fn project(&self, source: &Source<'_>, permissions: &dyn Permissions) -> tree::Projection {
        let visible = self
            .commands
            .iter()
            .filter(|(_, command)| source.has(&command.permission, permissions))
            .map(|(name, command)| (name.as_str(), command))
            .collect();
        tree::build(self.revision, visible)
    }
    pub fn visible(&self, source: &Source<'_>, permissions: &dyn Permissions) -> Vec<String> {
        let mut items: Vec<_> = self
            .commands
            .values()
            .filter(|c| source.has(&c.permission, permissions))
            .map(|c| c.name.clone())
            .collect();
        items.sort();
        items
    }
    pub fn complete(
        &self,
        input: &str,
        source: &Source<'_>,
        permissions: &dyn Permissions,
    ) -> Vec<String> {
        let q = input.trim_start().trim_start_matches('/');
        // Only label completions are supported; argument/tab/tree completions are a later milestone.
        if q.contains(char::is_whitespace) {
            return vec![];
        }
        let query = q.to_ascii_lowercase();
        let mut matches: Vec<_> = self
            .routes
            .iter()
            .filter_map(|(label, key)| {
                let command = self.commands.get(key)?;
                if label.starts_with(&query) && source.has(&command.permission, permissions) {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect();
        matches.sort();
        matches
    }
    fn usage(spec: &Command) -> String {
        let mut result = format!("/{}", spec.name);
        for param in &spec.parameters {
            if param.optional {
                result.push_str(&format!(" [{}]", param.label));
            } else {
                result.push_str(&format!(" <{}>", param.label));
            }
        }
        result
    }
    pub fn execute(
        &self,
        input: &str,
        source: &Source<'_>,
        permissions: &dyn Permissions,
        snapshot: &Snapshot<'_>,
    ) -> Result<Reply, Error> {
        let trimmed = input.trim_start().trim_start_matches('/');
        let args = tokenize(trimmed)?;
        if args.is_empty() || args[0].is_empty() {
            return Err(Error::Empty);
        }
        let label = args[0].to_ascii_lowercase();
        let key = self
            .routes
            .get(&label)
            .ok_or_else(|| Error::Unknown(label.clone()))?;
        let spec = self
            .commands
            .get(key)
            .expect("route belongs to registered command");
        if !source.has(&spec.permission, permissions) {
            return Err(Error::Denied(spec.permission.clone()));
        }
        let mut values = vec![];
        let mut idx = 1;
        for param in &spec.parameters {
            match param.kind {
                Argument::Greedy => {
                    if idx >= args.len() {
                        if !param.optional {
                            return Err(Error::Usage(Self::usage(spec)));
                        }
                    } else {
                        values.push(args[idx..].join(" "));
                        idx = args.len();
                    }
                }
                Argument::Word => {
                    if idx < args.len() {
                        values.push(args[idx].clone());
                        idx += 1;
                    } else if !param.optional {
                        return Err(Error::Usage(Self::usage(spec)));
                    }
                }
                Argument::Integer { min, max } => {
                    if idx < args.len() {
                        let n = args[idx]
                            .parse::<i64>()
                            .map_err(|_| Error::Usage(Self::usage(spec)))?;
                        if !(min..=max).contains(&n) {
                            return Err(Error::Usage(Self::usage(spec)));
                        }
                        values.push(n.to_string());
                        idx += 1;
                    } else if !param.optional {
                        return Err(Error::Usage(Self::usage(spec)));
                    }
                }
            }
        }
        if idx < args.len()
            && !(matches!(spec.handler, Handler::Plugin { .. }) && spec.parameters.is_empty())
        {
            return Err(Error::Usage(Self::usage(spec)));
        }
        let mut reply = Reply {
            lines: vec![],
            effect: Effect::None,
        };
        match &spec.handler {
            Handler::Core => {
                reply.effect = Effect::Core {
                    command: spec.name.clone(),
                    args: args[1..].join(" "),
                };
            }
            Handler::Plugin { owner } => {
                reply.effect = Effect::Plugin {
                    owner: owner.to_string(),
                    command: spec.name.clone(),
                    args: args[1..].join(" "),
                };
            }
            Handler::Builtin(command) => match command {
                Builtin::Help => {
                    if let Some(topic) = values.first() {
                        let label = topic.trim_start_matches('/').to_ascii_lowercase();
                        let candidate = self
                            .routes
                            .get(&label)
                            .and_then(|key| self.commands.get(key))
                            .filter(|cmd| source.has(&cmd.permission, permissions));
                        match candidate {
                            Some(command) => reply.lines.push(format!(
                                "{} — {}",
                                Self::usage(command),
                                command.description
                            )),
                            None => return Err(Error::Unknown(label)),
                        }
                    } else {
                        let entries = self.visible(source, permissions);
                        reply
                            .lines
                            .push(format!("Available commands: {}", entries.join(", ")));
                    }
                }
                Builtin::Stop => reply.effect = Effect::Stop,
                Builtin::Version => reply.lines.push(snapshot.version.to_owned()),
                Builtin::Plugins => reply.lines.push(snapshot.plugins.to_owned()),
                Builtin::Uptime => reply
                    .lines
                    .push(format!("Uptime: {} seconds", snapshot.uptime.as_secs())),
                Builtin::Tps => reply.lines.push(format!(
                    "Target TPS: 20; tick: {} (measured TPS not implemented)",
                    snapshot.tick
                )),
                Builtin::Tree => {
                    let tree = self.project(source, permissions);
                    let valid = tree.validate().is_ok();
                    reply.lines.push(format!(
                        "Command tree: {} nodes, {} root labels, registry revision {}, valid: {} (protocol-neutral only)",
                        tree.nodes.len(), tree.labels().len(), tree.registry_revision, valid
                    ));
                }
                Builtin::Echo => reply.lines.push(values.join(" ")),
            },
        }
        Ok(reply)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn state<'a>() -> Snapshot<'a> {
        Snapshot {
            version: "test",
            plugins: "no plugins",
            tick: 20,
            uptime: Duration::from_secs(42),
        }
    }
    fn player<'a>() -> Source<'a> {
        Source::Player {
            id: "steve",
            world: Some("minecraft:overworld"),
        }
    }
    #[test]
    fn quotes_and_escapes() {
        assert_eq!(
            tokenize(r#"say "hello world" one\ two ''"#).unwrap(),
            vec!["say", "hello world", "one two", ""]
        );
        assert!(tokenize("hi \"open").is_err());
        assert!(tokenize("hi \\").is_err());
    }
    #[test]
    fn tokenizer_rejects_large_input() {
        assert_eq!(tokenize(&"a".repeat(MAX_LINE + 1)), Err(Error::TooLong));
        assert_eq!(
            tokenize(&vec!["x"; MAX_TOKENS + 1].join(" ")),
            Err(Error::TooLong)
        );
    }
    #[test]
    fn console_dispatch_and_aliases() {
        let r = Registry::builtins();
        let g = Grants::default();
        assert_eq!(
            r.execute("/ver", &Source::Console, &g, &state())
                .unwrap()
                .lines,
            vec!["test"]
        );
        assert_eq!(
            r.execute("quit", &Source::Console, &g, &state())
                .unwrap()
                .effect,
            Effect::Stop
        );
        assert_eq!(
            r.execute(
                r#"/echo "hello world" second"#,
                &Source::Console,
                &g,
                &state()
            )
            .unwrap()
            .lines,
            vec!["hello world second"]
        );
        assert_eq!(
            r.execute("echo", &Source::Console, &g, &state()).err(),
            Some(Error::Usage("/echo <text>".into()))
        );
    }
    #[test]
    fn permission_denies_fail_closed_and_filters_help() {
        let r = Registry::builtins();
        let mut g = Grants::default();
        assert!(matches!(
            r.execute("/stop", &player(), &g, &state()),
            Err(Error::Denied(_))
        ));
        g.set("steve", "horizon.command.help", true, None);
        assert!(!r.execute("help", &player(), &g, &state()).unwrap().lines[0].contains("stop"));
        g.set(
            "steve",
            "horizon.command.stop",
            true,
            Some("minecraft:overworld"),
        );
        assert_eq!(
            r.execute("/stop", &player(), &g, &state()).unwrap().effect,
            Effect::Stop
        );
        assert!(!r
            .complete("st", &player(), &Grants::default())
            .contains(&"stop".to_string()));
    }
    #[test]
    fn specific_deny_overrides_wildcards() {
        let r = Registry::builtins();
        let mut g = Grants::default();
        g.set("group:admin", "horizon.command.*", true, None);
        g.set("steve", "horizon.command.stop", false, None);
        g.add_group("steve", "admin");
        assert!(r.execute("version", &player(), &g, &state()).is_ok());
        assert!(matches!(
            r.execute("stop", &player(), &g, &state()),
            Err(Error::Denied(_))
        ));
    }
    #[test]
    fn namespace_duplicates_alias_reservation_and_removal() {
        let mut r = Registry::builtins();
        let plugin = || Command {
            name: "mymod:warp".into(),
            aliases: vec!["warp".into()],
            permission: "mymod.warp".into(),
            description: "Example".into(),
            parameters: vec![],
            handler: Handler::Plugin {
                owner: "mymod".into(),
            },
        };
        r.register(plugin()).unwrap();
        assert!(matches!(r.register(plugin()), Err(Error::Duplicate(_))));
        assert_eq!(r.unregister_owner("mymod"), 1);
        assert!(matches!(
            r.execute("warp", &Source::Console, &Grants::default(), &state()),
            Err(Error::Unknown(_))
        ));
        assert_eq!(r.unregister_owner("mymod"), 0);
    }
    #[test]
    fn plugin_dispatch_and_validation() {
        let mut r = Registry::builtins();
        r.register(Command {
            name: "hello".into(),
            aliases: vec!["hi".into()],
            permission: "hello.use".into(),
            description: "Say hello".into(),
            parameters: vec![],
            handler: Handler::Plugin {
                owner: "rust:test".into(),
            },
        })
        .unwrap();
        let outcome = r
            .execute(
                r#"hi "alice smith""#,
                &Source::Console,
                &Grants::default(),
                &state(),
            )
            .unwrap();
        assert_eq!(
            outcome.effect,
            Effect::Plugin {
                owner: "rust:test".into(),
                command: "hello".into(),
                args: "alice smith".into()
            }
        );
        assert!(r
            .complete("he", &Source::Console, &Grants::default())
            .contains(&"hello".to_string()));
    }
    #[test]
    fn contextual_help_filters_denied_nodes() {
        let r = Registry::builtins();
        let g = Grants::default();
        let a = r
            .execute("help echo", &Source::Console, &g, &state())
            .unwrap();
        assert!(a.lines[0].contains("/echo <text>"));
        assert_eq!(
            r.execute("help stop", &player(), &g, &state()).err(),
            Some(Error::Denied("horizon.command.help".into()))
        );
        let mut grants = Grants::default();
        grants.set("steve", "horizon.command.help", true, None);
        assert_eq!(
            r.execute("help stop", &player(), &grants, &state()).err(),
            Some(Error::Unknown("stop".into()))
        );
    }
    #[test]
    fn contextual_permissions_and_group_inheritance() {
        let mut g = Grants::default();
        g.set(
            "group:builders",
            "worldedit.fill",
            true,
            Some("minecraft:overworld"),
        );
        g.add_group("group:admin", "builders");
        g.add_group("steve", "admin");
        assert!(g.allowed("steve", "worldedit.fill", Some("minecraft:overworld")));
        assert!(!g.allowed("steve", "worldedit.fill", Some("minecraft:the_nether")));
    }
    #[test]
    fn commandtree_console_diagnostic_uses_live_registry() {
        let mut r = Registry::builtins();
        let initial = r
            .execute(
                "commandtree",
                &Source::Console,
                &Grants::default(),
                &state(),
            )
            .unwrap()
            .lines;
        assert!(initial[0].contains("valid: true"));
        r.register(Command {
            name: "demo:ping".into(),
            aliases: vec![],
            permission: "demo.ping".into(),
            description: "demo".into(),
            parameters: vec![],
            handler: Handler::Plugin {
                owner: "demo".into(),
            },
        })
        .unwrap();
        let updated = r
            .execute(
                "commandtree",
                &Source::Console,
                &Grants::default(),
                &state(),
            )
            .unwrap()
            .lines;
        assert_ne!(initial, updated);
    }
    #[test]
    fn registered_typed_plugin_rejects_extra_arguments() {
        let mut r = Registry::default();
        r.register(Command {
            name: "typed".into(),
            aliases: vec![],
            permission: "example.typed".into(),
            description: "demo".into(),
            parameters: vec![Parameter {
                label: "number",
                kind: Argument::Integer { min: 0, max: 10 },
                optional: false,
            }],
            handler: Handler::Plugin {
                owner: "demo".into(),
            },
        })
        .unwrap();
        let p = Grants::default();
        assert!(r.execute("typed 4", &Source::Console, &p, &state()).is_ok());
        assert_eq!(
            r.execute("typed 4 extra", &Source::Console, &p, &state())
                .err(),
            Some(Error::Usage("/typed <number>".into()))
        );
    }
    #[test]
    fn registration_rejects_oversized_metadata_without_mutation() {
        let mut r = Registry::default();
        let value = Command {
            name: "example".into(),
            aliases: (0..33).map(|i| format!("alias{i}")).collect(),
            permission: "demo".into(),
            description: "demo".into(),
            parameters: vec![],
            handler: Handler::Plugin {
                owner: "demo".into(),
            },
        };
        assert_eq!(r.register(value), Err(Error::TooLong));
        assert!(r
            .project(&Source::Console, &Grants::default())
            .labels()
            .is_empty());
    }
    #[test]
    fn integer_bounds_are_checked() {
        let mut r = Registry::builtins();
        r.register(Command {
            name: "count".into(),
            aliases: vec![],
            permission: "test.count".into(),
            description: "Demo".into(),
            parameters: vec![Parameter {
                label: "amount",
                kind: Argument::Integer { min: 1, max: 10 },
                optional: false,
            }],
            handler: Handler::Plugin {
                owner: "test".into(),
            },
        })
        .unwrap();
        let g = Grants::default();
        assert!(r.execute("count 4", &Source::Console, &g, &state()).is_ok());
        assert!(r
            .execute("count 11", &Source::Console, &g, &state())
            .is_err());
        assert!(r
            .execute("count bananas", &Source::Console, &g, &state())
            .is_err());
    }
}
