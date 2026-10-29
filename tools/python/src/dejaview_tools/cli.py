"""The ``dejaview-tools`` executable: ``serve`` runs the MCP server on stdio."""

from __future__ import annotations

import sys
from pathlib import Path

from . import __version__, models, server


def fetch(args: list[str]) -> int:
    """``fetch [--from <dir or URL prefix>] [--models <dir>]``: the segmenter artifact and its
    goldens into the models directory, every byte checked against the shipped record."""
    import argparse

    ap = argparse.ArgumentParser(prog="dejaview-tools fetch")
    ap.add_argument(
        "--from", dest="source", help="a directory or URL prefix instead of the release"
    )
    ap.add_argument(
        "--models", type=Path, help="the models directory ($XDG_CACHE_HOME/dejaview/models)"
    )
    a = ap.parse_args(args)
    record = models.Record.load()
    try:
        where = models.fetch(record, a.models, a.source)
    except models.FetchError as e:
        print(f"dejaview-tools: {e}", file=sys.stderr)
        return 1
    print(f"fetch: {record.name} complete in {where}")
    return 0


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if not args or args[0] in ("-h", "--help"):
        print(
            "usage: dejaview-tools serve | fetch [--from <dir or URL>] [--models <dir>] | --version"
        )
        return 0 if args else 2
    if args[0] == "--version":
        print(__version__)
        return 0
    if args[0] == "serve":
        server.serve()
        return 0
    if args[0] == "fetch":
        return fetch(args[1:])
    print(f"dejaview-tools: unknown command {args[0]!r}", file=sys.stderr)
    return 2
