//! Ordered sequences produced by `order_by` / `order_by_descending`.
//!
//! Mirrors `IOrderedEnumerable<T>` from C# LINQ.

/// A sequence whose elements have been collected and sorted by one or more keys.
///
/// Produced by [`order_by`](crate::LinqExt::order_by) and
/// [`order_by_descending`](crate::LinqExt::order_by_descending).
pub struct OrderedQueryable<T> {
    pub(crate) data: Vec<T>,
}

impl<T> OrderedQueryable<T> {
    pub(crate) fn new(data: Vec<T>) -> Self {
        Self { data }
    }
}

/// Extension trait adding `then_by` / `then_by_descending` to an
/// [`OrderedQueryable`].
pub trait ThenBy<T>: Sized {
    /// Further sorts an already-ordered sequence by an ascending secondary key.
    fn then_by<K, F>(self, key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: FnMut(&T) -> K;

    /// Further sorts an already-ordered sequence by a descending secondary key.
    fn then_by_descending<K, F>(self, key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: FnMut(&T) -> K;
}

impl<T> ThenBy<T> for OrderedQueryable<T> {
    fn then_by<K, F>(mut self, mut key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        // stable sort preserves the existing primary order
        self.data.sort_by(|a, b| key_fn(a).cmp(&key_fn(b)));
        self
    }

    fn then_by_descending<K, F>(mut self, mut key_fn: F) -> OrderedQueryable<T>
    where
        K: Ord,
        F: FnMut(&T) -> K,
    {
        self.data.sort_by(|a, b| key_fn(b).cmp(&key_fn(a)));
        self
    }
}

impl<T> IntoIterator for OrderedQueryable<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
