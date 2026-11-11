//! The `mcp` command's opt-in flags and the `Restrictions` they produce.

use agent_desktop_core::session::{resolve_active_session, session_dir};
use clap::{Arg, ArgAction, ArgMatches};

use super::restrictions::Restrictions;

const ALLOW_LAUNCH: &str = "allow-launch";
const ALLOW_BATCH: &str = "allow-batch";
const ALLOW_FILE_WRITE: &str = "allow-file-write";
const ALLOW_SESSION_GC: &str = "allow-session-gc";
const ALLOW_CDP: &str = "allow-cdp";

pub(crate) fn flags() -> [Arg; 5] {
    let flag = |name: &'static str, help: &'static str| {
        Arg::new(name)
            .long(name)
            .action(ArgAction::SetTrue)
            .help(help)
    };
    [
        flag(
            ALLOW_LAUNCH,
            "Expose launch, which starts any program this user can run (never with --cwd)",
        ),
        flag(
            ALLOW_BATCH,
            "Expose batch, which runs several commands in one call (the same limits apply to each)",
        ),
        flag(
            ALLOW_FILE_WRITE,
            "Let tools read and write files, only inside <session dir>/mcp-output (created at startup); needs --session",
        ),
        flag(
            ALLOW_SESSION_GC,
            "Expose session gc, which deletes ended or stale sessions anywhere under the state directory, not only this server's output",
        ),
        flag(
            ALLOW_CDP,
            "Let launch open a DevTools or inspector endpoint (cdp, --remote-debugging-*, --inspect*)",
        ),
    ]
}

/// Reads the opt-in flags. File output needs a session, so that its directory exists to stay inside.
pub(crate) fn restrictions(matches: &ArgMatches) -> Result<Restrictions, String> {
    let on = |name: &str| matches.get_flag(name);
    let mut limits = Restrictions {
        launch: on(ALLOW_LAUNCH),
        batch: on(ALLOW_BATCH),
        file_write: on(ALLOW_FILE_WRITE),
        session_gc: on(ALLOW_SESSION_GC),
        cdp: on(ALLOW_CDP),
        file_root: None,
    };
    if limits.file_write {
        let session = resolve_active_session(
            matches.get_one::<String>("session").map(String::as_str),
            std::env::var("AGENT_DESKTOP_SESSION").ok().as_deref(),
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!("--{ALLOW_FILE_WRITE} needs --session <id> or AGENT_DESKTOP_SESSION, so file output has a session directory to stay in")
        })?;
        let dir = session_dir(&session).map_err(|error| error.to_string())?;
        limits.file_root = Some(Restrictions::output_root(&dir));
    }
    Ok(limits)
}

/// Creates the output directory file arguments must stay inside, so the child can resolve it before any file exists.
pub(crate) fn create_output_root(limits: &Restrictions) -> Result<(), String> {
    let Some(root) = limits.file_root.as_deref() else {
        return Ok(());
    };
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
