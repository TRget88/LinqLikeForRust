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

## D-002 — SQL translation: the seam is built
- **Status:** SETTLED (2026-09-09); **implemented 2026-09-10 by `W-20`.**
- **What shipped:** `linq_rs_sql::rows` — one query value with two interpreters.
  `query::<Employee>().filter(employees::salary.gt(100_000))` renders to
  `SELECT * FROM employees WHERE (salary > ?)` with bound parameters, *and*
  evaluates over a `&[Employee]` via `.to_memory()`. No Rust library offered
  this; the audit verified the gap from primary sources.
- **Three design findings, each compiled rather than argued:**
  - **The seam lives in `linq_rs_sql`, and a third crate is impossible.**
    Evaluating a predicate tree needs to read the node fields, which are
    `pub(crate)`; from outside it is `error[E0616]: field 'left' of struct 'Gt'
    is private`. A third crate would have forced 12 node structs' fields public.
    Rendering-only would have worked from outside — the asymmetry is the whole
    argument. The seam needs **no** dependency on `linq_rs`: `LinqExt` is
    blanket-implemented, so `.to_memory()` returning a named `Iterator` is
    enough.
  - **The predicate must be a type, not runtime data.** Measured across five
    designs: a `Vec<Step>` interpreter, `Box<dyn Iterator>`, and named-adaptor
    hybrids all land **2.3–13× slower** than `where_().select()`. A typed nest
    lands at **0.96–1.14×**, allocates identically to `filter().map()`, and
    streams to 10M rows. Verified end to end: 80.7 ms for the seam vs 86.6 ms
    for `linq_rs` and 94.2 ms for hand-written `filter().map()`, same checksum.
  - **The prototype's dynamic `Val` enum is unnecessary.** `Repr` is a type
    function from the column's declared SQL type to its Rust type, so a
    `Text`-vs-integer comparison is unrepresentable rather than silently
    `false`: `error[E0053]: method 'eval' has an incompatible type for trait`.
- **The prototype's three gaps are all closed:** it materialised (now streams,
  at parity), it lost type safety (now a compile error), and it needed two
  vocabularies (now one `entity!` line per column).
- **`linq_rs` is untouched.** Byte-identical; someone who never wants SQL never
  sees any of this.
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

## D-010 — Stable Rust only; MSRV 1.65, declared **and actually exercised**
- **Status:** SETTLED (2026-09-09)
- **Ruling:** No nightly. `rust-version = "1.65"`. CI builds **and runs
  doctests** on exactly 1.65.
- **Lowered from 1.75 by `D-106` (2026-09-10).** The floor was never a
  considered choice — it was wherever the 23 RPITIT sites happened to put it.
  Converting them to named types for unrelated reasons dropped it ten releases.
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
- **Superseded in part by `D-023`.** The `Forbids` clause above addressed the
  *tracked* lockfile and stopped there. That was the wrong fix; see `D-023`.
- **Enforced by:** **IMPLEMENTED** — the `msrv` CI job pins `1.65` (asserting
  `rustc --version` first) and runs `test-count-floor.sh`, which covers both
  `--all-targets` and `--doc`; the latter is the bucket a bare
  `cargo test --all-targets` would have skipped. `msrv-tarball.sh` additionally
  builds the packaged tarballs — verified on a genuine 1.65.0 toolchain. The CI job now also asserts `rustc --version` is 1.75.x
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

## D-020 — The SQL builder ships as a sibling crate
- **Status:** SETTLED (2026-09-10) — done.
- **Ruling:** the typed SQL query builder lives in **`linq_rs_sql`**, a separate
  crate in the same workspace. Neither crate depends on the other.
- **Why not inside `linq_rs`:** `D-205` forbids two vocabularies for one
  concept in a single crate, and that is exactly what it would have been —
  `LinqExt::where_` and `sql::filter` mean the same thing with no value passing
  between them. That was `AUDIT.md`'s top finding (**A-1**).
- **Why not left on a branch:** because the value is real and holding it cost
  users something. It is a compile-time-checked column model with bound
  parameters throughout — a genuine improvement over SQL in string literals, and
  the readability argument that motivates EF in the first place. Keeping working
  code unreleased to protect a thesis that is not built yet is a bad trade. Two
  crates with one vocabulary each is a better thing to hand someone than one
  crate with two.
- **Not a competitor to `D-002`.** `linq_rs_sql` is intended to become the
  *rendering backend* for the two-interpreter seam, not an alternative to it.
  When `W-20` lands, a `Query<T>` renders through this crate.
- **Forbids:** either crate depending on the other, and either gaining a
  dependency. Both are dependency-free and stay that way.
- **Enforced by:** **IMPLEMENTED** — `packaging-gate.sh` asserts the sibling's
  licence matches, its `repository` is set, and its dependency list is empty;
  `test-count-floor.sh` now counts `--workspace`, so the sibling's tests are
  gated rather than silently skipped; CI builds, clippies and documents the
  whole workspace.

## D-022 — the sibling crate is a separate legal and testable artifact
- **Status:** SETTLED (2026-09-10) — fixed and gated.
- **Found by inspecting the tarball rather than the repo.** `cargo package --list
  -p linq_rs_sql` showed three defects that every check up to that point had
  passed:
  1. It declared `license = "MIT OR Apache-2.0"` and **shipped neither text.**
     `packaging-gate.sh` did check the licence — it read the manifest *field*,
     and a field is not a file. The root tarball's file check existed and was
     simply never extended to the sibling when `D-020` split it out.
  2. Its README carried three `../` links (`../LICENSE-MIT`, `../LICENSE-APACHE`,
     `../README.md`). Those resolve in the git tree and 404 on crates.io and
     docs.rs, where the tarball has no parent.
  3. Its dev-dependency on `linq_rs` was path-only, and cargo **strips**
     path-only dev-deps from the published manifest — so `tests/seam.rs` would
     not compile from the tarball, defeating `D-013`'s reason for shipping
     `tests/` at all (the executed-test count checkable from the artifact, not
     just claimed in a README).
- **Ruling:** the sibling carries its own byte-identical licence texts; its
  README uses no parent-relative links.
