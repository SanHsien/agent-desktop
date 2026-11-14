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
const INSPECTOR_ENV_KEYS: [&str; 7] = [
    "NODE_OPTIONS",
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "IBM_JAVA_OPTIONS",
    "OPENJ9_JAVA_OPTIONS",
];
const ELECTRON_ENV_PREFIX: &str = "ELECTRON_";
/// Node's long options that preload a module, install a loader or load OpenSSL engines through a config file, in the
/// dash spelling Node folds `_` into.
const MODULE_LOADING_FLAGS: &[&str] = &[
    "--require",
    "--import",
    "--loader",
    "--experimental-loader",
    "--openssl-config",
    "--snapshot-blob",
];
/// The only options `NODE_OPTIONS` may carry for an MCP client: inspector endpoints, which `--allow-cdp` governs.
const NODE_ENDPOINT_OPTIONS: &[&str] = &[
    "--inspect",
    "--inspect-brk",
    "--inspect-port",
    "--inspect-wait",
    "--debug-port",
];
/// JVM options that load a Java or native agent, refused wherever they appear (wrapper variables such as `JAVA_OPTS`).
const JAVA_AGENT_OPTIONS: &[&str] = &["-javaagent:", "-agentpath:"];
/// Node's short preload option. Node matches it case-sensitively and only with a separate value.
const SHORT_REQUIRE_FLAG: &str = "-r";
/// Variables that decide which program, DLL, module or helper a new process finds or loads, or that name a program
/// the process runs on its own (an SSH, askpass, editor or pager helper, a Git config injection). Refused for MCP
/// clients under every opt-in, because none of them is needed to drive a managed app.
const CODE_LOADING_ENV_KEYS: &[&str] = &[
    "ELECTRON_RUN_AS_NODE",
    "ELECTRON_NO_ASAR",
    "ELECTRON_OVERRIDE_DIST_PATH",
    "ELECTRON_TRASH",
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
    "OPENSSL_CONF",
    "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
    "JAVA_HOME",
    "OPENSSL_ENGINES",
    "OPENSSL_MODULES",
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
/// the switch inside one argument. Option-shaped tokens are scanned again with `_` read as `-`, as Node reads them;
/// variable names such as `APP_DEBUG_PORT` are not option-shaped and stay as written.
pub(super) fn mentions_debug_endpoint(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    DEBUG_MARKERS.iter().any(|marker| lowered.contains(marker))
        || lowered
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '\'' | '"'))
            .map(option_part)
            .filter(|option| option.starts_with('-'))
            .any(|option| {
                let dashed = option.replace('_', "-");
                DEBUG_MARKERS.iter().any(|marker| dashed.contains(marker))
            })
}

/// A token without the leading `KEY=` of an environment entry.
fn option_part(token: &str) -> &str {
    token
        .split_once('=')
        .filter(|(key, _)| !key.starts_with('-'))
        .map_or(token, |(_, value)| value)
}

/// Finds module preloads and custom loaders before any endpoint opt-in. The text is read twice: split on whitespace
/// and quotes, and the way Node reads `NODE_OPTIONS`, which drops quotes and undoes backslash escapes, so a quote or
/// escape spliced into an option name does not hide it.
pub(super) fn mentions_code_loading(text: &str) -> bool {
    let node_reading: String = text
        .chars()
        .filter(|ch| !matches!(ch, '"' | '\'' | '\\'))
        .collect();
    text.split(|ch: char| ch.is_whitespace() || matches!(ch, '\'' | '"'))
        .chain(node_reading.split(char::is_whitespace))
        .any(is_module_loading_option)
}

/// One token: a leading `KEY=` of an environment entry is skipped, long names ignore letter case and treat `_` as `-`
/// as Node does, and `-r` must match exactly.
fn is_module_loading_option(token: &str) -> bool {
    let option = option_part(token);
    if option == SHORT_REQUIRE_FLAG {
        return true;
    }
    let lower = option.to_ascii_lowercase();
    if JAVA_AGENT_OPTIONS
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return true;
    }
    if !option.starts_with("--") {
        return false;
    }
    let name = option
        .split('=')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .replace('_', "-");
    MODULE_LOADING_FLAGS.contains(&name.as_str())
}

/// True when every option in a `NODE_OPTIONS` value is an inspector endpoint. An allow list, because Node keeps adding
/// options that load or run code (`--snapshot-blob` replaces the main script), and a deny list trails behind them.
pub(super) fn node_options_hold_only_endpoints(value: &str) -> bool {
    let node_reading: String = value
        .chars()
        .filter(|ch| !matches!(ch, '"' | '\'' | '\\'))
        .collect();
    node_reading
        .split(char::is_whitespace)
        .filter(|token| token.starts_with('-'))
        .all(|token| {
            let name = token
                .split('=')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .replace('_', "-");
            NODE_ENDPOINT_OPTIONS.contains(&name.as_str())
        })
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

#[path = "confinement_endpoint_env.rs"]
mod endpoint_env;
pub(super) use endpoint_env::inspector_env_value_is_endpoint_only;

#[cfg(test)]
#[path = "confinement_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "confinement_env_tests.rs"]
mod env_tests;

#[cfg(test)]
#[path = "confinement_node_options_tests.rs"]
mod node_options_tests;

#[cfg(test)]
#[path = "confinement_endpoint_env_tests.rs"]
mod endpoint_env_tests;
