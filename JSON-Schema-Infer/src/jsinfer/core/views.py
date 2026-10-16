"""Row-index views over the ObservationStore.

A :class:`View` is a set of Observations: for each shape-path column, a
sorted array of rows (a :class:`ViewPart`). The view of a logical Path such
as ``$.users[*].email`` is built from the root by :meth:`View.child_key`,
:meth:`View.child_map` (``{*}``), and :meth:`View.elements` (``[*]``). Record
properties, Map values, Variants, Tagged_Union branches, Tuple positions,
and samples are all views over the same columns, so no Observation is ever
copied.

Child views
    Children of the rows ``R`` of a column are found with ``np.searchsorted``
    on the child column's non-decreasing ``parent`` field: either the runs of
    each parent row, or a membership test of the candidates' parents in
    ``R`` (for a key that occurs less often than ``R`` has rows). Total work
    is O(observations · log). Tuple position ``i`` is the ``i``-th row of
    each run, because elements are stored in index order. Tuple positions
    are not Path steps: ``elements(i)`` has the Path of ``elements()``.

Union views
    Shared_Definitions and recursion are inferred from the union of several
    Paths. :meth:`View.union` merges the parts of several views, column by
    column, into one multi-part view whose ``path`` is a representative
    (by default the Path with the fewest steps). Parts are kept in canonical
    column order, so the union does not depend on the order of its inputs.

Flags
    Views never contain excluded rows (flagged, in a flagged document, or
    under a flagged ancestor; ``core.stats.FlagSet``). A view's rows are
    fixed when it is created, together with the store's flag epoch at that
    time. If flags change later, :meth:`View.live` drops the newly excluded
    rows, and every derived view (children, subsets, unions) and the
    ViewStats are computed from the live rows.

Digest
    :attr:`View.digest` is a 16-byte BLAKE2b digest of the view's columns
    and rows, so it identifies the row set by content. It does not include
    the logical ``path``: views with equal rows have equal statistics. It
    never uses ``id()`` or ``hash()``.
"""

from __future__ import annotations

import hashlib
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Final

import numpy as np

from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import (
    FIELD_DTYPES,
    FIELDS,
    ObservationStore,
    ShapePath,
    isin_sorted,
    ranges_to_rows,
)
from jsinfer.core.paths import (
    ELEM,
    ELEM_STEP,
    KEY,
    MAPVAL,
    MAPVAL_STEP,
    ROOT,
    Path,
    Step,
    fewest_steps_key,
    key_step,
    path_sort_key,
    render_path,
)
from jsinfer.core.stats import ViewStats, as_row_array, view_stats

__all__ = ["View", "ViewPart"]

_DIGEST_SIZE: Final = 16


@dataclass(frozen=True, eq=False, repr=False)
class ViewPart:
    """The rows of one column in a view: sorted, distinct, read-only int64."""

    column: ShapePath
    rows: np.ndarray

    def __len__(self) -> int:
        return len(self.rows)

    def __repr__(self) -> str:
        return f"<ViewPart {render_path(self.column)}: {len(self.rows)} rows>"


