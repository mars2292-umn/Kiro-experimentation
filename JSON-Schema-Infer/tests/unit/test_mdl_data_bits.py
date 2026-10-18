"""Unit tests for the data bits, exception costs, and local scoring of mdl/scorer.py (task 10.3).

Requirements 19.1, 19.2, 19.6. Expected values are built from the primitive
codes of ``mdl/codes.py`` following "Key Algorithms > MDL encoding" and the
scorer's module docstring. Sums of several terms are compared with a
relative tolerance of 1e-12, because the scorer multiplies each distinct
value's cost by its count before the single ``math.fsum``.
"""

from __future__ import annotations

import math
import random
from typing import Any

import pytest

from jsinfer.core.jsonvalue import JType
from jsinfer.core.observations import build_store
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.mdl.codes import CAT, LN0, LZ, STR, UNI, U
from jsinfer.mdl.scorer import (
    EXCEPTION_FLAG_BITS,
    FORMAT_ALPHABET_SIZES,
    FORMAT_COUNT,
    MDLScorer,
    Score,
    model_bits,
)
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.values.patterns import PLUS, Alternative, CharClass, Pattern, Quantifier, Token

SCORER = MDLScorer()
FLAG = EXCEPTION_FLAG_BITS
LOG10 = math.log2(10)
PHONE = Pattern(
    (
        Alternative(
            (Token(CharClass.DIGIT, Quantifier.exactly(3)), Token("-"), Token(CharClass.DIGIT, Quantifier.exactly(4)))
        ),
    )
)
WORD_OR_CODE = Pattern(
    (Alternative((Token(CharClass.LOWER, PLUS),)), Alternative((Token(CharClass.DIGIT, Quantifier(2, 4)),)))
)


def view_of(docs: list[Any], chunk_size: int = 2000) -> View:
    return View.root(build_store(docs, chunk_size))


def node(schema: Any) -> SchemaNode:
    return SchemaNode.from_json(schema)


def bits(schema: Any, docs: list[Any], **kwargs: Any) -> tuple[float, float, int]:
    """``(data, exceptions, n_rejected)`` of ``docs`` (root rows) under ``schema``."""
    return SCORER.data_bits(node(schema), view_of(docs), **kwargs)


def close(actual: float, *terms: float) -> bool:
    return math.isclose(actual, math.fsum(terms), rel_tol=1e-12, abs_tol=1e-12)


def accepted(schema: Any, docs: list[Any], *data_terms: float, flags: int = 1) -> None:
    """Every doc is accepted, and the data bits are ``flags`` exception flags plus ``data_terms``."""
    data, exceptions, rejected = bits(schema, docs)
    assert rejected == 0
    assert exceptions == flags * FLAG
    assert close(data, flags * FLAG, *data_terms), (data, math.fsum((flags * FLAG, *data_terms)))


# ---------------------------------------------------------------------------
# Exceptions (19.2)
# ---------------------------------------------------------------------------


def test_exception_cost_of_a_rejected_scalar() -> None:
    data, exceptions, rejected = bits({"type": "integer"}, [1, "a", 2])
    assert rejected == 1
    assert exceptions == math.fsum((FLAG, CAT((2, 1)), U("a")))
    assert close(data, exceptions, LZ(1), LZ(2))


def test_exception_cost_of_a_rejected_container_is_its_universal_code() -> None:
    value = {"k": [True, None, 2.5], "": "é"}
    _, exceptions, rejected = bits({"type": "integer"}, [1, value])
    assert rejected == 1
    assert close(exceptions, FLAG, CAT((1, 1)), U(value))


def test_every_rejected_observation_adds_a_positive_finite_cost() -> None:
    previous = bits({"type": "integer"}, [1, 2])[1]
    assert previous == FLAG
    docs: list[Any] = [1, 2]
    for extra in ("a", None, [1], {"x": "y"}, "a"):
        docs.append(extra)
        _, exceptions, rejected = bits({"type": "integer"}, docs)
        assert rejected == len(docs) - 2
        assert math.isfinite(exceptions) and exceptions > previous
        previous = exceptions


def test_false_and_true_schemas() -> None:
    data, exceptions, rejected = bits(False, [1, "a"])
    assert rejected == 2 and data == exceptions
    assert close(exceptions, FLAG, CAT((0, 2)), U(1), U("a"))
    assert bits(True, [1, "a"]) == bits({}, [1, "a"])


def test_descendants_of_rejected_rows_are_not_coded() -> None:
    schema = {"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a"]}
    _, exceptions, rejected = bits(schema, [{"a": "x"}, {"b": [1, 2, "deep"]}])
    assert rejected == 1  # the object, not its members
    assert close(exceptions, FLAG, CAT((1, 1)), U({"b": [1, 2, "deep"]}), FLAG)


# ---------------------------------------------------------------------------
# Type choice, enum, const
# ---------------------------------------------------------------------------


