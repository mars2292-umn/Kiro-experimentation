"""Property 55: Negative correctness and mutation discipline (task 26.2).

For any supported ground-truth schema and seed:

- each mutated schema differs from the original at exactly one applicable
  site: the only schema object that changes is the one the site pointer
  points into, and the site is one the operator lists for the schema;
- there is one mutated schema per applicable site, or a seeded subset of
  ``site_limit`` of them when there are more, and operators without a site
  are recorded as skipped;
- every retained Negative_Document is rejected by the ground-truth schema
  with ``format`` asserted, is distinct within its mutated schema, and is
  labeled with its operator, the schema id, and its site pointer;
- a rerun with the same inputs gives identical mutated schemas, negatives,
  attempt counts, skips, and unproductive operators.

Inputs
    Planted schemas from ``tests.strategies.planted_schemas`` (Records,
    Maps, Tuples, Tagged_Unions, Field_Groups with ``dependentRequired``,
    recursion, Formats, Patterns, enums), seeds, and a small site limit so
    the subset rule is exercised.

**Validates: Requirements 32.2, 32.3, 32.4, 32.5, 32.9, 32.10**
"""

# Feature: json-schema-inference, Property 55: Negative correctness and mutation discipline

from __future__ import annotations

from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.io.validator import CompileError, Validator
from jsinfer.learn.mutations import OPERATORS, holder_of, negatives, sites
from tests.strategies import planted_schemas
from tests.unit.test_mutations import differs_only_at

VALIDATOR = Validator()


@given(schema=planted_schemas(max_depth=2), seed=st.integers(0, 2**31 - 1), limit=st.integers(1, 3))
def test_mutations_isolate_one_site_and_negatives_are_rejected(schema: dict[str, Any], seed: int, limit: int) -> None:
    try:
        truth = VALIDATOR.compile(schema)
    except CompileError:
        event("uncompilable ground truth")
        return
    report = negatives(schema, "planted", seed=seed, target=2, attempts=12, site_limit=limit)
    again = negatives(schema, "planted", seed=seed, target=2, attempts=12, site_limit=limit)
    assert report.mutated == again.mutated and report.attempts == again.attempts
    assert [n.to_json() for n in report.negatives] == [n.to_json() for n in again.negatives]
    assert report.skipped == again.skipped and report.unproductive == again.unproductive

    by_operator: dict[str, list[str]] = {}
    for mutated in report.mutated:
        by_operator.setdefault(mutated.operator, []).append(mutated.site)
        applicable = sites(mutated.operator, schema)
        assert mutated.site in applicable
        holder = holder_of(schema, mutated.site)
        assert mutated.site.startswith(holder) and differs_only_at(schema, mutated.schema, holder), f"{mutated.operator} at {mutated.site}"
    for operator in OPERATORS:
        applicable = sites(operator, schema)
        if not applicable:
            assert operator in report.skipped and operator not in by_operator
            continue
        assert operator not in report.skipped
        chosen = by_operator[operator]
        assert len(chosen) == min(len(applicable), limit) and len(set(chosen)) == len(chosen)
        assert set(chosen) <= set(applicable)
        if len(applicable) > limit:
            event("site subset")
    seen: dict[tuple[str, str], set[str]] = {}
    for negative in report.negatives:
        assert negative.schema_id == "planted" and negative.operator in OPERATORS
        assert negative.site in by_operator[negative.operator]
        assert not VALIDATOR.is_valid(truth, negative.document), f"{negative.operator} at {negative.site} accepted {negative.document!r}"
        key = (negative.operator, negative.site)
        rendered = repr(negative.document)
        assert rendered not in seen.setdefault(key, set())
        seen[key].add(rendered)
    event(f"operators with negatives: {len(set(by_operator) - set(report.unproductive))}")
    if report.unproductive:
        event("an unproductive operator")
