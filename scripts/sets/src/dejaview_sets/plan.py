"""A set plan: what to generate, per family, and which construction op makes each item.

Strict: unknown keys anywhere are refused, so an answer label cannot sneak into a plan and from
there into a public file. The plan is owner material; it goes to owner storage with the record.
"""

from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from typing import Any

GENERATORS = ("fal", "openai", "gemini", "fake")
SPLITS = ("dev", "heldout", "blind")


@dataclass(frozen=True)
class SetSpec:
    id: str
    use_case: str
    split: str
    licence: str
    images_repo: str
    rules: dict[str, Any]
    images_rev: str | None = None


@dataclass(frozen=True)
class Source:
    prompt: str
    seed: int
    size: int = 1024


@dataclass(frozen=True)
class Item:
    id: str
    op: dict[str, Any]


@dataclass(frozen=True)
class Family:
    id: str
    generator: str
    description: str
    brief: str
    source: Source
    items: list[Item] = field(default_factory=list)
    # Protected subjects, one job requirement each (SF1 protects what the requirements name).
    protected: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class Plan:
    set: SetSpec
    families: list[Family]


def _only(table: dict[str, Any], allowed: set[str], where: str) -> None:
    extra = sorted(set(table) - allowed)
    if extra:
        raise ValueError(f"{where}: unknown key(s) {', '.join(extra)}")


def _require(table: dict[str, Any], keys: tuple[str, ...], where: str) -> None:
    missing = [k for k in keys if k not in table]
    if missing:
        raise ValueError(f"{where}: missing {', '.join(missing)}")


def parse(text: str) -> Plan:
    doc = tomllib.loads(text)
    _only(doc, {"set", "family"}, "plan")
    _require(doc, ("set", "family"), "plan")
    s = doc["set"]
    _only(s, {"id", "use_case", "split", "licence", "images_repo", "images_rev", "rules"}, "[set]")
    _require(s, ("id", "use_case", "split", "licence", "images_repo", "rules"), "[set]")
    if s["split"] not in SPLITS:
        raise ValueError(f"[set] split must be one of {SPLITS}")
    _only(s["rules"], {"base", "version"}, "[set] rules")
    _require(s["rules"], ("base", "version"), "[set] rules")
    spec = SetSpec(
        id=s["id"],
        use_case=s["use_case"],
        split=s["split"],
        licence=s["licence"],
        images_repo=s["images_repo"],
        images_rev=s.get("images_rev"),
        rules=dict(s["rules"]),
    )
    families: list[Family] = []
    seen_items: set[str] = set()
    for f in doc["family"]:
        where = f"family {f.get('id', '?')}"
        _only(f, {"id", "generator", "description", "brief", "source", "item", "protected"}, where)
        _require(f, ("id", "generator", "description", "brief", "source"), where)
        if f["generator"] not in GENERATORS:
            raise ValueError(f"{where}: generator must be one of {GENERATORS}")
        src = f["source"]
        _only(src, {"prompt", "seed", "size"}, f"{where} source")
        _require(src, ("prompt", "seed"), f"{where} source")
        items: list[Item] = []
        for it in f.get("item", []):
            iw = f"{where} item {it.get('id', '?')}"
            _only(it, {"id", "op"}, iw)
            _require(it, ("id", "op"), iw)
            if "kind" not in it["op"]:
                raise ValueError(f"{iw}: op needs a kind")
            if it["id"] in seen_items:
                raise ValueError(f"{iw}: duplicate item id")
            seen_items.add(it["id"])
            items.append(Item(id=it["id"], op=dict(it["op"])))
        if not items:
            raise ValueError(f"{where}: no items")
        families.append(
            Family(
                id=f["id"],
                generator=f["generator"],
                description=f["description"],
                brief=f["brief"],
                source=Source(
                    prompt=src["prompt"], seed=int(src["seed"]), size=int(src.get("size", 1024))
                ),
                items=items,
                protected=[str(x) for x in f.get("protected", [])],
            )
        )
    return Plan(set=spec, families=families)


def load(path) -> Plan:
    with open(path, "rb") as f:
        return parse(f.read().decode("utf-8"))
