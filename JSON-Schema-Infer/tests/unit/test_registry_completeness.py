"""Registry completeness (task 3.6; Requirements 28.1, 28.3, 28.8).

``test_registry.py`` checks each registry entry against the design's tables,
and ``test_run_config.py`` checks single examples of the Technique-state
errors. This module checks completeness:

- the registered Techniques are exactly the entries of Table 1, parsed from
  requirements.md itself, so an edit to either side fails here (28.1);
- every served Decision_Point kind has exactly one Fallback_Behavior, and
  calling it calls nothing else, so no Technique is invoked (28.3);
- prerequisite and unimplemented-Technique errors, on the shipped registry
  and on registries derived from it with ``implemented=True`` entries, so the
  checks stay meaningful once Techniques are built (28.8);
- the Learned_Thresholds that each ``TechniqueSpec`` names agree with the
  Techniques that each ``ThresholdSpec`` lists. ``config.registry`` and
  ``config.thresholds`` do not import each other, so only a test can
  cross-check them.
"""

from __future__ import annotations

import dataclasses
import re
import sys
from collections import Counter
from collections.abc import Callable, Mapping
from pathlib import Path
from types import CodeType, FrameType
from typing import Any

import pytest

from jsinfer.config import registry as registry_module
from jsinfer.config import run_config
from jsinfer.config.errors import ConfigError, ConfigProblem
from jsinfer.config.registry import REGISTRY, ConstantOutcome, Registry
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, WEAK_SUPERVISION, thresholds_used_by

REQUIREMENTS = Path(__file__).resolve().parents[2] / ".kiro" / "specs" / "json-schema-inference" / "requirements.md"
TABLE_1_CAPTION = "**Table 1: Minimum Technique set**"
HYBRID = "structure.hybrid_record_map"
MDL = "decide.mdl"
# Labeling_Function thresholds (23.1) are read by the Trainer, not at inference,
# so no TechniqueSpec names them.
LF_PREFIX = "lf."


def problems_of(flags: Any = None, file: Any = None) -> tuple[ConfigProblem, ...]:
    with pytest.raises(ConfigError) as caught:
        RunConfiguration.resolve(flags, file)
    return caught.value.problems


def enable(*tids: str) -> dict[str, bool]:
    return {technique_setting(tid): True for tid in tids}


# ---------------------------------------------------------------------------
# Table 1, parsed from requirements.md
# ---------------------------------------------------------------------------


def _cells(line: str) -> list[str]:
    stripped = line.strip()
    if not (stripped.startswith("|") and stripped.endswith("|")):
        raise ValueError(f"not a table row: {stripped!r}")
    return [cell.strip() for cell in stripped[1:-1].split("|")]


def parse_table_1(text: str) -> list[tuple[str, str]]:
    """The (area, entry) pairs of Table 1 in table order; ``ValueError`` saying what is malformed."""
    lines = text.splitlines()
    captions = [i for i, line in enumerate(lines) if line.strip().startswith(TABLE_1_CAPTION)]
    if len(captions) != 1:
        raise ValueError(f"expected exactly one line starting with {TABLE_1_CAPTION!r}, found {len(captions)}")
    start = captions[0] + 1
    while start < len(lines) and not lines[start].strip():
        start += 1
    table: list[str] = []
    for line in lines[start:]:
        if not line.strip().startswith("|"):
            break
        table.append(line)
    if len(table) < 3:
        raise ValueError(
            f"Table 1 needs a header, a separator, and at least one row; found {len(table)} table lines"
        )
    header, separator, *rows = table
    if _cells(header) != ["Area", "Techniques"]:
        raise ValueError(f"Table 1 header must be '| Area | Techniques |', got {header.strip()!r}")
    if not re.fullmatch(r"\|\s*:?-+:?\s*\|\s*:?-+:?\s*\|", separator.strip()):
        raise ValueError(f"Table 1 separator row is malformed: {separator.strip()!r}")
    entries: list[tuple[str, str]] = []
    for row in rows:
        cells = _cells(row)
        if len(cells) != 2 or not all(cells):
            raise ValueError(f"Table 1 row needs a non-empty area and Technique list: {row.strip()!r}")
        area, techniques = cells
        for entry in techniques.split(";"):
            if not entry.strip():
                raise ValueError(f"Table 1 row {area!r} has an empty semicolon-separated entry")
            entries.append((area, entry.strip()))
    return entries


