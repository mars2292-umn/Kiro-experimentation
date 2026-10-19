"""The Inference_Report: the machine-readable summary of one run (Requirements 20.5, 27.8, 27.10).

:class:`InferenceReport` is the per-run report builder that the pipeline
(``InferenceContext.report``) and the stages fill in. The same object is the
report carried by ``InferenceResult.report`` and, after an unrecoverable
error, by ``InferenceError.partial_report`` (27.13). ``report.json`` holds
its canonical JSON form (:meth:`InferenceReport.to_bytes`). Run metadata
gathering and pipeline orchestration (``pipeline.infer``) and file writing
(``cli``) live elsewhere; this module holds the data model, the builder, and
the stage timer.

Sections
    The JSON form is an object with exactly the thirteen sections of the
    design's report table, each always present:

    =================  =============================================================
    ``run``            :class:`RunInfo`: effective Run_Configuration, seed, Model_Bundle
                       id, jsinfer and CPython versions, locked package versions
                       (27.8). ``{}`` until recorded
    ``ingestion``      ``errors``: parse failures and unreadable files
                       (:class:`IngestError`) in ingestion order; ``skipped``: their
                       count (1.7)
    ``thresholds``     every Learned_Threshold read: name → value and bundle id (20.5)
    ``resources``      every Resource_Parameter read: name → value and origin (20.5)
    ``substitutions``  missing or invalid Learned_Thresholds and models with the
                       fallback applied (:class:`Substitution`; 20.6, 21.10, 22.14, 24.9)
    ``outliers``       Flagged_Items (:class:`FlaggedItem`; 17.5, 17.6) and whether
                       outlier exclusion was abandoned, with the discarded count (17.9)
    ``versions``       :class:`VersionsInfo`: whether change-point detection was
                       applied, the Ordering_Key choice, criterion values, and segments
                       (18.3–18.6, 18.10, 18.11). ``{}`` until recorded
    ``decisions``      one :class:`DecisionEntry` per resolved Decision_Point: kind,
                       Path, method, candidates and scores, cut-off (19.5, 20.7)
    ``rejected``       rejected candidates and suggestions (:class:`RejectedCandidate`),
                       grouped by :data:`REJECTED_CATEGORIES`
    ``soundness``      one :class:`SoundnessSummary` per Schema_Version or for the
                       single schema: the counts of 3.4 and the removed Constraints (3.6)
    ``nearest``        :class:`NearestInfo`: nearest-known-schema results or why there
                       are none (25.2–25.6). ``{}`` until recorded
    ``timings``        wall-clock seconds per completed stage, keyed by component name
                       (27.8)
    ``status``         :class:`Status`: ``ok``, or ``failed`` with the stage and the
                       error (27.10)
    =================  =============================================================

    Rejected-candidate categories: ``defs`` (Shared_Definition candidates,
    8.6), ``enums`` (open enum judgments, 15.8), ``field_groups`` (6.7),
    ``formats`` (Formats and semantic types, 13.6, 13.7), ``keys`` (keys,
    references, FDs, 12.6), ``patterns`` (14.7, 14.8), ``rules``
    (co-occurrence, Discriminator, and Exclusion_Rule candidates, 9.6, 10.1,
    10.3, 11.2–11.4), and ``suggestions`` (24.6, 24.7). Each entry holds the
    Path, the candidate as JSON, the producing Technique, and kind-specific
    ``details`` (counts, thresholds, description lengths, failed conditions).

Stage timer
    ``with report.stage("Ingestor"): ...`` times the block with
    ``time.perf_counter`` (monotonic) and adds the seconds to the component's
    ``timings`` entry. A component timed more than once (for example once per
    segment) reports the ``math.fsum`` of its runs. Stages may nest; each is
    timed inclusively, and a stage cannot be re-entered while it runs. When
    the block raises, the stage has not completed: it gets no timing, the
    status becomes ``failed`` with the stage and an :class:`ErrorInfo`, and
    the exception propagates unchanged. The first failure is kept, so with
    nested stages the innermost one is named. Errors that a requirement
    resolves by skipping, falling back, or recording and continuing must be
    handled inside the stage: an exception that leaves a stage is
    unrecoverable (27.10). The report is then the partial report of 27.10 and
    27.13, which lists every completed stage with its runtime.

Canonical order
    No section depends on the order in which entries were recorded, except
    ``ingestion.errors``, which keeps the Ingestor's order (input order, 1.5):

    - name-keyed sections (``thresholds``, ``resources``, ``timings``,
      ``packages``) are in code-point key order;
    - ``substitutions`` are unique and sorted by their fields;
    - Flagged_Items are unique per (document, Path) and sorted by document
      index, then Path (canonical Path order);
    - decisions and rejected candidates are sorted by Path, then kind, then
      their canonical bytes;
    - soundness summaries and nearest results are unique per version and
      sorted with the single schema (version ``None``) first;
    - within records, sets (effects, Path lists, document indices) are sorted
      and deduplicated, and neighbors are ranked by similarity (descending),
      then source (ascending), as 25.2 specifies.

    Paths are rendered with ``core.paths.render_path`` (``$.user.email``);
    Path strings are parsed and re-rendered, so ``$["user"]`` is stored as
    ``$.user``.

Reproducibility (27.12)
    Everything in the report is determined by the inputs except the
    per-stage runtimes, the run timestamps (``run.started`` and
    ``run.finished``), and the worker count. :meth:`InferenceReport.comparable_json`
    removes exactly these: the ``timings`` section, the two timestamps, and
    the :data:`WORKERS` setting at the top level or under the ``run`` or
    ``resources`` table of ``run.config`` and in ``resources``. Two runs that
    satisfy 27.5 have equal comparable forms.

Numbers and text
    Counts are ints >= 0. Scores, similarities, criterion values, and seconds
    are finite floats (ints and numpy scalars are converted, ``-0.0`` becomes
    ``0.0``). Learned_Threshold and Resource_Parameter values keep int or
    float. Booleans are rejected wherever a number is expected, and NaN and
    infinities everywhere. Text must be valid Unicode (no unpaired
    surrogates), so the strict UTF-8 output encoder cannot fail at write
    time; :meth:`ErrorInfo.from_exception` escapes surrogates instead, so that
    reporting a failure cannot fail. JSON payloads (``config``, ``details``,
    candidates, Ordering_Key values) are stored as canonical representatives
    (``core.jsonvalue.canonical_value``).

JSON form
    ``to_json()`` returns plain JSON with object keys in code-point order at
    every level, so ``core.determinism.encode_canonical`` writes the same
    bytes whether or not it sorts keys. Absent optional members are omitted
    rather than written as ``null``; lists that a requirement asks to be
    reported even when empty (Flagged_Item effects, removed Constraints,
    neighbor and Path lists) are always written. :meth:`InferenceReport.from_json`
    accepts exactly this form, apart from the spelling of numbers and the
    order of object keys, and raises :class:`ReportFormatError` (a
    ``ValueError``) with the JSON Pointer of the offending member otherwise;
    that includes entries out of canonical order, repeated entries, and
    derived members (``skipped``, ``found``) that disagree with the data. For
    every report ``r``, ``InferenceReport.from_json(r.to_json()) == r`` and
    the loaded report writes identical bytes.

Equality
    Records holding only typed fields compare field by field. Records holding
    JSON payloads, and :class:`InferenceReport` itself, compare by the
    canonical bytes of their JSON form (Python's ``==`` would equate ``true``
    with ``1``) and are unhashable.
"""

from __future__ import annotations

import math
import re
import time
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field, replace
from types import TracebackType
from typing import Any, Final, Literal, NoReturn, TypeAlias, TypeVar

import numpy as np

from jsinfer.core.determinism import canonical_bytes, codepoint_key, encode_canonical
from jsinfer.core.jsonvalue import JSON, canonical_value
from jsinfer.core.paths import Step, parse_path, path_sort_key, render_path
from jsinfer.core.provenance import (
    BASELINE_PREFIX,
    FALLBACK_PREFIX,
    Cutoff,
    FallbackInfo,
    ProvenanceFormatError,
    RelaxationRecord,
    Resolution,
)
from jsinfer.core.schema_model import join_pointer, split_pointer

__all__ = [
    "NEAREST_STATUSES",
    "ORDERING_KEY_SOURCES",
    "REJECTED_CATEGORIES",
    "RESOURCE_ORIGINS",
    "SECTIONS",
    "SUBSTITUTION_CAUSES",
    "WORKERS",
    "ChangePointCriterion",
    "DecisionEntry",
    "ErrorInfo",
    "FlagScore",
    "FlaggedItem",
    "InferenceReport",
    "IngestError",
    "NearestInfo",
    "NearestResult",
    "NearestStatus",
    "Neighbor",
    "OrderingKey",
    "OrderingKeySource",
    "RejectedCandidate",
    "RejectedCategory",
    "RemovedConstraint",
    "ReportFormatError",
    "ResourceOrigin",
    "ResourceUse",
    "RunInfo",
    "Segment",
    "SoundnessSummary",
    "Status",
    "StatusState",
    "Substitution",
    "SubstitutionCause",
    "ThresholdUse",
    "VersionsInfo",
]

T = TypeVar("T")

RejectedCategory: TypeAlias = Literal[
    "defs", "enums", "field_groups", "formats", "keys", "patterns", "rules", "suggestions"
]
ResourceOrigin: TypeAlias = Literal["default", "file", "flag"]
SubstitutionCause: TypeAlias = Literal["checksum", "invalid", "missing"]
OrderingKeySource: TypeAlias = Literal["configured", "document-index", "selected"]
NearestStatus: TypeAlias = Literal["disabled", "empty-index", "searched"]
StatusState: TypeAlias = Literal["failed", "ok"]

