"""Format detection through the Validator (Requirements 13.4, 13.9).

Formats
    :data:`FORMATS` lists the ten Formats of 13.4 in the requirement's order,
    which is also the documented candidate order (the ``order_key`` of a
    candidate). A string satisfies a Format exactly when
    :meth:`Validator.satisfies_format <jsinfer.io.validator.Validator.satisfies_format>`
    accepts it, which is the verdict of ``{"format": f}`` with format
    assertion on. No other rule (regex, parser, or cut-off) decides it.

Checks
    :class:`FormatDetector` belongs to one ObservationStore. It checks each
    distinct string once per Format and keeps the ten verdicts of a string as
    a bitmask indexed by the string's value id (or key id, for Map keys).
    Value and key ids are fixed for the store, so the masks are reused by
    every view of the store (Paths, Variants, unions, samples) and the
    Validator is asked at most once per distinct string and Format for the
    whole run. The Validator's own LRU cache is bounded; this one is not, and
    holds two bytes per distinct string.

    :meth:`FormatDetector.detect` covers the string values of a view's
    non-flagged rows, :meth:`FormatDetector.detect_keys` the keys of its
    non-flagged objects (each key counted once per object that has it, for
    ``propertyNames``, 4.4), and :meth:`FormatDetector.detect_strings` any
    multiset of strings (for example the Map-entry keys of a hybrid
    partition, 4.6). Non-string values are ignored: ``format`` constrains
    strings only.

Results
    A :class:`FormatDetection` holds one :class:`FormatCheck` per Format with
    the satisfied and rejected counts, with multiplicity and distinct. The
    candidates of 13.4 are the checks that at least one string satisfies.
    Withholding the candidates that reject a string (13.6) and the MDL
    selection among the rest (13.8) belong to the ValueInferrer, which uses
    :meth:`FormatCheck.rejection` for the report entry and
    :meth:`FormatCheck.candidate` for the DecisionEngine. Counts are
    integers summed per distinct value, so results do not depend on
    document order, chunking, or the worker count.

Fallback_Behavior (13.9)
    :func:`format_candidates` is the entry point of the ValueInferrer. With
    ``values.format`` disabled it returns no candidate and asks the Validator
    nothing, which leaves every other step of the Value_Inferrer unchanged. The detector itself does not read Technique
    states, so other Techniques (latent types, 13.3) may use its verdicts
    whatever the state of Format detection.
"""

from __future__ import annotations

import bisect
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass
from operator import itemgetter
from typing import Any, Final

import numpy as np

from jsinfer.config.registry import REGISTRY
from jsinfer.core.determinism import RANK_STRING
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import ObservationStore
from jsinfer.core.paths import Path
from jsinfer.core.report import RejectedCandidate
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate
from jsinfer.io.validator import Validator

__all__ = [
    "FORMATS",
    "REJECTED_CATEGORY",
    "TECHNIQUE",
    "FormatCheck",
    "FormatDetection",
    "FormatDetector",
    "format_candidates",
    "format_enabled",
]

# The Technique id and the Inference_Report category of withheld Formats (13.6).
TECHNIQUE: Final = "values.format"
REJECTED_CATEGORY: Final = "formats"

# The ten Formats of 13.4, in the documented candidate order.
FORMATS: Final[tuple[str, ...]] = (
    "date-time",
    "date",
    "time",
    "duration",
    "email",
    "hostname",
    "ipv4",
    "ipv6",
    "uri",
    "uuid",
)

_ORDER: Final[Mapping[str, int]] = {fmt: i for i, fmt in enumerate(FORMATS)}
# Bit i of a verdict mask is set when the string satisfies FORMATS[i].
_BITS: Final = np.left_shift(np.int64(1), np.arange(len(FORMATS), dtype=np.int64))
_UNCHECKED: Final = -1  # mask of a string the Validator has not been asked about yet
_MASK_DTYPE: Final = np.int16  # ten bits and the sentinel


