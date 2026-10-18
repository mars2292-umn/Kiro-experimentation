"""Unit tests for structure/arrays.py: Tuple-or-List and array nodes (task 13.4).

Requirements 7.1 (resolution with provenance and the rejected alternative),
7.2 (List ``items``), 7.3 (Tuple ``prefixItems``), 7.4 (``items: false``),
7.9 (only empty arrays), 7.10 (MDL with ties to List, fallback List), and
16.4 (item-count bounds).
"""

from __future__ import annotations

import math
import random
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import build_store
from jsinfer.core.paths import ELEM_STEP, ROOT
from jsinfer.core.provenance import FallbackInfo, laplace_confidence
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate, DecisionEngine, mdl_posterior
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.arrays import (
    ARRAY_GRAMMAR,
    ARRAY_PATTERN,
    DM_TUPLE_LIST,
    LIST,
    TUPLE,
    TUPLE_OR_LIST,
    ArrayAnalyzer,
    LengthStats,
    array_rows,
    list_encoding,
    tuple_encoding,
    tuple_list_features,
    tuple_or_list,
    type_entropies,
)
from jsinfer.values.inferrer import ValueInferrer

VALIDATOR = Validator()
MDL = "decide.mdl"
BOUNDS = "values.bounds"

# Fixed-length rows whose positions each hold one type: a Tuple by MDL.
ROWS = [[f"n{i}", i, i % 2 == 0] for i in range(30)]
# Variable-length integer lists: a List by MDL.
LISTS = [list(range(i % 10)) for i in range(30)]


class StubBundle:
    """Decision_Models by artifact name, like ResolvedBundle.model; no Learned_Thresholds."""

    def __init__(self, models: dict[str, Any]) -> None:
        self.models = models

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return None


class StubModel:
    """A Decision_Model with fixed calibrated probabilities; records the feature arrays it sees."""

    def __init__(self, classes: tuple[str, ...], proba: list[float]) -> None:
        self.classes = classes
        self.proba = np.array(proba, dtype=np.float64)
        self.seen: list[np.ndarray] = []

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        self.seen.append(np.array(x))
        return self.proba.reshape(1, -1)


def score(model_bits: float, data_bits: float) -> Score:
    return Score(float(model_bits), float(data_bits), 0.0, 0, 1)


class FixedScorer(MDLScorer):
    """Scores each candidate by its shape (``tuple`` with ``prefixItems``, else ``list``); forces ties and wins."""

    __slots__ = ("scores",)

    def __init__(self, **scores: Score) -> None:
        super().__init__(validator=VALIDATOR)
        self.scores = scores

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        return self.scores[TUPLE if "prefixItems" in node.kw else LIST]


# Scores under which the Tuple wins.
TUPLING = FixedScorer(tuple=score(1, 1), list=score(50, 50))


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def engine(states: dict[str, bool] | None = None, **kwargs: Any) -> tuple[DecisionEngine, InferenceReport]:
    report = InferenceReport()
    return DecisionEngine(states, report=report, **kwargs), report


def analyzer(
    states: dict[str, bool] | None = None, *, scorer: MDLScorer | None = None
) -> tuple[ArrayAnalyzer, InferenceReport]:
    report = InferenceReport()
    if scorer is None:
        return ArrayAnalyzer.for_techniques(states, report=report, validator=VALIDATOR), report
    values = ValueInferrer.for_techniques(states, report=report, validator=VALIDATOR)
    return ArrayAnalyzer(values, scorer=scorer), report


def decisions(report: InferenceReport, kind: str) -> list[Any]:
    return [entry for entry in report.decisions if entry.kind == kind]


def validates(schema: dict[str, Any], docs: list[Any]) -> bool:
    """Whether every doc validates against ``schema`` (``format`` asserted)."""
    compiled = VALIDATOR.compile(schema)
    return all(VALIDATOR.is_valid(compiled, doc) for doc in docs)


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_mdl_serves_tuple_or_list_with_ties_and_fallback_to_list() -> None:
    assert TUPLE_OR_LIST in REGISTRY.technique(MDL).serves
    assert REGISTRY.active_servers(TUPLE_OR_LIST, REGISTRY.default_states()) == (MDL,)  # no Decision_Model yet
    assert REGISTRY.fallback(TUPLE_OR_LIST)() == LIST
    assert REGISTRY.decision_point(TUPLE_OR_LIST).tie_rule.favors == (LIST,)  # 7.10
    assert not REGISTRY.technique(ARRAY_GRAMMAR).implemented


