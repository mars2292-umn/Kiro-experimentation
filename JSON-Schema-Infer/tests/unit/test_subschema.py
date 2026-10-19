"""Unit tests for io/subschema.py, the Subschema_Checker (task 19.2; Requirements 28.5, 31.7, 31.8).

The keyword scan, the draft-04 shim, and the outcome mapping are tested with
a recording stub runner; the last section runs the real jsonsubschema
(pinned in ``uv.lock``) in child processes through :class:`SubprocessRunner`,
plus fake interpreters for the timeout, crash, and unavailable paths.
"""

from __future__ import annotations

import copy
import math
import stat
import sys
import time
from pathlib import Path
from typing import Any

import pytest

from jsinfer.io.emit import SCHEMA_DIALECT, SchemaEmitter
from jsinfer.io.subschema import (
    BELOW_ROOT,
    EMBEDDED_RESOURCE,
    NON_SCALAR_ITEMS,
    NOT_LOCAL,
    NOT_SUPPORTED,
    OTHER_DIALECT,
    RECURSIVE,
    UNRESOLVED,
    CheckFailure,
    KeywordUse,
    Relation,
    SubprocessRunner,
    SubschemaChecker,
    Unsupported,
    to_draft4,
    unsupported_keywords,
)
from jsinfer.pipeline.naive_merge import naive_merge


class Recorder:
    """A stub runner: returns scripted outcomes in order and records each call."""

    def __init__(self, *outcomes: bool | CheckFailure) -> None:
        self.outcomes = list(outcomes)
        self.calls: list[tuple[Any, Any, float]] = []

    def __call__(self, lhs: Any, rhs: Any, timeout_s: float) -> bool | CheckFailure:
        self.calls.append((lhs, rhs, timeout_s))
        return self.outcomes.pop(0)


def uses(schema: Any) -> list[tuple[str, str, str]]:
    return [(use.pointer, use.keyword, use.reason) for use in unsupported_keywords(schema)]


# ---------------------------------------------------------------------------
# Keyword scan


@pytest.mark.parametrize(
    ("schema", "keywords"),
    [
        ({"type": "string", "format": "email"}, ("format",)),
        ({"contains": {"type": "integer"}}, ("contains",)),
        ({"propertyNames": {"maxLength": 3}}, ("propertyNames",)),
        ({"if": {"required": ["a"]}, "then": {"required": ["b"]}, "else": {}}, ("else", "if", "then")),
        ({"dependentRequired": {"a": ["b"]}}, ("dependentRequired",)),
        ({"dependentSchemas": {"a": {"required": ["b"]}}}, ("dependentSchemas",)),
        ({"unevaluatedProperties": False}, ("unevaluatedProperties",)),
        ({"definitions": {"a": {}}}, ("definitions",)),
        ({"properties": {"a": {"contentEncoding": "base64"}}}, ("contentEncoding",)),
        ({"madeUpKeyword": 1}, ("madeUpKeyword",)),
    ],
)
def test_keywords_outside_the_supported_list_are_unsupported(schema: dict[str, Any], keywords: tuple[str, ...]) -> None:
    runner = Recorder()
    result = SubschemaChecker(runner).relation(schema, {})
    assert isinstance(result, Unsupported)
    assert result.cause == "keywords"
    assert result.keywords == keywords
    assert all(use.reason == NOT_SUPPORTED for use in result.inferred)
    assert result.truth == ()
    assert runner.calls == []


def test_unsupported_keywords_are_listed_per_side_with_pointers() -> None:
    inferred = {"properties": {"a": {"type": "string", "format": "date"}}}
    truth = {"contains": {"format": "email"}, "x-note": {"format": "ignored"}}
    result = SubschemaChecker(Recorder()).relation(inferred, truth)
    assert isinstance(result, Unsupported)
    assert result.keywords == ("contains", "format")
    assert result.inferred == (KeywordUse("/properties/a", "format", NOT_SUPPORTED),)
    assert result.truth == (KeywordUse("", "contains", NOT_SUPPORTED), KeywordUse("/contains", "format", NOT_SUPPORTED))
    assert "format at /properties/a" in result.message


