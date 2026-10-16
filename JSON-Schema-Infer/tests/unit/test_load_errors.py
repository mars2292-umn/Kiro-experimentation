"""Unit tests for the Schema_Loader's error paths (task 6.8, Requirement 2.12).

A document that is not valid JSON, whose root is neither an object nor a
boolean, or that fails the meta-schema of its declared or assumed draft makes
:meth:`SchemaLoader.load` raise :class:`SchemaLoadError` naming the failed
check (``json``, ``root``, or ``meta``) and return no model. JSON syntax
errors carry their 1-based line and column; invalid UTF-8 carries the line and
a byte offset in the reason, since a character column is undefined there.
"""

from __future__ import annotations

import json
import pickle
from typing import Any

import pytest

from jsinfer.io.load import SCHEMA_LOAD_CHECKS, SchemaLoader, SchemaLoadError

D4 = "http://json-schema.org/draft-04/schema#"
D6 = "http://json-schema.org/draft-06/schema#"
D7 = "http://json-schema.org/draft-07/schema#"
D2019 = "https://json-schema.org/draft/2019-09/schema"
D2020 = "https://json-schema.org/draft/2020-12/schema"
BOM = b"\xef\xbb\xbf"
NAN, INF = float("nan"), float("inf")


@pytest.fixture(scope="module")
def loader() -> SchemaLoader:
    return SchemaLoader()


def load_error(loader: SchemaLoader, source: Any) -> SchemaLoadError:
    """Load ``source``, expecting a SchemaLoadError and no model."""
    model = None
    with pytest.raises(SchemaLoadError) as info:
        model = loader.load(source)
    assert model is None
    return info.value


def as_source(text: str, form: str) -> bytes | str:
    return text.encode("utf-8") if form == "bytes" else text


# ---------------------------------------------------------------------------
# Check "json": documents that are not valid JSON
# ---------------------------------------------------------------------------

# (text, line, column, reason fragment); each is loaded as bytes and as str.
NOT_JSON = [
    pytest.param('{"type": "string",}', 1, 19, "Expecting property name", id="trailing-comma"),
    pytest.param('{\n  "type": "string"\n  "minLength": 1\n}', 3, 3, "Expecting ',' delimiter", id="multi-line"),
    pytest.param('{\r\n  "a": 1,\r\n}', 3, 1, "Expecting property name", id="crlf"),
    pytest.param('{\n\t"a":\t[1,\n\t\t2,,]}', 3, 5, "Expecting value", id="tabs"),
    pytest.param('{"type": "string"', 1, 18, "Expecting ',' delimiter", id="unterminated"),
    pytest.param("", 1, 1, "Expecting value", id="empty"),
    pytest.param("{'a': 1}", 1, 2, "Expecting property name", id="single-quotes"),
    pytest.param('{"a": "line\nbreak"}', 1, 12, "Invalid control character", id="raw-control-character"),
    pytest.param("{} // comment", 1, 4, "Extra data", id="line-comment"),
    pytest.param("/* comment */ {}", 1, 1, "Expecting value", id="block-comment"),
    pytest.param("{} {}", 1, 4, "Extra data", id="two-values"),
    pytest.param('{"a": 1}\n{"b": 2}\n', 2, 1, "Extra data", id="two-lines-of-values"),
    pytest.param('{"minimum": NaN}', 1, 13, "NaN is not a JSON value", id="nan"),
    pytest.param('{"a": Infinity}', 1, 7, "Infinity is not a JSON value", id="infinity"),
    pytest.param('{\n  "maximum": -Infinity\n}', 2, 14, "-Infinity is not a JSON value", id="minus-infinity"),
    pytest.param('{"a": [1, NaN]}', 1, 11, "NaN is not a JSON value", id="nan-in-array"),
    pytest.param('{"a": 1e999}', 1, 7, "outside the range of an IEEE 754 double", id="overflowing-double"),
    pytest.param('{"a": "\\ud800"}', 1, 8, "unpaired UTF-16 surrogate escape \\ud800", id="lone-high-escape"),
    pytest.param('{"a": "\\udc00\\ud800"}', 1, 8, "unpaired UTF-16 surrogate escape \\udc00", id="reversed-pair"),
]


