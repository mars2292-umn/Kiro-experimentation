"""Property 18: Required means always present (task 16.6).

For any Variant, a field is in ``required`` if and only if it is present
(with any value, including ``null``) in every non-flagged object of the
Variant. This set equals the Variant's Base_Fields.

The test runs :meth:`StructureAnalyzer.infer_model` on generated documents
with flags and checks every Variant at every object Path of the emitted
Schema_Model against a reference built from the raw documents.

Reference
    Every node of every document has a location ``(document index, step,
    …)``, where a step is a key (str) or an array position (int). A node is
    non-flagged when its document is not flagged and neither it nor any
    ancestor is a Flagged_Item. The reference reads presence from the raw
    Python objects, not from the ObservationStore: an object's fields are
    its keys, whatever their values. A ``null`` value counts, and so does a
    member whose value is a Flagged_Item, because flagging the value
    excludes only its subtree, not its key from the parent (17.4). The
    reference ``required`` set of a Variant is the intersection of its
    objects' key sets.

    Store rows are matched to locations by following each row's ``parent``
    link up to the document row and reading its ``key`` or ``pos`` field
    (:func:`store_locations`). The match is only used to identify objects;
    the test checks that it covers every raw node exactly once.

Matching branches to Variants
    The walk goes down the emitted schema from the root, carrying the
    non-flagged raw instances that reach each subschema. They follow
    ``properties`` (the values of that key), ``additionalProperties`` of a
    Map (every member value), ``items`` (every element), ``prefixItems``
    (the elements at that position), and the branches of the analyzer's
    mixed-type rule (objects and arrays go to the branch whose ``type``
    lists their JSON type).

    A schema object that holds the object keywords of its instances is
    either a Map (keys present, no ``properties``) or the Record of a single
    final Variant. A combinator node whose branches include two or more
    object branches is a split Record. Its instances can satisfy several
    ``anyOf`` branches, so validation cannot assign them to branches.
    Instead the test builds the view of exactly those objects (the Path's
    view, restricted by location) and asks the run's own VariantAnalyzer
    for the final Variants: ``merge_or_split(select(view)).views()``. The
    analyzer emits branch ``i`` from ``views()[i]`` (``variant_branches``),
    and the Variant results depend only on the view's rows. At a node with
    one object schema, the same call must give one final Variant. The flags
    are checked to be unchanged by the walk, so the second call sees the
    same rows.

Inputs
    Documents come from a random template: Records with one to three
    Variants of disjoint key sets (plus maybe a key shared by all), each
    field always or sometimes present, and any member value ``null`` in
    about one draw in five. Templates also include keyless objects, Maps
    with data-like keys (``u17``), Lists and Tuples, and mixed-type values,
    nested up to three object levels below the root. Records therefore sit
    at the root, in Record properties, in Map values, in List and Tuple
    elements, in Variant branches, and beside scalars and arrays. Keys
    include the empty key and astral and non-ASCII keys
    (:data:`tests.strategies.KEYS`).

    Flags: up to two spoiler documents are always flagged; they come from
    the same template but may drop fields that are otherwise always
    present, so counting them would shrink ``required``. One more document
    may be flagged, and up to two objects and up to two member values of
    objects (any type) are flagged as items.

    Technique states come from ``run_configs()`` (implemented Techniques
    only), with small Resource_Parameters and a stub Model_Bundle of
    hand-set Learned_Thresholds. Each Technique's thresholds are present
    together or absent, so the grouping Techniques, co-occurrence mining,
    and the fallbacks all occur. In about half the draws ``decide.mdl`` is
    off, so a grouping with two or more Variants stays split (5.11) and
    split Records are common even in small collections.

Checks
    For every Variant (single or a split branch):

    - ``required`` lists exactly the reference set, with no repeats, and is
      omitted when the set is empty (keyless Records, or no field shared by
      every object);
    - :attr:`Presence.base_fields` and :func:`required_fields` of the
      Variant's view equal the reference set (the Variant's Base_Fields);
    - each entry has one Provenance_Record naming ``baseline:required``,
      with Support ``(n, n)`` over the Variant's ``n`` non-flagged objects,
      the Laplace Confidence ``(n + 1) / (n + 2)``, and no fallback or
      resolution;
    - at a split Record, :func:`family_base_fields` of the final Variants
      equals the intersection of the Variants' reference sets, and each of
      those fields is in ``required`` in every branch (6.5).

    A Map has no ``required`` (4.3). Every non-flagged object of every
    document reaches exactly one Record or Map. No other object schema
    holds ``required``, and every ``required`` record belongs to a checked
    entry.

    Not covered yet: when Discriminator value groups replace the Variant
    branches (task 34.1), branch routing must follow the Discriminator, and
    this walk must be extended then.

**Validates: Requirements 6.1, 9.1, 9.2, 9.7**
"""

