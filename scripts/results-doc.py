#!/usr/bin/env python3 -I
"""Writes the results document of a finished evaluation (PLAN.md task 3.8): every metric from
`dejaview eval report`, development and adversarial sets reported separately, the resolved model
ids, hosts and API versions from the decision headers, the cost per cell, and copies of the
matrix, summary and metrics beside it.

    scripts/results-doc.py <eval dir> <matrix.toml> docs/results/<name> [--bin target/debug/dejaview]
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import shutil
import subprocess


def report(binary: str, eval_dir: pathlib.Path, sets: list[str]) -> str:
    args = [binary, "eval", "report", "--out", str(eval_dir), "--sets", ",".join(sets)]
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("eval_dir", type=pathlib.Path)
    ap.add_argument("matrix", type=pathlib.Path)
    ap.add_argument("out", type=pathlib.Path, help="the results directory to create")
    ap.add_argument("--bin", default="target/debug/dejaview")
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
    shutil.copy(a.eval_dir / "summary.json", a.out / "summary.json")
    shutil.copy(a.eval_dir / "metrics.json", a.out / "metrics.json")
    shutil.copy(a.eval_dir / "descriptor.json", a.out / "descriptor.json")

    lines = [
        f"# {summary['matrix']}: arm A baseline",
        "",
        "**These are development sets.** Every item here was built, reviewed and labelled by the",
        "owner, and the sets are public; prompts and tools are tuned against them, so these numbers",
        "describe the harness on material it may have been shaped by, not on a blind set. The",
        "adversarial sets are reported separately below. Repetition 1 is the headline; repetition 2",
        "is reported as agreement.",
        "",
        f"- Matrix identity `{summary['matrix_identity']}`, descriptor `{summary['descriptor_sha256']}`",
        f"- Harness {descriptor['harness_version']}, prompt {descriptor['prompt_version']}, catalog version {descriptor['catalog_version']}",
        f"- {len(summary['decisions'])} decisions; ${summary['spent_usd']:.4f} of ${summary['cap_usd']:.2f} spent; completed {summary['completed']}, resumed {summary['skipped']}, partial redone {len(summary['partial'])}",
        f"- Files beside this document: `matrix.toml`, `summary.json`, `metrics.json`, `descriptor.json`",
        "",
        "## Cells: resolved model, host, API version, cost",
        "",
        "| Cell | Profile | Host | Resolved model | API version | Decisions | Errors | Cost |",
        "|---|---|---|---|---|---|---|---|",
    ]
    counts = collections.Counter(d["cell"] for d in summary["decisions"])
    for cell in sorted(counts):
        r = recipients.get(cell, {})
        lines.append(
            f"| {cell} | {r.get('profile', '')} | {r.get('host', '')} | {r.get('model', '')} | {r.get('api_version', '')} | {counts[cell]} | {errors[cell]} | ${cost[cell]:.4f} |"
        )
    lines += ["", "## Development sets", "", f"Sets: {', '.join(dev)}", ""]
    lines.append(report(a.bin, a.eval_dir, dev).split("\n", 1)[1].strip())
    lines += ["", "## Adversarial sets", "", f"Sets: {', '.join(adversarial)}", ""]
    if adversarial:
        lines.append(report(a.bin, a.eval_dir, adversarial).split("\n", 1)[1].strip())
    lines += ["", "## Every set together", ""]
    lines.append((a.eval_dir / "report.md").read_text().split("\n", 1)[1].strip())
    (a.out / "README.md").write_text("\n".join(lines) + "\n")
    print(f"wrote {a.out / 'README.md'}")


if __name__ == "__main__":
    main()
