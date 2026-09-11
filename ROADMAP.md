# linq_rs — Roadmap

This roadmap translates the **C# LINQ surface** into concrete tasks for `linq_rs`.
It is organized by phase, with each phase shippable on its own.

**This file is a work list, not a source of decisions.** Scope and API rulings
live in [DECISIONS.md](DECISIONS.md); cite `D-NNN` rather than restating them.
Phases 1 and 2 grew the surface from 48 to 90 methods, which `AUDIT.md` finding
A-2 identifies as the crate's principal liability — see the v1.0 cut line in
`AUDIT.md` §7.3 before adding another operator.

Legend: `[x]` shipped · `[ ]` todo · `[~]` partial / has gaps

---

> ## ⚠ Phases 0–2 are superseded by `D-019`
>
> Those phases record what was *built*, and they are accurate as history. They
> are **not** a description of the current surface: the v1.0 cut line removed 32
> of the 94 operators they list, on the criterion "could this become a SQL
> clause under the v2 thesis". `to_lookup`/`to_hashmap` were also renamed to
> `into_*` (`D-108`), and `join`/`group_by` to `inner_join`/`group_by_key`
> (`W-12`).
>
> **For what exists today, read the API Reference in `README.md`** — it is
> generated from the source and CI-diffed, so it cannot drift. This file is kept
> for the reasoning trail, not as a surface listing.

## Phase 0 — Currently shipped

These are already implemented. They form the baseline to build on.

- [x] **Filtering:** `where_`
- [x] **Projection:** `select`, `select_many`, `flatten_`
- [x] **Paging:** `skip`, `skip_while_`, `take_`, `take_while_`, `chunk`
- [x] **Set ops:** `distinct`, `distinct_by`, `except`, `intersect`, `union_`, `concat_`
- [x] **Ordering:** `order_by`, `order_by_descending`, `then_by`, `then_by_descending`, `reverse`
- [x] **Aggregation:** `aggregate(seed, f)`, `sum_`, `count_where`, `min_`, `max_`, `min_by_key_`, `max_by_key_`, `average`
- [x] **Elements:** `first_or_default`, `first_where`, `last_or_default`, `last_where`, `element_at`, `single_or_default`
- [x] **Quantifiers:** `any_`, `all_`, `contains_`, `is_empty_`
- [x] **Joining:** `inner_join`, `group_join` *(renamed from `join` in W-12 — see `D-005`)*
- [x] **Grouping:** `group_by_key` (key-only) *(renamed from `group_by` in W-12 — see `D-005`)*
- [x] **Conversion:** `to_vec`, `to_hashmap`, `to_hashset`, `to_lookup`
- [x] **Utility:** `zip_`, `append_item`, `prepend_item`, `for_each_`, `sequence_equal`

---

## Out-of-band fixes (shipped alongside Phase 1.1)

These were unplanned but needed: the integration tests in `linq_tests.rs`
were not wired into `Cargo.toml`, so they had never actually run. When
they were wired up, two pre-existing bugs surfaced and were fixed in the
same change:

- [x] **Register `linq_tests.rs` as an integration test** — added `[[test]]` block in `Cargo.toml`. 57 tests now run on every `cargo test`.
- [x] **`skip` → `skip_` rename** (breaking) — `LinqExt::skip` collided with `Iterator::skip` but lacked the trailing `_` per the project's documented convention. Renamed to match `take_`, `any_`, `all_`, etc.
- [x] **`then_by` correctness fix** — `OrderedQueryable` was eagerly sorting in `order_by` and `then_by` re-sorted on the secondary key *alone*, destroying the primary order. Refactored to defer sorting and stack comparators; sort runs once at `into_iter` time using the comparators in lexicographic order. The `Fn` bound on key selectors tightened from `FnMut` to `Fn` + `'static` to allow boxed dyn dispatch (custom mutable-state key functions are not a realistic use case).

---

## Phase 1 — Close the C# LINQ feature gap

The operators a C# dev would expect to see and not find today.

### 1.1 Missing operators (no Rust equivalent in std)

