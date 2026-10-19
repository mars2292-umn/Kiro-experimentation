"""Unit tests for the Synthetic_Generator (task 21.3; Requirements 22.1–22.3, 22.7–22.9)."""

from __future__ import annotations

import json
from collections import Counter
from typing import Any

import numpy as np
import pytest

from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.io.validator import Validator
from jsinfer.learn.labels import derive_labels
from jsinfer.learn.synth import (
    FORMAT_GENERATORS,
    GeneratorConfig,
    SampledParameters,
    SchemaExcluded,
    SyntheticGenerator,
    ValueIndex,
    collection_digest,
    sample_pattern,
)
from jsinfer.values.formats import FORMATS

VALIDATOR = Validator()
CFG = GeneratorConfig(size_min=6, size_max=10, max_depth=3, doc_attempts=20)


def _model(doc: object) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(doc))


def _docs(doc: object, seed: int = 0, cfg: GeneratorConfig = CFG, index: ValueIndex | None = None) -> list[Any]:
    return list(SyntheticGenerator(index=index).collection(_model(doc), "s", 0, cfg, seed).documents)


# -- configuration and parameters --------------------------------------------


@pytest.mark.parametrize(
    "kwargs",
    [
        {"size_min": 0},
        {"size_min": 5, "size_max": 4},
        {"presence_min": -0.1},
        {"presence_max": 1.5},
        {"presence_min": 0.6, "presence_max": 0.5},
        {"enum_skew_min": 2.0, "enum_skew_max": 1.0},
        {"doc_attempts": 0},
        {"max_depth": -1},
        {"array_length_max": -1.0},
        {"size_min": True},
    ],
)
def test_generator_config_rejects_bad_values(kwargs: dict[str, Any]) -> None:
    with pytest.raises(ValueError):
        GeneratorConfig(**kwargs)


def test_generator_config_defaults_come_from_the_resource_parameters() -> None:
    cfg = GeneratorConfig()
    assert cfg.max_depth == 6
    assert cfg.doc_attempts == 50
    assert json.loads(json.dumps(cfg.to_json()))["size_min"] == 1


def test_sampled_parameters_are_recorded_per_collection_and_round_trip() -> None:
    doc = {
        "type": "object",
        "required": ["a"],
        "properties": {"a": {"type": "string"}, "b": {"type": "string"}, "c": {"items": {"properties": {"d": {}}}}},
    }
    collection = SyntheticGenerator().collection(_model(doc), "s", 2, CFG, 7, split="train")
    params = collection.params
    assert collection.split == "train" and collection.k == 2 and collection.schema_id == "s"
    assert CFG.size_min <= params.size <= CFG.size_max and len(collection.documents) == params.size
    assert sorted(params.presence_rates) == ["$.b", "$.c", "$.c[*].d"]
    assert all(0.0 <= rate <= 1.0 for rate in params.presence_rates.values())
    assert 0.0 <= params.array_length_mean <= CFG.array_length_max
    assert CFG.enum_skew_min <= params.enum_skew <= CFG.enum_skew_max
    again = SampledParameters.from_json(json.loads(json.dumps(params.to_json())))
    assert again == params
    assert collection.labels == derive_labels(_model(doc), CFG.max_depth)


def test_parameters_differ_between_collections_of_one_schema() -> None:
    doc = {"type": "object", "properties": {"a": {"type": "string"}, "b": {"type": "string"}}}
    first = SyntheticGenerator().collection(_model(doc), "s", 0, CFG, 0).params
    second = SyntheticGenerator().collection(_model(doc), "s", 1, CFG, 0).params
    assert first != second


# -- correctness and determinism (22.7, 22.9) -------------------------------


PLANTED = {
    "type": "object",
    "required": ["id", "kind"],
    "properties": {
        "id": {"type": "string", "format": "uuid"},
        "kind": {"enum": ["a", "b", "c"]},
        "code": {"type": "string", "pattern": "^[A-Z]{2}-[0-9]{3}$"},
        "n": {"type": "integer", "minimum": 0, "maximum": 10, "multipleOf": 2},
        "m": {"type": "object", "additionalProperties": {"type": "number"}, "minProperties": 1},
        "t": {"type": "array", "prefixItems": [{"type": "string"}, {"type": "integer"}], "items": False},
        "l": {"type": "array", "items": {"$ref": "#/$defs/node"}, "maxItems": 3},
        "u": {
            "oneOf": [
                {"type": "object", "properties": {"k": {"const": "K"}}, "required": ["k"], "additionalProperties": False},
                {"type": "object", "properties": {"j": {"type": "boolean"}}, "required": ["j"], "additionalProperties": False},
            ]
        },
    },
    "dependentRequired": {"n": ["code"]},
    "$defs": {
        "node": {
            "type": "object",
            "properties": {"next": {"$ref": "#/$defs/node"}, "v": {"type": "string", "maxLength": 3}},
            "required": ["v"],
            "additionalProperties": False,
        }
    },
}


