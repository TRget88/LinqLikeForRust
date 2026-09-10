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

**Resolution rule:** each document must *cite the relevant `D-NNN`* rather than
restate a scope claim of its own.

| # | Document | Was | Status |
|---|---|---|---|
| X-1 | `CLAUDE.md` Non-goals | "We are not building an `IQueryable`/expression-tree analogue… We do not target databases" | **RESOLVED** 2026-09-09 — replaced with a scope-by-reference list citing `D-001`…`D-205`; the non-goal is struck, `D-002` stands |
| X-2 | `ROADMAP.md` *Rejected* | "`IQueryable` / expression trees — out of scope. We target in-memory iterators only." | **RESOLVED** 2026-09-09 — entry now reads "no longer rejected", cites `D-002`, and records that the SQL builder was committed as "ORM Phase 1" against a roadmap with no ORM phase |
| X-3 | `CLAUDE.md` Mission | "familiar to C# devs **and** idiomatic to Rust devs at the same time" | **RESOLVED** 2026-09-09 — now states that idiom wins, citing `D-005` |
| X-4 | `README.md:5` | "Brings the full power of C# LINQ" | **RESOLVED** 2026-09-09, then **re-fixed the same day.** The first fix replaced the claim with a hand-written count ("45 of C#'s 75 operator names") that was measured on `main` and wrong by ~20 names for this tree — written, with no irony intended, in the same sentence as a `D-016` citation saying counts must be generated. The README now states the C# denominators (75 names, 234 overloads, 44 comparer overloads — verified against learn.microsoft.com), states that this crate covers neither set completely, and **refuses to give a coverage figure** until `W-18` derives one |
| X-5 | `Cargo.toml` | `license = "MIT"` | **OPEN** — `W-17`. Must be `MIT OR Apache-2.0` with both licence files before the next publish |

The thesis ruling that closed X-1..X-3 (2026-09-09): **keep the SQL thesis.**
`D-002` is the crate's reason to exist; `src/sql/` is held rather than merged
because it is a disjoint second vocabulary (`D-205`), and is raw material for the
seam, not the seam.

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
- **Enforced by:** **IMPLEMENTED** — `packaging-gate.sh` asserts the crate has
  **zero** dependencies via `cargo metadata`, so any driver or SQL crate fails
  the build. (A grep for driver names would have been weaker; zero is exact.)

## D-002 — SQL translation is the v2 thesis: designed-for, not built
- **Status:** SETTLED (2026-09-09). X-1 and X-2 resolved the same day.
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
- **Enforced by:** *(NOT YET IMPLEMENTED — `W-19`.)* The intended gate is CI
  refusing a `v1.*` tag while any `D-1xx` reads `Status: OPEN`.
- **Resolved:** the owner ruled to keep the thesis (2026-09-09). `D-205` still
  applies to `src/sql/` as written — it is held on
  `feature/v0.1.0-and-sql-builder` pending the reshape (`W-20`).

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
- **Enforced by:** **IMPLEMENTED** — `packaging-gate.sh` greps `src/` for
  `Rc<|RefCell|Arc<|Mutex<|RwLock<` and fails on any hit. This was true by
  accident; the gate makes it true on purpose, so a later contributor cannot
  quietly reintroduce the identity map this decision forbids.
- **Note:** `to_hashmap` must not be used as an identity map — it silently picks a
  winner between two reads of the same key. See `D-018`.

## D-004 — Lazy loading and navigation properties are permanently out
- **Status:** SETTLED (2026-09-09)
- **Ruling:** No field access ever performs I/O. Related data is loaded eagerly
  and explicitly into an owned field.
- **Why:** Implicit loading needs interior mutability or a hidden global, and is
  the largest single source of N+1 pathologies in real EF codebases.
- **Enforced by:** **IMPLEMENTED** for the interior-mutability half, via
  `D-003`'s grep. The "no I/O behind field access" half is **not** gated — it
  would need a check on `Deref`/`Index` impls, and there are none today.

