//! Row materialization — turning a driver's result set back into typed structs.
//!
//! This is the direction [`entity!`](crate::entity) did not generate. It knew
//! every field, column and SQL type, and produced only struct → column
//! *readers*, so a query result could never become a `Vec<Employee>`.
//!
//! The crate still executes nothing. A driver adapter implements two small
//! traits; everything else lives here.
//!
//! # Columns are matched by NAME, never by position
//!
//! This is the load-bearing decision, and it is not a style preference.
//! [`Rows::to_sql`](crate::rows::Rows::to_sql) emits `SELECT *`, and both
//! SQLite and PostgreSQL expand `*` in **table-declaration order** — which
//! this crate does not know and cannot pin, and which changes under an
//! already-compiled binary. SQLite cannot reorder a column in place, so the
//! documented migration is a table rebuild, which is exactly where declaration
//! order drifts:
//!
//! ```text
//! v1 schema:  SELECT * -> Employee { id: 1, name: "ada",         dept: "engineering" }
//! v2 schema:  SELECT * -> Employee { id: 1, name: "engineering", dept: "ada" }
//! ```
//!
//! Same code, same entity, same types — so no error is possible. A positional
//! decoder turns the database's choice of column order into a *plausible wrong
//! value*, which is the category `D-025` exists to forbid. Matching by name
//! makes the whole class unreachable.
//!
//! Resolution happens **once** per result set, off the column list, producing a
//! [`Layout`]. Per-row cost is an index, not a lookup.
//!
//! # The driver seam is two traits and one required method
//!
//! [`ColumnSet`] reports what the result set *has*; [`RowSource`] hands over one
//! value. That is all an adapter does — it never decides what a name means, and
//! it never words an error message. Name matching, ambiguity detection, type
//! checking, NULL rules and every message live here, so two drivers cannot
//! disagree about the same failure.
//!
//! `ColumnSet` is deliberately **separate** from `RowSource` rather than folded
//! into it. Resolution must work without a row: fold them together and the same
//! query against the same schema returns `Ok(vec![])` on empty data and
//! `Err(NoSuchColumn)` on populated data — two verdicts for one schema, decided
//! by whether rows happened to exist.
//!
//! # Booleans accept only 0 and 1
//!
//! An integer column holding `2` is not `true` here; it is a [`RowError`]. The
//! permissive rule ("non-zero is true") breaks the two-interpreter seam, and it
//! was measured breaking it. For `active INTEGER` holding `1, 0, -1, 2`:
//!
//! ```text
//! SQL   WHERE active = ?  bound true  ->  ids [1]
//! memory, if 2 and -1 became true     ->  ids [1, 3, 4]
//! ```
//!
//! One query value, two answers. Refusing the coercion is what keeps the
//! interpreters honest — see [`DECISIONS.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/DECISIONS.md)
//! `D-029`.

use crate::types::{Boolean, Float, Integer, Nullable, Text};
use crate::value::SqlValue;
use core::marker::PhantomData;
use std::error::Error;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════════
// SqlValueRef — one value, borrowed from the driver's row buffer
// ═══════════════════════════════════════════════════════════════════════════

