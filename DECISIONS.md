# DECISIONS.md — the single source of truth for `linq_rs`

**This file is normative. Everything else in the repo is derived from it.**

If `README.md`, `ROADMAP.md`, `CLAUDE.md`, `AUDIT.md`, a rustdoc comment, a commit
message, or a conversation disagrees with this file, **this file wins** and the
other thing is a bug. If you are about to assume something about scope, naming,
semantics, or dependencies — the answer is here, or it has not been decided yet.

Established 2026-09-09 from the Phase 0 rulings and the `AUDIT.md` findings.

## Why this file exists

The audit found the same scope decision recorded in four places with four
different answers (`AUDIT.md` finding **A-1**). It also found three separate
instances of documentation that had drifted from its own code and stayed wrong for
months. That is the failure mode this file is built against: **assumptions and
rework are what kill a library this size, and they enter through decisions that
were made once, recorded nowhere, and quietly re-made differently later.**

## How to use this file

- **Before writing code**, read the decisions for the area you are touching.
  Assume nothing that is not written here.
- **Before re-opening a settled decision**, read its `Why` and its `Forbids`.
  If the reasoning is genuinely obsolete, do not edit the decision — add a **new**
  decision that supersedes it and set the old one's status to `SUPERSEDED by
  D-NNN`. History is never rewritten.
- **Every decision has an immutable ID.** Cite the ID in commit messages, PRs,
  issues and code comments (`// see D-005`). **Never restate a decision's content
  anywhere else** — link the ID. Restating is how drift starts, and this repo has
  already proved it four times.
- **`Enforced by` is the load-bearing field.** See below.

## The rule that makes this more than prose

A decision recorded only as prose **will** drift. This is not a hypothesis; it is
what happened here three times, and in each case a paragraph would not have
helped:

| What drifted | Why prose could not catch it | The gate that would have |
|---|---|---|
| `then_by`'s inline comment said "stable sort preserves the existing primary order" while the code destroyed it (`ordered.rs:41-42`) | A comment and its code can disagree silently forever. No tool reads comments | A test asserting the *documented* behaviour on **non-degenerate** data. One already existed (`test_then_by`) and would have caught it on day one |
| That test never ran, for five months, while `cargo test` reported success | Nothing in prose distinguishes "passed" from "ran zero tests" | CI asserting an **executed test count** floor, not exit status. `cargo test` printing `running 0 tests` must fail the build |
| The README's laziness table was wrong in five places, and the README's own naming rule was violated on the same page | A hand-maintained table is a second copy of the truth, so it can only decay | **Derive** the table from a test that measures eagerness with a counting source, and fail CI if the table and the measurement disagree. Plus `#![doc = include_str!("README.md")]` so every README code block is a doctest |

So: prefer a compiler error, a test, a clippy lint, or a CI step over a paragraph
asking people to be careful. **A decision whose `Enforced by` reads `prose only`
is a decision that is going to be broken** — treat it as a TODO, not a resting
state.

## Status vocabulary

| Status | Meaning |
|---|---|
| `SETTLED` | Decided. Do not re-open without a superseding decision. |
| `PROVISIONAL` | Decided for now; a named trigger forces a re-decision. |
| `OPEN` | Explicitly not yet decided. Do **not** assume either way. |
| `CONFLICTED` | Decided here, but another in-repo document still says otherwise. Fix the other document. |
| `SUPERSEDED by D-NNN` | Historical. Kept for the reasoning trail. |

## Numbering

`D-0xx` scope, product and process · `D-1xx` API-stability decisions that must be
settled before any 1.0 · `D-2xx` DO-NOT-BUILD.

---

# Open contradictions — fix these first

`AUDIT.md` §1 finding **A-1**. Until these are reconciled, nothing downstream can
be sequenced.

| # | Document | Says | Conflicts with |
|---|---|---|---|
| X-1 | `CLAUDE.md:12-14` | "We are not building an `IQueryable`/expression-tree analogue… We do not target databases" | `D-002`, and the branch's own `src/sql/` |
| X-2 | `ROADMAP.md` *Rejected / out of scope* | "`IQueryable` / expression trees — out of scope. We target in-memory iterators only." | `D-002`, and the commit that shipped "ORM Phase 1" |
| X-3 | `CLAUDE.md:7-8` | the aim is to feel "familiar to C# devs **and** idiomatic to Rust devs at the same time" | `D-005`, which ruled for idiomatic Rust when the two conflict |
| X-4 | `README.md:5` | "Brings the full power of C# LINQ" | `D-016`; measured at 45 of 75 C# operator names and 0 of 44 comparer overloads |
| X-5 | `Cargo.toml` (both trees) | `license = "MIT"` | `D-012` |

