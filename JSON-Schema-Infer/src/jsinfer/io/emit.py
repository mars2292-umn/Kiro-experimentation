"""Schema_Emitter: canonical layout, the pointer map, bytes, and the explanation file (Requirements 2.1–2.3, 26.5–26.8).

Task 6.1 implements the layout and the schema bytes; task 17.2 adds the
explanation file and inline ``x-provenance``.

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
      last (see Inline provenance).
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
      from the Provenance_Records instead (26.6; see Inline provenance).
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
    explanation file keys records by these pointers.

Bytes
    :meth:`SchemaEmitter.dumps` is ``core.determinism.encode_canonical``:
    ``json.dumps(doc, indent=2, ensure_ascii=False, allow_nan=False) + "\\n"``
    in strict UTF-8. It keeps the layout's key order.

Explanation file (26.5, 26.7, 26.8)
    :meth:`SchemaEmitter.explanation_file` pairs the pointer map of a layout
    with ``SchemaModel.provenance`` and returns an :class:`ExplanationFile`;
    :meth:`SchemaEmitter.explanation` returns its bytes. The document is::

        {"format": 1, "records": {<JSON Pointer>: <ProvenanceRecord.to_json()>, ...}}

    with the pointers in code-point order, written by the same canonical
    encoder as the schema. Every Constraint of the layout must have exactly
    one record, and every record must belong to a Constraint of the layout,
    so each key resolves to exactly one Constraint and every Constraint is
    the target of exactly one key (26.7). A Constraint without a record, a
    record that names no Constraint of the layout (for example one left
    behind by a removal), or a value that is not a ProvenanceRecord raises
    :class:`EmitError`. ``x-provenance`` is not a Constraint, so no key
    names it. Records serialize with keys in code-point order at every level
    (``core.provenance``), so the bytes depend only on the pointers and the
    record contents, and reading a file with
    ``io.load.load_explanation`` and writing it again with
    :meth:`ExplanationFile.dumps` gives the same bytes (26.8).

    The explanation file of a boolean root, which holds no Constraint, is
    ``{"format": 1, "records": {}}``.

Inline provenance (26.6, 26.11)
    With ``inline_provenance=True`` (the Run_Configuration setting
    ``output.inline_provenance``), :meth:`SchemaEmitter.layout` and
    :meth:`SchemaEmitter.emit` add one ``x-provenance`` keyword, last, to
    each schema object that holds at least one Constraint (the nid of the
    Constraint is the object's nid). Its value maps the JSON Pointer of each
    of those Constraints, relative to the schema object, to the
    Constraint's ``ProvenanceRecord.to_json()``, in code-point order. The
    relative pointer starts with ``/`` (``/type``, ``/required/0``,
    ``/anyOf/1``, ``/dependentRequired/a~1b``), so the schema object's own
    pointer followed by the key is the Constraint's key in the explanation
    file, and the two hold the same record. Schema objects with no
    Constraint and boolean schemas get no ``x-provenance``. The same
    record checks as for the explanation file apply.

    ``x-provenance`` is an annotation outside the 2020-12 vocabularies, so
    validators ignore it and the verdicts do not change. It is not a
    Constraint and sorts after every other keyword, so the pointer map is
    the same with and without it, and with ``inline_provenance=False`` (the
    default) the bytes are those of a layout without provenance. The
    Schema_Loader keeps it as an ``x-`` keyword, which 2.7 equality and this
    emitter leave out, so both forms load to equal models.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any, Final, TypeAlias

from jsinfer.core.determinism import codepoint_key, encode_canonical
from jsinfer.core.jsonvalue import JSON, JType, canonical_value, distinct_values
from jsinfer.core.provenance import ProvenanceRecord
from jsinfer.core.schema_model import (
    ITEMIZED_KEYWORDS,
    NON_CONSTRAINT_KEYWORDS,
    SUBSCHEMA_KEYWORDS,
    ConstraintId,
    SchemaModel,
    SchemaNode,
    join_pointer,
    split_pointer,
)
from jsinfer.io.validator import Draft

__all__ = [
    "ANNOTATION_PREFIX",
    "EXPLANATION_FORMAT",
    "KEYWORD_ORDER",
    "PROVENANCE_KEYWORD",
    "SCHEMA_DIALECT",
    "VOCABULARY_KEYWORDS",
    "EmitError",
    "ExplanationFile",
    "PointerMap",
    "SchemaEmitter",
    "keyword_rank",
]

# The $schema URI on the root of every emitted document (2.1).
SCHEMA_DIALECT: Final = Draft.DRAFT202012.uri

# Keywords outside the draft 2020-12 vocabularies carry this prefix (2.2).
ANNOTATION_PREFIX: Final = "x-"

# The inline-provenance keyword: dropped from nodes, generated from the
# Provenance_Records when inline provenance is on, and emitted last (26.6).
PROVENANCE_KEYWORD: Final = "x-provenance"

# The version written in, and the only one accepted from, the "format" member
# of an explanation file (26.5).
EXPLANATION_FORMAT: Final = 1

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
_DROPPED: Final = frozenset({"$schema", PROVENANCE_KEYWORD})

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
    if keyword == PROVENANCE_KEYWORD:
        return (2, 0, "")
    if isinstance(keyword, str) and keyword.startswith(ANNOTATION_PREFIX):
        return (1, 0, codepoint_key(keyword))
    raise ValueError(
        f"keyword {keyword!r} is not defined by the draft 2020-12 vocabularies "
        f"and lacks the {ANNOTATION_PREFIX!r} prefix"
    )


@dataclass(frozen=True, eq=False, slots=True)
class ExplanationFile:
    """The explanation file of a run: each Provenance_Record keyed by the JSON Pointer of its Constraint (26.5).

    ``records`` maps JSON Pointers into the run's top-level emitted schema
    to :class:`~jsinfer.core.provenance.ProvenanceRecord` values. It is
    stored as a new dict in code-point order of the pointers; treat it as
    read-only. Every key must be a syntactically valid, non-empty JSON
    Pointer: the empty pointer names the root schema object, which is not
    a Constraint. Whether a key resolves to a Constraint of a given schema
    is not checked here. Raises ``TypeError`` or ``ValueError`` otherwise.

    Two explanation files are equal when they hold equal records under
    equal pointers. Use :meth:`SchemaEmitter.explanation_file` to build one
    from a model, ``io.load.load_explanation`` to read one, and
    :meth:`dumps` to write one.
    """

    records: Mapping[str, ProvenanceRecord]

    def __post_init__(self) -> None:
        records = self.records
        if not isinstance(records, Mapping):
            raise TypeError(f"ExplanationFile records must be a mapping, got {type(records).__name__}")
        for pointer, record in records.items():
            _check_record_pointer(pointer)
            if not isinstance(record, ProvenanceRecord):
                raise TypeError(
                    f"ExplanationFile record at {pointer!r} must be a ProvenanceRecord, got {type(record).__name__}"
                )
        ordered = {pointer: records[pointer] for pointer in sorted(records, key=codepoint_key)}
        object.__setattr__(self, "records", ordered)

    def to_json(self) -> dict[str, JSON]:
        """``{"format": 1, "records": {pointer: record JSON}}``, every object in code-point key order."""
        return {
            "format": EXPLANATION_FORMAT,
            "records": {pointer: record.to_json() for pointer, record in self.records.items()},
        }

    def dumps(self) -> bytes:
        """The canonical bytes of the file (``core.determinism.encode_canonical`` of :meth:`to_json`)."""
        return encode_canonical(self.to_json())

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ExplanationFile):
            return NotImplemented
        return self is other or dict(self.records) == dict(other.records)

    __hash__ = None  # type: ignore[assignment]


def _check_record_pointer(pointer: Any) -> None:
    if not isinstance(pointer, str):
        raise TypeError(f"explanation file keys must be str JSON Pointers, got {type(pointer).__name__}")
    split_pointer(pointer)  # raises ValueError for a malformed pointer
    if not pointer:
        raise ValueError("the empty JSON Pointer names the root schema object, which is not a Constraint")


class SchemaEmitter:
    """Lays out Schema_Models as canonical draft 2020-12 documents (see the module docstring)."""

    def layout(
        self, model: SchemaModel, *, inline_provenance: bool = False
    ) -> tuple[dict[str, JSON] | bool, PointerMap]:
        """Return the emitted document and the ConstraintId → JSON Pointer map.

        With ``inline_provenance``, each schema object that holds a
        Constraint also gets an ``x-provenance`` keyword built from
        ``model.provenance`` (see Inline provenance in the module docstring);
        the pointer map is the same either way. The document is a fresh
        structure that shares nothing with the model, and the model is not
        changed. Raises :class:`EmitError` when the model cannot be emitted
        (see Checks in the module docstring) or, with ``inline_provenance``,
        when its records do not match its Constraints one to one.
        """
        if not isinstance(model, SchemaModel):
            raise TypeError(f"layout takes a SchemaModel, got {type(model).__name__}")
        if not isinstance(inline_provenance, bool):
            raise TypeError(f"inline_provenance must be a bool, got {type(inline_provenance).__name__}")
        provenance = model.provenance if inline_provenance else None
        doc, pointers = _Layout(provenance).run(model.root)
        if provenance is not None:
            _check_no_stale_records(provenance, pointers)
        return doc, pointers

    def dumps(self, doc: dict[str, JSON] | bool) -> bytes:
        """Return the canonical bytes of a laid-out document (``core.determinism.encode_canonical``)."""
        if not isinstance(doc, (dict, bool)):
            raise TypeError(f"a schema document is a dict or a bool, got {type(doc).__name__}")
        return encode_canonical(doc)

    def emit(self, model: SchemaModel, *, inline_provenance: bool = False) -> bytes:
        """Return the canonical bytes of ``model`` (``dumps(layout(model, inline_provenance=...)[0])``)."""
        doc, _ = self.layout(model, inline_provenance=inline_provenance)
        return self.dumps(doc)

    def explanation_file(self, model: SchemaModel, pointers: Mapping[ConstraintId, str]) -> ExplanationFile:
        """Return the explanation file of ``model``, keyed by the pointers of its layout (26.5, 26.7).

        ``pointers`` is the map returned by :meth:`layout` for ``model``.
        Raises :class:`EmitError` when a Constraint in ``pointers`` has no
        record, a record is not a ProvenanceRecord, a record names a
        ConstraintId missing from ``pointers``, or two Constraints share a
        pointer.
        """
        if not isinstance(model, SchemaModel):
            raise TypeError(f"explanation_file takes a SchemaModel, got {type(model).__name__}")
        if not isinstance(pointers, Mapping):
            raise TypeError(
                f"pointers must be a mapping of ConstraintIds to JSON Pointers, got {type(pointers).__name__}"
            )
        provenance = model.provenance
        records: dict[str, ProvenanceRecord] = {}
        for cid, pointer in pointers.items():
            if not isinstance(cid, ConstraintId) or not isinstance(pointer, str):
                raise TypeError(
                    f"pointers must map ConstraintIds to str JSON Pointers, got {cid!r} -> {type(pointer).__name__}"
                )
            if pointer in records:
                raise EmitError(pointer, f"two Constraints share this pointer; {cid!r} is the second")
            records[pointer] = _record_of(provenance, cid, pointer)
        _check_no_stale_records(provenance, pointers)
        try:
            return ExplanationFile(records)
        except (TypeError, ValueError) as exc:
            raise EmitError("", f"invalid explanation file key: {exc}") from None

    def explanation(self, model: SchemaModel, pointers: Mapping[ConstraintId, str]) -> bytes:
        """Return the canonical bytes of :meth:`explanation_file` (26.5, 26.8)."""
        return self.explanation_file(model, pointers).dumps()


def _record_of(provenance: Mapping[ConstraintId, Any], cid: ConstraintId, pointer: str) -> ProvenanceRecord:
    """The record of the Constraint ``cid`` at ``pointer``; EmitError when it has none (26.1)."""
    record = provenance.get(cid)
    if record is None:
        raise EmitError(pointer, f"the Constraint {cid!r} has no Provenance_Record")
    if not isinstance(record, ProvenanceRecord):
        raise EmitError(pointer, f"the record of {cid!r} must be a ProvenanceRecord, got {type(record).__name__}")
    return record


def _check_no_stale_records(provenance: Mapping[ConstraintId, Any], pointers: Mapping[ConstraintId, str]) -> None:
    """Raise EmitError when a record names a ConstraintId that the layout does not hold (26.7)."""
    stale = sorted(cid for cid in provenance if cid not in pointers)
    if stale:
        more = f" (and {len(stale) - 1} more)" if len(stale) > 1 else ""
        raise EmitError("", f"the Provenance_Record of {stale[0]!r}{more} names no Constraint of the emitted schema")


class _Layout:
    """One layout pass: the document, the pointer map, and the nids seen so far.

    ``provenance`` holds the records to embed as ``x-provenance``, or is
    ``None`` when inline provenance is off.
    """

    __slots__ = ("_held", "_nids", "_provenance", "pointers")

    def __init__(self, provenance: Mapping[ConstraintId, Any] | None = None) -> None:
        self.pointers: PointerMap = {}
        self._nids: dict[int, str] = {}  # nid -> pointer of the node that holds it
        self._provenance = provenance
        # nid -> (pointer, ConstraintId) of each Constraint the node holds, while
        # the node is laid out; filled only for inline provenance.
        self._held: dict[int, list[tuple[str, ConstraintId]]] = {}

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
        held = self._held.pop(node.nid, None)
        if self._provenance is not None and held:
            # x-provenance ranks after every other keyword, so it goes last.
            out[PROVENANCE_KEYWORD] = self._inline(pointer, held)
        return out

    def _inline(self, base: str, held: list[tuple[str, ConstraintId]]) -> dict[str, JSON]:
        """The ``x-provenance`` value of the node at ``base``: relative pointer -> record JSON, sorted."""
        assert self._provenance is not None
        entries: dict[str, JSON] = {}
        for pointer, cid in held:
            entries[pointer[len(base) :]] = _record_of(self._provenance, cid, pointer).to_json()
        return {key: entries[key] for key in sorted(entries, key=codepoint_key)}

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
        if self._provenance is not None:
            self._held.setdefault(node.nid, []).append((pointer, cid))


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
