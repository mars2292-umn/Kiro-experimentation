"""Property 10: Object nodes match their classification (task 16.3).

For any collection and any non-flagged object Path:

- Record: ``properties`` lists exactly the observed keys. If closed, the
  schema rejects an otherwise valid object with an added unknown key; if
  open, it accepts that object.
- Map: the node has a single ``additionalProperties`` value schema, no
  ``properties`` or ``required``, and any ``propertyNames`` Constraint
  accepts every observed key.
- Hybrid: the Record fields and Map entries partition the keys, the recorded
  partition DL is strictly below both pure encodings, and any
  ``patternProperties`` Pattern matches every Map key and no Record field.

The Structure_Analyzer (``StructureAnalyzer.infer_model``) runs on the root
view of each collection, and the clauses are checked at every object Path
of the emitted Schema_Model: the root, nested Records, objects in arrays
(List ``items`` and Tuple ``prefixItems`` positions), Map values, Variant
branches, and object parts beside other JSON types.

Reference
    The non-flagged instances at each schema node are collected from the raw
    documents by walking the schema and the documents together, with JSON
    Schema's applicator semantics: ``properties``, ``patternProperties``,
    and ``additionalProperties`` split an object's members, ``prefixItems``
    and ``items`` an array's elements. Instances are identified by their
    address ``(document index, steps)``; flagged documents, Flagged_Items,
    and everything below a Flagged_Item are left out. The observed keys of
    an object are all of its raw keys: a member whose value is a
    Flagged_Item still makes its key occur in the (non-flagged) object, as
    in Property 11 and the ViewStats.

Classification
    A spy :class:`RecordMapAnalyzer` subclass records each Record-or-Map and
    Closed-or-Open-Record outcome with the addresses of the objects it was
    resolved on (row addresses are rebuilt from the store's ``parent``,
    ``key``, and ``pos`` fields), and each Record node it builds. At an
    object Path the test looks up the outcome resolved on exactly the
    reference objects, so it also checks that the analyzer classified the
    Path's non-flagged objects. Every recorded outcome must be matched by
    exactly one object Path of the schema, which shows the walk reached
    every Path the analyzer classified, and the report's ``record_or_map``
    and ``closed_or_open_record`` entries must be those outcomes, Path for
    Path. A keyless Path (4.11) is a Record without a decision, and its
    ``type`` record notes that no keys were available.

Split Paths (two or more Variant branches)
    The Path's node is the ``anyOf``/``oneOf`` (possibly with a branch for
    the Path's other types). Each Variant branch is a Record node over its
    Variant's objects, taken from the spy (the branches must partition the
    Path's objects), so "``properties`` lists exactly the observed keys"
    holds per branch over that Variant's objects. Closed-or-Open is one
    decision per Path: closed puts ``additionalProperties: false`` inside
    every Variant branch and nothing beside the combinator; open puts it
    nowhere. The unknown-key test runs per branch (against the branch, on
    its Variant's objects) and against the whole combinator node.

The unknown-key test
    For a few non-flagged objects at the Path, the object is validated alone
    against the Path's node (or Variant branch); when it is valid, the same
    object with an added key that occurs nowhere at the Path must be
    rejected exactly when the Record is closed. An object that the node
    rejects (for example because of a Flagged_Item inside it) is skipped.
    When no ``anyOf``/``oneOf``/``allOf`` lies between the root and the
    Path, the whole emitted document is checked too: a non-flagged document
    that validates must be rejected after the key is added to the object
    exactly when the Record is closed. (Below a combinator, another branch
    may evaluate the same instance, so only the node-level test applies.)

Inputs
    Collections are drawn from a planted shape: Records with optional
    fields, Variants with disjoint key families, Maps with data-like keys
    (fresh ids, dates) or a small key pool, keyless objects, Lists, Tuples,
    and Paths mixing objects with scalars and arrays, nested up to three
    levels. Keys include the empty key and non-ASCII and astral keys.
    Outlier documents with a key found nowhere else are flagged, plus up to
    two other documents and up to three items (objects, member values, or
    elements). Technique states come from ``run_configs()`` (implemented
    Techniques only), with a stub Model_Bundle of hand-set Learned_Thresholds
    for the grouping Techniques and co-occurrence mining (each group present
    or absent).

    Two scenarios run with the full example budget each:

    - ``measured``: the real MDL_Scorer. Maps arise from data-like keys,
      splits from planted families; MDL only closes Records at several
      thousand objects, so measured Records are open.
    - ``stubbed``: the Record-or-Map and Closed-or-Open candidates are scored
      by a stub that picks open Record, closed Record, or Map per Path (from
      a drawn salt), with ``decide.mdl`` enabled, so closed Records and Maps
      appear at arbitrary Paths, including split Paths and keyless ones. In
      either scenario the stub bundle may hold a Closed-or-Open-Record
      Decision_Model that closes with certainty (task 22.4), so closed
      Records also occur in the measured scenario. The
      VariantAnalyzer's scorer halves the cost of Variant encodings at three
      Paths in four (same salt), so small planted families split there.
      Every other component uses the real MDL_Scorer.

Registry gating
    The hybrid clause applies once ``structure.hybrid_record_map`` is
    implemented (task 37.1) and enabled in the drawn configuration. Until
    then no object node may combine ``properties`` with
    ``patternProperties`` or an ``additionalProperties`` value schema. A
    hybrid node is recognized from its keywords; its partition DL is read
    from the report's ``record_map_partition`` entries at the Path and its
    pure encodings from the spy's Record-or-Map candidates (model plus data
    bits). Task 37.1 may tighten how the partition entry is located.

**Validates: Requirements 4.2, 4.3, 4.4, 4.5, 4.6, 4.8, 4.9**
"""

# Feature: json-schema-inference, Property 10: Object nodes match their classification

from __future__ import annotations

import copy
import hashlib
import itertools
from collections import Counter
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass, replace
from typing import Any