# Feature: json-schema-inference, Property 18: Required means always present

from __future__ import annotations

from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any

import numpy as np
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.observations import ObservationStore, ShapePath, build_store
from jsinfer.core.paths import ELEM, ELEM_STEP, MAPVAL_STEP, ROOT, Path, key_step, render_path
from jsinfer.core.provenance import Support, laplace_confidence
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.deps.presence import REQUIRED_PRODUCER, Presence, family_base_fields, required_fields
from jsinfer.io.validator import Validator
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.record_map import object_rows
from jsinfer.structure.variants import (
    ANY_OF,
    DBSCAN_TECHNIQUE as DBSCAN,
    GROUPING_TECHNIQUES,
    MINHASH_LSH,
    ONE_OF,
    MergeSplitOutcome,
)
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, run_configs

# One Validator for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
MDL = "decide.mdl"
SHARED_DEFS = "structure.shared_defs"
RECURSION = "structure.recursion"

Loc = tuple[Any, ...]  # (document index, key or position, …)
OBJECT = "object"
ARRAY = "array"
REQUIRED = "required"

# ---------------------------------------------------------------------------
# Templates and documents
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Template:
    """A generator of JSON values.

    ``record``: ``variants`` holds, per Variant, its fields as ``(key,
    always, child)``. ``map``, ``list``: ``children`` is the value template.
    ``tuple``: one template per position. ``mix``: the alternatives.
    """

    kind: str
    variants: tuple[tuple[tuple[str, bool, Template], ...], ...] = ()
    children: tuple[Template, ...] = ()


SCALAR = Template("scalar")
KEYLESS = Template("record", variants=((),))

SCALAR_VALUES = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(-3, 3),
    st.sampled_from([1.0, 2.5, -0.0, 2**70]),
    st.text("ab😀", max_size=3),
)
# Data-like Map keys, mostly distinct, so description length often prefers a Map; plus the shared alphabet.
MAP_KEYS = st.one_of(st.integers(0, 999).map(lambda i: f"u{i}"), st.sampled_from(KEYS))

# Container kinds are allowed while depth > 0; repetition sets the weights.
_KINDS = ("scalar", "scalar", "scalar", "record", "record", "keyless", "map", "list", "tuple", "mix")
_LEAF_KINDS = ("scalar", "scalar", "scalar", "keyless")


def child_templates(depth: int) -> st.SearchStrategy[Template]:
    """Templates of member values and elements one level down."""
    return templates(depth - 1) if depth > 0 else st.just(SCALAR)


@st.composite
def record_templates(draw: st.DrawFn, depth: int) -> Template:
    """One to three Variants with disjoint key sets, maybe plus one key in every Variant."""
    n_variants = draw(st.sampled_from([1, 2, 2, 3]), label="variants")
    order = draw(st.permutations(KEYS), label="key order")
    shared = order[:1] if n_variants > 1 and draw(st.integers(0, 3), label="a shared key") != 0 else ()
    rest = order[len(shared) :]
    variants: list[tuple[tuple[str, bool, Template], ...]] = []
    start = 0
    for _ in range(n_variants):
        size = draw(st.integers(1 if n_variants > 1 else 0, 3), label="own keys")
        own = rest[start : start + size]
        start += size
        fields = []
        for key in (*shared, *own):
            # A shared key is usually always present, so families often have Base_Fields (6.5).
            always = draw(st.integers(0, 3 if key in shared else 2), label=f"{key!r} always") != 0
            fields.append((key, always, draw(child_templates(depth), label=f"{key!r} values")))
        variants.append(tuple(fields))
    return Template("record", variants=tuple(variants))


