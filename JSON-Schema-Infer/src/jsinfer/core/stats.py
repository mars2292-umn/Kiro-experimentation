"""Flagged_Items (:class:`FlagSet`) and cached per-view statistics (:class:`ViewStats`).

Flags
    A :class:`FlagSet` holds flagged documents and flagged items, an item
    being one ``(column, row)`` Observation of the store. A row is excluded
    from every view when it is flagged itself, belongs to a flagged
    document, or has a flagged ancestor, so flagging a value excludes its
    whole subtree while the rest of its document stays in Constraint
    inference (Requirement 17.4). Each store owns one FlagSet
    (``store.flags``). Its ``epoch`` grows by one whenever the flags change,
    and views and caches compare epochs to notice changes.

ViewStats
    :func:`view_stats` returns the statistics of a view's non-flagged rows:
    type counts, value-id counts, string-length and array-length histograms,
    object-size histograms, and per-key presence bitsets over the objects.
    Everything is computed with vectorized numpy and holds integer counts
    only, so the statistics of a multiset of Observations are the same for
    every document order, chunking, and worker count (Requirements 19.6,
    27.6). Results are cached in ``store.stats_cache`` by ``(view digest,
    flag epoch)``: a view digest identifies its row set by content, and a
    change of flags starts a new epoch whose entries are computed afresh.

Presence describes the key sets of the view's objects themselves. A
flagged member value is excluded from the member's own views, but its key
still counts as present in its (non-flagged) parent object, so flags affect
only the flagged item's subtree.
"""

from __future__ import annotations

from collections.abc import Callable, Iterable
from dataclasses import dataclass
from functools import cached_property
from typing import TYPE_CHECKING, Any, Final, NamedTuple

import numpy as np

from jsinfer.core.jsonvalue import JType
from jsinfer.core.observations import (
    ShapePath,
    check_shape_path,
    isin_sorted,
    ranges_to_rows,
)
from jsinfer.core.paths import MAPVAL_STEP, path_sort_key

if TYPE_CHECKING:
    from jsinfer.core.observations import ObservationStore
    from jsinfer.core.views import View

__all__ = [
    "FlagSet",
    "Histogram",
    "StatsCache",
    "ViewStats",
    "as_row_array",
    "compute_stats",
    "view_stats",
]

N_JTYPES: Final = len(JType)

_EMPTY_I64: Final = np.empty(0, dtype=np.int64)
_EMPTY_I64.setflags(write=False)


def _readonly(arr: np.ndarray) -> np.ndarray:
    arr.setflags(write=False)
    return arr


def as_row_array(rows: Any, what: str = "rows") -> np.ndarray:
    """Return ``rows`` as a sorted, duplicate-free, read-only int64 array of non-negative integers."""
    arr = np.asarray(rows)
    if arr.size == 0:
        return _EMPTY_I64
    if arr.ndim != 1:
        raise ValueError(f"{what} must be a 1-D sequence of integers, got shape {arr.shape}")
    if arr.dtype == np.bool_ or not np.issubdtype(arr.dtype, np.integer):
        raise TypeError(f"{what} must be integers, got dtype {arr.dtype}")
    if arr.min() < 0:
        raise ValueError(f"{what} must be non-negative, got {int(arr.min())}")
    return _readonly(np.unique(arr.astype(np.int64, copy=False)))


# ---------------------------------------------------------------------------
# Flags
# ---------------------------------------------------------------------------


