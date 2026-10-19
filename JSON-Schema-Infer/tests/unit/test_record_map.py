"""Unit tests for structure/record_map.py: Record-or-Map, Closed-or-Open-Record, and object nodes (task 13.1).

Requirements 4.1 (resolution with provenance and the rejected alternative),
4.2 (Record ``properties``), 4.3 (one Map value schema, no ``properties``),
4.4 (Map-key ``propertyNames``), 4.7–4.9 (closed and open Records), 4.10
(MDL with ties to Record, fallback Record), 4.11 (no keys), and 16.5 (Map
key-count bounds).
"""

from __future__ import annotations

import math
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import build_store
from jsinfer.core.paths import MAPVAL_STEP, key_step
from jsinfer.core.provenance import FallbackInfo, laplace_confidence
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine, mdl_posterior
from jsinfer.io.validator import Validator
from jsinfer.mdl.codes import CAT, U
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.record_map import (
    CLOSED,
    CLOSED_OR_OPEN_RECORD,
    DM_RECORD_MAP,
    MAP,
    OPEN,
    RECORD,
    RECORD_OR_MAP,
    KeyStats,
    RecordMapAnalyzer,
    closed_or_open,
    map_encoding,
    object_rows,
    record_encoding,
    record_or_map,
    unseen_key_charge,
)
from jsinfer.values.inferrer import ValueInferrer

VALIDATOR = Validator()
MDL = "decide.mdl"

RECORDS = [{"name": f"n{i}", "age": i % 7, "email": f"e{i}@example.org"} for i in range(30)]
USER_MAPS = [{f"user{3 * i + j}": {"score": i + j} for j in range(3)} for i in range(20)]
UUID_MAPS = [
    {f"123e4567-e89b-12d3-a456-4266141{i:05d}": i, f"223e4567-e89b-12d3-a456-4266141{i:05d}": -i} for i in range(15)
]


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


def shape(node: SchemaNode) -> str:
    """Which candidate a node is: ``closed`` (additionalProperties: false), ``map``, or ``record`` (open)."""
    additional = node.kw.get("additionalProperties")
    if additional is None:
        return "record"
    return "closed" if additional.boolean is False else "map"


class FixedScorer(MDLScorer):
    """Scores each candidate by its :func:`shape`; forces ties and wins. ``universal_bits`` stays real."""

    __slots__ = ("scores",)

    def __init__(self, **scores: Score) -> None:
        super().__init__(validator=VALIDATOR)
        self.scores = scores

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        return self.scores[shape(node)]


# Scores under which Record wins Record-or-Map and the Record closes.
CLOSING = FixedScorer(record=score(50, 50), map=score(100, 100), closed=score(1, 1))


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def analyzer(
    states: dict[str, bool] | None = None, *, scorer: MDLScorer | None = None
) -> tuple[RecordMapAnalyzer, InferenceReport]:
    report = InferenceReport()
    if scorer is None:
        return RecordMapAnalyzer.for_techniques(states, report=report, validator=VALIDATOR), report
    values = ValueInferrer.for_techniques(states, report=report, validator=VALIDATOR)
    return RecordMapAnalyzer(values, scorer=scorer), report


def engine(states: dict[str, bool] | None = None, **kwargs: Any) -> tuple[DecisionEngine, InferenceReport]:
    report = InferenceReport()
    return DecisionEngine(states, report=report, **kwargs), report


def decisions(report: InferenceReport, kind: str) -> list[Any]:
    return [entry for entry in report.decisions if entry.kind == kind]


def validates(schema: dict[str, Any], docs: list[Any]) -> bool:
    """Whether every doc validates against ``schema`` (``format`` asserted)."""
    compiled = VALIDATOR.compile(schema)
    return all(VALIDATOR.is_valid(compiled, doc) for doc in docs)


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_mdl_selection_is_implemented_and_serves_both_judgment_calls() -> None:
    spec = REGISTRY.technique(MDL)
    assert spec.implemented and spec.default_enabled
    for kind in (RECORD_OR_MAP, CLOSED_OR_OPEN_RECORD):
        assert kind in spec.serves
        model = DM_RECORD_MAP if kind == RECORD_OR_MAP else "decide.dm_closed_record"
        assert REGISTRY.active_servers(kind, REGISTRY.default_states()) == (model, MDL)  # both on by default
    assert REGISTRY.fallback(RECORD_OR_MAP)() == RECORD and REGISTRY.fallback(CLOSED_OR_OPEN_RECORD)() == OPEN
    assert REGISTRY.decision_point(RECORD_OR_MAP).tie_rule.favors == (RECORD,)


