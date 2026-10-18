"""Unit tests for config/thresholds.py (task 3.2; Requirements 20.1, 20.2, 20.4, 20.6)."""

from __future__ import annotations

import copy
import functools
import json
import math
from typing import Any

import numpy as np
import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.config.intervals import Interval
from jsinfer.config.thresholds import (
    OBJECTIVES,
    THRESHOLDS,
    WEAK_SUPERVISION,
    ThresholdCheck,
    ThresholdFormatError,
    ThresholdProblem,
    ThresholdRecord,
    ThresholdSpec,
    check_threshold,
    check_thresholds,
    lf_threshold,
    list_thresholds,
    quantile_grid,
    threshold_spec,
    thresholds_used_by,
)
from jsinfer.core.report import SUBSTITUTION_CAUSES

# The design's Learned_Threshold table: name -> (Techniques, Decision_Point kind, objective, range, integer).
EXPECTED = {
    "changepoint.min_segment": (("robust.change_point",), "version_split", "boundary_f1", "[1, inf)", True),
    "changepoint.penalty": (("robust.change_point",), "version_split", "boundary_f1", "[0, inf)", False),
    "cooccur.min_confidence": (("rules.cooccurrence",), "cooccurrence_rule", "cooccurrence_f1", "[0, 1]", False),
    "cooccur.min_support": (("rules.cooccurrence",), "cooccurrence_rule", "cooccurrence_f1", "[1, inf)", True),
    "corpus.suggest_min_confidence": (
        ("corpus.cf_suggest", "corpus.name_suggest"), "suggestion", "suggestion_f1", "[0, 1]", False,
    ),
    "dbscan.eps": (("variants.dbscan",), "variant_grouping", "grouping_ari", "(0, 1]", False),
    "dbscan.min_samples": (("variants.dbscan",), "variant_grouping", "grouping_ari", "[1, inf)", True),
    "exclusion.min_confidence": (("rules.exclusion",), "exclusion_rule", "exclusion_f1", "[0, 1]", False),
    "fd.min_confidence": (("rules.fd_ind",), "fd", "half_split_f1", "[0, 1]", False),
    "hdbscan.min_cluster_size": (("variants.hdbscan",), "variant_grouping", "grouping_ari", "[2, inf)", True),
    "iforest.score": (("robust.isolation_forest",), "document_outlier", "corrupt_document_f1", "[0, 1]", False),
    "ind.min_confidence": (("rules.fd_ind",), "reference", "half_split_f1", "[0, 1]", False),
    "keys.min_confidence": (("rules.keys",), "key", "half_split_f1", "[0, 1]", False),
    "lof.n_neighbors": (("robust.lof",), "document_outlier", "corrupt_document_f1", "[1, inf)", True),
    "lof.score": (("robust.lof",), "document_outlier", "corrupt_document_f1", "[0, inf)", False),
    "lsh.keyset_jaccard": (("variants.minhash_lsh",), "variant_grouping", "grouping_ari", "[0, 1]", False),
    "semantic.min_probability": (
        ("values.semantic_type",), "string_constraint", "semantic_format_f1", "[0, 1]", False,
    ),
    "tagged.branch_support": (("rules.decision_trees",), "discriminator", "tagged_union_f1", "[0, 1]", False),
}


def record(name: str = "dbscan.eps", **changes: Any) -> dict[str, Any]:
    """The JSON of a valid record of the registered spec ``name``, with ``changes`` applied."""
    spec = THRESHOLDS[name]
    value = 3 if spec.integer else 0.5
    doc = ThresholdRecord.from_spec(spec, value=value, objective_value=0.75, n_examples=40, training_data="td-1").to_json()
    doc.update(changes)
    return doc


# ---------------------------------------------------------------------------
# The registry (20.1, 20.2)


def test_registry_matches_the_design_table() -> None:
    actual = {
        name: (spec.techniques, spec.decision_point, spec.objective, spec.admissible.render(), spec.integer)
        for name, spec in THRESHOLDS.items()
    }
    assert actual == EXPECTED


