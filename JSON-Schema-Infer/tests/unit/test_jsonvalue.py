"""Unit tests for core/jsonvalue.py (task 2.2)."""

from __future__ import annotations

import itertools
import json
import pickle
from typing import Any

import numpy as np
import pytest

from jsinfer.core import determinism as det
from jsinfer.core.jsonvalue import (
    JType,
    ValueInterner,
    ValueTable,
    canonical_key,
    canonical_value,
    distinct_values,
    json_equal,
    jtype_of,
)


def strict(value: Any) -> str:
    """A spelling that tells apart what Python's == conflates (True/1, 1/1.0, key order)."""
    return json.dumps(value, ensure_ascii=False)


# Equivalence classes under JSON Schema equality: values in one class are
# equal, values in different classes are not.
CLASSES: list[list[Any]] = [
    [None],
    [False],
    [True],
    [0, 0.0, -0.0],
    [1, 1.0, 1e0],
    [-1, -1.0],
    [1000, 1e3, 1000.0],
    [2**53, float(2**53)],
    [2**53 + 1],
    [2**64, float(2**64)],
    [2**64 + 1],
    [-(2**70)],
    [0.5],
    [-2.5],
    [0.1 + 0.2],
    [0.3],
    [""],
    ["1"],
    ["true"],
    ["é"],
    ["e\u0301"],  # NFD spelling of "é": strings are not normalized
    [[], ()],
    [[1, 2], [1.0, 2.0], (1, 2e0)],
    [[2, 1]],
    [[1]],
    [[True]],
    [[None]],
    [[[1.0]], [(1,)]],
    [{}],
    [{"a": 1, "b": [1.0]}, {"b": (1,), "a": 1.0}],
    [{"a": 1}],
    [{"a": True}],
    [{"a": None}],
    [{"1": 1}],
]
VALUES: list[Any] = [value for cls in CLASSES for value in cls]
CLASS_OF: list[int] = [i for i, cls in enumerate(CLASSES) for _ in cls]

# ---------------------------------------------------------------------------
# JType
# ---------------------------------------------------------------------------


def test_jtype_values_are_the_type_ranks() -> None:
    assert list(JType) == [JType.NULL, JType.BOOLEAN, JType.INTEGER, JType.NUMBER, JType.STRING, JType.ARRAY, JType.OBJECT]
    ranks = [det.RANK_NULL, det.RANK_BOOLEAN, det.RANK_INTEGER, det.RANK_NUMBER, det.RANK_STRING, det.RANK_ARRAY, det.RANK_OBJECT]
    assert [int(t) for t in JType] == ranks == list(range(7))


def test_jtype_keywords() -> None:
    assert [t.keyword for t in JType] == ["null", "boolean", "integer", "number", "string", "array", "object"]
    assert all(JType.from_keyword(t.keyword) is t for t in JType)
    assert [t for t in JType if t.is_scalar] == [JType.NULL, JType.BOOLEAN, JType.INTEGER, JType.NUMBER, JType.STRING]
    for bad in ("Integer", "float", "", None):
        with pytest.raises(ValueError, match="unknown JSON Schema type"):
            JType.from_keyword(bad)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("value", "jtype"),
    [
        (None, JType.NULL),
        (True, JType.BOOLEAN),
        (False, JType.BOOLEAN),
        (0, JType.INTEGER),
        (-5, JType.INTEGER),
        (2**64 + 1, JType.INTEGER),
        (-(2**200), JType.INTEGER),
        (1.0, JType.INTEGER),
        (1e3, JType.INTEGER),
        (-0.0, JType.INTEGER),
        (float(2**63), JType.INTEGER),
        (1e300, JType.INTEGER),
        (np.float64(4.0), JType.INTEGER),
        (0.5, JType.NUMBER),
        (-2.5, JType.NUMBER),
        (1e-300, JType.NUMBER),
        (5e-324, JType.NUMBER),
        ("", JType.STRING),
        ("1", JType.STRING),
        ([], JType.ARRAY),
        ((1,), JType.ARRAY),
        ({}, JType.OBJECT),
    ],
)
def test_jtype_of(value: Any, jtype: JType) -> None:
    assert jtype_of(value) is jtype


