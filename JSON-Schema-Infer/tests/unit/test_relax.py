"""Unit tests for decide/relax.py (task 15.1).

Requirements 3.3 and 3.5: the candidates of the relaxation table, their
documented order, the ``not``/``oneOf``/``$defs`` monotonicity rules, and the
Changes that 3.6 records are built from. Monotonicity is spot-checked here by
validating fixed documents before and after each candidate; Property 8
(task 15.2) checks it over generated schemas.
"""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import REMOVED, RelaxationRecord
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine, DecisionPoint
from jsinfer.decide.relax import (
    FALSE_SCHEMA,
    RELAXATION,
    Failure,
    Relaxation,
    relaxations,
    removal,
)
from jsinfer.io.validator import Validator

V = Validator()


def model(schema: dict[str, Any] | bool) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(schema))


def failure(
    m: SchemaModel,
    pointer: str,
    keyword: str,
    values: list[JSON],
    *,
    item: str | None = None,
    branches: tuple[tuple[Failure, ...], ...] = (),
    documents: tuple[int, ...] = (0,),
) -> Failure:
    return Failure(m.resolve(pointer).nid, keyword, item, values, documents, branches)


def candidates(m: SchemaModel, f: Failure) -> tuple[Relaxation, ...]:
    return relaxations(m, f, validator=V)


def labels(rs: tuple[Relaxation, ...]) -> list[str]:
    return [r.label for r in rs]


def relaxed(r: Relaxation, m: SchemaModel) -> JSON:
    return r.applied(m)[0].root.to_json()


def valid(schema: JSON, instance: JSON) -> bool:
    return V.is_valid(V.compile(schema), instance)  # type: ignore[arg-type]


def assert_monotone(m: SchemaModel, rs: tuple[Relaxation, ...], documents: list[JSON]) -> None:
    """Every document valid against ``m`` stays valid after each candidate."""
    before = m.root.to_json()
    for r in rs:
        after = relaxed(r, m)
        for doc in documents:
            if valid(before, doc):
                assert valid(after, doc), f"{r.label} rejects {doc!r}, which validated before"


def assert_fixes(m: SchemaModel, r: Relaxation, documents: list[JSON]) -> None:
    after = relaxed(r, m)
    for doc in documents:
        assert valid(after, doc), f"{r.label} still rejects {doc!r}"


# ---------------------------------------------------------------------------
# Failure
# ---------------------------------------------------------------------------


def test_failure_normalizes_and_names_its_constraint() -> None:
    f = Failure(7, "type", None, [1, "a"], [3, 1, 3])
    assert f.values == (1, "a")
    assert f.documents == (1, 3)
    assert f.constraint == ConstraintId(7, "type")
    assert Failure(7, "required", "email").constraint == ConstraintId(7, "required", "email")
    assert Failure(7, "anyOf").constraint is None
    assert Failure(7, FALSE_SCHEMA).constraint is None


