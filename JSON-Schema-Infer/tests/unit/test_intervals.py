"""Unit tests for config/intervals.py (task 3.2): admissible and allowed ranges."""

from __future__ import annotations

import math

import numpy as np
import pytest
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.config.intervals import Interval, IntervalFormatError, format_number


@pytest.mark.parametrize(
    ("interval", "text"),
    [
        (Interval(0, 1), "[0, 1]"),
        (Interval(0, 1, low_open=True), "(0, 1]"),
        (Interval(0.0, 1.0, high_open=True), "[0, 1)"),
        (Interval(1), "[1, inf)"),
        (Interval(0, None, low_open=True), "(0, inf)"),
        (Interval(None, 0), "(-inf, 0]"),
        (Interval(), "(-inf, inf)"),
        (Interval(1e-6, 0.5), "[1e-06, 0.5]"),
    ],
)
def test_render(interval: Interval, text: str) -> None:
    assert interval.render() == text
    assert str(interval) == text


def test_describe_names_the_kind() -> None:
    assert Interval(1).describe(integer=True) == "an integer in [1, inf)"
    assert Interval(0, 1, low_open=True).describe(integer=False) == "a number in (0, 1]"


@pytest.mark.parametrize(
    ("interval", "inside", "outside"),
    [
        (Interval(0, 1), [0, 0.0, 0.5, 1, 1.0, np.float64(0.25), np.int64(1)], [-1e-12, 1.0000001, 2]),
        (Interval(0, 1, low_open=True), [1e-300, 1], [0, 0.0, -0.0]),
        (Interval(0, 1, high_open=True), [0], [1, 1.0]),
        (Interval(1), [1, 10**400], [0, 0.999]),
        (Interval(None, 0), [-(10**400), -0.5, 0], [1e-9]),
    ],
)
def test_contains(interval: Interval, inside: list[object], outside: list[object]) -> None:
    for value in inside:
        assert interval.contains(value), value
        assert value in interval
    for value in outside:
        assert not interval.contains(value), value


@pytest.mark.parametrize("value", [True, False, np.True_, None, "1", [1], math.nan, math.inf, -math.inf])
def test_non_numbers_and_non_finite_values_are_never_members(value: object) -> None:
    assert not Interval().contains(value)
    assert not Interval(0, 1).contains(value)


def test_bounds_are_normalized() -> None:
    interval = Interval(np.int64(2), np.float64(3.5))
    assert type(interval.low) is int and type(interval.high) is float
    assert math.copysign(1.0, Interval(-0.0, 1).low) == 1.0  # -0.0 becomes 0.0


def test_closed_ends() -> None:
    assert Interval(0, 1).closed_ends() == (0, 1)
    assert Interval(0, 1, low_open=True).closed_ends() == (1,)
    assert Interval(2).closed_ends() == (2,)
    assert Interval(3, 3).closed_ends() == (3,)
    assert Interval().closed_ends() == ()


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"low": 2, "high": 1}, ValueError),
        ({"low": 1, "high": 1, "low_open": True}, ValueError),
        ({"low": None, "low_open": True}, ValueError),
        ({"high": None, "high_open": True}, ValueError),
        ({"low": math.inf}, ValueError),
        ({"high": math.nan}, ValueError),
        ({"low": True}, TypeError),
        ({"low": "0"}, TypeError),
        ({"low": 0, "low_open": 1}, TypeError),
    ],
)
def test_invalid_intervals(kwargs: dict[str, object], error: type[Exception]) -> None:
    with pytest.raises(error):
        Interval(**kwargs)  # type: ignore[arg-type]


def test_json_form_omits_closed_flags_and_unbounded_sides() -> None:
    assert Interval(0, 1).to_json() == {"high": 1, "low": 0}
    assert Interval(0, 1, low_open=True).to_json() == {"high": 1, "low": 0, "low_open": True}
    assert Interval(2).to_json() == {"low": 2}
    assert Interval().to_json() == {}
    assert list(Interval(0, 1, True, True).to_json()) == ["high", "high_open", "low", "low_open"]


_bounds = st.one_of(st.none(), st.integers(-5, 5), st.floats(-5, 5, allow_nan=False))


@given(_bounds, _bounds, st.booleans(), st.booleans())
def test_json_round_trip(low: float | None, high: float | None, low_open: bool, high_open: bool) -> None:
    try:
        interval = Interval(low, high, low_open and low is not None, high_open and high is not None)
    except ValueError:
        return  # empty or reversed
    assert Interval.from_json(interval.to_json()) == interval
    assert Interval.from_json(interval.to_json()).to_json() == interval.to_json()


@pytest.mark.parametrize(
    ("value", "location"),
    [
        ([0, 1], "/r"),
        ({"low": 0, "width": 1}, "/r"),
        ({"low": "0"}, "/r/low"),
        ({"low": True}, "/r/low"),
        ({"high": math.inf}, "/r/high"),
        ({"low": 0, "low_open": False}, "/r/low_open"),
        ({"low_open": True}, "/r"),
        ({"low": 2, "high": 1}, "/r"),
    ],
)
def test_from_json_rejects_malformed_intervals(value: object, location: str) -> None:
    with pytest.raises(IntervalFormatError) as info:
        Interval.from_json(value, location="/r")
    assert info.value.location == location
    assert isinstance(info.value, ValueError)


def test_format_number() -> None:
    assert [format_number(x) for x in (1, 1.0, 0.5, 1e-6, 1e20, math.inf, np.int64(7))] == [
        "1", "1", "0.5", "1e-06", "1e+20", "inf", "7",
    ]