def test_annotations_and_x_keywords_are_not_findings() -> None:
    schema = {
        "$schema": "http://json-schema.org/draft/2020-12/schema#",
        "$id": "https://example.com/s.json",
        "$comment": "c",
        "title": "t",
        "description": "d",
        "default": 1,
        "examples": [1],
        "deprecated": False,
        "readOnly": True,
        "writeOnly": False,
        "x-provenance": {"format": "x"},
        "x-latent-type": "date",
        "properties": {"a": {"$anchor": "A", "title": "a", "x-key": True}},
    }
    assert uses(schema) == []


def test_dialect_id_and_schema_rules() -> None:
    assert uses({"$schema": "http://json-schema.org/draft-07/schema#"}) == [("", "$schema", OTHER_DIALECT)]
    assert uses({"$schema": 7}) == [("", "$schema", OTHER_DIALECT)]
    assert uses({"items": {"$schema": SCHEMA_DIALECT}}) == [("/items", "$schema", BELOW_ROOT)]
    assert uses({"$defs": {"a": {"$id": "a.json"}}}) == [("/$defs/a", "$id", EMBEDDED_RESOURCE)]


@pytest.mark.parametrize(
    ("schema", "found"),
    [
        ({"properties": {"c": {"$ref": "#"}}}, [("/properties/c", "$ref", RECURSIVE)]),
        (
            {"$defs": {"a": {"items": {"$ref": "#/$defs/b"}}, "b": {"anyOf": [{"$ref": "#/$defs/a"}]}}},
            [("/$defs/a/items", "$ref", RECURSIVE), ("/$defs/b/anyOf/0", "$ref", RECURSIVE)],
        ),
        # A reference that only leads into a cycle never ends either.
        (
            {"$ref": "#/$defs/t", "$defs": {"t": {"properties": {"c": {"$ref": "#/$defs/t"}}}}},
            [("", "$ref", RECURSIVE), ("/$defs/t/properties/c", "$ref", RECURSIVE)],
        ),
        ({"$ref": "other.json#/a"}, [("", "$ref", NOT_LOCAL)]),
        ({"$ref": "#A", "$defs": {"a": {"$anchor": "A"}}}, [("", "$ref", NOT_LOCAL)]),
        ({"$ref": "#/$defs/missing"}, [("", "$ref", UNRESOLVED)]),
        ({"$ref": "#/$defs", "$defs": {"a": {}}}, [("", "$ref", UNRESOLVED)]),
        ({"$ref": "#/x-defs/a", "x-defs": {"a": {}}}, [("", "$ref", UNRESOLVED)]),
    ],
)
def test_reference_rules(schema: dict[str, Any], found: list[tuple[str, str, str]]) -> None:
    assert uses(schema) == found


def test_shared_non_recursive_references_are_supported() -> None:
    schema = {
        "$defs": {"leaf": {"type": "string"}, "pair": {"prefixItems": [{"$ref": "#/$defs/leaf"}, {"$ref": "#/$defs/leaf"}]}},
        "properties": {"a": {"$ref": "#/$defs/pair"}, "b": {"$ref": "#/$defs/pair/prefixItems/1"}},
    }
    assert uses(schema) == []


@pytest.mark.parametrize(
    "schema",
    [
        {"type": "array", "uniqueItems": True, "items": {"type": "integer"}},
        {"uniqueItems": True, "items": {"type": ["string", "null"]}},
        {"uniqueItems": True, "items": {"enum": [1, "a", None]}},
        {"uniqueItems": True, "prefixItems": [{"const": 1}], "items": False},
        {"uniqueItems": True, "items": {"anyOf": [{"type": "string"}, {"type": "boolean"}]}},
        {"uniqueItems": True, "items": {"allOf": [{}, {"type": "number"}]}},
        {"uniqueItems": True, "items": {"$ref": "#/$defs/s"}, "$defs": {"s": {"type": "string"}}},
        {"type": "string", "uniqueItems": True},  # admits no array: no effect
        {"uniqueItems": False},
    ],
)
def test_unique_items_over_scalars_is_supported(schema: dict[str, Any]) -> None:
    assert uses(schema) == []


