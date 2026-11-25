#!/usr/bin/env python3
"""The traceability table of the PACEMAKER experiment (PM31.1; task 9.7).

Usage (from the workspace root): docs/pacemaker/trace.py [--check]

Reads requirements.md (every acceptance criterion PMn.m), design.md (the
sections that cite a criterion), tasks.md (the tasks that cite it and
whether each is checked; a checked task's note names its evidence), the
sources of the experiment (the files that cite it), and
traceability_notes.toml (the criteria recorded and not implemented, with
their reasons, and the criteria whose evidence is known to fall short of
them, each with what is short: task 23.1). Writes traceability.md.

--check writes nothing: it fails when traceability.md is not what would be
written, when a criterion has neither a task nor a recorded reason, or when the
notes name a criterion the requirements do not have.
"""
import os
import re
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
CODE = ["crates/pm-core", "crates/pm-hw", "crates/pm-judge", "apps/pacemaker", "apps/heart-sim", "apps/ppi-probe", "spikes/pm-timer",
        "crates/rsk-gen/src", "crates/rsk-model/src", "crates/rsk-confcheck/src", "crates/rsk-kernel/src/arch", "crates/rsk-entry/src",
        "xtask/src", "proofs/creusot"]
ID = re.compile(r"(?<![A-Za-z0-9])PM(\d+)\.(\d+)(?!\d)")


def read(name):
    return open(os.path.join(HERE, name), encoding="utf-8").read()


def criteria():
    """[(id, requirement title, stage, text)] in the document's order."""
    out = []
    number = title = stage = None
    in_criteria = False
    for line in read("requirements.md").splitlines():
        m = re.match(r"^### Requirement (\d+): (.*?) \((Stage[^)]*)\)", line)
        if m:
            number, title, stage = int(m.group(1)), m.group(2), m.group(3)
            in_criteria = False
            continue
        if line.startswith("#### Acceptance Criteria"):
            in_criteria = True
            continue
        if line.startswith("### ") or line.startswith("## "):
            in_criteria = False
        m = re.match(r"^(\d+)\. (.*)", line)
        if in_criteria and m and number is not None:
            text = re.sub(r"\s+", " ", m.group(2))
            out.append((f"PM{number}.{m.group(1)}", title, stage, text))
    return out


def tasks():
    """{criterion: [(task, checked)]} from the `_Requirements:_` lines."""
    out = {}
    task = checked = None
    for line in read("tasks.md").splitlines():
        m = re.match(r"^\s*- \[( |x)\] (\d+(?:\.\d+)?)\.? ", line)
        if m:
            task, checked = m.group(2), m.group(1) == "x"
            continue
        if "_Requirements:" in line and task is not None:
            for a, b in ID.findall(line):
                out.setdefault(f"PM{a}.{b}", []).append((task, checked))
    return out


def design():
    """{criterion: [section]} for design.md."""
    out = {}
    section = "the head of the document"
    for line in read("design.md").splitlines():
        m = re.match(r"^#{2,3} (.*)", line)
        if m:
            section = m.group(1)
            continue
        for a, b in ID.findall(line):
            sections = out.setdefault(f"PM{a}.{b}", [])
            if section not in sections:
                sections.append(section)
    return out


def code():
    """{criterion: [file]} over the experiment's sources."""
    out = {}
    for top in CODE:
        for root, dirs, files in os.walk(os.path.join(ROOT, top)):
            dirs[:] = sorted(d for d in dirs if d not in ("target", "gen") and not d.startswith("gen-") and d != "verif")
            for f in sorted(files):
                if not f.endswith((".rs", ".py", ".toml")):
                    continue
                path = os.path.join(root, f)
                try:
                    text = open(path, encoding="utf-8").read()
                except (UnicodeDecodeError, OSError):
                    continue
                for a, b in set(ID.findall(text)):
                    out.setdefault(f"PM{a}.{b}", []).append(os.path.relpath(path, ROOT))
    return out


