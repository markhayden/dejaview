# Evaluation sets

An evaluation item is a complete job plus labels. The job is canonical and runnable by
`dejaview check --job`; the labels live in a separate file so blind sets ship without them. The
contract is `SPEC.md` under "Evaluation" ("Set manifest", "Seal"); this page is how the sets are
laid out and handled. How public sets are generated, labelled from construction records and
reviewed image by image arrives with task 3.2a.

## Layout

```
evalsets/<use-case>/<set>/        the set's id is <use-case>/<set>
  set.toml                        [set] metadata and [[item]] entries (id, family, job)
  families.toml                   [[family]] id and description
  labels.toml                     [[label]] per item; absent on a blind set, whose manifest
                                  carries labels_sha256 instead
  seal.toml                       written by `sets seal`
  items/<item>/job.toml           a full job; image paths relative to the set directory, each
                                  pinned by sha256
  items/<item>/*.png              the images, fetched by hash (see below)
```

Every file is parsed strictly: an unknown key anywhere (an `expected` on an item, a label in a
job) is a load error, which is how a public manifest or job is kept free of answer labels.

`[set]`: `id`, `use_case`, `split` (`dev`, `heldout`, `blind`), `licence`, `images_repo`,
optional `images_rev`, `labels` or `labels_sha256`, `families`, and `rules = { base, version }`
pinning the shipped rules the items are judged under. The fixture set
`fixtures/evalsets/source-fidelity/pairs-v1` is the smallest complete example and the one the
tests mutate.

## Commands

- `dejaview sets check <root or set directory>` loads every set under `<root>` (or the one set)
  and fails on the first problem: a set not at its id's path; an unknown or wrongly versioned
  rules base, or one for another use case; an item whose job has another use case or does not
  resolve against the pinned rules (a missing required role, for instance); an undeclared family;
  an image without a pin, missing, or with another hash; labels wrong for the split (a dev or
  heldout set names a label file that labels every item exactly once and nothing else; a blind
  set has no label file and carries `labels_sha256`); a region with the wrong geometry; and,
  across sets of one use case, a family in more than one split. Exit 0, or 1 with the reason on
  stderr; 64 when the root holds no set.
- `dejaview sets fetch --set <id or path> [--root evalsets]` places every pinned image beside the
  manifest from `images_repo`: `github.com/<owner>/<repo>` serves the file raw at `images_rev`,
  an `https://` base serves it at `<base>/<set id>/<path>`, and anything else is a directory
  relative to the set with the same `<set id>/<path>` layout. A download whose hash differs from
  the pin is refused and leaves nothing behind; an image already in place with the right hash is
  left alone.
- `dejaview sets seal --set <id or path> --attest "<text>" [--labels-out <dir>]` checks the set,
  then writes `seal.toml`: one SHA-256 per component (sorted item ids, family assignments, every
  canonical job, every image file, the pinned rules' name, version and content, the label file or
  its external hash) and one over all of them, with the time and the owner's attestation that no
  judge or tool has seen the items. For a blind set whose labels still sit beside the manifest,
  `--labels-out` names the owner-storage directory the label file is moved to; the manifest's
  `labels = …` line becomes `labels_sha256 = …`. The timestamp and attestation are outside the
  hash, so re-sealing unchanged content reproduces it.
- `dejaview sets verify-seal --set <id or path>` recomputes every component and names the ones
  that changed. Exit 0, or 1 with the components on stderr.

The seal records the owner's attestation; prior exposure of the items to a judge or tool cannot
be proven by code, and gate scoring treats journal searches as supporting evidence only.

## Building a set (task 3.2a, ADR 0013)

`scripts/sets/` is a small Python toolkit (`uv run dejaview-sets …` from that directory). A
**plan** names the set and its families; each family is one scene or subject with one generator
(`fal` for FLUX.2 klein, `openai` for gpt-image-2, `gemini` for Gemini 3.1 Flash Image, or
`fake`), a source prompt and seed, the protected subjects, a brief, and its items, each made from
the source by one construction op. `fixtures/sets/houses-v1.plan.toml` is the dry-run plan.

Two family kinds. A **scene** family (source fidelity) has one source and items made from it by
an op: `grade` (a mild colour grade: a pass), `mirror`, `rotate`, `crop` (the structural rule, a
fail over the whole image) and `edit` (the generator changes one named thing, the identity rule,
a fail whose region is the densest patch of pixel difference; any other dense patch the generator
changed unasked is reported as a secondary change for the reviewer). A **subject** family
(reference consistency) has one to four generated references of one subject and three item
forms: a `scene` composed from the references (a pass whose identity the owner confirms), an
item built `from` an earlier candidate by an `edit` that changes a marking (RC2), and a `scene`
with a `lookalike` description generated without the references (RC3). The draft label comes
from the construction alone; nobody looks at the picture to write it, and no judge or tool runs.
`fixtures/sets/pets-v1.plan.toml` is the first dev set's plan; the set itself is
`evalsets/reference-consistency/pets-v1` and `evalsets/README.md` tabulates every published set.

```
cd scripts/sets
uv run dejaview-sets build --plan <plan.toml> --out evalsets/<use-case>/<set> --owner <owner storage>
# review <owner storage>/<set id>/contact-<family>.png and labels-draft.toml:
# set each label's status to "confirmed" or "struck", fill adjudicated_on
uv run dejaview-sets promote --draft <owner storage>/<set id>/labels-draft.toml --set evalsets/<use-case>/<set>
dejaview sets check evalsets/
```

`rebuild --item <id> [--seed-offset N]` regenerates one item from the family's images already on
disk, with the plan as it now reads (a reworded edit, a new seed offset), and puts only that
item's label back to draft; this is the review loop for an item the owner struck.
`build` writes the public set directory (manifest, families, one job per item with pinned
images, the family's source stored once under `families/`) and, to owner storage, the
construction record (generator, model, prompt, seed, every edit instruction, hashes, dates), the
draft labels and a contact sheet per family. `promote` publishes the confirmed labels as
`labels.toml` and removes struck items from the set; a draft still unreviewed is refused. The
owner then pushes the images to the `dejaview-evalsets` repository and commits the manifest,
jobs and labels here. `just sets-check` lints and tests the toolkit and runs the offline dry run
through `dejaview sets check`; the live dry run of 2026-10-27 is recorded in ADR 0013.

Credentials for the generators come from the environment only (`FAL_KEY`, `OPENAI_API_KEY`,
`GEMINI_API_KEY`); the toolkit never writes them anywhere. Owner storage is a directory outside
both repositories; nothing in it is ever committed.
