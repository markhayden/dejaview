**With changes.** Draft 3 resolves most of review 2, including the seal, execution descriptor, crash-safe ledger, budget policy, and staged admission contracts. Before M0, correct the testkit publication/checking arrangement and the remaining task-order contradictions; the narrower verification fixes can be completed in their owning tasks. `tasks/todo.md` matches all **74 task IDs, their order, and all 13 checkpoint/intermediate tags**. No files were modified.

## Resolution of review 2

| # | Finding | Resolution |
|---|---|---|
| 1 | Seal does not protect the complete experiment | **Resolves.** Task 3.1 binds membership, families, canonical jobs, images, rules and external labels, with mutation tests. Task 5.5 correctly distinguishes owner attestation from supporting journal searches. |
| 2 | Incomplete resume identity and crash recovery | **Resolves.** Task 3.4a matches the edited spec: resolved execution descriptor, single writer, durable holds before sends, conservative recovery, settlement at most once and retained errors. |
| 3 | Estimator contradicts per-decision admission | **Resolves.** Task 3.4c now warns rather than refuses and compares individual estimates with individual reservations. Its acceptance explicitly permits completion when the envelope sum exceeds the cap. |
| 4 | Package identity and fake-provider build path contradict each other | **Resolves.** CLI selectors consistently use `-p squint-cli`, and the optional dependency/non-default feature is explicit. That choice introduces a publication dependency addressed below. |
| 5 | Impossible execution orders and inconsistent dependencies | **Partly resolves.** Adapter recording dependencies, checklist prerequisites and the principal M1 scenarios are corrected. The graph still contradicts task lists, and the revised admission, packaging and estimator tasks retain ordering problems. |
| 6 | Admission rules lack reproducible verification | **Resolves.** Tasks 1.15, 3.4b and 3.5a/b now assign reproducibility fixtures, arm construction, repetition-1 scoring and gate outcome tables. |
| 7 | Confirmation lacks a distinct state; scoring commits publication | **Resolves.** Initial and confirmation stages, linked receipts, configuration equality, disjoint confirmation families and confirmed-only arm D now match the spec. Publication produces reviewable files and never commits automatically. |
| 8 | Release checks do not establish registry installation | **Partly resolves.** Packaging and distribution are separated, with packaged-source and architecture checks. The publication dependency closure, first-publication dry run and explicit final registry installation remain incomplete. |
| 9 | MCP/provisioning proofs omit difficult cases | **Resolves.** The shared lifecycle deadline, reset/quiescence, replacement after cancellation, externally denied networking and guarded provisioning are explicit. Journal/report and geometry/viewer work are split. |
| 10 | Adversarial set violates the approved schema | **Resolves.** Task 3.3e now creates five schema-valid sets and inherits image review and construction-derived labels; the baseline reports them separately. |
| 11 | Verification relies on unavailable tools or malformed commands | **Partly resolves.** Clippy syntax and the review-workflow fallback are corrected; the displayed multiple-filter commands are valid. `just bootstrap` still assumes `just` is already installed, and filtered checks do not require nonzero execution. |
| 12 | Missing measurable command/tool completion conditions | **Resolves.** Shorthand equivalence, report regeneration, two presented evidence items, change-map goldens, class fixtures and exact metric totals/medians are assigned. |
| 13 | Secret-prefix scanner rejects legitimate source and fixtures | **Partly resolves.** Credential-shaped patterns address the literal-prefix problem. Whole-file exemptions for the hook, its test and the synthetic-fixture directory are broader than the requested narrowly controlled synthetic handling. |

## New findings