@dataclass(frozen=True, slots=True)
class FormatCheck:
    """The Validator's verdicts for one Format over the strings of a view.

    ``satisfied`` and ``rejected`` count strings with multiplicity (one per
    non-flagged Observation, or per object for keys); the ``distinct_``
    fields count distinct strings.
    """

    fmt: str
    satisfied: int
    rejected: int
    distinct_satisfied: int
    distinct_rejected: int

    def __post_init__(self) -> None:
        if self.fmt not in _ORDER:
            raise ValueError(f"unknown Format {self.fmt!r}; expected one of {list(FORMATS)}")
        for name in ("satisfied", "rejected", "distinct_satisfied", "distinct_rejected"):
            value = getattr(self, name)
            if isinstance(value, bool) or not isinstance(value, int) or value < 0:
                raise ValueError(f"FormatCheck {name} must be an int >= 0, got {value!r}")
        if self.distinct_satisfied > self.satisfied or self.distinct_rejected > self.rejected:
            raise ValueError("a FormatCheck counts more distinct strings than strings")

    @property
    def is_candidate(self) -> bool:
        """Whether at least one string satisfies the Format, so it is a candidate (13.4)."""
        return self.satisfied > 0

    @property
    def satisfies_all(self) -> bool:
        """Whether the Format is a candidate that accepts every string (13.6, 13.8)."""
        return self.satisfied > 0 and self.rejected == 0

    @property
    def keywords(self) -> dict[str, JSON]:
        """The Constraint the candidate stands for: ``{"format": fmt}``."""
        return {"format": self.fmt}

    @property
    def label(self) -> str:
        """The candidate's outcome label in Decision records, ``format=<fmt>``."""
        return f"format={self.fmt}"

    @property
    def order_key(self) -> tuple[int]:
        """The Format's position in the documented candidate order (:data:`FORMATS`)."""
        return (_ORDER[self.fmt],)

    def candidate(self, node: SchemaNode | None = None) -> Candidate:
        """The DecisionEngine candidate of this Format at ``string_constraint``.

        ``node`` is the partial schema the MDL_Scorer scores (for example the
        scalar's schema with this ``format``); the ValueInferrer builds it.
        """
        return Candidate(
            technique=TECHNIQUE, outcome=self.keywords, order_key=self.order_key, node=node, label=self.label
        )

    def rejection(self, path: Path | str) -> RejectedCandidate:
        """The Inference_Report entry of this Format withheld at ``path`` (13.6).

        ``details`` holds the count of rejected non-flagged strings (with
        multiplicity) and of distinct rejected strings.
        """
        if self.rejected == 0:
            raise ValueError(f"Format {self.fmt!r} rejects no string, so it is not withheld")
        return RejectedCandidate(
            REJECTED_CATEGORY,
            path,
            self.keywords,
            TECHNIQUE,
            {"rejected": self.rejected, "distinct_rejected": self.distinct_rejected},
        )


