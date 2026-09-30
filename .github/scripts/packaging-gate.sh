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
  # Normal deps AND dev-deps: this crate must have neither. `linq_rs` used to be
  # a dev-dependency here, and D-024 moved those two files to `seam-tests` to
  # remove it -- because `cargo publish` strips a dev-dep's `path` but keeps its
  # `version`, which made `linq_rs_sql` unpublishable until `linq_rs 0.2.0` was
  # live. The comment here still described that dev-dependency as present until
  # 2026-09-29.
  # D-032, the owner's rule, stated verbatim:
  #   linq_rs     -- no dependencies.
  #   linq_rs_sql -- only linq_rs.
  #   nothing else is acceptable.
  #
  # So the whole dependency graph must contain no third-party crate at all, of
  # any kind, including dev. This is stricter than D-024 (which only demanded the
  # two core crates be clean) and it is what retired linq_rs_sqlite: one driver
  # dependency pulled 24 crates into the graph via rusqlite -> libsqlite3-sys.
  #
  # Checked on the RESOLVED graph, not the manifests: a manifest lists direct
  # dependencies, and a transitive one is still a dependency.
  third="$(cargo metadata --format-version 1 2>/dev/null \
    | python3 -c "
import json,sys
own={'linq_rs','linq_rs_sql','seam-tests'}
names={p['name'] for p in json.load(sys.stdin)['packages']} - own
print(','.join(sorted(names)))")"
  if [ -n "$third" ]; then
    err "third-party crates in the dependency graph: ${third}"
  fi
  echo "dependency graph contains no third-party crate"

  # And per-package, so a violation names the package that introduced it.
  # `linq_rs_sql` MAY depend on `linq_rs`; it is permitted, not required, and it
  # currently has none, which is stricter and fine. Anything else fails.
  for pkg in linq_rs linq_rs_sql; do
    got="$(cargo metadata --no-deps --format-version 1 \
      | python3 -c "import json,sys; p=[x for x in json.load(sys.stdin)['packages'] if x['name']=='${pkg}'][0]; print(','.join(sorted(set(d['name'] for d in p['dependencies']))))")"
    # Plain string comparison, not grep: the ALLOWED value is the empty string,
    # and `printf '%s' "" | grep -qE '^$'` fails because grep sees zero lines.
    ok=no
    case "${pkg}:${got}" in
      linq_rs:)                ok=yes ;;
      linq_rs_sql:|linq_rs_sql:linq_rs) ok=yes ;;
    esac
    if [ "$ok" != yes ]; then
      err "${pkg} may depend only on $( [ "$pkg" = linq_rs_sql ] && echo 'linq_rs' || echo 'nothing' ); got '${got:-<none>}'"
    fi
    echo "${pkg} dependencies: ${got:-none}"
  done

  np="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c "import json,sys; print(','.join(sorted(p['name'] for p in json.load(sys.stdin)['packages'] if p.get('publish') != [])))")"
  [ "$np" = "linq_rs,linq_rs_sql" ] \
    || err "publishable packages changed: expected linq_rs + linq_rs_sql, got: ${np}"
  echo "publishable packages: ${np} (seam-tests is publish = false)"
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
echo "=== the tarball's lockfile must parse on the declared MSRV (D-010, D-023) ==="
# `cargo package` ALWAYS writes a Cargo.lock into the tarball, generated by
# whatever toolchain publishes. So the lockfile that reaches a consumer is not
# the one in git -- untracking it does not help, it just hides the version from
# every check that looks at the repo. A v4 lockfile against `rust-version =
# "1.65"` produces an artifact that cannot build on its own declared MSRV:
#   lock file version `4` was found, but this version of Cargo does not
#   understand this lock file
# v3 is understood by Cargo 1.53+, comfortably below the 1.65 floor.
MAX_LOCKFILE_VERSION=3
check_tarball_lockfile() {
  local pkg="$1" ver lock
  lock="$(cd "$ROOT" && cargo package --list --allow-dirty -p "$pkg" 2>/dev/null | grep -c '^Cargo.lock$')"
  [ "$lock" = "1" ] || err "${pkg}'s tarball has no Cargo.lock (expected exactly one, got ${lock})"
  # Read the version from the lockfile cargo would ship, which is the workspace
  # lockfile -- cargo copies it rather than regenerating when it is up to date.
  ver="$(awk -F'= *' '/^version *=/ {gsub(/[^0-9]/,"",$2); print $2; exit}' "$ROOT/Cargo.lock")"
  [ -n "$ver" ] || err "could not read a version from $ROOT/Cargo.lock"
  echo "${pkg}: ships Cargo.lock at format v${ver} (max ${MAX_LOCKFILE_VERSION})"
  [ "$ver" -le "$MAX_LOCKFILE_VERSION" ] \
    || err "lockfile v${ver} cannot be parsed by Cargo $(manifest_field rust_version); regenerate at v${MAX_LOCKFILE_VERSION}"
}
check_tarball_lockfile linq_rs
check_tarball_lockfile linq_rs_sql
# The lockfile must also be TRACKED. Untracked, CI and any publisher regenerate
# it at their toolchain's default version and the check above passes while the
# tarball still ships v4.
(cd "$ROOT" && git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1) \
  || err "Cargo.lock is not tracked; an untracked lockfile is regenerated at publish time and defeats this check"
