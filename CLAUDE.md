# linq_rs — Project Context

> **Scope lives in [DECISIONS.md](DECISIONS.md), not here.** This file describes
> how to work in the repo. It must not state scope of its own — cite a `D-NNN`
> instead. Three earlier versions of this file stated scope directly and all
> three drifted out of agreement with the code (see `AUDIT.md` finding A-1).

## Mission

A LINQ-style query library for Rust iterators: LINQ-shaped operators over any
`Iterator`, via a blanket extension trait.

Where familiarity to C# and idiomatic Rust conflict, **idiom wins** — that is
`D-005`, and it is a ruling, not a balance to strike per API.

Do not describe the crate as bringing "the full surface" or "the full power" of
C# LINQ: `System.Linq.Enumerable` has 75 operator names and 44 comparer
overloads, and this crate covers neither set completely. **Do not write a
coverage count anywhere** — `D-016` requires it to be generated from the source,
and the first draft of this very paragraph shipped a hand-written figure that was
measured on a different tree and was wrong by ~20 names. That is the failure mode
`D-016` exists to prevent, and it caught itself here. `W-18` lands the
generator.

**Scope, by reference — read the entry before assuming:**
- `D-001` v1.0 is LINQ-to-objects only.
- `D-002` **SQL translation is the v2 product.** One query value, two
  interpreters — evaluate over a collection, or render to SQL. This is the
  crate's thesis and the only capability no Rust library holds.
- `D-003` no ORM change tracking or identity map; `D-004` no lazy loading.
- `D-201` no `from x in xs where ...` macro DSL.
- `D-205` **one vocabulary per concept.** A second, parallel query surface in
  this crate is forbidden even when it works — that is why `src/sql/` is held on
  `feature/v0.1.0-and-sql-builder` rather than merged.
- `D-101`..`D-108` are **OPEN** and must close before any 1.0.

## Hard constraints

- **Zero dependencies, and this one is not negotiable** (`D-032`):
  - `linq_rs` — **no dependencies.**
  - `linq_rs_sql` — **only `linq_rs`.**
  - **Nothing else is acceptable.**

  Not "ask first", and **not** "gate it behind an opt-in cargo feature" — that
  escape hatch used to be written here and it is exactly the route `D-032`
  forbids. An optional dependency is still a dependency: it is in the manifest,
  it ends the claim as written, and it reaches the lockfile of everyone who
  wanted none. A whole crate was deleted for taking one (`linq_rs_sqlite`, whose
  single driver dependency pulled 24 crates into the resolved graph). If a
  feature needs a third-party crate, the feature does not belong here — publish
  it as a separate crate outside this workspace, or define a trait and let the
  caller supply the glue, as `RowSource` does.
- **Stable Rust.** No nightly features.
- **No `unsafe`.** This library is a thin layer over safe iterator combinators —
  there is no reason to reach for `unsafe`.

## File layout

Standard Rust layout (moved from a flat root layout during Phase 4):

```
LinqLikeForRust/
├── Cargo.toml
├── src/
│   ├── lib.rs              crate root — re-exports public API
│   ├── queryable.rs        the LinqExt trait — every method lives here
│   ├── adaptors.rs         iterator adaptor structs (Where, Select, Take, ...)
│   ├── grouping.rs         Grouping<K, V> — output of group_by
│   ├── lookup.rs           Lookup<K, V> — one-to-many dictionary
│   ├── ordered.rs          OrderedQueryable<'a, T> + its inherent then_by*
│   ├── error.rs            SingleError — the crate's only error type
│   └── sources.rs          range / repeat / empty — free fns at crate root
├── tests/
│   ├── linq_tests.rs       operator tests
│   ├── edge_cases.rs       edge cases and regressions
│   ├── adaptor_contracts.rs  contracts only visible when driving by hand
│   ├── laziness.rs         re-measures the evaluation-timing classification
│   └── interop.rs          coexistence with a competing extension trait
├── examples/               basic_pipeline, join, group_aggregate
├── .github/
│   ├── workflows/ci.yml    nine gates; see "Gates" below
│   ├── scripts/            the gates themselves
│   └── data/               inputs to the derived-docs generator
├── README.md               ALSO the crate docs, via include_str! in lib.rs
├── DECISIONS.md            NORMATIVE — read before assuming anything
├── AUDIT.md                dated evidence snapshot; not normative
├── CHANGELOG.md
├── ROADMAP.md
└── CLAUDE.md               this file
```