@pytest.mark.parametrize("form", ["bytes", "str"])
@pytest.mark.parametrize(("text", "line", "col", "fragment"), NOT_JSON)
def test_non_json_reports_line_and_column(
    loader: SchemaLoader, text: str, line: int, col: int, fragment: str, form: str
) -> None:
    error = load_error(loader, as_source(text, form))
    assert error.check == "json"
    assert (error.line, error.col) == (line, col)
    assert fragment in error.reason
    assert error.errors == ()
    assert str(error) == f"{error.reason} (line {line}, column {col})"


@pytest.mark.parametrize(
    ("text", "col"),
    [('{"title": "éé", }', 17), ('{"title": "😀", }', 16)],
    ids=["two-byte", "four-byte"],
)
@pytest.mark.parametrize("form", ["bytes", "str"])
def test_columns_count_characters_not_bytes(loader: SchemaLoader, text: str, col: int, form: str) -> None:
    error = load_error(loader, as_source(text, form))
    assert (error.check, error.line, error.col) == ("json", 1, col)


@pytest.mark.parametrize("text", ['{"a": }', '{\n  "a": NaN\n}', '[1,', '{"a": "\\udfff"}'])
def test_bom_is_stripped_and_not_counted(loader: SchemaLoader, text: str) -> None:
    plain = load_error(loader, text.encode("utf-8"))
    for source in (BOM + text.encode("utf-8"), "\ufeff" + text):
        error = load_error(loader, source)
        assert (error.check, error.reason, error.line, error.col) == ("json", plain.reason, plain.line, plain.col)


@pytest.mark.parametrize(
    ("source", "fragment"),
    [(BOM, "Expecting value"), (BOM + BOM + b"{}", "more than one leading byte order mark")],
    ids=["bom-only", "two-boms"],
)
def test_bom_alone_or_doubled_is_a_syntax_error(loader: SchemaLoader, source: bytes, fragment: str) -> None:
    error = load_error(loader, source)
    assert (error.check, error.line, error.col) == ("json", 1, 1)
    assert fragment in error.reason


@pytest.mark.parametrize(
    ("data", "line", "offset"),
    [
        (b'{"a": "\xff"}', 1, 7),
        (b'{\n"a": "x",\n"b": "\xc3"}', 3, 18),
        (BOM + b'{"a": "\xff"}', 1, 10),  # the offset counts the bytes passed in, BOM included
    ],
    ids=["first-line", "third-line", "after-bom"],
)
def test_invalid_utf8_reports_line_and_byte_offset(loader: SchemaLoader, data: bytes, line: int, offset: int) -> None:
    error = load_error(loader, data)
    assert (error.check, error.line, error.col) == ("json", line, None)
    assert f"invalid UTF-8 at byte offset {offset}" in error.reason
    assert str(error) == f"{error.reason} (line {line})"


@pytest.mark.parametrize(
    ("text", "line", "col", "unit"),
    [
        ('{"a": "\ud800"}', 1, 8, "U+D800"),
        ('{\n "type":\n  "x\udc00"}', 3, 5, "U+DC00"),
        ('["\ud83d\ude00"]', 1, 3, "U+D83D"),  # a pair of surrogate code points is not valid text
    ],
    ids=["high", "low-on-third-line", "surrogate-code-point-pair"],
)
def test_unpaired_surrogates_in_text_are_located(
    loader: SchemaLoader, text: str, line: int, col: int, unit: str
) -> None:
    error = load_error(loader, text)
    assert (error.check, error.line, error.col) == ("json", line, col)
    assert error.reason == f"invalid JSON text: unpaired surrogate {unit}"


def test_nesting_too_deep_to_parse_is_a_json_error(loader: SchemaLoader) -> None:
    depth = 100_000
    error = load_error(loader, b'{"not":' * depth + b"{}" + b"}" * depth)
    assert error.check == "json"
    assert "nesting" in error.reason
    assert error.line == 1 and error.col is not None