@pytest.fixture(scope="module")
def table_1() -> list[tuple[str, str]]:
    try:
        text = REQUIREMENTS.read_text("utf-8")
    except OSError as error:
        pytest.fail(f"cannot read Table 1 of Requirement 28 from {REQUIREMENTS}: {error}")
    try:
        return parse_table_1(text)
    except ValueError as error:
        pytest.fail(f"cannot parse Table 1 of Requirement 28 in {REQUIREMENTS}: {error}")


def test_table_1_parser_splits_rows_on_semicolons() -> None:
    text = f"intro\n{TABLE_1_CAPTION} (note)\n\n| Area | Techniques |\n|---|---|\n| A | x; y z |\n| B | w |\n\nafter |"
    assert parse_table_1(text) == [("A", "x"), ("A", "y z"), ("B", "w")]


@pytest.mark.parametrize(
    ("text", "message"),
    [
        ("| Area | Techniques |\n|---|---|\n| A | x |\n", "exactly one line"),
        (f"{TABLE_1_CAPTION}\n\nno table\n", "at least one row"),
        (f"{TABLE_1_CAPTION}\n| Area | Ids |\n|---|---|\n| A | x |\n", "header"),
        (f"{TABLE_1_CAPTION}\n| Area | Techniques |\n| A | x |\n| B | y |\n", "separator"),
        (f"{TABLE_1_CAPTION}\n| Area | Techniques |\n|---|---|\n| A | x;; y |\n", "empty"),
        (f"{TABLE_1_CAPTION}\n| Area | Techniques |\n|---|---|\n| A | x | y |\n", "non-empty area"),
    ],
)
def test_table_1_parser_rejects_malformed_tables(text: str, message: str) -> None:
    with pytest.raises(ValueError, match=message):
        parse_table_1(text)


# ---------------------------------------------------------------------------
# 28.1: one uniquely identified Technique per Table 1 entry
# ---------------------------------------------------------------------------


def test_table_1_has_35_distinct_entries(table_1: list[tuple[str, str]]) -> None:
    assert len(table_1) == 35
    assert len({entry for _, entry in table_1}) == 35, Counter(entry for _, entry in table_1).most_common(3)


def test_registry_has_35_unique_ids() -> None:
    # The source table, before the Registry keys it by id.
    ids = [spec.id for spec in registry_module._TECHNIQUES]
    assert len(ids) == 35
    duplicates = sorted(tid for tid, count in Counter(ids).items() if count > 1)
    assert not duplicates, f"Technique ids registered more than once: {duplicates}"
    assert REGISTRY.ids() == tuple(ids)


def test_registered_techniques_are_exactly_table_1(table_1: list[tuple[str, str]]) -> None:
    registered = [(spec.area, spec.title) for spec in REGISTRY.techniques.values()]
    missing = sorted(set(table_1) - set(registered))
    extra = sorted(set(registered) - set(table_1))
    assert not missing and not extra, f"Table 1 entries with no Technique: {missing}; Techniques not in Table 1: {extra}"
    assert Counter(registered) == Counter(table_1)  # one Technique per entry
    assert registered == table_1  # registration order is Table 1 order


# ---------------------------------------------------------------------------
# 28.3: exactly one Fallback_Behavior per served Decision_Point, invoking nothing
# ---------------------------------------------------------------------------

SERVED_KINDS = tuple(dict.fromkeys(kind for spec in REGISTRY.techniques.values() for kind in spec.serves))


def test_every_served_kind_is_registered_and_every_registered_kind_is_served() -> None:
    unregistered = sorted(set(SERVED_KINDS) - set(REGISTRY.kinds()))
    assert not unregistered, f"kinds served by a Technique but not registered: {unregistered}"
    unserved = sorted(set(REGISTRY.kinds()) - set(SERVED_KINDS))
    assert not unserved, f"registered kinds that no Technique serves: {unserved}"


def _python_calls_during(fallback: ConstantOutcome, view: object) -> tuple[object, list[CodeType]]:
    """The outcome of ``fallback(view)`` and the code of every Python function that ran during the call."""
    calls: list[CodeType] = []

    def profile(frame: FrameType, event: str, arg: object) -> None:
        if event == "call":
            calls.append(frame.f_code)

    sys.setprofile(profile)
    try:
        outcome = fallback(view)  # type: ignore[arg-type]
    finally:
        sys.setprofile(None)
    return outcome, calls


@pytest.mark.parametrize("kind", SERVED_KINDS)
def test_each_served_kind_has_one_constant_fallback_that_calls_nothing(kind: str) -> None:
    entries = [dp for dp in REGISTRY.decision_points.values() if dp.kind == kind]
    assert len(entries) == 1
    fallback = entries[0].fallback
    assert type(fallback) is ConstantOutcome
    # The outcome label is all a Fallback_Behavior holds: no Technique or callable to defer to.
    assert [f.name for f in dataclasses.fields(fallback)] == ["outcome"]
    outcome, calls = _python_calls_during(fallback, object())
    assert outcome == fallback.outcome
    assert calls == [ConstantOutcome.__call__.__code__], [code.co_qualname for code in calls]