**Resolution rule:** each of the above documents must be edited to *cite the
relevant `D-NNN`* rather than restate a scope claim of its own.

---

# Scope

## D-001 — v1.0 is LINQ-to-objects only
- **Status:** SETTLED (2026-09-09)
- **Ruling:** v1.0 ships in-memory query operators. No SQL, connection, entity or
  driver code in the v1.0 crate.
- **Why:** It is the only half that is finished, and the only scope that reaches a
  defensible release without inventing architecture the code does not have.
- **Forbids:** Any README claim implying database support. Any dependency on a
  driver crate in the default feature set.
- **Enforced by:** CI step — `grep` the default-feature dependency tree for driver
  crates; a non-empty result fails. Plus `D-016`'s README gate.

## D-002 — SQL translation is the v2 thesis: designed-for, not built
- **Status:** **CONFLICTED** (see X-1, X-2) — SETTLED here (2026-09-09)
- **Ruling:** Query translation is the stated next product. v1.0 does not ship it,
  but v1.0's public API must not foreclose it.
- **Why:** It is the only capability no Rust library holds. Verified from primary
  sources: Diesel's terminal ops all take `conn: &mut Conn`; SeaORM's most general
  hook receives already-rendered SQL and its `MockDatabase` replays scripted rows
  rather than evaluating; sqlx's checking dies at runtime assembly; polars and
  datafusion query columnar frames, not `Vec<MyStruct>`. The seam is **vacant
  ground, not held ground** — and `rinq` 0.1.0 has already planted a flag on the
  idea with a `rinq_explain!` macro.
- **Also why it is cheap:** a 115-LOC prototype with zero dependencies and zero
  macros produced one query value that evaluates in memory *and* renders
  parameterised SQL. Closure opacity was never the obstacle; the column reference
  inside the closure was. See `AUDIT.md` §7.1.
- **Forbids:** Stabilising any v1.0 API that a deferred/translated provider could
  not satisfy. Specifically: returning RPITIT opaque types from anything the seam
  must reach (`D-101` … `D-105`).
- **Enforced by:** `D-101`–`D-105` must be closed before any 1.0 tag; CI gate on
  the tag, not on the branch.
- **Blocked on:** X-1 and X-2. If the resolution is that v2 is abandoned, this
  decision is superseded and `D-205` applies to `src/sql/`.

## D-003 — Full ORM / EF change tracking is out of scope
- **Status:** SETTLED (2026-09-09)
- **Ruling:** No `DbContext`, identity map or unit of work. If change tracking is
  ever built it is **snapshot-diff** (clone on load, diff on save), never an
  `Rc<RefCell<_>>` identity map.
- **Why:** Change tracking, identity maps and navigation fixup assume one shared
  mutable aliased object graph, which Rust refuses. Snapshot-diff is the only
  point on the curve that gives a recognisable `SaveChanges()` without putting
  `RefCell` in users' signatures or making every future `!Send`.
- **Forbids:** `Rc`/`RefCell`/`Arc<Mutex<_>>` in any public signature for the
  purpose of entity identity.
- **Enforced by:** CI `grep` over `src/` for `Rc<|RefCell|Arc<|Mutex` — currently
  **zero hits in both trees**, so this is true today and the gate keeps it true.
- **Note:** `to_hashmap` must not be used as an identity map — it silently picks a
  winner between two reads of the same key. See `D-018`.

## D-004 — Lazy loading and navigation properties are permanently out
- **Status:** SETTLED (2026-09-09)
- **Ruling:** No field access ever performs I/O. Related data is loaded eagerly
  and explicitly into an owned field.
- **Why:** Implicit loading needs interior mutability or a hidden global, and is
  the largest single source of N+1 pathologies in real EF codebases.
- **Enforced by:** the `D-003` grep, plus code review on any `Deref` impl.

## D-005 — Naming: idiomatic Rust, LINQ-shaped
- **Status:** **CONFLICTED** (see X-3) — SETTLED here (2026-09-09)
- **Ruling:** Keep LINQ names only where `std` lacks the concept. Do not add a
  public method whose only content is delegating to a same-named `Iterator`
  method. Where familiarity and idiom conflict, **idiom wins**.