def test_registry_is_in_code_point_order_and_read_only() -> None:
    assert list(THRESHOLDS) == sorted(THRESHOLDS)
    assert all(name == spec.name for name, spec in THRESHOLDS.items())
    with pytest.raises(TypeError):
        THRESHOLDS["x.y"] = THRESHOLDS["dbscan.eps"]  # type: ignore[index]


def test_every_spec_has_a_registered_objective_and_no_default_value() -> None:
    for spec in THRESHOLDS.values():
        assert spec.objective in OBJECTIVES
        assert spec.description
        assert not hasattr(spec, "default")  # values come only from a Model_Bundle (20.6)


def test_threshold_spec_lookup_names_the_registered_thresholds() -> None:
    assert threshold_spec("lof.score") is THRESHOLDS["lof.score"]
    with pytest.raises(KeyError, match="dbscan.eps"):
        threshold_spec("dbscan.radius")


def test_thresholds_used_by() -> None:
    assert [s.name for s in thresholds_used_by(["robust.lof", "variants.dbscan"])] == [
        "dbscan.eps", "dbscan.min_samples", "lof.n_neighbors", "lof.score",
    ]
    # A threshold shared by two Techniques is required by either one.
    assert [s.name for s in thresholds_used_by(["corpus.cf_suggest"])] == ["corpus.suggest_min_confidence"]
    assert thresholds_used_by(["decide.mdl", "variants.bernoulli_mixture"]) == ()


def test_list_thresholds_enumerates_every_registered_threshold() -> None:
    rows = list_thresholds({"dbscan.eps": 0.25, "unregistered.name": 1})
    assert [row.name for row in rows] == list(THRESHOLDS)
    by_name = {row.name: row for row in rows}
    assert by_name["dbscan.eps"].value == 0.25
    assert by_name["dbscan.eps"].techniques == ("variants.dbscan",)
    assert by_name["dbscan.eps"].decision_point == "variant_grouping"
    assert all(row.value is None for row in rows if row.name != "dbscan.eps")
    assert [row.name for row in list_thresholds(specs=[THRESHOLDS["lof.score"]])] == ["lof.score"]


# ---------------------------------------------------------------------------
# ThresholdSpec


def spec_kwargs(**changes: Any) -> dict[str, Any]:
    kwargs: dict[str, Any] = {
        "name": "x.cut",
        "techniques": ("a.one",),
        "decision_point": "bound",
        "objective": "grouping_ari",
        "admissible": Interval(0, 1),
        "integer": False,
        "description": "a test threshold",
    }
    kwargs.update(changes)
    return kwargs


@pytest.mark.parametrize(
    "changes",
    [
        {"name": "nodot"},
        {"name": "Upper.case"},
        {"name": ""},
        {"techniques": ()},
        {"techniques": ("b.two", "a.one")},
        {"techniques": ("a.one", "a.one")},
        {"techniques": ("fallback:bound",)},
        {"techniques": ("baseline:type",)},
        {"techniques": "a.one"},
        {"decision_point": ""},
        {"objective": "accuracy"},
        {"admissible": (0, 1)},
        {"integer": 1},
        {"description": ""},
        {"grid": None},
    ],
)
def test_invalid_specs(changes: dict[str, Any]) -> None:
    with pytest.raises((TypeError, ValueError)):
        ThresholdSpec(**spec_kwargs(**changes))


@pytest.mark.parametrize(
    ("name", "value", "problem"),
    [
        ("dbscan.eps", None, "value is not a number (got null)"),
        ("dbscan.eps", "0.5", "value is not a number (got a string)"),
        ("dbscan.eps", True, "value is not a number (got a boolean)"),
        ("dbscan.eps", math.nan, "value nan is not finite"),
        ("dbscan.eps", math.inf, "value inf is not finite"),
        ("dbscan.eps", -math.inf, "value -inf is not finite"),
        ("dbscan.eps", 0, "value 0 is outside the admissible range (0, 1]"),
        ("dbscan.eps", 1.5, "value 1.5 is outside the admissible range (0, 1]"),
        ("hdbscan.min_cluster_size", 1, "value 1 is outside the admissible range [2, inf)"),
        ("hdbscan.min_cluster_size", 2.5, "value 2.5 is not an integer"),
        ("lof.score", 10**400, "value is too large for a float"),
    ],
)
def test_check_value_flags_unusable_values(name: str, value: object, problem: str) -> None:
    assert THRESHOLDS[name].check_value(value) == problem
    with pytest.raises(ValueError, match=name):
        THRESHOLDS[name].normalize_value(value)


