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
- [ ] 3.1 Set manifests, check, fetch, seal
- [ ] 3.2a Set-building toolkit and ADR 0013
- [ ] 3.2b First dev set adjudicated and published
- [ ] 3.3a Source fidelity dev set
- [ ] 3.3b Reality checks dev set
- [ ] 3.3c Brief compliance dev set
- [ ] 3.3d Product fidelity dev set
- [ ] 3.3e Adversarial sets (five, one per use case) → tag `m3.1`
- [ ] 3.4a Execution descriptor and durable evaluation ledger
- [ ] 3.4b Matrix runner, arms and resume
- [ ] 3.4c Pre-run estimate
- [ ] 3.5a Metrics, localisation, report regeneration
- [ ] 3.5b Gate lifecycle (stages, receipts, admissions)
- [ ] 3.6 Static report viewer
- [ ] 3.7 Canary and requalify
- [ ] 3.8 Arm A baseline and publication
- [ ] Checkpoint M3 → tag `m3`

## M4 Built-in tools and MCP client
- [ ] 4.0 Alignment feasibility spike and ADR 0014
- [ ] 4.1 Registry in production
- [ ] 4.2a `view`, `resize`, equirectangular geometry
- [ ] 4.2b Viewer equirectangular overlay
- [ ] 4.3a `align` flat and `visual_only`
- [ ] 4.3b `align` equirectangular
- [ ] 4.3c `edge_similarity` and `change_map` → tag `m4.1`
- [ ] 4.4a Full MCP client, ADR 0015, pagination, concurrency, images
- [ ] 4.4b MCP failure modes
- [ ] 4.5 Arm C on the dev sets
- [ ] 4.6 Segmenter artifact and MODEL.md
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
