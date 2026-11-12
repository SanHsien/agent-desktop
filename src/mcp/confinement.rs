//! The path and argument checks behind the MCP limits: where a file argument may point, which environment variables a
//! launch may set, and which text opens a debugging endpoint.

use std::ffi::OsStr;
use std::path::{Component, Path};

const DEBUG_MARKERS: [&str; 10] = [
    "remote-debugging",
    "remote-allow-origins",
    "inspect",
    "marionette",
    "start-debugger-server",
    "jdwp",
    "agentlib",
    "xrunjdwp",
    "debug-port",
    "debug-brk",
];
const INSPECTOR_ENV_KEYS: [&str; 5] = [
    "NODE_OPTIONS",
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
];
const ELECTRON_ENV_PREFIX: &str = "ELECTRON_";
/// Variables that decide which program, DLL, module or helper a new process finds or loads, or that name a program
/// the process runs on its own (an SSH, askpass, editor or pager helper, a Git config injection). Refused for MCP
/// clients under every opt-in, because none of them is needed to drive a managed app.
const CODE_LOADING_ENV_KEYS: &[&str] = &[
    "PATH",
    "PATHEXT",
    "COMSPEC",
    "PSMODULEPATH",
    "PYTHONPATH",
    "PYTHONSTARTUP",
    "PYTHONHOME",
    "PYTHONUSERBASE",
    "NODE_PATH",
    "NODE_REPL_EXTERNAL_MODULE",
    "CLASSPATH",
    "DOTNET_STARTUP_HOOKS",
    "DOTNET_ADDITIONAL_DEPS",
    "PERL5LIB",
    "PERL5OPT",
    "RUBYLIB",
    "RUBYOPT",
    "BASH_ENV",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "GIT_EXEC_PATH",
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "GIT_EDITOR",
    "GIT_PAGER",
    "GIT_PROXY_COMMAND",
    "GIT_EXTERNAL_DIFF",
];
/// Git reads its configuration, including `core.sshCommand` and `core.pager`, from these variables.
const GIT_CONFIG_ENV_PREFIX: &str = "GIT_CONFIG_";
/// The .NET and CoreCLR profiler settings in every spelling, with or without an architecture suffix such as `_64`.
/// A profiler loads a native DLL into the process, so any name under these prefixes that mentions `PROFIL` is refused.
const PROFILER_ENV_PREFIXES: &[&str] = &["COR_", "CORECLR_", "DOTNET_"];
const PROFILER_ENV_MARKER: &str = "PROFIL";
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

/// Variables that decide which program, DLL or module a new process finds or loads, or that make it run a helper.
pub(super) fn is_code_loading_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    CODE_LOADING_ENV_KEYS.contains(&upper.as_str())
        || upper.starts_with(GIT_CONFIG_ENV_PREFIX)
        || PROFILER_ENV_PREFIXES
            .iter()
            .any(|prefix| upper.starts_with(prefix) && upper.contains(PROFILER_ENV_MARKER))
}

/// Windows folds the case of non-ASCII letters in variable names (a dotless i matches `I`), which no ASCII
/// comparison above sees, so a name with any non-ASCII character is refused outright.
pub(super) fn has_non_ascii_name(key: &str) -> bool {
    !key.is_ascii()
}

/// True when `path` is an absolute path strictly below `root` that names only ordinary files and folders, with no
/// `.` or `..` step. Every component below `root` is read with `symlink_metadata`, which does not follow links, and
/// any symlink, junction or other reparse point there is refused whether or not its target exists. A component that
/// does not exist ends the walk, since nothing can exist under it. The deepest existing part must also resolve below
/// `root`. `root` must exist; anything that cannot be resolved is refused.
pub(super) fn is_inside(path: &Path, root: &Path) -> bool {
    if !path.is_absolute()
        || !has_only_plain_steps(path)
        || !is_below(path, root)
        || !names_below_are_plain(path, root)
        || !no_link_below(path, root)
    {
        return false;
    }
    let Ok(resolved_root) = root.canonicalize() else {
        return false;
    };
    let Some(existing) = path
        .ancestors()
        .find(|ancestor| std::fs::symlink_metadata(ancestor).is_ok())
    else {
        return false;
    };
    existing
        .canonicalize()
        .is_ok_and(|resolved| resolved == resolved_root || is_below(&resolved, &resolved_root))
}

/// Walks the components below `root` one at a time without following links. A link of any kind on that walk, even
/// a dangling one, returns false; a walk that cannot read a component also returns false.
fn no_link_below(path: &Path, root: &Path) -> bool {
    let mut current = root.to_path_buf();
    for part in path.components().skip(root.components().count()) {
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if is_link_like(&metadata) => return false,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
            Err(_) => return false,
        }
    }
    true
}

/// A symlink, or on Windows any reparse point (a junction, a mount point, or a cloud placeholder).
fn is_link_like(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
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
        upper.strip_prefix(device).is_some_and(|rest| {
            !rest.is_empty()
                && rest
                    .chars()
                    .all(|digit| matches!(digit, '0'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}'))
        })
    })
}

#[cfg(test)]
#[path = "confinement_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "confinement_env_tests.rs"]
mod env_tests;
