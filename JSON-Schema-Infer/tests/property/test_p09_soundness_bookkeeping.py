"""Property 9: Soundness bookkeeping (task 16.9).

For any run, including runs where over-tight Constraints are injected
before the soundness stage: the validated-document count equals the parsed
documents minus the Flagged_Documents of each schema; the first-pass
failure count is at most the validated count; the number of change records
equals the number of relaxations plus removals; and every relaxed
Constraint's Support is consistent with the post-change data
(Requirements 3.4, 3.6).

Runs
    Each example is a full ``pipeline.infer.infer_documents`` run with
    ``workers = 1``, a small ``soundness.max_passes`` (1 to 4, so both the
    relaxation passes and the removal loop of 3.7 occur), Technique states
    from ``run_configs()`` (``decide.mdl`` forced on or off in about two
    examples in three), small Resource_Parameters, and one of three
    Model_Bundles: the empty default bundle and two stub bundles of
    hand-set Learned_Thresholds for the implemented Techniques, written
    once per module. Three stage hooks are wrapped (``unittest.mock``, so
    no function-scoped fixture meets Hypothesis):

    - ``pipeline.infer._detect_outliers`` flags, in about three examples in
      five, one or two documents, one or two items (rows of any non-root
      column), or both, always leaving one document unflagged. No outlier
      Technique is implemented yet, so this stands in for the
      Outlier_Detector's output: the Flagged_Documents are the flagged
      documents plus every document holding a Flagged_Item (found by
      following ``parent`` links up to the root column).
    - ``StructureAnalyzer.infer_model`` returns the inferred model with
      over-tight Constraints injected (below).
    - ``pipeline.infer.check_soundness`` records the emitted layout and the
      Provenance_Records just before the stage, and the Provenance_Records
      just after it, so the checks see the stage's own records whatever
      provenance finalization (task 17.1) does afterwards.

Inputs
    Collections come from a random template (Records with optional
    fields, Maps with data-like keys, Lists, Tuples, and mixed types,
    nested up to two levels below the root), plus up to two arbitrary
    values (``tests.strategies.json_values``). Sometimes a NaN document is
    inserted; ``json.dumps`` cannot write it, so the Ingestor skips it and
    the parsed documents are the others, in order.

Injection
    One to four rows of the relaxation table are drawn per run (none in
    about one run in twelve; a row without a site is redrawn up to three
    times, and the row table is rotated by a function of the model so the
    draws spread over every row), and each is injected at a drawn node that
    the run's validated documents reach, tailored to the instances there so
    that some of them are rejected: tighter numeric, length, item-count,
    and key-count bounds (inclusive
    and exclusive); a narrowed ``type``; an ``enum`` or ``const`` missing an
    observed value; a ``format`` or ``pattern`` some strings miss; an extra
    ``required`` entry; a ``dependentRequired`` entry some objects break;
    ``additionalProperties: false`` with one observed field dropped from
    ``properties`` (or over a Map); ``items: false`` after a short
    ``prefixItems``; ``uniqueItems``; ``contains``; ``propertyNames``;
    ``not``; a ``oneOf`` that matches twice (sometimes with an ``anyOf``
    beside it, so that converting it creates an ``allOf`` entry); a
    ``false`` subschema (a property, a Tuple position, a branch, or rarely
    the root); and a subschema moved into the root ``$defs`` and referenced
    from its site (and from an equal subschema's site when there is one),
    then tightened inside the definition, sometimes with a ``not`` elsewhere
    referencing it too (widening the definition then needs a monotonicity
    guard). Each injected Constraint gets a Provenance_Record
    (technique ``decide.mdl``, details ``{"injected": row}``), loses any
    record it had, or keeps the inferred one, so both 3.6 record paths
    occur: an existing record extended, and a new record from the
    relaxation decision.

Reference
    Independent of ``pipeline.soundness``: the instances each node is
    evaluated on come from a walk of the raw parsed documents along the
    schema (``properties``, ``patternProperties``, ``additionalProperties``,
    ``propertyNames`` over keys, ``prefixItems``, ``items``, ``contains``
    over the elements it accepts, ``allOf``, the ``anyOf``/``oneOf`` branches
    that accept the instance, ``if`` with ``then`` or ``else`` by its
    verdict, and local ``$ref``; ``not`` contents and ``$defs`` only through
    references), each (document, instance location) once per node. A
    subschema is checked on its own with the root ``$defs`` copied in, which
    is enough because every reference in these models names a root
    ``$defs`` entry (the walk asserts it). The Support of a Constraint is
    ``total`` = the instances at its node of the JSON types its keyword
    applies to, and ``consistent`` = those that its keyword alone (with the
    siblings it reads: ``properties`` and ``patternProperties`` for
    ``additionalProperties``, ``prefixItems`` for ``items``, ``if`` for
    ``then`` and ``else``) accepts; for an ``anyOf``/``oneOf`` branch, those
    the branch accepts.

Checks
    - 3.4 counts: parsed documents = the non-NaN inputs; Flagged_Documents
      and validated documents equal the independent sets (in the result,
      the summary, and the pipeline's ``flagged``); ``failed_first_pass`` is
      exactly the validated documents the pre-soundness layout rejects, so
      at most the validated count; ``passes`` is one more than the last
      change's pass (one with no change, zero with nothing validated);
      ``pass_limit_reached`` holds exactly when the removal loop ran;
      relaxation passes precede ``max_passes`` and removal rounds follow it.
      One covering schema, recorded once in the report and equal to the
      summary; per-Version checks are gated on Schema_Versions being
      emitted (none are while ``robust.change_point`` is unimplemented).
    - Change records: their number is ``relaxations + removals``; removals
      are the removal loop's removal records; ``summary.removed`` holds one
      entry per removal record of either loop.
    - 3.6 records: each names its triggering documents (the Failure's,
      validated, and first-pass failures for pass 1), its before fragment,
      and its after fragment or REMOVED (a removal: ``after`` REMOVED or a
      ``false`` subschema replaced by ``true``). Under 3.3 it carries the
      decision's Resolution with every candidate scored (``mdl``, the
      selected one minimal) or the first in documented order
      (``fallback``), and that Resolution is a ``relaxation`` decision of
      the report, one per decision. A non-removal change under 3.3 gives a
      record to every Constraint it leaves; under 3.7 only to some.
      Removed Constraints' records leave the model; each other change's
      record is in the final record of every Constraint it was given to.
      An existing record is extended (its own fields kept), records of
      untouched Constraints are unchanged, and every record belongs to a
      Constraint of the final model.
    - Support: every Constraint whose record received a change has the
      reference Support over the final schema and the validated documents.

**Validates: Requirements 3.4, 3.6**
"""

