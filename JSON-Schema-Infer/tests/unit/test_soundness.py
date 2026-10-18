"""Unit tests for pipeline/soundness.py (task 15.3).

Requirements 3.1–3.4, 3.6, and 3.7: validation of every non-flagged document
of the covering segment, relaxation through the ``relaxation`` Decision_Point,
the removal loop after ``soundness.max_passes``, the RelaxationRecords and
Support of 3.6, and the counts of 3.4. Over-tight Constraints are injected
into hand-written models. Properties 7 and 9 (tasks 16.8, 16.9) check the
same over generated collections.
"""

from __future__ import annotations

from typing import Any

import numpy as np
import pytest

from jsinfer.core.determinism import encode_canonical
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import build_store
from jsinfer.core.paths import MAPVAL_STEP
from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.ingest import DocumentStore
from jsinfer.io.validator import CompileError, Validator
from jsinfer.pipeline.soundness import SoundnessResult, check_soundness, flagged_documents

V = Validator()
EMITTER = SchemaEmitter()


def model(schema: dict[str, Any] | bool) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(schema))


def run(
    m: SchemaModel, docs: list[JSON] | DocumentStore, *, mdl: bool = False, **kwargs: Any
) -> tuple[SoundnessResult, InferenceReport]:
    report = InferenceReport()
    engine = DecisionEngine({"decide.mdl": mdl}, report=report)
    result = check_soundness(m, docs, engine=engine, validator=V, report=report, **kwargs)
    return result, report


def emitted(m: SchemaModel) -> JSON:
    doc, _ = EMITTER.layout(m)
    return doc


def all_valid(m: SchemaModel, docs: list[JSON]) -> bool:
    compiled = V.compile(emitted(m))  # type: ignore[arg-type]
    return all(V.is_valid(compiled, doc) for doc in docs)


def nid(m: SchemaModel, pointer: str) -> int:
    return m.resolve(pointer).nid


def store_of(docs: list[JSON]) -> DocumentStore:
    parts = [encode_canonical(doc) for doc in docs]
    offsets = np.zeros(len(parts) + 1, dtype=np.int64)
    np.cumsum([len(part) for part in parts], out=offsets[1:])
    return DocumentStore(b"".join(parts), offsets)


def check_bookkeeping(result: SoundnessResult, report: InferenceReport) -> None:
    """The 3.4 counts agree with the change records (what Property 9 checks)."""
    summary = result.summary
    assert report.soundness == (summary,)
    assert len(result.changes) == summary.relaxations + summary.removals
    assert summary.removals == sum(1 for c in result.changes if c.counted_as == "removal")
    entries = [c.removed for c in result.changes if c.removed is not None]
    assert sorted(entries, key=lambda e: e.sort_key()) == list(summary.removed)
    assert summary.validated == len(result.validated)
    assert summary.failed_first_pass == len(result.failed_first_pass) <= summary.validated


# A Record with several over-tight Constraints: a bound, a type, an enum, a required
# entry, and a closed Record, each violated by a different document.
OVER_TIGHT = {
    "type": "object",
    "properties": {"a": {"type": "integer", "maximum": 140}, "b": {"type": "string", "enum": ["x", "y"]}},
    "required": ["a", "b"],
    "additionalProperties": False,
}
OVER_TIGHT_DOCS: list[JSON] = [
    {"a": 1, "b": "x"},
    {"a": 150, "b": "y"},
    {"a": 2.5, "c": True},
    {"a": 3, "b": "z", "d": [1, 2]},
]


# ---------------------------------------------------------------------------
# Clean runs
# ---------------------------------------------------------------------------


def test_valid_documents_take_one_pass_and_change_nothing() -> None:
    m = model({"type": "object", "properties": {"a": {"type": "integer"}}})
    before = emitted(m)
    result, report = run(m, [{"a": 1}, {"a": 2}, {}])
    summary = result.summary
    assert (summary.validated, summary.failed_first_pass, summary.passes) == (3, 0, 1)
    assert (summary.relaxations, summary.removals, summary.pass_limit_reached) == (0, 0, False)
    assert result.changes == () and summary.removed == ()
    assert emitted(m) == before
    assert report.decisions == ()
    check_bookkeeping(result, report)


