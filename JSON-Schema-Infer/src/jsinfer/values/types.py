"""Scalar ``type`` Constraints with the integer rule (Requirements 13.1, 13.2).

The Value_Inferrer emits, for each scalar Path, a ``type`` Constraint that
lists exactly the JSON types of the non-flagged scalar values at the Path,
drawn from ``null``, ``boolean``, ``integer``, ``number``, and ``string``
(13.1). Numbers are listed once:

- as ``integer`` when every non-flagged number has a zero fractional part,
  however it is written (``1``, ``1.0``, ``-0.0``, ``1e3``; 13.2);
- as ``number`` otherwise, in which case ``integer`` is not listed (an
  integer validates against ``number``), so the two never appear together.

``core.jsonvalue.JType`` already classifies every zero-fraction number as
INTEGER, and :class:`~jsinfer.core.stats.ViewStats` counts the non-flagged
rows of each JType, so this module only applies the integer rule to those
counts. Arrays and objects at the same Path are not scalar values and are
not listed here; the Structure_Analyzer describes them and combines the
type lists (task 16.1).

A Path whose strings encode a Latent_Type keeps ``string`` as their JSON
type (13.3); the Latent_Type goes into the ``x-latent-type`` annotation
(``values.latent``), never into ``type``.

The result depends only on integer counts of the view's non-flagged rows,
so it is the same for every document order, chunking, and worker count.
``type`` has no alternative, so its default Confidence is the Laplace rule
over :attr:`ScalarTypes.support` (``core.provenance.laplace_confidence``).
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from typing import Final

from jsinfer.core.jsonvalue import JType
from jsinfer.core.provenance import Support
from jsinfer.core.stats import ViewStats
from jsinfer.core.views import View

__all__ = [
    "SCALAR_JTYPES",
    "ScalarTypes",
    "scalar_type_names",
    "scalar_types",
    "type_names",
]

# The scalar JTypes in emitted ``type`` order.
SCALAR_JTYPES: Final = (JType.NULL, JType.BOOLEAN, JType.INTEGER, JType.NUMBER, JType.STRING)


def type_names(jtypes: Iterable[JType]) -> list[str]:
    """The ``type`` names of the given JTypes in emitted order, with the integer rule.

    ``integer`` is dropped when ``number`` is present (13.1, 13.2). Duplicates
    are ignored, and the input order does not matter.
    """
    present = [False] * len(JType)
    for jtype in jtypes:
        present[JType(jtype)] = True
    if present[JType.NUMBER]:
        present[JType.INTEGER] = False
    return [jtype.keyword for jtype in JType if present[jtype]]


@dataclass(frozen=True, slots=True)
class ScalarTypes:
    """The ``type`` Constraint of a scalar view.

    ``names`` are the listed type names in emitted order, and ``counts[i]``
    is the number of non-flagged rows of type ``names[i]``; under ``number``
    that includes the integral numbers. ``n`` is the number of non-flagged
    scalar rows, the sum of ``counts``.
    """

    names: tuple[str, ...]
    counts: tuple[int, ...]

    @property
    def n(self) -> int:
        return sum(self.counts)

    @property
    def is_empty(self) -> bool:
        """True when the view has no non-flagged scalar rows (no ``type`` to emit)."""
        return not self.names

    @property
    def keyword_value(self) -> list[str]:
        """The ``type`` keyword value: a fresh list of the names."""
        return list(self.names)

    def count(self, name: str) -> int:
        """Rows listed under the type name ``name`` (0 when it is not listed)."""
        for listed, count in zip(self.names, self.counts):
            if listed == name:
                return count
        return 0

    @property
    def support(self) -> Support:
        """Every non-flagged scalar row has a listed type, so all ``n`` are consistent (26.2)."""
        return Support(self.n, self.n)


def scalar_types(source: View | ViewStats) -> ScalarTypes:
    """The ``type`` Constraint of the non-flagged scalar rows of a view (13.1, 13.2).

    Takes the view or its (cached) ViewStats. Arrays and objects in the view
    are ignored; a view without scalar rows gives an empty result.
    """
    if isinstance(source, View):
        stats = source.stats()
    elif isinstance(source, ViewStats):
        stats = source
    else:
        raise TypeError(f"scalar_types takes a View or ViewStats, got {type(source).__name__}")
    counts = {jtype: stats.count(jtype) for jtype in SCALAR_JTYPES}
    if counts[JType.NUMBER]:
        counts[JType.NUMBER] += counts.pop(JType.INTEGER)
    listed = [(jtype.keyword, count) for jtype, count in counts.items() if count]
    return ScalarTypes(tuple(name for name, _ in listed), tuple(count for _, count in listed))


def scalar_type_names(source: View | ViewStats) -> list[str]:
    """The ``type`` names of the non-flagged scalar rows of a view, in emitted order."""
    return scalar_types(source).keyword_value
