"""Unit tests for mdl/codes.py (task 10.1; Requirement 19.1)."""

from __future__ import annotations

import math
from fractions import Fraction
from typing import Any

import numpy as np
import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.core.jsonvalue import canonical_value
from jsinfer.mdl.codes import (
    BOUND,
    CAT,
    INTEGER_LIMITS,
    LN,
    LN0,
    LZ,
    NUM,
    RISSANEN_C0,
    STR,
    SUB,
    U,
    UNI,
    decimal_parts,
    utf8_length,
)
from tests.strategies import json_values

LOG2_C0 = math.log2(RISSANEN_C0)
TAG = math.log2(7)  # U's type tag over the seven JTypes


def finite_non_negative(bits: float) -> bool:
    return isinstance(bits, float) and math.isfinite(bits) and bits >= 0.0


# ---------------------------------------------------------------------------
# LN, LN0, LZ, UNI


def test_ln_known_values() -> None:
    assert LN(1) == LOG2_C0
    assert LN(2) == math.fsum([LOG2_C0, 1.0])
    assert LN(3) == math.fsum([LOG2_C0, math.log2(3), math.log2(math.log2(3))])
    assert LN(16) == math.fsum([LOG2_C0, 4.0, 2.0, 1.0])


def test_ln_big_integers_are_exact() -> None:
    assert LN(2**65536) == math.fsum([LOG2_C0, 65536.0, 16.0, 4.0, 2.0, 1.0])
    assert finite_non_negative(LN(10**5000))
    assert LN(2**1000 + 1) >= LN(2**1000)  # equal once log2 n rounds to the same float


def test_ln_is_strictly_increasing_and_satisfies_kraft() -> None:
    lengths = [LN(n) for n in range(1, 20001)]
    assert all(a < b for a, b in zip(lengths, lengths[1:]))
    assert math.fsum(2.0**-bits for bits in lengths) < 1.0


def test_ln_domain() -> None:
    for bad in (0, -1, -(2**80)):
        with pytest.raises(ValueError):
            LN(bad)
    for wrong in (1.0, True, "1", None):
        with pytest.raises(TypeError):
            LN(wrong)  # type: ignore[arg-type]
    assert LN(np.int64(5)) == LN(5)


def test_ln0_and_lz() -> None:
    assert LN0(0) == LN(1)
    assert LN0(41) == LN(42)
    assert LN0(2**100) == LN(2**100 + 1)
    with pytest.raises(ValueError):
        LN0(-1)
    assert LZ(0) == 1.0 + LN(1)
    assert LZ(7) == LZ(-7) == 1.0 + LN(8)
    assert LZ(-(2**100)) == 1.0 + LN(2**100 + 1)
    with pytest.raises(TypeError):
        LZ(False)


def test_uni() -> None:
    assert UNI(-3) == UNI(0) == UNI(1) == 0.0
    assert UNI(2) == 1.0
    assert UNI(16) == 4.0
    assert UNI(7) == math.log2(7)
    assert UNI(2**200) == 200.0
    with pytest.raises(TypeError):
        UNI(True)
    with pytest.raises(TypeError):
        UNI(2.0)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# SUB


def test_sub_edges() -> None:
    assert SUB(0, 0) == LN0(0)
    assert SUB(0, 5) == LN0(0)
    assert SUB(5, 5) == LN0(5)
    for r, p in ((3, 2), (-1, 2), (1, -1)):
        with pytest.raises(ValueError):
            SUB(r, p)


def test_sub_matches_exact_binomial() -> None:
    for p in range(40):
        for r in range(p + 1):
            expected = LN0(r) + math.log2(math.comb(p, r))
            assert SUB(r, p) == pytest.approx(expected, rel=1e-12, abs=1e-12)
            assert SUB(r, p) - LN0(r) == pytest.approx(SUB(p - r, p) - LN0(p - r), rel=1e-12, abs=1e-12)


def log2_binomial_reference(p: int, r: int) -> float:
    """log2 C(p, r) from the exact falling factorial, for small r and any p."""
    return math.fsum([*(math.log2(p - i) for i in range(r)), -math.log2(math.factorial(r))])


def test_sub_large_p() -> None:
    # math.log2(math.comb(10**6, 5 * 10**5)), which takes seconds to compute exactly.
    assert SUB(5 * 10**5, 10**6) - LN0(5 * 10**5) == pytest.approx(999989.7084672899, rel=1e-14)
    # Both sides of the switch from the direct sum to Stirling's series, for big p.
    for p in (2**17, 2**40, 10**400):
        for r in (1, 3, 2**16, 2**16 + 1):
            assert SUB(r, p) - LN0(r) == pytest.approx(log2_binomial_reference(p, r), rel=1e-12)
    # Only a length beyond the float range raises, and never returns inf.
    with pytest.raises(OverflowError):
        SUB(10**309, 2 * 10**309)


# ---------------------------------------------------------------------------
# CAT


