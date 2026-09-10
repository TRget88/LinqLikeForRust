//! The [`LinqExt`] extension trait — brings LINQ methods to every `Iterator`.

use crate::adaptors::*;
use crate::error::SingleError;
use crate::grouping::Grouping;
use crate::lookup::Lookup;
use crate::ordered::OrderedQueryable;

/// Extends every `Iterator` with C# LINQ–style query operations.
///
/// # This does not claim per-method equivalence with C#
///
/// Method docs below name the C# analogue so you can find your way around, but
/// a shared name does **not** mean shared behaviour. Most operators here
/// delegate to `std::iter`, and where `std` and C# disagree, `std` wins
/// (`D-005`). The normative list of divergences — empty sequences, tie-breaking,
/// overflow, string collation, duplicate keys, evaluation timing — is the
/// *Differences from C# LINQ* section of the crate documentation. Three
/// operators (`for_each_`, `element_at_or`, `zip3`) name C# methods or overloads
/// that **do not exist**; they are marked below.
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

    /// Filters elements by a predicate. C# analogue: `Where`.
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
        Where {
            inner: self,
            predicate,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PROJECTION
    // ═══════════════════════════════════════════════════════════════════════

    /// Projects each element into a new form. C# analogue: `Select`.
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
    /// C# analogue: `SelectMany`.
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
        J: IntoIterator,
    {
        SelectMany {
            outer: self,
            f,
            current: None,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PAGING / SLICING
    // ═══════════════════════════════════════════════════════════════════════

    /// Skips the first `n` elements. C# analogue: `Skip`.
    ///
    /// Named `skip_` to avoid colliding with [`Iterator::skip`].
    fn skip_(self, n: usize) -> Skip<Self> {
        Skip {
            inner: self,
            remaining: n,
        }
    }

    /// Takes at most `n` elements. C# analogue: `Take`.
    fn take_(self, n: usize) -> Take<Self> {
        Take {
            inner: self,
            remaining: n,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SET OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`distinct`](Self::distinct)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Returns distinct elements. C# analogue: `Distinct`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let d: Vec<_> = vec![1, 2, 2, 3, 1].into_iter().distinct_partial_eq().collect();
    /// assert_eq!(d, [1, 2, 3]);
    /// ```
    fn distinct_partial_eq(self) -> Distinct<Self>
    where
        Self::Item: PartialEq + Clone,
    {
        Distinct {
            inner: self,
            seen: Vec::new(),
        }
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`distinct_by`](Self::distinct_by)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Returns distinct elements by a key selector. C# analogue: `DistinctBy`.
    fn distinct_by_partial_eq<K, F>(self, key_fn: F) -> DistinctBy<Self, F, K>
    where
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        DistinctBy {
            inner: self,
            key_fn,
            seen_keys: Vec::new(),
        }
    }

    /// Returns distinct elements, hash-backed and O(n).
    ///
    /// Requires `Eq + Hash`; for element types that are only `PartialEq` —
    /// `f64`, say — use [`distinct_partial_eq`](Self::distinct_partial_eq),
    /// which is O(n²).
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<_> = vec![1, 2, 2, 3, 1].into_iter().distinct().collect();
    /// assert_eq!(v, [1, 2, 3]);
    /// ```
    fn distinct(self) -> impl Iterator<Item = Self::Item>
    where
        Self::Item: Eq + std::hash::Hash + Clone,
    {
        let mut seen = std::collections::HashSet::new();
        self.filter(move |x| seen.insert(x.clone()))
    }

    /// Returns distinct elements by a key selector, using a `HashSet`.
    /// Requires `Eq + Hash` keys; for `PartialEq`-only keys use
    /// [`distinct_by_partial_eq`](Self::distinct_by_partial_eq). When the key
    /// type implements `Eq + Hash`.
    fn distinct_by<K, F>(self, mut key_fn: F) -> impl Iterator<Item = Self::Item>
    where
        K: Eq + std::hash::Hash,
        F: FnMut(&Self::Item) -> K,
    {
        let mut seen: std::collections::HashSet<K> = std::collections::HashSet::new();
        self.filter(move |x| seen.insert(key_fn(x)))
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`except`](Self::except)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Returns elements of `self` that are not in `other`. C# analogue: `Except`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let diff: Vec<_> = vec![1,2,3,4].into_iter().except_partial_eq(vec![2,4]).collect();
    /// assert_eq!(diff, [1, 3]);
    /// ```
    fn except_partial_eq<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq,
    {
        let exclusions: Vec<_> = other.into_iter().collect();
        self.where_(move |x| !exclusions.contains(x))
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`intersect`](Self::intersect)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Returns elements that appear in both sequences. C# analogue: `Intersect`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let inter: Vec<_> = vec![1,2,3,4].into_iter().intersect_partial_eq(vec![2,4,6]).collect();
    /// assert_eq!(inter, [2, 4]);
    /// ```
    fn intersect_partial_eq<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq,
    {
        let inclusion: Vec<_> = other.into_iter().collect();
        self.where_(move |x| inclusion.contains(x))
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`union_`](Self::union_)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Produces the set union of two sequences. C# analogue: `Union`.
    fn union_partial_eq<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: PartialEq,
    {
        let mut result: Vec<Self::Item> = Vec::new();
        for item in self.chain(other) {
            if !result.contains(&item) {
                result.push(item);
            }
        }
        result.into_iter()
    }

    /// Returns elements of `self` whose **projected key** is not in `other`.
    /// C# analogue: `ExceptBy(other, keySelector)`. Note that `other`
    /// supplies *keys*, not items.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let words = vec!["apple", "ant", "banana", "bear"];
    /// let v: Vec<_> = words
    ///     .into_iter()
    ///     .except_by(vec!['a'], |s| s.chars().next().unwrap())
    ///     .collect();
    /// assert_eq!(v, ["banana", "bear"]);
    /// ```
    fn except_by<I2, K, F>(self, other: I2, mut key_fn: F) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = K>,
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let exclusions: Vec<K> = other.into_iter().collect();
        self.where_(move |x| !exclusions.contains(&key_fn(x)))
    }

    /// Returns elements of `self` whose **projected key** appears in `other`.
    /// C# analogue: `IntersectBy(other, keySelector)`. Note that `other`
    /// supplies *keys*, not items.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    /// let v: Vec<_> = words
    ///     .into_iter()
    ///     .intersect_by(vec!['a', 'c'], |s| s.chars().next().unwrap())
    ///     .collect();
    /// assert_eq!(v, ["apple", "ant", "cherry"]);
    /// ```
    fn intersect_by<I2, K, F>(self, other: I2, mut key_fn: F) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = K>,
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let inclusions: Vec<K> = other.into_iter().collect();
        self.where_(move |x| inclusions.contains(&key_fn(x)))
    }

    /// Set difference: elements of `self` that are not in `other`.
    ///
    /// Hash-indexed, O(n + m). Requires `Eq + Hash`; for element types that
    /// are only `PartialEq` — `f64`, say — use
    /// [`except_partial_eq`](Self::except_partial_eq), which is O(n·m).
    fn except<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: Eq + std::hash::Hash,
    {
        let exclusions: std::collections::HashSet<Self::Item> = other.into_iter().collect();
        self.filter(move |x| !exclusions.contains(x))
    }

    /// Elements that appear in both sequences.
    ///
    /// Hash-indexed, O(n + m). Requires `Eq + Hash`; for `PartialEq`-only
    /// element types use [`intersect_partial_eq`](Self::intersect_partial_eq).
    fn intersect<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: Eq + std::hash::Hash,
    {
        let inclusions: std::collections::HashSet<Self::Item> = other.into_iter().collect();
        self.filter(move |x| inclusions.contains(x))
    }

    /// Set union of two sequences.
    ///
    /// Hash-indexed, O(n + m). Requires `Eq + Hash`; for `PartialEq`-only
    /// element types use [`union_partial_eq`](Self::union_partial_eq).
    fn union_<I2>(self, other: I2) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        Self::Item: Eq + std::hash::Hash + Clone,
    {
        let mut seen: std::collections::HashSet<Self::Item> = std::collections::HashSet::new();
        self.chain(other).filter(move |x| seen.insert(x.clone()))
    }

    /// Produces the set union of two sequences, deduplicating by projected
    /// key. C# analogue: `UnionBy(other, keySelector)`. Unlike
    /// `except_by` / `intersect_by`, `other` supplies *items* (the same type
    /// as `self`'s items), not keys.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let first  = vec!["apple", "ant"];
    /// let second = vec!["avocado", "banana"];
    /// // 'a' is the key for all three of "apple", "ant", "avocado".
    /// let v: Vec<_> = first
    ///     .into_iter()
    ///     .union_by(second, |s| s.chars().next().unwrap())
    ///     .collect();
    /// assert_eq!(v, ["apple", "banana"]);
    /// ```
    fn union_by<I2, K, F>(self, other: I2, mut key_fn: F) -> impl Iterator<Item = Self::Item>
    where
        I2: IntoIterator<Item = Self::Item>,
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut result: Vec<Self::Item> = Vec::new();
        let mut seen_keys: Vec<K> = Vec::new();
        for item in self.chain(other) {
            let k = key_fn(&item);
            if !seen_keys.contains(&k) {
                seen_keys.push(k);
                result.push(item);
            }
        }
        result.into_iter()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ORDERING
    // ═══════════════════════════════════════════════════════════════════════

    /// Sorts the sequence by a key in ascending order. C# analogue: `OrderBy`.
    ///
    /// Returns an [`OrderedQueryable`] that supports `.then_by()`. Sorting is
    /// **deferred** until iteration, so chained `then_by` calls compose into
    /// a single lexicographic sort.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let sorted: Vec<_> = vec!["banana","apple","cherry"]
    ///     .into_iter()
    ///     .order_by(|s| *s)
    ///     .collect();
    /// assert_eq!(sorted, ["apple", "banana", "cherry"]);
    /// ```
    fn order_by<'a, K, F>(self, key_fn: F) -> OrderedQueryable<'a, Self::Item>
    where
        K: Ord,
        F: Fn(&Self::Item) -> K + 'a,
        Self::Item: 'a,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(move |a, b| key_fn(a).cmp(&key_fn(b))))
    }

    /// Sorts with an explicit comparator, for keys that are `PartialOrd` but
    /// not `Ord`.
    ///
    /// `order_by` binds `K: Ord`, which rules out floats — and this crate ships
    /// no `IComparer` equivalent, so before this there was no way to sort by an
    /// `f64` at all. C# has no such restriction (`OrderBy` accepts `double`),
    /// which made it a real gap for a port.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let by_price: Vec<_> = vec![("b", 2.5f64), ("a", 1.0)]
    ///     .into_iter()
    ///     .order_by_with(|x, y| x.1.total_cmp(&y.1))
    ///     .collect();
    /// assert_eq!(by_price, [("a", 1.0), ("b", 2.5)]);
    /// ```
    fn order_by_with<'a, F>(self, cmp: F) -> OrderedQueryable<'a, Self::Item>
    where
        F: Fn(&Self::Item, &Self::Item) -> std::cmp::Ordering + 'a,
        Self::Item: 'a,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(cmp))
    }

    /// Sorts in descending order. C# analogue: `OrderByDescending`.
    fn order_by_descending<'a, K, F>(self, key_fn: F) -> OrderedQueryable<'a, Self::Item>
    where
        K: Ord,
        F: Fn(&Self::Item) -> K + 'a,
        Self::Item: 'a,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(move |a, b| key_fn(b).cmp(&key_fn(a))))
    }

    /// Sorts the sequence in ascending order using `Ord` on the elements
    /// themselves. C# analogue: `Order()` (.NET 6+).
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let sorted: Vec<_> = vec![3, 1, 4, 1, 5, 9, 2]
    ///     .into_iter()
    ///     .order()
    ///     .collect();
    /// assert_eq!(sorted, [1, 1, 2, 3, 4, 5, 9]);
    /// ```
    fn order<'a>(self) -> OrderedQueryable<'a, Self::Item>
    where
        Self::Item: Ord + 'a,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(|a, b| a.cmp(b)))
    }

    /// Sorts the sequence in descending order using `Ord` on the elements
    /// themselves. C# analogue: `OrderDescending()` (.NET 6+).
    fn order_descending<'a>(self) -> OrderedQueryable<'a, Self::Item>
    where
        Self::Item: Ord + 'a,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(|a, b| b.cmp(a)))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // AGGREGATION
    // ═══════════════════════════════════════════════════════════════════════

    /// Sums elements that implement `std::iter::Sum`. C# analogue: `Sum`.
    #[must_use]
    fn sum_<S>(self) -> S
    where
        S: std::iter::Sum<Self::Item>,
    {
        self.sum()
    }

    /// Projects each element via `selector` and returns the sum of the
    /// projection. C# analogue: `Sum(selector)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let pairs = vec![("a", 1), ("b", 2), ("c", 3)];
    /// let total: i32 = pairs.into_iter().sum_by(|(_, n)| n);
    /// assert_eq!(total, 6);
    /// ```
    #[must_use]
    fn sum_by<T, S, F>(self, selector: F) -> S
    where
        S: std::iter::Sum<T>,
        F: FnMut(Self::Item) -> T,
    {
        self.map(selector).sum()
    }

    /// Counts elements matching an optional predicate.
    /// `count()` with no argument counts all; `count_where(p)` counts matching.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// assert_eq!((1..=10).count_where(|x| x % 2 == 0), 5);
    /// ```
    #[must_use]
    fn count_where<P>(self, predicate: P) -> usize
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.where_(predicate).count()
    }

    /// Returns the minimum element, or `None` if the iterator is empty.
    #[must_use]
    fn min_(self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.min()
    }

    /// Returns the maximum element, or `None` if the iterator is empty.
    #[must_use]
    fn max_(self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.max()
    }

    /// Returns the minimum element under an explicit comparator.
    ///
    /// The comparator counterpart to [`min_by_key_`](Self::min_by_key_), for
    /// keys that are `PartialOrd` but not `Ord`. Delegates to
    /// [`Iterator::min_by`]; on ties the **first** minimum wins.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let cheapest = vec![("b", 2.5f64), ("a", 1.0)]
    ///     .into_iter()
    ///     .min_by_(|x, y| x.1.total_cmp(&y.1));
    /// assert_eq!(cheapest, Some(("a", 1.0)));
    /// ```
    #[must_use]
    fn min_by_<F>(self, cmp: F) -> Option<Self::Item>
    where
        F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering,
    {
        self.min_by(cmp)
    }

    /// Returns the maximum element under an explicit comparator.
    ///
    /// Delegates to [`Iterator::max_by`]; on ties the **last** maximum wins,
    /// which differs from C# `MaxBy`. See *Differences from C# LINQ*.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let dearest = vec![("b", 2.5f64), ("a", 1.0)]
    ///     .into_iter()
    ///     .max_by_(|x, y| x.1.total_cmp(&y.1));
    /// assert_eq!(dearest, Some(("b", 2.5)));
    /// ```
    #[must_use]
    fn max_by_<F>(self, cmp: F) -> Option<Self::Item>
    where
        F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering,
    {
        self.max_by(cmp)
    }

    /// Returns the **element** whose projected key is smallest.
    /// C# analogue: `MinBy(keySelector)`; delegates to [`Iterator::min_by_key`]. Ties agree with C# — the first minimum wins.
    ///
    /// Returns `None` if the sequence is empty.
    #[must_use]
    fn min_by_key_<K, F>(self, key_fn: F) -> Option<Self::Item>
    where
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        self.min_by_key(key_fn)
    }

    /// Returns the **element** whose projected key is largest.
    /// C# analogue: `MaxBy(keySelector)`; delegates to [`Iterator::max_by_key`]. **Ties differ from C#**: this returns the *last* maximum, C# `MaxBy` the first.
    ///
    /// Returns `None` if the sequence is empty.
    #[must_use]
    fn max_by_key_<K, F>(self, key_fn: F) -> Option<Self::Item>
    where
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        self.max_by_key(key_fn)
    }

    /// Computes the average of a sequence mapped to `f64`. C# analogue: `Average`.
    ///
    /// Returns `None` if the iterator is empty.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let avg = vec![1.0f64, 2.0, 3.0].into_iter().average(|x| x);
    /// assert_eq!(avg, Some(2.0));
    /// ```
    #[must_use]
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
        if count == 0 {
            None
        } else {
            Some(sum / count as f64)
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ELEMENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns the first element. **Panics** if the sequence is empty.
    ///
    /// C# analogue: `First()`. For a non-panicking version, see
    /// [`first_or_default`](Self::first_or_default).
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// assert_eq!((1..=5).first(), 1);
    /// ```
    #[must_use]
    fn first(mut self) -> Self::Item {
        self.next().expect("sequence contains no elements")
    }

    /// Returns the first element, or `None`. C# analogue: `FirstOrDefault`.
    #[must_use]
    fn first_or_default(mut self) -> Option<Self::Item> {
        self.next()
    }

    /// Returns the first element matching a predicate, or `None`.
    #[must_use]
    fn first_where<P>(self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.where_(predicate).next()
    }

    /// Returns the last element. **Panics** if the sequence is empty.
    ///
    /// Named `last_` to avoid colliding with [`Iterator::last`]. C# analogue:
    /// C# `Last()`. For a non-panicking version, see
    /// [`last_or_default`](Self::last_or_default).
    #[must_use]
    fn last_(self) -> Self::Item {
        self.last().expect("sequence contains no elements")
    }

    /// Returns the last element, or `None`. C# analogue: `LastOrDefault`.
    #[must_use]
    fn last_or_default(self) -> Option<Self::Item> {
        self.fold(None, |_, x| Some(x))
    }

    /// Returns the last element matching a predicate, or `None`.
    #[must_use]
    fn last_where<P>(self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.where_(predicate).fold(None, |_, x| Some(x))
    }

    /// Returns the single element, reporting which way it failed.
    ///
    /// Consumes at most two elements, so it terminates on an infinite source.
    ///
    /// # Errors
    ///
    /// [`SingleError::Empty`] if there were no elements;
    /// [`SingleError::MoreThanOne`] if there were two or more. The two cases
    /// are usually different bugs — "no row matched" is often recoverable,
    /// "several rows matched" usually means a uniqueness assumption is wrong.
    ///
    /// ```rust
    /// use linq_rs::{LinqExt, SingleError};
    /// assert_eq!(vec![7].into_iter().try_single(), Ok(7));
    /// assert_eq!(vec![1, 2].into_iter().try_single(), Err(SingleError::MoreThanOne));
    /// ```
    fn try_single(mut self) -> Result<Self::Item, SingleError> {
        let first = self.next().ok_or(SingleError::Empty)?;
        if self.next().is_some() {
            return Err(SingleError::MoreThanOne);
        }
        Ok(first)
    }

    /// Returns the single element. C# analogue: `Single()`.
    ///
    /// For a non-panicking form see [`try_single`](Self::try_single).
    ///
    /// # Panics
    ///
    /// Panics with `"sequence contains no elements"` if empty, or
    /// `"sequence contains more than one element"` if there are two or more.
    /// Both strings come from [`SingleError::message`], so they cannot drift
    /// from this section.
    #[must_use]
    fn single(self) -> Self::Item {
        match self.try_single() {
            Ok(value) => value,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Returns `Ok(Some(x))` for exactly one element, `Ok(None)` for none, and
    /// an error if there were several.
    ///
    /// **This is not C# `SingleOrDefault`, deliberately.** C# throws on two or
    /// more; an earlier version of this method returned `None` for *both*
    /// empty and too-many, so a caller could not tell "not found" from a broken
    /// uniqueness assumption. Absent stays ordinary (`Ok(None)`); ambiguous
    /// becomes an error you have to handle.
    ///
    /// To recover the old collapsing behaviour: `.single_or_default().ok().flatten()`.
    ///
    /// # Errors
    ///
    /// Only ever [`SingleError::MoreThanOne`] — the empty case is `Ok(None)`.
    ///
    /// ```rust
    /// use linq_rs::{LinqExt, SingleError};
    /// assert_eq!(vec![42].into_iter().single_or_default(), Ok(Some(42)));
    /// assert_eq!(Vec::<i32>::new().into_iter().single_or_default(), Ok(None));
    /// assert_eq!(vec![1, 2].into_iter().single_or_default(), Err(SingleError::MoreThanOne));
    /// ```
    fn single_or_default(self) -> Result<Option<Self::Item>, SingleError> {
        match self.try_single() {
            Ok(value) => Ok(Some(value)),
            Err(SingleError::Empty) => Ok(None),
            Err(SingleError::MoreThanOne) => Err(SingleError::MoreThanOne),
        }
    }

    /// Returns the first element, or `default` if the sequence is empty.
    /// C# analogue: `FirstOrDefault(defaultValue)` (.NET 6+).
    #[must_use]
    fn first_or(mut self, default: Self::Item) -> Self::Item {
        self.next().unwrap_or(default)
    }

    /// Returns the last element, or `default` if the sequence is empty.
    /// C# analogue: `LastOrDefault(defaultValue)` (.NET 6+).
    #[must_use]
    fn last_or(self, default: Self::Item) -> Self::Item {
        self.last().unwrap_or(default)
    }

    /// Returns the single element, or `default` if the sequence is empty.
    /// **Panics** if the sequence contains more than one element (mirrors
    /// C# `SingleOrDefault(defaultValue)` — the .NET 6+ overload still throws
    /// on multiple matches, the default only kicks in for empty).
    ///
    /// # Panics
    ///
    /// Panics with `"sequence contains more than one element"` if there are two
    /// or more. The string comes from [`SingleError::message`].
    #[must_use]
    fn single_or(self, default: Self::Item) -> Self::Item {
        match self.try_single() {
            Ok(value) => value,
            Err(SingleError::Empty) => default,
            Err(err @ SingleError::MoreThanOne) => panic!("{}", err.message()),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QUANTIFIERS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns `true` if any element satisfies the predicate. C# analogue: `Any`.
    #[must_use]
    fn any_<P>(mut self, predicate: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        self.any(predicate)
    }

    /// Returns `true` if every element satisfies the predicate. C# analogue: `All`.
    #[must_use]
    fn all_<P>(mut self, predicate: P) -> bool
    where
        P: FnMut(Self::Item) -> bool,
    {
        self.all(predicate)
    }

    /// Returns `true` if the sequence contains a specific value. C# analogue: `Contains`.
    #[must_use]
    fn contains_<T>(mut self, value: &T) -> bool
    where
        Self::Item: PartialEq<T>,
    {
        self.any(|x| x == *value)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // JOINING
    // ═══════════════════════════════════════════════════════════════════════

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`inner_join`](Self::inner_join)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Performs an inner join between `self` and `inner` on matching keys,
    /// projecting results with `result_selector`.
    ///
    /// C# analogue: `Join(inner, outerKey, innerKey, resultSelector)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    ///
    /// let people = vec![(1u32, "Alice"), (2, "Bob")];
    /// let orders = vec![(1u32, "Order A"), (1, "Order B"), (2, "Order C")];
    ///
    /// let mut results: Vec<_> = people.into_iter()
    ///     .inner_join_partial_eq(
    ///         orders,
    ///         |(id, _)| *id,
    ///         |(id, _)| *id,
    ///         |(_, name), (_, order)| format!("{name}: {order}"),
    ///     )
    ///     .collect();
    /// results.sort();
    /// assert_eq!(results, ["Alice: Order A", "Alice: Order B", "Bob: Order C"]);
    /// ```
    fn inner_join_partial_eq<Inner, OuterKey, InnerKey, R, OuterKeyFn, InnerKeyFn, ResultFn>(
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

    /// Inner join on matching keys, projecting results with `result_selector`.
    ///
    /// Builds a hash index over `inner`, so O(n + m) rather than the nested
    /// loop's O(n·m). Requires `Eq + Hash` keys; for `PartialEq`-only keys use
    /// [`inner_join_partial_eq`](Self::inner_join_partial_eq).
    /// **Prefer this** when both sides project to a `Key: Eq + Hash` (must
    /// be the same type for both, unlike the un-hashed version which
    /// supports asymmetric `PartialEq<...>` keys).
    fn inner_join<Inner, Key, R, OuterKeyFn, InnerKeyFn, ResultFn>(
        self,
        inner: Inner,
        outer_key_fn: OuterKeyFn,
        inner_key_fn: InnerKeyFn,
        result_selector: ResultFn,
    ) -> impl Iterator<Item = R>
    where
        Inner: IntoIterator,
        Key: Eq + std::hash::Hash,
        OuterKeyFn: Fn(&Self::Item) -> Key,
        InnerKeyFn: Fn(&Inner::Item) -> Key,
        ResultFn: Fn(Self::Item, Inner::Item) -> R,
        Self::Item: Clone,
        Inner::Item: Clone,
    {
        let mut inner_map: std::collections::HashMap<Key, Vec<Inner::Item>> =
            std::collections::HashMap::new();
        for item in inner {
            let k = inner_key_fn(&item);
            inner_map.entry(k).or_default().push(item);
        }
        let mut results = Vec::new();
        for outer_item in self {
            let outer_key = outer_key_fn(&outer_item);
            if let Some(inner_items) = inner_map.get(&outer_key) {
                for inner_item in inner_items {
                    results.push(result_selector(outer_item.clone(), inner_item.clone()));
                }
            }
        }
        results.into_iter()
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`group_join`](Self::group_join)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Performs a group join (left outer join with grouped inner elements).
    /// C# analogue: `GroupJoin`.
    fn group_join_partial_eq<Inner, OuterKey, InnerKey, R, OuterKeyFn, InnerKeyFn, ResultFn>(
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

    /// Left outer join with the matching inner elements grouped.
    ///
    /// Hash-indexed, O(n + m). Requires `Eq + Hash` keys; for `PartialEq`-only
    /// keys use [`group_join_partial_eq`](Self::group_join_partial_eq).
    /// **Prefer this** when both sides project to a `Key: Eq + Hash`.
    fn group_join<Inner, Key, R, OuterKeyFn, InnerKeyFn, ResultFn>(
        self,
        inner: Inner,
        outer_key_fn: OuterKeyFn,
        inner_key_fn: InnerKeyFn,
        result_selector: ResultFn,
    ) -> impl Iterator<Item = R>
    where
        Inner: IntoIterator,
        Key: Eq + std::hash::Hash,
        OuterKeyFn: Fn(&Self::Item) -> Key,
        InnerKeyFn: Fn(&Inner::Item) -> Key,
        ResultFn: Fn(Self::Item, Vec<Inner::Item>) -> R,
        Inner::Item: Clone,
    {
        let mut inner_map: std::collections::HashMap<Key, Vec<Inner::Item>> =
            std::collections::HashMap::new();
        for item in inner {
            let k = inner_key_fn(&item);
            inner_map.entry(k).or_default().push(item);
        }
        self.map(move |outer_item| {
            let outer_key = outer_key_fn(&outer_item);
            let group = inner_map.get(&outer_key).cloned().unwrap_or_default();
            result_selector(outer_item, group)
        })
        .collect::<Vec<_>>()
        .into_iter()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // GROUPING
    // ═══════════════════════════════════════════════════════════════════════

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`group_by_key`](Self::group_by_key)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Groups elements by a key selector. C# analogue: `GroupBy`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    ///
    /// let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    /// let mut groups: Vec<_> = words.into_iter()
    ///     .group_by_key_partial_eq(|w| w.chars().next().unwrap())
    ///     .collect();
    /// groups.sort_by_key(|g| *g.key());
    /// assert_eq!(*groups[0].key(), 'a');
    /// assert_eq!(groups[0].elements(), ["apple", "ant"]);
    /// ```
    fn group_by_key_partial_eq<K, F>(
        self,
        mut key_fn: F,
    ) -> impl Iterator<Item = Grouping<K, Self::Item>>
    where
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut groups: Vec<Grouping<K, Self::Item>> = Vec::new();
        for item in self {
            let key = key_fn(&item);
            if let Some(g) = groups.iter_mut().find(|g| *g.key() == key) {
                g.push(item);
            } else {
                let mut g = Grouping::new(key);
                g.push(item);
                groups.push(g);
            }
        }
        groups.into_iter()
    }

    /// Groups elements by a key, projecting each item through `element_fn`
    /// before bucketing. C# analogue:
    /// `GroupBy(keySelector, elementSelector)`.
    fn group_by_with_element<K, E, KF, EF>(
        self,
        mut key_fn: KF,
        mut element_fn: EF,
    ) -> impl Iterator<Item = Grouping<K, E>>
    where
        K: PartialEq,
        KF: FnMut(&Self::Item) -> K,
        EF: FnMut(Self::Item) -> E,
    {
        let mut groups: Vec<Grouping<K, E>> = Vec::new();
        for item in self {
            let key = key_fn(&item);
            let element = element_fn(item);
            if let Some(g) = groups.iter_mut().find(|g| *g.key() == key) {
                g.push(element);
            } else {
                let mut g = Grouping::new(key);
                g.push(element);
                groups.push(g);
            }
        }
        groups.into_iter()
    }

    /// Groups elements by a key, then projects each group through
    /// `result_fn(key, members)` to produce a per-group result. Equivalent
    /// to C# `GroupBy(keySelector, resultSelector)`.
    fn group_by_with_result<K, R, KF, RF>(
        self,
        mut key_fn: KF,
        mut result_fn: RF,
    ) -> impl Iterator<Item = R>
    where
        K: PartialEq,
        KF: FnMut(&Self::Item) -> K,
        RF: FnMut(K, Vec<Self::Item>) -> R,
    {
        let mut groups: Vec<Grouping<K, Self::Item>> = Vec::new();
        for item in self {
            let key = key_fn(&item);
            if let Some(g) = groups.iter_mut().find(|g| *g.key() == key) {
                g.push(item);
            } else {
                let mut g = Grouping::new(key);
                g.push(item);
                groups.push(g);
            }
        }
        groups.into_iter().map(move |g| {
            let (k, v) = g.into_parts();
            result_fn(k, v)
        })
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`count_by`](Self::count_by)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Groups by `key_fn` and yields `(key, count)` pairs. C# analogue:
    /// .NET 9+ `CountBy(keySelector)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    /// let mut counts: Vec<_> = words.into_iter()
    ///     .count_by_partial_eq(|w| w.chars().next().unwrap())
    ///     .collect();
    /// counts.sort_by_key(|(k, _)| *k);
    /// assert_eq!(counts, [('a', 2), ('b', 2), ('c', 1)]);
    /// ```
    fn count_by_partial_eq<K, F>(self, mut key_fn: F) -> impl Iterator<Item = (K, usize)>
    where
        K: PartialEq,
        F: FnMut(&Self::Item) -> K,
    {
        let mut counts: Vec<(K, usize)> = Vec::new();
        for item in self {
            let key = key_fn(&item);
            if let Some(c) = counts.iter_mut().find(|(k, _)| *k == key) {
                c.1 += 1;
            } else {
                counts.push((key, 1));
            }
        }
        counts.into_iter()
    }

    /// Groups elements by a key, yielding one [`Grouping`] per distinct key in
    /// **first-appearance order**.
    ///
    /// Hash-indexed, O(n). Requires `Eq + Hash + Clone` keys; for
    /// `PartialEq`-only keys use
    /// [`group_by_key_partial_eq`](Self::group_by_key_partial_eq), which is O(n·k).
    ///
    /// Groups are yielded in **insertion order of their first occurrence**,
    /// not in hash order.
    fn group_by_key<K, F>(self, mut key_fn: F) -> impl Iterator<Item = Grouping<K, Self::Item>>
    where
        K: Eq + std::hash::Hash + Clone,
        F: FnMut(&Self::Item) -> K,
    {
        let mut groups: Vec<Grouping<K, Self::Item>> = Vec::new();
        let mut index: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
        for item in self {
            let key = key_fn(&item);
            match index.get(&key) {
                Some(&pos) => groups[pos].push(item),
                None => {
                    let pos = groups.len();
                    index.insert(key.clone(), pos);
                    let mut g = Grouping::new(key);
                    g.push(item);
                    groups.push(g);
                }
            }
        }
        groups.into_iter()
    }

    /// Counts elements per key. Yields in hash order, not
    /// insertion order — the std `HashMap` iteration order is unspecified.
    fn count_by<K, F>(self, mut key_fn: F) -> impl Iterator<Item = (K, usize)>
    where
        K: Eq + std::hash::Hash,
        F: FnMut(&Self::Item) -> K,
    {
        let mut counts: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
        for item in self {
            let key = key_fn(&item);
            *counts.entry(key).or_insert(0) += 1;
        }
        counts.into_iter()
    }

    /// Fused group-and-aggregate. Yields in hash
    /// order, not insertion order.
    fn aggregate_by<K, Acc, KF, SF, AF>(
        self,
        mut key_fn: KF,
        mut seed_fn: SF,
        mut accum: AF,
    ) -> impl Iterator<Item = (K, Acc)>
    where
        K: Eq + std::hash::Hash,
        KF: FnMut(&Self::Item) -> K,
        SF: FnMut(&K) -> Acc,
        AF: FnMut(Acc, Self::Item) -> Acc,
    {
        let mut accs: std::collections::HashMap<K, Acc> = std::collections::HashMap::new();
        for item in self {
            let key = key_fn(&item);
            let new_acc = match accs.remove(&key) {
                Some(existing) => accum(existing, item),
                None => accum(seed_fn(&key), item),
            };
            accs.insert(key, new_acc);
        }
        accs.into_iter()
    }

    /// **Escape hatch.** Compares with `PartialEq` and scans linearly, so it
    /// works on element/key types that are not `Eq + Hash` — `f64`, most
    /// notably — at the cost of quadratic time. Prefer [`aggregate_by`](Self::aggregate_by)
    /// unless you need that. See `DECISIONS.md` `D-101`.
    ///
    /// Groups by `key_fn` and aggregates each group with `seed_fn` +
    /// `accum`. C# analogue: `AggregateBy(keySelector, seedSelector,
    /// func)` (.NET 6+). `seed_fn` receives the key so per-key seeds are possible;
    /// pass `|_| my_const` for a global seed.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let data = vec![("a", 1), ("b", 2), ("a", 3), ("b", 10)];
    /// let mut totals: Vec<_> = data.into_iter()
    ///     .aggregate_by_partial_eq(|(k, _)| *k, |_| 0, |acc, (_, v)| acc + v)
    ///     .collect();
    /// totals.sort_by_key(|(k, _)| *k);
    /// assert_eq!(totals, [("a", 4), ("b", 12)]);
    /// ```
    fn aggregate_by_partial_eq<K, Acc, KF, SF, AF>(
        self,
        mut key_fn: KF,
        mut seed_fn: SF,
        mut accum: AF,
    ) -> impl Iterator<Item = (K, Acc)>
    where
        K: PartialEq,
        KF: FnMut(&Self::Item) -> K,
        SF: FnMut(&K) -> Acc,
        AF: FnMut(Acc, Self::Item) -> Acc,
    {
        let mut accs: Vec<(K, Option<Acc>)> = Vec::new();
        for item in self {
            let key = key_fn(&item);
            if let Some(pos) = accs.iter().position(|(k, _)| *k == key) {
                let acc = accs[pos].1.take().expect("acc was None — internal bug");
                accs[pos].1 = Some(accum(acc, item));
            } else {
                let seed = seed_fn(&key);
                accs.push((key, Some(accum(seed, item))));
            }
        }
        accs.into_iter()
            .map(|(k, acc)| (k, acc.expect("acc was None — internal bug")))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // CONVERSION
    // ═══════════════════════════════════════════════════════════════════════

    /// Collects into a `HashMap` by a key selector. C# analogue: `ToDictionary`.
    #[must_use = "this consumes the iterator and allocates; if you only want the side effects, use `for_each_` instead"]
    fn into_hashmap<K, F>(self, key_fn: F) -> std::collections::HashMap<K, Self::Item>
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

    /// Builds a [`Lookup`] (one-to-many dictionary). C# analogue: `ToLookup`.
    fn into_lookup<K, F>(self, mut key_fn: F) -> Lookup<K, Self::Item>
    where
        K: Eq + std::hash::Hash + Clone,
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
}

// Blanket implementation — every `Iterator` gets all LINQ methods.
impl<I: Iterator> LinqExt for I {}
