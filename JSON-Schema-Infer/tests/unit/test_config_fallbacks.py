"""Unit tests for configuration-specific Fallback_Behaviors (task 19.9).

Each test runs the in-process pipeline (``pipeline.infer.infer_documents``,
one worker) under the configuration a criterion names and checks the
documented Fallback_Behavior in the emitted schema and its provenance: the
Provenance_Record of each Constraint the outcome produces (``fallback:<kind>``
producer, fallback indicator and reason, ``details.fallback_behavior``, no
cut-off, the Laplace default Confidence of the design's "Default
Confidences" row), or, for an outcome that produces no Constraint, the
Inference_Report decision entry (20.7).

Criteria and the component-level tests that already cover them:

- 4.10 (Record-or-Map without its Decision_Model: MDL, else Record).
  ``test_record_map``: ``test_mdl_disabled_falls_back_to_record``,
  ``test_ties_go_to_record_even_with_fewer_map_model_bits`` (the tie rule);
  ``test_decision_engine``: ``test_a_missing_decision_model_falls_through_to_mdl_and_is_recorded``.
- 5.10 (every grouping Technique disabled: one Variant).
  ``test_variants``: ``test_all_grouping_techniques_disabled_is_one_variant``,
  ``test_a_missing_threshold_is_named_by_the_fallback``.
- 5.11 (no Merge-or-Split Decision_Model and no MDL: the priority grouping stays split).
  ``test_variants``: ``test_without_the_decision_model_and_mdl_the_priority_grouping_stays_split``,
  ``test_without_mdl_the_lsh_grouping_selected_by_priority_is_kept``, ``test_without_mdl_the_priority_order_selects``.
- 7.10 (Tuple-or-List without its Decision_Model: Tuple only by a lower DL under MDL, else List).
  ``test_arrays``: ``test_mdl_disabled_falls_back_to_list``, ``test_ties_go_to_list_even_with_fewer_tuple_model_bits``.
- 9.9 (co-occurrence mining disabled: no rules, ``required`` kept).
  ``test_cooccurrence``: ``test_the_fallback_reports_no_candidates``.
- 15.7 (enum estimation disabled: every scalar Path open, whatever the Decision_Model).
  ``test_enums``: ``test_disabled_enum_estimation_falls_back_to_open`` (with the Decision_Model on and off).
- 16.8 (bound generalization disabled: no bound keyword).
  ``test_bounds``: ``test_disabled_bounds_omit_both_bounds_through_the_fallback``;
  ``test_record_map``: ``test_map_node_follows_technique_states``.
- 20.6 (a Learned_Threshold missing from, or invalid in, the Model_Bundle).
  ``test_bundle`` (the reader's Substitutions), ``test_thresholds`` (missing and invalid entries),
  ``test_cooccurrence`` (missing-threshold at the component), and
  ``test_decision_engine``: ``test_a_missing_learned_threshold_drops_the_techniques_candidates``.

The ``decide.dm_*`` Techniques are implemented (task 22.4), so they are on
by default; the stub bundles of this module hold no Decision_Model, so a run
that leaves one on records its ``missing-model`` fallback (22.14). The
Decision_Model branches of 4.10, 5.11, 7.10, and 15.7 (an enabled model that
selects, a missing model, and ``decide.dm_closed_enum`` enabled while
``values.enum_estimation`` is off) run through the pipeline in the last
section, with Decision_Models trained on a few labeled examples.
"""

from __future__ import annotations

import json
import math
from collections import Counter
from collections.abc import Iterable, Mapping
from pathlib import Path
from typing import Any

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.provenance import (
    BASELINE_PREFIX,
    FallbackInfo,
    ProvenanceRecord,
    fallback_technique,
    laplace_confidence,
)
from jsinfer.core.report import DecisionEntry, Substitution
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE
from jsinfer.pipeline.infer import PipelineResult, infer_documents

VALIDATOR = Validator()

MDL = "decide.mdl"
HYBRID = "structure.hybrid_record_map"  # needs decide.mdl (28.8): off with it
DM_RECORD_MAP = "decide.dm_record_map"
DM_TUPLE_LIST = "decide.dm_tuple_list"
DM_MERGE_SPLIT = "decide.dm_merge_split"
DM_CLOSED_ENUM = "decide.dm_closed_enum"
ENUM_ESTIMATION = "values.enum_estimation"
BOUNDS = "values.bounds"
COOCCURRENCE = "rules.cooccurrence"
LSH = "variants.minhash_lsh"
DBSCAN = "variants.dbscan"
HDBSCAN = "variants.hdbscan"
BERNOULLI = "variants.bernoulli_mixture"
GROUPING = (LSH, DBSCAN, HDBSCAN, BERNOULLI)

