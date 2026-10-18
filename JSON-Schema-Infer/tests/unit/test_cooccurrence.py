"""Unit tests for deps/cooccurrence.py: Co_Occurrence_Rule candidates, selection, and reduction (task 14.2).

Requirements 9.3 (candidates from antecedent ⊆ consequent with an optional
consequent), 9.4 (MDL selection: a rule only when it lowers the
description length), 9.5 (the emitted subset derives every selected rule
and is irredundant), 9.6 (candidates below ``cooccur.min_support`` are
discarded and reported), 9.8 (the Confidence threshold while MDL selection
is disabled), and 9.9 (no candidates while the Technique is disabled).
"""

from __future__ import annotations

import itertools
import math
import random
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import OUTCOME_NONE, REGISTRY
from jsinfer.config.thresholds import THRESHOLDS
from jsinfer.core.observations import build_store
from jsinfer.core.paths import MAPVAL_STEP, render_path
from jsinfer.core.provenance import MDL_COMPARISON, FallbackInfo
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import mdl_posterior
from jsinfer.deps.cooccurrence import (
    COOCCURRENCE,
    COOCCURRENCE_RULE,
    DEPENDENT_REQUIRED,
    MIN_CONFIDENCE,
    MIN_SUPPORT,
    NO_RULE,
    CooccurrenceMiner,
    RuleCandidate,
    group_rules,
    reduce_rules,
    rule_candidates,
    rule_confidence,
)
from jsinfer.deps.presence import presence, required_fields
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.pipeline.naive_merge import naive_merge_node

VALIDATOR = Validator()
MDL = "decide.mdl"
THRESHOLDS_OK = {MIN_SUPPORT: 2, MIN_CONFIDENCE: 0.5}

# Address fields travel together on odd ids; "note" sits on every seventh object.
ADDRESSES = [
    {"id": i, **({"street": "s", "city": "c", "zip": i} if i % 2 else {}), **({"note": "n"} if i % 7 == 0 else {})}
    for i in range(60)
]
# A chain: "c" implies "b", which implies "a" (so "c" implies "a" too); "d" is held once.
CHAIN = [
    {"id": i, **({"a": 1} if i % 2 else {}), **({"b": 1} if i % 4 == 1 else {}), **({"c": 1} if i % 8 == 1 else {})}
    for i in range(160)
] + [{"id": 999, "a": 1, "b": 1, "c": 1, "d": 1}]


class StubBundle:
    """Learned_Thresholds by name, like ResolvedBundle.threshold; no Decision_Models."""

    def __init__(self, thresholds: dict[str, int | float]) -> None:
        self.thresholds = thresholds

    def model(self, name: str) -> Any | None:
        return None

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def miner(
    states: dict[str, bool] | None = None, thresholds: dict[str, int | float] | None = THRESHOLDS_OK
) -> tuple[CooccurrenceMiner, InferenceReport]:
    report = InferenceReport()
    bundle = None if thresholds is None else StubBundle(thresholds)
    return CooccurrenceMiner.for_techniques(states, bundle=bundle, report=report, validator=VALIDATOR), report


def brute_force_candidates(objects: list[dict[str, Any]]) -> list[tuple[str, str, int]]:
    """The 9.3 candidates by exhaustive search: (antecedent, consequent, Support count) in code-point order."""
    keys = sorted({key for obj in objects for key in obj})
    out = []
    for a in keys:
        holders = [obj for obj in objects if a in obj]
        for b in keys:
            optional = not all(b in obj for obj in objects)
            if a != b and optional and all(b in obj for obj in holders):
                out.append((a, b, len(holders)))
    return out


def derives(rules: list[tuple[str, str]], rule: tuple[str, str]) -> bool:
    """Whether ``rule`` follows from ``rules`` by transitive chaining."""
    reach = {rule[0]}
    todo = [rule[0]]
    while todo:
        node = todo.pop()
        for a, b in rules:
            if a == node and b not in reach:
                reach.add(b)
                todo.append(b)
    return rule[1] in reach


def check_reduction(selected: list[tuple[str, str]], given: list[tuple[str, str]], emitted: tuple) -> None:
    """9.5: emitted ⊆ selected, (a) every selected rule is derived, (b) no emitted rule is derivable from the rest."""
    emitted_list = list(emitted)
    assert all(rule in selected for rule in emitted_list)
    for rule in selected:
        assert derives([*emitted_list, *given], rule), rule
    for rule in emitted_list:
        rest = [other for other in emitted_list if other != rule]
        assert not derives([*rest, *given], rule), rule


