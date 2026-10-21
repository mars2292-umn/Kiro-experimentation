"""Bound generalization: the candidate ladder and MDL bound selection (Requirement 16).

:func:`decide_bound` decides the two bounds of one set of observed
quantities at one Path: ``minimum``/``maximum`` for numbers (16.1),
``minLength``/``maxLength`` for string lengths in code points (16.3),
``minItems``/``maxItems`` for array element counts (16.4), and
``minProperties``/``maxProperties`` for Map key counts (16.5). The Value_Inferrer
and the Structure_Analyzer call it with the non-flagged quantities of a view
(``ViewStats.string_lengths``, ``array_lengths``, and ``object_sizes`` are
:class:`~jsinfer.core.stats.Histogram` values it takes directly).

Ladder (16.2)
    :func:`bound_ladder` lists one side's candidates in the documented
    order: omitting the bound; the observed extreme; zero; the extreme
    rounded outward (a minimum toward −∞, a maximum toward +∞) to one and to
    two significant digits (:func:`round_outward`: an observed maximum of 137
    gives 200 and 140); and, when every quantity is an integer, the lower
    limits (for a minimum) or upper limits (for a maximum) of the signed and
    unsigned 8-, 16-, 32-, and 64-bit integer ranges, signed first, by
    width. Candidates that an observed quantity violates are dropped, and so
    are negative candidates for lengths and counts. A value proposed twice
    keeps its first rung, so every candidate is a distinct number.

    Rounding works on the decimal ``m·10^e`` of
    :func:`~jsinfer.mdl.codes.decimal_parts` (the exact value of an integral
    number, the shortest round-trip decimal of any other) with integer
    arithmetic, so big integers and floats round exactly. A rounded value is
    an ``int`` when it is integral and otherwise the double nearest the
    rounded decimal (correctly rounded ``int`` division), which lies on the
    outward side of the extreme because rounding to the nearest double is
    monotone. Integral values are emitted as ``int`` (``3.0`` becomes ``3``),
    as the canonical encoder writes them.

Selection (16.2, 16.7)
    Each side is a ``bound`` Decision_Point resolved by the DecisionEngine
    with ``mdl_by="values.bounds"``: 16.2 defines the selection as an MDL
    comparison, so it does not depend on ``decide.mdl``. The minimum is
    decided first with the maximum omitted, then the maximum with the chosen
    minimum in place, because the uniform code with both bounds couples
    them. Every candidate carries its node (``base`` plus the bound
    keywords) and a precomputed :class:`~jsinfer.mdl.scorer.Score`. DL ties go
    to the candidate nearest the observed extreme (the registry's tie rule,
    ``tie_ranks["nearest-observed-extreme"]`` holds the rank of the exact
    distance), and omission, which has no rank, is the farthest. While
    ``values.bounds`` is disabled no candidate is offered and the
    Fallback_Behavior omits the bound (16.8); the engine records it.

Scores (19.1, 19.3)
    :func:`mdl_bound_score` scores a candidate node on the quantities with
    the MDL_Scorer's encoding: model bits are ``model_bits(node)`` (each
    bound costs a keyword slot and ``BOUND(v)``), and data bits are the code
    of the bounded quantities exactly as the MDL_Scorer writes it for the
    rows of the view:

    - numbers coded as integers (the node's ``type`` is absent or lists
      ``integer``, and the number is integral): ``UNI(hi − lo + 1)`` with
      both bounds, ``LN0(v − lo)`` or ``LN0(hi − v)`` with one, ``LZ(v)``
      with none, where ``lo``/``hi`` are the bounds rounded inward;
    - other numbers, ``m·10^−d`` with d decimal places: ``LN0(d)`` plus the
      same integer code of m on the grid of multiples of ``10^−d`` that the
      bounds allow (bounds read as decimals);
    - lengths and counts: ``UNI(max − min + 1)`` with a maximum (a missing
      minimum is 0), ``LN0(n − min)`` with only a minimum, ``LN0(n)`` with
      neither.

    Each distinct quantity's code is multiplied by its count and the terms
    are summed once with ``math.fsum``, so a Score depends only on the
    multiset of quantities. The parts of the view's description length that
    no bound changes (the exception flag, the type choice, characters, and
    descendants) are the same for every candidate and are left out, so
    ``data_bits`` is the code of the quantities alone. Candidates never
    reject a quantity, so ``exception_bits`` and ``n_rejected`` are 0 and
    ``n_scored`` is the number of quantities. ``decide_bound`` takes any
    scorer with the same signature, so tests can force ties.

Document_Support (16.9)
    A caller that knows which document each quantity came from passes a
    :class:`~jsinfer.decide.support.QuantitySupport`, and every rung that
    does not survive leave-one-document-out validation is offered withheld
    (``Candidate.withheld``), so the engine resolves the side among the
    rest and records it. Omission closes nothing and always survives. With
    one document every other rung is withheld. Otherwise only the one
    document that alone attains the observed extreme can change a rung by
    its absence: the rung's value is recomputed from the extreme over the
    other documents (:func:`round_outward` for the rounded rungs, the
    constants unchanged), and the rung survives iff that value still covers
    the observed extreme. When two or more documents attain the extreme,
    every rung survives. The design's ``minLength: 6`` from two strings and
    ``maxLength: 20`` from one are what this removes.

Skips (16.1, 16.3)
    No Decision_Point exists, and no decision is recorded, for an empty set
    of quantities, for numbers or strings at a Closed_Enum Path (the enum
    fixes every value), or for strings with a selected Format.
    :attr:`BoundDecision.skipped` says why.

Soundness (16.6)
    Every emitted bound is satisfied by every quantity it was decided on,
    because the ladder holds only such candidates. Its Provenance_Record
    (:meth:`BoundDecision.provenance`) therefore has Support
    ``(n, n)`` and the decision's MDL posterior as Confidence.

Determinism
    Quantities are merged under JSON Schema equality (``1`` equals ``1.0``)
    and sorted by exact value, distances are exact ``Fraction`` values, and
    all codes are summed with ``math.fsum``, so the decision does not depend
    on the order or the multiplicity layout of the input.
"""

