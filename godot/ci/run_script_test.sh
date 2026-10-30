#!/usr/bin/env bash
# run-script.sh's contract (ADR 30.10.26b D4), over the fixture project in
# godot/ci/fixtures/: a run passes only on exit 0 and the success line, a
# hang is killed at the bound, --check-only fails on an error line, no
# lane runs `--script` around the runner, and the watchdog's signal is KILL
# (30.10.26c). Unlike the other *_test.sh here it
# needs a real Godot, so it reads GODOT (`make test-godot` passes it).
#
# The parse-error and runtime-error rows assert the outcome and the bound,
# never macOS 4.5.2's exit codes: CI runs Linux 4.5.2 and 4.7.2, which
# nobody has measured.
set -euo pipefail

: "${GODOT:?set GODOT to a Godot binary (make test-godot passes it)}"
root=$(cd "$(dirname "$0")/../.." && pwd)
runner="$root/godot/ci/run-script.sh"
fixtures="$root/godot/ci/fixtures"
marker=LINGARA_GODOT_SCRIPT_OK
# A pass row's bound is generous so a cold start on a loaded machine cannot
# flake it; a hang row's is short.
pass_bound=20
hang_bound=5

work=$(mktemp -d "${TMPDIR:-/tmp}/run-script-test.XXXXXX")
trap 'rm -rf "$work"' EXIT

# run <bound> [--check-only] <fixture>: the runner over one fixture, its
# stdout and stderr in $work/out, its exit code in $rc and its wall clock in
# $took.
run() {
  local bound=$1 mode=
  shift
  if [ "$1" = --check-only ]; then mode=--check-only; shift; fi
  local start=$SECONDS
  set +e
  GODOT_SCRIPT_TIMEOUT=$bound bash "$runner" $mode -- --path "$fixtures" --script "res://$1" >"$work/out" 2>&1
  rc=$?
  set -e
  took=$((SECONDS - start))
}

fails() {
  echo "✗ $1"
  sed 's/^/    /' "$work/out"
  return 1
}

# 30.10.26b AC1.
test_a_script_that_reaches_its_marker_passes() {
  run $pass_bound ok.gd
  [ "$rc" -eq 0 ] || fails "ok.gd failed (rc $rc)"
  grep -qx "$marker" "$work/out" || fails "ok.gd's log was not passed through"
  echo "✓ a script that reaches its marker passes"
}

# 30.10.26b AC8.
test_a_passing_run_does_not_hold_the_pipe_open() {
  local start=$SECONDS
  set +e
  GODOT_SCRIPT_TIMEOUT=$pass_bound bash "$runner" -- --path "$fixtures" --script res://ok.gd 2>&1 | cat >"$work/out"
  rc=${PIPESTATUS[0]}
  set -e
  took=$((SECONDS - start))
  [ "$rc" -eq 0 ] || fails "ok.gd failed through a pipe (rc $rc)"
  [ "$took" -le 10 ] || fails "the pipe closed ${took}s after start, not within 10s: the watchdog holds it"
  echo "✓ a passing run's pipe closes within 10s (${took}s)"
}

# 30.10.26b AC2.
test_a_parse_error_fails() {
  run $hang_bound parse_error.gd
  [ "$rc" -ne 0 ] || fails "parse_error.gd passed"
  if grep -q "timed out" "$work/out"; then fails "parse_error.gd timed out rather than failing by name"; fi
  grep -qE "parse_error.gd: (exited [0-9]+ without $marker|exit [0-9]+)$" "$work/out" \
    || fails "parse_error.gd's failure is not named"
  echo "✓ a parse error fails by name, not by the bound"
}

# 30.10.26b AC3.
test_a_runtime_error_fails_within_the_bound() {
  run $hang_bound runtime_error.gd
  [ "$rc" -ne 0 ] || fails "runtime_error.gd passed"
  [ "$took" -le $((hang_bound + 2)) ] || fails "runtime_error.gd took ${took}s, past the bound"
  grep -qE "runtime_error.gd: (timed out after ${hang_bound}s|exited [0-9]+ without $marker|exit [0-9]+)$" "$work/out" \
    || fails "runtime_error.gd's failure is not named"
  echo "✓ a runtime error fails within the bound (${took}s)"
}

# 30.10.26b AC4.
test_a_script_that_never_quits_fails() {
  run $hang_bound no_quit.gd
  [ "$rc" -ne 0 ] || fails "no_quit.gd passed"
  [ "$took" -le $((hang_bound + 2)) ] || fails "no_quit.gd took ${took}s, past the bound"
  grep -q "no_quit.gd: timed out after ${hang_bound}s$" "$work/out" || fails "no_quit.gd did not time out by name"
  echo "✓ a script that never quits fails at the bound (${took}s)"
}