class FlagSet:
    """Flagged documents and flagged ``(column, row)`` items, with a change epoch.

    Flags only accumulate until :meth:`clear` drops them all (used when flags
    would cover every document). Every call that changes the set increments
    ``epoch``; calls that add nothing new leave it unchanged.
    """

    __slots__ = ("_docs", "_items", "_epoch")

    def __init__(self) -> None:
        self._docs: np.ndarray = _EMPTY_I64
        self._items: dict[ShapePath, np.ndarray] = {}
        self._epoch = 0

    @property
    def epoch(self) -> int:
        return self._epoch

    @property
    def is_empty(self) -> bool:
        return len(self._docs) == 0 and not self._items

    @property
    def documents(self) -> np.ndarray:
        """Flagged document indices, ascending (read-only)."""
        return self._docs

    @property
    def n_documents(self) -> int:
        return len(self._docs)

    @property
    def n_items(self) -> int:
        return sum(len(rows) for rows in self._items.values())

    def item_rows(self, column: ShapePath) -> np.ndarray:
        """Flagged rows of ``column``, ascending (read-only; empty when none)."""
        return self._items.get(column, _EMPTY_I64)

    def items(self) -> list[tuple[ShapePath, np.ndarray]]:
        """Flagged items as ``(column, rows)`` pairs in canonical column order."""
        return [(column, self._items[column]) for column in sorted(self._items, key=path_sort_key)]

    def flag_documents(self, docs: Iterable[int] | np.ndarray) -> int:
        """Flag documents by index; returns how many were not flagged before."""
        return self._bump(self._add_docs(as_row_array(docs, "document indices")))

    def flag_items(self, column: ShapePath, rows: Iterable[int] | np.ndarray) -> int:
        """Flag rows of ``column`` as Flagged_Items; returns how many were not flagged before."""
        return self._bump(self._add_items(check_shape_path(column), as_row_array(rows)))

    def flag_view(self, view: View) -> int:
        """Flag every row of ``view`` (for example a winning candidate's rejects); returns the new count."""
        return self._bump(sum(self._add_items(part.column, part.rows) for part in view.parts))

    def update(self, other: FlagSet) -> int:
        """Add every flag of ``other``; returns the number of new documents plus new items."""
        added = self._add_docs(other._docs)
        added += sum(self._add_items(column, rows) for column, rows in other.items())
        return self._bump(added)

    def _bump(self, added: int) -> int:
        """Start a new epoch when anything was added (one epoch per public call)."""
        if added:
            self._epoch += 1
        return added

    def _add_docs(self, new: np.ndarray) -> int:
        if len(new) == 0:
            return 0
        merged = np.union1d(self._docs, new) if len(self._docs) else new
        added = len(merged) - len(self._docs)
        if added:
            self._docs = _readonly(merged)
        return added

    def _add_items(self, column: ShapePath, new: np.ndarray) -> int:
        if len(new) == 0:
            return 0
        old = self._items.get(column, _EMPTY_I64)
        merged = np.union1d(old, new) if len(old) else new
        added = len(merged) - len(old)
        if added:
            self._items[column] = _readonly(merged)
        return added

    def clear(self) -> None:
        """Drop every flag (a new epoch when anything was flagged).

        Views only ever lose rows when flags change, so views created before
        the call do not regain the unflagged rows; build them afresh.
        """
        if not self.is_empty:
            self._docs = _EMPTY_I64
            self._items = {}
            self._epoch += 1

    def excluded(self, store: ObservationStore, column: ShapePath, rows: np.ndarray) -> np.ndarray:
        """Return a bool mask of the ``rows`` of ``column`` that are excluded.

        A row is excluded when it is flagged, its document is flagged, or an
        ancestor is flagged. Ancestors are found by following ``parent`` up
        to the shallowest column that has flagged items.
        """
        rows = np.asarray(rows, dtype=np.int64)
        out = np.zeros(len(rows), dtype=bool)
        if self.is_empty or len(rows) == 0:
            return out
        depths = [depth for depth in range(len(column) + 1) if column[:depth] in self._items]
        if depths:
            top = depths[0]
            current, shape = rows, column
            while True:
                flagged = self._items.get(shape)
                if flagged is not None:
                    out |= isin_sorted(current, flagged)
                if len(shape) == top:
                    break
                current = store.columns[shape].parent[current]
                shape = shape[:-1]
        if len(self._docs):
            out |= isin_sorted(store.columns[column].doc[rows], self._docs)
        return out

    def exclude(self, store: ObservationStore, column: ShapePath, rows: np.ndarray) -> np.ndarray:
        """Return the ``rows`` of ``column`` that are not excluded, in their original order."""
        if self.is_empty:
            return rows
        return rows[~self.excluded(store, column, rows)]

    def __repr__(self) -> str:
        return f"<FlagSet epoch {self._epoch}: {self.n_documents} documents, {self.n_items} items>"


