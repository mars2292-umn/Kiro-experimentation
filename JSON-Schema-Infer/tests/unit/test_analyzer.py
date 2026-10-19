"""Unit tests for structure/analyzer.py: the Structure_Analyzer walk (task 16.1).

Requirements 4.2 (Record ``properties`` with recursed subschemas), 4.3 (one
Map value schema, no ``properties`` or ``required``), 5.4 (one branch per
Variant), 7.2 (List ``items``), 7.3 (Tuple ``prefixItems``), 9.7
(``required`` per Variant), and 26.13 (``baseline:type`` records), plus the
design walk's mixed-type rule, closed Records with several Variants, 4.11,
7.9, provenance coverage of every emitted Constraint (26.1), and equality
with Naive_Merge when every Technique is off (28.5).
"""

from __future__ import annotations

import sys
from typing import Any

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.observations import build_store
from jsinfer.core.paths import MAPVAL_STEP
from jsinfer.core.provenance import BASELINE_PREFIX, FALLBACK_PREFIX, laplace_confidence
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import MDL_SELECTION
from jsinfer.deps.presence import REQUIRED_PRODUCER
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.analyzer import (
    BRANCH_CLOSED_ENUM,
    BRANCH_REMAINING_TYPES,
    DISCRIMINATOR,
    EXCLUSION_RULE,
    FIELD_GROUPS,
    RECORD_MAP_PARTITION,
    StructureAnalyzer,
)
from jsinfer.structure.arrays import ArrayAnalyzer
from jsinfer.structure.record_map import RecordMapAnalyzer
from jsinfer.structure.variants import ANY_OF, ONE_OF, VariantAnalyzer, pairwise_disjoint
from jsinfer.values.inferrer import TYPE_PRODUCER, ValueInferrer

VALIDATOR = Validator()
EMITTER = SchemaEmitter()
OFF = {tid: False for tid in REGISTRY.ids()}

# A Record with an optional field.
RECORDS = [{"name": f"n{i}", "age": i % 7, "email": f"e{i}@example.org"} for i in range(30)] + [{"name": "x", "age": 3}]
# Data keys: a Map by MDL.
USER_MAPS = [{f"user{3 * i + j}": {"score": i + j} for j in range(3)} for i in range(20)]
# Two kinds of objects: two Variants by the Bernoulli mixture (no Learned_Threshold needed).
USERS = [{"kind": "user", "name": f"u{i}", "age": 20 + i % 30} for i in range(24)]
ITEMS = [{"sku": f"s{i}", "price": i + 0.25, "tags": ["a", "b"][: i % 3]} for i in range(20)]
FAMILIES = USERS + ITEMS
# Fixed-length rows whose positions each hold one type: a Tuple by MDL.
TUPLES = [[i, f"n{i}"] for i in range(30)]
# Variable-length integer lists: a List by MDL.
LISTS = [list(range(i % 10)) for i in range(30)]
# Objects in arrays in objects.
ORDERS = [
    {
        "id": i,
        "lines": [{"sku": f"s{j}", "qty": j % 3 + 1} for j in range(i % 4)],
        "meta": {"tags": ["a", "b"][: i % 3], "source": "web" if i % 2 else "app"},
    }
    for i in range(24)
]
# Records next to distinct scalars and arrays: one object branch.
MIXED = [{"a": i, "b": "x"} for i in range(8)] + [1, 2.5, "s", None, True, [1], [2, 3]]
# A Closed_Enum next to arrays and objects.
ENUM_MIXED = ["a", "a", "b", "b", "a", [1], [2], {"x": 1}, {"x": 2}]

COLLECTIONS: dict[str, list[Any]] = {
    "scalars": ["a", "b", "a", 1, None],
    "records": RECORDS,
    "maps": USER_MAPS,
    "families": FAMILIES,
    "tuples": TUPLES,
    "lists": LISTS,
    "empty-arrays": [[], [], []],
    "keyless": [{}, {}, {}],
    "orders": ORDERS,
    "mixed": MIXED,
    "enum-mixed": ENUM_MIXED,
    "families-mixed": FAMILIES + ["x", "y", "z", [1], [2, 3]],
    "families-enum": FAMILIES + ["a", "a", "b", "b", [1], [2, 3]],
    "nested-mixed": [{"v": [1, "a", {"k": [None]}, []]}, {"v": {"k": 2}}, {"v": []}],
}


