"""`classes`: the named fixtures return exactly the recorded class lists (needs the fetched
segmenter), and the manifest and argument checks hold without it.

The fixtures' class lists were recorded from this code on the real segmenter, so that test pins
behaviour against regression; the correctness evidence for the segmenter is the parity test and
the planted-change test of `regions`."""

import base64
import io
import json
import os
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

from dejaview_tools import classes, models, server
from dejaview_tools.context import Context

FIXTURES = Path(__file__).parent / "fixtures" / "classes"
MODELS = Path(os.environ["DEJAVIEW_MODELS"]) if os.environ.get("DEJAVIEW_MODELS") else None
RECORD = models.Record.load()
PRESENT = models.artifact_path(RECORD, MODELS).is_file()
needs_model = pytest.mark.skipif(
    not PRESENT,
    reason=f"the segmenter is not under {models.artifact_path(RECORD, MODELS).parent}: run `dejaview-tools fetch`",
)


def handle(path: Path):
    img = Image.open(path).convert("RGB")
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return {
        "media_type": "image/png",
        "width": img.width,
        "height": img.height,
        "projection": "flat",
        "base64": base64.b64encode(buf.getvalue()).decode("ascii"),
    }


def context(name: str) -> Context:
    return Context.parse(
        {
            "run_id": "run-1",
            "call_id": "call:4",
            "images": {"source": handle(FIXTURES / f"{name}-source.png")},
            "frames": {
                "aligned:candidate:t1": {
                    "root": "candidate",
                    "projection": "flat",
                    "parent": "raw:source",
                    "transform": None,
                    "chain": [],
                    "pixels": handle(FIXTURES / f"{name}-candidate.png"),
                }
            },
            "hints": {},
        }
    )


def test_composition_orders_by_share_and_folds_the_small_away():
    names = ["a", "b", "c", "d"]
    labels = np.array([[0] * 60 + [1] * 30 + [2] * 9 + [3] * 1])
    assert classes.composition(labels, names) == [("a", 0.6), ("b", 0.3), ("c", 0.09)]
    assert classes.composition(np.array([[3]]), names) == [("d", 1.0)]
    # Five classes above the share floor fold to the four largest.
    five = np.array([[0] * 24 + [1] * 22 + [2] * 20 + [3] * 18 + [4] * 16])
    assert [n for n, _ in classes.composition(five, ["a", "b", "c", "d", "e"])] == [
        "a",
        "b",
        "c",
        "d",
    ]


class FakeSegmenter:
    """Segments by brightness: four bands of the padded square map to classes 0 to 3, so a test
    can plan what each side says without the model."""

    def __init__(self, bands=4):
        self.bands = bands

    def segment(self, images):
        from dejaview_tools.segmenter import prepare, semantic_map  # noqa: F401

        out = []
        for rgb in images:
            pv, place = prepare(rgb)
            gray = (
                pv * np.array([0.229, 0.224, 0.225])[:, None, None]
                + np.array([0.485, 0.456, 0.406])[:, None, None]
            ).mean(0)
            quarter = gray[::4, ::4]
            labels = np.clip((quarter * self.bands).astype(int), 0, self.bands - 1)
            out.append((labels, np.full(labels.shape, 0.9), place))
        return out


def test_four_classes_a_side_keep_both_confidences_under_the_measurement_cap(monkeypatch):
    monkeypatch.setattr(classes.segmenter, "shared", lambda: FakeSegmenter())
    monkeypatch.setattr(
        classes.segmenter, "class_names", lambda: ["dark", "dim", "light", "bright"] + ["x"] * 146
    )
    # Four vertical stripes of brightness on each side, so every side has four classes.
    stripes = np.zeros((64, 64, 3), np.uint8)
    for n, v in enumerate((20, 90, 160, 235)):
        stripes[:, n * 16 : (n + 1) * 16] = v
    ctx = Context.parse(
        {
            "run_id": "r",
            "call_id": "c",
            "images": {"source": _handle(stripes[:, ::-1])},
            "frames": {
                "aligned:candidate:t1": {
                    "root": "candidate",
                    "projection": "flat",
                    "parent": "raw:source",
                    "transform": None,
                    "chain": [],
                    "pixels": _handle(stripes),
                }
            },
            "hints": {},
        }
    )
    env = classes.classes(
        {"frame": "aligned:candidate:t1", "against": "raw:source", "bbox": [0.0, 0.0, 1.0, 1.0]},
        ctx,
    )
    names = [m["name"] for m in env["measurements"]]
    assert len(names) == classes.RETURNS["max_measurements"]
    assert "confidence_before" in names and "confidence_after" in names
    assert (
        sum(n.startswith("before:") for n in names) == 3
        and sum(n.startswith("after:") for n in names) == 3
    )
    assert env["provenance"]["version"] == classes.MANIFEST["version"]


def _handle(rgb):
    buf = io.BytesIO()
    Image.fromarray(rgb).save(buf, format="PNG")
    return {
        "media_type": "image/png",
        "width": rgb.shape[1],
        "height": rgb.shape[0],
        "projection": "flat",
        "base64": base64.b64encode(buf.getvalue()).decode("ascii"),
    }


@needs_model
@pytest.mark.segmenter
@pytest.mark.parametrize("name", sorted(p.stem for p in FIXTURES.glob("*.json")))
def test_named_fixtures_return_their_recorded_class_lists(name):
    expected = json.loads((FIXTURES / f"{name}.json").read_text())
    env = classes.classes(
        {"frame": "aligned:candidate:t1", "against": "raw:source", "bbox": expected["bbox"]},
        context(name),
    )
    assert env["status"] == "ok", env["summary"]
    got = {
        side: [
            m["name"].split(":", 1)[1]
            for m in env["measurements"]
            if m["name"].startswith(f"{side}:")
        ]
        for side in ("before", "after")
    }
    assert got == expected["classes"], env["summary"]
    assert [r["label"] for r in env["regions"]] == expected["regions"]
    assert env["regions"] == [] or env["regions"][0]["location"]["bbox"] == expected["bbox"]


def test_bad_arguments_are_the_models_mistake():
    ctx = context("block-on-floor")
    args = {"frame": "aligned:candidate:t1", "against": "raw:source"}
    assert "bbox" in classes.classes(args, ctx)["summary"]
    assert (
        "two different"
        in classes.classes({**args, "against": args["frame"], "bbox": [0, 0, 1, 1]}, ctx)["summary"]
    )
    assert (
        "unknown frame id"
        in classes.classes({**args, "frame": "crop:x:0", "bbox": [0, 0, 1, 1]}, ctx)["summary"]
    )


def test_the_server_lists_classes_as_a_class_difference_tool():
    out = io.StringIO()
    server.serve(
        io.StringIO(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}) + "\n"), out
    )
    listed = {t["name"]: t for t in json.loads(out.getvalue())["result"]["tools"]}
    note = listed["classes"]["annotations"]["dejaview"]
    assert note["evidence_kind"] == "class_difference" and note["requires"] == ["align"]
    assert listed["classes"]["inputSchema"]["required"] == ["frame", "against", "bbox", "_dejaview"]
