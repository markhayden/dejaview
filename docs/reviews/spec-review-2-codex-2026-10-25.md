**No—draft 2 is not ready to plan against.** It substantially improves the architecture and closes several original failure paths, but some replacements introduce new ones: requirement flags can remove mandatory obligations, exemptions can suppress violations, and the normative submission schema cannot be sent unchanged to OpenAI strict tools. The MCP lifecycle also conflicts with the current protocol revision. Assumptions 7–12 are reasonable proposed positions, but their implementation is not consistently specified; assumption 13 additionally conflicts with the example consumer policy. Official documentation confirms the corrected Claude tool-choice distinction and the named Gemini models. This was a read-only review; no files were modified.

## Resolution of review 1

| # | Review 1 finding | Resolution |
|---|---|---|
| 1 | Model-supplied rule IDs bypass aggregation | **Partly resolves.** Checklist dependencies are now immutable, and references are validated. Findings can still attribute a confident violation solely to an `allowed` rule, leaving it nonmandatory and therefore nonfailing. |
| 2 | Unresolved evidence becomes pass; inconsistent rule semantics | **Partly resolves.** Hidden items, tentative compliance, and mandatory violations now have explicit aggregation behavior. Exemption applicability remains too permissive, and tentative `allowed_change` always passes through as recorded. |
| 3 | Cache can return a verdict for another job | **Partly resolves.** The brief, requirements, roles, labels, projections, and preparation order are corrected. The key still lacks explicit harness/preprocessing implementation identity and prepared-byte hashes, allowing reuse after behavior changes without a schema or manifest change. |
| 4 | Operational failure can become pass | **Resolves.** Operational failures irreversibly invalidate the run, successful inability to measure is distinguished from execution failure, and refusals consistently receive the single repair. |
| 5 | Spend-cap reservation is not enforceable | **Partly resolves.** Shared ledgers, bounded outputs, retries, decision runs, and unknown billing are addressed. Whole-run and per-attempt reservations have no defined relationship or settlement procedure, and bounds do not explicitly include accumulated model output and protocol overhead. |
| 6 | Run loop is not a bounded state machine | **Partly resolves.** Turn accounting, final turns, batch limits, and mixed submissions are substantially clearer. Repairs still lack provider-valid call/result completion rules, and retryable failures/backoff remain undefined. |
| 7 | Checklist does not cover everything shown | **Partly resolves.** Structured requirements help, but crops do not necessarily become obligations, and evidence beyond the cap is explicitly left unlisted. Geometry-only deduplication can also collapse distinct judgments. |
| 8 | Two-run disagreement and repetitions undefined | **Partly resolves.** Decision runs and evaluation repetitions are separated, with a complete verdict table. Dynamic-item matching lacks one-to-one assignment and tie rules, while caching inside the run state machine threatens independence. |
| 9 | Evaluation arms cannot establish individual-tool benefit | **Partly resolves.** `D_X = C + X` fixes bundle attribution. Clearing only `measurements` leaves alignment diagnostics available through summaries, regions, statuses, and transform metadata. |
| 10 | Blind evaluation and admission gates absent | **Partly resolves.** Held-out sets, sealed labels, family separation, and inconclusive outcomes are added. Success still does not require an initially blind admission set, and “intervals too wide” has no executable threshold or paired uncertainty method. |
| 11 | Evaluation jobs and scoring incomplete | **Partly resolves.** Complete jobs, separate labels, policy outcomes, and denominators are substantial improvements. Localization does not require matching defect/rule identity, and aggregation across repetitions remains unspecified. |
| 12 | Claude controls incorrectly treated as API-wide | **Resolves.** Capabilities are model-specific and unsupported settings are rejected. The distinction between Opus/Sonnet 5.5 and Haiku 5.5 forced-tool support is confirmed by [Claude’s documentation](https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools). |
| 13 | Image processing and rejection limits conflated | **Resolves.** Preparation, acceptance limits, detail controls, cumulative history checks, and undocumented processing are separated. Official [Claude vision documentation](https://platform.claude.com/docs/en/build-with-claude/vision) confirms that historical and tool-result images count toward many-image limits. |
| 14 | Provider call matching and schema contract incomplete | **Partly resolves.** Call IDs, original signed Parts, OpenAI `call_id`, and a concrete schema are added. The schema violates OpenAI strict requirements, and Gemini’s missing-ID fallback should be limited to explicitly supported legacy contracts rather than applied universally. |
| 15 | Endpoint presets cannot express promised integrations | **Resolves.** Profiles now include routing paths, body transformations, credentials, framing, and error normalization. Bedrock and Vertex are explicitly deferred instead of being promised as trivial first-release presets. |
| 16 | Authentication scope exceeds credential lifecycle | **Resolves.** First-release credentials are explicitly environment-sourced API keys; OAuth and cloud credential chains require later ADRs. The earlier environment-only contradiction is removed. |
| 17 | Manifest lacks an evidence contract | **Partly resolves.** Output schemas, envelopes, prerequisites, hints, and bounds are added. Mapping arbitrary observations to code-owned obligations, wire encoding of the envelope, and bounds on all serialized output fields remain unspecified. |
| 18 | MCP startup and failure semantics incomplete | **Partly resolves.** The legacy handshake, pagination, cancellation, and shutdown are described. The promised current revision uses a different lifecycle, and shared-process reset/concurrency plus exact image-envelope bindings remain unresolved. |
| 19 | Locations cannot reliably map between frames | **Partly resolves.** Frame identities, transform provenance, explicit projection, and absent/global locations help. Spherical origin, FOV axis/aspect ratio, seam-crossing polygons, and partial/out-of-bounds transform mappings still require invented behavior. |
| 20 | Source-fidelity defaults privilege architecture | **Partly resolves.** Architecture is correctly separated into a profile. Mandatory preservation rules and allowed edits still lack explicit exemption relationships and conflict precedence. |
| 21 | Cloud consent and retention underspecified | **Partly resolves.** Disclosure contents, local permissions, retention, and enforced/attested distinctions are added. Consent remains an account label plus boolean, without binding to an allowed recipient chain, and gateway routing is not constrained. |
| 22 | Untrusted-content boundary is only a prompt assertion | **Partly resolves.** Trusted manifests, registry-only image references, and separate live injection evaluations are explicit. Pack-declared environment additions and the `SQUINT_*` wildcard need an overriding credential exclusion, while network restrictions are promised without a corresponding launch contract. |
| 23 | Reports retain hashes instead of inspectable evidence | **Partly resolves.** Full artifacts and an append-only journal address inspection. Cancellation terminalization, artifact durability before final recording, oversized-body handling, and cache references to pruned artifacts remain undefined. |
| 24 | Testing overclaims conformance and violates offline operation | **Partly resolves.** Capability/profile fixtures and explicit weight fetching improve coverage and reproducibility. Runtime `uvx` can still download dependencies, and excluding unspecified “signatures” from comparisons can hide broken thought-signature replay. |
| 25 | fal exclusion is factually overbroad | **Resolves.** The unsupported absence claim is replaced with an explicit scope decision and capability-based admission criteria. |

## New findings

1. **[blocker] Requirement enforcement incorrectly depends on `matters`**

   **Section:** “Obligations and checklist,” approximately line 315; “Aggregation,” line 420.

   **Why:** A `must_hold` job requirement with `matters = false` receives no checklist item, even though the equivalent rule remains mandatory and can fail. Conversely, any requirement with `matters = true` becomes mandatory regardless of its `kind`. Allowed job requirements are also advertised by SF3 but cannot be cited through `allowed_by`, which accepts only rule IDs.

   **Concrete change:** Add: “Rules and job requirements share obligation semantics: `kind` determines mandatory versus exempt versus observational behavior; `matters` controls uncertainty only. Every enabled `must_hold` and `must_not` requirement receives a checklist item regardless of `matters`. Exemptions may reference enabled `allowed` rules or requirements by their canonical IDs.”

2. **[blocker] Exemption validation creates another route from violation to pass**

   **Section:** “Rules,” approximately line 294; “Semantic validation,” line 408; “Aggregation,” line 435.

   **Why:** Being enabled and in the same image scope does not establish that an exemption applies to a protected obligation. For example, the architecture furniture exemption can currently validate `allowed_change` on the permanent-structure checklist item. A finding attributed only to SF3 can also declare a confident violation yet remain nonmandatory, while uncertainty about whether an exemption applies is always recorded without affecting the verdict.

   **Concrete change:** Add: “Every exemption declares the obligation IDs it may waive; `allowed_by` must belong to that code-owned set and satisfy its scope. Every violation finding must cite at least one mandatory obligation. Tentative exemption applicability produces `unsure` when the affected obligation matters. An observation can never affect aggregation, even if its `matters` field is set.”

3. **[blocker] The normative submission schema is incompatible with the required OpenAI strict mode**

   **Section:** “Submission schema,” approximately lines 336–401; “Adapter trait,” line 575.

   **Why:** Several object properties are optional, and the root omits `additionalProperties: false`, despite the prose saying otherwise. OpenAI strict tools require every property to be required, with nullable values representing optional fields; its documented union form is `anyOf`. The specification simultaneously requires strict schemas and rejection of unsupported schemas, so its own submission tool cannot be admitted unchanged. These requirements are confirmed in the official [function-calling guide](https://developers.openai.com/api/docs/guides/function-calling) and [structured-output guide](https://developers.openai.com/api/docs/guides/structured-outputs).

   **Concrete change:** Add: “Maintain a canonical internal schema and versioned provider wire schemas. For OpenAI strict tools, every object has `additionalProperties: false`, every property is required, optional values are nullable, and location alternatives use a supported, unambiguous `anyOf` representation. Normalize nullable wire fields before canonical semantic validation. `submit_verdict` must pass schema conformance for every shipped provider.”

4. **[major] Evidence overflow and spatial deduplication discard distinct obligations**

   **Section:** “Obligations and checklist,” approximately lines 320–332.

   **Why:** The revision explicitly permits shown evidence to lack a disposition once 24 items have accumulated, contradicting both the brief and the subsequent completeness claim. IoU alone cannot identify equivalent evidence: a logo-shape defect and incorrect label text can occupy the same bounding box. Keeping the first item also makes the surviving obligation depend on concurrent completion order.

   **Concrete change:** Replace the overflow/deduplication rules with: “Every evidence item shown to the model has a tracked disposition. If a result would exceed the evidence budget, terminate with `error: budget` before presenting it. Deduplicate only semantically equivalent items with the same root image and obligation dependencies; retain all provenance and use deterministic call order, not completion order.”

5. **[major] The promised MCP versions do not share the specified startup protocol**

   **Section:** “Transports,” approximately lines 758–774; “Open questions,” line 1155.

   **Why:** Draft 2 promises the current MCP revision and its predecessor but specifies only `initialize` followed by `notifications/initialized`. The current official revision, `2026-07-28`, uses per-request protocol metadata and no negotiation handshake; stdio compatibility uses `server/discover` probing. The written sequence is valid for older revisions, not a universal lifecycle. This is confirmed by MCP’s [versioning specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning) and [stdio binding](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio).

   **Concrete change:** Replace the moving target with: “First release supports MCP `2025-11-25` using its specified initialization lifecycle. Supporting `2026-07-28` or later requires a separate version-specific transport implementation and fixtures; there is no automatic commitment to the latest two revisions.”

6. **[major] Repair messages can leave provider conversations with unanswered tool calls**

   **Section:** “Run state machine,” approximately lines 508–524; “Adapter trait,” line 564.

   **Why:** Invalid submissions and mixed batches receive “one repair,” but the spec does not require results for every rejected call before another request. With append-only history, a plain validator message can leave an Anthropic `tool_use` unanswered and make the repair request invalid. Stop reasons also need precedence over dispatch so a truncated response containing a complete-looking call is not accepted prematurely. Anthropic explicitly requires correctly ordered, paired results in its [tool-call documentation](https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls).

   **Concrete change:** Add: “Validate terminal status before dispatch. For a rejected response containing complete tool calls, execute none and append one provider-native error result for every call ID, including `submit_verdict`, before requesting repair. Define adapter fixtures for truncated arguments, invalid submissions, and mixed batches; no repair request may contain unmatched complete calls.”

7. **[major] External tools receive degraded provider images instead of canonical measurement inputs**

   **Section:** “Job,” approximately line 204; “Image preparation,” line 585; “Transports,” line 768.

   **Why:** Rust tools operate over decoded images, whereas MCP tools receive provider-prepared images, potentially downscaled and JPEG-compressed. The first measurement tool can therefore lose precisely the small changes it is meant to recover, and its output can vary with the judge model’s image limits. This also creates different evidence semantics for the two supposedly interchangeable transports.

   **Concrete change:** Add: “All deterministic tools receive the same canonical, orientation-normalized, full-resolution image data through lossless handles. Tool-specific resizing is versioned preprocessing independent of the judge provider. Provider preparation applies only when images are sent to the vision model; each artifact records its canonical parent and transformation.”

8. **[major] Whole-run and per-attempt reservations can double-count the same spending**

   **Section:** “Reserve,” approximately line 502; “Cost ledger,” lines 652–660.

   **Why:** Step 3 reserves the whole run, then every attempt reserves again against the same caps. Nothing says whether these are nested allocations, additional holds, or replacements. Multiplying each attempt’s reservation by remaining retries further overlaps future attempts, and completed requests have no explicit settlement/release rule.

   **Concrete change:** Replace with: “Reserve one decision-level envelope covering all decision runs. Each billable attempt transfers its bound from that envelope’s unallocated balance to an attempt hold without increasing total reserved spending. Settle known usage to actual cost and release the difference; charge unknown usage at that attempt’s bound. Include prior model outputs, tool outputs, request framing, and every supported billable component in future-request bounds.”

9. **[major] Two independent decision runs can share a cached result**

   **Section:** “Aggregation,” approximately line 441; “Prepare,” line 500; “Cache key,” line 668.

   **Why:** Both decision runs are defined as complete executions of the state machine, which includes cache lookup and writing. Their semantic inputs and `decision_runs` setting are identical, so the second execution can retrieve the first execution’s result unless an unstated outer coordinator prevents it. Agreement would then measure cache reuse rather than independent judgments.

   **Concrete change:** Add: “Caching operates only at the complete-decision boundary. On a miss, execute all constituent decision runs with internal cache reads and writes disabled, aggregate them, and cache the complete decision. A cache hit returns all original constituent runs and their combined result.”

10. **[major] The gateway profile does not enforce the declared recipient chain**

    **Section:** “Endpoint profiles,” approximately lines 595–620; “Data leaving the machine,” line 682.

    **Why:** The Vercel example has an empty body transformation and records only the gateway host and model, which does not identify the upstream inference provider. Vercel supports provider routing and fallback; its documented `only` option is the hard restriction on eligible upstreams. Therefore the profile does not establish the promised no-provider-fallback behavior or the actual disclosure chain. The endpoint itself is valid, but the missing routing control matters; see [Vercel’s provider-filtering documentation](https://vercel.com/docs/ai-gateway/models-and-providers/provider-filtering-and-ordering).

    **Concrete change:** Add: “Gateway profiles declare and enforce an upstream-provider allowlist. The Vercel Anthropic profile sets `providerOptions.gateway.only = ['anthropic']`, disables model fallbacks, and records returned routing provenance. Consent binds to the endpoint/account and allowed upstream chain. A profile unable to enforce that chain is not shipped.”

11. **[major] The visual-access control still exposes measurement evidence**

    **Section:** “Result envelope,” approximately lines 724–729; “Runner and arms,” line 930.

    **Why:** Clearing `measurements` does not remove the integrity summary, alignment-failure regions, status information, or `inlier_fraction`. Some of these also generate checklist items, so the supposedly measurement-free arm can receive both diagnostic evidence and extra judgment obligations. The revision consequently makes the control’s defining property false.

    **Concrete change:** Add: “Both C and D_X use the same versioned alignment-result projection: transformed visual artifacts and mapping parameters only, with neutral status text and no quality scores, diagnostic summaries, diagnostic regions, or diagnostic checklist entries. Identical alignment requests return identical projected artifacts in both arms.”

12. **[major] Admission gates can admit a tool with substantial operational failures**

    **Section:** “Assumptions,” approximately lines 25–43; “Metrics” and “Gates,” lines 903–956.

    **Why:** With `on_error = review`, failures converted into errors are not missed failures, and there is no error-rate admission ceiling. A tool can gain three detections while producing errors on many other items and still satisfy every listed default. Assumption 13 says evaluation accepts `unsure`, but the example policy says `review`; additionally, “intervals too wide” cannot be executed using marginal Wilson intervals and flip counts alone.

    **Concrete change:** Add: “Every gate freezes explicit `on_unsure`, `on_error`, and `max_error_rate` values; D_X must not increase error rate versus C. Gate analysis uses one predeclared paired decision per item; additional repetitions are descriptive. Admission requires all numeric gates and a positive lower bound for the paired detection improvement’s 95% interval, calculated by 10,000 family-cluster bootstrap resamples with a recorded seed. The first admission analysis uses sealed labels and frozen configurations; confirmation uses a distinct untouched set.”

13. **[major] The prescribed `uvx` launch violates the no-download runtime assumption**

    **Section:** “Assumptions,” approximately line 36; “Python tool pack,” line 788; “Config,” lines 812–816.

    **Why:** Fetching weights separately does not provision Python, the tool package, or its dependencies. The configured `uvx --from ...` launch can create its environment and download packages during a run, especially after cache eviction. Official [uv documentation](https://docs.astral.sh/uv/concepts/tools/) describes these environments as disposable and automatically recreated, so pinning the top-level package version does not establish offline runtime operation.

    **Concrete change:** Add: “An explicit setup/fetch command provisions Python, the pinned tool package, locked dependencies, and model artifacts. Runtime launches the provisioned executable with offline dependency resolution and automatic Python downloads disabled; missing assets produce `error: resolve`. Record the dependency lock digest in tool provenance and the cache key. Network prohibition for trusted pack code is an operator contract unless a sandbox is explicitly implemented.”

14. **[minor] Signature normalization can hide the adapter defect the fixtures should detect**

    **Section:** “Testing strategy,” approximately lines 1085–1092.

    **Why:** The fixture comparator removes unspecified “signatures,” while another requirement mandates exact replay of thought signatures and opaque reasoning blocks. If the same exclusion covers provider conversation signatures, an adapter that drops or corrupts them can pass conformance. Google explicitly requires the signature to remain in its original Part in its [function-calling documentation](https://ai.google.dev/gemini-api/docs/generate-content/function-calling.md).

    **Concrete change:** Replace “signatures” with: “transport authentication signatures only. Thought signatures, signed Parts, and opaque reasoning payloads remain byte-exact comparison targets; replay tests must fail when any is dropped, relocated, or modified.”

15. **[minor] The prescribed Rust trait conflicts with boxed adapter trait objects**

    **Section:** “Adapter trait,” approximately line 549; “Code style,” line 1041.

    **Why:** The spec requires boxed trait objects but declares a dispatchable `async fn` directly on `Provider`. That trait is not dyn-compatible under the documented Rust rules, so the exact interface and implementation policy cannot both stand. This is confirmed by the [Rust Reference](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility).

    **Concrete change:** Replace the `async fn send` requirement with: “`Provider::send` is dyn-compatible and returns a boxed `Send` future tied to the request lifetime.” Alternatively, explicitly permit static dispatch and remove the boxed-trait-object mandate.

16. **[minor] Canary disabling contradicts the owner-approval boundary**

    **Section:** “Drift,” approximately line 675; “Boundaries,” line 1124.

    **Why:** Two changed canaries automatically disable a configuration, but disabling a configuration is also an ask-first action. The canary command therefore has two incompatible required outcomes. The comparison baseline for “consecutive changed” results is also unstated.

    **Concrete change:** Add: “Compare both confirming canaries against the same pinned baseline. Two consecutive deviations mark the configuration `suspect` and recommend disabling it; disabling requires explicit owner approval.”

## Questions for the owner

1. Should architectural `hidden_ok` waive verification only when an explicitly permitted dressing change causes the occlusion, or for any hidden structure?
2. What exact evaluation policies apply to `unsure` and `error`? In particular, should `unsure` be accepted as assumption 13 states, despite the example’s `review` setting?
3. Do you approve the default admission margins, and what error-rate ceiling and budget should govern the blind admission and independent confirmation sets?
4. Are all three gateway profiles first-release commitments, including routing/privacy conformance, or should the release commitment remain the three direct integrations?
5. Do you accept indefinite local artifact retention and trusted, unsandboxed tool packs, with network restrictions enforced contractually rather than technically?

## What is good

- Keep the single code-owned verdict authority, immutable checklist dependencies, and irreversible operational-error status.
- Keep generic source-fidelity rules separate from the architectural profile.
- Keep model-specific capability validation and exact opaque-history preservation. The corrected Claude forced-tool distinction is documented.
- Keep the native adapters and catalog verification milestone. Google documents both proposed models as stable with image input and function calling: [Gemini 4 Argon](https://ai.google.dev/gemini-api/docs/models/gemini-4-argon), [Gemini 3.8 Flash](https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash).
- Keep complete evaluation jobs, separated labels, family separation, individual-tool comparisons, and an explicit inconclusive outcome.
- Keep inspectable artifacts and the append-only journal; these are essential to making verdicts reviewable.