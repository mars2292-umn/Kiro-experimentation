"""jsonsubschema keyword coverage probe (task 25.6; Requirements 31.7, 31.8).

Each case runs a known inclusion pair through the real jsonsubschema 0.0.8
(pinned in ``uv.lock``) in a child process and records the verdict the
checker reports. The results of the probe, which fixed the supported-keyword
list in ``io/subschema.py``:

- Decided correctly, so supported: ``type``, ``enum``, ``const`` (scalar
  values), ``allOf``, ``anyOf``, ``oneOf``, ``not`` of a plain non-numeric
  type, the string, number, array, and object bounds,
  ``multipleOf`` (``multipleOf: 4 ⊆ multipleOf: 2`` holds, the converse does
  not), ``pattern``, ``items``, ``prefixItems``, ``properties``,
  ``patternProperties``, ``additionalProperties``, ``required``, ``$defs``
  with local ``$ref``, and ``uniqueItems`` over scalar items. Integer-valued
  enums behave as draft 2020-12 integers: ``{"enum": [1.0]} ⊆ integer`` holds
  and ``{"enum": [1.5]} ⊆ integer`` does not.
- Ignored by jsonsubschema, so unsupported: ``dependencies`` (hence
  ``dependentRequired``: ``{"type": "object"}`` is reported a subschema of a
  schema with a dependency, and the converse holds too) and ``format``
  (``string ⊆ string with format email`` is reported to hold).
- Decided wrongly, so unsupported: a negated numeric type. jsonsubschema
  reports ``{"not": {"type": "integer"}}`` and ``{"not": {"type":
  "number"}}`` equivalent in both directions, although ``1.5`` satisfies
  only the first (``NEGATED_NUMBER``). ``integer ⊆ not number`` is still
  reported false, so the fault is in comparing two negations.
- Raised inside jsonsubschema, so caught by the keyword scan as
  ``keywords`` (31.8): an ``enum`` or ``const`` of objects or arrays
  (``UnsupportedEnumCanonicalization``) and a ``not`` of an object or array
  schema with object or array keywords (``UnsupportedNegatedObject``,
  ``UnsupportedNegatedArray``). A negated bare type is fine.
"""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.io.subschema import (
    CONTAINER_VALUES,
    NEGATED_COMPOUND,
    NEGATED_NUMBER,
    Relation,
    SubprocessRunner,
    SubschemaChecker,
    Unsupported,
    to_draft4,
    unsupported_keywords,
)

CHECKER = SubschemaChecker()

OBJ = {"type": "object"}
DECIDED: list[tuple[str, dict[str, Any], dict[str, Any], Relation]] = [
    ("type", {"type": "integer"}, {"type": "number"}, Relation.SUBSET),
    ("enum", {"enum": ["a"]}, {"enum": ["a", "b"]}, Relation.SUBSET),
    ("const", {"const": "a"}, {"enum": ["a", "b"]}, Relation.SUBSET),
    ("enum integers", {"enum": [1, 2]}, {"type": "number"}, Relation.SUBSET),
    ("enum 1.0 is an integer", {"enum": [1.0]}, {"type": "integer"}, Relation.SUBSET),
    ("enum 1.5 is no integer", {"enum": [1.5]}, {"type": "integer"}, Relation.INCOMPARABLE),
    ("allOf", {"allOf": [{"type": "integer"}, {"minimum": 1}]}, {"type": "integer"}, Relation.SUBSET),
    ("anyOf", {"anyOf": [{"type": "integer"}, {"type": "string"}]}, {"type": ["integer", "string"]}, Relation.EQUIVALENT),
    ("oneOf", {"oneOf": [{"type": "integer"}, {"type": "string"}]}, {"type": ["integer", "string"]}, Relation.EQUIVALENT),
    ("not a type", {"type": "string"}, {"not": {"type": "object"}}, Relation.SUBSET),
    ("minimum", {"type": "number", "minimum": 2}, {"type": "number", "minimum": 1}, Relation.SUBSET),
    ("exclusiveMinimum", {"type": "number", "exclusiveMinimum": 0}, {"type": "number", "minimum": 0}, Relation.SUBSET),
    ("maximum", {"type": "number", "maximum": 1}, {"type": "number", "maximum": 2}, Relation.SUBSET),
    ("multipleOf", {"type": "integer", "multipleOf": 4}, {"type": "integer", "multipleOf": 2}, Relation.SUBSET),
    ("minLength", {"type": "string", "minLength": 2}, {"type": "string", "minLength": 1}, Relation.SUBSET),
    ("maxLength", {"type": "string", "maxLength": 1}, {"type": "string", "maxLength": 2}, Relation.SUBSET),
    ("pattern", {"type": "string", "pattern": "^a+$"}, {"type": "string", "pattern": "^[a-z]+$"}, Relation.SUBSET),
    ("items", {"type": "array", "items": {"type": "integer"}}, {"type": "array", "items": {"type": "number"}}, Relation.SUBSET),
    ("prefixItems", {"type": "array", "prefixItems": [{"type": "string"}], "items": False}, {"type": "array", "items": {"type": "string"}}, Relation.SUBSET),
    ("minItems", {"type": "array", "minItems": 2}, {"type": "array", "minItems": 1}, Relation.SUBSET),
    ("maxItems", {"type": "array", "maxItems": 1}, {"type": "array", "maxItems": 2}, Relation.SUBSET),
    ("uniqueItems", {"type": "array", "items": {"type": "integer"}, "uniqueItems": True}, {"type": "array", "items": {"type": "integer"}}, Relation.SUBSET),
    ("properties", {**OBJ, "properties": {"a": {"type": "integer"}}}, {**OBJ, "properties": {"a": {"type": "number"}}}, Relation.SUBSET),
    ("required", {**OBJ, "properties": {"a": {"type": "string"}}, "required": ["a"]}, {**OBJ, "properties": {"a": {"type": "string"}}}, Relation.SUBSET),
    ("additionalProperties", {**OBJ, "properties": {"a": {}}, "additionalProperties": False}, {**OBJ, "properties": {"a": {}}}, Relation.SUBSET),
    ("patternProperties", {**OBJ, "patternProperties": {"^x": {"type": "integer"}}}, OBJ, Relation.SUBSET),
    ("minProperties", {**OBJ, "minProperties": 2}, {**OBJ, "minProperties": 1}, Relation.SUBSET),
    ("maxProperties", {**OBJ, "maxProperties": 1}, {**OBJ, "maxProperties": 2}, Relation.SUBSET),
    ("$defs and $ref", {"$defs": {"s": {"type": "string"}}, "$ref": "#/$defs/s"}, {"type": "string"}, Relation.EQUIVALENT),
]


