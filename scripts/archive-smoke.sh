#!/usr/bin/env bash
# The offline smoke on a release archive, as a user would receive it: verify the checksum file
# when one sits beside the archive, unpack into a scratch directory, find the `dejaview` binary
# and run scripts/offline-smoke.sh on it. Works on any architecture's archive run on that
# architecture (CI) or the host's (`just archive-build` then `just archive-smoke`).
#
#   scripts/archive-smoke.sh target/distrib/dejaview-aarch64-apple-darwin.tar.xz
set -euo pipefail
archive="${1:?usage: archive-smoke.sh <dejaview archive (.tar.xz, .tar.gz or .zip)>}"
archive="$(cd "$(dirname "$archive")" && pwd)/$(basename "$archive")"
here="$(cd "$(dirname "$0")" && pwd)"
[ -f "$archive" ] || { echo "no such archive: $archive" >&2; exit 1; }

if [ -f "$archive.sha256" ]; then
  expected="$(awk '{print $1}' "$archive.sha256")"
  actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
  [ "$expected" = "$actual" ] || { echo "checksum mismatch for $archive" >&2; exit 1; }
elif [ "${REQUIRE_CHECKSUM:-0}" = "1" ]; then
  # CI always has the checksum file dist wrote beside the archive; its absence is a broken build.
  echo "no checksum file beside $archive" >&2; exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
case "$archive" in
  *.tar.xz) tar xJf "$archive" -C "$work" ;;
  *.tar.gz) tar xzf "$archive" -C "$work" ;;
  *.zip) unzip -q "$archive" -d "$work" ;;
  *) echo "unknown archive format: $archive" >&2; exit 1 ;;
esac
bin="$(find "$work" -type f -name dejaview | head -n 1)"
[ -n "$bin" ] || { echo "no dejaview binary inside $archive" >&2; exit 1; }
chmod +x "$bin"
"$here/offline-smoke.sh" "$bin"
echo "archive smoke ok: $(basename "$archive")"
