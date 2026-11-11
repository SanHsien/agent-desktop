//! The path and argument checks behind the MCP limits: where a file argument may point, which environment variables a
//! launch may set, and which text opens a debugging endpoint.

use std::ffi::OsStr;
use std::path::{Component, Path};

const DEBUG_MARKERS: [&str; 4] = [
    "remote-debugging",
    "remote-allow-origins",
    "inspect",
    "marionette",
];
const INSPECTOR_ENV_KEYS: [&str; 2] = ["NODE_OPTIONS", "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"];
const ELECTRON_ENV_PREFIX: &str = "ELECTRON_";
const PROGRAM_SEARCH_ENV_KEYS: [&str; 3] = ["PATH", "PATHEXT", "COMSPEC"];
const RESERVED_DEVICES: [&str; 6] = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];
const NUMBERED_DEVICES: [&str; 2] = ["COM", "LPT"];
const FORBIDDEN_NAME_CHARACTERS: [char; 7] = [':', '<', '>', '"', '|', '?', '*'];

/// True when `text` contains, in any letter case, a switch or option name that opens a DevTools or inspector
/// endpoint. A substring scan, because a program such as `cmd /c start msedge --remote-debugging-port=9222` carries
/// the switch inside one argument.
pub(super) fn mentions_debug_endpoint(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    DEBUG_MARKERS.iter().any(|marker| lowered.contains(marker))
}

/// The variable name of a `KEY=VALUE` launch environment entry.
pub(super) fn env_key(entry: &str) -> &str {
    entry.split('=').next().unwrap_or_default().trim()
}

/// Variables that make a program open an inspector or a remote-debugging port on their own.
pub(super) fn is_inspector_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    INSPECTOR_ENV_KEYS.contains(&upper.as_str()) || upper.starts_with(ELECTRON_ENV_PREFIX)
}

/// Variables that decide which executable or DLL a new process finds first.
pub(super) fn is_program_search_env_key(key: &str) -> bool {
    PROGRAM_SEARCH_ENV_KEYS.contains(&key.to_ascii_uppercase().as_str())
}

/// True when `path` is an absolute path strictly below `root` that names only ordinary files and folders, with no
/// `.` or `..` step. `root` must exist and the deepest existing part of `path` must resolve below it, so a junction
/// or symlink inside `root` cannot lead out of it; anything that cannot be resolved is refused.
pub(super) fn is_inside(path: &Path, root: &Path) -> bool {
    if !path.is_absolute()
        || !has_only_plain_steps(path)
        || !is_below(path, root)
        || !names_below_are_plain(path, root)
    {
        return false;
    }
    let Ok(resolved_root) = root.canonicalize() else {
        return false;
    };
    let Some(existing) = path.ancestors().find(|ancestor| ancestor.exists()) else {
        return false;
    };
    existing
        .canonicalize()
        .is_ok_and(|resolved| resolved == resolved_root || is_below(&resolved, &resolved_root))
}

fn has_only_plain_steps(path: &Path) -> bool {
    !path
        .components()
        .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
}

fn is_below(path: &Path, root: &Path) -> bool {
    let fold = |part: Component| {
        let text = part.as_os_str().to_string_lossy().into_owned();
        if cfg!(windows) {
            text.to_ascii_lowercase()
        } else {
            text
        }
    };
    let path = path.components().map(fold).collect::<Vec<_>>();
    let root = root.components().map(fold).collect::<Vec<_>>();
    path.len() > root.len() && path[..root.len()] == root[..]
}

fn names_below_are_plain(path: &Path, root: &Path) -> bool {
    path.components()
        .skip(root.components().count())
        .all(|part| match part {
            Component::Normal(name) => is_plain_name(name),
            _ => false,
        })
}

/// Refuses what Windows reads as something other than a file: reserved device names with or without an extension
/// (`NUL.png`, `COM1`, `LPT¹`), names ending in a dot or a space, alternate data streams, and wildcard characters.
fn is_plain_name(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    if name.is_empty() || name.ends_with(['.', ' ']) || name.contains(FORBIDDEN_NAME_CHARACTERS) {
        return false;
    }
    let stem = name.split('.').next().unwrap_or_default();
    !is_device_stem(&stem.trim_end().to_uppercase())
}

fn is_device_stem(upper: &str) -> bool {
    if RESERVED_DEVICES.contains(&upper) {
        return true;
    }
    NUMBERED_DEVICES.iter().any(|device| {
        let mut rest = upper.strip_prefix(device).unwrap_or_default().chars();
        matches!(
            (rest.next(), rest.next()),
            (Some('0'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}'), None)
        )
    })
}

#[cfg(test)]
#[path = "confinement_tests.rs"]
mod tests;
