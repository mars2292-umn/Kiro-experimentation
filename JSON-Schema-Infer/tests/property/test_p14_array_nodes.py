"""Property 14: Array nodes match their classification (task 16.5).

For any array Path with at least one element: a List emits ``items``
inferred from all elements and no ``prefixItems``; a Tuple emits
``prefixItems`` of length equal to the maximum observed length, plus
``items: false`` when no array is longer; a Path with only empty arrays
emits neither keyword.

Inputs
    Documents follow a drawn spec. The ``free`` layouts are one or two
    object families, root arrays, or a mixed root of arrays, objects, and
    scalars. Field values are positional arrays (distinct scalar types per
    position, sometimes cut short), variable-length lists, always-empty
    arrays, lists of arrays, lists of objects with array fields, Paths that
    hold an array or another type, and arbitrary JSON values
    (:func:`tests.strategies.json_values`). The ``planted`` layouts use 12
    to 20 documents so that MDL resolves real structure: objects with a
    field of fixed-length positional rows (Tuples by MDL), or two families
    with disjoint keys, alternating, each with an array field (Variant
    branches holding arrays). Scalars include integral floats (``1.0``,
    ``1e3``, ``-0.0``, ``2**70``) for the integer rule. The test runs once
    per scenario and layout group, so each gets the profile's full example
    budget.

    Flags: up to two documents; up to two arrays below the root (with their
    subtrees); up to three elements; every element at one position of every
    array at one array Path (a whole Tuple position flagged); and every
    element of one array (an array holding only flagged elements, so its
    Path may observe no element). Flags are set on the ObservationStore by
    the rows of these locations and kept as locations for the reference.

The walk and the reference
    The Structure_Analyzer runs with Technique states from ``run_configs()``
    in three scenarios: ``drawn`` (states drawn), ``tupling`` (``decide.mdl``
    on, and the ArrayAnalyzer scores Tuple-or-List with a stub MDL_Scorer
    under which the Tuple always wins, so variable-length Tuples occur), and
    ``all off`` (every Technique disabled, so every Decision_Point takes its
    Fallback_Behavior). Its ArrayAnalyzer records every ``array_node`` call
    (the view and the ArrayNode), which locates each array Path of the walk:
    nested arrays, Tuple positions, arrays in objects, in Variant branches,
    and at mixed-type Paths.

    Locations are tuples of a document index followed by keys (``str``) and
    array indices (``int``), read from the raw documents. A view row's
    location follows the store's ``parent``, ``key``, and ``pos`` fields up
    to its document; that is the only thing the test takes from the
    ObservationStore. Everything else comes from the raw documents and the
    flagged locations:

    - Coverage: the arrays of the calls are exactly the non-flagged raw
      arrays (not in a flagged document, not flagged, no flagged ancestor),
      each in exactly one call.
    - Lengths: an array's length is its raw length, flagged elements
      included (design, Open Decisions, "Tuple positions and flagged
      elements"); its elements are the non-flagged ones. A Path observes an
      element when one of its arrays holds a non-flagged element.
    - Element views: ``array_rows(view).elements()`` and ``elements(i)``
      hold exactly the reference elements (all, or those at index ``i``).
    - Subschemas. A List's ``items`` and a Tuple's ``prefixItems[i]`` must
      (a) accept every reference element (at position ``i``) that has no
      flagged descendant, with ``format`` asserted, and (b) equal the root
      of a fresh analyzer's ``infer_model`` on the reference element view
      (``{}`` for a position whose elements are all flagged). With every
      Technique off they must instead equal a Naive_Merge computed here from
      the raw elements and the flagged locations (``reference_naive``), as
      Requirement 28.5 makes the walk Naive_Merge.

Checks per array Path (the emitted node in the final Schema_Model)
    - Elements observed and List: ``items`` and no ``prefixItems`` (7.2).
    - Elements observed and Tuple: ``prefixItems`` of length the maximum raw
      length (7.3) and ``items: false`` (7.4; the prefix spans every array).
    - No element observed (only empty arrays, or only flagged elements):
      neither keyword, and the ``type`` record notes
      ``"elements_observed": false`` (7.9).
    - The classification agrees in the ``items``/``prefixItems`` records
      (``details["judgment"]`` and the length statistics) and in the
      report's ``tuple_or_list`` entries (one per resolved call, none for
      Paths without elements). The scenario fixes it where it can: List
      with every Technique off or ``decide.mdl`` off, Tuple under the stub.
    - The final node holds the ArrayNode's keywords unchanged (the node
      itself, or the merged node of a mixed-type Path), every node whose
      ``type`` lists ``array`` belongs to exactly one call, and no other
      node holds ``items`` or ``prefixItems``.

**Validates: Requirements 7.2, 7.3, 7.4, 7.9**
"""