- [x] **`default_if_empty(default)`** — if the iterator yields nothing, yield `default` once instead. Equivalent to `DefaultIfEmpty(T)`.
- [x] **`take_last(n)`** — yield only the last `n` items (eager — must buffer).
- [x] **`skip_last(n)`** — yield all but the last `n` items (lazy with a ring buffer of size `n`).
- [x] **`order()` / `order_descending()`** — .NET 7+ sort by the element itself (no key fn), for `T: Ord`.

### 1.2 Missing terminal operations

- [x] **`first()` / `last_()` / `single()` / `element_at_strict()`** — strict variants that **panic** when there is no match, alongside the existing `_or_default` versions. Decision: panic, not `Result`. Rationale: the strict variant is opted into explicitly; users wanting recoverable errors should use the `_or_default` form and `.ok_or(my_err)`. Matches C# `InvalidOperationException` semantics.
- [x] **`reduce_(f)`** (formerly `aggregate_no_seed`) — `Aggregate(func)` overload using the first element as the seed. Naming chosen to align with [`Iterator::reduce`]; doc-cross-references the LINQ equivalent.
- [x] **`aggregate_with_selector(seed, accum, result_selector)`** — three-arg `Aggregate(seed, func, resultSelector)`.
- [x] **`sum_by(selector)`** — `Sum(selector)`; sum of mapped projection.
- [~] **`min_by(selector)` / `max_by(selector)`** — **skipped intentionally.** C# `MinBy(keySelector)` / `MaxBy(keySelector)` return the *element* by projected key, which is already exactly what `min_by_key_` / `max_by_key_` do. Adding a second name would muddy the API. Doc comments were tightened to call out the C# parity. If a user later needs "min of the projected value" (C# `Min(selector)`), the idiomatic Rust is `iter.select(sel).min_()`.
- [~] **`long_count()` / `long_count_where(p)`** — **skipped intentionally.** Rust's `usize` is the right return type for collection lengths; an `i64`-typed parallel would just be cosmetic .NET parity with no Rust use case.

### 1.3 Set ops with key selectors (.NET 6+)

- [x] **`except_by(other, key_fn)`** — `other` supplies keys, matches C# `ExceptBy(IEnumerable<TKey>, ...)`.
- [x] **`intersect_by(other, key_fn)`** — `other` supplies keys, matches C# `IntersectBy(IEnumerable<TKey>, ...)`.
- [x] **`union_by(other, key_fn)`** — `other` supplies items (same type as `self`), matches C# `UnionBy(IEnumerable<TSource>, ...)`. Cross-side dedup verified by `test_union_by_cross_side_dedup`.

**Bonus cleanup:** dropped the unnecessary `Self::Item: Clone` bound and wasted `combined.clone()` allocation from `union_` while in the neighborhood. Existing tests still pass.

### 1.4 Generator helpers (.NET static `Enumerable.*`)

Exposed at the crate root (overriding the original `linq::*` path — `linq_rs::linq::range` would be redundant given the crate name).

- [x] **`linq_rs::range(start, count)`** — equivalent to `Enumerable.Range`. Wrapper over `(start..).take(count)`.
- [x] **`linq_rs::repeat(value, count)`** — equivalent to `Enumerable.Repeat`. Wrapper over `std::iter::repeat(...).take(count)`.
- [x] **`linq_rs::empty::<T>()`** — equivalent to `Enumerable.Empty<T>()`. Wrapper over `std::iter::empty`.

These live in `sources.rs` and are re-exported from `lib.rs`.

### 1.5 Type-tag operators

These are awkward in Rust (no runtime polymorphism), but we can support
useful subsets via `TryInto`.

