//! Ordered sequences produced by `order_by` / `order_by_descending` /
//! `order` / `order_descending`.
//!
//! Mirrors `IOrderedEnumerable<T>` from C# LINQ.
//!
//! ## How chaining works
//!
//! `order_by` / `order_by_descending` / `order` / `order_descending` collect
//! the source into a `Vec<T>` and stash a **comparator** alongside it without
//! sorting yet. Each subsequent `then_by` / `then_by_descending` pushes
//! another comparator. Sorting happens once, at `into_iter` time, by walking
//! the comparator stack in order and falling through on `Ordering::Equal` —
//! this is the only way to preserve the primary key's grouping while
//! refining within it.

use std::cmp::Ordering;

type Comparator<T> = Box<dyn Fn(&T, &T) -> Ordering>;

/// A sequence whose elements have been collected and will be sorted by one
/// or more keys at iteration time.
///
/// Produced by [`order_by`](crate::LinqExt::order_by),
/// [`order_by_descending`](crate::LinqExt::order_by_descending),
/// [`order`](crate::LinqExt::order), and
/// [`order_descending`](crate::LinqExt::order_descending).
pub struct OrderedQueryable<T> {
    pub(crate) data: Vec<T>,
    pub(crate) comparators: Vec<Comparator<T>>,
}

impl<T> OrderedQueryable<T> {
    pub(crate) fn new(data: Vec<T>, first: Comparator<T>) -> Self {
        Self {
            data,
            comparators: vec![first],
        }
    }

    pub(crate) fn push_comparator(mut self, c: Comparator<T>) -> Self {
        self.comparators.push(c);
        self
    }
}

/// Extension trait adding `then_by` / `then_by_descending` to an
/// [`OrderedQueryable`].
pub trait ThenBy<T>: Sized {
    /// Further sorts an already-ordered sequence by an ascending secondary key.
    fn then_by<K, F>(self, key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: Fn(&T) -> K + 'static;

    /// Further sorts an already-ordered sequence by a descending secondary key.
    fn then_by_descending<K, F>(self, key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: Fn(&T) -> K + 'static;
}

impl<T: 'static> ThenBy<T> for OrderedQueryable<T> {
    fn then_by<K, F>(self, key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: Fn(&T) -> K + 'static,
    {
        self.push_comparator(Box::new(move |a, b| key_fn(a).cmp(&key_fn(b))))
    }

    fn then_by_descending<K, F>(self, key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: Fn(&T) -> K + 'static,
    {
        self.push_comparator(Box::new(move |a, b| key_fn(b).cmp(&key_fn(a))))
    }
}

impl<T> IntoIterator for OrderedQueryable<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        let Self {
            mut data,
            comparators,
        } = self;
        data.sort_by(|a, b| {
            for c in &comparators {
                match c(a, b) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            Ordering::Equal
        });
        data.into_iter()
    }
}
