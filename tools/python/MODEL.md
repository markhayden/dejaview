# Model artifacts

The pack's segmenter is an ONNX export made by this project from publicly licensed upstream
weights (SPEC "Python tool pack", PLAN task 4.6, owner decision 11: nothing from the private
prototype is used). This version of the pack (0.0.1) still ships no model and `dejaview-tools
fetch` is a no-op; task 5.3 makes `fetch` download and verify the artifact recorded here, and the
`segmenter`-marked parity tests run only once it has.

## Upstream

| | |
|---|---|
| Model | `tue-mps/ade20k_semantic_eomt_large_512` on Hugging Face: EoMT (Encoder-only Mask Transformer, arXiv 2503.19108), large, trained for ADE20K semantic segmentation at 512 px |
| Backbone | DINOv2 ViT-L |
| Weights licence | MIT (the model card) |
| Code licence | MIT (github.com/tue-mps/eomt, `LICENSE`) |
| Backbone weights licence | Apache-2.0 (DINOv2) |
| Loaded through | `transformers.EomtForUniversalSegmentation` and `AutoImageProcessor` |

## Export

`scripts/export_segmenter.py` (run as `uv run tools/python/scripts/export_segmenter.py --out
<dir>`; its dependencies are declared inline and never join the pack's own lock) loads the
upstream model, wraps its two heads as plain tensors, exports with `torch.onnx.export` (opset 18,
the TorchScript exporter, `batch` dynamic), checks the ONNX model, runs it beside the PyTorch
model on four synthetic scenes generated in the script from a seed, and writes `record.json`
with everything below.

| | |
|---|---|
| Artifact | `eomt-large-ade20k-512.onnx`, 1,264,419,839 bytes |
| SHA-256 | `184ad11695a51bf67f91d2cf474670894ab8b5c98fdcfcd88e9fce1e23491b85` |
| Input | `pixel_values`, float32 `[batch, 3, 512, 512]` |
| Outputs | `class_queries_logits` (per query, 150 ADE20K classes plus no-object), `masks_queries_logits` (per query, a mask at a quarter of the input) |
| Preprocessing version | `1`: the image's shorter edge resized to 512 (bilinear), scaled to `[0, 1]`, normalised with mean `[0.485, 0.456, 0.406]` and std `[0.229, 0.224, 0.225]`; the semantic map is `softmax(class)[..., :-1]` combined with `sigmoid(masks)` and the argmax over classes |
| Export versions | torch 2.15.0, transformers 5.21.0, onnx 1.23.2, Python 3.13.3, arm64 (macOS); recorded by the script on every run |
| Runtime | onnxruntime 1.31.1, `CPUExecutionProvider` |
| Parity | ONNX against PyTorch on the four synthetic scenes: class logits within 6.5e-5, mask logits within 1.8e-3, no pixel of the semantic map differs. The tolerance the parity test holds the artifact to: class logits within 1e-3, mask logits within 1e-2, at most 0.1% of semantic-map pixels differing |
| Goldens | `golden-0.npz` to `golden-3.npz`: the preprocessed inputs and the ONNX outputs for the four scenes, with SHA-256 `b018a063…595a0`, `5c45eb12…80c18`, `0c0e1414…94ffd`, `1b763191…fca926` (in full in `record.json`); about 6 MB each, published beside the artifact, never committed |
| Seed | 20261028 for the synthetic scenes |

## Publication

The artifact, the four goldens and `record.json` are release assets of the harness repository
under the tag `segmenter-v1`, uploaded by the owner from `tools/python/dist/segmenter/`:

```
https://github.com/markhayden/dejaview/releases/download/segmenter-v1/eomt-large-ade20k-512.onnx
```

`fetch` (task 5.3) downloads from that URL and refuses any byte that does not hash to the value
above. Re-running the export script with the versions above reproduces the artifact's hash; a
different torch or transformers version is a new artifact with a new row here, never a silent
replacement.
