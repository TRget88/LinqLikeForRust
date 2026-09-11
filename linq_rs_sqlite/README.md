# linq_rs_sqlite

SQLite provider for [`linq_rs_sql`](https://crates.io/crates/linq_rs_sql). That
crate builds SQL and knows nothing about drivers; this one runs it and hands back
typed rows.

```rust
use linq_rs_sql::prelude::*;
use linq_rs_sqlite::Sqlite;
# linq_rs_sql::table! { employees (id) { id -> Integer, dept -> Text, salary -> Integer } }
# #[derive(Debug)] pub struct Employee { pub id: i64, pub dept: String, pub salary: i64 }
# linq_rs_sql::entity! { Employee => employees { id: Integer = id, dept: Text = dept, salary: Integer = salary } }
# fn run() -> Result<(), Box<dyn std::error::Error>> {
let conn = rusqlite::Connection::open_in_memory()?;
# conn.execute_batch("CREATE TABLE employees(id INTEGER, dept TEXT, salary INTEGER);
#                     INSERT INTO employees VALUES(1,'eng',180000),(2,'sales',90000);")?;
let db = Sqlite::new(&conn);

let staff: Vec<Employee> = db.fetch(
    &query::<Employee>()
        .filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"))
        .order_by_desc(employees::salary)
        .to_sql(),
)?;
# assert_eq!(staff.len(), 1);
# assert_eq!(staff[0].id, 1);
# Ok(())
# }
# run().unwrap();
```

`fetch`, `fetch_one` and `count`. That is the whole surface.

## Why a separate crate

`linq_rs` and `linq_rs_sql` declare **zero dependencies of any kind**, including
dev-dependencies. A `rusqlite` feature on `linq_rs_sql` would end that, and would
put an optional driver in the lockfile of everyone who only wants to build SQL
strings. Splitting the provider out keeps both core crates dependency-free, and
mirrors EF Core — where the provider is a separate package from the core.

## What it does not do

It does not hide SQLite. No connection pool, no transaction wrapper, no
`DbContext`: you own the `rusqlite::Connection` and this borrows it. Anything
rusqlite does better stays rusqlite's job.

## What you get over writing the SQL by hand

All of it enforced at compile time, except the last line:

| | |
|---|---|
| Typo'd column | `error[E0425]: cannot find value 'salry' in module 'employees'` |
| Wrong type | `error[E0271]: ... Text: CompareWith<Integer> is not satisfied` |
| Column from the wrong table | `error[E0277]: the trait bound 'budget: BelongsTo<employees::Marker>'` |
| SQL injection | values are always bound, never interpolated |
| `NULL` semantics | SQL three-valued logic, verified against SQLite cell by cell |
| Result column order | matched by name, so a migration that reorders columns cannot swap fields |
| **Schema drift** | **not protected** — `table!` is a hand-written declaration with no link to the real database |

That last row is the honest gap: every guarantee above is enforced against the
*declaration*. If the declaration and the database disagree, you find out at
runtime.

## Placeholders

`linq_rs_sql` emits `?`, which SQLite and MySQL accept and **PostgreSQL rejects**
(it wants `$1`, `$2`). So there is currently no PostgreSQL provider and this
crate is where that assumption is first written down.

## Licence

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your
option.
