"""Unit tests for core/provenance.py (task 2.5): the Provenance_Record data model and its JSON form."""

from __future__ import annotations

import copy
import json
import math
import pickle
from collections.abc import Callable
from dataclasses import FrozenInstanceError, replace
from typing import Any

import numpy as np
import pytest
from hypothesis import given

from jsinfer.core.determinism import dumps_canonical
from jsinfer.core.provenance import (
    DECISION_MODEL_COMPARISON,
    FALLBACK_REASONS,
    MDL_COMPARISON,
    REMOVED,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    ProvenanceFormatError,
    ProvenanceRecord,
    RelaxationRecord,
    Resolution,
    SampleInfo,
    Support,
    fallback_technique,
    laplace_confidence,
)
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from tests.strategies import provenance_records, relaxation_records, supports


def bound(outcome: str, score: float | None) -> CandidateScore:
    return CandidateScore("values.bounds", outcome, score)


def full_record() -> ProvenanceRecord:
    """A record with every optional part present."""
    mdl = Resolution(
        "mdl",
        1,
        (
            CandidateScore(
                "values.bounds", "maximum=omit", 40.5, model_bits=1.0, data_bits=39.5, exception_bits=0.0,
                n_rejected=0, n_scored=8,
            ),
            CandidateScore(
                "values.bounds", "maximum=100", 30.25, model_bits=8.0, data_bits=22.25, exception_bits=0.0,
                n_rejected=0, n_scored=8,
            ),
        ),
    )
    relax = Resolution("fallback", 0, (CandidateScore(None, "maximum=100", None), CandidateScore(None, "remove", None)))
    return ProvenanceRecord(
        technique="values.bounds",
        support=Support(7, 8),
        confidence=0.875,
        fallback=FallbackInfo("decide.mdl", "disabled"),
        resolution=mdl,
        cutoff=Cutoff(MDL_COMPARISON, scores=mdl.candidates),
        sample=SampleInfo(5, 8),
        changes=(RelaxationRecord({"maximum": 50}, {"maximum": 100}, (6,), resolution=relax),),
        details={"judgment": "closed", "expected": 2.0},
    )


FULL_JSON: dict[str, Any] = {
    "changes": [
        {
            "after": {"maximum": 100},
            "before": {"maximum": 50},
            "documents": [6],
            "resolution": {
                "candidates": [{"outcome": "maximum=100"}, {"outcome": "remove"}],
                "method": "fallback",
                "selected": 0,
            },
        }
    ],
    "confidence": 0.875,
    "cutoff": {
        "scores": [
            {
                "data_bits": 39.5, "exception_bits": 0.0, "model_bits": 1.0, "n_rejected": 0, "n_scored": 8,
                "outcome": "maximum=omit", "score": 40.5, "technique": "values.bounds",
            },
            {
                "data_bits": 22.25, "exception_bits": 0.0, "model_bits": 8.0, "n_rejected": 0, "n_scored": 8,
                "outcome": "maximum=100", "score": 30.25, "technique": "values.bounds",
            },
        ],
        "source": "MDL comparison",
    },
    "details": {"expected": 2, "judgment": "closed"},
    "fallback": {"reason": "disabled", "technique": "decide.mdl"},
    "resolution": {
        "candidates": [
            {
                "data_bits": 39.5, "exception_bits": 0.0, "model_bits": 1.0, "n_rejected": 0, "n_scored": 8,
                "outcome": "maximum=omit", "score": 40.5, "technique": "values.bounds",
            },
            {
                "data_bits": 22.25, "exception_bits": 0.0, "model_bits": 8.0, "n_rejected": 0, "n_scored": 8,
                "outcome": "maximum=100", "score": 30.25, "technique": "values.bounds",
            },
        ],
        "method": "mdl",
        "selected": 1,
    },
    "sample": {"sampled": 5, "total": 8},
    "support": {"consistent": 7, "ratio": 0.875, "total": 8},
    "technique": "values.bounds",
}


def written(value: Any) -> str:
    """The bytes (as text) that the canonical encoder writes for a record."""
    return dumps_canonical(value.to_json())


