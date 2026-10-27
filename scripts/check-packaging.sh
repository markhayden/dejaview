#!/usr/bin/env bash
# Every asset a crate embeds with include_str!/include_bytes! must sit inside that crate and be in
# the file list `cargo package` would ship, or the published crate cannot build. Checks every
# workspace member; CRATE_DIRS overrides the list (space-separated, for the self-test). Prints the
# embedded assets per crate.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ -z "${CRATE_DIRS:-}" ]; then
  CRATE_DIRS="$(cargo metadata --format-version 1 --no-deps | python3 -I -c 'import json,sys,os; m=json.load(sys.stdin); print(" ".join(os.path.dirname(p["manifest_path"]) for p in m["packages"]))')"
fi
status=0
for crate in $CRATE_DIRS; do
  listing="$(cd "$crate" && cargo package --list --allow-dirty 2>/dev/null)" || { echo "$crate: cargo package --list failed" >&2; status=1; continue; }
  python3 -I - "$crate" "$listing" <<'PY' || status=1
import pathlib, re, sys
crate = pathlib.Path(sys.argv[1]).resolve()
packaged = set(sys.argv[2].split())
pattern = re.compile(r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"')
bad = 0
assets = []
for src in sorted(crate.glob("src/**/*.rs")):
    for rel in pattern.findall(src.read_text()):
        target = (src.parent / rel).resolve()
        try:
            inside = target.relative_to(crate)
        except ValueError:
            print(f"{src.relative_to(crate.parent.parent)}: include of {rel} reaches outside the crate")
            bad += 1
            continue
        if str(inside) not in packaged:
            print(f"{src.relative_to(crate.parent.parent)}: {inside} is embedded but not packaged")
            bad += 1
            continue
        assets.append(str(inside))
name = crate.name
if assets:
    print(f"{name}: embeds " + ", ".join(sorted(set(assets))))
else:
    print(f"{name}: embeds nothing")
sys.exit(1 if bad else 0)
PY
done
[ "$status" -eq 0 ] && echo "packaging ok"
exit "$status"
