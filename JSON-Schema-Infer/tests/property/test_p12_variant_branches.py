"""Property 12: Variant branch emission (task 16.4).

For any Path emitted with two or more Variants, there is exactly one branch
per Variant, and each branch's ``properties`` are a subset of the fields
observed in its Variant. ``oneOf`` is used if and only if every pair of
branches is disjoint under the definition of 5.5. A merged Variant's schema
has the union of the fields, a ``required`` set equal to their intersection,
and the union of the observed types per field.

Inputs
    Documents come from a planted *slot* (``slots()``): families of object
    templates (a base of ``(key, type)`` entries, drawn mostly from the
    family's own share of the keys, varied by dropping a base key and by
    adding or retyping keys, as in Property 11), plus a few values at the
    Path that are not objects (scalars, repeated strings, and arrays), so
    some Paths are mixed. The root slot has two or three families, each
    instantiated 3–12 times, and all of its non-object values. A template
    entry is a scalar type, a nested object, or an array of objects; nested
    values come from a slot of their own (one or two families, sometimes a
    non-object value), so Variants also arise at nested Paths and at ``[*]``
    Paths, inside each Variant branch of the parent (per-branch recursion). In about
    half the slots every template of a family also holds ``kind`` with one or
    two values of that family (``"x"``, ``1``, ``1.0``, ``True``, ...), so the
    Closed-or-Open-Enum Judgment_Call can close it to ``const``/``enum`` and
    open branches can be disjoint by value; ``1`` and ``1.0`` are one value
    under JSON Schema equality, and ``True`` is not ``1``. Keys come from
    :data:`tests.strategies.KEYS`. Up to three values anywhere below a
    document root (objects, member values, arrays, elements) are flagged as
    items, and up to two documents are flagged.

Configurations
    Technique states come from ``run_configs()`` (implemented Techniques
    only), with ``decide.mdl`` and ``variants.bernoulli_mixture`` (which
    reads no Learned_Threshold) forced on in most examples. A stub
    Model_Bundle holds hand-set Learned_Thresholds; each Technique's
    thresholds are present together or absent, so all four grouping
    Techniques and ``rules.cooccurrence`` run in some examples and not in
    others. The test runs once per mode, each with the full example budget:

    ``mdl``
        Every component scores with the real MDL_Scorer. On collections this
        small MDL mostly keeps one Variant, so the stub modes below steer the
        Judgment_Calls (only while ``decide.mdl`` is on; the Fallback_Behaviors
        and the priority order are untouched).
    ``splitting``
        The VariantAnalyzer scores with the real MDL_Scorer plus a fixed
        penalty on the one-Variant encoding of a whole grouping
        (:class:`PenaltyScorer`), so selection prefers a grouping with two
        or more Variants when one is offered and Merge-or-Split splits.
        Open branches are then disjoint only through a Closed_Enum on a
        common required field (``kind``).
    ``closing``
        As ``splitting``, and the RecordMapAnalyzer scores with a stub (as
        ``CLOSING`` in ``tests/unit/test_analyzer.py``) under which every
        object Path is a Record that closes, so each Variant branch holds
        ``additionalProperties: false`` and branches whose Variant requires a
        field the other does not list are disjoint.
    ``merging``
        As ``splitting`` for grouping selection, but the Merge-or-Split
        candidates are scored with a penalty on the ``anyOf`` (split)
        encoding, so two or more candidate Variants reach Merge-or-Split and
        merge (as ``ShapeScorer`` in ``tests/unit/test_variants.py`` does).

Reference
    The analyzer runs with a recording VariantAnalyzer whose ``branches``
    keeps every :class:`VariantNode` it builds: its Merge-or-Split outcome,
    the final Variants' views, and the branch nodes. Each view row is
    resolved to the raw document value it stands for, by following the
    store's ``parent``, ``key``, and ``pos`` fields up to the document
    (:class:`Locator`), so every reference fact comes from the raw documents
    and the case's flags, not from ViewStats. The fields observed in a
    Variant are the keys of its non-flagged objects; as in ``core.stats``
    and Naive_Merge (17.4), a member whose value is a Flagged_Item still
    makes its key a field of its non-flagged parent object, while its value
    contributes no observed type.

Checks
    Split Paths (two or more final Variants): the recorded branches are
    found in the emitted Schema_Model, under one combinator node (the host).
    The host's branch records that name a ``variant`` are exactly one per
    Variant, each on that Variant's branch, and the views of the Variants
    partition the Path's objects, all of which are non-flagged raw objects.
    Each branch has ``type: ["object"]`` and ``properties`` within its
    Variant's observed fields. Conversely, every host in the model with
    Variant branch records is one the recording saw.

    Mixed nodes: a split Path that also holds other JSON types becomes an
    ``anyOf`` of the Variant branches plus a remaining-types branch (and a
    Closed_Enum beside other types gets its own branch); those extra branches
    carry ``branch`` (``remaining_types`` or ``closed_enum``) instead of
    ``variant``. "One branch per Variant" is counted over the Variant
    branches, and the ``oneOf`` "iff" applies to every emitted branch of the
    host: an extra branch requires nothing and permits every field, so no
    pair with it is disjoint and ``anyOf`` is the agreeing combinator.

    Disjointness is judged by :func:`pairwise_disjoint`, the 5.5 reading in
    the design's Open Decisions, so ``oneOf`` must be emitted exactly when
    it returns True. The verdict is cross-checked against a plain-JSON
    reading of the same definition (:func:`disjoint_reason`, for the
    keywords a Record branch holds), which also names the reason for the
    statistics, and under ``oneOf`` no object at the Path validates against
    two branches.

    Merged Paths (a Merge-or-Split that merged two or more candidate
    Variants): the one object schema emitted for the merged Variant (the
    node itself, or the mixed node its keywords moved to) has ``properties``
    equal to the union of the candidate Variants' fields (within it, for a
    Record with ``patternProperties`` or an ``additionalProperties`` schema,
    which the hybrid partition of task 37.1 would emit), ``required`` equal
    to their intersection, and for each field the types its subschema
    accepts (``type``, or the union over ``anyOf``/``oneOf`` branches) equal
    to the types of the field's non-flagged values across the candidate
    Variants, with the integer rule; a field whose values are all flagged
    gets ``{}``.

**Validates: Requirements 5.4, 5.5, 5.6**
"""