def strict(value: Any) -> str:
    """A spelling that tells apart what Python's == conflates (True and 1, 1 and 1.0)."""
    return json.dumps(value, ensure_ascii=False)


# ---------------------------------------------------------------------------
# Support and the default Confidence
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(("consistent", "total", "ratio"), [(3, 4, 0.75), (4, 4, 1.0), (0, 5, 0.0), (1, 3, 1 / 3)])
def test_support_ratio(consistent: int, total: int, ratio: float) -> None:
    support = Support(consistent, total)
    assert (support.consistent, support.total, support.ratio) == (consistent, total, ratio)
    assert type(support.ratio) is float


def test_support_ratio_is_one_without_observations() -> None:
    support = Support(0, 0)
    assert support.ratio == 1.0 and type(support.ratio) is float


def test_support_ratio_is_derived() -> None:
    with pytest.raises(TypeError):
        Support(1, 2, 0.5)  # type: ignore[call-arg]
    assert replace(Support(1, 2), consistent=2).ratio == 1.0
    with pytest.raises(FrozenInstanceError):
        Support(1, 2).ratio = 0.0  # type: ignore[misc]


@pytest.mark.parametrize(
    ("consistent", "total", "error"),
    [
        (5, 4, ValueError),  # consistent > total
        (1, 0, ValueError),
        (-1, 4, ValueError),
        (0, -1, ValueError),
        (1.0, 2, TypeError),
        (True, 2, TypeError),
        (np.bool_(True), 2, TypeError),
        (1, "2", TypeError),
        (None, 2, TypeError),
    ],
)
def test_support_rejects_invalid_counts(consistent: Any, total: Any, error: type[Exception]) -> None:
    with pytest.raises(error):
        Support(consistent, total)


def test_support_accepts_numpy_counts() -> None:
    support = Support(np.int64(1), np.int32(4))
    assert type(support.consistent) is int and type(support.total) is int
    assert support == Support(1, 4)


@pytest.mark.parametrize(
    ("consistent", "total", "confidence"),
    [(3, 4, 4 / 6), (0, 0, 0.5), (10, 10, 11 / 12), (0, 10, 1 / 12), (1, 1, 2 / 3)],
)
def test_laplace_confidence(consistent: int, total: int, confidence: float) -> None:
    assert laplace_confidence(Support(consistent, total)) == confidence


@given(supports())
def test_laplace_confidence_is_strictly_inside_the_unit_interval(support: Support) -> None:
    assert 0.0 < laplace_confidence(support) < 1.0


def test_laplace_record() -> None:
    support = Support(9, 10)
    record = ProvenanceRecord.laplace("values.latent_type", support, details={"n": 10})
    assert record == ProvenanceRecord("values.latent_type", support, 10 / 12, details={"n": 10})
    with pytest.raises(TypeError):
        laplace_confidence((9, 10))  # type: ignore[arg-type]


def test_fallback_technique() -> None:
    assert fallback_technique("record_or_map") == "fallback:record_or_map"
    with pytest.raises(ValueError):
        fallback_technique("")


# ---------------------------------------------------------------------------
# CandidateScore and Resolution
# ---------------------------------------------------------------------------


def test_candidate_score_normalizes_numbers() -> None:
    c = CandidateScore("decide.mdl", "map", 3, model_bits=np.float32(1.5), data_bits=-0.0, n_scored=np.int64(4))
    assert type(c.score) is float and c.score == 3.0
    assert c.model_bits == 1.5 and type(c.model_bits) is float
    assert c.data_bits == 0.0 and math.copysign(1.0, c.data_bits) == 1.0  # -0.0 becomes 0.0
    assert type(c.n_scored) is int
    # A generic score may be negative (for example a BIC value in a Cutoff).
    assert CandidateScore(None, "k=2", -12.5).score == -12.5


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"score": math.nan}, ValueError),
        ({"score": math.inf}, ValueError),
        ({"score": 2**2000}, ValueError),
        ({"score": True}, TypeError),
        ({"score": "1"}, TypeError),
        ({"model_bits": -1.0}, ValueError),
        ({"data_bits": math.inf}, ValueError),
        ({"exception_bits": -0.5}, ValueError),
        ({"n_rejected": -1}, ValueError),
        ({"n_scored": 1.0}, TypeError),
        ({"n_rejected": 5, "n_scored": 4}, ValueError),
        ({"technique": ""}, ValueError),
        ({"technique": "fallback:bound"}, ValueError),
        ({"technique": 3}, TypeError),
        ({"outcome": 1}, TypeError),
    ],
)
def test_candidate_score_rejects_invalid_fields(kwargs: dict[str, Any], error: type[Exception]) -> None:
    fields = {"technique": "decide.mdl", "outcome": "record", "score": 1.0, **kwargs}
    with pytest.raises(error):
        CandidateScore(**fields)


