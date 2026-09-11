# Changelog

All notable changes to `linq_rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — `linq_rs_sqlite 0.1.0`, the SQLite provider (D-031)

Queries now execute. A third crate, so the other two stay dependency-free.

```rust
use linq_rs_sqlite::Sqlite;

let db = Sqlite::new(&conn);
let staff: Vec<Employee> = db.fetch(
    &query::<Employee>()
        .filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"))
        .order_by_desc(employees::salary)
        .to_sql(),
)?;
```

`fetch`, `fetch_one`, `count`. That is the whole surface.

- **Not a feature flag on `linq_rs_sql`.** An optional dependency is still a
  dependency — it ends the zero-dependency claim and lands in the lockfile of
  everyone who only wanted SQL strings. Splitting is what lets D-024 stay true.
  It is also EF Core's own architecture: core plus a provider package per
  database.
- It does **not** hide SQLite — no pool, no transaction wrapper, no `DbContext`.
  You own the `rusqlite::Connection`; `Sqlite<'c>` borrows it.
- `count` wraps rather than rewrites (`SELECT COUNT(*) FROM (<sql>)`), so a
  `LIMIT` is not silently dropped.
- 12 tests against a real in-memory SQLite, including **the database and the
  in-memory interpreter agreeing on the same query value**.
- **First place the dialect assumption is written down:** `linq_rs_sql` emits `?`
  placeholders, which PostgreSQL rejects (it wants `$1`). There is consequently no
  PostgreSQL provider, and a dialect layer is still open work.

### Changed — `to_sql()` names the columns instead of emitting `*` (D-030)

`linq_rs_sql 0.3.0`. **The emitted SQL changes**, so a snapshot test on it will
move:

```diff
- SELECT * FROM employees WHERE (salary > ?) ORDER BY salary DESC
+ SELECT id, name, dept, salary, nick, active FROM employees WHERE (salary > ?) ORDER BY salary DESC
```

D-029 made `SELECT *` *safe* by reading columns by name. This makes it
unnecessary, and is **the precondition for projection** — a query cannot select
a subset of columns while its SELECT list is a wildcard, so `.select()` was
unreachable without it.

It is also defence in depth: column order moves from the database's control to
the query's, and resolution then provably returns the identity permutation.

- `Entity` gains `ALL_COLUMNS`, supplied by `entity!` on both arms. **Required
  with no default** — a default of `&[]` would silently fall back to `SELECT *`
  for an entity that forgot it. Breaking for hand-written `Entity` impls.
- Named `ALL_COLUMNS` rather than `COLUMNS` because `FromRow::COLUMNS` already
  exists and `Emp::COLUMNS` was `E0034`. That is the same class of defect that
  disqualified `linq_rs 0.1.0`.
- **The lower-level `Query` builder still emits `*`**, deliberately: it has no
  `Entity`, so there is no declared list to name. `employees::table()` gives
  `SELECT *`; `query::<Employee>()` gives the named list.
- Identifiers are still emitted **unquoted**. A column named `order` was already
  broken before this — the crate emits `WHERE (order > ?)`, which real SQLite
  rejects. Quoting is a dialect question wanting one ruling across every
  emission site, not a special case in the SELECT list.

### Added — row materialization: `FromRow`, `RowError`, and a driver seam (D-029)

A query result can now become typed structs. `entity!` generates the reverse
direction from the declaration it already had.

```rust
let stmt = conn.prepare(&q.to_sql().sql)?;
let layout = Emp::resolve(&Stmt(&stmt))?;      // once per statement
let emp = Emp::from_row(&Row(row, n), &layout)?;
```

The crate still executes nothing. A driver adapter implements `ColumnSet` and
one method of `RowSource` — measured at 36 lines for rusqlite.

- **Columns are matched by NAME, never by position.** `SELECT *` expands in
  table-declaration order, which this crate cannot pin and which a migration
  changes under an already-compiled binary. A positional decoder turns that into
  a silent wrong value. Verified: a table physically ordered
  `dept, active, id, nick, salary, name` resolves to `[2,5,0,4,3,1]` and
  materializes correctly.
- **Booleans accept only 0 and 1.** Every other SQLite binding treats non-zero
  as true; doing so breaks the seam. For `active INTEGER` holding `1, 0, -1, 2`,
  SQL `WHERE active = ?` bound `true` keeps `[1]` while a permissive reader keeps
  `[1, 3, 4]`. Narrowing is likewise checked, never an `as` cast.
- **`ColumnSet` is separate from `RowSource`** so resolution needs no row —
  otherwise an empty result set and a populated one give different verdicts for
  the same schema.
- **A duplicated column name is an error, not first-wins.** `SELECT * FROM a
  JOIN b` yields two `id` columns, and silently taking one makes the other
  table's data unreachable.
- **`Layout<R>` is tied to its shape**, so a layout resolved for one entity
  cannot be used with another of the same arity.
- `RowError` is `#[non_exhaustive]`, carries the column name as `&'static str`
  (no allocation), and can be given a row ordinal with `.at_row(n)`. A missing
  column and a NULL value are deliberately different variants.
- `Nullable<S>` needs no duplicate decoding impls — `LoadField` lifts `LoadOpt`
  once.
- Opt out with `entity! { … } no_from_row` when the struct has a borrowed field
  or a field that is not a column.

### Fixed — a query could be filtered by another table's column (D-028)

```rust
query::<Employee>().filter(departments::budget.gt(100i64)).to_sql()
// SELECT * FROM employees WHERE (budget > ?)
```

That SQL fails at runtime with *no such column* — or worse, silently matches a
same-named column meaning something else. It was present in **both** builders.

The in-memory path always rejected it, because `Eval<'_, Row>` is only
implemented for the row's own columns. `to_sql()` did not, so the production
path had the weaker guarantee.

Now a marker trait `BelongsTo<T>` is required by `Query::filter`,
`Rows::filter`, `Rows::to_sql` and `BoxedRows::filter`. Literals belong to every
table (they have no columns), so `salary.gt(100)` is unaffected; combinators
belong to `T` when every operand does. The error names both tables:

```
error[E0277]: the trait bound `budget: BelongsTo<employees::Marker>` is not satisfied
help: the trait `BelongsTo<employees::Marker>` is not implemented for `budget`
      but trait `BelongsTo<departments::Marker>` is implemented for it
```

Column impls are generated by `table!` rather than by a blanket impl over
`Column`, which would overlap the literal impls — Rust cannot prove
`i64: !Column`.

### Added — `into_boxed()`, for queries the type system cannot follow (D-027)

