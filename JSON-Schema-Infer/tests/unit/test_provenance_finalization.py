"""Unit tests for provenance finalization (task 17.1; Requirements 26.1, 26.2, 26.4, 26.10, 26.13).

``pipeline.infer.finalize_provenance`` recomputes the Support of every
emitted Constraint over the non-flagged Observations (with the soundness
check's walk, ``pipeline.soundness.count_instances``), refreshes the
Laplace default Confidences (``core.provenance.finalize_record``), and
checks that records and emitted Constraints correspond one to one
(``core.provenance.check_records``). Hand-written models fix the expected
counts; pipeline runs check that finalization keeps every other field and
every decision Confidence.
"""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.errors import InferenceError
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import build_store
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP
from jsinfer.core.provenance import (
    KIND_SPECIFIC_CONFIDENCE,
    CandidateScore,
    FallbackInfo,
    ProvenanceError,
    ProvenanceRecord,
    RelaxationRecord,
    Resolution,
    SampleInfo,
    Support,
    check_records,
    finalize_record,
    has_default_confidence,
    laplace_confidence,
)
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.pipeline import infer as pipeline
from jsinfer.pipeline.infer import SOUNDNESS_STAGE, PipelineResult, finalize_provenance, infer_documents
from jsinfer.pipeline.soundness import check_soundness, flagged_documents, flagged_item_locations
from jsinfer.structure.analyzer import StructureAnalyzer

V = Validator()
EMITTER = SchemaEmitter()
ONE_WORKER = RunConfiguration.resolve({"workers": 1})
ALL_OFF = RunConfiguration.resolve({"workers": 1, **{technique_setting(tid): False for tid in REGISTRY.ids()}})

STALE = Support(0, 0)  # the Support every hand-made record starts with, so finalization must replace it


def model(schema: dict[str, Any] | bool) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(schema))


def recorded(m: SchemaModel) -> SchemaModel:
    """Give every emitted Constraint without a record one: Baseline_Rule records for type and required."""
    _, pointers = EMITTER.layout(m)
    for cid in pointers:
        if cid.keyword in ("type", "required"):
            record = ProvenanceRecord.baseline(cid.keyword, STALE)
        else:
            record = ProvenanceRecord.laplace("decide.mdl", STALE)
        m.provenance.setdefault(cid, record)
    return m


def by_pointer(m: SchemaModel) -> dict[str, ProvenanceRecord]:
    _, pointers = EMITTER.layout(m)
    return {pointer: m.provenance[cid] for cid, pointer in pointers.items()}


def counts(m: SchemaModel) -> dict[str, tuple[int, int]]:
    return {pointer: (r.support.consistent, r.support.total) for pointer, r in by_pointer(m).items()}


def cid_at(m: SchemaModel, pointer: str) -> ConstraintId:
    _, pointers = EMITTER.layout(m)
    (cid,) = [cid for cid, at in pointers.items() if at == pointer]
    return cid


def mdl_record(technique: str, support: Support, confidence: float) -> ProvenanceRecord:
    scores = (CandidateScore(technique, "chosen", 3.5), CandidateScore(technique, "other", 7.0))
    return ProvenanceRecord(technique, support, confidence, resolution=Resolution("mdl", 0, scores))


def without(record: ProvenanceRecord, *names: str) -> dict[str, Any]:
    data = record.to_json()
    for name in names:
        data.pop(name, None)
    return data


# ---------------------------------------------------------------------------
# Default Confidences (core.provenance)
# ---------------------------------------------------------------------------

PRIORITY = Resolution("priority", 0, (CandidateScore("variants.dbscan", "grouping", None),))
FALLBACK = Resolution("fallback", 0, (CandidateScore(None, "record", None),))
CHANGE = RelaxationRecord({"type": "integer"}, {"type": ["integer", "string"]}, (3,))


