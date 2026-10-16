"""Unit tests for core/observations.py: flattening, merging, and the column layout (task 2.3)."""

from __future__ import annotations

import pickle

import numpy as np
import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.core.jsonvalue import JType
from jsinfer.core.observations import (
    Column,
    ObservationStore,
    build_store,
    check_shape_path,
    flatten_chunk,
    merge_partials,
    shape_of,
)
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, ROOT, key_step, parse_path
from tests.reference import assert_stores_equal
from tests.strategies import json_documents

M, E = MAPVAL_STEP, ELEM_STEP


def fields(store: ObservationStore, shape: tuple) -> dict[str, list[int]]:
    col = store.columns[shape]
    return {name: getattr(col, name).tolist() for name in ("parent", "doc", "jtype", "key", "pos", "value", "size")}


def test_flatten_nested_document() -> None:
    store = build_store([{"b": [1, {"x": None}], "a": "hé"}])
    assert list(store.columns) == [ROOT, (M,), (M, E), (M, E, M)]
    assert store.keys.values() == ("a", "b", "x")
    a, b, x = 0, 1, 2
    v = store.values.id_of
    assert fields(store, ROOT) == {
        "parent": [-1], "doc": [0], "jtype": [JType.OBJECT], "key": [-1], "pos": [-1], "value": [-1], "size": [2],
    }  # fmt: skip
    # Members in code-point key order: "a" before "b", whatever the source order.
    assert fields(store, (M,)) == {
        "parent": [0, 0], "doc": [0, 0], "jtype": [JType.STRING, JType.ARRAY], "key": [a, b],
        "pos": [-1, -1], "value": [v("hé"), -1], "size": [2, 2],
    }  # fmt: skip
    assert fields(store, (M, E)) == {
        "parent": [1, 1], "doc": [0, 0], "jtype": [JType.INTEGER, JType.OBJECT], "key": [-1, -1],
        "pos": [0, 1], "value": [v(1), -1], "size": [-1, 1],
    }  # fmt: skip
    assert fields(store, (M, E, M)) == {
        "parent": [1], "doc": [0], "jtype": [JType.NULL], "key": [x], "pos": [-1], "value": [v(None)], "size": [-1],
    }  # fmt: skip


def test_top_level_scalars_and_empty_containers() -> None:
    docs = [1, "xyz", None, True, 2.5, [], {}]
    store = build_store(docs)
    assert list(store.columns) == [ROOT]  # empty containers have no member columns
    root = fields(store, ROOT)
    assert root["jtype"] == [JType.INTEGER, JType.STRING, JType.NULL, JType.BOOLEAN, JType.NUMBER, JType.ARRAY, JType.OBJECT]
    assert root["size"] == [-1, 3, -1, -1, -1, 0, 0]
    assert root["doc"] == list(range(7))
    assert [store.values[vid] if vid >= 0 else "-" for vid in root["value"]] == [1, "xyz", None, True, 2.5, "-", "-"]
    assert len(store.keys) == 0


def test_numbers_share_ids_by_value_and_booleans_stay_apart() -> None:
    store = build_store([1, 1.0, 1e0, True, -0.0, 0, False, 2**70, float(2**70)])
    root = fields(store, ROOT)
    ids = root["value"]
    assert ids[0] == ids[1] == ids[2]
    assert ids[3] != ids[0] and ids[6] != ids[5]
    assert ids[4] == ids[5]
    assert ids[7] == ids[8]  # 2**70 is exactly representable
    assert root["jtype"][:3] == [JType.INTEGER] * 3
    assert root["jtype"][4] == JType.INTEGER
    # Ids follow canonical value order: null < booleans < integers.
    assert store.values.values() == (False, True, 0, 1, 2**70)


def test_member_order_in_the_source_does_not_matter() -> None:
    assert_stores_equal(build_store([{"b": 1, "a": [2, {"d": 0, "c": 1}]}]), build_store([{"a": [2, {"c": 1, "d": 0}], "b": 1}]))


def test_keys_are_interned_in_code_point_order() -> None:
    store = build_store([{"😀": 1, "\uffff": 2, "a": 3, "B": 4, "": 5}])
    assert store.keys.values() == ("", "B", "a", "\uffff", "😀")
    assert store.columns[(M,)].key.tolist() == [0, 1, 2, 3, 4]
    assert store.key_id("a") == 2 and store.key_id("missing") is None
    assert store.key_name(4) == "😀"
    with pytest.raises(TypeError):
        store.key_id(1)  # type: ignore[arg-type]