1. **[major] The optional testkit dependency is missing from publication**

   **Section:** Crate dependency graph; 0.1; 2.10a.

   **Why:** The CLI now has a normal optional dependency on `squint-testkit`, but the publication sequence omits that package. Disabling the feature does not remove the dependency from Cargo’s package-resolution obligations. Separately, successive per-crate `cargo publish --dry-run` calls do not upload preceding dependencies, so ordering those calls does not establish a working first publication. [Cargo dependency resolution](https://doc.rust-lang.org/cargo/reference/resolver.html#features), [Cargo publish](https://doc.rust-lang.org/cargo/commands/cargo-publish.html).

   **Concrete change:** Add `squint-testkit` after `core` in the publication dependency order and check ownership/availability of every published package name in 0.1. Replace independent per-crate dry runs with a workspace publication dry run supported by the pinned Cargo version; stage packaged dependencies in a temporary registry for the clean installation test. Add a final acceptance step after owner publication: “From outside the repository, `cargo install squint-cli --version <released-version> --locked` succeeds and passes the ordinary-release smoke.”

2. **[major] The mandatory checks do not explicitly run the feature-gated CLI suite**

   **Section:** Crate graph; commit strategy; 0.1–0.2; checkpoint M1.

   **Why:** Individual CLI verification commands enable `testkit`, but the mandatory commit/checkpoint command remains `cargo test --workspace`; `just check-scripted` is mentioned without being made part of `just check`. Default tests can consequently skip feature-required targets—or fail if those tests unconditionally expect scripted flags. A feature-enabled walking-slice executable also cannot itself prove feature-off rejection. [Cargo feature selection](https://doc.rust-lang.org/cargo/commands/cargo-test.html#feature-selection), [Required features on targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-required-features-field).

   **Concrete change:** Require `just check` and CI to run both `cargo test --workspace` and `cargo test --workspace --features squint-cli/testkit`, plus feature-enabled Clippy. Build the feature-off release executable separately and test flag rejection against that executable. Declare permitted dev-dependency edges separately from production edges, check the graph with all features enabled, and require named filtered suites to enumerate and execute at least one test.

3. **[major] The runner consumes an admission contract defined by its own downstream task**

   **Section:** 3.4a, 3.4b, 3.5a, 3.5b.

   **Why:** Task 3.4b must construct D from the admission record, but `admissions.rs` and that record’s round-trip contract belong to 3.5b. Task 3.5b already depends transitively on 3.4b through metrics, so adding the missing dependency would create a cycle. The stage semantics are sound; their implementation ownership is ordered incorrectly.

   **Concrete change:** Move the versioned admission-record type, receipt-reference type, reader and synthetic records into 3.4a. Keep scoring and admission-record writes in 3.5b. Require 3.4b to test confirmed-only membership against those fixtures, and 3.5b to test initial-stage rejection of non-blind or fewer-than-20-item sets, alongside confirmation rejection for insufficient items, overlapping families and changed judging configuration.

4. **[major] Packaging checks precede the artifacts they claim to verify**

   **Section:** 2.10a; 3.6; 5.2–5.4b; 5.6; checkpoint M5.

   **Why:** Task 2.10a requires packaged viewer assets, although the viewer is not implemented until 3.6 and cross-milestone code work is prohibited. Similarly, 5.2 verifies the “real pack” before segmenter fetching, `regions` and `classes` exist. Task 5.6 does not depend on 5.2, so its declared prerequisites allow a gate without a verified installable bundle containing the tested tool.

   **Concrete change:** Limit M2 packaging acceptance to assets implemented by M2, and add the viewer packaging check to 3.6. Move completion of 5.2 after 5.3, 5.4a and 5.4b, and add 5.2 to 5.6’s dependencies. At M5, rebuild the final Rust packages, platform archives and Python bundle from the final sources before rerunning their installation checks; mirror the changed order in todo.

5. **[major] Arm-C publication can compare against the wrong arm-A execution**

   **Section:** 4.5.

   **Why:** Task 4.5 runs a new matrix containing A and C but asks for paired flips “against” the earlier baseline. That baseline predates the M4 implementation and has a different harness identity; it is not the A cell of the new experiment. This wording allows publication to attribute differences between executions to visual access.

   **Concrete change:** Replace the comparison instruction with: “Compute paired flips between repetition 1 of A and C within this matrix, matching item, model and judging settings; verify that answer-affecting differences are limited to the declared arm. Show the M3 baseline separately as historical context.”

6. **[minor] The revised dependency graph still describes different schedules from the task lists**

   **Section:** Task dependency graph; 1.4b; 3.4c; Parallelisation.

   **Why:** Task 1.4b’s Pip CLI scenario requires the base64 image support in 1.2, but its dependency chain does not include 1.2. Task 3.4c’s new fake-run acceptance requires the runner in 3.4b, while its list names only 3.4a. The diagram also asserts absent dependencies such as `2.9 → 2.10a`, `3.6 → 3.7` and `4.5 → 4.6`, despite being labelled a dependency graph.

   **Concrete change:** Add 1.2 to 1.4b and 3.4b to 3.4c. Regenerate the graph from the corrected dependency lists, showing milestone barriers separately from task edges. State the MCP sequence explicitly as `1.12 → 1.13 → 1.14`, and replace 1.7b’s “every error class” criterion with “every error class implemented by this task”; complete coverage follows the later repair, ledger and adapter tasks.

7. **[minor] The fake MCP executable has no reproducible build-and-location contract**

   **Section:** 1.13; 4.4b.

   **Why:** The CLI scenario launches `squint-testkit-mcp`, while the listed source path `src/bin/mcp.rs` would ordinarily produce a binary named `mcp` unless its target name is configured. `cargo test -p squint-tools` does not establish that a separate dependency package’s executable is built and available on `PATH`. This makes the advertised standalone verification dependent on an unexplained previous build. [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html).

   **Concrete change:** Declare `[[bin]] name = "squint-testkit-mcp"` with `path = "src/bin/mcp.rs"`. Add a test helper that builds that target, obtains its absolute executable path from Cargo’s JSON output and supplies it to the MCP fixtures. Invoke that helper from the standalone MCP verification and standard checks.

8. **[minor] Scanner exemptions now permit arbitrary credentials in exempted files**

   **Section:** 0.2.

   **Why:** Exempting entire files and a directory fixes self-rejection by also exempting unrelated credential-shaped additions in those locations. Full-shape regexes do not themselves match the regex source, so exempting the hook’s entire source is unnecessary. The acceptance test only checks rejection in an ordinary file and would miss this regression.

   **Concrete change:** Remove whole-file exemptions. Allow only enumerated synthetic token values at documented fixture locations, and add a test proving that a different realistic-shaped token is rejected even inside the hook test or synthetic-fixture directory.

## Before M0 vs during the task

**Change before M0:**

- **1:** Close the publication dependency graph and define a viable first-publication verification path.
- **2:** Make both feature configurations mandatory in scaffold checks and CI.
- **3:** Establish the admission contract before its runner consumer.
- **4:** Correct packaging/bundle ordering and final-artifact verification.
- **6:** Reconcile dependency lists, the diagram and premature acceptance scenarios.

**Fix in the first task that touches the issue:**

- **5:** In 4.5, pair C with A from the same matrix execution.
- **7:** In 1.13, implement explicit fake-server build and executable discovery.
- **8:** In 0.2, replace file exemptions with exact synthetic-token handling.
- **Review 2 #11:** In 0.1, document installation of pinned `just` before invoking `just bootstrap`.

## Questions for the owner

- Which initial models/accounts and spending caps should 0.4 record for recordings, generation, release smoke and evaluations?
- Which generators, publication licence and prototype assets are approved, and what adjudication capacity is available?
- Is M6 funded for this release, or does release stop at provisional admission with `regions` excluded from D?

## What is good

- Keep the complete seal and the honest distinction between attestation and supporting evidence.
- Keep the execution descriptor, single-writer exclusion and conservative crash recovery.
- Keep per-decision budget admission and the estimator’s warning-only policy.
- Keep explicit initial/confirmation stages, reviewable publication files and confirmed-only arm D.
- Keep the corrected M1 walking slices, frozen provider history and separate repair forms.
- Keep exact plan/todo ID, order and tag parity.