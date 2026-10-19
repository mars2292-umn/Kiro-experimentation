"""Unit tests of the hybrid Record/Map partition (task 37.1; Requirements 4.5, 4.6, 4.7)."""

from __future__ import annotations

import random
from typing import Any

import pytest

from jsinfer.config.run_config import RunConfiguration
from jsinfer.io.validator import Validator
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.record_map import HYBRID, PARTITION_PATTERN, RECORD_MAP_PARTITION, Partition, key_frequencies

VALIDATOR = Validator()


def sparse_keyed(n: int = 60, *, seed: int = 1) -> list[dict[str, Any]]:
    """Objects with two stable fields and a few of 120 eight-digit keys each, holding one value shape."""
    rng = random.Random(seed)
    keys = [f"{rng.randrange(10**8):08d}" for _ in range(120)]
    docs: list[dict[str, Any]] = []
    for i in range(n):
        doc: dict[str, Any] = {"id": i, "kind": rng.choice(["a", "b"])}
        for key in rng.sample(keys, 4):
            doc[key] = {"qty": rng.randrange(1, 9), "ok": rng.random() < 0.5}
        docs.append(doc)
    return docs


def run(docs: list[dict[str, Any]], **flags: Any) -> PipelineResult:
    return infer_documents(docs, RunConfiguration.resolve({"workers": 1, **flags}))


def partition_entries(result: PipelineResult, path: str = "$") -> list[Any]:
    return [e for e in result.report.decisions if e.kind == RECORD_MAP_PARTITION and e.path == path]


def test_sparse_keys_beside_stable_fields_partition_under_a_pattern() -> None:
    docs = sparse_keyed()
    result = run(docs)
    doc = result.document
    assert isinstance(doc, dict)
    assert sorted(doc["properties"]) == ["id", "kind"] and doc["required"] == ["id", "kind"]
    (pattern,) = doc["patternProperties"]
    assert all(VALIDATOR.matches_pattern(pattern, key) for d in docs for key in d if key not in ("id", "kind"))
    assert not any(VALIDATOR.matches_pattern(pattern, name) for name in ("id", "kind"))
    assert doc["patternProperties"][pattern]["properties"].keys() == {"ok", "qty"}
    assert "additionalProperties" not in doc  # judged open (4.7, 4.9); a closed one would add false
    compiled = VALIDATOR.compile(doc)
    assert all(VALIDATOR.is_valid(compiled, d) for d in docs)
    # 4.5: the partition is recorded by MDL, strictly below both pure encodings, with the no-partition option scored.
    (entry,) = partition_entries(result)
    assert entry.resolution.method == "mdl"
    selected = entry.resolution.candidates[entry.resolution.selected]
    assert selected.outcome == PARTITION_PATTERN
    pure = entry.details["pure_bits"]
    assert selected.model_bits + selected.data_bits < pure
    none = next(c for c in entry.resolution.candidates if c.outcome == "none")
    assert none.model_bits + none.data_bits == pytest.approx(pure)
    assert all(c.model_bits + c.data_bits < pure for c in entry.resolution.candidates if c.outcome != "none")
    # The partition decision is the record of both parts; Closed-or-Open was resolved for the partitioned Record.
    root = result.model.root
    records = {cid.keyword: record for cid, record in result.model.provenance.items() if cid.nid == root.nid}
    assert records["properties"].technique == HYBRID and records["patternProperties"].technique == HYBRID
    assert records["properties"].details["judgment"] == "hybrid" and records["properties"].details["pattern"] == pattern
    assert [e for e in result.report.decisions if e.kind == "closed_or_open_record" and e.path == "$"]


def test_disabled_partition_records_its_fallback_and_keeps_the_pure_outcome() -> None:
    result = run(sparse_keyed(), **{"techniques.structure.hybrid_record_map": False})
    doc = result.document
    assert isinstance(doc, dict)
    assert "patternProperties" not in doc and ("properties" in doc) != ("additionalProperties" in doc and "properties" not in doc)
    (entry,) = partition_entries(result)
    assert entry.resolution.method == "fallback" and entry.fallback.reason == "disabled"


def test_a_plain_record_keeps_no_partition() -> None:
    docs = [{"id": i, "name": f"n{i}", "active": i % 2 == 0} for i in range(30)]
    result = run(docs)
    doc = result.document
    assert isinstance(doc, dict)
    assert sorted(doc["properties"]) == ["active", "id", "name"] and "patternProperties" not in doc
    assert not isinstance(doc.get("additionalProperties"), dict)
    (entry,) = partition_entries(result)
    assert entry.resolution.method == "mdl"
    assert entry.resolution.candidates[entry.resolution.selected].outcome == "none"
    assert entry.details["n_keys"] == 3 and entry.details["evaluated"] >= 2


def test_key_frequencies_order_and_partition_validation() -> None:
    from jsinfer.core.observations import build_store
    from jsinfer.core.views import View

    store = build_store([{"b": 1, "a": 1}, {"a": 2, "c": 3}, {"a": 3}])
    assert key_frequencies(View.root(store)) == [("a", 3), ("b", 1), ("c", 1)]
    with pytest.raises(ValueError):
        Partition((), ("a",), "top-0")
    with pytest.raises(ValueError):
        Partition(("a",), ("a",), "top-1")