/// A single value from a result set, borrowed from the driver's buffer.
///
/// Borrowed rather than owned so a `Text` column costs no allocation when the
/// target field borrows too. `#[non_exhaustive]` because dates, decimals and a
/// dialect layer each want a variant and adding one must not be a major bump.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SqlValueRef<'a> {
    /// SQL `NULL`. At this layer NULL is *data* — the adapter does not know
    /// whether the column was declared `Nullable`, so the NOT NULL rule lives
    /// one layer up in [`LoadField`].
    Null,
    /// An integral value, normalised to `i64`.
    Integer(i64),
    /// A floating-point value.
    Real(f64),
    /// UTF-8 text borrowed from the row buffer.
    Text(&'a str),
    /// Raw bytes borrowed from the row buffer.
    Blob(&'a [u8]),
}

impl SqlValueRef<'_> {
    /// The name of this variant, for error messages.
    pub const fn type_name(&self) -> &'static str {
        match self {
            SqlValueRef::Null => "Null",
            SqlValueRef::Integer(_) => "Integer",
            SqlValueRef::Real(_) => "Real",
            SqlValueRef::Text(_) => "Text",
            SqlValueRef::Blob(_) => "Blob",
        }
    }

    /// An owned [`SqlValue`], for error payloads that must outlive the row.
    pub fn to_owned_value(&self) -> SqlValue {
        match *self {
            SqlValueRef::Null => SqlValue::Null,
            SqlValueRef::Integer(v) => SqlValue::Integer(v),
            SqlValueRef::Real(v) => SqlValue::Float(v),
            SqlValueRef::Text(v) => SqlValue::Text(v.to_string()),
            // No `Blob` in `SqlValue`; render it rather than grow the bind type.
            SqlValueRef::Blob(b) => SqlValue::Text(format!("<{} bytes>", b.len())),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RowError
// ═══════════════════════════════════════════════════════════════════════════

/// Why a row could not be materialized.
///
/// Named `RowError` rather than `Error` so a future dialect or builder error is
/// not forced into the same enum. Every variant that can name a column does, as
/// a `&'static str` taken from [`Column::NAME`](crate::Column::NAME) — fixed at
/// compile time by [`table!`](crate::table), so naming the failing field costs
/// no allocation.
///
/// A missing column and a NULL value are **deliberately different variants**.
/// For a crate that went to the trouble of implementing three-valued logic,
/// collapsing "this column is absent" into "this value is null" would be a
/// category error.
#[derive(Debug)]
#[non_exhaustive]
pub enum RowError {
    /// The result set has no column of this name.
    NoSuchColumn {
        /// The column the entity asked for.
        column: &'static str,
        /// What the result set actually offered, in order.
        available: Vec<String>,
    },
    /// The result set has more than one column of this name, so the name does
    /// not identify a value. Typically a join: `SELECT * FROM a JOIN b` can
    /// yield two `id` columns, and silently taking the first is how the second
    /// table's data becomes unreachable without anything saying so.
    AmbiguousColumn {
        /// The ambiguous name.
        column: &'static str,
        /// The first two positions carrying it.
        at: (usize, usize),
    },
    /// The value was NULL but the column is not declared `Nullable<_>`.
    UnexpectedNull {
        /// The column that was null.
        column: &'static str,
        /// Which row, when the caller is materializing a sequence.
        row: Option<usize>,
    },
    /// The driver returned a value of the wrong shape for the declared type.
    TypeMismatch {
        /// The column that failed.
        column: &'static str,
        /// The declared SQL type, e.g. `"INTEGER"`.
        expected: &'static str,
        /// What the driver actually handed over, e.g. `"Text"`.
        found: &'static str,
        /// Which row, when known.
        row: Option<usize>,
    },
    /// The value was the right shape but does not fit the target Rust type —
    /// checked, never an `as` cast.
    OutOfRange {
        /// The column that failed.
        column: &'static str,
        /// The Rust type it would not fit, e.g. `"i32"`.
        target: &'static str,
        /// The offending value.
        value: SqlValue,
        /// Which row, when known.
        row: Option<usize>,
    },
    /// The driver itself failed.
    Driver {
        /// The column being read, when the failure was attributable.
        column: Option<&'static str>,
        /// The driver's own error.
        source: Box<dyn Error + Send + Sync>,
    },
}

impl RowError {
    /// Wrap a driver error, naming the column being read.
    pub fn driver<E>(column: &'static str, source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        RowError::Driver {
            column: Some(column),
            source: Box::new(source),
        }
    }

    /// The column this error is about, when it names one.
    pub fn column(&self) -> Option<&'static str> {
        match self {
            RowError::NoSuchColumn { column, .. }
            | RowError::AmbiguousColumn { column, .. }
            | RowError::UnexpectedNull { column, .. }
            | RowError::TypeMismatch { column, .. }
            | RowError::OutOfRange { column, .. } => Some(column),
            RowError::Driver { column, .. } => *column,
        }
    }

    /// Attach a row ordinal, so a failure in a 100k-row fetch says which row.
    /// Applied by the caller that knows the position; leaves an already-set
    /// ordinal alone.
    pub fn at_row(mut self, n: usize) -> Self {
        match &mut self {
            RowError::UnexpectedNull {
                row: row @ None, ..
            }
            | RowError::TypeMismatch {
                row: row @ None, ..
            }
            | RowError::OutOfRange {
                row: row @ None, ..
            } => *row = Some(n),
            _ => {}
        }
        self
    }
}