- **The dev-dependency clause is superseded by `D-024`.** This decision made it
  `{ path = "..", version = "0.2" }` so the shipped tests could resolve. `D-024`
  removes the dev-dependency entirely instead, which solves the same problem
  without coupling the two crates' publish order.
- **Consequence, accepted:** the version pin imposes a publish **order** —
  `linq_rs 0.2.0` must be live on crates.io before `linq_rs_sql` can be packaged
  or published at all. That order is the natural one anyway. Verified by probe
  that a versioned dev-dep does survive into the published manifest as
  `[dev-dependencies.linq_rs] version = "0.2"`; `--list` still works offline, so
  the gate below runs before either crate is published.
- **This does not weaken `D-020`.** Dev-dependencies never enter a consumer's
  graph; the gate asserts zero *runtime* dependencies, unchanged.
- **The link defect had a second instance, in the crate about to publish.** The
  root README linked `[`linq_rs_sql`](linq_rs_sql/)` — a workspace member, which
  is *not* in the root tarball, so that link 404s on crates.io and docs.rs too.
  The gate therefore checks link targets against each package's own
  `cargo package --list` rather than pattern-matching `../`: a relative link is
  valid iff the tarball actually ships it.
- **Enforced by:** `packaging-gate.sh` §"sibling crate tarball" — asserts the
  sibling's tarball ships `LICENSE-MIT`, `LICENSE-APACHE`, `README.md` and
  `tests/`, that both licence texts are byte-identical to the workspace copies,
  and (for **both** READMEs) that every relative link resolves to something in
  that package's own tarball. Every check verified to fail when the defect is
  reintroduced, not just to pass today.

## D-032 — the dependency rule, stated by the owner
- **Status:** SETTLED (2026-09-11) — implemented; **supersedes `D-031`**.
- **Ruling, verbatim from the owner:**
  - `linq_rs` — **no dependencies.**
  - `linq_rs_sql` — **only `linq_rs`.**
  - **Nothing else is acceptable.**
- **This retires `linq_rs_sqlite`.** `D-031` reasoned that a provider in its own
  crate was acceptable because it kept the two core crates clean, and cited EF
  Core's core-plus-provider split as precedent. That reasoning answered a question
  the owner had not asked. The rule is not "the core crates stay clean" — it is
  that **the project takes no third-party dependency at all**, and one driver
  dependency pulled **24 crates** into the resolved graph:
  `rusqlite → libsqlite3-sys → cc, pkg-config, vcpkg, syn, quote, proc-macro2, …`.
- **Nothing structural was lost.** `ColumnSet`, `RowSource`, `FromRow`, `LoadOpt`
  and `LoadField` all live in `linq_rs_sql` and have no dependencies (`D-029`).
  The deleted crate was only the rusqlite glue, and that glue is what a user was
  always going to write. It is preserved verbatim, with everything it was verified
  to do, in `docs/DRIVER_ADAPTER.md` — about 36 lines.
- **`linq_rs_sql` may depend on `linq_rs`; it does not.** The permission is
  recorded because it changes what is possible: `seam-tests` exists (`D-024`) only
  to hold cross-crate tests without that dependency, so it could now be collapsed.
  Not done here — a permission is not an instruction.
- **Enforced by:** `packaging-gate.sh`, on the **resolved** graph rather than the
  manifests, because a manifest lists direct dependencies and a transitive one is
  still a dependency. It asserts the graph contains no crate outside
  `{linq_rs, linq_rs_sql, seam-tests}`, and per-package that `linq_rs` has none and
  `linq_rs_sql` has at most `linq_rs`. Verified failing on a third-party
  dependency (`libc`), and on an inverted layering (`linq_rs` depending on
  `linq_rs_sql`).
- **Note on the check itself:** the first version compared with
  `printf '%s' "$got" | grep -qE '^$'`, which fails for the ALLOWED empty value —
  `printf` of an empty string emits zero lines and `grep` needs one. A rule whose
  check rejects the compliant state is worse than no check. It is a shell string
  comparison now.

## D-031 — the provider is its own crate, not a feature flag
- **Status:** **SUPERSEDED by `D-032`** (2026-09-11). The crate it created,
  `linq_rs_sqlite`, has been deleted: it took a third-party dependency, which the
  owner's rule does not permit anywhere in this project. The entry is kept because
  the driver-seam findings inside it are still correct and still in force.
- **Ruling:** executing queries lives in a **third crate** that depends on
  `linq_rs_sql` and one driver. `linq_rs` and `linq_rs_sql` keep zero
  dependencies of any kind.
- **Why not a `rusqlite` feature on `linq_rs_sql`.** An optional dependency is
  still a dependency: it appears in the manifest, it ends the "zero dependencies"
  claim as written, and it lands in the lockfile of everyone who only wanted to
  build SQL strings. Splitting is what *lets* `D-024` stay true.
- **It also happens to be EF's own architecture** — EF Core plus a separate
  provider package per database. That was not the reason, but it is a good sign
  the shape is right.
- **Surface: `fetch`, `fetch_one`, `count`.** Deliberately small, and it does not
  hide SQLite. No connection pool, no transaction wrapper, no `DbContext`: the
  caller owns the `rusqlite::Connection` and `Sqlite<'c>` borrows it. Anything
  rusqlite does better stays rusqlite's job.
- **`count` wraps rather than rewrites:** `SELECT COUNT(*) FROM (<sql>)`. A naive
  `SELECT COUNT(*)` rewrite drops `LIMIT` and over-reports. Tested.
- **This is the first place the dialect assumption is written down.**
  `linq_rs_sql` emits `?` placeholders for every literal, which SQLite and MySQL
  accept and **PostgreSQL rejects** — it wants `$1`, `$2`. So the crate has been
  a SQLite/MySQL dialect with no way to say so. A real dialect layer belongs in
  `linq_rs_sql` and is still open; building one provider first is how its shape
  gets discovered instead of guessed, which is the same reason `D-026` came
  before `D-027`.
