"""Unit tests for values/bounds.py (task 12.4).

Requirements 16.1, 16.2, 16.3, 16.6, 16.7, 16.8. Forced ties use a stub
scorer; MDL scores are checked against the MDL_Scorer on real views.
"""

from __future__ import annotations

import itertools
import math
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import FallbackInfo, Support
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.stats import Histogram
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine, mdl_posterior
from jsinfer.io.validator import Validator
from jsinfer.mdl.codes import INTEGER_LIMITS
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.values.bounds import (
    BOUNDS,
    INTEGER_LIMIT_RUNGS,
    QuantitySet,
    Rung,
    bound_ladder,
    decide_bound,
    mdl_bound_score,
    round_outward,
)

ON = {BOUNDS: True}
OFF = {BOUNDS: False}


def engine(states: dict[str, bool], report: InferenceReport | None = None) -> DecisionEngine:
    return DecisionEngine(states, report=report)


def tied(q: QuantitySet, node: SchemaNode, low: Any, high: Any) -> Score:
    """A scorer that gives every candidate the same description length."""
    return Score(1.0, 1.0, 0.0, 0, q.total)


class Recorder:
    """Wraps mdl_bound_score and records each call as (schema, low, high)."""

    def __init__(self) -> None:
        self.calls: list[tuple[Any, Any, Any]] = []

    def __call__(self, q: QuantitySet, node: SchemaNode, low: Any, high: Any) -> Score:
        self.calls.append((node.to_json(), low, high))
        return mdl_bound_score(q, node, low, high)


def ladder(kind: str, values: list[Any], side: str) -> list[tuple[str, Any]]:
    return [(rung.name, rung.value) for rung in bound_ladder(QuantitySet.of(kind, values), side)]  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Outward rounding
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("value", "digits", "side", "expected"),
    [
        (137, 1, "maximum", 200),
        (137, 2, "maximum", 140),
        (137, 1, "minimum", 100),
        (137, 2, "minimum", 130),
        (-137, 1, "minimum", -200),
        (-137, 2, "minimum", -140),
        (-137, 1, "maximum", -100),
        (999, 1, "maximum", 1000),
        (0.137, 1, "maximum", 0.2),
        (0.137, 2, "minimum", 0.13),
        (0.95, 1, "maximum", 1),
        (5, 1, "maximum", 5),
        (0, 2, "minimum", 0),
        (-0.0, 1, "maximum", 0),
        (3.0, 1, "maximum", 3),
        (2**70 + 1, 1, "maximum", 2 * 10**21),
        (2**70 + 1, 2, "minimum", 11 * 10**20),
        # The double 1e300 is slightly above 10**300, so outward rounding of its exact value goes up.
        (1e300, 1, "maximum", 2 * 10**300),
        (1e300, 1, "minimum", 10**300),
    ],
)
def test_round_outward(value: Any, digits: int, side: str, expected: Any) -> None:
    rounded = round_outward(value, digits, side)  # type: ignore[arg-type]
    assert rounded == expected and type(rounded) is type(expected)
    assert (rounded <= value) if side == "minimum" else (rounded >= value)


def test_round_outward_rejects_bad_arguments() -> None:
    with pytest.raises(ValueError):
        round_outward(5, 0, "maximum")
    with pytest.raises(ValueError):
        round_outward(5, 1, "upper")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        round_outward(True, 1, "maximum")


# ---------------------------------------------------------------------------
# The ladder (16.2)
# ---------------------------------------------------------------------------


def test_ladder_follows_the_documented_order_and_drops_violated_and_repeated_candidates() -> None:
    # 0 and the int8 maximum 127 are below the observed 137, so they are violated.
    assert ladder("number", [3, 137], "maximum") == [
        ("omit", None), ("observed", 137), ("1-digit", 200), ("2-digits", 140),
        ("int16", 2**15 - 1), ("int32", 2**31 - 1), ("int64", 2**63 - 1),
        ("uint8", 255), ("uint16", 2**16 - 1), ("uint32", 2**32 - 1), ("uint64", 2**64 - 1),
    ]
    # Both roundings of 3 are 3, and every unsigned minimum is 0, which zero already proposed.
    assert ladder("number", [3, 137], "minimum") == [
        ("omit", None), ("observed", 3), ("zero", 0),
        ("int8", -128), ("int16", -(2**15)), ("int32", -(2**31)), ("int64", -(2**63)),
    ]


