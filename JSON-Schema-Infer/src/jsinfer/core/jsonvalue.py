"""JSON values: JType, canonical keys and order, equality, and interning.

A JSON value here is what the Ingestor produces: ``None``, ``bool``, ``int``,
a finite ``float``, ``str``, ``list``, or ``dict`` with ``str`` keys. Tuples
are accepted as arrays, as in :mod:`jsinfer.core.determinism`. NaN and
infinities raise ``ValueError``; other objects (sets, bytes, numpy integers,
non-``str`` keys) raise ``TypeError``.

JType
    ``NULL < BOOLEAN < INTEGER < NUMBER < STRING < ARRAY < OBJECT``. A number
    with a zero fractional part is an INTEGER however it is written, so ``1``,
    ``1.0``, ``-0.0``, and ``1e3`` are integers (Requirement 13.2), and NUMBER
    holds only non-integral numbers. ``bool`` is BOOLEAN and never a number.
    The values are the type ranks of ``core.determinism``, and their order is
    the order of emitted ``type`` lists.

Canonical key
    ``canonical_key(v)`` is ``(type_rank, payload)``. The payload is ``None``
    for null, ``0`` or ``1`` for booleans, ``int(v)`` for integers (exact at
    any size, with no float coercion), the ``float`` for other numbers, the
    string itself, and ``canonical_bytes(v)`` for arrays and objects. Within
    one rank every payload has the same type, so keys are totally ordered and
    payloads of different types are never compared.

    Two values have equal keys exactly when they are equal under JSON Schema
    instance equality, which draft 2020-12 also uses for ``enum`` and
    ``const`` (Requirements 12.1, 15.1): null equals null; booleans compare by
    value and never equal a number (``true`` is not ``1``); numbers compare by
    mathematical value (``1`` equals ``1.0``); strings compare by code points,
    without Unicode normalization; arrays compare elementwise; objects compare
    by key set and member equality. :func:`json_equal` implements this
    definition directly.

Canonical value order
    Ascending canonical key: type rank first, then integers by value,
    non-integral numbers by value (so ``1 < 2 < 0.5``), strings in code-point
    order, and arrays and objects by their canonical bytes. This is the
    emitted ``enum`` order (Requirement 2.3); ``core.determinism.sorted_values``
    sorts by the same key.

Canonical representative
    ``canonical_value(v)`` writes integral numbers as ``int``, tuples as
    lists, and object keys in code-point order. Values with equal keys have
    identical representatives, and the representative is what gets emitted.

Interning
    :class:`ValueInterner` gives chunk-local ids in first-seen order.
    :class:`ValueTable` holds distinct values with ids in canonical value
    order, and :meth:`ValueTable.merge` combines chunk interners into one table
    plus an id remap per chunk. Global ids therefore depend only on the set of
    distinct values, not on chunking, worker count, or document order
    (Requirements 27.5, 27.6). Lookups use dicts keyed by canonical keys; no
    id or order depends on ``hash()``.

Python's ``==`` is not JSON equality (``True == 1`` and ``(1,) == (True,)``
hold in Python), so compare values with :func:`json_equal` or
:func:`canonical_key`.
"""

from __future__ import annotations

import math
import operator
from collections.abc import Iterable, Sequence
from enum import IntEnum
from typing import Any, Final, TypeAlias

import numpy as np

from jsinfer.core.determinism import (
    RANK_ARRAY,
    RANK_BOOLEAN,
    RANK_INTEGER,
    RANK_NULL,
    RANK_NUMBER,
    RANK_OBJECT,
    RANK_STRING,
    codepoint_key,
    json_type_rank,
    value_sort_key,
)

__all__ = [
    "JSON",
    "CanonicalKey",
    "JType",
    "ValueInterner",
    "ValueTable",
    "canonical_key",
    "canonical_value",
    "distinct_values",
    "json_equal",
    "jtype_of",
]

JSON: TypeAlias = None | bool | int | float | str | list["JSON"] | tuple["JSON", ...] | dict[str, "JSON"]

# (type_rank, payload); the payload type is fixed per rank (see the module docstring).
CanonicalKey: TypeAlias = tuple[int, None | int | float | str | bytes]