# ---------------------------------------------------------------------------
# Length statistics and features
# ---------------------------------------------------------------------------


def test_length_statistics() -> None:
    lengths = LengthStats.of(root([[1, 2], [3], [4, 5], [], "s"]))
    assert (lengths.n_arrays, lengths.n_elements, lengths.lengths, lengths.counts) == (4, 5, (0, 1, 2), (1, 1, 2))
    assert (lengths.min_length, lengths.max_length, lengths.mean) == (0, 2, 1.25)
    assert math.isclose(lengths.std, math.sqrt(((0 - 1.25) ** 2 + (1 - 1.25) ** 2 + 2 * (2 - 1.25) ** 2) / 4))
    assert (lengths.distinct_ratio, lengths.modal_share) == (0.75, 0.5)
    assert lengths.elements_observed and not LengthStats.of(root([[], []])).elements_observed


def test_type_entropies_separate_positions() -> None:
    pooled, positional = type_entropies(root(ROWS))
    assert math.isclose(pooled, math.log2(3)) and positional == 0.0  # each position holds one type
    pooled, positional = type_entropies(root([[1, "a"], ["b", 2]]))
    assert pooled == 1.0 and positional == 1.0
    assert type_entropies(root([[]])) == (0.0, 0.0)


def test_default_features() -> None:
    view = root(ROWS)
    scorer = MDLScorer(validator=VALIDATOR)
    candidates = [
        Candidate(technique=None, outcome=label, order_key=(i,), node=node, score=scorer.score(node, view))
        for i, (label, node) in enumerate(((TUPLE, tuple_encoding(view)), (LIST, list_encoding(view))))
    ]
    x = tuple_list_features(view, candidates)
    assert x.shape == (9,) and np.all(np.isfinite(x))
    # arrays, mean, std, distinct-length ratio, modal share, positional − pooled entropy, element types, max length
    assert np.allclose(x[:8], [30, 3.0, 0.0, 1 / 30, 1.0, -math.log2(3), 3, 3], rtol=1e-12, atol=1e-12)
    first, second = (c.score for c in candidates)
    assert x[8] == (first.total - second.total) / first.n_scored


# ---------------------------------------------------------------------------
# Tuple-or-List (7.1, 7.10)
# ---------------------------------------------------------------------------


def test_positional_types_are_a_tuple_by_mdl() -> None:
    eng, report = engine()
    outcome = tuple_or_list(root(ROWS), eng)
    assert outcome.is_tuple and outcome.constraint_keyword == "prefixItems"
    decision = outcome.decision
    assert decision.method == "mdl" and decision.technique == MDL
    record = outcome.provenance()
    # 7.1: the resolving Technique, the Confidence, and the rejected alternative with its score.
    assert record.technique == MDL and record.cutoff.source == "MDL comparison"
    candidates = record.resolution.candidates
    assert [c.outcome for c in candidates] == [TUPLE, LIST] and record.resolution.selected == 0
    assert all(c.technique is None and c.score is not None for c in candidates)
    assert candidates[0].score < candidates[1].score
    assert record.confidence == mdl_posterior([c.score for c in candidates])[0]
    assert (record.support.consistent, record.support.total) == (30, 30)
    assert record.details["judgment"] == TUPLE and record.details["length_statistics"]["max_length"] == 3
    assert [entry.resolution.method for entry in decisions(report, TUPLE_OR_LIST)] == ["mdl"]


def test_homogeneous_variable_length_arrays_are_a_list_by_mdl() -> None:
    eng, _ = engine()
    outcome = tuple_or_list(root(LISTS), eng)
    assert outcome.is_list and outcome.constraint_keyword == "items"
    resolution = outcome.provenance().resolution
    assert resolution.candidates[resolution.selected].outcome == LIST
    assert resolution.candidates[1].score < resolution.candidates[0].score


