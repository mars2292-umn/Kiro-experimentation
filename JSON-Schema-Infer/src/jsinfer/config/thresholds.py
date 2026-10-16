"""The Learned_Threshold registry, Model_Bundle threshold records, and their validation (Requirement 20).

Every numeric cut-off at a Decision_Point that is not an MDL comparison or a
model-selection criterion such as BIC is a Learned_Threshold (20.1). Each one
is registered here, once, as a :class:`ThresholdSpec` (20.2). No spec has a
default value: a value comes only from a Model_Bundle, and a missing or
invalid value makes the reading Technique apply its Fallback_Behavior
(20.6). Technique ids and Decision_Point kinds are plain strings that match
``config.registry``; this module does not import the registry.

ThresholdSpec
    ``name`` (dotted, lowercase); ``techniques``, the ids of every Technique
    that reads it, in code-point order (``corpus.suggest_min_confidence`` is
    read by both suggestion Techniques, so this is a tuple); the
    ``decision_point`` kind where it applies; the ``objective``, a name from
    :data:`OBJECTIVES` that the Trainer maximizes on validation-split
    examples (20.3); the ``admissible`` :class:`~jsinfer.config.intervals.Interval`;
    the ``integer`` flag; the candidate ``grid``; and a ``description``.

    :data:`THRESHOLDS` maps every registered name to its spec in code-point
    order. :func:`lf_threshold` builds the ``lf.<id>.<name>`` specs of
    Labeling_Function cut-offs (23.1), which the weak supervision task adds
    to the table.

Grids
    The Trainer fits a threshold by a deterministic grid search that
    maximizes the objective, with ties going to the smallest value.
    ``spec.candidates(statistic)`` returns the grid as a strictly increasing
    float array of admissible values, integral for integer specs. The
    default grid (:func:`quantile_grid`) holds the finite closed ends of the
    admissible range plus the statistic's empirical quantiles: every
    distinct observed value by default (the exact breakpoints of a ``>=`` or
    ``>`` comparison), or ``points`` evenly spaced quantile levels with
    ``method="inverted_cdf"``, which also returns observed values. For
    integer specs both the floor and the ceiling of each value are offered.
    Non-finite statistics are ignored.

ThresholdRecord
    What the Model_Bundle stores per Learned_Threshold (20.4): the spec's
    name, techniques, Decision_Point kind, objective, admissible range, and
    integer flag, plus the fitted ``value``, the ``objective_value`` reached
    at it, the number ``n_examples`` (>= 1) of validation-split examples
    used, and the ``training_data`` identifier. A record is always valid: its
    value is finite, integral when ``integer`` is set (and then an ``int``,
    otherwise a ``float``), and inside its recorded range.

    JSON form: an object with exactly those ten members in code-point order,
    ``admissible`` in the Interval JSON form and ``techniques`` as a sorted
    array. ``ThresholdRecord.from_json`` accepts exactly this form, apart
    from number spelling and key order, and raises
    :class:`ThresholdFormatError` (a ``ValueError``) with the JSON Pointer of
    the offending member; ``from_json(r.to_json()) == r``.

Validation (20.6)
    :func:`check_threshold` classifies the raw JSON entry that a Model_Bundle
    holds for one spec. It returns the :class:`ThresholdRecord` when the
    entry is usable, and otherwise a :class:`ThresholdProblem` whose
    ``cause`` uses the report's substitution causes:

    - ``"missing"``: no entry (``None``), or an entry without a ``value``;
    - ``"invalid"``: the value is not a number, is NaN or infinite, is not
      an integer for an integer spec, or lies outside the spec's admissible
      range; the entry is not a well-formed record (which includes a value
      outside the range the record itself states); or the record describes
      a different spec (another name, Techniques, Decision_Point kind,
      objective, range, or integer flag), as a bundle trained against an
      older registry would.

    :func:`check_thresholds` applies this to several specs and returns a
    :class:`ThresholdCheck`. Picking the specs required by the enabled
    Techniques (:func:`thresholds_used_by`) and turning problems into
    report Substitutions with the applied Fallback_Behavior is the bundle
    reader's job.

Enumeration (20.2)
    :func:`list_thresholds` lists every registered Learned_Threshold with its
    name, Techniques, Decision_Point kind, and value in the loaded bundle
    (``None`` when it has none), for ``list-thresholds`` and the library API.
"""

