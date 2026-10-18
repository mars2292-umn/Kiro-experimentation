"""Property 25: Enum statistics and closure rules (task 12.7).

For any multiset of scalar values at a Path, the 15.1 statistics computed by
``values.enums`` equal an independent reference: the observation count n,
the distinct value count S_obs, the counts f1 and f2 of values observed
exactly once and exactly twice, the Good–Turing estimate f1 / n, and Chao1,
``S_obs + f1² / (2·f2)``, or ``S_obs + f1·(f1 − 1) / 2`` when f2 is 0. They
are taken over the non-flagged scalar Observations of the Path, with values
distinct under draft 2020-12 ``enum`` equality.

The reference keys a value by its JSON type and exact mathematical value
(``fractions.Fraction``), not by ``core.jsonvalue``: ``1``, ``1.0``, and
``1e0`` are one value, as are ``0`` and ``-0.0``; ``true`` never equals
``1``; ``"1"`` is a string; an integral float beyond 2**53 equals exactly
the integer it is (``float(2**70)`` is ``2**70``, ``float(2**53 + 1)`` is
``2**53``, not ``2**53 + 1``), and ``1e300`` equals its 301-digit integer.

Closure (15.5): a Path whose distinct values are all singletons
(f1 = S_obs), or that has no non-flagged scalar Observation, is judged
open, both under MDL and when a Closed-or-Open-Enum Decision_Model votes
closed with certainty. Emission (15.3): a Closed_Enum's keywords are
``const`` when S_obs is 1 and otherwise ``enum`` listing each distinct
non-flagged scalar value exactly once (no two equal under the reference
equality, and exactly the reference's distinct values); the ValueInferrer
emits exactly those keywords, and none for an open Path. The Validator
accepts every non-flagged scalar value, as written in its document, against
the emitted ``enum`` or ``const``.

Inputs
    Values come from a pool of values that enum equality merges or keeps
    apart, mixed with arbitrary integers, finite floats, and strings. A
    case's multiset is arbitrary, all distinct (so f1 = S_obs), or a block
    of values repeated two to four times with a few extras (so closure is
    common). Each value is a document (``$``) or the member ``v`` of an
    object document (``$.v``), mixed with rows that are not scalar
    Observations of the Path (containers at the Path; in the nested shape
    also documents without ``v`` and top-level scalars), in any order; any
    documents may be flagged.

**Validates: Requirements 15.1, 15.3, 15.5**
"""

# Feature: json-schema-inference, Property 25: Enum statistics and closure rules

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from fractions import Fraction
from typing import Any

import numpy as np
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import decision_model_artifact
from jsinfer.values.enums import (
    CLOSED_OR_OPEN_ENUM,
    DM_CLOSED_ENUM,
    ENUM_ESTIMATION,
    EnumJudgment,
    EnumStats,
    enum_statistics,
    judge_enum,
)
from jsinfer.values.inferrer import ValueInferrer

# One Validator for the module, so each emitted enum compiles once.
VALIDATOR = Validator()

# The member holding the values in the nested shape ($.v).
KEY = "v"

# ---------------------------------------------------------------------------
# Reference enum equality and statistics
# ---------------------------------------------------------------------------

RefKey = tuple[str, Any]


def ref_key(value: Any) -> RefKey:
    """The value under draft 2020-12 ``enum`` equality: JSON type, then exact value."""
    if value is None:
        return ("null", None)
    if isinstance(value, bool):  # before numbers: bool is an int subclass, but true is not 1
        return ("boolean", value)
    if isinstance(value, (int, float)):
        assert math.isfinite(value), value
        return ("number", Fraction(value))  # exact: 1 == 1.0, 0 == -0.0, float(2**70) == 2**70
    if isinstance(value, str):
        return ("string", value)  # code points, no normalization
    raise AssertionError(f"not a JSON scalar: {value!r}")


@dataclass(frozen=True)
class Reference:
    """The 15.1 statistics of a non-empty multiset, in exact arithmetic."""

    counts: Mapping[RefKey, int]
    n: int
    s_obs: int
    f1: int
    f2: int
    good_turing: Fraction
    chao1: Fraction


def reference(values: Sequence[Any]) -> Reference:
    counts = Counter(ref_key(value) for value in values)
    n = sum(counts.values())
    s_obs = len(counts)
    f1 = sum(1 for count in counts.values() if count == 1)
    f2 = sum(1 for count in counts.values() if count == 2)
    if f2:
        chao1 = s_obs + Fraction(f1 * f1, 2 * f2)
    else:
        chao1 = s_obs + Fraction(f1 * (f1 - 1), 2)
    return Reference(dict(counts), n, s_obs, f1, f2, Fraction(f1, n), chao1)