def test_candidates_are_scored_on_the_same_array_rows() -> None:
    docs: list[Any] = [*ROWS[:10], [], ["x"], 5, {"k": [1]}]
    view = root(docs)
    arrays = array_rows(view)
    tuple_node, list_node = tuple_encoding(view), list_encoding(view)
    assert list_node.to_json() == {"type": ["array"], "items": naive_merge_node(arrays.elements()).to_json()}
    assert list_node.to_json() == naive_merge_node(arrays).to_json()
    assert tuple_node.to_json() == {
        "type": ["array"],
        "prefixItems": [naive_merge_node(arrays.elements(i)).to_json() for i in range(3)],
        "items": False,
    }
    eng, _ = engine()
    candidates = tuple_or_list(view, eng).provenance().resolution.candidates
    scorer = MDLScorer(validator=VALIDATOR)
    assert [c.score for c in candidates] == [scorer.score(tuple_node, arrays).total, scorer.score(list_node, arrays).total]
    assert {c.n_scored for c in candidates} == {scorer.score(list_node, arrays).n_scored}


def test_ties_go_to_list_even_with_fewer_tuple_model_bits() -> None:
    # Equal description lengths; the Tuple has fewer model bits, but 7.10 needs a strictly lower one.
    scorer = FixedScorer(tuple=score(1, 5), list=score(5, 1))
    eng, _ = engine()
    outcome = tuple_or_list(root(ROWS), eng, scorer=scorer)
    assert outcome.is_list and outcome.decision.method == "mdl"
    assert outcome.decision.confidence == 0.5


def test_mdl_disabled_falls_back_to_list() -> None:
    eng, report = engine({MDL: False})
    outcome = tuple_or_list(root(ROWS), eng)  # MDL would say Tuple
    assert outcome.is_list
    record = outcome.provenance()
    assert record.technique == "fallback:tuple_or_list"
    assert record.fallback == FallbackInfo(DM_TUPLE_LIST, "disabled")
    assert record.resolution.method == "fallback" and record.cutoff is None
    assert record.confidence == laplace_confidence(record.support)
    assert [entry.resolution.method for entry in decisions(report, TUPLE_OR_LIST)] == ["fallback"]


def test_a_decision_model_in_effect_decides_on_the_default_features() -> None:
    model = StubModel((TUPLE, LIST), [0.9, 0.1])
    bundle = StubBundle({"dm.tuple_or_list.synthetic": model})
    eng, _ = engine({DM_TUPLE_LIST: True}, bundle=bundle)
    outcome = tuple_or_list(root(LISTS), eng)  # MDL would say List
    assert outcome.is_tuple and outcome.decision.method == "decision-model"
    record = outcome.provenance()
    assert record.technique == DM_TUPLE_LIST and record.confidence == 0.9
    (x,) = model.seen
    assert x.shape == (1, 9) and np.all(np.isfinite(x))
    assert x[0, 0] == 30 and x[0, 7] == 9  # arrays and the maximum length


def test_a_missing_decision_model_falls_back_to_mdl() -> None:
    eng, report = engine({DM_TUPLE_LIST: True}, bundle=StubBundle({}))
    outcome = tuple_or_list(root(ROWS), eng)
    assert outcome.is_tuple and outcome.decision.method == "mdl"
    assert outcome.decision.fallback == FallbackInfo(DM_TUPLE_LIST, "missing-model")
    assert [s.artifact for s in report.substitutions] == ["dm.tuple_or_list.synthetic"]


def test_only_array_rows_count() -> None:
    docs: list[Any] = [*ROWS[:5], 1, "s", {"a": [1]}, None]
    eng, _ = engine()
    outcome = tuple_or_list(root(docs), eng)
    assert outcome.lengths.n_arrays == 5 and outcome.provenance().support.total == 5
    assert len(array_rows(root(docs))) == 5


def test_flagged_documents_are_excluded() -> None:
    store = build_store([*ROWS[:4], [1, 2, 3, 4, 5, 6]])
    store.flags.flag_documents([4])
    eng, _ = engine()
    outcome = tuple_or_list(View.root(store), eng)
    assert outcome.lengths.n_arrays == 4 and outcome.lengths.max_length == 3


def test_a_view_without_arrays_is_an_error() -> None:
    eng, _ = engine()
    with pytest.raises(ValueError, match="no non-flagged array"):
        tuple_or_list(root([1, {"a": 2}]), eng)


