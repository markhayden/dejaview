#!/usr/bin/env bash
# The toolkit's offline dry run: build the plan fixture with the fake generator, confirm every
# draft label as the owner would, promote, and have the harness check the result. Proves the
# toolkit writes what `squint sets check` accepts, with no network and no spend.
set -euo pipefail
cd "$(dirname "$0")/.."
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
for set_id in source-fidelity/houses-v1 reference-consistency/pets-v1 source-fidelity/scenes-v1 reality-checks/people-v1 brief-compliance/stills-v1 product-fidelity/packshots-v1 source-fidelity/adversarial-v1 reference-consistency/adversarial-v1 reality-checks/adversarial-v1 brief-compliance/adversarial-v1 product-fidelity/adversarial-v1; do
  case "$set_id" in */adversarial-v1) plan="fixtures/sets/adversarial-$(dirname "$set_id").plan.toml" ;; *) plan="fixtures/sets/$(basename "$set_id").plan.toml" ;; esac
  (cd scripts/sets && uv run --frozen --offline --no-sync squint-sets build \
    --plan "../../$plan" --out "$work/evalsets/$set_id" --owner "$work/owner" --fake)
  draft="$work/owner/$set_id/labels-draft.toml"
  sed -i.bak 's/status = "draft"/status = "confirmed"/g' "$draft" && rm -f "$draft.bak"
  (cd scripts/sets && uv run --frozen --offline --no-sync squint-sets promote --draft "$draft" --set "$work/evalsets/$set_id")
done
cargo run -q -p squint-cli -- sets check "$work/evalsets"
