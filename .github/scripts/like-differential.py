#!/usr/bin/env python3
"""Differential check: `linq_rs_sql`'s in-memory LIKE vs a real SQLite.

`D-034` says the in-memory matcher folds ASCII case so that `to_memory()` agrees
with SQLite's default `LIKE`. That is a claim about two independent
implementations, so the only honest test is to run both and compare.

The oracle is **python3's built-in `sqlite3`**, not a Rust crate. `D-032` permits
no third-party dependency anywhere in this workspace, including a dev-dependency
of a `publish = false` test crate -- it would still appear in the resolved
dependency graph that `packaging-gate.sh` checks. So the corpus lives here, SQLite
answers it here, and the Rust side asserts the same corpus in
`linq_rs_sql/tests/like.rs`. This script's job is to prove the two corpora agree,
which is what makes the Rust test's expectations trustworthy rather than
self-referential.

Run: `python3 .github/scripts/like-differential.py`
Exit 0 if SQLite agrees with every expectation the Rust tests assert.
"""

import sqlite3
import sys
import re
from pathlib import Path

RUST_TEST = Path(__file__).resolve().parents[2] / "linq_rs_sql" / "tests" / "like.rs"

# (text, pattern) pairs. Chosen to cover: ASCII case in both directions, the two
# wildcards alone and combined, anchoring, the empty pattern and empty text, a
# literal `%`/`_` in the TEXT, non-ASCII (which SQLite does NOT fold), and
# backtracking cases where a `%` must give ground.
CORPUS = [
    # plain ASCII case, both directions
    ("Eve", "eve"), ("eve", "EVE"), ("EVE", "Eve"), ("Eve", "Eve"),
    ("Eve", "evf"),
    # % wildcard
    ("apple", "a%"), ("apple", "A%"), ("apple", "%E"), ("apple", "%PL%"),
    ("apple", "%"), ("apple", "%%"), ("apple", "a%e"), ("apple", "A%E"),
    ("apple", "%x%"),
    # _ wildcard
    ("Eve", "Ev_"), ("Eve", "_ve"), ("Eve", "_V_"), ("Eve", "___"),
    ("Eve", "____"), ("Eve", "__"),
    # anchoring / exactness
    ("apple", "apple"), ("apple", "APPLE"), ("apple", "appl"), ("apple", "applee"),
    # empties
    ("", ""), ("", "%"), ("", "_"), ("a", ""),
    # backtracking: the % must yield
    ("aaa", "%a"), ("aaa", "a%a"), ("abcabc", "%abc"), ("abcabc", "a%c"),
    ("banana", "%na%na%"), ("banana", "%NA%NA%"),
    # a literal % or _ in the TEXT (not the pattern)
    ("50%", "50%"), ("50%", "5__"), ("a_b", "a_b"), ("a_b", "A_B"),
    # non-ASCII: SQLite's default LIKE does NOT case-fold these
    ("É", "é"), ("é", "É"), ("É", "É"), ("Straße", "strasse"),
    ("Straße", "STRAßE"),
]


def sqlite_says(text: str, pattern: str) -> bool:
    con = sqlite3.connect(":memory:")
    # Ask SQLite directly; no table needed. Values are bound, never interpolated,
    # so a `%` or quote in the corpus cannot change the statement.
    (got,) = con.execute("SELECT ? LIKE ?", (text, pattern)).fetchone()
    return bool(got)


def rust_expectations() -> dict:
    """Parse the expectation table out of the Rust test, so the two cannot drift.

    The Rust file is the source of truth for what the library asserts; this
    script's contribution is checking those assertions against SQLite.
    """
    if not RUST_TEST.exists():
        print(f"FAIL: {RUST_TEST} does not exist", file=sys.stderr)
        sys.exit(1)
    src = RUST_TEST.read_text()
    m = re.search(r"const CORPUS: &\[\(&str, &str, bool\)\] = &\[(.*?)\];", src, re.S)
    if not m:
        print("FAIL: could not find `const CORPUS` in the Rust test", file=sys.stderr)
        sys.exit(1)
    out = {}
    for text, pattern, expect in re.findall(
        r'\(\s*"((?:[^"\\]|\\.)*)"\s*,\s*"((?:[^"\\]|\\.)*)"\s*,\s*(true|false)\s*\)',
        m.group(1),
    ):
        # NOT `unicode_escape`: that decodes bytes as latin-1, so a UTF-8 `É`
        # comes back as `Ã‰` and every non-ASCII case mismatches. Handle only the
        # escapes a Rust string literal can contain, and leave UTF-8 alone.
        def unescape(v: str) -> str:
            return (
                v.replace(r"\\", "\x00ESC\x00")
                .replace(r"\"", '"')
                .replace(r"\n", "\n")
                .replace(r"\t", "\t")
                .replace("\x00ESC\x00", "\\")
            )

        out[(unescape(text), unescape(pattern))] = expect == "true"
    return out


def main() -> int:
    expectations = rust_expectations()
    print(f"corpus in the Rust test: {len(expectations)} cases")

    missing = [c for c in CORPUS if c not in expectations]
    extra = [c for c in expectations if c not in CORPUS]
    bad = 0

    for (text, pattern), expected in sorted(expectations.items()):
        actual = sqlite_says(text, pattern)
        if actual != expected:
            print(
                f"  MISMATCH  {text!r} LIKE {pattern!r}: "
                f"SQLite={actual}  Rust test asserts={expected}"
            )
            bad += 1

    if missing:
        print(f"  {len(missing)} case(s) in this script are absent from the Rust test:")
        for c in missing:
            print(f"    {c[0]!r} LIKE {c[1]!r}")
        bad += len(missing)
    if extra:
        # Not a failure: the Rust test may cover cases SQLite cannot express.
        # But say so, rather than silently checking a subset.
        print(f"  note: {len(extra)} case(s) asserted in Rust are not in this script's corpus")

    print()
    if bad:
        print(f"LIKE differential FAILED: {bad} problem(s)")
        return 1
    print(f"PASS: SQLite agrees with all {len(expectations)} expectations the Rust tests assert")
    return 0


if __name__ == "__main__":
    sys.exit(main())
