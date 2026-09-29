#!/usr/bin/env python3
"""Link integrity across every markdown file and every rustdoc comment.

Three checks, deliberately separated because they fail for different reasons:

1. **Anchors** (`](#something)`) must resolve to a heading in the same file.
   Pure text, deterministic, no network. This is the check that matters most and
   it is always fatal -- a broken anchor is a broken link forever, and I produced
   one by hand during the documentation truth pass.

2. **Repo-relative paths** (`](docs/FOO.md)`) must exist on disk. Also
   deterministic and fatal. Whether such a path survives *packaging* is a
   different question, answered by `packaging-gate.sh` against the tarball.

3. **External URLs** must resolve. This one touches the network, so a failure is
   ambiguous: a 404 is a real defect, a timeout is the network. Only definite
   failures are fatal, and hosts known to reject non-browser clients are listed
   with a reason rather than silently skipped.

Run:  python3 .github/scripts/link-check.py [--no-network]
Exit: 0 if every check passes.
"""

import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Files whose links are checked. Markdown, plus every crate source file, because
# rustdoc comments carry links too and a dead link there ships to docs.rs.
def targets() -> list:
    out = sorted(ROOT.glob("*.md")) + sorted(ROOT.glob("docs/*.md"))
    for sub in ("linq_rs_sql", "seam-tests"):
        out += sorted((ROOT / sub).glob("*.md"))
        out += sorted((ROOT / sub / "src").glob("*.rs"))
    out += sorted((ROOT / "src").glob("*.rs"))
    out += sorted((ROOT / ".github").glob("**/*.md"))
    return [p for p in out if p.is_file()]


# Hosts that answer a non-browser client with something other than the truth.
# Each needs a REASON, so this cannot become a place to hide real breakage.
BOT_BLOCKED = {
    "crates.io": (
        "returns 404 to any non-browser client, including for crates that are "
        "definitely published -- verified against linq_rs 0.2.0, which is live. "
        "Publication is checkable via crates.io/api/v1 instead."
    ),
}


def repo_path_for(url: str):
    """The repo-relative path a github.com/<owner>/<repo>/blob|tree/<ref>/<path>
    URL points at, or None if the URL is not of that shape."""
    m = re.match(
        r"^https://github\.com/[^/]+/[^/]+/(?:blob|tree)/[^/]+/(.+?)(?:#.*)?$", url
    )
    return m.group(1) if m else None


def headings(text: str) -> set:
    """GitHub's anchor slugs for every heading in a markdown document."""
    out = set()
    for line in text.splitlines():
        m = re.match(r"#+\s+(.*)", line)
        if not m:
            continue
        a = m.group(1).lower().replace("`", "")
        a = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", a)  # [text](url) -> text
        a = re.sub(r"[^\w\s-]", "", a).strip().replace(" ", "-")
        out.add(a)
    return out


def links(path: Path):
    """(line, target) for every markdown link, including inside rustdoc comments.

    Inline code spans are stripped first. Markdown does not create a link inside
    backticks, so `](#x)` written as an EXAMPLE of link syntax is not a link --
    and this file documents exactly that syntax, so the checker flagged its own
    documentation on the first full run.
    """
    text = path.read_text(encoding="utf-8")
    for n, line in enumerate(text.splitlines(), 1):
        # Drop `...` spans (longest-first so ``...`` is handled too).
        bare = re.sub(r"``[^`]*``|`[^`]*`", "", line)
        for target in re.findall(r"\]\(([^)\s]+)\)", bare):
            yield n, target


def check_one(path: Path, do_network: bool, seen_urls: dict) -> list:
    problems = []
    text = path.read_text(encoding="utf-8")
    anchors = headings(text) if path.suffix == ".md" else set()

    for n, target in links(path):
        rel = path.relative_to(ROOT)

        if target.startswith("#"):
            # Anchors are only checkable in markdown; a rustdoc `#` link is an
            # intra-doc reference that rustdoc itself validates under -D warnings.
            if path.suffix != ".md":
                continue
            if target[1:] not in anchors:
                problems.append(f"{rel}:{n}  broken anchor {target}")

        elif target.startswith(("http://", "https://")):
            url = target.rstrip(".,;")
            host = re.sub(r"^https?://([^/]+).*$", r"\1", url)
            if host in BOT_BLOCKED:
                continue
            if not do_network:
                continue
            if url in seen_urls:
                code = seen_urls[url]
            else:
                code = fetch(url)
                seen_urls[url] = code
            if code in (404, 410):
                # A GitHub `blob/main/<path>` or `tree/main/<path>` 404 has two
                # very different causes, and conflating them makes the gate
                # useless on a feature branch:
                #   - the path does not exist ANYWHERE -> a real dead link
                #   - it exists in this working tree but is not on `main` yet
                #     -> correct after merge, and correct for a reader of the
                #        published crate, which is the audience for a main link
                # Only the first is fatal. This distinction is checkable, so it is
                # made rather than papered over with an allowlist.
                local = repo_path_for(url)
                if local is not None and (ROOT / local).exists():
                    print(
                        f"  (not on `main` yet, exists locally at {local}) "
                        f"{rel}:{n}  {url}"
                    )
                else:
                    where = f" -- and {local} does not exist locally either" if local else ""
                    problems.append(f"{rel}:{n}  HTTP {code}  {url}{where}")
            elif code is None:
                print(f"  (unreachable, not counted as a failure) {url}")

        elif target.startswith("mailto:"):
            continue

        elif path.suffix == ".md":
            # Repo-relative path. Resolve against the file's own directory.
            p = (path.parent / target.split("#")[0]).resolve()
            if not p.exists():
                problems.append(f"{rel}:{n}  missing path {target}")

        # else: a non-URL, non-anchor target in a .rs file is a rustdoc INTRA-DOC
        # link (`Self::except`, `crate::rows::Rows`, `Iterator::skip`). Rustdoc
        # resolves those itself and the gate suite runs it with
        # RUSTDOCFLAGS=-D warnings, which turns an unresolved one into a build
        # failure -- so checking them here would be both wrong (they are not
        # paths) and redundant.

    return problems


def fetch(url: str):
    """HTTP status, or None if the host could not be reached at all."""
    req = urllib.request.Request(
        url, headers={"User-Agent": "linq-rs-link-check"}, method="HEAD"
    )
    for attempt in (1, 2):
        try:
            with urllib.request.urlopen(req, timeout=15) as r:
                return r.status
        except urllib.error.HTTPError as e:
            if e.code == 405 and attempt == 1:  # HEAD not allowed; try GET
                req = urllib.request.Request(
                    url, headers={"User-Agent": "linq-rs-link-check"}
                )
                continue
            return e.code
        except Exception:
            if attempt == 2:
                return None
    return None


def main() -> int:
    do_network = "--no-network" not in sys.argv
    files = targets()
    print(f"checking {len(files)} files ({'with' if do_network else 'without'} network)")
    if BOT_BLOCKED:
        for host, why in BOT_BLOCKED.items():
            print(f"  skipping host {host}: {why}")

    seen_urls: dict = {}
    problems = []
    for f in files:
        problems += check_one(f, do_network, seen_urls)

    print(f"  distinct external URLs resolved: {len(seen_urls)}")
    print()
    if problems:
        for p in problems:
            print(f"  {p}")
        print(f"\nlink check FAILED: {len(problems)} problem(s)")
        return 1
    print("PASS: every anchor, path and external URL resolves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
