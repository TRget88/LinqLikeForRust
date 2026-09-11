//! SQLite provider for [`linq_rs_sql`].
//!
//! `linq_rs_sql` builds SQL and knows nothing about drivers. This crate is the
//! other half: it runs what that crate produces against a real SQLite and hands
//! back typed rows.
//!
//! ```no_run
//! use linq_rs_sql::prelude::*;
//! use linq_rs_sqlite::Sqlite;
//!
//! # linq_rs_sql::table! { employees (id) { id -> Integer, dept -> Text, salary -> Integer } }
//! # #[derive(Debug)] pub struct Employee { pub id: i64, pub dept: String, pub salary: i64 }
//! # linq_rs_sql::entity! { Employee => employees { id: Integer = id, dept: Text = dept, salary: Integer = salary } }
//! # fn main() -> Result<(), linq_rs_sqlite::Error> {
//! let conn = rusqlite::Connection::open("staff.db")?;
//! let db = Sqlite::new(&conn);
//!
//! let staff: Vec<Employee> = db.fetch(
//!     &query::<Employee>()
//!         .filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"))
//!         .order_by_desc(employees::salary)
//!         .to_sql(),
//! )?;
//! # Ok(()) }
//! ```
//!
//! # Why a separate crate
//!
//! `linq_rs` and `linq_rs_sql` both declare **zero dependencies of any kind**,
//! including dev-dependencies (`D-024`). A `rusqlite` feature on `linq_rs_sql`
//! would end that, and would make every user who only wants to build SQL strings
//! carry an optional driver in their lockfile. Splitting the provider out keeps
//! both published crates dependency-free and mirrors EF Core, where the provider
//! is a separate package from the core.
//!
//! # What this crate does *not* do
//!
//! It does not hide SQLite. There is no connection pool, no transaction wrapper
//! and no `DbContext`: you own the [`rusqlite::Connection`], and this borrows it.
//! Anything rusqlite does better is rusqlite's job.
//!
//! # Placeholders, and the dialect this assumes
//!
//! `linq_rs_sql` emits `?` placeholders, which SQLite and MySQL accept and
//! **PostgreSQL rejects** — it wants `$1`, `$2`. So `linq_rs_sql` is currently a
//! SQLite/MySQL dialect with no way to say so. This crate is the first place that
//! assumption is written down rather than implied. A real dialect layer belongs
//! in `linq_rs_sql`, and building one provider first is how its shape gets
//! discovered rather than guessed — see `DECISIONS.md` `D-031`.

#![deny(missing_docs)]

use linq_rs_sql::{ColumnSet, FromRow, QueryOutput, RowError, RowSource, SqlValue, SqlValueRef};
use rusqlite::types::ValueRef;

/// Anything that can go wrong running a `linq_rs_sql` query here.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// SQLite refused the SQL, or the connection failed.
    Sqlite(rusqlite::Error),
    /// The result set did not match the entity — a missing or ambiguous column,
    /// an unexpected NULL, a type mismatch. See [`RowError`].
    Row(RowError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Sqlite(e) => write!(f, "sqlite: {e}"),
            Error::Row(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Sqlite(e) => Some(e),
            Error::Row(e) => Some(e),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Sqlite(e)
    }
}

