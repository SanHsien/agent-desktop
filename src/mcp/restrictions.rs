//! What an MCP client may ask the CLI to do. The server states the limits to every child it spawns through the
//! environment, and the child refuses a command outside them before it runs, whether the command arrives as a tool
//! call or as an entry of a batch. A client is as trusted as the user who started the server: these limits keep a
//! confused or injected model away from the surfaces that run arbitrary code, they are not a sandbox.

use agent_desktop_core::{AdapterError, AppError, DeliverySemantics, ErrorCode};
use std::path::{Component, Path, PathBuf};

use crate::cli::Commands;
use crate::cli_args::{
    cursor_overlay_action::CursorOverlayAction, session::SessionAction, system::LaunchArgs,
    trace::TraceAction,
};

pub(crate) const ALLOW_ENV: &str = "AGENT_DESKTOP_MCP_ALLOW";
pub(crate) const FILE_ROOT_ENV: &str = "AGENT_DESKTOP_MCP_FILE_ROOT";
const OUTPUT_DIR: &str = "mcp-output";
const DEBUG_SWITCH_PREFIXES: [&str; 3] = ["remote-debugging", "remote-allow-origins", "inspect"];

/// The opt-ins an MCP server was started with. The default allows none of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Restrictions {
    pub launch: bool,
    pub batch: bool,
    pub file_write: bool,
    pub cdp: bool,
    pub file_root: Option<PathBuf>,
}

impl Restrictions {
    /// Every opt-in on and no file root, so only the limits that no opt-in lifts apply.
    #[cfg(test)]
    pub(crate) fn all_allowed() -> Self {
        Self {
            launch: true,
            batch: true,
            file_write: true,
            cdp: true,
            file_root: None,
        }
    }

    /// The directory file arguments must stay inside: a subfolder of the session directory, so a client cannot
    /// overwrite the session's own manifest or trace.
    pub(crate) fn output_root(session_dir: &Path) -> PathBuf {
        session_dir.join(OUTPUT_DIR)
    }

    pub(crate) fn to_env(&self) -> Vec<(&'static str, String)> {
        let allowed = [
            ("launch", self.launch),
            ("batch", self.batch),
            ("file-write", self.file_write),
            ("cdp", self.cdp),
        ]
        .into_iter()
        .filter_map(|(name, on)| on.then_some(name))
        .collect::<Vec<_>>()
        .join(",");
        let mut env = vec![(ALLOW_ENV, allowed)];
        if let Some(root) = &self.file_root {
            env.push((FILE_ROOT_ENV, root.to_string_lossy().into_owned()));
        }
        env
    }

    /// The limits this process was started under, or `None` when it was not started by an MCP server.
    pub(crate) fn from_env() -> Option<Self> {
        let allowed = std::env::var(ALLOW_ENV).ok()?;
        let root = std::env::var_os(FILE_ROOT_ENV).map(PathBuf::from);
        Some(Self::parse(&allowed, root))
    }

    fn parse(allowed: &str, file_root: Option<PathBuf>) -> Self {
        let has = |name: &str| allowed.split(',').any(|item| item.trim() == name);
        Self {
            launch: has("launch"),
            batch: has("batch"),
            file_write: has("file-write"),
            cdp: has("cdp"),
            file_root,
        }
    }

    /// Refuses `command` when it is outside the limits.
    pub(crate) fn check(&self, command: &Commands) -> Result<(), AppError> {
        match command {
            Commands::Press(args) => self.refuse_force(args.force, command.name()),
            Commands::KeyDown(args) | Commands::KeyUp(args) => {
                self.refuse_force(args.force, command.name())
            }
            Commands::Launch(args) => self.check_launch(args),
            Commands::Batch(_) => self.require(self.batch, "batch", "--allow-batch"),
            Commands::Session(args) => match args.action {
                SessionAction::Gc(_) => {
                    self.require(self.file_write, "session gc", "--allow-file-write")
                }
                _ => Ok(()),
            },
            Commands::Screenshot(args) => self.check_optional_path(args.output_path.as_deref()),
            Commands::ClipboardGet(args) => self.check_optional_path(args.out.as_deref()),
            Commands::ClipboardSet(args) => {
                self.check_optional_path(args.image.as_deref())?;
                args.file_url
                    .iter()
                    .try_for_each(|url| self.check_path(Path::new(url)))
            }
            Commands::Trace(args) => match &args.action {
                TraceAction::Export(export) => match export.out.as_deref() {
                    Some(out) => self.check_path(out),
                    None => Err(denied(
                        "trace export needs an explicit out path under the session output directory",
                        "Pass out; a direct tool call may leave it out only when the server allows file output.",
                    )),
                },
                TraceAction::Show(_) => Ok(()),
            },
            Commands::CursorOverlay(args) => match &args.action {
                CursorOverlayAction::Enable(enable) => {
                    let images = &enable.style.images;
                    self.check_optional_path(images.image.as_deref())?;
                    self.check_optional_path(images.pointer_image.as_deref())
                }
                CursorOverlayAction::Disable => Ok(()),
            },
            _ => Ok(()),
        }
    }

