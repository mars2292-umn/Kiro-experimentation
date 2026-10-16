"""Unit tests for the model bits of mdl/scorer.py (task 10.2; Requirement 19.1)."""

from __future__ import annotations

import copy
import math
import random
import re
from typing import Any

import pytest

from jsinfer.core.schema_model import SchemaNode
from jsinfer.io.load import SUPPORTED_KEYWORDS
from jsinfer.mdl.codes import BOUND, LN0, STR, SUB, U, UNI
from jsinfer.mdl.scorer import (
    BOUND_KEYWORDS,
    CODED_KEYWORDS,
    KEYWORD_VOCABULARY_SIZE,
    UNCODED_KEYWORDS,
    MDLScorer,
    M_re,
    model_bits,
    regex_bits,
)
from jsinfer.values.patterns import PLUS, Alternative, CharClass, Pattern, Quantifier, Token

SLOT = math.log2(37)  # UNI(|KW|)
EMPTY = LN0(0)  # M({}) and M(true)
FALSE = (LN0(1), SLOT, LN0(0))  # M(false) = M({"not": {}})
PHONE = Pattern(
    (
        Alternative(
            (Token(CharClass.DIGIT, Quantifier.exactly(3)), Token("-"), Token(CharClass.DIGIT, Quantifier.exactly(4)))
        ),
    )
)
TYPE_STRING = (LN0(1), SLOT, SUB(1, 7))  # M({"type": "string"})


def M(schema: Any, **kwargs: Any) -> float:
    return model_bits(SchemaNode.from_json(schema), **kwargs)


def one_keyword(*terms: float) -> float:
    """M of a node with one coded keyword whose M_kw is the sum of ``terms``."""
    return math.fsum((LN0(1), SLOT, *terms))


# ---------------------------------------------------------------------------
# Vocabulary
# ---------------------------------------------------------------------------


def test_vocabulary() -> None:
    assert KEYWORD_VOCABULARY_SIZE == 37
    assert SUPPORTED_KEYWORDS <= CODED_KEYWORDS
    assert CODED_KEYWORDS - SUPPORTED_KEYWORDS == {"x-latent-type", "x-array-pattern"}
    assert not CODED_KEYWORDS & UNCODED_KEYWORDS
    assert BOUND_KEYWORDS <= SUPPORTED_KEYWORDS and len(BOUND_KEYWORDS) == 10
    assert {"title", "description", "x-provenance", "$schema", "$id"} <= UNCODED_KEYWORDS


# ---------------------------------------------------------------------------
# M_re
# ---------------------------------------------------------------------------


def quantifier_terms(low: int, high: int | None) -> tuple[float, ...]:
    return (1.0, LN0(low)) if high is None else (1.0, LN0(low), LN0(high - low))


def test_m_re_known_value() -> None:
    expected = math.fsum(
        (
            LN0(1),  # one alternative
            LN0(3),  # three tokens
            *(1.0, UNI(5), *quantifier_terms(3, 3)),  # [0-9]{3}
            *(1.0, 8.0, *quantifier_terms(1, 1)),  # -
            *(1.0, UNI(5), *quantifier_terms(4, 4)),  # [0-9]{4}
        )
    )
    assert M_re(PHONE) == expected
    assert regex_bits(PHONE.source) == expected


def test_m_re_alternatives_quantifiers_and_literal_bytes() -> None:
    p = Pattern(
        (
            Alternative((Token(CharClass.LOWER, PLUS),)),
            Alternative(),
            Alternative((Token("😀", Quantifier(2, 5)), Token("é"))),
        )
    )
    expected = math.fsum(
        (
            LN0(3),
            *(LN0(1), 1.0, UNI(5), *quantifier_terms(1, None)),
            LN0(0),
            *(LN0(2), 1.0, 32.0, *quantifier_terms(2, 5), 1.0, 16.0, *quantifier_terms(1, 1)),
        )
    )
    assert M_re(p) == expected
    assert M_re(Pattern((Alternative(),))) == math.fsum((LN0(1), LN0(0)))