def test_nothing_to_validate_still_compiles_the_schema() -> None:
    result, _ = run(model({"type": "integer"}), [])
    assert (result.summary.validated, result.summary.passes) == (0, 0)
    with pytest.raises(CompileError):
        run(model({"pattern": "("}), [1])


# ---------------------------------------------------------------------------
# Relaxation (3.3) and records (3.6)
# ---------------------------------------------------------------------------


def test_fallback_relaxes_injected_over_tight_constraints_least_widening_first() -> None:
    m = model(OVER_TIGHT)
    a, b, root = nid(m, "/properties/a"), nid(m, "/properties/b"), m.root.nid
    m.provenance[ConstraintId(a, "maximum")] = ProvenanceRecord.laplace("values.bounds", Support(3, 3))
    m.provenance[ConstraintId(a, "type")] = ProvenanceRecord.baseline("type", Support(3, 3))
    m.provenance[ConstraintId(root, "required", "b")] = ProvenanceRecord.baseline("required", Support(3, 3))
    result, report = run(m, OVER_TIGHT_DOCS)

    assert all_valid(m, OVER_TIGHT_DOCS)
    assert emitted(m) == {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "a": {"type": "number", "maximum": 150},
            "b": {"type": "string", "enum": ["x", "y", "z"]},
            "c": {"type": "boolean"},
            "d": {"type": "array", "items": {"type": "integer"}},
        },
        "additionalProperties": False,
        "required": ["a"],
    }
    summary = result.summary
    assert (summary.validated, summary.failed_first_pass, summary.passes) == (4, 3, 2)
    assert (summary.relaxations, summary.removals, summary.pass_limit_reached) == (5, 0, False)
    # Failures in target order (the node's pointer, keyword, item), whatever the documents' order.
    assert [(c.failure.keyword, c.failure.documents, c.change.edit) for c in result.changes] == [
        ("additionalProperties", (2, 3), "add-properties"),  # the closed root Record (root pointer first)
        ("required", (2,), "delete-entry"),  # required b
        ("maximum", (1,), "rewrite"),  # maximum of a, although document 1 fails first
        ("type", (2,), "rewrite"),  # type of a
        ("enum", (3,), "rewrite"),  # enum of b
    ]
    check_bookkeeping(result, report)

    # A relaxed Constraint keeps its record, gains the change, and has its Support recomputed.
    maximum = m.provenance[ConstraintId(a, "maximum")]
    assert maximum.technique == "values.bounds"
    (change,) = maximum.changes
    assert change.before == {"maximum": 140} and change.after == {"maximum": 150}
    assert change.documents == (1,)
    assert change.resolution is not None and change.resolution.method == "fallback"
    assert [c.outcome for c in change.resolution.candidates] == ["loosen-bound:maximum=150", "remove:maximum"]
    assert maximum.support == Support(4, 4)  # every number at $.a
    assert m.provenance[ConstraintId(a, "type")].technique == "baseline:type"
    assert m.provenance[ConstraintId(a, "type")].changes[0].after == {"type": ["number"]}
    # A Constraint without a record gets one from the relaxation decision.
    enum = m.provenance[ConstraintId(b, "enum")]
    assert enum.technique == "fallback:relaxation" and enum.fallback is not None
    assert enum.support == Support(3, 3)  # the strings at $.b
    # A removed Constraint leaves the model's provenance and goes to the report.
    assert ConstraintId(root, "required", "b") not in m.provenance
    (removed,) = summary.removed
    assert removed.pointer == "/required/1"
    assert removed.change.removed and removed.change.before == {"required": ["b"]}
    assert removed.change.documents == (2,)
    # One decision per relaxation, at the Path of the Failure's node.
    assert [(d.kind, d.path) for d in report.decisions] == [
        ("relaxation", "$"),
        ("relaxation", "$"),
        ("relaxation", "$.a"),
        ("relaxation", "$.a"),
        ("relaxation", "$.b"),
    ]


