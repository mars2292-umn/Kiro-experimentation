"""Resource_Parameters: setting specs with types, allowed ranges, and proposed defaults.

A Resource_Parameter is a numeric runtime knob (worker count, chunk and
sample sizes, budgets, time limits); it is never a Decision_Point cut-off,
which comes from MDL, BIC, or a Learned_Threshold (``config.thresholds``).
Every Resource_Parameter has a default (27.2): the design's proposed values,
and for ``workers`` the number of logical CPU cores that ``os.cpu_count()``
reports, or 1 when the OS does not say (29.4).

SettingSpec
    ``SettingSpec(name, type, range, default)`` describes one setting of the
    Run_Configuration. ``type`` is ``int``, ``float``, ``bool``, or ``str``;
    numeric settings carry an :class:`~jsinfer.config.intervals.Interval`
    ``range`` and the others none. A spec has either a constant ``default``
    or a ``default_factory`` evaluated by :meth:`SettingSpec.default_value`.
    The same type serves the other Run_Configuration settings (seed, input
    and output options) in ``config.run_config``.

    :meth:`SettingSpec.check` returns a :class:`SettingProblem` naming the
    setting and the accepted values when a value has the wrong type or lies
    outside the range (27.11, 29.11), and :meth:`SettingSpec.normalize`
    returns the value as the setting's type. Type rules: booleans are never
    numbers; ``int`` settings take ints only (``2.0`` is a float, so it has
    the wrong type); ``float`` settings take ints and floats and store a
    float, and NaN or an infinity is out of range.

The Resource_Parameter table
    :data:`RESOURCE_PARAMETERS` maps each name to its spec in code-point
    order. Every count, size, and budget is an integer >= 1, which covers
    the worker count and every ``sample.*`` sample size (29.11), except
    ``nearest.next`` (a non-negative count, 25.2). Time limits are numbers
    > 0 seconds, and ``ds.tol`` is a number >= 0. ``workers`` is written under
    ``[run]`` in a configuration file and the rest under ``[resources]``;
    that layout belongs to ``config.run_config``.

    One range depends on another setting: ``lsh.bands`` is at most
    ``minhash.perm``, so that each band has at least one row
    (``rows = perm // bands``). :func:`check_resources` checks a mapping of
    supplied values: unknown names, each value, then this relation over the
    effective values (supplied, else default), reported under the supplied
    setting.
"""

from __future__ import annotations

import os
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

import numpy as np

from jsinfer.config.intervals import Interval, is_number
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.report import WORKERS

__all__ = [
    "BOUNDED_BY",
    "RESOURCE_PARAMETERS",
    "SAMPLE_SIZES",
    "WORKERS",
    "SettingProblem",
    "SettingProblemReason",
    "SettingSpec",
    "SettingValue",
    "check_resources",
    "describe_value",
    "logical_cpu_count",
    "resource_defaults",
]

SettingValue: TypeAlias = int | float | bool | str
SettingProblemReason: TypeAlias = Literal["range", "type", "unknown"]

_TYPES: Final = (int, float, bool, str)
_TYPE_WORDS: Final = {bool: "a boolean", str: "a string"}


def logical_cpu_count() -> int:
    """The number of logical CPU cores reported by the OS, or 1 when it reports none (29.4)."""
    return os.cpu_count() or 1


def _got(value: Any) -> str:
    """How an offending value reads in a message."""
    if isinstance(value, (bool, np.bool_)):
        return f"the boolean {'true' if value else 'false'}"
    if isinstance(value, (int, np.integer)):
        return str(int(value))
    if is_number(value):
        return repr(float(value))  # 2.0 stays visibly a float
    if value is None:
        return "null"
    if isinstance(value, str):
        return f"the string {value!r}"
    if isinstance(value, (list, tuple)):
        return "an array"
    if isinstance(value, dict):
        return "a table"
    return f"a {type(value).__name__}"


def describe_value(value: Any) -> str:
    """How an offending value reads in a setting message: ``0``, ``2.0``, ``the string 'x'``, ``a table``."""
    return _got(value)


@dataclass(frozen=True, slots=True)
class SettingProblem:
    """An offending setting (27.11, 29.11): an unknown name, a wrong type, or a value out of range.

    ``message`` names the setting and, for a type or range problem, the
    accepted values and the offending value.
    """

    name: str
    reason: SettingProblemReason
    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(frozen=True, slots=True)
