"""Unit tests for values/latent.py: Latent_Type detection and its candidates (task 12.1, 13.3, 13.9)."""

from __future__ import annotations

import itertools
import re
from typing import Any

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import Support
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine, DecisionPoint
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import model_bits
from jsinfer.values.latent import (
    ANNOTATION_KEYWORD,
    STRING_CONSTRAINT,
    TECHNIQUE,
    LatentDetection,
    LatentType,
    common_latent_type,
    detect_latent,
    detect_latent_view,
    latent_pattern,
    latent_type_of,
    view_strings,
)
from jsinfer.values.patterns import Pattern

VALIDATOR = Validator()

INTEGER, DECIMAL, BOOLEAN = LatentType.INTEGER, LatentType.DECIMAL, LatentType.BOOLEAN
DATE, DATE_TIME, TIME = LatentType.DATE, LatentType.DATE_TIME, LatentType.TIME


def assert_sound(found: LatentDetection, strings: list[str]) -> None:
    """The candidate accepts every string under the Validator, as 13.6 will check."""
    for s in strings:
        if found.keyword == "format":
            assert VALIDATOR.satisfies_format(found.value, s), s
        else:
            assert found.pattern is not None and found.pattern.matches(s), s
            assert VALIDATOR.matches_pattern(found.value, s), s


# ---------------------------------------------------------------------------
# Classifier
# ---------------------------------------------------------------------------

CLASSIFIED: dict[LatentType | None, list[str]] = {
    BOOLEAN: ["true", "false", "True", "FALSE", "tRuE"],
    INTEGER: ["0", "-0", "+0", "7", "-12", "+345", "123456789012345678901234567890"],
    DECIMAL: ["0.5", "-1.25", "+3.0", "1.0", "1e3", "1E-3", "-2.5e+10", "0e0"],
    DATE: [
        "2024-03-15", "2024/3/5", "2024.12.01", "0000-01-01", "2024-02-29",
        "15/03/2024", "03/15/2024", "1.5.2024", "31-12-1999",
    ],
    TIME: ["12:30", "9:05", "23:59:59", "23:59:60Z", "00:00:00.123z", "12:30:00+01:00", "12:30-0530", "1:02:03.5"],
    DATE_TIME: [
        "2024-03-15T12:30:00Z", "2024-03-15t12:30:00z", "2024-03-15 12:30", "03/15/2024 9:30",
        "2024-03-15T12:30:00.5+05:30",
    ],
    None: [
        "", " 1", "1 ", "1\n", "007", "00", "-", "+", "1.", ".5", "1e", "1,000", "NaN", "Infinity", "0x1F",
        "١٢", "１２",  # non-ASCII digits
        "yes", "t", "truee", "falſe",  # ſ case-folds to s, but only ASCII letters count
        "2023-02-29", "2024-13-01", "2024-00-10", "2024-01-32", "13/13/2024", "2024-03/15", "24-03-15",
        "24:00", "12:60", "12:5", "12:30:61", "12:30.5", "12:30:00+24:00", "12:30:00+01:60",
        "2024-03-15X12:30", "2024-03-15T", "2024-03-15T25:00", "2024-03-15  12:30", "2024-02-30T12:30:00Z",
        "abc", "A-12",
    ],
}


@pytest.mark.parametrize(("expected", "s"), [(t, s) for t, ss in CLASSIFIED.items() for s in ss])
def test_latent_type_of(expected: LatentType | None, s: str) -> None:
    assert latent_type_of(s) is expected


def test_latent_type_values_and_formats() -> None:
    assert [t.value for t in LatentType] == ["integer", "decimal", "boolean", "date", "date-time", "time"]
    assert [t.format for t in LatentType] == [None, None, None, "date", "date-time", "time"]
    assert [t.numeric for t in LatentType] == [True, True, False, False, False, False]
    with pytest.raises(TypeError):
        latent_type_of(1)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("strings", "expected"),
    [
        (["1", "22"], INTEGER),
        (["1", "2.5"], DECIMAL),  # a mix of integer and decimal strings is decimal (13.3)
        (["2.5", "1e3"], DECIMAL),
        (["true", "False"], BOOLEAN),
        (["2024-01-01", "2024-01-01T00:00:00Z"], None),  # date and date-time differ
        (["1", "true"], None),
        (["1", "x"], None),
        ([], None),
    ],
)
def test_common_latent_type(strings: list[str], expected: LatentType | None) -> None:
    for order in itertools.permutations(strings):
        assert common_latent_type(order) is expected


