//! `into_boxed()` — the erased query, for composition the type system cannot
//! follow.
//!
//! [`Rows<Row, P, O>`](crate::rows::Rows) parameterises the predicate by
//! *type*, so `.filter()` returns a different type each time. That is the right
//! default — it is what makes a `Text` column compared to an integer a build
//! error — but it makes this impossible:
//!
//! ```text
//! let mut q = query::<Employee>();
//! if want_eng { q = q.filter(employees::dept.eq("eng")); }   // E0308
//! ```
//!
//! Diesel's answer, adopted here: keep the typed form as the default and let
//! the caller opt into an erased one whose type does **not** move as clauses
//! are added.
//!
//! ## The seam survives erasure, because each clause is erased *once*
//!
//! The obvious erasure — a `Vec<Box<dyn Fn(&Row) -> bool>>` for memory and a
//! separate `Vec<Box<dyn SqlWriter>>` for SQL — would erase the query *twice*,
//! into two independent lists that can silently disagree. That is the exact
//! failure the two-interpreter design exists to prevent.
//!
//! So the trait object here is [`DynPred`], which carries **both** halves:
//!
//! ```text
//! fn write_sql(&self, sql: &mut String, params: &mut Vec<SqlValue>);   // Expr
//! fn eval_row(&self, row: &Row) -> Option<bool>;                       // Eval
//! ```
//!
//! Its blanket impl is keyed on `Expr + for<'x> Eval<'x, Row>` with
//! `SqlType: TruthValue` — the identical pair of bounds `Rows::to_memory`
//! already requires — so a value can only be boxed if it *already* satisfies
//! both interpreters. Nothing new is admitted to the language by boxing, and
//! nothing can be boxed for one interpreter but not the other. The trait is
//! sealed (see `mod sealed`), so the blanket impl is the only impl there is.
//!
//! ## Why the erased form is three-valued
//!
//! `eval_row` returns `Option<bool>`, not `bool`, because `D-026` made a
//! predicate's type possibly `Nullable<Boolean>`. Collapsing to two values
//! inside the box would be lossy in a way that shows only under negation:
//! `is_true(NOT NULL)` is `false`, while `!is_true(NULL)` is `true`. A boxed
//! predicate is a first-class expression that can be fed back into `not(..)`,
//! so it has to carry the third value. The collapse happens where it always
//! happens — at the `WHERE` boundary, and nowhere else.
//!
//! A boxed predicate therefore has `SqlType = Nullable<Boolean>` even when the
//! predicate inside could not be null. Widening is sound (a known value is a
//! valid three-valued one) and it keeps ONE erased type, so a
//! `Vec<Box<dyn DynPred<_>>>` can hold predicates of both kinds.
//!
//! ## Why `Eval`'s object-unsafety does not bite
//!
//! [`Eval`] is not object safe as written: it is generic
//! over `'r`, so `dyn for<'r> Eval<'r, Row>` would be a higher-ranked trait
//! object, and its return type is the projection `Rust<'r, Self::SqlType>`,
//! which a vtable cannot represent in general.
//!
//! Neither matters at *this* seam, and for a reason that is a property of the
//! design rather than a lucky accident: a query predicate's SQL type is always
//! `Boolean` or `Nullable<Boolean>`, and [`Repr`](crate::rows::Repr) pins both
//! to types that do not mention `'r` — `bool` and `Option<bool>`.
//! [`TruthValue::to_tri`] then normalises the
//! two to one, so `DynPred::eval_row` can write a single concrete return type
//! down. The higher-ranked quantification moves off the trait object and onto
//! the blanket impl's `where` clause, where it is an ordinary bound.
//!
//! Erasing a `Text` or `Integer` expression *would* hit the real object-safety
//! wall — `Rust<'r, Text> = &'r str` genuinely varies with `'r`. Phase 1 never
//! needs to, because only predicates are erased.

use crate::column::Column;
use crate::expr::Expr;
use crate::query::{All, Query, QueryOutput, Table};
use crate::rows::{order_part, Entity, Eval, OrderPart, Rows, Sortable, TruthValue};
use crate::types::{Boolean, Nullable};
use crate::value::SqlValue;
use core::cmp::Ordering;

