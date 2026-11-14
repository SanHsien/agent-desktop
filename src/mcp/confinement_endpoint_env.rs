//! The value checks for inspector variables: under `--allow-cdp` they may carry only the debugging endpoint they exist for.

use super::node_options_hold_only_endpoints;

/// The Chromium switches `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` may carry for an MCP client.
const WEBVIEW2_ENDPOINT_SWITCHES: &[&str] = &["--remote-debugging-port", "--remote-allow-origins"];
/// The one Java agent the Java option variables may load for an MCP client.
const JDWP_AGENT_PREFIX: &str = "-agentlib:jdwp=";
/// JDWP sub-options that only shape the debugging endpoint. `transport` names a library JDWP loads, so it must be one
/// of the bundled transports; `launch`, `onthrow`, `onuncaught`, `onjcmd`, `logfile` and anything unknown are refused.
const JDWP_ENDPOINT_SUBOPTIONS: &[&str] = &[
    "transport",
    "server",
    "address",
    "suspend",
    "timeout",
    "quiet",
    "includevirtualthreads",
];
const JDWP_BUNDLED_TRANSPORTS: &[&str] = &["dt_socket", "dt_shmem"];

/// True when an inspector variable's value carries only the debugging endpoint it exists for. `NODE_OPTIONS` keeps its
/// inspector options; `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` keeps the remote-debugging switches, because Chromium
/// launcher switches such as `--renderer-cmd-prefix` start other programs; the Java option variables keep a JDWP agent
/// whose sub-options only shape the endpoint and whose transport is bundled, because `-agentpath:`, `-javaagent:`,
/// `@argfiles`, a JDWP `transport=` library and JDWP's `launch=` load or run code.
/// Every other variable is not an inspector variable and passes here.
pub(in crate::mcp) fn inspector_env_value_is_endpoint_only(key: &str, value: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    let unquoted: String = value
        .chars()
        .filter(|ch| !matches!(ch, '"' | '\''))
        .collect();
    let mut tokens = unquoted
        .split(char::is_whitespace)
        .filter(|token| !token.is_empty());
    match upper.as_str() {
        "NODE_OPTIONS" => node_options_hold_only_endpoints(value),
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS" => tokens.all(|token| {
            let name = token
                .split('=')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            WEBVIEW2_ENDPOINT_SWITCHES.contains(&name.as_str())
        }),
        "JAVA_TOOL_OPTIONS"
        | "_JAVA_OPTIONS"
        | "JDK_JAVA_OPTIONS"
        | "IBM_JAVA_OPTIONS"
        | "OPENJ9_JAVA_OPTIONS" => tokens.all(is_endpoint_only_jdwp_agent),
        _ => true,
    }
}

/// A `-agentlib:jdwp=` token whose every sub-option shapes the endpoint and whose transport is a bundled one.
fn is_endpoint_only_jdwp_agent(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    let Some(options) = lower.strip_prefix(JDWP_AGENT_PREFIX) else {
        return false;
    };
    !options.is_empty()
        && options.split(',').all(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            JDWP_ENDPOINT_SUBOPTIONS.contains(&name)
                && (name != "transport" || JDWP_BUNDLED_TRANSPORTS.contains(&value))
        })
}
