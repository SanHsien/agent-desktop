use super::*;
use crate::cli::Cli;
use clap::Parser;
use serde_json::json;

fn command(args: &[&str]) -> Commands {
    Cli::try_parse_from(std::iter::once("agent-desktop").chain(args.iter().copied()))
        .expect("the arguments parse")
        .command
        .expect("a command")
}

fn batch_entry(entry: serde_json::Value) -> Commands {
    crate::batch::parse_command(serde_json::from_value(entry).expect("a batch entry"))
        .expect("the entry decodes")
}

fn root() -> PathBuf {
    std::env::temp_dir()
        .join(format!("agent-desktop-restrictions-{}", std::process::id()))
        .join("mcp-output")
}

fn with_root(limits: Restrictions) -> Restrictions {
    Restrictions {
        file_root: Some(root()),
        ..limits
    }
}

fn denied_code(result: Result<(), AppError>) -> ErrorCode {
    match result.expect_err("the command must be refused") {
        AppError::Adapter(error) => error.code,
        other => panic!("unexpected error kind: {other}"),
    }
}

fn path_in_root(name: &str) -> String {
    root().join(name).to_string_lossy().into_owned()
}

#[test]
fn force_is_refused_for_every_command_that_has_it_even_with_every_opt_in() {
    let everything = with_root(Restrictions::all_allowed());
    for args in [
        ["press", "win+r", "--force"],
        ["key-down", "win", "--force"],
        ["key-up", "win", "--force"],
    ] {
        let code = denied_code(everything.check(&command(&args)));
        assert_eq!(code, ErrorCode::PolicyDenied, "{args:?}");
    }
}

#[test]
fn force_inside_a_batch_entry_is_refused_like_a_direct_call() {
    let everything = with_root(Restrictions::all_allowed());
    for entry in [
        json!({ "command": "press", "args": { "combo": "win+r", "force": true } }),
        json!({ "command": "key-down", "args": { "combo": "win", "force": true } }),
    ] {
        let code = denied_code(everything.check(&batch_entry(entry)));
        assert_eq!(code, ErrorCode::PolicyDenied);
    }
}

#[test]
fn a_combo_without_force_is_left_to_the_adapter_blocked_list() {
    let none = Restrictions::default();
    assert!(none.check(&command(&["press", "win+r"])).is_ok());
    assert!(none.check(&command(&["key-down", "shift"])).is_ok());
}

#[test]
fn launch_batch_and_session_gc_are_off_by_default_and_each_opt_in_turns_on_only_its_own() {
    let none = Restrictions::default();
    let launch = command(&["launch", "notepad"]);
    let batch = command(&["batch", "[]"]);
    let gc = command(&["session", "gc"]);
    for refused in [&launch, &batch, &gc] {
        assert_eq!(
            denied_code(none.check(refused)),
            ErrorCode::PolicyDenied,
            "{}",
            refused.name()
        );
    }
    let only_launch = Restrictions {
        launch: true,
        ..Restrictions::default()
    };
    assert!(only_launch.check(&launch).is_ok());
    assert!(only_launch.check(&batch).is_err());
    assert!(only_launch.check(&gc).is_err());
    let only_batch = Restrictions {
        batch: true,
        ..Restrictions::default()
    };
    assert!(only_batch.check(&batch).is_ok());
    assert!(only_batch.check(&launch).is_err());
    let only_files = with_root(Restrictions {
        file_write: true,
        ..Restrictions::default()
    });
    assert!(only_files.check(&gc).is_ok());
    assert!(only_files.check(&launch).is_err());
}

#[test]
fn a_batch_entry_cannot_launch_unless_launch_is_allowed() {
    let batch_only = Restrictions {
        batch: true,
        ..Restrictions::default()
    };
    let entry = json!({ "command": "launch", "args": { "app": "notepad" } });
    assert_eq!(
        denied_code(batch_only.check(&batch_entry(entry))),
        ErrorCode::PolicyDenied
    );
}

#[test]
fn launch_cwd_is_refused_even_when_launch_is_allowed() {
    let everything = with_root(Restrictions::all_allowed());
    let direct = command(&["launch", "notepad", "--cwd", r"C:\Users\Public"]);
    assert_eq!(
        denied_code(everything.check(&direct)),
        ErrorCode::PolicyDenied
    );
    let entry =
        json!({ "command": "launch", "args": { "app": "notepad", "cwd": "C:\\Users\\Public" } });
    assert!(everything.check(&batch_entry(entry)).is_err());
}

#[test]
fn devtools_and_inspector_switches_need_the_cdp_opt_in() {
    let launch_only = Restrictions {
        launch: true,
        ..Restrictions::default()
    };
    for args in [
        vec!["launch", "app", "--arg=--remote-debugging-port=9222"],
        vec!["launch", "app", "--arg=--Remote-Debugging-Pipe"],
        vec!["launch", "app", "--arg=-remote-debugging-port=9222"],
        vec!["launch", "app", "--arg=/remote-debugging-port=9222"],
        vec!["launch", "app", "--arg=--remote-allow-origins=*"],
        vec!["launch", "app", "--arg=--inspect-brk"],
        vec![
            "launch",
            "app",
            "--arg=--ok",
            "--arg=--inspect=0.0.0.0:9229",
        ],
        vec!["launch", "app", "--cdp"],
        vec!["launch", "app", "--cdp", "9222"],
    ] {
        let launch = command(&args);
        assert_eq!(
            denied_code(launch_only.check(&launch)),
            ErrorCode::PolicyDenied,
            "{args:?}"
        );
        let with_cdp = Restrictions {
            cdp: true,
            ..launch_only.clone()
        };
        assert!(with_cdp.check(&launch).is_ok(), "{args:?}");
    }
    let plain = command(&["launch", "app", "--arg=--profile", "--arg=work"]);
    assert!(launch_only.check(&plain).is_ok());
}

