use super::absolute_cwd;
use agent_desktop_core::ErrorCode;
use std::path::Path;

#[test]
fn absolute_cwd_refuses_network_and_device_namespace_directories() {
    for cwd in [
        r"\\server\share\dir",
        r"\\?\UNC\server\share\dir",
        r"\\.\pipe\dir",
    ] {
        let error = absolute_cwd(Path::new(cwd)).expect_err(cwd);
        assert_eq!(error.code, ErrorCode::InvalidArgs, "{cwd}");
    }
}

#[test]
fn absolute_cwd_keeps_local_verbatim_directories() {
    let cwd = Path::new(r"\\?\C:\agent-desktop-launch-cwd-probe");
    assert_eq!(absolute_cwd(cwd).expect("local verbatim cwd"), cwd);
}
