"""`classes`: the named fixtures return exactly the recorded class lists (needs the fetched
segmenter), and the manifest and argument checks hold without it."""

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
    assert classes.composition(labels, np.ones_like(labels, float), names) == [
        ("a", 0.6),
        ("b", 0.3),
        ("c", 0.09),
    ]
    assert classes.composition(np.array([[3]]), np.ones((1, 1)), names) == [("d", 1.0)]


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