Every `.filter()` used to return a different type, so this did not compile:

```rust
let mut q = query::<Employee>();
if want_eng { q = q.filter(employees::dept.eq("eng")); }   // E0308
```

Now it does, via an erased form whose type stays put:

```rust
fn build(s: &Search) -> Boxed<T> {
    let mut q = boxed_query::<T>();
    if let Some(m) = s.min_score { q = q.filter(t::score.gt(m)); }
    if s.only_named              { q = q.filter(t::nick.is_not_null()); }
    q
}
```

Conditional filters, a query in a struct field, a query returned from a
function, and a `Vec` of differently-shaped queries all work.

- **The type check survives erasure completely.** It fires at the `.gt()` call,
  before the box — `employees::dept.gt(3i64)` is still `E0271` on both paths.
- **Erased once, not twice.** `DynPred` carries the SQL half and the in-memory
  half behind one trait object, keyed on the same bounds `to_memory` already
  requires, so nothing can be boxed for one interpreter and not the other.
- **Sealed.** A public, unsealed `DynPred` would let a hand-written impl make
  SQL select every row and memory select none from the same value, through safe
  API. `mod sealed` prevents it and costs legitimate users nothing.
- **Three-valued**, per D-026: `eval_row` returns `Option<bool>`. Collapsing to
  `bool` inside the box would be wrong under negation, since `is_true(NOT NULL)`
  is `false` while `!is_true(NULL)` is `true`.
- Chosen over a runtime expression AST, which was prototyped and measured at
  2.3×–5.0× slower and which made a type-mismatched comparison representable
  again.

### Added — nullable columns with SQL three-valued logic (D-026)

`linq_rs_sql 0.2.0`. Columns can now be `NULL`, and both interpreters agree
about what that means.

```rust
table! { t (id) { id -> Integer, nick -> Nullable<Text>, score -> Nullable<Integer> } }
pub struct T { pub id: i64, pub nick: Option<String>, pub score: Option<i64> }
entity! { T => t { id: Integer = id, nick: Nullable<Text> = nick, score: Nullable<Integer> = score } }

query::<T>().filter(t::nick.is_null())
query::<T>().filter(t::score.gt(5i64))
```

Previously an `Option<String>` field gave `E0608: cannot index into a value of
type Option<String>` — a raw leak that did not mention nullability.

- **Nullability is a type-level property.** `Nullable<T>` is a distinct SQL
  marker; `Repr<Nullable<T>>::Rust = Option<T::Rust>`, so `Option<bool>` *is*
  the three-valued type and `None` is UNKNOWN. A comparison touching a nullable
  column has type `Nullable<Boolean>` rather than `Boolean`, and both
  interpreters can see the difference.
- **Three values collapse to two only at `WHERE`**, which keeps a row when the
  predicate is TRUE and drops it for FALSE and NULL alike. That is where SQL
  puts the collapse, and it never happens inside the expression tree.
- **The two cells that matter:** `NULL AND FALSE` is FALSE and `NULL OR TRUE` is
  TRUE — an absorbing operand beats the unknown. A naive `Option` zip returns
  UNKNOWN for both. Verified against real SQLite.
- `is_null()` / `is_not_null()` now evaluate in memory; previously `is_null`
  rendered SQL but had no `Eval` impl at all.
- Nullable and non-nullable columns can be compared to each other; the result
  is nullable.

**`entity!` accepts a type rather than a marker name** — `$ty:ty`, not
`$ty:ident` — because `Nullable<Text>` is a type and not a matchable token. No
call site changed: all 184 pre-existing tests passed untouched.

### Fixed — three silent wrong answers in the published crates (D-025)

`linq_rs 0.2.1` and `linq_rs_sql 0.1.1`. All three shipped; all three produced a
plausible wrong result with no warning.

- **`entity!` made the two interpreters disagree.** It generated
  `row.$field as i64` — a silent lossy cast — so a field whose Rust type did not
  match its declared SQL type gave different answers in SQL and in memory for
  the same data. An `f64` field declared `Integer` with value `2.9`: SQL returns
  the row (`2.9 > 2`), the in-memory path drops it (`2.9 as i64 == 2`). It now
  generates `From::from`, so the mismatch is a **compile error** at the
  `entity!` call. `i32 -> i64` and `f32 -> f64` still compile.

  *This can break a build that previously compiled — but only code that was
  already producing wrong answers.*

- **`.offset()` without `.limit()` emitted SQL that does not parse.**
  `SELECT * FROM users OFFSET 20` is valid on PostgreSQL and a syntax error on
  SQLite and MySQL, which parse `OFFSET` only as part of a `LIMIT` clause. A
  passing test asserted the broken string as correct. Now emits
  `LIMIT 9223372036854775807 OFFSET 20`, verified executing on real SQLite.
  (`LIMIT -1` is the SQLite idiom but PostgreSQL rejects a negative limit.)

- **`then_by` after iteration was wrong only in release builds.** The guard was
  `debug_assert!`, which compiles to nothing in release: the comparator was
  silently discarded and the caller got a plausible, wrongly-ordered answer,
  while debug builds panicked. Now `assert!` — it panics in every profile,
  because there is no correct answer to return.

Also fixed: the `#[must_use]` message on `into_lookup` told users to use
`for_each_`, which the v1.0 cut removed — rustc printed that to them.



### Yanked

- **`0.1.0` is yanked.** It was published on 2026-03-28 and should not be used.
  Three defects, any one of which is disqualifying:
  - `then_by` / `then_by_descending` re-sorted the whole buffer on the secondary
    key, discarding the primary ordering. The inline comment asserted the
    opposite. `tests/linq_tests.rs::test_then_by` caught it — and had never been
    compiled, because the file was not a cargo target.
  - `LinqExt::skip` collided with `Iterator::skip`, so merely importing the trait
    turned every unqualified `.skip(n)` in that module into `error[E0034]`,
    including calls on iterators unrelated to this crate. There was no `skip_`
    escape, so the name could not be spelled in method position at all.
  - `concat_`'s bound `I2: IntoIterator<IntoIter = Self>` required the argument's
    iterator type to be identical to the receiver's, which rejects every
    mid-chain call.

  Also shipped in that tarball: the 421-line never-compiled test file, and no
  `repository` link, so the published artifact had no path back to source. See
  `AUDIT.md` finding A-3 and `DECISIONS.md` `D-009`.

### Changed

- **Version is `0.2.0`, not `0.1.1`.** `skip` → `skip_` and the `then_by`
  behaviour change are both breaking. Pre-1.0, a minor bump is the correct
  vehicle (`README.md` § Versioning).
