# AUDIT.md — `linq_rs`

Audit date **2026-09-09**. Trees graded: `main` @ `bd9fd4f` and
`origin/feature/v0.1.0-and-sql-builder` @ `d99c405`.
Toolchains: `rustc 1.96.0`, plus `1.75.0` for the MSRV check.
Read-only audit: no library code was modified. Probe code lived in a scratch
directory and was not committed.

Confidence is marked throughout: **CONFIRMED** (run or read directly),
**CLAIMED** (asserted by docs, unverified), **DIVERGED** (docs say X, code does
Y), **UNVERIFIABLE**.

Decisions live in [DECISIONS.md](DECISIONS.md), not here. This file is a dated
snapshot of evidence; anything in it that should constrain future work has been
promoted to a `D-NNN` entry there.

---

## 0. Correction to this audit's own Phase 0

**Phase 0 graded the wrong tree, and the error was mine.** I read `git log` and
`git status` and never ran `git branch -a`. Phase 0 therefore asserted, in
`QUESTIONS.md`, that the repository contains "**zero** database, entity, or
query-translation code". That is true of `main`. It is false of the repository.

`origin/feature/v0.1.0-and-sql-builder` @ `d99c405` (2026-05-24, KirkGit) is
**not merged** into `main` and is two months newer. CONFIRMED:

```
git diff --stat main origin/feature/v0.1.0-and-sql-builder
 30 files changed, 5873 insertions(+), 1201 deletions(-)
```

It contains a `src/` layout, `.github/workflows/ci.yml`, `CHANGELOG.md`,
`ROADMAP.md` (206 lines), `CLAUDE.md` (143 lines), four `examples/`, three test
files, and `src/sql/` — **1,104 lines of typed SQL query builder with a `table!`
declarative macro**. I built and ran it: **263 tests pass** (0 lib + 64
`edge_cases` + 128 `linq_tests` + 28 `sql_phase1` + 43 doctests).

The branch independently found and fixed four of this audit's Phase 0 findings.
`ROADMAP.md` records them under "Out-of-band fixes", including — verbatim — "the
integration tests in `linq_tests.rs` were not wired into `Cargo.toml`, so they
had never actually run".

So the user's Phase 0 answer "all docs are in repo" was **correct**; I mis-scoped
the search. Two consequences for how to read this report:

1. Findings are tagged **[main]**, **[branch]**, or **[both]**. A finding tagged
   [main] only is already fixed on the branch and appears here for the record.
2. The single most important finding in this audit is not a bug. It is that the
   repository now holds two disjoint products and four mutually contradicting
   statements of intent — which is precisely the failure the owner asked this
   audit to prevent. See §1, finding **A-1**.

---

## 1. Executive summary

`linq_rs` is a competent, zero-dependency in-memory LINQ layer over
`Iterator` that has been built out well past the point where its scope was
last agreed. `main` is a 1,155-line 50-method surface with a test file that
never compiled. The unmerged branch is a 5,738-line 90-method surface with 263
green tests, CI, a changelog, a roadmap, an agent-context file, and a Diesel-
shaped SQL builder — none of which has been merged, tagged, or published. What
*is* published on crates.io is `main`: version 0.1.0, 2026-03-28, 25 downloads,
not yanked, no `repository` link (CONFIRMED via the crates.io API), containing
every bug the branch already fixed.

The engineering on the branch is genuinely good — `then_by` was rebuilt as a
comparator stack sorted once at `into_iter` time, which is the correct design and
the same one three independent reviewers in this audit arrived at. The problem is
not craft. It is that the crate has been growing surface area in the one
direction that cannot be defended, while the decision record that was supposed
to prevent that has been contradicting itself for five months.

### The three findings that matter most

**A-1 — Two products, four intents, no seam. [branch] CONFIRMED.**
`CLAUDE.md:12-14` states as explicit non-goals: "We are not building an
`IQueryable`/expression-tree analogue" and "We do not target databases, async
streams, or remote execution." `ROADMAP.md` lists under *Rejected / out of
scope*: "**`IQueryable` / expression trees** — out of scope. We target in-memory
iterators only." The same branch ships `src/sql/` — a 1,104-line SQL builder,
committed as "ORM Phase 1". And the Phase 0 ruling taken in this session makes
SQL translation the named v2 thesis. Four documents, four positions.

The two products share nothing. `grep -rn "LinqExt\|Iterator" src/sql/` returns
exactly one hit, and it is a doc comment saying the two are unrelated. The same
concept has two names — `LinqExt::where_` and `sql::filter` — and no value
crosses between them. `src/sql/mod.rs:7-10` is candid: "this module produces SQL
strings — it does **not** execute against a database."

This is the finding that will kill the library if nothing else does. It is not a
bug to fix; it is a scope decision that was made three times, differently, and
recorded in three places that do not reference each other.

**A-2 — The differentiation claim does not survive contact with `std`.
[both] CONFIRMED.** Of the 50 public methods on `main`, 29–31 are pure renames
or literal delegations of an existing `std::iter::Iterator` method — eight of
them call the std method one line down (`aggregate` is `self.fold(seed, f)`,
`sum_` is `self.sum()`, `to_vec` is `self.collect()`, `sequence_equal` is twelve
hand-written lines reimplementing `Iterator::eq`). A first pass concluded that
`join` and `group_join` were the two genuinely novel concepts. An adversarial
pass then **compiled** std-only reproductions of all six candidates and found
**zero** methods whose capability cannot be reproduced, byte-for-byte and under
identical trait bounds, in at most four statements of `std` alone — `join` in two
statements, `group_join` in one.

The branch's response to this pressure was to grow from 48 methods to 90. That
doubles down on the crate's principal liability. Meanwhile `itertools` 0.15.0
(2026-06-16, 1.48 billion downloads, same blanket-extension-trait architecture)
occupies the same architectural position with three times the surface, and
collides with this crate on two method names (§5).

**A-3 — What is published is the broken tree, and the fixed tree has never
run CI. [both] CONFIRMED.** The published 0.1.0 contains `then_by` returning
wrong results, a `LinqExt::skip` that makes every unqualified `.skip(n)` in an
importing module a hard `E0034`, and a `concat_` whose bound rejects any
mid-chain call. The branch fixes all three but its `Cargo.toml` still reads
`version = "0.1.0"`, so it cannot be published without a bump, and its CI has
never executed — the workflow triggers only on push to `main`/`master` or on a
PR, and the branch received neither.

Worse, its MSRV job would have failed on first run. The tracked `Cargo.lock` is
**lockfile v4**, which Cargo 1.75 cannot parse. I ran it:

```
$ rustup run 1.75.0 cargo build
error: failed to parse lock file at: .../Cargo.lock
Caused by:
  lock file version `4` was found, but this version of Cargo does not
  understand this lock file, perhaps Cargo needs to be updated?
```

With the lockfile removed both trees build clean on 1.75. So the code's 1.75
floor is real; **the repository's is not**, and a fresh `git clone` on the
declared MSRV fails before compiling a line. `.gitignore` lists `Cargo.lock`
under "# Cargo lockfile (ignore for libraries)", but the file is tracked, so the
pattern never took effect — `git check-ignore -v Cargo.lock` exits 1.

---

## 2. Ground truth

| | `main` @ `bd9fd4f` | branch @ `d99c405` |
|---|---|---|
| Date | 2026-03-28 | 2026-05-24 (unmerged) |
| Layout | 6 `.rs` at repo root, `[lib] path = "lib.rs"` | standard `src/` |
| Library LOC | 1,155 | 5,738 (all `.rs`) |
| `LinqExt` methods | **48** (+2 on `ThenBy` = 50) | **90** |
| Adaptor structs | 14 | 22 |
| Tests executed by `cargo test` | **0 unit, 0 integration, 17 doctests** | **263** (64 + 128 + 28 + 43 doctests) |
| `linq_tests.rs` | at repo root, **not a cargo target**, 42 `#[test]` fns, never compiled | in `tests/`, 128 tests, green |
| Dependencies | zero, incl. dev-deps | zero |
| Edition / MSRV | 2021 / **unspecified** | 2021 / `rust-version = "1.75"` |
| `size_hint` impls | 1 of 14 | 9 |
| `ExactSizeIterator` / `DoubleEndedIterator` | 0 / 0 | 6 / 2 |
| `FusedIterator` | 0 | **0** |
| `#[must_use]` / `#[inline]` | 0 / 0 | 0 / 0 |
| Crate attributes | **none** | `#![warn(missing_docs)]`, `#![forbid(unsafe_code)]` |
| CI | none | `ci.yml`, **never run** |
| `repository` in Cargo.toml | unset | **still unset** |
| `license` | `MIT` | **still `MIT`** (target is dual) |
| `unsafe` | 0 | 0 |
| `async` | none | none |
| clippy `--all-targets` | 5 warnings | passes with `-D warnings` per its own CI config |

Published artifact (CONFIRMED, crates.io API): `linq_rs` **0.1.0**, created
`2026-03-28T18:24:29Z`, 25 downloads / 7 recent, single version, **not yanked**,
`license = MIT`, `rust_version = null`, `repository = null`. Owner: crates.io
login `TRget88`, display name "Kirk". `linq-rs` and `LINQ_RS` normalise to the
same taken name; `linqrs` is free (404).

