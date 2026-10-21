"""Document_Support: leave-one-document-out validation of closing Constraints (4.12, 14.9, 15.9, 16.9).

A closing Constraint narrows what the schema accepts beyond the observed
types: a Closed_Enum, ``additionalProperties: false``, an induced Pattern,
or a bound. Each is inferred from the Observations of the
non-flagged documents, and the real-data benchmark (task 41) showed them
over-fitting small Document_Collections: with two documents every key set
is closed, every value set is an enum, every string shape is a Pattern, and
every key is required, so most held-out documents fail. The Decision_Models,
the MDL comparisons, and the Good–Turing charges all count Observations at
the Path, which arrays and Maps inflate far beyond the document count, and
none of them sees how many documents the evidence came from.

The rule
    A closing Constraint is **withheld** unless it survives
    leave-one-document-out validation: for every non-flagged document, the
    Constraint inferred from the other documents' Observations must accept
    that document's Observations. The rule is a validation, not a cut-off:
    it removes a candidate that the collection itself refutes, and the
    DecisionEngine resolves the Decision_Point among the rest
    (``Candidate.withheld``). With one document nothing can be held out, so
    every closing Constraint is withheld. The Baseline_Rules ``type`` and
    ``required`` are not Decision_Points and are unaffected: a run with every
    Technique off must still equal Naive_Merge (28.5), which lists both from
    one document as from many.

    The rule makes the Good–Turing charges of 15.6 and the closed Record
    inert in practice, without changing them: a value or key with Document_Support
    of two or more occurs twice or more, so ``f1`` is 0 and the charge is 0
    whenever the closed candidate is not withheld. The charges still apply
    when a caller offers the candidates without the rule.

    Each family has a closed form that needs no re-inference:

    - Closed_Enum (15.9): the enum inferred without a document lacks every
      value only that document holds, so the closed candidate survives iff
      every distinct value occurs in two or more documents
      (:func:`value_documents`).
    - Closed Record (4.12): ``additionalProperties: false`` inferred without
      a document rejects a key only that document's objects hold, so the
      closed candidate survives iff every key listed under ``properties`` is
      held in two or more documents (:func:`key_documents`).
    - Bounds (16.9): a bound inferred without the one document that attains
      the observed extreme is built from the extreme over the other
      documents; a rung survives iff its value, computed from that extreme,
      still covers the observed one (:class:`QuantitySupport`; the rungs are
      evaluated in ``values.bounds``). When two or more documents attain the
      extreme, leaving one out changes nothing and every rung survives.
    - Patterns (14.9): induction has no closed form, so the documents are
      split into at most :data:`PATTERN_FOLDS` folds, dealt in the order of
      the strings each document holds (:func:`document_folds`), and a rung
      survives iff, for every fold, the
      Pattern induced at that rung from the other folds' strings matches
      every string only that fold holds (``values.inferrer``).

Determinism
    Every statistic is a count over the ``doc`` field of the Observations
    (``core.observations``), so results depend only on the documents as
    JSON values and their order, not on chunking or the worker count (27.6).
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Final

import numpy as np

from jsinfer.core.jsonvalue import JSON
from jsinfer.core.views import View

__all__ = [
    "PATTERN_FOLDS",
    "RULE",
    "QuantitySupport",
    "SideSupport",
    "document_folds",
    "documents_of",
    "key_documents",
    "single_document_keys",
    "value_documents",
    "withheld",
]

# The rule named in every withheld candidate's details and report entry.
RULE: Final = "document_support"

# The most document folds pattern induction holds out (leave-one-document-out below it).
PATTERN_FOLDS: Final = 3

Number = int | float


def withheld(documents: int, **facts: JSON) -> dict[str, JSON]:
    """The ``withheld`` details of a candidate the rule removed: the rule, the document count, and ``facts``."""
    details: dict[str, JSON] = {"rule": RULE, "documents": documents}
    details.update(facts)
    return details


def _docs(view: View) -> np.ndarray:
    return view.live().gather("doc").astype(np.int64, copy=False)


def documents_of(view: View) -> int:
    """The number of distinct non-flagged documents with a row in ``view``."""
    if not isinstance(view, View):
        raise TypeError(f"expected a View, got {type(view).__name__}")
    docs = _docs(view)
    return int(len(np.unique(docs))) if len(docs) else 0


def _documents_per_id(ids: np.ndarray, docs: np.ndarray, n_docs: int, wanted: np.ndarray) -> np.ndarray:
    """For each id of ``wanted`` (sorted ascending), the number of distinct ``docs`` paired with it."""
    out = np.zeros(len(wanted), dtype=np.int64)
    if not len(ids) or not len(wanted):
        return out
    span = max(int(n_docs), int(docs.max()) + 1)
    pairs = np.unique(ids.astype(np.int64) * span + docs.astype(np.int64))
    seen, counts = np.unique(pairs // span, return_counts=True)
    slots = np.searchsorted(wanted, seen)
    hit = (slots < len(wanted)) & (wanted[np.minimum(slots, len(wanted) - 1)] == seen)
    out[slots[hit]] = counts[hit]
    return out


def value_documents(scalars: View) -> np.ndarray:
    """Per distinct scalar value of ``scalars``, the number of documents it occurs in.

    The result is int64 and aligned with ``scalars.stats().value_ids``;
    rows without a scalar value (containers) are ignored.
    """
    if not isinstance(scalars, View):
        raise TypeError(f"expected a View, got {type(scalars).__name__}")
    live = scalars.live()
    wanted = live.stats().value_ids.astype(np.int64)
    vids = live.gather("value")
    mask = vids >= 0
    return _documents_per_id(vids[mask], live.gather("doc")[mask], live.store.n_docs, wanted)


def key_documents(objects: View) -> np.ndarray:
    """Per distinct key of the objects of ``objects``, the number of documents whose objects hold it.

    The result is int64 and aligned with ``objects.stats().key_ids``. A key
    whose every member value is a Flagged_Item still counts as present in
    its parent (17.4) but has no live member row, so it gets 0 here, which
    withholds the closed candidate: the collection cannot vouch for it.
    """
    if not isinstance(objects, View):
        raise TypeError(f"expected a View, got {type(objects).__name__}")
    live = objects.live()
    wanted = live.stats().key_ids.astype(np.int64)
    members = live.child_map()
    return _documents_per_id(members.gather("key"), members.gather("doc"), live.store.n_docs, wanted)


def single_document_keys(objects: View, names: Sequence[str] | None = None) -> list[str]:
    """The keys of the objects of ``objects`` (restricted to ``names`` when given) held in fewer than two documents.

    In code-point order. A closed Record survives leave-one-document-out
    iff this list is empty and the objects come from two or more documents.
    """
    live = objects.live()
    store = live.store
    counts = key_documents(live)
    listed = None if names is None else set(names)
    found: list[str] = []
    for kid, count in zip(live.stats().key_ids.tolist(), counts.tolist()):
        name = store.key_name(int(kid))
        if count < 2 and (listed is None or name in listed):
            found.append(name)
    return found


# ---------------------------------------------------------------------------
# Bounds (16.9)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class SideSupport:
    """One side of :class:`QuantitySupport`.

    ``attained_by`` documents attain the observed extreme. ``others`` is the
    extreme over the other documents when exactly one attains it (None when
    that document is the only one, or when two or more attain it).
    """

    attained_by: int
    others: Number | None

    @property
    def unchanged(self) -> bool:
        """Whether leaving any one document out leaves the observed extreme in place."""
        return self.attained_by >= 2


@dataclass(frozen=True, slots=True)
class QuantitySupport:
    """The leave-one-document-out facts of one set of quantities: the document count and both sides."""

    documents: int
    minimum: SideSupport
    maximum: SideSupport

    @classmethod
    def of(cls, quantities: Sequence[Number], ranks: np.ndarray, docs: np.ndarray) -> QuantitySupport:
        """From the distinct ``quantities`` in ascending order, each row's ``ranks`` into them, and each row's ``docs``.

        Exact comparisons happen once, when the caller sorts the distinct
        quantities; rows are compared by rank, so big integers and floats
        are handled exactly and the per-row work is vectorized.
        """
        ranks = np.asarray(ranks, dtype=np.int64)
        docs = np.asarray(docs, dtype=np.int64)
        if ranks.shape != docs.shape or ranks.ndim != 1:
            raise ValueError(f"ranks and docs must be 1-D arrays of one length, got {ranks.shape} and {docs.shape}")
        if not len(ranks):
            raise ValueError("document support needs at least one quantity")
        if ranks.min() < 0 or ranks.max() >= len(quantities):
            raise ValueError("every rank must index the distinct quantities")
        documents = int(len(np.unique(docs)))
        return cls(documents, _side(quantities, ranks, docs, True), _side(quantities, ranks, docs, False))

    @classmethod
    def of_array(cls, values: np.ndarray, docs: np.ndarray) -> QuantitySupport:
        """From one integer quantity per row (lengths or counts) and the rows' documents."""
        values = np.asarray(values)
        if not np.issubdtype(values.dtype, np.integer):
            raise TypeError(f"of_array takes integer quantities, got dtype {values.dtype}")
        distinct, ranks = np.unique(values, return_inverse=True)
        return cls.of(distinct.tolist(), ranks.reshape(-1), docs)