@pytest.mark.parametrize(("name", "inferred", "truth", "relation"), DECIDED, ids=[c[0] for c in DECIDED])
def test_supported_keywords_are_decided(name: str, inferred: dict[str, Any], truth: dict[str, Any], relation: Relation) -> None:
    assert unsupported_keywords(inferred) == [] and unsupported_keywords(truth) == []
    assert CHECKER.relation(inferred, truth) == relation


IGNORED: list[tuple[str, dict[str, Any], dict[str, Any]]] = [
    ("dependentRequired", {**OBJ}, {**OBJ, "dependentRequired": {"a": ["b"]}}),
    ("format", {"type": "string"}, {"type": "string", "format": "email"}),
]


@pytest.mark.parametrize(("name", "inferred", "truth"), IGNORED, ids=[c[0] for c in IGNORED])
def test_keywords_jsonsubschema_ignores_are_unsupported(name: str, inferred: dict[str, Any], truth: dict[str, Any]) -> None:
    result = CHECKER.relation(inferred, truth)
    assert isinstance(result, Unsupported) and result.cause == "keywords" and name in result.keywords


RAISING: list[tuple[str, dict[str, Any], str, str]] = [
    ("enum of objects", {"enum": [{"a": 1}]}, "enum", CONTAINER_VALUES),
    ("enum of arrays", {"enum": [[1]]}, "enum", CONTAINER_VALUES),
    ("const object", {"const": {"a": 1}}, "const", CONTAINER_VALUES),
    ("negated object schema", {"not": {**OBJ, "properties": {"a": {"type": "integer"}}, "required": ["a"]}}, "not", NEGATED_COMPOUND),
    ("negated array schema", {"not": {"type": "array", "items": {"type": "string"}}}, "not", NEGATED_COMPOUND),
]


@pytest.mark.parametrize(("name", "schema", "keyword", "reason"), RAISING, ids=[c[0] for c in RAISING])
def test_cases_that_raise_inside_jsonsubschema_are_caught_by_the_scan(name: str, schema: dict[str, Any], keyword: str, reason: str) -> None:
    uses = unsupported_keywords(schema)
    assert [(u.keyword, u.reason) for u in uses] == [(keyword, reason)]
    result = CHECKER.relation(schema, {})
    assert isinstance(result, Unsupported) and result.cause == "keywords"
    # Without the scan, the check itself fails inside jsonsubschema: the probe that fixed the rule.
    outcome = SubprocessRunner()(to_draft4(schema), to_draft4({}), 30.0)
    assert not isinstance(outcome, bool), f"{name}: jsonsubschema decided it after all: {outcome!r}"


def test_negated_numeric_types_are_decided_wrongly_and_so_unsupported() -> None:
    not_number, not_integer = {"not": {"type": "number"}}, {"not": {"type": "integer"}}
    # The probe: both directions are reported to hold, although 1.5 satisfies only "not integer".
    assert SubprocessRunner()(to_draft4(not_integer), to_draft4(not_number), 30.0) is True
    assert SubprocessRunner()(to_draft4(not_number), to_draft4(not_integer), 30.0) is True
    for schema in (not_number, not_integer, {"not": {"type": ["integer", "string"]}}):
        assert [(u.keyword, u.reason) for u in unsupported_keywords(schema)] == [("not", NEGATED_NUMBER)]
    assert unsupported_keywords({"not": {"type": "string"}}) == []
