Run on 2026-10-28 from the matrix beside this document: arms A and C on the ten published sets,
five models, one repetition, decision cap $10, matrix cap $75, bootstrap seed 20261028.

- **Why five models.** claude-opus-5-5 is left out. In the baseline it cost about five times
  gpt-6.1-terra per decision with no better detection; its arm C envelope ($10.30) is over the
  per-decision cap; and with it the Anthropic share would pass the standing $25 per vendor.
- **Arm C.** The visual-access control: `view` (crops and perspective views) and `align` under
  its `visual_only` projection, so the model gets the aligned candidate as a picture and no
  measurement. Prompts differ from arm A only in the Tools section (the test
  `arm_c_prompts_differ_from_arm_a_only_in_the_tools_section` builds both from this matrix), so a
  flip between the arms is the tools' doing.
- **Paired flips.** The report pairs A against C per model on the items both scored: flips each
  way, detected failures and false rejects gained and lost, and the paired bootstrap of the
  detected-failure difference under the frozen seed. The M3 baseline is historical context,
  never the comparison arm.
- **The estimate.** The envelope planner first put every arm C decision near $500: it priced each
  possible tool result's stored envelope, inline pixels included, as text. It now prices what the
  model reads and the returned images as images; the estimate before the run was $1.47 to $5.15
  per decision, $5,286 as the sum of envelopes.
- **How the run went.** One pass, 1100 decisions, no resume, about 12 hours on 2026-10-28 and
  29 local time. The ledger's $28.31 is the vendor-side figure too: Anthropic $11.69,
  Google $8.42, OpenAI $8.21, each under the standing $25 per vendor. Three decisions ended in
  `error` and the frozen policy sends them to review: claude-sonnet-5-5 under C sent the
  submission's `items` as a JSON string rather than an array and its one repair did not mend it
  (`submission_invalid`); gemini-3.8-flash under C got `HTTP 400 INVALID_ARGUMENT` from Google
  on both garden-shed adversarial items four minutes apart, once before any tool call and once on
  the turn after `align` returned its picture. The same request shapes served the other 108
  items, so the two look transient; a 400 is not retried and each was charged at its bound.
- **Binary.** The run used a build of `393d032`, the last commit when it started. The M4 review
  fixes landed in `327a701` while it ran and did not touch the runner, the prompts or what `view`
  and `align` do; `report.md` beside the run and this document's report sections were regenerated with
  `327a701`, whose paired summary also names the number of families and hypotheses.
- **The tools were hard to call.** This is the finding that matters most for reading the arm C
  numbers. Of 196 `align` calls across the five models, 124 were refused as `invalid_args` because
  the model passed the frame id the prompt shows (`raw:candidate`) where the tool wants the image
  id (`candidate`); gpt-6.1-sol never made a valid `align` call in 35 tries, and the others
  recovered on a later turn only sometimes (align calls that ran: Sonnet 12 of 35, Flash 16 of
  44, Argon 16 of 45, Terra 8 of 37). claude-sonnet-5-5 made 325 `view` calls and none ran: it
  sent `yaw: 0` (sometimes with `pitch: 0`) and no `fov` against a flat frame, was told "yaw, pitch
  and fov go together", and repeated the identical call on the next turn, usually until its tool
  turns ran out. Only 12 of its 110 arm C decisions ever saw a tool picture, against 94 to 109 for
  the other four models. So for Sonnet, arm C was arm A plus up to four turns of refused calls: its
  cost per item tripled ($0.025 to $0.081), its unsure count rose from 22 to 35, and its detection
  did not move. Both refusals are the harness's to fix before the next tools run: `align` should
  take a frame id for a raw image, and `view`'s refusal should say what to send instead on a flat
  frame.
- **Journal double entry.** A tool that refuses its own arguments (the `view` case above) is
  journaled twice under one call id: once at dispatch with `executed: true`, once with the refusal.
  Harness-side refusals (the unknown id case) are journaled once. Counts in this document
  de-duplicate by call id; the double line is recorded in `tasks/todo.md`.