# Feature: json-schema-inference, Property 12: Variant branch emission

from __future__ import annotations

import dataclasses
from collections import Counter
from collections.abc import Iterator, Mapping, Sequence
from dataclasses import dataclass
from typing import Any

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.jsonvalue import CanonicalKey, canonical_key
from jsinfer.core.observations import ObservationStore, ShapePath, build_store
from jsinfer.core.paths import ELEM, ELEM_STEP, render_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.analyzer import BRANCH_CLOSED_ENUM, BRANCH_REMAINING_TYPES, StructureAnalyzer
from jsinfer.structure.record_map import RecordMapAnalyzer
from jsinfer.structure.variants import (
    ANY_OF,
    BERNOULLI_MIXTURE,
    GROUPING_TECHNIQUES,
    ONE_OF,
    GroupingScorer,
    MergeSplitOutcome,
    VariantAnalyzer,
    VariantGrouping,
    VariantNode,
    VariantSchema,
    pairwise_disjoint,
)
from jsinfer.values.inferrer import ValueInferrer
from tests.strategies import KEYS, SCHEMA_TYPE_NAMES, TECHNIQUE_SETTINGS, run_configs

# One Validator for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
MDL = "decide.mdl"
COOCCURRENCE = "rules.cooccurrence"
SHARED_DEFS = "structure.shared_defs"
RECURSION = "structure.recursion"
COMBINATORS = (ANY_OF, ONE_OF)

Where = tuple[int, tuple[str | int, ...]]  # (document index, steps from the document root)

# ---------------------------------------------------------------------------
# Raw-document reference
# ---------------------------------------------------------------------------


def json_type(value: Any) -> str:
    """The JSON Schema type name of a raw value; zero-fraction numbers are ``integer`` (13.2)."""
    if value is None:
        return "null"
    if isinstance(value, bool):  # before numbers: bool is an int subclass
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "integer" if value.is_integer() else "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    assert isinstance(value, dict), value
    return "object"


def type_names(values: Sequence[Any]) -> frozenset[str]:
    """The type names of ``values`` with the integer rule: ``number`` replaces ``integer`` when present."""
    names = {json_type(value) for value in values}
    if "number" in names:
        names.discard("integer")
    return frozenset(names)


def locations(d: int, value: Any, steps: tuple[str | int, ...] = ()) -> Iterator[Where]:
    """Every value inside document ``d``, its root included, in pre-order."""
    yield (d, steps)
    if isinstance(value, dict):
        for key in sorted(value):
            yield from locations(d, value[key], steps + (key,))
    elif isinstance(value, list):
        for i, item in enumerate(value):
            yield from locations(d, item, steps + (i,))