@pytest.mark.parametrize(
    ("args", "error"),
    [
        ((1, "properties"), ValueError),  # not a row of the table
        ((1, "required"), ValueError),  # an entry Failure needs its item
        ((1, "type", "x"), ValueError),  # a keyword-level Failure has no item
        ((True, "type"), TypeError),
        ((-1, "type"), ValueError),
    ],
)
def test_failure_rejects_malformed_targets(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        Failure(*args)


def test_failure_branches_only_for_combinators() -> None:
    inner = Failure(2, "type", None, [1])
    assert Failure(1, "oneOf", None, [1], (), ((inner,), ())).branches == ((inner,), ())
    with pytest.raises(ValueError, match="only anyOf and oneOf"):
        Failure(1, "type", None, [1], (), ((inner,),))


def test_relaxations_check_the_failure_against_the_model() -> None:
    m = model({"type": "integer", "required": ["a"], "anyOf": [{}, {}]})
    with pytest.raises(KeyError):
        candidates(m, Failure(10**9, "type"))
    with pytest.raises(ValueError, match="no 'maximum'"):
        candidates(m, Failure(m.root.nid, "maximum", None, [1]))
    with pytest.raises(ValueError, match="does not require"):
        candidates(m, Failure(m.root.nid, "required", "b", [{}]))
    with pytest.raises(ValueError, match="lists 1 branches"):
        candidates(m, Failure(m.root.nid, "anyOf", None, [1], (), ((),)))
    with pytest.raises(ValueError, match="boolean false"):
        candidates(m, Failure(m.root.nid, FALSE_SCHEMA, None, [1]))


# ---------------------------------------------------------------------------
# Value keywords
# ---------------------------------------------------------------------------


def test_type_adds_the_missing_types_with_the_integer_rule() -> None:
    m = model({"type": "integer"})
    rs = candidates(m, failure(m, "", "type", [1.5, "a"]))
    assert labels(rs) == ['add-types:type=["number","string"]', "remove:type"]
    assert relaxed(rs[0], m) == {"type": ["number", "string"]}
    assert relaxed(rs[1], m) == {}
    assert [r.order for r in rs] == [(0, 0), (5, 0)]
    assert_fixes(m, rs[0], [1.5, "a", 3])


def test_type_without_a_new_type_offers_only_removal() -> None:
    m = model({"type": ["integer", "string"], "minimum": 0})
    assert labels(candidates(m, failure(m, "", "type", [3]))) == ["remove:type"]


def test_enum_adds_values_in_canonical_order() -> None:
    m = model({"enum": ["b", 2]})
    rs = candidates(m, failure(m, "", "enum", ["a", 2.0, True]))
    assert labels(rs) == ["add-values:enum+2", "remove:enum"]
    assert relaxed(rs[0], m) == {"enum": [True, 2, "a", "b"]}
    assert_monotone(m, rs, ["a", "b", 2, True, 3])


def test_const_becomes_enum() -> None:
    m = model({"const": 3})
    rs = candidates(m, failure(m, "", "const", [4, 4]))
    assert labels(rs) == ["add-values:const->enum+1", "remove:const"]
    assert relaxed(rs[0], m) == {"enum": [3, 4]}


def test_const_beside_enum_is_only_removed() -> None:
    m = model({"const": 3, "enum": [3, 4]})
    assert labels(candidates(m, failure(m, "", "const", [4]))) == ["remove:const"]


@pytest.mark.parametrize("keyword", ["format", "uniqueItems", "contains", "propertyNames", "not"])
def test_remove_only_keywords(keyword: str) -> None:
    values: dict[str, Any] = {
        "format": "date",
        "uniqueItems": True,
        "contains": {"type": "string"},
        "propertyNames": {"maxLength": 1},
        "not": {"type": "integer"},
    }
    m = model({keyword: values[keyword], "title": "kept"})
    rs = candidates(m, failure(m, "", keyword, [1]))
    assert labels(rs) == [f"remove:{keyword}"]
    assert relaxed(rs[0], m) == {"title": "kept"}


def test_required_and_dependent_required_entries_are_removed_one_by_one() -> None:
    m = model({"required": ["a", "b"], "dependentRequired": {"a": ["b"], "c": ["a"]}})
    rs = candidates(m, failure(m, "", "required", [{"b": 1}], item="a"))
    assert labels(rs) == ["remove:required/a"]
    assert relaxed(rs[0], m) == {"required": ["b"], "dependentRequired": {"a": ["b"], "c": ["a"]}}
    rs = candidates(m, failure(m, "", "dependentRequired", [{"c": 1}], item="c"))
    assert relaxed(rs[0], m) == {"required": ["a", "b"], "dependentRequired": {"a": ["b"]}}
    last = model({"required": ["a"]})
    assert relaxed(candidates(last, failure(last, "", "required", [{}], item="a"))[0], last) == {}


# ---------------------------------------------------------------------------
# Bounds
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("schema", "keyword", "values", "expected"),
    [
        ({"maximum": 140}, "maximum", [150], 150),
        ({"maximum": 100}, "maximum", [137, 120], 137),
        ({"minimum": 0}, "minimum", [-5], -5),
        ({"maximum": 2.5}, "maximum", [2.75], 2.75),
        ({"exclusiveMaximum": 3}, "exclusiveMaximum", [3], 127),
        ({"exclusiveMinimum": 0}, "exclusiveMinimum", [0, 5], -128),
        ({"maxLength": 2}, "maxLength", ["abcde"], 5),
        ({"minItems": 2}, "minItems", [[1]], 1),
        ({"maxProperties": 1}, "maxProperties", [{"a": 1, "b": 2}], 2),
    ],
)
def test_bounds_loosen_to_the_next_looser_ladder_rung(
    schema: dict[str, Any], keyword: str, values: list[JSON], expected: int | float
) -> None:
    m = model(schema)
    rs = candidates(m, failure(m, "", keyword, values))
    assert [r.action for r in rs] == ["loosen-bound", "remove"]
    assert relaxed(rs[0], m) == {keyword: expected}
    assert_fixes(m, rs[0], values)
    assert_monotone(m, rs, [*values, -1000, -1, 0, 1, 3, 100, 2.5, "", "ab", [], [1, 2], {}, {"a": 1}])