from __future__ import annotations

import json
import math
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from fractions import Fraction
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

import numpy as np

from jsinfer.config.registry import TIE_NEAREST_EXTREME
from jsinfer.core.determinism import RANK_INTEGER, RANK_NUMBER, json_type_rank
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import Path
from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.stats import Histogram
from jsinfer.decide.engine import Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.decide.support import QuantitySupport, withheld
from jsinfer.mdl.codes import LN0, LZ, UNI, decimal_parts
from jsinfer.mdl.scorer import Score, model_bits

__all__ = [
    "BOUNDS",
    "BOUND_DECISION_POINT",
    "BOUND_KEYWORDS",
    "INTEGER_LIMIT_RUNGS",
    "ITEM_COUNT",
    "KEY_COUNT",
    "MAXIMUM",
    "MINIMUM",
    "NUMBER",
    "QUANTITY_KINDS",
    "RUNG_OBSERVED",
    "RUNG_OMIT",
    "RUNG_ONE_DIGIT",
    "RUNG_TWO_DIGITS",
    "RUNG_ZERO",
    "STRING_LENGTH",
    "BoundDecision",
    "BoundScorer",
    "BoundSide",
    "Number",
    "QuantityKind",
    "QuantitySet",
    "Rung",
    "Side",
    "SkipReason",
    "bound_ladder",
    "decide_bound",
    "mdl_bound_score",
    "round_outward",
]

Number: TypeAlias = int | float
QuantityKind: TypeAlias = Literal["number", "string_length", "item_count", "key_count"]
Side: TypeAlias = Literal["minimum", "maximum"]
SkipReason: TypeAlias = Literal["no-quantities", "closed-enum", "format"]

# The Technique and the Decision_Point kind it serves.
BOUNDS: Final = "values.bounds"
BOUND_DECISION_POINT: Final = "bound"