## D-005 — Naming: idiomatic Rust, LINQ-shaped
- **Status:** SETTLED (2026-09-09). X-3 resolved the same day.
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
- **Enforced by:** **PARTIALLY IMPLEMENTED** as of `W-12` —
  `tests/interop.rs`, 4 tests that pass *by compiling*. `W-12` renamed
  `join` → `inner_join` and `group_by` → `group_by_key`, which removed both
  collisions, and the test file now pins them shut: a competing extension trait
  with itertools 0.15's exact receiver shapes (`fn join(&mut self, sep)` and
  `fn group_by(self, key)`) coexists with `LinqExt` in one scope; `.skip(n)` on
  unrelated iterators still resolves to std; and the inherent `[T]::join` /
  `reverse` / `to_vec` still win on a `Vec`.
  **Plus a real-crate check, added the same day:** the CI job
  `itertools-interop` runs `.github/scripts/itertools-interop.sh`, which builds a
  throwaway crate in a temp directory depending on both `linq_rs` and the real
  `itertools`, and fails if they cannot coexist. Verified in both directions —
  it passes on this tree, and against a copy with `inner_join` reverted to `join`
  it reproduces the original three-error cascade and exits 1 with the correct
  diagnosis. It also distinguishes a genuine collision from an infrastructure
  failure rather than blaming the former for the latter.
- **Ruling on the dev-dependency (2026-09-09):** `itertools` is **not** added
  under `[dev-dependencies]`. Empirically, a dev-dependency never reaches
  consumers — a downstream crate's tree stays `consumer → linq_rs` and
  `itertools` appears zero times in its lockfile — so the zero-dependency promise
  to *users* would have survived. It was declined for three other reasons: it
  ships in the published manifest (verified by extracting the `.crate` tarball),
  it ends the fully-offline `git clone && cargo test`, and it contradicts
  `CLAUDE.md`'s hard constraint as literally written. The CI job gets the same
  coverage at none of those costs. **Deferred, not rejected** — see
  `ROADMAP.md` Phase 5.5 for the named triggers to revisit.
- **Not a dependency on itertools' algorithms.** `linq_rs` keeps its own
  `distinct`, `order_by`, `inner_join` and the rest. The only thing these gates
  test is whether the two method-name sets can coexist in one scope.
- **Accepted carve-out (2026-09-09), following the X-1 ruling:** `where_`,
  `select`, `order_by`/`then_by`, `join`, `group_join` and `group_by` are
  **kept**, because under `D-002`'s two-interpreter design they are clause
  constructors, not renames of `filter`/`map`/`sort_by_key` — `std` has no
  concept of "a clause that may or may not be a clause". The deletion list is
  therefore the *terminal and scalar* aliases (`sum_`, `min_`, `max_`,
  `min_by_key_`, `max_by_key_`, `any_`, `all_`, `for_each_`, `aggregate`,
  `to_vec`, `to_hashset`, `first_or_default`, `first_where`, `last_or_default`,
  `element_at`, `sequence_equal`, `flatten_`, `skip_`, `take_`, `skip_while_`,
  `take_while_`, `index_`, and the `*_indexed` family), which translate to
  nothing and delegate to std one line down. This narrows `D-005` from ~31
  deletions to ~25 and is the version the v1.0 cut line should use.

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
- **Enforced by:** *prose only, and nothing to gate until v2 exists.* At that
  point: a CI matrix pinned to exactly the declared backends, so adding a fourth
  requires editing this decision first.

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
- **Enforced by:** *(NOT APPLICABLE YET.)* `src/sql/` is held out of this line
  (`D-015(c)`), so there is nothing here to gate. It was verified on the branch —
  `filter(name.eq("Alice'; DROP TABLE employees; --"))` renders `WHERE (name = ?)`
  with the payload in `params` — and an injection-payload test must land with the
  code when it does.

## D-008 — Migrations are delegated
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Not in scope. Point users at `sqlx migrate` or `refinery`.
- **Trigger to revisit:** v2 shipped and in real use.

