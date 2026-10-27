#!/usr/bin/env bash
# Proves the repository checks fail when they should. Run by `just check`.
set -euo pipefail
cd "$(dirname "$0")/.."
pass=0; fail=0

# crate graph: an empty allowed-edge file must fail and list every edge.
empty="$(mktemp)"; : > "$empty"
if CRATE_GRAPH_FILE="$empty" scripts/check-crate-graph.sh >/dev/null 2>&1; then
  echo "FAIL: crate-graph check accepted an empty graph" >&2; fail=$((fail+1))
else pass=$((pass+1)); fi

# crate graph: one removed edge must fail and name it.
one="$(mktemp)"; grep -v 'dejaview-eval -> dejaview-tools' scripts/crate-graph.txt > "$one"
out="$(CRATE_GRAPH_FILE="$one" scripts/check-crate-graph.sh 2>&1 || true)"
if grep -q 'unexpected prod edge: dejaview-eval -> dejaview-tools' <<< "$out"; then pass=$((pass+1));
else echo "FAIL: crate-graph check did not name the missing edge" >&2; fail=$((fail+1)); fi
rm -f "$empty" "$one"

# check-filter: a filter that selects nothing must fail; an unknown package must fail loudly.
if scripts/check-filter.sh dejaview-core no_such_test_name >/dev/null 2>&1; then
  echo "FAIL: check-filter accepted an empty selection" >&2; fail=$((fail+1)); else pass=$((pass+1)); fi
if scripts/check-filter.sh no-such-package foo >/dev/null 2>&1; then
  echo "FAIL: check-filter accepted an unknown package" >&2; fail=$((fail+1)); else pass=$((pass+1)); fi

# check-packaging: an include that reaches outside its crate must fail and name it; an embedded
# file the manifest excludes must fail as "not packaged".
probe="$(mktemp -d)"; mkdir -p "$probe/crate/src" "$probe/crate/assets"
printf 'x = 1\n' > "$probe/outside.toml"; printf 'y = 2\n' > "$probe/crate/assets/a.toml"
printf '[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2024"\nexclude = ["assets/"]\n' > "$probe/crate/Cargo.toml"
printf 'pub const A: &str = include_str!("../../outside.toml");\n' > "$probe/crate/src/lib.rs"
out="$(CRATE_DIRS="$probe/crate" scripts/check-packaging.sh 2>&1 || true)"
if grep -q 'reaches outside the crate' <<< "$out"; then pass=$((pass+1));
else echo "FAIL: check-packaging accepted an include outside the crate: $out" >&2; fail=$((fail+1)); fi
printf 'pub const A: &str = include_str!("../assets/a.toml");\n' > "$probe/crate/src/lib.rs"
out="$(CRATE_DIRS="$probe/crate" scripts/check-packaging.sh 2>&1 || true)"
if grep -q 'embedded but not packaged' <<< "$out"; then pass=$((pass+1));
else echo "FAIL: check-packaging accepted an excluded asset: $out" >&2; fail=$((fail+1)); fi
printf '[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2024"\n' > "$probe/crate/Cargo.toml"
if CRATE_DIRS="$probe/crate" scripts/check-packaging.sh >/dev/null 2>&1; then pass=$((pass+1));
else echo "FAIL: check-packaging rejected a packaged asset" >&2; fail=$((fail+1)); fi
rm -rf "$probe"

echo "check tests: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
