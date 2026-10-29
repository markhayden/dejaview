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
measurements, regions, images, transforms and provenance. The model reads a presentation of it
(summary, measurements, regions with their checklist ids, the images' frame lines) and receives
the images as images; the envelope planner prices that presentation (`Bounds::presented_bytes`,
from the manifest's text bounds) plus the images, not the stored envelope with its inline pixels. Regions become checklist evidence items
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
manifests embedded from `crates/dejaview-tools/manifests/`, listed by `dejaview tools list`. A pack
tool of the same name takes the built-in's place (packs are registered first). Shipped so far:

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
SIGTERM, waits, then SIGKILL. `structuredContent` is the envelope, validated against the tool's
`outputSchema`; the content blocks must be exactly the image blocks of `images`, in order and
media type, and nothing else; `isError` with an `invalid_args` envelope is the one recoverable
form (`the_is_error_truth_table_is_exhaustive` enumerates the rest).

The client is multiplexed (ADR 0015): a reader thread routes each response to the request waiting
for its id, so tools whose manifests declare `concurrent = true` run together on one process while
the others take turns; a stray id, a malformed frame or the end of the stream fails every pending
request and taints the process. `tools/list` follows `nextCursor` to the end and refuses a
repeated cursor or more than 64 pages. A new run's lease waits, within its one deadline, for the
previous run's in-flight calls before resetting or restarting. The context's `frames` carry each
named frame's `chain` down to its raw root, so a pack can reproduce any location with no state of
its own. In the run, a batch of calls whose tools all declare `concurrent = true` is dispatched
together and settled in call order, so evidence items keep the batch's order whatever finishes
first.

Configured per pack as `[tools.packs.<name>] command = "..."` (plus `args`, `env`,
`startup_timeout_s`) for an executable, or `package` and `version` for a provisioned bundle.
With `shared = true` one process serves a whole evaluation: the binary connects the pack once and
every decision's runs take leases on it, keyed by run directory and run number so one decision's
`run-1` is not another's; a new lease waits for the previous run's calls, then resets or restarts.
The failure modes have a test each against the fake server: a server dying mid-call, a malformed
frame, a response for an unknown id, a late answer after cancellation, reset failure, shutdown
escalation, and two runs contending for one pack.

## Writing a tool, with `view` as the example

A built-in is a Rust type implementing `dejaview_core::tool::Tool` in
`crates/dejaview-tools/src/builtin/`, registered in `builtin::all()`. `view`
(`builtin/view.rs`, manifest `manifests/view.toml`) shows every part:

1. **The manifest** is a TOML file embedded with `include_str!` and parsed once in `new()`. It
   names the tool, says in plain words what it does, when to use it and when not to, and what its
   output means; declares the cost tier; lists which arguments take image ids (`image_args`) and
   frame ids (`frame_args`), so the harness resolves them and injects the pixels; bounds what the
   tool may return (`returns`); names the frames it makes (`frames_out`) and the projections it
   supports (`projections`); and gives the model schema (what the provider sees, closed, with a
   description per argument), the input schema (the model schema plus `_dejaview`) and the output
   schema (the measurement names and units it may report). `view` takes `frame`, an optional
   `also`, and either `bbox` or `yaw`, `pitch` and `fov`.
2. **Arguments** arrive validated against the model schema's required keys and closed shape; the
   tool still checks types and ranges itself, and answers a mistake with an `invalid_args`
   envelope (`super::invalid_args`), which the harness returns to the model as a tool error and
   never counts as an operational failure. `view` refuses both forms at once, angles on a flat
   frame, a box under eight pixels and `fov` outside 10 to 120 this way.
3. **Pixels** come through the context: `ctx.frames[id]` is a `FrameHandle` with the frame's
   projection, parent, mapping, chain to its root and a lossless PNG by path under the run
   directory (`raster::load` reads it); `ctx.images[id]` likewise for image arguments. Never a
   path from the model.
4. **Results** go back as an `Envelope`: a summary the model reads, measurements (named in the
   output schema), regions (which become checklist evidence of the manifest's `evidence_kind`),
   images and transforms. A returned image is a new frame: `crop:<call id>:<n>` with its parent
   and the mapping back into it (`raster::frame` encodes the PNG and names it), or
   `aligned:<image>:<transform>` after a transform in the same envelope. The harness checks every
   bound in `returns`, stores the envelope, registers the frames, shows the model the images and
   maps any location drawn in them back to the root image.
5. **Tests** live in `crates/dejaview-tools/tests/`: the tool called directly with a context
   built from synthetic images (`dejaview_testkit::images`), its outputs checked against what the
   maths says (a crop equals the region it cuts; a perspective view matches the analytic view of
   the same scene and a stored golden), its refusals checked one by one, and one run through
   `run::check` with a scripted provider showing the frame in the report and a finding on it
   mapped back to the root.

An MCP tool is the same contract over stdio: the manifest's fields travel in the tool listing's
`dejaview` annotation, the context arrives as `_dejaview`, and the envelope comes back as
`structuredContent` with the images as content blocks.

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

**The segmenter artifact** (task 4.6, `tools/python/MODEL.md`). `scripts/export_segmenter.py`
exports `tue-mps/ade20k_semantic_eomt_large_512` (EoMT-L on DINOv2, MIT weights and code) to ONNX
with inline dependencies of its own, checks the export against PyTorch on synthetic scenes
generated from a seed, and writes `record.json`; `MODEL.md` records the licences, the procedure
and versions, the artifact's SHA-256 (the script reproduces it), the preprocessing version, the
runtime and execution provider, the parity measured and the tolerance the parity test holds, and
the release URL the owner publishes the artifact and goldens at. Task 5.3's `fetch` downloads
from it and refuses a hash mismatch.

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
