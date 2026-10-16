"""Smoke tests for jsonschema-rs edge cases (task 5.2; Requirements 1.6, 1.11, 29.3).

The design leaves two questions open until milestone M1: whether
jsonschema-rs handles Python integers beyond 64 bits, and whether the two
RFC 8259 literals that :func:`~jsinfer.io.ingest.parse_document` rejects
(number literals that overflow an IEEE 754 double, and strings with an
unpaired UTF-16 surrogate escape) could be kept instead. These tests pin what
jsonschema-rs 0.58.6 does. They go through :class:`Validator` wherever it
can be used, and call ``jsonschema_rs`` directly only to describe the library
itself (with the Validator's options: ``validate_formats=True``,
``offline=True``).

Observed behavior
    Integers beyond 64 bits are exact. As instances they are integers and
    numbers; ``minimum``, ``maximum``, ``exclusiveMinimum``,
    ``exclusiveMaximum``, and ``multipleOf`` compare them exactly, also
    against bounds that are themselves big integers; ``enum``, ``const``,
    and ``uniqueItems`` tell apart neighbours such as ``2**64`` and
    ``2**64 + 1``, at the top level and nested in arrays and objects. Error
    messages print them in full. The Validator's compile cache keeps schemas
    that differ only in such neighbours apart.

    Integral floats beyond 2**53 are compared with integers by their
    shortest round-trip decimal (``repr``), not by their exact binary value.
    ``float(2**62)`` is exactly ``2**62``, yet ``const`` and ``uniqueItems``
    treat the two as different, while ``1e40`` (which is ``10**40 +
    303786028427003666890752``) is treated as equal to ``10**40``. Ordering
    follows the same decimal: ``2**64 + 1 <= float(2**64)`` and ``2**64 <
    float(2**64)`` hold for jsonschema-rs. Integral floats whose shortest
    decimal is exact (every one up to 2**53 in magnitude, and others such as
    ``1e20``) compare exactly. Under draft-04, jsonschema-rs also tells
    ``1.0`` from ``1`` for ``type: integer`` and in the meta-schema
    (``"maxLength": 2.0`` does not compile).

    Before this task the Validator keyed its compile cache by
    ``canonical_bytes``, which writes integral floats as integers, and passed
    schemas and instances on unchanged. ``{"const": 2**62}`` and ``{"const":
    float(2**62)}`` then shared one compiled validator although jsonschema-rs
    treats them differently, so a verdict depended on which of the two was
    compiled first (and could differ between worker processes). The
    Validator now hands jsonschema-rs exact integers for integral floats
    from draft-06 on, and keys draft-04 schemas by the spelling of their
    integral numbers as well (see the ``io/validator.py`` docstring,
    Numbers). The tests in the section on the Validator below pin that.

    Overflowing doubles have no faithful representation. Python has no
    finite float for them. jsonschema-rs reads ``inf`` and ``-inf`` as a
    value of no JSON type: every ``type`` rejects it, the numeric keywords
    skip it (``{"maximum": 0}`` accepts ``inf``), and messages print it as
    ``null``. The Validator cannot compile a schema that holds one (no
    canonical form). jsonschema-rs accepts ``decimal.Decimal`` instances and
    keyword values and compares them exactly, but a ``Decimal`` is not a
    JSON value anywhere else in jsinfer: the canonical encoder, the value
    model, and :meth:`Validator.compile` all reject it. Keeping the literal
    as a raw string would change its type.

    Unpaired surrogates get no dependable verdict. A Python string with a
    lone surrogate cannot be encoded as UTF-8, and jsonschema-rs raises
    ``UnicodeEncodeError`` whenever it needs the text. A top-level string
    raises from ``is_valid`` and ``errors`` under every schema, ``{}``
    included. A nested string or object key raises from both when a keyword
    reads its text (``pattern``, ``format``, ``const``, ``enum``,
    ``uniqueItems``, ``properties``, ``additionalProperties``,
    ``propertyNames``). When a keyword fails without reading it
    (``maxLength: 0``, ``type: integer``, ``maxProperties``, ``required``),
    ``is_valid`` returns false but ``errors`` raises, because the message
    prints the instance. Only keywords that pass without reading it give a
    full answer (``{}``, ``type: string``, ``maxLength: 1``).
    ``satisfies_format`` and ``matches_pattern`` raise as well. A schema
    that holds a lone surrogate anywhere, annotations included, is a
    :class:`CompileError`.

Outcome
    ``parse_document`` keeps its failure policy unchanged:

    - integers outside the 64-bit range go through the stdlib fallback and
      are kept as exact ``int`` (Requirement 1.6). The design's assumption
      that jsonschema-rs supports them holds;
    - number literals that overflow a double stay parse failures of kind
      ``number_overflow``. RFC 8259 section 9 lets an implementation limit
      the range of numbers, and neither ``inf`` nor ``Decimal`` is a
      revision that keeps verdicts right without changing the value model;
    - strings with an unpaired surrogate escape stay parse failures of kind
      ``surrogate``. RFC 8259 section 8.2 calls the behavior of software
      receiving them unpredictable, and jsonschema-rs gives no verdict for
      them, so admitting them would break Requirement 29.3.

    ``tests/unit/test_parse_document.py`` covers those parse failures.

    The integral-float comparisons do not involve the parsing policy. They
    led to the Validator change described above: from draft-06 on, the
    Validator's verdicts follow mathematical equality, as
    ``core.jsonvalue.json_equal`` does, so a schema that writes an integral
    float as an integer (as the Schema_Emitter does) accepts the documents
    that hold the float. Draft-04 verdicts stay those of jsonschema-rs on
    the numbers as written.
"""

