"""Unit tests for core/views.py and the FlagSet exclusion rules (task 2.3)."""

from __future__ import annotations

import numpy as np
import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, ROOT, key_step, parse_path
from jsinfer.core.stats import FlagSet
from jsinfer.core.views import View
from tests.reference import brute_excluded, present_paths, signature, view_signatures, walk
from tests.strategies import json_documents

M, E = MAPVAL_STEP, ELEM_STEP

DOCS = [
    {"a": {"b": 1, "c": [1, 2]}, "d": 2},
    {"a": {"b": 3}, "d": "x"},
    {"a": [{"b": 4}], "e": {"k1": 1, "k2": 2, "k3": 3}},
    "top",
]


def at(store, text: str) -> View:
    return View.root(store).descend(parse_path(text))


def values(view: View) -> list:
    return [view.store.values[vid] for vid in view.gather("value").tolist()]


@given(st.data(), json_documents())
def test_views_match_the_reference_walk(data: st.DataObject, docs: list) -> None:
    store = build_store(docs, chunk_size=2)
    path = data.draw(st.sampled_from(present_paths(docs)))
    view = View.root(store).descend(path)
    assert view.path == path
    # Same Observations in the same (document, pre-order) order.
    assert view_signatures(view) == [signature(d, v) for d, v, _ in walk(docs, path)]


@given(st.data(), json_documents(), st.integers(min_value=0, max_value=3))
def test_elements_at_a_position(data: st.DataObject, docs: list, pos: int) -> None:
    store = build_store(docs)
    path = data.draw(st.sampled_from(present_paths(docs, max_depth=3)))
    view = View.root(store).descend(path).elements(pos)
    assert view.path == path + (E,)
    expected = [signature(d, v) for d, v, p in walk(docs, path + (E,)) if p == pos]
    assert view_signatures(view) == expected


def test_child_key_paths_agree_for_small_and_large_parent_sets() -> None:
    docs = [{"a": i, "b": -i} if i % 3 else {"b": i} for i in range(60)]
    store = build_store(docs)
    root = View.root(store)
    full = root.child_key("a")  # as many parents as key rows: membership test
    assert values(full) == [i for i in range(60) if i % 3]
    mask = np.zeros(60, dtype=bool)
    mask[[1, 3, 4]] = True
    few = root.subset(mask).child_key("a")  # fewer parents than key rows: parent runs
    assert values(few) == [1, 4]
    assert few.gather("doc").tolist() == [1, 4]


def test_child_key_of_a_missing_key_is_empty() -> None:
    root = View.root(build_store(DOCS))
    view = root.child_key("nope")
    assert view.is_empty and len(view) == 0 and view.path == (key_step("nope"),)
    assert view.child_key("x").is_empty and view.elements().is_empty and view.stats().n == 0
    with pytest.raises(TypeError):
        root.child_key(3)  # type: ignore[arg-type]


def test_child_map_is_one_column_for_many_keys() -> None:
    store = build_store([{f"k{i}": i for i in range(500)}, {"k7": "x"}])
    view = View.root(store).child_map()
    assert view.path == (M,) and view.columns == ((M,),) and len(view) == 501
    assert len(at(store, "$.k7")) == 2


def test_elements_rejects_bad_positions() -> None:
    root = View.root(build_store([[1, 2]]))
    assert values(root.elements(1)) == [2] and root.elements(2).is_empty
    for bad in (-1, True, 1.0):
        with pytest.raises(ValueError):
            root.elements(bad)  # type: ignore[arg-type]


def test_subset_keeps_the_path_and_checks_the_mask() -> None:
    store = build_store(DOCS)
    view = at(store, "$.d")
    picked = view.subset(view.gather("jtype") == 4)  # strings
    assert picked.path == view.path and values(picked) == ["x"]
    with pytest.raises(ValueError):
        view.subset(np.ones(3, dtype=bool))
    with pytest.raises(ValueError):
        view.subset(np.ones(len(view), dtype=int))


def test_root_of_a_document_subset() -> None:
    store = build_store(DOCS)
    segment = View.root(store, docs=[2, 0, 2])
    assert segment.gather("doc").tolist() == [0, 2]
    assert values(segment.descend(parse_path("$.a[*].b"))) == [4]
    with pytest.raises(ValueError, match="out of range"):
        View.root(store, docs=[4])


def test_union_merges_columns_and_ignores_input_order() -> None:
    store = build_store(DOCS)
    a_b, d = at(store, "$.a.b"), at(store, "$.d")
    u1, u2 = View.union([a_b, d]), View.union([d, a_b])
    assert u1.digest == u2.digest and u1.path == u2.path == parse_path("$.d")  # fewest steps
    assert sorted(map(repr, values(u1))) == sorted(map(repr, [1, 3, 2, "x"]))
    assert View.union([d, d, d.subset(np.array([True, False]))]).digest == d.digest  # duplicates once
    assert View.union([a_b], path=parse_path("$.x")).path == parse_path("$.x")
    with pytest.raises(ValueError):
        View.union([])
    with pytest.raises(ValueError, match="different"):
        View.union([d, at(build_store(DOCS), "$.d")])


def test_union_of_recursion_levels_is_multi_part() -> None:
    store = build_store([{"a": {"a": {"x": 1}, "x": 2}}, {"a": {"x": 3}}])
    levels = View.union([at(store, "$.a"), at(store, "$.a.a")])
    assert levels.columns == ((M,), (M, M))
    xs = levels.child_key("x")
    assert xs.path == parse_path("$.a.x") and xs.columns == ((M, M), (M, M, M))
    assert values(xs) == [2, 3, 1]  # parts in column order, rows in store order