# Feature: json-schema-inference, Property 14: Array nodes match their classification

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from typing import Any

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.observations import ObservationStore, ShapePath, build_store
from jsinfer.core.paths import ELEM, ELEM_STEP, render_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.arrays import (
    LIST,
    TUPLE,
    TUPLE_OR_LIST,
    ArrayAnalyzer,
    ArrayNode,
    ArrayOutcome,
    ChildSchema,
    array_rows,
)
from jsinfer.values.inferrer import ValueInferrer
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_values, run_configs

# One Validator and MDL_Scorer for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
SCORER = MDLScorer(validator=VALIDATOR)
MDL = "decide.mdl"
SHARED_DEFS = "structure.shared_defs"
RECURSION = "structure.recursion"

Location = tuple[int | str, ...]  # (document index, then keys and array indices)


class TupleFavoringScorer(MDLScorer):
    """Scores the Tuple-or-List candidates by shape, so the Tuple (``prefixItems``) always wins under MDL."""

    __slots__ = ()

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        if "prefixItems" in node.kw:
            return Score(1.0, 1.0, 0.0, 0, 1)
        return Score(50.0, 50.0, 0.0, 0, 1)


TUPLING = TupleFavoringScorer(validator=VALIDATOR)


class RecordingArrayAnalyzer(ArrayAnalyzer):
    """An ArrayAnalyzer that records every ``array_node`` call of the walk: the view and the ArrayNode."""

    __slots__ = ("calls",)

    def __init__(self, values: ValueInferrer, *, scorer: MDLScorer | None = None) -> None:
        super().__init__(values, scorer=scorer)
        self.calls: list[tuple[View, ArrayNode]] = []

    def array_node(
        self, view: View, outcome: ArrayOutcome | None = None, *, child_schema: ChildSchema = naive_merge_node
    ) -> ArrayNode:
        result = super().array_node(view, outcome, child_schema=child_schema)
        self.calls.append((view, result))
        return result


# ---------------------------------------------------------------------------
# Document specs and documents
# ---------------------------------------------------------------------------

SCALARS: dict[str, st.SearchStrategy[Any]] = {
    "null": st.none(),
    "boolean": st.booleans(),
    "integer": st.one_of(st.integers(-3, 3), st.sampled_from([1.0, 1e3, -0.0, 2**70])),
    "number": st.sampled_from([2.5, -0.5, 0.1]),
    "string": st.text("ab😀", max_size=3),
}
SCALAR_KINDS = tuple(SCALARS)
# Positional types of a Tuple-like array, distinct so MDL prefers the Tuple when enough arrays are observed.
POSITION_KINDS = st.lists(st.sampled_from(SCALAR_KINDS), min_size=2, max_size=3, unique=True)
# The other alternative of a Path that holds an array or another type.
MIXED_OTHERS = (("scalar", ("string", "integer")), ("object", {"a": ("scalar", ("integer",))}, frozenset()))

# A spec is a plain tuple:
#   ("scalar", kinds)            a scalar of one of ``kinds``
#   ("tuple", kinds, short)      position i holds kinds[i]; with ``short``, sometimes a shorter prefix
#   ("list", spec, max_len)      0 … max_len elements drawn from ``spec``
#   ("empty",)                   always []
#   ("object", fields, optional) each key of ``fields`` with its spec; keys in ``optional`` may be missing
#   ("choice", specs)            one of ``specs``
#   ("any",)                     an arbitrary JSON value