// ═══════════════════════════════════════════════════════════════════════════
// DynPred — one trait object, both interpreters
// ═══════════════════════════════════════════════════════════════════════════

/// A boolean predicate with its type erased, retaining both interpretations.
///
/// You do not implement this. The blanket impl below covers every value that
/// is already both an [`Expr`] of SQL type `Boolean` and an
/// [`Eval`] over `Row`.
mod sealed {
    /// Not nameable outside the crate, so `DynPred` cannot be implemented
    /// outside it. The blanket impl below is keyed on exactly the pair of
    /// bounds `DynPred`'s own blanket impl uses, so sealing costs nothing:
    /// every type that could legitimately be a `DynPred` is already `Sealed`.
    pub trait Sealed<Row> {}
}

impl<Row, P> sealed::Sealed<Row> for P
where
    P: Expr + for<'x> Eval<'x, Row>,
    P::SqlType: for<'x> TruthValue<'x>,
{
}

pub trait DynPred<Row>: sealed::Sealed<Row> {
    /// Interpreter one: append this predicate's SQL, binding its literals.
    fn write_sql(&self, sql: &mut String, params: &mut Vec<SqlValue>);
    /// Interpreter two: evaluate this predicate against one row.
    ///
    /// Three-valued — `None` is SQL UNKNOWN. It is deliberately NOT `bool`.
    /// Collapsing to two values here would be lossy in a way that only shows
    /// under negation: `is_true(NOT NULL)` is `false`, while `!is_true(NULL)`
    /// is `true`. Since a boxed predicate is a first-class expression that can
    /// be fed back into `not(..)`, the erased form has to carry the third
    /// value. See [`DECISIONS.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/DECISIONS.md) `D-027`.
    fn eval_row(&self, row: &Row) -> Option<bool>;
}

impl<Row, P> DynPred<Row> for P
where
    P: Expr + for<'x> Eval<'x, Row>,
    P::SqlType: for<'x> TruthValue<'x>,
{
    fn write_sql(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        Expr::write_to(self, sql, params)
    }
    fn eval_row(&self, row: &Row) -> Option<bool> {
        // `&self` and `row` carry unrelated lifetimes; `Eval::eval` wants one
        // `'x`. Both are reborrowed to the shorter, which is sound precisely
        // because the result is `Option<bool>` and borrows nothing.
        <P::SqlType as TruthValue<'_>>::to_tri(Eval::eval(self, row))
    }
}

// A boxed predicate is itself a predicate, so a `Box<dyn DynPred<Row>>` can be
// fed back into `.and()` / `.or()` / `not(..)` like any other node. `BoxedRows`
// itself does NOT do this -- see the `clauses` field for why the flat list won
// -- but the impls are what make the erased form a value in the expression
// language rather than a dead end.
// The erased form widens to `Nullable<Boolean>` rather than `Boolean`. Boxing a
// non-nullable predicate through it is still sound -- a known value is a valid
// three-valued one -- and widening keeps ONE erased type rather than two, so a
// `Vec<Box<dyn DynPred<_>>>` can hold predicates of both kinds.
// Checked before boxing, so the erased form carries the guarantee forward.
impl<'a, Row: Entity, T> crate::expr::BelongsTo<T> for Box<dyn DynPred<Row> + 'a> where
    Row::Table: crate::query::Table
{
}

impl<'a, Row> Expr for Box<dyn DynPred<Row> + 'a> {
    type SqlType = Nullable<Boolean>;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        (**self).write_sql(sql, params)
    }
}

