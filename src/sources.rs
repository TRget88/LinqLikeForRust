//! Source generators — free functions that **create** iterators rather than
//! transforming an existing one.
//!
//! These mirror C#'s static `Enumerable.*` methods (`Range`, `Repeat`,
//! `Empty`) for parity. Each is a thin wrapper over `std::iter` and exists
//! mostly so C# devs can find them under familiar names.

/// Yields the integers `start, start + 1, ..., start + count - 1`.
/// Equivalent to C# `Enumerable.Range(start, count)`.
///
/// ```rust
/// let v: Vec<_> = linq_rs::range(1, 5).collect();
/// assert_eq!(v, [1, 2, 3, 4, 5]);
/// ```
pub fn range(start: i32, count: usize) -> impl Iterator<Item = i32> {
    (start..).take(count)
}

/// Yields `value`, `count` times. Equivalent to C# `Enumerable.Repeat(value, count)`.
///
/// ```rust
/// let v: Vec<_> = linq_rs::repeat("hi", 3).collect();
/// assert_eq!(v, ["hi", "hi", "hi"]);
/// ```
pub fn repeat<T: Clone>(value: T, count: usize) -> impl Iterator<Item = T> {
    std::iter::repeat(value).take(count)
}

/// Yields no elements. Equivalent to C# `Enumerable.Empty<T>()`.
///
/// ```rust
/// let v: Vec<i32> = linq_rs::empty().collect();
/// assert!(v.is_empty());
/// ```
pub fn empty<T>() -> impl Iterator<Item = T> {
    std::iter::empty()
}
