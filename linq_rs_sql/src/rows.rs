//! The two-interpreter seam (`D-002`, `W-20`).
//!
//! One query value, two interpreters: [`Rows::to_sql`] renders it through the
//! existing [`Query`] builder, and [`Rows::to_memory`] evaluates
//! it against a `Vec<Row>` — streaming, no allocation, no dynamic dispatch.
//!
//! The seam adds exactly one new leaf concept to the expression language:
//! [`Eval`], the memory half of an [`Expr`]. Everything else — the column
//! types, the comparison nodes, the SQL type markers, the renderer — is the
//! `linq_rs_sql` that already exists.
//!
//! ## Why the Rust type of a column is not a free choice
//!
//! [`Repr`] pins one Rust type per SQL type marker: `Integer` is `i64`, `Text`
//! is `&'r str`, `Boolean` is `bool`, `Float` is `f64` — the same normalisation
//! [`SqlValue`](crate::SqlValue) already performs. An [`Eval`] impl therefore
//! cannot choose what it returns; the column's declared SQL type chooses for
//! it. Comparing text to an integer is not "detected", it is unrepresentable.
//!
//! ## `D-102`: the boundary is a compile error
//!
//! [`Rows`] is not an `Iterator` and does not implement `LinqExt`. It exposes
//! `filter`, `order_by`, `order_by_desc`, `limit`, `offset`, `to_sql` and
//! `to_memory`, and nothing else. Reaching for `select_many`, `group_by_key`
//! or any other non-translatable operator is `E0599` at the call site.
//! `to_memory` is the single visible token that crosses the boundary.

use crate::column::Column;
use crate::expr::{And, Eq, Expr, Gt, GtEq, IsNotNull, IsNull, Like, Lt, LtEq, Not, NotEq, Or};
use crate::query::{All, Query, QueryOutput, Table};
use crate::types::{
    Boolean, CompareWith, Float, Integer, LogicWith, Negate, Nullable, Text, WhereClause,
};
use core::cmp::Ordering;
use core::marker::PhantomData;

// ═══════════════════════════════════════════════════════════════════════════
// Repr — the one Rust type per SQL type
// ═══════════════════════════════════════════════════════════════════════════

/// The Rust representation of a SQL type marker, for in-memory evaluation.
///
/// This is the fix for the prototype's dynamic `Val` enum. There, a column
/// accessor returned `Val::Str(..)` or `Val::I64(..)` and a mismatched
/// comparison silently produced `false`. Here the mapping is a type function:
/// a `Column` whose `SqlType` is `Integer` evaluates to `i64` or does not
/// compile.
pub trait Repr<'r> {
    /// The Rust type an expression of this SQL type evaluates to.
    type Rust;
}

impl<'r> Repr<'r> for Integer {
    type Rust = i64;
}
impl<'r> Repr<'r> for Text {
    type Rust = &'r str;
}
impl<'r> Repr<'r> for Boolean {
    type Rust = bool;
}
impl<'r> Repr<'r> for Float {
    type Rust = f64;
}

/// The one honest Rust representation of a nullable SQL type.
///
/// `Nullable<Text>` is `Option<&'r str>`, `Nullable<Integer>` is
/// `Option<i64>`. Crucially `Nullable<Boolean>` is `Option<bool>` — a *third*
/// value, which is what makes three-valued logic representable rather than
/// collapsed.
impl<'r, T: Repr<'r>> Repr<'r> for Nullable<T> {
    type Rust = Option<T::Rust>;
}

/// Shorthand for "the Rust value an expression of SQL type `S` produces".
pub type Rust<'r, S> = <S as Repr<'r>>::Rust;

/// SQL types that may appear in `ORDER BY`.
///
/// This exists instead of a `for<'x> Rust<'x, C::SqlType>: Ord` bound on
/// `order_by`, for two reasons. The blunt one: Rust **1.65** — the crate's
/// MSRV — cannot solve a higher-ranked bound through a two-level projection,
/// and reports `the trait bound `for<'x> <_ as Repr<'x>>::Rust: Ord` is not
/// satisfied` at every call site. (It compiles on 1.75.) The better one: which
/// SQL types are orderable is a *decision*, and this makes it a list someone
/// can read.
///
/// `Float` is deliberately absent, matching `linq_rs`, whose `order_by` binds
/// `K: Ord` and so already rejects `f64` keys (`D-018`).
pub trait Sortable: for<'x> Repr<'x> {
    /// Total order over this SQL type's Rust representation.
    fn compare<'x>(a: Rust<'x, Self>, b: Rust<'x, Self>) -> Ordering;
}

impl Sortable for Integer {
    fn compare<'x>(a: i64, b: i64) -> Ordering {
        a.cmp(&b)
    }
}

impl Sortable for Text {
    /// Stability class (`D-103`): **identical-modulo-collation**. This is
    /// Rust's byte-ordinal `Ord for str`; PostgreSQL orders by the column's
    /// collation and will disagree on case and on non-ASCII text.
    fn compare<'x>(a: &'x str, b: &'x str) -> Ordering {
        a.cmp(b)
    }
}