@pytest.mark.parametrize(
    ("label", "record", "default"),
    [
        ("Baseline_Rule type", ProvenanceRecord.baseline("type", Support(2, 2)), True),
        ("Baseline_Rule required", ProvenanceRecord.baseline("required", Support(5, 5)), True),
        ("annotation without a decision", ProvenanceRecord.laplace("values.latent_type", Support(4, 4)), True),
        ("priority, Laplace", ProvenanceRecord.laplace("variants.dbscan", Support(2, 3), resolution=PRIORITY), True),
        (
            "fallback",
            ProvenanceRecord.laplace(
                "fallback:record_or_map",
                Support(3, 3),
                fallback=FallbackInfo("decide.mdl", "disabled"),
                resolution=FALLBACK,
            ),
            True,
        ),
        ("relaxed, stale Laplace", ProvenanceRecord("baseline:type", Support(4, 4), 0.5, changes=(CHANGE,)), True),
        ("untouched, not Laplace", ProvenanceRecord("values.latent_type", Support(4, 4), 0.5), False),
        ("MDL posterior", mdl_record("values.bounds", Support(3, 3), 0.9), False),
        ("MDL posterior equal to Laplace", mdl_record("values.bounds", Support(3, 3), 0.8), False),
        (
            "Decision_Model probability",
            ProvenanceRecord(
                "decide.dm_record_map",
                Support(3, 3),
                0.8,
                resolution=Resolution(
                    "decision-model",
                    0,
                    (CandidateScore("decide.dm_record_map", "record", 0.8), CandidateScore(None, "map", 0.2)),
                ),
            ),
            False,
        ),
        (
            "kind-specific rule Confidence",
            ProvenanceRecord(
                "rules.cooccurrence",
                Support(9, 9),
                6 / 7,
                resolution=Resolution("priority", 0, (CandidateScore("rules.cooccurrence", "a->b", None),)),
                changes=(CHANGE,),
            ),
            False,
        ),
        (
            "unknown non-Laplace value",
            ProvenanceRecord("variants.dbscan", Support(2, 3), 0.42, resolution=PRIORITY),
            False,
        ),
    ],
)
def test_default_confidences_are_those_nothing_else_supplied(
    label: str, record: ProvenanceRecord, default: bool
) -> None:
    assert has_default_confidence(record) is default, label
    final = finalize_record(record, Support(6, 8))
    assert final.support == Support(6, 8)
    assert final.confidence == (laplace_confidence(Support(6, 8)) if default else record.confidence), label
    assert without(final, "support", "confidence") == without(record, "support", "confidence"), label


def test_kind_specific_confidences_name_registered_techniques() -> None:
    assert list(KIND_SPECIFIC_CONFIDENCE) == sorted(KIND_SPECIFIC_CONFIDENCE)
    assert all(tid in REGISTRY.ids() for tid in KIND_SPECIFIC_CONFIDENCE)


def test_finalizing_twice_changes_nothing() -> None:
    record = ProvenanceRecord.baseline("type", Support(1, 1), changes=(CHANGE,))
    once = finalize_record(record, Support(7, 7))
    assert finalize_record(once, Support(7, 7)) == once
    assert once.confidence == 8 / 9


# ---------------------------------------------------------------------------
# One record per Constraint (26.1)
# ---------------------------------------------------------------------------


def test_check_records_names_missing_and_orphaned_records() -> None:
    a, b, c = ConstraintId(1, "type"), ConstraintId(1, "required", "x"), ConstraintId(9, "anyOf", 0)
    record = ProvenanceRecord.baseline("type", Support(1, 1))
    check_records({a: "/type"}, {a: record})
    with pytest.raises(ProvenanceError) as caught:
        check_records({a: "/type", b: "/required/0"}, {a: record, c: record})
    error = caught.value
    assert error.missing == ((b, "/required/0"),) and error.orphaned == (c,)
    assert "/required/0" in str(error) and "1 Provenance_Record(s) of no emitted Constraint" in str(error)
    assert isinstance(error, RuntimeError)


def test_finalization_rejects_a_constraint_without_a_record() -> None:
    m = recorded(model({"type": "object", "properties": {"a": {"type": "integer", "minimum": 0}}}))
    del m.provenance[cid_at(m, "/properties/a/minimum")]
    with pytest.raises(ProvenanceError) as caught:
        finalize_provenance(m, [{"a": 1}], validator=V)
    assert [pointer for _, pointer in caught.value.missing] == ["/properties/a/minimum"]


