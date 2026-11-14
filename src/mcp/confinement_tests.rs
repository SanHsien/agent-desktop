use super::*;
use crate::cli::{Cli, Commands};
use crate::mcp::restrictions::Restrictions;
use agent_desktop_core::{AppError, ErrorCode};
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

fn command(args: &[&str]) -> Commands {
    Cli::try_parse_from(std::iter::once("agent-desktop").chain(args.iter().copied()))
        .expect("the arguments parse")
        .command
        .expect("a command")
}

fn launch_only() -> Restrictions {
    Restrictions {
        launch: true,
        ..Restrictions::default()
    }
}

fn denied_code(result: Result<(), AppError>) -> ErrorCode {
    match result.expect_err("the command must be refused") {
        AppError::Adapter(error) => error.code,
        other => panic!("unexpected error kind: {other}"),
    }
}

fn existing_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!(
            "agent-desktop-confinement-{tag}-{}",
            std::process::id()
        ))
        .join("mcp-output");
    std::fs::create_dir_all(&root).expect("root");
    root
}

#[test]
fn inspector_environment_variables_need_the_cdp_opt_in() {
    for entry in [
        "NODE_OPTIONS=--inspect=9229",
        "node_options=--inspect-port=0",
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222",
        "Webview2_Additional_Browser_Arguments=--remote-allow-origins=*",
        "electron_enable_logging=1",
        "JAVA_TOOL_OPTIONS=-agentlib:jdwp=transport=dt_socket,server=y,address=5005",
        "_java_options=-agentlib:jdwp=transport=dt_socket,server=y",
        "Jdk_Java_Options=-agentlib:jdwp=transport=dt_socket,address=5005",
    ] {
        let env = format!("--env={entry}");
        let launch = command(&["launch", "app", &env]);
        assert_eq!(
            denied_code(launch_only().check(&launch)),
            ErrorCode::PolicyDenied,
            "{entry}"
        );
        let with_cdp = Restrictions {
            cdp: true,
            ..launch_only()
        };
        assert!(with_cdp.check(&launch).is_ok(), "{entry}");
    }
    let plain = command(&["launch", "app", "--env=LANG=en_US", "--env=APP_MODE=fast"]);
    assert!(launch_only().check(&plain).is_ok());
}

#[test]
fn a_debug_switch_hidden_inside_a_longer_argument_or_an_env_value_is_found() {
    for args in [
        vec![
            "launch",
            "cmd",
            "--arg=/c start msedge --remote-debugging-port=9222",
        ],
        vec![
            "launch",
            "cmd",
            "--arg=/C START MSEDGE --REMOTE-DEBUGGING-PIPE",
        ],
        vec![
            "launch",
            "powershell",
            "--arg=-Command & { node --Inspect-Brk app.js }",
        ],
        vec!["launch", "firefox", "--arg=-marionette"],
        vec![
            "launch",
            "java",
            "--arg=-agentlib:jdwp=transport=dt_socket,server=y,address=5005",
        ],
        vec![
            "launch",
            "java",
            "--arg=-Xrunjdwp:transport=dt_socket,address=5005",
        ],
        vec!["launch", "app", "--arg=--debug-port=9229"],
        vec!["launch", "app", "--arg=--DEBUG-BRK"],
        vec!["launch", "app", "--env=OPTS=--remote-allow-origins=*"],
    ] {
        let launch = command(&args);
        assert_eq!(
            denied_code(launch_only().check(&launch)),
            ErrorCode::PolicyDenied,
            "{args:?}"
        );
        let with_cdp = Restrictions {
            cdp: true,
            ..launch_only()
        };
        assert!(with_cdp.check(&launch).is_ok(), "{args:?}");
    }
}