@st.composite
def templates(draw: st.DrawFn, depth: int, mix: bool = True) -> Template:
    kinds = _KINDS if depth > 0 else _LEAF_KINDS
    kind = draw(st.sampled_from(kinds if mix else [k for k in kinds if k != "mix"]), label="kind")
    if kind == "scalar":
        return SCALAR
    if kind == "keyless":
        return KEYLESS
    if kind == "record":
        return draw(record_templates(depth))
    if kind in ("map", "list"):
        return Template(kind, children=(draw(child_templates(depth)),))
    if kind == "tuple":
        positions = draw(st.lists(child_templates(depth), min_size=1, max_size=3), label="positions")
        return Template("tuple", children=tuple(positions))
    assert kind == "mix"
    options = draw(st.lists(templates(depth, mix=False), min_size=2, max_size=3), label="alternatives")
    return Template("mix", children=tuple(options))


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    """A template whose documents hold objects: a Record, a Record among other types, or an array of Records."""
    record = draw(record_templates(2), label="root record")
    shape = draw(st.sampled_from(["record", "record", "record", "mixed", "array"]), label="root shape")
    if shape == "record":
        return record
    if shape == "mixed":
        return Template("mix", children=(record, SCALAR, draw(templates(1, mix=False), label="other root")))
    return Template("list", children=(record,))


@st.composite
def instances(draw: st.DrawFn, template: Template, spoil: bool = False) -> Any:
    """A value of ``template``. A spoiler (``spoil``) may drop fields that are otherwise always present."""
    kind = template.kind
    if kind == "scalar":
        return draw(SCALAR_VALUES)
    if kind == "record":
        fields = template.variants[draw(st.integers(0, len(template.variants) - 1), label="variant")]
        out: dict[str, Any] = {}
        for key, always, child in fields:
            if (always and not spoil) or draw(st.booleans(), label=f"include {key!r}"):
                null = draw(st.integers(0, 4), label=f"{key!r} null") == 0
                out[key] = None if null else draw(instances(child, spoil))
        return out
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(MAP_KEYS, max_size=4), label="map keys"))
        return {key: draw(instances(template.children[0], spoil)) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0], spoil), max_size=3), label="elements")
    if kind == "tuple":
        length = draw(st.integers(0, len(template.children)), label="tuple length")
        return [draw(instances(child, spoil)) for child in template.children[:length]]
    assert kind == "mix"
    return draw(instances(template.children[draw(st.integers(0, len(template.children) - 1))], spoil))


def raw_nodes(docs: Sequence[Any]) -> list[tuple[Loc, Any, bool]]:
    """Every node of the documents as ``(location, value, parent is an object)``, flags ignored, in pre-order."""
    out: list[tuple[Loc, Any, bool]] = []

    def visit(loc: Loc, value: Any, in_object: bool) -> None:
        out.append((loc, value, in_object))
        if isinstance(value, dict):
            for key, child in value.items():
                visit((*loc, key), child, True)
        elif isinstance(value, list):
            for i, child in enumerate(value):
                visit((*loc, i), child, False)

    for d, doc in enumerate(docs):
        visit((d,), doc, False)
    return out


@dataclass(frozen=True)
class Case:
    """Documents and their flags: flagged document indices and Flagged_Item locations."""

    docs: tuple[Any, ...]
    flagged_docs: frozenset[int]
    flagged_items: frozenset[Loc]

    def excluded(self, loc: Loc) -> bool:
        """Whether the node at ``loc`` is in a flagged document, or is or lies under a Flagged_Item."""
        return loc[0] in self.flagged_docs or any(loc[:i] in self.flagged_items for i in range(2, len(loc) + 1))

    def live_objects(self) -> list[Loc]:
        """Every non-flagged object of every document (the reference for coverage)."""
        return [loc for loc, value, _ in raw_nodes(self.docs) if isinstance(value, dict) and not self.excluded(loc)]


@st.composite
def cases(draw: st.DrawFn) -> Case:
    root = draw(root_templates(), label="root template")
    clean = [draw(instances(root), label="document") for _ in range(draw(st.integers(4, 24), label="documents"))]
    spoilers = draw(st.lists(instances(root, spoil=True), max_size=2), label="spoilers")
    units = [(doc, False) for doc in clean] + [(doc, True) for doc in spoilers]
    units = [units[i] for i in draw(st.permutations(range(len(units))), label="document order")]
    docs = tuple(doc for doc, _ in units)
    extra = draw(st.sets(st.integers(0, len(docs) - 1), max_size=1), label="flagged clean document")
    flagged_docs = frozenset(i for i, (_, spoiler) in enumerate(units) if spoiler) | extra

    nodes = [(loc, value, in_object) for loc, value, in_object in raw_nodes(docs) if len(loc) > 1]
    nodes = [node for node in nodes if node[0][0] not in flagged_docs]
    objects = [loc for loc, value, _ in nodes if isinstance(value, dict)]
    members = [loc for loc, _, in_object in nodes if in_object]
    flagged: set[Loc] = set()
    if objects:
        flagged |= draw(st.sets(st.sampled_from(objects), max_size=2), label="flagged objects")
    if members:
        flagged |= draw(st.sets(st.sampled_from(members), max_size=2), label="flagged member values")
    return Case(docs, flagged_docs, frozenset(flagged))


