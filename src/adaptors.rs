//! Lazy iterator adaptors produced by the LINQ extension methods.
//!
//! All adaptors implement `Iterator` so they compose freely with each other
//! and with the standard library.

// ── Where ────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`where_`](crate::LinqExt::where_).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Where<I, P> {
    pub(crate) inner: I,
    pub(crate) predicate: P,
}

impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for Where<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.inner.next()?;
            if (self.predicate)(&item) {
                return Some(item);
            }
        }
    }
}

// ── Select ───────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`select`](crate::LinqExt::select).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Select<I, F> {
    pub(crate) inner: I,
    pub(crate) f: F,
}

impl<I: Iterator, B, F: FnMut(I::Item) -> B> Iterator for Select<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(&mut self.f)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I: DoubleEndedIterator, B, F: FnMut(I::Item) -> B> DoubleEndedIterator for Select<I, F> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(&mut self.f)
    }
}

impl<I: ExactSizeIterator, B, F: FnMut(I::Item) -> B> ExactSizeIterator for Select<I, F> {}

// ── SelectMany ───────────────────────────────────────────────────────────────

/// Iterator adaptor for [`select_many`](crate::LinqExt::select_many).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct SelectMany<I, F, J>
where
    I: Iterator,
    F: FnMut(I::Item) -> J,
    J: IntoIterator,
{
    pub(crate) outer: I,
    pub(crate) f: F,
    pub(crate) current: Option<J::IntoIter>,
}

impl<I, F, J> Iterator for SelectMany<I, F, J>
where
    I: Iterator,
    F: FnMut(I::Item) -> J,
    J: IntoIterator,
{
    type Item = J::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut inner) = self.current {
                if let Some(val) = inner.next() {
                    return Some(val);
                }
            }
            let outer_item = self.outer.next()?;
            self.current = Some((self.f)(outer_item).into_iter());
        }
    }
}

// ── Skip ─────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`skip_`](crate::LinqExt::skip_).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Skip<I> {
    pub(crate) inner: I,
    pub(crate) remaining: usize,
}

impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        // `mem::take` before skipping, exactly as `std::iter::Skip` does. The
        // previous form decremented inside the loop and used `?`, so a source
        // that returned `None` mid-skip left `remaining > 0` — and on the next
        // call it would skip all over again. On a non-fused source that
        // silently ate an element: given 10,20,None,40,50,60 driven by hand,
        // std yielded 40 and this yielded 50. `collect()` masks it, because it
        // stops at the first `None`.
        if self.remaining > 0 {
            self.inner.nth(std::mem::take(&mut self.remaining) - 1)?;
        }
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self.inner.size_hint();
        (
            low.saturating_sub(self.remaining),
            high.map(|h| h.saturating_sub(self.remaining)),
        )
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {}

// ── Take ─────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`take`](crate::LinqExt::take_).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Take<I> {
    pub(crate) inner: I,
    pub(crate) remaining: usize,
}

impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self.inner.size_hint();
        let low = low.min(self.remaining);
        let high = match high {
            Some(h) => Some(h.min(self.remaining)),
            None => Some(self.remaining),
        };
        (low, high)
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Take<I> {}

// ── DistinctPartialEq ─────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`distinct`](crate::LinqExt::distinct).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct DistinctPartialEq<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    pub(crate) inner: I,
    pub(crate) seen: Vec<I::Item>,
}

impl<I> Iterator for DistinctPartialEq<I>
where
    I: Iterator,
    I::Item: PartialEq + Clone,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.inner.next()?;
            if !self.seen.contains(&item) {
                self.seen.push(item.clone());
                return Some(item);
            }
        }
    }
}

// ── DistinctByPartialEq ───────────────────────────────────────────────────────────────

/// Iterator adaptor for [`distinct_by`](crate::LinqExt::distinct_by).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct DistinctByPartialEq<I, F, K>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
    pub(crate) inner: I,
    pub(crate) key_fn: F,
    pub(crate) seen_keys: Vec<K>,
}

impl<I, F, K> Iterator for DistinctByPartialEq<I, F, K>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.inner.next()?;
            let key = (self.key_fn)(&item);
            if !self.seen_keys.contains(&key) {
                self.seen_keys.push(key);
                return Some(item);
            }
        }
    }
}

// ── FusedIterator ────────────────────────────────────────────────────────────
//
// `Iterator` does not promise that `None` is final; only `FusedIterator` does.
// Without these impls a downstream `.fuse()`-bounded API rejects every adaptor
// in this crate, and — as the `Skip` bug showed — it is easy to write an
// adaptor that quietly misbehaves on a source that resumes after `None`.
//
// Most are conditional on the inner iterator: they are fused exactly when their
// source is. Two are unconditional and say so.

use std::iter::FusedIterator;

