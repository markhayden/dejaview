"""Draft regions from the pixel difference between a source and an edited candidate.

A regenerated image differs from its source by sparse noise everywhere and, where the generator
drifted, by patches it was not asked to change. The intended change is the dense blob with the
most difference in it; every other dense blob is reported as a secondary change so the reviewer
looks there before confirming the item.
"""

from __future__ import annotations

import io
from dataclasses import dataclass, field

from PIL import Image, ImageChops, ImageFilter

Box = list[float]


@dataclass
class Changes:
    """Boxes as [x0, y0, x1, y1] fractions of width and height (root coordinates)."""

    primary: Box | None
    secondary: list[Box] = field(default_factory=list)
    #: True when the change covers (nearly) the whole image.
    whole: bool = False


def _components(mask: Image.Image) -> list[list[tuple[int, int]]]:
    """8-connected components of a small binary mask, as lists of (x, y)."""
    w, h = mask.size
    px = mask.load()
    seen = [[False] * w for _ in range(h)]
    out: list[list[tuple[int, int]]] = []
    for y in range(h):
        for x in range(w):
            if seen[y][x] or px[x, y] == 0:
                continue
            comp: list[tuple[int, int]] = []
            stack = [(x, y)]
            seen[y][x] = True
            while stack:
                cx, cy = stack.pop()
                comp.append((cx, cy))
                for dy in (-1, 0, 1):
                    for dx in (-1, 0, 1):
                        nx, ny = cx + dx, cy + dy
                        if 0 <= nx < w and 0 <= ny < h and not seen[ny][nx] and px[nx, ny]:
                            seen[ny][nx] = True
                            stack.append((nx, ny))
            out.append(comp)
    return out


def difference_regions(
    before: bytes,
    after: bytes,
    threshold: int = 40,
    radius: int = 6,
    blurred_threshold: int = 24,
    cell: int = 8,
    secondary_fraction: float = 0.1,
) -> Changes:
    a = Image.open(io.BytesIO(before)).convert("RGB")
    b = Image.open(io.BytesIO(after)).convert("RGB")
    if a.size != b.size:
        b = b.resize(a.size)
    w, h = a.size
    raw = ImageChops.difference(a, b).convert("L")
    strong = raw.point(lambda v: 255 if v > threshold else 0)
    dense = strong.filter(ImageFilter.BoxBlur(radius)).point(
        lambda v: 255 if v > blurred_threshold else 0
    )
    if dense.getbbox() is None:
        return Changes(primary=None)
    # Components on a coarse grid (block maximum), then each component's box and raw-difference
    # mass at full resolution.
    gw, gh = max(1, w // cell), max(1, h // cell)
    coarse = dense.resize((gw, gh), Image.Resampling.BOX).point(lambda v: 255 if v > 0 else 0)
    raw_px = raw.load()
    scored: list[tuple[float, Box]] = []
    for comp in _components(coarse):
        xs = [c[0] for c in comp]
        ys = [c[1] for c in comp]
        x0, y0 = min(xs) * cell, min(ys) * cell
        x1, y1 = min(w, (max(xs) + 1) * cell), min(h, (max(ys) + 1) * cell)
        # The exact extent is that of the strong pixels inside the component's coarse box.
        exact = strong.crop((x0, y0, x1, y1)).getbbox()
        if exact is not None:
            x0, y0, x1, y1 = x0 + exact[0], y0 + exact[1], x0 + exact[2], y0 + exact[3]
        mass = 0.0
        for cx, cy in comp:
            for yy in range(cy * cell, min(h, (cy + 1) * cell)):
                for xx in range(cx * cell, min(w, (cx + 1) * cell)):
                    v = raw_px[xx, yy]
                    if v > threshold:
                        mass += v
        box = [round(x0 / w, 4), round(y0 / h, 4), round(x1 / w, 4), round(y1 / h, 4)]
        scored.append((mass, box))
    scored.sort(key=lambda s: s[0], reverse=True)
    primary = scored[0][1]
    px0, py0, px1, py1 = primary
    if (px1 - px0) * (py1 - py0) >= 0.9:
        return Changes(primary=None, whole=True)
    secondary = [box for mass, box in scored[1:] if mass >= secondary_fraction * scored[0][0]]
    return Changes(primary=primary, secondary=secondary)


def difference_bbox(before: bytes, after: bytes, **kw) -> Box | None:
    """The primary change's box, or None when nothing changed or the change is global."""
    return difference_regions(before, after, **kw).primary