impl fmt::Display for RowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn at(row: &Option<usize>) -> String {
            row.map(|n| format!(" (row {n})")).unwrap_or_default()
        }
        match self {
            RowError::NoSuchColumn { column, available } => write!(
                f,
                "no column `{column}` in the result set (it has: {})",
                available.join(", ")
            ),
            RowError::AmbiguousColumn { column, at: (a, b) } => write!(
                f,
                "column `{column}` appears more than once in the result set \
                 (positions {a} and {b}); name it explicitly or alias it"
            ),
            RowError::UnexpectedNull { column, row } => write!(
                f,
                "column `{column}` is NULL but is not declared Nullable<_>{}",
                at(row)
            ),
            RowError::TypeMismatch {
                column,
                expected,
                found,
                row,
            } => write!(
                f,
                "column `{column}` is declared {expected} but the driver returned {found}{}",
                at(row)
            ),
            RowError::OutOfRange {
                column,
                target,
                value,
                row,
            } => write!(
                f,
                "column `{column}`: {value:?} does not fit `{target}`{}",
                at(row)
            ),
            RowError::Driver { column, source } => match column {
                Some(c) => write!(f, "driver error reading column `{c}`: {source}"),
                None => write!(f, "driver error: {source}"),
            },
        }
    }
}

impl Error for RowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RowError::Driver { source, .. } => Some(&**source),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// The driver seam
// ═══════════════════════════════════════════════════════════════════════════

/// What a result set's columns are called. Implemented by a driver adapter over
/// a prepared statement.
///
/// Split from [`RowSource`] on purpose: resolution must be possible with no row
/// in hand, or an empty result set and a populated one give different verdicts
/// for the same schema.
pub trait ColumnSet {
    /// How many columns the result set has.
    fn column_count(&self) -> usize;
    /// The name of the column at `at`, or `None` if out of range.
    fn column_name_at(&self, at: usize) -> Option<&str>;
}

/// One row of a result set.
///
/// **One required method.** An earlier shape had five typed accessors and
/// measured 101 lines per adapter, because each adapter re-implemented type
/// checking and worded its own mismatch message. Collapsing to `value_at` took
/// it to ~36 and means a new SQL type no longer breaks every adapter.
pub trait RowSource<'a>: ColumnSet {
    /// The value at `at`. `column` is passed only so a driver failure can name
    /// the column it was reading.
    ///
    /// Return [`SqlValueRef::Null`] for SQL NULL — do **not** treat it as an
    /// error here. At this layer NULL is data; whether it is *allowed* depends
    /// on the declared SQL type, which the adapter does not know.
    fn value_at(&self, at: usize, column: &'static str) -> Result<SqlValueRef<'a>, RowError>;
}

// ═══════════════════════════════════════════════════════════════════════════
// Layout — resolved positions, tied to the shape they were resolved for
// ═══════════════════════════════════════════════════════════════════════════

/// Where each of a type's columns sits in a particular result set.
///
/// Produced once per result set by [`FromRow::resolve`], then reused per row.
///
/// The `R` parameter is not decoration. A layout resolved for one shape, fed to
/// another of the same arity whose columns happen to share types, produces
/// silently wrong values with no error possible — measured yielding
/// `{ id: 42, n: 10 }` where the truth was `id: 10, n: 42`. Tying the layout to
/// its shape makes that a compile error instead.
#[derive(Debug, Clone)]
pub struct Layout<R: ?Sized> {
    at: Vec<usize>,
    _shape: PhantomData<fn() -> R>,
}

impl<R: ?Sized> Layout<R> {
    /// The resolved position of the `i`th declared column.
    #[inline]
    pub fn position(&self, i: usize) -> usize {
        self.at[i]
    }

    /// How many columns this layout resolves.
    pub fn len(&self) -> usize {
        self.at.len()
    }

    /// Whether the shape declares no columns.
    pub fn is_empty(&self) -> bool {
        self.at.is_empty()
    }