# The top-level sections of report.json, in code-point (output) order.
SECTIONS: Final = (
    "decisions",
    "ingestion",
    "nearest",
    "outliers",
    "rejected",
    "resources",
    "run",
    "soundness",
    "status",
    "substitutions",
    "thresholds",
    "timings",
    "versions",
)

# Groups of the ``rejected`` section, in code-point order (see the module docstring).
REJECTED_CATEGORIES: Final[tuple[RejectedCategory, ...]] = (
    "defs",
    "enums",
    "field_groups",
    "formats",
    "keys",
    "patterns",
    "rules",
    "suggestions",
)

# Where a Resource_Parameter value came from (RunConfiguration.provenance_of):
# the default, or supplied by the user in the configuration file or a flag.
RESOURCE_ORIGINS: Final[tuple[ResourceOrigin, ...]] = ("default", "file", "flag")

# Why an artifact was substituted: it is absent, or present but unusable
# (a non-finite or out-of-range Learned_Threshold, 20.6).
SUBSTITUTION_CAUSES: Final[tuple[SubstitutionCause, ...]] = ("checksum", "invalid", "missing")

# How the Ordering_Key was chosen: configured (18.2), selected from the date
# Paths (18.3), or the document index (18.4, 18.11).
ORDERING_KEY_SOURCES: Final[tuple[OrderingKeySource, ...]] = ("configured", "document-index", "selected")

# The nearest-known-schema lookup: disabled (25.5), skipped because the
# Corpus_Index is empty (25.6), or searched with per-schema results (25.2–25.4).
NEAREST_STATUSES: Final[tuple[NearestStatus, ...]] = ("disabled", "empty-index", "searched")

# The worker-count setting, exempt from the reproducibility comparison (27.12).
WORKERS: Final = "workers"

_WORKER_COUNT_KEYS: Final = ((WORKERS,), ("run", WORKERS), ("resources", WORKERS))
_RUN_TIMESTAMPS: Final = ("finished", "started")
_SURROGATE_RE: Final = re.compile(r"[\ud800-\udfff]")

# The stage clock. Monotonic; module-level so tests can substitute a fake clock.
_clock: Callable[[], float] = time.perf_counter


# ---------------------------------------------------------------------------
# Construction checks
# ---------------------------------------------------------------------------


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _text(value: Any, what: str, *, empty: bool = False) -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a str, got {type(value).__name__}")
    if not value and not empty:
        raise ValueError(f"{what} must be non-empty")
    bad = _SURROGATE_RE.search(value)
    if bad is not None:
        raise ValueError(
            f"{what} contains the unpaired surrogate U+{ord(bad.group()):04X} at index {bad.start()}; "
            "report text must be valid Unicode"
        )
    return value


def _technique_id(value: Any, what: str) -> str:
    name = _text(value, what)
    if name.startswith(FALLBACK_PREFIX):
        raise ValueError(f"{what} must be a Technique id, not the fallback marker {name!r}")
    if name.startswith(BASELINE_PREFIX):
        raise ValueError(f"{what} must be a Technique id, not the Baseline_Rule marker {name!r}")
    return name


def _choice(value: Any, choices: tuple[str, ...], what: str) -> Any:
    if not isinstance(value, str) or value not in choices:
        raise ValueError(f"{what} must be one of {list(choices)}, got {value!r}")
    return value


def _flag(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise TypeError(f"{what} must be a bool, got {type(value).__name__}")
    return value


def _count(value: Any, what: str, minimum: int = 0) -> int:
    if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, np.integer)):
        raise TypeError(f"{what} must be an int, got {type(value).__name__}")
    n = int(value)
    if n < minimum:
        raise ValueError(f"{what} must be >= {minimum}, got {n}")
    return n


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


def _number(value: Any, what: str) -> int | float:
    """A finite number that keeps int or float (threshold and Resource_Parameter values)."""
    if isinstance(value, (int, np.integer)) and not isinstance(value, (bool, np.bool_)):
        return int(value)
    return _real(value, what)


def _seconds(value: Any, what: str) -> float:
    x = _real(value, what)
    if x < 0.0:
        raise ValueError(f"{what} must be >= 0 seconds, got {x!r}")
    return x


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


def _text_set(value: Any, what: str) -> tuple[str, ...]:
    """Strings as a sorted tuple without repeats (code-point order)."""
    if isinstance(value, str) or not isinstance(value, (list, tuple, set, frozenset)):
        raise TypeError(f"{what} must be a collection of str, got {type(value).__name__}")
    return tuple(sorted({_text(item, f"{what} entries") for item in value}, key=codepoint_key))


def _check_unicode(value: JSON, what: str) -> None:
    pending: list[Any] = [value]
    while pending:
        item = pending.pop()
        if isinstance(item, str):
            _text(item, what, empty=True)
        elif isinstance(item, list):
            pending.extend(item)
        elif isinstance(item, dict):
            for key, member in item.items():
                _text(key, f"{what} keys", empty=True)
                pending.append(member)


def _payload(value: Any, what: str) -> JSON:
    try:
        result = canonical_value(value)
    except (TypeError, ValueError) as error:
        raise type(error)(f"{what} must be a JSON value: {error}") from None
    _check_unicode(result, what)
    return result


def _json_object(value: Any, what: str) -> dict[str, JSON]:
    """A str-keyed mapping of JSON values as a new dict in code-point key order."""
    if not isinstance(value, Mapping):
        raise TypeError(f"{what} must be a mapping, got {type(value).__name__}")
    for key in value:
        if not isinstance(key, str):
            raise TypeError(f"{what} keys must be str, got {type(key).__name__}")
        _text(key, f"{what} keys", empty=True)
    return {key: _payload(value[key], f"{what}[{key!r}]") for key in sorted(value, key=codepoint_key)}


def _path(value: Any, what: str) -> str:
    """A Path or Path string, returned as its canonical rendering."""
    if isinstance(value, str):
        try:
            steps = parse_path(value)
        except ValueError as error:
            raise ValueError(f"{what}: {error}") from None
    elif isinstance(value, (tuple, list)) and all(isinstance(step, Step) for step in value):
        steps = tuple(value)
    else:
        raise TypeError(f"{what} must be a Path or a Path string, got {type(value).__name__}")
    return render_path(steps)


def _path_key(path: str) -> tuple[tuple[int, str], ...]:
    return path_sort_key(parse_path(path))


def _version_key(version: int | None) -> int:
    return -1 if version is None else version


def _ordered(members: dict[str, Any]) -> dict[str, Any]:
    """The members with keys in code-point order (the JSON form's key order)."""
    return {key: members[key] for key in sorted(members, key=codepoint_key)}


class _JsonEquality:
    """Equality by the canonical bytes of ``to_json()``, for records that hold JSON payloads."""

    __slots__ = ()

    def to_json(self) -> dict[str, JSON]:  # pragma: no cover - every subclass defines it
        raise NotImplementedError

    def __eq__(self, other: object) -> bool:
        if type(other) is not type(self):
            return NotImplemented
        return self is other or canonical_bytes(self.to_json()) == canonical_bytes(other.to_json())  # type: ignore[attr-defined]

    __hash__ = None  # type: ignore[assignment]


# ---------------------------------------------------------------------------
# JSON reading
# ---------------------------------------------------------------------------


class ReportFormatError(ValueError):
    """Malformed report JSON.

    ``location`` is the JSON Pointer of the offending member, relative to the
    document whose pointer was passed as ``location`` to ``from_json`` (the
    value itself by default). ``reason`` says what is wrong. Errors in
    embedded provenance records are reported as ReportFormatErrors too.
    """

    def __init__(self, location: str, reason: str) -> None:
        super().__init__(location, reason)  # args survive pickling
        self.location = location
        self.reason = reason

    def __str__(self) -> str:
        where = repr(self.location) if self.location else "the top level"
        return f"invalid report JSON at {where}: {self.reason}"


def _fail(location: str, reason: str) -> NoReturn:
    raise ReportFormatError(location, reason)


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