def score(model_bits: float, data_bits: float) -> Score:
    return Score(float(model_bits), float(data_bits), 0.0, 0, 1)


def shape(node: SchemaNode) -> str:
    """Which object candidate a node is: ``closed`` (additionalProperties: false), ``map``, or ``record`` (open)."""
    additional = node.kw.get("additionalProperties")
    if additional is None:
        return "record"
    return "closed" if additional.boolean is False else "map"


class FixedScorer(MDLScorer):
    """Scores each Record-or-Map and Closed-or-Open candidate by its :func:`shape`; ``universal_bits`` stays real."""

    __slots__ = ("scores",)

    def __init__(self, **scores: Score) -> None:
        super().__init__(validator=VALIDATOR)
        self.scores = scores

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        return self.scores[shape(node)]


# Scores under which every object Path is a Record and closes.
CLOSING = FixedScorer(record=score(50, 50), map=score(100, 100), closed=score(1, 1))


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def analyzer(
    states: dict[str, bool] | None = None, *, closing: bool = False
) -> tuple[StructureAnalyzer, InferenceReport]:
    report = InferenceReport()
    if not closing:
        return StructureAnalyzer.for_techniques(states, report=report, validator=VALIDATOR), report
    values = ValueInferrer.for_techniques(states, report=report, validator=VALIDATOR)
    return StructureAnalyzer(values, record_map=RecordMapAnalyzer(values, scorer=CLOSING)), report


def infer(
    docs: list[Any], states: dict[str, bool] | None = None, *, closing: bool = False
) -> tuple[SchemaModel, InferenceReport]:
    sa, report = analyzer(states, closing=closing)
    return sa.infer_model(root(docs)), report


def record(model: SchemaModel, node: SchemaNode, keyword: str, item: str | int | None = None) -> Any:
    return model.provenance[ConstraintId(node.nid, keyword, item)]


def decisions(report: InferenceReport, kind: str) -> list[Any]:
    return [entry for entry in report.decisions if entry.kind == kind]


def validates(schema: Any, docs: list[Any]) -> bool:
    compiled = VALIDATOR.compile(schema)
    return all(VALIDATOR.is_valid(compiled, doc) for doc in docs)


def assert_complete(model: SchemaModel) -> None:
    """Every emitted Constraint has exactly one record, every record belongs to one, and producers are valid."""
    _, pointers = EMITTER.layout(model)
    assert sorted(pointers) == sorted(model.provenance)
    for cid, rec in model.provenance.items():
        technique = rec.technique
        assert technique in REGISTRY.ids() or technique.startswith((FALLBACK_PREFIX, BASELINE_PREFIX)), cid
        assert 0.0 <= rec.confidence <= 1.0
        assert rec.support.consistent <= rec.support.total
        if cid.keyword == "type":
            assert technique == TYPE_PRODUCER and rec.fallback is None and rec.resolution is None
        if cid.keyword == "required":
            assert technique == REQUIRED_PRODUCER


def nested(depth: int) -> Any:
    value: Any = 1
    for _ in range(depth):
        value = {"a": value}
    return value


# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------


def test_components_share_one_engine_validator_and_report() -> None:
    sa, report = analyzer()
    engine = sa.engine
    assert sa.record_map.engine is engine and sa.variants.engine is engine
    assert sa.arrays.engine is engine and sa.cooccurrence.engine is engine
    assert sa.values.engine is engine and sa.validator is VALIDATOR and sa.report is report
    assert sa.record_map.values is sa.values and sa.arrays.values is sa.values


def test_for_run_takes_the_configuration() -> None:
    config = RunConfiguration.default()
    report = InferenceReport()
    sa = StructureAnalyzer.for_run(config, None, report, validator=VALIDATOR)
    assert dict(sa.engine.techniques) == dict(config.techniques) and sa.report is report
    assert sa.variants.seed == config.seed
    assert all(sa.variants.resources[name] == config.resources[name] for name in sa.variants.resources)