def test_bounds_ignore_values_of_other_types() -> None:
    m = model({"maximum": 3})
    assert labels(candidates(m, failure(m, "", "maximum", ["not a number"]))) == ["remove:maximum"]


# ---------------------------------------------------------------------------
# Patterns
# ---------------------------------------------------------------------------


def test_pattern_reinduction_widens_a_safe_pattern() -> None:
    old = "^(?:[0-9]{4}-[0-9]{2})$(?!\\n)"
    m = model({"pattern": old})
    rs = candidates(m, failure(m, "", "pattern", ["2024-123", "ab", "2024-01"]))
    assert labels(rs) == ["re-induce:pattern", "remove:pattern"]
    new = relaxed(rs[0], m)["pattern"]  # type: ignore[index]
    assert new == "^(?:[0-9]{4}-[0-9]{2,3}|[a-z]{2})$(?!\\n)"
    for s in ["2024-01", "1999-12", "2024-123", "ab", "zz"]:
        assert V.matches_pattern(new, s)
    assert not V.matches_pattern(new, "2024-1234")
    assert_monotone(m, rs, ["2024-01", "2024-123", "ab", "x", "2024-1"])


def test_pattern_reinduction_keeps_digit_runs_open_for_numbers() -> None:
    m = model({"pattern": "^(?:[0-9]+)$(?!\\n)"})
    rs = candidates(m, failure(m, "", "pattern", ["-5"]))
    assert relaxed(rs[0], m) == {"pattern": "^(?:[0-9]+|-[0-9]+)$(?!\\n)"}


def test_pattern_reinduction_alternates_with_other_regexes() -> None:
    m = model({"pattern": "^a|z$"})
    rs = candidates(m, failure(m, "", "pattern", ["b1"]))
    new = relaxed(rs[0], m)["pattern"]  # type: ignore[index]
    assert new == "(?:^a|z$)|^(?:[a-z][0-9])$(?!\\n)"
    assert_monotone(m, rs, ["a", "abc", "xz", "b1", "q"])
    assert_fixes(m, rs[0], ["b1"])


def test_pattern_with_no_rejected_string_offers_only_removal() -> None:
    m = model({"pattern": "^a"})
    assert labels(candidates(m, failure(m, "", "pattern", ["abc", 3]))) == ["remove:pattern"]


# ---------------------------------------------------------------------------
# Closed records and closed tuples
# ---------------------------------------------------------------------------


def test_closed_record_adds_naive_merge_properties() -> None:
    m = model({"properties": {"a": {}}, "patternProperties": {"^x": {"type": "integer"}}, "additionalProperties": False})
    values: list[JSON] = [{"a": 1, "b": 2, "xy": 3}, {"b": "s", "c": None}]
    rs = candidates(m, failure(m, "", "additionalProperties", values))
    assert labels(rs) == ["add-properties:+2", "remove:additionalProperties"]
    assert relaxed(rs[0], m) == {
        "properties": {"a": {}, "b": {"type": ["integer", "string"]}, "c": {"type": ["null"]}},
        "patternProperties": {"^x": {"type": "integer"}},
        "additionalProperties": False,
    }
    assert relaxed(rs[1], m) == {"properties": {"a": {}}, "patternProperties": {"^x": {"type": "integer"}}}
    assert_fixes(m, rs[0], values)
    assert_monotone(m, rs, [*values, {"a": 1}, {"xa": 1}, {"xa": "s"}, {"b": [1]}, {}])