def test_finalization_rejects_a_record_of_no_constraint() -> None:
    m = recorded(model({"type": "integer"}))
    m.provenance[ConstraintId(m.root.nid, "maximum")] = ProvenanceRecord.laplace("values.bounds", STALE)
    with pytest.raises(ProvenanceError) as caught:
        finalize_provenance(m, [1], validator=V)
    assert caught.value.orphaned == (ConstraintId(m.root.nid, "maximum"),) and caught.value.missing == ()


def test_a_missing_record_fails_the_run_in_the_soundness_stage(monkeypatch: pytest.MonkeyPatch) -> None:
    original = StructureAnalyzer.infer_model

    def lossy(self: StructureAnalyzer, view: Any) -> SchemaModel:
        m = original(self, view)
        m.provenance.pop(cid_at(m, "/properties/a/type"))
        return m

    monkeypatch.setattr(StructureAnalyzer, "infer_model", lossy)
    with pytest.raises(InferenceError) as caught:
        infer_documents([{"a": 1}, {"a": 2}], ONE_WORKER)
    assert caught.value.stage == SOUNDNESS_STAGE
    assert isinstance(caught.value.__cause__, ProvenanceError)


# ---------------------------------------------------------------------------
# Support over the instances at each Constraint (26.2)
# ---------------------------------------------------------------------------


def test_support_counts_the_instances_each_keyword_applies_to() -> None:
    m = recorded(
        model(
            {
                "type": "object",
                "properties": {
                    "a": {"type": ["integer", "string"], "maximum": 5, "maxLength": 3},
                    "b": {"type": "array", "items": {"type": "integer"}, "minItems": 0},
                },
                "required": ["a"],
            }
        )
    )
    docs: list[JSON] = [{"a": 1}, {"a": "xy", "b": [1, 2]}, {"a": 4, "b": []}]
    pointers = finalize_provenance(m, docs, validator=V)
    assert set(pointers) == set(m.provenance)
    assert counts(m) == {
        "/type": (3, 3),
        "/properties": (3, 3),
        "/properties/a/type": (3, 3),
        "/properties/a/maximum": (2, 2),  # numbers only
        "/properties/a/maxLength": (1, 1),  # strings only
        "/properties/b/type": (2, 2),
        "/properties/b/items": (2, 2),  # arrays, the empty one included
        "/properties/b/items/type": (2, 2),  # every element
        "/properties/b/minItems": (2, 2),
        "/required/0": (3, 3),
    }
    # Every record here is a Laplace default, refreshed from its final Support.
    assert all(r.confidence == laplace_confidence(r.support) for r in m.provenance.values())


def test_definitions_count_the_instances_of_every_referencing_site() -> None:
    m = recorded(
        model(
            {
                "$defs": {"N": {"type": "integer", "maximum": 30}, "M": {"type": "string"}},
                "type": "object",
                "properties": {"x": {"$ref": "#/$defs/N"}, "y": {"$ref": "#/$defs/N"}},
            }
        )
    )
    finalize_provenance(m, [{"x": 1, "y": 2}, {"x": 3}, {"y": 4}, {}], validator=V)
    found = counts(m)
    assert found["/$defs/N/type"] == found["/$defs/N/maximum"] == (4, 4)  # both sites
    assert found["/properties/x/$ref"] == found["/properties/y/$ref"] == (2, 2)
    assert found["/$defs/M/type"] == (0, 0)  # referenced nowhere
    assert by_pointer(m)["/$defs/M/type"].confidence == 0.5


def test_branch_support_counts_the_instances_each_branch_accepts() -> None:
    m = recorded(model({"anyOf": [{"type": "integer"}, {"minimum": 0}]}))
    finalize_provenance(m, [-1, 5, 2.5, "s"], validator=V)
    assert counts(m) == {
        "/anyOf/0": (2, 4),  # -1 and 5 among every instance
        "/anyOf/0/type": (2, 2),  # the instances the branch accepts
        "/anyOf/1": (3, 4),  # 5, 2.5, and "s"
        "/anyOf/1/minimum": (2, 2),  # the numbers among them
    }


