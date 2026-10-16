"""The Schema_Model: schema nodes, models, Constraint ids, and the equality of Requirement 2.7.

SchemaNode
    One schema position. A boolean schema has ``boolean`` set to ``True`` or
    ``False`` and an empty ``kw``; a keyword schema has ``boolean`` set to
    ``None`` and its keywords in ``kw``. Boolean schemas may stand at every
    subschema position, the root included (Requirement 2.4). Subschema-valued
    keywords (:data:`SUBSCHEMA_KEYWORDS`) hold a SchemaNode (``items``,
    ``additionalProperties``, ``propertyNames``, ``contains``, ``not``,
    ``if``, ``then``, ``else``), a list of SchemaNodes (``prefixItems``,
    ``anyOf``, ``oneOf``, ``allOf``), or a ``str``-keyed dict of SchemaNodes
    (``properties``, ``patternProperties``, ``$defs``). Every other keyword
    holds a JSON value. ``$ref`` is a pointer string, so a model is a tree
    and recursion is written with ``$ref``. A node object must appear at only
    one position: :meth:`SchemaNode.walk` raises ``ValueError`` for a node
    reached twice, which covers both sharing and cycles.

nid
    Each node carries an ``nid`` that identifies it while its keywords are
    edited (relaxation changes a node in place and its ConstraintIds stay
    valid). New nids come from a process-wide counter, so a nid is never
    given to a second node in the same process. The counter skips past every
    nid it sees: nids passed explicitly and nids of unpickled or deep-copied
    nodes. Pickling and ``copy.deepcopy`` keep nids, unlike ``id()``.

    Nids created in different processes (joblib workers) can coincide.
    :meth:`SchemaModel.renumber` gives every node of a model a fresh nid in
    canonical pre-order and remaps the provenance keys, so after it the nids
    of a model are distinct and their order is the canonical traversal order
    whatever the number of workers. Change nids only through ``renumber``.

ConstraintId
    ``(nid, keyword, item)`` names one Constraint at the granularity of the
    design's table: ``item`` is the property name of a ``required`` entry, the
    key of a ``dependentRequired`` entry, the index of an ``anyOf`` or
    ``oneOf`` branch, and ``None`` for every other Constraint keyword.
    ``$defs``, ``$schema``, annotation keywords, and container entries such
    as ``properties/<name>`` are not Constraints and are rejected
    (:data:`NON_CONSTRAINT_KEYWORDS`). ConstraintIds are totally ordered by
    nid, then keyword in code-point order, then item (``None`` first, then
    indices, then names); the item kind is fixed per keyword, so mixed items
    never meet in a comparison anyway. The Schema_Emitter maps ConstraintIds
    to JSON Pointers.

Equality (Requirement 2.7)
    Two models are equal when they hold the same keywords with equal values
    at the same JSON Pointer locations. In detail:

    - ``required``, ``enum``, and ``type`` lists, and the lists of each
      ``dependentRequired`` entry, compare as sets under JSON Schema equality
      (``core.jsonvalue``), so order and repeats do not count, ``[1]`` equals
      ``[1.0]``, and ``[1, true]`` differs from ``[true]``. A ``type`` string
      is the one-element set, so ``"string"`` equals ``["string"]``.
    - Every other non-subschema value (``const``, ``default``, ``examples``,
      bounds, ``format``, ``x-*`` annotations, ...) compares under JSON
      equality, not Python ``==``: ``1`` equals ``1.0``, ``true`` differs
      from ``1``, object members compare regardless of key order, and arrays
      such as ``examples`` compare in order.
    - Subschemas compare position by position: ``properties``,
      ``patternProperties``, and ``$defs`` by name, and ``prefixItems``,
      ``anyOf``, ``oneOf``, and ``allOf`` by index, because each element has
      its own pointer. A boolean schema equals only the same boolean schema
      (``true`` and ``{}`` are different values at their pointer).
    - Excluded: the ``$schema`` and ``x-provenance`` keywords at every node,
      nids, ``provenance``, ``ignored``, and ``dialect_assumed``. Property
      names are not keywords, so a property named ``x-provenance`` counts.
    - ``unsupported`` records compare as sets: 2.7 excludes only the
      records of ignored keywords and of an assumed dialect, and a partially
      supported model is not equal to a fully supported one.

    :meth:`SchemaNode.equality_key` returns canonical bytes of a normal form
    that has exactly this equality, so nodes can be grouped by content.
    ``SchemaNode.__eq__`` is the same equality restricted to a subtree;
    compare with ``is`` for identity. Nodes and models are mutable and
    unhashable.

Traversal
    :meth:`SchemaNode.children` lists direct subschemas with keywords in
    code-point order, dict entries in code-point order, and list entries by
    index. :meth:`SchemaNode.walk` yields ``(JSON Pointer, node)`` in that
    pre-order without recursion; :meth:`SchemaNode.resolve` follows a JSON
    Pointer (RFC 6901) to a subschema. :meth:`SchemaNode.from_json` and
    :meth:`SchemaNode.to_json` convert between node trees and plain draft
    2020-12 JSON structurally: no draft translation, no keyword filtering,
    and no canonical order, which are the Schema_Loader's and the
    Schema_Emitter's jobs.
"""

