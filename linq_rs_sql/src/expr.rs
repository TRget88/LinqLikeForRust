//! Expression tree — every node carries an associated [`SqlType`] so the
//! compiler can reject mismatched comparisons at the call site.
//!
//! The tree is consumed by [`Expr::write_to`], which appends SQL into a
//! buffer and pushes any literal values into a parameter `Vec`. Every
//! literal is emitted as a `?` placeholder; nothing user-supplied is
//! inlined into the SQL text.

use crate::types::{
    Boolean, CompareWith, Family, Float, Integer, LogicWith, Negate, Nullable, SqlType, Text,
};
use crate::value::SqlValue;
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

        impl<T, L: $crate::expr::BelongsTo<T>, R: $crate::expr::BelongsTo<T>>
            $crate::expr::BelongsTo<T> for $name<L, R>
        {
        }

        impl<L, R> Expr for $name<L, R>
        where
            L: Expr,
            R: Expr,
            L::SqlType: CompareWith<R::SqlType>,
        {
            // NOT `Boolean`. Comparing anything to a nullable operand yields
            // `Nullable<Boolean>`, because in SQL it yields NULL, not FALSE.
            type SqlType = <L::SqlType as CompareWith<R::SqlType>>::Out;
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
    L: Expr,
    R: Expr,
    L::SqlType: LogicWith<R::SqlType>,
{
    type SqlType = <L::SqlType as LogicWith<R::SqlType>>::Out;
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
    L: Expr,
    R: Expr,
    L::SqlType: LogicWith<R::SqlType>,
{
    type SqlType = <L::SqlType as LogicWith<R::SqlType>>::Out;
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

impl<E> Expr for Not<E>
where
    E: Expr,
    E::SqlType: Negate,
{
    type SqlType = <E::SqlType as Negate>::Out;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push_str("NOT (");
        self.inner.write_to(sql, params);
        sql.push(')');
    }
}

/// Boolean negation. Free function because `.not()` would collide with
/// [`std::ops::Not`] when called on a `bool` literal.
pub fn not<E>(expr: E) -> Not<E>
where
    E: Expr,
    E::SqlType: Negate,
{
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
    L: Expr,
    R: Expr,
    L::SqlType: Family<Base = Text> + CompareWith<R::SqlType>,
    R::SqlType: Family<Base = Text>,
{
    // `NULL LIKE 'a%'` is NULL in SQL, verified against SQLite.
    type SqlType = <L::SqlType as CompareWith<R::SqlType>>::Out;
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

/// `expr IS NOT NULL`. Produced by `.is_not_null`.
#[derive(Debug, Clone, Copy)]
pub struct IsNotNull<E> {
    pub(crate) inner: E,
}

impl<E: Expr> Expr for IsNotNull<E> {
    // Always two-valued: `x IS NULL` and `x IS NOT NULL` are never NULL.
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push('(');
        self.inner.write_to(sql, params);
        sql.push_str(" IS NOT NULL)");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// The NULL literal
// ═══════════════════════════════════════════════════════════════════════════

/// `Option<T>` is the literal form of a nullable value: `Some(3i64)` binds
/// `3`, `None::<i64>` binds SQL `NULL`.
///
/// Note what this makes possible and what it does not. `score.eq(None::<i64>)`
/// compiles and renders `(score = ?)` with a `NULL` parameter — which is
/// *never true* in SQL, and is never true here either. It is not rewritten
/// into `IS NULL`; rewriting it would be a different query.
impl<T: Expr> Expr for Option<T> {
    type SqlType = Nullable<T::SqlType>;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        match self {
            Some(v) => v.write_to(sql, params),
            None => {
                sql.push('?');
                params.push(SqlValue::Null);
            }
        }
    }
}

/// Build an `And` node with no bounds — the type checking happens in
/// `impl Expr for And`. Used by `rows::Conj`, which must not re-state the
/// bounds.
pub(crate) fn and_node<L, R>(left: L, right: R) -> And<L, R> {
    And { left, right }
}

// ═══════════════════════════════════════════════════════════════════════════
// BelongsTo — which table an expression's columns come from
// ═══════════════════════════════════════════════════════════════════════════

/// An expression every column of which belongs to table `T`.
///
/// Without this, a query over one table could be filtered by another table's
/// column and the mismatch would only surface as a database error at runtime:
///
/// ```text
/// query::<Employee>().filter(departments::budget.gt(100))
/// // SELECT * FROM employees WHERE (budget > ?)      <- no such column
/// ```
///
/// The in-memory interpreter already rejected that, because `Eval<'_, Row>` is
/// only implemented for the row's own columns. `to_sql()` did not — which is
/// backwards, since SQL is the production path. `BelongsTo` gives the SQL half
/// the same guarantee, at compile time. See `D-028`.
///
/// **Literals belong to every table**, because they have no columns. That is
/// what makes `salary.gt(100)` legal over any table that has a `salary`.
///
/// Implemented for columns by the [`table!`](crate::table) macro rather than by
/// a blanket impl over [`Column`](crate::Column): a blanket impl would overlap
/// the literal impls below, and Rust cannot prove `i64: !Column`.
/// A query over one table cannot be filtered by another table's column:
///
/// ```compile_fail
/// use linq_rs_sql::prelude::*;
/// table! { employees (id) { id -> Integer, salary -> Integer } }
/// table! { departments (id) { id -> Integer, budget -> Integer } }
/// pub struct Employee { pub id: i64, pub salary: i64 }
/// entity! { Employee => employees { id: Integer = id, salary: Integer = salary } }
///
/// // error[E0277]: the trait bound `budget: BelongsTo<employees::Marker>`
/// //               is not satisfied
/// let _ = query::<Employee>().filter(departments::budget.gt(100i64)).to_sql();
/// ```
///
/// The same check applies to the SQL-only builder:
///
/// ```compile_fail
/// use linq_rs_sql::prelude::*;
/// table! { employees (id) { id -> Integer } }
/// table! { departments (id) { budget -> Integer } }
///
/// let _ = employees::table().filter(departments::budget.gt(100i64)).to_sql();
/// ```
///
/// A column from the query's own table, and any literal, are fine:
///
/// ```rust
/// use linq_rs_sql::prelude::*;
/// table! { employees (id) { id -> Integer, salary -> Integer } }
/// pub struct Employee { pub id: i64, pub salary: i64 }
/// entity! { Employee => employees { id: Integer = id, salary: Integer = salary } }
///
/// let q = query::<Employee>().filter(employees::salary.gt(100i64));
/// assert_eq!(q.to_sql().sql, "SELECT * FROM employees WHERE (salary > ?)");
/// ```
pub trait BelongsTo<T> {}

// Literals: no columns, so every table.
impl<T> BelongsTo<T> for i32 {}
impl<T> BelongsTo<T> for i64 {}
impl<T> BelongsTo<T> for bool {}
impl<T> BelongsTo<T> for f32 {}
impl<T> BelongsTo<T> for f64 {}
impl<T> BelongsTo<T> for String {}
impl<T> BelongsTo<T> for &str {}
impl<T, S: SqlType> BelongsTo<T> for Lit<S> {}
impl<T, E: BelongsTo<T>> BelongsTo<T> for Option<E> {}

// Combinators: belong to `T` exactly when every operand does.
impl<T, L: BelongsTo<T>, R: BelongsTo<T>> BelongsTo<T> for And<L, R> {}
impl<T, L: BelongsTo<T>, R: BelongsTo<T>> BelongsTo<T> for Or<L, R> {}
impl<T, L: BelongsTo<T>, R: BelongsTo<T>> BelongsTo<T> for Like<L, R> {}
impl<T, E: BelongsTo<T>> BelongsTo<T> for Not<E> {}
impl<T, E: BelongsTo<T>> BelongsTo<T> for IsNull<E> {}
impl<T, E: BelongsTo<T>> BelongsTo<T> for IsNotNull<E> {}
