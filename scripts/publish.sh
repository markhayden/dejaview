#!/usr/bin/env bash
# Publish every workspace crate to crates.io in dependency order (Cargo orders them: core,
# testkit, providers, tools, eval, dejaview). The owner runs this from a clean, tagged checkout;
# CI and anyone else run it with --dry-run, which packages and verifies without uploading.
#
#   scripts/publish.sh --dry-run
#   scripts/publish.sh            # needs `cargo login` and a tag v<version> on HEAD
set -euo pipefail
cd "$(dirname "$0")/.."
dry_run=""
case "${1:-}" in
  --dry-run) dry_run="--dry-run" ;;
  "") ;;
  *) echo "usage: scripts/publish.sh [--dry-run]" >&2; exit 64 ;;
esac
version="$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="dejaview"))')"
if [ -z "$dry_run" ]; then
  if [ -n "$(git status --porcelain)" ]; then echo "working tree is not clean" >&2; exit 1; fi
  if ! git tag --points-at HEAD | grep -qx "v$version"; then
    echo "HEAD is not tagged v$version; tag the release first" >&2; exit 1
  fi
fi
scripts/check-packaging.sh
cargo publish --workspace --locked ${dry_run:+--allow-dirty} $dry_run
echo "published${dry_run:+ (dry run)} dejaview $version"
