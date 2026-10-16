"""Unit tests for pipeline/naive_merge.py, the Naive_Merge baseline (task 7.2, Requirement 34.4).

Emitted documents are checked against a direct reading of Requirement 34.4
(:func:`reference`), for the keyword whitelist, and for acceptance of every
input document by the Validator with ``format`` asserted.
"""

from __future__ import annotations

import sys
from collections.abc import Iterator
from typing import Any

import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.paths import MAPVAL_STEP, parse_path
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.io.emit import SCHEMA_DIALECT, SchemaEmitter
from jsinfer.io.validator import Draft, Validator
from jsinfer.pipeline.naive_merge import (
    NAIVE_MERGE_KEYWORDS,
    naive_merge,
    naive_merge_model,
    naive_merge_node,
    naive_merge_values,
    naive_type_names,
)
from tests.strategies import json_documents

EMITTER = SchemaEmitter()
VALIDATOR = Validator()

# Emitted type order (Requirement 34.4 lists the seven JSON types in this order).
TYPE_ORDER = ("null", "boolean", "integer", "number", "string", "array", "object")

# Every JSON type, nested arrays of objects and of arrays, empty containers,
# integral floats, a big integer, and keys that need escaping or sort apart
# in code-point and UTF-16 order.
WIDE_DOCS: list[Any] = [
    {"id": 1, "name": "ann", "tags": ["x", 2], "meta": {"k": True, "": None}, "😀": 1e3},
    {"id": 2.0, "name": None, "tags": [], "meta": {}, "\uffff": [[1.5], []]},
    {"id": 2**70, "tags": [{"a b": "s"}, {"a b": 1, "/": [None]}], "meta": {"k": False}},
    [[], [{"id": -0.0}], "s", 0.5],
    "plain",
    3,
    None,
    True,
    [],
]


def emitted(docs: list[Any], chunk_size: int = 2000) -> dict[str, Any]:
    """The emitted Naive_Merge document of ``docs``."""
    doc, _ = EMITTER.layout(naive_merge(docs, chunk_size))
    assert isinstance(doc, dict)
    return doc


def as_emitted(node: SchemaNode) -> dict[str, Any]:
    """A subschema in emitted form (a single type as a string), without the root ``$schema``."""
    doc, _ = EMITTER.layout(SchemaModel(root=node))
    assert isinstance(doc, dict) and doc.pop("$schema") == SCHEMA_DIALECT
    return doc


