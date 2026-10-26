"""The server's protocol and the echo tool, driven in-process."""

import io
import json

from dejaview_tools import __version__, server, tools


def run(messages):
    out = io.StringIO()
    stdin = io.StringIO("".join(json.dumps(m) + "\n" for m in messages))
    server.serve(stdin, out)
    return [json.loads(line) for line in out.getvalue().splitlines()]


def context(images=("candidate",)):
    return {
        "run_id": "run-1",
        "call_id": "call:1",
        "images": {
            i: {"media_type": "image/png", "width": 1, "height": 1, "projection": "flat"}
            for i in images
        },
        "frames": {},
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