def _mapping(value: Any, location: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(location, f"expected an object, got {_kind(value)}")
    return value


class _Obj:
    """A JSON object being read, with typed member readers that fail at the member's pointer."""

    __slots__ = ("location", "value")

    def __init__(
        self, value: Any, location: str, required: tuple[str, ...], optional: tuple[str, ...] = ()
    ) -> None:
        obj = _mapping(value, location)
        allowed = required + optional
        for key in obj:
            if key not in allowed:
                _fail(location, f"unknown member {key!r}; the members are {sorted(allowed)}")
        for key in required:
            if key not in obj:
                _fail(location, f"missing member {key!r}")
        self.value = obj
        self.location = location

    def at(self, *tokens: str | int) -> str:
        return join_pointer(self.location, *tokens)

    def has(self, key: str) -> bool:
        return key in self.value

    def read_int(self, key: str) -> int:
        item = self.value[key]
        if isinstance(item, bool) or not isinstance(item, int):
            _fail(self.at(key), f"expected an integer, got {_kind(item)}")
        return item

    def read_number(self, key: str) -> int | float:
        item = self.value[key]
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            _fail(self.at(key), f"expected a number, got {_kind(item)}")
        if isinstance(item, float) and not math.isfinite(item):
            _fail(self.at(key), f"expected a finite number, got {item!r}")
        return item

    def read_str(self, key: str) -> str:
        item = self.value[key]
        if not isinstance(item, str):
            _fail(self.at(key), f"expected a string, got {_kind(item)}")
        return item

    def read_bool(self, key: str) -> bool:
        item = self.value[key]
        if not isinstance(item, bool):
            _fail(self.at(key), f"expected a boolean, got {_kind(item)}")
        return item

    def read_array(self, key: str) -> list[Any]:
        item = self.value[key]
        if not isinstance(item, list):
            _fail(self.at(key), f"expected an array, got {_kind(item)}")
        return item

    def read_strs(self, key: str) -> list[str]:
        items = self.read_array(key)
        for index, item in enumerate(items):
            if not isinstance(item, str):
                _fail(self.at(key, index), f"expected a string, got {_kind(item)}")
        return items

    def read_object(self, key: str, *, non_empty: bool = False) -> dict[str, Any]:
        item = self.value[key]
        if not isinstance(item, dict):
            _fail(self.at(key), f"expected an object, got {_kind(item)}")
        if non_empty and not item:
            _fail(self.at(key), "expected a non-empty object (omit the member when there are no entries)")
        return item

    def optional(self, key: str, read: Callable[[str], T]) -> T | None:
        """Read an optional member; an explicit ``null`` is rejected by ``read`` (absent members are omitted)."""
        return read(key) if key in self.value else None


def _build(cls: Callable[..., T], location: str, *args: Any, **kwargs: Any) -> T:
    """Construct a record (or call a builder method), reporting errors as ReportFormatErrors at ``location``."""
    try:
        return cls(*args, **kwargs)
    except ReportFormatError:
        raise
    except (TypeError, ValueError) as error:
        raise ReportFormatError(location, str(error)) from None


def _nested(read: Callable[..., T], value: Any, location: str) -> T:
    """Read an embedded provenance record, reporting its errors as ReportFormatErrors."""
    try:
        return read(value, location=location)
    except ProvenanceFormatError as error:
        raise ReportFormatError(error.location, error.reason) from None


def _first_difference(expected: Any, actual: Any, location: str) -> str | None:
    """The pointer of the first member where ``actual`` differs from ``expected`` as JSON, or None."""
    try:
        if canonical_bytes(expected) == canonical_bytes(actual):
            return None
    except (TypeError, ValueError):
        return location
    if isinstance(expected, dict) and isinstance(actual, dict) and expected.keys() == actual.keys():
        for key in sorted(expected, key=codepoint_key):
            found = _first_difference(expected[key], actual[key], join_pointer(location, key))
            if found is not None:
                return found
    elif isinstance(expected, list) and isinstance(actual, list) and len(expected) == len(actual):
        for index, (want, got) in enumerate(zip(expected, actual)):
            found = _first_difference(want, got, join_pointer(location, index))
            if found is not None:
                return found
    return location


# ---------------------------------------------------------------------------
# run
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class RunInfo(_JsonEquality):
    """Run metadata for the ``run`` section (27.8).

    ``config`` is the effective Run_Configuration as a JSON object (the
    content of ``effective-config.toml``). ``packages`` maps each locked
    package name to its exact version; a run records the installed version
    of each runtime package of the lock file (``pipeline.infer.LOCKED_PACKAGES``).
    ``started`` and ``finished`` are optional run timestamps (a run records
    both, in UTC, RFC 3339; ``finished`` is added by
    :meth:`InferenceReport.finish_run`); they and the worker count inside
    ``config`` are exempt from the reproducibility comparison (27.12).
    """

    config: Mapping[str, JSON]
    seed: int
    bundle_id: str
    jsinfer_version: str
    python_version: str
    packages: Mapping[str, str] = field(default_factory=dict)
    started: str | None = None
    finished: str | None = None

    def __post_init__(self) -> None:
        _set(self, "config", _json_object(self.config, "RunInfo config"))
        _set(self, "seed", _count(self.seed, "RunInfo seed"))
        _text(self.bundle_id, "RunInfo bundle_id")
        _text(self.jsinfer_version, "RunInfo jsinfer_version")
        _text(self.python_version, "RunInfo python_version")
        packages = self.packages
        if not isinstance(packages, Mapping):
            raise TypeError(f"RunInfo packages must be a mapping, got {type(packages).__name__}")
        _set(
            self,
            "packages",
            {
                _text(name, "RunInfo package name"): _text(packages[name], f"RunInfo version of {name!r}")
                for name in sorted(packages, key=lambda name: _text(name, "RunInfo package name"))
            },
        )
        _optional(_text, self.started, "RunInfo started")
        _optional(_text, self.finished, "RunInfo finished")

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, Any] = {
            "bundle_id": self.bundle_id,
            "config": canonical_value(dict(self.config)),
            "jsinfer_version": self.jsinfer_version,
            "packages": dict(self.packages),
            "python_version": self.python_version,
            "seed": self.seed,
        }
        for name in _RUN_TIMESTAMPS:
            value = getattr(self, name)
            if value is not None:
                members[name] = value
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> RunInfo:
        obj = _Obj(
            value,
            location,
            ("bundle_id", "config", "jsinfer_version", "packages", "python_version", "seed"),
            _RUN_TIMESTAMPS,
        )
        packages = obj.read_object("packages")
        for name, version in packages.items():
            if not isinstance(version, str):
                _fail(obj.at("packages", name), f"expected a string, got {_kind(version)}")
        return _build(
            cls,
            location,
            config=obj.read_object("config"),
            seed=obj.read_int("seed"),
            bundle_id=obj.read_str("bundle_id"),
            jsinfer_version=obj.read_str("jsinfer_version"),
            python_version=obj.read_str("python_version"),
            packages=packages,
            started=obj.optional("started", obj.read_str),
            finished=obj.optional("finished", obj.read_str),
        )


# ---------------------------------------------------------------------------
# ingestion, thresholds, resources, substitutions
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class IngestError:
    """A skipped document or file (1.7): ``source`` path, 1-based ``line`` (None for an unreadable file), ``reason``."""

    source: str
    reason: str
    line: int | None = None

    def __post_init__(self) -> None:
        _text(self.source, "IngestError source")
        _text(self.reason, "IngestError reason")
        _set(self, "line", _optional(lambda v, w: _count(v, w, minimum=1), self.line, "IngestError line"))

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {"reason": self.reason, "source": self.source}
        if self.line is not None:
            members["line"] = self.line
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> IngestError:
        obj = _Obj(value, location, ("reason", "source"), ("line",))
        return _build(
            cls,
            location,
            source=obj.read_str("source"),
            reason=obj.read_str("reason"),
            line=obj.optional("line", obj.read_int),
        )


@dataclass(frozen=True, slots=True)
class ThresholdUse:
    """A Learned_Threshold read during the run: its value and the supplying Model_Bundle (20.5)."""

    value: int | float
    bundle_id: str

    def __post_init__(self) -> None:
        _set(self, "value", _number(self.value, "Learned_Threshold value"))
        _text(self.bundle_id, "Learned_Threshold bundle_id")

    def to_json(self) -> dict[str, JSON]:
        return {"bundle_id": self.bundle_id, "value": self.value}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> ThresholdUse:
        obj = _Obj(value, location, ("bundle_id", "value"))
        return _build(cls, location, value=obj.read_number("value"), bundle_id=obj.read_str("bundle_id"))


@dataclass(frozen=True, slots=True)
class ResourceUse:
    """A Resource_Parameter read during the run: its value and origin (20.5).

    ``origin`` is ``"default"``, or ``"file"`` or ``"flag"`` for a value the
    user supplied in the Run_Configuration.
    """

    value: int | float
    origin: ResourceOrigin

    def __post_init__(self) -> None:
        _set(self, "value", _number(self.value, "Resource_Parameter value"))
        _choice(self.origin, RESOURCE_ORIGINS, "Resource_Parameter origin")

    @property
    def user_supplied(self) -> bool:
        """True when the user supplied the value (in the configuration file or a flag)."""
        return self.origin != "default"

    def to_json(self) -> dict[str, JSON]:
        return {"origin": self.origin, "value": self.value}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> ResourceUse:
        obj = _Obj(value, location, ("origin", "value"))
        return _build(cls, location, value=obj.read_number("value"), origin=obj.read_str("origin"))