def test_closed_record_without_properties_gets_them() -> None:
    m = model({"type": "object", "additionalProperties": False})
    r = candidates(m, failure(m, "", "additionalProperties", [{"k": [1]}]))[0]
    relaxed_model, changes = r.applied(m)
    assert relaxed_model.root.to_json() == {
        "type": "object",
        "additionalProperties": False,
        "properties": {"k": {"type": ["array"], "items": {"type": ["integer"]}}},
    }
    nid = m.root.nid
    assert changes[0].before_ids == (ConstraintId(nid, "additionalProperties"),)
    assert changes[0].after_ids == (ConstraintId(nid, "additionalProperties"), ConstraintId(nid, "properties"))


def test_open_additional_properties_is_only_removed() -> None:
    m = model({"additionalProperties": {"type": "string"}})
    assert labels(candidates(m, failure(m, "", "additionalProperties", [{"a": 1}]))) == ["remove:additionalProperties"]


def test_closed_tuple_gets_naive_merge_items() -> None:
    m = model({"prefixItems": [{"type": "integer"}], "items": False})
    rs = candidates(m, failure(m, "", "items", [[1, 2, "a"], [3, None]]))
    assert labels(rs) == ["replace-items:items", "remove:items"]
    assert relaxed(rs[0], m) == {"prefixItems": [{"type": "integer"}], "items": {"type": ["null", "integer", "string"]}}
    assert_monotone(m, rs, [[1], [], [1, 2], ["a"], [1, "x"]])


def test_closed_tuple_without_extra_elements_is_only_removed() -> None:
    m = model({"prefixItems": [{}], "items": False})
    assert labels(candidates(m, failure(m, "", "items", [[1]]))) == ["remove:items"]


# ---------------------------------------------------------------------------
# anyOf and oneOf
# ---------------------------------------------------------------------------


def test_any_of_relaxes_each_failing_branch_or_adds_one() -> None:
    m = model({"anyOf": [{"type": "integer", "maximum": 3}, {"type": "string", "maxLength": 1}]})
    b0, b1 = m.root.kw["anyOf"]
    branches = (
        (Failure(b0.nid, "maximum", None, [5]),),
        (Failure(b1.nid, "type", None, [5]),),
    )
    rs = candidates(m, failure(m, "", "anyOf", [5], branches=branches))
    assert labels(rs) == ["relax-branch:anyOf/0", "relax-branch:anyOf/1", "add-branch:anyOf", "remove:anyOf"]
    assert [r.order for r in rs] == [(3, 0), (3, 1), (4, 0), (5, 0)]
    assert relaxed(rs[0], m) == {"anyOf": [{"type": "integer", "maximum": 5}, {"type": "string", "maxLength": 1}]}
    assert relaxed(rs[1], m) == {
        "anyOf": [{"type": "integer", "maximum": 3}, {"type": ["integer", "string"], "maxLength": 1}]
    }
    assert relaxed(rs[2], m) == {
        "anyOf": [{"type": "integer", "maximum": 3}, {"type": "string", "maxLength": 1}, {"type": ["integer"]}]
    }
    for r in rs:
        assert_fixes(m, r, [5])
    assert_monotone(m, rs, [1, 3, 4, "a", "ab", None])


def test_relax_branch_merges_failures_of_one_constraint() -> None:
    m = model({"anyOf": [{"type": "integer"}, {"type": "null"}]})
    b0 = m.root.kw["anyOf"][0]
    branches = ((Failure(b0.nid, "type", None, ["a"]), Failure(b0.nid, "type", None, [True])), ())
    r = candidates(m, failure(m, "", "anyOf", ["a", True], branches=branches))[0]
    assert r.label == "relax-branch:anyOf/0"
    assert relaxed(r, m) == {"anyOf": [{"type": ["boolean", "integer", "string"]}, {"type": "null"}]}


def test_one_of_offers_conversion_first() -> None:
    m = model({"oneOf": [{"type": "integer"}, {"minimum": 0}]})
    rs = candidates(m, failure(m, "", "oneOf", [5]))
    assert labels(rs) == ["convert:oneOf->anyOf", "add-branch:oneOf", "remove:oneOf"]
    assert relaxed(rs[0], m) == {"anyOf": [{"type": "integer"}, {"minimum": 0}]}
    assert relaxed(rs[1], m) == {"anyOf": [{"type": "integer"}, {"minimum": 0}, {"type": ["integer"]}]}
    assert_fixes(m, rs[0], [5])
    assert_monotone(m, rs, [5, -1, 1.5, "a"])


