"""The draft region of an edit is the dense change, not the sparse noise a regeneration leaves."""

import io
import random

from PIL import Image

from squint_sets import regions


def png(im):
    buf = io.BytesIO()
    im.save(buf, format="PNG")
    return buf.getvalue()


def test_a_dense_patch_is_boxed_and_sparse_noise_is_ignored():
    rng = random.Random(1)
    before = Image.new("RGB", (400, 300), (120, 120, 120))
    after = before.copy()
    # One real change: a 60x40 patch at (200, 100).
    after.paste((250, 30, 30), (200, 100, 260, 140))
    # Sparse noise everywhere, as a regenerated image has: 3% of pixels shifted by up to 90.
    px = after.load()
    for _ in range(int(400 * 300 * 0.03)):
        x, y = rng.randrange(400), rng.randrange(300)
        r, g, b = px[x, y]
        d = rng.choice((-90, 90))
        px[x, y] = (max(0, min(255, r + d)), g, b)
    box = regions.difference_bbox(png(before), png(after))
    assert box is not None, "the noise must not turn the region into whole-image"
    x0, y0, x1, y1 = box
    assert abs(x0 - 200 / 400) < 0.04 and abs(y0 - 100 / 300) < 0.05, box
    assert abs(x1 - 260 / 400) < 0.04 and abs(y1 - 140 / 300) < 0.05, box


def test_a_second_dense_patch_is_reported_as_a_secondary_change():
    before = Image.new("RGB", (400, 300), (120, 120, 120))
    after = before.copy()
    after.paste((250, 30, 30), (200, 100, 260, 140))  # the intended change
    after.paste((30, 30, 250), (20, 20, 40, 40))  # drift elsewhere, smaller
    changes = regions.difference_regions(png(before), png(after))
    assert changes.primary is not None and abs(changes.primary[0] - 0.5) < 0.04
    assert len(changes.secondary) == 1
    assert (
        abs(changes.secondary[0][0] - 0.05) < 0.04
        and abs(changes.secondary[0][1] - 20 / 300) < 0.05
    )


def test_no_change_and_a_global_change_are_none():
    before = Image.new("RGB", (100, 100), (10, 200, 10))
    assert regions.difference_bbox(png(before), png(before)) is None
    after = Image.new("RGB", (100, 100), (200, 10, 10))
    assert regions.difference_bbox(png(before), png(after)) is None