NUMBER: Final = "number"
STRING_LENGTH: Final = "string_length"
ITEM_COUNT: Final = "item_count"
KEY_COUNT: Final = "key_count"
QUANTITY_KINDS: Final[tuple[QuantityKind, ...]] = (NUMBER, STRING_LENGTH, ITEM_COUNT, KEY_COUNT)

MINIMUM: Final = "minimum"
MAXIMUM: Final = "maximum"
_SIDES: Final[tuple[Side, ...]] = (MINIMUM, MAXIMUM)

# The (lower, upper) bound keywords of each kind of quantity.
BOUND_KEYWORDS: Final[Mapping[QuantityKind, tuple[str, str]]] = MappingProxyType(
    {
        NUMBER: ("minimum", "maximum"),
        STRING_LENGTH: ("minLength", "maxLength"),
        ITEM_COUNT: ("minItems", "maxItems"),
        KEY_COUNT: ("minProperties", "maxProperties"),
    }
)

# The JSON type whose rows carry each kind of quantity: the default ``base``
# of the candidate nodes (numbers use "integer" when every number is integral).
_KIND_TYPES: Final[Mapping[QuantityKind, str]] = MappingProxyType(
    {NUMBER: "number", STRING_LENGTH: "string", ITEM_COUNT: "array", KEY_COUNT: "object"}
)

# Rung names, in ladder order (16.2).
RUNG_OMIT: Final = "omit"
RUNG_OBSERVED: Final = "observed"
RUNG_ZERO: Final = "zero"
RUNG_ONE_DIGIT: Final = "1-digit"
RUNG_TWO_DIGITS: Final = "2-digits"

# The lower and upper limits of the signed and unsigned 8/16/32/64-bit integer
# ranges, signed first, by width. These are the 16 values that
# ``codes.BOUND`` codes as ``UNI(16)`` (``codes.INTEGER_LIMITS``).
_WIDTHS: Final = (8, 16, 32, 64)
INTEGER_LIMIT_RUNGS: Final[Mapping[Side, tuple[tuple[str, int], ...]]] = MappingProxyType(
    {
        MINIMUM: (
            *((f"int{width}", -(2 ** (width - 1))) for width in _WIDTHS),
            *((f"uint{width}", 0) for width in _WIDTHS),
        ),
        MAXIMUM: (
            *((f"int{width}", 2 ** (width - 1) - 1) for width in _WIDTHS),
            *((f"uint{width}", 2**width - 1) for width in _WIDTHS),
        ),
    }
)

_OMIT_LABEL: Final = "omit"


def _choice(value: Any, choices: tuple[str, ...], what: str) -> Any:
    if not isinstance(value, str) or value not in choices:
        raise ValueError(f"{what} must be one of {list(choices)}, got {value!r}")
    return value


def _flag(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise TypeError(f"{what} must be a bool, got {type(value).__name__}")
    return value


# ---------------------------------------------------------------------------
# Quantities
# ---------------------------------------------------------------------------


def _scalar(value: Any) -> Any:
    """A numpy scalar as the Python value it holds; anything else unchanged."""
    return value.item() if isinstance(value, np.generic) else value


def _items(values: Any, what: str) -> list[Any]:
    if isinstance(values, np.ndarray):
        if values.ndim != 1:
            raise ValueError(f"{what} must be one-dimensional, got shape {values.shape}")
        return values.tolist()
    if isinstance(values, (str, bytes)) or not isinstance(values, Iterable):
        raise TypeError(f"{what} must be a sequence, got {type(values).__name__}")
    return [_scalar(value) for value in values]


def _number(value: Any) -> Number:
    """A JSON number as its canonical representative: integral numbers as ``int``."""
    rank = json_type_rank(value)  # TypeError for non-JSON values, ValueError for NaN and infinities
    if rank == RANK_INTEGER:
        return int(value)
    if rank == RANK_NUMBER:
        return float(value)
    raise TypeError(f"a bounded number must be a JSON number, got {type(value).__name__}")


def _length(value: Any) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"lengths and counts must be integers, got {type(value).__name__}")
    if value < 0:
        raise ValueError(f"lengths and counts must be non-negative, got {value}")
    return value


