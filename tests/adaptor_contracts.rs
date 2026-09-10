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

#[test]
fn chunk_does_not_allocate_from_its_argument() {
    // W-9. `Vec::with_capacity(size)` sized the allocation from the caller's
    // argument: chunk(2^28) over three u64s reserved 2 GiB, and chunk(2^40)
    // aborted the process -- an abort `catch_unwind` cannot catch.
    let huge = 1usize << 40;
    let batches: Vec<Vec<u64>> = vec![1u64, 2, 3].into_iter().chunk(huge).collect();

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0], [1, 2, 3]);
    assert!(
        batches[0].capacity() < 1 << 20,
        "capacity {} was taken from the argument, not the data",
        batches[0].capacity()
    );
}

#[test]
fn chunk_still_chunks_correctly() {
    let v: Vec<Vec<i32>> = (1..=7).chunk(3).collect();
    assert_eq!(v, [vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
    let exact: Vec<Vec<i32>> = (1..=6).chunk(2).collect();
    assert_eq!(exact, [vec![1, 2], vec![3, 4], vec![5, 6]]);
}

#[test]
#[should_panic(expected = "chunk size must be > 0")]
fn chunk_zero_panics_as_documented() {
    // The panic is intentional and now carries a `# Panics` section. C# throws
    // ArgumentOutOfRangeException here and `slice::chunks` panics.
    let _ = (1..=3).chunk(0);
}

// ── FusedIterator ────────────────────────────────────────────────────────────

/// Compile-time assertion: these adaptors are usable where `FusedIterator` is
/// required. Before this, no adaptor in the crate implemented it, so any
/// downstream API with a `FusedIterator` bound rejected every linq_rs query.
#[test]
fn adaptors_are_fused() {
    fn assert_fused<I: std::iter::FusedIterator>(_: I) {}

    let v = || vec![1, 2, 3].into_iter();
    assert_fused(v().where_(|x| *x > 1));
    assert_fused(v().select(|x| x * 2));
    assert_fused(v().skip_(1));
    assert_fused(v().take_(2));
    assert_fused(v().skip_while_(|x| *x < 2));
    assert_fused(v().take_while_(|x| *x < 3));
    // The `*_partial_eq` escape hatches return the named `Distinct` /
    // `DistinctBy` adaptors, so the impl is reachable.
    assert_fused(v().distinct_partial_eq());
    assert_fused(v().distinct_by_partial_eq(|x| *x % 2));
    assert_fused(v().concat_(vec![4, 5]));
    assert_fused(v().zip_(vec![9, 8, 7], |a, b| a + b));
    assert_fused(v().default_if_empty(0));
    assert_fused(v().skip_last(1));
    // Unconditionally fused, whatever the source does after `None`.
    assert_fused(v().reverse());
    assert_fused(v().chunk(2));
}

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