def test_digest_is_content_based() -> None:
    store = build_store(DOCS)
    root = View.root(store)
    by_key = root.child_key("a")
    by_mask = root.child_map().subset(root.child_map().gather("key") == store.key_id("a"))
    assert by_key.path != by_mask.path and by_key.digest == by_mask.digest
    assert by_key.digest != root.child_key("d").digest
    assert at(build_store(DOCS, chunk_size=1), "$.a").digest == by_key.digest
    assert len(by_key.digest) == 16


# ---------------------------------------------------------------------------
# Flags
# ---------------------------------------------------------------------------


def test_flagged_document_is_excluded_everywhere() -> None:
    store = build_store(DOCS)
    store.flags.flag_documents([0])
    assert View.root(store).gather("doc").tolist() == [1, 2, 3]
    assert values(at(store, "$.a.b")) == [3]
    assert values(at(store, "$.d")) == ["x"]
    assert at(store, "$.a.c[*]").is_empty


def test_flagged_item_excludes_its_subtree_and_keeps_the_rest_of_the_document() -> None:
    store = build_store(DOCS)
    a = at(store, "$.a")
    store.flags.flag_items((M,), a.parts[0].rows[:1])  # the $.a object of document 0
    assert at(store, "$.a").gather("doc").tolist() == [1, 2]
    assert values(at(store, "$.a.b")) == [3]
    assert at(store, "$.a.c").is_empty and at(store, "$.a.c[*]").is_empty
    assert values(at(store, "$.d")) == [2, "x"]  # sibling of the flagged item
    assert View.root(store).gather("doc").tolist() == [0, 1, 2, 3]


def test_flags_added_after_a_view_was_created() -> None:
    store = build_store(DOCS)
    a = at(store, "$.a")
    assert len(a) == 3
    store.flags.flag_items((M, M), at(store, "$.a.b").parts[0].rows[1:2])  # b of document 1
    store.flags.flag_documents([2])
    assert len(a) == 3  # a view's rows are fixed
    assert a.live().gather("doc").tolist() == [0, 1]
    assert values(a.child_key("b")) == [1]
    assert a.subset(np.ones(3, dtype=bool)).gather("doc").tolist() == [0, 1]
    assert View.union([a]).gather("doc").tolist() == [0, 1]
    fresh = a.live()
    assert fresh.epoch == store.flags.epoch and fresh.live() is fresh


def test_excluded_follows_ancestors() -> None:
    store = build_store([{"a": [[1, 2], [3]]}, {"a": [[4]]}])
    deep = at(store, "$.a[*][*]")
    rows = deep.parts[0].rows
    flags = store.flags
    assert not flags.excluded(store, (M, E, E), rows).any()
    flags.flag_items((M, E), [0])  # the inner array [1, 2]
    assert flags.excluded(store, (M, E, E), rows).tolist() == [True, True, False, False]
    flags.flag_items((), [1])  # document 1's root row
    assert flags.excluded(store, (M, E, E), rows).tolist() == [True, True, False, True]
    assert values(deep.live()) == [3]


@given(json_documents(), st.data())
def test_views_drop_exactly_the_excluded_rows(docs: list, data: st.DataObject) -> None:
    store = build_store(docs, chunk_size=2)
    path = data.draw(st.sampled_from(present_paths(docs)))
    before = View.root(store).descend(path)
    if store.n_docs:
        store.flags.flag_documents(data.draw(st.lists(st.integers(0, store.n_docs - 1), max_size=2)))
    for shape in data.draw(st.lists(st.sampled_from(list(store.columns)), max_size=3)):
        n = len(store.columns[shape])
        if n:
            store.flags.flag_items(shape, data.draw(st.lists(st.integers(0, n - 1), min_size=1, max_size=2)))
    after = View.root(store).descend(path)
    expected = [
        (part.column, row)
        for part in before.parts
        for row in part.rows.tolist()
        if not brute_excluded(store, part.column, row)
    ]
    assert [(part.column, row) for part in after.parts for row in part.rows.tolist()] == expected
    assert before.live().digest == after.digest


def test_flag_epoch_changes_only_when_flags_change() -> None:
    store = build_store(DOCS)
    d = at(store, "$.d")  # rows 1 and 3 of column {*}
    flags = store.flags
    assert flags.epoch == 0 and flags.is_empty
    assert flags.flag_documents([1, 1, 3]) == 2 and flags.epoch == 1
    assert flags.flag_documents([3]) == 0 and flags.epoch == 1
    assert flags.flag_items((M,), []) == 0 and flags.epoch == 1
    assert flags.flag_items((M,), [0, 2]) == 2 and flags.epoch == 2
    other = FlagSet()
    other.flag_documents([0, 1])
    other.flag_items((M,), [2, 5])
    other.flag_items((M, M), [0])
    assert flags.update(other) == 3 and flags.epoch == 3  # one epoch per call
    assert flags.documents.tolist() == [0, 1, 3] and flags.n_items == 4
    assert [(col, rows.tolist()) for col, rows in flags.items()] == [((M,), [0, 2, 5]), ((M, M), [0])]
    assert flags.flag_view(d) == 2 and flags.epoch == 4
    flags.clear()
    assert flags.is_empty and flags.epoch == 5
    flags.clear()
    assert flags.epoch == 5


def test_flag_set_validates_input() -> None:
    flags = FlagSet()
    with pytest.raises(ValueError):
        flags.flag_documents([-1])
    with pytest.raises(TypeError):
        flags.flag_documents([True])
    with pytest.raises(TypeError):
        flags.flag_items((M,), [0.5])
    with pytest.raises(ValueError):
        flags.flag_items((key_step("a"),), [0])
    with pytest.raises(ValueError):
        flags.flag_items(ROOT, [[0, 1]])
    assert flags.epoch == 0
