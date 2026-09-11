//! # `linq_rs_sql` — a typed SQL query builder
//!
//! Compiles in-Rust column and predicate expressions into a SQL string plus
//! its bound parameters. Columns and their types are declared once with
//! [`table!`], and from then on the compiler checks them: a misspelt column or
//! a type mismatch is a build error, and every value you compare against
//! becomes a bound parameter rather than text spliced into a query.
//!
//! ```rust
//! use linq_rs_sql::*;
//!
//! linq_rs_sql::table! {
//!     employees (id) { id -> Integer, name -> Text, salary -> Integer }
//! }
//!
//! let q = employees::table()
//!     .filter(employees::salary.gt(100_000))
//!     .filter(employees::name.like("A%"))
//!     .order_by_desc(employees::salary)
//!     .limit(5)
//!     .to_sql();
//!
//! assert!(q.sql.starts_with("SELECT * FROM employees WHERE"));
//! assert_eq!(q.params.len(), 2);   // both values are bound, not interpolated
//! ```
//!
//! ## What this is not
//!
//! **It does not execute anything.** `to_sql()` hands you a `String` and a
//! `Vec<SqlValue>`; you pass those to whatever driver you already use —
//! `rusqlite`, `tokio-postgres`, `sqlx-core`. There is no connection, no
//! pool, no async runtime, and no dependency of any kind.
//!
//! **It is not `linq_rs`.** The sibling crate is a LINQ-shaped query surface
//! over in-memory iterators, and the two vocabularies are deliberately
//! separate: this one says `filter`, that one says `where_`, and no value
//! passes between them. Keeping them in one crate would have meant two names
//! for one concept in a single API — see [`DECISIONS.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/DECISIONS.md) `D-205` in the
//! repository. Joining them properly is the two-interpreter design tracked as
//! `D-002`, and this crate is intended to become its rendering backend.
//!
//! ## Phase 1 scope
//!
//! - `SELECT` with column lists or all-columns
//! - `WHERE` with `=`, `!=`, `<`, `<=`, `>`, `>=`, `AND`, `OR`, `NOT`, `LIKE`, `IS NULL`
//! - `ORDER BY` (ascending / descending)
//! - `LIMIT` / `OFFSET`
//! - Parameter binding via `?` placeholders
//! - Compile-time SQL type checking via marker types
//!
//! Out of scope for Phase 1 (in roadmap order): JOINs, subqueries, GROUP BY,
//! INSERT / UPDATE / DELETE, migrations, driver integration, async.
//!
//! ## Quick start
//!
//! ```rust
//! use linq_rs_sql::*;
//!
//! linq_rs_sql::table! {
//!     users (id) {
//!         id    -> Integer,
//!         name  -> Text,
//!         age   -> Integer,
//!     }
//! }
//!
//! let q = users::table()
//!     .filter(users::age.gt(18))
//!     .select((users::id, users::name))
//!     .to_sql();
//!
//! assert_eq!(q.sql, "SELECT id, name FROM users WHERE (age > ?)");
//! assert_eq!(q.params, vec![SqlValue::Integer(18)]);
//! ```

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