def kt_reference_bits(counts: list[int]) -> float:
    """-log2 of the sequential KT probability, computed exactly with fractions."""
    k = len(counts)
    seen = [0] * k
    prob = Fraction(1)
    t = 0
    for i, c in enumerate(counts):
        for _ in range(c):
            prob *= Fraction(2 * seen[i] + 1, 2 * t + k)  # (seen + 1/2) / (t + k/2)
            seen[i] += 1
            t += 1
    return math.log2(prob.denominator) - math.log2(prob.numerator)


def test_cat_degenerate_cases_cost_nothing() -> None:
    assert CAT(()) == 0.0
    assert CAT([]) == 0.0
    assert CAT(np.array([], dtype=np.int64)) == 0.0
    assert CAT((0,)) == CAT((17,)) == CAT((10**12,)) == 0.0
    assert CAT((0, 0, 0)) == 0.0


def test_cat_known_values() -> None:
    assert CAT((1, 0)) == pytest.approx(1.0)
    assert CAT((1, 1)) == pytest.approx(3.0)
    assert CAT((2, 0)) == pytest.approx(math.log2(8 / 3))
    # An unused category still costs probability mass.
    assert CAT((5, 0)) > CAT((5,)) == 0.0


@pytest.mark.parametrize(
    "counts",
    [[1, 1], [3, 0], [2, 5], [0, 4, 1], [1, 1, 1, 1], [7, 0, 0, 2, 3], [10, 20, 30], [50, 1]],
)
def test_cat_matches_sequential_kt(counts: list[int]) -> None:
    assert CAT(counts) == pytest.approx(kt_reference_bits(counts), rel=1e-12)


def test_cat_numpy_and_validation() -> None:
    assert CAT(np.array([3, 1, 0])) == CAT([3, 1, 0])
    assert CAT(np.array([3, 1, 0], dtype=np.uint32)) == CAT((3, 1, 0))
    with pytest.raises(TypeError):
        CAT(np.array([1.0, 2.0]))
    with pytest.raises(ValueError):
        CAT(np.array([[1, 2]]))
    with pytest.raises(ValueError):
        CAT((3, -1))
    with pytest.raises(TypeError):
        CAT((True, 2))
    with pytest.raises(TypeError):
        CAT((1.5, 2))  # type: ignore[arg-type]


def test_cat_large_counts() -> None:
    bits = CAT((10**9, 10**9))
    # n·H(½) = 2e9 bits plus a redundancy of about ½ log2 n.
    assert 2e9 < bits < 2e9 + 64


@given(st.lists(st.integers(0, 10**12), max_size=12).flatmap(lambda cs: st.tuples(st.just(cs), st.permutations(cs))))
def test_cat_is_order_independent(pair: tuple[list[int], list[int]]) -> None:
    counts, permuted = pair
    bits = CAT(counts)
    assert finite_non_negative(bits)
    assert CAT(permuted) == bits  # bit-identical, not just approximately equal


# ---------------------------------------------------------------------------
# STR


def test_str() -> None:
    assert STR("") == LN0(0)
    assert STR("a") == LN0(1) + 8.0
    assert STR("é") == LN0(2) + 16.0
    assert STR("😀") == LN0(4) + 32.0
    assert utf8_length("\ud800") == 3  # lone surrogates are total, not an error
    assert STR("\ud800") == LN0(3) + 24.0
    for wrong in (b"a", 1, None):
        with pytest.raises(TypeError):
            STR(wrong)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# decimal_parts and NUM


@pytest.mark.parametrize(
    ("x", "parts"),
    [
        (0, (0, 0)),
        (-0.0, (0, 0)),
        (7, (7, 0)),
        (1000, (1, 3)),
        (1e3, (1, 3)),
        (-1200, (-12, 2)),
        (10**400, (1, 400)),
        (2**64, (18446744073709551616, 0)),
        (1e23, (99999999999999991611392, 0)),  # the double's exact value
        (0.5, (5, -1)),
        (-2.5, (-25, -1)),
        (0.1, (1, -1)),
        (0.1 + 0.2, (30000000000000004, -17)),
        (1.5e-05, (15, -6)),
        (5e-324, (5, -324)),
        (np.float64(0.25), (25, -2)),
    ],
)
def test_decimal_parts(x: Any, parts: tuple[int, int]) -> None:
    assert decimal_parts(x) == parts


def test_num_integers() -> None:
    assert NUM(5) == LZ(5)
    assert NUM(5.0) == NUM(5)
    assert NUM(-0.0) == NUM(0) == LZ(0)
    assert NUM(1e3) == LZ(1000)
    assert NUM(-(2**100)) == LZ(2**100)
    assert finite_non_negative(NUM(1.7976931348623157e308))


def test_num_non_integral() -> None:
    assert NUM(0.5) == LZ(5) + LZ(-1)
    assert NUM(-2.5) == LZ(-25) + LZ(-1)
    assert NUM(1e-300) == LZ(1) + LZ(-300)
    assert NUM(np.float64(0.25)) == NUM(0.25)


