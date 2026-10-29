# ADR 0014: Alignment by direct registration on normalised pyramids

Date: 2026-10-28. Status: accepted.

## Context

The `align` tool (SPEC "Tools") registers a candidate against its source: a homography for flat
images, a rotation on the sphere for equirectangular ones, under the assumption the tool exists to
check (same scene, same viewpoint, format intact), with measurements of how well the pair
registers. PLAN task 4.0 asked for a bounded spike before building it: can the needed accuracy be
reached (translation within 0.5 px at 2048 px width, rotation within 0.1°), with what approach and
dependencies, and how does it fail on unrelated, low-texture and degenerate pairs. Owner decision
11 rules out anything from the prototype, so the spike started from nothing but the `image` crate.

The pairs the harness sees are not general stereo pairs. The candidate is an edit of the source
made by a generator: the geometry is near-identical, the appearance may differ a lot (restyled
tone, colour, texture), and some regions have genuinely changed. The estimator must register on
shared structure, ignore appearance, tolerate changed regions, and refuse honestly when the pair is
not what it assumes.

## Decision

Direct intensity registration, implemented in `crates/dejaview-tools/src/align/` with no
dependency beyond `image`:

- Each image becomes a luminance pyramid (halving by 2×2 means to a smaller side of 24 px). When
  the two images differ in size by more than about 1.4×, the larger is halved first so every level
  pairs images of comparable pixel size.
- Every level is lightly blurred and **locally contrast-normalised**: `(I − mean) / sqrt(var + ε)`
  over a 9×9 window, `ε = 10⁻³` so contrast below about 3% of full scale (quantisation noise,
  faint gradients) is damped rather than amplified. Tone and colour changes between a source and
  its restyled candidate then matter little; structure matters. A six-pixel margin at every edge
  is excluded from every measurement, where the blur and the window are clipped.
- The coarsest level is searched exhaustively for the best **normalised cross-correlation** over a
  grid: for flat pairs every integer translation within half the image under rotations of ±20° in
  5° steps and scales 0.8 to 1.25; for panoramas every integer yaw column under pitch and roll of
  ±10° in 5° steps.
- Each level from there refines by **robust Gauss-Newton** on the residual between the warped
  candidate and the source: eight homography parameters in centred, size-normalised coordinates
  for flat pairs (so one parameterisation serves every level), the three parameters of a rotation
  vector composed onto the current rotation for panoramas (numerical Jacobian: six extra warps per
  iteration). A Huber weight (threshold 0.5 in normalised units) tames pixels that genuinely
  changed. Convergence is a step moving no source corner by more than a thousandth of a pixel;
  40 iterations per level at most. Levels above 1024 px on the longer side are not refined or
  measured: the cost would be real and the gain below the tolerance.
- The finest level used yields the **measurements** the tool reports: `overlap` (the share of the
  source landing inside the candidate), `ncc` over the overlap, and `inlier_fraction` (the share
  of structured source pixels, those at least 0.5 from their local mean, whose residual is below
  0.5), plus the weaker image's `texture` score (the square root of the smaller eigenvalue of the
  mean structure tensor at about 256 px).
- The outcome is `registered` only when registration converged and overlap ≥ 0.3, NCC ≥ 0.5 and
  inliers ≥ 0.5; `low_texture` when either image's texture score is below 0.004 before anything
  is estimated; otherwise `not_registered`. A pair that is not registered gets no transform (the
  pure size scaling, or the identity rotation), never a guess.

## Measured

Synthetic scenes rendered analytically under the true transform (no resampling in the ground
truth; `dejaview_testkit::images`), release build, `cargo test -p dejaview-tools --release
--test align measure_tolerances -- --ignored --nocapture`. Errors are pixel displacement at the
image centre and at the worst corner, and the rotation angle from the linear part.

| Flat case | Width | Centre px | Corner px | Angle ° | NCC |
|---|---|---|---|---|---|
| translation (29, −18 px at 2048) | 512 / 1024 / 2048 | 0.004 / 0.013 / 0.009 | 0.007 / 0.033 / 0.016 | 0.0003 / 0.0018 / 0.0001 | 0.998 / 0.985 / 0.997 |
| rotation 6.5°, scale 1.06 | 512 / 1024 / 2048 | 0.005 / 0.008 / 0.009 | 0.040 / 0.020 / 0.033 | 0.0005 / 0.0001 / 0.0001 | 0.998 / 0.986 / 0.998 |
| rotation 18° | 512 / 1024 / 2048 | 0.001 / 0.001 / 0.000 | 0.006 / 0.007 / 0.013 | 0.0008 / 0.0003 / 0.0002 | 0.998 / 0.987 / 0.998 |
| perspective (corners move ~4%) | 512 / 1024 / 2048 | 0.002 / 0.001 / 0.005 | 0.025 / 0.025 / 0.048 | 0.0004 / 0.0008 / 0.0005 | 0.997 / 0.984 / 0.996 |
| restyled (per-channel gamma and offset), rotation 3° | 512 / 1024 / 2048 | 0.039 / 0.036 / 0.048 | 0.271 / 0.221 / 0.664 | 0.0052 / 0.0021 / 0.0060 | 0.981 / 0.947 / 0.948 |

