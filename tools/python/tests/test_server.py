"""The server's protocol and the echo tool, driven in-process."""

import base64
import io
import json

import numpy as np
from PIL import Image

from dejaview_tools import __version__, server, tools


def run(messages):
    out = io.StringIO()
    stdin = io.StringIO("".join(json.dumps(m) + "\n" for m in messages))
    server.serve(stdin, out)
    return [json.loads(line) for line in out.getvalue().splitlines()]


def png(width, height, value=128):
    buf = io.BytesIO()
    Image.fromarray(np.full((height, width, 3), value, dtype=np.uint8)).save(buf, format="PNG")
    return buf.getvalue()


def handle(width, height, **where):
    return {
        "media_type": "image/png",
        "width": width,
        "height": height,
        "projection": "flat",
        **where,
    }


def context(images=("candidate",), frames=None):
    return {
        "run_id": "run-1",
        "call_id": "call:1",
        "images": {
            i: handle(20, 10, base64=base64.b64encode(png(20, 10)).decode("ascii")) for i in images
        },
        "frames": frames or {},
        "hints": {},
    }


def test_initialize_lists_and_calls():
    responses = run(
        [
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "2025-11-25"},
            },
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "echo_region",
                    "arguments": {
                        "image": "candidate",
                        "bbox": [0.1, 0.2, 0.3, 0.4],
                        "_dejaview": context(),
                    },
                },
            },
        ]
    )
    init, listed, called = responses
    assert init["result"]["protocolVersion"] == "2025-11-25"
    assert "tools" in init["result"]["capabilities"]
    assert init["result"]["serverInfo"]["version"] == __version__
    tool = listed["result"]["tools"][0]
    assert tool["name"] == "echo_region"
    assert tool["description"] == tools.ECHO_REGION["purpose"]
    assert tool["annotations"]["dejaview"]["evidence_kind"] == "change_region"
    assert "_dejaview" in tool["inputSchema"]["required"]
    envelope = called["result"]["structuredContent"]
    assert called["result"]["isError"] is False
    assert envelope["status"] == "ok"
    assert envelope["regions"][0]["location"] == {
        "frame": "raw:candidate",
        "kind": "bbox",
        "bbox": [0.1, 0.2, 0.3, 0.4],
        "polygon": None,
        "yaw": None,
        "pitch": None,
        "fov": None,
    }
    # The cut box is this call's one crop: named for the harness, its bytes in the one image
    # block and not in the envelope, its mapping the pixel box actually cut (x 2..6 of 20,
    # y 2..4 of 10).
    [image] = envelope["images"]
    assert image["frame"] == "crop:call:1:0" and image["parent"] == "raw:candidate"
    assert image["mapping"] == {"bbox": [0.1, 0.2, 0.3, 0.4]}
    assert (image["width"], image["height"], image["media_type"]) == (4, 2, "image/png")
    assert "base64" not in image
    [block] = called["result"]["content"]
    assert block["type"] == "image" and block["mimeType"] == "image/png"
    cut = Image.open(io.BytesIO(base64.b64decode(block["data"])))
    assert cut.size == (4, 2)


def test_the_box_is_cut_from_a_named_frame_and_an_unknown_frame_is_the_models_mistake():
    aligned = {
        "root": "candidate",
        "projection": "flat",
        "parent": "raw:source",
        "transform": {"id": "t1", "mapping": {"homography": [1, 0, 0, 0, 1, 0, 0, 0, 1]}},
        "chain": [],
        "pixels": handle(40, 20, base64=base64.b64encode(png(40, 20, 7)).decode("ascii")),
    }

    def call(frame, n):
        return {
            "jsonrpc": "2.0",
            "id": n,
            "method": "tools/call",
            "params": {
                "name": "echo_region",
                "arguments": {
                    "image": "candidate",
                    "bbox": [0.5, 0.5, 1.0, 1.0],
                    "frame": frame,
                    "_dejaview": context(frames={"aligned:candidate:t1": aligned}),
                },
            },
        }

    from_frame, unknown = run([call("aligned:candidate:t1", 1), call("crop:call:0:0", 2)])
    [image] = from_frame["result"]["structuredContent"]["images"]
    assert image["parent"] == "aligned:candidate:t1"
    assert (image["width"], image["height"]) == (20, 10)
    cut = Image.open(io.BytesIO(base64.b64decode(from_frame["result"]["content"][0]["data"])))
    assert cut.getpixel((0, 0)) == (7, 7, 7)
    assert unknown["result"]["isError"] is True
    assert "unknown frame id" in unknown["result"]["structuredContent"]["summary"]


def test_a_malformed_context_is_a_protocol_error_not_an_answer():
    [response] = run(
        [
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "echo_region",
                    "arguments": {"image": "candidate", "bbox": [0, 0, 1, 1], "_dejaview": "nope"},
                },
            }
        ]
    )
    assert response["error"]["code"] == -32602


def test_every_envelope_carries_exactly_the_provenance_the_harness_knows():
    # The harness refuses an envelope with an unknown provenance field, so a pack that annotates
    # its provenance breaks every call; the shape is fixed.
    from dejaview_tools.envelope import envelope

    assert set(envelope("x", "ok", "s")["provenance"]) == {
        "tool",
        "version",
        "duration_ms",
        "lock_digest",
    }
    [called] = run(
        [
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "echo_region",
                    "arguments": {
                        "image": "candidate",
                        "bbox": [0.1, 0.2, 0.3, 0.4],
                        "_dejaview": context(),
                    },
                },
            }
        ]
    )
    assert set(called["result"]["structuredContent"]["provenance"]) == {
        "tool",
        "version",
        "duration_ms",
        "lock_digest",
    }


def test_missing_context_and_bad_arguments_are_invalid_args():
    responses = run(
        [
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "echo_region",
                    "arguments": {"image": "candidate", "bbox": [0, 0, 1, 1]},
                },
            },
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "echo_region",
                    "arguments": {"image": "nope", "bbox": [0, 0, 1, 1], "_dejaview": context()},
                },
            },
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "echo_region",
                    "arguments": {
                        "image": "candidate",
                        "bbox": [1, 0, 0, 1],
                        "_dejaview": context(),
                    },
                },
            },
        ]
    )
    for r in responses:
        assert r["result"]["isError"] is True
        assert r["result"]["structuredContent"]["status"] == "invalid_args"


def test_reset_acknowledges_and_unknown_methods_are_not_found():
    responses = run(
        [
            {"jsonrpc": "2.0", "id": 1, "method": "dejaview/reset", "params": {}},
            {"jsonrpc": "2.0", "id": 2, "method": "resources/list", "params": {}},
            {"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 9}},
        ]
    )
    assert responses[0]["result"] == {}
    assert responses[1]["error"]["code"] == -32601
    assert len(responses) == 2