def test_one_of_with_no_matching_branch_skips_the_bare_conversion() -> None:
    m = model({"oneOf": [{"type": "integer", "maximum": 3}, {"type": "string"}]})
    b0, b1 = m.root.kw["oneOf"]
    branches = ((Failure(b0.nid, "maximum", None, [5]),), (Failure(b1.nid, "type", None, [5]),))
    rs = candidates(m, failure(m, "", "oneOf", [5], branches=branches))
    assert labels(rs) == ["relax-branch:oneOf/0", "relax-branch:oneOf/1", "add-branch:oneOf", "remove:oneOf"]
    for r in rs[:3]:
        assert "oneOf" not in relaxed(r, m)  # type: ignore[operator]
        assert_fixes(m, r, [5])
    assert_monotone(m, rs, [1, 4, "a", None])


def test_one_of_with_a_matching_branch_offers_the_bare_conversion() -> None:
    m = model({"oneOf": [{"type": "integer"}, {"minimum": 0}, {"type": "string"}]})
    b2 = m.root.kw["oneOf"][2]
    branches = ((), (), (Failure(b2.nid, "type", None, [5]),))
    rs = candidates(m, failure(m, "", "oneOf", [5], branches=branches))
    assert labels(rs) == ["convert:oneOf->anyOf", "relax-branch:oneOf/2", "add-branch:oneOf", "remove:oneOf"]


def test_one_of_beside_any_of_converts_into_all_of() -> None:
    m = model({"anyOf": [{"type": "integer"}, {"type": "string"}], "oneOf": [{"minimum": 0}, {"maximum": 10}]})
    r = candidates(m, failure(m, "", "oneOf", [5]))[0]
    relaxed_model, changes = r.applied(m)
    assert relaxed_model.root.to_json() == {
        "anyOf": [{"type": "integer"}, {"type": "string"}],
        "allOf": [{"anyOf": [{"minimum": 0}, {"maximum": 10}]}],
    }
    (change,) = changes
    root, wrapper = m.root.nid, relaxed_model.root.kw["allOf"][0]
    assert change.before == {"oneOf": [{"minimum": 0}, {"maximum": 10}]}
    assert change.after == {"allOf": [{"anyOf": [{"minimum": 0}, {"maximum": 10}]}]}
    assert change.before_ids == (ConstraintId(root, "oneOf", 0), ConstraintId(root, "oneOf", 1))
    # The owner's new allOf is a Constraint the Change creates, listed with the wrapper's branches.
    assert change.after_ids == (
        ConstraintId(root, "allOf"),
        ConstraintId(wrapper.nid, "anyOf", 0),
        ConstraintId(wrapper.nid, "anyOf", 1),
    )
    assert_monotone(m, (r,), [5, -1, 11, "a", 5.5])


def test_one_of_beside_any_of_extends_an_existing_all_of() -> None:
    m = model(
        {
            "anyOf": [{"type": "integer"}, {"type": "string"}],
            "allOf": [{"type": "integer"}],
            "oneOf": [{"minimum": 0}, {"maximum": 10}],
        }
    )
    r = candidates(m, failure(m, "", "oneOf", [5]))[0]
    relaxed_model, changes = r.applied(m)
    assert relaxed_model.root.to_json() == {
        "anyOf": [{"type": "integer"}, {"type": "string"}],
        "allOf": [{"type": "integer"}, {"anyOf": [{"minimum": 0}, {"maximum": 10}]}],
    }
    (change,) = changes
    root, wrapper = m.root.nid, relaxed_model.root.kw["allOf"][1]
    # The existing allOf is modified, so both fragments and both id lists name it.
    assert change.before == {"allOf": [{"type": "integer"}], "oneOf": [{"minimum": 0}, {"maximum": 10}]}
    assert change.after == {"allOf": [{"type": "integer"}, {"anyOf": [{"minimum": 0}, {"maximum": 10}]}]}
    assert change.before_ids == (
        ConstraintId(root, "allOf"),
        ConstraintId(root, "oneOf", 0),
        ConstraintId(root, "oneOf", 1),
    )
    assert change.after_ids == (
        ConstraintId(root, "allOf"),
        ConstraintId(wrapper.nid, "anyOf", 0),
        ConstraintId(wrapper.nid, "anyOf", 1),
    )
    assert_monotone(m, (r,), [5, -1, 11, "a", 5.5])


