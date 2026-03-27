//! [`Grouping`] — the result of a `group_by` operation.
//!
//! Mirrors `IGrouping<TKey, TElement>` from C# LINQ.

/// A group of elements that share a common key.
///
/// Produced by [`LinqExt::group_by`](crate::queryable::LinqExt::group_by).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grouping<K, V> {
    pub key: K,
    pub elements: Vec<V>,
}

impl<K, V> Grouping<K, V> {
    pub(crate) fn new(key: K) -> Self {
        Self { key, elements: Vec::new() }
    }

    /// The shared key for this group.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// All elements that belong to this group.
    pub fn elements(&self) -> &[V] {
        &self.elements
    }

    /// Consume the group and return an iterator over its elements.
    pub fn into_elements(self) -> impl Iterator<Item = V> {
        self.elements.into_iter()
    }
}
