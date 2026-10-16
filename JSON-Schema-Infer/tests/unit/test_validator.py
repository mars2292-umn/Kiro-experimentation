"""Unit tests for io/validator.py (task 5.1)."""

from __future__ import annotations

import math
import pickle
from typing import Any

import jsonschema_rs
import pytest

from jsinfer.io.validator import CompileError, Compiled, Draft, ErrorInfo, Validator, dialect_of


@pytest.fixture
def v() -> Validator:
    return Validator()


def locations(validator: Validator, schema: dict[str, Any] | bool, instance: Any) -> list[tuple[str, str]]:
    compiled = validator.compile(schema)
    return [(e.keyword_location, e.instance_location) for e in validator.errors(compiled, instance)]


# ---------------------------------------------------------------------------
# Dialects and draft selection
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("declared", "draft"),
    [
        ("http://json-schema.org/draft-04/schema#", Draft.DRAFT4),
        ("https://json-schema.org/draft-04/schema", Draft.DRAFT4),
        ("http://json-schema.org/draft-06/schema", Draft.DRAFT6),
        ("https://json-schema.org/draft-07/schema#", Draft.DRAFT7),
        ("https://json-schema.org/draft/2019-09/schema", Draft.DRAFT201909),
        ("http://json-schema.org/draft/2019-09/schema#", Draft.DRAFT201909),
        ("https://json-schema.org/draft/2020-12/schema", Draft.DRAFT202012),
        ("http://json-schema.org/draft/2020-12/schema#", Draft.DRAFT202012),
    ],
)
def test_dialect_of_recognizes_declared_drafts(declared: str, draft: Draft) -> None:
    assert dialect_of({"$schema": declared}) == (draft, False)


@pytest.mark.parametrize(
    "schema",
    [{}, True, False, {"$schema": "https://example.com/custom"}, {"$schema": 7}, {"$schema": "draft-07"}],
)
def test_dialect_of_assumes_2020_12(schema: dict[str, Any] | bool) -> None:
    assert dialect_of(schema) == (Draft.DRAFT202012, True)


def test_compile_uses_draft04_semantics_when_declared(v: Validator) -> None:
    d4 = {"$schema": Draft.DRAFT4.uri, "minimum": 1, "exclusiveMinimum": True}
    compiled = v.compile(d4)
    assert (compiled.draft, compiled.dialect_assumed) == (Draft.DRAFT4, False)
    assert not v.is_valid(compiled, 1)
    assert v.is_valid(compiled, 2)
    # The same keywords are not a valid 2020-12 schema.
    with pytest.raises(CompileError):
        v.compile({"minimum": 1, "exclusiveMinimum": True})


def test_compile_ignores_ref_siblings_only_in_drafts_up_to_07(v: Validator) -> None:
    body = {"definitions": {"n": {"type": "integer"}}, "properties": {"p": {"$ref": "#/definitions/n", "minimum": 5}}}
    d7 = v.compile({"$schema": Draft.DRAFT7.uri, **body})
    d2020 = v.compile({"$schema": Draft.DRAFT202012.uri, **body})
    assert v.is_valid(d7, {"p": 1})
    assert not v.is_valid(d2020, {"p": 1})


def test_compile_with_unknown_dialect_uses_2020_12(v: Validator) -> None:
    compiled = v.compile({"$schema": "https://example.com/custom", "prefixItems": [{"type": "integer"}]})
    assert (compiled.draft, compiled.dialect_assumed) == (Draft.DRAFT202012, True)
    assert not v.is_valid(compiled, ["x"])
    assert v.is_valid(compiled, [1, "x"])


def test_formats_are_asserted_and_lookaround_patterns_compile(v: Validator) -> None:
    assert not v.is_valid(v.compile({"format": "date"}), "2024-13-45")
    assert v.is_valid(v.compile({"format": "date"}), "2024-01-02")
    lookbehind = v.compile({"pattern": "(?<=a)b"})
    assert v.is_valid(lookbehind, "ab")
    assert not v.is_valid(lookbehind, "cb")


def test_boolean_schemas(v: Validator) -> None:
    assert v.is_valid(v.compile(True), {"anything": [1]})
    assert not v.is_valid(v.compile(False), None)
    assert locations(v, False, 1) == [("", "")]


# ---------------------------------------------------------------------------
# Caching
# ---------------------------------------------------------------------------


def test_compile_caches_by_canonical_bytes(v: Validator) -> None:
    a = v.compile({"type": "integer", "minimum": 1.0, "enum": [1, 2]})
    b = v.compile({"enum": [1, 2], "minimum": 1, "type": "integer"})
    c = v.compile({"type": "integer", "minimum": 2})
    assert a is b
    assert a is not c
    assert a.digest == b.digest != c.digest
    assert len(a.digest) == 64


