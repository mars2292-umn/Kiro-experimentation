"""Property 26: Bounds come from the candidate ladder (task 12.8).

For any set of observed quantities (JSON numbers, string lengths in code
points, array element counts, Map key counts), ``decide_bound`` offers each
side exactly the candidates of 16.2, every emitted bound is one of them,
every quantity satisfies it (checked in exact arithmetic and with the
Validator), and bounds on lengths and counts are non-negative integers. With a
stub scorer that forces description-length ties, the selected candidate is
the tied one nearest the observed extreme, omission being the farthest (16.7).

Reference ladder (independent of ``values.bounds``)
    For a side with observed extreme x (the least quantity for a minimum, the
    greatest for a maximum): omission; x; 0; x rounded outward (floor for a
    minimum, ceiling for a maximum) to 1 and to 2 significant digits; and,
    when every quantity is integral (``1.0`` and ``1e3`` count, 13.2), the
    lower (minimum) or upper (maximum) limits of the signed and unsigned 8-,
    16-, 32-, and 64-bit ranges, written out as literals. Rounding is exact
    Fraction arithmetic on the decimal the JSON number denotes: its exact
    value when integral, otherwise its shortest round-trip spelling
    (``repr``), with ``Decimal.adjusted`` giving the exponent of the leading
    digit. A rounded value is an ``int`` when integral, otherwise the double
    that ``float`` reads from its decimal text (correctly rounded). Candidates
    a quantity violates, negative candidates for lengths and counts, and
    repeated values (exact equality) are dropped.

Scenarios
    Numbers mix small and big integers (beyond 64 bits), integral floats,
    short decimals (``m·10^e``), arbitrary finite floats (subnormal to
    ``1.7976931348623157e308``), and edges around the integer limits and the
    rounding carries (137, 999, 0.95). Lengths and counts mix small values
    and edges around the unsigned limits (255/256, 65535/65536). Every list
    has repeated values in a drawn order, and may carry multiplicities
    (``counts=``) with zeros, which drop their quantity.

Checks
    1. Real MDL scorer (wrapped to record its calls): the minimum is offered
       exactly the reference ladder with the maximum omitted, then the maximum
       exactly its ladder with the chosen minimum in place; each emitted bound
       is a reference candidate (same value and JSON type), satisfied by every
       quantity, a non-negative int for lengths and counts, and accepted by
       the Validator on instances of every quantity (strings of astral
       characters, arrays, objects); the default scorer emits the same.
    2. Forced ties: each candidate's Score comes from a drawn table (often a
       shared description length, with the model/data split drawn so the
       nearest-extreme rule must beat fewer model bits); each side selects,
       among the candidates of least description length, the one nearest the
       observed extreme, omission last.
    3. End to end (16.3): strings with non-ASCII, combining, ZWJ, BOM, and
       astral characters, or hostname-like ASCII strings (so a Format is
       often selected), copied so bounds can pay, and maybe numbers, go
       through ``ValueInferrer.infer_scalar`` with the Closed-or-Open-Enum,
       Format, and Latent_Type Techniques toggled; the length quantities are the
       code-point lengths, the length bounds are skipped exactly at a
       Closed_Enum or a selected Format, and otherwise each emitted
       ``minLength``/``maxLength`` (and ``minimum``/``maximum``) is a
       reference candidate the Validator accepts every value under.

**Validates: Requirements 16.2, 16.3, 16.4, 16.5, 16.7**
"""

# Feature: json-schema-inference, Property 26: Bounds come from the candidate ladder

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from decimal import Decimal
from fractions import Fraction
from typing import Any, TypeAlias

from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import Score
from jsinfer.values.bounds import BoundDecision, BoundSide, QuantitySet, decide_bound, mdl_bound_score
from jsinfer.values.inferrer import ValueInferrer

Number: TypeAlias = int | float
Key: TypeAlias = Fraction | None  # a candidate's exact value; None is omission