import numpy as np
import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.observations import ObservationStore, ShapePath, build_store
from jsinfer.core.paths import MAPVAL_STEP, render_path
from jsinfer.core.provenance import CandidateScore
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.learn.bundle import decision_model_artifact
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.analyzer import (
    BRANCH_CLOSED_ENUM,
    BRANCH_REMAINING_TYPES,
    RECORD_MAP_PARTITION,
    StructureAnalyzer,
)
from jsinfer.structure.record_map import (
    CLOSED_OR_OPEN_RECORD,
    RECORD_OR_MAP,
    ClosedOpenOutcome,
    ObjectNode,
    RecordMapAnalyzer,
    RecordMapOutcome,
    object_rows,
)
from jsinfer.structure.variants import VariantAnalyzer
from jsinfer.values.inferrer import ValueInferrer
from tests.models import FixedDecisionModel
from tests.strategies import TECHNIQUE_SETTINGS, run_configs

# One Validator and MDL_Scorer for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
SCORER = MDLScorer(validator=VALIDATOR)
EMITTER = SchemaEmitter()

MDL = "decide.mdl"
HYBRID = "structure.hybrid_record_map"
COMBINATORS = ("anyOf", "oneOf")
# Object keywords: none may appear on a node that no object reaches.
OBJECT_KEYWORDS = frozenset(
    {
        "properties",
        "patternProperties",
        "additionalProperties",
        "propertyNames",
        "minProperties",
        "maxProperties",
        "required",
        "dependentRequired",
    }
)
# Objects tested per node by the unknown-key test, alone and in their documents.
LOCAL_TESTS = 3
DOCUMENT_TESTS = 2

Step = str | int
Address = tuple[int, tuple[Step, ...]]  # (document index, keys and array indices from the root)
Instance = tuple[Address, Any]


# ---------------------------------------------------------------------------
# Planted shapes and their documents
# ---------------------------------------------------------------------------

FIELD_NAMES = ("a", "b", "id", "name", "", "é", "😀", "a b", "\uffff")
# Disjoint key families, so Variants separate (and split under MDL with enough objects).
FAMILIES = (("kind", "name", "age"), ("sku", "price"), ("lat", "lon", "alt"))
MAP_POOL = ("k1", "k2", "k3", "é", "")
OUTLIER_KEY = "zz-outlier"
MAX_DEPTH = 3

SCALARS: dict[str, st.SearchStrategy[Any]] = {
    "int": st.integers(-3, 40),
    "str": st.text("ab😀", max_size=3),
    "bool": st.booleans(),
    "null": st.none(),
    "num": st.sampled_from([0.5, 2.5, -1.25, 1.0, 1e3]),
    "enum": st.sampled_from(["red", "green", "blue"]),
}
SCALAR_KINDS = tuple(SCALARS)
OBJECT_SHAPES = ("record", "variants", "map", "keyless")
# Root and inner shapes, with Variants weighted up so that split Paths are common.
ROOT_SHAPES = ("record", "variants", "variants", "map", "keyless", "mixed", "list")
INNER_SHAPES = ("scalar", "scalar", "record", "variants", "variants", "map", "keyless", "list", "tuple", "mixed")

Shape = tuple[Any, ...]


@st.composite
def shapes(draw: st.DrawFn, depth: int = 0, kinds: Sequence[str] | None = None) -> Shape:
    """A planted shape drawn from ``kinds`` (by default every kind, scalars only at ``MAX_DEPTH``).

    Shapes are ``("scalar", kind)``, ``("record", fields)``, ``("variants", records)``,
    ``("map", key style, value)``, ``("keyless",)``, ``("list", element)``,
    ``("tuple", positions)``, and ``("mixed", options)``.
    """
    if kinds is None:
        kinds = ("scalar", "keyless") if depth >= MAX_DEPTH else INNER_SHAPES
    kind = draw(st.sampled_from(kinds), label="shape")
    child = shapes(depth + 1)
    if kind == "scalar":
        return ("scalar", draw(st.sampled_from(SCALAR_KINDS)))
    if kind == "keyless":
        return ("keyless",)
    if kind == "record":
        names = draw(st.lists(st.sampled_from(FIELD_NAMES), min_size=1, max_size=4, unique=True), label="fields")
        return ("record", tuple((name, draw(child), draw(st.booleans(), label="optional")) for name in names))
    if kind == "variants":
        families = draw(st.permutations(FAMILIES), label="families")[: draw(st.sampled_from([2, 2, 3]))]
        records = []
        for family in families:
            fields = []
            for name in family:
                nested = draw(st.integers(0, 3), label="nested field") == 0
                sub = draw(child) if nested else ("scalar", draw(st.sampled_from(SCALAR_KINDS)))
                fields.append((name, sub, draw(st.integers(0, 3), label="optional") == 0))
            records.append(("record", tuple(fields)))
        return ("variants", tuple(records))
    if kind == "map":
        return ("map", draw(st.sampled_from(("ids", "dates", "pool")), label="key style"), draw(child))
    if kind == "list":
        return ("list", draw(child))
    if kind == "tuple":
        return ("tuple", tuple(draw(st.lists(child, min_size=1, max_size=3))))
    # Mixed: an object shape next to one or two shapes of any kind.
    first = draw(shapes(depth + 1, kinds=OBJECT_SHAPES if depth + 1 < MAX_DEPTH else ("keyless",)))
    return ("mixed", (first, *draw(st.lists(child, min_size=1, max_size=2))))