def test_mdl_scores_every_candidate_on_the_nodes_observations() -> None:
    m = model({"type": "integer", "maximum": 10})
    docs: list[JSON] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    result, report = run(m, docs, mdl=True)
    assert all_valid(m, docs)
    (applied,) = result.changes
    resolution = applied.record.resolution
    assert resolution is not None and resolution.method == "mdl"
    scores = [c.score for c in resolution.candidates]
    assert all(score is not None for score in scores)
    assert resolution.selected == scores.index(min(scores))  # type: ignore[type-var]
    assert {c.n_scored for c in resolution.candidates} == {12}  # every instance at the node, the failing one included
    (decision,) = report.decisions
    assert decision.cutoff is not None and decision.cutoff.source == "MDL comparison"
    check_bookkeeping(result, report)


@pytest.mark.parametrize(
    ("schema", "docs", "label"),
    [
        ({"type": "string", "minLength": 3}, ["abcd", "ab"], "loosen-bound:minLength=2"),
        ({"type": "string", "pattern": "^(?:[a-z]{3})$(?!\n)"}, ["abc", "ab1"], "re-induce:pattern"),
        ({"type": "string", "format": "date"}, ["2024-01-02", "soon"], "remove:format"),
        ({"const": 1}, [1, 2], "add-values:const->enum+1"),
        ({"type": "array", "uniqueItems": True}, [[1, 2], [1, 1]], "remove:uniqueItems"),
        ({"type": "array", "maxItems": 1}, [[1], [1, 2]], "loosen-bound:maxItems=2"),
        ({"not": {"type": "string"}}, [1, "s"], "remove:not"),
        ({"type": "object", "minProperties": 2}, [{"a": 1, "b": 2}, {}], "loosen-bound:minProperties=0"),
    ],
)
def test_each_kind_of_over_tight_constraint_is_relaxed(schema: dict[str, Any], docs: list[JSON], label: str) -> None:
    m = model(schema)
    result, report = run(m, docs)
    assert all_valid(m, docs)
    (decision,) = report.decisions
    selected = decision.resolution.selected_candidate.outcome
    assert selected.split(":")[0] == label.split(":")[0]
    if not label.startswith("re-induce"):
        assert selected == label
    check_bookkeeping(result, report)


def test_one_of_matched_twice_is_converted_and_its_branch_records_move() -> None:
    m = model({"oneOf": [{"type": "integer"}, {"minimum": 0}]})
    n = m.root.nid
    for index in (0, 1):
        m.provenance[ConstraintId(n, "oneOf", index)] = ProvenanceRecord.laplace("structure.variants", Support(1, 2))
    result, report = run(m, [5, -1])
    assert emitted(m)["anyOf"] == [{"type": "integer"}, {"minimum": 0}]  # type: ignore[index]
    assert "oneOf" not in emitted(m)  # type: ignore[operator]
    (applied,) = result.changes
    assert applied.change.edit == "convert" and applied.failure.keyword == "oneOf"
    assert applied.failure.branches == ((), ())  # both branches accepted 5
    assert applied.constraints == (ConstraintId(n, "anyOf", 0), ConstraintId(n, "anyOf", 1))
    assert not any(cid.keyword == "oneOf" for cid in m.provenance)
    first, second = m.provenance[ConstraintId(n, "anyOf", 0)], m.provenance[ConstraintId(n, "anyOf", 1)]
    assert first.technique == second.technique == "structure.variants"
    assert first.changes[0].after == {"anyOf": [{"type": "integer"}, {"minimum": 0}]}
    # Branch Support counts the instances each branch accepts.
    assert first.support == Support(2, 2) and second.support == Support(1, 2)
    check_bookkeeping(result, report)


