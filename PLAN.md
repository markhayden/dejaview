# Implementation plan: dejaview harness

Status: draft 4, 2026-10-26, written from `SPEC.md` draft 5 (approved) and revised against Codex
plan reviews 1 to 3 (`docs/reviews/plan-review-*.md`). Review 3 judged draft 3 ready with
changes and split them into before-M0 (all made here) and fix-in-the-owning-task items (noted in
the tasks that own them). The task checklist that tracks progress
is `tasks/todo.md`; this file holds the reasoning, the order and the commit strategy. When the
spec changes, this plan changes in the same commit.

## Overview

Build a Rust binary that runs a vision model as a judge over an image job, with deterministic
tools as evidence and the verdict computed in code, then prove each tool earns its place on
public labelled sets. The order follows the brief's method: a skeleton that runs end to end
offline, real provider adapters with recorded fixtures, public evaluation sets and a tool-less
baseline, the built-in zoom and alignment tools as the equal-visual-access control, then the first
ML tool scored by a predeclared gate on a sealed set, and a confirmation stage before any tool is
called admitted.

| Milestone | Delivers | Tag |
|---|---|---|
| M0 Scaffold | workspace, tool bootstrap, CI, lint, docs skeleton, README sections, owner readiness | `m0` |
| M1 Skeleton | `dejaview check` end to end against a scripted fake provider, every state, offline; minimal MCP client, minimal provisioning proof, frozen statistics | `m1` |
| M2 Adapters | privacy enforcement; Anthropic, Gemini, OpenAI adapters with fixtures; catalog; image prep; live smoke; packaging and distribution | `m2` |
| M3 Sets and baseline | set tooling, five public dev sets plus five adversarial sets, eval runner with execution descriptor and durable ledger, metrics, gate lifecycle, viewer, arm A baseline | `m3` |
| M4 Built-in tools | alignment spike, Tool registry, `view`/`align`/`resize`/`edge_similarity`/`change_map`, full MCP client, arm C, MODEL.md | `m4` |
| M5 First ML tool | Python pack, provisioning bundle, `regions` and `classes`, sealed blind set, initial gate | `m5` |
| M6 Confirmation | a distinct sealed set of at least 50 items, confirmation gate, admission record | `m6` |

Deferred past M6 with their own ADRs: server mode, Bedrock and Vertex profiles, the ChatGPT-plan
OAuth scheme, `dejaview tools serve`, scheduled canaries in CI, Vercel and Cloudflare profiles if
their fixtures do not pass during M2.

## Crate dependency graph

```
dejaview-core        no dependency on other workspace crates; owns job, images, frames, obligations,
                     checklist, prompt, state machine, submission, aggregation, decision, journal,
                     report, ledger, cache, and the Provider and Tool traits with their request,
                     response, manifest and envelope types
dejaview-providers   depends on core; adapters, wire schemas, profiles, catalog, auth, fixtures player
dejaview-tools       depends on core; built-ins, MCP client, provisioning, projections
dejaview-eval        depends on core, providers, tools; sets, seal, matrix, descriptor, eval ledger,
                     metrics, bootstrap, gates
dejaview-testkit     depends on core; scripted fake provider, fake tool, fake MCP server binary
dejaview (package)   directory crates/dejaview-cli; package name and binary name are both `dejaview`;
                     every selector in this plan is `-p dejaview`; depends on core, providers, tools,
                     eval; optional dependency on testkit behind the non-default feature `testkit`
```

