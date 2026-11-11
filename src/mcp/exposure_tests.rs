use super::*;
use crate::mcp::catalog::{Tool, tools};
use serde_json::{Map, Value};
use std::path::PathBuf;

fn root() -> PathBuf {
    std::env::temp_dir().join("agent-desktop-exposure-root")
}

fn file_limits() -> Restrictions {
    Restrictions {
        file_write: true,
        file_root: Some(root()),
        ..Restrictions::default()
    }
}

fn tool_at(limits: &Restrictions, path: &[&str]) -> Option<Tool> {
    tools(limits).into_iter().find(|tool| tool.path == path)
}

fn properties(tool: &Tool) -> Vec<String> {
    tool.definition()["inputSchema"]["properties"]
        .as_object()
        .expect("properties")
        .keys()
        .cloned()
        .collect()
}

fn arguments(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

#[test]
fn launch_batch_and_session_gc_are_not_listed_until_their_opt_in() {
    for path in [["launch"].as_slice(), &["batch"], &["session", "gc"]] {
        assert!(
            tool_at(&Restrictions::default(), path).is_none(),
            "{path:?}"
        );
        assert!(
            tool_at(&Restrictions::all_allowed(), path).is_some(),
            "{path:?}"
        );
    }
    let launch = Restrictions {
        launch: true,
        ..Restrictions::default()
    };
    assert!(tool_at(&launch, &["launch"]).is_some());
    assert!(tool_at(&launch, &["batch"]).is_none());
    assert!(tool_at(&launch, &["press"]).is_some());
}

#[test]
fn force_never_reaches_the_schema_or_the_argv_of_any_key_or_close_tool() {
    for path in [["press"], ["key-down"], ["key-up"]] {
        let tool = tool_at(&Restrictions::all_allowed(), &path).expect("tool");
        assert!(
            !properties(&tool).contains(&"force".to_string()),
            "{path:?}"
        );
        let error = tool
            .argv(&arguments(&[
                ("combo", "win".into()),
                ("force", true.into()),
            ]))
            .expect_err("force is not an argument");
        assert!(error.contains("Unknown argument 'force'"), "{error}");
    }
    let close_app = tool_at(&Restrictions::all_allowed(), &["close-app"]).expect("close-app");
    assert!(!properties(&close_app).contains(&"force".to_string()));
    assert!(properties(&close_app).contains(&"app".to_string()));
}

#[test]
fn launch_never_offers_cwd_and_offers_cdp_only_with_its_opt_in() {
    let launch_only = Restrictions {
        launch: true,
        ..Restrictions::default()
    };
    let tool = tool_at(&launch_only, &["launch"]).expect("launch");
    let names = properties(&tool);
    assert!(!names.contains(&"cwd".to_string()));
    assert!(!names.contains(&"cdp".to_string()));
    assert!(names.contains(&"args".to_string()));
    assert!(tool.argv(&arguments(&[("cwd", "C:\\".into())])).is_err());
    let cdp = Restrictions {
        cdp: true,
        ..launch_only
    };
    let names = properties(&tool_at(&cdp, &["launch"]).expect("launch"));
    assert!(names.contains(&"cdp".to_string()));
    assert!(!names.contains(&"cwd".to_string()));
}

#[test]
fn file_arguments_are_hidden_without_the_file_opt_in() {
    let cases: [(&[&str], &str); 7] = [
        (&["screenshot"], "output_path"),
        (&["trace", "export"], "out"),
        (&["clipboard-get"], "out"),
        (&["clipboard-set"], "image"),
        (&["clipboard-set"], "file_url"),
        (&["cursor-overlay", "enable"], "image"),
        (&["cursor-overlay", "enable"], "pointer_image"),
    ];
    for (path, argument) in cases {
        let off = tool_at(&Restrictions::default(), path).expect("tool");
        assert!(
            !properties(&off).contains(&argument.to_string()),
            "{path:?}"
        );
        let on = tool_at(&file_limits(), path).expect("tool");
        assert!(properties(&on).contains(&argument.to_string()), "{path:?}");
    }
}

#[test]
fn relative_file_arguments_land_in_the_output_directory_and_absolute_ones_are_left_alone() {
    let tool = tool_at(&file_limits(), &["screenshot"]).expect("screenshot");
    let relative = tool
        .argv(&arguments(&[("output_path", "shot.png".into())]))
        .expect("argv");
    assert_eq!(
        relative,
        [
            "screenshot",
            "--",
            &root().join("shot.png").to_string_lossy()
        ]
    );
    let absolute = std::env::temp_dir().join("elsewhere.png");
    let kept = tool
        .argv(&arguments(&[(
            "output_path",
            absolute.to_string_lossy().as_ref().into(),
        )]))
        .expect("argv");
    assert_eq!(kept[2], absolute.to_string_lossy());
    let urls = tool_at(&file_limits(), &["clipboard-set"]).expect("clipboard-set");
    let argv = urls
        .argv(&arguments(&[(
            "file_url",
            Value::from(vec!["a.txt", "b.txt"]),
        )]))
        .expect("argv");
    assert_eq!(
        argv,
        [
            "clipboard-set".to_string(),
            format!("--file-url={}", root().join("a.txt").display()),
            format!("--file-url={}", root().join("b.txt").display()),
        ]
    );
}

#[test]
fn trace_export_gets_a_default_output_inside_the_directory_only_when_files_are_allowed() {
    let on = tool_at(&file_limits(), &["trace", "export"]).expect("trace export");
    let argv = on.argv(&Map::new()).expect("argv");
    assert_eq!(
        argv.last().map(String::as_str),
        Some(format!("--out={}", root().join(TRACE_EXPORT_FILE).to_string_lossy()).as_str())
    );
    let named = on
        .argv(&arguments(&[("out", "mine.html".into())]))
        .expect("argv");
    assert_eq!(
        named.iter().filter(|arg| arg.starts_with("--out=")).count(),
        1
    );
    let off = tool_at(&Restrictions::default(), &["trace", "export"]).expect("trace export");
    assert!(
        off.argv(&Map::new())
            .expect("argv")
            .iter()
            .all(|arg| !arg.starts_with("--out="))
    );
}

#[test]
fn rebase_keeps_rooted_and_drive_paths_for_the_child_to_judge() {
    let base = root();
    assert_eq!(rebase(&base, r"\rooted.png"), r"\rooted.png");
    assert_eq!(rebase(&base, r"\\server\share\x"), r"\\server\share\x");
    assert_eq!(
        rebase(&base, "dir/x.png"),
        base.join("dir/x.png").to_string_lossy()
    );
}
