"""Schema_Emitter: canonical layout, the ConstraintId → JSON Pointer map, and bytes (Requirements 2.1–2.3, 26.5).

Task 6.1 implements the layout and the schema bytes. The explanation file
and inline ``x-provenance`` (task 17.2) are added here later.

Layout
    :meth:`SchemaEmitter.layout` turns a
    :class:`~jsinfer.core.schema_model.SchemaModel` into a plain draft 2020-12
    JSON document in which every order is fixed by the model's content and
    never by the order in which Constraints were added (2.3):

    - The keywords of each schema object follow :data:`KEYWORD_ORDER`, the
      design's rank order: ``$schema``, ``$id``, ``$ref``, annotations,
      ``type``, ``enum``, ``const``, ``format``, ``pattern``, numeric bounds,
      string length bounds, ``properties``, ``patternProperties``,
      ``additionalProperties``, ``propertyNames``, ``required``,
      ``dependentRequired``, object bounds, ``prefixItems``, ``items``,
      ``contains``, array bounds, ``uniqueItems``, ``anyOf``, ``oneOf``,
      ``allOf``, ``not``, ``if``/``then``/``else``, and ``$defs``. Draft
      2020-12 keywords outside the supported set (``multipleOf``,
      ``unevaluatedProperties``, ``$anchor``, ...) have fixed slots next to
      their kin, so any vocabulary keyword a model holds has a rank. Then
      come the ``x-`` keywords in code-point order, and ``x-provenance``
      last (its slot is reserved for task 17.2).
    - ``properties``, ``patternProperties``, ``$defs``, and
      ``dependentRequired`` entries are in code-point order of their names.
      ``prefixItems``, ``anyOf``, ``oneOf``, and ``allOf`` keep their index
      order, because each element is its own position (2.7).
    - ``type`` lists its distinct names in the order null, boolean, integer,
      number, string, array, object, written as a string when there is one
      name. ``required`` and each ``dependentRequired`` entry list their
      distinct names in code-point order. ``enum`` lists its distinct members
      under JSON Schema equality in canonical value order
      (``core.jsonvalue.distinct_values``). These are the lists that 2.7
      compares as sets, so repeats and order cannot change the bytes.
    - Every other JSON value (``const``, bounds, ``default``, ``examples``,
      ``x-`` annotations, ...) is written as its canonical representative
      (``core.jsonvalue.canonical_value``): integral floats become ``int``
      (``2.0`` → ``2``, ``-0.0`` → ``0``, ``1e40`` → the exact integer it
      equals), and object keys are in code-point order. Arrays such as
      ``examples`` keep their order.
    - ``$schema`` is set to the draft 2020-12 URI on the root schema object
      and is dropped from every other node, whatever the model holds (models
      are 2020-12 after loading). An ``x-provenance`` keyword held in a node
      is dropped too: 2.7 excludes it from equality, so emitting it would let
      equal models produce different bytes. Inline provenance is generated
      from the Provenance_Records instead (26.6, task 17.2).
    - A boolean root is emitted as ``true`` or ``false``. It has no schema
      object to hold ``$schema``, and wrapping it (``{}`` for ``true``) would
      change the model under 2.7. The Schema_Loader reads a boolean root as
      draft 2020-12 with an assumed dialect, which 2.7 ignores.

    Equal models under 2.7 therefore produce equal documents, and with
    :meth:`SchemaEmitter.dumps`, byte-identical files.

Checks
    Every emitted keyword must be in :data:`VOCABULARY_KEYWORDS` (the draft
    2020-12 vocabularies) or start with ``x-`` (2.2); names under
    ``properties``, ``patternProperties``, ``dependentRequired``, and
    ``$defs`` are not keywords. Pre-2020-12 keywords such as ``definitions``
    and ``dependencies`` are not in the vocabularies, so the Schema_Loader
    must have translated them. The emitter also rejects values it would have
    to reorder but cannot: ``type`` names other than the seven JSON types or
    an empty ``type`` list, non-string ``required`` and ``dependentRequired``
    names, a non-array ``enum``, non-JSON values (NaN, sets, ...), and
    subschema keywords holding the wrong container. Every node's ``nid``
    must be distinct, so that the pointer map is well defined; this also
    rejects one node object placed at two positions. Each check
    raises :class:`EmitError` with the JSON Pointer of the offending
    location. The emitter does not otherwise re-validate keyword values
    against the meta-schema; the Inferrer builds them, and the tests check
    meta-schema validity with the Validator (2.1).

Pointer map
    ``layout`` also returns the JSON Pointer, into the emitted document, of
    every Constraint in it, keyed by
    :class:`~jsinfer.core.schema_model.ConstraintId` at the granularity of
    the design's table (26.5):

    - a ``required`` entry is ``(nid, "required", name)`` at
      ``…/required/<i>``, where ``i`` is the name's index in the emitted
      (deduplicated, sorted) list;
    - a ``dependentRequired`` entry is ``(nid, "dependentRequired", key)`` at
      ``…/dependentRequired/<key>``;
    - an ``anyOf`` or ``oneOf`` branch is ``(nid, keyword, i)`` at
      ``…/anyOf/<i>``;
    - every other Constraint keyword (``type``, ``enum``, ``properties``,
      ``additionalProperties``, ``items``, bounds, ``not``, ``$ref``,
      ``x-latent-type``, ...) is ``(nid, keyword)`` at ``…/<keyword>``.

    ``nid`` is the node that holds the keyword: ``additionalProperties:
    false`` is a Constraint of its parent object, and boolean subschemas hold
    none. The keywords in ``core.schema_model.NON_CONSTRAINT_KEYWORDS``
    (``$schema``, ``$defs``, ``$id``, the retained annotations, and
    ``x-provenance``) and container entries such as ``properties/<name>``
    are not Constraints. Constraints inside ``$defs`` entries are included.
    Entries are in document order (pre-order, keywords in rank order). The
    explanation file of task 17.2 keys records by these pointers.

Bytes
    :meth:`SchemaEmitter.dumps` is ``core.determinism.encode_canonical``:
    ``json.dumps(doc, indent=2, ensure_ascii=False, allow_nan=False) + "\\n"``
    in strict UTF-8. It keeps the layout's key order.
"""