@dataclass(frozen=True, slots=True)
class Substitution:
    """A missing or invalid artifact replaced by a fallback (20.6, 21.10, 22.14, 24.9).

    ``artifact`` names the Learned_Threshold or the model (Decision_Model,
    Semantic_Type_Classifier, Corpus_Index). ``fallback`` names the Technique
    whose Fallback_Behavior (or, for a missing Decision_Model, the next
    resolution step) was applied and the reason, ``"missing-threshold"`` or
    ``"missing-model"``. ``cause`` says whether the artifact was missing or
    present but invalid. ``applied`` describes what was used instead.
    """

    artifact: str
    fallback: FallbackInfo
    cause: SubstitutionCause
    applied: str

    def __post_init__(self) -> None:
        _text(self.artifact, "Substitution artifact")
        fallback = _instance(self.fallback, FallbackInfo, "Substitution fallback")
        if fallback.reason == "disabled":
            raise ValueError("a Substitution replaces a missing or invalid artifact; its fallback reason cannot be 'disabled'")
        _choice(self.cause, SUBSTITUTION_CAUSES, "Substitution cause")
        _text(self.applied, "Substitution applied")

    def sort_key(self) -> tuple[str, str, str, str, str]:
        return (self.artifact, self.fallback.technique, self.fallback.reason, self.cause, self.applied)

    def to_json(self) -> dict[str, JSON]:
        return {
            "applied": self.applied,
            "artifact": self.artifact,
            "cause": self.cause,
            "fallback": self.fallback.to_json(),
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Substitution:
        obj = _Obj(value, location, ("applied", "artifact", "cause", "fallback"))
        return _build(
            cls,
            location,
            artifact=obj.read_str("artifact"),
            fallback=_nested(FallbackInfo.from_json, obj.value["fallback"], obj.at("fallback")),
            cause=obj.read_str("cause"),
            applied=obj.read_str("applied"),
        )


# ---------------------------------------------------------------------------
# outliers
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class FlagScore:
    """One flagging Technique's evidence (17.5).

    For Isolation Forest and Local Outlier Factor, ``score`` is the anomaly
    score and ``compared`` the Learned_Threshold. For MDL exception encoding,
    they are the description lengths in bits of flagging and of widening.
    """

    technique: str
    score: float
    compared: float

    def __post_init__(self) -> None:
        _technique_id(self.technique, "FlagScore technique")
        _set(self, "score", _real(self.score, "FlagScore score"))
        _set(self, "compared", _real(self.compared, "FlagScore compared"))

    def to_json(self) -> dict[str, JSON]:
        return {"compared": self.compared, "score": self.score, "technique": self.technique}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> FlagScore:
        obj = _Obj(value, location, ("compared", "score", "technique"))
        return _build(
            cls,
            location,
            technique=obj.read_str("technique"),
            score=obj.read_number("score"),
            compared=obj.read_number("compared"),
        )


@dataclass(frozen=True, slots=True)
class FlaggedItem:
    """The report entry of one Flagged_Item (17.5).

    ``document`` is the document index and ``path`` the Path of the item
    (``"$"`` for a whole document). ``flags`` holds one :class:`FlagScore`
    per flagging Technique (non-empty, stored sorted by Technique).
    ``effects`` lists the Constraints the item would have loosened or the
    Variant it would have added, possibly none (stored sorted, without repeats).
    """

    document: int
    path: str
    flags: tuple[FlagScore, ...]
    effects: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        _set(self, "document", _count(self.document, "FlaggedItem document"))
        _set(self, "path", _path(self.path, "FlaggedItem path"))
        flags = sorted(_instances(self.flags, FlagScore, "FlaggedItem flags"), key=lambda flag: flag.technique)
        if not flags:
            raise ValueError("a FlaggedItem names at least one flagging Technique")
        for before, after in zip(flags, flags[1:]):
            if before.technique == after.technique:
                raise ValueError(f"FlaggedItem flags name Technique {after.technique!r} more than once")
        _set(self, "flags", tuple(flags))
        _set(self, "effects", _text_set(self.effects, "FlaggedItem effects"))

    @property
    def identity(self) -> tuple[int, str]:
        """``(document, path)``: the report has one entry per Flagged_Item."""
        return (self.document, self.path)

    def to_json(self) -> dict[str, JSON]:
        return {
            "document": self.document,
            "effects": list(self.effects),
            "flags": [flag.to_json() for flag in self.flags],
            "path": self.path,
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> FlaggedItem:
        obj = _Obj(value, location, ("document", "effects", "flags", "path"))
        flags = tuple(
            FlagScore.from_json(item, location=obj.at("flags", index))
            for index, item in enumerate(obj.read_array("flags"))
        )
        return _build(
            cls,
            location,
            document=obj.read_int("document"),
            path=obj.read_str("path"),
            flags=flags,
            effects=tuple(obj.read_strs("effects")),
        )


# ---------------------------------------------------------------------------
# versions
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class OrderingKey:
    """How the Ordering_Key was chosen (18.2–18.4, 18.11).

    ``source`` is ``"configured"`` or ``"selected"`` with the Ordering_Key
    ``path``, or ``"document-index"`` without one. When a configured
    timestamp Path could not be used (18.11), ``fallback_from`` is that Path
    and ``fallback_documents`` the number of documents that caused the
    fallback; the source is then ``"document-index"``.
    """

    source: OrderingKeySource
    path: str | None = None
    fallback_from: str | None = None
    fallback_documents: int | None = None

    def __post_init__(self) -> None:
        source = _choice(self.source, ORDERING_KEY_SOURCES, "OrderingKey source")
        _set(self, "path", _optional(_path, self.path, "OrderingKey path"))
        _set(self, "fallback_from", _optional(_path, self.fallback_from, "OrderingKey fallback_from"))
        _set(
            self,
            "fallback_documents",
            _optional(lambda v, w: _count(v, w, minimum=1), self.fallback_documents, "OrderingKey fallback_documents"),
        )
        if (source == "document-index") != (self.path is None):
            raise ValueError("an OrderingKey has a path exactly when its source is 'configured' or 'selected'")
        if (self.fallback_from is None) != (self.fallback_documents is None):
            raise ValueError("an OrderingKey fallback records both the configured Path and the document count")
        if self.fallback_from is not None and source != "document-index":
            raise ValueError("an OrderingKey fallback uses the document index (source 'document-index')")

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {"source": self.source}
        for name in ("fallback_documents", "fallback_from", "path"):
            value = getattr(self, name)
            if value is not None:
                members[name] = value
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> OrderingKey:
        obj = _Obj(value, location, ("source",), ("fallback_documents", "fallback_from", "path"))
        return _build(
            cls,
            location,
            source=obj.read_str("source"),
            path=obj.optional("path", obj.read_str),
            fallback_from=obj.optional("fallback_from", obj.read_str),
            fallback_documents=obj.optional("fallback_documents", obj.read_int),
        )


@dataclass(frozen=True, slots=True)
class ChangePointCriterion:
    """The change-point selection (18.5): criterion ``name``, selected count, and both criterion values.

    ``selected`` is the criterion value of the selected segmentation and
    ``unsegmented`` that of the segmentation with no change points.
    """

    name: str
    change_points: int
    selected: float
    unsegmented: float

    def __post_init__(self) -> None:
        _text(self.name, "ChangePointCriterion name")
        _set(self, "change_points", _count(self.change_points, "ChangePointCriterion change_points"))
        _set(self, "selected", _real(self.selected, "ChangePointCriterion selected"))
        _set(self, "unsegmented", _real(self.unsegmented, "ChangePointCriterion unsegmented"))

    def to_json(self) -> dict[str, JSON]:
        return {
            "change_points": self.change_points,
            "name": self.name,
            "selected": self.selected,
            "unsegmented": self.unsegmented,
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> ChangePointCriterion:
        obj = _Obj(value, location, ("change_points", "name", "selected", "unsegmented"))
        return _build(
            cls,
            location,
            name=obj.read_str("name"),
            change_points=obj.read_int("change_points"),
            selected=obj.read_number("selected"),
            unsegmented=obj.read_number("unsegmented"),
        )


@dataclass(frozen=True, eq=False, slots=True)
class Segment(_JsonEquality):
    """One Schema_Version's segment (18.6): document index and Ordering_Key value of its first and last document."""

    first_document: int
    first_key: JSON
    last_document: int
    last_key: JSON

    def __post_init__(self) -> None:
        _set(self, "first_document", _count(self.first_document, "Segment first_document"))
        _set(self, "last_document", _count(self.last_document, "Segment last_document"))
        _set(self, "first_key", _payload(self.first_key, "Segment first_key"))
        _set(self, "last_key", _payload(self.last_key, "Segment last_key"))

    def to_json(self) -> dict[str, JSON]:
        return {
            "first_document": self.first_document,
            "first_key": canonical_value(self.first_key),
            "last_document": self.last_document,
            "last_key": canonical_value(self.last_key),
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Segment:
        obj = _Obj(value, location, ("first_document", "first_key", "last_document", "last_key"))
        return _build(
            cls,
            location,
            first_document=obj.read_int("first_document"),
            first_key=obj.value["first_key"],
            last_document=obj.read_int("last_document"),
            last_key=obj.value["last_key"],
        )


@dataclass(frozen=True, eq=False, slots=True)
class VersionsInfo(_JsonEquality):
    """The ``versions`` section (18.3–18.6, 18.10, 18.11).

    ``applied`` is False when change-point detection was not applied (18.10);
    nothing else is recorded then. Otherwise ``ordering_key`` and
    ``criterion`` are required, and ``segments`` lists the segments in
    Ordering_Key order: one more than the accepted change points, or none
    when no change point is accepted (18.9).
    """

    applied: bool
    ordering_key: OrderingKey | None = None
    criterion: ChangePointCriterion | None = None
    segments: tuple[Segment, ...] = ()

    def __post_init__(self) -> None:
        applied = _flag(self.applied, "VersionsInfo applied")
        _optional_instance(self.ordering_key, OrderingKey, "VersionsInfo ordering_key")
        _optional_instance(self.criterion, ChangePointCriterion, "VersionsInfo criterion")
        segments = _instances(self.segments, Segment, "VersionsInfo segments")
        _set(self, "segments", segments)
        if not applied:
            if self.ordering_key is not None or self.criterion is not None or segments:
                raise ValueError("change-point detection was not applied, so VersionsInfo records nothing else")
            return
        if self.ordering_key is None or self.criterion is None:
            raise ValueError("applied change-point detection records the Ordering_Key and the criterion")
        expected = self.criterion.change_points + 1 if self.criterion.change_points else 0
        if len(segments) != expected:
            raise ValueError(
                f"{self.criterion.change_points} accepted change points give {expected} segments, got {len(segments)}"
            )

    @classmethod
    def not_applied(cls) -> VersionsInfo:
        """The record of 18.10: change-point detection was not applied."""
        return cls(False)

    def to_json(self) -> dict[str, JSON]:
        if not self.applied:
            return {"applied": False}
        assert self.ordering_key is not None and self.criterion is not None
        return {
            "applied": True,
            "criterion": self.criterion.to_json(),
            "ordering_key": self.ordering_key.to_json(),
            "segments": [segment.to_json() for segment in self.segments],
        }

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> VersionsInfo:
        obj = _Obj(value, location, ("applied",), ("criterion", "ordering_key", "segments"))
        applied = obj.read_bool("applied")
        if applied:
            for key in ("criterion", "ordering_key", "segments"):
                if not obj.has(key):
                    _fail(location, f"missing member {key!r}")
        elif len(obj.value) > 1:
            _fail(location, "change-point detection was not applied, so 'applied' is the only member")
        if not applied:
            return cls(False)
        segments = tuple(
            Segment.from_json(item, location=obj.at("segments", index))
            for index, item in enumerate(obj.read_array("segments"))
        )
        return _build(
            cls,
            location,
            applied=True,
            ordering_key=OrderingKey.from_json(obj.value["ordering_key"], location=obj.at("ordering_key")),
            criterion=ChangePointCriterion.from_json(obj.value["criterion"], location=obj.at("criterion")),
            segments=segments,
        )


# ---------------------------------------------------------------------------
# decisions and rejected candidates
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class DecisionEntry(_JsonEquality):
    """The report entry of one resolved Decision_Point (19.5, 20.7, 26.3).

    ``kind`` is the Decision_Point kind and ``path`` its Path (``"$"`` for
    run-level decisions; further Paths go in ``details``). ``resolution``
    holds the method and every candidate with its score and MDL breakdown.
    ``cutoff`` records the cut-off source and its value or the compared
    scores. ``fallback`` names the Technique whose Fallback_Behavior was
    applied, and is required for a fallback resolution. ``details`` holds
    other JSON facts, for example which Decision_Model resolved the kind (23.9).
    """

    kind: str
    path: str
    resolution: Resolution
    cutoff: Cutoff | None = None
    fallback: FallbackInfo | None = None
    details: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        _text(self.kind, "DecisionEntry kind")
        _set(self, "path", _path(self.path, "DecisionEntry path"))
        _instance(self.resolution, Resolution, "DecisionEntry resolution")
        _optional_instance(self.cutoff, Cutoff, "DecisionEntry cutoff")
        _optional_instance(self.fallback, FallbackInfo, "DecisionEntry fallback")
        _set(self, "details", _json_object(self.details, "DecisionEntry details"))
        if self.resolution.method == "fallback" and self.fallback is None:
            raise ValueError("a fallback resolution must record fallback information")

    def sort_key(self) -> tuple[Any, ...]:
        return (_path_key(self.path), self.kind, canonical_bytes(self.to_json()))

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, Any] = {"kind": self.kind, "path": self.path, "resolution": self.resolution.to_json()}
        if self.cutoff is not None:
            members["cutoff"] = self.cutoff.to_json()
        if self.fallback is not None:
            members["fallback"] = self.fallback.to_json()
        if self.details:
            members["details"] = canonical_value(dict(self.details))
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> DecisionEntry:
        obj = _Obj(value, location, ("kind", "path", "resolution"), ("cutoff", "details", "fallback"))
        return _build(
            cls,
            location,
            kind=obj.read_str("kind"),
            path=obj.read_str("path"),
            resolution=_nested(Resolution.from_json, obj.value["resolution"], obj.at("resolution")),
            cutoff=obj.optional("cutoff", lambda key: _nested(Cutoff.from_json, obj.value[key], obj.at(key))),
            fallback=obj.optional("fallback", lambda key: _nested(FallbackInfo.from_json, obj.value[key], obj.at(key))),
            details=obj.optional("details", lambda key: obj.read_object(key, non_empty=True)) or {},
        )


@dataclass(frozen=True, eq=False, slots=True)
class RejectedCandidate(_JsonEquality):
    """A rejected candidate or suggestion (see :data:`REJECTED_CATEGORIES`).

    ``path`` is the candidate's Path (further Paths go in ``details``),
    ``candidate`` describes the candidate as JSON (fields, a Format, a
    Pattern, a rule), ``technique`` is the producing Technique, and
    ``details`` holds the facts its requirement asks for (counts,
    Confidence, threshold values, description lengths, the failed condition).
    """

    category: RejectedCategory
    path: str
    candidate: JSON
    technique: str | None = None
    details: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        _choice(self.category, REJECTED_CATEGORIES, "RejectedCandidate category")
        _set(self, "path", _path(self.path, "RejectedCandidate path"))
        _set(self, "candidate", _payload(self.candidate, "RejectedCandidate candidate"))
        _set(self, "technique", _optional(_technique_id, self.technique, "RejectedCandidate technique"))
        _set(self, "details", _json_object(self.details, "RejectedCandidate details"))

    def sort_key(self) -> tuple[Any, ...]:
        return (self.category, _path_key(self.path), canonical_bytes(self.to_json()))

    def to_json(self) -> dict[str, JSON]:
        """The entry within its category group (the category is the group's key)."""
        members: dict[str, Any] = {"candidate": canonical_value(self.candidate), "path": self.path}
        if self.technique is not None:
            members["technique"] = self.technique
        if self.details:
            members["details"] = canonical_value(dict(self.details))
        return _ordered(members)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, RejectedCandidate):
            return NotImplemented
        return self.category == other.category and _JsonEquality.__eq__(self, other)

    __hash__ = None  # type: ignore[assignment]

    @classmethod
    def from_json(cls, value: Any, *, category: str, location: str = "") -> RejectedCandidate:
        obj = _Obj(value, location, ("candidate", "path"), ("details", "technique"))
        return _build(
            cls,
            location,
            category=category,
            path=obj.read_str("path"),
            candidate=obj.value["candidate"],
            technique=obj.optional("technique", obj.read_str),
            details=obj.optional("details", lambda key: obj.read_object(key, non_empty=True)) or {},
        )


# ---------------------------------------------------------------------------
# soundness
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class RemovedConstraint(_JsonEquality):
    """A Constraint removed by the soundness stage (3.6): its JSON Pointer and its removal record."""

    pointer: str
    change: RelaxationRecord

    def __post_init__(self) -> None:
        pointer = _text(self.pointer, "RemovedConstraint pointer", empty=True)
        split_pointer(pointer)  # raises ValueError for a malformed pointer
        change = _instance(self.change, RelaxationRecord, "RemovedConstraint change")
        if not change.removed:
            raise ValueError("a RemovedConstraint records a removal (after = REMOVED); relaxations go in provenance")

    def sort_key(self) -> tuple[str, bytes]:
        return (self.pointer, canonical_bytes(self.change.to_json()))

    def to_json(self) -> dict[str, JSON]:
        return {"change": self.change.to_json(), "pointer": self.pointer}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> RemovedConstraint:
        obj = _Obj(value, location, ("change", "pointer"))
        return _build(
            cls,
            location,
            pointer=obj.read_str("pointer"),
            change=_nested(RelaxationRecord.from_json, obj.value["change"], obj.at("change")),
        )


_SOUNDNESS_COUNTS: Final = ("failed_first_pass", "flagged_documents", "passes", "relaxations", "removals", "validated")


@dataclass(frozen=True, eq=False, slots=True)
class SoundnessSummary(_JsonEquality):
    """The soundness record of one schema (3.4, 3.6).

    ``version`` is the index of the Schema_Version in Ordering_Key order, or
    None for the single emitted schema. The counts are the Flagged_Documents,
    the documents validated, those that failed the first pass, the
    relaxations applied under 3.3, the Constraints removed under 3.7, and the
    validation passes; ``pass_limit_reached`` says whether the 3.7 limit was
    reached. ``removed`` holds every removed Constraint, from 3.7 and from
    removals chosen under 3.3 (stored sorted by pointer).
    """

    flagged_documents: int
    validated: int
    failed_first_pass: int
    relaxations: int
    removals: int
    passes: int
    pass_limit_reached: bool
    removed: tuple[RemovedConstraint, ...] = ()
    version: int | None = None

    def __post_init__(self) -> None:
        for name in _SOUNDNESS_COUNTS:
            _set(self, name, _count(getattr(self, name), f"SoundnessSummary {name}"))
        _flag(self.pass_limit_reached, "SoundnessSummary pass_limit_reached")
        removed = sorted(_instances(self.removed, RemovedConstraint, "SoundnessSummary removed"), key=RemovedConstraint.sort_key)
        _set(self, "removed", tuple(removed))
        _set(self, "version", _optional(_count, self.version, "SoundnessSummary version"))
        if self.failed_first_pass > self.validated:
            raise ValueError(
                f"SoundnessSummary failed_first_pass ({self.failed_first_pass}) exceeds validated ({self.validated})"
            )
        if self.validated and not self.passes:
            raise ValueError("validating documents takes at least one pass")
        if not self.failed_first_pass and (self.relaxations or self.removals or self.pass_limit_reached or removed):
            raise ValueError("without first-pass failures there are no relaxations, removals, or pass-limit hits")
        if self.removals and not self.pass_limit_reached:
            raise ValueError("Constraints are removed under 3.7 only after the pass limit is reached")
        if not self.removals <= len(removed) <= self.relaxations + self.removals:
            raise ValueError(
                f"{len(removed)} removed Constraints do not fit {self.removals} removals plus at most "
                f"{self.relaxations} relaxations that removed"
            )

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, Any] = {name: getattr(self, name) for name in _SOUNDNESS_COUNTS}
        members["pass_limit_reached"] = self.pass_limit_reached
        members["removed"] = [entry.to_json() for entry in self.removed]
        if self.version is not None:
            members["version"] = self.version
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> SoundnessSummary:
        obj = _Obj(value, location, (*_SOUNDNESS_COUNTS, "pass_limit_reached", "removed"), ("version",))
        removed = tuple(
            RemovedConstraint.from_json(item, location=obj.at("removed", index))
            for index, item in enumerate(obj.read_array("removed"))
        )
        return _build(
            cls,
            location,
            **{name: obj.read_int(name) for name in _SOUNDNESS_COUNTS},
            pass_limit_reached=obj.read_bool("pass_limit_reached"),
            removed=removed,
            version=obj.optional("version", obj.read_int),
        )


# ---------------------------------------------------------------------------
# nearest
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Neighbor:
    """A known schema similar to the inferred one: source identifier and exact Jaccard similarity in (0, 1] (25.2)."""

    source: str
    similarity: float

    def __post_init__(self) -> None:
        _text(self.source, "Neighbor source")
        similarity = _real(self.similarity, "Neighbor similarity")
        if not 0.0 < similarity <= 1.0:
            raise ValueError(f"Neighbor similarity must be in (0, 1], got {similarity!r}")
        _set(self, "similarity", similarity)

    def to_json(self) -> dict[str, JSON]:
        return {"similarity": self.similarity, "source": self.source}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Neighbor:
        obj = _Obj(value, location, ("similarity", "source"))
        return _build(cls, location, source=obj.read_str("source"), similarity=obj.read_number("similarity"))


@dataclass(frozen=True, slots=True)
class NearestResult:
    """The nearest-known-schema result for one emitted schema or Schema_Version (25.2–25.4).

    ``neighbors`` is stored ranked by similarity (descending), then source
    (ascending); the first is the Nearest_Known_Schema. No neighbors means
    no nearest known schema was found (25.4). ``shared``, ``only_inferred``,
    and ``only_known`` are the normalized Paths compared with the
    Nearest_Known_Schema (25.3), stored sorted and disjoint, and empty when
    nothing was found. ``version`` is the Schema_Version index, or None for
    the single emitted schema.
    """

    neighbors: tuple[Neighbor, ...] = ()
    shared: tuple[str, ...] = ()
    only_inferred: tuple[str, ...] = ()
    only_known: tuple[str, ...] = ()
    version: int | None = None

    def __post_init__(self) -> None:
        neighbors = sorted(
            _instances(self.neighbors, Neighbor, "NearestResult neighbors"),
            key=lambda neighbor: (-neighbor.similarity, neighbor.source),
        )
        sources = [neighbor.source for neighbor in neighbors]
        if len(set(sources)) != len(sources):
            raise ValueError("NearestResult neighbors name a source more than once")
        _set(self, "neighbors", tuple(neighbors))
        lists = {}
        for name in ("shared", "only_inferred", "only_known"):
            lists[name] = _text_set(getattr(self, name), f"NearestResult {name}")
            _set(self, name, lists[name])
        if not neighbors and any(lists.values()):
            raise ValueError("a NearestResult without a Nearest_Known_Schema lists no Paths")
        names = list(lists)
        for i, first in enumerate(names):
            for second in names[i + 1 :]:
                common = set(lists[first]) & set(lists[second])
                if common:
                    raise ValueError(f"NearestResult {first} and {second} share {sorted(common)[0]!r}")
        _set(self, "version", _optional(_count, self.version, "NearestResult version"))

    @property
    def found(self) -> bool:
        """True when a Nearest_Known_Schema was found."""
        return bool(self.neighbors)

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {
            "found": self.found,
            "neighbors": [neighbor.to_json() for neighbor in self.neighbors],
            "only_inferred": list(self.only_inferred),
            "only_known": list(self.only_known),
            "shared": list(self.shared),
        }
        if self.version is not None:
            members["version"] = self.version
        return _ordered(members)

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> NearestResult:
        obj = _Obj(value, location, ("found", "neighbors", "only_inferred", "only_known", "shared"), ("version",))
        obj.read_bool("found")  # derived; checked against the neighbors by the canonical comparison
        neighbors = tuple(
            Neighbor.from_json(item, location=obj.at("neighbors", index))
            for index, item in enumerate(obj.read_array("neighbors"))
        )
        return _build(
            cls,
            location,
            neighbors=neighbors,
            shared=tuple(obj.read_strs("shared")),
            only_inferred=tuple(obj.read_strs("only_inferred")),
            only_known=tuple(obj.read_strs("only_known")),
            version=obj.optional("version", obj.read_int),
        )


@dataclass(frozen=True, slots=True)
class NearestInfo:
    """The ``nearest`` section (25.2–25.6).

    ``status`` is ``"disabled"`` (25.5), ``"empty-index"`` (25.6), or
    ``"searched"``, which alone carries ``results``: one per emitted schema
    or Schema_Version, stored with the single schema first, then by version.
    """

    status: NearestStatus
    results: tuple[NearestResult, ...] = ()

    def __post_init__(self) -> None:
        status = _choice(self.status, NEAREST_STATUSES, "NearestInfo status")
        results = sorted(
            _instances(self.results, NearestResult, "NearestInfo results"), key=lambda r: _version_key(r.version)
        )
        if results and status != "searched":
            raise ValueError(f"a {status!r} lookup has no results")
        versions = [result.version for result in results]
        if len(set(versions)) != len(versions):
            raise ValueError("NearestInfo results hold more than one result per schema")
        _set(self, "results", tuple(results))

    def to_json(self) -> dict[str, JSON]:
        if self.status != "searched":
            return {"status": self.status}
        return {"results": [result.to_json() for result in self.results], "status": self.status}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> NearestInfo:
        obj = _Obj(value, location, ("status",), ("results",))
        status = obj.read_str("status")
        if (status == "searched") != obj.has("results"):
            _fail(location, "a 'searched' lookup, and only it, has a 'results' member")
        results = tuple(
            NearestResult.from_json(item, location=obj.at("results", index))
            for index, item in enumerate(obj.read_array("results") if obj.has("results") else ())
        )
        return _build(cls, location, status=status, results=results)


# ---------------------------------------------------------------------------
# status
# ---------------------------------------------------------------------------


def _escape_surrogates(text: str) -> str:
    return text.encode("utf-8", "backslashreplace").decode("utf-8")


@dataclass(frozen=True, slots=True)
class ErrorInfo:
    """A description of an error (27.10): its ``type`` name and ``message``."""

    type: str
    message: str

    def __post_init__(self) -> None:
        _text(self.type, "ErrorInfo type")
        _text(self.message, "ErrorInfo message", empty=True)

    @classmethod
    def from_exception(cls, error: BaseException) -> ErrorInfo:
        """Describe ``error``: its class name (module-qualified unless built in) and ``str(error)``.

        Unpaired surrogates are escaped, so describing a failure cannot fail.
        """
        if not isinstance(error, BaseException):
            raise TypeError(f"expected an exception, got {error.__class__.__name__}")
        cls_ = error.__class__
        name = cls_.__qualname__ if cls_.__module__ == "builtins" else f"{cls_.__module__}.{cls_.__qualname__}"
        try:
            message = str(error)
        except Exception:
            message = f"<unprintable {name}>"
        return cls(_escape_surrogates(name), _escape_surrogates(message))

    def to_json(self) -> dict[str, JSON]:
        return {"message": self.message, "type": self.type}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> ErrorInfo:
        obj = _Obj(value, location, ("message", "type"))
        return _build(cls, location, type=obj.read_str("type"), message=obj.read_str("message"))


@dataclass(frozen=True, slots=True)
class Status:
    """The run status (27.10): ``ok``, or ``failed`` with the failed ``stage`` and the ``error``."""

    state: StatusState = "ok"
    stage: str | None = None
    error: ErrorInfo | None = None

    def __post_init__(self) -> None:
        state = _choice(self.state, ("failed", "ok"), "Status state")
        if state == "ok":
            if self.stage is not None or self.error is not None:
                raise ValueError("an 'ok' Status has no stage or error")
        else:
            _text(self.stage, "Status stage")
            _instance(self.error, ErrorInfo, "Status error")

    @property
    def ok(self) -> bool:
        return self.state == "ok"

    def to_json(self) -> dict[str, JSON]:
        if self.state == "ok":
            return {"state": "ok"}
        assert self.error is not None
        return {"error": self.error.to_json(), "stage": self.stage, "state": self.state}

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Status:
        obj = _Obj(value, location, ("state",), ("error", "stage"))
        state = obj.read_str("state")
        if state == "failed":
            for key in ("error", "stage"):
                if not obj.has(key):
                    _fail(location, f"missing member {key!r}")
            return _build(
                cls,
                location,
                state=state,
                stage=obj.read_str("stage"),
                error=ErrorInfo.from_json(obj.value["error"], location=obj.at("error")),
            )
        if state == "ok" and len(obj.value) > 1:
            _fail(location, "an 'ok' status has only the 'state' member")
        return _build(cls, location, state=state)


# ---------------------------------------------------------------------------
# The report builder
# ---------------------------------------------------------------------------


class _StageTimer:
    """The context manager returned by :meth:`InferenceReport.stage`."""

    __slots__ = ("_report", "_start", "component")

    def __init__(self, report: InferenceReport, component: str) -> None:
        self._report = report
        self.component = component
        self._start = 0.0

    def __enter__(self) -> _StageTimer:
        self._report._begin(self.component)
        self._start = _clock()
        return self

    def __exit__(
        self, exc_type: type[BaseException] | None, exc: BaseException | None, tb: TracebackType | None
    ) -> Literal[False]:
        elapsed = _clock() - self._start
        self._report._end(self.component, elapsed, exc)
        return False  # never suppress the exception


class InferenceReport:
    """The Inference_Report builder and report (see the module docstring).

    Recording methods validate their input and raise ``TypeError`` or
    ``ValueError`` for malformed records and for conflicts (a Learned_Threshold
    read with two values, a second entry for one Flagged_Item, a section set
    twice). Read accessors return entries in canonical order.
    """

    __slots__ = (
        "_active",
        "_decision_seqs",
        "_decisions",
        "_discarded",
        "_flagged",
        "_ingest_errors",
        "_nearest",
        "_rejected",
        "_rejected_seqs",
        "_resources",
        "_seq",
        "_run",
        "_soundness",
        "_status",
        "_substitutions",
        "_thresholds",
        "_timings",
        "_versions",
    )

    def __init__(self) -> None:
        self._run: RunInfo | None = None
        self._ingest_errors: list[IngestError] = []
        self._thresholds: dict[str, ThresholdUse] = {}
        self._resources: dict[str, ResourceUse] = {}
        self._substitutions: dict[Substitution, None] = {}
        self._flagged: dict[tuple[int, str], FlaggedItem] = {}
        self._discarded: int | None = None  # set when outlier exclusion is abandoned
        self._versions: VersionsInfo | None = None
        self._decisions: list[DecisionEntry] = []
        self._rejected: list[RejectedCandidate] = []
        self._seq = 0  # one sequence over decisions and rejected candidates, in recording order
        self._decision_seqs: list[int] = []
        self._rejected_seqs: list[int] = []
        self._soundness: dict[int | None, SoundnessSummary] = {}
        self._nearest: NearestInfo | None = None
        self._timings: dict[str, list[float]] = {}
        self._active: list[str] = []
        self._status = Status()

    def __repr__(self) -> str:
        return f"<InferenceReport {self._status.state}{'' if self.ok else ' at ' + repr(self._status.stage)}>"

    # -- run ---------------------------------------------------------------

    def set_run(self, run: RunInfo) -> None:
        """Record the run metadata (27.8). Recorded once per report."""
        _instance(run, RunInfo, "run")
        if self._run is not None:
            raise ValueError("the run metadata is already recorded")
        self._run = run

    def finish_run(self, finished: str) -> None:
        """Record the run's end timestamp in the recorded run metadata (``run.finished``). Recorded once.

        The run metadata is recorded when the run starts its work, before its
        end is known; this completes it.
        """
        if self._run is None:
            raise ValueError("the run metadata is not recorded")
        if self._run.finished is not None:
            raise ValueError("the run end timestamp is already recorded")
        self._run = replace(self._run, finished=_text(finished, "RunInfo finished"))

    @property
    def run(self) -> RunInfo | None:
        return self._run

    # -- ingestion ---------------------------------------------------------

    def record_ingest_error(self, error: IngestError) -> None:
        """Record a skipped document or file (1.7), in ingestion order; it counts toward ``skipped``."""
        self._ingest_errors.append(_instance(error, IngestError, "ingest error"))

    @property
    def ingest_errors(self) -> tuple[IngestError, ...]:
        return tuple(self._ingest_errors)

    @property
    def skipped(self) -> int:
        """The number of skipped documents and files (each has an IngestError)."""
        return len(self._ingest_errors)

    # -- thresholds and resources --------------------------------------------

    def record_threshold(self, name: str, value: int | float, bundle_id: str) -> None:
        """Record a Learned_Threshold read (20.5). Reading it again with the same value and bundle is a no-op."""
        _text(name, "Learned_Threshold name")
        use = ThresholdUse(value, bundle_id)
        known = self._thresholds.setdefault(name, use)  # an equal re-read keeps the first record
        if known != use:
            raise ValueError(f"Learned_Threshold {name!r} was read as {known} and as {use}")

    @property
    def thresholds(self) -> dict[str, ThresholdUse]:
        return {name: self._thresholds[name] for name in sorted(self._thresholds, key=codepoint_key)}

    def record_resource(self, name: str, value: int | float, origin: ResourceOrigin) -> None:
        """Record a Resource_Parameter read (20.5). Reading it again with the same value and origin is a no-op."""
        _text(name, "Resource_Parameter name")
        use = ResourceUse(value, origin)
        known = self._resources.setdefault(name, use)  # an equal re-read keeps the first record
        if known != use:
            raise ValueError(f"Resource_Parameter {name!r} was read as {known} and as {use}")

    @property
    def resources(self) -> dict[str, ResourceUse]:
        return {name: self._resources[name] for name in sorted(self._resources, key=codepoint_key)}

    # -- substitutions -----------------------------------------------------

    def record_substitution(self, substitution: Substitution) -> None:
        """Record a substitution (20.6, 21.10, 22.14, 24.9). Recording an equal one again is a no-op."""
        self._substitutions[_instance(substitution, Substitution, "substitution")] = None

    @property
    def substitutions(self) -> tuple[Substitution, ...]:
        return tuple(sorted(self._substitutions, key=Substitution.sort_key))

    # -- outliers ----------------------------------------------------------

    def record_flagged_item(self, item: FlaggedItem) -> None:
        """Record the one entry of a Flagged_Item (17.5); a second entry for the same (document, Path) is an error."""
        _instance(item, FlaggedItem, "flagged item")
        if item.identity in self._flagged:
            raise ValueError(f"Flagged_Item {item.path} of document {item.document} is already recorded")
        self._flagged[item.identity] = item

    def abandon_outliers(self) -> int:
        """Discard every recorded Flagged_Item and record the abandonment of outlier exclusion (17.9).

        Returns the number of discarded Flagged_Items. Abandonment happens at
        most once and discards at least one Flagged_Item.
        """
        if self._discarded is not None:
            raise ValueError("outlier exclusion was already abandoned")
        if not self._flagged:
            raise ValueError("abandoning outlier exclusion discards at least one recorded Flagged_Item")
        discarded = len(self._flagged)
        self._flagged.clear()
        self._discarded = discarded
        return discarded

    @property
    def flagged_items(self) -> tuple[FlaggedItem, ...]:
        return tuple(self._flagged[key] for key in sorted(self._flagged, key=lambda k: (k[0], _path_key(k[1]))))

    @property
    def outliers_abandoned(self) -> bool:
        return self._discarded is not None

    @property
    def discarded_flags(self) -> int:
        """The number of Flagged_Items discarded by abandonment (0 when not abandoned)."""
        return self._discarded or 0

    # -- versions ----------------------------------------------------------

    def set_versions(self, versions: VersionsInfo) -> None:
        """Record the ``versions`` section (18.3–18.6, 18.10, 18.11). Recorded once per report."""
        _instance(versions, VersionsInfo, "versions")
        if self._versions is not None:
            raise ValueError("the versions section is already recorded")
        self._versions = versions

    @property
    def versions(self) -> VersionsInfo | None:
        return self._versions

    # -- decisions and rejected candidates ---------------------------------

    def record_decision(self, entry: DecisionEntry) -> None:
        """Record a resolved Decision_Point (19.5, 20.7)."""
        self._decisions.append(_instance(entry, DecisionEntry, "decision"))
        self._decision_seqs.append(self._seq)
        self._seq += 1

    @property
    def decisions(self) -> tuple[DecisionEntry, ...]:
        return tuple(sorted(self._decisions, key=DecisionEntry.sort_key))

    def mark(self) -> int:
        """The position in the recording order of decisions and rejected candidates (for :meth:`forget_between`)."""
        return self._seq

    def forget_between(self, start: int, end: int) -> int:
        """Drop the decisions and rejected candidates recorded between two :meth:`mark` positions.

        For the entries a subtree's analysis produced when that subtree
        leaves the model (8.1-8.3: sites re-inferred as one
        Shared_Definition from the union of their Observations), so the
        report describes the schema as emitted. Entries recorded outside
        ``[start, end)``, such as those of definitions inferred later, stay.
        Returns the number of entries dropped.
        """
        before = len(self._decisions) + len(self._rejected)
        decisions = [(seq, e) for seq, e in zip(self._decision_seqs, self._decisions) if not start <= seq < end]
        rejected = [(seq, e) for seq, e in zip(self._rejected_seqs, self._rejected) if not start <= seq < end]
        self._decision_seqs, self._decisions = [s for s, _ in decisions], [e for _, e in decisions]
        self._rejected_seqs, self._rejected = [s for s, _ in rejected], [e for _, e in rejected]
        return before - len(self._decisions) - len(self._rejected)

    def record_rejected(self, entry: RejectedCandidate) -> None:
        """Record a rejected candidate or suggestion."""
        self._rejected.append(_instance(entry, RejectedCandidate, "rejected candidate"))
        self._rejected_seqs.append(self._seq)
        self._seq += 1

    @property
    def rejected(self) -> tuple[RejectedCandidate, ...]:
        """Every rejected candidate, by category, then Path, then content."""
        return tuple(sorted(self._rejected, key=RejectedCandidate.sort_key))

    # -- soundness ---------------------------------------------------------

    def record_soundness(self, summary: SoundnessSummary) -> None:
        """Record the soundness summary of one schema (3.4, 3.6); one per version."""
        _instance(summary, SoundnessSummary, "soundness summary")
        if summary.version in self._soundness:
            which = "the single schema" if summary.version is None else f"Schema_Version {summary.version}"
            raise ValueError(f"the soundness summary of {which} is already recorded")
        self._soundness[summary.version] = summary

    @property
    def soundness(self) -> tuple[SoundnessSummary, ...]:
        return tuple(self._soundness[key] for key in sorted(self._soundness, key=_version_key))

    # -- nearest -----------------------------------------------------------

    def set_nearest(self, nearest: NearestInfo) -> None:
        """Record the ``nearest`` section (25.2–25.6). Recorded once per report."""
        _instance(nearest, NearestInfo, "nearest")
        if self._nearest is not None:
            raise ValueError("the nearest section is already recorded")
        self._nearest = nearest

    @property
    def nearest(self) -> NearestInfo | None:
        return self._nearest

    # -- timings and status ------------------------------------------------

    def stage(self, component: str) -> _StageTimer:
        """A context manager that times one stage under ``component`` (see the module docstring)."""
        return _StageTimer(self, _text(component, "stage component"))

    def _begin(self, component: str) -> None:
        if component in self._active:
            raise ValueError(f"stage {component!r} is already running")
        self._active.append(component)

    def _end(self, component: str, elapsed: float, error: BaseException | None) -> None:
        if component in self._active:
            self._active.remove(component)
        if error is None:
            self._add_timing(component, elapsed)
        else:
            self.fail(component, error)

    def _add_timing(self, component: str, seconds: Any) -> None:
        _text(component, "stage component")
        self._timings.setdefault(component, []).append(_seconds(seconds, f"runtime of stage {component!r}"))

    @property
    def current_stage(self) -> str | None:
        """The innermost running stage, or None."""
        return self._active[-1] if self._active else None

    @property
    def timings(self) -> dict[str, float]:
        """Wall-clock seconds per completed stage, in code-point order of the component names."""
        return {name: math.fsum(self._timings[name]) for name in sorted(self._timings, key=codepoint_key)}

    def fail(self, stage: str, error: BaseException | ErrorInfo) -> None:
        """Record that ``stage`` failed with ``error`` (27.10). The first failure is kept."""
        _text(stage, "failed stage")
        info = error if isinstance(error, ErrorInfo) else ErrorInfo.from_exception(error)
        if self._status.ok:
            self._status = Status("failed", stage, info)

    @property
    def status(self) -> Status:
        return self._status

    @property
    def ok(self) -> bool:
        """True while no failure is recorded."""
        return self._status.ok

    # -- JSON form ---------------------------------------------------------

    def to_json(self) -> dict[str, JSON]:
        """The canonical JSON form of ``report.json`` (see the module docstring)."""
        rejected: dict[str, list[JSON]] = {category: [] for category in REJECTED_CATEGORIES}
        for entry in self.rejected:
            rejected[entry.category].append(entry.to_json())
        members: dict[str, Any] = {
            "decisions": [entry.to_json() for entry in self.decisions],
            "ingestion": {"errors": [error.to_json() for error in self._ingest_errors], "skipped": self.skipped},
            "nearest": self._nearest.to_json() if self._nearest is not None else {},
            "outliers": {
                "abandoned": self.outliers_abandoned,
                "discarded": self.discarded_flags,
                "items": [item.to_json() for item in self.flagged_items],
            },
            "rejected": rejected,
            "resources": {name: use.to_json() for name, use in self.resources.items()},
            "run": self._run.to_json() if self._run is not None else {},
            "soundness": [summary.to_json() for summary in self.soundness],
            "status": self._status.to_json(),
            "substitutions": [substitution.to_json() for substitution in self.substitutions],
            "thresholds": {name: use.to_json() for name, use in self.thresholds.items()},
            "timings": self.timings,
            "versions": self._versions.to_json() if self._versions is not None else {},
        }
        return members

    def to_bytes(self) -> bytes:
        """The bytes of ``report.json``: the canonical encoder applied to :meth:`to_json`."""
        return encode_canonical(self.to_json())

    def comparable_json(self) -> dict[str, JSON]:
        """:meth:`to_json` without the contents that 27.12 exempts from the reproducibility comparison.

        Removes the ``timings`` section, ``run.started`` and ``run.finished``,
        and the :data:`WORKERS` setting from ``run.config`` (top level, ``run``,
        and ``resources`` tables) and from ``resources``.
        """
        data: dict[str, Any] = self.to_json()
        del data["timings"]
        run = data["run"]
        for name in _RUN_TIMESTAMPS:
            run.pop(name, None)
        config = run.get("config")
        for *tables, name in _WORKER_COUNT_KEYS:
            holder = config
            for table in tables:
                holder = holder.get(table) if isinstance(holder, dict) else None
            if isinstance(holder, dict):
                holder.pop(name, None)
        data["resources"].pop(WORKERS, None)
        return data

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> InferenceReport:
        """Read a report from its JSON form, raising :class:`ReportFormatError` for anything else."""
        obj = _Obj(value, location, SECTIONS)
        report = cls()
        for section in SECTIONS:
            _READERS[section](report, obj.value[section], obj.at(section))
        produced = report.to_json()
        for section in SECTIONS:
            where = _first_difference(produced[section], obj.value[section], obj.at(section))
            if where is not None:
                _fail(
                    where,
                    "not in canonical form: entries must be in canonical order without repeats, "
                    "and derived members must agree with the recorded data",
                )
        return report

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, InferenceReport):
            return NotImplemented
        return self is other or canonical_bytes(self.to_json()) == canonical_bytes(other.to_json())

    __hash__ = None  # type: ignore[assignment]