from __future__ import annotations

import math
import re
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final, NoReturn, TypeAlias

import numpy as np

from jsinfer.config.intervals import Interval, IntervalFormatError, format_number, is_number
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.provenance import FALLBACK_PREFIX
from jsinfer.core.report import SUBSTITUTION_CAUSES, SubstitutionCause
from jsinfer.core.schema_model import join_pointer

__all__ = [
    "OBJECTIVES",
    "THRESHOLDS",
    "WEAK_SUPERVISION",
    "GridFn",
    "ThresholdCheck",
    "ThresholdFormatError",
    "ThresholdListing",
    "ThresholdProblem",
    "ThresholdRecord",
    "ThresholdSpec",
    "check_threshold",
    "check_thresholds",
    "lf_threshold",
    "list_thresholds",
    "quantile_grid",
    "threshold_spec",
    "thresholds_used_by",
]

# A grid maps a spec and the statistic values observed on validation-split
# examples to candidate threshold values.
GridFn: TypeAlias = Callable[["ThresholdSpec", np.ndarray], np.ndarray]

# Validation-split objectives the Trainer maximizes (20.3, 20.4): name -> what it measures.
OBJECTIVES: Final[Mapping[str, str]] = MappingProxyType(
    {
        "boundary_f1": (
            "boundary F1 (within train.cp_tolerance documents) on concatenations of validation "
            "schemas and their mutated successors"
        ),
        "cooccurrence_f1": "F1 of the emitted dependentRequired entries versus the source schema",
        "corrupt_document_f1": (
            "F1 of flagging mutation-generated corrupt documents injected into validation collections"
        ),
        "exclusion_f1": "F1 of the Exclusion_Rules versus the source schema",
        "grouping_ari": "adjusted Rand index of the Variant grouping versus the ground-truth branches",
        "half_split_f1": (
            "F1 where a candidate found on one half of a collection is true when it still holds on the other half"
        ),
        "lf_accuracy": "accuracy of the Labeling_Function on validation labels",
        "semantic_format_f1": "F1 of the emitted Format or Pattern versus the declared Format",
        "suggestion_f1": "F1 of the accepted suggestions versus the source Constraints",
        "tagged_union_f1": "F1 of the Tagged_Union branches versus the source schema",
    }
)

# The Technique that reads Labeling_Function thresholds (23.1).
WEAK_SUPERVISION: Final = "decide.weak_supervision"

_NAME_RE: Final = re.compile(r"[a-z][a-z0-9_]*(?:\.[a-z0-9][a-z0-9_-]*)+")
_LF_PART_RE: Final = re.compile(r"[a-z0-9][a-z0-9_-]*")
_RECORD_MEMBERS: Final = (
    "admissible",
    "decision_point",
    "integer",
    "n_examples",
    "name",
    "objective",
    "objective_value",
    "techniques",
    "training_data",
    "value",
)


# ---------------------------------------------------------------------------
# Construction checks
# ---------------------------------------------------------------------------


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _text(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a str, got {type(value).__name__}")
    if not value:
        raise ValueError(f"{what} must be non-empty")
    return value


def _threshold_name(value: Any, what: str) -> str:
    name = _text(value, what)
    if not _NAME_RE.fullmatch(name):
        raise ValueError(f"{what} must be a dotted lowercase name such as 'dbscan.eps', got {name!r}")
    return name


def _techniques(value: Any, what: str) -> tuple[str, ...]:
    if not isinstance(value, (tuple, list)):
        raise TypeError(f"{what} must be a tuple of Technique ids, got {type(value).__name__}")
    ids = tuple(_text(item, f"{what} entries") for item in value)
    if not ids:
        raise ValueError(f"{what} must name at least one Technique")
    for technique in ids:
        if technique.startswith(FALLBACK_PREFIX):
            raise ValueError(f"{what} must be Technique ids, not the fallback marker {technique!r}")
    if any(codepoint_key(a) >= codepoint_key(b) for a, b in zip(ids, ids[1:])):
        raise ValueError(f"{what} must be unique and in code-point order, got {list(ids)}")
    return ids


def _flag(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise TypeError(f"{what} must be a bool, got {type(value).__name__}")
    return value


def _real(value: Any, what: str) -> float:
    if not is_number(value):
        raise TypeError(f"{what} must be a number, got {type(value).__name__}")
    try:
        x = float(value)
    except OverflowError:
        raise ValueError(f"{what} is too large for a float: {value!r}") from None
    if not math.isfinite(x):
        raise ValueError(f"{what} must be finite, got {x!r}")
    return x + 0.0  # -0.0 becomes 0.0


def _count(value: Any, what: str, minimum: int) -> int:
    if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, np.integer)):
        raise TypeError(f"{what} must be an int, got {type(value).__name__}")
    n = int(value)
    if n < minimum:
        raise ValueError(f"{what} must be >= {minimum}, got {n}")
    return n