impl Sortable for Boolean {
    fn compare<'x>(a: bool, b: bool) -> Ordering {
        a.cmp(&b)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Three-valued logic (SPIKE) — the memory half of nullability
// ═══════════════════════════════════════════════════════════════════════════
//
// Every rule below was read off real SQLite (see `sqlite_truth.py`):
//
//   NULL = NULL   -> NULL        NULL AND TRUE  -> NULL
//   NULL > 5      -> NULL        NULL AND FALSE -> FALSE
//   NOT (NULL=1)  -> NULL        NULL OR  TRUE  -> TRUE
//   NULL IS NULL  -> TRUE        NULL OR  FALSE -> NULL
//
// The point of the exercise: none of these can be expressed by `Option`'s own
// `PartialOrd`/`PartialEq`, which say `None == None` is `true` and
// `None > Some(5)` is `false`. Both are wrong, and both are *silently* wrong —
// the D-025 failure mode. So `Option`'s comparison operators are never used
// here; every rule is written out.

/// A comparison operator as a type, so `cmp3` monomorphises to one inlined
/// comparison rather than a branch on a runtime tag or a `fn` pointer.
pub trait CmpOp {
    /// Apply this operator to two values of the same (non-null) Rust type.
    fn apply<T: PartialOrd>(l: &T, r: &T) -> bool;
}

macro_rules! cmp_ops {
    ($($name:ident => $op:tt),* $(,)?) => {$(
        /// Operator marker for [`CmpOp`].
        #[derive(Debug, Clone, Copy)]
        pub struct $name;
        impl CmpOp for $name {
            fn apply<T: PartialOrd>(l: &T, r: &T) -> bool { l $op r }
        }
    )*};
}
cmp_ops!(OpEq => ==, OpNe => !=, OpLt => <, OpLte => <=, OpGt => >, OpGte => >=);

/// The value half of [`CompareWith`]: how a comparison actually evaluates once
/// nullability is in play.
///
/// Shaped like [`Sortable`] — the operation lives *in* the trait rather than as
/// a `for<'x> Rust<'x, T>: PartialOrd` bound at the call site — for the same
/// reason `Sortable` is: Rust 1.65 cannot solve a higher-ranked bound through a
/// two-level projection.
pub trait Cmp3<'x, Rhs>: CompareWith<Rhs> + Repr<'x>
where
    Rhs: Repr<'x>,
    Self::Out: Repr<'x>,
{
    /// Three-valued comparison. `NULL` on either side propagates.
    fn cmp3<Op: CmpOp>(l: Rust<'x, Self>, r: Rust<'x, Rhs>) -> Rust<'x, Self::Out>;
}

macro_rules! impl_cmp3 {
    ($($base:ty),* $(,)?) => {$(
        impl<'x> Cmp3<'x, $base> for $base {
            fn cmp3<Op: CmpOp>(l: Rust<'x, $base>, r: Rust<'x, $base>) -> bool {
                Op::apply(&l, &r)
            }
        }
        impl<'x> Cmp3<'x, Nullable<$base>> for $base {
            fn cmp3<Op: CmpOp>(
                l: Rust<'x, $base>,
                r: Option<Rust<'x, $base>>,
            ) -> Option<bool> {
                match r {
                    Some(r) => Some(Op::apply(&l, &r)),
                    None => None,
                }
            }
        }
        impl<'x> Cmp3<'x, $base> for Nullable<$base> {
            fn cmp3<Op: CmpOp>(
                l: Option<Rust<'x, $base>>,
                r: Rust<'x, $base>,
            ) -> Option<bool> {
                match l {
                    Some(l) => Some(Op::apply(&l, &r)),
                    None => None,
                }
            }
        }
        impl<'x> Cmp3<'x, Nullable<$base>> for Nullable<$base> {
            fn cmp3<Op: CmpOp>(
                l: Option<Rust<'x, $base>>,
                r: Option<Rust<'x, $base>>,
            ) -> Option<bool> {
                // NOT `l == r`. `None == None` is `true` in Rust and NULL in
                // SQL; this is the exact place the two interpreters would
                // silently part company.
                match (l, r) {
                    (Some(l), Some(r)) => Some(Op::apply(&l, &r)),
                    _ => None,
                }
            }
        }
    )*};
}
impl_cmp3!(Integer, Text, Boolean, Float);

/// `LIKE` lifted over nullability. Separate from [`Cmp3`] because the
/// underlying operation is text-specific, not a `PartialOrd` comparison.
pub trait Like3<'x, Rhs>: CompareWith<Rhs> + Repr<'x>
where
    Rhs: Repr<'x>,
    Self::Out: Repr<'x>,
{
    /// Three-valued `LIKE`. `NULL LIKE 'a%'` is NULL, per SQLite.
    fn like3(l: Rust<'x, Self>, r: Rust<'x, Rhs>) -> Rust<'x, Self::Out>;
}

impl<'x> Like3<'x, Text> for Text {
    fn like3(l: &'x str, r: &'x str) -> bool {
        like_match(l, r)
    }
}
impl<'x> Like3<'x, Nullable<Text>> for Text {
    fn like3(l: &'x str, r: Option<&'x str>) -> Option<bool> {
        r.map(|r| like_match(l, r))
    }
}
impl<'x> Like3<'x, Text> for Nullable<Text> {
    fn like3(l: Option<&'x str>, r: &'x str) -> Option<bool> {
        l.map(|l| like_match(l, r))
    }
}
impl<'x> Like3<'x, Nullable<Text>> for Nullable<Text> {
    fn like3(l: Option<&'x str>, r: Option<&'x str>) -> Option<bool> {
        match (l, r) {
            (Some(l), Some(r)) => Some(like_match(l, r)),
            _ => None,
        }
    }
}

/// The value half of [`LogicWith`] — Kleene `AND` / `OR`.
///
/// The right operand arrives as a closure so `AND`/`OR` still short-circuit:
/// `FALSE AND anything` is `FALSE` without evaluating the right side, exactly
/// as the two-valued version did. It is a monomorphised `FnOnce`, so this
/// costs nothing at runtime.
pub trait Logic3<'x, Rhs>: LogicWith<Rhs> + Repr<'x>
where
    Rhs: Repr<'x>,
    Self::Out: Repr<'x>,
{
    /// Kleene `AND`.
    fn and3<F: FnOnce() -> Rust<'x, Rhs>>(l: Rust<'x, Self>, r: F) -> Rust<'x, Self::Out>;
    /// Kleene `OR`.
    fn or3<F: FnOnce() -> Rust<'x, Rhs>>(l: Rust<'x, Self>, r: F) -> Rust<'x, Self::Out>;
}

impl<'x> Logic3<'x, Boolean> for Boolean {
    fn and3<F: FnOnce() -> bool>(l: bool, r: F) -> bool {
        l && r()
    }
    fn or3<F: FnOnce() -> bool>(l: bool, r: F) -> bool {
        l || r()
    }
}