@dataclass(frozen=True, slots=True)
class FormatDetection:
    """The checks of every Format over one multiset of strings (see the module docstring).

    ``checks`` holds one :class:`FormatCheck` per entry of :data:`FORMATS`, in
    that order. ``n_strings`` counts the strings with multiplicity and
    ``n_distinct`` the distinct strings.
    """

    n_strings: int
    n_distinct: int
    checks: tuple[FormatCheck, ...]

    def __post_init__(self) -> None:
        if tuple(check.fmt for check in self.checks) != FORMATS:
            raise ValueError(f"a FormatDetection holds one check per Format, in the order {list(FORMATS)}")
        for check in self.checks:
            if check.satisfied + check.rejected != self.n_strings:
                raise ValueError(f"the counts of {check.fmt!r} do not add up to {self.n_strings} strings")
            if check.distinct_satisfied + check.distinct_rejected != self.n_distinct:
                raise ValueError(f"the distinct counts of {check.fmt!r} do not add up to {self.n_distinct} strings")

    @classmethod
    def empty(cls) -> FormatDetection:
        """The detection over no strings: no Format is a candidate."""
        return cls(0, 0, tuple(FormatCheck(fmt, 0, 0, 0, 0) for fmt in FORMATS))

    def check(self, fmt: str) -> FormatCheck:
        """The check of Format ``fmt``."""
        try:
            return self.checks[_ORDER[fmt]]
        except KeyError:
            raise KeyError(f"unknown Format {fmt!r}; expected one of {list(FORMATS)}") from None

    @property
    def candidates(self) -> tuple[FormatCheck, ...]:
        """The Formats at least one string satisfies (13.4), in :data:`FORMATS` order."""
        return tuple(check for check in self.checks if check.is_candidate)

    @property
    def satisfying(self) -> tuple[FormatCheck, ...]:
        """The candidates that accept every string (13.8), in :data:`FORMATS` order."""
        return tuple(check for check in self.checks if check.satisfies_all)

    @property
    def withheld(self) -> tuple[FormatCheck, ...]:
        """The candidates that reject at least one string, to be withheld (13.6), in :data:`FORMATS` order."""
        return tuple(check for check in self.checks if check.is_candidate and check.rejected > 0)


def _summarize(masks: np.ndarray, counts: np.ndarray) -> FormatDetection:
    """Aggregate the verdict masks of distinct strings, weighted by their counts."""
    if len(masks) == 0:
        return FormatDetection.empty()
    bits = (masks.astype(np.int64)[:, None] & _BITS) != 0  # (distinct strings, Formats)
    counts = counts.astype(np.int64, copy=False)
    satisfied = counts @ bits.astype(np.int64)
    distinct = bits.sum(axis=0, dtype=np.int64)
    n_strings, n_distinct = int(counts.sum()), len(masks)
    checks = tuple(
        FormatCheck(
            fmt,
            int(satisfied[i]),
            n_strings - int(satisfied[i]),
            int(distinct[i]),
            n_distinct - int(distinct[i]),
        )
        for i, fmt in enumerate(FORMATS)
    )
    return FormatDetection(n_strings, n_distinct, checks)