- [x] **`of_type::<U>()`** — uses `TryInto<U>`, drops elements where conversion fails. Lazy.
- [x] **`cast::<U>()`** — also uses `TryInto<U>`, but **panics** on the first failure (faithful to C# `Cast<U>()` semantics). The roadmap originally suggested aliasing `select(|x| x.into())` (infallible `Into`), but using `TryInto` preserves the panic-on-bad-element behaviour and works for both widening (`i32 → i64`) and narrowing (`i64 → i32`) numeric conversions. Requires `Error: Debug` for the panic message.

---

## Phase 2 — Overloads, indexed variants, and ergonomics

C# LINQ has many "with index" / "with comparer" overloads. Rust can't overload
on signature, so each becomes its own named method.

### 2.1 Index-aware variants

- [x] **`select_indexed(|x, i| ...)`** — equivalent to `Select((x, i) => ...)`.
- [x] **`where_indexed(|x, i| ...)`** — equivalent to `Where((x, i) => ...)`.
- [x] **`select_many_indexed(|x, i| ...)`**
- [x] **`take_while_indexed(|x, i| ...)`**
- [x] **`skip_while_indexed(|x, i| ...)`**

All implemented as one-line composition over `Iterator::enumerate()` —
no new adaptor structs needed.

### 2.2 GroupBy overloads

- [x] **`group_by_with_element(key_fn, element_fn)`** — transforms each item before bucketing.
- [x] **`group_by_with_result(key_fn, result_fn)`** — returns the projected aggregate per group.

### 2.3 Zip overloads

- [x] **`zip3(second, third, result_fn)`** — three-way zip (.NET 6+ has `Zip(second, third, resultSelector)`).

### 2.4 Comparer overloads

**Out of scope, intentionally.** Rust uses traits (`Ord`, `PartialEq`)
instead of pluggable comparer objects. Users who need custom equality
should wrap items in a newtype.

### 2.5 `*OrDefault(default)` (.NET 6+)

- [x] **`first_or(default)` / `last_or(default)` / `single_or(default)` / `element_at_or(index, default)`** — return the given default value instead of relying on `T: Default`. `single_or` panics on multiple-element input (matches .NET 6 `SingleOrDefault(default)` which still throws in that case).

### 2.6 Index() (.NET 9+)

- [x] **`index_()`** — yields `(index, item)` pairs. Thin alias for `Iterator::enumerate`.

### 2.7 Counting variants (.NET 9+)

- [x] **`count_by(key_fn)`** — group by key and yield `(key, count)` pairs.
- [x] **`aggregate_by(key_fn, seed_fn, accum)`** — fused `GroupBy` + `Aggregate`; `seed_fn` receives the key so per-key seeds are possible.

---

## Phase 3 — Performance

The current implementation prioritizes simplicity and zero deps. Several
operators were O(n²) where O(n) is achievable.

- [x] **Hash-backed algorithms are now the DEFAULT (`W-10`, supersedes the
  `*_hashed` twins below).** `distinct`, `distinct_by`, `except`, `intersect`,
  `union_`, `group_by_key`, `count_by`, `aggregate_by`, `inner_join`,
  `group_join` and `to_lookup` all use a hash index and require `Eq + Hash`.
  The `PartialEq` linear-scan implementations survive as `*_partial_eq` escape
  hatches for types that cannot implement `Hash`. The `*_hashed` names are gone.
  See `DECISIONS.md` `D-101` (settled) and `D-206`.
- [x] ~~**`distinct` / `distinct_by` fast path** — added `distinct_hashed` /
  `distinct_by_hashed`~~ *(superseded: the twins were the wrong shape — they left
  the quadratic version as the default a caller reaches for first.)*
- [x] **`size_hint` propagation** — added to `Skip`, `Take`, `Concat`, `Zip`, `Reverse`, `Chunk`, `DefaultIfEmpty`, `SkipLast`. `Select` already had it. The unpredictable ones (`Where`, `SkipWhile`, `TakeWhile`, `Distinct*`, `Flatten`, `SelectMany`) intentionally omitted — they can't give better than the default `(0, None)` without lying.
- [x] **`DoubleEndedIterator` / `ExactSizeIterator` impls** — `Select` (both), `Skip` (ESI), `Take` (ESI), `Reverse` (both), `Concat` (ESI), `Zip` (ESI). DEI on `Skip` and `Take` deferred — they require non-trivial buffering against the ESI len and are niche use cases.
- [~] **Benchmarks** — **deferred.** The built-in `test::Bencher` harness requires nightly Rust; the project is stable-only. Bringing in `criterion` would violate the zero-dep policy. Revisit when the project moves to a workspace where a `bench/` member with a one-off `criterion` dev-dep wouldn't pollute the main crate.

---

## Phase 4 — Project hygiene & layout

- [x] **Move source into `src/`** — standard Rust layout. `[lib]` block dropped from `Cargo.toml` (auto-discovery).
- [x] **Move `linq_tests.rs` into `tests/`** — standard integration test location. `[[test]]` block dropped.
- [x] **`examples/`** — three runnable examples: `basic_pipeline`, `join`, `group_aggregate`.
- [x] **CI** — `.github/workflows/ci.yml` runs build, test, clippy `-D warnings`, fmt `--check`, doc on Linux + Windows. Separate `msrv` job pinned to Rust 1.65 (see `D-010`).
- [x] **`#![warn(missing_docs)]`** — applied on the crate root. Caught two undocumented public fields on `Grouping<K, V>` (now documented).
- [x] **`#![forbid(unsafe_code)]`** — applied on the crate root.
- [x] **`CHANGELOG.md`** — Keep-a-Changelog format with the Phase 1–4 work documented.
- [x] **`rust-version` / MSRV** — pinned to `1.65`. It was `1.75`, forced by 23 return-position-`impl Trait`-in-trait sites; `D-106` converted those to named types for unrelated reasons and the floor dropped ten releases.
- [~] **`rustdoc` polish** — partial. The crate-level doc comment already shows a quick-start; a fuller side-by-side cheatsheet inside the rustdoc is deferred (it would duplicate the README).

**Other Phase 4 fixes shipped while we were here:**
- `cargo fmt` applied across the codebase. CI's `--check` step now passes.
- Two clippy lints addressed: `manual_div_ceil` (used `usize::div_ceil`) and `wrong_self_convention` on `is_empty_` (allow-listed with a doc comment explaining why the LINQ terminal-op convention diverges from std).

---

## Phase 5 — Release & distribution

- [x] **Publish to crates.io** — `linq_rs` is owned by the user; no rename needed. Actual `cargo publish` is the user's call.
- [x] **Fill in `Cargo.toml` metadata** — `homepage`, `documentation`, `categories`, `keywords`, `readme`, `license`, `rust-version`, `description` all set. `repository` left unset (no public source repo to point to — fill in when one exists).
- [x] **Semver policy** — documented in `README.md` under "Versioning". Headline: adding a `LinqExt` method is a minor bump, not breaking; tightening trait bounds is breaking; pre-1.0 anything can break on a minor.
- [ ] **First tagged release (`v0.1.0`)** — ready to tag. Run `git tag v0.1.0 && cargo publish` when you're ready. CI must be green first (use the workflow's first run as the gate).
- [ ] **`v1.0.0`** — after the API has marinated through at least one real user.