def assert_matches(stats: EnumStats | None, ref: Reference) -> None:
    assert stats is not None, "statistics are computed for a Path with non-flagged scalar Observations"
    counts = (stats.n, stats.s_obs, stats.f1, stats.f2)
    assert counts == (ref.n, ref.s_obs, ref.f1, ref.f2), f"(n, S_obs, f1, f2) = {counts}, reference {ref}"
    assert all(type(c) is int for c in counts), counts
    # f1 / n is one correctly rounded division, as is float(Fraction(f1, n)).
    assert stats.good_turing == float(ref.good_turing), (stats.good_turing, ref.good_turing)
    # Chao1 in floating point rounds twice (the quotient, then the sum).
    assert math.isclose(stats.chao1, float(ref.chao1), rel_tol=1e-12, abs_tol=0.0), (stats.chao1, ref.chao1)


# ---------------------------------------------------------------------------
# Strategies
# ---------------------------------------------------------------------------

# Values that enum equality merges (0, 0.0, -0.0; 1, 1.0; 2, 2.0; 1e3, 1000;
# 2**53, float(2**53), float(2**53 + 1); 2**70, float(2**70); 1e300 and its
# integer) or keeps apart (true and 1, false and 0, "1" and 1, 2**53 + 1).
_POOL: tuple[Any, ...] = (
    None, True, False, 0, 0.0, -0.0, 1, 1.0, 2, 2.0, 2.5, -0.5, 1e3, 1000,
    2**53, float(2**53), 2**53 + 1, float(2**53 + 1), 2**70, float(2**70), 1e300, int(1e300),
    "", "a", "b", "1", "true", "null", "é", "😀",
)  # fmt: skip

scalars = st.one_of(
    st.sampled_from(_POOL),
    st.integers(),
    st.floats(allow_nan=False, allow_infinity=False),
    st.text(st.characters(exclude_categories=("Cs",)), max_size=4),
)



