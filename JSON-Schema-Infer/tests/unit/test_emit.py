"""Unit tests for io/emit.py, the Schema_Emitter layout and bytes (task 6.1)."""

from __future__ import annotations

import copy
import json
import pickle
from typing import Any

import pytest

from jsinfer.core.determinism import encode_canonical
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode, new_nid, split_pointer
from jsinfer.io.emit import (
    KEYWORD_ORDER,
    SCHEMA_DIALECT,
    VOCABULARY_KEYWORDS,
    EmitError,
    SchemaEmitter,
    keyword_rank,
)
from jsinfer.io.validator import Draft, Validator

EMITTER = SchemaEmitter()

# The draft 2020-12 vocabularies, listed independently of the emitter's table.
VOCABULARIES_2020_12 = {
    "core": ["$id", "$schema", "$ref", "$anchor", "$dynamicRef", "$dynamicAnchor", "$vocabulary", "$comment", "$defs"],
    "applicator": [
        "prefixItems",
        "items",
        "contains",
        "additionalProperties",
        "properties",
        "patternProperties",
        "dependentSchemas",
        "propertyNames",
        "if",
        "then",
        "else",
        "allOf",
        "anyOf",
        "oneOf",
        "not",
    ],
    "unevaluated": ["unevaluatedItems", "unevaluatedProperties"],
    "validation": [
        "type",
        "const",
        "enum",
        "multipleOf",
        "maximum",
        "exclusiveMaximum",
        "minimum",
        "exclusiveMinimum",
        "maxLength",
        "minLength",
        "pattern",
        "maxItems",
        "minItems",
        "uniqueItems",
        "maxContains",
        "minContains",
        "maxProperties",
        "minProperties",
        "required",
        "dependentRequired",
    ],
    "meta-data": ["title", "description", "default", "deprecated", "readOnly", "writeOnly", "examples"],
    "format": ["format"],
    "content": ["contentEncoding", "contentMediaType", "contentSchema"],
}


def model(schema: Any) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(schema))


def doc_of(schema: Any) -> Any:
    return EMITTER.layout(model(schema))[0]


def strict(value: Any) -> str:
    """A spelling that tells apart what Python's == conflates (True/1, 2/2.0, key order)."""
    return json.dumps(value, ensure_ascii=False)


def at(doc: Any, pointer: str) -> Any:
    """The value at a JSON Pointer in a plain JSON document (KeyError or IndexError when absent)."""
    value = doc
    for token in split_pointer(pointer):
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def has_key_anywhere(value: Any, key: str) -> list[str]:
    """Pointers of the schema objects that hold ``key`` as a keyword (property names are skipped)."""
    found: list[str] = []
    stack: list[tuple[str, Any, bool]] = [("", value, True)]
    while stack:
        pointer, item, is_schema = stack.pop()
        if isinstance(item, dict):
            if is_schema and key in item:
                found.append(pointer)
            for k, v in item.items():
                # Values of map keywords are name -> schema maps; their keys are names, not keywords.
                if is_schema and k in ("properties", "patternProperties", "$defs"):
                    stack.extend((f"{pointer}/{k}/{n}", s, True) for n, s in v.items())
                elif is_schema and k in ("items", "additionalProperties", "propertyNames", "contains", "not"):
                    stack.append((f"{pointer}/{k}", v, True))
                elif is_schema and k in ("prefixItems", "anyOf", "oneOf", "allOf"):
                    stack.extend((f"{pointer}/{k}/{i}", s, True) for i, s in enumerate(v))
    return sorted(found)


RICH_SCHEMA: dict[str, Any] = {
    "$id": "https://example.com/rich",
    "title": "Rich",
    "description": "every supported keyword",
    "$comment": "c",
    "type": ["object", "null"],
    "properties": {
        "name": {"type": "string", "minLength": 1, "maxLength": 40, "pattern": "^[a-z]+$", "x-latent-type": "word"},
        "when": {"type": "string", "format": "date-time"},
        "kind": {"enum": ["b", "a"], "default": "a", "examples": ["a"]},
        "n": {"type": "number", "minimum": 0, "exclusiveMaximum": 100.5},
        "tags": {
            "type": "array",
            "items": {"type": "string"},
            "minItems": 0,
            "maxItems": 8,
            "uniqueItems": True,
            "contains": {"const": "x"},
        },
        "pair": {"type": "array", "prefixItems": [{"type": "integer"}, {"type": "boolean"}], "items": False},
        "map": {
            "type": "object",
            "additionalProperties": {"type": "integer"},
            "propertyNames": {"format": "uuid"},
            "minProperties": 1,
            "maxProperties": 100,
        },
        "ref": {"$ref": "#/$defs/node"},
    },
    "patternProperties": {"^x_": {"type": "string"}},
    "additionalProperties": False,
    "required": ["name", "kind"],
    "dependentRequired": {"when": ["n"]},
    "anyOf": [{"required": ["n"]}, True],
    "oneOf": [{"required": ["tags"]}, {"not": {"required": ["tags"]}}],
    "allOf": [{"if": {"required": ["pair"]}, "then": {"required": ["map"]}, "else": {}}],
    "$defs": {"node": {"type": "object", "properties": {"next": {"$ref": "#/$defs/node"}}}},
    "x-key": ["name"],
}