- Scope, naming and API rulings now live in **`DECISIONS.md`** and nowhere else.
  `CLAUDE.md`, `ROADMAP.md` and `README.md` cite `D-NNN` IDs instead of stating
  scope of their own — four documents had drifted into four different answers
  (`AUDIT.md` finding A-1). SQL translation is the v2 thesis (`D-002`);
  `src/sql/` is held on `feature/v0.1.0-and-sql-builder` pending reshape into a
  single query value with two interpreters, rather than merged as a disjoint
  second query vocabulary (`D-205`).
- CI now runs on every branch, gates on the **executed** test count rather than
  exit status (`D-013`), treats broken intra-doc links as errors, and checks
  doctests on the 1.75 MSRV. Previously it ran on none of them: the workflow
  triggered only on `main`/`master`, and had never executed.
- `Cargo.lock` is no longer tracked. It was lockfile v4, which Cargo 1.75 — the
  declared MSRV — cannot parse, so a fresh clone failed before compiling a line.

### Fixed

- Dangling intra-doc link in `src/adaptors.rs` left by the `skip` → `skip_`
  rename. It survived because `cargo doc --no-deps` exits 0 on broken links.

### Known issues in 0.2.0

Recorded here rather than discovered later. The yank notice above lists three
defects in 0.1.0; **two of the three are fixed. This is the third.**

- **`concat_` is still unusable mid-chain.** Its bound
  `I2: IntoIterator<Item = Self::Item, IntoIter = Self>` requires the argument's
  iterator type to be identical to the receiver's, so
  `v.into_iter().where_(..).concat_(other)` fails with `E0271`. Head-of-chain
  same-container calls do work. Use `Iterator::chain`. Unchanged from 0.1.0 —
  fixing it means changing the public `Concat<I>` type, so it is not a
  patch-level change. `AUDIT.md` finding E-8.
- **The operator surface grew from 48 to 90 methods, against this project's own
  stated direction.** `D-005` rules that methods which merely rename an existing
  `Iterator` method should not exist, and `AUDIT.md` finding A-2 identifies the
  alias surface as the crate's principal liability. The growth was merged anyway,
  deliberately, because the 42 new operators were not cleanly separable from the
  correctness fixes in the same commit — see the amendment on `D-015`. The v1.0
  cut line (`AUDIT.md` §7.3) reverses it.
- **Two methods land against DO-NOT-BUILD entries and must not be published.**
  `cast::<U>()` is a fallible conversion that panics via `.expect(...)` with no
  `Result` alternative (`D-204`) — still outstanding. The `D-206` half of this
  entry is **resolved**: `W-10` made the hash-backed algorithm the default and
  renamed the `PartialEq` one to `*_partial_eq`.
- ~~**The quadratic defaults are unchanged.**~~ **Resolved by `W-10`/`W-11`.**
  The defaults are hash-backed and `Lookup` is hash-indexed. The quadratic
  implementations remain only behind the `*_partial_eq` names, where they are
  the documented cost of supporting types that cannot implement `Hash`.
- **`D-005`'s own enforcement gate cannot be written yet.** It requires a test
  importing `LinqExt` and `itertools::Itertools` together, which cannot compile
  while `LinqExt::join` and `LinqExt::group_by` exist under those names. Blocked
  on `W-12`.

### Changed — zero dependencies now means zero, dev included (D-024)

`linq_rs` never had a dependency of any kind. `linq_rs_sql` had one:
`linq_rs = { path = "..", version = "0.2" }` under `[dev-dependencies]`, added
so `tests/seam.rs` could resolve `linq_rs::LinqExt` from the published tarball.

It is gone. Both published crates now declare **no dependencies of any kind**.

- The dev-dependency existed for exactly one import in exactly two files
  (`tests/seam.rs`, `examples/seam.rs`). Both moved verbatim into a new
  workspace member, `seam-tests`, which is `publish = false` and therefore free
  to depend on both crates by path.
- **It was coupling the two crates' release schedules.** `cargo package` strips
  a dev-dependency's `path` and keeps its `version`, turning it into a hard
  registry requirement — so `cargo publish -p linq_rs_sql` failed outright
  until `linq_rs 0.2.0` was live. Verified gone: that command now succeeds with
  `linq_rs 0.2.0` not on crates.io. The two crates can be published in any
  order, together or years apart.
- It also removed the `[patch.crates-io]` special case `msrv-tarball.sh` needed
  to build the sibling's tarball offline. Every tarball now builds `--offline`
  with no help.
- Test count unchanged: 182 / 53 / 235.

The packaging gate previously checked `kind is None` — runtime dependencies
only — which is precisely why a dev-dependency slipped past it. It now asserts
zero dependencies across **all** kinds for both published crates, and that the
publishable set is exactly `linq_rs` + `linq_rs_sql`. Both verified to fail when
the dev-dep is restored and when `publish = false` is removed.

### Fixed — the tarball could not build on its own declared MSRV (D-023)

`linq_rs 0.2.0` was one command from publishing an artifact that does not
build on the version it advertises. It declares `rust-version = "1.65"` and
packaged a `Cargo.lock` at format **v4**:

```
error: failed to parse lock file
Caused by: lock file version `4` was found, but this version of Cargo does not
understand this lock file
```

Reproduced on a genuine 1.65.0 toolchain, and on 1.75.0 — which is *above* the
floor, so 1.65 fails a fortiori.

`D-010` already forbade this, and its fix was to **untrack** `Cargo.lock`. That
was the wrong fix. `cargo package` *always* writes a lockfile into the tarball,
generated by whatever toolchain publishes — so untracking hid the version from
git and from every gate, while guaranteeing the shipped lockfile would be
whatever stable produced that day. D-010's own record of verifying "from a
clean export with no lockfile" describes a state no consumer is ever in.

- `Cargo.lock` is now **tracked at format v3**. Cargo preserves an existing
  lockfile's format version, and with zero dependencies it never churns, so one
  file fixes both the fresh clone and the tarball.
- New gate `.github/scripts/msrv-tarball.sh`, run by the `msrv` CI job:
  packages with **stable** (packaging on the MSRV generates a lockfile the MSRV
  can trivially read — the tautology that let this through), extracts every
  `.crate`, asserts each shipped lockfile is ≤ v3, and builds each on the
  declared floor. Verified to fail when the v4 lockfile is reintroduced.
- `linq_rs_sql` cited `DECISIONS.md` in three shipped files and shipped it zero
  times — it lives at the workspace root, which no member tarball can reach.
  All three now carry the URL, and the packaging gate checks it.