# ---------------------------------------------------------------------------
# Check "json": parsed sources that are not JSON values
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("value", "fragment"),
    [
        (NAN, "the number nan is not finite at (root)"),
        ({"minimum": NAN}, "the number nan is not finite at /minimum"),
        ({"enum": [1, INF]}, "the number inf is not finite at /enum/1"),
        ({"a/b~c": {"x": -INF}}, "the number -inf is not finite at /a~1b~0c/x"),
        ({1: "x"}, "an object key is int, not str at (root)"),
        ({"properties": {"p": {None: {}}}}, "an object key is NoneType, not str at /properties/p"),
        ({(1, 2): {}}, "an object key is tuple, not str at (root)"),
        ({"title": "\ud800"}, "a string holds an unpaired surrogate code point at /title"),
        ({"\udc00": {}}, "an object key holds an unpaired surrogate code point at (root)"),
        ({"enum": (1, 2)}, "tuple is not a JSON type at /enum"),
        ({"enum": [{1, 2}]}, "set is not a JSON type at /enum/0"),
        ({"const": b"x"}, "bytes is not a JSON type at /const"),
    ],
    ids=[
        "nan-root",
        "nan",
        "infinity-in-array",
        "escaped-pointer",
        "int-key",
        "nested-none-key",
        "tuple-key",
        "surrogate-value",
        "surrogate-key",
        "tuple",
        "set",
        "bytes",
    ],
)
def test_parsed_non_json_values_are_json_errors(loader: SchemaLoader, value: Any, fragment: str) -> None:
    error = load_error(loader, value)
    assert error.check == "json"
    assert error.reason == f"the schema is not a JSON value: {fragment}"
    assert (error.line, error.col, error.errors) == (None, None, ())
    assert str(error) == error.reason


# ---------------------------------------------------------------------------
# Check "root": roots that are neither objects nor booleans
# ---------------------------------------------------------------------------

ROOT_REASON = "the schema root is {}; a schema must be a JSON object or a boolean"


@pytest.mark.parametrize("form", ["bytes", "str"])
@pytest.mark.parametrize(
    ("text", "kind"),
    [
        ("null", "null"),
        ("0", "a number"),
        ("-1.5e3", "a number"),
        ("123456789012345678901234567890", "a number"),
        ('"schema"', "a string"),
        (' \n "s" \n', "a string"),
        ("[]", "an array"),
        ('[{"type": "string"}]', "an array"),
        ("\ufeff[]", "an array"),
    ],
)
def test_text_roots_of_other_kinds_are_rejected(loader: SchemaLoader, text: str, kind: str, form: str) -> None:
    error = load_error(loader, as_source(text, form))
    assert error.check == "root"
    assert error.reason == ROOT_REASON.format(kind)
    assert (error.line, error.col, error.errors) == (None, None, ())


# A parsed str is read as JSON text, so a string root is reachable only as text (above).
@pytest.mark.parametrize(
    ("value", "kind"),
    [
        (None, "null"),
        (0, "a number"),
        (1.5, "a number"),
        (-0.0, "a number"),
        (2**70, "a number"),
        ([], "an array"),
        ([{"type": "string"}], "an array"),
    ],
)
def test_parsed_roots_of_other_kinds_are_rejected(loader: SchemaLoader, value: Any, kind: str) -> None:
    error = load_error(loader, value)
    assert error.check == "root"
    assert error.reason == ROOT_REASON.format(kind)
    assert (error.line, error.col, error.errors) == (None, None, ())


@pytest.mark.parametrize(("source", "value"), [(b"true", True), (" false\n", False), (BOM + b"false", False)])
def test_boolean_text_roots_are_accepted(loader: SchemaLoader, source: bytes | str, value: bool) -> None:
    model = loader.load(source)
    assert model.root.boolean is value


# ---------------------------------------------------------------------------
# Check "meta": documents that fail their draft's meta-schema
# ---------------------------------------------------------------------------


def pointers(error: SchemaLoadError) -> list[str]:
    return [entry.split(": ", 1)[0] for entry in error.errors]