# ---------------------------------------------------------------------------
# Candidates
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("strings", "latent", "fmt"),
    [
        (["2024-03-15", "2023-12-01"], DATE, "date"),
        (["2024-03-15T12:30:00Z", "2024-03-15t01:02:03.5+05:30"], DATE_TIME, "date-time"),
        (["12:30:00Z", "23:59:60z", "00:00:00.1-01:00"], TIME, "time"),
    ],
)
def test_format_candidate_when_every_string_satisfies_it(strings: list[str], latent: LatentType, fmt: str) -> None:
    found = detect_latent(strings, VALIDATOR)
    assert found is not None
    assert (found.latent_type, found.keyword, found.value, found.pattern) == (latent, "format", fmt, None)
    assert found.constraint == {"format": fmt} and found.label == f"format={fmt}"
    assert found.annotation == (ANNOTATION_KEYWORD, latent.value) == ("x-latent-type", fmt)
    assert (found.n_strings, found.n_distinct, found.format_rejects) == (len(strings), len(strings), 0)
    assert found.details() == {"latent_type": fmt, "strings": len(strings), "distinct_strings": len(strings)}
    assert_sound(found, strings)


@pytest.mark.parametrize(
    ("strings", "latent", "source"),
    [
        (["-5", "12", "7"], INTEGER, "^(?:-[0-9]+|[0-9]+)$(?!\\n)"),
        (["1", "2.5", "-0.25"], DECIMAL, "^(?:-[0-9]+\\.[0-9]+|[0-9]+|[0-9]+\\.[0-9]+)$(?!\\n)"),
        (["1e3", "+2.5E-7"], DECIMAL, "^(?:\\+[0-9]+\\.[0-9]+[A-Z]-[0-9]+|[0-9]+[a-z][0-9]+)$(?!\\n)"),
        (["true", "false", "TRUE"], BOOLEAN, "^(?:[A-Z]{4}|[a-z]{4,5})$(?!\\n)"),
        (["3/15/2024", "12/1/2024"], DATE, "^(?:[0-9]{1,2}\\/[0-9]{1,2}\\/[0-9]{4})$(?!\\n)"),
        (["12:30", "9:05"], TIME, "^(?:[0-9]{1,2}:[0-9]{2})$(?!\\n)"),
        (
            ["2024-03-15 12:30:00", "2024-03-16T08:00:00Z"],
            DATE_TIME,
            "^(?:[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}"
            "|[0-9]{4}-[0-9]{2}-[0-9]{2}[A-Z][0-9]{2}:[0-9]{2}:[0-9]{2}[A-Z])$(?!\\n)",
        ),
    ],
)
def test_pattern_candidate(strings: list[str], latent: LatentType, source: str) -> None:
    found = detect_latent(strings, VALIDATOR)
    assert found is not None
    assert (found.latent_type, found.keyword, found.value) == (latent, "pattern", source)
    assert found.pattern == Pattern.from_source(source) and found.constraint == {"pattern": source}
    assert found.label == f"pattern={source}"
    assert_sound(found, strings)
    # Digit runs of numbers are open-ended; other runs keep their observed lengths.
    if latent is not BOOLEAN:
        longer = re.sub(r"[0-9]+", lambda run: run[0] * 3, strings[0])
        assert VALIDATOR.matches_pattern(source, longer) is latent.numeric


def test_temporal_pattern_records_format_rejections() -> None:
    # Two of the three date Observations satisfy the date Format, one (written twice) does not.
    strings, counts = ["2024-03-15", "2024-3-5", "2024-12-01"], [1, 2, 1]
    found = detect_latent(strings, VALIDATOR, counts=counts)
    assert found is not None
    assert (found.latent_type, found.keyword, found.format_rejects) == (DATE, "pattern", 2)
    assert (found.n_strings, found.n_distinct) == (4, 3)
    assert found.value == "^(?:[0-9]{4}-[0-9]{1,2}-[0-9]{1,2})$(?!\\n)"
    assert found.details() == {
        "latent_type": "date", "strings": 4, "distinct_strings": 3, "format": "date", "format_rejects": 2,
    }
    assert found.support == Support(4, 4)
    assert_sound(found, strings)


def test_no_latent_type_means_no_candidate() -> None:
    assert detect_latent([], VALIDATOR) is None
    assert detect_latent(["1", "x"], VALIDATOR) is None
    assert detect_latent(["2024-01-01", "12:30"], VALIDATOR) is None