@st.composite
def array_specs(draw: st.DrawFn, depth: int) -> tuple[Any, ...]:
    kinds = ["tuple", "tuple", "list", "list", "empty"] + (["nested", "records"] if depth > 0 else [])
    kind = draw(st.sampled_from(kinds), label="array kind")
    if kind == "tuple":
        positions = draw(POSITION_KINDS, label="positions")
        return ("tuple", tuple(positions), draw(st.booleans(), label="short arrays"))
    if kind == "list":
        types = draw(st.lists(st.sampled_from(SCALAR_KINDS), min_size=1, max_size=2, unique=True), label="types")
        return ("list", ("scalar", tuple(types)), draw(st.integers(1, 4), label="max length"))
    if kind == "empty":
        return ("empty",)
    if kind == "nested":
        return ("list", draw(array_specs(depth - 1)), draw(st.integers(1, 3), label="max length"))
    return ("list", draw(families(depth - 1)), draw(st.integers(1, 3), label="max length"))


@st.composite
def value_specs(draw: st.DrawFn, depth: int) -> tuple[Any, ...]:
    kinds = ["array", "array", "array", "scalar", "mixed", "any"] + (["object"] if depth > 0 else [])
    kind = draw(st.sampled_from(kinds), label="value kind")
    if kind == "array":
        return draw(array_specs(depth))
    if kind == "scalar":
        return ("scalar", (draw(st.sampled_from(SCALAR_KINDS)),))
    if kind == "mixed":
        return ("choice", (draw(array_specs(depth)), draw(st.sampled_from(MIXED_OTHERS))))
    if kind == "any":
        return ("any",)
    return draw(object_specs(depth - 1))


@st.composite
def object_specs(draw: st.DrawFn, depth: int, pool: Sequence[str] = KEYS, planted: bool = False) -> tuple[Any, ...]:
    """An object spec; a planted one has two or three keys, the first holding an array, and none optional."""
    low = min(2, len(pool)) if planted else 1
    keys = draw(st.lists(st.sampled_from(pool), min_size=low, max_size=3, unique=True), label="keys")
    fields = {
        key: draw(array_specs(depth) if planted and i == 0 else value_specs(depth), label=f"field {key!r}")
        for i, key in enumerate(keys)
    }
    if planted:
        return ("object", fields, frozenset())
    optional = frozenset(draw(st.sets(st.sampled_from(keys), max_size=1), label="optional key"))
    return ("object", fields, optional)


@st.composite
def families(draw: st.DrawFn, depth: int, planted: bool = False) -> tuple[Any, ...]:
    """One or two object families; two families have disjoint keys, so their objects may become Variants.

    ``planted`` forces two families of :func:`object_specs` with ``planted``.
    """
    if planted or draw(st.integers(0, 2), label="two families") != 0:
        split = draw(st.integers(2, len(KEYS) - 2) if planted else st.integers(1, len(KEYS) - 1), label="key split")
        pools = (KEYS[:split], KEYS[split:])
        return ("choice", tuple(draw(object_specs(depth, pool, planted)) for pool in pools))
    return ("choice", (draw(object_specs(depth)),))


@st.composite
def planted_rows(draw: st.DrawFn) -> tuple[Any, ...]:
    """Objects with a field of fixed-length rows whose positions hold distinct types: a Tuple by MDL."""
    positions = draw(POSITION_KINDS, label="positions")
    key, other = draw(st.lists(st.sampled_from(KEYS), min_size=2, max_size=2, unique=True), label="row keys")
    return ("object", {key: ("tuple", tuple(positions), False), other: draw(value_specs(1))}, frozenset({other}))


# Root layouts by group; each group is its own test run, so each gets the profile's full example budget.
# Planted layouts get larger collections, so MDL finds their Tuples and Variants.
LAYOUT_GROUPS = {
    "free": ("objects", "objects", "array", "mixed"),
    "planted": ("planted rows", "planted variants"),
}
PLANTED = frozenset(LAYOUT_GROUPS["planted"])


@st.composite
def root_specs(draw: st.DrawFn, group: str) -> tuple[str, tuple[Any, ...]]:
    layout = draw(st.sampled_from(LAYOUT_GROUPS[group]), label="root layout")
    if layout == "objects":
        return layout, draw(families(2))
    if layout == "array":
        return layout, draw(array_specs(2))
    if layout == "planted rows":
        return layout, draw(planted_rows())
    if layout == "planted variants":
        return layout, draw(families(1, planted=True))
    return layout, ("choice", (draw(array_specs(1)), draw(families(1)), ("scalar", ("string", "integer"))))


