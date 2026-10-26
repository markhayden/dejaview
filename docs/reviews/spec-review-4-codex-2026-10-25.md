**With changes.** Draft 4 fixes much of the earlier design: code-owned instances, per-instance aggregation, location records, separate tool schemas, repair histories, and explicit provisioning are substantial improvements. It is not ready to freeze into a plan unchanged: target scope now conflicts with reference-based comparisons, exemption and evidence-binding gaps remain, and the prescribed final-turn tool restriction conflicts with Anthropic’s documented thinking-history rules. Those contracts need correction first; the remaining boundedness, packaging, statistical, and table-completeness issues can be explicit M1 deliverables. No files were modified.

## Resolution of prior findings

`R3` denotes review 3’s new findings; `R2` and `R1` retain its earlier identifiers.

| # | Finding | Resolution |
|---|---|---|
| R3-1 | Provider schema passthrough is incompatible | **Resolves.** Every adapter now transforms schemas, preserves canonical validation, and checks representability before billing. Gemini’s JSON Schema representation is explicit; exact supported keywords can appropriately be pinned in M2. |
| R3-2 | Exemption scope can waive a larger obligation | **Partly resolves.** Per-target instances prevent cross-image waivers, and architectural SF3 no longer waives structure. Applicability still checks IDs and targets without requiring the exemption’s textual condition or assessment of the unaffected remainder of the same instance. |
| R3-3 | Empty dependencies and combined flags make aggregation non-total | **Partly resolves.** Nonempty dependencies and per-instance evaluation fix the principal aggregation defects. Tool binding still consists of filtering fixed obligation IDs, without a defined way to bind consumer requirements; the aggregation table also omits one legal row. |
| R3-4 | Location variants contradict their validation rules | **Resolves.** The table and schema now consistently require frames, support seam-safe polygons, identify the image for absence, and enforce yaw’s exclusive upper bound. |
| R3-5 | Partial-call repair needs a different history operation | **Resolves.** Excluded responses remain in the journal, reissue starts from valid history, increased allowance is reserved, and final repair remains submission-only. The separate tool-availability incompatibility is New 1 below. |
| R3-6 | `_dejaview` lacks compatible schemas and error handling | **Resolves.** Model and transport schemas are separated, injection follows validation, model-supplied context is rejected, and MCP results have explicit envelope and image-block requirements. M1 should encode the stated error precedence as an exhaustive truth table. |
| R3-7 | MCP cannot receive aligned inputs | **Resolves.** Calls select exact frame IDs and receive lossless pixels and complete transform chains. Finalizing the concrete frame-input schema does not require another architectural decision. |
| R3-8 | Shared reset threatens concurrent and independent runs | **Partly resolves.** Exclusive per-run leases, reset before reuse, unique IDs, and late-response tombstones address isolation. Waiting for quiescence remains unbounded and cancellation does not establish it. |
| R3-9 | Matching manufactures agreement across images or obligations | **Resolves.** Eligibility now requires matching root images and dependencies, both sides have deterministic tie-breaks, and exemption identities are compared. Localization also checks root image and obligation compatibility. |
| R3-10 | Observational rules cannot produce observational findings | **Resolves.** `observed` has valid observational attribution, location and reason, produces only `recorded`, and cannot satisfy mandatory items. |
| R3-11 | Bootstrap and family support are underspecified | **Partly resolves.** Estimand, resampling population, quantiles, seed, and cost statistic are specified. The family gate measures the wrong kind of disagreement, and the Holm instruction is not executable. |
| R3-12 | Provisioning does not install the claimed locked environment | **Partly resolves.** Setup now consumes a project lock and runtime executes directly. The bundle contains a wheel but does not explain how the project and lock select that wheel for installation. |
| R3-13 | Gateway example contains unverified wire fields | **Resolves.** Undocumented fields are removed, provenance is explicitly unresolved, missing provenance fails, and shipping remains conditional. The remaining `only` request field matches [Vercel’s documentation](https://vercel.com/docs/ai-gateway/models-and-providers/provider-filtering-and-ordering). |
| R2-2 | Exemptions permit violation-to-pass paths | **Partly resolves.** Target ownership and removal of SF3’s structural waiver help. Conditional permission still lacks the applicability and remainder-assessment rule described under R3-2. |
| R2-4 | Overflow and deduplication discard obligations | **Partly resolves.** Overflow fails before presentation and merged provenance is retained. Equal dependencies and overlapping geometry still do not prove two claims equivalent; provenance alone does not give each claim a disposition. |
| R2-6 | Repair leaves unanswered provider calls | **Resolves.** Complete rejected calls receive paired results, while incomplete responses are excluded and reissued. |
| R2-10 | Gateway does not enforce the recipient chain | **Resolves.** The request allowlist, provenance validation, consent binding, and shipping gate now form an explicit contract. Live verification remains a release prerequisite rather than an assumed capability. |
| R2-12 | Admission tolerates operational failure | **Partly resolves.** Error ceilings and nonincrease remain enforced, with errors included in cost. The revised statistical gate still needs New 6’s corrections. |
| R2-13 | Runtime `uvx` violates offline operation | **Partly resolves.** Runtime execution no longer resolves or downloads dependencies. The replacement installation contract remains incomplete. |
| R1-1 | Model-supplied IDs bypass aggregation | **Resolves.** Checklist dependencies are code-owned and nonempty; verdict-bearing findings require mandatory attribution at the relevant target. `observed` cannot discharge those dependencies. |
| R1-2 | Unresolved evidence becomes pass; inconsistent rule semantics | **Partly resolves.** Uncertainty is evaluated per instance, including the explicit `hidden_ok` exception. Conditional waiver semantics remain incomplete, and the table omits tentative `held` when `matters=false`. |
| R1-5 | Spend reservation cannot be enforced | **Resolves.** Envelope and image byte bounds now close the identified tool-output sizing gap, alongside the decision-envelope settlement rules. Actual provider token bounds still require the catalog verification already assigned to M2. |
| R1-6 | Run loop is not a bounded state machine | **Partly resolves.** Request, repair, retry, and call limits are substantially defined. Final-turn tool availability is incompatible with a named provider, and shared-process quiescence can wait indefinitely. |
| R1-7 | Checklist coverage exceeds what it proves | **Resolves.** The claim is now explicitly limited to tracked claims; crops, measurements, and summaries are identified as context. It no longer promises that every visible pixel or possible defect was dispositioned. |
| R1-8 | Two-run disagreement and repetitions are undefined | **Resolves.** Static comparison, dynamic eligibility, deterministic pairing, exemption identity, and repetition selection are specified. |
| R1-10 | Blind evaluation and admission gates are absent | **Partly resolves.** Sealing, independent confirmation, frozen policies, and inconclusive outcomes are present. Family support and multiplicity still need an executable definition. |
| R1-11 | Evaluation jobs and scoring are incomplete | **Resolves.** Complete jobs, frozen repetition and policy selection, and image-aware localization—including absence—address the identified gaps. |
| R1-14 | Provider matching and schema contracts are incomplete | **Partly resolves.** Native matching, legacy Gemini restrictions, and wire transformations are explicit. Final-turn tool changes still violate the documented conversation contract for a selected Anthropic model. |
| R1-17 | Manifest lacks an evidence contract | **Partly resolves.** Two argument schemas, bounded envelopes, explicit frames, and provenance are substantial fixes. Fixed `regions_bear_on` IDs still lack a job-specific binding mechanism covering consumer requirements. |
| R1-18 | MCP lifecycle and failure semantics are incomplete | **Partly resolves.** Initialization, capabilities, discovery, result validation, leases, cancellation, and shutdown are specified. Quiescence and process reuse after cancellation remain incomplete. |
| R1-19 | Locations cannot map reliably between frames | **Resolves.** Required frames, root identities, supported geometries, clipping, and absence identity are now consistent. |
| R1-20 | Source-fidelity defaults privilege architecture | **Resolves.** Architecture remains a named profile, and its decorative permission no longer formally waives structural preservation. |
| R1-21 | Cloud consent and retention are underspecified | **Resolves.** Consent identifies the recipient chain, provenance failures terminate, cross-host redirects are prohibited, and retention versus attested account controls is explicit. |
| R1-24 | Tests overclaim conformance and violate offline operation | **Partly resolves.** Offline commands and exact opaque-history replay are improved. Provisioning still needs a runnable bundle contract, and conformance fixtures must include the tool-availability transition in New 1. |

## New findings

Documentation checks confirmed Anthropic’s [result ordering](https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls), [truncation reissue](https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons), and [strict-schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs); Gemini’s [`parametersJsonSchema`](https://ai.google.dev/api/generate-content#FunctionDeclaration), [call IDs](https://ai.google.dev/gemini-api/docs/generate-content/function-calling.md), and [signature preservation](https://ai.google.dev/gemini-api/docs/generate-content/thought-signatures); and OpenAI’s [call-result matching](https://developers.openai.com/api/docs/guides/function-calling) and [stateless reasoning continuity](https://developers.openai.com/api/docs/guides/reasoning). MCP’s [initialization](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle) and [structured-result/error channels](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) also match the revision. These checks establish documented primitives, not live conformance of every repair request or combined schema. I did not verify live image limits, account behavior, Vercel response provenance, `rmcp` compatibility, or a functioning provisioning bundle.

1. **[blocker] Restricting the final tool list invalidates Anthropic thinking history**

   **Section:** “Run state machine,” approximately lines 624–638; “Final turn mechanics,” lines 669–673.

   **Why:** The final request offers only `submit_verdict`, implying replacement of the earlier top-level tool list while replaying thinking blocks. Anthropic documents that Opus 5.5 thinking blocks are bound to the preceding conversation, including `tools`; changing that prefix can return HTTP 400 for accounts subject to enforcement. Preserving the blocks byte-for-byte therefore does not suffice. Anthropic provides append-only tool-removal blocks, but that capability and its required header are absent from this contract. See the [migration rules](https://platform.claude.com/docs/en/models/opus-5-5/migration-guide) and [tool-change protocol](https://platform.claude.com/docs/en/build-with-claude/mid-conversation-system-messages).

   **Concrete change:** Add: “Final-turn restriction means restricting effective tool availability through a cataloged adapter mechanism. For Anthropic models with conversation-bound thinking, preserve the top-level system prompt and tool definitions and append documented `tool_removal` blocks after paired results, using the required header. Final repairs preserve that restriction and obey the resulting message-placement rules. Reject configurations lacking a supported mechanism during resolution.”

2. **[blocker] Scope now confuses the image being judged with the reference used to judge it**

   **Section:** “Job,” approximately lines 184–195; “Instances,” lines 278–282; “Checklist,” lines 368–371; semantic validation, lines 492–497.

   **Why:** RQ1 asks whether Pip’s patch appears correctly in the candidate but targets `ref:pip`. Under the revised rules, a finding locating the missing patch on `raw:candidate` cannot cite RQ1. Moving RQ1 to the candidate then makes an otherwise useful reference illegal unless another mandatory obligation targets the reference itself. Reference images are comparison evidence; they need not themselves be subjects of mandatory checks.

   **Concrete change:** Add: “`scope` identifies images whose correctness is judged. Comparison images are separately declared as `evidence_images` and do not become evaluation targets merely by being consulted. Reference checklist dependencies include instances that explicitly use that reference as evidence; a reference need not have an instance targeting itself.” Change the Pip requirements to target `candidate` and identify `ref:pip` as comparison evidence.

3. **[major] An explicit scope can resolve to no instances and produce a vacuous pass**

   **Section:** “Obligations,” approximately lines 272–282 and 346–351; “Aggregation,” lines 524–526.

   **Why:** Omitted scope means all images, but explicit empty scope and valid roles with no members have no defined rejection rule. A candidate-only job with mandatory scope `["reference"]` can therefore produce no mandatory instances or checklist entries. An empty submission then reaches the documented default `pass`.

   **Concrete change:** Add: “After role expansion and deduplication, every enabled mandatory obligation must have at least one target; otherwise resolution fails. Explicit `scope=[]` is invalid. A check must contain at least one mandatory instance. Nonapplicability must be expressed through explicit configuration, never an empty expansion.”

4. **[major] Cancellation leaves shared-process quiescence unknowable and potentially unbounded**

   **Section:** “Transports,” approximately lines 981–999.

   **Why:** The timeout path tombstones the request and ignores late responses, but the next lease waits for quiescence before sending reset. MCP cancellation is a notification: the server may be unable to cancel, and successful cancellation need not produce a response. Consequently, neither silence nor removing the request from the client’s pending map proves that server work stopped. The reset acknowledgement deadline begins too late to bound this wait. This follows directly from [MCP cancellation semantics](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/cancellation).

   **Concrete change:** Add: “A process with any timed-out or cancelled call is ineligible for reuse and undergoes bounded shutdown before another lease. For normal completion, lease acquisition, quiescence, reset acknowledgement, and any restart share a bounded acquisition deadline. Reset acknowledgement guarantees that prior-run tasks and mutable state are gone; restart repeats initialization and manifest verification.”

5. **[major] The wheel-only provisioning bundle is not a runnable uv project**

   **Section:** “Python tool pack,” approximately lines 1009–1019.

   **Why:** The bundle includes the project metadata, lock, and built wheel, but no source tree or rule making the wheel a dependency. `uv sync` installs the project described by its metadata; it does not automatically substitute an adjacent wheel for that project. With a normal development lock, this attempts to install the missing source project; without a build system, the root project is not installed at all. These behaviors are documented in [uv synchronization](https://docs.astral.sh/uv/concepts/projects/sync/), while wheel sources require an explicit [dependency source](https://docs.astral.sh/uv/concepts/projects/dependencies/#path).

   **Concrete change:** Replace the bundle rule with: “The bundle contains a dedicated provisioning project with `tool.uv.package=false`, an exact dependency on `dejaview-tools`, and a relative `tool.uv.sources` path to the included wheel. Its `uv.lock` is generated for that provisioning project. Setup synchronizes that project using the declared Python version; a clean-directory installation test verifies the executable and installed dependency versions against the bundle.”

6. **[major] The family gate and Holm instruction do not implement the declared detection test**

   **Section:** “Gates,” approximately lines 1197–1209.

   **Why:** The estimand measures changes in detection, but family eligibility counts any outcome difference. An expected-fail item changing from `pass` to `unsure` contributes a “different” family despite having detection indicator zero in both arms; four genuinely informative families plus this fifth family satisfy the stated minimum. Separately, Holm is a procedure over hypothesis-test p-values, not an instruction to adjust already-computed percentile bounds; no p-values or interval-inversion method are defined. See the [official R description of Holm adjustment](https://www.stat.ethz.ch/R-manual/R-devel/library/stats/help/p.adjust.methods.html).

   **Concrete change:** Add: “A family counts toward detection support only if at least one expected-fail item has a different `computed_verdict == fail` indicator between arms.” Replace the unspecified Holm rule with: “For `m` predeclared hypotheses, use Bonferroni-adjusted percentile intervals with endpoints `0.025/m` and `1−0.025/m`; admission requires the adjusted lower endpoint above zero. Insufficient bootstrap tail resolution yields `inconclusive`.” A different method is acceptable, but must specify its test, adjustment, and decision rule completely.

7. **[minor] The allegedly total aggregation table omits a legal case**

   **Section:** “Aggregation,” approximately lines 510–524; Rust example, lines 1313–1320.

   **Why:** `held`, `tentative`, `matters=false` matches no table row. The Rust example records it through the catch-all, so two purported specifications of the same function differ in completeness.

   **Concrete change:** Add the row: “`held` | `tentative` | `not matters` | `recorded`.” Require the table and executable function to cover the same complete disposition/confidence/flag domain.

8. **[minor] Empty-waiver permissions make the observational reporting instruction contradictory**

   **Section:** Architectural profile, approximately lines 310–312 and 341–344.

   **Why:** `waives=[]` correctly grants no mandatory waiver, and `observed` correctly requires observational attribution. However, SF4 says other changes are noted, while the explanatory text expressly permits furniture changes to go unreported. The revision therefore leaves the model two incompatible instructions about the same harmless change.

   **Concrete change:** Replace “or not at all” with: “When SF4 is enabled, report such changes as `observed` findings citing SF4. An exemption with `waives=[]` supplies permission context only; it neither satisfies a mandatory item nor overrides an enabled observation instruction.”

## Before planning vs during M1

**Before planning:**

- **New 1:** Separate effective tool availability from immutable provider declarations; specify Anthropic’s supported final-turn mechanism.
- **New 2:** Separate evaluation targets from comparison evidence before freezing instance, checklist, and finding types.
- **R3-2 / R2-2:** Require applicability of an exemption’s textual condition and assessment of the protected remainder; matching an ID and image is insufficient.
- **R2-4:** Preserve separate dispositions for distinct claims; geometry-only deduplication must not merge their obligations.
- **R3-3 / R1-17:** Define trusted, job-specific tool-to-obligation bindings, including requirements, before freezing the manifest/config interface.

**During M1:**

- **New 3:** Reject empty mandatory expansions and checks without mandatory instances through resolver validation.
- **New 4:** Specify and test lease deadlines, cancellation-driven restart, and reset guarantees with the fake MCP server.
- **New 5:** Build a minimal provisioning bundle and prove clean installation; this preserves the Rust/Python packaging architecture.
- **New 6:** Freeze the exact family predicate and multiplicity algorithm before implementing gate fixtures or collecting admission data.
- **New 7:** Complete the aggregation table and exhaustive row tests.
- **New 8:** Make observational reporting prose consistent; no schema change is necessary.

## Questions for the owner

- Is the documented Anthropic tool-removal beta acceptable for the first release, or should affected model configurations wait?
- Confirm whether architectural `hidden_ok` permits **any** occlusion to avoid `unsure`, regardless of its cause.
- Confirm `on_unsure=pass`, `on_error=review`, admission margins, and the budget for independently supported admission and confirmation sets.
- Confirm indefinite local retention and trusted, unsandboxed tool packs as the first-release operating model.

## What is good

- Keep the model as judge, tools as evidence, and code as the sole verdict authority.
- Keep code-owned instances, nonempty dependencies, and per-instance aggregation.
- Keep the corrected location table and schema, including image-specific absence.
- Keep separate journal and provider histories, paired rejected-call results, and reserved reissue allowance.
- Keep separate model/transport schemas and explicit lossless frame delivery to MCP tools.
- Keep complete-decision caching, equal-visual-access controls, sealed labels, independent confirmation, and an honest `inconclusive` outcome.