class Locator:
    """The raw location of every store row, from its ``parent``, ``key``, and ``pos`` fields."""

    def __init__(self, store: ObservationStore) -> None:
        self.store = store
        self._columns: dict[ShapePath, list[Where]] = {}

    def column(self, shape: ShapePath) -> list[Where]:
        found = self._columns.get(shape)
        if found is None:
            col = self.store.columns[shape]
            if not shape:
                found = [(int(d), ()) for d in col.doc.tolist()]
            else:
                parents = self.column(shape[:-1])
                if shape[-1] == ELEM_STEP:
                    steps: list[str | int] = col.pos.tolist()
                else:
                    steps = [self.store.key_name(int(kid)) for kid in col.key.tolist()]
                found = [(parents[p][0], parents[p][1] + (s,)) for p, s in zip(col.parent.tolist(), steps)]
            self._columns[shape] = found
        return found

    def rows(self, view: View) -> list[Where]:
        out: list[Where] = []
        for part in view.parts:
            located = self.column(part.column)
            out.extend(located[r] for r in part.rows.tolist())
        return out


# ---------------------------------------------------------------------------
# Strategies: planted slots of object families, nested and in arrays
# ---------------------------------------------------------------------------

SCALAR_KINDS = ("null", "boolean", "integer", "number", "string")
NESTED_KINDS = ("object", "array")
MAX_DEPTH = 2  # slots at depth 0 and 1 may nest objects and arrays of objects
# A share of KEYS per family (a partition), from which most families draw their base keys.
FAMILY_KEYS: tuple[tuple[str, ...], ...] = (KEYS[1:3] + KEYS[4:5], KEYS[3:4] + KEYS[5:6] + KEYS[8:9], KEYS[0:1] + KEYS[6:8])

VALUES: dict[str, st.SearchStrategy[Any]] = {
    "null": st.none(),
    "boolean": st.booleans(),
    "integer": st.one_of(st.integers(-3, 3), st.sampled_from([1.0, 1e3, -0.0, 2**70])),
    "number": st.sampled_from([2.5, -0.5, 0.1]),
    "string": st.text("ab😀", max_size=3),
}

# Values at an object Path that are not objects: repeated strings make a Closed_Enum beside objects likely.
OTHERS: tuple[Any, ...] = (None, 3, 2.5, "s", "s", "t", True, [], [1, "s"], [{"a": 1}])

# The tag field and its values; 1, 1.0 are equal under JSON Schema equality, True is not 1.
TAG_KEY = "kind"
TAG_VALUES: tuple[Any, ...] = ("x", "y", "z", 1, 1.0, True)


@dataclass(frozen=True)
class Tag:
    values: tuple[Any, ...]


@dataclass(frozen=True)
class Nested:
    kind: str  # "object" or "array"
    slot: Slot


Spec = str | Tag | Nested
Template = tuple[tuple[str, Spec], ...]


@dataclass(frozen=True)
class Slot:
    """The values planted at one Path: object templates by family, and non-object values."""

    depth: int
    families: tuple[tuple[Template, ...], ...]
    others: tuple[Any, ...]

    @property
    def templates(self) -> tuple[Template, ...]:
        return tuple(template for family in self.families for template in family)


@st.composite
def slots(draw: st.DrawFn, depth: int = 0) -> Slot:
    kinds = SCALAR_KINDS + (NESTED_KINDS if depth < MAX_DEPTH else ())
    kind_of = st.sampled_from(kinds)
    nested: dict[tuple[str, str], Nested] = {}

    def spec(key: str, kind: str) -> Spec:
        if kind in SCALAR_KINDS:
            return kind
        if (key, kind) not in nested:  # one nested slot per (key, kind) of the slot, shared by its families
            nested[(key, kind)] = Nested(kind, draw(slots(depth + 1), label="nested slot"))
        return nested[(key, kind)]

    tagged = draw(st.booleans(), label="tagged")
    families: list[tuple[Template, ...]] = []
    for f in range(draw(st.integers(2, 3) if depth == 0 else st.integers(1, 2), label="families")):
        # Each family's base keys come mostly from its own share of KEYS, so families differ in structure.
        own = st.sampled_from(FAMILY_KEYS[f]) if draw(st.integers(0, 3), label="own keys?") else st.sampled_from(KEYS)
        size = (2, 3) if depth == 0 else (1, 2)
        base = draw(st.dictionaries(own, kind_of, min_size=size[0], max_size=size[1]), label="base")
        tag: Tag | None = None
        if tagged:
            picked = draw(st.sets(st.integers(0, len(TAG_VALUES) - 1), min_size=1, max_size=2), label="tag values")
            tag = Tag(tuple(TAG_VALUES[i] for i in sorted(picked)))
        members: list[Template] = []
        for _ in range(draw(st.integers(1, 3 if depth == 0 else 2), label="family members")):
            member = dict(base)
            if base and draw(st.booleans(), label="drop a base key"):
                del member[draw(st.sampled_from(sorted(base)), label="dropped key")]
            member.update(draw(st.dictionaries(st.sampled_from(KEYS), kind_of, max_size=2), label="added keys"))
            template: Template = tuple((key, spec(key, member[key])) for key in sorted(member))
            if tag is not None:
                template += ((TAG_KEY, tag),)
            members.append(template)
        families.append(tuple(members))
    others = tuple(draw(st.lists(st.sampled_from(OTHERS), max_size=3), label="non-object values"))
    return Slot(depth, tuple(families), others)