def test_jtype_of_rejects_non_json_values() -> None:
    for value in (float("nan"), float("inf"), float("-inf")):
        with pytest.raises(ValueError, match="non-finite"):
            jtype_of(value)
    for value in ({1}, b"x", object(), np.int64(1)):
        with pytest.raises(TypeError, match="is not a JSON value"):
            jtype_of(value)


# ---------------------------------------------------------------------------
# Canonical key and representative
# ---------------------------------------------------------------------------


def test_canonical_key_examples() -> None:
    assert canonical_key(None) == (0, None)
    assert canonical_key(True) == (1, 1) and canonical_key(False) == (1, 0)
    assert canonical_key(1.0) == canonical_key(1) == (2, 1)
    assert canonical_key(-0.0) == (2, 0)
    assert canonical_key(0.5) == (3, 0.5)
    assert canonical_key("a") == (4, "a")
    assert canonical_key([1.0, {"b": 0, "a": True}]) == (5, b'[1,{"a":true,"b":0}]')
    assert canonical_key({"z": (), "a": 2e0}) == (6, b'{"a":2,"z":[]}')
    # Booleans are kept apart from numbers.
    assert canonical_key(True) != canonical_key(1)
    assert canonical_key(False) != canonical_key(0)


def test_canonical_key_integer_payloads_are_exact_ints() -> None:
    key = canonical_key(1e3)
    assert key == (2, 1000) and type(key[1]) is int
    assert canonical_key(2**80 + 1) == (2, 2**80 + 1)
    assert canonical_key(float(2**80)) == (2, 2**80)
    assert canonical_key(float(2**80)) != canonical_key(2**80 + 1)
    assert canonical_key(1e300) == (2, int(1e300))


def test_canonical_key_matches_value_sort_key() -> None:
    extra = [np.float64(0.25), np.float64(3.0), json.loads("1E2"), -1e-7]
    for value in [*VALUES, *extra]:
        assert canonical_key(value) == det.value_sort_key(value)


def test_canonical_key_rejects_non_json_values() -> None:
    with pytest.raises(ValueError):
        canonical_key(float("nan"))
    with pytest.raises(ValueError):
        canonical_key([float("inf")])
    with pytest.raises(TypeError):
        canonical_key({1: "a"})
    with pytest.raises(TypeError):
        canonical_key(np.int64(3))


def test_canonical_value_normalizes_numbers_tuples_and_key_order() -> None:
    assert strict(canonical_value(1.0)) == "1" and type(canonical_value(1.0)) is int
    assert strict(canonical_value(-0.0)) == "0"
    assert strict(canonical_value(1e3)) == "1000"
    assert type(canonical_value(np.float64(0.5))) is float
    assert canonical_value(True) is True and canonical_value(None) is None
    assert canonical_value(2**100 + 1) == 2**100 + 1
    nested = (1.0, {"b": 2.0, "a": (3, True)})
    assert strict(canonical_value(nested)) == '[1, {"a": [3, true], "b": 2}]'
    keys = {"é": 1, "z": 2, "\U0001f600": 3, "Z": 0, "\uffff": 4}
    assert list(canonical_value(keys)) == ["Z", "z", "é", "\uffff", "\U0001f600"]  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="object keys must be str"):
        canonical_value({1: 2})


def test_canonical_value_is_a_fixed_point_with_the_same_key() -> None:
    for value in VALUES:
        rep = canonical_value(value)
        assert canonical_key(rep) == canonical_key(value)
        assert strict(canonical_value(rep)) == strict(rep)


