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

impl<I: DoubleEndedIterator, B, F: FnMut(I::Item) -> B> DoubleEndedIterator for Select<I, F> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(&mut self.f)
    }
}

impl<I: ExactSizeIterator, B, F: FnMut(I::Item) -> B> ExactSizeIterator for Select<I, F> {}

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
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self.inner.size_hint();
        (
            low.saturating_sub(self.remaining),
            high.map(|h| h.saturating_sub(self.remaining)),
        )
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {}

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
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.on_second {
            self.second.size_hint()
        } else {
            let (al, ah) = self.first.size_hint();
            let (bl, bh) = self.second.size_hint();
            let low = al.saturating_add(bl);
            let high = match (ah, bh) {
                (Some(a), Some(b)) => a.checked_add(b),
                _ => None,
            };
            (low, high)
        }
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Concat<I> {}

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
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (al, ah) = self.first.size_hint();
        let (bl, bh) = self.second.size_hint();
        let low = al.min(bl);
        let high = match (ah, bh) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        (low, high)
    }
}

impl<I: ExactSizeIterator, J: ExactSizeIterator, R, F: FnMut(I::Item, J::Item) -> R>
    ExactSizeIterator for Zip<I, J, F>
{
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
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.buffer.size_hint()
    }
}

impl<I: Iterator> DoubleEndedIterator for Reverse<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.buffer.next_back()
    }
}

impl<I: Iterator> ExactSizeIterator for Reverse<I> {}

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
        if batch.is_empty() {
            None
        } else {
            Some(batch)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done {
            return (0, Some(0));
        }
        let (low, high) = self.inner.size_hint();
        (low.div_ceil(self.size), high.map(|h| h.div_ceil(self.size)))
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

// ── DefaultIfEmpty ───────────────────────────────────────────────────────────

/// Iterator adaptor for [`default_if_empty`](crate::LinqExt::default_if_empty).
pub struct DefaultIfEmpty<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) default: Option<I::Item>,
    pub(crate) yielded_anything: bool,
}

impl<I: Iterator> Iterator for DefaultIfEmpty<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(v) => {
                self.yielded_anything = true;
                self.default = None;
                Some(v)
            }
            None => {
                if !self.yielded_anything {
                    self.yielded_anything = true;
                    self.default.take()
                } else {
                    None
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self.inner.size_hint();
        if self.yielded_anything {
            (low, high)
        } else {
            // We'll yield at least 1 if inner is empty (the default).
            (low.max(1), high.map(|h| h.max(1)))
        }
    }
}

// ── SkipLast ─────────────────────────────────────────────────────────────────

/// Iterator adaptor for [`skip_last`](crate::LinqExt::skip_last).
///
/// Holds a ring buffer of size `n`. Yielded items are always at least `n`
/// steps behind the source, so the trailing `n` items are dropped on the floor.
pub struct SkipLast<I: Iterator> {
    pub(crate) inner: I,
    pub(crate) buffer: std::collections::VecDeque<I::Item>,
    pub(crate) n: usize,
}

impl<I: Iterator> Iterator for SkipLast<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            return self.inner.next();
        }
        while self.buffer.len() < self.n {
            match self.inner.next() {
                Some(v) => self.buffer.push_back(v),
                None => return None,
            }
        }
        match self.inner.next() {
            Some(v) => {
                self.buffer.push_back(v);
                self.buffer.pop_front()
            }
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        // yet-to-yield = (items in buffer + items remaining in inner) - n
        let (low, high) = self.inner.size_hint();
        let buf_len = self.buffer.len();
        let n = self.n;
        let map_low = (buf_len + low).saturating_sub(n);
        let map_high = high.map(|h| (buf_len + h).saturating_sub(n));
        (map_low, map_high)
    }
}
