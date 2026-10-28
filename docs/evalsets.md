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