The `testkit` feature is the one declared production edge from the CLI to testkit, because an
optional dependency is still a dependency for publication: `dejaview-testkit` is a published
package, and the publication order is `dejaview-core`, `dejaview-testkit`, `dejaview-providers`,
`dejaview-tools`, `dejaview-eval`, `dejaview`. Ordinary and release builds do not enable the
feature and reject `--provider-script` as an unknown flag; that rejection is tested against a
separately built feature-off executable, since a feature-on binary cannot prove it. Dev-dependency
edges (every crate's tests may use testkit) are declared separately from production edges, and the
crate-graph check runs with all features enabled. `just check` runs both feature configurations:
`cargo test --workspace` and `cargo test --workspace --features dejaview/testkit`, plus Clippy in
both; every filtered test invocation in this plan must enumerate and execute at least one test
(`--list` compared in the `just` recipe). Availability of all six package names on crates.io is
confirmed in 0.1 and a fallback recorded by ADR if needed.

## Task dependency edges

Each task's **Dependencies** line is the authority; this list is generated from those lines and
must match them. Milestone barriers apply on top: no task in M(n+1) starts before `m(n)` is
tagged.

```
M0  0.1 ← none        0.2 ← 0.1        0.3 ← none        0.4 ← none
M1  1.1 ← 0.1         1.2 ← 1.1        1.3 ← 1.1         1.4a ← 1.3
    1.4b ← 1.4a, 1.2  1.5a ← 1.1       1.5b ← 1.2, 1.4b, 1.5a
    1.6a ← 1.5b       1.6b ← 1.6a      1.7a ← 1.6b       1.7b ← 1.7a
    1.8 ← 1.7b        1.9 ← 1.8        1.10a ← 1.9       1.10b ← 1.10a
    1.11 ← 1.10b      1.12 ← 1.11      1.13 ← 1.12, 1.5a 1.14 ← 1.13      1.15 ← 0.1
M2  2.1 ← 1.3, 0.4    2.2 ← 1.6a, 2.1  2.3 ← 1.2, 2.1    2.4 ← 1.3, 1.10a
    2.5a ← 2.2, 2.3, 2.4, 1.8           2.5b ← 2.5a
    2.6a ← 2.5a       2.6b ← 2.6a      2.7a ← 2.5a       2.7b ← 2.7a
    2.8 ← 2.5b, 2.6b (optional)         2.9 ← 2.5b, 2.6b, 2.7b
    2.10a ← 2.5b, 2.6b, 2.7b            2.10b ← 2.10a
M3  3.1 ← 1.4b        3.2a ← 3.1, 0.4  3.2b ← 3.2a       3.3a..d ← 3.2b    3.3e ← 3.2b
    3.4a ← 1.9, 1.11, 3.1               3.4b ← 3.4a       3.4c ← 3.4b
    3.5a ← 3.4b, 1.15 3.5b ← 3.5a, 3.1 3.6 ← 1.10b       3.7 ← 3.4b
    3.8 ← 2.5b, 2.6b, 2.7b, 3.3a, 3.3b, 3.3c, 3.3d, 3.3e, 3.4c, 3.5a
M4  4.0 ← 1.2, 0.4    4.1 ← 1.5a, 1.7a 4.2a ← 4.1, 1.2   4.2b ← 4.2a, 3.6
    4.3a ← 4.0, 4.1   4.3b ← 4.3a, 4.2a                  4.3c ← 4.3a
    4.4a ← 1.13, 4.1  4.4b ← 4.4a      4.5 ← 4.2a, 4.3b, 4.3c, 3.8        4.6 ← 0.4
M5  5.1 ← 4.4b, 1.14  5.3 ← 4.6, 5.1   5.4a ← 5.3        5.4b ← 5.3
    5.2 ← 5.3, 5.4a, 5.4b               5.5 ← 3.2a, 3.1   5.6 ← 5.4a, 5.5, 4.5, 3.5b, 5.2
M6  6.1 ← 5.6, 0.4    6.2 ← 6.1
```

Every M1 task names a CLI scenario that demonstrates its behaviour through the binary using only
what precedes it.

## Commit strategy

**Initial commit.** Before M0, one documentation commit on `main` with `SPEC.md`, `PLAN.md`,
`tasks/todo.md`, `docs/adr/`, `docs/reviews/` and `.gitignore`; `docs/brief.md` stays excluded
and `git check-ignore docs/brief.md` is the test.

**Branches.** `main` holds only reviewed checkpoints. Each milestone is one branch `mN-<name>`
from the previous checkpoint tag. Tasks are committed on the milestone branch, or on short task
branches `mN/<task>` merged into the milestone branch when two tasks are in flight. Code work on
a later milestone never starts before the earlier milestone merges; owner labelling happens
outside git and may overlap. A milestone merges into `main` with a merge commit (no squash) after
the owner's review and the checkpoint checklist passes. Nothing is pushed by the agent; the owner
pushes. The owner's preference (2026-10-26): branches stay on the owner's machine and `main` is
pushed only at tagged checkpoints, so the public repository shows reviewed, working states; CI is
run locally with `just ci-local` before any push.

**Commits.** Conventional Commits with a scope. Every commit on a milestone branch leaves
`just check` green, which runs `cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings` in both feature configurations, `cargo test --workspace`,
`cargo test --workspace --features dejaview/testkit` and `cargo deny check`; a commit that cannot
is not made. `Cargo.lock` is committed. The
spec, plan, README and docs are updated in the same commit as the change they describe.
Attribution lines follow the repository's conventions. Secrets, private images and
`docs/brief.md` never enter a commit.

**Checkpoints and tags.** Every later checkpoint also requires `just ci-local` green. When a
milestone's checkpoint checklist passes and the owner approves,
the merge commit on `main` is tagged `mN` (annotated, with the checklist and the versions of
external artifacts: evalsets repository commit, model artifact hashes, published package
versions). Intermediate tags `mN.k` mark task groups on the milestone branch.

**Rollback.**

1. Revert one task with `git revert`; when later tasks depend on it, revert the dependents together
   in reverse dependency order and verify the suite before committing the revert.
2. Reset a milestone branch to an intermediate tag only with a clean tree and after
   `git branch rescue/<branch>-<date>`; never on `main`.
3. Abandon an unmerged milestone by deleting the branch after the rescue reference exists.
4. Undo a merged milestone with a reviewed `git revert -m 1 <merge commit>` on `main`; never a
   force push.

External state is not rolled back by git: published set images, released packages and model
artifacts are versioned and recorded in checkpoint tags; a blind set whose labels were exposed is
never reused; spent money is recorded in results documents. `eval-out/` and run directories are
never committed; results live under `docs/results/`.

**Reviews.** This plan is reviewed by Codex before M0 begins. Each milestone's diff is reviewed
with the review and test skills (`/agent-skills:review`, `/agent-skills:test`) available in the
owner's environment; where they are not available, `docs/review-checklist.md` (written in 0.3)
is applied by hand. The owner reviews every checkpoint.

## Milestone M0: Scaffold

### Task 0.1: Workspace, crate skeletons and tool bootstrap

**Description.** The workspace with the six crates from the crate graph, each with a `lib.rs`
or `main.rs`, a crate-level `thiserror` enum and one trivial test; `rust-toolchain.toml` pinning
stable with the `clippy` and `rustfmt` components; `docs/bootstrap.md` giving the one manual step
(`cargo install just --version <pinned>`), then `just bootstrap` installing pinned `cargo-deny`
and `cargo-dist` (and `uv` if absent) and `just check` running fmt, Clippy in both feature
configurations, `cargo test --workspace`, `cargo test --workspace --features dejaview/testkit`,
deny, the crate-graph check with all features, and a `--list` check that every named filtered
suite executes at least one test; `deny.toml` with the allowed-licence list; the `testkit` feature
wiring; availability of all six package names on crates.io checked and recorded (ADR 0012 if a
fallback is needed).

**Acceptance.**
- [ ] After the documented `just` install, `just bootstrap && just check` passes on a clean clone.
- [ ] The crate-graph check distinguishes production and dev edges and fails CI on any edge
      outside the graph.
- [ ] `cargo run -p dejaview -- --version` prints version and short commit; a separately built
      feature-off executable rejects `--provider-script`; the feature-on build lists it.

**Verification.** `cargo install just --version <pinned> && just bootstrap && just check`.

**Dependencies.** None. **Files.** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `deny.toml`,
`justfile`, `scripts/check-crate-graph.sh`, `crates/*/`. **Size.** M.

### Task 0.2: CI, lint and pre-commit

**Description.** GitHub Actions running `just check`, the crate-graph check, and the Python checks
once `tools/python` exists. A pre-commit hook installed by `just hooks` that runs fmt and clippy
and refuses staged content matching credential-shaped patterns (full-shape regexes such as
`sk-ant-[A-Za-z0-9_-]{24,}`, `AIza[0-9A-Za-z_-]{35}`, `sk-proj-[A-Za-z0-9_-]{24,}`,
`AKIA[0-9A-Z]{16}`, `ghp_[A-Za-z0-9]{36}`). No file is exempt; instead `scripts/synthetic-tokens.txt`
enumerates the exact synthetic token values the fixtures and the hook test use, and only those
values are allowed anywhere. `.gitignore` for `target/`, `eval-out/`, `dejaview-out/`,
`*.local.toml`, `.venv/`.

**Acceptance.**
- [ ] CI is green on the M0 branch.
- [ ] Hook test: accepts the hook's own source, the documentation examples and the enumerated
      synthetic tokens; rejects a realistic-shaped credential in an ordinary staged file and the
      same token inside the hook test or the fixtures directory; accepts a clean commit.
- [ ] `git check-ignore docs/brief.md` exits 0 and `git ls-files docs/brief.md` prints nothing.

**Verification.** `just hooks && scripts/test-hooks.sh`; draft PR with CI.

**Dependencies.** 0.1. **Files.** `.github/workflows/ci.yml`, `scripts/hooks/pre-commit`,
`scripts/scan-secrets.sh`, `scripts/test-hooks.sh`, `scripts/test-checks.sh`,
`fixtures/synthetic-secrets/`, `.gitignore`. **Size.** S.

### Task 0.3: Docs skeleton, review checklist and README sections

**Description.** `docs/architecture.md` from the spec; stubs for `docs/tools.md`, `docs/rules.md`,
`docs/evalsets.md`, `docs/viewer.md`, `docs/provider-data.md`, `docs/results/README.md`, each
naming its filling task; `docs/review-checklist.md` (the five-axis review and the test-coverage
checks, for use when the skills are unavailable); README "How it works" and "How it is built"
from the spec with links to SPEC, PLAN, ADRs and reviews.

**Acceptance.**
- [ ] README reads correctly for an empty harness and says nothing is measured yet.
- [ ] Every stub names its filling task; the review checklist exists.

**Dependencies.** None. **Files.** `README.md`, `docs/*.md`. **Size.** S.

### Task 0.4: Owner readiness

**Description.** `docs/owner-decisions.md` recording, before any money is spent: the initial
judge-model subset (recommendation: one model per vendor at first); which Anthropic account
(enforced-thinking or not) records fixtures; spend caps and approval points for fixture recording,
release and gateway smoke tests, image generation, the baseline, arm C, the initial gate and the
confirmation gate; the labelling plan (worksheet format, batch size, adjudicator); which prototype
images, outputs and artifacts are approved for reuse; whether M6 is funded in this release.
Generators and licence are deferred to 3.2a. Blank entries name the task they block.

**Acceptance.**
- [ ] Every entry filled or marked "decide before task X"; each paid task in this plan cites its
      entry.

**Dependencies.** None. **Files.** `docs/owner-decisions.md`. **Size.** S; owner time.

### Checkpoint M0
- [ ] `just check` green locally and on a fresh clone; `just ci-local` (the GitHub workflow run in
      Docker with act) green; hooks installed; README accurate; owner decisions recorded.
- [ ] Owner review; merge; tag `m0`.

## Milestone M1: Skeleton

### Task 1.1: Walking slice

**Description.** The thinnest valid end-to-end path: a job with a candidate and a source by path,
decoded without bounds checks (1.2 adds them); one hard-coded mandatory obligation `SF2` with
scope `candidate` resolving to one instance; a one-item checklist; a minimal system prompt; the
`Provider` trait exactly as the spec writes it with a scripted fake in `dejaview-testkit`
returning a `submit_verdict` whose single item is `held` or `violation`; schema-light validation;
aggregation of that one row; a journal with `resolved`, `turn_sent`, `turn_received`,
`submission`, `verdict`; a minimal `report.json`; exit codes 0 and 1; the `testkit` feature and
`--provider-script` flag.

**CLI scenario.** `cargo run -p dejaview --features testkit -- check --config fixtures/config/minimal.toml --job fixtures/jobs/pair.toml --provider-script fixtures/scripts/held.json` exits 0; with `violation.json` exits 1; both write a journal and report; a build without the feature rejects the flag.

**Acceptance.**
- [ ] Both scenarios and the feature-off rejection pass in tests; the report shows the instance,
      disposition and row; the journal is append-only JSONL and the report is derived from it.

**Verification.** `cargo test -p dejaview --features testkit --test walking_slice`.

**Dependencies.** 0.1. **Files.** `crates/dejaview-core/src/{job,provider,run,journal,report}.rs`,
`crates/dejaview-testkit/src/provider.rs`, `crates/dejaview-cli/src/main.rs`, fixtures.
**Size.** M. Tag `m1.1`.

### Task 1.2: Canonical images and frames registry

**Description.** Decode bounds, EXIF orientation, sRGB 8-bit, original and canonical SHA-256,
projection from field or GPano with the 2:1 warning, base64 sources; the frames registry with
`raw`, `aligned`, `crop` types, the location record and validity table, flat mapping to the root
with clipping and the `partial` flag (equirectangular maths in 4.2a).

**CLI scenario.** `check` on a job with an EXIF-rotated candidate reports upright dimensions and
hashes equal to `shasum -a 256`; a 60 MP image is refused with `error: resolve` naming the bound.

**Acceptance.**
- [ ] Oversized images refused before full decode; EXIF applied; hashes match; validity table
      enforced by a table test; crop-to-raw round trip within 1e-6; wholly outside is an error and
      partly outside is flagged.

**Verification.** `cargo test -p dejaview-core -- image frames`.

**Dependencies.** 1.1. **Files.** `crates/dejaview-core/src/{image,frames}.rs`, `fixtures/images/`.
**Size.** M.

### Task 1.3: Config loading, validation and the shorthand job

**Description.** The full `dejaview.toml` schema, `schemas/config-v1.json`, `${VAR}` from the
environment only, CLI overrides as repeatable `--set path=value`, credential-name refusal in pack
`env`, the `cloud_calls` gate,
redacted resolved config in the report, a built-in `fake` profile for tests, and the shorthand
`check --candidate … --ref name=path … --brief …` producing the same resolved job as the
equivalent `--job` file.

**CLI scenario.** `check --config full.toml --set budget.tool_turns=2` shows the override; the
same with `--set privacy.cloud_calls=false` refuses; the shorthand and `--job` forms produce
byte-identical resolved-job sections in their reports.

**Acceptance.**
- [ ] Every key in the spec example parses; unknown keys fail; the schema file regenerated by a
      test equals the committed one; a secret in the environment appears in no output path;
      shorthand equivalence fixture passes.

**Verification.** `cargo test -p dejaview-core -- config` and `cargo test -p dejaview --features testkit --test shorthand`.

**Dependencies.** 1.1. **Files.** `crates/dejaview-core/src/config.rs`, `crates/dejaview-cli/src/args.rs`,
`schemas/config-v1.json`. **Size.** M.

### Task 1.4a: Obligation model, merge and validation

**Description.** Obligation types and fields, rules files with `extends` and profiles, consumer
files, job requirements, merge order, every validation rule in the spec except those needing the
tool registry (evidence-kind existence is checked in 1.5b).

**CLI scenario.** `dejaview rules show --config …` prints the merged list with provenance; a
consumer file with a bad `waives` id fails naming the rule.

**Acceptance.**
- [ ] Table test with one failing fixture per validation rule; the spec's generic and
      architectural files merge to the documented result.

**Verification.** `cargo test -p dejaview-core -- obligations`.

**Dependencies.** 1.3. **Files.** `crates/dejaview-core/src/obligations.rs`,
`crates/dejaview-cli/src/rules.rs`. **Size.** M.

### Task 1.4b: Instances, rendering and shipped rule sets

**Description.** Scope and evidence resolution to targets, instances, the empty-expansion,
no-mandatory-instance and unconsulted-reference errors, the Obligations section rendering, the
five base rule sets and the architectural profile embedded, and a rendering fixture for the
architectural profile covering `waives = []`, SF4 reporting and the `allowed_change` assertions.

**CLI scenario.** `check --dry-run` on the Pip job prints the rendered Obligations section and
the instance list.

**Acceptance.**
- [ ] Each error case tested; rendering snapshots committed and byte-stable; all six shipped
      files load and validate.

**Verification.** `cargo test -p dejaview-core -- instances render`.

**Dependencies.** 1.4a, 1.2 (the Pip job uses a base64 source). **Files.**
`crates/dejaview-core/src/{instances,render}.rs`, `rules/`. **Size.** M.

### Task 1.5a: Manifest and envelope types

**Description.** The `Tool` trait, manifest type, model schema versus input schema, `_dejaview`
type, result envelope with `returns` bounds, evidence kinds, projection type, and a scripted fake
tool in `dejaview-testkit`; `dejaview tools list`.

**CLI scenario.** `dejaview tools list --config …` prints the fake tool's manifest as the model
will see it.

**Acceptance.**
- [ ] Manifest missing a required field refused; envelope over any bound is `error: tool`; model
      argument containing `_dejaview` is `invalid_args`; `schemas/{manifest,envelope}-v1.json`
      regenerated equal committed.

**Verification.** `cargo test -p dejaview-core -- manifest envelope`.

**Dependencies.** 1.1. **Files.** `crates/dejaview-core/src/tool/{manifest,envelope}.rs`,
`crates/dejaview-testkit/src/tool.rs`, `crates/dejaview-cli/src/tools.rs`. **Size.** M.

### Task 1.5b: Checklist and evidence binding

**Description.** Static items, evidence items by `evidence_kind` and root image, the
non-empty-dependency invariant, drops journaled, the evidence budget error, call-order ordering,
overlap cross-references, and the evidence-kind validation deferred from 1.4a. Exercised through
unit tests and `--dry-run` only; the tool-driven CLI scenario belongs to 1.7a.

**CLI scenario.** `check --dry-run` on the Pip job prints the static checklist items and their
dependencies.

**Acceptance.**
- [ ] Pip example yields the documented items; unit tests: three regions returned, one dropped
      for empty dependencies, two presented as items; the 25th region is `error: budget`; overlap
      cross-reference rendered.

**Verification.** `cargo test -p dejaview-core -- checklist`.

**Dependencies.** 1.2, 1.4b, 1.5a. **Files.** `crates/dejaview-core/src/checklist.rs`. **Size.** M.

### Task 1.6a: Submission schema and semantic validation

**Description.** Canonical submission types, `schemas/submission-v1.json`, `jsonschema`
validation, every semantic rule with a failing fixture each. Unit-level; the repair-driven CLI
scenario belongs to 1.8.

**CLI scenario.** `check` with a script whose submission is valid passes; a script whose
submission omits an item exits 3 with `error: submission_invalid` (repair not yet wired, so the
first failure is terminal until 1.8).

**Acceptance.**
- [ ] One failing fixture per semantic rule (unknown item, missing item, violation without
      location, inapplicable exemption, `observed` citing a mandatory id, evidence from a failed
      call, wrong location kind for the projection, mapping wholly outside).

**Verification.** `cargo test -p dejaview-core -- submission validate`.

**Dependencies.** 1.5b. **Files.** `crates/dejaview-core/src/{submission,validate}.rs`,
`schemas/submission-v1.json`. **Size.** M.

### Task 1.6b: Aggregation and decision tables

**Description.** Per-instance rows, strictest-row combination, verdict, decision table with
evidence matching, `[policy]` on the exit code only.

**CLI scenario.** `check` with `decision_runs = 2` and two scripts that disagree on a static
item exits 2 with both runs in the report.

**Acceptance.**
- [ ] Exhaustive test over disposition × confidence × (`matters`, `hidden_ok`) matches the spec
      table row by row; all 16 decision pairs tested; matching eligibility, ordering and the
      `held` exception tested.

**Verification.** `cargo test -p dejaview-core -- aggregate decision`.

**Dependencies.** 1.6a. **Files.** `crates/dejaview-core/src/{aggregate,decision}.rs`. **Size.** M.

### Task 1.7a: State machine: tool turns

**Description.** Resolve, prepare (stub sizing), reserve (stub ledger), tool turns with batch
validation before execution, call budget exhaustion with error results, serial execution (concurrent dispatch for `concurrent = true` packs lands in 4.4a)
execution, status handling, the versioned prompt template in `prompts/v1.md` with the Tools
section from manifests.

**CLI scenario.** `check` with a script of two tool turns (fake tool returning three regions,
one dropped) then a submission shows both turns, all calls and two evidence items in the report.

**Acceptance.**
- [ ] Scenarios: first look then submit; two tool turns; batch over budget answered with error
      results; fake tool crash is `error: tool`.
- [ ] Provider-history invariant test: accepted blocks, opaque payloads, system prompt and tool
      definitions unchanged across requests; new content appended in provider-valid order;
      request-level controls change only as the catalog permits; tool definitions identical on
      every request.

**Verification.** `cargo test -p dejaview-core -- run::tool_turns`.

**Dependencies.** 1.6b. **Files.** `crates/dejaview-core/src/{run,prompt}.rs`, `prompts/v1.md`.
**Size.** M.

### Task 1.7b: State machine: final turn and terminal states

**Description.** The `refuse` final-turn mechanism, stray-call error results, `no_submission`,
every `error:` class reachable, operational error irreversibility. Repair is a single terminal
failure here; 1.8 adds the retry of each class.

**CLI scenario.** A script that keeps calling tools exhausts `tool_turns`, receives the final
instruction, calls once more, gets the error result, and ends `error: no_submission`; a script
whose tool crashes and then submits validly still ends `error: tool`.

**Acceptance.**
- [ ] Both scenarios tested; every error class implemented by this task (`resolve`, `tool`,
      `provider`, `budget` stub, `no_submission`, `submission_invalid` as single failure) has a
      scripted path; journal order asserted. Classes added by later tasks are tested there.

**Verification.** `cargo test -p dejaview-core -- run::final_turn run::errors`.

**Dependencies.** 1.7a. **Files.** `crates/dejaview-core/src/run.rs`. **Size.** M. Tag `m1.2`.

### Task 1.8: Repair protocol and retries

**Description.** Both repair forms, `turn_excluded`, reissue policies per response class,
`repair_output_tokens`, the repair budget, `error: model_output` and `error: submission_invalid`
on the second failure, transport retries with backoff, jitter, `retry-after`, counted as billable
attempts.

**CLI scenario.** A script with a truncated first response then a valid submission exits 0 with
one excluded turn and one reissue in the report; a script with an invalid submission then a valid
one exits 0 with one repair.

**Acceptance.**
- [ ] Each repair case reaches the documented outcome once and the documented error twice;
      reissue assertions by class (truncation changes only the output allowance; refusal and
      call-less text append the discarded-output instruction; excluded responses never enter
      provider history); retries on a mocked clock (429 with `retry-after: 2` waits 2 s; stops
      after `retries`; 400 fails immediately).

**Verification.** `cargo test -p dejaview-core -- repair retry`.

**Dependencies.** 1.7b. **Files.** `crates/dejaview-core/src/{repair,retry}.rs`. **Size.** M.

### Task 1.9: Cost ledger

**Description.** Decision envelope from a stub catalog entry including the reissue allowance,
attempt holds transferred from the envelope, settlement, charge at bound on unknown usage,
release, the per-job ledger, a `Ledger` trait the evaluation ledger implements in 3.4a, refusal
when a bound cannot be computed.

**CLI scenario.** `check` with `spend_cap_usd = 0.01` exits 3 with `error: budget` before any
request; the report shows the envelope.

**Acceptance.**
- [ ] Table test: total reserved never exceeds the envelope and never increases; settlement
      returns the difference; unknown usage charges the bound; release at the end; two concurrent
      decisions against one shared ledger cannot overshoot (barrier test).

**Verification.** `cargo test -p dejaview-core -- ledger`.

**Dependencies.** 1.8. **Files.** `crates/dejaview-core/src/ledger.rs`. **Size.** M.

### Task 1.10a: Journal and artifact durability

**Description.** Artifacts fsynced before their events, content-addressed store, redaction, 2 MB
body truncation with full-body hash, SIGINT and SIGTERM to `cancelled`, file mode 0600.

**CLI scenario.** Kill a running `check` with SIGTERM; the journal ends in `cancelled` with the
state reached.

**Acceptance.**
- [ ] SIGINT and SIGTERM tested; truncation with correct hash; no secret in any artifact (grep
      test); every referenced artifact exists before its event (crash test with an injected fault
      between write and append).

**Verification.** `cargo test -p dejaview-core -- journal artifacts`.

**Dependencies.** 1.9. **Files.** `crates/dejaview-core/src/{journal,artifacts}.rs`. **Size.** M.

### Task 1.10b: Report, schema, rebuild and prune

**Description.** `report.json` with every spec field, `schemas/report-v1.json`, a test that every
boundary type derives the three traits with `deny_unknown_fields` and a version field,
`dejaview report rebuild` from a partial journal, `dejaview report prune`.

**CLI scenario.** `report rebuild` on the cancelled run from 1.10a yields terminal status
`cancelled`; `report prune --older-than 0s` removes it.

**Acceptance.**
- [ ] Rebuild from each truncation point of a journal yields a report with the right terminal
      status; boundary-type test passes; prune respects age.

**Verification.** `cargo test -p dejaview-core -- report` and `cargo test -p dejaview --features testkit --test report_cli`.

**Dependencies.** 1.10a. **Files.** `crates/dejaview-core/src/report.rs`, `schemas/report-v1.json`,
`crates/dejaview-cli/src/report.rs`. **Size.** S.

### Task 1.11: Cache at the decision boundary

**Description.** Cache key over the spec's list after prepare, decision-boundary semantics,
pruned-hit miss, evaluation mode never reads.

**CLI scenario.** The same `check` twice shows `cache = "hit"` the second time; with
`decision_runs = 2` the second run inside the first decision does not hit.

**Acceptance.**
- [ ] Table test flips each key input and each excluded input; cache spy for decision-boundary
      semantics; pruned artifacts make a miss.

**Verification.** `cargo test -p dejaview-core -- cache`.

**Dependencies.** 1.10b. **Files.** `crates/dejaview-core/src/cache.rs`. **Size.** S.

### Task 1.12: Isolation, environment filter and exit codes

**Description.** The pure environment-allowlist function, adversarial fixtures (instruction text
in pixels, filenames, labels, brief, tool results), registry-only id resolution, `_dejaview` from
the model rejected, the exit code matrix including 64.

**CLI scenario.** `check` on the adversarial job produces a prompt whose sections and tool list
equal the clean job's; a fake tool call with `image = "../../etc/passwd"` is rejected before any
filesystem access.

**Acceptance.**
- [ ] Prompt structure comparison per fixture; path-traversal test; environment filter table test
      (declared credential names still removed); exit codes 0, 1, 2, 3, 64 each produced.

**Verification.** `cargo test -p dejaview --features testkit --test isolation --test exit_codes`.

**Dependencies.** 1.11. **Files.** `crates/dejaview-core/src/env.rs`, `crates/dejaview-cli/tests/`,
`fixtures/adversarial/`. **Size.** M.

### Task 1.13: Minimal MCP client against a fake server

**Description.** An MCP 2025-11-25 stdio client proving the lifecycle contract: spawn with the
environment filter; `initialize` with exact version and `tools` capability check;
`notifications/initialized`; single-page `tools/list`; `tools/call` with `_dejaview`; per-call
deadline; `notifications/cancelled` with tombstones; the exclusive lease whose single
`startup_timeout_s` deadline covers waiting for quiescence, `dejaview/reset` and its
acknowledgement, or shutdown and reinitialisation; no reuse after a cancelled call; shutdown
escalation. Pagination, concurrency, image blocks, `isError` precedence and the `rmcp` decision
are 4.4a/b.

**CLI scenario.** `check` with `[tools.packs.fake] command = "<path>"` lists and calls the fake
tool through MCP, where `<path>` is the executable built from the testkit target declared as
`[[bin]] name = "dejaview-testkit-mcp", path = "src/bin/mcp.rs"`; a test helper builds that
target and reads its absolute path from Cargo's JSON output, so no test depends on `PATH` or a
previous build.

**Acceptance.** Fake-server fixtures for: wrong version refused; missing `tools` capability
refused; successful reset after which a probe shows no prior-run state survives; a server without
reset support is restarted per run; timeout cancels, tombstones, marks the process unreusable and
the next lease replaces it; the total acquisition deadline is honoured when reset hangs (one
clock); shutdown escalates on a server ignoring stdin close.

**Verification.** `cargo test -p dejaview-tools -- mcp::minimal`.

**Dependencies.** 1.12, 1.5a. **Files.** `crates/dejaview-tools/src/mcp/`,
`crates/dejaview-testkit/src/bin/mcp.rs`. **Size.** M.

### Task 1.14: Minimal provisioning bundle proof

**Description.** A tiny `tools/python/` uv project with one MCP tool (`echo_region`) and the
provisioning layout from the spec; `dejaview tools setup` (verify digests, unpack,
`uv sync --frozen --no-dev` with the declared Python); offline launch with `UV_OFFLINE=1` and
`UV_PYTHON_DOWNLOADS=never`. Two separate verifications: provisioning, which uses the network and
runs only under `DEJAVIEW_LIVE=1` or against a pre-built local bundle and a pre-seeded uv cache;
and runtime network denial, proven in the Linux CI job by launching the pack inside
`unshare -rn` (no network namespace) and asserting a successful call, with the pack-side
socket-failing test mode kept as a development aid, not proof.

**CLI scenario.** `dejaview tools setup --bundle tools/python/dist/bundle-0.0.1/` into an empty
`XDG_DATA_HOME`, then `check` with the pack enabled calls `echo_region` over MCP.

**Acceptance.**
- [ ] Clean-directory install yields a working executable whose installed versions match the
      lock; tampered digest fails; the `unshare -rn` launch succeeds in CI.

**Verification.** `cargo test -p dejaview-tools --test provisioning -- --ignored` (pre-seeded
bundle in CI); `cd tools/python && uv run --frozen --offline --no-sync pytest`.

**Dependencies.** 1.13. **Files.** `tools/python/`, `scripts/build-bundle.sh`,
`crates/dejaview-tools/src/provision.rs`, `.github/workflows/python.yml`. **Size.** M.

### Task 1.15: Statistics contract frozen

**Description.** In `dejaview-eval`: the paired bootstrap with a pinned RNG (ChaCha20 via
`rand_chacha`, dependency by ADR 0016, seed as a 64-bit unsigned decimal in the matrix), canonical
ordering (families sorted by id, items sorted by id within family), integer sampling by
`rand::Rng::gen_range` over the family index, a stored resample-index fixture for a small case
with unequal family sizes, type-7 quantiles, the detection-indicator family predicate, the
Bonferroni-adjusted endpoints and insufficient-tail rule, Wilson intervals, each against
hand-computed fixtures.

**CLI scenario.** `dejaview eval gate --fixture fixtures/stats/case-1.toml` prints the interval
and outcome for a synthetic result table.

**Acceptance.**
- [ ] The stored resample indices and interval are reproduced exactly from the stored seed; four
      differing families yield `inconclusive`; a tail with fewer than 20 resamples yields
      `inconclusive`; Wilson matches a reference table to 1e-9.

**Verification.** `cargo test -p dejaview-eval -- stats`.

**Dependencies.** 0.1. **Files.** `crates/dejaview-eval/src/{bootstrap,wilson,gate_rules}.rs`,
`fixtures/stats/`, `docs/adr/0016-rng.md`. **Size.** M.

### Checkpoint M1
- [ ] `cargo test --workspace` under five minutes, offline; transition and row coverage report in
      the PR.
- [ ] `docs/architecture.md`, `docs/rules.md`, `docs/tools.md` filled for what exists.
- [ ] Review and test skills (or the checklist) applied; findings fixed or recorded.
- [ ] Owner review; merge; tag `m1`.

## Milestone M2: Provider adapters

### Task 2.1: Model catalog and capability validation

**Description.** `providers/catalog.toml` with every spec field; values read from provider
documentation and model lists with `verified` date and source URL; resolve-time rejection of
unsupported settings; `dejaview catalog show`. Models: the owner's initial subset from 0.4 first.

**Acceptance.** Catalog self-test: every entry complete with sources; unsupported-setting
rejections tested; `catalog show` prints an entry.

**Verification.** `cargo test -p dejaview-providers -- catalog`.

**Dependencies.** 1.3, 0.4. **Files.** `providers/catalog.toml`,
`crates/dejaview-providers/src/catalog.rs`. **Size.** M.

### Task 2.2: Wire schema transformations

**Description.** Per-adapter transformations and normalisation back to canonical; resolve-time
representability check; snapshot fixtures.

**Acceptance.** For each adapter: transformed submission schema accepts the spec example; a
payload violating a removed constraint still fails canonical validation; an unsupported construct
is refused with the reason.

**Verification.** `cargo test -p dejaview-providers -- wire`.

**Dependencies.** 1.6a, 2.1. **Files.** `crates/dejaview-providers/src/wire.rs`, `fixtures/schemas/`.
**Size.** M.

### Task 2.3: Image preparation and preflight

**Description.** Deterministic preparation with `prep_version`, metadata stripping, preflight over
the whole provider history including tool images.

**Acceptance.** Identical bytes on repeat; exact native size; PNG kept with alpha; no EXIF or XMP;
a scripted conversation reaching the many-image threshold fails preflight before sending.

**Verification.** `cargo test -p dejaview-providers -- prepare preflight`.

**Dependencies.** 1.2, 2.1. **Files.** `crates/dejaview-providers/src/prepare.rs`. **Size.** M.

### Task 2.4: Privacy enforcement before any live call

**Description.** Consent bound to profile id, account and upstream allowlist; recipient chain in
the report; `error: auth` on a missing credential; cross-host redirects refused by the HTTP
client; no filenames or paths in requests; redaction re-tested with real adapter request shapes;
a fixture-directory test that greps every recorded request for paths and EXIF markers;
`docs/provider-data.md` columns defined.

**Acceptance.** Run refused without `account`; redirect to another host is `error: provider`;
missing key is `error: auth`; the fixture grep test exists and runs on every fixture directory.

**Verification.** `cargo test -p dejaview-providers -- privacy`.

**Dependencies.** 1.3, 1.10a. **Files.** `crates/dejaview-providers/src/{http,privacy}.rs`,
`crates/dejaview-core/src/report.rs`, `docs/provider-data.md`. **Size.** S.

### Task 2.5a: Anthropic adapter implementation and replay harness

**Description.** Serde types for the Messages API subset; pairing; thinking blocks echoed;
`classify`; the three final-turn mechanisms gated by the catalog; the `anthropic` profile; the
fixture replay harness shared by all adapters (canonical request comparison minus declared
volatile transport fields; opaque payloads byte-exact).

**Acceptance.** Replay harness tested with hand-written fixtures; dropping a thinking block fails
replay; changing a volatile header passes.

**Verification.** `cargo test -p dejaview-providers -- anthropic replay`.

**Dependencies.** 2.2, 2.3, 2.4, 1.8. **Files.** `crates/dejaview-providers/src/anthropic/`,
`crates/dejaview-providers/src/replay.rs`, `providers/profiles/anthropic.toml`. **Size.** M.

### Task 2.5b: Anthropic recordings

**Description.** Record live fixtures under `DEJAVIEW_LIVE=1` on the account named in 0.4 for
every capability combination: first look, parallel batch with two same-name calls, tool error,
multimodal tool result, each repair case in both forms, final-turn transition with a stray call
under each supported mechanism, the largest permitted image conversation; strip secrets; commit;
replay. One recording designated the profile-conformance fixture. Spend recorded against the 0.4
cap.

**Acceptance.** Every listed fixture exists and replays; live `dejaview check` on a trivial job
returns a verdict with resolved model, API version, usage and cost.

**Verification.** `cargo test -p dejaview-providers -- anthropic`; `DEJAVIEW_LIVE=1 cargo test -p dejaview-providers --test record -- --ignored anthropic`.

**Dependencies.** 2.5a. **Files.** `fixtures/adapters/anthropic/`, `fixtures/profiles/anthropic/`.
**Size.** M; real money. Tag `m2.1`.

### Task 2.6a: Gemini adapter implementation

**Description.** `functionCall` ids with the catalog exception, thought signatures as original
parts, function calling mode, `parametersJsonSchema`, `classify`, the `google-ai-studio` profile;
a replay test that a moved or altered thought signature fails.

**Acceptance.** As 2.5a plus the signature test.

**Verification.** `cargo test -p dejaview-providers -- gemini`.

**Dependencies.** 2.5a. **Files.** `crates/dejaview-providers/src/gemini/`,
`providers/profiles/google-ai-studio.toml`. **Size.** M.

### Task 2.6b: Gemini recordings

**Description.** As 2.5b for Gemini.

**Dependencies.** 2.6a. **Files.** `fixtures/adapters/gemini/`, `fixtures/profiles/google-ai-studio/`.
**Size.** M; real money.

### Task 2.7a: OpenAI adapter implementation

**Description.** Responses API: strict wire schema, `call_id` matching, reasoning items echoed,
`store = false` on every request (tested), `incomplete` as non-replayable, `classify`, the
`openai` profile.

**Acceptance.** As 2.5a plus the `store = false` test.

**Verification.** `cargo test -p dejaview-providers -- openai`.

**Dependencies.** 2.5a. **Files.** `crates/dejaview-providers/src/openai/`,
`providers/profiles/openai.toml`. **Size.** M.

### Task 2.7b: OpenAI recordings

**Description.** As 2.5b for OpenAI.

**Dependencies.** 2.7a. **Files.** `fixtures/adapters/openai/`, `fixtures/profiles/openai/`.
**Size.** M; real money.

### Task 2.8: Gateway profile candidates (optional)

**Description.** Profiles for `vercel-ai-gateway-anthropic`, `cloudflare-ai-gateway-anthropic`,
`cloudflare-ai-gateway-google` with upstream contracts; fixtures including routing body and
provenance; `unshipped` marking with reasons when provenance cannot be read. Only for gateways the
owner holds accounts for; spend against the 0.4 gateway-smoke entry. Nothing depends on this task.

**Acceptance.** Each shipped gateway profile has a routing and provenance fixture; a disallowed
upstream fails; unshipped candidates are refused by `check` with the reason.

**Verification.** `cargo test -p dejaview-providers -- profiles`.

**Dependencies.** 2.5b, 2.6b. **Files.** `providers/profiles/`, `fixtures/profiles/`. **Size.** M.

### Task 2.9: provider-data.md

**Description.** Fill enforced, attested and unverifiable columns for the three direct profiles
and any shipped gateway, with citations; a test that every shipped profile id appears.

**Dependencies.** 2.5b, 2.6b, 2.7b. **Files.** `docs/provider-data.md`. **Size.** S.

### Task 2.10a: Packaging for the registry

**Description.** Versioned registry dependencies between all six workspace crates; the assets
that exist by M2 (rules, prompts, profiles, catalog, schemas) packaged with the crates that embed
them, verified by `cargo package` and a test that lists every embedded asset from the packaged
crate (the viewer is added to this check in 3.6); a publication script in the order `core`,
`testkit`, `providers`, `tools`, `eval`, `dejaview` that the owner runs; a CI job that packages
every crate, stages the `.crate` files in a temporary local registry, and installs `dejaview` from
that registry in a clean container without the repository tree, then runs the offline smoke
(`dejaview --version`, `tools list`, `rules show`, `check` without a key yielding `error: auth`).
The workspace-level publish dry run supported by the pinned Cargo version runs in CI.

**Acceptance.** Staged-registry install smoke passes in CI; the workspace publish dry run passes.
After the owner's first real publication (scheduled at the M5 checkpoint), from outside the
repository `cargo install dejaview --version <released> --locked` succeeds and passes the smoke.

**Verification.** `.github/workflows/package.yml`; `scripts/publish.sh --dry-run`;
`scripts/registry-install-test.sh`.

**Dependencies.** 2.5b, 2.6b, 2.7b. **Files.** `scripts/publish.sh`, `.github/workflows/package.yml`,
crate manifests. **Size.** M.

### Task 2.10b: Binary distribution and live smoke

**Description.** `cargo-dist` producing macOS arm64 and Linux x86_64 and arm64 archives on a tag;
archive smoke on each architecture in CI (offline smoke as above); one shipped example config per
use case under `examples/`; a capped, owner-approved live smoke through all three direct profiles
from an installed archive, recorded in the PR.

**Acceptance.** Release workflow dry run produces three archives that pass the offline smoke on
their architectures; live smoke reports attached.

**Verification.** `.github/workflows/release.yml` dry run; `scripts/archive-smoke.sh`.

**Dependencies.** 2.10a. **Files.** `.github/workflows/release.yml`, `dist-workspace.toml`,
`examples/`. **Size.** M; small real money.

### Checkpoint M2
- [ ] Live smoke on all three direct profiles; reports and costs in the PR.
- [ ] Fixture replay covers every capability combination; one profile-conformance fixture per
      shipped profile.
- [ ] SPEC open question 1 closed.
- [ ] Review and test skills (or checklist); owner review; merge; tag `m2`.

## Milestone M3: Evaluation sets and baseline

### Task 3.1: Set manifests, check, fetch and seal

**Description.** Manifest parsing; `sets check` (hashes, roles, families across splits, label
presence per split, public manifests and jobs contain no answer labels, jobs resolve); `sets
fetch` by hash; `sets seal` writing `seal.toml` over sorted membership, families, canonical jobs,
image hashes, rules content and version, and the external label file's hash, with timestamp and
owner attestation text; `sets verify-seal`.

**Acceptance.**
- [ ] Each failure mode of `check` has a fixture; `fetch` refuses a mismatch; a test mutates each
      seal component in turn (membership, a family, a job field, an image, rules, labels) and
      `verify-seal` rejects each.

**Verification.** `cargo test -p dejaview-eval -- sets seal`.

**Dependencies.** 1.4b. **Files.** `crates/dejaview-eval/src/{sets,seal}.rs`,
`crates/dejaview-cli/src/sets.rs`, `fixtures/evalsets/`. **Size.** M.

### Task 3.2a: Set-building toolkit and ADR 0013

**Description.** `scripts/sets/` helpers: generate sources and candidates with the chosen image
models, plant defects by construction, write `job.toml`, draft labels and regions from the
construction record only, write the owner's worksheet (`labels-draft.toml` plus contact sheet) to
owner storage outside the repository, record generator and licence per item. ADR 0013 names
generators, licence and the review procedure. Create the `dejaview-evalsets` repository.
`docs/evalsets.md` documents the procedure.

**Acceptance.** A two-item dry run yields a valid set directory and an external worksheet; ADR
written; evalsets repository exists with README and licence.

**Dependencies.** 3.1, 0.4. **Files.** `scripts/sets/`, `docs/evalsets.md`,
`docs/adr/0013-set-generation.md`. **Size.** M.

### Task 3.2b: First dev set adjudicated and published

**Description.** Reference consistency, 20 items, at least two failure types, exemption-adjacent
items, families recorded; owner adjudicates the worksheet; images reviewed and pushed to the
evalsets repository by the owner; manifest and labels (dev sets publish labels) committed here.

**Acceptance.** `sets check` passes; the review log lists every image; the owner attests in the
PR that no judge or tool ran on the items (journal search of local output directories attached as
supporting evidence).

**Dependencies.** 3.2a. **Files.** `evalsets/reference-consistency/pets-v1/`. **Size.** M; owner
time.

### Tasks 3.3a to 3.3d: Remaining dev sets

**Description.** One task per use case: source fidelity (exercising the generic base and the
architectural profile), reality checks, brief compliance, product fidelity; 20 items each, roughly
balanced, at least two failure types, exemption-adjacent items, families recorded; built with the
accepted procedure and adjudicated by the owner.

**Acceptance.** As 3.2b per set; `evalsets/README.md` tabulates items, failure types and families.

**Dependencies.** 3.2b. **Files.** `evalsets/<use-case>/`. **Size.** M each; owner time.

### Task 3.3e: Adversarial sets

**Description.** Five two-item sets, one per use case under the existing schema, whose images,
labels or briefs carry instruction-like text aimed at the judge; expected verdicts from
construction; same review log and acceptance as 3.2b; the baseline reports them separately.

**Dependencies.** 3.2b. **Files.** `evalsets/<use-case>/adversarial-v1/`. **Size.** S; owner time.
Tag `m3.1`.

### Task 3.4a: Execution descriptor and durable evaluation ledger

**Description.** `matrix.toml` parsing; the execution descriptor over every answer-affecting
input (matrix, set manifests, jobs, image hashes, rules content, prompt bytes, tool manifests and
lock digests, profiles, catalog version, harness version); a single-writer lock on the evaluation
directory; the durable ledger: each attempt's hold persisted before send, settlement at most once,
unresolved sent-or-possibly-sent attempts charged at bound after restart, completed errors kept;
per-decision admission against the durable remaining cap; the versioned admission-record type
(`evalsets/admissions.toml`: tool, use case, stage, receipt id), the receipt-reference type, their
reader, and synthetic records for tests (scoring and writing them belong to 3.5b).

**Acceptance.** Crash tests with injected faults immediately before and after hold persistence,
send, settlement and completion marking each recover to a consistent ledger; a changed descriptor
refuses resume; two writers are refused; cumulative spend across restarts never exceeds the cap;
the admission record round-trips.

**Verification.** `cargo test -p dejaview-eval -- descriptor eval_ledger admissions`.

**Dependencies.** 1.9, 1.11, 3.1. **Files.** `crates/dejaview-eval/src/{matrix,descriptor,eval_ledger,admissions}.rs`.
**Size.** M.

### Task 3.4b: Matrix runner, arms and resume

**Description.** `eval run` executing every cell with cache disabled, immutable per-item run
directories, recovery states for partial items, resume skipping complete items, exact A/C/D_X/D
membership from the admission record (D includes only `confirmed` tools), prompt equivalence
across arms except the Tools section, projection application, summary JSON, repetition 1 flagged
as the decision repetition.

**Acceptance.** Fake-provider matrix runs every cell, never touches the cache (spy), stops at the
cap with `error: budget` per unstarted item, resumes after SIGINT without redoing complete items
and marks partial items; arm membership and prompt-equivalence tests pass; against the 3.4a
synthetic admission records, a `provisional` tool is excluded from D and a `confirmed` one
included.

**Verification.** `cargo test -p dejaview-eval -- runner arms`.

**Dependencies.** 3.4a. **Files.** `crates/dejaview-eval/src/{runner,arms}.rs`. **Size.** M.

### Task 3.4c: Pre-run estimate

**Description.** `dejaview eval estimate --matrix …` prints decisions, per-decision envelopes,
their sum and a cost range, and warns when the sum exceeds the cap (completion not guaranteed);
it never refuses, because admission is per decision.

**Acceptance.** Each estimated decision envelope equals the ledger's reservation for that
decision on a fake run; totals equal their sum; a matrix whose sum exceeds the cap can still
complete when settled costs permit (test).

**Dependencies.** 3.4b. **Files.** `crates/dejaview-eval/src/estimate.rs`. **Size.** S.

### Task 3.5a: Metrics, localisation and report regeneration

**Description.** Confusion, missed and detected failures under the frozen policy, false rejects,
unsure and error rates, localisation with eligibility rules, cost totals and medians, wall time
totals and medians, Wilson intervals from 1.15, headline numbers from repetition 1 only,
`dejaview eval report` regenerating the Markdown summary from saved results.

**Acceptance.** Hand-computed fixtures match to 1e-9 for proportions and exactly for counts,
totals and medians; localisation fixtures cover each eligibility rule; a fixture where later
repetitions reverse the apparent winner leaves the headline unchanged; `eval report` on saved
results reproduces the stored Markdown byte for byte.

**Verification.** `cargo test -p dejaview-eval -- metrics summary`.

**Dependencies.** 3.4b, 1.15. **Files.** `crates/dejaview-eval/src/{metrics,summary}.rs`. **Size.** M.

### Task 3.5b: Gate lifecycle

**Description.** `eval gate` with stages: verify the seal against supplied labels, verify results
against the descriptor and frozen matrix identity, refuse repeat scoring without a new ADR
reference, write the receipt, write label publication and split change as files for the owner to
review and commit (never commits), resume an interrupted scoring from the receipt without
rescoring; the confirmation stage requires the initial receipt, the identical frozen judging
configuration, at least 50 items from disjoint families, and yields `confirmed` only when both
stages are admitted; `evalsets/admissions.toml` records tool, use case, stage, receipt id.

**Acceptance.** Table tests for every ceiling, margin, family minimum, multiplicity boundary and
outcome precedence; label-hash mismatch, descriptor mismatch and repeat scoring refused; the
initial stage refuses a non-blind set or fewer than 20 items; the confirmation stage refuses a
missing initial receipt, fewer than 50 items, a family overlapping any earlier set, or a changed
judging configuration; interrupted scoring resumes; written admission records are read back by
the 3.4a reader.

**Verification.** `cargo test -p dejaview-eval -- gate`.

**Dependencies.** 3.5a, 3.1. **Files.** `crates/dejaview-eval/src/gate.rs`. **Size.** M.

### Task 3.6: Static report viewer

**Description.** `viewer/` with no build step; `dejaview report view <run-dir>` serves on
loopback or opens the file; shows images, tool results, checklist, flat locations in raw
coordinates, aggregation rows and decision table; declares supported `report_version`s; no
requests except to loopback.

**Acceptance.** Devtools check on a fake run: no console errors, no non-loopback requests; a
crop-frame location is drawn at the stored overlay position (pixel compare); the viewer assets are
added to the 2.10a embedded-asset packaging check and the staged-registry smoke opens a report.

**Verification.** Browser devtools script; `cargo test -p dejaview --features testkit --test viewer_serve`.

**Dependencies.** 1.10b. **Files.** `viewer/`, `crates/dejaview-cli/src/report.rs`, `docs/viewer.md`.
**Size.** M.

### Task 3.7: Canary and requalify

**Description.** `eval canary`: pinned canary set, uncached, compared to the stored baseline
identity; `suspect` on one deviation, `recommend_disable` on two consecutive against the same
baseline; never disables; `eval requalify` runs the full development set and records a new
baseline on owner approval.

**Acceptance.** Fake-provider tests for `clear`, `suspect`, `recommend_disable` and baseline
reset after requalification.

**Dependencies.** 3.4b. **Files.** `crates/dejaview-eval/src/canary.rs`. **Size.** S.

### Task 3.8: Arm A baseline and publication

**Description.** `eval estimate` approved by the owner against the 0.4 cap; run the matrix (five
dev sets plus five adversarial sets × the initial model subset × arm A × `decision_runs = 1`,
`repetitions = 2`); publish `docs/results/baseline-<date>.md` with every metric, resolved model
ids and API versions, costs, adversarial results reported separately, the matrix and summary JSON,
stating these are development sets.

**Acceptance.** Results document and matrix committed; spend under cap and recorded.

**Dependencies.** 2.5b, 2.6b, 2.7b, 3.3a, 3.3b, 3.3c, 3.3d, 3.3e, 3.4c, 3.5a. **Files.**
`docs/results/`, `evalsets/matrix/baseline.toml`. **Size.** M; real money.

### Checkpoint M3
- [ ] Ten public sets published after review; baseline published; viewer works offline.
- [ ] Review and test skills (or checklist); owner review; merge; tag `m3`.

## Milestone M4: Built-in tools and MCP client

### Task 4.0: Alignment feasibility spike and ADR 0014

**Description.** A bounded spike (two days): flat homography estimation with the `image` crate
plus candidate dependencies, on synthetic pairs with known transforms, unrelated pairs,
low-texture pairs and degenerate transforms; tolerances (translation within 0.5 px at 2048 width,
rotation within 0.1°, inlier fraction reported); failure fixtures; equirectangular rotation
estimation scoped the same way on newly generated or CC-licensed panoramas (owner decision 11:
nothing from the prototype is reused).

**Acceptance.** ADR 0014 with measured tolerances and the chosen approach; failure fixtures
committed.

**Dependencies.** 1.2, 0.4. **Files.** `docs/adr/0014-alignment.md`, `fixtures/images/align/`.
**Size.** M.

### Task 4.1: Registry in production

**Description.** Built-in registration, `[tools] enabled` and `order`, Tools section with
`requires` ordering, projection mechanism applied before the checklist, `dejaview tools list` over
real built-ins.

**Acceptance.** Ordering and `requires` tests; byte-stable rendering snapshot; projection applied
before the checklist sees the envelope.

**Dependencies.** 1.5a, 1.7a. **Files.** `crates/dejaview-tools/src/{registry,projection}.rs`.
**Size.** S.

### Task 4.2a: `view`, `resize` and equirectangular geometry

**Description.** Crop by bbox; square perspective view by yaw, pitch, fov (port of the
prototype's projection, verified against stored views); one image or two; `crop` frames;
`resize`; equirectangular registry maths (view to raw, seam rules, clipping).

**Acceptance.** Crop equals stored crop within 1 grey level per channel; perspective view within 2
grey levels of the stored view; view-to-raw round trip within 0.5 px at 2048 width;
seam-straddling bbox refused; `fov = 130` is `invalid_args`.

**Verification.** `cargo test -p dejaview-tools -- view resize` and `cargo test -p dejaview-core -- frames::sphere`.

**Dependencies.** 4.1, 1.2. **Files.** `crates/dejaview-tools/src/builtin/{view,resize}.rs`,
`crates/dejaview-core/src/frames.rs`. **Size.** M.

### Task 4.2b: Viewer equirectangular overlay

**Description.** The viewer draws `view` locations and equirectangular polygons on panoramas
through the frame registry.

**Acceptance.** Stored-overlay pixel compare for a `view` location on the fixture panorama.

**Dependencies.** 4.2a, 3.6. **Files.** `viewer/`. **Size.** S.

### Task 4.3a: `align` for flat pairs and `visual_only`

**Description.** From the spike: homography, `aligned` frame with lossless warped pixels,
integrity measurements, `not_applicable` for unrelated pairs, the `visual_only` projection.

**Acceptance.** Synthetic transforms recovered within ADR tolerances; unrelated pair is
`not_applicable`; `visual_only` strips `quality`, `measurements`, `regions` and fixes `summary`.

**Dependencies.** 4.0, 4.1. **Files.** `crates/dejaview-tools/src/builtin/align.rs`. **Size.** M.

### Task 4.3b: `align` for equirectangular pairs

**Description.** Rotation on the sphere, warped pixels, the same envelope.

**Acceptance.** Known rotations recovered within ADR tolerances on stored panoramas.

**Dependencies.** 4.3a, 4.2a. **Files.** `crates/dejaview-tools/src/builtin/align_sphere.rs`.
**Size.** M.

### Task 4.3c: `edge_similarity` and `change_map`

**Description.** Edge similarity over an aligned pair; change map over an aligned pair returning
one image and no regions.

**Acceptance.** Identical images give 1.0 within 1e-6; blank versus textured gives below 0.05;
the change map for a fixture pair equals the stored golden image within 1 grey level.

**Dependencies.** 4.3a. **Files.** `crates/dejaview-tools/src/builtin/{edge,change_map}.rs`.
**Size.** S. Tag `m4.1`.

### Task 4.4a: Full MCP client: ADR 0015, pagination, concurrency, images

**Description.** ADR deciding `rmcp` versus the in-house client after a bounded spike; paginated
`tools/list`; concurrent calls where declared; frames in `_dejaview`; image content blocks with
exact correspondence; `isError` precedence as an exhaustive truth table; `outputSchema` validation.

**Acceptance.** Fake-server tests for each; the truth table test enumerates every combination.

**Dependencies.** 1.13, 4.1. **Files.** `docs/adr/0015-mcp-client.md`, `crates/dejaview-tools/src/mcp/`.
**Size.** M.

### Task 4.4b: MCP failure modes

**Description.** Process death mid-call, malformed frames, unknown ids, late responses after
cancellation, reset failure restart, shutdown escalation on an unresponsive server, lease
contention between two decisions.

**Acceptance.** One fake-server test per mode; lease contention shows exclusive access.

**Dependencies.** 4.4a. **Files.** `crates/dejaview-tools/src/mcp/`,
`crates/dejaview-testkit/src/bin/mcp.rs`. **Size.** M.

### Task 4.5: Arm C on the dev sets

**Description.** `eval estimate` approved; matrix with arms A and C on the ten sets for the
initial models; publish `docs/results/arm-c-<date>.md` with every metric per set and model and the
identity fields. Paired flips are computed between repetition 1 of A and C within this matrix,
matching item, model and judging settings, with a check that answer-affecting differences are
limited to the declared arm; the M3 baseline is shown separately as historical context, never as
the comparison arm.

**Acceptance.** Results document and matrix committed with identity fields and within-matrix
paired flips; spend under cap.

**Dependencies.** 4.2a, 4.3b, 4.3c, 3.8. **Files.** `docs/results/`, `evalsets/matrix/arm-c.toml`.
**Size.** S; real money.

### Task 4.6: Segmenter artifact and MODEL.md

**Description.** Export a segmenter from publicly licensed upstream weights (candidate EoMT-L
ADE20K; licence check first, alternative by ADR if it fails) with this project's own export
script, publish it as a release asset, and write `tools/python/MODEL.md` (upstream source and
licences, export procedure and versions, URL, SHA-256, preprocessing version, onnxruntime version
and execution provider, parity tolerance, golden inputs generated here). Nothing from the
prototype is reused (owner decision 11). M4 exit prerequisite.

**Acceptance.** `MODEL.md` complete; the export script is committed and reproduces the artifact
hash; the asset downloads and matches.

**Dependencies.** 0.4. **Files.** `tools/python/MODEL.md`, `tools/python/scripts/export_segmenter.py`.
**Size.** M.

### Checkpoint M4
- [ ] `docs/tools.md` explains writing a tool with `view` as the worked example; `MODEL.md`
      complete; alignment ADR recorded.
- [ ] Review and test skills (or checklist); owner review; merge; tag `m4`.

## Milestone M5: First ML tool

### Task 5.1: Python pack skeleton and MCP server

**Description.** Grow the 1.14 pack into the real server: `_dejaview` images and frames, envelope
as `structuredContent` plus image blocks, `dejaview/reset`, `fetch` entry point, offline pytest
and ruff.

**Acceptance.** Rust client calls `echo_region` with an image and an aligned frame and receives a
valid envelope with one image block; offline tests pass.

**Dependencies.** 4.4b, 1.14. **Files.** `tools/python/src/dejaview_tools/`. **Size.** M.

### Task 5.3: Segmenter fetch and parity

**Description.** `fetch` downloads and verifies the artifact from `MODEL.md`; a `segmenter`-marked
parity test against stored golden outputs within the documented tolerance.

**Acceptance.** Parity passes; `fetch` refuses a hash mismatch.

**Dependencies.** 4.6, 5.1. **Files.** `tools/python/src/dejaview_tools/segmenter.py`. **Size.** S.

### Task 5.4a: `regions` tool

**Description.** Segmentation-based change-region finder over an aligned pair, evidence kind
`change_region`, accepts `hints.regions`, bounded regions with labels and scores, one overlay
image, the prototype's decision tables stripped to high-recall pointers; manifest with
`requires = ["align"]`.

**Acceptance.** On newly generated synthetic pairs with planted changes, every planted region is
found with IoU above 0.3 and at most `max_regions` are returned; the manifest passes all three
adapters' wire transformations.

**Dependencies.** 5.3. **Files.** `tools/python/src/dejaview_tools/regions.py`. **Size.** M.

### Task 5.4b: `classes` tool

**Description.** Segmenter classes before and after in a region, evidence kind
`class_difference`.

**Acceptance.** Named fixtures return the exact expected class lists; manifest passes wire
transformations.

**Dependencies.** 5.3. **Files.** `tools/python/src/dejaview_tools/classes.py`. **Size.** S.

### Task 5.2: Provisioning bundle for the real pack

**Description.** Bundle build from the final pack sources including `regions`, `classes` and the
segmenter fetch; `tools setup` end to end; lock digest in provenance and cache key; clean-install
CI job; `unshare -rn` runtime check. Numbered 5.2 for continuity with draft 2 but scheduled after
the tools it packages.

**Acceptance.** As 1.14 for the real pack, with `regions` and `classes` listed by the installed
server; cache key changes when the lock digest changes.

**Dependencies.** 5.3, 5.4a, 5.4b. **Files.** `tools/python/bundle/`, `scripts/build-bundle.sh`.
**Size.** S. Tag `m5.1`.

### Task 5.5: Sealed blind set for source fidelity

**Description.** Before any judge model or tool runs on these items: build 20 source-fidelity
items with families disjoint from the dev set, including exemption-adjacent items; the owner
adjudicates from construction records only; labels in owner storage from creation; `sets seal`;
images published with the manifest carrying `seal.toml` and `labels_sha256` only.

**Acceptance.** `sets check` and `verify-seal` pass; public manifest and jobs carry no answer
labels (test); owner attestation of no prior exposure recorded in the seal; local output
directories searched as supporting evidence.

**Dependencies.** 3.2a, 3.1. **Files.** `evalsets/source-fidelity/blind-v1/`. **Size.** M; owner
time.

### Task 5.6: Initial gate D_regions versus C

**Description.** Freeze the matrix (model, prompt, obligations version, budgets, policy, error
ceiling, hypothesis count, family minimum, seed); `eval estimate` approved; run arms C and
D_regions on the sealed set; `eval gate --stage initial` (seal verification, receipt, label and
split files); the owner reviews and commits the publication; publish
`docs/results/gate-regions-initial-<date>.md` with the outcome, marked provisional.

**Acceptance.** Receipt, publication and results committed; spend under cap; outcome stated as
`admitted (provisional)`, `rejected` or `inconclusive`; `admissions.toml` updated.

**Dependencies.** 5.4a, 5.5, 4.5, 3.5b, 5.2. **Files.** `docs/results/`,
`evalsets/matrix/gate-regions.toml`, `evalsets/admissions.toml`. **Size.** M; real money.

### Checkpoint M5
- [ ] Spec success criteria 1 to 7 checked one by one; the full offline suite rerun under five
      minutes with every fixture and tool present.
- [ ] Final artifacts rebuilt from the final sources: Rust packages (staged-registry install
      smoke), platform archives (architecture smoke), Python bundle (clean install); the owner
      publishes to crates.io and `cargo install dejaview --version <released> --locked` passes
      from outside the repository.
- [ ] README "How it is built" updated with measured results and links.
- [ ] Review and test skills (or checklist); owner review; merge; tag `m5`.

## Milestone M6: Confirmation

### Task 6.1: Confirmation set

**Description.** At least 50 source-fidelity items, families disjoint from every earlier set,
built with the 3.2a procedure in two batches of 25, owner-adjudicated from construction, labels
external, sealed before any run.

**Acceptance.** `sets check` and `verify-seal` pass; family disjointness against every published
set verified by a test; owner attestation recorded.

**Dependencies.** 5.6, 0.4 (M6 funding). **Files.** `evalsets/source-fidelity/confirm-v1/`.
**Size.** M per batch; owner time.

### Task 6.2: Confirmation gate and admission record

**Description.** Same frozen configuration as 5.6, referencing its receipt; run C and D_regions;
`eval gate --stage confirmation`; owner reviews and commits publication; publish
`docs/results/gate-regions-confirmation-<date>.md`; `admissions.toml` becomes `confirmed` only if
both stages are admitted, and arm D then includes `regions`; otherwise the outcome is recorded and
the tool stays out of D.

**Acceptance.** Receipt references the initial receipt; configuration equality verified by the
gate; results, publication and admission record committed; spend under cap.

**Dependencies.** 6.1. **Files.** `docs/results/`, `evalsets/admissions.toml`. **Size.** M; real
money.

### Checkpoint M6
- [ ] Owner review; merge; tag `m6`.

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Provider APIs differ from the documentation read at M2 | High | Record fixtures early; every catalog entry cites source and date; preflight fails loudly; re-verify before `m2`. |
| Preserved-thinking prefix check rejects a request believed append-only | High | Provider-history invariant test in 1.7a; frozen tool list; `refuse` mechanism; record on the enforced account named in 0.4. |
| Owner labelling time dominates M3, M5, M6 | Medium | Toolkit drafts labels from construction; worksheets before each set; batches of 20 to 25; labelling overlaps M2 outside git. |
| Generated sets easier than real failures | Medium | Generator recorded per item; real failures added over time; stated in every results document. |
| Model artifact unavailable, incompatible or not licence-clean | Medium | 4.6 is an M4 exit prerequisite; fallback is an alternative segmenter with the same contract, by ADR. |
| Alignment infeasible at the needed accuracy | Medium | 4.0 spike with tolerances and failure fixtures; `align` may return `not_applicable` honestly. |
| Too few independent families for a conclusive gate | Medium | Family minimum frozen before unsealing; `inconclusive`, never post-hoc expansion; M6 set of 50 or more. |
| `rmcp` does not fit | Low | 1.13 proves the lifecycle in-house; 4.4a decides by ADR. |
| Rust velocity on orchestration | Medium | Plain types, boxed traits, every task S or M, walking slice first. |
| Evaluation and recording spend | Medium | Caps and approval points in 0.4; `eval estimate` before every paid run; durable ledger with holds persisted before send. |
| Blind-set contamination | High | Labels external from creation; seal over the whole experiment before any run; gate verifies the seal; owner attestation; contaminated sets never reused. |
| Packaged crate missing embedded assets | Medium | 2.10a installs from packaged `.crate` files in a clean container and lists every asset. |
| Gateway provenance unavailable | Low | Candidates unshipped; nothing depends on them. |

## Parallelisation

Within a milestone, task branches may run in parallel when their dependencies allow: in M1, 1.2
and 1.3 after 1.1; 1.4a after 1.3 alongside 1.5a; 1.15 at any time after 0.1; 1.13 and 1.14 only
after 1.12. In M2, 2.6a and 2.7a after 2.5a; recordings after their implementations. In M3, 3.6
alongside 3.4 and 3.5; owner adjudication of 3.3a to 3.3e outside git while 3.4 and 3.5 are
built. In M4, 4.0 alongside 4.1; 4.4a after 4.1 alongside 4.2a. No code work on a later milestone
begins before the earlier milestone merges.

## Owner decision points (from 0.4)

| Before task | Decision |
|---|---|
| 2.1 | initial judge-model subset |
| 2.5b, 2.6b, 2.7b | recording cap; which Anthropic account (enforced thinking) |
| 2.8, 2.10b | gateway accounts and the release and gateway smoke cap |
| 3.2a | generators, licence, reuse of prototype assets |
| 3.2b, 3.3a to 3.3e, 5.5, 6.1 | labelling capacity and batch schedule |
| 3.8, 4.5, 5.6, 6.2 | spend cap per run, approved against `eval estimate` |
| 4.0, 4.6 | reuse approval for the prototype's views and segmenter export |
| 6.1 | whether M6 is funded in this release |

## Open questions

1. Generators and licence for public sets (3.2a, ADR 0013).
2. Decided 2026-10-26: nothing from the prototype is reused (owner decision 11); 5.4a uses new
   synthetic pairs and 4.6 exports its own segmenter.
3. Decided 2026-10-26: M6 is funded.