class SettingSpec:
    """One Run_Configuration setting: name, type, allowed range, and default; see the module docstring.

    Equality ignores ``default_factory`` (functions compare by identity).
    """

    name: str
    type: type
    range: Interval | None
    default: SettingValue | None = None
    description: str = ""
    default_factory: Callable[[], SettingValue] | None = field(default=None, compare=False, repr=False)

    def __post_init__(self) -> None:
        if not isinstance(self.name, str) or not self.name:
            raise ValueError(f"SettingSpec name must be a non-empty str, got {self.name!r}")
        if self.type not in _TYPES:
            raise TypeError(f"SettingSpec {self.name!r} type must be int, float, bool, or str, got {self.type!r}")
        numeric = self.type in (int, float)
        if numeric and not isinstance(self.range, Interval):
            raise TypeError(f"numeric SettingSpec {self.name!r} needs an Interval range, got {self.range!r}")
        if not numeric and self.range is not None:
            raise TypeError(f"SettingSpec {self.name!r} of type {self.type.__name__} takes no range")
        if (self.default is None) == (self.default_factory is None):
            raise ValueError(f"SettingSpec {self.name!r} needs exactly one of default and default_factory")
        if self.default_factory is not None and not callable(self.default_factory):
            raise TypeError(f"SettingSpec {self.name!r} default_factory must be callable")
        if not isinstance(self.description, str):
            raise TypeError(f"SettingSpec {self.name!r} description must be a str")
        if self.default is not None:
            problem = self.check(self.default)
            if problem is not None:
                raise ValueError(f"invalid default: {problem}")
            object.__setattr__(self, "default", self._convert(self.default))

    def accepted(self) -> str:
        """The accepted values in words, for example ``"an integer in [1, inf)"``."""
        if self.range is not None:
            return self.range.describe(integer=self.type is int)
        return _TYPE_WORDS[self.type]

    def _problem(self, reason: SettingProblemReason, value: Any) -> SettingProblem:
        return SettingProblem(self.name, reason, f"{self.name}: expected {self.accepted()}, got {_got(value)}")

    def check(self, value: Any) -> SettingProblem | None:
        """The problem with ``value`` for this setting (wrong type or out of range), or None."""
        if self.type is bool or self.type is str:
            return None if isinstance(value, self.type) else self._problem("type", value)
        if not is_number(value):
            return self._problem("type", value)
        if self.type is int and not isinstance(value, (int, np.integer)):
            return self._problem("type", value)
        if self.type is float and isinstance(value, (int, np.integer)):
            try:
                float(value)
            except OverflowError:
                return self._problem("range", value)
        assert self.range is not None
        if not self.range.contains(value):
            return self._problem("range", value)
        return None

    def _convert(self, value: Any) -> SettingValue:
        if self.type is int:
            return int(value)
        if self.type is float:
            return float(value) + 0.0  # -0.0 becomes 0.0
        return value

    def normalize(self, value: Any) -> SettingValue:
        """``value`` as the setting's type; raises ``ValueError`` with the problem message when it is invalid."""
        problem = self.check(value)
        if problem is not None:
            raise ValueError(problem.message)
        return self._convert(value)

    def default_value(self) -> SettingValue:
        """The default: the constant, or the factory's value (checked against the type and range)."""
        if self.default_factory is None:
            assert self.default is not None
            return self.default
        value = self.default_factory()
        problem = self.check(value)
        if problem is not None:
            raise ValueError(f"the default factory of {self.name!r} returned an invalid value: {problem}")
        return self._convert(value)


def _count(name: str, default: int, description: str, *, minimum: int = 1) -> SettingSpec:
    return SettingSpec(name, int, Interval(minimum), default, description)


def _seconds(name: str, default: float, description: str) -> SettingSpec:
    return SettingSpec(name, float, Interval(0, None, low_open=True), default, description)


