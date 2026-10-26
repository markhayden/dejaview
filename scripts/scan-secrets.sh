#!/usr/bin/env bash
# Scans text on stdin for credential-shaped strings. Exact values listed in
# scripts/synthetic-tokens.txt are allowed; any other match fails. Prints offending values.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
patterns=(
  'sk-ant-[A-Za-z0-9_-]{24,}'
  'AIza[0-9A-Za-z_-]{35}'
  'sk-proj-[A-Za-z0-9_-]{24,}'
  'AKIA[0-9A-Z]{16}'
  'ghp_[A-Za-z0-9]{36}'
)
allow_file="$here/synthetic-tokens.txt"
input="$(cat)"
status=0
for p in "${patterns[@]}"; do
  while IFS= read -r match; do
    [ -z "$match" ] && continue
    if grep -qxF -- "$match" "$allow_file"; then
      continue
    fi
    echo "credential-shaped value: ${match:0:12}…" >&2
    status=1
  done < <(printf '%s\n' "$input" | grep -oE -- "$p" || true)
done
exit $status
