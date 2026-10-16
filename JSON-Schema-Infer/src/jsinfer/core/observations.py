"""The columnar ObservationStore: shape-path columns, ``flatten_chunk``, and ``merge_partials``.

Shape paths
    Documents are flattened once into one :class:`Column` per *shape path*,
    which is a logical Path with every Key step replaced by ``{*}``. The
    members of an object at shape path ``S`` go to column ``S + ({*},)`` with
    their key in the ``key`` field; the elements of an array go to
    ``S + ([*],)`` with their index in the ``pos`` field. ``$.user.email`` and
    ``$.user.name`` are therefore rows of the single column ``${*}{*}``, told
    apart by key, and a Map with 100k distinct keys gives one column rather
    than 100k Paths. Logical Paths are row-index views (``core.views``).

Rows
    Every document contributes exactly one row to the root column ``()``, so
    root row ``i`` is document ``i``. Within a document, nodes are visited in
    pre-order, with object members in code-point key order and array
    elements in index order, and each column receives its rows in visit
    order. Documents follow each other in index order. As a result:

    - ``doc`` and ``parent`` are non-decreasing in every column, so the
      children of one parent row form one contiguous run that
      ``np.searchsorted`` finds (:meth:`Column.runs`);
    - within a run, object members are in ascending key id and array
      elements in position order, with ``pos`` equal to ``0, 1, …``;
    - the store depends only on the documents as JSON values and on their
      order. Member order in the source text, chunk boundaries, and the
      worker count do not affect any array (Requirement 27.6).

Fields
    ``parent``  int64 row in the parent column (``-1`` in the root column)
    ``doc``     int32 document index
    ``jtype``   uint8 :class:`~jsinfer.core.jsonvalue.JType`
    ``key``     int32 key id for object members, else ``-1``
    ``pos``     int32 array index for array elements, else ``-1``
    ``value``   int32 value id for scalars (null, boolean, integer, number,
                string), else ``-1``
    ``size``    int32 code points (string), elements (array), or members
                (object); ``-1`` for null, boolean, and numbers

Interning
    :func:`flatten_chunk` interns keys and scalar values with chunk-local ids.
    :func:`merge_partials` merges the chunk interners with
    :meth:`ValueTable.merge`, so global key ids follow code-point order and
    value ids follow canonical value order. Ids therefore depend only on the
    sets of distinct keys and values, not on chunking or document order.
    Integral numbers share ids with equal integers (``1`` and ``1.0``), and
    booleans never share ids with numbers.

All arrays of a merged store are read-only. Flags (``core.stats.FlagSet``)
and the ViewStats cache hang off the store, and views (``core.views``)
read the columns.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from types import MappingProxyType
from typing import TYPE_CHECKING, Any, Final, TypeAlias

import numpy as np

from jsinfer.core.determinism import (
    RANK_ARRAY,
    RANK_OBJECT,
    RANK_STRING,
    codepoint_key,
    json_type_rank,
)
from jsinfer.core.jsonvalue import ValueInterner, ValueTable
from jsinfer.core.paths import ELEM_STEP, KEY, MAPVAL_STEP, ROOT, Step, path_sort_key, render_path

if TYPE_CHECKING:
    from jsinfer.core.stats import FlagSet, StatsCache

__all__ = [
    "FIELDS",
    "FIELD_DTYPES",
    "Column",
    "ObservationStore",
    "PartialStore",
    "ShapePath",
    "build_store",
    "check_shape_path",
    "flatten_chunk",
    "isin_sorted",
    "merge_partials",
    "ranges_to_rows",
    "shape_of",
]

# A Path whose steps are all ``[*]`` or ``{*}``; ``()`` is the root column.
ShapePath: TypeAlias = tuple[Step, ...]

FIELDS: Final = ("parent", "doc", "jtype", "key", "pos", "value", "size")
FIELD_DTYPES: Final[Mapping[str, type[np.generic]]] = MappingProxyType(
    {
        "parent": np.int64,
        "doc": np.int32,
        "jtype": np.uint8,
        "key": np.int32,
        "pos": np.int32,
        "value": np.int32,
        "size": np.int32,
    }
)

_EMPTY_ROWS: Final = np.empty(0, dtype=np.int64)
_EMPTY_ROWS.setflags(write=False)


def shape_of(path: Sequence[Step]) -> ShapePath:
    """Return the shape path of a logical Path: every Key step becomes ``{*}``."""
    return tuple(MAPVAL_STEP if step.kind == KEY else step for step in path)


def check_shape_path(shape: Any) -> ShapePath:
    """Return ``shape`` if it is a tuple of ``[*]`` and ``{*}`` steps; raise otherwise."""
    if not isinstance(shape, tuple):
        raise TypeError(f"a shape path is a tuple of steps, got {type(shape).__name__}")
    for step in shape:
        if not isinstance(step, Step) or step.kind == KEY:
            raise ValueError(f"a shape path holds only [*] and {{*}} steps, got {step!r}")
    return shape


# ---------------------------------------------------------------------------
# Sorted-row helpers shared by views and stats
# ---------------------------------------------------------------------------


def isin_sorted(values: np.ndarray, sorted_unique: np.ndarray) -> np.ndarray:
    """Return a bool mask of the entries of ``values`` that occur in ``sorted_unique``.

    ``sorted_unique`` must be sorted ascending without duplicates. The test is
    one ``np.searchsorted``, O(len(values) · log len(sorted_unique)).
    """
    if len(sorted_unique) == 0 or len(values) == 0:
        return np.zeros(len(values), dtype=bool)
    idx = np.searchsorted(sorted_unique, values)
    np.minimum(idx, len(sorted_unique) - 1, out=idx)
    return sorted_unique[idx] == values


def ranges_to_rows(lo: np.ndarray, hi: np.ndarray) -> np.ndarray:
    """Concatenate the integer ranges ``[lo[i], hi[i])`` into one int64 array.

    The ranges come from :meth:`Column.runs` over sorted parent rows, so the
    result is sorted and has no duplicates.
    """
    lengths = hi - lo
    total = int(lengths.sum())
    if total == 0:
        return _EMPTY_ROWS
    starts = np.cumsum(lengths) - lengths
    return np.arange(total, dtype=np.int64) + np.repeat(lo - starts, lengths)


# ---------------------------------------------------------------------------
# Columns and stores
# ---------------------------------------------------------------------------


class Column:
    """The Observations at one shape path, as parallel read-only numpy arrays.

    See the module docstring for the fields. :meth:`by_key` gives the sorted
    rows with one key id; its index is built on first use.
    """

    __slots__ = ("shape", *FIELDS, "_key_order", "_key_sorted")

    shape: ShapePath
    parent: np.ndarray
    doc: np.ndarray
    jtype: np.ndarray
    key: np.ndarray
    pos: np.ndarray
    value: np.ndarray
    size: np.ndarray

    def __init__(self, shape: ShapePath, **arrays: np.ndarray) -> None:
        if set(arrays) != set(FIELDS):
            raise TypeError(f"a Column needs exactly the fields {list(FIELDS)}, got {sorted(arrays)}")
        self.shape = check_shape_path(shape)
        n = None
        for name in FIELDS:
            arr = np.ascontiguousarray(arrays[name], dtype=FIELD_DTYPES[name])
            if arr.ndim != 1 or (n is not None and len(arr) != n):
                raise ValueError(f"column fields must be 1-D arrays of one length; {name!r} has shape {arr.shape}")
            n = len(arr)
            arr.setflags(write=False)
            setattr(self, name, arr)
        self._key_order: np.ndarray | None = None
        self._key_sorted: np.ndarray | None = None

    def __len__(self) -> int:
        return len(self.parent)

    def runs(self, parent_rows: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """Return ``(lo, hi)`` with ``[lo[i], hi[i])`` the rows whose parent is ``parent_rows[i]``.

        ``parent`` is non-decreasing, so each run is found with two
        ``np.searchsorted`` calls. For sorted ``parent_rows`` the runs are
        disjoint and ascending.
        """
        lo = np.searchsorted(self.parent, parent_rows, side="left")
        hi = np.searchsorted(self.parent, parent_rows, side="right")
        return lo, hi

    def by_key(self, kid: int) -> np.ndarray:
        """Return the sorted rows whose key id is ``kid`` (a read-only array)."""
        if isinstance(kid, bool) or not isinstance(kid, (int, np.integer)) or kid < 0:
            raise ValueError(f"key id must be a non-negative integer, got {kid!r}")
        if self._key_order is None:
            # A stable sort keeps rows ascending within each key.
            order = np.argsort(self.key, kind="stable")
            order.setflags(write=False)
            self._key_sorted = self.key[order]
            self._key_order = order
        assert self._key_sorted is not None
        lo = int(np.searchsorted(self._key_sorted, kid, side="left"))
        hi = int(np.searchsorted(self._key_sorted, kid, side="right"))
        return self._key_order[lo:hi]

    def __repr__(self) -> str:
        return f"<Column {render_path(self.shape)}: {len(self)} rows>"


@dataclass(frozen=True, eq=False)
class PartialStore:
    """The flattened Observations of one chunk of consecutive documents.

    ``key`` and ``value`` fields hold chunk-local ids of ``keys`` and
    ``values``; ``parent`` holds rows within the chunk; ``doc`` holds global
    document indices ``first_index …``. Partial stores pickle, so joblib
    workers can return them.
    """

    first_index: int
    n_docs: int
    keys: ValueInterner
    values: ValueInterner
    columns: Mapping[ShapePath, Column]

    def __repr__(self) -> str:
        return f"<PartialStore of documents {self.first_index}..{self.first_index + self.n_docs - 1}>"


class ObservationStore:
    """All Observations of a Document_Collection in shape-path columns.

    ``columns`` maps shape paths to columns in canonical Path order and
    always contains the root column ``()``. ``keys`` is the key table (ids
    in code-point order) and ``values`` the scalar value table (ids in
    canonical value order). ``flags`` is the store's one
    :class:`~jsinfer.core.stats.FlagSet`; ``stats_cache`` caches ViewStats by
    view digest and flag epoch.
    """

    __slots__ = ("columns", "keys", "values", "n_docs", "_flags", "_stats_cache")

    def __init__(self, columns: Mapping[ShapePath, Column], keys: ValueTable, values: ValueTable, n_docs: int) -> None:
        # Deferred import: core.stats imports this module.
        from jsinfer.core.stats import FlagSet, StatsCache

        if ROOT not in columns or len(columns[ROOT]) != n_docs:
            raise ValueError("the root column must exist and hold one row per document")
        ordered = {shape: columns[shape] for shape in sorted(columns, key=path_sort_key)}
        self.columns: Mapping[ShapePath, Column] = MappingProxyType(ordered)
        self.keys = keys
        self.values = values
        self.n_docs = n_docs
        self._flags = FlagSet()
        self._stats_cache = StatsCache()

    def __getstate__(self) -> tuple[Any, ...]:
        # The stats cache is not pickled; a copy starts with an empty one.
        return (dict(self.columns), self.keys, self.values, self.n_docs, self._flags)

    def __setstate__(self, state: tuple[Any, ...]) -> None:
        from jsinfer.core.stats import StatsCache

        columns, self.keys, self.values, self.n_docs, self._flags = state
        self.columns = MappingProxyType(columns)
        self._stats_cache = StatsCache()

    @property
    def flags(self) -> FlagSet:
        """The store's FlagSet. Mutate it in place; it is never replaced, so its epoch only grows."""
        return self._flags

    @property
    def stats_cache(self) -> StatsCache:
        return self._stats_cache

    def column(self, shape: ShapePath) -> Column:
        """Return the column at ``shape``; raises ``KeyError`` when no Observation has that shape."""
        try:
            return self.columns[shape]
        except KeyError:
            raise KeyError(f"no column at shape path {render_path(shape)}") from None

    def key_id(self, name: str) -> int | None:
        """Return the id of object key ``name``, or ``None`` when no object has that key."""
        if not isinstance(name, str):
            raise TypeError(f"object keys are str, got {type(name).__name__}")
        try:
            return self.keys.id_of(name)
        except KeyError:
            return None

    def key_name(self, kid: int) -> str:
        """Return the object key with id ``kid``."""
        name = self.keys[kid]
        assert isinstance(name, str)
        return name

    @property
    def n_observations(self) -> int:
        return sum(len(column) for column in self.columns.values())

    def __repr__(self) -> str:
        return (
            f"<ObservationStore: {self.n_docs} documents, {self.n_observations} observations, "
            f"{len(self.columns)} columns, {len(self.keys)} keys, {len(self.values)} values>"
        )