def test_components_must_share_the_engine() -> None:
    values = ValueInferrer.for_techniques(validator=VALIDATOR)
    other = ValueInferrer.for_techniques(validator=VALIDATOR)
    with pytest.raises(ValueError, match="DecisionEngine"):
        StructureAnalyzer(values, arrays=ArrayAnalyzer(other))
    with pytest.raises(ValueError, match="resources and seed"):
        StructureAnalyzer(values, variants=VariantAnalyzer(values.engine), seed=3)
    with pytest.raises(TypeError, match="ValueInferrer"):
        StructureAnalyzer(object())  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="View"):
        StructureAnalyzer(values).infer_model([1])  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Scalars, Records, and Maps
# ---------------------------------------------------------------------------


def test_empty_view_is_the_empty_schema() -> None:
    store = build_store([1, {"a": 2}])
    store.flags.flag_documents([0, 1])
    sa, _ = analyzer()
    model = sa.infer_model(View.root(store))
    assert model.root.to_json() == {} and model.provenance == {}


def test_scalars_are_the_value_inferrers_node() -> None:
    docs = ["red", "green", "red", "blue", "green", "red"]
    model, _ = infer(docs)
    expected = ValueInferrer.for_techniques(validator=VALIDATOR).infer_scalar(root(docs))
    assert model.root.to_json() == expected.keywords()
    assert {cid.keyword for cid in model.provenance} == set(expected.provenance)
    rec = record(model, model.root, "type")
    assert rec.technique == TYPE_PRODUCER and (rec.support.consistent, rec.support.total) == (6, 6)
    assert_complete(model)


def test_record_lists_observed_keys_with_recursed_subschemas_and_required() -> None:
    model, report = infer(RECORDS)
    node = model.root
    assert node.kw["type"] == ["object"]
    assert sorted(node.kw["properties"]) == ["age", "email", "name"]  # 4.2
    assert node.kw["required"] == ["age", "name"]  # 9.7: email is optional
    # Children are inferred by the walk itself, not by Naive_Merge (age is a Closed_Enum).
    age = node.kw["properties"]["age"]
    assert age.kw["type"] == ["integer"] and age.kw["enum"] == list(range(7))
    kind = record(model, node, "properties")
    assert kind.technique == MDL_SELECTION and kind.details["judgment"] == "record"
    for name in ("age", "name"):
        assert record(model, node, "required", name).technique == REQUIRED_PRODUCER
    assert [e.path for e in decisions(report, "record_or_map")] == ["$"]
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], RECORDS)


def test_map_has_one_value_schema_and_no_properties_or_required() -> None:
    model, _ = infer(USER_MAPS)
    node = model.root
    assert node.kw["type"] == ["object"]
    assert "properties" not in node.kw and "required" not in node.kw  # 4.3
    values = node.kw["additionalProperties"]
    assert values.kw["type"] == ["object"] and sorted(values.kw["properties"]) == ["score"]
    assert values.kw["required"] == ["score"]  # the value schema is analyzed by the walk
    kind = record(model, node, "additionalProperties")
    assert kind.details["judgment"] == "map"
    names = node.kw.get("propertyNames")
    if names is not None:  # the key Constraint is a Constraint of its subschema too
        assert record(model, names, "format") == record(model, node, "propertyNames")
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], USER_MAPS)


def test_keyless_record_notes_on_type_that_no_keys_were_available() -> None:
    model, report = infer([{}, {}, {}])
    assert model.root.to_json() == {"type": ["object"]}
    rec = record(model, model.root, "type")
    assert rec.technique == TYPE_PRODUCER
    assert dict(rec.details) == {"judgment": "record", "keys_available": False}  # 4.11
    assert decisions(report, "record_or_map") == []
    assert_complete(model)


# ---------------------------------------------------------------------------
# Variants and closed Records
# ---------------------------------------------------------------------------