def test_conversion_rewrites_references_into_the_branches() -> None:
    m = model({"properties": {"a": {"$ref": "#/oneOf/0"}}, "oneOf": [{"type": "object"}, {"minProperties": 1}]})
    r = candidates(m, failure(m, "", "oneOf", [{"a": {}}]))[0]
    after = relaxed(r, m)
    assert after == {"properties": {"a": {"$ref": "#/anyOf/0"}}, "anyOf": [{"type": "object"}, {"minProperties": 1}]}
    assert valid(after, {"a": {}})
    assert not valid(after, {"a": 1})


# ---------------------------------------------------------------------------
# Monotonicity guards
# ---------------------------------------------------------------------------


def test_edit_inside_nested_one_of_branches_converts_each_enclosing_one_of() -> None:
    m = model({"oneOf": [{"properties": {"a": {"oneOf": [{"maximum": 3}, {"type": "string"}]}}}, {"required": ["b"]}]})
    inner = m.resolve("/oneOf/0/properties/a")
    leaf = m.resolve("/oneOf/0/properties/a/oneOf/0")
    rs = candidates(m, Failure(leaf.nid, "maximum", None, [5], (0,)))
    assert labels(rs) == ["loosen-bound:maximum=5", "remove:maximum"]
    for r in rs:
        assert r.guard_conversions == (m.root.nid, inner.nid)
        after = relaxed(r, m)
        assert "oneOf" not in after  # type: ignore[operator]
        assert "oneOf" not in after["anyOf"][0]["properties"]["a"]  # type: ignore[index]
    assert_monotone(m, rs, [{"a": 2}, {"a": 4}, {"a": "s"}, {"b": 1}, {"a": 2, "b": 1}, {"a": 5}, {}])


def test_removal_inside_a_one_of_branch_converts_it() -> None:
    m = model({"oneOf": [{"required": ["a", "b"]}, {"required": ["c"]}]})
    branch = m.resolve("/oneOf/0")
    r = removal(m, Failure(branch.nid, "required", "b", [{"a": 1, "c": 1}], (0,)))
    assert r.guard_conversions == (m.root.nid,)
    assert relaxed(r, m) == {"anyOf": [{"required": ["a"]}, {"required": ["c"]}]}
    assert_monotone(m, (r,), [{"a": 1, "b": 1}, {"c": 1}, {"a": 1, "c": 1}, {"a": 1, "b": 1, "c": 1}, {}])


def test_defs_edits_follow_every_reference() -> None:
    m = model(
        {
            "properties": {
                "a": {"oneOf": [{"$ref": "#/$defs/X"}, {"type": "string"}]},
                "b": {"$ref": "#/$defs/X"},
                "c": {"not": {"$ref": "#/$defs/X"}},
            },
            "$defs": {"X": {"maximum": 3}, "unused": {"not": {"$ref": "#/$defs/X"}}},
        }
    )
    a = m.resolve("/properties/a")
    c = m.resolve("/properties/c")
    rs = candidates(m, failure(m, "/$defs/X", "maximum", [5]))
    for r in rs:
        assert r.guard_conversions == (a.nid,)
        assert r.guard_removals == ((c.nid, "not"),)  # the unreferenced definition needs no guard
    assert relaxed(rs[0], m) == {
        "properties": {
            "a": {"anyOf": [{"$ref": "#/$defs/X"}, {"type": "string"}]},
            "b": {"$ref": "#/$defs/X"},
            "c": {},
        },
        "$defs": {"X": {"maximum": 5}, "unused": {"not": {"$ref": "#/$defs/X"}}},
    }
    docs: list[JSON] = [{"a": 2, "b": 2, "c": 4}, {"a": 4, "b": 1, "c": 7}, {"a": "s", "c": "t"}, {"c": 2}]
    assert_monotone(m, rs, docs)


def test_positive_references_need_no_guard() -> None:
    m = model({"properties": {"a": {"$ref": "#/$defs/X"}}, "anyOf": [{"$ref": "#/$defs/X"}, {}], "$defs": {"X": {}}})
    m.resolve("/$defs/X").kw["maximum"] = 3
    r = candidates(m, failure(m, "/$defs/X", "maximum", [5]))[0]
    assert r.guard_conversions == () and r.guard_removals == ()


