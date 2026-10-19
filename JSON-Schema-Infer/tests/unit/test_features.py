"""Unit tests for the versioned Judgment_Call featurizers (task 22.1; Requirements 15.2, 22.10)."""

from __future__ import annotations

import json
from typing import Any

import numpy as np
import pytest

from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.observations import build_store
from jsinfer.core.paths import ELEM_STEP, key_step
from jsinfer.core.views import View
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate, DecisionEngine
from jsinfer.io.validator import Validator
from jsinfer.learn.features import (
    FEATURE_SPEC,
    FEATURE_VERSION,
    FEATURIZERS,
    HASH_BUCKETS,
    closed_enum_features,
    closed_record_features,
    feature_names,
    feature_spec_json,
    merge_split_features,
    name_token_features,
    normalized_mdl_difference,
    record_map_features,
    tuple_list_features,
)
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.variants import VariantAnalyzer

RESOURCES = RunConfiguration.resolve({"workers": 1}).resources


def candidates(kind: str, first: float = 10.0, second: float = 4.0, n: int = 6) -> tuple[Candidate, Candidate]:
    """The two candidates of ``kind`` with description lengths ``first`` and ``second`` over ``n`` Observations."""
    a, b = JUDGMENT_OUTCOMES[kind]
    return (
        Candidate(technique=None, outcome=a, order_key=(0,), score=Score(float(first), 0.0, 0.0, 0, n)),
        Candidate(technique=None, outcome=b, order_key=(1,), score=Score(float(second), 0.0, 0.0, 0, n)),
    )


def view(docs: list[Any], *steps: Any) -> View:
    root = View.root(build_store(docs))
    return root.descend(tuple(steps)) if steps else root


RECORDS = [{"name": f"n{i}", "age": i % 3, "mail": "x@y.z"} for i in range(6)]
USERS = [{"kind": "u", "name": f"n{i}", "age": i} for i in range(8)]
ITEMS = [{"kind": "i", "sku": f"s{i}", "price": 1.5 * i} for i in range(8)]


# -- the spec ---------------------------------------------------------------------


def test_the_spec_names_every_kind_and_ends_with_the_shared_features() -> None:
    assert tuple(FEATURE_SPEC) == tuple(JUDGMENT_OUTCOMES) == tuple(FEATURIZERS)
    for kind, names in FEATURE_SPEC.items():
        assert len(names) == len(set(names)) and feature_names(kind) == names
        assert names[-2 * HASH_BUCKETS - 1] == "normalized_mdl_difference"
        assert names[-2 * HASH_BUCKETS :] == tuple(
            [f"field_token_bucket_{i}" for i in range(HASH_BUCKETS)] + [f"parent_token_bucket_{i}" for i in range(HASH_BUCKETS)]
        )
    spec = feature_spec_json()
    assert spec["version"] == FEATURE_VERSION and json.loads(json.dumps(spec)) == spec
    assert spec["kinds"]["record_or_map"]["dim"] == len(FEATURE_SPEC["record_or_map"])
    assert spec["kinds"]["merge_or_split"]["classes"] == ["merge", "split"]
    with pytest.raises(ValueError):
        feature_names("bound")


def test_the_normalized_mdl_difference_is_the_scaled_gap_of_the_two_scores() -> None:
    assert normalized_mdl_difference(candidates("record_or_map", 10.0, 4.0, 3)) == 2.0
    with pytest.raises(ValueError):
        normalized_mdl_difference(candidates("record_or_map")[:1])
    unscored = Candidate(technique=None, outcome="record", order_key=(0,))
    with pytest.raises(ValueError):
        normalized_mdl_difference((unscored, candidates("record_or_map")[1]))


def test_name_tokens_hash_the_field_and_its_parent() -> None:
    none = name_token_features(())
    assert none == [0.0] * (2 * HASH_BUCKETS)
    field = name_token_features((key_step("emailAddress"),))
    assert sum(field[:HASH_BUCKETS]) == 2 and sum(field[HASH_BUCKETS:]) == 0
    nested = name_token_features((key_step("user"), ELEM_STEP, key_step("email_address")))
    assert nested[:HASH_BUCKETS] == field[:HASH_BUCKETS] and sum(nested[HASH_BUCKETS:]) == 1
    assert name_token_features((key_step("EMAIL-ADDRESS"),)) == field  # normalized tokens


# -- the vectors -------------------------------------------------------------------


@pytest.mark.parametrize(
    ("featurizer", "kind", "docs"),
    [
        (record_map_features, "record_or_map", RECORDS),
        (closed_record_features, "closed_or_open_record", RECORDS),
        (tuple_list_features, "tuple_or_list", [[1, "a"], [2, "b"], [3]]),
        (closed_enum_features, "closed_or_open_enum", ["a", "b", "a", 1, None]),
    ],
)
def test_vectors_match_the_spec_are_finite_and_ignore_document_order(featurizer: Any, kind: str, docs: list[Any]) -> None:
    first = featurizer(view(docs), candidates(kind))
    assert first.shape == (len(FEATURE_SPEC[kind]),) and first.dtype == np.float64 and np.all(np.isfinite(first))
    shuffled = featurizer(view(list(reversed(docs))), candidates(kind))
    assert np.array_equal(first, shuffled)
    assert first[-2 * HASH_BUCKETS - 1] == normalized_mdl_difference(candidates(kind))