@pytest.mark.parametrize(
    ("method", "scores"),
    [("mdl", [12.5, 0.0]), ("decision-model", [0.25, 1.0]), ("priority", [None, None]), ("fallback", [None])],
)
def test_resolution_scores_match_the_method(method: str, scores: list[float | None]) -> None:
    candidates = [bound(f"c{i}", score) for i, score in enumerate(scores)]
    resolution = Resolution(method, len(candidates) - 1, candidates)  # type: ignore[arg-type]
    assert resolution.candidates == tuple(candidates)
    assert resolution.selected_candidate is candidates[-1]


def test_fallback_resolution_lists_the_fallback_outcome_without_a_technique() -> None:
    resolution = Resolution("fallback", 0, (CandidateScore(None, "record", None),))
    assert resolution.selected_candidate.technique is None


@pytest.mark.parametrize(
    ("method", "selected", "candidates", "error"),
    [
        ("argmin", 0, [bound("a", 1.0)], ValueError),
        ("mdl", 2, [bound("a", 1.0), bound("b", 2.0)], ValueError),
        ("mdl", -1, [bound("a", 1.0)], ValueError),
        ("mdl", True, [bound("a", 1.0), bound("b", 2.0)], TypeError),
        ("mdl", 0, [], ValueError),
        ("mdl", 0, [("a", 1.0)], TypeError),
        ("mdl", 0, bound("a", 1.0), TypeError),
        ("mdl", 0, [bound("a", 1.0), bound("b", None)], ValueError),
        ("mdl", 0, [bound("a", -1.0)], ValueError),
        ("decision-model", 0, [bound("a", 1.5)], ValueError),
        ("decision-model", 0, [bound("a", None)], ValueError),
        ("priority", 0, [bound("a", 1.0)], ValueError),
        ("fallback", 0, [CandidateScore(None, "record", 0.5)], ValueError),
    ],
)
def test_resolution_rejects_invalid_fields(
    method: Any, selected: Any, candidates: Any, error: type[Exception]
) -> None:
    with pytest.raises(error):
        Resolution(method, selected, candidates)


# ---------------------------------------------------------------------------
# FallbackInfo, Cutoff, SampleInfo
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("reason", FALLBACK_REASONS)
def test_fallback_info_reasons(reason: str) -> None:
    info = FallbackInfo("values.bounds", reason)  # type: ignore[arg-type]
    assert info.to_json() == {"reason": reason, "technique": "values.bounds"}
    assert FallbackInfo.from_json(info.to_json()) == info
    record = ProvenanceRecord("fallback:bound", Support(0, 3), 0.2, fallback=info)
    assert ProvenanceRecord.from_json(json.loads(written(record))) == record


@pytest.mark.parametrize(
    ("technique", "reason", "error"),
    [
        ("values.bounds", "off", ValueError),
        ("values.bounds", None, ValueError),
        ("", "disabled", ValueError),
        ("fallback:bound", "disabled", ValueError),
        (None, "disabled", TypeError),
    ],
)
def test_fallback_info_rejects_invalid_fields(technique: Any, reason: Any, error: type[Exception]) -> None:
    with pytest.raises(error):
        FallbackInfo(technique, reason)


