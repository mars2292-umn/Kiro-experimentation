"""Unit tests for io/load.py, the Schema_Loader (task 6.2).

Loader error paths (2.12) are covered by the task 6.8 tests.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

import pytest

from jsinfer.core.schema_model import SUBSCHEMA_KEYWORDS, SchemaModel, SchemaNode
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import ANNOTATION_KEYWORDS, SUPPORTED_KEYWORDS, SchemaLoader
from jsinfer.io.validator import Validator

D4 = "http://json-schema.org/draft-04/schema#"
D6 = "http://json-schema.org/draft-06/schema#"
D7 = "http://json-schema.org/draft-07/schema#"
D2019 = "https://json-schema.org/draft/2019-09/schema"
D2020 = "https://json-schema.org/draft/2020-12/schema"
TRANSLATED = [D4, D6, D7, D2019]


@pytest.fixture(scope="module")
def loader() -> SchemaLoader:
    return SchemaLoader()


def without_schema(schema: dict[str, Any]) -> dict[str, Any]:
    return {k: v for k, v in schema.items() if k != "$schema"}


# ---------------------------------------------------------------------------
# Dialects (2.11)
# ---------------------------------------------------------------------------

# Supported keywords that some drafts do not define; the declared draft's set of
# ignored ones identifies it.
DIALECT_PROBE: dict[str, Any] = {"const": 1, "if": {}, "dependentRequired": {"a": ["b"]}, "prefixItems": [{}]}


@pytest.mark.parametrize(
    ("declared", "ignored"),
    [
        ("http://json-schema.org/draft-04/schema#", ["const", "dependentRequired", "if", "prefixItems"]),
        ("https://json-schema.org/draft-04/schema", ["const", "dependentRequired", "if", "prefixItems"]),
        ("http://json-schema.org/draft-04/schema", ["const", "dependentRequired", "if", "prefixItems"]),
        ("https://json-schema.org/draft-06/schema#", ["dependentRequired", "if", "prefixItems"]),
        ("http://json-schema.org/draft-07/schema", ["dependentRequired", "prefixItems"]),
        ("https://json-schema.org/draft-07/schema#", ["dependentRequired", "prefixItems"]),
        ("https://json-schema.org/draft/2019-09/schema", ["prefixItems"]),
        ("http://json-schema.org/draft/2019-09/schema#", ["prefixItems"]),
        ("https://json-schema.org/draft/2020-12/schema", []),
        ("http://json-schema.org/draft/2020-12/schema#", []),
    ],
)
def test_declared_dialect_is_recognized_in_every_spelling(
    loader: SchemaLoader, declared: str, ignored: list[str]
) -> None:
    model = loader.load({"$schema": declared, **DIALECT_PROBE})
    assert not model.dialect_assumed
    assert model.fully_supported
    # Keywords the declared draft does not define have no effect under it, so they are left out.
    assert model.ignored == [("", keyword) for keyword in ignored]
    assert sorted(model.root.kw) == sorted(k for k in DIALECT_PROBE if k not in ignored)


def test_missing_dialect_is_assumed_2020_12(loader: SchemaLoader) -> None:
    model = loader.load(DIALECT_PROBE)
    assert model.dialect_assumed
    assert (model.ignored, model.unsupported) == ([], [])
    assert model.root == SchemaNode.from_json(DIALECT_PROBE)


@pytest.mark.parametrize("declared", ["https://example.com/custom-meta", "http://json-schema.org/schema#"])
def test_unrecognized_dialect_is_assumed_2020_12_and_unsupported(loader: SchemaLoader, declared: str) -> None:
    model = loader.load({"$schema": declared, **DIALECT_PROBE})
    assert model.dialect_assumed
    assert model.unsupported == [("", "$schema")]
    assert not model.fully_supported
    assert model.ignored == []
    assert model.root == SchemaNode.from_json(DIALECT_PROBE)  # loaded with 2020-12 semantics


@pytest.mark.parametrize("value", [True, False])
def test_boolean_root_is_a_boolean_schema_with_assumed_dialect(loader: SchemaLoader, value: bool) -> None:
    model = loader.load(value)
    assert model.root.boolean is value
    assert model.dialect_assumed
    assert model.fully_supported


# ---------------------------------------------------------------------------
# Supported keywords and boolean schemas (2.4)
# ---------------------------------------------------------------------------

ALL_SUPPORTED: dict[str, Any] = {
    "$schema": D2020,
    "type": ["object", "null"],
    "properties": {
        "s": {"type": "string", "minLength": 1, "maxLength": 9, "pattern": "^[a-z]+$", "format": "email"},
        "n": {"type": "number", "minimum": 0, "maximum": 1.5, "exclusiveMinimum": -1, "exclusiveMaximum": 2},
        "e": {"enum": [1, "a", None, [1], {"k": True}]},
        "c": {"const": {"nested": [1, 2.5]}},
        "a": {
            "type": "array",
            "prefixItems": [{"type": "integer"}, {"type": "string"}],
            "items": False,
            "contains": {"type": "integer"},
            "minItems": 1,
            "maxItems": 3,
            "uniqueItems": True,
        },
        "m": {
            "patternProperties": {"^x": {"type": "integer"}},
            "additionalProperties": {"type": "string"},
            "propertyNames": {"maxLength": 5},
            "minProperties": 0,
            "maxProperties": 4,
        },
        "u": {"anyOf": [{"type": "string"}, {"type": "integer"}], "oneOf": [{"minimum": 0}, {"maximum": -1}]},
        "i": {
            "allOf": [{"type": "integer"}],
            "not": {"const": 3},
            "if": {"minimum": 5},
            "then": {"maximum": 9},
            "else": {},
        },
        "r": {"$ref": "#/$defs/node"},
        # Property names are not keywords.
        "multipleOf": {"type": "string"},
        "x-name": {},
        "$id": {},
    },
    "required": ["s", "n"],
    "dependentRequired": {"s": ["n", "e"]},
    "$defs": {"node": {"type": "object", "properties": {"next": {"$ref": "#/$defs/node"}}}},
}


def keywords_of(node: SchemaNode) -> set[str]:
    return {keyword for _, n in node.walk() for keyword in n.kw}


def test_every_supported_keyword_loads_unchanged(loader: SchemaLoader) -> None:
    model = loader.load(ALL_SUPPORTED)
    expected = without_schema(ALL_SUPPORTED)
    assert keywords_of(SchemaNode.from_json(expected)) == SUPPORTED_KEYWORDS  # the fixture covers 2.4
    assert model.root.to_json() == expected
    assert model.root == SchemaNode.from_json(expected)
    assert (model.dialect_assumed, model.ignored, model.unsupported) == (False, [], [])
    assert model.fully_supported


def test_boolean_schemas_load_at_every_subschema_position(loader: SchemaLoader) -> None:
    schema = {
        "properties": {"p": True},
        "patternProperties": {"^q": False},
        "additionalProperties": False,
        "propertyNames": True,
        "items": True,
        "prefixItems": [False, True],
        "contains": True,
        "anyOf": [True, False],
        "oneOf": [False, True],
        "allOf": [True],
        "not": False,
        "if": True,
        "then": False,
        "else": True,
        "$defs": {"t": True, "f": False},
    }
    assert set(schema) == set(SUBSCHEMA_KEYWORDS)
    model = loader.load(schema)
    assert model.fully_supported
    assert model.root.to_json() == schema
    leaves = [node for pointer, node in model.walk() if pointer]
    assert len(leaves) == 19
    assert all(node.boolean is not None and not node.kw for node in leaves)


# ---------------------------------------------------------------------------
# Local references (2.4, 2.6)
# ---------------------------------------------------------------------------


def test_local_and_recursive_refs_load_as_written(loader: SchemaLoader) -> None:
    schema = {
        "$schema": D2020,
        "type": "object",
        "properties": {
            "self": {"$ref": "#"},
            "tree": {"$ref": "#/$defs/tree"},
            "sibling": {"$ref": "#/properties/tree", "description": "2020-12 keeps $ref siblings"},
            "space": {"$ref": "#/$defs/a%20b"},
            "slash": {"$ref": "#/$defs/c~1d"},
            "tilde": {"$ref": "#/$defs/e%7E0f"},
            "branch": {"$ref": "#/$defs/tree/anyOf/1"},
        },
        "$defs": {
            "tree": {"anyOf": [{"type": "null"}, {"type": "array", "items": {"$ref": "#/$defs/tree"}}]},
            "a b": {"type": "integer"},
            "c/d": {"type": "string"},
            "e~f": True,
        },
    }
    model = loader.load(schema)
    assert model.fully_supported and model.ignored == []
    assert model.root.to_json() == without_schema(schema)
    assert model.resolve("/$defs/tree/anyOf/1/items").kw == {"$ref": "#/$defs/tree"}


@pytest.mark.parametrize(
    "ref",
    [
        "https://example.com/other.json",  # remote
        "other.json#/$defs/a",  # relative
        "#a-name",  # plain-name fragment
        "#/$defs/missing",  # does not resolve
        "#/$defs",  # a container, not a schema
        "#/properties/p/type",  # a JSON value, not a schema
        "#/x-store/a",  # x- values are JSON, not subschemas
        "#/multipleOf",  # not a schema position
        "#/$defs/a~2",  # malformed pointer escape
    ],
)
def test_non_local_and_unresolved_refs_are_unsupported(loader: SchemaLoader, ref: str) -> None:
    schema = {
        "properties": {"p": {"$ref": ref, "type": "string"}},
        "$defs": {"a": {"type": "integer"}},
        "x-store": {"a": {"type": "integer"}},
    }
    model = loader.load(schema)
    assert model.unsupported == [("/properties/p", "$ref")]
    assert not model.fully_supported
    assert model.resolve("/properties/p").kw == {"type": "string"}
    assert model.resolve("/$defs/a").kw == {"type": "integer"}


def test_refs_into_omitted_constructs_are_unsupported(loader: SchemaLoader) -> None:
    schema = {
        "properties": {"p": {"$ref": "#/dependentSchemas/a"}, "q": {"$ref": "#/$defs/d/markdownExtra"}},
        "dependentSchemas": {"a": {"type": "object"}},
        "$defs": {"d": {"markdownExtra": {"type": "string"}}},
    }
    model = loader.load(schema)
    assert model.unsupported == [("", "dependentSchemas"), ("/properties/p", "$ref"), ("/properties/q", "$ref")]
    assert model.ignored == [("/$defs/d", "markdownExtra")]
    assert model.resolve("/properties/p").kw == {} and model.resolve("/properties/q").kw == {}


def test_refs_inside_embedded_resources_are_unsupported(loader: SchemaLoader) -> None:
    # Inside the resource, "#/$defs/c" means the resource's own $defs/c, not the document's.
    schema = {
        "$ref": "#/$defs/a",
        "$defs": {
            "c": {"type": "integer"},
            "a": {
                "$id": "https://example.com/a",
                "$ref": "#/$defs/c",
                "$defs": {"c": {"type": "string"}},
                "properties": {"deep": {"$ref": "#/$defs/c"}},
            },
        },
    }
    model = loader.load(schema)
    assert model.unsupported == [("/$defs/a", "$id"), ("/$defs/a", "$ref"), ("/$defs/a/properties/deep", "$ref")]
    assert model.root.kw["$ref"] == "#/$defs/a"  # outside the resource: kept
    assert "$ref" not in model.resolve("/$defs/a").kw
    assert model.resolve("/$defs/a/$defs/c").kw == {"type": "string"}


def test_ids_that_start_no_resource_keep_refs_local(loader: SchemaLoader) -> None:
    # A bare-fragment $id is an anchor, and drafts 04 to 07 ignore an $id next to $ref.
    schema = {
        "$schema": D7,
        "properties": {
            "anchor": {"$id": "#here", "properties": {"q": {"$ref": "#/$defs/b"}}},
            "with_ref": {"$id": "https://example.com/x", "$ref": "#/$defs/b"},
        },
        "$defs": {"b": {"type": "integer"}},
    }
    model = loader.load(schema)
    assert model.resolve("/properties/anchor/properties/q").kw == {"$ref": "#/$defs/b"}
    assert model.resolve("/properties/with_ref").kw == {"$ref": "#/$defs/b"}
    assert ("/properties/anchor", "$id") in model.unsupported  # $id below the root
    assert ("/properties/with_ref", "$id") in model.ignored  # hidden by $ref


def test_defs_load_under_older_drafts(loader: SchemaLoader) -> None:
    # $defs asserts nothing, and a JSON Pointer $ref reaches into it under every draft.
    schema = {"$schema": D4, "$defs": {"a": {"type": "string"}}, "properties": {"p": {"$ref": "#/$defs/a"}}}
    model = loader.load(schema)
    assert model.fully_supported and model.ignored == []
    assert model.root.to_json() == without_schema(schema)


# ---------------------------------------------------------------------------
# Annotations, root $id, and x- keywords (2.10)
# ---------------------------------------------------------------------------

ANNOTATIONS: dict[str, Any] = {
    "title": "T",
    "description": "D",
    "default": {"a": [1, None]},
    "examples": [1, "two"],
    "deprecated": True,
    "readOnly": False,
    "writeOnly": True,
    "$comment": "note",
}


@pytest.mark.parametrize("declared", [D2020, D7, D4])
def test_annotations_root_id_and_x_keywords_are_kept(loader: SchemaLoader, declared: str) -> None:
    assert set(ANNOTATIONS) == ANNOTATION_KEYWORDS
    root: dict[str, Any] = {"$schema": declared, "x-root": {"deep": [1, {"x": None}]}, **ANNOTATIONS}
    root["id" if declared == D4 else "$id"] = "https://example.com/schema.json"  # draft-04 names it "id"
    root["properties"] = {"p": {"type": "string", "x-provenance": {"technique": "t"}, **ANNOTATIONS}}
    model = loader.load(root)
    assert (model.ignored, model.unsupported) == ([], [])
    expected = without_schema(root)
    if declared == D4:
        expected["$id"] = expected.pop("id")
    assert model.root.to_json() == expected


def test_id_below_root_and_anchor_are_unsupported(loader: SchemaLoader) -> None:
    schema = {"$id": "https://example.com/s", "$anchor": "top", "items": {"$id": "item.json", "type": "integer"}}
    model = loader.load(schema)
    assert model.unsupported == [("", "$anchor"), ("/items", "$id")]
    assert model.root.kw["$id"] == "https://example.com/s"
    assert model.resolve("/items").kw == {"type": "integer"}


# ---------------------------------------------------------------------------
# Ignored and unsupported records (2.6, 2.10)
# ---------------------------------------------------------------------------


def test_unknown_keywords_are_ignored_with_their_pointers(loader: SchemaLoader) -> None:
    schema = {
        "$schema": D2020,
        "markdownDescription": "root",
        "id": "draft-04 only",
        "properties": {"a": {"type": "string", "markdownDescription": "a", "errorMessage": {"type": "x"}}},
        "items": {"$schema": D2020, "type": "integer"},
        "$defs": {"d": {"anyOf": [{"deprecationMessage": "old"}]}},
        "additionalItems": False,  # not a 2020-12 keyword
    }
    model = loader.load(schema)
    assert model.fully_supported
    assert model.ignored == [
        ("", "additionalItems"),
        ("", "id"),
        ("", "markdownDescription"),
        ("/$defs/d/anyOf/0", "deprecationMessage"),
        ("/items", "$schema"),
        ("/properties/a", "errorMessage"),
        ("/properties/a", "markdownDescription"),
    ]
    assert model.root.to_json() == {
        "properties": {"a": {"type": "string"}},
        "items": {"type": "integer"},
        "$defs": {"d": {"anyOf": [{}]}},
    }


@pytest.mark.parametrize(
    ("keyword", "value"),
    [
        ("multipleOf", 2),
        ("dependentSchemas", {"a": {"required": ["b"]}}),
        ("unevaluatedProperties", False),
        ("unevaluatedItems", False),
        ("minContains", 1),
        ("maxContains", 2),
        ("contentEncoding", "base64"),
        ("contentMediaType", "application/json"),
        ("contentSchema", {"type": "object"}),
        ("$dynamicRef", "#meta"),
        ("$dynamicAnchor", "meta"),
        ("$recursiveRef", "#"),
        ("$recursiveAnchor", "anchor"),
        ("$anchor", "here"),
        ("$vocabulary", {"https://json-schema.org/draft/2020-12/vocab/core": True}),
        ("definitions", {"a": {"type": "string"}}),
        ("dependencies", {"a": ["b"]}),
    ],
)
def test_unsupported_keywords_are_recorded_and_mark_partial_support(
    loader: SchemaLoader, keyword: str, value: Any
) -> None:
    inner = {"type": "integer", keyword: value, "x-keep": 1}
    schema = {"$schema": D2020, "properties": {"p": {"items": inner}}}
    model = loader.load(schema)
    assert model.unsupported == [("/properties/p/items", keyword)]
    assert not model.fully_supported
    assert model.ignored == []
    assert model.resolve("/properties/p/items").kw == {"type": "integer", "x-keep": 1}


def test_keywords_of_the_declared_draft_are_unsupported(loader: SchemaLoader) -> None:
    schema = {"$schema": D7, "contentMediaType": "text/plain", "unevaluatedProperties": False, "type": "string"}
    model = loader.load(schema)
    assert model.unsupported == [("", "contentMediaType"), ("", "unevaluatedProperties")]
    assert model.root.kw == {"type": "string"}


def test_records_do_not_depend_on_member_order(loader: SchemaLoader) -> None:
    schema = {
        "zeta": 1,
        "multipleOf": 3,
        "properties": {"b": {"alpha": 1, "minContains": 1}, "a": {"beta": 2, "$anchor": "x"}},
        "alpha": 2,
    }

    def reversed_members(value: Any) -> Any:
        if isinstance(value, dict):
            return {k: reversed_members(value[k]) for k in reversed(list(value))}
        return value

    first, second = loader.load(schema), loader.load(reversed_members(schema))
    assert first == second
    ignored = [("", "alpha"), ("", "zeta"), ("/properties/a", "beta"), ("/properties/b", "alpha")]
    unsupported = [("", "multipleOf"), ("/properties/a", "$anchor"), ("/properties/b", "minContains")]
    assert first.ignored == second.ignored == ignored
    assert first.unsupported == second.unsupported == unsupported


# ---------------------------------------------------------------------------
# Draft translation (2.5)
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("declared", TRANSLATED)
def test_definitions_become_defs_and_refs_follow(loader: SchemaLoader, declared: str) -> None:
    schema = {
        "$schema": declared,
        "properties": {"p": {"$ref": "#/definitions/a"}, "q": {"$ref": "#/definitions/a/properties/b"}},
        "definitions": {"a": {"properties": {"b": {"type": "string"}}, "markdownDescription": "m"}},
    }
    model = loader.load(schema)
    assert model.root.to_json() == {
        "properties": {"p": {"$ref": "#/$defs/a"}, "q": {"$ref": "#/$defs/a/properties/b"}},
        "$defs": {"a": {"properties": {"b": {"type": "string"}}}},
    }
    assert model.fully_supported
    assert model.ignored == [("/definitions/a", "markdownDescription")]  # source pointers


@pytest.mark.parametrize("declared", TRANSLATED)
def test_definitions_next_to_defs_are_unsupported(loader: SchemaLoader, declared: str) -> None:
    schema = {
        "$schema": declared,
        "properties": {"p": {"$ref": "#/definitions/a"}, "q": {"$ref": "#/$defs/a"}},
        "definitions": {"a": {"type": "string"}},
        "$defs": {"a": {"type": "integer"}},
    }
    model = loader.load(schema)
    assert model.unsupported == [("", "definitions"), ("/properties/p", "$ref")]
    assert model.root.to_json() == {
        "properties": {"p": {}, "q": {"$ref": "#/$defs/a"}},
        "$defs": {"a": {"type": "integer"}},
    }


@pytest.mark.parametrize("declared", TRANSLATED)
def test_array_items_become_prefix_items_and_additional_items_become_items(
    loader: SchemaLoader, declared: str
) -> None:
    schema = {
        "$schema": declared,
        "items": [{"type": "string", "markdownDescription": "m"}, {"$ref": "#/items/0"}],
        "additionalItems": {"$ref": "#/properties/n"},
        "properties": {"n": {"type": "number"}, "t": {"$ref": "#/additionalItems"}},
    }
    model = loader.load(schema)
    assert model.root.to_json() == {
        "prefixItems": [{"type": "string"}, {"$ref": "#/prefixItems/0"}],
        "items": {"$ref": "#/properties/n"},
        "properties": {"n": {"type": "number"}, "t": {"$ref": "#/items"}},
    }
    assert model.fully_supported
    assert model.ignored == [("/items/0", "markdownDescription")]


@pytest.mark.parametrize("declared", TRANSLATED)
@pytest.mark.parametrize("items", [None, {"type": "integer"}])
def test_additional_items_without_array_items_is_ignored(loader: SchemaLoader, declared: str, items: Any) -> None:
    schema: dict[str, Any] = {"$schema": declared, "additionalItems": False}
    if items is not None:
        schema["items"] = items
    model = loader.load(schema)
    assert model.ignored == [("", "additionalItems")]
    assert model.root.to_json() == ({} if items is None else {"items": items})


@pytest.mark.parametrize("declared", TRANSLATED)
def test_prefix_items_next_to_array_items_is_ignored(loader: SchemaLoader, declared: str) -> None:
    schema = {"$schema": declared, "prefixItems": [{"type": "null"}], "items": [{"type": "string"}]}
    model = loader.load(schema)
    assert model.ignored == [("", "prefixItems")]
    assert model.root.to_json() == {"prefixItems": [{"type": "string"}]}


@pytest.mark.parametrize("declared", TRANSLATED)
def test_array_dependencies_become_dependent_required(loader: SchemaLoader, declared: str) -> None:
    schema = {
        "$schema": declared,
        "properties": {
            "arrays": {"dependencies": {"a": ["b", "c"], "d": ["e"]}},
            "mixed": {"dependencies": {"a": ["b"], "c": {"required": ["d"]}, "e": {"minProperties": 2}}},
            "schemas": {"dependencies": {"c": {"required": ["d"]}}},
            "empty": {"dependencies": {}},
        },
    }
    model = loader.load(schema)
    assert model.root.to_json() == {
        "properties": {
            "arrays": {"dependentRequired": {"a": ["b", "c"], "d": ["e"]}},
            "mixed": {"dependentRequired": {"a": ["b"]}},
            "schemas": {},
            "empty": {"dependentRequired": {}},
        }
    }
    # Schema-valued entries are recorded once per object, at the object.
    assert model.unsupported == [("/properties/mixed", "dependencies"), ("/properties/schemas", "dependencies")]
    assert model.ignored == []


@pytest.mark.parametrize("declared", [D4, D6, D7])
def test_dependent_required_next_to_dependencies_is_ignored_where_undefined(
    loader: SchemaLoader, declared: str
) -> None:
    schema = {"$schema": declared, "dependentRequired": {"x": ["y"]}, "dependencies": {"a": ["b"]}}
    model = loader.load(schema)
    assert model.root.to_json() == {"dependentRequired": {"a": ["b"]}}
    assert (model.ignored, model.unsupported) == ([("", "dependentRequired")], [])


def test_dependencies_next_to_dependent_required_are_unsupported_in_2019_09(loader: SchemaLoader) -> None:
    schema = {"$schema": D2019, "dependentRequired": {"x": ["y"]}, "dependencies": {"a": ["b"]}}
    model = loader.load(schema)
    assert model.root.to_json() == {"dependentRequired": {"x": ["y"]}}
    assert (model.ignored, model.unsupported) == ([], [("", "dependencies")])


def test_draft4_id_becomes_id(loader: SchemaLoader) -> None:
    schema = {
        "$schema": D4,
        "id": "https://example.com/root.json",
        "properties": {
            "anchor": {"id": "#here", "type": "string"},
            "resource": {"id": "item.json", "properties": {"r": {"$ref": "#/definitions/a"}}},
        },
        "definitions": {"a": {"type": "string"}},
    }
    model = loader.load(schema)
    assert model.root.kw["$id"] == "https://example.com/root.json"
    assert model.resolve("/properties/anchor").kw == {"type": "string"}
    # Below the root, the renamed keyword is unsupported under its source name;
    # a non-fragment id starts an embedded resource, so references inside it are unsupported.
    assert model.unsupported == [
        ("/properties/anchor", "id"),
        ("/properties/resource", "id"),
        ("/properties/resource/properties/r", "$ref"),
    ]
    assert model.ignored == []


@pytest.mark.parametrize(
    ("declared", "ignored", "unsupported"),
    [
        (D4, [("", "$id")], [("/items", "$id"), ("/items", "id")]),  # draft-04 does not define $id
        (D6, [("", "id"), ("/items", "id")], [("/items", "$id")]),  # nor draft-06 id
    ],
)
def test_id_and_dollar_id_side_by_side(
    loader: SchemaLoader, declared: str, ignored: list[tuple[str, str]], unsupported: list[tuple[str, str]]
) -> None:
    schema = {
        "$schema": declared,
        "$id": "https://a.example/",
        "id": "https://b.example/",
        "items": {"$id": "#x", "id": "#y"},
    }
    model = loader.load(schema)
    kept = "https://b.example/" if declared == D4 else "https://a.example/"
    assert model.root.to_json() == {"$id": kept, "items": {}}
    assert (model.ignored, model.unsupported) == (ignored, unsupported)


@pytest.mark.parametrize(
    ("bounds", "expected"),
    [
        ({"minimum": 1, "exclusiveMinimum": True}, {"exclusiveMinimum": 1}),
        ({"minimum": 1, "exclusiveMinimum": False}, {"minimum": 1}),
        ({"maximum": 2.5, "exclusiveMaximum": True}, {"exclusiveMaximum": 2.5}),
        ({"maximum": 2, "exclusiveMaximum": False}, {"maximum": 2}),
        (
            {"exclusiveMaximum": True, "exclusiveMinimum": False, "maximum": 9, "minimum": 0},
            {"exclusiveMaximum": 9, "minimum": 0},
        ),
    ],
)
def test_draft4_boolean_exclusive_bounds_become_numeric(
    loader: SchemaLoader, bounds: dict[str, Any], expected: dict[str, Any]
) -> None:
    model = loader.load({"$schema": D4, "properties": {"n": bounds}})
    assert model.resolve("/properties/n").kw == expected
    assert (model.ignored, model.unsupported) == ([], [])


KNOWN_FORMATS = (
    "date", "date-time", "time", "duration", "email", "idn-email", "hostname", "idn-hostname", "ipv4", "ipv6",
    "uri", "uri-reference", "iri", "iri-reference", "uri-template", "json-pointer", "relative-json-pointer",
    "regex", "uuid", "not-a-format",
)  # fmt: skip
# Strings that every format jsonschema-rs asserts rejects at least one of.
FORMAT_PROBES = ("\x00not valid [ ( :: %%", "x", "", "{", "a b", "1", "%zz", "\\", "[", "-")


def _asserts_format(validator: Validator, schema: dict[str, Any]) -> bool:
    compiled = validator.compile(schema)
    return not all(validator.is_valid(compiled, probe) for probe in FORMAT_PROBES)


@pytest.mark.parametrize("declared", TRANSLATED)
@pytest.mark.parametrize("fmt", KNOWN_FORMATS)
def test_formats_asserted_only_from_a_later_draft_are_ignored(loader: SchemaLoader, declared: str, fmt: str) -> None:
    # Pins the loader's table against jsonschema-rs: a format that the declared
    # draft does not assert but draft 2020-12 does would change verdicts (2.9).
    validator = Validator()
    later_only = _asserts_format(validator, {"format": fmt}) and not _asserts_format(
        validator, {"$schema": declared, "format": fmt}
    )
    model = loader.load({"$schema": declared, "properties": {"s": {"format": fmt}}})
    assert model.fully_supported
    if later_only:
        assert model.resolve("/properties/s").kw == {}
        assert model.ignored == [("/properties/s", "format")]
    else:
        assert model.resolve("/properties/s").kw == {"format": fmt}
        assert model.ignored == []


def test_some_formats_are_asserted_only_from_a_later_draft(loader: SchemaLoader) -> None:
    for declared, fmt in [(D4, "uri-reference"), (D6, "iri"), (D7, "uuid"), (D7, "duration")]:
        assert loader.load({"$schema": declared, "format": fmt}).ignored == [("", "format")]
    assert loader.load({"$schema": D2019, "format": "uuid"}).root.kw == {"format": "uuid"}


@pytest.mark.parametrize("declared", [D4, D6, D7])
def test_ref_siblings_are_ignored_in_drafts_04_to_07(loader: SchemaLoader, declared: str) -> None:
    schema = {
        "$schema": declared,
        "$ref": "#/definitions/a",
        "type": "integer",
        "description": "hidden by $ref",
        "x-note": 1,
        "definitions": {"a": {"type": "string"}},
        "properties": {
            "p": {
                "$ref": "#/definitions/a",
                "$schema": declared,
                "id" if declared == D4 else "$id": "https://example.com/p",
                "maxLength": 1,
                "items": [{"type": "string"}],
                "additionalItems": False,
                "definitions": {"b": {"type": "null"}},
            },
            "q": {"$ref": "#/properties/p/definitions/b"},
        },
    }
    model = loader.load(schema)
    # The ref-target containers (definitions, $defs) and the root $schema are kept.
    assert model.root.to_json() == {"$ref": "#/$defs/a", "$defs": {"a": {"type": "string"}}}
    assert model.ignored == [
        ("", "description"),
        ("", "properties"),
        ("", "type"),
        ("", "x-note"),
    ]
    assert model.fully_supported
    nested = loader.load({"$schema": declared, "properties": schema["properties"]})
    assert nested.root.to_json() == {
        "properties": {
            "p": {"$defs": {"b": {"type": "null"}}},
            "q": {"$ref": "#/properties/p/$defs/b"},
        }
    }
    id_keyword = "id" if declared == D4 else "$id"
    assert nested.ignored == sorted(
        ("/properties/p", keyword) for keyword in ["$schema", id_keyword, "additionalItems", "items", "maxLength"]
    )
    # #/definitions/a does not resolve here: there is no root definitions.
    assert nested.unsupported == [("/properties/p", "$ref")]


def test_ref_siblings_are_kept_in_2019_09(loader: SchemaLoader) -> None:
    schema = {
        "$schema": D2019,
        "properties": {"p": {"$ref": "#/definitions/a", "maxLength": 1, "items": [{"type": "string"}]}},
        "definitions": {"a": {"type": "string"}},
    }
    model = loader.load(schema)
    assert model.resolve("/properties/p").to_json() == {
        "$ref": "#/$defs/a",
        "maxLength": 1,
        "prefixItems": [{"type": "string"}],
    }
    assert (model.ignored, model.unsupported) == ([], [])


@pytest.mark.parametrize(
    ("declared", "ref"),
    [
        (D7, "#/definitions"),  # a container, not a schema
        (D7, "#/items"),  # the array of array-form items, not a schema
        (D7, "#/additionalItems"),  # ignored: no array-form items here
        (D7, "#/prefixItems/0"),  # not a draft-07 keyword
        (D7, "#/dependencies/c"),  # schema-valued dependencies are unsupported
        (D7, "#/properties/hidden/properties/x"),  # hidden by a $ref next to it
        (D2019, "#/definitions/a/items"),  # array-form items moved to prefixItems
    ],
)
def test_refs_whose_target_is_not_translated_are_unsupported(loader: SchemaLoader, declared: str, ref: str) -> None:
    schema = {
        "$schema": declared,
        "items": [{"type": "string"}] if ref != "#/additionalItems" else {"type": "string"},
        "additionalItems": {"type": "integer"},
        "prefixItems": [{"type": "null"}],
        "dependencies": {"c": {"required": ["d"]}},
        "definitions": {"a": {"items": [{"type": "string"}]}},
        "properties": {
            "hidden": {"$ref": "#/definitions/a", "properties": {"x": {"type": "string"}}},
            "r": {"$ref": ref},
        },
    }
    model = loader.load(schema)
    assert ("/properties/r", "$ref") in model.unsupported
    assert model.resolve("/properties/r").kw == {}


def test_moved_refs_are_rewritten_as_uri_references(loader: SchemaLoader) -> None:
    name = "a b~c/d%é"
    defs = {name: {"type": "string"}, "plain": {"type": "integer"}}
    schema = {
        "$schema": D7,
        "properties": {
            "encoded": {"$ref": "#/definitions/a%20b~0c~1d%25%C3%A9"},
            "slashes": {"$ref": "#%2Fdefinitions%2Fplain"},  # %2F separates tokens too
            "spelled": {"$ref": "#/properties/%65ncoded"},  # unchanged target: kept as written
        },
        "definitions": defs,
    }
    model = loader.load(schema)
    assert model.fully_supported
    assert model.resolve("/properties/encoded").kw == {"$ref": "#/$defs/a%20b~0c~1d%25%C3%A9"}
    assert model.resolve("/properties/slashes").kw == {"$ref": "#/$defs/plain"}
    assert model.resolve("/properties/spelled").kw == {"$ref": "#/properties/%65ncoded"}
    # The emitted 2020-12 document passes its meta-schema and loads back to the same model.
    emitted, _ = SchemaEmitter().layout(model)
    assert loader.load(emitted) == model


def test_translation_does_not_depend_on_member_order(loader: SchemaLoader) -> None:
    schema = {
        "$schema": D4,
        "prefixItems": [{"type": "null"}],
        "items": [{"type": "string"}],
        "exclusiveMinimum": True,
        "minimum": 3,
        "dependentRequired": {"x": ["y"]},
        "dependencies": {"a": ["b"], "c": {"required": ["d"]}},
        "$id": "https://a.example/",
        "id": "https://b.example/",
    }
    reverse = {key: schema[key] for key in reversed(list(schema))}
    first, second = loader.load(schema), loader.load(reverse)
    assert first == second
    assert first.root.kw.keys() == {"prefixItems", "exclusiveMinimum", "dependentRequired", "$id"}
    assert (first.ignored, first.unsupported) == (second.ignored, second.unsupported)
    assert first.ignored == [("", "$id"), ("", "dependentRequired"), ("", "prefixItems")]
    assert first.unsupported == [("", "dependencies")]


def test_translation_leaves_the_source_unchanged(loader: SchemaLoader) -> None:
    schema = {
        "$schema": D7,
        "items": [{"$ref": "#/definitions/a"}],
        "additionalItems": {"enum": [[1]]},
        "dependencies": {"a": ["b"]},
        "definitions": {"a": {"$ref": "#/additionalItems", "type": "string"}},
    }
    original = copy.deepcopy(schema)
    model = loader.load(schema)
    assert schema == original
    schema["dependencies"]["a"].append("c")
    schema["additionalItems"]["enum"][0].append(2)
    assert model.root.kw["dependentRequired"] == {"a": ["b"]}
    assert model.resolve("/items").kw == {"enum": [[1]]}


# Each case: a schema in an older draft and instances whose verdicts tell its
# translated and untranslated readings apart.
VERDICT_CASES: list[tuple[dict[str, Any], list[Any]]] = [
    (
        {"$schema": D7, "$ref": "#/definitions/a", "type": "integer", "definitions": {"a": {"type": "string"}}},
        ["x", 1],
    ),
    (
        {
            "$schema": D7,
            "items": [{"type": "string"}, {"$ref": "#/items/0"}],
            "additionalItems": {"$ref": "#/definitions/n"},
            "definitions": {"n": {"type": "number"}},
        },
        [[], ["a"], ["a", "b"], ["a", "b", 1], ["a", "b", "c"], ["a", 1]],
    ),
    ({"$schema": D7, "items": {"type": "integer"}, "additionalItems": False}, [[1, 2], ["a"]]),
    ({"$schema": D6, "additionalItems": False, "maxItems": 2}, [[1, 2], [1, 2, 3]]),
    (
        {"$schema": D4, "minimum": 1, "exclusiveMinimum": True, "maximum": 5, "exclusiveMaximum": False},
        [0, 1, 2, 5, 6],
    ),
    (
        {"$schema": D4, "id": "https://example.com/s.json", "dependencies": {"a": ["b", "c"]}},
        [{"a": 1}, {"a": 1, "b": 1}, {"a": 1, "b": 1, "c": 1}, {"b": 1}],
    ),
    (
        {
            "$schema": D6,
            "properties": {"p": {"$id": "https://example.com/p", "$ref": "#/definitions/s", "maxLength": 1}},
            "definitions": {"s": {"type": "string"}},
        },
        [{"p": "ab"}, {"p": 1}],
    ),
    (
        {
            "$schema": D7,
            "properties": {"p": {"$ref": "#/definitions/a%20b"}},
            "definitions": {"a b": {"type": "string"}},
        },
        [{"p": "x"}, {"p": 1}],
    ),
    (
        {
            "$schema": D2019,
            "properties": {"n": {"$ref": "#/definitions/pos", "type": "integer"}},
            "definitions": {"pos": {"minimum": 0}},
            "dependencies": {"n": ["m"]},
            "items": [{"type": "object"}],
            "additionalItems": False,
        },
        [{"n": 1, "m": 0}, {"n": 1.5, "m": 0}, {"n": -1, "m": 0}, {"n": 1}, [{}], [{}, {}]],
    ),
    (
        {"$schema": D7, "properties": {"id": {"format": "uuid"}, "at": {"format": "date"}}},
        [{"id": "123e4567"}, {"at": "2023-02-29"}],
    ),
]


@pytest.mark.parametrize(("schema", "instances"), VERDICT_CASES)
def test_translated_schemas_keep_their_verdicts(
    loader: SchemaLoader, schema: dict[str, Any], instances: list[Any]
) -> None:
    validator = Validator()
    model = loader.load(schema)
    assert model.fully_supported
    emitted, _ = SchemaEmitter().layout(model)
    original, translated = validator.compile(schema), validator.compile(emitted)
    verdicts = [validator.is_valid(original, instance) for instance in instances]
    assert verdicts == [validator.is_valid(translated, instance) for instance in instances]
    assert True in verdicts and False in verdicts


# ---------------------------------------------------------------------------
# Sources and the resulting model
# ---------------------------------------------------------------------------


def test_bytes_text_and_parsed_sources_load_equal_models(loader: SchemaLoader) -> None:
    schema = {"$schema": D2020, "properties": {"n": {"enum": [1, 2**70]}}, "markdownDescription": "é"}
    text = json.dumps(schema, ensure_ascii=False)
    models = [
        loader.load(schema),
        loader.load(text),
        loader.load(text.encode("utf-8")),
        loader.load(b"\xef\xbb\xbf" + text.encode("utf-8")),
    ]
    for model in models:
        assert model == models[0]
        assert (model.ignored, model.unsupported, model.dialect_assumed) == ([("", "markdownDescription")], [], False)
    assert models[1].resolve("/properties/n").kw["enum"][1] == 2**70  # big integers stay exact


def test_model_does_not_share_data_with_a_parsed_source(loader: SchemaLoader) -> None:
    schema = {"properties": {"p": {"enum": [1, 2], "x-meta": {"k": [1]}}}}
    original = copy.deepcopy(schema)
    model = loader.load(schema)
    schema["properties"]["p"]["enum"].append(3)
    schema["properties"]["p"]["x-meta"]["k"].append(2)
    assert model.root.to_json() == original


@pytest.mark.parametrize("source", [{1, 2}, Path("schema.json"), object()])
def test_other_source_types_are_rejected(loader: SchemaLoader, source: Any) -> None:
    with pytest.raises(TypeError):
        loader.load(source)


def test_nids_follow_canonical_preorder(loader: SchemaLoader) -> None:
    model = loader.load(ALL_SUPPORTED)
    nids = [node.nid for _, node in model.walk()]
    assert nids == sorted(nids) and len(set(nids)) == len(nids)


def test_loader_accepts_a_shared_validator() -> None:
    validator = Validator()
    model = SchemaLoader(validator).load({"type": "string"})
    assert isinstance(model, SchemaModel) and model.root.kw == {"type": "string"}
    with pytest.raises(TypeError):
        SchemaLoader("not a validator")  # type: ignore[arg-type]
