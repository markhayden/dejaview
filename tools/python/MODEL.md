# Model artifacts

The pack's segmenter is an ONNX export made by this project from publicly licensed upstream
weights (SPEC "Python tool pack", PLAN task 4.6, owner decision 11: nothing from the private
prototype is used). The pack ships no model: `squint-tools fetch` downloads the artifact and
goldens recorded here into `$XDG_CACHE_HOME/squint/models/segmenter-v1/`, hashing every byte
against the record shipped inside the package (`src/squint_tools/models/segmenter-v1.json`) and
keeping nothing that does not match (task 5.3; `--from <dir or URL prefix>` fetches from
somewhere other than the release, `--models <dir>` into somewhere other than the cache). The
`segmenter`-marked parity tests run only once it has, and say so when skipped. `fetch` is the
point of verification: the segmenter checks the file's size when it loads, not its hash, since
hashing 1.2 GB at every server start would cost more than it protects.

## Upstream

| | |
|---|---|
| Model | `tue-mps/ade20k_semantic_eomt_large_512` on Hugging Face: EoMT (Encoder-only Mask Transformer, arXiv 2503.19108), large, trained for ADE20K semantic segmentation at 512 px |
| Revision | `aeb262f5a0bc609236d27f1277c3173ce57aeb27`, pinned in the export script; its `model.safetensors` must hash to SHA-256 `715f374b2bc9f2329caa99c9c3d425dcb072f2d0751a960534b4bfd35fe5930c` or the script stops |
| Backbone | DINOv2 ViT-L |
| Weights licence | MIT (the model card) |
| Code licence | MIT (github.com/tue-mps/eomt, `LICENSE`) |
| Backbone weights licence | Apache-2.0 (DINOv2) |
| Loaded through | `transformers.EomtForUniversalSegmentation` and `AutoImageProcessor` |

## Export

`scripts/export_segmenter.py` (run as `uv run tools/python/scripts/export_segmenter.py --out
<dir>`; its dependencies are declared inline, pinned to the exact versions below, and never join
the pack's own lock) refuses to run under other versions (`--allow-other-versions` makes a new
artifact, never a replacement), downloads the pinned revision and checks the weights hash, loads
the upstream model, wraps its two heads as plain tensors, exports with `torch.onnx.export` (opset
18, the TorchScript exporter, `batch` dynamic), checks the ONNX model, runs it beside the PyTorch
model on four synthetic scenes generated in the script from a seed, writes `record.json` with
everything below, and exits non-zero when the parity is outside the tolerance. The record of the
recorded artifact is committed inside the package as
`tools/python/src/squint_tools/models/segmenter-v1.json`, where `fetch` and the segmenter read
it.

| | |
|---|---|
| Artifact | `eomt-large-ade20k-512.onnx`, 1,264,419,839 bytes |
| SHA-256 | `184ad11695a51bf67f91d2cf474670894ab8b5c98fdcfcd88e9fce1e23491b85` |
| Input | `pixel_values`, float32 `[batch, 3, 512, 512]`: square only. The processor resizes the shorter edge to 512 and the export fixes both spatial sizes, so the tools pad: the longer edge is resized to 512, the shorter padded to 512 with the image's mean colour, centred, and the semantic map is read back from the same placement (`segmenter.prepare`, preprocessing `1-pad`; task 5.4a) |
| Outputs | `class_queries_logits` (per query, 150 ADE20K classes plus no-object), `masks_queries_logits` (per query, a mask at a quarter of the input) |
| Preprocessing version | `1`: the image's shorter edge resized to 512 (bilinear), scaled to `[0, 1]`, normalised with mean `[0.485, 0.456, 0.406]` and std `[0.229, 0.224, 0.225]`; the semantic map is `softmax(class)[..., :-1]` combined with `sigmoid(masks)` and the argmax over classes |
| Export versions | torch 2.15.0, transformers 5.21.0, onnx 1.23.2, Python 3.13.3, arm64 (macOS); pinned in the script and recorded on every run |
| Runtime | onnxruntime 1.31.1, `CPUExecutionProvider` |
| Parity | ONNX against PyTorch on the four synthetic scenes: class logits within 6.5e-5, mask logits within 1.8e-3, no pixel of the semantic map differs. The tolerance the script and the pack's parity test hold the artifact to: class logits within 1e-3, mask logits within 1e-2, at most 0.1% of semantic-map pixels differing |
| Goldens | `golden-0.npz` to `golden-3.npz`: the preprocessed inputs and the ONNX outputs for the four scenes; about 6 MB each, published beside the artifact, never committed. SHA-256: `b018a0634642aac14d4d0818d721415b199b0e505449c3e4813f06739da595a0`, `5c45eb12e06c3dedb07e900ad2b865088a56ebd5d8a66dc83ce7dfb564480c18`, `0c0e141483f69fa3adfdfb14e5624edfd2fc9380628c6a73bfab90eeffb94ffd`, `1b763191f7ef0feb8f304e167b5199623e1c5e1ecba035f4f49ae605dbfca926` |
| Seed | 20261028 for the synthetic scenes |

## Publication

Pending: the artifact, the four goldens and `record.json` wait in the gitignored
`tools/python/dist/segmenter/` for the owner to upload them as release assets of the harness
repository under the tag `segmenter-v1`. Once uploaded, the download URL is:

```
https://github.com/markhayden/squint/releases/download/segmenter-v1/eomt-large-ade20k-512.onnx
```

`fetch` downloads from that URL and the goldens beside it, and refuses any byte that does not
hash to the values above. Until the assets are uploaded, `squint-tools fetch --from
tools/python/dist/segmenter` fills the cache from the export's output, under the same checks.
Re-running the export script with the versions above reproduces the artifact's hash; a
different torch or transformers version is a new artifact with a new row here, never a silent
replacement.