def _kind(value: Any) -> str:
    """How a raw JSON value reads in a message."""
    if value is None:
        return "null"
    if isinstance(value, (bool, np.bool_)):
        return "a boolean"
    if is_number(value):
        return "a number"
    if isinstance(value, str):
        return "a string"
    if isinstance(value, (list, tuple)):
        return "an array"
    if isinstance(value, dict):
        return "an object"
    return f"a {type(value).__name__}"


def _value_problem(value: Any, admissible: Interval, integer: bool) -> str | None:
    """Why ``value`` is not a usable threshold value for the range and flag, or None."""
    if not is_number(value):
        return f"value is not a number (got {_kind(value)})"
    if isinstance(value, (float, np.floating)) and not math.isfinite(value):
        return f"value {format_number(value)} is not finite"
    if integer:
        if isinstance(value, (float, np.floating)) and not float(value).is_integer():
            return f"value {format_number(value)} is not an integer"
    elif isinstance(value, (int, np.integer)):
        try:
            float(value)
        except OverflowError:
            return "value is too large for a float"
    if not admissible.contains(value):
        return f"value {format_number(value)} is outside the admissible range {admissible.render()}"
    return None


def _normalized_value(value: Any, integer: bool) -> int | float:
    """A checked value as ``int`` (integer specs) or ``float``."""
    if integer:
        return int(value)
    return float(value) + 0.0


# ---------------------------------------------------------------------------
# Specs and grids
# ---------------------------------------------------------------------------


def quantile_grid(spec: ThresholdSpec, statistic: Any, points: int | None = None) -> np.ndarray:
    """Candidates: the finite closed ends of the admissible range plus quantiles of ``statistic``.

    With ``points=None`` every distinct finite observed value is a quantile;
    otherwise ``points`` (>= 2) evenly spaced levels from 0 to 1 are taken
    with ``method="inverted_cdf"``. Integer specs get the floor and ceiling of
    each value. Inadmissible values are dropped; the result is sorted and
    unique (see the module docstring). Use ``functools.partial`` to bind
    ``points`` in a spec's ``grid``.
    """
    stat = np.asarray(statistic, dtype=np.float64).ravel()
    stat = stat[np.isfinite(stat)]
    if points is not None:
        levels = np.linspace(0.0, 1.0, _count(points, "points", 2))
        stat = np.quantile(stat, levels, method="inverted_cdf") if stat.size else stat
    if spec.integer:
        stat = np.concatenate((np.floor(stat), np.ceil(stat)))
    values = np.concatenate((stat, np.asarray(spec.admissible.closed_ends(), dtype=np.float64))) + 0.0
    keep = np.fromiter((spec.admissible.contains(float(v)) for v in values), dtype=bool, count=values.size)
    return np.unique(values[keep])