def test_m_re_charges_for_longer_runs_and_wider_ranges() -> None:
    def single(q: Quantifier) -> float:
        return M_re(Pattern((Alternative((Token(CharClass.DIGIT, q),)),)))

    assert single(Quantifier(1, 1)) < single(Quantifier(1, 3)) < single(Quantifier(1, 300))
    assert single(Quantifier(2, 2)) < single(Quantifier(200, 200))


def test_regex_bits_falls_back_to_the_string_code() -> None:
    for source in ("^a+$", "^(?:[a-z][a-z])$(?!\\n)", "", "[0-9]{3}"):
        assert regex_bits(source) == STR(source)


def test_pattern_codes_reject_wrong_types() -> None:
    with pytest.raises(TypeError):
        M_re(PHONE.source)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        regex_bits(PHONE)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Per-keyword model bits
# ---------------------------------------------------------------------------


def test_empty_and_boolean_schemas() -> None:
    assert M({}) == EMPTY
    assert M(True) == M({}) == LN0(0)
    assert M(False) == M({"not": {}}) == math.fsum(FALSE)
    assert 0.0 < M(True) < M(False)


@pytest.mark.parametrize(
    ("schema", "terms"),
    [
        ({"type": "string"}, (SUB(1, 7),)),
        ({"type": ["string", "null", "string"]}, (SUB(2, 7),)),
        ({"type": []}, (SUB(0, 7),)),
        ({"properties": {"a": {}, "bc": True}}, (LN0(2), STR("a"), EMPTY, STR("bc"), EMPTY)),
        ({"properties": {"a": False}}, (LN0(1), STR("a"), *FALSE)),
        ({"additionalProperties": False}, (1.0,)),
        ({"additionalProperties": True}, (1.0,)),
        ({"additionalProperties": {"type": "string"}}, (1.0, *TYPE_STRING)),
        ({"items": False}, (1.0,)),
        ({"items": {}}, (1.0, EMPTY)),
        ({"patternProperties": {PHONE.source: {}, "^x$": False}}, (LN0(2), M_re(PHONE), EMPTY, STR("^x$"), *FALSE)),
        ({"prefixItems": [{}, False, {"type": "string"}]}, (LN0(3), EMPTY, *FALSE, *TYPE_STRING)),
        ({"enum": [1, 1.0, "a", None, {"k": [True]}]}, (LN0(4), U(1), U("a"), U(None), U({"k": [True]}))),
        ({"enum": []}, (LN0(0),)),
        ({"const": {"a": [1, 2.5]}}, (U({"a": [1, 2.5]}),)),
        ({"format": "email"}, (UNI(10),)),
        ({"pattern": PHONE.source}, (M_re(PHONE),)),
        ({"pattern": "^[a-z]+$"}, (STR("^[a-z]+$"),)),
        ({"uniqueItems": True}, ()),
        ({"$ref": "#"}, (0.0,)),
        ({"x-latent-type": "date"}, (UNI(6),)),
        (
            {"x-array-pattern": {"prefix": ["string"], "repeat": ["integer"]}},
            (U({"prefix": ["string"], "repeat": ["integer"]}),),
        ),
        ({"x-key": ["id"]}, ()),
        ({"x-references": "$.users[*].id"}, ()),
    ],
)
def test_single_keyword_costs(schema: dict[str, Any], terms: tuple[float, ...]) -> None:
    assert M(schema) == one_keyword(*terms)


@pytest.mark.parametrize("keyword", ["propertyNames", "contains", "not", "if", "then", "else"])
def test_single_subschema_keywords_cost_the_subschema(keyword: str) -> None:
    assert M({keyword: {"type": "string"}}) == one_keyword(*TYPE_STRING)
    assert M({keyword: False}) == one_keyword(*FALSE)
    assert M({keyword: True}) == one_keyword(EMPTY)


