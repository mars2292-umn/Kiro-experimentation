"""Unit tests of Exclusion_Rule mining (task 35.1; Requirements 11.1–11.7)."""

from __future__ import annotations

import json
import math
from itertools import combinations
from typing import Any

import pytest

from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.deps.exclusions import (
    EXCLUSION,
    ExclusionMiner,
    confidence_of,
    exclusion_candidates,
    expected_overlap,
    maximal_cliques,
    members_of,
    witnesses,
)
from jsinfer.deps.presence import Presence
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


def contacts(n: int = 40, *, none: int = 0) -> list[dict[str, Any]]:
    docs = []
    for i in range(n):
        d: dict[str, Any] = {"id": i}
        if i % 3 == 0:
            d["email"] = f"u{i}@x.org"
        elif i % 3 == 1:
            d["phone"] = f"+{i}"
        else:
            d["handle"] = f"@{i}"
        if i % 5 == 0:
            d["note"] = "n"
        docs.append(d)
    return docs + [{"id": 100 + j} for j in range(none)]


def presence(docs: list[Any]) -> Presence:
    return Presence.of(View.root(build_store(docs)))


def miner(threshold: float | None = 0.5, states: dict[str, bool] | None = None) -> tuple[ExclusionMiner, InferenceReport]:
    report = InferenceReport()
    engine = DecisionEngine(states, bundle=StubBundle({"exclusion.min_confidence": threshold} if threshold is not None else {}), report=report)
    return ExclusionMiner(engine, report=report, validator=VALIDATOR), report


def brute_cliques(n: int, adjacent: dict[int, set[int]]) -> list[tuple[int, ...]]:
    cliques = [c for size in range(1, n + 1) for c in combinations(range(n), size) if all(b in adjacent[a] for a, b in combinations(c, 2))]
    return sorted(c for c in cliques if not any(set(c) < set(d) for d in cliques))


def test_maximal_cliques_match_brute_force() -> None:
    adjacent = {0: {1, 2}, 1: {0, 2}, 2: {0, 1, 3}, 3: {2, 4}, 4: {3}, 5: set()}
    assert maximal_cliques(6, adjacent) == brute_cliques(6, adjacent) == [(0, 1, 2), (2, 3), (3, 4), (5,)]
    assert maximal_cliques(0, {}) == []
    assert maximal_cliques(3, {0: set(), 1: set(), 2: set()}) == [(0,), (1,), (2,)]


def test_members_fields_and_groups() -> None:
    p = presence(contacts())
    assert members_of(p) == [("email",), ("handle",), ("id",), ("note",), ("phone",)]
    assert members_of(p, [["phone", "handle"], ["email"], ["nope", "id"]]) == [("email",), ("handle", "phone"), ("id",), ("note",)]


def test_expected_overlap_and_confidence() -> None:
    assert expected_overlap([], 10) == 0.0
    assert expected_overlap([1.0, 1.0], 10) == pytest.approx(10.0)
    assert expected_overlap([0.5, 0.5], 10) == pytest.approx(2.5)
    assert expected_overlap([0.5, 0.5, 0.5], 8) == pytest.approx(8 * (1 - 0.125 - 0.375))
    assert confidence_of(0.0) == 0.0 and 0 < confidence_of(1.0) < confidence_of(2.0) < 1.0
    assert confidence_of(2.0) == pytest.approx(1 - math.exp(-2.0))
    with pytest.raises(ValueError):
        confidence_of(-1.0)


def test_candidates_are_maximal_exclusive_sets_with_confidence() -> None:
    [candidate] = exclusion_candidates(presence(contacts()))
    assert candidate.members == (("email",), ("handle",), ("phone",)) and candidate.exactly_one
    assert candidate.expected == pytest.approx(expected_overlap([14 / 40, 13 / 40, 13 / 40], 40))
    assert candidate.confidence == pytest.approx(confidence_of(candidate.expected))
    [loose] = exclusion_candidates(presence(contacts(none=2)))
    assert not loose.exactly_one
    assert exclusion_candidates(presence([{"a": 1, "b": 2}, {"a": 1}])) == []
    [grouped] = exclusion_candidates(presence(contacts()), groups=[["phone", "handle"]])
    assert grouped.members == (("email",), ("handle", "phone"))


