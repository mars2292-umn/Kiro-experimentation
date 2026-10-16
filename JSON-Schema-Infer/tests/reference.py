"""Plain-Python reference models that tests compare the columnar store against."""

from __future__ import annotations

from typing import Any

import numpy as np

from jsinfer.core.jsonvalue import JType, canonical_key, jtype_of
from jsinfer.core.observations import ObservationStore
from jsinfer.core.paths import ELEM, ELEM_STEP, KEY, MAPVAL_STEP, Path, Step, key_step, path_sort_key
from jsinfer.core.views import View


def walk(docs: list[Any], path: Path) -> list[tuple[int, Any, int | None]]:
    """``(doc, value, array position)`` of every Observation at ``path``, in store order.

    Store order is document order, then pre-order with object members in
    code-point key order; stepping level by level gives the same order.
    """
    current: list[tuple[int, Any, int | None]] = [(d, doc, None) for d, doc in enumerate(docs)]
    for step in path:
        nxt: list[tuple[int, Any, int | None]] = []
        for d, value, _ in current:
            if step.kind == KEY:
                if isinstance(value, dict) and step.key in value:
                    nxt.append((d, value[step.key], None))
            elif step.kind == ELEM:
                if isinstance(value, (list, tuple)):
                    nxt.extend((d, item, i) for i, item in enumerate(value))
            elif isinstance(value, dict):
                nxt.extend((d, value[name], None) for name in sorted(value))
        current = nxt
    return current


def signature(d: int, value: Any) -> tuple[int, int, Any]:
    """``(doc, jtype, canonical key or size)`` of one Observation."""
    jtype = jtype_of(value)
    return (d, int(jtype), canonical_key(value) if jtype.is_scalar else len(value))


def view_signatures(view: View) -> list[tuple[int, int, Any]]:
    """The :func:`signature` of every row of ``view``, in view order."""
    table = view.store.values
    out = []
    for d, jtype, vid, size in zip(
        view.gather("doc").tolist(),
        view.gather("jtype").tolist(),
        view.gather("value").tolist(),
        view.gather("size").tolist(),
    ):
        out.append((d, jtype, table.key(vid) if JType(jtype).is_scalar else size))
    return out


def present_paths(docs: list[Any], max_depth: int = 4) -> list[Path]:
    """Every logical Path with an Observation (Key and ``{*}`` forms of object steps), in Path order."""
    found: set[Path] = {()}  # the root, even for an empty collection

    def visit(value: Any, path: Path) -> None:
        found.add(path)
        if len(path) >= max_depth:
            return
        if isinstance(value, dict):
            for name, item in value.items():
                visit(item, path + (key_step(name),))
                visit(item, path + (MAPVAL_STEP,))
        elif isinstance(value, (list, tuple)):
            for item in value:
                visit(item, path + (ELEM_STEP,))

    for doc in docs:
        visit(doc, ())
    return sorted(found, key=path_sort_key)


def brute_excluded(store: ObservationStore, column: tuple[Step, ...], row: int) -> bool:
    """Whether a row is flagged, in a flagged document, or under a flagged ancestor (one row at a time)."""
    flags = store.flags
    if int(store.columns[column].doc[row]) in set(flags.documents.tolist()):
        return True
    shape, r = column, row
    while True:
        if r in set(flags.item_rows(shape).tolist()):
            return True
        if not shape:
            return False
        r = int(store.columns[shape].parent[r])
        shape = shape[:-1]


def assert_stores_equal(a: ObservationStore, b: ObservationStore) -> None:
    """Assert two stores have identical tables and column arrays."""
    assert a.n_docs == b.n_docs
    assert a.keys == b.keys
    assert a.values == b.values
    assert list(a.columns) == list(b.columns)
    for shape, col in a.columns.items():
        other = b.columns[shape]
        for name in ("parent", "doc", "jtype", "key", "pos", "value", "size"):
            x, y = getattr(col, name), getattr(other, name)
            assert x.dtype == y.dtype, (shape, name)
            np.testing.assert_array_equal(x, y, err_msg=f"{shape} {name}")
