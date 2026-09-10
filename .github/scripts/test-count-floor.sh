#!/usr/bin/env bash
#
# Enforces DECISIONS.md D-013: "The tests must actually run, and the count must
# not drop."
#
# Why this exists. `cargo test` exits 0 while running zero tests. In this repo
# that was not hypothetical: linq_tests.rs sat at the crate root with 42 #[test]
# functions that were never a cargo target, so for five months CI-shaped
# commands reported success while running none of them -- and a genuine
# correctness bug (then_by discarding the primary sort key) sat undetected
# behind a test that already caught it.
#
# So exit status is not the gate. The executed test count is.
#
# If this script fails because you deliberately removed tests, lower FLOOR in
# the same commit that removes them, and say why in the commit message. Never
# lower it in a separate "fix CI" commit -- that is how the floor stops meaning
# anything.
#
# Deliberate choices:
#   - We do NOT fail on an individual "running 0 tests" line. The lib target
#     legitimately has no unit tests (every test is an integration test or a
#     doctest), so that line is expected there.
#   - We sum across `--all-targets` (which EXCLUDES doctests) and `--doc`
#     (which is the only way to run them). Missing that split is how a repo
#     ends up with an MSRV job that never type-checks a single example.
#   - Output is never truncated. A `| tail -n` here would defeat the purpose.

set -euo pipefail

FLOOR=232

total=0
fail=0

run_and_count() {
  local label="$1"; shift
  echo "=== ${label}: $* ==="
  local out status
  set +e
  out="$("$@" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$out"
  if [ "$status" -ne 0 ]; then
    echo "!! ${label} exited ${status}"
    fail=1
  fi
  local n
  n="$(printf '%s\n' "$out" \
       | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
       | awk '{s+=$1} END {print s+0}')"
  echo "--- ${label}: ${n} tests passed ---"
  echo
  total=$((total + n))
}

run_and_count "all-targets" cargo test --all-targets
run_and_count "doctests"    cargo test --doc

echo "======================================================"
echo "executed tests: ${total}   floor: ${FLOOR}"

if [ "$fail" -ne 0 ]; then
  echo "FAIL: a cargo test invocation exited non-zero (see above)."
  exit 1
fi

if [ "$total" -eq 0 ]; then
  echo "FAIL: zero tests executed. Exit status 0 is not evidence -- see D-013."
  exit 1
fi

if [ "$total" -lt "$FLOOR" ]; then
  echo "FAIL: executed test count dropped from ${FLOOR} to ${total}."
  echo "      If this is intentional, lower FLOOR in the same commit and say why."
  exit 1
fi

if [ "$total" -gt "$FLOOR" ]; then
  echo "NOTE: count rose to ${total}. Consider raising FLOOR to match."
fi

echo "PASS"