def test_cutoff_records_a_value_or_scores() -> None:
    threshold = Cutoff("dbscan.min_samples", value=5)
    assert threshold.value == 5.0 and type(threshold.value) is float and threshold.scores == ()
    bic = Cutoff("BIC", scores=[CandidateScore("variants.bernoulli_mixture", "k=1", -3.5)])
    assert bic.value is None and isinstance(bic.scores, tuple)
    assert Cutoff(MDL_COMPARISON, scores=(bound("a", 1.0), bound("b", 2.0))).source == "MDL comparison"
    assert threshold.artifact is None and "artifact" not in threshold.to_json()


def test_a_decision_model_comparison_names_its_artifact() -> None:
    probabilities = (bound("record", 0.25), bound("map", 0.75))
    cutoff = Cutoff(DECISION_MODEL_COMPARISON, scores=probabilities, artifact="dm.record_or_map.weak")
    assert cutoff.to_json() == {
        "artifact": "dm.record_or_map.weak",
        "scores": [p.to_json() for p in probabilities],
        "source": "Decision_Model comparison",
    }
    assert Cutoff.from_json(cutoff.to_json()) == cutoff


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"source": "BIC"}, ValueError),  # neither
        ({"source": "BIC", "value": 1.0, "scores": (bound("a", 1.0),)}, ValueError),  # both
        ({"source": "", "value": 1.0}, ValueError),
        ({"source": "x", "value": math.nan}, ValueError),
        ({"source": "x", "value": False}, TypeError),
        ({"source": "x", "scores": (1.0,)}, TypeError),
        # A Decision_Model comparison needs its artifact and probabilities; no other source names an artifact.
        ({"source": DECISION_MODEL_COMPARISON, "scores": (bound("a", 0.5),)}, ValueError),
        ({"source": DECISION_MODEL_COMPARISON, "value": 0.5, "artifact": "dm.x.synthetic"}, ValueError),
        ({"source": DECISION_MODEL_COMPARISON, "scores": (bound("a", 1.5),), "artifact": "dm.x.synthetic"}, ValueError),
        ({"source": DECISION_MODEL_COMPARISON, "scores": (bound("a", None),), "artifact": "dm.x.synthetic"}, ValueError),
        ({"source": DECISION_MODEL_COMPARISON, "scores": (bound("a", 0.5),), "artifact": ""}, ValueError),
        ({"source": DECISION_MODEL_COMPARISON, "scores": (bound("a", 0.5),), "artifact": 3}, TypeError),
        ({"source": MDL_COMPARISON, "scores": (bound("a", 1.0),), "artifact": "dm.x.synthetic"}, ValueError),
    ],
)
def test_cutoff_rejects_invalid_fields(kwargs: dict[str, Any], error: type[Exception]) -> None:
    with pytest.raises(error):
        Cutoff(**kwargs)


def test_sample_info() -> None:
    assert SampleInfo(5000, 100_000).to_json() == {"sampled": 5000, "total": 100_000}
    assert SampleInfo(3, 3).sampled == 3
    for sampled, total in [(0, 5), (6, 5), (1, 0), (1.0, 2)]:
        with pytest.raises((TypeError, ValueError)):
            SampleInfo(sampled, total)


# ---------------------------------------------------------------------------
# RelaxationRecord
# ---------------------------------------------------------------------------


def test_relaxation_documents_are_sorted_without_repeats() -> None:
    assert RelaxationRecord({"type": "integer"}, {"type": "number"}, [5, 1, 5]).documents == (1, 5)
    assert RelaxationRecord({"type": "integer"}, REMOVED, np.array([3, 2])).documents == (2, 3)


def test_relaxation_removal_is_distinct_from_null() -> None:
    to_null = RelaxationRecord({"const": 1}, {"const": None}, [0])
    to_json_null = RelaxationRecord({"const": 1}, None, [0])
    removed = RelaxationRecord({"const": None}, REMOVED, [0])
    assert not to_null.removed and not to_json_null.removed and removed.removed
    assert to_json_null.to_json() == {"after": None, "before": {"const": 1}, "documents": [0]}
    assert removed.to_json() == {"before": {"const": None}, "documents": [0], "removed": True}
    assert to_json_null != removed
    assert RelaxationRecord.from_json(json.loads(written(to_json_null))).after is None
    assert RelaxationRecord.from_json(json.loads(written(removed))).after is REMOVED
    assert pickle.loads(pickle.dumps(REMOVED)) is REMOVED