@pytest.mark.parametrize(
    "schema",
    [
        {"type": "array", "uniqueItems": True},
        {"uniqueItems": True, "items": {}},
        {"uniqueItems": True, "items": {"type": "object"}},
        {"uniqueItems": True, "items": {"type": ["string", "array"]}},
        {"uniqueItems": True, "items": {"anyOf": [{"type": "string"}, {"type": "object"}]}},
        {"uniqueItems": True, "items": {"enum": [1, [2]]}},
        {"uniqueItems": True, "prefixItems": [{"type": "integer"}]},  # anything may follow the prefix
        {"uniqueItems": True, "items": {"$ref": "#/$defs/o"}, "$defs": {"o": {"type": "object"}}},
    ],
)
def test_unique_items_over_possible_containers_is_unsupported(schema: dict[str, Any]) -> None:
    assert ("", "uniqueItems", NON_SCALAR_ITEMS) in uses(schema)  # an enum of arrays is reported on its own too


# ---------------------------------------------------------------------------
# The draft-04 shim


def test_shim_strips_annotations_x_keywords_and_root_identifiers() -> None:
    schema = {
        "$schema": SCHEMA_DIALECT,
        "$id": "https://example.com/s.json",
        "title": "t",
        "x-provenance": {},
        "type": "object",
        "properties": {"a": {"description": "d", "$anchor": "A", "x-key": True, "type": "string"}},
    }
    assert to_draft4(schema) == {"type": "object", "properties": {"a": {"type": "string"}}}


def test_shim_renames_defs_and_rewrites_references() -> None:
    schema = {
        "$defs": {"a b": {"type": "string"}, "c/d": {"type": "integer"}},
        "properties": {"x": {"$ref": "#/$defs/a%20b"}, "y": {"$ref": "#/$defs/c~1d", "minimum": 1}},
    }
    assert to_draft4(schema) == {
        "definitions": {"a b": {"type": "string"}, "c/d": {"type": "integer"}},
        "properties": {
            "x": {"$ref": "#/definitions/a%20b"},
            # 2020-12 applies the siblings of $ref, so the reference becomes one more conjunct.
            "y": {"minimum": 1, "allOf": [{"$ref": "#/definitions/c~1d"}]},
        },
    }


def test_shim_translates_tuples_and_follows_moved_targets() -> None:
    schema = {
        "prefixItems": [{"type": "integer"}, True],
        "items": False,
        "properties": {"first": {"$ref": "#/prefixItems/0"}, "list": {"items": False}},
        "additionalProperties": False,
    }
    assert to_draft4(schema) == {
        "items": [{"type": "integer"}, {}],
        "additionalItems": False,
        "properties": {"first": {"$ref": "#/items/0"}, "list": {"items": {"not": {}}}},
        "additionalProperties": False,
    }
    assert to_draft4({"items": {"type": "string"}}) == {"items": {"type": "string"}}
    assert to_draft4(True) == {}
    assert to_draft4(False) == {"not": {}}
    assert to_draft4({"not": True, "anyOf": [False]}) == {"not": {}, "anyOf": [{"not": {}}]}


def test_shim_translates_const_and_enum() -> None:
    assert to_draft4({"const": "a"}) == {"enum": ["a"]}
    assert to_draft4({"enum": [1, 1.0, True, "a", 2.5]}) == {"enum": [True, 1, 2.5, "a"]}
    assert to_draft4({"enum": ["a", "b"], "const": "a", "allOf": [{"type": "string"}]}) == {
        "enum": ["a", "b"],
        "allOf": [{"type": "string"}, {"enum": ["a"]}],
    }


@pytest.mark.parametrize(
    ("schema", "expected"),
    [
        ({"exclusiveMinimum": 0}, {"minimum": 0, "exclusiveMinimum": True}),
        ({"minimum": 1, "exclusiveMinimum": 0}, {"minimum": 1}),
        ({"minimum": 0, "exclusiveMinimum": 0}, {"minimum": 0, "exclusiveMinimum": True}),
        ({"minimum": 0, "exclusiveMinimum": 0.5}, {"minimum": 0.5, "exclusiveMinimum": True}),
        ({"exclusiveMaximum": 10.0}, {"maximum": 10, "exclusiveMaximum": True}),
        ({"maximum": 3, "exclusiveMaximum": 5}, {"maximum": 3}),
        ({"maximum": 5, "exclusiveMaximum": 5}, {"maximum": 5, "exclusiveMaximum": True}),
        ({"minLength": 2.0, "maxItems": 3.0, "multipleOf": 2.0}, {"minLength": 2, "maxItems": 3, "multipleOf": 2}),
    ],
)
def test_shim_writes_draft4_bounds(schema: dict[str, Any], expected: dict[str, Any]) -> None:
    translated = to_draft4(schema)
    assert translated == expected
    assert all(type(value) is type(expected[key]) for key, value in translated.items())


