"""Parity of the fetched segmenter against the published goldens, within the recorded tolerance.

These tests need the artifact and goldens that `dejaview-tools fetch` places under the models
directory (`DEJAVIEW_MODELS` points at another directory); without them they are skipped and
say so. They are marked `segmenter`.
"""

import os
from pathlib import Path

import numpy as np
import pytest

from dejaview_tools import models, segmenter

MODELS = Path(os.environ["DEJAVIEW_MODELS"]) if os.environ.get("DEJAVIEW_MODELS") else None
RECORD = models.Record.load()
PRESENT = all((models.artifact_path(RECORD, MODELS).parent / f).is_file() for f in RECORD.files)

needs_model = pytest.mark.skipif(
    not PRESENT,
    reason=f"the segmenter and its goldens are not under {models.artifact_path(RECORD, MODELS).parent}: run `dejaview-tools fetch`",
)


def test_the_semantic_map_matches_the_export_scripts_definition():
    rng = np.random.default_rng(1)
    c = rng.normal(size=(1, 4, 6)).astype(np.float32)
    m = rng.normal(size=(1, 4, 3, 3)).astype(np.float32)
    probs = np.exp(c) / np.exp(c).sum(-1, keepdims=True)
    expected = np.einsum("bqc,bqhw->bchw", probs[..., :-1], 1 / (1 + np.exp(-m))).argmax(1)
    assert (segmenter.semantic_map(c, m) == expected).all()


@needs_model
@pytest.mark.segmenter
def test_the_artifact_reproduces_every_golden_within_the_recorded_tolerance():
    seg = segmenter.Segmenter(MODELS)
    tol = RECORD.data["tolerance"]
    worst = {"class": 0.0, "mask": 0.0, "label_disagreement": 0.0}
    for golden in RECORD.data["goldens"]:
        z = np.load(models.artifact_path(RECORD, MODELS).parent / golden["file"])
        c, m = seg.run(z["pixel_values"])
        worst["class"] = max(worst["class"], float(np.abs(c - z["class_queries_logits"]).max()))
        worst["mask"] = max(worst["mask"], float(np.abs(m - z["masks_queries_logits"]).max()))
        ours = segmenter.semantic_map(c, m)
        theirs = segmenter.semantic_map(z["class_queries_logits"], z["masks_queries_logits"])
        worst["label_disagreement"] = max(
            worst["label_disagreement"], float((ours != theirs).mean())
        )
    assert all(worst[k] <= tol[k] for k in tol), f"{worst} outside {tol}"


def test_a_missing_or_wrong_sized_artifact_is_unavailable_not_a_crash(tmp_path):
    with pytest.raises(segmenter.SegmenterUnavailable, match="fetch"):
        segmenter.Segmenter(tmp_path)
    fake = models.artifact_path(RECORD, tmp_path)
    fake.parent.mkdir(parents=True)
    fake.write_bytes(b"not the model")
    with pytest.raises(segmenter.SegmenterUnavailable, match="bytes"):
        segmenter.Segmenter(tmp_path)