#[test]
fn program_search_variables_are_refused_even_with_every_opt_in() {
    let everything = Restrictions::all_allowed();
    for entry in [
        "PATH=C:\\evil",
        "path=C:\\evil",
        "PathExt=.EXE",
        "COMSPEC=C:\\evil.exe",
        "PSModulePath=C:\\evil",
        "pythonpath=C:\\evil",
        "PYTHONSTARTUP=C:\\evil.py",
        "Node_Path=C:\\evil",
        "CLASSPATH=C:\\evil",
        "DOTNET_STARTUP_HOOKS=C:\\evil.dll",
        "dotnet_additional_deps=C:\\evil",
        "PERL5LIB=C:\\evil",
        "RubyLib=C:\\evil",
    ] {
        let env = format!("--env={entry}");
        let launch = command(&["launch", "app", &env]);
        assert_eq!(
            denied_code(everything.check(&launch)),
            ErrorCode::PolicyDenied,
            "{entry}"
        );
    }
    let batch = crate::batch::parse_command(
        serde_json::from_value(
            json!({ "command": "launch", "args": { "app": "app", "env": ["PATH=C:\\evil"] } }),
        )
        .expect("a batch entry"),
    )
    .expect("the entry decodes");
    assert!(everything.check(&batch).is_err());
}

#[test]
fn a_variable_name_with_a_non_ascii_letter_is_refused_even_with_every_opt_in() {
    let everything = Restrictions::all_allowed();
    for entry in [
        "NODE_OPT\u{131}ONS=--inspect=9229",
        "P\u{c0}TH=C:\\evil",
        "caf\u{e9}=1",
    ] {
        let env = format!("--env={entry}");
        let launch = command(&["launch", "app", &env]);
        assert_eq!(
            denied_code(everything.check(&launch)),
            ErrorCode::PolicyDenied,
            "{entry}"
        );
    }
    let ascii = command(&["launch", "app", "--env=CAFE=caf\u{e9}"]);
    assert!(
        everything.check(&ascii).is_ok(),
        "a non-ASCII value is fine"
    );
}

#[test]
fn close_app_force_is_refused_and_a_graceful_close_is_not() {
    let everything = Restrictions::all_allowed();
    let forced = command(&["close-app", "Notepad", "--force"]);
    assert_eq!(
        denied_code(everything.check(&forced)),
        ErrorCode::PolicyDenied
    );
    assert!(
        everything
            .check(&command(&["close-app", "Notepad"]))
            .is_ok()
    );
    let batch = crate::batch::parse_command(
        serde_json::from_value(
            json!({ "command": "close-app", "args": { "app": "Notepad", "force": true } }),
        )
        .expect("a batch entry"),
    )
    .expect("the entry decodes");
    assert!(everything.check(&batch).is_err());
}

#[test]
fn reserved_device_names_and_stream_tricks_are_not_files_in_the_output_directory() {
    let root = existing_root("names");
    for name in [
        "COM1",
        "com9",
        "COM\u{b9}",
        "LPT\u{b2}.png",
        "LPT0",
        "COM10",
        "com123.txt",
        "LPT12.png",
        "COM\u{b9}0",
        "COM\u{b2}\u{b3}",
        "NUL.png",
        "nul",
        "CON",
        "con.txt",
        "PRN.log",
        "AUX",
        "aux .png",
        "CONIN$",
        "shot.png.",
        "shot.png ",
        "shot.png:stream",
        "wild*.png",
        "a<b.png",
    ] {
        assert!(!is_inside(&root.join(name), &root), "{name}");
    }
    assert!(!is_inside(&root.join("sub").join("COM1"), &root));
    for name in [
        "shot.png",
        "console.png",
        "com.png",
        "comet.png",
        "lptx",
        "nul2",
        "auxiliary.txt",
        "a.b.c",
    ] {
        assert!(is_inside(&root.join(name), &root), "{name}");
    }
}

#[test]
fn an_output_directory_that_does_not_exist_admits_nothing() {
    let missing = std::env::temp_dir()
        .join(format!(
            "agent-desktop-confinement-missing-{}",
            std::process::id()
        ))
        .join("mcp-output");
    assert!(!missing.exists());
    assert!(!is_inside(&missing.join("shot.png"), &missing));
    let limits = Restrictions {
        file_write: true,
        file_root: Some(missing.clone()),
        ..Restrictions::default()
    };
    let screenshot = command(&["screenshot", &missing.join("shot.png").to_string_lossy()]);
    assert!(limits.check(&screenshot).is_err());
}

