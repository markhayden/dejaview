#!/usr/bin/env bash
# Proves the credential scanner and the pre-commit hook accept what they must and reject what they
# must. Run by `just check`. Realistic-shaped values are assembled at run time so this file never
# contains a full match.
set -euo pipefail
cd "$(dirname "$0")/.."
repo="$(pwd)"
pass=0; fail=0
ok ()  { pass=$((pass+1)); }
bad () { echo "FAIL: $1" >&2; fail=$((fail+1)); }

expect_scan_ok ()  { if scripts/scan-secrets.sh <"$1" 2>/dev/null; then ok; else bad "scanner should accept: $2"; fi; }
expect_scan_bad () { if scripts/scan-secrets.sh <"$1" 2>/dev/null; then bad "scanner should reject: $2"; else ok; fi; }

rep () { printf "$1%.0s" $(seq 1 "$2"); }   # rep <char> <n>

# --- scanner: positive cases ---------------------------------------------------------------
expect_scan_ok  scripts/scan-secrets.sh       "the scanner's own source"
expect_scan_ok  scripts/hooks/pre-commit      "the hook's own source"
expect_scan_ok  scripts/synthetic-tokens.txt  "the synthetic token list"
expect_scan_ok  fixtures/synthetic-secrets/example.env "a fixture using only synthetic tokens"
# Every tracked file that is present; a file deleted in the working tree but still in the index
# (for example a recording being replaced) is skipped rather than killing the run under set -e.
tree="$(mktemp)"
git ls-files -z | while IFS= read -r -d '' f; do [ -f "$f" ] && cat "$f" || true; done > "$tree"
expect_scan_ok  "$tree" "the whole tracked tree"

# --- scanner: negative cases ---------------------------------------------------------------
tmp="$(mktemp)"; trap 'rm -f "$tmp" "$tree"' EXIT
printf 'ANTHROPIC_API_KEY=sk-ant-%s\n' "$(rep r 30)" >"$tmp";            expect_scan_bad "$tmp" "Anthropic-shaped key"
printf 'AKIA%s\n' "$(rep Q 16)" >"$tmp";                                  expect_scan_bad "$tmp" "AWS-shaped key id"
printf 'token = "ghp_%s"\n' "$(rep a 36)" >"$tmp";                        expect_scan_bad "$tmp" "GitHub-shaped token"
printf 'GEMINI_API_KEY=AIza%s\n' "$(rep z 35)" >"$tmp";                   expect_scan_bad "$tmp" "Google-shaped API key"
printf 'OPENAI_API_KEY=sk-proj-%s\n' "$(rep p 30)" >"$tmp";               expect_scan_bad "$tmp" "OpenAI-shaped project key"
printf 'KEY=\x93sk-ant-%s\x94\n' "$(rep r 30)" >"$tmp";                   expect_scan_bad "$tmp" "key wrapped in CP1252 smart quotes"
printf 'sk-ant-%s\xe9\n' "$(rep r 30)" >"$tmp";                           expect_scan_bad "$tmp" "key followed by a Latin-1 byte"
{ head -c 64 /dev/urandom; printf 'sk-ant-%s' "$(rep r 30)"; head -c 64 /dev/urandom; } >"$tmp"
expect_scan_bad "$tmp" "key embedded in binary bytes"
printf 'nothing to see here\n' >"$tmp";                                    expect_scan_ok  "$tmp" "a clean change"

# --- allowlist invariant: an entry without SYNTHETIC makes the scanner refuse ------------------
fake_allow="$(mktemp -d)"; cp scripts/scan-secrets.sh "$fake_allow/scan-secrets.sh"
printf 'sk-ant-%s\n' "$(rep r 30)" > "$fake_allow/synthetic-tokens.txt"
if printf 'x\n' | "$fake_allow/scan-secrets.sh" 2>/dev/null; then bad "allowlist entry without SYNTHETIC should be refused"; else ok; fi
rm -rf "$fake_allow"

# --- hook end to end in a scratch repository -----------------------------------------------
work="$(mktemp -d)"
cleanup () { rm -rf "$work"; }
trap 'cleanup; rm -f "$tmp" "$tree"' EXIT
git -C "$work" init -q
git -C "$work" config user.email test@example.invalid
git -C "$work" config user.name test
mkdir -p "$work/scripts/hooks"
cp scripts/scan-secrets.sh scripts/synthetic-tokens.txt "$work/scripts/"
cp scripts/hooks/pre-commit scripts/hooks/commit-msg "$work/scripts/hooks/"
cp scripts/check-attribution.sh "$work/scripts/"
git -C "$work" config core.hooksPath scripts/hooks
export DEJAVIEW_HOOK_SKIP_CARGO=1

commit_ok ()  { if git -C "$work" commit -q -m "$1" >/dev/null 2>&1; then ok; else bad "hook should accept: $1"; fi; }
commit_bad () { if git -C "$work" commit -q -m "$1" >/dev/null 2>&1; then bad "hook should reject: $1"; else ok; git -C "$work" reset -q; fi; }

git -C "$work" add -A; commit_ok "initial scaffold"
echo "hello" > "$work/a.txt"; git -C "$work" add a.txt; commit_ok "an ordinary addition"
git -C "$work" rm -q a.txt; commit_ok "a deletion-only commit"
printf 'sk-ant-%s\n' "$(rep r 30)" > "$work/ordinary.txt"; git -C "$work" add ordinary.txt; commit_bad "real-shaped key in an ordinary file"
mkdir -p "$work/fixtures/synthetic-secrets"
printf 'sk-ant-%s\n' "$(rep r 30)" > "$work/fixtures/synthetic-secrets/x.env"; git -C "$work" add fixtures; commit_bad "real-shaped key inside the synthetic fixtures directory"
printf '# test\nAKIA%s\n' "$(rep Q 16)" > "$work/scripts/test-hooks.sh"; git -C "$work" add scripts; commit_bad "real-shaped key inside the hook test file"
cp fixtures/synthetic-secrets/example.env "$work/synthetic.env"; git -C "$work" add synthetic.env; commit_ok "the synthetic tokens"
{ head -c 64 /dev/urandom; printf 'AKIA%s' "$(rep Q 16)"; head -c 64 /dev/urandom; } > "$work/blob.bin"; git -C "$work" add blob.bin; commit_bad "key inside a binary blob"
printf 'KEY=\x93sk-ant-%s\x94\n' "$(rep r 30)" > "$work/quoted.txt"; git -C "$work" add quoted.txt; commit_bad "key in smart quotes"
# --- commit-msg hook: no agent attribution in a message (the patterns are assembled at run time
# so this file never contains a full match) ---------------------------------------------------
who="Cla""ude"; addr="noreply@anthro""pic.com"; generated="Generated ""with ["
echo "plain" > "$work/plain.txt"; git -C "$work" add plain.txt
commit_bad "a change

Co-Authored-By: $who Example <$addr>"
git -C "$work" add plain.txt; commit_bad "a change

${generated}Some Tool](https://example.invalid)"
git -C "$work" add plain.txt; commit_ok "a change

Co-Authored-By: A Person <person@example.invalid>"
unset DEJAVIEW_HOOK_SKIP_CARGO

echo "hook tests: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
