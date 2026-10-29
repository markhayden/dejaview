#!/usr/bin/env bash
# Publish every workspace crate to crates.io in dependency order (Cargo orders them: core,
# testkit, providers, tools, eval, squint). The owner runs this from a clean, tagged checkout;
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
version="$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="squint-cli"))')"
if [ -z "$dry_run" ]; then
  if [ -n "$(git status --porcelain)" ]; then echo "working tree is not clean" >&2; exit 1; fi
  if ! git tag --points-at HEAD | grep -qx "v$version"; then
    echo "HEAD is not tagged v$version; tag the release first" >&2; exit 1
  fi
fi
scripts/check-packaging.sh
# Verification builds each tarball against the sibling crates staged in a temporary registry,
# whose extracted sources carry fixed timestamps and are cached by name and version; a target
# directory of this run's own and the cached extractions of our crates removed make it build the
# sources as packaged now (the same stale-sibling trap scripts/registry-install-test.sh avoids).
rm -rf "$HOME"/.cargo/registry/src/*/squint-*-"$version"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
CARGO_TARGET_DIR="$scratch/target" cargo publish --workspace --locked ${dry_run:+--allow-dirty} $dry_run
echo "published${dry_run:+ (dry run)} squint $version"
