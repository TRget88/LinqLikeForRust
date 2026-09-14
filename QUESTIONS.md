# QUESTIONS.md — Phase 0 target elicitation

> ## ⚠ SUPERSEDED — Phase 0 record, not a description of the crate
>
> **Written 2026-09-09. Do not read any statement here as current.** Its premise
> about the repository was already wrong when written (see the correction below),
> and the code has since moved much further: the claim that
> "no `IQueryable`/`IEnumerable` seam exists **or is possible** in the current
> design" is false — the seam exists, executes against a real SQLite, and is
> gated in CI — and "LINQ-to-Entities is a greenfield decision" describes
> something that is now built and tested. Kept as the record of what was asked
> and answered at Phase 0.

> ## ⚠ CORRECTION (2026-09-09, after Phase 1)
>
> **This document was written against `main` only and one of its premises is
> false.** The "Context you should have" section below states that the repository
> contains "**zero** database, entity, or query-translation code". That is true of
> `main` @ `bd9fd4f`. It is **false of the repository**: the unmerged branch
> `origin/feature/v0.1.0-and-sql-builder` @ `d99c405` (2026-05-24) contains
> `ROADMAP.md`, `CLAUDE.md`, CI, 263 passing tests, and 1,104 lines of typed SQL
> query builder. The Phase 0 search ran `git log` and `git status` but never
> `git branch -a`.
>
> The answers recorded against these questions are still valid — they are the
> owner's rulings, not statements about the code — and they are now in
> [DECISIONS.md](DECISIONS.md) as `D-001`…`D-018`. Read that file, not this one,
> for what has been decided. This file is kept only as the record of what was
> asked and on what (partly wrong) basis.
>
> See [AUDIT.md](AUDIT.md) §0 for the full correction and §1 finding **A-1** for
> why the branch matters more than any bug in it.

Audit of `linq_rs` @ `bd9fd4f`. Written 2026-09-09.

The current-state audit is done from the code. The **target** state cannot be
audited into existence — it has to be decided. Every question below has a
*why it matters* and a **recommended default**, so you can answer "defaults"
and I proceed, or override individually.

Answer format that works: `Q1: b`, `Q4: sqlite+postgres, async only`, etc.

---

## Context you should have before answering

Findings that change what these answers *cost* (evidence in the Phase 0 summary):

- The repo is **one product, not two**: ~1,155 LOC of LINQ-to-objects. There is
  **zero** database, entity, or query-translation code — `grep -i 'sql|database|entity|migration'`
  over all sources and the README returns 0 hits. So "LINQ-to-Entities + EF" is
  not a half-built feature to be assessed; it is a greenfield decision.
- `linq_tests.rs` (42 `#[test]` fns) **is not a Cargo target** and has never been
  compiled. `cargo test` runs 0 unit tests, 0 integration tests, 17 doctests.
  When wired up in a scratch copy: 1 of 42 fails (`then_by` is genuinely broken)
  and 1 did not compile (`skip` is ambiguous with `Iterator::skip`, E0034).
- The architectural commitment is **typed combinator chains over `Iterator`**
  via a blanket-impl extension trait. There is no expression tree, no closure
  capture, no IR — i.e. **no `IQueryable`/`IEnumerable` seam exists or is
  possible in the current design**. Q2 is therefore the load-bearing question in
  this document.

---

## Q1 — Scope: which halves are actually in scope?

Options:
- **(a)** LINQ-to-objects only. Polish, correct, publish the crate that exists.
- **(b)** LINQ-to-objects + query translation to SQL (no ORM: no change tracking,
  no unit of work, no migrations).
- **(c)** All three: objects + translation + EF-style ORM.

**Why it matters.** (a) is a finishing job measured in days. (b) requires
inventing the seam that does not exist today — a query IR plus a `Queryable`
trait — and is measured in months. (c) adds the ORM concerns, which are where
Rust's ownership model actively fights the EF design (see Q2), and is measured
in quarters, by a team.

**Recommended default: (a) for v1.0, with (b) explicitly named as the v2 thesis
and designed-for but not built.** Reason: (a) is the only option where you ship
something defensible this quarter, and the code you have is 85% of it. Shipping
(a) also buys the information you need to price (b) honestly.

---

## Q2 — Which parts of EF do you actually want? *(only if Q1 ≥ (c))*

