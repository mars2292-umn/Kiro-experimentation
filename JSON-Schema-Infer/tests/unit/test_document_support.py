"""Unit tests for decide/support.py and the Document_Support rule at its four sites (task 51.1).

Requirements 4.12, 14.9, 15.9, 16.9: the per-Path document statistics, the
engine's handling of withheld candidates (no step sees them, the
Decision_Model is skipped, the decision records them), and each family's
closed form: Closed_Enum, closed Record, bound rungs, and induced Pattern
rungs across document folds. The Baseline_Rule ``required`` is unaffected.
"""

from __future__ import annotations

from typing import Any

import numpy as np
import pytest

from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate, DecisionEngine
from jsinfer.decide.support import (
    RULE,
    QuantitySupport,
    document_folds,
    documents_of,
    key_documents,
    single_document_keys,
    value_documents,
    withheld,
)
from jsinfer.io.validator import Validator
from jsinfer.structure.record_map import CLOSED, OPEN, closed_or_open
from jsinfer.values.bounds import BOUNDS, NUMBER, STRING_LENGTH, decide_bound
from jsinfer.values.enums import ENUM_ESTIMATION, judge_enum
from jsinfer.values.inferrer import PATTERN, ValueInferrer
from tests.unit.test_analyzer import infer

SYNTHETIC_ENUM = "dm.closed_or_open_enum.synthetic"


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


class StubBundle:
    def __init__(self, models: dict[str, Any] | None = None) -> None:
        self.models = dict(models or {})

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return None


class ClosingModel:
    """A Closed-or-Open-Enum Decision_Model that always prefers closed."""

    classes = ("closed", "open")

    def __init__(self) -> None:
        self.calls = 0

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        self.calls += 1
        return np.array([[0.9, 0.1]])


# ---------------------------------------------------------------------------
# Statistics
# ---------------------------------------------------------------------------


def test_documents_of_counts_distinct_documents_with_a_row() -> None:
    view = root([{"a": 1}, {"a": 2}, {"b": 3}])
    assert documents_of(view) == 3
    assert documents_of(view.child_key("a")) == 2
    assert documents_of(view.child_key("b")) == 1
    assert documents_of(view.child_key("zzz")) == 0


def test_value_documents_is_aligned_with_the_distinct_values() -> None:
    elements = root([["a", "b", "b"], ["a"], ["c"]]).elements()
    stats = elements.stats()
    table = elements.store.values
    by_value = dict(zip((table[int(v)] for v in stats.value_ids.tolist()), value_documents(elements).tolist()))
    assert by_value == {"a": 2, "b": 1, "c": 1}  # "b" twice in one document is still one document


def test_key_documents_and_single_document_keys() -> None:
    objects = root([{"a": 1, "b": 2}, {"a": 3}, {"a": 4, "c": 5}])
    names = [objects.store.key_name(int(k)) for k in objects.stats().key_ids.tolist()]
    assert dict(zip(names, key_documents(objects).tolist())) == {"a": 3, "b": 1, "c": 1}
    assert single_document_keys(objects) == ["b", "c"]
    assert single_document_keys(objects, ["a", "b"]) == ["b"]
    assert single_document_keys(objects, ["a"]) == []


def test_quantity_support_sides() -> None:
    support = QuantitySupport.of_array(np.array([3, 3, 2]), np.array([0, 1, 2]))
    assert support.documents == 3
    assert (support.minimum.attained_by, support.minimum.others) == (1, 3)  # only document 2 attains 2
    assert support.maximum.attained_by == 2 and support.maximum.unchanged
    single = QuantitySupport.of_array(np.array([2, 2]), np.array([0, 0]))
    assert single.documents == 1 and (single.minimum.attained_by, single.minimum.others) == (1, None)
    exact = QuantitySupport.of([1, 1.5, 10**20], np.array([1, 2, 0, 0]), np.array([0, 1, 1, 2]))
    assert (exact.maximum.attained_by, exact.maximum.others) == (1, 1.5)  # exact: 10**20 is not a float
    assert (exact.minimum.attained_by, exact.minimum.others) == (2, None)
    with pytest.raises(ValueError):
        QuantitySupport.of([1], np.array([1]), np.array([0]))
    with pytest.raises(TypeError):
        QuantitySupport.of_array(np.array([1.5]), np.array([0]))