from __future__ import annotations

from collections.abc import Mapping
from types import MappingProxyType
from typing import Any, Final, TypeAlias

from jsinfer.core.determinism import codepoint_key, encode_canonical
from jsinfer.core.jsonvalue import JSON, JType, canonical_value, distinct_values
from jsinfer.core.schema_model import (
    ITEMIZED_KEYWORDS,
    NON_CONSTRAINT_KEYWORDS,
    SUBSCHEMA_KEYWORDS,
    ConstraintId,
    SchemaModel,
    SchemaNode,
    join_pointer,
)
from jsinfer.io.validator import Draft

__all__ = [
    "ANNOTATION_PREFIX",
    "KEYWORD_ORDER",
    "SCHEMA_DIALECT",
    "VOCABULARY_KEYWORDS",
    "EmitError",
    "PointerMap",
    "SchemaEmitter",
    "keyword_rank",
]

# The $schema URI on the root of every emitted document (2.1).
SCHEMA_DIALECT: Final = Draft.DRAFT202012.uri

# Keywords outside the draft 2020-12 vocabularies carry this prefix (2.2).
ANNOTATION_PREFIX: Final = "x-"

# The inline-provenance keyword: dropped from nodes, emitted last (task 17.2).
_PROVENANCE_KEYWORD: Final = "x-provenance"