from __future__ import annotations

import importlib.metadata
import math
from decimal import Decimal
from typing import Any

import jsonschema_rs
import orjson
import pytest

from jsinfer.core.determinism import canonical_bytes, encode_canonical
from jsinfer.core.jsonvalue import canonical_key
from jsinfer.io.ingest import parse_document
from jsinfer.io.validator import (
    CompileError,
    Compiled,
    Draft,
    Validator,
    _may_hold_inexact_float,
    _rs_instance,
    _with_exact_integers,
)

# The jsonschema-rs release these results describe. A different version must
# re-run these tests and revisit the outcome in the module docstring.
JSONSCHEMA_RS_VERSION = "0.58.6"


@pytest.fixture
def v() -> Validator:
    return Validator()


def ok(validator: Validator, schema: dict[str, Any] | bool, instance: Any) -> bool:
    return validator.is_valid(validator.compile(schema), instance)


def rs_ok(schema: dict[str, Any] | bool, instance: Any) -> bool:
    """jsonschema-rs's own verdict, with the options the Validator uses."""
    return bool(jsonschema_rs.Draft202012Validator(schema, validate_formats=True, offline=True).is_valid(instance))


def replace_floats(value: Any, how: Any) -> Any:
    """``value`` with every float replaced by ``how(float)``."""
    if isinstance(value, float):
        return how(value)
    if isinstance(value, list):
        return [replace_floats(item, how) for item in value]
    if isinstance(value, dict):
        return {key: replace_floats(item, how) for key, item in value.items()}
    return value


def test_results_describe_the_pinned_jsonschema_rs() -> None:
    assert importlib.metadata.version("jsonschema-rs") == JSONSCHEMA_RS_VERSION


# ---------------------------------------------------------------------------
# Integers beyond 64 bits
# ---------------------------------------------------------------------------

BIG_INTEGERS = [2**63, 2**64 - 1, 2**64, 2**64 + 1, -(2**63) - 1, -(2**64) - 1, 10**40, -(10**40), 10**400]


@pytest.mark.parametrize("n", BIG_INTEGERS)
def test_big_integers_are_integers(v: Validator, n: int) -> None:
    assert ok(v, {"type": "integer"}, n)
    assert ok(v, {"type": "number"}, n)
    assert not ok(v, {"type": "string"}, n)
    assert not ok(v, {"type": "boolean"}, n)