def test_type_choice_over_all_seven_types_when_type_is_absent() -> None:
    counts = (1, 1, 1, 0, 1, 0, 0)  # null, boolean, integer, number, string, array, object
    accepted({}, [None, True, 1, "ab"], CAT(counts), CAT((1, 0)), LZ(1), LN0(2), 16.0)


def test_type_choice_over_the_listed_types() -> None:
    schema = {"type": ["string", "integer", "null"]}
    accepted(schema, [1, "ab", None, None], CAT((1, 2, 1)), LZ(1), LN0(2), 16.0)
    assert bits(schema, [1, True])[2] == 1


def test_integral_numbers_under_number_use_the_decimal_code() -> None:
    # 1 is d = 0, m = 1; 2.5 is d = 1, m = 25; 1000 is d = 0, m = 1000.
    accepted({"type": "number"}, [1, 2.5, 1000], LN0(0), LZ(1), LN0(1), LZ(25), LN0(0), LZ(1000))
    accepted({"type": ["integer", "number"]}, [1, 2.5], CAT((1, 1)), LZ(1), LN0(1), LZ(25))


def test_enum_codes_member_counts_instead_of_type_and_value() -> None:
    schema = {"type": "string", "enum": ["a", "b", "c", "a"]}
    accepted(schema, ["a", "a", "b"], CAT((2, 1, 0)))
    data, exceptions, rejected = bits(schema, ["a", "z"])
    assert rejected == 1
    assert close(exceptions, FLAG, CAT((1, 1)), U("z"))
    assert close(data, exceptions, CAT((1, 0, 0)))


def test_enum_of_containers_and_json_equality() -> None:
    schema = {"enum": [[1, 2], {"x": None}, 1]}
    accepted(schema, [[1, 2], {"x": None}, [1.0, 2], 1.0], CAT((1, 2, 1)))  # members in canonical order: 1, [1,2], {x}
    assert bits(schema, [[2, 1], True])[2] == 2


def test_const_costs_nothing() -> None:
    accepted({"const": "a"}, ["a", "a"])
    accepted({"const": {"k": [1]}}, [{"k": [1.0]}])
    assert bits({"const": "a"}, ["a", "b"])[2] == 1
    assert bits({"const": "a", "enum": ["b"]}, ["a"])[2] == 1


# ---------------------------------------------------------------------------
# Numbers
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("schema", "docs", "terms"),
    [
        ({"type": "integer"}, [1, -4], (LZ(1), LZ(-4))),
        ({"type": "integer", "minimum": 0, "maximum": 9}, [0, 5, 9], (UNI(10), UNI(10), UNI(10))),
        ({"type": "integer", "minimum": 3}, [3, 10], (LN0(0), LN0(7))),
        ({"type": "integer", "maximum": 10}, [3, 10], (LN0(7), LN0(0))),
        ({"type": "integer", "exclusiveMinimum": 0, "exclusiveMaximum": 10}, [1, 9], (UNI(9), UNI(9))),
        ({"type": "integer", "minimum": 0.5, "maximum": 3.5}, [1, 3], (UNI(3), UNI(3))),
        ({"type": "integer", "minimum": 0, "exclusiveMinimum": 2}, [3], (LN0(0),)),  # the tighter bound wins
        ({"type": "integer", "minimum": -(2**70)}, [2**70], (LN0(2**71),)),
    ],
)
def test_integer_codes(schema: dict[str, Any], docs: list[Any], terms: tuple[float, ...]) -> None:
    accepted(schema, docs, *terms)