def test_only_empty_arrays_resolve_no_judgment_call() -> None:
    eng, report = engine()
    outcome = tuple_or_list(root([[], [], 3]), eng)
    assert outcome.kind is None and outcome.decision is None and not outcome.elements_observed
    assert outcome.constraint_keyword == "type"  # 7.9: the note goes on type
    assert outcome.details() == {"elements_observed": False}
    assert decisions(report, TUPLE_OR_LIST) == []
    with pytest.raises(ValueError, match="7.9"):
        outcome.provenance()


def test_decisions_do_not_depend_on_document_order() -> None:
    docs: list[Any] = [*ROWS[:6], *LISTS[:6], [], ["x", 1.5]]
    seen = set()
    rng = random.Random(0)
    for _ in range(4):
        order = docs[:]
        rng.shuffle(order)
        eng, report = engine()
        tuple_or_list(root(order), eng)
        seen.add(canonical_bytes([entry.to_json() for entry in report.decisions]))
    assert len(seen) == 1


# ---------------------------------------------------------------------------
# Array nodes (7.2, 7.3, 7.4, 7.9, 16.4)
# ---------------------------------------------------------------------------


def test_list_node_has_one_items_schema_over_every_element() -> None:
    aa, report = analyzer()
    seen: list[View] = []

    def child(view: View) -> SchemaNode:
        seen.append(view)
        return naive_merge_node(view)

    node = aa.array_node(root(LISTS), child_schema=child)
    assert node.outcome.is_list
    kw = node.keywords()
    assert "prefixItems" not in kw and kw["items"] == {"type": ["integer"]}  # 7.2
    assert len(seen) == 1 and seen[0].path == (ELEM_STEP,) and len(seen[0]) == sum(map(len, LISTS))
    assert node.provenance["items"] is not None and node.provenance["items"].details["judgment"] == LIST
    assert node.constraint_provenance()[ConstraintId(node.node.nid, "items")] == node.provenance["items"]
    assert node.type_details() == {}
    assert validates({"type": "array", **kw}, LISTS)
    # The design walk's order: Tuple-or-List, the array grammar's fallback (28.4), then both item-count bounds.
    kinds = [entry.kind for entry in report.decisions]
    assert kinds.count(TUPLE_OR_LIST) == 1 and kinds.count(ARRAY_PATTERN) == 1 and kinds.count("bound") == 2
    assert node.pattern is not None and node.pattern.method == "fallback"
    assert node.pattern.label == "base-array-schema" and node.pattern.fallback == FallbackInfo(ARRAY_GRAMMAR, "disabled")


def test_tuple_node_has_one_schema_per_position_and_items_false() -> None:
    docs: list[Any] = [*ROWS, ["short", 1]]
    aa, _ = analyzer()
    seen: list[View] = []

    def child(view: View) -> SchemaNode:
        seen.append(view)
        return naive_merge_node(view)

    node = aa.array_node(root(docs), child_schema=child)
    assert node.outcome.is_tuple
    kw = node.keywords()
    assert kw["prefixItems"] == [{"type": ["string"]}, {"type": ["integer"]}, {"type": ["boolean"]}]  # 7.3
    assert kw["items"] is False  # 7.4: no array is longer than the prefix
    # Each position's view holds the elements at that index; positions are not Path steps.
    assert [len(view) for view in seen] == [31, 31, 30] and {view.path for view in seen} == {(ELEM_STEP,)}
    assert node.provenance["prefixItems"] is node.provenance["items"]
    assert node.provenance["prefixItems"].details["judgment"] == TUPLE
    assert kw["minItems"] == 2 and kw["maxItems"] == 3  # 16.4
    assert set(node.provenance) == {"prefixItems", "items", "minItems", "maxItems"}
    assert validates({"type": "array", **kw}, docs)
    assert not validates({"type": "array", **kw}, [["a", 1, True, None]])  # items: false
    assert not validates({"type": "array", **kw}, [[1, "a", True]])  # positions are typed


