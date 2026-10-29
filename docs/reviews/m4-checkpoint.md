# M4 checkpoint: review and coverage findings

Branch `m4-tools` (tasks 4.0 to 4.6) was reviewed by the code-review and test-engineer passes on
2026-10-28, while the arm C evaluation ran, before the owner's review. Every finding is either
fixed on the branch or recorded in `tasks/todo.md` under "M4 review follow-ups".

## Code review

The pass found the alignment design and its ADR sound (constants and the Jacobian checked against
the code), the sphere conventions right and pinned in both Rust and JavaScript, the MCP client
doing what ADR 0015 says with one test per failure mode, the executor split clean, projections
applied inside the tool boundary, the statistics careful, and no secret, private id or non-synthetic
fixture in the diff. Its verdict was "request changes" for the findings below.

| Severity | Finding | Resolution |
|---|---|---|
| Critical | A Gauss-Newton refinement that did not converge (singular normal matrix, non-finite parameters, too few samples, divergence) was reported as "not registered", which `align` turned into "the pair is not the same scene" with an `alignment_failure` evidence item: a numerical failure became a verdict. The loop's fall-through also called an oscillating fit at the iteration cap "converged". | Fixed: refinement ends in `Settled`, `Exhausted` or `Failed`; a failed refinement, or an exhausted one whose coarse and fine estimates agree, is `NotConverged`, which `align` maps to `not_applicable` with no region, like low texture. A small last step at the iteration cap counts as settled. Tests `a_refinement_that_does_not_converge_is_not_converged_rather_than_not_registered` and `a_registration_that_does_not_converge_gives_no_evidence`. |
| Important | Paths handed to a tool were relative to the harness's working directory, so a pack restarted in the run directory could not resolve them. | Fixed: the output directory is made absolute once, so every path a tool receives is absolute. Test `a_tool_is_handed_absolute_paths_to_its_pixels`. |
| Important | A server request with an id (a `ping`) was treated as a stray response and failed every in-flight call. | Fixed: `ping` is answered, other server requests get method-not-found, notifications are ignored. Fake server `--ping`; tested. |
| Important | Cancel removed the pending entry before writing the tombstone, so a late answer in that window failed the sibling calls. | Fixed: the tombstone is written first. |
| Important | The server's stderr was captured in an unbounded buffer and never reached the journal. | Partly fixed: the buffer keeps the last 200 lines and a failed call's error carries them into the journal. A stderr event of its own is recorded as a follow-up. |
| Important | The stored envelope artifact of an `align` result carried the aligned frame's pixels and was truncated at the artifact cap. | Fixed: envelopes are stored without image bytes; the frame files hold the pixels. |
| Important | The run accepted an aligned frame whose root was not a registered image and never validated the transform parameters; a point behind the camera produced NaN. | Fixed: the root must be a raw frame among the call's images; parameters must be nine finite numbers and invertible; a point behind the camera is dropped (`apply` returns an option). |
| Important | The pole test was made against the view rather than the region, so any box in a pole-holding view mapped to the whole polar cap. | Fixed: the projected pole is tested against the region itself in Rust and JavaScript; fixture cases added to `fixtures/render/view-footprints.json`. |
| Important | Seam crossing was measured from the view centre, could not fire at yaw 0, and clipped to one side. | Fixed: a crossing is a jump between consecutive vertices and the footprint is cut into pieces at the seam; SPEC's bbox and polygon rows say so. |
| Important | A bbox crop of a panorama inherited the equirectangular projection, so `view` would render a part as a full sphere. | Fixed: a crop keeps the parent projection only for the full box. |
| Important | A `view` location on a raw frame reached the report with no root geometry. | Fixed: `to_root` replaces it with its footprint. |
| Important | `whole_image` through a bbox crop mapped to the whole parent. | Fixed: it becomes the crop's box. |
| Important | The registry emitted a full-width pole box its own validator refused as seam-straddling. | Fixed: a box from x 0 to 1 is allowed; SPEC says so. |
| Important | `presented_bytes` under-bounded polygon regions and aligned-frame mappings, and the planner change had no test. | Fixed: regions are priced at the 64-vertex polygon, images at 320 bytes, measurements at 160; test `the_planner_prices_the_presentation_the_model_reads_not_the_stored_envelope`. |
| Important | The arm-limited-differences check did not exist and the notes cited a test by a wrong name. | Fixed: `arm_c_prompts_differ_from_arm_a_only_in_the_tools_section` builds every arm of `evalsets/matrix/arm-c.toml` from its real tool selection; notes corrected. A runtime refusal of pairs whose policy or rules differ is recorded as a follow-up. |
| Important | The report's bootstrap hard-coded one hypothesis. | Fixed: the matrix's count flows through `BootstrapSpec`; the paired bootstrap records hypotheses, families and percentiles, and the report prints them. |
| Important | `edge_similarity` and `change_map` stretched frames of different aspect into one grid. | Fixed: aspect ratios differing by more than one percent are `invalid_args`; tests for aspect and projection mismatch. |
| Important | The alignment pyramid builds every level, including those finer than `max_side` that are never refined. | Recorded in tasks/todo.md. |
| Important | `gray.rs` cast a non-finite coordinate to `usize`, which saturates and then overflows. | Fixed: bounded in f64 before the cast. |
| Important | The 2048 px tolerances lived only in an ignored test. | Fixed: `at_2048_a_rotation_with_scale_is_recovered_within_the_recorded_tolerance` under `cfg(not(debug_assertions))`, passing in release at 0.05 px centre, 0.1 px corner, 0.01°. |
| Important | The export script pinned nothing. | Fixed: exact versions in the script header, checked at run time; another version is refused unless asked for. |
| Important | Upstream weights were neither revision-pinned nor hash-verified. | Fixed: revision `aeb262f5…` pinned; `model.safetensors` must hash to the recorded value. |
| Important | The parity tolerance was recorded, never enforced. | Fixed: the script exits non-zero outside tolerance. |
| Important | The provenance record lived only in a gitignored directory and publication read as done. | Fixed: `tools/python/segmenter-v1.record.json` committed with the full golden hashes; MODEL.md says publication is pending and the input is square only. The export was re-run under the pins and reproduced the hash. |
| Suggestion | `view.toml` advertised `maximum = 180` for yaw while the tool refused 180. | Fixed: `exclusiveMaximum`. |
| Suggestion | A non-finite measurement would serialise as `null` and pass the bounds. | Fixed: refused before the envelope is built. |
| Suggestion | A singular registered homography fell back to the identity. | Fixed: an error. |
| Suggestion | docs/tools.md had the pack-versus-built-in precedence backwards; ADR 0015 understated the client's size; PLAN named files that do not exist. | Fixed. |
| Suggestion | Lease release, string ids, reader line bound, `isError = false` with `invalid_args`, summary bound on rejections, frame-file name collisions, decode limits, failure overlap, coarse-search cost, per-pixel basis, unbounded chain walks, late `view` validation on aligned panoramas, silent node skip, unpaired counts and inconclusive note, metrics schema, `[f64; 9]`, opset in the artifact name. | Recorded in tasks/todo.md. |

