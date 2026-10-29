**With changes.** Draft 2 substantially improves the plan, but it is not ready to execute unchanged. The main remaining risks are an incomplete blind-set seal, insufficiently specified crash recovery and evaluation identity, contradictory budget rules, and dependencies that still require later functionality. Release packaging now has a task, but does not yet prove the promised registry installation. `tasks/todo.md` matches all **71 task IDs, their order, and all six intermediate tags**; the problems are primarily in the task contracts it mirrors. No files were modified.

## Resolution of review 1

| # | Finding | Resolution |
|---|---|---|
| 1 | Blind-set contamination before sealing | **Partly resolves.** Tasks 3.1 and 5.5 correctly require external labels from creation and prohibit prior judge/tool output. The seal still does not bind membership, complete jobs and image hashes, while filename-history searches cannot establish label secrecy. |
| 2 | Mandatory M1 deliverables deferred | **Partly resolves.** Tasks 1.13–1.15 put all three deliverables in M1. Their acceptance criteria still leave important lifecycle, provisioning and statistical reproducibility details unspecified. |
| 3 | M1 lacks working vertical slices | **Partly resolves.** Task 1.1 now produces a non-vacuous decision and journal, and shared contracts have explicit homes. Several subsequent CLI scenarios still require tool execution, final-turn handling or repairs implemented later. |
| 4 | No installable-release task | **Partly resolves.** Task 2.10 adds binaries, examples and installation checks. Package naming contradicts the verification commands, and local installation does not establish that the workspace can be published and installed from crates.io. |
| 5 | Incomplete seal-to-confirmation lifecycle | **Partly resolves.** Task 3.5b adds hash checks, receipts, repeat-scoring protection and split transition; M6 adds confirmation. Confirmation lacks independent acceptance criteria, and the generic gate command still marks admission provisional unconditionally. |
| 6 | Privacy enforcement follows live calls | **Partly resolves.** Task 2.4 now precedes live work and covers the missing enforcement requirements. However, 2.6b and 2.7b do not explicitly depend on their respective adapter implementations. |
| 7 | Incorrect history and repair assertions | **Resolves.** Tasks 1.7a and 1.8 distinguish immutable history from mutable request controls and distinguish truncation from refusal/text reissues, with explicit tests. |
| 8 | Unresolved task splits and oversized tasks | **Partly resolves.** Most former large tasks are split. Gemini/OpenAI still share implementation/recording entries, 1.10 and 4.2 combine independent subsystems, and several later tasks lack acceptance or verification sections. |
| 9 | Resume can reset financial history | **Partly resolves.** Tasks 3.4a/b introduce persistence, identity and recovery tests. They do not define crash-safe ordering around sends and settlements, and 3.4c reintroduces the whole-matrix budget restriction that 3.4a explicitly removes. |
| 10 | Owner readiness and spending decisions too narrow | **Partly resolves.** Task 0.4 and the decision-point table cover the principal work. Release smoke tests and optional gateway recordings need explicit cap entries, and the estimator’s acceptance criterion is inconsistent with the ledger. |
| 11 | Rollback and initial-commit omissions | **Resolves.** The strategy now covers dependent reverts, recovery references, reviewed merge reverts, external-state limitations, the initial documentation commit and `Cargo.lock`. |
| 12 | Late feasibility and provenance checks | **Resolves.** Task 4.0 adds a bounded alignment spike with numerical tolerances and failure fixtures; 4.6 makes `MODEL.md` an M4 exit prerequisite with a download/hash check. |
| 13 | Optional dependencies and parallel branches inconsistent | **Partly resolves.** Gateway shipping is conditional and cross-milestone code overlap is prohibited. The dependency diagram and parallelisation prose still contradict individual task dependencies. |
| 14 | Verification instructions cannot prove their claims | **Partly resolves.** Filters after `--` are valid, and viewer, ignore checks and several tolerances are corrected. Clippy syntax, tool bootstrap, offline provisioning verification and some numerical assertions remain incomplete. |
| G1 | Cargo publication, platform binaries and clean install | **Partly resolves.** Task 2.10 exists, but registry publication dependencies and package contents remain unverified. |
| G2 | `tools list` and shorthand `check` | **Partly resolves.** Tasks 1.5a/4.1 cover `tools list`; no task explicitly tests `check --candidate --ref --brief`. |
| G3 | SIGTERM cancellation | **Resolves.** Task 1.10 explicitly tests both SIGINT and SIGTERM, cancellation journaling and report rebuilding. |
| G4 | Equirectangular viewer overlays | **Resolves.** Task 4.2 owns the implementation and a stored-overlay check; M3 supplies the viewer beforehand. |
| G5 | Report schema and boundary-type requirements | **Resolves.** Task 1.10 names the committed report schema and tests schema derivation, closed fields and versioning for boundary types. |
| G6 | Redirects, credentials, consent and direct-profile fixtures | **Partly resolves.** Enforcement and designated recordings are now explicit, but the Gemini/OpenAI recording prerequisites remain incorrect. |
| G7 | Live adversarial evaluation | **Partly resolves.** Task 3.3e and the baseline schedule it. A single set spanning use cases does not fit the specified single-use-case manifest, and its acceptance omits the image-review requirement used by the other public sets. |
| G8 | Gate lifecycle and independent confirmation | **Partly resolves.** The lifecycle is scheduled, but confirmation-state handling and its verification are incomplete. |
| G9 | Canary baseline and full requalification | **Resolves.** Task 3.7 specifies baseline identity, consecutive deviations, full-development-set requalification and baseline-reset tests. |
| G10 | Pre-M5 `MODEL.md` | **Resolves.** Task 4.6 and checkpoint M4 establish the required prerequisite. |
| G11 | Review 4’s three M1 proofs | **Partly resolves.** All three have M1 tasks, but their executable contracts need the additions below. |

