"""Unit tests for values/inferrer.py: ValueInferrer orchestration (task 12.5).

Requirements 4.4 (Map-key Formats under ``propertyNames``), 13.6 (withheld
candidates with rejected counts in the report), 13.8 (MDL selection with the
alternatives in provenance), and 13.9 (disabled Techniques produce nothing),
plus the design order of the scalar steps and the provenance of every
emitted keyword.
"""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.core.observations import build_store
from jsinfer.core.paths import parse_path, render_path
from jsinfer.core.provenance import laplace_confidence
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId
from jsinfer.core.views import View
from jsinfer.decide.engine import DecisionEngine
from jsinfer.io.validator import Validator
from jsinfer.values.bounds import ITEM_COUNT, decide_bound
from jsinfer.values.inferrer import (
    CORPUS_TECHNIQUES,
    FORMAT,
    LATENT_TYPE,
    PATTERN,
    SEMANTIC_TYPE,
    STRING_CONSTRAINT,
    TYPE_PRODUCER,
    KeyConstraint,
    StringSet,
    ValueInferrer,
)

VALIDATOR = Validator()
UUIDS = [f"123e4567-e89b-12d3-a456-42661417400{i}" for i in range(4)]
DATES = ["2024-01-02", "2023-12-31", "2024-01-02", "1999-02-28"]


def at(store: Any, text: str = "$") -> View:
    return View.root(store).descend(parse_path(text))


def inferrer(states: dict[str, bool] | None = None) -> tuple[ValueInferrer, InferenceReport]:
    report = InferenceReport()
    return ValueInferrer.for_techniques(states, report=report, validator=VALIDATOR), report


def infer(docs: list[Any], path: str = "$", states: dict[str, bool] | None = None) -> tuple[Any, InferenceReport]:
    vi, report = inferrer(states)
    return vi.infer_scalar(at(build_store(docs), path)), report


def decisions(report: InferenceReport, kind: str) -> list[Any]:
    return [entry for entry in report.decisions if entry.kind == kind]


def rejected(report: InferenceReport, category: str = "formats") -> list[Any]:
    return [entry for entry in report.rejected if entry.category == category]


# ---------------------------------------------------------------------------
# type (13.1, 13.2)
# ---------------------------------------------------------------------------


def test_type_uses_the_integer_rule_and_the_laplace_default() -> None:
    result, _ = infer([1, 2.0, 1e3, None, True])
    assert result.keywords()["type"] == ["null", "boolean", "integer"]
    record = result.provenance["type"]
    assert record.technique == TYPE_PRODUCER == "baseline:type"
    assert record.fallback is None and record.resolution is None  # a Baseline_Rule resolves no Decision_Point
    assert (record.support.consistent, record.support.total) == (5, 5)
    assert record.confidence == laplace_confidence(record.support)


def test_a_view_without_scalars_emits_nothing() -> None:
    result, report = infer([{"a": 1}, [1, 2]])
    assert result.keywords() == {} and dict(result.provenance) == {}
    assert result.string is None and result.number_bounds is None and result.length_bounds is None
    assert report.decisions == ()


# ---------------------------------------------------------------------------
# String candidates and selection (13.3, 13.4, 13.8)
# ---------------------------------------------------------------------------


def test_dates_select_the_latent_format_and_list_the_alternatives() -> None:
    result, report = infer(DATES)
    kw = result.keywords()
    assert kw["format"] == "date" and kw["x-latent-type"] == "date"
    string = result.string
    assert string.selected.technique == LATENT_TYPE
    # Format detection offered the same Constraint; it is merged, not a competing alternative.
    assert [c.technique for c in string.duplicates()] == [FORMAT]
    record = result.provenance["format"]
    assert record.technique == LATENT_TYPE and record.resolution.method == "mdl"
    assert record.details["also_offered_by"] == [FORMAT]
    others = [c for i, c in enumerate(record.resolution.candidates) if i != record.resolution.selected]
    assert record.details["alternatives"] == [{"outcome": c.outcome, "technique": c.technique} for c in others]
    assert (record.support.consistent, record.support.total) == (4, 4)
    # A selected Format skips the length bounds (16.3); the annotation has its own record.
    assert result.length_bounds.skipped == "format" and "minLength" not in kw and "maxLength" not in kw
    assert result.provenance["x-latent-type"].technique == LATENT_TYPE
    assert [entry.resolution.method for entry in decisions(report, STRING_CONSTRAINT)] == ["mdl"]