- **Why:** ~31 of 48 methods on `main` are renames or literal delegations; eight
  call the std method one line down. The trailing-underscore surface is the
  uncanny valley — a C# developer still cannot type `.Where(...)`, and a Rust
  developer sees a worse `.filter()`. Measured: a literal LINQ transcription takes
  three compile-fix cycles and five deviations. Four of the 18 underscores
  (`concat_`, `union_`, `contains_`, `is_empty_`) collide with nothing at all.
- **Forbids:** New pure-alias methods. Trailing underscores on names that do not
  collide.
- **Enforced by:** a test that imports `LinqExt`, `Iterator` **and**
  `itertools::Itertools` in one scope and calls `.skip(1)`, `.join(", ")` and
  `.group_by(..)` — it must compile. That single test pins `D-005`, `E-1`, `E-2`
  and `E-3` shut at once.
- **Recorded dissent:** under a two-interpreter design `where_` and `select` are
  *not* renames of `filter`/`map` — they are clause constructors, and `std` has no
  concept of "a clause that may or may not be a clause". If X-1/X-2 resolve in
  favour of v2, revisit this decision explicitly rather than reading it literally.

## D-006 — Backends (v2 only): PostgreSQL, async only, on `sqlx`
- **Status:** SETTLED (2026-09-09)
- **Ruling:** When v2 is built: PostgreSQL first and only, async only, `sqlx` as
  the driver layer; SQLite solely to make tests hermetic. MySQL and SQL Server are
  not promised.
- **Why:** "Both sync and async" is two surfaces or a generating macro, not a flag.
  Diesel's async story is a separate 0.x crate in a personal repo with a
  thread-pool SQLite shim — the lesson is to make the execution seam pluggable
  from day one, not to fork later.
- **Note:** `sea-query` 1.0.2 is the obvious rendering backend rather than a
  competitor, but its MSRV is 1.88 / edition 2024, so it can only ever be an
  optional feature — never a default. See `D-010`.

## D-007 — Query construction (v2): runtime-built plan, compile-time-checked inputs
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Query *structure* is a runtime value. Table and column identifiers
  come from a generated descriptor, never from a caller-supplied string.
- **Why:** Compile-time-typed structure costs build time and error legibility;
  string-building reopens identifier injection. Descriptor-sourced identifiers
  close that by construction.
- **Forbids:** Any public API that interpolates a caller-supplied string into a
  table or column position. Any operator accepting a raw SQL fragment; if a raw
  escape hatch is ever added it takes a distinct, obviously-unsafe name and does
  not compose with the typed builder.
- **Enforced by:** the branch's `src/sql/` already satisfies this — verified,
  `filter(name.eq("Alice'; DROP TABLE employees; --"))` renders `WHERE (name = ?)`
  with the payload in `params`. Keep an injection-payload test as the gate.

## D-008 — Migrations are delegated
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Not in scope. Point users at `sqlx migrate` or `refinery`.
- **Trigger to revisit:** v2 shipped and in real use.

---

# Product and process

## D-009 — Publishing: the gate is behind us; remediate
- **Status:** SETTLED (2026-09-09) — **supersedes the Phase 0 framing**
- **Ruling:** `linq_rs` **0.1.0 is already published** (crates.io, 2026-03-28, 25
  downloads, not yanked, `repository: null`) and it is `main` — the tree with the
  broken `then_by`, the `.skip` collision and the unusable `concat_`. **Yank it**,
  then supersede with a bumped version. Do not delete, hide, or quietly replace it.
- **Why:** Concrete harm, not embarrassment: merely adding the dependency can
  break unrelated `.skip()` calls in the importing module, and importing it
  alongside `itertools` silently breaks `.join(", ")`. `cargo yank` removes it from
  new resolution while leaving existing lockfiles working, so it costs nothing.
- **Forbids:** Publishing again before `repository`, `readme`, `rust-version`,
  `categories`, `exclude` and the dual licence are set — cargo already prints
  `warning: manifest has no documentation, homepage or repository` and that warning
  was ignored at publish time.
- **Blocked on:** confirming that crates.io user `TRget88` is the owner's own
  account. If it is not, this is an ownership dispute, not a version bump.
- **Enforced by:** a `cargo package --list` check in CI that fails if
  `tests/`/`*_tests.rs` appear in the tarball, plus a manifest-completeness step.

## D-010 — Stable Rust only; MSRV 1.75, declared **and actually exercised**
- **Status:** SETTLED (2026-09-09)
- **Ruling:** No nightly. `rust-version = "1.75"`. CI builds **and runs
  doctests** on exactly 1.75.