def test_equal_values_have_identical_representatives() -> None:
    for cls in CLASSES:
        assert len({strict(canonical_value(value)) for value in cls}) == 1


# ---------------------------------------------------------------------------
# Equality
# ---------------------------------------------------------------------------


def test_equality_matches_the_equivalence_classes() -> None:
    # json_equal, canonical-key equality, and canonical-bytes equality all
    # agree with the hand-made classes on every ordered pair.
    for (a, ca), (b, cb) in itertools.product(zip(VALUES, CLASS_OF), repeat=2):
        expected = ca == cb
        assert json_equal(a, b) is expected, (a, b)
        assert (canonical_key(a) == canonical_key(b)) is expected, (a, b)
        assert (det.canonical_bytes(a) == det.canonical_bytes(b)) is expected, (a, b)


def test_json_equal_does_not_coerce() -> None:
    assert not json_equal(True, 1)
    assert not json_equal(False, 0)
    assert not json_equal([True], [1])
    assert not json_equal({"a": False}, {"a": 0})
    assert not json_equal("1", 1)
    assert not json_equal({"a": None}, {})  # null is a value; absence is not
    # A float comparison would call these equal: float(2**64 + 1) == float(2**64).
    assert not json_equal(2**64 + 1, float(2**64))
    assert not json_equal(2**64 + 1, 2**64)
    assert json_equal(2**64, float(2**64))


def test_json_equal_handles_deep_nesting() -> None:
    def nest(leaf: Any, depth: int) -> Any:
        for _ in range(depth):
            leaf = [leaf]
        return leaf

    assert json_equal(nest(1, 5000), nest(1.0, 5000))
    assert not json_equal(nest(1, 5000), nest(True, 5000))


def test_json_equal_rejects_non_json_values() -> None:
    with pytest.raises(ValueError):
        json_equal(float("nan"), float("nan"))
    with pytest.raises(TypeError):
        json_equal({1: 2}, {1: 2})
    with pytest.raises(TypeError):
        json_equal({1, 2}, {1, 2})


# ---------------------------------------------------------------------------
# Canonical value order
# ---------------------------------------------------------------------------

UNSORTED = [
    {"b": 1}, [1], "b", 0.5, -2.5, True, 1.0, None, "a", 2**64 + 1, -3, False,
    [1, 2], {"a": 2}, "", 1, "\U0001f600", "\uffff", [], {}, 1e0, (1,),
]  # fmt: skip
# Integers come before non-integral numbers (rank, then value). Containers
# order by canonical bytes: b"[1,2]" < b"[1]" < b"[]" since "," < "1" < "]".
SORTED_DISTINCT = [
    None, False, True, -3, 1, 2**64 + 1, -2.5, 0.5,
    "", "a", "b", "\uffff", "\U0001f600",
    [1, 2], [1], [],
    {"a": 2}, {"b": 1}, {},
]  # fmt: skip


def test_distinct_values_in_canonical_order() -> None:
    assert [strict(v) for v in distinct_values(UNSORTED)] == [strict(v) for v in SORTED_DISTINCT]


def test_canonical_order_does_not_depend_on_input_order() -> None:
    gen = det.rng("test.jsonvalue.order")
    expected = [strict(v) for v in SORTED_DISTINCT]
    for _ in range(20):
        shuffled = [UNSORTED[i] for i in gen.permutation(len(UNSORTED))]
        assert [strict(v) for v in distinct_values(shuffled)] == expected
        assert [canonical_key(v) for v in sorted(shuffled, key=canonical_key)] == sorted(map(canonical_key, UNSORTED))


def test_canonical_key_order_is_total_over_mixed_types() -> None:
    # Sorting never compares payloads of different Python types.
    keys = sorted(canonical_key(v) for v in VALUES)
    assert [k[0] for k in keys] == sorted(k[0] for k in keys)


# ---------------------------------------------------------------------------
# Interning
# ---------------------------------------------------------------------------


