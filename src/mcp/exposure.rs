//! Which tools and arguments the MCP catalog advertises under a set of `Restrictions`. The child process enforces the
//! same limits on the parsed command, so hiding here keeps the schema honest and is not the only barrier.

use std::path::Path;

use super::restrictions::Restrictions;

pub(crate) const TRACE_EXPORT_FILE: &str = "trace-export.html";

fn steps(path: &[String]) -> Vec<&str> {
    path.iter().map(String::as_str).collect()
}

/// False when the tool is off under `limits`.
pub(crate) fn tool_exposed(path: &[String], limits: &Restrictions) -> bool {
    match steps(path).as_slice() {
        ["launch"] => limits.launch,
        ["batch"] => limits.batch,
        ["session", "gc"] => limits.session_gc,
        _ => true,
    }
}

/// An argument that names a file the command reads or writes, and so stays inside the session output directory.
pub(crate) fn is_path_argument(path: &[String], arg_id: &str) -> bool {
    matches!(
        (steps(path).as_slice(), arg_id),
        (["screenshot"], "output_path")
            | (["trace", "export"], "out")
            | (["clipboard-get"], "out")
            | (["clipboard-set"], "image" | "file_url")
            | (["cursor-overlay", _], "image" | "pointer_image")
    )
}

/// False when the argument is off under `limits`.
pub(crate) fn arg_exposed(path: &[String], arg_id: &str, limits: &Restrictions) -> bool {
    if is_path_argument(path, arg_id) {
        return limits.file_write;
    }
    match (steps(path).as_slice(), arg_id) {
        (["press" | "key-down" | "key-up" | "close-app"], "force") | (["launch"], "cwd") => false,
        (["launch"], "cdp") => limits.cdp,
        _ => true,
    }
}

/// A relative file argument means a file below `root`; an absolute one is left for the child to check.
pub(crate) fn rebase(root: &Path, value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute() || value.starts_with(['/', '\\']) {
        return value.to_string();
    }
    root.join(path).to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "exposure_tests.rs"]
mod tests;
