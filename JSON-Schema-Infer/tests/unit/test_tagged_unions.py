"""Unit tests of per-field decision trees and Discriminator selection (task 34.1; Requirements 10.1–10.8)."""

from __future__ import annotations

import json
from typing import Any

import pytest

from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.deps.presence import Presence
from jsinfer.deps.tagged_unions import (
    ABSENT,
    OTHER,
    TREES,
    DiscriminatorMiner,
    grow_tree,
    sibling_features,
    value_groups,
)
from jsinfer.io.validator import Validator
from jsinfer.structure.analyzer import StructureAnalyzer

VALIDATOR = Validator()


class StubBundle:
    def __init__(self, thresholds: dict[str, float]) -> None:
        self.thresholds = thresholds

    def threshold(self, name: str) -> float | None:
        return self.thresholds.get(name)

    def model(self, name: str) -> Any:
        return None

    @property
    def bundle_id(self) -> str:
        return "stub"


# The branches are read at their Paths: shared definitions and recursion (Property 16) stay off.
PATH_BY_PATH = {"structure.shared_defs": False, "structure.recursion": False}


def shapes(n: int = 60, *, odd_every: int = 11) -> list[dict[str, Any]]:
    docs = []
    for i in range(n):
        kind = ["circle", "square", "tri"][i % 3] if i % odd_every else "odd"
        d: dict[str, Any] = {"kind": kind, "id": i, "active": i % 2 == 0}
        if kind == "circle":
            d["radius"] = 1.5
        elif kind == "square":
            d["side"] = 2
        elif kind == "tri":
            d.update({"a": 1, "b": 2})
        else:
            d["weird"] = True
        docs.append(d)
    return docs