@pytest.mark.parametrize(
    ("schema", "instance", "expected"),
    [
        # Bounds that are big integers, against their neighbours.
        ({"minimum": 2**64 + 1}, 2**64, False),
        ({"minimum": 2**64 + 1}, 2**64 + 1, True),
        ({"maximum": 2**64}, 2**64 + 1, False),
        ({"maximum": 2**64}, 2**64, True),
        ({"exclusiveMaximum": 10**40}, 10**40, False),
        ({"exclusiveMaximum": 10**40}, 10**40 - 1, True),
        ({"exclusiveMinimum": -(10**40)}, -(10**40), False),
        ({"exclusiveMinimum": -(10**40)}, -(10**40) + 1, True),
        # The 64-bit type limits that bound selection proposes.
        ({"maximum": 2**63 - 1}, 2**63, False),
        ({"maximum": 2**64 - 1}, 2**64, False),
        ({"minimum": -(2**63)}, -(2**63) - 1, False),
        ({"minimum": -(2**63)}, -(2**63), True),
        # Small or fractional bounds against huge instances.
        ({"maximum": 0}, 10**400, False),
        ({"minimum": 0}, -(10**400), False),
        ({"minimum": 1.5}, 10**40, True),
        ({"maximum": 1.5}, 10**40, False),
        # multipleOf, with big instances and a big divisor.
        ({"multipleOf": 2}, 10**40, True),
        ({"multipleOf": 2}, 10**40 + 1, False),
        ({"multipleOf": 3}, 3 * 10**40, True),
        ({"multipleOf": 3}, 3 * 10**40 + 1, False),
        ({"multipleOf": 7}, -7 * 10**30, True),
        ({"multipleOf": 2**64 + 1}, 2 * (2**64 + 1), True),
        ({"multipleOf": 2**64 + 1}, 2 * (2**64 + 1) + 1, False),
    ],
)
def test_numeric_keywords_compare_big_integers_exactly(
    v: Validator, schema: dict[str, Any], instance: int, expected: bool
) -> None:
    assert ok(v, schema, instance) is expected


@pytest.mark.parametrize(
    ("schema", "instance", "expected"),
    [
        ({"const": 2**64 + 1}, 2**64 + 1, True),
        ({"const": 2**64 + 1}, 2**64, False),
        ({"const": 2**64}, 2**64 + 1, False),
        ({"const": -(2**63) - 1}, -(2**63), False),
        ({"const": 10**400}, 10**400, True),
        ({"const": 10**400}, 10**400 + 1, False),
        ({"enum": [2**64 + 1]}, 2**64, False),
        ({"enum": [10**40, 1]}, 10**40, True),
        ({"enum": [10**40, 1]}, 10**40 + 1, False),
        ({"enum": [[10**40]]}, [10**40], True),
        ({"enum": [[10**40]]}, [10**40 + 1], False),
        ({"enum": [{"a": 10**40}]}, {"a": 10**40}, True),
        ({"enum": [{"a": 10**40}]}, {"a": 10**40 + 1}, False),
        ({"uniqueItems": True}, [2**64, 2**64 + 1], True),
        ({"uniqueItems": True}, [10**40, 10**40], False),
    ],
)
def test_enum_const_and_unique_items_compare_big_integers_exactly(
    v: Validator, schema: dict[str, Any], instance: Any, expected: bool
) -> None:
    assert ok(v, schema, instance) is expected


def test_errors_print_big_integers_in_full(v: Validator) -> None:
    compiled = v.compile({"$defs": {"n": {"minimum": 2**64 + 1}}, "$ref": "#/$defs/n"})
    [error] = v.errors(compiled, 2**64)
    assert error.keyword_location == "/$defs/n/minimum"
    assert str(2**64) in error.message
    assert str(2**64 + 1) in error.message