def test_witnesses_combine_two_members_from_observed_objects() -> None:
    docs = contacts()
    [candidate] = exclusion_candidates(presence(docs))
    found = witnesses(candidate, docs)
    assert len(found) == 6 and all(sum(1 for m in candidate.members if any(n in w for n in m)) >= 2 for w in found)
    assert all("id" in w for w in found)


def test_miner_emits_one_of_when_every_object_holds_exactly_one_member() -> None:
    docs = contacts()
    m, report = miner(0.5)
    objects = View.root(build_store(docs))
    target = SchemaNode(kw={"type": ["object"], "properties": {k: SchemaNode(kw={}) for k in ("id", "email", "phone", "handle", "note")}, "required": ["id"]})
    rules = m.mine(objects, target=target)
    assert len(rules.emitted) == 1 and rules.emitted[0][1] == "oneOf" and rules.emitted[0][2].method == "mdl"
    records = rules.apply(target)
    assert [b.kw["required"] for b in target.kw["oneOf"]] == [["email"], ["handle"], ["phone"]]
    assert all(r.technique == EXCLUSION and r.details["form"] == "oneOf" and "expected_overlap" in r.details for r in records.values())
    assert {cid.keyword for cid in records} == {"oneOf", "required"}
    compiled = VALIDATOR.compile(target.to_json())
    assert all(VALIDATOR.is_valid(compiled, d) for d in docs) and not VALIDATOR.is_valid(compiled, {"id": 1, "email": "e", "phone": "p"})
    assert report.rejected == ()


def test_miner_emits_not_when_some_object_holds_no_member() -> None:
    docs = contacts(none=3)
    m, _ = miner(0.5)
    target = SchemaNode(kw={"type": ["object"], "properties": {k: SchemaNode(kw={}) for k in ("id", "email", "phone", "handle", "note")}, "required": ["id"]})
    rules = m.mine(View.root(build_store(docs)), target=target)
    [(candidate, form, _)] = rules.emitted
    assert form == "not" and not candidate.exactly_one
    records = rules.apply(target)
    sub = target.kw["not"]
    assert [b.kw["required"] for b in sub.kw["anyOf"]] == [["email", "handle"], ["email", "phone"], ["handle", "phone"]]
    assert {cid.keyword for cid in records} == {"not", "anyOf", "required"}
    compiled = VALIDATOR.compile(target.to_json())
    assert all(VALIDATOR.is_valid(compiled, d) for d in docs) and not VALIDATOR.is_valid(compiled, {"id": 1, "handle": "h", "phone": "p"})


def test_two_member_not_form_is_a_plain_required() -> None:
    docs = [{"a": 1} if i % 2 else {"b": 2} for i in range(20)] + [{}]
    m, _ = miner(0.5)
    target = SchemaNode(kw={"type": ["object"], "properties": {"a": SchemaNode(kw={}), "b": SchemaNode(kw={})}})
    rules = m.mine(View.root(build_store(docs)), target=target)
    rules.apply(target)
    assert target.kw["not"].kw == {"required": ["a", "b"]}


def test_already_enforced_candidates_are_discarded_and_reported() -> None:
    docs = contacts()
    m, report = miner(0.5)
    branches = [SchemaNode(kw={"type": ["object"], "required": ["id", name], "properties": {"id": SchemaNode(kw={}), name: SchemaNode(kw={})}, "additionalProperties": SchemaNode(boolean=False)}) for name in ("email", "handle", "phone")]
    target = SchemaNode(kw={"anyOf": branches})
    rules = m.mine(View.root(build_store(docs)), target=target, branches=branches)
    assert rules.emitted == () and len(rules.enforced) == 1
    [entry] = report.rejected
    assert entry.category == "rules" and entry.technique == EXCLUSION and entry.details["reason"] == "already enforced" and entry.details["branches"] == [0, 1, 2]
    assert "expected_overlap" in entry.details