# ---------------------------------------------------------------------------
# The store, its flags, and row locations
# ---------------------------------------------------------------------------


def store_locations(store: ObservationStore) -> dict[ShapePath, list[Loc]]:
    """The location of every row of every column, found by following ``parent`` links to the document row."""
    out: dict[ShapePath, list[Loc]] = {}
    for shape in sorted(store.columns, key=len):
        column = store.columns[shape]
        if not shape:
            out[shape] = [(row,) for row in range(len(column))]  # root row i is document i
            continue
        parents = out[shape[:-1]]
        if shape[-1] == MAPVAL_STEP:
            steps: list[Any] = [store.key_name(int(kid)) for kid in column.key.tolist()]
        else:
            steps = column.pos.tolist()
        out[shape] = [(*parents[parent], step) for parent, step in zip(column.parent.tolist(), steps)]
    return out


def build(case: Case) -> tuple[ObservationStore, dict[ShapePath, list[Loc]]]:
    """A fresh store of the documents with the case's flags, and its row locations."""
    store = build_store(list(case.docs))
    locations = store_locations(store)
    where = {loc: (shape, row) for shape, locs in locations.items() for row, loc in enumerate(locs)}
    raw = [loc for loc, _, _ in raw_nodes(case.docs)]
    assert len(where) == len(raw) == sum(len(locs) for locs in locations.values()), "rows and raw nodes differ"
    assert set(where) == set(raw), "row locations differ from the raw documents' locations"
    for loc in sorted(case.flagged_items, key=repr):
        shape, row = where[loc]
        store.flags.flag_items(shape, [row])
    if case.flagged_docs:
        store.flags.flag_documents(sorted(case.flagged_docs))
    return store, locations


def flag_state(store: ObservationStore) -> tuple[Any, ...]:
    flags = store.flags
    return flags.epoch, flags.documents.tolist(), [(column, rows.tolist()) for column, rows in flags.items()]


# ---------------------------------------------------------------------------
# Technique states, Resource_Parameters, and the stub Model_Bundle
# ---------------------------------------------------------------------------

# Hand-set Learned_Thresholds inside their admissible ranges, with the extremes common.
THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    "lsh.keyset_jaccard": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
    "dbscan.eps": st.one_of(st.sampled_from([1e-9, 0.34, 0.5, 1.0]), st.floats(0.0, 1.0, exclude_min=True)),
    "dbscan.min_samples": st.integers(1, 6),
    "hdbscan.min_cluster_size": st.integers(2, 6),
    "cooccur.min_support": st.integers(1, 4),
    "cooccur.min_confidence": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
    "tagged.branch_support": st.one_of(st.sampled_from([0.0, 0.1, 0.3, 1.0]), st.floats(0.0, 1.0)),
}
THRESHOLD_TECHNIQUES = tuple(
    tid for tid in REGISTRY.ids() if any(name in THRESHOLD_VALUES for name in REGISTRY.technique(tid).thresholds)
)


@st.composite
def resource_values(draw: st.DrawFn) -> dict[str, int | float]:
    """The Resource_Parameters the grouping Techniques read, kept small so MinHash and EM stay fast."""
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    return {
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.dbscan"),
        "sample.hdbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(1, 6), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
    }


def _tagged_union(branches: Sequence[SchemaNode]) -> bool:
    """Whether the branches fix one common property with const/enum/not (a Discriminator's value groups, 10.4)."""
    properties = [b.kw.get("properties", {}) for b in branches]
    common = set.intersection(*(set(p) for p in properties)) if properties else set()
    return any(all(isinstance(p[name], SchemaNode) and _fixes_value(p[name]) for p in properties) for name in common)


