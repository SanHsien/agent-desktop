use super::validate_optional;
use std::path::Path;

#[test]
fn local_relative_and_extended_drive_file_paths_remain_allowed() {
    for path in [
        "image.png",
        r"C:\images\image.png",
        "C:/images/image.png",
        r"\\?\C:\images\image.png",
    ] {
        assert!(validate_optional(Some(Path::new(path))).is_ok(), "{path}");
    }
    assert!(validate_optional(None).is_ok());
}

#[test]
fn unc_and_device_file_paths_return_invalid_args_without_file_access() {
    for path in [
        r"\\server\share\image.png",
        "//server/share/image.png",
        r"\\?\UNC\server\share\image.png",
        r"\\?\unc\server\share\image.png",
        "//?/UNC/server/share/image.png",
        r"\\.\pipe\image",
        r"\\?\GLOBALROOT\Device\image",
        r"/\server\share\image",
        r"\/server/share/image",
        r"\\?\C:relative.png",
    ] {
        let error = validate_optional(Some(Path::new(path))).expect_err(path);
        assert_eq!(error.code(), "INVALID_ARGS", "{path}: {error}");
        assert!(error.to_string().contains("UNC"), "{path}: {error}");
    }
}

#[test]
fn unc_file_paths_are_rejected_in_every_supported_batch_file_argument() {
    for (command, args) in [
        (
            "screenshot",
            serde_json::json!({"output_path": r"\\server\share\out.png"}),
        ),
        (
            "clipboard-get",
            serde_json::json!({"out": r"\\server\share\out.png"}),
        ),
        (
            "clipboard-set",
            serde_json::json!({"image": r"\\server\share\in.png"}),
        ),
        (
            "clipboard-set",
            serde_json::json!({"file_url": ["local.png", r"\\server\share\in.png"]}),
        ),
        (
            "trace",
            serde_json::json!({"action": "export", "out": r"\\server\share\out.html"}),
        ),
    ] {
        let command = crate::batch::parse_command(
            serde_json::from_value(serde_json::json!({"command": command, "args": args})).unwrap(),
        )
        .unwrap();
        let error = crate::command_policy::preflight(
            &command,
            &agent_desktop_core::PermissionReport::default(),
        )
        .expect_err("batch file argument must refuse UNC");
        assert_eq!(error.code(), "INVALID_ARGS");
        assert!(error.to_string().contains("UNC"), "{error}");
    }
}

#[test]
fn local_file_arguments_remain_allowed_for_every_cli_file_command() {
    use clap::Parser;

    for path in [
        "image.png",
        r"C:\images\image.png",
        r"\\?\C:\images\image.png",
    ] {
        for args in [
            vec!["screenshot", path],
            vec!["clipboard-get", "--out", path],
            vec!["clipboard-set", "--image", path],
            vec!["clipboard-set", "--file-url", path],
            vec!["trace", "export", "--out", path],
            vec!["cursor-overlay", "enable", "--image", path],
            vec!["cursor-overlay", "enable", "--pointer-image", path],
        ] {
            let cli = crate::cli::Cli::try_parse_from(
                std::iter::once("agent-desktop").chain(args.clone()),
            )
            .unwrap();
            assert!(
                super::validate_command(&cli.command.unwrap()).is_ok(),
                "{args:?}"
            );
        }
    }
}

#[test]
fn nt_unc_globalroot_and_device_file_paths_are_rejected_before_file_access() {
    for path in [
        r"\??\UNC\server\share\image.png",
        r"\??\GLOBALROOT\Device\image",
        "/??/UNC/server/share/image.png",
        r"\??\pipe\image",
        r"\??\C:relative.png",
    ] {
        let error = validate_optional(Some(Path::new(path))).expect_err(path);
        assert_eq!(error.code(), "INVALID_ARGS", "{path}");
        assert!(error.to_string().contains("NetNTLM"), "{path}: {error}");
        let agent_desktop_core::AppError::Adapter(error) = error else {
            panic!("path rejection must carry a suggestion");
        };
        assert!(error.suggestion.is_some());
    }
}

#[test]
fn nt_absolute_drive_file_paths_remain_allowed() {
    assert!(validate_optional(Some(Path::new(r"\??\C:\images\image.png"))).is_ok());
}