impl From<RowError> for Error {
    fn from(e: RowError) -> Self {
        Error::Row(e)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// The adapter — the whole driver seam
// ═══════════════════════════════════════════════════════════════════════════

/// A prepared statement's column list.
struct Cols<'s>(&'s rusqlite::Statement<'s>);

impl ColumnSet for Cols<'_> {
    fn column_count(&self) -> usize {
        self.0.column_count()
    }
    fn column_name_at(&self, at: usize) -> Option<&str> {
        self.0.column_name(at).ok()
    }
}

/// One row, plus its result set's column count.
struct Row<'a, 's>(&'a rusqlite::Row<'s>, usize);

impl ColumnSet for Row<'_, '_> {
    fn column_count(&self) -> usize {
        self.1
    }
    fn column_name_at(&self, at: usize) -> Option<&str> {
        self.0.as_ref().column_name(at).ok()
    }
}

impl<'a, 's> RowSource<'a> for Row<'a, 's> {
    fn value_at(&self, at: usize, column: &'static str) -> Result<SqlValueRef<'a>, RowError> {
        // `self.0` is `&'a Row`, so what `get_ref` lends is borrowed for `'a`.
        // That one fact is the entire borrowing mechanism: a TEXT column reaches
        // a `&'a str` field with no allocation.
        let row: &'a rusqlite::Row<'s> = self.0;
        match row.get_ref(at).map_err(|e| RowError::driver(column, e))? {
            ValueRef::Null => Ok(SqlValueRef::Null),
            ValueRef::Integer(v) => Ok(SqlValueRef::Integer(v)),
            ValueRef::Real(v) => Ok(SqlValueRef::Real(v)),
            ValueRef::Text(b) => std::str::from_utf8(b)
                .map(SqlValueRef::Text)
                .map_err(|e| RowError::driver(column, e)),
            ValueRef::Blob(b) => Ok(SqlValueRef::Blob(b)),
        }
    }
}

/// Bind `linq_rs_sql`'s params positionally.
fn bind(params: &[SqlValue]) -> Vec<Box<dyn rusqlite::ToSql>> {
    params
        .iter()
        .map(|p| match p {
            SqlValue::Integer(v) => Box::new(*v) as Box<dyn rusqlite::ToSql>,
            SqlValue::Text(v) => Box::new(v.clone()),
            SqlValue::Boolean(v) => Box::new(*v),
            SqlValue::Float(v) => Box::new(*v),
            SqlValue::Null => Box::new(Option::<i64>::None),
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// Sqlite — the user-facing handle
// ═══════════════════════════════════════════════════════════════════════════

/// Runs `linq_rs_sql` queries against a borrowed [`rusqlite::Connection`].
///
/// Borrows rather than owns, so transactions, pooling and pragmas stay yours.
pub struct Sqlite<'c> {
    conn: &'c rusqlite::Connection,
}

impl<'c> Sqlite<'c> {
    /// Wrap a connection.
    pub fn new(conn: &'c rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// The connection, for anything this crate does not cover.
    pub fn connection(&self) -> &'c rusqlite::Connection {
        self.conn
    }

    /// Run a rendered query and materialize every row.
    ///
    /// Column resolution happens **once**, off the prepared statement, before
    /// any row is read — so a schema mismatch fails identically whether the
    /// query matched zero rows or a million.
    pub fn fetch<E: FromRow>(&self, q: &QueryOutput) -> Result<Vec<E>, Error> {
        let mut stmt = self.conn.prepare(&q.sql)?;
        let layout = {
            let cols = Cols(&stmt);
            E::resolve(&cols)?
        };
        let n = stmt.column_count();
        let bound = bind(&q.params);
        let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| &**b).collect();
        let mut rows = stmt.query(refs.as_slice())?;
        let mut out = Vec::new();
        let mut i = 0usize;
        while let Some(r) = rows.next()? {
            out.push(E::from_row(&Row(r, n), &layout).map_err(|e| e.at_row(i))?);
            i += 1;
        }
        Ok(out)
    }

    /// Run a rendered query and materialize at most one row.
    ///
    /// `Ok(None)` when the result set is empty. More than one row is **not** an
    /// error here — SQL's own `LIMIT 1` semantics — so pair it with `.limit(1)`
    /// if that matters.
    pub fn fetch_one<E: FromRow>(&self, q: &QueryOutput) -> Result<Option<E>, Error> {
        let mut stmt = self.conn.prepare(&q.sql)?;
        let layout = {
            let cols = Cols(&stmt);
            E::resolve(&cols)?
        };
        let n = stmt.column_count();
        let bound = bind(&q.params);
        let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| &**b).collect();
        let mut rows = stmt.query(refs.as_slice())?;
        match rows.next()? {
            Some(r) => Ok(Some(E::from_row(&Row(r, n), &layout)?)),
            None => Ok(None),
        }
    }

    /// How many rows a rendered query would return, without materializing them.
    ///
    /// Wraps the query rather than rewriting it: `SELECT COUNT(*) FROM (<sql>)`.
    /// That keeps every clause intact, including `LIMIT`, which a naive
    /// `COUNT(*)` rewrite would drop.
    pub fn count(&self, q: &QueryOutput) -> Result<i64, Error> {
        let sql = format!("SELECT COUNT(*) FROM ({})", q.sql);
        let bound = bind(&q.params);
        let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| &**b).collect();
        let n = self
            .conn
            .query_row(&sql, refs.as_slice(), |r| r.get::<_, i64>(0))?;
        Ok(n)
    }
}
