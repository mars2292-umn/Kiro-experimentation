"""Property 58: Results serialization and aggregation (task 30.4).

For any result set, the CSV and JSON outputs contain identical values;
timeouts, failures, and unavailable or undefined values are excluded from
the means and counted per system, metric, and Dataset_Group; paired
comparisons use only the datasets where both systems returned a schema
with a defined value.
"""

# Feature: json-schema-inference, Property 58: Results serialization and aggregation

from __future__ import annotations

import json
import math
import tempfile
from pathlib import Path

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.bench.reporting import Row, aggregate, read_csv, write_csv, write_json

SYSTEMS = ("jsinfer", "b1", "b2")
GROUPS = ("real", "synthetic@10", "synthetic@100")
STATUSES = ("ok", "ok", "ok", "failure", "timeout", "unavailable")
values = st.one_of(st.none(), st.floats(0, 1, allow_nan=False, allow_infinity=False))


@st.composite
def result_sets(draw: st.DrawFn) -> list[Row]:
    n_datasets = draw(st.integers(1, 6), label="datasets")
    rows: list[Row] = []
    for system in SYSTEMS:
        for group in draw(st.lists(st.sampled_from(GROUPS), min_size=1, max_size=3, unique=True), label=f"{system} groups"):
            for d in range(n_datasets):
                if draw(st.integers(0, 4), label=f"{system} {group} d{d} absent") == 0:
                    continue
                status = draw(st.sampled_from(STATUSES), label=f"{system} {group} d{d} status")
                metrics = {"recall": draw(values, label="recall"), "harmonic": draw(values, label="harmonic")} if status == "ok" else {"recall": None, "harmonic": None}
                rows.append(Row(system, system.upper(), f"d{d}", group, "default", 0, status, None if status == "ok" else status, metrics, draw(values, label="seconds"), None, draw(st.one_of(st.none(), st.integers(0, 50)), label="size"), None, {"n": d + 2}))
    return rows


@given(rows=result_sets())
def test_csv_and_json_identical_and_aggregates_exclude_missing(rows: list[Row]) -> None:
    with tempfile.TemporaryDirectory() as directory:
        out = Path(directory)
        write_csv(rows, out / "r.csv")
        write_json(rows, out / "r.json")
        assert read_csv(out / "r.csv") == json.loads((out / "r.json").read_text(encoding="utf-8"))
    agg = aggregate(rows, seed=0, resamples=20)
    event(f"{len(rows)} rows")
    for summary in agg.summaries:
        cell = [r for r in rows if r.system == summary.system and r.group == summary.group]
        defined = [r.metrics[summary.metric] for r in cell if r.ok and r.metrics[summary.metric] is not None]
        assert summary.summary.count == len(defined)
        assert summary.summary.undefined == sum(1 for r in cell if r.ok and r.metrics[summary.metric] is None)
        assert summary.missing_total == sum(1 for r in cell if not r.ok)
        for status in ("failure", "timeout", "unavailable"):
            assert summary.missing.get(status, 0) == sum(1 for r in cell if r.status == status)
        if defined:
            assert math.isclose(summary.summary.mean, sum(defined) / len(defined))
            assert (summary.summary.ci is None) == (len(defined) < 2)
        else:
            assert summary.summary.mean is None and summary.summary.ci is None
    for comparison in agg.comparisons:
        assert comparison.system != "jsinfer"
        reference = {r.dataset for r in rows if r.system == "jsinfer" and r.group == comparison.group and r.ok and r.metrics[comparison.metric] is not None}
        other = {r.dataset for r in rows if r.system == comparison.system and r.group == comparison.group and r.ok and r.metrics[comparison.metric] is not None}
        assert comparison.n_pairs == len(reference & other)
        assert (comparison.test is None) == (comparison.n_pairs < 2)