BOUNDS = "values.bounds"
VALIDATOR = Validator()
SIDES = ("minimum", "maximum")
KINDS = ("number", "string_length", "item_count", "key_count")
KEYWORDS = {
    "number": ("minimum", "maximum"),
    "string_length": ("minLength", "maxLength"),
    "item_count": ("minItems", "maxItems"),
    "key_count": ("minProperties", "maxProperties"),
}
JSON_TYPES = {"number": "number", "string_length": "string", "item_count": "array", "key_count": "object"}

# The lower and upper limits of the signed and unsigned 8/16/32/64-bit integer ranges (16.2).
INTEGER_LIMITS = {
    "minimum": (
        ("int8", -128),
        ("int16", -32768),
        ("int32", -2147483648),
        ("int64", -9223372036854775808),
        ("uint8", 0),
        ("uint16", 0),
        ("uint32", 0),
        ("uint64", 0),
    ),
    "maximum": (
        ("int8", 127),
        ("int16", 32767),
        ("int32", 2147483647),
        ("int64", 9223372036854775807),
        ("uint8", 255),
        ("uint16", 65535),
        ("uint32", 4294967295),
        ("uint64", 18446744073709551615),
    ),
}


# ---------------------------------------------------------------------------
# The reference ladder
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Rung:
    name: str
    value: Number | None  # None: omit the bound


def key(value: Number | None) -> Key:
    return None if value is None else Fraction(value)


def canonical(x: Number) -> Number:
    """A JSON number as emitted: integral numbers as ``int``."""
    return int(x) if Fraction(x).denominator == 1 else x


def denoted_decimal(x: Number) -> Decimal:
    """The decimal a JSON number denotes: its exact value when integral, else its shortest round-trip spelling."""
    return Decimal(int(x)) if Fraction(x).denominator == 1 else Decimal(repr(x))


def round_out(x: Number, digits: int, side: str) -> Number:
    """``x`` rounded outward to ``digits`` significant digits: floor for a minimum, ceiling for a maximum."""
    d = denoted_decimal(x)
    if d == 0:
        return 0
    k = d.adjusted() - digits + 1  # the exponent of the last kept digit
    scaled = Fraction(d) / Fraction(10) ** k
    n = math.floor(scaled) if side == "minimum" else math.ceil(scaled)
    rounded = n * Fraction(10) ** k
    return int(rounded) if rounded.denominator == 1 else float(f"{n}e{k}")


def extreme_of(quantities: Sequence[Number], side: str) -> Fraction:
    exact = [Fraction(q) for q in quantities]
    return min(exact) if side == "minimum" else max(exact)


def reference_ladder(quantities: Sequence[Number], kind: str, side: str) -> list[Rung]:
    """The candidates of 16.2 for one side, omission first (see the module docstring)."""
    exact = [Fraction(q) for q in quantities]
    extreme = extreme_of(quantities, side)
    x = canonical(quantities[exact.index(extreme)])
    proposed: list[tuple[str, Number]] = [
        ("observed", x),
        ("zero", 0),
        ("1-digit", round_out(x, 1, side)),
        ("2-digits", round_out(x, 2, side)),
    ]
    if all(f.denominator == 1 for f in exact):
        proposed.extend(INTEGER_LIMITS[side])
    ladder = [Rung("omit", None)]
    seen: set[Fraction] = set()
    for name, value in proposed:
        f = Fraction(value)
        if (f > extreme) if side == "minimum" else (f < extreme):
            continue  # a quantity violates it
        if kind != "number" and f < 0:
            continue
        if f in seen:
            continue
        seen.add(f)
        ladder.append(Rung(name, value))
    return ladder


def live_quantities(values: Sequence[Number], counts: Sequence[int] | None) -> list[Number]:
    """The quantities with a positive count (every one when there are no counts)."""
    return list(values) if counts is None else [v for v, c in zip(values, counts) if c > 0]