# The fixed rank order of the draft 2020-12 vocabulary keywords. Keywords
# outside the Schema_Loader's supported set sit next to their kin.
KEYWORD_ORDER: Final[tuple[str, ...]] = (
    # Core
    "$schema",
    "$id",
    "$ref",
    "$anchor",
    "$dynamicAnchor",
    "$dynamicRef",
    "$vocabulary",
    # Annotations
    "title",
    "description",
    "$comment",
    "default",
    "examples",
    "deprecated",
    "readOnly",
    "writeOnly",
    # Values
    "type",
    "enum",
    "const",
    "format",
    "pattern",
    # Numeric bounds
    "minimum",
    "exclusiveMinimum",
    "maximum",
    "exclusiveMaximum",
    "multipleOf",
    # String length bounds and content
    "minLength",
    "maxLength",
    "contentEncoding",
    "contentMediaType",
    "contentSchema",
    # Objects
    "properties",
    "patternProperties",
    "additionalProperties",
    "unevaluatedProperties",
    "propertyNames",
    "required",
    "dependentRequired",
    "dependentSchemas",
    "minProperties",
    "maxProperties",
    # Arrays
    "prefixItems",
    "items",
    "unevaluatedItems",
    "contains",
    "minContains",
    "maxContains",
    "minItems",
    "maxItems",
    "uniqueItems",
    # Combinators and conditionals
    "anyOf",
    "oneOf",
    "allOf",
    "not",
    "if",
    "then",
    "else",
    # Definitions
    "$defs",
)

# Every keyword defined by the draft 2020-12 vocabularies (core, applicator,
# unevaluated, validation, meta-data, format, and content).
VOCABULARY_KEYWORDS: Final = frozenset(KEYWORD_ORDER)

_RANKS: Final[Mapping[str, int]] = MappingProxyType({keyword: rank for rank, keyword in enumerate(KEYWORD_ORDER)})

# Keywords never copied from a node: $schema goes on the root only, and
# x-provenance is generated from the Provenance_Records.
_DROPPED: Final = frozenset({"$schema", _PROVENANCE_KEYWORD})

# Emitted type order: null, boolean, integer, number, string, array, object.
_TYPE_ORDER: Final[Mapping[str, int]] = MappingProxyType({jtype.keyword: int(jtype) for jtype in JType})

PointerMap: TypeAlias = dict[ConstraintId, str]


class EmitError(ValueError):
    """A Schema_Model cannot be emitted as a canonical draft 2020-12 document.

    ``pointer`` is the JSON Pointer of the offending location in the model
    (``""`` for the root).
    """

    def __init__(self, pointer: str, message: str) -> None:
        super().__init__(pointer, message)  # args survive pickling
        self.pointer = pointer
        self.message = message

    def __str__(self) -> str:
        return f"{self.pointer or '(root)'}: {self.message}"


def keyword_rank(keyword: str) -> tuple[int, int, str]:
    """Sort key of a keyword in the emitted rank order (see the module docstring).

    Vocabulary keywords come first in :data:`KEYWORD_ORDER` order, then other
    ``x-`` keywords in code-point order, then ``x-provenance``. Raises
    ``ValueError`` for a keyword that is neither.
    """
    rank = _RANKS.get(keyword)
    if rank is not None:
        return (0, rank, "")
    if keyword == _PROVENANCE_KEYWORD:
        return (2, 0, "")
    if isinstance(keyword, str) and keyword.startswith(ANNOTATION_PREFIX):
        return (1, 0, codepoint_key(keyword))
    raise ValueError(
        f"keyword {keyword!r} is not defined by the draft 2020-12 vocabularies "
        f"and lacks the {ANNOTATION_PREFIX!r} prefix"
    )


