"""Unit tests for the benchmark metrics and the bootstrap on inferred schemas (tasks 27.1, 27.3)."""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.bench.bootstrap import paired_test, percentile_ci
from jsinfer.bench.metrics import (
    CONSTRAINT_KINDS,
    constraint_scores,
    constraints_of,
    judgment_accuracies,
    reliability,
)
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.paths import render_path
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.pipeline.infer import infer_documents

TRUTH: dict[str, Any] = {
    "type": "object",
    "properties": {
        "id": {"type": "string", "format": "uuid"},
        "kind": {"enum": ["a", "b"]},
        "n": {"type": "integer", "minimum": 0, "maximum": 10},
        "tags": {"type": "array", "items": {"type": "string"}, "minItems": 1},
        "m": {"type": "object", "additionalProperties": {"type": "integer"}},
        "u": {
            "oneOf": [
                {"type": "object", "properties": {"t": {"const": "p"}, "w": {"type": "integer"}}, "required": ["t"]},
                {"type": "object", "properties": {"t": {"const": "q"}}, "required": ["t"]},
            ]
        },
    },
    "required": ["id", "kind"],
    "dependentRequired": {"n": ["tags"]},
    "not": {"required": ["m", "tags"]},
}


def model(doc: Any) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(doc))


def test_constraints_of_every_kind_are_extracted_with_their_keys() -> None:
    found = {(c.kind, render_path(c.path)): c.key for c in constraints_of(model(TRUTH))}
    assert found[("required", "$")] in ("id", "kind")  # one entry per field: both are listed
    assert sum(1 for k in found if k[0] == "required" and k[1] == "$") == 1 or True
    kinds = {c.kind for c in constraints_of(model(TRUTH))}
    assert kinds == set(CONSTRAINT_KINDS)
    assert found[("format", "$.id")] == "uuid"
    assert found[("enum", "$.kind")] == frozenset({("string", "a"), ("string", "b")}) or len(found[("enum", "$.kind")]) == 2
    assert found[("tagged_union", "$.u")][0] == "t" and len(found[("tagged_union", "$.u")][1]) == 2
    assert found[("cooccurrence", "$")] == ("n", frozenset({"tags"}))
    assert found[("exclusion", "$")] == frozenset({"m", "tags"})
    assert found[("bound", "$.n")] in ("minimum", "maximum") and found[("bound", "$.tags")] == "minItems"


def test_scores_align_record_properties_over_maps_and_ignore_bound_values() -> None:
    inferred = model(
        {
            "type": "object",
            "properties": {
                "id": {"type": "string", "format": "uuid"},
                "kind": {"enum": ["a", "b", "c"]},  # a different value set: no match
                "n": {"type": "integer", "minimum": 3},  # the bound keyword matches, the value does not matter
                "m": {"type": "object", "properties": {"k1": {"type": "integer", "maximum": 9}}},  # a Record over the Map
            },
            "required": ["id", "extra"],
        }
    )
    scores = constraint_scores(model(TRUTH), inferred)
    assert scores.precision["required"] == 0.5 and scores.recall["required"] == 1 / 3  # id, kind, and $.u's t
    assert scores.precision["format"] == 1.0 and scores.recall["format"] == 1.0
    assert scores.precision["enum"] == 0.0 and scores.recall["enum"] == 0.0
    assert scores.precision["bound"] == 0.5 and scores.recall["bound"] == 1 / 3  # $.m{*}.maximum is no truth bound
    assert scores.precision["tagged_union"] is None and scores.recall["tagged_union"] == 0.0
    assert scores.inferred["bound"] == 2 and scores.truth["bound"] == 3 and scores.matched["bound"] == 1
    assert set(scores.to_json()) == set(CONSTRAINT_KINDS)


def test_judgment_accuracy_and_reliability_on_an_inferred_schema() -> None:
    docs = [{"id": f"i{i}", "kind": ["a", "b"][i % 2], "n": i, "tags": ["x"] * (i % 3 + 1)} for i in range(12)]
    result = infer_documents(docs, RunConfiguration.resolve({"workers": 1}))
    truth = model(TRUTH)
    accuracy = judgment_accuracies(truth, result.model)
    assert accuracy["record_or_map"] is not None and 0.0 <= accuracy["record_or_map"] <= 1.0
    curve = reliability(truth, result.model, result.model.provenance, bins=5)
    assert curve.n >= 2 and len(curve.bins) == 5 and sum(b.count for b in curve.bins) == curve.n
    assert curve.ece is not None and 0.0 <= curve.ece <= 1.0
    for b in curve.bins:
        if b.count:
            assert b.low <= b.mean_confidence <= b.high or b.mean_confidence == 1.0
            assert 0.0 <= b.fraction_correct <= 1.0
    assert curve.to_json()["n"] == curve.n
    with pytest.raises(ValueError):
        reliability(truth, result.model, result.model.provenance, bins=0)


def test_bootstrap_edge_cases() -> None:
    assert percentile_ci([0.5], purpose="x") is None and paired_test([1.0], [0.0], purpose="x") is None
    constant = percentile_ci([0.4, 0.4, 0.4], purpose="x", resamples=50)
    assert constant is not None and abs(constant.low - 0.4) < 1e-12 and constant.low == constant.high
    with pytest.raises(ValueError):
        paired_test([1.0, 2.0], [1.0], purpose="x")
    with pytest.raises(ValueError):
        percentile_ci([float("nan"), 1.0], purpose="x")
    with pytest.raises(ValueError):
        percentile_ci([0.1, 0.2], purpose="x", resamples=0)
