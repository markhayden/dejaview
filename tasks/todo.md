# dejaview task checklist

Mirror of `PLAN.md` draft 4. Tick a task only when its acceptance criteria and verification pass
and the commit is on the milestone branch. Checkpoints need the owner's review before merge and
tag.

## Pre-M0
- [ ] Initial documentation commit on `main` (SPEC, PLAN, todo, ADRs, reviews, .gitignore; brief excluded)

## M0 Scaffold
- [x] 0.1 Workspace, crate skeletons, tool bootstrap, crate-graph check
- [x] 0.2 CI, lint, pre-commit hook, ignore rules
- [x] 0.3 Docs skeleton, review checklist, README sections
- [x] 0.4 Owner readiness document
- [x] Checkpoint M0 → tag `m0` (2026-10-26)

## M1 Skeleton
- [x] 1.1 Walking slice → tag `m1.1`
- [x] 1.2 Canonical images and frames registry
- [x] 1.3 Config loading, validation, shorthand job
- [x] 1.4a Obligation model, merge, validation
- [x] 1.4b Instances, rendering, shipped rule sets
- [x] 1.5a Manifest and envelope types, fake tool
- [x] 1.5b Checklist and evidence binding
- [x] 1.6a Submission schema and semantic validation
- [x] 1.6b Aggregation and decision tables
- [x] 1.7a State machine: tool turns
- [x] 1.7b State machine: final turn and terminal states → tag `m1.2`
- [x] 1.8 Repair protocol and retries
- [x] 1.9 Cost ledger
- [x] 1.10a Journal and artifact durability
- [x] 1.10b Report, schema, rebuild, prune
- [x] 1.11 Cache at the decision boundary
- [x] 1.12 Isolation, environment filter, exit codes
- [x] 1.13 Minimal MCP client against a fake server
- [x] 1.14 Minimal provisioning bundle proof
- [x] 1.15 Statistics contract frozen (ADR 0016 RNG)
- [x] Checkpoint M1 → tag `m1` (2026-10-26; `just check` under a minute, docs filled, review and coverage passes applied: docs/reviews/m1-checkpoint.md)