# Feature: json-schema-inference, Property 9: Soundness bookkeeping

from __future__ import annotations

import copy
import json
import math
from collections import Counter
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any
from unittest import mock
from urllib.parse import quote, unquote

import pytest
from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import ObservationStore, ShapePath
from jsinfer.core.provenance import REMOVED, ProvenanceRecord, Support
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode, join_pointer, split_pointer
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Compiled, Validator
from jsinfer.pipeline import infer as pipeline
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.pipeline.soundness import AppliedChange, SoundnessResult
from jsinfer.structure.analyzer import StructureAnalyzer
from tests.records import complete_records
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_values, run_configs

# One Validator and one Schema_Emitter for the module: compiled schemas are cached across examples.
VALIDATOR = Validator()
EMITTER = SchemaEmitter()
MDL = "decide.mdl"
MAX_PASSES = "soundness.max_passes"

# The stage functions the test wraps, captured before any patch.
ORIGINAL_INFER_MODEL = StructureAnalyzer.infer_model
ORIGINAL_DETECT_OUTLIERS = pipeline._detect_outliers
ORIGINAL_CHECK_SOUNDNESS = pipeline.check_soundness

Loc = tuple[Any, ...]  # an instance location: keys, positions, and ("name", key) for a key under propertyNames
Inst = tuple[int, Loc]  # (document index, instance location)

# ---------------------------------------------------------------------------
# Model_Bundles
# ---------------------------------------------------------------------------

# Hand-set Learned_Thresholds of the implemented Techniques, inside their admissible ranges.
STUB_THRESHOLDS: dict[str, dict[str, int | float]] = {
    "stub-loose": {
        "lsh.keyset_jaccard": 0.5,
        "dbscan.eps": 0.34,
        "dbscan.min_samples": 1,
        "hdbscan.min_cluster_size": 2,
        "cooccur.min_support": 1,
        "cooccur.min_confidence": 0.5,
    },
    "stub-tight": {
        "lsh.keyset_jaccard": 1.0,
        "dbscan.eps": 1e-9,
        "dbscan.min_samples": 2,
        "hdbscan.min_cluster_size": 3,
        "cooccur.min_support": 2,
        "cooccur.min_confidence": 0.0,
    },
}


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> dict[str, str]:
    """Bundle identifiers by name: the shipped empty default and the stub bundles, written once."""
    root: Path = tmp_path_factory.mktemp("p09-bundles")
    found = {"default": DEFAULT_MODEL_BUNDLE}
    for name, values in STUB_THRESHOLDS.items():
        directory = root / name
        directory.mkdir()
        manifest = {"artifacts": {}, "bundle_id": f"p09-{name}", "format": 1}
        entries = {
            threshold: ThresholdRecord.from_spec(
                THRESHOLDS[threshold], value=value, objective_value=0.0, n_examples=1, training_data="p09"
            ).to_json()
            for threshold, value in values.items()
        }
        (directory / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (directory / "thresholds.json").write_text(json.dumps(entries), encoding="utf-8")
        found[name] = str(directory)
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


def json_at(document: JSON, pointer: str) -> JSON:
    value: Any = document
    for token in split_pointer(pointer):
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def is_skipped(value: Any) -> bool:
    return isinstance(value, float) and math.isnan(value)


# ---------------------------------------------------------------------------
# Collections
# ---------------------------------------------------------------------------

STRINGS = ("", "a", "ab", "abc", "A1", "12", "-5", "2.5", "a-1", "2024-01-02", "2024-01-02T03:04:05Z", "x@y.co",
           "10.0.0.1", "é😀")
SCALAR_VALUES = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(-3, 40),
    st.sampled_from([0.5, 2.5, -1.25, 1.0, 1e3, -0.0, 2**70]),
    st.sampled_from(STRINGS),
)
MAP_KEYS = st.integers(0, 99).map(lambda i: f"u{i}")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values: ``record`` fields ``(key, always, child)``; other kinds ``children``."""

    kind: str
    fields: tuple[tuple[str, bool, Template], ...] = ()
    children: tuple[Template, ...] = ()


SCALAR = Template("scalar")


@st.composite
def templates(draw: st.DrawFn, depth: int) -> Template:
    kinds = ["scalar", "scalar", "scalar"] + (["record", "record", "map", "list", "tuple", "mix"] if depth > 0 else [])
    kind = draw(st.sampled_from(kinds), label="kind")
    if kind == "scalar":
        return SCALAR
    if kind == "record":
        keys = draw(st.lists(st.sampled_from(KEYS), max_size=4, unique=True), label="fields")
        fields = tuple((key, draw(st.booleans(), label=f"{key!r} always"), draw(templates(depth - 1))) for key in keys)
        return Template("record", fields=fields)
    if kind in ("map", "list"):
        return Template(kind, children=(draw(templates(depth - 1)),))
    if kind == "tuple":
        return Template("tuple", children=tuple(draw(st.lists(templates(depth - 1), min_size=1, max_size=3))))
    return Template("mix", children=tuple(draw(st.lists(templates(depth - 1), min_size=2, max_size=3))))


@st.composite
def instances(draw: st.DrawFn, template: Template) -> Any:
    kind = template.kind
    if kind == "scalar":
        return draw(SCALAR_VALUES)
    if kind == "record":
        return {
            key: draw(instances(child))
            for key, always, child in template.fields
            if always or draw(st.booleans(), label=f"include {key!r}")
        }
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(MAP_KEYS, max_size=4), label="map keys"))
        return {key: draw(instances(template.children[0])) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0]), max_size=3), label="elements")
    if kind == "tuple":
        length = draw(st.integers(0, len(template.children)), label="tuple length")
        return [draw(instances(child)) for child in template.children[:length]]
    options = template.children
    return draw(instances(options[draw(st.integers(0, len(options) - 1), label="alternative")]))


@dataclass(frozen=True)
class Case:
    """The values ``infer_documents`` receives, NaN documents (skipped by the Ingestor) included."""

    values: tuple[Any, ...]

    @property
    def parsed(self) -> list[Any]:
        """Document ``i`` of the run: the ``i``-th value the Ingestor can write."""
        return [value for value in self.values if not is_skipped(value)]


@st.composite
def record_templates(draw: st.DrawFn, depth: int) -> Template:
    """A Record with one to five fields, so that object rows of the relaxation table find sites."""
    keys = draw(st.lists(st.sampled_from(KEYS), min_size=1, max_size=5, unique=True), label="root fields")
    fields = tuple((key, draw(st.booleans(), label=f"{key!r} always"), draw(templates(depth - 1))) for key in keys)
    return Template("record", fields=fields)


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    shape = draw(st.sampled_from(["record", "record", "record", "records", "mixed", "any"]), label="root shape")
    if shape == "record":
        return draw(record_templates(2))
    if shape == "records":
        return Template("list", children=(draw(record_templates(2)),))
    if shape == "mixed":
        return Template("mix", children=(draw(record_templates(1)), SCALAR, draw(templates(1))))
    return draw(templates(2))


@st.composite
def cases(draw: st.DrawFn) -> Case:
    root = draw(root_templates(), label="root template")
    docs = [draw(instances(root), label="document") for _ in range(draw(st.integers(3, 14), label="documents"))]
    docs += draw(st.lists(json_values(max_leaves=6), max_size=2), label="arbitrary documents")
    docs = [docs[i] for i in draw(st.permutations(range(len(docs))), label="order")]
    if draw(st.integers(0, 3), label="skipped documents") == 0:
        for _ in range(draw(st.integers(1, 2))):
            docs.insert(draw(st.integers(0, len(docs))), float("nan"))
    return Case(tuple(docs))


# ---------------------------------------------------------------------------
# Run_Configurations
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Setup:
    config: RunConfiguration
    bundle: str

    @property
    def max_passes(self) -> int:
        return int(self.config.resources[MAX_PASSES])

    @property
    def mdl(self) -> bool:
        return bool(self.config.techniques[MDL])


@st.composite
def setups(draw: st.DrawFn, bundles: Mapping[str, str]) -> Setup:
    bundle = draw(st.sampled_from(sorted(bundles)), label="bundle")
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    base: dict[str, Any] = {
        "workers": 1,
        # 3 first: the simplest draw then relaxes before any removal loop.
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
    mdl = draw(st.sampled_from(["drawn", "on", "off"]), label="decide.mdl state")
    if mdl != "drawn":
        base[technique_setting(MDL)] = mdl == "on"
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    return Setup(config, bundle)


# ---------------------------------------------------------------------------
# The reference walk
# ---------------------------------------------------------------------------


def ref_target(model: SchemaModel, node: SchemaNode) -> SchemaNode | None:
    """The node a local ``$ref`` names; every reference in these models names a root ``$defs`` entry."""
    if node.boolean is not None or "$ref" not in node.kw:
        return None
    ref = node.kw["$ref"]
    pointer = unquote(ref[1:]) if isinstance(ref, str) and ref.startswith("#") else ""
    if pointer == "":  # a recursive definition whose body is the root (8.4)
        return model.root
    assert split_pointer(pointer)[:1] == ["$defs"], f"the reference walk needs root $defs references, got {ref!r}"
    return model.resolve(pointer)


ROOT_DEF = "root"


def _refers_to_root(value: JSON) -> bool:
    if isinstance(value, dict):
        return any(k == "$ref" and v == "#" or _refers_to_root(v) for k, v in value.items())
    return isinstance(value, list) and any(_refers_to_root(v) for v in value)


def _reroot(value: JSON, name: str) -> JSON:
    """``value`` with every ``{"$ref": "#"}`` pointing at ``#/$defs/<name>`` instead."""
    if isinstance(value, dict):
        return {k: (f"#/$defs/{name}" if k == "$ref" and v == "#" else _reroot(v, name)) for k, v in value.items()}
    if isinstance(value, list):
        return [_reroot(v, name) for v in value]
    return value


