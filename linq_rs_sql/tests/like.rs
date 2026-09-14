//! `LIKE`, against expectations derived from a real SQLite.
//!
//! `D-034`: the in-memory matcher folds ASCII case so that `to_memory()` agrees
//! with SQLite's default `LIKE`. Before that it compared chars literally, and the
//! two interpreters disagreed on every case-varying pattern — a `to_memory` test
//! passed while the database returned different rows.
//!
//! **Every `bool` in `CORPUS` below was produced by asking SQLite**, not by
//! reasoning about the spec:
//!
//! ```text
//! SELECT ? LIKE ?     -- sqlite3 3.46.1, bound values
//! ```
//!
//! `.github/scripts/like-differential.py` re-derives them and fails if SQLite and
//! this table ever disagree. That script is the reason these expectations are not
//! self-referential: nothing here is checked against the implementation that
//! produced it. It uses python3's built-in `sqlite3` because `D-032` permits no
//! third-party crate anywhere in this workspace — including a dev-dependency of a
//! `publish = false` test crate, which would still land in the resolved graph.
//!
//! ASCII-only folding is deliberate. SQLite folds `EVE`/`eve` and does **not**
//! fold `É`/`é`; the non-ASCII rows below pin that.

use linq_rs_sql::prelude::*;

table! { t (id) { id -> Integer, s -> Text } }
pub struct T {
    pub id: i64,
    pub s: String,
}
entity! { T => t { id: Integer = id, s: Text = s } }

/// `(text, pattern, what SQLite answers)`. Derived, never hand-written.
const CORPUS: &[(&str, &str, bool)] = &[
    ("Eve", "eve", true),
    ("eve", "EVE", true),
    ("EVE", "Eve", true),
    ("Eve", "Eve", true),
    ("Eve", "evf", false),
    ("apple", "a%", true),
    ("apple", "A%", true),
    ("apple", "%E", true),
    ("apple", "%PL%", true),
    ("apple", "%", true),
    ("apple", "%%", true),
    ("apple", "a%e", true),
    ("apple", "A%E", true),
    ("apple", "%x%", false),
    ("Eve", "Ev_", true),
    ("Eve", "_ve", true),
    ("Eve", "_V_", true),
    ("Eve", "___", true),
    ("Eve", "____", false),
    ("Eve", "__", false),
    ("apple", "apple", true),
    ("apple", "APPLE", true),
    ("apple", "appl", false),
    ("apple", "applee", false),
    ("", "", true),
    ("", "%", true),
    ("", "_", false),
    ("a", "", false),
    ("aaa", "%a", true),
    ("aaa", "a%a", true),
    ("abcabc", "%abc", true),
    ("abcabc", "a%c", true),
    ("banana", "%na%na%", true),
    ("banana", "%NA%NA%", true),
    ("50%", "50%", true),
    ("50%", "5__", true),
    ("a_b", "a_b", true),
    ("a_b", "A_B", true),
    ("É", "é", false),
    ("é", "É", false),
    ("É", "É", true),
    ("Straße", "strasse", false),
    ("Straße", "STRAßE", true),
];

/// The in-memory interpreter must answer exactly what SQLite answers, for every
/// case in the corpus. This is the assertion the divergence broke.
#[test]
fn in_memory_like_matches_sqlite_on_every_corpus_case() {
    let mut wrong = Vec::new();
    for &(text, pattern, expected) in CORPUS {
        let rows = vec![T {
            id: 1,
            s: text.to_string(),
        }];
        let q = query::<T>().filter(t::s.like(pattern));
        let hit = q.to_memory(&rows).count() == 1;
        if hit != expected {
            wrong.push(format!(
                "{text:?} LIKE {pattern:?}: in-memory={hit} sqlite={expected}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} cases disagree with SQLite:\n  {}",
        wrong.len(),
        CORPUS.len(),
        wrong.join("\n  ")
    );
}

/// The specific cases the old matcher got wrong, called out so a regression is
/// legible rather than buried in a corpus count.
#[test]
fn ascii_case_is_folded_in_both_directions() {
    for (text, pattern) in [
        ("Eve", "eve"),
        ("eve", "EVE"),
        ("apple", "%PL%"),
        ("Eve", "_V_"),
    ] {
        let rows = vec![T {
            id: 1,
            s: text.to_string(),
        }];
        let q = query::<T>().filter(t::s.like(pattern));
        assert_eq!(
            q.to_memory(&rows).count(),
            1,
            "{text:?} LIKE {pattern:?} must match: SQLite folds ASCII case"
        );
    }
}

/// And the boundary: folding is ASCII-only, because SQLite's is.
#[test]
fn non_ascii_case_is_not_folded() {
    let rows = vec![T {
        id: 1,
        s: "É".to_string(),
    }];
    let q = query::<T>().filter(t::s.like("é"));
    assert_eq!(
        q.to_memory(&rows).count(),
        0,
        "SQLite does not fold non-ASCII case, so neither does this"
    );
}

/// The SQL side is unchanged — the fix was to the in-memory matcher only.
#[test]
fn the_emitted_sql_is_unchanged() {
    assert_eq!(
        query::<T>().filter(t::s.like("a%")).to_sql().sql,
        "SELECT id, s FROM t WHERE (s LIKE ?)"
    );
}