echo "Cargo.lock is tracked"

echo
echo "=== a published crate must not cite a file it does not ship (D-023) ==="
# linq_rs_sql referenced `DECISIONS.md` in three shipped files and shipped it
# zero times -- it lives at the workspace root, which no member tarball can
# reach. A bare filename gives the reader nothing to follow; a URL does.
# Take the file list from the TARBALL, not from git. `git ls-files` cannot see a
# newly-added file until it is staged, so a fresh source file citing a
# repo-root doc passed this check locally and failed on CI -- the gate was
# reading the repo when its entire subject is what ships. (D-023's own lesson,
# reappearing inside D-023's gate.)
# Generalized 2026-09-29. This checked ONE crate (linq_rs_sql) against ONE
# filename (DECISIONS.md), so the identical defect was live and unguarded in the
# ROOT crate: src/lookup.rs bare-cited `AUDIT.md` in a rendered `//!` doc on
# `pub mod lookup`, and AUDIT.md is in the root crate's `exclude`. "The same check
# for the sibling" was assumed and had never been true. Now: every published
# crate, every repo-root doc, derived rather than hardcoded.
bare=0
for pkg in linq_rs linq_rs_sql; do
  if [ "$pkg" = "linq_rs" ]; then
    pkg_dir="$ROOT"; pkg_listing="$listing"
  else
    pkg_dir="$ROOT/linq_rs_sql"; pkg_listing="$sib_listing"
  fi
  pkg_files="$(printf '%s\n' "$pkg_listing" | grep -E '^(src/.*\.rs|README\.md)$' || true)"
  for doc in $(cd "$ROOT" && ls *.md docs/*.md 2>/dev/null); do
    # Shipped by THIS tarball? Then a bare mention is followable. Compare against
    # the tarball listing, never the repo -- that is D-023's whole point.
    if printf '%s\n' "$pkg_listing" | grep -qxF "$doc"; then continue; fi
    docbase="${doc##*/}"
    esc="$(printf '%s' "$docbase" | sed 's/\./\\./g')"
    for f in $pkg_files; do
      [ -f "$pkg_dir/$f" ] || continue
      if grep -q "$esc" "$pkg_dir/$f" \
         && ! grep -q "LinqLikeForRust/blob/main/[^ )]*${esc}" "$pkg_dir/$f"; then
        echo "  ${pkg}/${f} cites ${docbase} with no URL, and ${pkg}'s tarball does not ship it"
        bare=1
      fi
    done
  done
done
if [ "$bare" -eq 0 ]; then
  echo "every citation of an unshipped doc carries a URL (both crates, all root docs)"
else
  # `err` accumulates rather than exits, so an unconditional success echo here
  # would print directly under its own FAIL line.
  err "a published crate cites a doc it does not ship, without a URL"
fi

