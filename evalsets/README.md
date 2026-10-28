# Evaluation sets

The manifests, jobs, families and confirmed labels of the public sets (docs/evalsets.md). Images
live in [dejaview-evalsets](https://github.com/markhayden/dejaview-evalsets) and are placed here
by `dejaview sets fetch --set <id>`; `dejaview sets check evalsets/` checks everything that is
checkable by code, and each set's `seal.toml` binds its content to the owner's attestation.

| Set | Split | Items | Families | Generators | Failure types | Reviewed |
|---|---|---|---|---|---|---|
| `source-fidelity/scenes-v1` | dev | 20 (10 pass, 10 fail) | 5 scenes: a brick house, a hotel pool, a village street, a red barn, a river bridge | FLUX.2 klein ×2, gpt-image-2 ×2, Gemini 3.1 Flash Image ×1 | a structural transform: mirror, rotate or crop (SF2, 5 items); a protected structure removed or changed by an edit (SF1, 5 items); pass items include a permitted restyle or dressing change per family | 2026-10-27 by the owner from the contact sheets; all twenty confirmed |
| `reference-consistency/pets-v1` | dev | 20 (10 pass, 10 fail) | 5 subjects: Pip (golden retriever), Marlow (tabby cat), Ada (grey parrot), Bramble (bay horse), Hobbes (corgi) | FLUX.2 klein ×2, gpt-image-2 ×2, Gemini 3.1 Flash Image ×1 | a marking changed by an edit (RC2, 5 items); a lookalike generated without the references (RC3, 5 items) | 2026-10-27 by the owner from the contact sheets; all twenty confirmed; two edits rebuilt before review because their first draws left the marking in place or painted it on |

**Review log.** Every generated image, kept or replaced, is in the owner's construction record
for the set (owner storage, never published), with generator, model, prompt, seed where the API
has one, every edit instruction, and the hash of each file. For pets-v1: 30 images kept (10
references, 20 candidates), 2 candidates replaced by `rebuild` before the review, 0 struck. No
judge model or tool of the harness ran on any item before labelling. The assistant that built the
set viewed the contact sheets of this dev set to report on edit quality; it never views a blind
set's items.
