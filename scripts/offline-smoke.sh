#!/usr/bin/env bash
# The offline smoke for an installed `dejaview` binary: version, the two inspection commands and a
# live check with no credential, which must stop with the auth error before anything leaves the
# machine. Runs from a scratch directory, never from the repository tree.
#
#   scripts/offline-smoke.sh /path/to/dejaview
set -euo pipefail
bin="${1:?usage: offline-smoke.sh <dejaview binary>}"
bin="$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"
cat > smoke.toml <<'TOML'
[provider]
profile = "anthropic"
model = "claude-haiku-5-5"

[privacy]
cloud_calls = true
account = "anthropic:smoke"

[rules]
base = "source-fidelity"

[output]
dir = "./out"
TOML

version="$("$bin" --version)"
case "$version" in dejaview\ *) ;; *) echo "unexpected --version output: $version" >&2; exit 1 ;; esac
"$bin" tools list --config smoke.toml > /dev/null
"$bin" rules show --config smoke.toml | grep -q '^SF1 ' || { echo "rules show did not print the shipped base" >&2; exit 1; }
set +e
err="$(env -u ANTHROPIC_API_KEY "$bin" check --config smoke.toml --candidate a.png --source b.png --use-case source-fidelity 2>&1 >/dev/null)"
code=$?
set -e
if [ "$code" -ne 64 ] || ! grep -q 'auth:' <<< "$err"; then
  echo "check without a credential: expected exit 64 with an auth error, got exit $code: $err" >&2; exit 1
fi
echo "offline smoke ok: $version"