def test_one_of_converted_beside_any_of_gives_the_new_all_of_a_record() -> None:
    m = model({"anyOf": [{"type": "integer"}, {"type": "string"}], "oneOf": [{"type": "integer"}, {"minimum": 0}]})
    n = m.root.nid
    # Distinct Confidences show which old branch record each new branch took (Confidences are not recomputed).
    m.provenance[ConstraintId(n, "oneOf", 0)] = ProvenanceRecord.laplace("structure.variants", Support(1, 2))
    m.provenance[ConstraintId(n, "oneOf", 1)] = ProvenanceRecord.laplace("structure.variants", Support(2, 2))
    result, report = run(m, [5, -1])
    assert all_valid(m, [5, -1])
    branches = [{"type": "integer"}, {"minimum": 0}]
    assert emitted(m)["allOf"] == [{"anyOf": branches}]  # type: ignore[index]
    assert "oneOf" not in emitted(m)  # type: ignore[operator]
    (applied,) = result.changes
    assert applied.change.edit == "convert" and applied.failure.keyword == "oneOf"
    w = m.root.kw["allOf"][0].nid
    owner_all_of = ConstraintId(n, "allOf")
    assert applied.constraints == (owner_all_of, ConstraintId(w, "anyOf", 0), ConstraintId(w, "anyOf", 1))
    assert not any(cid.keyword == "oneOf" for cid in m.provenance)
    # The branch records move to the wrapper's branches, in order.
    first, second = m.provenance[ConstraintId(w, "anyOf", 0)], m.provenance[ConstraintId(w, "anyOf", 1)]
    assert first.technique == second.technique == "structure.variants"
    assert (first.confidence, second.confidence) == (0.5, 0.75)
    assert first.support == Support(2, 2) and second.support == Support(1, 2)
    # The new allOf Constraint gets a record from the relaxation decision (3.3), with its Support recomputed.
    all_of = m.provenance[owner_all_of]
    assert all_of.technique == "fallback:relaxation" and all_of.fallback is not None
    (change,) = all_of.changes
    assert change.before == {"oneOf": branches} and change.after == {"allOf": [{"anyOf": branches}]}
    assert change.documents == (0,)
    assert change.resolution is not None and change.resolution.method == "fallback"
    assert all_of.support == Support(2, 2)
    check_bookkeeping(result, report)


def test_any_of_without_a_match_relaxes_a_branch_from_its_branch_failures() -> None:
    m = model({"anyOf": [{"type": "integer", "maximum": 3}, {"type": "string"}]})
    result, report = run(m, [1, "a", 5])
    (applied,) = result.changes
    branch0, branch1 = applied.failure.branches
    assert [f.keyword for f in branch0] == ["maximum"] and [f.keyword for f in branch1] == ["type"]
    assert report.decisions[0].resolution.selected_candidate.outcome == "relax-branch:anyOf/0"
    assert emitted(m)["anyOf"][0] == {"type": "integer", "maximum": 5}  # type: ignore[index]
    assert all_valid(m, [1, "a", 5])
    check_bookkeeping(result, report)


@pytest.mark.parametrize("mdl", [False, True])
def test_an_edit_in_defs_applies_at_every_referencing_site(mdl: bool) -> None:
    schema = {
        "$defs": {"N": {"type": "integer", "maximum": 10}},
        "properties": {"x": {"$ref": "#/$defs/N"}, "y": {"$ref": "#/$defs/N"}},
        "required": ["x", "y", "z"],
    }
    m = model(schema)
    docs: list[JSON] = [{"x": 1, "y": 2, "z": 0}, {"x": 20, "y": 30, "z": 0}, {"x": 3, "y": 4}]
    result, report = run(m, docs, mdl=mdl)
    defs = nid(m, "/$defs/N")
    shared = [c for c in result.changes if c.failure.nid == defs]
    (applied,) = shared  # one Failure for the shared node, with the values of both sites
    assert applied.failure.values == (20, 30)
    # The shared node's Path has the fewest steps, then comes first in Path order (the report sorts by Path).
    assert [d.path for d in report.decisions] == ["$", "$.x"]
    if not mdl:
        assert emitted(m)["$defs"]["N"] == {"type": "integer", "maximum": 30}  # type: ignore[index]
        assert m.provenance[ConstraintId(defs, "maximum")].support == Support(6, 6)  # instances from both sites
    assert all_valid(m, docs)
    check_bookkeeping(result, report)


