"""Property 50: Seeded uniform sampling (task 2.7).

For any row count n, sample size k, and seed, ``sample_rows`` returns exactly
min(n, k) distinct rows of the view, the same rows for the same seed, purpose,
and Path, and samples only when n > k. A Provenance_Record of a Constraint
derived from the sample records the sampled count and the total, and keeps
both through the JSON round trip.

**Validates: Requirements 26.4, 29.8**
"""

# Feature: json-schema-inference, Property 50: Seeded uniform sampling

from __future__ import annotations

import itertools
import json
from collections import Counter

import numpy as np
from hypothesis import event, given
from hypothesis import strategies as st
from scipy.stats import chisquare

from jsinfer.core.determinism import encode_canonical, rng
from jsinfer.core.paths import parse_path, render_path
from jsinfer.core.provenance import ProvenanceRecord, SampleInfo, Support
from jsinfer.core.sampling import sample_rows

# Stream purposes: super-linear Technique ids, which are also valid
# ProvenanceRecord techniques.
TECHNIQUES = st.sampled_from(
    [
        "variants.dbscan",
        "variants.hdbscan",
        "outliers.lof",
        "groups.fpgrowth_slim",
        "groups.bmf",
        "values.patterns",
        "versions.changepoints",
    ]
)
# Any non-empty purpose string keys a stream.
PURPOSES = TECHNIQUES | st.text(min_size=1, max_size=8)
# The Path half of a stream key (Technique, Path).
PATHS = st.sampled_from(["$", "$.users[*].id", "$.tags", '$["a b"]{*}', '$["é"][*]']).map(parse_path)
# Run_Configuration seeds: the default 0 and arbitrary non-negative integers.
SEEDS = st.just(0) | st.integers(0, 2**64 - 1)


@st.composite
def views(draw: st.DrawFn, min_size: int = 0) -> np.ndarray:
    """Sorted, distinct int64 row ids of a view with at least ``min_size`` rows.

    Small views come from arbitrary distinct ids. Large views (n > 10,000)
    reach numpy's tail-shuffle path of ``choice`` when k > n // 50.
    """
    if draw(st.booleans()):
        n = draw(st.integers(10_001, 25_000))
        start = draw(st.integers(0, 10**6))
        step = draw(st.integers(1, 7))
        return start + step * np.arange(n, dtype=np.int64)
    ids = draw(st.lists(st.integers(0, 10**9), unique=True, min_size=min_size, max_size=250))
    return np.array(sorted(ids), dtype=np.int64)


@st.composite
def views_and_sizes(draw: st.DrawFn, *, sampled: bool = False) -> tuple[np.ndarray, int]:
    """A view and a sample size k >= 1; with ``sampled=True``, n > k always."""
    rows = draw(views(min_size=2 if sampled else 0))
    n = len(rows)
    upper = n - 1 if sampled else 2 * n + 2
    # Small k reaches numpy's Floyd path on large views; the other branch
    # spreads k over both sides of n.
    k = draw(st.integers(1, min(upper, n // 50 + 1)) | st.integers(1, upper))
    return rows, k


@given(views_and_sizes(), PURPOSES, PATHS, SEEDS)
def test_sample_has_min_n_k_distinct_view_rows_and_samples_only_when_n_exceeds_k(
    view: tuple[np.ndarray, int], purpose: str, path: tuple, seed: int
) -> None:
    rows, k = view
    n = len(rows)
    gen = rng(purpose, path, seed=seed)
    state_before = gen.bit_generator.state

    sample = sample_rows(rows, k, gen)

    assert isinstance(sample, np.ndarray)
    assert sample.dtype == rows.dtype
    assert len(sample) == min(n, k)
    assert np.unique(sample).size == len(sample)  # distinct
    assert np.isin(sample, rows).all()  # rows of the view
    assert np.all(np.diff(sample) > 0)  # sorted, as the view is
    event("n > 10,000" if n > 10_000 else "n <= 10,000")
    if n <= k:
        event("n <= k (no sampling)")
        # No sampling: every row, and no randomness consumed.
        assert np.array_equal(sample, rows)
        assert gen.bit_generator.state == state_before
    else:
        event("n > k (sampled)")
        assert len(sample) == k


@given(views_and_sizes(), PURPOSES, PATHS, SEEDS)
def test_same_seed_purpose_and_path_give_identical_samples(
    view: tuple[np.ndarray, int], purpose: str, path: tuple, seed: int
) -> None:
    rows, k = view
    first = sample_rows(rows, k, rng(purpose, path, seed=seed))
    # A recreated stream, and an equal Path built anew, give the same rows.
    again = sample_rows(rows, k, rng(purpose, parse_path(render_path(path)), seed=seed))
    assert np.array_equal(first, again)
    # Sequence input picks the same positions as array input.
    assert sample_rows(rows.tolist(), k, rng(purpose, path, seed=seed)) == first.tolist()


@given(views_and_sizes(sampled=True), TECHNIQUES, PATHS, SEEDS, st.data())
def test_provenance_records_sampled_count_and_total(
    view: tuple[np.ndarray, int], technique: str, path: tuple, seed: int, data: st.DataObject
) -> None:
    rows, k = view
    n = len(rows)
    sample = sample_rows(rows, k, rng(technique, path, seed=seed))

    info = SampleInfo(sampled=len(sample), total=n)
    assert (info.sampled, info.total) == (k, n)
    assert info.sampled < info.total

    # Support still counts every Observation, not only the sampled ones (26.4).
    support = Support(data.draw(st.integers(0, n), label="consistent"), n)
    record = ProvenanceRecord.laplace(technique, support, sample=info)

    written = encode_canonical(record.to_json())
    loaded = ProvenanceRecord.from_json(json.loads(written))
    assert loaded == record
    assert loaded.sample == SampleInfo(k, n)
    assert loaded.support.total == loaded.sample.total == n
    assert encode_canonical(loaded.to_json()) == written


def test_every_k_subset_is_equally_likely() -> None:
    """Fixed-stream chi-square check of uniformity over all C(6, 3) = 20 subsets.

    The streams are fixed, so the outcome is deterministic: it cannot flake,
    and it fails only if the sampler favors some subsets.
    """
    rows = np.arange(6, dtype=np.int64)
    draws = 4000
    counts = Counter(
        tuple(sample_rows(rows, 3, rng("sample.uniformity", "$", i)).tolist()) for i in range(draws)
    )
    subsets = list(itertools.combinations(range(6), 3))
    assert set(counts) == set(subsets)
    observed = [counts[subset] for subset in subsets]
    assert chisquare(observed).pvalue > 1e-3
