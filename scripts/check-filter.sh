#!/usr/bin/env bash
# Runs `cargo test -p <package> -- <filters...>` and fails if the filters select no tests.
set -euo pipefail
pkg="$1"; shift
listed=$(cargo test -p "$pkg" -- --list "$@" 2>/dev/null | grep -c ': test$' || true)
if [ "$listed" -eq 0 ]; then
  echo "filter selects no tests in $pkg: $*" >&2
  exit 1
fi
cargo test -p "$pkg" -- "$@"