def test_document_folds_deal_round_robin_and_leave_one_out_below_three() -> None:
    assert document_folds(1).tolist() == [0]
    assert document_folds(2).tolist() == [0, 1]
    assert document_folds(5).tolist() == [0, 1, 2, 0, 1]
    assert document_folds(0).tolist() == []
    with pytest.raises(ValueError):
        document_folds(-1)


def test_pattern_folds_do_not_depend_on_document_order() -> None:
    docs = ["x_1", "x_22", "x_333", "x_4444", "x_55555", "x_666666", "y_7", "y_88"]
    first, _ = scalar(docs)
    second, _ = scalar(list(reversed(docs)))
    assert first.node.to_json() == second.node.to_json()
    assert [(c.label, c.refuted) for c in first.string.candidates] == [(c.label, c.refuted) for c in second.string.candidates]


def test_withheld_names_the_rule_and_the_document_count() -> None:
    assert withheld(1) == {"rule": RULE, "documents": 1}
    assert withheld(4, fold=2) == {"rule": RULE, "documents": 4, "fold": 2}


# ---------------------------------------------------------------------------
# The engine
# ---------------------------------------------------------------------------


def test_a_withheld_candidate_needs_a_rule() -> None:
    with pytest.raises(TypeError):
        Candidate(technique=None, outcome="closed", withheld={"documents": 1})  # type: ignore[arg-type]


def test_a_withheld_candidate_skips_the_decision_model_and_is_recorded() -> None:
    model = ClosingModel()
    bundle = StubBundle({SYNTHETIC_ENUM: model})
    docs = ["red", "blue"] * 20 + ["green"]  # "green" is held by one document
    report = InferenceReport()
    engine = DecisionEngine({ENUM_ESTIMATION: True}, bundle=bundle, report=report)
    judgment = judge_enum(root(docs), engine, report=report)
    assert not judgment.closed and model.calls == 0
    decision = judgment.decision
    assert decision is not None and decision.method == "mdl"
    assert [c.outcome for c in decision.resolution.candidates] == ["open"]
    assert decision.details["withheld"] == [
        {"label": "closed", "rule": RULE, "documents": 41, "single_document_values": 1}
    ]
    (entry,) = report.decisions
    assert entry.details["withheld"] == decision.details["withheld"]


def test_without_a_withheld_candidate_the_decision_model_decides() -> None:
    model = ClosingModel()
    engine = DecisionEngine({ENUM_ESTIMATION: True}, bundle=StubBundle({SYNTHETIC_ENUM: model}))
    judgment = judge_enum(root(["red", "blue"] * 20), engine)
    assert judgment.closed and model.calls == 1
    assert judgment.decision is not None and judgment.decision.method == "decision-model"
    assert "withheld" not in judgment.decision.details


# ---------------------------------------------------------------------------
# Closed_Enum (15.9)
# ---------------------------------------------------------------------------


def test_an_enum_from_one_document_is_withheld_however_often_its_values_repeat() -> None:
    engine = DecisionEngine({ENUM_ESTIMATION: True})
    judgment = judge_enum(root([["a", "a", "b", "b", "a", "b"]]).elements(), engine)
    assert not judgment.closed
    assert judgment.decision is not None
    assert judgment.decision.details["withheld"] == [
        {"label": "closed", "rule": RULE, "documents": 1, "single_document_values": 2}
    ]


def test_values_held_by_two_documents_each_are_not_withheld() -> None:
    engine = DecisionEngine({ENUM_ESTIMATION: True})
    judgment = judge_enum(root([["a", "b"], ["b", "a"], ["a", "b"]]).elements(), engine)
    assert judgment.decision is not None and "withheld" not in judgment.decision.details
    assert [c.outcome for c in judgment.decision.resolution.candidates] == ["closed", "open"]


