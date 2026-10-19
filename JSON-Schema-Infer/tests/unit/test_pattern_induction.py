"""Unit tests of pattern induction and its limits (task 32.2; Requirements 4.4, 14.1–14.3, 14.6–14.8)."""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.core.observations import build_store
from jsinfer.core.paths import parse_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.values.inferrer import LATENT_TYPE, PATTERN, PATTERN_CATEGORY, ValueInferrer
from jsinfer.values.patterns import LADDER, Induction, Pattern, alternative_at, merge_alternatives, profiles, tokenize

VALIDATOR = Validator()
CODES = [f"AB_{1000 + i}" for i in range(30)] + [f"cd_{i:02d}" for i in range(10)]  # underscores: no Format matches


def infer(docs: list[Any], path: str = "$", states: dict[str, bool] | None = None, resources: dict[str, Any] | None = None, seed: int = 0):
    report = InferenceReport()
    vi = ValueInferrer.for_techniques(states, report=report, validator=VALIDATOR, resources=resources, seed=seed)
    return vi.infer_scalar(View.root(build_store(docs)).descend(parse_path(path))), report


def rejected(report: InferenceReport) -> list[Any]:
    return [entry for entry in report.rejected if entry.category == PATTERN_CATEGORY]


# -- induction -----------------------------------------------------------------


def test_tokenize_and_profiles() -> None:
    assert [t.source for t in tokenize("AB-1234")] == ["[A-Z]{2}", "-", "[0-9]{4}"]
    assert [t.source for t in tokenize("--x9")] == ["(?:-){2}", "[a-z]", "[0-9]"]
    found = profiles(["AB-12", "CD-345", "ab"], [3, 1, 2])
    assert [p.atoms for p in found][0][0].value == "[A-Z]" and len(found) == 2
    assert found[0].runs == ((2, 1, 2), (2, 1, 3)) and found[0].count == 4 and found[1].count == 2
    assert found[0].low(2) == 2 and found[0].high(2) == 3


def test_ladder_rungs_increase_generality() -> None:
    [profile] = profiles(["AB-12", "CD-345"])
    sources = [Pattern([alternative_at(profile, rung)]).source for rung in LADDER]
    assert sources[0] == sources[1] == "^(?:[A-Z]{2}-[0-9]{2,3})$(?!\\n)"
    assert sources[2] == "^(?:[A-Za-z]{2}-[0-9A-Za-z]{2,3})$(?!\\n)"
    assert sources[3] == "^(?:[A-Za-z]+-[0-9A-Za-z]+)$(?!\\n)"
    [exact] = profiles(["AB-12", "CD-34"])
    assert Pattern([alternative_at(exact, "exact")]).source == "^(?:[A-Z]{2}-[0-9]{2})$(?!\\n)"


def test_merge_alternatives_joins_classes_and_unions_quantifiers() -> None:
    [a] = profiles(["AB-12"])
    [b] = profiles(["cd-3456"])
    merged = merge_alternatives(alternative_at(a, "range"), alternative_at(b, "range"))
    assert merged is not None and Pattern([merged]).source == "^(?:[A-Za-z]{2}-[0-9]{2,4})$(?!\\n)"
    [c] = profiles(["x"])
    assert merge_alternatives(alternative_at(a, "range"), alternative_at(c, "range")) is None  # token counts differ
    [d] = profiles(["AB+12"])
    assert merge_alternatives(alternative_at(a, "range"), alternative_at(d, "range")) is None  # literals differ


def test_candidates_are_sound_distinct_and_capped() -> None:
    strings = CODES + ["x1", "y22"]
    assert list(Induction(strings, max_alternatives=1).candidates()) == []  # "AB-1234" and "x1" cannot align
    for cap in (2, 4):
        found = list(Induction(strings, max_alternatives=cap).candidates())
        assert found, cap
        assert len({c.pattern.source for c in found}) == len(found)
        for candidate in found:
            assert candidate.sub_patterns <= cap
            assert all(VALIDATOR.matches_pattern(candidate.pattern.source, s) for s in strings), candidate.pattern.source
            assert candidate.rung in LADDER
    assert [c.rung for c in Induction(["AB-12", "CD-34"]).candidates()] == ["exact", "widen", "plus"]  # range equals exact
    assert list(Induction([]).candidates()) == []
    with pytest.raises(ValueError, match="max_alternatives"):
        Induction(["a"], max_alternatives=0)


def test_unmergeable_profiles_yield_no_candidate_at_the_cap() -> None:
    found = list(Induction(["AB-12", "x", "1-2-3"], max_alternatives=1).candidates())
    assert found == []  # three atom sequences of different lengths cannot become one alternative
    assert [c.sub_patterns for c in Induction(["AB-12", "x", "1-2-3"], max_alternatives=3).candidates()] == [3, 3, 3]