**Two numbers, used consistently throughout this report:** **48** is the method
count on the `LinqExt` trait alone; **50** is the full public query surface,
i.e. those 48 plus `ThenBy::then_by` and `ThenBy::then_by_descending`. Where a
claim is about receivers, trailing underscores, or the trait itself it says 48;
where it is about operator coverage it says 50.

Two counts carried into this audit were wrong and are corrected here: the
`LinqExt` surface on `main` is **48** methods, not 49 (a naive `grep -c 'fn '`
also matches `queryable.rs:563`, `let mut key_fn = key_fn;`), and there are
**14** adaptor structs, not 12.

### The architectural commitment, from the code

**Typed combinator chains via a blanket-impl extension trait.** CONFIRMED:
`impl<I: Iterator> LinqExt for I {}` (`queryable.rs:662` on `main`). No proc
macro, no expression tree, no IR, no reified query object. `OrderedQueryable` is
a `Vec` plus — on the branch — a comparator stack; it is not a queryable.

**Verdict: viable for what it is, and structurally sealed against what the v2
thesis needs.** Combinators execute as they compose; nothing is captured for
later inspection, so there is no `IQueryable`/`IEnumerable` duality. The branch's
SQL builder does not create one — it is a second, parallel, Diesel-shaped
builder with its own vocabulary. Two specific foreclosures are compiler-verified
in §7.

Read the SQL builder on its own terms and it is decent work: parameterised
placeholders, marker-type SQL typing, identifiers sourced from `table!`
literals. I verified the injection posture on the branch —
`filter(employees::name.eq("Alice'; DROP TABLE employees; --"))` renders
`WHERE (name = ?)` with the payload in `params`, so **A-2 aside, the identifier
and value handling is right by construction** (CONFIRMED). It simply is not
connected to `LinqExt`.

---

## 3. Feature matrix

Rows are the LINQ operator families and the EF concerns from the brief. "Tested"
means covered by a test that **actually executes** — on `main` that is only the
17 doctests.

### LINQ operators

| Operator | In-memory | DB path | Tested [main] | Tested [branch] | Documented | Matches the C# it claims |
|---|---|---|---|---|---|---|
| `Where` (`where_`) | yes | no | doctest `:24` | yes | yes | yes |
| `Select` (`select`) | yes | no | doctest `:42` | yes | yes | yes |
| `SelectMany` (`select_many`) | yes | no | doctest `:57` | yes | yes | binds `J: Iterator` where `zip_` binds `IntoIterator` — internal inconsistency |
| `OrderBy` / `OrderByDescending` | yes | no | doctest `:220` | yes | yes | byte-ordinal strings, not culture-aware; `K: Ord` so no `f64` |
| `ThenBy` / `ThenByDescending` | yes | no | **none** | yes | yes | **[main] NO — destroys primary order.** [branch] yes |
| `GroupBy` (`group_by`) | yes | no | doctest `:518` | yes | yes | ordering yes; O(n·k) |
| `Join` (`join`) | yes | no | doctest `:432` | yes | yes | true inner join, C# order; O(n·m) |
| `GroupJoin` (`group_join`) | yes | no | **none** | yes | yes | true LEFT OUTER, C# order; forces `Clone` |
| `Distinct` / `DistinctBy` | yes | no | doctest `:135` / none | yes | yes | no NaN reflexivity; O(n·d) |
| `Skip` / `Take` (+`While`) | yes | no | **none** | yes | yes | `skip` unusable in method position [main] |
| `First` / `Single` / `Any` / `All` | yes | no | **none** | yes | yes | `single_or_default` loses the >1 case |
| `Aggregate` | yes | no | doctest `:263` | yes | yes | yes |
| Set ops (`Except`/`Intersect`/`Union`) | yes | no | doctest `:166`,`:182` / none | yes | yes | **no dedup**; `union_` dedups one side only |
| `Chunk` | yes | no | doctest `:119` | yes | yes | two undocumented panic paths |
| `Reverse`, `Zip`, `Concat`, `Append`/`Prepend`, `SequenceEqual` | yes | no | `zip_` `:601` only | yes | yes | `concat_` bound unusable mid-chain |
| `Sum`/`Min`/`Max`/`Average`/`Count` | yes | no | `:263`,`:286`,`:335` | yes | yes | std-delegating; documented C# equivalence is wrong |
| Comparer overloads (44 in C#) | **none** | no | — | — | declared out of scope in `ROADMAP.md` §2.4 | intentional |
| `TakeLast`/`SkipLast`/`DefaultIfEmpty`/`Order`/`Index`/`CountBy`/`AggregateBy`/`OfType`/`Cast`/generators | **[main] absent** | no | — | **[branch] present** | branch only | branch: yes |

Coverage on `main`, recounted independently twice: **16 of 50 methods** have an
executed test; 34 have none. Of the 17 doctests one (`queryable.rs:12`) is only a
`use` statement, so there is exactly zero incidental coverage.

### EF / ORM concerns

| Concern | Status |
|---|---|
| Entity mapping | **absent both trees.** Branch's `table!` declares SQL columns, not Rust entities |
| Keys | `table! users (id)` declares a PK for SQL text generation only |
| Relationships | absent. Branch SQL builder: JOINs explicitly out of Phase 1 scope |
| Change tracking / identity map / unit of work | **absent.** Ruled out — `D-003` |
| Transactions / connection pooling | absent; the SQL builder does not execute |
| Migrations | absent; delegated — `D-008` |
| Eager / lazy loading | absent; lazy loading permanently out — `D-004` |

---

## 4. Correctness, ergonomics, performance

### 4.1 The C#-equivalence claim is one defect wearing many hats

The README maps each Rust method to a named C# method; 20 of 50 rows diverge from
the C# behaviour they advertise (every C# claim below was verified against a live
`learn.microsoft.com` page). But an adversarial pass established that most of
those "divergences" are the **correct Rust choice** — the crate behaves exactly
like the `std` method it delegates to — and the defect is the assertion of
equivalence, not the behaviour:

| Behaviour | C# | Rust/`std` | Verdict |
|---|---|---|---|
| `to_hashmap` duplicate key | `ToDictionary` throws | `HashMap::insert` last-wins | **documentation** — matching std is right |
| `sum_` integer overflow | version-dependent | panics in debug, wraps in release, exactly like `Iterator::sum` | **documentation** |
| `order_by` on strings | culture-aware | byte-ordinal, exactly like `Ord for str` | **documentation** |
| `min_`/`max_` over `Option<T>` | skips nulls | `Ord for Option` puts `None` first, exactly like `Iterator::min` | **documentation** |
| `min_`/`max_`/`average` on empty | throws | `None` | **documentation** |
| `max_by_key_` ties | C# keeps first (settled from the dotnet/runtime source) | std keeps last | **documentation** |

The right fix is a single `# Differences from C# LINQ` section, not six code
changes. What is left after that filter is genuinely wrong:

- **`then_by` destroys the primary ordering. [main] CONFIRMED, critical.**
  `ordered.rs:42` sorts the whole buffer on the secondary key. Its own inline
  comment (`ordered.rs:41`) asserts the opposite, and so does its doc comment.
  Measured: `order_by(.0).then_by(.1)` on `[("a",2),("b",0),("b",1),("a",0)]`
  returns data sorted purely by `.1`. `order_by` alone *is* stable, so the bug is
  confined to `ThenBy`. `test_then_by` catches it exactly — and had never been
  compiled. **Fixed on the branch** as a comparator stack sorted once at
  `into_iter` time.
- **`union_` applies the set rule to one side only. [main] CONFIRMED, high.**
  `[1,1,2,2,3].union_([4])` → `[1,1,2,2,3,4]`: neither a set nor a faithful
  concatenation. `queryable.rs:202-208` dedups the `other` loop and copies the
  receiver unfiltered. No reading defends it. (`except`/`intersect` also skip
  dedup but their doc comments honestly describe them as filters — that is the
  documentation defect above, not this one.) **Fixed on the branch.**
- **`single_or_default` loses information. [both] CONFIRMED, high.** Returns
  `None` for both "empty" and "more than one", so a programmer error is
  indistinguishable from "not found". Its doc comment promises a panic the body
  does not contain. The idiomatic Rust answer is `Result`, or
  `itertools::exactly_one`/`at_most_one`.
- **Eleven operators bound equality on `PartialEq` where the algorithm needs an
  equivalence relation. [both] CONFIRMED, medium.** The visible consequence:
  `Lookup::get(&f64::NAN)` returns an empty slice for a key `count()` says
  exists. The fix is `Eq`, which is what `to_hashmap`/`to_hashset` already
  require and what `HashSet`/`HashMap`/`Itertools::unique` do — it turns every
  NaN case into a compile error. Note the branch preserved the `PartialEq`
  versions deliberately, "for types that can't `Hash` (e.g. floats)"
  (`ROADMAP.md` Phase 3) — but `order_by` binds `K: Ord`, so that same float
  audience cannot sort at all. The stated rationale does not hold up.
- **`Skip::next` loses an element on a non-fused source. [both] CONFIRMED,
  medium, new.** `adaptors.rs:90-94` never resets `remaining` when the inner
  iterator returns `None` mid-skip; `std::iter::Skip` zeroes it unconditionally.
  On `[10,20,None,40,50,60]` driven by hand, std yields `40` and this yields
  `50`. `collect()` masks it. One line to fix, and it is exactly the bug class
  the missing `FusedIterator` impls invite.
