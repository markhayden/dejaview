#!/usr/bin/env bash
# Scans text on stdin for credential-shaped strings. Exact values listed in
# scripts/synthetic-tokens.txt are allowed, and every entry there must contain "SYNTHETIC" so the
# allowlist cannot be used to launder a real key. Any other match fails; the report names the
# pattern and line number, never the value.
#
# Known limits, by design: a token split across lines, or base64/URL-encoded, is not detected.
# Byte-wise scanning (LC_ALL=C, grep -a) so keys adjacent to non-UTF-8 bytes or inside binary
# blobs are still seen.
set -euo pipefail
export LC_ALL=C
here="$(cd "$(dirname "$0")" && pwd)"
allow_file="$here/synthetic-tokens.txt"

patterns=(
  'anthropic:sk-ant-[A-Za-z0-9_-]{24,}'
  'google-api:AIza[0-9A-Za-z_-]{35}'
  'google-oauth:GOCSPX-[A-Za-z0-9_-]{28}'
  'openai-project:sk-proj-[A-Za-z0-9_-]{24,}'
  'openai-service:sk-svcacct-[A-Za-z0-9_-]{24,}'
  'openai-legacy:sk-[A-Za-z0-9]{48}'
  'aws-access-key:AKIA[0-9A-Z]{16}'
  'github-token:gh[opusr]_[A-Za-z0-9]{36}'
  'github-pat:github_pat_[A-Za-z0-9_]{82}'
  'private-key:-----BEGIN [A-Z ]*PRIVATE KEY-----'
)

# The allowlist is only trusted if every entry is visibly synthetic.
while IFS= read -r entry; do
  [ -z "$entry" ] && continue
  case "$entry" in \#*) continue ;; esac
  case "$entry" in *SYNTHETIC*) ;; *) echo "synthetic-tokens.txt entry without SYNTHETIC: refusing to trust the allowlist" >&2; exit 2 ;; esac
done < "$allow_file"

input="$(mktemp)"
trap 'rm -f "$input"' EXIT
cat > "$input"

status=0
for spec in "${patterns[@]}"; do
  name="${spec%%:*}"
  pattern="${spec#*:}"
  # grep exits 1 on no match; that is not an error here.
  matches="$(grep -naoE -- "$pattern" "$input" || true)"
  [ -z "$matches" ] && continue
  while IFS= read -r hit; do
    line="${hit%%:*}"
    value="${hit#*:}"
    if grep -qxF -- "$value" "$allow_file"; then
      continue
    fi
    echo "credential-shaped value ($name) at input line $line" >&2
    status=1
  done <<< "$matches"
done
exit $status
