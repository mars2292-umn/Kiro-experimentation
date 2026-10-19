"""Property 48: Technique isolation (task 19.6).

For any subset of Technique toggles that respects prerequisites:

- the effective state of each other Technique is unchanged;
- no Provenance_Record names a disabled Technique;
- every Decision_Point whose serving Techniques are all disabled is resolved
  by its Fallback_Behavior;
- each such resolution records the Decision_Point, the fallback indicator,
  and the reason in the Provenance_Record.

(Requirements 26.10, 28.2, 28.4, 28.11, 28.12.)

Runs
    Each example is a full in-process ``pipeline.infer.infer_documents`` run
    with ``workers = 1`` and small Resource_Parameters, on a generated
    collection. The toggle scenario is a test parameter, so each gets the
    profile's examples: toggles drawn by ``tests.strategies.setting_flags``
    (any subset of the toggles, each implemented Technique on or off,
    unimplemented ones left out or off, the only state the
    Run_Configuration accepts for them, and prerequisites repaired); the
    same with ``decide.mdl`` off (priority order, and the Fallback_Behaviors
    of the Judgment_Calls and of ``relaxation``); every Technique off; and,
    for each implemented Technique, that Technique alone off and that
    Technique alone on. The Model_Bundle is the shipped empty default or one
    of two stub bundles of hand-set Learned_Thresholds, one complete and one
    partial, written once per module, so that grouping and co-occurrence
    Techniques are in effect in some runs and enabled but missing their
    Learned_Thresholds in others.

    To reach the ``relaxation`` Decision_Point (3.3, served by ``decide.mdl``
    alone), ``StructureAnalyzer.infer_model`` is wrapped (``unittest.mock``)
    and, in about half the runs, one ``type`` Constraint of its model is
    narrowed, keeping its ``baseline:type`` record, so the soundness check
    relaxes it.

Collections
    A random template (Records with optional fields, Maps with identifier
    keys, Lists, Tuples, mixed types, and Variants with disjoint fields)
    instantiated 2 to 12 times, or 14 to 20 times so that Tuples, Maps, and
    split Variants can win under MDL, plus up to one arbitrary value. Scalars
    are free values, dates, or small categories that repeat.

Interpretation
    - A Technique is *in effect* at a Decision_Point kind when it is enabled,
      its serving gate (15.7) is enabled, and the Model_Bundle holds every
      Learned_Threshold it reads at that kind (20.6; design, Open Decisions,
      "Co-occurrence thresholds under MDL"). The property's "disabled" is
      checked exactly as stated (state off), and the same checks are also
      applied to Techniques that are enabled but not in effect, which the
      DecisionEngine treats alike: they produce no candidates, and a
      Decision_Point none of whose servers is in effect takes its
      Fallback_Behavior, with reason ``missing-threshold`` (or
      ``missing-model``) instead of ``disabled``.
    - "Names a Technique" means any registered Technique id anywhere in the
      record's JSON form (producer, candidates of the resolution, cut-off
      scores, relaxation resolutions under ``changes``, and ``details``),
      except inside a fallback indicator: 26.10 requires the indicator to
      name the Technique whose Fallback_Behavior was applied, which is
      disabled or not in effect by definition. That covers the record's own
      ``fallback`` member, including the Decision_Model fallback of a
      Judgment_Call (22.13) and the priority fallback of a disabled
      ``decide.mdl`` (19.7), and the ``fallback`` member of a decision
      summary embedded in ``details`` (a Merge-or-Split record carries its
      ``variant_grouping`` decision, whose priority resolution names the
      disabled ``decide.mdl`` there). Every indicator's reason is checked
      against the run.
    - Baseline_Rule records (``baseline:type``, ``baseline:required``) name
      no Technique, resolve no Decision_Point, and are present whatever the
      toggles; they are only checked to hold no fallback indicator and no
      resolution (26.13).
    - Decision_Points are observed through the Inference_Report's decision
      entries (every DecisionEngine resolution is recorded there, 20.7),
      because a Fallback_Behavior whose outcome is "none" produces no
      Constraint and so no Provenance_Record. ``document_outlier`` and
      ``version_split`` are resolved once at ``$`` in every run, since their
      Techniques are not implemented yet (design, Open Decisions, "Pipeline
      stages and run-level hooks").

Checks
    - 28.2: the Run_Configuration's state of every Technique is its toggle,
      else its registered default, and the run (its configuration, the
      report's ``run`` section, and the DecisionEngine built for it) uses
      those states. Setting any one more toggle (to the other state, where
      that is valid, or to its current state) changes no other Technique's
      effective state.
    - 28.11: no Provenance_Record of the explanation file (which holds
      exactly the model's records) names a disabled Technique (nor one not
      in effect), and every record's producer is a registered, implemented,
      enabled Technique, a ``fallback:<kind>`` marker, or a Baseline_Rule
      marker. No decision entry of the report names a disabled Technique or
      lists a candidate of a Technique that is not in effect at its kind.
    - 28.4: every decision entry whose kind's servers are all disabled (or
      all out of effect) is a Fallback_Behavior resolution, which selects
      the kind's Fallback_Behavior outcome (the first candidate for the
      "documented candidate order" of ``relaxation``) and lists no candidate
      of any Technique (28.3); conversely, a Fallback_Behavior resolves only
      such kinds. At the schema level, a kind out of effect leaves none of
      the keywords that only its other outcomes produce (bounds, ``format``
      and ``pattern``, ``dependentRequired``, ``enum``/``const``,
      ``propertyNames``, ``additionalProperties: false``, ``prefixItems``).
    - 26.10, 28.12: each fallback indicator names a serving Technique of the
      kind (or a disabled gate) with the reason that holds for it in the run
      (``disabled`` whenever every server is disabled). Each record of a
      Fallback_Behavior resolution names ``fallback:<kind>`` as its
      producer, holds the fallback indicator and reason, the
      Fallback_Behavior (``details.fallback_behavior``), the Laplace default
      Confidence, and matches a report entry of that kind with the same
      resolution and fallback indicator. Every other record's resolution
      (and every relaxation resolution under ``changes``) matches a report
      entry too, so it comes from a kind with a server in effect.

**Validates: Requirements 26.10, 28.2, 28.4, 28.11, 28.12**
"""