def _fixes_value(sub: SchemaNode) -> bool:
    """A const, an enum, or the fallback's ``not: {enum: …}``; a ``not`` over ``required`` is an Exclusion_Rule (11)."""
    if sub.boolean is not None:
        return False
    if {"const", "enum"} & set(sub.kw):
        return True
    negated = sub.kw.get("not")
    return isinstance(negated, SchemaNode) and negated.boolean is None and "enum" in negated.kw


class StubBundle:
    """A Model_Bundle of hand-set Learned_Thresholds by name; no Decision_Models."""

    def __init__(self, thresholds: Mapping[str, int | float]) -> None:
        self.thresholds = dict(thresholds)

    def model(self, name: str) -> Any | None:
        return None

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


SCENARIOS = ("drawn", "priority", "dbscan first")


@dataclass(frozen=True)
class Setup:
    scenario: str
    config: RunConfiguration
    thresholds: Mapping[str, int | float]


@st.composite
def setups(draw: st.DrawFn) -> Setup:
    """Technique states from ``run_configs()``, small resources, and thresholds present per Technique or absent.

    Scenarios: ``drawn`` varies every implemented Technique. ``priority`` turns ``decide.mdl`` off, so
    Judgment_Calls resolve by priority and a grouping with two or more Variants stays split (5.11).
    ``dbscan first`` also takes LSH out and gives DBSCAN a small radius with ``min_samples`` 1, so DBSCAN's
    grouping (about one Variant per signature or per close signatures) is the one selected. The last two
    make split Records common even in small collections.
    """
    scenario = draw(st.sampled_from(SCENARIOS), label="scenario")
    base: dict[str, Any] = dict(draw(resource_values(), label="resources"))
    if scenario != "drawn":
        base[technique_setting(MDL)] = False
    if scenario == "dbscan first":
        base[technique_setting(MINHASH_LSH)] = False
        base[technique_setting(DBSCAN)] = True
    base[technique_setting(SHARED_DEFS)] = False  # Path-by-Path walks: shared definitions are Property 16's
    base[technique_setting(RECURSION)] = False
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    if scenario == "dbscan first":
        present[DBSCAN] = True
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    if scenario == "dbscan first":
        thresholds["dbscan.eps"] = draw(st.sampled_from([1e-9, 0.34, 0.5]), label="small dbscan.eps")
        thresholds["dbscan.min_samples"] = 1
    return Setup(scenario, config, thresholds)


# ---------------------------------------------------------------------------
# The reference walk
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Inst:
    """A non-flagged raw node that reaches a subschema."""

    loc: Loc
    value: Any


def _types(node: SchemaNode) -> list[str]:
    listed = node.kw.get("type", []) if node.boolean is None else []
    return list(listed) if isinstance(listed, list) else [listed]