def _count(value: Any) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"quantity counts must be integers, got {type(value).__name__}")
    if value < 0:
        raise ValueError(f"quantity counts must be non-negative, got {value}")
    return value


@dataclass(frozen=True, eq=False, slots=True)
class QuantitySet:
    """The distinct observed quantities of one kind with their counts; build it with :meth:`of`.

    ``values`` are distinct under JSON Schema equality, in ascending order,
    with integral numbers as ``int``; ``counts`` are their positive counts
    and ``total`` their sum. ``integers`` says whether every quantity is an
    integer (always true for lengths and counts).
    """

    kind: QuantityKind
    values: tuple[Number, ...]
    counts: tuple[int, ...]
    total: int
    integers: bool

    @classmethod
    def of(
        cls,
        kind: QuantityKind,
        quantities: Histogram | Sequence[Number] | np.ndarray,
        counts: Sequence[int] | np.ndarray | None = None,
    ) -> QuantitySet:
        """Normalize ``quantities`` (with optional aligned ``counts``, each 1 by default).

        A :class:`~jsinfer.core.stats.Histogram` brings its own counts.
        Numbers must be JSON numbers (``bool`` is not a number); lengths and
        counts non-negative integers. Zero counts are dropped.
        """
        kind = _choice(kind, QUANTITY_KINDS, "quantity kind")
        if isinstance(quantities, Histogram):
            if counts is not None:
                raise ValueError("a Histogram carries its own counts; pass no counts with it")
            items, counts = quantities.values.tolist(), quantities.counts.tolist()
        else:
            items = _items(quantities, "quantities")
        if counts is None:
            weights = [1] * len(items)
        else:
            weights = [_count(c) for c in _items(counts, "counts")]
            if len(weights) != len(items):
                raise ValueError(f"counts has {len(weights)} entries for {len(items)} quantities")
        convert = _number if kind == NUMBER else _length
        merged: dict[Number, int] = {}  # int and float keys hash alike when equal, as JSON equality requires
        for item, weight in zip(items, weights):
            value = convert(item)
            if weight:
                merged[value] = merged.get(value, 0) + weight
        values = tuple(sorted(merged))  # Python compares ints and floats exactly
        return cls(
            kind=kind,
            values=values,
            counts=tuple(merged[value] for value in values),
            total=sum(merged.values()),
            integers=all(isinstance(value, int) for value in values),
        )

    @property
    def low(self) -> Number:
        """The smallest quantity."""
        if not self.values:
            raise ValueError("no quantities")
        return self.values[0]

    @property
    def high(self) -> Number:
        """The largest quantity."""
        if not self.values:
            raise ValueError("no quantities")
        return self.values[-1]

    def extreme(self, side: Side) -> Number:
        """The observed extreme of a side: the smallest quantity for a minimum, the largest for a maximum."""
        return self.low if _choice(side, _SIDES, "side") == MINIMUM else self.high


# ---------------------------------------------------------------------------
# The candidate ladder (16.2)
# ---------------------------------------------------------------------------


def _digit_count(n: int) -> int:
    """The number of decimal digits of a positive integer of any size."""
    digits = int(math.log10(n)) + 1  # math.log10 takes ints of any size; the estimate may be off by one
    if 10 ** (digits - 1) > n:
        digits -= 1
    elif 10**digits <= n:
        digits += 1
    return digits


def _from_decimal(m: int, e: int) -> Number:
    """The number ``m·10^e``: an ``int`` when integral, else the nearest double."""
    if m == 0:
        return 0
    while m % 10 == 0:
        m //= 10
        e += 1
    if e >= 0:
        return m * 10**e
    return m / 10**-e  # int true division is correctly rounded


