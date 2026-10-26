<p align="center">
  <img src=".github/assets/logo.png" alt="dejaview" width="320" />
</p>

<p align="center"><strong>Because "looks good to me" doesn't scale.</strong></p>

dejaview is an agentic harness for evaluating the integrity of images. You give it an image to
check, whatever it should be checked against (a source photo, a reference subject, a brief),
and the rules that matter. A vision model works through the question with a set of tools,
then returns a verdict you can check: `pass`, `fail` or `unsure`, what it found, where it is
in the image, and every step it took to get there.

## Why

Image models are good enough to use in real pipelines and still wrong often enough that
someone has to look at every output. That someone is usually a person squinting at two images,
or a single similarity number that cannot tell a new background from a redrawn logo. Neither scales,
and neither explains itself.

Asking a vision model "does this look right?" gets you part of the way. It is good at
deciding whether a change matters and bad at noticing small ones: a logo nudged a few pixels,
a spot that went missing, a sixth finger in a crowd. dejaview gives the model
tools for exactly that, and makes it show its work.

## What it checks

Each of these is the same shape: something to compare against, rules, tools, a verdict.

- **Source fidelity.** An edited, restyled or re-generated image kept what mattered in the
  original. Swap the background of a portrait and the face, pose and wardrobe stay as shot;
  regrade a landscape for a campaign and the horizon and landmarks keep their shape. Only what
  was meant to change, changed.
- **Reference consistency.** A subject from a reference image shows up faithfully in a new
  scene. Put your dog on the bridge of a starship and it should still be your dog: same breed,
  same build, same spots.
- **Reality checks.** Mistakes no real photo would make: extra or missing limbs, fused
  fingers, impossible shadows and reflections, garbled text, objects melting into each other.
- **Brief compliance.** The image contains what was asked for and nothing it should not: the
  right objects in the right numbers, the right colours, text spelled correctly, no stray
  logos, watermarks or people.
- **Product fidelity.** A product shot keeps the exact product: label text, logo, shape,
  colour and proportions, whatever the scene around it.

These are where it starts. The harness is meant for any image evaluation you can write down as
rules and back with tools.

## How it works

TBD.

## How it is built

TBD.

## Origins

dejaview grew out of [Pixel](https://github.com/markhayden/bakin-bits-official/tree/main/agents/pixel),
the image-artist agent for [Bakin](https://github.com/markhayden/bakin), where generating
images turned out to be the easy half and knowing whether they were right was the hard one.

## License

[Apache-2.0](LICENSE).
