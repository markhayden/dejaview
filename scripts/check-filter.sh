#!/usr/bin/env bash
# Runs `cargo test -p <package> -- <filters...>` and fails if the filters select no tests.
# Compile errors and unknown packages fail loudly instead of being read as "no tests".
set -euo pipefail
pkg="$1"; shift
list_out="$(mktemp)"
trap 'rm -f "$list_out"' EXIT
if ! cargo test -p "$pkg" -- --list "$@" > "$list_out" 2>&1; then
  cat "$list_out" >&2
  echo "cargo test --list failed for $pkg" >&2
  exit 1
fi
listed="$(grep -c ': test$' "$list_out" || true)"
if [ "${listed:-0}" -eq 0 ]; then
  echo "filter selects no tests in $pkg: $*" >&2
  exit 1
fi
cargo test -p "$pkg" -- "$@"