# ---------------------------------------------------------------------------
# Flattening
# ---------------------------------------------------------------------------


def _check_index(first_index: Any) -> int:
    if isinstance(first_index, bool) or not isinstance(first_index, (int, np.integer)) or first_index < 0:
        raise ValueError(f"first_index must be a non-negative integer, got {first_index!r}")
    return int(first_index)


def _sorted_member_names(obj: dict[Any, Any]) -> list[str]:
    for name in obj:
        if not isinstance(name, str):
            raise TypeError(f"object keys must be str, got {type(name).__name__}")
    return sorted(obj, key=codepoint_key)


def flatten_chunk(docs: Iterable[Any], first_index: int) -> PartialStore:
    """Flatten consecutive documents ``first_index, first_index + 1, …`` into a PartialStore.

    The walk is iterative, so deeply nested documents do not hit the
    recursion limit. Values that are not JSON raise ``TypeError`` (for
    example non-``str`` keys) or ``ValueError`` (NaN and infinities).
    """
    base = _check_index(first_index)
    keys = ValueInterner()
    values = ValueInterner()
    intern_key = keys.intern
    intern_value = values.intern

    # Shapes are numbered as they appear; child_ids give the [*] and {*} child of each.
    shapes: list[ShapePath] = [ROOT]
    tables: list[list[tuple[int, int, int, int, int, int, int]]] = [[]]
    elem_child: list[int] = [-1]
    map_child: list[int] = [-1]

    def child_shape(sid: int, children: list[int], step: Step) -> int:
        cid = children[sid]
        if cid < 0:
            cid = len(shapes)
            shapes.append(shapes[sid] + (step,))
            tables.append([])
            elem_child.append(-1)
            map_child.append(-1)
            children[sid] = cid
        return cid

    n_docs = 0
    for doc in docs:
        d = base + n_docs
        n_docs += 1
        # (value, shape id, parent row, key id, position); popped in pre-order.
        stack: list[tuple[Any, int, int, int, int]] = [(doc, 0, -1, -1, -1)]
        while stack:
            value, sid, parent, kid, pos = stack.pop()
            table = tables[sid]
            row = len(table)
            rank = json_type_rank(value)
            if rank == RANK_OBJECT:
                names = _sorted_member_names(value)
                if names:
                    cid = child_shape(sid, map_child, MAPVAL_STEP)
                    stack.extend((value[name], cid, row, intern_key(name), -1) for name in reversed(names))
                table.append((parent, d, rank, kid, pos, -1, len(names)))
            elif rank == RANK_ARRAY:
                size = len(value)
                if size:
                    cid = child_shape(sid, elem_child, ELEM_STEP)
                    stack.extend((value[i], cid, row, -1, i) for i in range(size - 1, -1, -1))
                table.append((parent, d, rank, kid, pos, -1, size))
            else:
                size = len(value) if rank == RANK_STRING else -1
                table.append((parent, d, rank, kid, pos, intern_value(value), size))

    columns = {shape: _column_from_table(shape, table) for shape, table in zip(shapes, tables)}
    return PartialStore(first_index=base, n_docs=n_docs, keys=keys, values=values, columns=columns)