New modules go in `src/` and get wired into `src/lib.rs`. New integration
tests go in `tests/`. New examples go in `examples/` (one `main` fn per file).

## Naming conventions

Methods that **collide with Rust keywords or `std::iter::Iterator` methods** get
a trailing underscore:

| Reason for `_` suffix | Examples |
|---|---|
| Reserved keyword       | `where_` |
| Shadows `Iterator` method | `all_`, `any_`, `contains_`, `last_`, `max_`, `max_by_`, `max_by_key_`, `min_`, `min_by_`, `min_by_key_`, `skip_`, `sum_`, `take_`, `union_` |

`skip_` carries the suffix for a reason worth remembering: as bare `skip` it
collided with `Iterator::skip`, and merely importing `LinqExt` turned every
unqualified `.skip(n)` in the module into `error[E0034]` — including calls on
iterators unrelated to this crate. That rename is why 0.2.0 is a breaking bump.
`union_` and `contains_` collide with nothing on `Iterator` and keep the suffix
only for family consistency; see `D-005`.

| Clarity from `std`     | `union_` (parallels the set-operation family) |

When adding a new operator, the rule is: **if Rust or the prelude already
binds the name, add `_`; otherwise don't.** Don't invent cute alternative
names (`filter_via_predicate`, `pick`, etc.) — match C# LINQ semantics first
and resolve the collision with `_`.

For operators that take a value instead of a closure where C# overloads on
type, suffix with `_where` (e.g. `first_where`, `last_where`).

## Lazy vs eager semantics

**Do not restate the classification here.** It is measured with a counting
source, committed to `.github/data/laziness.tsv`, generated into the README's
*Evaluation timing* section, and re-measured by `tests/laziness.rs`. A copy in
this file would be a second source of truth, which is what `D-016` forbids — and
the copy that used to live here had already drifted.

The four classes are `lazy`, `eager_at_call`, `half_eager` (receiver streams,
argument drained at call time) and `terminal`. When you add an operator you must
add its row to `laziness.tsv`; `gen-docs.py` fails if a method has no
classification.

Note this crate's eager operators drain the source when **called**, where C#
defers to the first `MoveNext`. That difference is real and documented.

## How adaptors are wired

A new lazy operator requires three pieces:

1. A struct in `adaptors.rs` holding the upstream iterator + any state, with an
   `impl Iterator` block.
2. A method on the `LinqExt` trait in `queryable.rs` that constructs the
   struct. Field access between modules works because the struct fields use
   `pub(crate)`.
3. A re-export. `adaptors.rs` is already glob re-exported from `lib.rs`, so
   anything `pub` there is exposed.

**No method may return `impl Iterator`.** `D-106` requires named return types
and CI enforces it by grepping `queryable.rs` for `-> impl Iterator` and failing
on any hit. Eager operators return a named newtype over `vec::IntoIter` — see
`InnerJoin<R>`, `GroupByKey<K, V>`, `Except<I>` in `adaptors.rs`. Opaque returns
carry no operator identity (`distinct`, `except` and `intersect` were all
`Filter<I, closure>`, so one blanket impl covered all three), and they were what
pinned the MSRV at 1.75.

## Doc comments

Every public method in `queryable.rs` has a doc comment with:
- A one-line summary.
- The C# LINQ equivalent it mirrors.
- A doctest showing the smallest realistic call.

Doctests **must compile and pass** under `cargo test`. New operators follow
this pattern — don't merge a method without a doctest.

## Tests

Integration tests live in `tests/` — `linq_tests.rs` and `edge_cases.rs` (run by
`cargo test`). Each operator has at least one happy-path test; aim to also
cover:
- empty-input behaviour,
- single-element input,
- ordering preservation across operations that don't reorder,
- chained composition with at least one other operator.

When changing an operator's behaviour, **update its tests in the same diff**
— don't leave drifted assertions.

## Build / test commands

```powershell
# from the repository root
cargo build
cargo test               # tests/linq_tests.rs + tests/edge_cases.rs + doctests
# CI does not run bare `cargo test` -- it runs the gate below, because
# `cargo test` exits 0 while running zero tests (D-013):
./.github/scripts/test-count-floor.sh
cargo doc --no-deps      # build docs locally
```

## When in doubt

Defer to **C# LINQ's documented semantics** for edge cases (empty inputs, null
keys, duplicate keys, ordering stability). If C# and Rust idioms truly conflict,
prefer the Rust convention but call out the deviation in the doc comment.
