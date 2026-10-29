"""Result envelopes (SPEC "Results"): the structured result every tool returns, and the images it
carries, which travel to the harness as MCP image blocks beside the envelope."""

from __future__ import annotations

import base64
import io
import os
from typing import Any

import numpy as np
from PIL import Image

from . import __version__


def provenance(tool: str, duration_ms: int = 0) -> dict[str, Any]:
    return {
        "tool": tool,
        "version": __version__,
        "duration_ms": duration_ms,
        "lock_digest": os.environ.get("DEJAVIEW_LOCK_DIGEST"),
    }


def envelope(
    tool: str,
    status: str,
    summary: str,
    *,
    regions: list[dict[str, Any]] | None = None,
    measurements: list[dict[str, Any]] | None = None,
    images: list[dict[str, Any]] | None = None,
    transforms: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    return {
        "envelope_version": 1,
        "status": status,
        "summary": summary,
        "measurements": measurements or [],
        "regions": regions or [],
        "images": images or [],
        "transforms": transforms or [],
        "provenance": provenance(tool),
    }


def invalid_args(tool: str, message: str) -> dict[str, Any]:
    """The model's mistake: the summary tells it what to send instead."""
    return envelope(tool, "invalid_args", message)


def bbox_location(frame: str, bbox: list[float]) -> dict[str, Any]:
    return {
        "frame": frame,
        "kind": "bbox",
        "bbox": [float(v) for v in bbox],
        "polygon": None,
        "yaw": None,
        "pitch": None,
        "fov": None,
    }


def png_image(
    frame: str, parent: str, mapping: dict[str, Any], pixels: np.ndarray
) -> dict[str, Any]:
    """A returned image: lossless PNG, named as the harness requires (``crop:<call id>:<n>`` or an
    aligned frame), with how it sits in its parent."""
    buf = io.BytesIO()
    Image.fromarray(np.ascontiguousarray(pixels, dtype=np.uint8), "RGB").save(buf, format="PNG")
    height, width = pixels.shape[:2]
    return {
        "frame": frame,
        "parent": parent,
        "mapping": mapping,
        "media_type": "image/png",
        "width": int(width),
        "height": int(height),
        "base64": base64.b64encode(buf.getvalue()).decode("ascii"),
    }


def split_images(env: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """The envelope with its images' bytes removed, and those bytes as MCP image blocks in the
    same order: the harness pairs block *n* with image *n* by position and media type."""
    blocks = []
    stripped = []
    for img in env.get("images", []):
        blocks.append({"type": "image", "data": img["base64"], "mimeType": img["media_type"]})
        stripped.append({k: v for k, v in img.items() if k != "base64"})
    return {**env, "images": stripped}, blocks
