"""The ``_squint`` context a call carries: the images and frames the harness resolved for it.

The harness injects it into every call's arguments (SPEC "Calls"): the run and call ids, the
canonical images named by the tool's ``image_args``, the frames named by its ``frame_args`` with
their transform chains, and the rules' hints. Pixels arrive by path under the run directory or
inline as base64; the pack never resolves a path of its own.
"""

from __future__ import annotations

import base64
import io
from dataclasses import dataclass, field
from typing import Any

import numpy as np
from PIL import Image


class ContextError(ValueError):
    """The context is not what the harness sends: a malformed call, never the model's mistake."""


@dataclass(frozen=True)
class Pixels:
    """How to reach one image's lossless pixels."""

    media_type: str
    width: int
    height: int
    projection: str
    path: str | None = None
    base64: str | None = None

    def load(self) -> np.ndarray:
        """The pixels as an RGB uint8 array of shape (height, width, 3)."""
        if self.path is not None:
            img = Image.open(self.path)
        elif self.base64 is not None:
            img = Image.open(io.BytesIO(base64.b64decode(self.base64)))
        else:
            raise ContextError("pixels carry neither a path nor base64")
        rgb = np.asarray(img.convert("RGB"))
        if rgb.shape[:2] != (self.height, self.width):
            raise ContextError(
                f"pixels are {rgb.shape[1]}x{rgb.shape[0]}, the handle says {self.width}x{self.height}"
            )
        return rgb


@dataclass(frozen=True)
class Frame:
    """A frame the call named: where it sits in the frame tree and its pixels."""

    root: str
    projection: str
    parent: str | None
    transform: dict[str, Any] | None
    chain: list[dict[str, Any]]
    pixels: Pixels


@dataclass(frozen=True)
class Context:
    run_id: str
    call_id: str
    images: dict[str, Pixels] = field(default_factory=dict)
    frames: dict[str, Frame] = field(default_factory=dict)
    hints: Any = None

    @staticmethod
    def parse(value: Any) -> Context:
        if not isinstance(value, dict):
            raise ContextError("_squint must be an object")
        try:
            images = {
                image_id: _pixels(handle)
                for image_id, handle in (value.get("images") or {}).items()
            }
            frames = {
                frame_id: Frame(
                    root=handle["root"],
                    projection=handle["projection"],
                    parent=handle.get("parent"),
                    transform=handle.get("transform"),
                    chain=list(handle.get("chain") or []),
                    pixels=_pixels(handle["pixels"]),
                )
                for frame_id, handle in (value.get("frames") or {}).items()
            }
            return Context(
                run_id=str(value["run_id"]),
                call_id=str(value["call_id"]),
                images=images,
                frames=frames,
                hints=value.get("hints"),
            )
        except (KeyError, TypeError) as e:
            raise ContextError(f"_squint is malformed: {e!r}") from e

    def frame_pixels(self, frame_id: str) -> Pixels | None:
        """The pixels of a frame the call named, or of an image's raw frame (``raw:<image>``)."""
        if frame_id in self.frames:
            return self.frames[frame_id].pixels
        image_id = frame_id.removeprefix("raw:")
        if frame_id != image_id and image_id in self.images:
            return self.images[image_id]
        return None


def _pixels(handle: dict[str, Any]) -> Pixels:
    return Pixels(
        media_type=handle["media_type"],
        width=int(handle["width"]),
        height=int(handle["height"]),
        projection=handle["projection"],
        path=handle.get("path"),
        base64=handle.get("base64"),
    )