def test_mdl_picks_the_shortest_survivor() -> None:
    result, _ = infer(["192.168.0.1", "10.0.0.1", "8.8.8.8", "10.0.0.1"])
    string = result.string
    assert len(string.resolved) >= 2  # ipv4 strings satisfy more than one Format
    resolution = result.provenance[string.keyword].resolution
    scores = [c.score for c in resolution.candidates]
    assert scores[resolution.selected] == min(scores)
    assert [c.outcome for c in resolution.candidates] == [c.label for c in string.resolved]
    assert all(VALIDATOR.satisfies_format(string.value, s) for s in string.strings.strings)


def test_selection_is_by_mdl_whatever_the_state_of_decide_mdl() -> None:
    for state in (False, True):  # the engine takes raw states, implemented or not
        result, _ = infer(["192.168.0.1", "10.0.0.1"], states={"decide.mdl": state})
        assert result.string.decision.method == "mdl"
        assert result.string.decision.fallback is None


def test_latent_pattern_is_emitted_when_no_format_competes() -> None:
    strings = ["2024-3-15", "2023-12-1", "2024-3-15"]
    result, _ = infer(strings, states={FORMAT: False})
    kw = result.keywords()
    assert kw["x-latent-type"] == "date"
    assert result.string.selected.technique == LATENT_TYPE and "pattern" in kw
    assert all(VALIDATOR.matches_pattern(kw["pattern"], s) for s in strings)
    details = result.provenance["pattern"].details
    assert details["latent_type"] == "date" and details["format_rejects"] == 3
    # A Pattern is not a Format, so the length bounds are decided (16.3).
    assert result.length_bounds.skipped is None


# ---------------------------------------------------------------------------
# Withholding (13.6)
# ---------------------------------------------------------------------------


def test_rejecting_candidates_are_withheld_with_counts() -> None:
    strings = ["a@example.com", "b@example.com", "nope!", "nope!"]
    result, report = infer(strings, states={PATTERN: False})  # induction (14) would offer a Pattern here
    kw = result.keywords()
    assert "format" not in kw and "pattern" not in kw
    assert kw["type"] == ["string"]  # the other Constraints stay
    assert result.string.selected is None and result.string.decision is None
    entries = {entry.candidate["format"]: entry for entry in rejected(report)}
    assert set(entries) == {c.value for c in result.string.candidates}
    for fmt, entry in entries.items():
        bad = [s for s in strings if not VALIDATOR.satisfies_format(fmt, s)]
        assert entry.path == "$" and entry.technique == FORMAT
        assert entry.details == {"rejected": len(bad), "distinct_rejected": len(set(bad))}
    assert decisions(report, STRING_CONSTRAINT) == []  # Format detection ran and had nothing to offer


def test_string_set_counts_rejections_with_multiplicity() -> None:
    strings = StringSet(("2024-01-02", "x"), (3, 2))
    assert strings.total == 5 and strings.n_distinct == 2
    assert strings.rejections("format", "date", VALIDATOR) == (2, 1)
    assert strings.rejections("pattern", "^(?:[0-9-]+)$(?!\\n)", VALIDATOR) == (2, 1)


# ---------------------------------------------------------------------------
# Technique states (13.9)
# ---------------------------------------------------------------------------


def test_disabled_string_techniques_give_the_fallback() -> None:
    result, report = infer(DATES, states={LATENT_TYPE: False, FORMAT: False, PATTERN: False})
    kw = result.keywords()
    assert "x-latent-type" not in kw and "format" not in kw and "pattern" not in kw
    assert result.latent is None and result.string.candidates == ()
    assert result.string.decision.method == "fallback" and result.string.selected is None
    (entry,) = decisions(report, STRING_CONSTRAINT)
    assert entry.resolution.method == "fallback" and entry.fallback is not None
    # Type, enum, and bounds still apply (13.9).
    assert kw["type"] == ["string"] and result.length_bounds.skipped is None


