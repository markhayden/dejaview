#!/usr/bin/env bash
# Proves the published crates are complete: package every crate exactly as `cargo publish` would,
# install `dejaview` from those .crate files in a scratch directory outside the repository (the
# sibling crates come from the extracted packages through a [patch] section, as crates.io would
# supply them), then run the offline smoke on the installed binary.
set -euo pipefail
cd "$(dirname "$0")/.."
repo="$(pwd)"
version="$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="dejaview"))')"
# The stage is a fresh directory: a caller-named one must not already hold anything (it is wiped
# at the end), otherwise a scratch directory that is removed on exit.
if [ -n "${REGISTRY_STAGE_DIR:-}" ]; then
  stage="$REGISTRY_STAGE_DIR"
  if [ -e "$stage" ] && [ -n "$(ls -A "$stage")" ]; then
    echo "REGISTRY_STAGE_DIR $stage exists and is not empty; refusing to use it" >&2; exit 1
  fi
else
  stage="$(mktemp -d)"
fi
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/crates" "$stage/.cargo"
# Packaging verifies each tarball by building it against the sibling crates staged in a temporary
# registry. The extracted sources carry fixed timestamps and are cached by name and version, so
# with the shared target directory a sibling compiled on an earlier run of the same version is
# reused as if nothing had changed, and a crate "verifies" against a stale dependency. A target
# directory of this run's own, and the cached extractions of our crates removed, make the
# verification see the sources as packaged now.
rm -rf "$HOME"/.cargo/registry/src/*/dejaview-*-"$version"
CARGO_TARGET_DIR="$stage/target" cargo package --workspace --locked --allow-dirty
packages="$stage/target/package"
for crate in "$packages"/*-"$version".crate; do tar xzf "$crate" -C "$stage/crates"; done
{
  echo "[patch.crates-io]"
  for dir in "$stage"/crates/*/; do
    name="$(basename "$dir")"; name="${name%-"$version"}"
    # The binary itself is what gets installed; every other crate, the optional testkit included,
    # stands in for its crates.io release: cargo resolves an optional dependency even when its
    # feature is off, then reports the patch unused. Without it the install fails with "no matching
    # package named dejaview-testkit".
    case "$name" in dejaview) continue ;; esac
    echo "$name = { path = \"$dir\" }"
  done
} > "$stage/.cargo/config.toml"
(cd "$stage" && cargo install --path "$stage/crates/dejaview-$version" --root "$stage/root" --locked)
"$repo/scripts/offline-smoke.sh" "$stage/root/bin/dejaview"
echo "registry install ok: dejaview $version from $(ls "$packages"/*.crate | wc -l | tr -d ' ') crates"