def test_normalize_value_keeps_int_for_integer_specs_and_float_otherwise() -> None:
    assert THRESHOLDS["hdbscan.min_cluster_size"].normalize_value(4.0) == 4
    assert type(THRESHOLDS["hdbscan.min_cluster_size"].normalize_value(np.float64(4.0))) is int
    assert type(THRESHOLDS["dbscan.eps"].normalize_value(1)) is float
    assert THRESHOLDS["hdbscan.min_cluster_size"].normalize_value(10**30) == 10**30


_any_value = st.one_of(
    st.floats(allow_nan=True, allow_infinity=True),
    st.integers(-5, 10**6),
    st.sampled_from([None, True, "1", [1]]),
)


@given(st.sampled_from(sorted(THRESHOLDS)), _any_value)
def test_check_value_accepts_exactly_finite_admissible_values(name: str, value: object) -> None:
    spec = THRESHOLDS[name]
    usable = (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
        and (not spec.integer or float(value).is_integer())
        and spec.admissible.contains(value)
    )
    assert (spec.check_value(value) is None) == usable


# ---------------------------------------------------------------------------
# Grids


def test_default_grid_is_range_ends_plus_every_distinct_observed_value() -> None:
    spec = THRESHOLDS["lsh.keyset_jaccard"]
    grid = spec.candidates([0.5, 0.25, 0.5, math.nan, math.inf, 1.5, -0.0])
    assert grid.tolist() == [0.0, 0.25, 0.5, 1.0]
    assert math.copysign(1.0, grid[0]) == 1.0


def test_default_grid_drops_open_ends_and_rounds_integer_specs() -> None:
    assert THRESHOLDS["dbscan.eps"].candidates([0.0, 0.3]).tolist() == [0.3, 1.0]
    assert THRESHOLDS["hdbscan.min_cluster_size"].candidates([1.5, 3.2, 7.0]).tolist() == [2.0, 3.0, 4.0, 7.0]
    assert THRESHOLDS["lof.score"].candidates([]).tolist() == [0.0]


def test_quantile_grid_with_points_takes_observed_quantiles() -> None:
    spec = ThresholdSpec(**spec_kwargs(grid=functools.partial(quantile_grid, points=3)))
    grid = spec.candidates(np.arange(1, 10) / 10)
    assert grid.tolist() == [0.0, 0.1, 0.5, 0.9, 1.0]
    with pytest.raises(ValueError):
        quantile_grid(spec, [0.5], points=1)


@pytest.mark.parametrize(
    "grid",
    [
        lambda spec, stat: np.array([]),
        lambda spec, stat: np.array([0.5, 0.25]),
        lambda spec, stat: np.array([0.5, 0.5]),
        lambda spec, stat: np.array([0.5, 2.0]),
        lambda spec, stat: np.array([[0.5]]),
    ],
)
def test_candidates_rejects_unusable_custom_grids(grid: Any) -> None:
    with pytest.raises(ValueError):
        ThresholdSpec(**spec_kwargs(grid=grid)).candidates([0.5])


def test_grid_does_not_affect_spec_equality() -> None:
    assert ThresholdSpec(**spec_kwargs()) == ThresholdSpec(**spec_kwargs(grid=lambda spec, stat: stat))


# ---------------------------------------------------------------------------
# Labeling_Function thresholds


def test_lf_threshold() -> None:
    spec = lf_threshold(
        "rm_key_entropy", "min_entropy", decision_point="record_or_map", admissible=Interval(0), integer=False,
        description="key-entropy cut-off of the JXPlain Record-or-Map rule",
    )
    assert spec.name == "lf.rm_key_entropy.min_entropy"
    assert spec.techniques == (WEAK_SUPERVISION,)
    assert spec.objective == "lf_accuracy"
    assert spec.decision_point == "record_or_map"
    with pytest.raises(ValueError):
        lf_threshold("Bad Id", "x", decision_point="record_or_map", admissible=Interval(0), integer=False, description="d")