def instantiate(draw: st.DrawFn, shape: Shape, fresh: Iterator[int]) -> Any:
    """One JSON value of ``shape``; ``fresh`` numbers Map ids, so they are distinct across the collection."""
    kind = shape[0]
    if kind == "scalar":
        return draw(SCALARS[shape[1]])
    if kind == "keyless":
        return {}
    if kind == "record":
        return {
            name: instantiate(draw, sub, fresh)
            for name, sub, optional in shape[1]
            if not optional or draw(st.booleans())
        }
    if kind == "variants":
        return instantiate(draw, shape[1][draw(st.integers(0, len(shape[1]) - 1))], fresh)
    if kind == "map":
        style, value = shape[1], shape[2]
        n = draw(st.integers(0, 3))
        if style == "ids":
            keys = [f"u{next(fresh)}" for _ in range(n)]
        elif style == "dates":
            keys = [f"2024-{draw(st.integers(1, 12)):02d}-{draw(st.integers(1, 28)):02d}" for _ in range(n)]
        else:
            keys = [draw(st.sampled_from(MAP_POOL)) for _ in range(n)]
        return {key: instantiate(draw, value, fresh) for key in keys}
    if kind == "list":
        return [instantiate(draw, shape[1], fresh) for _ in range(draw(st.integers(0, 3)))]
    if kind == "tuple":
        return [instantiate(draw, sub, fresh) for sub in shape[1]]
    assert kind == "mixed", shape
    return instantiate(draw, shape[1][draw(st.integers(0, len(shape[1]) - 1))], fresh)


def value_addresses(doc: Any, d: int) -> Iterator[Address]:
    """The address of every value inside document ``d``, except the document itself."""
    stack: list[tuple[tuple[Step, ...], Any]] = [((), doc)]
    while stack:
        steps, value = stack.pop()
        if steps:
            yield (d, steps)
        if isinstance(value, dict):
            stack.extend((steps + (key,), member) for key, member in value.items())
        elif isinstance(value, list):
            stack.extend((steps + (i,), element) for i, element in enumerate(value))


@dataclass(frozen=True)
class Case:
    """Documents and their flags: flagged document indices and flagged item addresses."""

    docs: tuple[Any, ...]
    flagged_docs: tuple[int, ...]
    flagged_items: tuple[Address, ...]
    shape: Shape


@st.composite
def cases(draw: st.DrawFn) -> Case:
    shape = draw(shapes(kinds=ROOT_SHAPES), label="root shape")
    fresh = itertools.count()
    # Larger collections let MDL split planted Variants (about ten objects per family) and pay for Maps.
    n = draw(st.one_of(st.integers(1, 8), st.integers(20, 40)), label="documents")
    units = [(instantiate(draw, shape, fresh), False) for _ in range(n)]
    for _ in range(draw(st.integers(0, 2), label="outliers")):
        outlier = instantiate(draw, shape, fresh)
        units.append(({**outlier, OUTLIER_KEY: 1} if isinstance(outlier, dict) else {OUTLIER_KEY: outlier}, True))
    order = draw(st.permutations(range(len(units))), label="document order")
    units = [units[i] for i in order]
    docs = tuple(doc for doc, _ in units)
    extra = draw(st.sets(st.integers(0, len(docs) - 1), max_size=2), label="flagged clean documents")
    flagged_docs = tuple(i for i, (_, outlier) in enumerate(units) if outlier or i in extra)
    inner = [address for d, doc in enumerate(docs) for address in value_addresses(doc, d)]
    picks = draw(st.sets(st.integers(0, len(inner) - 1), max_size=3), label="flagged items") if inner else set()
    return Case(docs, flagged_docs, tuple(inner[i] for i in sorted(picks)), shape)


# ---------------------------------------------------------------------------
# Store rows to addresses, and the store with its flags
# ---------------------------------------------------------------------------


def row_addresses(store: ObservationStore) -> dict[ShapePath, list[Address]]:
    """The address of every row of every column, rebuilt from the ``parent``, ``key``, and ``pos`` fields."""
    out: dict[ShapePath, list[Address]] = {}
    for shape in sorted(store.columns, key=len):
        column = store.columns[shape]
        if not shape:
            out[shape] = [(int(d), ()) for d in column.doc.tolist()]
            continue
        parents = out[shape[:-1]]
        if shape[-1] == MAPVAL_STEP:
            steps: list[Step] = [store.key_name(int(kid)) for kid in column.key.tolist()]
        else:
            steps = [int(pos) for pos in column.pos.tolist()]
        out[shape] = [
            (parents[p][0], parents[p][1] + (step,)) for p, step in zip(column.parent.tolist(), steps, strict=True)
        ]
    return out


def view_addresses(view: View, rows: Mapping[ShapePath, list[Address]]) -> frozenset[Address]:
    return frozenset(rows[part.column][r] for part in view.parts for r in part.rows.tolist())


def build(case: Case) -> tuple[ObservationStore, dict[ShapePath, list[Address]]]:
    """A fresh store of the documents with the case's flags, and its row addresses."""
    store = build_store(list(case.docs))
    rows = row_addresses(store)
    where = {address: (shape, r) for shape, addresses in rows.items() for r, address in enumerate(addresses)}
    by_column: dict[ShapePath, list[int]] = {}
    for address in case.flagged_items:
        shape, r = where[address]
        by_column.setdefault(shape, []).append(r)
    for shape, flagged in by_column.items():
        store.flags.flag_items(shape, np.array(sorted(flagged), dtype=np.int64))
    if case.flagged_docs:
        store.flags.flag_documents(case.flagged_docs)
    return store, rows


# ---------------------------------------------------------------------------
# The spy, the stub scorers, and the analyzer
# ---------------------------------------------------------------------------


