**No—the spec is not ready to plan against.** The architecture is sound, but the current contracts permit false passes, incorrect cache reuse, and spending beyond the stated cap. The evaluation design cannot yet substantiate individual-tool claims, and several provider and MCP details would force implementation-time design decisions. Official documentation confirms much of the provider approach, but also exposes model-specific exceptions and missing authentication requirements. No files were modified.

## Findings

1. **[blocker] Model-supplied rule IDs can bypass aggregation**

   **Section:** “Checklist,” “Submission,” and “Code style,” approximately lines 231, 247–270, and 695–706.

   **Why:** Checklist items have code-owned rule dependencies, but aggregation trusts the model’s independently supplied `rule_ids`. A submission can include every required checklist ID while assigning its violation to `[]` or an unrelated `matters = false` rule, causing the implementation shown to skip it. The stated validation does not require those dependencies to match.

   **Concrete change:** Add: “For checklist dispositions, aggregation uses the checklist’s immutable rule dependencies; submitted `rule_ids` must match them exactly. Findings require nonempty, enabled, known rule IDs. Validate image IDs, locations, and evidence references against this run before aggregation.”

2. **[blocker] Aggregation treats unresolved evidence as passing and contradicts rule semantics**

   **Section:** “Rules,” “Submission,” “Aggregation,” and “Code style,” approximately lines 209–211, 266–280, and 695–706.

   **Why:** `hidden` explicitly does not establish that something held, yet falls through to `pass`; so does `held` with tentative confidence. `matters` is defined as controlling uncertainty, but aggregation also uses it to suppress confident violations of `must_hold` and `must_not` rules. The prose considers `cant_tell` only on items, while the Rust example includes findings. The brief requires: “Any confident violation → fail; any ‘can't tell’ on something that matters → unsure; otherwise pass.”

   **Concrete change:** Replace both aggregation definitions with one normative table: confident violations of mandatory rules fail regardless of `matters`; tentative violations produce unsure; `hidden`, `cant_tell`, and tentative assertions of compliance produce unsure when the obligation matters. Apply identical rules to items and findings, and permit `allowed_change` only under an explicitly applicable exemption.

3. **[blocker] The cache can return a verdict for a different job**

   **Section:** “Run loop” and “Cost, caching, drift,” approximately lines 309–313 and 410–413.

   **Why:** The enumerated key omits the job brief, image roles and labels, projection overrides, and `max_output_tokens`. Identical images checked against “preserve the logo” and “remove the logo” can therefore share a key despite opposite requirements. Cache lookup also precedes image preparation even though prepared sizes are part of the key. The brief says: “Key on everything that can change the answer.”

   **Concrete change:** Add: “Construct the key only after canonical job resolution, image preparation, and tool discovery. Hash the complete semantic job, effective request settings, rendered instructions, prepared-image identities, implementation/preprocessing versions, and all catalog/tool dependencies; exclude only secrets and nonsemantic output destinations.”

4. **[blocker] Failure handling contains paths from operational failure to pass**

   **Section:** “Architecture,” “Run loop,” and “Adapter trait,” approximately lines 129–130, 323–333, and 369.

   **Why:** A failed tool becomes an error result, after which the model can submit all items as held and obtain `pass`; nothing marks the run invalid. A tool timeout could follow that route despite the global timeout-is-error rule. Refusals are repairable in the run loop but unconditional errors in the Anthropic requirements. The brief explicitly says: “A provider error, timeout or budget overrun is an **error**, never a verdict and never an automatic pass.”

   **Concrete change:** Define one failure taxonomy: terminal provider/transport failures, tool crashes, timeouts, and budget overruns invalidate the run; a successfully executed tool’s inability to measure is structured evidence requiring disposition. Specify which model-output failures receive the single repair, and state that repaired output cannot erase an operational run error.