MAP_SCHEMA = {"type": "object", "additionalProperties": {"type": "integer"}, "propertyNames": {"format": "uuid"}}
UUIDS = (
    "0e8d6f3c-8a4e-4c5e-9a2b-1f0e2d3c4b5a",
    "1f9e7a4d-9b5f-4d6f-8b3c-2a1f3e4d5c6b",
    "2a0f8b5e-ac60-4e70-9c4d-3b2a4f5e6d7c",
)


def test_key_constraints_share_one_support_over_the_keys() -> None:
    m = model(MAP_SCHEMA)
    names = m.root.kw["propertyNames"]
    shared = mdl_record("values.format", STALE, 0.97)
    m.provenance[ConstraintId(m.root.nid, "propertyNames")] = shared
    m.provenance[ConstraintId(names.nid, "format")] = shared
    recorded(m)
    keys = UUIDS
    docs: list[JSON] = [{keys[0]: 1, keys[1]: 2}, {keys[2]: 3}, {}]
    finalize_provenance(m, docs, validator=V)
    records = by_pointer(m)
    assert records["/propertyNames"] == records["/propertyNames/format"]  # still one record (design, Open Decisions)
    assert counts(m)["/propertyNames"] == (3, 3)  # the keys, not the objects
    assert records["/propertyNames"].confidence == 0.97  # an MDL posterior stays

    # A flagged member's key is part of its Flagged_Item.
    flagged = recorded(model(MAP_SCHEMA))
    finalize_provenance(flagged, docs, flagged_items={0: [(keys[0],)]}, validator=V)
    assert counts(flagged)["/propertyNames"] == counts(flagged)["/propertyNames/format"] == (2, 2)
    assert counts(flagged)["/additionalProperties/type"] == (2, 2)
    assert counts(flagged)["/type"] == (3, 3)


# ---------------------------------------------------------------------------
# Non-flagged Observations (26.2, 17.4)
# ---------------------------------------------------------------------------

FLAG_DOCS: list[JSON] = [
    {"a": 1, "b": "x"},
    {"a": 2, "b": "yy"},
    {"a": "bad", "b": "zzzzz"},  # $.a is a Flagged_Item
    {"a": 9, "b": "q"},  # a Flagged_Document
]
FLAG_SCHEMA = {
    "type": "object",
    "properties": {"a": {"type": "integer", "maximum": 5}, "b": {"type": "string", "maxLength": 3}},
    "required": ["a", "b"],
}


def test_flagged_item_locations_follow_the_store() -> None:
    store = build_store(FLAG_DOCS)
    column = store.columns[(MAPVAL_STEP,)]
    row = next(r for r in range(len(column)) if column.doc[r] == 2 and store.keys[int(column.key[r])] == "a")
    store.flags.flag_items((MAPVAL_STEP,), [row])
    store.flags.flag_documents([3])
    assert flagged_item_locations(store) == {2: (("a",),)}  # flagged documents are not items
    assert flagged_documents(store).tolist() == [2, 3]  # what the soundness check leaves out

    nested = build_store([{"x": [{"y": 1}, {"y": 2}]}])
    nested.flags.flag_items((MAPVAL_STEP, ELEM_STEP, MAPVAL_STEP), [1])  # the second element's y
    assert flagged_item_locations(nested) == {0: (("x", "1", "y"),)}


def test_support_excludes_flagged_documents_and_items() -> None:
    store = build_store(FLAG_DOCS)
    column = store.columns[(MAPVAL_STEP,)]
    row = next(r for r in range(len(column)) if column.doc[r] == 2 and store.keys[int(column.key[r])] == "a")
    store.flags.flag_items((MAPVAL_STEP,), [row])
    store.flags.flag_documents([3])
    docs = [doc for doc in range(len(FLAG_DOCS)) if doc not in store.flags.documents.tolist()]
    m = recorded(model(FLAG_SCHEMA))
    finalize_provenance(m, FLAG_DOCS, docs=docs, flagged_items=flagged_item_locations(store), validator=V)
    assert counts(m) == {
        "/type": (3, 3),  # document 3 is flagged
        "/properties": (3, 3),
        "/properties/a/type": (2, 2),  # document 2's a is flagged
        "/properties/a/maximum": (2, 2),
        "/properties/b/type": (3, 3),  # document 2's b is not
        "/properties/b/maxLength": (2, 3),  # ... and violates maxLength, which no flag explains
        "/required/0": (3, 3),  # a flagged member's key still counts as present
        "/required/1": (3, 3),
    }