def test_disabling_latent_types_keeps_format_detection() -> None:
    result, _ = infer(DATES, states={LATENT_TYPE: False})
    kw = result.keywords()
    assert "x-latent-type" not in kw and kw["format"] == "date"
    assert result.provenance["format"].technique == FORMAT
    assert all(c.technique == FORMAT for c in result.string.candidates)


def test_disabling_format_detection_keeps_latent_types() -> None:
    result, _ = infer(DATES, states={FORMAT: False})
    assert result.keywords()["format"] == "date" and result.keywords()["x-latent-type"] == "date"
    assert all(c.technique == LATENT_TYPE for c in result.string.candidates)


UNIMPLEMENTED = [tid for tid in (SEMANTIC_TYPE, *CORPUS_TECHNIQUES, PATTERN) if not REGISTRY.technique(tid).implemented]


@pytest.mark.parametrize("tid", UNIMPLEMENTED)
def test_unimplemented_techniques_are_skipped_by_registry_state(tid: str) -> None:
    store = build_store([*DATES, "2024-3-15"])
    baseline, _ = inferrer()
    enabled, _ = inferrer({tid: True})  # the engine takes raw states; the registry still says unimplemented
    assert not enabled.runs(tid) and enabled.engine.enabled(tid)
    expected = baseline.infer_scalar(at(store))
    got = enabled.infer_scalar(at(store))
    assert got.keywords() == expected.keywords()
    assert all(c.technique != tid for c in got.string.candidates)


# ---------------------------------------------------------------------------
# Enums and bounds (15, 16)
# ---------------------------------------------------------------------------


def test_closed_enum_emits_enum_and_skips_value_bounds() -> None:
    result, _ = infer(["red", "blue"] * 20)
    kw = result.keywords()
    assert kw["enum"] == ["blue", "red"]
    record = result.provenance["enum"]
    assert record.technique == "values.enum_estimation" and record.details["outcome"] == "closed"
    assert result.length_bounds.skipped == "closed-enum" and "minLength" not in kw


def test_emitted_bounds_hold_for_every_value_and_carry_provenance() -> None:
    numbers = [*range(3, 140, 7), 0.5]
    strings = ["ab cd", "abc def", "a b"]
    result, report = infer([*numbers, *strings])  # every value once: open (15.5)
    assert not result.enum.closed
    kw = result.keywords()
    assert kw["type"] == ["number", "string"]
    for keyword in ("minimum", "maximum", "minLength", "maxLength"):
        if keyword in kw:
            assert result.provenance[keyword].technique == "values.bounds"
    assert all(kw.get("minimum", n) <= n <= kw.get("maximum", n) for n in numbers)
    assert all(kw.get("minLength", 0) <= len(s) <= kw.get("maxLength", len(s)) for s in strings)
    assert len(decisions(report, "bound")) == 4  # minimum, maximum, minLength, maxLength


def test_decide_bound_delegates_to_the_bound_ladder() -> None:
    vi, _ = inferrer()
    ours = vi.decide_bound([0, 5, 9, 9], ITEM_COUNT, path="$.a")
    theirs = decide_bound([0, 5, 9, 9], ITEM_COUNT, DecisionEngine(), path="$.a")
    assert ours.keywords() == theirs.keywords()


# ---------------------------------------------------------------------------
# Map keys (4.4)
# ---------------------------------------------------------------------------


