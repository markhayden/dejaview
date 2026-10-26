"""The tools this pack serves: their dejaview manifests and handlers."""

from __future__ import annotations

import os
from typing import Any

from . import __version__

RETURNS = {
    "max_images": 0,
    "max_long_edge": 0,
    "max_image_bytes": 0,
    "max_summary_bytes": 2048,
    "max_regions": 4,
    "max_region_label_bytes": 120,
    "max_measurements": 4,
    "max_transforms": 0,
    "max_envelope_bytes": 65_536,
}

ECHO_REGION_MODEL_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": ["image", "bbox"],
    "properties": {
        "image": {"type": "string", "description": "The image id to place the region on."},
        "bbox": {
            "type": "array",
            "items": {"type": "number"},
            "minItems": 4,
            "maxItems": 4,
            "description": "Normalised [x0, y0, x1, y1] in the image's raw frame.",
        },
    },
}

ECHO_REGION = {
    "manifest_version": 1,
    "name": "echo_region",
    "version": __version__,
    "purpose": "Echoes a box on an image as a change region, to prove the pack transport.",
    "when_to_use": "Only to exercise the tool pack.",
    "when_not_to_use": "For any judgement about an image.",
    "output_guidance": "The region is exactly the box given; it says nothing about the image.",
    "cost": {"tier": "cheap", "seconds": 0.0},
    "requires": [],
    "pairs_with": [],
    "image_args": ["image"],
    "frame_args": [],
    "accepts_hints": [],
    "evidence_kind": "change_region",
    "returns": RETURNS,
    "frames_out": [],
    "concurrent": False,
    "preconditions": "",
    "model_schema": ECHO_REGION_MODEL_SCHEMA,
    "input_schema": {
        **ECHO_REGION_MODEL_SCHEMA,
        "required": ["image", "bbox", "_dejaview"],
        "properties": {**ECHO_REGION_MODEL_SCHEMA["properties"], "_dejaview": {"type": "object"}},
    },
    "output_schema": {"type": "object"},
}

MANIFESTS = [ECHO_REGION]


def provenance(tool: str) -> dict[str, Any]:
    return {
        "tool": tool,
        "version": __version__,
        "duration_ms": 0,
        "lock_digest": os.environ.get("DEJAVIEW_LOCK_DIGEST"),
    }


def envelope(tool: str, status: str, summary: str, regions: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "envelope_version": 1,
        "status": status,
        "summary": summary,
        "measurements": [],
        "regions": regions,
        "images": [],
        "transforms": [],
        "provenance": provenance(tool),
    }


def invalid_args(tool: str, message: str) -> dict[str, Any]:
    return envelope(tool, "invalid_args", message, [])


def echo_region(args: dict[str, Any], context: dict[str, Any]) -> dict[str, Any]:
    """One region at the given box on the named image's raw frame."""
    image = args.get("image")
    bbox = args.get("bbox")
    if not isinstance(image, str) or image not in context.get("images", {}):
        return invalid_args("echo_region", f"image: unknown image id {image!r}")
    if (
        not isinstance(bbox, list)
        or len(bbox) != 4
        or not all(isinstance(v, (int, float)) for v in bbox)
        or not (0.0 <= bbox[0] < bbox[2] <= 1.0 and 0.0 <= bbox[1] < bbox[3] <= 1.0)
    ):
        return invalid_args(
            "echo_region", "bbox: four numbers, 0 <= x0 < x1 <= 1 and 0 <= y0 < y1 <= 1"
        )
    region = {
        "id": "r1",
        "location": {
            "frame": f"raw:{image}",
            "kind": "bbox",
            "bbox": [float(v) for v in bbox],
            "polygon": None,
            "yaw": None,
            "pitch": None,
            "fov": None,
        },
        "label": "echo",
        "score": 1.0,
    }
    return envelope("echo_region", "ok", f"Echoed the box on raw:{image}.", [region])


HANDLERS = {"echo_region": echo_region}