# ---------------------------------------------------------------------------
# Record-or-Map (4.1, 4.10)
# ---------------------------------------------------------------------------


def test_fixed_field_names_are_a_record_by_mdl() -> None:
    eng, report = engine()
    outcome = record_or_map(root(RECORDS), eng)
    assert outcome.is_record and outcome.keys_available and outcome.constraint_keyword == "properties"
    decision = outcome.decision
    assert decision.method == "mdl" and decision.technique == MDL
    record = outcome.provenance()
    # 4.1: the producing Technique, the Confidence, and the rejected alternative with its score.
    assert record.technique == MDL and record.cutoff.source == "MDL comparison"
    candidates = record.resolution.candidates
    assert [c.outcome for c in candidates] == [RECORD, MAP] and record.resolution.selected == 0
    assert all(c.technique is None and c.score is not None for c in candidates)
    assert candidates[0].score < candidates[1].score
    assert record.confidence == mdl_posterior([c.score for c in candidates])[0]
    assert (record.support.consistent, record.support.total) == (30, 30)
    assert record.details["judgment"] == RECORD
    assert [entry.resolution.method for entry in decisions(report, RECORD_OR_MAP)] == ["mdl"]


@pytest.mark.parametrize("docs", [USER_MAPS, UUID_MAPS], ids=["user-ids", "uuids"])
def test_data_keys_are_a_map_by_mdl(docs: list[Any]) -> None:
    eng, _ = engine()
    outcome = record_or_map(root(docs), eng)
    assert outcome.is_map and outcome.constraint_keyword == "additionalProperties"
    resolution = outcome.provenance().resolution
    assert resolution.candidates[resolution.selected].outcome == MAP
    assert resolution.candidates[1].score < resolution.candidates[0].score


def test_candidates_are_scored_on_the_same_object_rows() -> None:
    view = root(RECORDS)
    record, map_ = record_encoding(view), map_encoding(view)
    assert record.to_json() == naive_merge_node(view).to_json()
    assert set(record.kw) == {"type", "properties", "required"}
    assert map_.to_json() == {"type": ["object"], "additionalProperties": naive_merge_node(view.child_map()).to_json()}
    eng, _ = engine()
    candidates = record_or_map(view, eng).provenance().resolution.candidates
    scorer = MDLScorer(validator=VALIDATOR)
    assert [c.score for c in candidates] == [scorer.score(record, view).total, scorer.score(map_, view).total]
    assert {c.n_scored for c in candidates} == {scorer.score(record, view).n_scored}


def test_ties_go_to_record_even_with_fewer_map_model_bits() -> None:
    # Equal description lengths; the Map has fewer model bits, but 4.10 mandates Record.
    scorer = FixedScorer(record=score(5, 1), map=score(1, 5))
    eng, _ = engine()
    outcome = record_or_map(root(USER_MAPS), eng, scorer=scorer)
    assert outcome.is_record and outcome.decision.method == "mdl"
    assert outcome.decision.confidence == 0.5


def test_mdl_disabled_falls_back_to_record() -> None:
    eng, report = engine({MDL: False})
    outcome = record_or_map(root(USER_MAPS), eng)  # MDL would say Map
    assert outcome.is_record
    record = outcome.provenance()
    assert record.technique == "fallback:record_or_map"
    assert record.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")  # on by default, no model (22.14)
    assert record.resolution.method == "fallback" and record.cutoff is None
    assert record.confidence == laplace_confidence(record.support)
    assert [entry.resolution.method for entry in decisions(report, RECORD_OR_MAP)] == ["fallback"]


def test_a_decision_model_in_effect_decides_on_the_default_features() -> None:
    model = StubModel((RECORD, MAP), [0.2, 0.8])
    bundle = StubBundle({"dm.record_or_map.synthetic": model})
    eng, _ = engine({DM_RECORD_MAP: True}, bundle=bundle)
    outcome = record_or_map(root(RECORDS), eng)  # MDL would say Record
    assert outcome.is_map and outcome.decision.method == "decision-model"
    assert outcome.provenance().technique == DM_RECORD_MAP and outcome.provenance().confidence == 0.8
    (x,) = model.seen
    keys = KeyStats.of(root(RECORDS))
    assert x.shape == (1, 5) and np.all(np.isfinite(x))
    assert x[0, :4].tolist() == [30, 3, 3.0, 0.0] and keys.n_keys == 3


