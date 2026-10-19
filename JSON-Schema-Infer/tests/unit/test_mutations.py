"""Unit tests for the Mutation_Operators and Negative_Document generation (task 26.1; Requirement 32)."""

from __future__ import annotations

import copy
from typing import Any

import pytest

from jsinfer.io.validator import Validator
from jsinfer.learn.mutations import OPERATORS, MutationError, Negative, holder_of, mutate, negatives, sites

VALIDATOR = Validator()
OBJ = {"type": "object"}
SCHEMA: dict[str, Any] = {
    **OBJ,
    "properties": {
        "id": {"type": "string", "format": "uuid"},
        "name": {"type": "string", "minLength": 1, "maxLength": 8, "pattern": "^[a-z]*$"},
        "kind": {"enum": ["a", "b"]},
        "n": {"type": "integer", "minimum": 1, "maximum": 9, "multipleOf": 3},
        "tags": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 3, "uniqueItems": True},
        "pair": {"type": "array", "prefixItems": [{"type": "string"}, {"type": "integer"}], "items": False},
        "street": {"type": "string"},
        "city": {"type": "string"},
        "x": {"type": "integer"},
        "y": {"type": "integer"},
        "u": {
            "oneOf": [
                {**OBJ, "properties": {"t": {"const": "p"}, "w": {"type": "integer"}}, "required": ["t", "w"], "additionalProperties": False},
                {**OBJ, "properties": {"t": {"enum": ["q"]}, "h": {"type": "string"}}, "required": ["t", "h"], "additionalProperties": False},
            ]
        },
        "g": {**OBJ, "properties": {"a": {}, "b": {}, "c": {}}, "oneOf": [{"required": ["a"]}, {"required": ["b", "c"]}]},
    },
    "required": ["id", "kind"],
    "additionalProperties": False,
    "dependentRequired": {"street": ["city"]},
    "not": {"required": ["x", "y"]},
    "minProperties": 2,
}


def test_every_operator_has_its_sites_in_the_example() -> None:
    expected = {
        "remove_required_field": [
            "/required/0", "/required/1",
            "/properties/u/oneOf/0/required/0", "/properties/u/oneOf/0/required/1",
            "/properties/u/oneOf/1/required/0", "/properties/u/oneOf/1/required/1",
            "/properties/g/oneOf/0/required/0", "/properties/g/oneOf/1/required/0", "/properties/g/oneOf/1/required/1",
        ],
        "add_undeclared_property": ["/additionalProperties", "/properties/u/oneOf/0/additionalProperties", "/properties/u/oneOf/1/additionalProperties"],
        "enum_non_member": ["/properties/kind/enum", "/properties/u/oneOf/0/properties/t/const", "/properties/u/oneOf/1/properties/t/enum"],
        "violate_pattern": ["/properties/name/pattern"],
        "violate_format": ["/properties/id/format"],
        "violate_numeric_bound": ["/properties/n/minimum", "/properties/n/maximum", "/properties/n/multipleOf"],
        "violate_length_bound": ["/properties/name/minLength", "/properties/name/maxLength"],
        "violate_count_bound": ["/minProperties", "/properties/tags/minItems", "/properties/tags/maxItems"],
        "reorder_tuple": ["/properties/pair/prefixItems"],
        "mismatch_discriminator": ["/properties/u/oneOf/0/properties/t", "/properties/u/oneOf/1/properties/t"],
        "break_cooccurrence": ["/dependentRequired/street"],
        "combine_exclusive": ["/not", "/properties/g/oneOf"],
        "duplicate_element": ["/properties/tags/uniqueItems"],
    }
    for operator, found in expected.items():
        assert sites(operator, SCHEMA) == found, operator
    assert len(sites("change_type", SCHEMA)) == 18
    with pytest.raises(ValueError):
        sites("nope", SCHEMA)


def differs_only_at(original: Any, mutated: Any, holder: str) -> bool:
    """Whether ``mutated`` equals ``original`` everywhere outside the schema object at ``holder`` (32.2)."""
    from jsinfer.core.schema_model import split_pointer

    def without(document: Any) -> Any:
        document = copy.deepcopy(document)
        tokens = split_pointer(holder)
        if not tokens:
            return None
        node = document
        for token in tokens[:-1]:
            node = node[int(token)] if isinstance(node, list) else node[token]
        last = tokens[-1]
        if isinstance(node, list):
            node[int(last)] = None
        else:
            node[last] = None
        return document

    return without(original) == without(mutated) and original != mutated


