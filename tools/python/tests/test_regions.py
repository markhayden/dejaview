"""`regions`: planted changes on synthetic pairs are found (needs the fetched segmenter), and the
map machinery behind it works without the model."""

import base64
import io
import json
import os
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

from dejaview_tools import models, regions, server
from dejaview_tools.context import Context

MODELS = Path(os.environ["DEJAVIEW_MODELS"]) if os.environ.get("DEJAVIEW_MODELS") else None
RECORD = models.Record.load()
PRESENT = models.artifact_path(RECORD, MODELS).is_file()
needs_model = pytest.mark.skipif(
    not PRESENT,
    reason=f"the segmenter is not under {models.artifact_path(RECORD, MODELS).parent}: run `dejaview-tools fetch`",
)


def test_components_label_each_connected_blob_once():
    mask = np.zeros((8, 10), bool)
    mask[1:3, 1:4] = True
    mask[5:7, 6:9] = True
    mask[0, 9] = True
    labels, n = regions.components(mask)
    assert n == 3
    assert len(np.unique(labels[mask])) == 3 and (labels[~mask] == 0).all()
    assert len(np.unique(labels[1:3, 1:4])) == 1


def test_box_blur_is_the_window_mean_and_normalised_luminance_removes_the_mean():
    a = np.arange(25, dtype=np.float32).reshape(5, 5)
    assert regions.box_blur(a, 1)[2, 2] == pytest.approx(12.0)
    flat = np.full((32, 32, 3), 90, np.uint8)
    assert np.abs(regions.normalised_luminance(flat)).max() < 1e-3