---
- **Enforced by:** *prose only.* There is nothing to gate while no schema-diff
  code exists; the gate becomes a CI grep for a migrations module if v2 ships.

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
- **Enforced by:** **IMPLEMENTED** — `.github/scripts/packaging-gate.sh`, wired
  into CI. It asserts `repository` is set, that the tarball does **not** carry
  `AUDIT.md`/`QUESTIONS.md`/`CLAUDE.md`/`.github/`, and that it **does** carry
  `tests/`, `DECISIONS.md` and both licence files — because the executed-test
  count should be verifiable from the published artifact, not only claimed in a
  README. Verified in both directions: it passes on this tree and fails when
  `exclude` is removed or `repository` is deleted.

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
- **Enforced by:** **IMPLEMENTED** — the `msrv` CI job pins `1.75.0` and runs
  `test-count-floor.sh`, which covers both `--all-targets` and `--doc`; the
  latter is the bucket a bare `cargo test --all-targets` would have skipped.
  Verified on a genuine 1.75.0 toolchain: 237 tests pass from a clean export
  with no lockfile. The CI job now also asserts `rustc --version` is 1.75.x
  before trusting the run — during this remediation two local verifications used
  a `PATH` that silently fell through to stable, which produces a confident and
  false MSRV claim.

## D-011 — `std` only, `alloc` door left open
- **Status:** PROVISIONAL (2026-09-09)
- **Ruling:** Target `std`. Do not add `no_std` support.
- **Why:** Measured — `grouping.rs`, `lookup.rs` and `lib.rs` use no `std::` paths;
  `adaptors.rs` uses only `std::iter`/`std::vec`; genuine `std` need is confined to
  `to_hashmap`/`to_hashset`, so an `alloc`-only build would cover 46 of 48 methods.
  It is *achievable*, which is exactly why it will keep coming up. It has no user.
- **Trigger to revisit:** an actual embedded user asks.
- **Enforced by:** *prose only, deliberately.* A `no_std` CI target would be
  the gate, and by this ruling it should not exist. Revisit with the trigger.

## D-012 — License: dual `MIT OR Apache-2.0`
- **Status:** SETTLED (2026-09-09). X-5 still OPEN in `Cargo.toml` — see `W-17`.
- **Ruling:** `license = "MIT OR Apache-2.0"`, both licence files present.
- **Why:** Ecosystem norm; trivial now, consent-requiring once outside
  contributors arrive.
- **Enforced by:** **IMPLEMENTED** — the same packaging gate asserts
  `license == "MIT OR Apache-2.0"`, that `LICENSE-MIT` and `LICENSE-APACHE` both
  exist, and that `LICENSE-APACHE` hashes to the canonical Apache-2.0 text
  (`cfc7749b…`), so a truncated or edited copy is caught. Verified failing when
  the field is reverted to `MIT`.
- **Note:** 0.1.0 is published MIT-only and stays MIT forever. Dual licensing can
  only begin at the next version — done at 0.2.0.

## D-013 — The tests must actually run, and the count must not drop
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Every test file is a real cargo target. CI fails if the executed
  test count is zero, and fails if it drops.
- **Why:** `linq_tests.rs` sat at the repo root with 42 `#[test]` functions that
  had never been compiled. `cargo test` reported success while running zero of
  them, and a genuine correctness bug sat undetected behind a test that already
  caught it. The branch fixed the wiring; the *gate* is what stops a recurrence.
- **Forbids:** Treating "cargo test passed" as evidence without a test count.
- **Enforced by:** **IMPLEMENTED** — `.github/scripts/test-count-floor.sh`,
  run by both the `test` and `msrv` jobs. It enforces a floor **per bucket**
  (`--all-targets` and `--doc` separately) as well as on the total, because one
  bucket collapsing while the other grows is exactly the historical bug.
  Verified against `main` @ `bd9fd4f`: `executed test count dropped from 236 to
  17`, exit 1.

## D-019 — The v1.0 cut line: keep what can become a SQL clause
- **Status:** SETTLED (2026-09-09) — applied.
- **Ruling:** an operator earns its place in v1.0 if it could become a SQL
  clause, or a translatable execution of one, under `D-002`'s two-interpreter
  design. Everything else is cut. `LinqExt` went **94 → 62 methods**.
- **Why this criterion rather than a size target:** it follows from the thesis
  already chosen, so the surface has a reason to be the shape it is instead of
  an arbitrary count. It is also *derivable*: the `translatable` column in
  `.github/data/operator-map.tsv` records the classification per method, so the
  cut is checkable rather than a one-time judgement that decays.
