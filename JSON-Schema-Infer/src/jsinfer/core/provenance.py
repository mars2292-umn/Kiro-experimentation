"""Provenance_Records: how each Constraint was derived (Requirement 26).

This module holds the record types and their JSON form (task 2.5).
Provenance finalization (task 17.1) and ``explain`` rendering (task 17.3)
are added here later.

Record types
    :class:`ProvenanceRecord` is attached to one Constraint
    (``SchemaModel.provenance`` maps ConstraintIds to records). It holds:

    - ``technique``: the producing Technique id (Requirement 28.1), or
      ``"fallback:<Decision_Point kind>"`` when a Fallback_Behavior produced
      the Constraint (28.12; see :func:`fallback_technique`);
    - ``support``: a :class:`Support` (26.2);
    - ``confidence``: a number in [0, 1] (26.2);
    - ``fallback``: a :class:`FallbackInfo` naming the Technique whose
      Fallback_Behavior was applied and the reason (26.10). Its presence is
      the fallback indicator;
    - ``resolution``: a :class:`Resolution` with the method, the selected
      candidate, and every candidate with its :class:`CandidateScore`
      (19.5, 26.3);
    - ``cutoff``: a :class:`Cutoff` with the cut-off source and either its
      value or the scores of the compared alternatives, plus the
      Decision_Model artifact for a Decision_Model comparison (20.7);
    - ``sample``: a :class:`SampleInfo` with the sampled and total
      Observation counts (26.4);
    - ``changes``: one :class:`RelaxationRecord` per soundness change (3.6);
    - ``details``: other JSON facts, for example Good–Turing statistics, an
      expected count, or a Judgment_Call outcome.

    A record whose technique is a fallback marker, or whose resolution method
    is ``"fallback"``, must carry ``fallback`` (26.10). Other records may
    carry it too: when ``decide.mdl`` is disabled, the priority-order
    Fallback_Behavior of 19.7 keeps the producing Technique of the selected
    candidate.

Support and Confidence
    ``Support.ratio`` is ``consistent / total``, and 1.0 when ``total`` is 0
    (no Observation contradicts the Constraint); ``consistent <= total``
    always holds. Where a Constraint has no alternative (types, observed
    ``required``), the default Confidence is the Laplace rule of succession
    ``(consistent + 1) / (total + 2)`` (:func:`laplace_confidence`), which
    lies strictly inside (0, 1) and is 0.5 with no Observations. Otherwise
    the caller supplies the MDL posterior, the calibrated Decision_Model
    probability, the corpus conditional frequency, or a kind-specific value.

Scores
    The score of a candidate depends on the resolution method (26.3):

    ==================  ====================================================
    ``mdl``             description length in bits (finite, >= 0), required
    ``decision-model``  calibrated probability in [0, 1], required
    ``priority``        absent (``None``)
    ``fallback``        absent (``None``)
    ==================  ====================================================

    ``candidates`` lists every candidate, the selected one included, and
    ``selected`` is its index. A fallback resolution lists the
    Fallback_Behavior's outcome as a candidate with no Technique, because a
    Fallback_Behavior invokes none (28.3).

Numbers
    Counts are ints >= 0. Scores, bits, ratios, Confidences, and cut-off
    values are stored as finite floats: ints and numpy scalars are converted
    and ``-0.0`` becomes ``0.0``. Booleans are rejected wherever a number is
    expected. NaN and infinities are rejected: the MDL_Scorer reports finite
    bits (19.1), probabilities lie in [0, 1], and JSON cannot represent
    them. JSON payloads (``details`` values and the Constraint fragments of
    a RelaxationRecord) are stored as canonical representatives
    (``core.jsonvalue.canonical_value``): ``1.0`` is stored as ``1``, tuples
    as lists, and object keys in code-point order.

JSON form
    ``to_json()`` returns plain JSON with object keys in code-point order at
    every level, so the bytes written by ``core.determinism.encode_canonical``
    do not depend on whether the encoder sorts keys. Absent optional members
    (``None``, an empty tuple, empty ``details``) are omitted rather than
    written as ``null``. ``from_json`` accepts exactly this form, apart from
    the spelling of numbers and the order of object keys, and raises
    :class:`ProvenanceFormatError` (a ``ValueError``) with the JSON Pointer
    of the offending member otherwise. For every record ``r``,
    ``from_json(r.to_json()) == r``, and writing the loaded record again
    gives identical bytes (26.8).

Equality
    The small record types compare field by field. ``ProvenanceRecord`` and
    ``RelaxationRecord`` hold JSON payloads, where Python's ``==`` would
    equate ``true`` with ``1``, so they compare by the canonical bytes of
    their JSON form instead and are unhashable.
"""