def test_a_guard_removes_a_not_that_shares_the_edited_definition() -> None:
    schema = {
        "$defs": {"D": {"type": "integer", "maximum": 3}},
        "properties": {"x": {"$ref": "#/$defs/D"}, "y": {"not": {"$ref": "#/$defs/D"}}},
    }
    m = model(schema)
    docs: list[JSON] = [{"x": 5}, {"y": 2}]
    result, report = run(m, docs)
    # Loosening D would widen the not's subschema, so the candidate also removes that not;
    # the second document's Failure at the not is then gone and is skipped.
    assert len(report.decisions) == 1
    assert [(c.change.edit, c.change.guard) for c in result.changes] == [("rewrite", False), ("rewrite", True)]
    assert [e.pointer for e in result.summary.removed] == ["/properties/y/not"]
    assert result.summary.relaxations == 2 and result.summary.removals == 0
    assert all_valid(m, docs)
    check_bookkeeping(result, report)


def test_false_subschemas_and_closed_items_are_replaced_by_naive_merges() -> None:
    m = model({"properties": {"v": False}, "prefixItems": [{"type": "string"}], "items": False})
    docs: list[JSON] = [{"v": 1}, ["a", 1, 2]]
    result, report = run(m, docs)
    assert emitted(m)["properties"] == {"v": {"type": "integer"}}  # type: ignore[index]
    assert emitted(m)["items"] == {"type": "integer"}  # type: ignore[index]
    # Target order: the root's items (pointer "") before the false node at /properties/v.
    assert [c.change.edit for c in result.changes] == ["rewrite", "replace"]
    assert all_valid(m, docs)
    check_bookkeeping(result, report)


@pytest.mark.parametrize("name", ["x", ""])
def test_a_false_additional_properties_without_properties_fails_on_the_object(name: str) -> None:
    # Property 9 (task 16.9) counterexample: a closed Record without properties under the key "". Without
    # properties, jsonschema-rs reports additionalProperties: false as a falseSchema error at the object (not
    # at a member, as for items: false), so taking the parent instance relaxed with the whole document as the
    # failing value (only removal fixed it), and under "" the error named no subschema of the model.
    m = model({"type": "object", "properties": {name: {"type": "object", "additionalProperties": False}}})
    docs: list[JSON] = [{name: {}}, {name: {"a": 1, "b": "s"}}]
    result, report = run(m, docs)
    (applied,) = result.changes
    assert applied.failure.target == (nid(m, f"/properties/{name}"), "additionalProperties", None)
    assert applied.failure.values == ({"a": 1, "b": "s"},) and applied.failure.documents == (1,)
    assert applied.change.edit == "add-properties"
    assert emitted(m)["properties"][name] == {  # type: ignore[index]
        "type": "object",
        "properties": {"a": {"type": "integer"}, "b": {"type": "string"}},
        "additionalProperties": False,
    }
    assert all_valid(m, docs)
    check_bookkeeping(result, report)


def test_dependent_required_names_each_antecedent() -> None:
    m = model({"dependentRequired": {"p": ["q"], "r": ["q"], "s": ["q"]}})
    result, report = run(m, [{"p": 1, "r": 1}])
    assert [c.failure.target for c in result.changes] == [
        (m.root.nid, "dependentRequired", "p"),
        (m.root.nid, "dependentRequired", "r"),
    ]
    assert [e.pointer for e in result.summary.removed] == ["/dependentRequired/p", "/dependentRequired/r"]
    assert emitted(m)["dependentRequired"] == {"s": ["q"]}  # type: ignore[index]
    check_bookkeeping(result, report)


def test_property_names_and_contains_are_keywords_of_their_owner() -> None:
    schema = {
        "$defs": {"K": {"maxLength": 1}},
        "properties": {"n": {"propertyNames": {"$ref": "#/$defs/K"}}, "y": {"contains": {"type": "string"}}},
    }
    m = model(schema)
    result, report = run(m, [{"n": {"long": 1}, "y": [1]}])
    assert [c.failure.target for c in result.changes] == [
        (nid(m, "/properties/n"), "propertyNames", None),
        (nid(m, "/properties/y"), "contains", None),
    ]
    assert [e.pointer for e in result.summary.removed] == ["/properties/n/propertyNames", "/properties/y/contains"]
    check_bookkeeping(result, report)


# ---------------------------------------------------------------------------
# Pass limit and removal loop (3.7)
# ---------------------------------------------------------------------------