impl<I: FusedIterator, P: FnMut(&I::Item) -> bool> FusedIterator for Where<I, P> {}
impl<I: FusedIterator, B, F: FnMut(I::Item) -> B> FusedIterator for Select<I, F> {}
impl<I: FusedIterator> FusedIterator for Skip<I> {}
impl<I: FusedIterator> FusedIterator for Take<I> {}
// These two back the `*_partial_eq` escape hatches, so their bounds are
// `PartialEq`, matching their `Iterator` impls above.
impl<I> FusedIterator for DistinctPartialEq<I>
where
    I: FusedIterator,
    I::Item: PartialEq + Clone,
{
}
impl<I, F, K> FusedIterator for DistinctByPartialEq<I, F, K>
where
    I: FusedIterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
}

// ═════════════════════════════════════════════════════════════════════════════
// D-106 — named returns for all 23 sites that returned `impl Iterator`.
// ═════════════════════════════════════════════════════════════════════════════

/// Newtype over `vec::IntoIter` for the operators that fully materialise.
/// Because they are eager, every closure is consumed at construction time and
/// **none of them appears in the public type**.
macro_rules! eager_vec_adaptor {
    ($name:ident<$($g:ident),*>, $item:ty, $doc:expr) => {
        #[doc = $doc]
        #[must_use]
        pub struct $name<$($g),*> {
            pub(crate) inner: std::vec::IntoIter<$item>,
        }
        impl<$($g),*> Iterator for $name<$($g),*> {
            type Item = $item;
            fn next(&mut self) -> Option<$item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }
        impl<$($g),*> DoubleEndedIterator for $name<$($g),*> {
            fn next_back(&mut self) -> Option<$item> { self.inner.next_back() }
        }
        impl<$($g),*> ExactSizeIterator for $name<$($g),*> {}
        impl<$($g),*> FusedIterator for $name<$($g),*> {}
    };
}

/// Same, over `hash_map::IntoIter` — fused and exact-size, but **not**
/// double-ended, which is why these cannot share the macro above.
macro_rules! eager_map_adaptor {
    ($name:ident<$k:ident, $v:ident>, $doc:expr) => {
        #[doc = $doc]
        #[must_use]
        pub struct $name<$k, $v> {
            pub(crate) inner: std::collections::hash_map::IntoIter<$k, $v>,
        }
        impl<$k, $v> Iterator for $name<$k, $v> {
            type Item = ($k, $v);
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }
        impl<$k, $v> ExactSizeIterator for $name<$k, $v> {}
        impl<$k, $v> FusedIterator for $name<$k, $v> {}
    };
}

eager_vec_adaptor!(
    UnionPartialEq<T>,
    T,
    "Adaptor for [`union_partial_eq`](crate::LinqExt::union_partial_eq)."
);
eager_vec_adaptor!(
    UnionBy<T>,
    T,
    "Adaptor for [`union_by`](crate::LinqExt::union_by)."
);
eager_vec_adaptor!(
    InnerJoin<R>,
    R,
    "Adaptor for [`inner_join`](crate::LinqExt::inner_join)."
);
eager_vec_adaptor!(
    InnerJoinPartialEq<R>,
    R,
    "Adaptor for [`inner_join_partial_eq`](crate::LinqExt::inner_join_partial_eq)."
);
eager_vec_adaptor!(
    GroupJoin<R>,
    R,
    "Adaptor for [`group_join`](crate::LinqExt::group_join)."
);
eager_vec_adaptor!(
    GroupJoinPartialEq<R>,
    R,
    "Adaptor for [`group_join_partial_eq`](crate::LinqExt::group_join_partial_eq)."
);
eager_vec_adaptor!(
    GroupByWithResult<R>,
    R,
    "Adaptor for [`group_by_with_result`](crate::LinqExt::group_by_with_result)."
);
eager_vec_adaptor!(
    CountByPartialEq<K>,
    (K, usize),
    "Adaptor for [`count_by_partial_eq`](crate::LinqExt::count_by_partial_eq)."
);
eager_vec_adaptor!(AggregateByPartialEq<K, Acc>, (K, Acc), "Adaptor for [`aggregate_by_partial_eq`](crate::LinqExt::aggregate_by_partial_eq).");
eager_vec_adaptor!(GroupByKey<K, T>, crate::grouping::Grouping<K, T>, "Adaptor for [`group_by_key`](crate::LinqExt::group_by_key).");
eager_vec_adaptor!(GroupByKeyPartialEq<K, T>, crate::grouping::Grouping<K, T>, "Adaptor for [`group_by_key_partial_eq`](crate::LinqExt::group_by_key_partial_eq).");
eager_vec_adaptor!(GroupByWithElement<K, E>, crate::grouping::Grouping<K, E>, "Adaptor for [`group_by_with_element`](crate::LinqExt::group_by_with_element).");