class SpyRecordMap(RecordMapAnalyzer):
    """A RecordMapAnalyzer that records each outcome and Record node with the addresses of its objects."""

    def __init__(self, values: ValueInferrer, *, scorer: MDLScorer, rows: Mapping[ShapePath, list[Address]]) -> None:
        super().__init__(values, scorer=scorer)
        self.rows = rows
        self.kinds: list[tuple[frozenset[Address], RecordMapOutcome]] = []
        self.closures: list[tuple[frozenset[Address], ClosedOpenOutcome]] = []
        self.records: dict[int, frozenset[Address]] = {}

    def objects(self, view: View) -> frozenset[Address]:
        return view_addresses(object_rows(view), self.rows)

    def record_or_map(self, view: View) -> RecordMapOutcome:
        outcome = super().record_or_map(view)
        self.kinds.append((self.objects(view), outcome))
        return outcome

    def closed_or_open(self, view: View, *, record: SchemaNode | None = None) -> ClosedOpenOutcome:
        outcome = super().closed_or_open(view, record=record)
        self.closures.append((self.objects(view), outcome))
        return outcome

    def record_node(
        self,
        view: View,
        outcome: RecordMapOutcome,
        closed: ClosedOpenOutcome | None = None,
        *,
        child_schema: Any = naive_merge_node,
    ) -> ObjectNode:
        result = super().record_node(view, outcome, closed, child_schema=child_schema)
        self.records[result.node.nid] = self.objects(view)
        return result


def candidate_shape(node: SchemaNode) -> str:
    """Which object candidate a node is: ``closed`` (additionalProperties: false), ``map``, or ``record`` (open)."""
    additional = node.kw.get("additionalProperties")
    if additional is None:
        return "record"
    return "closed" if additional.boolean is False else "map"


CHOICES = ("open", "closed", "map")
# Candidate scores (model bits, data bits) by the stub's choice for a Path. The closed candidate's
# Good–Turing charge is added to its data bits, so the losing scores are far above any charge.
STUB_SCORES: dict[str, dict[str, tuple[float, float]]] = {
    "open": {"record": (10.0, 10.0), "map": (1e6, 1e6), "closed": (1e6, 1e6)},
    "closed": {"record": (1e5, 1e5), "map": (1e6, 1e6), "closed": (0.0, 0.0)},
    "map": {"record": (1e6, 1e6), "map": (0.0, 0.0), "closed": (1e6, 1e6)},
}


def salted(salt: int, purpose: str, view: View, n: int) -> int:
    """A choice in ``range(n)`` that depends only on the salt, the purpose, and the view's Path."""
    digest = hashlib.blake2b(f"{salt}|{purpose}|{render_path(view.path)}".encode(), digest_size=4).digest()
    return int.from_bytes(digest, "big") % n


class ChoiceScorer(MDLScorer):
    """Scores the Record-or-Map and Closed-or-Open candidates so that each Path takes a salted choice.

    ``universal_bits`` (the unseen-key charge) stays real.
    """

    def __init__(self, salt: int) -> None:
        super().__init__(validator=VALIDATOR)
        self.salt = salt

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        choice = CHOICES[salted(self.salt, "object", view, len(CHOICES))]
        model_bits, data_bits = STUB_SCORES[choice][candidate_shape(node)]
        return Score(model_bits, data_bits, 0.0, 0, 1)


class SplitScorer(MDLScorer):
    """The real MDL_Scorer, except that at three Paths in four (salted) a Variant encoding costs half.

    The grouping Techniques' greedy steps and noise assignment compare group
    lengths, which stay real; Variant selection and Merge-or-Split compare
    encodings with ``anyOf``, which this favors, so small planted families
    split at those Paths.
    """

    def __init__(self, salt: int) -> None:
        super().__init__(validator=VALIDATOR)
        self.salt = salt

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        score = super().score(node, view, **kwargs)
        if "anyOf" not in node.kw or salted(self.salt, "split", view, 4) == 0:
            return score
        return replace(
            score,
            model_bits=score.model_bits / 2,
            data_bits=score.data_bits / 2,
            exception_bits=score.exception_bits / 2,
        )


class StubBundle:
    """A Model_Bundle of hand-set Learned_Thresholds by name and stub Decision_Models by artifact name."""

    def __init__(self, thresholds: Mapping[str, int | float], models: Mapping[str, Any] | None = None) -> None:
        self.thresholds = dict(thresholds)
        self.models = dict(models or {})

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    "lsh.keyset_jaccard": st.floats(0.0, 1.0),
    "dbscan.eps": st.floats(0.0, 1.0, exclude_min=True),
    "dbscan.min_samples": st.integers(1, 6),
    "hdbscan.min_cluster_size": st.integers(2, 6),
    "cooccur.min_support": st.integers(1, 5),
    "cooccur.min_confidence": st.floats(0.0, 1.0),
}
THRESHOLD_TECHNIQUES = ("variants.minhash_lsh", "variants.dbscan", "variants.hdbscan", "rules.cooccurrence")


@st.composite
def thresholds(draw: st.DrawFn) -> dict[str, int | float]:
    """Each Technique's Learned_Thresholds, present together or absent (20.6: a missing one disables it)."""
    out: dict[str, int | float] = {}
    for tid in THRESHOLD_TECHNIQUES:
        if draw(st.booleans(), label=f"thresholds of {tid}"):
            for name in REGISTRY.technique(tid).thresholds:
                out[name] = draw(THRESHOLD_VALUES[name], label=name)
    return out


SCENARIOS = ("measured", "stubbed")


@dataclass(frozen=True)
class Setup:
    scenario: str
    config: RunConfiguration
    thresholds: Mapping[str, int | float]
    salt: int
    closing: bool  # the bundle holds a Closed-or-Open-Record Decision_Model that closes with certainty

    def models(self) -> dict[str, Any]:
        if not self.closing:
            return {}
        return {decision_model_artifact("closed_or_open_record", "synthetic"): FixedDecisionModel("closed_or_open_record", (1.0, 0.0))}


@st.composite
def setups(draw: st.DrawFn, scenario: str) -> Setup:
    base: dict[str, Any] = {technique_setting(MDL): True} if scenario == "stubbed" else {}
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    salt = draw(st.integers(0, 2**32 - 1), label="salt")
    closing = draw(st.booleans(), label="closing Decision_Model")
    return Setup(scenario, config, draw(thresholds(), label="thresholds"), salt, closing)