@dataclass(frozen=True, eq=False, repr=False)
class View:
    """A set of Observations, with the logical Path they stand for.

    Create views with :meth:`root` and derive others with the methods below;
    the constructor does not filter flags or sort rows. ``parts`` are in
    canonical column order, each non-empty. ``epoch`` is the flag epoch at
    which the rows were last filtered.
    """

    store: ObservationStore
    path: Path
    parts: tuple[ViewPart, ...]
    epoch: int

    # -- construction -------------------------------------------------------

    @classmethod
    def root(cls, store: ObservationStore, docs: Iterable[int] | np.ndarray | None = None) -> View:
        """The root view ``$`` of every non-flagged document, or of the documents ``docs``.

        ``docs`` selects a Document_Collection subset, such as a Version
        segment. Indices outside ``0 … n_docs - 1`` raise ``ValueError``.
        """
        if docs is None:
            rows = np.arange(store.n_docs, dtype=np.int64)
        else:
            rows = as_row_array(docs, "document indices")
            if len(rows) and rows[-1] >= store.n_docs:
                raise ValueError(f"document index {int(rows[-1])} out of range for {store.n_docs} documents")
        rows = store.flags.exclude(store, ROOT, rows)
        return cls._make(store, ROOT, {ROOT: rows})

    @classmethod
    def _make(
        cls,
        store: ObservationStore,
        path: Path,
        rows_by_column: Mapping[ShapePath, np.ndarray],
        epoch: int | None = None,
    ) -> View:
        """Build a view from sorted, distinct rows that are not excluded at ``epoch`` (default: now)."""
        parts = []
        for column in sorted(rows_by_column, key=path_sort_key):
            rows = rows_by_column[column]
            if len(rows):
                rows = np.asarray(rows, dtype=np.int64)
                rows.setflags(write=False)
                parts.append(ViewPart(column, rows))
        return cls(store, tuple(path), tuple(parts), store.flags.epoch if epoch is None else epoch)

    def _children(self, path: Path, rows_by_column: Mapping[ShapePath, np.ndarray]) -> View:
        """Children of live rows: only their own item flags can exclude them."""
        flags = self.store.flags
        kept = {}
        for column, rows in rows_by_column.items():
            flagged = flags.item_rows(column)
            kept[column] = rows[~isin_sorted(rows, flagged)] if len(flagged) else rows
        return View._make(self.store, path, kept)

    @staticmethod
    def union(views: Iterable[View], path: Path | None = None) -> View:
        """Merge views of one store into a (multi-part) union view.

        Rows present in several inputs appear once. ``path`` defaults to the
        input Path with the fewest steps (ties by Path order).
        """
        views = list(views)
        if not views:
            raise ValueError("a union needs at least one view")
        store = views[0].store
        if any(view.store is not store for view in views):
            raise ValueError("cannot unite views of different ObservationStores")
        if path is None:
            path = min((view.path for view in views), key=fewest_steps_key)
        pieces: dict[ShapePath, list[np.ndarray]] = {}
        for view in views:
            for part in view.live().parts:
                pieces.setdefault(part.column, []).append(part.rows)
        merged = {
            column: rows[0] if len(rows) == 1 else np.unique(np.concatenate(rows)) for column, rows in pieces.items()
        }
        return View._make(store, path, merged)

    # -- inspection ---------------------------------------------------------

    def __len__(self) -> int:
        return sum(len(part.rows) for part in self.parts)

    @property
    def is_empty(self) -> bool:
        return not self.parts

    @property
    def columns(self) -> tuple[ShapePath, ...]:
        return tuple(part.column for part in self.parts)

    def gather(self, name: str) -> np.ndarray:
        """Return field ``name`` (see ``core.observations.FIELDS``) of every row, in view order."""
        if name not in FIELD_DTYPES:
            raise ValueError(f"unknown column field {name!r}; expected one of {list(FIELDS)}")
        if not self.parts:
            return np.empty(0, dtype=FIELD_DTYPES[name])
        columns = self.store.columns
        arrays = [getattr(columns[part.column], name)[part.rows] for part in self.parts]
        return arrays[0] if len(arrays) == 1 else np.concatenate(arrays)

    @cached_property
    def digest(self) -> bytes:
        """16-byte BLAKE2b digest of the columns and rows (not of ``path``)."""
        h = hashlib.blake2b(digest_size=_DIGEST_SIZE, person=b"jsinfer.view")
        h.update(canonical_bytes([[list(part.column), len(part.rows)] for part in self.parts]))
        for part in self.parts:
            h.update(np.ascontiguousarray(part.rows, dtype="<i8").tobytes())
        return h.digest()

    def stats(self) -> ViewStats:
        """The cached ViewStats of the live rows (``core.stats.view_stats``)."""
        return view_stats(self)

    def __repr__(self) -> str:
        return f"<View {render_path(self.path)}: {len(self)} rows in {len(self.parts)} columns>"

    # -- flags --------------------------------------------------------------

    def live(self) -> View:
        """This view without the rows excluded by flags added since it was created."""
        flags = self.store.flags
        if self.epoch == flags.epoch:
            return self
        kept = {part.column: flags.exclude(self.store, part.column, part.rows) for part in self.parts}
        return View._make(self.store, self.path, kept)

    # -- derived views ------------------------------------------------------

    def child_key(self, name: str) -> View:
        """The members with key ``name`` of the objects in this view (a Record property)."""
        if not isinstance(name, str):
            raise TypeError(f"object keys are str, got {type(name).__name__}")
        live = self.live()
        path = live.path + (key_step(name),)
        kid = live.store.key_id(name)
        found: dict[ShapePath, np.ndarray] = {}
        if kid is not None:
            for part in live.parts:
                column = part.column + (MAPVAL_STEP,)
                child = live.store.columns.get(column)
                if child is None:
                    continue
                candidates = child.by_key(kid)
                if len(part.rows) < len(candidates):
                    # Fewer parents than key occurrences: walk the parents' runs.
                    rows = ranges_to_rows(*child.runs(part.rows))
                    rows = rows[child.key[rows] == kid]
                else:
                    rows = candidates[isin_sorted(child.parent[candidates], part.rows)]
                found[column] = rows
        return live._children(path, found)

    def child_map(self) -> View:
        """Every member of the objects in this view (the Map wildcard ``{*}``)."""
        live = self.live()
        found: dict[ShapePath, np.ndarray] = {}
        for part in live.parts:
            column = part.column + (MAPVAL_STEP,)
            child = live.store.columns.get(column)
            if child is not None:
                found[column] = ranges_to_rows(*child.runs(part.rows))
        return live._children(live.path + (MAPVAL_STEP,), found)

    def elements(self, pos: int | None = None) -> View:
        """The elements of the arrays in this view, or only those at index ``pos``.

        The Path is ``path + [*]`` either way, since Tuple positions are not
        Path steps.
        """
        if pos is not None and (isinstance(pos, bool) or not isinstance(pos, (int, np.integer)) or pos < 0):
            raise ValueError(f"array position must be a non-negative integer, got {pos!r}")
        live = self.live()
        found: dict[ShapePath, np.ndarray] = {}
        for part in live.parts:
            column = part.column + (ELEM_STEP,)
            child = live.store.columns.get(column)
            if child is None:
                continue
            lo, hi = child.runs(part.rows)
            if pos is None:
                found[column] = ranges_to_rows(lo, hi)
            else:
                # Elements are stored in index order, so index pos is lo + pos.
                at = lo + int(pos)
                found[column] = at[at < hi]
        return live._children(live.path + (ELEM_STEP,), found)

    def subset(self, mask: Any) -> View:
        """The rows of this view where the bool ``mask`` (one entry per row, in view order) is true.

        Build masks from :meth:`gather` of this same view. The result keeps
        the Path and, like every derived view, drops rows excluded by flags
        added since this view was created.
        """
        mask = np.asarray(mask)
        if mask.dtype != np.bool_ or mask.shape != (len(self),):
            raise ValueError(f"subset needs a bool mask of shape ({len(self)},), got {mask.dtype} {mask.shape}")
        picked: dict[ShapePath, np.ndarray] = {}
        start = 0
        for part in self.parts:
            stop = start + len(part.rows)
            picked[part.column] = part.rows[mask[start:stop]]
            start = stop
        # The picked rows were live at this view's epoch; live() catches up with newer flags.
        return View._make(self.store, self.path, picked, self.epoch).live()

    def descend(self, path: Sequence[Step]) -> View:
        """Follow the steps of a relative Path: Key → :meth:`child_key`, ``{*}`` → :meth:`child_map`, ``[*]`` → :meth:`elements`."""
        view = self
        for step in path:
            if not isinstance(step, Step):
                raise TypeError(f"expected a Step, got {type(step).__name__}")
            if step.kind == KEY:
                view = view.child_key(step.key)
            elif step.kind == ELEM:
                view = view.elements()
            elif step.kind == MAPVAL:
                view = view.child_map()
        return view
