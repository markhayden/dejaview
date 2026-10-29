"""The ``_squint`` context: images and frames with pixels by path and inline."""

import base64
import io

import numpy as np
import pytest
from PIL import Image

from squint_tools.context import Context, ContextError


def png_bytes(width, height, value):
    buf = io.BytesIO()
    Image.fromarray(np.full((height, width, 3), value, dtype=np.uint8)).save(buf, format="PNG")
    return buf.getvalue()


def handle(width, height, **where):
    return {
        "media_type": "image/png",
        "width": width,
        "height": height,
        "projection": "flat",
        **where,
    }


def test_images_and_frames_load_their_pixels_by_path_and_inline(tmp_path):
    path = tmp_path / "raw_candidate.png"
    path.write_bytes(png_bytes(8, 6, 10))
    inline = base64.b64encode(png_bytes(4, 3, 200)).decode("ascii")
    ctx = Context.parse(
        {
            "run_id": "run-1",
            "call_id": "call:7",
            "images": {"candidate": handle(8, 6, path=str(path))},
            "frames": {
                "aligned:candidate:t1": {
                    "root": "candidate",
                    "projection": "flat",
                    "parent": "raw:source",
                    "transform": {
                        "id": "t1",
                        "mapping": {"homography": [1, 0, 0, 0, 1, 0, 0, 0, 1]},
                    },
                    "chain": [{"frame": "aligned:candidate:t1", "parent": "raw:source"}],
                    "pixels": handle(4, 3, base64=inline),
                }
            },
            "hints": {},
        }
    )
    assert ctx.call_id == "call:7"
    assert ctx.images["candidate"].load().shape == (6, 8, 3)
    assert ctx.images["candidate"].load()[0, 0].tolist() == [10, 10, 10]
    frame = ctx.frames["aligned:candidate:t1"]
    assert frame.root == "candidate" and frame.parent == "raw:source"
    assert frame.pixels.load().shape == (3, 4, 3)
    # A raw frame id reaches the image's pixels; an unknown frame reaches nothing.
    assert ctx.frame_pixels("raw:candidate") is ctx.images["candidate"]
    assert ctx.frame_pixels("aligned:candidate:t1") is frame.pixels
    assert ctx.frame_pixels("crop:call:1:0") is None


def test_a_malformed_context_or_mismatched_pixels_are_refused(tmp_path):
    with pytest.raises(ContextError):
        Context.parse("not an object")
    with pytest.raises(ContextError):
        Context.parse({"run_id": "r", "images": {"candidate": {"width": 1}}})
    path = tmp_path / "raw_candidate.png"
    path.write_bytes(png_bytes(8, 6, 10))
    ctx = Context.parse(
        {"run_id": "r", "call_id": "c", "images": {"candidate": handle(9, 6, path=str(path))}}
    )
    with pytest.raises(ContextError):
        ctx.images["candidate"].load()