# ---------------------------------------------------------------------------
# ThresholdRecord (20.4)


def test_record_holds_the_spec_fields_and_the_fit() -> None:
    spec = THRESHOLDS["hdbscan.min_cluster_size"]
    r = ThresholdRecord.from_spec(spec, value=np.float64(5.0), objective_value=np.float64(0.5), n_examples=9, training_data="td")
    assert (r.name, r.techniques, r.decision_point, r.objective, r.admissible, r.integer) == (
        spec.name, spec.techniques, spec.decision_point, spec.objective, spec.admissible, spec.integer,
    )
    assert r.value == 5 and type(r.value) is int
    assert type(r.objective_value) is float
    assert r.mismatch(spec) is None


@pytest.mark.parametrize(
    "changes",
    [
        {"value": 0},
        {"value": math.nan},
        {"value": "0.5"},
        {"objective_value": math.inf},
        {"n_examples": 0},
        {"n_examples": 1.0},
        {"training_data": ""},
        {"techniques": ()},
    ],
)
def test_invalid_records(changes: dict[str, Any]) -> None:
    spec = THRESHOLDS["dbscan.eps"]
    kwargs: dict[str, Any] = {
        "name": spec.name,
        "techniques": spec.techniques,
        "decision_point": spec.decision_point,
        "objective": spec.objective,
        "admissible": spec.admissible,
        "integer": spec.integer,
        "value": 0.5,
        "objective_value": 0.5,
        "n_examples": 3,
        "training_data": "td",
    }
    ThresholdRecord(**kwargs)  # the unchanged record is valid
    kwargs.update(changes)
    with pytest.raises((TypeError, ValueError)):
        ThresholdRecord(**kwargs)


_records = st.sampled_from(sorted(THRESHOLDS)).flatmap(
    lambda name: st.builds(
        lambda value, objective, n: ThresholdRecord.from_spec(
            THRESHOLDS[name], value=value, objective_value=objective, n_examples=n, training_data="td-" + name
        ),
        (
            st.integers(THRESHOLDS[name].admissible.low, 10**6)
            if THRESHOLDS[name].integer
            else st.floats(
                THRESHOLDS[name].admissible.low,
                THRESHOLDS[name].admissible.high if THRESHOLDS[name].admissible.high is not None else 1e9,
                exclude_min=THRESHOLDS[name].admissible.low_open,
            )
        ),
        st.floats(-1, 1),
        st.integers(1, 10**6),
    )
)


@given(_records)
def test_record_json_round_trip(r: ThresholdRecord) -> None:
    doc = r.to_json()
    assert list(doc) == sorted(doc)
    reread = ThresholdRecord.from_json(json.loads(json.dumps(doc, allow_nan=False)))
    assert reread == r
    assert reread.to_json() == doc
    assert check_threshold(THRESHOLDS[r.name], doc) == r


@pytest.mark.parametrize(
    ("change", "location"),
    [
        (lambda d: d.pop("objective"), ""),
        (lambda d: d.update(extra=1), ""),
        (lambda d: d.update(name=None), "/name"),
        (lambda d: d.update(techniques="variants.dbscan"), "/techniques"),
        (lambda d: d.update(techniques=["variants.dbscan", 1]), "/techniques/1"),
        (lambda d: d.update(techniques=["z.z", "variants.dbscan"]), ""),
        (lambda d: d.update(admissible={"low": "0"}), "/admissible/low"),
        (lambda d: d.update(integer="no"), "/integer"),
        (lambda d: d.update(value=math.nan), "/value"),
        (lambda d: d.update(n_examples=2.0), "/n_examples"),
        (lambda d: d.update(n_examples=0), ""),
    ],
)
def test_from_json_rejects_malformed_records(change: Any, location: str) -> None:
    doc = record()
    change(doc)
    with pytest.raises(ThresholdFormatError) as info:
        ThresholdRecord.from_json(doc)
    assert info.value.location == location


def test_from_json_rejects_non_objects() -> None:
    with pytest.raises(ThresholdFormatError):
        ThresholdRecord.from_json([record()], location="/thresholds")


