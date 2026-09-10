//! Interop regression tests — the enforcement gate for `DECISIONS.md` `D-005`.
//!
//! This file exists because of three separate defects that all had the same
//! shape: a `LinqExt` method name that broke code which never asked for it.
//!
//!   * `LinqExt::skip` collided with `Iterator::skip`, so merely importing the
//!     trait turned every unqualified `.skip(n)` in the module into `E0034` —
//!     including calls on iterators unrelated to this crate. (`AUDIT.md` E-3.)
//!   * `LinqExt::join` silently *shadowed* `Itertools::join`. Every method on
//!     `LinqExt` takes `self` by value, so it wins the by-value step of method
//!     resolution against a competitor taking `&mut self`, with **no ambiguity
//!     diagnostic at all** — a working `.join(", ")` became three unrelated
//!     errors that never mentioned `LinqExt`. (`AUDIT.md` E-1.)
//!   * `LinqExt::group_by` collided with `Itertools::group_by`, which still
//!     exists in itertools 0.15 as a deprecated alias. Both take `self` by
//!     value, so that one was a loud `E0034`. (`AUDIT.md` E-2.)
//!
//! The first is fixed by the `skip` → `skip_` rename; the other two by
//! `join` → `inner_join` and `group_by` → `group_by_key` (`W-12`).
//!
//! **These tests pass by compiling.** If a future rename reintroduces a
//! collision, this file stops building — which is the point. A test that only
//! checked values would not catch it, because the failure is at type-check time.
//!
//! Note what is *not* covered here: this file mimics the shape of a competing
//! extension trait rather than depending on the real `itertools`. The shapes are
//! taken from itertools 0.15 (`fn join(&mut self, sep: &str) -> String where
//! Self::Item: Display`, and `fn group_by(self, key: F)`), so the resolution
//! rules exercised are the real ones — but a test against the real crate would
//! be strictly better. See `D-005`.

use linq_rs::LinqExt;
use std::fmt::Display;

// ── A stand-in for `Itertools`, with itertools 0.15's exact receiver shapes ───

/// Mirrors `Itertools::join` — **borrowed** receiver. This is the dangerous
/// case: a by-value competitor wins against it silently, with no diagnostic.
trait ForeignBorrowed: Iterator {
    fn join(&mut self, sep: &str) -> String
    where
        Self::Item: Display,
    {
        let mut out = String::new();
        let mut first = true;
        // `&mut *self` rather than `self.by_ref()`: `by_ref` requires
        // `Self: Sized`, which would change the receiver shape this test exists
        // to exercise. `impl Iterator for &mut I` is `?Sized`, so this does not.
        for item in &mut *self {
            if !first {
                out.push_str(sep);
            }
            first = false;
            out.push_str(&item.to_string());
        }
        out
    }
}
impl<I: Iterator> ForeignBorrowed for I {}

/// Mirrors `Itertools::group_by` — **by-value** receiver. Two by-value
/// candidates tie at the same resolution step, which surfaces as `E0034`.
trait ForeignByValue: Iterator + Sized {
    fn group_by<K, F>(self, mut key: F) -> Vec<(K, Vec<Self::Item>)>
    where
        F: FnMut(&Self::Item) -> K,
        K: PartialEq,
    {
        let mut out: Vec<(K, Vec<Self::Item>)> = Vec::new();
        for item in self {
            let k = key(&item);
            match out.iter_mut().find(|(ek, _)| *ek == k) {
                Some((_, v)) => v.push(item),
                None => out.push((k, vec![item])),
            }
        }
        out
    }
}
impl<I: Iterator> ForeignByValue for I {}

// ── The gate ─────────────────────────────────────────────────────────────────

#[test]
fn foreign_borrowed_join_is_not_shadowed() {
    // Both traits in scope. Before W-12 this failed to compile with three
    // unrelated errors, none of which named LinqExt.
    let joined = vec![1, 2, 3].into_iter().join("-");
    assert_eq!(joined, "1-2-3");

    // And LinqExt's own operator is still reachable, under its new name.
    let people = vec![(1u32, "Alice"), (2, "Bob")];
    let orders = vec![(1u32, "Laptop"), (2, "Keyboard")];
    let mut rows: Vec<String> = people
        .into_iter()
        .inner_join(
            orders,
            |(id, _)| *id,
            |(id, _)| *id,
            |(_, name), (_, item)| format!("{name}:{item}"),
        )
        .collect();
    rows.sort();
    assert_eq!(rows, ["Alice:Laptop", "Bob:Keyboard"]);
}

#[test]
fn foreign_by_value_group_by_is_not_ambiguous() {
    // Before W-12 this was E0034: multiple applicable items in scope.
    let grouped = vec![1, 2, 3, 4].into_iter().group_by(|x| x % 2);
    assert_eq!(grouped.len(), 2);

    // LinqExt's own grouping, under its new name.
    let groups: Vec<_> = vec!["apple", "ant", "bear"]
        .into_iter()
        .group_by_key(|w| w.chars().next().unwrap())
        .collect();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].key, 'a');
    assert_eq!(groups[0].elements, ["apple", "ant"]);
}

#[test]
fn importing_linqext_does_not_break_unrelated_iterator_calls() {
    // Every one of these is a std method on an iterator that has nothing to do
    // with this crate. Importing LinqExt used to make `.skip(n)` an E0034 here.
    assert_eq!("hello".chars().skip(1).collect::<String>(), "ello");
    let m: std::collections::BTreeMap<i32, i32> = [(1, 10), (2, 20), (3, 30)].into();
    assert_eq!(m.values().skip(2).copied().collect::<Vec<_>>(), vec![30]);
    assert_eq!((1..=5).skip(3).collect::<Vec<_>>(), vec![4, 5]);

    // And LinqExt's own paging operator, under its suffixed name.
    assert_eq!((1..=5).skip_(3).collect::<Vec<_>>(), vec![4, 5]);
}

#[test]
fn inherent_slice_methods_still_win_on_a_vec() {
    // `[T]::join`, `[T]::reverse` and `[T]::to_vec` are inherent methods on
    // slices. A Vec is not an Iterator, so LinqExt was never a candidate --
    // but these are the names most likely to be confused, so pin them.
    // A genuine Vec, not an array: Vec is the receiver most likely to be
    // confused with an iterator, so it is the one worth pinning. Built by
    // collect() so clippy::useless_vec does not fire on an unused `vec!`.
    let words: Vec<&str> = ["a", "b", "c"].into_iter().collect();
    assert_eq!(words.join("-"), "a-b-c");

    let mut nums = vec![1, 2, 3];
    nums.reverse();
    assert_eq!(nums, [3, 2, 1]);
    assert_eq!(nums.to_vec(), [3, 2, 1]);

    // LinqExt's iterator-level equivalents, for contrast.
    assert_eq!(vec![1, 2, 3].into_iter().reverse().to_vec(), [3, 2, 1]);
}