def distinct(quantities: Sequence[Number]) -> list[Number]:
    return list({Fraction(q): q for q in quantities}.values())


# ---------------------------------------------------------------------------
# Scorers
# ---------------------------------------------------------------------------

# A forced Score as (model, data) in half bits, so every sum is exact.
Table: TypeAlias = Mapping[Key, tuple[int, int]]
UNLISTED = Score(0.0, 1000.0, 0.0, 0, 1)


class RecordingScorer:
    """Records every call; scores with ``mdl_bound_score``, or from a per-side table of forced Scores.

    The first ``n_minimum`` calls score the minimum's candidates (the
    reference ladder's length), the rest the maximum's.
    """

    def __init__(self, n_minimum: int, tables: Mapping[str, Table] | None = None) -> None:
        self.n_minimum = n_minimum
        self.tables = tables
        self.calls: list[tuple[Number | None, Number | None]] = []

    def __call__(self, q: QuantitySet, node: SchemaNode, low: Number | None, high: Number | None) -> Score:
        side = "minimum" if len(self.calls) < self.n_minimum else "maximum"
        self.calls.append((low, high))
        if self.tables is None:
            return mdl_bound_score(q, node, low, high)
        halves = self.tables[side].get(key(low if side == "minimum" else high))
        if halves is None:
            return UNLISTED  # not a reference candidate: check_offered reports it
        model, data = halves
        return Score(model / 2, data / 2, 0.0, 0, q.total)


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def check_offered(scorer: RecordingScorer, ladders: Mapping[str, list[Rung]], decision: BoundDecision) -> None:
    """16.2: each side is offered exactly its ladder; the maximum is decided given the chosen minimum."""
    n_minimum = len(ladders["minimum"])
    expected = {side: Counter(key(rung.value) for rung in ladders[side]) for side in SIDES}
    assert len(scorer.calls) == n_minimum + len(ladders["maximum"]), (
        f"scored {scorer.calls}, expected the ladders {ladders}"
    )
    first, second = scorer.calls[:n_minimum], scorer.calls[n_minimum:]
    assert all(high is None for _, high in first), f"the minimum was scored with a maximum: {first}"
    assert Counter(key(low) for low, _ in first) == expected["minimum"], (
        f"minimum candidates {[low for low, _ in first]}, expected {ladders['minimum']}"
    )
    assert decision.minimum is not None
    chosen = key(decision.minimum.value)
    assert all(key(low) == chosen for low, _ in second), f"the maximum was scored without the chosen minimum: {second}"
    assert Counter(key(high) for _, high in second) == expected["maximum"], (
        f"maximum candidates {[high for _, high in second]}, expected {ladders['maximum']}"
    )


def check_side(
    side_decision: BoundSide | None, side: str, kind: str, quantities: Sequence[Number], ladder: Sequence[Rung]
) -> Rung:
    """The reference rung of the emitted bound (omission when omitted), checked to be sound and well-typed."""
    assert side_decision is not None, f"no {side} decision"
    keyword = KEYWORDS[kind][SIDES.index(side)]
    assert side_decision.keyword == keyword
    assert Fraction(side_decision.extreme) == extreme_of(quantities, side)
    value = side_decision.value
    rung = next((r for r in ladder if key(r.value) == key(value)), None)
    assert rung is not None, f"{keyword}={value!r} is not a 16.2 candidate; the ladder is {ladder}"
    if value is None:
        return rung
    assert type(value) is type(rung.value), f"{keyword}={value!r} should be the {type(rung.value).__name__} {rung}"
    bound = Fraction(value)
    violated = [q for q in quantities if (Fraction(q) < bound if side == "minimum" else Fraction(q) > bound)]
    assert not violated, f"{keyword}={value!r} is violated by {violated}"
    if kind != "number":
        assert type(value) is int and value >= 0, f"{keyword}={value!r} must be a non-negative int"
    return rung


