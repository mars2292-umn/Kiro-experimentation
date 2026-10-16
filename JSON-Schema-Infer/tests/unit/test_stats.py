"""Unit tests for core/stats.py: ViewStats contents, invariance, and caching (task 2.3)."""

from __future__ import annotations

from collections import Counter

import numpy as np
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.core.jsonvalue import JType, canonical_key, jtype_of
from jsinfer.core.observations import ObservationStore, build_store
from jsinfer.core.paths import MAPVAL_STEP, Path, parse_path
from jsinfer.core.stats import ViewStats, compute_stats
from jsinfer.core.views import View
from tests.reference import present_paths, walk
from tests.strategies import json_documents

DOCS = [{"a": 1, "b": "xy"}, {"a": 1.0, "c": [1, 2, "z"]}, {"a": True}, "s", []]


def at(store: ObservationStore, path: Path | str) -> View:
    return View.root(store).descend(parse_path(path) if isinstance(path, str) else path)


def hist(h) -> dict[int, int]:
    return dict(zip(h.values.tolist(), h.counts.tolist()))


def test_stats_of_a_small_collection() -> None:
    store = build_store(DOCS)
    root = View.root(store).stats()
    assert root.n == 5 and root.types == (JType.STRING, JType.ARRAY, JType.OBJECT)
    assert root.count(JType.OBJECT) == 3 and root.count(JType.NULL) == 0
    assert hist(root.string_lengths) == {1: 1} and hist(root.array_lengths) == {0: 1}
    assert hist(root.object_sizes) == {1: 1, 2: 2} and root.object_sizes.total == 3
    assert root.object_index.tolist() == [0, 1, 2] and root.n_objects == 3
    a, b, c = (store.key_id(k) for k in "abc")
    assert root.key_ids.tolist() == [a, b, c] and root.key_counts.tolist() == [3, 1, 1]
    assert root.objects_with(a).tolist() == [0, 1, 2] and root.objects_with(c).tolist() == [1]
    assert root.bitset(a).tolist() == [0b111] and root.bitset(b).tolist() == [0b001]
    assert root.presence.tolist() == [[0b111], [0b001], [0b010]]
    assert root.objects_with(99).tolist() == [] and root.bitset(99).tolist() == [0]

    sa = at(store, "$.a").stats()
    assert sa.n == 3 and sa.count(JType.INTEGER) == 2 and sa.count(JType.BOOLEAN) == 1
    assert sa.value_count(store.values.id_of(1)) == 2  # 1 and 1.0 are one value
    assert sa.value_count(store.values.id_of(True)) == 1
    assert sa.value_count(store.values.id_of("xy")) == 0
    assert sa.n_objects == 0 and sa.presence.shape == (0, 0)

    elems = at(store, "$.c[*]").stats()
    assert elems.count(JType.INTEGER) == 2 and hist(elems.string_lengths) == {1: 1}
    assert [store.values[v] for v in elems.value_ids.tolist()] == [1, 2, "z"]


def test_bitsets_are_little_endian_over_many_objects() -> None:
    present = [i % 3 == 0 or i == 17 for i in range(21)]
    store = build_store([{"k": i} if p else {"other": i} for i, p in enumerate(present)])
    stats = View.root(store).stats()
    bits = stats.bitset(store.key_id("k"))
    assert len(bits) == stats.n_bytes == 3
    assert np.unpackbits(bits, bitorder="little")[:21].astype(bool).tolist() == present
    np.testing.assert_array_equal(stats.presence[stats.key_ids.tolist().index(store.key_id("k"))], bits)


def reference_stats(docs: list, path: Path) -> dict:
    obs = [v for _, v, _ in walk(docs, path)]
    objects = [v for v in obs if isinstance(v, dict)]
    keys = sorted({k for obj in objects for k in obj})
    return {
        "types": Counter(int(jtype_of(v)) for v in obs),
        "values": Counter(canonical_key(v) for v in obs if jtype_of(v).is_scalar),
        "strings": Counter(len(v) for v in obs if isinstance(v, str)),
        "arrays": Counter(len(v) for v in obs if isinstance(v, list)),
        "objects": Counter(len(v) for v in obs if isinstance(v, dict)),
        "presence": {k: [k in obj for obj in objects] for k in keys},
    }