# Feature: json-schema-inference, Property 48: Technique isolation

from __future__ import annotations

import json
from collections.abc import Iterator, Mapping
from dataclasses import dataclass
from pathlib import Path as FilePath
from typing import Any
from unittest import mock

import pytest
from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.config.registry import OUTCOME_CANDIDATE_ORDER, REGISTRY
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.provenance import (
    BASELINE_PREFIX,
    BASELINE_RULES,
    FALLBACK_PREFIX,
    FALLBACK_REASONS,
    FallbackInfo,
    Resolution,
    laplace_confidence,
)
from jsinfer.core.report import DecisionEntry
from jsinfer.core.schema_model import SchemaNode
from jsinfer.decide.engine import DecisionEngine
from jsinfer.learn.bundle import DECISION_MODEL_TECHNIQUES
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from tests.models import fixed_models, write_model_bundle
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_values, setting_flags

IDS = frozenset(REGISTRY.ids())
IMPLEMENTED = tuple(tid for tid in REGISTRY.ids() if REGISTRY.technique(tid).implemented)
MDL = "decide.mdl"
RUN_LEVEL_KINDS = ("document_outlier", "version_split")

# The keywords that only the non-fallback outcomes of a Decision_Point kind produce: while no server of the
# kind is in effect, none of them may be emitted (bounds omitted, no Format or Pattern, no rules, open enums,
# Records rather than Maps, open Records, Lists rather than Tuples).
KIND_KEYWORDS: dict[str, tuple[str, ...]] = {
    "bound": (
        "minimum", "maximum", "minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties",
    ),
    "string_constraint": ("format", "pattern"),
    "cooccurrence_rule": ("dependentRequired",),  # also emitted by field_groups cliques; see check_keywords
    "closed_or_open_enum": ("enum", "const"),
    "record_or_map": ("propertyNames",),
    "tuple_or_list": ("prefixItems",),
}
CLOSED_RECORD = "closed_or_open_record"  # additionalProperties: false

# ---------------------------------------------------------------------------
# Model_Bundles
# ---------------------------------------------------------------------------

