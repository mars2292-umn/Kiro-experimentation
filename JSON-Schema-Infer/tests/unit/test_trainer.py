"""Unit tests for oracle featurization and Learned_Threshold fitting (tasks 22.2, 22.5)."""

from __future__ import annotations

from typing import Any

import numpy as np
import pytest

from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.paths import render_path
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.learn.features import FEATURE_SPEC
from jsinfer.learn.labels import KINDS
from jsinfer.learn.synth import GeneratorConfig, SyntheticGenerator
from jsinfer.learn.trainer import (
    EXCLUSION_CAUSES,
    M2_THRESHOLDS,
    OmittedThreshold,
    cooccurrence_examples,
    featurization_config,
    featurize_collection,
    featurize_collections,
    fit_m2_thresholds,
    fit_threshold,
    grouping_examples,
    thresholds_json,
)

PLANTED = {
    "type": "object",
    "required": ["id", "kind", "items", "attrs", "u"],
    "properties": {
        "id": {"type": "string", "format": "uuid"},
        "kind": {"enum": ["a", "b", "c"]},
        "n": {"type": "integer"},
        "items": {"type": "array", "items": {"type": "integer"}},
        "pair": {"type": "array", "prefixItems": [{"type": "string"}, {"type": "integer"}], "items": False, "minItems": 2},
        "attrs": {"type": "object", "additionalProperties": {"type": "number"}, "minProperties": 2},
        "u": {
            "oneOf": [
                {"type": "object", "properties": {"kind": {"const": "x"}, "w": {"type": "integer"}}, "required": ["kind", "w"], "additionalProperties": False},
                {"type": "object", "properties": {"kind": {"const": "y"}, "h": {"type": "string"}, "d": {"type": "string"}}, "required": ["kind", "h"], "additionalProperties": False},
            ]
        },
        "addr": {
            "type": "object",
            "properties": {"street": {"type": "string"}, "city": {"type": "string"}, "zip": {"type": "string"}},
            "dependentRequired": {"street": ["city"]},
            "additionalProperties": False,
        },
    },
    "additionalProperties": False,
}
CFG = GeneratorConfig(size_min=30, size_max=30, max_depth=3, presence_min=0.4, presence_max=0.9)


def model_of(doc: Any) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(doc))


@pytest.fixture(scope="module")
def planted() -> Any:
    model = model_of(PLANTED)
    collection = SyntheticGenerator().collection(model, "planted", 0, CFG, 1, split="val")
    return featurize_collection(collection, model)


# -- featurization (22.6, 22.10) --------------------------------------------------


def test_the_oracle_run_records_one_example_per_judgment_call_with_its_label(planted: Any) -> None:
    examples = planted.examples
    counts = examples.counts()
    assert counts["record_or_map"] >= 4 and counts["tuple_or_list"] == 2 and counts["merge_or_split"] >= 1
    by_path = {(render_path(e.path), e.kind): e.label for e in examples.examples}
    assert by_path[("$", "record_or_map")] == "record" and by_path[("$.attrs", "record_or_map")] == "map"
    assert by_path[("$", "closed_or_open_record")] == "closed" and by_path[("$.u", "merge_or_split")] == "split"
    assert by_path[("$.items", "tuple_or_list")] == "list" and by_path[("$.pair", "tuple_or_list")] == "tuple"
    assert by_path[("$.kind", "closed_or_open_enum")] == "closed"
    for example in examples.examples:
        assert example.features.shape == (len(FEATURE_SPEC[example.kind]),) and np.all(np.isfinite(example.features))
        assert example.schema_id == "planted" and example.split == "val" and example.k == 0
        assert render_path(example.schema_path) == render_path(example.path)  # no Map stood in for a Record here
        assert set(example.to_json()) == {"kind", "path", "schema_path", "features", "label", "schema_id", "split", "k"}
    x, y = examples.matrix("record_or_map")
    assert x.shape == (counts["record_or_map"], len(FEATURE_SPEC["record_or_map"])) and len(y) == x.shape[0]
    assert examples.matrix("record_or_map", split="train") == (pytest.approx(np.zeros((0, x.shape[1]))), [])


def test_upstream_judgment_calls_take_the_ground_truth_label(planted: Any) -> None:
    # MDL never closes a Record on such inputs (task 22.4), but the source schema is closed: the oracle forces it,
    # so the emitted schema carries additionalProperties: false, and the children were judged in that context.
    document = planted.result.document
    assert document["additionalProperties"] is False
    assert document["properties"]["addr"]["additionalProperties"] is False
    assert "additionalProperties" in document["properties"]["attrs"] and "properties" not in document["properties"]["attrs"]
    oracle = [e for e in planted.result.report.decisions if e.kind == "closed_or_open_record" and e.path == "$"]
    assert oracle and oracle[0].resolution.method == "mdl"  # the engine resolved by MDL; the label overrode it


def test_exclusions_are_counted_per_kind_and_cause(planted: Any) -> None:
    exclusions = planted.examples.exclusions_json()
    assert set(exclusions) == set(KINDS) and all(set(v) == set(EXCLUSION_CAUSES) for v in exclusions.values())
    # Scalar Paths of all-distinct values (uuid ids, integers) are judged open by 15.5 without a Judgment_Call.
    assert exclusions["closed_or_open_enum"]["no-observation"] >= 2
    assert sum(exclusions["record_or_map"].values()) == 0