class Reach:
    """The instances each node of a model is evaluated on (see Reference in the module docstring)."""

    def __init__(self, model: SchemaModel) -> None:
        self.model = model
        self.doc, _ = EMITTER.layout(model)
        self.pointers = {node.nid: pointer for pointer, node in model.walk()}
        self.nodes = model.index()
        defs = self.doc.get("$defs") if isinstance(self.doc, dict) else None
        self.defs: JSON | None = defs
        self._checks: dict[int, Compiled] = {}

    def standalone(self, part: JSON) -> JSON:
        """``part`` as a schema of its own: the root ``$defs`` copied in, which every reference names.

        A ``"#"`` reference (a recursive root definition, 8.4) would name the
        part itself, so the root is copied in as a definition and such
        references are redirected to it.
        """
        if isinstance(part, bool):
            return part
        defs = dict(self.defs) if isinstance(self.defs, dict) else {}
        if isinstance(self.doc, dict) and _refers_to_root(part):
            name = ROOT_DEF
            while name in defs:
                name += "_"
            defs[name] = _reroot({k: v for k, v in self.doc.items() if k != "$defs"}, name)
            part = _reroot(part, name)
        if not defs:
            return part
        return {**part, "$defs": defs}  # type: ignore[dict-item]

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

    def instances(self, documents: Iterable[tuple[int, JSON]]) -> dict[int, dict[Inst, JSON]]:
        """Per nid, the instances evaluated there, keyed by (document, instance location)."""
        found: dict[int, dict[Inst, JSON]] = {}
        for doc, value in documents:
            seen: set[tuple[int, Loc]] = set()
            stack: list[tuple[SchemaNode, Any, Loc]] = [(self.model.root, value, ())]
            while stack:
                node, instance, loc = stack.pop()
                if (node.nid, loc) in seen:
                    continue
                seen.add((node.nid, loc))
                found.setdefault(node.nid, {})[(doc, loc)] = instance
                stack.extend(self.successors(node, instance, loc))
        return found

    def successors(self, node: SchemaNode, instance: Any, loc: Loc) -> list[tuple[SchemaNode, Any, Loc]]:
        if node.boolean is not None:
            return []
        kw = node.kw
        out: list[tuple[SchemaNode, Any, Loc]] = []
        if isinstance(instance, dict):
            properties = kw.get("properties", {})
            patterns = kw.get("patternProperties", {})
            additional = kw.get("additionalProperties")
            names = kw.get("propertyNames")
            for key, member in instance.items():
                matched = [p for p in patterns if VALIDATOR.matches_pattern(p, key)]
                if key in properties:
                    out.append((properties[key], member, (*loc, key)))
                out.extend((patterns[p], member, (*loc, key)) for p in matched)
                if key not in properties and not matched and isinstance(additional, SchemaNode):
                    out.append((additional, member, (*loc, key)))
                if isinstance(names, SchemaNode):
                    out.append((names, key, (*loc, ("name", key))))
        if isinstance(instance, list):
            prefix = kw.get("prefixItems", [])
            items = kw.get("items")
            contains = kw.get("contains")
            for index, element in enumerate(instance):
                if index < len(prefix):
                    out.append((prefix[index], element, (*loc, index)))
                elif isinstance(items, SchemaNode):
                    out.append((items, element, (*loc, index)))
                if isinstance(contains, SchemaNode) and self.accepts(contains, element):
                    out.append((contains, element, (*loc, index)))
        for child in kw.get("allOf", []):
            out.append((child, instance, loc))
        for keyword in ("anyOf", "oneOf"):
            for child in kw.get(keyword, []):
                if self.accepts(child, instance):
                    out.append((child, instance, loc))
        condition = kw.get("if")
        if isinstance(condition, SchemaNode):
            out.append((condition, instance, loc))
            branch = kw.get("then") if self.accepts(condition, instance) else kw.get("else")
            if isinstance(branch, SchemaNode):
                out.append((branch, instance, loc))
        target = ref_target(self.model, node)
        if target is not None:
            out.append((target, instance, loc))
        return out

    # -- Support ---------------------------------------------------------------

    def support(self, cid: ConstraintId, found: Mapping[Inst, JSON]) -> Support:
        """The reference Support of ``cid`` over the instances at its node."""
        sub = self.subschema(self.nodes[cid.nid])
        assert isinstance(sub, dict), f"{cid} names a keyword of a boolean schema"
        values = [value for value in found.values() if applies(cid.keyword, value)]
        compiled = VALIDATOR.compile(self.standalone(fragment(sub, cid)))  # type: ignore[arg-type]
        return Support(sum(1 for value in values if VALIDATOR.is_valid(compiled, value)), len(values))