# 30.10.26c AC1. Godot-free: the runner kills the pid it started, so a stub
# shell stands in for Godot and records any TERM it is sent. The stub waits
# on a background `sleep` because bash holds a trapped signal until a
# foreground child finishes, which would hide the TERM until the KILL.
test_the_watchdog_kills_without_terminating() {
  local stub="$work/stub-godot" termed="$work/termed"
  printf '#!/usr/bin/env bash\ntrap %s TERM\nwhile :; do sleep 1 & wait $!; done\n' \
    "': >\"$termed\"'" >"$stub"
  chmod +x "$stub"
  local start=$SECONDS
  set +e
  GODOT="$stub" GODOT_SCRIPT_TIMEOUT=2 bash "$runner" -- --path "$fixtures" --script res://no_quit.gd >"$work/out" 2>&1
  rc=$?
  set -e
  took=$((SECONDS - start))
  [ ! -e "$termed" ] || fails "the watchdog sent the run TERM"
  grep -q "no_quit.gd: timed out after 2s$" "$work/out" || fails "the stub run did not time out by name"
  [ "$took" -le $((2 + 2)) ] || fails "the stub run took ${took}s, past the bound"
  echo "✓ the watchdog kills a run past its bound without TERM (${took}s)"
}

# 30.10.26b AC5.
test_quit_zero_without_the_marker_fails() {
  run $pass_bound no_marker.gd
  [ "$rc" -ne 0 ] || fails "no_marker.gd passed"
  grep -q "no_marker.gd: exited 0 without $marker$" "$work/out" || fails "no_marker.gd's failure is not named"
  echo "✓ exit 0 without the success line fails"
}

# 30.10.26b AC6.
test_check_only_fails_a_parse_error_and_passes_a_clean_script() {
  run $pass_bound --check-only parse_error.gd
  [ "$rc" -ne 0 ] || fails "--check-only passed parse_error.gd"
  grep -qE "parse_error.gd: ((SCRIPT )?ERROR:.*|exit [0-9]+)$" "$work/out" \
    || fails "--check-only's failure on parse_error.gd names no error line"
  run $pass_bound --check-only ok.gd
  [ "$rc" -eq 0 ] || fails "--check-only failed ok.gd (rc $rc)"
  echo "✓ --check-only fails a parse error and passes a clean script"
}

# bypasses <file…>: file:line for each logical line (`\`-continuations
# joined, comment lines dropped) that runs `--script` without the runner.
bypasses() {
  awk '
    FNR == 1 { buf = "" }
    {
      if (buf == "") start = FNR
      line = $0
      if (sub(/\\$/, "", line)) { buf = buf line " "; next }
      buf = buf line
      lead = buf
      sub(/^[ \t]+/, "", lead)
      if (lead !~ /^@?#/ && index(buf, "--script") && !index(buf, "run-script.sh")) print FILENAME ":" start
      buf = ""
    }
  ' "$@"
}

# 30.10.26b AC7.
test_no_script_run_bypasses_the_runner() {
  local scanned=() f found
  for f in "$root"/make/*.mk "$root"/godot/ci/*.sh; do
    case "$(basename "$f")" in run-script.sh | run_script_test.sh) ;; *) scanned+=("$f") ;; esac
  done
  found=$(bypasses "${scanned[@]}")
  if [ -n "$found" ]; then
    echo "✗ a --script run bypasses godot/ci/run-script.sh:"
    echo "$found" | sed "s|^$root/|    |"
    return 1
  fi
  # The known positive: the five call sites as they were before the runner.
  found=$(bypasses "$fixtures"/bypass/* | wc -l | tr -d ' ')
  [ "$found" -eq 5 ] || { echo "✗ the bypass check found $found bypasses in fixtures/bypass/, not 5"; return 1; }
  echo "✓ no --script run bypasses the runner, and the check finds the five old ones"
}

test_a_script_that_reaches_its_marker_passes
test_a_passing_run_does_not_hold_the_pipe_open
test_a_parse_error_fails
test_a_runtime_error_fails_within_the_bound
test_a_script_that_never_quits_fails
test_the_watchdog_kills_without_terminating
test_quit_zero_without_the_marker_fails
test_check_only_fails_a_parse_error_and_passes_a_clean_script
test_no_script_run_bypasses_the_runner