- **`chunk` has the only argument-reachable panics, and one is uncatchable.
  [both] CONFIRMED, high.** `chunk(0)` panics at `queryable.rs:125` with no
  `# Panics` section. `Vec::with_capacity(self.size)` (`adaptors.rs:298`)
  allocates on the *argument*, not the data: `chunk(2^28)` on three `u64`s
  reserves 2 GiB; `chunk(2^40)` gives `memory allocation of 8796093022208 bytes
  failed` / `Aborted (core dumped)`, exit 134 — `catch_unwind` cannot see it.
  Everything else saturates cleanly (`take_(usize::MAX)`, `skip(usize::MAX)`,
  `element_at(usize::MAX)`).
- **Seven operators evaluate at *construction*, not on first `next()`. [both]
  CONFIRMED, medium.** `order_by`, `order_by_descending`, `reverse`, `group_by`,
  `union_`, `join`, `group_join` drain the source at call time; `except` and
  `intersect` drain their `other` argument at call time even if the result is
  never iterated. Measured with a counting source. C# defers all of them to the
  first `MoveNext`, so `README:213`'s "This mirrors C# LINQ's behaviour" is wrong
  twice over. `distinct`, which the README lists as eager, provably streams — it
  terminates on an infinite source.

### 4.2 Ergonomics: the collisions are the real story

- **`LinqExt::skip` breaks unrelated code. [main] CONFIRMED, high.** Importing
  `LinqExt` makes every unqualified `.skip(n)` in that module a hard `E0034`,
  including on iterators that never touch the crate (`s.chars().skip(1)`,
  `m.values().skip(2)`). There is no `skip_` escape, so the name cannot be
  spelled in method position at all — the author already worked around this
  internally, calling `Iterator::skip(self, index)` in UFCS form at
  `queryable.rs:385`. Narrowing from the first pass: the blast radius is
  **module-scoped**, a function-scoped `use` confines it further, and both rustc
  `help:` lines compile verbatim. **Fixed on the branch** (`skip` → `skip_`).
- **`LinqExt` silently shadows `Itertools::join`. [both] CONFIRMED, high.** All
  48 methods take `self` by value, so `LinqExt` wins the by-value
  receiver-adjustment step against any competing extension trait whose method
  takes `&self`/`&mut self` — with **no ambiguity diagnostic**. Reproduced
  against itertools 0.14 and 0.15: a working `.join(", ")` becomes three
  unrelated errors totalling 37 lines that never mention `LinqExt`. Verified with
  two controls (a by-value/by-value pair *does* raise `E0034`; a
  by-value/by-ref pair does not), and import order is irrelevant. *(An earlier
  explanation attributing this to `Itertools::join`'s `Display` bound "filtering
  it out" is wrong — where-clause bounds do not filter method candidates. The
  conclusion held; the mechanism did not.)*
- **`LinqExt::group_by` collides with `Itertools::group_by`. [both] CONFIRMED,
  high, new.** `Itertools::group_by` still exists in 0.15 as a deprecated alias,
  and both take `self` by value, so this one is a loud `E0034` with no
  resolution short of UFCS. Independently confirmed by two agents. So any crate
  importing both traits — the overwhelmingly likely case for a Rust user reaching
  for LINQ operators — hits one silent breakage and one hard error.
- **Key selectors cannot borrow from the item when items are owned. [both]
  CONFIRMED, medium.** Ten methods take `FnMut(&Self::Item) -> K` with `K` free,
  so `group_by(|p| &p.dept)` after `.into_iter()` fails with a bare
  `error: lifetime may not live long enough` — no error code, no suggested fix.
  Narrowed from "impossible, no escape": iterating by reference (`.iter()`) makes
  `Self::Item = &T` and borrowed keys work for every affected method with zero
  clones, and `Iterator::max_by_key`/`slice::sort_by_key` emit a byte-identical
  diagnostic for the same idiom. It is a signature limitation shared with std,
  not a crate defect — but it is the load-bearing pre-1.0 API decision (§7).
- **`order_by` cannot sort by an `f64` key. [both] CONFIRMED, medium.** `K: Ord`,
  and the crate ships zero comparator-taking methods, so there is no escape
  hatch where std has `sort_by`/`min_by` and C# has an `IComparer` overload.
  Sorting by price is routine. A user-side `total_cmp` newtype works; the
  missing `*_by` variant costs about 11 lines.
- **`OrderedQueryable` is not an `Iterator`. [both] CONFIRMED, medium.** Every
  sorting chain needs a manual `.into_iter()`, and `then_by` needs a second
  import beyond `LinqExt`.
- **`pub use adaptors::*` hoists seven colliding names. [main] CONFIRMED,
  medium.** `Skip`, `SkipWhile`, `Take`, `TakeWhile`, `Zip`, `Flatten` collide
  with `std::iter::*` and `Reverse` with `std::cmp::*`. The deny-by-default
  `ambiguous_glob_imports` error fires when an ambiguous name is *used*, not on
  the imports alone.
- **Type inference is clean. CONFIRMED.** No turbofish or annotation was needed
  anywhere beyond what std demands, including `join`'s seven type parameters, and
  a lazy query can be returned from a function by naming its adaptor type.
- **A 20-operator chain compiles within noise of a std chain. CONFIRMED**, but
  the "shorter error message" result is a false positive: the error is shorter
  because `to_vec()` returns a concrete type and never names the offending
  operator, where std's longer output pinpoints it. Swap `to_vec()` for
  `collect()` and linq_rs's output grows to 79 lines.
- **The rustc fix-it for a missing operator can silently change semantics.
  CONFIRMED, medium, new.** `.take_last(3)` → `help: there is a method `take_`
  with a similar name`. Applying the machine-applicable suggestion **compiles**
  and returns `[1,2,3]` where C# `TakeLast(3)` returns `[8,9,10]`. rustc's
  matcher is semantics-blind and the trailing-underscore convention manufactures
  near-misses in exactly the operator family where the semantics invert.

### 4.3 Performance: one good result, one systemic one

Numbers below were produced by one harness and then independently reproduced by a
second, hostile one. Where the two disagreed, the **narrower** figure is quoted:
the first harness ran all benchmarks in one binary, which de-optimised its
`std`/`HashSet`/`HashMap` baselines and inflated every ratio. The linq-side
absolute numbers reproduced well; the ratios did not.

- **The hot path is genuinely fine. CONFIRMED.** `where_().select()` collected
  into a `Vec` runs at **1.00–1.16×** hand-written `filter().map()` and allocates
  nothing.
- **But internal iteration is not. CONFIRMED, medium.** No adaptor overrides
  `fold`/`try_fold`, so `sum`/`fold`/`aggregate`/`count` fall back to external
  `next()` and lose std's auto-vectorisation — up to **4.2×** slower. An earlier
  "linq is 0.72× i.e. faster" reading was an artifact of one non-vectorisable
  predicate and must not be quoted as a win.
- **Everything built on `Vec::contains` is quadratic. CONFIRMED, critical.**
  Independently fitted log-log slopes: `distinct` **2.06**, `group_by`/`to_lookup`
  **1.95**, `join` **1.97**. `distinct` at n=1,000,000 all-distinct takes
  **5–10 minutes** against ~0.2–0.35 s for a `HashSet` (~1,700–1,900×);
  `group_by` at 320,000 distinct keys takes **2–4 minutes** against ~0.1–0.2 s
  for a `HashMap`; `join` at 100,000×100,000 performs 1e10 comparisons in
  **4–5 s** against 35–115 ms for a hash join. The claim that the linear scan
  wins at low cardinality was **REFUTED** — it wins only below ~25 distinct
  values and is already 2× slower at 100.
- **The 100 ms crossovers cluster between n≈17,000 and n≈30,000.** Below ~5,000
  elements every quadratic operator is invisible. That is why none of these
  defects can ever surface in the crate's own doctests, examples, or demos.
- **`Lookup` is O(k) per access. CONFIRMED, high.** `get`, `contains_key` and
  `insert` are all linear scans (`lookup.rs:38-48`) — **288×** slower than
  `HashMap::get` at 10,000 keys, in the one type whose entire purpose is keyed
  random access.
- **`order_by().then_by()` sorted twice and cloned per comparison. [main]
  CONFIRMED, high.** 933,107 allocations at n=20,000 against 40,002 for an
  equivalent single-pass sort — arithmetically explained: 446,552 comparisons
  across two passes × 2 key invocations × one `String` clone each, plus the
  baseline. **Fixed on the branch** by the comparator-stack rewrite.
- **13 of 14 adaptors report `size_hint() == (0, None)`. [main] CONFIRMED,
  high.** `reverse().to_vec()` is **5.1×** slower than `rev().collect()` (8.4×
  net of the shared clone), of which 5.3× is attributable to the missing hint
  alone — proved with a control wrapper that hides only `size_hint`. Even
  `Reverse`, backed by a fully materialised `Vec` of known length, discards it.
  An earlier "10.3×" figure rested on an implausible baseline. **Branch: 9 impls
  added**, deliberately omitting the unpredictable ones.
- **The branch's answer to the quadratics was to add 10 `_hashed` twins and keep
  the quadratic defaults. CONFIRMED.** 12 linear-scan sites remain in the default
  paths, `lookup.rs` still scans, and the API now has two of every set operator.
  Every complexity finding above still applies to the method a user reaches for
  first.
- **Build cost and binary size are non-issues. CONFIRMED.** +207 ms on a clean
  release build; a 10-operator linq chain produced a stripped binary 928 bytes
  *smaller* than the std equivalent.

