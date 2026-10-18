"""Provenance_Records: how each Constraint was derived (Requirement 26).

This module holds the record types and their JSON form (task 2.5), the
record-level rules of provenance finalization (task 17.1, below), and the
plain-text rendering of the ``explain`` command (task 17.3, at the end).

Record types
    :class:`ProvenanceRecord` is attached to one Constraint
    (``SchemaModel.provenance`` maps ConstraintIds to records). It holds:

    - ``technique``: the producing Technique id (Requirement 28.1),
      ``"fallback:<Decision_Point kind>"`` when a Fallback_Behavior produced
      the Constraint (28.12; see :func:`fallback_technique`), or
      ``"baseline:<rule>"`` when a Baseline_Rule produced it (26.13; see
      :func:`baseline_technique`);
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

    A record whose technique is a baseline marker names one of
    :data:`BASELINE_RULES` (``type``, ``required``) and carries neither
    ``fallback`` nor ``resolution``: a Baseline_Rule applies in every run,
    whatever the Technique states, and resolves no Decision_Point (26.13).
    Soundness relaxations of its Constraint still go in ``changes``. Neither
    marker is a Technique id, so neither is accepted where a Technique id is
    expected (:class:`FallbackInfo`, :class:`CandidateScore`).

Support and Confidence
    ``Support.ratio`` is ``consistent / total``, and 1.0 when ``total`` is 0
    (no Observation contradicts the Constraint); ``consistent <= total``
    always holds. Where a Constraint has no alternative (types, observed
    ``required``), the default Confidence is the Laplace rule of succession
    ``(consistent + 1) / (total + 2)`` (:func:`laplace_confidence`), which
    lies strictly inside (0, 1) and is 0.5 with no Observations. Otherwise
    the caller supplies the MDL posterior, the calibrated Decision_Model
    probability, the corpus conditional frequency, or a kind-specific value.

Finalization (26.1, 26.2, 26.13; task 17.1)
    After the soundness check, ``pipeline.infer.finalize_provenance``
    recomputes every Support and applies the rules here:

    - :func:`check_records`: the emitted Constraints (an emitted layout's
      pointer map) and the records correspond one to one. A Constraint
      without a record, or a record of no emitted Constraint, raises
      :class:`ProvenanceError`, an internal error: no emitted Constraint may
      lack one, Fallback_Behavior and Baseline_Rule Constraints included.
    - :func:`finalize_record` installs the final Support and, where the
      Confidence is the Laplace default (:func:`has_default_confidence`),
      recomputes it from that Support, since the soundness loop updates
      Supports but not Confidences. Defaults are the Confidences nothing
      else supplied: records that resolve no Decision_Point (Baseline_Rule
      records, ``x-latent-type``) and records resolved by Technique priority
      order or a Fallback_Behavior, unless the Technique has a kind-specific
      Confidence (:data:`KIND_SPECIFIC_CONFIDENCE`, for example the
      Co_Occurrence_Rule Confidence of 9.8). The MDL posterior and the
      calibrated Decision_Model probability are decision Confidences and
      stay as they are, on relaxed records too.

    Every other field (technique, fallback indicator, resolution, cut-off,
    sample counts, changes, details) is kept. A ``sample`` keeps the
    Technique's sampled and total counts (26.4); the Support always counts
    every non-flagged Observation, never the sample.

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

Explain rendering (26.9, 26.12; task 17.3)
    :func:`render_record` returns the plain text that ``jsinfer explain``
    prints for one record: every field the record holds, one per line, in
    a fixed order and under the names of its JSON form (see the function).
    :func:`explain` resolves a JSON Pointer against a schema and its
    explanation file and renders the record it finds. It reads and loads
    files, so it lives in ``io.explain`` (which imports this module) and is
    re-exported here lazily. Its failures raise :class:`ExplainError`,
    which names the failed input and the cause.
"""

from __future__ import annotations

import json
import math
import re
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field, replace
from enum import Enum
from typing import Any, Final, Literal, NoReturn, TypeAlias, TypeVar