- **The gate had to be loosened, so it was also tightened.** `packaging-gate.sh`
  now permits dependencies for the provider and nothing else, and additionally
  asserts: the publishable set is exactly the three crates; the provider depends
  on exactly `linq_rs_sql` + `rusqlite`; and **no core crate depends on the
  provider**, because an inverted layering would make the split pointless. All
  three verified failing when violated.
- **Found while writing the MSRV gate, and worth recording separately:**
  `cargo package` **normalises** the manifest, so a dependency is emitted as
  `[dependencies.name]` and never as `[dependencies]` + `name = …`. My detection
  matched the repo's shape, so it silently never fired and the provider's tarball
  went unbuilt while the gate reported PASS. That is the third instance this
  session of grading the repo's shape instead of the artifact's — `D-023`'s
  lesson is apparently easy to re-learn. The gate now patches the unpublished
  sibling to its extracted tarball and resolves the driver from the registry,
  and says so in its output rather than skipping silently.
- **A count in this entry was wrong when written.** It said "four sites"; it is
  nine. I took it from a `grep | head -4`, which is the truncation trap that has
  its own note in this ledger. Counts are now omitted here rather than corrected
  — a number in prose drifts, and `D-016` is the rule that numbers which matter
  get generated.
- **Forbids:** a driver dependency or feature on `linq_rs` or `linq_rs_sql`; a
  core crate depending on the provider; a second driver in this crate.
- **Enforced by:** *nothing — the named target is deleted.* This said
  `linq_rs_sqlite/tests/roundtrip.rs`, which went with the crate under `D-032`.
  What those 12 tests proved is recorded in `docs/DRIVER_ADAPTER.md`; the
  `linq_rs_sql`-side behaviour they exercised is still covered by
  `linq_rs_sql/tests/{from_row,nullable,boxed}.rs`. Left visible rather than
  quietly deleted, because an `Enforced by` line pointing at nothing is precisely
  what the field exists to prevent. Historically it was:
  `linq_rs_sqlite/tests/roundtrip.rs` — 12 tests against a real
  in-memory SQLite, including the database and the in-memory interpreter
  agreeing on one query value, three-valued logic matching the database,
  conditional composition, hostile input staying in `params`, `count` respecting
  `LIMIT`, a NULL in a non-nullable column naming column and row, the `D-029`
  bool refusal, join ambiguity, and a missing column failing identically on
  empty and populated data. Plus the packaging and MSRV gates above.

## D-030 — `to_sql()` names the columns
- **Status:** SETTLED (2026-09-11) — implemented. `linq_rs_sql 0.3.0`; the
  emitted SQL changes.
- **Ruling:** `Rows::to_sql` and `BoxedRows::to_sql` emit the entity's declared
  columns instead of `*`. `Entity` gains `ALL_COLUMNS`, supplied by `entity!` on
  both arms.
- **Why, given `D-029` already made `*` safe.** Two reasons, and the first is the
  one that matters:
  1. **It is the precondition for projection.** A query cannot select a subset of
     columns while its SELECT list is a wildcard. `.select()` — LINQ's
     `Select`/`new { … }` — is unreachable without this.
  2. Defence in depth. Naming the columns moves order from the *database's*
     control to the *query's*, and resolution then provably returns the identity
     permutation. By-name reading still earns its place for what the crate does
     not control: a hand-written query, a join, a view, a driver that reorders.
- **`ALL_COLUMNS` is required with no default.** A default of `&[]` would make
  `to_sql` silently fall back to `SELECT *` for an entity that forgot it — a
  silent wrong default in place of a compile error, which is the `D-025`
  category. Breaking for hand-written `Entity` impls, deliberately.
- **Named `ALL_COLUMNS`, not `COLUMNS`.** `FromRow::COLUMNS` already exists, both
  would be in scope on the same type, and `Emp::COLUMNS` was `E0034` — verified
  while writing this. That is precisely the defect that disqualified
  `linq_rs 0.1.0`, where `LinqExt::skip` shadowed `Iterator::skip`. They are also
  not the same concept: one is what to SELECT, the other what to READ, and a
  future tuple projection will have the second without the first.
- **The lower-level `Query` builder still emits `*`, on purpose.** It has no
  `Entity`, so there is no declared list for it to name; a caller who wants one
  passes `.select(...)`. So `employees::table()` gives `SELECT *` while
  `query::<Employee>()` gives the named list. Pinned by a test so the asymmetry
  is deliberate rather than discovered.
- **Identifiers are emitted unquoted, unchanged.** A generated column list needs
  quoting for a column named `order`, and there is no spelling valid on SQLite,
  PostgreSQL and MySQL alike (`"order"` fails MySQL's default mode, backticks
  fail PostgreSQL). But this is **not a new problem and not this decision's to
  solve**: the crate already emits `WHERE (order > ?)` unquoted, which real
  SQLite already rejects — verified. `*` was merely immune. Quoting wants one
  ruling covering every emission site, alongside the dialect layer `D-025`
  deferred, not a special case here. The failure is loud (a parse error), which
  `D-025`'s rule permits.
- **Cost, measured:** 25 assertion updates across six test files plus two
  doctests, applied by re-running the suite and taking the actual output rather
  than by hand. One of them, `predicates_over_the_querys_own_table_...`, had to
  be reverted by hand: it asserts all three builders, and two of them correctly
  still emit `*`.
- **Forbids:** giving `ALL_COLUMNS` a default; letting the typed and erased forms
  emit different SELECT lists.
- **Enforced by:** `tests/boxed.rs::erased_sql_is_byte_identical_to_typed_sql`
  (which caught the erased form still emitting `*` during this change),
  `tests/from_row.rs::a_query_this_crate_generated_resolves_to_the_identity`, and
  `tests/from_row.rs::the_entity_free_query_builder_still_emits_star`.

## D-029 — row materialization: by name, and a refused coercion
- **Status:** SETTLED (2026-09-11) — implemented.
- `entity!` knew every field, column and SQL type, and generated only struct →
  column *readers*. A query result could never become a `Vec<Employee>`, so
  nothing could ever execute a query usefully. This is the reverse direction.
- **Three competing designs were prototyped, each proven against a real
  in-memory SQLite. All three were killed by adversarial review.** The
  architecture below is what survived; the trees did not.