### 4.4 Code health

`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` is clean on `main` — all four
intra-doc links suspected of being broken resolve. Clippy at pedantic+nursery
yields 22 warnings across 14 lints, of which exactly one indicates a defect
(`union_`'s dead clone). No dead code beyond the orphaned test file. All 48
methods carry a doc comment; only 15 carry a runnable example.

Unexamined by the operator-level passes and confirmed by the completeness
critic — the three helper types were never audited *as APIs*:

- `Grouping` has `pub key` and `pub elements` **alongside** `key()`/`elements()`
  accessors, so the one-group-per-key invariant is unenforceable from outside; a
  `group_by` result can be mutated into two groups sharing a key. Note the
  perverse repair incentive: on the branch `#![warn(missing_docs)]` made those
  fields *documented* rather than private.
- `Lookup::default()` is a publicly reachable constructor for a permanently
  empty, un-fillable value (`insert` is `pub(crate)`). `Lookup` has `count()`
  where Rust says `len()`/`is_empty()`, no `IntoIterator`, no `FromIterator`, and
  derives `Debug, Clone` but not `PartialEq` — while `Grouping` does derive
  `PartialEq`, so you can `assert_eq!` two groupings but not two lookups.
- `OrderedQueryable` has **zero** public methods and no `Debug`/`Clone`/
  `PartialEq` — it cannot be printed in a test.
- `pub trait ThenBy<T>` is **unsealed with exactly one impl**, so a downstream
  crate can implement it and any added method is a breaking change. `LinqExt` is
  de facto sealed by its blanket impl; `ThenBy` is not.
- Zero doc examples outside `queryable.rs`: all 10 public methods on the three
  helper types and all 14 adaptors have none.

Packaging, both trees: `cargo package --list` **ships `linq_tests.rs`** — 421
lines of never-compiled code go to crates.io and docs.rs, with no `exclude`
field. Cargo prints `warning: manifest has no documentation, homepage or
repository` on every package, and that warning was ignored at publish time.
`README.md:27` tells consumers `linq_rs = { path = "." }`, which resolves to
themselves; it sits in a ```` ```toml ```` fence, so no doctest can catch it.
`main` has zero crate-level attributes, so "zero `unsafe`" is an observed
property of the current text, not an enforced invariant — the branch fixes that
with `#![forbid(unsafe_code)]`.

A genuine `no_std` measurement, since the question keeps recurring: `grouping.rs`,
`lookup.rs` and `lib.rs` use no `std::` paths at all; `adaptors.rs` uses only
`std::iter`/`std::vec`; `ordered.rs` only `std::vec`. Real `std` need is confined
to `to_hashmap`/`to_hashset`. An `alloc`-only build would cover 46 of 48 methods.
Recorded as measurement, not as a recommendation — `D-011` stands.

---

## 5. Positioning

All versions and statuses below were read off live sources on 2026-09-09, not
recalled.

| Library | Version | Released | Status | Approach |
|---|---|---|---|---|
| `itertools` | **0.15.0** | 2026-06-16 | dominant; repo pushed 2026-09-05; 1.48 B downloads | blanket `Itertools: Iterator`, ~146 methods — *the same architecture as this crate* |
| `std::iter::Iterator` | Rust 1.98.1 | 6-week train | — | ~75 provided methods, blanket |
| `diesel` | **2.3.13** | 2026-09-04 | active, single-maintainer-dominant | compile-time type-level query AST via `table!`; 3 backends |
| `diesel-async` | 0.9.2 | 2026-06-19 | active but 0.x, in a personal repo | same `QueryDsl`, async terminals; SQLite is a thread-pool shim |
| `sea-orm` | **2.0.2** | 2026-08-12 | active, tight cadence; 9.9 k stars | runtime-built schema-aware queries + `ActiveModel` |
| `sea-query` | 1.0.2 | 2026-08-12 | active | dialect-aware SQL AST — the closest thing to a LINQ expression tree in Rust |
| `sqlx` | **0.9.0** | 2026-05-21 | active, slow cadence; org moved to `transact-rs/sqlx` | raw SQL verified against a live DB at build time; "SQLx is not an ORM" |
| `rinq` | 0.1.0 | 2026-03-28 | one push, 0 stars, 27 downloads | type-state `QueryBuilder`, 121 methods, `pred!` + **`rinq_explain!`** |
| `rust-queries-core` | 1.0.8 | **2025-11-23** | **dormant** — repo last pushed 2025-10-11, 0 stars | type-safe query builder over in-memory collections, incl. hash joins |
| historical `linq`/`chain_linq`/`cute` | — | 2019 / 2021 / 2017 | all dead | `linq!` / `c!` query-comprehension macros |

*(One prior-art claim needed correcting: `rust-queries-core` was reported as
"updated 2026-09-01". It is not — the crate's last release is 2025-11-23 and the
GitHub repo's last push is 2025-10-11. It is a real crate that ships in-memory
joins, but it is dormant and unnoticed, not a live incumbent.)*

**What each does better.** `itertools` does everything this crate does in the
overlapping 44 methods, with better algorithms (`unique` is hash-based; this
crate's `distinct` is a linear scan) and better-shaped returns
(`at_most_one`/`exactly_one` return `Result` where `single_or_default` collapses
two outcomes into `None`). Diesel, SeaORM and sqlx all actually talk to a
database, which this crate does not — Diesel is 74,060 LOC of `src` plus 11,523
of derives against this crate's 5,738. sea-query already is the dialect-aware SQL
AST that a v2 renderer would otherwise have to build.

**What this crate could do better.** Exactly one thing, and it is real: **no
Rust library evaluates the same query expression either over an in-memory
collection or against a database.** CONFIRMED from primary sources: Diesel's
terminal operations all live on `RunQueryDsl` and take `conn: &mut Conn`, so a
Diesel chain cannot execute over a `Vec<T>` by construction; SeaORM's most
general extension point, `ProxyDatabaseTrait::query`, receives an
*already-rendered* SQL `Statement`, and its `MockDatabase` replays scripted
result sets rather than evaluating anything — so a wrong filter is invisible to a
SeaORM unit test; sqlx's compile-time checking dies the moment a query is
assembled at runtime (`QueryBuilder::build` documents "this API has no way to
check it for you", and issue #1488 asking for dynamic-query checking has been
open since 2021 with 40 reactions); polars and datafusion query a *columnar
frame*, not a `Vec<MyStruct>`, and neither converts from a struct collection;
rayon has no relational operator at all. sea-query's AST is one-directional — it
renders to SQL and nothing else.

### The honest differentiation statement

> Pick `linq_rs` over `itertools` when your problem is *relational* rather than
> combinatorial — a keyed equi-join, a left-outer group join, order-preserving
> grouping, or composable mixed-direction multi-key ordering — because
> `Itertools::join` builds a `String`, `merge_join_by` requires pre-sorted input
> and a comparator, and `into_group_map_by` returns a `HashMap` that loses group
> order. Do not pick it over Diesel or SeaORM at all: those are database
> libraries and this one has no I/O; the only thing this crate could hold that
> they cannot is one query expression with two interpreters, and today that seam
> is **vacant ground, not held ground**.

That statement is narrower than it looks, and the audit must say why. An
adversarial pass **compiled** std-only reproductions of every one of those
relational operators: `group_join` is one statement of `into_group_map_by` plus a
`map`, `join` is two, order-preserving grouping is three lines with `indexmap`,
and correct mixed-direction multi-key sorting is one line of
`sort_by(|a,b| ...then_with(...))`. So the differentiation is **convenience and
correctness, not capability**. That is a legitimate reason for a crate to exist —
it is most of why `itertools` exists — but it only holds if this crate's versions
are measurably *better* than the four-line hand-rolled alternative. Today they
are measurably worse: quadratic, `Clone`-demanding, and in `then_by`'s case
wrong. **The differentiation is conditional on the rewrite, not on the trim.**

### The C#-migrant thesis

**It does not currently hold, and `CLAUDE.md` states a goal that cannot be met as
written.** `CLAUDE.md:7-8` says the aim is to feel "**familiar to C# devs** and
**idiomatic to Rust devs** at the same time". Measured: a literal transcription of
`people.Where(x => x.Age > 30).OrderBy(x => x.Name).Select(x => x.Name).ToList()`
takes **three compile-fix cycles and five deviations**, and one of the fixes buys
a forced `String` clone on every comparison. `.Skip(2)` — the commonest LINQ
paging idiom — cannot be spelled in method position at all on `main`.
`.Contains(3)` becomes `.contains_(&3)`. 18 of 48 names carry a trailing
underscore, and four of those (`concat_`, `union_`, `contains_`, `is_empty_`)
collide with nothing at all, contradicting the crate's own stated naming rule at
`README:215`.

This is the uncanny valley the brief warned about: a C# developer still cannot
type what they know, and a Rust developer sees a worse `filter`. The two goals in
`CLAUDE.md` are in genuine tension and one of them has to be dropped explicitly.
Phase 0 already dropped familiarity (`D-005`); `CLAUDE.md` has not been updated
to match, which is the same drift as `A-1`.

---

## 6. Phase 4 — scenario results

| # | Scenario | Verdict | Evidence |
|---|---|---|---|
| 1 | C# dev writes their first query from LINQ muscle memory | **GAP** | 3 compile-fix cycles, 5 deviations, a forced per-element `String` clone; `.Skip(2)` unspellable on `main` |
| 2 | A query uses an unsupported operator | **GAP** (boundary undocumented; one fix-it silently wrong) | 30 probes, **all fail at compile time**, zero runtime failures. C# `System.Linq.Enumerable` = 75 names / 234 overloads / 44 comparer overloads; `main` covers 45 names and 0 comparer overloads. Zero hits for `unsupported\|not implemented\|out of scope` anywhere. `take_last(3)` → rustc suggests `take_(3)`, which **compiles and returns the wrong rows** |
| 3 | User filters on a value from user input: parameterized? | **NOT-APPLICABLE** (no SQL on `main`); **PASS** on the branch's builder; **GAP** on the panic surface | Branch renders `WHERE (name = ?)` with the injection payload in `params`, identifiers from `table!` literals. But `chunk(2^40)` → uncatchable abort, exit 134 |
| 4 | A query returns 10 million rows: streaming or materialisation? | **PASS** for a pure lazy chain; **GAP** the moment any eager operator is upstream | Pure `where_().select().take_(10)` streams in bounded memory. With an eager operator upstream all 10 M source elements are consumed and 80–158 MB materialised before `take_(10)` sees anything — and `distinct()` at 10 M distinct values does not merely allocate, it extrapolates to **hours** |
| 5 | Two entities loaded, one mutated, save called | **NOT-APPLICABLE** (no persistence); **GAP** on forward-compatibility | Zero `Rc<`/`RefCell`/`Arc<`/`Mutex` in either tree, so `D-003` is still cleanly reachable. But `to_hashmap` silently picks a winner between two reads of the same key — the exact shape of this scenario — and by-value `self` on all 48 methods means a future `Repository::to_vec(&self)` on an `Iterator` type is silently shadowed (measured: resolves to `LinqExt::to_vec`, the only signal being `warning: trait Repository is never used`) |
| 6 | A nullable column compared to a value: do in-memory and SQL agree? | **GAP** (in-memory half disagrees with both C# and SQL); SQL half **NOT-APPLICABLE** | Measured on `[Some(3),None,Some(1),None,Some(2)]`: `where_(|x| *x < Some(2))` → `[None, Some(1), None]` — **the NULL rows satisfy a `<` predicate**, because `Ord for Option` puts `None` first. In SQL they are dropped. `sum_` → `None`: one NULL annihilates the whole aggregate (C# skips nulls, SQL ignores them — three different answers for one operator). `order_by` ASC puts nulls **first**; PostgreSQL's default is `NULLS LAST` |
| 7 | Same query in-memory and against the DB: identical results? | **NOT-APPLICABLE** (no SQL execution path; no DB connected) | And **not achievable by default even in principle**: Rust `Ord for str` is byte-ordinal, C# `OrderBy` is culture-aware, PostgreSQL orders by the column's collation. Measured: `["a","B","c","D"]` → `["B","D","a","c"]`; canonically-equal NFC `é` and NFD `e\u{301}` land on **opposite sides of `"f"`** |
| 8 | A user chains twenty operators | **PASS** on compile ergonomics only | +2 % median full build vs an equivalent std chain. Qualifier: the measured chain contains `order_by`, `reverse` and `group_by`, so it passes at compile time while materialising the source three times at runtime — and its error message is shorter only because it never names the offending operator |

Two scenario results carry decisions the owner has not yet made; both are
recorded in `DECISIONS.md` as `OPEN`:

- **The v2 translation boundary (`D-102`).** EF Core already ran this
  experiment: silent client-side fallback before 3.0, documented as causing
  "unnoticed performance issues", changed to a runtime throw in 3.0. C# cannot do
  better because `IQueryable<T>` exposes the same method set regardless of
  provider. **Rust can**, and this crate accidentally proves it — the boundary is
  already 100 % compile-time, because operators are trait methods on concrete
  types. Recommended: **compile error, with an explicit one-token opt-in**
  (`.to_memory()`) that consumes the queryable and hands back a plain `Iterator`
  on which the full surface reappears. Corollaries: no partial translation, and
  provider-specific subsets are per-provider trait method sets, not runtime
  capability flags.
- **Three-valued logic (`D-103`).** Rust agrees with C# LINQ-to-Objects on
  equality (`None == None` is `true`) and null ordering, disagrees with both on
  aggregation, and PostgreSQL's default null ordering is **inverted** from Rust's.
  A v2 that promises "same query, same answer" would be lying. What can honestly
  be promised is a **stability class per operator**, declared per provider.

---

## 7. Gap analysis and roadmap

### 7.1 What is architecturally blocked, and by what

Two foreclosures are **compiler-verified**, and both are cheap to avoid now and
impossible to undo after a 1.0:

**B-1 — Eight `-> impl Iterator` return sites in trait position permanently seal
the three operators the v2 thesis needs most.** `join`, `group_join` and
`group_by` return RPITIT opaque types. A later trait cannot be called on them:
probing yields `error[E0599]: no method named 'sql' found for opaque type`. The
same eight sites are what pin the MSRV at exactly 1.75 with zero headroom.

**B-2 — The obvious v2 migration path does not compile.** Widening
`where_<P: FnMut(&Item) -> bool>` into `where_<P: IntoPredicate<Item>>` breaks
every existing call site: `error: implementation of 'FnMut' is not general
enough` — a bare closure passed to a method whose bound is an indirect trait
fails closure inference, and adding `for<'a>` to the blanket impl does not fix it.
**Therefore any v2 must be purely additive**: `where_` stays closure-only
forever, and a second method (`where_expr`) takes the typed form. Verified
compiling both ways.

**What is *not* blocked, contrary to the received wisdom.** The
`IQueryable`/`IEnumerable` seam does **not** require a proc macro or an IR at
this scale. A reviewer built a working prototype: **115 non-comment LOC, zero
dependencies, zero macros, 4/4 tests**, in which one query value yields both
`[1,2]` evaluated in memory *and*
`SELECT * FROM users WHERE (age > $1 AND dept = $2) ORDER BY dept ASC, age DESC
LIMIT 2` with `params = [I64(30), Str("eng")]`. The trick is that closure opacity
was never the real obstacle — the thing that needed to be transparent was the
*column reference inside* the closure. A `Field<T> { name, get: fn(&T) -> Val }`
descriptor pairs the column name with the accessor in one value, so the same value
feeds both interpreters, and a raw closure survives as an `Opaque` variant that
evaluates in memory and makes `to_sql()` return `Err(NotTranslatable)` rather than
silently doing something else. A derive macro would generate those `const` lines
and nothing else — which makes the expensive part exactly the part that can wait.

Two honest caveats on that prototype, from its own author: it materialises and
clones on every row, so it **did not solve** reconciling "the query is an
inspectable plan value" with "in-memory execution is a lazy zero-copy stream" —
and lazy streaming is the crate's one measurably excellent property. And its
`Val` is a dynamic enum, so comparing a `Str` to an `I64` silently yields `false`
instead of failing to compile; a properly typed `Field<T, V>` is materially
harder than 115 LOC.

### 7.2 The work list

Ordered by dependency. "Cell" names the feature-matrix row it fills.

| # | What | Why | Effort | Depends on | Cell |
|---|---|---|---|---|---|
| W-1 | Resolve `A-1`: pick one thesis and make `CLAUDE.md`, `ROADMAP.md`, `README.md` and `DECISIONS.md` agree | Four documents state four scopes. Nothing below can be sequenced until this is settled | **S** (a decision, not code) | — | — |
| W-2 | Yank `linq_rs` 0.1.0 | A build that breaks unrelated `.skip()` calls, returns wrong `then_by` results and silently shadows `Itertools::join` is publicly downloadable | **S** | — | — |
| W-3 | Delete the tracked `Cargo.lock`, or regenerate it as v3 | A fresh clone fails on the declared MSRV before compiling. `.gitignore` already intends this; the tracked file defeats it | **S** | — | — |
| W-4 | Merge the branch's **correctness and hygiene** work; do **not** merge its surface expansion or `src/sql/` yet | 263 green tests, the `then_by` rewrite, `skip_`, `union_`'s dead clone, `size_hint`, `src/` layout, CI, `#![forbid(unsafe_code)]` — all pure win | **M** | W-1, W-3 | most correctness rows |
| W-5 | Make CI run: trigger on all branches, add a test-count floor, and run doctests on the MSRV job (`cargo test --all-targets` excludes them) | The branch's CI has never executed and its MSRV job would have failed | **S** | W-4 | — |
| W-6 | Wire the README as doctests (`#![doc = include_str!("README.md")]`) and fix `linq_rs = { path = "." }` | No README block is compiled by any build; the flagship example passes only on degenerate data | **S** | W-4 | — |
| W-7 | Replace the C#-equivalence tables with a single `# Differences from C# LINQ` section, and delete "the full power of C# LINQ", "all lazy, zero-copy", and the whole eagerness bullet | The crate asserts C# equivalence dozens of times; ~6 of those assertions are individually false while the behaviour is individually correct | **M** | W-6 | all "matches C#" cells |
| W-8 | Fix `Skip::next` to zero `remaining` on inner exhaustion; add `FusedIterator` where applicable | Loses an element on a non-fused source | **S** | W-4 | Skip/Take row |
| W-9 | Make `chunk` stop trusting its argument for capacity; take `NonZeroUsize` or document `# Panics` | The only argument-reachable panics, one of them an uncatchable abort | **S** | W-4 | Chunk row |
| W-10 | Make the hash-backed algorithm the **default** for `distinct`, set ops, `group_by`, `to_lookup`, `join`, `group_join`; keep `PartialEq` variants only as explicitly-named `*_by`/`*_eq` escape hatches | The branch added `_hashed` twins and left the quadratic versions as the default a user reaches for first. 12 linear-scan sites remain | **M** | W-4, D-101 | Distinct / set ops / GroupBy / Join / Lookup rows |
| W-11 | Rebuild `Lookup` on a hash index; rename `count()`→`len()`, add `is_empty`, `IntoIterator`, `FromIterator`, `PartialEq`; make `Default` private or fillable | 288× slower than `HashMap::get`; `Lookup::default()` is a public constructor for an un-fillable value | **M** | W-10 | Lookup row |
| W-12 | Rename `join`→`inner_join` (or `join_on`) and `group_by`→`group_by_key`; add a test that imports `LinqExt` **and** `Itertools` in one scope and calls `.join(", ")`, `.skip(1)`, `.group_by(..)` | Two live collisions with the most-depended-on iterator crate in Rust: one silent, one hard `E0034` | **S** | W-4 | — |
| W-13 | Add comparator variants (`order_by_with`, `min_by_`, `max_by_`) so `f64` keys are sortable | `K: Ord` with no escape hatch, where std has `sort_by` and C# has an `IComparer` overload | **S** | — | OrderBy row |
| W-14 | Make `OrderedQueryable` implement `Iterator`, fold `then_by` into inherent methods, seal or remove the `ThenBy` trait | Every sorting chain needs a manual `.into_iter()` plus a second import; `ThenBy` is an unsealed one-impl public trait | **M** | W-4 | ThenBy row |
| W-15 | Make `Grouping`'s fields private; give the three helper types `Debug`/`Clone`/`PartialEq` and doc examples | The one-group-per-key invariant is unenforceable; `OrderedQueryable` cannot be printed in a test | **S** | W-14 | — |
| W-16 | Replace `single_or_default` with a `Result`-returning `single`, or delegate to `itertools` | Collapses "empty" and "more than one" into one value; its doc comment promises a panic that does not exist | **S** | — | Single row |
| W-17 | Add `exclude`, `repository`, dual licence + `LICENSE-APACHE`; add `#[must_use]` to every deferred return | `linq_tests.rs` ships to crates.io; the published crate has no source link; a discarded eager `order_by` emits no diagnostic where std's `Filter`/`Map` do | **S** | W-2 | — |
| W-18 | Publish the operator boundary as a **derived** table: implemented / delegated to std / will not be implemented (comparer overloads) | Zero mentions of the boundary anywhere, against a README claiming the full C# surface. Derive the counts at doc-build time | **M** | W-7 | unsupported-operator row |
| W-19 | Decide `D-101`…`D-105` (§7.4) before any 1.0 tag | Each is free now and breaking later | **S** each | W-1 | — |
| W-20 | *If and only if* the v2 thesis survives W-1: land the `Query`/`Field`/`Pred` seam as an additive second vocabulary, and resolve the laziness conflict first | The seam is the one defensible niche, and the prototype shows it is cheap — but it currently costs the streaming property | **L** | W-1, W-19, B-1, B-2 | DB column, all rows |
| W-21 | *If the v2 thesis does not survive:* move `src/sql/` to its own crate | A Diesel-shaped builder with its own vocabulary inside a LINQ crate is `A-1` in code form | **M** | W-1 | — |

### 7.3 Recommended v1.0 cut line

Three independent reviewers proposed cut lines from a portfolio angle, an
ecosystem-honesty angle, and a v2-seam angle. They disagreed about the name and
about `where_`/`select`, and **converged on everything else**: the relational
family is the core, the quadratic implementations must be rewritten before the
crate can claim anything, the 42 orphan tests are the most valuable asset in the
repo, and the alias surface has to go.

**Ship in 1.0 — the relational core, ~8–12 methods:**

- `inner_join` — hash equi-join, `K: Hash + Eq`, buffers `inner` into an index,
  streams `outer`, no `Clone` on the outer item.
- `left_join` — the shape C# itself lacks (`LeftJoin` is `GroupJoin` +
  `SelectMany` + `DefaultIfEmpty`), and the single strongest reason to add the
  dependency rather than write four lines.
- `group_join` — hash-bucketed; selector receives `&[Inner]`, not a cloned `Vec`.
- `group_by_key` → `impl Iterator<Item = Grouping<K, V>>`, hash-indexed,
  first-appearance order as a **tested invariant** (this is the one claim that
  survives contact with `itertools`, so it must be a test, not a sentence).
- `into_lookup` → a hash-backed `Lookup` with O(1) `get`.
- `order_by` / `order_by_descending` → an `Ordered<T>` that **implements
  `Iterator`**, with inherent `then_by`/`then_by_descending` built as one
  accumulated comparator and one sort — the branch's design.
- Optionally `chunk` and `reverse`, the two genuine small shape deltas over
  `itertools`, but only with `chunk`'s capacity bug fixed.

**Delete:** the ~31 std renames and the ~11 ≤3-line compositions, and with them
`adaptors.rs` in large part. That single deletion resolves ten separate findings
at once — 13 missing `size_hint`s, the absent `ExactSizeIterator`/
`DoubleEndedIterator`/`FusedIterator` impls, the missing `Clone`/`Debug`, the
`Reverse` phantom parameter, the `Distinct<I>` struct/impl bound mismatch, and the
`pub use adaptors::*` glob collisions — because those structs existed only to
support the methods being cut.

**Defer, named explicitly:** SQL translation (`W-20`/`W-21`); sort-merge join;
`*_by` comparator escape hatches for non-`Hash` keys (name them in the 1.0 docs so
adding them is additive); anti-join / semi-join; rayon and serde features;
`no_std`.

**The deliverable that does the real work is the README**, not the code: a table
mapping the C# LINQ surface onto Rust in which **most rows point away from this
crate**, to `std::iter` or `itertools`, with the handful that point here. That
page cannot be written by someone who does not know all three of std, itertools
and LINQ, and it converts the crate's largest liability — the alias surface — into
its strongest evidence of judgement. It must be **mechanically generated and
compile-tested**, not maintained as prose: one wrong row claiming `itertools`
lacks something it has does more damage than any missing method.

**Two dissents worth recording rather than resolving here.** (1) The
ecosystem-honesty reviewer argues the crate should be **renamed** — `linq` in the
name advertises the 42 methods being deleted and hides the 8 being kept — and
that the C#-migrant positioning should be dropped entirely because it teaches a C#
reader the wrong Rust. (2) The v2-seam reviewer argues `where_` and `select` must
be **kept**, because under a two-interpreter design they are not renames of
`filter`/`map` — they are clause constructors, and `std` has no concept of "a
clause that may or may not be a clause". Both dissents are downstream of `W-1`.
Settle the thesis and both resolve themselves.

### 7.4 API-stability decisions that must be settled before any 1.0

Each is free now and a breaking change later. All are recorded as `OPEN` in
`DECISIONS.md`.

- **`D-101` — Key bound: `Hash + Eq` or `PartialEq`?** Determines the complexity
  class of every hash-backed operator *and* whether `f64` keys compile at all.
  The current `PartialEq` choice is defended in `ROADMAP.md` as serving float
  keys, but `order_by`'s `K: Ord` means that audience cannot sort — and
  `Lookup::get(&NAN)` cannot find a key it just inserted. Recommend `Hash + Eq`
  with `*_by` variants named in the docs at 1.0.
- **`D-104` — Key-selector signature.** All key selectors are
  `FnMut(&Self::Item) -> K` with `K` free, so keys cannot borrow from owned items.
  Every shipped method takes a key selector, so this is unfixable after 1.0
  without breaking the whole API. Decide: HRTB/GAT form, `K: Borrow<..>`, or
  accept-and-document with the clone cost stated.
- **`D-105` — `Fn` vs `FnMut` on predicates and selectors.** Currently
  inconsistent: `order_by` binds `FnMut` (`queryable.rs:229`), `join` binds `Fn`
  (`:460`). A translator needs purity, and narrowing later is breaking — verified:
  a v1-legal counting closure against a v2 `Fn` bound gives `error[E0594]: cannot
  assign to 'calls', as it is a captured variable in a 'Fn' closure`. Bind `Fn`
  in anything the plan vocabulary might ever contain.
- **`Ordered<T>`'s traits.** Adding `Iterator`/`IntoIterator` later is
  non-breaking; removing them is. Add now.
- **`to_` vs `into_` on consuming conversions.** `to_lookup` consumes `self`
  against the convention; clippy does not catch it. Free now.
- **Seal `LinqExt` and `ThenBy`,** or declare them non-implementable. `LinqExt` is
  de facto sealed by its blanket impl; `ThenBy` is a public unsealed trait with
  one impl, so any added method is potentially breaking.
- **Do not export adaptor structs whose closure type sits in a user-nameable
  parameter position** if the wrapper might ever change — returning `Where<Self,P>`
  today and `Where<Self, ClosurePred<P>>` later breaks anyone who named the type,
  and naming adaptor types to return lazy queries is a pattern the crate's own
  docs demonstrate.

---

## 8. Findings table

Severity reflects the **post-verification** rating: four skeptics were tasked with
refuting the first pass, and several findings were demoted when the behaviour
turned out to be correct Rust that merely contradicted a doc sentence. Tree tags:
[m] `main` only (fixed on the branch), [b] branch only, [B] both.

| ID | Tree | Area | Sev | Conf | Finding | Evidence | Action | Eff |
|---|---|---|---|---|---|---|---|---|
| A-1 | [b] | scope | **crit** | CONFIRMED | Two disjoint products and four contradicting statements of intent | `CLAUDE.md:11-14` + `ROADMAP.md` Rejected vs `src/sql/` (1,104 LOC); `grep LinqExt\|Iterator src/sql/` = 1 doc-comment hit | Pick one thesis; make all four documents cite `DECISIONS.md` IDs instead of restating scope | S |
| A-2 | [B] | positioning | **crit** | CONFIRMED | Zero of 50 methods carry capability irreproducible in ≤4 statements of `std`; branch grew 48→90 | compiled std-only reproductions of all 6 bucket-C candidates; `join` = 2 statements, `group_join` = 1 | Cut to the relational core and rewrite it to be *better* than the 4-line alternative | L |
| A-3 | [B] | release | **crit** | CONFIRMED | The broken tree is published; the fixed tree has never run CI and cannot be published without a version bump | crates.io API: 0.1.0, not yanked, `repository: null`; branch `Cargo.toml` still `version = "0.1.0"`; workflow triggers only on `main`/`master`/PR | Yank 0.1.0; make CI trigger on all branches; bump | S |
| C-1 | [m] | correctness | **crit** | CONFIRMED | `then_by` destroys the primary ordering; its own comment and doc assert the opposite | `ordered.rs:41-42`; `order_by(.0).then_by(.1)` returns data sorted purely by `.1`; `test_then_by` catches it and never compiled | Already fixed on the branch (comparator stack, one sort at `into_iter`) — merge it | — |
| C-2 | [B] | perf | **crit** | CONFIRMED | `distinct`/set ops/`group_by`/`to_lookup`/`join` are quadratic; the branch left them as the defaults | fitted slopes 2.06 / 1.95 / 1.97; 100 ms crossover at n≈17k–30k; 12 linear-scan sites remain | Make hash-backed the default; `PartialEq` versions become named escape hatches | M |
| A-4 | [B] | robustness | high | CONFIRMED | `Cargo.lock` v4 breaks a fresh build on the declared MSRV 1.75 | ran `rustup run 1.75.0 cargo build` → lockfile parse error; builds clean once removed; `.gitignore` intends to ignore it but the file is tracked | Untrack the lockfile | S |
| E-1 | [B] | api-design | high | CONFIRMED | `LinqExt` silently shadows `Itertools::join` — by-value `self` wins receiver adjustment with no diagnostic | reproduced on itertools 0.14 and 0.15; 37 lines of unrelated errors; two controls isolate the mechanism | Rename to `inner_join`; add an interop regression test | S |
| E-2 | [B] | api-design | high | CONFIRMED | `LinqExt::group_by` hard-collides with `Itertools::group_by` (`E0034`) | `Itertools::group_by` still present in 0.15 as a deprecated alias; both by-value | Rename to `group_by_key` | S |
| E-3 | [m] | api-design | high | CONFIRMED | `LinqExt::skip` makes every unqualified `.skip(n)` in the module an `E0034`, with no `skip_` escape | reproduced on unrelated iterators; the author already used UFCS internally at `queryable.rs:385` | Fixed on the branch (`skip_`) — merge | — |
| C-3 | [m] | correctness | high | CONFIRMED | `union_` dedups `other` but not the receiver | `[1,1,2,2,3].union_([4])` → `[1,1,2,2,3,4]`; `queryable.rs:202-208` | Fixed on the branch — merge | — |
| C-4 | [B] | api-design | high | CONFIRMED | `single_or_default` collapses "empty" and ">1"; its doc promises a panic the body lacks | `queryable.rs:388-393`; C# `SingleOrDefault` throws on >1 (live MS docs, net-5.0…net-11.0) | Return `Result`, or delegate to `itertools::exactly_one` | S |
| R-1 | [B] | robustness | high | CONFIRMED | `chunk` allocates on its argument; `chunk(2^40)` is an **uncatchable** abort, and `chunk(0)`'s panic is undocumented | `adaptors.rs:298`, `queryable.rs:125`; exit 134, `catch_unwind` blind | Clamp capacity to the data; take `NonZeroUsize` | S |
| P-1 | [B] | perf | high | CONFIRMED | `Lookup` is O(k) per access — 288× `HashMap::get` at 10k keys | `lookup.rs:38-48`; two independent harnesses within 1.5 % | Hash index | M |
| P-2 | [m] | perf | high | CONFIRMED | 13 of 14 adaptors report `(0, None)`; `reverse().to_vec()` is 5.1× `rev().collect()` | control wrapper hiding only `size_hint` reproduces 5.3× of it | Branch adds 9 impls — merge | — |
| T-1 | [m] | test-coverage | high | CONFIRMED | 34 of 50 methods have zero executed coverage; the 42-test file was never a cargo target | `cargo metadata` shows one target; recounted twice, 16 covered | Fixed on the branch (263 tests) — merge, and add a CI test-count floor | S |
| D-1 | [B] | docs | high | DIVERGED | Dozens of C#-equivalence assertions, ~6 of them false while the behaviour is individually correct Rust | `to_hashmap`, `sum_`, `order_by` strings, `Option` aggregates, empty-sequence, `MaxBy` ties — all match the delegated `std` method exactly | One `# Differences from C# LINQ` section; delete the equivalence claims | M |
| D-2 | [B] | docs | high | DIVERGED | README's laziness model is wrong in five places and self-contradicting on naming | `distinct` provably streams on an infinite source; `group_by` missing from the eager list; `skip` unsuffixed on the same page as the suffix rule | Derive the eagerness table from tests; delete "all lazy, zero-copy" | M |
| C-5 | [B] | correctness | med | CONFIRMED | Eleven operators bound equality on `PartialEq` where the algorithm needs `Eq`; `Lookup::get(&NAN)` cannot find its own key | measured; `to_hashmap`/`to_hashset` already require `Eq`, as do `HashSet`/`Itertools::unique` | Move to `Eq`; the NaN cases become compile errors | M |
| C-6 | [B] | correctness | med | CONFIRMED | `Skip::next` loses an element on a non-fused source | `adaptors.rs:90-94` vs `std::iter::Skip`; hand-driven `next()` on `[10,20,None,40,50,60]` | Zero `remaining` on exhaustion; add `FusedIterator` | S |
| C-7 | [B] | correctness | med | CONFIRMED | Seven operators evaluate at construction, not first `next()`; two drain `other` even if unused | counting-source measurement; C# defers all of them | Document the real eager set; drop the "mirrors C#" claim | S |
| E-4 | [B] | ergonomics | med | CONFIRMED | Key selectors cannot borrow from owned items; the diagnostic has no error code and no fix | `error: lifetime may not live long enough`, 11 lines. Narrowed: `.iter()` works, and std emits the identical diagnostic | Settle `D-104` before 1.0 | M |
| E-5 | [B] | ergonomics | med | CONFIRMED | `order_by` binds `K: Ord` with no comparator escape, so `f64` keys cannot sort | grepped: no `Ordering`-taking parameter anywhere | Add `order_by_with` (~11 lines) | S |
| E-6 | [B] | ergonomics | med | CONFIRMED | `OrderedQueryable` is not an `Iterator`; `then_by` needs a second import | `ordered.rs:56-62` | Implement `Iterator`; make `then_by` inherent | M |
| E-7 | [B] | ergonomics | med | CONFIRMED | rustc's fix-it for `take_last` suggests `take_`, which compiles and returns the wrong rows | applied the machine-applicable suggestion: `[1,2,3]` vs C#'s `[8,9,10]` | Implement `take_last`/`skip_last` (branch does) or drop `take_` per `D-005` | S |
| P-3 | [B] | perf | med | CONFIRMED | No adaptor overrides `fold`/`try_fold`, losing std's internal iteration — up to 4.2× on `sum`/`fold` | narrowed from a first pass that reported this as *faster*; that reading was a non-vectorisable-predicate artifact | Override `fold`/`try_fold` on the adaptors that survive the cut | M |
| H-1 | [B] | code-health | med | CONFIRMED | The three helper types were never designed as APIs: public `Grouping` fields defeat the one-key invariant; `Lookup::default()` is un-fillable; `OrderedQueryable` has no methods and no `Debug` | `grouping.rs:10-11`, `lookup.rs:62-66`, `ordered.rs:9`; probe mutates a `group_by` result into two same-key groups | Private fields, standard derives, doc examples | S |
| H-2 | [B] | code-health | med | CONFIRMED | `pub trait ThenBy` is unsealed with one impl | `ordered.rs:21` | Seal or fold into `Ordered<T>` | S |
| H-3 | [B] | packaging | med | CONFIRMED | `cargo package` ships the 421-line orphan test file; no `exclude`; no `repository`; MIT-only against the dual-licence ruling | `cargo package --list`; cargo's own ignored `warning: manifest has no documentation, homepage or repository` | Add `exclude`, `repository`, `LICENSE-APACHE` | S |
| H-4 | [B] | code-health | med | CONFIRMED | Zero `#[must_use]` anywhere, so a discarded eager `order_by` — which allocates and sorts — emits no diagnostic where std's `Filter`/`Map` do | `grep must_use` → nothing; measured, std warns twice and this crate zero times | Add `#[must_use]` to every deferred return | S |
| H-5 | [m] | code-health | med | CONFIRMED | `pub use adaptors::*` hoists 7 names colliding with `std::iter`/`std::cmp` | `lib.rs:27`; `ambiguous_glob_imports` is deny-by-default once a name is used | No glob re-exports; name every public item | S |
| H-6 | [B] | code-health | med | CONFIRMED | One linear-scan primitive is hand-written at eight sites; `group_by`'s body is character-identical to `Lookup::insert` | `queryable.rs:534-545` vs `lookup.rs:21-29` | Extract once, or delete with `W-10` | S |
| E-8 | [m] | api-design | med | CONFIRMED | `concat_`'s `IntoIter = Self` bound makes it unusable mid-chain | narrowed: head-of-chain same-container calls do work; any closure-carrying adaptor fails (`E0271`) | Bind `IntoIterator<Item = Self::Item>` and chain | S |
| E-9 | [B] | api-design | med | CONFIRMED | `select_many` binds `J: Iterator` while `zip_` binds `IntoIterator` — internal inconsistency | `queryable.rs:65-68` vs `:608-611`; failure cascades into a second `E0599` | Bind `IntoIterator` | S |
| B-1 | [B] | architecture | high | CONFIRMED | Eight RPITIT sites permanently seal `join`/`group_join`/`group_by` against any future trait, and pin MSRV at exactly 1.75 | `error[E0599]: no method named 'sql' found for opaque type` | Return named types for anything the v2 seam must reach | M |
| B-2 | [B] | architecture | high | CONFIRMED | Widening a closure bound into a trait bound breaks every existing call site, so v2 must be purely additive | `error: implementation of 'FnMut' is not general enough`; `for<'a>` does not fix it | Record as a constraint (`D-102`); plan `where_expr`, never a widened `where_` | S |
| M-1 | [B] | robustness | med | CONFIRMED | The declared MSRV was never actually exercised: the branch's msrv job runs `cargo test --all-targets`, which excludes doctests, and never ran at all | verified by running 1.75 directly | Run doctests on the MSRV job | S |
| D-3 | [B] | docs | med | CONFIRMED | The operator boundary is undocumented against a README claiming the full C# surface (45 of 75 names, 0 of 44 comparer overloads) | zero hits for `unsupported\|not implemented\|out of scope` in any source or the README | Derived boundary table (`W-18`) | M |
| N-1 | [B] | correctness | med | CONFIRMED | `Option<T>` flows in ways that agree with C# LINQ-to-Objects but disagree with SQL, undocumented: `where_(\|x\| *x < Some(2))` **keeps** the NULLs; `sum_` annihilates on one `None` | measured on `[Some(3),None,Some(1),None,Some(2)]` | Document; settle `D-103` before any v2 | S |
| I-1 | [B] | docs | low | CONFIRMED | `README:27` tells consumers `linq_rs = { path = "." }` | in a ```` ```toml ```` fence, so no doctest can catch it | `linq_rs = "0.2"` | S |
| I-2 | [B] | docs | low | CONFIRMED | Four trailing underscores (`concat_`, `union_`, `contains_`, `is_empty_`) collide with nothing, contradicting the crate's own naming rule | compiled without the suffix, no error | Resolve under `D-005` | S |

**Not applicable, stated once:** SQL injection on `main` — no SQL, no query-string
construction, no I/O of any kind (CONFIRMED, zero hits across all six library
files and the README). On the branch the SQL builder does construct query text,
and its posture is **safe by construction** — values are `?` placeholders bound
into `params`, identifiers come from `table!` literals — but the verdict now has
to be re-derived per release rather than dismissed.

---

## 9. DO-NOT-BUILD

Features that sound natural for a LINQ/EF port and are bad fits for Rust or for
this project's scope. Recorded so they stop resurfacing. Each is also a
`DECISIONS.md` entry; cite the ID, not this list.

| Do not build | Why | Ledger |
|---|---|---|
| An EF-style identity map with `Rc<RefCell<_>>` / `Arc<Mutex<_>>` entities | Requires a shared mutable aliased object graph, which Rust refuses. Leaks into every user signature, makes every future `!Send`, and converts aliasing bugs into runtime panics | `D-003` |
| Lazy loading / navigation properties | Field access performing I/O needs interior mutability or a hidden global, and is the largest single source of N+1 pathologies in real EF codebases | `D-004` |
| Migrations | A schema-diff engine, a version ledger and a CLI, sharing almost no code with query translation | `D-008` |
| A `linq!` / `from…where…select` query-comprehension macro | Three Rust crates tried it (`linq` 2019, `chain_linq` 2021, `cute` 2017) and **all three are dead**, while the extension-trait crate in the space has 1.48 B downloads. `ROADMAP.md` already rejects it — this entry is for when it comes back | `D-201` |
| Pluggable `IEqualityComparer<T>` / `IComparer<T>` per call (44 C# overloads) | Rust expresses this with traits and newtypes. `ROADMAP.md` §2.4 already rejects it correctly. The *legitimate* subset is `*_by` key/comparator variants — those are `W-13`, not this | `D-202` |
| Full C# semantic fidelity | Six separate audit findings dissolved on inspection into "this behaves exactly like the `std` method it delegates to". Chasing `ToDictionary`-throws, checked `Sum`, or culture-aware string ordering would make the crate *worse* Rust to match a foreign contract, and culture-aware collation needs an ICU dependency the zero-dep rule forbids | `D-203` |
| Methods that merely rename an `Iterator` or `Itertools` method | ~31 of 48 on `main`; 8 call the std method one line down. This is the crate's single largest liability, and it costs measurable performance as well as credibility | `D-005` |
| `TryInto`-based `cast::<U>()` that panics on the first bad element | On the branch, faithful to C# `Cast<T>` and therefore wrong for Rust: a fallible conversion that panics on data, with no `Result` alternative. C# needs it for runtime downcasting, which Rust does not have | `D-204` (new) |
| Two vocabularies for one concept in one crate | `LinqExt::where_` and `sql::filter` mean the same thing and share no value. This is `A-1` in code form; the SQL builder either becomes the seam or becomes its own crate | `D-205` (new) |
| A second implementation of every operator (`*_hashed` twins) | Doubles the surface to avoid a breaking change, and leaves the slow version as the default a user reaches for first. Pick the right default and make the other an explicitly-named escape hatch | `D-206` (new) |
| Silent in-memory fallback at a v2 translation boundary | EF Core shipped exactly this, documented it as causing "unnoticed performance issues", and reversed it in 3.0. Rust can make the boundary a compile error, which C# cannot | `D-102` |
| Partial translation ("translate the prefix, evaluate the suffix") | The seam where EF Core's own memory-leak documentation lives | `D-102` |
| `async` support before the sync seam exists | There is no async surface in either tree. Diesel's async story is a separate 0.x crate in a personal repo with a thread-pool SQLite shim — the lesson is to make the execution seam pluggable, not to fork | `D-006` |
| `no_std` | Measured: only `to_hashmap`/`to_hashset` genuinely need `std`, so it is *achievable* — which is exactly why it will keep coming up. It has no user | `D-011` |
| Benchmarks via nightly `test::Bencher`, or `criterion` in the main crate | `ROADMAP.md` deferred benchmarks for this reason and was right. The way out is a workspace with a `benches/` member, not a dev-dependency on the library | `D-207` (new) |

---

## 10. Could not verify

- **Whether crates.io user `TRget88` is the owner's account.** Inferred from the
  display name "Kirk" and byte-identical published docs. Registry ownership cannot
  be confirmed from outside, and **every crates.io recommendation in this report
  depends on it**. If it is not the owner's account, the correct action is an
  ownership dispute, not a version bump.
- **The DB half of every scenario.** No database was connected (the owner has one
  available on a Synology but did not connect it), and more fundamentally there is
  no execution path to exercise: the branch's SQL builder produces strings and
  explicitly does not connect. Scenarios 3, 6 and 7 are therefore resolved by
  design analysis from primary documentation, not by execution.
- **`rinq`'s internals.** Its operator count, type-state machine and
  `rinq_explain!` macro are taken from its published documentation and crates.io
  metadata; its source was not audited. It is by `kazuma0606`, not the owner
  (owners endpoint checked), so the same-day publication is coincidence.
- **`MinBy`/`MaxBy` tie-breaking in C#.** `learn.microsoft.com` does not document
  it; the behaviour was settled from the `dotnet/runtime` source, which is
  evidence about an implementation rather than a contract.
- **Whether `#[inline]` closes the ~1.5× `Where::next` gap** attributed to loop
  shape. Plausible, untested.
- **The branch's 42 new operators.** They were inventoried and their tests run
  green, but they were not individually graded against the C# methods their doc
  comments name — the operator-level differential covered `main`'s 50. If the cut
  line in §7.3 is adopted most of them are deleted, which is why this was not
  pursued; if it is not adopted, they need the same treatment `main`'s 50 received.
- **Whether the branch's `src/sql/` output is correct SQL beyond its 28 tests.**
  No dialect conformance testing was done and no server ever parsed the output.
- **Demand.** Nothing in this audit measures whether anyone wants an iterator
  equi-join in Rust. Every positioning argument here is about *defensibility*, not
  about users. The most likely outcome of a well-executed §7.3 is a correct
  8-method crate with single-digit downloads, and that should be an accepted
  outcome rather than a surprise.