- **The 32 cut:** the `*_indexed` family, `zip_`/`zip3`, `cast`/`of_type`,
  `append_item`/`prepend_item`, `chunk`, `reverse`, `flatten_`, `concat_`,
  `take_last`/`skip_last`, `skip_while_`/`take_while_`, `default_if_empty`,
  `element_at*`, `sequence_equal`, `is_empty_`, `index_`, `aggregate`/`reduce_`/
  `aggregate_with_selector`, `to_vec`/`to_hashset`. None of them names a SQL
  concept; all are `std::iter` in a different spelling.
- **Two judgement calls worth naming.** `to_lookup` and `to_hashmap` are kept
  while `to_vec` and `to_hashset` are cut: the first two produce shapes
  `collect()` cannot (a multimap; a keyed map with a selector), the last two
  *are* `collect()`. And the whole element family (`first`, `single`, …) is kept
  because `LIMIT 1` and `LIMIT 2` are real clauses, which is why the surviving
  count is 62 rather than the ~40 first estimated.
- **Forbids:** adding an operator whose `translatable` value would be `none`.
  If you want one anyway, change this decision first.
- **Enforced by:** **IMPLEMENTED** — `gen-docs.py` requires every `LinqExt`
  method to carry a `translatable` value in `operator-map.tsv` and fails on a
  method it does not know, so a new operator cannot be added without classifying
  it. It also now checks the README in **both** directions: a public method
  missing from the API Reference, *and* an API Reference row naming a method
  that no longer exists — the second check was added because the cut left 32
  such rows behind.

## D-014 — This file is the single source of truth
- **Status:** SETTLED (2026-09-09)
- **Ruling:** Scope, naming, semantics and dependency decisions live here and
  nowhere else. Other documents cite `D-NNN`; they do not restate content.
  `AUDIT.md` is a dated evidence snapshot, not a source of decisions — anything in
  it that constrains future work must be promoted to an entry here or it does not
  bind.
- **Enforced by:** *(NOT YET IMPLEMENTED.)* The intended gate is a CI `grep` for
  scope keywords (`out of scope`, `non-goal`, `we do not`, `IQueryable`) in
  `CLAUDE.md`/`ROADMAP.md`/`README.md` that are not accompanied by a `D-`
  citation. Until it exists, this decision is enforced by review alone — which is
  precisely how the four-way contradiction in the X-table arose.

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
- **Enforced by:** *(process only, and it was not followed — see the amendment
  below.)* The intent was three separate PRs, each citing this ID.
- **AMENDED 2026-09-09, same day.** The owner ruled to merge **(a) and (b)
  together**, minus (c), on the evidence that (b) is *not* cleanly separable: the
  branch is a single commit, `src/queryable.rs` is a full rewrite that git sees
  as delete+add, and the 192 tests in `tests/linq_tests.rs` and
  `tests/edge_cases.rs` cover the old and new methods together. Extracting (b)
  would have meant hand-splitting a 1,513-line file with no green-test safety
  net, to remove methods that the v1.0 cut line deletes anyway. So `LinqExt`
  went **48 → 90 methods** on this branch, deliberately and unpublished.
  Two consequences must be recorded rather than left implicit, because the merge
  landed methods that this file forbids:
  - `cast::<U>()` violates **`D-204`** — `TryInto` plus `.expect(...)`, a fallible
    data conversion that panics with no `Result` alternative.
  - The ten `*_hashed` twins (`distinct_hashed`, `distinct_by_hashed`,
    `except_hashed`, `intersect_hashed`, `union_hashed`, `group_by_hashed`,
    `group_join_hashed`, `join_hashed`, `count_by_hashed`,
    `aggregate_by_hashed`)
    violated **`D-206`** — a second implementation of every operator that left
    the quadratic version as the default a user reaches for first.
    **Fixed by `W-10`.**
  Both are on the `W-10`/`W-18` cut-line list. Neither may be published: `D-009`
  gates publishing, and the cut line comes first.

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
- **Enforced by:** **IMPLEMENTED** (`W-6`, `W-18`) — two mechanisms:
  - `src/lib.rs` carries `#![doc = include_str!("../README.md")]`, so every
    ```` ```rust ```` block in the README is a doctest CI runs. Two blocks that
    stated their expected output in a *comment* — passing whether or not it was
    true — now `assert_eq!`. Mutation-tested: changing one expected value fails
    the build.
  - `.github/scripts/gen-docs.py --check`, run in CI. It **derives** the public
    surface from `src/`, cross-checks it against `.github/data/operator-map.tsv`
    in both directions, cross-checks that file's C# column against
    `.github/data/csharp-operators.tsv`, verifies every public method appears in
    the README's API Reference, computes every count, and fails if the committed
    README differs from what it generates.
- **Vindicated three times over.** `D-016` was violated twice before this gate
  existed, and a third time was found while building it: the README's
  "75 operator names / 234 overloads / 44 comparer overloads" is the **.NET 11
  preview** superset. The docs page ships every version's rows in one table and
  filters client-side, so a row count returns the newest. The .NET 10 figures are
  74 / 228 / 38 — now generated, and version-pinned by `TARGET_DOTNET`.

## D-017 — Document the deviations once, do not chase C# semantics
- **Status:** SETTLED (2026-09-09)
- **Ruling:** A single `# Differences from C# LINQ` section covers duplicate-key
  `to_hashmap`, `sum_` overflow, byte-ordinal string ordering, `Option` aggregate
  flow, empty-sequence `None`, and `MaxBy` tie-breaking. The behaviour does not
  change.