@dataclass(frozen=True, slots=True)
class ThresholdSpec:
    """One registered Learned_Threshold; see the module docstring.

    Equality ignores ``grid`` (functions compare by identity).
    """

    name: str
    techniques: tuple[str, ...]
    decision_point: str
    objective: str
    admissible: Interval
    integer: bool
    description: str
    grid: GridFn = field(default=quantile_grid, compare=False, repr=False)

    def __post_init__(self) -> None:
        _threshold_name(self.name, "ThresholdSpec name")
        _set(self, "techniques", _techniques(self.techniques, f"ThresholdSpec {self.name!r} techniques"))
        _text(self.decision_point, "ThresholdSpec decision_point")
        if self.objective not in OBJECTIVES:
            raise ValueError(
                f"ThresholdSpec {self.name!r} objective must be one of {sorted(OBJECTIVES)}, got {self.objective!r}"
            )
        if not isinstance(self.admissible, Interval):
            raise TypeError(f"ThresholdSpec admissible must be an Interval, got {type(self.admissible).__name__}")
        _flag(self.integer, "ThresholdSpec integer")
        _text(self.description, "ThresholdSpec description")
        if not callable(self.grid):
            raise TypeError("ThresholdSpec grid must be callable")

    def accepted(self) -> str:
        """The admissible values in words, for example ``"an integer in [2, inf)"``."""
        return self.admissible.describe(integer=self.integer)

    def check_value(self, value: Any) -> str | None:
        """Why ``value`` is not a usable value of this threshold (20.6), or None when it is."""
        return _value_problem(value, self.admissible, self.integer)

    def normalize_value(self, value: Any) -> int | float:
        """``value`` as ``int`` (integer specs) or ``float``; raises ``ValueError`` when it is not usable."""
        problem = self.check_value(value)
        if problem is not None:
            raise ValueError(f"Learned_Threshold {self.name!r}: {problem}")
        return _normalized_value(value, self.integer)

    def candidates(self, statistic: Any) -> np.ndarray:
        """The grid for ``statistic``: a non-empty, strictly increasing float array of admissible values."""
        grid = np.asarray(self.grid(self, np.asarray(statistic, dtype=np.float64)), dtype=np.float64)
        if grid.ndim != 1 or grid.size == 0:
            raise ValueError(f"the grid of {self.name!r} must be a non-empty 1-d array, got shape {grid.shape}")
        if np.any(np.diff(grid) <= 0.0):
            raise ValueError(f"the grid of {self.name!r} must be strictly increasing")
        for value in grid.tolist():
            problem = self.check_value(value)
            if problem is not None:
                raise ValueError(f"the grid of {self.name!r} holds an unusable candidate: {problem}")
        return grid


def lf_threshold(
    lf_id: str,
    name: str,
    *,
    decision_point: str,
    admissible: Interval,
    integer: bool,
    description: str,
) -> ThresholdSpec:
    """The spec of the Labeling_Function cut-off ``lf.<lf_id>.<name>`` (23.1).

    It is read by ``decide.weak_supervision`` at the Labeling_Function's
    Judgment_Call kind, and fitted on the LF's validation-label accuracy.
    """
    for part, what in ((lf_id, "Labeling_Function id"), (name, "LF threshold name")):
        if not isinstance(part, str) or not _LF_PART_RE.fullmatch(part):
            raise ValueError(f"{what} must be lowercase letters, digits, '_' or '-', got {part!r}")
    return ThresholdSpec(
        name=f"lf.{lf_id}.{name}",
        techniques=(WEAK_SUPERVISION,),
        decision_point=decision_point,
        objective="lf_accuracy",
        admissible=admissible,
        integer=integer,
        description=description,
    )


_UNIT: Final = Interval(0, 1)
_NON_NEGATIVE: Final = Interval(0)
_POSITIVE_COUNT: Final = Interval(1)

