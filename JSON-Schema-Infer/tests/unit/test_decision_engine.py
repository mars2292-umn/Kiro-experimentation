"""Unit tests for decide/engine.py (tasks 11.1 and 11.3).

Requirements 17.2, 17.3, 19.3, 19.4, 19.5, 19.7, 20.1, 20.7, 22.12, 22.13,
22.14, 23.8, 23.9, 26.3, 26.10, 28.4, 28.6, 28.7, 28.11, 28.12.
Decision_Models are stubs, and forced ties use precomputed Scores.
"""

from __future__ import annotations

import itertools
import math
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import (
    DECISION_MODEL_COMPARISON,
    MDL_COMPARISON,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    ProvenanceRecord,
    Resolution,
    Support,
    laplace_confidence,
)
from jsinfer.core.report import InferenceReport, Substitution
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import (
    JUDGMENT_OUTCOMES,
    Candidate,
    DecisionEngine,
    DecisionPoint,
    mdl_posterior,
)
from jsinfer.learn.bundle import load_bundle
from jsinfer.mdl.scorer import MDLScorer, Score

DM_RECORD_MAP = "decide.dm_record_map"
MDL = "decide.mdl"
WEAK = "decide.weak_supervision"
EXCEPTIONS = "robust.mdl_exceptions"
SYNTHETIC = "dm.record_or_map.synthetic"
WEAK_MODEL = "dm.record_or_map.weak"


class StubBundle:
    """Models and Learned_Thresholds by name, like ResolvedBundle.model and .threshold."""

    def __init__(self, models: dict[str, Any] | None = None, thresholds: dict[str, float] | None = None) -> None:
        self.models = dict(models or {})
        self.thresholds = dict(thresholds or {})

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


class StubModel:
    """A Decision_Model with fixed calibrated probabilities; records the feature arrays it sees."""

    def __init__(self, classes: tuple[str, ...], proba: list[float]) -> None:
        self.classes = classes
        self.proba = np.asarray(proba, dtype=np.float64)
        self.seen: list[np.ndarray] = []

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        self.seen.append(np.array(x))
        return self.proba.reshape(1, -1)


def engine(*enabled: str, bundle: Any = None, report: InferenceReport | None = None) -> DecisionEngine:
    """An engine with exactly the ``enabled`` Techniques on; the others are off, whatever their default."""
    return DecisionEngine({tid: tid in enabled for tid in REGISTRY.ids()}, bundle=bundle, report=report)


def score(model_bits: float, data_bits: float, *, exception_bits: float = 0.0, n_rejected: int = 0) -> Score:
    return Score(float(model_bits), float(data_bits), float(exception_bits), n_rejected, max(n_rejected, 1))


def judgment(kind: str, scores: tuple[Score, Score]) -> list[Candidate]:
    """Technique-less candidates for the two outcomes of a Judgment_Call kind, with forced scores."""
    return [
        Candidate(technique=None, outcome=label, order_key=(i,), score=s)
        for i, (label, s) in enumerate(zip(JUDGMENT_OUTCOMES[kind], scores))
    ]


def features() -> np.ndarray:
    return np.array([1.0, 2.0, 3.0])


# ---------------------------------------------------------------------------
# MDL selection (19.3, 19.4, 19.5, 20.7, 28.6)
# ---------------------------------------------------------------------------