def test_every_document_validates_and_runs_are_byte_identical() -> None:
    model = _model(PLANTED)
    compiled = VALIDATOR.compile(PLANTED)
    first = SyntheticGenerator().collection(model, "planted", 0, CFG, 3)
    second = SyntheticGenerator(Validator()).collection(model, "planted", 0, CFG, 3)
    assert first.documents and all(VALIDATOR.is_valid(compiled, doc) for doc in first.documents)
    assert collection_digest(first) == collection_digest(second)
    assert first.labels == second.labels and first.params == second.params
    assert collection_digest(first) != collection_digest(SyntheticGenerator().collection(model, "planted", 0, CFG, 4))
    assert collection_digest(first) != collection_digest(SyntheticGenerator().collection(model, "planted", 1, CFG, 3))


def test_documents_are_independent_of_order_and_of_the_worker_count() -> None:
    model = _model(PLANTED)
    generator = SyntheticGenerator()
    whole = generator.collection(model, "planted", 0, CFG, 1)
    order = list(range(whole.params.size))
    np.random.default_rng(5).shuffle(order)
    single = {i: generator.document(model, "planted", 0, i, CFG, whole.params, 1)[0] for i in order}
    assert [single[i] for i in range(whole.params.size)] == list(whole.documents)
    streamed = list(generator.documents(model, 3, (1, "planted", 0), CFG))
    assert streamed == list(whole.documents[:3])

    sources = [("planted", model, "train"), ("empty", _model(False), "val"), ("other", _model({"type": "integer"}), "val")]
    serial, excluded = generator.collections(sources, CFG, 1, 2, workers=1)
    parallel, excluded_parallel = generator.collections(sources, CFG, 1, 2, workers=2)
    assert [(c.schema_id, c.k, c.split) for c in serial] == [("planted", 0, "train"), ("planted", 1, "train"), ("other", 0, "val"), ("other", 1, "val")]
    assert [collection_digest(c) for c in serial] == [collection_digest(c) for c in parallel]
    assert [c.labels for c in serial] == [c.labels for c in parallel]
    assert excluded == excluded_parallel == [("empty", "no accepted document within 20 attempts (collection 0, document 0)")]
    assert collection_digest(serial[0]) == collection_digest(whole)


def test_collections_rejects_duplicate_ids_and_bad_counts() -> None:
    generator = SyntheticGenerator()
    with pytest.raises(ValueError):
        generator.collections([("a", _model({}), None), ("a", _model({}), None)], CFG, 0, 1)
    with pytest.raises(ValueError):
        generator.collections([], CFG, 0, 1, workers=0)
    with pytest.raises(ValueError):
        generator.collection(_model({}), "", 0, CFG, 0)
    with pytest.raises(ValueError):
        generator.collection(_model({}), "a", -1, CFG, 0)


# -- exclusions (22.8) ---------------------------------------------------------


def test_uncompilable_schemas_are_excluded_with_the_cause() -> None:
    with pytest.raises(SchemaExcluded) as info:
        SyntheticGenerator().collection(_model({"type": "string", "pattern": "("}), "broken", 0, CFG, 0)
    assert info.value.schema_id == "broken" and info.value.reason.startswith("uncompilable: ")
    assert "broken" in str(info.value)


def test_unsatisfiable_schemas_exhaust_the_attempt_budget() -> None:
    cfg = GeneratorConfig(size_min=1, size_max=1, doc_attempts=3)
    with pytest.raises(SchemaExcluded) as info:
        SyntheticGenerator().collection(_model({"type": "integer", "minimum": 5, "maximum": 4}), "s", 0, cfg, 0)
    assert info.value.reason == "no accepted document within 3 attempts (collection 0, document 0)"