def round_outward(value: Number, digits: int, side: Side) -> Number:
    """``value`` rounded outward to ``digits`` significant digits: down for a minimum, up for a maximum.

    The rounding is exact on the decimal of ``codes.decimal_parts``. A value
    with at most ``digits`` significant digits is returned unchanged (as an
    ``int`` when integral): ``round_outward(137, 1, "maximum") == 200``,
    ``round_outward(137, 2, "maximum") == 140``, and
    ``round_outward(-137, 1, "minimum") == -200``.
    """
    side = _choice(side, _SIDES, "side")
    if isinstance(digits, bool) or not isinstance(digits, int) or digits < 1:
        raise ValueError(f"digits must be a positive integer, got {digits!r}")
    m, e = decimal_parts(_number(value))
    if m == 0:
        return 0
    excess = _digit_count(abs(m)) - digits
    if excess > 0:
        scale = 10**excess
        m = m // scale if side == MINIMUM else -(-m // scale)  # floor toward −∞, ceiling toward +∞
        e += excess
    return _from_decimal(m, e)


@dataclass(frozen=True, slots=True)
class Rung:
    """One candidate of a side's ladder: the rung ``name`` and the bound ``value`` (None for omission)."""

    name: str
    value: Number | None


def bound_ladder(quantities: QuantitySet, side: Side) -> tuple[Rung, ...]:
    """The candidates of 16.2 for one side, in the documented order (see the module docstring).

    Omission comes first. Candidates that a quantity violates, negative
    candidates for lengths and counts, and repeated values are dropped.
    """
    if not isinstance(quantities, QuantitySet):
        raise TypeError(f"quantities must be a QuantitySet, got {type(quantities).__name__}")
    extreme = quantities.extreme(side)
    proposed: list[tuple[str, Number]] = [
        (RUNG_OBSERVED, extreme),
        (RUNG_ZERO, 0),
        (RUNG_ONE_DIGIT, round_outward(extreme, 1, side)),
        (RUNG_TWO_DIGITS, round_outward(extreme, 2, side)),
    ]
    if quantities.integers:
        proposed.extend(INTEGER_LIMIT_RUNGS[side])
    ladder = [Rung(RUNG_OMIT, None)]
    seen: dict[Fraction, None] = {}
    for name, value in proposed:
        if (value > extreme) if side == MINIMUM else (value < extreme):
            continue  # an observed quantity violates it
        if quantities.kind != NUMBER and value < 0:
            continue
        exact = Fraction(value)
        if exact in seen:
            continue
        seen[exact] = None
        ladder.append(Rung(name, value))
    return tuple(ladder)


def _distance_ranks(ladder: Sequence[Rung], extreme: Number) -> list[float]:
    """Each candidate's rank by exact distance from the extreme (0 is nearest); omission ranks last (inf)."""
    target = Fraction(extreme)
    distances = [None if rung.value is None else abs(Fraction(rung.value) - target) for rung in ladder]
    order = {d: rank for rank, d in enumerate(sorted({d: None for d in distances if d is not None}))}
    return [math.inf if d is None else float(order[d]) for d in distances]


# ---------------------------------------------------------------------------
# Scores (19.1, 19.3)
# ---------------------------------------------------------------------------

# (quantities, candidate node, lower bound, upper bound) -> Score; None is an omitted bound.
BoundScorer: TypeAlias = Callable[[QuantitySet, SchemaNode, Number | None, Number | None], Score]


def _decimal(x: Number) -> Fraction:
    """A bound as the decimal the MDL_Scorer reads it as (``codes.decimal_parts``)."""
    m, e = decimal_parts(x)
    return Fraction(m * 10**e) if e >= 0 else Fraction(m, 10**-e)


def _grid(low: Number | None, high: Number | None, d: int) -> tuple[int | None, int | None]:
    """The lowest and highest multiples of ``10^−d`` (as integers) that the bounds allow."""
    scale = 10**d
    lo = None if low is None else math.ceil(_decimal(low) * scale)
    hi = None if high is None else math.floor(_decimal(high) * scale)
    return lo, hi


def _grid_bits(m: int, low: int | None, high: int | None) -> float:
    """The integer code of m in [low, high]: uniform, one-sided LN0, or LZ."""
    if low is not None and high is not None:
        return UNI(high - low + 1)
    if low is not None:
        return LN0(m - low)
    if high is not None:
        return LN0(high - m)
    return LZ(m)


def _integer_code(node: SchemaNode) -> bool:
    """Whether the MDL_Scorer codes integral numbers as integers under ``node``: no ``type``, or ``integer`` listed."""
    types = node.kw.get("type")
    if types is None:
        return True
    return types == "integer" if isinstance(types, str) else "integer" in types