class SchemaEmitter:
    """Lays out Schema_Models as canonical draft 2020-12 documents (see the module docstring)."""

    def layout(self, model: SchemaModel) -> tuple[dict[str, JSON] | bool, PointerMap]:
        """Return the emitted document and the ConstraintId → JSON Pointer map.

        The document is a fresh structure that shares nothing with the model,
        and the model is not changed. Raises :class:`EmitError` when the model
        cannot be emitted (see Checks in the module docstring).
        """
        if not isinstance(model, SchemaModel):
            raise TypeError(f"layout takes a SchemaModel, got {type(model).__name__}")
        return _Layout().run(model.root)

    def dumps(self, doc: dict[str, JSON] | bool) -> bytes:
        """Return the canonical bytes of a laid-out document (``core.determinism.encode_canonical``)."""
        if not isinstance(doc, (dict, bool)):
            raise TypeError(f"a schema document is a dict or a bool, got {type(doc).__name__}")
        return encode_canonical(doc)

    def emit(self, model: SchemaModel) -> bytes:
        """Return the canonical bytes of ``model`` (``dumps(layout(model)[0])``)."""
        doc, _ = self.layout(model)
        return self.dumps(doc)


class _Layout:
    """One layout pass: the document, the pointer map, and the nids seen so far."""

    __slots__ = ("_nids", "pointers")

    def __init__(self) -> None:
        self.pointers: PointerMap = {}
        self._nids: dict[int, str] = {}  # nid -> pointer of the node that holds it

    def run(self, root: SchemaNode) -> tuple[dict[str, JSON] | bool, PointerMap]:
        if not isinstance(root, SchemaNode):
            raise EmitError("", f"the root must be a SchemaNode, got {type(root).__name__}")
        doc = self._node(root, "", is_root=True)
        return doc, self.pointers

    # -- nodes ---------------------------------------------------------------

    def _node(self, node: SchemaNode, pointer: str, *, is_root: bool = False) -> dict[str, JSON] | bool:
        self._claim(node, pointer)
        if node.boolean is not None:
            if not isinstance(node.boolean, bool):
                raise EmitError(pointer, f"boolean must be True, False, or None, got {type(node.boolean).__name__}")
            if node.kw:
                raise EmitError(pointer, "a boolean schema has no keywords")
            return node.boolean
        kw = node.kw
        if not isinstance(kw, dict):
            raise EmitError(pointer, f"kw must be a dict, got {type(kw).__name__}")
        keywords: list[str] = []
        for keyword in kw:
            if not isinstance(keyword, str):
                raise EmitError(pointer, f"schema keywords must be str, got {type(keyword).__name__}")
            if keyword in _DROPPED:
                continue
            try:
                keyword_rank(keyword)
            except ValueError as exc:
                raise EmitError(join_pointer(pointer, keyword), str(exc)) from None
            keywords.append(keyword)
        keywords.sort(key=keyword_rank)

        out: dict[str, JSON] = {}
        if is_root:
            out["$schema"] = SCHEMA_DIALECT
        for keyword in keywords:
            out[keyword] = self._keyword(node, keyword, kw[keyword], join_pointer(pointer, keyword))
        return out

    def _claim(self, node: Any, pointer: str) -> None:
        if not isinstance(node, SchemaNode):
            raise EmitError(pointer, f"a subschema must be a SchemaNode, got {type(node).__name__}")
        first = self._nids.get(node.nid)
        if first is not None:
            raise EmitError(
                pointer,
                f"nid {node.nid} is also held by the node at {first or '(root)'!s}; "
                "every node of a model needs a distinct nid",
            )
        self._nids[node.nid] = pointer

    # -- keywords --------------------------------------------------------------

    def _keyword(self, node: SchemaNode, keyword: str, value: Any, pointer: str) -> JSON:
        shape = SUBSCHEMA_KEYWORDS.get(keyword)
        if shape == "single":
            self._constraint(node, keyword, pointer)
            return self._node(value, pointer)
        if shape == "list":
            if not isinstance(value, (list, tuple)):
                raise EmitError(pointer, f"{keyword!r} must hold a list of SchemaNodes, got {type(value).__name__}")
            if keyword in ITEMIZED_KEYWORDS:  # anyOf, oneOf: one Constraint per branch
                for index in range(len(value)):
                    self._constraint(node, keyword, join_pointer(pointer, index), index)
            else:
                self._constraint(node, keyword, pointer)
            return [self._node(child, join_pointer(pointer, index)) for index, child in enumerate(value)]
        if shape == "map":
            if not isinstance(value, dict):
                raise EmitError(pointer, f"{keyword!r} must hold a dict of SchemaNodes, got {type(value).__name__}")
            self._constraint(node, keyword, pointer)
            names = _names(value, pointer, f"{keyword!r} names")
            return {name: self._node(value[name], join_pointer(pointer, name)) for name in names}

        if keyword == "type":
            emitted: JSON = _type_value(value, pointer)
            self._constraint(node, keyword, pointer)
        elif keyword == "required":
            names = _string_set(value, pointer, "'required'")
            for index, name in enumerate(names):
                self._constraint(node, keyword, join_pointer(pointer, index), name)
            emitted = names
        elif keyword == "dependentRequired":
            emitted = _dependent_required(value, pointer)
            for key in emitted:
                self._constraint(node, keyword, join_pointer(pointer, key), key)
        elif keyword == "enum":
            if not isinstance(value, (list, tuple)):
                raise EmitError(pointer, f"'enum' must be an array, got {type(value).__name__}")
            emitted = _json(distinct_values, value, pointer)
            self._constraint(node, keyword, pointer)
        else:
            emitted = _json(canonical_value, value, pointer)
            self._constraint(node, keyword, pointer)
        return emitted

    def _constraint(self, node: SchemaNode, keyword: str, pointer: str, item: str | int | None = None) -> None:
        if keyword in NON_CONSTRAINT_KEYWORDS:
            return
        cid = ConstraintId(node.nid, keyword, item)
        # nids are distinct (checked by _claim), so a ConstraintId cannot repeat.
        self.pointers[cid] = pointer


