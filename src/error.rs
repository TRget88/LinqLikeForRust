//! The crate's only error type.
//!
//! It lives in its own module so that its trait impls sit *outside*
//! `queryable.rs`. `.github/scripts/gen-docs.py` derives the public `LinqExt`
//! surface by scraping method declarations, and an
//! `impl fmt::Display for SingleError { fn fmt(..) }` in that file sits at the
//! same indentation as a trait method. (The generator is now scoped to the
//! trait body so it would not be fooled, but keeping the type here means the
//! two concerns stay separated on their own merits.)

use core::fmt;

/// Why a sequence did not contain exactly one element.
///
/// Returned by [`LinqExt::try_single`](crate::LinqExt::try_single) and
/// [`LinqExt::single_or_default`](crate::LinqExt::single_or_default).
///
/// The two variants are the **complete** partition of "not exactly one": a
/// sequence has zero, one, or more than one element, and one is the success
/// case. No third failure mode can appear later, so this enum is deliberately
/// *not* `#[non_exhaustive]` — callers may match it exhaustively, and the
/// "adding a variant to a public enum is breaking" rule in the README's
/// versioning section costs nothing here, because there is no variant left to
/// add.
///
/// ```rust
/// use linq_rs::{LinqExt, SingleError};
///
/// assert_eq!(vec![7].into_iter().try_single(), Ok(7));
/// assert_eq!(Vec::<i32>::new().into_iter().try_single(), Err(SingleError::Empty));
/// assert_eq!(vec![1, 2].into_iter().try_single(), Err(SingleError::MoreThanOne));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SingleError {
    /// The sequence yielded no elements. Usually "not found" — recoverable.
    Empty,
    /// The sequence yielded two or more elements. Usually a broken uniqueness
    /// assumption in the caller — a bug, not a missing row.
    MoreThanOne,
}

impl SingleError {
    /// The message the panicking variants use.
    ///
    /// Single-sourced here so that [`single`](crate::LinqExt::single)'s
    /// documented panic text, its actual panic, and this type's `Display`
    /// cannot drift apart. Three copies of a string is exactly the shape of
    /// defect `DECISIONS.md` `D-016` exists to prevent.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            SingleError::Empty => "sequence contains no elements",
            SingleError::MoreThanOne => "sequence contains more than one element",
        }
    }
}

impl fmt::Display for SingleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for SingleError {}
