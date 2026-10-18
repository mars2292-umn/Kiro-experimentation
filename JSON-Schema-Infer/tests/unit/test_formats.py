"""Unit tests for values/formats.py: Format detection through the Validator (task 12.2)."""

from __future__ import annotations

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.observations import build_store
from jsinfer.core.paths import parse_path
from jsinfer.core.report import RejectedCandidate
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine, DecisionPoint
from jsinfer.io.validator import Validator
from jsinfer.values.formats import (
    FORMATS,
    REJECTED_CATEGORY,
    TECHNIQUE,
    FormatCheck,
    FormatDetection,
    FormatDetector,
    format_candidates,
    format_enabled,
)

# One string per Format that the Validator accepts for it.
EXAMPLES = {
    "date-time": "2024-01-02T03:04:05Z",
    "date": "2024-01-02",
    "time": "03:04:05Z",
    "duration": "P1DT2H",
    "email": "someone@example.com",
    "hostname": "example.com",
    "ipv4": "192.168.0.1",
    "ipv6": "2001:db8::1",
    "uri": "https://example.com/a?b=c",
    "uuid": "123e4567-e89b-12d3-a456-426614174000",
}
NOISE = ["", "not a value!", "12:xx", "a b c"]


class CountingValidator(Validator):
    """The real Validator, counting how often each (Format, string) is asked for."""

    def __init__(self) -> None:
        super().__init__()
        self.calls: dict[tuple[str, str], int] = {}

    def satisfies_format(self, fmt: str, s: str) -> bool:
        self.calls[(fmt, s)] = self.calls.get((fmt, s), 0) + 1
        return super().satisfies_format(fmt, s)


def at(store, text: str) -> View:
    return View.root(store).descend(parse_path(text))


def reference(strings: list[str], validator: Validator) -> dict[str, tuple[int, int]]:
    """Brute force: per Format, (satisfied, rejected) over the strings with multiplicity."""
    out = {}
    for fmt in FORMATS:
        ok = sum(validator.satisfies_format(fmt, s) for s in strings)
        out[fmt] = (ok, len(strings) - ok)
    return out


def counts(detection: FormatDetection) -> dict[str, tuple[int, int]]:
    return {check.fmt: (check.satisfied, check.rejected) for check in detection.checks}


def test_formats_are_the_ten_of_requirement_13_4_in_order() -> None:
    assert FORMATS == tuple(EXAMPLES)
    assert REGISTRY.technique(TECHNIQUE).implemented and REGISTRY.technique(TECHNIQUE).default_enabled
    assert TECHNIQUE in REGISTRY.servers("string_constraint")


@pytest.mark.parametrize("fmt", FORMATS)
def test_each_example_is_a_candidate_for_its_format(fmt: str) -> None:
    detector = FormatDetector(build_store([]))
    detection = detector.detect_strings([EXAMPLES[fmt]])
    assert fmt in [check.fmt for check in detection.candidates]
    assert detection.check(fmt).satisfies_all


def test_view_detection_matches_the_validator_with_multiplicity() -> None:
    strings = [*EXAMPLES.values(), *NOISE, EXAMPLES["date"], EXAMPLES["date"], EXAMPLES["uuid"]]
    docs = [{"s": s} for s in strings] + [{"s": 5}, {"s": None}, {"s": True}, {"s": [EXAMPLES["date"]]}]
    store = build_store(docs, chunk_size=3)
    validator = Validator()
    detection = FormatDetector(store, validator).detect(at(store, "$.s"))
    # Non-string values (numbers, null, booleans, arrays) are not Format checks.
    assert detection.n_strings == len(strings)
    assert detection.n_distinct == len(set(strings))
    assert counts(detection) == reference(strings, validator)
    distinct = sorted(set(strings))
    for check in detection.checks:
        assert check.distinct_satisfied == sum(validator.satisfies_format(check.fmt, s) for s in distinct)
    expected = [fmt for fmt in FORMATS if reference(strings, validator)[fmt][0] > 0]
    assert [check.fmt for check in detection.candidates] == expected


def test_candidates_satisfying_and_withheld() -> None:
    dates = ["2024-01-02", "2023-12-31", "1999-02-28"]
    store = build_store([{"d": d} for d in dates] + [{"e": d} for d in dates] + [{"e": "x y"}])
    detector = FormatDetector(store)
    all_dates = detector.detect(at(store, "$.d"))
    assert "date" in [check.fmt for check in all_dates.satisfying]
    assert all_dates.withheld == tuple(c for c in all_dates.candidates if not c.satisfies_all)
    mixed = detector.detect(at(store, "$.e"))
    date = mixed.check("date")
    assert (date.satisfied, date.rejected, date.distinct_rejected) == (3, 1, 1)
    assert date.is_candidate and not date.satisfies_all
    assert date in mixed.withheld and date not in mixed.satisfying


def test_no_strings_means_no_candidates() -> None:
    store = build_store([{"n": 1}, {"n": 2.5}, {}])
    detection = FormatDetector(store).detect(at(store, "$.n"))
    assert detection.n_strings == 0 and detection.candidates == () and detection.satisfying == ()
    assert FormatDetector(store).detect(at(store, "$.missing")) == FormatDetection.empty()


def test_flagged_rows_are_left_out() -> None:
    store = build_store([{"d": "2024-01-02"}, {"d": "nope"}, {"d": "2024-01-03"}])
    detector = FormatDetector(store)
    assert detector.detect(at(store, "$.d")).check("date").rejected == 1
    store.flags.flag_documents([1])
    detection = detector.detect(at(store, "$.d"))
    assert detection.n_strings == 2 and detection.check("date").satisfies_all