def test_compile_cache_keeps_neighbouring_big_integers_apart(v: Validator) -> None:
    low, high = v.compile({"const": 2**64}), v.compile({"const": 2**64 + 1})
    assert low is not high
    assert low.digest != high.digest
    assert v.is_valid(low, 2**64) and not v.is_valid(low, 2**64 + 1)
    assert v.is_valid(high, 2**64 + 1) and not v.is_valid(high, 2**64)


def test_parsed_big_integers_reach_the_validator_exactly(v: Validator) -> None:
    data = b"[18446744073709551617, -9223372036854775809, 1" + b"0" * 40 + b"]"
    value, parser = parse_document(data)
    assert parser == "stdlib"
    compiled = v.compile({"prefixItems": [{"const": 2**64 + 1}, {"const": -(2**63) - 1}, {"const": 10**40}]})
    assert v.is_valid(compiled, value)
    # The DocumentStore's encoding round-trips them, so stored documents validate the same.
    assert v.is_valid(compiled, parse_document(encode_canonical(value))[0])
    assert not v.is_valid(compiled, [2**64, -(2**63) - 1, 10**40])


# ---------------------------------------------------------------------------
# Integral floats against integers
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("n", [2**53, 2**53 + 2, -(2**53), 10**15, 2**54 + 4, 10**20])
def test_integral_floats_compare_exactly_when_their_shortest_decimal_is_exact(n: int) -> None:
    f = float(n)
    assert int(f) == n and Decimal(repr(f)) == n
    assert rs_ok({"const": n}, f)
    assert rs_ok({"const": f}, n)
    assert rs_ok({"enum": [n]}, f)
    assert not rs_ok({"uniqueItems": True}, [n, f])
    assert rs_ok({"maximum": f}, n) and rs_ok({"minimum": f}, n)


# (schema, instance, jsonschema-rs's verdict). Each verdict differs from the
# one for the exact values of the floats, and equals the one for their
# shortest round-trip decimals.
SHORTEST_DECIMAL_CASES = [
    # Equal values that jsonschema-rs tells apart: float(2**62) is exactly 2**62.
    ({"const": 2**62}, float(2**62), False),
    ({"const": float(2**64)}, 2**64, False),
    ({"enum": [2**60]}, float(2**60), False),
    ({"uniqueItems": True}, [2**60, float(2**60)], True),
    # Different values that jsonschema-rs equates: 1e40 is 10**40 + 303786028427003666890752.
    ({"const": 10**40}, 1e40, True),
    ({"uniqueItems": True}, [10**40, 1e40], False),
    # Ordering by the decimal 18446744073709552000 instead of 2**64.
    ({"maximum": float(2**64)}, 2**64 + 1, True),
    ({"exclusiveMaximum": float(2**64)}, 2**64, True),
    ({"maximum": 2**62}, float(2**62), False),
]


@pytest.mark.parametrize(("schema", "instance", "observed"), SHORTEST_DECIMAL_CASES)
def test_integral_floats_beyond_2_53_compare_by_their_shortest_decimal(
    schema: dict[str, Any], instance: Any, observed: bool
) -> None:
    assert rs_ok(schema, instance) is observed

    def exact(f: float) -> int:
        return int(f)

    def shortest(f: float) -> Decimal:
        return Decimal(repr(f))

    assert rs_ok(replace_floats(schema, exact), replace_floats(instance, exact)) is not observed
    assert rs_ok(replace_floats(schema, shortest), replace_floats(instance, shortest)) is observed


def test_draft04_tells_integral_floats_from_integers() -> None:
    def d4(schema: dict[str, Any], instance: Any) -> bool:
        return bool(jsonschema_rs.Draft4Validator(schema, validate_formats=True, offline=True).is_valid(instance))

    assert d4({"type": "integer"}, 1) and not d4({"type": "integer"}, 1.0)
    assert not d4({"type": "integer"}, 1e19)
    assert d4({"enum": [1.0]}, 1)
    with pytest.raises(jsonschema_rs.ValidationError):
        jsonschema_rs.Draft4Validator({"maxLength": 2.0})


