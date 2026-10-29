#!/usr/bin/env bash
# The live smoke (PLAN.md task 2.10b, owner decision 4): one `check` through each direct profile
# whose credential and account are in the environment, from the binary given (an unpacked release
# archive for the release record), on the small fixture image pair, with the cheapest model of
# each vendor. Prints one line per vendor: verdict, exit code, cost. Account ids and keys never
# appear in the output. Exit codes 0 (pass), 1 (fail) and 2 (unsure) are all a working harness;
# anything else fails the smoke. Spends a few cents per vendor.
#
#   set -a; source .env; set +a
#   scripts/live-smoke.sh /path/to/squint
set -euo pipefail
bin="${1:?usage: live-smoke.sh <squint binary>}"
bin="$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")"
repo="$(cd "$(dirname "$0")/.." && pwd)"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
ran=0; failed=0

smoke() { # profile model key-var account-var account-prefix
  local profile="$1" model="$2" key="$3" acct="$4" prefix="$5"
  if [ -z "${!key:-}" ] || [ -z "${!acct:-}" ]; then
    echo "$profile: skipped ($key or $acct not set)"; return
  fi
  ran=$((ran+1))
  local dir="$out/$profile"; mkdir -p "$dir"
  set +e
  (cd "$repo" && "$bin" check --config examples/source-fidelity.toml \
      --set "provider.profile=$profile" --set "provider.model=$model" \
      --set "privacy.account=$prefix:${!acct}" \
      --use-case source-fidelity \
      --candidate fixtures/images/candidate-64x48.png --source fixtures/images/source-64x48.png \
      --out "$dir" > "$dir/stdout" 2> "$dir/stderr")
  local code=$?
  set -e
  local report verdict cost
  report="$(find "$dir" -name report.json | head -n 1)"
  if [ -n "$report" ]; then
    # A report the reader cannot parse is a failed vendor, not an aborted smoke.
    read -r verdict cost < <(python3 -I - "$report" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
verdict = r.get("verdict") or ("error" if r["decision"].get("error") else "none")
spent = r["cost"]["spent_usd"]
print(verdict, f'{spent if spent > 0 else 0.0:.4f}')
PY
) || { verdict="unreadable-report"; cost="-"; code=99; }
  else
    verdict="no-report"; cost="-"
  fi
  case "$code" in 0|1|2) ;; *) failed=$((failed+1)); echo "  stderr: $(head -c 400 "$dir/stderr")" ;; esac
  echo "$profile ($model): verdict=$verdict exit=$code cost_usd=$cost"
}

smoke anthropic claude-haiku-5-5 ANTHROPIC_API_KEY SQUINT_ANTHROPIC_ACCOUNT anthropic
smoke google-ai-studio gemini-3.8-flash GEMINI_API_KEY SQUINT_GOOGLE_ACCOUNT google
smoke openai gpt-6.1-terra OPENAI_API_KEY SQUINT_OPENAI_ACCOUNT openai

[ "$ran" -gt 0 ] || { echo "live smoke ran nothing: no vendor credentials in the environment" >&2; exit 1; }
[ "$failed" -eq 0 ] || { echo "live smoke: $failed vendor(s) failed" >&2; exit 1; }
echo "live smoke ok: $ran vendor(s) from $("$bin" --version)"
