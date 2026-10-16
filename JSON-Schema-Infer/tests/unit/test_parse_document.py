"""Unit tests for ``parse_document`` in io/ingest.py (task 4.1).

Requirements 1.4 (UTF-8 with an optional BOM, any top-level type), 1.6
(exact big integers through the fallback parser), 1.11 (constructs outside
RFC 8259 fail with both parsers), and 29.2 (orjson first; the fallback only
for documents orjson rejects). Property 2 (parsing fidelity) is task 4.4.
"""

from __future__ import annotations

import json
import pickle
import sys
from typing import Any

import pytest

from jsinfer.io.ingest import PARSE_FAILURE_KINDS, UTF8_BOM, DocumentParseError, parse_document


def strict(value: Any) -> str:
    """A spelling that tells apart what Python's == conflates (True/1, 1/1.0)."""
    return json.dumps(value, ensure_ascii=False)


def parse_error(data: bytes) -> DocumentParseError:
    with pytest.raises(DocumentParseError) as info:
        parse_document(data)
    return info.value


# ---------------------------------------------------------------------------
# Accepted documents (1.4, 29.2)
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("data", "expected"),
    [
        (b'{"a": [1, 2.5, "x", true, false, null], "b": {}}', {"a": [1, 2.5, "x", True, False, None], "b": {}}),
        (b"[]", []),
        (b'"text"', "text"),
        (b"-12", -12),
        (b"0.5", 0.5),
        (b"1e18", 1e18),
        (b"true", True),
        (b"false", False),
        (b"null", None),
        (b" \t\r\n{}\r\n ", {}),
        ('"h\u00e9llo \\u00e9 \\ud83d\\ude00 \U0001f600"'.encode(), "h\u00e9llo \u00e9 \U0001f600 \U0001f600"),
        (b'"\\\\ud800"', "\\ud800"),  # an escaped backslash, not a surrogate escape
        (b'{"e9": "base64e1"}', {"e9": "base64e1"}),
    ],
)
def test_rfc8259_documents_parse_with_orjson(data: bytes, expected: Any) -> None:
    value, parser = parse_document(data)
    assert parser == "orjson"
    assert strict(value) == strict(expected)


def test_one_leading_bom_is_stripped() -> None:
    assert parse_document(UTF8_BOM + b'{"a": 1}') == ({"a": 1}, "orjson")
    assert parse_document(UTF8_BOM + b"18446744073709551616") == (2**64, "stdlib")


def test_non_bytes_input_is_a_type_error() -> None:
    with pytest.raises(TypeError, match="bytes"):
        parse_document('{"a": 1}')  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Integers and the lossy-float fallback (1.6, 29.2)
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("number", [2**64 - 1, 2**63, 2**63 - 1, -(2**63)])
def test_64_bit_integers_stay_exact_with_orjson(number: int) -> None:
    value, parser = parse_document(str(number).encode())
    assert parser == "orjson"
    assert type(value) is int and value == number


@pytest.mark.parametrize("number", [2**64, -(2**63) - 1, 10**30, -(10**30), 10**400, -(3**2000)])
def test_big_integers_fall_back_to_stdlib_and_stay_exact(number: int) -> None:
    value, parser = parse_document(str(number).encode())
    assert parser == "stdlib"
    assert type(value) is int and value == number


def test_nested_big_integer_falls_back_and_keeps_the_rest() -> None:
    data = b'{"id": 123456789012345678901234567890, "xs": [1, 2.5, {"n": -18446744073709551617}], "s": "x"}'
    value, parser = parse_document(data)
    assert parser == "stdlib"
    assert strict(value) == strict(
        {"id": 123456789012345678901234567890, "xs": [1, 2.5, {"n": -18446744073709551617}], "s": "x"}
    )


@pytest.mark.parametrize(
    ("literal", "parser"),
    [
        (b"9.223372036854775e18", "orjson"),  # rounds below 2**63
        (b"9.2233720368547758e18", "stdlib"),  # rounds to 2**63 exactly
        (b"1e19", "stdlib"),
        (b"-1.5e300", "stdlib"),
        (b"1.5", "orjson"),
    ],
)
def test_integral_floats_from_2_to_the_63_take_the_fallback(literal: bytes, parser: str) -> None:
    value, used = parse_document(b"[" + literal + b"]")
    assert used == parser
    assert value == [float(literal)] and type(value[0]) is float