# ---------------------------------------------------------------------------
# Section readers (InferenceReport.from_json)
# ---------------------------------------------------------------------------


def _read_entries(value: Any, location: str) -> Iterable[tuple[Any, str]]:
    if not isinstance(value, list):
        _fail(location, f"expected an array, got {_kind(value)}")
    return ((item, join_pointer(location, index)) for index, item in enumerate(value))


def _read_run(report: InferenceReport, value: Any, location: str) -> None:
    if _mapping(value, location):
        report.set_run(RunInfo.from_json(value, location=location))


def _read_ingestion(report: InferenceReport, value: Any, location: str) -> None:
    obj = _Obj(value, location, ("errors", "skipped"))
    obj.read_int("skipped")  # derived; checked by the canonical comparison
    for item, where in _read_entries(obj.value["errors"], obj.at("errors")):
        report.record_ingest_error(IngestError.from_json(item, location=where))


def _read_thresholds(report: InferenceReport, value: Any, location: str) -> None:
    for name, item in _mapping(value, location).items():
        where = join_pointer(location, name)
        use = ThresholdUse.from_json(item, location=where)
        _build(report.record_threshold, where, name, use.value, use.bundle_id)


def _read_resources(report: InferenceReport, value: Any, location: str) -> None:
    for name, item in _mapping(value, location).items():
        where = join_pointer(location, name)
        use = ResourceUse.from_json(item, location=where)
        _build(report.record_resource, where, name, use.value, use.origin)


