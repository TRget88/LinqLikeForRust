//! [`Grouping`] — the result of a `group_by_key` operation.
//!
//! Plays the role of C# `IGrouping<TKey, TElement>`, but holds its elements
//! in a `Vec` rather than being a lazily-enumerable sequence.

/// A group of elements that share a common key.
///
/// Produced by [`LinqExt::group_by_key`](crate::queryable::LinqExt::group_by_key).
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Grouping<K, V> {
    key: K,
    elements: Vec<V>,
}

impl<K, V> Grouping<K, V> {
    pub(crate) fn new(key: K) -> Self {
        Self {
            key,
            elements: Vec::new(),
        }
    }

    /// The shared key for this group.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// All elements that belong to this group.
    pub fn elements(&self) -> &[V] {
        &self.elements
    }

    /// Consumes the group and returns its key and elements.
    ///
    /// The only way to take both out at once; the fields are private so that
    /// the one-group-per-key invariant cannot be broken from outside. (They
    /// used to be `pub` *alongside* these accessors, so a caller could split a
    /// group in two or empty it.)
    #[must_use]
    pub fn into_parts(self) -> (K, Vec<V>) {
        (self.key, self.elements)
    }

    /// Appends an element. In-crate only: the grouping operators build these.
    pub(crate) fn push(&mut self, value: V) {
        self.elements.push(value);
    }

    /// Consumes the group and returns an iterator over its elements.
    pub fn into_elements(self) -> impl Iterator<Item = V> {
        self.elements.into_iter()
    }
}
