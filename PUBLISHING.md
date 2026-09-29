# Publishing

Operational steps only. The *rulings* behind them live in `DECISIONS.md`
(`D-009` publishing prerequisites, `D-012` licence, `D-022` the sibling's
tarball, `D-023` the tarball is the artifact). Excluded from both tarballs.

Every command below was run on this repo with cargo 1.96.0 before being written
down. Where a command's behaviour is surprising, the actual output is quoted.

## What "creating the crates.io project" is

There is no project to create and nothing to reserve. crates.io has **one**
publish endpoint (`PUT /api/v1/crates/new`) for both new crates and new
versions; **the first successful `cargo publish` creates the crate and makes
you its sole owner**, permanently.

- **A version number can never be reused or replaced.** `cargo yank` only stops
  *new* resolution from selecting it; the tarball stays downloadable forever.
  A mistake costs a version number, not a fix.
- **Ownership is per-crate.** Owning `linq_rs` grants nothing over
  `linq_rs_sql`. crates.io has no namespacing or prefix concept.
- **`linq_rs_sql` and `linq-rs-sql` are the same name** to crates.io, which
  canonicalises as `replace(lower(name), '-', '_')` under a unique index.
  Publishing either permanently blocks the other. Normalisation does **not**
  extend to dependency resolution, though — consumers must write the underscore
  form or resolution fails.

## Current state (verified 2026-09-10, after release)

**Both crates are published.** The runbook below is for the **next** release; the
completed 2026-09-10 run is recorded at the end.

| Crate         | On crates.io                                      |
|---------------|---------------------------------------------------|
| `linq_rs`     | `0.2.0` live; `0.1.0` **yanked** 2026-09-10       |
| `linq_rs_sql` | `0.1.0` live — name claimed 2026-09-10T19:33:58Z  |

Both are `MIT OR Apache-2.0`, declare `rust-version = "1.65"`, carry the
`repository` link that `0.1.0` lacked, and built cleanly on docs.rs. Sole owner
of both: `TRget88` (crates.io user id 401874).

## The token

Mint at <https://crates.io/settings/tokens>.

**For the next release, `publish-update` alone is enough** — both crate names
already exist. `publish-new` was needed only the first time, and the two are not
interchangeable: From crates.io's own publish handler, an existing crate
takes `PublishUpdate` and a new one takes `PublishNew`, so a `publish-update`
token **cannot** create the second crate. A crate-scope pattern of `linq_rs*`
covers both; patterns match by prefix, so it covers `linq_rs_sql` before it
exists.

**Trusted Publishing cannot be used for step 5.** crates.io rejects it with
`"Trusted Publishing tokens do not support creating new crates. Publish the
crate manually, first"`. Move to it afterwards if you want to stop holding a
long-lived token.

## Order no longer matters

Both crates have **zero dependencies of any kind** (`D-032`, which extends
`D-024`), so neither blocks the other. Publish them in any order, together or months apart. Verified:
`cargo publish --dry-run -p linq_rs_sql` succeeds with `linq_rs 0.2.0` not yet
on crates.io.

This was not always true. Until 2026-09-10 the sibling dev-depended on
`linq_rs = { version = "0.2" }`, and because `cargo package` strips a dev-dep's
`path` but keeps its `version`, `linq_rs_sql` could not be packaged at all until
`linq_rs 0.2.0` was live. If that error ever returns —

```
failed to select a version for the requirement `linq_rs = "^0.2"`
```

— a dependency has crept back in; `packaging-gate.sh` should have caught it.

## The next release

`linq_rs 0.2.1` and `linq_rs_sql 0.2.0`. Everything below is un-run. Order does
not matter (see above) and nothing is published until you say so.

**Before anything:** the work is spread across a stack of branches, none merged.
Publishing from an unmerged branch leaves `.cargo_vcs_info.json` pointing at a
commit that may be deleted — that was one of the original `0.1.0` defects. Land it
first.

### 1. Land the stack on `main`

Bottom-up, so each merge is a fast-forward. Yours to run; I do not merge.

```bash
gh pr view 2
```

### 2. Run every gate on `main`

```bash
./.github/scripts/test-count-floor.sh && ./.github/scripts/packaging-gate.sh && ./.github/scripts/msrv-tarball.sh && ./.github/scripts/release-gate.sh && python3 .github/scripts/gen-docs.py --check && python3 .github/scripts/like-differential.py
```

`msrv-tarball.sh` is the one that matters: a green repo does not imply a green
tarball (`D-023`).

### 3. Authenticate

```bash
cargo login
```

Paste the token at the prompt. Scopes: `publish-update` is enough — both crate
names already exist, so `publish-new` is not needed this time.

### 4. Dry-run the workspace

```bash
cargo publish --dry-run --workspace
```

Expect both crates packaged, each followed by `warning: aborting upload due to dry
run`. `seam-tests` is `publish = false` and is skipped silently.

### 5. Publish — IRREVERSIBLE

```bash
cargo publish -p linq_rs
```
```bash
cargo publish -p linq_rs_sql
```

`-p` is mandatory: a bare `cargo publish` at the root selects only the default
members and silently publishes `linq_rs` alone.

### 6. Tag per crate

```bash
git tag -a linq_rs-v0.2.1 -m "linq_rs 0.2.1" && git tag -a linq_rs_sql-v0.2.0 -m "linq_rs_sql 0.2.0"
```
```bash
git push origin linq_rs-v0.2.1 linq_rs_sql-v0.2.0
```

### 7. Move the CHANGELOG's `[Unreleased]` under a dated heading

`CHANGELOG.md` uses dated release headings because the two crates version
independently. After publishing, add
`## [<date>] — linq_rs 0.2.1, linq_rs_sql 0.2.0` above what is currently
`[Unreleased]`, and leave `[Unreleased]` empty.

### 8. Verify

```bash
curl -sS -A "your-email@example.com" https://crates.io/api/v1/crates/linq_rs_sql | python3 -m json.tool | head -30
```

Use the **API**, not the HTML page — crates.io returns 404 to any non-browser
client, so a `curl` of `/crates/linq_rs_sql` 404s even for a published crate.

---

## Record: the 2026-09-10 release

`linq_rs 0.2.0` and `linq_rs_sql 0.1.0`, published from `fae7a35`. All steps
completed: branch pushed, merged to `main`, authenticated, both crates published,
`linq_rs 0.1.0` yanked, the README's yank note flipped afterwards (it was written
to instruct its own replacement, so the repo could never claim a yank that had not
happened). Tags `linq_rs-v0.2.0` and `linq_rs_sql-v0.1.0`.

Two things that release taught, both now gated rather than remembered:

- `cargo package` always writes a lockfile into the tarball, so an untracked
  `Cargo.lock` shipped v4 against a declared MSRV of 1.65 and the artifact could
  not build on its own floor (`D-023`).
- The published crate had no `repository` link, so there was no path from the
  artifact back to source (`AUDIT.md` A-3).

## After: docs.rs

docs.rs builds automatically from the uploaded tarball; nothing to trigger. Both
crates have zero runtime dependencies and now build on 1.65, so there is little
to fail. A failed docs build is fixed by publishing a new patch version — never
by re-uploading.
