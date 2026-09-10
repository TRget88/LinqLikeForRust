#!/usr/bin/env bash
#
# CI-only real-crate interop check. Enforces DECISIONS.md D-005 against the
# actual itertools, without adding it as a dependency of this crate.
#
# WHY THIS SHAPE. tests/interop.rs mimics itertools' receiver shapes and is the
# always-on gate. A mimic can silently stop matching what it stands in for --
# which is the exact failure mode this repo keeps having -- so this script does
# the same check against the real crate. It builds a THROWAWAY crate in a temp
# directory that depends on both, so:
#   * itertools never appears in this crate's Cargo.toml
#   * it never ships in the published manifest
#   * a bare checkout still runs the full test suite offline
#
# WHAT IT IS NOT. This does not depend on itertools' algorithms or borrow its
# implementations. linq_rs has its own. The only thing under test is whether the
# two method-name sets can coexist in one scope.
#
# WHAT IT CATCHES that the mimic cannot: a future itertools release adding a
# method whose name matches one of ours. itertools has ~146 methods and does add
# more; if one lands on `distinct`, `order_by`, `chunk` or similar, downstream
# users importing both traits break and we find out here rather than from a bug
# report.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ITERTOOLS_REQ="${ITERTOOLS_REQ:-0.15}"
echo "repo:      $REPO"
echo "itertools: $ITERTOOLS_REQ"
echo "workdir:   $WORK"
echo

mkdir -p "$WORK/src"
cat > "$WORK/Cargo.toml" <<EOF
[package]
name = "itertools_interop"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
linq_rs = { path = "$REPO" }
itertools = "$ITERTOOLS_REQ"
EOF

cat > "$WORK/src/main.rs" <<'EOF'
//! Both traits in scope at once. This file exists to COMPILE; if it does, the
//! two name sets coexist. Every call below is one that has broken before.
use itertools::Itertools;
use linq_rs::{LinqExt, ThenBy};

fn main() {
    // Itertools::join takes &mut self. linq_rs::join took self by value and beat
    // it at the by-value resolution step with NO diagnostic, turning this line
    // into three unrelated errors. Renamed to inner_join in W-12.
    let s: String = vec![1, 2, 3].into_iter().join("-");
    assert_eq!(s, "1-2-3");

    // Itertools::group_by is a deprecated alias in 0.15 but still present, and
    // both took self by value -> hard E0034. Renamed to group_by_key in W-12.
    #[allow(deprecated)]
    {
        let g = vec![1, 1, 2].into_iter().group_by(|x| *x);
        assert_eq!(g.into_iter().count(), 2);
    }

    // std. Importing LinqExt used to make every unqualified .skip(n) an E0034,
    // including on iterators unrelated to this crate. Renamed to skip_ earlier.
    assert_eq!((1..=5).skip(3).collect::<Vec<_>>(), vec![4, 5]);

    // A few more itertools methods whose names are near ours, to catch a future
    // collision early rather than from a downstream bug report.
    assert_eq!(vec![1, 1, 2].into_iter().unique().count(), 2);
    assert_eq!(vec![3, 1, 2].into_iter().sorted().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(vec![1, 2, 3].iter().counts().len(), 3);

    // And linq_rs's own operators, under their post-W-12 names, in the same scope.
    assert_eq!((1..=5).skip_(3).to_vec(), vec![4, 5]);
    assert_eq!(vec![1, 1, 2].into_iter().distinct().to_vec(), vec![1, 2]);
    let rows: Vec<String> = vec![(1u32, "a")]
        .into_iter()
        .inner_join(vec![(1u32, "x")], |(k, _)| *k, |(k, _)| *k, |(_, l), (_, r)| format!("{l}{r}"))
        .collect();
    assert_eq!(rows, ["ax"]);
    let groups = vec!["ant", "bee"].into_iter().group_by_key(|w| w.chars().next().unwrap());
    assert_eq!(groups.count(), 2);
    let sorted = vec![(2, "b"), (1, "a")].into_iter().order_by(|t| t.0).then_by(|t| t.1);
    assert_eq!(sorted.into_iter().next().unwrap().1, "a");

    println!("itertools interop OK");
}
EOF

# The workflow sets RUSTFLAGS=-D warnings globally; the throwaway crate is not
# ours to hold to that, and itertools' own code may warn on a future compiler.
unset RUSTFLAGS
# And it must build in its own temp dir, not inherit a caller's target dir
# (an empty CARGO_TARGET_DIR is a hard cargo error, not a collision).
unset CARGO_TARGET_DIR

cd "$WORK"
echo "=== building throwaway crate against real itertools ==="
set +e
out="$(cargo run --quiet ${CARGO_OFFLINE:-} 2>&1)"
status=$?
set -e
printf '%s\n' "$out"
echo

if [ "$status" -eq 0 ]; then
  echo "PASS: linq_rs and itertools $ITERTOOLS_REQ coexist in one scope."
  exit 0
fi

# Do not blame a collision for what may be an infrastructure failure -- a
# network hiccup and a genuine name clash are different problems and deserve
# different messages.
if printf '%s\n' "$out" | grep -qE 'error\[E0034\]|multiple applicable items|error\[E0599\]|error\[E0277\]|error\[E0061\]'; then
  echo "FAIL: a method name collides again."
  echo "      linq_rs and itertools $ITERTOOLS_REQ can no longer be imported into"
  echo "      one scope. See DECISIONS.md D-005 and AUDIT.md findings E-1/E-2."
else
  echo "FAIL: the throwaway crate did not build, but not with a resolution error."
  echo "      This looks like an infrastructure problem (network, registry,"
  echo "      toolchain), not a name collision. Read the output above."
fi
exit 1
