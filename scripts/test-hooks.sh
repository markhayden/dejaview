#!/usr/bin/env bash
# Proves the credential scanner accepts what it must and rejects what it must. Run by `just check`.
set -euo pipefail
cd "$(dirname "$0")/.."
pass=0; fail=0
expect_ok ()  { if scripts/scan-secrets.sh <"$1" 2>/dev/null; then pass=$((pass+1)); else echo "FAIL (should accept): $2" >&2; fail=$((fail+1)); fi; }
expect_bad () { if scripts/scan-secrets.sh <"$1" 2>/dev/null; then echo "FAIL (should reject): $2" >&2; fail=$((fail+1)); else pass=$((pass+1)); fi; }

expect_ok  scripts/scan-secrets.sh       "the scanner's own source"
expect_ok  scripts/hooks/pre-commit      "the hook's own source"
expect_ok  scripts/synthetic-tokens.txt  "the synthetic token list"
expect_ok  docs/bootstrap.md             "documentation"
expect_ok  fixtures/synthetic-secrets/example.env "a fixture using only synthetic tokens"

tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
# Realistic-shaped values are assembled at run time so this file never contains a full match.
printf 'ANTHROPIC_API_KEY=sk-ant-%s\n' "$(printf 'r%.0s' $(seq 1 30))" >"$tmp"
expect_bad "$tmp" "a realistic Anthropic-shaped key in an ordinary file"
printf '# inside the fixtures dir or the hook test, same shape\nAKIA%s\n' "$(printf 'Q%.0s' $(seq 1 16))" >"$tmp"
expect_bad "$tmp" "a realistic AWS-shaped key, path-independent"
printf 'token = "ghp_%s"\n' "$(printf 'a%.0s' $(seq 1 36))" >"$tmp"
expect_bad "$tmp" "a realistic GitHub-shaped token"
printf 'nothing to see here\n' >"$tmp"
expect_ok  "$tmp" "a clean change"

echo "hook scanner: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
