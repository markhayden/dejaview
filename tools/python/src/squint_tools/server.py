"""A stdio MCP server, revision 2025-11-25, speaking newline-delimited JSON-RPC 2.0.

The squint harness is the only client: it initialises, lists, calls with ``_squint`` injected,
resets between runs with ``squint/reset``, and cancels with ``notifications/cancelled``.
"""

from __future__ import annotations

import json
import sys
from contextlib import suppress
from typing import Any, TextIO

from . import __version__, segmenter, tools
from .context import Context, ContextError
from .envelope import invalid_args, split_images
from .segmenter import SegmenterUnavailable

PROTOCOL_VERSION = "2025-11-25"


def mcp_tool(manifest: dict[str, Any]) -> dict[str, Any]:
    """A squint manifest as an MCP tool: native name, description and schemas; the rest annotated."""
    annotation = {
        k: v for k, v in manifest.items() if k not in ("name", "input_schema", "output_schema")
    }
    return {
        "name": manifest["name"],
        "description": manifest["purpose"],
        "inputSchema": manifest["input_schema"],
        "outputSchema": manifest["output_schema"],
        "annotations": {"squint": annotation},
    }


class Server:
    def __init__(self, out: TextIO) -> None:
        self.out = out
        self.calls_since_reset = 0

    def respond(self, message: dict[str, Any]) -> None:
        self.out.write(json.dumps(message, separators=(",", ":")) + "\n")
        self.out.flush()

    def result(self, request_id: Any, result: Any) -> None:
        self.respond({"jsonrpc": "2.0", "id": request_id, "result": result})

    def error(self, request_id: Any, code: int, message: str) -> None:
        self.respond(
            {"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}}
        )

    def handle(self, message: dict[str, Any]) -> None:
        method = message.get("method", "")
        request_id = message.get("id")
        params = message.get("params") or {}
        if method == "initialize":
            # The segmenter is loaded once here when its artifact is present, so the first
            # `regions` call does not pay the load inside its own timeout; when it is absent the
            # tools that need it answer each call with an error and the rest serve.
            with suppress(SegmenterUnavailable):
                segmenter.shared()
            self.result(
                request_id,
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "squint-tools", "version": __version__},
                },
            )
        elif method in ("notifications/initialized", "notifications/cancelled"):
            return
        elif method == "tools/list":
            self.result(request_id, {"tools": [mcp_tool(m) for m in tools.MANIFESTS]})
        elif method == "tools/call":
            self.call(request_id, params)
        elif method == "squint/reset":
            self.calls_since_reset = 0
            self.result(request_id, {})
        elif request_id is not None:
            self.error(request_id, -32601, "Method not found")

    def call(self, request_id: Any, params: dict[str, Any]) -> None:
        self.calls_since_reset += 1
        name = params.get("name", "")
        arguments = params.get("arguments") or {}
        handler = tools.HANDLERS.get(name)
        if handler is None:
            self.error(request_id, -32602, f"unknown tool {name}")
            return
        if "_squint" not in arguments:
            envelope = invalid_args(name, "_squint is required")
        else:
            try:
                context = Context.parse(arguments["_squint"])
            except ContextError as e:
                # The harness's call is malformed: a JSON-RPC error, which it records as a
                # tool error, never an answer the model could act on.
                self.error(request_id, -32602, str(e))
                return
            model_args = {k: v for k, v in arguments.items() if k != "_squint"}
            try:
                envelope = handler(model_args, context)
            except (SegmenterUnavailable, OSError, ValueError) as e:
                # The artifact `fetch` places is not there, or a handle's pixels cannot be read
                # (a corrupt file, a decompression bomb, bad base64): the harness records the
                # call as a tool error with these words, and the server goes on serving.
                self.error(request_id, -32603, f"{name}: {type(e).__name__}: {e}")
                return
        # The images' bytes travel as content blocks, in the envelope's order (SPEC "Results").
        structured, blocks = split_images(envelope)
        self.result(
            request_id,
            {
                "content": blocks,
                "structuredContent": structured,
                "isError": envelope["status"] == "invalid_args",
            },
        )


def serve(stdin: TextIO = sys.stdin, stdout: TextIO = sys.stdout) -> None:
    server = Server(stdout)
    for line in stdin:
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            print("squint-tools: malformed line", file=sys.stderr)
            continue
        if isinstance(message, dict):
            server.handle(message)