def test_shim_does_not_change_its_input() -> None:
    schema = {"$defs": {"a": {"const": 1.0}}, "$ref": "#/$defs/a", "title": "t", "prefixItems": [True]}
    before = copy.deepcopy(schema)
    to_draft4(schema)
    assert schema == before


@pytest.mark.parametrize(
    "schema",
    [{"format": "email"}, {"$ref": "https://example.com/a.json"}, {"$ref": "#/$defs/missing"}, {"items": [{}]}],
)
def test_shim_rejects_what_the_scan_rules_out(schema: dict[str, Any]) -> None:
    with pytest.raises(ValueError):
        to_draft4(schema)


# ---------------------------------------------------------------------------
# Outcome mapping (stub runner)


@pytest.mark.parametrize(
    ("included", "includes", "relation"),
    [
        (True, True, Relation.EQUIVALENT),
        (True, False, Relation.SUBSET),
        (False, True, Relation.SUPERSET),
        (False, False, Relation.INCOMPARABLE),
    ],
)
def test_inclusion_outcomes_map_to_relations(included: bool, includes: bool, relation: Relation) -> None:
    runner = Recorder(included, includes)
    inferred = {"type": "integer", "title": "inferred"}
    truth = {"type": "number"}
    assert SubschemaChecker(runner).relation(inferred, truth) is relation
    assert runner.calls == [({"type": "integer"}, {"type": "number"}, 30.0), ({"type": "number"}, {"type": "integer"}, 30.0)]


@pytest.mark.parametrize("kind", ["error", "timeout", "unavailable"])
def test_a_failed_first_check_is_unsupported_and_skips_the_second(kind: str) -> None:
    runner = Recorder(CheckFailure(kind, "went wrong"))  # type: ignore[arg-type]
    result = SubschemaChecker(runner).relation({}, {})
    assert result == Unsupported(kind, "checking that the inferred schema is a subschema of the truth schema: went wrong")  # type: ignore[arg-type]
    assert len(runner.calls) == 1


def test_a_failed_second_check_is_unsupported() -> None:
    runner = Recorder(True, CheckFailure("timeout", "too slow"))
    result = SubschemaChecker(runner).relation({}, {})
    assert isinstance(result, Unsupported)
    assert (result.cause, result.keywords) == ("timeout", ())
    assert result.message == "checking that the truth schema is a subschema of the inferred schema: too slow"


def test_timeout_comes_from_the_resource_parameter() -> None:
    runner = Recorder(True, True)
    SubschemaChecker(runner).relation({}, {}, timeout_s=5)
    assert [call[2] for call in runner.calls] == [5.0, 5.0]
    for bad in (0, -1.0, math.nan, math.inf, True, "30"):
        with pytest.raises(ValueError, match=r"subschema\.timeout_s"):
            SubschemaChecker(Recorder()).relation({}, {}, timeout_s=bad)  # type: ignore[arg-type]


def test_schemas_invalid_against_the_meta_schema_are_errors() -> None:
    runner = Recorder()
    result = SubschemaChecker(runner).relation({"type": "object"}, {"minLength": -1})
    assert isinstance(result, Unsupported)
    assert result.cause == "error"
    assert result.message.startswith("the truth schema is not a valid draft 2020-12 schema: /minLength")
    assert runner.calls == []


def test_bad_arguments_raise() -> None:
    with pytest.raises(TypeError):
        SubschemaChecker(Recorder()).relation([], {})  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="bool or a CheckFailure"):
        SubschemaChecker(Recorder(None)).relation({}, {})  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        CheckFailure("crash", "x")  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        Unsupported("error", "x", keywords=("format",))


# ---------------------------------------------------------------------------
# jsonsubschema in child processes

CHECKER = SubschemaChecker()
EMITTER = SchemaEmitter()