# ---------------------------------------------------------------------------
# Keyword rank order
# ---------------------------------------------------------------------------


def test_keyword_table_is_the_2020_12_vocabulary() -> None:
    expected = [keyword for vocabulary in VOCABULARIES_2020_12.values() for keyword in vocabulary]
    assert len(set(expected)) == len(expected) == 57
    assert VOCABULARY_KEYWORDS == frozenset(expected)
    assert len(KEYWORD_ORDER) == len(VOCABULARY_KEYWORDS)


def test_keywords_follow_the_rank_order() -> None:
    schema = {
        "x-😀": 0,
        "x-provenance": {"dropped": True},
        "$defs": {"d": {}},
        "else": {},
        "then": {},
        "if": {},
        "not": {},
        "allOf": [{}],
        "oneOf": [{}],
        "anyOf": [{}],
        "uniqueItems": True,
        "maxItems": 3,
        "minItems": 1,
        "contains": {},
        "items": {},
        "prefixItems": [{}],
        "maxProperties": 4,
        "minProperties": 1,
        "dependentRequired": {"a": ["b"]},
        "required": ["a"],
        "propertyNames": {},
        "additionalProperties": False,
        "patternProperties": {"^a": {}},
        "properties": {"a": {}},
        "maxLength": 3,
        "minLength": 1,
        "exclusiveMaximum": 9,
        "maximum": 10,
        "exclusiveMinimum": 0,
        "minimum": 0,
        "pattern": "a",
        "format": "date",
        "const": 1,
        "enum": [1],
        "type": "integer",
        "x-a": 2,
        "writeOnly": False,
        "readOnly": False,
        "deprecated": False,
        "examples": [1],
        "default": 1,
        "$comment": "c",
        "description": "d",
        "title": "t",
        "x-é": 3,
        "$ref": "#/$defs/d",
        "x-B": 4,
        "$id": "https://example.com/s",
        "x-\uffff": 5,
    }
    assert list(doc_of(schema)) == [
        "$schema",
        "$id",
        "$ref",
        "title",
        "description",
        "$comment",
        "default",
        "examples",
        "deprecated",
        "readOnly",
        "writeOnly",
        "type",
        "enum",
        "const",
        "format",
        "pattern",
        "minimum",
        "exclusiveMinimum",
        "maximum",
        "exclusiveMaximum",
        "minLength",
        "maxLength",
        "properties",
        "patternProperties",
        "additionalProperties",
        "propertyNames",
        "required",
        "dependentRequired",
        "minProperties",
        "maxProperties",
        "prefixItems",
        "items",
        "contains",
        "minItems",
        "maxItems",
        "uniqueItems",
        "anyOf",
        "oneOf",
        "allOf",
        "not",
        "if",
        "then",
        "else",
        "$defs",
        # x- keywords in code-point order ("\uffff" < "😀", unlike UTF-16 order)
        "x-B",
        "x-a",
        "x-é",
        "x-\uffff",
        "x-😀",
    ]


def test_keyword_rank_puts_x_keywords_after_the_vocabulary_and_x_provenance_last() -> None:
    ranked = sorted(["x-provenance", "x-z", "$defs", "x-a", "$schema"], key=keyword_rank)
    assert ranked == ["$schema", "$defs", "x-a", "x-z", "x-provenance"]
    with pytest.raises(ValueError, match="x-"):
        keyword_rank("markdownDescription")


# ---------------------------------------------------------------------------
# Canonical orders of names and set-compared lists
# ---------------------------------------------------------------------------

NAMES = ["b", "B", "a", "😀", "\uffff", "é", "", "a b"]
SORTED_NAMES = ["", "B", "a", "a b", "b", "é", "\uffff", "😀"]


