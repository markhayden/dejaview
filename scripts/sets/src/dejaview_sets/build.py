"""Build a set from a plan: generate each family's source once, derive every item by its op,
write the public set directory (manifest, families, jobs, images) and the owner worksheet
(construction record, draft labels, contact sheets) to owner storage. Then `promote` publishes
the labels the owner confirmed and drops the items the owner struck.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import io
import shutil
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw

from . import ops, toml_out
from .plan import Family, Plan

SOURCE_FILE = "source.png"
CANDIDATE_FILE = "candidate.png"


@dataclass
class BuildReport:
    items: int = 0
    images: int = 0


@dataclass
class PromoteReport:
    confirmed: int = 0
    struck: int = 0


def _sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _today() -> str:
    return dt.date.today().isoformat()


def _requirements(family: Family) -> list[dict[str, Any]]:
    return [
        {
            "id": f"RQ{n}",
            "kind": "must_hold",
            "scope": ["candidate"],
            "evidence": ["source"],
            "text": f"{subject[0].upper() + subject[1:]} keeps its identity, shape, position and extent relative to the source.",
        }
        for n, subject in enumerate(family.protected, start=1)
    ]


def _job_text(
    plan: Plan,
    family: Family,
    candidate_rel: str,
    candidate_sha: str,
    source_rel: str,
    source_sha: str,
) -> str:
    tables: list[tuple[str, dict[str, Any], bool]] = [
        ("job", {"use_case": plan.set.use_case, "brief": family.brief}, False)
    ]
    for rq in _requirements(family):
        tables.append(("job.requirements", rq, True))
    tables.append(
        (
            "job.images",
            {
                "id": "candidate",
                "role": "candidate",
                "source": {"path": candidate_rel, "sha256": candidate_sha},
            },
            True,
        )
    )
    tables.append(
        (
            "job.images",
            {
                "id": "source",
                "role": "source",
                "source": {"path": source_rel, "sha256": source_sha},
            },
            True,
        )
    )
    return toml_out.document(
        f"Item of {plan.set.id}; built by construction, see the owner's record.", tables
    )


def _contact_sheet(
    family: Family, source: bytes, candidates: list[tuple[str, bytes, ops.DraftLabel]], path: Path
) -> None:
    thumb = 256
    cells = [("source", source, None)] + [(i, b, lab) for i, b, lab in candidates]
    sheet = Image.new(
        "RGB", (thumb * len(cells) + 8 * (len(cells) + 1), thumb + 70), (245, 245, 245)
    )
    d = ImageDraw.Draw(sheet)
    for n, (name, b, lab) in enumerate(cells):
        im = Image.open(io.BytesIO(b)).convert("RGB")
        im.thumbnail((thumb, thumb))
        x = 8 + n * (thumb + 8)
        sheet.paste(im, (x, 8))
        caption = name if lab is None else f"{name}\n{lab.expected}: {lab.words[:48]}"
        d.multiline_text((x, thumb + 14), caption, fill=(20, 20, 20))
    sheet.save(path, format="PNG")


def build(plan: Plan, out: Path, owner: Path, generators: dict[str, Any]) -> BuildReport:
    out = Path(out)
    owner_dir = Path(owner) / plan.set.id
    out.mkdir(parents=True, exist_ok=True)
    owner_dir.mkdir(parents=True, exist_ok=True)
    report = BuildReport()
    manifest_items: list[dict[str, Any]] = []
    families_out: list[dict[str, Any]] = []
    record_families: list[dict[str, Any]] = []
    record_items: list[dict[str, Any]] = []
    draft_labels: list[tuple[str, dict[str, Any], bool]] = []
    for family in plan.families:
        gen = generators.get(family.generator)
        if gen is None:
            raise ValueError(f"family {family.id}: no generator {family.generator!r} available")
        source = gen.generate(family.source.prompt, family.source.seed, family.source.size)
        source_rel = f"families/{family.id}/{SOURCE_FILE}"
        (out / source_rel).parent.mkdir(parents=True, exist_ok=True)
        (out / source_rel).write_bytes(source)
        source_sha = _sha256(source)
        report.images += 1
        families_out.append({"id": family.id, "description": family.description})
        record_families.append(
            {
                "id": family.id,
                "generator": family.generator,
                "model": getattr(gen, "model", family.generator),
                "prompt": family.source.prompt,
                "seed": family.source.seed,
                "size": family.source.size,
                "source_sha256": source_sha,
                "generated_on": _today(),
                "protected": list(family.protected),
            }
        )
        candidates: list[tuple[str, bytes, ops.DraftLabel]] = []
        for n, item in enumerate(family.items):
            candidate, label = ops.apply(item.op, source, gen, family.source.seed + n + 1)
            if label.expected == "fail" and label.obligation_ids == ["SF1"]:
                target = item.op.get("removes") or item.op.get("alters") or ""
                for rq in _requirements(family):
                    if target and target.lower() in rq["text"].lower():
                        label.obligation_ids.append(rq["id"])
            item_dir = out / "items" / item.id
            item_dir.mkdir(parents=True, exist_ok=True)
            candidate_rel = f"items/{item.id}/{CANDIDATE_FILE}"
            (out / candidate_rel).write_bytes(candidate)
            candidate_sha = _sha256(candidate)
            (item_dir / "job.toml").write_text(
                _job_text(plan, family, candidate_rel, candidate_sha, source_rel, source_sha)
            )
            manifest_items.append(
                {"id": item.id, "family": family.id, "job": f"items/{item.id}/job.toml"}
            )
            record_items.append(
                {
                    "id": item.id,
                    "family": family.id,
                    "op": dict(item.op),
                    "candidate_sha256": candidate_sha,
                    "generated_on": _today(),
                    **({"edit_notes": label.notes} if label.notes else {}),
                }
            )
            fields: dict[str, Any] = {
                "item": item.id,
                "expected": label.expected,
                "failure_types": [label.failure_type] if label.failure_type else [],
                "words": label.words,
                "obligation_ids": list(label.obligation_ids),
                "adjudicated_by": "owner",
                "adjudicated_on": "",
                "status": "draft",
            }
            draft_labels.append(("label", fields, True))
            if label.region_kind:
                region: dict[str, Any] = {
                    "frame": "raw:candidate",
                    "kind": label.region_kind,
                    "defect": label.failure_type or "changed",
                    "obligation_ids": list(label.obligation_ids),
                }
                if label.bbox:
                    region["bbox"] = label.bbox
                draft_labels.append(("label.regions", region, True))
            candidates.append((item.id, candidate, label))
            report.items += 1
            report.images += 1
        _contact_sheet(family, source, candidates, owner_dir / f"contact-{family.id}.png")
    set_fields: dict[str, Any] = {
        "id": plan.set.id,
        "use_case": plan.set.use_case,
        "split": plan.set.split,
        "licence": plan.set.licence,
        "images_repo": plan.set.images_repo,
        "images_rev": plan.set.images_rev,
        "labels": "labels.toml",
        "families": "families.toml",
        "rules": dict(plan.set.rules),
    }
    manifest = [("set", set_fields, False)] + [("item", i, True) for i in manifest_items]
    (out / "set.toml").write_text(
        toml_out.document(
            "Built by dejaview-sets; labels arrive with `promote` after the owner's review.",
            manifest,
        )
    )
    (out / "families.toml").write_text(
        toml_out.document(
            "Subject or scene families; a family never appears in more than one split.",
            [("family", f, True) for f in families_out],
        )
    )
    (owner_dir / "construction.toml").write_text(
        toml_out.document(
            f"Construction record of {plan.set.id}: owner material, never published.",
            [("set", {"id": plan.set.id, "built_on": _today()}, False)]
            + [("family", f, True) for f in record_families]
            + [("item", i, True) for i in record_items],
        )
    )
    (owner_dir / "labels-draft.toml").write_text(
        toml_out.document(
            'Draft labels from the construction record. Set each label\'s status to "confirmed" or "struck"\nand adjudicated_on to the date, then run `dejaview-sets promote`.',
            draft_labels,
        )
    )
    return report


def promote(draft_path: Path, set_dir: Path) -> PromoteReport:
    """Publishes the confirmed labels as labels.toml and removes struck items from the set."""
    draft = tomllib.loads(Path(draft_path).read_text())
    set_dir = Path(set_dir)
    report = PromoteReport()
    confirmed: list[dict[str, Any]] = []
    struck: set[str] = set()
    for label in draft.get("label", []):
        status = label.get("status", "draft")
        if status == "confirmed":
            if not label.get("adjudicated_on"):
                label["adjudicated_on"] = _today()
            label.pop("status", None)
            confirmed.append(label)
            report.confirmed += 1
        elif status == "struck":
            struck.add(label["item"])
            report.struck += 1
        else:
            raise ValueError(
                f"label for {label.get('item')} is still a draft; confirm or strike every label first"
            )
    manifest = tomllib.loads((set_dir / "set.toml").read_text())
    kept = [i for i in manifest.get("item", []) if i["id"] not in struck]
    for i in manifest.get("item", []):
        if i["id"] in struck:
            shutil.rmtree(set_dir / "items" / i["id"], ignore_errors=True)
    tables: list[tuple[str, dict[str, Any], bool]] = [("set", manifest["set"], False)] + [
        ("item", i, True) for i in kept
    ]
    (set_dir / "set.toml").write_text(
        toml_out.document("Built by dejaview-sets; labels confirmed by the owner.", tables)
    )
    label_tables: list[tuple[str, dict[str, Any], bool]] = []
    for label in confirmed:
        regions = label.pop("regions", [])
        label_tables.append(("label", label, True))
        for r in regions:
            label_tables.append(("label.regions", r, True))
    (set_dir / "labels.toml").write_text(
        toml_out.document(
            "Answer labels adjudicated by the owner from the construction record.", label_tables
        )
    )
    return report