# ---------------------------------------------------------------------------
# The Validator's handling of integral floats
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(("schema", "instance", "observed"), SHORTEST_DECIMAL_CASES)
def test_validator_compares_integral_floats_by_exact_value(
    v: Validator, schema: dict[str, Any], instance: Any, observed: bool
) -> None:
    assert ok(v, schema, instance) is not observed
    for declared in (Draft.DRAFT6, Draft.DRAFT7, Draft.DRAFT201909, Draft.DRAFT202012):
        assert ok(v, {"$schema": declared.uri, **schema}, instance) is not observed


@pytest.mark.parametrize("first", [2**62, float(2**62)])
def test_verdicts_do_not_depend_on_which_spelling_compiled_first(first: int | float) -> None:
    v = Validator()
    a = v.compile({"const": first})
    b = v.compile({"const": float(2**62) if isinstance(first, int) else 2**62})
    assert a is b
    assert v.is_valid(b, 2**62) and v.is_valid(b, float(2**62))
    assert not v.is_valid(b, 2**62 + 1)
    assert not v.is_valid(b, float(2**62 + 2**11))


def test_errors_print_large_integral_floats_as_integers(v: Validator) -> None:
    [error] = v.errors(v.compile({"maximum": 2**62 - 1}), float(2**62))
    assert error.message == f"{2**62} is greater than the maximum of {2**62 - 1}"
    [error] = v.errors(v.compile({"items": {"const": 10**40}}), [10**40, 1e40])
    assert (error.keyword_location, error.instance_location) == ("/items/const", "/1")


@pytest.mark.parametrize("first", ["int", "float"])
def test_draft04_schemas_keep_the_spelling_of_integral_numbers(first: str) -> None:
    v = Validator()

    def compile_int() -> Compiled:
        return v.compile({"$schema": Draft.DRAFT4.uri, "maxLength": 2})

    def compile_float() -> None:
        with pytest.raises(CompileError) as info:
            v.compile({"$schema": Draft.DRAFT4.uri, "maxLength": 2.0})
        assert info.value.digest == compile_int().digest  # equal JSON values, one digest

    if first == "int":
        compiled = compile_int()
        compile_float()
    else:
        compile_float()
        compiled = compile_int()
    assert v.is_valid(compiled, "ab") and not v.is_valid(compiled, "abc")


def test_draft04_instances_are_passed_as_written(v: Validator) -> None:
    compiled = v.compile({"$schema": Draft.DRAFT4.uri, "type": "integer"})
    assert v.is_valid(compiled, 1) and v.is_valid(compiled, 2**62)
    assert not v.is_valid(compiled, 1.0)
    assert not v.is_valid(compiled, float(2**62))
    assert [e.keyword for e in v.errors(compiled, float(2**62))] == ["type"]
    shared = v.compile({"$schema": Draft.DRAFT4.uri, "enum": [1, 2]})
    assert shared is v.compile({"$schema": Draft.DRAFT4.uri, "enum": [1, 2]})


def test_exact_integer_rewrite_copies_only_when_needed() -> None:
    plain = {"a": [1, 2.5, 1.0, float(2**52), "x", True, None], "b": {"c": -(2**53) + 1.0}}
    assert _with_exact_integers(plain, float(2**53)) is plain
    doc = {"z": (float(2**53), 1.0), "a": [{"k": -1e40, "j": 2.5}], "b": True}
    rewritten = _with_exact_integers(doc, float(2**53))
    assert rewritten == {"z": [2**53, 1.0], "a": [{"k": -(10**40) - 303786028427003666890752, "j": 2.5}], "b": True}
    assert list(rewritten) == ["z", "a", "b"] and list(rewritten["a"][0]) == ["k", "j"]
    assert type(rewritten["z"][0]) is int and type(rewritten["z"][1]) is float
    assert doc["z"] == (float(2**53), 1.0)  # the original is unchanged
    assert _with_exact_integers({"x": [1.0, -0.0]}, 0.0) == {"x": [1, 0]}
    deep: Any = 1e20  # deeper than orjson encodes, so the pre-check falls back to the walk
    for _ in range(5000):
        deep = [deep]
    out = _with_exact_integers(deep, float(2**53))
    for _ in range(5000):
        out = out[0]
    assert out == 10**20 and type(out) is int


