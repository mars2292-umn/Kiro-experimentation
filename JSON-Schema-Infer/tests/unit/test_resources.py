"""Unit tests for config/resources.py (task 3.2; Requirements 27.2, 29.4, 29.11)."""

from __future__ import annotations

import math
from typing import Any

import numpy as np
import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.config import resources
from jsinfer.config.intervals import Interval
from jsinfer.config.resources import (
    BOUNDED_BY,
    RESOURCE_PARAMETERS,
    SAMPLE_SIZES,
    WORKERS,
    SettingProblem,
    SettingSpec,
    check_resources,
    logical_cpu_count,
    resource_defaults,
)
from jsinfer.config.thresholds import THRESHOLDS

# The design's proposed Resource_Parameter defaults (workers aside).
DESIGN_DEFAULTS: dict[str, int | float] = {
    "ingest.chunk_size": 2_000,
    "soundness.max_passes": 5,
    "minhash.perm": 128,
    "lsh.bands": 32,
    "variants.max_components": 10,
    "variants.em_max_iter": 100,
    "variants.em_tol": 1e-6,
    "fpgrowth.min_support": 2,
    "patterns.max_alternatives": 4,
    "patterns.attempt_budget": 200,
    "trees.max_depth": 4,
    "trees.max_values": 32,
    "outliers.hash_dims": 256,
    "iforest.n_estimators": 200,
    "sample.dbscan": 5_000,
    "sample.hdbscan": 5_000,
    "sample.lof": 20_000,
    "sample.fpgrowth": 50_000,
    "sample.bmf": 20_000,
    "sample.trees": 50_000,
    "sample.fd": 100_000,
    "sample.patterns": 20_000,
    "sample.changepoint": 20_000,
    "nearest.ref_depth": 8,
    "nearest.next": 5,
    "synth.collections_per_schema": 3,
    "synth.doc_attempts": 50,
    "synth.max_depth": 6,
    "train.max_schemas": 2_000,
    "ece.bins": 15,
    "bootstrap.resamples": 2_000,
    "subschema.timeout_s": 30.0,
    "bench.run_timeout_s": 1_800.0,
    "ds.max_iter": 100,
    "ds.tol": 1e-6,
}


# ---------------------------------------------------------------------------
# The table


def test_table_holds_the_design_parameters_and_workers() -> None:
    assert sorted(RESOURCE_PARAMETERS) == sorted([*DESIGN_DEFAULTS, WORKERS])
    assert list(RESOURCE_PARAMETERS) == sorted(RESOURCE_PARAMETERS)
    assert WORKERS == "workers"


def test_defaults_are_the_design_proposals_with_matching_types() -> None:
    defaults = resource_defaults()
    for name, expected in DESIGN_DEFAULTS.items():
        assert defaults[name] == expected, name
        assert type(defaults[name]) is RESOURCE_PARAMETERS[name].type, name
    assert list(defaults) == list(RESOURCE_PARAMETERS)


def test_every_default_passes_its_own_checks() -> None:
    assert check_resources(resource_defaults()) == []
    assert check_resources({}) == []


def test_workers_defaults_to_the_logical_cpu_count(monkeypatch: pytest.MonkeyPatch) -> None:
    assert RESOURCE_PARAMETERS[WORKERS].default is None
    monkeypatch.setattr(resources.os, "cpu_count", lambda: 3)
    assert logical_cpu_count() == 3
    assert RESOURCE_PARAMETERS[WORKERS].default_value() == 3
    assert resource_defaults()[WORKERS] == 3
    monkeypatch.setattr(resources.os, "cpu_count", lambda: None)
    assert RESOURCE_PARAMETERS[WORKERS].default_value() == 1


def test_sample_sizes() -> None:
    assert SAMPLE_SIZES == tuple(sorted(name for name in DESIGN_DEFAULTS if name.startswith("sample.")))
    assert len(SAMPLE_SIZES) == 9


def test_no_resource_parameter_shares_a_learned_threshold_name() -> None:
    assert not [name for name in RESOURCE_PARAMETERS if name in THRESHOLDS]


def test_counts_are_integers_at_least_one_except_nearest_next() -> None:
    for name, spec in RESOURCE_PARAMETERS.items():
        if spec.type is int:
            expected = 0 if name == "nearest.next" else 1
            assert spec.range == Interval(expected), name


# ---------------------------------------------------------------------------
# Checks (27.11, 29.11)


