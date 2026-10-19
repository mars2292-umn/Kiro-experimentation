"""Property 53: Metric definitions (task 27.2).

For any verdict counts, metric samples, negatives, and schemas:

- recall and the negative rejection rate are count over total, the harmonic
  mean is ``2RN / (R + N)`` and 0 when both are 0 (31.3–31.5);
- a zero denominator gives undefined, and an undefined input makes the
  harmonic mean undefined (31.14);
- aggregates skip undefined values and report their count (31.14);
- per-operator rejection rates are not applicable for operators with zero
  negatives, and otherwise the rejected share (32.8);
- Constraint precision and recall of a schema against itself are 1 for
  every kind the schema holds, undefined for the others, and the labels'
  accuracy against itself is 1 (31.9, 31.10).

**Validates: Requirements 31.3, 31.4, 31.5, 31.10, 31.14, 32.8, 33.7, 33.9**
"""

# Feature: json-schema-inference, Property 53: Metric definitions

from __future__ import annotations

import math

from hypothesis import given
from hypothesis import strategies as st

from jsinfer.bench.metrics import (
    CONSTRAINT_KINDS,
    aggregate,
    constraint_scores,
    constraints_of,
    harmonic_mean,
    judgment_accuracies,
    operator_rejection_rates,
    recall,
    rejection_rate,
)
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.learn.labels import KINDS, UNKNOWN, derive_labels
from jsinfer.learn.mutations import OPERATORS, Negative
from tests.strategies import planted_schemas, schema_models

unit = st.floats(0.0, 1.0)
metric = st.none() | unit


@given(accepted=st.integers(0, 50), extra=st.integers(0, 50), rejected=st.integers(0, 50), more=st.integers(0, 50))
def test_rates_and_the_harmonic_mean_follow_their_formulas(accepted: int, extra: int, rejected: int, more: int) -> None:
    positives, negatives = accepted + extra, rejected + more
    r, n = recall(accepted, positives), rejection_rate(rejected, negatives)
    assert r == (None if positives == 0 else accepted / positives)
    assert n == (None if negatives == 0 else rejected / negatives)
    h = harmonic_mean(r, n)
    if r is None or n is None:
        assert h is None
    elif r == 0 and n == 0:
        assert h == 0.0
    else:
        assert math.isclose(h, 2 * r * n / (r + n))
        assert min(r, n) <= h <= max(r, n) + 1e-12


@given(values=st.lists(metric, max_size=20))
def test_aggregates_skip_undefined_values_and_count_them(values: list[float | None]) -> None:
    summary = aggregate(values)
    defined = [v for v in values if v is not None]
    assert summary.count == len(defined) and summary.undefined == len(values) - len(defined)
    if defined:
        assert math.isclose(summary.mean, math.fsum(defined) / len(defined))
    else:
        assert summary.mean is None


@given(data=st.data())
def test_per_operator_rates_are_not_applicable_without_negatives(data: st.DataObject) -> None:
    operators = data.draw(st.lists(st.sampled_from(OPERATORS), max_size=12), label="operators")
    negatives = [Negative({"i": i}, op, "s", "/x") for i, op in enumerate(operators)]
    rejected = data.draw(st.lists(st.booleans(), min_size=len(negatives), max_size=len(negatives)), label="verdicts")
    rates = operator_rejection_rates(negatives, rejected)
    assert set(rates) == set(OPERATORS)
    for op in OPERATORS:
        produced = sum(1 for n in negatives if n.operator == op)
        hits = sum(1 for n, hit in zip(negatives, rejected) if n.operator == op and hit)
        rate = rates[op]
        assert rate.produced == produced and rate.rejected == hits and rate.applicable == (produced > 0)
        assert rate.rate == (None if produced == 0 else hits / produced)


@given(model=st.one_of(schema_models(max_leaves=8), planted_schemas(max_depth=2).map(lambda d: SchemaModel(SchemaNode.from_json(d)))))
def test_a_schema_scores_one_against_itself(model: SchemaModel) -> None:
    scores = constraint_scores(model, model)
    kinds_present = {c.kind for c in constraints_of(model)}
    for kind in CONSTRAINT_KINDS:
        if kind in kinds_present:
            assert scores.precision[kind] == 1.0 and scores.recall[kind] == 1.0, kind
        else:
            assert scores.precision[kind] is None and scores.recall[kind] is None, kind
    accuracy = judgment_accuracies(model, model)
    labels = derive_labels(model)
    for kind in KINDS:
        known = any(k == kind and label != UNKNOWN for (_, k), label in labels.items())
        assert accuracy[kind] == (1.0 if known else None)