def test_compile_cache_is_bounded() -> None:
    small = Validator(max_compiled=1)
    first = small.compile({"type": "integer"})
    small.compile({"type": "string"})
    again = small.compile({"type": "integer"})
    assert again is not first
    assert again.digest == first.digest


def test_compiled_keeps_a_private_copy_of_the_schema(v: Validator) -> None:
    schema = {"$defs": {"n": {"type": "integer"}}, "properties": {"p": {"$ref": "#/$defs/n"}}}
    compiled = v.compile(schema)
    schema["$defs"] = {}
    assert compiled.schema["$defs"] == {"n": {"type": "integer"}}
    assert [e.keyword_location for e in v.errors(compiled, {"p": "x"})] == ["/$defs/n/type"]


@pytest.mark.parametrize("bad", [{"max_compiled": 0}, {"max_values": -1}, {"max_values": True}, {"max_compiled": 1.5}])
def test_cache_bounds_are_checked(bad: dict[str, Any]) -> None:
    with pytest.raises(ValueError):
        Validator(**bad)


# ---------------------------------------------------------------------------
# CompileError
# ---------------------------------------------------------------------------


def test_compile_error_names_the_schema_and_cause(v: Validator) -> None:
    with pytest.raises(CompileError) as info:
        v.compile({"pattern": "(?<"}, name="corpus/foo.json")
    error = info.value
    assert error.schema_id == "corpus/foo.json"
    assert "regex" in error.cause and "/pattern" in error.cause
    assert str(error) == f"cannot compile schema corpus/foo.json: {error.cause}"
    assert error.digest is not None and len(error.digest) == 64


def test_compile_error_falls_back_to_id_then_digest(v: Validator) -> None:
    with pytest.raises(CompileError) as by_id:
        v.compile({"$id": "https://example.com/s.json", "type": 5})
    assert by_id.value.schema_id == "https://example.com/s.json"
    with pytest.raises(CompileError) as by_d4_id:
        v.compile({"$schema": Draft.DRAFT4.uri, "id": "https://example.com/d4.json", "type": 5})
    assert by_d4_id.value.schema_id == "https://example.com/d4.json"
    with pytest.raises(CompileError) as by_digest:
        v.compile({"$ref": "#/$defs/missing"})
    assert by_digest.value.schema_id == f"sha256:{by_digest.value.digest[:12]}"
    assert "/$defs/missing" in by_digest.value.cause


def test_compile_failures_are_cached_and_still_raise(v: Validator) -> None:
    with pytest.raises(CompileError) as first:
        v.compile({"type": 5}, name="first")
    with pytest.raises(CompileError) as second:
        v.compile({"type": 5}, name="second")
    assert second.value.schema_id == "second"
    assert second.value.cause == first.value.cause


def test_remote_refs_are_never_fetched(v: Validator) -> None:
    with pytest.raises(CompileError) as info:
        v.compile({"$ref": "https://example.invalid/other.json"})
    assert "example.invalid" in info.value.cause


def test_non_json_schemas(v: Validator) -> None:
    with pytest.raises(CompileError) as info:
        v.compile({"minimum": math.nan}, name="nan")
    assert info.value.schema_id == "nan" and info.value.digest is None
    with pytest.raises(TypeError):
        v.compile([{"type": "integer"}])  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        v.compile({}, name="")


def test_compile_error_pickles() -> None:
    error = CompileError("s", "cause", "ab" * 32)
    clone = pickle.loads(pickle.dumps(error))
    assert (clone.schema_id, clone.cause, clone.digest, str(clone)) == ("s", "cause", "ab" * 32, str(error))


# ---------------------------------------------------------------------------
# errors(): absolute keyword locations
# ---------------------------------------------------------------------------


def test_errors_is_empty_exactly_when_valid(v: Validator) -> None:
    compiled = v.compile({"type": "object", "required": ["a"], "properties": {"a": {"type": "integer"}}})
    for instance in [{"a": 1}, {"a": "x"}, {}, [], {"a": 1, "b": 2}]:
        assert (v.errors(compiled, instance) == []) == v.is_valid(compiled, instance)


def test_errors_follow_defs_chains(v: Validator) -> None:
    schema = {
        "$defs": {
            "A": {"type": "object", "properties": {"x": {"$ref": "#/$defs/B"}}},
            "B": {"type": "integer", "minimum": 3},
        },
        "properties": {"a": {"$ref": "#/$defs/A"}},
    }
    assert locations(v, schema, {"a": {"x": 1}}) == [("/$defs/B/minimum", "/a/x")]
    assert locations(v, schema, {"a": {"x": "q"}}) == [("/$defs/B/type", "/a/x")]