def test_reaching_the_pass_limit_runs_the_removal_loop() -> None:
    m = model(OVER_TIGHT)
    a = nid(m, "/properties/a")
    m.provenance[ConstraintId(a, "maximum")] = ProvenanceRecord.laplace("values.bounds", Support(3, 3))
    result, report = run(m, OVER_TIGHT_DOCS, max_passes=1)
    summary = result.summary
    assert summary.pass_limit_reached
    assert summary.relaxations == 0 and summary.removals == len(summary.removed) == 5
    assert all(c.phase == "removal" and c.record.resolution is None for c in result.changes)
    assert [e.pointer for e in summary.removed] == [
        "/additionalProperties",
        "/properties/a/maximum",
        "/properties/a/type",
        "/properties/b/enum",
        "/required/1",
    ]
    assert ConstraintId(a, "maximum") not in m.provenance
    assert summary.passes == 2 and report.decisions == ()
    assert all_valid(m, OVER_TIGHT_DOCS)
    check_bookkeeping(result, report)


def test_the_removal_loop_removes_a_failing_any_of_as_a_whole() -> None:
    m = model({"anyOf": [{"type": "integer", "maximum": 3}, {"type": "integer", "minimum": 10}]})
    n = m.root.nid
    m.provenance[ConstraintId(n, "anyOf", 0)] = ProvenanceRecord.laplace("structure.variants", Support(1, 2))
    docs: list[JSON] = [1, 11, 5]
    result, report = run(m, docs, max_passes=1)
    assert result.summary.pass_limit_reached and result.summary.removals == 1
    (entry,) = result.summary.removed
    assert entry.pointer == "/anyOf"  # one record for the keyword, holding both branches
    assert entry.change.before == {"anyOf": [{"type": "integer", "maximum": 3}, {"type": "integer", "minimum": 10}]}
    assert not any(cid.nid == n for cid in m.provenance)
    assert all_valid(m, docs)
    check_bookkeeping(result, report)


def test_default_max_passes_relax_before_any_removal() -> None:
    m = model(OVER_TIGHT)
    result, report = run(m, OVER_TIGHT_DOCS, max_passes=None)
    assert not result.summary.pass_limit_reached and result.summary.removals == 0
    assert all(c.phase == "relaxation" for c in result.changes)
    check_bookkeeping(result, report)


@pytest.mark.parametrize("max_passes", [1, 3])
def test_failures_under_empty_names_are_mapped_to_their_constraints(max_passes: int) -> None:
    # Property 7 (task 16.8) counterexample: a Map whose value Record has a property named "", under an
    # array, with maxLength 0 injected there. jsonschema-rs drops the empty names from its error paths,
    # which made this raise SoundnessError ("does not end at a keyword") or KeyError before the fix.
    schema = {
        "type": "array",
        "items": {
            "type": "object",
            "properties": {"": {"additionalProperties": {"properties": {"": {"type": "string", "maxLength": 0}}}}},
        },
    }
    docs: list[JSON] = [[], [{"": {"shipping": {"": "0.0.0.0", "a": "red"}}}], [{"": {"billing": {"": "x"}}}]]
    m = model(schema)
    result, report = run(m, docs, max_passes=max_passes)
    assert result.summary.failed_first_pass == 2
    (change,) = result.changes
    assert change.failure.keyword == "maxLength" and change.failure.nid == nid(m, "/items/properties//additionalProperties/properties/")
    assert change.failure.documents == (1, 2)
    assert all_valid(m, docs)
    check_bookkeeping(result, report)


# ---------------------------------------------------------------------------
# Flags, segments, documents, and determinism (3.2, 27.5)
# ---------------------------------------------------------------------------


def test_flagged_documents_and_other_segments_are_not_validated() -> None:
    m = model({"type": "integer", "maximum": 10})
    docs: list[JSON] = [1, 50, 2, 70, 3]
    before = emitted(m)
    result, report = run(m, docs, flagged=[1], segment=[0, 1, 2, 4])
    summary = result.summary
    assert (summary.flagged_documents, summary.validated, summary.failed_first_pass) == (1, 3, 0)
    assert result.validated == (0, 2, 4) and result.flagged == (1,)
    assert emitted(m) == before
    check_bookkeeping(result, report)