@st.composite
def instances(draw: st.DrawFn, spec: tuple[Any, ...]) -> Any:
    tag = spec[0]
    if tag == "scalar":
        return draw(SCALARS[draw(st.sampled_from(spec[1]))])
    if tag == "tuple":
        positions, short = spec[1], spec[2]
        n = draw(st.integers(0, len(positions))) if short and draw(st.booleans()) else len(positions)
        return [draw(SCALARS[kind]) for kind in positions[:n]]
    if tag == "list":
        return [draw(instances(spec[1])) for _ in range(draw(st.integers(0, spec[2])))]
    if tag == "empty":
        return []
    if tag == "object":
        fields, optional = spec[1], spec[2]
        return {
            key: draw(instances(field)) for key, field in fields.items() if key not in optional or draw(st.booleans())
        }
    if tag == "choice":
        return draw(instances(draw(st.sampled_from(spec[1]))))
    assert tag == "any", spec
    return draw(json_values(max_leaves=6))


# ---------------------------------------------------------------------------
# Raw locations and flags
# ---------------------------------------------------------------------------


def raw_walk(docs: Sequence[Any]) -> Iterator[tuple[Location, Any, bool]]:
    """Every value of the documents as ``(location, value, parent is an array)``, flags ignored."""
    stack: list[tuple[Location, Any, bool]] = [((d,), doc, False) for d, doc in enumerate(docs)]
    while stack:
        loc, value, in_array = stack.pop()
        yield loc, value, in_array
        if isinstance(value, dict):
            stack.extend((loc + (key,), child, False) for key, child in value.items())
        elif isinstance(value, list):
            stack.extend((loc + (i,), child, True) for i, child in enumerate(value))


def value_at(docs: Sequence[Any], loc: Location) -> Any:
    value = docs[loc[0]]  # type: ignore[index]
    for component in loc[1:]:
        value = value[component]
    return value


def logical(loc: Location) -> tuple[str | None, ...]:
    """The logical Path of a location: keys kept, array indices replaced by None (``[*]``)."""
    return tuple(None if isinstance(component, int) else component for component in loc[1:])


@dataclass(frozen=True)
class Case:
    docs: tuple[Any, ...]
    flagged_docs: frozenset[int]
    flagged_items: frozenset[Location]
    whole_position: bool  # every element at one position of one array Path was flagged
    emptied_array: bool  # every element of one array was flagged

    def excluded(self, loc: Location) -> bool:
        """Whether ``loc`` is in a flagged document, flagged, or under a flagged item."""
        if loc[0] in self.flagged_docs:
            return True
        return any(loc[:j] in self.flagged_items for j in range(2, len(loc) + 1))

    @property
    def dirty(self) -> frozenset[Location]:
        """Locations with a flagged descendant."""
        return frozenset(item[:j] for item in self.flagged_items for j in range(1, len(item)))


# Larger collections let MDL find Tuples and Variants; small ones keep few observations per Path.
COLLECTION_SIZES = (1, 3, 8, 12, 16, 20)
PLANTED_SIZES = (12, 16, 20)


@st.composite
def cases(draw: st.DrawFn, group: str) -> Case:
    layout, spec = draw(root_specs(group), label="spec")
    n = draw(st.sampled_from(PLANTED_SIZES if layout in PLANTED else COLLECTION_SIZES), label="documents")
    if layout == "planted variants":
        # Alternate the two families, so both have objects (a free choice per document favors the first).
        docs = tuple(draw(instances(spec[1][i % 2]), label=f"document {i}") for i in range(n))
    else:
        docs = tuple(draw(instances(spec), label=f"document {i}") for i in range(n))
    walked = list(raw_walk(docs))
    arrays = sorted((loc for loc, value, _ in walked if isinstance(value, list) and len(loc) > 1), key=repr)
    elements = sorted((loc for loc, _, in_array in walked if in_array), key=repr)

    flagged_docs: set[int] = set()
    items: set[Location] = set()
    if n > 1 and draw(st.integers(0, 3), label="flag documents") == 0:
        flagged_docs |= draw(st.sets(st.integers(0, n - 1), min_size=1, max_size=2), label="flagged documents")
    if arrays and draw(st.integers(0, 3), label="flag arrays") == 0:
        items |= draw(st.sets(st.sampled_from(arrays), min_size=1, max_size=2), label="flagged arrays")
    if elements and draw(st.integers(0, 2), label="flag elements") == 0:
        items |= draw(st.sets(st.sampled_from(elements), min_size=1, max_size=3), label="flagged elements")

    # Every element at one position of every array at one array Path.
    whole_position = False
    groups: dict[tuple[str | None, ...], list[Location]] = {}
    for loc, value, _ in walked:
        if isinstance(value, list) and value:
            groups.setdefault(logical(loc), []).append(loc)
    if groups and draw(st.integers(0, 3), label="flag a position") == 0:
        path = draw(st.sampled_from(sorted(groups, key=repr)), label="position Path")
        longest = max(len(value_at(docs, loc)) for loc in groups[path])
        position = draw(st.integers(0, longest - 1), label="flagged position")
        items |= {loc + (position,) for loc in groups[path] if len(value_at(docs, loc)) > position}
        whole_position = True

    # Every element of one array.
    emptied_array = False
    filled = sorted((loc for loc, value, _ in walked if isinstance(value, list) and value), key=repr)
    if filled and draw(st.integers(0, 3), label="empty an array") == 0:
        loc = draw(st.sampled_from(filled), label="emptied array")
        items |= {loc + (i,) for i in range(len(value_at(docs, loc)))}
        emptied_array = True
    return Case(docs, frozenset(flagged_docs), frozenset(items), whole_position, emptied_array)


