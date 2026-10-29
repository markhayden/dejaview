Run on 2026-10-29 from the matrix beside this document: the initial admission gate of the
`regions` tool for source fidelity (PLAN task 5.6). Arms C and D_regions on the sealed blind set
`source-fidelity/blind-v1` (20 items in 5 families, 10 expected fail and 10 expected pass) under
gemini-4-argon, one repetition, decision cap $10, matrix cap $25, bootstrap seed 20261030, 10,000
resamples, one hypothesis. The labels were held by the owner until the gate scored; receipt
`8e0c8307a72475ac`.

- **Outcome: rejected.** Both arms detected every one of the ten planted failures, so the
  detected-failure difference is zero, its paired interval is zero to zero, and no family's
  detection differs between the arms. The gate holds a tool to "detected failures higher than C"
  and that cannot be met on a set the control already gets entirely right. The result says the
  set is too easy for this model, not that the tool is useless: the admission rule is doing what
  it was written to do.
- **What the tool did change.** The one flip was on the pass side: under C the model rejected
  `greenhouse-interior-grade`, a colour grade, as a failure; under D_regions, with `regions`
  reporting what differed, it passed it. False rejects went from one to none. Localisation of the
  ten detected failures went from 9 hits of 10 labelled regions to 10 of 10, with more spurious
  predictions (30 of 40 against 20 of 29): the tool's evidence items are pointers the model
  then cites, and it cited more of them.
- **How the model used the tools.** Under C the model called `align` on every item (15 registered;
  the 5 structural transforms came back not applicable, which it read correctly as the SF2
  violation) and `view` on 11. Under D_regions it added `regions` on 14 of 20 items (13 answered
  with regions, one measured nothing), which added 82 evidence items to the checklists, and never
  called `classes`. Turns rose from 44 to 62.
- **Spend.** The ledger's $1.32 is the vendor-side figure: $0.52 under C, $0.79 under D_regions
  (1.5 times), all Google. No errors, no unsure answers, no resume.
- **Binary.** The run used a build of `3957e6b` (the `m5-ml-tool` branch after the blind set was
  published). The pack was the installed bundle 0.0.1 at lock digest `ca3e746…`, with the
  segmenter fetched from the export's directory, as one process for the whole evaluation.
- **Two things learned about the harness.** The summary of this run recorded decision
  directories as absolute paths, because the run now makes them absolute for the packs it
  restarts there while the evaluation directory stays as given; the runner strips both forms
  from now on, and the published `summary.json` here is relativised by the results-doc script
  (the receipt's results hash covers the summary as written, so the run's own file is left
  alone). And the blind set's item ids name their constructions, as the dev sets' do; the model
  never sees an id, so the gate stands, but the next blind plan gives items opaque ids.