def test_one_validator_check_per_distinct_string_and_format() -> None:
    strings = [EXAMPLES["date"]] * 5 + [EXAMPLES["email"]] * 3 + ["x"]
    store = build_store([{"a": s, "b": s, "c": {s: 1}} for s in strings])
    validator = CountingValidator()
    detector = FormatDetector(store, validator)
    detector.detect(at(store, "$.a"))
    detector.detect(at(store, "$.b"))  # same strings at another Path: no new checks
    detector.detect(at(store, "$.a"))
    distinct = set(strings)
    assert set(validator.calls) == {(fmt, s) for fmt in FORMATS for s in distinct}
    assert set(validator.calls.values()) == {1}
    detector.detect_keys(at(store, "$.c"))  # the same strings as keys have key ids: one check each
    assert set(validator.calls.values()) == {2}
    detector.detect_keys(at(store, "$.c"))
    assert max(validator.calls.values()) == 2


def test_keys_count_once_per_object() -> None:
    docs = [{"m": {"192.168.0.1": 1, "10.0.0.1": 2}}, {"m": {"10.0.0.1": 3, "host-x": 4}}, {"m": {}}, {"m": 7}]
    store = build_store(docs)
    detection = FormatDetector(store).detect_keys(at(store, "$.m"))
    assert (detection.n_strings, detection.n_distinct) == (4, 3)
    assert counts(detection)["ipv4"] == (3, 1)
    assert detection.check("ipv4").distinct_satisfied == 2


def test_detect_strings_accepts_counts_or_strings() -> None:
    detector = FormatDetector(build_store([]))
    by_count = detector.detect_strings({EXAMPLES["uuid"]: 3, "zz zz": 2})
    by_list = detector.detect_strings([EXAMPLES["uuid"]] * 3 + ["zz zz"] * 2)
    assert by_count == by_list
    assert by_count.check("uuid").satisfied == 3 and by_count.check("uuid").rejected == 2
    with pytest.raises(ValueError):
        detector.detect_strings({"a": 0})
    with pytest.raises(TypeError):
        detector.detect_strings("abc")
    with pytest.raises(TypeError):
        detector.detect_strings([1])  # type: ignore[list-item]


def test_fallback_when_format_detection_is_disabled() -> None:
    store = build_store([{"d": EXAMPLES["date"]}])
    validator = CountingValidator()
    detector = FormatDetector(store, validator)
    view = at(store, "$.d")
    assert format_enabled({}) is True  # implemented, so on by default
    assert format_candidates(detector, view, {TECHNIQUE: False}) == ()
    assert format_candidates(detector, view, {TECHNIQUE: False}, keys=True) == ()
    assert validator.calls == {}  # the Fallback_Behavior invokes no check (13.9)
    found = format_candidates(detector, view, {TECHNIQUE: True})
    assert [check.fmt for check in found] == [c.fmt for c in detector.detect(view).candidates]
    assert "date" in [check.fmt for check in found]


def test_rejection_entry_names_path_candidate_technique_and_count() -> None:
    check = FormatCheck("email", satisfied=2, rejected=3, distinct_satisfied=2, distinct_rejected=1)
    entry = check.rejection(parse_path("$.contact"))
    assert entry == RejectedCandidate(
        REJECTED_CATEGORY, "$.contact", {"format": "email"}, TECHNIQUE, {"rejected": 3, "distinct_rejected": 1}
    )
    with pytest.raises(ValueError):
        FormatCheck("email", 2, 0, 1, 0).rejection("$.contact")


def test_check_and_detection_validation() -> None:
    with pytest.raises(ValueError):
        FormatCheck("color", 1, 0, 1, 0)
    with pytest.raises(ValueError):
        FormatCheck("date", -1, 0, 0, 0)
    with pytest.raises(ValueError):
        FormatCheck("date", 1, 0, 2, 0)
    with pytest.raises(ValueError):
        FormatDetection(1, 1, FormatDetection.empty().checks)
    with pytest.raises(KeyError):
        FormatDetection.empty().check("color")


def test_views_of_another_store_are_refused() -> None:
    detector = FormatDetector(build_store([{"a": "x"}]))
    with pytest.raises(ValueError):
        detector.detect(View.root(build_store([{"a": "x"}])))
    with pytest.raises(TypeError):
        detector.detect("$.a")  # type: ignore[arg-type]


def test_candidates_resolve_through_the_decision_engine() -> None:
    dates = ["2024-01-02", "2023-12-31", "2001-07-04"]
    store = build_store([{"d": d} for d in dates])
    view = at(store, "$.d")
    satisfying = FormatDetector(store).detect(view).satisfying
    assert {"date", "hostname"} <= {check.fmt for check in satisfying}
    candidates = [check.candidate(SchemaNode.from_json({"type": "string", **check.keywords})) for check in satisfying]
    assert [c.order_key for c in candidates] == sorted(c.order_key for c in candidates)
    assert all(c.technique == TECHNIQUE and c.label == f"format={c.outcome['format']}" for c in candidates)
    decision = DecisionEngine().resolve(
        DecisionPoint("string_constraint", view.path), candidates, view, mdl_by=TECHNIQUE
    )
    assert decision.method == "mdl"
    assert decision.candidate in candidates
    # date strings are written with a smaller alphabet than hostnames, so date costs fewer bits.
    assert decision.candidate.outcome == {"format": "date"}