def test_required_recursion_beyond_the_depth_cap_terminates_with_an_exclusion() -> None:
    doc = {"$ref": "#/$defs/n", "$defs": {"n": {"type": "object", "required": ["next"], "properties": {"next": {"$ref": "#/$defs/n"}}}}}
    cfg = GeneratorConfig(size_min=1, size_max=1, max_depth=2, doc_attempts=5)
    with pytest.raises(SchemaExcluded):
        SyntheticGenerator().collection(_model(doc), "deep", 0, cfg, 0)


def test_optional_recursion_stays_within_the_depth_cap() -> None:
    doc = {"$ref": "#/$defs/n", "$defs": {"n": {"type": "object", "required": ["v"], "properties": {"v": {"type": "integer"}, "next": {"$ref": "#/$defs/n"}}}}}
    cfg = GeneratorConfig(size_min=20, size_max=20, max_depth=2, presence_min=0.9, presence_max=1.0)
    docs = _docs(doc, cfg=cfg)

    def depth(value: Any) -> int:
        return 1 + depth(value["next"]) if "next" in value else 1

    assert max(depth(d) for d in docs) >= 2
    assert all(depth(d) <= cfg.max_depth + 1 for d in docs)


# -- value sources (22.3) ------------------------------------------------------


@pytest.mark.parametrize("fmt", FORMATS)
def test_format_generators_produce_values_the_validator_accepts(fmt: str) -> None:
    rng = np.random.default_rng(11)
    values = [FORMAT_GENERATORS[fmt](rng) for _ in range(60)]
    assert all(VALIDATOR.satisfies_format(fmt, v) for v in values), [v for v in values if not VALIDATOR.satisfies_format(fmt, v)]
    assert len(set(values)) > 1


