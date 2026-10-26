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

echo "check tests: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
