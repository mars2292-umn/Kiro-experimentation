"""Property 41: Provenance completeness (task 17.4).

For any run, every Constraint in every emitted schema has exactly one
Provenance_Record. Each record names a registered Technique id, a fallback
marker, or a Baseline_Rule marker (``baseline:type`` on a ``type``
Constraint, ``baseline:required`` on a ``required`` entry, with no fallback
indicator and no resolution), consistent ≤ total, ratio = consistent/total,
and Confidence in [0, 1]. For Constraints inside ``$defs``, totals include
the Observations of every referencing Path. Every Judgment_Call outcome and
Good–Turing record is present (Requirements 4.1, 4.7, 7.1, 15.8, 26.1,
26.2, 26.13).

Runs
    Each example is a full ``pipeline.infer.infer_documents`` run with
    ``workers = 1``, a small ``soundness.max_passes`` (1 to 4), small
    Resource_Parameters, and one of four scenarios: Technique states drawn
    by ``run_configs()``, ``decide.mdl`` forced on, ``decide.mdl`` forced
    off, or every Technique off. The Model_Bundle is the empty default or
    one of three stub bundles of hand-set Learned_Thresholds (complete,
    tight, and partial, so that missing-threshold fallbacks occur), written
    once per module. Three stage hooks are wrapped (``unittest.mock``):

    - ``pipeline.infer._detect_outliers`` flags, in about three examples in
      five, one or two documents, one or two items (rows of any non-root
      column), or both, always leaving one document unflagged. No outlier
      Technique is implemented yet, so this stands in for the
      Outlier_Detector. The instance location of each flagged item is
      derived from the store's columns and checked against the raw document.
    - ``StructureAnalyzer.infer_model`` keeps a deep copy of the analyzer's
      model (the Judgment_Call checks read it), then injects over-tight
      Constraints and shared definitions (below).
    - ``pipeline.infer.check_soundness`` runs unchanged, then gives every
      surviving Constraint the injection left without a record a placeholder
      (``tests.records.complete_records``), as a component would.

Inputs
    Collections come from a random template: Records with optional fields,
    Maps with identifier or date keys (so ``propertyNames`` Formats occur),
    Lists, Tuples (some always full length, with positions of different
    types), mixed types, and Variants (Records with disjoint fields at one
    Path), nested up to two levels below the root. Scalars are free values,
    dates, or small categories whose values repeat (so Closed_Enums occur).
    About half the collections hold 15 to 30 documents, enough for Tuples
    and split Variants to win under MDL; the others 3 to 12. Up to one
    arbitrary value (``tests.strategies.json_values``) is added, and
    sometimes a NaN document, which the Ingestor skips. Under MDL a closed
    Record does not win on these inputs (the closed candidate's extra model
    bits exceed the extras flag it saves); the ``models`` bundle holds a
    stub Closed-or-Open-Record Decision_Model that closes, so the closed
    Closed-or-Open-Record clause runs in those examples (task 22.4).

Injection
    Up to three rows per run: a tighter bound, an ``enum`` or ``const``
    missing an observed value, a narrowed ``type``, an extra ``required``
    entry, ``additionalProperties: false`` with one observed field dropped,
    and a shared definition: a reached subschema moved into the root
    ``$defs`` and referenced from its site and, when an equal subschema
    sits elsewhere, from that site too, sometimes tightened inside the
    definition. Each injected Constraint gets an injected record
    (``details.injected``), loses its record, or keeps the inferred one.

Reference
    Support is recomputed from the raw parsed documents, independent of
    ``pipeline.soundness``: documents flagged as a whole are left out; the
    others are walked along the final schema (``properties``,
    ``patternProperties``, ``additionalProperties``, ``propertyNames`` over
    the keys, ``prefixItems``, ``items``, ``contains``, ``allOf``,
    ``anyOf``/``oneOf`` branches, ``if``/``then``/``else``, and local
    ``$ref``; ``not`` contents and ``$defs`` only through references),
    never entering a Flagged_Item (nor counting a flagged member's key).
    ``total`` counts the instances at a Constraint's node of the types its
    keyword applies to, and ``consistent`` those the keyword (with its
    context siblings) or the branch accepts. The Constraints of a
    ``propertyNames`` subschema and the ``propertyNames`` Constraint share
    one Support over the keys (design, Open Decisions).

    Verdicts are taken modulo Flagged_Items in the pipeline. The reference
    does not model that rule: where a verdict could depend on it (a branch,
    ``contains``, or ``if`` check, or a Constraint check, failing for an
    instance with a Flagged_Item below it, or a subschema-valued keyword
    failing in a document that holds Flagged_Items), the instance is
    undecided and the reference gives bounds instead of a count. Valid
    documents and every passing verdict are decided, so runs without
    Flagged_Items compare exactly.

Checks
    - 26.1: the Constraint pointers enumerated from the emitted document by
      the granularity table equal the pointer map's, distinct, and the
      records are keyed by exactly the pointer map's ConstraintIds.
    - 26.2, 26.10, 26.13 producers: a registered Technique id, a
      ``fallback:<registered kind>`` marker with fallback information, or a
      Baseline_Rule marker on its own keyword with neither fallback
      information nor resolution. Fallback reasons agree with the
      Run_Configuration and the bundle. Records the analyzer produced name
      implemented, enabled Techniques, and every ``type`` and ``required``
      Constraint the analyzer emitted carries its marker, in its model and,
      untouched by the injection, in the final one.
    - 26.2 Support and Confidence: consistent ≤ total, ratio =
      consistent/total (1.0 for 0/0), Confidence in [0, 1]; an MDL record's
      Confidence is the code-length posterior of its candidates, a
      Decision_Model record's the selected probability, a
      Co_Occurrence_Rule's under priority ``(n + 1) / (n + 2)`` of its
      Support count, and every other record's (no resolution, priority, or
      fallback) the Laplace value of its final Support.
    - 26.2 Support values: every Constraint's Support equals the reference
      (or lies within its bounds), Constraints inside ``$defs`` included,
      whose totals count every referencing site.
    - 4.1, 4.7, 7.1, 26.3 Judgment_Calls: in the analyzer's model, each
      Constraint a Judgment_Call produces (``properties`` and a Map's
      ``additionalProperties``: Record-or-Map; ``additionalProperties:
      false``: Closed-or-Open-Record; ``items`` and ``prefixItems``:
      Tuple-or-List; ``enum`` and ``const``: Closed-or-Open-Enum; ``anyOf``
      and ``oneOf`` branches: Merge-or-Split or Closed-or-Open-Enum) carries
      the decision's resolution, its outcome selected, and with MDL or a
      Decision_Model every alternative with its score; and the report's
      decision entries match those records one to one by (kind, Path,
      resolution), each outcome that produces a Constraint on a record and
      each that produces none (open Record, open enum, merged Variants)
      only in the report (20.7). Every Record Path resolves Closed-or-Open
      (4.7), and 4.11 and 7.9 are recorded on ``type``. In the final model,
      each such record survives on its node unless the soundness stage
      removed the Constraint or its node.
    - 15.8 Good–Turing: each Closed_Enum's record and each open judgment's
      rejected ``enums`` entry holds the six statistics, well formed, and
      the outcome; each scalar Path of the analyzer's model has one of them
      (or, with enum estimation off, a Fallback_Behavior decision); where a
      Path has one judgment and nothing undecided, the statistics and the
      enum values equal those recomputed from the raw documents.

**Validates: Requirements 4.1, 4.7, 7.1, 15.8, 26.1, 26.2, 26.13**
"""

# Feature: json-schema-inference, Property 41: Provenance completeness

from __future__ import annotations

import copy
import json
import math
from collections import Counter
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path as FilePath
from typing import Any
from unittest import mock
from urllib.parse import quote, unquote

import pytest
from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import ObservationStore, ShapePath
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, Step, key_step, render_path
from jsinfer.core.provenance import ProvenanceRecord, Resolution, Support
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode, split_pointer
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Compiled, Validator
from jsinfer.pipeline import infer as pipeline
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from tests.records import complete_records
from tests.models import fixed_models, write_model_bundle
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_values, run_configs

# One Validator and one Schema_Emitter for the module: compiled schemas are cached across examples.
VALIDATOR = Validator()
EMITTER = SchemaEmitter()
MDL = "decide.mdl"
MAX_PASSES = "soundness.max_passes"
ENUM_ESTIMATION = "values.enum_estimation"
COOCCURRENCE = "rules.cooccurrence"

# The stage functions the test wraps, captured before any patch.
ORIGINAL_INFER_MODEL = StructureAnalyzer.infer_model
ORIGINAL_DETECT_OUTLIERS = pipeline._detect_outliers
ORIGINAL_CHECK_SOUNDNESS = pipeline.check_soundness

Loc = tuple[Any, ...]  # an instance location: member names, positions, and ("name", key) for a key under propertyNames
Inst = tuple[int, Loc]  # (document index, instance location)

# ---------------------------------------------------------------------------
# Vocabulary from the requirements and the design
# ---------------------------------------------------------------------------