def test_integer_limits_need_integer_quantities() -> None:
    assert ladder("number", [0.137, 2.5], "maximum") == [("omit", None), ("observed", 2.5), ("1-digit", 3)]
    assert ladder("number", [0.137, 2.5], "minimum") == [
        ("omit", None), ("observed", 0.137), ("zero", 0), ("1-digit", 0.1), ("2-digits", 0.13),
    ]
    # Integral floats are integers (13.2), and integral candidates are ints.
    assert ladder("number", [2.0, 7.0], "maximum")[:2] == [("omit", None), ("observed", 7)]
    assert ("uint8", 255) in ladder("number", [2.0, 7.0], "maximum")


@pytest.mark.parametrize("kind", ["string_length", "item_count", "key_count"])
def test_lengths_and_counts_get_no_negative_candidates(kind: str) -> None:
    assert ladder(kind, [7, 12], "minimum") == [("omit", None), ("observed", 7), ("zero", 0)]
    assert all(value is None or value >= 0 for _, value in ladder(kind, [0, 5], "minimum"))
    # Numbers keep them.
    assert ("int8", -128) in ladder("number", [7, 12], "minimum")


def test_integer_limit_rungs_are_the_bound_codes_limits() -> None:
    proposed = {value for rungs in INTEGER_LIMIT_RUNGS.values() for _, value in rungs}
    assert sorted(proposed) == sorted(set(INTEGER_LIMITS))


@pytest.mark.parametrize(
    "values",
    [
        [3, 137],
        [-1000, -999, 5],
        [0.1, 0.3, 2.675],
        [-0.007, 1e-300, 5e-324],
        [2**64 + 7, -(2**70)],
        [1e300, 1.7976931348623157e308],
        [-1.7976931348623157e308, 0],
        [42],
    ],
)
def test_every_ladder_candidate_satisfies_every_quantity(values: list[Any]) -> None:
    q = QuantitySet.of("number", values)
    for side in ("minimum", "maximum"):
        rungs = bound_ladder(q, side)  # type: ignore[arg-type]
        assert rungs[0] == Rung("omit", None)
        bounds = [rung.value for rung in rungs[1:]]
        assert len({canonical_bytes(b) for b in bounds}) == len(bounds)  # distinct
        for bound in bounds:
            assert isinstance(bound, int) or not float(bound).is_integer()  # integral values are ints
            assert all((bound <= v) if side == "minimum" else (bound >= v) for v in values)


# ---------------------------------------------------------------------------
# Selection (16.1, 16.2, 16.7)
# ---------------------------------------------------------------------------


def test_bounds_are_selected_by_description_length() -> None:
    report = InferenceReport()
    decision = decide_bound(list(range(101)) * 3, "number", engine(ON, report), path="$.n")
    assert decision.keywords() == {"minimum": 0, "maximum": 100}
    assert decision.skipped is None
    for side in decision.sides:
        d = side.decision
        assert (d.method, d.technique, d.fallback) == ("mdl", BOUNDS, None)
        listed = d.resolution.candidates
        assert listed[0].outcome == f"{side.keyword}=omit"
        assert all(c.technique == BOUNDS for c in listed)
        scores = [c.score for c in listed]
        assert listed[d.resolution.selected].score == min(scores)
        assert d.confidence == mdl_posterior(scores)[d.resolution.selected]
    assert [c.outcome for c in decision.maximum.decision.resolution.candidates][:3] == [
        "maximum=omit", "maximum=100", "maximum=127",
    ]
    assert [(entry.kind, entry.path) for entry in report.decisions] == [("bound", "$.n"), ("bound", "$.n")]


def test_few_observations_do_not_pay_for_a_bound() -> None:
    assert decide_bound([0.25, 1.5, 3.75], "number", engine(ON)).keywords() == {}


def test_the_maximum_is_decided_given_the_minimum() -> None:
    recorder = Recorder()
    decision = decide_bound(list(range(101)) * 3, "number", engine(ON), scorer=recorder)
    chosen = decision.minimum.value
    assert chosen == 0
    n_minimum = len(bound_ladder(decision.quantities, "minimum"))
    first, second = recorder.calls[:n_minimum], recorder.calls[n_minimum:]
    assert first and second
    # The minimum is decided with the maximum omitted.
    assert all(high is None for _, _, high in first)
    # The maximum is decided with the chosen minimum in the node and the data code.
    assert all(schema.get("minimum") == chosen and low == chosen for schema, low, _ in second)
    assert [high for _, _, high in second] == [rung.value for rung in bound_ladder(decision.quantities, "maximum")]


