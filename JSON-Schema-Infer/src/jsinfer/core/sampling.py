"""Seeded uniform sampling without replacement (Requirement 29.8).

Super-linear Techniques run on a sample of their view's Observations when
the view has more rows than the Technique's sample-size Resource_Parameter.
The sample is drawn with a stream from ``core.determinism.rng`` keyed by the
Technique and the Path, so it depends only on the seed and the data.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any, TypeVar, overload

import numpy as np

__all__ = ["sample_rows"]

T = TypeVar("T")


@overload
def sample_rows(rows: np.ndarray, k: int, rng: np.random.Generator) -> np.ndarray: ...
@overload
def sample_rows(rows: Sequence[T], k: int, rng: np.random.Generator) -> list[T]: ...


def sample_rows(rows: Any, k: int, rng: np.random.Generator) -> Any:
    """Return a uniform sample of exactly ``k`` rows when ``len(rows) > k``, else all rows.

    The sample is ``rows[sorted(rng.choice(n, size=k, replace=False))]``: every
    ``k``-subset of positions is equally likely, and the chosen rows keep their
    relative order in ``rows``. View rows are sorted and distinct, so the
    result is sorted and distinct too. When ``len(rows) <= k`` no randomness is
    consumed and every row is returned.

    ``rows`` is a numpy array (a numpy array is returned) or any other sequence
    (a new list is returned). ``k`` must be an integer >= 1, matching the
    accepted range of sample-size Resource_Parameters (Requirement 29.11).
    """
    if isinstance(k, bool) or not isinstance(k, (int, np.integer)) or k < 1:
        raise ValueError(f"sample size must be an integer >= 1, got {k!r}")
    k = int(k)
    n = len(rows)
    is_array = isinstance(rows, np.ndarray)
    if n <= k:
        return rows if is_array else list(rows)
    # shuffle=False skips an unneeded permutation; the subset stays uniform.
    picks = np.sort(rng.choice(n, size=k, replace=False, shuffle=False))
    if is_array:
        return rows[picks]
    return [rows[i] for i in picks.tolist()]
