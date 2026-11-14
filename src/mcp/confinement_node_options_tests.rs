//! Tests for the NODE_OPTIONS allow list and the Node options that run code in place of the main script.

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

fn assert_launch_refused_with_and_without_cdp(args: Vec<String>, env: Vec<String>) {
    let launch = crate::batch::parse_command(
        serde_json::from_value(json!({
            "command": "launch", "args": { "app": "app", "args": args, "env": env }
        }))
        .expect("a batch entry"),
    )
    .expect("the entry decodes");
    for limits in [launch_only(), Restrictions::all_allowed()] {
        assert_eq!(denied_code(limits.check(&launch)), ErrorCode::PolicyDenied);
    }
}

#[test]
fn node_options_may_hold_only_inspector_endpoint_options() {
    for value in [
        "--snapshot-blob ./snap.blob",
        "--snapshot_blob=./snap.blob",
        r#""--snapshot-blob" "./snap.blob" --inspect-port=0"#,
        "--max-old-space-size=4096",
        "--inspect=9229 --title=x",
    ] {
        assert_launch_refused_with_and_without_cdp(vec![], vec![format!("NODE_OPTIONS={value}")]);
    }
    let with_cdp = Restrictions {
        cdp: true,
        ..launch_only()
    };
    for value in [
        "--inspect=9229",
        "--inspect-brk=127.0.0.1:9229",
        " --inspect_port=0 --inspect-wait ",
        "--debug-port 9229",
    ] {
        let launch = command(&["launch", "app", &format!("--env=node_options={value}")]);
        assert!(with_cdp.check(&launch).is_ok(), "{value}");
    }
}

#[test]
fn snapshot_blob_in_arguments_is_refused_because_it_runs_its_own_main() {
    for value in ["--snapshot-blob=./snap.blob", "--SNAPSHOT_BLOB ./snap.blob"] {
        assert!(super::mentions_code_loading(value), "{value}");
        assert_launch_refused_with_and_without_cdp(vec![value.to_owned()], vec![]);
    }
}