# ---------------------------------------------------------------------------
# Failures (1.11): kind, line, and column
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("data", "kind", "line"),
    [
        # NaN and infinities; the "NaN" inside a string is not the culprit.
        (b"NaN", "constant", 1),
        (b"[1,\n Infinity]", "constant", 2),
        (b'{"a": "NaN",\n "b": -Infinity}', "constant", 2),
        # Comments, trailing commas, single quotes, several top-level values.
        (b"// comment\n1", "syntax", 1),
        (b"1 /* comment */", "syntax", 1),
        (b"[1, 2,]", "syntax", 1),
        (b'{"a": 1,\n}', "syntax", 2),
        (b"{'a': 1}", "syntax", 1),
        (b"'a'", "syntax", 1),
        (b"1\n2", "syntax", 2),
        (b"{}{}", "syntax", 1),
        # Empty documents, a second BOM, a BOM after whitespace, a raw control character.
        (b"", "syntax", 1),
        (b" \n ", "syntax", 2),
        (UTF8_BOM + UTF8_BOM + b"1", "syntax", 1),
        (b" " + UTF8_BOM + b"1", "syntax", 1),
        (b'"\x01"', "syntax", 1),
        # Invalid UTF-8: a bad start byte, an encoded surrogate, a truncated sequence, Latin-1.
        (b'{"a":\n"\xff"}', "encoding", 2),
        (b'"\xed\xa0\x80"', "encoding", 1),
        (b'"\xc3"', "encoding", 1),
        (b'[\n\n"\xe9t\xe9"]', "encoding", 3),
        # Doubles that overflow; the literal inside a string is not the culprit.
        (b"1e400", "number_overflow", 1),
        (b'["1e400",\n -1e400]', "number_overflow", 2),
        (b"1" * 400 + b".0", "number_overflow", 1),
        # Unpaired surrogate escapes, in values and keys.
        (b'"\\ud800"', "surrogate", 1),
        (b'"\\uDC00"', "surrogate", 1),
        (b'"\\ud800x"', "surrogate", 1),
        (b'{"k\\udbff": 1}', "surrogate", 1),
        (b'["ok",\n "\\ud83d\\ude00", "\\ud800\\ud800\\udc00"]', "surrogate", 2),
        # Nesting beyond both parsers' limits.
        (b"[" * 5000 + b"]" * 5000, "depth", 1),
    ],
)
def test_forbidden_constructs_fail_with_kind_and_line(data: bytes, kind: str, line: int) -> None:
    error = parse_error(data)
    assert error.kind == kind
    assert error.line == line
    assert error.reason
    assert error.kind in PARSE_FAILURE_KINDS


def test_syntax_error_reports_the_stdlib_line_and_column() -> None:
    error = parse_error(b"[1,\n 2,\n x]")
    assert (error.kind, error.line, error.column) == ("syntax", 3, 2)
    assert error.reason == "invalid JSON: Expecting value"
    assert str(error) == "invalid JSON: Expecting value (line 3, column 2)"


def test_rejected_literal_is_named_with_its_column() -> None:
    error = parse_error(b'{"x": "-Infinity",\n  "y": -Infinity}')
    assert (error.line, error.column) == (2, 8)
    assert "-Infinity" in error.reason
    error = parse_error(b'[0,\n   1e999]')
    assert (error.kind, error.line, error.column) == ("number_overflow", 2, 4)
    assert "1e999" in error.reason


def test_first_unpaired_surrogate_is_reported() -> None:
    # The first \ud800 has no low half; the second pairs with \udc00.
    error = parse_error(b'["a",  "\\ud800\\ud800\\udc00"]')
    assert (error.line, error.column) == (1, 9)
    assert "\\ud800" in error.reason


def test_invalid_utf8_reports_the_byte_offset_of_the_input() -> None:
    error = parse_error(UTF8_BOM + b'["ok",\n"\xff"]')
    assert (error.kind, error.line, error.column) == ("encoding", 2, None)
    assert "byte offset 11" in error.reason


@pytest.mark.skipif(sys.get_int_max_str_digits() == 0, reason="integer string conversion is unlimited")
def test_integer_beyond_the_conversion_limit_fails() -> None:
    digits = sys.get_int_max_str_digits() + 1
    error = parse_error(b'[1,\n-' + b"9" * digits + b"]")
    assert (error.kind, error.line, error.column) == ("integer_digits", 2, 1)
    assert f"{digits} digits" in error.reason


def test_error_survives_pickling() -> None:
    error = parse_error(b"[1,\n NaN]")
    copy = pickle.loads(pickle.dumps(error))
    assert type(copy) is DocumentParseError
    assert (copy.kind, copy.reason, copy.line, copy.column) == (error.kind, error.reason, error.line, error.column)
    assert isinstance(copy, ValueError)


def test_error_validates_its_fields() -> None:
    with pytest.raises(ValueError, match="kind"):
        DocumentParseError("other", "x")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="reason"):
        DocumentParseError("syntax", "")
    with pytest.raises(ValueError, match="line"):
        DocumentParseError("syntax", "x", 0)
    assert str(DocumentParseError("depth", "too deep")) == "too deep"
