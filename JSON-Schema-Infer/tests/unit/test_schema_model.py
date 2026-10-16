"""Unit tests for core/schema_model.py (task 2.4)."""

from __future__ import annotations

import copy
import json
import pickle
import random as _random
from dataclasses import FrozenInstanceError
from typing import Any

import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.core.schema_model import (
    SUBSCHEMA_KEYWORDS,
    ConstraintId,
    SchemaModel,
    SchemaNode,
    escape_token,
    join_pointer,
    new_nid,
    split_pointer,
)
from tests.strategies import json_scalars, json_values


def node(schema: Any) -> SchemaNode:
    return SchemaNode.from_json(schema)


def model(schema: Any, **kwargs: Any) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(schema), **kwargs)


def strict(value: Any) -> str:
    """A spelling that tells apart what Python's == conflates (True/1, key order)."""
    return json.dumps(value, ensure_ascii=False)


# ---------------------------------------------------------------------------
# SchemaNode construction
# ---------------------------------------------------------------------------


def test_boolean_and_keyword_nodes() -> None:
    t, f, empty = SchemaNode(True), SchemaNode(boolean=False), SchemaNode()
    assert (t.boolean, t.kw) == (True, {})
    assert (f.boolean, f.kw) == (False, {})
    assert (empty.boolean, empty.kw) == (None, {})
    child = SchemaNode(kw={"type": "string"})
    parent = SchemaNode(
        kw={"properties": {"a": child}, "items": SchemaNode(True), "anyOf": [SchemaNode(False)], "minimum": 0}
    )
    assert parent.kw["properties"]["a"] is child


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"boolean": True, "kw": {"type": "string"}}, ValueError),
        ({"boolean": 1}, TypeError),
        ({"kw": [("type", "string")]}, TypeError),
        ({"kw": {1: "x"}}, TypeError),
        ({"kw": {"items": {"type": "string"}}}, TypeError),  # raw JSON where a node belongs
        ({"kw": {"items": True}}, TypeError),  # boolean schemas are SchemaNode(True)
        ({"kw": {"anyOf": SchemaNode()}}, TypeError),
        ({"kw": {"anyOf": [SchemaNode(), {}]}}, TypeError),
        ({"kw": {"properties": [SchemaNode()]}}, TypeError),
        ({"kw": {"properties": {"a": {}}}}, TypeError),
        ({"kw": {"$defs": {1: SchemaNode()}}}, TypeError),
        ({"nid": True}, TypeError),
        ({"nid": 1.0}, TypeError),
        ({"nid": -1}, ValueError),
    ],
)
def test_invalid_nodes(kwargs: dict[str, Any], error: type[Exception]) -> None:
    with pytest.raises(error):
        SchemaNode(**kwargs)


def test_subschema_keyword_shapes() -> None:
    assert dict(SUBSCHEMA_KEYWORDS) == {
        "$defs": "map",
        "additionalProperties": "single",
        "allOf": "list",
        "anyOf": "list",
        "contains": "single",
        "else": "single",
        "if": "single",
        "items": "single",
        "not": "single",
        "oneOf": "list",
        "patternProperties": "map",
        "prefixItems": "list",
        "properties": "map",
        "propertyNames": "single",
        "then": "single",
    }


# ---------------------------------------------------------------------------
# nids
# ---------------------------------------------------------------------------


def test_new_nids_are_distinct_and_increasing() -> None:
    nids = [SchemaNode().nid for _ in range(5)] + [new_nid()]
    assert nids == sorted(set(nids))


def test_explicit_nids_are_reserved() -> None:
    big = new_nid() + 1000
    explicit = SchemaNode(nid=big)
    assert explicit.nid == big
    assert SchemaNode().nid > big


def test_pickle_and_deepcopy_keep_nids_and_reserve_them() -> None:
    tree = node({"properties": {"a": {"type": "string"}}, "items": True})
    nids = [n.nid for _, n in tree.walk()]
    for clone in (pickle.loads(pickle.dumps(tree)), copy.deepcopy(tree)):
        assert [n.nid for _, n in clone.walk()] == nids
        assert clone == tree and clone is not tree

    # A node whose nid this process has not seen (as from a worker) reserves it on unpickling.
    foreign = SchemaNode()
    foreign.nid = new_nid() + 5000  # bypasses reservation, as a nid made elsewhere would
    restored = pickle.loads(pickle.dumps(foreign))
    assert restored.nid == foreign.nid
    assert new_nid() > foreign.nid