# ---------------------------------------------------------------------------
# Value normalization
# ---------------------------------------------------------------------------


def _json(convert: Any, value: Any, pointer: str) -> Any:
    try:
        return convert(value)
    except (TypeError, ValueError) as exc:
        raise EmitError(pointer, f"not a JSON value: {exc}") from None


def _names(mapping: dict[Any, Any], pointer: str, what: str) -> list[str]:
    for name in mapping:
        if not isinstance(name, str):
            raise EmitError(pointer, f"{what} must be str, got {type(name).__name__}")
    return sorted(mapping, key=codepoint_key)


def _string_set(value: Any, pointer: str, what: str) -> list[str]:
    """Distinct names in code-point order."""
    if not isinstance(value, (list, tuple)):
        raise EmitError(pointer, f"{what} must be an array of strings, got {type(value).__name__}")
    for name in value:
        if not isinstance(name, str):
            raise EmitError(pointer, f"{what} entries must be strings, got {type(name).__name__}")
    return sorted(dict.fromkeys(value), key=codepoint_key)


def _type_value(value: Any, pointer: str) -> str | list[str]:
    """Distinct type names in the order null, boolean, …, object; a single name as a string."""
    names = [value] if isinstance(value, str) else value
    if not isinstance(names, (list, tuple)):
        raise EmitError(pointer, f"'type' must be a string or an array of strings, got {type(value).__name__}")
    for name in names:
        if not isinstance(name, str) or name not in _TYPE_ORDER:
            raise EmitError(pointer, f"{name!r} is not a JSON Schema type; expected one of {list(_TYPE_ORDER)}")
    distinct = sorted(dict.fromkeys(names), key=_TYPE_ORDER.__getitem__)
    if not distinct:
        raise EmitError(pointer, "'type' must list at least one type")
    return distinct[0] if len(distinct) == 1 else distinct


def _dependent_required(value: Any, pointer: str) -> dict[str, JSON]:
    """Entries in code-point order of their keys, each listing distinct names in code-point order."""
    if not isinstance(value, dict):
        raise EmitError(pointer, f"'dependentRequired' must be an object, got {type(value).__name__}")
    return {
        key: _string_set(value[key], join_pointer(pointer, key), "'dependentRequired' entries")
        for key in _names(value, pointer, "'dependentRequired' keys")
    }