# ---------------------------------------------------------------------------
# Validation against the registry (20.6)


@pytest.mark.parametrize(
    ("entry", "cause", "detail"),
    [
        (None, "missing", "no record"),
        ({k: v for k, v in record().items() if k != "value"}, "missing", "no value"),
        ([record()], "invalid", "an array, not an object"),
        (record(value=math.nan), "invalid", "not finite"),
        (record(value=math.inf), "invalid", "not finite"),
        (record(value="0.5"), "invalid", "not a number"),
        (record(value=None), "invalid", "not a number"),
        (record(value=0.0), "invalid", "outside the admissible range (0, 1]"),
        (record(value=1.25), "invalid", "outside the admissible range (0, 1]"),
        (record(n_examples=0), "invalid", "malformed"),
        (record(admissible={"high": 0.5, "low": 0, "low_open": True}, value=0.75), "invalid", "malformed"),
        (record(admissible={"high": 0.5, "low": 0, "low_open": True}, value=0.25), "invalid", "admissible range"),
        (record(decision_point="merge_or_split"), "invalid", "decision_point"),
        (record(techniques=["variants.hdbscan"]), "invalid", "techniques"),
        (record(objective="cooccurrence_f1"), "invalid", "objective"),
        (record(name="lsh.keyset_jaccard"), "invalid", "name"),
    ],
)
def test_check_threshold_flags_missing_and_invalid_values(entry: Any, cause: str, detail: str) -> None:
    result = check_threshold(THRESHOLDS["dbscan.eps"], entry)
    assert isinstance(result, ThresholdProblem)
    assert result.cause == cause and cause in SUBSTITUTION_CAUSES
    assert detail in result.detail
    assert (result.name, result.techniques, result.decision_point) == (
        "dbscan.eps", ("variants.dbscan",), "variant_grouping",
    )
    assert str(result).startswith(f"Learned_Threshold 'dbscan.eps' is {cause}: ")


def test_check_threshold_integer_specs() -> None:
    spec = THRESHOLDS["dbscan.min_samples"]
    assert check_threshold(spec, record("dbscan.min_samples", value=4.0)).value == 4  # type: ignore[union-attr]
    problem = check_threshold(spec, record("dbscan.min_samples", value=4.5))
    assert isinstance(problem, ThresholdProblem) and "not an integer" in problem.detail
    problem = check_threshold(spec, record("dbscan.min_samples", value=0))
    assert isinstance(problem, ThresholdProblem) and "[1, inf)" in problem.detail


def test_check_threshold_does_not_modify_the_entry() -> None:
    entry = record()
    before = copy.deepcopy(entry)
    assert isinstance(check_threshold(THRESHOLDS["dbscan.eps"], entry), ThresholdRecord)
    assert entry == before


def test_check_thresholds_splits_records_and_problems_in_name_order() -> None:
    specs = [THRESHOLDS[n] for n in ("lof.score", "dbscan.eps", "hdbscan.min_cluster_size", "cooccur.min_support")]
    entries = {
        "dbscan.eps": record(),
        "hdbscan.min_cluster_size": record("hdbscan.min_cluster_size", value=1),
        "lof.score": record("lof.score"),
        "keys.min_confidence": record("keys.min_confidence"),  # not requested: ignored
    }
    result = check_thresholds(specs, entries)
    assert isinstance(result, ThresholdCheck)
    assert list(result.records) == ["dbscan.eps", "lof.score"]
    assert result.values == {"dbscan.eps": 0.5, "lof.score": 0.5}
    assert [(p.name, p.cause) for p in result.problems] == [
        ("cooccur.min_support", "missing"),
        ("hdbscan.min_cluster_size", "invalid"),
    ]
    with pytest.raises(ValueError, match="twice"):
        check_thresholds([THRESHOLDS["dbscan.eps"]] * 2, entries)


def test_an_empty_bundle_leaves_every_threshold_missing() -> None:
    result = check_thresholds(THRESHOLDS.values(), {})
    assert result.records == {}
    assert [p.name for p in result.problems] == list(THRESHOLDS)
    assert all(p.cause == "missing" for p in result.problems)