# ---------------------------------------------------------------------------
# Registry and thresholds
# ---------------------------------------------------------------------------


def test_registry_entry() -> None:
    spec = REGISTRY.technique(COOCCURRENCE)
    assert spec.implemented and spec.default_enabled and spec.tier == "must"
    assert spec.serves == (COOCCURRENCE_RULE,) and set(spec.thresholds) == {MIN_SUPPORT, MIN_CONFIDENCE}
    point = REGISTRY.decision_point(COOCCURRENCE_RULE)
    assert point.served_by == (COOCCURRENCE,) and point.fallback() == OUTCOME_NONE == NO_RULE
    assert point.tie_rule is None  # adding an entry costs model bits, so ties go to "none" anyway
    for name in (MIN_SUPPORT, MIN_CONFIDENCE):
        threshold = THRESHOLDS[name]
        assert threshold.techniques == (COOCCURRENCE,) and threshold.decision_point == COOCCURRENCE_RULE
        assert threshold.objective == "cooccurrence_f1"  # the Trainer's validation objective
    assert THRESHOLDS[MIN_SUPPORT].integer and not THRESHOLDS[MIN_CONFIDENCE].integer


# ---------------------------------------------------------------------------
# Candidates (9.3)
# ---------------------------------------------------------------------------


def test_candidates_of_a_field_group_and_a_rare_field() -> None:
    candidates = rule_candidates(root(ADDRESSES))
    assert [c.rule for c in candidates] == [
        ("city", "street"), ("city", "zip"), ("street", "city"), ("street", "zip"), ("zip", "city"), ("zip", "street"),
    ]
    assert {c.support_count for c in candidates} == {30}
    # "id" is required, so it is never a consequent; "note" co-occurs with nothing every time.
    assert not any("id" in c.rule or "note" in c.rule for c in candidates)


def test_candidates_match_a_brute_force_reference() -> None:
    gen = random.Random(92)
    keys = ["a", "B", "c", "é", "d-e"]
    for _ in range(25):
        docs: list[Any] = []
        for _ in range(gen.randrange(1, 30)):
            if gen.random() < 0.1:
                docs.append(gen.choice([1, "s", [1], None]))  # non-objects at the Path are ignored
                continue
            # Nested chains a ⊆ B ⊆ c are common, so implications occur often.
            level = gen.random()
            doc = {k: gen.choice([None, 0, "x"]) for k, cut in zip(keys[:3], (0.3, 0.6, 0.8)) if level < cut}
            doc.update({k: 1 for k in keys[3:] if gen.random() < 0.3})
            docs.append(doc)
        objects = [doc for doc in docs if isinstance(doc, dict)]
        if not objects:
            continue
        found = [(c.antecedent, c.consequent, c.support_count) for c in rule_candidates(root(docs))]
        assert found == brute_force_candidates(objects)


def test_null_values_are_present_and_flagged_objects_do_not_count() -> None:
    store = build_store([{"a": None, "b": None}, {"a": 1, "b": 2}, {"c": 1}, {"a": 1, "c": 2}])
    assert [c.rule for c in rule_candidates(View.root(store))] == [("b", "a")]
    store.flags.flag_documents([3])  # the only object holding "a" without "b"
    assert [c.rule for c in rule_candidates(View.root(store))] == [("a", "b"), ("b", "a")]


def test_a_flagged_member_value_keeps_its_key_present() -> None:
    store = build_store([{"a": 1, "b": 2}, {"a": 3, "b": 4}, {"c": 1}])
    b = View.root(store).child_key("b")
    store.flags.flag_items((MAPVAL_STEP,), b.parts[0].rows[:1])
    assert [c.rule for c in rule_candidates(View.root(store))] == [("a", "b"), ("b", "a")]


def test_no_candidates_without_optional_fields() -> None:
    assert rule_candidates(root([{"a": 1, "b": 2}] * 3)) == ()
    assert rule_candidates(root([{}, {}])) == ()
    assert rule_candidates(presence(root([{"a": 1}, {"b": 1}]))) == ()  # disjoint optional fields