from __future__ import annotations

import copy
import re
import threading
from collections.abc import Iterable, Iterator, Mapping
from dataclasses import dataclass, field, replace
from types import MappingProxyType
from typing import TYPE_CHECKING, Any, Final, Literal, TypeAlias

from jsinfer.core.determinism import canonical_bytes, codepoint_key
from jsinfer.core.jsonvalue import JSON, distinct_values

if TYPE_CHECKING:
    # core.provenance imports this module at runtime, so import it for typing only.
    from jsinfer.core.provenance import ProvenanceRecord

__all__ = [
    "EXCLUDED_KEYWORDS",
    "ITEMIZED_KEYWORDS",
    "NON_CONSTRAINT_KEYWORDS",
    "SET_COMPARED_KEYWORDS",
    "SUBSCHEMA_KEYWORDS",
    "ConstraintId",
    "SchemaModel",
    "SchemaNode",
    "SubschemaShape",
    "escape_token",
    "join_pointer",
    "new_nid",
    "split_pointer",
]

SubschemaShape: TypeAlias = Literal["single", "list", "map"]

# How each subschema-valued keyword holds its subschemas. All other keywords
# hold JSON values.
SUBSCHEMA_KEYWORDS: Final[Mapping[str, SubschemaShape]] = MappingProxyType(
    {
        "$defs": "map",
        "additionalProperties": "single",
        "allOf": "list",
        "anyOf": "list",
        "contains": "single",
        "else": "single",
        "if": "single",
        "items": "single",
        "not": "single",
        "oneOf": "list",
        "patternProperties": "map",
        "prefixItems": "list",
        "properties": "map",
        "propertyNames": "single",
        "then": "single",
    }
)

# Keywords whose lists compare as sets (for dependentRequired, the list of each entry).
SET_COMPARED_KEYWORDS: Final = frozenset({"dependentRequired", "enum", "required", "type"})

# Keywords left out of the comparison at every node.
EXCLUDED_KEYWORDS: Final = frozenset({"$schema", "x-provenance"})

# Keywords that are never Constraints: $defs, $schema, the root $id, and the
# annotation keywords the Schema_Loader retains (Requirement 2.10).
NON_CONSTRAINT_KEYWORDS: Final = frozenset(
    {
        "$comment",
        "$defs",
        "$id",
        "$schema",
        "default",
        "deprecated",
        "description",
        "examples",
        "readOnly",
        "title",
        "writeOnly",
        "x-provenance",
    }
)

# Keywords whose Constraints are per entry, with the type of the entry item.
ITEMIZED_KEYWORDS: Final[Mapping[str, type]] = MappingProxyType(
    {"anyOf": int, "dependentRequired": str, "oneOf": int, "required": str}
)


# ---------------------------------------------------------------------------
# nids
# ---------------------------------------------------------------------------