# ---------------------------------------------------------------------------
# Closed Record (4.12)
# ---------------------------------------------------------------------------


def test_a_key_held_in_one_document_withholds_the_closed_record() -> None:
    engine = DecisionEngine()
    shared: list[Any] = [{"a": 1, "b": 2}, {"a": 3, "b": 4}, {"a": 5, "b": 6}]
    steady = closed_or_open(root(shared), engine)
    assert "withheld" not in steady.decision.details
    assert [c.outcome for c in steady.decision.resolution.candidates] == [CLOSED, OPEN]
    rare = closed_or_open(root([*shared, {"a": 7, "b": 8, "c": 9}]), engine)
    assert not rare.closed
    assert rare.decision.details["withheld"] == [{"label": CLOSED, "rule": RULE, "documents": 4, "single_document_keys": 1}]
    alone = closed_or_open(root([{"a": 1, "b": 2}]), engine)
    assert not alone.closed and alone.decision.details["withheld"][0]["documents"] == 1


def test_only_listed_keys_count_for_the_partitioned_record() -> None:
    from jsinfer.core.schema_model import SchemaNode

    engine = DecisionEngine()
    docs: list[Any] = [{"id": 1, "x1": 0}, {"id": 2, "x2": 0}, {"id": 3, "x3": 0}]
    record = SchemaNode.from_json({"type": "object", "properties": {"id": {"type": "integer"}}})
    outcome = closed_or_open(root(docs), engine, record=record)
    assert "withheld" not in outcome.decision.details  # x1…x3 are Map entries, not listed fields


# ---------------------------------------------------------------------------
# Bounds (16.9)
# ---------------------------------------------------------------------------


def test_an_extreme_attained_by_one_document_withholds_the_rungs_it_moves() -> None:
    engine = DecisionEngine({BOUNDS: True})
    support = QuantitySupport.of_array(np.array([5, 5, 5, 2]), np.array([0, 1, 2, 3]))
    decision = decide_bound([5, 5, 5, 2], STRING_LENGTH, engine, support=support)
    assert decision.minimum is not None and decision.maximum is not None
    assert decision.minimum.decision.details["withheld"] == [
        {"label": "minLength=2", "rule": RULE, "documents": 4, "leave_one_out_value": 5}
    ]
    assert decision.minimum.value in (None, 0)  # observed 2 is gone; omission or zero remain
    assert [c.outcome for c in decision.minimum.decision.resolution.candidates] == ["minLength=omit", "minLength=0"]
    assert "withheld" not in decision.maximum.decision.details  # three documents attain 5


def test_quantities_from_one_document_keep_only_omission() -> None:
    engine = DecisionEngine({BOUNDS: True})
    support = QuantitySupport.of([1, 2, 3], np.array([0, 1, 2]), np.array([0, 0, 0]))
    decision = decide_bound([1, 2, 3], NUMBER, engine, support=support)
    assert decision.keywords() == {}
    for side in decision.sides:
        listed = side.decision.details["withheld"]
        assert listed and all(item["documents"] == 1 and "leave_one_out_value" not in item for item in listed)
        assert [c.outcome for c in side.decision.resolution.candidates] == [f"{side.keyword}=omit"]


def test_the_leave_one_out_value_is_recomputed_per_rung() -> None:
    engine = DecisionEngine({BOUNDS: True})
    support = QuantitySupport.of([1, 5], np.array([0, 0, 1]), np.array([0, 1, 2]))
    decision = decide_bound([1, 1, 5], NUMBER, engine, support=support)
    assert decision.minimum is not None and "withheld" not in decision.minimum.decision.details  # two documents attain 1
    assert decision.maximum is not None
    labels = {item["label"]: item["leave_one_out_value"] for item in decision.maximum.decision.details["withheld"]}
    assert labels == {"maximum=5": 1}  # recomputed from the other documents' extreme, 1; 0 never made the ladder
    assert decision.maximum.value is None or decision.maximum.value >= 127  # the integer limits survive