# ---------------------------------------------------------------------------
# Store rows <-> locations
# ---------------------------------------------------------------------------


def row_locations(store: ObservationStore) -> dict[ShapePath, list[Location]]:
    """The location of every row of every column, from the ``parent``, ``key``, and ``pos`` fields."""
    out: dict[ShapePath, list[Location]] = {}
    for column in sorted(store.columns, key=len):
        col = store.columns[column]
        if not column:
            out[column] = [(int(d),) for d in col.doc.tolist()]
            continue
        parents = out[column[:-1]]
        if column[-1].kind == ELEM:
            components: list[int | str] = [int(p) for p in col.pos.tolist()]
        else:
            components = [store.key_name(int(k)) for k in col.key.tolist()]
        out[column] = [parents[p] + (c,) for p, c in zip(col.parent.tolist(), components)]
    return out


def build(case: Case) -> tuple[View, dict[ShapePath, list[Location]]]:
    """A fresh store of the documents with the case's flags, its root view, and its row locations."""
    store = build_store(list(case.docs))
    table = row_locations(store)
    rows: dict[ShapePath, list[int]] = {}
    for column, locs in table.items():
        for row, loc in enumerate(locs):
            if loc in case.flagged_items:
                rows.setdefault(column, []).append(row)
    for column in sorted(rows, key=len):
        store.flags.flag_items(column, rows[column])
    if case.flagged_docs:
        store.flags.flag_documents(sorted(case.flagged_docs))
    return View.root(store), table


def locations_of(view: View, table: dict[ShapePath, list[Location]]) -> list[Location]:
    return [table[part.column][row] for part in view.parts for row in part.rows.tolist()]


# ---------------------------------------------------------------------------
# References
# ---------------------------------------------------------------------------

TYPE_ORDER = ("null", "boolean", "integer", "number", "string", "array", "object")


def type_name(value: Any) -> str:
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


def reference_naive(case: Case, found: Sequence[tuple[Location, Any]]) -> dict[str, Any]:
    """Naive_Merge of non-flagged values from the raw documents (Requirement 34.4), honoring flagged items.

    A flagged member still makes its key present (it counts for ``required``);
    its value is left out of the key's subschema.
    """
    if not found:
        return {}
    present = {type_name(value) for _, value in found}
    if "number" in present:
        present.discard("integer")
    out: dict[str, Any] = {"type": [name for name in TYPE_ORDER if name in present]}
    objects = [(loc, value) for loc, value in found if isinstance(value, dict)]
    keys = sorted({key for _, obj in objects for key in obj}, key=codepoint_key)
    if keys:
        out["properties"] = {}
        for key in keys:
            members = [(loc + (key,), obj[key]) for loc, obj in objects if key in obj]
            out["properties"][key] = reference_naive(case, [m for m in members if m[0] not in case.flagged_items])
        required = [key for key in keys if all(key in obj for _, obj in objects)]
        if required:
            out["required"] = required
    elements = [
        (loc + (i,), element)
        for loc, value in found
        if isinstance(value, list)
        for i, element in enumerate(value)
        if loc + (i,) not in case.flagged_items
    ]
    if elements:
        out["items"] = reference_naive(case, elements)
    return out