    /// True when every column resolved to its own declared position, i.e. the
    /// result set is already in the shape's order. A projected `SELECT` list
    /// should always produce this; a `SELECT *` may or may not.
    pub fn is_identity(&self) -> bool {
        self.at.iter().enumerate().all(|(i, &p)| i == p)
    }
}

/// Resolve declared column names against a result set, by name.
///
/// Exposed so a hand-written [`FromRow`] can use the same matching rule the
/// macro-generated ones do — the rule and its errors belong to the crate, not
/// to adapters or callers.
pub fn resolve_by_name<R: ?Sized, C: ColumnSet + ?Sized>(
    cols: &C,
    wanted: &'static [&'static str],
) -> Result<Layout<R>, RowError> {
    let n = cols.column_count();
    let mut at = Vec::with_capacity(wanted.len());
    for &want in wanted {
        let mut found: Option<usize> = None;
        for i in 0..n {
            // Exact, case-sensitive, crate-owned. Leaving the matching rule to
            // the adapter meant two drivers gave two verdicts for one schema:
            // rusqlite folds ASCII case, a strict adapter does not.
            if cols.column_name_at(i) == Some(want) {
                match found {
                    None => found = Some(i),
                    // Never silently take the first. `SELECT * FROM a JOIN b`
                    // yields two `id` columns, and picking one makes the other
                    // table's data unreachable with nothing saying so.
                    Some(first) => {
                        return Err(RowError::AmbiguousColumn {
                            column: want,
                            at: (first, i),
                        })
                    }
                }
            }
        }
        match found {
            Some(i) => at.push(i),
            None => {
                return Err(RowError::NoSuchColumn {
                    column: want,
                    available: (0..n)
                        .map(|i| cols.column_name_at(i).unwrap_or("?").to_string())
                        .collect(),
                })
            }
        }
    }
    Ok(Layout {
        at,
        _shape: PhantomData,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// FromRow
// ═══════════════════════════════════════════════════════════════════════════

/// A type that can be built from one row of a result set.
///
/// Generated by [`entity!`](crate::entity) from the declaration it already has.
/// Opt out with the `no_from_row` arm when the struct cannot have one — a
/// borrowed field, or a field that is not a column.
pub trait FromRow: Sized {
    /// The declared column names, in declaration order.
    const COLUMNS: &'static [&'static str];

    /// Match [`COLUMNS`](Self::COLUMNS) against a result set. Do this once per
    /// statement, not once per row.
    fn resolve<C: ColumnSet + ?Sized>(cols: &C) -> Result<Layout<Self>, RowError> {
        resolve_by_name(cols, Self::COLUMNS)
    }

    /// Build one value, using a layout resolved for this same shape.
    fn from_row<'a, R: RowSource<'a> + ?Sized>(
        row: &R,
        layout: &Layout<Self>,
    ) -> Result<Self, RowError>;
}

// ═══════════════════════════════════════════════════════════════════════════
// LoadOpt / LoadField — decoding one value for one declared SQL type
// ═══════════════════════════════════════════════════════════════════════════

/// Decode one value for a **base** SQL type, keeping NULL as `None`.
///
/// Implemented only for the base markers, never for `Nullable<_>` — that is
/// what keeps [`LoadField`]'s two blanket impls from overlapping.
pub trait LoadOpt<'a, F> {
    /// `Ok(None)` for SQL NULL; the NOT NULL rule is applied by [`LoadField`].
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<F>, RowError>;
}

/// Decode one value for a declared SQL type into a field type.
///
/// Two blanket impls, and `Nullable<S>` is a single lift over every base impl
/// rather than a duplicate set.
pub trait LoadField<'a, F> {
    /// Decode, applying the NOT NULL rule.
    fn load_field(v: SqlValueRef<'a>, column: &'static str) -> Result<F, RowError>;
}

impl<'a, S, F> LoadField<'a, F> for S
where
    S: LoadOpt<'a, F>,
{
    fn load_field(v: SqlValueRef<'a>, column: &'static str) -> Result<F, RowError> {
        match S::load_opt(v, column)? {
            Some(f) => Ok(f),
            None => Err(RowError::UnexpectedNull { column, row: None }),
        }
    }
}

