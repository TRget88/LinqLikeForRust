//! Runtime SQL values. These are what end up in the `params` vec returned
//! by [`Query::to_sql`](crate::Query::to_sql) — they're the bind
//! values the user will pass to a driver alongside the SQL string.

/// A SQL value at runtime, suitable for binding as a `?` placeholder.
///
/// Currently five variants, one per SQL type the crate exposes at the type level.
///
/// # `#[non_exhaustive]`: new variants are additive (`D-109`)
///
/// A new SQL type means a new variant here, and the dialect layer will need at
/// least one. Without this attribute each addition would break every downstream
/// `match` — so the attribute is what makes the extension *additive* rather than
/// a major bump, and it is the reason the sentence above can promise growth at
/// all. It had to be added before 1.0: applying it is itself a breaking change,
/// because it forces downstream matches to carry a wildcard arm.
///
/// What that means for you:
///
/// - **Constructing** any variant is unaffected — `SqlValue::Integer(7)` works
///   from any crate, which is what a driver adapter does.
/// - **Matching** from outside this crate needs a `_ =>` arm. Prefer making that
///   arm do something honest (return an error naming the unhandled value) over
///   `unreachable!()`, which becomes a panic the day a variant is added.
///
/// `SqlValueRef` and `RowError` carry the same attribute for the same reason.
/// `Direction` and `linq_rs`'s `SingleError` deliberately do not: they are
/// exhaustive by nature — SQL has two sort directions, and a sequence yields
/// none, one, or more than one — so a wildcard arm there would cost callers
/// something and buy nothing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
