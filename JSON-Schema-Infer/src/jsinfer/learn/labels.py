"""Ground-truth Judgment_Call labels and Path alignment (Requirements 22.4, 22.5, 31.9, 31.10).

:func:`derive_labels` reads one label per applicable Judgment_Call kind at
each Path of a Schema_Model from the schema's keywords, on the
``$ref``-expanded model and with the Path stepping the Inferrer uses
(``corpus.normalize.walk_paths``): ``properties`` adds a Key step,
``patternProperties`` and a schema-valued ``additionalProperties`` add a
MapVal step, and ``items`` and ``prefixItems`` add an Elem step. The
subschemas at one Path are the node reached there, its ``allOf``, ``anyOf``,
``oneOf``, ``if``, ``then``, and ``else`` branches, and the targets of its
``$ref``. The rules of 22.4, read per subschema:

Record-or-Map (``record_or_map``)
    At a Path with an object subschema (one that declares ``properties``,
    ``patternProperties``, or ``additionalProperties``, or whose ``type``
    includes ``object``). ``record`` when a subschema declares
    ``properties``; ``map`` when it declares ``patternProperties`` or a
    schema-valued ``additionalProperties`` (a keyword schema, not ``true``
    or ``false``) and no ``properties``.
Closed-or-Open-Record (``closed_or_open_record``)
    At a Path where some subschema declares ``properties`` (a Record).
    ``closed`` when an object subschema there declares
    ``additionalProperties: false``; ``open`` when it declares
    ``additionalProperties`` as ``true`` or as a schema.
Tuple-or-List (``tuple_or_list``)
    At a Path with an array subschema (``prefixItems``, ``items``,
    ``contains``, or ``type`` including ``array``). ``tuple`` when a
    subschema declares ``prefixItems``; ``list`` when it declares ``items``
    without ``prefixItems``.
Closed-or-Open-Enum (``closed_or_open_enum``)
    At a Path with a scalar subschema (``enum`` or ``const``, or ``type``
    including null, boolean, integer, number, or string). ``closed`` when a
    subschema declares ``enum`` or ``const``; ``open`` when a string,
    integer, or number subschema declares neither.
Merge-or-Split (``merge_or_split``)
    At a Path with an object subschema. ``split`` when a subschema declares
    ``anyOf`` or ``oneOf`` with two or more object branches (a branch that is
    a bare ``$ref`` counts as its target); ``merge`` when the Path has
    exactly one object subschema.

A kind that applies at a Path but whose rules give no label, or give
different labels in different subschemas of the Path, is recorded as
:data:`UNKNOWN` (22.5; design, Labels). Kinds that do not apply at a Path
have no entry. The ``$ref`` expansion is capped at ``max_depth`` Path steps
(the ``synth.max_depth`` default), so recursive schemas give finite label
sets. Keys are ``(Path, kind)``; kinds are the keys of
:data:`~jsinfer.decide.engine.JUDGMENT_OUTCOMES`.

:func:`judgment_accuracy` is the per-kind accuracy of 31.9 between two label
sets: over the Paths with a known label of the kind in the ground truth, the
fraction at which the other set has the same Path and the same label; a Path
absent from the other set counts as incorrect. :class:`SchemaPaths` holds a
schema's Path set, and :func:`align` maps an inferred Path to the schema Path
it stands for (31.10; design, Featurization): the same steps where the schema
has them, a Key step mapped to the schema's MapVal step when the schema has a
Map where the Inferrer emitted a Record property, and ``None`` when a step
has no counterpart.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Iterable, Mapping
from typing import Any, Final

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.paths import KEY, MAPVAL_STEP, Path, Step, path_sort_key
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.corpus.normalize import ref_target, walk_paths
from jsinfer.decide.engine import JUDGMENT_OUTCOMES

__all__ = [
    "DEFAULT_MAX_DEPTH",
    "KINDS",
    "UNKNOWN",
    "Labels",
    "SchemaPaths",
    "align",
    "derive_labels",
    "judgment_accuracy",
    "schema_paths",
]

Labels = dict[tuple[Path, str], str]

# The Judgment_Call kinds, in the order of the design's table.
KINDS: Final[tuple[str, ...]] = tuple(JUDGMENT_OUTCOMES)
UNKNOWN: Final = "unknown"
DEFAULT_MAX_DEPTH: Final[int] = int(RESOURCE_PARAMETERS["synth.max_depth"].default)  # type: ignore[arg-type]

_SCALAR_TYPES: Final = frozenset({"null", "boolean", "integer", "number", "string"})
_OPEN_ENUM_TYPES: Final = frozenset({"integer", "number", "string"})
_OBJECT_KEYWORDS: Final = ("properties", "patternProperties", "additionalProperties")
_ARRAY_KEYWORDS: Final = ("prefixItems", "items", "contains")


def _types(node: SchemaNode) -> frozenset[str]:
    declared = node.kw.get("type")
    if isinstance(declared, str):
        return frozenset({declared})
    if isinstance(declared, (list, tuple)):
        return frozenset(name for name in declared if isinstance(name, str))
    return frozenset()


def _target(model: SchemaModel, node: SchemaNode) -> SchemaNode:
    """``node``, or the target of its bare ``$ref`` when it has no other keywords."""
    ref = node.kw.get("$ref")
    if node.boolean is None and isinstance(ref, str) and len(node.kw) == 1:
        pointer = ref_target(ref)
        if pointer is not None:
            try:
                return model.resolve(pointer)
            except (KeyError, ValueError):
                return node
    return node


def _is_object(node: SchemaNode) -> bool:
    return node.boolean is None and ("object" in _types(node) or any(k in node.kw for k in _OBJECT_KEYWORDS))


def _is_array(node: SchemaNode) -> bool:
    return node.boolean is None and ("array" in _types(node) or any(k in node.kw for k in _ARRAY_KEYWORDS))


def _is_scalar(node: SchemaNode) -> bool:
    return node.boolean is None and (
        "enum" in node.kw or "const" in node.kw or bool(_types(node) & _SCALAR_TYPES)
    )


def _schema_valued(value: Any) -> bool:
    return isinstance(value, SchemaNode) and value.boolean is None


def _record_or_map(node: SchemaNode) -> str | None:
    kw = node.kw
    if "properties" in kw:
        return "record"
    if "patternProperties" in kw or _schema_valued(kw.get("additionalProperties")):
        return "map"
    return None


def _closed_or_open(node: SchemaNode) -> str | None:
    additional = node.kw.get("additionalProperties")
    if not isinstance(additional, SchemaNode):
        return None
    if additional.boolean is False:
        return "closed"
    return "open"


def _tuple_or_list(node: SchemaNode) -> str | None:
    kw = node.kw
    if "prefixItems" in kw:
        return "tuple"
    if "items" in kw:
        return "list"
    return None


def _closed_or_open_enum(node: SchemaNode) -> str | None:
    kw = node.kw
    if "enum" in kw or "const" in kw:
        return "closed"
    if _types(node) & _OPEN_ENUM_TYPES:
        return "open"
    return None


def _combine(votes: Iterable[str | None]) -> str:
    """One label from the per-subschema readings: the common label, or unknown when none or several."""
    found = sorted({vote for vote in votes if vote is not None})
    if len(found) == 1:
        return found[0]
    return UNKNOWN


def _merge_or_split(model: SchemaModel, nodes: list[SchemaNode], objects: list[SchemaNode]) -> str:
    for node in nodes:
        if node.boolean is not None:
            continue
        for keyword in ("anyOf", "oneOf"):
            branches = node.kw.get(keyword, ())
            if sum(1 for branch in branches if _is_object(_target(model, branch))) >= 2:
                return "split"
    if len(objects) == 1:
        return "merge"
    return UNKNOWN


def derive_labels(model: SchemaModel, max_depth: int = DEFAULT_MAX_DEPTH) -> Labels:
    """The ground-truth labels of ``model`` per ``(Path, kind)`` (see the module docstring)."""
    if not isinstance(model, SchemaModel):
        raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
    by_path: dict[Path, list[SchemaNode]] = defaultdict(list)
    for path, node in walk_paths(model, max_depth):
        by_path[path].append(node)
    labels: Labels = {}
    for path in sorted(by_path, key=path_sort_key):
        nodes = by_path[path]
        # A node that is only a $ref stands for its target, which the walk also
        # yields at this Path; counting both would double every object.
        own = [node for node in nodes if not (node.boolean is None and set(node.kw) == {"$ref"})]
        objects = [node for node in own if _is_object(node)]
        arrays = [node for node in own if _is_array(node)]
        scalars = [node for node in own if _is_scalar(node)]
        if objects:
            labels[path, "record_or_map"] = _combine(_record_or_map(node) for node in objects)
            if any("properties" in node.kw for node in objects):
                labels[path, "closed_or_open_record"] = _combine(_closed_or_open(node) for node in objects)
            labels[path, "merge_or_split"] = _merge_or_split(model, own, objects)
        if arrays:
            labels[path, "tuple_or_list"] = _combine(_tuple_or_list(node) for node in arrays)
        if scalars:
            labels[path, "closed_or_open_enum"] = _combine(_closed_or_open_enum(node) for node in scalars)
    return labels


def judgment_accuracy(truth: Mapping[tuple[Path, str], str], inferred: Mapping[tuple[Path, str], str]) -> dict[str, float | None]:
    """Per-kind accuracy of ``inferred`` against ``truth`` (31.9); None for a kind with no known label in ``truth``."""
    correct: dict[str, int] = dict.fromkeys(KINDS, 0)
    known: dict[str, int] = dict.fromkeys(KINDS, 0)
    for (path, kind), label in truth.items():
        if label == UNKNOWN:
            continue
        known[kind] += 1
        if inferred.get((path, kind)) == label:
            correct[kind] += 1
    return {kind: (correct[kind] / known[kind] if known[kind] else None) for kind in KINDS}


class SchemaPaths:
    """The Path set of a schema, with :meth:`align` (see the module docstring)."""

    __slots__ = ("_paths",)

    def __init__(self, paths: Iterable[Path]) -> None:
        self._paths = frozenset(tuple(path) for path in paths)

    @classmethod
    def of(cls, model: SchemaModel, max_depth: int = DEFAULT_MAX_DEPTH) -> SchemaPaths:
        return cls(path for path, _ in walk_paths(model, max_depth))

    @property
    def paths(self) -> frozenset[Path]:
        return self._paths

    def __contains__(self, path: object) -> bool:
        return path in self._paths

    def align(self, path: Path) -> Path | None:
        """The schema Path that inferred ``path`` stands for, or None when a step has no counterpart."""
        aligned: Path = ()
        for step in path:
            if not isinstance(step, Step):
                raise TypeError(f"a Path is a tuple of Steps, got {type(step).__name__}")
            if aligned + (step,) in self._paths:
                aligned += (step,)
            elif step.kind == KEY and aligned + (MAPVAL_STEP,) in self._paths:
                aligned += (MAPVAL_STEP,)
            else:
                return None
        return aligned


def schema_paths(model: SchemaModel, max_depth: int = DEFAULT_MAX_DEPTH) -> frozenset[Path]:
    """The Paths of ``model`` under the Path stepping of the Inferrer, ``$ref`` expanded up to ``max_depth``."""
    return SchemaPaths.of(model, max_depth).paths


def align(schema: SchemaModel | SchemaPaths, path: Path) -> Path | None:
    """Map inferred ``path`` to the Path of ``schema`` it stands for (31.10), or None (see :meth:`SchemaPaths.align`)."""
    paths = schema if isinstance(schema, SchemaPaths) else SchemaPaths.of(schema)
    return paths.align(tuple(path))
