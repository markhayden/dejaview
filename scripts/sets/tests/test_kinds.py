"""The remaining family kinds: reality checks (single candidates, defects planted by edit), brief
compliance (candidates generated from the brief or an altered brief) and product fidelity (the
subject kind under the product rules)."""

import tomllib
from pathlib import Path

from dejaview_sets import build
from dejaview_sets import plan as plan_mod
from dejaview_sets.generators import FakeGenerator

REALITY = """
[set]
id = "reality-checks/people-v1"
use_case = "reality-checks"
split = "dev"
licence = "CC-BY-4.0"
images_repo = "."
rules = { base = "reality-checks", version = 1 }

[[family]]
id = "market"
generator = "fake"
description = "a market stall with a vendor handing over a bag"
brief = "A photograph of a market stall; a vendor hands a paper bag to a customer; a chalkboard reads FRESH PEARS."
source = { prompt = "A photograph of a market stall; a vendor hands a paper bag to a customer; a chalkboard reads FRESH PEARS.", seed = 7 }

[[family.item]]
id = "market-clean"
op = { kind = "as_is" }

[[family.item]]
id = "market-extra-fingers"
op = { kind = "edit", prompt = "Give the vendor's visible hand six fingers. Change nothing else.", defect = "anatomy", words = "the vendor's hand has six fingers" }

[[family.item]]
id = "market-garbled-sign"
op = { kind = "edit", prompt = "Replace the chalkboard writing with letter-like scribbles that spell nothing. Change nothing else.", defect = "text", words = "the chalkboard text is garbled" }
"""

BRIEF = """
[set]
id = "brief-compliance/stills-v1"
use_case = "brief-compliance"
split = "dev"
licence = "CC-BY-4.0"
images_repo = "."
rules = { base = "brief-compliance", version = 1 }

[[family]]
id = "cafe-window"
kind = "brief"
generator = "fake"
description = "a cafe window with a sign"
brief = "A photograph of a cafe window from the street: a red bicycle leaning on the wall below the window, two white coffee cups on the sill, and a hanging sign that reads OPEN. No people."
asks = ["a red bicycle leaning on the wall below the window", "two white coffee cups on the sill", "a hanging sign that reads OPEN"]
forbids = ["people"]
text = ["OPEN"]

[[family.item]]
id = "cafe-window-asked"
op = { kind = "generate", seed = 31 }

[[family.item]]
id = "cafe-window-no-cups"
op = { kind = "generate", seed = 32, prompt = "A photograph of a cafe window from the street: a red bicycle leaning on the wall below the window, an empty sill, and a hanging sign that reads OPEN. No people.", violates = "BC1", about = "two white coffee cups on the sill" }

[[family.item]]
id = "cafe-window-person"
op = { kind = "generate", seed = 33, prompt = "A photograph of a cafe window from the street: a red bicycle leaning on the wall below the window, two white coffee cups on the sill, a hanging sign that reads OPEN, and a man standing at the door.", violates = "BC2", about = "people" }

[[family.item]]
id = "cafe-window-misspelt"
op = { kind = "generate", seed = 34, prompt = "A photograph of a cafe window from the street: a red bicycle leaning on the wall below the window, two white coffee cups on the sill, and a hanging sign that reads OPNE. No people.", violates = "BC3", about = "OPEN" }
"""

PRODUCT = """
[set]
id = "product-fidelity/packshots-v1"
use_case = "product-fidelity"
split = "dev"
licence = "CC-BY-4.0"
images_repo = "."
rules = { base = "product-fidelity", version = 1 }

[[family]]
id = "tonic"
kind = "subject"
generator = "fake"
description = "a tall green glass bottle of tonic water with a white oval label reading HARBOUR TONIC"
subject = "a tall green glass bottle with a white oval label that reads HARBOUR TONIC in black serif capitals and a silver screw cap"
protected = ["the white oval label reading HARBOUR TONIC", "the silver screw cap", "the green glass colour"]
references = [ { prompt = "Studio packshot of the bottle on white.", seed = 41 } ]

[[family.item]]
id = "tonic-bar"
scene = { prompt = "The bottle on a bar counter, warm light.", seed = 51 }

[[family.item]]
id = "tonic-bar-label"
from = "tonic-bar"
op = { kind = "edit", prompt = "Change the label text to HARBOR TONIC. Change nothing else.", alters = "white oval label reading HARBOUR TONIC", aspect = "label" }

[[family.item]]
id = "tonic-bar-colour"
from = "tonic-bar"
op = { kind = "edit", prompt = "Make the glass amber instead of green. Change nothing else.", alters = "green glass colour", aspect = "colour" }

[[family.item]]
id = "tonic-picnic-lookalike"
scene = { prompt = "A bottle on a picnic blanket.", seed = 54 }
lookalike = "a short brown bottle with a round red label"
"""