def test_relaxation_payloads_are_canonical() -> None:
    record = RelaxationRecord({"maximum": 10.0, "a": (1, 2.5)}, {"maximum": 100}, [4])
    assert strict(record.before) == strict({"a": [1, 2.5], "maximum": 10})
    assert record == RelaxationRecord({"a": [1, 2.5], "maximum": 10}, {"maximum": 100.0}, (4,))


def test_relaxation_equality_is_json_equality() -> None:
    assert RelaxationRecord({"const": True}, REMOVED, [0]) != RelaxationRecord({"const": 1}, REMOVED, [0])
    with pytest.raises(TypeError):
        hash(RelaxationRecord({"const": 1}, REMOVED, [0]))


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"documents": []}, ValueError),
        ({"documents": [-1]}, ValueError),
        ({"documents": [True]}, TypeError),
        ({"documents": "12"}, TypeError),
        ({"documents": 1}, TypeError),
        ({"before": {1, 2}}, TypeError),
        ({"before": {1: "a"}}, TypeError),
        ({"after": math.nan}, ValueError),
        ({"after": {"maximum": math.inf}}, ValueError),
        ({"resolution": "mdl"}, TypeError),
    ],
)
def test_relaxation_rejects_invalid_fields(kwargs: dict[str, Any], error: type[Exception]) -> None:
    fields = {"before": {"maximum": 10}, "after": {"maximum": 20}, "documents": [3], **kwargs}
    with pytest.raises(error):
        RelaxationRecord(**fields)


# ---------------------------------------------------------------------------
# ProvenanceRecord
# ---------------------------------------------------------------------------


def test_minimal_record() -> None:
    record = ProvenanceRecord("values.latent_type", Support(3, 4), 1)
    assert record.confidence == 1.0 and type(record.confidence) is float
    assert (record.fallback, record.resolution, record.cutoff, record.sample) == (None, None, None, None)
    assert (record.changes, record.details) == ((), {})
    assert record.to_json() == {
        "confidence": 1.0,
        "support": {"consistent": 3, "ratio": 0.75, "total": 4},
        "technique": "values.latent_type",
    }
    assert written(record) == (
        '{\n  "confidence": 1.0,\n  "support": {\n    "consistent": 3,\n    "ratio": 0.75,\n    "total": 4\n  },\n'
        '  "technique": "values.latent_type"\n}\n'
    )


@pytest.mark.parametrize(
    ("confidence", "error"),
    [(1.5, ValueError), (-0.1, ValueError), (math.nan, ValueError), (True, TypeError), ("0.5", TypeError)],
)
def test_record_rejects_invalid_confidence(confidence: Any, error: type[Exception]) -> None:
    with pytest.raises(error):
        ProvenanceRecord("values.latent_type", Support(1, 1), confidence)


def test_fallback_produced_constraints_record_fallback_information() -> None:
    support = Support(0, 0)
    info = FallbackInfo("values.enum_estimation", "disabled")
    with pytest.raises(ValueError, match="fallback information"):
        ProvenanceRecord(fallback_technique("closed_or_open_enum"), support, 0.5)
    with pytest.raises(ValueError, match="Decision_Point kind"):
        ProvenanceRecord("fallback:", support, 0.5, fallback=info)
    fallback_resolution = Resolution("fallback", 0, (CandidateScore(None, "open", None),))
    with pytest.raises(ValueError, match="fallback information"):
        ProvenanceRecord("values.enum_estimation", support, 0.5, resolution=fallback_resolution)
    record = ProvenanceRecord(
        "fallback:closed_or_open_enum", support, 0.5, fallback=info, resolution=fallback_resolution
    )
    assert record.fallback == info
    # 19.7: the priority-order Fallback_Behavior keeps the producing Technique.
    priority = Resolution("priority", 0, (bound("a", None), bound("b", None)))
    mdl_off = FallbackInfo("decide.mdl", "disabled")
    ProvenanceRecord("values.bounds", support, 0.5, fallback=mdl_off, resolution=priority)


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"technique": ""}, ValueError),
        ({"technique": None}, TypeError),
        ({"support": (1, 2)}, TypeError),
        ({"fallback": "disabled"}, TypeError),
        ({"resolution": {"method": "mdl"}}, TypeError),
        ({"cutoff": 0.5}, TypeError),
        ({"sample": (1, 2)}, TypeError),
        ({"changes": RelaxationRecord({"type": "string"}, REMOVED, [0])}, TypeError),
        ({"changes": [{"before": 1}]}, TypeError),
        ({"details": [("a", 1)]}, TypeError),
        ({"details": {1: "a"}}, TypeError),
        ({"details": {"a": {1, 2}}}, TypeError),
        ({"details": {"a": [math.nan]}}, ValueError),
    ],
)
def test_record_rejects_invalid_fields(kwargs: dict[str, Any], error: type[Exception]) -> None:
    fields = {"technique": "values.bounds", "support": Support(1, 2), "confidence": 0.5, **kwargs}
    with pytest.raises(error):
        ProvenanceRecord(**fields)