def test_key_constraint_emits_a_format_under_property_names() -> None:
    docs = [{"m": {UUIDS[0]: 1, UUIDS[1]: 2}}, {"m": {UUIDS[1]: 3, UUIDS[2]: 4}}, {"m": {}}]
    vi, report = inferrer()
    found = vi.key_constraint(at(build_store(docs), "$.m"))
    assert isinstance(found, KeyConstraint)
    assert found.candidate.value == "uuid" and found.candidate.technique == FORMAT
    names = found.keywords()["propertyNames"]
    assert names.to_json() == {"format": "uuid"}
    assert all(VALIDATOR.satisfies_format("uuid", key) for d in docs for key in d["m"])
    record = found.provenance()
    assert record.technique == FORMAT and record.details["target"] == "keys"
    assert (record.support.consistent, record.support.total) == (4, 4)  # keys once per object
    (entry,) = decisions(report, STRING_CONSTRAINT)
    assert entry.path == "$.m" and entry.resolution.method == "mdl"


def test_key_formats_that_reject_a_key_are_withheld() -> None:
    docs = [{UUIDS[0]: 1, "not a key!": 2}, {UUIDS[1]: 3}]
    vi, report = inferrer({PATTERN: False})  # induction (14) would offer a key Pattern here
    assert vi.key_constraint(at(build_store(docs))) is None
    entries = rejected(report)
    assert entries and all(entry.details["target"] == "keys" for entry in entries)
    uuid = next(entry for entry in entries if entry.candidate == {"format": "uuid"})
    assert (uuid.details["rejected"], uuid.details["distinct_rejected"]) == (1, 1)


def test_key_constraint_with_format_disabled_records_the_fallback() -> None:
    vi, report = inferrer({FORMAT: False, LATENT_TYPE: False, PATTERN: False})
    assert vi.key_constraint(at(build_store([{UUIDS[0]: 1}]))) is None
    (entry,) = decisions(report, STRING_CONSTRAINT)
    assert entry.resolution.method == "fallback"


def test_key_constraint_without_keys_is_none() -> None:
    vi, report = inferrer()
    assert vi.key_constraint(at(build_store([{}, {}, 3]))) is None
    assert report.decisions == () and report.rejected == ()


def test_key_constraint_rejects_a_base_with_property_names() -> None:
    vi, _ = inferrer()
    with pytest.raises(ValueError):
        vi.key_constraint(at(build_store([{UUIDS[0]: 1}])), base={"propertyNames": {}})


# ---------------------------------------------------------------------------
# Results and determinism
# ---------------------------------------------------------------------------


def test_constraint_provenance_is_keyed_by_the_node_nid() -> None:
    result, _ = infer(DATES)
    keyed = result.constraint_provenance()
    assert set(keyed) == {ConstraintId(result.node.nid, keyword) for keyword in result.keywords()}
    assert all(keyed[ConstraintId(result.node.nid, k)] is r for k, r in result.provenance.items())


def test_results_do_not_depend_on_document_order() -> None:
    docs = [{"v": v} for v in [*DATES, "2024-07-01", 5, 12, 5, None]]
    first, report_a = infer(docs, "$.v")
    second, report_b = infer(list(reversed(docs)), "$.v")
    assert first.keywords() == second.keywords()
    assert {k: r.to_json() for k, r in first.provenance.items()} == {
        k: r.to_json() for k, r in second.provenance.items()
    }
    assert report_a.to_json()["decisions"] == report_b.to_json()["decisions"]
    assert report_a.to_json()["rejected"] == report_b.to_json()["rejected"]


def test_flagged_rows_are_left_out() -> None:
    store = build_store([{"d": "2024-01-02"}, {"d": "no pe!"}, {"d": "2024-01-03"}])
    vi, _ = inferrer()
    assert "format" not in vi.infer_scalar(at(store, "$.d")).keywords()
    store.flags.flag_documents([1])
    result = vi.infer_scalar(at(store, "$.d"))
    assert result.keywords()["format"] == "date" and render_path(result.path) == "$.d"


def test_rejects_wrong_argument_types() -> None:
    with pytest.raises(TypeError):
        ValueInferrer({})  # type: ignore[arg-type]
    vi, _ = inferrer()
    with pytest.raises(TypeError):
        vi.infer_scalar([1, 2])  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        vi.key_constraint("$")  # type: ignore[arg-type]
