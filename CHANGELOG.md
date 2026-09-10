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
  `Result` alternative (`D-204`). The ten `*_hashed` twins leave the quadratic
  implementation as the default a caller reaches for first, doubling the surface
  to avoid a breaking change (`D-206`). `W-10` makes the hash-backed algorithm
  the default instead of a twin.
- **The quadratic defaults are unchanged.** `distinct`, `except`, `intersect`,
  `union_`, `group_by_key`, `to_lookup`, `inner_join` and `group_join` are all O(n²) in the
  default form, and `Lookup::get`/`contains_key` are linear scans. Measured
  crossover into visible slowness is around n≈17,000–30,000. `AUDIT.md` §4.3.
- **`D-005`'s own enforcement gate cannot be written yet.** It requires a test
  importing `LinqExt` and `itertools::Itertools` together, which cannot compile
  while `LinqExt::join` and `LinqExt::group_by` exist under those names. Blocked
  on `W-12`.

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
  `index_` is covered by `std::iter::Enumerate`, and `then_by`/`to_lookup` are
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
- Hash-backed fast paths: `distinct_hashed`, `distinct_by_hashed`, `except_hashed`, `intersect_hashed`, `union_hashed`, `group_by_key_hashed`, `count_by_hashed`, `aggregate_by_hashed`, `inner_join_hashed`, `group_join_hashed`. O(n) instead of O(n²); require `Eq + Hash`.
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