def test_record_map_features_read_the_key_statistics() -> None:
    x = record_map_features(view(RECORDS), candidates("record_or_map"))
    names = FEATURE_SPEC["record_or_map"]
    by = dict(zip(names, x.tolist()))
    assert by["n_objects"] == 6 and by["n_keys"] == 3
    assert by["keys_per_object_mean"] == 3.0 and by["keys_per_object_std"] == 0.0
    assert by["singleton_key_fraction"] == 0.0 and by["pairwise_keyset_jaccard"] == 1.0
    assert by["key_length_mean"] == pytest.approx(11 / 3) and by["integer_key_fraction"] == 0.0
    maps = [{f"{i}": i} for i in range(6)]
    by = dict(zip(names, record_map_features(view(maps), candidates("record_or_map")).tolist()))
    assert by["n_keys"] == 6 and by["singleton_key_fraction"] == 1.0 and by["integer_key_fraction"] == 1.0
    assert by["pairwise_keyset_jaccard"] == 0.0


def test_closed_record_features_read_presence_depth_and_stability() -> None:
    docs = [{"a": 1, "b": 2}, {"a": 1}, {"a": 1, "c": 3}, {"a": 1}]
    by = dict(zip(FEATURE_SPEC["closed_or_open_record"], closed_record_features(view(docs), candidates("closed_or_open_record")).tolist()))
    assert by["n_objects"] == 4 and by["n_keys"] == 3 and by["f1"] == 2 and by["f2"] == 0
    assert by["good_turing_unseen_mass"] == 2 / 6 and by["depth"] == 0
    assert by["optional_key_presence"] == 0.25  # b and c each in one object of four
    nested = [{"o": {"x": 1}} for _ in range(3)]
    by = dict(zip(FEATURE_SPEC["closed_or_open_record"], closed_record_features(view(nested, key_step("o")), candidates("closed_or_open_record")).tolist()))
    assert by["depth"] == 1 and by["keyset_stability"] == 1.0 and by["optional_key_presence"] == 1.0


def test_tuple_list_features_read_the_length_statistics() -> None:
    docs = [[1, "a"], [2, "b"], [3, "c"], [4]]
    by = dict(zip(FEATURE_SPEC["tuple_or_list"], tuple_list_features(view(docs), candidates("tuple_or_list")).tolist()))
    assert by["n_arrays"] == 4 and by["max_length"] == 2 and by["modal_length_share"] == 0.75
    assert by["distinct_length_ratio"] == 0.5 and by["distinct_element_types"] == 2
    assert by["positional_minus_pooled_type_entropy"] < 0  # positions are typed, the pool is mixed


def test_enum_features_extend_the_six_statistics() -> None:
    docs = ["red", "red", "blue", "x-1", "x-1", "x-1"]
    x = closed_enum_features(view(docs), candidates("closed_or_open_enum"))
    by = dict(zip(FEATURE_SPEC["closed_or_open_enum"], x.tolist()))
    assert (by["n"], by["s_obs"], by["f1"], by["f2"]) == (6, 3, 1, 1)
    assert by["s_obs_ratio"] == 0.5 and by["dominant_type_rank"] == 4  # string
    assert by["string_length_mean"] == (3 * 2 + 4 + 3 * 3) / 6 and by["identifier_fraction"] == 1.0
    with pytest.raises(ValueError):
        closed_enum_features(view([[1], {"a": 1}]), candidates("closed_or_open_enum"))


def test_merge_split_features_include_the_scalar_discriminator_information() -> None:
    engine = DecisionEngine(scorer=MDLScorer())
    variants = VariantAnalyzer(engine, scorer=MDLScorer(), resources=RESOURCES, seed=0, validator=Validator())
    grouping = variants.select(view([*USERS, *ITEMS]))
    assert grouping.n_variants == 2
    x = merge_split_features(grouping, candidates("merge_or_split"))
    by = dict(zip(FEATURE_SPEC["merge_or_split"], x.tolist()))
    assert by["n_variants"] == 2 and by["keyset_jaccard_max"] < 0.5 and by["group_size_entropy"] == 1.0
    assert by["scalar_field_information"] == 1.0  # "kind" predicts the Variant exactly
    assert by["shared_key_type_agreement"] == 1.0
    plain = [{k: v for k, v in d.items() if k != "kind"} for d in [*USERS, *ITEMS]]
    grouping = variants.select(view(plain))
    assert grouping.n_variants == 2
    by = dict(zip(FEATURE_SPEC["merge_or_split"], merge_split_features(grouping, candidates("merge_or_split")).tolist()))
    assert by["scalar_field_information"] == 0.0 and by["keyset_jaccard_min"] == 0.0
    with pytest.raises(TypeError):
        merge_split_features(view(plain), candidates("merge_or_split"))  # type: ignore[arg-type]


def test_the_analyzer_judges_with_the_versioned_featurizers() -> None:
    analyzer = StructureAnalyzer.for_techniques(resources=RESOURCES)
    assert analyzer.values.enum_features is closed_enum_features