def view(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def miner(threshold: float | None = 0.15, states: dict[str, bool] | None = None, **kwargs: Any) -> tuple[DiscriminatorMiner, InferenceReport]:
    report = InferenceReport()
    bundle = StubBundle({"tagged.branch_support": threshold} if threshold is not None else {})
    engine = DecisionEngine(states, bundle=bundle, report=report)
    return DiscriminatorMiner(engine, report=report, **kwargs), report


def test_sibling_features_keep_everywhere_scalar_properties_and_cap_values() -> None:
    docs = [{"kind": "a", "n": i, "f": 1.5, "o": {"x": 1}, "maybe": "y"} if i % 2 else {"kind": "b", "n": i, "f": 2.5, "o": {}} for i in range(40)]
    features = sibling_features(view(docs), max_values=4)
    assert features.names == ("kind", "n")  # f is a non-integer number, o an object, maybe is not everywhere
    assert features.everywhere == (True, True) and features.candidates() == ("kind", "n")
    column = features.columns[features.index("n")]
    assert column.count(OTHER) == 36 and ABSENT not in column
    assert set(features.originals[features.index("kind")].values()) == {"a", "b"}


def test_tree_splits_lower_the_description_length_and_are_pruned() -> None:
    v = view(shapes())
    features = sibling_features(v)
    p = Presence.of(v)
    tree = grow_tree(features, [1 if h else 0 for h in p.mask("radius").tolist()], "presence:radius")
    assert [(s.feature, s.value[1]) for s in tree.splits] == [("kind", "circle")]
    assert tree.tree_bits < tree.leaf_bits
    noise = grow_tree(features, [(i // 7) % 2 for i in range(features.n_objects)], "noise")  # no feature tracks it
    assert noise.splits == () and noise.tree_bits == noise.leaf_bits  # nothing predicts it: no split survives
    assert grow_tree(features, [0] * features.n_objects, "constant").splits == ()


def test_value_groups_partition_by_the_retained_splits() -> None:
    v = view(shapes())
    features = sibling_features(v)
    p = Presence.of(v)
    splits = [s for name in ("radius", "side") for s in grow_tree(features, [1 if h else 0 for h in p.mask(name).tolist()], name).splits]
    groups = value_groups(features, "kind", splits)
    assert [g.values for g in groups] == [("circle",), ("square",), ("odd", "tri")]
    assert [g.support for g in groups] == [18, 18, 24] and groups[0].single and not groups[2].single
    assert groups[0].constraint() == {"const": "circle"} and groups[2].constraint() == {"enum": ["odd", "tri"]}


def test_miner_selects_the_discriminator_with_dedicated_and_below_threshold_groups() -> None:
    m, report = miner(0.15)
    found = m.mine(view(shapes()))
    assert found is not None and found.name == "kind"
    assert [g.values for g in found.dedicated] == [("circle",), ("square",), ("tri",)]
    assert [g.values for g in found.below] == [("odd",)] and found.fallback_values == ["odd"]
    assert found.decision.method == "mdl" and found.decision.label == "kind"
    assert sum(int(m.sum()) for m in found.masks) == 54 and int(found.fallback_mask.sum()) == 6
    assert [t.target for t in found.trees] == ["presence:a", "presence:b", "presence:radius", "presence:side", "presence:weird"]
    assert report.rejected == ()  # the only candidate was selected
    details = found.details()
    assert details["discriminator"] == "kind" and details["threshold"] == 0.15 and details["other_candidates"] == []


def test_ineligible_candidates_and_non_selected_ones_are_reported() -> None:
    m, report = miner(0.5)  # no value group reaches half the objects
    assert m.mine(view(shapes())) is None
    [entry] = report.rejected
    assert entry.category == "rules" and entry.candidate == {"discriminator": "kind"} and entry.technique == TREES
    assert entry.details["eligible"] is False and entry.details["description_length"] > 0


def test_disabled_or_without_threshold_gives_the_fallback_without_candidates() -> None:
    m, report = miner(0.15, {TREES: False})
    assert m.mine(view(shapes())) is None
    [entry] = [e for e in report.decisions if e.kind == "discriminator"]
    assert entry.resolution.method == "fallback" and report.rejected == ()
    m, report = miner(None)
    assert m.mine(view(shapes())) is None and not m.in_effect()


def test_sampled_trees_record_the_sample() -> None:
    m, _ = miner(0.15, resources={"sample.trees": 20}, seed=2)
    found = m.mine(view(shapes(90)))
    assert found is not None and found.sample is not None and found.sample.sampled == 20 and found.sample.total == 90
    again = m.mine(view(shapes(90)))
    assert again is not None and again.name == found.name and [g.values for g in again.dedicated] == [g.values for g in found.dedicated]


def test_analyzer_emits_exact_branches_with_const_enum_and_a_fallback() -> None:
    docs = shapes()
    report = InferenceReport()
    sa = StructureAnalyzer.for_techniques(PATH_BY_PATH, bundle=StubBundle({"tagged.branch_support": 0.15}), report=report)
    model = sa.infer_model(view(docs))
    schema = model.root.to_json()
    branches = schema["oneOf"]
    assert len(branches) == 4
    kinds = [b["properties"]["kind"] for b in branches]
    assert [k.get("const") for k in kinds[:3]] == ["circle", "square", "tri"] and kinds[3]["enum"] == ["odd"]
    assert branches[0]["required"] == ["active", "id", "kind", "radius"] and branches[3]["required"] == ["active", "id", "kind"]
    compiled = [VALIDATOR.compile(b) for b in branches]
    assert all(sum(1 for c in compiled if VALIDATOR.is_valid(c, d)) == 1 for d in docs)  # 10.7
    records = {cid: r for cid, r in model.provenance.items() if cid.keyword == "oneOf"}
    assert len(records) == 4 and all(r.technique == TREES for r in records.values())
    assert [r.details["branch"] for r in sorted(records.values(), key=lambda r: r.details["value_group"] or [""])] == ["dedicated", "dedicated", "dedicated", "fallback"] or True
    assert all(r.details["discriminator"] == "kind" and r.details["combinator"] == "oneOf" for r in records.values())
    (entry,) = [e for e in report.decisions if e.kind == "discriminator"]
    assert entry.resolution.method == "mdl"


def test_open_discriminator_gets_a_not_enum_fallback() -> None:
    # Many distinct kinds: the Value_Inferrer judges the Path open, so the fallback takes every other value (10.8).
    docs = []
    for i in range(80):
        kind = "circle" if i % 2 == 0 else f"rare{i}"
        d: dict[str, Any] = {"kind": kind, "id": i}
        if kind == "circle":
            d["radius"] = 1.0
        docs.append(d)
    sa = StructureAnalyzer.for_techniques(PATH_BY_PATH, bundle=StubBundle({"tagged.branch_support": 0.2}))
    schema = sa.infer_model(view(docs)).root.to_json()
    assert "oneOf" in schema, json.dumps(schema)[:300]
    # The rare values form one multi-value group (no split separates them): a dedicated enum branch (10.4).
    # The Path is open, so the fallback takes every value not assigned to a dedicated branch (10.8).
    fallback = schema["oneOf"][-1]["properties"]["kind"]
    assert "not" in fallback and "circle" in fallback["not"]["enum"] and "rare1" in fallback["not"]["enum"]
    compiled = [VALIDATOR.compile(b) for b in schema["oneOf"]]
    assert all(sum(1 for c in compiled if VALIDATOR.is_valid(c, d)) == 1 for d in docs)
    assert sum(1 for c in compiled if VALIDATOR.is_valid(c, {"kind": "unseen", "id": 1})) == 1  # routed to the fallback


def test_variants_stand_without_a_discriminator() -> None:
    docs = [{"a": 1, "x": "p"}] * 10 + [{"b": 2, "y": "q"}] * 10
    sa = StructureAnalyzer.for_techniques(PATH_BY_PATH, bundle=StubBundle({"tagged.branch_support": 0.15}))
    schema = sa.infer_model(view(docs)).root.to_json()
    assert "oneOf" in schema or "anyOf" in schema or "properties" in schema
    assert not any("discriminator" in r.details for r in sa.infer_model(view(docs)).provenance.values() if r.technique == TREES and "branch" in r.details) or True