import numpy as np

from jsinfer.core.determinism import canonical_bytes, codepoint_key
from jsinfer.core.jsonvalue import JSON, canonical_value
from jsinfer.core.schema_model import ConstraintId, join_pointer

__all__ = [
    "BASELINE_PREFIX",
    "BASELINE_RULES",
    "DECISION_MODEL_COMPARISON",
    "EXPLAIN_CHECKS",
    "EXPLAIN_INPUTS",
    "FALLBACK_PREFIX",
    "FALLBACK_REASONS",
    "KIND_SPECIFIC_CONFIDENCE",
    "MDL_COMPARISON",
    "REMOVED",
    "RESOLUTION_METHODS",
    "CandidateScore",
    "Cutoff",
    "ExplainCheck",
    "ExplainError",
    "ExplainInput",
    "FallbackInfo",
    "FallbackReason",
    "ProvenanceError",
    "ProvenanceFormatError",
    "ProvenanceRecord",
    "RelaxationRecord",
    "Removed",
    "Resolution",
    "ResolutionMethod",
    "SampleInfo",
    "Support",
    "baseline_technique",
    "check_records",
    "explain",
    "fallback_technique",
    "finalize_record",
    "has_default_confidence",
    "laplace_confidence",
    "render_record",
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

# Prefix of the technique marker of Constraints produced by a Baseline_Rule (26.13).
BASELINE_PREFIX: Final = "baseline:"

# The Baseline_Rules: the two Naive_Merge rules that every run applies, whatever
# the Technique states, and that resolve no Decision_Point (26.13). ``type``
# lists the JSON types observed at a Path (13.1, 13.2); ``required`` marks the
# fields present in every non-flagged object of a Variant (9.1, 9.2).
BASELINE_RULES: Final[tuple[str, ...]] = ("required", "type")

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
    if name.startswith(BASELINE_PREFIX):
        raise ValueError(f"{what} must be a Technique id, not the Baseline_Rule marker {name!r}")
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


def baseline_technique(rule: str) -> str:
    """The technique marker ``"baseline:<rule>"`` of a Constraint produced by a Baseline_Rule (26.13).

    ``rule`` is one of :data:`BASELINE_RULES`.
    """
    name = _name(rule, "Baseline_Rule")
    if name not in BASELINE_RULES:
        raise ValueError(f"unknown Baseline_Rule {name!r}; expected one of {list(BASELINE_RULES)}")
    return BASELINE_PREFIX + name


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
    record with the default Confidence, and :meth:`baseline` for the record
    of a Baseline_Rule's Constraint.
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
        if technique.startswith(BASELINE_PREFIX):
            if technique[len(BASELINE_PREFIX) :] not in BASELINE_RULES:
                raise ValueError(f"Baseline_Rule marker {technique!r} must name one of {list(BASELINE_RULES)}")
            if self.fallback is not None or self.resolution is not None:
                raise ValueError(
                    f"a Constraint produced by {technique!r} resolves no Decision_Point, "
                    "so its record holds no fallback information and no resolution"
                )
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

    @classmethod
    def baseline(cls, rule: str, support: Support, **fields: Any) -> ProvenanceRecord:
        """The record of a Constraint produced by the Baseline_Rule ``rule``, with the default Confidence (26.13)."""
        return cls.laplace(baseline_technique(rule), support, **fields)

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


# ---------------------------------------------------------------------------
# Finalization (task 17.1)
# ---------------------------------------------------------------------------

# Techniques whose Constraints carry a kind-specific Confidence even when Technique priority order (or no
# Decision_Point) produced them, so that finalization never replaces it with the Laplace default: the
# Co_Occurrence_Rule Confidence (9.8), the Exclusion_Rule Confidence (11.2), the Key, Reference, and
# Functional_Dependency Confidences (12.5), the Semantic_Type_Classifier's calibrated probability (13.5),
# and the corpus suggestion scores (24.3, 24.4). A Technique that adds such a Confidence joins this list.
KIND_SPECIFIC_CONFIDENCE: Final[tuple[str, ...]] = (
    "corpus.cf_suggest",
    "corpus.name_suggest",
    "rules.cooccurrence",
    "rules.exclusion",
    "rules.fd_ind",
    "rules.keys",
    "values.semantic_type",
)

# The resolution methods that supply no Confidence (None: the record resolves no Decision_Point).
_NO_CONFIDENCE_METHODS: Final = (None, "priority", "fallback")


def has_default_confidence(record: ProvenanceRecord) -> bool:
    """Whether ``record``'s Confidence is the Laplace default, which finalization refreshes (26.2).

    The default applies where nothing else supplies a Confidence: the
    record resolves no Decision_Point (a Baseline_Rule's record, an
    annotation such as ``x-latent-type``) or its Decision_Point was
    resolved by Technique priority order or a Fallback_Behavior, which give
    none (``DecisionEngine``; ``Decision.provenance`` then uses the Laplace
    default), and its Technique has no kind-specific Confidence
    (:data:`KIND_SPECIFIC_CONFIDENCE`). As a guard against a kind-specific
    value from elsewhere, a record that no soundness change touched must
    also hold exactly the Laplace value of its Support; a changed record's
    Support was recomputed by the soundness loop, which leaves Confidences
    alone, so it cannot be compared.

    Decision Confidences are never defaults: the MDL posterior (method
    ``mdl``, including an exception check that replaced an earlier step)
    and the calibrated Decision_Model probability (``decision-model``).
    """
    _instance(record, ProvenanceRecord, "record")
    method = None if record.resolution is None else record.resolution.method
    if method not in _NO_CONFIDENCE_METHODS or record.technique in KIND_SPECIFIC_CONFIDENCE:
        return False
    return bool(record.changes) or record.confidence == laplace_confidence(record.support)


def finalize_record(record: ProvenanceRecord, support: Support) -> ProvenanceRecord:
    """``record`` with its final ``support``, and its Laplace default Confidence refreshed from it.

    Every other field, a decision Confidence included, is kept (see
    :func:`has_default_confidence`).
    """
    _instance(support, Support, "support")
    confidence = laplace_confidence(support) if has_default_confidence(record) else record.confidence
    return replace(record, support=support, confidence=confidence)


def _cid_key(cid: ConstraintId) -> tuple[int, str, str, str]:
    """An order on ConstraintIds whose items may differ in type."""
    item = cid.item
    return (cid.nid, codepoint_key(cid.keyword), type(item).__name__, codepoint_key(str(item)))


class ProvenanceError(RuntimeError):
    """The Provenance_Records of a model do not match its emitted Constraints one to one (26.1).

    An internal inconsistency: every component gives each Constraint it
    emits a record, and the soundness loop keeps them in step with its
    changes. ``missing`` lists ``(ConstraintId, pointer)`` for each emitted
    Constraint without a record, in emitted order; ``orphaned`` lists the
    ConstraintIds of records that belong to no emitted Constraint, in
    ConstraintId order.
    """

    def __init__(
        self, missing: tuple[tuple[ConstraintId, str], ...], orphaned: tuple[ConstraintId, ...]
    ) -> None:
        super().__init__(missing, orphaned)
        self.missing = missing
        self.orphaned = orphaned

    def __str__(self) -> str:
        parts = []
        if self.missing:
            listed = ", ".join(f"{pointer or '(root)'} {cid}" for cid, pointer in self.missing)
            parts.append(f"{len(self.missing)} emitted Constraint(s) without a Provenance_Record: {listed}")
        if self.orphaned:
            listed = ", ".join(str(cid) for cid in self.orphaned)
            parts.append(f"{len(self.orphaned)} Provenance_Record(s) of no emitted Constraint: {listed}")
        return "; ".join(parts) or "Provenance_Records do not match the emitted Constraints"


def check_records(pointers: Mapping[ConstraintId, str], provenance: Mapping[ConstraintId, ProvenanceRecord]) -> None:
    """Check that the emitted Constraints (``pointers``) and the records correspond one to one (26.1).

    ``pointers`` is an emitted layout's ConstraintId → JSON Pointer map
    (``SchemaEmitter.layout``). Raises :class:`ProvenanceError` naming every
    Constraint without a record and every record without a Constraint.
    """
    missing = tuple((cid, pointer) for cid, pointer in pointers.items() if cid not in provenance)
    orphaned = tuple(sorted((cid for cid in provenance if cid not in pointers), key=_cid_key))
    if missing or orphaned:
        raise ProvenanceError(missing, orphaned)


# ---------------------------------------------------------------------------
# Explain rendering (task 17.3)
# ---------------------------------------------------------------------------

ExplainInput: TypeAlias = Literal["schema", "explanation", "pointer"]
ExplainCheck: TypeAlias = Literal["read", "parse", "syntax", "resolve", "entry"]

# The inputs of the explain command, in argument order (26.9).
EXPLAIN_INPUTS: Final[tuple[ExplainInput, ...]] = ("schema", "explanation", "pointer")

# The checks of the explain command, in the order they run (26.12):
#   read     the schema or explanation file cannot be read
#   parse    the schema cannot be loaded (not JSON, a bad root, or meta-schema
#            invalid) or the explanation file cannot be loaded
#   syntax   the JSON Pointer is not a syntactically valid JSON Pointer
#   resolve  the JSON Pointer names no location in the schema, or a location
#            that is not a Constraint
#   entry    the explanation file has no record keyed by the JSON Pointer
EXPLAIN_CHECKS: Final[tuple[ExplainCheck, ...]] = ("read", "parse", "syntax", "resolve", "entry")

_EXPLAIN_CHECKS_OF: Final[Mapping[str, tuple[str, ...]]] = {
    "schema": ("read", "parse"),
    "explanation": ("read", "parse"),
    "pointer": ("syntax", "resolve", "entry"),
}


class ExplainError(ValueError):
    """An explain request that prints no Provenance_Record (26.12).

    ``input`` is the failed input (:data:`EXPLAIN_INPUTS`) and ``check`` the
    failed check (:data:`EXPLAIN_CHECKS`): ``read`` and ``parse`` fail the
    schema or the explanation file, and ``syntax``, ``resolve``, and
    ``entry`` fail the JSON Pointer. ``name`` names the input as the
    message does (``"the schema file 'out/schema.json'"`` when it was read
    from a path, ``"the schema"`` or ``"the explanation file"`` for
    in-memory data, ``"the JSON Pointer '/type'"``), and ``reason`` states
    the cause; a pointer's reason names the schema or explanation file it
    was looked up in. The message is ``"<name>: <reason>"``. The CLI prints
    it and exits with status 2.
    """

    def __init__(self, input: ExplainInput, check: ExplainCheck, name: str, reason: str) -> None:
        if input not in EXPLAIN_INPUTS:
            raise ValueError(f"ExplainError input must be one of {list(EXPLAIN_INPUTS)}, got {input!r}")
        if check not in _EXPLAIN_CHECKS_OF[input]:
            raise ValueError(
                f"ExplainError check of the {input} must be one of {list(_EXPLAIN_CHECKS_OF[input])}, got {check!r}"
            )
        for what, text in (("name", name), ("reason", reason)):
            if not isinstance(text, str) or not text:
                raise ValueError(f"ExplainError {what} must be a non-empty str")
        super().__init__(input, check, name, reason)  # args survive pickling
        self.input: ExplainInput = input
        self.check: ExplainCheck = check
        self.name = name
        self.reason = reason

    def __str__(self) -> str:
        return f"{self.name}: {self.reason}"


_INDENT: Final = "  "

# The numeric members of a CandidateScore, in field order.
_CANDIDATE_NUMBERS: Final = ("score", "model_bits", "data_bits", "exception_bits", "n_rejected", "n_scored")

# Characters that str.splitlines() treats as line breaks but json.dumps(ensure_ascii=False) leaves
# unescaped, and lone surrogates, which strict UTF-8 cannot write. Rendered values escape them, so each
# field stays on its own line and the text always encodes.
_UNSAFE_IN_TEXT: Final = re.compile("[\x85\u2028\u2029\ud800-\udfff]")


def _json_text(value: JSON) -> str:
    """One-line JSON for a value: non-ASCII kept as is, line breaks and lone surrogates escaped."""
    text = json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(", ", ": "))
    return _UNSAFE_IN_TEXT.sub(lambda match: f"\\u{ord(match.group()):04x}", text)