_SPECS: Final[tuple[SettingSpec, ...]] = (
    SettingSpec(
        WORKERS,
        int,
        Interval(1),
        description="maximum concurrent joblib worker processes for per-chunk and per-Path work (29.4); "
        "default: the logical CPU count",
        default_factory=logical_cpu_count,
    ),
    _count("ingest.chunk_size", 2_000, "documents per fixed-size ingestion and flattening chunk, independent of the worker count"),
    _count("soundness.max_passes", 5, "validation passes before Constraints that still fail are removed (3.7)"),
    _count("minhash.perm", 128, "MinHash permutations for Variant candidate grouping and the Corpus_Index (5.7, 25.1)"),
    _count("lsh.bands", 32, "LSH bands, at most minhash.perm; rows per band = minhash.perm // lsh.bands (5.7, 25.1)"),
    _count("variants.max_components", 10, "maximum Bernoulli-mixture component count (5.2)"),
    _count("fpgrowth.min_support", 2, "FP-growth minimum support in objects (6.3)"),
    _count("patterns.max_alternatives", 4, "maximum sub-patterns per induced Pattern (14.1)"),
    _count("patterns.attempt_budget", 200, "candidate Patterns evaluated per Path before induction stops (14.8)"),
    _count("trees.max_depth", 4, "maximum depth of a Tagged_Union decision tree (10.2)"),
    _count("trees.max_values", 32, "most frequent values kept per decision-tree feature (10.2)"),
    _count("outliers.hash_dims", 256, "hashed presence and type feature buckets for document outlier scores (17.1)"),
    _count("iforest.n_estimators", 200, "Isolation Forest trees (17.1)"),
    # Sample sizes of superlinear Techniques (29.8, 29.11)
    _count("sample.dbscan", 5_000, "Object_Signatures sampled for DBSCAN (29.8)"),
    _count("sample.hdbscan", 5_000, "Object_Signatures sampled for HDBSCAN (29.8)"),
    _count("sample.lof", 20_000, "documents sampled to fit the Local Outlier Factor (29.8)"),
    _count("sample.fpgrowth", 50_000, "objects sampled for FP-growth (29.8)"),
    _count("sample.bmf", 20_000, "objects sampled for Boolean matrix factorization (29.8)"),
    _count("sample.trees", 50_000, "objects sampled for Tagged_Union decision trees (29.8)"),
    _count("sample.fd", 100_000, "objects sampled for functional dependency discovery (29.8)"),
    _count("sample.patterns", 20_000, "distinct string values sampled for pattern induction (29.8)"),
    _count("sample.changepoint", 20_000, "documents sampled for change-point detection (29.8)"),
    # Corpus_Advisor (25.1, 25.2)
    _count("nearest.ref_depth", 8, "Path depth up to which $ref is expanded in normalized Path sets (25.1)"),
    _count("nearest.next", 5, "next-nearest known schemas reported after the nearest one (25.2)", minimum=0),
    # Trainer and Synthetic_Generator (21.5, 22.1, 22.8, 23.3)
    _count("synth.collections_per_schema", 3, "Document_Collections generated per training- or validation-split schema (22.1)"),
    _count("synth.doc_attempts", 50, "generation attempts per document before the schema is excluded (22.8, 31.12)"),
    _count("synth.max_depth", 6, "$ref recursion depth of the Synthetic_Generator"),
    _count("ece.bins", 15, "equal-width Confidence bins for calibration error and reliability curves (21.5, 35.6)"),
    _count("ds.max_iter", 100, "Dawid-Skene EM iteration cap (23.3)"),
    SettingSpec("ds.tol", float, Interval(0), 1e-6, "Dawid-Skene EM convergence tolerance (23.3)"),
    # Evaluation_Harness (31.8, 31.13, 34.7, 35.1)
    _count("bootstrap.resamples", 2_000, "seeded bootstrap resamples per confidence interval (35.1)"),
    _seconds("subschema.timeout_s", 30.0, "per-check time limit of the Subschema_Checker in seconds (31.8)"),
    _seconds("bench.run_timeout_s", 1_800.0, "per-run time limit of a benchmarked system in seconds (31.13, 34.7)"),
)


def _registry(specs: tuple[SettingSpec, ...]) -> Mapping[str, SettingSpec]:
    table: dict[str, SettingSpec] = {}
    for spec in sorted(specs, key=lambda s: codepoint_key(s.name)):
        if spec.name in table:
            raise ValueError(f"Resource_Parameter {spec.name!r} is registered twice")
        table[spec.name] = spec
    return MappingProxyType(table)


# Every Resource_Parameter, by name in code-point order.
RESOURCE_PARAMETERS: Final[Mapping[str, SettingSpec]] = _registry(_SPECS)

# The sample-size Resource_Parameters of superlinear Techniques (29.8, 29.11).
SAMPLE_SIZES: Final[tuple[str, ...]] = tuple(name for name in RESOURCE_PARAMETERS if name.startswith("sample."))

# Ranges bounded by another setting: (setting, the setting that bounds it from above).
BOUNDED_BY: Final[tuple[tuple[str, str], ...]] = (("lsh.bands", "minhash.perm"),)


def resource_defaults() -> dict[str, SettingValue]:
    """Every Resource_Parameter with its default, in code-point order (``workers`` from the CPU count)."""
    return {name: spec.default_value() for name, spec in RESOURCE_PARAMETERS.items()}


def check_resources(values: Mapping[str, Any]) -> list[SettingProblem]:
    """Every problem with a mapping of Resource_Parameter values, in name order.

    Names not in :data:`RESOURCE_PARAMETERS` are ``"unknown"``; each known
    value is checked against its spec; then each :data:`BOUNDED_BY` relation
    is checked using the given values, or the defaults for absent names.
    Absent names are not problems: they take their defaults.
    """
    problems: list[SettingProblem] = []
    for name in sorted(values, key=codepoint_key):
        spec = RESOURCE_PARAMETERS.get(name)
        if spec is None:
            problems.append(SettingProblem(name, "unknown", f"unknown Resource_Parameter {name!r}"))
            continue
        problem = spec.check(values[name])
        if problem is not None:
            problems.append(problem)
    invalid = [problem.name for problem in problems]
    for name, bound in BOUNDED_BY:
        if (name not in values and bound not in values) or name in invalid or bound in invalid:
            continue  # two defaults are consistent; a bad value is already reported
        value = _effective(values, name)
        limit = _effective(values, bound)
        if value > limit:
            # Name the setting the user supplied; when both were supplied, the bounded one.
            offender = name if name in values else bound
            problems.append(
                SettingProblem(
                    offender,
                    "range",
                    f"{offender}: {name} ({_given(values, name)}) must not exceed {bound} ({_given(values, bound)})",
                )
            )
    return sorted(problems, key=lambda p: codepoint_key(p.name))


def _effective(values: Mapping[str, Any], name: str) -> SettingValue:
    return values[name] if name in values else RESOURCE_PARAMETERS[name].default_value()


def _given(values: Mapping[str, Any], name: str) -> str:
    value = _effective(values, name)
    return _got(value) if name in values else f"{_got(value)}, the default"