impl<'x> Logic3<'x, Nullable<Boolean>> for Boolean {
    fn and3<F: FnOnce() -> Option<bool>>(l: bool, r: F) -> Option<bool> {
        // FALSE AND NULL is FALSE; TRUE AND NULL is NULL.
        if l {
            r()
        } else {
            Some(false)
        }
    }
    fn or3<F: FnOnce() -> Option<bool>>(l: bool, r: F) -> Option<bool> {
        // TRUE OR NULL is TRUE; FALSE OR NULL is NULL.
        if l {
            Some(true)
        } else {
            r()
        }
    }
}

impl<'x> Logic3<'x, Boolean> for Nullable<Boolean> {
    fn and3<F: FnOnce() -> bool>(l: Option<bool>, r: F) -> Option<bool> {
        match l {
            Some(false) => Some(false),
            other => {
                if r() {
                    other
                } else {
                    Some(false)
                }
            }
        }
    }
    fn or3<F: FnOnce() -> bool>(l: Option<bool>, r: F) -> Option<bool> {
        match l {
            Some(true) => Some(true),
            other => {
                if r() {
                    Some(true)
                } else {
                    other
                }
            }
        }
    }
}

impl<'x> Logic3<'x, Nullable<Boolean>> for Nullable<Boolean> {
    fn and3<F: FnOnce() -> Option<bool>>(l: Option<bool>, r: F) -> Option<bool> {
        match l {
            Some(false) => Some(false),
            other => match r() {
                Some(false) => Some(false),
                Some(true) => other,
                None => None,
            },
        }
    }
    fn or3<F: FnOnce() -> Option<bool>>(l: Option<bool>, r: F) -> Option<bool> {
        match l {
            Some(true) => Some(true),
            other => match r() {
                Some(true) => Some(true),
                Some(false) => other,
                None => None,
            },
        }
    }
}

/// The value half of [`Negate`]. `NOT NULL` is NULL.
pub trait Negate3<'x>: Negate + Repr<'x>
where
    Self::Out: Repr<'x>,
{
    /// Three-valued `NOT`.
    fn not3(v: Rust<'x, Self>) -> Rust<'x, Self::Out>;
}

impl<'x> Negate3<'x> for Boolean {
    fn not3(v: bool) -> bool {
        !v
    }
}
impl<'x> Negate3<'x> for Nullable<Boolean> {
    fn not3(v: Option<bool>) -> Option<bool> {
        // NOT NULL is NULL, not TRUE.
        v.map(|b| !b)
    }
}

/// `IS NULL` / `IS NOT NULL`, which are the *only* two-valued things in this
/// module: they answer TRUE or FALSE and never NULL.
///
/// Implemented for the non-nullable markers too, returning `false`. That is
/// not a shortcut — a column declared `Integer` is declared NOT NULL, and
/// `NOT NULL col IS NULL` is FALSE in SQL as well. Both interpreters still
/// agree; the query is merely pointless.
pub trait NullCheck<'x>: Repr<'x> {
    /// Is this value SQL NULL?
    fn is_null(v: Rust<'x, Self>) -> bool;
}

macro_rules! impl_null_check {
    ($($base:ty),* $(,)?) => {$(
        impl<'x> NullCheck<'x> for $base {
            fn is_null(_v: Rust<'x, $base>) -> bool { false }
        }
    )*};
}
impl_null_check!(Integer, Text, Boolean, Float);

impl<'x, T: Repr<'x>> NullCheck<'x> for Nullable<T> {
    fn is_null(v: Option<<T as Repr<'x>>::Rust>) -> bool {
        v.is_none()
    }
}

/// The collapse from three values to two, at the `WHERE` boundary and nowhere
/// else. A row is kept when the predicate is TRUE; FALSE and NULL both drop it.
pub trait TruthValue<'x>: WhereClause + Repr<'x> {
    /// The three-valued result, normalised: `None` is UNKNOWN.
    ///
    /// This is the lossless view. [`is_true`](Self::is_true) is the lossy one,
    /// and the difference matters: the collapse to two values distributes over
    /// `AND` and `OR` but **not** over `NOT` — `is_true(NOT NULL)` is `false`
    /// while `!is_true(NULL)` is `true`. Anything that might later negate a
    /// value must carry the three-valued form, which is why type erasure
    /// ([`crate::boxed`]) transports `Option<bool>` and not `bool`.
    fn to_tri(v: Rust<'x, Self>) -> Option<bool>;

    /// Does this predicate value keep the row? TRUE keeps it; FALSE and NULL
    /// both drop it.
    fn is_true(v: Rust<'x, Self>) -> bool {
        matches!(Self::to_tri(v), Some(true))
    }
}

impl<'x> TruthValue<'x> for Boolean {
    fn to_tri(v: bool) -> Option<bool> {
        Some(v)
    }
}
impl<'x> TruthValue<'x> for Nullable<Boolean> {
    fn to_tri(v: Option<bool>) -> Option<bool> {
        v
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Eval — the memory half of Expr
// ═══════════════════════════════════════════════════════════════════════════

/// The in-memory interpretation of an [`Expr`], evaluated against one row.
///
/// `Expr::write_to` renders a node to SQL; `Eval::eval` computes it over a
/// borrowed row. Implementing both for one type is what makes a query value
/// interpretable two ways.
///
/// The lifetime is what lets `Text` evaluate to `&'r str` rather than a cloned
/// `String`, so filtering a `Vec<Employee>` on a name allocates nothing.
pub trait Eval<'r, Row>: Expr
where
    Self::SqlType: Repr<'r>,
{
    /// Evaluate this node against `row`.
    fn eval(&'r self, row: &'r Row) -> Rust<'r, Self::SqlType>;
}

// ── literals ───────────────────────────────────────────────────────────────

impl<'r, Row> Eval<'r, Row> for i32 {
    fn eval(&'r self, _row: &'r Row) -> i64 {
        i64::from(*self)
    }
}
impl<'r, Row> Eval<'r, Row> for i64 {
    fn eval(&'r self, _row: &'r Row) -> i64 {
        *self
    }
}
impl<'r, Row> Eval<'r, Row> for bool {
    fn eval(&'r self, _row: &'r Row) -> bool {
        *self
    }
}
impl<'r, Row> Eval<'r, Row> for f32 {
    fn eval(&'r self, _row: &'r Row) -> f64 {
        f64::from(*self)
    }
}
impl<'r, Row> Eval<'r, Row> for f64 {
    fn eval(&'r self, _row: &'r Row) -> f64 {
        *self
    }
}
impl<'r, Row> Eval<'r, Row> for String {
    fn eval(&'r self, _row: &'r Row) -> &'r str {
        self.as_str()
    }
}
impl<'r, 'a, Row> Eval<'r, Row> for &'a str
where
    'a: 'r,
{
    fn eval(&'r self, _row: &'r Row) -> &'r str {
        self
    }
}

/// `None::<i64>` is the SQL `NULL` literal of type `Nullable<Integer>`;
/// `Some(3i64)` is a nullable-typed `3`.
impl<'r, Row, T> Eval<'r, Row> for Option<T>
where
    T: Eval<'r, Row>,
    T::SqlType: Repr<'r>,
{
    fn eval(&'r self, row: &'r Row) -> Option<Rust<'r, T::SqlType>> {
        self.as_ref().map(|v| v.eval(row))
    }
}

// ── comparison nodes ───────────────────────────────────────────────────────

macro_rules! eval_compare {
    ($node:ident, $op:ty) => {
        impl<'r, Row, L, R> Eval<'r, Row> for $node<L, R>
        where
            L: Eval<'r, Row>,
            R: Eval<'r, Row>,
            L::SqlType: Cmp3<'r, R::SqlType>,
            R::SqlType: Repr<'r>,
            <L::SqlType as CompareWith<R::SqlType>>::Out: Repr<'r>,
            Self: Expr<SqlType = <L::SqlType as CompareWith<R::SqlType>>::Out>,
        {
            fn eval(&'r self, row: &'r Row) -> Rust<'r, Self::SqlType> {
                <L::SqlType as Cmp3<'r, R::SqlType>>::cmp3::<$op>(
                    self.left.eval(row),
                    self.right.eval(row),
                )
            }
        }
    };
}