class JType(IntEnum):
    """The JSON type of a value, ordered as in emitted ``type`` lists.

    INTEGER is every number with a zero fractional part (``1``, ``1.0``,
    ``1e3``); NUMBER is every other number.
    """

    NULL = RANK_NULL
    BOOLEAN = RANK_BOOLEAN
    INTEGER = RANK_INTEGER
    NUMBER = RANK_NUMBER
    STRING = RANK_STRING
    ARRAY = RANK_ARRAY
    OBJECT = RANK_OBJECT

    @property
    def keyword(self) -> str:
        """The JSON Schema ``type`` name, for example ``"integer"``."""
        return self.name.lower()

    @property
    def is_scalar(self) -> bool:
        """True for null, boolean, integer, number, and string."""
        return self <= JType.STRING

    @classmethod
    def from_keyword(cls, name: str) -> JType:
        """Return the JType for a JSON Schema ``type`` name; raises ``ValueError`` for others."""
        jtype = _BY_KEYWORD.get(name) if isinstance(name, str) else None
        if jtype is None:
            raise ValueError(f"unknown JSON Schema type {name!r}; expected one of {list(_BY_KEYWORD)}")
        return jtype


_JTYPES: Final = tuple(JType)  # indexed by type rank
_BY_KEYWORD: Final = {jtype.keyword: jtype for jtype in JType}


def jtype_of(value: Any) -> JType:
    """Return the JType of a JSON value (integral numbers are INTEGER, booleans BOOLEAN)."""
    return _JTYPES[json_type_rank(value)]


def canonical_key(value: Any) -> CanonicalKey:
    """Return the canonical key ``(type_rank, payload)`` of a JSON value.

    Equal keys mean equal values under JSON Schema instance and ``enum``
    equality, and sorting by the key gives canonical value order. The result
    equals ``core.determinism.value_sort_key(value)``; exact builtin scalars
    take a shorter path because interning calls this once per Observation.
    """
    cls = type(value)
    if cls is str:
        return (RANK_STRING, value)
    if cls is int:
        return (RANK_INTEGER, value)
    if cls is float and math.isfinite(value):
        return (RANK_INTEGER, int(value)) if value.is_integer() else (RANK_NUMBER, value)
    if value is None:
        return (RANK_NULL, None)
    if cls is bool:
        return (RANK_BOOLEAN, int(value))
    # Containers, subclasses, and invalid values (which raise there).
    return value_sort_key(value)


def _check_keys(obj: dict[Any, Any]) -> None:
    for key in obj:
        if not isinstance(key, str):
            raise TypeError(f"object keys must be str, got {type(key).__name__}")


def canonical_value(value: Any) -> JSON:
    """Return the canonical representative of a JSON value.

    Integral numbers become ``int`` (``1.0`` → ``1``, ``-0.0`` → ``0``),
    other numbers ``float``, tuples lists, and object keys are put in
    code-point order. Booleans stay booleans. Big integers are kept exactly.
    """
    rank = json_type_rank(value)
    if rank == RANK_NULL:
        return None
    if rank == RANK_BOOLEAN:
        return bool(value)
    if rank == RANK_INTEGER:
        return int(value)
    if rank == RANK_NUMBER:
        return float(value)
    if rank == RANK_STRING:
        return str(value)
    if rank == RANK_ARRAY:
        return [canonical_value(item) for item in value]
    _check_keys(value)
    return {key: canonical_value(value[key]) for key in sorted(value, key=codepoint_key)}


def json_equal(a: Any, b: Any) -> bool:
    """Return whether two JSON values are equal under JSON Schema instance equality.

    This is also the equality of ``enum`` and ``const``. Numbers compare by
    mathematical value with integers compared exactly (``2**64 + 1`` does not
    equal ``float(2**64)``); booleans never equal numbers; objects compare by
    key set and members, arrays elementwise. The walk is iterative, so deep
    nesting does not hit the recursion limit. It stops at the first
    difference and raises ``TypeError`` or ``ValueError`` for a non-JSON value
    it reaches before then.
    """
    pending: list[tuple[Any, Any]] = [(a, b)]
    while pending:
        x, y = pending.pop()
        rank = json_type_rank(x)
        if rank != json_type_rank(y):
            return False
        if rank == RANK_ARRAY:
            if len(x) != len(y):
                return False
            pending.extend(zip(x, y))
        elif rank == RANK_OBJECT:
            _check_keys(x)
            _check_keys(y)
            if x.keys() != y.keys():
                return False
            pending.extend((x[key], y[key]) for key in x)
        elif rank in (RANK_BOOLEAN, RANK_INTEGER):
            if int(x) != int(y):
                return False
        elif rank != RANK_NULL and x != y:  # non-integral numbers and strings
            return False
    return True


def distinct_values(values: Iterable[Any]) -> list[JSON]:
    """Return the distinct values under ``enum`` equality, as representatives in canonical value order."""
    return list(ValueTable(values).values())


# ---------------------------------------------------------------------------
# Interning
# ---------------------------------------------------------------------------