def _column_from_table(shape: ShapePath, table: list[tuple[int, ...]]) -> Column:
    data = np.array(table, dtype=np.int64).reshape(len(table), len(FIELDS))
    return Column(shape, **{name: data[:, j] for j, name in enumerate(FIELDS)})


def _remap(local: np.ndarray, remap: np.ndarray) -> np.ndarray:
    """Map chunk-local ids to global ids, keeping ``-1`` ("none") as ``-1``."""
    out = local.astype(np.int32, copy=True)
    present = local >= 0
    out[present] = remap[local[present]]
    return out


def merge_partials(parts: Sequence[PartialStore]) -> ObservationStore:
    """Merge chunk stores, given in chunk order, into one ObservationStore.

    The chunks must cover documents ``0 … n-1`` contiguously and in order.
    Keys and scalar values are re-interned with ids in canonical order, and
    rows are concatenated in chunk order, so the result is identical for
    every chunking of the same document sequence.
    """
    parts = list(parts)
    expected = 0
    for j, part in enumerate(parts):
        if part.first_index != expected:
            raise ValueError(
                f"partial stores must cover documents contiguously from 0 in chunk order; "
                f"part {j} starts at document {part.first_index}, expected {expected}"
            )
        expected += part.n_docs
    key_table, key_remaps = ValueTable.merge([part.keys for part in parts])
    value_table, value_remaps = ValueTable.merge([part.values for part in parts])

    shapes = sorted({ROOT, *(shape for part in parts for shape in part.columns)}, key=path_sort_key)
    columns: dict[ShapePath, Column] = {}
    for shape in shapes:
        pieces: dict[str, list[np.ndarray]] = {name: [] for name in FIELDS}
        parent_offset = 0  # rows of the parent column in earlier parts
        for j, part in enumerate(parts):
            col = part.columns.get(shape)
            if col is not None and len(col):
                pieces["parent"].append(col.parent + parent_offset if shape else col.parent)
                pieces["doc"].append(col.doc)
                pieces["jtype"].append(col.jtype)
                pieces["key"].append(_remap(col.key, key_remaps[j]))
                pieces["pos"].append(col.pos)
                pieces["value"].append(_remap(col.value, value_remaps[j]))
                pieces["size"].append(col.size)
            if shape:
                parent_col = part.columns.get(shape[:-1])
                parent_offset += len(parent_col) if parent_col is not None else 0
        arrays = {
            name: np.concatenate(chunks) if chunks else np.empty(0, dtype=FIELD_DTYPES[name])
            for name, chunks in pieces.items()
        }
        columns[shape] = Column(shape, **arrays)
    return ObservationStore(columns, key_table, value_table, expected)


def build_store(docs: Iterable[Any], chunk_size: int = 2000) -> ObservationStore:
    """Flatten ``docs`` in fixed-size chunks, sequentially, and merge them.

    The result does not depend on ``chunk_size``; the pipeline passes the
    Resource_Parameter ``ingest.chunk_size`` and may flatten chunks in
    parallel, which gives the same store.
    """
    if isinstance(chunk_size, bool) or not isinstance(chunk_size, (int, np.integer)) or chunk_size < 1:
        raise ValueError(f"chunk_size must be an integer >= 1, got {chunk_size!r}")
    parts: list[PartialStore] = []
    chunk: list[Any] = []
    first = 0
    for doc in docs:
        chunk.append(doc)
        if len(chunk) == chunk_size:
            parts.append(flatten_chunk(chunk, first))
            first += len(chunk)
            chunk = []
    if chunk:
        parts.append(flatten_chunk(chunk, first))
    return merge_partials(parts)
