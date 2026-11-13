use super::*;

const NETWORK_IDENTIFIERS: [&str; 9] = [
    r"\\server\share\app.exe",
    r"\\127.0.0.1\c$\app.exe",
    r"\\?\UNC\server\share\app.exe",
    r"\\?\unc\server\share\app.exe",
    r"\\.\pipe\agent-desktop",
    r"\\?\GLOBALROOT\Device\HarddiskVolume1\app.exe",
    r"//server/share/app.exe",
    r"\/server\share\app.exe",
    r"/\server/share/app.exe",
];

#[test]
fn network_and_device_namespace_paths_are_refused_as_launch_identifiers() {
    for identifier in NETWORK_IDENTIFIERS {
        let error = launch_identifier(identifier).expect_err(identifier);
        assert_eq!(error.code, ErrorCode::InvalidArgs, "{identifier}");
        assert!(
            error.message.contains("network"),
            "{identifier}: the error must name the network path rule, got {:?}",
            error.message
        );
    }
}

/// `AppNotFound` would mean the path reached `is_file`, which is the SMB request
/// that leaks the user's NetNTLM hash; `InvalidArgs` means it was refused first.
#[test]
fn network_paths_are_refused_before_the_filesystem_is_touched() {
    for identifier in NETWORK_IDENTIFIERS {
        let error = resolve_executable(identifier).expect_err(identifier);
        assert_eq!(error.code, ErrorCode::InvalidArgs, "{identifier}");
    }
}

#[test]
fn network_path_detection_ignores_local_drive_paths() {
    for identifier in [
        r"C:\Windows\System32\notepad.exe",
        r"\\?\C:\Windows\notepad.exe",
        r"\\?\c:\Windows\notepad.exe",
        "notepad.exe",
        r"D:\tools\app.exe",
    ] {
        assert!(!is_network_path(identifier), "{identifier}");
    }
    for identifier in NETWORK_IDENTIFIERS {
        assert!(is_network_path(identifier), "{identifier}");
    }
}

#[test]
fn verbatim_local_drive_paths_remain_launch_identifiers() {
    assert!(launch_identifier(r"\\?\C:\Windows\notepad.exe").is_ok());
    assert!(launch_identifier(r"\\?\c:\Windows\notepad.exe").is_ok());
}

#[test]
fn nt_unc_globalroot_and_device_launch_paths_are_refused_by_the_network_guard() {
    for path in [
        r"\??\UNC\server\share\app.exe",
        r"\??\GLOBALROOT\Device\app.exe",
        "/??/UNC/server/share/app.exe",
        r"\??\pipe\app",
        r"\??\C:relative.exe",
    ] {
        assert!(is_network_path(path), "{path}");
        let error = refuse_network_path(path).expect_err(path);
        assert_eq!(error.code, ErrorCode::InvalidArgs);
        assert!(error.message.contains("NetNTLM"));
        assert!(error.suggestion.is_some());
        assert!(
            launch_identifier(path)
                .unwrap_err()
                .message
                .contains("NetNTLM")
        );
        assert!(
            resolve_executable(path)
                .unwrap_err()
                .message
                .contains("NetNTLM")
        );
    }
}

#[test]
fn nt_absolute_drive_launch_paths_are_not_network_paths() {
    assert!(!is_network_path(r"\??\C:\Windows\notepad.exe"));
}