@pytest.mark.parametrize("name", [WORKERS, *SAMPLE_SIZES])
@pytest.mark.parametrize("value", [0, -1, 1.5, 2.0, True, "8", None, math.nan])
def test_worker_count_and_sample_sizes_must_be_integers_at_least_one(name: str, value: Any) -> None:
    problem = RESOURCE_PARAMETERS[name].check(value)
    assert isinstance(problem, SettingProblem)
    assert problem.name == name
    assert problem.reason == ("range" if isinstance(value, int) and not isinstance(value, bool) else "type")
    # The message names the setting and the accepted range (29.11).
    assert problem.message.startswith(f"{name}: expected an integer in [1, inf), got ")
    assert str(problem) == problem.message


@pytest.mark.parametrize("name", [WORKERS, *SAMPLE_SIZES])
def test_worker_count_and_sample_sizes_accept_positive_integers(name: str) -> None:
    spec = RESOURCE_PARAMETERS[name]
    for value in (1, 7, np.int64(64), 10**30):
        assert spec.check(value) is None
        assert spec.normalize(value) == int(value) and type(spec.normalize(value)) is int


def test_messages_show_the_offending_value() -> None:
    lof = RESOURCE_PARAMETERS["sample.lof"]
    assert str(lof.check(2.0)) == "sample.lof: expected an integer in [1, inf), got 2.0"
    assert str(lof.check("8")) == "sample.lof: expected an integer in [1, inf), got the string '8'"
    assert str(lof.check(True)) == "sample.lof: expected an integer in [1, inf), got the boolean true"
    timeout = RESOURCE_PARAMETERS["subschema.timeout_s"]
    assert str(timeout.check(0)) == "subschema.timeout_s: expected a number in (0, inf), got 0"


def test_float_settings_take_ints_and_store_floats() -> None:
    spec = RESOURCE_PARAMETERS["bench.run_timeout_s"]
    assert spec.normalize(60) == 60.0 and type(spec.normalize(60)) is float
    assert spec.normalize(np.float32(0.5)) == 0.5
    for bad in (0, -1.0, math.inf, math.nan, 10**400, False, "60"):
        assert spec.check(bad) is not None, bad
    tol = RESOURCE_PARAMETERS["ds.tol"]
    assert tol.check(0) is None and tol.check(0.0) is None and tol.check(-1e-9) is not None
    assert math.copysign(1.0, tol.normalize(-0.0)) == 1.0


def test_nearest_next_accepts_zero() -> None:
    assert RESOURCE_PARAMETERS["nearest.next"].check(0) is None
    assert RESOURCE_PARAMETERS["nearest.next"].check(-1) is not None
    assert RESOURCE_PARAMETERS["nearest.ref_depth"].check(0) is not None


def test_normalize_raises_with_the_problem_message() -> None:
    with pytest.raises(ValueError, match=r"^workers: expected an integer in \[1, inf\), got 0$"):
        RESOURCE_PARAMETERS[WORKERS].normalize(0)


@given(st.sampled_from(sorted(RESOURCE_PARAMETERS)), st.one_of(st.integers(-3, 10**7), st.floats()))
def test_check_agrees_with_type_and_range(name: str, value: int | float) -> None:
    spec = RESOURCE_PARAMETERS[name]
    assert spec.range is not None
    type_ok = isinstance(value, int) or spec.type is float
    expected = type_ok and math.isfinite(value) and spec.range.contains(value)
    assert (spec.check(value) is None) == expected


# ---------------------------------------------------------------------------
# check_resources


def test_check_resources_reports_every_problem_in_name_order() -> None:
    problems = check_resources({"workers": 0, "sample.lof": 2.5, "no.such": 1, "ds.tol": math.inf, "trees.max_depth": 3})
    assert [(p.name, p.reason) for p in problems] == [
        ("ds.tol", "range"),
        ("no.such", "unknown"),
        ("sample.lof", "type"),
        ("workers", "range"),
    ]
    assert str(problems[1]) == "unknown Resource_Parameter 'no.such'"


def test_lsh_bands_is_bounded_by_minhash_perm() -> None:
    assert BOUNDED_BY == (("lsh.bands", "minhash.perm"),)
    assert check_resources({"lsh.bands": 128}) == []
    assert check_resources({"lsh.bands": 16, "minhash.perm": 16}) == []
    (problem,) = check_resources({"lsh.bands": 129})
    assert (problem.name, problem.reason) == ("lsh.bands", "range")
    assert problem.message == "lsh.bands: lsh.bands (129) must not exceed minhash.perm (128, the default)"
    # Lowering only minhash.perm below the default band count names minhash.perm.
    (problem,) = check_resources({"minhash.perm": 16})
    assert problem.name == "minhash.perm"
    assert problem.message == "minhash.perm: lsh.bands (32, the default) must not exceed minhash.perm (16)"
    (problem,) = check_resources({"minhash.perm": 16, "lsh.bands": 17})
    assert problem.name == "lsh.bands"


