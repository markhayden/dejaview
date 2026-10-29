**With changes.** The milestone sequence and testing discipline are sound, but the plan needs another revision before M0 starts. It defers three explicit M1 obligations, leaves early tasks dependent on later infrastructure, omits release packaging, and does not fully protect blind evaluation or resumed-run spending. The proposed task splits also need to become executable S/M tasks with their own dependencies and acceptance criteria. These are planning corrections, not reasons to reopen the approved architecture.

## Findings

1. **[blocker] The blind-set workflow permits contamination before sealing**

   **Section:** Tasks 3.1, 3.2, 5.5; “Parallelisation.”

   **Why:** Task 5.5 requires labelling before any `regions` output exists, whereas the spec requires labels before any judge-model output exists. Its timestamp check covers matrix runs only; an earlier `check`, exploratory adapter run, or label-drafting helper could already have exposed predictions. Moving labels outside the tree also does not remove labels previously committed to Git history.

   **Concrete change:** Replace the ordering rule with: “Construct and owner-adjudicate blind labels without judge-model or measurement-tool output on these items. Keep labels in owner-controlled storage outside both repositories from creation; commit only their hash. Seal membership, jobs, image hashes, rules versions and labels hash before any evaluation or exploratory check. Generation records and construction-derived draft labels are permitted; prediction-derived labels are not.” Add tests that sealing preserves the external labels and that public manifests and jobs contain no answer labels.

2. **[major] Three mandatory M1 deliverables are deferred or missing**

   **Section:** Checkpoint M1; Tasks 3.5, 4.4, 5.2.

   **Why:** Review 4 explicitly assigns lease/reset/cancellation tests, a minimal clean-install provisioning proof, and freezing the statistical algorithm to M1. The plan puts the first two in M4/M5 and has no M1 statistical-contract task. Empty-expansion rejection and exhaustive aggregation are scheduled correctly; observational consistency is already fixed in draft 5 but should be preserved by prompt fixtures.

   **Concrete change:** Add three M1 tasks: “Prove bounded lease acquisition, cancellation-driven restart and reset guarantees against a fake MCP server”; “Build a minimal wheel provisioning bundle and verify clean installation and locked dependency versions”; and “Freeze the detection-disagreement family predicate, type-7 quantiles, seed handling, Bonferroni endpoints and insufficient-tail rule in executable fixtures.” Production MCP integration and full provisioning can remain later. Add an architectural-profile rendering fixture covering `waives=[]`, SF4 reporting and exemption assertions.

3. **[major] M1 is not yet a sequence of working vertical slices**

   **Section:** Dependency graph; Tasks 1.1–1.12 and 4.1.

   **Why:** Task 1.1 builds a substantial horizontal layer before `check` works, and 1.2 deliberately produces a vacuous pass that the approved spec forbids. Tasks 1.5 and 1.7 require journal events before 1.10; 1.7 needs manifests before 4.1; 1.12 requires pack-environment enforcement before 4.4. The plan leaves an agent to invent temporary interfaces and decide when each task becomes observable through the binary.

   **Concrete change:** Start M1 with a minimal valid candidate, one mandatory instance, one disposition, aggregation and a durable journal/report through `check`; tag that slice instead of the empty-checklist pass. Move minimal manifest/envelope contracts and the environment-filter function before their consumers, and introduce the journal sink before 1.5. Expand image handling, rules, evidence and state transitions in subsequent slices. Require every M1 task to name a CLI scenario demonstrating its new behavior, and declare an acyclic crate dependency graph before assigning shared types.

4. **[major] There is no task that produces the promised installable release**

   **Section:** M0, Checkpoint M5.

   **Why:** Checking success criteria at M5 cannot substitute for building their deliverables. Nothing creates the `cargo install squint-cli` package, release binaries for the three target platforms, or a release workflow. The planned CLI crate name also needs an explicit relationship to the package users install.

   **Concrete change:** Add S/M tasks for Cargo package naming and publishability, release builds for macOS arm64 and Linux x86_64/arm64, and clean-install verification. Acceptance: “A clean installation exposes `squint`; one shipped example config and one environment variable produce a report through each direct profile.” Schedule authorized package publication explicitly, and rerun the complete offline suite under the five-minute limit after all adapter fixtures and tools exist.