def instance(kind: str, q: Number) -> Any:
    """A JSON value whose bounded quantity is ``q``."""
    if kind == "number":
        return q
    n = int(q)
    if kind == "string_length":
        return "\U0001f600" * n  # n code points (2n UTF-16 units, 4n UTF-8 bytes)
    if kind == "item_count":
        return [None] * n
    return {f"k{i}": None for i in range(n)}


def check_validator(kind: str, keywords: Mapping[str, Any], instances: Sequence[Any]) -> None:
    """16.6 by the Validator: every instance satisfies the emitted bounds."""
    compiled = VALIDATOR.compile({"type": JSON_TYPES[kind], **keywords})
    rejected = [i for i in instances if not VALIDATOR.is_valid(compiled, i)]
    assert not rejected, f"{keywords} rejects {[describe(i) for i in rejected]}"


def describe(value: Any) -> str:
    if isinstance(value, (str, list, dict)):
        return f"{type(value).__name__} of length {len(value)}"
    return repr(value)


# ---------------------------------------------------------------------------
# Strategies
# ---------------------------------------------------------------------------

NUMBER_EDGES = (
    0, -0.0, 1.0, -1.0, 1e3, -1e3, 0.5, 9.5, 0.95, 0.1, 0.3, 2.675, -0.007, 137, -137, 999, -999, 1000,
    127, 128, -128, -129, 255, 256, 32767, 32768, -32769, 65535, 65536, 2**31 - 1, 2**32, -(2**31) - 1,
    2**63 - 1, 2**63, -(2**63), -(2**63) - 1, 2**64 - 1, 2**64, 2**70 + 1, -(2**70), 10**30, 2**53 + 1,
    1e23, 2.0**64, -(2.0**63), 5e-324, -5e-324, 1e-300, 2.2250738585072014e-308, 1e300, -1e300,
    1.7976931348623157e308, -1.7976931348623157e308,
)  # fmt: skip
NUMBERS = st.one_of(
    st.integers(-300, 300),
    st.integers(-(2**80), 2**80),
    st.integers(-(10**6), 10**6).map(float),  # integral floats
    st.builds(lambda m, e: float(f"{m}e{e}"), st.integers(-9999, 9999), st.integers(-12, 12)),  # short decimals
    st.floats(allow_nan=False, allow_infinity=False),  # subnormal to huge magnitudes
    st.sampled_from(NUMBER_EDGES),
)
COUNT_EDGES = (0, 1, 2, 9, 10, 11, 99, 100, 101, 127, 128, 137, 255, 256, 999, 1000, 32767, 32768, 65535, 65536)
COUNTS = st.integers(0, 40) | st.sampled_from(COUNT_EDGES)
ELEMENTS = {"number": NUMBERS, "string_length": COUNTS, "item_count": COUNTS, "key_count": COUNTS}

# Non-ASCII, a combining mark, ZWJ, BOM, a CJK character, and astral characters: one code point each.
CHARS = "ab0 -.é\u0301中\u200d\ufeff\U0001f600\U0010ffff"
# Hostname-like strings, so a Format is often selected while values.format is enabled.
HOSTNAME_CHARS = "ab0"
LENGTH_EDGES = (9, 10, 11, 99, 100, 101, 127, 128, 137, 255, 256)


def strings_of(alphabet: str) -> st.SearchStrategy[str]:
    return st.one_of(
        st.text(alphabet=alphabet, max_size=8),
        st.builds(lambda ch, n: ch * n, st.sampled_from(alphabet), st.sampled_from(LENGTH_EDGES)),
    )


@st.composite
def string_documents(draw: st.DrawFn) -> list[str]:
    """Strings over one alphabet, with repeats, the whole list copied 1 to 20 times (so bounds can pay)."""
    alphabet = draw(st.sampled_from((CHARS, HOSTNAME_CHARS)), label="alphabet")
    strings = draw(with_repeats(strings_of(alphabet)), label="strings")
    return strings * draw(st.integers(1, 20), label="copies")


