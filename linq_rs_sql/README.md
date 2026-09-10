# linq_rs_sql

A **typed SQL query builder** for Rust — zero dependencies.

Declare your columns once; from then on the compiler checks them, and every
value you compare against becomes a bound parameter instead of text spliced
into a query.

```rust
use linq_rs_sql::*;

linq_rs_sql::table! {
    employees (id) {
        id     -> Integer,
        name   -> Text,
        dept   -> Text,
        salary -> Integer,
        active -> Boolean,
    }
}

let q = employees::table()
    .filter(employees::salary.gt(100_000))
    .filter(employees::name.like("A%"))
    .filter(employees::active)
    .select((employees::id, employees::name, employees::salary))
    .order_by_desc(employees::salary)
    .limit(5)
    .to_sql();

// q.sql    -> "SELECT id, name, salary FROM employees WHERE (salary > ?) AND
//              (name LIKE ?) AND (active) ORDER BY salary DESC LIMIT ?"
// q.params -> [Integer(100000), Text("A%"), Integer(5)]
```

A misspelt column is a build error. So is comparing a `Text` column to an
integer.

## What it does not do

**It does not execute anything.** `to_sql()` returns a `String` and a
`Vec<SqlValue>`; you hand those to whatever driver you already use — `rusqlite`,
`tokio-postgres`, `sqlx-core`. There is no connection, no pool, no async
runtime, and no dependency of any kind.

That is a deliberate boundary, not an unfinished one. Building SQL and running
it are different jobs with different dependency footprints, and only one of them
needs to be in this crate.

## Scope today

Supported: `SELECT` with column lists or `*`; `WHERE` with `=`, `!=`, `<`, `<=`,
`>`, `>=`, `AND`, `OR`, `NOT`, `LIKE`, `IS NULL`; `ORDER BY` ascending and
descending; `LIMIT` / `OFFSET`; parameter binding throughout.

Not yet: `JOIN`, subqueries, `GROUP BY` / `HAVING`, `INSERT` / `UPDATE` /
`DELETE`, migrations, dialect differences.

## Relationship to `linq_rs`

They are siblings in one repository and **neither depends on the other**.

[`linq_rs`](../README.md) is a LINQ-shaped query surface over in-memory
iterators. This crate builds SQL. The two vocabularies are deliberately separate
— this one says `filter`, that one says `where_` — because a single crate with
two names for one concept is a worse thing to hand a user than two crates with
one each. (`DECISIONS.md` `D-205`, and the split itself is `D-020`.)

Joining them properly means one query value with two interpreters: evaluate it
over a `Vec` in a unit test, render it to SQL in production. That is `D-002`,
and this crate is intended to become its rendering backend rather than a
competitor to it.

## Safety

Values are always bound, never interpolated. Identifiers come from the `table!`
declaration, so a caller-supplied string cannot reach a table or column
position. Verified by test: a `"Robert'); DROP TABLE students;--"` value renders
as `?` with the payload in `params`.

## License

Dual-licensed under [Apache-2.0](../LICENSE-APACHE) or [MIT](../LICENSE-MIT), at
your option.