# ---------------------------------------------------------------------------
# Overflowing doubles
# ---------------------------------------------------------------------------

JSON_TYPES = ["null", "boolean", "integer", "number", "string", "array", "object"]


@pytest.mark.parametrize("x", [math.inf, -math.inf])
def test_infinite_floats_have_no_json_type(v: Validator, x: float) -> None:
    for name in JSON_TYPES:
        assert not ok(v, {"type": name}, x)
        assert not ok(v, {"items": {"type": name}}, [x])
    assert ok(v, {}, x)
    # Numeric keywords skip the value, so wrong bounds accept it.
    assert ok(v, {"maximum": 0, "minimum": 0}, x)
    [error] = v.errors(v.compile({"type": "number"}), x)
    assert error.message == 'null is not of type "number"'


@pytest.mark.parametrize("schema", [{"maximum": math.inf}, {"const": -math.inf}, {"enum": [1, math.inf]}])
def test_infinite_floats_in_schemas_are_compile_errors(v: Validator, schema: dict[str, Any]) -> None:
    with pytest.raises(CompileError, match="not a JSON value"):
        v.compile(schema)


def test_decimals_work_in_jsonschema_rs_but_are_not_json_values_in_jsinfer(v: Validator) -> None:
    big = Decimal("1e400")
    # jsonschema-rs: an exact number, also as a keyword value.
    assert rs_ok({"type": "integer"}, big)
    assert not rs_ok({"type": "integer"}, Decimal("1.5"))
    assert not rs_ok({"maximum": 1}, big)
    assert rs_ok({"const": 10**400}, big)
    assert not rs_ok({"const": big}, Decimal(10**400 + 1))
    assert not rs_ok({"maximum": big}, Decimal(10**400 + 1))
    assert not rs_ok({"type": "number"}, Decimal("Infinity"))
    # jsinfer: no canonical form, no value-model key, no output encoding, no compiled schema.
    with pytest.raises(TypeError):
        canonical_bytes(big)
    with pytest.raises(TypeError):
        canonical_key(big)
    with pytest.raises(TypeError):
        encode_canonical(big)
    with pytest.raises(CompileError, match="not a JSON value"):
        v.compile({"maximum": big})


# ---------------------------------------------------------------------------
# Unpaired surrogates
# ---------------------------------------------------------------------------

LONE_SURROGATES = ["\ud800", "\udfff", "a\udc00b"]


@pytest.mark.parametrize("s", LONE_SURROGATES)
@pytest.mark.parametrize(
    "schema",
    [
        {},
        {"type": "string"},
        {"type": "integer"},
        {"maxLength": 0},
        {"pattern": "^a"},
        {"format": "date"},
        {"const": "a"},
        {"enum": ["a", 1]},
    ],
)
def test_lone_surrogate_strings_get_no_verdict(v: Validator, schema: dict[str, Any], s: str) -> None:
    compiled = v.compile(schema)
    with pytest.raises(UnicodeEncodeError):
        v.is_valid(compiled, s)
    with pytest.raises(UnicodeEncodeError):
        v.errors(compiled, s)


@pytest.mark.parametrize(
    ("schema", "instance"),
    [
        ({"items": {"pattern": "^a"}}, ["\ud800"]),
        ({"items": {"format": "date"}}, ["\ud800"]),
        ({"properties": {"k": {"const": "a"}}}, {"k": "\udc00"}),
        ({"items": {"enum": ["a"]}}, ["\ud800"]),
        ({"uniqueItems": True}, ["\udc00", "a"]),
        ({"properties": {"a": {}}}, {"\ud800": 1}),
        ({"additionalProperties": False}, {"\ud800": 1}),
        ({"propertyNames": {"maxLength": 3}}, {"\ud800": 1}),
    ],
)
def test_nested_lone_surrogates_raise_when_a_keyword_reads_them(
    v: Validator, schema: dict[str, Any], instance: Any
) -> None:
    compiled = v.compile(schema)
    with pytest.raises(UnicodeEncodeError):
        v.is_valid(compiled, instance)
    with pytest.raises(UnicodeEncodeError):
        v.errors(compiled, instance)


