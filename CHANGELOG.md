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
