#!/usr/bin/env bash
#
# Enforces DECISIONS.md D-009 (publishing prerequisites) and D-012 (licence).
#
# Why this exists. 0.1.0 was published with no `repository` field, MIT-only
# against a dual-licence ruling, and carrying 421 lines of never-compiled test
# code in the tarball. Cargo warned about the missing metadata and the warning
# was ignored. A ruling in DECISIONS.md that is checked by nothing is a ruling
# that will be broken again, so this is the check.

set -euo pipefail

fail=0
err() { echo "FAIL: $*"; fail=1; }

manifest_field() {
  cargo metadata --no-deps --format-version 1 \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['packages'][0].get('$1') or '')"
}

echo "=== licence (D-012) ==="
lic="$(manifest_field license)"
echo "license = '${lic}'"
[ "$lic" = "MIT OR Apache-2.0" ] || err "license must be exactly 'MIT OR Apache-2.0', got '${lic}'"
for f in LICENSE-MIT LICENSE-APACHE; do
  if [ -f "$f" ]; then echo "present: $f"; else err "missing $f"; fi
done
# The canonical Apache-2.0 text, so a truncated or edited copy is caught.
apache_sha=cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30
if [ -f LICENSE-APACHE ]; then
  got="$(sha256sum LICENSE-APACHE | cut -d' ' -f1)"
  if [ "$got" = "$apache_sha" ]; then
    echo "LICENSE-APACHE matches the canonical text"
  else
    err "LICENSE-APACHE is not the canonical Apache-2.0 text (sha256 ${got})"
  fi
fi

echo
echo "=== source link (D-009) ==="
repo="$(manifest_field repository)"
echo "repository = '${repo}'"
[ -n "$repo" ] || err "repository must be set; a published crate with no source link is not acceptable"

echo
echo "=== tarball contents (D-009) ==="
# --allow-dirty so this is runnable locally on a work-in-progress tree; on CI the
# checkout is clean and the flag is a no-op.
listing="$(cargo package --list --allow-dirty)"
printf '%s\n' "$listing"
echo
# Must NOT ship: internal-only files.
while read -r bad; do
  [ -z "$bad" ] && continue
  if printf '%s\n' "$listing" | grep -q "^${bad}"; then
    err "tarball ships '${bad}', which exclude should have removed"
  fi
done <<'EXCLUDED'
AUDIT.md
QUESTIONS.md
CLAUDE.md
.github/
EXCLUDED
# Must ship: the things whose absence would make a claim unverifiable.
while read -r want; do
  [ -z "$want" ] && continue
  if printf '%s\n' "$listing" | grep -q "^${want}"; then
    echo "ships (intended): ${want}"
  else
    err "tarball is missing '${want}', which D-009 deliberately keeps"
  fi
done <<'REQUIRED'
tests/
DECISIONS.md
LICENSE-MIT
LICENSE-APACHE
REQUIRED

echo
[ "$fail" -eq 0 ] || { echo "packaging gate FAILED"; exit 1; }
echo "PASS"