echo
echo "=== D-109: SqlValue must stay #[non_exhaustive] ==="
# Settled as option (a): new variants are additive. That promise is carried by one
# attribute, and deleting it is silent -- the crate keeps compiling, every test
# keeps passing, and the breakage lands in a downstream `match` at the next
# release. So assert the attribute rather than trusting it.
#
# Read from the repo source, but only after the LISTING confirms the file ships.
# That split is deliberate: cargo NORMALIZES Cargo.toml and GENERATES its own
# Cargo.lock, which is why D-023 forbids checking those against the repo -- but it
# copies `.rs` files verbatim, so for a source-level assertion the repo copy and
# the shipped copy are the same bytes. If cargo ever rewrites source on package,
# this check has to extract the tarball instead.
val_rel="src/value.rs"
printf '%s\n' "$sib_listing" | grep -qxF "$val_rel" \
  || err "linq_rs_sql's tarball does not ship ${val_rel}; the D-109 check cannot see it"
if [ -f "$ROOT/linq_rs_sql/$val_rel" ]; then
  # The attribute must be on SqlValue SPECIFICALLY, not merely present in the file.
  #
  # Implementation: take the lines above `pub enum SqlValue {` up to the nearest
  # preceding blank line -- in rustfmt'd source that span is the item's own
  # attribute/doc block -- and require the attribute in it. Simple enough to read
  # in one pass, which a gate has to be. The first version of this check needed
  # careful tracing to trust and would have false-FAILED on a doc comment sitting
  # between the attribute and the enum.
  #
  # Four cases measured against this version:
  #   attribute on SqlValue                     -> PASS
  #   attribute deleted                          -> FAIL
  #   attribute present but on another item here -> FAIL  (`grep -q` says PASS)
  #   doc comment between attribute and enum     -> PASS  (old version FAILED)
  if sed -n '/^pub enum SqlValue {/q;p' "$ROOT/linq_rs_sql/$val_rel" \
       | tac | sed -n '/^[[:space:]]*$/q;p' | grep -q '^#\[non_exhaustive\]$'; then
    echo "SqlValue is #[non_exhaustive] (D-109)"
  else
    err "SqlValue has lost #[non_exhaustive]. D-109 settled that new variants are
      ADDITIVE, and that promise is this attribute. Removing it is a major bump:
      every downstream \`match\` on SqlValue silently becomes exhaustive again, and
      the next new variant breaks all of them. If the ruling is being reversed, change
      D-109 in DECISIONS.md and this check in the same commit."
  fi
else
  err "$ROOT/linq_rs_sql/$val_rel is missing -- the D-109 check cannot have run"
fi

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
# Both crates' src/, not just the root's. This scanned `src/` alone until
# 2026-09-29, so `linq_rs_sql` -- which holds the whole seam -- was never checked
# for the identity map D-003/D-004 forbid.
# Comment lines are excluded, or the gate fires on prose. `rows.rs` documents
# which types a field may deref to and legitimately names `Rc<str>` and
# `Arc<str>` in a doc comment; neither is interior mutability and neither is code.
# A real `Rc<RefCell<_>>` is never on a `//` line, so this filter cannot hide one.
# Verified by injecting `let _x: Rc<RefCell<u8>>;` into each crate and confirming
# the gate still fails.
# "Looked and found nothing" was byte-identical to "could not look": with `src/`
# absent the grep finds nothing, the `if` is false, and the gate printed success.
# Assert the directories exist before trusting a clean result.
for d in src linq_rs_sql/src; do
  [ -d "$ROOT/$d" ] || err "$d does not exist -- this scan cannot have looked at it"
done
if hits="$(grep -rnE 'Rc<|RefCell|Arc<|Mutex<|RwLock<' src/ linq_rs_sql/src/ 2>/dev/null \
            | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//')"; then
  err "interior mutability found in src/ — D-003 and D-004 forbid it:"
  printf '%s\n' "$hits"
else
  echo "no Rc/RefCell/Arc/Mutex/RwLock in src/ or linq_rs_sql/src/ (D-003, D-004)"
fi

echo
[ "$fail" -eq 0 ] || { echo "packaging gate FAILED"; exit 1; }
echo "PASS"