def _read_substitutions(report: InferenceReport, value: Any, location: str) -> None:
    for item, where in _read_entries(value, location):
        report.record_substitution(Substitution.from_json(item, location=where))


def _read_outliers(report: InferenceReport, value: Any, location: str) -> None:
    obj = _Obj(value, location, ("abandoned", "discarded", "items"))
    abandoned = obj.read_bool("abandoned")
    discarded = obj.read_int("discarded")
    if abandoned and discarded < 1:
        _fail(obj.at("discarded"), "abandoning outlier exclusion discards at least one Flagged_Item")
    if not abandoned and discarded:
        _fail(obj.at("discarded"), "Flagged_Items are discarded only when outlier exclusion is abandoned")
    if abandoned:
        report._discarded = discarded
    for item, where in _read_entries(obj.value["items"], obj.at("items")):
        _build(report.record_flagged_item, where, FlaggedItem.from_json(item, location=where))


def _read_versions(report: InferenceReport, value: Any, location: str) -> None:
    if _mapping(value, location):
        report.set_versions(VersionsInfo.from_json(value, location=location))


def _read_decisions(report: InferenceReport, value: Any, location: str) -> None:
    for item, where in _read_entries(value, location):
        report.record_decision(DecisionEntry.from_json(item, location=where))


def _read_rejected(report: InferenceReport, value: Any, location: str) -> None:
    obj = _Obj(value, location, REJECTED_CATEGORIES)
    for category in REJECTED_CATEGORIES:
        for item, where in _read_entries(obj.value[category], obj.at(category)):
            report.record_rejected(RejectedCandidate.from_json(item, category=category, location=where))