# ---------------------------------------------------------------------------
# ViewStats
# ---------------------------------------------------------------------------


class Histogram(NamedTuple):
    """A sparse histogram: distinct ``values`` ascending with their ``counts`` (both int64)."""

    values: np.ndarray
    counts: np.ndarray

    @property
    def total(self) -> int:
        return int(self.counts.sum())


def _histogram(data: np.ndarray) -> Histogram:
    values, counts = np.unique(data.astype(np.int64, copy=False), return_counts=True)
    return Histogram(_readonly(values), _readonly(counts.astype(np.int64, copy=False)))


@dataclass(frozen=True, eq=False)
class ViewStats:
    """Integer statistics of the non-flagged rows of a view.

    Positions (``object_index`` and the object positions behind the
    presence data) refer to the rows of ``view.live()`` in view order, that
    is, part by part in canonical column order and by row within a part.
    Object ``i`` is the ``i``-th object row in that order.

    Presence is stored sparsely, by key: ``key_ids`` are the distinct keys
    of the objects, ``key_counts[j]`` is the number of objects that have key
    ``key_ids[j]``, and :meth:`objects_with` lists those objects.
    :meth:`bitset` and :attr:`presence` pack the same data into bitsets with
    ``bitorder="little"``: object ``i`` is bit ``i % 8`` of byte ``i // 8``.
    """

    n: int
    type_counts: np.ndarray  # int64[7], indexed by JType
    value_ids: np.ndarray  # int32 distinct scalar value ids, ascending (canonical value order)
    value_counts: np.ndarray  # int64, aligned with value_ids
    string_lengths: Histogram  # code points of the strings
    array_lengths: Histogram  # elements of the arrays
    object_sizes: Histogram  # members of the objects
    object_index: np.ndarray  # int64 positions of the object rows in the live view
    key_ids: np.ndarray  # int32 distinct keys over the objects, ascending (code-point order)
    key_counts: np.ndarray  # int64 number of objects that have each key
    key_indptr: np.ndarray  # int64[len(key_ids) + 1] offsets into key_objects
    key_objects: np.ndarray  # int64 object positions, grouped by key, ascending within a key

    @property
    def n_objects(self) -> int:
        return len(self.object_index)

    def count(self, jtype: JType) -> int:
        """Number of rows of type ``jtype``."""
        return int(self.type_counts[JType(jtype)])

    @property
    def types(self) -> tuple[JType, ...]:
        """The JTypes present, in JType order."""
        return tuple(JType(t) for t in np.flatnonzero(self.type_counts).tolist())

    def value_count(self, vid: int) -> int:
        """Number of rows whose scalar value id is ``vid`` (0 when absent)."""
        j = int(np.searchsorted(self.value_ids, vid))
        return int(self.value_counts[j]) if j < len(self.value_ids) and self.value_ids[j] == vid else 0

    def _key_slot(self, kid: int) -> int | None:
        j = int(np.searchsorted(self.key_ids, kid))
        return j if j < len(self.key_ids) and self.key_ids[j] == kid else None

    def objects_with(self, kid: int) -> np.ndarray:
        """Positions of the objects that have key id ``kid``, ascending."""
        j = self._key_slot(kid)
        if j is None:
            return _EMPTY_I64
        return self.key_objects[self.key_indptr[j] : self.key_indptr[j + 1]]

    @property
    def n_bytes(self) -> int:
        """Bytes per packed presence bitset."""
        return (self.n_objects + 7) // 8

    def bitset(self, kid: int) -> np.ndarray:
        """The packed presence bitset of key id ``kid`` over the objects (uint8)."""
        bits = np.zeros(self.n_objects, dtype=bool)
        bits[self.objects_with(kid)] = True
        return np.packbits(bits, bitorder="little")

    @cached_property
    def presence(self) -> np.ndarray:
        """Packed presence bitsets, one row per entry of ``key_ids`` (uint8, read-only).

        Memory is ``len(key_ids) × n_bytes``, so Map-like views with many
        keys should use :meth:`objects_with` instead.
        """
        n_keys, n_bytes = len(self.key_ids), self.n_bytes
        out = np.zeros((n_keys, n_bytes), dtype=np.uint8)
        if len(self.key_objects):
            key_row = np.repeat(np.arange(n_keys, dtype=np.int64), np.diff(self.key_indptr))
            # (key row, object) pairs are sorted, so flat byte indices are non-decreasing.
            flat = key_row * n_bytes + self.key_objects // 8
            bit = np.left_shift(np.uint8(1), (self.key_objects % 8).astype(np.uint8))
            starts = np.flatnonzero(np.r_[True, flat[1:] != flat[:-1]])
            out.reshape(-1)[flat[starts]] = np.bitwise_or.reduceat(bit, starts)
        return _readonly(out)


