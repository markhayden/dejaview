**No—draft 3 is not yet ready to plan against.** It fixes the decision-cache boundary, reservation accounting, mandatory-requirement selection, signature preservation, and much of the evaluation control. However, executable contracts still conflict: Anthropic cannot accept the prescribed strict schema unchanged; exemption scope and checklist dependencies remain incomplete; locations contradict their validation rules; and partial-call repair, MCP integration, provisioning, and statistical gates still require implementation-time design decisions. The official-documentation checks below confirm several underlying protocols, but do not establish live conformance of the proposed integrations. No files were modified.

## Resolution of prior findings

`R2` identifies review 2’s new findings; `R1` identifies the requested partially resolved findings from review 1.

| # | Finding | Resolution |
|---|---|---|
| R2-1 | Requirement enforcement depends on `matters` | **Resolves.** Mandatory kinds now receive checklist items regardless of `matters`, and requirements share the rules’ exemption semantics. |
| R2-2 | Exemptions permit violation-to-pass paths | **Partly resolves.** Explicit `waives`, mandatory attribution, and tentative-exemption uncertainty close several paths. Scope coverage remains insufficiently defined, and architectural SF3 still formally waives SF1 despite expressly prohibiting structural changes. |
| R2-3 | Submission schema incompatible with OpenAI strict mode | **Resolves.** Objects are closed, properties are required, nullable fields represent optional values, and a provider transformation preserves unsupported constraints in local validation. These structural requirements match the [official OpenAI documentation](https://developers.openai.com/api/docs/guides/structured-outputs); the separate location-semantics defect remains below. |
| R2-4 | Evidence overflow and deduplication discard obligations | **Partly resolves.** Overflow now terminates before presentation, and ordering is deterministic. Identical dependencies plus overlapping geometry still do not establish semantic equivalence, and retaining every merged item’s provenance is not required. |
| R2-5 | Promised MCP versions have incompatible lifecycles | **Resolves.** First-release support is pinned to 2025-11-25, whose documented lifecycle uses `initialize` followed by `notifications/initialized`. See the [versioned MCP lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle). |
| R2-6 | Repair leaves unanswered provider tool calls | **Partly resolves.** Complete rejected calls receive paired error results, and terminal status precedes dispatch. Incomplete call blocks still have no provider-valid continuation rule compatible with unconditional append-only history. |
| R2-7 | External tools receive degraded provider images | **Resolves.** Both transports now receive canonical, full-resolution, lossless images; provider preparation is separate. Delivery of derived frames to MCP tools remains a separate gap. |
| R2-8 | Whole-run and per-attempt reservations double-count spending | **Resolves.** Attempts transfer funds from one decision envelope; known usage settles, unknown usage consumes its bound, and unused funds are released. This resolves the accounting relationship, although completeness of the bounds remains an R1-5 concern. |
| R2-9 | Independent decision runs share cached results | **Resolves.** Caching is exclusively at the complete-decision boundary, with constituent-run cache access disabled. |
| R2-10 | Gateway profile does not enforce recipient chain | **Partly resolves.** Vercel’s `only` restriction is now specified and consent includes the allowlist. The fallback field and provenance header remain unverified, and missing provenance has no explicit runtime outcome. |
| R2-11 | Visual-access control exposes alignment diagnostics | **Resolves.** The projection removes measurements, regions, quality fields, and diagnostic summaries before model presentation and checklist growth. C and D_X share it; alignment availability and geometry are appropriately part of their common visual access. |
| R2-12 | Admission gates tolerate operational failure | **Partly resolves.** Frozen error policy, a 5% ceiling, and nonincrease versus C close the principal admission loophole. The bootstrap estimator, interval construction, minimum independent-family support, and cost statistic are still unspecified. |
| R2-13 | Runtime `uvx` violates no-download operation | **Partly resolves.** Direct execution of a provisioned environment removes runtime dependency resolution. The prescribed setup command does not explain how `uv.lock` is consumed or how the promised version-specific installation is produced. |
| R2-14 | Fixture normalization removes conversation signatures | **Resolves.** Only transport authentication fields are excluded; thought signatures, signed Parts, and opaque reasoning payloads must compare exactly. |
| R2-15 | Async trait conflicts with boxed trait objects | **Resolves.** `send` now returns a boxed `Send` future with an explicit lifetime, consistent with the declared dynamic dispatch. |
| R2-16 | Canary disabling contradicts owner approval | **Resolves.** Both canaries compare against the same baseline, deviations produce a recommendation, and disabling requires approval. |
| R1-1 | Model-supplied rule IDs bypass aggregation | **Partly resolves.** Checklist dependencies are immutable and violation findings require mandatory attribution. Reference and evidence items can nevertheless acquire empty dependency sets, without corresponding item validation. |
| R1-2 | Unresolved evidence becomes pass; inconsistent rule semantics | **Partly resolves.** Most uncertainty rows and the intentional `hidden_ok` exception are explicit. Exemption scope remains incomplete, and combining obligation flags before evaluating rows can contradict “take the strictest row.” |
| R1-3 | Cache returns a verdict for another job | **Resolves.** The key now includes the resolved job, prepared-byte hashes, preprocessing identity, harness identity, endpoint profile, tool locks, settings, and budgets, after preparation. |
| R1-5 | Spend-cap reservation is not enforceable | **Partly resolves.** Envelope transfers and settlement are coherent. The manifest example still lacks total serialized-result and encoded-image byte bounds, despite claiming every field is bounded; labels, transform parameters, and other strings therefore lack an executable reservation bound. |
| R1-6 | Run loop is not a bounded state machine | **Partly resolves.** Turns, retries, batches, and final submission are substantially defined. Partial-call repair and the allowed tools/state transition after a repaired turn remain insufficiently specified. |
| R1-7 | Checklist does not cover everything shown | **Partly resolves.** Mandatory requirements and bounded regions are tracked. Crops, measurements, and summaries are expressly excluded, so the claim that everything shown received a disposition still exceeds what the checklist proves. |
| R1-8 | Two-run disagreement and repetitions undefined | **Partly resolves.** Repetitions and cache independence are fixed. Dynamic matching lacks image/dependency eligibility checks and a complete tie-break rule, so unrelated evidence can count as agreement. |
| R1-9 | Arms cannot establish individual-tool benefit | **Resolves.** D_X adds one tool to the common visual-access control, and bundle claims are limited to bundles. The revised alignment projection addresses the prior diagnostic leakage. |
| R1-10 | Blind evaluation and admission gates absent | **Partly resolves.** Sealed initial admission, separate confirmation, and an inconclusive outcome are now required. Class/family adequacy, executable interval construction, and multiplicity across tested configurations remain open. |
| R1-11 | Evaluation jobs and scoring incomplete | **Partly resolves.** Complete jobs, frozen policies, repetition selection, and obligation-aware localization improve scoring. Matching still omits explicit root-image identity, while unframed `absent` findings cannot identify which image lacks the object. |
| R1-14 | Provider call matching and schema contract incomplete | **Partly resolves.** Native IDs and opaque-history preservation are explicit. Gemini’s missing-ID exception remains unrestricted by model capability, and the Anthropic schema passthrough is incompatible with documented strict-tool constraints. |
| R1-17 | Manifest lacks an evidence contract | **Partly resolves.** Envelopes, dependencies, hints, and projections are defined. Job-specific obligation binding, complete size bounds, and the distinction between model arguments and MCP transport arguments remain incomplete. |
| R1-18 | MCP startup and failure semantics incomplete | **Partly resolves.** Version pinning, initialization, pagination, deadlines, and shutdown improve the contract. Shared-process isolation, injected-argument schemas, native MCP error precedence, and derived-frame delivery still need rules. |
| R1-19 | Locations cannot map reliably between frames | **Partly resolves.** Coordinate conventions and clipping are added. The tagged-record rules omit required frames for spatial variants, prohibit previously allowed equirectangular polygons, and leave absence without image identity. |
| R1-20 | Source-fidelity defaults privilege architecture | **Partly resolves.** Architecture is correctly isolated as a profile. Its formal exemption still grants the structural waiver its prose forbids, and exemption coverage across an obligation’s scope is not executable. |
| R1-21 | Cloud consent and retention underspecified | **Partly resolves.** Consent now names the profile/account/upstream chain, with explicit local retention and enforced-versus-attested controls. Gateway provenance remains conditional, and the handling of missing provenance and redirects is unspecified. |
| R1-22 | Untrusted-content boundary is only a prompt assertion | **Resolves.** Trusted inputs are distinguished, registry-only image references are required, credential exclusions override pack environment declarations, and unsandboxed network restrictions are accurately described as contractual. |
| R1-23 | Reports retain hashes instead of inspectable evidence | **Resolves.** Artifacts, cancellation, artifact-before-event durability, oversized-body handling, reconstruction, and pruned-cache misses now have explicit rules. Oversized transcripts remain intentionally incomplete, with that limitation recorded. |
| R1-24 | Testing overclaims conformance and violates offline operation | **Partly resolves.** Capability fixtures and exact signature replay are improved, and runtime downloads are removed. Locked provisioning remains unimplemented as a contract, and plain development `uv run` can synchronize dependencies before the supposedly offline test command. |

## New findings

1. **[blocker] Anthropic’s prescribed schema passthrough rejects the submission tool**

   **Section:** “Wire schemas,” approximately lines 653–668.

   **Why:** The revision transforms unsupported constraints only for OpenAI while requiring Anthropic to receive the canonical schema unchanged. That schema contains numerical bounds, string lengths, and array constraints that Anthropic explicitly does not support in strict tools; the documented result is HTTP 400. This was confirmed in [Anthropic’s schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations). Gemini also distinguishes `parameters` from `parametersJsonSchema`, so “canonical schema as is” does not identify its wire representation. See [Google’s FunctionDeclaration reference](https://ai.google.dev/api/generate-content#FunctionDeclaration).

   **Concrete change:** Replace the passthrough rule with: “Every adapter has a versioned schema transformation. Anthropic strict schemas remove unsupported constraints from the wire schema while retaining them in canonical validation, and validate the combined tool set against schema-complexity limits. Gemini uses `parametersJsonSchema` for JSON Schema definitions, with an explicitly documented supported subset. Unsupported representations fail resolution before any billable request.”

2. **[blocker] A location-scoped exemption can waive an obligation covering a larger scope**

   **Section:** “Obligations,” “Checklist,” and “Semantic validation,” approximately lines 263–268, 347–350, and 457–459.

   **Why:** There is one checklist item per obligation, even when that obligation covers several images or subjects. Exemption validation checks only that the exemption covers the submitted location’s image; it does not require coverage of the entire obligation instance being marked `allowed_change`. Thus an exemption for one reference can satisfy the stated checks for a broader obligation, suppressing verification elsewhere. The architectural profile compounds this by placing SF1 in SF3’s `waives` despite saying structural changes are never waived.

   **Concrete change:** Add: “Resolve scope into explicit target image IDs, defaulting omitted scope to all job images, and create a mandatory obligation instance per target. Exemption validation uses that instance’s code-owned target, not a model-selected location; a waiver cannot cover other targets. Exemption applicability must also satisfy its textual condition. Architectural SF3 does not waive SF1; permission for dressing changes is recorded separately, while permitted occlusion is governed solely by SF1’s `hidden_ok`.”

3. **[major] Empty dependency sets and combined flags make aggregation non-total**

   **Section:** “Checklist,” “Aggregation,” and “Code style,” approximately lines 349–365, 474–492, and 1207–1217.

   **Why:** A reference checklist item can have no dependencies when all mandatory rules are candidate-scoped; an evidence item can become empty after scope filtering. Validation requires mandatory attribution only for findings, so a violation on such an item reaches the Rust example’s `unreachable!`, while `cant_tell` is silently recorded. Separately, combining `matters = any` and `hidden_ok = all` before evaluation is not equivalent to evaluating each obligation: a hidden item depending on `(matters=true, hidden_ok=true)` and `(matters=false, hidden_ok=false)` incorrectly becomes unsure.

   **Concrete change:** Add: “Every verdict-bearing checklist item has at least one enabled mandatory dependency; reject invalid bindings during resolution or evidence ingestion. Validate this invariant for items and findings before aggregation. Evaluate each dependency using its own `matters` and `hidden_ok`, then take the strictest resulting row. Bind tool evidence dependencies against the resolved job, including requirements; do not assume fixed SF IDs exist for every use case.”

4. **[major] The tagged location record has contradictory validity rules**

   **Section:** “Frames and locations” and “Submission schema,” approximately lines 235–250 and 448–466.

   **Why:** The exact-non-null rule lists only `bbox` for a bbox location and only coordinates for a view, apparently requiring their `frame` to be null; this conflicts with frame lookup and raw-coordinate mapping. The earlier section permits seam-split equirectangular polygons, while semantic validation later prohibits polygons on equirectangular frames. `absent` has neither a required frame nor a separate image ID, so exemption scope and localization cannot determine which image is missing something.

   **Concrete change:** Replace the variant rule with: “`bbox`, `polygon`, and `view` require a non-null `frame` plus their respective coordinate fields; `whole_image` and `absent` require a non-null frame and null coordinate fields. All other coordinate fields are null. Frame projection determines allowed geometry: seam-safe bbox/polygon geometry is permitted on equirectangular frames; `view` requires an equirectangular frame. Enforce yaw’s exclusive upper bound of 180 degrees.”

5. **[major] Partial-call repair requires a different history operation from complete-call rejection**

   **Section:** “Run state machine,” “Repair protocol,” and “Protocol requirements,” approximately lines 596–612 and 674–687.

   **Why:** Answering every complete call fixes ordinary invalid submissions, but does not make an incomplete tool block replayable. Anthropic’s documented treatment is to retry the preceding request with a higher output allowance, rather than append a user instruction after malformed tool history; see [incomplete tool-use handling](https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons#incomplete-tool-use-blocks). OpenAI’s `call_id` outputs and Gemini’s original signed Parts are confirmed, but neither supports treating every partial response as a universally appendable turn. The spec also leaves unclear whether repair after the final turn may reopen ordinary tool use.

   **Concrete change:** Add: “The journal is append-only; provider conversation history contains only replayable turns. Complete rejected calls receive native error outputs before repair. A partial, non-replayable turn is journaled but excluded from the next provider request; consume the single repair by reissuing from the preceding valid history under an adapter-specific, fixture-tested policy. Predeclare and reserve any larger repair output allowance. Repair after the final turn remains submission-only. Missing Gemini call IDs are accepted only by explicitly cataloged legacy contracts.”

6. **[major] `_dejaview` injection lacks a compatible MCP schema and error contract**

   **Section:** “Manifest” and “Transports,” approximately lines 867–871 and 895–909.

   **Why:** The same input schema is described as both the model tool definition and native MCP `inputSchema`, but Rust subsequently injects `_dejaview`. A closed schema without that property rejects the actual MCP call; a required property exposes transport metadata and paths to the model unless a projection removes it. The draft also does not define precedence between MCP `isError`/JSON-RPC errors and envelope `status`, allowing conflicting success and failure signals. MCP documents input validation, structured output schemas, and two distinct error channels in its [tools specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).

   **Concrete change:** Add: “MCP `inputSchema` validates the complete arguments including required `_dejaview`; a versioned model-schema projection removes harness-owned fields. The harness rejects model attempts to supply those fields and injects them only after model-argument validation. Native `outputSchema` describes the complete envelope. Conflicting `isError` and envelope status is `error: tool`; only a documented `invalid_args` mapping is recoverable. Require exact image-block count/order/media-type correspondence, including an empty `content` array when no blocks exist.”

7. **[major] The MCP image envelope cannot deliver the aligned inputs required by the first ML tool**

   **Section:** “Manifest,” “Transports,” and “Python tool pack,” approximately lines 833–841, 901–904, and 918–920.

   **Why:** `regions` requires an aligned frame produced by the Rust `align` tool. The injected MCP envelope supplies canonical PNGs and image metadata, but no transform registry, selected transform, or derived-frame handle. The Python process therefore cannot identify or reproduce the alignment that satisfied its precondition. Having an aligned frame somewhere in Rust’s registry does not make it available to a separate process.

   **Concrete change:** Add: “Calls requiring derived frames name the exact frame IDs. `_dejaview` includes their canonical parents, complete transform chains and parameters, and either lossless derived pixels or the versioned information needed to reproduce them. Preconditions identify the selected frame explicitly; multiple eligible alignments are never resolved by implicit ‘latest’ state. Both transports receive equivalent frame inputs.”

8. **[major] Shared MCP reset can corrupt concurrent or supposedly independent runs**

   **Section:** “Transports,” approximately lines 904–913.

   **Why:** A shared process lasts for an evaluation, but reset occurs only “between items,” not between the two independent runs of one decision. Concurrent decisions can also overlap a reset unless the process is leased exclusively, and `concurrent=true` governs calls rather than reset safety. After cancellation, a late response must not be treated as a fresh unknown-ID failure affecting another run; MCP explicitly requires graceful handling of cancellation races. See [MCP cancellation](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/cancellation).

   **Concrete change:** Add: “A shared process is leased exclusively to one run, including both constituent runs of a decision separately. Reset occurs after all previous calls are quiescent and before every new run, with a bounded acknowledgement deadline; failure restarts the process. Request IDs remain unique for the process lifetime. Tombstone cancelled IDs and ignore their late responses. Require exact protocol-version acceptance and an advertised tools capability before discovery.”

9. **[major] Spatial matching can manufacture agreement across different images or obligations**

   **Section:** “Decisions and runs” and “Metrics,” approximately lines 510–513 and 1064–1068.

   **Why:** Mapping two boxes into raw coordinates does not make them comparable if their root images differ. The decision matcher also does not require matching obligation dependencies, so identical coordinates for different claims can count as agreement. Localization has the same image-identity omission, and the decision tie-break identifies only run 1’s call order, leaving multiple run-2 candidates ambiguous.

   **Concrete change:** Add: “Decision evidence pairs are eligible only when root image IDs and canonical mandatory dependency sets match. Sort eligible pairs by descending IoU, then run-1 call/region order, then run-2 call/region order, before greedy assignment. Compare exemption identity for `allowed_change` as well as disposition and confidence. Localization additionally requires the same root image and compatible obligation identity; whole-image and absence matches use those same eligibility checks.”

10. **[major] Observational rules cannot produce a valid observational finding**

    **Section:** “Obligations” and “Submission schema,” approximately lines 293–296, 412–418, and 460–463.

    **Why:** SF4 requires other changes to be noted, but findings permit only `violation`, `allowed_change`, and `cant_tell`. Observational IDs may appear only alongside a mandatory ID, so a harmless change unrelated to a protected obligation cannot be reported honestly. Requiring it to cite a mandatory obligation changes its meaning and can introduce uncertainty into aggregation.

    **Concrete change:** Add an `observed` finding disposition: “`observed` findings cite one or more enabled `observe` obligations, require a reason and location, have `allowed_by = null`, and always produce `Recorded`. They may not satisfy mandatory checklist items. Other findings retain their mandatory-attribution requirements.”

11. **[major] The bootstrap gate remains underspecified and can claim certainty from one family**

    **Section:** “Metrics” and “Gates,” approximately lines 1071–1074 and 1093–1111.

    **Why:** “10,000 bootstrap resamples clustered by family” does not specify the estimand, weighting, interval method, or handling of samples with no failing items. If all improved failures belong to one family, resampling that sole family produces a degenerate positive interval without demonstrating independent-family evidence. The 2× cost gate is also ambiguous because the metrics expose both median and total, and Wilson intervals do not apply to cost or wall time.

    **Concrete change:** Add: “For detection, use the item-weighted paired difference in fail-detection proportions over expected-fail items. Resample failure-bearing families with replacement, preserving all paired items within each selected family, and use the 2.5th and 97.5th percentile interval under a pinned quantile algorithm. Freeze a justified minimum number of independent failure-bearing families; below it, return `inconclusive`, never admission. Apply Wilson intervals only to binomial proportions. The cost gate uses total charged cost divided by all evaluated items, including errors. Freeze the tested model/tool hypotheses and multiplicity policy before unsealing.”

12. **[major] The new provisioning command does not install the claimed locked environment**

    **Section:** “Python tool pack” and “Commands,” approximately lines 923–935 and 1135–1140.

    **Why:** `uv tool install dejaview-tools==<version>` installs a package into uv’s tool environment; it does not automatically consume that package’s repository `uv.lock` or create the specified dejaview version directory. Consequently, recording a lock digest would not prove that the installed dependencies match it. uv documents tool installation separately from project lock synchronization; ordinary `uv run` can also synchronize before running tests. See [uv tool installation](https://docs.astral.sh/uv/reference/cli/#uv-tool-install) and [locking and syncing](https://docs.astral.sh/uv/concepts/projects/sync/).

    **Concrete change:** Replace setup with: “Distribute a versioned provisioning bundle containing `pyproject.toml`, `uv.lock`, the tool package, and their digests. Setup verifies the bundle and runs `uv sync --frozen --no-dev` in its version-specific directory using the declared Python version, then fetches model artifacts. Runtime launches that environment’s executable directly. Offline development checks use the provisioned environment or `uv run --frozen --offline --no-sync`; absent dependencies fail without downloading.”

13. **[minor] The Vercel example mixes a verified allowlist with unverified wire fields**

    **Section:** “Endpoint profiles,” approximately lines 715–728; “Open questions,” line 1322.

    **Why:** Official documentation confirms `providerOptions.gateway.only` on `/v1/messages`, including failure when no allowed provider is available. It documents model fallbacks through `gateway.models`; I could not confirm `fallbacks = false` or `x-vercel-ai-provider` in the inspected official material. The candidate-only shipping condition is appropriate, but the example currently presents these placeholders as executable facts, and only out-of-allowlist provenance—not missing provenance—is explicitly rejected. See [provider filtering](https://vercel.com/docs/ai-gateway/models-and-providers/provider-filtering-and-ordering) and [model fallbacks](https://vercel.com/docs/ai-gateway/models-and-providers/model-fallbacks).

    **Concrete change:** Add: “Remove undocumented `fallbacks` fields; send no model-fallback list and reject virtual-model routing for this profile. Mark the provenance mechanism unresolved until documented and live-tested. Missing, malformed, or disallowed upstream provenance yields `error: provider`. Keep the profile unshipped until these fixtures pass.”

## Questions for the owner

1. Confirm the architectural policy: may **any** occlusion permit a pass through `hidden_ok`, or only occlusion caused by an explicitly allowed edit?
2. Confirm `on_unsure = pass`, `on_error = review`, the proposed admission margins, and the evaluation budget needed for independent admission and confirmation families.
3. Confirm indefinite local artifact retention and trusted, unsandboxed tool packs as the first-release operating model.

## What is good

- Keep the vision model as the judge, deterministic tools as evidence, and code as the sole verdict authority.
- Keep mandatory selection independent of `matters`, with explicit exemptions and irreversible operational errors.
- Keep complete-decision caching and the decision-envelope transfer/settlement model.
- Keep canonical measurement images separate from provider-prepared images, and retain the shared `visual_only` control.
- Keep exact native call pairing and opaque-history preservation. Google confirms original signed Parts and exact call IDs; OpenAI confirms `call_id` outputs and stateless reasoning continuity. See [Gemini function calling](https://ai.google.dev/gemini-api/docs/generate-content/function-calling.md), [OpenAI function calling](https://developers.openai.com/api/docs/guides/function-calling), and [OpenAI reasoning](https://developers.openai.com/api/docs/guides/reasoning).
- Keep sealed initial admission, independent confirmation, inspectable artifacts, and an explicit inconclusive outcome.