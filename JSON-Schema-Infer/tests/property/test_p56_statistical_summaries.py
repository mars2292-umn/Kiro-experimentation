"""Property 56: Statistical summaries (task 27.4).

For any sample of per-dataset values and seed: the bootstrap interval has
``low <= high``, contains the sample mean for a non-degenerate sample, and
is reproducible; it is not computable with fewer than two values or pairs;
the paired test's mean difference equals the mean of the pairwise
differences and its p-value lies in [0, 1]; and for any confidence and
correctness pairs the ECE equals the count-weighted mean of
|mean confidence − accuracy| over the non-empty equal-width bins, with
empty bins reported at count zero (21.5, 35.1, 35.2, 35.6).

**Validates: Requirements 21.5, 35.1, 35.2, 35.6**
"""

# Feature: json-schema-inference, Property 56: Statistical summaries

from __future__ import annotations

import math

from hypothesis import given
from hypothesis import strategies as st

from jsinfer.bench.bootstrap import paired_test, percentile_ci, summarize
from jsinfer.bench.metrics import reliability
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.learn.decision_models import expected_calibration_error

unit = st.floats(0.0, 1.0)


@given(values=st.lists(unit, max_size=12), seed=st.integers(0, 2**31 - 1))
def test_the_interval_is_ordered_reproducible_and_absent_below_two_values(values: list[float], seed: int) -> None:
    ci = percentile_ci(values, purpose="p56", seed=seed, resamples=200)
    if len(values) < 2:
        assert ci is None
        return
    assert ci is not None and ci.low <= ci.high
    assert min(values) - 1e-12 <= ci.low and ci.high <= max(values) + 1e-12
    assert percentile_ci(values, purpose="p56", seed=seed, resamples=200) == ci
    summary = summarize([*values, None], purpose="p56", seed=seed, resamples=200)
    assert summary.count == len(values) and summary.undefined == 1 and summary.ci == ci
    assert math.isclose(summary.mean, math.fsum(values) / len(values))


@given(pairs=st.lists(st.tuples(unit, unit), max_size=12), seed=st.integers(0, 2**31 - 1))
def test_the_paired_test_reports_the_mean_difference_and_a_probability(pairs: list[tuple[float, float]], seed: int) -> None:
    first = [a for a, _ in pairs]
    second = [b for _, b in pairs]
    test = paired_test(first, second, purpose="p56", seed=seed, resamples=200)
    if len(pairs) < 2:
        assert test is None
        return
    assert test is not None and test.n_pairs == len(pairs)
    assert math.isclose(test.mean_difference, math.fsum(a - b for a, b in pairs) / len(pairs))
    assert 0.0 <= test.p_value <= 1.0 and test.ci.low <= test.ci.high
    assert paired_test(first, second, purpose="p56", seed=seed, resamples=200) == test
    if all(a == b for a, b in pairs):
        assert test.p_value == 1.0 and test.mean_difference == 0.0


@given(pairs=st.lists(st.tuples(unit, st.booleans()), max_size=30), bins=st.integers(1, 12))
def test_ece_is_the_count_weighted_gap_over_non_empty_bins(pairs: list[tuple[float, bool]], bins: int) -> None:
    confidences = [c for c, _ in pairs]
    correct = [h for _, h in pairs]
    ece = expected_calibration_error(confidences, correct, bins)
    buckets: dict[int, list[tuple[float, bool]]] = {}
    for c, h in pairs:
        buckets.setdefault(min(int(c * bins), bins - 1), []).append((c, h))
    expected = math.fsum(len(b) * abs(math.fsum(c for c, _ in b) / len(b) - sum(1 for _, h in b if h) / len(b)) for b in buckets.values())
    expected = expected / len(pairs) if pairs else 0.0
    assert math.isclose(ece, expected, abs_tol=1e-12)


def test_reliability_bins_report_empty_bins_at_count_zero() -> None:
    truth = SchemaModel(SchemaNode.from_json({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a"]}))
    curve = reliability(truth, truth, {}, bins=4)
    assert curve.n == 0 and curve.ece is None and [b.count for b in curve.bins] == [0, 0, 0, 0]
    assert [(b.low, b.high) for b in curve.bins] == [(0.0, 0.25), (0.25, 0.5), (0.5, 0.75), (0.75, 1.0)]
