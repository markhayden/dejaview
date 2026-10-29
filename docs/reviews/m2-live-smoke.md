# M2 live smoke (task 2.10b)

Owner decision 4 (2026-10-27): the standing $25 per vendor applies. Run on 2026-10-27 with
`scripts/live-smoke.sh` from the binary inside the release archive built by `dist` on this machine
(`squint-cli-aarch64-apple-darwin.tar.xz`, built from the 2.10a commit), after
`scripts/archive-smoke.sh` had verified the checksum file and passed the offline smoke on the same
archive. The Linux x86_64 and arm64 archives are built and smoked by the `archive` job of the
`package` workflow and, on a tag, by the `archive-smoke` workflow before the release is created.

The job: `examples/source-fidelity.toml` pointed at each direct profile with `--set`, the fixture
pair `fixtures/images/source-64x48.png` and `candidate-64x48.png`, each vendor's cheapest model.
Account ids are attestations from the environment and are not recorded here.

| Profile | Model | Verdict | Exit | Cost |
|---|---|---|---|---|
| `anthropic` | `claude-haiku-5-5` | unsure | 2 | $0.0008 |
| `google-ai-studio` | `gemini-3.8-flash` | pass | 0 | $0.0125 |
| `openai` | `gpt-6.1-terra` | pass | 0 | $0.0033 |

Total for the passing run: $0.017 (computed before the checkpoint fix that prices prompt-cache
reads and writes; the true figure is within a cent of it). Two earlier runs were refused before any call left the machine
for Gemini and OpenAI (below) and cost about $0.002 more on Anthropic. Whole task: about $0.02 of
the $25 per vendor.

## Found on the way

**The default budget refuses the cheapest Gemini and OpenAI models.** With the shipped defaults
(`retries = 3`, `repair_output_tokens = 32000`, `spend_cap_usd = 2.00`) the planned envelope was
$2.74 for Gemini 3.8 Flash and $3.74 for GPT-6.1 Terra, so `check` stopped with
`error: budget: decision envelope ... exceeds the remaining $2.00 of the $2.00 cap` and exit 3.
Haiku 5.5 fit. The envelope is a worst-case bound (every tool turn, the final turn and the repair,
each retried at the full output allowance), while the three checks above cost under two cents. The
checkpoint changed the formula (SPEC "Cost ledger"): each request is planned once plus a per-run
retry reserve of `retries` times the largest bound, and a retry the balance cannot cover is
refused with `error: budget`. The defaults now fit every shipped model; see the re-run below.

**Re-run under the default budget (2026-10-27, after the checkpoint).** With the envelope
planning each request once plus a retry reserve, the examples went back to the defaults
(`retries = 3`, `repair_output_tokens = 32000`, `spend_cap_usd = 2.00`) and the smoke was re-run
from the binary rebuilt at the checkpoint, with prompt-cache tokens priced:

| Profile | Model | Verdict | Exit | Cost |
|---|---|---|---|---|
| `anthropic` | `claude-haiku-5-5` | unsure | 2 | $0.0008 |
| `google-ai-studio` | `gemini-3.8-flash` | pass | 0 | $0.0104 |
| `openai` | `gpt-6.1-terra` | pass | 0 | $0.0048 |

**Verdict on the fixture pair.** The 64×48 pattern images are not a meaningful source-fidelity
job; `unsure` from Haiku and `pass` from the other two are all a working harness. The smoke accepts
exit codes 0, 1 and 2 and fails on anything else.