@pytest.mark.parametrize(
    ("schema", "docs", "terms"),
    [
        ({"type": "number"}, [0.25], (LN0(2), LZ(25))),
        ({"type": "number", "minimum": 0, "maximum": 1}, [0.25, 0.5], (LN0(2), UNI(101), LN0(1), UNI(11))),
        ({"type": "number", "minimum": 0}, [0.25], (LN0(2), LN0(25))),
        ({"type": "number", "maximum": 1}, [0.25], (LN0(2), LN0(75))),
        ({"type": "number", "minimum": 0.05, "maximum": 1}, [0.5], (LN0(1), UNI(10))),  # grid points 0.1 … 1.0
        ({"type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1}, [0.5], (LN0(1), UNI(9))),
        ({"type": "number", "minimum": -1.5, "maximum": 1.5}, [-1.25], (LN0(2), UNI(301))),
        # Bounds off the binary grid are read as decimals, like values: 0.1 … 0.3 is three grid points.
        ({"type": "number", "minimum": 0.1, "maximum": 0.3}, [0.1, 0.2, 0.3], (LN0(1), UNI(3)) * 3),
        ({"type": "number", "minimum": 0.1}, [0.1], (LN0(1), LN0(0))),
        ({"type": "number", "maximum": 0.3}, [0.3], (LN0(1), LN0(0))),
        ({"type": "number", "exclusiveMaximum": 0.3}, [0.2], (LN0(1), LN0(0))),
    ],
)
def test_decimal_codes(schema: dict[str, Any], docs: list[Any], terms: tuple[float, ...]) -> None:
    accepted(schema, docs, *terms)


@pytest.mark.parametrize(
    ("schema", "value"),
    [
        ({"minimum": 0}, -1),
        ({"exclusiveMinimum": 0}, 0),
        ({"maximum": 1}, 1.5),
        ({"exclusiveMaximum": 1}, 1),
        ({"maximum": 2**64}, 2**64 + 1),
    ],
)
def test_out_of_bounds_numbers_are_exceptions(schema: dict[str, Any], value: Any) -> None:
    data, exceptions, rejected = bits(schema, [value])
    assert rejected == 1 and data == exceptions
    assert close(exceptions, FLAG, CAT((0, 1)), U(value))
    assert bits(schema, ["not a number"])[2] == 0  # numeric bounds do not apply to strings


# ---------------------------------------------------------------------------
# Strings
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("schema", "docs", "terms"),
    [
        ({"type": "string"}, ["ab", "é"], (LN0(2), 16.0, LN0(1), 16.0)),  # é is two UTF-8 bytes
        ({"type": "string", "minLength": 1, "maxLength": 3}, ["ab"], (UNI(3), 16.0)),
        ({"type": "string", "minLength": 1}, ["ab"], (LN0(1), 16.0)),
        ({"type": "string", "maxLength": 3}, ["ab"], (UNI(4), 16.0)),  # a missing minLength is 0
        ({"type": "string", "pattern": PHONE.source}, ["555-1234"], (LN0(8), 3 * LOG10, 4 * LOG10)),
        (
            {"type": "string", "pattern": WORD_OR_CODE.source},
            ["abc", "12"],
            (LN0(3), UNI(2), 3 * math.log2(26), LN0(3 - 1), LN0(2), UNI(2), 2 * LOG10, UNI(3)),
        ),
        ({"type": "string", "format": "date"}, ["2024-01-02"], (LN0(10), 10 * 4.0)),
        ({"type": "string", "format": "uuid"}, ["123e4567-e89b-12d3-a456-426614174000"], (LN0(36), 36 * math.log2(17))),
        ({"type": "string", "format": "x-unknown"}, ["ab"], (LN0(2), 16.0)),
        ({"type": "string", "pattern": "^a+$"}, ["aa"], (LN0(2), 16.0)),  # not a safe Pattern: raw characters
    ],
)
def test_string_codes(schema: dict[str, Any], docs: list[Any], terms: tuple[float, ...]) -> None:
    accepted(schema, docs, *terms)


@pytest.mark.parametrize(
    ("schema", "value"),
    [
        ({"minLength": 3}, "ab"),
        ({"maxLength": 1}, "ab"),
        ({"pattern": PHONE.source}, "555-12345"),
        ({"pattern": WORD_OR_CODE.source}, "a1"),
        ({"pattern": "^a+$"}, "b"),
        ({"format": "date"}, "2024-13-40"),
        ({"format": "email"}, "not an email"),
    ],
)
def test_string_mismatches_are_exceptions(schema: dict[str, Any], value: str) -> None:
    _, exceptions, rejected = bits({"type": "string", **schema}, [value])
    assert rejected == 1
    assert close(exceptions, FLAG, CAT((0, 1)), U(value))


def test_format_alphabets() -> None:
    assert len(FORMAT_ALPHABET_SIZES) == FORMAT_COUNT == 10
    assert set(FORMAT_ALPHABET_SIZES) == {
        "date-time",
        "date",
        "time",
        "duration",
        "email",
        "hostname",
        "ipv4",
        "ipv6",
        "uri",
        "uuid",
    }
    assert FORMAT_ALPHABET_SIZES["date"] == 16 and FORMAT_ALPHABET_SIZES["uuid"] == 17
    assert FORMAT_ALPHABET_SIZES["email"] == FORMAT_ALPHABET_SIZES["hostname"] == FORMAT_ALPHABET_SIZES["uri"] == 64


def test_a_matching_pattern_is_cheaper_than_raw_characters() -> None:
    docs = [f"555-{i:04d}" for i in range(20)]
    with_pattern = bits({"type": "string", "pattern": PHONE.source}, docs)[0]
    raw = bits({"type": "string"}, docs)[0]
    assert with_pattern < raw


# ---------------------------------------------------------------------------
# Booleans, null
# ---------------------------------------------------------------------------


def test_booleans_and_null() -> None:
    accepted({"type": "boolean"}, [True, True, False], CAT((2, 1)))
    accepted({"type": "null"}, [None, None])
    accepted({"type": ["boolean", "null"]}, [True, None], CAT((1, 1)), CAT((1, 0)))


# ---------------------------------------------------------------------------
# Arrays
# ---------------------------------------------------------------------------


def test_list_codes_lengths_and_elements() -> None:
    schema = {"type": "array", "items": {"type": "integer"}}
    accepted(schema, [[1, 2], [3], []], LN0(2), LN0(1), LN0(0), LZ(1), LZ(2), LZ(3), flags=2)
    bounded = {"type": "array", "maxItems": 3, "items": {"type": "integer"}}
    accepted(bounded, [[1, 2], [3]], UNI(4), UNI(4), LZ(1), LZ(2), LZ(3), flags=2)
    accepted({"type": "array", "minItems": 1}, [[1]], LN0(0), CAT((0, 0, 1, 0, 0, 0, 0)), LZ(1), flags=2)


def test_tuple_codes_positions_and_rejects_overflow_under_items_false() -> None:
    schema = {"type": "array", "prefixItems": [{"type": "string"}, {"type": "integer"}], "items": False}
    data, exceptions, rejected = bits(schema, [["a", 1], ["b", 2, 3]])
    assert rejected == 1
    assert close(exceptions, 3 * FLAG, CAT((1, 1)), U(["b", 2, 3]))
    assert close(data, exceptions, LN0(2), LN0(1), 8.0, LZ(1))
    # Positions past the prefix use items.
    open_tuple = {"type": "array", "prefixItems": [{"type": "string"}], "items": {"type": "integer"}}
    accepted(open_tuple, [["a", 1, 2]], LN0(3), LN0(1), 8.0, LZ(1), LZ(2), flags=3)


def test_array_bounds_unique_items_and_contains_only_reject() -> None:
    assert bits({"minItems": 2}, [[1], [1, 2]])[2] == 1
    assert bits({"maxItems": 1}, [[1], [1, 2]])[2] == 1
    unique = bits({"type": "array", "uniqueItems": True}, [[1, 2], [1, 1.0], [{"a": 1}, {"a": 1.0}], [[1], [2]]])
    assert unique[2] == 2
    contains = bits({"type": "array", "contains": {"type": "string"}}, [[1, "a"], [1, 2], []])
    assert contains[2] == 2
    # The checks add no data bits for accepted arrays.
    assert bits({"type": "array", "uniqueItems": True}, [[1, 2]])[0] == bits({"type": "array"}, [[1, 2]])[0]


# ---------------------------------------------------------------------------
# Records
# ---------------------------------------------------------------------------


def test_record_optional_presence_and_children() -> None:
    schema = {
        "type": "object",
        "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
        "required": ["a"],
    }
    # Presence of b, no extras, then a's values and b's value.
    accepted(schema, [{"a": 1}, {"a": 2, "b": 3}], CAT((1, 1)), CAT((2, 0)), LZ(1), LZ(2), LZ(3), flags=3)
    _, _, rejected = bits(schema, [{"b": 3}])
    assert rejected == 1


def test_dependent_required_codes_the_consequent_only_without_the_antecedent() -> None:
    props: dict[str, Any] = {"a": {}, "b": {}, "c": {}}
    docs: list[Any] = [{"a": 1, "b": 1}, {"b": 1}, {}, {"c": 1}]
    plain = bits({"properties": props}, docs)[0]
    dependent = bits({"properties": props, "dependentRequired": {"a": ["b"]}}, docs)[0]
    # b's presence moves from all four objects to the three without a.
    assert close(dependent - plain, CAT((1, 2)), -CAT((2, 2)))
    assert bits({"properties": props, "dependentRequired": {"a": ["b"]}}, [{"a": 1}])[2] == 1
    # A required antecedent makes the consequent's presence free.
    always = bits({"properties": props, "required": ["a"], "dependentRequired": {"a": ["b"]}}, [{"a": 1, "b": 2}])
    plain_required = bits({"properties": props, "required": ["a"]}, [{"a": 1, "b": 2}])
    assert close(always[0], plain_required[0], -CAT((1, 0)))


def test_presence_bits_are_the_records_own_presence_terms() -> None:
    props: dict[str, Any] = {"a": {"type": "integer"}, "b": {}, "c": {"type": "string"}}
    docs: list[Any] = [{"a": 1, "b": 1}, {"b": 1}, {}, {"c": "x"}, 7]
    view = view_of(docs)
    plain = node({"properties": props})
    dependent = node({"properties": props, "dependentRequired": {"a": ["b"]}})
    # One CAT per optional property; with a → b, b is coded only on the objects without a.
    assert close(SCORER.presence_bits(plain, view), CAT((1, 3)), CAT((2, 2)), CAT((1, 3)))
    assert close(SCORER.presence_bits(dependent, view), CAT((1, 3)), CAT((1, 2)), CAT((1, 3)))
    # A rule every object satisfies changes the data bits by exactly the change of the presence bits.
    change = SCORER.data_bits(dependent, view)[0] - SCORER.data_bits(plain, view)[0]
    assert close(change, SCORER.presence_bits(dependent, view), -SCORER.presence_bits(plain, view))
    # Required properties cost nothing; rows the node rejects (the scalar 7 under type object) are not coded.
    required = node({"type": "object", "properties": props, "required": ["b"]})
    assert close(SCORER.presence_bits(required, view_of([{"a": 1, "b": 1}, {"b": 1}, 7])), CAT((1, 1)), CAT((0, 2)))
    for schema in ({"type": "object"}, {"enum": [{"a": 1}]}, {"const": {"a": 1}}, False):
        assert SCORER.presence_bits(node(schema), view) == 0.0
    assert SCORER.presence_bits(plain, view_of([1, "s"])) == 0.0  # no object rows
    for schema in ({"anyOf": [{"properties": props}]}, {"$ref": "#/$defs/r", "$defs": {"r": {}}}):
        with pytest.raises(ValueError, match="Record node itself"):
            SCORER.presence_bits(node(schema), view)


def test_field_group_presence_is_coded_once() -> None:
    props: dict[str, Any] = {"a": {}, "b": {}, "c": {}}
    docs: list[Any] = [{"a": 1, "b": 1}, {}, {"c": 1}]
    plain = bits({"properties": props}, docs)[0]
    group = {"properties": props, "dependentRequired": {"a": ["b"], "b": ["a"]}}
    assert close(bits(group, docs)[0], plain, -CAT((1, 2)))
    assert bits(group, [{"a": 1}])[2] == 1  # a partial group is an exception
    # A three-member cycle is one group too.
    cycle = {"properties": props, "dependentRequired": {"a": ["b"], "b": ["c"], "c": ["a"]}}
    full: list[Any] = [{"a": 1, "b": 1, "c": 1}, {}]
    assert close(bits(cycle, full)[0], bits({"properties": props}, full)[0], -2 * CAT((1, 1)))


def test_closed_record_rejects_extras() -> None:
    schema = {"properties": {"a": {}}, "additionalProperties": False}
    _, exceptions, rejected = bits(schema, [{"a": 1}, {"a": 1, "x": 2}])
    assert rejected == 1
    assert close(exceptions, 2 * FLAG, CAT((1, 1)), U({"a": 1, "x": 2}))
    # patternProperties keys are not extras.
    patterned = {"properties": {"a": {}}, "patternProperties": {PHONE.source: {}}, "additionalProperties": False}
    assert bits(patterned, [{"a": 1, "555-1234": 2}])[2] == 0


def test_universal_bits_are_the_exception_payload() -> None:
    docs: list[Any] = [{"a": [1, "s", None], "b": {"c": True}}, 2.5, "x", {}]
    assert close(SCORER.universal_bits(view_of(docs)), *(U(doc) for doc in docs))
    # They are what a rejecting schema pays on top of the flag and CAT(|A|, |R|).
    _, exceptions, rejected = bits(False, docs)
    assert rejected == len(docs) and close(exceptions, FLAG, CAT((0, 4)), SCORER.universal_bits(view_of(docs)))
    assert SCORER.universal_bits(view_of([])) == 0.0


def test_closed_record_codes_no_extras_flag() -> None:
    # No accepted object can hold an extra, so the open Record's CAT(n, 0) flag is not paid (design: closed vs open).
    docs: list[Any] = [{"a": 1}, {"a": 2}, {"a": 3}]
    record = {"type": "object", "properties": {"a": {"type": "integer"}}, "required": ["a"]}
    accepted({**record, "additionalProperties": False}, docs, LZ(1), LZ(2), LZ(3), flags=2)
    accepted(record, docs, CAT((3, 0)), LZ(1), LZ(2), LZ(3), flags=2)
    # With patternProperties, pattern-matched extras can occur, so the flag stays.
    patterned = {"properties": {"a": {}}, "patternProperties": {PHONE.source: {}}, "additionalProperties": False}
    plain_data = bits({"properties": {"a": {}}, "additionalProperties": False}, docs)[0]
    assert close(bits(patterned, docs)[0], plain_data, CAT((3, 0)))


def test_open_record_extras() -> None:
    schema = {"type": "object", "properties": {"a": {"type": "integer"}}, "required": ["a"]}
    docs = [{"a": 1}, {"a": 1, "x": "s"}]
    accepted(schema, docs, CAT((1, 1)), LN0(1), STR("x"), U("s"), 2 * LZ(1), flags=2)
    typed = {**schema, "additionalProperties": {"type": "string"}}
    accepted(typed, docs, CAT((1, 1)), LN0(1), STR("x"), LN0(1), 8.0, 2 * LZ(1), flags=3)


def test_hybrid_record_and_map() -> None:
    schema = {
        "type": "object",
        "properties": {"id": {"type": "integer"}},
        "required": ["id"],
        "additionalProperties": {"type": "integer"},
    }
    # The Record field id, then the Map part: one object with two extras, their keys, and their values.
    accepted(schema, [{"id": 1, "a": 2, "b": 3}], CAT((0, 1)), LN0(2), STR("a"), STR("b"), LZ(1), LZ(2), LZ(3), flags=3)


# ---------------------------------------------------------------------------
# Maps
# ---------------------------------------------------------------------------


def test_map_codes_key_counts_keys_and_values() -> None:
    schema = {"type": "object", "additionalProperties": {"type": "integer"}}
    docs: list[Any] = [{"x": 1, "y": 2}, {}]
    accepted(schema, docs, LN0(2), LN0(0), STR("x"), STR("y"), LZ(1), LZ(2), flags=2)
    bounded = {**schema, "maxProperties": 2}
    accepted(bounded, docs, UNI(3), UNI(3), STR("x"), STR("y"), LZ(1), LZ(2), flags=2)
    named = {**schema, "propertyNames": {"maxLength": 1}}
    accepted(named, docs, LN0(2), LN0(0), UNI(2), 8.0, UNI(2), 8.0, LZ(1), LZ(2), flags=2)
    listed = {**schema, "propertyNames": {"enum": ["x", "y", "z"]}}
    accepted(listed, docs, LN0(2), LN0(0), CAT((1, 1, 0)), LZ(1), LZ(2), flags=2)


def test_property_names_and_pattern_properties() -> None:
    names = {"type": "object", "additionalProperties": {}, "propertyNames": {"pattern": PHONE.source}}
    assert bits(names, [{"555-1234": 1}, {"bad": 1}])[2] == 1
    keyed = {"type": "object", "patternProperties": {PHONE.source: {"type": "integer"}}, "additionalProperties": False}
    assert bits(keyed, [{"555-1234": 1}, {"x": 1}])[2] == 1
    assert bits(keyed, [{"555-1234": "s"}])[2] == 1  # the value is rejected by the pattern's subschema
    # A Map keyed by the pattern: key count, key literal, and the value under the pattern's subschema.
    accepted(keyed, [{"555-1234": 1}], LN0(1), STR("555-1234"), LZ(1), flags=2)


def test_record_or_map_by_local_scoring() -> None:
    def record_and_map(docs: list[Any]) -> tuple[Score, Score]:
        view = view_of(docs)
        names = sorted({key for doc in docs for key in doc})
        record = SchemaNode(
            kw={
                "type": "object",
                "properties": {name: naive_merge_node(view.child_key(name)) for name in names},
            }
        )
        mapping = SchemaNode(kw={"type": "object", "additionalProperties": naive_merge_node(view.child_map())})
        return SCORER.score(record, view), SCORER.score(mapping, view)

    record, mapping = record_and_map([{"id": i, "name": f"n{i}"} for i in range(30)])
    assert record.total < mapping.total
    record, mapping = record_and_map([{f"key{i}": i} for i in range(30)])
    assert mapping.total < record.total
    assert record.n_scored == mapping.n_scored == 60


def test_tuple_or_list_by_local_scoring() -> None:
    docs = [[f"s{i}", i] for i in range(30)]
    view = view_of(docs)
    elements = view.elements()
    tuple_node = SchemaNode(
        kw={
            "type": "array",
            "prefixItems": [naive_merge_node(view.elements(0)), naive_merge_node(view.elements(1))],
            "items": SchemaNode(boolean=False),
        }
    )
    list_node = SchemaNode(kw={"type": "array", "items": naive_merge_node(elements)})
    assert SCORER.score(tuple_node, view).total < SCORER.score(list_node, view).total


# ---------------------------------------------------------------------------
# anyOf / oneOf
# ---------------------------------------------------------------------------


def test_any_of_codes_branch_counts_and_branch_data() -> None:
    schema = {"anyOf": [{"type": "integer"}, {"type": "string"}]}
    accepted(schema, [1, "a", "b"], CAT((1, 2)), LZ(1), LN0(1), 8.0, LN0(1), 8.0, flags=3)
    _, _, rejected = bits(schema, [1, None])
    assert rejected == 1


def test_any_of_rows_go_to_the_first_branch_that_accepts_the_whole_value() -> None:
    accepted({"anyOf": [{}, {"type": "integer"}]}, [1], CAT((1, 0)), CAT((0, 0, 1, 0, 0, 0, 0)), LZ(1), flags=2)
    deep = {
        "anyOf": [
            {"type": "object", "properties": {"a": {"type": "string"}}},
            {"type": "object", "properties": {"a": {"type": "integer"}}},
        ]
    }
    accepted(deep, [{"a": 1}], CAT((0, 1)), CAT((1, 0)), CAT((1, 0)), LZ(1), flags=3)


def test_branch_views_override_the_assignment() -> None:
    store = build_store([1, 2, "a"])
    view = View.root(store)
    schema = node({"anyOf": [{}, {"type": ["integer", "string"]}]})
    hints = {schema.nid: [View.root(store, [2]), View.root(store, [0, 1])]}
    data, exceptions, rejected = SCORER.data_bits(schema, view, branch_views=hints)
    assert rejected == 0 and exceptions == 3 * FLAG
    # "a" under {}: type choice over 7, then the string; 1 and 2 under the second branch.
    terms = (CAT((1, 2)), CAT((0, 0, 0, 0, 1, 0, 0)), LN0(1), 8.0, CAT((2, 0)), LZ(1), LZ(2))
    assert close(data, exceptions, *terms)
    # A hinted branch codes its rows even when it rejects them (exceptions inside the branch);
    # rows no hint holds go to the first accepting branch.
    narrow = node({"anyOf": [{"type": "integer"}, {}]})
    hint = {narrow.nid: [View.root(store, [2]), View.root(store, [])]}
    assert SCORER.data_bits(narrow, view, branch_views=hint)[2] == 1
    with pytest.raises(ValueError, match="2 branches"):
        SCORER.data_bits(schema, view, branch_views={schema.nid: [view]})


def test_one_of_rejects_ambiguous_and_unmatched_rows() -> None:
    schema = {"oneOf": [{"type": "integer"}, {"minimum": 0}]}
    # 1 matches both branches; -1 only the first; "a" only the second (minimum ignores strings).
    _, _, rejected = bits(schema, [1, -1, "a"])
    assert rejected == 1
    assert bits({"oneOf": [{"type": "integer"}, {"type": "string"}]}, [None])[2] == 1


# ---------------------------------------------------------------------------
# Keywords that only reject, and $ref
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("schema", "docs", "n_rejected"),
    [
        ({"not": {"type": "string"}}, [1, "a"], 1),
        ({"not": {"required": ["a"]}}, [{"a": 1}, {"b": 1}], 1),
        ({"allOf": [{"minimum": 0}, {"maximum": 5}]}, [1, -1, 6], 2),
        ({"if": {"type": "integer"}, "then": {"minimum": 0}, "else": {"type": "string"}}, [1, -1, "a", None], 2),
        ({"then": {"minimum": 0}}, [-1], 0),  # then without if has no effect
        ({"properties": {"a": {"not": {}}}}, [{"a": 1}, {}], 1),
        ({"minProperties": 1, "maxProperties": 1}, [{}, {"a": 1}, {"a": 1, "b": 2}], 2),
    ],
)
def test_check_only_keywords(schema: dict[str, Any], docs: list[Any], n_rejected: int) -> None:
    assert bits(schema, docs)[2] == n_rejected