def test_unknown_labels_and_unaligned_paths_are_excluded() -> None:
    # A Record with no additionalProperties keyword: Closed-or-Open-Record is unknown (22.5).
    doc = {"type": "object", "properties": {"a": {"type": "string"}, "b": {"type": "integer"}}, "required": ["a", "b"]}
    model = model_of(doc)
    collection = SyntheticGenerator().collection(model, "open", 0, GeneratorConfig(size_min=8, size_max=8), 0)
    featurized = featurize_collection(collection, model)
    assert featurized.examples.exclusions[("closed_or_open_record", "unknown-label")] == 1
    assert [e.kind for e in featurized.examples.examples].count("closed_or_open_record") == 0
    assert featurized.model is model and featurized.labels == collection.labels


def test_the_featurization_configuration_disables_the_decision_models() -> None:
    config = featurization_config({"seed": 3})
    assert config.seed == 3 and config.workers == 1
    assert not any(config.techniques[f"decide.dm_{name}"] for name in ("record_map", "closed_record", "tuple_list", "closed_enum", "merge_split"))
    model = model_of({"type": "integer"})
    collection = SyntheticGenerator().collection(model, "ints", 0, GeneratorConfig(size_min=2, size_max=2), 0)
    with pytest.raises(ValueError, match="disable"):
        featurize_collection(collection, model, config=RunConfiguration.resolve({"workers": 1}))
    with pytest.raises(TypeError):
        featurize_collection(collection, {"type": "integer"})  # type: ignore[arg-type]
    (featurized,) = featurize_collections([(collection, model)])
    assert featurized.schema_id == "ints"


# -- threshold fitting (20.3, 20.4, 20.8) -----------------------------------------


def test_the_grid_search_maximizes_the_objective_with_ties_to_the_smallest_value() -> None:
    spec = THRESHOLDS["cooccur.min_support"]
    statistic = np.array([3, 1, 7, 7, 2])
    seen: list[float] = []

    def objective(value: float) -> float:
        seen.append(value)
        return 1.0 if value >= 2 else 0.0  # every candidate from 2 up ties

    record = fit_threshold(spec, statistic, objective, n_examples=4, training_data="t")
    assert isinstance(record, ThresholdRecord) and record.name == spec.name
    assert record.value == 2 and isinstance(record.value, int) and record.objective_value == 1.0
    assert record.n_examples == 4 and record.training_data == "t"
    assert seen == sorted(seen) and seen == spec.candidates(statistic).tolist()
    unit = fit_threshold(THRESHOLDS["lsh.keyset_jaccard"], [0.5, 0.25], lambda v: -abs(v - 0.5), n_examples=1, training_data="t")
    assert unit.value == 0.5 and isinstance(unit.value, float)
    with pytest.raises(ValueError, match="NaN"):
        fit_threshold(spec, statistic, lambda v: float("nan"), n_examples=1, training_data="t")


def test_the_m2_thresholds_are_fitted_on_validation_examples(planted: Any) -> None:
    groups = grouping_examples([planted])
    assert [render_path(g.path) for g in groups] == ["$.u"] and groups[0].signatures.n_signatures >= 2
    truth = groups[0].object_truth()
    assert set(truth.tolist()) == {0, 1} and len(truth) == len(groups[0].signatures.assignment)
    rules = cooccurrence_examples([planted])
    assert any(("street", "city") in r.truth for r in rules)
    records, omitted = fit_m2_thresholds([planted], training_data="unit")
    assert omitted == [] and sorted(records) == sorted(M2_THRESHOLDS)
    for name, record in records.items():
        assert record.name == name and record.training_data == "unit" and record.n_examples >= 1
        assert THRESHOLDS[name].check_value(record.value) is None
        assert isinstance(record.value, int) == THRESHOLDS[name].integer
    assert records["lsh.keyset_jaccard"].objective_value == 1.0  # the two branches have disjoint keys
    assert records["dbscan.eps"].objective_value == records["dbscan.min_samples"].objective_value  # fitted jointly
    assert records["cooccur.min_support"].objective_value == records["cooccur.min_confidence"].objective_value
    assert records["cooccur.min_confidence"].value >= 0.5  # a minimum Support count in disguise
    assert list(thresholds_json(records)) == sorted(records)
    again, _ = fit_m2_thresholds([planted], training_data="unit")
    assert again == records


def test_thresholds_without_validation_examples_are_omitted_with_a_reason() -> None:
    model = model_of({"type": "array", "items": {"type": "integer"}})
    collection = SyntheticGenerator().collection(model, "arrays", 0, GeneratorConfig(size_min=5, size_max=5), 0, split="val")
    featurized = featurize_collection(collection, model)
    records, omitted = fit_m2_thresholds([featurized], training_data="none")
    assert records == {} and sorted(o.name for o in omitted) == sorted(M2_THRESHOLDS)
    assert all(isinstance(o, OmittedThreshold) and "no validation-split" in o.reason for o in omitted)
    assert omitted[0].to_json() == {"name": omitted[0].name, "reason": omitted[0].reason}
    # Training-split collections do not count (20.3).
    train_only = featurize_collection(SyntheticGenerator().collection(model_of(PLANTED), "p", 0, CFG, 1, split="train"), model_of(PLANTED))
    assert fit_m2_thresholds([train_only], training_data="x")[0] == {}
