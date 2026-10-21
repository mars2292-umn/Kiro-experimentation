"""Unit tests for values/enums.py (task 12.3).

Requirements 15.1–15.8: the Good–Turing/Chao1 statistics under ``enum``
equality, the Closed-or-Open-Enum Judgment_Call through the DecisionEngine
(Decision_Model gated on enum estimation, MDL with the Good–Turing charge and
ties to open, the all-singletons override, the open fallback), ``const`` /
``enum`` emission, and the records in provenance and the report.
Decision_Models are stubs.
"""

from __future__ import annotations

import math
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import MDL_COMPARISON, FallbackInfo
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.mdl.codes import CAT, U
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.values.enums import (
    CLOSED_OR_OPEN_ENUM,
    DM_CLOSED_ENUM,
    ENUM_ESTIMATION,
    EnumStats,
    enum_keywords,
    enum_statistics,
    good_turing_charge,
    judge_enum,
    scalar_view,
)

SYNTHETIC = "dm.closed_or_open_enum.synthetic"
COLORS = ["red", "blue"] * 20
# Without the Good–Turing charge the closed candidate is shorter; with it, open wins (15.6).
MOSTLY_SINGLETONS = ["a" * 20, "a" * 20, "b" * 20, "c" * 20, "d" * 20]


class StubBundle:
    def __init__(self, models: dict[str, Any] | None = None) -> None:
        self.models = dict(models or {})

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return None


class StubModel:
    """A Decision_Model with fixed calibrated probabilities; records the features it sees."""

    def __init__(self, proba: list[float], classes: tuple[str, ...] = ("closed", "open")) -> None:
        self.classes = classes
        self.proba = np.asarray(proba, dtype=np.float64)
        self.seen: list[np.ndarray] = []

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        self.seen.append(np.array(x))
        return self.proba.reshape(1, -1)