def test_record_details_are_canonical_copies() -> None:
    source = {"b": [1.0, (2, 3)], "a": {"y": 1, "x": True}}
    record = ProvenanceRecord("values.enum_estimation", Support(2, 2), 0.75, details=source)
    source["b"].append(4)
    assert list(record.details) == ["a", "b"]
    assert strict(record.details) == strict({"a": {"x": True, "y": 1}, "b": [1, [2, 3]]})
    record.to_json()["details"]["a"]["x"] = False  # the JSON form is a copy
    assert record.details["a"]["x"] is True


def test_record_equality_is_json_equality() -> None:
    def with_details(**details: Any) -> ProvenanceRecord:
        return ProvenanceRecord("t", Support(1, 1), 0.5, details=details)

    assert with_details(x=1.0) == with_details(x=1)
    assert with_details(x=True) != with_details(x=1)
    assert with_details(x=[1, 2]) != with_details(x=[2, 1])
    assert ProvenanceRecord("t", Support(1, 1), 0.5) != ProvenanceRecord("t", Support(1, 1), 0.25)
    assert full_record() == full_record()
    assert full_record() != replace(full_record(), sample=None)
    with pytest.raises(TypeError):
        hash(full_record())
    with pytest.raises(FrozenInstanceError):
        full_record().confidence = 0.0  # type: ignore[misc]


def test_record_pickles_and_copies() -> None:
    record = full_record()
    assert pickle.loads(pickle.dumps(record)) == record
    assert copy.deepcopy(record) == record
    restored = pickle.loads(pickle.dumps(record))
    assert written(restored) == written(record)


def test_schema_model_holds_records() -> None:
    root = SchemaNode.from_json({"type": "integer", "maximum": 100})
    record = full_record()
    old_nid = root.nid
    model = SchemaModel(root, provenance={ConstraintId(old_nid, "maximum"): record})
    mapping = model.renumber()
    restored = pickle.loads(pickle.dumps(model))
    assert restored.provenance == {ConstraintId(mapping[old_nid], "maximum"): record}


# ---------------------------------------------------------------------------
# JSON form
# ---------------------------------------------------------------------------


def test_full_record_json() -> None:
    assert strict(full_record().to_json()) == strict(FULL_JSON)
    assert ProvenanceRecord.from_json(copy.deepcopy(FULL_JSON)) == full_record()


EXAMPLES: list[Any] = [
    Support(0, 0),
    Support(2, 3),
    CandidateScore(None, "record", None),
    CandidateScore("decide.mdl", "map", 12.5, model_bits=2, data_bits=10.5, exception_bits=1, n_rejected=1, n_scored=9),
    Resolution("decision-model", 0, (bound("record", 0.8), bound("map", 0.2))),
    Resolution("priority", 1, (bound("a", None), bound("b", None))),
    FallbackInfo("values.bounds", "missing-threshold"),
    Cutoff("lsh.keyset_jaccard", value=0.3),
    Cutoff("BIC", scores=(CandidateScore("variants.bernoulli_mixture", "k=2", -1.5),)),
    Cutoff(DECISION_MODEL_COMPARISON, scores=(bound("tuple", 0.4), bound("list", 0.6)), artifact="dm.tuple_or_list.synthetic"),
    SampleInfo(20_000, 100_000),
    RelaxationRecord(
        {"enum": [1, 2]}, {"enum": [1, 2, 3]}, (4, 9), resolution=Resolution("mdl", 0, (bound("add", 3.5),))
    ),
    RelaxationRecord({"required": ["a"]}, REMOVED, (0,)),
    ProvenanceRecord("values.latent_type", Support(0, 0), 0.5),
    full_record(),
]