def test_conditional_containing_the_edit_is_removed_with_then_and_else() -> None:
    m = model(
        {
            "properties": {"a": {"$ref": "#/$defs/X"}},
            "if": {"properties": {"a": {"$ref": "#/$defs/X"}}},
            "then": {"required": ["b"]},
            "else": {"required": ["c"]},
            "$defs": {"X": {"maximum": 3}},
        }
    )
    r = candidates(m, failure(m, "/$defs/X", "maximum", [5]))[0]
    assert r.guard_removals == ((m.root.nid, "if"),)
    assert relaxed(r, m) == {"properties": {"a": {"$ref": "#/$defs/X"}}, "$defs": {"X": {"maximum": 5}}}
    assert_monotone(m, (r,), [{"a": 1, "b": 1}, {"a": 4, "c": 1}, {"a": 1}, {"c": 1}, {"a": 5, "b": 1}])


def test_not_nested_in_not_is_removed_at_the_outermost_reachable_level() -> None:
    m = model({"not": {"not": {"$ref": "#/$defs/X"}}, "properties": {"a": {"$ref": "#/$defs/X"}}, "$defs": {"X": {"maximum": 3}}})
    r = candidates(m, failure(m, "/$defs/X", "maximum", [5]))[0]
    assert r.guard_removals == ((m.root.nid, "not"),)
    assert_monotone(m, (r,), [1, 4, {"a": 1}, {"a": 4}])


# ---------------------------------------------------------------------------
# False subschemas and relocated references
# ---------------------------------------------------------------------------


def test_false_subschema_is_replaced_by_naive_merge_or_true() -> None:
    m = model({"properties": {"a": False, "b": {}}})
    rs = candidates(m, failure(m, "/properties/a", FALSE_SCHEMA, [1, 2.5]))
    assert labels(rs) == ["replace:false", "remove:false"]
    assert relaxed(rs[0], m) == {"properties": {"a": {"type": ["number"]}, "b": {}}}
    assert relaxed(rs[1], m) == {"properties": {"a": True, "b": {}}}
    assert_monotone(m, rs, [{}, {"a": 1}, {"b": 1}, {"a": "s"}])


def test_false_root_is_replaced_in_place() -> None:
    m = model(False)
    rs = candidates(m, Failure(m.root.nid, FALSE_SCHEMA, None, ["x"], (0,)))
    relaxed_model, (change,) = rs[0].applied(m)
    assert relaxed_model.root.nid == m.root.nid
    assert relaxed_model.root.to_json() == {"type": ["string"]}
    assert change.before is False and change.before_ids == ()
    assert change.after_ids == (ConstraintId(m.root.nid, "type"),)


def test_removing_a_referenced_subschema_relocates_it() -> None:
    m = model({"properties": {"a": {"not": {"type": "string"}}, "b": {"$ref": "#/properties/a/not"}}})
    r = removal(m, failure(m, "/properties/a", "not", ["s"]))
    after = relaxed(r, m)
    assert after == {
        "properties": {"a": {}, "b": {"$ref": "#/$defs/relaxed"}},
        "$defs": {"relaxed": {"type": "string"}},
    }
    assert valid(after, {"a": "s", "b": "t"})
    assert not valid(after, {"b": 1})
    assert_monotone(m, (r,), [{"a": 1, "b": "t"}, {"b": 1}, {"a": "s"}])


def test_relocated_subschemas_keep_references_between_them() -> None:
    # Regression (Property 8): the second relocated branch refers to the first. Its reference
    # used to be rewritten before it was moved into the model, so it kept pointing into the removed not.
    m = model({"not": {"anyOf": [{"maximum": 0}, {"$ref": "#/not/anyOf/0"}]}, "properties": {"p": {"$ref": "#/not/anyOf/1"}}})
    r = removal(m, failure(m, "", "not", [-1]))
    after = relaxed(r, m)
    assert after == {
        "properties": {"p": {"$ref": "#/$defs/relaxed-2"}},
        "$defs": {"relaxed": {"maximum": 0}, "relaxed-2": {"$ref": "#/$defs/relaxed"}},
    }
    assert valid(after, {"p": -1})
    assert not valid(after, {"p": 1})
    assert_monotone(m, (r,), [5, "s", {"p": -1}, {"p": 1}, {}])


