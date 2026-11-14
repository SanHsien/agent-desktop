//! Tests for the WebView2 and Java option variables: under `--allow-cdp` they may carry only debugging endpoints.

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

fn with_cdp() -> Restrictions {
    Restrictions {
        cdp: true,
        ..launch_only()
    }
}

#[test]
fn webview2_arguments_may_hold_only_remote_debugging_switches() {
    for value in [
        "--renderer-cmd-prefix=calc",
        "--gpu-launcher=calc",
        r"--browser-subprocess-path=C:\x\evil.exe",
        "--remote-debugging-port=9222 --utility-cmd-prefix=calc",
        r#""--renderer-cmd-prefix"=calc"#,
        "--disable-gpu",
        "https://example.invalid",
    ] {
        assert_launch_refused_with_and_without_cdp(
            vec![],
            vec![format!("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS={value}")],
        );
    }
    for value in [
        "--remote-debugging-port=9222",
        " --remote-debugging-port=0  --remote-allow-origins=* ",
        "--REMOTE-DEBUGGING-PORT=9222",
    ] {
        let entry = format!("--env=webview2_additional_browser_arguments={value}");
        let launch = command(&["launch", "app", &entry]);
        assert!(with_cdp().check(&launch).is_ok(), "{value}");
        assert_eq!(
            denied_code(launch_only().check(&launch)),
            ErrorCode::PolicyDenied,
            "{value}"
        );
    }
}

#[test]
fn java_option_variables_may_hold_only_a_jdwp_agent_that_launches_nothing() {
    for key in ["JAVA_TOOL_OPTIONS", "_java_options", "Jdk_Java_Options"] {
        for value in [
            r"-agentpath:C:\x\evil.dll",
            "-agentlib:evil",
            r"-javaagent:C:\x\evil.jar",
            "-XX:OnOutOfMemoryError=calc",
            "-Xmx1g",
            "@argfile.txt",
            "-agentlib:jdwp=transport=dt_socket,server=y,address=5005 -javaagent:x.jar",
            "-agentlib:jdwp=transport=dt_socket,server=y,onthrow=java.io.IOException,launch=calc",
            "-Xrunjdwp:transport=dt_socket,server=y",
        ] {
            assert_launch_refused_with_and_without_cdp(vec![], vec![format!("{key}={value}")]);
        }
        let entry = format!("--env={key}=-agentlib:jdwp=transport=dt_socket,server=y,address=5005");
        let launch = command(&["launch", "app", &entry]);
        assert!(with_cdp().check(&launch).is_ok(), "{key}");
        assert_eq!(
            denied_code(launch_only().check(&launch)),
            ErrorCode::PolicyDenied,
            "{key}"
        );
    }
}

#[test]
fn jdwp_suboptions_that_load_libraries_or_write_files_are_refused() {
    for value in [
        r"-agentlib:jdwp=transport=C:\Users\x\evil,server=y,address=5005",
        r"-agentlib:jdwp=transport=\host\share\evil,server=y",
        "-agentlib:jdwp=transport=evil,server=y",
        r"-agentlib:jdwp=transport=dt_socket,server=y,logfile=C:\x\log.txt",
        "-agentlib:jdwp=transport=dt_socket,server=y,onjcmd=y",
        "-agentlib:jdwp=transport=dt_socket,server=y,unknownoption=1",
        "-agentlib:jdwp=",
    ] {
        assert_launch_refused_with_and_without_cdp(
            vec![],
            vec![format!("JAVA_TOOL_OPTIONS={value}")],
        );
    }
    for value in [
        "-agentlib:jdwp=transport=dt_shmem,server=y,suspend=n,address=jdbconn",
        "-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=*:5005,timeout=1000,quiet=y",
    ] {
        let launch = command(&["launch", "app", &format!("--env=JAVA_TOOL_OPTIONS={value}")]);
        assert!(with_cdp().check(&launch).is_ok(), "{value}");
    }
}

#[test]
fn openj9_option_variables_follow_the_java_allow_list() {
    for key in ["IBM_JAVA_OPTIONS", "openj9_java_options"] {
        assert_launch_refused_with_and_without_cdp(
            vec![],
            vec![format!(r"{key}=-agentpath:C:\x\evil.dll")],
        );
        let entry = format!("--env={key}=-agentlib:jdwp=transport=dt_socket,server=y");
        let launch = command(&["launch", "app", &entry]);
        assert!(with_cdp().check(&launch).is_ok(), "{key}");
        assert_eq!(
            denied_code(launch_only().check(&launch)),
            ErrorCode::PolicyDenied,
            "{key}"
        );
    }
}

#[test]
fn runtime_location_variables_are_refused_even_with_every_opt_in() {
    for entry in [
        r"WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=C:\x\fake-runtime",
        r"java_home=C:\x\fake-jdk",
    ] {
        assert_launch_refused_with_and_without_cdp(vec![], vec![entry.to_owned()]);
    }
}

#[test]
fn java_agent_options_in_any_argument_or_variable_are_refused() {
    for value in [
        r"-javaagent:C:\x\evil.jar",
        r"-AGENTPATH:C:\x\evil.dll",
        r"JAVA_OPTS=-javaagent:C:\x\evil.jar",
    ] {
        assert!(super::mentions_code_loading(value), "{value}");
    }
    assert_launch_refused_with_and_without_cdp(vec![r"-javaagent:C:\x\evil.jar".into()], vec![]);
    assert_launch_refused_with_and_without_cdp(
        vec![],
        vec![r"MAVEN_OPTS=-agentpath:C:\x\evil.dll".into()],
    );
}