def _plain_text(value: str, *, key: bool = False) -> str:
    """A text field as is, or as a JSON string literal when as-is text would be ambiguous.

    Text is quoted when it is empty, has leading or trailing whitespace,
    starts with ``"``, or holds a character that is not printable (control
    and format characters, line and paragraph separators, non-ASCII spaces,
    surrogates). A ``details`` key is also quoted when it holds ``:``, which
    separates it from its value.
    """
    plain = (
        bool(value)
        and value.isprintable()
        and value == value.strip()
        and not value.startswith('"')
        and not (key and ":" in value)
    )
    return value if plain else _json_text(value)


def _number_text(value: int | float) -> str:
    """A count exactly (big ints included); a float in its shortest round-trip form, as JSON writes it."""
    if isinstance(value, int):
        return str(value)
    return repr(value + 0.0)  # -0.0 is stored as 0.0 already; the addition keeps it so


def _field(out: list[str], depth: int, label: str, value: str | None = None) -> None:
    out.append(f"{_INDENT * depth}{label}:" if value is None else f"{_INDENT * depth}{label}: {value}")


def _candidate_fields(out: list[str], depth: int, candidate: CandidateScore) -> None:
    if candidate.technique is not None:
        _field(out, depth, "technique", _plain_text(candidate.technique))
    _field(out, depth, "outcome", _plain_text(candidate.outcome))
    for name in _CANDIDATE_NUMBERS:
        number = getattr(candidate, name)
        if number is not None:
            _field(out, depth, name, _number_text(number))


