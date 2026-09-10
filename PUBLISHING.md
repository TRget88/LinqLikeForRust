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

## Current state (verified 2026-09-10)

| Crate         | On crates.io                                | To publish |
|---------------|---------------------------------------------|------------|
| `linq_rs`     | `0.1.0`, live, **not yanked**, 25 downloads | `0.2.0` |
| `linq_rs_sql` | does not exist — name is free               | `0.1.0` |

Sole owner of `linq_rs`: `TRget88` (crates.io user id 401874), confirmed via
`GET /api/v1/crates/linq_rs/owners`.

## The token

Mint at <https://crates.io/settings/tokens> with **both**:

- `publish-new` — required to *create* `linq_rs_sql`
- `publish-update` — required for `linq_rs 0.2.0`

Not interchangeable. From crates.io's own publish handler, an existing crate
takes `PublishUpdate` and a new one takes `PublishNew`, so a `publish-update`
token **cannot** create the second crate. A crate-scope pattern of `linq_rs*`
covers both; patterns match by prefix, so it covers `linq_rs_sql` before it
exists.

**Trusted Publishing cannot be used for step 5.** crates.io rejects it with
`"Trusted Publishing tokens do not support creating new crates. Publish the
crate manually, first"`. Move to it afterwards if you want to stop holding a
long-lived token.

## Order is not optional, and cargo enforces it

`linq_rs_sql` dev-depends on `linq_rs = { path = "..", version = "0.2" }`
(`D-022`). `cargo package` strips the `path` and keeps the `version`, turning it
into a hard registry requirement. So **`cargo publish -p linq_rs_sql` fails
until `linq_rs 0.2.0` is live**:

```
error: failed to prepare local package for uploading
Caused by: failed to select a version for the requirement `linq_rs = "^0.2"`
  candidate versions found which didn't match: 0.1.0
```

That is expected, not a defect.

**`--workspace` is the exception.** `cargo publish --dry-run --workspace`
succeeds today, because cargo builds a temporary local overlay registry so
members resolve against their siblings. It is the only meaningful pre-flight
before anything is published.

## Steps

Run everything from the repo root.

### 1. Push the branch — do this first, today

```bash
git push -u origin feature/v0.2.0-remediation
```

31 commits exist on no remote ref, and they contain the entire `linq_rs_sql`
crate. Neither remote branch contains a single `linq_rs_sql` path. Nothing
remote could rebuild this.

### 2. Land it on `main`

```bash
git checkout main
```
```bash
git merge --no-ff feature/v0.2.0-remediation
```
```bash
git push origin main
```

The tarball embeds `.cargo_vcs_info.json` — the commit it was built from.
Publishing from a branch you later delete leaves the artifact with no path back
to source, which was one of the `0.1.0` defects (`AUDIT.md` A-3). This also
un-404s `README.md`'s link to `tree/main/linq_rs_sql`.

Let CI go green on `main` before continuing.

### 3. Run the gates

```bash
./.github/scripts/packaging-gate.sh && ./.github/scripts/msrv-tarball.sh && ./.github/scripts/release-gate.sh && ./.github/scripts/test-count-floor.sh
```

`msrv-tarball.sh` is the one that matters here: it packages with stable,
extracts every `.crate`, and builds it on 1.65. A green repo does not imply a
green tarball — see `D-023`.

### 4. Authenticate

```bash
cargo login
```

Paste the token at the prompt — do not pass it as an argument, where it lands in
shell history. It is stored **unencrypted** at `~/.cargo/credentials.toml`.

### 5. Dry-run the whole workspace

```bash
cargo publish --dry-run --workspace
```

Expect `Packaged 27 files` for `linq_rs` and `Packaged 21 files` for
`linq_rs_sql`, each followed by `warning: aborting upload due to dry run`. If
only `linq_rs` is mentioned, you dropped `--workspace`.

### 6. Publish `linq_rs 0.2.0` — IRREVERSIBLE

```bash
cargo publish -p linq_rs
```

**The `-p` is not optional.** A bare `cargo publish` at the root selects only
the default members and silently publishes `linq_rs` alone — verified: the
dry-run prints `Packaging linq_rs v0.2.0` with no mention of the sibling, and
exits 0.

### 7. Confirm it reached the index

```bash
curl -sS https://index.crates.io/li/nq/linq_rs | tail -1
```

The last line should be a JSON record containing `"vers":"0.2.0"`. Do not start
step 8 before this returns.

### 8. Publish `linq_rs_sql 0.1.0` — IRREVERSIBLE, and this claims the name

```bash
cargo publish -p linq_rs_sql
```

`cargo publish --workspace` would do steps 6 and 8 in one correctly-ordered
invocation, but multi-package publishing is explicitly **non-atomic**: per the
Cargo 1.90 changelog, a server-side error leaves the workspace partially
published. Sequential gives a clean failure boundary.

### 9. Yank `0.1.0`

```bash
cargo yank --version 0.1.0 linq_rs
```

Only now — yanking earlier leaves the crate with no usable version in between.
`0.1.0` shipped a `then_by` that discarded the primary sort key and a `skip`
that turned every unqualified `.skip(n)` in an importing module into a compile
error (`D-009`, `AUDIT.md` A-3).

### 10. Flip the README note

`README.md` says `0.1.0` "is being yanked … update this note to 'yanked' once
that has run, **not before**." Now, and only now.

### 11. Tag per crate

```bash
git tag -a linq_rs-v0.2.0 -m "linq_rs 0.2.0" && git tag -a linq_rs_sql-v0.1.0 -m "linq_rs_sql 0.1.0"
```
```bash
git push origin linq_rs-v0.2.0 linq_rs_sql-v0.1.0
```

Not a single `v0.2.0`. The two crates version independently — root is `0.2.0`,
member is `0.1.0` — so an unprefixed tag is ambiguous from the first release.
This is what tokio (`tokio-1.53.1`, `tokio-util-0.7.19`) and axum
(`axum-v0.8.9`) do.

### 12. Verify ownership landed

```bash
curl -sS -A "your-email@example.com" https://crates.io/api/v1/crates/linq_rs_sql/owners
```

Expect `"login":"TRget88"`. Use `curl`, not `cargo owner --list`, which wants a
token even though the endpoint is public. crates.io rejects requests with no
User-Agent.

## After: docs.rs

docs.rs builds automatically from the uploaded tarball; nothing to trigger. Both
crates have zero runtime dependencies and now build on 1.65, so there is little
to fail. A failed docs build is fixed by publishing a new patch version — never
by re-uploading.
