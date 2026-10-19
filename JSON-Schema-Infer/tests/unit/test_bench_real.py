"""Tests of the real-data benchmark (task 29.2; Requirements 33.1–33.5, 33.7–33.9)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from jsinfer.bench.real import (
    CleanedDataset,
    DatasetExclusion,
    RealConfig,
    RealDataset,
    clean,
    evaluate,
    folds,
    load_manifest,
    run_real,
    schemastore_datasets,
)
from jsinfer.bench.runner import CommandAdapter
from jsinfer.bench.splits import CorpusEntry, CorpusManifest

SCHEMA = {"type": "object", "properties": {"id": {"type": "integer"}, "email": {"type": "string", "format": "email"}}, "required": ["id"]}
POSITIVES = [{"id": i, "email": f"u{i}@x.org"} for i in range(6)]
ACCEPT_ALL = "import json, sys\njson.dump({}, open(sys.argv[2] + '/schema.json', 'w'))\n"
REJECT_ALL = "import json, sys\njson.dump(False, open(sys.argv[2] + '/schema.json', 'w'))\n"
COPY = "import shutil, sys\nshutil.copy(sys.argv[4], sys.argv[2] + '/schema.json')\n"
FAIL = "import sys\nsys.exit(3)\n"


def _adapter(tmp_path: Path, name: str, body: str, *extra: str) -> CommandAdapter:
    script = tmp_path / f"{name}.py"
    script.write_text(body, encoding="utf-8")
    return CommandAdapter(name, name, (sys.executable, "-I", str(script), "{docs}", "{out}", "{seed}", *extra))


def _raw(documents: list) -> tuple[tuple[str, bytes], ...]:
    return tuple((f"d{i}", json.dumps(d).encode()) for i, d in enumerate(documents))


def test_clean_excludes_and_records_each_kind() -> None:
    documents = [*_raw(POSITIVES), ("bad", b"{not json"), ("rejected", b'{"id": "x"}')]
    negatives = [("n0", b'{"id": "no"}'), ("n1", b'{"id": 1, "email": "not-an-email"}'), ("accepted", b'{"id": 7}'), ("nbad", b"[")]
    dataset = RealDataset("ds", tuple(documents), SCHEMA, tuple(negatives))
    cleaned = clean(dataset)
    assert isinstance(cleaned, CleanedDataset)
    assert [i for i, _ in cleaned.positives] == [f"d{i}" for i in range(6)]
    assert [i for i, _ in cleaned.negatives] == ["n0", "n1"]  # format is asserted: n1 stays a negative (33.5)
    assert [(e.document_id, e.reason) for e in cleaned.excluded] == [
        ("bad", "parse failure"), ("rejected", "positive rejected"), ("accepted", "negative accepted"), ("nbad", "parse failure"),
    ]
    assert cleaned.excluded[0].detail and cleaned.excluded[0].to_json()["dataset_id"] == "ds"


def test_clean_excludes_whole_datasets() -> None:
    too_few = clean(RealDataset("few", _raw(POSITIVES[:1]) + (("r", b'{"id": "x"}'),), SCHEMA))
    assert isinstance(too_few, DatasetExclusion) and too_few.remaining == 1 and "fewer than 2" in too_few.reason
    unusable = clean(RealDataset("bad", _raw(POSITIVES), {"type": "nonsense"}))
    assert isinstance(unusable, DatasetExclusion) and "cannot be used for validation" in unusable.reason


def test_folds_partition_deterministically_into_min_k_n() -> None:
    parts = folds(10, 4, "ds", seed=3)
    assert len(parts) == 4 and sorted(i for p in parts for i in p) == list(range(10))
    assert parts == folds(10, 4, "ds", seed=3)
    assert parts != folds(10, 4, "ds", seed=4) or parts != folds(10, 4, "other", seed=3)
    assert sorted(folds(3, 5, "ds")) == [[0], [1], [2]]
    assert len(folds(3, 5, "ds")) == 3
    with pytest.raises(ValueError, match="k must"):
        folds(3, 1, "ds")


def test_evaluate_cross_validates_every_system_on_the_same_folds(tmp_path: Path) -> None:
    dataset = clean(RealDataset("ds", _raw(POSITIVES), SCHEMA, (("n0", b'{"id": "no"}'),)))
    assert isinstance(dataset, CleanedDataset)
    truth = tmp_path / "truth.json"
    truth.write_text(json.dumps(SCHEMA))
    systems = [
        _adapter(tmp_path, "copy", COPY, str(truth)),
        _adapter(tmp_path, "accept", ACCEPT_ALL),
        _adapter(tmp_path, "reject", REJECT_ALL),
        _adapter(tmp_path, "broken", FAIL),
    ]
    cfg = RealConfig(folds=3, seed=1, timeout_s=120)
    results = {r.system: r for r in evaluate(dataset, systems, tmp_path / "runs", cfg)}
    assert all(r.n == 6 and r.fold_count == 3 and r.negatives_source == "supplied" and r.negatives == 1 for r in results.values())
    assert results["copy"].recall == 1.0 and results["copy"].rejection == 1.0 and results["copy"].harmonic == 1.0
    assert results["accept"].recall == 1.0 and results["accept"].rejection == 0.0 and results["accept"].harmonic == 0.0
    assert results["reject"].recall == 0.0 and results["reject"].rejection == 1.0 and results["reject"].harmonic == 0.0
    assert results["copy"].fold_rejection == (1.0, 1.0, 1.0)
    assert results["broken"].missing == "failure" and results["broken"].reason == "exit status 3" and results["broken"].recall is None
    for i in range(3):
        lines = (tmp_path / "runs" / f"fold{i}" / "docs.jsonl").read_text().splitlines()
        assert len(lines) == 4
        assert (tmp_path / "runs" / f"fold{i}" / "copy" / "schema-2020-12.json").exists()
    assert results["copy"].to_json()["fold_rejection"] == [1.0, 1.0, 1.0]


def test_negatives_are_generated_when_none_are_supplied(tmp_path: Path) -> None:
    dataset = clean(RealDataset("ds", _raw(POSITIVES), SCHEMA))
    assert isinstance(dataset, CleanedDataset) and not dataset.negatives
    accept = _adapter(tmp_path, "accept", ACCEPT_ALL)
    [result] = evaluate(dataset, [accept], tmp_path / "runs", RealConfig(folds=2, timeout_s=120))
    assert result.negatives_source == "generated" and result.negatives > 0
    assert result.rejection == 0.0 and result.harmonic == 0.0


def test_no_negatives_at_all_leaves_rejection_not_available(tmp_path: Path) -> None:
    dataset = clean(RealDataset("ds", _raw([True, False, True]), {}))  # nothing to mutate
    assert isinstance(dataset, CleanedDataset)
    accept = _adapter(tmp_path, "accept", ACCEPT_ALL)
    [result] = evaluate(dataset, [accept], tmp_path / "runs", RealConfig(folds=5, timeout_s=120))
    assert result.negatives_source == "none" and result.fold_count == 3
    assert result.recall == 1.0 and result.rejection is None and result.harmonic is None


def test_manifest_definitions_and_their_exclusions(tmp_path: Path) -> None:
    (tmp_path / "docs.jsonl").write_text("".join(json.dumps(d) + "\n" for d in POSITIVES))
    (tmp_path / "docs").mkdir()
    for i, d in enumerate(POSITIVES[:2]):
        (tmp_path / "docs" / f"{i}.json").write_text(json.dumps(d))
    (tmp_path / "schema.json").write_text(json.dumps(SCHEMA))
    (tmp_path / "neg.jsonl").write_text('{"id": "no"}\n')
    manifest = {
        "datasets": [
            {"id": "a", "documents": "docs.jsonl", "schema": "schema.json", "negatives": "neg.jsonl", "ordering_key": "$.id"},
            {"id": "b", "documents": "docs", "schema": "schema.json"},
            {"id": "a", "documents": "docs.jsonl", "schema": "schema.json"},
            {"id": "c", "schema": "schema.json"},
            {"id": "d", "documents": "missing.jsonl", "schema": "schema.json"},
            {"documents": "docs.jsonl", "schema": "schema.json"},
        ]
    }
    (tmp_path / "manifest.json").write_text(json.dumps(manifest))
    datasets, exclusions = load_manifest(tmp_path / "manifest.json")
    assert [d.id for d in datasets] == ["a", "b"]
    assert len(datasets[0].documents) == 6 and datasets[0].ordering_key == "$.id" and len(datasets[0].negatives) == 1
    assert [i for i, _ in datasets[1].documents] == [str(tmp_path / "docs" / "0.json"), str(tmp_path / "docs" / "1.json")]
    assert [(e.dataset_id, e.reason.split(":")[0]) for e in exclusions] == [
        ("a", "the identifier repeats an earlier definition"),
        ("c", "the definition lacks documents"),
        ("d", "a referenced file cannot be read"),
        ("#5", "the definition has no identifier"),
    ]


def test_schemastore_datasets_from_the_corpus_manifest(tmp_path: Path) -> None:
    root = tmp_path / "corpus"
    (root / "tests").mkdir(parents=True)
    (root / "s.json").write_text(json.dumps(SCHEMA))
    (root / "tests" / "p.json").write_text(json.dumps(POSITIVES[0]))
    (root / "tests" / "n.json").write_text('{"id": "no"}')
    entries = [
        CorpusEntry("schemastore/s", "schemastore", "s.json", {"tests": {"positive": ["tests/p.json"], "negative": ["tests/n.json"]}}),
        CorpusEntry("schemastore/none", "schemastore", "s.json", {"tests": {"positive": [], "negative": []}}),
        CorpusEntry("jsonschemabench/x", "jsonschemabench", "s.json", {"tests": {"positive": ["tests/p.json"], "negative": []}}),
        CorpusEntry("schemastore/gone", "schemastore", "s.json", {"tests": {"positive": ["tests/missing.json"], "negative": []}}),
    ]
    datasets, exclusions = schemastore_datasets(CorpusManifest.of(entries, root))
    assert [d.id for d in datasets] == ["schemastore/s"]
    assert datasets[0].source == "schemastore" and len(datasets[0].negatives) == 1
    assert [e.dataset_id for e in exclusions] == ["schemastore/gone"]


def test_run_real_collects_results_and_exclusions(tmp_path: Path) -> None:
    datasets = [
        RealDataset("ok", _raw(POSITIVES), SCHEMA, (("n0", b'{"id": "no"}'),)),
        RealDataset("few", _raw(POSITIVES[:1]), SCHEMA),
    ]
    accept = _adapter(tmp_path, "accept", ACCEPT_ALL)
    out = run_real(datasets, [accept], RealConfig(folds=2, timeout_s=120), tmp_path / "runs", exclusions=[DatasetExclusion("x", "the definition lacks schema")])
    assert [r.dataset_id for r in out.results] == ["ok"]
    assert out.systems == ["accept"] and out.missing("accept") == {}
    assert out.aggregate("accept", "recall").mean == 1.0 and out.aggregate("accept", "harmonic").mean == 0.0
    assert out.exclusion_reasons() == {"fewer than 2 positive documents remain": 1, "the definition lacks schema": 1}
    assert out.to_json()["dataset_exclusions"][1]["remaining"] == 1
    with pytest.raises(ValueError, match="unknown metric"):
        out.aggregate("accept", "precision")