## New findings

1. **[major] The seal protects labels but not the complete experiment**

   **Section:** 3.1, 5.5; seal verification in 3.5b.

   **Why:** Draft 2 records a label hash, timestamp and rules version, but does not require an immutable digest of set membership, jobs and image hashes. Those inputs could change after sealing while the label hash still passes. Checking journal timestamps only establishes ordering for available journals, and grepping history for a label filename misses renamed labels, worksheets and copied contents.

   **Concrete change:** Add: “The seal binds sorted item membership, family assignments, complete canonical jobs, image hashes, rules content/version and the external label-file hash. Any change invalidates the seal. Tests mutate each component and require rejection.” Replace the filename-grep claim with checks for prohibited label fields/artifacts plus an owner attestation covering prior exposure; state that available journal searches are supporting evidence, not proof of no prior execution.

2. **[major] Resume identity and crash recovery can still mix experiments or lose charges**

   **Section:** 3.4a, 3.4b.

   **Why:** The proposed identity hashes the matrix file, manifests and version identifiers, but not necessarily referenced job contents, prompt bytes, tool manifests/locks, provider profiles or the harness build. A resumed directory could therefore combine results produced by different effective configurations. “Preserves charges after a simulated crash” also leaves unspecified what happens when a request was sent but its settlement was not persisted.

   **Concrete change:** Add: “Persist an immutable execution descriptor containing all resolved answer-affecting inputs, including jobs, images, prompts, rules, tools/locks, provider/catalog settings and harness identity; reject resume on any mismatch. Durably record each attempt hold before sending. After restart, charge unresolved sent-or-possibly-sent attempts at their bounds, settle at most once, and preserve completed errors.” Require crash tests immediately before/after hold persistence, send, settlement and completion marking, plus exclusion of two simultaneous writers to one evaluation directory.

