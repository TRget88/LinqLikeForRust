//! [`Lookup`] — a one-to-many dictionary.
//!
//! Mirrors `ILookup<TKey, TElement>` from C# LINQ.

use crate::grouping::Grouping;

/// A keyed collection that maps each key to one or more values.
///
/// Produced by [`LinqExt::to_lookup`](crate::queryable::LinqExt::to_lookup).
#[derive(Debug, Clone)]
pub struct Lookup<K, V> {
    /// Groups stored in insertion order.
    groups: Vec<Grouping<K, V>>,
}

impl<K: PartialEq, V> Lookup<K, V> {
    pub(crate) fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.key == key) {
            g.elements.push(value);
        } else {
            let mut g = Grouping::new(key);
            g.elements.push(value);
            self.groups.push(g);
        }
    }

    /// Number of distinct keys.
    pub fn count(&self) -> usize {
        self.groups.len()
    }

    /// Returns a slice of all elements associated with `key`, or an empty
    /// slice if the key does not exist.
    pub fn get(&self, key: &K) -> &[V] {
        self.groups
            .iter()
            .find(|g| &g.key == key)
            .map(|g| g.elements.as_slice())
            .unwrap_or(&[])
    }

    /// Returns `true` if `key` exists in the lookup.
    pub fn contains_key(&self, key: &K) -> bool {
        self.groups.iter().any(|g| &g.key == key)
    }

    /// Iterate over all groups.
    pub fn groups(&self) -> impl Iterator<Item = &Grouping<K, V>> {
        self.groups.iter()
    }

    /// Consume the lookup and iterate over all groups.
    pub fn into_groups(self) -> impl Iterator<Item = Grouping<K, V>> {
        self.groups.into_iter()
    }
}

impl<K: PartialEq, V> Default for Lookup<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