def test_rule_candidate_fields() -> None:
    candidate = RuleCandidate("é", "a\"b", 4)
    assert candidate.rule == ("é", 'a"b') and candidate.entry == {"é": ['a"b']}
    assert candidate.label == '{"é":["a\\"b"]}'
    assert candidate.confidence == rule_confidence(4) == 5 / 6
    assert candidate.to_json() == {"antecedent": "é", "consequent": 'a"b'}
    assert rule_confidence(1) < rule_confidence(2) < rule_confidence(100) < 1  # grows with the Support count
    with pytest.raises(ValueError, match="two different fields"):
        RuleCandidate("a", "a", 1)
    for bad in (0, -1, True, 1.5):
        with pytest.raises(ValueError, match="positive integer"):
            RuleCandidate("a", "b", bad)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Support filter (9.6)
# ---------------------------------------------------------------------------


def test_candidates_below_min_support_are_discarded_and_reported() -> None:
    m, report = miner(thresholds={MIN_SUPPORT: 3, MIN_CONFIDENCE: 0.5})
    rules = m.rules(root(CHAIN), variant=2)
    low = [c.rule for c in rules.discarded]
    assert low and all(c.support_count < 3 for c in rules.discarded)
    assert ("d", "a") in low and ("d", "c") in low  # "d" is held once
    assert all(e.candidate.support_count >= 3 for e in rules.evaluations)
    assert len(rules.discarded) + len(rules.evaluations) == len(rules.candidates)
    assert rules.thresholds[MIN_SUPPORT] == 3
    entries = [entry for entry in report.rejected if entry.details["threshold"] == MIN_SUPPORT]
    assert len(entries) == len(low)
    for entry in entries:
        assert entry.category == "rules" and entry.path == "$" and entry.technique == COOCCURRENCE
        assert (entry.candidate["antecedent"], entry.candidate["consequent"]) in low
        assert entry.details["support_count"] == 1 and entry.details["threshold_value"] == 3
        assert entry.details["variant"] == 2
    # Discarded candidates are not Decision_Points.
    assert len(report.decisions) == len(rules.evaluations)


def test_min_support_at_the_count_keeps_the_candidate() -> None:
    m, _ = miner(thresholds={MIN_SUPPORT: 30, MIN_CONFIDENCE: 0.5})
    rules = m.rules(root(ADDRESSES))
    assert not rules.discarded and len(rules.evaluations) == 6
    m, _ = miner(thresholds={MIN_SUPPORT: 31, MIN_CONFIDENCE: 0.5})
    rules = m.rules(root(ADDRESSES))
    assert len(rules.discarded) == 6 and rules.evaluations == () and rules.is_empty


# ---------------------------------------------------------------------------
# MDL selection (9.4)
# ---------------------------------------------------------------------------


def with_entry(node: SchemaNode, antecedent: str, consequent: str) -> SchemaNode:
    schema = node.to_json()
    assert isinstance(schema, dict)
    entries = schema.setdefault(DEPENDENT_REQUIRED, {})
    entries.setdefault(antecedent, []).append(consequent)
    return SchemaNode.from_json(schema)


@pytest.mark.parametrize("docs", [ADDRESSES, CHAIN], ids=["addresses", "chain"])
def test_mdl_selects_exactly_the_rules_that_lower_the_description_length(docs: list[Any]) -> None:
    m, report = miner(thresholds={MIN_SUPPORT: 1, MIN_CONFIDENCE: 0.5})
    view = root(docs)
    rules = m.rules(view)
    objects = presence(view).objects
    scorer = MDLScorer(validator=VALIDATOR)
    base = naive_merge_node(objects)
    without = scorer.score(base, objects).total
    assert rules.evaluations
    for evaluation in rules.evaluations:
        a, b = evaluation.candidate.rule
        full = scorer.score(with_entry(base, a, b), objects)
        assert evaluation.selected == (full.total < without)  # strictly lower (9.4)
        decision = evaluation.decision
        assert decision is not None and decision.method == "mdl" and decision.kind == COOCCURRENCE_RULE
        none, rule = decision.resolution.candidates
        assert (none.outcome, none.technique) == (NO_RULE, None)
        assert (rule.outcome, rule.technique) == (evaluation.candidate.label, COOCCURRENCE)
        # The composed score equals scoring the rule's node in full.
        assert math.isclose(rule.score, full.total, rel_tol=1e-12)
        assert none.score == without and rule.model_bits == full.model_bits
        assert decision.cutoff is not None and decision.cutoff.source == MDL_COMPARISON
        if evaluation.selected:
            assert decision.technique == COOCCURRENCE
            assert evaluation.confidence == decision.confidence == mdl_posterior([none.score, rule.score])[1]
        else:
            assert decision.label == NO_RULE
    assert [entry.kind for entry in report.decisions] == [COOCCURRENCE_RULE] * len(rules.evaluations)