@pytest.mark.parametrize("keyword", ["anyOf", "oneOf", "allOf"])
def test_branch_keywords(keyword: str) -> None:
    assert M({keyword: [{"type": "string"}, True, False]}) == one_keyword(LN0(3), *TYPE_STRING, EMPTY, *FALSE)


@pytest.mark.parametrize("keyword", sorted(BOUND_KEYWORDS))
@pytest.mark.parametrize("value", [0, 255, 0.5, -2147483648, 1e3])
def test_bounds(keyword: str, value: int | float) -> None:
    assert M({keyword: value}) == one_keyword(BOUND(value))


def test_required_is_a_subset_of_the_properties() -> None:
    props: dict[str, Any] = {"a": {}, "b": {}, "c": {}}
    props_terms = (LN0(3), STR("a"), EMPTY, STR("b"), EMPTY, STR("c"), EMPTY)
    expected = math.fsum((LN0(2), SLOT, *props_terms, SLOT, SUB(2, 3)))
    assert M({"properties": props, "required": ["a", "c"]}) == expected
    assert M({"properties": props, "required": ["c", "a", "c"]}) == expected


def test_required_names_outside_the_properties_are_literals() -> None:
    assert M({"required": ["a"]}) == one_keyword(SUB(0, 0), LN0(1), STR("a"))
    expected = math.fsum((LN0(2), SLOT, LN0(1), STR("a"), EMPTY, SLOT, SUB(1, 1), LN0(1), STR("zz")))
    assert M({"properties": {"a": {}}, "required": ["zz", "a"]}) == expected


def test_dependent_required() -> None:
    props: dict[str, Any] = {"a": {}, "b": {}, "c": {}}
    props_terms = (LN0(3), STR("a"), EMPTY, STR("b"), EMPTY, STR("c"), EMPTY)
    schema = {"properties": props, "dependentRequired": {"a": ["b"], "c": ["a", "b", "a"]}}
    expected = math.fsum((LN0(2), SLOT, *props_terms, SLOT, LN0(2), UNI(3), SUB(1, 2), UNI(3), SUB(2, 2)))
    assert M(schema) == expected
    # An antecedent outside the properties is a literal and leaves all p names as consequents;
    # a self-dependency is a name outside the subset.
    odd = {"properties": props, "dependentRequired": {"x": ["a"], "b": ["b"]}}
    odd_terms = (LN0(2), STR("x"), SUB(1, 3), UNI(3), SUB(0, 2), LN0(1), STR("b"))
    assert M(odd) == math.fsum((LN0(2), SLOT, *props_terms, SLOT, *odd_terms))


def test_ref_picks_one_of_the_root_definitions() -> None:
    schema = {"$defs": {"x": {}, "y": {"$ref": "#/$defs/x"}}, "$ref": "#/$defs/y"}
    expected = math.fsum((LN0(2), SLOT, LN0(2), EMPTY, LN0(1), SLOT, UNI(2), SLOT, UNI(2)))
    assert M(schema) == expected
    # Definition names are derived and cost nothing.
    renamed = {"$defs": {"a-much-longer-name": {}, "b": {"$ref": "#/$defs/a-much-longer-name"}}, "$ref": "#/$defs/b"}
    assert M(renamed) == expected
    # A subschema scored alone takes D from n_defs, and D = 0 without it.
    assert M({"$ref": "#/$defs/a"}) == one_keyword(0.0)
    assert M({"$ref": "#/$defs/a"}, n_defs=4) == one_keyword(2.0)
    assert MDLScorer(n_defs=4).model_bits(SchemaNode.from_json({"$ref": "#/$defs/a"})) == one_keyword(2.0)
    assert MDLScorer(n_defs=8).model_bits(SchemaNode.from_json(schema)) == math.fsum(
        (LN0(2), SLOT, LN0(2), EMPTY, LN0(1), SLOT, 3.0, SLOT, 3.0)
    )


