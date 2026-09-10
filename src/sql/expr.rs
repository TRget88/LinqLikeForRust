//! Expression tree — every node carries an associated [`SqlType`] so the
//! compiler can reject mismatched comparisons at the call site.
//!
//! The tree is consumed by [`Expr::write_to`], which appends SQL into a
//! buffer and pushes any literal values into a parameter `Vec`. Every
//! literal is emitted as a `?` placeholder; nothing user-supplied is
//! inlined into the SQL text.

use crate::sql::types::{Boolean, Float, Integer, SqlType, Text};
use crate::sql::value::SqlValue;
use core::marker::PhantomData;

// ═══════════════════════════════════════════════════════════════════════════
// Core trait
// ═══════════════════════════════════════════════════════════════════════════

/// A typed SQL expression. Implementors include columns (one per declared
/// schema field), primitive Rust literals (`i32`, `&str`, `bool`, …), and
/// the binary / logical / NULL nodes built by the operator methods.
///
/// The associated [`SqlType`] marker drives type-checking: a comparison
/// node like `Eq<L, R>` requires `R::SqlType == L::SqlType`.
pub trait Expr {
    /// SQL-side type of this expression.
    type SqlType;
    /// Append SQL text for this expression into `sql`. Any literal values
    /// are pushed as `?` placeholders and their bound values appended to
    /// `params`.
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>);
}

// ═══════════════════════════════════════════════════════════════════════════
// Primitive literal impls — let users write `col.gt(18)` not `col.gt(lit(18))`
// ═══════════════════════════════════════════════════════════════════════════

impl Expr for i32 {
    type SqlType = Integer;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Integer(i64::from(*self)));
    }
}

impl Expr for i64 {
    type SqlType = Integer;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Integer(*self));
    }
}

impl Expr for bool {
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Boolean(*self));
    }
}

impl Expr for f32 {
    type SqlType = Float;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Float(f64::from(*self)));
    }
}

impl Expr for f64 {
    type SqlType = Float;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Float(*self));
    }
}

impl Expr for String {
    type SqlType = Text;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Text(self.clone()));
    }
}