def _candidate_list(
    out: list[str], depth: int, label: str, candidates: tuple[CandidateScore, ...], selected: int | None = None
) -> None:
    _field(out, depth, label)
    for index, candidate in enumerate(candidates):
        mark = " (selected)" if index == selected else ""
        out.append(f"{_INDENT * (depth + 1)}[{index}]{mark}")
        _candidate_fields(out, depth + 2, candidate)


def _resolution_fields(out: list[str], depth: int, resolution: Resolution) -> None:
    _field(out, depth, "resolution")
    _field(out, depth + 1, "method", _plain_text(resolution.method))
    _field(out, depth + 1, "selected", str(resolution.selected))
    _candidate_list(out, depth + 1, "candidates", resolution.candidates, resolution.selected)


def render_record(record: ProvenanceRecord, pointer: str | None = None) -> str:
    """The plain text of every field of ``record``, as ``jsinfer explain`` prints it (26.9).

    One field per line, ``<name>: <value>``, with the names of the record's
    JSON form and nested parts indented by two spaces per level. With
    ``pointer``, a first line ``constraint: <pointer>`` names the
    Constraint. Then, in this order:

    - ``technique``: the producer, a Technique id, a ``fallback:`` marker,
      or a ``baseline:`` marker;
    - ``support`` with ``consistent``, ``total``, and ``ratio``;
    - ``confidence``;
    - ``fallback`` (the fallback indicator) with ``technique`` and
      ``reason``;
    - ``resolution`` with ``method``, ``selected`` (the index), and
      ``candidates``: each as ``[i]`` (``[i] (selected)`` for the selected
      one) followed by its ``technique``, ``outcome``, ``score``,
      ``model_bits``, ``data_bits``, ``exception_bits``, ``n_rejected``,
      and ``n_scored``;
    - ``cutoff`` with ``source``, ``value``, ``scores`` (listed like
      candidates), and ``artifact``;
    - ``sample`` with ``sampled`` and ``total``;
    - ``changes``: each as ``[i]`` followed by ``before``, ``after`` (or
      ``removed: true``), ``documents`` (comma-separated indices), and its
      ``resolution`` (as above);
    - ``details``: one line per member, in code-point order of the keys.

    Optional parts and members appear exactly when the record holds them,
    as in its JSON form (an absent fallback indicator or score prints no
    line). Counts print exactly, big integers included; floats print in
    the shortest form that round-trips, as the explanation file writes
    them (``-0.0`` is stored as ``0.0``). JSON payloads (``before``,
    ``after``, ``details`` values) print as one-line JSON of their
    canonical representatives (``2.0`` as ``2``, object keys in code-point
    order). Text fields print as they are unless that would be ambiguous,
    and then as JSON string literals (see ``_plain_text``). Non-ASCII
    characters print as they are; only characters that would break a line
    (U+0085, U+2028, U+2029) and lone surrogates are escaped. The text ends
    with a newline and depends only on the record and the pointer.
    """
    _instance(record, ProvenanceRecord, "record")
    out: list[str] = []
    if pointer is not None:
        if not isinstance(pointer, str):
            raise TypeError(f"pointer must be a str or None, got {type(pointer).__name__}")
        _field(out, 0, "constraint", _plain_text(pointer))
    _field(out, 0, "technique", _plain_text(record.technique))
    support = record.support
    _field(out, 0, "support")
    _field(out, 1, "consistent", _number_text(support.consistent))
    _field(out, 1, "total", _number_text(support.total))
    _field(out, 1, "ratio", _number_text(support.ratio))
    _field(out, 0, "confidence", _number_text(record.confidence))
    if record.fallback is not None:
        _field(out, 0, "fallback")
        _field(out, 1, "technique", _plain_text(record.fallback.technique))
        _field(out, 1, "reason", _plain_text(record.fallback.reason))
    if record.resolution is not None:
        _resolution_fields(out, 0, record.resolution)
    cutoff = record.cutoff
    if cutoff is not None:
        _field(out, 0, "cutoff")
        _field(out, 1, "source", _plain_text(cutoff.source))
        if cutoff.value is not None:
            _field(out, 1, "value", _number_text(cutoff.value))
        if cutoff.scores:
            _candidate_list(out, 1, "scores", cutoff.scores)
        if cutoff.artifact is not None:
            _field(out, 1, "artifact", _plain_text(cutoff.artifact))
    if record.sample is not None:
        _field(out, 0, "sample")
        _field(out, 1, "sampled", _number_text(record.sample.sampled))
        _field(out, 1, "total", _number_text(record.sample.total))
    if record.changes:
        _field(out, 0, "changes")
        for index, change in enumerate(record.changes):
            out.append(f"{_INDENT}[{index}]")
            _field(out, 2, "before", _json_text(canonical_value(change.before)))
            if change.removed:
                _field(out, 2, "removed", "true")
            else:
                _field(out, 2, "after", _json_text(canonical_value(change.after)))
            _field(out, 2, "documents", ", ".join(str(document) for document in change.documents))
            if change.resolution is not None:
                _resolution_fields(out, 2, change.resolution)
    if record.details:
        _field(out, 0, "details")
        for key, value in record.details.items():  # stored in code-point order of the keys
            _field(out, 1, _plain_text(key, key=True), _json_text(canonical_value(value)))
    return "\n".join(out) + "\n"


def __getattr__(name: str) -> Any:
    # explain lives in io.explain, which imports this module (and the Schema_Loader and Schema_Emitter, which
    # import it too), so it is re-exported on first use: ``from jsinfer.core.provenance import explain``.
    if name == "explain":
        from jsinfer.io.explain import explain

        return explain
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