class _NidCounter:
    """Process-wide source of nids: 1, 2, 3, ..., skipping past every reserved nid."""

    __slots__ = ("_lock", "_next")

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._next = 1

    def take(self) -> int:
        with self._lock:
            nid = self._next
            self._next = nid + 1
            return nid

    def reserve(self, nid: int) -> None:
        with self._lock:
            if nid >= self._next:
                self._next = nid + 1


_NIDS: Final = _NidCounter()


def new_nid() -> int:
    """Return a fresh nid, greater than every nid made, passed, or unpickled in this process so far."""
    return _NIDS.take()


def _check_nid(nid: Any, what: str) -> None:
    if isinstance(nid, bool) or not isinstance(nid, int):
        raise TypeError(f"{what} must be an int, got {type(nid).__name__}")
    if nid < 0:
        raise ValueError(f"{what} must be non-negative, got {nid}")


# ---------------------------------------------------------------------------
# JSON Pointers
# ---------------------------------------------------------------------------


_BAD_ESCAPE_RE: Final = re.compile(r"~(?![01])")


def escape_token(token: str | int) -> str:
    """Return one JSON Pointer reference token: ``~`` becomes ``~0`` and ``/`` becomes ``~1``."""
    if isinstance(token, bool) or not isinstance(token, (str, int)):
        raise TypeError(f"pointer token must be str or int, got {type(token).__name__}")
    if isinstance(token, int):
        if token < 0:
            raise ValueError(f"array index token must be non-negative, got {token}")
        return str(token)
    return token.replace("~", "~0").replace("/", "~1")


def join_pointer(base: str, *tokens: str | int) -> str:
    """Return ``base`` extended by ``tokens``.

    For example ``join_pointer("", "properties", "a/b")`` is ``/properties/a~1b``.
    """
    return base + "".join("/" + escape_token(token) for token in tokens)


def split_pointer(pointer: str) -> list[str]:
    """Return the unescaped reference tokens of a JSON Pointer (``""`` is the root).

    Raises ``ValueError`` when the pointer neither is empty nor starts with
    ``/``, or holds a ``~`` not followed by ``0`` or ``1``.
    """
    if not isinstance(pointer, str):
        raise TypeError(f"JSON Pointer must be str, got {type(pointer).__name__}")
    if not pointer:
        return []
    if not pointer.startswith("/"):
        raise ValueError(f"JSON Pointer {pointer!r} must be empty or start with '/'")
    bad = _BAD_ESCAPE_RE.search(pointer)
    if bad is not None:
        raise ValueError(f"JSON Pointer {pointer!r} has a '~' not followed by '0' or '1' at index {bad.start()}")
    return [token.replace("~1", "/").replace("~0", "~") for token in pointer[1:].split("/")]


def _array_index(token: str, length: int) -> int | None:
    """The index an RFC 6901 array token names, or None when it is malformed or out of range."""
    if not token or not token.isascii() or not token.isdigit() or (len(token) > 1 and token[0] == "0"):
        return None
    index = int(token)
    return index if index < length else None


# ---------------------------------------------------------------------------
# SchemaNode
# ---------------------------------------------------------------------------


def _check_keyword_value(keyword: Any, value: Any) -> None:
    """Check the key and, for subschema keywords, the container shape (not the subschemas themselves)."""
    if not isinstance(keyword, str):
        raise TypeError(f"schema keywords must be str, got {type(keyword).__name__}")
    shape = SUBSCHEMA_KEYWORDS.get(keyword)
    if shape is None:
        return
    if shape == "single":
        if not isinstance(value, SchemaNode):
            raise TypeError(f"{keyword!r} must hold a SchemaNode, got {type(value).__name__}")
    elif shape == "list":
        if not isinstance(value, (list, tuple)):
            raise TypeError(f"{keyword!r} must hold a list of SchemaNodes, got {type(value).__name__}")
        for child in value:
            if not isinstance(child, SchemaNode):
                raise TypeError(f"{keyword!r} entries must be SchemaNodes, got {type(child).__name__}")
    else:
        if not isinstance(value, dict):
            raise TypeError(f"{keyword!r} must hold a dict of SchemaNodes, got {type(value).__name__}")
        for name, child in value.items():
            if not isinstance(name, str):
                raise TypeError(f"{keyword!r} names must be str, got {type(name).__name__}")
            if not isinstance(child, SchemaNode):
                raise TypeError(f"{keyword!r} entries must be SchemaNodes, got {type(child).__name__}")