@pytest.mark.parametrize(
    ("kind", "values"),
    [("number", [3, 137, 50]), ("number", [0.137, 2.5]), ("number", [-2**70, 2**64]), ("item_count", [1, 9])],
)
def test_ties_go_to_the_candidate_nearest_the_observed_extreme(kind: str, values: list[Any]) -> None:
    decision = decide_bound(values, kind, engine(ON), scorer=tied)  # type: ignore[arg-type]
    q = decision.quantities
    assert [side.value for side in decision.sides] == [q.low, q.high]
    for side in decision.sides:
        assert side.decision.method == "mdl" and side.value == side.extreme


def test_omission_is_the_farthest_candidate_in_a_tie() -> None:
    # Only omission and the observed extremes tie; omission still loses.
    def extremes_tie(q: QuantitySet, node: SchemaNode, low: Any, high: Any) -> Score:
        bounds = [node.kw[keyword] for keyword in ("minimum", "maximum") if keyword in node.kw]
        return Score(1.0, 1.0 if all(b in (q.low, q.high) for b in bounds) else 50.0, 0.0, 0, q.total)

    decision = decide_bound([4, 9], "number", engine(ON), scorer=extremes_tie)
    assert decision.keywords() == {"minimum": 4, "maximum": 9}
    assert all(len(side.decision.resolution.candidates) > 2 for side in decision.sides)


def test_lengths_counts_and_integral_floats_emit_ints() -> None:
    for kind in ("string_length", "item_count", "key_count"):
        decision = decide_bound([2, 3, 5, 8] * 50, kind, engine(ON), scorer=tied)  # type: ignore[arg-type]
        assert all(type(value) is int for value in decision.keywords().values())
    decision = decide_bound([1.0, 1e3], "number", engine(ON), scorer=tied)
    assert decision.keywords() == {"minimum": 1, "maximum": 1000}
    assert all(type(value) is int for value in decision.keywords().values())


def test_decisions_do_not_depend_on_order_or_multiplicity_layout() -> None:
    def entries(values: Any, counts: Any = None) -> list[bytes]:
        report = InferenceReport()
        decide_bound(values, "number", engine(ON, report), counts=counts, path="$.x")
        return [canonical_bytes(entry.to_json()) for entry in report.decisions]

    expected = entries([1, 1.0, 3, 5, 5, 5, 0.5] * 20)
    assert entries([5, 0.5, 3, 1] , counts=[60, 20, 20, 40]) == expected
    assert entries(np.array([5.0, 0.5, 3.0, 1.0]), counts=np.array([60, 20, 20, 40])) == expected


def test_histograms_are_taken_with_their_counts() -> None:
    store = build_store(["a", "😀😀", "abc", "abc", "", "ab"] * 40)
    stats = View.root(store).stats()
    from_stats = decide_bound(stats.string_lengths, "string_length", engine(ON))
    from_list = decide_bound([len(s) for s in ["a", "😀😀", "abc", "abc", "", "ab"]] * 40, "string_length", engine(ON))
    assert from_stats.quantities.values == (0, 1, 2, 3)  # code points: the emoji pair has length 2
    assert from_stats.keywords() == from_list.keywords()
    with pytest.raises(ValueError, match="own counts"):
        decide_bound(stats.string_lengths, "string_length", engine(ON), counts=[1, 1, 1, 1])


def test_emitted_bounds_are_sound() -> None:
    # 16.6: every observed value satisfies the emitted bounds, checked by the Validator.
    validator = Validator()
    numbers = [-17, 0.5, 3, 250, 1e-3, 99.25] * 30 + [2**64 + 1]
    strings = ["", "x", "😀😀😀", "hello"] * 40
    arrays = [[], [1], [1, 2, 3]] * 40
    cases = [
        ("number", numbers, numbers, "number"),
        ("string_length", [len(s) for s in strings], strings, "string"),
        ("item_count", [len(a) for a in arrays], arrays, "array"),
    ]
    for kind, quantities, instances, type_name in cases:
        for scorer in (None, tied):
            keywords = decide_bound(quantities, kind, engine(ON), scorer=scorer).keywords()  # type: ignore[arg-type]
            assert keywords or scorer is None, kind  # ties emit both observed extremes
            compiled = validator.compile({"type": type_name, **keywords})
            assert all(validator.is_valid(compiled, instance) for instance in instances), (kind, keywords)


