//! # `linq_rs::sql` — typed SQL query builder
//!
//! A Diesel-style query builder that compiles in-Rust column / predicate
//! expressions to a `(String, Vec<SqlValue>)` SQL fragment plus its
//! parameter binds.
//!
//! Unlike [`LinqExt`](crate::LinqExt) (which runs over in-memory iterators),
//! this module produces SQL strings — it does **not** execute against a
//! database. Pair it with whatever driver you already use (`rusqlite`,
//! `tokio-postgres`, `sqlx-core`, etc.).
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
//! use linq_rs::sql::*;
//!
//! linq_rs::table! {
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

pub mod column;
pub mod expr;
pub mod query;
pub mod types;
pub mod value;

mod macros;

pub use column::{BoolOps, Column, ExprExt, FloatOps, IntOps, TextOps};
pub use expr::{not, And, Eq, Expr, Gt, GtEq, IsNull, Like, Lit, Lt, LtEq, Not, NotEq, Or};
pub use query::{All, Direction, Query, QueryOutput, Selection, Table};
pub use value::SqlValue;
// Re-export the SQL type markers and the sealed `SqlType` trait.
pub use types::{Boolean, Float, Integer, SqlType, Text};