@pytest.mark.parametrize(
    ("schema", "draft", "how", "expected"),
    [
        pytest.param({"type": 5}, "draft 2020-12", "assumed", ["/type"], id="type-number"),
        pytest.param({"minLength": -1}, "draft 2020-12", "assumed", ["/minLength"], id="negative-length"),
        pytest.param(
            {"exclusiveMinimum": True, "minimum": 0}, "draft 2020-12", "assumed", ["/exclusiveMinimum"],
            id="draft4-boolean-bound",
        ),
        pytest.param({"items": [{"type": "string"}]}, "draft 2020-12", "assumed", ["/items"], id="array-items"),
        pytest.param(
            {"properties": {"a/b": {"type": 5}}}, "draft 2020-12", "assumed", ["/properties/a~1b/type"],
            id="nested-escaped-pointer",
        ),
        pytest.param({"pattern": "["}, "draft 2020-12", "assumed", ["/pattern"], id="format-asserted"),
        pytest.param(
            {"$schema": "https://example.com/meta", "type": 5}, "draft 2020-12", "assumed", ["/type"],
            id="unrecognized-dialect",
        ),
        pytest.param({"$schema": 5}, "draft 2020-12", "assumed", ["/$schema"], id="non-string-dialect"),
        pytest.param(
            {"$schema": D2020, "$defs": {"a": {"minItems": 1.5}}}, "draft 2020-12", "declared",
            ["/$defs/a/minItems"], id="declared-2020-12",
        ),
        pytest.param({"$schema": D2019, "type": 5}, "draft 2019-09", "declared", ["/type"], id="declared-2019-09"),
        pytest.param(
            {"$schema": D7, "exclusiveMinimum": True}, "draft-07", "declared", ["/exclusiveMinimum"],
            id="draft7-boolean-bound",
        ),
        pytest.param({"$schema": D7, "if": 5}, "draft-07", "declared", ["/if"], id="draft7-if"),
        pytest.param(
            {"$schema": D7, "$ref": "#/definitions/a", "type": 3, "definitions": {"a": {}}}, "draft-07", "declared",
            ["/type"], id="draft7-ref-sibling",  # $ref hides its siblings, but the meta-schema still checks them
        ),
        pytest.param(
            {"$schema": D6, "exclusiveMaximum": False}, "draft-06", "declared", ["/exclusiveMaximum"],
            id="draft6-boolean-bound",
        ),
        pytest.param({"$schema": D4, "required": []}, "draft-04", "declared", ["/required"], id="draft4-required"),
        pytest.param(
            {"$schema": D4, "properties": {"a": True}}, "draft-04", "declared", ["/properties/a"],
            id="draft4-boolean-subschema",
        ),
    ],
)
def test_meta_schema_failures_name_the_draft_and_every_error(
    loader: SchemaLoader, schema: dict[str, Any], draft: str, how: str, expected: list[str]
) -> None:
    error = load_error(loader, schema)
    assert error.check == "meta"
    assert pointers(error) == expected
    assert error.reason == f"the schema is not valid against the {draft} meta-schema ({how}): {error.errors[0]}"
    assert (error.line, error.col) == (None, None)
    assert str(error) == error.reason


@pytest.mark.parametrize(
    ("schema", "how", "expected"),
    [
        ({"type": 5, "minLength": -1}, "assumed", ["/minLength", "/type"]),
        ({"$schema": D4, "exclusiveMinimum": 5}, "declared", ["(root)", "/exclusiveMinimum"]),
    ],
    ids=["two-keywords", "draft4-dependency-and-type"],
)
def test_every_meta_error_is_kept_in_pointer_order(
    loader: SchemaLoader, schema: dict[str, Any], how: str, expected: list[str]
) -> None:
    error = load_error(loader, schema)
    assert error.check == "meta"
    assert pointers(error) == expected
    assert error.reason.endswith(f"({how}): {error.errors[0]} (and 1 more)")