def test_a_supplied_outcome_is_used() -> None:
    aa, report = analyzer(scorer=TUPLING)
    outcome = aa.tuple_or_list(root(LISTS))  # the fixed scorer makes it a Tuple
    node = aa.array_node(root(LISTS), outcome)
    assert node.outcome is outcome and len(node.keywords()["prefixItems"]) == 9
    assert len(decisions(report, TUPLE_OR_LIST)) == 1
    with pytest.raises(ValueError, match="not for"):
        aa.array_node(root([{"a": [1]}]).child_key("a"), outcome)


def test_flagged_elements_are_excluded_but_keep_their_position() -> None:
    store = build_store([[1, "x", True], [2, "y", True]])
    column = (ELEM_STEP,)
    last = int(np.flatnonzero(store.columns[column].pos == 2)[0])
    store.flags.flag_items(column, [last])  # the last element of the first array
    aa, _ = analyzer(scorer=TUPLING)
    node = aa.array_node(View.root(store))
    kw = node.keywords()
    assert node.outcome.lengths.max_length == 3 and node.outcome.lengths.n_elements == 5
    assert kw["prefixItems"][2] == {"type": ["boolean"]} and kw["items"] is False


def test_only_empty_arrays_emit_neither_keyword() -> None:
    aa, report = analyzer()
    docs: list[Any] = [[] for _ in range(50)]
    node = aa.array_node(root(docs))
    kw = node.keywords()
    assert "items" not in kw and "prefixItems" not in kw  # 7.9
    assert node.type_details() == {"elements_observed": False}
    assert node.pattern is None and decisions(report, TUPLE_OR_LIST) == [] and decisions(report, ARRAY_PATTERN) == []
    assert len(decisions(report, "bound")) == 2  # 16.4 counts empty arrays too
    assert validates({"type": "array", **kw}, docs)


def test_item_count_bounds_follow_the_bounds_technique() -> None:
    aa, report = analyzer({BOUNDS: False})
    node = aa.array_node(root(ROWS))
    assert set(node.keywords()) == {"prefixItems", "items"}  # 16.8
    assert node.item_bounds.kind == "item_count"
    assert [entry.resolution.method for entry in decisions(report, "bound")] == ["fallback", "fallback"]


def test_all_techniques_off_match_naive_merge() -> None:
    off = {tid: False for tid in REGISTRY.techniques}
    for docs in (ROWS, LISTS, [[1, [2, "a"]], ["b"], []]):
        aa, _ = analyzer(off)
        node = aa.array_node(root(docs))
        assert node.outcome.is_list
        assert {"type": ["array"], **node.keywords()} == naive_merge_node(root(docs)).to_json()


def test_array_nodes_accept_every_non_flagged_document() -> None:
    collections: list[list[Any]] = [
        ROWS,
        LISTS,
        [[], [1], [1, "a", None], [{"k": 1}, [2]]],
        [[1.5, 2], [3, "x", {"y": [True]}], []],
        [[f"id-{i}", i * 0.5] for i in range(40)],
    ]
    for docs in collections:
        for scorer in (None, TUPLING):
            aa, _ = analyzer(scorer=scorer)
            node = aa.array_node(root(docs))
            assert validates({"type": "array", **node.keywords()}, docs), (docs, node.keywords())


def test_array_grammar_hook_raises_once_the_technique_is_marked_implemented(monkeypatch: pytest.MonkeyPatch) -> None:
    aa, report = analyzer({ARRAY_GRAMMAR: True})
    node = aa.array_node(root(LISTS))  # enabled outside a validated run, not implemented: no candidates
    assert node.pattern is None and decisions(report, ARRAY_PATTERN) == []
    original = ValueInferrer.runs
    monkeypatch.setattr(ValueInferrer, "runs", lambda self, tid: tid == ARRAY_GRAMMAR or original(self, tid))
    with pytest.raises(NotImplementedError, match=ARRAY_GRAMMAR):
        aa.array_node(root(LISTS))


def test_path_and_type_checks() -> None:
    aa, _ = analyzer()
    view = root([{"a": [1, 2]}, {"a": [3]}]).child_key("a")
    node = aa.array_node(view)
    assert node.path == view.path and node.path != ROOT
    with pytest.raises(TypeError, match="ArrayOutcome"):
        aa.array_node(view, "list")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="no non-flagged array"):
        aa.array_node(root([1]))
