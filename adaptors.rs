//! Lazy iterator adaptors produced by the LINQ extension methods.
//!
//! All adaptors implement `Iterator` so they compose freely with each other
//! and with the standard library.

// ── Where ────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`where_`](crate::LinqExt::where_).
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

// ── SelectMany ───────────────────────────────────────────────────────────────

/// Iterator adaptor for [`select_many`](crate::LinqExt::select_many).
pub struct SelectMany<I, F, J>
where
    I: Iterator,
    F: FnMut(I::Item) -> J,
    J: Iterator,
{
    pub(crate) outer: I,
    pub(crate) f: F,
    pub(crate) current: Option<J>,
}

impl<I, F, J> Iterator for SelectMany<I, F, J>
where
    I: Iterator,
    F: FnMut(I::Item) -> J,
    J: Iterator,
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
            self.current = Some((self.f)(outer_item));
        }
    }
}

// ── Skip ─────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`skip`](crate::LinqExt::skip).
pub struct Skip<I> {
    pub(crate) inner: I,
    pub(crate) remaining: usize,
}

impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining > 0 {
            self.inner.next()?;
            self.remaining -= 1;
        }
        self.inner.next()
    }
}

// ── SkipWhile ────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`skip_while`](crate::LinqExt::skip_while_).
pub struct SkipWhile<I, P> {
    pub(crate) inner: I,
    pub(crate) predicate: P,
    pub(crate) done_skipping: bool,
}

impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for SkipWhile<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.inner.next()?;
            if self.done_skipping {
                return Some(item);
            }
            if !(self.predicate)(&item) {
                self.done_skipping = true;
                return Some(item);
            }
        }
    }
}

// ── Take ─────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`take`](crate::LinqExt::take_).
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
}

// ── TakeWhile ────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`take_while_`](crate::LinqExt::take_while_).
pub struct TakeWhile<I, P> {
    pub(crate) inner: I,
    pub(crate) predicate: P,
    pub(crate) done: bool,
}

impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for TakeWhile<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let item = self.inner.next()?;
        if (self.predicate)(&item) {
            Some(item)
        } else {
            self.done = true;
            None
        }
    }
}

// ── Distinct ─────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`distinct`](crate::LinqExt::distinct).
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

// ── Concat ───────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`concat`](crate::LinqExt::concat_).
pub struct Concat<I> {
    pub(crate) first: I,
    pub(crate) second: I,
    pub(crate) on_second: bool,
}

impl<I: Iterator> Iterator for Concat<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.on_second {
            if let Some(v) = self.first.next() {
                return Some(v);
            }
            self.on_second = true;
        }
        self.second.next()
    }
}

// ── Zip ──────────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`zip_`](crate::LinqExt::zip_).
pub struct Zip<I, J, F> {
    pub(crate) first: I,
    pub(crate) second: J,
    pub(crate) result_selector: F,
}

impl<I: Iterator, J: Iterator, R, F: FnMut(I::Item, J::Item) -> R> Iterator for Zip<I, J, F> {
    type Item = R;
    fn next(&mut self) -> Option<Self::Item> {
        let a = self.first.next()?;
        let b = self.second.next()?;
        Some((self.result_selector)(a, b))
    }
}

// ── Reverse ──────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`reverse`](crate::LinqExt::reverse).
pub struct Reverse<I: Iterator> {
    pub(crate) buffer: std::iter::Rev<std::vec::IntoIter<I::Item>>,
}

impl<I: Iterator> Iterator for Reverse<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.next()
    }
}

// ── Chunk / Batch ─────────────────────────────────────────────────────────────

/// Iterator adaptor for [`chunk`](crate::LinqExt::chunk).
pub struct Chunk<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) size: usize,
    pub(crate) done: bool,
}

impl<I: Iterator> Iterator for Chunk<I> {
    type Item = Vec<I::Item>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let mut batch = Vec::with_capacity(self.size);
        for _ in 0..self.size {
            match self.inner.next() {
                Some(v) => batch.push(v),
                None => {
                    self.done = true;
                    break;
                }
            }
        }
        if batch.is_empty() { None } else { Some(batch) }
    }
}

// ── Flatten ──────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`flatten_`](crate::LinqExt::flatten_).
pub struct Flatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    pub(crate) outer: I,
    pub(crate) current: Option<<I::Item as IntoIterator>::IntoIter>,
}

impl<I> Iterator for Flatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    type Item = <I::Item as IntoIterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut inner) = self.current {
                if let Some(v) = inner.next() {
                    return Some(v);
                }
            }
            let outer_item = self.outer.next()?;
            self.current = Some(outer_item.into_iter());
        }
    }
}