@pytest.mark.parametrize("value", EXAMPLES, ids=lambda value: type(value).__name__)
def test_round_trip_of_every_record_type(value: Any) -> None:
    text = written(value)
    loaded = type(value).from_json(json.loads(text))
    assert loaded == value
    assert written(loaded) == text


@given(provenance_records())
def test_record_round_trip_is_exact(record: ProvenanceRecord) -> None:
    """Writing, reading, and writing again gives an equal record and identical bytes (unit-level round trip)."""
    text = written(record)
    loaded = ProvenanceRecord.from_json(json.loads(text))
    assert loaded == record
    assert written(loaded) == text
    # Keys are in code-point order at every level, so a sorting encoder writes the same bytes.
    assert json.dumps(json.loads(text), indent=2, ensure_ascii=False, sort_keys=True) + "\n" == text


@given(relaxation_records())
def test_relaxation_round_trip_is_exact(record: RelaxationRecord) -> None:
    text = written(record)
    loaded = RelaxationRecord.from_json(json.loads(text))
    assert loaded == record and written(loaded) == text


def test_from_json_accepts_other_number_spellings_and_key_orders() -> None:
    value = {"technique": "t", "support": {"total": 0, "ratio": 1, "consistent": 0}, "confidence": 1}
    record = ProvenanceRecord.from_json(value)
    assert record == ProvenanceRecord("t", Support(0, 0), 1.0)
    assert '"confidence": 1.0' in written(record) and '"ratio": 1.0' in written(record)


# ---------------------------------------------------------------------------
# Malformed JSON
# ---------------------------------------------------------------------------


def _set_path(*path: str | int, value: Any) -> Callable[[Any], Any]:
    def mutate(doc: Any) -> Any:
        target = doc
        for step in path[:-1]:
            target = target[step]
        target[path[-1]] = value
        return doc

    return mutate


def _delete(*path: str | int) -> Callable[[Any], Any]:
    def mutate(doc: Any) -> Any:
        target = doc
        for step in path[:-1]:
            target = target[step]
        del target[path[-1]]
        return doc

    return mutate


