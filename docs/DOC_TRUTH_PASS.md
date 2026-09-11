# Documentation truth pass

**Pinned SHA: `8501f879110a8feb16d013313b5c250ed10f32a2`** (branch `docs/readme-refresh`).
Every verdict below rests on something that was compiled, run, or fetched at that
commit. Corrections were made in commits after it; the SHA is the state assessed,
not the state now.

## Executive summary

The documentation made **1788 atomic claims**. **284** were verified by
execution: **187 TRUE**, **46 FALSE**, **49 PARTIAL**,
**2 UNTESTABLE**. **28** were
flagged dangerous — believing them would make a user write incorrect code.
**33** have a different verdict
against the versions actually on crates.io than against this SHA.

**The most dangerous thing the documentation said** was that the two interpreters
agree. `README.md:288` — *"The same value renders to SQL for a database or evaluates
lazily over a `Vec` in a unit test, and the two agree"* — is false for `LIKE`, on
six of six patterns tested:

```text
LIKE 'eve'    SQLite [1, 2, 3]   in-memory [2]
LIKE '%PL%'   SQLite [4, 5]      in-memory [5]
```

SQLite folds ASCII case in `LIKE`; the in-memory matcher at `rows.rs:595-623`
compares chars literally. A `to_memory` unit test passes while the database returns
different rows. Three things make it worse: **no test anywhere runs `LIKE` against a
real database** (0 matches for `like` in `linq_rs_sqlite/tests/`); `rows.rs:582`
already labels `LIKE` provider-defined; and `DECISIONS.md:1210` (`D-103`) explicitly
*forbids* "any claim that a query 'gives the same result' across providers". The
README was making the one claim the project's own ledger prohibits.

## Adjudication

| Verdict | Count |
|---|--:|
| DOC WRONG — code correct, documentation fixed | 90 |
| ASPIRATIONAL — never built or since removed | 2 |
| INTENT UNCLEAR — escalated, then approved | 5 |
| CODE WRONG — documentation right, code defective | 0 |

No claim required a code change. One **code finding** was filed rather than fixed,
per the agreed scope (documentation pass only).

### Code finding — `LIKE` diverges between the interpreters

**Severity: high.** The divergence is silent, it sits under the crate's headline
capability, and no test covers it.

*Action plan, in words.* Decide first whether the in-memory matcher should imitate
SQLite's default ASCII case folding, or whether case-sensitive matching is the
intended contract and SQLite is the outlier. That is a semantics decision, not a bug
fix, because MySQL folds case too while PostgreSQL does not — whichever behaviour is
chosen will disagree with some provider, which is exactly why `D-103` classifies
`LIKE` as provider-defined rather than promising identity. Then add a differential
test that runs every `LIKE` pattern shape through both interpreters against a real
SQLite and asserts the documented relationship, so the gap cannot reopen silently.
Until both exist, the README's disclosure stands in for the missing guarantee.

## Claim register

Full register, verdicts and evidence: `register.json`, `verdicts.json`,
`adjudication.json` in the pass's working directory (not committed — probe artifacts
are disposable by policy).

| Artifact group | Claims | Verified |
|---|--:|--:|
| README.md | 299 | 198 |
| sibling READMEs | 220 | 0 |
| linq_rs_sql rustdoc | 317 | 0 |
| linq_rs rustdoc + examples | 354 | 0 |
| CLAUDE.md + PUBLISHING.md | 244 | 0 |
| Cargo metadata + provider rustdoc | 182 | 0 |
| AUDIT.md + QUESTIONS.md | 172 | 86 |

### Claim types

| Type | Count | How it was settled |
|---|--:|---|
| BEHAVIOR | 638 | exercised, incl. edge cases |
| API_EXISTS | 212 | probe crate naming each item as documented |
| SIGNATURE | 184 | same probe |
| LINK | 158 | resolved; internal by path, external by HTTP |
| STATUS | 131 | checked against what the tests prove |
| NUMBER | 126 | re-derived; generated ones gated two-way |
| SUPPORT | 109 | compiled on the named toolchain |
| EXAMPLE | 87 | compiled and run as printed (doctests) |
| TRANSLATION | 79 | emitted SQL captured and run against SQLite |
| UNTESTABLE | 64 | reported, not passed over |

## What changed, per artifact