This is the question the whole project turns on. Change tracking, the identity
map, lazy loading, and navigation-property fixup all assume **one shared,
mutable, aliased object graph** — precisely what Rust's ownership model refuses.
There is no design that gives you all of it. The honest menu:

| | Model | Fits Rust | Feels like EF | Cost |
|---|---|---|---|---|
| **(a)** | **Explicit-save repository.** You load owned entities, mutate them freely, and call `repo.update(&entity)`. No tracking; the save is the diff. | Excellent — plain `&mut`, no interior mutability, `Send + Sync` falls out free | Low. This is closer to Dapper or a hand-rolled repository than to `DbContext` | S–M |
| **(b)** | **Snapshot-diff change tracking.** `ctx.track(entity)` clones the entity on load; `SaveChanges` diffs current against snapshot and emits `UPDATE` for changed columns only. | Good — needs `Clone` on entities and a `&mut Context`, nothing exotic | Medium-high. `SaveChanges()` works and updates only dirty columns, which is the part users actually notice | M–L |
| **(c)** | **Interior-mutability identity map.** `Rc<RefCell<Entity>>` (or `Arc<RwLock<_>>`) in a context-owned map, enabling real reference identity, navigation fixup, and lazy loading. | Poor — `Rc`/`RefCell` leak into every user signature, `!Send` kills async unless you pay for `Arc<Mutex<_>>`, and `RefCell` turns aliasing bugs into runtime panics | High. This is the only option that gets you lazy loading and navigation fixup | L, and permanent |

**Why it matters.** This choice is not reversible. It is stamped into every
public signature — whether users write `&mut User` or `Rc<RefCell<User>>`, and
whether your futures are `Send`. Deciding it late means rewriting the API.

**Recommended default: (b), snapshot-diff.** It is the only point on the curve
that delivers the recognisable `SaveChanges()` moment without putting `RefCell`
in your users' function signatures. Explicitly **do not** do (c) — see the
DO-NOT-BUILD list this audit will produce.

*Corollary, needs its own yes/no:* **lazy loading / navigation properties — in
or out?** Recommended: **out, permanently.** `user.orders` implicitly firing a
query requires either interior mutability or a hidden global, and it is the
single largest source of N+1 pathologies in real EF codebases. Eager `.include()`
into an owned `Vec<Order>` field gives 90% of the value with none of the magic.

---

## Q3 — Fidelity: how C#-like should this feel?

The current code has already made this ruling implicitly, and it landed in an
awkward spot: **18 of 49** public methods carry a trailing-underscore name
(`where_`, `flatten_`, `skip_while_`, `take_`, `take_while_`, `concat_`,
`union_`, `sum_`, `min_`, `max_`, `min_by_key_`, `max_by_key_`, `any_`, `all_`,
`contains_`, `zip_`, `for_each_`, `is_empty_`) purely to dodge collisions with
`std`. The result reads as neither C# nor Rust.

Options:
- **(a) Maximum familiarity.** Keep chasing LINQ names; accept the underscores.
- **(b) Idiomatic Rust, LINQ-shaped.** Drop the aliases that merely rename an
  existing `Iterator` method (`sum_`, `min_`, `max_`, `any_`, `all_`, `for_each_`,
  `take_`, `skip`, `flatten_`, `contains_` are all thin wrappers over std today —
  see `queryable.rs:276-329`, `398-421`). Keep LINQ names only where they name
  something std lacks: `group_by`, `to_lookup`, `join`, `group_join`,
  `order_by`/`then_by`, `distinct_by`, `chunk`, `select_many`.
- **(c) Namespaced fidelity.** Full LINQ names with no underscores, exposed on a
  wrapper type (`.linq()` → `Linq<I>`) instead of a blanket impl on `Iterator`,
  so `where`-adjacent names never collide with std.

**Why it matters.** This is your differentiation *and* your main adoption risk.
Option (a)'s underscores are exactly the "uncanny valley" failure — a C# dev
still can't type `.Where(...)` from muscle memory, and a Rust dev sees a worse
`.filter()`. Option (c) is the only one that delivers real C# muscle memory,
because inside a wrapper type `where_` can just be `filter`-free naming.

**Recommended default: (b).** It is the smallest, most honest crate and it makes
the value proposition legible: *the LINQ operators Rust is missing*, not a
second name for the ones it has. (c) is the interesting swing if the C#-migrant
thesis is the actual point of the project — say so and I will grade it that way
in Phase 2.

---

