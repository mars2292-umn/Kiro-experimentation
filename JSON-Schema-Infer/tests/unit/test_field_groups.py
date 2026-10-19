"""Unit tests of Field_Group mining and selection (task 33.1; Requirements 6.2, 6.3, 6.6–6.9)."""

from __future__ import annotations

import json
from typing import Any

import pytest

from jsinfer.core.observations import build_store
from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.deps.presence import Presence
from jsinfer.io.validator import Validator
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.field_groups import (
    FPGROWTH,
    FieldGroupMiner,
    FieldGroups,
    clique_entries,
    code_table,
    frequent_itemsets,
    merge_records,
    transactions,
    violations,
)

VALIDATOR = Validator()


def planted(n: int = 40, *, broken: bool = False) -> list[dict[str, Any]]:
    docs = []
    for i in range(n):
        d: dict[str, Any] = {"id": i, "name": "n"}
        if i % 2 == 0:
            d.update({"street": "s", "city": "c", "zip": "z"})
        if i % 3 == 0:
            d.update({"phone": "p", "ext": "e"})
        if i % 7 == 0:
            d["note"] = "x"
        docs.append(d)
    if broken:
        docs[5]["zip"] = "alone"
    return docs


def presence(docs: list[Any]) -> Presence:
    return Presence.of(View.root(build_store(docs)))


def test_transactions_are_optional_key_sets_with_counts() -> None:
    found = transactions(presence(planted()))
    assert found[frozenset({"city", "street", "zip"})] == 11 and found[frozenset({"ext", "phone"})] == 6
    assert all("id" not in s and "name" not in s for s in found)
    assert sum(found.values()) == 40 - 11  # the 11 objects with no optional field make no transaction
    assert transactions(presence(planted()), base=["city", "street", "zip"]).get(frozenset({"ext", "phone"})) == 12


def test_frequent_itemsets_match_the_brute_force_count() -> None:
    weighted = {frozenset("ab"): 3, frozenset("abc"): 2, frozenset("bc"): 1, frozenset("d"): 1}
    found = frequent_itemsets(weighted, 2)
    assert found == {
        frozenset("a"): 5, frozenset("b"): 6, frozenset("c"): 3,
        frozenset("ab"): 5, frozenset("ac"): 2, frozenset("bc"): 3, frozenset("abc"): 2,
    }
    assert list(found)[:3] == [frozenset("a"), frozenset("b"), frozenset("c")]  # size, then code-point order
    assert frequent_itemsets({}, 2) == {} and frequent_itemsets(weighted, 7) == {}
    with pytest.raises(ValueError, match="min_support"):
        frequent_itemsets(weighted, 0)


def test_code_table_accepts_the_planted_groups_only() -> None:
    weighted = transactions(presence(planted()))
    frequent = {s: c for s, c in frequent_itemsets(weighted, 2).items() if len(s) >= 2}
    assert set(code_table(weighted, frequent)) == {frozenset({"city", "street", "zip"}), frozenset({"ext", "phone"})}
    assert code_table(weighted, {}) == [] and code_table({}, frequent) == []


def test_violations_and_clique_entries() -> None:
    p = presence(planted(broken=True))
    assert violations(p, ["city", "street", "zip"]) == 1 and violations(p, ["ext", "phone"]) == 0
    assert violations(p, ["id", "missing"]) == 40
    assert clique_entries([["b", "a"], ["z", "y", "x"]]) == {"a": ["b"], "b": ["a"], "x": ["y", "z"], "y": ["x", "z"], "z": ["x", "y"]}


def test_miner_selects_filters_and_reports() -> None:
    report = InferenceReport()
    out = FieldGroupMiner.for_techniques(report=report).mine(presence(planted(broken=True)), variant=2)
    assert out.selected == (("ext", "phone"),)
    assert out.decision is not None and out.decision.method == "mdl" and out.decision.label == "fp-growth"
    assert out.entries() == {"ext": ["phone"], "phone": ["ext"]} and out.keywords() == {"dependentRequired": out.entries()}
    [address] = [c for c in out.candidates if c.fields == ("city", "street", "zip")]
    assert address.excluded and address.violators == 1 and address.technique == FPGROWTH
    [entry] = report.rejected
    assert entry.category == "field_groups" and entry.path == "$" and entry.candidate == {"fields": ["city", "street", "zip"]}
    assert entry.details == {"violators": 1, "support_count": 20, "variant": 2}
    records = out.provenance
    assert set(records) == {"ext", "phone"} and all(r.technique == FPGROWTH for r in records.values())
    assert records["ext"].details["group"] == ["ext", "phone"] and records["ext"].details["variant"] == 2
    assert records["ext"].details["thresholds"] == {"fpgrowth.min_support": 2} and records["ext"].sample is None
    assert out.thresholds == {"fpgrowth.min_support": 2}


def test_miner_keeps_disjoint_groups_by_gain_and_selects_by_mdl() -> None:
    out = FieldGroupMiner.for_techniques().mine(presence(planted()))
    assert out.selected == (("city", "street", "zip"), ("ext", "phone"))
    groups = out.groups
    assert all(len(g) >= 2 for g in groups) and not any(a & b for a in groups for b in groups if a is not b)
    assert all(c.gain is not None and c.gain > 0 for c in out.candidates)