BOUND_KEYWORDS = frozenset(
    {"minimum", "maximum", "minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties"}
)

# Learned_Thresholds of the stub bundles. TIGHT keeps LSH from joining signatures below 0.9 Jaccard similarity.
GROUPING_TIGHT: dict[str, int | float] = {
    "lsh.keyset_jaccard": 0.9,
    "dbscan.eps": 0.05,
    "dbscan.min_samples": 1,
    "hdbscan.min_cluster_size": 2,
}
LOOSE: dict[str, int | float] = {
    "lsh.keyset_jaccard": 0.5,
    "dbscan.eps": 0.5,
    "dbscan.min_samples": 2,
    "hdbscan.min_cluster_size": 2,
    "cooccur.min_support": 1,
    "cooccur.min_confidence": 0.5,
}
COOCCUR_THRESHOLDS = ("cooccur.min_confidence", "cooccur.min_support")


# ---------------------------------------------------------------------------
# Collections
# ---------------------------------------------------------------------------

# Objects whose keys are data: a Map by MDL.
USER_MAPS = [{f"user{3 * i + j}": {"score": i + j} for j in range(3)} for i in range(20)]
# Fixed field names: a Record by MDL.
RECORDS = [{"name": f"n {i}", "age": i % 7, "email": f"e{i}@example.org"} for i in range(30)]
# Fixed-length rows whose positions each hold one type: a Tuple by MDL.
ROWS = [[f"n{i}", i, i % 2 == 0] for i in range(30)]
# Variable-length integer lists: a List by MDL.
LISTS = [list(range(i % 10)) for i in range(30)]
# Two object families under one key.
NESTED = [{"id": i, "obj": {"x": i, "y": "a b"} if i % 2 else {"p": True, "q": [i]}} for i in range(20)]
# Two families that MDL groups into one Variant, while LSH at TIGHT keeps them apart.
PETS = [{"name": f"p{i}", "kind": "cat", "lives": i % 9} for i in range(12)] + [
    {"name": f"d{i}", "kind": "dog", "bark": "loud"} for i in range(12)
]
# Optional fields that always travel together, at the root and in a nested Record.
RULED = [
    {
        "id": i,
        **({"street": f"st {i}", "city": f"c {i}"} if i % 2 else {}),
        "contact": {"name": f"n {i}", **({"phone": f"p {i}", "ext": i} if i % 3 == 0 else {})},
    }
    for i in range(30)
]
# Repeated scalar values: Closed_Enums by MDL.
COLORS = [{"color": ["red", "green", "blue"][i % 3], "size": i % 4, "tags": [["a", "b"][i % 2]]} for i in range(40)]
# Numbers, strings without a Format, arrays, and a Map: every bound keyword by MDL.
BOUNDED = [
    {"id": i, "label": f"c {i}", "xs": list(range(i % 5 + 1)), "m": {f"k{j}x{i}": j for j in range(i % 3 + 2)}}
    for i in range(40)
]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def config(bundle: str | None = None, *, disabled: Iterable[str] = ()) -> RunConfiguration:
    """One worker, ``bundle`` (default: the shipped empty bundle), and the ``disabled`` Techniques off."""
    flags: dict[str, Any] = {"workers": 1, **{technique_setting(tid): False for tid in disabled}}
    if bundle is not None:
        flags["model_bundle"] = bundle
    return RunConfiguration.resolve(flags)


def threshold_entry(name: str, value: int | float, *, stored: Any = None) -> dict[str, Any]:
    """The ``thresholds.json`` entry of ``name`` fitted at ``value``; ``stored`` replaces the stored value
    (out of range or NaN) when given."""
    entry = ThresholdRecord.from_spec(
        THRESHOLDS[name], value=value, objective_value=0.0, n_examples=1, training_data="t19.9"
    ).to_json()
    if stored is not None:
        entry["value"] = stored
    return entry


