#!/usr/bin/env bash
#
# Compile the driver adapter in `docs/DRIVER_ADAPTER.md`.
#
# WHY THIS EXISTS. `D-032` deleted `linq_rs_sqlite` and preserved its rusqlite glue
# as documentation. From then until this script, nothing built that code. It cannot
# be a doctest: compiling it needs `rusqlite`, and `D-032` permits no third-party
# dependency anywhere in this workspace -- including a dev-dependency of a
# `publish = false` crate, which would still appear in the resolved graph
# `packaging-gate.sh` checks.
#
# So the crate's central promise -- "the driver glue is yours to write, here it is"
# -- was the one piece of code in the project with no mechanical check at all. That
# is not hypothetical: settling `D-109` (`SqlValue` became `#[non_exhaustive]`) broke
# the adapter's `bind` helper, whose `match` had no wildcard arm, and nothing said
# so. It was caught by hand, afterwards.
#
# HOW. The stub below is a hand-written stand-in for the small slice of rusqlite the
# adapter touches, built as a throwaway crate *named* `rusqlite` in a temp dir -- the
# same shape `itertools-interop.sh` established. Naming it `rusqlite` is what lets
# the documented `use rusqlite::types::ValueRef;` compile **verbatim**; a local
# `mod rusqlite` would not, because a bare `use rusqlite::..` path resolves to a
# crate. Nothing is added to the workspace, so the resolved graph is untouched.
#
# WHAT THIS DOES AND DOES NOT PROVE.
#   * It DOES prove the adapter type-checks against `linq_rs_sql`'s real traits --
#     `ColumnSet`, `RowSource`, `FromRow`, `SqlValueRef`, `SqlValue`, `RowError` --
#     with their real signatures, lifetimes and exhaustiveness. That is the half
#     that drifts, because it is the half this repo changes.
#   * It does NOT prove the adapter compiles against rusqlite itself. The stub's
#     signatures are copied from rusqlite's public API, but if rusqlite changes one,
#     this gate keeps passing. Verifying that needs the dependency `D-032` forbids,
#     so it stays a documented limitation rather than a silent assumption.
#
# Run: ./.github/scripts/adapter-gate.sh
# Exit: 0 only if every fenced Rust block in the doc compiles.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$REPO/docs/DRIVER_ADAPTER.md"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "repo:    $REPO"
echo "doc:     $DOC"
echo "workdir: $WORK"
echo

[ -f "$DOC" ] || { echo "FAIL: $DOC does not exist"; exit 1; }

# ── Extract, and refuse to proceed on a bad extraction ──────────────────────────
# A gate whose input silently becomes empty reports success. `release-gate.sh` did
# exactly that, on this same repo, so the count is asserted rather than assumed.
python3 - "$DOC" "$WORK/extracted.rs" <<'PY'
import re, sys
doc, out = sys.argv[1], sys.argv[2]
src = open(doc, encoding="utf-8").read()
blocks = re.findall(r"```rust\n(.*?)```", src, re.S)
if not blocks:
    print("FAIL: no ```rust blocks found -- the doc's format changed and this "
          "extraction stopped seeing it", file=sys.stderr)
    sys.exit(1)
body = "\n".join(blocks)
# Sanity-check the extraction actually captured the adapter, not some other snippet.
need = ["impl ColumnSet for", "impl<'a, 's> RowSource<'a> for", "fn bind(", "fn fetch<"]
missing = [n for n in need if n not in body]
if missing:
    print(f"FAIL: extracted {len(blocks)} block(s) but they are missing: {missing}",
          file=sys.stderr)
    print("      Either the adapter changed shape or extraction is broken. Both "
          "must fail, not pass.", file=sys.stderr)
    sys.exit(1)
open(out, "w", encoding="utf-8").write(body)
print(f"extracted {len(blocks)} Rust block(s), {len(body.splitlines())} lines")
PY

# ── The stub crate, named `rusqlite` so the documented `use` compiles verbatim ───
# Signatures copied from rusqlite's public API. Deliberately no behaviour: every
# body is `unimplemented!()`, because this gate type-checks and never runs.
mkdir -p "$WORK/rusqlite/src"
cat > "$WORK/rusqlite/Cargo.toml" <<'EOF'
[package]
name = "rusqlite"
version = "0.0.0"
edition = "2021"
publish = false
EOF
cat > "$WORK/rusqlite/src/lib.rs" <<'EOF'
//! Hand-written stand-in for the slice of rusqlite the documented adapter touches.
//! Not rusqlite, not a reimplementation, and never published. Bodies are
//! `unimplemented!()` on purpose: the gate compiles this, it does not run it.
#![allow(clippy::all)]