def _number_terms(q: QuantitySet, node: SchemaNode, low: Number | None, high: Number | None) -> list[float]:
    integer_code = _integer_code(node)
    grids: dict[int, tuple[int | None, int | None]] = {}
    terms: list[float] = []
    for value, count in zip(q.values, q.counts):
        as_integer = integer_code and isinstance(value, int)
        if as_integer:
            m, d = value, 0
        else:
            m, e = decimal_parts(value)
            d, m = (-e, m) if e < 0 else (0, m * 10**e)
        grid = grids.get(d)
        if grid is None:
            grid = grids[d] = _grid(low, high, d)
        # Integers: the integer code; other numbers: the decimal places, then m on the grid of 10^-d.
        bits = _grid_bits(m, *grid) if as_integer else LN0(d) + _grid_bits(m, *grid)
        if bits:
            terms.append(bits * count)
    return terms


def _count_terms(q: QuantitySet, low: Number | None, high: Number | None) -> list[float]:
    terms: list[float] = []
    for n, count in zip(q.values, q.counts):
        assert isinstance(n, int)
        if high is not None:
            bits = UNI(int(high) - int(low or 0) + 1)  # a missing minimum of a count is 0
        elif low is not None:
            bits = LN0(n - int(low))
        else:
            bits = LN0(n)
        if bits:
            terms.append(bits * count)
    return terms


def mdl_bound_score(quantities: QuantitySet, node: SchemaNode, low: Number | None, high: Number | None) -> Score:
    """The MDL Score of ``node`` on ``quantities`` under the bounds ``low`` and ``high`` (see the module docstring).

    ``node`` is the candidate's schema (its model bits); ``low`` and
    ``high`` are its lower and upper bound, None when omitted.
    """
    if not isinstance(quantities, QuantitySet):
        raise TypeError(f"quantities must be a QuantitySet, got {type(quantities).__name__}")
    if quantities.kind == NUMBER:
        terms = _number_terms(quantities, node, low, high)
    else:
        terms = _count_terms(quantities, low, high)
    return Score(model_bits(node), math.fsum(terms), 0.0, 0, quantities.total)


# ---------------------------------------------------------------------------
# Decisions (16.1–16.8)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class BoundSide:
    """The decision for one bound: its ``keyword``, the emitted ``value`` (None when omitted), and the engine's decision.

    ``extreme`` is the observed extreme the ladder was built from.
    """

    side: Side
    keyword: str
    value: Number | None
    extreme: Number
    decision: Decision

    def provenance(self, support: Support) -> ProvenanceRecord:
        """The Provenance_Record of the emitted bound, with the observed extreme in its details."""
        if self.value is None:
            raise ValueError(f"{self.keyword} was omitted, so it has no Provenance_Record")
        return self.decision.provenance(support, details={"observed_extreme": self.extreme})


@dataclass(frozen=True, eq=False, slots=True)
class BoundDecision:
    """Both bounds of one set of quantities (see :func:`decide_bound`).

    ``minimum`` and ``maximum`` are None exactly when the Decision_Point was
    ``skipped`` (no quantities, a Closed_Enum, or a selected Format).
    """

    kind: QuantityKind
    quantities: QuantitySet
    minimum: BoundSide | None = None
    maximum: BoundSide | None = None
    skipped: SkipReason | None = None

    @property
    def sides(self) -> tuple[BoundSide, ...]:
        """The decided sides, minimum first."""
        return tuple(side for side in (self.minimum, self.maximum) if side is not None)

    def keywords(self) -> dict[str, JSON]:
        """The emitted bound keywords with their values, lower bound first."""
        return {side.keyword: side.value for side in self.sides if side.value is not None}

    def provenance(self) -> dict[str, ProvenanceRecord]:
        """The Provenance_Record of each emitted bound, keyed by keyword.

        Every quantity satisfies an emitted bound (16.6), so its Support is
        ``(total, total)``; Confidence is the decision's MDL posterior.
        """
        support = Support(self.quantities.total, self.quantities.total)
        return {side.keyword: side.provenance(support) for side in self.sides if side.value is not None}