def spellings(value: Any) -> list[Any]:
    """Python values equal to ``value`` under enum equality: an integral number as int, exact float, and -0.0."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return [value]
    if isinstance(value, float) and not value.is_integer():
        return [value]
    exact = int(value)
    found: list[Any] = [exact]
    try:
        as_float = float(exact)
    except OverflowError:  # beyond the largest double
        as_float = None
    if as_float is not None and int(as_float) == exact:
        found.append(as_float)
    if exact == 0:
        found.append(-0.0)
    return found


# Rows at the Path that are not scalar Observations.
_CONTAINERS = st.one_of(
    st.lists(st.none() | st.integers(0, 2), max_size=2),
    st.dictionaries(st.just("k"), st.integers(0, 2), max_size=1),
)


@dataclass(frozen=True)
class Case:
    docs: tuple[Any, ...]
    flagged: tuple[int, ...]
    nested: bool
    live_values: tuple[Any, ...]  # the non-flagged scalar values at the Path, as written

    def view(self) -> View:
        """A fresh store's view of the Path (a fresh store, so no other call's flags leak in)."""
        store = build_store(list(self.docs))
        if self.flagged:
            store.flags.flag_documents(list(self.flagged))
        root = View.root(store)
        return root.child_key(KEY) if self.nested else root


@st.composite
def cases(draw: st.DrawFn) -> Case:
    shape = draw(st.sampled_from(["arbitrary", "all distinct", "repeated"]), label="multiset shape")
    if shape == "all distinct":
        values = draw(st.lists(scalars, unique_by=ref_key, max_size=12))
    elif shape == "repeated":
        block = draw(st.lists(scalars, min_size=1, max_size=6))
        repeats = draw(st.integers(2, 4), label="repeats")
        # Each repeat may spell a number differently (1 or 1.0, 0 or -0.0).
        values = [draw(st.sampled_from(spellings(value))) for _ in range(repeats) for value in block]
        values += draw(st.lists(scalars, max_size=4))
    else:
        values = draw(st.lists(scalars, max_size=30))

    nested = draw(st.booleans(), label="nested")
    if nested:
        noise_rows = st.one_of(
            _CONTAINERS.map(lambda container: {KEY: container}),  # not a scalar at $.v
            st.dictionaries(st.just("k"), scalars, max_size=1),  # $.v absent
            scalars,  # a scalar at $, not at $.v
        )
    else:
        noise_rows = _CONTAINERS
    noise = draw(st.lists(noise_rows, max_size=4), label="noise")

    rows = [(True, value) for value in values] + [(False, doc) for doc in noise]
    rows = draw(st.permutations(rows), label="document order")
    docs = tuple(({KEY: value} if nested else value) if is_value else value for is_value, value in rows)
    if docs and draw(st.booleans(), label="flag documents"):
        flagged = tuple(sorted(draw(st.sets(st.integers(0, len(docs) - 1), max_size=len(docs)))))
    else:
        flagged = ()
    skipped = set(flagged)
    live = tuple(value for i, (is_value, value) in enumerate(rows) if is_value and i not in skipped)
    return Case(docs, flagged, nested, live)


# ---------------------------------------------------------------------------
# A Decision_Model certain that every Path is closed
# ---------------------------------------------------------------------------

_ARTIFACT = decision_model_artifact(CLOSED_OR_OPEN_ENUM, "synthetic")


class AlwaysClosed:
    """A Closed-or-Open-Enum Decision_Model with calibrated probability 1 for closed."""

    classes = ("closed", "open")

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        return np.array([[1.0, 0.0]])


class AlwaysClosedBundle:
    """A Model_Bundle holding only :class:`AlwaysClosed` (no Learned_Thresholds)."""

    def model(self, name: str) -> Any | None:
        return AlwaysClosed() if name == _ARTIFACT else None

    def threshold(self, name: str) -> int | float | None:
        return None


MDL, MODEL = "MDL", "Decision_Model votes closed"


def check_judgment(judgment: EnumJudgment, ref: Reference | None, live_values: Sequence[Any]) -> None:
    """The 15.1 statistics, the 15.5 forced-open rule, and the 15.3 emission rule of one judgment."""
    if ref is None:
        assert judgment.stats is None
    else:
        assert_matches(judgment.stats, ref)
    if ref is None or ref.f1 == ref.s_obs:
        assert not judgment.closed, f"judged closed with all-singleton statistics {judgment.stats}"
        assert judgment.keywords() == {}
        return
    keywords = judgment.keywords()
    if not judgment.closed:
        assert keywords == {}
        return
    if ref.s_obs == 1:
        assert set(keywords) == {"const"}, keywords
        listed = [keywords["const"]]
    else:
        assert set(keywords) == {"enum"}, keywords
        listed = keywords["enum"]
        assert isinstance(listed, list), listed
    keys = [ref_key(value) for value in listed]
    assert len(set(keys)) == len(keys), f"a value is listed twice under enum equality: {listed}"
    assert set(keys) == set(ref.counts), f"listed {listed}, but the distinct values are {sorted(ref.counts, key=repr)}"
    compiled = VALIDATOR.compile(keywords)
    rejected = [value for value in live_values if not VALIDATOR.is_valid(compiled, value)]
    assert not rejected, f"the Validator rejects observed values {rejected} against {keywords}"


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


@given(cases())
def test_statistics_equal_the_reference_formulas(case: Case) -> None:
    stats = enum_statistics(case.view())
    if not case.live_values:
        assert stats is None
        event("no non-flagged scalar Observation")
        return
    ref = reference(case.live_values)
    assert_matches(stats, ref)
    assert_matches(EnumStats.of_values(case.live_values), ref)
    assert_matches(EnumStats.from_counts(list(ref.counts.values())), ref)
    event("f2 = 0" if ref.f2 == 0 else "f2 > 0")
    if ref.s_obs < ref.n:
        event("values merged under enum equality" if len(set(map(repr, case.live_values))) > ref.s_obs else "repeats")


@given(cases(), st.sampled_from([MDL, MODEL]))
def test_all_singletons_are_open_and_closed_enums_list_each_value_once(case: Case, resolver: str) -> None:
    states = {ENUM_ESTIMATION: True, DM_CLOSED_ENUM: resolver == MODEL}
    bundle = AlwaysClosedBundle() if resolver == MODEL else None
    ref = reference(case.live_values) if case.live_values else None

    report = InferenceReport()
    judgment = judge_enum(case.view(), DecisionEngine(states, bundle=bundle, report=report), report=report)
    check_judgment(judgment, ref, case.live_values)

    # End to end: the ValueInferrer emits exactly the judgment's keywords.
    report = InferenceReport()
    inferrer = ValueInferrer.for_techniques(states, bundle=bundle, report=report)
    scalar = inferrer.infer_scalar(case.view())
    check_judgment(scalar.enum, ref, case.live_values)
    emitted = scalar.keywords()
    assert {kw: emitted[kw] for kw in ("const", "enum") if kw in emitted} == scalar.enum.keywords()

    if ref is None:
        event("no non-flagged scalar Observation")
    elif ref.f1 == ref.s_obs:
        event(f"all singletons, {resolver}")
    elif judgment.closed:
        event(f"closed: {judgment.keyword}, {resolver}")
    else:
        event(f"open, {resolver}")