@dataclass(eq=False)
class SchemaNode:
    """A boolean schema (``boolean`` is True or False) or a keyword schema (``kw``).

    Subschema-valued keywords hold SchemaNodes (see the module docstring);
    construction checks their container shapes. ``nid`` defaults to a fresh
    nid. ``==`` is the equality of Requirement 2.7 on the subtree and
    ignores nids.
    """

    boolean: bool | None = None
    kw: dict[str, Any] = field(default_factory=dict)
    nid: int = field(default_factory=new_nid)

    def __post_init__(self) -> None:
        _check_nid(self.nid, "SchemaNode nid")
        if self.boolean is not None and not isinstance(self.boolean, bool):
            raise TypeError(f"boolean must be True, False, or None, got {type(self.boolean).__name__}")
        if not isinstance(self.kw, dict):
            raise TypeError(f"kw must be a dict, got {type(self.kw).__name__}")
        if self.boolean is not None and self.kw:
            raise ValueError("a boolean schema has no keywords")
        for keyword, value in self.kw.items():
            _check_keyword_value(keyword, value)
        _NIDS.reserve(self.nid)

    def __setstate__(self, state: dict[str, Any]) -> None:
        # Unpickled and deep-copied nodes keep their nid; new nodes in this
        # process must not take it.
        self.__dict__.update(state)
        _NIDS.reserve(self.nid)

    # -- conversion ------------------------------------------------------------

    @classmethod
    def from_json(cls, schema: Any) -> SchemaNode:
        """Build a node tree from a draft 2020-12 schema (a ``dict`` or a ``bool``).

        Nids are assigned in pre-order. Non-subschema values are deep-copied.
        Raises ``TypeError`` for a schema that is neither an object nor a
        boolean, or a subschema keyword with the wrong container (such as the
        pre-2020-12 array form of ``items``).
        """
        if isinstance(schema, bool):
            return cls(boolean=schema)
        if not isinstance(schema, dict):
            raise TypeError(f"a schema is a JSON object or a boolean, got {type(schema).__name__}")
        node = cls()
        for keyword, value in schema.items():
            if not isinstance(keyword, str):
                raise TypeError(f"schema keywords must be str, got {type(keyword).__name__}")
            shape = SUBSCHEMA_KEYWORDS.get(keyword)
            if shape is None:
                node.kw[keyword] = copy.deepcopy(value)
            elif shape == "single":
                node.kw[keyword] = cls.from_json(value)
            elif shape == "list":
                if not isinstance(value, (list, tuple)):
                    raise TypeError(f"{keyword!r} must be an array of schemas, got {type(value).__name__}")
                node.kw[keyword] = [cls.from_json(child) for child in value]
            else:
                if not isinstance(value, dict):
                    raise TypeError(f"{keyword!r} must be an object of schemas, got {type(value).__name__}")
                node.kw[keyword] = {name: cls.from_json(child) for name, child in value.items()}
        return node

    def to_json(self) -> JSON:
        """Return the subtree as plain JSON, keywords in ``kw`` order (not the emitter's canonical form)."""
        if self.boolean is not None:
            return self.boolean
        out: dict[str, JSON] = {}
        for keyword, value in self.kw.items():
            shape = SUBSCHEMA_KEYWORDS.get(keyword)
            _check_keyword_value(keyword, value)
            if shape is None:
                out[keyword] = copy.deepcopy(value)
            elif shape == "single":
                out[keyword] = value.to_json()
            elif shape == "list":
                out[keyword] = [child.to_json() for child in value]
            else:
                out[keyword] = {name: child.to_json() for name, child in value.items()}
        return out

    # -- traversal -------------------------------------------------------------

    def children(self) -> Iterator[tuple[str, str | int | None, SchemaNode]]:
        """Yield ``(keyword, entry, child)`` for each direct subschema in canonical order.

        ``entry`` is the name for ``properties``, ``patternProperties``, and
        ``$defs``, the index for list keywords, and ``None`` for
        single-subschema keywords. Keywords and names are in code-point order.
        """
        if self.boolean is not None:
            return
        kw = self.kw
        for keyword in sorted(kw, key=codepoint_key):
            shape = SUBSCHEMA_KEYWORDS.get(keyword)
            if shape is None:
                continue
            value = kw[keyword]
            _check_keyword_value(keyword, value)
            if shape == "single":
                yield keyword, None, value
            elif shape == "list":
                for index, child in enumerate(value):
                    yield keyword, index, child
            else:
                for name in sorted(value, key=codepoint_key):
                    yield keyword, name, value[name]

    def walk(self, base: str = "") -> Iterator[tuple[str, SchemaNode]]:
        """Yield ``(pointer, node)`` for this node and every subschema, in canonical pre-order.

        Pointers are JSON Pointers relative to the walk root, prefixed by
        ``base``. Raises ``ValueError`` when one node object is reached twice
        (shared or cyclic structure).
        """
        seen: set[int] = set()  # id()s, for membership only; never iterated or ordered
        stack: list[tuple[str, SchemaNode]] = [(base, self)]
        while stack:
            pointer, node = stack.pop()
            marker = id(node)
            if marker in seen:
                raise ValueError(
                    f"node nid {node.nid} is reached twice (again at {pointer!r}); a Schema_Model is a tree"
                )
            seen.add(marker)
            yield pointer, node
            entries = [
                (join_pointer(pointer, keyword) if entry is None else join_pointer(pointer, keyword, entry), child)
                for keyword, entry, child in node.children()
            ]
            stack.extend(reversed(entries))

    def resolve(self, pointer: str) -> SchemaNode:
        """Return the subschema at a JSON Pointer relative to this node.

        Raises ``KeyError`` when the pointer does not lead to a subschema (a
        missing keyword or entry, a non-subschema keyword, or a container such
        as ``/properties`` itself) and ``ValueError`` for a malformed pointer.
        """
        tokens = split_pointer(pointer)
        node = self
        i = 0
        while i < len(tokens):
            keyword = tokens[i]
            shape = SUBSCHEMA_KEYWORDS.get(keyword)
            if node.boolean is not None or shape is None or keyword not in node.kw:
                raise KeyError(f"{pointer!r} does not resolve to a subschema: no subschema keyword {keyword!r}")
            value = node.kw[keyword]
            _check_keyword_value(keyword, value)
            if shape == "single":
                node = value
                i += 1
                continue
            if i + 1 == len(tokens):
                raise KeyError(f"{pointer!r} points at the {keyword!r} container, not at a subschema")
            entry = tokens[i + 1]
            if shape == "list":
                index = _array_index(entry, len(value))
                if index is None:
                    raise KeyError(f"{pointer!r} does not resolve: {keyword!r} has no entry {entry!r}")
                node = value[index]
            else:
                if entry not in value:
                    raise KeyError(f"{pointer!r} does not resolve: {keyword!r} has no entry {entry!r}")
                node = value[entry]
            i += 2
        return node

    # -- equality --------------------------------------------------------------

    def equality_key(self) -> bytes:
        """Canonical bytes that are equal exactly when the subtrees are equal under Requirement 2.7."""
        return canonical_bytes(_normal_form(self))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, SchemaNode):
            return NotImplemented
        return self is other or self.equality_key() == other.equality_key()

    __hash__ = None  # type: ignore[assignment]


