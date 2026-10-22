"""Property 37: Leakage guards (task 39.5).

Weak supervision (23.6): for any unlabeled-collection manifest and any
Real_Dataset identity, no kept collection has a Real_Dataset's id, no kept
document equals (as a parsed JSON value) a Real_Dataset document, every
other document is kept in order, the removed counts add up, and every
manifest entry is either kept or excluded with a reason.

The harness (30.7, 30.8, 30.11): a training manifest whose
``unlabeled_collections`` names a Real_Dataset collection, whose schemas
include a held-out group member, or whose schemas are missing from the
corpus makes ``check_leakage`` abort, listing exactly those offenders; a
manifest with none passes.

Inputs
    Collections of small JSON documents drawn from a shared pool (so some
    equal a Real_Dataset document), ids drawn from a small alphabet (so
    duplicates and Real_Dataset ids occur), and training manifests drawn
    over a fixed split.

**Validates: Requirements 23.6, 30.7, 30.8, 30.11**
"""

# Feature: json-schema-inference, Property 37: Leakage guards

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.bench.leakage import LeakageError, check_leakage, held_out_groups
from jsinfer.bench.splits import SchemaSplit
from jsinfer.core.determinism import canonical_bytes
from jsinfer.learn.weak_supervision import RealIdentity, load_unlabeled

POOL = [{"a": 1}, {"a": 2}, {"b": [1, 2]}, {"b": []}, [1, "x"], "s", 3, None, {"c": {"d": True}}]
IDS = ["c1", "c2", "c3", "c4"]


@st.composite
def unlabeled_cases(draw: st.DrawFn) -> tuple[list[tuple[str, list]], RealIdentity]:
    entries = draw(st.lists(st.tuples(st.sampled_from(IDS), st.lists(st.sampled_from(POOL), max_size=8)), max_size=6))
    real_ids = draw(st.frozensets(st.sampled_from(IDS)))
    real_docs = draw(st.frozensets(st.sampled_from(range(len(POOL)))))
    return entries, RealIdentity(real_ids, frozenset(canonical_bytes(POOL[i]) for i in real_docs))


@given(unlabeled_cases())
def test_weak_supervision_never_reads_a_real_dataset(case: tuple[list[tuple[str, list]], RealIdentity]) -> None:
    entries, identity = case
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        listed = []
        for i, (cid, docs) in enumerate(entries):
            (root / f"{i}.jsonl").write_text("".join(json.dumps(d) + "\n" for d in docs))
            listed.append({"id": cid, "path": f"{i}.jsonl"})
        (root / "manifest.json").write_text(json.dumps({"collections": listed}))
        loaded = load_unlabeled(str(root / "manifest.json"), identity)
    assert len(loaded.collections) + len(loaded.excluded) == len(entries)
    assert all(e["reason"] for e in loaded.excluded)
    first = {}
    for cid, docs in entries:
        first.setdefault(cid, docs)
    for collection in loaded.collections:
        assert collection.id not in identity.ids
        assert not any(canonical_bytes(d) in identity.documents for d in collection.documents)
        expected = [d for d in first[collection.id] if canonical_bytes(d) not in identity.documents]
        assert list(collection.documents) == expected
        assert collection.removed == len(first[collection.id]) - len(expected)


SPLIT = SchemaSplit(
    "s", 0, ("a", "b", "c"), ("d",), ("h1", "h2"),
    {"groups": {"a": "a", "b": "a", "c": "c", "d": "d", "h1": "h1", "h2": "h2", "z": "h2"}},
)
CORPUS = ("a", "b", "c", "d", "h1", "h2", "z")
REAL = ("real-1", "real-2")


@given(
    st.lists(st.sampled_from([*CORPUS, "ghost"]), min_size=1, max_size=6, unique=True),
    st.lists(st.sampled_from(["col-1", "col-2", *REAL]), max_size=4, unique=True),
)
def test_the_harness_aborts_on_exactly_the_offenders(schemas: list[str], collections: list[str]) -> None:
    manifest = {"schemas": {"train": schemas, "val": []}, "unlabeled_collections": [{"id": c} for c in collections]}
    unknown = [s for s in schemas if s not in CORPUS]
    held_out = {s: g for s, g in held_out_groups(SPLIT).items() if s in schemas}
    leaked = [c for c in collections if c in REAL]
    if not (unknown or held_out or leaked):
        report = check_leakage(manifest, SPLIT, CORPUS, real_collections=REAL)
        assert set(report.collections) == set(collections)
        return
    with pytest.raises(LeakageError) as info:
        check_leakage(manifest, SPLIT, CORPUS, real_collections=REAL)
    assert set(info.value.unknown) == set(unknown)
    assert dict(info.value.held_out) == held_out
    assert set(info.value.collections) == set(leaked)