def test_relation_is_skipped_when_a_side_is_already_invalid() -> None:
    problems = check_resources({"minhash.perm": 0, "lsh.bands": 32})
    assert [(p.name, p.reason) for p in problems] == [("minhash.perm", "range")]


# ---------------------------------------------------------------------------
# SettingSpec


@pytest.mark.parametrize(
    "kwargs",
    [
        {"name": "", "type": int, "range": Interval(1), "default": 1},
        {"name": "x", "type": list, "range": None, "default": []},
        {"name": "x", "type": int, "range": None, "default": 1},
        {"name": "x", "type": bool, "range": Interval(0), "default": True},
        {"name": "x", "type": int, "range": Interval(1)},
        {"name": "x", "type": int, "range": Interval(1), "default": 1, "default_factory": lambda: 1},
        {"name": "x", "type": int, "range": Interval(1), "default": 0},
        {"name": "x", "type": int, "range": Interval(1), "default": 1.0},
        {"name": "x", "type": int, "range": Interval(1), "default_factory": 3},
    ],
)
def test_invalid_setting_specs(kwargs: dict[str, Any]) -> None:
    with pytest.raises((TypeError, ValueError)):
        SettingSpec(**kwargs)


def test_bool_and_str_settings() -> None:
    flag = SettingSpec("output.inline_provenance", bool, None, False)
    assert flag.check(True) is None
    assert flag.accepted() == "a boolean"
    assert str(flag.check(1)) == "output.inline_provenance: expected a boolean, got 1"
    text = SettingSpec("run.model_bundle", str, None, "bundles/default")
    assert text.check("x") is None
    assert text.check(3) is not None
    assert text.normalize("x") == "x"


def test_a_factory_returning_an_invalid_value_is_an_error() -> None:
    spec = SettingSpec("x", int, Interval(1), default_factory=lambda: 0)
    with pytest.raises(ValueError, match="default factory"):
        spec.default_value()


# ---------------------------------------------------------------------------
# ResourceReader (task 19.1; Requirement 20.5)
# ---------------------------------------------------------------------------


def test_reader_lists_each_read_with_its_origin_and_nothing_else() -> None:
    from jsinfer.config.resources import ResourceReader
    from jsinfer.config.run_config import RunConfiguration
    from jsinfer.core.report import InferenceReport

    config = RunConfiguration.resolve({"minhash.perm": 64}, {"resources": {"sample.dbscan": 7}})
    report = InferenceReport()
    reader = ResourceReader.for_run(config, report)
    assert reader.report is report
    assert reader.value("minhash.perm") == 64 and report.resources == {}  # value() records nothing
    assert reader.read("minhash.perm") == 64
    assert reader.read("sample.dbscan") == 7
    assert reader.read("lsh.bands") == RESOURCE_PARAMETERS["lsh.bands"].default
    assert reader.read("minhash.perm") == 64  # a second equal read is one entry
    listed = {name: (use.value, use.origin) for name, use in report.resources.items()}
    assert listed == {"lsh.bands": (32, "default"), "minhash.perm": (64, "flag"), "sample.dbscan": (7, "file")}


def test_reader_of_a_mapping_records_nothing_and_fills_defaults() -> None:
    from jsinfer.config.resources import ResourceReader

    reader = ResourceReader.of({"sample.dbscan": 3})
    assert ResourceReader.of(reader) is reader and reader.report is None
    assert reader.read("sample.dbscan") == 3 and reader.origin("sample.dbscan") == "default"
    assert reader.value("variants.em_tol") == 1e-6 and type(ResourceReader({"ds.tol": 0}).value("ds.tol")) is float
    with pytest.raises(KeyError, match="unknown Resource_Parameter"):
        reader.value("not.a.setting")
    with pytest.raises(KeyError, match="unknown Resource_Parameter"):
        reader.origin("not.a.setting")


@pytest.mark.parametrize(
    ("values", "origins", "match"),
    [
        ({"sample.dbscan": 0}, None, "invalid Resource_Parameters"),
        ({"not.a.setting": 1}, None, "invalid Resource_Parameters"),
        ({"lsh.bands": 256}, None, "invalid Resource_Parameters"),
        ({}, {"sample.dbscan": "flag"}, "no supplied value"),
        ({"sample.dbscan": 3}, {"sample.dbscan": "cli"}, "origin of Resource_Parameter"),
        ({"sample.dbscan": 3}, {}, "origin of every supplied value"),
    ],
)
def test_reader_rejects_invalid_values_and_origins(values: dict[str, Any], origins: Any, match: str) -> None:
    from jsinfer.config.resources import ResourceReader
    from jsinfer.core.report import InferenceReport

    with pytest.raises(ValueError, match=match):
        ResourceReader(values, origins=origins, report=InferenceReport())