## M2 Provider adapters
- [x] 2.1 Model catalog and capability validation
- [x] 2.2 Wire schema transformations
- [x] 2.3 Image preparation and preflight
- [x] 2.4 Privacy enforcement before any live call
- [x] 2.5a Anthropic adapter and replay harness
- [x] 2.5b Anthropic recordings → tag `m2.1`
- [x] 2.6a Gemini adapter
- [x] 2.6b Gemini recordings
- [x] 2.7a OpenAI adapter
- [x] 2.7b OpenAI recordings
- [x] Follow-up from the recordings: models cited the `SF3` exemption in `allowed_by`; the rule text no longer asks them to cite anything, the rendered hint and the rejection name `held` as the disposition to use, and every vendor was re-recorded (2026-10-27)
- [ ] 2.8 Gateway profile candidates (optional)
- [x] 2.9 provider-data.md
- [x] 2.10a Packaging for the registry
- [x] 2.10b Binary distribution and live smoke (record: docs/reviews/m2-live-smoke.md)
- [x] Follow-up from the live smoke: the default budget planned an envelope above the default cap on Gemini 3.8 Flash and GPT-6.1 Terra; resolved by the envelope change above
- [x] Checkpoint M2 → merged to `main` and tagged `m2` on 2026-10-27 after the owner's go-ahead (docs/reviews/m2-checkpoint.md); the push is the owner's
- [x] From the M2 review (owner 2026-10-27: "do whatever you need to do"): the envelope plans each request once plus a per-run retry reserve of `retries ×` the largest bound, and a retry the balance cannot cover is refused with `error: budget`; the defaults now fit every shipped model and the examples are back on them
- [ ] 4.1 follow-up: a run whose tool images can carry the history past `many_image_threshold` fails preflight mid-run with `error: image_limit` after money is spent; at resolve, either prepare at `many_image_max_dimension_px` when the worst case can pass the threshold (and record it in `prep_version`) or refuse before the first call
- [ ] Recorded from the M2 review, not fixed: transport-error excerpts (up to 2 KB of a provider's error body) reach the journal and stderr without the redaction pass (the credential is never in a response body, so nothing has leaked; route them through the redaction list when the journal gains a redaction step); the `dist` installer in the `package` workflow is fetched by `curl | sh` without a checksum, as the generated release workflow does

## M3 Evaluation sets and baseline
- [x] 3.1 Set manifests, check, fetch, seal
- [x] 3.2a Set-building toolkit and ADR 0013 (toolkit, dry run and ADR done 2026-10-27; the public `dejaview-evalsets` repository is the owner's to create, with README and licence, before 3.2b publishes images)
- [x] 3.2b First dev set adjudicated and published (reference-consistency/pets-v1, 2026-10-27; images in markhayden/dejaview-evalsets, pushed by the owner)
- [x] 3.3a Source fidelity dev set (source-fidelity/scenes-v1, 2026-10-27)
- [x] 3.3b Reality checks dev set (reality-checks/people-v1, 2026-10-27)
- [x] 3.3c Brief compliance dev set (brief-compliance/stills-v1, 2026-10-27)
- [x] 3.3d Product fidelity dev set (product-fidelity/packshots-v1, 2026-10-27)
- [x] 3.3e Adversarial sets (five, one per use case) → tag `m3.1` (2026-10-27)
- [x] 3.4a Execution descriptor and durable evaluation ledger (2026-10-27)
- [x] 3.4b Matrix runner, arms and resume (2026-10-27)
- [x] 3.4c Pre-run estimate (2026-10-27)
- [x] 3.5a Metrics, localisation, report regeneration
- [x] 3.5b Gate lifecycle (stages, receipts, admissions)
- [x] 3.6 Static report viewer
- [x] 3.7 Canary and requalify
- [x] 3.8 Arm A baseline and publication (docs/results/baseline-2026-10, 2026-10-28: six models, 1,320 decisions, ledger $46.54 of $75 with $18.48 of it refused Anthropic calls charged at bound; the first run found the localisation scorer ignoring checklist-item violations, fixed and re-scored)
  - follow-ups from the baseline, owner decisions on the metric's definition, not blocking: `whole_image` labels against boxed predictions never match (label the defect's region, or let a whole-image label accept any box on that image); one defect reported under several obligations gives the same region several times and one-to-one matching counts the extras as spurious (count identical root geometries once with the union of their obligations); a call refused before processing (an error body, no usage) is charged at its bound in the ledger, which overstates spend after an outage or an empty balance; the estimate's expected-cost figure should lead with per-model observed medians (Anthropic ran at about five times the others per decision)
- [x] Checkpoint M3 → merged to `main` and tagged `m3` on 2026-10-28 (docs/reviews/m3-checkpoint.md); the push is the owner's
  - follow-ups recorded at the M3 review (2026-10-28), not blocking: a real SIGINT test of `eval run` through the binary; prompt equivalence across arms checked on the journaled system prompt; an automated devtools check of the viewer; a test that an environment-expanded matrix account is `[redacted]` in the report; the seal's image and rules components isolated in the mutation tests; one fixture per `sets check` failure mode; `JsonSchema` on `DecisionResult`, `Summary`, `Estimate` and `Matrix`; a concurrency cap on the viewer server; a harness-level test that instruction-like text in a label or brief never alters the prompt

## M4 Built-in tools and MCP client
- [x] 4.0 Alignment feasibility spike and ADR 0014 (2026-10-28: direct registration on normalised pyramids, within 0.05 px and 0.01° at 2048 px, sphere within 0.03°; `dejaview_tools::align`, failure fixtures under fixtures/images/align)
- [x] 4.1 Registry in production (2026-10-28: manifest `projections`, `ToolRegistry::with_projections` wrapping tools so envelopes are projected before the run sees them, the projection on the report's tool calls, the Tools section snapshot; `tools list` prints projections; built-in registration lands with the first built-in in 4.2a)
- [x] 4.2a `view`, `resize`, equirectangular geometry (2026-10-28: built-ins registered and listed; perspective views written from first principles, not ported from the prototype (owner decision 11), checked against the stored view and the analytic scene; view frames map to polygon footprints on the panorama, seam cut, pole as a full-width box)
- [x] 4.2b Viewer equirectangular overlay (2026-10-28: `geometry.js` mirrors `frames::sphere`; view crops and `view` locations drawn as footprints on the panorama; held to the Rust reference on stored footprints under node rather than by a rendered-pixel compare, which needs a browser)
- [x] 4.3a `align` flat and `visual_only` (2026-10-28; still to do from this line: run `eval gate` end to end through the binary now that arms C and D can run, and look at the gate tests' forty-second fake runs)
- [x] 4.3b `align` equirectangular (2026-10-28, with 4.3a: rotation on the sphere, warped panorama, the same envelope; aligned frames carry the rotation and map back through its inverse)
- [x] 4.3c `edge_similarity` and `change_map` → tag `m4.1` (2026-10-28: edge-map correlation and the locally normalised difference map, golden under fixtures/images/change-map)
- [x] 4.4a Full MCP client, ADR 0015, pagination, concurrency, images (2026-10-28: in-house client kept; multiplexed requests, pagination guards, concurrent tools and serial turns, lease waits for in-flight calls, outputSchema validation, exact blocks, isError truth table, frame chain, concurrent batch dispatch in the run)
- [x] 4.4b MCP failure modes (2026-10-28: one fake-server test per mode; leases keyed by decision directory; `shared = true` packs connected once per evaluation)
- [ ] 4.5 Arm C on the dev sets (2026-10-28: `evalsets/matrix/arm-c.toml`, arms A and C, five models (Opus left out: over the per-decision cap under arm C and over the standing $25 per vendor with it), one repetition, bootstrap seed frozen; the paired-flips section with the bootstrap interval is in the report; the planner priced each stored envelope's inline pixels as text and put arm C envelopes at $500 each, now it prices the presentation the model reads, and the estimate gives $1.47 to $5.15 per decision; the run follows)
- [x] 4.6 Segmenter artifact and MODEL.md (2026-10-28: `scripts/export_segmenter.py` exports EoMT-L ADE20K to ONNX, 1.26 GB, hash reproduced on a second run, parity within 6.5e-5 on class logits and 1.8e-3 on mask logits; MODEL.md complete; the artifact, goldens and record wait in tools/python/dist/segmenter for the owner to upload as the `segmenter-v1` release assets)
- [ ] Checkpoint M4 → tag `m4`

## M5 First ML tool
- [ ] 5.1 Python pack skeleton and MCP server
- [ ] 5.3 Segmenter fetch and parity
- [ ] 5.4a `regions` tool
- [ ] 5.4b `classes` tool
- [ ] 5.2 Provisioning bundle for the real pack (after the tools it packages) → tag `m5.1`
- [ ] 5.5 Sealed blind set for source fidelity
- [ ] 5.6 Initial gate D_regions versus C
- [ ] Checkpoint M5 → tag `m5`

## M6 Confirmation
- [ ] 6.1 Confirmation set (50+, two batches of 25)
- [ ] 6.2 Confirmation gate and admission record
- [ ] Checkpoint M6 → tag `m6`