5. **[blocker] The spend-cap guarantee lacks a finite, enforceable reservation model**

   **Section:** “Run loop,” “Cost, caching, drift,” and “Manifest,” approximately lines 314–316, 407–409, and 429–441.

   **Why:** The whole-run estimate does not explicitly include final requests, repair requests, retries, both decision runs, or repeated transmission of growing history. Tool manifests do not bound returned text, images, or regions, so the stated worst case cannot be calculated. A timed-out request may have incurred cost even without returned usage. The brief requires: “Reserve each request's worst-case cost before sending so a spend cap cannot be overshot.”

   **Concrete change:** Add: “Before every billable attempt, atomically reserve a conservative bound against shared job and evaluation ledgers. Include all history, output/reasoning allowances, retries, repairs, and applicable endpoint charges; retain the reservation when billing is unknown. Bound tool outputs and reject any request whose upper bound cannot be established.”

6. **[major] The run loop is not a bounded state machine yet**

   **Section:** “Run loop” and “Config,” approximately lines 317–333 and 505–511.

   **Why:** It is unclear whether `rounds = 4` includes first look, final submission, and repairs. Separate repair clauses could each grant another request, and behavior is undefined when a parallel batch exceeds the remaining call allowance. Early submission, duplicate submissions, and submission alongside outstanding tools also lack rules.

   **Concrete change:** Specify states and counters explicitly: “First look counts toward tool-enabled turns; one submission-only turn is reserved separately; one run-wide repair request is allowed. Validate an entire call batch before execution and reserve its call allowance atomically. Submission must be the sole call, with no pending tools; reject mixed or duplicate submissions. Retries have a separate bounded attempt counter.”

7. **[major] Checklist completeness does not cover everything the model must judge**

   **Section:** “Checklist,” approximately lines 224–232.

   **Why:** Free-text brief requirements receive no independently tracked obligations, so a model can ignore one while completing every checklist item. Tool-returned crops, class results, or other items need not enter the checklist unless the tool explicitly “pointed at” a region. The brief requires: “Every region or item the model was shown gets a disposition.” Rule-level checkboxes cannot establish that all defects were noticed.

   **Concrete change:** Add structured, identified job obligations alongside contextual brief text. Require every enabled mandatory rule and every distinct evidence region/item presented to the model to map to a checklist entry, with deterministic deduplication and bounded growth. Replace “Silence cannot hide a miss” with “Missing dispositions are detectable; undetected defects remain an evaluation concern.”

8. **[major] Two-run precedence is complete, but disagreement and repetition are undefined**

   **Section:** “Aggregation” and “Runner,” approximately lines 283–293 and 581–593.

   **Why:** The verdict precedence covers all verdict pairs, but “shared item” is ambiguous when each run independently discovers regions. Stable IDs within a run do not establish identity across runs, and comparing only dispositions ignores differing rule attribution or confidence. The matrix’s `runs` dimension also conflates independent evaluation repetitions with two runs aggregated into one decision.

   **Concrete change:** Separate `decision_runs = 1|2` from `evaluation_repetitions`. Specify the symmetric table: `F×{P,F,U,E}=F`; `E×{P,U,E}=E`; `U×{P,U}=U`; `P×P=P` only when the defined comparison agrees, otherwise `U`. Define comparison over canonical mandatory-item IDs and normalized dispositions/confidence; explicitly specify treatment of unmatched dynamic regions.

9. **[major] The arms cannot establish that an individual tool helps**

   **Section:** “Python tool pack” and “Runner,” approximately lines 471–475 and 589–590.

   **Why:** Comparing the full tool set D against C attributes any improvement to a bundle, not to the tool being admitted. C is called “no measurements” even though `align` returns an integrity summary. The brief specifies “Tools one at a time” and requires comparison with “the same zoom ability but without the measurement.”

   **Concrete change:** Define `C` as a fixed visual-access control and `D_X = C + X`, holding model, instructions unrelated to X, decision repetitions, and budgets constant. Supply identical alignment transforms in both arms and either suppress diagnostic alignment evidence or expose it equally. Evaluate bundles separately and claim only bundle-level benefit from bundle comparisons.

