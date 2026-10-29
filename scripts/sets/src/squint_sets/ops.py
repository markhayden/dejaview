"""Construction operations: each turns a source image into a candidate and says what the change
means under the rules, which is the draft label. Deterministic ops need no generator; `edit` asks
the family's generator to change one thing and draws the region from the pixel difference.
"""

from __future__ import annotations

import io
from dataclasses import dataclass, field
from typing import Any

from PIL import Image, ImageEnhance

from . import regions

#: Reality-check defects an edit can plant, and the rule each one breaks.
REALITY_DEFECTS = {"anatomy": "RE1", "shadow": "RE2", "text": "RE3", "merge": "RE4"}


@dataclass
class DraftLabel:
    expected: str
    failure_type: str | None
    obligation_ids: list[str]
    words: str
    region_kind: str | None = None
    bbox: list[float] | None = None
    notes: dict[str, Any] = field(default_factory=dict)


def _load(b: bytes) -> Image.Image:
    return Image.open(io.BytesIO(b)).convert("RGB")


def _png(im: Image.Image) -> bytes:
    buf = io.BytesIO()
    im.save(buf, format="PNG")
    return buf.getvalue()


def apply(op: dict[str, Any], source: bytes, generator, seed: int) -> tuple[bytes, DraftLabel]:
    kind = op.get("kind")
    if kind == "as_is":
        return source, DraftLabel(
            expected="pass",
            failure_type=None,
            obligation_ids=[],
            words="the generated image as it is; confirm nothing in it breaks the rules",
        )
    if kind == "generate":
        # The candidate from its own prompt (reality checks: a defect the generators will draw when
        # asked for it directly but ignore when asked to edit it in). No region: whole image.
        if generator is None:
            raise ValueError("generate: the family's generator is needed")
        prompt = op["prompt"]
        defect = op.get("defect")
        out = generator.generate(prompt, seed, int(op.get("size", 1024)))
        if defect:
            rule = REALITY_DEFECTS.get(defect)
            if rule is None:
                raise ValueError(
                    f"generate: defect must be one of {sorted(REALITY_DEFECTS)}, got {defect!r}"
                )
            return out, DraftLabel(
                expected="fail",
                failure_type=defect,
                obligation_ids=[rule],
                words=op.get("words") or f"a generated {defect} defect: {prompt}",
                region_kind="whole_image",
                notes={"prompt": prompt},
            )
        return out, DraftLabel(
            expected="pass",
            failure_type=None,
            obligation_ids=[],
            words="generated clean from its own prompt; confirm nothing in it breaks the rules",
            notes={"prompt": prompt},
        )
    if kind == "grade":
        warmth = float(op.get("warmth", 0.1))
        im = _load(source)
        r, g, b = im.split()
        r = r.point(lambda v: min(255, int(v * (1 + warmth))))
        b = b.point(lambda v: max(0, int(v * (1 - warmth))))
        im = Image.merge("RGB", (r, g, b))
        im = ImageEnhance.Contrast(im).enhance(1 + warmth / 2)
        return _png(im), DraftLabel(
            expected="pass",
            failure_type=None,
            obligation_ids=[],
            words=f"a mild colour grade (warmth {warmth:+.2f}); structure and subjects unchanged",
        )
    if kind == "mirror":
        im = _load(source).transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        return _png(im), DraftLabel(
            expected="fail",
            failure_type="mirrored",
            obligation_ids=["SF2"],
            words="the candidate is the source mirrored left to right",
            region_kind="whole_image",
        )
    if kind == "rotate":
        degrees = int(op.get("degrees", 90))
        if degrees not in (90, 180, 270):
            raise ValueError(f"rotate: degrees must be 90, 180 or 270, got {degrees}")
        im = _load(source).rotate(degrees, expand=True)
        return _png(im), DraftLabel(
            expected="fail",
            failure_type="rotated",
            obligation_ids=["SF2"],
            words=f"the candidate is the source rotated by {degrees} degrees",
            region_kind="whole_image",
        )
    if kind == "crop":
        keep = float(op.get("keep", 0.75))
        if not 0.2 <= keep < 1.0:
            raise ValueError(f"crop: keep must be in [0.2, 1), got {keep}")
        im = _load(source)
        w, h = im.size
        cw, ch = round(w * keep), round(h * keep)
        x0, y0 = (w - cw) // 2, (h - ch) // 2
        im = im.crop((x0, y0, x0 + cw, y0 + ch))
        return _png(im), DraftLabel(
            expected="fail",
            failure_type="cropped",
            obligation_ids=["SF2"],
            words=f"the candidate is the central {int(keep * 100)}% of the source; the edges are cropped away",
            region_kind="whole_image",
        )
    if kind == "restyle":
        if generator is None:
            raise ValueError("restyle: the family's generator is needed")
        prompt = op["prompt"]
        out = generator.edit(source, prompt, seed)
        changes = regions.difference_regions(source, out)
        label = DraftLabel(
            expected="pass",
            failure_type=None,
            obligation_ids=[],
            words=f"a permitted restyle ({prompt.split('.')[0].strip().lower()}); structure and every protected subject must be unchanged, confirm against the source",
            notes={"prompt": prompt},
        )
        label.notes["changed_area"] = (
            "whole_image" if changes.whole or changes.primary is None else changes.primary
        )
        if changes.secondary:
            label.notes["secondary_changes"] = changes.secondary
        return out, label
    if kind == "edit":
        if generator is None:
            raise ValueError("edit: the family's generator is needed")
        prompt = op["prompt"]
        removes = op.get("removes")
        alters = op.get("alters")
        defect = op.get("defect")
        out = generator.edit(source, prompt, seed)
        changes = regions.difference_regions(source, out)
        box = changes.primary
        if defect:
            # A planted reality-check defect: the rule it breaks is the defect's.
            rule = REALITY_DEFECTS.get(defect)
            if rule is None:
                raise ValueError(
                    f"edit: defect must be one of {sorted(REALITY_DEFECTS)}, got {defect!r}"
                )
            words = op.get("words") or f"a planted {defect} defect: {prompt}"
            failure_type = defect
            obligations = [rule]
        else:
            if removes:
                words = f"the {removes} is gone from the candidate"
            elif alters:
                words = f"the {alters} is changed in the candidate"
            else:
                words = f"edited: {prompt}"
            failure_type = (
                f"{removes or alters or 'subject'}-changed" if (removes or alters) else "edited"
            )
            obligations = ["SF1"]
        label = DraftLabel(
            expected="fail",
            failure_type=failure_type,
            obligation_ids=obligations,
            words=words,
            notes={"prompt": prompt},
        )
        if box is None:
            label.region_kind = "whole_image"
        else:
            label.region_kind = "bbox"
            label.bbox = list(box)
        if changes.secondary:
            label.notes["secondary_changes"] = changes.secondary
            label.words += (
                f"; the generator also changed {len(changes.secondary)} other area(s), check them"
            )
        return out, label
    raise ValueError(f"unknown op kind {kind!r}")