- **Why:** Six audit findings dissolved into "this behaves exactly like the `std`
  method it delegates to". Chasing the C# contract would make the crate worse Rust,
  and culture-aware collation needs an ICU dependency `D-001` forbids. See `D-203`.
- **Enforced by:** **IMPLEMENTED** (`W-7`). The README's
  *Differences from C# LINQ* section is crate documentation, so all of its
  examples are doctests CI runs — every divergence it claims is executed. And
  no doc comment asserts equivalence any more: all 74 "Equivalent to `X`"
  phrases were demoted to "C# analogue: `X`", with the trait-level doc saying
  once that a shared name does not mean shared behaviour. Three that named
  methods or overloads which **do not exist** (`for_each_`→`ForEach`,
  `element_at_or`→a nonexistent `ElementAtOrDefault` overload, `zip3`→a
  nonexistent `Zip` overload) now say so.

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
- **Enforced by:** **PARTIALLY IMPLEMENTED** (`W-7`). The *Differences from C#
  LINQ* section's examples are doctests, so the documented `Option` behaviour is
  executed. A dedicated `Option`-semantics test file is still worth adding when
  the v2 SQL work starts, since that is when the three-way split matters.

# API stability — must be closed before any 1.0

All `OPEN`. Each is free now and a breaking change later.

**Shared gate for this whole section — IMPLEMENTED (`W-19`).** None of
`D-101`…`D-108` can carry a code gate while it is OPEN: they are decisions, not
code, so there is nothing in `src/` to check against an undecided ruling. The
gate is therefore at the release boundary.
`.github/scripts/release-gate.sh`, wired into CI on `refs/tags/v1.*`, parses
this file and **fails the release while any `D-1xx` reads `Status: OPEN`**.
Verified in both directions: it currently refuses (7 open), and passes when the
statuses are settled. `v0.x` tags are unaffected, so pre-1.0 releases can ship
while these remain open — which is exactly the intended latitude.

## D-101 — Key bound: `Hash + Eq`
- **Status:** **SETTLED (2026-09-09)** — implemented by `W-10`.
- **Ruling:** the default for every operator that compares elements or keys is
  **`Eq + Hash`** (plus `Clone` where an order-preserving index is needed). The
  `PartialEq` linear-scan implementations remain, under explicit
  `*_partial_eq` names, for types that cannot implement `Hash`.
- **Why:** `PartialEq` does not guarantee reflexivity, and every one of these
  algorithms needs an equivalence relation. The visible consequence was a
  `Lookup` that could not retrieve a key it had just inserted (`f64::NAN`).
  `Eq` is Rust's marker for exactly that property, and it is what `HashSet`,
  `HashMap` and `Itertools::unique` already require. The cost was also
  measured, not assumed: the `PartialEq` versions are O(n²) and cross 100 ms
  around n≈17,000–30,000 (`AUDIT.md` §4.3).