def test_only_object_rows_count() -> None:
    docs: list[Any] = [*RECORDS[:5], 1, "s", [1], None]
    eng, _ = engine()
    outcome = record_or_map(root(docs), eng)
    assert outcome.keys.n_objects == 5 and outcome.provenance().support.total == 5
    assert len(object_rows(root(docs))) == 5


def test_flagged_documents_are_excluded() -> None:
    store = build_store([*RECORDS[:4], {"zzz": 1}])
    store.flags.flag_documents([4])
    eng, _ = engine()
    outcome = record_or_map(View.root(store), eng)
    assert outcome.keys.n_objects == 4 and outcome.keys.n_keys == 3


def test_a_view_without_objects_is_an_error() -> None:
    eng, _ = engine()
    with pytest.raises(ValueError, match="no non-flagged object"):
        record_or_map(root([1, 2]), eng)


def test_no_keys_is_a_record_without_a_judgment_call() -> None:
    eng, report = engine()
    outcome = record_or_map(root([{}, {}, 3]), eng)
    assert outcome.is_record and outcome.decision is None and not outcome.keys_available
    assert outcome.constraint_keyword == "type"  # 4.11: the object-kind provenance goes on type
    assert outcome.details() == {"judgment": RECORD, "keys_available": False}
    assert decisions(report, RECORD_OR_MAP) == []
    with pytest.raises(ValueError, match="4.11"):
        outcome.provenance()


def test_decisions_do_not_depend_on_document_order() -> None:
    docs = [*USER_MAPS[:3], *RECORDS[:3], {"name": "solo", "rare": 1}]
    seen = set()
    for order in (docs, docs[::-1], docs[2:] + docs[:2], docs[1::2] + docs[::2]):
        eng, report = engine()
        record_or_map(root(order), eng)
        closed_or_open(root(order), eng)
        seen.add(canonical_bytes([entry.to_json() for entry in report.decisions]))
    assert len(seen) == 1


# ---------------------------------------------------------------------------
# The Good–Turing unseen-key charge
# ---------------------------------------------------------------------------


def test_unseen_key_charge_follows_the_design_formula() -> None:
    docs: list[Any] = [{"a": 1}, {"a": 2, "x": 1, "y": 2}, {"a": 3, "z": "s"}, {"a": 4}]
    view = root(docs)
    keys = KeyStats.of(view)
    assert (keys.n_objects, keys.n_keys, keys.f1, keys.f2, keys.singleton_objects) == (4, 4, 3, 0, 2)
    assert keys.good_turing == 3 / 7
    scorer = MDLScorer(validator=VALIDATOR)
    # f1 = 3 singleton keys, held by m = 2 of n = 4 objects: f1 × (CAT(n − m, m) + Σ U(o)) / m.
    expected = (3 / 2) * math.fsum([CAT((2, 2)), U(docs[1]), U(docs[2])])
    assert math.isclose(unseen_key_charge(view, scorer), expected, rel_tol=1e-12)
    # Restricted to the listed keys: only z is listed, so f1 = m = 1 and the charge is the exception cost itself.
    only_z = math.fsum([CAT((3, 1)), U(docs[2])])
    assert math.isclose(unseen_key_charge(view, scorer, ["a", "z"]), only_z, rel_tol=1e-12)
    assert unseen_key_charge(root([{"a": 1}, {"a": 2}]), scorer) == 0.0


def test_the_closed_candidate_pays_the_charge() -> None:
    docs: list[Any] = [{"a": i} for i in range(10)] + [{"a": 0, "rare": True}]
    view = root(docs)
    eng, _ = engine()
    outcome = closed_or_open(view, eng)
    scorer = MDLScorer(validator=VALIDATOR)
    closed_node = SchemaNode.from_json({**record_encoding(view).to_json(), "additionalProperties": False})
    charge = unseen_key_charge(view, scorer)
    assert charge > 0 and outcome.charge == charge
    closed, open_ = outcome.decision.resolution.candidates
    assert (closed.outcome, open_.outcome) == (CLOSED, OPEN)
    plain = scorer.score(closed_node, view)
    assert closed.model_bits == plain.model_bits and closed.n_rejected == 0
    assert closed.data_bits == math.fsum((plain.data_bits, charge))
    assert closed.exception_bits == math.fsum((plain.exception_bits, charge))
    assert open_.score == scorer.score(record_encoding(view), view).total