**The general rule:** gate the artifact, not the repository. This is the third
time the two disagreed — the sibling's missing licence texts and its `../`
README links (D-022) were the first two. A check that never extracts a `.crate`
is not checking what ships.

Also corrected: `ci.yml`'s MSRV comment said "actually 1.75" directly above code
asserting 1.65; D-010's Enforced-by still cited a 1.75.0 verification after the
ruling moved to 1.65; and DECISIONS.md restated the sibling's test count as 32
when it is 50. All three are the hand-written-number failure mode these gates
exist to prevent, reappearing inside the gates' own documentation.

### Fixed — `linq_rs_sql` would have shipped broken (D-022)

Found by reading `cargo package --list` rather than the repo. Three defects,
none of which any existing check caught:

- **It declared `MIT OR Apache-2.0` and shipped neither licence text.** The
  packaging gate did check the licence — it read the manifest *field*. A field
  is not a file. Both texts now ship, byte-identical to the workspace copies.
- **Three `../` links in its README** (`../LICENSE-MIT`, `../LICENSE-APACHE`,
  `../README.md`) — fine in the git tree, 404 on crates.io and docs.rs, where
  the tarball has no parent.
- **Its `linq_rs` dev-dependency was path-only, and cargo strips those from the
  published manifest**, so `tests/seam.rs` would not compile from the tarball —
  defeating the reason `tests/` is shipped at all (D-013: the executed-test
  count checkable from the artifact, not merely claimed). Now
  `{ path = "..", version = "0.2" }`.

  This imposes a publish **order**: `linq_rs 0.2.0` must be live before
  `linq_rs_sql` can be packaged or published. That is the natural order anyway.

The link defect had a second instance in `linq_rs` itself: the root README
linked `[`linq_rs_sql`](linq_rs_sql/)`, and workspace members are not in the
root tarball either. Now a GitHub URL.

The packaging gate now checks both READMEs' relative links against each
package's own `cargo package --list` — a relative link is valid iff the tarball
actually ships it — plus the sibling's licence texts and required files. Each
check was verified to fail when the defect is reintroduced.

### Added — `pred!`, closure-shaped filters (D-021)

- `linq_rs_sql` gained `pred!`, a front end over the predicate builder:

  ```rust
  .filter(pred!(employees, |e| e.salary > 100_000 && e.dept == "eng"))
  ```

  expands to exactly

  ```rust
  .filter(employees::salary.gt(100_000).and(employees::dept.eq("eng")))
  ```

  Same types, same SQL, same params, same in-memory evaluation, same laziness —
  asserted, not assumed, by `linq_rs_sql/tests/pred.rs`.

- **Why a macro.** A closure cannot be translated: `|e| e.salary > 100_000`
  compiles to a function and nothing at runtime can ask it which column, which
  operator, which value. C# escapes this with a compiler feature Rust lacks —
  a lambda typed `Expression<Func<T,bool>>` is emitted as a syntax *tree*.
  Rust's substitute is a macro, because macros see syntax before it becomes code.

- **The grammar is small on purpose:** `binding.field OP operand` for the six
  comparison operators, joined by `&&`/`||` with correct precedence, plus
  parentheses. The grammar boundary and the translation boundary are the same
  line — `|e| e.salary * 2 > budget` does not parse, and could not have become
  SQL either. Two `compile_fail` doctests pin that boundary.

- **Error quality, measured rather than assumed.** Both cases point at the
  user's own line and token, not at macro internals: exceeding the grammar gives
  `error: no rules expected '*'` at the `*`, and a mistyped comparison still
  gives `error[E0271]: type mismatch resolving '<i64 as Expr>::SqlType == Text'`
  at the call site.

- **Added `linq_rs_sql::prelude`.** `pred!` expands to methods on the ops traits,
  which must be in scope. Without them the error is actively misleading, because
  `Iterator::gt` exists and rustc finds it instead:
  ``error[E0599]: `salary` is not an iterator``.

- This is **not** the DSL `D-201` rejects. That rules out a
  `from … where … select …` comprehension replacing method chaining. `pred!` is
  one expression macro in one argument position; delete it and nothing changes
  but how the predicate is spelled.

### Added — the two-interpreter seam (W-20, D-002)

**One query value, two interpreters.** The same expression renders to SQL and
evaluates over a `Vec`:

```rust
let q = query::<Employee>()
    .filter(employees::salary.gt(100_000i64))
    .filter(employees::dept.eq("eng"))
    .order_by_desc(employees::salary)
    .limit(2);

q.to_sql();              // SELECT * FROM employees WHERE ((salary > ?) AND (dept = ?))
                         //   ORDER BY salary DESC LIMIT 2   params [Integer(100000), Text("eng")]
q.to_memory(&people);    // the same query, over a &[Employee], lazily
```

No Rust library offered this. Diesel's terminal operations all take
`conn: &mut Conn`; SeaORM's `MockDatabase` replays scripted rows rather than
evaluating; sqlx's checking dies at runtime assembly; polars and datafusion
query columnar frames, not a `Vec<MyStruct>`.

- **It streams, at parity.** 10M rows: **80.7 ms** for the seam, 86.6 ms for
  `linq_rs`, 94.2 ms for hand-written `filter().map()`, same checksum. Laziness
  is pinned by counters, not timing — `LIMIT 1` over 5 rows pulls exactly 1, and
  building the iterator pulls 0. Five designs were measured; runtime-data plans
  came in 2.3–13× slower, so **the predicate is a type**, not a `Vec` of steps.
- **Wrong-typed comparisons do not compile.** `Repr` maps a column's declared
  SQL type to its Rust type, so comparing `Text` to an integer is
  unrepresentable rather than silently `false`.
- **The `D-102` boundary is a compile error**, pinned by a differential doctest
  pair: `query::<Employee>().select_many(..)` is `compile_fail`, and the
  byte-identical expression after `.to_memory(&people)` passes.
- **`linq_rs` is byte-identical.** The seam lives in `linq_rs_sql`, which needs
  no dependency on it. A third crate turned out to be *impossible*: evaluating a
  predicate needs the node fields, which are `pub(crate)` — `error[E0616]` from
  outside.
- Builds on **Rust 1.65**, no dependencies, no proc macro.

### Added — `linq_rs_sql`, a sibling crate (D-020)

The typed SQL query builder now ships, as its own crate in the same workspace.
**Neither crate depends on the other**, and both remain dependency-free.

```rust
use linq_rs_sql::*;
linq_rs_sql::table! { employees (id) { id -> Integer, salary -> Integer } }

let q = employees::table().filter(employees::salary.gt(100_000)).to_sql();
// q.sql    == "SELECT * FROM employees WHERE (salary > ?)"
// q.params == [Integer(100000)]
```