def test_disabled_technique_produces_nothing() -> None:
    # 13.9: no x-latent-type annotation and no candidate.
    assert detect_latent(["2024-01-01"], VALIDATOR, enabled=False) is None
    view = View.root(build_store(["2024-01-01"]))
    assert detect_latent_view(view, VALIDATOR, enabled=False) is None
    assert detect_latent_view(view, VALIDATOR) is not None


def test_repeats_and_order_do_not_matter() -> None:
    strings = ["10", "-3", "2.5", "10", "7"]
    expected = detect_latent(strings, VALIDATOR)
    assert expected is not None and expected.n_strings == 5 and expected.n_distinct == 4
    for order in itertools.permutations(strings):
        found = detect_latent(order, VALIDATOR)
        assert found is not None and (found.value, found.n_strings) == (expected.value, expected.n_strings)
    tallied = detect_latent(["-3", "10", "2.5", "7"], VALIDATOR, counts=[1, 2, 1, 1])
    assert tallied is not None and (tallied.value, tallied.n_strings) == (expected.value, 5)


def test_count_validation() -> None:
    with pytest.raises(ValueError):
        detect_latent(["1", "2"], VALIDATOR, counts=[1])
    with pytest.raises(ValueError):
        detect_latent(["1"], VALIDATOR, counts=[0])
    with pytest.raises(TypeError):
        detect_latent([1], VALIDATOR)  # type: ignore[list-item]
    with pytest.raises(TypeError):
        detect_latent(["1"], object())  # type: ignore[arg-type]


def test_latent_pattern_direct() -> None:
    pattern = latent_pattern(["ab-1", "xyz-22", "Q"], DATE)  # any strings; profiles in code-point order
    assert pattern.source == "^(?:[A-Z]|[a-z]{2,3}-[0-9]{1,2})$(?!\\n)"
    assert all(pattern.matches(s) for s in ["ab-1", "xyz-22", "Q"])
    with pytest.raises(ValueError):
        latent_pattern([], INTEGER)
    with pytest.raises(TypeError):
        latent_pattern(["1"], "integer")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Views, candidates, and the registry
# ---------------------------------------------------------------------------


def test_view_strings_and_detection_skip_non_strings_and_flags() -> None:
    docs: list[Any] = [
        {"d": "2024-01-02"}, {"d": "2024-02-03"}, {"d": 5}, {"d": None}, {"d": "2024-01-02"}, {"d": "soon"},
    ]
    store = build_store(docs)
    view = View.root(store).child_key("d")
    assert view_strings(view) == (["2024-01-02", "2024-02-03", "soon"], [2, 1, 1])
    assert detect_latent_view(view, VALIDATOR) is None  # "soon" encodes no Latent_Type
    store.flags.flag_documents([5])
    assert view_strings(view) == (["2024-01-02", "2024-02-03"], [2, 1])
    found = detect_latent_view(view, VALIDATOR)
    assert found is not None
    assert (found.latent_type, found.keyword, found.value, found.n_strings, found.n_distinct) == (
        DATE, "format", "date", 3, 2,
    )
    assert detect_latent_view(View.root(build_store([1, None, True])), VALIDATOR) is None


def test_candidate_resolves_at_string_constraint() -> None:
    found = detect_latent(["12", "-4"], VALIDATOR)
    assert found is not None
    candidate = found.to_candidate({"type": ["string"]})
    assert (candidate.technique, candidate.outcome, candidate.label) == (TECHNIQUE, ("pattern", found.value), found.label)
    assert candidate.node is not None and candidate.node.to_json() == {"type": ["string"], "pattern": found.value}
    assert found.node().to_json() == {"pattern": found.value}  # nodes are fresh
    assert model_bits(candidate.node) > 0

    store = build_store(["12", "-4", "12"])
    view = View.root(store)
    point = DecisionPoint(STRING_CONSTRAINT)
    on = DecisionEngine({TECHNIQUE: True, "decide.mdl": False}).resolve(point, [candidate], view)
    assert (on.method, on.candidate, on.technique, on.outcome) == ("priority", candidate, TECHNIQUE, candidate.outcome)
    # With every serving Technique off, the Fallback_Behavior (no Format or Pattern) applies.
    off = DecisionEngine(dict.fromkeys(REGISTRY.servers(STRING_CONSTRAINT), False)).resolve(point, [candidate], view)
    assert (off.method, off.candidate, off.label) == ("fallback", None, "none")


def test_registry_entry() -> None:
    spec = REGISTRY.technique(TECHNIQUE)
    assert spec.implemented and spec.default_enabled
    assert spec.serves == (STRING_CONSTRAINT,) and spec.thresholds == ()