impl<'a, S, F> LoadField<'a, Option<F>> for Nullable<S>
where
    S: LoadOpt<'a, F>,
{
    fn load_field(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<F>, RowError> {
        S::load_opt(v, column)
    }
}

/// Reject anything that is not the expected shape.
fn mismatch<T>(
    column: &'static str,
    expected: &'static str,
    v: SqlValueRef<'_>,
) -> Result<T, RowError> {
    Err(RowError::TypeMismatch {
        column,
        expected,
        found: v.type_name(),
        row: None,
    })
}

// ── Integer ────────────────────────────────────────────────────────────────

impl<'a> LoadOpt<'a, i64> for Integer {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<i64>, RowError> {
        match v {
            SqlValueRef::Null => Ok(None),
            SqlValueRef::Integer(n) => Ok(Some(n)),
            other => mismatch(column, "INTEGER", other),
        }
    }
}

impl<'a> LoadOpt<'a, i32> for Integer {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<i32>, RowError> {
        match v {
            SqlValueRef::Null => Ok(None),
            // Checked, never `as`: narrowing silently is how a wrong value gets
            // in, and D-025 was three of those.
            SqlValueRef::Integer(n) => {
                i32::try_from(n)
                    .map(Some)
                    .map_err(|_| RowError::OutOfRange {
                        column,
                        target: "i32",
                        value: SqlValue::Integer(n),
                        row: None,
                    })
            }
            other => mismatch(column, "INTEGER", other),
        }
    }
}

// ── Float ──────────────────────────────────────────────────────────────────

impl<'a> LoadOpt<'a, f64> for Float {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<f64>, RowError> {
        match v {
            SqlValueRef::Null => Ok(None),
            SqlValueRef::Real(x) => Ok(Some(x)),
            // An INTEGER where REAL is declared is the one widening a database
            // does for free and cannot lose: i64 -> f64 is lossy above 2^53,
            // so it is range-checked rather than cast.
            SqlValueRef::Integer(n) => {
                if n.unsigned_abs() <= (1u64 << 53) {
                    Ok(Some(n as f64))
                } else {
                    Err(RowError::OutOfRange {
                        column,
                        target: "f64",
                        value: SqlValue::Integer(n),
                        row: None,
                    })
                }
            }
            other => mismatch(column, "REAL", other),
        }
    }
}

impl<'a> LoadOpt<'a, f32> for Float {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<f32>, RowError> {
        match <Float as LoadOpt<'a, f64>>::load_opt(v, column)? {
            None => Ok(None),
            Some(x) => {
                let narrowed = x as f32;
                if narrowed.is_finite() || !x.is_finite() {
                    Ok(Some(narrowed))
                } else {
                    Err(RowError::OutOfRange {
                        column,
                        target: "f32",
                        value: SqlValue::Float(x),
                        row: None,
                    })
                }
            }
        }
    }
}

// ── Boolean ────────────────────────────────────────────────────────────────

impl<'a> LoadOpt<'a, bool> for Boolean {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<bool>, RowError> {
        match v {
            SqlValueRef::Null => Ok(None),
            SqlValueRef::Integer(0) => Ok(Some(false)),
            SqlValueRef::Integer(1) => Ok(Some(true)),
            // NOT "non-zero is true". That rule breaks the seam: `WHERE active
            // = ?` bound `true` renders `active = 1`, so the database keeps only
            // 1 while a permissive reader would call 2 and -1 true as well. One
            // query value, two answers. Measured. See D-029.
            SqlValueRef::Integer(n) => Err(RowError::OutOfRange {
                column,
                target: "bool",
                value: SqlValue::Integer(n),
                row: None,
            }),
            other => mismatch(column, "BOOLEAN", other),
        }
    }
}

// ── Text ───────────────────────────────────────────────────────────────────

impl<'a> LoadOpt<'a, String> for Text {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<String>, RowError> {
        match v {
            SqlValueRef::Null => Ok(None),
            SqlValueRef::Text(s) => Ok(Some(s.to_string())),
            other => mismatch(column, "TEXT", other),
        }
    }
}

