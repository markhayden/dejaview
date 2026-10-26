"""A stdio MCP server, revision 2025-11-25, speaking newline-delimited JSON-RPC 2.0.

The dejaview harness is the only client: it initialises, lists, calls with ``_dejaview`` injected,
resets between runs with ``dejaview/reset``, and cancels with ``notifications/cancelled``.
"""

from __future__ import annotations

import json
import sys
from typing import Any, TextIO

from . import __version__, tools

PROTOCOL_VERSION = "2025-11-25"


def mcp_tool(manifest: dict[str, Any]) -> dict[str, Any]:
    """A dejaview manifest as an MCP tool: native name, description and schemas; the rest annotated."""
    annotation = {
        k: v for k, v in manifest.items() if k not in ("name", "input_schema", "output_schema")
    }
    return {
        "name": manifest["name"],
        "description": manifest["purpose"],
        "inputSchema": manifest["input_schema"],
        "outputSchema": manifest["output_schema"],
        "annotations": {"dejaview": annotation},
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
            self.result(
                request_id,
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "dejaview-tools", "version": __version__},
                },
            )
        elif method in ("notifications/initialized", "notifications/cancelled"):
            return
        elif method == "tools/list":
            self.result(request_id, {"tools": [mcp_tool(m) for m in tools.MANIFESTS]})
        elif method == "tools/call":
            self.call(request_id, params)
        elif method == "dejaview/reset":
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
        context = arguments.get("_dejaview")
        if not isinstance(context, dict):
            envelope = tools.invalid_args(name, "_dejaview is required")
        else:
            model_args = {k: v for k, v in arguments.items() if k != "_dejaview"}
            envelope = handler(model_args, context)
        self.result(
            request_id,
            {
                "content": [],
                "structuredContent": envelope,
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
            print("dejaview-tools: malformed line", file=sys.stderr)
            continue
        if isinstance(message, dict):
            server.handle(message)