def test_a_rule_that_saves_no_bits_is_not_selected() -> None:
    # "b" is held by one object of 40 with "a": coding it given "a" saves less than the entry costs.
    docs: list[Any] = [{"a": 1, "b": 1}] + [{"a": 1}] * 9 + [{"c": 1}] * 30
    m, _ = miner(thresholds={MIN_SUPPORT: 1, MIN_CONFIDENCE: 0.5})
    rules = m.rules(root(docs))
    assert [e.candidate.rule for e in rules.evaluations] == [("b", "a")]
    assert rules.selected == () and rules.keywords() == {}


def test_mdl_scores_against_the_callers_record() -> None:
    view = root(ADDRESSES)
    record = naive_merge_node(view)
    record.kw.pop("required")
    record.kw.update(required_fields(view).keywords())
    record.kw["additionalProperties"] = SchemaNode(boolean=False)  # a closed Record
    m, report = miner()
    rules = m.rules(view, record=record)
    scorer = MDLScorer(validator=VALIDATOR)
    without = scorer.score(record, view).total
    for evaluation in rules.evaluations:
        none, rule = evaluation.decision.resolution.candidates  # type: ignore[union-attr]
        assert none.score == without
        full = scorer.score(with_entry(record, *evaluation.candidate.rule), view).total
        assert math.isclose(rule.score, full, rel_tol=1e-12)
    with pytest.raises(ValueError, match="every field"):
        m.rules(view, record=SchemaNode(kw={"properties": {"id": SchemaNode()}}))
    with pytest.raises(ValueError, match="keyword schema"):
        m.rules(view, record=SchemaNode(boolean=True))


# ---------------------------------------------------------------------------
# Confidence threshold (9.8)
# ---------------------------------------------------------------------------


def test_without_mdl_rules_at_or_above_min_confidence_are_selected() -> None:
    support = 30  # every address candidate
    at = rule_confidence(support)
    m, report = miner({MDL: False}, thresholds={MIN_SUPPORT: 1, MIN_CONFIDENCE: at})
    rules = m.rules(root(ADDRESSES))
    assert len(rules.selected) == 6 and rules.thresholds == {MIN_SUPPORT: 1, MIN_CONFIDENCE: at}
    for evaluation in rules.evaluations:
        decision = evaluation.decision
        assert decision is not None and decision.method == "priority" and decision.technique == COOCCURRENCE
        assert decision.cutoff is not None
        assert (decision.cutoff.source, decision.cutoff.value) == (MIN_CONFIDENCE, at)  # 20.7
        assert decision.fallback is None and evaluation.confidence == at
    record = rules.provenance["city"]
    assert record.confidence == at and record.cutoff.source == MIN_CONFIDENCE  # type: ignore[union-attr]
    assert record.resolution is not None and record.resolution.method == "priority"
    assert report.rejected == ()


def test_without_mdl_rules_below_min_confidence_are_rejected_and_reported() -> None:
    m, report = miner({MDL: False}, thresholds={MIN_SUPPORT: 1, MIN_CONFIDENCE: rule_confidence(3)})
    rules = m.rules(root(CHAIN), variant=1)
    low = [e for e in rules.evaluations if e.candidate.support_count < 3]
    assert low and all(not e.selected and e.decision is None for e in low)
    assert all(e.selected for e in rules.evaluations if e.candidate.support_count >= 3)
    rejected = {(r.candidate["antecedent"], r.candidate["consequent"]): r for r in report.rejected}
    assert sorted(rejected) == sorted(e.candidate.rule for e in low)
    for evaluation in low:
        entry = rejected[evaluation.candidate.rule]
        assert entry.details["threshold"] == MIN_CONFIDENCE
        assert entry.details["threshold_value"] == rule_confidence(3)
        assert entry.details["confidence"] == evaluation.candidate.confidence
        assert entry.details["variant"] == 1
    # Only the candidates that reached the threshold are Decision_Points.
    assert len(report.decisions) == len(rules.evaluations) - len(low)