/// Adaptor for [`count_by`](crate::LinqExt::count_by).
#[must_use]
pub struct CountBy<K> {
    pub(crate) inner: std::collections::hash_map::IntoIter<K, usize>,
}
impl<K> Iterator for CountBy<K> {
    type Item = (K, usize);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K> ExactSizeIterator for CountBy<K> {}
impl<K> FusedIterator for CountBy<K> {}
eager_map_adaptor!(AggregateBy<K, Acc>, "Adaptor for [`aggregate_by`](crate::LinqExt::aggregate_by).");

// ── Lazy, hash-backed: state that used to live in a captured closure ─────────

/// Adaptor for [`distinct`](crate::LinqExt::distinct).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Distinct<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) seen: std::collections::HashSet<I::Item>,
}
impl<I> Iterator for Distinct<I>
where
    I: Iterator,
    I::Item: Eq + std::hash::Hash + Clone,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if self.seen.insert(x.clone()) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I> FusedIterator for Distinct<I>
where
    I: Iterator + FusedIterator,
    I::Item: Eq + std::hash::Hash + Clone,
{
}

/// Adaptor for [`distinct_by`](crate::LinqExt::distinct_by).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct DistinctBy<I, F, K> {
    pub(crate) inner: I,
    pub(crate) key_fn: F,
    pub(crate) seen: std::collections::HashSet<K>,
}
impl<I, F, K> Iterator for DistinctBy<I, F, K>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: Eq + std::hash::Hash,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if self.seen.insert((self.key_fn)(&x)) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I, F, K> FusedIterator for DistinctBy<I, F, K>
where
    I: FusedIterator,
    F: FnMut(&I::Item) -> K,
    K: Eq + std::hash::Hash,
{
}

/// Adaptor for [`except`](crate::LinqExt::except).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Except<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) exclusions: std::collections::HashSet<I::Item>,
}
impl<I> Iterator for Except<I>
where
    I: Iterator,
    I::Item: Eq + std::hash::Hash,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if !self.exclusions.contains(&x) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I> FusedIterator for Except<I>
where
    I: Iterator + FusedIterator,
    I::Item: Eq + std::hash::Hash,
{
}

/// Adaptor for [`intersect`](crate::LinqExt::intersect).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Intersect<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) inclusions: std::collections::HashSet<I::Item>,
}
impl<I> Iterator for Intersect<I>
where
    I: Iterator,
    I::Item: Eq + std::hash::Hash,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if self.inclusions.contains(&x) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I> FusedIterator for Intersect<I>
where
    I: Iterator + FusedIterator,
    I::Item: Eq + std::hash::Hash,
{
}

/// Adaptor for [`union_`](crate::LinqExt::union_).
///
/// Two iterator parameters: the second is `I2::IntoIter`, which is how a
/// lazy two-source operator has to spell its other side.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Union<I: Iterator, J> {
    pub(crate) inner: std::iter::Chain<I, J>,
    pub(crate) seen: std::collections::HashSet<I::Item>,
}
impl<I, J> Iterator for Union<I, J>
where
    I: Iterator,
    J: Iterator<Item = I::Item>,
    I::Item: Eq + std::hash::Hash + Clone,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if self.seen.insert(x.clone()) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I, J> FusedIterator for Union<I, J>
where
    I: FusedIterator,
    J: FusedIterator<Item = I::Item>,
    I::Item: Eq + std::hash::Hash + Clone,
{
}

// ── Lazy, PartialEq escape hatches ───────────────────────────────────────────

/// Adaptor for [`except_partial_eq`](crate::LinqExt::except_partial_eq).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExceptPartialEq<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) exclusions: Vec<I::Item>,
}
impl<I> Iterator for ExceptPartialEq<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if !self.exclusions.contains(&x) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I> FusedIterator for ExceptPartialEq<I>
where
    I: FusedIterator,
    I::Item: PartialEq,
{
}

/// Adaptor for [`intersect_partial_eq`](crate::LinqExt::intersect_partial_eq).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IntersectPartialEq<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) inclusions: Vec<I::Item>,
}
impl<I> Iterator for IntersectPartialEq<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if self.inclusions.contains(&x) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I> FusedIterator for IntersectPartialEq<I>
where
    I: FusedIterator,
    I::Item: PartialEq,
{
}

/// Adaptor for [`except_by`](crate::LinqExt::except_by).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExceptBy<I, F, K> {
    pub(crate) inner: I,
    pub(crate) key_fn: F,
    pub(crate) exclusions: Vec<K>,
}
impl<I, F, K> Iterator for ExceptBy<I, F, K>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if !self.exclusions.contains(&(self.key_fn)(&x)) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I, F, K> FusedIterator for ExceptBy<I, F, K>
where
    I: FusedIterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
}

/// Adaptor for [`intersect_by`](crate::LinqExt::intersect_by).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IntersectBy<I, F, K> {
    pub(crate) inner: I,
    pub(crate) key_fn: F,
    pub(crate) inclusions: Vec<K>,
}
impl<I, F, K> Iterator for IntersectBy<I, F, K>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.inner.next()?;
            if self.inclusions.contains(&(self.key_fn)(&x)) {
                return Some(x);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}
impl<I, F, K> FusedIterator for IntersectBy<I, F, K>
where
    I: FusedIterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
}
