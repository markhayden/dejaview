# /// script
# requires-python = ">=3.13,<3.14"
# dependencies = [
#   "torch==2.15.0",
#   "transformers==5.21.0",
#   "onnx==1.23.2",
#   "onnxruntime==1.31.1",
#   "huggingface_hub>=1.0",
#   "pillow>=11",
#   "numpy>=2",
#   "scipy>=1.14",
# ]
# ///
"""Exports the segmenter artifact (PLAN.md task 4.6, SPEC "Python tool pack") from publicly
licensed upstream weights, and records what MODEL.md needs: the upstream source, the export
procedure and versions, the artifact's SHA-256, the preprocessing, and the parity of the ONNX
export against the PyTorch model on inputs generated here. Nothing from the private prototype is
used (owner decision 11).

    uv run tools/python/scripts/export_segmenter.py --out dist/segmenter [--opset 18] [--size 512]

Upstream: `tue-mps/ade20k_semantic_eomt_large_512` on Hugging Face (EoMT, Encoder-only Mask
Transformer, DINOv2 ViT-L backbone), weights MIT, code MIT (github.com/tue-mps/eomt), the
backbone's DINOv2 weights Apache-2.0, pinned to one revision whose weights file must hash to the
value below. The export runs on the CPU and is deterministic given the pinned versions, which the
script refuses to run without (`--allow-other-versions` makes a new, differently recorded
artifact). The parity check is a gate: the script exits non-zero when the ONNX export strays from
PyTorch by more than the tolerances MODEL.md states.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import platform
import sys
import time

MODEL_ID = "tue-mps/ade20k_semantic_eomt_large_512"
# The Hugging Face revision the artifact is exported from, and the SHA-256 of its weights file.
REVISION = "aeb262f5a0bc609236d27f1277c3173ce57aeb27"
WEIGHTS_FILE = "model.safetensors"
WEIGHTS_SHA256 = "715f374b2bc9f2329caa99c9c3d425dcb072f2d0751a960534b4bfd35fe5930c"
PREPROCESSING_VERSION = "1"
# The versions the recorded artifact was made with; a different version is a different artifact.
PINNED_VERSIONS = {
    "torch": "2.15.0",
    "transformers": "5.21.0",
    "onnx": "1.23.2",
    "onnxruntime": "1.31.1",
}
# What the ONNX export may stray from PyTorch by: class logits, mask logits (max abs) and the
# share of semantic-map pixels whose label differs.
TOLERANCE = {"class": 1e-3, "mask": 1e-2, "label_disagreement": 1e-3}


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def synthetic_inputs(size: int, count: int, seed: int) -> list:
    """Images generated here: smooth colour fields with a few sharp shapes, like a scene."""
    import numpy as np
    from PIL import Image, ImageDraw

    rng = np.random.default_rng(seed)
    images = []
    for _ in range(count):
        base = np.zeros((size, size, 3), dtype=np.float32)
        yy, xx = np.mgrid[0:size, 0:size] / size
        for c in range(3):
            base[..., c] = 0.35 + 0.3 * np.sin(
                2 * np.pi * (rng.random() * xx + rng.random() * yy + rng.random())
            )
        img = Image.fromarray((base.clip(0, 1) * 255).astype(np.uint8))
        draw = ImageDraw.Draw(img)
        for _ in range(6):
            x0, y0 = rng.integers(0, size - 64, 2)
            w, h = rng.integers(32, size // 3, 2)
            colour = tuple(int(v) for v in rng.integers(0, 255, 3))
            if rng.random() < 0.5:
                draw.rectangle([x0, y0, x0 + w, y0 + h], fill=colour)
            else:
                draw.ellipse([x0, y0, x0 + w, y0 + h], fill=colour)
        images.append(img)
    return images


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--out", type=pathlib.Path, required=True, help="directory for the artifact and its record"
    )
    ap.add_argument("--opset", type=int, default=18)
    ap.add_argument("--size", type=int, default=512)
    ap.add_argument("--inputs", type=int, default=4, help="synthetic images for the parity check")
    ap.add_argument("--seed", type=int, default=20261028)
    ap.add_argument(
        "--allow-other-versions",
        action="store_true",
        help="export with versions other than the pinned ones (a new artifact, never a replacement)",
    )
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)

    import numpy as np
    import onnx
    import onnxruntime as ort
    import torch
    import transformers
    from huggingface_hub import hf_hub_download
    from transformers import AutoImageProcessor, EomtForUniversalSegmentation

    started = time.time()
    versions = {
        "torch": torch.__version__,
        "transformers": transformers.__version__,
        "onnx": onnx.__version__,
        "onnxruntime": ort.__version__,
    }
    print(
        " ".join(f"{k} {v}" for k, v in versions.items())
        + f" python {platform.python_version()} {platform.machine()}"
    )
    other = {k: (v, PINNED_VERSIONS[k]) for k, v in versions.items() if v != PINNED_VERSIONS[k]}
    if other and not a.allow_other_versions:
        for k, (have, want) in other.items():
            print(f"{k} {have} is not the pinned {want}", file=sys.stderr)
        print("refusing: the pinned versions reproduce the recorded artifact", file=sys.stderr)
        return 2

    weights = pathlib.Path(hf_hub_download(MODEL_ID, WEIGHTS_FILE, revision=REVISION))
    weights_digest = sha256(weights)
    if weights_digest != WEIGHTS_SHA256:
        print(
            f"{WEIGHTS_FILE} at revision {REVISION} hashes to {weights_digest}, not {WEIGHTS_SHA256}",
            file=sys.stderr,
        )
        return 2
    print(f"weights {WEIGHTS_FILE} sha256 {weights_digest} (revision {REVISION})")
    torch.manual_seed(a.seed)
    processor = AutoImageProcessor.from_pretrained(MODEL_ID, revision=REVISION)
    model = EomtForUniversalSegmentation.from_pretrained(MODEL_ID, revision=REVISION).eval()
    print(f"loaded {MODEL_ID}: {sum(p.numel() for p in model.parameters()) / 1e6:.1f}M parameters")
    print(
        "processor:",
        json.dumps(
            {
                k: v
                for k, v in processor.to_dict().items()
                if k
                in ("image_mean", "image_std", "size", "do_normalize", "do_rescale", "resample")
            },
            default=str,
        ),
    )

    images = synthetic_inputs(a.size, a.inputs, a.seed)
    batch = processor(images=images[:1], return_tensors="pt")
    pixel_values = batch["pixel_values"]
    print("pixel_values", tuple(pixel_values.shape), str(pixel_values.dtype))

    class Wrapped(torch.nn.Module):
        """The model's two heads as plain tensors: class logits and mask logits."""

        def __init__(self, inner):
            super().__init__()
            self.inner = inner

        def forward(self, pixel_values):
            out = self.inner(pixel_values=pixel_values)
            return out.class_queries_logits, out.masks_queries_logits

    wrapped = Wrapped(model).eval()
    artifact = a.out / f"eomt-large-ade20k-{a.size}.onnx"
    with torch.no_grad():
        torch.onnx.export(
            wrapped,
            (pixel_values,),
            str(artifact),
            opset_version=a.opset,
            input_names=["pixel_values"],
            output_names=["class_queries_logits", "masks_queries_logits"],
            dynamic_axes={
                "pixel_values": {0: "batch"},
                "class_queries_logits": {0: "batch"},
                "masks_queries_logits": {0: "batch"},
            },
            dynamo=False,
        )
    onnx.checker.check_model(str(artifact), full_check=False)
    size_mb = artifact.stat().st_size / 1e6
    digest = sha256(artifact)
    print(f"exported {artifact.name}: {size_mb:.1f} MB sha256 {digest}")

    # Parity: the ONNX export against PyTorch on every synthetic input.
    session = ort.InferenceSession(str(artifact), providers=["CPUExecutionProvider"])
    worst = {"class": 0.0, "mask": 0.0, "label_disagreement": 0.0}
    goldens = []
    for i, img in enumerate(images):
        pv = processor(images=[img], return_tensors="pt")["pixel_values"]
        with torch.no_grad():
            tc, tm = wrapped(pv)
        oc, om = session.run(None, {"pixel_values": pv.numpy()})
        worst["class"] = max(worst["class"], float(np.abs(tc.numpy() - oc).max()))
        worst["mask"] = max(worst["mask"], float(np.abs(tm.numpy() - om).max()))

        # The semantic map each side implies, and how often they disagree.
        def semantic(c, m):
            probs = torch.softmax(torch.as_tensor(c), -1)[..., :-1]
            masks = torch.sigmoid(torch.as_tensor(m))
            seg = torch.einsum("bqc,bqhw->bchw", probs, masks)
            return seg.argmax(1)

        st, so = semantic(tc, tm), semantic(oc, om)
        worst["label_disagreement"] = max(
            worst["label_disagreement"], float((st != so).float().mean())
        )
        golden = a.out / f"golden-{i}.npz"
        np.savez_compressed(
            golden, pixel_values=pv.numpy(), class_queries_logits=oc, masks_queries_logits=om
        )
        goldens.append({"file": golden.name, "sha256": sha256(golden)})
    print("parity:", json.dumps(worst))
    parity_ok = all(worst[k] <= TOLERANCE[k] for k in TOLERANCE)

    record = {
        "model_id": MODEL_ID,
        "revision": REVISION,
        "weights_file": WEIGHTS_FILE,
        "weights_sha256": weights_digest,
        "artifact": artifact.name,
        "sha256": digest,
        "size_bytes": artifact.stat().st_size,
        "opset": a.opset,
        "input": {
            "name": "pixel_values",
            "shape": ["batch", 3, a.size, a.size],
            "dtype": "float32",
        },
        "outputs": ["class_queries_logits", "masks_queries_logits"],
        "preprocessing_version": PREPROCESSING_VERSION,
        "processor": {
            k: v
            for k, v in processor.to_dict().items()
            if k in ("image_mean", "image_std", "size", "do_normalize", "do_rescale", "resample")
        },
        "versions": {
            **versions,
            "python": platform.python_version(),
            "machine": platform.machine(),
        },
        "execution_provider": "CPUExecutionProvider",
        "parity": worst,
        "tolerance": TOLERANCE,
        "parity_ok": parity_ok,
        "goldens": goldens,
        "seed": a.seed,
        "elapsed_s": round(time.time() - started, 1),
    }
    (a.out / "record.json").write_text(json.dumps(record, indent=2, default=str) + "\n")
    print(f"record: {a.out / 'record.json'}")
    if not parity_ok:
        print(f"parity outside tolerance {json.dumps(TOLERANCE)}: {json.dumps(worst)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
