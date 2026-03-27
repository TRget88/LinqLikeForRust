//! # linq_rs
//!
//! A LINQ-style query library for Rust. Extends any `Iterator` with familiar
//! C# LINQ methods — all lazy, zero-copy where possible, and with no external
//! dependencies.
//!
//! ## Quick start
//!
//! ```rust
//! use linq_rs::LinqExt;
//!
//! let result: Vec<_> = vec![1, 2, 3, 4, 5, 6]
//!     .into_iter()
//!     .where_(|x| x % 2 == 0)
//!     .select(|x| x * x)
//!     .to_vec();
//!
//! assert_eq!(result, vec![4, 16, 36]);
//! ```

pub mod adaptors;
pub mod grouping;
pub mod lookup;
pub mod ordered;
pub mod queryable;

pub use adaptors::*;
pub use grouping::Grouping;
pub use lookup::Lookup;
pub use ordered::{OrderedQueryable, ThenBy};
pub use queryable::LinqExt;