# ---------------------------------------------------------------------------
# 28.8: an enabled Technique with a disabled prerequisite
# ---------------------------------------------------------------------------


def derived_registry(
    implemented: Callable[[str], bool] = lambda tid: True,
    *,
    default_enabled: bool = False,
    extra_prerequisites: Mapping[str, tuple[str, ...]] | None = None,
) -> Registry:
    """The shipped registry with other implemented flags, default states, and added prerequisites."""
    extra = extra_prerequisites or {}
    techniques = [
        dataclasses.replace(
            spec,
            implemented=implemented(tid),
            default_enabled=default_enabled and implemented(tid),
            prerequisites=(*spec.prerequisites, *extra.get(tid, ())),
        )
        for tid, spec in REGISTRY.techniques.items()
    ]
    return Registry(techniques, REGISTRY.decision_points.values())


@pytest.fixture
def install(monkeypatch: pytest.MonkeyPatch) -> Callable[[Registry], None]:
    """Make ``RunConfiguration`` resolve against another registry with the same ids."""

    def install(registry: Registry) -> None:
        assert registry.ids() == REGISTRY.ids()  # SETTING_NAMES is built from the shipped ids
        monkeypatch.setattr(run_config, "REGISTRY", registry)

    return install


def assert_prerequisite_problems(problems: tuple[ConfigProblem, ...], pairs: list[tuple[str, str]]) -> None:
    """The prerequisite problems are one per (Technique, disabled prerequisite) pair, in ``pairs`` order."""
    found = [p for p in problems if p.reason == "prerequisite"]
    assert [p.subject for p in found] == [tid for tid, _ in pairs], [p.message for p in found]
    for problem, (tid, prerequisite) in zip(found, pairs, strict=True):
        assert repr(tid) in problem.message and repr(prerequisite) in problem.message, problem.message


WITH_PREREQUISITES = [tid for tid, spec in REGISTRY.techniques.items() if spec.prerequisites]


@pytest.mark.parametrize("tid", WITH_PREREQUISITES)
def test_shipped_registry_names_every_disabled_prerequisite(tid: str) -> None:
    prerequisites = REGISTRY.technique(tid).prerequisites
    flags = {**enable(tid), **{technique_setting(p): False for p in prerequisites}}
    assert_prerequisite_problems(problems_of(flags), [(tid, p) for p in sorted(prerequisites)])


def test_prerequisite_check_holds_for_implemented_techniques(install: Callable[[Registry], None]) -> None:
    install(derived_registry())
    assert REGISTRY.technique(HYBRID).prerequisites == (MDL,)
    assert not any(RunConfiguration.resolve().techniques.values())  # disabled Techniques need nothing
    problems = problems_of(enable(HYBRID))
    assert [(p.subject, p.reason) for p in problems] == [(HYBRID, "prerequisite")]
    assert_prerequisite_problems(problems, [(HYBRID, MDL)])
    config = RunConfiguration.resolve(enable(HYBRID, MDL))
    assert {tid for tid, on in config.techniques.items() if on} == {HYBRID, MDL}


@pytest.mark.parametrize(
    ("flags", "file", "word"),
    [
        ({technique_setting(MDL): False}, None, "flag"),
        (None, {"techniques": {MDL: False}}, "configuration file"),
    ],
)
def test_disabling_a_prerequisite_that_is_on_by_default(
    install: Callable[[Registry], None], flags: Any, file: Any, word: str
) -> None:
    # Once both are built, both are on by default, and switching decide.mdl off alone is an error.
    install(derived_registry(default_enabled=True))
    assert all(RunConfiguration.resolve().techniques.values())
    problems = problems_of(flags, file)
    assert [(p.subject, p.reason) for p in problems] == [(HYBRID, "prerequisite")]
    message = problems[0].message
    assert f"{HYBRID!r} is enabled (default)" in message and f"{MDL!r} is disabled ({word})" in message
    # Switching both off is valid, and leaves every other Technique on.
    config = RunConfiguration.resolve({technique_setting(HYBRID): False}, {"techniques": {MDL: False}})
    assert {tid for tid, on in config.techniques.items() if not on} == {HYBRID, MDL}