# ---------------------------------------------------------------------------
# Fallback (9.9, 20.6)
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("states", "thresholds", "reason"),
    [
        ({COOCCURRENCE: False}, THRESHOLDS_OK, "disabled"),
        (None, {MIN_SUPPORT: 2}, "missing-threshold"),
        (None, {MIN_CONFIDENCE: 0.5}, "missing-threshold"),
        (None, None, "missing-threshold"),
    ],
)
def test_the_fallback_reports_no_candidates(
    states: dict[str, bool] | None, thresholds: dict[str, int | float] | None, reason: str
) -> None:
    m, report = miner(states, thresholds)
    view = root(ADDRESSES)
    rules = m.rules(view)
    assert not m.in_effect()
    assert rules.candidates == () and rules.evaluations == () and rules.is_empty and rules.keywords() == {}
    decision = rules.fallback
    assert decision is not None and decision.method == "fallback" and decision.label == NO_RULE
    assert decision.fallback == FallbackInfo(COOCCURRENCE, reason)  # type: ignore[arg-type]
    assert report.decisions == (decision.entry,) and report.rejected == ()
    # The required list of 9.7 is unaffected.
    assert required_fields(view).keywords() == {"required": ["id"]}


def test_in_effect_needs_both_thresholds() -> None:
    assert miner()[0].in_effect()
    assert not miner(thresholds={MIN_SUPPORT: 2})[0].in_effect()


# ---------------------------------------------------------------------------
# Reduction (9.5)
# ---------------------------------------------------------------------------


def test_a_strongly_connected_component_becomes_a_canonical_cycle() -> None:
    selected = [(a, b) for a, b in itertools.permutations(["zip", "city", "street"], 2)]
    emitted = reduce_rules(selected)
    assert emitted == (("city", "street"), ("street", "zip"), ("zip", "city"))
    check_reduction(selected, [], emitted)


def test_transitively_implied_rules_are_dropped() -> None:
    selected = [("c", "b"), ("b", "a"), ("c", "a"), ("d", "a"), ("d", "b"), ("d", "c")]
    emitted = reduce_rules(selected)
    assert emitted == (("b", "a"), ("c", "b"), ("d", "c"))
    check_reduction(selected, [], emitted)


def test_edges_between_components_use_the_first_rule() -> None:
    # {x, y} is a cycle; both imply "a" and "b" implies "x".
    selected = [("x", "y"), ("y", "x"), ("y", "a"), ("x", "a"), ("b", "y"), ("b", "x"), ("b", "a")]
    emitted = reduce_rules(selected)
    assert emitted == (("b", "x"), ("x", "a"), ("x", "y"), ("y", "x"))
    check_reduction(selected, [], emitted)


def test_given_field_group_rules_are_not_emitted_again() -> None:
    given = list(group_rules([["street", "city"]]))
    assert given == [("city", "street"), ("street", "city")]
    selected = [*itertools.permutations(["city", "street", "zip"], 2), ("note", "city"), ("note", "zip")]
    emitted = reduce_rules(selected, given)
    # Units {city, street} and {zip}: a cycle over their representatives; "note" implies the component once.
    assert emitted == (("city", "zip"), ("note", "city"), ("zip", "city"))
    check_reduction(selected, given, emitted)
    # A rule inside one Field_Group only is never emitted.
    assert reduce_rules([("city", "street")], given) == ()


def test_a_cycle_edge_that_was_not_selected_falls_back_to_an_irredundant_subset() -> None:
    given = list(group_rules([["a", "b"]]))
    # The canonical cycle over units {a, b} and {c} needs a → c, which is not selected.
    selected = [("b", "c"), ("c", "a"), ("c", "b")]
    emitted = reduce_rules(selected, given)
    assert emitted == (("b", "c"), ("c", "a"))
    check_reduction(selected, given, emitted)


def test_reduction_holds_on_random_rule_sets() -> None:
    gen = random.Random(95)
    names = ["a", "b", "c", "d", "e", "f"]
    for _ in range(300):
        selected = [(a, b) for a, b in itertools.permutations(names, 2) if gen.random() < 0.25]
        groups = [] if gen.random() < 0.5 else [gen.sample(names, 2)]
        given = list(group_rules(groups))
        emitted = reduce_rules(selected, given)
        check_reduction(selected, given, emitted)
        assert emitted == tuple(sorted(emitted))
        assert reduce_rules(list(reversed(selected)), list(reversed(given))) == emitted  # order-free