def test_variants_become_one_branch_each() -> None:
    model, _ = infer(FAMILIES)
    node = model.root
    (keyword,) = node.kw
    branches = node.kw[keyword]
    assert keyword in (ANY_OF, ONE_OF) and len(branches) == 2  # 5.4
    assert (keyword == ONE_OF) == pairwise_disjoint(branches, validator=VALIDATOR)
    assert [sorted(b.kw["properties"]) for b in branches] == [["age", "kind", "name"], ["price", "sku", "tags"]]
    for index, branch in enumerate(branches):
        assert branch.kw["type"] == ["object"]
        assert record(model, branch, "type").technique == TYPE_PRODUCER
        rec = record(model, node, keyword, index)
        assert rec.details["variant"] == index and rec.details["combinator"] == keyword
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], FAMILIES)


def test_closed_record_with_variants_closes_each_branch_and_nothing_beside() -> None:
    model, report = infer(FAMILIES, closing=True)
    node = model.root
    (keyword,) = node.kw  # no additionalProperties beside the combinator
    branches = node.kw[keyword]
    assert len(branches) == 2
    closing = decisions(report, "closed_or_open_record")
    assert [e.path for e in closing] == ["$"] and closing[0].resolution.method == "mdl"
    for branch in branches:
        assert branch.kw["additionalProperties"].boolean is False
        rec = record(model, branch, "additionalProperties")
        assert rec.technique == MDL_SELECTION and rec.details["judgment"] == "closed"
    # Each branch requires a field the other does not permit, so the branches are disjoint (5.5).
    assert keyword == ONE_OF and pairwise_disjoint(branches, validator=VALIDATOR)
    schema = EMITTER.layout(model)[0]
    assert validates(schema, FAMILIES)
    assert not validates(schema, [{**USERS[0], "zzz": 1}])
    assert_complete(model)


# ---------------------------------------------------------------------------
# Arrays
# ---------------------------------------------------------------------------


def test_tuple_has_one_prefix_subschema_per_position() -> None:
    model, _ = infer(TUPLES)
    node = model.root
    assert node.kw["type"] == ["array"]
    prefix = node.kw["prefixItems"]
    assert [p.kw["type"] for p in prefix] == [["integer"], ["string"]]  # 7.3
    assert node.kw["items"].boolean is False  # 7.4
    shared = record(model, node, "prefixItems")
    assert shared.details["judgment"] == "tuple" and record(model, node, "items") == shared
    assert record(model, node, "type").technique == TYPE_PRODUCER
    assert_complete(model)


def test_list_has_one_items_subschema() -> None:
    model, _ = infer(LISTS)
    node = model.root
    assert node.kw["type"] == ["array"] and "prefixItems" not in node.kw  # 7.2
    assert node.kw["items"].kw["type"] == ["integer"]
    assert record(model, node, "items").details["judgment"] == "list"
    assert_complete(model)


def test_empty_only_arrays_note_on_type_that_no_element_was_observed() -> None:
    model, report = infer([[], [], []])
    node = model.root
    assert node.kw["type"] == ["array"] and "items" not in node.kw and "prefixItems" not in node.kw  # 7.9
    assert dict(record(model, node, "type").details) == {"elements_observed": False}
    assert decisions(report, "tuple_or_list") == []
    assert_complete(model)


# ---------------------------------------------------------------------------
# Mixed types (design walk, step 4)
# ---------------------------------------------------------------------------


def test_one_object_branch_coexists_with_the_other_types() -> None:
    model, _ = infer(MIXED)
    node = model.root
    assert node.kw["type"] == ["null", "boolean", "number", "string", "array", "object"]
    assert "properties" in node.kw and "items" in node.kw and ANY_OF not in node.kw
    rec = record(model, node, "type")
    assert rec.technique == TYPE_PRODUCER and (rec.support.consistent, rec.support.total) == (len(MIXED),) * 2
    assert record(model, node, "properties").details["judgment"] == "record"
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], MIXED)


def test_two_object_branches_and_a_branch_for_the_other_types() -> None:
    docs = COLLECTIONS["families-mixed"]
    model, _ = infer(docs)
    node = model.root
    assert list(node.kw) == [ANY_OF]
    branches = node.kw[ANY_OF]
    assert len(branches) == 3
    assert [b.kw["type"] for b in branches] == [["object"], ["object"], ["string", "array"]]
    for index in (0, 1):
        rec = record(model, node, ANY_OF, index)
        assert rec.details["variant"] == index and rec.details["combinator"] == ANY_OF
        assert rec.support.total == len(docs)
    rest = record(model, node, ANY_OF, 2)
    assert rest.details["branch"] == BRANCH_REMAINING_TYPES and rest.details["types"] == ["string", "array"]
    assert (rest.support.consistent, rest.support.total) == (5, len(docs))
    assert rest.resolution is not None and rest.resolution.method == record(model, node, ANY_OF, 0).resolution.method
    assert record(model, branches[2], "type").support.total == 5
    assert not pairwise_disjoint(branches, validator=VALIDATOR)  # anyOf agrees with 5.5 over the emitted branches
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], docs)


