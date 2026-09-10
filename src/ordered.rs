//! Ordered sequences produced by `order_by` / `order_by_descending` /
//! `order` / `order_descending` / `order_by_with`.
//!
//! Plays the role of C# `IOrderedEnumerable<T>`, with two differences: the
//! source is collected when the operator is *called* where C# defers
//! everything to the first `MoveNext`, and comparison is by `Ord`
//! (byte-ordinal for strings) where C# uses the culture-sensitive
//! `Comparer<T>.Default`.
//!
//! ## How chaining works
//!
//! `order_by` and friends collect the source into a `Vec<T>` and stash a
//! **comparator** alongside it without sorting yet. Each subsequent `then_by`
//! pushes another comparator. The sort happens once, on the first `next()`, by
//! walking the comparator stack and falling through on `Ordering::Equal` —
//! which is the only way to preserve the primary key's grouping while refining
//! within it. Sorting on the *secondary* key alone would discard the primary
//! ordering, which is precisely the bug this design replaced.
//!
//! `OrderedQueryable` is itself an [`Iterator`], so the whole of
//! [`LinqExt`](crate::LinqExt) is available directly on a sorted sequence and
//! no `.into_iter()` hop is needed:
//!
//! ```rust
//! use linq_rs::LinqExt;
//!
//! let names: Vec<&str> = vec![("b", 2), ("a", 2), ("c", 1)]
//!     .into_iter()
//!     .order_by(|t| t.1)
//!     .then_by(|t| t.0)
//!     .select(|t| t.0)
//!     .collect();
//! assert_eq!(names, ["c", "a", "b"]);
//! ```

use std::cmp::Ordering;

type Comparator<'a, T> = Box<dyn Fn(&T, &T) -> Ordering + 'a>;

/// A sequence that will be sorted by one or more keys on first iteration.
///
/// Produced by [`order_by`](crate::LinqExt::order_by),
/// [`order_by_descending`](crate::LinqExt::order_by_descending),
/// [`order`](crate::LinqExt::order), [`order_descending`](crate::LinqExt::order_descending)
/// and [`order_by_with`](crate::LinqExt::order_by_with).
///
/// Implements [`Iterator`], so every `LinqExt` operator works on it directly.
#[must_use = "this buffers the source when constructed and sorts on first next(); dropping it wastes both"]
pub struct OrderedQueryable<'a, T> {
    /// Unsorted until `next()` is first called; `None` afterwards.
    pending: Option<Vec<T>>,
    comparators: Vec<Comparator<'a, T>>,
    sorted: Option<std::vec::IntoIter<T>>,
}

impl<'a, T> OrderedQueryable<'a, T> {
    pub(crate) fn new(data: Vec<T>, first: Comparator<'a, T>) -> Self {
        Self {
            pending: Some(data),
            comparators: vec![first],
            sorted: None,
        }
    }

    fn push_comparator(mut self, c: Comparator<'a, T>) -> Self {
        // `assert!`, not `debug_assert!`. Once iteration has begun the buffer is
        // already sorted, so a comparator pushed afterwards is silently ignored
        // and the caller gets a plausible, wrongly-ordered answer -- in release
        // only, where it is hardest to find. Measured before this changed:
        // `order_by(|x| x.1)`, one `next()`, then `then_by(|x| x.0)` yielded
        // `[("b", 2), ("a", 2)]` with the secondary key discarded.
        //
        // This is a programming error, not a runtime condition, so it panics
        // rather than returning an error: there is no correct answer to give.
        // See D-025.
        assert!(
            self.pending.is_some(),
            "then_by/then_by_with cannot be called after iteration has begun: \
             the buffer is already sorted, so the added comparator would be \
             silently ignored"
        );
        self.comparators.push(c);
        self
    }

    /// Further sorts by an ascending secondary key, applied only where the
    /// existing keys compare equal.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<_> = vec![("b", 2), ("a", 2), ("c", 1)]
    ///     .into_iter()
    ///     .order_by(|t| t.1)
    ///     .then_by(|t| t.0)
    ///     .collect();
    /// assert_eq!(v, [("c", 1), ("a", 2), ("b", 2)]);
    /// ```
    pub fn then_by<K, F>(self, key_fn: F) -> Self
    where
        K: Ord,
        F: Fn(&T) -> K + 'a,
        T: 'a,
    {
        self.push_comparator(Box::new(move |a, b| key_fn(a).cmp(&key_fn(b))))
    }

    /// Further sorts by a descending secondary key.
    pub fn then_by_descending<K, F>(self, key_fn: F) -> Self
    where
        K: Ord,
        F: Fn(&T) -> K + 'a,
        T: 'a,
    {
        self.push_comparator(Box::new(move |a, b| key_fn(b).cmp(&key_fn(a))))
    }

    /// Further sorts with an explicit comparator, for keys that are `PartialOrd`
    /// but not `Ord` — floats, most often.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<_> = vec![("a", 2.5f64), ("b", 1.0), ("c", 2.5)]
    ///     .into_iter()
    ///     .order_by_with(|x, y| x.1.total_cmp(&y.1))
    ///     .then_by_with(|x, y| x.0.cmp(y.0))
    ///     .collect();
    /// assert_eq!(v, [("b", 1.0), ("a", 2.5), ("c", 2.5)]);
    /// ```
    pub fn then_by_with<F>(self, cmp: F) -> Self
    where
        F: Fn(&T, &T) -> Ordering + 'a,
        T: 'a,
    {
        self.push_comparator(Box::new(cmp))
    }

    fn ensure_sorted(&mut self) {
        if let Some(mut data) = self.pending.take() {
            let comparators = std::mem::take(&mut self.comparators);
            data.sort_by(|a, b| {
                for c in &comparators {
                    match c(a, b) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
                Ordering::Equal
            });
            self.sorted = Some(data.into_iter());
        }
    }
}

impl<T> Iterator for OrderedQueryable<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.ensure_sorted();
        self.sorted.as_mut().and_then(Iterator::next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = match (&self.pending, &self.sorted) {
            (Some(v), _) => v.len(),
            (None, Some(it)) => it.len(),
            (None, None) => 0,
        };
        (n, Some(n))
    }
}

impl<T> ExactSizeIterator for OrderedQueryable<'_, T> {}
impl<T> std::iter::FusedIterator for OrderedQueryable<'_, T> {}

impl<T> DoubleEndedIterator for OrderedQueryable<'_, T> {
    fn next_back(&mut self) -> Option<T> {
        self.ensure_sorted();
        self.sorted
            .as_mut()
            .and_then(DoubleEndedIterator::next_back)
    }
}
