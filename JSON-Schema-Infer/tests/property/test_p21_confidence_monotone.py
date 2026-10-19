"""Property 21: Confidence functions are monotone, exclusion clause (task 35.2). Keys, references, and FDs join in 44.3.

For any expected co-presence counts E1 < E2 at one Path, the Exclusion_Rule
Confidence satisfies c(E1) < c(E2) within [0, 1], and the expected count
itself grows with the members' presence rates.
"""

# Feature: json-schema-inference, Property 21: Confidence functions are monotone

from __future__ import annotations

from hypothesis import given
from hypothesis import strategies as st

from jsinfer.deps.exclusions import confidence_of, expected_overlap


@given(e1=st.floats(0, 1e6, allow_nan=False), e2=st.floats(0, 1e6, allow_nan=False))
def test_confidence_is_strictly_increasing_in_the_expected_count(e1: float, e2: float) -> None:
    c1, c2 = confidence_of(e1), confidence_of(e2)
    assert 0.0 <= c1 <= 1.0 and 0.0 <= c2 <= 1.0
    if e1 < e2:
        assert c1 < c2 or (c1 == c2 == 1.0)  # equal only at the floating-point ceiling
    elif e1 == e2:
        assert c1 == c2


@given(rates=st.lists(st.floats(0.01, 0.99), min_size=2, max_size=6), n=st.integers(1, 500), bump=st.floats(0.0, 0.5))
def test_expected_overlap_grows_with_presence_rates(rates: list[float], n: int, bump: float) -> None:
    base = expected_overlap(rates, n)
    assert 0.0 <= base <= n
    raised = [min(0.999, r + bump) for r in rates]
    assert expected_overlap(raised, n) >= base - 1e-9
    assert expected_overlap(rates, n + 1) >= base