_SPECS: Final[tuple[ThresholdSpec, ...]] = (
    # Variant discovery (5.7, 5.8)
    ThresholdSpec(
        name="lsh.keyset_jaccard",
        techniques=("variants.minhash_lsh",),
        decision_point="variant_grouping",
        objective="grouping_ari",
        admissible=_UNIT,
        integer=False,
        description=(
            "minimum MinHash-estimated key-set Jaccard similarity at which an LSH candidate pair of "
            "Object_Signatures joins one candidate Variant group (5.7)"
        ),
    ),
    ThresholdSpec(
        name="dbscan.eps",
        techniques=("variants.dbscan",),
        decision_point="variant_grouping",
        objective="grouping_ari",
        admissible=Interval(0, 1, low_open=True),  # scikit-learn requires eps > 0
        integer=False,
        description="DBSCAN neighborhood radius in Jaccard distance between Object_Signatures (5.8)",
    ),
    ThresholdSpec(
        name="dbscan.min_samples",
        techniques=("variants.dbscan",),
        decision_point="variant_grouping",
        objective="grouping_ari",
        admissible=_POSITIVE_COUNT,
        integer=True,
        description=(
            "DBSCAN minimum neighbor count (total signature weight in a neighborhood, the signature "
            "included) of a core signature (5.8)"
        ),
    ),
    ThresholdSpec(
        name="hdbscan.min_cluster_size",
        techniques=("variants.hdbscan",),
        decision_point="variant_grouping",
        objective="grouping_ari",
        admissible=Interval(2),  # scikit-learn requires min_cluster_size >= 2
        integer=True,
        description="HDBSCAN minimum cluster size (5.8)",
    ),
    # Inter-field rules (9.6, 9.8, 10.3-10.8, 11.4, 12.6)
    ThresholdSpec(
        name="cooccur.min_support",
        techniques=("rules.cooccurrence",),
        decision_point="cooccurrence_rule",
        objective="cooccurrence_f1",
        admissible=_POSITIVE_COUNT,
        integer=True,
        description=(
            "minimum Support count (non-flagged objects of the Variant that contain the antecedent "
            "field) of a Co_Occurrence_Rule candidate; smaller candidates are discarded (9.6)"
        ),
    ),
    ThresholdSpec(
        name="cooccur.min_confidence",
        techniques=("rules.cooccurrence",),
        decision_point="cooccurrence_rule",
        objective="cooccurrence_f1",
        admissible=_UNIT,
        integer=False,
        description="minimum Confidence of an emitted Co_Occurrence_Rule when MDL selection is disabled (9.8)",
    ),
    ThresholdSpec(
        name="tagged.branch_support",
        techniques=("rules.decision_trees",),
        decision_point="discriminator",
        objective="tagged_union_f1",
        admissible=_UNIT,
        integer=False,
        description=(
            "minimum Support ratio of a Discriminator value group for a dedicated Tagged_Union branch (10.3-10.5, 10.8)"
        ),
    ),
    ThresholdSpec(
        name="exclusion.min_confidence",
        techniques=("rules.exclusion",),
        decision_point="exclusion_rule",
        objective="exclusion_f1",
        admissible=_UNIT,
        integer=False,
        description="minimum Confidence of an Exclusion_Rule that does not reduce total description length (11.4)",
    ),
    ThresholdSpec(
        name="keys.min_confidence",
        techniques=("rules.keys",),
        decision_point="key",
        objective="half_split_f1",
        admissible=_UNIT,
        integer=False,
        description="minimum Confidence of a Key_Constraint candidate (12.6)",
    ),
    ThresholdSpec(
        name="ind.min_confidence",
        techniques=("rules.fd_ind",),
        decision_point="reference",
        objective="half_split_f1",
        admissible=_UNIT,
        integer=False,
        description="minimum Confidence of a Reference_Constraint (inclusion dependency) candidate (12.6)",
    ),
    ThresholdSpec(
        name="fd.min_confidence",
        techniques=("rules.fd_ind",),
        decision_point="fd",
        objective="half_split_f1",
        admissible=_UNIT,
        integer=False,
        description="minimum Confidence of a Functional_Dependency candidate (12.6)",
    ),
    # Values (13.7)
    ThresholdSpec(
        name="semantic.min_probability",
        techniques=("values.semantic_type",),
        decision_point="string_constraint",
        objective="semantic_format_f1",
        admissible=_UNIT,
        integer=False,
        description=(
            "minimum calibrated probability of the predicted semantic type for the Semantic_Type_Classifier "
            "to produce a Format or Pattern candidate (13.7)"
        ),
    ),
    # Robustness (17.1, 18.5)
    ThresholdSpec(
        name="iforest.score",
        techniques=("robust.isolation_forest",),
        decision_point="document_outlier",
        objective="corrupt_document_f1",
        admissible=_UNIT,  # -score_samples of an Isolation Forest lies in (0, 1]
        integer=False,
        description="Isolation Forest anomaly score above which a document is flagged (17.1)",
    ),
    ThresholdSpec(
        name="lof.score",
        techniques=("robust.lof",),
        decision_point="document_outlier",
        objective="corrupt_document_f1",
        admissible=_NON_NEGATIVE,  # the local outlier factor is positive
        integer=False,
        description="Local Outlier Factor anomaly score above which a document is flagged (17.1)",
    ),
    ThresholdSpec(
        name="lof.n_neighbors",
        techniques=("robust.lof",),
        decision_point="document_outlier",
        objective="corrupt_document_f1",
        admissible=_POSITIVE_COUNT,
        integer=True,
        description="number of neighbors of the Local Outlier Factor (17.1)",
    ),
    ThresholdSpec(
        name="changepoint.penalty",
        techniques=("robust.change_point",),
        decision_point="version_split",
        objective="boundary_f1",
        admissible=_NON_NEGATIVE,
        integer=False,
        description="per-change penalty in bits added to the MDL segmentation criterion (18.5)",
    ),
    ThresholdSpec(
        name="changepoint.min_segment",
        techniques=("robust.change_point",),
        decision_point="version_split",
        objective="boundary_f1",
        admissible=_POSITIVE_COUNT,
        integer=True,
        description="minimum number of documents in a Schema_Version segment (18.5)",
    ),
    # Corpus (24.5)
    ThresholdSpec(
        name="corpus.suggest_min_confidence",
        techniques=("corpus.cf_suggest", "corpus.name_suggest"),
        decision_point="suggestion",
        objective="suggestion_f1",
        admissible=_UNIT,
        integer=False,
        description="minimum Confidence at which a Corpus_Advisor suggestion is accepted (24.5)",
    ),
)


