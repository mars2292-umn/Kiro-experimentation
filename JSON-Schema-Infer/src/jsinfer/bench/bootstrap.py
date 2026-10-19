"""Bootstrap confidence intervals and paired tests (task 27.3; Requirements 35.1, 35.2).

:func:`percentile_ci` is the 95% percentile bootstrap interval of the mean
of a sample: ``bootstrap.resamples`` resamples with replacement, drawn from
``rng("bootstrap", purpose)`` so the interval is reproducible, and the 2.5th
and 97.5th percentiles of the resampled means. It is not computable (None)
with fewer than two values (35.1). :func:`summarize` pairs the mean, the
contributing count, and the interval of a sample of metric values, skipping
undefined ones.

:func:`paired_test` compares two systems over the datasets where both have
a value: the mean difference (first minus second), the bootstrap interval of
the difference, the two-sided p-value ``min(1, 2·min(P*(Δ ≤ 0), P*(Δ ≥ 0)))``
over the resampled mean differences, and the number of pairs; not
computable with fewer than two pairs (35.2).
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Sequence
from dataclasses import dataclass

import numpy as np

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import rng
from jsinfer.core.jsonvalue import JSON

__all__ = ["DEFAULT_RESAMPLES", "Interval", "PairedTest", "Summary", "paired_test", "percentile_ci", "summarize"]

DEFAULT_RESAMPLES: int = int(RESOURCE_PARAMETERS["bootstrap.resamples"].default)  # type: ignore[arg-type]


@dataclass(frozen=True)
class Interval:
    """A 95% percentile bootstrap interval, ``low <= high``."""

    low: float
    high: float

    def to_json(self) -> dict[str, JSON]:
        return {"low": self.low, "high": self.high}


@dataclass(frozen=True)
class Summary:
    """The mean of a sample of metric values, the contributing count, the undefined count, and the interval (35.1)."""

    mean: float | None
    count: int
    undefined: int
    ci: Interval | None

    def to_json(self) -> dict[str, JSON]:
        return {"mean": self.mean, "count": self.count, "undefined": self.undefined, "ci": None if self.ci is None else self.ci.to_json()}


@dataclass(frozen=True)
class PairedTest:
    """A paired bootstrap comparison (35.2): first minus second."""

    mean_difference: float
    ci: Interval
    p_value: float
    n_pairs: int

    def to_json(self) -> dict[str, JSON]:
        return {"mean_difference": self.mean_difference, "ci": self.ci.to_json(), "p_value": self.p_value, "n_pairs": self.n_pairs}


def _resamples(value: int | None) -> int:
    chosen = DEFAULT_RESAMPLES if value is None else value
    problem = RESOURCE_PARAMETERS["bootstrap.resamples"].check(chosen)
    if problem is not None:
        raise ValueError(f"bootstrap.resamples: {problem}")
    return int(chosen)


def _values(values: Iterable[float], what: str) -> np.ndarray:
    out = np.asarray(list(values), dtype=np.float64)
    if out.ndim != 1 or not np.all(np.isfinite(out)):
        raise ValueError(f"{what} must be finite numbers")
    return out


def _resampled_means(values: np.ndarray, resamples: int, purpose: str, seed: int) -> np.ndarray:
    stream = rng("bootstrap", purpose, seed=seed)
    indices = stream.integers(0, len(values), size=(resamples, len(values)))
    return values[indices].mean(axis=1)


def percentile_ci(values: Iterable[float], *, purpose: str = "", seed: int = 0, resamples: int | None = None) -> Interval | None:
    """The 95% percentile bootstrap interval of the mean; None with fewer than two values (35.1)."""
    sample = _values(values, "values")
    if len(sample) < 2:
        return None
    means = _resampled_means(sample, _resamples(resamples), purpose, seed)
    low, high = np.percentile(means, [2.5, 97.5])
    return Interval(float(low), float(high))


def summarize(values: Iterable[float | None], *, purpose: str = "", seed: int = 0, resamples: int | None = None) -> Summary:
    """The mean, counts, and interval of a sample of metric values, undefined ones skipped (35.1, 35.12)."""
    items = list(values)
    defined = [float(v) for v in items if v is not None]
    mean = math.fsum(defined) / len(defined) if defined else None
    return Summary(mean, len(defined), len(items) - len(defined), percentile_ci(defined, purpose=purpose, seed=seed, resamples=resamples))


def paired_test(
    first: Sequence[float], second: Sequence[float], *, purpose: str = "", seed: int = 0, resamples: int | None = None
) -> PairedTest | None:
    """The paired bootstrap test of ``first`` against ``second`` over aligned datasets; None below two pairs (35.2)."""
    a = _values(first, "first")
    b = _values(second, "second")
    if len(a) != len(b):
        raise ValueError("paired samples must have the same length")
    if len(a) < 2:
        return None
    differences = a - b
    means = _resampled_means(differences, _resamples(resamples), purpose, seed)
    low, high = np.percentile(means, [2.5, 97.5])
    at_most = float(np.mean(means <= 0.0))
    at_least = float(np.mean(means >= 0.0))
    p_value = min(1.0, 2.0 * min(at_most, at_least))
    return PairedTest(float(differences.mean()), Interval(float(low), float(high)), p_value, int(len(a)))