def test_unknown_keywords_are_rejected() -> None:
    for keyword in ("multipleOf", "unevaluatedProperties", "definitions", "$anchor"):
        with pytest.raises(ValueError, match=re.escape(keyword)):
            M({keyword: 1})


@pytest.mark.parametrize(
    "schema",
    [
        {"type": "str"},
        {"type": 3},
        {"required": "a"},
        {"required": [1]},
        {"enum": "a"},
        {"enum": [float("nan")]},
        {"format": 1},
        {"$ref": 1},
        {"uniqueItems": 1},
        {"minimum": True},
        {"maxLength": "3"},
        {"dependentRequired": {"a": "b"}},
        {"dependentRequired": []},
        {"const": {1, 2}},
    ],
)
def test_malformed_values_are_rejected(schema: dict[str, Any]) -> None:
    node = SchemaNode(kw=schema)
    with pytest.raises((TypeError, ValueError)):
        model_bits(node)


# ---------------------------------------------------------------------------
# Annotations, composition, and order
# ---------------------------------------------------------------------------

RICH: dict[str, Any] = {
    "type": ["object", "null"],
    "properties": {
        "id": {"type": "integer", "minimum": 1, "maximum": 4294967295},
        "email": {"type": "string", "format": "email", "maxLength": 254},
        "phone": {"type": "string", "pattern": PHONE.source},
        "tags": {"type": "array", "items": {"enum": ["a", "b", 3, None]}, "uniqueItems": True, "maxItems": 10},
        "pair": {"prefixItems": [{"type": "number"}, {"const": [1, {"x": 2}]}], "items": False},
        "kind": {"anyOf": [{"type": "string", "x-latent-type": "date"}, False, {"not": {"type": "null"}}]},
        "meta": {"type": "object", "patternProperties": {"^x-": {}}, "propertyNames": {"maxLength": 5}},
        "next": {"$ref": "#/$defs/node"},
    },
    "required": ["id", "email", "kind"],
    "dependentRequired": {"phone": ["email", "id"], "pair": ["tags"]},
    "additionalProperties": False,
    "$defs": {"node": {"type": "object", "properties": {"v": True}, "oneOf": [{"required": ["v"]}, True]}},
}


def annotate(schema: Any) -> Any:
    """Add every uncoded annotation to every keyword schema in the tree."""
    if isinstance(schema, bool):
        return schema
    out: dict[str, Any] = {}
    for key, value in schema.items():
        if key in {"properties", "patternProperties", "$defs"}:
            out[key] = {name: annotate(child) for name, child in value.items()}
        elif key in {"prefixItems", "anyOf", "oneOf", "allOf"}:
            out[key] = [annotate(child) for child in value]
        elif key in {"items", "additionalProperties", "propertyNames", "contains", "not", "if", "then", "else"}:
            out[key] = annotate(value)
        else:
            out[key] = value
    out.update(
        {
            "title": "t",
            "description": "a long description " * 20,
            "$comment": "c",
            "default": [1, 2, 3],
            "examples": [{"a": 1}],
            "deprecated": True,
            "readOnly": False,
            "writeOnly": False,
            "x-provenance": {"technique": "values.bounds", "dl": 12.5},
        }
    )
    return out


def shuffled(schema: Any, rng: random.Random) -> Any:
    """The same schema with every object's members, and every set-compared list, in random order."""
    if isinstance(schema, list):
        return [shuffled(item, rng) for item in schema]
    if not isinstance(schema, dict):
        return schema
    items = [(key, shuffled(value, rng)) for key, value in schema.items()]
    for index, (key, value) in enumerate(items):
        if key in {"required", "enum"} or (key == "type" and isinstance(value, list)):
            value = list(value)
            rng.shuffle(value)
            items[index] = (key, value)
    rng.shuffle(items)
    return dict(items)


