"""Unit tests for the public API in ``jsinfer/__init__.py`` (task 18.1).

Requirements 27.9 (an in-memory run returns the Schema_Model, the
Provenance_Records, and the Inference_Report, equal to a run on a ``.jsonl``
file holding the same documents), 27.13 (typed errors that carry the stage
and the partial report), 1.7, 1.8, and 1.11 (skipped documents), 2.4 and
2.7 (load/emit round trip), and 26.5–26.8 (the explanation file and inline
provenance). The non-JSON-values decision (the module docstring of
``jsinfer``) is checked here too.
"""

from __future__ import annotations

import enum
import json
from collections import OrderedDict
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest

import jsinfer
from jsinfer import (
    ConfigError,
    EmitError,
    ExplainError,
    ExplanationLoadError,
    InferenceError,
    InferenceResult,
    NoDocumentsError,
    RunConfiguration,
    SchemaLoadError,
    emit_schema,
    explain,
    infer,
    infer_files,
    load_explanation,
    load_schema,
)
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaModel
from jsinfer.io.emit import PROVENANCE_KEYWORD, SchemaEmitter
from jsinfer.io.ingest import VALUES_SOURCE, documents_to_jsonl
from jsinfer.pipeline import infer as pipeline
from jsinfer.pipeline.infer import ANALYZER_STAGE, EMITTER_STAGE, INGESTOR_STAGE, infer_documents, infer_paths
from jsinfer.structure.analyzer import StructureAnalyzer

ONE_WORKER = RunConfiguration.resolve({"workers": 1})
INLINE = RunConfiguration.resolve({"workers": 1, "output.inline_provenance": True})
EMITTER = SchemaEmitter()

# Records with an optional field, nested arrays of records, a Map, and a Tuple.
ORDERS: list[Any] = [
    {
        "id": i,
        "status": ["new", "paid", "sent"][i % 3],
        "lines": [{"sku": f"s{j}", "qty": j % 3 + 1} for j in range(i % 4)],
        "scores": {f"user{3 * i + j}": j for j in range(i % 3)},
        "point": [i, f"p{i}"],
        **({"note": f"note {i}"} if i % 5 == 0 else {}),
    }
    for i in range(18)
]
MIXED: list[Any] = [*ORDERS[:5], [1, "a"], 2.5, "text\u2028\U0001f600", None, True, 2**70, -0.0]
# Documents a .jsonl file cannot carry as RFC 8259 JSON that jsinfer accepts (1.11), between valid ones.
WITH_SKIPS: list[Any] = [
    *ORDERS[:6],
    {"id": float("nan")},
    float("inf"),
    {"name": "\ud800"},
    [float("-inf")],
    *ORDERS[6:12],
]
COLLECTIONS = {"orders": ORDERS, "mixed": MIXED, "with skips": WITH_SKIPS}


def comparable(report: InferenceReport) -> dict[str, Any]:
    """The reproducible part of a report, with the ingestion source names left out (they name the input)."""
    data = report.comparable_json()
    for error in data["ingestion"]["errors"]:  # type: ignore[index, union-attr]
        error["source"] = "<source>"
    return data


def emitted_pointers(result: InferenceResult) -> list[str]:
    """The Constraint pointers in document order (ConstraintIds hold nids, which differ between runs)."""
    return list(result.pointers.values())


def same_outputs(first: InferenceResult, second: InferenceResult) -> None:
    assert first.schema == second.schema
    assert first.schema_bytes == second.schema_bytes
    assert first.document == second.document and emitted_pointers(first) == emitted_pointers(second)
    assert first.provenance == second.provenance
    assert first.explanation_bytes() == second.explanation_bytes()
    assert first.config_bytes() == second.config_bytes()
    assert comparable(first.report) == comparable(second.report)


def write_jsonl(path: Path, documents: list[Any], **dumps: Any) -> Path:
    """A .jsonl file holding ``documents`` one per line, written by Python's json module."""
    path.write_text("".join(json.dumps(value, **dumps) + "\n" for value in documents), encoding="ascii")
    return path


