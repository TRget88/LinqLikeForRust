#!/usr/bin/env bash
#
# Enforces DECISIONS.md D-009 (publishing prerequisites), D-012 (licence),
# D-001 (zero dependencies) and D-003/D-004 (no interior mutability).
#
# Why this exists. 0.1.0 was published with no `repository` field, MIT-only
# against a dual-licence ruling, and carrying 421 lines of never-compiled test
# code in the tarball. Cargo warned about the missing metadata and the warning
# was ignored. A ruling in DECISIONS.md that is checked by nothing is a ruling
# that will be broken again, so this is the check.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail=0
err() { echo "FAIL: $*"; fail=1; }

manifest_field() {
  cargo metadata --no-deps --format-version 1 \
    | python3 -c "import json,sys; p=[x for x in json.load(sys.stdin)['packages'] if x['name']=='linq_rs'][0]; print(p.get('$1') or '')"
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
echo "=== sibling crate manifest (D-020) ==="
sib="$ROOT/linq_rs_sql/Cargo.toml"
if [ -f "$sib" ]; then
  sib_meta() {
    cargo metadata --no-deps --format-version 1 \
      | python3 -c "import json,sys; p=[x for x in json.load(sys.stdin)['packages'] if x['name']=='linq_rs_sql'][0]; print(p.get('$1') or '')"
  }
  sl="$(sib_meta license)"; sr="$(sib_meta repository)"
  echo "linq_rs_sql license = '${sl}', repository = '${sr}'"
  [ "$sl" = "MIT OR Apache-2.0" ] || err "linq_rs_sql license must match the workspace ('MIT OR Apache-2.0'), got '${sl}'"
  [ -n "$sr" ] || err "linq_rs_sql has no repository field"
  # It must stay dependency-free for the same reason its sibling does.
  # Normal deps only. `linq_rs` is a DEV-dependency so the seam's tests can
  # prove `.to_memory()` hands back something LinqExt works on; dev-deps never
  # enter a consumer's graph, so D-020's "neither depends on the other" holds
  # for anyone actually using either crate.
  sd="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c "import json,sys; p=[x for x in json.load(sys.stdin)['packages'] if x['name']=='linq_rs_sql'][0]; print(','.join(d['name'] for d in p['dependencies'] if d['kind'] is None))")"
  if [ -z "$sd" ]; then echo "linq_rs_sql runtime dependencies: none"; else err "linq_rs_sql gained a runtime dependency: ${sd}"; fi
else
  err "linq_rs_sql/Cargo.toml is missing — D-020 split the SQL builder into it"
fi

echo
echo "=== sibling crate tarball (D-012, D-020) ==="
# The root tarball is checked above. The sibling ships SEPARATELY and is a
# separate legal artifact: it declares `MIT OR Apache-2.0`, so it must carry
# both texts itself. It shipped neither until 2026-09-10 -- the licence check
# above reads the manifest FIELD, and a field is not a file.
sib_listing="$(cd "$ROOT" && cargo package --list --allow-dirty -p linq_rs_sql 2>/dev/null)"
[ -n "$sib_listing" ] || err "cargo package --list -p linq_rs_sql produced nothing"
while read -r want; do
  [ -z "$want" ] && continue
  if printf '%s\n' "$sib_listing" | grep -q "^${want}"; then
    echo "linq_rs_sql ships: ${want}"
  else
    err "linq_rs_sql's tarball is missing '${want}'"
  fi
done <<'SIBREQUIRED'
LICENSE-MIT
LICENSE-APACHE
README.md
tests/
SIBREQUIRED
for f in LICENSE-MIT LICENSE-APACHE; do
  if ! cmp -s "$ROOT/$f" "$ROOT/linq_rs_sql/$f"; then
    err "linq_rs_sql/${f} differs from the workspace ${f}"
  fi
done
echo "licence texts are byte-identical to the workspace copies"
# A relative link resolves in the git tree and 404s on crates.io and docs.rs,
# where the tarball is the whole world. Two flavours, both found live:
#   ../LICENSE-MIT   in the sibling  -- the tarball has no parent
#   linq_rs_sql/     in the root     -- workspace members are not in the root
#                                       tarball, so the target is simply absent
# Anchors (#...) are fine; so is anything the tarball actually ships.
check_relative_links() {
  local readme="$1" pkg="$2" listing="$3" bad=0 target
  while read -r target; do
    [ -z "$target" ] && continue
    case "$target" in \#*) continue ;; esac
    if [ "${target#../}" != "$target" ]; then
      echo "  ${pkg}/README.md -> ${target} (escapes the tarball)"; bad=1; continue
    fi
    printf '%s\n' "$listing" | grep -q "^${target%/}" \
      || { echo "  ${pkg}/README.md -> ${target} (not in the tarball)"; bad=1; }
  done < <(grep -oE '\]\([^)]+\)' "$readme" \
           | sed -E 's/^\]\(//; s/\)$//' | grep -vE '^[a-z]+:' | sort -u)
  [ "$bad" -eq 0 ] || err "${pkg}/README.md has links that break outside the git tree"
  echo "${pkg}/README.md: every relative link resolves inside the tarball"
}
check_relative_links "$ROOT/linq_rs_sql/README.md" linq_rs_sql "$sib_listing"
check_relative_links "$ROOT/README.md" linq_rs "$listing"

echo
echo "=== source invariants (D-001, D-003, D-004) ==="
# D-001: v1.0 is LINQ-to-objects only, and the crate advertises zero
# dependencies. Assert it rather than trusting it.
deps="$(cargo metadata --no-deps --format-version 1 \
  | python3 -c "import json,sys; p=[x for x in json.load(sys.stdin)['packages'] if x['name']=='linq_rs'][0]; print(','.join(d['name'] for d in p['dependencies']))")"
if [ -z "$deps" ]; then
  echo "dependencies: none (D-001)"
else
  err "crate has dependencies: ${deps}. D-001 keeps v1.0 dependency-free; a driver or SQL crate here would also breach D-002's staging."
fi

# D-003 / D-004: no interior mutability in the library. This is currently true
# by accident; the gate makes it true on purpose, so a later contributor cannot
# quietly reintroduce the Rc<RefCell<_>> identity map those decisions forbid.
if hits="$(grep -rnE 'Rc<|RefCell|Arc<|Mutex<|RwLock<' src/ 2>/dev/null)"; then
  err "interior mutability found in src/ — D-003 and D-004 forbid it:"
  printf '%s\n' "$hits"
else
  echo "no Rc/RefCell/Arc/Mutex/RwLock in src/ (D-003, D-004)"
fi

echo
[ "$fail" -eq 0 ] || { echo "packaging gate FAILED"; exit 1; }
echo "PASS"