@st.composite
def with_repeats(draw: st.DrawFn, elements: st.SearchStrategy, max_distinct: int = 8) -> list[Any]:
    """A list of ``elements`` with repeated values, in a drawn order."""
    drawn = draw(st.lists(elements, min_size=1, max_size=max_distinct))
    repeats = draw(st.lists(st.sampled_from(drawn), max_size=4))
    return draw(st.permutations([*drawn, *repeats]))


@dataclass(frozen=True)
class Scenario:
    kind: str
    values: list[Number]
    counts: list[int] | None  # multiplicities aligned with values; at least one is positive


@st.composite
def scenarios(draw: st.DrawFn) -> Scenario:
    kind = draw(st.sampled_from(KINDS), label="kind")
    values = draw(with_repeats(ELEMENTS[kind]), label="quantities")
    counts = None
    if draw(st.booleans(), label="with counts"):
        counts = draw(st.lists(st.integers(0, 3) | st.integers(0, 400), min_size=len(values), max_size=len(values)))
        if not any(counts):
            counts[0] = 1
    return Scenario(kind, values, counts)


def draw_table(data: st.DataObject, ladder: Sequence[Rung], side: str) -> dict[Key, tuple[int, int]]:
    """Forced Scores: each candidate takes a shared description length or draws its own."""
    shared = data.draw(st.integers(0, 4), label=f"{side} shared description length (half bits)")
    table: dict[Key, tuple[int, int]] = {}
    for rung in ladder:
        own = data.draw(st.booleans(), label=f"{rung.name} draws its own description length")
        total = data.draw(st.integers(0, 6), label="description length (half bits)") if own else shared
        model = data.draw(st.integers(0, total), label="model bits (half bits)")
        table[key(rung.value)] = (model, total - model)
    return table


def nearest_tied(ladder: Sequence[Rung], extreme: Fraction, table: Table) -> tuple[Rung, int]:
    """16.7: among the candidates of least description length, the nearest the extreme (omission farthest)."""
    totals = {key(r.value): sum(table[key(r.value)]) for r in ladder}
    least = min(totals.values())
    tied = [r for r in ladder if totals[key(r.value)] == least]
    winner = min(tied, key=lambda r: (1, Fraction(0)) if r.value is None else (0, abs(Fraction(r.value) - extreme)))
    return winner, len(tied)


def engine() -> DecisionEngine:
    return DecisionEngine({BOUNDS: True})


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


@given(scenarios())
def test_every_emitted_bound_is_a_satisfied_candidate_of_the_ladder(scenario: Scenario) -> None:
    kind = scenario.kind
    live = live_quantities(scenario.values, scenario.counts)
    ladders = {side: reference_ladder(live, kind, side) for side in SIDES}
    note(f"ladders: {ladders}")

    scorer = RecordingScorer(len(ladders["minimum"]))
    decision = decide_bound(scenario.values, kind, engine(), counts=scenario.counts, scorer=scorer)  # type: ignore[arg-type]
    assert decision.skipped is None
    check_offered(scorer, ladders, decision)
    for side in SIDES:
        rung = check_side(getattr(decision, side), side, kind, live, ladders[side])
        event(f"{kind} {side}: {rung.name}")

    keywords = decision.keywords()
    assert list(keywords) == [s.keyword for s in decision.sides if s.value is not None]
    check_validator(kind, keywords, [instance(kind, q) for q in distinct(live)])
    # The default scorer is the MDL bound score.
    assert decide_bound(scenario.values, kind, engine(), counts=scenario.counts).keywords() == keywords  # type: ignore[arg-type]


