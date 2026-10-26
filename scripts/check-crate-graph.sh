#!/usr/bin/env bash
# Compares the workspace's actual crate-to-crate edges (all features enabled) with the allowed
# edges in scripts/crate-graph.txt (override with CRATE_GRAPH_FILE for tests). Exits 1 and lists
# any unexpected edge.
set -euo pipefail
cd "$(dirname "$0")/.."
graph_file="${CRATE_GRAPH_FILE:-scripts/crate-graph.txt}"
meta_file="$(mktemp)"
trap 'rm -f "$meta_file"' EXIT
cargo metadata --format-version 1 --all-features --no-deps > "$meta_file"
python3 -I - "$graph_file" "$meta_file" <<'PY'
import json, sys, pathlib
graph_file, meta_file = sys.argv[1], sys.argv[2]
meta = json.loads(pathlib.Path(meta_file).read_text())
members = {p["name"] for p in meta["packages"]}
actual = set()
for p in meta["packages"]:
    for d in p["dependencies"]:
        if d["name"] not in members:
            continue
        kind = "dev" if d["kind"] == "dev" else "prod"
        actual.add((kind, p["name"], d["name"]))
allowed = set()
for line in pathlib.Path(graph_file).read_text().splitlines():
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    kind, rest = line.split(" ", 1)
    src, dst = [s.strip() for s in rest.split("->")]
    allowed.add((kind, src, dst))
bad = sorted(actual - allowed)
for kind, src, dst in bad:
    print(f"unexpected {kind} edge: {src} -> {dst}")
sys.exit(1 if bad else 0)
PY
echo "crate graph ok"