use std::fmt;

/// Stands in for `rusqlite::Error`. The adapter passes this to
/// `RowError::driver`, whose bound is `Error + Send + Sync + 'static`, so the
/// stub must satisfy all three or the gate would pass code that cannot compile.
#[derive(Debug)]
pub struct Error;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stub rusqlite error")
    }
}
impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub mod types {
    /// `rusqlite::types::ValueRef`. Note `Text`/`Blob` lend BYTES, not `&str` --
    /// which is why the adapter runs `from_utf8` and maps the failure into a
    /// `RowError`. A stub that lent `&str` would quietly delete that step.
    #[derive(Debug, Clone, Copy)]
    pub enum ValueRef<'a> {
        Null,
        Integer(i64),
        Real(f64),
        Text(&'a [u8]),
        Blob(&'a [u8]),
    }
}

pub trait ToSql {}
impl ToSql for i64 {}
impl ToSql for f64 {}
impl ToSql for bool {}
impl ToSql for String {}
impl<T: ToSql> ToSql for Option<T> {}

pub struct Connection;
impl Connection {
    pub fn prepare(&self, _sql: &str) -> Result<Statement<'_>> {
        unimplemented!()
    }
}

pub struct Statement<'conn>(std::marker::PhantomData<&'conn ()>);
impl<'conn> Statement<'conn> {
    pub fn column_count(&self) -> usize {
        unimplemented!()
    }
    pub fn column_name(&self, _i: usize) -> Result<&str> {
        unimplemented!()
    }
    /// `&mut self`, as in rusqlite -- which is what makes the adapter's ordering
    /// significant: `Cols(&stmt)` must be done with before `query` borrows mutably.
    pub fn query<'s>(&'s mut self, _params: &[&dyn ToSql]) -> Result<Rows<'s>> {
        unimplemented!()
    }
}

pub struct Rows<'stmt>(std::marker::PhantomData<&'stmt ()>);
impl<'stmt> Rows<'stmt> {
    pub fn next(&mut self) -> Result<Option<&Row<'stmt>>> {
        unimplemented!()
    }
}

pub struct Row<'stmt>(std::marker::PhantomData<&'stmt ()>);
impl<'stmt> Row<'stmt> {
    /// Lends for the ROW's borrow, not the call's. This is the fact the adapter's
    /// comment turns on -- a TEXT column reaching a `&'a str` field with no
    /// allocation -- so the stub has to reproduce it or the gate proves nothing
    /// about the borrowing it claims to check.
    pub fn get_ref(&self, _i: usize) -> Result<types::ValueRef<'stmt>> {
        unimplemented!()
    }
}
impl<'stmt> AsRef<Statement<'stmt>> for Row<'stmt> {
    fn as_ref(&self) -> &Statement<'stmt> {
        unimplemented!()
    }
}
EOF

# ── The crate under test: the extracted code, unmodified ────────────────────────
mkdir -p "$WORK/adapter/src"
cat > "$WORK/adapter/Cargo.toml" <<EOF
[package]
name = "adapter_check"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
linq_rs_sql = { path = "$REPO/linq_rs_sql" }
rusqlite = { path = "$WORK/rusqlite" }
EOF

# `dead_code` only: nothing calls these, and the point is that they type-check.
# Everything else stays a hard error.
{
  echo '//! Generated by .github/scripts/adapter-gate.sh from docs/DRIVER_ADAPTER.md.'
  echo '//! Do not edit; edit the doc.'
  echo '#![allow(dead_code)]'
  echo '#![deny(warnings)]'
  echo
  cat "$WORK/extracted.rs"
} > "$WORK/adapter/src/lib.rs"

echo
echo "=== compiling the documented adapter ==="
if (cd "$WORK/adapter" && cargo build 2>&1); then
  status=0
else
  status=1
fi

echo
if [ "$status" -ne 0 ]; then
  cat <<'MSG'
FAIL: docs/DRIVER_ADAPTER.md does not compile.

That document is what the README hands a user when it says the driver glue is
theirs to write, so a compile error there is shipped guidance that does not work.
Fix the doc -- it is the source of truth; this script only reads it.

If the break came from a deliberate change in linq_rs_sql, the adapter is the
first downstream consumer to notice, which is the entire point of this gate.
MSG
  exit 1
fi

echo "PASS: every Rust block in docs/DRIVER_ADAPTER.md compiles against"
echo "      linq_rs_sql's real traits (rusqlite itself is stubbed -- see the"
echo "      header for what that does and does not prove)."