eval_compare!(Eq, OpEq);
eval_compare!(NotEq, OpNe);
eval_compare!(Lt, OpLt);
eval_compare!(LtEq, OpLte);
eval_compare!(Gt, OpGt);
eval_compare!(GtEq, OpGte);

// ── logical nodes ──────────────────────────────────────────────────────────

impl<'r, Row, L, R> Eval<'r, Row> for And<L, R>
where
    L: Eval<'r, Row>,
    R: Eval<'r, Row>,
    L::SqlType: Logic3<'r, R::SqlType>,
    R::SqlType: Repr<'r>,
    <L::SqlType as LogicWith<R::SqlType>>::Out: Repr<'r>,
    Self: Expr<SqlType = <L::SqlType as LogicWith<R::SqlType>>::Out>,
{
    fn eval(&'r self, row: &'r Row) -> Rust<'r, Self::SqlType> {
        // Still short-circuits: the right operand is a closure.
        <L::SqlType as Logic3<'r, R::SqlType>>::and3(self.left.eval(row), || self.right.eval(row))
    }
}

impl<'r, Row, L, R> Eval<'r, Row> for Or<L, R>
where
    L: Eval<'r, Row>,
    R: Eval<'r, Row>,
    L::SqlType: Logic3<'r, R::SqlType>,
    R::SqlType: Repr<'r>,
    <L::SqlType as LogicWith<R::SqlType>>::Out: Repr<'r>,
    Self: Expr<SqlType = <L::SqlType as LogicWith<R::SqlType>>::Out>,
{
    fn eval(&'r self, row: &'r Row) -> Rust<'r, Self::SqlType> {
        <L::SqlType as Logic3<'r, R::SqlType>>::or3(self.left.eval(row), || self.right.eval(row))
    }
}

impl<'r, Row, E> Eval<'r, Row> for Not<E>
where
    E: Eval<'r, Row>,
    E::SqlType: Negate3<'r>,
    <E::SqlType as Negate>::Out: Repr<'r>,
    Self: Expr<SqlType = <E::SqlType as Negate>::Out>,
{
    fn eval(&'r self, row: &'r Row) -> Rust<'r, Self::SqlType> {
        <E::SqlType as Negate3<'r>>::not3(self.inner.eval(row))
    }
}

// ── LIKE ───────────────────────────────────────────────────────────────────

impl<'r, Row, L, R> Eval<'r, Row> for Like<L, R>
where
    L: Eval<'r, Row>,
    R: Eval<'r, Row>,
    L::SqlType: Like3<'r, R::SqlType>,
    R::SqlType: Repr<'r>,
    <L::SqlType as CompareWith<R::SqlType>>::Out: Repr<'r>,
    Self: Expr<SqlType = <L::SqlType as CompareWith<R::SqlType>>::Out>,
{
    /// Stability class (`D-103`): **provider-defined**. This evaluates
    /// case-sensitively with `%` and `_` wildcards, which matches PostgreSQL
    /// `LIKE` and SQLite with `PRAGMA case_sensitive_like=ON`; MySQL's default
    /// collation and SQLite's default are case-insensitive for ASCII.
    fn eval(&'r self, row: &'r Row) -> Rust<'r, Self::SqlType> {
        <L::SqlType as Like3<'r, R::SqlType>>::like3(self.left.eval(row), self.right.eval(row))
    }
}

