#!/usr/bin/env python3
"""Generate the README's derived tables, and gate them.

Enforces DECISIONS.md D-016: any coverage count or C#-to-Rust mapping is
generated from the source, never written by hand, and CI fails if the committed
README differs from what this produces.

D-016 exists because three hand-written counts in this project have been wrong,
one of them inside the fix for the previous one. So:

  * the linq_rs surface is DERIVED from src/ on every run
  * the C# surface is DATA (it cannot be derived from this repo)
  * the GATE is the cross-check between them

Usage:
    gen-docs.py --write    rewrite the generated blocks in README.md
    gen-docs.py --check    fail if README.md differs from generated output
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DATA = os.path.join(ROOT, ".github", "data")
README = os.path.join(ROOT, "README.md")

# Which .NET the README quotes. See .github/data/README.md for why this is not
# just "the number on the docs page".
TARGET_DOTNET = "net-10.0"

# Oldest to newest. A name is present in TARGET_DOTNET if its dotnet_added is
# empty (shipped with original LINQ) or ranks at or below the target.
MONIKERS = ["netframework-3.5", "netframework-4.0", "netframework-4.8",
            "netcore-1.0", "netcore-2.0", "netcore-3.1",
            "net-5.0", "net-6.0", "net-7.0", "net-8.0", "net-9.0",
            "net-10.0", "net-11.0"]


def die(msg, details=()):
    print(f"FAIL: {msg}", file=sys.stderr)
    for d in details:
        print(f"  {d}", file=sys.stderr)
    sys.exit(1)


def read_tsv(path):
    with open(path, encoding="utf-8") as fh:
        rows = [ln.rstrip("\n").split("\t") for ln in fh if ln.strip()]
    head, body = rows[0], rows[1:]
    out = []
    for r in body:
        r = r + [""] * (len(head) - len(r))
        out.append(dict(zip(head, r)))
    return out


# ── derive the real surface from source ──────────────────────────────────────

def derive_surface():
    """The half that must be mechanical."""
    def scan(rel, pattern):
        src = open(os.path.join(ROOT, rel), encoding="utf-8").read()
        return re.findall(pattern, src, re.M)

    linqext = scan("src/queryable.rs", r"^    fn ([a-z_0-9]+)[<(]")
    # ThenBy: take the trait declaration only, not its impl. The declarations
    # are the ones followed by a `where` clause rather than a body.
    ordered = open(os.path.join(ROOT, "src/ordered.rs"), encoding="utf-8").read()
    trait_body = ordered[ordered.index("pub trait ThenBy"):]
    trait_body = trait_body[:trait_body.index("\n}")]
    thenby = re.findall(r"^    fn ([a-z_0-9]+)[<(]", trait_body, re.M)
    free = scan("src/sources.rs", r"^pub fn ([a-z_0-9]+)[<(]")
    # Qualified, because `get` on Lookup and `key` on Grouping are different
    # items that would otherwise collide in one namespace.
    grouping = [f"Grouping::{m}" for m in
                scan("src/grouping.rs", r"^    pub fn ([a-z_0-9]+)[<(]")]
    lookup = [f"Lookup::{m}" for m in
              scan("src/lookup.rs", r"^    pub fn ([a-z_0-9]+)[<(]")]
    return {"linqext": linqext, "thenby": thenby, "free_fn": free,
            "type_method": grouping + lookup}


def cross_check(surface, mapping, csharp):
    """The gate. Every discrepancy here is drift that would otherwise ship."""
    errors = []
    by_kind = {}
    for row in mapping:
        by_kind.setdefault(row["kind"], set()).add(row["rust_name"])

    for kind, names in surface.items():
        src_set, map_set = set(names), by_kind.get(kind, set())
        for missing in sorted(src_set - map_set):
            errors.append(f"{kind} `{missing}` exists in src/ but is absent from "
                          f"operator-map.tsv — add it (renamed? new?)")
        for phantom in sorted(map_set - src_set):
            errors.append(f"{kind} `{phantom}` is in operator-map.tsv but not in "
                          f"src/ — it was renamed or removed")

    cs_names = {r["name"] for r in csharp}
    for row in mapping:
        cs = row["csharp"].strip()
        if cs and cs not in cs_names:
            errors.append(f"`{row['rust_name']}` maps to C# `{cs}`, which is not "
                          f"in csharp-operators.tsv")
        v = row["variant_of"].strip()
        if v and v not in {r["rust_name"] for r in mapping}:
            errors.append(f"`{row['rust_name']}` is a variant_of `{v}`, which is "
                          f"not itself a mapped method")
    return errors


def check_api_reference(surface, text):
    """The hand-written API Reference is not generated (yet), but it must at
    least mention every public LinqExt method and no nonexistent one."""
    start = text.index("## API Reference")
    end = text.index("## Realistic Example")
    section = text[start:end]
    mentioned = set(re.findall(r"`([a-z_0-9]+)\(", section))
    mentioned |= set(re.findall(r"`([a-z_0-9]+)`", section))
    errors = []
    for m in sorted(set(surface["linqext"]) - mentioned):
        errors.append(f"`{m}` is public but appears in no API Reference table")
    return errors


def rank(moniker):
    return MONIKERS.index(moniker) if moniker in MONIKERS else -1


# ── rendering ────────────────────────────────────────────────────────────────

def render_coverage(mapping, csharp):
    target = rank(TARGET_DOTNET)
    in_target = [r for r in csharp
                 if not r["dotnet_added"].strip() or rank(r["dotnet_added"]) <= target]
    names = len(in_target)
    overloads = sum(int(r["overloads"] or 0) for r in in_target)
    comparers = sum(int(r["comparer_overloads"] or 0) for r in in_target)

    implemented = {r["csharp"] for r in mapping if r["csharp"].strip()}
    covered = [r for r in in_target if r["name"] in implemented]
    absent = [r for r in in_target if r["name"] not in implemented]
    statics = [r for r in in_target if r["is_static"] == "yes"]

    lines = [
        f"`System.Linq.Enumerable` in **.NET {TARGET_DOTNET.split('-')[1]}** exposes "
        f"**{names} operator names** across **{overloads} overloads**, "
        f"**{comparers}** of which take an `IEqualityComparer` or `IComparer`.",
        "",
        f"This crate implements **{len(covered)} of those {names} names** and "
        f"**none of the {comparers} comparer overloads** — the latter deliberately "
        "(`D-202`): Rust expresses that with traits and newtypes, and the honest "
        "substitute is the `*_by` key-selector family.",
        "",
        f"**Not implemented ({len(absent)}):** "
        + ", ".join(f"`{r['name']}`" for r in sorted(absent, key=lambda r: r["name"]))
        + ".",
        "",
        f"Of the {names}, **{len(statics)}** are static generator methods on "
        "`Enumerable` rather than extension methods, so they are free functions "
        "here, not `LinqExt` methods: "
        + ", ".join(f"`{r['name']}`" for r in sorted(statics, key=lambda r: r["name"]))
        + ".",
        "",
        "> These numbers are generated by `.github/scripts/gen-docs.py` from"
        " `.github/data/`, and CI fails if they drift. Do not edit them by hand.",
    ]
    return "\n".join(lines)


def render_std_overlap(mapping):
    linq = [r for r in mapping if r["kind"] == "linqext"]
    with_std = [r for r in linq if r["std_equivalent"].strip()]
    pct = round(100 * len(with_std) / len(linq))
    lines = [
        f"**{len(with_std)} of this crate's {len(linq)} `LinqExt` methods "
        f"({pct}%) are a rename or a short composition of something "
        "`std::iter::Iterator` already gives you.** If you are not porting C# "
        "code, reach for std first.",
        "",
        "| linq_rs | use this instead |",
        "|---|---|",
    ]
    for r in sorted(with_std, key=lambda r: r["rust_name"]):
        lines.append(f"| `{r['rust_name']}` | `{r['std_equivalent']}` |")
    lines += ["",
              f"The remaining {len(linq) - len(with_std)} have no direct std "
              "equivalent — that is the part of this crate with a reason to exist."]
    return "\n".join(lines)


BLOCKS = {"coverage": render_coverage, "std-overlap": render_std_overlap}


def splice(text, name, body):
    begin, end = f"<!-- BEGIN GENERATED: {name} -->", f"<!-- END GENERATED: {name} -->"
    if begin not in text or end not in text:
        die(f"README.md is missing the {name} markers ({begin} / {end})")
    pre = text[:text.index(begin) + len(begin)]
    post = text[text.index(end):]
    return f"{pre}\n{body}\n{post}"


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "--check"
    if mode not in ("--check", "--write"):
        die("usage: gen-docs.py [--check|--write]")

    surface = derive_surface()
    mapping = read_tsv(os.path.join(DATA, "operator-map.tsv"))
    csharp = read_tsv(os.path.join(DATA, "csharp-operators.tsv"))

    errors = cross_check(surface, mapping, csharp)
    text = open(README, encoding="utf-8").read()
    errors += check_api_reference(surface, text)
    if errors:
        die("source and documentation data disagree", errors)

    out = text
    for name, render in BLOCKS.items():
        body = render(mapping, csharp) if name == "coverage" else render(mapping)
        out = splice(out, name, body)

    counts = {k: len(v) for k, v in surface.items()}
    if mode == "--write":
        if out != text:
            open(README, "w", encoding="utf-8").write(out)
            print(f"README.md updated. Derived surface: {counts}")
        else:
            print(f"README.md already current. Derived surface: {counts}")
        return
    if out != text:
        die("README.md is stale — run `.github/scripts/gen-docs.py --write` and "
            "commit the result",
            ["the generated blocks do not match the current source + data"])
    print(f"PASS: generated blocks are current. Derived surface: {counts}")


if __name__ == "__main__":
    main()
