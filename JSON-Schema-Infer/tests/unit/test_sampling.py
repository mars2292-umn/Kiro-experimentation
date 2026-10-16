"""Unit tests for core/sampling.py (task 1.2)."""

from __future__ import annotations

import numpy as np
import pytest

from jsinfer.core.determinism import rng
from jsinfer.core.sampling import sample_rows


def test_sample_has_exactly_k_sorted_distinct_rows_when_n_exceeds_k() -> None:
    rows = np.arange(10, 1010, 2, dtype=np.int64)  # sorted, distinct view rows
    sample = sample_rows(rows, 37, rng("sample.test", "$"))
    assert isinstance(sample, np.ndarray)
    assert sample.dtype == rows.dtype
    assert len(sample) == 37
    assert np.all(np.diff(sample) > 0)
    assert np.isin(sample, rows).all()


def test_sample_is_deterministic_for_equal_streams_and_varies_with_seed() -> None:
    rows = np.arange(5000, dtype=np.int64)
    a = sample_rows(rows, 100, rng("sample.test", "$.a"))
    b = sample_rows(rows, 100, rng("sample.test", "$.a"))
    c = sample_rows(rows, 100, rng("sample.test", "$.a", seed=1))
    assert np.array_equal(a, b)
    assert not np.array_equal(a, c)


@pytest.mark.parametrize("n", [0, 1, 5, 20])
def test_all_rows_returned_when_n_at_most_k(n: int) -> None:
    rows = np.arange(n, dtype=np.int64)
    gen = rng("sample.test")
    before = gen.bit_generator.state
    assert np.array_equal(sample_rows(rows, 20, gen), rows)
    assert gen.bit_generator.state == before  # no randomness consumed


def test_sequence_input_returns_subsequence_list() -> None:
    values = ["delta", "alpha", "charlie", "bravo", "echo", "foxtrot"]  # not sorted
    sample = sample_rows(values, 3, rng("patterns.induction", "$.code"))
    assert isinstance(sample, list)
    assert len(sample) == len(set(sample)) == 3
    positions = [values.index(v) for v in sample]
    assert positions == sorted(positions)
    assert sample_rows(tuple(values), 10, rng("x")) == values


def test_large_sample_from_large_population() -> None:
    # Exercises numpy's tail-shuffle path (n > 10_000 and k > n // 50).
    rows = np.arange(50_000, dtype=np.int64)
    sample = sample_rows(rows, 20_000, rng("sample.lof"))
    assert len(np.unique(sample)) == 20_000
    assert np.all(np.diff(sample) > 0)


@pytest.mark.parametrize("k", [0, -1, 2.5, True, "3"])
def test_invalid_sample_size_is_rejected(k: object) -> None:
    with pytest.raises(ValueError):
        sample_rows(np.arange(10), k, rng("x"))  # type: ignore[arg-type]