def test_errors_follow_recursive_refs(v: Validator) -> None:
    to_root = {"type": "object", "properties": {"child": {"$ref": "#"}, "v": {"type": "integer"}}}
    assert locations(v, to_root, {"child": {"child": {"v": "x"}}}) == [("/properties/v/type", "/child/child/v")]
    self_def = {
        "$defs": {"node": {"properties": {"next": {"$ref": "#/$defs/node"}, "v": {"type": "string"}}}},
        "$ref": "#/$defs/node",
    }
    assert locations(v, self_def, {"next": {"next": {"v": 1}}, "v": 2}) == [
        ("/$defs/node/properties/v/type", "/next/next/v"),
        ("/$defs/node/properties/v/type", "/v"),
    ]


def test_errors_follow_ref_to_ref_and_escaped_names(v: Validator) -> None:
    schema = {
        "$defs": {"A": {"$ref": "#/$defs/B"}, "B": {"type": "integer"}, "a/b": {"const": 1}, "a b": {"const": 2}},
        "properties": {
            "chain": {"$ref": "#/$defs/A"},
            "slash": {"$ref": "#/$defs/a~1b"},
            "space": {"$ref": "#/$defs/a%20b"},
            "x/y": {"type": "null"},
        },
    }
    instance = {"chain": "s", "slash": 0, "space": 0, "x/y": 0}
    assert locations(v, schema, instance) == [
        ("/$defs/B/type", "/chain"),
        ("/$defs/a b/const", "/space"),
        ("/$defs/a~1b/const", "/slash"),
        ("/properties/x~1y/type", "/x~1y"),
    ]


def test_errors_follow_refs_in_array_and_applicator_keywords(v: Validator) -> None:
    schema = {
        "$defs": {"int": {"type": "integer"}},
        "prefixItems": [{"$ref": "#/$defs/int"}, {"type": "string"}],
        "items": {"$ref": "#/$defs/int"},
        "allOf": [{"minItems": 4}],
    }
    assert locations(v, schema, ["a", 1, "b"]) == [
        ("/$defs/int/type", "/0"),
        ("/$defs/int/type", "/2"),
        ("/allOf/0/minItems", ""),
        ("/prefixItems/1/type", "/1"),
    ]


def test_errors_resolve_against_embedded_resources(v: Validator) -> None:
    # jsonschema-rs's schema_path is relative to the embedded resource here.
    schema = {
        "$id": "https://example.com/root.json",
        "$defs": {
            "B": {
                "$id": "b.json",
                "$defs": {"C": {"type": "integer"}},
                "properties": {"z": {"$ref": "#/$defs/C"}},
            }
        },
        "properties": {"p": {"$ref": "b.json"}, "q": {"$ref": "https://example.com/b.json#/$defs/C"}},
    }
    assert locations(v, schema, {"p": {"z": "x"}, "q": "y"}) == [
        ("/$defs/B/$defs/C/type", "/p/z"),
        ("/$defs/B/$defs/C/type", "/q"),
    ]
    rs_paths = [e.schema_path for e in jsonschema_rs.validator_for(schema).iter_errors({"p": {"z": "x"}})]
    assert rs_paths == [["$defs", "C", "type"]]


def test_errors_resolve_draft04_ids_and_draft07_anchor_ids(v: Validator) -> None:
    d4 = {
        "$schema": Draft.DRAFT4.uri,
        "id": "http://example.com/r.json",
        "definitions": {
            "B": {"id": "b.json", "definitions": {"C": {"type": "integer"}}, "properties": {"z": {"$ref": "#/definitions/C"}}}
        },
        "properties": {"p": {"$ref": "b.json"}},
    }
    assert locations(v, d4, {"p": {"z": "x"}}) == [("/definitions/B/definitions/C/type", "/p/z")]
    d7 = {
        "$schema": Draft.DRAFT7.uri,
        "definitions": {"B": {"$id": "#bee", "type": "integer"}},
        "properties": {"p": {"$ref": "#bee"}},
    }
    assert locations(v, d7, {"p": "x"}) == [("/definitions/B/type", "/p")]


def test_errors_resolve_anchors(v: Validator) -> None:
    schema = {
        "$id": "https://example.com/r",
        "$defs": {"B": {"$anchor": "bee", "type": "integer"}, "E": {"$id": "e", "$anchor": "x", "type": "string"}},
        "properties": {"p": {"$ref": "#bee"}, "q": {"$ref": "e#x"}},
    }
    assert locations(v, schema, {"p": "s", "q": 1}) == [("/$defs/B/type", "/p"), ("/$defs/E/type", "/q")]