def compute_stats(view: View) -> ViewStats:
    """Compute the ViewStats of ``view.live()`` without consulting the cache."""
    live = view.live()
    store = live.store
    jtype = live.gather("jtype")
    sizes = live.gather("size")
    values = live.gather("value")
    values = values[values >= 0]
    value_ids, value_counts = np.unique(values, return_counts=True)
    object_index = np.flatnonzero(jtype == JType.OBJECT).astype(np.int64, copy=False)

    # (object position, key id) for every member of every object in the view.
    member_objects: list[np.ndarray] = []
    member_keys: list[np.ndarray] = []
    offset = 0
    for part in live.parts:
        column = store.columns[part.column]
        objects = part.rows[column.jtype[part.rows] == JType.OBJECT]
        children = store.columns.get(part.column + (MAPVAL_STEP,))
        if children is not None and len(objects):
            lo, hi = children.runs(objects)
            member_rows = ranges_to_rows(lo, hi)
            member_objects.append(np.repeat(np.arange(offset, offset + len(objects), dtype=np.int64), hi - lo))
            member_keys.append(children.key[member_rows])
        offset += len(objects)
    if member_keys:
        keys = np.concatenate(member_keys)
        objects_of = np.concatenate(member_objects)
        # Pairs are ordered by object; a stable sort by key keeps objects ascending per key.
        order = np.argsort(keys, kind="stable")
        keys, objects_of = keys[order], objects_of[order]
    else:
        keys = np.empty(0, dtype=np.int32)
        objects_of = _EMPTY_I64
    key_ids, starts, key_counts = np.unique(keys, return_index=True, return_counts=True)

    return ViewStats(
        n=len(jtype),
        type_counts=_readonly(np.bincount(jtype, minlength=N_JTYPES).astype(np.int64)),
        value_ids=_readonly(value_ids.astype(np.int32, copy=False)),
        value_counts=_readonly(value_counts.astype(np.int64, copy=False)),
        string_lengths=_histogram(sizes[jtype == JType.STRING]),
        array_lengths=_histogram(sizes[jtype == JType.ARRAY]),
        object_sizes=_histogram(sizes[jtype == JType.OBJECT]),
        object_index=_readonly(object_index),
        key_ids=_readonly(key_ids.astype(np.int32, copy=False)),
        key_counts=_readonly(key_counts.astype(np.int64, copy=False)),
        key_indptr=_readonly(np.append(starts, len(keys)).astype(np.int64)),
        key_objects=_readonly(objects_of),
    )


class StatsCache:
    """ViewStats cached by ``(view digest, flag epoch)``.

    Epochs only grow, so entries of an older epoch can never be hit again;
    they are dropped as soon as a lookup arrives with a newer epoch.
    """

    __slots__ = ("_epoch", "_entries")

    def __init__(self) -> None:
        self._epoch = -1
        self._entries: dict[bytes, ViewStats] = {}

    def get(self, digest: bytes, epoch: int, compute: Callable[[], ViewStats]) -> ViewStats:
        if epoch != self._epoch:
            self._entries = {}
            self._epoch = epoch
        stats = self._entries.get(digest)
        if stats is None:
            stats = self._entries[digest] = compute()
        return stats

    def __len__(self) -> int:
        return len(self._entries)

    def __repr__(self) -> str:
        return f"<StatsCache epoch {self._epoch}: {len(self)} entries>"


def view_stats(view: View) -> ViewStats:
    """Return the (cached) ViewStats of the non-flagged rows of ``view``."""
    store = view.store
    return store.stats_cache.get(view.digest, store.flags.epoch, lambda: compute_stats(view))