def test_candidates_below_the_threshold_without_mdl_gain_are_rejected() -> None:
    # Two rare fields never co-occur by chance: E is small, the Confidence low, and the rule not worth its bits.
    docs = [{"id": i, "x": 1} if i == 0 else {"id": i, "y": 2} if i == 1 else {"id": i} for i in range(60)]
    m, report = miner(0.9)
    target = SchemaNode(kw={"type": ["object"], "properties": {"id": SchemaNode(kw={}), "x": SchemaNode(kw={}), "y": SchemaNode(kw={})}, "required": ["id"]})
    rules = m.mine(View.root(build_store(docs)), target=target)
    assert rules.emitted == () and len(rules.rejected_candidates) == 1
    [entry] = report.rejected
    assert entry.details["reason"] == "neither shorter nor confident" and entry.details["confidence"] < 0.9 and entry.details["threshold"] == 0.9
    accepted, _ = miner(0.0)
    assert len(accepted.mine(View.root(build_store(docs)), target=SchemaNode(kw=dict(target.kw))).emitted) == 1  # the cut-off admits it


def test_disabled_or_without_threshold_gives_the_fallback() -> None:
    m, report = miner(0.5, {EXCLUSION: False})
    rules = m.mine(View.root(build_store(contacts())), target=SchemaNode(kw={"type": ["object"]}))
    assert rules.is_empty and rules.fallback is not None and rules.fallback.method == "fallback" and report.rejected == ()
    m, _ = miner(None)
    assert not m.in_effect()


def test_analyzer_emits_exclusions_and_the_schema_stays_sound() -> None:
    docs = contacts(none=2)
    report = InferenceReport()
    sa = StructureAnalyzer.for_techniques(bundle=StubBundle({"exclusion.min_confidence": 0.5}), report=report)
    model = sa.infer_model(View.root(build_store(docs)))
    schema = model.root.to_json()
    assert "not" in schema, json.dumps(schema)[:300]
    compiled = VALIDATOR.compile(schema)
    assert all(VALIDATOR.is_valid(compiled, d) for d in docs) and not VALIDATOR.is_valid(compiled, {"id": 1, "email": "e", "phone": "p"})
    records = [r for cid, r in model.provenance.items() if r.technique == EXCLUSION]
    assert records and all(0.0 <= r.confidence < 1.0 or r.confidence == 1.0 for r in records)
    (entry,) = [e for e in report.decisions if e.kind == "exclusion_rule"][:1]
    assert entry.resolution.method in ("mdl", "priority")
    off = StructureAnalyzer.for_techniques({EXCLUSION: False}, bundle=StubBundle({"exclusion.min_confidence": 0.5})).infer_model(View.root(build_store(docs)))
    assert "not" not in off.root.to_json() and not any(r.technique == EXCLUSION for r in off.provenance.values())


def test_clique_and_member_caps_bound_the_search() -> None:
    # Pairs (a_i, b_i) co-occur; across pairs nothing does: the co-absence graph is complete minus a matching,
    # whose maximal cliques number 2^k (one of each pair). The caps make both the count and the members finite.
    k = 4
    docs = [{f"a{i}": 1, f"b{i}": 1} for i in range(k)]
    p = presence(docs)
    assert len(exclusion_candidates(p)) == 2**k
    capped = exclusion_candidates(p, max_cliques=3)
    assert len(capped) == 3 and capped == sorted(capped, key=lambda c: c.members)
    assert capped == exclusion_candidates(p, max_cliques=3)  # deterministic
    fewer = exclusion_candidates(p, max_members=2)
    assert all(len(c.fields) <= 2 for c in fewer) and fewer
    with pytest.raises(ValueError):
        exclusion_candidates(p, max_cliques=0)
    with pytest.raises(ValueError):
        maximal_cliques(2, {0: set(), 1: set()}, max_cliques=0)