def analyzer(setup: Setup, rows: Mapping[ShapePath, list[Address]]) -> tuple[StructureAnalyzer, SpyRecordMap]:
    config = setup.config
    report = InferenceReport()
    bundle = StubBundle(setup.thresholds, setup.models())
    engine = DecisionEngine(config.techniques, bundle=bundle, report=report, scorer=SCORER)
    values = ValueInferrer(engine, validator=VALIDATOR, report=report, scorer=SCORER)
    stubbed = setup.scenario == "stubbed"
    spy = SpyRecordMap(values, scorer=ChoiceScorer(setup.salt) if stubbed else SCORER, rows=rows)
    variants = VariantAnalyzer(
        engine,
        scorer=SplitScorer(setup.salt) if stubbed else SCORER,
        resources=config.resources,
        seed=config.seed,
        validator=VALIDATOR,
    )
    return StructureAnalyzer(values, scorer=SCORER, record_map=spy, variants=variants), spy


# ---------------------------------------------------------------------------
# Validation helpers
# ---------------------------------------------------------------------------


def accepts(schema: Any, instance: Any) -> bool:
    return VALIDATOR.is_valid(VALIDATOR.compile(schema), instance)


def unknown_key(observed: set[str], patterns: Sequence[str] = ()) -> str | None:
    """A key that occurs nowhere at the Path and matches none of ``patterns``; None if none is found."""
    for i in range(20):
        key = f"zz-unknown-{i}"
        if key not in observed and not any(VALIDATOR.matches_pattern(p, key) for p in patterns):
            return key
    return None


def with_key(doc: Any, steps: Sequence[Step], key: str) -> Any:
    """A copy of ``doc`` in which the object at ``steps`` also holds ``key`` (with a null value)."""
    out = copy.deepcopy(doc)
    target = out
    for step in steps:
        target = target[step]
    assert isinstance(target, dict) and key not in target
    target[key] = None
    return out


def keys_of(objects: Sequence[Instance]) -> set[str]:
    return {key for _, obj in objects for key in obj}


def description_length(score: CandidateScore) -> float:
    assert score.model_bits is not None and score.data_bits is not None, f"{score} has no MDL breakdown"
    return score.model_bits + score.data_bits


def is_value_schema(node: Any) -> bool:
    """Whether ``node`` is a keyword subschema (not absent and not ``true``/``false``)."""
    return isinstance(node, SchemaNode) and node.boolean is None


def is_hybrid(node: SchemaNode) -> bool:
    """Whether an object node holds Record fields next to Map entries (4.5, 4.6)."""
    map_values = "patternProperties" in node.kw or is_value_schema(node.kw.get("additionalProperties"))
    return "properties" in node.kw and map_values


def members(objects: Sequence[Instance], keep: Callable[[str], bool]) -> list[Instance]:
    """The member values of ``objects`` whose keys ``keep`` accepts."""
    return [((d, (*steps, key)), value) for (d, steps), obj in objects for key, value in obj.items() if keep(key)]


def elements(arrays: Sequence[Instance], keep: Callable[[int], bool]) -> list[Instance]:
    """The elements of ``arrays`` whose indices ``keep`` accepts."""
    return [((d, (*steps, i)), value) for (d, steps), array in arrays for i, value in enumerate(array) if keep(i)]


# ---------------------------------------------------------------------------
# The walk over the schema and the reference instances
# ---------------------------------------------------------------------------