def instance(draw: st.DrawFn, template: Template) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, spec in template:
        if isinstance(spec, str):
            out[key] = draw(VALUES[spec])
        elif isinstance(spec, Tag):
            out[key] = draw(st.sampled_from(spec.values))
        elif spec.kind == "object":
            out[key] = slot_value(draw, spec.slot)
        else:
            size = draw(st.integers(0, 3 if spec.slot.depth <= 1 else 2), label="array length")
            out[key] = [slot_value(draw, spec.slot) for _ in range(size)]
    return out


def slot_value(draw: st.DrawFn, slot: Slot) -> Any:
    """One value at the slot's Path: mostly an instance of a template, sometimes a non-object."""
    if slot.others and draw(st.integers(0, 4), label="non-object value?") == 0:
        return draw(st.sampled_from(slot.others))
    templates = slot.templates
    return instance(draw, templates[draw(st.integers(0, len(templates) - 1), label="template")])


@dataclass(frozen=True)
class Case:
    docs: tuple[Any, ...]
    flagged_docs: tuple[int, ...]
    flagged_items: frozenset[Where]  # locations below a document root

    def live(self, where: Where) -> bool:
        """Whether the value at ``where`` is non-flagged: its document, itself, and its ancestors unflagged."""
        d, steps = where
        if d in self.flagged_docs:
            return False
        return not any((d, steps[:end]) in self.flagged_items for end in range(1, len(steps) + 1))

    def value(self, where: Where) -> Any:
        d, steps = where
        value = self.docs[d]
        for step in steps:
            value = value[step]
        return value


@st.composite
def cases(draw: st.DrawFn) -> Case:
    root = draw(slots(), label="root slot")
    # Every family of the root slot gets several documents, then the slot's non-object values join them.
    docs: list[Any] = []
    for family in root.families:
        for _ in range(draw(st.integers(3, 12), label="family size")):
            docs.append(instance(draw, draw(st.sampled_from(family), label="family member")))
    docs.extend(root.others)
    n = len(docs)
    docs = [docs[i] for i in draw(st.permutations(range(n)), label="document order")]

    below = [where for d, doc in enumerate(docs) for where in locations(d, doc) if where[1]]
    items: frozenset[Where] = frozenset()
    if below:
        items = frozenset(draw(st.lists(st.sampled_from(below), max_size=3), label="flagged items"))
    flagged_docs = tuple(sorted(draw(st.sets(st.integers(0, n - 1), max_size=min(2, n - 2)), label="flagged docs")))
    return Case(tuple(docs), flagged_docs, items)


def build(case: Case) -> tuple[ObservationStore, Locator]:
    """A store of the documents with the case's flags, and its row locator."""
    store = build_store(list(case.docs))
    locator = Locator(store)
    for shape in list(store.columns):
        if not shape:
            continue
        rows = [r for r, where in enumerate(locator.column(shape)) if where in case.flagged_items]
        if rows:
            store.flags.flag_items(shape, rows)
    if case.flagged_docs:
        store.flags.flag_documents(case.flagged_docs)
    return store, locator


# ---------------------------------------------------------------------------
# Configurations, the stub bundle, and the scorers of each mode
# ---------------------------------------------------------------------------

THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    "lsh.keyset_jaccard": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
    "dbscan.eps": st.one_of(st.sampled_from([1e-9, 0.34, 0.5, 1.0]), st.floats(0.0, 1.0, exclude_min=True)),
    "dbscan.min_samples": st.integers(1, 6),
    "hdbscan.min_cluster_size": st.integers(2, 6),
    "cooccur.min_support": st.integers(1, 5),
    "cooccur.min_confidence": st.floats(0.0, 1.0),
    "tagged.branch_support": st.one_of(st.sampled_from([0.0, 0.1, 0.3, 1.0]), st.floats(0.0, 1.0)),
}
THRESHOLD_TECHNIQUES = (*GROUPING_TECHNIQUES, COOCCURRENCE)