/// SQL `LIKE` glob match: `%` matches any run, `_` matches one character.
/// Greedy with backtracking, over `char` boundaries, and — unlike the obvious
/// `Vec<char>` version — it allocates nothing, which is the whole point of
/// `Text` evaluating to `&str`.
fn like_match(text: &str, pattern: &str) -> bool {
    let (mut ti, mut pi) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    loop {
        match (text[ti..].chars().next(), pattern[pi..].chars().next()) {
            // Text exhausted: the rest of the pattern must be all `%`.
            (None, _) => return pattern[pi..].chars().all(|c| c == '%'),
            // `%` is checked before literal equality, so a `%` in the pattern
            // is always a wildcard even when the text also holds a `%`.
            (Some(_), Some('%')) => {
                star = Some(pi);
                mark = ti;
                pi += 1;
            }
            (Some(t), Some(p)) if p == '_' || p == t => {
                ti += t.len_utf8();
                pi += p.len_utf8();
            }
            (Some(_), _) => match star {
                // `mark <= ti < text.len()` here, so this index is in bounds.
                Some(s) => {
                    pi = s + 1;
                    mark += text[mark..].chars().next().map_or(1, char::len_utf8);
                    ti = mark;
                }
                None => return false,
            },
        }
    }
}

// `IsNull` / `IsNotNull` now HAVE `Eval` impls. The note that used to stand
// here said there was "no honest in-memory answer" because Phase 1 had no
// nullable columns; `Nullable<T>` supplies one.

impl<'r, Row, E> Eval<'r, Row> for IsNull<E>
where
    E: Eval<'r, Row>,
    E::SqlType: NullCheck<'r>,
    Self: Expr<SqlType = Boolean>,
{
    fn eval(&'r self, row: &'r Row) -> bool {
        <E::SqlType as NullCheck<'r>>::is_null(self.inner.eval(row))
    }
}