def _registry(specs: Iterable[ThresholdSpec]) -> Mapping[str, ThresholdSpec]:
    table: dict[str, ThresholdSpec] = {}
    for spec in sorted(specs, key=lambda s: codepoint_key(s.name)):
        if spec.name in table:
            raise ValueError(f"Learned_Threshold {spec.name!r} is registered twice")
        table[spec.name] = spec
    return MappingProxyType(table)


# Every registered Learned_Threshold, by name in code-point order (20.2).
THRESHOLDS: Final[Mapping[str, ThresholdSpec]] = _registry(_SPECS)


def threshold_spec(name: str) -> ThresholdSpec:
    """The registered spec of ``name``; ``KeyError`` naming every registered threshold otherwise."""
    try:
        return THRESHOLDS[name]
    except KeyError:
        raise KeyError(f"unknown Learned_Threshold {name!r}; the registered ones are {list(THRESHOLDS)}") from None


def thresholds_used_by(technique_ids: Iterable[str], specs: Iterable[ThresholdSpec] | None = None) -> tuple[ThresholdSpec, ...]:
    """The specs read by at least one of ``technique_ids``, in name order (default: every registered spec)."""
    wanted = frozenset(technique_ids)
    pool = THRESHOLDS.values() if specs is None else specs
    chosen = [spec for spec in pool if any(t in wanted for t in spec.techniques)]
    return tuple(sorted(chosen, key=lambda s: codepoint_key(s.name)))


# ---------------------------------------------------------------------------
# Records
# ---------------------------------------------------------------------------


class ThresholdFormatError(ValueError):
    """Malformed threshold-record JSON.

    ``location`` is the JSON Pointer of the offending member, relative to the
    ``location`` passed to ``from_json``; ``reason`` says what is wrong.
    """

    def __init__(self, location: str, reason: str) -> None:
        super().__init__(location, reason)  # args survive pickling
        self.location = location
        self.reason = reason

    def __str__(self) -> str:
        where = repr(self.location) if self.location else "the top level"
        return f"invalid threshold record at {where}: {self.reason}"


def _fail(location: str, reason: str) -> NoReturn:
    raise ThresholdFormatError(location, reason)


def _member(obj: dict[str, Any], key: str, location: str, check: Callable[[Any], bool], expected: str) -> Any:
    value = obj[key]
    if not check(value):
        _fail(join_pointer(location, key), f"expected {expected}, got {_kind(value)}")
    return value


def _is_str(value: Any) -> bool:
    return isinstance(value, str)


def _is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _is_finite_number(value: Any) -> bool:
    return is_number(value) and (not isinstance(value, float) or math.isfinite(value))