10. **[major] Blind evaluation and admission gates are absent from the deliverable**

    **Section:** “Set manifest,” “Runner,” and “Success criteria,” approximately lines 576–593 and 765–768.

    **Why:** Success requires only five development sets, yet also requires admission or rejection of the first measurement tool. A committed label hash establishes integrity, not blindness, especially if labels remain in the public manifest or developers repeatedly inspect results across gates. Twenty items per use case have no specified pass/fail balance, independence, detectable effect, or uncertainty calculation. The brief requires “Separate development sets … from a blind set” and states that “‘Inconclusive’ is an allowed result.”

    **Concrete change:** Require a distinct held-out release set before admission, with source/subject families separated across splits. Freeze configurations, gates, repetitions, consumer policy, and analysis before unsealing; document access to labels. Declare class counts, paired effect measures, uncertainty intervals, and multiplicity handling; retain `inconclusive` in success criteria rather than forcing admission or rejection.

11. **[major] Evaluation items do not fully specify the job or scoring**

    **Section:** “Set manifest” and “Runner,” approximately lines 534–587.

    **Why:** Items carry images and labels but no explicit job brief or pinned effective rules, making brief-compliance examples and item-specific permitted changes ambiguous. “Localisation hit rate” lacks matching rules, thresholds, handling of multiple defects, and treatment of whole-image or absent-object failures. Consumer acceptance policy is mentioned but has no defined configuration or scoring denominator.

    **Concrete change:** Each item must reference a complete canonical job and separately stored labels. Define label granularity, adjudication, accepted verdicts, treatment of errors, and localization matching—including frame conversion, one-to-one matching, threshold, and unmatched predictions. Publish numerator and denominator for every metric, including abstentions and operational failures.

