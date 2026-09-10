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
echo "${total} API-stability decisions; ${open_count} still OPEN."
echo

if [ "$open_count" -eq 0 ]; then
  echo "PASS: every D-1xx is settled. A v1.* tag is allowed."
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