def _side(quantities: Sequence[Number], ranks: np.ndarray, docs: np.ndarray, minimum: bool) -> SideSupport:
    extreme = int(ranks.min() if minimum else ranks.max())
    attaining = np.unique(docs[ranks == extreme])
    if len(attaining) >= 2:
        return SideSupport(len(attaining), None)
    rest = ranks[docs != attaining[0]]
    if not len(rest):
        return SideSupport(1, None)
    return SideSupport(1, quantities[int(rest.min() if minimum else rest.max())])


# ---------------------------------------------------------------------------
# Patterns (14.9)
# ---------------------------------------------------------------------------


def document_folds(n: int, folds: int = PATTERN_FOLDS) -> np.ndarray:
    """The fold of each of ``n`` documents in a given order: ``min(n, folds)`` folds, dealt round-robin.

    With fewer documents than ``folds`` every document is its own fold,
    which is leave-one-document-out. The caller orders the documents by
    content (what each holds at the Path), not by index, so the folds do not
    depend on document order (27.6): documents with equal content are
    interchangeable, and their assignment cannot change which strings a fold
    alone holds.
    """
    if isinstance(n, bool) or not isinstance(n, (int, np.integer)) or n < 0:
        raise ValueError(f"n must be a non-negative integer, got {n!r}")
    if isinstance(folds, bool) or not isinstance(folds, (int, np.integer)) or folds < 1:
        raise ValueError(f"folds must be a positive integer, got {folds!r}")
    k = min(int(folds), int(n))
    return np.arange(int(n), dtype=np.int64) % k if k else np.empty(0, dtype=np.int64)