def test_num_rejects_non_numbers() -> None:
    for wrong in (True, False, "1", None, [1], np.int64(1)):
        with pytest.raises(TypeError):
            NUM(wrong)  # type: ignore[arg-type]
    for bad in (math.nan, math.inf, -math.inf):
        with pytest.raises(ValueError):
            NUM(bad)


# ---------------------------------------------------------------------------
# BOUND


def test_integer_limits_table() -> None:
    assert len(INTEGER_LIMITS) == 16
    expected = {0, 255, 65535, 2**32 - 1, 2**64 - 1}
    for width in (8, 16, 32, 64):
        expected |= {-(2 ** (width - 1)), 2 ** (width - 1) - 1}
    assert set(INTEGER_LIMITS) == expected


def test_bound_families() -> None:
    flag = math.log2(3)
    assert BOUND(0) == BOUND(0.0) == BOUND(-0.0) == flag
    for limit in INTEGER_LIMITS:
        if limit:
            assert BOUND(limit) == flag + 4.0
        if abs(limit) < 2**53:  # float(2**63 - 1) is 2**63, which is not a limit
            assert BOUND(float(limit)) == BOUND(limit)
    assert BOUND(1000) == flag + (LZ(1) + LZ(3))
    assert BOUND(128) == flag + (LZ(128) + LZ(0))
    assert BOUND(0.25) == flag + (LZ(25) + LZ(-2))
    assert BOUND(-1.5) == flag + (LZ(-15) + LZ(-1))


def test_bound_prefers_round_numbers_and_zero() -> None:
    assert BOUND(0) < BOUND(1) < BOUND(7)
    assert BOUND(1000) < BOUND(999)
    assert BOUND(0.5) < BOUND(0.53)
    assert BOUND(2**64 - 1) < BOUND(2**64)  # just past a limit, the decimal family applies


def test_bound_rejects_non_numbers() -> None:
    for wrong in (True, "0", None):
        with pytest.raises(TypeError):
            BOUND(wrong)  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        BOUND(math.inf)


# ---------------------------------------------------------------------------
# U


def test_u_scalars() -> None:
    assert U(None) == TAG
    assert U(True) == U(False) == TAG + 1.0
    assert U(3) == U(3.0) == math.fsum([TAG, NUM(3)])
    assert U(0.5) == math.fsum([TAG, NUM(0.5)])
    assert U("ab") == math.fsum([TAG, STR("ab")])
    assert U([]) == U({}) == U(()) == math.fsum([TAG, LN0(0)])


def test_u_containers() -> None:
    value = {"a": [1, None]}
    assert U(value) == math.fsum([TAG, LN0(1), STR("a"), TAG, LN0(2), TAG, NUM(1), TAG])
    assert U((1, "x")) == U([1, "x"])


def test_u_is_independent_of_key_order() -> None:
    a = {"a": 1, "b": [2.5, "x", {"c": None, "d": True}]}
    b = {"b": [2.5, "x", {"d": True, "c": None}], "a": 1}
    assert U(a) == U(b)


def test_u_deep_nesting_does_not_recurse() -> None:
    value: Any = []
    for _ in range(20000):
        value = [value]
    expected = 20000 * (TAG + LN0(1)) + TAG + LN0(0)
    assert U(value) == pytest.approx(expected, rel=1e-12)


def test_u_rejects_non_json() -> None:
    for wrong in ({1: 2}, {"a": {1, 2}}, b"x", object()):
        with pytest.raises(TypeError):
            U(wrong)
    with pytest.raises(ValueError):
        U([math.nan])


@given(json_values())
def test_u_is_finite_positive_and_canonical(value: Any) -> None:
    bits = U(value)
    assert finite_non_negative(bits)
    assert bits >= TAG > 0.0
    # Equal JSON values (1.0 vs 1, any key order) cost the same bits.
    assert U(canonical_value(value)) == bits


# ---------------------------------------------------------------------------
# Every code is finite and non-negative over its whole domain


@given(st.integers(min_value=1) | st.sampled_from([2**64 - 1, 2**4096, 10**700]))
def test_integer_codes_are_finite(n: int) -> None:
    for bits in (LN(n), LN0(n), LZ(n), LZ(-n), UNI(n), NUM(n), NUM(-n), BOUND(n), BOUND(-n), SUB(1, n)):
        assert finite_non_negative(bits)


@given(st.floats(allow_nan=False, allow_infinity=False))
def test_number_codes_are_finite(x: float) -> None:
    assert finite_non_negative(NUM(x))
    assert finite_non_negative(BOUND(x))
    m, e = decimal_parts(x)
    assert m == 0 or m % 10 != 0
    if not x.is_integer():
        assert float(f"{m}e{e}") == x  # the decimal reads back as the same double


@given(st.text())
def test_str_is_finite(s: str) -> None:
    bits = STR(s)
    assert finite_non_negative(bits)
    assert bits == LN0(utf8_length(s)) + 8.0 * utf8_length(s)
