use super::*;

fn start_session_in_dir(dir: &std::path::Path) -> String {
    let mut c = binary();
    c.args(["session", "start", "--no-trace"])
        .env("AGENT_DESKTOP_HOME", dir);
    let out = c.output().expect("session start runs");
    let env: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    env["data"]["session_id"].as_str().unwrap().to_owned()
}

fn run_batch(dir: &std::path::Path, cmds: serde_json::Value, extra: &[&str]) -> serde_json::Value {
    let mut c = binary();
    c.arg("batch")
        .arg(cmds.to_string())
        .args(extra)
        .env("AGENT_DESKTOP_HOME", dir);
    let o = c.output().expect("batch runs");
    serde_json::from_slice(&o.stdout).unwrap_or_else(|error| {
        panic!(
            "batch stdout is not JSON: {error}; stderr: {}",
            String::from_utf8_lossy(&o.stderr)
        )
    })
}

#[test]
fn batch_entry_after_mid_batch_session_end_is_skipped_without_aborting_batch() {
    if support::run_in_private_session() {
        return;
    }
    let dir = unique_temp_dir("cli-batch-session-end");
    let session_id = start_session_in_dir(&dir);
    let data = run_batch(
        &dir,
        serde_json::json!([
            {"command": "session", "session": session_id, "args": {"action": "end"}},
            {"command": "clipboard-clear", "session": session_id, "args": {}},
            {"command": "version", "args": {}},
        ]),
        &[],
    )["data"]
        .clone();
    assert_eq!(data["results"][0]["execution"], "completed");
    assert_eq!(data["results"][0]["ok"], true);
    assert_eq!(data["results"][0]["data"]["session_id"], session_id);
    assert!(data["results"][0]["data"]["ended_at"].is_number());
    assert_eq!(data["results"][1]["execution"], "not_started");
    assert_eq!(data["results"][1]["not_started_reason"], "session_ended");
    assert_eq!(data["results"][1]["ok"], false);
    assert_eq!(data["results"][1]["error"]["code"], "INVALID_ARGS");
    assert!(
        data["results"][1]["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains(session_id.as_str()))
    );
    assert_eq!(data["results"][2]["execution"], "completed");
    assert_eq!(data["results"][2]["ok"], true);
    assert_eq!(data["completed_entries"], 2);
    assert_eq!(data["not_started_entries"], 1);
    assert!(data.get("stopped").is_none());
    let next_id = start_session_in_dir(&dir);
    let stopped = run_batch(
        &dir,
        serde_json::json!([
            {"command": "session", "session": next_id, "args": {"action": "end"}},
            {"command": "version", "session": next_id, "args": {}},
            {"command": "version", "args": {}}
        ]),
        &["--stop-on-error"],
    )["data"]
        .clone();
    assert_eq!(stopped["stopped"]["reason"], "stop_on_error");
    assert_eq!(stopped["results"].as_array().unwrap().len(), 2);
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("sessions").join(&session_id).join("session.json"))
            .expect("ended manifest remains on disk"),
    )
    .expect("manifest is JSON");
    assert!(manifest["ended_at"].is_number());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn batch_session_ended_skip_leaves_a_following_entry_of_another_session_running() {
    if support::run_in_private_session() {
        return;
    }
    let dir = unique_temp_dir("cli-batch-session-end-wait-cross");
    let sid_a = start_session_in_dir(&dir);
    let sid_b = start_session_in_dir(&dir);
    let env = run_batch(
        &dir,
        serde_json::json!([
            {"command": "session", "session": sid_a, "args": {"action": "end"}},
            {"command": "clipboard-clear", "session": sid_a, "args": {}},
            {"command": "version", "session": sid_b, "args": {}},
            {"command": "version", "args": {}}
        ]),
        &[],
    );
    let results = env["data"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 4, "no hard-stop: {env}");
    assert_eq!(results[1]["not_started_reason"], "session_ended", "{env}");
    assert_eq!(
        results[2]["execution"], "completed",
        "entry of the other session dispatched: {env}"
    );
    assert!(env["data"].get("stopped").is_none(), "{env}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn batch_session_ended_skip_covers_a_following_entry_of_the_same_session() {
    if support::run_in_private_session() {
        return;
    }
    let dir = unique_temp_dir("cli-batch-session-end-wait-same");
    let sid_a = start_session_in_dir(&dir);
    let env = run_batch(
        &dir,
        serde_json::json!([
            {"command": "session", "session": sid_a, "args": {"action": "end"}},
            {"command": "clipboard-clear", "session": sid_a, "args": {}},
            {"command": "version", "session": sid_a, "args": {}},
            {"command": "version", "args": {}}
        ]),
        &[],
    );
    let results = env["data"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 4, "no hard-stop: {env}");
    assert_eq!(results[1]["not_started_reason"], "session_ended", "{env}");
    assert_eq!(
        results[2]["not_started_reason"], "session_ended",
        "entry of the ended session skipped: {env}"
    );
    assert!(env["data"].get("stopped").is_none(), "{env}");
    let _ = std::fs::remove_dir_all(&dir);
}