@given(scenarios(), st.data())
def test_description_length_ties_go_to_the_candidate_nearest_the_observed_extreme(
    scenario: Scenario, data: st.DataObject
) -> None:
    kind = scenario.kind
    live = live_quantities(scenario.values, scenario.counts)
    ladders = {side: reference_ladder(live, kind, side) for side in SIDES}
    tables = {side: draw_table(data, ladders[side], side) for side in SIDES}
    note(f"ladders: {ladders}")

    scorer = RecordingScorer(len(ladders["minimum"]), tables)
    decision = decide_bound(scenario.values, kind, engine(), counts=scenario.counts, scorer=scorer)  # type: ignore[arg-type]
    check_offered(scorer, ladders, decision)
    for side in SIDES:
        side_decision = getattr(decision, side)
        rung = check_side(side_decision, side, kind, live, ladders[side])
        expected, n_tied = nearest_tied(ladders[side], extreme_of(live, side), tables[side])
        assert side_decision.decision.method == "mdl"
        assert rung == expected, f"{side}: selected {rung}, expected {expected} (ladder {ladders[side]}, {tables[side]})"
        event(f"{side}: {'all' if n_tied == len(ladders[side]) else 'some'} candidates tied, {expected.name} selected")
    check_validator(kind, decision.keywords(), [instance(kind, q) for q in distinct(live)])


@given(
    string_documents(),
    st.one_of(st.just([]), st.builds(lambda ns, k: ns * k, with_repeats(NUMBERS, 4), st.integers(1, 20))),
    st.fixed_dictionaries(
        {tid: st.booleans() for tid in ("values.enum_estimation", "values.format", "values.latent_type")}
    ),
    st.data(),
)
def test_string_lengths_are_bounded_in_code_points_through_the_value_inferrer(
    strings: list[str], numbers: list[Number], states: dict[str, bool], data: st.DataObject
) -> None:
    documents = data.draw(st.permutations([*strings, *numbers]), label="document order")
    inferrer = ValueInferrer.for_techniques({BOUNDS: True, **states})
    result = inferrer.infer_scalar(View.root(build_store(documents)))
    emitted = result.keywords()

    # 16.3: string lengths in Unicode code points, skipped at a Closed_Enum or a selected Format.
    lengths = [len(s.encode("utf-32-le")) // 4 for s in strings]
    bounds = result.length_bounds
    assert bounds is not None
    format_selected = result.string is not None and result.string.keyword == "format"
    skipped = "closed-enum" if result.enum.closed else "format" if format_selected else None
    assert bounds.skipped == skipped
    assert dict(zip(bounds.quantities.values, bounds.quantities.counts)) == Counter(lengths)
    length_keywords = {k: emitted[k] for k in KEYWORDS["string_length"] if k in emitted}
    assert length_keywords == bounds.keywords()
    if skipped is None:
        for side in SIDES:
            ladder = reference_ladder(lengths, "string_length", side)
            rung = check_side(getattr(bounds, side), side, "string_length", lengths, ladder)
            event(f"string_length {side}: {rung.name}")
    else:
        assert not length_keywords
        event(f"length bounds skipped: {skipped}")
    check_validator("string_length", length_keywords, strings)

    # Numbers at the same Path (16.1, 16.2): skipped only at a Closed_Enum.
    number_bounds = result.number_bounds
    event("with numbers" if numbers else "strings only")
    if not numbers:
        assert number_bounds is None
        return
    assert number_bounds is not None
    assert number_bounds.skipped == ("closed-enum" if result.enum.closed else None)
    observed = Counter(Fraction(n) for n in numbers)
    assert {Fraction(v): c for v, c in zip(number_bounds.quantities.values, number_bounds.quantities.counts)} == observed
    number_keywords = {k: emitted[k] for k in KEYWORDS["number"] if k in emitted}
    assert number_keywords == number_bounds.keywords()
    if number_bounds.skipped is None:
        for side in SIDES:
            ladder = reference_ladder(numbers, "number", side)
            rung = check_side(getattr(number_bounds, side), side, "number", numbers, ladder)
            event(f"number {side}: {rung.name}")
    check_validator("number", number_keywords, numbers)