@pytest.mark.parametrize(
    ("inferred", "truth", "relation"),
    [
        ({"type": "integer"}, {"type": ["integer", "string"]}, Relation.SUBSET),
        ({"type": ["integer", "string"]}, {"type": "integer"}, Relation.SUPERSET),
        ({"type": ["string", "null"]}, {"anyOf": [{"type": "null"}, {"type": "string"}]}, Relation.EQUIVALENT),
        ({"type": "integer"}, {"type": "string"}, Relation.INCOMPARABLE),
        # jsonsubschema returns None for number ⊆ integer; the checker reads it as "does not hold".
        ({"type": "number"}, {"type": "integer"}, Relation.SUPERSET),
        (
            {"type": "array", "prefixItems": [{"type": "integer", "exclusiveMinimum": 0}], "items": False},
            {"type": "array", "items": {"type": "integer", "minimum": 0}},
            Relation.SUBSET,
        ),
        (
            {"$defs": {"s": {"type": "string"}}, "type": "object", "properties": {"a": {"$ref": "#/$defs/s", "minLength": 1}}},
            {"type": "object", "properties": {"a": {"type": "string"}}},
            Relation.SUBSET,
        ),
        ({"const": "a"}, {"enum": ["a", "b"]}, Relation.SUBSET),
        (True, {"not": False}, Relation.EQUIVALENT),
    ],
)
def test_relations_from_jsonsubschema(inferred: Any, truth: Any, relation: Relation) -> None:
    assert CHECKER.relation(inferred, truth) is relation


def test_naive_merge_schemas_end_to_end() -> None:
    """Emitted Naive_Merge documents (the all-Techniques-off comparison of 28.5) go through unchanged."""
    docs = [{"a": 1, "b": "x", "c": [1.5]}, {"a": 2, "c": []}]
    emitted, _ = EMITTER.layout(naive_merge(docs))
    assert isinstance(emitted, dict) and emitted["$schema"] == SCHEMA_DIALECT
    annotated = {**copy.deepcopy(emitted), "title": "truth", "x-provenance": {"/": "baseline"}}
    assert CHECKER.relation(emitted, annotated) is Relation.EQUIVALENT
    stricter = copy.deepcopy(emitted)
    stricter["required"] = sorted({*stricter.get("required", []), "b"})
    assert CHECKER.relation(emitted, stricter) is Relation.SUPERSET


def test_jsonsubschema_errors_reach_the_runner_as_failures() -> None:
    # The keyword scan now refuses negated object schemas before any check (task 25.6); the runner itself
    # still turns jsonsubschema's exception into a failure with cause "error".
    outcome = SubprocessRunner()({"type": "object", "not": {"type": "object", "required": ["a"]}}, {"type": "object"}, 30.0)
    assert isinstance(outcome, CheckFailure) and outcome.kind == "error"
    assert "UnsupportedNegatedObject" in outcome.message


def _fake_python(directory: Path, body: str) -> Path:
    """An executable shell script that stands in for the check interpreter."""
    path = directory / "python"
    path.write_text(f"#!/bin/sh\n{body}\n", encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def test_a_check_past_the_time_limit_is_killed(tmp_path: Path) -> None:
    runner = SubprocessRunner(_fake_python(tmp_path, "exec sleep 60"))
    started = time.monotonic()
    result = SubschemaChecker(runner).relation({}, {}, timeout_s=0.5)
    assert time.monotonic() - started < 10
    assert isinstance(result, Unsupported)
    assert result.cause == "timeout"
    assert result.message.endswith("the check did not finish within 0.5 s")


def test_a_crashing_check_process_is_an_error(tmp_path: Path) -> None:
    runner = SubprocessRunner(_fake_python(tmp_path, "echo boom >&2\nexit 3"))
    assert runner({}, {}, 30.0) == CheckFailure("error", "the check process exited with status 3: boom")


def test_missing_interpreter_or_jsonsubschema_is_unavailable(tmp_path: Path) -> None:
    missing = SubprocessRunner(tmp_path / "no-such-python")({}, {}, 30.0)
    assert isinstance(missing, CheckFailure) and missing.kind == "unavailable"
    # -S skips site-packages, so this interpreter cannot import jsonsubschema.
    bare = SubprocessRunner(_fake_python(tmp_path, f'exec "{sys.executable}" -S "$@"'))({}, {}, 30.0)
    assert isinstance(bare, CheckFailure) and bare.kind == "unavailable"
    assert "cannot import jsonsubschema: ModuleNotFoundError" in bare.message


def test_default_runner_uses_this_interpreter() -> None:
    runner = SubschemaChecker().runner
    assert isinstance(runner, SubprocessRunner) and runner.python == sys.executable
    assert runner({"type": "integer"}, {"type": "number"}, 30.0) is True