def test_nids_do_not_affect_equality() -> None:
    a = SchemaNode(kw={"type": "string"}, nid=new_nid() + 10)
    b = SchemaNode(kw={"type": "string"})
    assert a.nid != b.nid and a == b


def test_from_json_assigns_nids_in_pre_order() -> None:
    tree = node({"properties": {"b": {}, "a": {"items": {}}}, "anyOf": [True, False]})
    walked = [(pointer, n.nid) for pointer, n in tree.walk()]
    creation = sorted(walked, key=lambda entry: entry[1])
    # Creation order is the document's insertion order; walk order is canonical.
    assert [pointer for pointer, _ in creation] == [
        "",
        "/properties/b",
        "/properties/a",
        "/properties/a/items",
        "/anyOf/0",
        "/anyOf/1",
    ]


def test_renumber_follows_canonical_pre_order_and_remaps_provenance() -> None:
    m = model({"required": ["a"], "properties": {"b": {}, "a": {"anyOf": [{}, {}]}}})
    root, a = m.root, m.resolve("/properties/a")
    stale = ConstraintId(new_nid(), "type")
    m.provenance = {
        ConstraintId(root.nid, "required", "a"): "r",
        ConstraintId(a.nid, "anyOf", 1): "branch",
        stale: "stale",
    }
    old = {pointer: n.nid for pointer, n in m.walk()}
    floor = new_nid()

    mapping = m.renumber()

    new = {pointer: n.nid for pointer, n in m.walk()}
    assert list(new) == ["", "/properties/a", "/properties/a/anyOf/0", "/properties/a/anyOf/1", "/properties/b"]
    assert list(new.values()) == sorted(new.values())
    assert min(new.values()) > floor
    assert mapping == {old[pointer]: new[pointer] for pointer in old}
    assert m.provenance == {
        ConstraintId(new[""], "required", "a"): "r",
        ConstraintId(new["/properties/a"], "anyOf", 1): "branch",
        stale: "stale",
    }
    assert len(m.index()) == 5


def test_renumber_and_index_reject_repeated_nids() -> None:
    a = SchemaNode(kw={"type": "string"})
    twin = SchemaNode(kw={"type": "integer"}, nid=a.nid)
    m = SchemaModel(SchemaNode(kw={"anyOf": [a, twin]}))
    before = [n.nid for _, n in m.walk()]
    with pytest.raises(ValueError, match=f"nid {a.nid} appears at '/anyOf/0' and at '/anyOf/1'"):
        m.index()
    with pytest.raises(ValueError):
        m.renumber()
    assert [n.nid for _, n in m.walk()] == before


def test_shared_and_cyclic_nodes_are_rejected() -> None:
    shared = SchemaNode()
    with pytest.raises(ValueError, match="reached twice"):
        list(SchemaNode(kw={"anyOf": [shared, shared]}).walk())
    cyclic = SchemaNode()
    cyclic.kw["items"] = cyclic
    with pytest.raises(ValueError, match="reached twice"):
        list(cyclic.walk())


# ---------------------------------------------------------------------------
# ConstraintId
# ---------------------------------------------------------------------------


def test_constraint_ids() -> None:
    cid = ConstraintId(17, "required", "email")
    assert (cid.nid, cid.keyword, cid.item) == (17, "required", "email")
    assert ConstraintId(3, "type").item is None
    assert ConstraintId(3, "anyOf", 0) == ConstraintId(3, "anyOf", 0)
    assert len({ConstraintId(3, "anyOf", 0), ConstraintId(3, "anyOf", 0), ConstraintId(3, "oneOf", 0)}) == 2
    with pytest.raises(FrozenInstanceError):
        cid.nid = 3  # type: ignore[misc]
    assert pickle.loads(pickle.dumps(cid)) == cid
    for keyword in ("x-latent-type", "$ref", "additionalProperties", "maximum", "not", "properties"):
        ConstraintId(1, keyword)
    ConstraintId(1, "dependentRequired", "a")


