"""Image generators behind one small interface: `generate(prompt, seed, size)` and
`edit(image, prompt, seed)`, both returning PNG or JPEG bytes. Credentials come from the
environment only (FAL_KEY, OPENAI_API_KEY, GEMINI_API_KEY) and are never written anywhere.
"""

from __future__ import annotations

import base64
import io
import json
import os
import urllib.request
import uuid

from PIL import Image, ImageDraw


class GeneratorError(RuntimeError):
    pass


def _post_json(url: str, body: dict, headers: dict, timeout: int = 300) -> dict:
    req = urllib.request.Request(
        url, data=json.dumps(body).encode(), headers={"Content-Type": "application/json", **headers}
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.load(r)
    except urllib.error.HTTPError as e:
        detail = e.read()[:400].decode("utf-8", "replace")
        raise GeneratorError(f"{url}: HTTP {e.code}: {detail}") from None


def _get_bytes(url: str, timeout: int = 120) -> bytes:
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return r.read()


def _env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise GeneratorError(f"{name} is not set")
    return value


def _data_uri(image: bytes) -> str:
    kind = "image/png" if image[:8] == b"\x89PNG\r\n\x1a\n" else "image/jpeg"
    return f"data:{kind};base64," + base64.b64encode(image).decode()


class FalKlein:
    """FLUX.2 [klein] 4B on fal: text to image and instruction edits, seeded."""

    name = "fal"
    model = "fal-ai/flux-2-klein/4b"
    edit_model = "fal-ai/flux-2/klein/4b/edit"

    def _call(self, model: str, body: dict) -> bytes:
        d = _post_json(
            f"https://fal.run/{model}", body, {"Authorization": "Key " + _env("FAL_KEY")}
        )
        images = d.get("images") or []
        if not images:
            raise GeneratorError(f"{model}: no image in the response")
        return _get_bytes(images[0]["url"])

    def generate(self, prompt: str, seed: int, size: int) -> bytes:
        return self._call(
            self.model,
            {
                "prompt": prompt,
                "seed": seed,
                "image_size": {"width": size, "height": size},
                "num_images": 1,
                "output_format": "png",
            },
        )

    def edit(self, image: bytes, prompt: str, seed: int) -> bytes:
        return self._call(
            self.edit_model,
            {
                "prompt": prompt,
                "image_urls": [_data_uri(image)],
                "seed": seed,
                "num_images": 1,
                "output_format": "png",
            },
        )


class OpenAiImage:
    """gpt-image-2 through the Images API; no seed, so the record carries prompt and date."""

    name = "openai"
    model = "gpt-image-2"

    def generate(self, prompt: str, seed: int, size: int) -> bytes:
        d = _post_json(
            "https://api.openai.com/v1/images/generations",
            {
                "model": self.model,
                "prompt": prompt,
                "size": f"{size}x{size}",
                "quality": "medium",
                "output_format": "png",
                "n": 1,
            },
            {"Authorization": "Bearer " + _env("OPENAI_API_KEY")},
        )
        return base64.b64decode(d["data"][0]["b64_json"])

    def edit(self, image: bytes, prompt: str, seed: int) -> bytes:
        boundary = "----dejaview" + uuid.uuid4().hex
        parts = []
        for name, value in (
            ("model", self.model),
            ("prompt", prompt),
            ("quality", "medium"),
            ("output_format", "png"),
        ):
            parts.append(
                f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"\r\n\r\n{value}\r\n'.encode()
            )
        parts.append(
            f'--{boundary}\r\nContent-Disposition: form-data; name="image[]"; filename="source.png"\r\nContent-Type: image/png\r\n\r\n'.encode()
            + image
            + b"\r\n"
        )
        parts.append(f"--{boundary}--\r\n".encode())
        req = urllib.request.Request(
            "https://api.openai.com/v1/images/edits",
            data=b"".join(parts),
            headers={
                "Authorization": "Bearer " + _env("OPENAI_API_KEY"),
                "Content-Type": f"multipart/form-data; boundary={boundary}",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=300) as r:
                d = json.load(r)
        except urllib.error.HTTPError as e:
            raise GeneratorError(
                f"images/edits: HTTP {e.code}: {e.read()[:400].decode('utf-8', 'replace')}"
            ) from None
        return base64.b64decode(d["data"][0]["b64_json"])


class GeminiImage:
    """Gemini 3.1 Flash Image through generateContent; no seed."""

    name = "gemini"
    model = "gemini-3.1-flash-image"

    def _call(self, parts: list[dict]) -> bytes:
        d = _post_json(
            f"https://generativelanguage.googleapis.com/v1beta/models/{self.model}:generateContent",
            {
                "contents": [{"parts": parts}],
                "generationConfig": {
                    "responseModalities": ["IMAGE"],
                    "imageConfig": {"aspectRatio": "1:1"},
                },
            },
            {"x-goog-api-key": _env("GEMINI_API_KEY")},
        )
        for p in d.get("candidates", [{}])[0].get("content", {}).get("parts", []):
            if "inlineData" in p:
                return base64.b64decode(p["inlineData"]["data"])
        raise GeneratorError(f"{self.model}: no image part in the response")

    def generate(self, prompt: str, seed: int, size: int) -> bytes:
        return self._call([{"text": prompt}])

    def edit(self, image: bytes, prompt: str, seed: int) -> bytes:
        mime = "image/png" if image[:8] == b"\x89PNG\r\n\x1a\n" else "image/jpeg"
        return self._call(
            [
                {"inlineData": {"mimeType": mime, "data": base64.b64encode(image).decode()}},
                {"text": prompt},
            ]
        )


class FakeGenerator:
    """Deterministic pictures for tests and dry runs without a network: a coloured scene with a
    few shapes; `edit` paints over a patch so the difference region is well defined."""

    name = "fake"
    model = "fake-model"

    def generate(self, prompt: str, seed: int, size: int) -> bytes:
        im = Image.new("RGB", (size, size), (120 + seed % 60, 140, 160))
        d = ImageDraw.Draw(im)
        d.rectangle((size * 0.3, size * 0.4, size * 0.7, size * 0.9), fill=(170, 60, 50))
        d.rectangle((size * 0.45, size * 0.65, size * 0.55, size * 0.9), fill=(20, 80, 40))
        d.ellipse((size * 0.05, size * 0.1, size * 0.3, size * 0.5), fill=(60, 120, 50))
        d.ellipse((size * 0.72, size * 0.8, size * 0.9, size * 0.9), fill=(40, 60, 200))
        buf = io.BytesIO()
        im.save(buf, format="PNG")
        return buf.getvalue()

    def edit(self, image: bytes, prompt: str, seed: int) -> bytes:
        im = Image.open(io.BytesIO(image)).convert("RGB")
        w, h = im.size
        ImageDraw.Draw(im).rectangle((w * 0.7, h * 0.78, w * 0.92, h * 0.92), fill=(120, 140, 160))
        buf = io.BytesIO()
        im.save(buf, format="PNG")
        return buf.getvalue()


def live() -> dict:
    return {
        "fal": FalKlein(),
        "openai": OpenAiImage(),
        "gemini": GeminiImage(),
        "fake": FakeGenerator(),
    }