### Columns are matched by NAME, never by position
`Rows::to_sql` emits `SELECT *`, and SQLite and PostgreSQL expand `*` in
**table-declaration order** — which this crate does not know, cannot pin, and
which changes under an already-compiled binary. SQLite cannot reorder a column
in place, so the documented migration is a table rebuild, which is exactly where
declaration order drifts. Verified against real SQLite:
```
v1 schema:  SELECT * -> Employee { id: 1, name: "ada",         dept: "engineering" }
v2 schema:  SELECT * -> Employee { id: 1, name: "engineering", dept: "ada" }
```
Same code, same entity, same types, so **no error is possible**. Positional
decoding converts the database's choice of column order into a plausible wrong
value — the `D-025` category. Two of the three spikes reproduced it
independently. By-name makes the class unreachable: a table physically ordered
`dept, active, id, nick, salary, name` resolves to `[2,5,0,4,3,1]` and
materializes correct values.
- **The survey's rule:** the SELECT list may be `*` **iff** you read by name.
  Nobody who decodes positionally emits it; Diesel makes `*` *unrepresentable*
  in a data-returning select (`star` has `type SqlType = NotSelectable`).

### Booleans accept only 0 and 1
Every other SQLite binding treats non-zero as true. Doing so **breaks the
seam**, measured: for `active INTEGER` holding `1, 0, -1, 2`, SQL
`WHERE active = ?` bound `true` renders `active = 1` and keeps `[1]`, while a
permissive reader calls `-1` and `2` true and keeps `[1, 3, 4]`. One query value,
two answers. Refusing the coercion makes the row fail loudly instead.
Narrowing is likewise checked, never `as`.

### `ColumnSet` is separate from `RowSource`
Resolution must work with no row in hand. Fold the traits together and the same
query against the same schema returns `Ok(vec![])` on empty data and
`Err(NoSuchColumn)` on populated data — two verdicts for one schema, decided by
whether rows happened to exist. Found by building it, not by inspection.

### One required driver method
`value_at(at, column) -> SqlValueRef<'a>`, and nothing else. The five-typed-
accessor shape measured 101 lines per adapter because each re-implemented type
checking and worded its own mismatch message; collapsing to one took it to ~36,
and a new SQL type no longer breaks every adapter. **Name matching, ambiguity
detection, type checking, NULL rules and every message live in the crate** — an
earlier version delegated matching, and rusqlite's ASCII case folding then gave
a different verdict from a strict adapter for the same schema.

### A duplicated name is ambiguous, not first-wins
`SELECT * FROM a JOIN b` yields two `id` columns. Taking the first makes the
second table's data silently unreachable, and joins are the main reason anyone
wants materialization at all. `AmbiguousColumn` names both positions.

### `Layout<R>` is tied to its shape
A layout resolved for one shape, fed to another of the same arity whose columns
share types, produced `{ id: 42, n: 10 }` where the truth was `id: 10, n: 42`.
The `PhantomData<fn() -> R>` parameter makes that a compile error.

### `Nullable<S>` is one lift, and `FromRow` is opt-out
`LoadOpt` is implemented only for base markers and keeps NULL as `None`;
`LoadField` applies the NOT NULL rule in two blanket impls, so `Nullable<S>`
needs no duplicate set. Generation is opt-out via `entity! { … } no_from_row`,
because a borrowed field or a non-column field cannot have a generated impl and
both are legal today.

- **Forbids:** positional decoding; treating non-zero as `true`; `as` casts in
  decoding; resolving a name to the first of several matches; delegating name
  matching or error wording to an adapter.
- **Enforced by:** 17 unit tests in `linq_rs_sql/src/from_row.rs` and 12 in
  `linq_rs_sql/tests/from_row.rs`, including a reordered result set, two
  same-typed adjacent columns not swapping, the bool refusal, join ambiguity,
  and a missing column failing identically on empty and populated result sets.
  Separately proven end-to-end against a real SQLite through a 36-line rusqlite
  adapter.
- **Still open, and next:** `to_sql()` emits `SELECT *`. By-name reading makes
  that *safe*, not *good* — naming the columns moves order from the database's
  control to the query's and is the precondition for `.select()` projection. It
  costs a `0.3.0` behavioural break and re-opens the dialect question, because a
  generated column list must quote identifiers and `"order"` versus `` `order` ``
  has no spelling valid on SQLite, PostgreSQL and MySQL alike.

## D-028 — a predicate's columns must belong to the table being queried
- **Status:** SETTLED (2026-09-10) — fixed and gated.
- **The defect.** A query over one table could be filtered by another table's
  column, and the SQL builder emitted it:
  ```rust
  query::<Employee>().filter(departments::budget.gt(100i64)).to_sql()
  // SELECT * FROM employees WHERE (budget > ?)
  ```
  That SQL fails at runtime with *no such column* — or worse, silently matches
  a same-named column that means something else. Present in **both** builders,
  found while assessing the crate against EF rather than by any gate.
- **It was backwards.** `to_memory()` always rejected it, because
  `Eval<'_, Row>` is only implemented for the row's own columns. `to_sql()` did
  not. The production path had the weaker guarantee.
- **Ruling:** a marker trait, `BelongsTo<T>` — an expression every column of
  which belongs to table `T`. `Query::filter`, `Rows::filter`, `Rows::to_sql`
  and `BoxedRows::filter` all require it.
  - **Literals belong to every table**, because they have no columns. That is
    what keeps `salary.gt(100)` legal.
  - Combinators belong to `T` exactly when every operand does.
  - **Columns get their impl from the `table!` macro, not from a blanket impl
    over `Column`.** A blanket `impl<C: Column> BelongsTo<C::Table> for C` would
    overlap the literal impls, because Rust has no negative reasoning and cannot
    prove `i64: !Column`. Generating per column sidesteps coherence entirely.
  - `Fragment` and `Box<dyn DynPred<Row>>` accept any table: both are built from
    a predicate that already passed the check, so the guarantee was established
    upstream rather than discarded.
