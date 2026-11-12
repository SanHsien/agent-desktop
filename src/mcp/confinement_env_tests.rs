//! Tests for the launch environment and the debug switches that MCP clients may not reach without an opt-in.

use crate::cli::{Cli, Commands};
use crate::mcp::restrictions::Restrictions;
use agent_desktop_core::{AppError, ErrorCode};
use clap::Parser;
use serde_json::json;

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
#[test]
fn code_loading_variables_are_refused_even_with_every_opt_in() {
    let everything = Restrictions::all_allowed();
    for entry in [
        "COR_ENABLE_PROFILING=1",
        "cor_profiler={00000000-0000-0000-0000-000000000000}",
        "COR_PROFILER_PATH=C:\review.dll",
        "COR_PROFILER_PATH_32=C:\review.dll",
        "COR_PROFILER_PATH_64=C:\review.dll",
        "CORECLR_ENABLE_PROFILING=1",
        "CORECLR_PROFILER={00000000-0000-0000-0000-000000000000}",
        "CoreClr_Profiler_Path_64=C:\review.dll",
        "DOTNET_ENABLE_PROFILING=1",
        "DOTNET_PROFILER_PATH=C:\review.dll",
        "dotnet_profiler_path_32=C:\review.dll",
        "RUBYOPT=-rreview",
        "PERL5OPT=-MReview",
        "PYTHONHOME=C:\review",
        "pythonuserbase=C:\review",
        "NODE_REPL_EXTERNAL_MODULE=C:\review.js",
        "GIT_SSH_COMMAND=review.exe",
        "GIT_SSH=review.exe",
        "GIT_EXEC_PATH=C:\review",
        "GIT_ASKPASS=review.exe",
        "SSH_ASKPASS=review.exe",
        "GIT_EDITOR=review.exe",
        "GIT_PAGER=review.exe",
        "GIT_PROXY_COMMAND=review.exe",
        "GIT_EXTERNAL_DIFF=review.exe",
        "GIT_CONFIG_COUNT=1",
        "GIT_CONFIG_KEY_0=core.sshCommand",
        "GIT_CONFIG_VALUE_0=review.exe",
        "GIT_CONFIG_PARAMETERS='core.pager=review.exe'",
        "BASH_ENV=C:\review.sh",
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
            json!({ "command": "launch", "args": { "app": "app", "env": ["RUBYOPT=-rreview"] } }),
        )
        .expect("a batch entry"),
    )
    .expect("the entry decodes");
    assert!(everything.check(&batch).is_err());
}

#[test]
fn ordinary_variables_that_only_look_similar_are_still_allowed() {
    let everything = Restrictions::all_allowed();
    for entry in [
        "APP_PROFILE=fast",
        "MY_COR_PROFILER_NOTE=1",
        "DOTNET_TieredPGO=0",
        "DOTNET_gcServer=1",
        "CORECLR_SERVER_GC=1",
        "GIT_TRACE=1",
        "GIT_TERMINAL_PROMPT=0",
        "PYTHONOPTIMIZE=1",
    ] {
        let env = format!("--env={entry}");
        let launch = command(&["launch", "app", &env]);
        assert!(everything.check(&launch).is_ok(), "{entry}");
    }
}

#[test]
fn firefox_debugger_server_switch_needs_the_cdp_opt_in_in_every_spelling() {
    for args in [
        vec!["launch", "firefox", "--arg=--start-debugger-server=9222"],
        vec!["launch", "firefox", "--arg=-start-debugger-server=9222"],
        vec!["launch", "firefox", "--arg=/start-debugger-server=9222"],
        vec!["launch", "firefox", "--arg=--START-DEBUGGER-SERVER=9222"],
        vec!["launch", "firefox", "--arg=--Start-Debugger-Server"],
        vec![
            "launch",
            "firefox",
            "--arg",
            "--start-debugger-server",
            "--arg",
            "9222",
        ],
        vec![
            "launch",
            "firefox",
            "--arg",
            "-start-debugger-server",
            "--arg",
            "ws:9222",
        ],
        vec![
            "launch",
            "firefox",
            "--arg=--start-debugger-server=/tmp/review.sock",
        ],
        vec![
            "launch",
            "cmd",
            "--arg=/c start firefox --start-debugger-server 9222",
        ],
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