| Sphere case | Width | Rotation error ° | NCC | Inliers |
|---|---|---|---|---|
| yaw 23°, pitch 4°, roll −2.5° | 256 / 512 / 1024 / 2048 | 0.019 / 0.003 / 0.002 / 0.002 | 0.98 | 0.98 to 0.99 |
| yaw 150° | 256 / 512 / 1024 / 2048 | 0.030 / 0.007 / 0.007 / 0.007 | 0.99 | 0.98 to 1.00 |
| pitch 9° | 256 / 512 / 1024 / 2048 | 0.010 / 0.006 / 0.003 / 0.003 | 0.99 | 1.00 |

A real generated image (a 1024 px set item, upscaled to 2048 and resampled bilinearly through
the transform, so the truth carries interpolation): translation within 0.002 / 0.030 px at the
centre and 0.007 / 0.085 at the corners at 1024 / 2048; rotation 5° with scale 1.03 within 0.004
px and 0.0002°.

Against the plan's tolerances: translation at 2048 is within 0.05 px at the centre (0.5 asked)
and rotation within 0.006° (0.1° asked), with corners within 0.05 px for geometric changes and
within 0.7 px when the candidate's tone curve also changed. The sphere is within 0.03° at every
size from 256 px up. The whole measurement, fifteen flat and twelve sphere alignments up to 2048
px plus four on the real image, took 70 s in release: a few seconds per alignment at 2048 px,
under a second at 1024 px.

Failure fixtures (`fixtures/images/align/`, outcomes in `cases.toml`, checked by
`the_failure_fixtures_give_their_recorded_outcomes`): two unrelated scenes, a linear gradient
pair (`low_texture`), a mirror image, a quarter turn, a shift of 84% of the width, and a
registered control. The tests also hold independent-noise pairs to `low_texture` and reject each
degenerate case generated on the fly.

## Alternatives

- **Feature matching (ORB, AKAZE) with RANSAC homography.** The standard general-purpose answer,
  and the right one for different viewpoints. It needs either a computer-vision dependency tree
  (the rust-cv crates, thinly maintained) or a large in-house implementation, matches poorly
  across a restyle that changes local texture, and its accuracy stops at feature localisation
  unless followed by exactly the direct refinement adopted here. The pairs the harness sees have
  the same viewpoint by construction, so the direct method's limited basin (about ±20° and
  0.8× to 1.25× before the coarse search misses) is the assumption the tool checks, not a
  limitation.
- **Phase correlation (Fourier-Mellin) for the coarse step.** Elegant for translation and
  similarity, but it needs an FFT dependency, handles perspective not at all and partial overlap
  poorly; the exhaustive grid at a 24 to 48 px level costs tens of milliseconds and handles both.
- **Inverse-compositional or ESM Gauss-Newton.** Faster per iteration than the forward-additive
  update used; the iteration counts measured (about 25 per alignment) make the saving
  unnecessary at this stage, and the simpler update is easier to read and check.
- **An external library (OpenCV).** Would have been a C++ build dependency in a Rust binary the
  owner installs from crates.io; ruled out by ADR 0001's posture.

## Consequences

- Task 4.3a wraps `align_flat` and 4.3b `align_sphere` into the `align` tool: the envelope's
  `transforms` carry the homography or rotation with `quality` from these measurements, the
  `visual_only` projection drops `quality`, and `not_registered` and `low_texture` become the
  envelope's `not_applicable` with the summary saying which. The `aligned:<image>:<transform>`
  frame is the candidate warped by the transform's inverse.
- The thresholds (`Options`) are the spike's; the first real pairs through the tool (4.5, arm C)
  may move them, and any change is recorded here as a refinement.
- Appearance changes that invert local contrast (a negative, a strong colour-to-luminance swap)
  defeat the normalisation and register as `not_registered`; that is the honest answer for a
  format that is not intact.
- The synthetic scene and sphere generators live in `dejaview_testkit::images` for every later
  geometry test; the analytical rendering gives exact ground truth, which the resampled real
  image cannot.
