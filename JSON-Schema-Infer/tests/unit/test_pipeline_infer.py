"""Unit tests for pipeline/infer.py: pipeline orchestration (task 16.2).

Requirements 1.8 (no documents), 3.2 (soundness before emission), 17.6 (zero
Flagged_Items while every outlier Technique is off), 18.10 (change-point
detection not applied), 27.7 (a completed run yields the schema and the
report), 27.8 (stage runtimes by component name), 27.10 and 27.13
(unrecoverable stage errors carry the stage and the partial report), plus
determinism across worker counts and chunk sizes (27.5, 27.12) and equality
with Naive_Merge when every Technique is off (28.5).
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from jsinfer.config.errors import ConfigError
from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.errors import InferenceError, NoDocumentsError
from jsinfer.core.provenance import BASELINE_PREFIX, FALLBACK_PREFIX
from jsinfer.core.report import InferenceReport, VersionsInfo
from jsinfer.core.schema_model import SchemaModel
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Draft, Validator
from jsinfer.pipeline import infer as pipeline
from jsinfer.pipeline.infer import (
    ANALYZER_STAGE,
    BUNDLE_STAGE,
    CONFIG_STAGE,
    DOCUMENT_OUTLIER,
    EMITTER_STAGE,
    FLATTEN_STAGE,
    INGESTOR_STAGE,
    OUTLIER_TECHNIQUES,
    SOUNDNESS_STAGE,
    STAGES,
    VERSION_SPLIT,
    InferenceContext,
    JsonlSource,
    PipelineResult,
    ValueSource,
    infer_documents,
    infer_paths,
    run_pipeline,
)
from jsinfer.pipeline.naive_merge import naive_merge
from jsinfer.structure.analyzer import StructureAnalyzer

VALIDATOR = Validator()
ONE_WORKER = RunConfiguration.resolve({"workers": 1})
ALL_OFF = RunConfiguration.resolve({"workers": 1, **{technique_setting(tid): False for tid in REGISTRY.ids()}})

# Records with optional fields, nested arrays of records, a Map, Tuples, and mixed types.
ORDERS: list[Any] = [
    {
        "id": i,
        "status": ["new", "paid", "sent"][i % 3],
        "lines": [{"sku": f"s{j}", "qty": j % 3 + 1} for j in range(i % 4)],
        "meta": {"tags": ["a", "b"][: i % 3], "source": "web" if i % 2 else "app"},
        **({"note": f"note {i}"} if i % 5 == 0 else {}),
    }
    for i in range(24)
]
MIXED: list[Any] = [
    *ORDERS[:6],
    *({f"user{3 * i + j}": {"score": i + j} for j in range(3)} for i in range(8)),
    *([i, f"n{i}"] for i in range(8)),
    1,
    2.5,
    "text",
    None,
    True,
]


def config(**flags: Any) -> RunConfiguration:
    return RunConfiguration.resolve({"workers": 1, **flags})


def all_valid(result: PipelineResult, documents: list[Any]) -> bool:
    compiled = VALIDATOR.compile(result.document)
    return all(VALIDATOR.is_valid(compiled, doc) for doc in documents)


def provenance_by_pointer(result: PipelineResult) -> dict[str, Any]:
    """Every Provenance_Record keyed by its emitted JSON Pointer (independent of nids)."""
    return {result.pointers[cid]: record.to_json() for cid, record in result.model.provenance.items()}


# ---------------------------------------------------------------------------
# Stages and timings (27.7, 27.8)
# ---------------------------------------------------------------------------


def test_stages_run_in_order_and_are_timed_by_component(monkeypatch: pytest.MonkeyPatch) -> None:
    started: list[str] = []
    original = InferenceReport.stage

    def spy(self: InferenceReport, component: str) -> Any:
        started.append(component)
        return original(self, component)

    monkeypatch.setattr(InferenceReport, "stage", spy)
    result = infer_documents(ORDERS, ONE_WORKER)
    assert tuple(started) == STAGES
    assert set(result.report.timings) == set(STAGES)
    assert all(seconds >= 0 for seconds in result.report.timings.values())
    assert result.report.ok and result.report.current_stage is None
    assert result.report.to_json()["status"] == {"state": "ok"}


def test_a_completed_run_returns_the_schema_and_the_report() -> None:
    result = infer_documents(ORDERS, ONE_WORKER)
    assert isinstance(result.model, SchemaModel) and result.config == ONE_WORKER
    assert result.schema == SchemaEmitter().dumps(result.document)
    assert json.loads(result.schema) == result.document
    assert set(result.pointers) == set(result.model.provenance)  # one record per emitted Constraint
    assert len(result.documents) == len(ORDERS) == result.store.n_docs == len(result.origins)
    assert VALIDATOR.meta_errors(result.document, Draft.DRAFT202012) == []
    run = result.report.run
    assert run is not None and run.seed == ONE_WORKER.seed and run.bundle_id
    assert run.config == ONE_WORKER.to_json()
    resources = result.report.resources
    assert resources["workers"].value == 1 and resources["workers"].origin == "flag"
    assert resources["ingest.chunk_size"].origin == "default"
    assert resources["soundness.max_passes"].origin == "default"


def test_no_config_means_every_default() -> None:
    result = infer_documents([{"a": 1}, {"a": 2}])
    assert result.config == RunConfiguration.default()
    assert result.report.ok


def test_the_context_derives_streams_from_the_run_seed() -> None:
    ctx = InferenceContext(config(seed=7))
    assert ctx.rng("x", 1).integers(1 << 30) == InferenceContext(config(seed=7)).rng("x", 1).integers(1 << 30)
    assert ctx.rng("x", 1).integers(1 << 30) != InferenceContext(config(seed=8)).rng("x", 1).integers(1 << 30)
    with pytest.raises(RuntimeError):
        ctx.engine


# ---------------------------------------------------------------------------
# Sources
# ---------------------------------------------------------------------------


def test_values_jsonl_bytes_and_files_give_the_same_run(tmp_path: Path) -> None:
    lines = "\n".join(json.dumps(doc) for doc in ORDERS)
    path = tmp_path / "orders.jsonl"
    path.write_text(lines + "\n\n", encoding="utf-8")
    results = [
        infer_documents(ORDERS, ONE_WORKER),
        infer_documents(lines.encode(), ONE_WORKER),
        infer_paths([path], ONE_WORKER),
        run_pipeline(ValueSource(iter(ORDERS)), ONE_WORKER),
        run_pipeline(JsonlSource(lines.encode(), "orders"), ONE_WORKER),
    ]
    first = results[0]
    for other in results[1:]:
        assert other.schema == first.schema and other.model == first.model
        assert other.documents == first.documents
        assert provenance_by_pointer(other) == provenance_by_pointer(first)
        assert other.report.to_json()["decisions"] == first.report.to_json()["decisions"]
    assert [origin.source for origin in results[2].origins] == [str(path)] * len(ORDERS)
    assert {origin.source for origin in results[4].origins} == {"orders"}


def test_skipped_documents_are_recorded_and_the_run_continues() -> None:
    result = infer_documents([{"a": 1}, float("nan"), {"a": 2}], ONE_WORKER)
    assert len(result.documents) == 2
    assert [(error.source, error.line) for error in result.report.ingest_errors] == [("<documents>", 2)]
    assert result.report.skipped == 1
    jsonl = infer_documents(b'{"a": 1}\n{oops\n{"a": 2}\n', ONE_WORKER)
    assert [error.line for error in jsonl.report.ingest_errors] == [2]
    assert jsonl.schema == result.schema


def test_sources_and_configs_are_type_checked() -> None:
    with pytest.raises(TypeError):
        infer_documents("not documents")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        run_pipeline([{"a": 1}])  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        infer_documents([1], {"workers": 1})  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        infer_paths("one/path.json")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Errors (1.8, 27.10, 27.13)
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("documents", [[], b"", b"\n \t\n", [float("nan"), float("inf")]], ids=repr)
def test_empty_input_raises_no_documents_with_the_partial_report(documents: Any) -> None:
    with pytest.raises(NoDocumentsError) as caught:
        infer_documents(documents, ONE_WORKER)
    error = caught.value
    assert isinstance(error, InferenceError) and error.stage == INGESTOR_STAGE
    assert "no valid documents" in str(error)
    report = error.partial_report
    assert report is not None and not report.ok
    assert report.status.stage == INGESTOR_STAGE
    assert report.status.error is not None and report.status.error.type.endswith("NoDocumentsError")
    assert set(report.timings) == {CONFIG_STAGE, BUNDLE_STAGE}  # the completed stages
    assert report.ingest_errors == error.errors and report.skipped == error.skipped
    assert report.soundness == () and report.run is not None


def _raise(*args: Any, **kwargs: Any) -> Any:
    raise RuntimeError("injected failure")


@pytest.mark.parametrize(
    ("stage", "target", "name"),
    [
        (FLATTEN_STAGE, pipeline, "merge_partials"),
        (pipeline.OUTLIER_STAGE, pipeline, "_detect_outliers"),
        (ANALYZER_STAGE, StructureAnalyzer, "infer_model"),
        (SOUNDNESS_STAGE, pipeline, "check_soundness"),
        (EMITTER_STAGE, pipeline, "SchemaEmitter"),  # the soundness check lays out with its own emitter
    ],
)
def test_a_stage_failure_becomes_an_inference_error(
    monkeypatch: pytest.MonkeyPatch, stage: str, target: Any, name: str
) -> None:
    monkeypatch.setattr(target, name, _raise)
    with pytest.raises(InferenceError) as caught:
        infer_documents(ORDERS, ONE_WORKER)
    error = caught.value
    assert type(error) is InferenceError and error.stage == stage
    assert stage in str(error) and "injected failure" in str(error)
    assert isinstance(error.__cause__, RuntimeError)
    report = error.partial_report
    assert report is not None and report.status.stage == stage
    assert report.status.error is not None
    assert (report.status.error.type, report.status.error.message) == ("RuntimeError", "injected failure")
    assert report.to_json()["status"]["state"] == "failed"
    completed = STAGES[: STAGES.index(stage)]
    assert set(report.timings) == set(completed)  # completed stages with their runtimes, not the failed one


def test_configuration_errors_are_not_stage_errors(tmp_path: Path) -> None:
    with pytest.raises(ConfigError) as caught:
        infer_documents([1], config(model_bundle=str(tmp_path / "no-bundle")))
    assert not isinstance(caught.value, InferenceError)
    with pytest.raises(ConfigError):
        infer_paths([tmp_path / "missing.jsonl"], ONE_WORKER)  # 1.10: a rejected supplied path


# ---------------------------------------------------------------------------
# Outlier and change-point hooks (17.6, 18.10)
# ---------------------------------------------------------------------------


def test_no_flagged_items_and_change_point_detection_not_applied() -> None:
    assert not any(REGISTRY.technique(tid).implemented for tid in (*OUTLIER_TECHNIQUES, "robust.mdl_exceptions"))
    result = infer_documents(MIXED, ONE_WORKER)
    report = result.report
    assert report.flagged_items == () and result.flagged == () and not report.outliers_abandoned
    assert report.to_json()["outliers"] == {"abandoned": False, "discarded": 0, "items": []}
    assert report.versions == VersionsInfo.not_applied()
    assert report.to_json()["versions"] == {"applied": False}
    (summary,) = report.soundness
    assert summary.version is None and summary.flagged_documents == 0 and summary.validated == len(MIXED)
    hooks = {entry.kind: entry for entry in report.decisions if entry.kind in (DOCUMENT_OUTLIER, VERSION_SPLIT)}
    assert set(hooks) == {DOCUMENT_OUTLIER, VERSION_SPLIT}
    for entry in hooks.values():
        assert entry.path == "$" and entry.fallback is not None and entry.fallback.reason == "disabled"
        assert entry.resolution.method == "fallback" and entry.cutoff is None


# ---------------------------------------------------------------------------
# Soundness (3.2)
# ---------------------------------------------------------------------------


def test_soundness_runs_before_emission_over_every_document() -> None:
    result = infer_documents(MIXED, ONE_WORKER)
    (summary,) = result.report.soundness
    assert summary == result.soundness.summary
    assert summary.validated == len(MIXED) and result.soundness.validated == tuple(range(len(MIXED)))
    assert all_valid(result, MIXED)


def test_soundness_relaxes_an_over_tight_model_before_emission(monkeypatch: pytest.MonkeyPatch) -> None:
    original = StructureAnalyzer.infer_model

    def over_tight(self: StructureAnalyzer, view: Any) -> SchemaModel:
        model = original(self, view)
        model.root.kw["maxProperties"] = 0  # rejects every non-empty object
        return model

    monkeypatch.setattr(StructureAnalyzer, "infer_model", over_tight)
    result = infer_documents(ORDERS, ONE_WORKER)
    summary = result.soundness.summary
    assert summary.failed_first_pass == len(ORDERS) and summary.passes >= 2
    assert summary.relaxations + summary.removals >= 1 and len(result.soundness.changes) >= 1
    assert result.document.get("maxProperties", 1) != 0  # type: ignore[union-attr]
    assert all_valid(result, ORDERS)
    assert result.report.soundness == (summary,)


# ---------------------------------------------------------------------------
# Determinism (27.5, 27.12) and all Techniques off (28.5)
# ---------------------------------------------------------------------------


def _sections(report: InferenceReport, *drop: str) -> dict[str, Any]:
    data = report.comparable_json()
    for name in drop:
        del data[name]
    return data


def test_runs_do_not_depend_on_workers_or_chunk_size() -> None:
    base = infer_documents(MIXED, config(**{"ingest.chunk_size": 2000}))
    for workers, chunk in ((1, 1), (2, 3), (1, 3)):
        other = infer_documents(MIXED, config(workers=workers, **{"ingest.chunk_size": chunk}))
        assert other.schema == base.schema and other.model == base.model
        assert other.documents == base.documents
        assert provenance_by_pointer(other) == provenance_by_pointer(base)
        # The chunk size is a setting of its own, so only the sections that record settings differ.
        assert _sections(other.report, "run", "resources") == _sections(base.report, "run", "resources")
    one = infer_documents(MIXED, config(workers=1, **{"ingest.chunk_size": 3}))
    two = infer_documents(MIXED, config(workers=2, **{"ingest.chunk_size": 3}))
    assert one.report.comparable_json() == two.report.comparable_json()  # 27.12: only the worker count differs
    assert one.report.to_bytes() != b"" and one.schema == two.schema


def test_every_technique_off_equals_naive_merge_after_soundness() -> None:
    for documents in (ORDERS, MIXED):
        result = infer_documents(documents, ALL_OFF)
        expected = naive_merge(documents)
        assert result.model == expected
        assert result.schema == SchemaEmitter().emit(expected)
        assert result.soundness.summary.failed_first_pass == 0
        assert all(
            record.technique.startswith((FALLBACK_PREFIX, BASELINE_PREFIX)) for record in result.model.provenance.values()
        )