#[test]
fn an_unresolvable_target_is_refused_not_waved_through() {
    let root = existing_root("unresolvable");
    let limits = Restrictions {
        file_write: true,
        file_root: Some(root.clone()),
        ..Restrictions::default()
    };
    for name in ["NUL.png", "CON", "COM1"] {
        let path = root.join(name).to_string_lossy().into_owned();
        assert!(
            limits.check(&command(&["screenshot", &path])).is_err(),
            "{name}"
        );
    }
    let fine = root.join("shot.png").to_string_lossy().into_owned();
    assert!(limits.check(&command(&["screenshot", &fine])).is_ok());
}

/// Owns a scratch tree for the junction tests. Its drop removes every junction it recorded first (`rmdir`, which never
/// descends into a target), then the tree, so a failed assertion leaves nothing behind.
#[cfg(windows)]
struct ScratchTree {
    base: PathBuf,
    links: Vec<PathBuf>,
}

#[cfg(windows)]
impl ScratchTree {
    fn new(tag: &str) -> Self {
        let base = std::env::temp_dir().join(format!("agent-desktop-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("scratch base");
        Self {
            base,
            links: Vec::new(),
        }
    }

    fn dir(&self, relative: &str) -> PathBuf {
        let path = relative
            .split('/')
            .fold(self.base.clone(), |path, part| path.join(part));
        std::fs::create_dir_all(&path).expect("scratch dir");
        path
    }

    /// Creates a junction at `link` pointing at `target` and records it for removal.
    fn junction(&mut self, link: &Path, target: &Path) {
        self.links.push(link.to_path_buf());
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .expect("cmd runs");
        assert!(made.status.success(), "mklink /J failed");
    }
}

#[cfg(windows)]
impl Drop for ScratchTree {
    fn drop(&mut self) {
        for link in &self.links {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "rmdir"])
                .arg(link)
                .status();
        }
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

#[cfg(windows)]
#[test]
fn a_dangling_junction_under_the_output_directory_is_refused() {
    let mut tree = ScratchTree::new("dangling");
    let output = tree.dir("mcp-output");
    let gone = tree.dir("outside/gone");
    let link = output.join("link");
    tree.junction(&link, &gone);
    std::fs::remove_dir(&gone).expect("the target goes, leaving the junction dangling");
    assert!(!is_inside(&link.join("stolen.png"), &output));
    assert!(!is_inside(&link.join("nested").join("stolen.png"), &output));
    assert!(
        !is_inside(&link, &output),
        "a dangling link must not be skipped as if it were absent"
    );
}

#[cfg(windows)]
#[test]
fn a_junction_named_as_the_output_file_is_refused_whether_or_not_it_resolves() {
    let mut tree = ScratchTree::new("final-link");
    let output = tree.dir("mcp-output");
    let live = tree.dir("outside/live");
    let gone = tree.dir("outside/gone");
    let live_link = output.join("live.png");
    let gone_link = output.join("gone.png");
    tree.junction(&live_link, &live);
    tree.junction(&gone_link, &gone);
    std::fs::remove_dir(&gone).expect("the second target goes");
    assert!(!is_inside(&live_link, &output), "a link to a live target");
    assert!(
        !is_inside(&gone_link, &output),
        "a link to a missing target"
    );
}

#[cfg(windows)]
#[test]
fn a_live_junction_back_into_the_output_directory_is_refused_too() {
    let mut tree = ScratchTree::new("alias");
    let output = tree.dir("mcp-output");
    let inner = tree.dir("mcp-output/inner");
    let alias = output.join("alias");
    tree.junction(&alias, &inner);
    assert!(
        !is_inside(&alias.join("x.png"), &output),
        "a link that stays inside the root is still a link"
    );
    assert!(!is_inside(&alias, &output));
    assert!(
        is_inside(&inner.join("x.png"), &output),
        "the real folder is fine"
    );
}