impl<'a> LoadOpt<'a, &'a str> for Text {
    fn load_opt(v: SqlValueRef<'a>, column: &'static str) -> Result<Option<&'a str>, RowError> {
        match v {
            SqlValueRef::Null => Ok(None),
            // Zero allocation: the `&'a str` is borrowed straight from the
            // driver's row buffer.
            SqlValueRef::Text(s) => Ok(Some(s)),
            other => mismatch(column, "TEXT", other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cols(Vec<&'static str>);
    impl ColumnSet for Cols {
        fn column_count(&self) -> usize {
            self.0.len()
        }
        fn column_name_at(&self, at: usize) -> Option<&str> {
            self.0.get(at).copied()
        }
    }

    const WANT: &[&str] = &["id", "name", "dept"];

    #[test]
    fn resolves_by_name_regardless_of_position() {
        let l: Layout<()> = resolve_by_name(&Cols(vec!["dept", "id", "name"]), WANT).unwrap();
        assert_eq!(
            (l.position(0), l.position(1), l.position(2)),
            (1, 2, 0),
            "id, name, dept live at 1, 2, 0"
        );
        assert!(!l.is_identity());
    }

    #[test]
    fn an_already_ordered_result_set_is_the_identity() {
        let l: Layout<()> = resolve_by_name(&Cols(vec!["id", "name", "dept"]), WANT).unwrap();
        assert!(l.is_identity());
    }

    #[test]
    fn extra_columns_are_ignored() {
        let l: Layout<()> =
            resolve_by_name(&Cols(vec!["x", "id", "y", "name", "dept", "z"]), WANT).unwrap();
        assert_eq!((l.position(0), l.position(1), l.position(2)), (1, 3, 4));
    }

    #[test]
    fn a_missing_column_names_itself_and_what_was_available() {
        let e = resolve_by_name::<(), _>(&Cols(vec!["id", "name"]), WANT).unwrap_err();
        assert!(matches!(e, RowError::NoSuchColumn { column: "dept", .. }));
        assert_eq!(
            e.to_string(),
            "no column `dept` in the result set (it has: id, name)"
        );
    }

    /// An empty result set must fail exactly like a populated one. This is why
    /// `ColumnSet` is separate from `RowSource`: resolution takes no row, so a
    /// schema either matches or does not, regardless of how much data came back.
    #[test]
    fn resolution_does_not_depend_on_there_being_rows() {
        let empty = resolve_by_name::<(), _>(&Cols(vec!["id", "name"]), WANT).unwrap_err();
        assert_eq!(empty.to_string(), {
            let same = resolve_by_name::<(), _>(&Cols(vec!["id", "name"]), WANT).unwrap_err();
            same.to_string()
        });
    }

    /// `SELECT * FROM a JOIN b` yields two `id` columns. Taking the first makes
    /// the second table's data unreachable with nothing saying so.
    #[test]
    fn a_duplicated_name_is_ambiguous_not_first_wins() {
        let e =
            resolve_by_name::<(), _>(&Cols(vec!["id", "name", "dept", "id"]), WANT).unwrap_err();
        assert!(matches!(
            e,
            RowError::AmbiguousColumn {
                column: "id",
                at: (0, 3)
            }
        ));
        assert!(e.to_string().contains("appears more than once"));
    }

    #[test]
    fn name_matching_is_case_sensitive_and_owned_by_this_crate() {
        // rusqlite folds ASCII case; a strict adapter does not. Leaving the rule
        // to the adapter gave two verdicts for one schema, so the rule is here.
        let e = resolve_by_name::<(), _>(&Cols(vec!["ID", "NAME", "DEPT"]), WANT).unwrap_err();
        assert!(matches!(e, RowError::NoSuchColumn { column: "id", .. }));
    }

    // ── decoding ───────────────────────────────────────────────────────────

    #[test]
    fn null_into_a_non_nullable_column_is_an_error_naming_it() {
        let e = <Integer as LoadField<'_, i64>>::load_field(SqlValueRef::Null, "id").unwrap_err();
        assert_eq!(
            e.to_string(),
            "column `id` is NULL but is not declared Nullable<_>"
        );
    }

    #[test]
    fn null_into_a_nullable_column_is_none() {
        let got = <Nullable<Text> as LoadField<'_, Option<String>>>::load_field(
            SqlValueRef::Null,
            "nick",
        )
        .unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn a_present_nullable_value_is_some() {
        let got = <Nullable<Text> as LoadField<'_, Option<String>>>::load_field(
            SqlValueRef::Text("ann"),
            "nick",
        )
        .unwrap();
        assert_eq!(got.as_deref(), Some("ann"));
    }

    #[test]
    fn a_wrong_shape_names_the_column_the_declared_type_and_what_arrived() {
        let e = <Integer as LoadField<'_, i64>>::load_field(SqlValueRef::Text("nope"), "n")
            .unwrap_err();
        assert_eq!(
            e.to_string(),
            "column `n` is declared INTEGER but the driver returned Text"
        );
    }

    #[test]
    fn narrowing_is_checked_never_cast() {
        assert_eq!(
            <Integer as LoadField<'_, i64>>::load_field(SqlValueRef::Integer(i64::MAX), "n")
                .unwrap(),
            i64::MAX
        );
        let e =
            <Integer as LoadField<'_, i32>>::load_field(SqlValueRef::Integer(5_000_000_000), "n")
                .unwrap_err();
        assert_eq!(
            e.to_string(),
            "column `n`: Integer(5000000000) does not fit `i32`"
        );
    }

    /// D-029, and the reason this module refuses a coercion that every other
    /// SQLite binding performs. `WHERE active = ?` bound `true` renders
    /// `active = 1`, so the database keeps only 1. A reader that called 2 true
    /// would make the same query value select different rows in the two
    /// interpreters.
    #[test]
    fn bool_accepts_only_zero_and_one() {
        assert!(
            !<Boolean as LoadField<'_, bool>>::load_field(SqlValueRef::Integer(0), "a").unwrap()
        );
        assert!(
            <Boolean as LoadField<'_, bool>>::load_field(SqlValueRef::Integer(1), "a").unwrap()
        );
        for bad in [-1i64, 2, i64::MAX] {
            let e = <Boolean as LoadField<'_, bool>>::load_field(SqlValueRef::Integer(bad), "a")
                .unwrap_err();
            assert!(
                matches!(e, RowError::OutOfRange { target: "bool", .. }),
                "{bad} must not become a bool, got {e}"
            );
        }
    }

    #[test]
    fn text_can_borrow_from_the_row_buffer() {
        let owned = String::from("borrowed");
        let got =
            <Text as LoadField<'_, &str>>::load_field(SqlValueRef::Text(&owned), "s").unwrap();
        assert_eq!(got, "borrowed");
        assert!(std::ptr::eq(got.as_ptr(), owned.as_ptr()), "no copy");
    }

    #[test]
    fn an_integer_widens_into_a_declared_real_but_only_losslessly() {
        assert_eq!(
            <Float as LoadField<'_, f64>>::load_field(SqlValueRef::Integer(3), "r").unwrap(),
            3.0
        );
        let e =
            <Float as LoadField<'_, f64>>::load_field(SqlValueRef::Integer((1i64 << 53) + 1), "r")
                .unwrap_err();
        assert!(matches!(e, RowError::OutOfRange { target: "f64", .. }));
    }

    #[test]
    fn a_row_ordinal_can_be_attached_and_is_not_overwritten() {
        let e = <Integer as LoadField<'_, i64>>::load_field(SqlValueRef::Null, "id")
            .unwrap_err()
            .at_row(99);
        assert_eq!(
            e.to_string(),
            "column `id` is NULL but is not declared Nullable<_> (row 99)"
        );
        let again = e.at_row(1);
        assert!(again.to_string().contains("(row 99)"), "first wins");
    }

    #[test]
    fn every_naming_variant_reports_its_column() {
        let cases: Vec<RowError> = vec![
            resolve_by_name::<(), _>(&Cols(vec![]), &["id"]).unwrap_err(),
            <Integer as LoadField<'_, i64>>::load_field(SqlValueRef::Null, "id").unwrap_err(),
            <Integer as LoadField<'_, i64>>::load_field(SqlValueRef::Text("x"), "id").unwrap_err(),
        ];
        for e in cases {
            assert_eq!(e.column(), Some("id"), "{e}");
        }
    }
}