class StubBundle:
    """A Model_Bundle of hand-set Learned_Thresholds by name; no Decision_Models."""

    def __init__(self, thresholds: Mapping[str, int | float]) -> None:
        self.thresholds = dict(thresholds)

    def model(self, name: str) -> Any | None:
        return None

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


@dataclass(frozen=True)
class Setup:
    config: RunConfiguration
    thresholds: Mapping[str, int | float]


@st.composite
def setups(draw: st.DrawFn) -> Setup:
    perm = draw(st.sampled_from([8, 32, 128]), label="minhash.perm")
    base: dict[str, Any] = {
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.dbscan"),
        "sample.hdbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(2, 6), label="variants.max_components"),
    }
    # Mostly with MDL selection, and with the Bernoulli mixture (which reads no Learned_Threshold) in effect,
    # so most examples reach Merge-or-Split; otherwise both toggles are drawn too.
    for tid in (MDL, BERNOULLI_MIXTURE):
        if draw(st.integers(0, 3), label=f"force {tid}") != 0:
            base[technique_setting(tid)] = True
    base[technique_setting(SHARED_DEFS)] = False  # Path-by-Path walks: shared definitions are Property 16's
    base[technique_setting(RECURSION)] = False
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    # Each Technique's Learned_Thresholds are present together or absent (20.6: a missing one disables it).
    thresholds: dict[str, int | float] = {}
    for tid in THRESHOLD_TECHNIQUES:
        names = REGISTRY.technique(tid).thresholds
        if names and draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0:
            thresholds.update({name: draw(THRESHOLD_VALUES[name], label=name) for name in names})
    return Setup(config, thresholds)


def fixed(model_bits: float, data_bits: float) -> Score:
    return Score(float(model_bits), float(data_bits), 0.0, 0, 1)


class ClosingScorer(MDLScorer):
    """Scores Record-or-Map and Closed-or-Open candidates by shape: Record beats Map, closed beats open.

    ``universal_bits`` stays real, so the unseen-key charge of the closed
    candidate is the real one; it is far below the gap of the fixed scores.
    """

    __slots__ = ()

    SCORES: Mapping[str, Score] = {"record": fixed(5e4, 5e4), "map": fixed(1e5, 1e5), "closed": fixed(1, 1)}

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        additional = node.kw.get("additionalProperties")
        if additional is None:
            return self.SCORES["record"]
        return self.SCORES["closed" if additional.boolean is False else "map"]


class PenaltyScorer(MDLScorer):
    """The real MDL_Scorer, plus PENALTY model bits on whole-grouping encodings that are (or are not) ``anyOf``.

    :meth:`GroupingScorer.score` scores a whole grouping's encoding on the
    Path's objects and passes ``branch_views`` (None for one group); that is
    what grouping selection and Merge-or-Split compare, and exactly the
    encodings of two or more groups are ``anyOf`` nodes. The group lengths
    that the grouping Techniques' own DL-driven merges compare
    (:meth:`GroupingScorer.group_bits`) are scored without ``branch_views``
    and stay real. ``on_any_of=False`` (SPLITTING) makes the one-Variant
    encoding expensive, so selection prefers a grouping with two or more
    Variants whenever one is offered and Merge-or-Split splits;
    ``on_any_of=True`` (MERGING) makes the split encoding expensive, so
    Merge-or-Split merges.
    """

    __slots__ = ("on_any_of",)

    PENALTY = 1e6

    def __init__(self, *, on_any_of: bool) -> None:
        super().__init__(validator=VALIDATOR)
        self.on_any_of = on_any_of

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        found = super().score(node, view, **kwargs)
        if "branch_views" in kwargs and (ANY_OF in node.kw) == self.on_any_of:
            return dataclasses.replace(found, model_bits=found.model_bits + self.PENALTY)
        return found


CLOSING = ClosingScorer(validator=VALIDATOR)
SPLITTING = PenaltyScorer(on_any_of=False)
MERGING = PenaltyScorer(on_any_of=True)

# Mode → (RecordMapAnalyzer scorer, VariantAnalyzer scorer, Merge-or-Split scorer); None is the real MDL_Scorer.
MODES: dict[str, tuple[MDLScorer | None, MDLScorer | None, MDLScorer | None]] = {
    "mdl": (None, None, None),
    "splitting": (None, SPLITTING, None),
    "closing": (CLOSING, SPLITTING, None),
    "merging": (None, SPLITTING, MERGING),
}