# ---------------------------------------------------------------------------
# Exports
# ---------------------------------------------------------------------------


def test_the_package_exports_the_api_and_the_error_types() -> None:
    for name in jsinfer.__all__:
        assert hasattr(jsinfer, name), name
    from jsinfer.config.errors import ConfigError as config_error
    from jsinfer.core.errors import InferenceError as inference_error
    from jsinfer.core.errors import NoDocumentsError as no_documents_error
    from jsinfer.core.provenance import ExplainError as explain_error
    from jsinfer.io.emit import EmitError as emit_error
    from jsinfer.io.load import ExplanationLoadError as explanation_load_error
    from jsinfer.io.load import SchemaLoadError as schema_load_error

    assert (ConfigError, InferenceError, NoDocumentsError, ExplainError) == (
        config_error, inference_error, no_documents_error, explain_error
    )
    assert (EmitError, ExplanationLoadError, SchemaLoadError) == (emit_error, explanation_load_error, schema_load_error)
    assert issubclass(NoDocumentsError, InferenceError) and not issubclass(ConfigError, InferenceError)
    assert isinstance(jsinfer.__version__, str) and jsinfer.__version__


# ---------------------------------------------------------------------------
# infer and infer_files
# ---------------------------------------------------------------------------


def test_infer_returns_the_pipeline_outputs() -> None:
    result = infer(ORDERS, ONE_WORKER)
    run = infer_documents(ORDERS, ONE_WORKER)
    assert isinstance(result.schema, SchemaModel) and result.schema == run.model
    assert result.schema_bytes == run.schema == EMITTER.dumps(result.document)
    assert result.document == run.document and emitted_pointers(result) == list(run.pointers.values())
    assert result.provenance == run.explanation == EMITTER.explanation_file(result.schema, result.pointers)
    assert result.explanation_bytes() == EMITTER.explanation(result.schema, result.pointers)
    assert comparable(result.report) == comparable(run.report)
    assert result.config == ONE_WORKER and result.report.ok
    # 27.9: the Schema_Model, its Provenance_Records (by ConstraintId and by pointer), and the report.
    assert set(result.schema.provenance) == set(result.pointers)
    assert {result.pointers[cid]: record for cid, record in result.schema.provenance.items()} == dict(
        result.provenance.records
    )
    assert (result.versions, result.combined, result.diffs) == ((), None, ())
    assert "InferenceResult" in repr(result) and "status ok" in repr(result)


def test_infer_defaults_to_every_default_setting() -> None:
    result = infer(iter([{"a": 1}, {"a": 2}]))
    assert result.config == RunConfiguration.default() and result.report.ok


def test_infer_files_returns_the_pipeline_outputs(tmp_path: Path) -> None:
    folder = tmp_path / "orders"
    folder.mkdir()
    for index, document in enumerate(ORDERS):
        (folder / f"{index:02d}.json").write_text(json.dumps(document), encoding="utf-8")
    extra = write_jsonl(tmp_path / "mixed.jsonl", MIXED)
    result = infer_files([folder, str(extra)], ONE_WORKER)
    run = infer_paths([folder, extra], ONE_WORKER)
    assert result.schema == run.model and result.schema_bytes == run.schema
    assert result.provenance == run.explanation and comparable(result.report) == comparable(run.report)
    # The same documents in memory, in the same order, give the same outputs (27.9).
    same_outputs(result, infer([*ORDERS, *MIXED], ONE_WORKER))