def test_reality_checks_family_single_candidates(tmp_path: Path):
    out = tmp_path / "evalsets/reality-checks/people-v1"
    owner = tmp_path / "owner"
    r = build.build(plan_mod.parse(REALITY), out, owner, generators={"fake": FakeGenerator()})
    assert r.items == 3
    job = tomllib.loads((out / "items/market-extra-fingers/job.toml").read_text())["job"]
    assert [i["role"] for i in job["images"]] == ["candidate"], (
        "a reality-check job has the candidate only"
    )
    assert "requirements" not in job
    draft = tomllib.loads((owner / "reality-checks/people-v1/labels-draft.toml").read_text())
    by_item = {label["item"]: label for label in draft["label"]}
    assert by_item["market-clean"]["expected"] == "pass"
    assert by_item["market-extra-fingers"]["expected"] == "fail"
    assert by_item["market-extra-fingers"]["obligation_ids"] == ["RE1"]
    assert by_item["market-extra-fingers"]["words"] == "the vendor's hand has six fingers"
    assert by_item["market-extra-fingers"]["regions"][0]["kind"] in ("bbox", "whole_image")
    assert by_item["market-garbled-sign"]["obligation_ids"] == ["RE3"]
    assert by_item["market-garbled-sign"]["failure_types"] == ["text"]


def test_brief_family_generates_from_the_brief_or_an_altered_one(tmp_path: Path):
    out = tmp_path / "evalsets/brief-compliance/stills-v1"
    owner = tmp_path / "owner"
    r = build.build(plan_mod.parse(BRIEF), out, owner, generators={"fake": FakeGenerator()})
    assert r.items == 4 and r.images == 4
    job = tomllib.loads((out / "items/cafe-window-asked/job.toml").read_text())["job"]
    assert job["brief"].startswith("A photograph of a cafe window")
    assert [i["role"] for i in job["images"]] == ["candidate"]
    kinds = [(rq["id"], rq["kind"]) for rq in job["requirements"]]
    assert kinds == [
        ("RQ1", "must_hold"),
        ("RQ2", "must_hold"),
        ("RQ3", "must_hold"),
        ("RQ4", "must_not"),
    ]
    assert "people" in job["requirements"][3]["text"]
    draft = tomllib.loads((owner / "brief-compliance/stills-v1/labels-draft.toml").read_text())
    by_item = {label["item"]: label for label in draft["label"]}
    assert by_item["cafe-window-asked"]["expected"] == "pass"
    assert by_item["cafe-window-no-cups"]["obligation_ids"] == ["BC1", "RQ2"]
    assert by_item["cafe-window-person"]["obligation_ids"] == ["BC2", "RQ4"]
    assert by_item["cafe-window-misspelt"]["obligation_ids"] == ["BC3"]
    assert by_item["cafe-window-misspelt"]["failure_types"] == ["text-misspelt"]
    record = tomllib.loads((owner / "brief-compliance/stills-v1/construction.toml").read_text())
    assert record["item"][1]["op"]["violates"] == "BC1"
    assert record["family"][0]["kind"] == "brief"


def test_product_family_uses_the_product_rules(tmp_path: Path):
    out = tmp_path / "evalsets/product-fidelity/packshots-v1"
    owner = tmp_path / "owner"
    r = build.build(plan_mod.parse(PRODUCT), out, owner, generators={"fake": FakeGenerator()})
    assert r.items == 4 and r.images == 5
    job = tomllib.loads((out / "items/tonic-bar/job.toml").read_text())["job"]
    assert job["use_case"] == "product-fidelity"
    assert job["requirements"][0]["text"].endswith("matches the reference exactly.")
    draft = tomllib.loads((owner / "product-fidelity/packshots-v1/labels-draft.toml").read_text())
    by_item = {label["item"]: label for label in draft["label"]}
    assert by_item["tonic-bar-label"]["obligation_ids"][:1] == ["PF2"]
    assert "RQ1" in by_item["tonic-bar-label"]["obligation_ids"]
    assert by_item["tonic-bar-colour"]["obligation_ids"][:1] == ["PF3"]
    assert by_item["tonic-picnic-lookalike"]["obligation_ids"] == ["PF1", "PF2"]