@pytest.mark.parametrize(
    ("schema", "instance"),
    [
        ({"properties": {"a": False}, "additionalProperties": False, "required": ["z"]}, {"a": 1, "b": 2}),
        ({"anyOf": [{"type": "string"}, {"$ref": "#/$defs/B"}], "$defs": {"B": {"type": "integer"}}}, 1.5),
        ({"if": {"type": "integer"}, "then": {"$ref": "#/$defs/S"}, "$defs": {"S": {"minimum": 10}}}, 3),
        ({"patternProperties": {"^a": {"$ref": "#/$defs/B"}}, "$defs": {"B": {"type": "integer"}}}, {"abc": "x"}),
        ({"propertyNames": {"$ref": "#/$defs/S"}, "$defs": {"S": {"maxLength": 2}}}, {"abc": 1}),
        ({"dependentRequired": {"a": ["b"]}, "properties": {"a": {"enum": [2]}}}, {"a": 1}),
        ({"items": {"$ref": "#/$defs/O"}, "$defs": {"O": {"properties": {"k": {"$ref": "#/$defs/O"}}, "maxProperties": 1}}},
         [{"k": {"k": {}, "j": 1}}]),
        ({"$schema": Draft.DRAFT7.uri, "definitions": {"B": {"type": "integer"}}, "items": [{"$ref": "#/definitions/B"}]}, ["x"]),
    ],
)
def test_errors_agree_with_schema_path_without_embedded_resources(
    v: Validator, schema: dict[str, Any], instance: Any
) -> None:
    rs = jsonschema_rs.validator_for(schema, validate_formats=True)
    expected = sorted(
        ("".join("/" + str(t).replace("~", "~0").replace("/", "~1") for t in e.schema_path), e.kind.name)
        for e in rs.iter_errors(instance)
    )
    got = sorted((e.keyword_location, e.keyword) for e in v.errors(v.compile(schema), instance))
    assert got == expected and got


def test_dynamic_refs_fall_back_to_schema_path(v: Validator) -> None:
    schema = {
        "$dynamicAnchor": "node",
        "properties": {"c": {"$dynamicRef": "#node"}, "v": {"type": "integer"}},
    }
    assert locations(v, schema, {"c": {"v": "x"}}) == [("/properties/v/type", "/c/v")]


def test_errors_are_sorted_error_infos(v: Validator) -> None:
    compiled = v.compile({"properties": {"b": {"type": "integer"}, "a": {"type": "integer"}}, "minProperties": 3})
    found = v.errors(compiled, {"b": "x", "a": "y"})
    assert found == sorted(found)
    assert all(isinstance(e, ErrorInfo) for e in found)
    assert [e.keyword for e in found] == ["minProperties", "type", "type"]
    assert found[1] == ErrorInfo("/properties/a/type", "/a", "type", '"y" is not of type "integer"')


def test_validate_rejects_non_compiled(v: Validator) -> None:
    with pytest.raises(TypeError):
        v.is_valid({"type": "integer"}, 1)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        v.errors({"type": "integer"}, 1)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# meta_errors
# ---------------------------------------------------------------------------


def test_meta_errors_empty_for_valid_schemas(v: Validator) -> None:
    for draft in Draft:
        assert v.meta_errors({"type": "object", "properties": {"a": {"type": "integer"}}}, draft) == []


def test_meta_errors_lists_every_error_with_its_pointer(v: Validator) -> None:
    found = v.meta_errors({"type": 5, "properties": {"a": {"minLength": -1}}}, Draft.DRAFT202012)
    assert len(found) == 2
    assert found == sorted(found)
    assert found[0].startswith("/properties/a/minLength: ")
    assert found[1].startswith("/type: ")


def test_meta_errors_use_the_given_draft(v: Validator) -> None:
    d4_style = {"minimum": 1, "exclusiveMinimum": True}
    assert v.meta_errors(d4_style, Draft.DRAFT4) == []
    assert v.meta_errors(d4_style, Draft.DRAFT202012) != []
    assert v.meta_errors(True, Draft.DRAFT202012) == []
    assert v.meta_errors(True, Draft.DRAFT4) == ['(root): true is not of type "object"']


def test_meta_errors_assert_formats(v: Validator) -> None:
    for draft in Draft:
        assert v.meta_errors({"pattern": "(?<"}, draft) != []