class Walk:
    """Walks the emitted Schema_Model with the non-flagged instances at each node, checking every object Path."""

    def __init__(self, case: Case, model: SchemaModel, spy: SpyRecordMap, report: InferenceReport, setup: Setup):
        self.case = case
        self.model = model
        self.spy = spy
        self.report = report
        self.hybrid_in_effect = REGISTRY.technique(HYBRID).implemented and setup.config.techniques[HYBRID]
        self.flagged = set(case.flagged_items)
        self.document = VALIDATOR.compile(EMITTER.layout(model)[0])
        self.kind_index: dict[frozenset[Address], list[int]] = {}
        for i, (seen, _) in enumerate(spy.kinds):
            self.kind_index.setdefault(seen, []).append(i)
        self.closure_index: dict[frozenset[Address], list[int]] = {}
        for i, (seen, _) in enumerate(spy.closures):
            self.closure_index.setdefault(seen, []).append(i)
        self.matched_kinds: list[int] = []
        self.matched_closures: list[int] = []

    def run(self) -> None:
        flagged_docs = set(self.case.flagged_docs)
        roots = [
            ((d, ()), doc)
            for d, doc in enumerate(self.case.docs)
            if d not in flagged_docs and (d, ()) not in self.flagged
        ]
        self.node(self.model.root, roots, "$", combinator_above=False, context=())
        # Each classified object view is exactly one object Path of the schema.
        matched = set(self.matched_kinds)
        unmatched = [render_path(outcome.path) for i, (_, outcome) in enumerate(self.spy.kinds) if i not in matched]
        assert sorted(self.matched_kinds) == list(range(len(self.spy.kinds))), (
            f"the analyzer classified object views at {unmatched} that no object Path of the schema holds"
        )
        assert sorted(self.matched_closures) == list(range(len(self.spy.closures))), "unmatched Closed-or-Open outcomes"

    # -- instances -----------------------------------------------------------

    def live(self, instances: Sequence[Instance]) -> list[Instance]:
        return [instance for instance in instances if instance[0] not in self.flagged]

    def record(self, node: SchemaNode, keyword: str, item: str | int | None = None) -> Any:
        cid = ConstraintId(node.nid, keyword, item)
        assert cid in self.model.provenance, f"no Provenance_Record for {cid}"
        return self.model.provenance[cid]

    # -- nodes ---------------------------------------------------------------

    def node(
        self,
        node: SchemaNode,
        instances: list[Instance],
        where: str,
        *,
        combinator_above: bool,
        context: tuple[str, ...],
    ) -> None:
        """Check the object Path at ``node`` (if objects reach it), then descend."""
        if node.boolean is not None:
            return
        kw = node.kw
        if "$ref" in kw:
            return  # Shared_Definitions are inferred from union views (Property 16)
        objects = [instance for instance in instances if isinstance(instance[1], dict)]
        combinator = next((keyword for keyword in COMBINATORS if keyword in kw), None)
        if combinator is not None:
            self.combinator(node, combinator, instances, objects, where, combinator_above, context)
            return
        if objects:
            assert "object" in kw.get("type", ()), f"{where}: objects reach a node whose type is {kw.get('type')}"
            self.object_path(node, objects, where, combinator_above, context, len(objects) < len(instances))
        else:
            stray = sorted(OBJECT_KEYWORDS.intersection(kw))
            assert not stray, f"{where}: object keywords {stray} on a node that no non-flagged object reaches"
        self.children(node, instances, where, combinator_above, context)

    def children(
        self, node: SchemaNode, instances: list[Instance], where: str, combinator_above: bool, context: tuple[str, ...]
    ) -> None:
        """Descend with JSON Schema's applicator semantics for members and elements."""
        kw = node.kw
        objects = [instance for instance in instances if isinstance(instance[1], dict)]
        arrays = [instance for instance in instances if isinstance(instance[1], list)]
        properties: dict[str, SchemaNode] = kw.get("properties", {})
        patterns: dict[str, SchemaNode] = kw.get("patternProperties", {})
        prefix: list[SchemaNode] = kw.get("prefixItems", [])

        def visit(sub: SchemaNode, values: list[Instance], step: str, tag: str) -> None:
            self.node(sub, self.live(values), where + step, combinator_above=combinator_above, context=(*context, tag))

        def unmatched(key: str) -> bool:
            return key not in properties and not any(VALIDATOR.matches_pattern(p, key) for p in patterns)

        for name, sub in properties.items():
            visit(sub, members(objects, lambda key: key == name), f"[{name!r}]", "record")  # noqa: B023
        for pattern, sub in patterns.items():
            matching = members(objects, lambda key: VALIDATOR.matches_pattern(pattern, key))  # noqa: B023
            visit(sub, matching, f"{{{pattern!r}}}", "map")
        if is_value_schema(kw.get("additionalProperties")):
            visit(kw["additionalProperties"], members(objects, unmatched), "{*}", "map")
        for i, sub in enumerate(prefix):
            visit(sub, elements(arrays, lambda j: j == i), f"[{i}]", "array")  # noqa: B023
        if is_value_schema(kw.get("items")):
            visit(kw["items"], elements(arrays, lambda j: j >= len(prefix)), "[*]", "array")
        for i, sub in enumerate(kw.get("allOf", [])):
            self.node(sub, instances, f"{where}/allOf/{i}", combinator_above=True, context=context)

    def combinator(
        self,
        node: SchemaNode,
        keyword: str,
        instances: list[Instance],
        objects: list[Instance],
        where: str,
        combinator_above: bool,
        context: tuple[str, ...],
    ) -> None:
        """A node with ``anyOf``/``oneOf``: Variant branches, a Closed_Enum branch, and a remaining-types branch."""
        branches: list[SchemaNode] = node.kw[keyword]
        assigned: dict[int, list[Instance]] = {}
        roles: dict[int, str] = {}
        for i, branch in enumerate(branches):
            role = self.record(node, keyword, i).details.get("branch")
            if role is None:
                assert branch.nid in self.spy.records, f"{where}/{keyword}/{i} is neither a Variant nor a typed branch"
                variant = self.spy.records[branch.nid]
                roles[i] = "variant"
                assigned[i] = [instance for instance in objects if instance[0] in variant]
                assert len(assigned[i]) == len(variant), f"{where}/{keyword}/{i}: Variant objects not at the Path"
            else:
                assert role in (BRANCH_CLOSED_ENUM, BRANCH_REMAINING_TYPES), f"{where}/{keyword}/{i}: branch {role!r}"
                roles[i] = role
        taken = {address for members in assigned.values() for address, _ in members}
        for i, role in roles.items():
            if role == BRANCH_CLOSED_ENUM:
                assigned[i] = [inst for inst in instances if not isinstance(inst[1], (dict, list))]
                taken.update(address for address, _ in assigned[i])
        rest = [inst for inst in instances if inst[0] not in taken]
        remaining = [i for i, role in roles.items() if role == BRANCH_REMAINING_TYPES]
        assert len(remaining) <= 1, f"{where}: {len(remaining)} remaining-types branches"
        if remaining:
            assigned[remaining[0]] = rest
        else:
            assert not rest, f"{where}: instances {[a for a, _ in rest][:3]} reach no branch of the {keyword}"

        variants = [(branches[i], assigned[i]) for i in sorted(roles) if roles[i] == "variant"]
        if variants:
            assert len(variants) >= 2, f"{where}: a {keyword} with a single Variant branch"
            mixed = len(variants) < len(branches)
            self.split_path(node, keyword, variants, objects, where, combinator_above, context, mixed)
        for i, branch in enumerate(branches):
            branch_where = f"{where}/{keyword}/{i}"
            if roles[i] == "variant":
                # The Path's object clauses were checked per branch above; continue below the Record.
                self.children(branch, assigned[i], branch_where, True, context)
            else:
                self.node(branch, assigned[i], branch_where, combinator_above=True, context=context)

    # -- object Paths --------------------------------------------------------

    def classification(self, objects: Sequence[Instance], where: str) -> RecordMapOutcome:
        found = self.kind_index.get(frozenset(address for address, _ in objects), [])
        assert len(found) == 1, f"{where}: {len(found)} Record-or-Map outcomes over the Path's non-flagged objects"
        self.matched_kinds.append(found[0])
        return self.spy.kinds[found[0]][1]

    def closure(self, objects: Sequence[Instance], where: str) -> ClosedOpenOutcome:
        found = self.closure_index.get(frozenset(address for address, _ in objects), [])
        assert len(found) == 1, f"{where}: {len(found)} Closed-or-Open outcomes over the Record's objects"
        self.matched_closures.append(found[0])
        return self.spy.closures[found[0]][1]

    def no_closure(self, objects: Sequence[Instance], where: str) -> None:
        found = self.closure_index.get(frozenset(address for address, _ in objects), [])
        assert not found, f"{where}: a Map Path resolved Closed-or-Open-Record"

    @staticmethod
    def note(label: str, context: tuple[str, ...], mixed: bool) -> None:
        event(label)
        if context:
            event("object Path below the root")
        if "array" in context:
            event("object Path in an array")
        if "map" in context:
            event("object Path in a Map value")
        if mixed:
            event("object Path beside other types")

    def object_path(
        self,
        node: SchemaNode,
        objects: list[Instance],
        where: str,
        combinator_above: bool,
        context: tuple[str, ...],
        mixed: bool,
    ) -> None:
        """The clauses at a Path whose objects share one schema object (no Variant split)."""
        outcome = self.classification(objects, where)
        observed = keys_of(objects)
        kw = node.kw
        if is_hybrid(node):
            self.hybrid(node, objects, outcome, observed, where, combinator_above)
            self.note("hybrid", context, mixed)
            return
        if not observed:
            # 4.11: a Record with no properties, classified without the Judgment_Call.
            assert outcome.is_record and outcome.decision is None, f"{where}: a keyless Path was resolved"
            assert "properties" not in kw, f"{where}: a keyless Record lists properties"
            details = self.record(node, "type").details
            assert details.get("keys_available") is False and details.get("judgment") == "record", (
                f"{where}: the type record of a keyless Record notes {dict(details)}"
            )
            closed = self.closure(objects, where)
            self.check_closing(node, closed.closed, where)
            self.unknown_key_tests(node, objects, closed.closed, observed, where, combinator_above)
            self.note(f"keyless Record ({closed.outcome})", context, mixed)
            return
        if outcome.is_map:
            self.map_path(node, objects, outcome, observed, where)
            self.note("Map with propertyNames" if "propertyNames" in kw else "Map", context, mixed)
            return
        # 4.2: properties lists exactly the observed keys.
        listed = set(kw.get("properties", {}))
        assert listed == observed, (
            f"{where}: properties {sorted(listed)} but the non-flagged objects hold {sorted(observed)}"
        )
        assert self.record(node, "properties").details["judgment"] == "record"
        assert "patternProperties" not in kw, f"{where}: a Record has patternProperties"
        closed = self.closure(objects, where)
        self.check_closing(node, closed.closed, where)
        self.unknown_key_tests(node, objects, closed.closed, observed, where, combinator_above)
        self.note(f"Record ({closed.outcome})", context, mixed)

    def check_closing(self, node: SchemaNode, closed: bool, where: str) -> None:
        """4.8: a closed Record has ``additionalProperties: false``; 4.9: an open one has none."""
        additional = node.kw.get("additionalProperties")
        if closed:
            assert isinstance(additional, SchemaNode) and additional.boolean is False, (
                f"{where}: a closed Record emits additionalProperties {additional!r}"
            )
            assert self.record(node, "additionalProperties").details["judgment"] == "closed"
        else:
            assert additional is None, f"{where}: an open Record emits additionalProperties {additional!r}"

    def map_path(
        self, node: SchemaNode, objects: list[Instance], outcome: RecordMapOutcome, observed: set[str], where: str
    ) -> None:
        """4.3 and 4.4 at a Map Path."""
        kw = node.kw
        assert outcome.decision is not None
        stray = sorted({"properties", "required", "patternProperties"}.intersection(kw))
        assert not stray, f"{where}: a Map emits {stray}"
        additional = kw.get("additionalProperties")
        assert isinstance(additional, SchemaNode) and additional.boolean is None, (
            f"{where}: a Map's additionalProperties is {additional!r}, not one value schema"
        )
        assert self.record(node, "additionalProperties").details["judgment"] == "map"
        self.no_closure(objects, where)
        names = kw.get("propertyNames")
        if names is not None:
            schema = names.to_json()
            rejected = sorted(key for key in observed if not accepts(schema, key))
            assert not rejected, f"{where}: propertyNames {schema!r} rejects observed keys {rejected[:3]!r}"

    def split_path(
        self,
        node: SchemaNode,
        keyword: str,
        variants: list[tuple[SchemaNode, list[Instance]]],
        objects: list[Instance],
        where: str,
        combinator_above: bool,
        context: tuple[str, ...],
        mixed: bool,
    ) -> None:
        """The Record clauses at a Path split into Variant branches, per branch over its Variant's objects."""
        outcome = self.classification(objects, where)
        assert outcome.is_record, f"{where}: a {outcome.kind} Path was split into Variants"
        owners = Counter(address for _, members in variants for address, _ in members)
        assert set(owners) == {address for address, _ in objects} and max(owners.values()) == 1, (
            f"{where}: the Variant branches do not partition the Path's non-flagged objects"
        )
        assert set(node.kw) == {keyword}, f"{where}: keywords {sorted(node.kw)} beside the {keyword}"
        closed = self.closure(objects, where)
        observed = keys_of(objects)
        for index, (branch, members) in enumerate(variants):
            branch_where = f"{where}/{keyword}/variant {index}"
            assert not is_hybrid(branch), f"{branch_where}: a Variant branch holds Map entries"
            expected = keys_of(members)
            listed = set(branch.kw.get("properties", {}))
            assert listed == expected, (
                f"{branch_where}: properties {sorted(listed)} but the Variant's objects hold {sorted(expected)}"
            )
            if expected:
                assert self.record(branch, "properties").details["judgment"] == "record"
            self.check_closing(branch, closed.closed, branch_where)
            self.unknown_key_tests(branch, members, closed.closed, observed, branch_where, combinator_above=True)
        self.unknown_key_tests(node, objects, closed.closed, observed, where, combinator_above)
        self.note(f"split Record ({closed.outcome}) under {keyword}", context, mixed)

    def hybrid(
        self,
        node: SchemaNode,
        objects: list[Instance],
        outcome: RecordMapOutcome,
        observed: set[str],
        where: str,
        combinator_above: bool,
    ) -> None:
        """4.5 and 4.6 at a partitioned Path (gated on ``structure.hybrid_record_map``)."""
        assert self.hybrid_in_effect, f"{where}: Record fields next to Map entries while {HYBRID} is not in effect"
        kw = node.kw
        fields = set(kw["properties"])
        entries = observed - fields
        assert fields <= observed, f"{where}: Record fields {sorted(fields - observed)} were never observed"
        assert fields and entries, f"{where}: a partition needs Record fields and Map entries"
        patterns: dict[str, SchemaNode] = kw.get("patternProperties", {})
        additional = kw.get("additionalProperties")
        if patterns:
            assert len(patterns) == 1, f"{where}: {len(patterns)} Map-entry Patterns"
            (pattern,) = patterns
            unmatched = sorted(key for key in entries if not VALIDATOR.matches_pattern(pattern, key))
            claimed = sorted(name for name in fields if VALIDATOR.matches_pattern(pattern, name))
            assert not unmatched and not claimed, (
                f"{where}: Pattern {pattern!r} misses Map keys {unmatched[:3]} or matches Record fields {claimed[:3]}"
            )
            assert additional is None or additional.boolean is False, f"{where}: additionalProperties beside a Pattern"
            closed = self.closure(objects, where)
            self.check_closing(node, closed.closed, where)
            self.unknown_key_tests(node, objects, closed.closed, observed, where, combinator_above, (pattern,))
        else:
            assert isinstance(additional, SchemaNode) and additional.boolean is None
            self.no_closure(objects, where)
        # The partition replaced the outcome only with a description length below both pure encodings.
        assert outcome.decision is not None
        pure = min(description_length(score) for score in outcome.decision.resolution.candidates)
        rendered = render_path(outcome.path)
        partitions = [
            entry
            for entry in self.report.decisions
            if entry.kind == RECORD_MAP_PARTITION and entry.path == rendered and entry.resolution.method != "fallback"
        ]
        selected = [entry.resolution.candidates[entry.resolution.selected] for entry in partitions]
        assert any(description_length(score) < pure for score in selected), (
            f"{where}: no recorded partition at {rendered} is below both pure encodings ({pure} bits)"
        )

    # -- the unknown-key test -------------------------------------------------

    def unknown_key_tests(
        self,
        node: SchemaNode,
        objects: Sequence[Instance],
        closed: bool,
        observed: set[str],
        where: str,
        combinator_above: bool,
        patterns: Sequence[str] = (),
    ) -> None:
        """4.8/4.9: an otherwise valid object with an added unknown key is rejected iff the Record is closed."""
        key = unknown_key(observed, patterns)
        if key is None:
            return
        compiled = VALIDATOR.compile(node.to_json())
        tested = 0
        for _, obj in objects:
            if tested == LOCAL_TESTS:
                break
            if not VALIDATOR.is_valid(compiled, obj):
                event("object rejected by its own node (skipped)")
                continue
            tested += 1
            accepted = VALIDATOR.is_valid(compiled, {**obj, key: None})
            assert accepted != closed, (
                f"{where}: a {'closed' if closed else 'open'} Record {'accepts' if accepted else 'rejects'} "
                f"{obj!r} with the unknown key {key!r}"
            )
        if combinator_above:
            return
        tested = 0
        for (d, steps), _ in objects:
            if tested == DOCUMENT_TESTS:
                break
            doc = self.case.docs[d]
            if not VALIDATOR.is_valid(self.document, doc):
                event("document not otherwise valid (skipped)")
                continue
            tested += 1
            accepted = VALIDATOR.is_valid(self.document, with_key(doc, steps, key))
            assert accepted != closed, (
                f"{where}: the emitted schema {'accepts' if accepted else 'rejects'} document {d} with the unknown "
                f"key {key!r} added at {steps!r}, but the Record is {'closed' if closed else 'open'}"
            )
            event("unknown key checked in the document")


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


