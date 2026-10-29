"""`squint-sets build` and `squint-sets promote`."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from . import build as build_mod
from . import plan as plan_mod
from .generators import FakeGenerator, live


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="squint-sets", description="squint set-building toolkit")
    sub = ap.add_subparsers(dest="command", required=True)
    b = sub.add_parser(
        "build", help="generate a set from a plan into a set directory and an owner worksheet"
    )
    b.add_argument("--plan", required=True, type=Path)
    b.add_argument(
        "--out", required=True, type=Path, help="the set directory (evalsets/<use-case>/<set>)"
    )
    b.add_argument(
        "--owner", required=True, type=Path, help="owner storage root, outside every repository"
    )
    b.add_argument(
        "--fake", action="store_true", help="use the fake generator for every family (no network)"
    )
    r = sub.add_parser(
        "rebuild",
        help="regenerate one item from the family's existing images, with the plan as it now reads",
    )
    r.add_argument("--plan", required=True, type=Path)
    r.add_argument("--out", required=True, type=Path)
    r.add_argument("--owner", required=True, type=Path)
    r.add_argument("--item", required=True)
    r.add_argument(
        "--seed-offset",
        type=int,
        default=0,
        help="added to the item's seed, to get a different draw",
    )
    r.add_argument("--fake", action="store_true")
    p = sub.add_parser("promote", help="publish confirmed labels and drop struck items")
    p.add_argument("--draft", required=True, type=Path, help="the reviewed labels-draft.toml")
    p.add_argument("--set", required=True, type=Path, help="the set directory")
    args = ap.parse_args(argv)
    try:
        if args.command == "build":
            plan = plan_mod.load(args.plan)
            gens = live()
            if args.fake:
                gens = {name: FakeGenerator() for name in gens}
            r = build_mod.build(plan, args.out, args.owner, gens)
            print(
                f"built {plan.set.id}: {r.items} items, {r.images} images in {args.out}; worksheet in {args.owner / plan.set.id}"
            )
        elif args.command == "rebuild":
            plan = plan_mod.load(args.plan)
            gens = live()
            if args.fake:
                gens = {name: FakeGenerator() for name in gens}
            build_mod.rebuild(plan, args.out, args.owner, gens, args.item, args.seed_offset)
            print(
                f"rebuilt {args.item} in {args.out}; its label is back to draft in {args.owner / plan.set.id}"
            )
        else:
            r = build_mod.promote(args.draft, args.set)
            print(
                f"promoted {r.confirmed} confirmed label(s), struck {r.struck} item(s) in {args.set}"
            )
        return 0
    except (ValueError, OSError, KeyError) as e:
        print(f"squint-sets: {e}", file=sys.stderr)
        return 1