## Q4 — Backends and async *(only if Q1 ≥ (b))*

Which databases, and sync, async, or both?

**Why it matters.** Async trait methods returning `impl Future` interact badly
with lifetime-carrying query builders, and "both" is not one API with a flag —
it is two API surfaces with a shared IR, or a macro that generates both.
Supporting three databases multiplies your SQL-dialect test matrix by three
before you have written a single operator.

**Recommended default: PostgreSQL first and only, async only, on `sqlx` for the
driver layer.** Add SQLite second solely because it makes tests hermetic. Do not
promise MySQL or SQL Server.

---

## Q5 — Compile-time-checked queries vs runtime-built dynamic queries

**Why it matters.** Diesel's compile-time typing is why its error messages are
notoriously long and why its build times are what they are; SeaORM's dynamic
model is why its queries can be composed at runtime but a typo survives to
runtime. You cannot have both without a macro that reads your schema, and that
macro is a project in itself.

**Recommended default: runtime-built, statically-typed *inputs*.** Query
structure is a runtime value (an IR you can build in a loop); column and table
identifiers come from a `derive(Entity)`-generated table descriptor, so they are
still compile-time checked and never string-interpolated. This keeps compile
times sane and closes the SQL-injection question for identifiers by
construction.

---

## Q6 — Migrations: in scope or delegated?

**Why it matters.** Migrations are a schema-diff engine, a version ledger, and a
CLI. They share almost no code with query translation. Every ORM that built them
early spent more time on them than on queries.

**Recommended default: delegated.** Point users at `sqlx migrate` or `refinery`,
and ship a `derive(Entity)` that can *emit* a `CREATE TABLE` for scaffolding
only. Reconsider only after v2 ships.

---

## Q7 — Audience and success criteria

Which is this?
- **(a)** Personal tool — you are the only user.
- **(b)** Portfolio artifact — it exists to be read by someone evaluating you.
- **(c)** Published crate with real users and a support burden.

**Why it matters.** It sets the bar for everything else. (b) rewards a small,
immaculate, well-documented crate with a sharp thesis. (c) demands semver
discipline, a CI matrix, an MSRV policy, and a deprecation story — and makes the
Q3 naming decision permanent on the day you publish 1.0.

**Recommended default: (b) now, (c) as an explicit later gate.** Also tell me
what *done* looks like in one sentence — I will hold the roadmap to it.

---

## Q8 — Constraints

- **Stable Rust only?** Recommended: **yes.** Nothing in the current code needs
  nightly, and RPITIT (used at `queryable.rs:171`, `455`, `529`) has been stable
  since 1.75.
- **MSRV floor?** Currently **unspecified** — `Cargo.toml` has no `rust-version`
  field, so `cargo` will not enforce one. Recommended: **declare 1.75**, which is
  the real floor imposed by `-> impl Iterator` in trait methods, and add it to
  CI. (Local toolchain is 1.96.0.)
- **`no_std`?** Recommended: **no**, but keep the door open — `alloc` would
  suffice for everything except `to_hashmap`/`to_hashset`, which could go behind
  a `std` feature.
- **License?** MIT is already in place (`LICENSE`, "Copyright (c) 2026 Kirk").
  Recommended: **dual MIT/Apache-2.0**, the Rust ecosystem norm, and set
  `license = "MIT OR Apache-2.0"` before any publish. Trivial now, awkward later.
- **Timeline?** No default — tell me the real one. It decides the v1.0 cut line.

---

## Q9 — Logistics for the rest of this audit

1. **Is a database available** for exercising a DB path? (Postgres/SQLite
   connection string, or "no".) Currently irrelevant — there is no DB code — but
   it decides whether Phase 4's DB scenarios are *GAP* or *UNVERIFIABLE*.
2. **Any prior design notes** beyond `README.md`? Sketches, a Notion page, an
   earlier branch? I will treat them as stated intent and grade drift against
   them. `git log` shows 4 commits and no design docs.
3. **Should `linq_tests.rs` be treated as intended-to-run?** I am assuming yes —
   that its exclusion is an accident, not a decision. Confirm, because it changes
   whether the failing `then_by` assertion is a known-broken feature or a
   regression nobody could see.
4. **Is `README.md` binding?** I am treating its API tables as claims to be
   graded, including "Brings the full power of C# LINQ" and the "Lazy by default"
   design note. Say so if it is aspirational marketing rather than a contract.