@dataclass
class Checker:
    """Walks the emitted Schema_Model with the raw instances that reach each subschema (see the module docstring)."""

    case: Case
    store: ObservationStore
    locations: Mapping[ShapePath, list[Loc]]
    analyzer: StructureAnalyzer
    model: SchemaModel
    # Checked Record nodes (by nid) → their reference required set; objects that reached a Record or a Map.
    records: dict[int, frozenset[str]] = field(default_factory=dict)
    reached: list[Loc] = field(default_factory=list)
    notes: set[str] = field(default_factory=set)

    # -- raw instances --------------------------------------------------------

    def roots(self) -> list[Inst]:
        return [Inst((d,), doc) for d, doc in enumerate(self.case.docs) if not self.case.excluded((d,))]

    def members(self, objects: Sequence[Inst], key: str) -> list[Inst]:
        """The non-flagged values of ``key`` in ``objects`` (an object holding a flagged value still has the key)."""
        flagged = self.case.flagged_items
        return [
            Inst((*o.loc, key), o.value[key]) for o in objects if key in o.value and (*o.loc, key) not in flagged
        ]

    def all_members(self, objects: Sequence[Inst]) -> list[Inst]:
        flagged = self.case.flagged_items
        return [Inst((*o.loc, k), v) for o in objects for k, v in o.value.items() if (*o.loc, k) not in flagged]

    def elements(self, arrays: Sequence[Inst]) -> list[Inst]:
        flagged = self.case.flagged_items
        return [Inst((*a.loc, i), v) for a in arrays for i, v in enumerate(a.value) if (*a.loc, i) not in flagged]

    # -- views ----------------------------------------------------------------

    def row_locations(self, view: View) -> list[Loc]:
        """The locations of the view's rows, in view order."""
        return [self.locations[part.column][row] for part in view.parts for row in part.rows.tolist()]

    def final_variants(self, objects: Sequence[Inst], path: Path) -> MergeSplitOutcome:
        """The run's final Variants of exactly ``objects`` at ``path``, from the view of those objects."""
        view = object_rows(View.root(self.store).descend(path))
        wanted = {o.loc for o in objects}
        mask = np.array([loc in wanted for loc in self.row_locations(view)], dtype=bool)
        assert int(mask.sum()) == len(wanted) == len(objects), f"{render_path(path)}: objects missing from the view"
        variants = self.analyzer.variants
        return variants.merge_or_split(variants.select(view.subset(mask)))

    # -- the walk -------------------------------------------------------------

    def walk(self, node: SchemaNode, insts: Sequence[Inst], path: Path) -> None:
        where = render_path(path)
        if not insts:
            assert node.boolean is None and not node.kw, f"{where}: nothing observed, but the subschema is {node.to_json()}"
            return
        assert node.boolean is None, f"{where}: a boolean subschema for observed instances"
        objects = [i for i in insts if isinstance(i.value, dict)]
        arrays = [i for i in insts if isinstance(i.value, list)]
        if "type" in node.kw:
            self.typed(node, objects, arrays, path)
            return

        combinator = ANY_OF if ANY_OF in node.kw else ONE_OF
        assert combinator in node.kw, f"{where}: neither 'type' nor a combinator in {sorted(node.kw)}"
        branches: list[SchemaNode] = node.kw[combinator]
        holders = [b for b in branches if OBJECT in _types(b)]
        rest = branches
        if len(holders) >= 2:
            # A split Record: its Variant branches come first, then any branch for the other types.
            assert all(a is b for a, b in zip(branches, holders)), f"{where}: Variant branches are not first"
            self.split(combinator, holders, objects, path)
            rest = branches[len(holders) :]
            objects = []
            if len(rest):
                self.notes.add("split Record beside other types")
        else:
            assert combinator == ANY_OF, f"{where}: a {combinator} without Variant branches"
        for kind, insts_of_kind in ((OBJECT, objects), (ARRAY, arrays)):
            owners = [b for b in rest if kind in _types(b)]
            assert len(owners) == (1 if insts_of_kind else 0), f"{where}: {len(owners)} branches hold the {kind}s"
        for branch in rest:
            types = _types(branch)
            self.typed(branch, objects if OBJECT in types else [], arrays if ARRAY in types else [], path)

    def typed(self, node: SchemaNode, objects: Sequence[Inst], arrays: Sequence[Inst], path: Path) -> None:
        where = render_path(path)
        types = _types(node)
        assert (OBJECT in types) == bool(objects), f"{where}: type {types} for {len(objects)} objects"
        assert (ARRAY in types) == bool(arrays), f"{where}: type {types} for {len(arrays)} arrays"
        if objects and len(types) > 1:
            self.notes.add("objects beside other types in one schema object")
        if objects:
            if any(o.value for o in objects) and "properties" not in node.kw:
                self.map(node, objects, path)
            else:
                outcome = self.final_variants(objects, path)
                assert outcome.n_variants == 1, f"{where}: {outcome.n_variants} final Variants but one object schema"
                self.notes.add("candidate Variants merged" if outcome.merged else "single-Variant Record")
                self.record(node, objects, outcome.views()[0], path)
        if arrays:
            self.array(node, arrays, path)

    def split(self, combinator: str, branches: Sequence[SchemaNode], objects: Sequence[Inst], path: Path) -> None:
        """A split Record: assign the objects to branches by the final Variants' views, then check each branch."""
        where = render_path(path)
        outcome = self.final_variants(objects, path)
        views = outcome.views()
        by_loc = {o.loc: o for o in objects}
        if _tagged_union(branches):
            # A Discriminator's value groups replaced the Variants (10): every object validates against exactly one
            # branch (10.7), which assigns it; the branch views are the objects of each group.
            compiled = [VALIDATOR.compile(branch.to_json()) for branch in branches]
            groups = [[o for o in objects if VALIDATOR.is_valid(c, o.value)] for c in compiled]
            assert Counter(o.loc for g in groups for o in g) == Counter(o.loc for o in objects), f"{where}: not exactly one branch"
            parent = object_rows(View.root(self.store).descend(path))
            locations = self.row_locations(parent)
            views = tuple(parent.subset(np.array([loc in {o.loc for o in g} for loc in locations], dtype=bool)) for g in groups)
            self.notes.add("Tagged_Union branches")
        else:
            assert len(views) == len(branches), f"{where}: {len(views)} final Variants for {len(branches)} branches"
            groups = [[by_loc[loc] for loc in self.row_locations(view)] for view in views]
        assigned = Counter(o.loc for group in groups for o in group)
        assert assigned == Counter(o.loc for o in objects), f"{where}: the final Variants do not partition the objects"

        # 6.1 and 6.5: the family's Base_Fields, and every one of them in every branch's required list.
        always = [frozenset.intersection(*(frozenset(o.value) for o in group)) for group in groups]
        family = frozenset.intersection(*always)
        assert family_base_fields(views) == tuple(sorted(family)), f"{where}: family Base_Fields differ"
        for i, (branch, group, view) in enumerate(zip(branches, groups, views)):
            assert _types(branch) == [OBJECT], f"{where}: Variant branch {i} has type {_types(branch)}"
            self.record(branch, group, view, path, f" (Variant branch {i})")
            listed = set(branch.kw.get(REQUIRED, []))
            assert family <= listed, f"{where}: family Base_Fields {sorted(family - listed)} missing in branch {i}"
        self.notes.add(f"split Record ({combinator})")
        self.notes.add("split Record with 2 Variants" if len(views) == 2 else "split Record with 3 or more Variants")
        if path:
            self.notes.add("split Record below the root")
        if family:
            self.notes.add("family Base_Fields in every branch")

    def record(self, node: SchemaNode, objects: Sequence[Inst], view: View, path: Path, label: str = "") -> None:
        """One Variant: ``required`` equals the fields every object holds, with ``baseline:required`` records."""
        where = render_path(path) + label
        assert Counter(self.row_locations(view)) == Counter(o.loc for o in objects), f"{where}: view ≠ objects"
        n = len(objects)
        key_sets = [frozenset(o.value) for o in objects]
        always = frozenset.intersection(*key_sets)
        names = tuple(sorted(always))

        # 9.1, 9.2, 9.7: required lists exactly the fields present in every object, null or flagged values included.
        listed = node.kw.get(REQUIRED)
        if always:
            assert isinstance(listed, list), f"{where}: no 'required' for fields {list(names)}"
            assert len(listed) == len(set(listed)), f"{where}: 'required' repeats a field: {listed}"
            assert set(listed) == always, f"{where}: required {sorted(listed)} ≠ always present {list(names)}"
        else:
            assert REQUIRED not in node.kw, f"{where}: 'required' {listed} although no field is always present"

        # 6.1: the set equals the Variant's Base_Fields.
        presence = Presence.of(view)
        assert presence.n_objects == n
        assert presence.base_fields == presence.required == names, f"{where}: Base_Fields {presence.base_fields}"
        assert required_fields(view).names == names

        # 26.13 and the design's Open Decisions: Support over the Variant's own non-flagged objects.
        expected = laplace_confidence(Support(n, n))
        assert expected == (n + 1) / (n + 2)
        for name in names:
            record = self.model.provenance.get(ConstraintId(node.nid, REQUIRED, name))
            assert record is not None, f"{where}: no Provenance_Record for required {name!r}"
            assert record.technique == REQUIRED_PRODUCER, f"{where}: {name!r} is produced by {record.technique}"
            assert (record.support.consistent, record.support.total) == (n, n), f"{where}: {name!r} {record.support}"
            assert record.confidence == expected, f"{where}: {name!r} Confidence {record.confidence} ≠ {expected}"
            assert record.fallback is None and record.resolution is None, f"{where}: {name!r} is not a Baseline_Rule"

        assert node.nid not in self.records, f"{where}: one schema object checked as two Variants"
        self.records[node.nid] = always
        self.reached.extend(o.loc for o in objects)
        self.note_record(objects, always, path)

        properties: Mapping[str, SchemaNode] = node.kw.get("properties", {})
        keys = frozenset().union(*key_sets)
        assert set(properties) == keys, f"{where}: properties {sorted(properties)} ≠ observed keys {sorted(keys)}"
        for key in sorted(keys):
            self.walk(properties[key], self.members(objects, key), (*path, key_step(key)))

    def map(self, node: SchemaNode, objects: Sequence[Inst], path: Path) -> None:
        """A Map: no ``required`` (4.3), and one value schema over every non-flagged member value."""
        where = render_path(path)
        assert REQUIRED not in node.kw, f"{where}: a Map lists 'required'"
        values = node.kw.get("additionalProperties")
        assert isinstance(values, SchemaNode) and values.boolean is not False, f"{where}: a Map without a value schema"
        self.reached.extend(o.loc for o in objects)
        self.notes.add("Map")
        self.walk(values, self.all_members(objects), (*path, MAPVAL_STEP))

    def array(self, node: SchemaNode, arrays: Sequence[Inst], path: Path) -> None:
        where = render_path(path)
        elements = self.elements(arrays)
        child = (*path, ELEM_STEP)
        prefix = node.kw.get("prefixItems")
        items = node.kw.get("items")
        if prefix is not None:
            for position, schema in enumerate(prefix):
                self.walk(schema, [e for e in elements if e.loc[-1] == position], child)
            beyond = [e for e in elements if e.loc[-1] >= len(prefix)]
            if beyond:
                assert isinstance(items, SchemaNode) and items.boolean is None, f"{where}: elements beyond the prefix"
                self.walk(items, beyond, child)
        elif items is not None:
            self.walk(items, elements, child)
        else:
            assert not elements, f"{where}: {len(elements)} elements, but neither 'items' nor 'prefixItems'"

    # -- whole-model checks -----------------------------------------------------

    def check_model(self) -> None:
        """Every non-flagged object reached one Record or Map; no other object schema holds ``required``."""
        assert Counter(self.reached) == Counter(self.case.live_objects()), "objects reached ≠ non-flagged objects"
        for pointer, node in self.model.walk():
            if node.boolean is None and REQUIRED in node.kw and OBJECT in _types(node):
                assert node.nid in self.records, f"{pointer!r}: 'required' on an object schema that is no Variant"
        for cid in self.model.provenance:
            if cid.keyword == REQUIRED:
                assert cid.nid in self.records and cid.item in self.records[cid.nid], f"stray record {cid}"

    def note_record(self, objects: Sequence[Inst], always: frozenset[str], path: Path) -> None:
        if not any(o.value for o in objects):
            self.notes.add("keyless Record")
        elif not always:
            self.notes.add("Record with no always-present field")
        if any(o.value[name] is None for o in objects for name in always):
            self.notes.add("null-valued required field")
        flagged = self.case.flagged_items
        held = [(o, key) for o in objects for key in o.value if (*o.loc, key) in flagged]
        if held:
            self.notes.add("flagged member value kept its key")
            if any(key in always for _, key in held):
                self.notes.add("flagged member value of a required field")
        if path:
            self.notes.add("nested Record")
        if any(step.kind == ELEM for step in path):
            self.notes.add("Record in an array")

    def note_case(self) -> None:
        if self.case.flagged_docs:
            event("flagged documents")
        if any(isinstance(self._value_at(loc), dict) for loc in self.case.flagged_items):
            event("flagged objects")
        for note in sorted(self.notes):
            event(note)

    def _value_at(self, loc: Loc) -> Any:
        value = self.case.docs[loc[0]]
        for step in loc[1:]:
            value = value[step]
        return value


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@given(setup=setups(), case=cases())
def test_required_lists_exactly_the_fields_every_non_flagged_object_of_the_variant_holds(
    setup: Setup, case: Case
) -> None:
    store, locations = build(case)
    config = setup.config
    analyzer = StructureAnalyzer.for_techniques(
        config.techniques,
        bundle=StubBundle(setup.thresholds),
        report=InferenceReport(),
        validator=VALIDATOR,
        resources=config.resources,
        seed=config.seed,
    )
    before = flag_state(store)
    model = analyzer.infer_model(View.root(store))
    assert flag_state(store) == before, "the walk changed the flags, so the Variant views would differ"

    checker = Checker(case, store, locations, analyzer, model)
    checker.walk(model.root, checker.roots(), ROOT)
    checker.check_model()
    checker.note_case()
    event(f"scenario {setup.scenario}")
    if not config.techniques[MDL]:
        event("MDL selection disabled")
    if not any(config.techniques[tid] for tid in GROUPING_TECHNIQUES):
        event("every grouping Technique disabled")
