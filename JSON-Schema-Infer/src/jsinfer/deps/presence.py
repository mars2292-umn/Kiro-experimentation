"""Presence bitsets, Base_Fields, and required fields (Requirements 6.1, 6.5, 9.1, 9.2, 9.7, 26.13).

Presence (9.1, 9.2)
    :class:`Presence` records which fields the non-flagged objects of one
    Variant hold. A field is present in an object when the object has the
    field's key with any value, ``null`` included (9.2). Only the view's
    non-flagged object rows count
    (:func:`~jsinfer.structure.record_map.object_rows`): scalars and arrays
    at the same Path are not objects, and flagged objects, objects in
    flagged documents, and objects under a flagged ancestor are excluded.
    As in ``core.stats``, a member whose value is a Flagged_Item still makes
    its key present in its non-flagged parent object, so flags affect only
    the flagged item's subtree (17.4).

    The fields are the keys held by at least one of the objects, in
    code-point order (key ids follow it). Each field is classified as
    exactly one of required (held by every object) or optional (held by at
    least one but not every object), using only the Variant's own objects
    (9.1).

Packed bitsets
    ``Presence.bits`` holds one packed bitset per field, row ``j`` for
    ``names[j]``: a read-only ``uint8`` array of shape
    ``(n_fields, n_bytes)`` with ``n_bytes = ceil(n_objects / 8)`` and
    ``bitorder="little"``, so object ``i`` is bit ``i % 8`` of byte
    ``i // 8``. Object ``i`` is the ``i``-th row of ``Presence.objects``,
    the object rows the bitsets cover, so ``objects.subset(mask(name))`` is
    the view of the objects that hold ``name``. The padding bits of the last
    byte are 0 in every row. The array is the ViewStats presence matrix of
    the object rows (``core.stats``), shared, not copied.

    The queries the Dependency_Miner builds on:

    - :meth:`Presence.count` is the number of objects that hold a field, the
      Support count of a Co_Occurrence_Rule antecedent (9.6);
    - :meth:`Presence.joint_count` and :meth:`Presence.joint_counts` count
      the objects that hold two fields, by popcount of the AND of their
      bitsets;
    - :meth:`Presence.implies` and :meth:`Presence.implication_matrix` test
      ``rows(a) ⊆ rows(b)`` as ``bits[a] & ~bits[b] == 0`` in every byte,
      the antecedent ⊆ consequent test of the Co_Occurrence_Rule candidates
      (9.3, task 14.2). ``implication_matrix()[i, j]`` is True when every
      object holding ``names[i]`` also holds ``names[j]``, so its diagonal is
      True and a required ``names[j]`` makes column ``j`` all True. Every
      field is held by at least one object, so every row has an antecedent.

    A Co_Occurrence_Rule candidate ``a → b`` of 9.3 is a pair ``a ≠ b``
    with ``implication_matrix()[a, b]`` and ``b`` optional; its Support
    count is ``count(a)``. Field_Group violations (6.7, task 33.1) are the
    objects that hold some but not all members: the popcount of the OR of
    the members' bitsets minus that of their AND (:func:`popcount`).
    Exclusion candidates (11.1, task 35.1) are the pairs whose joint count
    is 0.

Base_Fields (6.1)
    The Base_Fields of a Variant are the fields every non-flagged object of
    the Variant holds: :attr:`Presence.base_fields`, the same set as
    :attr:`Presence.required` (Property 18). The Base_Fields of a family of
    Variants, the Variants emitted as the branches of one ``anyOf`` or
    ``oneOf`` at a Path, are the intersection of the Variants' Base_Fields
    (:func:`family_base_fields`), in code-point order. When the branches'
    Variants partition the Path's objects (as the final Variants of
    Merge-or-Split do), that intersection equals the Base_Fields of the
    union of their objects.

``required`` (6.5, 9.7, 26.13)
    :class:`RequiredFields` is the ``required`` keyword of one Variant's
    object schema: every field marked required for the Variant and no
    optional one (9.7). Every Base_Field of the Variant is therefore listed
    (6.5), and since a family's Base_Fields are Base_Fields of each of its
    Variants, every one of them is under ``required`` in every branch of the
    family. With no required field the keyword is omitted. The rule is the
    ``required`` Baseline_Rule, which every run applies and no Technique
    serves, so there is no Decision_Point, no Fallback_Behavior, and no
    registry entry to switch.

    Each entry is its own Constraint, ``(nid, "required", name)`` at
    ``…/required/<i>`` (the design's granularity table; ``i`` is the name's
    index in the emitted, code-point-sorted list). Each entry's
    Provenance_Record names the Baseline_Rule marker :data:`REQUIRED_PRODUCER`
    (``baseline:required``), with no fallback indicator and no resolution
    (26.13). Its Support is every non-flagged object of the Variant, each of
    which holds the field, and its Confidence is the Laplace default
    ``(n + 1) / (n + 2)``, since the entry has no alternative. Provenance
    finalization (task 17.1) recomputes Support after the soundness check,
    and soundness relaxations of an entry (removing it, 3.3) go in the
    record's ``changes`` (3.6).

Hooks
    The analyzer walk (task 16.1, design walk step 2.5) calls
    :func:`required_fields` on each final branch's object view (the views of
    :meth:`~jsinfer.structure.variants.MergeSplitOutcome.views`, or the
    Path's view for a single Variant), adds :meth:`RequiredFields.keywords`
    to the Variant's Record node next to the keywords of
    :meth:`~jsinfer.structure.record_map.RecordMapAnalyzer.record_node`, and
    collects :meth:`RequiredFields.constraint_provenance` of that node. Its
    ``variant_schema`` for :func:`~jsinfer.structure.variants.variant_branches`
    does this per branch, so the branch schemas carry their own ``required``
    lists. Co-occurrence mining (task 14.2) receives the same
    :class:`Presence` (``RequiredFields.presence``) for its candidates.
    Discriminator value groups (10.4, 10.5, task 34.1) list the family's
    Base_Fields under ``required`` in each branch and in the fallback branch,
    and Field_Group mining (6.3, 6.4, task 33.1) restricts its candidates to
    fields that are not Base_Fields.

Determinism
    Fields follow key ids, which are in code-point order, and bitsets,
    counts, and classifications depend only on the multiset of non-flagged
    objects' key sets, so the results do not depend on document order,
    chunking, or worker count. A Presence holds the object rows that were
    non-flagged when it was built; after flags change, build it afresh.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

import numpy as np

from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import ProvenanceRecord, Support, baseline_technique
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.structure.record_map import object_rows

__all__ = [
    "OPTIONAL",
    "REQUIRED",
    "REQUIRED_PRODUCER",
    "FieldClass",
    "Presence",
    "RequiredFields",
    "family_base_fields",
    "popcount",
    "presence",
    "required_fields",
]

# The keyword of the required-field Constraints, and the Baseline_Rule that produces them (9.2, 26.13).
REQUIRED: Final = "required"
OPTIONAL: Final = "optional"

# The producer named in the Provenance_Record of each ``required`` entry (26.13).
REQUIRED_PRODUCER: Final = baseline_technique(REQUIRED)

FieldClass: TypeAlias = Literal["required", "optional"]


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _readonly(arr: np.ndarray) -> np.ndarray:
    if arr.flags.writeable:
        arr.setflags(write=False)
    return arr


def popcount(bits: np.ndarray) -> int:
    """The number of set bits in a packed ``uint8`` bitset (any shape)."""
    arr = np.asarray(bits)
    if arr.dtype != np.uint8:
        raise TypeError(f"a packed bitset is uint8, got {arr.dtype}")
    return int(np.bitwise_count(arr).sum(dtype=np.int64))


# ---------------------------------------------------------------------------
# Presence
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class Presence:
    """Packed presence bitsets of the fields of one Variant's non-flagged objects (see the module docstring).

    Build it with :meth:`of` (or :func:`presence`). ``objects`` are the
    object rows the bitsets cover; ``names`` are the fields in code-point
    order, with their ``key_ids`` (int32), ``counts`` (int64, the objects
    holding each field), and packed ``bits`` (uint8, one row per field).
    """

    objects: View
    names: tuple[str, ...]
    key_ids: np.ndarray
    counts: np.ndarray
    bits: np.ndarray
    _index: Mapping[str, int] = field(init=False, repr=False)
    _required: tuple[str, ...] = field(init=False, repr=False)
    _optional: tuple[str, ...] = field(init=False, repr=False)

    def __post_init__(self) -> None:
        if not isinstance(self.objects, View):
            raise TypeError(f"objects must be a View, got {type(self.objects).__name__}")
        names = tuple(self.names)
        if any(not isinstance(name, str) for name in names):
            raise TypeError("field names must be str")
        n = len(self.objects)
        n_bytes = (n + 7) // 8
        key_ids = _readonly(np.asarray(self.key_ids, dtype=np.int32))
        counts = _readonly(np.asarray(self.counts, dtype=np.int64))
        bits = _readonly(np.asarray(self.bits))
        if key_ids.shape != (len(names),) or counts.shape != (len(names),):
            raise ValueError("key_ids and counts need one entry per field")
        if bits.dtype != np.uint8 or bits.shape != (len(names), n_bytes):
            raise ValueError(f"bits must be uint8 of shape ({len(names)}, {n_bytes}), got {bits.dtype} {bits.shape}")
        if len(names) and (counts.min() < 1 or counts.max() > n):
            raise ValueError(f"every field is held by 1 … {n} objects")
        index = {name: j for j, name in enumerate(names)}
        if len(index) != len(names):
            raise ValueError("field names must be distinct")
        _set(self, "names", names)
        _set(self, "key_ids", key_ids)
        _set(self, "counts", counts)
        _set(self, "bits", bits)
        _set(self, "_index", MappingProxyType(index))
        full = counts == n
        _set(self, "_required", tuple(name for name, always in zip(names, full.tolist()) if always))
        _set(self, "_optional", tuple(name for name, always in zip(names, full.tolist()) if not always))

    @classmethod
    def of(cls, view: View) -> Presence:
        """The presence of the fields of the non-flagged object rows of ``view``.

        ``view`` is a Variant's object view or a Path's view (non-object rows
        are ignored). Raises ``ValueError`` when it has no non-flagged object.
        """
        objects = object_rows(view)
        if objects.is_empty:
            raise ValueError(f"no non-flagged object at {render_path(view.path)}")
        stats = objects.stats()
        store = objects.store
        # Key ids follow code-point order, so the names are already sorted.
        names = tuple(store.key_name(int(kid)) for kid in stats.key_ids.tolist())
        return cls(objects, names, stats.key_ids, stats.key_counts, stats.presence)

    # -- inspection ---------------------------------------------------------

    @property
    def path(self) -> Path:
        return self.objects.path

    @property
    def n_objects(self) -> int:
        return len(self.objects)

    @property
    def n_fields(self) -> int:
        return len(self.names)

    @property
    def n_bytes(self) -> int:
        """Bytes per packed bitset."""
        return (self.n_objects + 7) // 8

    @property
    def epoch(self) -> int:
        """The flag epoch at which the object rows were non-flagged."""
        return self.objects.epoch

    def __contains__(self, name: object) -> bool:
        return isinstance(name, str) and name in self._index

    def index(self, name: str) -> int:
        """The row of field ``name`` in ``bits``; ``KeyError`` when no object holds it."""
        try:
            return self._index[name]
        except KeyError:
            raise KeyError(f"no non-flagged object at {render_path(self.path)} holds {name!r}") from None
        except TypeError:
            raise TypeError(f"field names are str, got {type(name).__name__}") from None

    def count(self, name: str) -> int:
        """The number of objects that hold ``name`` (0 when none does)."""
        return int(self.counts[self._index[name]]) if name in self else 0

    def bitset(self, name: str) -> np.ndarray:
        """The packed bitset of ``name`` (read-only uint8 of ``n_bytes``)."""
        return self.bits[self.index(name)]

    def mask(self, name: str) -> np.ndarray:
        """A bool array over ``objects``: which objects hold ``name``."""
        row = np.unpackbits(self.bitset(name), count=self.n_objects, bitorder="little")
        return row.astype(bool)

    # -- classification (9.1, 9.2) ------------------------------------------

    @property
    def required(self) -> tuple[str, ...]:
        """The fields every object holds, in code-point order (9.2)."""
        return self._required

    @property
    def optional(self) -> tuple[str, ...]:
        """The fields held by at least one but not every object, in code-point order (9.1)."""
        return self._optional

    @property
    def base_fields(self) -> tuple[str, ...]:
        """The Variant's Base_Fields: the fields every non-flagged object holds (6.1), the same as :attr:`required`."""
        return self._required

    def is_required(self, name: str) -> bool:
        """Whether every object holds ``name``."""
        return name in self and int(self.counts[self._index[name]]) == self.n_objects

    def classify(self, name: str) -> FieldClass:
        """``required`` or ``optional`` for a field some object holds (9.1); ``KeyError`` otherwise."""
        return REQUIRED if int(self.counts[self.index(name)]) == self.n_objects else OPTIONAL

    def classification(self) -> dict[str, FieldClass]:
        """Every field with its class, in code-point order of the names (9.1)."""
        return {name: self.classify(name) for name in self.names}

    # -- bitset queries -------------------------------------------------------

    def joint_count(self, a: str, b: str) -> int:
        """The number of objects that hold both ``a`` and ``b``."""
        return popcount(self.bitset(a) & self.bitset(b))

    def joint_counts(self) -> np.ndarray:
        """``(n_fields, n_fields)`` int64: entry ``[i, j]`` counts the objects holding ``names[i]`` and ``names[j]``.

        The diagonal is :attr:`counts`. Each row is one vectorized popcount
        over every field's bitset, so the cost is ``n_fields² · n_bytes``.
        """
        out = np.empty((self.n_fields, self.n_fields), dtype=np.int64)
        for i in range(self.n_fields):
            out[i] = np.bitwise_count(self.bits[i] & self.bits).sum(axis=1, dtype=np.int64)
        return out

    def implies(self, a: str, b: str) -> bool:
        """Whether every object that holds ``a`` also holds ``b``: ``rows(a) ⊆ rows(b)``."""
        return not np.any(self.bitset(a) & ~self.bitset(b))

    def implication_matrix(self) -> np.ndarray:
        """``(n_fields, n_fields)`` bool: ``[i, j]`` when every object holding ``names[i]`` holds ``names[j]``.

        Tested on the packed bitsets as ``bits[i] & ~bits[j] == 0`` (the
        padding bits are 0 in ``bits[i]``, so they never count).
        """
        out = np.empty((self.n_fields, self.n_fields), dtype=bool)
        inverse = ~self.bits
        for i in range(self.n_fields):
            out[i] = ~np.any(self.bits[i] & inverse, axis=1)
        return out

    def __repr__(self) -> str:
        return (
            f"<Presence {render_path(self.path)}: {self.n_objects} objects, {self.n_fields} fields, "
            f"{len(self._required)} required>"
        )


