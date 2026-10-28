"""`rebuild` regenerates one item from the family's existing images (references, source, the
base candidate of a from item) with a changed prompt or seed, and updates its job, record and
draft label in place, leaving every other item untouched."""

import hashlib
import tomllib
from pathlib import Path

from dejaview_sets import build
from dejaview_sets import plan as plan_mod
from dejaview_sets.generators import FakeGenerator
from tests.test_subject import PLAN


def test_rebuild_one_item_keeps_the_rest(tmp_path: Path):
    out = tmp_path / "evalsets/reference-consistency/pets-v1"
    owner = tmp_path / "owner"
    gens = {"fake": FakeGenerator()}
    build.build(plan_mod.parse(PLAN), out, owner, generators=gens)
    before_ref = (out / "families/pip/ref-1.png").read_bytes()
    before_bridge = (out / "items/pip-bridge/candidate.png").read_bytes()
    draft_path = owner / "reference-consistency/pets-v1/labels-draft.toml"
    # The owner struck an item and changed the plan's edit prompt; rebuild that item only.
    text = draft_path.read_text().replace('status = "draft"', 'status = "confirmed"')
    draft_path.write_text(text)
    changed = PLAN.replace(
        "remove the white patch. Change nothing else.", "blend the patch away. Change nothing else."
    )
    report = build.rebuild(
        plan_mod.parse(changed),
        out,
        owner,
        generators=gens,
        item_id="pip-bridge-no-patch",
        seed_offset=5,
    )
    assert report.items == 1
    assert (out / "families/pip/ref-1.png").read_bytes() == before_ref, "references untouched"
    assert (out / "items/pip-bridge/candidate.png").read_bytes() == before_bridge, (
        "other items untouched"
    )
    job = tomllib.loads((out / "items/pip-bridge-no-patch/job.toml").read_text())["job"]
    cand = next(i for i in job["images"] if i["id"] == "candidate")
    assert (
        hashlib.sha256((out / cand["source"]["path"]).read_bytes()).hexdigest()
        == cand["source"]["sha256"]
    )
    record = tomllib.loads((owner / "reference-consistency/pets-v1/construction.toml").read_text())
    entry = next(i for i in record["item"] if i["id"] == "pip-bridge-no-patch")
    assert "blend the patch away" in entry["op"]["prompt"]
    assert entry["seed_offset"] == 5
    assert [i["id"] for i in record["item"]] == [
        "pip-bridge",
        "pip-bridge-no-patch",
        "pip-bridge-lookalike",
    ], "order kept"
    draft = tomllib.loads(draft_path.read_text())
    by_item = {label["item"]: label for label in draft["label"]}
    assert by_item["pip-bridge-no-patch"]["status"] == "draft", (
        "the rebuilt item goes back to draft"
    )
    assert by_item["pip-bridge"]["status"] == "confirmed", "other labels keep their review"
    assert (owner / "reference-consistency/pets-v1/contact-pip.png").exists()
    # An item the plan does not have is refused.
    try:
        build.rebuild(plan_mod.parse(changed), out, owner, generators=gens, item_id="nope")
    except ValueError as e:
        assert "nope" in str(e)
    else:
        raise AssertionError("rebuilt an unknown item")