def test_without_support_nothing_is_withheld() -> None:
    engine = DecisionEngine({BOUNDS: True})
    decision = decide_bound([5, 5, 5, 2], STRING_LENGTH, engine)
    assert all("withheld" not in side.decision.details for side in decision.sides)
    assert [c.outcome for c in decision.minimum.decision.resolution.candidates] == ["minLength=omit", "minLength=2", "minLength=0"]  # type: ignore[union-attr]
    with pytest.raises(TypeError):
        decide_bound([1], NUMBER, engine, support="docs")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Patterns (14.9)
# ---------------------------------------------------------------------------


def scalar(docs: list[Any]) -> tuple[Any, InferenceReport]:
    report = InferenceReport()
    inferrer = ValueInferrer.for_techniques(None, report=report)
    return inferrer.infer_scalar(root(docs)), report


def test_patterns_induced_from_one_document_are_withheld_and_reported() -> None:
    node, report = scalar(["ORD_0001"])
    assert "pattern" not in node.node.kw
    induced = [c for c in node.string.candidates if c.technique == PATTERN]
    assert induced and all(c.refuted == {"rule": RULE, "documents": 1} and c.withheld for c in induced)
    entries = [e for e in report.rejected if e.category == "patterns"]
    assert len(entries) == len(induced)
    assert all(e.details["withheld"] == {"rule": RULE, "documents": 1} and e.details["rejected"] == 0 for e in entries)


def test_a_rung_that_fails_a_fold_is_withheld_while_a_generalizing_one_survives() -> None:
    node, report = scalar(["x_1", "x_22", "x_333", "x_4444", "x_55555", "x_666666"])
    selected = node.string.selected
    assert selected is not None and selected.technique == PATTERN
    validator = Validator()
    assert all(validator.matches_pattern(selected.value, s) for s in node.string.strings.strings)
    refuted = [c for c in node.string.candidates if c.refuted is not None]
    assert refuted and all(c.refuted["documents"] == 6 and c.refuted["unmatched"] >= 1 for c in refuted)
    assert all("fold" in c.refuted and "held_out" in c.refuted for c in refuted)
    assert {str(c.details["rung"]) for c in refuted} < {str(c.details["rung"]) for c in node.string.candidates}


def test_a_shape_shared_by_two_documents_survives_leave_one_document_out() -> None:
    node, _ = scalar(["ORD_0001", "INV_0002"])
    assert node.string.selected is not None and node.string.selected.technique == PATTERN
    assert all(c.refuted is None for c in node.string.candidates)


# ---------------------------------------------------------------------------
# End to end
# ---------------------------------------------------------------------------


def test_two_documents_yield_an_open_schema_that_accepts_a_third() -> None:
    docs: list[Any] = [
        {"name": "alice", "tags": ["x", "y"], "n": 3, "kind": "user"},
        {"name": "bob", "tags": ["z"], "n": 7, "kind": "admin"},
    ]
    model, _ = infer(docs)
    schema = model.root.to_json()
    assert isinstance(schema, dict)
    flat = str(schema)
    for keyword in ("enum", "const", "pattern", "minLength", "maxLength", "minimum", "maximum", "minItems", "maxItems"):
        assert f"'{keyword}'" not in flat, keyword
    assert "additionalProperties" not in schema
    assert schema["required"] == ["kind", "n", "name", "tags"]  # the Baseline_Rule stands (28.5)
    compiled = Validator().compile(schema)
    assert Validator().is_valid(compiled, {"name": "carol", "tags": [], "n": -1, "kind": "guest", "extra": True})
    assert not Validator().is_valid(compiled, {"name": 1, "tags": [], "n": 0, "kind": "user"})
    # A value both documents hold survives leave-one-document-out and may still close: that is the rule's boundary.
    same_kind, _ = infer([{**docs[0]}, {**docs[1], "kind": "user"}])
    kind = same_kind.root.to_json()["properties"]["kind"]  # type: ignore[index, call-overload]
    assert kind.get("const") == "user"
