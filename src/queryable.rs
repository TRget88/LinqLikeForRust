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

    /// Filter with access to the item's 0-based index. C# analogue: the
    /// C# `Where((item, index) => ...)` overload.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// // Keep items at even indices.
    /// let v: Vec<_> = vec!["a", "b", "c", "d", "e"]
    ///     .into_iter()
    ///     .where_indexed(|_, i| i % 2 == 0)
    ///     .collect();
    /// assert_eq!(v, ["a", "c", "e"]);
    /// ```
    fn where_indexed<P>(self, mut predicate: P) -> impl Iterator<Item = Self::Item>
    where
        P: FnMut(&Self::Item, usize) -> bool,
    {
        self.enumerate()
            .filter_map(move |(i, x)| if predicate(&x, i) { Some(x) } else { None })
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

    /// Project with access to the item's 0-based index. C# analogue: the
    /// C# `Select((item, index) => ...)` overload.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<_> = vec!["a", "b", "c"]
    ///     .into_iter()
    ///     .select_indexed(|s, i| format!("{i}:{s}"))
    ///     .collect();
    /// assert_eq!(v, ["0:a", "1:b", "2:c"]);
    /// ```
    fn select_indexed<B, F>(self, mut f: F) -> impl Iterator<Item = B>
    where
        F: FnMut(Self::Item, usize) -> B,
    {
        self.enumerate().map(move |(i, x)| f(x, i))
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
        J: Iterator,
    {
        SelectMany {
            outer: self,
            f,
            current: None,
        }
    }

    /// `select_many` with the item's 0-based index. C# analogue:
    /// `SelectMany((item, index) => ...)`.
    fn select_many_indexed<J, F>(self, mut f: F) -> impl Iterator<Item = J::Item>
    where
        F: FnMut(Self::Item, usize) -> J,
        J: IntoIterator,
    {
        self.enumerate().flat_map(move |(i, x)| f(x, i))
    }

    /// Flattens one level of nesting. C# analogue: `SelectMany(x => x)`.
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
        Flatten {
            outer: self,
            current: None,
        }
    }

    /// Yields only the elements that can be converted into `U` via
    /// [`TryInto`]; conversion failures are silently dropped.
    ///
    /// C# analogue: `OfType<U>()`. The natural Rust analogue of C#'s
    /// runtime type test — but here it goes through the type system via
    /// `TryInto`, so the compiler verifies that *some* conversion exists.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// // i64 -> i32: values that don't fit are dropped.
    /// let v: Vec<i32> = vec![1i64, 2, i64::MAX, 3].into_iter().of_type::<i32>().collect();
    /// assert_eq!(v, [1, 2, 3]);
    /// ```
    fn of_type<U>(self) -> impl Iterator<Item = U>
    where
        Self::Item: TryInto<U>,
    {
        self.filter_map(|x| x.try_into().ok())
    }

    /// Converts each element to `U` via [`TryInto`]. **Panics** on the first
    /// failed conversion. Named after C# `Cast<U>()`, but **this converts values** via `TryInto` where C# tests runtime types — `cast::<i64>()` widens an `i32` here, while C# `Cast<long>()` on a boxed `int` throws. (C# throws
    /// `InvalidCastException`). For the non-panicking version, see
    /// [`of_type`](Self::of_type).
    ///
    /// Lazy — the panic happens at iteration time, not when `cast` is called.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<i64> = vec![1i32, 2, 3].into_iter().cast::<i64>().collect();
    /// assert_eq!(v, [1i64, 2, 3]);
    /// ```
    fn cast<U>(self) -> impl Iterator<Item = U>
    where
        Self::Item: TryInto<U>,
        <Self::Item as TryInto<U>>::Error: std::fmt::Debug,
    {
        self.map(|x| x.try_into().expect("cast failed"))
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

    /// Skips elements while the predicate holds. C# analogue: `SkipWhile`.
    fn skip_while_<P>(self, predicate: P) -> SkipWhile<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        SkipWhile {
            inner: self,
            predicate,
            done_skipping: false,
        }
    }

    /// `skip_while_` with the item's 0-based index. C# analogue:
    /// `SkipWhile((item, index) => ...)`.
    fn skip_while_indexed<P>(self, mut predicate: P) -> impl Iterator<Item = Self::Item>
    where
        P: FnMut(&Self::Item, usize) -> bool,
    {
        self.enumerate()
            .skip_while(move |(i, x)| predicate(x, *i))
            .map(|(_, x)| x)
    }

    /// Takes at most `n` elements. C# analogue: `Take`.
    fn take_(self, n: usize) -> Take<Self> {
        Take {
            inner: self,
            remaining: n,
        }
    }

    /// Takes elements while the predicate holds. C# analogue: `TakeWhile`.
    fn take_while_<P>(self, predicate: P) -> TakeWhile<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        TakeWhile {
            inner: self,
            predicate,
            done: false,
        }
    }

    /// `take_while_` with the item's 0-based index. C# analogue:
    /// `TakeWhile((item, index) => ...)`.
    fn take_while_indexed<P>(self, mut predicate: P) -> impl Iterator<Item = Self::Item>
    where
        P: FnMut(&Self::Item, usize) -> bool,
    {
        self.enumerate()
            .take_while(move |(i, x)| predicate(x, *i))
            .map(|(_, x)| x)
    }

    /// Splits the sequence into fixed-size chunks. The last chunk may be
    /// shorter.
    ///
    /// # Panics
    ///
    /// Panics if `size` is zero. (C# `Chunk` throws
    /// `ArgumentOutOfRangeException` here; a panic is the Rust equivalent, and
    /// matches `slice::chunks`.) A very large `size` does **not** allocate
    /// eagerly — capacity is taken from what the source can supply.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let chunks: Vec<_> = (1..=7).chunk(3).collect();
    /// assert_eq!(chunks, [vec![1,2,3], vec![4,5,6], vec![7]]);
    /// ```
    fn chunk(self, size: usize) -> Chunk<Self> {
        assert!(size > 0, "chunk size must be > 0");
        Chunk {
            inner: self,
            size,
            done: false,
        }
    }

    /// Yields all elements except the last `n`. C# analogue: `SkipLast`.
    ///
    /// Lazy — uses a ring buffer of size `n`. If the source has fewer than
    /// `n` items, yields nothing.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let result: Vec<_> = (1..=5).skip_last(2).collect();
    /// assert_eq!(result, [1, 2, 3]);
    /// ```
    fn skip_last(self, n: usize) -> SkipLast<Self> {
        SkipLast {
            inner: self,
            buffer: std::collections::VecDeque::with_capacity(n),
            n,
        }
    }

    /// Yields only the last `n` elements. C# analogue: `TakeLast`.
    ///
    /// Eager — must consume the source to find the tail. If `n` is `0`,
    /// the source is not iterated.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let result: Vec<_> = (1..=5).take_last(2).collect();
    /// assert_eq!(result, [4, 5]);
    /// ```
    #[must_use = "this consumes the iterator and allocates; if you only want the side effects, use `for_each_` instead"]
    fn take_last(self, n: usize) -> std::vec::IntoIter<Self::Item> {
        if n == 0 {
            return Vec::new().into_iter();
        }
        let mut buf: std::collections::VecDeque<Self::Item> =
            std::collections::VecDeque::with_capacity(n);
        for item in self {
            if buf.len() == n {
                buf.pop_front();
            }
            buf.push_back(item);
        }
        let v: Vec<Self::Item> = buf.into_iter().collect();
        v.into_iter()
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

    /// Concatenates two sequences. C# analogue: `Concat`.
    fn concat_<I2>(self, other: I2) -> Concat<Self>
    where
        I2: IntoIterator<Item = Self::Item, IntoIter = Self>,
    {
        Concat {
            first: self,
            second: other.into_iter(),
            on_second: false,
        }
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
    /// use linq_rs::{LinqExt, ThenBy};
    /// let sorted: Vec<_> = vec!["banana","apple","cherry"]
    ///     .into_iter()
    ///     .order_by(|s| *s)
    ///     .into_iter()
    ///     .collect();
    /// assert_eq!(sorted, ["apple", "banana", "cherry"]);
    /// ```
    fn order_by<K, F>(self, key_fn: F) -> OrderedQueryable<Self::Item>
    where
        K: Ord,
        F: Fn(&Self::Item) -> K + 'static,
        Self::Item: 'static,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(move |a, b| key_fn(a).cmp(&key_fn(b))))
    }

    /// Sorts in descending order. C# analogue: `OrderByDescending`.
    fn order_by_descending<K, F>(self, key_fn: F) -> OrderedQueryable<Self::Item>
    where
        K: Ord,
        F: Fn(&Self::Item) -> K + 'static,
        Self::Item: 'static,
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
    ///     .into_iter()
    ///     .collect();
    /// assert_eq!(sorted, [1, 1, 2, 3, 4, 5, 9]);
    /// ```
    fn order(self) -> OrderedQueryable<Self::Item>
    where
        Self::Item: Ord + 'static,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(|a, b| a.cmp(b)))
    }

    /// Sorts the sequence in descending order using `Ord` on the elements
    /// themselves. C# analogue: `OrderDescending()` (.NET 6+).
    fn order_descending(self) -> OrderedQueryable<Self::Item>
    where
        Self::Item: Ord + 'static,
    {
        let data: Vec<_> = self.collect();
        OrderedQueryable::new(data, Box::new(|a, b| b.cmp(a)))
    }

    /// Reverses the sequence. C# analogue: `Reverse`.
    fn reverse(self) -> Reverse<Self> {
        let buffer: Vec<_> = self.collect();
        Reverse {
            buffer: buffer.into_iter().rev(),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // AGGREGATION
    // ═══════════════════════════════════════════════════════════════════════

    /// Applies an accumulator function over the sequence with an explicit seed.
    /// C# analogue: `Aggregate(seed, func)`. (The seedless `Aggregate(func)` overload is `reduce_`, not this.)
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let sum = (1..=5).aggregate(0, |acc, x| acc + x);
    /// assert_eq!(sum, 15);
    /// ```
    #[must_use]
    fn aggregate<Acc, F>(self, seed: Acc, f: F) -> Acc
    where
        F: FnMut(Acc, Self::Item) -> Acc,
    {
        self.fold(seed, f)
    }

    /// Applies an accumulator function over the sequence, using the first
    /// element as the seed. Returns `None` if the sequence is empty.
    ///
    /// C# analogue: `Aggregate(func)`, the no-seed overload — but C# throws on an empty sequence and this returns `None`. Alias
    /// for [`Iterator::reduce`].
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let sum = (1..=5).reduce_(|acc, x| acc + x);
    /// assert_eq!(sum, Some(15));
    /// ```
    #[must_use]
    fn reduce_<F>(self, f: F) -> Option<Self::Item>
    where
        F: FnMut(Self::Item, Self::Item) -> Self::Item,
    {
        self.reduce(f)
    }

    /// Applies an accumulator function with an explicit seed, then projects
    /// the final accumulator through `result_selector`. C# analogue:
    /// `Aggregate(seed, func, resultSelector)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let doubled_sum: i32 = (1..=5).aggregate_with_selector(
    ///     0i32,
    ///     |acc, x| acc + x,
    ///     |sum| sum * 2,
    /// );
    /// assert_eq!(doubled_sum, 30);
    /// ```
    #[must_use]
    fn aggregate_with_selector<Acc, R, F, S>(self, seed: Acc, f: F, result_selector: S) -> R
    where
        F: FnMut(Acc, Self::Item) -> Acc,
        S: FnOnce(Acc) -> R,
    {
        result_selector(self.fold(seed, f))
    }

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

    /// Returns the element at `index`, or `None`. C# analogue: `ElementAtOrDefault`.
    #[must_use]
    fn element_at(self, index: usize) -> Option<Self::Item> {
        Iterator::skip(self, index).next()
    }

    /// Returns the element at `index`. **Panics** if the index is out of
    /// bounds. C# analogue: `ElementAt(index)`.
    ///
    /// For a non-panicking version, see [`element_at`](Self::element_at).
    #[must_use]
    fn element_at_strict(self, index: usize) -> Self::Item {
        Iterator::skip(self, index)
            .next()
            .expect("index out of bounds")
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

    /// Returns the element at `index`, or `default` if out of bounds.
    /// **No C# counterpart.** `ElementAtOrDefault` has only `(Index)` and `(Int32)` overloads; neither takes a fallback value.
    #[must_use]
    fn element_at_or(self, index: usize, default: Self::Item) -> Self::Item {
        Iterator::skip(self, index).next().unwrap_or(default)
    }

    /// If the sequence is empty, yields `default` once; otherwise yields all
    /// elements unchanged. C# analogue: `DefaultIfEmpty(value)`.
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<i32> = Vec::<i32>::new().into_iter().default_if_empty(99).collect();
    /// assert_eq!(v, [99]);
    ///
    /// let v: Vec<i32> = vec![1, 2].into_iter().default_if_empty(99).collect();
    /// assert_eq!(v, [1, 2]);
    /// ```
    fn default_if_empty(self, default: Self::Item) -> DefaultIfEmpty<Self> {
        DefaultIfEmpty {
            inner: self,
            default: Some(default),
            yielded_anything: false,
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
    /// groups.sort_by_key(|g| g.key);
    /// assert_eq!(groups[0].key, 'a');
    /// assert_eq!(groups[0].elements, ["apple", "ant"]);
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
            if let Some(g) = groups.iter_mut().find(|g| g.key == key) {
                g.elements.push(element);
            } else {
                let mut g = Grouping::new(key);
                g.elements.push(element);
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
            if let Some(g) = groups.iter_mut().find(|g| g.key == key) {
                g.elements.push(item);
            } else {
                let mut g = Grouping::new(key);
                g.elements.push(item);
                groups.push(g);
            }
        }
        groups
            .into_iter()
            .map(move |g| result_fn(g.key, g.elements))
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
                Some(&pos) => groups[pos].elements.push(item),
                None => {
                    let pos = groups.len();
                    index.insert(key.clone(), pos);
                    let mut g = Grouping::new(key);
                    g.elements.push(item);
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

    /// Collects into a `Vec`. C# analogue: `ToList`.
    #[must_use = "this consumes the iterator and allocates; if you only want the side effects, use `for_each_` instead"]
    fn to_vec(self) -> Vec<Self::Item> {
        self.collect()
    }

    /// Collects into a `HashMap` by a key selector. C# analogue: `ToDictionary`.
    #[must_use = "this consumes the iterator and allocates; if you only want the side effects, use `for_each_` instead"]
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

    /// Collects into a `HashSet`. C# analogue: `ToHashSet`.
    #[must_use = "this consumes the iterator and allocates; if you only want the side effects, use `for_each_` instead"]
    fn to_hashset(self) -> std::collections::HashSet<Self::Item>
    where
        Self::Item: std::hash::Hash + Eq,
    {
        self.collect()
    }

    /// Builds a [`Lookup`] (one-to-many dictionary). C# analogue: `ToLookup`.
    fn to_lookup<K, F>(self, mut key_fn: F) -> Lookup<K, Self::Item>
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

    /// Merges two sequences element-by-element using a result selector.
    /// C# analogue: `Zip(second, resultSelector)`.
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

    /// Merges three sequences element-by-element using a three-arg result
    /// selector. Stops as soon as any of the three is exhausted.
    /// C# analogue: `Zip(second, third, resultSelector)` (.NET 6+).
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<_> = vec![1, 2, 3].into_iter()
    ///     .zip3(vec![10, 20, 30], vec![100, 200, 300], |a, b, c| a + b + c)
    ///     .collect();
    /// assert_eq!(v, [111, 222, 333]);
    /// ```
    fn zip3<J, K, R, F>(
        self,
        second: J,
        third: K,
        mut result_selector: F,
    ) -> impl Iterator<Item = R>
    where
        J: IntoIterator,
        K: IntoIterator,
        F: FnMut(Self::Item, J::Item, K::Item) -> R,
    {
        let mut s = second.into_iter();
        let mut t = third.into_iter();
        self.map_while(move |a| {
            let b = s.next()?;
            let c = t.next()?;
            Some(result_selector(a, b, c))
        })
    }

    /// Applies an action to each element (for side effects). C# analogue: `ForEach` / `Do`.
    fn for_each_<F>(self, f: F)
    where
        F: FnMut(Self::Item),
    {
        self.for_each(f)
    }

    /// Appends a single element to the end of the sequence. C# analogue: `Append`.
    fn append_item(self, item: Self::Item) -> impl Iterator<Item = Self::Item> {
        self.chain(std::iter::once(item))
    }

    /// Prepends a single element to the front of the sequence. C# analogue: `Prepend`.
    fn prepend_item(self, item: Self::Item) -> impl Iterator<Item = Self::Item> {
        std::iter::once(item).chain(self)
    }

    /// Returns `true` if the sequence contains no elements. C# analogue: `!Any()`.
    ///
    /// `is_empty_` consumes the iterator like other LINQ terminal ops; clippy's
    /// `wrong_self_convention` lint expects `is_*` methods to take `&self`,
    /// but here the consumption is intentional and consistent with `any_` /
    /// `all_` / `count_where`.
    #[allow(clippy::wrong_self_convention)]
    #[must_use]
    fn is_empty_(mut self) -> bool {
        self.next().is_none()
    }

    /// Yields `(index, item)` pairs. C# analogue: .NET 9+ `Index()`; an
    /// alias for [`Iterator::enumerate`].
    ///
    /// ```rust
    /// use linq_rs::LinqExt;
    /// let v: Vec<_> = vec!["a", "b", "c"].into_iter().index_().collect();
    /// assert_eq!(v, [(0, "a"), (1, "b"), (2, "c")]);
    /// ```
    fn index_(self) -> std::iter::Enumerate<Self> {
        self.enumerate()
    }

    /// Sequence equality — two sequences are equal if they yield the same
    /// elements in the same order. C# analogue: `SequenceEqual`.
    #[must_use]
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