def _normal_form(node: SchemaNode) -> JSON:
    """The JSON normal form whose canonical bytes define equality (see the module docstring)."""
    if node.boolean is not None:
        if not isinstance(node.boolean, bool):
            raise TypeError(f"boolean must be True, False, or None, got {type(node.boolean).__name__}")
        if node.kw:
            raise ValueError(f"boolean schema nid {node.nid} has keywords")
        return node.boolean
    return {
        keyword: _normal_value(keyword, value) for keyword, value in node.kw.items() if keyword not in EXCLUDED_KEYWORDS
    }


def _normal_value(keyword: str, value: Any) -> JSON:
    _check_keyword_value(keyword, value)
    shape = SUBSCHEMA_KEYWORDS.get(keyword)
    if shape == "single":
        return _normal_form(value)
    if shape == "list":
        return [_normal_form(child) for child in value]
    if shape == "map":
        return {name: _normal_form(child) for name, child in value.items()}
    if keyword == "type" and isinstance(value, str):
        return [value]
    if keyword in SET_COMPARED_KEYWORDS and isinstance(value, (list, tuple)):
        return distinct_values(value)
    if keyword == "dependentRequired" and isinstance(value, dict):
        return {
            name: distinct_values(entry) if isinstance(entry, (list, tuple)) else entry for name, entry in value.items()
        }
    # Plain JSON: canonical_bytes applies JSON equality (1 is 1.0, true is not 1).
    return value