@pytest.mark.parametrize("keyword", ["properties", "patternProperties", "$defs"])
def test_map_keywords_list_names_in_code_point_order(keyword: str) -> None:
    doc = doc_of({keyword: {name: {} for name in NAMES}})
    assert list(doc[keyword]) == SORTED_NAMES


def test_required_and_dependent_required_are_distinct_and_sorted() -> None:
    doc = doc_of({"required": ["b", "a", "😀", "\uffff", "a"], "dependentRequired": {"z": ["y", "x", "y"], "B": []}})
    assert doc["required"] == ["a", "b", "\uffff", "😀"]
    assert list(doc["dependentRequired"]) == ["B", "z"]
    assert doc["dependentRequired"] == {"B": [], "z": ["x", "y"]}


def test_type_lists_distinct_names_in_type_order() -> None:
    names = ["object", "array", "string", "number", "integer", "boolean", "null", "string"]
    assert doc_of({"type": names})["type"] == ["null", "boolean", "integer", "number", "string", "array", "object"]
    assert doc_of({"type": ["string", "string"]})["type"] == "string"
    assert doc_of({"type": ["string"]})["type"] == "string"
    assert doc_of({"type": "string"})["type"] == "string"


def test_enum_lists_distinct_members_in_canonical_value_order() -> None:
    members = ["b", 2.0, None, {"y": 1, "x": 2}, [1], True, 1.5, "a", 2, False, {"x": 2.0, "y": 1}, 1, [1.0]]
    emitted = doc_of({"enum": members})["enum"]
    assert strict(emitted) == strict([None, False, True, 1, 2, 1.5, "a", "b", [1], {"x": 2, "y": 1}])


# ---------------------------------------------------------------------------
# Values
# ---------------------------------------------------------------------------


def test_integral_floats_become_ints_everywhere() -> None:
    schema = {
        "minimum": 2.0,
        "maximum": 1e3,
        "exclusiveMinimum": -0.0,
        "minLength": 1.0,
        "const": {"b": -0.0, "a": [1.0, 2.5]},
        "default": 3.0,
        "examples": [4.0, "x"],
        "enum": [6.0, 6],
        "x-n": {"k": 5.0},
        "properties": {"big": {"maximum": 1e40}},
    }
    doc = doc_of(schema)
    assert strict(doc["minimum"]) == "2"
    assert strict(doc["maximum"]) == "1000"
    assert strict(doc["exclusiveMinimum"]) == "0"
    assert strict(doc["minLength"]) == "1"
    assert strict(doc["const"]) == '{"a": [1, 2.5], "b": 0}'
    assert strict(doc["default"]) == "3"
    assert strict(doc["examples"]) == '[4, "x"]'
    assert strict(doc["enum"]) == "[6]"
    assert strict(doc["x-n"]) == '{"k": 5}'
    big = doc["properties"]["big"]["maximum"]
    assert type(big) is int and big == int(1e40)
    assert b".0" not in EMITTER.dumps(doc)
    assert strict(doc_of({"exclusiveMaximum": 100.5})["exclusiveMaximum"]) == "100.5"


def test_layout_shares_nothing_with_the_model_and_leaves_it_unchanged() -> None:
    m = model(RICH_SCHEMA | {"const": {"a": [1]}, "x-provenance": {"keep": "in model"}})
    before = strict(m.root.to_json())
    doc, _ = EMITTER.layout(m)
    assert strict(m.root.to_json()) == before
    doc["const"]["a"].append(2)
    doc["properties"]["name"]["type"] = "integer"
    assert strict(m.root.to_json()) == before


# ---------------------------------------------------------------------------
# $schema and boolean roots
# ---------------------------------------------------------------------------


def test_schema_uri_is_on_the_root_only() -> None:
    schema = {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "properties": {"a": {"$schema": SCHEMA_DIALECT, "type": "string"}, "$schema": {"type": "integer"}},
        "items": {"$schema": "https://example.com/custom"},
        "$defs": {"d": {"$schema": SCHEMA_DIALECT}},
    }
    doc = doc_of(schema)
    assert SCHEMA_DIALECT == "https://json-schema.org/draft/2020-12/schema"
    assert next(iter(doc)) == "$schema"
    assert doc["$schema"] == SCHEMA_DIALECT
    assert has_key_anywhere(doc, "$schema") == [""]
    # A property named "$schema" is a name, not a keyword.
    assert doc["properties"]["$schema"] == {"type": "integer"}
    assert doc["items"] == {}
    assert doc_of({}) == {"$schema": SCHEMA_DIALECT}


