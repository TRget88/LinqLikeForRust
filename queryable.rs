//! The [`LinqExt`] extension trait — brings LINQ methods to every `Iterator`.

use crate::adaptors::*;
use crate::grouping::Grouping;
use crate::lookup::Lookup;
use crate::ordered::OrderedQueryable;

/// Extends every `Iterator` with C# LINQ–style query operations.
///
/// Import this trait to unlock all methods:
///
/// ```rust
/// use linq_rs::LinqExt;
/// ```
pub trait LinqExt: Iterator + Sized {
    // ═══════════════════════════════════════════════════════════════════════
    // FILTERING
    // ═══════════════════════════════════════════════════════════════════════

    /// Filters elements by a predicate. Equivalent to `Where` in C# LINQ.
    ///
    /// Named `where_` to avoid the Rust keyword `where`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let evens: Vec<_> = (1..=10).where_(|x| x % 2 == 0).collect();
    /// assert_eq!(evens, [2, 4, 6, 8, 10]);
    /// ```
    fn where_<P>(self, predicate: P) -> Where<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        Where { inner: self, predicate }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PROJECTION
    // ═══════════════════════════════════════════════════════════════════════

    /// Projects each element into a new form. Equivalent to `Select`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let squares: Vec<_> = (1..=5).select(|x| x * x).collect();
    /// assert_eq!(squares, [1, 4, 9, 16, 25]);
    /// ```
    fn select<B, F>(self, f: F) -> Select<Self, F>
    where
        F: FnMut(Self::Item) -> B,
    {
        Select { inner: self, f }
    }

    /// Projects each element to an iterator and flattens the results.
    /// Equivalent to `SelectMany`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let flat: Vec<_> = vec![vec![1, 2], vec![3, 4]]
    ///     .into_iter()
    ///     .select_many(|v| v.into_iter())
    ///     .collect();
    /// assert_eq!(flat, [1, 2, 3, 4]);
    /// ```
    fn select_many<J, F>(self, f: F) -> SelectMany<Self, F, J>
    where
        F: FnMut(Self::Item) -> J,
        J: Iterator,
    {
        SelectMany { outer: self, f, current: None }
    }

    /// Flattens one level of nesting. Equivalent to `SelectMany(x => x)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let flat: Vec<_> = vec![vec![1, 2], vec![3]].into_iter().flatten_().collect();
    /// assert_eq!(flat, [1, 2, 3]);
    /// ```
    fn flatten_(self) -> Flatten<Self>
    where
        Self::Item: IntoIterator,
    {
        Flatten { outer: self, current: None }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PAGING / SLICING
    // ═══════════════════════════════════════════════════════════════════════

    /// Skips the first `n` elements. Equivalent to `Skip`.
    fn skip(self, n: usize) -> Skip<Self> {
        Skip { inner: self, remaining: n }
    }

    /// Skips elements while the predicate holds. Equivalent to `SkipWhile`.
    fn skip_while_<P>(self, predicate: P) -> SkipWhile<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        SkipWhile { inner: self, predicate, done_skipping: false }
    }

    /// Takes at most `n` elements. Equivalent to `Take`.
    fn take_(self, n: usize) -> Take<Self> {
        Take { inner: self, remaining: n }
    }

