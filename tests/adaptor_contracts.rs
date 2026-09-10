//! Adaptor contracts that only surface when you drive the iterator by hand.
//!
//! Both bugs here were invisible to `collect()`, which is why they survived:
//! `collect()` stops at the first `None`, so a source that resumes afterwards
//! is never exercised, and a chunk's *capacity* is not something an equality
//! assertion looks at.

use linq_rs::LinqExt;

/// A source that returns `None` in the middle and then keeps going. Legal:
/// `Iterator` only promises `None` is final for `FusedIterator`.
struct Resuming {
    items: Vec<Option<i32>>,
    pos: usize,
}

impl Iterator for Resuming {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        let v = self.items.get(self.pos).copied().flatten();
        if self.pos < self.items.len() {
            self.pos += 1;
        }
        v
    }
}

fn resuming() -> Resuming {
    Resuming {
        items: vec![Some(10), Some(20), None, Some(40), Some(50), Some(60)],
        pos: 0,
    }
}

#[test]
fn skip_matches_std_on_a_non_fused_source() {
    // W-8. The old `skip_` decremented inside a loop guarded by `?`, so a
    // `None` mid-skip left `remaining > 0` and the next call skipped again --
    // eating element 40. std zeroes the counter with `mem::take` first.
    let mut theirs = Iterator::skip(resuming(), 3);
    let mut ours = resuming().skip_(3);

    let theirs: Vec<_> = (0..6).map(|_| theirs.next()).collect();
    let ours: Vec<_> = (0..6).map(|_| ours.next()).collect();

    assert_eq!(
        ours, theirs,
        "skip_ must agree with Iterator::skip on a non-fused source"
    );
    // Pin the actual value too, so a change to both does not pass silently.
    assert_eq!(ours, [None, Some(40), Some(50), Some(60), None, None]);
}

// ── FusedIterator ────────────────────────────────────────────────────────────

/// Compile-time assertion: these adaptors are usable where `FusedIterator` is
/// required. Before this, no adaptor in the crate implemented it, so any
/// downstream API with a `FusedIterator` bound rejected every linq_rs query.
/// The hash-backed defaults return `impl Iterator`, and `FusedIterator` is not
/// an auto trait, so it cannot leak through the opaque type however the body is
/// implemented. `assert_fused(v().distinct())` does not compile.
///
/// This is `AUDIT.md` finding **B-1** in miniature: return-position `impl
/// Trait` seals these operators against any trait a caller might later want,
/// and the fix is to return named types (`DECISIONS.md` `D-106`, still OPEN).
/// Recorded as a test comment rather than a `compile_fail` doctest so that
/// closing D-106 makes this note obsolete rather than making a test fail.
#[test]
fn rpitit_returns_cannot_be_fused() {
    // Compiles: the adaptor is usable, just not nameable or bound-able.
    let n = vec![1, 1, 2].into_iter().distinct().count();
    assert_eq!(n, 2);
}

// ── Bound relaxations (E-8, E-9) ─────────────────────────────────────────────

#[test]
fn select_many_accepts_any_into_iterator() {
    // E-9. `select_many` bound `J: Iterator` while `zip_` bound
    // `J: IntoIterator` -- an internal inconsistency that forced a stray
    // `.into_iter()` inside the closure for the direct C# translation.
    #[derive(Clone)]
    struct Person {
        orders: Vec<&'static str>,
    }
    let people = vec![
        Person {
            orders: vec!["a", "b"],
        },
        Person { orders: vec!["c"] },
    ];
    // No `.into_iter()` inside the closure.
    let all: Vec<&str> = people.into_iter().select_many(|p| p.orders).collect();
    assert_eq!(all, ["a", "b", "c"]);

    // Arrays and ranges work too.
    let nested: Vec<i32> = vec![1, 2]
        .into_iter()
        .select_many(|x| [x, x * 10])
        .collect();
    assert_eq!(nested, [1, 10, 2, 20]);
}

// ── Ordering and grouping API shape (W-13, W-14, W-15) ───────────────────────
