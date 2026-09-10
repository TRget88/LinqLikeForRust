//! [`Lookup`] — a one-to-many keyed collection.
//!
//! Plays the role of C# `ILookup<TKey, TElement>`. Keys are `Eq + Hash`
//! rather than compared by an `IEqualityComparer`.
//!
//! Keys are `Eq + Hash + Clone` and lookups are O(1). The `Clone` bound buys
//! the index: groups are held in a `Vec` so that iteration order is
//! first-appearance order, and a `HashMap<K, usize>` points into it, so each
//! key is stored twice. For the key types this is built for — integers, `&str`,
//! `String` — that is cheap, and the alternative was `get()` being a linear scan
//! over every group, which is what this type did before (`AUDIT.md` P-1: 288x
//! slower than `HashMap::get` at 10,000 keys, in the one type whose entire
//! purpose is keyed random access).

use crate::grouping::Grouping;
use std::collections::HashMap;
use std::hash::Hash;

/// A keyed collection that maps each key to one or more values.
///
/// Produced by [`LinqExt::to_lookup`](crate::queryable::LinqExt::to_lookup).
///
/// Groups are kept in **first-appearance order**, and the values within each
/// group in source order. Key lookup is O(1).
///
/// ```rust
/// use linq_rs::LinqExt;
///
/// let data = vec![("a", 1), ("b", 2), ("a", 3)];
/// let lookup = data.into_iter().to_lookup(|(k, _)| *k);
///
/// assert_eq!(lookup.get(&"a"), &[("a", 1), ("a", 3)]);
/// assert_eq!(lookup.get(&"z"), &[]);          // missing key: empty slice
/// assert_eq!(lookup.len(), 2);
/// assert!(lookup.contains_key(&"b"));
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Lookup<K, V> {
    /// Groups in first-appearance order.
    groups: Vec<Grouping<K, V>>,
    /// Key -> position in `groups`.
    index: HashMap<K, usize>,
}

impl<K: Eq + Hash + Clone, V> Lookup<K, V> {
    /// Creates an empty lookup.
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Appends `value` to the group for `key`, creating the group if it is new.
    ///
    /// Public so that a `Lookup` can be built directly, not only via
    /// [`to_lookup`](crate::queryable::LinqExt::to_lookup). Before this was
    /// public, `Lookup::default()` produced a value that could never be filled.
    pub fn insert(&mut self, key: K, value: V) {
        match self.index.get(&key) {
            Some(&pos) => self.groups[pos].push(value),
            None => {
                let pos = self.groups.len();
                self.index.insert(key.clone(), pos);
                let mut g = Grouping::new(key);
                g.push(value);
                self.groups.push(g);
            }
        }
    }

    /// Number of distinct keys.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Returns `true` if there are no keys.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// All elements associated with `key`, or an empty slice if absent. O(1).
    #[must_use]
    pub fn get(&self, key: &K) -> &[V] {
        match self.index.get(key) {
            Some(&pos) => self.groups[pos].elements(),
            None => &[],
        }
    }

    /// Returns `true` if `key` exists. O(1).
    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// Iterate over all groups, in first-appearance order.
    pub fn groups(&self) -> impl Iterator<Item = &Grouping<K, V>> {
        self.groups.iter()
    }

    /// Consume the lookup and iterate over all groups, in first-appearance
    /// order.
    pub fn into_groups(self) -> impl Iterator<Item = Grouping<K, V>> {
        self.groups.into_iter()
    }
}

/// Two lookups are equal when they hold the same groups in the same order. The
/// `index` is derived state and is deliberately not compared — a `#[derive]`
/// would have compared it, and would also have demanded `K: Hash` for the
/// `HashMap` field even where the comparison itself does not need it.
impl<K: PartialEq, V: PartialEq> PartialEq for Lookup<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.groups == other.groups
    }
}

impl<K: Eq, V: Eq> Eq for Lookup<K, V> {}

impl<K: Eq + Hash + Clone, V> Default for Lookup<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Clone, V> FromIterator<(K, V)> for Lookup<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut lookup = Self::new();
        for (k, v) in iter {
            lookup.insert(k, v);
        }
        lookup
    }
}

impl<K, V> IntoIterator for Lookup<K, V> {
    type Item = Grouping<K, V>;
    type IntoIter = std::vec::IntoIter<Grouping<K, V>>;
    fn into_iter(self) -> Self::IntoIter {
        self.groups.into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a Lookup<K, V> {
    type Item = &'a Grouping<K, V>;
    type IntoIter = std::slice::Iter<'a, Grouping<K, V>>;
    fn into_iter(self) -> Self::IntoIter {
        self.groups.iter()
    }
}
