# Tools

Tools supply evidence; the model decides what it means; code computes the verdict. This page
describes the tool contract as built so far. The full design is SPEC.md "Tools".

## Manifest

Every tool describes itself once, in a manifest (`schemas/manifest-v1.json`): name, version,
purpose and guidance text, cost tier, dependencies, which arguments take image ids and frame ids,
the evidence kind its regions carry, the bounds its results stay within (`returns`), the
evaluation projections it supports (`projections`), and three schemas: the model schema (what the provider receives), the input schema (the model schema plus
the required `_dejaview` object the harness injects), and the output schema. `manifest_version`
defaults to the current version and is validated. `fixtures/tools/regions.toml` is the example.

## Envelope

Every result is an envelope (`schemas/envelope-v1.json`): status, a one-paragraph summary,
measurements, regions, images, transforms and provenance. Regions become checklist evidence items
by evidence kind; images become crop frames; `invalid_args` is answered to the model as a tool
error; anything else out of bounds is `error: tool`.

## Evaluation projections

An evaluation arm may run a tool under a named projection of its envelope (SPEC "Evaluation
projections"): `ToolRegistry::with_projections` wraps the named tools so every envelope is
projected before the run, the checklist or the model sees it. The only projection so far is
`visual_only`, the control arms' `align`: images and transforms (with their parameters) kept,
`quality`, measurements and regions withheld, a fixed summary, `status` mapped to `ok` or
`not_applicable`. A tool must declare a projection in its manifest to run under it; the report's
tool call entry names the projection and its stored envelope is the projected one. The runner
takes the projections from the arm (`align:visual_only` in the descriptor's selections).

## Context injection

After validating the model's arguments against the model schema (no `_dejaview`, no unknown
keys, required keys present), the harness resolves image and frame arguments through its frame
registry and injects `_dejaview`: run and call ids, the named images with their lossless pixels by
path under the run directory, the named frames with their transform chains, and the rules' hints
for the tool. An id that is not in the registry, such as a path, is an unknown id and never
touches the filesystem.

## Transports

**Built-in (in-process Rust).** `Tool` implementations in `crates/dejaview-tools/src/builtin/`,
manifests embedded from `crates/dejaview-tools/manifests/`, registered ahead of any pack of the
same name and listed by `dejaview tools list`. Shipped so far:

- `view` (`frame`, optional `also`; either `bbox` or `yaw`, `pitch` and `fov`): a crop of a frame
  by normalised box, snapped outward to whole pixels (the mapping records the pixels actually cut)
  and scaled to a 1024 px long edge when larger; or a square perspective view into an
  equirectangular frame, sized to the panorama's own resolution over the field of view between
  256 and 1024 px, rendered by inverse-mapping every pixel to the sphere and sampling the panorama
  across the seam (`dejaview_core::frames::sphere`). `also` repeats the same region in a second
  frame of the same projection. Boxes under 8 px, angles on a flat frame, both forms at once, or
  `fov` outside 10 to 120 are `invalid_args`. Returned frames are `crop:<call>:<n>` with the
  mapping the registry maps locations through: a box on a view frame becomes the polygon of its
  footprint on the panorama, cut at the seam and flagged when it straddles it, a full-width box
  when it holds a pole.
- `resize` (`frame`, `long_edge` 64 to 1024): the frame scaled down with a triangle filter,
  mapping the whole parent; it never enlarges.
- `align` (`image`, `against`): registers one image onto another of the same projection with the
  estimators of ADR 0014 (a homography for flat pairs, a rotation for panoramas). A registered
  pair returns the transform `t1` with its quality, the measurements (`ncc`, `inlier_fraction`,
  `overlap`, `texture`, and `translation_px`, `rotation_deg`, `scale` or `yaw_deg`, `pitch_deg`,
  `roll_deg`) and the aligned frame `aligned:<image>:t1`: the image warped losslessly into
  `against`'s grid, capped at 1024 px, whose coordinates are `against`'s and whose pixels are the
  image's. A location on it maps back to the image's raw frame through the inverse transform
  (`frames::aligned`): a box becomes the polygon of its corners under a homography, its footprint
  under a rotation. A pair that does not register is `not_applicable` with one
  `alignment_failure` region over the image, the evidence SF2 binds to; a textureless pair is
  `not_applicable` with no region. It declares the `visual_only` projection the control arms run
  it under.
- `edge_similarity` (`frame`, `against`, two frames in one grid such as `raw:source` and
  `aligned:candidate:t1`): the normalised cross-correlation of the two frames' edge-strength maps
  (gradient magnitude of the blurred luminance at a working size of 768 px), 1 for the same
  structure, 0 when either frame has no edges; with the weaker frame's mean edge energy. It
  survives a tone change and drops for a different scene. Requires `align`.
- `change_map` (`frame`, `against`): a grey map in `frame`'s grid of the absolute difference of
  the two frames' locally contrast-normalised luminances, saturating at three units, returned as
  `crop:<call>:0` over the whole frame with `changed_fraction`, the share of pixels differing by
  more than one unit. Bright where structure changed, dark under a uniform tone or colour change.
  Requires `align`.

The alignment estimators behind `align` are in place (`dejaview_tools::align`, ADR 0014): `align_flat` returns
the homography from source to candidate pixel coordinates and `align_sphere` the rotation between
two equirectangular panoramas, each with `overlap`, `ncc`, `inlier_fraction` and `texture` and
an outcome of `registered`, `low_texture` or `not_registered`. Direct registration on locally
contrast-normalised pyramids: an exhaustive coarse search, then robust Gauss-Newton per level;
measured within 0.05 px and 0.01° at 2048 px on synthetic scenes with exact ground truth, and
refusing unrelated, textureless, mirrored, quarter-turned and barely overlapping pairs
(`fixtures/images/align/`).

**MCP over stdio, revision 2025-11-25** (`crates/dejaview-tools/src/mcp/`). The client is
synchronous like the `Tool` trait. On connect it spawns the server with the environment allowlist
(`PATH`, `HOME`, the locale, the pack's declared variables minus every credential name), sends
`initialize` and requires exactly `2025-11-25` and the `tools` capability (otherwise
`error: resolve`), sends `notifications/initialized`, lists tools following `nextCursor`, and
freezes the manifests: each tool's `name`, `inputSchema` and `outputSchema` from the native
fields, the rest from a `dejaview` object in its annotations, with `description` equal to
`purpose`.

A pack serves one run at a time under a lease. A run acquires the lease before its first call,
within `startup_timeout_s`, one deadline for everything: `dejaview/reset` and its acknowledgement
(after which no prior-run state survives), or, when the server answers method-not-found, hangs, or
is tainted, shutdown and a restart whose tool list must equal the frozen one. A call runs under
`budget.tool_timeout_s`; past it the client sends `notifications/cancelled`, tombstones the id so
a late answer is ignored, and marks the process unreusable. Shutdown closes stdin, waits, sends
SIGTERM, waits, then SIGKILL. `structuredContent` is the envelope; image content blocks must match
`images` in count and media type; `isError` with an `invalid_args` envelope is the one recoverable
form.

Configured per pack as `[tools.packs.<name>] command = "..."` (plus `args`, `env`,
`startup_timeout_s`) for an executable, or `package` and `version` for a provisioned bundle.
Pagination beyond a cursor loop, concurrent calls, shared leases across an evaluation and the
`rmcp` decision are M4.

## The Python pack and its provisioning bundle

`tools/python/` is a uv project (`dejaview-tools`, no runtime dependencies yet) whose
`dejaview-tools serve` runs the stdio server; this version carries one tool, `echo_region`, a
transport proof. `just python-check` lints and tests it.

`scripts/build-bundle.sh` (`just bundle`) builds the wheel and assembles
`tools/python/dist/bundle-<version>/`: the wheel, a provisioning project whose `pyproject.toml`
pins `dejaview-tools==<version>` by path to that wheel, its `uv.lock`, `MODEL.md`, and
`bundle.toml` with the SHA-256 of every file, the declared Python and the executable path.

`dejaview tools setup --bundle <dir>` verifies every digest (a mismatch or an unlisted file is
`error: resolve` and nothing is installed), copies the bundle to
`$XDG_DATA_HOME/dejaview/tools/<package>-<version>/`, verifies the copy, and runs
`uv sync --frozen --no-dev --python <declared>` there. A pack configured by `package` and
`version` is then launched as `<dir>/.venv/bin/dejaview-tools serve` with `UV_OFFLINE=1`,
`UV_PYTHON_DOWNLOADS=never` and the lock digest in `DEJAVIEW_LOCK_DIGEST`, which the pack echoes
in every envelope's provenance. Downloading a published bundle by name arrives with M5.

`just provisioning-test` builds the bundle and runs the ignored tests in
`crates/dejaview-tools/tests/provisioning.rs`: a clean-directory install whose executable
reports the version the lock pins, an `echo_region` call through MCP, and a tampered bundle
refused. CI runs the same install and call inside `unshare -rn`, a namespace with no network, as
the proof that the launch needs none.

## Testing

`dejaview-testkit` ships `dejaview-testkit-mcp`, a fake server with flags for every misbehaviour
the contract guards against (`--protocol-version`, `--no-tools-capability`, `--no-reset`,
`--hang-reset`, `--slow-call-ms`, `--ignore-stdin-close`, `--envelopes <file>`), and
`mcp_server::path()` builds and locates it. `cargo test -p dejaview-tools -- mcp::minimal` runs
the lifecycle tests.