@pytest.mark.parametrize(
    ("args", "error"),
    [
        ((True, "type"), TypeError),
        ((1.0, "type"), TypeError),
        ((-1, "type"), ValueError),
        ((1, 7), TypeError),
        ((1, ""), ValueError),
        ((1, "$defs"), ValueError),
        ((1, "$schema"), ValueError),
        ((1, "title"), ValueError),
        ((1, "x-provenance"), ValueError),
        ((1, "properties", "name"), ValueError),  # container entries are not Constraints
        ((1, "type", 0), ValueError),
        ((1, "required"), ValueError),
        ((1, "required", 0), ValueError),
        ((1, "dependentRequired", None), ValueError),
        ((1, "anyOf", "0"), ValueError),
        ((1, "anyOf", True), ValueError),
        ((1, "oneOf", -1), ValueError),
    ],
)
def test_invalid_constraint_ids(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        ConstraintId(*args)


def test_constraint_id_order() -> None:
    ids = [
        ConstraintId(2, "type"),
        ConstraintId(1, "required", "b"),
        ConstraintId(1, "anyOf", 10),
        ConstraintId(1, "anyOf", 2),
        ConstraintId(1, "required", "B"),
        ConstraintId(1, "$ref"),
        ConstraintId(10, "enum"),
    ]
    assert sorted(ids) == [
        ConstraintId(1, "$ref"),
        ConstraintId(1, "anyOf", 2),
        ConstraintId(1, "anyOf", 10),
        ConstraintId(1, "required", "B"),
        ConstraintId(1, "required", "b"),
        ConstraintId(2, "type"),
        ConstraintId(10, "enum"),
    ]
    a, b = ConstraintId(1, "type"), ConstraintId(2, "type")
    assert a < b and a <= b and b > a and b >= a and a <= a and a >= a
    with pytest.raises(TypeError):
        _ = a < (1, "type", None)  # type: ignore[operator]


# ---------------------------------------------------------------------------
# Equality (Requirement 2.7)
# ---------------------------------------------------------------------------

EQUAL_PAIRS: list[tuple[Any, Any]] = [
    # Set-compared lists.
    ({"required": ["a", "b"]}, {"required": ["b", "a", "b"]}),
    ({"type": ["string", "null"]}, {"type": ["null", "string"]}),
    ({"type": "string"}, {"type": ["string"]}),
    ({"enum": [1, "x", None]}, {"enum": [None, "x", 1]}),
    ({"enum": [1]}, {"enum": [1.0]}),
    ({"enum": [1, 1.0]}, {"enum": [1]}),
    ({"enum": [{"a": 1, "b": [2]}]}, {"enum": [{"b": [2.0], "a": 1}]}),
    ({"enum": [2**64 + 1]}, {"enum": [2**64 + 1]}),
    ({"dependentRequired": {"a": ["b", "c"], "d": []}}, {"dependentRequired": {"d": [], "a": ["c", "b", "b"]}}),
    # JSON equality of other values.
    ({"minimum": 1}, {"minimum": 1.0}),
    ({"maxLength": 0}, {"maxLength": -0.0}),
    ({"const": {"a": [1], "b": None}}, {"const": {"b": None, "a": [1.0]}}),
    ({"default": [1, {"x": 2}]}, {"default": [1.0, {"x": 2.0}]}),
    # Keyword and property order.
    ({"type": "object", "properties": {"a": {}, "b": {}}}, {"properties": {"b": {}, "a": {}}, "type": "object"}),
    # Exclusions at the root and below.
    ({"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "string"}, {"type": "string"}),
    ({"x-provenance": {"technique": "t"}, "type": "string"}, {"type": "string", "x-provenance": {"other": 1}}),
    (
        {"properties": {"a": {"$schema": "x", "x-provenance": {}, "type": "string"}}},
        {"properties": {"a": {"type": "string"}}},
    ),
    ({"items": {"x-provenance": {}}}, {"items": {}}),
    # Boolean schemas.
    (True, True),
    ({"items": False, "prefixItems": [True]}, {"prefixItems": [True], "items": False}),
]

UNEQUAL_PAIRS: list[tuple[Any, Any]] = [
    # Booleans are not numbers.
    ({"enum": [1, True]}, {"enum": [True]}),
    ({"enum": [1]}, {"enum": [True]}),
    ({"enum": [0]}, {"enum": [False]}),
    ({"enum": [[1]]}, {"enum": [[True]]}),
    ({"const": True}, {"const": 1}),
    ({"const": False}, {"const": 0}),
    ({"default": False}, {"default": 0}),
    ({"const": 2**64 + 1}, {"const": float(2**64)}),
    ({"x-note": {"a": 1}}, {"x-note": {"a": True}}),
    # Different values, keywords, and sets.
    ({"minimum": 0}, {}),
    ({"minimum": 0}, {"exclusiveMinimum": 0}),
    ({"type": ["integer"]}, {"type": ["number"]}),
    ({"type": ["integer", "number"]}, {"type": ["number"]}),
    ({"required": ["a"]}, {"required": ["a", "b"]}),
    ({"required": []}, {}),
    ({"enum": [[1, 2]]}, {"enum": [[2, 1]]}),
    ({"dependentRequired": {"a": ["b"]}}, {"dependentRequired": {"a": ["c"]}}),
    ({"dependentRequired": {"a": ["b"]}}, {"dependentRequired": {"a": ["b"], "c": []}}),
    ({"$ref": "#/$defs/a"}, {"$ref": "#/$defs/b"}),
    # Ordered lists: each element has its own pointer.
    ({"examples": [1, 2]}, {"examples": [2, 1]}),
    ({"anyOf": [{"type": "string"}, {"type": "null"}]}, {"anyOf": [{"type": "null"}, {"type": "string"}]}),
    ({"oneOf": [{"const": 1}, {"const": 2}]}, {"oneOf": [{"const": 2}, {"const": 1}]}),
    ({"prefixItems": [{"type": "string"}, {}]}, {"prefixItems": [{}, {"type": "string"}]}),
    ({"allOf": [{}]}, {"allOf": [{}, {}]}),
    # Nested differences.
    ({"properties": {"a": {"type": "string"}}}, {"properties": {"a": {"type": "integer"}}}),
    ({"properties": {"a": {}}}, {"properties": {"b": {}}}),
    ({"properties": {"a": {}}}, {"patternProperties": {"a": {}}}),
    ({"items": {"minimum": 1}}, {"items": {"minimum": 2}}),
    ({"items": {"enum": [1]}}, {"items": {"enum": [True]}}),
    ({"prefixItems": [{}, {"const": 1}]}, {"prefixItems": [{}, {"const": True}]}),
    ({"anyOf": [{"enum": ["a"]}]}, {"anyOf": [{"enum": ["b"]}]}),
    ({"$defs": {"x": {"type": "string"}}}, {"$defs": {"x": {"type": "null"}}}),
    ({"$defs": {"x": {}}}, {"$defs": {"y": {}}}),
    ({"patternProperties": {"^a": {"maxLength": 3}}}, {"patternProperties": {"^a": {"maxLength": 4}}}),
    ({"not": {"required": ["a"]}}, {"not": {"required": ["b"]}}),
    ({"if": {}, "then": {"minimum": 0}}, {"if": {}, "else": {"minimum": 0}}),
    (
        {"properties": {"a": {"items": {"properties": {"b": {"const": 1}}}}}},
        {"properties": {"a": {"items": {"properties": {"b": {"const": 1.5}}}}}},
    ),
    # Property names are not keywords: a property named x-provenance counts.
    ({"properties": {"x-provenance": True}}, {"properties": {}}),
    ({"properties": {"$schema": {}}}, {"properties": {}}),
    # Boolean schemas.
    (True, False),
    (True, {}),
    (False, {"not": {}}),
    ({"items": True}, {"items": False}),
    ({"items": True}, {"items": {}}),
    ({"additionalProperties": False}, {"additionalProperties": {}}),
    ({"anyOf": [True]}, {"anyOf": [{}]}),
]


@pytest.mark.parametrize(("a", "b"), EQUAL_PAIRS)
def test_equal_schemas(a: Any, b: Any) -> None:
    assert node(a) == node(b)
    assert node(a).equality_key() == node(b).equality_key()
    assert model(a) == model(b)
    assert not (model(a) != model(b))


@pytest.mark.parametrize(("a", "b"), UNEQUAL_PAIRS)
def test_unequal_schemas(a: Any, b: Any) -> None:
    for x, y in ((a, b), (b, a)):
        assert node(x) != node(y)
        assert node(x).equality_key() != node(y).equality_key()
        assert model(x) != model(y)


def test_model_metadata_excluded_from_equality() -> None:
    schema = {"type": "object", "required": ["a"], "properties": {"a": {"enum": [1, 2]}}}
    plain = model(schema)
    root = SchemaNode.from_json(schema)
    rich = SchemaModel(
        root,
        dialect_assumed=True,
        ignored=[("/properties/a", "markdownDescription"), ("", "$comment")],
        provenance={ConstraintId(root.nid, "required", "a"): object(), ConstraintId(root.nid, "type"): object()},
    )
    assert rich == plain


def test_unsupported_records_compare_as_sets() -> None:
    schema = {"type": "string"}
    a = model(schema, unsupported=[("", "multipleOf"), ("/items", "minContains")])
    b = model(schema, unsupported=[("/items", "minContains"), ("", "multipleOf"), ("", "multipleOf")])
    assert a == b
    assert not a.fully_supported and model(schema).fully_supported
    assert model(schema) != a
    assert a != model(schema, unsupported=[("", "multipleOf")])


def test_equality_with_other_types_and_hashing() -> None:
    n, m = node({"type": "string"}), model({"type": "string"})
    assert n != {"type": "string"} and m != n and n != m
    assert m == m and n == n
    with pytest.raises(TypeError):
        hash(n)
    with pytest.raises(TypeError):
        hash(m)


def test_malformed_trees_raise_on_comparison() -> None:
    bad = SchemaNode()
    bad.kw["items"] = {"type": "string"}  # raw JSON put in after construction
    with pytest.raises(TypeError):
        bad.equality_key()
    worse = SchemaNode(True)
    worse.kw["type"] = "string"
    with pytest.raises(ValueError):
        worse.equality_key()


# ---------------------------------------------------------------------------
# SchemaModel
# ---------------------------------------------------------------------------


def test_model_defaults_and_validation() -> None:
    m = SchemaModel(SchemaNode())
    assert (m.dialect_assumed, m.ignored, m.unsupported, m.provenance) == (False, [], [], {})
    assert m.fully_supported
    assert SchemaModel(SchemaNode(), ignored=[["/a", "b"]]).ignored == [("/a", "b")]
    for kwargs, error in [
        ({"root": {"type": "string"}}, TypeError),
        ({"root": SchemaNode(), "dialect_assumed": 1}, TypeError),
        ({"root": SchemaNode(), "ignored": [("/a",)]}, TypeError),
        ({"root": SchemaNode(), "unsupported": [("", 1)]}, TypeError),
        ({"root": SchemaNode(), "provenance": [(ConstraintId(1, "type"), None)]}, TypeError),
        ({"root": SchemaNode(), "provenance": {(1, "type", None): None}}, TypeError),
    ]:
        with pytest.raises(error):
            SchemaModel(**kwargs)


def test_model_pickles() -> None:
    m = model({"properties": {"a": {"enum": [1, True]}}}, dialect_assumed=True, unsupported=[("", "multipleOf")])
    m.provenance[ConstraintId(m.root.nid, "properties")] = {"technique": "t"}
    restored = pickle.loads(pickle.dumps(m))
    assert restored == m
    assert restored.provenance == m.provenance
    assert (restored.dialect_assumed, restored.unsupported) == (True, [("", "multipleOf")])


# ---------------------------------------------------------------------------
# Traversal and JSON Pointers
# ---------------------------------------------------------------------------

TREE: dict[str, Any] = {
    "type": "object",
    "properties": {"b": {"items": {"type": "string"}}, "a/b": True, "m~n": {}},
    "anyOf": [{"prefixItems": [{}, False]}, {"$ref": "#"}],
    "$defs": {"d": {"not": {}}},
    "x-note": {"items": {"type": "string"}},  # plain JSON, not a subschema
    "enum": [{"properties": {}}],
}


def test_walk_yields_pointers_in_canonical_pre_order() -> None:
    assert [pointer for pointer, _ in node(TREE).walk()] == [
        "",
        "/$defs/d",
        "/$defs/d/not",
        "/anyOf/0",
        "/anyOf/0/prefixItems/0",
        "/anyOf/0/prefixItems/1",
        "/anyOf/1",
        "/properties/a~1b",
        "/properties/b",
        "/properties/b/items",
        "/properties/m~0n",
    ]
    assert [pointer for pointer, _ in node(TREE).walk("/$defs/x")][:2] == ["/$defs/x", "/$defs/x/$defs/d"]
    assert list(SchemaNode(True).walk())[0][0] == ""


def test_children() -> None:
    tree = node(TREE)
    assert [(keyword, entry) for keyword, entry, _ in tree.children()] == [
        ("$defs", "d"),
        ("anyOf", 0),
        ("anyOf", 1),
        ("properties", "a/b"),
        ("properties", "b"),
        ("properties", "m~n"),
    ]
    assert list(SchemaNode(False).children()) == []


def test_resolve() -> None:
    m = SchemaModel(node(TREE))
    for pointer, n in m.walk():
        assert m.resolve(pointer) is n
    assert m.resolve("/properties/a~1b").boolean is True
    assert m.resolve("/anyOf/0/prefixItems/1").boolean is False
    for pointer in (
        "/properties",  # a container, not a subschema
        "/properties/c",
        "/x-note",
        "/x-note/items",
        "/enum/0",
        "/type",
        "/anyOf/2",
        "/anyOf/01",
        "/anyOf/-1",
        "/anyOf/a",
        "/properties/a~1b/items",  # below a boolean schema
        "/items",
    ):
        with pytest.raises(KeyError):
            m.resolve(pointer)
    for pointer in ("properties", "/properties/m~2n", "/a~"):
        with pytest.raises(ValueError):
            m.resolve(pointer)


def test_pointer_helpers() -> None:
    assert escape_token("a/b~c") == "a~1b~0c"
    assert escape_token(3) == "3"
    assert join_pointer("", "properties", "a/b") == "/properties/a~1b"
    assert join_pointer("/anyOf", 0, "items") == "/anyOf/0/items"
    assert join_pointer("/x") == "/x"
    assert split_pointer("") == []
    assert split_pointer("/") == [""]
    assert split_pointer("/a~01/~1b//c") == ["a~1", "/b", "", "c"]
    for token in ("", "~", "/", "~0", "~1", "a/~b", "~~01"):
        assert split_pointer(join_pointer("", token)) == [token]
    for bad in ("a", "#/a", "/~2", "/~", "/~~01"):
        with pytest.raises(ValueError):
            split_pointer(bad)
    with pytest.raises(TypeError):
        escape_token(True)  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        escape_token(-1)


def test_from_json_and_to_json() -> None:
    tree = node(TREE)
    assert strict(tree.to_json()) == strict(TREE)
    assert tree.kw["x-note"] == {"items": {"type": "string"}} and isinstance(tree.kw["x-note"], dict)
    # Values are copied in both directions.
    source = {"enum": [1, 2]}
    copied = node(source)
    copied.kw["enum"].append(3)
    assert source == {"enum": [1, 2]}
    out = copied.to_json()
    out["enum"].append(4)  # type: ignore[index, union-attr]
    assert copied.kw["enum"] == [1, 2, 3]
    assert node(True).to_json() is True
    for bad in (None, 1, "string", [{}]):
        with pytest.raises(TypeError):
            node(bad)
    for bad_kw in ({"items": [{}]}, {"anyOf": {}}, {"properties": []}, {"not": 1}, {"properties": {"a": None}}):
        with pytest.raises(TypeError):
            node(bad_kw)


# ---------------------------------------------------------------------------
# Equality invariants over generated schemas
# ---------------------------------------------------------------------------

_NAMES = st.sampled_from(["", "a", "b", "a/b", "m~n", "é", "x-provenance"])
_TYPES = st.sampled_from(["null", "boolean", "integer", "number", "string", "array", "object"])
_LEAF = st.fixed_dictionaries(
    {},
    optional={
        "type": st.one_of(_TYPES, st.lists(_TYPES, min_size=1, max_size=3, unique=True)),
        "enum": st.lists(json_scalars, min_size=1, max_size=4),
        "const": json_values(max_leaves=4),
        "required": st.lists(_NAMES, max_size=3, unique=True),
        "minimum": st.integers(min_value=-3, max_value=3),
        "dependentRequired": st.dictionaries(_NAMES, st.lists(_NAMES, max_size=3, unique=True), max_size=2),
        "x-note": json_values(max_leaves=4),
    },
)


def _with_children(children: st.SearchStrategy) -> st.SearchStrategy:
    containers = st.fixed_dictionaries(
        {},
        optional={
            "properties": st.dictionaries(_NAMES, children, max_size=3),
            "$defs": st.dictionaries(_NAMES, children, max_size=2),
            "items": children,
            "not": children,
            "prefixItems": st.lists(children, min_size=1, max_size=3),
            "anyOf": st.lists(children, min_size=1, max_size=3),
        },
    )
    return st.tuples(_LEAF, containers).map(lambda parts: {**parts[0], **parts[1]})


SCHEMAS = st.recursive(st.one_of(st.booleans(), _LEAF), _with_children, max_leaves=8)


def _respell(value: Any, rnd: _random.Random) -> Any:
    """An equal JSON value: integral numbers as floats, object members reordered."""
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, int):
        return float(value) if float(value) == value and rnd.random() < 0.5 else value
    if isinstance(value, list):
        return [_respell(item, rnd) for item in value]
    if isinstance(value, dict):
        items = list(value.items())
        rnd.shuffle(items)
        return {key: _respell(item, rnd) for key, item in items}
    return value


def _shuffled_set(values: list[Any], rnd: _random.Random) -> list[Any]:
    out = [_respell(v, rnd) for v in values]
    if out:
        out.append(_respell(rnd.choice(values), rnd))  # a repeat
    rnd.shuffle(out)
    return out


def _variant(schema: Any, rnd: _random.Random) -> Any:
    """A schema equal to ``schema`` under 2.7, spelled differently."""
    if isinstance(schema, bool):
        return schema
    out: dict[str, Any] = {}
    for keyword, value in schema.items():
        shape = SUBSCHEMA_KEYWORDS.get(keyword)
        if shape == "single":
            out[keyword] = _variant(value, rnd)
        elif shape == "list":
            out[keyword] = [_variant(child, rnd) for child in value]
        elif shape == "map":
            names = list(value)
            rnd.shuffle(names)
            out[keyword] = {name: _variant(value[name], rnd) for name in names}
        elif keyword in ("enum", "required"):
            out[keyword] = _shuffled_set(value, rnd)
        elif keyword == "type":
            out[keyword] = _shuffled_set([value] if isinstance(value, str) else value, rnd)
        elif keyword == "dependentRequired":
            names = list(value)
            rnd.shuffle(names)
            out[keyword] = {name: _shuffled_set(value[name], rnd) for name in names}
        else:
            out[keyword] = _respell(value, rnd)
    out["x-provenance"] = {"technique": rnd.choice(["a", "b"])}
    if rnd.random() < 0.5:
        out["$schema"] = "https://json-schema.org/draft/2020-12/schema"
    keys = list(out)
    rnd.shuffle(keys)
    return {key: out[key] for key in keys}


@given(SCHEMAS, st.randoms(use_true_random=False))
def test_respelled_schemas_are_equal(schema: Any, rnd: _random.Random) -> None:
    original = node(schema)
    variant = node(_variant(schema, rnd))
    assert variant == original
    assert variant.equality_key() == original.equality_key()
    assert SchemaModel(variant, dialect_assumed=True) == SchemaModel(original)


@given(SCHEMAS)
def test_json_round_trip_preserves_equality_and_spelling(schema: Any) -> None:
    tree = node(schema)
    assert strict(tree.to_json()) == strict(schema)
    assert node(tree.to_json()) == tree
    m = SchemaModel(tree)
    pointers = [pointer for pointer, _ in m.walk()]
    assert len(pointers) == len(set(pointers)) == len(m.index())
    assert all(m.resolve(pointer) is n for pointer, n in m.walk())
