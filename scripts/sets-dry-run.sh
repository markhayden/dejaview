#!/usr/bin/env bash
# The toolkit's offline dry run: build the plan fixture with the fake generator, confirm every
# draft label as the owner would, promote, and have the harness check the result. Proves the
# toolkit writes what `dejaview sets check` accepts, with no network and no spend.
set -euo pipefail
cd "$(dirname "$0")/.."
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
(cd scripts/sets && uv run --frozen --offline --no-sync dejaview-sets build \
  --plan ../../fixtures/sets/houses-v1.plan.toml \
  --out "$work/evalsets/source-fidelity/houses-v1" --owner "$work/owner" --fake)
draft="$work/owner/source-fidelity/houses-v1/labels-draft.toml"
sed -i.bak 's/status = "draft"/status = "confirmed"/g' "$draft" && rm -f "$draft.bak"
(cd scripts/sets && uv run --frozen --offline --no-sync dejaview-sets promote --draft "$draft" --set "$work/evalsets/source-fidelity/houses-v1")
cargo run -q -p dejaview -- sets check "$work/evalsets"
