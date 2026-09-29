#!/usr/bin/env bash
#
# Enforces DECISIONS.md's shared gate for the API-stability section:
#
#   "CI must refuse to build a `v1.*` tag while any `D-1xx` entry still reads
#    Status: OPEN."
#
# Why a release-boundary gate rather than a per-commit one. D-101..D-108 are
# decisions, not code. Nothing in src/ can be checked against an undecided
# ruling, so there is nothing to gate on a branch. But every one of them is
# cheap now and a breaking change after 1.0 -- which makes the tag the exact
# moment they stop being deferrable.
#
# Run with no arguments to report status. Exit code is 0 only if every D-1xx is
# settled, so it is safe to wire into a tag-triggered job as-is.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LEDGER="$ROOT/DECISIONS.md"

[ -f "$LEDGER" ] || { echo "FAIL: $LEDGER not found"; exit 1; }

echo "=== API-stability decisions (DECISIONS.md D-1xx) ==="
echo

open_count=0
resolved=0
total=0

# Walk each `## D-1NN` heading and the first `- **Status:**` line beneath it.
while IFS= read -r line; do
  case "$line" in
    '## D-1'*)
      id="${line#\#\# }"
      id="${id%% *}"
      current="$id"
      total=$((total + 1))
      ;;
    '- **Status:**'*)
      if [ -n "${current:-}" ]; then
        resolved=$((resolved + 1))
        status="${line#- \*\*Status:\*\* }"
        # Trim to the first sentence; entries carry a recommendation after it.
        short="$(printf '%s' "$status" | cut -c1-72)"
        if printf '%s' "$status" | grep -qw 'OPEN'; then
          printf '  %-8s OPEN      %s\n' "$current" "$short"
          open_count=$((open_count + 1))
        else
          printf '  %-8s settled   %s\n' "$current" "$short"
        fi
        current=""
      fi
      ;;
  esac
done < "$LEDGER"

echo
echo "${total} API-stability decisions; ${resolved} with a parsed Status; ${open_count} still OPEN."
echo

# An `open_count` of zero is only meaningful if the parse actually worked, and it
# did not have to. This gate used to pass on a ledger with NO `## D-1` headings,
# on an EMPTY ledger, and on entries whose Status lines had lost the leading `- `
# -- every parse failure biased toward PASS. That is D-013's lesson ("cargo test
# exits 0 while running zero tests") reappearing in the gate that guards 1.0. All
# three were reproduced before this fix; only the exactly-formatted OPEN path
# failed, so the OPEN path was never the problem -- the silence around it was.
#
# A heading with no readable Status is doubly bad: it counts as settled, AND it
# leaves the walker attributing the NEXT entry's Status to the wrong id.
#
# So the count is the gate, not the exit status -- the rule test-count-floor.sh
# states. FLOOR lives in this file and nowhere else; raise it in the same commit
# that adds a D-1xx entry.
FLOOR_D1XX=9

if [ "$total" -lt "$FLOOR_D1XX" ]; then
  echo "FAIL: found ${total} D-1xx heading(s); floor is ${FLOOR_D1XX}."
  echo "      Either the ledger lost entries, or the heading format changed and"
  echo "      this parser stopped seeing them. Both must fail, not pass."
  exit 1
fi

if [ "$resolved" -ne "$total" ]; then
  echo "FAIL: ${total} heading(s) but only ${resolved} parsed Status line(s)."
  echo "      An entry whose Status this parser cannot read is silently counted"
  echo "      as settled. Fix the ledger's format, or fix this parser."
  exit 1
fi

if [ "$open_count" -eq 0 ]; then
  echo "PASS: all ${total} D-1xx settled, all ${resolved} Status lines parsed."
  echo "      A 1.0 tag is allowed."
  exit 0
fi

cat <<MSG
FAIL: refusing a 1.0 release while ${open_count} API-stability decision(s) are OPEN.

Each of these is free to change now and a breaking change after 1.0. That is
the whole reason this gate exists at the tag rather than on the branch.

Settle them in DECISIONS.md -- change the Status line and record the ruling and
its reason -- then re-tag. To ship a pre-1.0 release in the meantime, tag
v0.x instead: this gate only guards v1.*.
MSG
exit 1
