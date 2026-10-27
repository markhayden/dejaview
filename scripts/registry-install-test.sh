#!/usr/bin/env bash
# Proves the published crates are complete: package every crate exactly as `cargo publish` would,
# install `dejaview` from those .crate files in a scratch directory outside the repository (the
# sibling crates come from the extracted packages through a [patch] section, as crates.io would
# supply them), then run the offline smoke on the installed binary.
set -euo pipefail
cd "$(dirname "$0")/.."
repo="$(pwd)"
version="$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="dejaview"))')"
cargo package --workspace --locked --allow-dirty
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
for crate in "$repo"/target/package/*-"$version".crate; do tar xzf "$crate" -C "$stage/crates"; done
{
  echo "[patch.crates-io]"
  for dir in "$stage"/crates/*/; do
    name="$(basename "$dir")"; name="${name%-"$version"}"
    # The binary itself is what gets installed; the testkit is a dev-only dependency the
    # installed binary never enables, so patching it would only draw an "unused patch" warning.
    case "$name" in dejaview|dejaview-testkit) continue ;; esac
    echo "$name = { path = \"$dir\" }"
  done
} > "$stage/.cargo/config.toml"
(cd "$stage" && cargo install --path "$stage/crates/dejaview-$version" --root "$stage/root" --locked)
"$repo/scripts/offline-smoke.sh" "$stage/root/bin/dejaview"
echo "registry install ok: dejaview $version from $(ls "$repo"/target/package/*.crate | wc -l | tr -d ' ') crates"