class FormatDetector:
    """Format checks for the views of one ObservationStore (see the module docstring).

    ``validator`` gives every verdict; by default the detector builds its
    own. Verdicts are cached per value id and key id of ``store``.
    """

    __slots__ = ("_store", "_validator", "_string_lo", "_value_masks", "_key_masks")

    def __init__(self, store: ObservationStore, validator: Validator | None = None) -> None:
        if not isinstance(store, ObservationStore):
            raise TypeError(f"store must be an ObservationStore, got {type(store).__name__}")
        if validator is not None and not isinstance(validator, Validator):
            raise TypeError(f"validator must be a Validator or None, got {type(validator).__name__}")
        self._store = store
        self._validator = validator if validator is not None else Validator()
        # Value ids follow canonical value order, so the strings hold one contiguous id range.
        keys = store.values.keys()
        rank = itemgetter(0)
        self._string_lo = bisect.bisect_left(keys, RANK_STRING, key=rank)
        string_hi = bisect.bisect_right(keys, RANK_STRING, key=rank)
        self._value_masks = np.full(string_hi - self._string_lo, _UNCHECKED, dtype=_MASK_DTYPE)
        self._key_masks = np.full(len(store.keys), _UNCHECKED, dtype=_MASK_DTYPE)

    @property
    def store(self) -> ObservationStore:
        return self._store

    @property
    def validator(self) -> Validator:
        return self._validator

    def _mask(self, s: str) -> int:
        """The verdict mask of one string: one Validator check per Format."""
        mask = 0
        for i, fmt in enumerate(FORMATS):
            if self._validator.satisfies_format(fmt, s):
                mask |= 1 << i
        return mask

    def _masks(self, cache: np.ndarray, slots: np.ndarray, string_at: Callable[[int], Any]) -> np.ndarray:
        """The masks at ``slots`` of ``cache``, checking each string ``string_at(slot)`` not seen before."""
        masks = cache[slots]
        for slot in slots[masks == _UNCHECKED].tolist():
            s = string_at(slot)
            assert isinstance(s, str)
            cache[slot] = self._mask(s)
        return cache[slots]

    def detect(self, view: View) -> FormatDetection:
        """The checks over the string values of the non-flagged rows of ``view`` (13.4)."""
        stats = self._own(view).stats()
        lo = self._string_lo
        ids = stats.value_ids
        is_string = (ids >= lo) & (ids < lo + len(self._value_masks))
        values = self._store.values
        masks = self._masks(self._value_masks, ids[is_string].astype(np.int64) - lo, lambda slot: values[slot + lo])
        return _summarize(masks, stats.value_counts[is_string])

    def detect_keys(self, view: View) -> FormatDetection:
        """The checks over the keys of the non-flagged objects of ``view``, each key once per object (4.4)."""
        stats = self._own(view).stats()
        masks = self._masks(self._key_masks, stats.key_ids.astype(np.int64), self._store.key_name)
        return _summarize(masks, stats.key_counts)

    def detect_strings(self, strings: Mapping[str, int] | Iterable[str]) -> FormatDetection:
        """The checks over a multiset of strings: a mapping from string to count, or an iterable of strings.

        These strings are not ids of the store, so only the Validator's own
        cache applies to them.
        """
        tally: dict[str, int] = {}
        if isinstance(strings, Mapping):
            for s, count in strings.items():
                s = self._string(s)
                if isinstance(count, bool) or not isinstance(count, (int, np.integer)) or count < 1:
                    raise ValueError(f"the count of {s!r} must be an int >= 1, got {count!r}")
                tally[s] = int(count)
        elif isinstance(strings, (str, bytes)) or not isinstance(strings, Iterable):
            raise TypeError(f"strings must be a mapping or an iterable of str, got {type(strings).__name__}")
        else:
            for s in strings:
                s = self._string(s)
                tally[s] = tally.get(s, 0) + 1
        masks = np.fromiter((self._mask(s) for s in tally), dtype=_MASK_DTYPE, count=len(tally))
        return _summarize(masks, np.fromiter(tally.values(), dtype=np.int64, count=len(tally)))

    @staticmethod
    def _string(s: Any) -> str:
        if not isinstance(s, str):
            raise TypeError(f"Format checks apply to str values, got {type(s).__name__}")
        return s

    def _own(self, view: View) -> View:
        if not isinstance(view, View):
            raise TypeError(f"view must be a View, got {type(view).__name__}")
        if view.store is not self._store:
            raise ValueError("the view belongs to another ObservationStore than this FormatDetector")
        return view

    def __repr__(self) -> str:
        checked = int((self._value_masks != _UNCHECKED).sum()) + int((self._key_masks != _UNCHECKED).sum())
        return f"<FormatDetector: {checked} distinct strings checked>"


def format_enabled(techniques: Mapping[str, bool]) -> bool:
    """Whether ``values.format`` is enabled; an omitted id takes its registered default (28.2)."""
    if not isinstance(techniques, Mapping):
        raise TypeError(f"techniques must map Technique ids to bools, got {type(techniques).__name__}")
    return bool(techniques.get(TECHNIQUE, REGISTRY.technique(TECHNIQUE).default_enabled))


def format_candidates(
    detector: FormatDetector, view: View, techniques: Mapping[str, bool], *, keys: bool = False
) -> tuple[FormatCheck, ...]:
    """The Format candidates at ``view`` under the Technique states ``techniques`` (13.4, 13.9).

    These are the Formats at least one non-flagged string satisfies, in
    :data:`FORMATS` order, each with its counts; the ValueInferrer withholds
    those with ``rejected > 0`` (13.6). With ``values.format`` disabled this
    is the Fallback_Behavior: no candidate, and no Validator check. ``keys``
    selects the keys of the view's objects instead of its string values.
    """
    if not format_enabled(techniques):
        return ()
    detection = detector.detect_keys(view) if keys else detector.detect(view)
    return detection.candidates