LOOSE: dict[str, int | float] = {
    "lsh.keyset_jaccard": 0.5,
    "dbscan.eps": 0.34,
    "dbscan.min_samples": 1,
    "hdbscan.min_cluster_size": 2,
    "cooccur.min_support": 1,
    "cooccur.min_confidence": 0.5,
}
STUB_THRESHOLDS: dict[str, dict[str, int | float]] = {
    "stub-full": LOOSE,
    # DBSCAN, HDBSCAN, and co-occurrence mining (which needs both of its thresholds) miss Learned_Thresholds.
    "stub-partial": {name: LOOSE[name] for name in ("lsh.keyset_jaccard", "cooccur.min_support")},
}
BUNDLE_THRESHOLDS: dict[str, frozenset[str]] = {
    "default": frozenset(),
    "models": frozenset(),  # stub Decision_Models for every Judgment_Call kind, no Learned_Threshold
    **{name: frozenset(values) for name, values in STUB_THRESHOLDS.items()},
}
# The Judgment_Call kinds whose Decision_Model each bundle holds.
BUNDLE_MODELS: dict[str, frozenset[str]] = {name: frozenset() for name in BUNDLE_THRESHOLDS}
BUNDLE_MODELS["models"] = frozenset(DECISION_MODEL_TECHNIQUES)


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> dict[str, str]:
    """Bundle identifiers by name: the shipped empty default and the stub bundles, written once."""
    root: FilePath = tmp_path_factory.mktemp("p48-bundles")
    found = {"default": DEFAULT_MODEL_BUNDLE}
    for name, values in STUB_THRESHOLDS.items():
        directory = root / name
        directory.mkdir()
        manifest = {"artifacts": {}, "bundle_id": f"p48-{name}", "format": 1}
        entries = {
            threshold: ThresholdRecord.from_spec(
                THRESHOLDS[threshold], value=value, objective_value=0.0, n_examples=1, training_data="p48"
            ).to_json()
            for threshold, value in values.items()
        }
        (directory / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (directory / "thresholds.json").write_text(json.dumps(entries), encoding="utf-8")
        found[name] = str(directory)
    found["models"] = write_model_bundle(root / "models", "p48-models", fixed_models())
    return found


# ---------------------------------------------------------------------------
# Collections
# ---------------------------------------------------------------------------

STRINGS = ("", "a", "ab", "A1", "12", "-5", "2.5", "2024-01-02T03:04:05Z", "x@y.co", "é😀")
SCALARS = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(-3, 40),
    st.sampled_from([0.5, 2.5, -1.25, 1.0, 1e3, 2**70]),
    st.sampled_from(STRINGS),
)
CATEGORY_POOL = ("red", "green", "blue", 0, 1, True, None)
DATES = ("2024-01-02", "2023-12-31", "2024-02-29")
ID_KEYS = st.integers(0, 99).map(lambda i: f"u{i}")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values: ``record`` fields ``(key, always, child)``; other kinds ``children``."""

    kind: str
    fields: tuple[tuple[str, bool, Template], ...] = ()
    children: tuple[Template, ...] = ()
    values: tuple[Any, ...] = ()  # a category's values
    fixed: bool = False  # a Tuple whose arrays always hold every position


@st.composite
def templates(draw: st.DrawFn, depth: int) -> Template:
    leaves = ["scalar", "category", "category", "date"]
    kind = draw(st.sampled_from(leaves + (["record", "record", "map", "list", "tuple", "mix"] if depth else [])))
    if kind == "category":
        return Template(kind, values=tuple(draw(st.lists(st.sampled_from(CATEGORY_POOL), min_size=1, max_size=3))))
    if kind in ("scalar", "date"):
        return Template(kind)
    if kind == "record":
        return draw(record_templates(depth))
    if kind in ("map", "list"):
        return Template(kind, children=(draw(templates(depth - 1)),))
    children = tuple(draw(st.lists(templates(depth - 1), min_size=1 if kind == "tuple" else 2, max_size=3)))
    return Template(kind, children=children, fixed=kind == "tuple" and draw(st.booleans(), label="fixed length"))


@st.composite
def record_templates(draw: st.DrawFn, depth: int, keys: tuple[str, ...] = KEYS, min_fields: int = 0) -> Template:
    names = draw(st.lists(st.sampled_from(keys), min_size=min_fields, max_size=4, unique=True), label="fields")
    fields = tuple((key, draw(st.booleans(), label=f"{key!r} always"), draw(templates(depth - 1))) for key in names)
    return Template("record", fields=fields)


@st.composite
def instances(draw: st.DrawFn, template: Template) -> Any:
    kind = template.kind
    if kind == "scalar":
        return draw(SCALARS)
    if kind == "category":
        return draw(st.sampled_from(template.values))
    if kind == "date":
        return draw(st.sampled_from(DATES))
    if kind == "record":
        return {
            key: draw(instances(child))
            for key, always, child in template.fields
            if always or draw(st.booleans(), label=f"include {key!r}")
        }
    if kind == "map":
        return {key: draw(instances(template.children[0])) for key in draw(st.lists(ID_KEYS, max_size=4))}
    if kind == "list":
        return draw(st.lists(instances(template.children[0]), max_size=3), label="elements")
    if kind == "tuple":
        size = len(template.children)
        length = size if template.fixed else draw(st.integers(0, size), label="tuple length")
        return [draw(instances(child)) for child in template.children[:length]]
    options = template.children
    return draw(instances(options[draw(st.integers(0, len(options) - 1), label="alternative")]))


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    shapes = ["record", "record", "variants", "variants", "records", "tuple", "any"]
    shape = draw(st.sampled_from(shapes), label="root shape")
    if shape == "record":
        return draw(record_templates(2, min_fields=1))
    if shape == "records":
        return Template("list", children=(draw(record_templates(2, min_fields=1)),))
    if shape == "tuple":
        first = Template("category", values=(0, 1, 7))
        return Template("tuple", children=(first, Template("date")), fixed=True)
    if shape == "variants":  # Records with disjoint fields, so that splitting can beat merging
        keys = draw(st.permutations(KEYS), label="variant fields")
        cut = draw(st.integers(1, 3), label="first variant's fields")
        first = draw(record_templates(1, keys=tuple(keys[:cut]), min_fields=1))
        second = draw(record_templates(1, keys=tuple(keys[cut : cut + 3]), min_fields=1))
        return Template("mix", children=(first, second))
    return draw(templates(2))


@st.composite
def collections(draw: st.DrawFn) -> list[Any]:
    root = draw(root_templates(), label="root template")
    size = st.integers(14, 20) if draw(st.booleans(), label="large collection") else st.integers(2, 12)
    docs = [draw(instances(root), label="document") for _ in range(draw(size, label="documents"))]
    docs += draw(st.lists(json_values(max_leaves=4), max_size=1), label="arbitrary documents")
    return docs


# ---------------------------------------------------------------------------
# Toggles and the run's Technique states
# ---------------------------------------------------------------------------


def prerequisite_conflicts(states: Mapping[str, bool]) -> list[tuple[str, str]]:
    """(enabled Technique, disabled prerequisite) pairs (28.8)."""
    return [
        (tid, prerequisite)
        for tid in REGISTRY.ids()
        if states[tid]
        for prerequisite in REGISTRY.technique(tid).prerequisites
        if not states[prerequisite]
    ]


def acceptable(states: Mapping[str, bool]) -> bool:
    """Whether the Run_Configuration accepts these states: prerequisites respected, nothing unimplemented enabled."""
    implemented = all(REGISTRY.technique(tid).implemented or not on for tid, on in states.items())
    return implemented and not prerequisite_conflicts(states)


@dataclass(frozen=True)
class Setup:
    """A run's flags, the Technique toggles among them, the resolved configuration, and the bundle."""

    flags: dict[str, Any]
    toggles: dict[str, bool]
    config: RunConfiguration
    bundle: str
    scenario: str

    @property
    def states(self) -> Mapping[str, bool]:
        return self.config.techniques

    @property
    def held(self) -> frozenset[str]:
        return BUNDLE_THRESHOLDS[self.bundle]

    def cause(self, tid: str, kind: str) -> str | None:
        """Why ``tid`` cannot act at Decision_Point ``kind`` in this run, or None when it is in effect there."""
        if not self.states[tid]:
            return "disabled"
        spec = REGISTRY.decision_point(kind)
        gate = spec.serves_only_with.get(tid)
        if gate is not None and not self.states[gate]:
            return "gated"
        if DECISION_MODEL_TECHNIQUES.get(kind) == tid:
            return None if kind in BUNDLE_MODELS[self.bundle] else "missing-model"
        thresholds = REGISTRY.technique(tid).thresholds
        if any(THRESHOLDS[name].decision_point == kind and name not in self.held for name in thresholds):
            return "missing-threshold"
        return None

    def all_disabled(self, kind: str) -> bool:
        return not any(self.states[tid] for tid in REGISTRY.servers(kind))

    def out_of_effect(self, kind: str) -> bool:
        return all(self.cause(tid, kind) is not None for tid in REGISTRY.servers(kind))

    def in_effect_somewhere(self, tid: str) -> bool:
        """Enabled and holding every Learned_Threshold it reads (for records, whose kind is not stored)."""
        if not self.states[tid]:
            return False
        return all(name in self.held for name in REGISTRY.technique(tid).thresholds)