# ---------------------------------------------------------------------------
# Changes, records, and copies
# ---------------------------------------------------------------------------


def test_apply_reports_changes_and_applied_leaves_the_input_alone() -> None:
    m = model({"oneOf": [{"maximum": 3}, {"type": "string"}], "required": ["a"]})
    key = m.equality_key()
    branch = m.resolve("/oneOf/0")
    r = candidates(m, Failure(branch.nid, "maximum", None, [5], (4, 2)))[0]
    relaxed_model, changes = r.applied(m)
    assert m.equality_key() == key
    primary, guard = changes
    assert (primary.edit, primary.guard, primary.nid) == ("rewrite", False, branch.nid)
    assert (primary.before, primary.after) == ({"maximum": 3}, {"maximum": 5})
    assert primary.before_ids == primary.after_ids == (ConstraintId(branch.nid, "maximum"),)
    assert (guard.edit, guard.guard) == ("convert", True)
    assert guard.before_ids == (ConstraintId(m.root.nid, "oneOf", 0), ConstraintId(m.root.nid, "oneOf", 1))
    assert guard.after_ids == (ConstraintId(m.root.nid, "anyOf", 0), ConstraintId(m.root.nid, "anyOf", 1))
    record = primary.record(r.failure.documents)
    assert record == RelaxationRecord({"maximum": 3}, {"maximum": 5}, (2, 4))
    # Applying in place gives the same model.
    r.apply(m)
    assert m.equality_key() == relaxed_model.equality_key()


def test_removal_changes_are_marked_removed() -> None:
    m = model({"required": ["a"], "maximum": 3})
    r = removal(m, failure(m, "", "required", [{}], item="a"))
    (change,) = r.apply(m)
    assert change.removed and change.after is REMOVED
    assert change.before == {"required": ["a"]}
    assert change.before_ids == (ConstraintId(m.root.nid, "required", "a"),) and change.after_ids == ()
    assert change.record([0]).removed


def test_removal_is_the_last_candidate() -> None:
    m = model({"type": "integer", "maximum": 3})
    f = failure(m, "", "maximum", [5])
    last = candidates(m, f)[-1]
    alone = removal(m, f)
    assert (alone.label, alone.order, alone.action) == (last.label, last.order, last.action) == ("remove:maximum", (5, 0), "remove")
    assert relaxed(alone, m) == relaxed(last, m)


def test_candidates_are_deterministic() -> None:
    schema = {"anyOf": [{"type": "integer", "maximum": 3}, {"pattern": "^a"}], "additionalProperties": False}
    outputs = []
    for _ in range(2):
        m = model(schema)
        b0, b1 = m.root.kw["anyOf"]
        branches = ((Failure(b0.nid, "maximum", None, [7]),), (Failure(b1.nid, "pattern", None, ["b"]),))
        rs = candidates(m, failure(m, "", "anyOf", [7, "b"], branches=branches))
        outputs.append([(r.label, r.order, relaxed(r, m)) for r in rs])
    assert outputs[0] == outputs[1]


# ---------------------------------------------------------------------------
# DecisionEngine integration
# ---------------------------------------------------------------------------


def test_fallback_takes_the_first_candidate_in_documented_order() -> None:
    m = model({"type": "integer", "maximum": 140})
    rs = candidates(m, failure(m, "", "maximum", [150]))
    engine = DecisionEngine({"decide.mdl": False})
    # Offered in reverse, the canonical order still puts loosening first.
    decision = engine.resolve(DecisionPoint(RELAXATION), [r.candidate() for r in reversed(rs)])
    assert decision.method == "fallback"
    assert decision.outcome is rs[0]
    assert decision.fallback is not None and decision.fallback.technique == "decide.mdl"


def test_mdl_scores_the_previews_on_the_path_view() -> None:
    m = model({"type": "integer", "maximum": 140})
    rs = candidates(m, failure(m, "", "maximum", [150]))
    view = View.root(build_store([10, 120, 140, 150]))
    decision = DecisionEngine().resolve(
        DecisionPoint(RELAXATION), [r.candidate(node=r.preview(m)) for r in rs], view
    )
    assert decision.method == "mdl"
    assert decision.outcome in rs
    assert [score.outcome for score in decision.resolution.candidates] == labels(rs)