class ValueInterner:
    """Chunk-local value interning: one dense id per canonical key, in first-seen order.

    ``intern(1)``, ``intern(1.0)``, and ``intern(1e0)`` return the same id;
    ``intern(True)`` returns a different one. The stored representative is
    ``canonical_value`` of the first occurrence, which is the same for every
    equal value. Interners pickle, so joblib workers can return them.
    """

    __slots__ = ("_ids", "_values")

    def __init__(self, values: Iterable[Any] = ()) -> None:
        self._ids: dict[CanonicalKey, int] = {}
        self._values: list[JSON] = []
        for value in values:
            self.intern(value)

    def intern(self, value: Any) -> int:
        """Return the local id of ``value``, assigning the next id if it is new."""
        key = canonical_key(value)
        ids = self._ids
        vid = ids.get(key)
        if vid is None:
            vid = len(ids)
            ids[key] = vid
            self._values.append(canonical_value(value))
        return vid

    def __len__(self) -> int:
        return len(self._ids)

    def keys(self) -> list[CanonicalKey]:
        """Canonical keys in local id order."""
        return list(self._ids)

    def values(self) -> list[JSON]:
        """Representatives in local id order (shared objects; do not mutate them)."""
        return list(self._values)

    def __repr__(self) -> str:
        return f"<ValueInterner of {len(self)} values>"


class ValueTable:
    """Distinct JSON values with ids ``0 … n-1`` in canonical value order.

    Because ids follow canonical value order, sorting ids sorts values, and a
    table depends only on the set of distinct values. Tables compare equal
    when they hold the same values under JSON equality.
    """

    __slots__ = ("_index", "_keys", "_values")

    _keys: tuple[CanonicalKey, ...]
    _values: tuple[JSON, ...]
    _index: dict[CanonicalKey, int]

    def __init__(self, values: Iterable[Any] = ()) -> None:
        reps: dict[CanonicalKey, JSON] = {}
        for value in values:
            key = canonical_key(value)
            if key not in reps:
                reps[key] = canonical_value(value)
        self._assign(reps)

    def _assign(self, reps: dict[CanonicalKey, JSON]) -> None:
        keys = sorted(reps)
        self._keys = tuple(keys)
        self._values = tuple(reps[key] for key in keys)
        self._index = {key: vid for vid, key in enumerate(keys)}

    @classmethod
    def merge(cls, parts: Sequence[ValueInterner | ValueTable]) -> tuple[ValueTable, list[np.ndarray]]:
        """Merge chunk interners (or tables) into one table in canonical order.

        Returns the table and, for each part, an ``int32`` array ``remap`` with
        ``remap[local_id] == global_id``. The table is the same for every way of
        splitting the values into parts and for every part order.
        """
        reps: dict[CanonicalKey, JSON] = {}
        for part in parts:
            for key, value in zip(part.keys(), part.values()):
                reps.setdefault(key, value)
        table = cls.__new__(cls)
        table._assign(reps)
        index = table._index
        remaps = [np.fromiter((index[key] for key in part.keys()), dtype=np.int32, count=len(part)) for part in parts]
        return table, remaps

    def __len__(self) -> int:
        return len(self._keys)

    def __getitem__(self, vid: int) -> JSON:
        """Return the representative with id ``vid``; negative ids raise ``IndexError``."""
        return self._values[self._check_id(vid)]

    def key(self, vid: int) -> CanonicalKey:
        """Return the canonical key with id ``vid``."""
        return self._keys[self._check_id(vid)]

    def _check_id(self, vid: int) -> int:
        i = operator.index(vid)
        if not 0 <= i < len(self._keys):
            raise IndexError(f"value id {i} out of range for a table of {len(self._keys)} values")
        return i

    def id_of(self, value: Any) -> int:
        """Return the id of ``value`` (any equal spelling); raises ``KeyError`` if absent."""
        key = canonical_key(value)
        try:
            return self._index[key]
        except KeyError:
            raise KeyError(f"value not in table: {value!r}") from None

    def __contains__(self, value: Any) -> bool:
        return canonical_key(value) in self._index

    def keys(self) -> tuple[CanonicalKey, ...]:
        """Canonical keys in id order (ascending)."""
        return self._keys

    def values(self) -> tuple[JSON, ...]:
        """Representatives in id order, i.e. canonical value order (shared; do not mutate)."""
        return self._values

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ValueTable):
            return NotImplemented
        return self._keys == other._keys

    __hash__ = None  # type: ignore[assignment]

    def __repr__(self) -> str:
        return f"<ValueTable of {len(self)} values>"