def test_formats_come_first_then_declared_values_then_typed_values() -> None:
    # (a): a Format generator wins over a non-conforming example.
    emails = _docs({"type": "string", "format": "email", "examples": ["not an email"]})
    assert all(VALIDATOR.satisfies_format("email", v) and v != "not an email" for v in emails)
    # (b): declared examples and defaults are used when they satisfy the subschema.
    assert set(_docs({"type": "string", "examples": ["x", "y"], "default": "z"})) <= {"x", "y", "z"}
    assert set(_docs({"type": "integer", "examples": ["x", 4], "maximum": 3, "default": 2})) == {2}
    # (b): enum values respect the constraints and const is a one-value enum.
    assert set(_docs({"enum": ["p", "q", 1], "type": "string"})) <= {"p", "q"}
    assert set(_docs({"const": 5})) == {5}
    # (c): typed values respect bounds, multiples, lengths, and patterns.
    for value in _docs({"type": "integer", "minimum": 3, "maximum": 9, "multipleOf": 3}):
        assert value in (3, 6, 9)
    for value in _docs({"type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 0.5}):
        assert 0 < value < 0.5
    for value in _docs({"type": "string", "minLength": 2, "maxLength": 3}):
        assert 2 <= len(value) <= 3
    for value in _docs({"type": "string", "pattern": "^[a-c]{2}[0-9]$"}):
        assert VALIDATOR.matches_pattern("^[a-c]{2}[0-9]$", value)


def test_same_name_fields_of_the_given_schemas_supply_values() -> None:
    other = _model({"properties": {"email_address": {"examples": ["one@example.com", "two@example.com"]}, "n": {"enum": [1, 2]}}})
    index = ValueIndex.build([("other", other)])
    assert index.lookup(("email", "address")) == ("one@example.com", "two@example.com")
    assert index.lookup(("n",)) == (1, 2) and index.lookup(("missing",)) == () and len(index) == 2
    docs = _docs({"type": "object", "required": ["emailAddress"], "properties": {"emailAddress": {"type": "string"}}}, index=index)
    assert {d["emailAddress"] for d in docs} <= {"one@example.com", "two@example.com"}
    # Without the index, the field gets typed strings instead.
    plain = _docs({"type": "object", "required": ["emailAddress"], "properties": {"emailAddress": {"type": "string"}}})
    assert any(d["emailAddress"] not in ("one@example.com", "two@example.com") for d in plain)


def test_enum_skew_favours_the_first_declared_values() -> None:
    doc = {"enum": ["first", "second", "third", "fourth"]}
    cfg = GeneratorConfig(size_min=200, size_max=200, enum_skew_min=3.0, enum_skew_max=3.0)
    counts = Counter(_docs(doc, cfg=cfg))
    assert counts["first"] > counts["second"] > counts["fourth"]
    flat = Counter(_docs(doc, cfg=GeneratorConfig(size_min=200, size_max=200, enum_skew_min=0.0, enum_skew_max=0.0)))
    assert all(counts[v] > 0 for v in doc["enum"]) and flat["first"] < counts["first"]


def test_presence_rates_drive_optional_fields() -> None:
    doc = {"type": "object", "properties": {"a": {"type": "integer"}}}
    always = GeneratorConfig(size_min=20, size_max=20, presence_min=1.0, presence_max=1.0)
    never = GeneratorConfig(size_min=20, size_max=20, presence_min=0.0, presence_max=0.0)
    assert all("a" in d for d in _docs(doc, cfg=always))
    assert all("a" not in d for d in _docs(doc, cfg=never))


def test_structural_keywords_are_honoured() -> None:
    docs = _docs(
        {
            "type": "object",
            "required": ["tuple", "map", "closed", "uniq", "has"],
            "properties": {
                "tuple": {"prefixItems": [{"const": 1}, {"type": "string"}], "items": False, "minItems": 2},
                "map": {"additionalProperties": {"type": "boolean"}, "minProperties": 2, "maxProperties": 3},
                "closed": {"properties": {"x": {"type": "null"}}, "additionalProperties": False, "required": ["x"]},
                "uniq": {"items": {"type": "integer", "minimum": 0, "maximum": 2}, "uniqueItems": True},
                "has": {"items": {"type": "integer"}, "contains": {"const": 7}, "maxItems": 3},
                "named": {"propertyNames": {"pattern": "^k[0-9]$"}, "additionalProperties": {"type": "null"}, "minProperties": 1},
            },
            "additionalProperties": False,
        }
    )
    for d in docs:
        assert d["tuple"][0] == 1 and isinstance(d["tuple"][1], str) and len(d["tuple"]) == 2
        assert 2 <= len(d["map"]) <= 3 and all(isinstance(v, bool) for v in d["map"].values())
        assert d["closed"] == {"x": None}
        assert len(set(d["uniq"])) == len(d["uniq"]) and set(d["uniq"]) <= {0, 1, 2}
        assert 7 in d["has"] and len(d["has"]) <= 3
        if "named" in d:
            assert all(VALIDATOR.matches_pattern("^k[0-9]$", k) for k in d["named"])


def test_documents_are_canonical_values() -> None:
    docs = _docs({"type": "number", "multipleOf": 1.0})
    assert docs and all(isinstance(v, int) for v in docs)  # integral floats are canonical ints
    assert canonical_bytes(docs) == canonical_bytes(docs)


# -- pattern sampler -----------------------------------------------------------


@pytest.mark.parametrize(
    "pattern",
    [
        "^[A-Z]{2}-\\d{3}$",
        "\\w+@\\w+\\.com",
        "(?<year>\\d{4})-(?<m>\\d\\d)",
        "^(foo|bar)+$",
        "[^abc]x",
        "a{2,}b*c?",
        "^\\s*$",
        "^\\D\\W\\S$",
        "^[a-z][a-z0-9_]{0,5}$",
        "colou?r",
        ".+",
        "^v\\d+(\\.\\d+)*$",
    ],
)
def test_sampled_strings_match_their_pattern(pattern: str) -> None:
    rng = np.random.default_rng(2)
    for _ in range(20):
        text = sample_pattern(pattern, rng, VALIDATOR)
        assert text is not None and VALIDATOR.matches_pattern(pattern, text), (pattern, text)


@pytest.mark.parametrize("pattern", ["(a)\\1", "(", "^a$b"])
def test_unsupported_invalid_and_unsatisfiable_patterns_give_none(pattern: str) -> None:
    assert sample_pattern(pattern, np.random.default_rng(0), VALIDATOR) is None


@pytest.mark.parametrize("pattern", ["(?=a)a", "a(?!b)", "^a(?!b)[a-c]$", "(?<=x)y|z"])
def test_lookarounds_are_sampled_and_checked(pattern: str) -> None:
    rng = np.random.default_rng(0)
    found = [sample_pattern(pattern, rng, VALIDATOR) for _ in range(10)]
    assert all(text is None or VALIDATOR.matches_pattern(pattern, text) for text in found)
    assert any(text is not None for text in found)
    docs = _docs({"type": "string", "pattern": pattern})
    assert docs and all(VALIDATOR.matches_pattern(pattern, v) for v in docs)