def test_annotations_and_provenance_do_not_change_model_bits() -> None:
    annotated = annotate(RICH)
    annotated["$schema"] = "https://json-schema.org/draft/2020-12/schema"
    annotated["$id"] = "https://example.com/s.json"
    assert M(annotated) == M(RICH)


def test_model_bits_are_independent_of_member_and_list_order() -> None:
    base = M(RICH)
    rng = random.Random(20251016)
    for _ in range(50):
        assert M(shuffled(RICH, rng)) == base


def test_model_bits_compose_over_subschemas() -> None:
    root = SchemaNode.from_json(RICH)
    child = root.kw["properties"]["tags"]
    with_child = model_bits(root)
    without_child = copy.deepcopy(root)
    without_child.kw["properties"]["tags"] = SchemaNode()
    assert math.isclose(with_child - model_bits(without_child), model_bits(child) - EMPTY, rel_tol=1e-12)
    # A keyword schema under items costs the flag bit plus its own bits.
    items = child.kw["items"]
    assert math.isclose(
        model_bits(child)
        - model_bits(SchemaNode.from_json({"type": "array", "items": False, "uniqueItems": True, "maxItems": 10})),
        model_bits(items),
        rel_tol=1e-12,
    )


def test_model_bits_are_finite_non_negative_and_deterministic() -> None:
    schemas: tuple[Any, ...] = (True, False, {}, RICH, annotate(RICH), {"properties": {"": {}}})
    for schema in schemas:
        bits = M(schema)
        assert isinstance(bits, float) and math.isfinite(bits) and bits >= 0.0
        assert M(schema) == bits


def test_scorer_and_function_agree() -> None:
    node = SchemaNode.from_json(RICH)
    assert MDLScorer().model_bits(node) == model_bits(node)
    assert MDLScorer().n_defs is None and MDLScorer(n_defs=3).n_defs == 3
    assert MDLScorer(n_defs=1).model_bits(node) == model_bits(node, n_defs=1) == model_bits(node)


@pytest.mark.parametrize(("n_defs", "error"), [(-1, ValueError), (True, TypeError), (1.0, TypeError), ("2", TypeError)])
def test_bad_n_defs(n_defs: Any, error: type[Exception]) -> None:
    with pytest.raises(error):
        MDLScorer(n_defs=n_defs)
    with pytest.raises(error):
        model_bits(SchemaNode(), n_defs=n_defs)


def test_model_bits_reject_non_nodes() -> None:
    with pytest.raises(TypeError):
        model_bits({"type": "string"})  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        MDLScorer().model_bits(True)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Structure
# ---------------------------------------------------------------------------


def test_deep_nesting_does_not_recurse() -> None:
    depth = 20_000
    node = SchemaNode()
    for _ in range(depth):
        node = SchemaNode(kw={"not": node})
    expected = math.fsum([*([LN0(1), SLOT] * depth), EMPTY])
    assert model_bits(node) == expected
    items = SchemaNode(boolean=False)
    for _ in range(depth):
        items = SchemaNode(kw={"items": items})
    assert model_bits(items) == math.fsum([*([LN0(1), SLOT, 1.0] * depth)])


def test_shared_and_cyclic_nodes_are_rejected() -> None:
    shared = SchemaNode.from_json({"type": "string"})
    with pytest.raises(ValueError, match="reached twice"):
        model_bits(SchemaNode(kw={"anyOf": [shared, shared]}))
    loop = SchemaNode()
    loop.kw["not"] = SchemaNode(kw={"items": loop})
    with pytest.raises(ValueError, match="reached twice"):
        model_bits(loop)


def test_malformed_containers_are_rejected() -> None:
    for keyword, value in (("properties", []), ("anyOf", {}), ("not", {}), ("items", True), ("$defs", {"a": 1})):
        node = SchemaNode()
        node.kw[keyword] = value  # bypasses the constructor's shape check
        with pytest.raises(TypeError):
            model_bits(node)