- **The error names both tables**, which is most of the value:
  ```
  error[E0277]: the trait bound `budget: BelongsTo<employees::Marker>` is not satisfied
  help: the trait `BelongsTo<employees::Marker>` is not implemented for `budget`
        but trait `BelongsTo<departments::Marker>` is implemented for it
  ```
- **Known wart:** a foreign column reports twice on the seam — once at
  `.filter()` and once at `.to_sql()`, since both carry the bound. The
  `.filter()` span comes first and is the right one. Not worth relaxing
  `to_sql`'s bound to silence.
- **Forbids:** any new `filter`-shaped entry point that does not carry
  `BelongsTo`.
- **Enforced by:** two `compile_fail` doctests on `BelongsTo` (the seam and the
  SQL-only builder) plus
  `linq_rs_sql/tests/sql_phase1.rs::predicates_over_the_querys_own_table_still_compile_everywhere`,
  which pins the side that must keep working across all three builders — own
  columns, literals and combinators.

## D-027 — type erasure for dynamic composition, sealed and three-valued
- **Status:** SETTLED (2026-09-10) — implemented.
- **The problem.** `Rows<Row, P, O>` parameterises the predicate by *type*, so
  every `.filter()` returns a different type. That is the right default — it is
  what makes a `Text` column compared to an integer a build error — but it makes
  the way applications actually build queries impossible:
  ```rust
  let mut q = query::<Employee>();
  if want_eng { q = q.filter(employees::dept.eq("eng")); }   // E0308
  ```
  No conditional filters, no query in a struct field, no query returned from a
  function, no `Vec` of queries.
- **Ruling:** Diesel's answer — keep the typed form as the default and add
  `into_boxed()` / `boxed_query()`, an erased form whose type does not move as
  clauses are added. Chosen over a runtime AST, which was prototyped and
  measured at **2.3×–5.0× slower** in memory and which made a type-mismatched
  comparison *representable* through public API again.
- **The type check survives erasure completely**, and this is the load-bearing
  fact: the check fires at the `.gt()` call, before the box. Erasure cannot lose
  what was already proven.
- **Erased ONCE, not twice.** The obvious erasure — a `Vec<Box<dyn Fn(&Row) ->
  bool>>` for memory and a separate list for SQL — erases the query into two
  independent structures that can silently disagree. `DynPred` carries *both*
  halves behind one trait object, and its blanket impl is keyed on the identical
  bounds `to_memory` already requires, so nothing can be boxed for one
  interpreter and not the other.
- **Sealed, and this is not optional.** The first prototype left `DynPred`
  public and unsealed. A hand-written impl could then make SQL select every row
  and memory select none, from the same value, through entirely safe API — a
  fresh instance of `D-025`. `mod sealed` makes the supertrait unnameable
  downstream; the seal costs legitimate users nothing because its blanket impl
  is keyed on exactly the bounds `DynPred`'s own blanket impl uses. Verified: a
  hostile impl fails with
  `` error[E0277]: the trait bound `Evil: sealed::Sealed<T>` is not satisfied ``.
- **Three-valued, because of `D-026`.** `eval_row` returns `Option<bool>`, not
  `bool`. The collapse to two values distributes over `AND` and `OR` but **not**
  over `NOT` — `is_true(NOT NULL)` is `false` while `!is_true(NULL)` is `true` —
  and a boxed predicate is a first-class expression that can be fed back into
  `not(..)`. Collapsing inside the box would therefore be wrong the moment the
  erased form was negated. A boxed predicate has `SqlType = Nullable<Boolean>`
  even when its contents cannot be null: widening is sound, and it keeps one
  erased type so a `Vec<Box<dyn DynPred<_>>>` can hold both kinds.
  **This is why `D-026` was built first** — building erasure on the
  non-nullable shape would have meant rewriting `DynPred`'s signature.
- **Clauses are kept flat**, not folded into a nested `And` tree: re-boxing per
  `.filter()` makes the nested form a *dependent* chain of indirect calls per
  row. Measured over 1M rows, flat is ~20% faster at three clauses and ~14%
  slower at one.
- **Ordering goes through one code path.** `order_part` was extracted from
  `Rows::push_order` so the erased form builds order parts with the same
  function; two copies would be two chances for the forms to sort differently.
- **Forbids:** unsealing `DynPred`; erasing to `bool`; a second erasure path
  that carries only one interpreter.
- **Enforced by:** `linq_rs_sql/tests/boxed.rs` and
  `linq_rs_sql/tests/boxed_adversarial.rs` — 35 tests, including erased SQL
  being byte-identical to the typed form across multi-clause chains, nested
  `OR`/`NOT`, multi-key `ORDER BY` and limit/offset combinations, and both
  interpreters agreeing on identical data.

## D-026 — nullable columns, and three-valued logic that agrees with SQL
- **Status:** SETTLED (2026-09-10) — implemented.
- **The problem.** There were four SQL type markers and none was nullable. An
  `Option<String>` field gave `E0608: cannot index into a value of type
  Option<String>` — a raw leak from `&row.$field[..]` that did not mention
  nullability at all. Most real tables have nullable columns, so the crate could
  not model most real schemas.
- **The trap, which is why this came before the erasure work.** SQL is
  three-valued and Rust's `Option` is not. In SQL `NULL = NULL` is `NULL`, and
  `NULL > 5` is `NULL` — neither true nor false. In Rust `None == None` is
  `true`. Implement the obvious thing and the two interpreters disagree on
  exactly the rows where it matters, which is `D-025` all over again.
- **Ruling:** `Nullable<T>` is a distinct SQL type marker, and nullability
  propagates at the **type** level:
  - `Repr<Nullable<T>>::Rust = Option<T::Rust>` — so `Option<bool>` *is* the
    three-valued type and `None` is UNKNOWN. There is no separate `Tri`.
  - `CompareWith<Rhs>::Out` is `Boolean` when neither side is nullable and
    `Nullable<Boolean>` when either is. `LogicWith` and `Negate` do the same for
    `AND`/`OR`/`NOT`. A comparison touching a nullable column therefore has a
    *different type* from one that cannot be null, and both interpreters see it.
  - `Family` maps `T` and `Nullable<T>` to the same base, so `IntOps`, `TextOps`
    and friends apply to nullable columns without being duplicated.
  - `WhereClause` is implemented for `Boolean` **and** `Nullable<Boolean>`, and
    a `WHERE` keeps a row only when the predicate is TRUE. **The collapse from
    three values to two happens there and nowhere else** — never inside the
    expression tree. That is exactly where SQL puts it.