def presence(view: View) -> Presence:
    """The presence bitsets of the non-flagged objects of ``view`` (the Dependency_Miner's ``presence``, 9.1, 9.2)."""
    return Presence.of(view)


def _as_presence(source: View | Presence) -> Presence:
    if isinstance(source, Presence):
        return source
    if isinstance(source, View):
        return Presence.of(source)
    raise TypeError(f"expected a View or a Presence, got {type(source).__name__}")


# ---------------------------------------------------------------------------
# Base_Fields of a family (6.1)
# ---------------------------------------------------------------------------


def family_base_fields(variants: Iterable[View | Presence]) -> tuple[str, ...]:
    """The Base_Fields of a family of Variants: the intersection of the Variants' Base_Fields (6.1).

    ``variants`` are the Variants emitted as the branches of one ``anyOf`` or
    ``oneOf`` (their object views or presences), at least one, all over one
    ObservationStore. The result is in code-point order.
    """
    members = [_as_presence(variant) for variant in variants]
    if not members:
        raise ValueError("a family has at least one Variant")
    store = members[0].objects.store
    if any(member.objects.store is not store for member in members):
        raise ValueError("the Variants of a family come from one ObservationStore")
    first, rest = members[0], members[1:]
    return tuple(name for name in first.base_fields if all(member.is_required(name) for member in rest))