# The toggle scenarios, one test each: any subset of the toggles; any subset with MDL selection off (priority order
# and the Judgment_Calls' Fallback_Behaviors, 19.7, 28.4); every Technique off; and, for each implemented
# Technique, that Technique alone off and that Technique alone on (every other one off).
SCENARIOS: tuple[tuple[str, str | None], ...] = (
    ("drawn", None),
    ("mdl off", None),
    ("all off", None),
    *(("one off", tid) for tid in IMPLEMENTED),
    *(("one on", tid) for tid in IMPLEMENTED),
)


@st.composite
def setups(draw: st.DrawFn, bundles: Mapping[str, str], scenario: str, technique: str | None) -> Setup:
    bundle = draw(st.sampled_from(sorted(bundles)), label="bundle")
    base: dict[str, Any] = {
        "workers": 1,
        "model_bundle": bundles[bundle],
        "soundness.max_passes": draw(st.sampled_from([3, 1]), label="soundness.max_passes"),
        "minhash.perm": 8,
        "lsh.bands": draw(st.sampled_from([1, 2, 4, 8]), label="lsh.bands"),
        "variants.max_components": draw(st.integers(1, 3), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([5, 100]), label="variants.em_max_iter"),
    }
    vary: tuple[str, ...] = ("seed",)
    if scenario in ("drawn", "mdl off"):
        vary = (*TECHNIQUE_SETTINGS, "seed")
        if scenario == "mdl off":
            base[technique_setting(MDL)] = False
    elif scenario == "one off":
        assert technique is not None
        base[technique_setting(technique)] = False
        # A disabled prerequisite takes its dependents with it, transitively (28.8; the ablation rule of 30.10).
        off = {technique}
        while True:
            more = {t for t in REGISTRY.ids() if t not in off and set(REGISTRY.technique(t).prerequisites) & off}
            if not more:
                break
            off |= more
        base.update({technique_setting(t): False for t in off})
    else:  # "one on" (with its prerequisites, none of which an implemented Technique has yet) or "all off"
        closure = set() if technique is None else {technique, *REGISTRY.technique(technique).prerequisites}
        base.update({technique_setting(tid): tid in closure for tid in REGISTRY.ids()})
    flags = draw(setting_flags(vary=vary, base=base), label="flags")
    toggles = {tid: flags[technique_setting(tid)] for tid in REGISTRY.ids() if technique_setting(tid) in flags}
    return Setup(flags, toggles, RunConfiguration.resolve(flags), bundle, scenario)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def is_indicator(value: JSON) -> bool:
    """Whether ``value`` is a fallback indicator in JSON form (``FallbackInfo.to_json``)."""
    return isinstance(value, dict) and sorted(value) == ["reason", "technique"] and value["reason"] in FALLBACK_REASONS


