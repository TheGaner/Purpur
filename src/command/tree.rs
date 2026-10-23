// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic, protocol-neutral projection of the registered command graph.
//! This is NOT a Minecraft 26.3 Commands packet encoder. The wire projection,
//! parser IDs, suggestions ranges and redirect validation require a later phase.
use super::{Argument, Command, Handler};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Root,
    Literal(String),
    Argument {
        label: &'static str,
        kind: ArgumentKind,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    Word,
    Integer { min: i64, max: i64 },
    Greedy,
}
impl From<&Argument> for ArgumentKind {
    fn from(value: &Argument) -> Self {
        match value {
            Argument::Word => Self::Word,
            Argument::Integer { min, max } => Self::Integer {
                min: *min,
                max: *max,
            },
            Argument::Greedy => Self::Greedy,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    pub children: Vec<usize>,
    pub redirect: Option<usize>,
    pub executable: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    /// Changes when commands are registered or unregistered.
    pub registry_revision: u64,
    /// Arena indices are deterministic given the same command definitions and
    /// authorization snapshot, but not stable across snapshots or revisions.
    pub nodes: Vec<Node>,
}
impl Projection {
    fn push(&mut self, parent: usize, kind: Kind, executable: bool) -> usize {
        let index = self.nodes.len();
        self.nodes.push(Node {
            kind,
            children: Vec::new(),
            redirect: None,
            executable,
        });
        self.nodes[parent].children.push(index);
        index
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.nodes.first().map(|n| &n.kind) != Some(&Kind::Root) {
            return Err("first node must be root");
        }
        for (index, node) in self.nodes.iter().enumerate() {
            if node
                .children
                .iter()
                .any(|child| *child <= index || *child >= self.nodes.len())
            {
                return Err("invalid child index");
            }
            if let Some(target) = node.redirect {
                if target >= self.nodes.len() || target == index {
                    return Err("invalid redirect");
                }
            }
        }
        Ok(())
    }
    pub fn labels(&self) -> Vec<&str> {
        self.nodes[0]
            .children
            .iter()
            .filter_map(|index| match &self.nodes[*index].kind {
                Kind::Literal(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }
}
pub(super) fn build(revision: u64, mut visible: Vec<(&str, &Command)>) -> Projection {
    // Hash-map insertion order is deliberately irrelevant to serialized
    // command-tree node IDs, making reconnects reproducible.
    visible.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut graph = Projection {
        registry_revision: revision,
        nodes: vec![Node {
            kind: Kind::Root,
            children: Vec::new(),
            redirect: None,
            executable: false,
        }],
    };
    for (canonical, command) in visible {
        let freeform =
            command.parameters.is_empty() && matches!(command.handler, Handler::Plugin { .. });
        let optional_root = command.parameters.first().is_some_and(|p| p.optional);
        let root_executable = command.parameters.is_empty() || optional_root;
        let root = graph.push(0, Kind::Literal(canonical.to_string()), root_executable);
        let mut parent = root;
        for (i, param) in command.parameters.iter().enumerate() {
            // Optional arguments are a suffix; executing at the parent must
            // also be allowed once an optional argument begins.
            let executable = i + 1 == command.parameters.len()
                || command.parameters[i + 1..].iter().all(|next| next.optional);
            parent = graph.push(
                parent,
                Kind::Argument {
                    label: param.label,
                    kind: ArgumentKind::from(&param.kind),
                },
                executable,
            );
        }
        if freeform {
            // The legacy native ABI accepts arbitrary strings after a command
            // that declared no typed parameters. Show this honestly instead
            // of claiming its argument tree ends at the root.
            graph.push(
                parent,
                Kind::Argument {
                    label: "args",
                    kind: ArgumentKind::Greedy,
                },
                true,
            );
        }
        let mut aliases: Vec<_> = command
            .aliases
            .iter()
            .map(|a| a.to_ascii_lowercase())
            .collect();
        aliases.sort();
        for alias in aliases {
            let node = graph.push(0, Kind::Literal(alias), false);
            graph.nodes[node].redirect = Some(root);
        }
    }
    debug_assert!(graph.validate().is_ok());
    graph
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{
        Builtin, Command, Grants, Handler, Parameter, Permissions, Registry, Source,
    };
    fn root_names(tree: &Projection) -> Vec<&str> {
        tree.labels()
    }
    #[test]
    fn deterministic_with_different_registration_orders() {
        fn cmd(name: &str) -> Command {
            Command {
                name: name.into(),
                aliases: vec![],
                permission: format!("demo.{name}"),
                description: "Demo".into(),
                parameters: vec![],
                handler: Handler::Builtin(Builtin::Version),
            }
        }
        let mut a = Registry::default();
        a.register(cmd("beta")).unwrap();
        a.register(cmd("alpha")).unwrap();
        let mut b = Registry::default();
        b.register(cmd("alpha")).unwrap();
        b.register(cmd("beta")).unwrap();
        let x = a.project(&Source::Console, &Grants::default());
        let y = b.project(&Source::Console, &Grants::default());
        assert_eq!(x.nodes, y.nodes);
        assert_eq!(root_names(&x), vec!["alpha", "beta"]);
        assert!(x.validate().is_ok());
    }
    #[test]
    fn permission_filtered_and_aliases_redirect_to_canonical() {
        let r = Registry::builtins();
        let mut g = Grants::default();
        g.set("steve", "horizon.command.help", true, None);
        let source = Source::Player {
            id: "steve",
            world: None,
        };
        let tree = r.project(&source, &g);
        assert_eq!(root_names(&tree), vec!["help", "?"]);
        let canonical = tree.nodes[0].children[0];
        let alias = tree.nodes[0].children[1];
        assert!(tree.nodes[alias].redirect == Some(canonical));
        assert!(tree.nodes[canonical].executable);
        assert!(!tree.labels().contains(&"stop"));
    }
    #[test]
    fn typed_args_optional_suffix_and_freeform_plugin() {
        let mut r = Registry::default();
        r.register(Command {
            name: "example:typed".into(),
            aliases: vec![],
            permission: "demo".into(),
            description: "types".into(),
            parameters: vec![
                Parameter {
                    label: "required",
                    kind: Argument::Integer { min: 1, max: 10 },
                    optional: false,
                },
                Parameter {
                    label: "optional",
                    kind: Argument::Word,
                    optional: true,
                },
            ],
            handler: Handler::Builtin(Builtin::Version),
        })
        .unwrap();
        r.register(Command {
            name: "example:freeform".into(),
            aliases: vec![],
            permission: "demo".into(),
            description: "free".into(),
            parameters: vec![],
            handler: Handler::Plugin {
                owner: "plugin".into(),
            },
        })
        .unwrap();
        let graph = r.project(&Source::Console, &Grants::default());
        assert_eq!(graph.labels(), vec!["example:freeform", "example:typed"]);
        let f = graph.nodes[0].children[0];
        assert!(graph.nodes[f].executable);
        let rest = graph.nodes[f].children[0];
        assert!(matches!(
            graph.nodes[rest].kind,
            Kind::Argument {
                kind: ArgumentKind::Greedy,
                ..
            }
        ));
        let t = graph.nodes[0].children[1];
        assert!(!graph.nodes[t].executable);
        let integer = graph.nodes[t].children[0];
        assert!(graph.nodes[integer].executable);
        assert!(matches!(
            graph.nodes[integer].kind,
            Kind::Argument {
                kind: ArgumentKind::Integer { min: 1, max: 10 },
                ..
            }
        ));
        assert!(graph.nodes[integer].children.len() == 1);
    }
    #[test]
    fn unregister_changes_revision_and_projection() {
        let mut r = Registry::default();
        let initial = r
            .project(&Source::Console, &Grants::default())
            .registry_revision;
        r.register(Command {
            name: "plugin:foo".into(),
            aliases: vec!["foo".into()],
            permission: "plugin.foo".into(),
            description: "demo".into(),
            parameters: vec![],
            handler: Handler::Plugin {
                owner: "plugin".into(),
            },
        })
        .unwrap();
        assert!(
            r.project(&Source::Console, &Grants::default())
                .registry_revision
                > initial
        );
        assert_eq!(r.unregister_owner("plugin"), 1);
        assert_eq!(
            r.project(&Source::Console, &Grants::default()).labels(),
            Vec::<&str>::new()
        );
    }
    #[test]
    fn fails_closed_even_on_group_permissions() {
        let mut r = Registry::default();
        r.register(Command {
            name: "admin".into(),
            aliases: vec![],
            permission: "admin".into(),
            description: "demo".into(),
            parameters: vec![],
            handler: Handler::Builtin(Builtin::Stop),
        })
        .unwrap();
        let mut g = Grants::default();
        g.set("group:staff", "admin", true, None);
        let user = Source::Player {
            id: "alex",
            world: None,
        };
        assert!(r.project(&user, &g).labels().is_empty());
        g.add_group("alex", "staff");
        assert_eq!(r.project(&user, &g).labels(), vec!["admin"]);
        assert!(g.allowed("alex", "admin", None));
    }
}