from __future__ import annotations

import math
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Final, Literal, NoReturn, TypeAlias, TypeVar

import numpy as np

from jsinfer.core.determinism import canonical_bytes, codepoint_key
from jsinfer.core.jsonvalue import JSON, canonical_value
from jsinfer.core.schema_model import join_pointer

__all__ = [
    "DECISION_MODEL_COMPARISON",
    "FALLBACK_PREFIX",
    "FALLBACK_REASONS",
    "MDL_COMPARISON",
    "REMOVED",
    "RESOLUTION_METHODS",
    "CandidateScore",
    "Cutoff",
    "FallbackInfo",
    "FallbackReason",
    "ProvenanceFormatError",
    "ProvenanceRecord",
    "RelaxationRecord",
    "Removed",
    "Resolution",
    "ResolutionMethod",
    "SampleInfo",
    "Support",
    "fallback_technique",
    "laplace_confidence",
]

T = TypeVar("T")

FallbackReason: TypeAlias = Literal["disabled", "missing-threshold", "missing-model"]
ResolutionMethod: TypeAlias = Literal["mdl", "decision-model", "priority", "fallback"]

# Why a Fallback_Behavior was applied: every serving Technique is disabled
# (28.4); a Learned_Threshold is missing or invalid (20.6); or a
# Decision_Model, the Semantic_Type_Classifier, or the Corpus_Index is
# missing from the Model_Bundle (21.10, 22.14).
FALLBACK_REASONS: Final[tuple[FallbackReason, ...]] = ("disabled", "missing-threshold", "missing-model")

# How a Decision_Point was resolved (26.3): MDL_Scorer, Decision_Model,
# Technique priority order (28.7), or Fallback_Behavior.
RESOLUTION_METHODS: Final[tuple[ResolutionMethod, ...]] = ("mdl", "decision-model", "priority", "fallback")

# Prefix of the technique marker of fallback-produced Constraints (28.12).
FALLBACK_PREFIX: Final = "fallback:"

# Cut-off sources of an MDL comparison and of a Decision_Model comparison
# (20.1, 20.7). The other sources are the name of a model-selection criterion
# (such as "BIC") or of a Learned_Threshold.
MDL_COMPARISON: Final = "MDL comparison"
DECISION_MODEL_COMPARISON: Final = "Decision_Model comparison"


# ---------------------------------------------------------------------------
# Construction checks
# ---------------------------------------------------------------------------


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _real(value: Any, what: str) -> float:
    if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, float, np.integer, np.floating)):
        raise TypeError(f"{what} must be a number, got {type(value).__name__}")
    try:
        x = float(value)
    except OverflowError:
        raise ValueError(f"{what} is too large for a float: {value!r}") from None
    if not math.isfinite(x):
        raise ValueError(f"{what} must be finite, got {x!r}")
    return x + 0.0  # -0.0 becomes 0.0


def _bits(value: Any, what: str) -> float:
    x = _real(value, what)
    if x < 0.0:
        raise ValueError(f"{what} must be >= 0 bits, got {x!r}")
    return x


def _unit(value: Any, what: str) -> float:
    x = _real(value, what)
    if not 0.0 <= x <= 1.0:
        raise ValueError(f"{what} must be in [0, 1], got {x!r}")
    return x


def _count(value: Any, what: str, minimum: int = 0) -> int:
    if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, np.integer)):
        raise TypeError(f"{what} must be an int, got {type(value).__name__}")
    n = int(value)
    if n < minimum:
        raise ValueError(f"{what} must be >= {minimum}, got {n}")
    return n


