#!/usr/bin/env bash
# Every lane run of `godot --script` goes through here (ADR 30.10.26b D1):
#
#   run-script.sh [--check-only] -- <godot args…>
#
# Godot cannot be trusted to fail: measured on macOS 4.5.2, a parse error
# exits 0 (--check-only included), a runtime error hangs, and --quit-after
# turns the hang into exit 0. So a run passes only when Godot exits 0 and
# its log holds the line LINGARA_GODOT_SCRIPT_OK, which each test prints on
# its passing path; under --check-only, which runs nothing, when it exits 0
# and printed no `SCRIPT ERROR:` / `ERROR:` line. A run that outlives
# GODOT_SCRIPT_TIMEOUT seconds is killed (KILL) at the bound.
#
# The log is buffered and written to stdout when the run ends, stderr
# merged in; the runner's own ✗ lines go to stderr. GODOT must be the Godot
# binary itself, not a wrapper: the pid killed is the pid started.
# bash 3.2 (stock macOS), no coreutils `timeout`.
set -uo pipefail

check_only=
if [ "${1:-}" = --check-only ]; then check_only=--check-only; shift; fi
if [ "${1:-}" != -- ]; then
  echo "usage: run-script.sh [--check-only] -- <godot args…>" >&2
  exit 2
fi
shift
: "${GODOT:?set GODOT to a Godot binary}"
: "${GODOT_SCRIPT_TIMEOUT:?set GODOT_SCRIPT_TIMEOUT to a bound in seconds (make/godot.mk defaults it)}"
bound=$GODOT_SCRIPT_TIMEOUT
marker=LINGARA_GODOT_SCRIPT_OK

script=godot
prev=
for arg in "$@"; do
  if [ "$prev" = --script ]; then script=$arg; fi
  prev=$arg
done

work=$(mktemp -d "${TMPDIR:-/tmp}/run-script.XXXXXX")
log="$work/log"
flag="$work/timed-out"
watchdog=
cleanup() {
  if [ -n "$watchdog" ]; then kill "$watchdog" 2>/dev/null; fi
  rm -rf "$work"
}
trap cleanup EXIT

"$GODOT" --headless $check_only "$@" </dev/null >"$log" 2>&1 &
godot=$!

# The watchdog holds no fd of the lane's and sleeps in a child its TERM
# trap kills: otherwise killing it on the success path leaves a `sleep`
# holding make's stdout, and a reader waiting for EOF (batch-lanes, a
# `| tee`) waits out the whole bound after every passing run. The flag
# file, not wait's 137, says the run timed out. The mono build's .NET
# runtime turns TERM into a teardown that aborts and leaves a crash report
# (ADR 30.10.26c), so the runner never sends it.
(
  s=
  trap - EXIT
  trap 'if [ -n "$s" ]; then kill "$s" 2>/dev/null; fi; exit 0' TERM
  sleep "$bound" &
  s=$!
  wait "$s"
  : >"$flag"
  kill -KILL "$godot" 2>/dev/null
) </dev/null >/dev/null 2>&1 &
watchdog=$!

wait "$godot"
rc=$?
kill "$watchdog" 2>/dev/null
wait "$watchdog" 2>/dev/null
watchdog=
cat "$log"

fail() {
  echo "✗ $script: $1" >&2
  exit 1
}

if [ -e "$flag" ]; then fail "timed out after ${bound}s"; fi
if [ -n "$check_only" ]; then
  error=$(grep -m 1 -E '^(SCRIPT ERROR:|ERROR:)' "$log")
  if [ -n "$error" ]; then fail "$error"; fi
  if [ "$rc" -ne 0 ]; then fail "exit $rc"; fi
else
  if [ "$rc" -ne 0 ]; then fail "exit $rc"; fi
  grep -qx "$marker" "$log" || fail "exited $rc without $marker"
fi