def test_provenance_records_the_mdl_decision_and_full_support() -> None:
    decision = decide_bound(list(range(101)) * 3, "number", engine(ON))
    records = decision.provenance()
    assert list(records) == ["minimum", "maximum"]
    for side in decision.sides:
        record = records[side.keyword]
        assert record.technique == BOUNDS and record.support == Support(303, 303)
        assert record.confidence == side.decision.confidence
        assert record.resolution == side.decision.resolution
        assert record.details == {"observed_extreme": side.extreme}


# ---------------------------------------------------------------------------
# Fallback and skips (16.1, 16.3, 16.8)
# ---------------------------------------------------------------------------


def test_disabled_bounds_omit_both_bounds_through_the_fallback() -> None:
    report = InferenceReport()
    recorder = Recorder()
    decision = decide_bound([1, 2, 3] * 100, "number", engine(OFF, report), path="$.n", scorer=recorder)
    assert decision.keywords() == {} and decision.provenance() == {}
    assert recorder.calls == []  # a disabled Technique produces no candidates
    for side in decision.sides:
        assert side.value is None
        assert side.decision.method == "fallback" and side.decision.label == "none"
        assert side.decision.fallback == FallbackInfo(BOUNDS, "disabled")
    assert [entry.fallback for entry in report.decisions] == [FallbackInfo(BOUNDS, "disabled")] * 2


@pytest.mark.parametrize(
    ("kind", "values", "options", "reason"),
    [
        ("number", [1, 2], {"closed_enum": True}, "closed-enum"),
        ("string_length", [1, 2], {"closed_enum": True}, "closed-enum"),
        ("string_length", [10, 20], {"format_selected": True}, "format"),
        ("item_count", [], {}, "no-quantities"),
        ("number", [5], {"counts": [0]}, "no-quantities"),
    ],
)
def test_skipped_paths_have_no_decision_point(kind: str, values: list[Any], options: dict[str, Any], reason: str) -> None:
    report = InferenceReport()
    decision = decide_bound(values, kind, engine(ON, report), **options)  # type: ignore[arg-type]
    assert decision.skipped == reason
    assert decision.minimum is None and decision.maximum is None and decision.keywords() == {}
    assert report.decisions == ()


def test_skip_options_apply_only_where_the_requirements_say() -> None:
    with pytest.raises(ValueError, match="closed_enum"):
        decide_bound([1], "item_count", engine(ON), closed_enum=True)
    with pytest.raises(ValueError, match="format_selected"):
        decide_bound([1], "number", engine(ON), format_selected=True)
    with pytest.raises(TypeError):
        decide_bound([1], "number", engine(ON), closed_enum=1)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Scores agree with the MDL_Scorer (19.1, 19.3)
# ---------------------------------------------------------------------------


def scorer_score(node: dict[str, Any], view: View) -> Score:
    return MDLScorer().score(SchemaNode.from_json(node), view)


def bounded(base: dict[str, Any], kind: str, low: Any, high: Any) -> dict[str, Any]:
    names = {"number": ("minimum", "maximum"), "string_length": ("minLength", "maxLength"),
             "item_count": ("minItems", "maxItems")}[kind]
    out = dict(base)
    for name, value in zip(names, (low, high)):
        if value is not None:
            out[name] = value
    return out


@pytest.mark.parametrize(
    ("kind", "base", "documents", "quantity", "bounds"),
    [
        ("number", {"type": "integer"}, [0, 5, 5, 17, 100, -3], lambda v: v,
         [(None, None), (-3, None), (None, 100), (-3, 100), (-128, 255), (-200, 200)]),
        ("number", {"type": "number"}, [0.25, 1.5, 3, 0.125, -2.5, 1.5], lambda v: v,
         [(None, None), (-2.5, None), (None, 3), (-3, 3), (-2.5, 3), (-10, 4.5), (0.1 - 3, 3.3)]),
        ("string_length", {"type": "string"}, ["", "ab", "😀😀😀", "abcd", "ab"], len,
         [(None, None), (0, None), (None, 4), (0, 4), (0, 10), (None, 7)]),
        ("item_count", {"type": "array"}, [[], [1], [1, "a"], [None, 2, 3]], len,
         [(None, None), (0, None), (None, 3), (0, 3), (0, 255)]),
    ],
)
def test_scores_agree_with_the_mdl_scorer(
    kind: str, base: dict[str, Any], documents: list[Any], quantity: Any, bounds: list[tuple[Any, Any]]
) -> None:
    view = View.root(build_store(documents))
    q = QuantitySet.of(kind, [quantity(doc) for doc in documents])  # type: ignore[arg-type]
    ours = {}
    theirs = {}
    for low, high in bounds:
        schema = bounded(base, kind, low, high)
        mine = mdl_bound_score(q, SchemaNode.from_json(schema), low, high)
        reference = scorer_score(schema, view)
        assert mine.model_bits == reference.model_bits
        assert (mine.exception_bits, mine.n_rejected, mine.n_scored) == (0.0, 0, len(documents))
        assert reference.n_rejected == 0
        ours[low, high], theirs[low, high] = mine.data_bits, reference.data_bits
    # The MDL_Scorer adds terms that no bound changes (exception flag, type, characters, elements).
    for a, b in itertools.combinations(bounds, 2):
        assert ours[a] - ours[b] == pytest.approx(theirs[a] - theirs[b], rel=1e-12, abs=1e-9)