- **The two cells that decide whether a design is right:** `NULL AND FALSE` is
  **FALSE**, and `NULL OR TRUE` is **TRUE** — an absorbing operand beats the
  unknown. A naive `Option` zip returns `None` for both. Verified against real
  SQLite, along with `NULL = NULL`, `NULL > 5`, `NOT (NULL = 1)`, `NULL AND
  TRUE` and `NULL OR FALSE`.
- **A row-set assertion cannot catch this**, which is the subtle part: `WHERE`
  drops FALSE and NULL alike, so a broken Kleene `AND` still selects the right
  rows. The truth table is therefore asserted cell by cell on the raw
  `Option<bool>`, not on surviving ids.
- **`entity!` changed shape** from per-type `@col` arms matching `$ty:ident` to a
  single generic arm over `$ty:ty`, because `Nullable<Text>` is a type and not a
  matchable token. `$ty` then resolves in the caller's scope, so the expansion
  wraps its impls in `const _: () = { use $crate::types::*; … }` — trait impls
  register globally regardless of the block they are written in. **No call site
  changed**, verified: all 184 pre-existing tests passed untouched.
- **Forbids:** collapsing three values to two anywhere but `WHERE`; adding a
  comparison whose `Out` is `Boolean` when either operand is nullable.
- **Enforced by:** `linq_rs_sql/tests/nullable.rs` — 15 tests, including
  `the_kleene_truth_table_matches_sqlite_cell_by_cell` (asserts the raw
  three-valued result, the only thing that can catch a broken absorbing case)
  and `row_sets_match_sqlite_on_the_same_four_rows` (six predicates whose
  expected rows were read out of a real SQLite).

## D-025 — a wrong answer is worse than a compile error or a panic
- **Status:** SETTLED (2026-09-10) — three defects fixed in published crates.
- Found by probing the published surface with compiled code rather than reading
  it. All three shipped. All three were **silent**: no warning, no error, a
  plausible wrong result.

### 1. `entity!` made the two interpreters disagree
`entity!` generated `row.$field as i64`. `as` is a silent lossy cast, so a field
whose Rust type did not match its declared SQL type gave **different answers in
SQL and in memory, for the same data**. Measured, with zero warnings:
```
struct M { n: f64 }   entity! { M => m { n: Integer = n } }   // n = 2.9
SQL      : SELECT * FROM m WHERE (n > ?)  params=[Integer(2)]  -> row returned (2.9 > 2)
in-memory: []                                                  -> row dropped (2.9 as i64 == 2)
```
This is the seam's central promise — one value, two interpreters, one answer —
broken by a cast. **Ruling:** the macro emits `From::from`, which exists only
for lossless widening, so the mismatch is a compile error at the `entity!` call
site: `` error[E0277]: the trait bound `i64: From<f64>` is not satisfied ``.
`i32 -> i64` and `f32 -> f64` still compile, and must.

### 2. `OFFSET` without `LIMIT` emitted SQL that does not parse
`.offset(20)` alone emitted `SELECT * FROM users OFFSET 20`. PostgreSQL accepts
a bare `OFFSET`; **SQLite and MySQL parse `OFFSET` only as part of a `LIMIT`
clause and reject it.** Verified against real SQLite:
`near "2": syntax error`. A passing test asserted the broken string as correct,
which is worse than no test — it pinned the defect. **Ruling:** emit
`LIMIT 9223372036854775807 OFFSET n`. `LIMIT -1` is the usual SQLite idiom but
PostgreSQL rejects a negative limit, so `i64::MAX` is the portable spelling of
"no limit". Verified executing on real SQLite.
- **Note the underlying gap:** this crate has no dialect layer. That is the real
  fix and it is not this one. Until then, prefer emission that is valid
  everywhere over emission that is idiomatic somewhere.

### 3. `then_by` after iteration was wrong only in release
`push_comparator` used `debug_assert!`, which compiles to nothing in release. In
a release build the comparator was silently discarded and the caller got a
plausible, wrongly-ordered result — `[("b", 2), ("a", 2)]`, secondary key gone —
while a debug build panicked. **Ruling:** `assert!`. This is a programming
error, not a runtime condition; there is no correct answer to return, so it
panics in every profile.

- **The pattern in all three:** each preferred producing *something* over
  refusing. A cast over a compile error, a string over a rejection, a skipped
  check over a panic. Prefer the loud failure; it is the cheap one.
- **Enforced by:** `tests/adaptor_contracts.rs::then_by_after_iteration_panics_in_every_profile`
  (`#[should_panic]`, so it runs in both profiles);
  `linq_rs_sql/tests/sql_phase1.rs::entity_accepts_every_lossless_field_type`
  (asserts the two interpreters agree) plus a `compile_fail` doctest on
  `entity!` for the rejection side; and
  `offset_only_still_emits_a_limit_because_offset_alone_is_not_portable`,
  renamed from `offset_only` so the assertion states the rule rather than
  restating the output.

## D-024 — zero dependencies means zero, dev-dependencies included
- **Status:** SETTLED (2026-09-10) — implemented. **Extended by `D-032`**, which
  is the binding statement: this entry only required the two core crates to be
  clean, and `D-032` forbids a third-party dependency *anywhere*. `D-032` also
  **permits** `linq_rs_sql` to depend on `linq_rs`, which this entry treats as
  forbidden — so the `seam-tests` rationale below survives only on its second
  ground (publish order), not its first.
- **Ruling:** `linq_rs` and `linq_rs_sql` each declare **no dependencies of any
  kind**. Cross-crate tests live in `seam-tests`, a workspace member with
  `publish = false` that is free to depend on both by path because nothing ever
  uploads it.