- **Why:** Eight return-position-`impl Trait`-in-trait sites pin the floor at
  exactly 1.75 with zero headroom. The branch declares it, but its MSRV job runs
  `cargo test --all-targets`, which **excludes doctests** — and the job never ran
  at all, because the workflow triggers only on `main`/`master`/PR.
- **Forbids:** Committing a `Cargo.lock` newer than Cargo 1.75 can parse. The
  tracked lockfile is **v4**; a fresh clone fails on the declared MSRV before
  compiling a line, verified:
  `lock file version '4' was found, but this version of Cargo does not understand
  this lock file`. `.gitignore` already intends to ignore it; the tracked file
  defeats the pattern.
- **Enforced by:** CI job on `1.75.0` running `cargo test` (not
  `--all-targets`), from a clean clone, on every branch.

## D-011 — `std` only, `alloc` door left open
- **Status:** PROVISIONAL (2026-09-09)
- **Ruling:** Target `std`. Do not add `no_std` support.
- **Why:** Measured — `grouping.rs`, `lookup.rs` and `lib.rs` use no `std::` paths;
  `adaptors.rs` uses only `std::iter`/`std::vec`; genuine `std` need is confined to
  `to_hashmap`/`to_hashset`, so an `alloc`-only build would cover 46 of 48 methods.
  It is *achievable*, which is exactly why it will keep coming up. It has no user.
- **Trigger to revisit:** an actual embedded user asks.

## D-012 — License: dual `MIT OR Apache-2.0`
- **Status:** **CONFLICTED** (see X-5) — SETTLED here (2026-09-09)
- **Ruling:** `license = "MIT OR Apache-2.0"`, both licence files present.
- **Why:** Ecosystem norm; trivial now, consent-requiring once outside
  contributors arrive.
- **Note:** 0.1.0 is published MIT-only and stays MIT forever. Dual licensing can
  only begin at the next version.

## D-013 — The tests must actually run, and the count must not drop
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Every test file is a real cargo target. CI fails if the executed
  test count is zero, and fails if it drops.
- **Why:** `linq_tests.rs` sat at the repo root with 42 `#[test]` functions that
  had never been compiled. `cargo test` reported success while running zero of
  them, and a genuine correctness bug sat undetected behind a test that already
  caught it. The branch fixed the wiring; the *gate* is what stops a recurrence.
- **Forbids:** Treating "cargo test passed" as evidence without a test count.
- **Enforced by:** CI parses `cargo test` output and fails on `running 0 tests` or
  on a total below a committed floor.

## D-014 — This file is the single source of truth
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Scope, naming, semantics and dependency decisions live here and
  nowhere else. Other documents cite `D-NNN`; they do not restate content.
  `AUDIT.md` is a dated evidence snapshot, not a source of decisions — anything in
  it that constrains future work must be promoted to an entry here or it does not
  bind.
- **Enforced by:** the X-table above is the outstanding work list; CI `grep` for
  scope keywords (`out of scope`, `non-goal`, `we do not`, `IQueryable`) in
  `CLAUDE.md`/`ROADMAP.md`/`README.md` that are not accompanied by a `D-` citation.

## D-015 — The unmerged branch: merge correctness and hygiene, not surface
- **Status:** SETTLED (2026-09-09)
- **Ruling:** `origin/feature/v0.1.0-and-sql-builder` (5,873 insertions, 263 green
  tests) is merged in three separable pieces: **(a)** correctness and hygiene —
  the `then_by` comparator rewrite, `skip_`, `union_`'s dead clone, `size_hint`
  impls, `src/` layout, CI, `#![forbid(unsafe_code)]`, the wired tests — merge
  now; **(b)** the 42 new operators — hold pending `D-001`/`D-005`, since most are
  deleted by the cut line; **(c)** `src/sql/` — hold pending X-1/X-2, then either
  become the seam or move out per `D-205`.
- **Why:** (a) is pure win and fixes four audit findings. (b) grows the surface
  that is the crate's principal liability from 48 to 90 methods. (c) is `A-1` in
  code form.
- **Enforced by:** three separate PRs, each citing this ID.

## D-016 — Claims about coverage must be derived, not written
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Any statement of the form "supports N operators", "lazy /eager",
  "equivalent to C# X", or a C#-to-Rust mapping table is **generated from the
  source or from a test**, at doc-build time, and CI fails if the generated form
  differs from the committed one.
