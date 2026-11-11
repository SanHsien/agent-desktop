use super::*;
use clap::Command;

fn parse(args: &[&str]) -> ArgMatches {
    Command::new("mcp")
        .arg(Arg::new("session").long("session").global(true))
        .args(flags())
        .try_get_matches_from(std::iter::once("mcp").chain(args.iter().copied()))
        .expect("the flags parse")
}

#[test]
fn nothing_is_allowed_unless_a_flag_says_so() {
    let limits = restrictions(&parse(&[])).expect("restrictions");
    assert_eq!(limits, Restrictions::default());
}

#[test]
fn each_flag_turns_on_only_its_own_capability() {
    let launch = restrictions(&parse(&["--allow-launch"])).expect("restrictions");
    assert_eq!(
        launch,
        Restrictions {
            launch: true,
            ..Restrictions::default()
        }
    );
    let rest = restrictions(&parse(&["--allow-batch", "--allow-cdp"])).expect("restrictions");
    assert_eq!(
        rest,
        Restrictions {
            batch: true,
            cdp: true,
            ..Restrictions::default()
        }
    );
}

#[test]
fn file_output_is_confined_to_a_folder_of_the_named_session() {
    let limits = restrictions(&parse(&["--allow-file-write", "--session", "s8f3k2p9"]))
        .expect("restrictions");
    let root = limits.file_root.expect("a root");
    assert!(limits.file_write);
    assert_eq!(
        root.file_name().and_then(|name| name.to_str()),
        Some("mcp-output")
    );
    let session = root.parent().expect("session dir");
    assert_eq!(
        session.file_name().and_then(|name| name.to_str()),
        Some("s8f3k2p9")
    );
    assert_eq!(
        session
            .parent()
            .and_then(|dir| dir.file_name())
            .and_then(|name| name.to_str()),
        Some("sessions")
    );
}

#[test]
fn file_output_without_a_session_stops_the_server_with_a_reason() {
    if std::env::var_os("AGENT_DESKTOP_SESSION").is_some() {
        return;
    }
    let error = restrictions(&parse(&["--allow-file-write"])).expect_err("needs a session");
    assert!(error.contains("--session"), "{error}");
}