NUMBER_KEYWORDS = frozenset({"minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf"})
STRING_KEYWORDS = frozenset({"minLength", "maxLength", "pattern", "format"})
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
    """Whether a Constraint keyword applies to an instance (otherwise it is not counted in Support)."""
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


# ---------------------------------------------------------------------------
# Injection of over-tight Constraints
# ---------------------------------------------------------------------------

FORMATS = ("date", "date-time", "email", "ipv4", "uuid", "hostname")
PATTERNS = ("^(?:[a-z]{1,3})$(?!\\n)", "^(?:[0-9]+)$(?!\\n)", "^[a-z]*$", "^a", "\\d", "^(?:[a-z]+|[0-9]{2})$(?!\\n)")
DEF_NAMES = ("d0", "a b", "é~/")
# Characters left unescaped in an injected $ref fragment.
REF_SAFE = "/"


@dataclass
class Injector:
    """Draws over-tight Constraints into a model (see Injection in the module docstring)."""

    data: st.DataObject
    model: SchemaModel
    values: dict[int, list[JSON]]  # per nid, the validated documents' instances in the inferred model
    rows: list[str] = field(default_factory=list)
    with_record: int = 0
    without_record: int = 0

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
        """Give the injected Constraint a record, take away its record, or keep the inferred one."""
        choice = self.draw(st.sampled_from(["record", "none", "keep"]), f"record of {cid.keyword}")
        if choice == "record":
            self.model.provenance[cid] = ProvenanceRecord.laplace(MDL, Support(n, n), details={"injected": row})
        elif choice == "none":
            self.model.provenance.pop(cid, None)
        if cid in self.model.provenance:
            self.with_record += 1
        else:
            self.without_record += 1

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
        exclusive = inj.draw(st.booleans(), "exclusive bound")
        if side == "max":
            keyword = "exclusiveMaximum" if exclusive else "maximum"
            value = high if exclusive else (low if low < high else low - 1)
        else:
            keyword = "exclusiveMinimum" if exclusive else "minimum"
            value = low if exclusive else (high if low < high else high + 1)
    else:
        if side == "max" and high == 0:
            side = "min"
        keyword = side + kind
        value = inj.draw(st.integers(0, high - 1) if side == "max" else st.integers(low + 1, high + 1), "bound")
    inj.set(node, keyword, value, "bound")
    return True


def inject_type(inj: Injector, nodes: list[SchemaNode]) -> bool:
    if not nodes:
        return False
    node = inj.index(nodes, "type site")
    current = node.kw.get("type")
    listed = [current] if isinstance(current, str) else list(current or [])
    if len(listed) >= 2:
        drop = inj.draw(st.integers(0, len(listed) - 1), "dropped type")
        narrowed = [name for i, name in enumerate(listed) if i != drop]
    else:
        observed = {jtype(v) for v in inj.of(node)}
        others = [name for name in TYPE_NAMES if name not in observed and name not in listed]
        if "number" in listed:
            others.append("integer")
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
    inj.set(node, keyword, value, keyword)
    return True


def strings_of(inj: Injector, node: SchemaNode) -> list[str]:
    return [v for v in inj.of(node) if isinstance(v, str)]


def inject_format(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if strings_of(inj, node)]
    if not sites:
        return False
    node = inj.index(sites, "format site")
    strings = strings_of(inj, node)
    missed = [f for f in FORMATS if any(not VALIDATOR.satisfies_format(f, s) for s in strings)]
    inj.set(node, "format", inj.draw(st.sampled_from(missed or list(FORMATS)), "format"), "format")
    return True


