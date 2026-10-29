# /// script
# requires-python = ">=3.12,<3.14"
# dependencies = [
#   "torch>=2.6",
#   "transformers>=4.57",
#   "onnx>=1.17",
#   "onnxruntime>=1.20",
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
backbone's DINOv2 weights Apache-2.0. The export runs on the CPU and is deterministic given the
versions it prints.
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
PREPROCESSING_VERSION = "1"


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
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)

    import numpy as np
    import onnx
    import onnxruntime as ort
    import torch
    import transformers
    from transformers import AutoImageProcessor, EomtForUniversalSegmentation

    started = time.time()
    print(
        f"torch {torch.__version__} transformers {transformers.__version__} onnx {onnx.__version__} onnxruntime {ort.__version__} python {platform.python_version()} {platform.machine()}"
    )
    torch.manual_seed(a.seed)
    processor = AutoImageProcessor.from_pretrained(MODEL_ID)
    model = EomtForUniversalSegmentation.from_pretrained(MODEL_ID).eval()
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

    record = {
        "model_id": MODEL_ID,
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
            "torch": torch.__version__,
            "transformers": transformers.__version__,
            "onnx": onnx.__version__,
            "onnxruntime": ort.__version__,
            "python": platform.python_version(),
            "machine": platform.machine(),
        },
        "execution_provider": "CPUExecutionProvider",
        "parity": worst,
        "goldens": goldens,
        "seed": a.seed,
        "elapsed_s": round(time.time() - started, 1),
    }
    (a.out / "record.json").write_text(json.dumps(record, indent=2, default=str) + "\n")
    print(f"record: {a.out / 'record.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