# -- the Value_Inferrer step (14.1–14.3, 14.6–14.8, 4.4) ------------------------------


def test_induced_pattern_is_emitted_and_sound() -> None:
    docs = [{"code": c} for c in CODES]
    result, report = infer(docs, "$.code")
    kw = result.node.kw
    assert "pattern" in kw and "format" not in kw
    assert all(VALIDATOR.matches_pattern(kw["pattern"], c) for c in CODES)
    assert result.string.selected.technique == PATTERN
    assert kw["pattern"].startswith("^(?:") and kw["pattern"].endswith(")$(?!\\n)")
    record = result.provenance["pattern"]
    assert record.technique == PATTERN and record.details["sub_patterns"] >= 1 and record.sample is None
    assert rejected(report) == []


def test_disabled_induction_leaves_the_latent_pattern_unchanged() -> None:
    docs = [{"ip": f"10.0.{i}.{i}"} for i in range(20)]
    on, _ = infer(docs, "$.ip")
    off, report = infer(docs, "$.ip", {PATTERN: False})
    assert "pattern" not in off.node.kw or off.string.selected.technique == LATENT_TYPE or off.node.kw.get("format")
    assert rejected(report) == []
    assert on.node.kw.get("format") == off.node.kw.get("format")


def test_attempt_budget_stops_induction_and_is_reported() -> None:
    docs = [{"code": c} for c in CODES]
    result, report = infer(docs, "$.code", resources={"patterns.attempt_budget": 1})
    evaluated = [c for c in result.string.candidates if c.technique == PATTERN]
    assert len(evaluated) == 1
    [entry] = rejected(report)
    assert entry.path == "$.code" and entry.technique == PATTERN
    assert entry.details["budget_exhausted"] is True and entry.details["evaluated"] == 1
    assert "pattern" in result.node.kw  # the selection of 14.2 still runs over the candidates evaluated so far


def test_alternatives_cap_bounds_every_induced_candidate() -> None:
    docs = [{"code": c} for c in CODES + ["x1y", "q2z", "9k"]]  # four atom sequences; two of them merge
    result, _ = infer(docs, "$.code", resources={"patterns.max_alternatives": 3})
    induced = [c for c in result.string.candidates if c.technique == PATTERN]
    assert induced and all(c.details["sub_patterns"] <= 3 for c in induced)
    assert any(c.details["merged_profiles"] == 1 for c in induced)
    wide, _ = infer(docs, "$.code", resources={"patterns.max_alternatives": 8})
    assert max(c.details["sub_patterns"] for c in wide.string.candidates if c.technique == PATTERN) == 4


def test_sampled_candidates_that_reject_strings_are_discarded_and_reported() -> None:
    # Two strings are sampled for induction; the exact-length candidate they give rejects the rest (14.7).
    docs = [{"id": f"K_{i:03d}"} for i in range(20)] + [{"id": "longer_tail_99"}, {"id": "Z"}]
    result, report = infer(docs, "$.id", resources={"sample.patterns": 2}, seed=3)
    entries = rejected(report)
    assert entries, "a candidate induced from the sample should reject an unsampled string"
    for entry in entries:
        assert entry.path == "$.id" and entry.technique == PATTERN
        assert entry.details["rejected"] >= 1 and entry.details["distinct_rejected"] >= 1
        assert "pattern" in entry.candidate
    if "pattern" in result.node.kw:
        assert all(VALIDATOR.matches_pattern(result.node.kw["pattern"], d["id"]) for d in docs)
        assert result.provenance["pattern"].sample is not None and result.provenance["pattern"].sample.sampled == 2
    again, _ = infer(docs, "$.id", resources={"sample.patterns": 2}, seed=3)
    assert again.node.kw == result.node.kw  # the sample is seeded


def test_map_keys_get_an_induced_property_names_pattern() -> None:
    docs = [{"m": {f"id_{i:04d}": i, f"id_{i + 1:04d}": i}} for i in range(25)]
    report = InferenceReport()
    vi = ValueInferrer.for_techniques(report=report, validator=VALIDATOR)
    constraint = vi.key_constraint(View.root(build_store(docs)).descend(parse_path("$.m")))
    assert constraint is not None
    sub = constraint.subschema().kw
    assert "pattern" in sub and all(VALIDATOR.matches_pattern(sub["pattern"], k) for d in docs for k in d["m"])
    assert constraint.provenance().details.get("target") == "keys"