- **Why:** Measured drift — the README's laziness table is wrong in five places,
  its naming rule is violated on the same page, "the full power of C# LINQ" is
  45 of 75 names and 0 of 44 comparer overloads, and the flagship example passes
  only because its data is degenerate. Prose warnings about drift do not stop
  drift.
- **Enforced by:** `#![doc = include_str!("README.md")]` so every README block is
  a doctest, plus a generator + CI diff for every table.

## D-017 — Document the deviations once, do not chase C# semantics
- **Status:** SETTLED (2026-09-09)
- **Ruling:** A single `# Differences from C# LINQ` section covers duplicate-key
  `to_hashmap`, `sum_` overflow, byte-ordinal string ordering, `Option` aggregate
  flow, empty-sequence `None`, and `MaxBy` tie-breaking. The behaviour does not
  change.
- **Why:** Six audit findings dissolved into "this behaves exactly like the `std`
  method it delegates to". Chasing the C# contract would make the crate worse Rust,
  and culture-aware collation needs an ICU dependency `D-001` forbids. See `D-203`.

## D-018 — `Option<T>` null semantics are documented, never SQL-shaped
- **Status:** SETTLED (2026-09-09)
- **Ruling:** `Option<T>` is an ordinary value: `None` sorts first, compares equal
  to `None`, and forms its own group. Document it. Do not special-case it.
- **Why:** This matches C# LINQ-to-Objects and **does not** match SQL. Measured:
  `where_(|x| *x < Some(2))` **keeps** the `None` rows, because `Ord for Option`
  puts `None` first — in SQL they are dropped; PostgreSQL's default is `NULLS
  LAST` where Rust's is nulls-first; and `sum_` returns `None` if any element is
  `None`, annihilating the aggregate, where C# and SQL both skip nulls. Three
  different answers for one operator.
- **Blocks:** `D-103` — a v2 that promises "same query, same answer" would be
  lying.

---

# API stability — must be closed before any 1.0

All `OPEN`. Each is free now and a breaking change later.

## D-101 — Key bound: `Hash + Eq` or `PartialEq`?
- **Status:** OPEN. **Recommended: `Hash + Eq`**, with `*_by` comparator variants
  *named in the 1.0 docs* so adding them later is purely additive.
- **Why it matters:** determines the complexity class of every hash-backed
  operator *and* whether `f64` keys compile at all. `PartialEq` gives no
  reflexivity, so `Lookup::get(&f64::NAN)` cannot find a key it just inserted.
  `ROADMAP.md` defends `PartialEq` as serving float keys — but `order_by` binds
  `K: Ord`, so that audience cannot sort anyway. The stated rationale does not
  hold. `Eq` turns every NaN case into a compile error, which is what
  `HashSet`/`HashMap`/`Itertools::unique` already do.

## D-102 — The v2 translation boundary
- **Status:** OPEN. **Recommended: compile error, with an explicit one-token
  opt-in** (`.to_memory()`) that consumes the queryable and returns a plain
  `Iterator` on which the full surface reappears.
- **Why it matters:** EF Core ran this experiment — silent client-side fallback
  before 3.0, documented as causing "unnoticed performance issues", changed to a
  runtime throw in 3.0. C# cannot do better because `IQueryable<T>` exposes the
  same methods regardless of provider. **Rust can**, and this crate already
  accidentally proves it: the boundary is 100 % compile-time today.
- **Also forbids, if adopted:** partial translation ("translate the prefix,
  evaluate the suffix") and runtime capability flags — provider subsets are
  per-provider trait method sets.
- **Hard constraint from the audit (`B-2`, compiler-verified):** v2 must be
  **purely additive**. Widening `where_<P: FnMut(&T)->bool>` into
  `where_<P: IntoPredicate<T>>` breaks every existing call site with
  `error: implementation of 'FnMut' is not general enough`, and `for<'a>` does not
  fix it. Plan a *second* method (`where_expr`), never a widened `where_`.

## D-103 — Three-valued logic across the seam
- **Status:** OPEN. **Recommended:** do not promise result identity. Promise a
  **stability class per operator, declared per provider**.
- **Why it matters:** "same query, two backends, same answer" is not achievable by
  default. Rust `Ord for str` is byte-ordinal, C# `OrderBy` is culture-aware,
  PostgreSQL orders by the column's collation — measured, `["a","B","c","D"]`
  sorts to `["B","D","a","c"]`, and canonically-equal NFC `é` and NFD `e\u{301}`
  land on opposite sides of `"f"`. Add `D-018`'s null split on top.