def test_integer_scores_match_the_scorer_up_to_the_exception_flag() -> None:
    documents = [0, 5, 5, 17, 100, -3]
    view = View.root(build_store(documents))
    q = QuantitySet.of("number", documents)
    for low, high in [(None, None), (-3, None), (None, 100), (-3, 100)]:
        schema = bounded({"type": "integer"}, "number", low, high)
        mine = mdl_bound_score(q, SchemaNode.from_json(schema), low, high)
        # One type costs no type choice; the only other term is the 1-bit exception flag.
        assert scorer_score(schema, view).data_bits == pytest.approx(math.fsum([1.0, mine.data_bits]), rel=1e-15)


def test_base_keywords_count_in_the_model_bits() -> None:
    plain = decide_bound([1, 2, 3] * 50, "number", engine(ON), scorer=tied)
    based = decide_bound([1, 2, 3] * 50, "number", engine(ON), base={"type": ["integer", "string"]})
    node = based.minimum.decision.candidate.node
    assert node is not None and node.kw["type"] == ["integer", "string"]
    for candidate in based.minimum.decision.resolution.candidates:
        assert candidate.model_bits is not None and candidate.model_bits > 0
    assert plain.minimum.decision.candidate.node.kw == {"type": "integer", "minimum": 1}
    with pytest.raises(ValueError, match="must not hold"):
        decide_bound([1], "number", engine(ON), base={"minimum": 0})


# ---------------------------------------------------------------------------
# Input checks and registry state
# ---------------------------------------------------------------------------


def test_quantities_are_validated() -> None:
    with pytest.raises(TypeError):
        QuantitySet.of("number", [True])
    with pytest.raises(TypeError):
        QuantitySet.of("number", ["1"])
    with pytest.raises(ValueError):
        QuantitySet.of("number", [float("nan")])
    with pytest.raises(ValueError):
        QuantitySet.of("string_length", [-1])
    with pytest.raises(TypeError):
        QuantitySet.of("item_count", [1.0])
    with pytest.raises(ValueError, match="entries"):
        QuantitySet.of("number", [1, 2], counts=[1])
    with pytest.raises(ValueError):
        QuantitySet.of("number", [1], counts=[-1])
    with pytest.raises(ValueError):
        QuantitySet.of("lengths", [1])  # type: ignore[arg-type]
    q = QuantitySet.of("number", np.array([3, 1, 3], dtype=np.int64))
    assert (q.values, q.counts, q.total, q.integers) == ((1, 3), (1, 2), 3, True)
    h = QuantitySet.of("item_count", Histogram(np.array([0, 4]), np.array([2, 5])))
    assert (h.values, h.counts, h.total) == ((0, 4), (2, 5), 7)


def test_a_scorer_must_return_a_score() -> None:
    with pytest.raises(TypeError, match="Score"):
        decide_bound([1, 2], "number", engine(ON), scorer=lambda *args: 1.0)  # type: ignore[arg-type,return-value]


def test_values_bounds_is_implemented_and_on_by_default() -> None:
    spec = REGISTRY.technique(BOUNDS)
    assert spec.implemented and spec.default_enabled
    assert spec.serves == ("bound",) and spec.thresholds == ()
    # With default states the engine resolves bounds by MDL.
    assert decide_bound(list(range(101)) * 3, "number", DecisionEngine()).minimum.decision.method == "mdl"