def write_bundle(root: Path, entries: Mapping[str, Any]) -> str:
    """A Model_Bundle directory with the raw threshold ``entries`` and no models."""
    root.mkdir()
    manifest = {"format": BUNDLE_FORMAT, "bundle_id": f"t19.9-{root.name}", "artifacts": {}}
    (root / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
    (root / THRESHOLDS_FILE).write_text(json.dumps(entries), encoding="utf-8")  # NaN is written as the NaN token
    return str(root)


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> dict[str, str]:
    """Stub bundle paths by name, written once."""
    root = tmp_path_factory.mktemp("t19-9-bundles")
    fitted = {
        "grouping": GROUPING_TIGHT,
        "full": LOOSE,
        "support-only": {"cooccur.min_support": 1},
    }
    found = {
        name: write_bundle(root / name, {n: threshold_entry(n, v) for n, v in values.items()})
        for name, values in fitted.items()
    }
    # Out of range (min_support < 1, Jaccard > 1) and non-finite: invalid values (20.6).
    found["invalid"] = write_bundle(
        root / "invalid",
        {
            **{n: threshold_entry(n, v) for n, v in LOOSE.items()},
            "cooccur.min_support": threshold_entry("cooccur.min_support", 1, stored=0),
            "cooccur.min_confidence": threshold_entry("cooccur.min_confidence", 0.5, stored=math.nan),
            "lsh.keyset_jaccard": threshold_entry("lsh.keyset_jaccard", 0.5, stored=1.5),
        },
    )
    return found


def run(documents: list[Any], cfg: RunConfiguration) -> PipelineResult:
    """Run the pipeline; the run completes and its schema accepts every document (``format`` asserted)."""
    result = infer_documents(documents, cfg)
    assert result.report.ok
    compiled = VALIDATOR.compile(result.document)
    assert all(VALIDATOR.is_valid(compiled, doc) for doc in documents)
    return result


def records(result: PipelineResult) -> dict[str, ProvenanceRecord]:
    """Every Provenance_Record by the emitted JSON Pointer of its Constraint."""
    return {result.pointers[cid]: record for cid, record in result.model.provenance.items()}


def decisions(result: PipelineResult, kind: str, path: str | None = None) -> list[DecisionEntry]:
    """The report's decision entries of ``kind`` (at ``path``, when given)."""
    return [e for e in result.report.decisions if e.kind == kind and (path is None or e.path == path)]


def keywords(result: PipelineResult) -> Counter[str]:
    """How often each keyword occurs in the emitted schema (property and definition names excluded)."""
    return Counter(keyword for _, node in result.model.walk() for keyword in node.kw)

def no_cooccurrence_entries(result: PipelineResult) -> bool:
    """No ``dependentRequired`` entry carries a co-occurrence record; Field_Group cliques (6) may still be emitted."""
    return not any("/dependentRequired/" in pointer and record.technique == COOCCURRENCE for pointer, record in records(result).items())



def subschema(document: Any, *steps: str | int) -> Any:
    for step in steps:
        document = document[step]
    return document


def assert_fallback_record(record: ProvenanceRecord, kind: str, fallback: FallbackInfo) -> None:
    """A Constraint produced by ``kind``'s Fallback_Behavior (26.10, 28.12, 20.7, design "Default Confidences")."""
    assert record.technique == fallback_technique(kind)
    assert record.fallback == fallback
    assert record.resolution is not None and record.resolution.method == "fallback"
    assert record.details["fallback_behavior"] == REGISTRY.decision_point(kind).fallback_doc
    assert record.cutoff is None  # a constant outcome applies no cut-off
    assert record.confidence == laplace_confidence(record.support)


def assert_fallback_entry(entry: DecisionEntry, kind: str, fallback: FallbackInfo) -> None:
    """The report entry of a Decision_Point resolved by ``kind``'s Fallback_Behavior."""
    assert entry.kind == kind and entry.resolution.method == "fallback"
    assert entry.fallback == fallback
    assert entry.details["fallback_behavior"] == REGISTRY.decision_point(kind).fallback_doc
    assert entry.cutoff is None
    selected = entry.resolution.candidates[entry.resolution.selected]
    assert selected.outcome == REGISTRY.fallback(kind)() and selected.technique is None


def techniques_named(result: PipelineResult) -> set[str]:
    """Every producer named by a Provenance_Record of the emitted schema."""
    return {record.technique for record in result.model.provenance.values()}


# ---------------------------------------------------------------------------
# 4.10: Record-or-Map without its Decision_Model
# ---------------------------------------------------------------------------


def test_record_or_map_without_its_decision_model_is_resolved_by_mdl() -> None:
    maps = run(USER_MAPS, config())
    assert set(maps.document) >= {"additionalProperties", "propertyNames"} and "properties" not in maps.document
    record = records(maps)["/additionalProperties"]
    # MDL is the Decision_Model's Fallback_Behavior (22.13): recorded with the Decision_Model's indicator.
    assert record.technique == MDL and record.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")
    assert record.resolution.method == "mdl" and record.cutoff.source == "MDL comparison"
    record_score, map_score = (c.score for c in record.resolution.candidates)
    assert [c.outcome for c in record.resolution.candidates] == ["record", "map"] and map_score < record_score
    (entry,) = decisions(maps, "record_or_map", "$")
    assert entry.resolution.method == "mdl" and entry.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")

    recs = run(RECORDS, config())
    assert set(recs.document["properties"]) == {"name", "age", "email"}
    record = records(recs)["/properties"]
    assert record.technique == MDL and record.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")
    record_score, map_score = (c.score for c in record.resolution.candidates)
    assert record.resolution.selected == 0 and record_score < map_score


def test_record_or_map_without_mdl_is_a_record() -> None:
    # Shared definitions (8.1) would fold the equal user Records into one definition: off, to read one entry per Path.
    result = run(USER_MAPS, config(disabled=[MDL, HYBRID, "structure.shared_defs", "structure.recursion"]))  # MDL would say Map
    keys = {key for doc in USER_MAPS for key in doc}
    assert set(result.document["properties"]) == keys and "additionalProperties" not in result.document
    by_pointer = records(result)
    fallback = FallbackInfo(DM_RECORD_MAP, "missing-model")  # the first serving Technique in priority order
    assert_fallback_record(by_pointer["/properties"], "record_or_map", fallback)
    assert by_pointer["/properties"].details["judgment"] == "record"
    # Every object Path takes the fallback, the nested Records included.
    entries = decisions(result, "record_or_map")
    assert sorted(e.path for e in entries) == sorted(["$", *(f"$.{key}" for key in keys)])
    for entry in entries:
        assert_fallback_entry(entry, "record_or_map", fallback)
    assert MDL not in techniques_named(result)


# ---------------------------------------------------------------------------
# 5.10: every grouping Technique disabled
# ---------------------------------------------------------------------------


def test_all_grouping_techniques_disabled_put_every_object_in_one_variant(bundles: dict[str, str]) -> None:
    grouped = run(NESTED, config(bundles["grouping"]))
    assert len(subschema(grouped.document, "properties", "obj", "anyOf")) == 2  # the Techniques split $.obj

    result = run(NESTED, config(bundles["grouping"], disabled=GROUPING))
    obj = subschema(result.document, "properties", "obj")
    assert "anyOf" not in obj and "oneOf" not in obj
    assert set(obj["properties"]) == {"p", "q", "x", "y"} and "required" not in obj  # one merged Variant
    # One Variant produces no Constraint of its own, so the report holds the decision (20.7).
    (entry,) = decisions(result, "variant_grouping")
    assert entry.path == "$.obj"
    assert_fallback_entry(entry, "variant_grouping", FallbackInfo(LSH, "disabled"))
    assert len(entry.resolution.candidates) == 1  # only the fallback outcome: disabled Techniques offer nothing
    assert decisions(result, "merge_or_split") == []  # one candidate Variant: no Judgment_Call
    assert techniques_named(result).isdisjoint(GROUPING)


# ---------------------------------------------------------------------------
# 5.11: no Merge-or-Split Decision_Model and no MDL
# ---------------------------------------------------------------------------


def test_without_the_decision_model_and_mdl_the_priority_grouping_is_kept_split(bundles: dict[str, str]) -> None:
    merged = run(PETS, config(bundles["grouping"]))
    assert not {"anyOf", "oneOf"} & set(merged.document)  # MDL selects a one-Variant grouping

    result = run(PETS, config(bundles["grouping"], disabled=[MDL, HYBRID]))
    (combinator,) = {"anyOf", "oneOf"} & set(result.document)
    branches = result.document[combinator]
    assert [set(branch["properties"]) for branch in branches] == [{"bark", "kind", "name"}, {"kind", "lives", "name"}]
    # The grouping is the first in registry priority order (19.7), recorded with decide.mdl's indicator.
    (grouping,) = decisions(result, "variant_grouping")
    assert grouping.resolution.method == "priority" and grouping.resolution.selected == 0
    assert [c.technique for c in grouping.resolution.candidates] == list(REGISTRY.servers("variant_grouping"))
    assert grouping.fallback == FallbackInfo(MDL, "disabled")
    assert grouping.cutoff.source == "lsh.keyset_jaccard" and grouping.cutoff.value == 0.9
    # Merge-or-Split takes its Fallback_Behavior: keep the candidate Variants separate.
    fallback = FallbackInfo(DM_MERGE_SPLIT, "missing-model")
    (entry,) = decisions(result, "merge_or_split")
    assert_fallback_entry(entry, "merge_or_split", fallback)
    by_pointer = records(result)
    for index in range(len(branches)):
        record = by_pointer[f"/{combinator}/{index}"]
        assert_fallback_record(record, "merge_or_split", fallback)
        assert record.details["judgment"] == "split" and record.details["variant"] == index
        assert record.details["grouping"]["technique"] == LSH
        nested = record.details["variant_grouping"]
        assert (nested["method"], nested["technique"]) == ("priority", LSH)
        assert nested["fallback"] == FallbackInfo(MDL, "disabled").to_json()


def test_without_mdl_a_single_grouping_is_kept_split_without_the_mdl_indicator() -> None:
    # The shipped bundle leaves only the Bernoulli mixture in effect: priority among one candidate is no fallback.
    result = run(PETS, config(disabled=[MDL, HYBRID]))
    (grouping,) = decisions(result, "variant_grouping")
    assert grouping.resolution.method == "priority" and grouping.fallback is None
    assert [c.technique for c in grouping.resolution.candidates] == [BERNOULLI]
    (entry,) = decisions(result, "merge_or_split")
    assert_fallback_entry(entry, "merge_or_split", FallbackInfo(DM_MERGE_SPLIT, "missing-model"))
    (combinator,) = {"anyOf", "oneOf"} & set(result.document)
    assert len(result.document[combinator]) == 2


# ---------------------------------------------------------------------------
# 7.10: Tuple-or-List without its Decision_Model
# ---------------------------------------------------------------------------


def test_tuple_or_list_without_its_decision_model_is_a_tuple_only_by_a_lower_description_length() -> None:
    rows = run(ROWS, config())
    assert len(rows.document["prefixItems"]) == 3 and rows.document["items"] is False
    by_pointer = records(rows)
    for pointer in ("/prefixItems", "/items"):  # both Constraints carry the Tuple-or-List record
        record = by_pointer[pointer]
        assert record.technique == MDL and record.fallback == FallbackInfo(DM_TUPLE_LIST, "missing-model")
        tuple_score, list_score = (c.score for c in record.resolution.candidates)
        assert record.resolution.selected == 0 and tuple_score < list_score

    lists = run(LISTS, config())
    assert "prefixItems" not in lists.document and isinstance(lists.document["items"], dict)
    record = records(lists)["/items"]
    assert record.technique == MDL and record.fallback == FallbackInfo(DM_TUPLE_LIST, "missing-model")
    tuple_score, list_score = (c.score for c in record.resolution.candidates)
    assert record.resolution.selected == 1 and list_score < tuple_score


def test_tuple_or_list_without_mdl_is_a_list() -> None:
    result = run(ROWS, config(disabled=[MDL, HYBRID]))  # MDL would say Tuple
    assert "prefixItems" not in result.document and set(result.document["items"]["type"]) == {
        "boolean",
        "integer",
        "string",
    }
    fallback = FallbackInfo(DM_TUPLE_LIST, "missing-model")
    record = records(result)["/items"]
    assert_fallback_record(record, "tuple_or_list", fallback)
    assert record.details["judgment"] == "list"
    (entry,) = decisions(result, "tuple_or_list")
    assert_fallback_entry(entry, "tuple_or_list", fallback)


# ---------------------------------------------------------------------------
# 9.9: co-occurrence mining disabled
# ---------------------------------------------------------------------------


def required_lists(result: PipelineResult) -> dict[str, list[str]]:
    return {pointer: list(node.kw["required"]) for pointer, node in result.model.walk() if "required" in node.kw}


def test_disabled_cooccurrence_mining_emits_no_rules_and_keeps_required(bundles: dict[str, str]) -> None:
    mined = run(RULED, config(bundles["full"]))
    assert mined.document["dependentRequired"] == {"city": ["street"], "street": ["city"]}
    assert subschema(mined.document, "properties", "contact", "dependentRequired") == {"ext": ["phone"], "phone": ["ext"]}

    result = run(RULED, config(bundles["full"], disabled=[COOCCURRENCE]))
    # Field_Groups (6) may still emit dependentRequired cliques (9.9); no entry comes from co-occurrence mining.
    assert no_cooccurrence_entries(result)
    assert required_lists(result) == required_lists(mined) == {"": ["contact", "id"], "/properties/contact": ["name"]}
    by_pointer = records(result)
    assert {by_pointer[p].technique for p in by_pointer if p.startswith(("/required/", "/properties/contact/required/"))} == {
        f"{BASELINE_PREFIX}required"
    }
    # No rule is a Constraint, so each Variant's decision is in the report (20.7), with no candidate offered.
    entries = decisions(result, "cooccurrence_rule")
    assert sorted(e.path for e in entries) == ["$", "$.contact"]
    for entry in entries:
        assert_fallback_entry(entry, "cooccurrence_rule", FallbackInfo(COOCCURRENCE, "disabled"))
        assert len(entry.resolution.candidates) == 1
    assert [e for e in result.report.rejected if e.category == "rules"] == []
    assert COOCCURRENCE not in techniques_named(result)
    # A disabled Technique's thresholds are not read (20.6); only the absent Decision_Models are substituted.
    assert all(s.fallback.reason == "missing-model" or s.fallback.reason == "missing-threshold" for s in result.report.substitutions)  # the loose bundle also lacks tagged.branch_support


# ---------------------------------------------------------------------------
# 15.7: enum estimation disabled
# ---------------------------------------------------------------------------


def test_disabled_enum_estimation_judges_every_scalar_path_open() -> None:
    closed = run(COLORS, config())
    assert "enum" in subschema(closed.document, "properties", "color")
    assert "enum" in subschema(closed.document, "properties", "size")

    result = run(COLORS, config(disabled=[ENUM_ESTIMATION]))
    counts = keywords(result)
    assert counts["enum"] == 0 and counts["const"] == 0
    # 15.4: the open Paths keep their type, Format, and bound Constraints.
    size = subschema(result.document, "properties", "size")
    assert size["type"] == "integer" and {"minimum", "maximum"} <= set(size)
    assert subschema(result.document, "properties", "color", "format") == "hostname"
    # No Constraint comes from the open outcome: one fallback entry per scalar Path (20.7).
    entries = decisions(result, "closed_or_open_enum")
    assert sorted(e.path for e in entries) == ["$.color", "$.size", "$.tags[*]"]
    for entry in entries:
        # The Decision_Model is on but gated by the disabled enum estimation, which is named (15.7, 28.12).
        assert_fallback_entry(entry, "closed_or_open_enum", FallbackInfo(ENUM_ESTIMATION, "disabled"))
    # 15.8: no statistics were computed, so no rejected enum candidate is reported.
    assert [e for e in result.report.rejected if e.category == "enums"] == []
    assert ENUM_ESTIMATION not in techniques_named(result)


# ---------------------------------------------------------------------------
# 16.8: bound generalization disabled
# ---------------------------------------------------------------------------


def test_disabled_bound_generalization_emits_no_bound_keyword() -> None:
    bounded = run(BOUNDED, config())
    assert BOUND_KEYWORDS <= set(keywords(bounded))  # every keyword is emitted with the Technique on

    result = run(BOUNDED, config(disabled=[BOUNDS]))
    assert BOUND_KEYWORDS.isdisjoint(keywords(result))
    # Every bound the Technique would decide takes the fallback: a minimum and a maximum per Path.
    entries = decisions(result, "bound")
    assert Counter(e.path for e in entries) == Counter(e.path for e in decisions(bounded, "bound"))
    assert set(e.path for e in entries) == {"$.id", "$.label", "$.m", "$.xs"}
    for entry in entries:
        assert_fallback_entry(entry, "bound", FallbackInfo(BOUNDS, "disabled"))
    assert BOUNDS not in techniques_named(result)
    # A Map without key-count bounds is still a Map.
    assert set(subschema(result.document, "properties", "m")) == {"type", "additionalProperties", "propertyNames"}


# ---------------------------------------------------------------------------
# 20.6: a Learned_Threshold missing from, or invalid in, the Model_Bundle
# ---------------------------------------------------------------------------


def substitution_triples(result: PipelineResult) -> list[tuple[str, str, str, str, str]]:
    return [
        (s.artifact, s.fallback.technique, s.fallback.reason, s.cause, s.applied) for s in result.report.substitutions
    ]


def applied(kind: str) -> str:
    return f"Fallback_Behavior of {kind}: {REGISTRY.decision_point(kind).fallback_doc}"


def test_missing_cooccurrence_thresholds_apply_the_fallback_at_every_rule_decision(bundles: dict[str, str]) -> None:
    result = run(RULED, config(DEFAULT_MODEL_BUNDLE))  # the shipped bundle holds no Learned_Threshold
    assert no_cooccurrence_entries(result)
    entries = decisions(result, "cooccurrence_rule")
    # The full bundle emits rules at these Paths; without the thresholds each takes the fallback.
    mined = run(RULED, config(bundles["full"]))
    assert {e.path for e in decisions(mined, "cooccurrence_rule")} == {e.path for e in entries} == {"$", "$.contact"}
    for entry in entries:
        assert_fallback_entry(entry, "cooccurrence_rule", FallbackInfo(COOCCURRENCE, "missing-threshold"))
    # The report names each threshold, the Technique, the reason and cause, and the Fallback_Behavior applied.
    expected = [
        (name, COOCCURRENCE, "missing-threshold", "missing", applied("cooccurrence_rule")) for name in COOCCUR_THRESHOLDS
    ]
    assert [t for t in substitution_triples(result) if t[1] == COOCCURRENCE] == expected
    assert [s for s in result.report.to_json()["substitutions"] if s["artifact"] in COOCCUR_THRESHOLDS] == [
        Substitution(name, FallbackInfo(COOCCURRENCE, "missing-threshold"), "missing", applied("cooccurrence_rule")).to_json()
        for name in COOCCUR_THRESHOLDS
    ]
    assert not set(COOCCUR_THRESHOLDS) & set(result.report.thresholds)  # nothing was read (20.5)


def test_one_missing_cooccurrence_threshold_is_enough_for_the_fallback(bundles: dict[str, str]) -> None:
    # Design, Open Decisions ("Co-occurrence thresholds under MDL"): both are needed, even while MDL selects.
    result = run(RULED, config(bundles["support-only"]))
    assert no_cooccurrence_entries(result)
    entries = decisions(result, "cooccurrence_rule")
    assert {e.path for e in entries} == {"$", "$.contact"}
    for entry in entries:
        assert_fallback_entry(entry, "cooccurrence_rule", FallbackInfo(COOCCURRENCE, "missing-threshold"))
    assert [t for t in substitution_triples(result) if t[1] == COOCCURRENCE] == [
        ("cooccur.min_confidence", COOCCURRENCE, "missing-threshold", "missing", applied("cooccurrence_rule"))
    ]


def test_invalid_threshold_values_are_substituted_like_missing_ones(bundles: dict[str, str]) -> None:
    result = run(RULED, config(bundles["invalid"]))
    assert no_cooccurrence_entries(result)
    entries = decisions(result, "cooccurrence_rule")
    assert {e.path for e in entries} == {"$", "$.contact"}
    for entry in entries:
        assert_fallback_entry(entry, "cooccurrence_rule", FallbackInfo(COOCCURRENCE, "missing-threshold"))
    triples = substitution_triples(result)
    assert [t for t in triples if t[1] in (COOCCURRENCE, LSH)] == [
        ("cooccur.min_confidence", COOCCURRENCE, "missing-threshold", "invalid", applied("cooccurrence_rule")),
        ("cooccur.min_support", COOCCURRENCE, "missing-threshold", "invalid", applied("cooccurrence_rule")),
        ("lsh.keyset_jaccard", LSH, "missing-threshold", "invalid", f"{DBSCAN}, {HDBSCAN}, {BERNOULLI} at variant_grouping"),
    ]
    # The valid thresholds of the other grouping Techniques keep them in effect; LSH offers no grouping.
    assert not [t for t in triples if t[1] in (DBSCAN, HDBSCAN)]
    groupings = decisions(result, "variant_grouping")
    assert groupings
    for entry in groupings:
        assert [c.technique for c in entry.resolution.candidates] == [DBSCAN, HDBSCAN, BERNOULLI]
    assert "lsh.keyset_jaccard" not in result.report.thresholds
    assert {"dbscan.eps", "dbscan.min_samples", "hdbscan.min_cluster_size"} <= set(result.report.thresholds)


def test_missing_grouping_thresholds_leave_only_the_bernoulli_mixture() -> None:
    result = run(PETS, config(DEFAULT_MODEL_BUNDLE))
    (entry,) = decisions(result, "variant_grouping")
    assert entry.resolution.method == "mdl" and [c.technique for c in entry.resolution.candidates] == [BERNOULLI]
    assert techniques_named(result).isdisjoint({LSH, DBSCAN, HDBSCAN})
    by_technique = {t[0]: t for t in substitution_triples(result) if t[1] in (LSH, DBSCAN, HDBSCAN)}
    assert by_technique == {
        name: (name, spec.techniques[0], "missing-threshold", "missing", f"{BERNOULLI} at variant_grouping")
        for name, spec in THRESHOLDS.items()
        if spec.decision_point == "variant_grouping"
    }


def test_missing_grouping_thresholds_without_the_bernoulli_mixture_give_one_variant() -> None:
    result = run(NESTED, config(DEFAULT_MODEL_BUNDLE, disabled=[BERNOULLI]))
    obj = subschema(result.document, "properties", "obj")
    assert "anyOf" not in obj and set(obj["properties"]) == {"p", "q", "x", "y"}
    (entry,) = decisions(result, "variant_grouping")
    # The first serving Technique in priority order that cannot act is named, with its reason.
    assert entry.path == "$.obj"
    assert_fallback_entry(entry, "variant_grouping", FallbackInfo(LSH, "missing-threshold"))
    assert decisions(result, "merge_or_split") == []
    assert fallback_technique("variant_grouping") not in techniques_named(result)  # one Variant emits no Constraint


# ---------------------------------------------------------------------------
# 4.10, 5.11, 7.10, 15.7: the Decision_Model branches through the pipeline (task 22.4)
# ---------------------------------------------------------------------------

from jsinfer.learn.bundle import decision_model_artifact  # noqa: E402
from tests.models import FIXED_PROBABILITIES, fixed_models, write_model_bundle  # noqa: E402


@pytest.fixture(scope="module")
def model_bundle(tmp_path_factory: pytest.TempPathFactory) -> str:
    """A bundle holding a stub Decision_Model per Judgment_Call kind (``tests.models``) and the LOOSE thresholds."""
    root = tmp_path_factory.mktemp("t22-4-models")
    return write_model_bundle(root / "models", "t22.4-models", fixed_models(), {n: threshold_entry(n, v) for n, v in LOOSE.items()})


@pytest.fixture(scope="module")
def record_model_bundle(tmp_path_factory: pytest.TempPathFactory) -> str:
    """Like ``model_bundle``, but its Record-or-Map stub says Record, so Variants and Merge-or-Split are reached."""
    root = tmp_path_factory.mktemp("t22-4-record-models")
    models = fixed_models({**FIXED_PROBABILITIES, "record_or_map": (0.9, 0.1)})
    return write_model_bundle(root / "models", "t22.4-record-models", models, {n: threshold_entry(n, v) for n, v in LOOSE.items()})


def test_an_enabled_decision_model_selects_by_its_calibrated_probability(model_bundle: str) -> None:
    # MDL says Record for RECORDS; the stub Record-or-Map model says Map with probability 0.7 (22.12).
    result = run(RECORDS, config(model_bundle))
    assert "additionalProperties" in result.document and "properties" not in result.document
    record = records(result)["/additionalProperties"]
    assert record.technique == DM_RECORD_MAP and record.fallback is None
    assert record.resolution is not None and record.resolution.method == "decision-model"
    assert record.confidence == FIXED_PROBABILITIES["record_or_map"][1] == 0.7
    assert record.cutoff is not None and record.cutoff.source == "Decision_Model comparison"
    assert record.cutoff.artifact == decision_model_artifact("record_or_map", "synthetic")
    assert sorted((c.outcome, c.score) for c in record.cutoff.scores) == [("map", 0.7), ("record", 0.3)]
    assert record.details["decision_model"] == decision_model_artifact("record_or_map", "synthetic")
    assert not any(s.fallback.technique == DM_RECORD_MAP for s in result.report.substitutions)
    # MDL says List for LISTS; the stub Tuple-or-List model says Tuple with probability 0.6 (7.10).
    lists = run(LISTS, config(model_bundle))
    assert "prefixItems" in lists.document and lists.document["items"] is False
    prefix = records(lists)["/prefixItems"]
    assert prefix.technique == DM_TUPLE_LIST and prefix.confidence == 0.6
    assert prefix.cutoff is not None and prefix.cutoff.source == "Decision_Model comparison"


def test_equal_probabilities_are_resolved_by_the_mdl_scorer(record_model_bundle: str) -> None:
    # The stub Merge-or-Split model ties (0.5, 0.5): the MDL comparison over the tied outcomes decides (22.12, 20.7).
    result = run(NESTED, config(record_model_bundle))
    assert records(result)["/properties"].technique == DM_RECORD_MAP and records(result)["/properties"].confidence == 0.9
    (entry,) = decisions(result, "merge_or_split", "$.obj")
    assert entry.resolution.method == "decision-model" and entry.fallback is None
    assert entry.cutoff is not None and entry.cutoff.source == "MDL comparison" and entry.details["tie_break"] == "MDL comparison"
    assert entry.details["decision_model"] == decision_model_artifact("merge_or_split", "synthetic")


def test_a_missing_decision_model_falls_through_to_mdl_and_is_substituted() -> None:
    result = run(RECORDS, config(DEFAULT_MODEL_BUNDLE))  # the shipped bundle holds no model
    record = records(result)["/properties"]
    assert record.technique == MDL and record.resolution is not None and record.resolution.method == "mdl"
    assert record.fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")
    artifact = decision_model_artifact("record_or_map", "synthetic")
    assert (artifact, DM_RECORD_MAP, "missing-model", "missing", "decide.mdl at record_or_map") in substitution_triples(result)
    assert decisions(result, "record_or_map", "$")[0].fallback == FallbackInfo(DM_RECORD_MAP, "missing-model")


def test_the_closed_enum_model_serves_only_with_enum_estimation(record_model_bundle: str) -> None:
    # 15.7: with enum estimation off, every scalar Path is open, whatever the (present, enabled) model says.
    off = run(COLORS, config(record_model_bundle, disabled=[ENUM_ESTIMATION]))
    counts = keywords(off)
    assert counts["enum"] == 0 and counts["const"] == 0
    for entry in decisions(off, "closed_or_open_enum"):
        assert_fallback_entry(entry, "closed_or_open_enum", FallbackInfo(ENUM_ESTIMATION, "disabled"))
    assert not any(e.resolution.method == "decision-model" for e in decisions(off, "closed_or_open_enum"))
    # With both on, the model (closed with 0.55) judges the repeated values closed and is named as the producer.
    on = run(COLORS, config(record_model_bundle))
    color = records(on)["/properties/color/enum"]
    assert color.technique == DM_CLOSED_ENUM and color.confidence == 0.55
    assert color.cutoff is not None and color.cutoff.artifact == decision_model_artifact("closed_or_open_enum", "synthetic")
