"""A small TOML writer for the shapes the toolkit emits: tables, arrays of tables, inline tables,
strings, integers, floats, booleans and lists of those."""

from __future__ import annotations

from typing import Any


def _s(v: str) -> str:
    return '"' + v.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def value(v: Any) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return repr(float(v))
    if isinstance(v, str):
        return _s(v)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k} = {value(x)}" for k, x in v.items() if x is not None) + " }"
    raise TypeError(f"cannot write {type(v).__name__} to TOML")


def table(name: str, fields: dict[str, Any], array: bool = False) -> str:
    head = f"[[{name}]]" if array else f"[{name}]"
    lines = [head]
    for k, v in fields.items():
        if v is None:
            continue
        lines.append(f"{k} = {value(v)}")
    return "\n".join(lines) + "\n"


def document(comment: str, tables: list[tuple[str, dict[str, Any], bool]]) -> str:
    out = [f"# {line}" for line in comment.splitlines()] if comment else []
    body = "\n".join(table(n, f, a) for n, f, a in tables)
    return ("\n".join(out) + "\n" + body) if out else body