@dataclass(frozen=True, slots=True)
class ThresholdRecord:
    """A fitted Learned_Threshold as stored in a Model_Bundle (20.4); see the module docstring."""

    name: str
    techniques: tuple[str, ...]
    decision_point: str
    objective: str
    admissible: Interval
    integer: bool
    value: int | float
    objective_value: float
    n_examples: int
    training_data: str

    def __post_init__(self) -> None:
        _threshold_name(self.name, "ThresholdRecord name")
        _set(self, "techniques", _techniques(self.techniques, "ThresholdRecord techniques"))
        _text(self.decision_point, "ThresholdRecord decision_point")
        _text(self.objective, "ThresholdRecord objective")
        if not isinstance(self.admissible, Interval):
            raise TypeError(f"ThresholdRecord admissible must be an Interval, got {type(self.admissible).__name__}")
        _flag(self.integer, "ThresholdRecord integer")
        if not is_number(self.value):
            raise TypeError(f"ThresholdRecord value must be a number, got {type(self.value).__name__}")
        problem = _value_problem(self.value, self.admissible, self.integer)
        if problem is not None:
            raise ValueError(f"ThresholdRecord {self.name!r}: {problem}")
        _set(self, "value", _normalized_value(self.value, self.integer))
        _set(self, "objective_value", _real(self.objective_value, "ThresholdRecord objective_value"))
        _set(self, "n_examples", _count(self.n_examples, "ThresholdRecord n_examples", 1))
        _text(self.training_data, "ThresholdRecord training_data")

    @classmethod
    def from_spec(
        cls, spec: ThresholdSpec, *, value: Any, objective_value: Any, n_examples: int, training_data: str
    ) -> ThresholdRecord:
        """The record of ``spec`` fitted to ``value`` (what the Trainer writes)."""
        return cls(
            name=spec.name,
            techniques=spec.techniques,
            decision_point=spec.decision_point,
            objective=spec.objective,
            admissible=spec.admissible,
            integer=spec.integer,
            value=value,
            objective_value=objective_value,
            n_examples=n_examples,
            training_data=training_data,
        )

    def mismatch(self, spec: ThresholdSpec) -> str | None:
        """How the record's description differs from ``spec``, or None when it describes ``spec``."""
        fields = (
            ("name", self.name, spec.name),
            ("techniques", list(self.techniques), list(spec.techniques)),
            ("decision_point", self.decision_point, spec.decision_point),
            ("objective", self.objective, spec.objective),
            ("integer", self.integer, spec.integer),
        )
        for label, recorded, registered in fields:
            if recorded != registered:
                return f"the record's {label} {recorded!r} differs from the registered {registered!r}"
        if self.admissible.to_json() != spec.admissible.to_json():
            return (
                f"the record's admissible range {self.admissible.render()} differs from the registered "
                f"{spec.admissible.render()}"
            )
        return None

    def to_json(self) -> dict[str, JSON]:
        return {
            "admissible": self.admissible.to_json(),
            "decision_point": self.decision_point,
            "integer": self.integer,
            "n_examples": self.n_examples,
            "name": self.name,
            "objective": self.objective,
            "objective_value": self.objective_value,
            "techniques": list(self.techniques),
            "training_data": self.training_data,
            "value": self.value,
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> ThresholdRecord:
        if not isinstance(value, dict):
            _fail(location, f"expected an object, got {_kind(value)}")
        for key in value:
            if key not in _RECORD_MEMBERS:
                _fail(location, f"unknown member {key!r}; the members are {list(_RECORD_MEMBERS)}")
        for key in _RECORD_MEMBERS:
            if key not in value:
                _fail(location, f"missing member {key!r}")
        string = _is_str
        techniques = _member(value, "techniques", location, lambda v: isinstance(v, list), "an array")
        for i, item in enumerate(techniques):
            if not isinstance(item, str):
                _fail(join_pointer(location, "techniques", i), f"expected a string, got {_kind(item)}")
        try:
            admissible = Interval.from_json(value["admissible"], location=join_pointer(location, "admissible"))
        except IntervalFormatError as error:
            raise ThresholdFormatError(error.location, error.reason) from None
        kwargs = {
            "name": _member(value, "name", location, string, "a string"),
            "techniques": tuple(techniques),
            "decision_point": _member(value, "decision_point", location, string, "a string"),
            "objective": _member(value, "objective", location, string, "a string"),
            "admissible": admissible,
            "integer": _member(value, "integer", location, lambda v: isinstance(v, bool), "a boolean"),
            "value": _member(value, "value", location, _is_finite_number, "a finite number"),
            "objective_value": _member(value, "objective_value", location, _is_finite_number, "a finite number"),
            "n_examples": _member(value, "n_examples", location, _is_int, "an integer"),
            "training_data": _member(value, "training_data", location, string, "a string"),
        }
        try:
            return cls(**kwargs)
        except (TypeError, ValueError) as error:
            raise ThresholdFormatError(location, str(error)) from None


# ---------------------------------------------------------------------------
# Validation against the registry (20.6)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ThresholdProblem:
    """A required Learned_Threshold the Model_Bundle lacks or holds an unusable value for (20.6).

    ``techniques`` and ``decision_point`` come from the spec: each enabled one
    of these Techniques applies its Fallback_Behavior. ``cause`` is
    ``"missing"`` or ``"invalid"``; ``detail`` says what is wrong.
    """

    name: str
    techniques: tuple[str, ...]
    decision_point: str
    cause: SubstitutionCause
    detail: str

    def __post_init__(self) -> None:
        _threshold_name(self.name, "ThresholdProblem name")
        _set(self, "techniques", _techniques(self.techniques, "ThresholdProblem techniques"))
        _text(self.decision_point, "ThresholdProblem decision_point")
        if self.cause not in SUBSTITUTION_CAUSES:
            raise ValueError(f"ThresholdProblem cause must be one of {list(SUBSTITUTION_CAUSES)}, got {self.cause!r}")
        _text(self.detail, "ThresholdProblem detail")

    def __str__(self) -> str:
        return f"Learned_Threshold {self.name!r} is {self.cause}: {self.detail}"


def _problem(spec: ThresholdSpec, cause: SubstitutionCause, detail: str) -> ThresholdProblem:
    return ThresholdProblem(spec.name, spec.techniques, spec.decision_point, cause, detail)


def check_threshold(spec: ThresholdSpec, entry: Any) -> ThresholdRecord | ThresholdProblem:
    """Classify the raw bundle ``entry`` (parsed JSON, or None when absent) for ``spec`` (20.6)."""
    if entry is None:
        return _problem(spec, "missing", "the Model_Bundle has no record of it")
    if not isinstance(entry, dict):
        return _problem(spec, "invalid", f"its record is {_kind(entry)}, not an object")
    if "value" not in entry:
        return _problem(spec, "missing", "its record has no value")
    detail = spec.check_value(entry["value"])
    if detail is not None:
        return _problem(spec, "invalid", detail)
    try:
        record = ThresholdRecord.from_json(entry)
    except ThresholdFormatError as error:
        return _problem(spec, "invalid", f"its record is malformed: {error}")
    mismatch = record.mismatch(spec)
    if mismatch is not None:
        return _problem(spec, "invalid", mismatch)
    return record


@dataclass(frozen=True)
class ThresholdCheck:
    """The outcome of :func:`check_thresholds`: usable records and problems, both in name order."""

    records: Mapping[str, ThresholdRecord]
    problems: tuple[ThresholdProblem, ...]

    @property
    def values(self) -> dict[str, int | float]:
        """Name -> fitted value of every usable record."""
        return {name: record.value for name, record in self.records.items()}


def check_thresholds(specs: Iterable[ThresholdSpec], entries: Mapping[str, Any]) -> ThresholdCheck:
    """Check the bundle ``entries`` (name -> raw record JSON) of each spec; other entries are ignored."""
    records: dict[str, ThresholdRecord] = {}
    problems: list[ThresholdProblem] = []
    seen: list[str] = []
    for spec in sorted(specs, key=lambda s: codepoint_key(s.name)):
        if seen and seen[-1] == spec.name:
            raise ValueError(f"Learned_Threshold {spec.name!r} is checked twice")
        seen.append(spec.name)
        result = check_threshold(spec, entries.get(spec.name))
        if isinstance(result, ThresholdRecord):
            records[spec.name] = result
        else:
            problems.append(result)
    return ThresholdCheck(MappingProxyType(records), tuple(problems))


# ---------------------------------------------------------------------------
# Enumeration (20.2)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ThresholdListing:
    """One row of the Learned_Threshold enumeration: ``value`` is None when the bundle has none."""

    name: str
    techniques: tuple[str, ...]
    decision_point: str
    value: int | float | None


def list_thresholds(
    values: Mapping[str, int | float] | None = None, specs: Iterable[ThresholdSpec] | None = None
) -> tuple[ThresholdListing, ...]:
    """Every registered Learned_Threshold (or each of ``specs``) with its value in ``values``, in name order."""
    found = {} if values is None else values
    pool = THRESHOLDS.values() if specs is None else specs
    return tuple(
        ThresholdListing(spec.name, spec.techniques, spec.decision_point, found.get(spec.name))
        for spec in sorted(pool, key=lambda s: codepoint_key(s.name))
    )