@pytest.mark.parametrize("value", [True, False])
def test_boolean_roots_are_emitted_as_booleans(value: bool) -> None:
    doc, pointers = EMITTER.layout(SchemaModel(SchemaNode(value)))
    assert doc is value
    assert pointers == {}
    assert EMITTER.dumps(doc) == (b"true\n" if value else b"false\n")
    assert Validator().meta_errors(doc, Draft.DRAFT202012) == []


def test_boolean_subschemas_stay_booleans() -> None:
    doc = doc_of({"items": False, "additionalProperties": True, "anyOf": [True, False]})
    assert doc["items"] is False and doc["additionalProperties"] is True and doc["anyOf"] == [True, False]


# ---------------------------------------------------------------------------
# x- prefix check and value checks
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("schema", "pointer"),
    [
        ({"markdownDescription": "md"}, "/markdownDescription"),
        ({"properties": {"a": {"definitions": {}}}}, "/properties/a/definitions"),
        ({"items": {"dependencies": {"a": ["b"]}}}, "/items/dependencies"),
        ({"anyOf": [{}, {"nullable": True}]}, "/anyOf/1/nullable"),
        ({"$defs": {"d~/": {"X-upper": 1}}}, "/$defs/d~0~1/X-upper"),
    ],
)
def test_keywords_outside_the_vocabulary_need_the_x_prefix(schema: dict[str, Any], pointer: str) -> None:
    with pytest.raises(EmitError, match="x-") as info:
        EMITTER.layout(model(schema))
    assert info.value.pointer == pointer


def test_names_are_not_keywords_and_x_provenance_is_dropped() -> None:
    schema = {
        "properties": {"markdownDescription": {"x-provenance": {"r": 1}}, "x-provenance": {}},
        "patternProperties": {"nullable": {}},
        "dependentRequired": {"definitions": ["x-provenance"]},
        "$defs": {"dependencies": {}},
        "x-provenance": [],
        "x-custom": {"anything": None},
    }
    doc = doc_of(schema)
    assert "x-provenance" not in doc
    assert doc["properties"] == {"markdownDescription": {}, "x-provenance": {}}
    assert doc["dependentRequired"] == {"definitions": ["x-provenance"]}
    assert doc["x-custom"] == {"anything": None}


def _with(kw: dict[str, Any]) -> SchemaModel:
    """A model whose root keywords bypass SchemaNode's construction checks."""
    root = SchemaNode()
    root.kw.update(kw)
    return SchemaModel(root)


@pytest.mark.parametrize(
    ("kw", "pointer", "match"),
    [
        ({"type": "float"}, "/type", "not a JSON Schema type"),
        ({"type": []}, "/type", "at least one type"),
        ({"type": 5}, "/type", "string or an array"),
        ({"type": ["string", 1]}, "/type", "not a JSON Schema type"),
        ({"required": [1]}, "/required", "strings"),
        ({"required": "a"}, "/required", "array of strings"),
        ({"dependentRequired": {"a": "b"}}, "/dependentRequired/a", "array of strings"),
        ({"dependentRequired": ["a"]}, "/dependentRequired", "object"),
        ({"enum": "a"}, "/enum", "array"),
        ({"minimum": float("nan")}, "/minimum", "not a JSON value"),
        ({"const": {1, 2}}, "/const", "not a JSON value"),
        ({"x-a": {1: "a"}}, "/x-a", "not a JSON value"),
        ({"items": {"type": "string"}}, "/items", "SchemaNode"),
        ({"anyOf": SchemaNode()}, "/anyOf", "list of SchemaNodes"),
        ({"properties": [SchemaNode()]}, "/properties", "dict of SchemaNodes"),
        ({"prefixItems": [True]}, "/prefixItems/0", "SchemaNode"),
    ],
)
def test_values_that_cannot_be_emitted_raise(kw: dict[str, Any], pointer: str, match: str) -> None:
    with pytest.raises(EmitError, match=match) as info:
        EMITTER.layout(_with(kw))
    assert info.value.pointer == pointer


def test_emit_error_names_the_location_and_pickles() -> None:
    error = EmitError("/properties/a/type", "bad")
    assert str(error) == "/properties/a/type: bad"
    assert str(EmitError("", "bad")) == "(root): bad"
    again = pickle.loads(pickle.dumps(error))
    assert (again.pointer, again.message, str(again)) == (error.pointer, error.message, str(error))
    assert isinstance(error, ValueError)


