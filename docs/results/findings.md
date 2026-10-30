# Findings log

One entry per evaluation run for the record: the question it asked, the headline numbers with
their intervals, what surprised us, and what the harness changed because of it. The results
documents in this directory hold every number; this log holds the story around them. Plain
English, nothing private.

## 2026-10-28: the arm A baseline ([baseline-2026-10](baseline-2026-10/README.md))

**Question.** With no tools, how well do six models judge the ten published sets, at what price,
and how repeatable is each one?

**Headline, development sets, repetition 1 (100 items: 60 expected fail, 40 expected pass).**

| Model | Detected failures | Missed | False rejects | Unsure | Median cost per item | Agreement with repetition 1 |
|---|---|---|---|---|---|---|
| gemini-4-argon | 60/60 (0.94 to 1.00) | 0 | 11/40 (0.16 to 0.43) | 2/100 | $0.017 | 0.97 |
| gpt-6.1-sol | 58/60 (0.89 to 0.99) | 1 | 11/40 (0.16 to 0.43) | 3/100 | $0.018 | 0.96 |
| claude-opus-5-5 | 57/60 (0.86 to 0.98) | 0 | 8/40 (0.11 to 0.35) | 22/100 | $0.050 | 0.94 |
| gpt-6.1-terra | 57/60 (0.86 to 0.98) | 2 | 8/40 (0.11 to 0.35) | 9/100 | $0.009 | 0.97 |
| claude-sonnet-5-5 | 56/60 (0.84 to 0.97) | 0 | 5/40 (0.06 to 0.26) | 21/100 | $0.025 | 0.94 |
| gemini-3.8-flash | 52/60 (0.76 to 0.93) | 3 | 7/40 (0.09 to 0.32) | 9/100 | $0.010 | 0.85 |

Intervals are 95% Wilson. A "missed" failure is one the frozen policy would let through as a
pass; the Anthropic models missed none because their doubt goes to `unsure`, which the policy
sends to review.

**What stood out.**

- Every model catches nearly every planted defect without tools. The sets are not hard enough yet
  to separate the models on detection; the intervals all overlap. Separation will have to come
  from harder items, from false rejects and from cost.
- The two Anthropic models hedge. They answered `unsure` on about a fifth of all items and on
  roughly half of the clean images (Opus 19 of 40, Sonnet 17 of 40), where the other four models
  said `pass`. Under the frozen policy that is a review queue, not a wrong answer, but it is the
  main difference in behaviour between vendors.
- Gemini 4 Argon is the most decisive: everything detected, two unsures in a hundred, but the
  highest false-reject rate alongside GPT-6.1 Sol. The cheapest two models (Terra, Flash) are
  within the intervals of the dearest on detection at a fifth of the cost.
- Cost per decision differs more than quality does. Opus is about five times Terra per item.
  The pre-run estimate (about $33) undercounted the Anthropic share; the vendors charged about
  $28 in all for the decisions that completed, Anthropic $18 of it.
- The run exhausted the Anthropic credit balance near its end. The harness recorded the 68
  refused decisions as `error: provider`, charged them at their bound because a refused call
  returns no usage, and the resume redid exactly those 68 after the credit was topped up.

**What changed because of it.**

- Localisation scoring. The first scoring reported almost no hits: the models put violation
  locations on checklist items, as the prompt asks, and the scorer read only free-form findings.
  Checklist-item violations are now predictions; the baseline was re-scored from its saved
  reports. The remaining low hit rate is mostly `whole_image` labels against boxed predictions
  and one defect reported under several obligations, both open questions for the metric's
  definition (`tasks/todo.md`).
- Open: whether a call refused before processing should cost nothing in the ledger rather than
  its bound, and whether the estimate should lead with per-model observed costs.

**Next.** M4 adds `view`, `align` and the change-map tools. The same matrix under arms B, C and D
against this baseline is the first paired comparison, and the question becomes whether tools
earn their cost on the false rejects and on harder items.

## 2026-10-29: arm C, the visual-access control ([arm-c-2026-10](arm-c-2026-10/README.md))

**Question.** When a model can look closer, does it judge better? Arm C gives the model `view`
(crops and perspective views) and `align` as a picture only, no measurements, and pairs every
item against the same model under arm A from the same run. Five models (Opus left out: over the
per-decision cap under arm C and over the standing $25 per vendor with it), one repetition,
bootstrap seed 20261028.

**Headline, development sets, repetition 1 (100 items in 25 families: 60 expected fail, 40
expected pass). Detection difference is C minus A with its paired bootstrap interval.**

