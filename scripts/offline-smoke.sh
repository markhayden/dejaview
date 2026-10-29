#!/usr/bin/env bash
# The offline smoke for an installed `squint` binary: version, the two inspection commands and a
# live check with no credential, which must stop with the auth error before anything leaves the
# machine. Runs from a scratch directory, never from the repository tree.
#
#   scripts/offline-smoke.sh /path/to/squint
set -euo pipefail
bin="${1:?usage: offline-smoke.sh <squint binary>}"
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
case "$version" in squint\ *) ;; *) echo "unexpected --version output: $version" >&2; exit 1 ;; esac
"$bin" tools list --config smoke.toml > /dev/null
# Captured first: `grep -q` stops reading at its first match, and a binary still writing then
# fails on the broken pipe.
rules="$("$bin" rules show --config smoke.toml)"
grep -q '^SF1 ' <<<"$rules" || { echo "rules show did not print the shipped base" >&2; exit 1; }
set +e
err="$(env -u ANTHROPIC_API_KEY "$bin" check --config smoke.toml --candidate a.png --source b.png --use-case source-fidelity 2>&1 >/dev/null)"
code=$?
set -e
if [ "$code" -ne 64 ] || ! grep -q 'auth:' <<< "$err"; then
  echo "check without a credential: expected exit 64 with an auth error, got exit $code: $err" >&2; exit 1
fi
# The report viewer serves on loopback: a directory with a report answers with the embedded page.
mkdir -p view-dir
echo '{"report_version":1}' > view-dir/report.json
"$bin" report view view-dir --port 0 > view.log 2>&1 &
view_pid=$!
# The viewer is waited for before the work directory goes: removing it while the viewer is still
# exiting can find the directory not yet empty.
trap '{ kill "$view_pid"; wait "$view_pid"; } 2>/dev/null || true; rm -rf "$work"' EXIT
url=""
for _ in $(seq 1 50); do
  url="$(grep -o 'http://127\.0\.0\.1:[0-9]*/' view.log 2>/dev/null | head -n1 || true)"
  [ -n "$url" ] && break
  sleep 0.1
done
[ -n "$url" ] || { echo "report view printed no URL: $(cat view.log)" >&2; exit 1; }
curl -fsS "$url" | grep -q '<title>squint report</title>' || { echo "report view did not serve the viewer at $url" >&2; exit 1; }
curl -fsS "${url}report.json" | grep -q '"report_version":1' || { echo "report view did not serve report.json" >&2; exit 1; }
echo "offline smoke ok: $version"