# The outcome labels of the five Judgment_Call kinds (22.4).
JUDGMENT_OUTCOMES: dict[str, tuple[str, str]] = {
    "record_or_map": ("record", "map"),
    "closed_or_open_record": ("closed", "open"),
    "tuple_or_list": ("tuple", "list"),
    "closed_or_open_enum": ("closed", "open"),
    "merge_or_split": ("merge", "split"),
}
# The outcomes that produce a Constraint; the others (open Record, open enum, merged Variants) produce none.
PRODUCING = frozenset(
    {
        ("record_or_map", "record"),
        ("record_or_map", "map"),
        ("closed_or_open_record", "closed"),
        ("tuple_or_list", "tuple"),
        ("tuple_or_list", "list"),
        ("closed_or_open_enum", "closed"),
        ("merge_or_split", "split"),
    }
)
BASELINE_RULES = ("type", "required")
FALLBACK_REASONS = ("disabled", "missing-threshold", "missing-model")
# Techniques whose Constraints carry a kind-specific Confidence (9.8, 11.2, 12.5, 13.5, 24.3, 24.4).
KIND_SPECIFIC = frozenset(
    {COOCCURRENCE, "rules.exclusion", "rules.keys", "rules.fd_ind", "values.semantic_type", "corpus.name_suggest",
     "corpus.cf_suggest"}
)
STATISTICS = ("chao1", "f1", "f2", "good_turing", "n", "s_obs")

# Keywords that are never Constraints (design, granularity table): $defs, $schema, $id, and annotations.
NOT_CONSTRAINTS = frozenset(
    {"$schema", "$defs", "$id", "$comment", "title", "description", "default", "examples", "deprecated",
     "readOnly", "writeOnly", "x-provenance"}
)
SINGLE_SUBSCHEMA = frozenset(
    {"additionalProperties", "contains", "else", "if", "items", "not", "propertyNames", "then"}
)
LIST_SUBSCHEMAS = frozenset({"allOf", "anyOf", "oneOf", "prefixItems"})
MAP_SUBSCHEMAS = frozenset({"$defs", "patternProperties", "properties"})
SUBSCHEMA_KEYWORDS = SINGLE_SUBSCHEMA | LIST_SUBSCHEMAS | MAP_SUBSCHEMAS

# ---------------------------------------------------------------------------
# Model_Bundles
# ---------------------------------------------------------------------------