---

## Phase 2.8 — `linq_rs_sql` call-site ergonomics

Both found by writing real calls rather than by reading the API, which is the
only way this class of thing surfaces.

### 2.8.1 The typed query is one-shot for in-memory use

`Rows::to_memory` takes `self` by value, so this does not compile:

```rust
let q = query::<Employee>().filter(pred!(employees, |e| e.salary > 100_000i64));
let sql = q.to_sql();                      // borrows
let got = q.to_memory(&rows);              // moves  -> E0382
```

The caller has to bind `q.to_sql()` first. The **erased** form
(`boxed_query()`) takes `&self` and can be used repeatedly, so the asymmetry
runs backwards from expectation: the ergonomic form is the type-erased one.

`to_memory` returns a lazy iterator borrowing the predicate, which is why it
took ownership. Worth checking whether `&self` is achievable now that `D-027`
proved it for `BoxedRows`.

### 2.8.2 Literals need explicit type suffixes

`e.salary > 100_000` does not infer `i64` from the column; `100_000i64` is
required. C# infers it, and this is the most visible remaining difference at a
`pred!` call site. Likely wants the comparison operators to accept anything
`Into<Lit<Self::SqlType>>` rather than a bare `Expr`, so an untyped integer
literal has somewhere to land.

---

## Phase 2.9 — Schema drift is the one EF protection with no counterpart

`table!` is a **hand-written declaration with no link to the real database**.
Everything it asserts is enforced against the *declaration*, never against the
schema. Demonstrated against real SQLite:

```
declared: table! { emp (id) { id -> Integer, salary -> Integer, bonus -> Integer } }
actual  : CREATE TABLE emp(id INTEGER, salary TEXT)

compiles cleanly, emits:  SELECT id, salary, bonus FROM emp WHERE (bonus > ?)
the database says:        no such column: bonus
```