@pytest.mark.parametrize(
    ("schema", "instance"),
    [
        ({"items": {"maxLength": 0}}, ["\ud800"]),
        ({"items": {"type": "integer"}}, ["\ud800"]),
        ({"maxProperties": 0}, {"\ud800": 1}),
        ({"required": ["a"]}, {"\ud800": 1}),
    ],
)
def test_nested_lone_surrogates_fail_without_errors_otherwise(
    v: Validator, schema: dict[str, Any], instance: Any
) -> None:
    compiled = v.compile(schema)
    assert not v.is_valid(compiled, instance)
    with pytest.raises(UnicodeEncodeError):  # the message prints the instance
        v.errors(compiled, instance)


@pytest.mark.parametrize(
    ("schema", "instance"),
    [
        ({}, ["\ud800"]),
        ({}, {"\ud800": 1}),
        ({"items": {"type": "string"}}, ["\ud800"]),
        ({"items": {"maxLength": 1}}, ["\ud800"]),
    ],
)
def test_nested_lone_surrogates_pass_when_nothing_reads_them(
    v: Validator, schema: dict[str, Any], instance: Any
) -> None:
    compiled = v.compile(schema)
    assert v.is_valid(compiled, instance)
    assert v.errors(compiled, instance) == []


def test_format_and_pattern_checks_raise_on_lone_surrogates(v: Validator) -> None:
    with pytest.raises(UnicodeEncodeError):
        v.satisfies_format("email", "\ud800")
    with pytest.raises(UnicodeEncodeError):
        v.matches_pattern("a", "a\udc00")


@pytest.mark.parametrize(
    "schema",
    [
        {"const": "\ud800"},
        {"enum": ["a", "\udfff"]},
        {"pattern": "\ud800"},
        {"properties": {"\ud800": {}}},
        {"required": ["\udc00"]},
        {"title": "\ud800"},
        {"x-note": "a\udc00b"},
    ],
)
def test_lone_surrogates_in_schemas_are_compile_errors(v: Validator, schema: dict[str, Any]) -> None:
    with pytest.raises(CompileError, match="UTF-8"):
        v.compile(schema)


def test_byte_precheck_flags_every_float_from_2_53_up() -> None:
    floats = [float(k) for k in range(2**53 - 4, 2**53 + 64, 2)]
    floats += [m * 2.0**e for e in range(50, 1024) for m in (1.0, 1.25, 1.5, 1.9999999999999998)]
    floats += [9999999999999998.0, 1e16, 1e16 + 2, 1.7976931348623157e308]
    for x in floats:
        for signed in (x, -x):
            if math.isfinite(signed) and abs(signed) >= 2**53:
                assert _may_hold_inexact_float(orjson.dumps({"k": [signed]})), signed
    # Integers, strings, and floats below 16 digits or with negative exponents are not flagged.
    for plain in (1.0, 2.5, 999999999999999.0, -123456789012345.0, 5e-324, 1e-300):
        assert not _may_hold_inexact_float(orjson.dumps([plain, 12345678901234567890, "1e15"])), plain
    # Floats from 1e15 up to 2**53 are flagged, which only costs the walk: nothing is rewritten.
    small = [float(2**52)]
    assert _may_hold_inexact_float(orjson.dumps(small))
    assert _rs_instance(small, Draft.DRAFT202012) is small
    assert _rs_instance([float(2**53)], Draft.DRAFT202012) == [2**53]
    assert type(_rs_instance([float(2**53)], Draft.DRAFT202012)[0]) is int
    assert type(_rs_instance([float(2**53)], Draft.DRAFT4)[0]) is float