@dataclass(frozen=True)
class ArraysRef:
    """The non-flagged arrays of one call, their raw lengths, and their non-flagged elements by position."""

    arrays: tuple[Location, ...]
    lengths: tuple[int, ...]
    positions: tuple[tuple[tuple[Location, Any], ...], ...]

    @property
    def elements(self) -> list[tuple[Location, Any]]:
        return [entry for position in self.positions for entry in position]

    @property
    def max_length(self) -> int:
        return max(self.lengths)


def arrays_reference(case: Case, view: View, table: dict[ShapePath, list[Location]]) -> ArraysRef:
    """The reference arrays of a call's view, from the raw documents (the view only locates its rows)."""
    arrays: list[Location] = []
    for loc in locations_of(view, table):
        assert not case.excluded(loc), f"{loc} is in the view at {render_path(view.path)}, but it is flagged"
        if isinstance(value_at(case.docs, loc), list):
            arrays.append(loc)
    assert arrays, f"array_node was called at {render_path(view.path)} without a non-flagged array"
    raw = [value_at(case.docs, loc) for loc in arrays]
    lengths = tuple(len(value) for value in raw)
    positions = tuple(
        tuple(
            (loc + (i,), value[i])
            for loc, value in zip(arrays, raw)
            if i < len(value) and not case.excluded(loc + (i,))
        )
        for i in range(max(lengths))
    )
    return ArraysRef(tuple(arrays), lengths, positions)


# ---------------------------------------------------------------------------
# Analyzers
# ---------------------------------------------------------------------------

SCENARIOS = ("drawn", "tupling", "all off")


def configs(scenario: str) -> st.SearchStrategy[RunConfiguration]:
    # Path-by-Path walks: shared definitions and recursion (Property 16) stay off.
    off = {technique_setting(SHARED_DEFS): False, technique_setting(RECURSION): False}
    if scenario == "drawn":
        return run_configs(vary=TECHNIQUE_SETTINGS, base=off)
    if scenario == "tupling":
        return run_configs(vary=TECHNIQUE_SETTINGS, base={**off, technique_setting(MDL): True})
    return run_configs(vary=TECHNIQUE_SETTINGS, base={technique_setting(tid): False for tid in REGISTRY.ids()})


def make_analyzer(
    config: RunConfiguration, scenario: str, *, recording: bool
) -> tuple[StructureAnalyzer, InferenceReport]:
    """A Structure_Analyzer with the configuration's Technique states; ``tupling`` gives its ArrayAnalyzer the stub."""
    report = InferenceReport()
    engine = DecisionEngine(config.techniques, report=report, scorer=SCORER)
    values = ValueInferrer(engine, validator=VALIDATOR, report=report, scorer=SCORER)
    array_scorer = TUPLING if scenario == "tupling" else SCORER
    arrays = (RecordingArrayAnalyzer if recording else ArrayAnalyzer)(values, scorer=array_scorer)
    analyzer = StructureAnalyzer(values, scorer=SCORER, arrays=arrays, resources=config.resources, seed=config.seed)
    return analyzer, report


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def accepts(schema: SchemaNode, instance: Any) -> bool:
    return VALIDATOR.is_valid(VALIDATOR.compile(schema.to_json()), instance)


def array_type(node: SchemaNode) -> bool:
    types = node.kw.get("type") if node.boolean is None else None
    return isinstance(types, list) and "array" in types


ARRAY_KEYWORDS = ("items", "prefixItems", "minItems", "maxItems")