@given(json_documents())
def test_column_invariants(docs: list) -> None:
    store = build_store(docs, chunk_size=3)
    assert store.columns[ROOT].doc.tolist() == list(range(len(docs)))
    assert (store.columns[ROOT].parent == -1).all()
    for shape, col in store.columns.items():
        assert (np.diff(col.parent) >= 0).all() and (np.diff(col.doc) >= 0).all()
        if not shape:
            continue
        parent = store.columns[shape[:-1]]
        # Each parent row's run of children has the parent's size and document.
        counts = np.bincount(col.parent, minlength=len(parent))
        is_parent = parent.jtype == (JType.OBJECT if shape[-1] == M else JType.ARRAY)
        np.testing.assert_array_equal(counts[is_parent], parent.size[is_parent])
        assert (counts[~is_parent] == 0).all()
        np.testing.assert_array_equal(col.doc, parent.doc[col.parent])
        lo, hi = col.runs(np.arange(len(parent)))
        for start, stop in zip(lo.tolist(), hi.tolist()):
            if shape[-1] == E:
                assert col.pos[start:stop].tolist() == list(range(stop - start))
            else:
                assert (np.diff(col.key[start:stop]) > 0).all()
        assert ((col.key >= 0) == (shape[-1] == M)).all()
        assert ((col.pos >= 0) == (shape[-1] == E)).all()
    for col in store.columns.values():
        scalar = col.jtype <= JType.STRING
        assert ((col.value >= 0) == scalar).all()


@given(json_documents(), st.integers(min_value=1, max_value=4))
def test_merge_is_independent_of_chunking(docs: list, chunk_size: int) -> None:
    whole = merge_partials([flatten_chunk(docs, 0)])
    assert_stores_equal(build_store(docs, chunk_size=chunk_size), whole)


@given(json_documents(), st.randoms(use_true_random=False))
def test_ids_do_not_depend_on_document_order(docs: list, random) -> None:
    shuffled = list(docs)
    random.shuffle(shuffled)
    a, b = build_store(docs, chunk_size=2), build_store(shuffled, chunk_size=3)
    assert a.keys == b.keys
    assert a.values == b.values
    assert list(a.columns) == list(b.columns)


def test_merge_partials_requires_contiguous_chunks_in_order() -> None:
    docs = [{"a": 1}, {"a": 2}, {"b": 3}]
    first, second = flatten_chunk(docs[:2], 0), flatten_chunk(docs[2:], 2)
    assert_stores_equal(merge_partials([first, second]), build_store(docs))
    with pytest.raises(ValueError, match="contiguously"):
        merge_partials([second, first])
    with pytest.raises(ValueError, match="contiguously"):
        merge_partials([first, flatten_chunk(docs[2:], 3)])
    with pytest.raises(ValueError, match="first_index"):
        flatten_chunk(docs, -1)


def test_partial_stores_and_stores_pickle() -> None:
    docs = [{"a": [1, "x", {"b": None}]}, [True, 2.5]]
    part = pickle.loads(pickle.dumps(flatten_chunk(docs, 0)))
    store = build_store(docs)
    assert_stores_equal(merge_partials([part]), store)
    store.flags.flag_documents([1])
    copy = pickle.loads(pickle.dumps(store))
    assert_stores_equal(copy, store)
    assert copy.flags.documents.tolist() == [1] and copy.flags.epoch == store.flags.epoch
    assert len(copy.stats_cache) == 0


def test_empty_collection() -> None:
    store = build_store([])
    assert store.n_docs == 0 and list(store.columns) == [ROOT] and len(store.columns[ROOT]) == 0
    assert len(store.keys) == 0 and len(store.values) == 0


def test_deep_nesting_does_not_recurse() -> None:
    depth = 1500  # beyond the default recursion limit of 1000
    doc: object = "leaf"
    for _ in range(depth):
        doc = [doc]
    store = build_store([doc])
    assert len(store.columns) == depth + 1
    assert store.columns[(E,) * depth].jtype.tolist() == [JType.STRING]


@pytest.mark.parametrize(
    ("doc", "error"),
    [({1: "x"}, TypeError), (float("nan"), ValueError), ([float("inf")], ValueError), ({"a": {1, 2}}, TypeError)],
)
def test_invalid_values_raise(doc: object, error: type[Exception]) -> None:
    with pytest.raises(error):
        flatten_chunk([doc], 0)


def test_columns_are_read_only_and_indexed_by_key() -> None:
    store = build_store([{"a": 1, "b": 2}, {"b": 3}, {"a": 4}])
    col = store.columns[(M,)]
    with pytest.raises(ValueError):
        col.value[0] = 5
    assert col.by_key(store.key_id("a")).tolist() == [0, 3]
    assert col.by_key(store.key_id("b")).tolist() == [1, 2]
    assert col.by_key(99).tolist() == []
    with pytest.raises(ValueError):
        col.by_key(-1)
    with pytest.raises(TypeError):
        Column((), parent=np.zeros(1))


def test_shape_paths() -> None:
    assert shape_of(parse_path("$.a[*].b{*}")) == (M, E, M, M)
    assert check_shape_path((M, E)) == (M, E)
    with pytest.raises(ValueError):
        check_shape_path((key_step("a"),))
    with pytest.raises(TypeError):
        check_shape_path([M])
    with pytest.raises(KeyError, match=r"\$\[\*\]"):
        build_store([{}]).column((E,))