impl Expr for &str {
    type SqlType = Text;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(SqlValue::Text((*self).to_string()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Explicit literal wrapper (mostly for type-inference escape hatches)
// ═══════════════════════════════════════════════════════════════════════════

/// Explicit literal carrier — useful when type inference can't pick the
/// right primitive `Expr` impl (e.g. ambiguous numeric literal). Day-to-day,
/// passing a primitive directly is enough.
#[derive(Debug, Clone)]
pub struct Lit<T> {
    value: SqlValue,
    _ty: PhantomData<T>,
}

impl<T: SqlType> Lit<T> {
    /// Wrap a raw `SqlValue` with a SQL-type tag. Use the `Lit::int`,
    /// `Lit::text`, etc. constructors instead unless you really need to
    /// hand-craft the value.
    pub fn from_raw(value: SqlValue) -> Self {
        Self {
            value,
            _ty: PhantomData,
        }
    }
}

impl Lit<Integer> {
    /// Integer literal.
    pub fn int(value: i64) -> Self {
        Self::from_raw(SqlValue::Integer(value))
    }
}

impl Lit<Text> {
    /// Text literal.
    pub fn text(value: impl Into<String>) -> Self {
        Self::from_raw(SqlValue::Text(value.into()))
    }
}

impl Lit<Boolean> {
    /// Boolean literal.
    pub fn boolean(value: bool) -> Self {
        Self::from_raw(SqlValue::Boolean(value))
    }
}

impl Lit<Float> {
    /// Float literal.
    pub fn float(value: f64) -> Self {
        Self::from_raw(SqlValue::Float(value))
    }
}

impl<T: SqlType> Expr for Lit<T> {
    type SqlType = T;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('?');
        params.push(self.value.clone());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Binary comparison nodes
// ═══════════════════════════════════════════════════════════════════════════

macro_rules! define_binary_compare {
    ($(#[$attr:meta])* $name:ident, $sep:literal) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name<L, R> {
            pub(crate) left: L,
            pub(crate) right: R,
        }

        impl<L, R> Expr for $name<L, R>
        where
            L: Expr,
            R: Expr<SqlType = L::SqlType>,
        {
            type SqlType = Boolean;
            fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
                sql.push('(');
                self.left.write_to(sql, params);
                sql.push_str($sep);
                self.right.write_to(sql, params);
                sql.push(')');
            }
        }
    };
}

define_binary_compare!(
    /// `lhs = rhs` — exact equality. Produced by `.eq`.
    Eq, " = "
);
define_binary_compare!(
    /// `lhs != rhs`. Produced by `.ne`.
    NotEq, " != "
);
define_binary_compare!(
    /// `lhs < rhs`. Produced by `.lt`.
    Lt, " < "
);
define_binary_compare!(
    /// `lhs <= rhs`. Produced by `.lte`.
    LtEq, " <= "
);
define_binary_compare!(
    /// `lhs > rhs`. Produced by `.gt`.
    Gt, " > "
);
define_binary_compare!(
    /// `lhs >= rhs`. Produced by `.gte`.
    GtEq, " >= "
);

// ═══════════════════════════════════════════════════════════════════════════
// Logical combinators
// ═══════════════════════════════════════════════════════════════════════════

/// `lhs AND rhs`. Produced by `.and`.
#[derive(Debug, Clone, Copy)]
pub struct And<L, R> {
    pub(crate) left: L,
    pub(crate) right: R,
}

impl<L, R> Expr for And<L, R>
where
    L: Expr<SqlType = Boolean>,
    R: Expr<SqlType = Boolean>,
{
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('(');
        self.left.write_to(sql, params);
        sql.push_str(" AND ");
        self.right.write_to(sql, params);
        sql.push(')');
    }
}

/// `lhs OR rhs`. Produced by `.or`.
#[derive(Debug, Clone, Copy)]
pub struct Or<L, R> {
    pub(crate) left: L,
    pub(crate) right: R,
}

impl<L, R> Expr for Or<L, R>
where
    L: Expr<SqlType = Boolean>,
    R: Expr<SqlType = Boolean>,
{
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('(');
        self.left.write_to(sql, params);
        sql.push_str(" OR ");
        self.right.write_to(sql, params);
        sql.push(')');
    }
}

/// `NOT expr`. Produced by the free function [`not`].
#[derive(Debug, Clone, Copy)]
pub struct Not<E> {
    pub(crate) inner: E,
}

impl<E: Expr<SqlType = Boolean>> Expr for Not<E> {
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push_str("NOT (");
        self.inner.write_to(sql, params);
        sql.push(')');
    }
}

/// Boolean negation. Free function because `.not()` would collide with
/// [`std::ops::Not`] when called on a `bool` literal.
pub fn not<E: Expr<SqlType = Boolean>>(expr: E) -> Not<E> {
    Not { inner: expr }
}

// ═══════════════════════════════════════════════════════════════════════════
// Text-only ops
// ═══════════════════════════════════════════════════════════════════════════

/// `lhs LIKE rhs` — text pattern match. Produced by `.like`.
#[derive(Debug, Clone, Copy)]
pub struct Like<L, R> {
    pub(crate) left: L,
    pub(crate) right: R,
}

impl<L, R> Expr for Like<L, R>
where
    L: Expr<SqlType = Text>,
    R: Expr<SqlType = Text>,
{
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('(');
        self.left.write_to(sql, params);
        sql.push_str(" LIKE ");
        self.right.write_to(sql, params);
        sql.push(')');
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// NULL test
// ═══════════════════════════════════════════════════════════════════════════

/// `expr IS NULL`. Produced by `.is_null`.
#[derive(Debug, Clone, Copy)]
pub struct IsNull<E> {
    pub(crate) inner: E,
}

impl<E: Expr> Expr for IsNull<E> {
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('(');
        self.inner.write_to(sql, params);
        sql.push_str(" IS NULL)");
    }
}