def scene(rng, w, h):
    """A sky over a floor with two fixed blocks, lightly noisy: the same every call for one rng."""
    img = np.zeros((h, w, 3), np.float32)
    y = np.linspace(0, 1, h)[:, None]
    img[..., 0], img[..., 1], img[..., 2] = 120 + 60 * y, 140 + 40 * y, 180 - 60 * y
    img[h // 2 :] = [110, 90, 70]
    img[int(0.6 * h) : int(0.9 * h), int(0.1 * w) : int(0.35 * w)] = (200, 30, 30)
    img[int(0.5 * h) : int(0.8 * h), int(0.6 * w) : int(0.9 * w)] = (30, 160, 40)
    return img, rng.normal(0, 2.5, img.shape)


def planted_pair(seed, w=640, h=480, changes=2):
    rng = np.random.default_rng(seed)
    base, noise = scene(rng, w, h)
    candidate = base.copy()
    planted = []
    while len(planted) < changes:
        bw, bh = rng.uniform(0.12, 0.3), rng.uniform(0.12, 0.3)
        x0, y0 = rng.uniform(0.02, 0.98 - bw), rng.uniform(0.02, 0.98 - bh)
        box = [x0, y0, x0 + bw, y0 + bh]
        # Separate boxes: two that touch are one change to any honest finder.
        if any(
            box[0] < q[2] + 0.04
            and q[0] < box[2] + 0.04
            and box[1] < q[3] + 0.04
            and q[1] < box[3] + 0.04
            for q in planted
        ):
            continue
        colour = rng.uniform(0, 255, 3)
        candidate[int(y0 * h) : int((y0 + bh) * h), int(x0 * w) : int((x0 + bw) * w)] = colour
        planted.append(box)
    to_u8 = lambda a: np.clip(a + noise, 0, 255).astype(np.uint8)
    return to_u8(candidate), to_u8(base), planted


def iou(a, b):
    ix = max(0.0, min(a[2], b[2]) - max(a[0], b[0]))
    iy = max(0.0, min(a[3], b[3]) - max(a[1], b[1]))
    inter = ix * iy
    return inter / ((a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - inter)


def handle(rgb):
    buf = io.BytesIO()
    Image.fromarray(rgb).save(buf, format="PNG")
    return {
        "media_type": "image/png",
        "width": rgb.shape[1],
        "height": rgb.shape[0],
        "projection": "flat",
        "base64": base64.b64encode(buf.getvalue()).decode("ascii"),
    }


def context(candidate, source, hints=None):
    return Context.parse(
        {
            "run_id": "run-1",
            "call_id": "call:3",
            "images": {"source": handle(source)},
            "frames": {
                "aligned:candidate:t1": {
                    "root": "candidate",
                    "projection": "flat",
                    "parent": "raw:source",
                    "transform": {
                        "id": "t1",
                        "mapping": {"homography": [1, 0, 0, 0, 1, 0, 0, 0, 1]},
                    },
                    "chain": [],
                    "pixels": handle(candidate),
                }
            },
            "hints": hints or {},
        }
    )


@needs_model
@pytest.mark.segmenter
@pytest.mark.parametrize("seed", [1, 2, 3, 4, 5])
def test_every_planted_change_is_found_within_the_cap(seed):
    candidate, source, planted = planted_pair(seed, changes=1 + seed % 3)
    env = regions.regions(
        {"frame": "aligned:candidate:t1", "against": "raw:source"}, context(candidate, source)
    )
    assert env["status"] == "ok", env["summary"]
    found = env["regions"]
    assert 1 <= len(found) <= regions.MAX_REGIONS
    for box in planted:
        best = max(iou(box, r["location"]["bbox"]) for r in found)
        assert best > 0.3, f"seed {seed}: planted {box} best IoU {best:.2f} among {found}"
    assert all(r["location"]["frame"] == "aligned:candidate:t1" for r in found)
    assert all(" -> " in r["label"] or r["label"].endswith("changed") for r in found)
    [image] = env["images"]
    assert image["frame"] == "crop:call:3:0" and image["mapping"] == {"bbox": [0.0, 0.0, 1.0, 1.0]}
    assert set(env["provenance"]) == {"tool", "version", "duration_ms", "lock_digest"}
    names = {m["name"] for m in env["measurements"]}
    assert names == {"changed_fraction", "photometric_fraction", "class_disagreement", "candidates"}


@needs_model
@pytest.mark.segmenter
def test_an_unchanged_pair_measures_nothing_and_hints_nudge_scores():
    candidate, source, _ = planted_pair(7, changes=0)
    env = regions.regions(
        {"frame": "aligned:candidate:t1", "against": "raw:source"}, context(candidate, source)
    )
    assert env["status"] == "measured_nothing", env["summary"]
    assert env["regions"] == [] and len(env["images"]) == 1
    candidate, source, _ = planted_pair(8, changes=1)
    plain = regions.regions(
        {"frame": "aligned:candidate:t1", "against": "raw:source"}, context(candidate, source)
    )
    before, after = (
        plain["regions"][0]["label"].split(" -> ")
        if " -> " in plain["regions"][0]["label"]
        else (plain["regions"][0]["label"].removesuffix(" changed"),) * 2
    )
    nudged = regions.regions(
        {"frame": "aligned:candidate:t1", "against": "raw:source"},
        context(candidate, source, {"regions": {"ignore": [before, after]}}),
    )
    assert nudged["regions"][0]["score"] <= plain["regions"][0]["score"]


def test_bad_frames_are_the_models_mistake():
    candidate, source, _ = planted_pair(9, w=64, h=48, changes=0)
    ctx = context(candidate, source)
    same = regions.regions({"frame": "raw:source", "against": "raw:source"}, ctx)
    assert same["status"] == "invalid_args" and "two different" in same["summary"]
    unknown = regions.regions({"frame": "crop:call:0:0", "against": "raw:source"}, ctx)
    assert unknown["status"] == "invalid_args" and "unknown frame id" in unknown["summary"]
    squashed = context(candidate[:, :32], source)
    grid = regions.regions({"frame": "aligned:candidate:t1", "against": "raw:source"}, squashed)
    assert grid["status"] == "invalid_args" and "one grid" in grid["summary"]


def test_the_server_lists_regions_with_its_dependencies():
    out = io.StringIO()
    server.serve(
        io.StringIO(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}) + "\n"), out
    )
    listed = {t["name"]: t for t in json.loads(out.getvalue())["result"]["tools"]}
    note = listed["regions"]["annotations"]["dejaview"]
    assert note["requires"] == ["align"] and note["frame_args"] == ["frame", "against"]
    assert note["accepts_hints"] == ["regions"] and note["returns"]["max_regions"] == 12
