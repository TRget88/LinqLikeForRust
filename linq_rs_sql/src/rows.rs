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
use crate::expr::{And, Eq, Expr, Gt, GtEq, IsNull, Like, Lt, LtEq, Not, NotEq, Or};
use crate::query::{All, Query, QueryOutput, Table};
use crate::types::{Boolean, Float, Integer, Text};
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

// ── comparison nodes ───────────────────────────────────────────────────────

macro_rules! eval_compare {
    ($node:ident, $op:tt) => {
        impl<'r, Row, L, R> Eval<'r, Row> for $node<L, R>
        where
            L: Eval<'r, Row>,
            R: Expr<SqlType = L::SqlType> + Eval<'r, Row>,
            L::SqlType: Repr<'r>,
            Rust<'r, L::SqlType>: PartialOrd,
            Self: Expr<SqlType = Boolean>,
        {
            fn eval(&'r self, row: &'r Row) -> bool {
                self.left.eval(row) $op self.right.eval(row)
            }
        }
    };
}

eval_compare!(Eq, ==);
eval_compare!(NotEq, !=);
eval_compare!(Lt, <);
eval_compare!(LtEq, <=);
eval_compare!(Gt, >);
eval_compare!(GtEq, >=);

// ── logical nodes ──────────────────────────────────────────────────────────

impl<'r, Row, L, R> Eval<'r, Row> for And<L, R>
where
    L: Expr<SqlType = Boolean> + Eval<'r, Row>,
    R: Expr<SqlType = Boolean> + Eval<'r, Row>,
{
    fn eval(&'r self, row: &'r Row) -> bool {
        // Short-circuits, like SQL is permitted (but not required) to.
        self.left.eval(row) && self.right.eval(row)
    }
}

impl<'r, Row, L, R> Eval<'r, Row> for Or<L, R>
where
    L: Expr<SqlType = Boolean> + Eval<'r, Row>,
    R: Expr<SqlType = Boolean> + Eval<'r, Row>,
{
    fn eval(&'r self, row: &'r Row) -> bool {
        self.left.eval(row) || self.right.eval(row)
    }
}

impl<'r, Row, E> Eval<'r, Row> for Not<E>
where
    E: Expr<SqlType = Boolean> + Eval<'r, Row>,
{
    fn eval(&'r self, row: &'r Row) -> bool {
        !self.inner.eval(row)
    }
}

// ── LIKE ───────────────────────────────────────────────────────────────────

impl<'r, Row, L, R> Eval<'r, Row> for Like<L, R>
where
    L: Expr<SqlType = Text> + Eval<'r, Row>,
    R: Expr<SqlType = Text> + Eval<'r, Row>,
{
    /// Stability class (`D-103`): **provider-defined**. This evaluates
    /// case-sensitively with `%` and `_` wildcards, which matches PostgreSQL
    /// `LIKE` and SQLite with `PRAGMA case_sensitive_like=ON`; MySQL's default
    /// collation and SQLite's default are case-insensitive for ASCII.
    fn eval(&'r self, row: &'r Row) -> bool {
        like_match(self.left.eval(row), self.right.eval(row))
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

// NOTE, deliberately: there is **no** `Eval` impl for `IsNull<E>`.
// Phase 1 has no nullable columns, so there is no honest in-memory answer,
// and `D-103` says nulls are where the three interpreters disagree worst.
// Under `D-102` the absence of the impl *is* the design: `.is_null()` in a
// two-interpreter query is a compile error, not a silently-`false` row.

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
            Self: Expr<SqlType = Boolean>,
            P2: Expr<SqlType = Boolean>,
        {
            type Out = And<Self, P2>;
            fn conj(self, rhs: P2) -> And<Self, P2> {
                crate::column::BoolOps::and(self, rhs)
            }
        }
    )*};
}
conj_binary!(Eq, NotEq, Lt, LtEq, Gt, GtEq, And, Or, Like);

macro_rules! conj_unary {
    ($($node:ident),* $(,)?) => {$(
        impl<E, P2> Conj<P2> for $node<E>
        where
            Self: Expr<SqlType = Boolean>,
            P2: Expr<SqlType = Boolean>,
        {
            type Out = And<Self, P2>;
            fn conj(self, rhs: P2) -> And<Self, P2> {
                crate::column::BoolOps::and(self, rhs)
            }
        }
    )*};
}
conj_unary!(Not, IsNull);

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

struct OrderPart<Row: Entity> {
    render: SqlStep<Row::Table>,
    cmp: RowCmp<Row>,
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
    pred: P,
    filtered: bool,
    order: Vec<OrderPart<Row>>,
    limit: Option<u64>,
    offset: Option<u64>,
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
        P2: Expr<SqlType = Boolean>,
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
        self.order.push(OrderPart {
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
        });
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
        P: Expr<SqlType = Boolean> + Clone + 'static,
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
        P: Expr<SqlType = Boolean> + for<'x> Eval<'x, Row>,
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
    P: Expr<SqlType = Boolean> + for<'x> Eval<'x, Row>,
{
    type Item = &'a Row;

    fn next(&mut self) -> Option<&'a Row> {
        if self.left == Some(0) {
            return None;
        }
        for row in self.inner.by_ref() {
            if !self.pred.eval(row) {
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
        P: Expr<SqlType = Boolean> + for<'x> Eval<'x, Row>,
    {
        let pred = &self.pred;
        let mut kept: Vec<&'a Row> = src.into_iter().filter(|r| pred.eval(r)).collect();
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
#[macro_export]
macro_rules! entity {
    ($row:ty => $table:ident { $($col:ident : $ty:ident = $field:ident),* $(,)? }) => {
        impl $crate::rows::Entity for $row {
            type Table = $table::Marker;
        }
        $( $crate::entity!(@col $row, $table::$col, $ty, $field); )*
    };
    (@col $row:ty, $col:path, Integer, $field:ident) => {
        impl<'r> $crate::rows::Eval<'r, $row> for $col {
            fn eval(&'r self, row: &'r $row) -> i64 { row.$field as i64 }
        }
    };
    (@col $row:ty, $col:path, Text, $field:ident) => {
        impl<'r> $crate::rows::Eval<'r, $row> for $col {
            fn eval(&'r self, row: &'r $row) -> &'r str { &row.$field[..] }
        }
    };
    (@col $row:ty, $col:path, Boolean, $field:ident) => {
        impl<'r> $crate::rows::Eval<'r, $row> for $col {
            fn eval(&'r self, row: &'r $row) -> bool { row.$field }
        }
    };
    (@col $row:ty, $col:path, Float, $field:ident) => {
        impl<'r> $crate::rows::Eval<'r, $row> for $col {
            fn eval(&'r self, row: &'r $row) -> f64 { row.$field as f64 }
        }
    };
}