3. **[major] The estimator reinstates the budget policy that the ledger task removes**

   **Section:** 3.4a–3.4c.

   **Why:** Task 3.4a explicitly chooses per-decision admission, but 3.4c refuses execution unless the sum of all worst-case envelopes fits. These are different policies: a matrix can legitimately complete under per-decision admission because earlier decisions release unused reservations. The estimator’s total envelope sum also cannot equal a single reservation on a multi-decision run.

   **Concrete change:** Replace 3.4c’s refusal with: “Report the worst-case envelope sum and warn when completion is not guaranteed; execution admits each decision against the durable remaining cap.” Replace its acceptance criterion with: “Each estimated decision envelope equals the ledger’s reservation for that decision; totals equal their sum; a matrix whose envelope sum exceeds the cap can still complete when settled costs permit.”

4. **[major] Package identity and the fake-provider build path are contradictory**

   **Section:** Crate graph, 0.1, 1.1; CLI verification commands.

   **Why:** With `package.name = "squint"`, `cargo ... -p squint-cli` does not select that package merely because its directory or conceptual crate is called `squint-cli`. Cargo package selection uses package identity. Separately, the CLI imports scripted support behind a feature while the graph declares testkit exclusively a dev-dependency; “enabled for tests and debug builds” does not itself define Cargo feature activation. [Cargo package selection](https://doc.rust-lang.org/cargo/commands/cargo-pkgid.html), [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html).

   **Concrete change:** State: “Directory `crates/squint-cli`; package and binary `squint`; all package selectors use `-p squint-cli`.” Resolve name availability in 0.1. Define the scripted CLI build explicitly—such as a non-default `testkit` feature enabling an optional testkit dependency—and add the necessary graph exception and feature-enabled test command; verify ordinary release builds reject `--provider-script`.

5. **[major] The dependency lists still permit impossible execution orders**

   **Section:** Task dependency graph, 1.5b–1.8, 2.6a/b, 2.7a/b, 3.8, 4.5; Parallelisation.

   **Why:** The declared graph contains no obvious directed cycle, but several acceptance scenarios implicitly depend on later tasks: evidence-producing CLI runs precede tool execution, and final-turn scenarios precede the repair machinery they require. Both Gemini and OpenAI recording entries inherit only `2.5a`, while 3.8 can start before 3.3a–d and 4.5 before `view` or spherical alignment. Parallelisation also permits 1.13/1.14 alongside their prerequisites, and the diagram misleadingly connects optional 2.8 to required later work.

   **Concrete change:** Make these explicit changes:
   - Give 2.6a/b and 2.7a/b separate entries; require `2.6b → 2.6a` and `2.7b → 2.7a`.
   - Add `1.2` before frame-dependent checklist/validation work; put registry-dependent rule validation after the manifest contract.
   - Move 1.5b’s tool-driven CLI scenario to 1.7a, repair-dependent scenarios to 1.8, and 1.7a’s final-turn assertion to 1.7b.
   - Add all 3.3a–e to 3.8’s prerequisites, and 4.2 plus 4.3b to 4.5’s.
   - Show `1.12 → 1.13 → 1.14`; remove the contradictory parallelisation claims and optional-gateway chain.
   
   Keep the already-matching todo IDs/order/tags synchronised with these corrected entries.

6. **[major] Evaluation tasks do not yet test the rules that determine admission**

   **Section:** 1.15, 3.4b, 3.5a/b.

   **Why:** A seeded interval fixture does not fully freeze RNG choice, family/item ordering or integer sampling, so reproducibility remains implementation-dependent. The runner and metrics criteria omit repetition 1 as the exclusive source of headline numbers and gates. No task explicitly verifies arm construction, identical non-Tools prompts, exclusion of unconfirmed tools from D, or the complete admitted/rejected/inconclusive decision table.

   **Concrete change:** Add to 1.15 a pinned RNG algorithm, seed encoding, canonical sampling order and stored resample-index fixture, including unequal-sized families. Add runner tests for exact A/C/D_X/D membership and prompt/projection equivalence. Add scoring fixtures where later repetitions reverse the apparent winner but do not change the gate, and table tests for every ceiling, margin, family minimum, multiplicity boundary and outcome precedence.

7. **[major] Confirmation has no distinct gate state, and scoring is coupled to a Git commit**

   **Section:** 3.5b, 5.6, 6.1, 6.2.

   **Why:** Task 3.5b always marks admission provisional; task 6.2 invokes the same command and then independently writes `admitted` into rules metadata. There is no tested transition tying confirmation to the initial receipt and frozen configuration, and neither M6 task has acceptance criteria. Making `eval gate` itself publish and commit labels also conflicts with the owner-controlled publication workflow and leaves recovery after a partially completed transition undefined.

   **Concrete change:** Define initial and confirmation gate stages. Require confirmation to reference the initial receipt, match its frozen judging configuration, use at least 50 untouched items from disjoint families, and produce confirmed admission only when the required initial and confirmation outcomes pass. Have scoring produce a durable receipt and reviewable label/split changes; publication is a separate owner-controlled step. Test interrupted scoring/publication recovery without rescoring, and define a versioned admission record consumed by arm D.

8. **[major] Release checks can pass while `cargo install squint-cli` remains impossible**

   **Section:** 2.10; checkpoint M5.

   **Why:** `cargo install --path` can use unpublished workspace dependencies and repository-level embedded assets that a packaged crate cannot access. Task 2.10 specifies neither publication order for the library packages nor verification that rules, prompts, catalog and viewer assets survive packaging. Its alternative of using the fake profile in CI also conflicts with the intended release binary’s test-only fake-provider support. [Cargo packaging requirements](https://doc.rust-lang.org/nightly/cargo/commands/cargo-package.html).

   **Concrete change:** Split package verification from binary distribution. Require versioned registry dependencies, dependency-ordered owner publication, and building/installing packaged sources without the repository tree; verify every embedded asset is present. Test archives on each target architecture, use ordinary-release offline smoke checks in CI, and perform capped owner-approved live checks through all three profiles. Explicitly schedule the final registry install check and assign release/gateway smoke spending to entries in 0.4.

9. **[major] The early MCP and provisioning proofs still omit their hardest cases**

   **Section:** 1.13, 1.14; sizing of 1.10 and 4.2.

   **Why:** Task 1.13 tests a bounded wait and reset failure, but not that the entire wait/reset/shutdown/reinitialisation sequence shares one deadline or that prior-run work is gone before reuse. Task 1.14’s pack-controlled socket failure mode does not establish that child processes cannot use networking, and its ignored installation test can download without the spec’s live-test guard. These tasks remain substantial proofs, while journal/rebuild/prune and geometry/viewer work are also grouped into large nominal M tasks.

   **Concrete change:** Add lifecycle fixtures for successful reset with no surviving prior work, missing reset support, cancellation-driven process replacement and the total acquisition deadline. Separate explicit online provisioning from an externally network-denied runtime test; guard any network-using test with `SQUINT_LIVE=1`, or pre-provision outside the test. Split 1.10 into journal/artifact durability and report/rebuild/prune, and 4.2 into tool/frame geometry and viewer integration, each with its own acceptance criteria.

10. **[major] The adversarial set does not fit the approved set schema**

    **Section:** 3.3e, 3.8.

    **Why:** The spec gives each set one `use_case` and pinned base rules, but 3.3e specifies one set containing items across use cases. The plan provides no schema extension or grouping mechanism to make that valid. Unlike 3.2b and the remaining dev sets, its acceptance also does not require an image-by-image publication review.

    **Concrete change:** Use five two-item adversarial sets, one per use case, under the existing schema; update matrix inputs, set counts and estimates accordingly. Add the same owner image-review log and construction-derived label acceptance as 3.2b, and require the baseline to report the adversarial results separately.

11. **[minor] Verification still depends on unavailable tools and one malformed command**

    **Section:** Commit strategy, 0.1/0.2, checkpoint review commands.

    **Why:** The commit command omits the separator in `cargo clippy --workspace --all-targets -D warnings`; the documented form passes lint flags after `--`. In the current environment, `just` and `cargo-deny` are absent and the active toolchain lacks Clippy, yet M0 specifies no bootstrap procedure. Conversely, the revised `cargo test -p ... -- image frames` form is valid because libtest accepts multiple filters after `--`. [Clippy usage](https://doc.rust-lang.org/stable/clippy/usage.html), [Rust test filters](https://doc.rust-lang.org/rustc/tests/).

    **Concrete change:** Use `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Add pinned installation/preflight instructions for required developer tools and CI components. Replace `/agent-skills:review` and `/agent-skills:test` with named available workflows, or document their installation and invocation; ensure filtered checks enumerate and actually execute the intended tests.

12. **[minor] Several command and tool contracts still have no measurable completion condition**

    **Section:** 1.3, 1.5b, 3.5a, 4.3c, 4.5, 5.4b.

    **Why:** The shorthand `check --candidate --ref --brief` and specified `eval report` command still have no explicit implementation test. Task 1.5b says three returned regions yield three evidence items with one dropped, contradicting removal before presentation. `change_map`, arm-C publication and `classes` have incomplete acceptance, and metric tests omit exact cost/time totals and medians.

    **Concrete change:** Add a shorthand-versus-job equivalence fixture in 1.3 and regeneration of an evaluation report from saved results in 3.5a. Change 1.5b to “three regions returned, one dropped, two evidence items presented.” Add a golden change-map output, exact expected classes on named fixtures, a complete arm-C metric/identity publication check, and explicit cost/time total and median fixtures.

13. **[minor] Literal secret-prefix scanning can reject the hook and its own fixtures**

    **Section:** 0.2; commit strategy.

    **Why:** The proposed hook greps staged content for literal prefixes that must also appear in its implementation and fake-key tests. A literal implementation therefore rejects legitimate commits, while an indiscriminate fixture exemption could hide real credentials. The acceptance test currently covers only a fake key and an unspecified clean commit.

    **Concrete change:** Define credential-shaped matches and narrowly documented synthetic fixture handling. Require the hook test to accept its own source, documentation examples and approved synthetic fixtures, while rejecting a realistic credential in an ordinary staged file. Keep owner-controlled pushes and the reviewed checkpoint strategy unchanged.

## Before M0 vs during the task

**Change the plan before M0:**

- **1:** Define what the blind-set seal freezes and what exposure checks can actually prove.
- **2:** Define execution identity and crash-safe financial recovery before designing ledger interfaces.
- **3:** Choose one budget-admission policy and make the estimator consistent with it.
- **4:** Resolve package identity and the test-only CLI dependency/feature arrangement.
- **5:** Correct dependency edges, premature CLI scenarios and parallelisation claims.
- **7:** Define initial versus confirmation admission and separate scoring from publication.
- **8:** Add the actual registry-publication path and packaged-resource verification.

**Fix in the first task that touches the issue:**

- **6:** In 1.15, freeze reproducibility details and assign the downstream runner/scoring fixtures.
- **9:** Before implementing 1.10, split its work and tighten the subsequent MCP/provisioning contracts.
- **10:** In 3.3e, use schema-valid adversarial sets and apply the existing publication-review procedure.
- **11:** In 0.1/0.2, bootstrap tools and replace invalid or unavailable verification invocations.
- **12:** Start in 1.3 with shorthand coverage; add the listed checks to their owning tasks before implementation.
- **13:** In 0.2, specify and test the hook’s matching and synthetic-fixture rules.

## Questions for the owner

- Which initial judge models and accounts should receive recordings, and what caps cover recordings, generation, release smoke tests and each evaluation phase?
- Which generators, publication licence and prototype assets are approved, and what adjudication capacity is available?
- Is M6 funded within this release, or should the release explicitly stop at provisional admission with `regions` excluded from arm D?

## What is good

- Keep code as the sole verdict authority and operational errors separate from verdicts.
- Keep the valid walking slice, offline scripted providers and recorded adapter replay before paid evaluations.
- Keep frozen tool definitions and the corrected distinction between provider history and request controls.
- Keep equal visual access for C versus D_regions, external blind labels and honest inconclusive outcomes.
- Keep the pre-M5 alignment/provenance work, owner-reviewed checkpoints and history-preserving rollback.
- Keep the exact task-ID, ordering and tag parity between the plan and todo.