//! SQL type tags — zero-sized marker types used at the type level to
//! enforce column / literal compatibility at compile time.
//!
//! A `Column<SqlType = Integer>` cannot be compared against a `&str`
//! literal because `&str` implements [`Expr`](crate::Expr) with
//! `SqlType = Text`, not `Integer`. The compiler refuses the call.

/// Marker for SQL `INTEGER` columns. Rust `i32` and `i64` literals satisfy
/// `Expr<SqlType = Integer>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Integer;

/// Marker for SQL `TEXT` / `VARCHAR` columns. Rust `&str` and `String`
/// satisfy `Expr<SqlType = Text>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text;

/// Marker for SQL `BOOLEAN` columns. Rust `bool` satisfies
/// `Expr<SqlType = Boolean>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boolean;

/// Marker for SQL `REAL` / `DOUBLE` columns. Rust `f32` / `f64` satisfy
/// `Expr<SqlType = Float>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Float;

/// Sealed trait implemented by every SQL type marker. Lets generic code
/// constrain "any SQL type."
pub trait SqlType: sealed::Sealed + 'static {
    /// The SQL keyword for this type (e.g. `"INTEGER"`). Used by future
    /// DDL emission; not consumed by Phase 1 SELECT builders.
    const SQL_KEYWORD: &'static str;
}

impl SqlType for Integer {
    const SQL_KEYWORD: &'static str = "INTEGER";
}
impl SqlType for Text {
    const SQL_KEYWORD: &'static str = "TEXT";
}
impl SqlType for Boolean {
    const SQL_KEYWORD: &'static str = "BOOLEAN";
}
impl SqlType for Float {
    const SQL_KEYWORD: &'static str = "REAL";
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Integer {}
    impl Sealed for super::Text {}
    impl Sealed for super::Boolean {}
    impl Sealed for super::Float {}
}

// ═══════════════════════════════════════════════════════════════════════════
// Nullability (SPIKE)
// ═══════════════════════════════════════════════════════════════════════════

use core::marker::PhantomData;

/// Marker for a column or expression that may be SQL `NULL`.
///
/// `Nullable<Text>` is a distinct SQL type from `Text`. That is the whole
/// point: the *type* records that three-valued logic applies, so a comparison
/// touching it produces `Nullable<Boolean>` and not `Boolean`, and the
/// difference is visible to both interpreters.
pub struct Nullable<T>(PhantomData<T>);

impl<T: SqlType> sealed::Sealed for Nullable<T> {}

impl<T: SqlType> SqlType for Nullable<T> {
    const SQL_KEYWORD: &'static str = T::SQL_KEYWORD;
}

/// The non-nullable core of a SQL type: `Integer` and `Nullable<Integer>`
/// both have base `Integer`.
///
/// This is what lets the operator traits (`IntOps`, `TextOps`, …) apply to a
/// nullable column without duplicating them.
pub trait Family {
    /// The non-nullable marker underneath.
    type Base;
}

/// Type-level result of a comparison: `Boolean` when neither side is
/// nullable, `Nullable<Boolean>` when either side is.
///
/// This replaces `R: Expr<SqlType = L::SqlType>` on the comparison nodes.
/// The set of impls *is* the answer to "can a nullable column be compared to
/// a non-nullable one?" — yes, within one family, and the result is nullable.
pub trait CompareWith<Rhs> {
    /// `Boolean` or `Nullable<Boolean>`.
    type Out;
}

/// Type-level result of `AND` / `OR`. Same rule as [`CompareWith`], over the
/// boolean family only.
pub trait LogicWith<Rhs> {
    /// `Boolean` or `Nullable<Boolean>`.
    type Out;
}

/// Type-level result of `NOT`. Nullability passes through.
pub trait Negate {
    /// `Boolean` or `Nullable<Boolean>`.
    type Out;
}

/// SQL types that may stand as a `WHERE` clause. `Boolean` and
/// `Nullable<Boolean>`, and nothing else.
///
/// A `WHERE` clause keeps a row when the predicate is **TRUE**; `FALSE` and
/// `NULL` both drop it. That collapse from three values to two happens here
/// and only here — never inside the expression tree.
pub trait WhereClause {}

impl WhereClause for Boolean {}
impl WhereClause for Nullable<Boolean> {}

macro_rules! nullability {
    ($($base:ty),* $(,)?) => {$(
        impl Family for $base { type Base = $base; }
        impl Family for Nullable<$base> { type Base = $base; }

        impl CompareWith<$base> for $base { type Out = Boolean; }
        impl CompareWith<Nullable<$base>> for $base { type Out = Nullable<Boolean>; }
        impl CompareWith<$base> for Nullable<$base> { type Out = Nullable<Boolean>; }
        impl CompareWith<Nullable<$base>> for Nullable<$base> { type Out = Nullable<Boolean>; }

    )*};
}
nullability!(Integer, Text, Boolean, Float);

impl LogicWith<Boolean> for Boolean {
    type Out = Boolean;
}
impl LogicWith<Nullable<Boolean>> for Boolean {
    type Out = Nullable<Boolean>;
}
impl LogicWith<Boolean> for Nullable<Boolean> {
    type Out = Nullable<Boolean>;
}
impl LogicWith<Nullable<Boolean>> for Nullable<Boolean> {
    type Out = Nullable<Boolean>;
}

impl Negate for Boolean {
    type Out = Boolean;
}
impl Negate for Nullable<Boolean> {
    type Out = Nullable<Boolean>;
}