def test_one_error_names_every_pair_with_a_disabled_prerequisite(install: Callable[[Registry], None]) -> None:
    extra = {"rules.fd_ind": ("rules.keys",), "values.pattern": ("values.format", "values.latent_type")}
    install(derived_registry(extra_prerequisites=extra))
    enabled = enable(HYBRID, "rules.fd_ind", "values.pattern")
    problems = problems_of(enabled)
    assert {p.reason for p in problems} == {"prerequisite"}
    assert_prerequisite_problems(
        problems,
        [
            (HYBRID, MDL),
            ("rules.fd_ind", "rules.keys"),
            ("values.pattern", "values.format"),
            ("values.pattern", "values.latent_type"),
        ],
    )
    # Enabling some prerequisites, from either source, leaves exactly the other pairs.
    problems = problems_of({**enabled, **enable(MDL)}, {"techniques": {"values.format": True}})
    assert_prerequisite_problems(problems, [("rules.fd_ind", "rules.keys"), ("values.pattern", "values.latent_type")])


# ---------------------------------------------------------------------------
# Enabling an unimplemented Technique
# ---------------------------------------------------------------------------

UNIMPLEMENTED = [tid for tid, spec in REGISTRY.techniques.items() if not spec.implemented]


@pytest.mark.parametrize("tid", UNIMPLEMENTED)
def test_enabling_any_unimplemented_technique_is_an_error_naming_it(tid: str) -> None:
    found = [p for p in problems_of(enable(tid)) if p.reason == "unimplemented"]
    assert [p.subject for p in found] == [tid]
    assert repr(tid) in found[0].message


def test_every_enabled_unimplemented_technique_is_reported_in_one_error() -> None:
    half = len(UNIMPLEMENTED) // 2
    flags = enable(*UNIMPLEMENTED[:half])
    file = {"techniques": dict.fromkeys(UNIMPLEMENTED[half:], True)}
    problems = problems_of(flags, file)
    assert [(p.subject, p.reason) for p in problems] == [(tid, "unimplemented") for tid in UNIMPLEMENTED]


def test_unimplemented_errors_follow_the_implemented_flag(install: Callable[[Registry], None]) -> None:
    ids = REGISTRY.ids()
    built = set(ids[::2])
    built |= {p for tid in built for p in REGISTRY.technique(tid).prerequisites}
    assert built and built != set(ids)
    install(derived_registry(lambda tid: tid in built))
    problems = problems_of(enable(*ids))
    assert [(p.subject, p.reason) for p in problems] == [(tid, "unimplemented") for tid in ids if tid not in built]
    config = RunConfiguration.resolve(enable(*sorted(built)))
    assert {tid for tid, on in config.techniques.items() if on} == built


# ---------------------------------------------------------------------------
# Learned_Threshold cross-references between config.registry and config.thresholds
# ---------------------------------------------------------------------------


def test_technique_thresholds_match_threshold_spec_techniques() -> None:
    named_by_registry = {(tid, name) for tid, spec in REGISTRY.techniques.items() for name in spec.thresholds}
    listed_by_specs = {
        (tid, name) for name, spec in THRESHOLDS.items() if not name.startswith(LF_PREFIX) for tid in spec.techniques
    }
    unregistered = sorted(name for _, name in named_by_registry if name not in THRESHOLDS)
    assert not unregistered, f"TechniqueSpec.thresholds names unregistered Learned_Thresholds: {unregistered}"
    assert sorted(named_by_registry - listed_by_specs) == [], "TechniqueSpec reads it; its ThresholdSpec omits it"
    assert sorted(listed_by_specs - named_by_registry) == [], "ThresholdSpec lists it; its TechniqueSpec omits it"
    # What the Model_Bundle reader requires per Technique is then what the registry declares.
    for tid, spec in REGISTRY.techniques.items():
        required = {s.name for s in thresholds_used_by([tid]) if not s.name.startswith(LF_PREFIX)}
        assert required == set(spec.thresholds), tid


def test_threshold_specs_name_registered_techniques_at_a_kind_they_serve() -> None:
    assert WEAK_SUPERVISION in REGISTRY.techniques
    for name, spec in THRESHOLDS.items():
        unregistered = [tid for tid in spec.techniques if tid not in REGISTRY.techniques]
        assert not unregistered, f"{name} lists unregistered Techniques {unregistered}"
        if name.startswith(LF_PREFIX):
            assert spec.techniques == (WEAK_SUPERVISION,), name
            continue
        assert spec.decision_point in REGISTRY.decision_points, name
        not_serving = [tid for tid in spec.techniques if spec.decision_point not in REGISTRY.technique(tid).serves]
        assert not not_serving, f"{name} applies at {spec.decision_point!r}, which {not_serving} do not serve"