def test_ref_codes_like_its_target() -> None:
    docs: list[Any] = [{"x": 1}, {"x": 7}, {}]
    inline = {"properties": {"x": {"type": "integer", "minimum": 0}}}
    by_ref = {"$defs": {"n": {"type": "integer", "minimum": 0}}, "properties": {"x": {"$ref": "#/$defs/n"}}}
    assert bits(by_ref, docs) == bits(inline, docs)
    # A subschema scored alone resolves $ref against the scorer's root.
    root = node(by_ref)
    scorer = MDLScorer(root=root)
    assert scorer.n_defs == 1
    child_view = view_of(docs).child_key("x")
    assert scorer.data_bits(root.kw["properties"]["x"], child_view) == SCORER.data_bits(
        node({"type": "integer", "minimum": 0}), child_view
    )


def test_recursive_definitions_follow_the_data() -> None:
    recursive = {
        "$defs": {"t": {"type": "object", "properties": {"next": {"$ref": "#/$defs/t"}}}},
        "$ref": "#/$defs/t",
    }
    innermost = {"type": "object", "properties": {"next": {}}}
    unrolled = {
        "type": "object",
        "properties": {"next": {"type": "object", "properties": {"next": innermost}}},
    }
    docs: list[Any] = [{"next": {"next": {}}}]
    assert close(bits(recursive, docs)[0], bits(unrolled, docs)[0])


