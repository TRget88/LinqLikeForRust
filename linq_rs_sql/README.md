# linq_rs_sql

A typed SQL query builder with an unusual property: **one query value, two
interpreters.** The same value renders to SQL for a database, or evaluates
lazily over a `Vec` in a unit test — and the two agree.

Zero dependencies. No driver, no connection, no runtime — this crate builds SQL
and hands you the string plus its bound parameters. Running it is your driver's
job; [`docs/DRIVER_ADAPTER.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/docs/DRIVER_ADAPTER.md)
has a complete ~36-line rusqlite adapter, verified end-to-end.

```rust
use linq_rs_sql::prelude::*;

table! { employees (id) { id -> Integer, name -> Text, dept -> Text, salary -> Integer } }
pub struct Employee { pub id: i64, pub name: String, pub dept: String, pub salary: i64 }
entity! { Employee => employees { id: Integer = id, name: Text = name, dept: Text = dept, salary: Integer = salary } }
# let staff = vec![
#     Employee { id: 1, name: "Ada".into(), dept: "eng".into(), salary: 180_000 },
#     Employee { id: 2, name: "Bo".into(),  dept: "sales".into(), salary: 90_000 },
# ];

let q = query::<Employee>()
    .filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"))
    .order_by_desc(employees::salary)
    .limit(2);

assert_eq!(
    q.to_sql().sql,
    "SELECT id, name, dept, salary FROM employees \
     WHERE ((salary > ?) AND (dept = ?)) ORDER BY salary DESC LIMIT 2"
);

// The same value, over a &[Employee], lazily.
let names: Vec<&str> = q.to_memory_sorted(&staff).map(|e| e.name.as_str()).collect();
assert_eq!(names, ["Ada"]);
```

## Writing filters

`pred!` takes closure-shaped source and expands to the builder calls below it.
Same types, same SQL, same in-memory evaluation:

```text
.filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"))
.filter(employees::salary.gt(100_000i64).and(employees::dept.eq("eng")))   // identical
```

It exists because a closure *cannot* be translated: `|e| e.salary > 100_000`
compiles to a function, and nothing at runtime can ask it which column, which
operator, which value. C# escapes this with `Expression<Func<T,bool>>`, a
compiler feature Rust does not have. A macro is the substitute, because macros
see syntax before it becomes code.

The grammar is comparisons (`>`, `>=`, `<`, `<=`, `==`, `!=`) joined by `&&` and
`||`, with parentheses. Nothing else parses — and that boundary is the same as
the translation boundary, so anything it rejects could not have become SQL
anyway. Reach for `.to_memory()` and a real closure when you want arbitrary Rust.

A bare integer literal works — `e.salary > 100_000` compiles and emits identical
SQL. Literals default to `i32`, so a value above `i32::MAX` needs an explicit
`i64` suffix, and the error you get if you forget is about `i32`, not about the
column.

## Nullable columns

`Nullable<T>` maps to `Option<T>`, and comparisons follow **SQL's three-valued
logic** rather than Rust's `Option` semantics:

```rust
use linq_rs_sql::prelude::*;
table! { t (id) { id -> Integer, nick -> Nullable<Text>, score -> Nullable<Integer> } }
pub struct T { pub id: i64, pub nick: Option<String>, pub score: Option<i64> }
entity! { T => t { id: Integer = id, nick: Nullable<Text> = nick, score: Nullable<Integer> = score } }
# let rows = vec![
#     T { id: 1, nick: Some("ann".into()), score: Some(10) },
#     T { id: 2, nick: None,               score: None },
# ];

let missing = query::<T>().filter(t::nick.is_null());
assert_eq!(missing.to_sql().sql, "SELECT id, nick, score FROM t WHERE (nick IS NULL)");
assert_eq!(missing.to_memory(&rows).map(|r| r.id).collect::<Vec<_>>(), [2]);

// `NULL > 5` is NULL, not false, so row 2 does not survive.
let high = query::<T>().filter(t::score.gt(5i64));
assert_eq!(high.to_memory(&rows).map(|r| r.id).collect::<Vec<_>>(), [1]);
```

This matters more than it sounds. In SQL `NULL = NULL` is `NULL`, and
`NULL > 5` is `NULL` — neither true nor false. In Rust `None == None` is `true`.
Getting that wrong would make the two interpreters disagree on exactly the rows
where it counts, so nullability is tracked in the *type*: a comparison touching a
nullable column has type `Nullable<Boolean>`, not `Boolean`, and the collapse
from three values to two happens only at `WHERE`.

The two cells that catch a naive implementation: `NULL AND FALSE` is **FALSE**
and `NULL OR TRUE` is **TRUE** — an absorbing operand beats the unknown. Both
verified against real SQLite.

## Building a query conditionally

Every `.filter()` changes the type, so this does not compile on the typed form.
`into_boxed()` / `boxed_query()` give an erased one whose type stays put:

```rust
use linq_rs_sql::prelude::*;
# table! { employees (id) { id -> Integer, dept -> Text, salary -> Integer, active -> Boolean } }
# pub struct Employee { pub id: i64, pub dept: String, pub salary: i64, pub active: bool }
# entity! { Employee => employees { id: Integer = id, dept: Text = dept, salary: Integer = salary, active: Boolean = active } }
# let (min_salary, dept, active_only) = (Some(100_000i64), Some("eng"), true);
let mut q = boxed_query::<Employee>();
if let Some(m) = min_salary { q = q.filter(employees::salary.gt(m)); }
if let Some(d) = dept       { q = q.filter(employees::dept.eq(d)); }
if active_only              { q = q.filter(employees::active.eq(true)); }
# assert!(q.to_sql().sql.contains("AND"));
```

Conditional filters, a query stored in a struct field, a query returned from a
function, a `Vec` of differently-shaped queries. The column type checking
survives erasure completely — it fires at the `.gt()` call, before the box.

## Getting rows back

`entity!` also generates the reverse direction, so a result set becomes typed
structs. **Columns are matched by name, never by position**, because `SELECT *`
expands in table-declaration order — the database's choice, not the query's, and
one that a migration changes under an already-compiled binary.

```text
let layout = Employee::resolve(&statement)?;   // once per statement
let e = Employee::from_row(&row, &layout)?;    // per row
```

A driver adapter implements `ColumnSet` and one method of `RowSource` — about 36
lines. Opt out of generation with `entity! { … } no_from_row` when a struct has a
borrowed field or a field that is not a column.

A complete rusqlite adapter, and the `fetch` loop that uses it, is in
[`docs/DRIVER_ADAPTER.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/docs/DRIVER_ADAPTER.md).