@pytest.mark.parametrize("name", list(COLLECTIONS))
def test_in_memory_documents_equal_the_same_documents_in_a_jsonl_file(tmp_path: Path, name: str) -> None:
    documents = COLLECTIONS[name]
    result = infer(documents, ONE_WORKER)
    # Written by json.dumps with its defaults: the same documents, so the same model and output bytes.
    loose = infer_files([write_jsonl(tmp_path / "loose.jsonl", documents)], ONE_WORKER)
    assert loose.schema == result.schema and loose.schema_bytes == result.schema_bytes
    assert loose.explanation_bytes() == result.explanation_bytes()
    assert [e.line for e in loose.report.ingest_errors] == [e.line for e in result.report.ingest_errors]
    # Written compactly, as infer writes its lines: the reasons (with their columns) match too.
    compact = infer_files([write_jsonl(tmp_path / "compact.jsonl", documents, separators=(",", ":"))], ONE_WORKER)
    same_outputs(result, compact)
    assert {error.source for error in result.report.ingest_errors} <= {VALUES_SOURCE}


def test_documents_that_a_file_could_not_carry_are_skipped_and_recorded() -> None:
    result = infer(WITH_SKIPS, ONE_WORKER)
    errors = result.report.ingest_errors
    assert [(error.source, error.line) for error in errors] == [(VALUES_SOURCE, line) for line in (7, 8, 9, 10)]
    assert result.report.skipped == 4
    assert [error.reason.split(" ")[0] for error in errors] == ["NaN", "Infinity", "string", "-Infinity"]
    assert "surrogate" in errors[2].reason
    # The skipped documents take no part in the model.
    assert result.schema == infer([*ORDERS[:6], *ORDERS[6:12]], ONE_WORKER).schema


def test_subclasses_of_the_json_types_are_their_base_values() -> None:
    class Level(enum.IntEnum):
        LOW = 1

    class Text(str):
        pass

    documents = [OrderedDict([("b", Level.LOW), ("a", Text("x"))]), {"b": 2, "a": "y"}]
    plain = [{"b": 1, "a": "x"}, {"b": 2, "a": "y"}]
    same_outputs(infer(documents, ONE_WORKER), infer(plain, ONE_WORKER))


# ---------------------------------------------------------------------------
# Non-JSON values are rejected before any stage runs
# ---------------------------------------------------------------------------

CIRCULAR: list[Any] = []
CIRCULAR.append({"self": CIRCULAR})


@pytest.mark.parametrize(
    ("value", "error", "message"),
    [
        ((1, 2), TypeError, "tuple at (root) is not a JSON type"),
        ({"a": [1, (2, 3)]}, TypeError, "tuple at /a/1"),
        ({"tags": {"x", "y"}}, TypeError, "set at /tags"),
        ({"blob": b"\x00"}, TypeError, "bytes at /blob"),
        ({"price": Decimal("1.5")}, TypeError, "Decimal at /price"),
        ({"when": object()}, TypeError, "object at /when"),
        ({1: "a", "1": "b"}, TypeError, "has a key of type int"),  # json.dumps would write two "1" members
        ({"a": {None: 1}}, TypeError, "the object at /a has a key of type NoneType"),
        ({"a": {True: 1}}, TypeError, "key of type bool"),
        ({"a": {1.5: 1}}, TypeError, "key of type float"),
        (CIRCULAR, ValueError, "the array at /0/self contains itself"),
    ],
    ids=repr,
)
def test_non_json_values_raise_before_any_stage(
    monkeypatch: pytest.MonkeyPatch, value: Any, error: type[Exception], message: str
) -> None:
    def no_run(*args: Any, **kwargs: Any) -> Any:
        raise AssertionError("the pipeline must not start")

    monkeypatch.setattr(pipeline, "run_pipeline", no_run)
    consumed: list[int] = []

    def documents() -> Any:
        for index, document in enumerate([{"ok": 1}, value, {"ok": 2}]):
            consumed.append(index)
            yield document

    with pytest.raises(error) as caught:
        infer(documents(), ONE_WORKER)
    assert not isinstance(caught.value, InferenceError)
    assert "document 1 of the iterable" in str(caught.value) and message in str(caught.value)
    assert consumed == [0, 1]  # the error names the first offending document, as soon as it is read