def inject_pattern(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if strings_of(inj, node)]
    if not sites:
        return False
    node = inj.index(sites, "pattern site")
    strings = strings_of(inj, node)
    missed = [p for p in PATTERNS if any(not VALIDATOR.matches_pattern(p, s) for s in strings)]
    inj.set(node, "pattern", inj.draw(st.sampled_from(missed or list(PATTERNS)), "pattern"), "pattern")
    return True


def objects_of(inj: Injector, node: SchemaNode) -> list[dict[str, Any]]:
    return [v for v in inj.of(node) if isinstance(v, dict)]


def keys_of(objects: Iterable[dict[str, Any]]) -> list[str]:
    return list(dict.fromkeys(key for obj in objects for key in obj))


def inject_required(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if objects_of(inj, node)]
    if not sites:
        return False
    node = inj.index(sites, "required site")
    objects = objects_of(inj, node)
    optional = [key for key in keys_of(objects) if any(key not in obj for obj in objects)]
    name = inj.draw(st.sampled_from([*optional, "zz"]), "required name")
    listed = list(node.kw.get("required", []))
    if name not in listed:
        listed.append(name)
    inj.set(node, "required", listed, "required", name)
    return True


def inject_dependent_required(inj: Injector, nodes: list[SchemaNode]) -> bool:
    options: list[tuple[SchemaNode, str, str]] = []
    for node in nodes:
        objects = objects_of(inj, node)
        keys = keys_of(objects)
        for key in keys:
            for other in (*keys, "zz"):
                if other != key and any(key in obj and other not in obj for obj in objects):
                    options.append((node, key, other))
    if not options:
        return False
    node, key, other = inj.index(options, "dependentRequired entry")
    entries = {name: list(deps) for name, deps in (node.kw.get("dependentRequired") or {}).items()}
    deps = entries.setdefault(key, [])
    if other not in deps:
        deps.append(other)
    inj.set(node, "dependentRequired", entries, "dependentRequired", key)
    return True


def inject_closed(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if any(objects_of(inj, node))]
    if not sites:
        return False
    node = inj.index(sites, "closed site")
    objects = objects_of(inj, node)
    properties = node.kw.get("properties")
    if isinstance(properties, dict):
        present = [key for key in properties if any(key in obj for obj in objects)]
        if present:
            key = inj.draw(st.sampled_from(present), "dropped field")
            inj.forget(properties[key])
            del properties[key]
            if not properties:
                del node.kw["properties"]
                inj.model.provenance.pop(ConstraintId(node.nid, "properties"), None)
    old = node.kw.get("additionalProperties")
    if isinstance(old, SchemaNode):
        inj.forget(old)
    inj.set(node, "additionalProperties", SchemaNode(boolean=False), "closed")
    return True