def test_reduction_input_checks() -> None:
    assert reduce_rules([]) == () and reduce_rules([("a", "a")]) == ()
    with pytest.raises(TypeError, match="pairs"):
        reduce_rules([("a", "b", "c")])  # type: ignore[list-item]
    with pytest.raises(ValueError, match="at least two"):
        group_rules([["a"]])
    with pytest.raises(TypeError, match="collection"):
        group_rules(["ab"])


# ---------------------------------------------------------------------------
# Emission and records
# ---------------------------------------------------------------------------


def test_emitted_entries_hold_and_are_canonical() -> None:
    m, _ = miner()
    view = root(ADDRESSES)
    rules = m.rules(view)
    assert rules.selected == tuple(c.rule for c in rules.candidates)
    assert rules.emitted == (("city", "street"), ("street", "zip"), ("zip", "city"))
    assert rules.implied == (("city", "zip"), ("street", "city"), ("zip", "street"))
    keywords = rules.keywords()
    assert keywords == {DEPENDENT_REQUIRED: {"city": ["street"], "street": ["zip"], "zip": ["city"]}}
    keywords[DEPENDENT_REQUIRED]["city"].append("x")  # fresh lists
    assert rules.entries()["city"] == ["street"]
    node = naive_merge_node(view)
    node.kw.update(rules.keywords())
    compiled = VALIDATOR.compile(node.to_json())  # type: ignore[arg-type]
    assert all(VALIDATOR.is_valid(compiled, doc) for doc in ADDRESSES)
    assert not VALIDATOR.is_valid(compiled, {"id": 1, "city": "c"})


def test_entry_records_and_pointers() -> None:
    m, report = miner()
    view = root(CHAIN)
    rules = m.rules(view, variant=3)
    assert rules.selected == (("b", "a"), ("c", "a"), ("c", "b"))
    assert rules.emitted == (("b", "a"), ("c", "b")) and rules.implied == (("c", "a"),)  # 9.5
    assert rules.provenance["c"].details["implied"] == ["a"]
    node = naive_merge_node(view)
    node.kw.update(rules.keywords())
    records = rules.constraint_provenance(node)
    _, pointers = SchemaEmitter().layout(SchemaModel(root=node))
    assert sorted(pointers[cid] for cid in records) == sorted(f"/dependentRequired/{a}" for a in rules.entries())
    n = presence(view).n_objects
    evaluations = {e.candidate.rule: e for e in rules.evaluations}
    for antecedent, consequents in rules.entries().items():
        record = records[ConstraintId(node.nid, DEPENDENT_REQUIRED, antecedent)]
        assert record.technique == COOCCURRENCE and record.fallback is None
        assert (record.support.consistent, record.support.total) == (n, n)
        weakest = min((evaluations[(antecedent, b)] for b in consequents), key=lambda e: e.confidence)
        assert record.confidence == weakest.confidence
        assert record.resolution == weakest.decision.resolution  # type: ignore[union-attr]
        assert record.details["antecedent"] == antecedent and record.details["variant"] == 3
        assert [rule["consequent"] for rule in record.details["rules"]] == consequents
        assert record.details["support_count"] == presence(view).count(antecedent)
        assert record.details["thresholds"] == {MIN_SUPPORT: 2}
    # The node must hold every emitted entry.
    with pytest.raises(ValueError, match="must list"):
        rules.constraint_provenance(naive_merge_node(view))
    assert rules.selected  # the chain yields rules
    assert render_path(rules.path) == "$"


def test_field_groups_are_given_and_scored_in_the_default_record() -> None:
    m, _ = miner()
    rules = m.rules(root(ADDRESSES), groups=[["city", "street", "zip"]])
    assert rules.given == group_rules([["city", "street", "zip"]])
    assert rules.selected == ()  # the clique already codes the presence once, so no rule saves bits
    assert rules.emitted == () and rules.keywords() == {}
    for evaluation in rules.evaluations:
        none, rule = evaluation.decision.resolution.candidates  # type: ignore[union-attr]
        assert none.score <= rule.score


# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------


def test_outcomes_do_not_depend_on_document_order() -> None:
    reference, reference_report = miner()
    expected = reference.rules(root(CHAIN))
    shuffled = list(CHAIN)
    random.Random(4).shuffle(shuffled)
    for docs in (CHAIN[::-1], shuffled):
        m, report = miner()
        rules = m.rules(root(docs))
        assert rules.emitted == expected.emitted and rules.selected == expected.selected
        assert [c.rule for c in rules.candidates] == [c.rule for c in expected.candidates]
        assert report.to_json() == reference_report.to_json()