- **What this cost, stated plainly:** `f64` keys no longer work on the default
  methods — `vec![1.0, 2.0, 1.0].distinct()` does not compile. `ROADMAP.md`
  previously defended `PartialEq` as serving float keys, but that rationale did
  not survive contact with the code: `order_by` binds `K: Ord`, so the same
  float audience could not sort anyway. The escape hatches keep the capability
  under a name that says what it costs.
- **Forbids:** adding a new comparing operator whose default is `PartialEq`.
  Reintroducing `*_hashed`-style twins (`D-206`).
- **Enforced by:** **IMPLEMENTED** — the type system. Every default binds
  `Eq + Hash`; `tests/edge_cases.rs` covers the float path through
  `distinct_partial_eq`, and `tests/linq_tests.rs` carries a `*_partial_eq`
  test per operator.

## D-102 — The v2 translation boundary: compile error, explicit opt-in
- **Status:** **SETTLED (2026-09-10).**
- **Ruling:** an operator the SQL path cannot translate is a **compile error**,
  not a silent fallback and not a runtime throw. The SQL-backed type exposes
  only the translatable subset; `.to_memory()` consumes it, executes what it
  has, and returns a plain `Iterator` on which the full `LinqExt` surface
  reappears. Crossing the boundary is one visible token, greppable in review.
- **Why:** EF Core ran this experiment. It fell back to client evaluation before
  3.0, documented the result as "unnoticed performance issues", and changed it
  to a runtime throw in 3.0. C# cannot do better — `IQueryable<T>` exposes the
  same method set regardless of provider, so translatability is not expressible
  in its type system. **Rust's is**, and this crate proves it accidentally: the
  boundary is already 100% compile-time, because operators are trait methods on
  concrete types.
