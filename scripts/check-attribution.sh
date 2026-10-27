#!/usr/bin/env bash
# The repository never shows an agent association: no Co-Authored-By trailer naming an assistant,
# no "Generated with <tool>" line, no assistant no-reply address, in any commit message or tracked
# file. With file arguments, scans those files only (the commit-msg hook and the self-test); with
# none, scans every commit message reachable from a local branch or tag (remote-tracking refs may lag a
# rewrite until the owner pushes) and every tracked text file.
set -euo pipefail
cd "$(dirname "$0")/.."
pattern='Co-Authored-By:[^\n]*(Claude|Anthropic|Copilot|Codex|ChatGPT|Gemini|Cursor|assistant)|Generated with \[|noreply@anthropic\.com|Claude Code'
status=0
if [ "$#" -gt 0 ]; then
  for f in "$@"; do
    if grep -n -i -E "$pattern" "$f"; then echo "$f: agent attribution" >&2; status=1; fi
  done
  exit "$status"
fi
if git log --branches --tags --format='%H %B' | grep -i -E "$pattern" | head -n 5 | grep .; then
  echo "commit messages carry agent attribution (above); rewrite them" >&2; status=1
fi
# Tracked text files; the recorded fixtures are vendor payloads and name models, not agents.
if git ls-files -z -- . ':!fixtures/adapters' ':!fixtures/profiles' \
  | xargs -0 grep -n -I -i -E "$pattern" -- 2>/dev/null | grep -v '^scripts/check-attribution.sh:' | head -n 10 | grep .; then
  echo "tracked files carry agent attribution (above)" >&2; status=1
fi
[ "$status" -eq 0 ] && echo "attribution ok"
exit "$status"