def _label(keyword: str, value: Number | None) -> str:
    return f"{keyword}={_OMIT_LABEL if value is None else json.dumps(value)}"


def _base(base: Mapping[str, Any] | SchemaNode | None, q: QuantitySet) -> dict[str, Any]:
    """The keywords every candidate node holds besides its bounds (default: the type of the quantities' rows)."""
    if base is None:
        name = "integer" if q.kind == NUMBER and q.integers else _KIND_TYPES[q.kind]
        return {"type": name}
    if isinstance(base, SchemaNode):
        schema = base.to_json()
        if not isinstance(schema, dict):
            raise ValueError("base must be a keyword schema, not a boolean schema")
        base = schema
    if not isinstance(base, Mapping):
        raise TypeError(f"base must be a keyword mapping or a SchemaNode, got {type(base).__name__}")
    clash = [keyword for keyword in BOUND_KEYWORDS[q.kind] if keyword in base]
    if clash:
        raise ValueError(f"base must not hold the bounds being decided, found {clash}")
    return dict(base)


def _leave_one_out_value(rung: Rung, others: Number, side: Side) -> Number:
    """The rung's value had the one document attaining the extreme been left out: built from ``others``."""
    if rung.name == RUNG_OBSERVED:
        return others
    if rung.name == RUNG_ONE_DIGIT:
        return round_outward(others, 1, side)
    if rung.name == RUNG_TWO_DIGITS:
        return round_outward(others, 2, side)
    assert rung.value is not None
    return rung.value  # zero and the integer limits do not depend on the quantities


def _constant(rung: Rung, q: QuantitySet, side: Side) -> bool:
    """Whether the rung's value is also a constant proposal (zero, or an integer limit of integer quantities).

    The ladder keeps the first name of a value proposed twice, so an observed
    extreme of 0 is the ``observed`` rung; leaving a document out cannot move
    a constant, so such a rung survives.
    """
    if rung.value is None:
        return False
    exact = Fraction(rung.value)
    if exact == 0:
        return True
    return q.integers and any(Fraction(value) == exact for _, value in INTEGER_LIMIT_RUNGS[side])


def _refutation(
    support: QuantitySupport, side: Side, rung: Rung, extreme: Number, q: QuantitySet
) -> dict[str, JSON] | None:
    """The ``withheld`` details of a rung that fails leave-one-document-out (16.9); None when it survives."""
    if rung.value is None:
        return None  # omission closes nothing
    if support.documents < 2:
        return withheld(support.documents)
    facts = support.minimum if side == MINIMUM else support.maximum
    if facts.unchanged or facts.others is None or _constant(rung, q, side):
        return None
    value = _leave_one_out_value(rung, facts.others, side)
    covers = Fraction(value) <= Fraction(extreme) if side == MINIMUM else Fraction(value) >= Fraction(extreme)
    return None if covers else withheld(support.documents, leave_one_out_value=value)


def _decide_side(
    engine: DecisionEngine,
    dp: DecisionPoint,
    q: QuantitySet,
    side: Side,
    base: Mapping[str, Any],
    low: Number | None,
    scorer: BoundScorer,
    support: QuantitySupport | None,
) -> BoundSide:
    """Resolve one side; for the maximum, ``low`` is the chosen minimum (None when omitted)."""
    keyword = BOUND_KEYWORDS[q.kind][0 if side == MINIMUM else 1]
    extreme = q.extreme(side)
    candidates: list[Candidate] = []
    if engine.enabled(BOUNDS):  # a disabled Technique produces no candidates (28.4, 16.8)
        ladder = bound_ladder(q, side)
        ranks = _distance_ranks(ladder, extreme)
        for index, (rung, rank) in enumerate(zip(ladder, ranks)):
            schema = dict(base)
            if rung.value is not None:
                schema[keyword] = rung.value
            node = SchemaNode.from_json(schema)
            score = scorer(q, node, rung.value, None) if side == MINIMUM else scorer(q, node, low, rung.value)
            if not isinstance(score, Score):
                raise TypeError(f"the bound scorer must return a Score, got {type(score).__name__}")
            candidates.append(
                Candidate(
                    technique=BOUNDS,
                    outcome=rung.value,
                    label=_label(keyword, rung.value),
                    order_key=(index,),
                    node=node,
                    score=score,
                    tie_ranks={} if rung.value is None else {TIE_NEAREST_EXTREME: rank},
                    withheld=None if support is None else _refutation(support, side, rung, extreme, q),
                )
            )
    decision = engine.resolve(dp, candidates, mdl_by=BOUNDS)
    value = None if decision.candidate is None else decision.candidate.outcome
    return BoundSide(side, keyword, value, extreme, decision)