def named(value: JSON) -> Iterator[str]:
    """Every registered Technique id in a record's (or entry's) JSON, outside its fallback indicators.

    A fallback indicator names the Technique whose Fallback_Behavior was applied
    (26.10): the record's own ``fallback`` member, and the ``fallback`` member
    of a decision summary a record embeds in ``details`` (for example the
    ``variant_grouping`` decision behind a Merge-or-Split record).
    """
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "fallback" and is_indicator(item):
                continue
            yield from named(item)
    elif isinstance(value, list):
        for item in value:
            yield from named(item)
    elif isinstance(value, str) and value in IDS:
        yield value


def indicators(value: JSON) -> Iterator[dict[str, Any]]:
    """Every fallback indicator in a record's JSON, at any depth."""
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "fallback" and is_indicator(item):
                yield item
            else:
                yield from indicators(item)
    elif isinstance(value, list):
        for item in value:
            yield from indicators(item)


def check_indicator_reason(info: Mapping[str, Any], setup: Setup, where: str) -> None:
    """26.10 without the kind: a fallback indicator's reason holds for its Technique in this run."""
    tid, reason = info["technique"], info["reason"]
    assert tid in IDS, f"{where}: a fallback indicator names the unregistered {tid!r}"
    if reason == "disabled":
        assert not setup.states[tid], f"{where}: the indicator says {tid} is disabled, but it is enabled"
    else:
        assert setup.states[tid], f"{where}: the indicator says {tid} is {reason}, but it is disabled"
    if reason == "missing-threshold":
        assert not setup.in_effect_somewhere(tid), f"{where}: {tid} misses no Learned_Threshold"