def test_value_interner_assigns_first_seen_ids_per_canonical_key() -> None:
    interner = ValueInterner()
    assert interner.intern(1.0) == 0
    assert interner.intern("x") == 1
    assert interner.intern(1) == 0 and interner.intern(1e0) == 0
    assert interner.intern(True) == 2
    assert interner.intern([1.0]) == interner.intern((1,)) == 3
    assert len(interner) == 4
    assert interner.keys() == [(2, 1), (4, "x"), (1, 1), (5, b"[1]")]
    assert [strict(v) for v in interner.values()] == ["1", '"x"', "true", "[1]"]
    clone = pickle.loads(pickle.dumps(interner))
    assert clone.keys() == interner.keys() and clone.intern(True) == 2 and len(clone) == 4


def test_value_table_ids_follow_canonical_order() -> None:
    table = ValueTable(["b", 2.5, 1.0, True, None, "a", 1, 1e0])
    assert [strict(v) for v in table.values()] == ["null", "true", "1", "2.5", '"a"', '"b"']
    assert len(table) == 6
    assert table.id_of(1) == table.id_of(1.0) == 2
    assert table.id_of(True) == 1
    assert table[2] == 1 and type(table[2]) is int
    assert table[np.int32(3)] == 2.5
    assert table.key(4) == (4, "a")
    assert list(table.keys()) == sorted(table.keys())
    assert 1.0 in table and True in table and False not in table and 0 not in table
    with pytest.raises(KeyError, match="value not in table"):
        table.id_of(False)
    for bad in (-1, 6):
        with pytest.raises(IndexError, match="out of range"):
            table[bad]


def test_value_tables_compare_by_json_equality() -> None:
    assert ValueTable([1.0, "a", True]) == ValueTable([True, "a", 1])
    assert ValueTable([True]) != ValueTable([1])  # (True,) == (1,) in Python
    assert ValueTable() == ValueTable([]) and len(ValueTable()) == 0
    with pytest.raises(TypeError):
        hash(ValueTable())
    table = ValueTable([1, [2.0], {"k": None}])
    assert pickle.loads(pickle.dumps(table)) == table


def test_merge_does_not_depend_on_chunking_or_part_order() -> None:
    stream = [3, "b", 1.0, True, None, "a", 1, [1], 0.5, "b", False, (1.0,), 3.0, {"k": 1}, -0.0, 0]
    reference = ValueTable(stream)
    splits = [[stream], [stream[:5], stream[5:11], stream[11:]], [[v] for v in stream], [stream[:8], [], stream[8:]]]
    for chunks in splits:
        for ordered in (chunks, chunks[::-1]):
            interners = [ValueInterner() for _ in ordered]
            local_ids = [[interner.intern(v) for v in chunk] for interner, chunk in zip(interners, ordered)]
            table, remaps = ValueTable.merge(interners)
            assert table == reference
            assert [strict(v) for v in table.values()] == [strict(v) for v in reference.values()]
            for chunk, ids, remap in zip(ordered, local_ids, remaps):
                assert remap.dtype == np.int32
                assert remap.shape == (len(set(ids)),)
                for value, local in zip(chunk, ids):
                    assert table.key(int(remap[local])) == canonical_key(value)


def test_merge_edge_cases() -> None:
    table, remaps = ValueTable.merge([])
    assert len(table) == 0 and remaps == []
    table, remaps = ValueTable.merge([ValueInterner()])
    assert len(table) == 0 and remaps[0].dtype == np.int32 and remaps[0].shape == (0,)
    # Tables can be merged too.
    table, remaps = ValueTable.merge([ValueTable(["b", 2]), ValueInterner(["a", 2.0])])
    assert [strict(v) for v in table.values()] == ['2', '"a"', '"b"']
    assert remaps[0].tolist() == [0, 2] and remaps[1].tolist() == [1, 0]
