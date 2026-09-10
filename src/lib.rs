//! The crate-level documentation is `README.md`, included below. It is not
//! duplicated here: a hand-maintained second description of the crate is
//! exactly the drift `DECISIONS.md` `D-016` exists to stop, and the block
//! that used to live here had already drifted — it claimed the operators are
//! "all lazy", which `group_by_key`, `union_`, `inner_join` and `group_join`
//! disprove.
//!
//! Note the path: `include_str!` resolves relative to *this file*, so
//! `"README.md"` would look for `src/README.md` and fail to compile.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod adaptors;
pub mod grouping;
pub mod lookup;
pub mod ordered;
pub mod queryable;
pub mod sources;

pub use adaptors::*;
pub use grouping::Grouping;
pub use lookup::Lookup;
pub use ordered::{OrderedQueryable, ThenBy};
pub use queryable::LinqExt;
pub use sources::{empty, range, repeat};
