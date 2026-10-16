"""Numeric intervals: admissible Learned_Threshold ranges and Resource_Parameter ranges.

:class:`Interval` is shared by ``config.thresholds`` (the admissible range of
a Learned_Threshold, 20.4, 20.6) and ``config.resources`` (the allowed range
of a Resource_Parameter, 27.11, 29.11).

Bounds
    ``low`` and ``high`` are finite numbers, or ``None`` for an unbounded
    side. Each finite bound is closed unless ``low_open`` / ``high_open`` is
    set. An unbounded side is open by nature, so its flag must stay
    ``False``; that keeps one spelling per interval. Intervals are non-empty:
    ``low <= high``, and a degenerate interval ``[x, x]`` must be closed.
    Bounds keep int or float as given (``1`` and ``1.0`` compare equal).

Membership
    :meth:`Interval.contains` accepts ints, floats, and numpy scalars, but not
    booleans. NaN and infinities are never members. Ints are compared exactly
    (Python compares int with float without rounding), so very large ints
    never overflow.

Rendering
    :meth:`Interval.render` writes the scikit-learn style ``[0, 1]``,
    ``(0, 1]``, or ``[1, inf)``; :meth:`Interval.describe` adds the value kind
    (``"an integer in [1, inf)"``). Error messages use these forms, so they
    name the accepted range (29.11).

JSON form
    ``{"low": 0, "high": 1}`` with ``"low_open": true`` or
    ``"high_open": true`` only when set, and an unbounded side's member
    omitted, so ``{"low": 1}`` is ``[1, inf)``. :meth:`Interval.from_json`
    raises :class:`IntervalFormatError` (a ``ValueError``) with the JSON
    Pointer of the offending member, and ``from_json(i.to_json()) == i``.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any, Final

import numpy as np

from jsinfer.core.jsonvalue import JSON
from jsinfer.core.schema_model import join_pointer

__all__ = ["Interval", "IntervalFormatError", "format_number", "is_number"]

_MEMBERS: Final = ("high", "high_open", "low", "low_open")


def is_number(value: Any) -> bool:
    """True for ints, floats, and numpy numeric scalars; False for booleans and everything else."""
    return not isinstance(value, (bool, np.bool_)) and isinstance(value, (int, float, np.integer, np.floating))


def _finite(value: Any) -> bool:
    """True for a number that is not NaN or infinite (ints are always finite)."""
    if isinstance(value, (int, np.integer)):
        return True
    return math.isfinite(value)


def format_number(value: int | float) -> str:
    """A short spelling of a finite or infinite number for messages: ``1``, ``0.5``, ``1e-06``, ``inf``.

    Integral floats of moderate size are written without a fraction, so a
    range reads ``[0, 1]`` whether its bounds are ints or floats.
    """
    if isinstance(value, (int, np.integer)) and not isinstance(value, (bool, np.bool_)):
        return str(int(value))
    x = float(value)
    if math.isfinite(x) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def _bound(value: Any, what: str) -> int | float | None:
    if value is None:
        return None
    if not is_number(value):
        raise TypeError(f"{what} must be a number or None, got {type(value).__name__}")
    if isinstance(value, (int, np.integer)):
        return int(value)
    x = float(value)
    if not math.isfinite(x):
        raise ValueError(f"{what} must be finite (use None for an unbounded side), got {x!r}")
    return x + 0.0  # -0.0 becomes 0.0


def _flag(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise TypeError(f"{what} must be a bool, got {type(value).__name__}")
    return value


@dataclass(frozen=True, slots=True)
class Interval:
    """A non-empty interval of the real line; see the module docstring."""

    low: int | float | None = None
    high: int | float | None = None
    low_open: bool = False
    high_open: bool = False

    def __post_init__(self) -> None:
        low = _bound(self.low, "Interval low")
        high = _bound(self.high, "Interval high")
        low_open = _flag(self.low_open, "Interval low_open")
        high_open = _flag(self.high_open, "Interval high_open")
        if low is None and low_open:
            raise ValueError("an unbounded low side is already open; leave low_open False")
        if high is None and high_open:
            raise ValueError("an unbounded high side is already open; leave high_open False")
        if low is not None and high is not None:
            if low > high:
                raise ValueError(f"Interval low ({format_number(low)}) must not exceed high ({format_number(high)})")
            if low == high and (low_open or high_open):
                raise ValueError(f"the interval at {format_number(low)} is empty unless both bounds are closed")
        object.__setattr__(self, "low", low)
        object.__setattr__(self, "high", high)

    # -- membership ---------------------------------------------------------

    def contains(self, value: Any) -> bool:
        """True when ``value`` is a finite number (not a boolean) inside the interval."""
        if not is_number(value) or not _finite(value):
            return False
        x = int(value) if isinstance(value, (int, np.integer)) else float(value)
        if self.low is not None and (x < self.low or (self.low_open and x == self.low)):
            return False
        if self.high is not None and (x > self.high or (self.high_open and x == self.high)):
            return False
        return True

    def __contains__(self, value: object) -> bool:
        return self.contains(value)

    def closed_ends(self) -> tuple[int | float, ...]:
        """The finite closed bounds, low first (the range ends of a threshold grid)."""
        ends: list[int | float] = []
        if self.low is not None and not self.low_open:
            ends.append(self.low)
        if self.high is not None and not self.high_open and self.high != self.low:
            ends.append(self.high)
        return tuple(ends)

    # -- rendering ----------------------------------------------------------

    def render(self) -> str:
        """``[0, 1]``, ``(0, 1]``, ``[1, inf)``, or ``(-inf, inf)``."""
        left = "(-inf" if self.low is None else ("(" if self.low_open else "[") + format_number(self.low)
        right = "inf)" if self.high is None else format_number(self.high) + (")" if self.high_open else "]")
        return f"{left}, {right}"

    def describe(self, *, integer: bool) -> str:
        """The accepted values in words: ``"an integer in [1, inf)"`` or ``"a number in (0, 1]"``."""
        return f"{'an integer' if integer else 'a number'} in {self.render()}"

    def __str__(self) -> str:
        return self.render()

    # -- JSON ---------------------------------------------------------------

    def to_json(self) -> dict[str, JSON]:
        members: dict[str, JSON] = {}
        if self.high is not None:
            members["high"] = self.high
            if self.high_open:
                members["high_open"] = True
        if self.low is not None:
            members["low"] = self.low
            if self.low_open:
                members["low_open"] = True
        return members

    @classmethod
    def from_json(cls, value: Any, *, location: str = "") -> Interval:
        if not isinstance(value, dict):
            raise IntervalFormatError(location, f"expected an object, got {type(value).__name__}")
        for key in value:
            if key not in _MEMBERS:
                raise IntervalFormatError(location, f"unknown member {key!r}; the members are {list(_MEMBERS)}")
        for key in ("high", "low"):
            if key in value and (not is_number(value[key]) or not _finite(value[key])):
                raise IntervalFormatError(join_pointer(location, key), f"expected a finite number, got {value[key]!r}")
        for key in ("high_open", "low_open"):
            if key in value and value[key] is not True:
                raise IntervalFormatError(
                    join_pointer(location, key), f"expected true (omit the member when the bound is closed), got {value[key]!r}"
                )
        try:
            return cls(
                low=value.get("low"),
                high=value.get("high"),
                low_open="low_open" in value,
                high_open="high_open" in value,
            )
        except (TypeError, ValueError) as error:
            raise IntervalFormatError(location, str(error)) from None


class IntervalFormatError(ValueError):
    """Malformed interval JSON: ``location`` is the JSON Pointer of the offending member."""

    def __init__(self, location: str, reason: str) -> None:
        super().__init__(location, reason)  # args survive pickling
        self.location = location
        self.reason = reason

    def __str__(self) -> str:
        where = repr(self.location) if self.location else "the top level"
        return f"invalid interval JSON at {where}: {self.reason}"