def link(model: SchemaModel, calls: Sequence[tuple[View, ArrayNode]]) -> list[tuple[str, SchemaNode]]:
    """The final ``(pointer, node)`` of each call; checks that calls and array-typed nodes match one to one.

    A call's node is in the model as it is (one JSON-type family), or its
    keywords were merged into the node of a mixed-type Path, which then
    holds the ArrayNode's Provenance_Records (the same objects, re-keyed). A
    call without records (no element observed and no item-count bounds) and
    a merged node keep only ``type`` of the array keywords, so such calls
    take any remaining node that holds none of them.
    """
    nodes = list(model.walk())
    finals = {node.nid: (pointer, node) for pointer, node in nodes if array_type(node)}
    stray = [
        pointer for pointer, node in nodes if not array_type(node) and ("items" in node.kw or "prefixItems" in node.kw)
    ]
    assert not stray, f"nodes without type array hold items or prefixItems: {stray[:3]}"
    assert len(finals) == len(calls), f"{len(finals)} array nodes in the model for {len(calls)} array_node calls"
    owner = {id(record): cid.nid for cid, record in model.provenance.items()}
    nids: list[int | None] = []
    for view, result in calls:
        found = {owner.get(id(record)) for record in result.provenance.values()}
        if result.node.nid in finals:
            found.add(result.node.nid)
        assert None not in found and len(found) <= 1, f"the array records of {render_path(view.path)} are misplaced"
        nids.append(found.pop() if found else None)
    taken = Counter(nid for nid in nids if nid is not None)
    assert all(count == 1 for count in taken.values()), "two calls share one final node"
    spare = [nid for nid in finals if nid not in taken and not any(k in finals[nid][1].kw for k in ARRAY_KEYWORDS)]
    out: list[tuple[str, SchemaNode]] = []
    for (view, _), nid in zip(calls, nids):
        if nid is None:
            assert spare, f"no final node left for the call at {render_path(view.path)}"
            nid = spare.pop()
        out.append(finals[nid])
    assert len({node.nid for _, node in out}) == len(finals), "some array node belongs to no call"
    return out


def note_case(case: Case) -> None:
    if case.flagged_docs:
        event("flagged documents")
    if case.flagged_items:
        event("flagged arrays or elements")
    if case.whole_position:
        event("flagged every element at one position of a Path")
    if case.emptied_array:
        event("flagged every element of one array")


