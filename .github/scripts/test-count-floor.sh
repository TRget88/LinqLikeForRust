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
#   - `--workspace`, not just the root package. The repository grew a sibling
#     crate (`linq_rs_sql`), and a bare `cargo test` at the root would silently
#     ignore its 32 tests -- which is the same "exit 0 having run nothing"
#     failure this gate exists to prevent, one directory over.
#   - We enforce a floor PER BUCKET, not one sum. `--all-targets` EXCLUDES
#     doctests and `--doc` is the only way to run them, so a single total would
#     let one bucket collapse while the other grew -- and the historical bug was
#     exactly a whole bucket disappearing (main: all-targets 0, doctests 17).
#     Missing that split is also how a repo ends up with an MSRV job that never
#     type-checks a single example.
#   - Output is never truncated. A `| tail -n` here would defeat the purpose.

set -euo pipefail

# Per-bucket floors. Keep them in this file only -- never restate a count in a
# .md, or it becomes a second copy of the truth (D-016).
FLOOR_ALL_TARGETS=169
FLOOR_DOCTESTS=48
FLOOR_TOTAL=217

total=0
fail=0
declare -A counts=()

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
  counts[$label]=$n
  total=$((total + n))
}

run_and_count "all-targets" cargo test --workspace --all-targets
run_and_count "doctests"    cargo test --workspace --doc

echo "======================================================"

# A failed run makes every count meaningless: a target reporting
# "test result: FAILED. 64 passed; 1 failed" contributes 0 to the sum, so
# printing a total here would read as a count regression rather than a test
# failure. Report the failure and stop.
if [ "$fail" -ne 0 ]; then
  echo "FAIL: a cargo test invocation exited non-zero (see above)."
  echo "      Counts are not reported: a FAILED target contributes 0 to the sum,"
  echo "      so any total printed here would be misleading."
  exit 1
fi

echo "all-targets: ${counts[all-targets]:-0} (floor ${FLOOR_ALL_TARGETS})"
echo "doctests:    ${counts[doctests]:-0} (floor ${FLOOR_DOCTESTS})"
echo "total:       ${total} (floor ${FLOOR_TOTAL})"

if [ "$total" -eq 0 ]; then
  echo "FAIL: zero tests executed. Exit status 0 is not evidence -- see D-013."
  exit 1
fi

status=0
check() {
  local label="$1" got="$2" floor="$3"
  if [ "$got" -lt "$floor" ]; then
    echo "FAIL: ${label} executed count dropped from ${floor} to ${got}."
    status=1
  elif [ "$got" -gt "$floor" ]; then
    echo "NOTE: ${label} rose to ${got}. Consider raising its floor to match."
  fi
}
check "all-targets" "${counts[all-targets]:-0}" "$FLOOR_ALL_TARGETS"
check "doctests"    "${counts[doctests]:-0}"    "$FLOOR_DOCTESTS"
check "total"       "$total"                    "$FLOOR_TOTAL"

if [ "$status" -ne 0 ]; then
  echo
  echo "If a drop is intentional, lower the matching FLOOR_* in THIS FILE in the"
  echo "same commit that removes the tests, and say why in the commit message."
  echo "Never lower it in a separate \"fix CI\" commit."
  exit 1
fi

echo "PASS"
