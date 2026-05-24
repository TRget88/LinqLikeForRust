//! SQL type tags — zero-sized marker types used at the type level to
//! enforce column / literal compatibility at compile time.
//!
//! A `Column<SqlType = Integer>` cannot be compared against a `&str`
//! literal because `&str` implements [`Expr`](crate::sql::Expr) with
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