def test_branches_are_chosen_modulo_flagged_items() -> None:
    def branch(tag: str, value_type: str) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {"k": {"const": tag}, "v": {"type": value_type}},
            "required": ["k"],
        }

    m = recorded(model({"anyOf": [branch("p", "integer"), branch("q", "string")]}))
    docs: list[JSON] = [{"k": "p", "v": 1}, {"k": "q", "v": "s"}, {"k": "p", "v": "oops"}]
    finalize_provenance(m, docs, flagged_items={2: [("v",)]}, validator=V)
    found = counts(m)
    assert found["/anyOf/0"] == (2, 3) and found["/anyOf/1"] == (1, 3)  # document 2 belongs to the first branch
    assert found["/anyOf/0/properties/k/const"] == (2, 2)
    assert found["/anyOf/0/properties/v/type"] == (1, 1)  # document 2's v is flagged
    assert found["/anyOf/1/properties/v/type"] == (1, 1)


# ---------------------------------------------------------------------------
# Relaxed and new records
# ---------------------------------------------------------------------------


def test_relaxed_and_relaxation_created_records_are_finalized() -> None:
    m = model(
        {
            "type": "object",
            "properties": {"a": {"type": "integer", "maximum": 2}},
            "required": ["a"],
            "additionalProperties": False,
        }
    )
    a = m.root.kw["properties"]["a"]
    m.provenance[ConstraintId(a.nid, "maximum")] = mdl_record("values.bounds", Support(1, 1), 0.9)
    recorded(m)
    docs: list[JSON] = [{"a": 1}, {"a": 4}, {"a": 2, "c": {"d": True}}]
    report = InferenceReport()
    check_soundness(m, docs, engine=DecisionEngine({"decide.mdl": False}, report=report), validator=V, report=report)
    finalize_provenance(m, docs, validator=V)
    records = by_pointer(m)

    # A relaxed bound keeps its decision Confidence; its Support covers every number.
    maximum = records["/properties/a/maximum"]
    assert maximum.changes and maximum.support == Support(3, 3) and maximum.confidence == 0.9
    # The Naive_Merge subschema of the new property holds Constraints from the relaxation decision.
    for pointer in ("/properties/c/type", "/properties/c/properties", "/properties/c/properties/d/type"):
        record = records[pointer]
        assert record.technique == "fallback:relaxation" and record.fallback is not None, pointer
        assert record.support == Support(1, 1) and record.confidence == 2 / 3, pointer  # refreshed from (0, 0)
    assert records["/properties/c/required/0"].support == Support(1, 1)
    # A relaxed Laplace default is refreshed too: the closed Record took the add-properties change.
    closed = records["/additionalProperties"]
    assert closed.changes and closed.support == Support(3, 3)
    assert closed.confidence == laplace_confidence(Support(3, 3))
    assert records["/properties"].support == Support(3, 3) and not records["/properties"].changes
    check_records(EMITTER.layout(m)[1], m.provenance)


# ---------------------------------------------------------------------------
# Pipeline runs
# ---------------------------------------------------------------------------

