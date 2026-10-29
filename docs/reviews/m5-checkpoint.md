# M5 checkpoint: review and coverage findings

Branch `m5-ml-tool` (tasks 5.1 to 5.6, the arm C tool-interface fix and the regions gate) was
reviewed by the code-review and test-engineer passes on 2026-10-29, after the gate had scored and
before the owner's review. Every finding is either fixed on the branch or recorded in
`tasks/todo.md` under "M5 review follow-ups".

## Code review

The pass found `fetch` sound (streamed through a temp file, every byte hashed, nothing kept on a
mismatch, re-fetch idempotent), the `view` refusal and the raw-frame acceptance right and tested
with the exact failing shapes from the arm C run, the two photometric views and the class pass of
`regions` explained in the code and the guide in the same words with a real correctness test, the
cache-key test complete, and no local path in any published summary. Its verdict was "request
changes" for the findings below.

| Severity | Finding | Resolution |
|---|---|---|
| Critical | `tools setup` ran the pack's `fetch` (and `uv sync`) with the caller's whole environment, credentials included, while the server launch filters it through the allowlist. | Fixed: both subprocesses take the environment as a parameter, filter it through `dejaview_core::env::allowlist` and `env_clear()`, and add only the harness's own variables. Test `the_fetch_subprocess_sees_the_allowlist_and_the_harness_variables_and_no_credential` passes an environment holding a credential and a script that fails if it arrives. |
| Important | The server had no exception boundary: a missing segmenter artifact, an unreadable image or a non-object hint escaped the handler, the process died, and a shared pack was gone for every later decision. | Fixed: `Server.call` answers a JSON-RPC error (-32603, the pack's words) for `SegmenterUnavailable`, `OSError` and `ValueError` and goes on serving; `initialize` loads the segmenter once when the artifact is present so the first call does not pay for it; `regions` ignores hints that are not an object. Tests `a_missing_segmenter_is_an_error_reply_and_the_server_goes_on` and `unreadable_pixels_are_an_error_reply_not_a_crash`. Failing `initialize` when the artifact is absent (the spec's `error: resolve`) is recorded: the provisioning and CI tests connect to the pack without the 1.2 GB artifact. |
| Important | `classes` emitted up to ten measurements for a cap of eight and the truncation dropped the two confidences the manifest promises. | Fixed: the confidences come first, then the shares by rank across both sides. Test `four_classes_a_side_keep_both_confidences_under_the_measurement_cap`. |
| Important | `components` was iterative min-label propagation whose iterations grow with a component's path length: a serpentine outline took 4.3 s on the 256 grid, twice per call. | Fixed: a two-pass run-based union-find, proportional to the number of runs. Test `components_on_a_serpentine_mask_stay_fast_and_count_one` holds a 64-turn snake under one second. |
| Important | `classes.read` upsampled the 128-grid maps to the full image to read one box, hundreds of megabytes on a large pair. | Fixed: the box is read at the map's own resolution. |
| Important | The per-component loop in `regions` was `O(n · H·W)`. | Fixed: areas, boxes, residual and disagreement sums and the majority class per component come from one pass of `bincount` and `minimum.at`/`maximum.at`. |
| Suggestion | The tools overwrote the pack version in provenance with their manifest version after the fact. | Fixed: `envelope()` takes the version; the pack's own tools pass their manifest's. |
| Suggestion | `composition` took an unused confidence parameter. | Fixed. |
| Suggestion | The `classes` fixtures were recorded from the code under test. | Said so in the test module's docstring; the correctness evidence is the parity test and the planted-change test. |
| Suggestion | Load-time integrity of the artifact is a size check only. | Recorded in MODEL.md: `fetch` is the point of verification. |
| Suggestion | A 1.2 GB `Fetch::Release` runs silently under `.output()`. | Recorded: stream stderr during setup. |
| Suggestion | `results-doc.py` matched the evaluation directory's prefix without a separator. | Fixed, with the separator. |
| Suggestion | `docs/tools.md` still said the pack had no dependencies beyond numpy and Pillow and one tool. | Fixed. |
| Suggestion | The mirror mistake, an image id given to a frame argument, could get the same help in its refusal. | Recorded. |

## Test coverage

The pass found the happy paths and the tamper paths well covered and reported the gaps below.

| Gap | Resolution |
|---|---|
| A missing artifact killed the server; nothing tested it. | Fixed above, two server tests. |
| The segmenter-marked tests never run in CI, and two of them needed no artifact. | The two run offline now. The parity, planted-change and fixture tests still need the artifact, which is unpublished; recorded: CI runs them once the release assets exist, with `fetch` in the python job. |
| `classes` dropped its confidences at the cap. | Fixed above. |
| `regions`: the cap and renumbering, `MIN_AREA`, the second-pass dedupe, the overlay's size cap and the projection refusal were untested. | Tests added on synthetic maps with a fake segmenter: sixteen blocks keep twelve with "4 weaker candidates were left out"; a speck is dropped and a class box inside a photometric box is not counted twice; a 2000-pixel frame gives a 1024-pixel overlay; flat against equirectangular is `invalid_args`. |
| The hints test passed if hints did nothing. | Test `hints_move_scores_by_exactly_their_factors`: half for `ignore`, 1.25 for `preserve`, and a non-object hint ignored. |
| `fetch` from a bad path raised an uncaught `ValueError` and left a temp file; the mid-stream failure had no test. | Fixed: a source that is neither a directory nor an http(s) URL is a `FetchError`, every failure unlinks the temp file. Two tests. |
| `Installed::fetch`'s failure path ran only in the ignored 1.2 GB test. | Unit tests with a script executable: a failure is `resolve` with the pack's words; `Skip` never runs it. |
| `composition`'s fold to four was untested. | Tested. |
| No test tied the receipt to the published set: seal verified, hashes equal, attestation present, results copy identical. | Test `every_published_seal_holds_and_every_receipt_agrees_with_its_set`. |
| The relative-output bug was covered only by a unit test; the script's absolute branch never ran. | CLI test with a relative `--out` and a summary rewritten absolute. |
| MODEL.md and the shipped record could drift. | Test `model_md_agrees_with_the_shipped_record`. |
| Low: unknown tool, `--version`, unknown command; `raw:nope` refused in the model's words. | Tested. |
| `tools setup --no-fetch` and `--models-from` exclusivity. | Recorded (clap enforces `conflicts_with`). |
| A missing `admissions.toml` meaning "nothing admitted". | Recorded: the gate creates the file; arm D on an absent file is covered by the gate test's default. |
