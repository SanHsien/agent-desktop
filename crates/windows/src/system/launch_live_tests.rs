use super::super::*;
use super::{
    KillOnDrop, copy_probe_binary, copy_system_exe, deadline, full_image_path, matching_pids,
    probe_launch_options, scratch_dir, start_named_windowless, start_windowed_probe,
};
use crate::system::launch_path::resolve_executable;
use agent_desktop_core::{Deadline, DeliveryDisposition, ErrorCode};
use std::sync::{Mutex, MutexGuard};

static NOTEPAD_LAUNCH_LOCK: Mutex<()> = Mutex::new(());

/// Serializes the tests that start windowed probes. A test that panics while
/// holding the lock poisons it; taking the guard anyway keeps that one failure
/// from turning every other test in the group into a `PoisonError` that hides
/// which test actually broke.
fn notepad_launch_lock() -> MutexGuard<'static, ()> {
    NOTEPAD_LAUNCH_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn scratch_launch_returns_window_for_created_process() {
    let _lock = notepad_launch_lock();
    crate::tree::fixture::bootstrap();
    let dir = scratch_dir("notepad-scratch");
    let probe = copy_probe_binary(&dir, "launchprobe_host_scratch.exe");
    let image = "launchprobe_host_scratch.exe";
    let window = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &probe_launch_options(false, 8_000),
        deadline(10_000),
    )
    .expect("launch notepad probe");
    let _guard = KillOnDrop(window.pid);
    assert!(
        matching_pids(image).contains(&window.pid),
        "returned window pid must belong to the probe notepad process"
    );
    assert!(window.process_instance.is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn attach_true_reuses_single_running_instance() {
    let _lock = notepad_launch_lock();
    crate::tree::fixture::bootstrap();
    let dir = scratch_dir("notepad-attach");
    let probe = copy_probe_binary(&dir, "launchprobe_host_attach.exe");
    let image = "launchprobe_host_attach.exe";
    let _first = start_windowed_probe(&probe);
    let before = matching_pids(image);
    assert_eq!(
        before.len(),
        1,
        "fixture must leave exactly one probe notepad"
    );
    let window = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &probe_launch_options(true, 8_000),
        deadline(10_000),
    )
    .expect("attach");
    assert_eq!(window.pid, before[0]);
    assert_eq!(matching_pids(image).len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn attach_true_does_not_reuse_same_basename_from_another_path() {
    let _lock = notepad_launch_lock();
    crate::tree::fixture::bootstrap();
    let dir = scratch_dir("same-basename");
    let intended_dir = dir.join("intended");
    let impostor_dir = dir.join("impostor");
    std::fs::create_dir_all(&intended_dir).expect("intended dir");
    std::fs::create_dir_all(&impostor_dir).expect("impostor dir");
    let image = format!("launchprobe_host_same_{}.exe", std::process::id());
    let intended = copy_probe_binary(&intended_dir, &image);
    let impostor = copy_probe_binary(&impostor_dir, &image);
    let running_impostor = start_windowed_probe(&impostor);
    assert_eq!(matching_pids(&image), vec![running_impostor.0]);

    let launched = launch_app_impl(
        intended.to_str().expect("intended path"),
        &probe_launch_options(true, 8_000),
        deadline(10_000),
    )
    .expect("launch the requested executable instead of attaching to its basename impostor");
    let launched_guard = KillOnDrop(launched.pid);
    assert_ne!(launched.pid, running_impostor.0);
    let actual = full_image_path(launched.pid).expect("launched image path");
    assert_eq!(
        actual.canonicalize().ok(),
        intended.canonicalize().ok(),
        "the returned process must run the requested path"
    );
    assert_eq!(matching_pids(&image).len(), 2);

    drop(launched_guard);
    drop(running_impostor);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn attach_false_fails_naming_running_pid() {
    let _lock = notepad_launch_lock();
    crate::tree::fixture::bootstrap();
    let dir = scratch_dir("notepad-noattach");
    let probe = copy_probe_binary(&dir, "launchprobe_host_noattach.exe");
    let image = "launchprobe_host_noattach.exe";
    let _running = start_windowed_probe(&probe);
    let running_pid = matching_pids(image)
        .into_iter()
        .next()
        .expect("probe notepad running");
    let error = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &LaunchOptions {
            attach_if_running: false,
            ..Default::default()
        },
        deadline(10_000),
    )
    .expect_err("no-attach must fail");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::NotDelivered
    );
    let details = error.details.expect("details");
    assert_eq!(details["pid"], u32::from(running_pid));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn attach_true_windowless_does_not_launch_second_process() {
    let dir = scratch_dir("windowless");
    let probe = copy_system_exe("ping.exe", &dir, "launchprobe_ping.exe");
    let image = "launchprobe_ping.exe";
    let _ping = start_named_windowless(&probe, &["-n", "60", "127.0.0.1"]);
    let running = matching_pids(image);
    assert_eq!(running.len(), 1);
    let result = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &LaunchOptions {
            attach_if_running: true,
            timeout_ms: 300,
            ..Default::default()
        },
        Deadline::after(1_000).expect("deadline"),
    )
    .expect("attaching to a windowless process is a successful launch, not a failure");
    assert!(
        result.window.is_none(),
        "a windowless process reports no window rather than an error: {result:?}"
    );
    assert_eq!(
        result.pid, running[0],
        "attach returns the already-running process rather than starting one"
    );
    assert_eq!(
        matching_pids(image).len(),
        1,
        "attach must not spawn a second process"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_matches_are_ambiguous_before_launch() {
    let _lock = notepad_launch_lock();
    crate::tree::fixture::bootstrap();
    let dir = scratch_dir("notepad-ambiguous");
    let probe = copy_probe_binary(&dir, "launchprobe_host_ambiguous.exe");
    let image = "launchprobe_host_ambiguous.exe";
    let _one = start_windowed_probe(&probe);
    let _two = start_windowed_probe(&probe);
    let before = matching_pids(image);
    assert!(before.len() >= 2);
    let error = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &LaunchOptions {
            attach_if_running: true,
            ..Default::default()
        },
        deadline(10_000),
    )
    .expect_err("ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::NotDelivered
    );
    assert_eq!(matching_pids(image).len(), before.len());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cwd_is_honored_for_create_process() {
    let temp = scratch_dir("cwd");
    let probe = copy_system_exe("cmd.exe", &temp, "launchprobe_cwd.exe");
    let marker = temp.join("cwd.txt");
    let options = LaunchOptions {
        args: vec!["/C".into(), format!("cd > \"{}\"", marker.display())],
        cwd: Some(temp.clone()),
        attach_if_running: false,
        timeout_ms: 0,
        ..Default::default()
    };
    let result = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &options,
        deadline(10_000),
    )
    .expect("a windowless probe is a successful launch");
    let _guard = KillOnDrop(result.pid);
    assert!(
        result.window.is_none(),
        "the probe presents no window: {result:?}"
    );
    let written = poll_read_marker(&marker, Deadline::after(5_000).expect("deadline"));
    let normalized_written = written.trim().trim_end_matches('\\').to_ascii_lowercase();
    let normalized_temp = temp
        .to_string_lossy()
        .trim_end_matches('\\')
        .to_ascii_lowercase();
    assert_eq!(normalized_written, normalized_temp);
    let _ = std::fs::remove_dir_all(&temp);
}

/// The marker is written by a child shell redirecting `cd` output, a step
/// that lands strictly after `CreateProcessW` returns to this test. Reading
/// it once, immediately, races that write; polling under a deadline lets the
/// child finish without turning a slow scheduler into a false failure.
fn poll_read_marker(marker: &std::path::Path, deadline: Deadline) -> String {
    loop {
        if let Ok(contents) = std::fs::read_to_string(marker) {
            if !contents.trim().is_empty() {
                return contents;
            }
        }
        if deadline.remaining().is_zero() {
            return std::fs::read_to_string(marker).expect("cwd marker");
        }
        std::thread::sleep(std::time::Duration::from_millis(50).min(deadline.remaining()));
    }
}

#[test]
fn timeout_zero_checks_once_for_process_without_window() {
    let dir = scratch_dir("timeout0");
    let probe = copy_system_exe("ping.exe", &dir, "launchprobe_timeout.exe");
    let result = launch_app_impl(
        probe.to_str().expect("utf8 path"),
        &LaunchOptions {
            args: vec!["-n".into(), "30".into(), "127.0.0.1".into()],
            attach_if_running: false,
            timeout_ms: 0,
            ..Default::default()
        },
        deadline(10_000),
    )
    .expect("a windowless process is a successful launch");
    let _guard = KillOnDrop(result.pid);
    assert!(
        result.window.is_none(),
        "timeout 0 observes once and reports no window rather than polling: {result:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Resolves `name` with the process current directory set to `plant_dir`, then
/// puts the directory back before returning, so no caller holds a changed cwd
/// across anything slow.
fn resolve_under_planted_cwd(name: &str, plant_dir: &std::path::Path) -> std::path::PathBuf {
    let previous = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(plant_dir).expect("enter plant dir");
    let resolved = resolve_executable(name);
    let _ = std::env::set_current_dir(&previous);
    resolved.expect("resolve ignores cwd")
}

/// A bare executable name must resolve to the system directories, never to a
/// same-named file sitting in the current directory. The hijack needs the
/// literal bare name, since bare-name resolution against the real system
/// directories is what is under test; a uniquely-named probe copy would not
/// exercise it. Resolution is captured once under the planted cwd and the cwd
/// is restored immediately, before the already-resolved absolute path is
/// launched, so nothing downstream depends on cwd being anything at all.
///
/// The launched process is `ping.exe`, not `notepad.exe`. What a launch of the
/// OS-supplied Notepad does differs per Windows build: some builds forward to
/// the Store app and exit at once (the window then belongs to another pid),
/// one build crashed on startup, and a Notepad the developer already has open
/// makes `attach_if_running: false` refuse the launch. A windowless console
/// tool has none of that, and the property under test is which image runs, not
/// that a window appears. Resolving `notepad.exe` is still checked, since that
/// needs no process.
#[test]
fn bare_name_never_chooses_planted_binary_in_current_directory() {
    let plant_dir = scratch_dir("hijack");
    for name in ["notepad.exe", "ping.exe"] {
        let planted = plant_dir.join(name);
        std::fs::write(&planted, b"not a real pe").expect("plant decoy");
        let system = resolve_executable(name).expect("system executable");
        let resolved = resolve_under_planted_cwd(name, &plant_dir);
        assert_ne!(
            resolved.canonicalize().ok(),
            planted.canonicalize().ok(),
            "resolver must not select the planted cwd binary for {name}"
        );
        assert_eq!(
            resolved.canonicalize().ok(),
            system.canonicalize().ok(),
            "the bare name {name} must resolve to the same system binary with or without a planted cwd"
        );
    }

    let resolved = resolve_under_planted_cwd("ping.exe", &plant_dir);
    let result = launch_app_impl(
        resolved.to_str().expect("utf8 path"),
        &LaunchOptions {
            args: vec!["-n".into(), "30".into(), "127.0.0.1".into()],
            attach_if_running: false,
            timeout_ms: 0,
            ..Default::default()
        },
        deadline(10_000),
    )
    .expect("launch system ping (no other ping.exe may be running)");
    let _guard = KillOnDrop(result.pid);
    let image = full_image_path(result.pid).expect("image path of the launched process");
    let image_canon = image.canonicalize().unwrap_or(image);
    let planted_canon = plant_dir
        .join("ping.exe")
        .canonicalize()
        .expect("planted decoy");
    assert_ne!(image_canon, planted_canon);
    assert_eq!(
        image_canon
            .file_name()
            .map(|name| name.to_ascii_lowercase()),
        Some(std::ffi::OsString::from("ping.exe"))
    );
    assert_eq!(
        image_canon.canonicalize().ok(),
        resolved.canonicalize().ok(),
        "the process that ran must be the resolved system binary"
    );
    let _ = std::fs::remove_dir_all(&plant_dir);
}