| Artifact | Claim IDs | Change |
|---|---|---|
| `README.md` | RR-124, RR-118, RR-132, RR-139, RR-129 | The seam section now states what the two interpreters agree on, by operator, and discloses the `LIKE` divergence as a known defect. Adds that compile-time column checks are against the *declaration*, not the database, and that the projected column list comes from `entity!`. |
| `README.md` | RR-047, RR-071, RR-086, RR-095, RR-100, RR-169–RR-174 | `single_or_default` returns `Result<Option<T>, SingleError>`, not `Option<T>`. `contains_(&value)`. "Ten operators compare keys" → twenty-eight. `.key()`/`.elements()` borrow. Empty "Utility" table removed. |
| `README.md` | RR-014, RR-016, RR-028, RR-177, RR-178 | Counter-example compiles as printed. `.github/` paths link to GitHub and say they do not ship. `AUDIT.md` marked superseded at the reference. `CHANGELOG` heading state disclosed. |
| `README.md` | RR-029, RR-030 | Surface and rename figures re-framed published-to-published: 48 → 62 (a gain), five renames not two. |
| `.github/data/operator-map.tsv` | RR-143, RR-145, RR-146, RR-150, RR-151, RR-160 | Six `std_equivalent` cells were not substitutable. `first` → `next().expect(..)`, etc. Fixed in the data because the table is generated. |
| `.github/scripts/gen-docs.py` | RR-011, RR-012, RR-014 | Stops claiming unimplemented C# statics are free functions here; prints Rust names; links the excluded `.github/` paths. |
| `AUDIT.md`, `QUESTIONS.md` | AQ-001, AQ-002, AQ-010, AQ-013–AQ-020, AQ-079, AQ-149, AQ-152, AQ-153 | SUPERSEDED banners naming the specific reversals. **No claim inside either file was rewritten** — correcting 900 lines of dated evidence to match today's code would destroy the only thing they are good for. |
| `linq_rs_sql/README.md`, `ROADMAP.md` | RR-129 | The `i64` suffix is not required; the real limit is that literals default to `i32`. Roadmap item rewritten around the actual defect. |
| `ROADMAP.md` | — | Two ungated historical numbers dated rather than deleted. |
| `linq_rs_sql/src/rows.rs` | RR-139 | `select_many` no longer cites a `D-019` classification that contradicts `operator-map.tsv`. |

## Gates

**Installed before this pass, verified working:**

- `#![doc = include_str!("../README.md")]` on all three crates — every README code
  block is a doctest. This is the single highest-value gate and it is why the
  *examples* held up far better than the prose.
- `cargo test --doc` in CI via `test-count-floor.sh`, with a **separate floor for
  doctests** (59) so a silently-deleted example fails the build. Output is not
  truncated — a `| tail -N` here previously hid a real failure.
- `gen-docs.py` derives every coverage/overlap/laziness number from `.github/data/`
  and cross-checks both directions. It caught two of my own edits during this pass:
  one that touched a generated block, one that went stale.
- `packaging-gate.sh` resolves every relative link against each package's own
  `cargo package --list`, so a link valid in the repo but broken in the tarball fails.

**Gaps this pass revealed and did not close:**

| Gap | Evidence it matters |
|---|---|
| No external-URL checker | Two genuinely broken links found (`crates.io/crates/linq_rs_sqlite`, `tree/main/linq_rs_sqlite`) — both from documenting unmerged work as live |
| No markdown anchor checker | I broke an anchor *during this pass* and caught it by hand |
| `deny(missing_docs)` on 1 of 3 crates | `linq_rs` and `linq_rs_sql` public items can ship undocumented |
| No stated lockstep rule | Nothing requires a behaviour change to update its documentation in the same commit |

**Unguarded claim types after all of it.** BEHAVIOR (590 claims) and TRANSLATION (73)
have no mechanical gate — a doctest proves an example runs, not that prose describing
an edge case is true. That is where the remaining risk lives, and it is where every
dangerous claim in this pass was found. STATUS claims are likewise unguarded: nothing
detects a feature described as planned that has since shipped.

## Could not verify

| What | Blocked by |
|---|---|
| PostgreSQL and MySQL dialect claims | No server available. The crate emits `?` placeholders, which PostgreSQL rejects, so those claims are moot until a dialect layer exists. |
| `crates.io/crates/{linq_rs,linq_rs_sql}` link targets | crates.io returns 404 to any non-browser client; both confirmed published via its API instead. |
| `crates.io/settings/tokens` | Authenticated page. |
| `DECISIONS.md`, `CHANGELOG.md`, `ROADMAP.md` prose claims | Extraction exceeded the agent output limit twice. The highest-value part was verified mechanically instead: all 39 `Enforced by` blocks exist, 47/49 named targets resolve, and all four gates pass. Their remaining prose is unassessed. |
| Whether a claim is true for a *user* today | Every verdict is against this SHA, which is the tip of a seven-branch stack that is neither merged nor published. 33 claims have a different verdict against the live crates. |

## Addendum — `linq_rs_sqlite` removed after this pass

The owner's dependency rule (`D-032`) is stricter than the one this pass assessed
against: `linq_rs` takes no dependencies, `linq_rs_sql` may take only `linq_rs`,
and nothing else is acceptable anywhere in the project. `linq_rs_sqlite` violated
it and has been deleted.

Consequences for the verdicts above. Every claim verified *against* a real SQLite
still stands — the evidence was produced by running real queries, and deleting the
crate does not unrun them. What changes is reachability: the adapter that produced
that evidence now lives in `docs/DRIVER_ADAPTER.md` as code a user writes, not as
a crate they depend on. The `LIKE` finding is unaffected and still open; it is a
defect in `linq_rs_sql`'s in-memory matcher, which was never part of the provider.

Claim counts in this report are as of `8501f87` and are not restated for the
removal.
