// SPDX-License-Identifier: AGPL-3.0-or-later
//! Always-on original Rust administrative primitives.
//! These are neither Java plugins nor complete reimplementations of
//! LuckPerms/EssentialsX. They will feed the future player/world APIs.
pub mod permissions;
pub mod utilities;

use crate::command::{Argument, Command, Handler, Parameter, Registry};
use permissions::{Decision, NativePermissions};
use std::time::{SystemTime, UNIX_EPOCH};
use utilities::{NativeUtilities, Position};

fn usage() -> &'static str {
    "permissions <set PRINCIPAL NODE allow|deny|clear [WORLD|-] [EXPIRY_UNIX]|check PRINCIPAL NODE [WORLD|-]|parent PRINCIPAL GROUP|unparent PRINCIPAL GROUP>"
}
pub fn register(registry: &mut Registry) {
    let specs: [(&str, &[&str], &str, bool); 11] = [
        (
            "permissions",
            &["perm"],
            "Manage native Rust permission grants and groups",
            true,
        ),
        (
            "homes",
            &[],
            "List a player's saved homes (console data inspection)",
            true,
        ),
        (
            "home",
            &[],
            "Inspect a saved home; teleport unavailable until play works",
            true,
        ),
        (
            "sethome",
            &[],
            "Store a player's home from explicit coordinates",
            true,
        ),
        ("delhome", &[], "Delete a player's stored home", true),
        ("warps", &[], "List server warps", false),
        (
            "warp",
            &[],
            "Inspect saved warp; no player teleport yet",
            true,
        ),
        (
            "setwarp",
            &[],
            "Store server warp from explicit coordinates",
            true,
        ),
        ("delwarp", &[], "Delete server warp", true),
        (
            "nativefeatures",
            &[],
            "Show available built-in Rust services",
            false,
        ),
        ("permissioncheck", &[], "Inspect a permission result", true),
    ];
    for (name, aliases, desc, arguments) in specs {
        registry
            .register(Command {
                name: name.into(),
                aliases: aliases.iter().map(ToString::to_string).collect(),
                permission: format!("horizon.core.{name}"),
                description: desc.into(),
                parameters: if arguments {
                    vec![Parameter {
                        label: "arguments",
                        kind: Argument::Greedy,
                        optional: true,
                    }]
                } else {
                    vec![]
                },
                handler: Handler::Core,
            })
            .expect("unique built-in native command names");
    }
}
fn position(args: &[&str]) -> Result<Position, String> {
    if args.len() != 4 {
        return Err("expected WORLD X Y Z".into());
    }
    let x = args[1].parse::<f64>().map_err(|_| "X must be a number")?;
    let y = args[2].parse::<f64>().map_err(|_| "Y must be a number")?;
    let z = args[3].parse::<f64>().map_err(|_| "Z must be a number")?;
    Position::new(args[0], x, y, z).map_err(|e| e.to_string())
}
fn fmt_pos(pos: &Position) -> String {
    format!("{} ({}, {}, {})", pos.world, pos.x, pos.y, pos.z)
}
pub fn dispatch(
    name: &str,
    args: &str,
    permissions: &mut NativePermissions,
    utilities: &mut NativeUtilities,
) -> Result<String, String> {
    let words: Vec<_> = args.split_whitespace().collect();
    match name {
        "permissions" => match words.as_slice() {
            ["set",principal,node,decision] | ["set",principal,node,decision,_] | ["set",principal,node,decision,_,_] => {
                let decision=match *decision {
                    "allow"=>Decision::Allow, "deny"=>Decision::Deny, "clear"=>Decision::Undefined,
                    _=>return Err(usage().into()),
                };
                let world=words.get(4).copied().filter(|s|*s!="-");
                let expires=words.get(5).map(|s|s.parse::<u64>().map_err(|_|"expiry must be UNIX seconds")).transpose()?;
                permissions.set(principal,node,decision,world,expires).map_err(|e|e.to_string())?;
                Ok(format!("Native permissions updated: {principal} {node} {decision:?} (revision {})",permissions.revision()))
            },
            ["check",principal,node] | ["check",principal,node,_] => {
                let world=words.get(3).copied().filter(|s|*s!="-");
                let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let decision=permissions.resolve(principal,node,world,now);
                Ok(format!("{principal} {node}: {decision:?}"))
            },
            ["parent",principal,group] => {
                permissions.add_parent(principal,group).map_err(|e|e.to_string())?;
                Ok(format!("Added group:{group} to {principal}"))
            },
            ["unparent",principal,group] => {
                permissions.remove_parent(principal,group).map_err(|e|e.to_string())?;
                Ok(format!("Removed group:{group} from {principal}"))
            },
            [] => Ok(format!("Native permissions active; {}",usage())),
            _=>Err(usage().into())
        },
        "permissioncheck" => match words.as_slice() {
            [principal,node] | [principal,node,_] => {
                let world=words.get(2).copied().filter(|s|*s!="-");
                let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                Ok(format!("{principal}: {node} = {:?}",permissions.resolve(principal,node,world,now)))
            }, _=>Err("permissioncheck PRINCIPAL NODE [WORLD]".into())
        },
        "warps" => Ok(format!("Native warps: {}", utilities.list_warps().join(", "))),
        "warp" => match words.as_slice() {
            [name] => utilities.warp(name).map(|p|format!("{name}: {}; teleport unavailable until play engine exists",fmt_pos(p)))
                .ok_or_else(||format!("No saved warp {name}")),
            _=>Err("warp NAME (lookup only)".into()),
        },
        "setwarp" => match words.as_slice() {
            [name,world,x,y,z] => {
                let location=position(&[*world,*x,*y,*z])?;
                utilities.set_warp(name,location).map_err(|e|e.to_string())?;
                Ok(format!("Stored warp {name}"))
            },
            _=>Err("setwarp NAME WORLD X Y Z".into()),
        },
        "delwarp" => match words.as_slice() {
            [name] => Ok(format!("Deleted warp: {}",utilities.delete_warp(name).map_err(|e|e.to_string())?)),
            _=>Err("delwarp NAME".into())
        },
        "homes" => match words.as_slice() {
            [user] => Ok(format!("Homes for {user}: {}",utilities.list_homes(user).join(", "))),
            _=>Err("homes PLAYER".into())
        },
        "home" => match words.as_slice() {
            [user] | [user,_] => {
                let name=words.get(1).copied().unwrap_or("home");
                utilities.home(user,name).map(|p|format!("Home {user}/{name}: {}; teleport unavailable until play engine exists",fmt_pos(p)))
                    .ok_or_else(||format!("No home {user}/{name}"))
            },
            _=>Err("home PLAYER [NAME] (lookup only)".into())
        },
        "sethome" => match words.as_slice() {
            [user,name,world,x,y,z] => {
                let location=position(&[*world,*x,*y,*z])?;
                utilities.set_home(user,name,location).map_err(|e|e.to_string())?;
                Ok(format!("Stored home {user}/{name}"))
            },
            _=>Err("sethome PLAYER NAME WORLD X Y Z".into())
        },
        "delhome" => match words.as_slice() {
            [user,name] => Ok(format!("Deleted home: {}",utilities.delete_home(user,name).map_err(|e|e.to_string())?)),
            _=>Err("delhome PLAYER NAME".into())
        },
        "nativefeatures" if words.is_empty() => Ok("Built-in Rust permissions (grants/group/context/cache) and utilities (persistent homes/warps); full LuckPerms or EssentialsX parity and teleports NOT implemented".into()),
        _=>Err(format!("Unsupported native command: {name}"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    #[test]
    fn registered_native_commands_dispatch_without_plugins() {
        let base = std::env::temp_dir().join(format!(
            "purpur-horizon-core-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut perms = NativePermissions::open(base.join("permissions.json")).unwrap();
        let mut utils = NativeUtilities::open(base.join("utilities.json")).unwrap();
        let mut reg = Registry::builtins();
        register(&mut reg);
        let names = reg.visible(&command::Source::Console, &perms);
        for name in [
            "permissions",
            "setwarp",
            "warps",
            "sethome",
            "homes",
            "nativefeatures",
        ] {
            assert!(names.contains(&name.into()));
        }
        assert!(dispatch(
            "permissions",
            "set alex essentials.home allow",
            &mut perms,
            &mut utils
        )
        .is_ok());
        assert!(dispatch(
            "permissions",
            "check alex essentials.home",
            &mut perms,
            &mut utils
        )
        .unwrap()
        .contains("Allow"));
        assert!(dispatch("sethome", "Alex base world 1 64 2", &mut perms, &mut utils).is_ok());
        assert!(dispatch("homes", "alex", &mut perms, &mut utils)
            .unwrap()
            .contains("base"));
        assert!(dispatch("setwarp", "spawn world 1 64 2", &mut perms, &mut utils).is_ok());
        assert!(dispatch("warp", "spawn", &mut perms, &mut utils)
            .unwrap()
            .contains("teleport unavailable"));
        let fresh = NativeUtilities::open(base.join("utilities.json")).unwrap();
        assert!(fresh.home("alex", "base").is_some());
        let _ = std::fs::remove_dir_all(base);
    }
    #[test]
    fn malformed_admin_actions_do_not_mutate_services() {
        let base = std::env::temp_dir().join(format!(
            "purpur-horizon-core-bad-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut perms = NativePermissions::open(base.join("permissions.json")).unwrap();
        let mut utils = NativeUtilities::open(base.join("utilities.json")).unwrap();
        assert!(dispatch(
            "permissions",
            "set alex essentials.home garbage",
            &mut perms,
            &mut utils
        )
        .is_err());
        assert!(dispatch(
            "sethome",
            "alex base world NaN 65 0",
            &mut perms,
            &mut utils
        )
        .is_err());
        assert_eq!(
            perms.resolve("alex", "essentials.home", None, 100),
            Decision::Undefined
        );
        assert!(utils.home("alex", "base").is_none());
        let _ = std::fs::remove_dir_all(base);
    }
}