@pytest.mark.parametrize(
    ("body", "failing", "passing"),
    [
        ({"exclusiveMinimum": True, "minimum": 0}, None, D4),
        ({"items": [{"type": "string"}]}, None, D7),
        ({"required": []}, D4, None),
        ({"properties": {"a": True}}, D4, D6),
    ],
    ids=["draft4-bound", "array-items", "empty-required", "boolean-subschema"],
)
def test_meta_check_uses_the_declared_or_assumed_draft(
    loader: SchemaLoader, body: dict[str, Any], failing: str | None, passing: str | None
) -> None:
    def declare(uri: str | None) -> dict[str, Any]:
        return body if uri is None else {"$schema": uri, **body}

    assert load_error(loader, declare(failing)).check == "meta"
    loader.load(declare(passing))


def test_meta_errors_do_not_depend_on_the_source_form(loader: SchemaLoader) -> None:
    schema = {"$schema": D7, "type": "strin", "properties": {"p": {"minimum": "0"}}}
    text = json.dumps(schema)
    first, *others = (load_error(loader, source) for source in (schema, text, text.encode("utf-8")))
    assert first.check == "meta"
    assert pointers(first) == ["/properties/p/minimum", "/type"]
    for error in others:
        assert (error.check, error.reason, error.errors) == (first.check, first.reason, first.errors)


# ---------------------------------------------------------------------------
# Check order: json, then root, then meta
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("source", "check"),
    [
        (b"[1,", "json"),
        (b"[NaN]", "json"),
        ([NAN], "json"),
        (b'"\\ud800"', "json"),
        (b'{"type": 5,}', "json"),
        (b'{"type": 5, "a": NaN}', "json"),
        ({"type": 5, "x": NAN}, "json"),
        (b'[{"type": 5}]', "root"),
        ('[{"type": 5}]', "root"),
        ([{"type": 5}], "root"),
        (b'{"type": 5}', "meta"),
    ],
)
def test_the_first_failing_check_is_reported(loader: SchemaLoader, source: Any, check: str) -> None:
    assert load_error(loader, source).check == check


def test_loader_still_loads_after_errors(loader: SchemaLoader) -> None:
    for source in (b"{", b"[]", {"type": 5}):
        load_error(loader, source)
    assert loader.load({"type": "string"}).root.kw == {"type": "string"}


# ---------------------------------------------------------------------------
# SchemaLoadError
# ---------------------------------------------------------------------------


def test_checks_are_json_root_meta() -> None:
    assert SCHEMA_LOAD_CHECKS == ("json", "root", "meta")
    assert issubclass(SchemaLoadError, ValueError)


@pytest.mark.parametrize("source", [b'{\n  "a": ]}', b"[]", {"type": 5, "minLength": -1}], ids=SCHEMA_LOAD_CHECKS)
def test_errors_survive_pickling(loader: SchemaLoader, source: Any) -> None:
    error = load_error(loader, source)
    copy = pickle.loads(pickle.dumps(error))
    assert type(copy) is SchemaLoadError
    assert copy.args == error.args
    assert (copy.check, copy.reason, copy.line, copy.col, copy.errors) == (
        error.check,
        error.reason,
        error.line,
        error.col,
        error.errors,
    )
    assert str(copy) == str(error)


@pytest.mark.parametrize(
    ("line", "col", "text"),
    [(None, None, "bad"), (3, None, "bad (line 3)"), (3, 7, "bad (line 3, column 7)")],
)
def test_str_appends_the_location(line: int | None, col: int | None, text: str) -> None:
    assert str(SchemaLoadError("json", "bad", line, col)) == text


@pytest.mark.parametrize(
    ("kwargs", "exc"),
    [
        ({"check": "syntax"}, ValueError),
        ({"reason": ""}, ValueError),
        ({"line": 0}, ValueError),
        ({"col": True}, ValueError),
        ({"line": 1.0}, ValueError),
        ({"errors": [1]}, TypeError),
    ],
    ids=["unknown-check", "empty-reason", "line-zero", "bool-column", "float-line", "non-str-error"],
)
def test_malformed_errors_are_refused(kwargs: dict[str, Any], exc: type[Exception]) -> None:
    arguments: dict[str, Any] = {"check": "json", "reason": "bad", **kwargs}
    with pytest.raises(exc):
        SchemaLoadError(**arguments)
