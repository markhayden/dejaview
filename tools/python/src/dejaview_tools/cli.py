"""The ``dejaview-tools`` executable: ``serve`` runs the MCP server on stdio."""

from __future__ import annotations

import sys

from . import __version__, server


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if not args or args[0] in ("-h", "--help"):
        print("usage: dejaview-tools serve | fetch | --version")
        return 0 if args else 2
    if args[0] == "--version":
        print(__version__)
        return 0
    if args[0] == "serve":
        server.serve()
        return 0
    if args[0] == "fetch":
        print("dejaview-tools: no model artifacts to fetch in this version")
        return 0
    print(f"dejaview-tools: unknown command {args[0]!r}", file=sys.stderr)
    return 2