This is what EF closes with scaffolding (model from database) or migrations
(database from model). Two candidate directions, neither started:

- **Verify at runtime, once.** A `check_schema(&conn)` that compares
  `ALL_COLUMNS` and the declared SQL types against the driver's introspection and
  returns a diff. Cheap, needs a driver, catches drift at startup rather than on
  the first query that touches the missing column.
- **Generate the declaration.** A `table!` emitted from the live schema, so the
  two cannot disagree. Bigger, needs a build step, and is the EF answer.

Worth noting what is *already* protected so the gap is not overstated — typo'd
column, wrong type, wrong table, hostile input and NULL semantics are all caught
at compile time (`D-026`, `D-028`, `D-029`). Drift is the one that is not.

---

## Phase 5.5 — To revisit

Decided-for-now, with a named reason to look again. See `DECISIONS.md`.

- [ ] **`itertools` as a dev-dependency for interop testing** — *deferred, not
  rejected* (`D-005`). Today the real-crate check runs as a CI-only job
  (`.github/scripts/itertools-interop.sh`) that builds a throwaway crate
  depending on both, so `itertools` never enters `Cargo.toml`, never ships in the
  published manifest, and a bare checkout still tests offline. The always-on
  gate is the mimic in `tests/interop.rs`.

  **To be clear about what this is not:** it is not a dependency on itertools'
  *algorithms*. `linq_rs` has and keeps its own `distinct`, `order_by`, `join`
  and everything else. The only thing under test is whether the two method-name
  sets can coexist in one scope.

  **Revisit if:** the CI job proves too flaky to keep (it needs the network), or
  the interop surface grows enough that a throwaway crate stops being a
  reasonable way to express it, or the zero-dev-dependency stance is relaxed for
  another reason (benchmarks under `D-207` would be the likeliest trigger, since
  a `criterion` dev-dependency raises the same question).

## Phase 6 — Optional / opt-in features

Each of these would ship behind a cargo feature flag (no impact on the
default zero-dep build).

- [ ] **`parallel` feature** — `rayon` integration. Expose `par_where_`, `par_select`, etc. on `ParallelIterator`.
- [ ] **`serde` feature** — `Serialize` / `Deserialize` impls for `Grouping` and `Lookup`.
- [ ] **`async` feature** — equivalent extension trait for `futures::Stream`.

These are speculative — defer until someone actually asks for them.

---

## Rejected / out of scope (decisions we've made)

> Superseded by [DECISIONS.md](DECISIONS.md). Do not add rulings here — add a
> `D-NNN` entry there and cite it. This section previously read "`IQueryable` /
> expression trees — out of scope. We target in-memory iterators only", while
> the same branch shipped a 1,104-line SQL query builder committed as "ORM
> Phase 1" against a roadmap that has no ORM phase. That is `AUDIT.md` finding
> A-1, and it is the reason `DECISIONS.md` exists.

- **`IQueryable` / expression trees** — **no longer rejected.** `D-002` makes SQL
  translation the v2 product, on a one-query-two-interpreters design. Not built;
  designed-for. `D-101`..`D-108` are the decisions that keep it reachable.
- **A `from/select/where` macro DSL** — out of scope. See `D-201`: three Rust
  crates tried it (2017, 2019, 2021) and all three are dead.
- **Pluggable `IEqualityComparer<T>` per call** — out of scope. See `D-202`. The
  legitimate subset is `*_by` key/comparator variants, which are wanted (`W-13`).
- **`Cast<T>`** — shipped in Phase 1.5, and `D-204` says it should not have been:
  a fallible data conversion that panics, with no `Result` alternative. C# needs
  it for runtime downcasting; Rust has none. Slated for removal.

---

## How to extend this roadmap

When adding a task:
1. Drop it under the right phase (or open a new phase if the task is genuinely new in kind).
2. Reference the C# LINQ method it maps to, if any.
3. Note Phase 3 perf work separately from Phase 1 API additions — don't bundle.
4. When a task ships, flip `[ ]` to `[x]` in the same PR that lands the change.
