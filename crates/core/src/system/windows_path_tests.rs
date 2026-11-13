use super::is_windows_network_path;

#[test]
fn nt_unc_globalroot_devices_and_relative_drives_are_rejected() {
    for path in [
        r"\??\UNC\server\share\x.png",
        r"\??\unc\server\share\x.png",
        r"\??\GLOBALROOT\Device\x.png",
        "/??/UNC/server/share/x.png",
        r"\??\pipe\x",
        r"\??\C:relative.png",
        r"\??\1:\x.png",
        r"\??\",
        r"\\server\share\x.png",
        r"\\?\UNC\server\share\x.png",
        r"\\.\pipe\x",
        r"/\server/share/x.png",
    ] {
        assert!(is_windows_network_path(path), "{path}");
    }
}

#[test]
fn nt_and_extended_absolute_drives_and_ordinary_paths_remain_allowed() {
    for path in [
        r"\??\C:\images\x.png",
        "/??/c:/images/x.png",
        r"\\?\C:\images\x.png",
        "//?/c:/images/x.png",
        r"C:\images\x.png",
        "C:/images/x.png",
        "image.png",
        r"images\x.png",
        "",
    ] {
        assert!(!is_windows_network_path(path), "{path}");
    }
}