    fn check_launch(&self, args: &LaunchArgs) -> Result<(), AppError> {
        self.require(self.launch, "launch", "--allow-launch")?;
        if args.cwd.is_some() {
            return Err(denied(
                "launch --cwd is not available to MCP clients: the working directory picks which DLLs the new process loads",
                "Omit cwd; the app starts in its own default directory.",
            ));
        }
        let debug_switch = args.args.iter().any(|arg| is_debug_switch(arg));
        if args.cdp.is_some() || debug_switch {
            return self.require(
                self.cdp,
                "launch with a DevTools or inspector endpoint (cdp, --remote-debugging-*, --inspect*)",
                "--allow-cdp",
            );
        }
        Ok(())
    }

    fn refuse_force(&self, force: bool, command: &str) -> Result<(), AppError> {
        if !force {
            return Ok(());
        }
        Err(denied(
            format!(
                "{command} --force is not available to MCP clients: it bypasses the blocked-shortcut list (win+r, alt+f4, ...)"
            ),
            "Send the combo without force; a person can run it from the CLI if it is really intended.",
        ))
    }

    fn check_optional_path(&self, path: Option<&Path>) -> Result<(), AppError> {
        path.map_or(Ok(()), |path| self.check_path(path))
    }

    fn check_path(&self, path: &Path) -> Result<(), AppError> {
        self.require(self.file_write, "file path arguments", "--allow-file-write")?;
        let root = self.file_root.as_deref().ok_or_else(|| {
            denied(
                "file path arguments need the server to run with --session so they have a session directory",
                "Restart the MCP server with --session <id> or AGENT_DESKTOP_SESSION set.",
            )
        })?;
        if is_inside(path, root) {
            return Ok(());
        }
        Err(denied(
            format!(
                "'{}' is outside the session output directory '{}'",
                path.display(),
                root.display()
            ),
            "Use an absolute path inside that directory, or a relative path in a direct tool call.",
        ))
    }

    fn require(&self, allowed: bool, what: &str, flag: &str) -> Result<(), AppError> {
        if allowed {
            return Ok(());
        }
        Err(denied(
            format!("{what} is off for MCP clients"),
            format!("The user can start the server with `agent-desktop mcp {flag}` to allow it."),
        ))
    }
}

/// Refuses `command` when this process runs under an MCP server's limits; any other process allows it.
pub(crate) fn enforce(command: &Commands) -> Result<(), AppError> {
    Restrictions::from_env().map_or(Ok(()), |limits| limits.check(command))
}

fn denied(message: impl Into<String>, suggestion: impl Into<String>) -> AppError {
    AdapterError::new(ErrorCode::PolicyDenied, message)
        .with_suggestion(suggestion)
        .with_disposition(DeliverySemantics::not_delivered())
        .into()
}

fn is_debug_switch(argument: &str) -> bool {
    let switch = argument.trim_start_matches(['-', '/']).to_ascii_lowercase();
    DEBUG_SWITCH_PREFIXES
        .iter()
        .any(|prefix| switch.starts_with(prefix))
}

/// True when `path` is an absolute path strictly below `root`, with no `.` or `..` step, and, where it already
/// exists, resolves below the resolved `root` too, so a junction or symlink inside `root` cannot lead out of it.
fn is_inside(path: &Path, root: &Path) -> bool {
    if !path.is_absolute() || !has_only_plain_steps(path) || !is_below(path, root) {
        return false;
    }
    let Ok(resolved_root) = root.canonicalize() else {
        return true;
    };
    let Some(existing) = path.ancestors().find(|ancestor| ancestor.exists()) else {
        return true;
    };
    existing
        .canonicalize()
        .is_ok_and(|resolved| is_below(&resolved, &resolved_root) || resolved == resolved_root)
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

#[cfg(test)]
#[path = "restrictions_tests.rs"]
mod tests;