@pytest.mark.parametrize("documents", [b'{"a": 1}\n', bytearray(b"1"), "text", {"a": 1}, 5, None], ids=repr)
def test_documents_must_be_an_iterable_of_values(documents: Any) -> None:
    with pytest.raises(TypeError, match="iterable of JSON values"):
        infer(documents, ONE_WORKER)


def test_arguments_are_type_checked() -> None:
    with pytest.raises(TypeError, match="RunConfiguration"):
        infer([1], {"workers": 1})  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="RunConfiguration"):
        infer_files(["a.json"], {"workers": 1})  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="single path"):
        infer_files("a.json", ONE_WORKER)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        load_schema(object())  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        emit_schema({"type": "string"})  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        emit_schema(load_schema(True), inline_provenance=1)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Inline provenance and the explanation file
# ---------------------------------------------------------------------------


def schema_objects(document: Any, pointer: str = "") -> list[tuple[str, dict[str, Any]]]:
    """Every schema object of an emitted document with its pointer (applicator keywords only)."""
    if not isinstance(document, dict):
        return []
    found = [(pointer, document)]
    for keyword in ("properties", "patternProperties", "$defs"):
        for name, child in document.get(keyword, {}).items():
            escaped = name.replace("~", "~0").replace("/", "~1")
            found += schema_objects(child, f"{pointer}/{keyword}/{escaped}")
    for keyword in ("prefixItems", "anyOf", "oneOf", "allOf"):
        for index, child in enumerate(document.get(keyword, [])):
            found += schema_objects(child, f"{pointer}/{keyword}/{index}")
    for keyword in ("items", "additionalProperties", "propertyNames", "not", "if", "then", "else", "contains"):
        if keyword in document:
            found += schema_objects(document[keyword], f"{pointer}/{keyword}")
    return found


def test_inline_provenance_follows_the_configuration() -> None:
    off = infer(ORDERS, ONE_WORKER)
    on = infer(ORDERS, INLINE)
    assert off.schema_bytes == emit_schema(off.schema) and PROVENANCE_KEYWORD.encode() not in off.schema_bytes
    assert on.schema_bytes == emit_schema(on.schema, inline_provenance=True) != off.schema_bytes
    assert json.loads(on.schema_bytes) == on.document
    # Only the schema bytes differ: the model, the pointer map, and the explanation file are the same.
    assert on.schema == off.schema and emitted_pointers(on) == emitted_pointers(off)
    assert on.explanation_bytes() == off.explanation_bytes()
    assert load_schema(on.schema_bytes) == load_schema(off.schema_bytes) == off.schema
    # Each inline record is its explanation entry (26.6), and every entry appears inline once.
    records = {pointer: record.to_json() for pointer, record in on.provenance.records.items()}
    inline: dict[str, Any] = {}
    for base, node in schema_objects(on.document):
        for relative, record in node.get(PROVENANCE_KEYWORD, {}).items():
            inline[base + relative] = record
    assert inline == records


def test_the_explanation_file_round_trips_and_explains_every_constraint() -> None:
    result = infer(MIXED, ONE_WORKER)
    data = result.explanation_bytes()
    loaded = load_explanation(data)
    assert loaded == result.provenance and loaded.dumps() == data
    assert load_explanation(data.decode("utf-8")) == load_explanation(json.loads(data)) == loaded
    for pointer, record in result.provenance.records.items():
        text = explain(result.schema_bytes, data, pointer)
        assert text.startswith(f"constraint: {pointer}\n") and f"technique: {record.technique}" in text