5. **[major] Gate scoring lacks the complete seal-to-confirmation lifecycle**

   **Section:** Tasks 3.5, 5.5, 5.6.

   **Why:** The scoring task describes statistical calculations but not verification that supplied labels match the seal or that results match the frozen matrix. Task 5.6 omits publishing labels and converting the scored blind set to heldout. It also records admission without stating that a 20-item result remains provisional, and never schedules the distinct untouched confirmation set.

   **Concrete change:** Add acceptance criteria: “Reject a label-hash mismatch, result/configuration mismatch, or repeat scoring without a new ADR; preserve a scoring receipt identifying the seal, matrix and results.” After scoring, publish the labels and update the split atomically with the result record. Mark admission `provisional`, and schedule a separate untouched heldout set of at least 50 items plus confirmation; that task may be explicitly post-M5, but confirmed admission must remain unavailable until it passes.

6. **[major] Privacy enforcement comes after the first live calls**

   **Section:** Tasks 1.3, 2.4–2.8.

   **Why:** Task 1.3 checks the cloud-call boolean, but profile/account/upstream consent binding and recipient-chain reporting appear only in 2.8, after live recording and smoke tests. The plan has no explicit implementation criterion for rejecting cross-host redirects or missing credentials. Adapter fixtures also do not explicitly satisfy the separate requirement for a recorded conversation for every shipped direct profile.

   **Concrete change:** Split 2.8 into pre-live enforcement and final documentation. Make live tasks depend on enforcement of profile/account/upstream consent, `error: auth`, redirect restrictions, recipient-chain recording and secret redaction across all output paths. Require one identified profile-conformance recording for each shipped direct profile; adapter recordings may serve both purposes if that coverage is explicit.

7. **[major] Two acceptance criteria contradict the repair and history contracts**

   **Section:** Tasks 1.7 and 1.8; preserved-thinking risk mitigation.

   **Why:** Append-only provider history does not mean every new request contains the previous request’s entire JSON body as a prefix. Request-level output allowances and tool-choice controls can change while history remains intact. Likewise, refusal and call-less-text reissues append an instruction; only truncation reissues are described as changing the output allowance alone.

   **Concrete change:** Replace the prefix criterion with: “Previously accepted provider-history blocks, opaque payloads, bound system prompt and frozen tool definitions remain unchanged; new replayable content is appended in provider-valid order. Request-level controls change only as the catalog permits.” Split reissue assertions by response class: truncation raises the allowance; refusal/text append the discarded-output instruction; excluded responses never enter history.

8. **[major] L tasks remain unresolved, and several M labels hide multiple deliverables**

   **Section:** Tasks 1.4, 1.6, 1.7, 2.4–2.6, 3.2–3.3, 3.5, 4.3–4.4, 5.4; `tasks/todo.md`.

   **Why:** “Split if it runs long” is not a usable implementation plan. The checklist gives some subtasks names, but they lack independent criteria and dependencies; Gemini and OpenAI remain unsplit altogether. Four datasets in 3.3, statistics plus all gate logic in 3.5, and spherical alignment plus two other tools in 4.3b are still too broad for isolated review and rollback.

   **Concrete change:** Replace each L parent with complete S/M task entries. Split 3.2 into generation procedure/toolkit and first-set adjudication/publication; split 3.3 by use case; split 3.5 into deterministic metrics/localisation, bootstrap/multiplicity, and gate lifecycle/reporting. Separate spherical alignment, `edge_similarity` and `change_map`; separate each adapter’s implementation from recording. Give every resulting task its own dependency list, observable acceptance criteria and matching checklist entry.