#[test]
fn file_paths_need_the_opt_in_and_stay_inside_the_output_directory() {
    let inside = path_in_root("shot.png");
    let outside = std::env::temp_dir()
        .join("elsewhere.png")
        .to_string_lossy()
        .into_owned();
    let off = with_root(Restrictions::default());
    let on = with_root(Restrictions {
        file_write: true,
        ..Restrictions::default()
    });
    let invocations: [fn(&str) -> Vec<String>; 6] = [
        |path| vec!["screenshot".into(), path.into()],
        |path| vec!["trace".into(), "export".into(), format!("--out={path}")],
        |path| vec!["clipboard-get".into(), format!("--out={path}")],
        |path| vec!["clipboard-set".into(), format!("--image={path}")],
        |path| vec!["clipboard-set".into(), format!("--file-url={path}")],
        |path| {
            vec![
                "cursor-overlay".into(),
                "enable".into(),
                format!("--image={path}"),
            ]
        },
    ];
    for invocation in invocations {
        let at = |path: &str| {
            let args = invocation(path);
            command(&args.iter().map(String::as_str).collect::<Vec<_>>())
        };
        assert!(
            off.check(&at(&inside)).is_err(),
            "{:?}",
            invocation(&inside)
        );
        assert!(on.check(&at(&inside)).is_ok(), "{:?}", invocation(&inside));
        assert!(
            on.check(&at(&outside)).is_err(),
            "{:?}",
            invocation(&outside)
        );
    }
}

#[test]
fn paths_that_look_inside_but_are_not_are_refused() {
    let on = with_root(Restrictions {
        file_write: true,
        ..Restrictions::default()
    });
    let escaping = root()
        .join("..")
        .join("escape.png")
        .to_string_lossy()
        .into_owned();
    let sibling = format!("{}-sibling\\x.png", root().display());
    for path in [
        escaping.as_str(),
        sibling.as_str(),
        "relative.png",
        r"\\server\share\x.png",
        r"\\?\UNC\server\share\x.png",
        r"\rooted.png",
    ] {
        assert!(on.check(&command(&["screenshot", path])).is_err(), "{path}");
    }
    assert!(
        on.check(&command(&["screenshot", &root().to_string_lossy()]))
            .is_err(),
        "the output directory itself is not a file"
    );
}

#[test]
fn file_output_without_a_session_directory_is_refused_not_guessed() {
    let no_root = Restrictions {
        file_write: true,
        ..Restrictions::default()
    };
    let error = no_root.check(&command(&["screenshot", &path_in_root("a.png")]));
    assert_eq!(denied_code(error), ErrorCode::PolicyDenied);
}

#[test]
fn trace_export_must_name_its_output() {
    let on = with_root(Restrictions {
        file_write: true,
        ..Restrictions::default()
    });
    assert!(on.check(&command(&["trace", "export"])).is_err());
    assert!(on.check(&command(&["trace", "show"])).is_ok());
}

#[test]
fn commands_without_files_or_dangerous_flags_pass_every_limit() {
    let none = Restrictions::default();
    for args in [
        vec!["snapshot", "--app", "Notepad"],
        vec!["click", "@s8f3k2p9:e1"],
        vec!["screenshot"],
        vec!["clipboard-set", "hello"],
        vec!["session", "list"],
        vec!["close-app", "Notepad"],
    ] {
        assert!(none.check(&command(&args)).is_ok(), "{args:?}");
    }
}

#[test]
fn the_limits_survive_the_trip_through_the_child_environment() {
    for limits in [
        Restrictions::default(),
        Restrictions {
            launch: true,
            cdp: true,
            ..Restrictions::default()
        },
        with_root(Restrictions::all_allowed()),
    ] {
        let env = limits.to_env();
        let value = |name: &str| {
            env.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        };
        let allowed = value(ALLOW_ENV).expect("the allow list is always set");
        let restored = Restrictions::parse(&allowed, value(FILE_ROOT_ENV).map(PathBuf::from));
        assert_eq!(restored, limits);
    }
}

#[cfg(windows)]
#[test]
fn a_junction_inside_the_output_directory_cannot_lead_out_of_it() {
    let base = std::env::temp_dir().join(format!("agent-desktop-junction-{}", std::process::id()));
    let output = base.join("mcp-output");
    let outside = base.join("outside");
    std::fs::create_dir_all(&output).expect("output dir");
    std::fs::create_dir_all(&outside).expect("outside dir");
    let link = output.join("link");
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .output()
        .expect("cmd runs");
    let escaped = is_inside(&link.join("stolen.png"), &output);
    let inside = is_inside(&output.join("fine.png"), &output);
    let _ = std::fs::remove_dir_all(&base);
    assert!(made.status.success(), "mklink /J failed");
    assert!(
        !escaped,
        "a junction must not be a way out of the output directory"
    );
    assert!(inside);
}