    /// Takes elements while the predicate holds. Equivalent to `TakeWhile`.
    fn take_while_<P>(self, predicate: P) -> TakeWhile<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        TakeWhile { inner: self, predicate, done: false }
    }

    /// Splits the sequence into fixed-size chunks. Equivalent to `Chunk`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let chunks: Vec<_> = (1..=7).chunk(3).collect();
    /// assert_eq!(chunks, [vec![1,2,3], vec![4,5,6], vec![7]]);
    /// ```
    fn chunk(self, size: usize) -> Chunk<Self> {
        assert!(size > 0, "chunk size must be > 0");
        Chunk { inner: self, size, done: false }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SET OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns distinct elements. Equivalent to `Distinct`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let d: Vec<_> = vec![1, 2, 2, 3, 1].into_iter().distinct().collect();
    /// assert_eq!(d, [1, 2, 3]);
    /// ```
    fn distinct(self) -> Distinct<Self>
    where
        Self::Item: PartialEq + Clone,
    {
        Distinct { inner: self, seen: Vec::new() }
    }

    /// Returns distinct elements by a key selector. Equivalent to `DistinctBy`.
    fn distinct_by<K, F>(self, key_fn: F) -> DistinctBy<Self, F, K>
    where
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        DistinctBy { inner: self, key_fn, seen_keys: Vec::new() }
    }

    /// Concatenates two sequences. Equivalent to `Concat`.
    fn concat_<I2>(self, other: I2) -> Concat<Self>
    where
        I2: IntoIterator<Item = Self::Item, IntoIter = Self>,
    {
        Concat { first: self, second: other.into_iter(), on_second: false }
    }

    /// Returns elements of `self` that are not in `other`. Equivalent to `Except`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let diff: Vec<_> = vec![1,2,3,4].into_iter().except(vec![2,4]).collect();
    /// assert_eq!(diff, [1, 3]);
    /// ```
    fn except<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq,
    {
        let exclusions: Vec<_> = other.into_iter().collect();
        self.where_(move |x| !exclusions.contains(x))
    }

    /// Returns elements that appear in both sequences. Equivalent to `Intersect`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let inter: Vec<_> = vec![1,2,3,4].into_iter().intersect(vec![2,4,6]).collect();
    /// assert_eq!(inter, [2, 4]);
    /// ```
    fn intersect<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq,
    {
        let inclusion: Vec<_> = other.into_iter().collect();
        self.where_(move |x| inclusion.contains(x))
    }

    /// Produces the set union of two sequences. Equivalent to `Union`.
    fn union_<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq + Clone,
    {
        let combined: Vec<_> = self.collect();
        let mut result = combined.clone();
        for item in other {
            if !result.contains(&item) {
                result.push(item);
            }
        }
        result.into_iter()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ORDERING
    // ═══════════════════════════════════════════════════════════════════════

    /// Sorts the sequence by a key in ascending order. Equivalent to `OrderBy`.
    ///
    /// Returns an [`OrderedQueryable`] that supports `.then_by()`.
    ///
    /// ```rust
    /// use linq_rs::{LinqExt, ThenBy};
    /// let sorted: Vec<_> = vec!["banana","apple","cherry"]
    ///     .into_iter()
    ///     .order_by(|s| *s)
    ///     .into_iter()
    ///     .collect();
    /// assert_eq!(sorted, ["apple", "banana", "cherry"]);
    /// ```
    fn order_by<K, F>(self, mut key_fn: F) -> OrderedQueryable<Self::Item>
    where
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        let mut data: Vec<_> = self.collect();
        data.sort_by(|a, b| key_fn(a).cmp(&key_fn(b)));
        OrderedQueryable::new(data)
    }

    /// Sorts in descending order. Equivalent to `OrderByDescending`.
    fn order_by_descending<K, F>(self, mut key_fn: F) -> OrderedQueryable<Self::Item>
    where
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        let mut data: Vec<_> = self.collect();
        data.sort_by(|a, b| key_fn(b).cmp(&key_fn(a)));
        OrderedQueryable::new(data)
    }

    /// Reverses the sequence. Equivalent to `Reverse`.
    fn reverse(self) -> Reverse<Self> {
        let buffer: Vec<_> = self.collect();
        Reverse { buffer: buffer.into_iter().rev() }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // AGGREGATION
    // ═══════════════════════════════════════════════════════════════════════

    /// Applies an accumulator function over the sequence with an explicit seed.
    /// Equivalent to `Aggregate` / `Aggregate(seed, func)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let sum = (1..=5).aggregate(0, |acc, x| acc + x);
    /// assert_eq!(sum, 15);
    /// ```
    fn aggregate<Acc, F>(self, seed: Acc, f: F) -> Acc
    where
        F: FnMut(Acc, Self::Item) -> Acc,
    {
        self.fold(seed, f)
    }

    /// Sums elements that implement `std::iter::Sum`. Equivalent to `Sum`.
    fn sum_<S>(self) -> S
    where
        S: std::iter::Sum<Self::Item>,
    {
        self.sum()
    }

    /// Counts elements matching an optional predicate.
    /// `count()` with no argument counts all; `count_where(p)` counts matching.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// assert_eq!((1..=10).count_where(|x| x % 2 == 0), 5);
    /// ```
    fn count_where<P>(self, predicate: P) -> usize
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.where_(predicate).count()
    }

    /// Returns the minimum element, or `None` if the iterator is empty.
    fn min_<>(self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.min()
    }

    /// Returns the maximum element, or `None` if the iterator is empty.
    fn max_<>(self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.max()
    }

    /// Returns the minimum element by a key selector.
    fn min_by_key_<K, F>(self, key_fn: F) -> Option<Self::Item>
    where
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        self.min_by_key(key_fn)
    }

    /// Returns the maximum element by a key selector.
    fn max_by_key_<K, F>(self, key_fn: F) -> Option<Self::Item>
    where
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        self.max_by_key(key_fn)
    }

    /// Computes the average of a sequence mapped to `f64`. Equivalent to `Average`.
    ///
    /// Returns `None` if the iterator is empty.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let avg = vec![1.0f64, 2.0, 3.0].into_iter().average(|x| x);
    /// assert_eq!(avg, Some(2.0));
    /// ```
    fn average<F>(self, selector: F) -> Option<f64>
    where
        F: FnMut(Self::Item) -> f64,
    {
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for val in self.select(selector) {
            sum += val;
            count += 1;
        }
        if count == 0 { None } else { Some(sum / count as f64) }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ELEMENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns the first element, or `None`. Equivalent to `FirstOrDefault`.
    fn first_or_default(mut self) -> Option<Self::Item> {
        self.next()
    }

    /// Returns the first element matching a predicate, or `None`.
    fn first_where<P>(self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.where_(predicate).next()
    }

    /// Returns the last element, or `None`. Equivalent to `LastOrDefault`.
    fn last_or_default(self) -> Option<Self::Item> {
        self.fold(None, |_, x| Some(x))
    }

    /// Returns the last element matching a predicate, or `None`.
    fn last_where<P>(self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.where_(predicate).fold(None, |_, x| Some(x))
    }

    /// Returns the element at `index`, or `None`. Equivalent to `ElementAtOrDefault`.
    fn element_at(self, index: usize) -> Option<Self::Item> {
        self.skip(index).next()
    }

    /// Returns the single element, or panics / returns `None` if there is not
    /// exactly one. Equivalent to `SingleOrDefault`.
    fn single_or_default(mut self) -> Option<Self::Item> {
        let first = self.next()?;
        if self.next().is_some() { None } else { Some(first) }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QUANTIFIERS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns `true` if any element satisfies the predicate. Equivalent to `Any`.
    fn any_<P>(self, predicate: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        self.any(predicate)
    }

    /// Returns `true` if every element satisfies the predicate. Equivalent to `All`.
    fn all_<P>(self, predicate: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        self.all(predicate)
    }

    /// Returns `true` if the sequence contains a specific value. Equivalent to `Contains`.
    fn contains_<T>(self, value: &T) -> bool
    where
        Self::Item: PartialEq<T>,
    {
        self.any(|x| x == *value)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // JOINING
    // ═══════════════════════════════════════════════════════════════════════

    /// Performs an inner join between `self` and `inner` on matching keys,
    /// projecting results with `result_selector`.
    ///
    /// Equivalent to `Join(inner, outerKey, innerKey, resultSelector)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    ///
    /// let people = vec![(1u32, "Alice"), (2, "Bob")];
    /// let orders = vec![(1u32, "Order A"), (1, "Order B"), (2, "Order C")];
    ///
    /// let mut results: Vec<_> = people.into_iter()
    ///     .join(
    ///         orders,
    ///         |(id, _)| *id,
    ///         |(id, _)| *id,
    ///         |(_, name), (_, order)| format!("{name}: {order}"),
    ///     )
    ///     .collect();
    /// results.sort();
    /// assert_eq!(results, ["Alice: Order A", "Alice: Order B", "Bob: Order C"]);
    /// ```
    fn join<Inner, OuterKey, InnerKey, R, OuterKeyFn, InnerKeyFn, ResultFn>(
        self,
        inner: Inner,
        outer_key_fn: OuterKeyFn,
        inner_key_fn: InnerKeyFn,
        result_selector: ResultFn,
    ) -> impl Iterator<Item = R>
    where
        Inner: IntoIterator,
        OuterKey: PartialEq,
        InnerKey: PartialEq<OuterKey>,
        OuterKeyFn: Fn(&Self::Item) -> OuterKey,
        InnerKeyFn: Fn(&Inner::Item) -> InnerKey,
        ResultFn: Fn(Self::Item, Inner::Item) -> R,
        Self::Item: Clone,
        Inner::Item: Clone,
    {
        let inner_vec: Vec<Inner::Item> = inner.into_iter().collect();
        let outer_vec: Vec<Self::Item> = self.collect();
        let mut results = Vec::new();
        for outer_item in outer_vec {
            let outer_key = outer_key_fn(&outer_item);
            for inner_item in &inner_vec {
                if inner_key_fn(inner_item) == outer_key {
                    results.push(result_selector(outer_item.clone(), inner_item.clone()));
                }
            }
        }
        results.into_iter()
    }

    /// Performs a group join (left outer join with grouped inner elements).
    /// Equivalent to `GroupJoin`.
    fn group_join<Inner, OuterKey, InnerKey, R, OuterKeyFn, InnerKeyFn, ResultFn>(
        self,
        inner: Inner,
        outer_key_fn: OuterKeyFn,
        inner_key_fn: InnerKeyFn,
        result_selector: ResultFn,
    ) -> impl Iterator<Item = R>
    where
        Inner: IntoIterator,
        OuterKey: PartialEq,
        InnerKey: PartialEq<OuterKey>,
        OuterKeyFn: Fn(&Self::Item) -> OuterKey,
        InnerKeyFn: Fn(&Inner::Item) -> InnerKey,
        ResultFn: Fn(Self::Item, Vec<Inner::Item>) -> R,
        Inner::Item: Clone,
    {
        let inner_vec: Vec<Inner::Item> = inner.into_iter().collect();
        self.map(move |outer_item| {
            let outer_key = outer_key_fn(&outer_item);
            let group: Vec<Inner::Item> = inner_vec
                .iter()
                .filter(|i| inner_key_fn(i) == outer_key)
                .cloned()
                .collect();
            result_selector(outer_item, group)
        })
        .collect::<Vec<_>>()
        .into_iter()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // GROUPING
    // ═══════════════════════════════════════════════════════════════════════

    /// Groups elements by a key selector. Equivalent to `GroupBy`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    ///
    /// let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    /// let mut groups: Vec<_> = words.into_iter()
    ///     .group_by(|w| w.chars().next().unwrap())
    ///     .collect();
    /// groups.sort_by_key(|g| g.key);
    /// assert_eq!(groups[0].key, 'a');
    /// assert_eq!(groups[0].elements, ["apple", "ant"]);
    /// ```
    fn group_by<K, F>(self, mut key_fn: F) -> impl Iterator<Item = Grouping<K, Self::Item>>
    where
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut groups: Vec<Grouping<K, Self::Item>> = Vec::new();
        for item in self {
            let key = key_fn(&item);
            if let Some(g) = groups.iter_mut().find(|g| g.key == key) {
                g.elements.push(item);
            } else {
                let mut g = Grouping::new(key);
                g.elements.push(item);
                groups.push(g);
            }
        }
        groups.into_iter()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // CONVERSION
    // ═══════════════════════════════════════════════════════════════════════

    /// Collects into a `Vec`. Equivalent to `ToList`.
    fn to_vec(self) -> Vec<Self::Item> {
        self.collect()
    }

    /// Collects into a `HashMap` by a key selector. Equivalent to `ToDictionary`.
    fn to_hashmap<K, F>(self, key_fn: F) -> std::collections::HashMap<K, Self::Item>
    where
        K: std::hash::Hash + Eq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut key_fn = key_fn;
        let mut map = std::collections::HashMap::new();
        for item in self {
            let key = key_fn(&item);
            map.insert(key, item);
        }
        map
    }

    /// Collects into a `HashSet`. Equivalent to `ToHashSet`.
    fn to_hashset(self) -> std::collections::HashSet<Self::Item>
    where
        Self::Item: std::hash::Hash + Eq,
    {
        self.collect()
    }

    /// Builds a [`Lookup`] (one-to-many dictionary). Equivalent to `ToLookup`.
    fn to_lookup<K, F>(self, mut key_fn: F) -> Lookup<K, Self::Item>
    where
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut lookup = Lookup::new();
        for item in self {
            let key = key_fn(&item);
            lookup.insert(key, item);
        }
        lookup
    }

    // ═══════════════════════════════════════════════════════════════════════
    // UTILITY
    // ═══════════════════════════════════════════════════════════════════════

    /// Merges two sequences element-by-element using a result selector.
    /// Equivalent to `Zip(second, resultSelector)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let sums: Vec<_> = vec![1, 2, 3].into_iter()
    ///     .zip_(vec![10, 20, 30], |a, b| a + b)
    ///     .collect();
    /// assert_eq!(sums, [11, 22, 33]);
    /// ```
    fn zip_<J, R, F>(self, other: J, result_selector: F) -> Zip<Self, J::IntoIter, F>
    where
        J: IntoIterator,
        F: FnMut(Self::Item, J::Item) -> R,
    {
        Zip {
            first: self,
            second: other.into_iter(),
            result_selector,
        }
    }

    /// Applies an action to each element (for side effects). Equivalent to `ForEach` / `Do`.
    fn for_each_<F>(self, f: F)
    where
        F: FnMut(Self::Item),
    {
        self.for_each(f)
    }

    /// Appends a single element to the end of the sequence. Equivalent to `Append`.
    fn append_item(self, item: Self::Item) -> impl Iterator<Item = Self::Item> {
        self.chain(std::iter::once(item))
    }

    /// Prepends a single element to the front of the sequence. Equivalent to `Prepend`.
    fn prepend_item(self, item: Self::Item) -> impl Iterator<Item = Self::Item> {
        std::iter::once(item).chain(self)
    }

    /// Returns `true` if the sequence contains no elements. Equivalent to `!Any()`.
    fn is_empty_(mut self) -> bool {
        self.next().is_none()
    }

    /// Sequence equality — two sequences are equal if they yield the same
    /// elements in the same order. Equivalent to `SequenceEqual`.
    fn sequence_equal<I2>(self, other: I2) -> bool
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq,
    {
        let mut other = other.into_iter();
        for item in self {
            match other.next() {
                Some(o) if o == item => continue,
                _ => return false,
            }
        }
        other.next().is_none()
    }
}

// Blanket implementation — every `Iterator` gets all LINQ methods.
impl<I: Iterator> LinqExt for I {}