9. **[major] Resumability can reset the evaluation’s financial history**

   **Section:** Task 3.4; “Evaluation budget.”

   **Why:** Skipping completed items by run ID does not explain how resumed execution retains prior charges, unknown-usage charges or in-flight reservations. A restarted process could create a fresh ledger and spend the cap again. The plan also conflicts internally: 3.4 stops partway through a matrix, while the budget section says the entire envelope sum must fit before starting.

   **Concrete change:** Add a durable evaluation ledger and immutable matrix identity before enabling resume. Acceptance: “Restart after interruption preserves charged and conservatively charged attempts; completed errors remain recorded; partial work has an explicit recovery state; changed matrices cannot resume under the same identity; cumulative spending across restarts never exceeds the cap.” Align the budget section with per-decision admission under the shared ledger, or explicitly define whole-matrix reservation.

10. **[major] Owner preparation and spending decisions are scheduled too narrowly**

    **Section:** Tasks 2.4–2.7, 3.2–3.3, 3.8, 4.5, 5.5–5.6; “Evaluation budget”; owner questions.

    **Why:** The plan asks for caps for three evaluation runs but also spends money on fixture recording, smoke tests and image generation. Its example matrix contains 1,200 item decisions before tool turns or retries, not merely several hundred model requests. Labelling is acknowledged as a risk without an advance handoff, and the “three models” estimate is not reconciled with the larger candidate catalog.

    **Concrete change:** Add an owner-readiness task at M0 defining the initial model subset, caps and approval points for recording, generation, baseline, control, admission and confirmation. Before M3, deliver the labelling worksheet and schedule review batches for 100 dev items plus the blind set; identify confirmation work separately. Add a pre-run estimate showing decisions, request bounds and expected cost range, and freeze the selected models in each matrix.

11. **[major] Rollback covers unfinished branches but not completed milestones or external state**

    **Section:** “Commit strategy,” “Rollback,” “Parallelisation.”

    **Why:** Deleting a branch restores nothing after its milestone has merged. Reverting a task with dependent successors can also break the promised green history, and hard reset has no clean-tree or preservation prerequisite. Dataset publication, released artifacts, spent money and exposed blind labels cannot be rolled back with code.

    **Concrete change:** Add: “Revert dependent task commits together in reverse dependency order and verify before committing. Before hard reset or branch deletion, require a clean tree and create a recovery reference. For a merged milestone, use a reviewed revert of its merge commit on `main`, preserving history.” Record external artifact versions at checkpoints; never reuse a contaminated blind set. Add an initial documentation commit containing the approved spec, plan, ADRs and checklist while excluding the brief, and explicitly commit `Cargo.lock` for reproducible binary builds.

12. **[major] The plan postpones feasibility and provenance checks until they block M5**

    **Section:** Tasks 4.3, 5.3–5.4; “Risks and mitigations.”

    **Why:** The spec requires `MODEL.md` before M5 begins, but task 5.3 runs after the milestone has started. Alignment assumes an implementation and suitable dependencies without a bounded feasibility task, despite being critical to both the control and region finder. Existing golden views do not establish robustness on unrelated scenes, low-texture images or degenerate transforms.

    **Concrete change:** Move artifact licence/provenance, URL/hash, runtime/provider selection and parity-tolerance documentation to an M4 exit prerequisite. Add a bounded alignment spike before production implementation, with named numerical tolerances, failure fixtures and any required dependency ADR. Add risks for unavailable or incompatible model artifacts, alignment feasibility, and insufficient independent families; mitigation for weak statistical support must remain an honest `inconclusive`, not post-result set expansion.