# ---------------------------------------------------------------------------
# The required keyword (6.5, 9.7, 26.13)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class RequiredFields:
    """The ``required`` keyword of one Variant's object schema and its entries' records (see the module docstring)."""

    presence: Presence
    record: ProvenanceRecord = field(init=False, repr=False)

    def __post_init__(self) -> None:
        if not isinstance(self.presence, Presence):
            raise TypeError(f"presence must be a Presence, got {type(self.presence).__name__}")
        _set(self, "record", ProvenanceRecord.baseline(REQUIRED, self.support))

    @property
    def path(self) -> Path:
        return self.presence.path

    @property
    def names(self) -> tuple[str, ...]:
        """The required fields in code-point order: the Variant's Base_Fields."""
        return self.presence.required

    @property
    def is_empty(self) -> bool:
        """True when no field is required, so no ``required`` keyword is emitted."""
        return not self.names

    @property
    def support(self) -> Support:
        """Every non-flagged object of the Variant holds every required field (26.2)."""
        n = self.presence.n_objects
        return Support(n, n)

    def keywords(self) -> dict[str, list[str]]:
        """``{"required": [...]}`` with a fresh list of the names, or ``{}`` when none is required (9.7)."""
        return {REQUIRED: list(self.names)} if self.names else {}

    @property
    def provenance(self) -> Mapping[str, ProvenanceRecord]:
        """The Provenance_Record of each entry, by field name (the ``baseline:required`` record)."""
        return MappingProxyType({name: self.record for name in self.names})

    def constraint_provenance(self, node: SchemaNode) -> dict[ConstraintId, ProvenanceRecord]:
        """The entries' records keyed by ``(node.nid, "required", name)``.

        ``node`` is the Variant's object schema, whose ``required`` keyword
        must list exactly :attr:`names` (in any order). Raises
        ``ValueError`` otherwise.
        """
        if not isinstance(node, SchemaNode):
            raise TypeError(f"node must be a SchemaNode, got {type(node).__name__}")
        listed = node.kw.get(REQUIRED, []) if node.boolean is None else []
        if not isinstance(listed, list) or frozenset(listed) != frozenset(self.names) or len(listed) != len(self.names):
            raise ValueError(
                f"the node's 'required' keyword must list exactly {list(self.names)} for {render_path(self.path)}"
            )
        return {ConstraintId(node.nid, REQUIRED, name): self.record for name in self.names}


def required_fields(source: View | Presence) -> RequiredFields:
    """The ``required`` keyword of a Variant from its object view or its :class:`Presence` (6.5, 9.7)."""
    return RequiredFields(_as_presence(source))
