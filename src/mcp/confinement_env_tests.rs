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
fn node_module_loading_arguments_are_refused_even_with_cdp() {
    for flag in ["--require", "--import", "--loader", "--experimental-loader"] {
        for spelling in [flag.to_owned(), flag.to_ascii_uppercase()] {
            for args in [
                vec![format!("{spelling}=module.js")],
                vec![spelling.clone(), "module.js".to_owned()],
                vec![format!(" \t{spelling} module.js \t")],
                vec![format!("/c node {spelling}=module.js")],
            ] {
                assert_launch_refused_with_and_without_cdp(args, vec![]);
            }
        }
    }
    let launch = command(&["launch", "app", "--arg=-r", "--arg=module.js"]);
    assert_eq!(
        denied_code(Restrictions::all_allowed().check(&launch)),
        ErrorCode::PolicyDenied
    );
    assert_launch_refused_with_and_without_cdp(vec!["-r".into(), "module.js".into()], vec![]);
}

#[test]
fn node_options_module_loading_is_refused_even_with_cdp() {
    for flag in ["--require", "--import", "--loader", "--experimental-loader"] {
        for spelling in [flag.to_owned(), flag.to_ascii_uppercase()] {
            for value in [
                format!("{spelling}=module.js"),
                format!("{spelling} module.js"),
                format!("  --inspect=9229 \t{spelling} = module.js  "),
                format!("\"{spelling}=module.js\""),
            ] {
                let entry = format!("  nOdE_oPtIoNs = {value}  ");
                let direct = command(&["launch", "app", &format!("--env={entry}")]);
                assert_eq!(
                    denied_code(Restrictions::all_allowed().check(&direct)),
                    ErrorCode::PolicyDenied,
                    "{entry}"
                );
                assert_launch_refused_with_and_without_cdp(vec![], vec![entry]);
            }
        }
    }
    assert_launch_refused_with_and_without_cdp(vec![], vec!["NODE_OPTIONS=-r module.js".into()]);
}

#[test]
fn electron_code_loading_and_target_variables_are_refused_even_with_cdp() {
    for key in [
        "ELECTRON_RUN_AS_NODE",
        "ELECTRON_NO_ASAR",
        "ELECTRON_OVERRIDE_DIST_PATH",
        "ELECTRON_TRASH",
    ] {
        for spelling in [key.to_owned(), key.to_ascii_lowercase()] {
            for value in ["1", "0", "", "C:\\module"] {
                let entry = format!("  {spelling} = {value} ");
                let direct = command(&["launch", "app", &format!("--env={entry}")]);
                assert_eq!(
                    denied_code(Restrictions::all_allowed().check(&direct)),
                    ErrorCode::PolicyDenied,
                    "{entry}"
                );
                assert_launch_refused_with_and_without_cdp(vec![], vec![entry]);
            }
        }
    }
}