| Model | Detected A → C | Detection difference | False rejects A → C | Unsure A → C | Median cost A → C |
|---|---|---|---|---|---|
| gemini-4-argon | 60 → 60 | 0.000 (0.000 to 0.000) | 12 → 13 | 0 → 0 | $0.017 → $0.025 |
| gpt-6.1-sol | 58 → 58 | 0.000 (0.000 to 0.000) | 10 → 9 | 1 → 3 | $0.018 → $0.029 |
| gpt-6.1-terra | 57 → 57 | 0.000 (0.000 to 0.000) | 8 → 9 | 9 → 12 | $0.009 → $0.015 |
| claude-sonnet-5-5 | 56 → 56 | 0.000 (0.000 to 0.000) | 5 → 4 | 18 → 29 | $0.025 → $0.081 |
| gemini-3.8-flash | 54 → 56 | 0.033 (-0.048 to 0.115) | 7 → 10 | 4 → 5 | $0.010 → $0.019 |

Detection is out of 60, false rejects out of 40, unsure out of 100. The intervals are 95% paired
bootstrap over families; a zero interval means no family's detection changed, not certainty.

**What stood out.**

- Looking closer did not change the verdicts. Across the five models, 39 of 500 paired items
  flipped, and nearly every flip was between `pass` and `unsure` on a clean image. Detection moved
  for one model (Flash, four gained and two lost) and its interval straddles zero. The sets are
  still too easy for a visual-access tool to matter: every planted defect is visible in the
  presented image, so a crop adds nothing the model did not already have.
- The tools were hard to call, and that is the harness's finding, not the models'. 124 of 196
  `align` calls were refused because the model passed the frame id the prompt shows
  (`raw:candidate`) where the tool wants the image id (`candidate`); GPT-6.1 Sol never got one
  through in 35 tries. Claude Sonnet made 325 `view` calls and none ran: it sent `yaw: 0` without
  `fov` on a flat frame, was told the three go together, and repeated the same call next turn until
  its turns ran out. Only 12 of Sonnet's 110 arm C decisions ever saw a tool picture, against 94 to
  109 for the other models. Sonnet's arm C is therefore arm A plus refused calls: triple the cost,
  eleven more unsures, the same detection.
- Cost is where the arms differ. Arm C cost 1.5 to 2 times arm A per item for the four models
  that used the tools, 3.3 times for Sonnet. Argon and Terra remain the decisive and the cheap
  choices respectively, as in the baseline.
- Reliability. Three decisions in 1100 ended in error: Sonnet once sent the submission's `items`
  as a JSON string and its one repair did not mend it; Gemini Flash returned `HTTP 400` twice,
  minutes apart, on the two garden-shed adversarial items, once before any tool call. The run
  cost $28.31, Anthropic $11.69 of it; the pre-run estimate was $1.47 to $5.15 per decision after
  the planner was fixed, and the observed median was $0.015 to $0.081.

**What changed because of it.**

- The planner. Before the run it priced every stored tool envelope's inline pixels as text and put
  each arm C decision near $500; it now prices the presentation the model reads, with images as
  images.
- The report. Paired flips per model with gained and lost detections, false rejects, the families
  that differ and the paired bootstrap with its seed, families and hypothesis count are part of
  `eval report` and the results document.
- Open, first in line before the next tools run: `align` takes a frame id for a raw image;
  `view`'s refusal says what to send on a flat frame; a tool-side `invalid_args` is journaled once.
  Recorded in `tasks/todo.md`.

**Next.** M5 tests the measuring arms, DX (one measurement tool at a time) and D (all of them),
against this run's arm A. The sets need harder items too: defects a crop or an alignment would
reveal and the presented image hides, such as small text, fine geometry and subtle edits.

## 2026-10-29: the first admission gate, regions on the blind set ([gate-regions-initial-2026-10](gate-regions-initial-2026-10/README.md))

**Question.** Does a change-region finder earn its place? The `regions` tool points at where an
aligned pair differs (photometric change and segmentation class disagreement, boxed and labelled).
The initial gate ran arms C and D_regions under gemini-4-argon on `source-fidelity/blind-v1`, a
sealed set of 20 items in 5 new scene families whose labels the owner held until scoring.

**Headline (receipt `8e0c8307a72475ac`).**

| Arm | Detected failures | Missed | False rejects | Unsure | Errors | Cost | Localisation hits |
|---|---|---|---|---|---|---|---|
| C (view, align as a picture) | 10/10 | 0 | 1/10 | 0 | 0 | $0.52 | 9 of 10 |
| D_regions (C plus regions) | 10/10 | 0 | 0/10 | 0 | 0 | $0.79 | 10 of 10 |