def _name(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a str, got {type(value).__name__}")
    if not value:
        raise ValueError(f"{what} must be non-empty")
    return value


def _technique_id(value: Any, what: str) -> str:
    name = _name(value, what)
    if name.startswith(FALLBACK_PREFIX):
        raise ValueError(f"{what} must be a Technique id, not the fallback marker {name!r}")
    return name


def _optional(check: Callable[[Any, str], T], value: Any, what: str) -> T | None:
    return None if value is None else check(value, what)


def _instance(value: Any, cls: type[T], what: str) -> T:
    if not isinstance(value, cls):
        raise TypeError(f"{what} must be a {cls.__name__}, got {type(value).__name__}")
    return value


def _optional_instance(value: Any, cls: type[T], what: str) -> T | None:
    if value is not None and not isinstance(value, cls):
        raise TypeError(f"{what} must be a {cls.__name__} or None, got {type(value).__name__}")
    return value


def _instances(value: Any, cls: type[T], what: str) -> tuple[T, ...]:
    if not isinstance(value, (list, tuple)):
        raise TypeError(f"{what} must be a tuple of {cls.__name__}, got {type(value).__name__}")
    for item in value:
        _instance(item, cls, f"{what} entries")
    return tuple(value)


def _payload(value: Any, what: str) -> JSON:
    try:
        return canonical_value(value)
    except (TypeError, ValueError) as error:
        raise type(error)(f"{what} must be a JSON value: {error}") from None


def _ordered(members: dict[str, JSON]) -> dict[str, JSON]:
    """The members with keys in code-point order (the JSON form's key order)."""
    return {key: members[key] for key in sorted(members, key=codepoint_key)}


# ---------------------------------------------------------------------------
# JSON reading
# ---------------------------------------------------------------------------


class ProvenanceFormatError(ValueError):
    """Malformed provenance JSON.

    ``location`` is the JSON Pointer of the offending member, relative to the
    document whose pointer was passed as ``location`` to ``from_json`` (the
    value itself by default). ``reason`` says what is wrong.
    """

    def __init__(self, location: str, reason: str) -> None:
        super().__init__(location, reason)  # args survive pickling
        self.location = location
        self.reason = reason

    def __str__(self) -> str:
        where = repr(self.location) if self.location else "the top level"
        return f"invalid provenance JSON at {where}: {self.reason}"


def _fail(location: str, reason: str) -> NoReturn:
    raise ProvenanceFormatError(location, reason)


def _kind(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "a boolean"
    if isinstance(value, (int, float)):
        return "a number"
    if isinstance(value, str):
        return "a string"
    if isinstance(value, (list, tuple)):
        return "an array"
    if isinstance(value, dict):
        return "an object"
    return f"a {type(value).__name__}"


def _members(value: Any, location: str, required: tuple[str, ...], optional: tuple[str, ...] = ()) -> dict[str, Any]:
    """Check that ``value`` is an object with every required member and no unknown member."""
    if not isinstance(value, dict):
        _fail(location, f"expected an object, got {_kind(value)}")
    allowed = required + optional
    for key in value:
        if key not in allowed:
            _fail(location, f"unknown member {key!r}; the members are {sorted(allowed)}")
    for key in required:
        if key not in value:
            _fail(location, f"missing member {key!r}")
    return value


def _read_int(obj: dict[str, Any], key: str, location: str) -> int:
    value = obj[key]
    if isinstance(value, bool) or not isinstance(value, int):
        _fail(join_pointer(location, key), f"expected an integer, got {_kind(value)}")
    return value


def _read_number(obj: dict[str, Any], key: str, location: str) -> int | float:
    value = obj[key]
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        _fail(join_pointer(location, key), f"expected a number, got {_kind(value)}")
    if isinstance(value, float) and not math.isfinite(value):
        _fail(join_pointer(location, key), f"expected a finite number, got {value!r}")
    return value


def _read_str(obj: dict[str, Any], key: str, location: str) -> str:
    value = obj[key]
    if not isinstance(value, str):
        _fail(join_pointer(location, key), f"expected a string, got {_kind(value)}")
    return value


def _read_array(obj: dict[str, Any], key: str, location: str, *, non_empty: bool = False) -> list[Any]:
    value = obj[key]
    if not isinstance(value, list):
        _fail(join_pointer(location, key), f"expected an array, got {_kind(value)}")
    if non_empty and not value:
        _fail(join_pointer(location, key), "expected a non-empty array (omit the member when there are no entries)")
    return value


def _read_object(obj: dict[str, Any], key: str, location: str, *, non_empty: bool = False) -> dict[str, Any]:
    value = obj[key]
    if not isinstance(value, dict):
        _fail(join_pointer(location, key), f"expected an object, got {_kind(value)}")
    if non_empty and not value:
        _fail(join_pointer(location, key), "expected a non-empty object (omit the member when there are no entries)")
    return value


def _read_optional(
    obj: dict[str, Any], key: str, location: str, read: Callable[[dict[str, Any], str, str], T]
) -> T | None:
    """Read an optional member; an explicit ``null`` is rejected by ``read`` (absent members are omitted)."""
    return read(obj, key, location) if key in obj else None


def _build(cls: Callable[..., T], location: str, **kwargs: Any) -> T:
    """Construct a record, reporting construction errors as ProvenanceFormatErrors at ``location``."""
    try:
        return cls(**kwargs)
    except (TypeError, ValueError) as error:
        raise ProvenanceFormatError(location, str(error)) from None


# ---------------------------------------------------------------------------
# Support and the default Confidence
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Support:
    """``consistent`` of ``total`` Observations at the Constraint's Path agree with it (26.2).

    ``ratio`` is computed: ``consistent / total``, or 1.0 when ``total`` is 0.
    Raises ``ValueError`` when ``consistent > total`` or a count is negative.
    """

    consistent: int
    total: int
    ratio: float = field(init=False)

    def __post_init__(self) -> None:
        consistent = _count(self.consistent, "Support consistent")
        total = _count(self.total, "Support total")
        if consistent > total:
            raise ValueError(f"Support consistent ({consistent}) must not exceed total ({total})")
        _set(self, "consistent", consistent)
        _set(self, "total", total)
        _set(self, "ratio", consistent / total if total else 1.0)

    def to_json(self) -> dict[str, JSON]:
        return {"consistent": self.consistent, "ratio": self.ratio, "total": self.total}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Support:
        obj = _members(value, location, ("consistent", "ratio", "total"))
        consistent = _read_int(obj, "consistent", location)
        total = _read_int(obj, "total", location)
        ratio = _read_number(obj, "ratio", location)
        support = _build(cls, location, consistent=consistent, total=total)
        if ratio != support.ratio:
            _fail(
                join_pointer(location, "ratio"),
                f"ratio {ratio!r} does not equal consistent / total = {support.ratio!r}",
            )
        return support


def laplace_confidence(support: Support) -> float:
    """The default Confidence ``(consistent + 1) / (total + 2)`` (Laplace rule of succession).

    Used for Constraints with no alternative, such as types and observed
    ``required`` entries. The result is in (0, 1); it is 0.5 when ``total`` is 0.
    """
    _instance(support, Support, "support")
    return (support.consistent + 1) / (support.total + 2)


def fallback_technique(kind: str) -> str:
    """The technique marker ``"fallback:<kind>"`` of a Constraint produced by a Fallback_Behavior (28.12)."""
    return FALLBACK_PREFIX + _name(kind, "Decision_Point kind")


# ---------------------------------------------------------------------------
# Candidates and resolutions
# ---------------------------------------------------------------------------

_CANDIDATE_OPTIONAL: Final = (
    "data_bits",
    "exception_bits",
    "model_bits",
    "n_rejected",
    "n_scored",
    "score",
    "technique",
)
_CANDIDATE_BITS: Final = ("data_bits", "exception_bits", "model_bits")
_CANDIDATE_COUNTS: Final = ("n_rejected", "n_scored")


@dataclass(frozen=True, slots=True)
class CandidateScore:
    """One candidate at a Decision_Point with its score (19.5, 26.3).

    ``technique`` is the producing Technique id, or ``None`` for an outcome no
    Technique produced (a Fallback_Behavior's). ``outcome`` describes the
    candidate (for example ``"record"``, ``"map"``, or ``"maximum=100"``).
    ``score`` is a finite number or ``None``; its meaning depends on the
    resolution method (see the module docstring). The MDL breakdown of 19.5
    (``model_bits``, ``data_bits`` including ``exception_bits``,
    ``n_rejected`` of ``n_scored`` Observations) is optional; bits are
    finite and >= 0, counts are >= 0, and ``n_rejected <= n_scored``.
    """

    technique: str | None
    outcome: str
    score: float | None
    model_bits: float | None = None
    data_bits: float | None = None
    exception_bits: float | None = None
    n_rejected: int | None = None
    n_scored: int | None = None

    def __post_init__(self) -> None:
        _set(self, "technique", _optional(_technique_id, self.technique, "CandidateScore technique"))
        if not isinstance(self.outcome, str):
            raise TypeError(f"CandidateScore outcome must be a str, got {type(self.outcome).__name__}")
        _set(self, "score", _optional(_real, self.score, "CandidateScore score"))
        for name in _CANDIDATE_BITS:
            _set(self, name, _optional(_bits, getattr(self, name), f"CandidateScore {name}"))
        for name in _CANDIDATE_COUNTS:
            _set(self, name, _optional(_count, getattr(self, name), f"CandidateScore {name}"))
        if self.n_rejected is not None and self.n_scored is not None and self.n_rejected > self.n_scored:
            raise ValueError(
                f"CandidateScore n_rejected ({self.n_rejected}) must not exceed n_scored ({self.n_scored})"
            )

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {"outcome": self.outcome}
        for name in _CANDIDATE_OPTIONAL:
            value = getattr(self, name)
            if value is not None:
                members[name] = value
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> CandidateScore:
        obj = _members(value, location, ("outcome",), _CANDIDATE_OPTIONAL)
        kwargs: dict[str, Any] = {
            "technique": _read_optional(obj, "technique", location, _read_str),
            "outcome": _read_str(obj, "outcome", location),
            "score": _read_optional(obj, "score", location, _read_number),
        }
        for name in _CANDIDATE_BITS:
            kwargs[name] = _read_optional(obj, name, location, _read_number)
        for name in _CANDIDATE_COUNTS:
            kwargs[name] = _read_optional(obj, name, location, _read_int)
        return _build(cls, location, **kwargs)


@dataclass(frozen=True, slots=True)
class Resolution:
    """How a Decision_Point was resolved: ``method``, every candidate, and the ``selected`` index (26.3).

    ``candidates`` is non-empty and includes the selected candidate; the
    others are the rejected ones. Scores must match the method (see the
    module docstring).
    """

    method: ResolutionMethod
    selected: int
    candidates: tuple[CandidateScore, ...]

    def __post_init__(self) -> None:
        method = self.method
        if not isinstance(method, str) or method not in RESOLUTION_METHODS:
            raise ValueError(f"Resolution method must be one of {list(RESOLUTION_METHODS)}, got {method!r}")
        candidates = _instances(self.candidates, CandidateScore, "Resolution candidates")
        if not candidates:
            raise ValueError("Resolution candidates must be non-empty (they include the selected candidate)")
        selected = _count(self.selected, "Resolution selected")
        if selected >= len(candidates):
            raise ValueError(f"Resolution selected index {selected} is out of range for {len(candidates)} candidates")
        for index, candidate in enumerate(candidates):
            score = candidate.score
            if method == "mdl":
                if score is None or score < 0.0:
                    raise ValueError(
                        f"an MDL resolution scores candidates by description length in bits (>= 0); "
                        f"candidate {index} has score {score!r}"
                    )
            elif method == "decision-model":
                if score is None or not 0.0 <= score <= 1.0:
                    raise ValueError(
                        f"a Decision_Model resolution scores candidates by probability in [0, 1]; "
                        f"candidate {index} has score {score!r}"
                    )
            elif score is not None:
                raise ValueError(f"a {method!r} resolution has no scores; candidate {index} has score {score!r}")
        _set(self, "selected", selected)
        _set(self, "candidates", candidates)

    @property
    def selected_candidate(self) -> CandidateScore:
        """The selected candidate."""
        return self.candidates[self.selected]

    def to_json(self) -> dict[str, JSON]:
        return {
            "candidates": [candidate.to_json() for candidate in self.candidates],
            "method": self.method,
            "selected": self.selected,
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Resolution:
        obj = _members(value, location, ("candidates", "method", "selected"))
        method = _read_str(obj, "method", location)
        selected = _read_int(obj, "selected", location)
        items = _read_array(obj, "candidates", location)
        candidates = tuple(
            CandidateScore.from_json(item, location=join_pointer(location, "candidates", index))
            for index, item in enumerate(items)
        )
        return _build(cls, location, method=method, selected=selected, candidates=candidates)


# ---------------------------------------------------------------------------
# Fallbacks, cut-offs, and samples
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class FallbackInfo:
    """The Technique whose Fallback_Behavior was applied, and why (26.10).

    ``reason`` is ``"disabled"`` (28.4), ``"missing-threshold"`` (a missing
    or invalid Learned_Threshold, 20.6), or ``"missing-model"`` (a missing
    Decision_Model, Semantic_Type_Classifier, or Corpus_Index, 21.10).
    """

    technique: str
    reason: FallbackReason

    def __post_init__(self) -> None:
        _technique_id(self.technique, "FallbackInfo technique")
        if not isinstance(self.reason, str) or self.reason not in FALLBACK_REASONS:
            raise ValueError(f"FallbackInfo reason must be one of {list(FALLBACK_REASONS)}, got {self.reason!r}")

    def to_json(self) -> dict[str, JSON]:
        return {"reason": self.reason, "technique": self.technique}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> FallbackInfo:
        obj = _members(value, location, ("reason", "technique"))
        return _build(
            cls,
            location,
            technique=_read_str(obj, "technique", location),
            reason=_read_str(obj, "reason", location),
        )


@dataclass(frozen=True, slots=True)
class Cutoff:
    """The cut-off of a Decision_Point (20.1, 20.7).

    ``source`` is :data:`MDL_COMPARISON`, :data:`DECISION_MODEL_COMPARISON`,
    the name of a model-selection criterion (such as ``"BIC"``), or the name
    of a Learned_Threshold. Exactly one of ``value`` (the cut-off value, a
    finite number) and ``scores`` (the compared alternatives, for an MDL,
    Decision_Model, or model-selection comparison) is given.

    A Decision_Model comparison records the calibrated probabilities of the
    compared outcomes as ``scores`` (each in [0, 1]) and the identifier of
    the Decision_Model artifact that produced them as ``artifact`` (for
    example ``"dm.record_or_map.synthetic"``). ``artifact`` is given for that
    source and only for it.
    """

    source: str
    value: float | None = None
    scores: tuple[CandidateScore, ...] = ()
    artifact: str | None = None

    def __post_init__(self) -> None:
        source = _name(self.source, "Cutoff source")
        _set(self, "value", _optional(_real, self.value, "Cutoff value"))
        scores = _instances(self.scores, CandidateScore, "Cutoff scores")
        if (self.value is None) == (not scores):
            raise ValueError("a Cutoff records exactly one of a value and the scores of the compared alternatives")
        _set(self, "scores", scores)
        artifact = _optional(_name, self.artifact, "Cutoff artifact")
        if source == DECISION_MODEL_COMPARISON:
            if artifact is None:
                raise ValueError("a Decision_Model comparison must name the Decision_Model artifact")
            if not scores:
                raise ValueError("a Decision_Model comparison records the calibrated probabilities as scores")
            for index, score in enumerate(scores):
                if score.score is None or not 0.0 <= score.score <= 1.0:
                    raise ValueError(
                        f"a Decision_Model comparison scores outcomes by probability in [0, 1]; "
                        f"score {index} is {score.score!r}"
                    )
        elif artifact is not None:
            raise ValueError(f"only a {DECISION_MODEL_COMPARISON!r} cut-off names an artifact, not {source!r}")

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {"source": self.source}
        if self.value is not None:
            members["value"] = self.value
        if self.scores:
            members["scores"] = [score.to_json() for score in self.scores]
        if self.artifact is not None:
            members["artifact"] = self.artifact
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Cutoff:
        obj = _members(value, location, ("source",), ("artifact", "scores", "value"))
        items = _read_optional(obj, "scores", location, lambda o, k, loc: _read_array(o, k, loc, non_empty=True))
        scores = tuple(
            CandidateScore.from_json(item, location=join_pointer(location, "scores", index))
            for index, item in enumerate(items or ())
        )
        return _build(
            cls,
            location,
            source=_read_str(obj, "source", location),
            value=_read_optional(obj, "value", location, _read_number),
            scores=scores,
            artifact=_read_optional(obj, "artifact", location, _read_str),
        )


@dataclass(frozen=True, slots=True)
class SampleInfo:
    """A Constraint derived from a seeded sample of ``sampled`` of ``total`` Observations (26.4, 29.8).

    ``1 <= sampled <= total``. The Support still counts all ``total``.
    """

    sampled: int
    total: int

    def __post_init__(self) -> None:
        sampled = _count(self.sampled, "SampleInfo sampled", minimum=1)
        total = _count(self.total, "SampleInfo total", minimum=1)
        if sampled > total:
            raise ValueError(f"SampleInfo sampled ({sampled}) must not exceed total ({total})")
        _set(self, "sampled", sampled)
        _set(self, "total", total)

    def to_json(self) -> dict[str, JSON]:
        return {"sampled": self.sampled, "total": self.total}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> SampleInfo:
        obj = _members(value, location, ("sampled", "total"))
        return _build(
            cls,
            location,
            sampled=_read_int(obj, "sampled", location),
            total=_read_int(obj, "total", location),
        )


# ---------------------------------------------------------------------------
# Relaxation records
# ---------------------------------------------------------------------------


class Removed(Enum):
    """The type of :data:`REMOVED`."""

    REMOVED = "removed"

    def __repr__(self) -> str:
        return "REMOVED"


# The ``after`` of a RelaxationRecord whose Constraint was removed. It is
# distinct from every JSON value, ``null`` included (``const: null`` is a
# Constraint).
REMOVED: Final = Removed.REMOVED


@dataclass(frozen=True, eq=False, slots=True)
class RelaxationRecord:
    """One soundness change to a Constraint (Requirement 3.6).

    ``before`` and ``after`` are JSON fragments of the schema object that
    express the Constraint before and after the change (for example
    ``{"maximum": 10}`` and ``{"maximum": 100}``); ``after`` is
    :data:`REMOVED` for a removal. ``documents`` holds the indices of the
    documents that triggered the change; it is non-empty and stored sorted
    without repeats. ``resolution`` holds the candidate relaxations
    considered, with their scores, for a change made under 3.3, and is
    ``None`` for a removal under 3.7.
    """

    before: JSON
    after: JSON | Removed
    documents: tuple[int, ...]
    resolution: Resolution | None = None

    def __post_init__(self) -> None:
        _set(self, "before", _payload(self.before, "RelaxationRecord before"))
        if self.after is not REMOVED:
            _set(self, "after", _payload(self.after, "RelaxationRecord after"))
        documents = self.documents
        if not isinstance(documents, (list, tuple, np.ndarray)):
            raise TypeError(f"RelaxationRecord documents must be a tuple of ints, got {type(documents).__name__}")
        indices = sorted({_count(index, "RelaxationRecord document index") for index in documents})
        if not indices:
            raise ValueError("RelaxationRecord documents must name at least one triggering document")
        _set(self, "documents", tuple(indices))
        _optional_instance(self.resolution, Resolution, "RelaxationRecord resolution")

    @property
    def removed(self) -> bool:
        """True when the change removed the Constraint."""
        return self.after is REMOVED

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {"before": canonical_value(self.before), "documents": list(self.documents)}
        if self.after is REMOVED:
            members["removed"] = True
        else:
            members["after"] = canonical_value(self.after)
        if self.resolution is not None:
            members["resolution"] = self.resolution.to_json()
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> RelaxationRecord:
        obj = _members(value, location, ("before", "documents"), ("after", "removed", "resolution"))
        if "removed" in obj:
            if obj["removed"] is not True:
                _fail(
                    join_pointer(location, "removed"),
                    f"expected true, got {_kind(obj['removed'])} (omit the member unless the Constraint was removed)",
                )
            if "after" in obj:
                _fail(location, "a removed Constraint has no 'after' member")
            after: Any = REMOVED
        elif "after" in obj:
            after = obj["after"]
        else:
            _fail(location, "missing member 'after' (or 'removed': true for a removal)")
        items = _read_array(obj, "documents", location)
        documents: list[int] = []
        for index, item in enumerate(items):
            if isinstance(item, bool) or not isinstance(item, int):
                _fail(join_pointer(location, "documents", index), f"expected an integer, got {_kind(item)}")
            if documents and item <= documents[-1]:
                _fail(join_pointer(location, "documents"), "document indices must be strictly ascending")
            documents.append(item)
        resolution = (
            Resolution.from_json(obj["resolution"], location=join_pointer(location, "resolution"))
            if "resolution" in obj
            else None
        )
        return _build(
            cls, location, before=obj["before"], after=after, documents=tuple(documents), resolution=resolution
        )

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, RelaxationRecord):
            return NotImplemented
        return self is other or canonical_bytes(self.to_json()) == canonical_bytes(other.to_json())

    __hash__ = None  # type: ignore[assignment]


# ---------------------------------------------------------------------------
# ProvenanceRecord
# ---------------------------------------------------------------------------

_RECORD_OPTIONAL: Final = ("changes", "cutoff", "details", "fallback", "resolution", "sample")


@dataclass(frozen=True, eq=False, slots=True)
class ProvenanceRecord:
    """The Provenance_Record of one Constraint (Requirement 26; fields in the module docstring).

    ``details`` is stored as a new dict of canonical JSON representatives in
    code-point key order; treat it as read-only. Use :meth:`laplace` for a
    record with the default Confidence.
    """

    technique: str
    support: Support
    confidence: float
    fallback: FallbackInfo | None = None
    resolution: Resolution | None = None
    cutoff: Cutoff | None = None
    sample: SampleInfo | None = None
    changes: tuple[RelaxationRecord, ...] = ()
    details: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        technique = _name(self.technique, "ProvenanceRecord technique")
        _instance(self.support, Support, "ProvenanceRecord support")
        _set(self, "confidence", _unit(self.confidence, "ProvenanceRecord confidence"))
        _optional_instance(self.fallback, FallbackInfo, "ProvenanceRecord fallback")
        _optional_instance(self.resolution, Resolution, "ProvenanceRecord resolution")
        _optional_instance(self.cutoff, Cutoff, "ProvenanceRecord cutoff")
        _optional_instance(self.sample, SampleInfo, "ProvenanceRecord sample")
        _set(self, "changes", _instances(self.changes, RelaxationRecord, "ProvenanceRecord changes"))
        _set(self, "details", self._normal_details(self.details))
        if technique.startswith(FALLBACK_PREFIX):
            if technique == FALLBACK_PREFIX:
                raise ValueError(f"fallback marker {technique!r} must name a Decision_Point kind")
            if self.fallback is None:
                raise ValueError(f"a Constraint produced by {technique!r} must record fallback information")
        if self.resolution is not None and self.resolution.method == "fallback" and self.fallback is None:
            raise ValueError("a Constraint from a fallback resolution must record fallback information")

    @staticmethod
    def _normal_details(details: Any) -> dict[str, JSON]:
        if not isinstance(details, Mapping):
            raise TypeError(f"ProvenanceRecord details must be a mapping, got {type(details).__name__}")
        for key in details:
            if not isinstance(key, str):
                raise TypeError(f"ProvenanceRecord details keys must be str, got {type(key).__name__}")
        return {
            key: _payload(details[key], f"ProvenanceRecord details[{key!r}]")
            for key in sorted(details, key=codepoint_key)
        }

    @classmethod
    def laplace(cls, technique: str, support: Support, **fields: Any) -> ProvenanceRecord:
        """A record whose Confidence is the default :func:`laplace_confidence` of ``support``."""
        return cls(technique, support, laplace_confidence(support), **fields)

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {
            "confidence": self.confidence,
            "support": self.support.to_json(),
            "technique": self.technique,
        }
        if self.changes:
            members["changes"] = [change.to_json() for change in self.changes]
        if self.details:
            members["details"] = canonical_value(dict(self.details))
        for name in ("cutoff", "fallback", "resolution", "sample"):
            part = getattr(self, name)
            if part is not None:
                members[name] = part.to_json()
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> ProvenanceRecord:
        obj = _members(value, location, ("confidence", "support", "technique"), _RECORD_OPTIONAL)

        def part(name: str, reader: Callable[..., T]) -> T | None:
            return reader(obj[name], location=join_pointer(location, name)) if name in obj else None

        changes = _read_optional(obj, "changes", location, lambda o, k, loc: _read_array(o, k, loc, non_empty=True))
        details = _read_optional(obj, "details", location, lambda o, k, loc: _read_object(o, k, loc, non_empty=True))
        return _build(
            cls,
            location,
            technique=_read_str(obj, "technique", location),
            support=Support.from_json(obj["support"], location=join_pointer(location, "support")),
            confidence=_read_number(obj, "confidence", location),
            fallback=part("fallback", FallbackInfo.from_json),
            resolution=part("resolution", Resolution.from_json),
            cutoff=part("cutoff", Cutoff.from_json),
            sample=part("sample", SampleInfo.from_json),
            changes=tuple(
                RelaxationRecord.from_json(item, location=join_pointer(location, "changes", index))
                for index, item in enumerate(changes or ())
            ),
            details=details or {},
        )

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ProvenanceRecord):
            return NotImplemented
        return self is other or canonical_bytes(self.to_json()) == canonical_bytes(other.to_json())

    __hash__ = None  # type: ignore[assignment]