@pytest.mark.parametrize("operator", OPERATORS)
def test_each_mutation_changes_one_site_and_yields_rejected_documents(operator: str) -> None:
    for site in sites(operator, SCHEMA):
        before = copy.deepcopy(SCHEMA)
        mutated = mutate(operator, SCHEMA, site)
        assert SCHEMA == before, "mutate changed its input"
        assert mutated != SCHEMA
        VALIDATOR.compile(mutated)
        holder = holder_of(SCHEMA, site)
        assert site.startswith(holder) and differs_only_at(SCHEMA, mutated, holder), f"{operator} at {site} (holder {holder})"
    report = negatives(SCHEMA, "example", seed=1, target=2, attempts=40, operators=[operator])
    assert report.skipped == [] and report.unproductive == {}
    assert len(report.negatives) >= 1 and all(n.operator == operator and n.schema_id == "example" for n in report.negatives)
    truth = VALIDATOR.compile(SCHEMA)
    for negative in report.negatives:
        assert not VALIDATOR.is_valid(truth, negative.document), f"{operator}: {negative.document!r} is accepted"
        assert negative.site in sites(operator, SCHEMA)


def test_inapplicable_sites_and_unknown_operators_are_errors() -> None:
    with pytest.raises(MutationError):
        mutate("violate_pattern", SCHEMA, "/properties/id/format")
    with pytest.raises(ValueError):
        negatives(SCHEMA, "x", operators=["nope"])
    with pytest.raises(ValueError):
        negatives(SCHEMA, "")


def test_skips_unproductive_operators_and_the_site_limit_are_recorded() -> None:
    schema = {"type": "integer", "minimum": 0}
    report = negatives(schema, "ints", seed=0, target=2, attempts=10)
    assert "violate_numeric_bound" not in report.skipped and report.per_operator()["violate_numeric_bound"] == 2
    assert set(report.skipped) == set(OPERATORS) - {"change_type", "violate_numeric_bound"}
    # An unsatisfiable mutation (nothing is both >= 0 and a non-number... here: a bound no generated value violates)
    stuck = {"type": "integer", "minimum": 0, "maximum": 0}
    stuck_report = negatives(stuck, "stuck", seed=0, target=1, attempts=4, operators=["violate_numeric_bound"])
    assert stuck_report.unproductive == {} or all(v <= 8 for v in stuck_report.unproductive.values())
    many = {"type": "object", "properties": {f"p{i}": {"type": "string"} for i in range(12)}, "required": [f"p{i}" for i in range(12)]}
    limited = negatives(many, "many", seed=3, target=1, attempts=5, site_limit=4, operators=["remove_required_field"])
    assert len(limited.mutated) == 4 and len({m.site for m in limited.mutated}) == 4
    assert negatives(many, "many", seed=3, target=1, attempts=5, site_limit=4, operators=["remove_required_field"]).mutated == limited.mutated
    other = negatives(many, "many", seed=4, target=1, attempts=5, site_limit=4, operators=["remove_required_field"])
    assert isinstance(other.mutated[0].site, str)


def test_reruns_are_identical_and_negatives_are_distinct() -> None:
    first = negatives(SCHEMA, "example", seed=7, target=4, attempts=60)
    second = negatives(SCHEMA, "example", seed=7, target=4, attempts=60)
    assert [n.to_json() for n in first.negatives] == [n.to_json() for n in second.negatives]
    assert first.mutated == second.mutated and first.attempts == second.attempts
    per_site: dict[tuple[str, str], list[Any]] = {}
    for negative in first.negatives:
        per_site.setdefault((negative.operator, negative.site), []).append(negative.document)
    for docs in per_site.values():
        assert len(docs) <= 4 and len({repr(d) for d in docs}) == len(docs)
    assert isinstance(first.negatives[0], Negative) and set(first.negatives[0].to_json()) == {"document", "operator", "schema_id", "site"}


def test_a_not_beside_an_existing_not_goes_into_allof() -> None:
    schema = {"type": "string", "pattern": "^a$", "not": {"const": "zzz"}}
    mutated = mutate("violate_pattern", schema, "/pattern")
    assert mutated == {"type": "string", "not": {"const": "zzz"}, "allOf": [{"not": {"pattern": "^a$"}}]}
