#!/usr/bin/env -S python3 -I
"""Writes the results document of a finished evaluation (PLAN.md tasks 3.8 and 4.5): every metric from
`dejaview eval report`, development and adversarial sets reported separately, the resolved model
ids, hosts and API versions from the decision headers, the cost per cell, and copies of the
matrix, summary and metrics beside it.

    scripts/results-doc.py <eval dir> <matrix.toml> docs/results/<name> --title "arm A baseline" [--bin target/debug/dejaview] [--notes notes.md]

`--title` finishes the heading after the matrix name. `--notes` names a Markdown file whose body becomes the document's "Notes" section: how the run
went (resumes, refused calls, what the ledger counted) and what a reader should know before the
numbers. It is copied beside the document as `notes.md`.
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import pathlib
import shutil
import subprocess


def report(binary: str, eval_dir: pathlib.Path, evalsets_root: str, sets: list[str]) -> str:
    args = [binary, "eval", "report", "--out", str(eval_dir), "--evalsets-root", evalsets_root, "--sets", ",".join(sets)]
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("eval_dir", type=pathlib.Path)
    ap.add_argument("matrix", type=pathlib.Path)
    ap.add_argument("out", type=pathlib.Path, help="the results directory to create")
    ap.add_argument("--title", required=True, help="what the run was, after the matrix name in the heading")
    ap.add_argument("--bin", default="target/debug/dejaview")
    ap.add_argument("--evalsets-root", default=os.environ.get("DEJAVIEW_EVALSETS_ROOT", "evalsets"))
    ap.add_argument("--notes", type=pathlib.Path, help="a Markdown file for the Notes section")
    a = ap.parse_args()
    summary = json.loads((a.eval_dir / "summary.json").read_text())
    descriptor = json.loads((a.eval_dir / "descriptor.json").read_text())
    sets = sorted({d["set"] for d in summary["decisions"]})
    dev = [s for s in sets if not s.endswith("/adversarial-v1")]
    adversarial = [s for s in sets if s.endswith("/adversarial-v1")]

    # Resolved recipients and costs per cell, from the decision headers and the summary.
    recipients: dict[str, dict] = {}
    cost = collections.defaultdict(float)
    errors = collections.defaultdict(int)
    for d in summary["decisions"]:
        cost[d["cell"]] += d["cost_usd"]
        if d["error_class"]:
            errors[d["cell"]] += 1
        if d["cell"] not in recipients:
            header = a.eval_dir / d["dir"] / "decision.json"
            if header.exists():
                r = json.loads(header.read_text()).get("recipient")
                if r:
                    recipients[d["cell"]] = r

    a.out.mkdir(parents=True, exist_ok=True)
    shutil.copy(a.matrix, a.out / "matrix.toml")
    # The published summary keeps every decision's error class but not the error text, which
    # may quote a provider's response body.
    published = json.loads(json.dumps(summary))
    eval_abs = str(a.eval_dir.resolve()) + os.sep
    for d in published["decisions"]:
        if d.get("error"):
            d["error"] = f"[{d['error_class']}: text withheld from the published copy]"
        # Summaries written before the runner relativised them carry the decision directory as an
        # absolute path (the run makes it absolute for the tools); the published copy is relative
        # to the evaluation directory, so no local path leaves here.
        if d["dir"].startswith(eval_abs):
            d["dir"] = d["dir"][len(eval_abs):]
    (a.out / "summary.json").write_text(json.dumps(published, indent=2) + "\n")
    shutil.copy(a.eval_dir / "metrics.json", a.out / "metrics.json")
    shutil.copy(a.eval_dir / "descriptor.json", a.out / "descriptor.json")

    repetitions = max(d["repetition"] for d in summary["decisions"])
    lines = [
        f"# {summary['matrix']}: {a.title}",
        "",
        "**These are development sets.** Every item here was built, reviewed and labelled by the",
        "owner, and the sets are public; prompts and tools are tuned against them, so these numbers",
        "describe the harness on material it may have been shaped by, not on a blind set. The",
        "adversarial sets are reported separately below. Repetition 1 is the headline"
        + ("; later repetitions are reported as agreement." if repetitions > 1 else "; there is one repetition."),
        "",
        f"- Matrix identity `{summary['matrix_identity']}`, descriptor `{summary['descriptor_sha256']}`",
        f"- Harness {descriptor['harness_version']}, prompt {descriptor['prompt_version']}, catalog version {descriptor['catalog_version']}",
        f"- {len(summary['decisions'])} decisions; ${summary['spent_usd']:.4f} of ${summary['cap_usd']:.2f} spent; completed {summary['completed']}, resumed {summary['skipped']}, partial redone {len(summary['partial'])}",
        f"- Files beside this document: `matrix.toml`, `summary.json`, `metrics.json`, `descriptor.json`"
        + (", `notes.md`" if a.notes else ""),
    ]
    if a.notes:
        notes = a.notes.read_text().strip()
        if a.notes.resolve() != (a.out / "notes.md").resolve():
            (a.out / "notes.md").write_text(notes + "\n")
        lines += ["", "## Notes", "", notes]
    lines += [
        "",
        "## Cells: resolved model, host, API version, cost",
        "",
        "| Cell | Profile | Host | Resolved model | API version | Decisions | Errors | Cost |",
        "|---|---|---|---|---|---|---|---|",
    ]
    counts = collections.Counter(d["cell"] for d in summary["decisions"])
    for cell in sorted(counts):
        r = recipients.get(cell, {})
        profile = r.get("profile") or ("scripted" if cell.startswith("fake:") else "")
        lines.append(
            f"| {cell} | {profile} | {r.get('host', '')} | {r.get('model', '')} | {r.get('api_version', '')} | {counts[cell]} | {errors[cell]} | ${cost[cell]:.4f} |"
        )
    lines += ["", "## Development sets", "", f"Sets: {', '.join(dev)}", ""]
    lines.append(report(a.bin, a.eval_dir, a.evalsets_root, dev).split("\n", 1)[1].strip())
    lines += ["", "## Adversarial sets", "", f"Sets: {', '.join(adversarial)}", ""]
    if adversarial:
        lines.append(report(a.bin, a.eval_dir, a.evalsets_root, adversarial).split("\n", 1)[1].strip())
    else:
        lines.append("(none in this matrix)")
    lines += ["", "## Every set together", ""]
    lines.append((a.eval_dir / "report.md").read_text().split("\n", 1)[1].strip())
    (a.out / "README.md").write_text("\n".join(lines) + "\n")
    print(f"wrote {a.out / 'README.md'}")


if __name__ == "__main__":
    main()
