"""Tests of the results files and summary tables (task 30.3; Requirements 33.6, 34.6, 34.9, 34.10, 35.7, 35.8, 35.12)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from jsinfer.bench.measure import schema_size
from jsinfer.bench.real import RealConfig, RealResult, RealResults, DatasetExclusion
from jsinfer.bench.reporting import Row, aggregate, latex, markdown, read_csv, real_rows, synthetic_rows, write_csv, write_json, write_report
from jsinfer.bench.synthetic import SyntheticConfig, SyntheticResult, SyntheticResults
from jsinfer.io.subschema import Relation


def _row(system: str, dataset: str, status: str = "ok", recall: float | None = 1.0, group: str = "real", **facts) -> Row:
    return Row(system, system.upper(), dataset, group, "default", 0, status, None if status == "ok" else status, {"recall": recall, "harmonic": None if recall is None else recall / 2}, 1.5, 20.0, 7, 12.5, facts)


ROWS = [
    _row("jsinfer", "d1"), _row("jsinfer", "d2", recall=0.5), _row("jsinfer", "d3", "timeout", None), _row("jsinfer", "d4", recall=None),
    _row("naive", "d1", recall=0.8), _row("naive", "d2", "failure", None), _row("naive", "d3", recall=0.2), _row("naive", "d4", recall=0.1),
    Row("recg", "ReCG reimplementation", "*", "real", "default", 0, "unavailable", "no runtime", {"recall": None, "harmonic": None}),
]


def test_csv_and_json_hold_identical_values(tmp_path: Path) -> None:
    write_csv(ROWS, tmp_path / "r.csv")
    write_json(ROWS, tmp_path / "r.json")
    from_json = json.loads((tmp_path / "r.json").read_text())
    from_csv = read_csv(tmp_path / "r.csv")
    assert from_csv == from_json
    assert list(from_json[0]) == ["system", "label", "dataset", "group", "config", "seed", "status", "reason", "harmonic", "recall", "seconds", "peak_rss_mb", "schema_size", "dl_bits"]


def test_aggregates_exclude_missing_and_undefined_and_pair_only_shared_datasets() -> None:
    agg = aggregate(ROWS, seed=1, resamples=100)
    s = agg.summary("jsinfer", "real", "recall")
    assert s is not None and s.summary.mean == 0.75 and s.summary.count == 2 and s.summary.undefined == 1 and s.missing == {"timeout": 1}
    n = agg.summary("naive", "real", "recall")
    assert n is not None and n.summary.count == 3 and n.missing == {"failure": 1}
    assert agg.summary("recg", "real", "recall").missing == {"unavailable": 1}
    [naive, recg] = [c for c in agg.comparisons if c.metric == "recall"]
    assert naive.system == "naive" and naive.n_pairs == 1 and naive.test is None  # only d1 is ok for both
    assert recg.n_pairs == 0 and recg.test is None
    assert agg.unavailable == ["recg"] and agg.systems == ["jsinfer", "naive", "recg"]


def test_tables_put_real_first_mark_unavailable_and_carry_labels(tmp_path: Path) -> None:
    synthetic = [_row("jsinfer", "s1", group="synthetic@10", recall=0.9), _row("naive", "s1", group="synthetic@10", recall=0.3)]
    rows = [*synthetic, *ROWS]
    real = RealResults(RealConfig(), dataset_exclusions=[DatasetExclusion("x", "the definition lacks schema")])
    agg = aggregate(rows, seed=0, resamples=50)
    assert agg.groups == ["real", "synthetic@10"]
    md = markdown(agg, rows, real)
    assert md.index("Real datasets (primary comparison)") < md.index("Synthetic benchmark, training size 10")
    assert "ReCG reimplementation (unavailable)" in md
    assert "Real datasets excluded" in md and "the definition lacks schema | 1" in md
    assert "Paired comparisons" in md and "not computable" in md
    tex = latex(agg, rows, real)
    assert "\\caption{Real datasets (primary comparison)}" in tex and "ReCG reimplementation (unavailable)" in tex
    assert tex.index("primary comparison") < tex.index("training size 10")
    written = write_report(rows, tmp_path, real=real, seed=0, resamples=50)
    assert {p.name for p in tmp_path.iterdir()} == {"results.csv", "results.json", "aggregates.csv", "aggregates.json", "summary.md", "summary.tex"}
    assert json.loads((tmp_path / "aggregates.json").read_text())["unavailable"] == ["recg"]
    assert written.reference == "jsinfer"


def test_rows_from_benchmark_results() -> None:
    cfg = SyntheticConfig(sizes=(3,), held_out=2, seed=7)
    results = SyntheticResults(cfg, [
        SyntheticResult("s1", 3, "jsinfer", recall=1.0, rejection=0.5, harmonic=2 / 3, relation=Relation.SUBSET, judgments={"record_or_map": 1.0}, seconds=1.0, peak_rss_mb=3.0, schema_size=9, dl_bits=40.0),
        SyntheticResult("s1", 3, "naive", missing="failure", reason="exit status 1"),
    ])
    rows = synthetic_rows(results, {"jsinfer": "Inferrer"}, unavailable=["jsonoid"])
    assert [(r.system, r.group, r.status) for r in rows] == [("jsinfer", "synthetic@3", "ok"), ("naive", "synthetic@3", "failure"), ("jsonoid", "synthetic@3", "unavailable")]
    assert rows[0].label == "Inferrer" and rows[0].metrics["judgment:record_or_map"] == 1.0 and rows[0].facts == {"relation": "subset"} and rows[0].schema_size == 9
    assert rows[1].metrics == {"recall": None, "rejection": None, "harmonic": None} and rows[1].seed == 7
    real = RealResults(RealConfig(folds=3, seed=7), [RealResult("ds", "jsinfer", 6, 3, 1, "generated", 4, recall=1.0, rejection=0.5, harmonic=2 / 3, schema_size=5.0, dl_bits=10.0)])
    [row] = real_rows(real)
    assert row.group == "real" and row.facts == {"n": 6, "folds": 3, "excluded": 1, "negatives_source": "generated", "negatives": 4}


def test_schema_size_counts_keywords_under_defs() -> None:
    doc = {"$schema": "x", "type": "object", "properties": {"a": {"type": "string", "format": "email"}}, "$defs": {"d": {"type": "integer", "minimum": 0}}, "allOf": [{"required": ["a"]}]}
    assert schema_size(doc) == 5 + 2 + 2 + 1
    assert schema_size(True) == 0


def test_row_rejects_unknown_status() -> None:
    with pytest.raises(ValueError, match="status"):
        Row("s", "s", "d", "real", "c", 0, "weird")