#[test]
fn inspect_endpoints_still_require_cdp_but_do_not_count_as_module_loading() {
    for flag in [
        "--inspect",
        "--inspect-brk",
        "--inspect-port",
        "--inspect-wait",
    ] {
        for source in ["arg", "env"] {
            let option = if source == "arg" {
                format!("--arg=  {}=9229  ", flag.to_ascii_uppercase())
            } else {
                format!("--env= node_options =  {flag}=9229  ")
            };
            let launch = command(&["launch", "app", &option]);
            assert_eq!(
                denied_code(launch_only().check(&launch)),
                ErrorCode::PolicyDenied
            );
            assert!(Restrictions::all_allowed().check(&launch).is_ok());
        }
    }
    for value in [
        "--max-old-space-size=128",
        "--requirement=note",
        "--import-map=note",
    ] {
        let launch = command(&["launch", "app", &format!("--env=NODE_OPTIONS={value}")]);
        assert_eq!(
            denied_code(Restrictions::all_allowed().check(&launch)),
            ErrorCode::PolicyDenied,
            "{value}"
        );
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
        "OPENSSL_CONF=C:\review.cnf",
        "openssl_engines=C:\review",
        "OPENSSL_MODULES=C:\review",
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

#[test]
fn chromium_single_dash_switches_are_not_module_loading_arguments() {
    for value in [
        "-remote-debugging-port=9222",
        "-remote-debugging-pipe",
        "-remote-allow-origins=*",
        "-renderer-process-limit=2",
        "/c chrome -remote-debugging-port=9222 -remote-allow-origins=*",
    ] {
        assert!(!super::mentions_code_loading(value), "{value}");
        let launch = command(&["launch", "app", &format!("--arg={value}")]);
        assert!(
            Restrictions::all_allowed().check(&launch).is_ok(),
            "{value}"
        );
        let batch = crate::batch::parse_command(
            serde_json::from_value(json!({
                "command": "launch", "args": { "app": "app", "args": [value] }
            }))
            .expect("a batch entry"),
        )
        .expect("the entry decodes");
        assert!(Restrictions::all_allowed().check(&batch).is_ok(), "{value}");
        assert_launch_refused_with_and_without_cdp(
            vec![value.into(), "-r".into(), "module.js".into()],
            vec![],
        );
    }
}

#[test]
fn node_rejected_attached_short_require_values_are_not_module_loading() {
    for value in ["-rmodule.js", "-r=module.js"] {
        assert!(!super::mentions_code_loading(value), "{value}");
        let launch = command(&["launch", "app", &format!("--arg={value}")]);
        assert!(
            Restrictions::all_allowed().check(&launch).is_ok(),
            "{value}"
        );
        let entry = format!("NODE_OPTIONS={value}");
        let launch = command(&["launch", "app", &format!("--env={entry}")]);
        assert_eq!(
            denied_code(Restrictions::all_allowed().check(&launch)),
            ErrorCode::PolicyDenied,
            "{entry}"
        );
    }
}

#[test]
fn electron_install_target_variables_are_not_runtime_code_loading() {
    for key in ["ELECTRON_INSTALL_PLATFORM", "ELECTRON_INSTALL_ARCH"] {
        for spelling in [key.to_owned(), key.to_ascii_lowercase()] {
            assert!(!super::is_code_loading_env_key(&spelling), "{spelling}");
            let entry = format!("  {spelling} = arm64 ");
            let launch = command(&["launch", "app", &format!("--env={entry}")]);
            assert!(
                Restrictions::all_allowed().check(&launch).is_ok(),
                "{entry}"
            );
            assert_eq!(
                denied_code(launch_only().check(&launch)),
                ErrorCode::PolicyDenied
            );
        }
    }
}

#[test]
fn underscore_spellings_of_loader_options_are_refused_like_dashes() {
    for value in [
        "--experimental_loader=./m.js",
        "--experimental_loader ./m.js",
    ] {
        assert!(super::mentions_code_loading(value), "{value}");
        assert_launch_refused_with_and_without_cdp(vec![value.to_owned()], vec![]);
        assert_launch_refused_with_and_without_cdp(vec![], vec![format!("NODE_OPTIONS={value}")]);
    }
}

#[test]
fn quotes_and_escapes_spliced_into_node_options_are_refused() {
    for value in [
        r#"--re""quire=./m.js"#,
        r#""--req\uire" ./m.js"#,
        r#"-""r ./m.js"#,
    ] {
        assert!(super::mentions_code_loading(value), "{value}");
        assert_launch_refused_with_and_without_cdp(vec![], vec![format!("NODE_OPTIONS={value}")]);
    }
}

#[test]
fn uppercase_short_r_is_not_module_loading_because_node_options_are_case_sensitive() {
    for value in ["-R", "LESS=-R", "grep -R pattern ."] {
        assert!(!super::mentions_code_loading(value), "{value}");
    }
    let plain = command(&["launch", "app", "--arg=-R", "--env=LESS=-R"]);
    assert!(Restrictions::all_allowed().check(&plain).is_ok());
}

#[test]
fn openssl_config_is_refused_because_it_loads_engines() {
    for value in ["--openssl-config=./o.cnf", "--openssl_config ./o.cnf"] {
        assert!(super::mentions_code_loading(value), "{value}");
        assert_launch_refused_with_and_without_cdp(vec![], vec![format!("NODE_OPTIONS={value}")]);
    }
}

#[test]
fn underscore_spellings_of_debug_options_need_the_cdp_opt_in() {
    for value in ["--debug_port=9229", "--remote_debugging_port=9222"] {
        assert!(super::mentions_debug_endpoint(value), "{value}");
        assert!(
            super::mentions_debug_endpoint(&format!("NODE_OPTIONS={value}")),
            "{value}"
        );
    }
    for entry in ["APP_DEBUG_PORT=1", "MY_REMOTE_DEBUGGING=0"] {
        assert!(!super::mentions_debug_endpoint(entry), "{entry}");
    }
}