def observed_stats(store: ObservationStore, stats: ViewStats) -> dict:
    presence = {}
    for j, kid in enumerate(stats.key_ids.tolist()):
        row = np.unpackbits(stats.presence[j], bitorder="little")[: stats.n_objects].astype(bool)
        np.testing.assert_array_equal(row, np.unpackbits(stats.bitset(kid), bitorder="little")[: stats.n_objects])
        assert int(row.sum()) == int(stats.key_counts[j])
        presence[store.key_name(kid)] = row.tolist()
    return {
        "types": Counter({t: int(c) for t, c in enumerate(stats.type_counts.tolist()) if c}),
        "values": Counter({store.values.key(v): int(c) for v, c in zip(stats.value_ids.tolist(), stats.value_counts.tolist())}),
        "strings": Counter(hist(stats.string_lengths)),
        "arrays": Counter(hist(stats.array_lengths)),
        "objects": Counter(hist(stats.object_sizes)),
        "presence": presence,
    }


@given(st.data(), json_documents())
def test_stats_match_the_reference(data: st.DataObject, docs: list) -> None:
    store = build_store(docs, chunk_size=3)
    path = data.draw(st.sampled_from(present_paths(docs)))
    stats = at(store, path).stats()
    assert stats.n == len(walk(docs, path))
    assert observed_stats(store, stats) == reference_stats(docs, path)


def counts_only(stats: ViewStats) -> tuple:
    """Everything that does not depend on row positions (the multiset statistics)."""
    return (
        stats.n,
        stats.type_counts.tolist(),
        stats.value_ids.tolist(),
        stats.value_counts.tolist(),
        hist(stats.string_lengths),
        hist(stats.array_lengths),
        hist(stats.object_sizes),
        stats.key_ids.tolist(),
        stats.key_counts.tolist(),
    )


@given(json_documents(), st.randoms(use_true_random=False), st.integers(min_value=1, max_value=4))
def test_stats_do_not_depend_on_document_order_or_chunking(docs: list, random, chunk_size: int) -> None:
    shuffled = list(docs)
    random.shuffle(shuffled)
    a, b = build_store(docs), build_store(shuffled, chunk_size=chunk_size)
    for path in present_paths(docs, max_depth=3):
        assert counts_only(at(a, path).stats()) == counts_only(at(b, path).stats()), path


def test_stats_are_cached_by_digest_and_flag_epoch() -> None:
    store = build_store(DOCS)
    view = at(store, "$.a")
    first = view.stats()
    assert view.stats() is first
    assert View.union([view]).stats() is first  # another view with the same rows
    assert len(store.stats_cache) == 1

    store.flags.flag_documents([2])  # removes True from $.a
    second = view.stats()
    assert second is not first and second.n == 2 and second.count(JType.BOOLEAN) == 0
    assert view.stats() is second and len(store.stats_cache) == 1  # older epochs are dropped
    assert counts_only(second) == counts_only(compute_stats(view.live()))

    store.flags.flag_documents([3])  # not in $.a: new epoch, same contents
    third = view.stats()
    assert third is not second and counts_only(third) == counts_only(second)


def test_presence_ignores_flags_on_member_values() -> None:
    store = build_store(DOCS)
    a = at(store, "$.a")
    store.flags.flag_items((MAPVAL_STEP,), a.parts[0].rows[:1])  # document 0's "a" value
    root = View.root(store).stats()
    assert root.key_counts[root.key_ids.tolist().index(store.key_id("a"))] == 3
    assert hist(root.object_sizes) == {1: 1, 2: 2}
    assert at(store, "$.a").stats().n == 2 and a.stats().n == 2


def test_empty_view_stats() -> None:
    stats = View.root(build_store([])).stats()
    assert stats.n == 0 and stats.types == () and stats.type_counts.tolist() == [0] * 7
    assert stats.value_ids.tolist() == [] and stats.string_lengths.total == 0
    assert stats.presence.shape == (0, 0) and stats.bitset(0).tolist() == []
