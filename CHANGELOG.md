# Changelog

All notable changes to `linq_rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Hash-backed fast paths: `distinct_hashed`, `distinct_by_hashed`, `except_hashed`, `intersect_hashed`, `union_hashed`, `group_by_hashed`, `count_by_hashed`, `aggregate_by_hashed`, `join_hashed`, `group_join_hashed`. O(n) instead of O(n²); require `Eq + Hash`.
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
