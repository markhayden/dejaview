"""Deterministic construction operations and the draft label each one implies."""

import io

from PIL import Image

from dejaview_sets import ops


def png(w=64, h=48, colour=(200, 40, 40)):
    im = Image.new("RGB", (w, h), colour)
    # A distinct patch so flips and rotations are observable.
    im.paste((40, 40, 200), (0, 0, 16, 12))
    buf = io.BytesIO()
    im.save(buf, format="PNG")
    return buf.getvalue()


def load(b):
    return Image.open(io.BytesIO(b)).convert("RGB")


def test_mirror_flips_horizontally_and_is_a_structural_fail():
    out, label = ops.apply({"kind": "mirror"}, png(), generator=None, seed=1)
    im = load(out)
    assert im.size == (64, 48)
    assert im.getpixel((63, 0)) == (40, 40, 200), "the patch moved to the right edge"
    assert label.expected == "fail"
    assert label.obligation_ids == ["SF2"]
    assert label.region_kind == "whole_image"
    assert label.failure_type == "mirrored"


def test_rotate_changes_dimensions_for_90_degrees():
    out, label = ops.apply({"kind": "rotate", "degrees": 90}, png(), generator=None, seed=1)
    assert load(out).size == (48, 64)
    assert label.expected == "fail" and label.failure_type == "rotated"


def test_crop_keeps_a_fraction_and_fails_structure():
    out, label = ops.apply({"kind": "crop", "keep": 0.75}, png(), generator=None, seed=1)
    assert load(out).size == (48, 36)
    assert label.expected == "fail" and label.obligation_ids == ["SF2"]


def test_grade_is_a_mild_permitted_change():
    out, label = ops.apply({"kind": "grade", "warmth": 0.1}, png(), generator=None, seed=1)
    im = load(out)
    assert im.size == (64, 48)
    r, _g, b = im.getpixel((40, 40))
    assert r >= 200 and b < 40, "warmer: red up, blue down"
    assert label.expected == "pass"
    assert label.obligation_ids == []
    assert label.region_kind is None


class FakeGen:
    name = "fake"
    model = "fake-model"

    def edit(self, image, prompt, seed):
        im = load(image)
        im.paste((0, 255, 0), (20, 20, 40, 40))
        buf = io.BytesIO()
        im.save(buf, format="PNG")
        return buf.getvalue()


def test_edit_uses_the_generator_and_draws_the_region_from_the_pixel_difference():
    out, label = ops.apply(
        {
            "kind": "edit",
            "prompt": "Remove the bicycle; change nothing else.",
            "removes": "bicycle",
        },
        png(),
        generator=FakeGen(),
        seed=7,
    )
    assert load(out).getpixel((30, 30)) == (0, 255, 0)
    assert label.expected == "fail" and label.obligation_ids == ["SF1"]
    assert label.region_kind == "bbox"
    x0, y0, x1, y1 = label.bbox
    assert abs(x0 - 20 / 64) < 0.15 and abs(y0 - 20 / 48) < 0.2, label.bbox
    assert abs(x1 - 40 / 64) < 0.15 and abs(y1 - 40 / 48) < 0.2, label.bbox
    assert "bicycle" in label.words


def test_unknown_op_is_refused():
    try:
        ops.apply({"kind": "explode"}, png(), generator=None, seed=1)
    except ValueError as e:
        assert "explode" in str(e)
    else:
        raise AssertionError("accepted an unknown op")