def key(resolution: Resolution, fallback: FallbackInfo | None) -> str:
    return json.dumps(
        [resolution.to_json(), None if fallback is None else fallback.to_json()], sort_keys=True, ensure_ascii=False
    )


def check_fallback_info(info: FallbackInfo | None, kind: str, setup: Setup, where: str, *, fallback: bool) -> None:
    """26.10: the Technique whose Fallback_Behavior applied, with the reason that holds for it in this run.

    ``fallback`` is whether a Fallback_Behavior resolved the kind. Otherwise the
    indicator is that of a Decision_Model (22.13) or of the disabled MDL selection
    behind a priority resolution (19.7).
    """
    assert info is not None, f"{where}: no fallback indicator"
    spec = REGISTRY.decision_point(kind)
    tid = info.technique
    gates = set(spec.serves_only_with.values())
    allowed = set(spec.served_by) | gates | (set() if fallback else {MDL})
    assert tid in allowed, f"{where}: the fallback indicator names {tid!r}, which neither serves nor gates {kind!r}"
    if tid in spec.served_by:
        cause = setup.cause(tid, kind)
        assert cause != "gated", f"{where}: names the gated {tid!r} rather than its disabled gate"
    else:
        cause = None if setup.states[tid] else "disabled"
    assert info.reason == cause, f"{where}: reason {info.reason!r} for {tid!r}, which is {cause or 'in effect'}"
    if fallback and setup.all_disabled(kind):
        assert info.reason == "disabled", f"{where}: every server of {kind!r} is disabled, reason {info.reason!r}"
    event(f"fallback indicator {info.reason}{' (Fallback_Behavior)' if fallback else ''}")


def check_fallback_resolution(resolution: Resolution, kind: str, where: str) -> None:
    """28.3, 28.4: the Fallback_Behavior's outcome selected, and no candidate of any Technique listed."""
    spec = REGISTRY.decision_point(kind)
    assert resolution.method == "fallback", f"{where}: resolved by {resolution.method!r}, not the Fallback_Behavior"
    techniques = [c.technique for c in resolution.candidates if c.technique is not None]
    assert not techniques, f"{where}: a Fallback_Behavior lists candidates of {techniques}"
    outcome = spec.fallback.outcome
    if outcome == OUTCOME_CANDIDATE_ORDER:
        assert resolution.selected == 0, f"{where}: not the first candidate in the documented order"
    else:
        selected = resolution.selected_candidate.outcome
        assert selected == outcome, f"{where}: selected {selected!r}, the Fallback_Behavior is {outcome!r}"


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def check_effective_states(setup: Setup, result: PipelineResult) -> None:
    """28.2: toggles, else registered defaults; setting one more toggle changes no other Technique's state."""
    expected = {tid: setup.toggles.get(tid, REGISTRY.technique(tid).default_enabled) for tid in REGISTRY.ids()}
    assert dict(setup.config.techniques) == expected, "the effective states are not the toggles and defaults"
    assert dict(result.config.techniques) == expected, "the run used other Technique states"
    assert result.report.run is not None
    assert result.report.run.config["techniques"] == expected, "the report's run section lists other states"
    assert dict(DecisionEngine.for_run(result.config, None, None).techniques) == expected
    for tid in REGISTRY.ids():
        for state in (not expected[tid], expected[tid]):
            states = {**expected, tid: state}
            if not acceptable(states):
                continue  # an unimplemented Technique enabled, or a prerequisite violated
            config = RunConfiguration.resolve({**setup.flags, technique_setting(tid): state})
            changed = sorted(t for t in REGISTRY.ids() if config.techniques[t] != expected[t])
            assert changed == ([tid] if state != expected[tid] else []), (
                f"setting {tid} to {state} changed the effective state of {changed}"
            )


