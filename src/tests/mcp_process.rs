use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_agent-desktop"))
}

fn serve(requests: &[Value], args: &[&str]) -> Vec<Value> {
    let mut child = binary()
        .arg("mcp")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("binary starts");
    let mut stdin = child.stdin.take().expect("stdin");
    for request in requests {
        writeln!(stdin, "{request}").expect("request written");
    }
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("server exits after stdin closes");
    assert!(output.status.success());
    let mut responses = String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("one JSON-RPC message per line"))
        .collect::<Vec<_>>();
    responses.sort_by_key(|response| response["id"].as_i64());
    responses
}

fn call(id: i64, name: &str, arguments: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": { "name": name, "arguments": arguments } })
}

#[test]
fn mcp_tool_calls_run_the_cli_command_and_return_its_envelope() {
    let responses = serve(
        &[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": { "protocolVersion": "2025-11-25", "capabilities": {},
                                "clientInfo": { "name": "test", "version": "0" } } }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            call(2, "desktop_version", json!({})),
            call(
                3,
                "desktop_skills_get",
                json!({ "name": "desktop", "reference": null }),
            ),
        ],
        &[],
    );
    assert_eq!(responses.len(), 3, "{responses:?}");
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    let version = &responses[1]["result"];
    assert_eq!(version["isError"], false, "{version}");
    assert_eq!(version["structuredContent"]["command"], "version");
    assert_eq!(
        version["structuredContent"]["data"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    let skill = &responses[2]["result"];
    assert_eq!(skill["isError"], false, "{skill}");
    assert!(
        skill["structuredContent"]["data"]["content"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
}

#[test]
fn mcp_server_global_flags_reach_every_call() {
    let responses = serve(
        &[call(1, "desktop_session_list", json!({}))],
        &["--debug", "--screenshot", "x.html"],
    );
    let result = &responses[0]["result"];
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(
        result["structuredContent"]["error"]["message"],
        "--debug supports snapshot and click only"
    );
}

#[test]
fn malformed_messages_get_errors_and_the_server_keeps_serving() {
    let responses = serve(
        &[
            json!([{ "jsonrpc": "2.0", "id": 9, "method": "tools/list" }]),
            json!(42),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "no/such/method" }),
            call(4, "desktop_skills_get", json!({ "name": "a\u{0}b" })),
            call(5, "desktop_version", json!({})),
        ],
        &[],
    );
    let codes = responses
        .iter()
        .map(|response| (response["id"].clone(), response["error"]["code"].clone()))
        .collect::<Vec<_>>();
    assert_eq!(codes[0], (Value::Null, json!(-32600)), "{responses:?}");
    assert_eq!(codes[1], (Value::Null, json!(-32600)), "{responses:?}");
    assert_eq!(codes[2], (json!(2), json!(-32601)), "{responses:?}");
    let nul = &responses[3]["result"];
    assert!(
        nul["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("Cannot run agent-desktop")),
        "{nul}"
    );
    assert_eq!(responses[4]["result"]["isError"], false);
}

fn listed_tools(args: &[&str]) -> Vec<String> {
    let responses = serve(
        &[json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })],
        args,
    );
    responses[0]["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("name").to_string())
        .collect()
}

#[test]
fn mcp_server_hides_launch_batch_and_session_gc_until_each_is_allowed() {
    let default = listed_tools(&[]);
    for name in ["desktop_launch", "desktop_batch", "desktop_session_gc"] {
        assert!(!default.iter().any(|tool| tool == name), "{name}");
    }
    assert!(default.iter().any(|tool| tool == "desktop_snapshot"));
    let opted_in = listed_tools(&["--allow-launch", "--allow-batch"]);
    assert!(opted_in.iter().any(|tool| tool == "desktop_launch"));
    assert!(opted_in.iter().any(|tool| tool == "desktop_batch"));
    assert!(!opted_in.iter().any(|tool| tool == "desktop_session_gc"));
}

#[test]
fn mcp_server_refuses_force_and_unlisted_tools_before_anything_runs() {
    let responses = serve(
        &[
            call(
                1,
                "desktop_press",
                json!({ "combo": "win+r", "force": true }),
            ),
            call(2, "desktop_batch", json!({ "commands_json": "[]" })),
            call(3, "desktop_launch", json!({ "app": "notepad" })),
        ],
        &[],
    );
    let force = &responses[0]["result"];
    assert_eq!(force["isError"], true, "{force}");
    assert!(
        force["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("Unknown argument 'force'")),
        "{force}"
    );
    assert_eq!(responses[1]["error"]["code"], -32602, "{responses:?}");
    assert_eq!(responses[2]["error"]["code"], -32602, "{responses:?}");
}

#[test]
fn a_child_started_by_the_server_refuses_commands_outside_its_limits() {
    let run = |allow: &str| {
        let output = binary()
            .args(["batch", "[]"])
            .env("AGENT_DESKTOP_MCP_ALLOW", allow)
            .output()
            .expect("binary runs");
        serde_json::from_slice::<Value>(&output.stdout).expect("one JSON envelope")
    };
    let refused = run("");
    assert_eq!(refused["ok"], false, "{refused}");
    assert_eq!(refused["error"]["code"], "POLICY_DENIED", "{refused}");
    let allowed = run("batch");
    assert_ne!(allowed["error"]["code"], "POLICY_DENIED", "{allowed}");
}

#[test]
fn file_output_without_a_session_stops_the_server_instead_of_guessing_a_directory() {
    let output = binary()
        .args(["mcp", "--allow-file-write"])
        .env_remove("AGENT_DESKTOP_SESSION")
        .stdin(Stdio::null())
        .output()
        .expect("binary runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--session"));
}