Paired bootstrap of the detected-failure difference: point 0, interval 0 to 0; no family's
detection differs. **Outcome: rejected.** Admission is unchanged.

**What stood out.**

- The control got everything right. Ten planted failures, ten detected, under the visual-access
  tools alone. A gate written as "detect more than the control" cannot be passed on a set the
  control already saturates, and the rule said so plainly: rejected on "detected failures not
  higher", with the interval and the family count agreeing. This is the gate working as designed,
  and a lesson about the sets: planted removals of a named subject at this size are easy for
  this model.
- The tool's effect showed up where the gate does not score it. The one flip was a false reject
  taken back: under C the model failed a colour grade; with `regions` reporting what had actually
  changed, it passed it. Localisation of the detected failures rose from 9 of 10 to 10 of 10,
  because the model cited the tool's boxes. Both are the kind of gain a tool of this sort should
  give, and neither is the admission estimand.
- The model used the tool when offered: 14 of 20 items, 82 evidence items added, at 1.5 times
  the control's cost. It never called `classes`.
- The run itself was clean: 40 decisions, $1.32, no errors, no unsure answers.

**What changed because of it.**

- The published summary carries decision directories relative to the evaluation directory
  whatever the run recorded (the run had started writing absolute paths for the packs it restarts
  in them).
- Recorded: the next blind plan gives items opaque ids, since the ids of this one name their
  constructions.
- Open, for the owner: the next set needs to be harder for the control before a detection gate
  can separate tools. Candidates are smaller removals, removals of one subject among many alike,
  partial changes (a window bricked up rather than a building removed), and changes under a strong
  restyle, where alignment and a change map should matter most. And a gate that credits a tool
  for false rejects taken back and for localisation would need an ADR, since the estimand was
  frozen before the labels were unsealed.

**Next.** M5 closes with its checkpoint. Before the confirmation stage, a harder source-fidelity set
of at least 50 items in new families, built with the opaque-id plan and the owner's review.

## 2026-10-30: the second admission gate, regions on a harder blind set ([gate-regions-initial-2-2026-10](gate-regions-initial-2-2026-10/README.md))

**Question.** ADR 0017: was the first rejection only a set the control saturated? blind-v2 was
built to be hard for the control: 25 items in 5 new scene families, 15 planted failures of four
harder kinds (a small object removed, one of several alike removed, part of an object altered, a
removal under a strong restyle), opaque ids, the same model and judging configuration, scored at
two hypotheses so the earlier attempt counts.

**Headline (receipt `be41ae402334a394`).**

| Arm | Detected failures | Missed | False rejects | Unsure | Errors | Cost | Localisation hits (spurious) |
|---|---|---|---|---|---|---|---|
| C (view, align) | 15/15 | 0 | 1/10 | 0 | 0 | $0.70 | 8 of 15 (25 of 33) |
| D_regions (C plus regions) | 15/15 | 0 | 1/10 | 0 | 0 | $0.97 | 8 of 15 (45 of 53) |

Paired bootstrap of the detected-failure difference at 0.0125 and 0.9875: point 0, interval 0 to
0; no family differs. **Outcome: rejected.** Admission is unchanged.

**What stood out.**

- Harder was not hard enough. gemini-4-argon with zoom and alignment caught every planted
  failure: one lamp post, one chimney of five, one rowing boat of four, a clock face, a bandstand
  roof, a crane under a dusk restyle. Removal-shaped failures that a generator can plant cleanly
  are at ceiling for this model, so a detection gate cannot separate any tool from it here.
- This time the tool changed nothing at all: no item flipped either way, the same localisation
  hits, nearly twice the spurious boxes, 1.39 times the cost and a third slower. On blind-v1 it
  took back one false reject; here there was nothing for it to take back.
- Both arms rejected the same pass item (`boatyard-5`, a dusk restyle) for the same reason, a
  roller door raised far higher than in the source. Two arms agreeing on a specific structural
  change suggests the generator moved the door and the label is wrong; the owner checks it.
- 50 decisions, $1.67, no errors, no unsure answers. The set's build cost a few dollars more.

**What changed because of it.**

- The set consistency test knows a sealed blind set (labels only by hash), and the set docs build
  into owner storage as the publish script expects; the first build went into the repository by
  following the old docs.

**Next.** An owner decision on what a tool has to do to earn a place when the control is this
strong (see the reply to the owner of 2026-10-30): a gate under a cheaper model, where the control
misses more and a tool could close the gap to a stronger model; a gate on false rejects and
localisation (ADR 0017 option 2); or leave `regions` out of source fidelity and move on (option 3).