def view_of(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def engine(states: dict[str, bool], *, bundle: Any = None, report: InferenceReport | None = None) -> DecisionEngine:
    return DecisionEngine(states, bundle=bundle, report=report)


def judge(docs: list[Any], states: dict[str, bool] | None = None, **kwargs: Any) -> tuple[Any, InferenceReport]:
    report = InferenceReport()
    states = {ENUM_ESTIMATION: True} if states is None else states
    bundle = kwargs.pop("bundle", None)
    judgment = judge_enum(view_of(docs), engine(states, bundle=bundle, report=report), report=report, **kwargs)
    return judgment, report


# ---------------------------------------------------------------------------
# Statistics (15.1)
# ---------------------------------------------------------------------------


def test_statistics_follow_the_reference_formulas() -> None:
    # Distinct: 1 (twice, as 1 and 1.0), 2, "a" (twice), "b", true.
    stats = EnumStats.of_values([1, 1.0, 2, "a", "a", "b", True])
    assert (stats.n, stats.s_obs, stats.f1, stats.f2) == (7, 5, 3, 2)
    assert stats.good_turing == 3 / 7
    assert stats.chao1 == 5 + 9 / 4


def test_chao1_uses_the_bias_corrected_form_without_doubletons() -> None:
    stats = EnumStats.from_counts([1, 1, 1, 3])
    assert (stats.f1, stats.f2) == (3, 0)
    assert stats.chao1 == 4 + 3 * 2 / 2
    assert EnumStats.from_counts([5]).chao1 == 1.0


def test_enum_equality_merges_integral_numbers_and_keeps_booleans_apart() -> None:
    assert EnumStats.of_values([1, 1.0, 1e0]).s_obs == 1
    assert EnumStats.of_values([True, 1, False, 0]).s_obs == 4


def test_statistics_need_observations_and_positive_counts() -> None:
    with pytest.raises(ValueError, match="at least one"):
        EnumStats.from_counts([])
    with pytest.raises(ValueError, match="at least once"):
        EnumStats.from_counts([2, 0])
    with pytest.raises(TypeError):
        EnumStats.from_counts([True])


def test_view_statistics_cover_only_non_flagged_scalars() -> None:
    store = build_store(["x", "x", "y", [1], {"a": 1}, None, 2.0, 2])
    view = View.root(store)
    stats = enum_statistics(view)
    assert stats == EnumStats.of_values(["x", "x", "y", None, 2, 2])
    store.flags.flag_documents([0, 6])
    assert enum_statistics(view) == EnumStats.of_values(["x", "y", None, 2])
    assert enum_statistics(view_of([{"a": 1}, []])) is None
    assert len(scalar_view(view)) == 4


def test_vector_and_json_hold_the_six_statistics() -> None:
    stats = EnumStats.from_counts([1, 2, 2, 4])
    assert stats.vector().tolist() == [9, 4, 1, 2, 1 / 9, 4 + 1 / 4]
    assert stats.to_json() == {"chao1": 4.25, "f1": 1, "f2": 2, "good_turing": 1 / 9, "n": 9, "s_obs": 4}


# ---------------------------------------------------------------------------
# Emission (15.3)
# ---------------------------------------------------------------------------


def test_enum_keywords_list_each_distinct_value_once_in_canonical_order() -> None:
    assert enum_keywords([2, 1.0, 1, True, "b", "a", None]) == {"enum": [None, True, 1, 2, "a", "b"]}
    assert enum_keywords([3.0, 3]) == {"const": 3}
    with pytest.raises(ValueError):
        enum_keywords([])


def test_a_repeated_value_set_is_closed_as_enum() -> None:
    judgment, report = judge(COLORS)
    assert judgment.closed and judgment.keyword == "enum"
    assert judgment.keywords() == {"enum": ["blue", "red"]}
    decision = judgment.decision
    assert (decision.method, decision.technique) == ("mdl", ENUM_ESTIMATION)
    # No bundle holds the Decision_Model, so MDL is its Fallback_Behavior (22.14).
    assert decision.fallback == FallbackInfo(DM_CLOSED_ENUM, "missing-model")
    assert [entry.kind for entry in report.decisions] == [CLOSED_OR_OPEN_ENUM]
    assert report.rejected == ()


def test_a_single_repeated_value_is_closed_as_const() -> None:
    judgment, _ = judge(["x", "x", "x"])
    assert judgment.closed and judgment.keywords() == {"const": "x"}
    judgment, _ = judge([1, 1.0])
    assert judgment.keywords() == {"const": 1}


def test_closed_provenance_records_statistics_outcome_and_technique() -> None:
    judgment, _ = judge(COLORS)
    record = judgment.provenance()
    assert record.technique == ENUM_ESTIMATION
    assert (record.support.consistent, record.support.total) == (40, 40)
    assert record.details["enum_statistics"] == EnumStats.of_values(COLORS).to_json()
    assert record.details["outcome"] == "closed"
    assert record.confidence == judgment.decision.confidence
    assert record.cutoff is not None and record.cutoff.source == MDL_COMPARISON


# ---------------------------------------------------------------------------
# MDL with the Good–Turing charge (15.6)
# ---------------------------------------------------------------------------


def test_good_turing_charge_is_the_exception_cost_of_the_singletons() -> None:
    view = view_of(MOSTLY_SINGLETONS)
    expected = math.fsum([CAT((2, 3)), U("b" * 20), U("c" * 20), U("d" * 20)])
    assert good_turing_charge(view) == expected
    assert good_turing_charge(view_of(COLORS)) == 0.0


def test_the_closed_candidate_pays_the_good_turing_charge() -> None:
    view = view_of(MOSTLY_SINGLETONS)
    judgment, report = judge(MOSTLY_SINGLETONS)
    scorer = MDLScorer()
    closed_node = SchemaNode(kw={"enum": ["a" * 20, "b" * 20, "c" * 20, "d" * 20]})
    uncharged = scorer.score(closed_node, view).total
    open_dl = scorer.score(SchemaNode(kw={"type": ["string"]}), view).total
    assert uncharged < open_dl  # the charge would be what keeps the Path open
    assert good_turing_charge(view) > 1.0
    # 15.9: three values occur in one document each, so the closed candidate is withheld before any comparison
    # (Document_Support); the engine resolves the point among the open candidate alone, without the model.
    (open_,) = judgment.decision.resolution.candidates
    assert open_.outcome == "open" and open_.score == open_dl
    assert judgment.decision.details["withheld"] == [
        {"label": "closed", "rule": "document_support", "documents": 5, "single_document_values": 3}
    ]
    assert not judgment.closed and judgment.keywords() == {}
    (entry,) = report.rejected
    assert (entry.category, entry.path, entry.technique) == ("enums", "$", ENUM_ESTIMATION)
    assert entry.candidate == {"distinct_values": 4, "keyword": "enum"}
    assert entry.details["enum_statistics"] == EnumStats.of_values(MOSTLY_SINGLETONS).to_json()
    assert (entry.details["outcome"], entry.details["method"]) == ("open", "mdl")


def test_mdl_selection_does_not_depend_on_decide_mdl() -> None:
    off, _ = judge(COLORS, {ENUM_ESTIMATION: True, "decide.mdl": False})
    on, _ = judge(COLORS, {ENUM_ESTIMATION: True, "decide.mdl": True})
    assert off.closed and on.closed
    assert off.decision.resolution == on.decision.resolution


def test_a_caller_supplied_open_node_is_scored() -> None:
    open_node = SchemaNode(kw={"type": ["string"], "maxLength": 4})
    judgment, _ = judge(COLORS, open_node=open_node)
    assert judgment.decision.resolution.candidates[1].score == MDLScorer().score(open_node, view_of(COLORS)).total
    with pytest.raises(ValueError, match="must not hold"):
        judge(COLORS, open_node=SchemaNode(kw={"enum": ["red"]}))


# ---------------------------------------------------------------------------
# Forced open (15.5) and the fallback (15.7)
# ---------------------------------------------------------------------------


def test_all_singletons_force_open_without_consulting_the_engine() -> None:
    model = StubModel([1.0, 0.0])  # would say closed
    states = {ENUM_ESTIMATION: True, DM_CLOSED_ENUM: True}
    judgment, report = judge(["a", "b", "c"], states, bundle=StubBundle({SYNTHETIC: model}))
    assert not judgment.closed and judgment.decision is None and judgment.rule == "15.5"
    assert model.seen == [] and report.decisions == ()
    (entry,) = report.rejected
    assert entry.technique == ENUM_ESTIMATION
    assert entry.details["enum_statistics"] == EnumStats.of_values(["a", "b", "c"]).to_json()
    assert (entry.details["outcome"], entry.details["rule"]) == ("open", "15.5")


def test_no_scalar_observation_is_open_with_nothing_recorded() -> None:
    judgment, report = judge([{"a": 1}, [2]])
    assert not judgment.closed and judgment.stats is None and judgment.technique is None
    assert report.decisions == () and report.rejected == ()
    store = build_store(["x", "x"])
    store.flags.flag_documents([0, 1])
    rep = InferenceReport()
    judgment = judge_enum(View.root(store), engine({ENUM_ESTIMATION: True}, report=rep), report=rep)
    assert not judgment.closed and judgment.stats is None and rep.decisions == ()


@pytest.mark.parametrize("dm_enabled", [False, True])
def test_disabled_enum_estimation_falls_back_to_open(dm_enabled: bool) -> None:
    model = StubModel([1.0, 0.0])
    states = {ENUM_ESTIMATION: False, DM_CLOSED_ENUM: dm_enabled}
    judgment, report = judge(COLORS, states, bundle=StubBundle({SYNTHETIC: model}))
    assert not judgment.closed and judgment.stats is None and judgment.keywords() == {}
    assert judgment.decision.method == "fallback"
    assert judgment.technique == f"fallback:{CLOSED_OR_OPEN_ENUM}"
    assert judgment.decision.fallback == FallbackInfo(ENUM_ESTIMATION if dm_enabled else DM_CLOSED_ENUM, "disabled")
    assert model.seen == []
    (entry,) = report.decisions
    assert entry.fallback is not None
    assert report.rejected == ()  # no statistics were computed (15.8)
    with pytest.raises(ValueError, match="open"):
        judgment.provenance()


# ---------------------------------------------------------------------------
# The Decision_Model (15.2)
# ---------------------------------------------------------------------------


def test_the_decision_model_decides_on_the_six_statistics() -> None:
    model = StubModel([0.2, 0.8])
    states = {ENUM_ESTIMATION: True, DM_CLOSED_ENUM: True}
    judgment, report = judge(COLORS, states, bundle=StubBundle({SYNTHETIC: model}))
    assert not judgment.closed  # MDL would close it; the model says open
    assert (judgment.decision.method, judgment.technique) == ("decision-model", DM_CLOSED_ENUM)
    assert judgment.decision.confidence == 0.8
    (seen,) = model.seen
    assert seen.ravel().tolist() == EnumStats.of_values(COLORS).vector().tolist()
    (entry,) = report.rejected
    assert entry.technique == DM_CLOSED_ENUM and entry.details["method"] == "decision-model"


def test_a_closed_decision_model_judgment_records_its_technique() -> None:
    states = {ENUM_ESTIMATION: True, DM_CLOSED_ENUM: True}
    judgment, _ = judge(COLORS, states, bundle=StubBundle({SYNTHETIC: StubModel([0.9, 0.1])}))
    record = judgment.provenance()
    assert judgment.closed and record.technique == DM_CLOSED_ENUM and record.confidence == 0.9
    assert record.details["outcome"] == "closed"


def test_equal_probabilities_go_to_mdl() -> None:
    states = {ENUM_ESTIMATION: True, DM_CLOSED_ENUM: True}
    judgment, _ = judge(COLORS, states, bundle=StubBundle({SYNTHETIC: StubModel([0.5, 0.5])}))
    assert judgment.closed and judgment.decision.cutoff.source == MDL_COMPARISON


def test_custom_features_receive_the_scalar_view_and_candidates() -> None:
    model = StubModel([0.9, 0.1])
    seen: list[Any] = []

    def features(view: View, candidates: Any) -> np.ndarray:
        seen.append((len(view), [c.label for c in candidates]))
        return np.ones(3)

    states = {ENUM_ESTIMATION: True, DM_CLOSED_ENUM: True}
    judge(COLORS + [{"k": 1}], states, bundle=StubBundle({SYNTHETIC: model}), features=features)
    assert seen == [(40, ["closed", "open"])]
    assert model.seen[0].ravel().tolist() == [1.0, 1.0, 1.0]


def test_a_missing_decision_model_falls_back_to_mdl() -> None:
    report = InferenceReport()
    eng = engine({ENUM_ESTIMATION: True, DM_CLOSED_ENUM: True}, bundle=StubBundle(), report=report)
    judgment = judge_enum(view_of(COLORS), eng, report=report)
    assert judgment.closed and judgment.decision.method == "mdl"
    assert judgment.decision.fallback == FallbackInfo(DM_CLOSED_ENUM, "missing-model")
    assert [sub.artifact for sub in report.substitutions] == [SYNTHETIC]


def test_enum_estimation_is_implemented_and_enabled_by_default() -> None:
    spec = REGISTRY.technique(ENUM_ESTIMATION)
    assert spec.implemented and spec.default_enabled
    assert spec.serves == (CLOSED_OR_OPEN_ENUM,)
