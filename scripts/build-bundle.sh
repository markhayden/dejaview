#!/usr/bin/env bash
# Builds the Python tool pack's provisioning bundle (SPEC "Python tool pack"): the wheel, a
# provisioning project pinning it by path, that project's uv.lock, MODEL.md, and bundle.toml with
# the SHA-256 of every file. Output: tools/python/dist/bundle-<version>/.
set -euo pipefail
cd "$(dirname "$0")/../tools/python"

version=$(uv version --short)
rm -rf dist
uv build --wheel --out-dir dist/wheel >/dev/null
wheel=$(ls dist/wheel/*.whl)
wheel_name=$(basename "$wheel")
bundle="dist/bundle-$version"
mkdir -p "$bundle"
cp "$wheel" "$bundle/"
cp MODEL.md "$bundle/"
cat > "$bundle/pyproject.toml" <<PY
[project]
name = "dejaview-tools-provisioning"
version = "$version"
requires-python = ">=3.12"
dependencies = ["dejaview-tools==$version"]

[tool.uv]
package = false

[tool.uv.sources]
dejaview-tools = { path = "$wheel_name" }
PY
(cd "$bundle" && uv lock --quiet)
python3 - "$bundle" "$version" <<'PY'
import hashlib, pathlib, sys
bundle = pathlib.Path(sys.argv[1]); version = sys.argv[2]
lines = ["[bundle]", 'package = "dejaview-tools"', f'version = "{version}"', 'python = ">=3.12"',
         'executable = ".venv/bin/dejaview-tools"', "", "[bundle.files]"]
for f in sorted(p for p in bundle.iterdir() if p.name != "bundle.toml"):
    lines.append(f'"{f.name}" = "{hashlib.sha256(f.read_bytes()).hexdigest()}"')
(bundle / "bundle.toml").write_text("\n".join(lines) + "\n")
PY
echo "$bundle"