def type_name(value: Any) -> str:
    """The JSON type of a value, with the integer rule of Requirement 13.2 (zero fraction is integer)."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int) or (isinstance(value, float) and value.is_integer()):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    return "array" if isinstance(value, list) else "object"


def reference(values: list[Any]) -> dict[str, Any]:
    """Requirement 34.4 applied directly to the values observed at one Path, in emitted form."""
    if not values:
        return {}
    names = {type_name(value) for value in values}
    if "number" in names:
        names.discard("integer")
    types = [name for name in TYPE_ORDER if name in names]
    schema: dict[str, Any] = {"type": types[0] if len(types) == 1 else types}
    objects = [value for value in values if isinstance(value, dict)]
    keys = sorted({key for obj in objects for key in obj})  # str order is code-point order
    if keys:
        schema["properties"] = {key: reference([obj[key] for obj in objects if key in obj]) for key in keys}
        required = [key for key in keys if all(key in obj for obj in objects)]
        if required:
            schema["required"] = required
    elements = [item for value in values if isinstance(value, list) for item in value]
    if elements:
        schema["items"] = reference(elements)
    return schema


def schema_objects(schema: dict[str, Any], pointer: str = "") -> Iterator[tuple[str, Any]]:
    """``(pointer, subschema)`` for every subschema reached through ``properties`` and ``items``."""
    yield pointer, schema
    for name, child in schema.get("properties", {}).items():
        yield from schema_objects(child, f"{pointer}/properties/{name}")
    if "items" in schema:
        yield from schema_objects(schema["items"], f"{pointer}/items")


def assert_naive_keywords(doc: dict[str, Any]) -> None:
    """Only the five Naive_Merge keywords appear, ``$schema`` at the root only, never integer with number."""
    assert doc["$schema"] == SCHEMA_DIALECT
    for pointer, schema in schema_objects(doc):
        assert isinstance(schema, dict), pointer
        assert set(schema) <= set(NAIVE_MERGE_KEYWORDS), pointer
        if pointer:
            assert "$schema" not in schema, pointer
        types = schema.get("type", [])
        types = [types] if isinstance(types, str) else types
        assert not {"integer", "number"} <= set(types), pointer


def assert_accepts(docs: list[Any]) -> None:
    """The emitted schema is a valid 2020-12 schema that accepts every document, formats asserted."""
    doc = emitted(docs)
    assert VALIDATOR.meta_errors(doc, Draft.DRAFT202012) == []
    compiled = VALIDATOR.compile(doc, name="naive-merge")
    for index, document in enumerate(docs):
        assert VALIDATOR.errors(compiled, document) == [], index


# -- the keyword whitelist ---------------------------------------------------


def test_keywords_are_those_of_requirement_34_4() -> None:
    assert NAIVE_MERGE_KEYWORDS == ("$schema", "type", "properties", "items", "required")


def test_only_whitelisted_keywords_appear() -> None:
    doc = emitted(WIDE_DOCS)
    assert_naive_keywords(doc)
    # Every whitelisted keyword is exercised by this collection.
    used = {keyword for _, schema in schema_objects(doc) for keyword in schema}
    assert used == set(NAIVE_MERGE_KEYWORDS)


def test_a_small_collection() -> None:
    docs = [
        {"id": 1, "name": "ann", "tags": ["x", 2], "note": None},
        {"id": 2.0, "name": None, "tags": []},
        {"id": 3, "name": "bo", "tags": ["y"], "extra": {"k": True}},
    ]
    assert emitted(docs) == {
        "$schema": SCHEMA_DIALECT,
        "type": "object",
        "properties": {
            "extra": {"type": "object", "properties": {"k": {"type": "boolean"}}, "required": ["k"]},
            "id": {"type": "integer"},
            "name": {"type": ["null", "string"]},
            "note": {"type": "null"},
            "tags": {"type": "array", "items": {"type": ["integer", "string"]}},
        },
        "required": ["id", "name", "tags"],
    }


@given(json_documents())
def test_matches_a_direct_reading_of_the_requirement(docs: list[Any]) -> None:
    doc = emitted(docs, chunk_size=3)
    assert doc == {"$schema": SCHEMA_DIALECT, **reference(docs)}
    assert_naive_keywords(doc)


# -- the integer rule --------------------------------------------------------


@pytest.mark.parametrize(
    ("values", "expected"),
    [
        ([1], "integer"),
        ([1.0], "integer"),
        ([1e3], "integer"),
        ([1, 1.0, 1e3, -0.0, 2**70], "integer"),
        ([2.5], "number"),
        ([1, 2.5], "number"),
        ([1e3, -0.5, 7], "number"),
        ([True, False], "boolean"),
        ([True, 1], ["boolean", "integer"]),
        ([False, 0.5], ["boolean", "number"]),
        ([None, 1.0, "1"], ["null", "integer", "string"]),
    ],
)
def test_integer_rule(values: list[Any], expected: str | list[str]) -> None:
    assert as_emitted(naive_merge_values(values)) == {"type": expected}


def test_integer_rule_applies_per_path() -> None:
    doc = emitted([{"n": 1, "m": 1, "a": [1, 2.0]}, {"n": 1.5, "m": 1e3, "a": [1e3]}])
    assert doc["properties"]["n"] == {"type": "number"}
    assert doc["properties"]["m"] == {"type": "integer"}
    assert doc["properties"]["a"] == {"type": "array", "items": {"type": "integer"}}


def test_naive_type_names() -> None:
    def names(values: list[Any]) -> list[str]:
        return naive_type_names(View.root(build_store(values)).stats())

    assert names([]) == []
    assert names([1, 1.0, 1e3]) == ["integer"]
    assert names(["s", 2.5, 1, None, True]) == ["null", "boolean", "number", "string"]
    assert names([[], {}]) == ["array", "object"]


# -- properties, required, and items -----------------------------------------


def test_required_lists_the_keys_of_every_object() -> None:
    # Null values count as present; non-object values at the Path do not count.
    docs = [{"a": None, "b": 1, "c": 1}, {"a": None, "c": None}, "not an object", [{"d": 1}]]
    doc = emitted(docs)
    assert doc["required"] == ["a", "c"]
    assert doc["properties"] == {"a": {"type": "null"}, "b": {"type": "integer"}, "c": {"type": ["null", "integer"]}}
    assert doc["items"] == {"type": "object", "properties": {"d": {"type": "integer"}}, "required": ["d"]}


def test_no_required_when_no_key_is_in_every_object() -> None:
    doc = emitted([{"a": 1}, {"b": 2}, {}])
    assert set(doc["properties"]) == {"a", "b"} and "required" not in doc


def test_objects_without_keys_have_no_properties() -> None:
    assert emitted([{}, {}]) == {"$schema": SCHEMA_DIALECT, "type": "object"}


def test_items_merges_every_element_of_every_array() -> None:
    doc = emitted([[1, "a"], [{"k": 1}], [[], [2.5]], []])
    assert doc == {
        "$schema": SCHEMA_DIALECT,
        "type": "array",
        "items": {
            "type": ["integer", "string", "array", "object"],
            "properties": {"k": {"type": "integer"}},
            "required": ["k"],
            "items": {"type": "number"},
        },
    }


def test_empty_arrays_emit_no_items() -> None:
    assert emitted([[], {"a": []}, {"a": [[]]}]) == {
        "$schema": SCHEMA_DIALECT,
        "type": ["array", "object"],
        "properties": {"a": {"type": "array", "items": {"type": "array"}}},
        "required": ["a"],
    }


def test_an_empty_collection_gives_the_empty_schema() -> None:
    model = naive_merge([])
    assert model.root.boolean is None and model.root.kw == {}
    assert model.fully_supported and not model.dialect_assumed and model.provenance == {}
    assert emitted([]) == {"$schema": SCHEMA_DIALECT}
    assert naive_merge_values([]).kw == {}


# -- acceptance of every input document --------------------------------------


def test_accepts_every_input_document() -> None:
    assert_accepts(WIDE_DOCS)


@given(json_documents())
def test_accepts_every_document_of_any_collection(docs: list[Any]) -> None:
    assert_accepts(docs)


# -- determinism ---------------------------------------------------------------


@given(json_documents(), st.randoms(use_true_random=False), st.integers(min_value=1, max_value=4))
def test_bytes_do_not_depend_on_document_order_or_chunking(docs: list[Any], random, chunk_size: int) -> None:
    shuffled = list(docs)
    random.shuffle(shuffled)
    assert EMITTER.emit(naive_merge(shuffled, chunk_size)) == EMITTER.emit(naive_merge(docs))


def test_nodes_are_fresh_and_numbered_in_canonical_pre_order() -> None:
    first, second = naive_merge(WIDE_DOCS), naive_merge(WIDE_DOCS)
    nids = [node.nid for _, node in first.walk()]
    assert nids == sorted(set(nids))
    assert not set(nids) & {node.nid for _, node in second.walk()}
    assert first == second


def test_deep_documents_do_not_hit_the_recursion_limit() -> None:
    depth = sys.getrecursionlimit() + 100
    deep: list[Any] = []
    for _ in range(depth):
        deep = [deep]
    nodes = [node for _, node in naive_merge([deep]).walk()]
    assert len(nodes) == depth + 1
    assert all(node.kw["type"] == ["array"] for node in nodes)
    assert "items" not in nodes[-1].kw  # the innermost array is empty


# -- per-view subschemas --------------------------------------------------------


def test_node_of_a_view_equals_the_merge_of_its_values() -> None:
    docs = [{"a": [1, {"b": "x"}], "c": 1}, {"a": [2.5]}, {"a": "s"}, 7]
    store = build_store(docs)
    root = View.root(store)
    a = root.child_key("a")
    assert naive_merge_node(a) == naive_merge_values([[1, {"b": "x"}], [2.5], "s"])

    elements = root.descend(parse_path("$.a[*]"))
    assert as_emitted(naive_merge_node(elements)) == {
        "type": ["number", "object"],
        "properties": {"b": {"type": "string"}},
        "required": ["b"],
    }
    assert naive_merge_node(elements) == naive_merge_values([1, {"b": "x"}, 2.5])
    assert naive_merge_node(a.elements(0)) == naive_merge_values([1, 2.5])  # a Tuple position
    assert naive_merge_node(root.child_map()) == naive_merge_values([[1, {"b": "x"}], 1, [2.5], "s"])
    union = View.union([a, root.child_key("c")])
    assert naive_merge_node(union) == naive_merge_values([[1, {"b": "x"}], [2.5], "s", 1])
    assert naive_merge_node(root.child_key("missing")).kw == {}


def test_model_of_a_store_or_a_document_subset() -> None:
    store = build_store(WIDE_DOCS)
    assert naive_merge_model(store) == naive_merge(WIDE_DOCS)
    assert naive_merge_model(View.root(store, docs=[1, 3])) == naive_merge([WIDE_DOCS[1], WIDE_DOCS[3]])


def test_other_sources_are_rejected() -> None:
    with pytest.raises(TypeError, match="takes a View"):
        naive_merge_node(build_store([1]))  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="ObservationStore or a View"):
        naive_merge_model([1])  # type: ignore[arg-type]


# -- flags -----------------------------------------------------------------------


def test_flagged_documents_are_left_out() -> None:
    docs = [{"a": 1, "b": "x"}, {"a": 2.5}, {"a": 3, "b": None}]
    store = build_store(docs)
    store.flags.flag_documents([1])
    model = naive_merge_model(store)
    assert model == naive_merge([docs[0], docs[2]])
    assert as_emitted(model.root)["required"] == ["a", "b"]


def test_flagged_items_affect_only_their_subtree() -> None:
    docs = [{"a": 2.5, "b": 1}, {"a": 2, "b": 2}]
    store = build_store(docs)
    first_a = View.root(store).child_key("a").parts[0].rows[:1]  # document 0's "a" value
    store.flags.flag_items((MAPVAL_STEP,), first_a)
    assert as_emitted(naive_merge_model(store).root) == {
        "type": "object",
        "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
        "required": ["a", "b"],
    }
    # With every value under "a" flagged, the key stays present and its subschema is {}.
    store.flags.flag_view(View.root(store).child_key("a"))
    assert as_emitted(naive_merge_model(store).root) == {
        "type": "object",
        "properties": {"a": {}, "b": {"type": "integer"}},
        "required": ["a", "b"],
    }
