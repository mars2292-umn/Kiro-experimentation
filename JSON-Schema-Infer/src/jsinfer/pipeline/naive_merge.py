"""Naive_Merge: the baseline schema, and the per-view subschemas that MDL scoring and relaxation reuse.

Naive_Merge (Requirement 34.4) describes a set of Observations with five
keywords only: ``$schema``, ``type``, ``properties``, ``items``, and
``required``. For one view (``core.views``) the subschema is:

``type``
    The JSON types of the view's rows in emitted order (null, boolean,
    integer, number, string, array, object), with the integer rule of
    Requirement 13.2: ``integer`` is listed when every number has a zero
    fractional part (``1``, ``1.0``, and ``1e3`` are all integers, see
    ``core.jsonvalue.JType``), and ``number`` replaces it as soon as one
    number is non-integral, so the two never appear together.
``properties``
    One entry per key of the view's objects (the union of their keys), each
    holding the Naive_Merge subschema of the member values under that key
    (:meth:`View.child_key`). Present only when some object has a key.
``required``
    The keys present in every object of the view, whatever the member's
    value (a ``null`` value counts as present). Present only when non-empty.
``items``
    One subschema merging every element of every array of the view
    (:meth:`View.elements`). Present only when some element is observed:
    a Path with only empty arrays emits no ``items``, which accepts every
    array, as Requirement 7.9 does for the Inferrer.

A view with no rows gets the empty schema ``{}``: there is nothing to
describe, and ``{}`` accepts every value. ``$schema`` is not a node keyword
in a Schema_Model; the Schema_Emitter writes it at the root of every emitted
document (Requirement 2.1), so ``SchemaEmitter().emit(naive_merge(docs))``
is the Naive_Merge baseline file.

Soundness
    Every keyword only lists what was observed, so each value of a view
    validates against the view's subschema: its type is listed (an integer
    is also a ``number``), every required key occurs in every object, and
    members and elements validate against their merged subschemas by
    induction. In particular every non-flagged document validates against
    :func:`naive_merge` of a collection.

Flags
    Views exclude flagged documents, flagged items, and their subtrees
    (``core.stats.FlagSet``), so Naive_Merge describes exactly the
    non-flagged Observations, as Constraint inference does (Requirement
    17.4). As in ``core.stats``, a flagged member value still makes its key
    present in its non-flagged parent object: the key stays in
    ``properties`` and counts for ``required``, and when every value under
    the key is flagged its subschema is ``{}``.

Reuse
    :func:`naive_merge_node` gives the subschema of any view (a logical
    Path, a Variant, a Tuple position, or a multi-part union view). The
    MDL_Scorer represents the descendants a candidate implies by these
    subschemas (local scoring), and the relaxation table uses them for new
    properties under ``additionalProperties: false`` and for extra elements
    beyond ``items: false``. :func:`naive_merge_values` gives the subschema of
    loose JSON values, such as the failing values of a relaxation, by
    flattening them into a small store of their own. With every Technique
    disabled, each Decision_Point's Fallback_Behavior reproduces this
    module's output (Requirement 28.5, Property 49), and the Naive_Merge
    Baseline_Adapter calls :func:`naive_merge`.

Determinism
    The result depends only on the multiset of Observations in the view:
    types come from integer counts, and property names follow key ids,
    which are in code-point order (``core.observations``), so neither
    document order, chunking, nor worker count can change it. Nodes are
    created in the canonical pre-order of :meth:`SchemaNode.walk`, so their
    nids increase in that order. The walk uses an explicit stack, so deep
    documents do not hit the recursion limit.

Each call returns fresh SchemaNodes that the caller may place in a tree and
edit; nothing is shared between calls.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import Final

from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.observations import ObservationStore, build_store
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.core.stats import ViewStats
from jsinfer.core.views import View

__all__ = [
    "NAIVE_MERGE_KEYWORDS",
    "naive_merge",
    "naive_merge_model",
    "naive_merge_node",
    "naive_merge_values",
    "naive_type_names",
]

# The only keywords a Naive_Merge document holds (Requirement 34.4). The
# Schema_Emitter adds $schema at the root; nodes hold the other four.
NAIVE_MERGE_KEYWORDS: Final = ("$schema", "type", "properties", "items", "required")


def naive_type_names(stats: ViewStats) -> list[str]:
    """The ``type`` names of a view's rows in emitted order, with the integer rule.

    ``integer`` is listed when the view holds integral numbers and no
    non-integral one; ``number`` replaces it otherwise. The list is empty
    exactly when the view has no rows.
    """
    present = stats.types
    if JType.NUMBER in present:
        present = tuple(jtype for jtype in present if jtype != JType.INTEGER)
    return [jtype.keyword for jtype in present]


def _fill(node: SchemaNode, view: View) -> list[tuple[View, dict[str, SchemaNode], str]]:
    """Set the keywords of ``node`` from ``view``; return the child slots in canonical pre-order.

    Each slot is ``(child view, container, name)``: the child's subschema
    goes to ``container[name]``. Children come in the order of
    :meth:`SchemaNode.children`: ``items`` first, then ``properties`` entries
    in code-point order of their names.
    """
    stats = view.stats()
    if stats.n == 0:
        return []
    kw = node.kw
    kw["type"] = naive_type_names(stats)
    slots: list[tuple[View, dict[str, SchemaNode], str]] = []

    if len(stats.key_ids):
        store = view.store
        # Key ids follow code-point order, so names are already sorted.
        names = [store.key_name(int(kid)) for kid in stats.key_ids.tolist()]
        properties: dict[str, SchemaNode] = {}
        kw["properties"] = properties
        n_objects = stats.n_objects
        required = [name for name, count in zip(names, stats.key_counts.tolist()) if count == n_objects]
        if required:
            kw["required"] = required
        slots.extend((view.child_key(name), properties, name) for name in names)

    if stats.count(JType.ARRAY):
        elements = view.elements()
        if not elements.is_empty:
            # kw doubles as the container of the single items subschema; items sorts before properties.
            slots.insert(0, (elements, kw, "items"))
    return slots


def naive_merge_node(view: View) -> SchemaNode:
    """The Naive_Merge subschema of the non-flagged rows of ``view`` (a fresh node tree).

    See the module docstring for the keywords. An empty view gives ``{}``.
    """
    if not isinstance(view, View):
        raise TypeError(f"naive_merge_node takes a View, got {type(view).__name__}")
    holder: dict[str, SchemaNode] = {}
    # (view, container, name), popped in canonical pre-order.
    stack: list[tuple[View, dict[str, SchemaNode], str]] = [(view, holder, "root")]
    while stack:
        current, container, name = stack.pop()
        node = SchemaNode()
        container[name] = node
        stack.extend(reversed(_fill(node, current)))
    return holder["root"]


def naive_merge_model(source: ObservationStore | View) -> SchemaModel:
    """The Naive_Merge Schema_Model of a store (its root view) or of a view, such as a document subset.

    The model is fully supported, declares no assumed dialect, and carries no
    Provenance_Records; the Schema_Emitter adds ``$schema`` at the root.
    """
    if isinstance(source, ObservationStore):
        view = View.root(source)
    elif isinstance(source, View):
        view = source
    else:
        raise TypeError(f"naive_merge_model takes an ObservationStore or a View, got {type(source).__name__}")
    return SchemaModel(root=naive_merge_node(view))


def naive_merge(documents: Iterable[JSON], chunk_size: int = 2000) -> SchemaModel:
    """The Naive_Merge Schema_Model of a Document_Collection given as parsed JSON values.

    ``chunk_size`` is the flattening chunk size (``ingest.chunk_size``); the
    result does not depend on it. An empty collection gives the model of
    ``{}``, which accepts every document.
    """
    return naive_merge_model(build_store(documents, chunk_size))


def naive_merge_values(values: Iterable[JSON]) -> SchemaNode:
    """The Naive_Merge subschema of loose JSON values, for example the failing values of a relaxation.

    The values are flattened into a store of their own (each value is one
    root row), so the result equals :func:`naive_merge_node` of a view that
    holds exactly these values. No values give ``{}``.
    """
    return naive_merge_node(View.root(build_store(values)))