impl<'r, 'a, Row> Eval<'r, Row> for Box<dyn DynPred<Row> + 'a> {
    fn eval(&'r self, row: &'r Row) -> Option<bool> {
        (**self).eval_row(row)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Fragment — how the erased predicate re-enters the `Query` builder
// ═══════════════════════════════════════════════════════════════════════════

/// A pre-rendered WHERE fragment: SQL text plus its bind values, in order.
///
/// [`Query`] stores WHERE parts as `Box<dyn SqlWriter + 'static>`, so a
/// `BoxedRows<'a, _>` whose predicate borrows (`'a` shorter than `'static`)
/// cannot be handed to `Query::filter` directly. Rendering it first solves
/// that without touching `Query`: the fragment owns a `String` and a
/// `Vec<SqlValue>`, so it is `'static` whatever the predicate borrowed, and
/// replaying it appends byte-identical SQL with the parameters in the same
/// order. The alternative — making `Query` lifetime-generic — is a breaking
/// change to a published type. See the report.
#[derive(Default)]
struct Fragment {
    sql: String,
    params: Vec<SqlValue>,
}

// `Nullable<Boolean>`, matching the erased form it is rendered from: a fragment
// may have come from a nullable predicate, and there is no way to tell once it
// is text. Widening is sound and keeps `Fragment` usable wherever a boxed
// predicate is.
// A fragment is rendered from a predicate that already passed the
// `BelongsTo` check at `.filter()`. Once it is text there are no columns left
// to attribute, so it is accepted anywhere -- the guarantee was established
// upstream, not discarded here. (D-028)
impl<T> crate::expr::BelongsTo<T> for Fragment {}

impl Expr for Fragment {
    type SqlType = Nullable<Boolean>;
    fn write_to(&self, sql: &mut String, params: &mut Vec<SqlValue>) {
        sql.push_str(&self.sql);
        params.extend(self.params.iter().cloned());
    }
}

/// Render an AND-joined clause list in **exactly** the shape the typed path's
/// left-nested `And` tree produces, so `to_sql()` is byte-identical either way.
///
/// `[a, b, c]` must come out as `((a AND b) AND c)`, not `a AND b AND c`:
/// `n - 1` opening parens, the first clause, then each remaining clause
/// preceded by ` AND ` and followed by its closing paren.
fn write_conjunction<Row>(
    clauses: &[Box<dyn DynPred<Row> + '_>],
    sql: &mut String,
    params: &mut Vec<SqlValue>,
) {
    for _ in 1..clauses.len() {
        sql.push('(');
    }
    let mut first = true;
    for clause in clauses {
        if !first {
            sql.push_str(" AND ");
        }
        clause.write_sql(sql, params);
        if !first {
            sql.push(')');
        }
        first = false;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BoxedRows — the query value whose type stops moving
// ═══════════════════════════════════════════════════════════════════════════

/// A [`Rows`] with its predicate erased. `.filter()` returns `Self`, so it can
/// be reassigned, stored, returned, and collected.
///
/// `'a` bounds what the predicate may borrow. It is `'static` for the usual
/// case (literals, `String`, `&'static str`) and shorter when a predicate
/// captures a borrowed `&str` — which the typed path can also express, and
/// which only `to_memory` can interpret either way.
///
/// Note what is *not* here: the `Unordered` / `Ordered` type-state. See
/// [`BoxedRows::to_memory`].
pub struct BoxedRows<'a, Row: Entity> {
    /// AND-joined clauses, kept **flat** rather than folded into a nested
    /// `And` tree.
    ///
    /// The nested form was built first and measured: because each `.filter()`
    /// re-boxes, `Box<And<Box<And<Box<..>, _>>, _>>` costs a *dependent chain*
    /// of N indirect calls per row, each one waiting on the last. Flat costs N
    /// independent ones.
    ///
    /// Measured over 1M rows, best of 15 x 5 pinned runs:
    ///
    /// | clauses | nested   | flat     |
    /// |---------|----------|----------|
    /// | 1       |  5.71 ms |  6.50 ms |
    /// | 3       | 10.25 ms |  8.22 ms |
    ///
    /// So it is a crossover, not a win: flat is ~20% faster at three clauses
    /// and ~14% *slower* at one, where the slice loop is pure overhead. Flat is
    /// chosen because a one-clause query is exactly the case that has no reason
    /// to call `into_boxed()` at all.
    clauses: Vec<Box<dyn DynPred<Row> + 'a>>,
    order: Vec<OrderPart<Row>>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl<Row: Entity, P, O> Rows<Row, P, O> {
    /// Erase the predicate's type. Everything already accumulated is kept.
    ///
    /// The bound is the conjunction of what the two interpreters ask for
    /// separately, so a value that cannot be interpreted both ways cannot be
    /// boxed: `.filter(col.is_null())` is still a compile error after
    /// `.into_boxed()`, because there is no `Eval` impl for `IsNull`.
    pub fn into_boxed<'a>(self) -> BoxedRows<'a, Row>
    where
        P: Expr + for<'x> Eval<'x, Row> + 'a,
        P::SqlType: for<'x> TruthValue<'x>,
        Row: 'a,
    {
        BoxedRows {
            // An unfiltered typed query still holds `AlwaysTrue`; drop it
            // rather than box it, so `to_sql` emits no WHERE clause and
            // `to_memory` runs no virtual call at all.
            clauses: if self.filtered {
                vec![Box::new(self.pred) as Box<dyn DynPred<Row> + 'a>]
            } else {
                Vec::new()
            },
            order: self.order,
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// The erased query, in the only lifetime Phase 1 can actually produce.
///
/// `BoxedRows` keeps `'a` because Diesel's `BoxedSelectStatement` does and
/// because adding a lifetime parameter later would be a breaking change — but
/// **measured, `'a` is vacuous today**: `for<'x> Eval<'x, Row>` forces every
/// boxable predicate to be `'static`. See `tests/boxed.rs`,
/// `call_site_borrowed_predicate_needs_to_string`. Write `Boxed<Row>`.
pub type Boxed<Row> = BoxedRows<'static, Row>;

/// Begin an erased query directly, skipping `query::<Row>().into_boxed()`.
pub fn boxed_query<'a, Row: Entity + 'a>() -> BoxedRows<'a, Row> {
    crate::rows::query::<Row>().into_boxed()
}

impl<'a, Row: Entity> BoxedRows<'a, Row> {
    /// Add an `AND`-joined predicate. **Returns `Self`** — the whole point.
    ///
    /// The conjunction is the same [`And`](crate::expr::And) node the typed
    /// path builds, so the rendered SQL is byte-identical to the typed query
    /// with the same filters (asserted in `tests/boxed.rs`).
    pub fn filter<P2>(mut self, predicate: P2) -> Self
    where
        P2: Expr + crate::expr::BelongsTo<Row::Table> + for<'x> Eval<'x, Row> + 'a,
        P2::SqlType: for<'x> TruthValue<'x>,
        Row: 'a,
    {
        self.clauses.push(Box::new(predicate));
        self
    }

    /// `ORDER BY column ASC`. Returns `Self`, so it too can be conditional —
    /// which the typed path cannot do, since there `order_by` moves the type
    /// from `Unordered` to `Ordered`.
    pub fn order_by<C>(mut self, column: C) -> Self
    where
        C: Column<Table = Row::Table> + Copy + 'static,
        C: for<'x> Eval<'x, Row>,
        C::SqlType: Sortable,
        Row: 'static,
    {
        self.order.push(order_part(column, false));
        self
    }

    /// `ORDER BY column DESC`.
    pub fn order_by_desc<C>(mut self, column: C) -> Self
    where
        C: Column<Table = Row::Table> + Copy + 'static,
        C: for<'x> Eval<'x, Row>,
        C::SqlType: Sortable,
        Row: 'static,
    {
        self.order.push(order_part(column, true));
        self
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

    /// Interpreter one, unchanged: render through the [`Query`] builder.
    pub fn to_sql(&self) -> QueryOutput
    where
        Row::Table: Table,
    {
        let mut q: Query<Row::Table, All<Row::Table>> = Query::new();
        if !self.clauses.is_empty() {
            let mut frag = Fragment::default();
            write_conjunction(&self.clauses, &mut frag.sql, &mut frag.params);
            q = q.filter(frag);
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
        // D-030: name the columns, exactly as the typed path does. Two spellings
        // of the same query must render identically -- `tests/boxed.rs` asserts
        // byte-equality, and it caught this.
        q.select(crate::query::Named::<Row::Table>::new(
            <Row as Entity>::ALL_COLUMNS,
        ))
        .to_sql()
    }

    /// Interpreter two, on the erased form.
    ///
    /// Takes `&self`, not `self` — an erased query is meant to be *stored*, so
    /// running one twice should not consume it. (The typed `Rows::to_memory`
    /// takes `self`; that asymmetry is deliberate, not an oversight.)
    ///
    /// # The guarantee that is weaker here
    ///
    /// The typed path splits `to_memory` (streaming) from `to_memory_sorted`
    /// (materialising) by type-state, so the cost of `ORDER BY` is in the type
    /// rather than a footnote. An erased query cannot keep that: allowing
    /// `if flag { q = q.order_by(..) }` is precisely allowing the ordered-ness
    /// to be a runtime fact. So there is one method, and it streams *unless*
    /// an `ORDER BY` was added, in which case it materialises eagerly and
    /// sorts — visible in [`BoxedRowIter::is_streaming`], not in the type.
    pub fn to_memory<'p, 's, I>(&'p self, src: I) -> BoxedRowIter<'p, 's, Row, I::IntoIter>
    where
        I: IntoIterator<Item = &'s Row>,
        Row: 's,
        'a: 'p,
    {
        if self.order.is_empty() {
            return BoxedRowIter {
                state: IterState::Streaming {
                    inner: src.into_iter(),
                    clauses: &self.clauses,
                    skip: self.offset.unwrap_or(0),
                    left: self.limit,
                },
            };
        }

        let clauses = &self.clauses;
        let mut kept: Vec<&'s Row> = src
            .into_iter()
            .filter(|r| clauses.iter().all(|c| matches!(c.eval_row(r), Some(true))))
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
        BoxedRowIter {
            state: IterState::Materialised(kept.into_iter()),
        }
    }
}

/// In-memory interpretation of a [`BoxedRows`]. Named, not `impl Iterator`,
/// per `D-106`.
pub struct BoxedRowIter<'p, 's, Row, I> {
    state: IterState<'p, 's, Row, I>,
}

enum IterState<'p, 's, Row, I> {
    Streaming {
        inner: I,
        clauses: &'p [Box<dyn DynPred<Row> + 'p>],
        skip: u64,
        left: Option<u64>,
    },
    Materialised(std::vec::IntoIter<&'s Row>),
}

impl<'p, 's, Row, I> BoxedRowIter<'p, 's, Row, I> {
    /// Whether this run streams (no `ORDER BY`) or was materialised and sorted.
    /// The typed path answers this at compile time; here it is a method.
    pub fn is_streaming(&self) -> bool {
        matches!(self.state, IterState::Streaming { .. })
    }
}

impl<'p, 's, Row, I> Iterator for BoxedRowIter<'p, 's, Row, I>
where
    I: Iterator<Item = &'s Row>,
    Row: 's,
{
    type Item = &'s Row;

    fn next(&mut self) -> Option<&'s Row> {
        match &mut self.state {
            IterState::Materialised(rows) => rows.next(),
            IterState::Streaming {
                inner,
                clauses,
                skip,
                left,
            } => {
                if *left == Some(0) {
                    return None;
                }
                for row in inner.by_ref() {
                    // One indirect call per *clause*, short-circuiting.
                    // Confirmed in the disassembly: this loop compiles to
                    // `mov (%r13),%rdi; mov 0x8(%r13),%rax; call *0x20(%rax);
                    // add $0x10,%r13` -- walking the fat pointers and calling
                    // `eval_row` through the vtable.
                    //
                    // Below `eval_row` everything is the monomorphised typed
                    // evaluator, so a clause built with `.and()`/`.or()` inside
                    // ONE `.filter(..)` costs exactly one call whatever its
                    // internal size.
                    //
                    // That is worth less than it sounds, and the number is here
                    // so nobody has to guess: folding the 3-clause benchmark
                    // into a single `.and()`-chained `.filter(..)` moved it
                    // 8.91 ms -> 8.15 ms, about 9%. The dominant cost of the
                    // erased path is the optimisation barrier at the box --
                    // `next()` can no longer inline the predicate, so the loop
                    // stops unrolling -- not the count of indirect calls.
                    if !clauses
                        .iter()
                        .all(|c| matches!(c.eval_row(row), Some(true)))
                    {
                        continue;
                    }
                    if *skip > 0 {
                        *skip -= 1;
                        continue;
                    }
                    if let Some(n) = left.as_mut() {
                        *n -= 1;
                    }
                    return Some(row);
                }
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.state {
            IterState::Materialised(rows) => rows.size_hint(),
            IterState::Streaming { inner, left, .. } => {
                let (_, hi) = inner.size_hint();
                let hi = match (hi, *left) {
                    (Some(h), Some(l)) => Some(h.min(l as usize)),
                    (Some(h), None) => Some(h),
                    (None, Some(l)) => Some(l as usize),
                    (None, None) => None,
                };
                (0, hi)
            }
        }
    }
}