Columns and their types are declared once and checked by the compiler
thereafter; values are always bound, never interpolated; identifiers can only
come from the `table!` declaration.

**Why a sibling and not a module.** `LinqExt::where_` and `sql::filter` mean the
same thing, and one crate with two names for one concept is `AUDIT.md`'s top
finding (A-1), forbidden by `D-205`. **Why not left on a branch:** the value is
real, and keeping working code unreleased to protect a thesis that is not built
yet is a bad trade. Two crates with one vocabulary each beats one crate with two.

It does **not** execute anything — `to_sql()` returns a string and its
parameters for whatever driver you already use. It is intended to become the
rendering backend for the two-interpreter seam (`D-002`), not a competitor to it.

Both gates were extended rather than left pointing at one crate:
`test-count-floor.sh` now counts `--workspace` (a bare `cargo test` at the root
would have silently skipped the sibling's 32 tests — the same "exit 0 having run
nothing" failure it exists to prevent, one directory over), and
`packaging-gate.sh` asserts the sibling's licence, `repository` and empty
dependency list. Running strict rustdoc over the SQL code for the first time
also turned up two latent dangling doc links, now fixed.

### Fixed — the documentation prose had drifted (D-016)

The *generated* blocks were gated from the start. The paragraphs around them
were not — and between them `README.md`, `CLAUDE.md` and `.github/data/README.md`
named **28 methods that had been renamed or cut**, `CLAUDE.md` described a
`src/` layout missing `error.rs` and four test files, still documented the
deleted `ThenBy` trait, and still told a contributor to return `impl Iterator`
from eager operators — which `D-106` now forbids and CI now rejects.

- All three live docs corrected against the current surface.
- `ROADMAP.md` gains a supersession header rather than 38 edits: its phase log
  is accurate as *history*, and rewriting it would falsify the trail. It now
  says so and points at the generated API Reference for what exists.
- **New gate.** `.github/data/removed.tsv` records every removed name with when
  and why, and `gen-docs.py` fails if a live doc mentions one — unless the
  surrounding lines are explaining the removal. Verified: appending
  "use `to_vec()` and `chunk(3)`" to the README fails the build with both names.

### Changed — named return types everywhere (D-106). Breaking, and it unpins the MSRV.

No method on `LinqExt` returns an opaque type any more. All **23** sites now
return named adaptor structs with `pub(crate)` fields — nameable but not
constructible, the same contract as `std::iter::Filter`.

**`AUDIT.md` finding B-1 was wrong on its premise and its count.** It said
opaque returns are "permanently sealed" against a later trait; they are not. A
four-crate semver workspace with ten downstream call patterns compiled
byte-identically against opaque and named libraries — opaque → named is a
*minor* change. And there were 23 sites, not the 8 claimed.

Two reasons to do it anyway, neither previously on file:

1. **The opaque witness carries no operator identity.** `distinct`, `except`
   and `intersect` were all `Filter<I, closure>`, so one blanket impl covered
   all three and a future `Sql` trait could not give them different SQL. That,
   not nameability, is what blocked the v2 seam.
2. **Method-bearing bounds are not additive.** `+ FusedIterator` can be added
   to a shipped opaque return later; `+ ExactSizeIterator` gives downstream
   `error[E0034]`. Only a conditional impl on a named type adds a capability
   without adding an input bound, and RPITIT cannot express one.

**The MSRV drops from 1.75 to 1.65.** Those 23 sites *were* the pin — the floor
was never a considered choice. The full suite passes on a real `rustc 1.65.0`.

The feared unnameable signature never materialised: the eager operators have
already run their closures by the time the type exists, so `inner_join` returns
`InnerJoin<R>` — one parameter, not five. 14 of the 23 were trivial newtypes.
Zero test edits; zero behaviour change.

### Fixed — ordering over borrowed data (D-104). Breaking.

`OrderedQueryable`'s boxed comparator carried an elided `'static`, which
propagated `Self::Item: 'static` onto all eight ordering methods. So
`people.iter().order_by(|p| &p.dept)` — sorting a view of a collection you still
own — did not compile, failing with `error[E0597]`. Introduced by the W-14
rewrite and invisible to the whole suite, because every test and doctest sorted
an owned `Vec` of `'static` elements. `OrderedQueryable` now carries a lifetime
parameter.

This also unblocked `D-104`'s own ruling, which rests on "iterate by reference"
as the answer for borrowed keys — a mitigation that did not work until this was
fixed.

### Changed — `to_lookup`/`to_hashmap` → `into_lookup`/`into_hashmap` (D-108). Breaking.

They consume `self`, and Rust reserves `to_` for borrow-to-owned.

### Removed — the v1.0 cut line (D-019). Breaking.

**`LinqExt` goes from 94 methods to 62.** An operator earns its place if it
could become a SQL clause, or a translatable execution of one, under `D-002`'s
two-interpreter design. Everything else is cut.

The criterion matters more than the number: it follows from the thesis already
chosen, so the surface has a reason to be this shape rather than an arbitrary
size — and it is *derivable*, recorded per method in a `translatable` column, so
the cut is a gate rather than a one-time judgement that decays.

**Cut (32):** the `*_indexed` family, `zip_`/`zip3`, `cast`/`of_type`,
`append_item`/`prepend_item`, `chunk`, `reverse`, `flatten_`, `concat_`,
`take_last`/`skip_last`, `skip_while_`/`take_while_`, `default_if_empty`,
`element_at`/`element_at_strict`/`element_at_or`, `sequence_equal`, `is_empty_`,
`index_`, `aggregate`/`reduce_`/`aggregate_with_selector`, `to_vec`,
`to_hashset`. Nine adaptor structs went with them. None names a SQL concept; all
are `std::iter` in a different spelling — use `collect()`, `chain()`, `rev()`,
`zip()`, `fold()`, `enumerate()`, `eq()`.

**Two calls worth naming.** `into_lookup` and `into_hashmap` are kept while `to_vec`
and `to_hashset` are cut: the first two produce shapes `collect()` cannot, the
last two *are* `collect()`. And the element family (`first`, `single`, …) is
kept because `LIMIT 1` and `LIMIT 2` are real clauses — which is why 62 survive
rather than the ~40 first estimated.

The generated overlap figure moved with it: **59% → 44%** of the surface is a
std rename.

### Added — 1.0 release gate (W-19)

`.github/scripts/release-gate.sh` parses `DECISIONS.md` and **fails a `v1.*` tag
while any `D-1xx` API-stability decision still reads `Status: OPEN`.** Seven do
today.

These are decisions rather than code, so nothing in `src/` can be checked
against them on a branch — but each is free to change now and a breaking change
after 1.0, which makes the tag the exact moment they stop being deferrable.
`v0.x` tags are unaffected, so pre-1.0 releases ship normally.

### Changed — API shape (W-13, E-8, E-9, W-14, W-15). All breaking.

- **`f64` keys can be sorted (W-13).** `order_by` binds `K: Ord`, and the crate
  ships no `IComparer` equivalent, so there was no way to sort by a float at
  all — C# `OrderBy` accepts `double`. New: `order_by_with(cmp)`,
  `then_by_with(cmp)`, `min_by_(cmp)`, `max_by_(cmp)`.
- **`concat_` works mid-chain (E-8).** Its bound was
  `I2: IntoIterator<IntoIter = Self>`, forcing the argument's iterator type to
  be *identical* to the receiver's — so any closure-carrying adaptor upstream
  made it impossible, since no two closures share a type. `Concat` now carries
  two type parameters and the bound is just `IntoIterator<Item = Self::Item>`.
- **`select_many` accepts any `IntoIterator` (E-9).** It bound `J: Iterator`
  while `zip_` bound `IntoIterator` — an inconsistency inside one trait that
  forced a stray `.into_iter()` in the closure. `select_many(|p| p.orders)` now
  works on a `Vec` field.
- **`OrderedQueryable` is an `Iterator` (W-14).** It implemented only
  `IntoIterator`, so every sorting chain needed a manual `.into_iter()` hop and
  `then_by` needed a second import. It now implements `Iterator`,
  `ExactSizeIterator`, `DoubleEndedIterator` and `FusedIterator`, sorting lazily
  on first `next()`. `then_by`/`then_by_descending`/`then_by_with` are inherent
  methods and **the `ThenBy` trait is deleted**. Existing `.into_iter()` calls
  keep compiling (std's blanket impl) but are now redundant, and clippy says so.
- **`Grouping`'s fields are private (W-15).** `key` and `elements` were `pub`
  *alongside* `key()`/`elements()` accessors, so the one-group-per-key invariant
  was unenforceable — a caller could empty a group or forge two sharing a key.
  Use `key()`, `elements()`, `into_elements()`, or the new `into_parts()` to
  take both out at once.

### Changed — no more C#-equivalence claims (W-7)

The crate asserted equivalence with `System.Linq.Enumerable` 74 times in doc
comments and again in the README tables. An audit of every one against live
docs found **69 that were not true as stated**: 2 named methods that do not
exist on `Enumerable` at all, 5 named overloads that do not exist, 23 diverged
in behaviour, and 38 were misleading.

- **All 74 "Equivalent to `X`" phrases are now "C# analogue: `X`."** The
  `LinqExt` trait doc says once that a shared name does not mean shared
  behaviour, rather than each method implying otherwise.
- **Three named things that do not exist**, and now say so: `for_each_`
  (`ForEach` is `List<T>.ForEach`, not LINQ), `element_at_or` (no
  `ElementAtOrDefault(index, defaultValue)` overload exists in any .NET
  version), `zip3` (`Zip`'s three-sequence overload returns tuples and takes no
  selector).
- **`of_type` and `cast` were the sharpest misses.** They are `TryInto` *value*
  conversions; C# `OfType`/`Cast` are runtime *type tests*. `cast::<i64>()`
  widens an `i32` here, while C# `Cast<long>()` on a boxed `int` throws.
- **New `## Differences from C# LINQ` section**, ~260 lines covering empty
  sequences, `single_or_default`, `max_by_key_` tie-breaking, duplicate keys,
  overflow, string collation, `of_type`/`cast`, hash-order grouping, evaluation
  timing, and the operators with no C# counterpart. Because the README is crate
  documentation, **every example in it is a doctest CI runs.**
- **`max_by_key_` ties differ from C#** and this was previously undocumented:
  C# `MaxBy` keeps the *first* maximum, `Iterator::max_by_key` the *last*.
  Measured. `min_by_key_` agrees with C#.

### Changed — the laziness table is measured and generated (W-7)

The README's evaluation-timing bullet flagged *itself* as "hand-maintained and
therefore suspect". It was right to: it was wrong in five places.

- Every operator's timing is now **measured** with a counting source and
  committed to `.github/data/laziness.tsv`: 29 lazy, 20 eager-at-call,
  6 half-eager, 36 terminal. The README table is generated from it and
  `tests/laziness.rs` re-runs the measurements, so neither can drift.
- **`union_` was listed as buffering. It is fully lazy** — W-10's rewrite made
  it `chain().filter()`, which also made it *match* C#'s deferred-streaming
  `Union` rather than diverge from it. The same error had propagated into
  `src/lib.rs`.
- **`order_by` "only stashes comparators" was half true.** The sort is
  deferred; the source is collected at call time.
- **`take_last` is an eager slicing operator**, so "no allocation until you
  collect" was false for it.
- The half-eager class has six members; the README named two.

### Changed — `single_or_default` distinguishes its two failures (W-16)

It returned `None` both when the sequence was empty and when it had more than
one element, so a caller could not tell "not found" from a broken uniqueness
assumption. C# `SingleOrDefault` throws on the latter.

- **`single_or_default` now returns `Result<Option<T>, SingleError>`** —
  `Ok(None)` for empty, `Err(MoreThanOne)` for too many. Breaking, and
  deliberately a compile error at every call site rather than a silent change
  of behaviour.
- **New `try_single() -> Result<T, SingleError>`** for the three-way answer.
- **New `SingleError { Empty, MoreThanOne }`** — the crate's first and only
  error type. The two variants are the complete partition of "not exactly one",
  so it is deliberately not `#[non_exhaustive]`.
- `single`, `single_or` and `try_single` share one panic string via
  `SingleError::message`, so the `# Panics` docs, the panic, and `Display`
  cannot drift apart.
- Old behaviour is one call away: `.single_or_default().ok().flatten()`.

### Fixed — two adaptor defects only visible when driving by hand (W-8, W-9)

Both survived because `collect()` masks them: it stops at the first `None`, and
it never looks at a `Vec`'s capacity.

- **`skip_` ate an element on a non-fused source.** It decremented inside a loop
  guarded by `?`, so a source returning `None` mid-skip left the counter set and
  the next call skipped again. Given `10, 20, None, 40, 50, 60` driven by hand,
  `std` yields `40` and this yielded `50`. Now uses `mem::take` before skipping,
  exactly as `std::iter::Skip` does. `Iterator` only promises `None` is final for
  a `FusedIterator`, so a resuming source is legal, not exotic.
- **`chunk` sized its allocation from its argument.** `chunk(2^28)` over three
  `u64`s reserved 2 GiB; `chunk(2^40)` aborted the process with
  `memory allocation of 8796093022208 bytes failed` — an abort `catch_unwind`
  cannot catch. Capacity now comes from the source's `size_hint`, capped at 4096
  when unknown. The `Vec` still grows as needed.
- **`chunk(0)`'s panic is now documented** with a `# Panics` section. It stays a
  panic: C# throws `ArgumentOutOfRangeException` here and `slice::chunks` panics,
  so this is the idiomatic Rust answer rather than a `NonZeroUsize` signature
  that would make every ordinary call site noisier.

### Added — `FusedIterator` (W-8)

No adaptor implemented it, so any downstream API with a `FusedIterator` bound
rejected every linq_rs query. Fourteen adaptors now do — conditionally on their
source, except `Reverse` (buffers into a `Vec` at construction) and `Chunk`
(latches a `done` flag), which are unconditional.

Not all of them are reachable: the hash-backed defaults return `impl Iterator`,
and `FusedIterator` is not an auto trait, so it cannot leak through the opaque
type. `distinct_partial_eq()` can be fused-bounded; `distinct()` cannot. That is
`AUDIT.md` finding **B-1** in miniature, and closing `D-106` (return named types)
is what fixes it.

### Added — the README is now executable and its tables are generated (W-6, W-18)

`D-016` says any coverage count or C#-mapping table must be generated from the
source and CI-diffed, never hand-written. It had been violated twice — and a
third time was found while building the gate, in the sentence written to fix the
second.

- **`src/lib.rs` now carries `#![doc = include_str!("../README.md")]`.** Every
  ```` ```rust ```` block in the README is a doctest CI runs (41 → 44 doctests).
  Two blocks stated their expected output in a *comment*, so they passed whether
  or not it was true; both now `assert_eq!`, mutation-tested to confirm they bite.
- **The old `//!` crate-doc block is gone.** It was a hand-maintained second copy
  of the crate's pitch and had already drifted — it claimed the operators are
  "all lazy", which `group_by_key`, `union_`, `inner_join` and `group_join`
  disprove.
- **`.github/scripts/gen-docs.py`** derives the public surface from `src/`,
  cross-checks it against `.github/data/operator-map.tsv` both ways, cross-checks
  that file's C# column against `.github/data/csharp-operators.tsv`, verifies
  every public method appears in the README's API Reference, computes every
  count, and fails if the committed README differs. Wired into CI.
- **The corrected figure.** The README claimed `System.Linq.Enumerable` has
  "75 operator names across 234 overloads, 44 comparer overloads". That is the
  **.NET 11 preview** superset: the docs page ships every version's rows in one
  HTML table and filters client-side via `data-moniker`, so counting rows returns
  the newest. Measured per version — net-8: 66/216/33, net-9: 69/220/36,
  **net-10: 74/228/38**, net-11: 75/234/44. The README now says 74/228/38, of
  which this crate implements 65, and the version is pinned in the generator.
- **Ten `*_partial_eq` methods were public but in no API Reference table.** Added.
- **Five relative links (`DECISIONS.md`, `AUDIT.md`, …) 404 on docs.rs** —
  verified against the live 0.1.0 docs — and rustdoc does not warn about them
  even under `-D rustdoc::all`. Now absolute GitHub URLs.

### Fixed — six self-referential doc links (W-10 fallout)

The W-10 rename left `distinct`, `distinct_by`, `except`, `intersect`, `union_`
and `group_by_key` each telling the reader to "prefer this over" *itself*. Found
mechanically, not by reading.

### Changed — hash-backed by default (W-10, breaking)

`D-101` is settled: **`Eq + Hash` is the default bound.**

Ten operators shipped in two forms — a `PartialEq` version that was the default
and a `*_hashed` twin that was fast. That is backwards: the version a caller
reaches for first was the O(n²) one, and the count of implementations doubled to
avoid a breaking change. `D-206` forbids exactly that shape.

- **The hash implementation now has the plain name.** `distinct`, `distinct_by`,
  `except`, `intersect`, `union_`, `group_by_key`, `count_by`, `aggregate_by`,
  `inner_join`, `group_join` are hash-indexed and require `Eq + Hash`.
- **The `PartialEq` implementation is now `*_partial_eq`.** Nothing was deleted;
  the slow path just has to be asked for by name. Long names on purpose.
- **`*_hashed` is gone** as a suffix.
- **`f64` keys no longer work on the defaults** — `vec![1.0].distinct()` will not
  compile; `.distinct_partial_eq()` will. This is the deliberate cost of the
  ruling. The old rationale for `PartialEq`-by-default was that it served float
  keys, but `order_by` binds `K: Ord`, so that audience could never sort anyway.

### Changed — `Lookup` rebuilt (W-11)

The one type whose entire purpose is keyed random access was a linear scan:
`get`, `contains_key` and `insert` all walked every group — measured 288x slower
than `HashMap::get` at 10,000 keys (`AUDIT.md` P-1).

- **Hash-indexed.** `get` and `contains_key` are O(1). Groups are still held in a
  `Vec` so iteration stays **first-appearance order**; a `HashMap<K, usize>`
  points into it. That costs storing each key twice, hence `K: Eq + Hash + Clone`.
- **`count()` → `len()`, plus `is_empty()`** — Rust convention.
- **`insert` is now public.** It was `pub(crate)` while `Default` was public, so
  `Lookup::default()` produced a value that could never be filled.
- **Added `IntoIterator` (owned and borrowed), `FromIterator<(K, V)>`, and
  `PartialEq`/`Eq`.** `Grouping` derived `PartialEq` and `Lookup` did not, so you
  could `assert_eq!` two groupings but not two lookups. The `PartialEq` impl is
  hand-written and compares only the groups — the index is derived state, and a
  `#[derive]` would both have compared it and demanded `K: Hash` needlessly.

### Changed — breaking renames (W-12)

Two method names collided with the most-depended-on iterator crate in Rust, and
one of the two collisions was **silent**.

- **`join` → `inner_join`** (and `join_hashed` → `inner_join_hashed`). Every
  `LinqExt` method takes `self` by value, so it won the by-value step of method
  resolution against `Itertools::join`, which takes `&mut self` — with **no
  ambiguity diagnostic**. A working `.join(", ")` in any crate that imported both
  turned into three unrelated errors that never mentioned `LinqExt`. The name was
  also confusable with the inherent `[T]::join`, which concatenates strings.
- **`group_by` → `group_by_key`** (and `group_by_hashed` →
  `group_by_key_hashed`). `Itertools::group_by` still exists in 0.15 as a
  deprecated alias, and both took `self` by value, so this one was a loud
  `E0034` with no fix short of a fully-qualified call.
- `group_by_with_element` and `group_by_with_result` keep their names — they
  collide with nothing. That leaves the family spelled inconsistently
  (`group_by_key` beside `group_by_with_element`); the v1.0 cut line deletes the
  overloads, so they were not renamed just to be deleted.
- Added `tests/interop.rs`: 4 tests that **pass by compiling**, holding all three
  collisions shut — the two renamed above plus the earlier `skip` → `skip_`.
  This is `D-005`'s enforcement gate, which until now was a gate on paper.

### Packaging and licence (W-17)

- **Dual-licensed `MIT OR Apache-2.0`** (`D-012`), the Rust ecosystem norm.
  `LICENSE` is renamed to `LICENSE-MIT` and `LICENSE-APACHE` is added — the
  canonical Apache-2.0 text, verified by SHA-256 against
  `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30` rather than
  retyped. **0.1.0 remains MIT-only forever**; a published version's licence
  cannot be changed retroactively, so the dual licence begins at 0.2.0.
- **`repository` is set** to `https://github.com/TRget88/LinqLikeForRust`. 0.1.0
  was published with no source link at all, so the artifact had no path back to
  the code. Also removes `homepage`, which pointed at the crate's own crates.io
  page.
- **`exclude` added**, so the tarball no longer carries `AUDIT.md`,
  `QUESTIONS.md`, `CLAUDE.md` or `.github/`. `tests/`, `DECISIONS.md`,
  `ROADMAP.md` and `CHANGELOG.md` are deliberately **kept**: the executed-test
  count should be verifiable from the published artifact and not only from a
  claim in this file.

### Added — `#[must_use]` across the public surface (W-17)

Discarding a query used to be silent. It no longer is, in 52 places.

- All 16 lazy adaptors carry std's own message, "iterators are lazy and do
  nothing unless consumed". `Reverse` gets a truer one — it buffers the whole
  source when constructed.
- `OrderedQueryable` says it buffers on construction and sorts on `into_iter()`,
  so dropping it wastes both. `Grouping` and `Lookup` are annotated too.
- The 35 value-returning terminals (`to_vec`, `sum_`, `any_`, `first`, …) are
  annotated; the four that consume *and* allocate carry a message pointing at
  `for_each_`.
- Nothing was added where it would double-report: the 33 `-> impl Iterator`
  methods are already covered because `Iterator` is itself `#[must_use]` in std,
  `index_` is covered by `std::iter::Enumerate`, and `then_by`/`into_lookup` are
  covered by their return types. `for_each_` is deliberately not annotated —
  it exists for side effects.
- Verified from a downstream crate: 11 discarded results produce exactly 11
  warnings, and `for_each_` produces none.

### Added

#### Phase 1 — close the C# LINQ feature gap
- `default_if_empty(value)`, `take_last(n)`, `skip_last(n)`, `order()`, `order_descending()` (.NET 7+ no-key sorts)
- Strict element accessors that panic on missing input: `first()`, `last_()`, `single()`, `element_at_strict()`
- `reduce_(f)` — no-seed aggregate, alias for `Iterator::reduce`, mirrors C# `Aggregate(func)`
- `aggregate_with_selector(seed, func, result_selector)` — three-arg `Aggregate`
- `sum_by(selector)` — projecting sum
- `except_by`, `intersect_by`, `union_by` — set ops with key selectors (.NET 6+)
- `of_type::<U>()` and `cast::<U>()` — `TryInto`-based type filters; `cast` panics on failure
- Free functions: `linq_rs::range(start, count)`, `linq_rs::repeat(value, count)`, `linq_rs::empty::<T>()`

#### Phase 2 — overloads & ergonomics
- Index-aware variants: `where_indexed`, `select_indexed`, `select_many_indexed`, `skip_while_indexed`, `take_while_indexed`
- `group_by_with_element` and `group_by_with_result` overloads
- `zip3(second, third, result_fn)` — three-way zip
- `first_or`, `last_or`, `single_or`, `element_at_or` — `*OrDefault(defaultValue)` family
- `index_()` — alias for `enumerate`, mirrors .NET 9+ `Index()`
- `count_by(key_fn)` and `aggregate_by(key_fn, seed_fn, accum)` — .NET 9+ fused group+aggregate

#### Phase 3 — performance
- Hash-backed fast paths: `distinct`, `distinct_by`, `except`, `intersect`, `union_`, `group_by_key`, `count_by`, `aggregate_by`, `inner_join`, `group_join`. O(n) instead of O(n²); require `Eq + Hash`.
- `size_hint` propagation on `Skip`, `Take`, `Concat`, `Zip`, `Reverse`, `Chunk`, `DefaultIfEmpty`, `SkipLast`
- `ExactSizeIterator` impls for `Select`, `Skip`, `Take`, `Reverse`, `Concat`, `Zip`
- `DoubleEndedIterator` impls for `Select` and `Reverse`

#### Phase 4 — project hygiene
- Moved sources into `src/` and integration tests into `tests/` (standard Rust layout).
- `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]` on the crate root.
- MSRV pinned to Rust 1.75 (needed for return-position `impl Trait` in trait methods).

### Changed
- **BREAKING:** `LinqExt::skip` renamed to `skip_` to match the project's documented convention (trailing `_` for any method that shadows `Iterator`). The new name also unblocks the `(1..=10).skip(3).take_(4)` pattern.
- **BREAKING:** `OrderedQueryable<T>` rewritten to defer sorting until `into_iter()`. Stores a stack of comparators; `then_by` and `then_by_descending` push onto it instead of re-sorting. The key-function bound tightened from `FnMut` to `Fn + 'static` (needed for the dyn-dispatched comparator storage).
- `union_` no longer requires `Self::Item: Clone` — the unnecessary clone of the collected `self` was removed.

### Fixed
- `then_by` and `then_by_descending` previously re-sorted on the secondary key alone, destroying the primary order. The integration tests in `linq_tests.rs` were not wired into `Cargo.toml` so this never showed up before.

### Infrastructure
- Wired `linq_tests.rs` into `Cargo.toml` as an integration test. Now 128 tests run per `cargo test` (was 0 before).
- Added 40 doctests; `cargo test --doc` runs them.