def test_closed_enum_beside_other_types_is_its_own_branch() -> None:
    model, _ = infer(ENUM_MIXED)
    node = model.root
    branches = node.kw[ANY_OF]
    assert [b.kw["type"] for b in branches] == [["string"], ["array", "object"]]
    assert branches[0].kw["enum"] == ["a", "b"] and "enum" not in branches[1].kw
    first, second = record(model, node, ANY_OF, 0), record(model, node, ANY_OF, 1)
    assert first.technique == second.technique == "values.enum_estimation"
    assert first.details["branch"] == BRANCH_CLOSED_ENUM and second.details["branch"] == BRANCH_REMAINING_TYPES
    assert (first.support.consistent, second.support.consistent, first.support.total) == (5, 4, 9)
    assert not pairwise_disjoint(branches, validator=VALIDATOR)
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], ENUM_MIXED)


def test_closed_enum_beside_variants_comes_after_them() -> None:
    docs = COLLECTIONS["families-enum"]
    model, _ = infer(docs, {"structure.shared_defs": False})  # Shared_Definitions (8) would merge the two Variants
    branches = model.root.kw[ANY_OF]
    assert [b.kw["type"] for b in branches] == [["object"], ["object"], ["string"], ["array"]]
    assert branches[2].kw["enum"] == ["a", "b"]
    assert record(model, model.root, ANY_OF, 2).details["branch"] == BRANCH_CLOSED_ENUM
    assert record(model, model.root, ANY_OF, 3).details["branch"] == BRANCH_REMAINING_TYPES
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], docs)


# ---------------------------------------------------------------------------
# Recursion
# ---------------------------------------------------------------------------


def test_nested_paths_are_analyzed_at_every_level() -> None:
    model, report = infer(ORDERS)
    lines = model.root.kw["properties"]["lines"]
    line = lines.kw["items"]
    assert line.kw["type"] == ["object"] and line.kw["required"] == ["qty", "sku"]
    assert line.kw["properties"]["qty"].kw["enum"] == [1, 2, 3]
    meta = model.root.kw["properties"]["meta"]
    assert meta.kw["properties"]["tags"].kw["type"] == ["array"]
    paths = {e.path for e in decisions(report, "record_or_map")}
    assert paths == {"$", "$.lines[*]", "$.meta"}
    assert {e.path for e in decisions(report, "tuple_or_list")} == {"$.lines", "$.meta.tags"}
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], ORDERS)


def test_subtrees_equal_the_walk_of_their_own_views() -> None:
    sa, _ = analyzer()
    view = root(ORDERS)
    model = sa.infer_model(view)
    meta = sa.infer_model(view.child_key("meta"))
    assert model.root.kw["properties"]["meta"] == meta.root


def test_deep_nesting_gets_recursion_room_and_restores_the_limit() -> None:
    view = root([nested(60), nested(60)])
    sa, _ = analyzer(OFF)
    original = sys.getrecursionlimit()
    depth = 0
    frame = sys._getframe()
    while frame is not None:
        depth, frame = depth + 1, frame.f_back
    sys.setrecursionlimit(depth + 200)  # far fewer frames than 60 object levels take (about ten each)
    try:
        model = sa.infer_model(view)
        assert sys.getrecursionlimit() == depth + 200
    finally:
        sys.setrecursionlimit(original)
    assert model.root == naive_merge_node(view)


# ---------------------------------------------------------------------------
# Records, flags, and the all-off baseline
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("name", list(COLLECTIONS))
@pytest.mark.parametrize("states", [None, OFF, {MDL_SELECTION: False}], ids=["defaults", "all-off", "no-mdl"])
def test_every_emitted_constraint_has_exactly_one_record(name: str, states: dict[str, bool] | None) -> None:
    docs = COLLECTIONS[name]
    model, _ = infer(docs, states)
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], docs)