def test_layout_rejects_non_models() -> None:
    with pytest.raises(TypeError):
        EMITTER.layout(SchemaNode())  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        EMITTER.dumps("{}")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Pointer map
# ---------------------------------------------------------------------------


def test_pointer_map_follows_the_granularity_table() -> None:
    leaf_a = SchemaNode(kw={"title": "A", "format": "date", "type": "string"})
    leaf_b = SchemaNode(kw={"minimum": 0, "type": ["integer", "null"]})
    closed = SchemaNode(False)
    branch0 = SchemaNode(kw={"const": 1})
    branch1 = SchemaNode(True)
    definition = SchemaNode(kw={"x-latent-type": "date", "type": "object", "description": "d"})
    ref = SchemaNode(kw={"$ref": "#/$defs/d"})
    root = SchemaNode(
        kw={
            "x-key": ["b"],
            "$defs": {"d": definition},
            "anyOf": [branch0, branch1],
            "dependentRequired": {"a/b": ["b"], "b": ["a/b", "b"]},
            "required": ["b", "a/b", "b"],
            "additionalProperties": closed,
            "properties": {"b": leaf_b, "a/b": leaf_a, "c": ref},
            "type": "object",
            "description": "root",
            "examples": [{}],
            "$id": "https://example.com/s",
            "$comment": "not a Constraint",
            "x-provenance": {"r": 1},
        }
    )
    m = SchemaModel(root)
    doc, pointers = EMITTER.layout(m)
    r = root.nid
    assert pointers == {
        ConstraintId(r, "type"): "/type",
        ConstraintId(r, "properties"): "/properties",
        ConstraintId(leaf_a.nid, "type"): "/properties/a~1b/type",
        ConstraintId(leaf_a.nid, "format"): "/properties/a~1b/format",
        ConstraintId(leaf_b.nid, "type"): "/properties/b/type",
        ConstraintId(leaf_b.nid, "minimum"): "/properties/b/minimum",
        ConstraintId(ref.nid, "$ref"): "/properties/c/$ref",
        ConstraintId(r, "additionalProperties"): "/additionalProperties",
        ConstraintId(r, "required", "a/b"): "/required/0",
        ConstraintId(r, "required", "b"): "/required/1",
        ConstraintId(r, "dependentRequired", "a/b"): "/dependentRequired/a~1b",
        ConstraintId(r, "dependentRequired", "b"): "/dependentRequired/b",
        ConstraintId(r, "anyOf", 0): "/anyOf/0",
        ConstraintId(r, "anyOf", 1): "/anyOf/1",
        ConstraintId(branch0.nid, "const"): "/anyOf/0/const",
        ConstraintId(definition.nid, "type"): "/$defs/d/type",
        ConstraintId(definition.nid, "x-latent-type"): "/$defs/d/x-latent-type",
        ConstraintId(r, "x-key"): "/x-key",
    }
    # Every pointer resolves to the Constraint it names in the emitted document.
    for cid, pointer in pointers.items():
        target = at(doc, pointer)
        tokens = split_pointer(pointer)
        if cid.keyword in ("required", "dependentRequired"):
            assert tokens[-2] == cid.keyword
            assert (target if cid.keyword == "required" else tokens[-1]) == cid.item
        elif cid.keyword in ("anyOf", "oneOf"):
            assert tokens[-2:] == [cid.keyword, str(cid.item)]
        else:
            assert tokens[-1] == cid.keyword
    assert closed.nid not in {cid.nid for cid in pointers}
    assert branch1.nid not in {cid.nid for cid in pointers}
    # Pointers are distinct, and a second layout gives the same map in the same order.
    assert len(set(pointers.values())) == len(pointers)
    assert list(EMITTER.layout(m)[1].items()) == list(pointers.items())


def test_pointer_map_covers_every_constraint_of_the_rich_schema() -> None:
    m = model(RICH_SCHEMA)
    doc, pointers = EMITTER.layout(m)
    nodes = dict(m.walk())  # pointer -> node; layout keeps every subschema at its model pointer
    expected: dict[ConstraintId, str] = {}
    for pointer, node in nodes.items():
        for keyword, value in node.kw.items():
            if keyword in ("$schema", "$defs", "$id", "$comment", "title", "description", "default", "examples"):
                continue
            if keyword == "required":
                for index, name in enumerate(sorted(set(value))):
                    expected[ConstraintId(node.nid, keyword, name)] = f"{pointer}/required/{index}"
            elif keyword == "dependentRequired":
                for key in value:
                    expected[ConstraintId(node.nid, keyword, key)] = f"{pointer}/dependentRequired/{key}"
            elif keyword in ("anyOf", "oneOf"):
                for index in range(len(value)):
                    expected[ConstraintId(node.nid, keyword, index)] = f"{pointer}/{keyword}/{index}"
            else:
                expected[ConstraintId(node.nid, keyword)] = f"{pointer}/{keyword}"
    assert pointers == expected
    for pointer in pointers.values():
        at(doc, pointer)  # resolves