## What you get over writing the SQL by hand

Everything here is a compile error except the last row:

| | |
|---|---|
| Typo'd column | `error[E0425]: cannot find value 'salry' in module 'employees'` |
| Wrong type | `error[E0271]: Text: CompareWith<Integer> is not satisfied` |
| Column from another table | `error[E0277]: the trait bound 'budget: BelongsTo<employees::Marker>'` |
| SQL injection | values are always bound, never interpolated |
| `NULL` semantics | three-valued logic, matching SQL |
| Result column order | matched by name, so a reordering migration cannot swap fields |
| **Schema drift** | **not protected** |

That last row is the honest gap. `table!` is a hand-written declaration with no
link to the real database, so every guarantee above is enforced against the
*declaration*. If it and the database disagree, you find out at runtime.

## Scope today

**Has:** `SELECT`, `WHERE`, `ORDER BY`, `LIMIT`, `OFFSET`, `LIKE`, `IS NULL`,
nullable columns, conditional composition, row materialization.

**Does not have:** `JOIN`, subqueries, `GROUP BY` / `HAVING`, aggregates,
projection to a subset of columns, `INSERT` / `UPDATE` / `DELETE`, migrations,
change tracking.

**Dialect:** `?` placeholders, which SQLite and MySQL accept and **PostgreSQL
rejects** (it wants `$1`, `$2`). Identifiers are emitted unquoted, so a column
named `order` is a parse error. There is no dialect layer yet; this is a
SQLite/MySQL builder.

## Relationship to `linq_rs`

Siblings in one repository, and **neither depends on the other**.
[`linq_rs`](https://github.com/TRget88/LinqLikeForRust#readme) is a LINQ-shaped
surface over in-memory iterators; this crate builds SQL. The vocabularies are
deliberately separate — this one says `filter`, that one says `where_` — because
one crate with two names for one concept is worse to hand a user than two crates
with one each.

`linq_rs` is the closer of the two to C# LINQ, because it takes real closures: it
never has to translate them. This crate must produce text, so its predicates have
to be inspectable. That is the whole reason for the macro and the column
constants.

## Why it is built this way

Every non-obvious decision is recorded, numbered and immutable, in
[`DECISIONS.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/DECISIONS.md) —
including the ones that were wrong first. Relevant here: `D-020` (why a separate
crate), `D-021` (`pred!` is not a DSL), `D-026` (nullable and three-valued
logic), `D-027` (type erasure), `D-028` (cross-table columns), `D-029` (row
materialization), `D-030` (naming the columns), `D-031` (the provider).

## Safety

Values are always bound, never interpolated. Identifiers come from the `table!`
declaration, so a caller-supplied string cannot reach a table or column position.
Verified by test: `"Robert'); DROP TABLE students;--"` renders as `?` with the
payload in `params`, and the table survives.

## License

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your
option.