def test_files_hold_every_output() -> None:
    result = infer(ORDERS, INLINE)
    files = result.files()
    assert list(files) == ["schema.json", "explanation.json", "report.json", "effective-config.toml"]
    assert files["schema.json"] == result.schema_bytes
    assert files["explanation.json"] == result.explanation_bytes() == result.provenance.dumps()
    assert files["report.json"] == result.report_bytes() == result.report.to_bytes()
    assert json.loads(files["report.json"]) == result.report.to_json()
    assert RunConfiguration.from_toml(files["effective-config.toml"].decode("utf-8")) == result.config == INLINE
    assert all(data.endswith(b"\n") for data in files.values())
    # A second run gives the same files, apart from the report's exempt contents (27.5, 27.12).
    again = infer(ORDERS, INLINE).files()
    assert {name: data for name, data in again.items() if name != "report.json"} == {
        name: data for name, data in files.items() if name != "report.json"
    }
    assert InferenceReport.from_json(json.loads(again["report.json"])).comparable_json() == (
        result.report.comparable_json()
    )


# ---------------------------------------------------------------------------
# load_schema and emit_schema
# ---------------------------------------------------------------------------


def test_load_schema_and_emit_schema_round_trip() -> None:
    result = infer(MIXED, ONE_WORKER)
    for source in (result.schema_bytes, result.schema_bytes.decode("utf-8"), json.loads(result.schema_bytes)):
        model = load_schema(source)
        assert model == result.schema  # 2.7 equality
        assert emit_schema(model) == result.schema_bytes
    assert emit_schema(result.schema, inline_provenance=True) == infer(MIXED, INLINE).schema_bytes
    for boolean in (True, False):
        assert emit_schema(load_schema(boolean)) == (b"true\n" if boolean else b"false\n")


