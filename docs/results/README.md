# Published results

Each evaluation that is run for the record gets one document here with every metric as numerator
over denominator with intervals, the resolved model ids and API versions, costs, the matrix file
and the summary JSON, plus `notes.md` on how the run went. Development-set results say so.
`scripts/results-doc.py` generates a document from a finished evaluation directory.

- [findings.md](findings.md): the running log, one entry per run, of what was asked, what came
  out and what changed because of it.
- [baseline-2026-10](baseline-2026-10/README.md): the arm A baseline (task 3.8), six models, no
  tools, ten development sets, 2026-10-28.