def test_flagged_documents_include_documents_holding_flagged_items() -> None:
    docs: list[JSON] = [{"a": 1}, {"a": 2}, {"a": 3}, {"a": 4}]
    store = build_store(docs)
    store.flags.flag_documents([0])
    column = store.column((MAPVAL_STEP,))
    store.flags.flag_items((MAPVAL_STEP,), np.flatnonzero(column.doc == 2))
    assert flagged_documents(store).tolist() == [0, 2]
    m = model({"properties": {"a": {"maximum": 2}}})
    result, _ = run(m, docs, flagged=flagged_documents(store))
    assert result.validated == (1, 3) and result.summary.failed_first_pass == 1


def test_results_do_not_depend_on_chunking_workers_or_document_container() -> None:
    def outcome(documents: list[JSON] | DocumentStore, **kwargs: Any) -> tuple[bytes, bytes, list[bytes]]:
        m = model(OVER_TIGHT)
        result, _ = run(m, documents, mdl=True, **kwargs)
        records = [encode_canonical(c.record.to_json()) for c in result.changes]
        return EMITTER.emit(m), encode_canonical(result.summary.to_json()), records

    expected = outcome(OVER_TIGHT_DOCS)
    for chunk_size, workers in ((1, 1), (3, 1), (1, 2), (100, 2)):
        assert outcome(OVER_TIGHT_DOCS, chunk_size=chunk_size, workers=workers) == expected
    assert outcome(store_of(OVER_TIGHT_DOCS), chunk_size=2) == expected


# Property 47 (task 19.5) counterexample: 23 lists whose analyzer model the test tightened with maxItems 0
# and minItems 2 at the root. Document 0 fails both bounds and document 1 only minItems. While groups were
# ordered by their first failing document, maxItems was relaxed first and removed; with document 0 moved to
# the end, minItems was relaxed first and maxItems, scored against the model without minItems, was loosened
# to 3 (task 19.10).
LISTS: list[JSON] = [
    [6], [], [8], [6], [7], [9], [], [2, 4], [], [], [4, 8], [9, 2, 4],
    [], [1, 5, 7], [1, 5], [5], [3, 8], [7], [4, 0], [0, 1], [6, 0, 9], [5], [],
]  # fmt: skip
LISTS_ITEMS = {"type": "integer", "minimum": 0, "maximum": 9}


def test_the_processing_order_does_not_depend_on_the_document_order() -> None:
    def outcome(order: list[int]) -> tuple[JSON, list[tuple[str, JSON]], list[JSON]]:
        """The emitted model, the change records in the numbering of ``LISTS``, and the decisions."""
        m = model({"type": "array", "items": LISTS_ITEMS, "maxItems": 0, "minItems": 2})
        result, report = run(m, [LISTS[i] for i in order], mdl=True)
        check_bookkeeping(result, report)
        records = [
            (c.failure.keyword, {**c.record.to_json(), "documents": sorted(order[d] for d in c.record.documents)})
            for c in result.changes
        ]
        return emitted(m), records, [d.to_json() for d in report.decisions]

    identity = list(range(len(LISTS)))
    expected = outcome(identity)
    schema, records, _ = expected
    assert schema == {"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "array", "items": LISTS_ITEMS}
    # Both bounds of the root, in keyword order, each removed by the MDL choice.
    assert [(keyword, record["removed"]) for keyword, record in records] == [("maxItems", True), ("minItems", True)]
    for order in (identity[1:] + identity[:1], identity[::-1]):
        assert outcome(order) == expected


def test_arguments_are_checked() -> None:
    m = model({"type": "integer"})
    with pytest.raises(ValueError, match="soundness.max_passes"):
        check_soundness(m, [1], max_passes=0)
    with pytest.raises(ValueError, match="workers"):
        check_soundness(m, [1], workers=0)
    with pytest.raises(ValueError, match="out of range"):
        check_soundness(m, [1], flagged=[3])
    with pytest.raises(TypeError):
        check_soundness(m, "not documents")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        check_soundness({"type": "integer"}, [1])  # type: ignore[arg-type]