def test_load_schema_translates_older_drafts() -> None:
    draft07 = {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {"id": {"type": "integer"}},
        "type": "object",
        "properties": {"id": {"$ref": "#/definitions/id"}, "pair": {"items": [{"type": "string"}]}},
    }
    emitted = json.loads(emit_schema(load_schema(draft07)))
    assert emitted["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert emitted["$defs"] == {"id": {"type": "integer"}}
    assert emitted["properties"]["id"] == {"$ref": "#/$defs/id"}
    assert emitted["properties"]["pair"] == {"prefixItems": [{"type": "string"}]}


# ---------------------------------------------------------------------------
# Errors (27.10, 27.11, 27.13, 1.8, 1.10, 2.12, 26.12)
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("documents", [[], iter(()), [float("nan"), "\udc00"]], ids=["empty", "exhausted", "skipped"])
def test_no_documents_raise_no_documents_error(documents: Any) -> None:
    with pytest.raises(NoDocumentsError) as caught:
        infer(documents, ONE_WORKER)
    error = caught.value
    assert isinstance(error, InferenceError) and error.stage == INGESTOR_STAGE
    assert "no valid documents were found" in str(error)
    report = error.partial_report
    assert report is not None and not report.ok and report.status.stage == INGESTOR_STAGE
    assert report.ingest_errors == error.errors


def test_files_without_documents_raise_no_documents_error(tmp_path: Path) -> None:
    (tmp_path / "notes.txt").write_text("not a document", encoding="utf-8")
    (tmp_path / "blank.jsonl").write_text("\n \t\n", encoding="utf-8")
    with pytest.raises(NoDocumentsError) as caught:
        infer_files([tmp_path], ONE_WORKER)
    assert caught.value.partial_report is not None and caught.value.errors == ()


def test_a_stage_failure_raises_inference_error_and_the_process_continues(monkeypatch: pytest.MonkeyPatch) -> None:
    def fail(*args: Any, **kwargs: Any) -> Any:
        raise RuntimeError("injected failure")

    with monkeypatch.context() as patch:
        patch.setattr(StructureAnalyzer, "infer_model", fail)
        with pytest.raises(InferenceError) as caught:
            infer(ORDERS, ONE_WORKER)
    error = caught.value
    assert type(error) is InferenceError and error.stage == ANALYZER_STAGE
    assert isinstance(error.__cause__, RuntimeError) and "injected failure" in str(error)
    report = error.partial_report
    assert report is not None and report.status.stage == ANALYZER_STAGE and ANALYZER_STAGE not in report.timings
    assert infer(ORDERS, ONE_WORKER).report.ok  # 27.13: the calling process keeps running


def test_an_emission_error_is_a_stage_error(monkeypatch: pytest.MonkeyPatch) -> None:
    def fail(*args: Any, **kwargs: Any) -> Any:
        raise EmitError("/type", "injected")

    monkeypatch.setattr(SchemaEmitter, "explanation_file", fail)
    with pytest.raises(InferenceError) as caught:
        infer(ORDERS, ONE_WORKER)
    assert caught.value.stage == EMITTER_STAGE and isinstance(caught.value.__cause__, EmitError)


def test_configuration_and_path_errors_raise_config_error(tmp_path: Path) -> None:
    with pytest.raises(ConfigError, match="workers"):
        RunConfiguration.resolve({"workers": 0})
    missing_bundle = RunConfiguration.resolve({"workers": 1, "model_bundle": str(tmp_path / "no-bundle")})
    for run in (lambda: infer(ORDERS, missing_bundle), lambda: infer_files([tmp_path / "missing.jsonl"])):
        with pytest.raises(ConfigError) as caught:
            run()
        assert not isinstance(caught.value, InferenceError)
    (tmp_path / "data.txt").write_text("1", encoding="utf-8")
    with pytest.raises(ConfigError) as caught:
        infer_files([tmp_path / "data.txt", tmp_path / "gone.json"], ONE_WORKER)
    assert len(caught.value.problems) == 2  # every rejected path in one error (1.10)


def test_load_schema_errors() -> None:
    with pytest.raises(SchemaLoadError) as caught:
        load_schema(b'{"type":\n  oops}')
    assert (caught.value.check, caught.value.line, caught.value.col) == ("json", 2, 3)
    with pytest.raises(SchemaLoadError) as caught:
        load_schema("[1]")
    assert caught.value.check == "root"
    with pytest.raises(SchemaLoadError) as caught:
        load_schema({"type": 5})
    assert caught.value.check == "meta" and caught.value.errors


def test_emit_schema_errors() -> None:
    loaded = load_schema(b'{"type": "string", "minLength": 1}')  # a loaded model has no Provenance_Records
    assert emit_schema(loaded) == b'{\n  "$schema": "https://json-schema.org/draft/2020-12/schema",\n' \
        b'  "type": "string",\n  "minLength": 1\n}\n'
    with pytest.raises(EmitError, match="has no Provenance_Record"):
        emit_schema(loaded, inline_provenance=True)
    assert emit_schema(load_schema({}), inline_provenance=True) == emit_schema(load_schema({}))  # no Constraints


def test_explanation_and_explain_errors() -> None:
    result = infer(ORDERS, ONE_WORKER)
    with pytest.raises(ExplanationLoadError) as caught:
        load_explanation(b"{}")
    assert caught.value.check == "format"
    with pytest.raises(ExplanationLoadError) as caught:
        load_explanation(b"{not json")
    assert caught.value.check == "json"
    data = result.explanation_bytes()
    for pointer, check in (("type", "syntax"), ("/nowhere", "resolve"), ("", "resolve")):
        with pytest.raises(ExplainError) as explained:
            explain(result.schema_bytes, data, pointer)
        assert (explained.value.input, explained.value.check) == ("pointer", check)
    with pytest.raises(ExplainError) as explained:
        explain(b"{oops", data, "/type")
    assert (explained.value.input, explained.value.check) == ("schema", "parse")
    with pytest.raises(ExplainError) as explained:
        explain(result.schema_bytes, b'{"format": 1, "records": {}}', next(iter(result.provenance.records)))
    assert (explained.value.input, explained.value.check) == ("pointer", "entry")


def test_the_jsonl_lines_of_infer_are_the_ones_documents_to_jsonl_writes(tmp_path: Path) -> None:
    path = tmp_path / "lines.jsonl"
    path.write_bytes(documents_to_jsonl(MIXED))
    same_outputs(infer(MIXED, ONE_WORKER), infer_files([path], ONE_WORKER))