ORDERS: list[Any] = [
    {
        "id": i,
        "status": ["new", "paid", "sent"][i % 3],
        "lines": [{"sku": f"s{j}", "qty": j % 3 + 1} for j in range(i % 4)],
        "meta": {"tags": ["a", "b"][: i % 3], "source": "web" if i % 2 else "app"},
        **({"note": f"note {i}"} if i % 5 == 0 else {}),
    }
    for i in range(24)
]
# A Closed_Enum beside objects: the mixed-type rule's closed_enum and remaining_types branches.
ENUM_AND_OBJECTS: list[Any] = ["red", "green"] * 4 + [{"a": 1}, {"a": 2}]
MIXED: list[Any] = [
    *ORDERS[:6],
    *({f"user{3 * i + j}": {"score": i + j} for j in range(3)} for i in range(8)),
    *([i, f"n{i}"] for i in range(8)),
    1,
    2.5,
    "text",
    None,
    True,
]


def run_with_snapshot(
    monkeypatch: pytest.MonkeyPatch, docs: list[Any], config: RunConfiguration
) -> tuple[PipelineResult, dict[ConstraintId, ProvenanceRecord]]:
    """A pipeline run, with the records as they stood just before finalization."""
    before: dict[ConstraintId, ProvenanceRecord] = {}
    original = pipeline.finalize_provenance

    def spy(m: SchemaModel, *args: Any, **kwargs: Any) -> Any:
        before.update(m.provenance)
        return original(m, *args, **kwargs)

    monkeypatch.setattr(pipeline, "finalize_provenance", spy)
    return infer_documents(docs, config), before


@pytest.mark.parametrize("config", [ONE_WORKER, ALL_OFF], ids=["defaults", "all-off"])
@pytest.mark.parametrize("docs", [ORDERS, MIXED, ENUM_AND_OBJECTS], ids=["orders", "mixed", "enum-and-objects"])
def test_finalization_changes_only_supports_and_default_confidences(
    monkeypatch: pytest.MonkeyPatch, docs: list[Any], config: RunConfiguration
) -> None:
    result, before = run_with_snapshot(monkeypatch, docs, config)
    after = result.model.provenance
    assert set(after) == set(result.pointers) == set(before)  # exactly one record per emitted Constraint
    for cid, record in after.items():
        old = before[cid]
        where = result.pointers[cid]
        assert without(record, "support", "confidence") == without(old, "support", "confidence"), where
        assert record.support.consistent <= record.support.total, where
        if has_default_confidence(old):
            assert record.confidence == laplace_confidence(record.support), where
        else:
            assert record.confidence == old.confidence, where
    if config is ALL_OFF:
        # Every Constraint comes from a Baseline_Rule or a Fallback_Behavior, so every Confidence is a default.
        assert all(r.technique.startswith(("baseline:", "fallback:")) for r in after.values())
        assert all(r.confidence == laplace_confidence(r.support) for r in after.values())


def test_mixed_type_branches_count_every_row_at_the_path() -> None:
    result = infer_documents(ENUM_AND_OBJECTS, ONE_WORKER)
    records = {result.pointers[cid]: record for cid, record in result.model.provenance.items()}
    enum, objects = records["/anyOf/0"], records["/anyOf/1"]
    assert (enum.details["branch"], objects.details["branch"]) == ("closed_enum", "remaining_types")
    assert enum.support == Support(8, 10) and objects.support == Support(2, 10)
    assert records["/anyOf/1/required/0"].support == Support(2, 2)


def test_samples_keep_their_counts_while_support_counts_everything() -> None:
    sampled = ProvenanceRecord.laplace("variants.dbscan", STALE, resolution=PRIORITY, sample=SampleInfo(2, 5))
    m = model({"type": "integer"})
    m.provenance[ConstraintId(m.root.nid, "type")] = sampled
    finalize_provenance(m, [1, 2, 3, 4, 5, 6], validator=V)
    record = m.provenance[ConstraintId(m.root.nid, "type")]
    assert record.sample == SampleInfo(2, 5) and record.support == Support(6, 6)


def test_finalization_runs_in_the_soundness_stage(monkeypatch: pytest.MonkeyPatch) -> None:
    def broken(*args: Any, **kwargs: Any) -> Any:
        raise RuntimeError("injected failure")

    monkeypatch.setattr(pipeline, "finalize_provenance", broken)
    with pytest.raises(InferenceError) as caught:
        infer_documents([1, 2], ONE_WORKER)
    assert caught.value.stage == SOUNDNESS_STAGE