- **What it replaces.** `D-022` gave `linq_rs_sql` the dev-dependency
  `linq_rs = { path = "..", version = "0.2" }` so `tests/seam.rs` could resolve
  from the published tarball. That reasoning was sound and the conclusion was
  still wrong, for two reasons the owner named:
  1. **The crate advertises "no dependencies" and had one.** A dev-dependency
     never enters a consumer's graph, so the claim was defensible — but it was
     defensible rather than simply true, and the manifest is what people read.
  2. **It forced a publish order.** `cargo package` strips a dev-dep's `path`
     and keeps its `version`, turning it into a hard registry requirement, so
     `cargo publish -p linq_rs_sql` failed outright until `linq_rs 0.2.0` was
     live. Two independent crates were coupled by a line that served two test
     files.
- **What it cost:** nothing measurable. The dev-dependency existed for exactly
  one import, `use linq_rs::LinqExt;`, in exactly two files
  (`tests/seam.rs`, `examples/seam.rs`). Both moved verbatim. Test count is
  unchanged at 182 / 53 / 235, and the seam is still exercised on every CI run.
- **Verified:** `cargo publish --dry-run -p linq_rs_sql` now succeeds with
  `linq_rs 0.2.0` **not** on crates.io — the coupling is gone, not hidden. It
  also removed the `[patch.crates-io]` special case `msrv-tarball.sh` needed to
  build the sibling's tarball offline.
- **Forbids:** any dependency, of any kind, in either published crate; making
  `seam-tests` publishable.
- **Enforced by:** `packaging-gate.sh` — asserts each published crate has zero
  dependencies across **all** kinds (not just `kind is None`, which is what let
  the dev-dep through), and that the set of publishable packages is exactly
  `linq_rs` + `linq_rs_sql`. Both verified to fail when the dev-dep is restored
  and when `publish = false` is removed. `msrv-tarball.sh` additionally builds
  every tarball `--offline`, which cannot pass if a dependency returns.

## D-023 — the tarball is the artifact; gate that, not the repo
- **Status:** SETTLED (2026-09-10) — fixed and gated.
- **The defect.** `linq_rs 0.2.0` was one command away from publishing an
  artifact that **could not build on its own declared MSRV.** It declares
  `rust-version = "1.65"` and packaged a `Cargo.lock` at format **v4**:
  ```
  error: failed to parse lock file
  Caused by: lock file version `4` was found, but this version of Cargo does
  not understand this lock file
  ```
  Reproduced on a genuine 1.65.0 toolchain, and on 1.75.0 — which is *above*
  the floor, so 1.65 fails a fortiori.
- **Why every existing check passed.** `D-010` already forbade this and its fix
  was to **untrack** `Cargo.lock`. That hid the version from `git` and from the
  gates, not from consumers: `cargo package` **always** writes a lockfile into
  the tarball, generated by whatever toolchain publishes. Untracking guaranteed
  the shipped lockfile would be whatever stable produced that day. `D-010`'s
  own Enforced-by even records verifying "from a clean export with no lockfile"
  — a state no consumer is ever in.
- **Ruling:** `Cargo.lock` is **tracked, at format v3**. Cargo preserves an
  existing lockfile's format version, and with zero dependencies it never
  churns, so this fixes the fresh clone and the tarball with one file.
- **The general rule this is an instance of:** *gate the artifact, not the
  repository.* This is now the third time the two disagreed — the sibling's
  missing licence texts and its `../` README links (`D-022`) were the first two,
  and all three were invisible to checks that read the working tree. A check
  that never extracts a `.crate` is not checking what ships.
- **Two more instances fixed here:** `linq_rs_sql` cited `DECISIONS.md` in three
  shipped files (`README.md:91`, `src/lib.rs:38`, `src/pred.rs:28`) and shipped
  it zero times — it lives at the workspace root, which no member tarball can
  reach. All three now carry the URL.
- **Forbids:** shipping a lockfile format above v3 while the floor is 1.65;
  untracking `Cargo.lock` as a remedy for anything; citing a repo-root document
  from a member crate without a URL.
- **Enforced by:** `.github/scripts/msrv-tarball.sh`, run by the `msrv` CI job —
  packages with **stable** (packaging on the MSRV would generate a lockfile the
  MSRV can trivially read, which is the tautology that let this through),
  extracts every `.crate`, asserts each shipped lockfile is ≤ v3, and builds
  each on the declared floor. Verified to fail when the v4 lockfile is
  reintroduced. `packaging-gate.sh` additionally asserts `Cargo.lock` is
  tracked, and that no shipped `linq_rs_sql` file cites `DECISIONS.md` without
  a URL — both verified failing before the fix.

## D-021 — `pred!`: closure-shaped syntax, and why it is not `D-201`
- **Status:** SETTLED (2026-09-10) — implemented.
- **Ruling:** a `macro_rules!` front end, `pred!`, accepts closure-shaped source
  and expands to the existing builder calls.
  `pred!(employees, |e| e.salary > 100_000 && e.dept == "eng")` becomes
  `employees::salary.gt(100_000).and(employees::dept.eq("eng"))`.
- **Why a macro at all.** A closure cannot be translated: `|e| e.salary > 100_000`
  compiles to a function, and nothing at runtime can ask it which column, which
  operator, which value. C# escapes this with a compiler feature Rust lacks — a
  lambda typed `Expression<Func<T,bool>>` is emitted as a syntax **tree** rather
  than a method, and that tree is what EF walks. Rust's substitute is a macro,
  because macros see syntax before it becomes code.
- **This is not the DSL `D-201` rejects.** That rules out a
  `from x in xs where … select …` comprehension replacing method chaining, on
  the evidence that all three Rust crates which tried it are dead. `pred!` is
  one expression macro in one argument position; the chain, the types and the
  operators are unchanged, and removing it changes nothing but how the predicate
  is spelled. It is a front end, not a language.
- **The grammar is deliberately small:** `binding.field OP operand` for the six
  comparison operators, joined by `&&`/`||` with correct precedence, and
  parentheses. Nothing else parses — and **the grammar boundary and the
  translation boundary are the same line**, which is convenient rather than
  accidental: `|e| e.salary * 2 > budget` could not have become SQL either.