# ---------------------------------------------------------------------------
# ConstraintId
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ConstraintId:
    """One Constraint: ``(17, "required", "email")``, ``(3, "anyOf", 0)``, or ``(5, "type")``.

    ``item`` is required for :data:`ITEMIZED_KEYWORDS` (with the listed type)
    and must be ``None`` otherwise. Keywords in
    :data:`NON_CONSTRAINT_KEYWORDS` raise ``ValueError``.
    """

    nid: int
    keyword: str
    item: str | int | None = None

    def __post_init__(self) -> None:
        _check_nid(self.nid, "ConstraintId nid")
        keyword = self.keyword
        if not isinstance(keyword, str):
            raise TypeError(f"ConstraintId keyword must be str, got {type(keyword).__name__}")
        if not keyword:
            raise ValueError("ConstraintId keyword must be non-empty")
        if keyword in NON_CONSTRAINT_KEYWORDS:
            raise ValueError(f"{keyword!r} is not a Constraint keyword")
        item = self.item
        expected = ITEMIZED_KEYWORDS.get(keyword)
        if expected is None:
            if item is not None:
                raise ValueError(f"{keyword!r} is a keyword-level Constraint; item must be None, got {item!r}")
        elif expected is int:
            if isinstance(item, bool) or not isinstance(item, int) or item < 0:
                raise ValueError(f"{keyword!r} Constraints are per branch; item must be an index >= 0, got {item!r}")
        elif not isinstance(item, str):
            raise ValueError(f"{keyword!r} Constraints are per entry; item must be a str name, got {item!r}")

    def _order(self) -> tuple[int, str, int, int | str]:
        item = self.item
        if item is None:
            return (self.nid, self.keyword, 0, 0)
        return (self.nid, self.keyword, 1 if isinstance(item, int) else 2, item)

    def __lt__(self, other: object) -> bool:
        if not isinstance(other, ConstraintId):
            return NotImplemented
        return self._order() < other._order()

    def __le__(self, other: object) -> bool:
        if not isinstance(other, ConstraintId):
            return NotImplemented
        return self._order() <= other._order()

    def __gt__(self, other: object) -> bool:
        if not isinstance(other, ConstraintId):
            return NotImplemented
        return self._order() > other._order()

    def __ge__(self, other: object) -> bool:
        if not isinstance(other, ConstraintId):
            return NotImplemented
        return self._order() >= other._order()


# ---------------------------------------------------------------------------
# SchemaModel
# ---------------------------------------------------------------------------


def _records(entries: Iterable[Any], what: str) -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    for entry in entries:
        if not isinstance(entry, (tuple, list)) or len(entry) != 2:
            raise TypeError(f"{what} entries are (pointer, keyword) pairs, got {entry!r}")
        pointer, keyword = entry
        if not isinstance(pointer, str) or not isinstance(keyword, str):
            raise TypeError(f"{what} entries are (pointer, keyword) pairs of str, got {entry!r}")
        out.append((pointer, keyword))
    return out