LOOSE = {
    "lsh.keyset_jaccard": 0.5,
    "dbscan.eps": 0.34,
    "dbscan.min_samples": 1,
    "hdbscan.min_cluster_size": 2,
    "cooccur.min_support": 1,
    "cooccur.min_confidence": 0.5,
}
STUB_THRESHOLDS: dict[str, dict[str, int | float]] = {
    "stub-loose": LOOSE,
    "stub-tight": {
        "lsh.keyset_jaccard": 1.0,
        "dbscan.eps": 1e-9,
        "dbscan.min_samples": 2,
        "hdbscan.min_cluster_size": 3,
        "cooccur.min_support": 2,
        "cooccur.min_confidence": 0.0,
    },
    # DBSCAN and HDBSCAN miss their Learned_Thresholds, so their fallbacks name a missing threshold (20.6).
    "stub-partial": {
        name: LOOSE[name] for name in ("lsh.keyset_jaccard", "cooccur.min_support", "cooccur.min_confidence")
    },
}
BUNDLE_THRESHOLDS: dict[str, frozenset[str]] = {
    "default": frozenset(),
    "models": frozenset(),  # stub Decision_Models for every Judgment_Call kind, no Learned_Threshold
    **{name: frozenset(values) for name, values in STUB_THRESHOLDS.items()},
}


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> dict[str, str]:
    """Bundle identifiers by name: the shipped empty default and the stub bundles, written once."""
    root: FilePath = tmp_path_factory.mktemp("p41-bundles")
    found = {"default": DEFAULT_MODEL_BUNDLE}
    for name, values in STUB_THRESHOLDS.items():
        directory = root / name
        directory.mkdir()
        manifest = {"artifacts": {}, "bundle_id": f"p41-{name}", "format": 1}
        entries = {
            threshold: ThresholdRecord.from_spec(
                THRESHOLDS[threshold], value=value, objective_value=0.0, n_examples=1, training_data="p41"
            ).to_json()
            for threshold, value in values.items()
        }
        (directory / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (directory / "thresholds.json").write_text(json.dumps(entries), encoding="utf-8")
        found[name] = str(directory)
    found["models"] = write_model_bundle(root / "models", "p41-models", fixed_models())
    return found


# ---------------------------------------------------------------------------
# JSON helpers
# ---------------------------------------------------------------------------

TYPE_NAMES = ("null", "boolean", "integer", "number", "string", "array", "object")


def jtype(value: Any) -> str:
    """The JSON Schema type name of a value (integral numbers are integers; booleans are not numbers)."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "integer" if value.is_integer() else "number"
    if isinstance(value, str):
        return "string"
    return "array" if isinstance(value, list) else "object"


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def is_scalar(value: Any) -> bool:
    return not isinstance(value, (list, dict))


def canon(value: Any) -> str:
    """Canonical text under JSON Schema equality (``1.0`` is ``1``; ``true`` is not ``1``)."""

    def norm(v: Any) -> Any:
        if isinstance(v, float) and v.is_integer():
            return int(v)
        if isinstance(v, list):
            return [norm(x) for x in v]
        if isinstance(v, dict):
            return {k: norm(x) for k, x in v.items()}
        return v

    return json.dumps(norm(value), sort_keys=True, ensure_ascii=False)


def distinct(values: Iterable[Any]) -> list[Any]:
    return list({canon(v): v for v in values}.values())


def is_skipped(value: Any) -> bool:
    return isinstance(value, float) and math.isnan(value)


def escape(token: Any) -> str:
    return str(token).replace("~", "~0").replace("/", "~1")


def pointer_of(base: str, *tokens: Any) -> str:
    return base + "".join("/" + escape(token) for token in tokens)


def json_at(document: JSON, pointer: str) -> JSON:
    value: Any = document
    for token in split_pointer(pointer):
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def value_at(document: Any, loc: Loc) -> Any:
    for token in loc:
        document = document[token]
    return document


def resolution_text(resolution: Resolution | None) -> str | None:
    return None if resolution is None else json.dumps(resolution.to_json(), sort_keys=True)


def posterior(scores: Sequence[float], index: int) -> float:
    """The code-length posterior ``2^-DL(c) / Σ 2^-DL(c')`` of candidate ``index``."""
    low = min(scores)
    weights = [2.0 ** (low - score) for score in scores]
    return weights[index] / math.fsum(weights)


# ---------------------------------------------------------------------------
# Collections
# ---------------------------------------------------------------------------

STRINGS = ("", "a", "ab", "A1", "12", "-5", "2.5", "a-1", "2024-01-02T03:04:05Z", "x@y.co", "é😀")
SCALAR_VALUES = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(-3, 40),
    st.sampled_from([0.5, 2.5, -1.25, 1.0, 1e3, -0.0, 2**70]),
    st.sampled_from(STRINGS),
)
CATEGORY_POOL = ("red", "green", "blue", "A1", 0, 1, 1.0, True, None)
DATES = ("2024-01-02", "2023-12-31", "2024-02-29", "2024-03-10")
ID_KEYS = st.integers(0, 99).map(lambda i: f"u{i}")
DATE_KEYS = st.integers(1, 28).map(lambda d: f"2024-01-{d:02d}")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values: ``record`` fields ``(key, always, child)``; other kinds ``children``."""

    kind: str
    fields: tuple[tuple[str, bool, Template], ...] = ()
    children: tuple[Template, ...] = ()
    values: tuple[Any, ...] = ()  # a category's values
    keys: str = "id"  # a Map's key family
    fixed: bool = False  # a Tuple whose arrays always hold every position


@st.composite
def leaf_templates(draw: st.DrawFn) -> Template:
    kind = draw(st.sampled_from(["scalar", "scalar", "category", "category", "date"]), label="leaf kind")
    if kind == "category":
        pool = st.sampled_from(CATEGORY_POOL)
        return Template("category", values=tuple(draw(st.lists(pool, min_size=1, max_size=3, unique_by=canon))))
    return Template(kind)


@st.composite
def record_templates(draw: st.DrawFn, depth: int, min_fields: int = 0) -> Template:
    keys = draw(st.lists(st.sampled_from(KEYS), min_size=min_fields, max_size=4, unique=True), label="fields")
    fields = tuple((key, draw(st.booleans(), label=f"{key!r} always"), draw(templates(depth - 1))) for key in keys)
    return Template("record", fields=fields)


@st.composite
def templates(draw: st.DrawFn, depth: int) -> Template:
    kinds = ["leaf", "leaf", "leaf"] + (["record", "record", "map", "list", "tuple", "mix"] if depth > 0 else [])
    kind = draw(st.sampled_from(kinds), label="kind")
    if kind == "leaf":
        return draw(leaf_templates())
    if kind == "record":
        return draw(record_templates(depth))
    if kind == "map":
        return Template("map", children=(draw(templates(depth - 1)),), keys=draw(st.sampled_from(["id", "date"])))
    if kind == "list":
        return Template("list", children=(draw(templates(depth - 1)),))
    if kind == "tuple":
        return draw(tuple_templates(depth))
    return Template("mix", children=tuple(draw(st.lists(templates(depth - 1), min_size=2, max_size=3))))


@st.composite
def tuple_templates(draw: st.DrawFn, depth: int) -> Template:
    children = tuple(draw(st.lists(templates(depth - 1), min_size=1, max_size=3)))
    return Template("tuple", children=children, fixed=draw(st.booleans(), label="fixed length"))


@st.composite
def instances(draw: st.DrawFn, template: Template) -> Any:
    kind = template.kind
    if kind == "scalar":
        return draw(SCALAR_VALUES)
    if kind == "category":
        return draw(st.sampled_from(template.values))
    if kind == "date":
        return draw(st.sampled_from(DATES))
    if kind == "record":
        return {
            key: draw(instances(child))
            for key, always, child in template.fields
            if always or draw(st.booleans(), label=f"include {key!r}")
        }
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(ID_KEYS if template.keys == "id" else DATE_KEYS, max_size=4)))
        return {key: draw(instances(template.children[0])) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0]), max_size=3), label="elements")
    if kind == "tuple":
        size = len(template.children)
        length = size if template.fixed else draw(st.integers(0, size), label="tuple length")
        return [draw(instances(child)) for child in template.children[:length]]
    options = template.children
    return draw(instances(options[draw(st.integers(0, len(options) - 1), label="alternative")]))


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    shapes = ["record", "variants", "records", "record", "tuples", "mixed", "variants", "any"]
    shape = draw(st.sampled_from(shapes), label="root shape")
    if shape == "record":
        return draw(record_templates(2, min_fields=1))
    if shape == "records":
        return Template("list", children=(draw(record_templates(2, min_fields=1)),))
    if shape == "tuples":
        # Positions of different types, so that a Tuple can beat a List.
        first = Template("category", values=tuple(draw(st.lists(st.sampled_from([0, 1, 7, True]), min_size=1,
                                                                 max_size=2, unique_by=canon))))
        return Template("tuple", children=(first, draw(st.sampled_from([Template("date"), draw(leaf_templates())]))),
                        fixed=True)
    if shape == "variants":
        # Records with disjoint fields, so that splitting can beat merging.
        keys = draw(st.permutations(KEYS), label="variant fields")
        cut = draw(st.integers(1, 3), label="first variant's fields")
        variants = []
        for names in (keys[:cut], keys[cut : cut + draw(st.integers(1, 3), label="second variant's fields")]):
            fields = tuple((key, draw(st.booleans(), label=f"{key!r} always"), draw(templates(1))) for key in names)
            variants.append(Template("record", fields=fields))
        return Template("mix", children=tuple(variants))
    if shape == "mixed":
        parts = (draw(record_templates(1, min_fields=1)), draw(leaf_templates()), draw(templates(1)))
        return Template("mix", children=parts)
    return draw(templates(2))


@dataclass(frozen=True)
class Case:
    """The values ``infer_documents`` receives, NaN documents (skipped by the Ingestor) included."""

    values: tuple[Any, ...]

    @property
    def parsed(self) -> list[Any]:
        """Document ``i`` of the run: the ``i``-th value the Ingestor can write."""
        return [value for value in self.values if not is_skipped(value)]


@st.composite
def cases(draw: st.DrawFn) -> Case:
    root = draw(root_templates(), label="root template")
    # Larger collections let Tuples and split Variants beat their alternatives under MDL.
    size = st.integers(15, 30) if draw(st.booleans(), label="large collection") else st.integers(3, 12)
    docs = [draw(instances(root), label="document") for _ in range(draw(size, label="documents"))]
    docs += draw(st.lists(json_values(max_leaves=5), max_size=1), label="arbitrary documents")
    docs = [docs[i] for i in draw(st.permutations(range(len(docs))), label="order")]
    if draw(st.integers(0, 4), label="skipped documents") == 2:
        docs.insert(draw(st.integers(0, len(docs))), float("nan"))
    return Case(tuple(docs))


# ---------------------------------------------------------------------------
# Run_Configurations
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Setup:
    config: RunConfiguration
    bundle: str
    scenario: str

    def enabled(self, tid: str) -> bool:
        return bool(self.config.techniques[tid])


@st.composite
def setups(draw: st.DrawFn, bundles: Mapping[str, str]) -> Setup:
    bundle = draw(st.sampled_from(sorted(bundles)), label="bundle")
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    base: dict[str, Any] = {
        "workers": 1,
        MAX_PASSES: draw(st.sampled_from([3, 2, 4, 1]), label=MAX_PASSES),
        "model_bundle": bundles[bundle],
        "ingest.chunk_size": draw(st.sampled_from([1, 3, 2000]), label="ingest.chunk_size"),
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(st.sampled_from([2, 5_000]), label="sample.dbscan"),
        "sample.hdbscan": draw(st.sampled_from([2, 5_000]), label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(1, 4), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
    }
    scenario = draw(st.sampled_from(["drawn", "all off", "mdl off", "mdl on", "drawn"]), label="scenario")
    vary: tuple[str, ...] = (*TECHNIQUE_SETTINGS, "seed")
    if scenario == "all off":
        base.update({technique_setting(tid): False for tid in REGISTRY.ids()})
        vary = ("seed",)
    elif scenario != "drawn":
        base[technique_setting(MDL)] = scenario == "mdl on"
    config = draw(run_configs(vary=vary, base=base), label="config")
    return Setup(config, bundle, scenario)


# ---------------------------------------------------------------------------
# The reference walk
# ---------------------------------------------------------------------------

NUMBER_KEYWORDS = frozenset({"minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf"})
STRING_KEYWORDS = frozenset({"minLength", "maxLength", "pattern", "format", "x-latent-type"})
ARRAY_KEYWORDS = frozenset({"prefixItems", "items", "contains", "minItems", "maxItems", "uniqueItems"})
OBJECT_KEYWORDS = frozenset(
    {"properties", "patternProperties", "additionalProperties", "propertyNames", "required", "dependentRequired",
     "minProperties", "maxProperties"}
)
# The sibling keywords a keyword's verdict depends on.
CONTEXT: dict[str, tuple[str, ...]] = {
    "additionalProperties": ("properties", "patternProperties"),
    "items": ("prefixItems",),
    "contains": ("minContains", "maxContains"),
    "then": ("if",),
    "else": ("if",),
}


def applies(keyword: str, value: Any) -> bool:
    """Whether a Constraint keyword applies to an instance (26.2: Observations it can constrain)."""
    if keyword in NUMBER_KEYWORDS:
        return is_number(value)
    if keyword in STRING_KEYWORDS:
        return isinstance(value, str)
    if keyword in ARRAY_KEYWORDS:
        return isinstance(value, list)
    if keyword in OBJECT_KEYWORDS:
        return isinstance(value, dict)
    return True


def fragment(sub: dict[str, Any], cid: ConstraintId) -> JSON:
    """The schema that checks one Constraint alone: an ``anyOf``/``oneOf`` branch, or the keyword and its context."""
    keyword, item = cid.keyword, cid.item
    if keyword in ("anyOf", "oneOf") and isinstance(item, int):
        return sub[keyword][item]
    if keyword == "required":
        return {"required": [item]}
    if keyword == "dependentRequired":
        return {"dependentRequired": {item: sub[keyword][item]}}
    return {name: sub[name] for name in (keyword, *CONTEXT.get(keyword, ())) if name in sub}


@dataclass
class Found:
    """The instances that reach one node: decided, and those whose routing depended on Flagged_Items."""

    certain: dict[Inst, JSON] = field(default_factory=dict)
    maybe: dict[Inst, JSON] = field(default_factory=dict)


@dataclass(frozen=True)
class Bounds:
    """Reference Support bounds; ``exact`` when nothing was undecided."""

    total_low: int
    total_high: int
    consistent_low: int
    consistent_high: int

    @property
    def exact(self) -> bool:
        return self.total_low == self.total_high and self.consistent_low == self.consistent_high

    def holds(self, support: Support) -> bool:
        return (
            self.total_low <= support.total <= self.total_high
            and self.consistent_low <= support.consistent <= self.consistent_high
        )


class Reference:
    """The instances each node of a model is evaluated on (see Reference in the module docstring)."""

    def __init__(self, model: SchemaModel, masked: Mapping[int, frozenset[Loc]] | None = None) -> None:
        self.model = model
        self.doc, _ = EMITTER.layout(model)
        self.pointers = {node.nid: pointer for pointer, node in model.walk()}
        self.nodes = model.index()
        defs = self.doc.get("$defs") if isinstance(self.doc, dict) else None
        self.defs: JSON | None = defs
        self.masked = dict(masked or {})
        self._checks: dict[int, Compiled] = {}
        self.names_owner: dict[int, int] = {}  # propertyNames subschema nid -> nid of the node holding it
        for _, node in model.walk():
            names = None if node.boolean is not None else node.kw.get("propertyNames")
            if isinstance(names, SchemaNode):
                self.names_owner[names.nid] = node.nid
        self.names_of = {owner: names for names, owner in self.names_owner.items()}

    # -- verdicts --------------------------------------------------------------

    def standalone(self, part: JSON) -> JSON:
        """``part`` as a schema of its own, with the root ``$defs`` copied in for its references."""
        if isinstance(part, bool) or self.defs is None:
            return part
        return {**part, "$defs": self.defs}  # type: ignore[dict-item]

    def subschema(self, node: SchemaNode) -> JSON:
        return json_at(self.doc, self.pointers[node.nid])

    def accepts(self, node: SchemaNode, value: JSON) -> bool:
        if node.boolean is not None:
            return node.boolean
        compiled = self._checks.get(node.nid)
        if compiled is None:
            schema = self.standalone(self.subschema(node))
            compiled = self._checks[node.nid] = VALIDATOR.compile(schema)  # type: ignore[arg-type]
        return VALIDATOR.is_valid(compiled, value)

    def below(self, doc: int, loc: Loc) -> bool:
        """Whether a Flagged_Item of ``doc`` lies strictly below ``loc``."""
        depth = len(loc)
        return any(len(spot) > depth and spot[:depth] == loc for spot in self.masked.get(doc, ()))

    def verdict(self, node: SchemaNode, value: JSON, doc: int, loc: Loc) -> bool | None:
        """The verdict modulo Flagged_Items where it is decided without the rule; None when it is not."""
        if self.accepts(node, value):
            return True  # a passing verdict passes modulo any Flagged_Items
        return None if self.below(doc, loc) else False

    # -- the walk ----------------------------------------------------------------

    def target(self, node: SchemaNode) -> SchemaNode | None:
        """The node a local ``$ref`` names."""
        if node.boolean is not None or "$ref" not in node.kw:
            return None
        ref = node.kw["$ref"]
        assert isinstance(ref, str) and ref.startswith("#"), f"a non-local $ref {ref!r}"
        return self.model.resolve(unquote(ref[1:]))

    def walk(self, documents: Iterable[tuple[int, JSON]]) -> dict[int, Found]:
        """Per nid, the instances that reach it, keyed by (document, instance location)."""
        found: dict[int, Found] = {}
        for doc, value in documents:
            masked = self.masked.get(doc, frozenset())
            assert () not in masked, "a flagged root row is a flagged document"
            seen: set[tuple[int, Loc, bool]] = set()
            stack: list[tuple[SchemaNode, Any, Loc, bool]] = [(self.model.root, value, (), False)]
            while stack:
                node, instance, loc, maybe = stack.pop()
                if (node.nid, loc, False) in seen or (node.nid, loc, maybe) in seen:
                    continue
                seen.add((node.nid, loc, maybe))
                entry = found.setdefault(node.nid, Found())
                if maybe:
                    entry.maybe[(doc, loc)] = instance
                else:
                    entry.certain[(doc, loc)] = instance
                    entry.maybe.pop((doc, loc), None)
                stack.extend(self.successors(node, instance, loc, maybe, doc, masked))
        return found

    def successors(
        self, node: SchemaNode, instance: Any, loc: Loc, maybe: bool, doc: int, masked: frozenset[Loc]
    ) -> list[tuple[SchemaNode, Any, Loc, bool]]:
        if node.boolean is not None:
            return []
        kw = node.kw
        out: list[tuple[SchemaNode, Any, Loc, bool]] = []
        if isinstance(instance, dict):
            properties = kw.get("properties", {})
            patterns = kw.get("patternProperties", {})
            additional = kw.get("additionalProperties")
            names = kw.get("propertyNames")
            for key, member in instance.items():
                spot = (*loc, key)
                if spot in masked:
                    continue  # a Flagged_Item, and its key with it
                matched = [p for p in patterns if VALIDATOR.matches_pattern(p, key)]
                if key in properties:
                    out.append((properties[key], member, spot, maybe))
                out.extend((patterns[p], member, spot, maybe) for p in matched)
                if key not in properties and not matched and isinstance(additional, SchemaNode):
                    out.append((additional, member, spot, maybe))
                if isinstance(names, SchemaNode):
                    out.append((names, key, (*loc, ("name", key)), maybe))
        if isinstance(instance, list):
            prefix = kw.get("prefixItems", [])
            items = kw.get("items")
            contains = kw.get("contains")
            for index, element in enumerate(instance):
                spot = (*loc, index)
                if spot in masked:
                    continue
                if index < len(prefix):
                    out.append((prefix[index], element, spot, maybe))
                elif isinstance(items, SchemaNode):
                    out.append((items, element, spot, maybe))
                if isinstance(contains, SchemaNode):
                    verdict = self.verdict(contains, element, doc, spot)
                    if verdict is not False:
                        out.append((contains, element, spot, maybe or verdict is None))
        for child in kw.get("allOf", []):
            out.append((child, instance, loc, maybe))
        for keyword in ("anyOf", "oneOf"):
            for child in kw.get(keyword, []):
                verdict = self.verdict(child, instance, doc, loc)
                if verdict is not False:
                    out.append((child, instance, loc, maybe or verdict is None))
        condition = kw.get("if")
        if isinstance(condition, SchemaNode):
            out.append((condition, instance, loc, maybe))
            verdict = self.verdict(condition, instance, doc, loc)
            for keyword, taken in (("then", True), ("else", False)):
                branch = kw.get(keyword)
                if isinstance(branch, SchemaNode) and verdict in (taken, None):
                    out.append((branch, instance, loc, maybe or verdict is None))
        reference = self.target(node)
        if reference is not None:
            out.append((reference, instance, loc, maybe))
        return out

    # -- Support -----------------------------------------------------------------

    def bounds(self, cid: ConstraintId, found: Mapping[int, Found]) -> Bounds:
        """The reference Support of ``cid`` (see Reference in the module docstring)."""
        if cid.nid in self.names_owner or (cid.keyword == "propertyNames" and cid.nid in self.names_of):
            # The Constraints of a propertyNames subschema share one Support over the keys.
            nid = cid.nid if cid.nid in self.names_owner else self.names_of[cid.nid]
            entry = found.get(nid, Found())
            names = self.nodes[nid]
            accepted = sum(1 for key in entry.certain.values() if self.accepts(names, key))
            undecided = len(entry.maybe)
            return Bounds(len(entry.certain), len(entry.certain) + undecided, accepted, accepted + undecided)
        node = self.nodes[cid.nid]
        sub = self.subschema(node)
        assert isinstance(sub, dict), f"{cid} names a keyword of a boolean schema"
        branch = cid.keyword in ("anyOf", "oneOf")
        check = VALIDATOR.compile(self.standalone(fragment(sub, cid)))  # type: ignore[arg-type]
        self_contained = cid.keyword not in SUBSCHEMA_KEYWORDS and cid.keyword != "$ref"
        entry = found.get(cid.nid, Found())
        total = consistent = undecided = 0
        for (doc, loc), value in entry.certain.items():
            if not applies(cid.keyword, value):
                continue
            total += 1
            if VALIDATOR.is_valid(check, value):
                consistent += 1
            elif doc in self.masked and (self.below(doc, loc) or not (branch or self_contained)):
                undecided += 1
        maybe = sum(1 for value in entry.maybe.values() if applies(cid.keyword, value))
        return Bounds(total, total + maybe, consistent, consistent + undecided + maybe)


def logical_paths(model: SchemaModel) -> dict[int, tuple[Step, ...]]:
    """The logical Path of each node reached from the root without ``$defs``, ``not``, or ``propertyNames``."""
    paths: dict[int, tuple[Step, ...]] = {model.root.nid: ()}
    stack = [model.root]
    while stack:
        node = stack.pop()
        if node.boolean is not None:
            continue
        base = paths[node.nid]
        children: list[tuple[SchemaNode, tuple[Step, ...]]] = []
        for keyword, value in node.kw.items():
            if keyword == "properties":
                children.extend((child, (*base, key_step(name))) for name, child in value.items())
            elif keyword == "patternProperties":
                children.extend((child, (*base, MAPVAL_STEP)) for child in value.values())
            elif keyword == "additionalProperties":
                children.append((value, (*base, MAPVAL_STEP)))
            elif keyword in ("items", "contains"):
                children.append((value, (*base, ELEM_STEP)))
            elif keyword == "prefixItems":
                children.extend((child, (*base, ELEM_STEP)) for child in value)
            elif keyword in ("anyOf", "oneOf", "allOf"):
                children.extend((child, base) for child in value)
            elif keyword in ("if", "then", "else"):
                children.append((value, base))
        for child, path in children:
            paths[child.nid] = path
            stack.append(child)
    return paths


# ---------------------------------------------------------------------------
# Injection of over-tight Constraints and shared definitions
# ---------------------------------------------------------------------------

DEF_NAMES = ("d0", "a b", "é~/")
REF_SAFE = "/"


@dataclass
class Injector:
    """Draws over-tight Constraints and shared definitions into a model (see Injection in the module docstring)."""

    data: st.DataObject
    model: SchemaModel
    values: dict[int, list[JSON]]  # per nid, the validated documents' instances in the inferred model
    injected: set[ConstraintId] = field(default_factory=set)
    rows: list[str] = field(default_factory=list)
    sites: list[int] = field(default_factory=list)  # referencing sites of each injected definition

    def draw(self, strategy: st.SearchStrategy[Any], label: str) -> Any:
        return self.data.draw(strategy, label=label)

    def index(self, items: Sequence[Any], label: str) -> Any:
        return items[self.draw(st.integers(0, len(items) - 1), label)]

    def nodes(self) -> list[SchemaNode]:
        """Keyword nodes that the validated documents reach, in canonical pre-order."""
        return [node for _, node in self.model.walk() if node.boolean is None and self.values.get(node.nid)]

    def of(self, node: SchemaNode) -> list[JSON]:
        return self.values.get(node.nid, [])

    def slots(self) -> dict[int, tuple[SchemaNode, str, Any]]:
        found: dict[int, tuple[SchemaNode, str, Any]] = {}
        for _, node in self.model.walk():
            for keyword, entry, child in node.children():
                found.setdefault(child.nid, (node, keyword, entry))
        return found

    def replace(self, node: SchemaNode, new: SchemaNode) -> None:
        parent, keyword, entry = self.slots()[node.nid]
        if entry is None:
            parent.kw[keyword] = new
        else:
            parent.kw[keyword][entry] = new

    def forget(self, node: SchemaNode) -> None:
        """Drop the Provenance_Records of a subtree that leaves the model."""
        gone = {n.nid for _, n in node.walk()}
        for cid in [cid for cid in self.model.provenance if cid.nid in gone]:
            del self.model.provenance[cid]

    def record(self, cid: ConstraintId, row: str, n: int) -> None:
        """Give the injected Constraint an injected record, take its record away, or keep the inferred one."""
        self.injected.add(cid)
        choice = self.draw(st.sampled_from(["record", "none", "keep"]), f"record of {cid.keyword}")
        if choice == "record":
            self.model.provenance[cid] = ProvenanceRecord.laplace(MDL, Support(n, n), details={"injected": row})
        elif choice == "none":
            self.model.provenance.pop(cid, None)

    def set(self, node: SchemaNode, keyword: str, value: Any, row: str, item: Any = None) -> None:
        node.kw[keyword] = value
        self.record(ConstraintId(node.nid, keyword, item), row, len(self.of(node)))


def inject_bound(inj: Injector, nodes: list[SchemaNode]) -> bool:
    options: list[tuple[SchemaNode, str, list[Any]]] = []
    for node in nodes:
        values = inj.of(node)
        for kind, quantities in (
            ("number", [v for v in values if is_number(v)]),
            ("Length", [len(v) for v in values if isinstance(v, str)]),
            ("Items", [len(v) for v in values if isinstance(v, list)]),
            ("Properties", [len(v) for v in values if isinstance(v, dict)]),
        ):
            if quantities:
                options.append((node, kind, quantities))
    if not options:
        return False
    node, kind, quantities = inj.index(options, "bound site")
    low, high = min(quantities), max(quantities)
    side = inj.draw(st.sampled_from(["min", "max"]), "bound side")
    if kind == "number":
        keyword = "maximum" if side == "max" else "minimum"
        value = (low if low < high else low - 1) if side == "max" else (high if low < high else high + 1)
    else:
        if side == "max" and high == 0:
            side = "min"
        keyword = side + kind
        value = inj.draw(st.integers(0, high - 1) if side == "max" else st.integers(low + 1, high + 1), "bound")
    inj.set(node, keyword, value, "bound")
    return True


def inject_type(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if "type" in node.kw]
    if not sites:
        return False
    node = inj.index(sites, "type site")
    current = node.kw["type"]
    listed = [current] if isinstance(current, str) else list(current)
    if len(listed) >= 2:
        drop = inj.draw(st.integers(0, len(listed) - 1), "dropped type")
        narrowed = [name for i, name in enumerate(listed) if i != drop]
    else:
        observed = {jtype(v) for v in inj.of(node)}
        others = [name for name in TYPE_NAMES if name not in observed and name not in listed]
        narrowed = [inj.draw(st.sampled_from(others or ["null"]), "narrowed type")]
    inj.set(node, "type", narrowed, "type")
    return True


def inject_enum(inj: Injector, nodes: list[SchemaNode], keyword: str) -> bool:
    sites = [node for node in nodes if all(is_scalar(v) for v in inj.of(node))]
    if not sites:
        return False
    node = inj.index(sites, f"{keyword} site")
    members = distinct(inj.of(node))
    if len(members) >= 2:
        if keyword == "const":
            value: Any = copy.deepcopy(inj.index(members, "const member"))
        else:
            drop = inj.draw(st.integers(0, len(members) - 1), "missing member")
            value = [copy.deepcopy(m) for i, m in enumerate(members) if i != drop]
    else:
        missing = [m for m in ("zz", 99, False, None) if canon(m) != canon(members[0])]
        chosen = inj.draw(st.sampled_from(missing), "missing value")
        value = chosen if keyword == "const" else [chosen]
    for other in ("enum", "const"):  # one of the two at a node, as the analyzer emits them
        if other != keyword and other in node.kw:
            del node.kw[other]
            inj.model.provenance.pop(ConstraintId(node.nid, other), None)
            inj.injected.add(ConstraintId(node.nid, other))
    inj.set(node, keyword, value, keyword)
    return True


def objects_of(inj: Injector, node: SchemaNode) -> list[dict[str, Any]]:
    return [v for v in inj.of(node) if isinstance(v, dict)]


def inject_required(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if objects_of(inj, node)]
    if not sites:
        return False
    node = inj.index(sites, "required site")
    objects = objects_of(inj, node)
    keys = list(dict.fromkeys(key for obj in objects for key in obj))
    optional = [key for key in keys if any(key not in obj for obj in objects)]
    name = inj.draw(st.sampled_from([*optional, "zz"]), "required name")
    listed = list(node.kw.get("required", []))
    if name not in listed:
        listed.append(name)
    inj.set(node, "required", listed, "required", name)
    return True


def inject_closed(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [
        node
        for node in nodes
        if objects_of(inj, node)
        and isinstance(node.kw.get("properties"), dict)
        and "additionalProperties" not in node.kw
    ]
    if not sites:
        return False
    node = inj.index(sites, "closed site")
    objects = objects_of(inj, node)
    properties = node.kw["properties"]
    present = [key for key in properties if any(key in obj for obj in objects)]
    if present:
        key = inj.draw(st.sampled_from(present), "dropped field")
        inj.forget(properties[key])
        del properties[key]
        inj.injected.add(ConstraintId(node.nid, "properties"))
        if not properties:
            del node.kw["properties"]
            inj.model.provenance.pop(ConstraintId(node.nid, "properties"), None)
    inj.set(node, "additionalProperties", SchemaNode(boolean=False), "closed")
    return True


VALUE_ROWS: dict[str, Callable[[Injector, list[SchemaNode]], bool]] = {
    "bound": inject_bound,
    "type": inject_type,
    "enum": lambda inj, nodes: inject_enum(inj, nodes, "enum"),
    "const": lambda inj, nodes: inject_enum(inj, nodes, "const"),
}


def inject_defs(inj: Injector, nodes: list[SchemaNode]) -> bool:
    """Move a reached subschema into the root ``$defs``, referenced from its site and maybe from an equal twin's."""
    root = inj.model.root
    if root.boolean is not None:
        return False
    slots = inj.slots()
    in_defs = {n.nid for p, n in inj.model.walk() if p.startswith("/$defs/")}
    names_nodes = {
        node.kw["propertyNames"].nid for _, node in inj.model.walk()
        if node.boolean is None and isinstance(node.kw.get("propertyNames"), SchemaNode)
    }
    movable = [
        node for node in nodes
        if node is not root and node.nid in slots and node.nid not in in_defs and node.nid not in names_nodes
    ]
    defs = root.kw.get("$defs", {})
    names = [name for name in DEF_NAMES if name not in defs]
    if not movable or not names:
        return False
    moved = inj.index(movable, "defined subschema")
    inside = {n.nid for _, n in moved.walk()}
    twins = [
        node
        for node in movable
        if node is not moved and node.nid not in inside and moved.nid not in {n.nid for _, n in node.walk()}
        and node == moved
    ]
    # A second site: an equal subschema when there is one, else any other reached subschema that does not
    # overlap the first (the soundness loop then widens the definition for both sites' instances).
    others = [
        node
        for node in movable
        if node is not moved and node.nid not in inside and moved.nid not in {n.nid for _, n in node.walk()}
    ]
    sites = [moved]
    second = inj.draw(st.sampled_from(["twin", "other", "none"]), "a second referencing site")
    if second != "none" and (twins if second == "twin" else others):
        sites.append(inj.index(twins if second == "twin" else others, "second site"))
    name = inj.draw(st.sampled_from(names), "definition name")
    ref = "#" + quote(pointer_of("", "$defs", name), safe=REF_SAFE)
    values = [v for site in sites for v in inj.of(site)]
    for site in sites:
        reference = SchemaNode.from_json({"$ref": ref})
        if site is not moved:
            inj.forget(site)
        inj.replace(site, reference)
        inj.record(ConstraintId(reference.nid, "$ref"), "$defs", len(inj.of(site)))
    root.kw.setdefault("$defs", {})[name] = moved
    inj.values[moved.nid] = values
    inj.sites.append(len(sites))
    if inj.draw(st.booleans(), "tightened inside the definition"):
        row = inj.draw(st.sampled_from(sorted(VALUE_ROWS)), "definition row")
        if VALUE_ROWS[row](inj, [moved]):
            event(f"injected {row} inside $defs")
    return True


ROWS: dict[str, Callable[[Injector, list[SchemaNode]], bool]] = {
    **VALUE_ROWS,
    "required": inject_required,
    "closed": inject_closed,
    "$defs": inject_defs,
}
ROW_NAMES = tuple(ROWS)


def inject(data: st.DataObject, model: SchemaModel, values: dict[int, list[JSON]]) -> Injector:
    """Zero to three rows; a row without a site is redrawn once. ``$defs`` is drawn first in about half the runs."""
    inj = Injector(data, model, values)
    wanted = data.draw(st.integers(0, 3), label="rows")
    if wanted and data.draw(st.booleans(), label="a shared definition first"):
        if inject_defs(inj, inj.nodes()):
            inj.rows.append("$defs")
        wanted -= 1
    offset = (len(model.provenance) + 3 * len(values)) % len(ROW_NAMES)  # spread the draws over the rows
    for _ in range(wanted):
        for _attempt in range(2):
            row = ROW_NAMES[(offset + data.draw(st.integers(0, len(ROW_NAMES) - 1), label="row")) % len(ROW_NAMES)]
            note(f"injecting {row}")
            if ROWS[row](inj, inj.nodes()):
                inj.rows.append(row)
                break
    return inj


# ---------------------------------------------------------------------------
# Stage hooks
# ---------------------------------------------------------------------------


@dataclass
class RunState:
    """What the hooks saw: the flags, the analyzer's model, and the injection."""

    case: Case
    flagged_docs: tuple[int, ...] = ()  # documents flagged as a whole
    flagged_items: dict[int, frozenset[Loc]] = field(default_factory=dict)  # per document, its Flagged_Items
    analyzer: SchemaModel | None = None  # the Structure_Analyzer's model, before the injection
    injection: Injector | None = None

    @property
    def flagged(self) -> set[int]:
        """The Flagged_Documents: flagged documents and every document holding a Flagged_Item."""
        return {*self.flagged_docs, *self.flagged_items}

    @property
    def injected(self) -> set[ConstraintId]:
        return set() if self.injection is None else self.injection.injected


def location_of(store: ObservationStore, shape: ShapePath, row: int) -> tuple[int, Loc]:
    """The document and instance location of a store row, following ``parent`` links up to the root column."""
    tokens: list[Any] = []
    while shape:
        cells = store.columns[shape]
        if shape[-1] == MAPVAL_STEP:
            tokens.append(str(store.keys[int(cells.key[row])]))
        else:
            tokens.append(int(cells.pos[row]))
        row = int(cells.parent[row])
        shape = shape[:-1]
    return int(store.columns[()].doc[row]), tuple(reversed(tokens))


def flagger(data: st.DataObject, state: RunState) -> Callable[[Any], None]:
    def detect_outliers(ctx: Any) -> None:
        ORIGINAL_DETECT_OUTLIERS(ctx)
        store: ObservationStore = ctx.store
        n = store.n_docs
        kind = data.draw(st.sampled_from(["none", "none", "documents", "items", "both"]), label="flags")
        if n < 2 or kind == "none":
            return
        keeper = data.draw(st.integers(0, n - 1), label="unflagged document")
        others = [doc for doc in range(n) if doc != keeper]
        docs: list[int] = []
        if kind != "items":
            docs = data.draw(st.lists(st.sampled_from(others), min_size=1, max_size=2, unique=True), label="flagged")
        shapes = [shape for shape in store.columns if shape]
        items: dict[int, set[Loc]] = {}
        for _ in range(data.draw(st.integers(1, 2), label="flagged items") if shapes and kind != "documents" else 0):
            shape = shapes[data.draw(st.integers(0, len(shapes) - 1), label="item column")]
            rows = [row for row in range(len(store.columns[shape])) if location_of(store, shape, row)[0] != keeper]
            if not rows:
                continue
            row = rows[data.draw(st.integers(0, len(rows) - 1), label="item row")]
            doc, loc = location_of(store, shape, row)
            value_at(state.case.parsed[doc], loc)  # the location names an instance of the raw document
            store.flags.flag_items(shape, [row])
            items.setdefault(doc, set()).add(loc)
        if docs:
            store.flags.flag_documents(sorted(docs))
        state.flagged_docs = tuple(sorted(docs))
        state.flagged_items = {doc: frozenset(locs) for doc, locs in sorted(items.items()) if doc not in docs}

    return detect_outliers


def injector(data: st.DataObject, state: RunState) -> Callable[[StructureAnalyzer, Any], SchemaModel]:
    def infer_model(self: StructureAnalyzer, view: Any) -> SchemaModel:
        model = ORIGINAL_INFER_MODEL(self, view)
        state.analyzer = copy.deepcopy(model)
        parsed = state.case.parsed
        flagged = state.flagged
        live = [(doc, parsed[doc]) for doc in range(len(parsed)) if doc not in flagged]
        found = Reference(model).walk(live)
        values = {nid: list(entry.certain.values()) for nid, entry in found.items()}
        state.injection = inject(data, model, values)
        return model

    return infer_model


def spy(state: RunState) -> Callable[..., Any]:
    def check_soundness(model: SchemaModel, documents: Any, **kwargs: Any) -> Any:
        result = ORIGINAL_CHECK_SOUNDNESS(model, documents, **kwargs)
        # The injection leaves some Constraints without records on purpose (3.3); a component would give
        # every surviving Constraint one, so give those the soundness loop left alone a placeholder.
        if complete_records(model):
            event("placeholder records added")
        return result

    return check_soundness


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def constraint_pointers(schema: Any, base: str = "") -> list[str]:
    """The JSON Pointer of every Constraint in an emitted document, by the design's granularity table."""
    out: list[str] = []
    if not isinstance(schema, dict):
        return out
    for keyword, value in schema.items():
        here = pointer_of(base, keyword)
        if keyword not in NOT_CONSTRAINTS:
            if keyword in ("required", "anyOf", "oneOf"):
                out.extend(pointer_of(here, index) for index in range(len(value)))
            elif keyword == "dependentRequired":
                out.extend(pointer_of(here, key) for key in value)
            else:
                out.append(here)
        if keyword in SINGLE_SUBSCHEMA:
            out.extend(constraint_pointers(value, here))
        elif keyword in LIST_SUBSCHEMAS:
            for index, child in enumerate(value):
                out.extend(constraint_pointers(child, pointer_of(here, index)))
        elif keyword in MAP_SUBSCHEMAS:
            for name, child in value.items():
                out.extend(constraint_pointers(child, pointer_of(here, name)))
    return out


def check_one_to_one(result: PipelineResult) -> None:
    """26.1: one record per emitted Constraint, and no record of anything else."""
    expected = constraint_pointers(result.document)
    assert len(expected) == len(set(expected)), "the emitted document repeats a Constraint pointer"
    pointers = result.pointers
    assert sorted(pointers.values()) == sorted(expected), "the pointer map differs from the emitted Constraints"
    provenance = result.model.provenance
    missing = [pointers[cid] for cid in pointers if cid not in provenance]
    orphaned = [cid for cid in provenance if cid not in pointers]
    assert not missing, f"emitted Constraints without a Provenance_Record: {missing}"
    assert not orphaned, f"Provenance_Records of no emitted Constraint: {orphaned}"
    document, again = EMITTER.layout(result.model)
    assert again == pointers and EMITTER.dumps(document) == result.schema, "the result's layout is not the model's"
    for pointer in expected:
        json_at(result.document, pointer)  # every pointer resolves in the emitted document


def check_fallback(info: Any, setup: Setup, where: str) -> None:
    """26.10, 28.12: the Technique whose Fallback_Behavior applied and a reason that matches the run."""
    if info is None:
        return
    assert info.technique in REGISTRY.ids(), f"{where}: fallback names an unregistered Technique {info.technique!r}"
    assert info.reason in FALLBACK_REASONS, f"{where}: fallback reason {info.reason!r}"
    spec = REGISTRY.technique(info.technique)
    if info.reason == "disabled":
        assert not (spec.implemented and setup.enabled(info.technique)), f"{where}: {info.technique} is not disabled"
    elif info.reason == "missing-threshold":
        assert setup.enabled(info.technique), f"{where}: a missing threshold of the disabled {info.technique}"
        held = BUNDLE_THRESHOLDS[setup.bundle]
        assert any(name not in held for name in spec.thresholds), f"{where}: {info.technique} misses no threshold"
    event(f"fallback reason {info.reason}")


def check_producers(result: PipelineResult, setup: Setup, state: RunState) -> None:
    """26.2, 26.10, 26.13: what each record names as its producer."""
    ids = set(REGISTRY.ids())
    kinds = set(REGISTRY.decision_points)
    for cid, record in result.model.provenance.items():
        where = result.pointers[cid]
        technique = record.technique
        if technique.startswith("baseline:"):
            rule = technique[len("baseline:") :]
            assert rule in BASELINE_RULES, f"{where}: unknown Baseline_Rule marker {technique!r}"
            assert cid.keyword == rule, f"{where}: {technique} on a {cid.keyword!r} Constraint"
            assert record.fallback is None and record.resolution is None, f"{where}: {technique} resolved something"
            event(f"producer {technique}")
        elif technique.startswith("fallback:"):
            assert technique[len("fallback:") :] in kinds, f"{where}: fallback marker of an unknown kind {technique!r}"
            assert record.fallback is not None, f"{where}: {technique} without its fallback indicator"
            event("producer fallback marker")
        else:
            assert technique in ids, f"{where}: unregistered producer {technique!r}"
            event("producer Technique id")
        check_fallback(record.fallback, setup, where)
    for entry in result.report.decisions:  # Fallback_Behaviors that produced no Constraint (20.7)
        check_fallback(entry.fallback, setup, f"the {entry.kind} decision at {entry.path}")

    analyzer = state.analyzer
    assert analyzer is not None
    for cid, record in analyzer.provenance.items():
        technique = record.technique
        if technique in ids:
            spec = REGISTRY.technique(technique)
            assert spec.implemented and setup.enabled(technique), f"{cid}: produced by the inactive {technique}"
        if cid.keyword in BASELINE_RULES:
            assert technique == f"baseline:{cid.keyword}", f"the analyzer's {cid} is produced by {technique!r}"
    final = result.model.provenance
    injected = state.injected
    for cid, record in final.items():
        if cid.keyword in BASELINE_RULES and cid in analyzer.provenance and cid not in injected:
            where = result.pointers[cid]
            assert record.technique == f"baseline:{cid.keyword}", f"{where} lost its Baseline_Rule marker"


def check_support_and_confidence(result: PipelineResult) -> None:
    """26.2: Support well formed; Confidence in [0, 1] and from the source the record names."""
    for cid, record in result.model.provenance.items():
        where = result.pointers[cid]
        support = record.support
        assert 0 <= support.consistent <= support.total, f"{where}: Support {support}"
        assert support.ratio == (support.consistent / support.total if support.total else 1.0), f"{where}: ratio"
        confidence = record.confidence
        assert math.isfinite(confidence) and 0.0 <= confidence <= 1.0, f"{where}: Confidence {confidence}"
        if record.sample is not None:
            assert record.sample.sampled <= record.sample.total, f"{where}: sample {record.sample}"
            event("sampled record")
        resolution = record.resolution
        method = None if resolution is None else resolution.method
        if method == "mdl":
            assert resolution is not None
            scores = [candidate.score for candidate in resolution.candidates]
            assert None not in scores, f"{where}: an MDL resolution without every description length"
            expected = posterior(scores, resolution.selected)  # type: ignore[arg-type]
            assert math.isclose(confidence, expected, rel_tol=1e-9, abs_tol=1e-12), f"{where}: not the MDL posterior"
            event("Confidence: MDL posterior")
        elif method == "decision-model":
            assert resolution is not None
            assert confidence == resolution.candidates[resolution.selected].score, f"{where}: not the probability"
            event("Confidence: Decision_Model probability")
        elif record.technique in KIND_SPECIFIC:
            if record.technique == COOCCURRENCE:
                count = record.details["support_count"]
                assert isinstance(count, int) and confidence == (count + 1) / (count + 2), f"{where}: rule Confidence"
            event("Confidence: kind-specific")
        else:
            laplace = (support.consistent + 1) / (support.total + 2)
            assert confidence == laplace, f"{where}: Confidence {confidence} is not the Laplace value {laplace}"
            event("Confidence: Laplace default")
        if record.changes:
            event("relaxed record")


def check_reference_support(result: PipelineResult, state: RunState) -> None:
    """26.2: every Support equals the reference over the raw documents, ``$defs`` totals over every site."""
    parsed = state.case.parsed
    reference = Reference(result.model, state.flagged_items)
    walked = [(doc, parsed[doc]) for doc in range(len(parsed)) if doc not in set(state.flagged_docs)]
    found = reference.walk(walked)
    in_defs = {node.nid for pointer, node in result.model.walk() if pointer.startswith("/$defs/")}
    sites: dict[int, int] = {}  # definition root nid -> number of nodes referencing it
    for node in reference.nodes.values():
        target = reference.target(node)
        if target is not None:
            sites[target.nid] = sites.get(target.nid, 0) + 1
    for cid, pointer in result.pointers.items():
        support = result.model.provenance[cid].support
        expected = reference.bounds(cid, found)
        assert expected.holds(support), f"{pointer}: Support {support} differs from the reference {expected}"
        event("Support compared exactly" if expected.exact else "Support compared within bounds")
        if cid.nid in in_defs:
            event("$defs Constraint")
            definition = reference.model.resolve(pointer_of("", *split_pointer(pointer)[:2]))
            if sites.get(definition.nid, 0) >= 2 and expected.exact and support.total:
                event("$defs Constraint counted over two referencing sites")
        if cid.keyword in ("anyOf", "oneOf") and support.consistent < support.total:
            event("branch Support below its total")


def judgment_of(node: SchemaNode, cid: ConstraintId, record: ProvenanceRecord) -> tuple[str, str] | None:
    """The Judgment_Call kind and outcome a Constraint expresses, if it is one's (design granularity table)."""
    keyword = cid.keyword
    if keyword == "properties":
        return "record_or_map", "record"
    if keyword == "additionalProperties":
        value = node.kw[keyword]
        return ("closed_or_open_record", "closed") if value.boolean is False else ("record_or_map", "map")
    if keyword == "prefixItems":
        return "tuple_or_list", "tuple"
    if keyword == "items":
        return "tuple_or_list", "tuple" if "prefixItems" in node.kw else "list"
    if keyword in ("enum", "const"):
        return "closed_or_open_enum", "closed"
    if keyword in ("anyOf", "oneOf"):
        assert record.resolution is not None, f"the branch {cid} carries no resolution"
        labels = {candidate.outcome for candidate in record.resolution.candidates}
        return ("merge_or_split", "split") if labels <= {"merge", "split"} else ("closed_or_open_enum", "closed")
    return None


def selected_label(resolution: Resolution) -> str:
    return resolution.candidates[resolution.selected].outcome


def check_judgment_calls(result: PipelineResult, state: RunState) -> None:
    """4.1, 4.7, 7.1, 26.3: every Judgment_Call outcome on its Constraint's record or in the report."""
    analyzer = state.analyzer
    assert analyzer is not None
    paths = logical_paths(analyzer)
    nodes = analyzer.index()
    names = {
        inner.nid
        for node in nodes.values()
        if node.boolean is None and isinstance(node.kw.get("propertyNames"), SchemaNode)
        for _, inner in node.kw["propertyNames"].walk()
    }
    assert set(nodes) - set(paths) <= names, "the analyzer's model holds nodes off its Paths ($defs or not)"

    held: dict[tuple[str, str, str | None], list[ConstraintId]] = {}
    for cid, record in analyzer.provenance.items():
        node = nodes[cid.nid]
        judged = judgment_of(node, cid, record)
        if judged is None:
            continue
        kind, label = judged
        resolution = record.resolution
        assert resolution is not None, f"{cid}: a {kind} outcome without its resolution"
        labels = [candidate.outcome for candidate in resolution.candidates]
        assert set(labels) <= set(JUDGMENT_OUTCOMES[kind]), f"{cid}: candidates {labels} of {kind}"
        assert selected_label(resolution) == label, f"{cid}: {kind} selected {selected_label(resolution)!r}"
        if resolution.method in ("mdl", "decision-model"):
            # 4.1, 26.3: the rejected alternative with its score.
            assert sorted(labels) == sorted(JUDGMENT_OUTCOMES[kind]), f"{cid}: {kind} lacks an alternative"
            assert all(c.score is not None for c in resolution.candidates), f"{cid}: an unscored alternative"
        if kind in ("record_or_map", "tuple_or_list"):
            assert record.details.get("judgment") == label, f"{cid}: judgment detail {record.details.get('judgment')}"
        held.setdefault((kind, render_path(paths[cid.nid]), resolution_text(resolution)), []).append(cid)
        event(f"Judgment_Call {kind}={label} on its Constraint ({resolution.method})")

    entries = [entry for entry in result.report.decisions if entry.kind in JUDGMENT_OUTCOMES]
    keys = {(entry.kind, entry.path, resolution_text(entry.resolution)) for entry in entries}
    for key, cids in held.items():
        assert key in keys, f"{cids}: no report decision of {key[0]} at {key[1]} with the record's resolution"
    rejected = [entry for entry in result.report.rejected if entry.category == "enums"]
    for entry in entries:
        label = selected_label(entry.resolution)
        key = (entry.kind, entry.path, resolution_text(entry.resolution))
        if (entry.kind, label) in PRODUCING:
            assert key in held, f"{entry.kind}={label} at {entry.path} produced no Constraint carrying its record"
        else:
            assert key not in held, f"{entry.kind}={label} at {entry.path} is on a Constraint it does not produce"
            event(f"Judgment_Call {entry.kind}={label} in the report only ({entry.resolution.method})")
            if entry.kind == "closed_or_open_enum" and entry.resolution.method != "fallback":
                found = [
                    r for r in rejected
                    if r.path == entry.path and r.details.get("method") == entry.resolution.method
                    and r.details.get("outcome") == "open" and "enum_statistics" in r.details
                ]
                assert found, f"the open enum at {entry.path} has no rejected enum candidate with its statistics"

    # 4.7: every Record Path resolves Closed-or-Open, the 4.11 Paths without keys included.
    records = Counter(e.path for e in entries if e.kind == "record_or_map" and selected_label(e.resolution) == "record")
    keyless = Counter(
        render_path(paths[cid.nid])
        for cid, record in analyzer.provenance.items()
        if cid.keyword == "type" and record.details.get("keys_available") is False
    )
    closed_open = Counter(e.path for e in entries if e.kind == "closed_or_open_record")
    assert closed_open == records + keyless, f"Closed-or-Open {dict(closed_open)} ≠ Records {dict(records + keyless)}"

    # 4.11 and 7.9: recorded on the type Constraint.
    branches = {child.nid for node in nodes.values() if node.boolean is None
                for keyword in ("anyOf", "oneOf") for child in node.kw.get(keyword, [])}
    for nid, node in nodes.items():
        if node.boolean is not None or "type" not in node.kw:
            continue
        listed = node.kw["type"]
        types = [listed] if isinstance(listed, str) else list(listed)
        details = analyzer.provenance[ConstraintId(nid, "type")].details
        additional = node.kw.get("additionalProperties")
        keyed = "properties" in node.kw or (isinstance(additional, SchemaNode) and additional.boolean is not False)
        if details.get("keys_available") is False:
            assert "object" in types and not keyed, f"node {nid}: 'no keys' on a node with keys"
            event("4.11 recorded on type")
        elif "object" in types and nid not in branches:
            assert keyed, f"node {nid}: an object node without keys does not record 4.11"
        arrays = "items" in node.kw or "prefixItems" in node.kw
        if details.get("elements_observed") is False:
            assert "array" in types and not arrays, f"node {nid}: 'no element' on a node with elements"
            event("7.9 recorded on type")
        elif "array" in types:
            assert arrays, f"node {nid}: an array node without elements does not record 7.9"

    # The final model keeps each outcome's record unless the soundness stage removed its Constraint or node.
    final = result.model.provenance
    final_nodes = result.model.index()
    removed = {cid for c in result.soundness.changes if c.removed is not None for cid in c.change.before_ids}
    injected = state.injected
    for (kind, path, text), cids in held.items():
        for cid in cids:
            if cid in injected or cid in removed or cid.nid not in final_nodes:
                continue
            source = analyzer.provenance[cid]
            kept = [
                record for other, record in final.items()
                if other.nid == cid.nid and resolution_text(record.resolution) == text
                and record.details == source.details
            ]
            assert kept, f"the {kind} outcome of {cid} at {path} left the final model"


def check_statistics(stats: Any, where: str) -> dict[str, Any]:
    """The six 15.1 statistics, well formed."""
    assert isinstance(stats, dict) and sorted(stats) == sorted(STATISTICS), f"{where}: statistics {stats}"
    n, s_obs, f1, f2 = stats["n"], stats["s_obs"], stats["f1"], stats["f2"]
    assert 1 <= s_obs <= n and 0 <= f1 <= s_obs and 0 <= f2 <= s_obs - f1, f"{where}: counts {stats}"
    assert math.isclose(stats["good_turing"], f1 / n), f"{where}: Good–Turing {stats}"
    chao1 = s_obs + f1 * f1 / (2 * f2) if f2 else s_obs + f1 * (f1 - 1) / 2
    assert math.isclose(stats["chao1"], chao1), f"{where}: Chao1 {stats}"
    return stats


def statistics_of(values: Sequence[Any]) -> dict[str, Any]:
    counts = Counter(canon(value) for value in values)
    n = sum(counts.values())
    f1 = sum(1 for c in counts.values() if c == 1)
    f2 = sum(1 for c in counts.values() if c == 2)
    s_obs = len(counts)
    chao1 = s_obs + f1 * f1 / (2 * f2) if f2 else s_obs + f1 * (f1 - 1) / 2
    return {"chao1": chao1, "f1": f1, "f2": f2, "good_turing": f1 / n, "n": n, "s_obs": s_obs}


def check_good_turing(result: PipelineResult, setup: Setup, state: RunState) -> None:
    """15.8: the statistics, outcome, and Technique of every Closed-or-Open-Enum judgment with statistics."""
    analyzer = state.analyzer
    assert analyzer is not None
    paths = logical_paths(analyzer)
    nodes = analyzer.index()
    judged: dict[str, list[tuple[dict[str, Any], list[Any] | None]]] = {}  # Path -> (statistics, enum values)
    for cid, record in analyzer.provenance.items():
        if cid.keyword not in ("enum", "const"):
            continue
        where = f"the {cid.keyword} at {render_path(paths[cid.nid])}"
        assert record.technique in (ENUM_ESTIMATION, "decide.dm_closed_enum"), f"{where}: by {record.technique}"
        assert record.details.get("outcome") == "closed", f"{where}: outcome {record.details.get('outcome')}"
        stats = check_statistics(record.details.get("enum_statistics"), where)
        value = nodes[cid.nid].kw[cid.keyword]
        members = [value] if cid.keyword == "const" else list(value)
        assert stats["s_obs"] == len(members) > stats["f1"], f"{where}: {len(members)} members, statistics {stats}"
        assert record.support.total == stats["n"], f"{where}: Support {record.support} over n = {stats['n']}"
        judged.setdefault(render_path(paths[cid.nid]), []).append((stats, members))
        event("Good–Turing record in provenance")
    for entry in result.report.rejected:
        if entry.category != "enums":
            continue
        where = f"the rejected enum at {entry.path}"
        assert entry.technique == ENUM_ESTIMATION, f"{where}: by {entry.technique}"
        assert entry.details.get("outcome") == "open", f"{where}: outcome {entry.details.get('outcome')}"
        stats = check_statistics(entry.details.get("enum_statistics"), where)
        keyword = "const" if stats["s_obs"] == 1 else "enum"
        assert entry.candidate == {"distinct_values": stats["s_obs"], "keyword": keyword}, f"{where}: {entry.candidate}"
        if entry.details.get("rule") == "15.5":
            assert stats["f1"] == stats["s_obs"], f"{where}: judged open by 15.5 with {stats}"
        else:
            assert entry.details.get("method") in ("mdl", "decision-model"), f"{where}: method {entry.details}"
        judged.setdefault(entry.path, []).append((stats, None))
        event("Good–Turing record in the report")

    enum_entries = [e for e in result.report.decisions if e.kind == "closed_or_open_enum"]
    enabled = setup.enabled(ENUM_ESTIMATION)
    if not enabled:
        assert not judged, "statistics recorded while enum estimation is disabled (15.7)"
        assert all(e.resolution.method == "fallback" and selected_label(e.resolution) == "open" for e in enum_entries)

    # The scalar Paths of the analyzer's model, and their scalar instances (not under Flagged_Items).
    parsed = state.case.parsed
    reference = Reference(analyzer, state.flagged_items)
    found = reference.walk((doc, parsed[doc]) for doc in range(len(parsed)) if doc not in set(state.flagged_docs))
    scalars: dict[str, dict[Inst, Any]] = {}
    undecided: set[str] = set()
    for nid, entry in found.items():
        node = nodes[nid]
        if nid not in paths or node.boolean is not None or "type" not in node.kw:
            continue
        typed = VALIDATOR.compile({"type": node.kw["type"]})
        path = render_path(paths[nid])
        for part, into in ((entry.certain, scalars.setdefault(path, {})), (entry.maybe, None)):
            for inst, value in part.items():
                if is_scalar(value) and VALIDATOR.is_valid(typed, value):
                    if into is None:
                        undecided.add(path)
                    else:
                        into[inst] = value
    fallback_paths = {e.path for e in enum_entries if e.resolution.method == "fallback"}
    for path in sorted(p for p in scalars if scalars[p] or p in undecided):
        if enabled:
            assert path in judged, f"the scalar Path {path} has no Closed-or-Open-Enum record (15.8)"
        else:
            assert path in fallback_paths, f"the scalar Path {path} has no Closed-or-Open-Enum fallback decision"
        if path in judged and len(judged[path]) == 1 and path not in undecided:
            stats, members = judged[path][0]
            values = list(scalars[path].values())
            raw = statistics_of(values)
            assert all(stats[k] == raw[k] for k in ("n", "s_obs", "f1", "f2")) and all(
                math.isclose(stats[k], raw[k]) for k in ("good_turing", "chao1")
            ), f"{path}: statistics {stats} ≠ the raw documents' {raw}"
            if members is not None:
                assert sorted(map(canon, members)) == sorted({canon(v) for v in values}), f"{path}: enum {members}"
            event("Good–Turing statistics recomputed")


def note_run(result: PipelineResult, setup: Setup, state: RunState) -> None:
    event(f"scenario {setup.scenario}")
    event(f"bundle {setup.bundle}")
    injection = state.injection
    for row in injection.rows if injection is not None else []:
        event(f"injected {row}")
    for count in injection.sites if injection is not None else []:
        event(f"$defs with {count} referencing site(s)")
    if state.flagged_docs:
        event("flagged documents")
    if state.flagged_items:
        event("Flagged_Items")
    summary = result.soundness.summary
    if summary.relaxations:
        event("relaxations")
    if summary.removals:
        event("removals")


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@given(data=st.data())
def test_every_emitted_constraint_has_one_complete_provenance_record(
    bundles: dict[str, str], data: st.DataObject
) -> None:
    case = data.draw(cases(), label="case")
    setup = data.draw(setups(bundles), label="setup")
    state = RunState(case)
    with (
        mock.patch.object(pipeline, "_detect_outliers", flagger(data, state)),
        mock.patch.object(StructureAnalyzer, "infer_model", injector(data, state)),
        mock.patch.object(pipeline, "check_soundness", spy(state)),
    ):
        result = infer_documents(list(case.values), setup.config)
    note(f"flagged documents {state.flagged_docs}, Flagged_Items {state.flagged_items}")
    note(f"schema: {result.schema.decode()}")

    check_one_to_one(result)
    check_producers(result, setup, state)
    check_support_and_confidence(result)
    check_reference_support(result, state)
    check_judgment_calls(result, state)
    check_good_turing(result, setup, state)
    note_run(result, setup, state)