def check_report_decisions(setup: Setup, result: PipelineResult) -> None:
    """28.4, 28.11, 26.10 over every resolved Decision_Point, Constraint-producing or not."""
    seen: dict[str, int] = {kind: 0 for kind in RUN_LEVEL_KINDS}
    for entry in result.report.decisions:
        kind = entry.kind
        where = f"the {kind} decision at {entry.path}"
        resolution = entry.resolution
        # 28.11: no candidate of a Technique that is not in effect at this kind is generated or selected.
        listed = list(resolution.candidates) + (list(entry.cutoff.scores) if entry.cutoff is not None else [])
        for candidate in listed:
            if candidate.technique is not None:
                cause = setup.cause(candidate.technique, kind)
                assert cause is None, f"{where}: lists a candidate of {candidate.technique} ({cause})"
        for tid in named(entry.to_json()):
            assert setup.states[tid], f"{where}: names the disabled {tid}"
        # 28.4: a kind none of whose servers is in effect is resolved by its Fallback_Behavior, and only such kinds.
        if setup.out_of_effect(kind):  # all_disabled implies out_of_effect
            check_fallback_resolution(resolution, kind, where)
            event(f"Fallback_Behavior at {kind} ({'all disabled' if setup.all_disabled(kind) else 'out of effect'})")
        if resolution.method == "fallback":
            assert setup.out_of_effect(kind), f"{where}: a Fallback_Behavior while a server is in effect"
            assert entry.details.get("fallback_behavior") == REGISTRY.decision_point(kind).fallback_doc, (
                f"{where}: the Fallback_Behavior is not recorded"
            )
            check_fallback_info(entry.fallback, kind, setup, where, fallback=True)
        elif entry.fallback is not None:
            check_fallback_info(entry.fallback, kind, setup, where, fallback=False)
        if kind in seen:
            assert entry.path == "$", f"{where}: a run-level Decision_Point below the root"
            seen[kind] += 1
    for kind, count in seen.items():
        assert count == 1, f"the run-level {kind} Decision_Point was resolved {count} times"


def check_records(setup: Setup, result: PipelineResult) -> None:
    """28.11, 28.12, 26.10, 26.13 over every Provenance_Record of the explanation file."""
    entries: dict[str, list[DecisionEntry]] = {}
    relaxations: dict[str, list[DecisionEntry]] = {}  # by resolution alone: a RelaxationRecord holds no indicator
    for entry in result.report.decisions:
        entries.setdefault(key(entry.resolution, entry.fallback), []).append(entry)
        if entry.kind == "relaxation":
            relaxations.setdefault(key(entry.resolution, None), []).append(entry)
    records = result.explanation.records
    assert records == {result.pointers[cid]: record for cid, record in result.model.provenance.items()}
    for pointer, record in records.items():
        where = f"the record at {pointer}"
        technique = record.technique
        # 28.11: the record names no disabled Technique, nor one that is enabled but not in effect.
        for tid in named(record.to_json()):
            assert setup.states[tid], f"{where}: names the disabled {tid}"
            assert setup.in_effect_somewhere(tid), f"{where}: names {tid}, which misses a Learned_Threshold"
        for info in indicators(record.to_json()):
            check_indicator_reason(info, setup, where)
        for change in record.changes:
            if change.resolution is not None:  # a relaxation decision (3.3)
                assert key(change.resolution, None) in relaxations, f"{where}: an unreported relaxation"
                if setup.out_of_effect("relaxation"):
                    check_fallback_resolution(change.resolution, "relaxation", f"{where} (relaxation)")
        if technique.startswith(BASELINE_PREFIX):  # 26.13
            assert technique[len(BASELINE_PREFIX) :] in BASELINE_RULES, f"{where}: {technique!r}"
            assert record.fallback is None and record.resolution is None, f"{where}: {technique} resolved something"
            event("Baseline_Rule record")
            continue
        fallback = technique.startswith(FALLBACK_PREFIX)
        if not fallback:
            assert technique in IDS, f"{where}: unregistered producer {technique!r}"
            assert REGISTRY.technique(technique).implemented, f"{where}: unimplemented producer {technique}"
        resolution = record.resolution
        if fallback or (resolution is not None and resolution.method == "fallback"):
            # 28.12, 26.10: the Decision_Point and the Fallback_Behavior in place of a producer, with the
            # fallback indicator and the reason.
            assert fallback, f"{where}: a Fallback_Behavior resolution produced by {technique!r}"
            kind = technique[len(FALLBACK_PREFIX) :]
            assert kind in REGISTRY.decision_points, f"{where}: fallback marker of an unknown kind {technique!r}"
            assert resolution is not None, f"{where}: {technique} without its resolution"
            check_fallback_resolution(resolution, kind, where)
            assert setup.out_of_effect(kind), f"{where}: {technique} while a server of {kind} is in effect"
            check_fallback_info(record.fallback, kind, setup, where, fallback=True)
            assert record.details.get("fallback_behavior") == REGISTRY.decision_point(kind).fallback_doc, (
                f"{where}: the Fallback_Behavior is not recorded"
            )
            assert record.confidence == laplace_confidence(record.support), f"{where}: not the default Confidence"
            matches = entries.get(key(resolution, record.fallback), [])
            assert any(e.kind == kind for e in matches), f"{where}: no {kind} decision in the report matches"
            event(f"Fallback_Behavior record of {kind}")
        elif resolution is not None:
            matches = entries.get(key(resolution, record.fallback), [])
            assert matches, f"{where}: its resolution is not a reported decision"
            assert all(not setup.out_of_effect(e.kind) for e in matches), f"{where}: from a kind out of effect"
            event(f"record resolved by {resolution.method}")
        else:
            assert record.fallback is None, f"{where}: a fallback indicator without a resolution"


