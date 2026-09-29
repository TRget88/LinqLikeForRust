//! Runtime SQL values. These are what end up in the `params` vec returned
//! by [`Query::to_sql`](crate::Query::to_sql) — they're the bind
//! values the user will pass to a driver alongside the SQL string.

/// A SQL value at runtime, suitable for binding as a `?` placeholder.
///
/// Phase 1 keeps this small (the five SQL types we expose at the type
/// level). Add new variants alongside new `SqlType` markers when extending.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    /// A SQL `INTEGER` value, normalized to `i64`.
    Integer(i64),
    /// A SQL `TEXT` value.
    Text(String),
    /// A SQL `BOOLEAN` value.
    Boolean(bool),
    /// A SQL `REAL` / `DOUBLE` value.
    Float(f64),
    /// SQL `NULL`.
    ///
    /// Emitted by `<Option<T> as Expr>::write_to` (`expr.rs`): `None::<i64>`
    /// binds one `?` with this value. `IS NULL` / `IS NOT NULL` bind no parameter
    /// at all, so this variant appears only for a `None` *literal*.
    ///
    /// Read `expr.rs`'s note on what that makes possible: `nick.eq(None::<&str>)`
    /// renders `(nick = ?)` bound to `NULL`, which is never true in SQL and is
    /// never true in `to_memory` either. It is deliberately **not** rewritten to
    /// `IS NULL` — that would be a different query. Verified against SQLite: both
    /// interpreters return no rows.
    Null,
}