## Test coverage

The pass found paginated listing and its guards, serial and concurrent pack calls, every failure
mode at the pack level, the `isError` table, projections before the checklist, the Tools section
snapshot, flat `align` through a run, the view and compare goldens, the geometry twin and the
paired flips covered. Its gaps:

| Gap | Resolution |
|---|---|
| Nothing proved an arm C envelope fits after the planner change. | `the_planner_prices_the_presentation_the_model_reads_not_the_stored_envelope` plans a decision with `view` and `align` under `visual_only` and bounds the growth per tool turn and the largest request. |
| A rejected call inside a concurrent batch. | `a_rejected_call_in_a_concurrent_batch_is_answered_while_the_rest_run`. |
| A cancel during a concurrent batch. | `a_cancel_during_a_concurrent_batch_lands_without_waiting_for_the_calls`. |
| A shared server dying between decisions. | Fake server `--exit-after-calls N`; `a_pack_whose_server_died_is_restarted_under_the_next_lease_and_within_a_run`. |
| No equirectangular aligned frame through a run. | `a_finding_on_an_aligned_panorama_maps_back_through_the_rotation`. |
| Image blocks and frame handles over the MCP wire end to end. | Recorded in tasks/todo.md. |
| The arm-limited-differences check. | `arm_c_prompts_differ_from_arm_a_only_in_the_tools_section`. |
| A JSON-RPC `error` reply to `tools/call`. | Recorded. |
| Seam, yaw and pitch refusals in `view`. | Added to `view_refuses_bad_arguments_as_the_models_mistake`. |
| The viewer's chain walker under node; the silent node skip. | Recorded. |
| Stored sphere alignment fixtures. | Recorded. |
| The 2048 px tolerances. | The release-only test above. |
| Paired-metrics edges. | `paired_metrics_ignore_later_repetitions_and_say_why_no_interval_exists`. |
| `executed` after a tool-side `invalid_args`. | Recorded. |
| Offline tests of the export script. | Recorded; the record is now in the tree for such a test to anchor on. |
| `tools list` printing projections. | Recorded. |
