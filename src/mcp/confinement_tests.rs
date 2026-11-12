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
        "node_options=--max-old-space-size=1",
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222",
        "Webview2_Additional_Browser_Arguments=--x",
        "ELECTRON_RUN_AS_NODE=1",
        "electron_enable_logging=1",
        "JAVA_TOOL_OPTIONS=-agentlib:jdwp=transport=dt_socket,server=y,address=5005",
        "_java_options=-Xmx1g",
        "Jdk_Java_Options=-Xmx1g",
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

#[cfg(windows)]
fn make_junction(link: &Path, target: &Path) {
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("cmd runs");
    assert!(made.status.success(), "mklink /J failed");
}

/// Removes only the junction itself: `rmdir` without `/S` never descends into the target.
#[cfg(windows)]
fn remove_junction(link: &Path) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "rmdir"])
        .arg(link)
        .status();
}

#[cfg(windows)]
#[test]
fn a_dangling_junction_under_the_output_directory_is_refused() {
    let base = std::env::temp_dir().join(format!("agent-desktop-dangling-{}", std::process::id()));
    let output = base.join("mcp-output");
    let gone = base.join("outside").join("gone");
    std::fs::create_dir_all(&output).expect("output dir");
    std::fs::create_dir_all(&gone).expect("target dir");
    let link = output.join("link");
    make_junction(&link, &gone);
    std::fs::remove_dir(&gone).expect("the target goes, leaving the junction dangling");
    let checks = [
        is_inside(&link.join("stolen.png"), &output),
        is_inside(&link.join("nested").join("stolen.png"), &output),
        is_inside(&link, &output),
    ];
    remove_junction(&link);
    let _ = std::fs::remove_dir_all(&base);
    assert_eq!(
        checks,
        [false, false, false],
        "a link whose target does not exist must not be skipped as if it were absent"
    );
}

#[cfg(windows)]
#[test]
fn a_junction_named_as_the_output_file_is_refused_whether_or_not_it_resolves() {
    let base =
        std::env::temp_dir().join(format!("agent-desktop-final-link-{}", std::process::id()));
    let output = base.join("mcp-output");
    let live = base.join("outside").join("live");
    let gone = base.join("outside").join("gone");
    std::fs::create_dir_all(&output).expect("output dir");
    std::fs::create_dir_all(&live).expect("live target");
    std::fs::create_dir_all(&gone).expect("gone target");
    let live_link = output.join("live.png");
    let gone_link = output.join("gone.png");
    make_junction(&live_link, &live);
    make_junction(&gone_link, &gone);
    std::fs::remove_dir(&gone).expect("the second target goes");
    let checks = [
        is_inside(&live_link, &output),
        is_inside(&gone_link, &output),
    ];
    remove_junction(&live_link);
    remove_junction(&gone_link);
    let _ = std::fs::remove_dir_all(&base);
    assert_eq!(checks, [false, false]);
}