MALFORMED: list[tuple[str, Callable[[Any], Any], str, str]] = [
    ("not an object", lambda doc: [doc], "", "expected an object, got an array"),
    ("missing technique", _delete("technique"), "", "missing member 'technique'"),
    ("unknown member", _set_path("x-extra", value=1), "", "unknown member 'x-extra'"),
    ("technique type", _set_path("technique", value=1), "/technique", "expected a string"),
    ("confidence string", _set_path("confidence", value="0.5"), "/confidence", "expected a number, got a string"),
    ("confidence bool", _set_path("confidence", value=True), "/confidence", "expected a number, got a boolean"),
    ("confidence NaN", _set_path("confidence", value=math.nan), "/confidence", "expected a finite number"),
    ("confidence range", _set_path("confidence", value=1.5), "", "[0, 1]"),
    ("ratio mismatch", _set_path("support", "ratio", value=0.5), "/support/ratio", "does not equal"),
    ("consistent > total", _set_path("support", "consistent", value=9), "/support", "must not exceed"),
    ("total float", _set_path("support", "total", value=8.0), "/support/total", "expected an integer"),
    ("support missing ratio", _delete("support", "ratio"), "/support", "missing member 'ratio'"),
    ("null fallback", _set_path("fallback", value=None), "/fallback", "expected an object, got null"),
    ("fallback reason", _set_path("fallback", "reason", value="off"), "/fallback", "reason must be one of"),
    ("selected range", _set_path("resolution", "selected", value=9), "/resolution", "out of range"),
    ("method", _set_path("resolution", "method", value="vote"), "/resolution", "method must be one of"),
    (
        "candidate score",
        _set_path("resolution", "candidates", 1, "score", value="x"),
        "/resolution/candidates/1/score",
        "expected a number",
    ),
    (
        "null score",
        _set_path("resolution", "candidates", 0, "score", value=None),
        "/resolution/candidates/0/score",
        "got null",
    ),
    (
        "candidate member",
        _set_path("resolution", "candidates", 0, "bits", value=1),
        "/resolution/candidates/0",
        "unknown member 'bits'",
    ),
    ("candidates type", _set_path("resolution", "candidates", value={}), "/resolution/candidates", "expected an array"),
    ("cutoff both", _set_path("cutoff", "value", value=0.5), "/cutoff", "exactly one"),
    ("cutoff empty scores", _set_path("cutoff", "scores", value=[]), "/cutoff/scores", "non-empty array"),
    ("cutoff artifact type", _set_path("cutoff", "artifact", value=1), "/cutoff/artifact", "expected a string"),
    ("cutoff stray artifact", _set_path("cutoff", "artifact", value="dm.x.weak"), "/cutoff", "names an artifact"),
    ("sample zero", _set_path("sample", "sampled", value=0), "/sample", "must be >= 1"),
    ("empty changes", _set_path("changes", value=[]), "/changes", "non-empty array"),
    ("documents order", _set_path("changes", 0, "documents", value=[6, 2]), "/changes/0/documents", "ascending"),
    ("documents repeat", _set_path("changes", 0, "documents", value=[6, 6]), "/changes/0/documents", "ascending"),
    ("documents empty", _set_path("changes", 0, "documents", value=[]), "/changes/0", "at least one"),
    ("document type", _set_path("changes", 0, "documents", 0, value="6"), "/changes/0/documents/0", "integer"),
    ("after and removed", _set_path("changes", 0, "removed", value=True), "/changes/0", "no 'after' member"),
    ("removed false", _set_path("changes", 0, "removed", value=False), "/changes/0/removed", "expected true"),
    ("no after", _delete("changes", 0, "after"), "/changes/0", "missing member 'after'"),
    (
        "change resolution",
        _set_path("changes", 0, "resolution", "candidates", 0, "score", value=1.0),
        "/changes/0/resolution",
        "has no scores",
    ),
    ("empty details", _set_path("details", value={}), "/details", "non-empty object"),
    ("details NaN", _set_path("details", "expected", value=math.inf), "", "JSON value"),
    (
        "marker without fallback",
        lambda doc: _delete("fallback")(_set_path("technique", value="fallback:bound")(doc)),
        "",
        "fallback information",
    ),
]


@pytest.mark.parametrize(("mutate", "location", "message"), [m[1:] for m in MALFORMED], ids=[m[0] for m in MALFORMED])
def test_from_json_rejects_malformed_input(mutate: Callable[[Any], Any], location: str, message: str) -> None:
    with pytest.raises(ProvenanceFormatError) as info:
        ProvenanceRecord.from_json(mutate(copy.deepcopy(FULL_JSON)))
    assert info.value.location == location
    assert message in info.value.reason


def test_format_error_names_the_location_in_the_enclosing_document() -> None:
    bad = _set_path("support", "total", value="8")(copy.deepcopy(FULL_JSON))
    with pytest.raises(ProvenanceFormatError) as info:
        ProvenanceRecord.from_json(bad, location="/records/~1maximum")
    error = info.value
    assert error.location == "/records/~1maximum/support/total"
    assert isinstance(error, ValueError)
    assert str(error) == (
        "invalid provenance JSON at '/records/~1maximum/support/total': expected an integer, got a string"
    )
    restored = pickle.loads(pickle.dumps(error))
    assert (restored.location, restored.reason, str(restored)) == (error.location, error.reason, str(error))
    with pytest.raises(ProvenanceFormatError, match="at the top level"):
        Support.from_json(None)