def test_fallback_when_every_serving_technique_is_off() -> None:
    report = InferenceReport()
    out = FieldGroupMiner.for_techniques({FPGROWTH: False, "groups.bmf": False}, report=report).mine(presence(planted()))
    assert out.is_empty and out.candidates == () and out.decision is not None and out.decision.method == "fallback"
    assert report.rejected == () and out.provenance == {}


def test_sampled_mining_records_the_sample_and_filters_on_every_object() -> None:
    report = InferenceReport()
    miner = FieldGroupMiner.for_techniques(report=report, resources={"sample.fpgrowth": 12}, seed=4)
    out = miner.mine(presence(planted(broken=True)))
    assert all(r.sample is not None and r.sample.sampled == 12 and r.sample.total == 40 for r in out.provenance.values())
    assert all(c.fields != ("city", "street", "zip") for c in out.candidates if not c.excluded)  # the violator is seen
    again = miner.mine(presence(planted(broken=True)))
    assert again.selected == out.selected


def test_min_support_resource_prunes_rare_groups() -> None:
    docs = planted() + [{"id": 99, "name": "n", "rare_a": 1, "rare_b": 2}]
    weighted = transactions(presence(docs))
    rare = frozenset({"rare_a", "rare_b"})
    assert rare in frequent_itemsets(weighted, 1) and rare not in frequent_itemsets(weighted, 2)
    high = FieldGroupMiner.for_techniques(resources={"fpgrowth.min_support": 2}).mine(presence(docs))
    assert not any(c.fields == ("rare_a", "rare_b") for c in high.candidates)
    assert high.thresholds == {"fpgrowth.min_support": 2}


def test_merge_records_keeps_the_group_record_with_the_rule_inside() -> None:
    group = ProvenanceRecord(FPGROWTH, Support(10, 10), 0.9, details={"group": ["a", "b"]})
    rule = ProvenanceRecord("rules.cooccurrence", Support(10, 10), 0.7, details={"antecedent": "a"})
    merged = merge_records(group, rule)
    assert merged.technique == FPGROWTH and merged.confidence == 0.7
    assert merged.details["group"] == ["a", "b"] and merged.details["cooccurrence"]["technique"] == "rules.cooccurrence"


def test_analyzer_emits_cliques_and_the_schema_accepts_every_document() -> None:
    docs = planted()
    report = InferenceReport()
    analyzer = StructureAnalyzer.for_techniques(report=report)
    model = analyzer.infer_model(View.root(build_store(docs)))
    schema = model.root.to_json()
    assert schema["required"] == ["id", "name"]
    assert schema["dependentRequired"] == {"city": ["street", "zip"], "ext": ["phone"], "phone": ["ext"], "street": ["city", "zip"], "zip": ["city", "street"]}
    compiled = VALIDATOR.compile({"type": "object", **schema})
    assert all(VALIDATOR.is_valid(compiled, d) for d in docs)
    records = {cid.item: r for cid, r in model.provenance.items() if cid.keyword == "dependentRequired"}
    assert set(records) == {"city", "ext", "phone", "street", "zip"} and all(r.technique == FPGROWTH for r in records.values())
    off_model = StructureAnalyzer.for_techniques({FPGROWTH: False}).infer_model(View.root(build_store(docs)))
    assert not any(r.technique == FPGROWTH for r in off_model.provenance.values())  # no Field_Group record when off
    off = off_model.root.to_json()
    assert off.get("dependentRequired", {}).get("ext") in (None, ["phone"])  # a co-occurrence rule may still say it


def test_field_groups_rejects_a_node_without_its_cliques() -> None:
    out = FieldGroupMiner.for_techniques().mine(presence(planted()))
    from jsinfer.core.schema_model import SchemaNode

    with pytest.raises(ValueError, match="dependentRequired"):
        out.constraint_provenance(SchemaNode(kw={"type": "object"}))
    assert FieldGroups(presence(planted())).keywords() == {}


def test_frequent_itemsets_are_capped_in_size_and_count() -> None:
    # 20 optional fields that always co-occur: 2^20 - 1 frequent itemsets without the caps.
    names = [f"f{i:02d}" for i in range(20)]
    weighted = {frozenset(names): 3}
    found = frequent_itemsets(weighted, 2, max_itemsets=500)
    assert len(found) == 500 and all(len(s) <= 8 for s in found)
    assert frequent_itemsets(weighted, 2, max_itemsets=500) == found  # the stop is deterministic
    small = frequent_itemsets(weighted, 2, max_size=2, max_itemsets=10_000)
    assert len(small) == 20 + 190
    docs = [{"id": i, **{name: 1 for name in names}} for i in range(6)] + [{"id": 9}]
    out = FieldGroupMiner.for_techniques(resources={"fpgrowth.max_itemsets": 200}).mine(presence(docs))
    assert out.thresholds == {"fpgrowth.min_support": 2}
    assert all(len(g) <= 8 for g in out.groups)