def render():
    crit = criteria()
    by_task, by_design, by_code = tasks(), design(), code()
    notes = tomllib.loads(read("traceability_notes.toml"))
    recorded = notes.get("recorded", {})
    short = notes.get("short", {})
    rows = []
    counts = {"evidence": 0, "in part": 0, "open": 0, "recorded": 0}
    missing = []
    for cid, title, stage, text in crit:
        ts = by_task.get(cid, [])
        if cid in recorded:
            status = "recorded"
        elif not ts:
            status = "open"
            missing.append(cid)
        elif all(c for _, c in ts):
            status = "evidence"
        elif any(c for _, c in ts):
            status = "in part"
        else:
            status = "open"
        counts[status] += 1
        brief = text if len(text) <= 120 else text[:117].rstrip() + "..."
        brief = brief.replace("|", "\\|")
        task_cell = ", ".join(f"{t}{' ✓' if c else ''}" for t, c in ts) or "—"
        design_cell = "; ".join(by_design.get(cid, [])) or "—"
        files = by_code.get(cid, [])
        code_cell = "; ".join(f"`{f}`" for f in files[:4]) + (f"; +{len(files) - 4}" if len(files) > 4 else "") if files else "—"
        note = recorded.get(cid, "")
        if cid in short and status != "recorded":
            note = "**short of the criterion:** " + short[cid]
            counts["short"] = counts.get("short", 0) + 1
        rows.append(f"| {cid} | {stage} | {brief} | {design_cell} | {task_cell} | {code_cell} | {status}{': ' + note if note else ''} |")
    unknown = sorted((set(by_task) | set(short) | set(recorded)) - {c[0] for c in crit})
    head = f"""# Traceability

**PACEMAKER on rsk: every acceptance criterion with its design, tasks, code and status**

Generated by `docs/pacemaker/trace.py` from `requirements.md`, `design.md`, `tasks.md`, the experiment's sources and `traceability_notes.toml`; `trace.py --check` fails when this file is out of date or a criterion has neither a task nor a recorded reason. Do not edit.

How to read it. *Design*: the sections of `design.md` that cite the criterion. *Tasks*: the tasks of `tasks.md` that cite it; ✓ marks a checked task, whose `Done` note names the evidence and what it left open. *Code*: the source files that cite it. *Status*: `evidence` when every task that cites the criterion is checked, `in part` when some are, `open` when none is, `recorded` when the criterion is recorded and not implemented, with its reason. A status of `evidence` says that the evidence the tasks name exists, not that it is complete. Where the evidence is known to fall short of the criterion as it is worded, the status says so and what is short (task 23.1, from each task's notes of what it did not do, read again on 2026-11-25); a criterion without such a note has none known, which is not a proof that there is none.

{len(crit)} criteria: {counts['evidence']} with evidence, {counts['in part']} in part, {counts['open']} open, {counts['recorded']} recorded and not implemented. Of those with evidence or in part, {counts.get('short', 0)} are marked as short of the criterion.

| Criterion | Stage | Text | Design | Tasks | Code | Status |
|---|---|---|---|---|---|---|
"""
    return head + "\n".join(rows) + "\n", missing, unknown, counts, len(crit)


def main():
    text, missing, unknown, counts, total = render()
    path = os.path.join(HERE, "traceability.md")
    problems = []
    if missing:
        problems.append(f"criteria with neither a task nor a recorded reason: {', '.join(missing)}")
    if unknown:
        problems.append(f"tasks.md or the notes cite criteria that requirements.md does not have: {', '.join(unknown)}")
    if "--check" in sys.argv:
        current = open(path, encoding="utf-8").read() if os.path.isfile(path) else ""
        if current != text:
            problems.append("traceability.md is not current: run docs/pacemaker/trace.py")
    else:
        open(path, "w", encoding="utf-8").write(text)
    print(f"{total} criteria: {counts}")
    for p in problems:
        print(f"error: {p}")
    sys.exit(1 if problems else 0)


main()
