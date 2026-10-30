#!/usr/bin/env bash
# `make godot-headless` (ADR 1.10.26ab D10): K2's token server with --fake and
# `conformance-server serve` over the demo's own case, each on a port of its
# own choosing, then the headless demo against both. Both servers are killed
# however the run ends. Expects them built (the make target builds them) and
# GODOT and GODOT_SCRIPT_TIMEOUT set; the demo runs through run-script.sh
# (ADR 30.10.26b).
set -euo pipefail

: "${GODOT:?set GODOT to a Godot binary}"
root=$(cd "$(dirname "$0")/../.." && pwd)
bin="$root/target/debug"
work=$(mktemp -d)
pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$work"
}
trap cleanup EXIT

# start <name> <log> <args…>: runs a server, waits for `listening <port>`,
# and prints the port.
start() {
  local name=$1 log=$2
  shift 2
  "$bin/$name" "$@" > "$log" 2>&1 &
  pids+=("$!")
  for _ in $(seq 1 100); do
    if port=$(sed -n 's/^listening \([0-9]*\)$/\1/p' "$log" | head -1) && [ -n "$port" ]; then
      echo "$port"
      return 0
    fi
    sleep 0.1
  done
  echo "✗ $name never printed listening <port>:" >&2
  cat "$log" >&2
  return 1
}

api=$(start conformance-server "$work/api.log" serve --cases "$root/godot/demo/tests/cases" --port 0)
tokens=$(start lingara-demo-token-server "$work/tokens.log" --fake --port 0)

LINGARA_DEMO_API_URL="http://127.0.0.1:$api" \
LINGARA_DEMO_TOKEN_URL="http://127.0.0.1:$tokens/token" \
  bash "$root/godot/ci/run-script.sh" -- --path "$root/godot/demo" --script res://tests/headless_demo.gd
