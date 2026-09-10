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

// ── Distinct ─────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`distinct`](crate::LinqExt::distinct).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Distinct<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    pub(crate) inner: I,
    pub(crate) seen: Vec<I::Item>,
}

impl<I> Iterator for Distinct<I>
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

// ── DistinctBy ───────────────────────────────────────────────────────────────

/// Iterator adaptor for [`distinct_by`](crate::LinqExt::distinct_by).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct DistinctBy<I, F, K>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
    pub(crate) inner: I,
    pub(crate) key_fn: F,
    pub(crate) seen_keys: Vec<K>,
}

impl<I, F, K> Iterator for DistinctBy<I, F, K>
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
impl<I> FusedIterator for Distinct<I>
where
    I: FusedIterator,
    I::Item: PartialEq + Clone,
{
}
impl<I, F, K> FusedIterator for DistinctBy<I, F, K>
where
    I: FusedIterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
}