def test_meta_errors_type_checks(v: Validator) -> None:
    with pytest.raises(TypeError):
        v.meta_errors({}, "https://json-schema.org/draft/2020-12/schema")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        v.meta_errors([], Draft.DRAFT202012)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# satisfies_format and matches_pattern
# ---------------------------------------------------------------------------

FORMAT_EXAMPLES = {
    "date-time": ("2024-01-02T03:04:05Z", "2024-01-02"),
    "date": ("2024-01-02", "2024-13-02"),
    "time": ("03:04:05Z", "25:00:00Z"),
    "duration": ("P1DT2H", "1D"),
    "email": ("a@example.com", "not an email"),
    "hostname": ("example.com", "exa mple.com"),
    "ipv4": ("192.168.0.1", "256.1.1.1"),
    "ipv6": ("::1", "not:ipv6"),
    "uri": ("https://example.com/x", "not a uri"),
    "uuid": ("123e4567-e89b-12d3-a456-426614174000", "123e4567"),
}


@pytest.mark.parametrize("fmt", sorted(FORMAT_EXAMPLES))
def test_satisfies_format_for_the_ten_formats(v: Validator, fmt: str) -> None:
    good, bad = FORMAT_EXAMPLES[fmt]
    assert v.satisfies_format(fmt, good)
    assert not v.satisfies_format(fmt, bad)


def test_satisfies_format_agrees_with_schema_verdicts(v: Validator) -> None:
    for fmt, examples in FORMAT_EXAMPLES.items():
        compiled = v.compile({"type": "string", "format": fmt})
        for s in examples:
            assert v.satisfies_format(fmt, s) == v.is_valid(compiled, s)


def test_format_and_pattern_results_are_cached_per_distinct_value(v: Validator) -> None:
    assert len(v._values) == 0
    for _ in range(3):
        v.satisfies_format("date", "2024-01-02")
        v.matches_pattern("^a", "abc")
    assert len(v._values) == 2
    v.satisfies_format("date", "2024-01-03")
    v.satisfies_format("time", "2024-01-03")
    v.matches_pattern("^b", "abc")
    assert len(v._values) == 5


def test_value_cache_is_bounded() -> None:
    small = Validator(max_values=2)
    for s in ["2024-01-01", "2024-01-02", "x", "2024-01-01"]:
        small.satisfies_format("date", s)
    assert len(small._values) == 2
    assert small.satisfies_format("date", "2024-01-01")
    assert not small.satisfies_format("date", "x")


def test_matches_pattern_uses_unanchored_ecma_style_search(v: Validator) -> None:
    assert v.matches_pattern("b", "abc")
    assert not v.matches_pattern("^b", "abc")
    assert v.matches_pattern("^[0-9]{3}$", "123")
    assert not v.matches_pattern("^[0-9]{3}$", "1234")
    assert v.matches_pattern("(?<!x)y", "ay")
    assert not v.matches_pattern("(?<!x)y", "xy")
    assert v.matches_pattern("^\\p{L}+$", "héllo")


def test_matches_pattern_raises_compile_error_for_bad_patterns(v: Validator) -> None:
    with pytest.raises(CompileError) as info:
        v.matches_pattern("(unclosed", "x")
    assert info.value.schema_id == '{"pattern": "(unclosed"}'


def test_format_and_pattern_type_checks(v: Validator) -> None:
    with pytest.raises(TypeError):
        v.satisfies_format("date", 5)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        v.matches_pattern(None, "x")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Pickling (validators are rebuilt in each process)
# ---------------------------------------------------------------------------


def test_compiled_pickles_by_schema_and_recompiles(v: Validator) -> None:
    schema = {"$defs": {"n": {"type": "integer"}}, "items": {"$ref": "#/$defs/n"}}
    compiled = v.compile(schema)
    clone = pickle.loads(pickle.dumps(compiled))
    assert isinstance(clone, Compiled) and clone is not compiled
    assert (clone.digest, clone.draft, clone.dialect_assumed) == (compiled.digest, compiled.draft, True)
    assert v.is_valid(clone, [1, 2]) and not v.is_valid(clone, [1, "x"])
    assert v.errors(clone, [1, "x"]) == v.errors(compiled, [1, "x"])


def test_validator_pickles_with_empty_caches(v: Validator) -> None:
    v.compile({"type": "integer"})
    v.satisfies_format("date", "2024-01-02")
    clone = pickle.loads(pickle.dumps(Validator(max_compiled=7, max_values=9)))
    assert (clone._max_compiled, clone._max_values) == (7, 9)
    fresh = pickle.loads(pickle.dumps(v))
    assert len(fresh._compiled) == 0 and len(fresh._values) == 0
    assert fresh.is_valid(fresh.compile({"type": "integer"}), 3)