def test_mdl_selects_the_lowest_description_length_on_the_view() -> None:
    view = View.root(build_store([{"a": i, "b": -i} for i in range(25)]))
    record = SchemaNode.from_json(
        {"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}}, "required": ["a", "b"]}
    )
    map_ = SchemaNode.from_json({"type": "object", "additionalProperties": {"type": "integer"}})
    candidates = [
        Candidate(technique=None, outcome="map", order_key=(1,), node=map_),
        Candidate(technique=None, outcome="record", order_key=(0,), node=record),
    ]
    report = InferenceReport()
    decision = engine(MDL, report=report).resolve(DecisionPoint("record_or_map"), candidates, view)

    expected = {"record": MDLScorer().score(record, view), "map": MDLScorer().score(map_, view)}
    winner = min(expected, key=lambda label: expected[label].total)
    assert decision.method == "mdl" and decision.label == winner
    assert decision.technique == MDL
    # The Decision_Model is disabled, so MDL is its Fallback_Behavior (22.13).
    assert decision.fallback == FallbackInfo(DM_RECORD_MAP, "disabled")
    # Every candidate in canonical order with the 19.5 breakdown.
    listed = decision.resolution.candidates
    assert [c.outcome for c in listed] == ["record", "map"]
    for entry in listed:
        s = expected[entry.outcome]
        assert entry.technique is None
        assert (entry.score, entry.model_bits, entry.data_bits, entry.exception_bits) == (
            s.total,
            s.model_bits,
            s.data_bits,
            s.exception_bits,
        )
        assert (entry.n_rejected, entry.n_scored) == (s.n_rejected, s.n_scored)
    assert listed[decision.resolution.selected].outcome == winner
    # Confidence is the code-length posterior; the cut-off is the MDL comparison (20.7).
    posterior = mdl_posterior([expected["record"].total, expected["map"].total])
    assert decision.confidence == posterior[decision.resolution.selected]
    assert decision.cutoff == Cutoff(MDL_COMPARISON, scores=listed)
    assert decision.path == () and report.decisions == (decision.entry,)


def test_mdl_posterior_is_normalized_stable_and_order_free() -> None:
    assert mdl_posterior([3.0, 4.0]) == pytest.approx([2 / 3, 1 / 3])
    huge = mdl_posterior([1e7, 1e7 + 1.0, 2e7])
    assert huge == pytest.approx([2 / 3, 1 / 3, 0.0])
    assert math.fsum(mdl_posterior([5.0, 1.0, 9.5, 1.0])) == pytest.approx(1.0)
    lengths = [12.5, 3.25, 3.25, 700.0]
    for order in itertools.permutations(range(4)):
        shuffled = mdl_posterior([lengths[i] for i in order])
        assert [shuffled[order.index(i)] for i in range(4)] == mdl_posterior(lengths)
    with pytest.raises(ValueError):
        mdl_posterior([])


def test_a_description_length_tie_goes_to_the_mandated_rule_before_model_bits() -> None:
    # Record has more model bits but the same total; 4.10 sends the tie to Record.
    candidates = judgment("record_or_map", (score(10, 5), score(5, 10)))
    decision = engine(MDL).resolve(DecisionPoint("record_or_map", "$.x"), candidates)
    assert decision.label == "record" and decision.confidence == pytest.approx(0.5)


def test_without_a_tie_rule_fewer_model_bits_win() -> None:
    candidates = judgment("closed_or_open_record", (score(10, 5), score(5, 10)))
    assert engine(MDL).resolve(DecisionPoint("closed_or_open_record"), candidates).label == "open"


def test_remaining_ties_go_to_priority_then_order_key() -> None:
    dp = DecisionPoint("string_constraint", "$.s")
    tied = score(4, 4)
    latent = Candidate(technique="values.latent_type", outcome="format=date", order_key=(5,), score=tied)
    date = Candidate(technique="values.format", outcome="format=date", order_key=(1,), score=tied)
    email = Candidate(technique="values.format", outcome="format=email", order_key=(0,), score=tied)
    on = engine(MDL, "values.latent_type", "values.format")
    for order in itertools.permutations([latent, date, email]):
        decision = on.resolve(dp, order)
        assert decision.candidate is latent and decision.technique == "values.latent_type"
    # Within one Technique, the documented candidate order decides.
    assert on.resolve(dp, [date, email]).candidate is email


def test_tie_criteria_rank_the_tied_candidates() -> None:
    # Bounds (16.7): the candidate nearest the observed extreme, omission farthest.
    tied = score(3, 3)
    bounds = engine("values.bounds")
    ladder = [
        Candidate(technique="values.bounds", outcome="omit", order_key=(0,), score=tied),
        Candidate(technique="values.bounds", outcome="maximum=200", order_key=(1,), score=tied,
                  tie_ranks={"nearest-observed-extreme": 100}),
        Candidate(technique="values.bounds", outcome="maximum=100", order_key=(2,), score=tied,
                  tie_ranks={"nearest-observed-extreme": 0}),
    ]
    decision = bounds.resolve(DecisionPoint("bound", "$.n"), ladder, mdl_by="values.bounds")
    assert (decision.method, decision.label, decision.technique) == ("mdl", "maximum=100", "values.bounds")
    assert decision.fallback is None
    # Pattern selection (14.2): no Pattern first, then fewer sub-patterns.
    patterns = engine("values.pattern")
    one = Candidate(technique="values.pattern", outcome="p1", order_key=(1,), score=tied,
                    tie_ranks={"fewer-subpatterns": 1})
    two = Candidate(technique="values.pattern", outcome="p2", order_key=(0,), score=tied,
                    tie_ranks={"fewer-subpatterns": 2})
    none = Candidate(technique="values.pattern", outcome="none", order_key=(2,), score=tied)
    dp = DecisionPoint("string_constraint", "$.s")
    assert patterns.resolve(dp, [one, two, none], mdl_by="values.pattern").label == "none"
    assert patterns.resolve(dp, [one, two], mdl_by="values.pattern").label == "p1"


def test_mdl_by_needs_its_technique_in_effect() -> None:
    candidate = Candidate(technique="values.bounds", outcome="omit", score=score(1, 1))
    with pytest.raises(ValueError, match="does not serve"):
        engine("values.bounds").resolve(DecisionPoint("bound"), [candidate], mdl_by="values.format")


# ---------------------------------------------------------------------------
# Exception-bearing candidates (17.2, 17.3)
# ---------------------------------------------------------------------------


def exception_case(widen_bits: float, flag_bits: float) -> tuple[View, list[Candidate]]:
    store = build_store([1, 2, 3, 1000])
    rejects = View.root(store, [3])
    candidates = [
        Candidate(technique="values.bounds", outcome="maximum=1000", order_key=(0,), score=score(1, widen_bits)),
        Candidate(technique=EXCEPTIONS, outcome="maximum=3", order_key=(0,), rejects=rejects,
                  score=score(1, flag_bits, exception_bits=1.0, n_rejected=1)),
    ]
    return rejects, candidates


def test_exception_candidates_are_offered_only_with_mdl_exceptions() -> None:
    rejects, candidates = exception_case(widen_bits=30, flag_bits=10)
    decision = engine("values.bounds").resolve(DecisionPoint("bound"), candidates, mdl_by="values.bounds")
    assert decision.label == "maximum=1000" and decision.flagged is None
    assert [c.technique for c in decision.resolution.candidates] == ["values.bounds"]
    assert rejects.store.flags.is_empty


def test_a_winning_exception_candidate_flags_its_rows() -> None:
    rejects, candidates = exception_case(widen_bits=30, flag_bits=10)
    decision = engine("values.bounds", EXCEPTIONS).resolve(DecisionPoint("bound"), candidates, mdl_by="values.bounds")
    assert (decision.label, decision.technique) == ("maximum=3", EXCEPTIONS)
    assert decision.flagged is rejects and decision.details == {"flagged": 1}
    assert rejects.store.flags.n_items == 1 and len(rejects.live()) == 0
    assert [c.technique for c in decision.resolution.candidates] == ["values.bounds", EXCEPTIONS]


def test_description_length_ties_go_to_widening() -> None:
    rejects, candidates = exception_case(widen_bits=10, flag_bits=10)
    decision = engine("values.bounds", EXCEPTIONS).resolve(DecisionPoint("bound"), candidates, mdl_by="values.bounds")
    assert decision.label == "maximum=1000" and decision.flagged is None
    assert rejects.store.flags.is_empty


# ---------------------------------------------------------------------------
# Decision_Models (22.12, 22.14, 23.8, 23.9)
# ---------------------------------------------------------------------------


def test_the_decision_model_picks_the_higher_calibrated_probability() -> None:
    model = StubModel(("map", "record"), [0.7, 0.3])
    # MDL would pick Record; the Decision_Model comes first.
    candidates = judgment("record_or_map", (score(1, 1), score(50, 50)))
    report = InferenceReport()
    decision = engine(DM_RECORD_MAP, MDL, bundle=StubBundle({SYNTHETIC: model}), report=report).resolve(
        DecisionPoint("record_or_map", "$.m"), candidates, features=features
    )
    assert (decision.method, decision.label, decision.technique) == ("decision-model", "map", DM_RECORD_MAP)
    assert decision.confidence == 0.7 and decision.fallback is None
    assert decision.resolution.candidates == (CandidateScore(None, "record", 0.3), CandidateScore(None, "map", 0.7))
    # 20.7: the argmax is a Decision_Model comparison of the calibrated probabilities, naming the artifact.
    assert decision.cutoff == Cutoff(DECISION_MODEL_COMPARISON, scores=decision.resolution.candidates, artifact=SYNTHETIC)
    assert decision.details == {"decision_model": SYNTHETIC}
    assert len(model.seen) == 1 and model.seen[0].shape == (1, 3)
    assert report.substitutions == ()


def test_the_weak_model_resolves_while_weak_supervision_is_on() -> None:
    bundle = StubBundle(
        {SYNTHETIC: StubModel(("record", "map"), [0.9, 0.1]), WEAK_MODEL: StubModel(("record", "map"), [0.2, 0.8])}
    )
    candidates = judgment("record_or_map", (score(1, 1), score(1, 1)))
    dp = DecisionPoint("record_or_map")
    weak = engine(DM_RECORD_MAP, WEAK, bundle=bundle).resolve(dp, candidates, features=features)
    assert (weak.label, weak.details["decision_model"]) == ("map", WEAK_MODEL)
    synthetic = engine(DM_RECORD_MAP, bundle=bundle).resolve(dp, candidates, features=features)
    assert (synthetic.label, synthetic.details["decision_model"]) == ("record", SYNTHETIC)


def test_a_missing_weak_model_falls_back_to_the_synthetic_model() -> None:
    report = InferenceReport()
    bundle = StubBundle({SYNTHETIC: StubModel(("record", "map"), [0.6, 0.4])})
    on = engine(DM_RECORD_MAP, WEAK, bundle=bundle, report=report)
    candidates = judgment("record_or_map", (score(1, 1), score(1, 1)))
    decision = on.resolve(DecisionPoint("record_or_map"), candidates, features=features)
    assert decision.details["decision_model"] == SYNTHETIC
    assert report.substitutions == (
        Substitution(
            WEAK_MODEL,
            FallbackInfo(WEAK, "missing-model"),
            "missing",
            "Fallback_Behavior of judgment_model: synthetic-only Decision_Model",
        ),
    )


def test_a_missing_decision_model_falls_through_to_mdl_and_is_recorded() -> None:
    report = InferenceReport()

    def no_features() -> np.ndarray:
        raise AssertionError("features are computed only when a Decision_Model is used")

    candidates = judgment("record_or_map", (score(9, 9), score(1, 1)))
    decision = engine(DM_RECORD_MAP, MDL, bundle=StubBundle(), report=report).resolve(
        DecisionPoint("record_or_map"), candidates, features=no_features
    )
    assert (decision.method, decision.label) == ("mdl", "map")
    assert decision.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")
    assert report.substitutions == (
        Substitution(SYNTHETIC, FallbackInfo(DM_RECORD_MAP, "missing-model"), "missing", "decide.mdl at record_or_map"),
    )
    # With MDL disabled too, the kind's Fallback_Behavior applies, citing the missing model.
    fallback = engine(DM_RECORD_MAP, bundle=StubBundle()).resolve(DecisionPoint("record_or_map"), candidates)
    assert (fallback.method, fallback.label) == ("fallback", "record")
    assert fallback.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")


def test_the_engine_records_the_bundle_stage_substitution() -> None:
    states = {DM_RECORD_MAP: True, MDL: True}
    resolved = load_bundle(DEFAULT_MODEL_BUNDLE).resolve(states)
    report = InferenceReport()
    for sub in resolved.substitutions:
        report.record_substitution(sub)
    before = report.substitutions
    DecisionEngine(states, bundle=resolved, report=report).resolve(
        DecisionPoint("record_or_map"), judgment("record_or_map", (score(1, 2), score(1, 1)))
    )
    assert report.substitutions == before and any(s.artifact == SYNTHETIC for s in before)


def test_equal_probabilities_are_resolved_by_mdl() -> None:
    bundle = StubBundle({SYNTHETIC: StubModel(("record", "map"), [0.5, 0.5])})
    candidates = judgment("record_or_map", (score(5, 5), score(2, 2)))
    dp = DecisionPoint("record_or_map")
    for enabled in ((DM_RECORD_MAP, MDL), (DM_RECORD_MAP,)):  # 22.12 holds whatever the state of decide.mdl
        decision = engine(*enabled, bundle=bundle).resolve(dp, candidates, features=features)
        assert (decision.method, decision.label, decision.confidence) == ("decision-model", "map", 0.5)
        assert decision.details == {"decision_model": SYNTHETIC, "tie_break": MDL_COMPARISON}
        assert decision.cutoff is not None and [s.score for s in decision.cutoff.scores] == [10.0, 4.0]


def test_decision_model_inputs_are_checked() -> None:
    dp = DecisionPoint("record_or_map")
    candidates = judgment("record_or_map", (score(1, 1), score(1, 1)))
    wrong = engine(DM_RECORD_MAP, bundle=StubBundle({SYNTHETIC: StubModel(("a", "b"), [0.5, 0.5])}))
    with pytest.raises(ValueError, match="one candidate per class"):
        wrong.resolve(dp, candidates, features=features)
    right = engine(DM_RECORD_MAP, bundle=StubBundle({SYNTHETIC: StubModel(("record", "map"), [0.5, 0.5])}))
    with pytest.raises(ValueError, match="features callable"):
        right.resolve(dp, candidates)
    bad = engine(DM_RECORD_MAP, bundle=StubBundle({SYNTHETIC: StubModel(("record", "map"), [1.5, -0.5])}))
    with pytest.raises(ValueError, match="probability"):
        bad.resolve(dp, candidates, features=features)


# ---------------------------------------------------------------------------
# Priority order (19.7, 28.7)
# ---------------------------------------------------------------------------


def test_priority_order_when_mdl_is_disabled() -> None:
    dp = DecisionPoint("string_constraint", "$.s")
    fmt = Candidate(technique="values.format", outcome="format=email", score=score(1, 1))
    latent = Candidate(technique="values.latent_type", outcome="format=date", score=score(9, 9))
    cutoff = Cutoff("semantic.min_probability", value=0.5)
    decision = engine("values.latent_type", "values.format").resolve(dp, [fmt, latent], cutoff=cutoff)
    assert (decision.method, decision.candidate, decision.technique) == ("priority", latent, "values.latent_type")
    assert [c.score for c in decision.resolution.candidates] == [None, None]
    assert decision.fallback == FallbackInfo(MDL, "disabled") and decision.confidence is None
    assert decision.cutoff == cutoff
    # One candidate is not a selection, so no Fallback_Behavior is recorded.
    single = engine("values.format").resolve(dp, [fmt, latent])
    assert single.candidate is fmt and single.fallback is None


# ---------------------------------------------------------------------------
# The exception check after a Decision_Model or priority step (17.2, 17.3, 20.7)
# ---------------------------------------------------------------------------


def model_case(flag_bits: float) -> tuple[View, list[Candidate]]:
    """Record (2 bits), Map (10 bits; the Decision_Model's pick), and an exception-bearing Map of 1 + flag_bits."""
    rejects = View.root(build_store([{"a": 1}, {"b": 2}, {"c": 3}]), [2])
    flagger = Candidate(technique=EXCEPTIONS, outcome="map", order_key=(0,), rejects=rejects,
                        score=score(1, flag_bits, exception_bits=1.0, n_rejected=1))
    return rejects, [*judgment("record_or_map", (score(1, 1), score(5, 5))), flagger]


def test_an_exception_candidate_with_fewer_bits_replaces_the_decision_models_winner() -> None:
    rejects, candidates = model_case(flag_bits=6)
    model = StubModel(("record", "map"), [0.3, 0.7])
    report = InferenceReport()
    on = engine(DM_RECORD_MAP, EXCEPTIONS, bundle=StubBundle({SYNTHETIC: model}), report=report)
    decision = on.resolve(DecisionPoint("record_or_map"), candidates, features=features)

    assert (decision.method, decision.candidate, decision.technique) == ("mdl", candidates[2], EXCEPTIONS)
    # The comparison covers the widening winner and the exception-bearing candidates, in canonical order.
    listed = decision.resolution.candidates
    assert [(c.technique, c.score) for c in listed] == [(None, 10.0), (EXCEPTIONS, 7.0)]
    assert decision.resolution.selected == 1
    assert decision.confidence == mdl_posterior([10.0, 7.0])[1]
    assert decision.cutoff == Cutoff(MDL_COMPARISON, scores=listed)
    assert decision.fallback is None  # the Decision_Model step recorded none
    assert decision.details == {"decision_model": SYNTHETIC, "replaced": "decision-model", "flagged": 1}
    assert decision.flagged is rejects and rejects.store.flags.n_items == 1
    assert len(model.seen) == 1 and report.decisions == (decision.entry,)


def test_the_decision_models_winner_survives_a_tie_and_records_the_comparison() -> None:
    rejects, candidates = model_case(flag_bits=9)  # 10 bits each: ties go to widening
    bundle = StubBundle({SYNTHETIC: StubModel(("record", "map"), [0.3, 0.7])})
    decision = engine(DM_RECORD_MAP, EXCEPTIONS, bundle=bundle).resolve(
        DecisionPoint("record_or_map"), candidates, features=features
    )
    assert (decision.method, decision.candidate, decision.confidence) == ("decision-model", candidates[1], 0.7)
    assert decision.cutoff is not None and decision.cutoff.source == DECISION_MODEL_COMPARISON
    compared = Resolution("mdl", 0, (CandidateScore(None, "map", 10.0, model_bits=5, data_bits=5, exception_bits=0,
                                                    n_rejected=0, n_scored=1),
                                     CandidateScore(EXCEPTIONS, "map", 10.0, model_bits=1, data_bits=9,
                                                    exception_bits=1, n_rejected=1, n_scored=1)))
    assert decision.details == {"decision_model": SYNTHETIC, "exception_check": compared.to_json()}
    assert decision.flagged is None and rejects.store.flags.is_empty
    # With robust.mdl_exceptions off, no check runs.
    off = engine(DM_RECORD_MAP, bundle=bundle).resolve(DecisionPoint("record_or_map"), candidates, features=features)
    assert off.details == {"decision_model": SYNTHETIC} and off.candidate is candidates[1]


def test_an_exception_candidate_with_fewer_bits_replaces_the_priority_winner() -> None:
    rejects, candidates = exception_case(widen_bits=30, flag_bits=10)
    omit = Candidate(technique="values.bounds", outcome="omit", order_key=(1,), score=score(1, 40))
    cutoff = Cutoff("semantic.min_probability", value=0.5)
    decision = engine("values.bounds", EXCEPTIONS).resolve(DecisionPoint("bound"), [*candidates, omit], cutoff=cutoff)
    assert (decision.method, decision.label, decision.technique) == ("mdl", "maximum=3", EXCEPTIONS)
    # The omitted bound is not compared: only the priority winner and the exception-bearing candidates are.
    assert [c.outcome for c in decision.resolution.candidates] == ["maximum=1000", "maximum=3"]
    assert decision.cutoff == Cutoff(MDL_COMPARISON, scores=decision.resolution.candidates)
    # 19.7: priority among two candidates was the Fallback_Behavior of the disabled MDL selection; it is kept.
    assert decision.fallback == FallbackInfo(MDL, "disabled")
    assert decision.details == {"replaced": "priority", "flagged": 1}
    assert decision.flagged is rejects and rejects.store.flags.n_items == 1


def test_the_priority_winner_survives_a_tie_and_keeps_its_cutoff() -> None:
    rejects, candidates = exception_case(widen_bits=10, flag_bits=10)
    decision = engine("values.bounds", EXCEPTIONS).resolve(DecisionPoint("bound"), candidates)
    assert (decision.method, decision.label, decision.confidence) == ("priority", "maximum=1000", None)
    assert decision.cutoff is None and decision.fallback is None  # no cut-off applied (20.7)
    check = decision.details["exception_check"]
    assert isinstance(check, dict) and check["method"] == "mdl" and check["selected"] == 0
    assert [c["score"] for c in check["candidates"]] == [11, 11]  # type: ignore[index]
    assert rejects.store.flags.is_empty


def test_the_fallback_step_runs_no_exception_check() -> None:
    rejects, candidates = exception_case(widen_bits=30, flag_bits=1)
    for enabled in ((EXCEPTIONS,), (EXCEPTIONS, MDL)):  # 28.4: values.bounds is disabled, so nothing can act
        states = {**dict.fromkeys(enabled, True), "values.bounds": False}
        decision = DecisionEngine(states).resolve(DecisionPoint("bound"), candidates)
        assert (decision.method, decision.outcome, decision.candidate) == ("fallback", "none", None)
        assert decision.cutoff is None and decision.details == {"fallback_behavior": "omit"}
    assert rejects.store.flags.is_empty


# ---------------------------------------------------------------------------
# Fallback_Behaviors (26.10, 28.4, 28.11, 28.12)
# ---------------------------------------------------------------------------


def test_every_serving_technique_disabled_applies_the_fallback() -> None:
    report = InferenceReport()
    candidates = judgment("record_or_map", (score(9, 9), score(1, 1)))
    decision = engine(report=report).resolve(DecisionPoint("record_or_map", "$.r"), candidates)
    assert (decision.method, decision.label, decision.technique) == ("fallback", "record", "fallback:record_or_map")
    assert decision.candidate is candidates[0] and decision.confidence is None and decision.cutoff is None
    assert decision.fallback == FallbackInfo(DM_RECORD_MAP, "disabled")
    assert decision.details == {"fallback_behavior": "Record"}
    assert decision.resolution.candidates == (CandidateScore(None, "record", None), CandidateScore(None, "map", None))
    support = Support(3, 4)
    record = decision.provenance(support)
    assert record.confidence == laplace_confidence(support) and record.fallback == decision.fallback
    assert ProvenanceRecord.from_json(record.to_json()) == record
    assert report.decisions == (decision.entry,)


def test_disabled_techniques_produce_no_candidates() -> None:
    grouping = Candidate(technique="variants.bernoulli_mixture", outcome="g1", score=score(1, 1))
    decision = engine(MDL).resolve(DecisionPoint("variant_grouping"), [grouping])
    assert (decision.method, decision.outcome, decision.candidate) == ("fallback", "one-variant", None)
    assert decision.resolution.candidates == (CandidateScore(None, "one-variant", None),)
    assert decision.fallback == FallbackInfo("variants.minhash_lsh", "disabled")


def test_a_missing_learned_threshold_drops_the_techniques_candidates() -> None:
    dp = DecisionPoint("variant_grouping")
    lsh = Candidate(technique="variants.minhash_lsh", outcome="lsh", score=score(1, 1))
    missing = engine("variants.minhash_lsh", bundle=StubBundle()).resolve(dp, [lsh])
    assert missing.method == "fallback"
    assert missing.fallback == FallbackInfo("variants.minhash_lsh", "missing-threshold")
    with_threshold = StubBundle(thresholds={"lsh.keyset_jaccard": 0.5})
    present = engine("variants.minhash_lsh", bundle=with_threshold).resolve(dp, [lsh])
    assert (present.method, present.candidate) == ("priority", lsh)


def test_a_gated_decision_model_names_its_disabled_gate() -> None:
    candidates = [
        Candidate(technique="values.enum_estimation", outcome="closed", order_key=(0,), score=score(1, 1)),
        Candidate(technique="values.enum_estimation", outcome="open", order_key=(1,), score=score(2, 2)),
    ]
    gated = DecisionEngine({"decide.dm_closed_enum": True, "values.enum_estimation": False})
    decision = gated.resolve(DecisionPoint("closed_or_open_enum"), candidates)
    assert (decision.method, decision.label) == ("fallback", "open")
    assert decision.fallback == FallbackInfo("values.enum_estimation", "disabled")
    # With enum estimation on and its Decision_Model off, enum estimation selects by MDL (15.6).
    mdl = engine("values.enum_estimation").resolve(
        DecisionPoint("closed_or_open_enum"), candidates, mdl_by="values.enum_estimation"
    )
    assert (mdl.method, mdl.label, mdl.technique) == ("mdl", "closed", "values.enum_estimation")
    assert mdl.fallback == FallbackInfo("decide.dm_closed_enum", "disabled")


def test_the_candidate_order_fallback_selects_the_first_candidate() -> None:
    candidates = [
        Candidate(technique=None, outcome="remove", order_key=(3,)),
        Candidate(technique=None, outcome="add-type", order_key=(0,)),
    ]
    decision = engine().resolve(DecisionPoint("relaxation", "$.x"), candidates)
    assert (decision.method, decision.label) == ("fallback", "add-type")
    assert decision.fallback == FallbackInfo(MDL, "disabled")


def test_a_usable_technique_without_candidates_is_an_error() -> None:
    with pytest.raises(ValueError, match="values.bounds is in effect"):
        engine("values.bounds").resolve(DecisionPoint("bound"), [])


def test_the_default_run_configuration_resolves_judgment_calls_by_mdl() -> None:
    # decide.mdl is implemented, so it is on by default; the Decision_Models are not built yet.
    config = RunConfiguration.resolve()
    on = DecisionEngine.for_run(config, load_bundle(DEFAULT_MODEL_BUNDLE).resolve(config.techniques), InferenceReport())
    decision = on.resolve(DecisionPoint("tuple_or_list"), judgment("tuple_or_list", (score(1, 1), score(2, 2))))
    assert (decision.method, decision.label, decision.technique) == ("mdl", "tuple", MDL)
    assert decision.fallback == FallbackInfo("decide.dm_tuple_list", "disabled")  # the Decision_Model's fallback (22.13)
    # With decide.mdl off as well, the Fallback_Behavior applies.
    off = DecisionEngine.for_run(RunConfiguration.resolve({technique_setting(MDL): False}), None, None)
    decision = off.resolve(DecisionPoint("tuple_or_list"), judgment("tuple_or_list", (score(1, 1), score(2, 2))))
    assert (decision.method, decision.label) == ("fallback", "list")


# ---------------------------------------------------------------------------
# Determinism and validation
# ---------------------------------------------------------------------------


def test_results_do_not_depend_on_candidate_order() -> None:
    tied = score(2, 2)
    candidates = [
        Candidate(technique="values.format", outcome="format=uri", order_key=(1,), score=tied),
        Candidate(technique="values.format", outcome="format=email", order_key=(0,), score=score(1, 3)),
        Candidate(technique="values.latent_type", outcome="format=date", order_key=(0,), score=score(3, 1)),
        Candidate(technique="values.pattern", outcome="none", order_key=(0,), score=score(2, 3)),
    ]
    on = engine(MDL, "values.format", "values.latent_type", "values.pattern")
    seen = {
        canonical_bytes(on.resolve(DecisionPoint("string_constraint"), order).entry.to_json())
        for order in itertools.permutations(candidates)
    }
    assert len(seen) == 1


def test_candidate_validation() -> None:
    with pytest.raises(TypeError, match="needs a str label"):
        Candidate(technique=None, outcome=3)
    assert Candidate(technique=None, outcome=3, label="three").label == "three"
    with pytest.raises(ValueError, match="unknown criteria"):
        Candidate(technique=None, outcome="x", tie_ranks={"closest": 1})
    with pytest.raises(TypeError, match="order_key"):
        Candidate(technique=None, outcome="x", order_key=[1])  # type: ignore[arg-type]
    on = engine(MDL)
    unordered = [Candidate(technique=None, outcome=o, score=score(1, 1)) for o in ("record", "map")]
    with pytest.raises(ValueError, match="share the order key"):
        on.resolve(DecisionPoint("record_or_map"), unordered)
    with pytest.raises(ValueError, match="does not serve"):
        on.resolve(DecisionPoint("bound"), [Candidate(technique="values.format", outcome="x", score=score(1, 1))])
    with pytest.raises(ValueError, match="needs the candidate's node"):
        on.resolve(DecisionPoint("record_or_map"), [Candidate(technique=None, outcome="record")])
    with pytest.raises(ValueError, match="unregistered Decision_Point kind"):
        DecisionPoint("nope")
    with pytest.raises(ValueError, match="unregistered Technique ids"):
        DecisionEngine({"decide.nope": True})