13. **[minor] Optional dependencies and parallel branches are inconsistent**

    **Section:** Tasks 2.8, 3.8; “Parallelisation”; “Branches.”

    **Why:** Task 2.7 is declared nonblocking, yet 2.8 depends on it and 3.8 depends on all of `2.x`. The parallelisation section permits 3.2 and 3.3 together even though 3.3 depends on the completed first-set procedure. Starting later milestones before M2 merges also needs a branch-base strategy under the “one branch per milestone” policy.

    **Concrete change:** Make 2.8 and 3.8 depend on the three direct profiles and any gateway actually selected for shipping. Allow remaining-set work only after the first-set procedure is accepted. State whether overlapping work uses stacked milestone branches or separate task branches, and how prerequisite commits are incorporated before checkpoint review.

14. **[minor] Several verification instructions cannot establish their claimed result**

    **Section:** Tasks 0.2, 1.1, 1.6, 1.8, 3.5–3.6, 4.2–4.3, 5.2.

    **Why:** Commands such as `cargo test -p squint-core job:: frames::` supply multiple positional filters rather than an executable verification command. A bogus proxy does not prove networking is disabled, and “within tolerance,” “near 0” and “last digit” leave numerical acceptance undefined. The viewer’s localhost-serving option conflicts with an absolute “no network requests” criterion, while an excluded brief normally does not appear as untracked in ordinary status output.

    **Concrete change:** Use valid separate test commands or a single encompassing filter. Pin numeric tolerances and deterministic bootstrap behavior; assert cost/time totals and medians separately from proportion intervals. Verify runtime offline behavior with enforced network denial, and perform downloads only through explicit provisioning steps. Specify “no external requests; loopback static serving permitted” for the viewer, and verify brief exclusion with ignore/tracked-file checks rather than ordinary status.

## Coverage gaps

- **Decisions §1; Success criterion 1 →** Cargo publication/install path, three-platform release binaries, release workflow and clean-install smoke tasks.
- **Commands →** `squint tools list --config …`; explicit coverage of the shorthand `check --candidate --ref --brief` invocation.
- **Report and journal →** SIGTERM cancellation handling and verification; 1.10 covers SIGINT only.
- **Report viewer; Success criterion 6 →** Equirectangular/raw-coordinate overlay implementation and verification. Task 3.6 defers it to M4, but no M4 task owns it.
- **Project structure / boundary schemas →** Generated committed report schema and a check that every boundary type has the required versioning, closed-field behavior and schema derivation.
- **Endpoint profiles / Data leaving the machine →** Explicit cross-host redirect rejection, missing-credential handling, consent binding, and direct-profile conformance recordings.
- **Testing strategy: Isolation →** A live adversarial evaluation set measuring model susceptibility; 1.12 supplies harness-boundary fixtures only.
- **Evaluation: Gates →** Seal verification at scoring, repeat-scoring protection, label publication/split transition, provisional admission and independent ≥50-item confirmation.
- **Drift →** Full development-set requalification after disabling; baseline identity and reset behavior for consecutive canary deviations also need explicit criteria.
- **Python tool pack →** The pre-M5 `MODEL.md` prerequisite is scheduled too late.
- **Review 4, “During M1” →** M1 lease/reset/cancellation proof, minimal provisioning clean-install proof, and statistical-contract freeze.

## Questions for the owner

- Which initial judge models should receive live fixtures and enter the evaluation matrices, and what caps apply to recording, image generation and each evaluation phase?
- Which generators and publication licence should the public sets use, and which prototype images, outputs and model artifacts are approved for reuse?
- What labelling/review capacity is available for the dev and blind sets, and should independent confirmation be funded within this release or scheduled afterward with admission explicitly provisional?

## What is good

- Keep code as the sole verdict authority, with operational errors preserved and consumer policy affecting exit codes only.
- Keep the fake-provider skeleton and offline fixture replay before paid evaluation.
- Keep arm A before tools, and C versus D_regions with equal visual access.
- Keep the separate journal/provider histories, decision-boundary cache and conservative spending reservations.
- Keep reviewed milestone checkpoints, addressable task commits and owner-controlled pushes.
- Keep gateway shipping conditional on conformance, and preserve `inconclusive` as a valid evaluation outcome.