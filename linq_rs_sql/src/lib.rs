//! The crate-level documentation is `README.md`, included below. It is not
//! duplicated here: a hand-maintained second description of the crate is
//! exactly the drift [`DECISIONS.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/DECISIONS.md) `D-016` exists to stop, and the block that
//! used to live here HAD drifted -- it described a crate with no nullable
//! columns, no conditional composition and no row materialization, months after
//! all three shipped.
//!
//! Including the README also makes its examples doctests, so it cannot claim
//! SQL the crate does not emit.
//!
//! Note the path: `include_str!` resolves relative to *this file*, so
//! `"README.md"` would look for `src/README.md` and fail to compile.
#![doc = include_str!("../README.md")]

#[macro_use]
pub mod pred;
pub mod boxed;
pub mod column;
pub mod expr;
pub mod from_row;
pub mod query;
pub mod rows;
pub mod types;
pub mod value;

mod macros;

pub use boxed::{boxed_query, Boxed, BoxedRowIter, BoxedRows, DynPred};
pub use column::{BoolOps, Column, ExprExt, FloatOps, IntOps, TextOps};
pub use expr::{
    not, And, Eq, Expr, Gt, GtEq, IsNotNull, IsNull, Like, Lit, Lt, LtEq, Not, NotEq, Or,
};
pub use from_row::{
    resolve_by_name, ColumnSet, FromRow, Layout, LoadField, LoadOpt, RowError, RowSource,
    SqlValueRef,
};
pub use query::{All, Direction, Named, Query, QueryOutput, Selection, Table};
pub use value::SqlValue;
// Re-export the SQL type markers and the sealed `SqlType` trait.
pub use types::{
    Boolean, CompareWith, Family, Float, Integer, LogicWith, Negate, Nullable, SqlType, Text,
    WhereClause,
};

/// Everything `pred!` and the builder need, in one import.
///
/// The comparison operators live on type-specific traits (`IntOps`, `TextOps`,
/// …) so that a `Text` column cannot be compared to an integer. They have to be
/// in scope for `.gt(..)` to resolve — and if they are not, the error is
/// confusing rather than helpful, because **`Iterator::gt` exists** and rustc
/// finds that instead:
///
/// ```text
/// error[E0599]: `salary` is not an iterator
///    method `gt` not found for this struct because it doesn't satisfy `salary: Iterator`
/// ```
///
/// So import the prelude:
///
/// ```rust
/// use linq_rs_sql::prelude::*;
/// ```
pub mod prelude {
    pub use crate::boxed::{boxed_query, Boxed, BoxedRows};
    pub use crate::column::{BoolOps, Column, ExprExt, FloatOps, IntOps, TextOps};
    pub use crate::expr::Expr;
    pub use crate::from_row::{ColumnSet, FromRow, Layout, RowError, RowSource, SqlValueRef};
    pub use crate::rows::{query, Entity, Rows};
    pub use crate::types::{Boolean, Float, Integer, Nullable, Text};
    pub use crate::value::SqlValue;
    pub use crate::{entity, pred, table};
}