# ---------------------------------------------------------------------------
# Closed-or-Open-Record (4.7–4.9)
# ---------------------------------------------------------------------------


def test_a_small_record_is_open_by_mdl() -> None:
    eng, report = engine()
    outcome = closed_or_open(root(RECORDS), eng)
    assert not outcome.closed and outcome.keywords() == {}
    assert outcome.decision.method == "mdl" and outcome.decision.technique == MDL
    # 20.7: an open Record produces no Constraint, so the report entry is the decision's record.
    (entry,) = decisions(report, CLOSED_OR_OPEN_RECORD)
    assert entry.resolution.method == "mdl" and entry.cutoff.source == "MDL comparison"
    with pytest.raises(ValueError, match="open"):
        outcome.provenance()


def test_many_objects_with_stable_keys_close_the_record() -> None:
    docs = [{"a": 1, "b": 2} for _ in range(20000)]
    eng, _ = engine()
    outcome = closed_or_open(root(docs), eng)
    assert outcome.closed and outcome.charge == 0.0
    (keyword, node), = outcome.keywords().items()
    assert keyword == "additionalProperties" and node.boolean is False
    record = outcome.provenance()
    assert record.technique == MDL and record.details["judgment"] == CLOSED
    assert record.details["key_statistics"]["n_objects"] == 20000 and record.details["unseen_key_charge"] == 0
    assert (record.support.consistent, record.support.total) == (20000, 20000)


def test_ties_go_to_open_by_fewer_model_bits() -> None:
    # RECORDS hold no singleton key, so the closed candidate pays no charge and the totals tie.
    scorer = FixedScorer(closed=score(3, 1), record=score(1, 3))
    eng, _ = engine()
    outcome = closed_or_open(root(RECORDS), eng, scorer=scorer)
    assert outcome.charge == 0.0 and not outcome.closed and outcome.decision.method == "mdl"


def test_mdl_disabled_falls_back_to_open() -> None:
    eng, report = engine({MDL: False})
    outcome = closed_or_open(root(RECORDS), eng, scorer=CLOSING)  # MDL would close
    assert not outcome.closed and outcome.decision.method == "fallback"
    assert outcome.decision.technique == "fallback:closed_or_open_record"
    assert outcome.decision.fallback == FallbackInfo("decide.dm_closed_record", "missing-model")
    assert [entry.resolution.method for entry in decisions(report, CLOSED_OR_OPEN_RECORD)] == ["fallback"]


def test_the_open_record_can_be_supplied() -> None:
    view = root(RECORDS)
    eng, _ = engine()
    custom = SchemaNode.from_json({"type": "object", "properties": {"name": {}, "age": {}, "email": {}}})
    outcome = closed_or_open(view, eng, record=custom)
    closed, open_ = outcome.decision.resolution.candidates
    scorer = MDLScorer(validator=VALIDATOR)
    assert open_.score == scorer.score(custom, view).total
    closed_custom = SchemaNode.from_json({**custom.to_json(), "additionalProperties": False})  # type: ignore[dict-item]
    assert closed.score == scorer.score(closed_custom, view).total
    with pytest.raises(ValueError, match="additionalProperties"):
        closed_or_open(view, eng, record=SchemaNode.from_json({"additionalProperties": False}))


# ---------------------------------------------------------------------------
# Object nodes (4.2, 4.3, 4.4, 4.8, 16.5)
# ---------------------------------------------------------------------------


def test_record_node_lists_exactly_the_observed_keys() -> None:
    docs: list[Any] = [{"b": 1, "a": "x"}, {"a": "y", "c": None}, 7]
    ra, _ = analyzer({MDL: False})  # the fallback Record, whatever MDL says about two objects
    view = root(docs)
    seen: list[View] = []

    def child(view: View) -> SchemaNode:
        seen.append(view)
        return naive_merge_node(view)

    node = ra.object_node(view, child_schema=child)
    assert node.kind.is_record and not node.closed.closed
    assert list(node.node.kw) == ["properties"]
    assert sorted(node.node.kw["properties"]) == ["a", "b", "c"]  # 4.2
    assert [v.path[-1] for v in seen] == [key_step("a"), key_step("b"), key_step("c")]
    assert node.keywords()["properties"]["a"] == {"type": ["string"]}
    assert set(node.provenance) == {"properties"}
    assert node.constraint_provenance() == {ConstraintId(node.node.nid, "properties"): node.kind.provenance()}
    assert node.type_details() == {}
    assert validates({"type": "object", **node.keywords()}, [d for d in docs if isinstance(d, dict)])