@pytest.mark.parametrize("group", tuple(LAYOUT_GROUPS))
@pytest.mark.parametrize("scenario", SCENARIOS)
@given(data=st.data())
def test_array_nodes_match_their_classification(scenario: str, group: str, data: st.DataObject) -> None:
    config = data.draw(configs(scenario), label="config")
    case = data.draw(cases(group), label="case")
    root, table = build(case)
    analyzer, report = make_analyzer(config, scenario, recording=True)
    model = analyzer.infer_model(root)
    recorder = analyzer.arrays
    assert isinstance(recorder, RecordingArrayAnalyzer)
    calls = recorder.calls
    fresh, _ = make_analyzer(config, scenario, recording=False)
    dirty = case.dirty
    mdl_on = config.techniques[MDL]

    # Coverage: every non-flagged raw array is in exactly one call's view.
    references = [arrays_reference(case, view, table) for view, _ in calls]
    covered = Counter(loc for ref in references for loc in ref.arrays)
    expected = Counter(
        loc for loc, value, _ in raw_walk(case.docs) if isinstance(value, list) and not case.excluded(loc)
    )
    assert covered == expected, "the array_node calls do not cover each non-flagged array exactly once"

    resolved: Counter[tuple[str, str]] = Counter()
    for (view, result), ref, (pointer, node) in zip(calls, references, link(model, calls)):
        where = f"{render_path(view.path)} (node {pointer or '/'})"
        kw = node.kw
        # The final node holds the ArrayNode's keywords unchanged.
        for keyword, value in result.node.kw.items():
            if keyword != "type":
                assert keyword in kw and (kw[keyword] is value or kw[keyword] == value), f"{keyword} changed at {where}"
        lengths = result.outcome.lengths
        assert (lengths.n_arrays, lengths.max_length, lengths.n_elements) == (
            len(ref.arrays),
            ref.max_length,
            len(ref.elements),
        ), f"length statistics at {where}"
        assert dict(zip(lengths.lengths, lengths.counts)) == dict(Counter(ref.lengths)), f"length histogram at {where}"
        type_record = model.provenance[ConstraintId(node.nid, "type")]

        if not ref.elements:  # 7.9: only empty arrays, or only flagged elements
            assert "items" not in kw and "prefixItems" not in kw, f"{where} observes no element, but emits {sorted(kw)}"
            assert type_record.details.get("elements_observed") is False, f"type record at {where}"
            assert result.outcome.kind is None
            cause = "only flagged elements" if ref.max_length else "empty arrays"
            event(f"{scenario}: no element observed ({cause})")
            continue
        assert "elements_observed" not in type_record.details, f"type record at {where}"

        arrays_view = array_rows(view)
        assert Counter(locations_of(arrays_view.elements(), table)) == Counter(loc for loc, _ in ref.elements), (
            f"the element view at {where} differs from the non-flagged elements"
        )
        kind = TUPLE if "prefixItems" in kw else LIST
        assert result.outcome.kind == kind
        records = [model.provenance[ConstraintId(node.nid, k)] for k in ("prefixItems", "items") if k in kw]
        for record in records:
            assert record.details["judgment"] == kind, f"record judgment at {where}"
            stats = record.details["length_statistics"]
            assert (stats["max_length"], stats["min_length"], stats["n_arrays"], stats["n_elements"]) == (
                ref.max_length,
                min(ref.lengths),
                len(ref.arrays),
                len(ref.elements),
            ), f"recorded length statistics at {where}"
            assert math.isclose(stats["mean_length"], math.fsum(ref.lengths) / len(ref.lengths))
        resolved[(render_path(view.path), kind)] += 1
        if scenario == "tupling":
            assert kind == TUPLE, f"the stub scorer makes {where} a Tuple"
        if scenario == "all off" or not mdl_on:
            assert kind == LIST, f"the Fallback_Behavior makes {where} a List"

        if kind == LIST:
            # 7.2: one items subschema over every element, and no prefixItems.
            assert len(records) == 1 and "prefixItems" not in kw
            items = kw["items"]
            assert isinstance(items, SchemaNode) and items.boolean is None
            subschemas = [(items, ref.elements, arrays_view.elements())]
            event(f"{scenario}: List")
        else:
            # 7.3: one subschema per position up to the maximum length; 7.4: items false.
            prefix = kw["prefixItems"]
            assert len(prefix) == ref.max_length, f"prefixItems at {where}: {len(prefix)} entries, not {ref.max_length}"
            items = kw.get("items")
            assert isinstance(items, SchemaNode) and items.boolean is False, f"no items: false at {where}"
            assert records[0] == records[1]
            subschemas = [(prefix[i], list(ref.positions[i]), arrays_view.elements(i)) for i in range(ref.max_length)]
            event(f"{scenario}: Tuple")
            if min(ref.lengths) < ref.max_length:
                event(f"{scenario}: Tuple with shorter arrays")
            for i, position in enumerate(ref.positions):
                if not position:
                    assert prefix[i] == SchemaNode(), f"position {i} at {where} holds only flagged elements"
                    event(f"{scenario}: Tuple position whose elements are all flagged")

        for subschema, elements, element_view in subschemas:
            assert Counter(locations_of(element_view, table)) == Counter(loc for loc, _ in elements)
            rejected = [value for loc, value in elements if loc not in dirty and not accepts(subschema, value)]
            assert not rejected, f"a subschema at {where} rejects non-flagged elements {rejected[:3]!r}"
            if scenario == "all off":
                reference = SchemaNode.from_json(reference_naive(case, elements))
            else:
                reference = fresh.infer_model(element_view).root
            assert subschema == reference, (
                f"subschema at {where} differs from its reference:\n{subschema.to_json()!r}\n{reference.to_json()!r}"
            )
        clean = [value_at(case.docs, loc) for loc in ref.arrays if loc not in dirty]
        rejected = [value for value in clean if not accepts(node, value)]
        assert not rejected, f"the node at {where} rejects non-flagged arrays {rejected[:3]!r}"

        tokens = pointer.split("/")
        if any(token in ("anyOf", "oneOf") for token in tokens[:-2]):
            event("array node below an anyOf or oneOf branch (a Variant branch)")
        elif len(tokens) >= 2 and tokens[-2] == "anyOf":
            event("array node as the remaining-types branch")
        if len(kw["type"]) > 1:
            event("array node at a mixed-type Path")
        if ELEM_STEP in view.path:
            event("array Path below an array")

    # The report holds one tuple_or_list entry per resolved call, and none for Paths without elements.
    entries = Counter(
        (entry.path, entry.resolution.candidates[entry.resolution.selected].outcome)
        for entry in report.decisions
        if entry.kind == TUPLE_OR_LIST
    )
    assert entries == resolved, f"report tuple_or_list entries {dict(entries)} != resolved calls {dict(resolved)}"
    note_case(case)