def inject_items_false(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [
        node
        for node in nodes
        if any(isinstance(v, list) for v in inj.of(node))
        and (isinstance(node.kw.get("prefixItems"), list) or isinstance(node.kw.get("items"), SchemaNode))
    ]
    if not sites:
        return False
    node = inj.index(sites, "items site")
    prefix, items = node.kw.get("prefixItems"), node.kw.get("items")
    if isinstance(prefix, list):
        if len(prefix) >= 2:
            keep = inj.draw(st.integers(1, len(prefix) - 1), "prefix kept")
            for child in prefix[keep:]:
                inj.forget(child)
            node.kw["prefixItems"] = prefix[:keep]
        if isinstance(items, SchemaNode):
            inj.forget(items)
    else:
        node.kw["prefixItems"] = [items]  # the List's element schema becomes position 0
    inj.record(ConstraintId(node.nid, "prefixItems"), "items-false", len(inj.of(node)))
    inj.set(node, "items", SchemaNode(boolean=False), "items-false")
    return True


def has_duplicates(value: Any) -> bool:
    return isinstance(value, list) and len({canon(e) for e in value}) < len(value)


def inject_unique_items(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if any(isinstance(v, list) for v in inj.of(node))]
    if not sites:
        return False
    duplicated = [node for node in sites if any(has_duplicates(v) for v in inj.of(node))]
    node = inj.index(duplicated or sites, "uniqueItems site")
    inj.set(node, "uniqueItems", True, "uniqueItems")
    return True


def inject_contains(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if any(isinstance(v, list) for v in inj.of(node))]
    if not sites:
        return False
    node = inj.index(sites, "contains site")
    elements = distinct(e for v in inj.of(node) if isinstance(v, list) for e in v if is_scalar(e))
    options = [{"type": "null"}, *({"const": e} for e in elements[:3])]
    old = node.kw.get("contains")
    if isinstance(old, SchemaNode):
        inj.forget(old)
    inj.set(node, "contains", SchemaNode.from_json(inj.draw(st.sampled_from(options), "contains")), "contains")
    return True


def inject_property_names(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if keys_of(objects_of(inj, node))]
    if not sites:
        return False
    node = inj.index(sites, "propertyNames site")
    keys = keys_of(objects_of(inj, node))
    options: list[dict[str, Any]] = [{"pattern": "^[a-z]+$"}]
    longest = max(len(key) for key in keys)
    if longest >= 1:
        options.append({"maxLength": longest - 1})
    if len(keys) >= 2:
        options.append({"enum": keys[1:]})
    old = node.kw.get("propertyNames")
    if isinstance(old, SchemaNode):
        inj.forget(old)
    schema = SchemaNode.from_json(inj.draw(st.sampled_from(options), "propertyNames"))
    inj.set(node, "propertyNames", schema, "propertyNames")
    return True


def inject_not(inj: Injector, nodes: list[SchemaNode]) -> bool:
    sites = [node for node in nodes if "not" not in node.kw]
    if not sites:
        return False
    node = inj.index(sites, "not site")
    values = inj.of(node)
    options: list[dict[str, Any]] = [{"type": name} for name in sorted({jtype(v) for v in values})]
    options += [{"const": v} for v in distinct(v for v in values if is_scalar(v))[:2]]
    inj.set(node, "not", SchemaNode.from_json(inj.draw(st.sampled_from(options), "not")), "not")
    return True


def inject_one_of(inj: Injector, nodes: list[SchemaNode]) -> bool:
    if not nodes:
        return False
    node = inj.index(nodes, "oneOf site")
    branches = node.kw.get("oneOf")
    if isinstance(branches, list):
        new = [len(branches)]
        branches.append(SchemaNode.from_json({}))  # every instance now matches its own branch and this one
    else:
        name = inj.draw(st.sampled_from(sorted({jtype(v) for v in inj.of(node)})), "oneOf type")
        pair = [{}, {"type": [name]}]
        if inj.draw(st.booleans(), "swap oneOf branches"):
            pair.reverse()
        node.kw["oneOf"] = [SchemaNode.from_json(branch) for branch in pair]
        new = [0, 1]
    for index in new:
        inj.record(ConstraintId(node.nid, "oneOf", index), "oneOf", len(inj.of(node)))
    if "anyOf" not in node.kw and inj.draw(st.booleans(), "an anyOf beside the oneOf"):
        # Converting the oneOf then needs an allOf entry (design, "allOf created by a relaxation").
        node.kw["anyOf"] = [SchemaNode.from_json({})]
        inj.record(ConstraintId(node.nid, "anyOf", 0), "oneOf", len(inj.of(node)))
    if "anyOf" in node.kw:
        event("injected oneOf beside anyOf")
    return True


def inject_false(inj: Injector, nodes: list[SchemaNode]) -> bool:
    positions: list[SchemaNode] = []
    for node in nodes:
        objects = objects_of(inj, node)
        arrays = [v for v in inj.of(node) if isinstance(v, list)]
        for name, child in node.kw.get("properties", {}).items():
            if child.boolean is not False and any(name in obj for obj in objects):
                positions.append(child)
        for index, child in enumerate(node.kw.get("prefixItems", [])):
            if child.boolean is not False and any(len(a) > index for a in arrays):
                positions.append(child)
        for keyword in ("anyOf", "oneOf"):
            positions.extend(child for child in node.kw.get(keyword, []) if child.boolean is not False)
    root = inj.model.root
    # A value inside the range: Hypothesis favors the ends of integer ranges.
    if root.boolean is None and inj.draw(st.integers(0, 15), "false root") == 9:
        inj.model.root = SchemaNode(boolean=False)
        inj.model.provenance.clear()
        event("injected a false root")
        return True
    if not positions:
        return False
    target = inj.index(positions, "false position")
    inj.forget(target)
    inj.replace(target, SchemaNode(boolean=False))
    return True


VALUE_ROWS: dict[str, Callable[[Injector, list[SchemaNode]], bool]] = {
    "bound": inject_bound,
    "type": inject_type,
    "enum": lambda inj, nodes: inject_enum(inj, nodes, "enum"),
    "const": lambda inj, nodes: inject_enum(inj, nodes, "const"),
    "format": inject_format,
    "pattern": inject_pattern,
}


def inject_defs(inj: Injector, nodes: list[SchemaNode]) -> bool:
    root = inj.model.root
    if root.boolean is not None:
        return False
    slots = inj.slots()
    in_defs = {n.nid for p, n in inj.model.walk() if p.startswith("/$defs/")}
    movable = [node for node in nodes if node is not root and node.nid in slots and node.nid not in in_defs]
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
    sites = [moved]
    if twins and inj.draw(st.booleans(), "a second referencing site"):
        sites.append(inj.index(twins, "second site"))
        event("injected $defs with two referencing sites")
    name = inj.draw(st.sampled_from(names), "definition name")
    ref = "#" + quote(join_pointer("", "$defs", name), safe=REF_SAFE)
    values = [v for site in sites for v in inj.of(site)]
    for site in sites:
        reference = SchemaNode.from_json({"$ref": ref})
        if site is not moved:
            inj.forget(site)
        inj.replace(site, reference)
        inj.record(ConstraintId(reference.nid, "$ref"), "$defs", len(inj.of(site)))
    root.kw.setdefault("$defs", {})[name] = moved
    inj.values[moved.nid] = values
    row = inj.draw(st.sampled_from(sorted(VALUE_ROWS)), "tightened inside the definition")
    if VALUE_ROWS[row](inj, [moved]):
        event(f"injected {row} inside $defs")
    negated = [node for node in inj.nodes() if "not" not in node.kw and node.nid not in inside]
    if negated and inj.draw(st.booleans(), "a not referencing the definition"):
        # Widening the definition could narrow this not, so a relaxation of it must remove the not (a guard).
        site = inj.index(negated, "not site")
        inj.set(site, "not", SchemaNode.from_json({"$ref": ref}), "$defs")
        event("injected a not referencing $defs")
    return True


ROWS: dict[str, Callable[[Injector, list[SchemaNode]], bool]] = {
    **VALUE_ROWS,
    "required": inject_required,
    "dependentRequired": inject_dependent_required,
    "closed": inject_closed,
    "items-false": inject_items_false,
    "uniqueItems": inject_unique_items,
    "contains": inject_contains,
    "propertyNames": inject_property_names,
    "not": inject_not,
    "oneOf": inject_one_of,
    "false": inject_false,
    "$defs": inject_defs,
}
ROW_NAMES = tuple(ROWS)


def inject(data: st.DataObject, model: SchemaModel, values: dict[int, list[JSON]]) -> Injector:
    """One to four rows (none in about one run in twelve); a row without a site is redrawn up to three times."""
    inj = Injector(data, model, values)
    # A value inside the range: Hypothesis favors the ends of integer ranges.
    nothing = data.draw(st.integers(0, 11), label="inject nothing") == 5
    wanted = 0 if nothing else data.draw(st.integers(1, 4), label="rows")
    # Row draws shrink towards index 0; rotating the table by a function of the model spreads the
    # examples over every row instead of crowding the first one.
    offset = (len(model.provenance) + 3 * len(values)) % len(ROW_NAMES)
    for _ in range(wanted):
        for _attempt in range(3):
            if model.root.boolean is not None:
                return inj
            row = ROW_NAMES[(offset + data.draw(st.integers(0, len(ROW_NAMES) - 1), label="row")) % len(ROW_NAMES)]
            note(f"injecting {row}")
            if ROWS[row](inj, inj.nodes()):
                inj.rows.append(row)
                break
            event(f"no site for {row}")
    return inj


# ---------------------------------------------------------------------------
# Stage hooks
# ---------------------------------------------------------------------------


@dataclass
class RunState:
    """What the hooks saw: the flags, the injection, and the model just before and after soundness."""

    case: Case
    flagged: tuple[int, ...] = ()
    injection: Injector | None = None
    pre_doc: JSON = None
    pre_provenance: dict[ConstraintId, ProvenanceRecord] = field(default_factory=dict)
    pre_defs: frozenset[int] = frozenset()
    post_provenance: dict[ConstraintId, ProvenanceRecord] = field(default_factory=dict)


def document_of(store: ObservationStore, shape: ShapePath, row: int) -> int:
    """The document of a row, by following ``parent`` links up to the root column (root row i is document i)."""
    while shape:
        row = int(store.columns[shape].parent[row])
        shape = shape[:-1]
    return row


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
            chosen = st.lists(st.sampled_from(others), min_size=1, max_size=2, unique=True)
            docs = data.draw(chosen, label="flagged documents")
        shapes = [shape for shape in store.columns if shape]
        items: list[tuple[ShapePath, int]] = []
        for _ in range(data.draw(st.integers(1, 2), label="flagged items") if shapes and kind != "documents" else 0):
            shape = shapes[data.draw(st.integers(0, len(shapes) - 1), label="item column")]
            rows = [row for row in range(len(store.columns[shape])) if document_of(store, shape, row) != keeper]
            if rows:
                items.append((shape, rows[data.draw(st.integers(0, len(rows) - 1), label="item row")]))
        if docs:
            store.flags.flag_documents(sorted(docs))
        for shape, row in items:
            store.flags.flag_items(shape, [row])
        state.flagged = tuple(sorted({*docs, *(document_of(store, shape, row) for shape, row in items)}))

    return detect_outliers


def injector(data: st.DataObject, state: RunState) -> Callable[[StructureAnalyzer, Any], SchemaModel]:
    def infer_model(self: StructureAnalyzer, view: Any) -> SchemaModel:
        model = ORIGINAL_INFER_MODEL(self, view)
        parsed = state.case.parsed
        flagged = set(state.flagged)
        live = [(doc, parsed[doc]) for doc in range(len(parsed)) if doc not in flagged]
        values = {nid: list(found.values()) for nid, found in Reach(model).instances(live).items()}
        state.injection = inject(data, model, values)
        return model

    return infer_model


def spy(state: RunState) -> Callable[..., SoundnessResult]:
    def check_soundness(model: SchemaModel, documents: Any, **kwargs: Any) -> SoundnessResult:
        state.pre_doc = copy.deepcopy(EMITTER.layout(model)[0])
        state.pre_provenance = dict(model.provenance)
        state.pre_defs = frozenset(n.nid for p, n in model.walk() if p.startswith("/$defs/"))
        result = ORIGINAL_CHECK_SOUNDNESS(model, documents, **kwargs)
        state.post_provenance = dict(model.provenance)
        # The injector leaves some Constraints without records on purpose (3.3: "a Constraint that had
        # none"); provenance finalization (26.1, task 17.1) rejects survivors without one, so give them
        # placeholders once the soundness state is captured.
        complete_records(model)
        return result

    return check_soundness


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def is_removal(change: AppliedChange) -> bool:
    """A removal record (design, What a removal is): ``after`` REMOVED, or ``false`` replaced by ``true``."""
    edit = change.change
    return edit.removed or (edit.edit == "replace" and edit.before is False and edit.after is True)


def resolution_text(resolution: Any) -> str:
    return json.dumps(resolution.to_json(), sort_keys=True)


def check_counts(result: PipelineResult, case: Case, setup: Setup, state: RunState) -> tuple[list[int], list[int]]:
    """3.4: the counts against the independent document sets; returns (validated, first-pass failures)."""
    parsed = case.parsed
    soundness = result.soundness
    summary = soundness.summary
    report = result.report

    versions = report.versions
    assert versions is not None and not versions.applied, "Schema_Versions are emitted: add per-Version segments here"
    assert report.soundness == (summary,), "the report's soundness entry differs from the summary"
    assert summary.version is None

    assert len(result.documents) == len(parsed), "parsed documents differ from the non-NaN inputs"
    flagged = list(state.flagged)
    validated = [doc for doc in range(len(parsed)) if doc not in set(flagged)]
    assert result.flagged == tuple(flagged) == soundness.flagged
    assert summary.flagged_documents == len(flagged)
    assert soundness.validated == tuple(validated)
    assert summary.validated == len(validated) == len(parsed) - len(flagged)

    compiled = VALIDATOR.compile(state.pre_doc)  # type: ignore[arg-type]
    failing = [doc for doc in validated if not VALIDATOR.is_valid(compiled, parsed[doc])]
    assert soundness.failed_first_pass == tuple(failing), "first-pass failures differ from the pre-soundness verdicts"
    assert summary.failed_first_pass == len(failing) <= summary.validated

    changes = soundness.changes
    removal_loop = [c for c in changes if c.phase == "removal"]
    if not validated:
        assert summary.passes == 0 and not changes
    elif not changes:
        assert summary.passes == 1
    else:
        assert summary.passes == max(c.pass_index for c in changes) + 1, "passes ≠ last change's pass + 1"
    assert bool(failing) == bool(changes), "first-pass failures without changes, or changes without failures"
    assert summary.pass_limit_reached == bool(removal_loop)
    if summary.pass_limit_reached:
        assert summary.passes > setup.max_passes
    else:
        assert summary.passes <= setup.max_passes
    for c in changes:
        if c.phase == "relaxation":
            assert 1 <= c.pass_index < setup.max_passes, f"relaxation after pass {c.pass_index}"
        else:
            assert c.pass_index >= setup.max_passes, f"removal round after pass {c.pass_index}"

    # Change records number relaxations + removals; summary.removed has one entry per removal record.
    removal_records = [c for c in changes if is_removal(c)]
    assert len(changes) == summary.relaxations + summary.removals
    assert summary.removals == sum(1 for c in removal_loop if is_removal(c))
    assert summary.relaxations == len(changes) - summary.removals
    assert [c for c in changes if c.removed is not None] == removal_records
    entries = [c.removed for c in removal_records if c.removed is not None]
    assert sorted(e.sort_key() for e in entries) == [e.sort_key() for e in summary.removed]
    return validated, failing


def check_records(
    result: PipelineResult, setup: Setup, state: RunState, validated: list[int], failing: list[int]
) -> None:
    """3.6: the change records, their Resolutions, and where they went."""
    changes = result.soundness.changes
    final = state.post_provenance
    for c in changes:
        record = c.record
        assert record.documents and record.documents == c.failure.documents
        assert set(record.documents) <= set(validated), "a change triggered by a document that is not validated"
        if c.pass_index == 1:
            assert set(record.documents) <= set(failing), "a pass-1 change triggered by a document that passed"
        assert record.before == c.change.before
        if is_removal(c):
            assert record.after is REMOVED and c.removed is not None and c.removed.change is record
            assert c.constraints == ()
        else:
            assert record.after is not REMOVED and record.after == c.change.after and c.removed is None
        if c.phase == "relaxation":
            resolution = record.resolution
            assert resolution is not None and resolution.candidates, "a 3.3 change without its candidates"
            if setup.mdl:
                scores = [candidate.score for candidate in resolution.candidates]
                assert resolution.method == "mdl" and None not in scores
                assert scores[resolution.selected] == min(scores)  # type: ignore[type-var]
            else:
                assert resolution.method == "fallback" and resolution.selected == 0
            if not is_removal(c):
                assert set(c.constraints) == set(c.change.after_ids), "a 3.3 change left a Constraint without a record"
        else:
            assert record.resolution is None
            assert set(c.constraints) <= set(c.change.after_ids)

    # One relaxation decision in the report per Resolution of the 3.3 records.
    resolutions = {id(c.record.resolution): c.record.resolution for c in changes if c.phase == "relaxation"}
    decided = [d.resolution for d in result.report.decisions if d.kind == "relaxation"]
    assert Counter(map(resolution_text, resolutions.values())) == Counter(map(resolution_text, decided))

    # Removed Constraints leave the model; other records sit in their holders' final records.
    nodes = result.model.index()
    assert all(cid.nid in nodes for cid in final), "a record of a Constraint the final model lacks"
    for j, c in enumerate(changes):
        given_later = {cid for d in changes[j + 1 :] for cid in d.constraints}
        removed_later = {cid for d in changes[j + 1 :] if d.removed is not None for cid in d.change.before_ids}
        if c.removed is not None:
            for cid in c.change.before_ids:
                assert cid not in final or cid in given_later, f"the removed Constraint {cid} kept its record"
        for cid in c.constraints:
            if cid in final and cid not in removed_later:
                assert any(r is c.record for r in final[cid].changes), f"the record of {cid} lost a change"

    removed_ids = {cid for c in changes if c.removed is not None for cid in c.change.before_ids}
    holders = {cid for c in changes for cid in c.constraints}
    for cid, record in final.items():
        before = state.pre_provenance.get(cid)
        if not record.changes:
            if before is not None:
                assert record.to_json() == before.to_json(), f"the record of the untouched {cid} changed"
            continue
        assert cid in holders, f"{cid} has changes no change record gave it"
        if before is not None and cid not in removed_ids:
            # 3.6: an existing record is extended, not replaced.
            kept = ("technique", "confidence", "fallback", "resolution", "cutoff", "sample", "details")
            assert {k: record.to_json().get(k) for k in kept} == {k: before.to_json().get(k) for k in kept}, cid
            assert record.changes[: len(before.changes)] == before.changes
            event("existing record extended")
        elif record.resolution is not None and resolution_text(record.resolution) == resolution_text(
            record.changes[0].resolution
        ):
            event("new record from the relaxation decision")
        else:
            event("record moved to a renamed Constraint")


def check_support(result: PipelineResult, case: Case, state: RunState, validated: list[int]) -> None:
    """3.6: the Support of every relaxed Constraint over the final schema and the validated documents."""
    relaxed = sorted(cid for cid, record in state.post_provenance.items() if record.changes)
    if not relaxed:
        return
    reach = Reach(result.model)
    parsed = case.parsed
    found = reach.instances((doc, parsed[doc]) for doc in validated)
    for cid in relaxed:
        expected = reach.support(cid, found.get(cid.nid, {}))
        actual = state.post_provenance[cid].support
        assert actual == expected, f"Support of the relaxed {cid}: {actual} ≠ reference {expected}"
        if cid.keyword in ("anyOf", "oneOf") and expected.consistent < expected.total:
            event("relaxed branch Support below its total")


def note_run(result: PipelineResult, case: Case, setup: Setup, state: RunState) -> None:
    summary = result.soundness.summary
    injection = state.injection
    rows = injection.rows if injection is not None else []
    for row in rows:
        event(f"injected {row}")
    if not rows:
        event("nothing injected")
    if injection is not None and injection.with_record:
        event("injected Constraint with a record")
    if injection is not None and injection.without_record:
        event("injected Constraint without a record")
    event(f"max_passes {setup.max_passes}")
    event("decide.mdl on" if setup.mdl else "decide.mdl off")
    event(f"bundle {setup.bundle}")
    if state.flagged:
        event("Flagged_Documents")
    if len(case.parsed) < len(case.values):
        event("skipped documents")
    if summary.failed_first_pass == 0:
        event("first pass clean")
    if summary.pass_limit_reached:
        event("pass limit reached")
    final_defs = {n.nid for p, n in result.model.walk() if p.startswith("/$defs/")}
    for c in result.soundness.changes:
        if c.phase == "relaxation":
            event("removal chosen under 3.3" if is_removal(c) else "relaxation under 3.3")
        else:
            event("removal-loop removal" if is_removal(c) else "removal-loop guard change")
        if c.change.guard:
            event("monotonicity guard change")
        if c.change.edit == "convert":
            event("oneOf converted into allOf" if any(cid.keyword == "allOf" for cid in c.change.after_ids)
                  else "oneOf converted to anyOf")
        if c.change.nid in state.pre_defs or c.change.nid in final_defs:
            event("$defs edit")


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@given(data=st.data())
def test_soundness_bookkeeping_matches_the_documents_and_the_change_records(
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
    note(f"schema before soundness: {json.dumps(state.pre_doc, ensure_ascii=False)}")
    note(f"schema after soundness: {result.schema.decode()}")
    note(f"summary: {json.dumps(result.soundness.summary.to_json(), ensure_ascii=False)}")

    validated, failing = check_counts(result, case, setup, state)
    check_records(result, setup, state, validated, failing)
    check_support(result, case, state, validated)
    note_run(result, case, setup, state)
