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
