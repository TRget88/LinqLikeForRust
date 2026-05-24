# linq_rs — Project Context

## Mission

A LINQ-style query library for Rust iterators. Brings the full surface of C# LINQ
(filter, project, group, join, aggregate, etc.) to any `Iterator` as a blanket
extension trait. The aim is to feel **familiar to C# devs** and **idiomatic to
Rust devs** at the same time.

**Non-goals:**
- We are not building a query expression DSL (no `from x in xs where ...` macro).
- We are not building an `IQueryable`/expression-tree analogue. Operators run
  against in-memory `Iterator`s, period.
- We do not target databases, async streams, or remote execution.

## Hard constraints

- **Zero dependencies.** `Cargo.toml` is `std`-only. Do not introduce a crate
  dependency without explicit user approval. If a feature genuinely needs one
  (e.g. `rayon`, `serde`), gate it behind an opt-in cargo feature.
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
│   ├── ordered.rs          OrderedQueryable<T> + ThenBy trait
│   └── sources.rs          range / repeat / empty — free fns at crate root
├── tests/
│   └── linq_tests.rs       integration tests
├── examples/               runnable demos: basic_pipeline, join, group_aggregate
├── .github/workflows/ci.yml CI — build / test / clippy / fmt / doc on Linux + Windows
├── README.md
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
| Shadows `Iterator` method | `take_`, `any_`, `all_`, `min_`, `max_`, `sum_`, `concat_`, `union_`, `zip_`, `is_empty_`, `for_each_`, `take_while_`, `skip_while_`, `min_by_key_`, `max_by_key_` |
| Clarity from `std`     | `flatten_` (parallels `Iterator::flatten`) |

When adding a new operator, the rule is: **if Rust or the prelude already
binds the name, add `_`; otherwise don't.** Don't invent cute alternative
names (`filter_via_predicate`, `pick`, etc.) — match C# LINQ semantics first
and resolve the collision with `_`.

For operators that take a value instead of a closure where C# overloads on
type, suffix with `_item` or `_where` (e.g. `append_item`, `first_where`).

## Lazy vs eager semantics

Mirror C# LINQ behaviour:

- **Lazy** (return an adaptor struct that lazily implements `Iterator`):
  `where_`, `select`, `select_many`, `flatten_`, `skip`, `skip_while_`, `take_`,
  `take_while_`, `chunk`, `distinct`, `distinct_by`, `concat_`, `zip_`.
- **Eager** (collect into `Vec` first, then re-yield):
  `order_by`, `order_by_descending`, `reverse`, `union_`, `except`, `intersect`,
  `group_by`, `group_join`, `join`, `to_lookup`.
- **Terminal** (consume the iterator, return a non-iterator value):
  `aggregate`, `sum_`, `count_where`, `min_*`, `max_*`, `average`,
  `first_*`, `last_*`, `element_at`, `single_or_default`, `any_`, `all_`,
  `contains_`, `is_empty_`, `sequence_equal`, `to_vec`, `to_hashmap`,
  `to_hashset`, `for_each_`.

When adding new operators, **document which bucket they fall in** at the top of
the doc comment.

## How adaptors are wired

A new lazy operator requires three pieces:

1. A struct in `adaptors.rs` holding the upstream iterator + any state, with an
   `impl Iterator` block.
2. A method on the `LinqExt` trait in `queryable.rs` that constructs the
   struct. Field access between modules works because the struct fields use
   `pub(crate)`.
3. A re-export. `adaptors.rs` is already glob re-exported from `lib.rs`, so
   anything `pub` there is exposed.

Eager operators usually skip the adaptor struct and just return
`impl Iterator<Item = T>` after collecting internally — see `union_`, `except`,
`intersect`, `join` in `queryable.rs`.

## Doc comments

Every public method in `queryable.rs` has a doc comment with:
- A one-line summary.
- The C# LINQ equivalent it mirrors.
- A doctest showing the smallest realistic call.

Doctests **must compile and pass** under `cargo test`. New operators follow
this pattern — don't merge a method without a doctest.

## Tests

Integration tests live in `linq_tests.rs` at the project root (run by
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
# from D:\RandomProgrammingProjects\LinqLikeForRust
cargo build
cargo test               # runs linq_tests.rs + doctests
cargo doc --no-deps      # build docs locally
```

## When in doubt

Defer to **C# LINQ's documented semantics** for edge cases (empty inputs, null
keys, duplicate keys, ordering stability). If C# and Rust idioms truly conflict,
prefer the Rust convention but call out the deviation in the doc comment.