def check_report(report: InferenceReport, spy: SpyRecordMap) -> None:
    """The report's Record-or-Map and Closed-or-Open entries are the spied outcomes, Path for Path."""
    reported = Counter(entry.path for entry in report.decisions if entry.kind == RECORD_OR_MAP)
    resolved = Counter(render_path(outcome.path) for _, outcome in spy.kinds if outcome.decision is not None)
    assert reported == resolved, f"record_or_map entries {dict(reported)} but outcomes {dict(resolved)}"
    reported = Counter(entry.path for entry in report.decisions if entry.kind == CLOSED_OR_OPEN_RECORD)
    resolved = Counter(render_path(outcome.path) for _, outcome in spy.closures)
    assert reported == resolved, f"closed_or_open_record entries {dict(reported)} but outcomes {dict(resolved)}"


@pytest.mark.parametrize("scenario", SCENARIOS)
@given(data=st.data())
def test_object_nodes_match_their_classification(scenario: str, data: st.DataObject) -> None:
    case = data.draw(cases(), label="case")
    setup = data.draw(setups(scenario), label="setup")
    store, rows = build(case)
    sa, spy = analyzer(setup, rows)
    model = sa.infer_model(View.root(store))
    report = sa.report
    assert report is not None
    check_report(report, spy)
    Walk(case, model, spy, report, setup).run()

    event(f"scenario {scenario}")
    if case.flagged_docs:
        event("flagged documents")
    if case.flagged_items:
        event("flagged items")
    if not setup.config.techniques[MDL]:
        event("decide.mdl disabled")