- **Forbids:** partial translation ("translate the prefix, evaluate the
  suffix") — where EF Core's own memory-leak documentation lives — and runtime
  capability flags. A provider's operator subset is its trait's method set.
- **Constrained by `B-2` (compiler-verified):** v2 must be **purely additive**.
  Widening `where_<P: FnMut(&T)->bool>` to `where_<P: IntoPredicate<T>>` breaks
  every call site with `error: implementation of 'FnMut' is not general enough`,
  and `for<'a>` does not fix it. v2 adds `where_expr`; it never re-bounds
  `where_`.
- **Enforced by:** *prose until v2 exists* — there is no boundary to gate yet.
  When there is: the SQL type must not implement `LinqExt`, and a `compile_fail`
  test must pin a non-translatable operator against it.

## D-103 — Three-valued logic: promise a stability class, not identity
- **Status:** **SETTLED (2026-09-10).**
- **Ruling:** the crate does **not** promise a query returns the same answer in
  memory and in SQL. It promises, per operator, a documented *stability class*:
  `identical`, `identical-modulo-collation`, or `provider-defined`.
- **Why:** identity is not achievable, and claiming it would be the
  silent-wrong-answer class this audit spent its length on. Measured: Rust
  `Ord for str` is byte-ordinal, C# `OrderBy` is culture-aware, PostgreSQL
  orders by the column's collation — `["a","B","c","D"]` sorts to
  `["B","D","a","c"]` here, and canonically-equal NFC `é` and NFD `e\u{301}`
  land on opposite sides of `"f"`. Nulls split worse: Rust and C#
  LINQ-to-Objects agree `None == None`, SQL says `UNKNOWN`; Rust sorts `None`
  first, PostgreSQL defaults `NULLS LAST`; `sum_` annihilates on one `None`
  where C# and SQL skip nulls. Three answers for one operator (`D-018`).
- **Forbids:** any claim that a query "gives the same result" across providers.
- **Enforced by:** *prose until v2 exists.* Then the stability class becomes a
  column in `.github/data/operator-map.tsv`, generated into the docs like every
  other per-operator fact (`D-016`).

## D-104 — Key selectors cannot borrow from owned items: accept and document
- **Status:** **SETTLED (2026-09-10)** — signature unchanged, limitation
  documented, and a real bug found and fixed on the way.
- **Ruling:** key selectors keep `Fn(&Self::Item) -> K` with `K` a free type
  parameter. A key therefore cannot borrow from an item the iterator owns.
  Iterating by reference (`.iter()`) makes `Self::Item = &T` and borrowed keys
  work with **zero clones**; that is the documented answer.
- **Scope, measured:** 23 of the 62 `LinqExt` methods take a key selector, plus
  two on `OrderedQueryable` — 25 sites. (The earlier claim that "every shipped
  method" takes one was wrong.)
- **Why not the alternatives — each compiled, not reasoned about:**
  - `for<'a> Fn(&'a T) -> K<'a>` does not express it:
    `error[E0109]: lifetime arguments are not allowed on type parameter 'K'`.
  - `for<'a> Fn(&'a T) -> &'a K` compiles but rejects every *computed* key
    (`|p| p.n` → `E0308`), and the storing operators clone anyway.
  - A GAT key trait **does** compile on 1.75 — so the MSRV is not what kills it.
    Coherence is: restoring closure syntax needs one blanket impl per key shape
    and they collide (`E0119`). Storing operators remain impossible
    (`E0597`/`E0505`). Rejecting it "because 1.75 can't" would have been a wrong
    reason for a right answer.
  - `Cow<'a, K>` is the one alternative that genuinely works for non-storing
    operators, and is rejected on cost: every owned key needs a turbofish, since
    `ToOwned` is not invertible (`E0283`).
  - Every alternative fails on the **storing** operators — `group_by_key`,
    `into_lookup`, `into_hashmap`, `aggregate_by` — because `Grouping` must own
    a key derived from the item being moved into it. The option costing a total
    API break buys nothing where the pain is worst.
- **`std` does the same.** `vec.into_iter().max_by_key(|p| &p.dept)` and
  `vec.sort_by_key(|p| &p.dept)` emit a byte-identical
  `error: lifetime may not live long enough`. A user who hits this has hit it in
  `sort_by_key` already.
- **Survives the v2 thesis, and fits it best.** A key selector on a
  *translatable* query names a column, and `B-2` means the SQL path gets a
  second method taking an expression value, never a widened closure bound. A
  column reference has nothing to borrow from, so a lifetime-parameterised key
  type would be dead weight there — and would compound `D-106`, which needs
  these operators to return nameable types.
- **The precondition that had to be fixed first.** The mitigation this ruling
  rests on *did not work*. `OrderedQueryable`'s boxed comparator carried an
  elided `'static`, which propagated `Self::Item: 'static` onto all eight
  ordering methods — so `people.iter().order_by(|p| &p.dept)`, sorting a view of
  a collection you still own, failed with
  `error[E0597]: 'people' does not live long enough`. Introduced by `W-14`, and
  invisible because every test and doctest sorted an owned `Vec` of `'static`
  elements. Fixed by parameterising the lifetime (`OrderedQueryable<'a, T>`).
  Documenting the mitigation without fixing this would have been precisely the
  comment-and-code-disagree failure this repo is built against.
- **Enforced by:** **IMPLEMENTED** —
  `tests/adaptor_contracts.rs::ordering_works_over_a_borrowed_collection` and
  `::ordering_accepts_a_non_static_comparator` exercise the whole ordering
  family over borrowed data, including a borrowed key and a non-`'static`
  comparator.

## D-105 — Closure bounds: `FnMut`, matching `std`
- **Status:** **SETTLED (2026-09-10)** — this **reverses** the recommendation
  previously on file, which said to bind `Fn` everywhere.
- **Ruling:** the in-memory surface binds **`FnMut`**, as `std::iter` does. The
  ordering operators keep `Fn + 'a` because they *box* their comparator — an
  implementation constraint, documented, not a preference.
- **Why the reversal:** the case for `Fn` was "a translator needs purity, and
  narrowing later is breaking". The first half is true; the second is moot,
  because **`B-2` is compiler-verified** — v2 cannot re-bound these methods at
  all, it must add new ones. The scenario `Fn` insured against cannot happen, so
  paying for it means being less capable than `std` for nothing, which `D-005`
  forbids.
- **Measured:** relaxing the boxed operators to `FnMut` compiles and passes the
  full suite, but buys nothing — a stateful closure still fails there on the
  lifetime bound, not on `Fn`. `where_` with `FnMut` *does* accept a stateful
  predicate. Documenting the real constraint beats hiding it behind a
  uniform-looking bound.
- **Enforced by:** the compiler. Binding `Fn` on a non-boxing operator is a
  deliberate act a reviewer can see.

## D-106 — Named return types for anything the seam must reach
- **Status:** OPEN. **Recommended: return named types** from `join`, `group_join`,
  `group_by` and any future relational operator.
- **Enforced by:** nothing yet — see the shared gate for this section (`W-19`).
- **Why it matters:** `B-1`, compiler-verified — the eight `-> impl Iterator` sites
  in trait position permanently seal those operators against any future trait
  (`error[E0599]: no method named 'sql' found for opaque type`), and they are the
  three operators the v2 thesis needs most. The same sites pin the MSRV at exactly
  1.75 with zero headroom.

## D-107 — Sealing: not needed; the blanket impl already seals
- **Status:** **SETTLED (2026-09-10)** — no code change.
- **Ruling:** `LinqExt` needs no sealed supertrait. `impl<I: Iterator> LinqExt
  for I {}` already makes a downstream impl impossible **for any type**.
  Verified: a downstream crate with its own `Iterator` writing
  `impl LinqExt for MyIter {}` gets `error[E0119]: conflicting implementations
  of trait 'LinqExt' for type 'MyIter'`, citing `impl<I> LinqExt for I` here.
- **Why record a no-op:** "seal the public traits" reads like outstanding work,
  and someone would eventually add the machinery. The `ThenBy` half *was* real —
  a public unsealed trait with one impl — and `W-14` removed it by deleting the
  trait.
- **Enforced by:** the coherence rules. Nothing to add.

## D-108 — `to_` vs `into_`: renamed; `#[must_use]` shipped
- **Status:** **SETTLED (2026-09-10)** — both halves done.
- **Ruling:** consuming conversions take `into_`. `to_lookup` → `into_lookup`,
  `to_hashmap` → `into_hashmap`. (`to_vec`/`to_hashset` were cut by `D-019`;
  they were `collect()` in a different spelling.)
- **Why:** Rust reserves `to_` for borrow-to-owned and `into_` for
  owned-to-owned; both of these consume `self`. Clippy does not catch it, which
  is why it needed a decision rather than a lint.
- **`#[must_use]`:** shipped in `W-17` — 52 annotations, verified downstream at
  11 discarded results → 11 warnings, with side-effect methods correctly exempt.
- **Enforced by:** the compiler for `#[must_use]`; naming is a review matter,
  and the surface is now consistent.

# DO-NOT-BUILD

Cite these IDs when the idea comes back. Full reasoning in `AUDIT.md` §9.

| ID | Do not build | One-line reason |
|---|---|---|
| `D-201` | A `linq!` / `from…where…select` comprehension macro | Three Rust crates tried it (2017, 2019, 2021); **all three are dead**, while the extension-trait crate in the space has 1.48 B downloads |
| `D-202` | Pluggable `IEqualityComparer`/`IComparer` per call (44 C# overloads) | Rust expresses this with traits and newtypes. The legitimate subset is `*_by` variants |
| `D-203` | Full C# semantic fidelity | Would make the crate worse Rust to match a foreign contract; culture-aware collation needs an ICU dependency `D-001` forbids. See `D-017` |
| `D-204` | `TryInto`-based `cast::<U>()` that panics on the first bad element | A fallible data conversion that panics, with no `Result` alternative. C# needs it for runtime downcasting; Rust has none |
| `D-205` | Two vocabularies for one concept in one crate | `LinqExt::where_` and `sql::filter` mean the same thing and share no value. The SQL builder either becomes the seam (`D-002`) or its own crate |
| `D-206` | A second implementation of every operator (`*_hashed` twins) | Doubles the surface to dodge a breaking change and leaves the slow version as the default. **Resolved by `W-10`**: the hash implementation took the plain name and the `PartialEq` one became `*_partial_eq`. The *count* of methods is unchanged — what changed is which one a caller reaches for by default, and that the slow one now has to be asked for by name |
| `D-207` | Benchmarks via nightly `test::Bencher`, or `criterion` in the main crate | `ROADMAP.md` deferred benchmarks for this reason and was right. The way out is a workspace `benches/` member |
| `D-208` | `async` before the sync seam exists | Diesel's async story is a separate 0.x crate in a personal repo with a thread-pool SQLite shim. Make the execution seam pluggable, not forked. See `D-006` |
