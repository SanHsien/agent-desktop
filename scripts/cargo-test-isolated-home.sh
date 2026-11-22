#!/usr/bin/env bash
set -euo pipefail

original_home="${HOME:?HOME is required}"
cargo_home="${CARGO_HOME:-$original_home/.cargo}"
rustup_home="${RUSTUP_HOME:-$original_home/.rustup}"
test_home="$(mktemp -d "${TMPDIR:-/tmp}/agent-desktop-test-home.XXXXXX")"

cleanup() {
    rm -rf -- "$test_home"
}
trap cleanup EXIT

# These variables override the isolated HOME or inject ambient state into tests
# (AGENT_DESKTOP_HOME takes precedence over HOME for the state root). Tests that
# need one set it themselves, so clear the caller's values.
unset AGENT_DESKTOP_HOME AGENT_DESKTOP_SESSION AGENT_DESKTOP_AGENT_ID     AGENT_DESKTOP_MCP_ALLOW AGENT_DESKTOP_MCP_FILE_ROOT

HOME="$test_home" CARGO_HOME="$cargo_home" RUSTUP_HOME="$rustup_home" cargo "$@"
