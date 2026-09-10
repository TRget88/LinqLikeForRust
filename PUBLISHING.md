# Publishing

Operational steps only. The *rulings* behind them live in `DECISIONS.md`
(`D-009` publishing prerequisites, `D-012` licence, `D-022` the sibling's
tarball). This file is excluded from both tarballs.

## What "creating the crates.io project" actually is

There is no project to create ahead of time. **The first successful
`cargo publish` creates the crate and claims the name**, permanently, for the
account that ran it. There is no separate registration step and no way to
reserve a name.

Two consequences worth internalising before running anything:

- **A version number can never be reused or replaced.** `cargo yank` only stops
  *new* dependency resolution from selecting it; the tarball stays downloadable
  forever, and `0.2.0` can never be uploaded twice. A mistake costs a version
  number, not a fix.
- **Ownership is per-crate.** Owning `linq_rs` does not reserve `linq_rs_sql`,
  and crates.io normalises `-` and `_`, so `linq_rs_sql` and `linq-rs-sql` are
  the same name.

## Current state (checked 2026-09-10)

| Crate         | On crates.io                        | To publish |
|---------------|-------------------------------------|------------|
| `linq_rs`     | `0.1.0`, live, **not yanked**, 25 downloads | `0.2.0` |
| `linq_rs_sql` | does not exist — name is free       | `0.1.0` |

## Order is not optional

`linq_rs_sql` dev-depends on `linq_rs = { path = "..", version = "0.2" }`
(`D-022`). Cargo resolves dev-dependencies at publish time, so until
`linq_rs 0.2.0` is live on crates.io, `cargo package -p linq_rs_sql` and
`cargo publish -p linq_rs_sql` both fail with:

```
failed to select a version for the requirement `linq_rs = "^0.2"`
```

That is expected, not a defect. Publish `linq_rs` first.

`cargo package --list` does *not* resolve dependencies, so the packaging gate
keeps working before either crate is published.

## Steps

### 0. Land the branch first

The tarball embeds `.cargo_vcs_info.json` — the commit it was built from. If you
publish from `feature/v0.2.0-remediation`, that pointer is to a branch you will
later delete, and the published artifact loses its path back to source. That was
one of the `0.1.0` defects (`AUDIT.md` A-3).

```bash
git checkout main && git merge --no-ff feature/v0.2.0-remediation && git push origin main
```

Let CI go green on `main` before continuing.

### 1. Authenticate

Get a token at <https://crates.io/settings/tokens>. Scope it to `publish-update`
if you are offered scopes; it does not need `yank` until step 4.

```bash
cargo login
```

Paste the token at the prompt — do not pass it as an argument, where it lands in
your shell history.

### 2. Publish `linq_rs` 0.2.0

Dry run, then the real thing:

```bash
cargo publish --dry-run -p linq_rs
```

```bash
cargo publish -p linq_rs
```

### 3. Publish `linq_rs_sql` 0.1.0 — this claims the name

Wait for the index to carry `linq_rs 0.2.0` (usually under a minute), then:

```bash
cargo publish --dry-run -p linq_rs_sql
```

```bash
cargo publish -p linq_rs_sql
```

### 4. Yank `0.1.0`

Only now — yanking first would leave the crate with no usable version in the
window between the two commands.

```bash
cargo yank --version 0.1.0 linq_rs
```

`0.1.0` shipped a `then_by` that discarded the primary sort key and a `skip`
that turned every unqualified `.skip(n)` in an importing module into a compile
error (`D-009`, `AUDIT.md` A-3).

### 5. Flip the README note

`README.md` currently says `0.1.0` "is being yanked … update this note to
'yanked' once that has run, **not before**." Do that now, and only now — the
note is deliberately written so the repo never claims a yank that has not
happened.

### 6. Tag

```bash
git tag -a v0.2.0 -m "linq_rs 0.2.0" && git push origin v0.2.0
```

## Before any of it: run the gates

```bash
./.github/scripts/packaging-gate.sh && ./.github/scripts/release-gate.sh && ./.github/scripts/test-count-floor.sh
```

`release-gate.sh` refuses a `v1.*` tag while any `D-1xx` decision is `OPEN`.
All eight are settled, so it passes — this is a `0.x` release either way.

## After: docs.rs

docs.rs builds automatically from the uploaded tarball; nothing to trigger. Both
crates set `rust-version = "1.65"` and have zero runtime dependencies, so there
is little to fail. If a build does fail, it is fixed by publishing a new patch
version — never by re-uploading.