def _read_soundness(report: InferenceReport, value: Any, location: str) -> None:
    for item, where in _read_entries(value, location):
        _build(report.record_soundness, where, SoundnessSummary.from_json(item, location=where))


def _read_nearest(report: InferenceReport, value: Any, location: str) -> None:
    if _mapping(value, location):
        report.set_nearest(NearestInfo.from_json(value, location=location))


def _read_timings(report: InferenceReport, value: Any, location: str) -> None:
    for name, seconds in _mapping(value, location).items():
        where = join_pointer(location, name)
        if isinstance(seconds, bool) or not isinstance(seconds, (int, float)):
            _fail(where, f"expected a number, got {_kind(seconds)}")
        _build(report._add_timing, where, name, seconds)


def _read_status(report: InferenceReport, value: Any, location: str) -> None:
    report._status = Status.from_json(value, location=location)


_READERS: Final[Mapping[str, Callable[[InferenceReport, Any, str], None]]] = {
    "decisions": _read_decisions,
    "ingestion": _read_ingestion,
    "nearest": _read_nearest,
    "outliers": _read_outliers,
    "rejected": _read_rejected,
    "resources": _read_resources,
    "run": _read_run,
    "soundness": _read_soundness,
    "status": _read_status,
    "substitutions": _read_substitutions,
    "thresholds": _read_thresholds,
    "timings": _read_timings,
    "versions": _read_versions,
}