impl<'r, Row, E> Eval<'r, Row> for IsNotNull<E>
where
    E: Eval<'r, Row>,
    E::SqlType: NullCheck<'r>,
    Self: Expr<SqlType = Boolean>,
{
    fn eval(&'r self, row: &'r Row) -> bool {
        !<E::SqlType as NullCheck<'r>>::is_null(self.inner.eval(row))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Entity — binds a Rust struct to a `table!` module
// ═══════════════════════════════════════════════════════════════════════════

/// Binds a Rust row struct to the table it is stored in.
///
/// One impl per struct; generated by [`crate::entity!`].
pub trait Entity: Sized {
    /// The `Marker` type from this struct's `table!` invocation.
    type Table: Table;
}

// ═══════════════════════════════════════════════════════════════════════════
// Predicate accumulation
// ═══════════════════════════════════════════════════════════════════════════

/// The predicate of a query that has not been filtered yet.
#[derive(Debug, Clone, Copy)]
pub struct AlwaysTrue;

// No columns, so every table -- the identity of the conjunction.
impl<T> crate::expr::BelongsTo<T> for AlwaysTrue {}

impl Expr for AlwaysTrue {
    type SqlType = Boolean;
    fn write_to(&self, sql: &mut String, _params: &mut Vec<crate::SqlValue>) {
        // Unreachable in practice: `Rows::to_sql` skips the WHERE clause
        // entirely while the predicate is still `AlwaysTrue`.
        sql.push_str("1 = 1");
    }
}

impl<'r, Row> Eval<'r, Row> for AlwaysTrue {
    fn eval(&'r self, _row: &'r Row) -> bool {
        true
    }
}

/// How a new `.filter(..)` combines with what the query already has.
///
/// `AlwaysTrue` is replaced; anything else is `AND`-ed, matching
/// [`Query::filter`](crate::Query::filter). Written as one impl per node type
/// rather than a blanket impl because a blanket impl plus the `AlwaysTrue`
/// impl is an `E0119` coherence conflict — rustc cannot know `AlwaysTrue`
/// is excluded by the blanket's bounds.
pub trait Conj<P2> {
    /// The predicate type after conjunction.
    type Out;
    /// Combine.
    fn conj(self, rhs: P2) -> Self::Out;
}

impl<P2> Conj<P2> for AlwaysTrue {
    type Out = P2;
    fn conj(self, rhs: P2) -> P2 {
        rhs
    }
}

macro_rules! conj_binary {
    ($($node:ident),* $(,)?) => {$(
        impl<L, R, P2> Conj<P2> for $node<L, R>
        where
            Self: Expr,
            P2: Expr,
            <Self as Expr>::SqlType: LogicWith<P2::SqlType>,
        {
            type Out = And<Self, P2>;
            fn conj(self, rhs: P2) -> And<Self, P2> {
                crate::expr::and_node(self, rhs)
            }
        }
    )*};
}
conj_binary!(Eq, NotEq, Lt, LtEq, Gt, GtEq, And, Or, Like);

macro_rules! conj_unary {
    ($($node:ident),* $(,)?) => {$(
        impl<E, P2> Conj<P2> for $node<E>
        where
            Self: Expr,
            P2: Expr,
            <Self as Expr>::SqlType: LogicWith<P2::SqlType>,
        {
            type Out = And<Self, P2>;
            fn conj(self, rhs: P2) -> And<Self, P2> {
                crate::expr::and_node(self, rhs)
            }
        }
    )*};
}
conj_unary!(Not, IsNull, IsNotNull);

// ═══════════════════════════════════════════════════════════════════════════
// Rows — the query value
// ═══════════════════════════════════════════════════════════════════════════

/// Type-state: no `ORDER BY` yet, so `to_memory` can stream.
#[derive(Debug, Clone, Copy)]
pub struct Unordered;
/// Type-state: `ORDER BY` present, so `to_memory` must materialise.
#[derive(Debug, Clone, Copy)]
pub struct Ordered;

/// Renders one accumulated `ORDER BY` column into the SQL builder. A boxed
/// closure rather than a `fn` pointer because `table!`'s column types do not
/// derive `Default`, so the column value has to be captured.
type SqlStep<T> = Box<dyn Fn(Query<T, All<T>>) -> Query<T, All<T>>>;
/// Compares two rows by one accumulated `ORDER BY` column.
type RowCmp<Row> = Box<dyn Fn(&Row, &Row) -> Ordering>;

pub(crate) struct OrderPart<Row: Entity> {
    pub(crate) render: SqlStep<Row::Table>,
    pub(crate) cmp: RowCmp<Row>,
}

/// Builds one `ORDER BY` part: the SQL step and the in-memory comparator, from
/// a single column and direction.
///
/// Extracted so [`crate::boxed::BoxedRows`] orders through exactly this code
/// rather than a second copy — two copies would be two chances for the erased
/// and typed forms to sort differently.
pub(crate) fn order_part<Row, C>(column: C, desc: bool) -> OrderPart<Row>
where
    Row: Entity + 'static,
    C: Column<Table = Row::Table> + Copy + 'static,
    C: for<'x> Eval<'x, Row>,
    C::SqlType: Sortable,
{
    OrderPart {
        render: if desc {
            Box::new(move |q: Query<Row::Table, All<Row::Table>>| q.order_by_desc(column))
        } else {
            Box::new(move |q: Query<Row::Table, All<Row::Table>>| q.order_by(column))
        },
        cmp: Box::new(move |a: &Row, b: &Row| {
            let ord = <C::SqlType as Sortable>::compare(column.eval(a), column.eval(b));
            if desc {
                ord.reverse()
            } else {
                ord
            }
        }),
    }
}

/// A query value with two interpreters.
///
/// `Row` is the Rust struct, `P` is the accumulated predicate **as a type**,
/// and `O` is [`Unordered`] or [`Ordered`]. Build with [`query`].
///
/// `P` is a type parameter and the `ORDER BY` list is runtime data on purpose:
/// the predicate is where the type checking has to happen (a `Text` column
/// compared to an integer must not compile), and the clause list is
/// homogeneous data with nothing to prove. `O` is the one piece of shape
/// promoted to the type level, because it changes an operational guarantee —
/// see [`Rows::to_memory`].
pub struct Rows<Row: Entity, P, O> {
    // `pub(crate)` only so `crate::boxed::Rows::into_boxed` can move these out.
    // Still private to consumers -- the erased form is the supported way to get
    // at them.
    pub(crate) pred: P,
    pub(crate) filtered: bool,
    pub(crate) order: Vec<OrderPart<Row>>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    _ord: PhantomData<O>,
}

/// Begin a query over `Row`, in memory or in SQL.
///
/// # Where the `D-102` gate can and cannot live
///
/// It cannot live here. A `compile_fail` doctest in this crate cannot import
/// `linq_rs` (`D-020` forbids the dependency), so it can only ever pin half
/// the pair — and `compile_fail` does not check *which* error it got, so a
/// doctest that fails because of a missing import passes the gate just as
/// happily as one that fails for the intended reason. Measured: the positive
/// half fails with `E0432: unresolved import 'linq_rs'` while the negative
/// half passes for the wrong reason.
///
/// The gate belongs in a consumer test crate that depends on both, run in CI —
/// the shape `.github/scripts/itertools-interop.sh` already established. There
/// the two halves are a differential pair: the same expression must fail
/// before `.to_memory()` and compile after it.
pub fn query<Row: Entity>() -> Rows<Row, AlwaysTrue, Unordered> {
    Rows {
        pred: AlwaysTrue,
        filtered: false,
        order: Vec::new(),
        limit: None,
        offset: None,
        _ord: PhantomData,
    }
}

impl<Row: Entity, P, O> Rows<Row, P, O> {
    /// Add an `AND`-joined predicate. The predicate's *type* changes, which is
    /// how the query value records its own shape.
    pub fn filter<P2>(self, predicate: P2) -> Rows<Row, P::Out, O>
    where
        P: Conj<P2>,
        // D-028: the predicate's columns must belong to THIS row's table.
        // `to_memory` already enforced it through `Eval`; `to_sql` did not.
        P2: Expr + crate::expr::BelongsTo<Row::Table>,
        P2::SqlType: WhereClause,
    {
        Rows {
            pred: self.pred.conj(predicate),
            filtered: true,
            order: self.order,
            limit: self.limit,
            offset: self.offset,
            _ord: PhantomData,
        }
    }

    /// `LIMIT n`.
    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// `OFFSET n`.
    pub fn offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

    fn push_order<C>(mut self, column: C, desc: bool) -> Rows<Row, P, Ordered>
    where
        C: Column<Table = Row::Table> + Copy + 'static,
        C: for<'x> Eval<'x, Row>,
        C::SqlType: Sortable,
        Row: 'static,
    {
        self.order.push(order_part(column, desc));
        Rows {
            pred: self.pred,
            filtered: self.filtered,
            order: self.order,
            limit: self.limit,
            offset: self.offset,
            _ord: PhantomData,
        }
    }

    /// `ORDER BY column ASC`. Stable and cumulative, like
    /// [`Query::order_by`](crate::Query::order_by).
    ///
    /// The `Ord` bound falls on the column's [`Repr`], so `Float` columns are
    /// rejected here exactly as `f64` keys are rejected by `linq_rs`'s
    /// `order_by` (`D-018`).
    pub fn order_by<C>(self, column: C) -> Rows<Row, P, Ordered>
    where
        C: Column<Table = Row::Table> + Copy + 'static,
        C: for<'x> Eval<'x, Row>,
        C::SqlType: Sortable,
        Row: 'static,
    {
        self.push_order(column, false)
    }

    /// `ORDER BY column DESC`.
    pub fn order_by_desc<C>(self, column: C) -> Rows<Row, P, Ordered>
    where
        C: Column<Table = Row::Table> + Copy + 'static,
        C: for<'x> Eval<'x, Row>,
        C::SqlType: Sortable,
        Row: 'static,
    {
        self.push_order(column, true)
    }

    /// Interpreter one: render through [`Query`], the crate's existing SQL
    /// builder. Takes `&self`, so the same value can then be evaluated.
    pub fn to_sql(&self) -> QueryOutput
    where
        P: Expr + crate::expr::BelongsTo<Row::Table> + Clone + 'static,
        P::SqlType: WhereClause,
    {
        let mut q: Query<Row::Table, All<Row::Table>> = Query::new();
        if self.filtered {
            q = q.filter(self.pred.clone());
        }
        for part in &self.order {
            q = (part.render)(q);
        }
        if let Some(n) = self.limit {
            q = q.limit(n);
        }
        if let Some(n) = self.offset {
            q = q.offset(n);
        }
        q.to_sql()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// D-102 signposts (experiment — see the report; not necessarily kept)
// ═══════════════════════════════════════════════════════════════════════════

/// Never implemented. Its only job is to make the `E0277` for a
/// non-translatable operator read like a sentence.
pub trait CallToMemoryFirst {}

impl<Row: Entity, P, O> Rows<Row, P, O> {
    /// Not translatable to SQL (`D-019`: `terminal`). Call
    /// [`Rows::to_memory`] first.
    pub fn select_many<F>(self, _f: F) -> Self
    where
        Self: CallToMemoryFirst,
    {
        unreachable!()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Interpreter two: in memory
// ═══════════════════════════════════════════════════════════════════════════

impl<Row: Entity, P> Rows<Row, P, Unordered> {
    /// Interpreter two, streaming.
    ///
    /// Returns a named adaptor (`D-106`) that pulls one row at a time: no
    /// intermediate `Vec`, no clone, no dynamic dispatch — the predicate is a
    /// monomorphised type, so `eval` inlines to field accesses. This is the
    /// property the 115-line prototype lost when `run_in_memory` collected.
    ///
    /// This method exists only on `Rows<_, _, Unordered>`. Once `order_by` has
    /// been called the type is `Ordered` and the materialising
    /// [`Rows::to_memory_sorted`] is what you get instead —
    /// the cost is in the type, not in a footnote.
    pub fn to_memory<'a, I>(self, src: I) -> RowIter<'a, Row, I::IntoIter, P>
    where
        I: IntoIterator<Item = &'a Row>,
        Row: 'a,
        P: Expr + for<'x> Eval<'x, Row>,
        P::SqlType: for<'x> TruthValue<'x>,
    {
        RowIter {
            inner: src.into_iter(),
            pred: self.pred,
            skip: self.offset.unwrap_or(0),
            left: self.limit,
            _row: PhantomData,
        }
    }
}

/// Streaming in-memory interpretation of a [`Rows`] value. Named, not
/// `impl Iterator`, per `D-106`.
pub struct RowIter<'a, Row, I, P> {
    inner: I,
    pred: P,
    skip: u64,
    left: Option<u64>,
    _row: PhantomData<&'a Row>,
}

impl<'a, Row, I, P> Iterator for RowIter<'a, Row, I, P>
where
    I: Iterator<Item = &'a Row>,
    Row: 'a,
    P: Expr + for<'x> Eval<'x, Row>,
    P::SqlType: for<'x> TruthValue<'x>,
{
    type Item = &'a Row;

    fn next(&mut self) -> Option<&'a Row> {
        if self.left == Some(0) {
            return None;
        }
        for row in self.inner.by_ref() {
            // The single place three values collapse to two: TRUE keeps the
            // row, FALSE and NULL both drop it — SQL's `WHERE` rule exactly.
            if !<P::SqlType as TruthValue<'_>>::is_true(self.pred.eval(row)) {
                continue;
            }
            if self.skip > 0 {
                self.skip -= 1;
                continue;
            }
            if let Some(n) = self.left.as_mut() {
                *n -= 1;
            }
            return Some(row);
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, hi) = self.inner.size_hint();
        let hi = match (hi, self.left) {
            (Some(h), Some(l)) => Some(h.min(l as usize)),
            (Some(h), None) => Some(h),
            (None, Some(l)) => Some(l as usize),
            (None, None) => None,
        };
        (0, hi)
    }
}

impl<Row: Entity, P> Rows<Row, P, Ordered> {
    /// Interpreter two, materialising — the only honest shape for `ORDER BY`.
    ///
    /// Collects references (never clones rows), sorts by the accumulated keys,
    /// then applies `OFFSET`/`LIMIT`. `linq_rs`'s own `order_by` materialises
    /// too; this does not make it worse.
    pub fn to_memory_sorted<'a, I>(self, src: I) -> SortedRows<'a, Row>
    where
        I: IntoIterator<Item = &'a Row>,
        Row: 'a,
        P: Expr + for<'x> Eval<'x, Row>,
        P::SqlType: for<'x> TruthValue<'x>,
    {
        let pred = &self.pred;
        let mut kept: Vec<&'a Row> = src
            .into_iter()
            .filter(|r| <P::SqlType as TruthValue<'_>>::is_true(pred.eval(r)))
            .collect();
        let order = &self.order;
        kept.sort_by(|a, b| {
            for part in order {
                match (part.cmp)(a, b) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            Ordering::Equal
        });
        let start = (self.offset.unwrap_or(0) as usize).min(kept.len());
        let end = match self.limit {
            Some(n) => (start + n as usize).min(kept.len()),
            None => kept.len(),
        };
        kept.truncate(end);
        kept.drain(..start);
        SortedRows {
            rows: kept.into_iter(),
        }
    }
}

/// Materialised, sorted in-memory interpretation of a [`Rows`] value.
pub struct SortedRows<'a, Row> {
    rows: std::vec::IntoIter<&'a Row>,
}

impl<'a, Row> Iterator for SortedRows<'a, Row> {
    type Item = &'a Row;
    fn next(&mut self) -> Option<&'a Row> {
        self.rows.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rows.size_hint()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ReadField — the Rust-field -> SQL-type bridge `entity!` expands to
// ═══════════════════════════════════════════════════════════════════════════

/// How a Rust struct field of type `F` is read as SQL type `Self`.
///
/// This replaces `entity!`'s per-SQL-type macro arms. The reason is
/// mechanical: `Nullable<Text>` is not an `ident`, so the old
/// `(@col $row:ty, $col:path, Text, $field:ident)` dispatch could not match
/// it. Moving the dispatch from the macro to the trait system also makes
/// nullability compositional — `Nullable<S>` is **one** impl that lifts every
/// base impl, instead of four more macro arms.
///
/// `D-025` is preserved verbatim: the numeric impls are keyed on
/// `i64: From<F>` / `f64: From<F>`, which is exactly the lossless-widening
/// whitelist the macro used to spell as `::core::convert::From::from`. An
/// `f64` field declared `Integer` is still a compile error at the `entity!`
/// call site.
pub trait ReadField<'r, F: ?Sized>: Repr<'r> {
    /// Read the field.
    fn read(field: &'r F) -> Self::Rust;
}

impl<'r, F: Copy> ReadField<'r, F> for Integer
where
    i64: From<F>,
{
    fn read(field: &'r F) -> i64 {
        i64::from(*field)
    }
}

impl<'r, F: Copy> ReadField<'r, F> for Float
where
    f64: From<F>,
{
    fn read(field: &'r F) -> f64 {
        f64::from(*field)
    }
}

impl<'r> ReadField<'r, bool> for Boolean {
    fn read(field: &'r bool) -> bool {
        *field
    }
}

/// Any field that derefs to `str`: `String`, `Box<str>`, `Rc<str>`,
/// `Arc<str>`, `Cow<str>`, `&str`. This is deliberately as wide as the old
/// `&row.$field[..]`, which accepted all of those through autoderef — a
/// narrower whitelist here would have been a silent API break.
impl<'r, F> ReadField<'r, F> for Text
where
    F: core::ops::Deref<Target = str> + ?Sized,
{
    fn read(field: &'r F) -> &'r str {
        field
    }
}

/// The one impl that makes every base type nullable. `Option<F>` read as
/// `Nullable<S>` is `S` read through the `Some`.
impl<'r, S, F> ReadField<'r, Option<F>> for Nullable<S>
where
    S: ReadField<'r, F>,
{
    fn read(field: &'r Option<F>) -> Option<S::Rust> {
        field.as_ref().map(<S as ReadField<'r, F>>::read)
    }
}

/// Free function so `entity!` can name the SQL type by turbofish
/// (`read_field::<Nullable<Text>, _>(&row.nick)`) and let the field type be
/// inferred. Written as a method call the `Self` type would be ambiguous.
pub fn read_field<'r, S, F>(field: &'r F) -> Rust<'r, S>
where
    S: ReadField<'r, F>,
    F: ?Sized,
{
    S::read(field)
}

// ═══════════════════════════════════════════════════════════════════════════
// entity! — the usability answer
// ═══════════════════════════════════════════════════════════════════════════

/// Bind a Rust struct to a `table!` module and give every column its accessor.
///
/// ```rust
/// use linq_rs_sql::*;
/// linq_rs_sql::table! { staff (id) { id -> Integer, name -> Text } }
///
/// struct Person { id: i64, name: String }
///
/// linq_rs_sql::entity! {
///     Person => staff {
///         id: Integer = id,
///         name: Text = name,
///     }
/// }
/// ```
///
/// The SQL type is restated so the macro knows which normalisation to apply,
/// and the compiler checks it against the `table!` declaration — writing
/// `name: Integer = name` fails to build.
/// A field whose Rust type cannot convert losslessly to the declared SQL type
/// is a compile error, not a silent truncation:
///
/// ```compile_fail
/// use linq_rs_sql::prelude::*;
/// table! { m (id) { id -> Integer, n -> Integer } }
/// pub struct M { pub id: i64, pub n: f64 }
/// // error[E0277]: the trait bound `i64: From<f64>` is not satisfied
/// entity! { M => m { id: Integer = id, n: Integer = n } }
/// ```
#[macro_export]
macro_rules! entity {
    // Opt out of `FromRow` generation. Required, not a convenience: a struct
    // with a borrowed field (`name: &'d str`) or a field that is not a column
    // cannot have a generated `FromRow`, and both are legal today.
    ($row:ty => $table:ident { $($col:ident : $ty:ty = $field:ident),* $(,)? } no_from_row) => {
        $crate::entity!(@base $row => $table { $($col : $ty = $field),* });
    };
    ($row:ty => $table:ident { $($col:ident : $ty:ty = $field:ident),* $(,)? }) => {
        $crate::entity!(@base $row => $table { $($col : $ty = $field),* });

        // The reverse direction: one row of a result set -> one `$row`.
        // Columns are matched by NAME, resolved once per result set. See D-029.
        impl $crate::from_row::FromRow for $row {
            const COLUMNS: &'static [&'static str] =
                &[$(<$table::$col as $crate::Column>::NAME),*];

            fn from_row<'r_, R_>(
                row: &R_,
                layout: &$crate::from_row::Layout<Self>,
            ) -> ::core::result::Result<Self, $crate::from_row::RowError>
            where
                R_: $crate::from_row::RowSource<'r_> + ?Sized,
            {
                #[allow(unused_imports)]
                use $crate::types::*;
                let mut i_ = 0usize;
                ::core::result::Result::Ok(Self {
                    $($field: {
                        let at_ = layout.position(i_);
                        let name_ = <$table::$col as $crate::Column>::NAME;
                        i_ += 1;
                        <$ty as $crate::from_row::LoadField<'r_, _>>::load_field(
                            row.value_at(at_, name_)?,
                            name_,
                        )?
                    },)*
                })
            }
        }
    };
    (@base $row:ty => $table:ident { $($col:ident : $ty:ty = $field:ident),* $(,)? }) => {
        impl $crate::rows::Entity for $row {
            type Table = $table::Marker;
        }
        // The impls are generated inside an anonymous `const` so the SQL type
        // markers can be brought into scope without the caller importing them
        // and without leaking the glob into the caller's namespace. Trait impls
        // register globally regardless of the block they are written in.
        //
        // `$ty` is a `:ty`, not an `:ident`, because `Nullable<Text>` is a type
        // and not a token the macro can match per-variant -- which is exactly
        // what makes nullable columns expressible. That means `$ty` resolves in
        // the scope of the expansion, so the markers have to be here.
        const _: () = {
            #[allow(unused_imports)]
            use $crate::types::*;
            $(
                impl<'r> $crate::rows::Eval<'r, $row> for $table::$col {
                    fn eval(&'r self, row: &'r $row) -> $crate::rows::Rust<'r, $ty> {
                        $crate::rows::read_field::<$ty, _>(&row.$field)
                    }
                }
            )*
        };
    };
}