def test_repeated_nids_are_rejected() -> None:
    twin_a = SchemaNode(kw={"type": "string"})
    twin_b = SchemaNode(kw={"type": "integer"}, nid=twin_a.nid)
    m = SchemaModel(SchemaNode(kw={"properties": {"a": twin_a, "b": twin_b}}))
    with pytest.raises(EmitError, match="distinct nid") as info:
        EMITTER.layout(m)
    assert info.value.pointer == "/properties/b"
    twin_b.nid = new_nid()
    _, pointers = EMITTER.layout(m)
    assert pointers[ConstraintId(twin_b.nid, "type")] == "/properties/b/type"
    assert len(pointers) == 3

    shared = SchemaNode(kw={"type": "string"})
    with pytest.raises(EmitError, match="nid"):
        EMITTER.layout(SchemaModel(SchemaNode(kw={"anyOf": [shared, shared]})))


# ---------------------------------------------------------------------------
# Meta-schema validity, bytes, and structure preservation
# ---------------------------------------------------------------------------


def test_emitted_documents_validate_against_the_2020_12_meta_schema() -> None:
    validator = Validator()
    doc = doc_of(RICH_SCHEMA | {"x-provenance": {"r": 1}, "x-array-pattern": "K*"})
    assert validator.meta_errors(doc, Draft.DRAFT202012) == []
    compiled = validator.compile(doc)
    assert compiled.draft is Draft.DRAFT202012 and not compiled.dialect_assumed
    good = {"name": "abc", "kind": "a", "n": 1, "tags": ["x"]}
    assert validator.is_valid(compiled, good)
    assert not validator.is_valid(compiled, good | {"name": "ABC"})
    assert not validator.is_valid(compiled, good | {"unknown": 1})
    assert validator.meta_errors(doc_of({"type": ["integer", "integer"], "enum": [1, 1.0]}), Draft.DRAFT202012) == []


def test_dumps_is_the_canonical_encoder() -> None:
    doc = doc_of({"properties": {"é": {"const": "☃"}}})
    data = EMITTER.dumps(doc)
    assert data == encode_canonical(doc)
    assert data == (
        '{\n  "$schema": "https://json-schema.org/draft/2020-12/schema",\n'
        '  "properties": {\n    "é": {\n      "const": "☃"\n    }\n  }\n}\n'
    ).encode()
    assert EMITTER.emit(model({"properties": {"é": {"const": "☃"}}})) == data


def test_equal_models_give_identical_bytes() -> None:
    first = {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": ["string", "null", "string"],
        "required": ["b", "a"],
        "enum": [1, "a", 1.0],
        "const": {"b": 1, "a": 2},
        "properties": {"b": {"type": "string"}, "a": {"x-provenance": {"r": 1}}},
        "dependentRequired": {"b": ["a"], "a": ["b", "b"]},
        "x-z": 1,
        "x-a": [1.0],
    }
    second = {
        "x-a": [1],
        "x-z": 1.0,
        "dependentRequired": {"a": ["b"], "b": ["a"]},
        "properties": {"a": {}, "b": {"type": ["string"]}},
        "const": {"a": 2.0, "b": 1},
        "enum": ["a", 1],
        "required": ["a", "b", "a"],
        "type": ["null", "string"],
        "x-provenance": [],
    }
    assert model(first) == model(second)
    assert EMITTER.emit(model(first)) == EMITTER.emit(model(second))
    reordered = dict(reversed(list(RICH_SCHEMA.items())))
    assert EMITTER.emit(model(reordered)) == EMITTER.emit(model(RICH_SCHEMA))


def test_emitted_document_has_the_model_structure() -> None:
    m = model(RICH_SCHEMA)
    doc = json.loads(EMITTER.emit(m))
    assert SchemaModel(SchemaNode.from_json(doc)) == m
    assert SchemaModel(SchemaNode.from_json(copy.deepcopy(doc))).equality_key() == m.equality_key()