def test_closed_record_node_emits_additional_properties_false() -> None:
    ra, _ = analyzer(scorer=CLOSING)
    node = ra.object_node(root(RECORDS))
    assert node.closed.closed
    kw = node.keywords()
    assert kw["additionalProperties"] is False and sorted(kw["properties"]) == ["age", "email", "name"]
    assert node.provenance["additionalProperties"].details["judgment"] == CLOSED
    assert validates({"type": "object", **kw}, RECORDS)
    assert not validates({"type": "object", **kw}, [{"name": "n", "extra": 1}])  # 4.8


def test_record_node_without_keys() -> None:
    ra, report = analyzer()
    node = ra.object_node(root([{}, {}]))
    assert node.kind.decision is None and node.keywords() == {} and dict(node.provenance) == {}
    assert node.type_details() == {"judgment": RECORD, "keys_available": False}  # 4.11
    assert decisions(report, RECORD_OR_MAP) == []
    assert [entry.kind for entry in report.decisions] == [CLOSED_OR_OPEN_RECORD]  # 4.7: still a Record


def test_map_node_has_one_value_schema_key_format_and_key_count_bounds() -> None:
    ra, report = analyzer()
    seen: list[View] = []

    def child(view: View) -> SchemaNode:
        seen.append(view)
        return naive_merge_node(view)

    node = ra.object_node(root(UUID_MAPS), child_schema=child)
    assert node.kind.is_map and node.closed is None
    kw = node.keywords()
    assert "properties" not in kw and "required" not in kw  # 4.3
    assert kw["additionalProperties"] == {"type": ["integer"]}
    assert len(seen) == 1 and seen[0].path[-1] == MAPVAL_STEP
    assert kw["propertyNames"] == {"format": "uuid"}  # 4.4
    keys = [key for doc in UUID_MAPS for key in doc]
    assert all(VALIDATOR.satisfies_format("uuid", key) for key in keys)
    assert kw["minProperties"] == 2 and kw["maxProperties"] == 2  # 16.5: observed key counts are all 2
    assert set(node.provenance) == {"additionalProperties", "propertyNames", "minProperties", "maxProperties"}
    assert node.provenance["additionalProperties"].details["judgment"] == MAP
    assert node.provenance["propertyNames"].details["target"] == "keys"
    assert node.key_bounds is not None and node.key_bounds.kind == "key_count"
    assert validates({"type": "object", **kw}, UUID_MAPS)
    kinds = [entry.kind for entry in report.decisions]
    assert kinds.count(RECORD_OR_MAP) == 1 and kinds.count("bound") == 2 and CLOSED_OR_OPEN_RECORD not in kinds


def test_map_node_follows_technique_states() -> None:
    ra, report = analyzer({"values.bounds": False, "values.format": False})
    node = ra.object_node(root(UUID_MAPS))
    kw = node.keywords()
    assert node.kind.is_map and set(kw) == {"additionalProperties"}  # 16.8 and no key Format candidates
    bound_methods = [entry.resolution.method for entry in decisions(report, "bound")]
    assert bound_methods == ["fallback", "fallback"]


def test_object_nodes_accept_every_non_flagged_document() -> None:
    for docs in (RECORDS, USER_MAPS, UUID_MAPS, [{"k": [1, {"z": None}]}, {"k": "s", "m": 1.5}, {}]):
        ra, _ = analyzer()
        node = ra.object_node(root(docs))
        assert validates({"type": "object", **node.keywords()}, docs)


def test_node_builders_check_the_outcome_kind() -> None:
    ra, _ = analyzer()
    record = ra.record_or_map(root(RECORDS))
    mapping = ra.record_or_map(root(USER_MAPS))
    with pytest.raises(ValueError, match="Map outcome"):
        ra.map_node(root(RECORDS), record)
    with pytest.raises(ValueError, match="Record outcome"):
        ra.record_node(root(USER_MAPS), mapping)