@pytest.mark.parametrize("name", ["records", "families", "orders", "mixed", "enum-mixed"])
def test_every_emitted_constraint_has_exactly_one_record_when_records_close(name: str) -> None:
    docs = COLLECTIONS[name]
    model, _ = infer(docs, closing=True)
    assert_complete(model)
    assert validates(EMITTER.layout(model)[0], docs)


def test_type_records_name_the_baseline_rule_with_the_laplace_confidence() -> None:
    model, _ = infer(COLLECTIONS["families-mixed"] + ORDERS)
    typed = [node for _, node in model.walk() if node.boolean is None and "type" in node.kw]
    assert typed
    for node in typed:
        rec = record(model, node, "type")
        assert rec.technique == TYPE_PRODUCER and rec.fallback is None and rec.resolution is None
        assert rec.confidence == laplace_confidence(rec.support)


@pytest.mark.parametrize("name", list(COLLECTIONS))
def test_every_technique_off_equals_naive_merge(name: str) -> None:
    docs = COLLECTIONS[name]
    model, _ = infer(docs, OFF)
    expected = naive_merge_node(root(docs))
    assert model.root.to_json() == expected.to_json()
    assert model.root == expected


def test_flagged_documents_and_items_are_left_out() -> None:
    docs = [{"a": 1, "b": "x"}, {"a": "odd", "c": [1]}, {"a": 3, "b": None}]
    store = build_store(docs)
    store.flags.flag_documents([1])
    first_a = View.root(store).child_key("a").parts[0].rows[:1]
    store.flags.flag_items((MAPVAL_STEP,), first_a)
    sa, _ = analyzer(OFF)
    model = sa.infer_model(View.root(store))
    assert model.root.to_json() == naive_merge_node(View.root(store)).to_json()
    assert sorted(model.root.kw["properties"]) == ["a", "b"]
    assert model.root.kw["properties"]["a"].kw["type"] == ["integer"]


# ---------------------------------------------------------------------------
# Hooks of unimplemented Techniques
# ---------------------------------------------------------------------------


def test_disabled_decision_points_record_their_fallbacks() -> None:
    _, report = infer(RECORDS, {"structure.hybrid_record_map": False})
    expected = {
        RECORD_MAP_PARTITION: "structure.hybrid_record_map",
    }
    for kind, technique in expected.items():
        (entry,) = decisions(report, kind)
        assert entry.path == "$" and entry.resolution.method == "fallback"
        assert (entry.fallback.technique, entry.fallback.reason) == (technique, "disabled")
    _, report = infer(RECORDS)  # the hybrid partition (task 37.1) is selected by MDL, no partition included
    (entry,) = decisions(report, RECORD_MAP_PARTITION)
    assert entry.path == "$" and entry.resolution.method == "mdl"
    (groups,) = decisions(report, FIELD_GROUPS)  # Field_Groups (task 33.1) are selected by MDL
    assert groups.path == "$" and groups.resolution.method == "mdl"
    (tagged,) = decisions(report, DISCRIMINATOR)  # decision trees (task 34.1) need tagged.branch_support (10.6)
    assert tagged.resolution.method == "fallback" and tagged.fallback.reason == "missing-threshold"
    (exclusion,) = decisions(report, EXCLUSION_RULE)  # exclusion mining (task 35.1) needs exclusion.min_confidence (11.7)
    assert exclusion.resolution.method == "fallback" and exclusion.fallback.reason == "missing-threshold"
    # A Path with no keys reaches none of them.
    _, report = infer([{}, {}], {"structure.hybrid_record_map": False})
    assert all(decisions(report, kind) == [] for kind in expected)


def test_enabled_unimplemented_techniques_offer_nothing() -> None:
    model, report = infer(RECORDS, {"groups.bmf": True, "groups.fpgrowth_slim": False})
    assert decisions(report, FIELD_GROUPS) == []
    assert len(decisions(report, EXCLUSION_RULE)) == 1
    assert_complete(model)