@dataclass(eq=False)
class SchemaModel:
    """A schema tree with the Schema_Loader's records and the Provenance_Records.

    ``ignored`` and ``unsupported`` hold ``(JSON Pointer, keyword)`` pairs;
    a model with no unsupported record is fully supported (Requirement 2.6).
    ``dialect_assumed`` records that no recognized ``$schema`` was declared
    (2.11). ``provenance`` maps ConstraintIds to ProvenanceRecords
    (``core.provenance``). ``==`` is the equality of Requirement 2.7.
    """

    root: SchemaNode
    dialect_assumed: bool = False
    ignored: list[tuple[str, str]] = field(default_factory=list)
    unsupported: list[tuple[str, str]] = field(default_factory=list)
    provenance: dict[ConstraintId, ProvenanceRecord] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.root, SchemaNode):
            raise TypeError(f"root must be a SchemaNode, got {type(self.root).__name__}")
        if not isinstance(self.dialect_assumed, bool):
            raise TypeError(f"dialect_assumed must be a bool, got {type(self.dialect_assumed).__name__}")
        self.ignored = _records(self.ignored, "ignored")
        self.unsupported = _records(self.unsupported, "unsupported")
        if not isinstance(self.provenance, dict):
            raise TypeError(f"provenance must be a dict, got {type(self.provenance).__name__}")
        for cid in self.provenance:
            if not isinstance(cid, ConstraintId):
                raise TypeError(f"provenance keys must be ConstraintIds, got {type(cid).__name__}")

    @property
    def fully_supported(self) -> bool:
        """True when the Schema_Loader recorded no unsupported construct."""
        return not self.unsupported

    # -- traversal -------------------------------------------------------------

    def walk(self) -> Iterator[tuple[str, SchemaNode]]:
        """Yield ``(pointer, node)`` for every node in canonical pre-order (see :meth:`SchemaNode.walk`)."""
        return self.root.walk()

    def resolve(self, pointer: str) -> SchemaNode:
        """Return the subschema at a JSON Pointer from the root (see :meth:`SchemaNode.resolve`)."""
        return self.root.resolve(pointer)

    def index(self) -> dict[int, SchemaNode]:
        """Return every node by nid, in canonical pre-order; raises ``ValueError`` on a repeated nid."""
        nodes: dict[int, SchemaNode] = {}
        where: dict[int, str] = {}
        for pointer, node in self.walk():
            if node.nid in nodes:
                raise ValueError(f"nid {node.nid} appears at {where[node.nid]!r} and at {pointer!r}")
            nodes[node.nid] = node
            where[node.nid] = pointer
        return nodes

    def renumber(self) -> dict[int, int]:
        """Give every node a fresh nid in canonical pre-order and remap ``provenance``.

        Returns the map from old to new nids. Afterwards the nids of the model
        are distinct and increase in canonical pre-order, so ConstraintId
        order follows the tree and not the order in which nodes were made.
        Provenance keys whose nid is not in the model are kept unchanged.
        Raises ``ValueError`` (and changes nothing) when two nodes share a nid
        or one node is reached twice.
        """
        nodes = list(self.index().values())
        for cid in self.provenance:
            _NIDS.reserve(cid.nid)  # fresh nids must not meet a stale key either
        mapping: dict[int, int] = {}
        for node in nodes:
            new = new_nid()
            mapping[node.nid] = new
            node.nid = new
        self.provenance = {
            (replace(cid, nid=mapping[cid.nid]) if cid.nid in mapping else cid): record
            for cid, record in self.provenance.items()
        }
        return mapping

    # -- equality --------------------------------------------------------------

    def equality_key(self) -> bytes:
        """Canonical bytes that are equal exactly when the models are equal under Requirement 2.7."""
        unsupported = sorted(set(self.unsupported), key=lambda e: (codepoint_key(e[0]), codepoint_key(e[1])))
        return canonical_bytes([_normal_form(self.root), unsupported])

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, SchemaModel):
            return NotImplemented
        return self is other or self.equality_key() == other.equality_key()

    __hash__ = None  # type: ignore[assignment]
