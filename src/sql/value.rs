//! Runtime SQL values. These are what end up in the `params` vec returned
//! by [`Query::to_sql`](crate::sql::Query::to_sql) — they're the bind
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
    /// SQL `NULL`. Reserved — Phase 1 does not yet emit this; future
    /// nullability support will.
    Null,
}