@pytest.mark.parametrize(
    "schema",
    [
        {"$defs": {"a": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"},
        {"$defs": {"a": {"$ref": "#/$defs/b"}, "b": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"},
        {"$defs": {"a": {"anyOf": [{"$ref": "#/$defs/a"}]}}, "$ref": "#/$defs/a"},
        {"$defs": {"a": {"type": "integer", "not": {"$ref": "#/$defs/a"}}}, "$ref": "#/$defs/a"},
    ],
)
def test_ref_cycles_without_a_step_are_rejected(schema: dict[str, Any]) -> None:
    with pytest.raises(ValueError, match="cycle"):
        bits(schema, [1])


def test_unresolvable_refs_are_rejected() -> None:
    with pytest.raises(ValueError, match="local"):
        bits({"$ref": "https://example.com/s.json"}, [1])
    with pytest.raises(ValueError, match="does not resolve"):
        bits({"$ref": "#/$defs/missing"}, [1])


def test_annotations_cost_no_data_bits() -> None:
    plain = bits({"type": "string"}, ["a"])
    annotated = {"type": "string", "title": "t", "x-latent-type": "date", "x-key": True, "$defs": {"u": {}}}
    assert bits(annotated, ["a"]) == plain


# ---------------------------------------------------------------------------
# Score, flags, determinism, errors
# ---------------------------------------------------------------------------


def test_score_fields() -> None:
    schema = node({"type": "object", "properties": {"a": {"type": "integer"}}, "required": ["a"]})
    view = view_of([{"a": 1}, {"a": "x"}, {"b": [1, 2]}])
    score = SCORER.score(schema, view)
    assert score.model_bits == model_bits(schema)
    assert (score.data_bits, score.exception_bits, score.n_rejected) == SCORER.data_bits(schema, view)
    assert score.n_rejected == 2  # the object without a, and the string under a
    assert score.n_scored == 3 + 3 + 2  # three objects, three members, and the two elements under b
    assert score.total == math.fsum((score.model_bits, score.data_bits))
    assert 0.0 < score.exception_bits <= score.data_bits
    empty = SCORER.score(schema, View.root(build_store([1]), []))
    assert (empty.data_bits, empty.exception_bits, empty.n_rejected, empty.n_scored) == (0.0, 0.0, 0, 0)


@pytest.mark.parametrize(
    "fields",
    [
        (-1.0, 0.0, 0.0, 0, 0),
        (0.0, math.inf, 0.0, 0, 0),
        (0.0, 1.0, 2.0, 0, 0),
        (0.0, 0.0, 0.0, 2, 1),
        (0.0, 0.0, 0.0, -1, 0),
        (1, 0.0, 0.0, 0, 0),
    ],
)
def test_score_rejects_inconsistent_fields(fields: tuple[Any, ...]) -> None:
    with pytest.raises((TypeError, ValueError)):
        Score(*fields)


def test_flagged_documents_and_items_are_excluded() -> None:
    docs: list[Any] = [{"a": 1}, {"a": "x", "b": 2}, {"a": 3}]
    schema = node({"type": "object", "properties": {"a": {"type": "integer"}}})
    store = build_store(docs)
    store.flags.flag_documents([1])
    without = build_store([docs[0], docs[2]])
    assert SCORER.score(schema, View.root(store)) == SCORER.score(schema, View.root(without))
    # A view made before the flags changed catches up.
    other = build_store(docs)
    stale = View.root(other)
    other.flags.flag_documents([1])
    assert SCORER.score(schema, stale) == SCORER.score(schema, View.root(without))


def test_scores_are_bit_identical_across_document_order_and_chunking() -> None:
    docs: list[Any] = [
        {"id": i, "tags": ["x", i % 3, 2.5], "kind": ["a", "b", None][i % 3], "extra": {"k": i % 2 == 0}}
        for i in range(40)
    ] + ["s", 7, None, [1, [2]]]
    view = view_of(docs)
    candidates = [
        naive_merge_node(view),
        node({}),
        node({"type": "object", "properties": {"id": {"type": "integer", "minimum": 0}}, "additionalProperties": False}),
        node({"anyOf": [{"type": "object"}, {"type": ["string", "integer"]}]}),
    ]
    expected = [SCORER.score(candidate, view) for candidate in candidates]
    rng = random.Random(20251016)
    for chunk_size in (1, 3, 17):
        shuffled = list(docs)
        rng.shuffle(shuffled)
        other = view_of(shuffled, chunk_size)
        assert [SCORER.score(candidate, other) for candidate in candidates] == expected


def test_union_views_are_scored_like_their_rows() -> None:
    store = build_store([{"a": 1, "b": {"a": 2}}])
    root = View.root(store)
    union = View.union([root.child_key("a"), root.child_key("b").child_key("a")])
    assert len(union.parts) == 2
    data, exceptions, rejected = SCORER.data_bits(node({"type": "integer"}), union)
    assert (exceptions, rejected) == (FLAG, 0)
    assert close(data, FLAG, LZ(1), LZ(2))


def test_argument_errors() -> None:
    view = view_of([1])
    with pytest.raises(TypeError):
        SCORER.score({"type": "integer"}, view)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        SCORER.score(node({}), [1])  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        SCORER.data_bits(node({}), view, branch_views=[view])  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        SCORER.data_bits(node({}), view, branch_views={1: [1]})  # type: ignore[list-item]
    with pytest.raises(ValueError, match="another"):
        SCORER.data_bits(node({}), view, branch_views={1: [view_of([1])]})
    with pytest.raises(TypeError):
        MDLScorer(root={"type": "integer"})  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        MDLScorer(validator=object())  # type: ignore[arg-type]


@pytest.mark.parametrize(
    "schema",
    [
        {"multipleOf": 2},
        {"type": "str"},
        {"minLength": -1},
        {"minLength": 1.5},
        {"maximum": "3"},
        {"minimum": True},
        {"pattern": 3},
        {"enum": "a"},
        {"uniqueItems": 1},
        {"required": "a"},
    ],
)
def test_malformed_keywords_are_rejected(schema: dict[str, Any]) -> None:
    with pytest.raises((TypeError, ValueError)):
        SCORER.data_bits(SchemaNode(kw=schema), view_of([1]))


def test_type_counts_cover_every_jtype() -> None:
    # One row of every JType under {}: the type choice is CAT of seven ones.
    docs: list[Any] = [None, True, 1, 0.5, "", [], {}]
    data, exceptions, rejected = bits({}, docs)
    assert rejected == 0 and exceptions == FLAG
    terms = (CAT([1] * len(JType)), CAT((1, 0)), LZ(1), LN0(1), LZ(5), LN0(0), LN0(0), CAT((1, 0)))
    assert close(data, FLAG, *terms)