12. **[major] Claude controls are incorrectly described as API-wide facts**

    **Section:** “Run loop,” “Adapter trait,” and “Model catalog,” approximately lines 325–329, 364–369, and 395–403.

    **Why:** Current official documentation confirms forced-tool rejection for Opus 5.5 and Sonnet 5.5, but explicitly says Haiku 5.5 accepts it; the spec includes all three. Thus “Current Claude models reject forced `tool_choice`” is overbroad, while `strict` guarantees argument shape rather than invocation. Sampling support is likewise a model/thinking capability, not simply an API-version property. [Claude tool definitions](https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools), [thinking compatibility](https://platform.claude.com/docs/en/about-claude/models/extended-thinking-models).

    **Concrete change:** Add catalog capabilities for forced-tool modes, strict schemas, sampling controls, thinking compatibility, and required headers. Select final-round behavior by the exact model/endpoint combination and require fixtures for each supported capability combination; reject unsupported settings before sending.

13. **[major] Image limits confuse native processing limits with request rejection limits**

    **Section:** “Run loop,” “Adapter trait,” and “Model catalog,” approximately lines 312–313, 366–368, and 397–400.

    **Why:** Claude’s documentation distinguishes maximum accepted dimensions, model-specific native-resolution/token limits, encoded byte limits, and the stricter many-image limit; 1568 pixels is not a universal current maximum. The many-image count includes historical images and tool-result images, so checking only initial inputs is insufficient. OpenAI also has model-specific `detail` preprocessing that is absent from config. The brief requires: “Freeze image sizes per provider and test the largest possible conversation.” [Claude vision limits](https://platform.claude.com/docs/en/build-with-claude/vision), [OpenAI image processing](https://developers.openai.com/api/docs/guides/images-vision).

    **Concrete change:** Catalog accepted formats, native resize/token rules, explicit detail settings, per-image and request byte limits, and cumulative image limits per model/endpoint. Define deterministic preparation separately from rejection, preflight every outgoing history, bound tool images, and record all transformations. Treat undocumented server preprocessing as unverified rather than claiming it cannot occur.

14. **[major] Gemini call matching and the shared provider schema contract are incomplete**

    **Section:** “Submission” and “Adapter trait,” approximately lines 236–270 and 351–374.

    **Why:** Gemini responses cannot be matched merely “by name”: current documentation requires returning the exact function-call ID, which matters when two parallel calls use the same tool. Preserving thought signatures is correct, but requires preserving their original Parts. OpenAI Responses supports the proposed functions and reasoning-item continuation, but “its equivalent” leaves exact call/output mapping, strict-schema translation, and terminal-status handling unspecified. The submission example is an instance sketch, not the promised JSON Schema. [Gemini function calling](https://ai.google.dev/gemini-api/docs/generate-content/function-calling), [OpenAI function calling](https://developers.openai.com/api/docs/guides/function-calling).

    **Concrete change:** Specify actual versioned schemas and their supported provider subset. Require Gemini `functionCall.id` → `functionResponse.id`, unchanged signed Parts, OpenAI `call_id`-matched outputs and required reasoning items, and Anthropic paired results. Add fixtures for repeated same-name parallel calls, multimodal tool outputs, nullable fields, malformed arguments, and incomplete terminal responses.

15. **[major] Endpoint presets cannot express every promised cloud integration**

    **Section:** “Decisions” and “Endpoint presets,” approximately lines 68 and 376–393.

    **Why:** A base URL, headers, authentication scheme, and model prefix do not describe model-in-path routing or body transformations. Google-managed Claude uses prediction endpoints with `model` in the URL and `anthropic_version` in the body; Anthropic-managed Google Cloud uses a different contract. Modern Bedrock does expose a native Messages endpoint, so treating all Bedrock integrations as incompatible would also be wrong. The exact backend must be selected before calling it a preset. [Google Cloud integration comparison](https://platform.claude.com/docs/en/build-with-claude/claude-models-on-google-cloud/overview), [Bedrock Messages endpoint](https://platform.claude.com/docs/en/build-with-claude/claude-in-amazon-bedrock).

    **Concrete change:** Add an explicit transport/backend profile containing path construction, body/header transformations, credential resolution, region/project requirements, streaming framing, error normalization, and fallback controls. Name supported cloud products precisely and require conformance fixtures per shipped preset, not just per wire adapter.

16. **[major] Authentication scope exceeds the specified credential lifecycle**

    **Section:** “Decisions,” “Adapter trait,” “Endpoint presets,” and “Open questions,” approximately lines 85–87, 372–393, and 777–778.

    **Why:** Official OpenAI documentation confirms an open-source ChatGPT-plan flow, but it requires registration, PKCE, granted inference scopes, account-bound credentials, and refresh handling—not merely obtaining a client ID. Its inference contract requires `store=false`, `stream=true`, and successful stream completion; those requirements are missing. The inspected documentation does not establish this harness’s complete image-plus-tool workflow, so live conformance remains necessary. ADC and persistent OAuth credentials also need reconciliation with “Secrets only from the environment.” [OpenAI sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in), [inference contract](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference).

    **Concrete change:** Make first-release API-key support explicit. Gate OAuth behind a separately specified credential lifecycle and image/tool conformance suite; list permitted credential sources and storage exceptions for each auth scheme. Distinguish local open-source eligibility from hosted-service eligibility, and never substitute another billing/auth path silently.

17. **[major] The manifest does not define the evidence contract**

    **Section:** “Manifest” and “Python tool pack,” approximately lines 429–475.

    **Why:** Output is only prose, and the stated rendering omits even that output guidance. There is no machine-readable contract for regions, measurements, units, transforms, evidence IDs, applicability, or output bounds, yet the harness must use results to grow the checklist. `pairs_with` does not distinguish optional companions from prerequisites, and rule hints have no specified delivery path. The brief requires tools to describe “inputs and their validation, output, cost, when to use it, what it pairs with.”

    **Concrete change:** Require a versioned output schema and common result envelope containing status, measurements, regions, image artifacts, provenance, and bounds. Add applicability/preconditions and explicit dependencies; define how rule hints reach tools. Render output interpretation and prerequisites into guidance, and reject manifests whose schemas cannot be represented by the selected provider.

18. **[major] The MCP transport omits mandatory startup and failure semantics**

    **Section:** “Transports” and “Config,” approximately lines 461–465 and 499–510.

    **Why:** Startup omits protocol/capability negotiation and the required `notifications/initialized` notification before normal operation. Image IDs become paths or base64 without a defined argument envelope, and `accepts` is both pack-declared in prose and config-owned in the example. Per-call deadlines, malformed responses, process death, concurrent request correlation, pagination, and the effect of killing a process shared across an evaluation are unspecified. [MCP lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle), [MCP tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools).

    **Concrete change:** Pin supported MCP versions and specify initialization, capabilities, initialized notification, paginated discovery, and frozen tool definitions. Define the exact image argument/result encoding, artifact lifetimes, call deadlines, size limits, cancellation, shutdown escalation, and pending-call failure handling. Choose process isolation per run or define an explicit reset contract between evaluation items.

19. **[major] Locations cannot reliably map between raw, aligned, and cropped images**

    **Section:** “Job,” “Submission,” and “Transports,” approximately lines 165–170, 249–269, and 455–460.

    **Why:** `aligned` is defined only relative to a source, while jobs may have several references and no source. A location does not identify the transform or crop that produced its frame, so the viewer and localization scorer cannot reconstruct it unambiguously. A 2:1 aspect ratio does not establish equirectangular projection, and spherical coordinates lack conventions. Requiring a location for every violation also lacks a representation for missing objects or whole-image defects.

    **Concrete change:** Define immutable frame IDs with source/target image IDs, transform IDs, crop provenance, and mappings back to canonical raw coordinates. Specify bbox ordering, polygon validity, spherical axes/ranges/wrapping/FOV, and orientation/color normalization. Require explicit projection when metadata is inconclusive, and add whole-image and unlocalizable/expected-region location variants.

20. **[major] The shipped source-fidelity rules privilege the prototype’s use case**

    **Section:** “Rules,” approximately lines 175–206.

    **Why:** The example is explicitly marked as the shipped base set, yet it permits people, furniture, vegetation, and other objects to disappear. That conflicts with the README’s portrait example, which preserves face, pose, and wardrobe while changing the background. It effectively promotes architectural fidelity to the default meaning of source fidelity. The brief says: “The harness must not be shaped around one of them.”

    **Concrete change:** Name the existing rules as an architectural-restyling profile. Define generic source fidelity around explicit protected subjects/features and allowed edits, with named profiles for narrower workflows. Specify rule scope, conflicting-rule precedence, merge order, and validation that job use case, base rules, and required image roles agree.

21. **[major] Cloud consent does not specify the actual disclosure and retention policy**

    **Section:** “Data leaving the machine,” “Report,” and “Config,” approximately lines 297–305, 418–423, and 518–520.

    **Why:** A boolean and account label do not identify gateway recipients, logging/caching behavior, model destinations, or which images and metadata leave the machine. Documentation of required account settings is not equivalent to enforcing request-level settings; OpenAI Responses, for example, has default application-state retention unless configured otherwise. Local reports and caches also retain images without permissions, retention, or deletion rules. The brief requires explicit opt-in and documentation of “each provider's retention and no-training settings.” [OpenAI data controls](https://developers.openai.com/api/docs/guides/your-data).

    **Concrete change:** Bind consent to an explicit endpoint/account/region and recipient chain. Define request storage controls, gateway logging/cache requirements, metadata stripping, redirect policy, and local artifact permissions/retention. Document separately what the harness enforces, what the operator attests, and what the provider does not expose for verification.

22. **[major] Untrusted-content handling is a prompt assertion rather than a defined boundary**

    **Section:** “Rules,” “Manifest,” “Transports,” and “Testing strategy,” approximately lines 218–220, 444–451, 461–465, and 734–735.

    **Why:** Tool guidance is promoted into the system prompt without a stated trust model for tool authors, while subprocesses have no specified environment isolation. Model-selected image IDs have no explicit prohibition against becoming arbitrary filesystem paths. Offline fake-provider tests can prove that data does not alter harness control flow, but cannot prove that image instructions never influence a real model’s verdict. Server URL inputs additionally introduce an unspecified network-fetch boundary.

    **Concrete change:** Declare config, rules, and approved manifests trusted; keep image text, filenames, labels, and tool observations in delimited data fields. Resolve image IDs only through the job registry; launch packs with an environment allowlist excluding provider credentials. Specify bounded decoding and safe URL fetching for server mode, and separate deterministic isolation tests from live prompt-injection evaluations.

23. **[major] Reports retain digests where inspection requires evidence**

    **Section:** “Report” and “Run loop,” approximately lines 297–305 and 334–337.

    **Why:** Request, response, and tool-result digests cannot show what the model actually saw or what a measurement returned. The report therefore cannot deliver the README’s “every step it took to get there” or support meaningful diagnosis of a disagreement. Recording only at the end also leaves early provider, preparation, or reservation failures without a specified durable report path, despite the requirement to retain errors.

    **Concrete change:** Store bounded, redacted model-visible request/response and structured tool-result artifacts, referenced by content hash from an append-only run journal. Persist terminal status for every started evaluation item, including early failures and cancellation. Define partial-report recovery and viewer/schema compatibility for previously published report versions.

24. **[major] The testing contract overclaims conformance and contradicts offline operation**

    **Section:** “Commands,” “Testing strategy,” and “Tech stack,” approximately lines 601–604, 621, and 723–742.

    **Why:** Replaying one recorded fixture per adapter cannot validate every model capability and endpoint preset promised by the spec. Byte-identical request testing needs a rule for dynamic authentication, timestamps, and signatures. Ordinary Python tests are documented to download weights, while the testing boundary says no test accesses the network without `SQUINT_LIVE=1`. The private segmenter export also lacks a specified public artifact, license record, and reproducibility contract.

    **Concrete change:** Define fixtures by supported model-capability/backend combination, compare canonical requests after excluding explicitly identified volatile fields, and include full image/tool/repair conversations. Separate artifact fetching from offline tests. Require the segmenter’s immutable artifact hash, license/provenance record, preprocessing version, runtime/execution provider, and determinism or tolerance guarantees before its milestone can begin.

25. **[minor] The stated reason for excluding fal is factually too broad**

    **Section:** “Decisions,” approximately line 88.

    **Why:** fal documents a Florence-2 caption endpoint for vision-language tasks with textual output, so “no vision-language endpoint that returns text” is not a sound exclusion criterion. Its general Any VLM endpoint is marked deprecated, which supports caution but does not establish absence of all such endpoints. Neither fact establishes suitability for this harness’s agentic protocol. [Florence-2 caption API](https://fal.ai/models/fal-ai/florence-2-large/caption/api), [deprecated Any VLM API](https://fal.ai/models/fal-ai/any-llm/vision/api).

    **Concrete change:** Replace the claim with: “fal is out of scope for the initial judge adapters. Admission requires verified multi-image input, structured tool interaction, conversation continuity, model identity, and error semantics.”

## Questions for the owner

1. When a protected feature is hidden by an allowed edit, should the result be `unsure`, or can particular rules explicitly waive verification?
2. What missed-failure ceiling, false-rejection allowance, abstention rate, and cost increase justify admitting a tool—and what consumer policy applies to `unsure` and `error`?
3. Who holds blind labels and authorizes their release, and what evaluation budget is available for sufficiently sized held-out sets?
4. Should initial source fidelity be a general preservation contract with explicit allowed edits, or a collection of named domain profiles?
5. Are OAuth, Bedrock, Vertex, and gateways first-release commitments, or should the first release prove the three direct API-key integrations?
6. What local image/transcript retention and cloud/gateway retention policies are acceptable, and are external tool packs trusted to access local files and the network?

## What is good

- Keep one code-owned verdict authority and retain the model’s overall opinion only as an observation.
- Keep deterministic tools as evidence providers; do not turn measurement thresholds into verdict gates.
- Keep the Rust core independent of the CLI, with injected provider and tool transports.
- Keep native provider adapters, opaque history preservation, and recorded conformance fixtures.
- Keep equal-visual-access controls, immutable evaluation records, and cache bypass during evaluation.
- Keep explicit cloud opt-in and the exclusion of Claude subscription credentials; the latter is supported by the current [Anthropic credential policy](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use).