- **Error quality, measured rather than assumed.** I expected macro-internal
  spans and warned about them; both cases point at the user's own line and
  token. Exceeding the grammar gives
  `error: no rules expected '*' --> src/main.rs:7:70`; a wrong type still gives
  `error[E0271]: type mismatch resolving '<&str as Expr>::SqlType == Integer' …
  expected 'Integer', found 'Text'` at the call site.
- **It forced a prelude, and that was a real find.** The comparison operators
  live on type-specific traits so a `Text` column cannot be compared to an
  integer, and they must be in scope. Without them, **`Iterator::gt` exists** and
  rustc finds it instead, giving
  `` error[E0599]: `salary` is not an iterator ``. `linq_rs_sql::prelude` now
  exists for the same reason Diesel's does.
- **Forbids:** growing the grammar to cover expressions that cannot be
  translated. If it does not become SQL, it does not belong in `pred!` — use
  `.to_memory()` and a real closure.
- **Enforced by:** `linq_rs_sql/tests/pred.rs` — six tests asserting the macro
  expands to *exactly* the builder form (same SQL, same params), that `&&` binds
  tighter than `||`, that parentheses override it, that all six operators
  translate, that the same macro-built value evaluates in memory, and that it
  still streams.

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
- **Enforced by:** **IMPLEMENTED** (`W-6`, `W-18`) — three mechanisms:
  - `src/lib.rs` carries `#![doc = include_str!("../README.md")]`, so every
    ```` ```rust ```` block in the README is a doctest CI runs. Two blocks that
    stated their expected output in a *comment* — passing whether or not it was
    true — now `assert_eq!`. Mutation-tested: changing one expected value fails
    the build.
  - `.github/scripts/gen-docs.py --check`, run in CI. It **derives** the public
    surface from `src/`, cross-checks it against `.github/data/operator-map.tsv`
    in both directions, cross-checks that file's C# column against
    `.github/data/csharp-operators.tsv`, verifies every public method appears in
    the README's API Reference **and that no API Reference row names a method
    that does not exist**, computes every count, and fails if the committed
    README differs from what it generates.
  - The same script checks the **prose**, not just the generated blocks. The
    tables were gated from the start; the paragraphs around them were not, and
    they drifted badly — `README.md`, `CLAUDE.md` and `.github/data/README.md`
    between them named 28 methods that had been renamed or cut, and `CLAUDE.md`
    still told a contributor to return `impl Iterator`, which `D-106` forbids.
    Every removed name now lives in `.github/data/removed.tsv` with when and
    why, and a live doc naming one fails the build unless the surrounding lines
    are *explaining* the removal. `AUDIT.md`, `QUESTIONS.md`, `CHANGELOG.md` and
    `ROADMAP.md` are exempt by design: they are dated records, and rewriting
    them would falsify the trail.
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
- **Enforced by:** **IMPLEMENTED** (`W-20`) — `linq_rs_sql::rows` is the seam,
  and `Rows<Row, P, O>` deliberately does **not** implement `LinqExt`. A
  differential doctest pair pins the boundary: the negative half is
  `compile_fail` on `query::<Employee>().select_many(..)`, and the positive half
  is the byte-identical expression after `.to_memory(&people)`.

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

## D-106 — Named return types, everywhere
- **Status:** **SETTLED (2026-09-10)** — implemented; all 23 sites converted.
- **Ruling:** no method on `LinqExt` returns an opaque type. Every operator
  returns a named adaptor struct with `pub(crate)` fields — nameable but not
  constructible, exactly `std::iter::Filter`'s contract.
- **`AUDIT.md` B-1's premise was false, and its count was stale.** B-1 said
  opaque returns are "permanently sealed" against a later trait. They are not:
  a four-crate semver workspace with ten downstream call patterns — chaining,
  re-export behind the caller's own `impl Trait`, `Box<dyn Iterator + Send>`,
  generic fns, non-fused sources, `thread::spawn` — compiled **byte-identically**
  against opaque and named libraries. Auto-trait leakage survives, generically.
  Opaque → named is a *minor* change. And there were **23** opaque sites, not
  the 8 B-1 claimed (confirmed by 1.74 rejecting exactly 23 with `E0562`).
- **Two reasons it is worth doing anyway, neither previously on file:**
  1. **The opaque witness carries no operator identity.** `distinct`, `except`
     and `intersect` were all `Filter<I, closure>`. One blanket impl covers all
     three, so a future `Sql` trait could not give them different SQL. *That* is
     what blocked the v2 seam — not nameability. Verified after conversion: a
     downstream trait impl'd separately for `Except`, `Intersect` and
     `GroupByKey` prints `EXCEPT / INTERSECT / GROUP BY`.
  2. **Method-bearing bounds are not additive.** `+ FusedIterator` can be added
     to a shipped RPITIT later; `+ ExactSizeIterator` gives downstream
     `error[E0034]: multiple applicable items in scope`, and `+ Clone` needs a
     new input bound. So the impl set must be frozen at 1.0 whichever
     representation is chosen — and only a conditional impl on a named type
     (`impl FusedIterator for Distinct<I> where I: FusedIterator`) adds a
     capability without adding an input bound. RPITIT cannot express that.
- **The bonus: the MSRV drops from 1.75 to 1.65.** Those 23 RPITIT sites *were*
  the pin. Verified: the full suite passes on a real `rustc 1.65.0`. Ten
  releases of headroom, recovered by a change made for other reasons. See
  `D-010`.
- **Cost:** ~430 changed lines across `adaptors.rs` and `queryable.rs`, **zero
  test edits**, zero behaviour change. 14 of the 23 were trivial newtypes over
  `vec::IntoIter`, because the eager operators have already run their closures
  by the time the type exists — so the feared unnameable
  `InnerJoin<I, J, KO, KI, R>` signature never materialised; `inner_join`
  returns `InnerJoin<R>`, one parameter.
- **Enforced by:** **IMPLEMENTED** — CI greps `src/queryable.rs` for
  `-> impl Iterator` and fails on any non-zero count. Currently zero.

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