## D-104 — Key-selector signature
- **Status:** OPEN. **Recommended:** decide explicitly between an HRTB/GAT form,
  `K: Borrow<..>`, and accept-and-document-with-the-clone-cost-stated.
- **Why it matters:** every key selector is `FnMut(&Self::Item) -> K` with `K`
  free, so a key cannot borrow from an owned item. Every shipped method takes a
  key selector, so this is unfixable after 1.0 without breaking the whole API. The
  diagnostic is a bare `error: lifetime may not live long enough` — no error code,
  no suggested fix. Mitigation that already works and must be documented either
  way: iterating by reference (`.iter()`) makes borrowed keys work with zero
  clones.

## D-105 — `Fn` vs `FnMut` on predicates and selectors
- **Status:** OPEN. **Recommended: `Fn`** on anything the plan vocabulary might
  ever contain.
- **Why it matters:** currently inconsistent — `order_by` binds `FnMut`
  (`queryable.rs:229`), `join` binds `Fn` (`:460`). A translator needs purity, and
  narrowing later is breaking: verified, a v1-legal counting closure against a v2
  `Fn` bound gives `error[E0594]: cannot assign to 'calls', as it is a captured
  variable in a 'Fn' closure`.

## D-106 — Named return types for anything the seam must reach
- **Status:** OPEN. **Recommended: return named types** from `join`, `group_join`,
  `group_by` and any future relational operator.
- **Why it matters:** `B-1`, compiler-verified — the eight `-> impl Iterator` sites
  in trait position permanently seal those operators against any future trait
  (`error[E0599]: no method named 'sql' found for opaque type`), and they are the
  three operators the v2 thesis needs most. The same sites pin the MSRV at exactly
  1.75 with zero headroom.

## D-107 — Seal the public traits
- **Status:** OPEN. **Recommended: seal both.**
- **Why it matters:** `LinqExt` is de facto sealed by its blanket impl, but
  `ThenBy` is a public unsealed trait with exactly one impl, so any added method is
  potentially breaking for a downstream implementor. Free now, impossible later.

## D-108 — `to_` vs `into_`, and `#[must_use]`
- **Status:** OPEN. **Recommended:** rename consuming conversions to `into_`; add
  `#[must_use]` to every deferred return.
- **Why it matters:** `to_lookup` consumes `self` against the convention reserving
  `to_` for borrow-to-owned, and clippy does not catch it. And there is **zero**
  `#[must_use]` in either tree, so a discarded eager `order_by` — which allocates
  and sorts — emits no diagnostic where std's `Filter`/`Map` both warn.

---

# DO-NOT-BUILD

Cite these IDs when the idea comes back. Full reasoning in `AUDIT.md` §9.

| ID | Do not build | One-line reason |
|---|---|---|
| `D-201` | A `linq!` / `from…where…select` comprehension macro | Three Rust crates tried it (2017, 2019, 2021); **all three are dead**, while the extension-trait crate in the space has 1.48 B downloads |
| `D-202` | Pluggable `IEqualityComparer`/`IComparer` per call (44 C# overloads) | Rust expresses this with traits and newtypes. The legitimate subset is `*_by` variants |
| `D-203` | Full C# semantic fidelity | Would make the crate worse Rust to match a foreign contract; culture-aware collation needs an ICU dependency `D-001` forbids. See `D-017` |
| `D-204` | `TryInto`-based `cast::<U>()` that panics on the first bad element | A fallible data conversion that panics, with no `Result` alternative. C# needs it for runtime downcasting; Rust has none |
| `D-205` | Two vocabularies for one concept in one crate | `LinqExt::where_` and `sql::filter` mean the same thing and share no value. The SQL builder either becomes the seam (`D-002`) or its own crate |
| `D-206` | A second implementation of every operator (`*_hashed` twins) | Doubles the surface to dodge a breaking change and leaves the slow version as the default. Pick the right default; make the other a named escape hatch |
| `D-207` | Benchmarks via nightly `test::Bencher`, or `criterion` in the main crate | `ROADMAP.md` deferred benchmarks for this reason and was right. The way out is a workspace `benches/` member |
| `D-208` | `async` before the sync seam exists | Diesel's async story is a separate 0.x crate in a personal repo with a thread-pool SQLite shim. Make the execution seam pluggable, not forked. See `D-006` |
