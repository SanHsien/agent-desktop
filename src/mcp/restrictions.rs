//! What an MCP client may ask the CLI to do. The server states the limits to every child it spawns through the
//! environment, and the child refuses a command outside them before it runs, whether the command arrives as a tool
//! call or as an entry of a batch. A client is as trusted as the user who started the server: these limits keep a
//! confused or injected model away from the surfaces that run arbitrary code, they are not a sandbox.

use agent_desktop_core::{AdapterError, AppError, DeliverySemantics, ErrorCode};
use std::path::{Path, PathBuf};

use super::confinement::{
    env_key, has_non_ascii_name, is_code_loading_env_key, is_inside, is_inspector_env_key,
    mentions_code_loading, mentions_debug_endpoint, node_options_hold_only_endpoints,
};
use crate::cli::Commands;
use crate::cli_args::{
    cursor_overlay_action::CursorOverlayAction, session::SessionAction, system::LaunchArgs,
    trace::TraceAction,
};

pub(crate) const ALLOW_ENV: &str = "AGENT_DESKTOP_MCP_ALLOW";
pub(crate) const FILE_ROOT_ENV: &str = "AGENT_DESKTOP_MCP_FILE_ROOT";
const OUTPUT_DIR: &str = "mcp-output";
const SHORTCUT_REASON: &str = "it bypasses the blocked-shortcut list (win+r, alt+f4, ...)";
const TERMINATE_REASON: &str = "it terminates the app's processes instead of closing it";

/// The opt-ins an MCP server was started with. The default allows none of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Restrictions {
    pub launch: bool,
    pub batch: bool,
    pub file_write: bool,
    pub session_gc: bool,
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
            session_gc: true,
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
            ("session-gc", self.session_gc),
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
            session_gc: has("session-gc"),
            cdp: has("cdp"),
            file_root,
        }
    }

    /// Refuses `command` when it is outside the limits.
    pub(crate) fn check(&self, command: &Commands) -> Result<(), AppError> {
        match command {
            Commands::Press(args) => self.refuse_force(args.force, command.name(), SHORTCUT_REASON),
            Commands::KeyDown(args) | Commands::KeyUp(args) => {
                self.refuse_force(args.force, command.name(), SHORTCUT_REASON)
            }
            Commands::CloseApp(args) => {
                self.refuse_force(args.force, command.name(), TERMINATE_REASON)
            }
            Commands::Launch(args) => self.check_launch(args),
            Commands::Batch(_) => self.require(self.batch, "batch", "--allow-batch"),
            Commands::Session(args) => match args.action {
                SessionAction::Gc(_) => {
                    self.require(self.session_gc, "session gc", "--allow-session-gc")
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
        if let Some(entry) = args
            .env
            .iter()
            .find(|entry| has_non_ascii_name(env_key(entry)))
        {
            return Err(denied(
                format!(
                    "launch --env {} is not available to MCP clients: variable names must be ASCII, because Windows folds the case of other letters",
                    env_key(entry)
                ),
                "Use an ASCII variable name.",
            ));
        }
        if let Some(entry) = args
            .env
            .iter()
            .find(|entry| is_code_loading_env_key(env_key(entry)))
        {
            return Err(denied(
                format!(
                    "launch --env {} is not available to MCP clients: it decides which program, DLL or module the new process finds or loads",
                    env_key(entry)
                ),
                "Omit that variable; the app starts with the server's own search path.",
            ));
        }
        if args
            .args
            .iter()
            .chain(args.env.iter())
            .any(|text| mentions_code_loading(text))
        {
            return Err(denied(
                "launch module-loading arguments are not available to MCP clients, even with --allow-cdp",
                "Omit require, import and loader arguments; cdp only permits debugging endpoints.",
            ));
        }
        if let Some(entry) = args.env.iter().find(|entry| {
            env_key(entry).eq_ignore_ascii_case("NODE_OPTIONS")
                && !node_options_hold_only_endpoints(
                    entry.split_once('=').map_or("", |(_, value)| value),
                )
        }) {
            return Err(denied(
                format!(
                    "launch --env {} is not available to MCP clients: NODE_OPTIONS may carry only inspector endpoint options, even with --allow-cdp",
                    env_key(entry)
                ),
                "Keep only --inspect, --inspect-brk, --inspect-port, --inspect-wait or --debug-port in NODE_OPTIONS.",
            ));
        }
        if self.cdp {
            return Ok(());
        }
        let texts = args
            .args
            .iter()
            .map(String::as_str)
            .chain(args.env.iter().map(String::as_str));
        let opens_endpoint = args.cdp.is_some()
            || args
                .env
                .iter()
                .any(|entry| is_inspector_env_key(env_key(entry)))
            || texts.into_iter().any(mentions_debug_endpoint);
        self.require(
            !opens_endpoint,
            "launch with a DevTools or inspector endpoint (cdp, remote-debugging, inspect, marionette, NODE_OPTIONS, ELECTRON_*)",
            "--allow-cdp",
        )
    }

    fn refuse_force(&self, force: bool, command: &str, reason: &str) -> Result<(), AppError> {
        if !force {
            return Ok(());
        }
        Err(denied(
            format!("{command} --force is not available to MCP clients: {reason}"),
            "Run it without force; a person can use the CLI if force is really intended.",
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

#[cfg(test)]
#[path = "restrictions_tests.rs"]
mod tests;