class RecordingVariants(VariantAnalyzer):
    """A VariantAnalyzer that keeps every VariantNode it emits, in emission order.

    With ``merge_scorer``, the Merge-or-Split candidates are scored by it
    (through a fresh GroupingScorer) instead of by the grouping's scorer.
    """

    def __init__(self, *args: Any, merge_scorer: MDLScorer | None = None, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.emitted: list[VariantNode] = []
        self.merge_scorer = merge_scorer

    def merge_or_split(self, variants: VariantGrouping) -> MergeSplitOutcome:
        if self.merge_scorer is not None and variants.n_variants >= 2:
            variants = dataclasses.replace(variants, scorer=GroupingScorer(variants.signatures, self.merge_scorer))
        return super().merge_or_split(variants)

    def branches(self, outcome: MergeSplitOutcome, *, variant_schema: VariantSchema = naive_merge_node) -> VariantNode:
        node = super().branches(outcome, variant_schema=variant_schema)
        self.emitted.append(node)
        return node


def analyzer(setup: Setup, mode: str) -> tuple[StructureAnalyzer, RecordingVariants]:
    config = setup.config
    report = InferenceReport()
    scorer = MDLScorer(validator=VALIDATOR)
    engine = DecisionEngine(config.techniques, bundle=StubBundle(setup.thresholds), report=report, scorer=scorer)
    values = ValueInferrer(engine, validator=VALIDATOR, report=report, scorer=scorer)
    record_scorer, variant_scorer, merge_scorer = MODES[mode]
    record_map = RecordMapAnalyzer(values, scorer=record_scorer or scorer)
    variants = RecordingVariants(
        engine,
        scorer=variant_scorer or scorer,
        resources=config.resources,
        seed=config.seed,
        validator=VALIDATOR,
        merge_scorer=merge_scorer,
    )
    return StructureAnalyzer(values, scorer=scorer, record_map=record_map, variants=variants), variants


# ---------------------------------------------------------------------------
# Plain-JSON reading of 5.5 and accepted types
# ---------------------------------------------------------------------------

# The object keywords a Record branch may hold; the reference reading covers exactly these.
RECORD_KEYWORDS = frozenset({"properties", "additionalProperties", "required", "dependentRequired"})
OBJECT_KEYWORDS = RECORD_KEYWORDS | {"patternProperties", "propertyNames", "minProperties", "maxProperties"}
ALL_TYPES = frozenset(SCHEMA_TYPE_NAMES)


def value_set(branch: Any, name: str) -> frozenset[CanonicalKey] | None:
    """The values ``const``/``enum`` of ``properties[name]`` allow (intersected); None when unconstrained."""
    if not isinstance(branch, dict):
        return None
    sub = branch.get("properties", {}).get(name)
    if not isinstance(sub, dict):
        return None
    allowed: frozenset[CanonicalKey] | None = None
    if "const" in sub:
        allowed = frozenset({canonical_key(sub["const"])})
    if "enum" in sub:
        members = frozenset(canonical_key(value) for value in sub["enum"])
        allowed = members if allowed is None else allowed & members
    return allowed


def permits(branch: Any, name: str) -> bool:
    """Whether a Record branch lets an object hold key ``name``."""
    if isinstance(branch, bool):
        return branch
    properties = branch.get("properties", {})
    if name in properties:
        return properties[name] is not False
    return branch.get("additionalProperties", True) is not False


def disjoint_reason(a: Any, b: Any) -> str | None:
    """Why no object can match both branches by the 5.5 definition; None when they overlap."""
    required_a = a.get("required", []) if isinstance(a, dict) else []
    required_b = b.get("required", []) if isinstance(b, dict) else []
    for name in required_a:
        if name in required_b:
            va, vb = value_set(a, name), value_set(b, name)
            if va is not None and vb is not None and not va & vb:
                return "common required field with disjoint const/enum values"
    if any(not permits(b, name) for name in required_a) or any(not permits(a, name) for name in required_b):
        return "required field the other branch does not permit"
    return None


def accepted_types(schema: Any) -> frozenset[str]:
    """The type names a subschema lets through: ``type``, the union over combinator branches, or enum values."""
    if schema is True:
        return ALL_TYPES
    if schema is False:
        return frozenset()
    if "type" in schema:
        return frozenset(schema["type"])
    for keyword in COMBINATORS:
        if keyword in schema:
            return frozenset().union(*(accepted_types(branch) for branch in schema[keyword]))
    if "enum" in schema:
        return type_names(schema["enum"])
    if "const" in schema:
        return type_names([schema["const"]])
    return ALL_TYPES


def validates(schema: Any, instance_value: Any) -> bool:
    return VALIDATOR.is_valid(VALIDATOR.compile(schema), instance_value)


# ---------------------------------------------------------------------------
# The checks
# ---------------------------------------------------------------------------


def check_objects(case: Case, wheres: Sequence[Where], where: str) -> None:
    for at in wheres:
        assert case.live(at), f"{where}: {at} is flagged or under a flagged value"
        assert isinstance(case.value(at), dict), f"{where}: {at} is not an object"


def note_path(outcome: MergeSplitOutcome) -> None:
    path = outcome.path
    if any(step.kind == ELEM for step in path):
        event("Variants at a Path inside an array")
    elif path:
        event("Variants at a nested Path")
    else:
        event("Variants at $")


@dataclass(frozen=True)
class Host:
    node: SchemaNode
    keyword: str
    position: int


def combinator_hosts(model: SchemaModel) -> dict[int, Host]:
    """Each branch nid → the combinator node that holds it, the keyword, and its position."""
    hosts: dict[int, Host] = {}
    for node in model.index().values():
        for keyword in COMBINATORS:
            branches = node.kw.get(keyword)
            if isinstance(branches, list):
                for position, branch in enumerate(branches):
                    hosts[branch.nid] = Host(node, keyword, position)
    return hosts


def check_split(
    case: Case, locator: Locator, model: SchemaModel, hosts: Mapping[int, Host], emitted: VariantNode
) -> int:
    """5.4 and 5.5 at one Path with two or more final Variants; returns the host's nid."""
    outcome = emitted.outcome
    where = f"split at {render_path(outcome.path)}"
    views = outcome.views()
    n = outcome.n_variants
    assert n >= 2 and not outcome.merged and len(emitted.branches) == n == len(views), where
    assert [view.digest for view in emitted.views] == [view.digest for view in views], where

    # The branches are emitted together under one combinator node.
    assert emitted.branches[0].nid in hosts, f"{where}: the Variant branches are not in the emitted schema"
    host = hosts[emitted.branches[0].nid]
    node, keyword = host.node, host.keyword
    branches: list[SchemaNode] = node.kw[keyword]
    records = [model.provenance[ConstraintId(node.nid, keyword, p)] for p in range(len(branches))]

    # Exactly one branch per Variant (5.4), identified by the branch records.
    positions: dict[int, int] = {}
    extras: list[str] = []
    for p, rec in enumerate(records):
        if "variant" in rec.details:
            variant = rec.details["variant"]
            assert variant not in positions, f"{where}: Variant {variant} has two branches"
            positions[variant] = p
        else:
            extras.append(rec.details.get("branch"))
    assert sorted(positions) == list(range(n)), f"{where}: Variant branches {sorted(positions)} for {n} Variants"
    for variant, p in positions.items():
        assert branches[p] is emitted.branches[variant], f"{where}: branch {p} is not Variant {variant}'s"
    assert all(extra in (BRANCH_REMAINING_TYPES, BRANCH_CLOSED_ENUM) for extra in extras), f"{where}: {extras}"
    if extras:
        assert keyword == ANY_OF, f"{where}: a mixed node uses {keyword}"
        event("mixed node: Variant branches plus " + " and ".join(sorted(set(extras))))
    else:
        assert node is emitted.node and keyword == emitted.keyword, f"{where}: an objects-only node was replaced"

    # The Variants partition the Path's objects; each branch lists only its Variant's fields (5.4).
    members = [locator.rows(view) for view in views]
    everything = locator.rows(outcome.variants.signatures.objects)
    flat = Counter(at for wheres in members for at in wheres)
    assert flat == Counter(everything) and max(flat.values()) == 1, f"{where}: the Variants do not partition"
    check_objects(case, everything, where)
    for variant, wheres in enumerate(members):
        branch = emitted.branches[variant]
        assert branch.kw.get("type") == ["object"], f"{where}: Variant {variant} branch type {branch.kw.get('type')}"
        fields = {key for at in wheres for key in case.value(at)}
        listed = set(branch.kw.get("properties", {}))
        assert listed <= fields, f"{where}: Variant {variant} lists unobserved fields {sorted(listed - fields)}"

    # oneOf iff every pair of emitted branches is disjoint (5.5).
    disjoint = pairwise_disjoint(branches, validator=VALIDATOR)
    assert (keyword == ONE_OF) == disjoint, f"{where}: {keyword} with pairwise_disjoint = {disjoint}"
    documents = [branch.to_json() for branch in branches]
    if not any(isinstance(doc, dict) and set(doc) & (OBJECT_KEYWORDS - RECORD_KEYWORDS) for doc in documents):
        reasons = [
            disjoint_reason(documents[i], documents[j])
            for i in range(len(documents))
            for j in range(i + 1, len(documents))
        ]
        assert all(reasons) == disjoint, f"{where}: the 5.5 reading gives {reasons}, pairwise_disjoint {disjoint}"
        for reason in reasons:
            event(f"branch pair: {reason or 'overlapping'}")
    if keyword == ONE_OF:
        for at in everything:
            matching = [p for p, doc in enumerate(documents) if validates(doc, case.value(at))]
            assert len(matching) <= 1, f"{where}: object {at} matches oneOf branches {matching}"
    event(f"combinator {keyword}")
    note_path(outcome)
    return node.nid


def check_merged(case: Case, locator: Locator, model: SchemaModel, emitted: VariantNode) -> None:
    """5.6 at one Path whose candidate Variants were merged into one Variant."""
    outcome = emitted.outcome
    where = f"merge at {render_path(outcome.path)}"
    node = emitted.node
    candidates = [locator.rows(view) for view in outcome.variants.views()]
    assert len(candidates) >= 2 and outcome.n_variants == 1, where
    everything = [at for wheres in candidates for at in wheres]
    assert Counter(everything) == Counter(locator.rows(outcome.variants.signatures.objects)), where
    check_objects(case, everything, where)

    # The merged schema is emitted: the node itself, or the mixed node its keywords moved to.
    properties = node.kw.get("properties")
    nodes = model.index()
    assert node.nid in nodes or any(
        other.kw.get("properties") is properties for other in nodes.values()
    ), f"{where}: the merged Variant's schema is not in the emitted schema"
    assert node.kw.get("type") == ["object"], f"{where}: type {node.kw.get('type')}"

    keys = [set(case.value(at)) for at in everything]
    fields = set().union(*keys)
    required = set.intersection(*keys)
    listed = properties or {}
    assert set(listed) <= fields, f"{where}: properties {sorted(listed)} beyond the fields {sorted(fields)}"
    additional = node.kw.get("additionalProperties")
    if "patternProperties" not in node.kw and (additional is None or additional.boolean is False):
        # A plain Record lists every field (a hybrid Record/Map partition, task 37.1, would list only its Record part).
        assert set(listed) == fields, f"{where}: properties {sorted(listed)}, union of fields {sorted(fields)}"
    assert set(node.kw.get("required", [])) == required, (
        f"{where}: required {node.kw.get('required')}, intersection {sorted(required)}"
    )
    for name in sorted(listed):
        holders = [(d, steps + (name,)) for d, steps in everything if name in case.value((d, steps))]
        observed = [case.value(member) for member in holders if case.live(member)]  # flagged values add no type
        sub = listed[name].to_json()
        if not observed:
            assert sub == {}, f"{where}: field {name!r} has only flagged values but subschema {sub}"
            continue
        expected = type_names(observed)
        got = accepted_types(sub)
        assert got == expected, f"{where}: field {name!r} accepts {sorted(got)}, observed {sorted(expected)}"
    event("merged Variants")
    note_path(outcome)


@pytest.mark.parametrize("mode", MODES)
@given(case=cases(), setup=setups())
def test_variant_branches_follow_5_4_to_5_6(mode: str, case: Case, setup: Setup) -> None:
    store, locator = build(case)
    sa, variants = analyzer(setup, mode)
    model = sa.infer_model(View.root(store))
    hosts = combinator_hosts(model)

    checked: set[int] = set()
    for emitted in variants.emitted:
        if emitted.keyword is not None:
            host = check_split(case, locator, model, hosts, emitted)
            assert host not in checked, f"two splits share the combinator node {host}"
            checked.add(host)
        elif emitted.outcome.merged:
            check_merged(case, locator, model, emitted)

    # Every combinator in the schema with Variant branch records is a split the analyzer emitted.
    recorded = {
        cid.nid
        for cid, rec in model.provenance.items()
        if cid.keyword in COMBINATORS and "variant" in rec.details
    }
    assert recorded == checked, f"Variant combinators {sorted(recorded)}, recorded splits {sorted(checked)}"

    if not checked and not any(e.outcome.merged for e in variants.emitted):
        event("no Path with two or more candidate Variants")
    if case.flagged_docs:
        event("flagged documents")
    if case.flagged_items:
        event("flagged items")