def check_keywords(setup: Setup, result: PipelineResult) -> None:
    """28.4 at the schema level: a kind out of effect emits only its Fallback_Behavior's outcome."""
    absent = {kind: words for kind, words in KIND_KEYWORDS.items() if setup.out_of_effect(kind)}
    if "cooccurrence_rule" in absent and not setup.out_of_effect("field_groups"):
        absent["cooccurrence_rule"] = ()  # Field_Group cliques (6.6) share dependentRequired with the rules (9.4)
    closed = setup.out_of_effect(CLOSED_RECORD)
    for pointer, node in result.model.walk():
        if node.boolean is not None:
            continue
        for kind, words in absent.items():
            found = [word for word in words if word in node.kw]
            assert not found, f"{pointer or '#'}: {found} emitted while no server of {kind} is in effect"
        additional = node.kw.get("additionalProperties")
        if closed and isinstance(additional, SchemaNode):
            assert additional.boolean is not False, f"{pointer or '#'}: a closed Record while {CLOSED_RECORD} is out"
    for kind in absent:
        event(f"schema without {kind} keywords")


# ---------------------------------------------------------------------------
# Reaching the relaxation Decision_Point
# ---------------------------------------------------------------------------

ORIGINAL_INFER_MODEL = StructureAnalyzer.infer_model
TYPE_NAMES = ("null", "boolean", "integer", "number", "string", "array", "object")


def narrowing(data: st.DataObject) -> Any:
    """``StructureAnalyzer.infer_model``, then, in about half the runs, one ``type`` Constraint narrowed.

    The soundness check then relaxes it (3.3), so the ``relaxation``
    Decision_Point, served by ``decide.mdl`` alone, is reached. The
    Constraint keeps its ``baseline:type`` record, which names no Technique.
    """

    def infer_model(self: StructureAnalyzer, view: Any) -> Any:
        model = ORIGINAL_INFER_MODEL(self, view)
        typed = [(pointer, node) for pointer, node in model.walk() if node.boolean is None and "type" in node.kw]
        if typed and data.draw(st.booleans(), label="narrow a type"):
            pointer, node = data.draw(st.sampled_from(typed), label="narrowed node")
            value = node.kw["type"]
            names = [value] if isinstance(value, str) else list(value)
            if len(names) > 1:
                names.remove(data.draw(st.sampled_from(names), label="dropped type"))
            else:
                names = [data.draw(st.sampled_from([t for t in TYPE_NAMES if t not in names]), label="new type")]
            node.kw["type"] = names[0] if len(names) == 1 else names
            note(f"narrowed type at {pointer or '#'}: {value!r} -> {node.kw['type']!r}")
        return model

    return infer_model


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("scenario", "technique"),
    SCENARIOS,
    ids=[" ".join(filter(None, case)).replace(" ", "-") for case in SCENARIOS],
)
@given(data=st.data())
def test_technique_toggles_are_isolated_and_disabled_decision_points_fall_back(
    bundles: dict[str, str], scenario: str, technique: str | None, data: st.DataObject
) -> None:
    setup = data.draw(setups(bundles, scenario, technique), label="setup")
    docs = data.draw(collections(), label="documents")
    off = sorted(tid for tid in IMPLEMENTED if not setup.states[tid])
    note(f"bundle {setup.bundle}, implemented Techniques off: {off}")
    event(f"bundle {setup.bundle}")

    with mock.patch.object(StructureAnalyzer, "infer_model", narrowing(data)):
        result = infer_documents(docs, setup.config)
    note(f"schema: {result.schema.decode()}")
    if any(entry.kind == "relaxation" for entry in result.report.decisions):
        event("relaxation decision")

    check_effective_states(setup, result)
    check_report_decisions(setup, result)
    check_records(setup, result)
    check_keywords(setup, result)