def decide_bound(
    quantities: Histogram | Sequence[Number] | np.ndarray | QuantitySet,
    kind: QuantityKind,
    engine: DecisionEngine,
    *,
    counts: Sequence[int] | np.ndarray | None = None,
    path: Path | str | None = None,
    base: Mapping[str, Any] | SchemaNode | None = None,
    closed_enum: bool = False,
    format_selected: bool = False,
    scorer: BoundScorer | None = None,
    support: QuantitySupport | None = None,
) -> BoundDecision:
    """Decide the lower and upper bound of the non-flagged ``quantities`` at ``path`` (16.1–16.9).

    ``kind`` names the quantities: ``"number"`` (JSON numbers),
    ``"string_length"`` (code points), ``"item_count"``, or ``"key_count"``
    (a Map's key counts). ``counts`` gives each quantity's multiplicity
    unless ``quantities`` is a Histogram or a QuantitySet. ``base`` holds
    the other keywords of the node the bounds join (its model bits count;
    by default the quantities' ``type``). ``closed_enum`` marks a Closed_Enum
    Path (numbers and strings) and ``format_selected`` a string Path with a
    selected Format; both skip the Decision_Point. ``scorer`` replaces
    :func:`mdl_bound_score`. ``support`` carries the documents behind the
    quantities; with it, rungs that fail leave-one-document-out are offered
    withheld (16.9). The engine resolves the minimum, then the maximum with
    the chosen minimum, and records both in its report.
    """
    kind = _choice(kind, QUANTITY_KINDS, "quantity kind")
    if not isinstance(engine, DecisionEngine):
        raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
    if support is not None and not isinstance(support, QuantitySupport):
        raise TypeError(f"support must be a QuantitySupport or None, got {type(support).__name__}")
    if _flag(closed_enum, "closed_enum") and kind not in (NUMBER, STRING_LENGTH):
        raise ValueError(f"closed_enum applies to numbers and string lengths, not to {kind!r} (16.1, 16.3)")
    if _flag(format_selected, "format_selected") and kind != STRING_LENGTH:
        raise ValueError(f"format_selected applies to string lengths, not to {kind!r} (16.3)")
    if isinstance(quantities, QuantitySet):
        if quantities.kind != kind:
            raise ValueError(f"the QuantitySet holds {quantities.kind!r} quantities, not {kind!r}")
        if counts is not None:
            raise ValueError("a QuantitySet carries its own counts; pass no counts with it")
        q = quantities
    else:
        q = QuantitySet.of(kind, quantities, counts)
    skipped: SkipReason | None = None
    if q.total == 0:
        skipped = "no-quantities"
    elif closed_enum:
        skipped = "closed-enum"
    elif format_selected:
        skipped = "format"
    if skipped is not None:
        return BoundDecision(kind, q, skipped=skipped)

    score = mdl_bound_score if scorer is None else scorer
    dp = DecisionPoint(BOUND_DECISION_POINT, path)
    node_base = _base(base, q)
    minimum = _decide_side(engine, dp, q, MINIMUM, node_base, None, score, support)
    if minimum.value is not None:
        node_base = {**node_base, minimum.keyword: minimum.value}
    maximum = _decide_side(engine, dp, q, MAXIMUM, node_base, minimum.value, score, support)
    return BoundDecision(kind, q, minimum, maximum)
