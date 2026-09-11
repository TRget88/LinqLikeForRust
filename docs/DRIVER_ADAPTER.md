# Writing a driver adapter

`linq_rs_sql` produces a SQL string plus its bound parameters, and defines the two
traits needed to read a result set back — `ColumnSet` and `RowSource`. It does not
depend on a driver, and **neither it nor `linq_rs` will ever take a third-party
dependency**: `linq_rs` has none, and `linq_rs_sql` may depend only on `linq_rs`.
See `DECISIONS.md` `D-032`.

So the driver glue is yours to write. It is small — the version below is about 36
lines of real code and was verified end-to-end against a real in-memory SQLite
before being moved here from a crate that has since been deleted for taking a
dependency.

`RowSource` has **one required method**. An earlier design had five typed
accessors and measured 101 lines per adapter, because each adapter re-implemented
type checking and worded its own mismatch messages. Name matching, ambiguity
detection, type checking, NULL rules and every error message live in
`linq_rs_sql`, so two adapters cannot disagree about the same failure.

## rusqlite

```rust
use linq_rs_sql::{ColumnSet, RowError, RowSource, SqlValue, SqlValueRef};
use rusqlite::types::ValueRef;

/// A prepared statement's column list. Resolution needs this WITHOUT a row —
/// otherwise an empty result set and a populated one give different verdicts for
/// the same schema.
struct Cols<'s>(&'s rusqlite::Statement<'s>);

impl ColumnSet for Cols<'_> {
    fn column_count(&self) -> usize { self.0.column_count() }
    fn column_name_at(&self, at: usize) -> Option<&str> { self.0.column_name(at).ok() }
}

/// One row, plus its result set's column count.
struct Row<'a, 's>(&'a rusqlite::Row<'s>, usize);

impl ColumnSet for Row<'_, '_> {
    fn column_count(&self) -> usize { self.1 }
    fn column_name_at(&self, at: usize) -> Option<&str> {
        self.0.as_ref().column_name(at).ok()
    }
}

impl<'a, 's> RowSource<'a> for Row<'a, 's> {
    fn value_at(&self, at: usize, column: &'static str)
        -> Result<SqlValueRef<'a>, RowError>
    {
        // `self.0` is `&'a Row`, so what `get_ref` lends is borrowed for `'a`.
        // That one fact is the whole borrowing mechanism: a TEXT column reaches a
        // `&'a str` field with no allocation.
        let row: &'a rusqlite::Row<'s> = self.0;
        match row.get_ref(at).map_err(|e| RowError::driver(column, e))? {
            ValueRef::Null       => Ok(SqlValueRef::Null),
            ValueRef::Integer(v) => Ok(SqlValueRef::Integer(v)),
            ValueRef::Real(v)    => Ok(SqlValueRef::Real(v)),
            ValueRef::Text(b)    => std::str::from_utf8(b)
                                        .map(SqlValueRef::Text)
                                        .map_err(|e| RowError::driver(column, e)),
            ValueRef::Blob(b)    => Ok(SqlValueRef::Blob(b)),
        }
    }
}
```

Binding the parameters, and the fetch loop:

```rust
fn bind(params: &[SqlValue]) -> Vec<Box<dyn rusqlite::ToSql>> {
    params.iter().map(|p| match p {
        SqlValue::Integer(v) => Box::new(*v) as Box<dyn rusqlite::ToSql>,
        SqlValue::Text(v)    => Box::new(v.clone()),
        SqlValue::Boolean(v) => Box::new(*v),
        SqlValue::Float(v)   => Box::new(*v),
        SqlValue::Null       => Box::new(Option::<i64>::None),
    }).collect()
}

fn fetch<E: linq_rs_sql::FromRow>(
    conn: &rusqlite::Connection,
    q: &linq_rs_sql::QueryOutput,
) -> Result<Vec<E>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(&q.sql)?;
    // Resolve ONCE, off the prepared statement, before any row is read.
    let layout = { let cols = Cols(&stmt); E::resolve(&cols)? };
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
```

## Counting without materializing

Wrap the query; do not rewrite it. `SELECT COUNT(*) FROM (<sql>)` keeps every
clause, including `LIMIT`, which a naive `COUNT(*)` rewrite silently drops and
then over-reports.

## What this was verified to do

Against a real in-memory SQLite, over a table whose **physical column order
deliberately differed** from the entity's declaration — so every case also
exercised by-name resolution:

- a filtered, ordered query round-trips, including `Nullable<Text>` as `Option<String>`
- the database and `to_memory` agree on the same query value for `=`, `!=`, `<`,
  `>`, `IS NULL`, `IS NOT NULL`, `NOT`, `AND`, `OR`, `LIMIT`/`OFFSET`, `ORDER BY`
  and three-valued NULL logic — **but not for `LIKE`**, which is provider-defined
  (`D-103`); see the README
- a NULL in a non-nullable column errors, naming the column and the row
- an integer that is not 0 or 1 is refused for a `Boolean` column rather than coerced
- a duplicated column name in a join is `AmbiguousColumn`, not first-wins
- a missing column fails identically on empty and populated result sets
- hostile input reaches the database as a bound parameter, never as SQL
