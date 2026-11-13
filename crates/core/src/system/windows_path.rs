/// Detects Windows network and device namespaces without accessing the filesystem.
/// NT and extended prefixes accept only absolute drive-letter paths.
pub fn is_windows_network_path(path: &str) -> bool {
    let normalized = path.replace('/', "\\");
    let local = normalized
        .strip_prefix(r"\\?\")
        .or_else(|| normalized.strip_prefix(r"\??\"));
    if let Some(local) = local {
        let bytes = local.as_bytes();
        return !(bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\');
    }
    normalized.starts_with(r"\\")
}

#[cfg(test)]
#[path = "windows_path_tests.rs"]
mod tests;
