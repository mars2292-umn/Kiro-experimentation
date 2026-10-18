"""Property 31: Cut-off provenance (task 19.8).

For any run, every resolved Decision_Point records either a cut-off source
that is "MDL comparison", "Decision_Model comparison" (with the
Decision_Model artifact), a model-selection criterion name, or a registered
Learned_Threshold, along with the value or the compared scores or
probabilities; or, for a constant Fallback_Behavior or a priority selection
with no Learned_Threshold applied, that no cut-off was applied. Every
Learned_Threshold and Resource_Parameter read is listed in the report with
its value and origin.

(Requirements 20.1, 20.5, 20.7.)

Runs
    Each example is a full in-process ``pipeline.infer.infer_documents`` run
    on a generated collection, with ``workers = 1``. Technique toggles are
    drawn by ``tests.strategies.setting_flags`` (any subset, prerequisites
    repaired), with ``decide.mdl`` on in half the runs and off in the
    other half, so that priority selections (19.7) and the
    Confidence-threshold path of co-occurrence mining (9.8) are reached.
    Each Resource_Parameter is left at its default or supplied, with a small
    value, as a flag or in a configuration file, so every origin occurs.
    The Model_Bundle is a test parameter, so each gets the profile's
    examples: the shipped default (no Learned_Threshold) or one of four stub
    bundles with distinct ids and hand-set Learned_Thresholds, written once
    per module, so that each grouping Technique is the first one in effect
    under some bundle, and grouping and co-occurrence Techniques read their
    thresholds under some bundles and miss them under others.
    Collections are the templates of Property 48 plus Records whose
    optional fields occur together. In about half the runs one ``type``
    Constraint of the analyzer's model is narrowed (as in Property 48), so
    that the soundness check reaches the ``relaxation`` Decision_Point.

Observing reads (independently of the report)
    During the run, every value source hands out *tainted* numbers: an
    ``int`` or ``float`` subclass that remembers which Resource_Parameter or
    Learned_Threshold it is and records a *use* whenever it is converted
    (``int()``, ``float()``, ``index``), compared, computed with, hashed, or
    formatted. The sources are

    - for Resource_Parameters: ``ResourceReader.value`` (the access behind
      every ``ResourceReader.read``), the run's Run_Configuration
      (``config.resources``, so ``config.workers`` and ``config.value`` as
      well), and the defaults (``SettingSpec.default_value`` of a
      Resource_Parameter). Handing a value out is also recorded as a read,
      with its source;
    - for Learned_Thresholds: the records of the run's ``ResolvedBundle``
      (``ModelBundle.resolve``), so both ``ResolvedBundle.threshold`` and a
      direct ``thresholds[name].value`` give tainted values. Handing a value
      out through ``threshold()`` is recorded too.

    A few places inspect values without reading them for the run, and
    record nothing: ``ResourceReader`` construction (it checks and
    normalizes the run's values), ``RunConfiguration.to_json`` (the
    effective configuration in the ``run`` section), the VariantAnalyzer's
    ``resources`` snapshot (``structure.variants._resource_values``; using a
    snapshot value later does record a use), and the report's own
    ``record_resource`` and ``record_threshold``. An availability check of a
    Learned_Threshold (``threshold(name) is None``, 20.6) neither converts
    nor compares the value, so it is not a use.

Checks
    - 20.5, Resource_Parameters: every Resource_Parameter handed out or used
      during the run is listed in the report, and every listed one was
      handed out (the listing is exact). Each listing, and every value the
      run obtained, equals the run's effective value (type included), and
      the listed origin is how the test supplied it (``flag``, ``file``,
      or ``default``), which is also ``RunConfiguration.provenance_of``.
    - 20.5, Learned_Thresholds: every Learned_Threshold used during the run
      is listed, and every listed one is registered and was handed out by
      the bundle. Its value is the one the test stored, and its bundle id is
      the id in the stub's manifest; the default bundle lists none.
    - 20.1, 20.7, on every report decision entry (every DecisionEntry the
      DecisionEngine records): the cut-off source is exactly one of the four
      kinds and never a Resource_Parameter.

      * ``mdl``: "MDL comparison" with the scores of exactly the resolved
        candidates.
      * ``decision-model``: "Decision_Model comparison" with the artifact
        ``dm.<kind>.synthetic`` or ``dm.<kind>.weak`` and the calibrated
        probabilities of the candidates, the selected one highest; or, for
        equal probabilities, "MDL comparison" over the tied candidates
        (22.12).
      * ``priority``: a registered Learned_Threshold that the selected
        candidate's Technique reads at this kind, with its value, which is
        the value listed in the report; a model-selection criterion
        (``BIC``) of a Technique that selects by it, with the compared
        scores; or no cut-off, and then the selected Technique reads no
        Learned_Threshold at this kind and selects by no criterion.
      * ``fallback``: no cut-off, and the kind's Fallback_Behavior is a
        constant outcome (every registered one is).

    - 20.7, Provenance_Records: every record with a resolution carries the
      cut-off of a report decision entry with the same resolution and
      fallback indicator (so the entry checks cover it); a record without a
      resolution carries no cut-off. Every relaxation resolution under a
      record's ``changes`` is a reported relaxation decision.
    - 20.7, outcomes without a Constraint or a decision entry: each rejected
      co-occurrence candidate (below ``cooccur.min_support``, 9.6, or
      ``cooccur.min_confidence``, 9.8) names its Learned_Threshold and the
      value, which is the value the report lists.

Interpretation
    - "Read" means obtained from a value source outside the inspections
      above, or used. Gating a Technique on whether the bundle holds a
      Learned_Threshold is not a read, so ``cooccur.min_confidence`` is not
      listed while MDL selects co-occurrence rules (9.4), although
      ``rules.cooccurrence`` needs it to be in effect.
    - A relaxation that changes an existing Constraint records its
      resolution in that Constraint's ``changes`` (3.6), and its cut-off in
      the report's ``relaxation`` decision entry; a Constraint created under
      3.3 takes the relaxation decision's whole record.
    - The model-selection criteria are those the design names for
      Decision_Point cut-offs: BIC, for the Bernoulli mixture (5.2).
    - Decision_Model comparisons cannot occur yet: every ``decide.dm_*``
      Technique is unimplemented, so it is disabled in every valid
      Run_Configuration. The checks above apply to them once they land.
    - Limitations: a use inside C code that reads the number without Python
      conversion (numpy, ``math.isfinite``) is not seen as a use, so
      Resource_Parameters also count when handed out; reads of
      ``SettingSpec.default`` (the attribute) or of the raw
      ``thresholds.json`` entries (``ModelBundle.threshold_entries``) are
      not observed.

**Validates: Requirements 20.1, 20.5, 20.7**
"""

# Feature: json-schema-inference, Property 31: Cut-off provenance

from __future__ import annotations

import copy
import dataclasses
import json
import math
import operator
from collections.abc import Callable, Iterator, Mapping
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass
from pathlib import Path as FilePath
from types import MappingProxyType
from typing import Any
from unittest import mock

import pytest
from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY, ConstantOutcome
from jsinfer.config.resources import RESOURCE_PARAMETERS, WORKERS, ResourceReader, SettingSpec
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.provenance import (
    DECISION_MODEL_COMPARISON,
    MDL_COMPARISON,
    Cutoff,
    FallbackInfo,
    Resolution,
)
from jsinfer.core.report import InferenceReport
from jsinfer.learn.bundle import (
    BUNDLE_FORMAT,
    DECISION_MODEL_SOURCES,
    DECISION_MODEL_TECHNIQUES,
    MANIFEST_FILE,
    THRESHOLDS_FILE,
    ModelBundle,
    ResolvedBundle,
    decision_model_artifact,
    load_bundle,
)
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure import variants as variants_module
from jsinfer.structure.analyzer import StructureAnalyzer
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_values, setting_flags

MDL = "decide.mdl"

# The model-selection criteria of 20.1 and the Techniques that select by them (design: Variant grouping, 5.2).
MODEL_SELECTION_CRITERIA: dict[str, frozenset[str]] = {"BIC": frozenset({"variants.bernoulli_mixture"})}

# The co-occurrence miner's category of rejected candidates and the Learned_Thresholds it names there.
RULES_CATEGORY = "rules"
COOCCURRENCE_THRESHOLDS = frozenset({"cooccur.min_support", "cooccur.min_confidence"})


# ---------------------------------------------------------------------------
# Model_Bundles
# ---------------------------------------------------------------------------

# Hand-set Learned_Thresholds of the stub bundles; each stub has its own values and bundle id, and each
# grouping Technique is the first one in effect (the priority selection, 5.11) under some stub.
STUB_THRESHOLDS: dict[str, dict[str, int | float]] = {
    "full": {
        "lsh.keyset_jaccard": 0.5,
        "dbscan.eps": 0.34,
        "dbscan.min_samples": 1,
        "hdbscan.min_cluster_size": 2,
        "cooccur.min_support": 1,
        "cooccur.min_confidence": 0.5,
    },
    # DBSCAN and HDBSCAN miss their Learned_Thresholds.
    "rules": {"lsh.keyset_jaccard": 0.75, "cooccur.min_support": 2, "cooccur.min_confidence": 0.8},
    # LSH misses its Learned_Threshold, and co-occurrence mining its Confidence threshold.
    "density": {"dbscan.eps": 0.2, "dbscan.min_samples": 2, "hdbscan.min_cluster_size": 3, "cooccur.min_support": 1},
    # Only HDBSCAN and co-occurrence mining hold theirs.
    "hdbscan": {"hdbscan.min_cluster_size": 2, "cooccur.min_support": 1, "cooccur.min_confidence": 0.6},
}


@dataclass(frozen=True)
class Bundle:
    """A Model_Bundle of the module: its identifier, its bundle id, and the Learned_Thresholds it holds."""

    identifier: str
    bundle_id: str
    values: Mapping[str, int | float]


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> dict[str, Bundle]:
    """The shipped default bundle and the stub bundles (written once), by name."""
    shipped = load_bundle(DEFAULT_MODEL_BUNDLE)
    found = {"default": Bundle(DEFAULT_MODEL_BUNDLE, shipped.bundle_id, shipped.threshold_values())}
    root: FilePath = tmp_path_factory.mktemp("p31-bundles")
    for name, values in STUB_THRESHOLDS.items():
        directory = root / name
        directory.mkdir()
        bundle_id = f"p31-{name}"
        manifest = {"artifacts": {}, "bundle_id": bundle_id, "format": BUNDLE_FORMAT}
        entries = {
            threshold: ThresholdRecord.from_spec(
                THRESHOLDS[threshold], value=value, objective_value=0.0, n_examples=1, training_data="p31"
            ).to_json()
            for threshold, value in values.items()
        }
        (directory / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
        (directory / THRESHOLDS_FILE).write_text(json.dumps(entries), encoding="utf-8")
        found[name] = Bundle(str(directory), bundle_id, dict(values))
    return found


# ---------------------------------------------------------------------------
# Run_Configurations
# ---------------------------------------------------------------------------

# Small values for the Resource_Parameters the implemented components read; the others get a small value in
# their range (only an unexpected read would make a large one costly).
SMALL_VALUES: dict[str, st.SearchStrategy[Any]] = {
    "ingest.chunk_size": st.integers(1, 40),
    "soundness.max_passes": st.integers(1, 6),
    "variants.max_components": st.integers(1, 4),
    "variants.em_max_iter": st.integers(1, 40),
    "variants.em_tol": st.sampled_from([0, 0.0, 1e-6, 1e-3, 0.25]),  # a float setting takes ints too
    "sample.dbscan": st.integers(1, 30),
    "sample.hdbscan": st.integers(1, 30),
}
PAIRED = ("minhash.perm", "lsh.bands")  # lsh.bands <= minhash.perm (BOUNDED_BY)
ORIGINS = ("default", "flag", "file")


def small_values(name: str) -> st.SearchStrategy[Any]:
    if name in SMALL_VALUES:
        return SMALL_VALUES[name]
    spec = RESOURCE_PARAMETERS[name]
    assert spec.range is not None and spec.range.low is not None
    low = spec.range.low
    if spec.type is int:
        start = math.ceil(low) + (1 if spec.range.low_open and math.ceil(low) == low else 0)
        return st.integers(start, start + 40)
    return st.floats(float(low), float(low) + 60.0, exclude_min=spec.range.low_open, allow_nan=False)


@dataclass(frozen=True)
class Setup:
    """A run's settings: flags and configuration file, the resolved configuration, and its bundle."""

    flags: dict[str, Any]
    file: dict[str, dict[str, Any]]
    config: RunConfiguration
    bundle: Bundle
    bundle_name: str

    @property
    def resources(self) -> dict[str, int | float]:
        """The effective value of every Resource_Parameter (plain numbers)."""
        return dict(self.config.resources)

    def origin(self, name: str) -> str:
        """How the test supplied Resource_Parameter ``name``."""
        if name in self.flags:
            return "flag"
        if name in self.file.get("resources", {}) or name in self.file.get("run", {}):
            return "file"
        return "default"


@st.composite
def resource_settings(draw: st.DrawFn) -> dict[str, tuple[str, Any]]:
    """Resource_Parameter name -> (origin, value) for the supplied ones; ``workers`` is always 1."""
    chosen: dict[str, tuple[str, Any]] = {WORKERS: (draw(st.sampled_from(ORIGINS[1:]), label="workers origin"), 1)}
    perm_mode = draw(st.sampled_from(["neither", "both", "bands", "perm"]), label="minhash.perm and lsh.bands")
    if perm_mode == "both":
        perm = draw(st.integers(1, 32), label="minhash.perm")
        chosen["minhash.perm"] = (draw(st.sampled_from(ORIGINS[1:]), label="minhash.perm origin"), perm)
        chosen["lsh.bands"] = (draw(st.sampled_from(ORIGINS[1:]), label="lsh.bands origin"), draw(st.integers(1, perm)))
    elif perm_mode == "bands":  # at most the default minhash.perm (128)
        chosen["lsh.bands"] = (draw(st.sampled_from(ORIGINS[1:])), draw(st.integers(1, 32), label="lsh.bands"))
    elif perm_mode == "perm":  # at least the default lsh.bands (32)
        chosen["minhash.perm"] = (draw(st.sampled_from(ORIGINS[1:])), draw(st.integers(32, 48), label="minhash.perm"))
    for name in RESOURCE_PARAMETERS:
        if name == WORKERS or name in PAIRED:
            continue
        origin = draw(st.sampled_from(ORIGINS), label=f"{name} origin")
        if origin != "default":
            chosen[name] = (origin, draw(small_values(name), label=name))
    return chosen


@st.composite
def setups(draw: st.DrawFn, bundles: Mapping[str, Bundle], name: str) -> Setup:
    bundle = bundles[name]
    base = {technique_setting(MDL): draw(st.booleans(), label="MDL selection")}
    flags = draw(setting_flags(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="toggles and seed")
    flags["model_bundle"] = bundle.identifier
    file: dict[str, dict[str, Any]] = {}
    for setting, (origin, value) in draw(resource_settings(), label="Resource_Parameters").items():
        if origin == "flag":
            flags[setting] = value
        else:
            file.setdefault("run" if setting == WORKERS else "resources", {})[setting] = value
    return Setup(flags, file, RunConfiguration.resolve(flags, file), bundle, name)


# ---------------------------------------------------------------------------
# Collections (the templates of Property 48, plus fields that occur together)
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
    """A generator of JSON values: ``record`` fields ``(key, always, child)``; other kinds ``children``.

    The optional fields of a record named in ``together`` are present together or absent together.
    """

    kind: str
    fields: tuple[tuple[str, bool, Template], ...] = ()
    children: tuple[Template, ...] = ()
    values: tuple[Any, ...] = ()  # a category's values
    fixed: bool = False  # a Tuple whose arrays always hold every position
    together: tuple[str, ...] = ()


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


# Leaves of one JSON type each, so that a field's (key, type) signature entry does not vary.
STABLE_LEAVES = (
    Template("date"),
    Template("category", values=("red", "green", "blue")),
    Template("category", values=(0, 1)),
)


@st.composite
def paired_templates(draw: st.DrawFn) -> Template:
    """Records whose optional fields mostly occur together, as Co_Occurrence_Rule candidates (9.3–9.8).

    Most fields are always present, with one type each, so that the objects
    stay similar enough to share a Variant under the stub thresholds; two
    optional fields occur together, and at most one occurs on its own (a
    rule below the Confidence threshold).
    """
    keys = draw(st.permutations(KEYS), label="paired fields")
    n_always = draw(st.integers(4, 6), label="fields always present")
    n_alone = draw(st.integers(0, 1), label="fields present on their own")
    leaf = st.sampled_from(STABLE_LEAVES)
    always = tuple((key, True, draw(leaf)) for key in keys[:n_always])
    together = tuple((key, False, draw(leaf)) for key in keys[n_always : n_always + 2])
    alone = tuple((key, False, draw(leaf)) for key in keys[n_always + 2 : n_always + 2 + n_alone])
    return Template("record", fields=always + together + alone, together=tuple(key for key, _, _ in together))


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
        grouped = draw(st.booleans(), label="include the grouped fields") if template.together else False
        out: dict[str, Any] = {}
        for key, always, child in template.fields:
            if key in template.together:
                present = grouped
            else:
                present = always or draw(st.booleans(), label=f"include {key!r}")
            if present:
                out[key] = draw(instances(child))
        return out
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
    shapes = ["record", "variants", "variants", "paired", "paired", "records", "tuple", "any"]
    shape = draw(st.sampled_from(shapes), label="root shape")
    if shape == "record":
        return draw(record_templates(2, min_fields=1))
    if shape == "paired":
        return draw(paired_templates())
    if shape == "records":
        return Template("list", children=(draw(record_templates(2, min_fields=1)),))
    if shape == "tuple":
        return Template("tuple", children=(Template("category", values=(0, 1, 7)), Template("date")), fixed=True)
    if shape == "variants":  # Records with disjoint fields, so that grouping Techniques split them
        keys = draw(st.permutations(KEYS), label="variant fields")
        cut = draw(st.integers(1, 3), label="first variant's fields")
        first = draw(record_templates(1, keys=tuple(keys[:cut]), min_fields=1))
        second = draw(record_templates(1, keys=tuple(keys[cut : cut + 3]), min_fields=1))
        return Template("mix", children=(first, second))
    return draw(templates(2))


@st.composite
def collections(draw: st.DrawFn) -> list[Any]:
    root = draw(root_templates(), label="root template")
    size = st.integers(14, 30) if draw(st.booleans(), label="large collection") else st.integers(2, 12)
    docs = [draw(instances(root), label="document") for _ in range(draw(size, label="documents"))]
    docs += draw(st.lists(json_values(max_leaves=4), max_size=1), label="arbitrary documents")
    return docs


# ---------------------------------------------------------------------------
# Reaching the relaxation Decision_Point (the narrowing of Property 48)
# ---------------------------------------------------------------------------

ORIGINAL_INFER_MODEL = StructureAnalyzer.infer_model
TYPE_NAMES = ("null", "boolean", "integer", "number", "string", "array", "object")


def narrowing(data: st.DataObject) -> Callable[[StructureAnalyzer, Any], Any]:
    """``StructureAnalyzer.infer_model``, then, in about half the runs, one ``type`` Constraint narrowed."""

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
# Observing reads: tainted numbers and the value sources that hand them out
# ---------------------------------------------------------------------------

RESOURCE, THRESHOLD = "Resource_Parameter", "Learned_Threshold"


class Probe:
    """Records the Resource_Parameter and Learned_Threshold values a run obtains and uses (module docstring)."""

    def __init__(self) -> None:
        self.active = False
        self._quiet = 0
        self._reading = 0
        # name -> [(source, value)] handed out; name -> [value] used
        self.resource_reads: dict[str, list[tuple[str, int | float]]] = {}
        self.resource_uses: dict[str, list[int | float]] = {}
        self.threshold_reads: dict[str, list[int | float]] = {}
        self.threshold_uses: dict[str, list[int | float]] = {}

    @property
    def recording(self) -> bool:
        return self.active and self._quiet == 0

    @contextmanager
    def inspecting(self) -> Iterator[None]:
        """Values handed out or used inside are inspected, not read for the run."""
        self._quiet += 1
        try:
            yield
        finally:
            self._quiet -= 1

    @contextmanager
    def reading(self) -> Iterator[None]:
        """Inside ``ResourceReader.read``: the reader's own access."""
        self._reading += 1
        try:
            yield
        finally:
            self._reading -= 1

    def resource(self, name: str, value: Any, source: str) -> Any:
        """A Resource_Parameter value handed out by ``source``, recorded, as a tainted number."""
        plain = untainted(value)
        if self.recording:
            via = "ResourceReader.read" if source == "ResourceReader.value" and self._reading else source
            self.resource_reads.setdefault(name, []).append((via, plain))
        return taint(self, RESOURCE, name, plain)

    def threshold(self, name: str, value: Any) -> None:
        """A Learned_Threshold value handed out by the run's Model_Bundle."""
        if self.recording:
            self.threshold_reads.setdefault(name, []).append(untainted(value))

    def use(self, value: Tainted) -> None:
        if self.recording:
            uses = self.resource_uses if value.p31_kind == RESOURCE else self.threshold_uses
            uses.setdefault(value.p31_name, []).append(untainted(value))


class Tainted:
    """Marker base of the tainted number types (attributes set by :func:`taint`)."""

    p31_probe: Probe
    p31_kind: str
    p31_name: str


class TaintedInt(int, Tainted):
    pass


class TaintedFloat(float, Tainted):
    pass


def untainted(value: Any) -> Any:
    """The plain number behind a tainted one (recording nothing), else ``value``."""
    if isinstance(value, TaintedInt):
        return int.__int__(value)
    if isinstance(value, TaintedFloat):
        return float.__float__(value)
    return value


def taint(probe: Probe, kind: str, name: str, value: Any) -> Any:
    """``value`` as a tainted number of ``kind`` and ``name`` (non-numbers and booleans stay as they are)."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return value
    out: Tainted = TaintedInt(value) if isinstance(value, int) else TaintedFloat(value)
    out.p31_probe, out.p31_kind, out.p31_name = probe, kind, name
    return out


def _used(value: Any) -> Any:
    """An operand: a tainted one records its use; plain numbers come back."""
    if isinstance(value, Tainted):
        value.p31_probe.use(value)
        return untainted(value)
    return value


# Every operation that reads a number's value; each records a use, then computes on plain values.
_OPERATIONS: dict[str, Callable[..., Any]] = {
    "__add__": operator.add,
    "__radd__": lambda a, b: b + a,
    "__sub__": operator.sub,
    "__rsub__": lambda a, b: b - a,
    "__mul__": operator.mul,
    "__rmul__": lambda a, b: b * a,
    "__truediv__": operator.truediv,
    "__rtruediv__": lambda a, b: b / a,
    "__floordiv__": operator.floordiv,
    "__rfloordiv__": lambda a, b: b // a,
    "__mod__": operator.mod,
    "__rmod__": lambda a, b: b % a,
    "__divmod__": divmod,
    "__rdivmod__": lambda a, b: divmod(b, a),
    "__pow__": lambda a, b, m=None: pow(a, b) if m is None else pow(a, b, m),
    "__rpow__": lambda a, b, m=None: pow(b, a) if m is None else pow(b, a, m),
    "__lshift__": operator.lshift,
    "__rlshift__": lambda a, b: b << a,
    "__rshift__": operator.rshift,
    "__rrshift__": lambda a, b: b >> a,
    "__and__": operator.and_,
    "__rand__": lambda a, b: b & a,
    "__or__": operator.or_,
    "__ror__": lambda a, b: b | a,
    "__xor__": operator.xor,
    "__rxor__": lambda a, b: b ^ a,
    "__lt__": operator.lt,
    "__le__": operator.le,
    "__gt__": operator.gt,
    "__ge__": operator.ge,
    "__eq__": operator.eq,
    "__ne__": operator.ne,
    "__neg__": operator.neg,
    "__pos__": operator.pos,
    "__abs__": abs,
    "__invert__": operator.invert,
    "__bool__": bool,
    "__int__": int,
    "__float__": float,
    "__index__": operator.index,
    "__round__": lambda a, n=None: round(a) if n is None else round(a, n),
    "__trunc__": math.trunc,
    "__floor__": math.floor,
    "__ceil__": math.ceil,
    "__hash__": hash,
    "__repr__": repr,
    "__str__": str,
    "__format__": format,
}


def _operation(compute: Callable[..., Any]) -> Callable[..., Any]:
    def method(self: Tainted, *args: Any) -> Any:
        self.p31_probe.use(self)
        return compute(untainted(self), *map(_used, args))

    return method


for _cls, _base in ((TaintedInt, int), (TaintedFloat, float)):
    for _name, _compute in _OPERATIONS.items():
        if hasattr(_base, _name):
            setattr(_cls, _name, _operation(_compute))
    _cls.__copy__ = lambda self: self  # type: ignore[attr-defined]  # immutable: copies are the value itself
    _cls.__deepcopy__ = lambda self, memo: self  # type: ignore[attr-defined]


class ConfigurationValues(Mapping[str, Any]):
    """The Run_Configuration's ``resources``: each item access hands out a tainted value."""

    def __init__(self, probe: Probe, values: Mapping[str, int | float]) -> None:
        self._probe = probe
        self._values = dict(values)

    def __getitem__(self, name: str) -> Any:
        return self._probe.resource(name, self._values[name], "Run_Configuration")

    def __iter__(self) -> Iterator[str]:
        return iter(self._values)

    def __len__(self) -> int:
        return len(self._values)


def _plain_json(value: Any) -> Any:
    if isinstance(value, dict):
        return {key: _plain_json(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return type(value)(_plain_json(item) for item in value)
    return untainted(value)


@contextmanager
def observed(probe: Probe, config: RunConfiguration) -> Iterator[RunConfiguration]:
    """Patch the value sources for one run; yields the Run_Configuration to run with."""
    run_config = copy.copy(config)  # an equal copy (origins included) whose resources are spied on
    object.__setattr__(run_config, "resources", ConfigurationValues(probe, config.resources))

    reader_init = ResourceReader.__init__
    reader_value = ResourceReader.value
    reader_read = ResourceReader.read
    to_json = RunConfiguration.to_json
    default_value = SettingSpec.default_value
    snapshot = variants_module._resource_values
    resolve = ModelBundle.resolve
    threshold = ResolvedBundle.threshold
    record_resource = InferenceReport.record_resource
    record_threshold = InferenceReport.record_threshold

    def quietly(function: Callable[..., Any]) -> Callable[..., Any]:
        def inspect(*args: Any, **kwargs: Any) -> Any:
            with probe.inspecting():
                return function(*args, **kwargs)

        return inspect

    def value(self: ResourceReader, name: str) -> Any:
        return probe.resource(name, reader_value(self, name), "ResourceReader.value")

    def read(self: ResourceReader, name: str) -> Any:
        with probe.reading():
            return reader_read(self, name)

    def config_json(self: RunConfiguration) -> Any:
        with probe.inspecting():
            return _plain_json(to_json(self))

    def default(self: SettingSpec) -> Any:
        result = default_value(self)
        if RESOURCE_PARAMETERS.get(self.name) is self:
            return probe.resource(self.name, result, "SettingSpec.default_value")
        return result

    def resolved(self: ModelBundle, techniques: Mapping[str, bool]) -> ResolvedBundle:
        bundle = resolve(self, techniques)
        records: dict[str, ThresholdRecord] = {}
        for name, record in bundle.thresholds.items():
            tainted = dataclasses.replace(record)
            object.__setattr__(tainted, "value", taint(probe, THRESHOLD, name, record.value))
            records[name] = tainted
        object.__setattr__(bundle, "thresholds", MappingProxyType(records))
        return bundle

    def handed_out(self: ResolvedBundle, name: str) -> Any:
        result = threshold(self, name)
        if result is not None:
            probe.threshold(name, result)
        return result

    with ExitStack() as stack:
        for target, attribute, replacement in (
            (ResourceReader, "__init__", quietly(reader_init)),
            (ResourceReader, "value", value),
            (ResourceReader, "read", read),
            (RunConfiguration, "to_json", config_json),
            (SettingSpec, "default_value", default),
            (variants_module, "_resource_values", quietly(snapshot)),
            (ModelBundle, "resolve", resolved),
            (ResolvedBundle, "threshold", handed_out),
            (InferenceReport, "record_resource", quietly(record_resource)),
            (InferenceReport, "record_threshold", quietly(record_threshold)),
        ):
            stack.enter_context(mock.patch.object(target, attribute, replacement))
        probe.active = True
        try:
            yield run_config
        finally:
            probe.active = False


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def check_resources(setup: Setup, probe: Probe, report: InferenceReport) -> None:
    """20.5: every Resource_Parameter read is listed, exactly, with the run's value and its origin."""
    listed = report.resources
    effective = setup.resources
    read = probe.resource_reads
    for name in sorted(set(read) | set(probe.resource_uses)):
        sources = sorted({source for source, _ in read.get(name, [])}) or ["a value obtained by an inspection"]
        assert name in listed, (
            f"Resource_Parameter {name} was read during the run (through {', '.join(sources)}), "
            f"but the report lists only {sorted(listed)}"
        )
    for name, use in listed.items():
        assert name in RESOURCE_PARAMETERS, f"the report lists the unregistered Resource_Parameter {name!r}"
        assert name in read, f"the report lists {name}, which the run never read"
        expected = effective[name]
        assert use.value == expected and type(use.value) is type(expected), (
            f"{name} is listed as {use.value!r}, the run's value is {expected!r}"
        )
        origin = setup.origin(name)
        assert use.origin == origin == setup.config.provenance_of[name], (
            f"{name} is listed with origin {use.origin!r}; it was supplied as {origin!r}"
        )
        assert use.user_supplied == (origin != "default")
        for source, value in read[name]:
            assert value == expected and type(value) is type(expected), (
                f"{name} was read through {source} as {value!r}; the run's value is {expected!r}"
            )
        for value in probe.resource_uses.get(name, []):
            assert value == expected, f"{name} was used as {value!r}; the run's value is {expected!r}"
        event(f"Resource_Parameter listed with origin {origin}")
    assert report.to_json()["resources"] == {name: use.to_json() for name, use in listed.items()}
    unread = sorted(set(RESOURCE_PARAMETERS) - set(listed))
    note(f"Resource_Parameters read: {sorted(listed)}; not read: {unread}")


def check_thresholds(setup: Setup, probe: Probe, report: InferenceReport) -> None:
    """20.5: every Learned_Threshold read is listed, with the stored value and the supplying bundle's id."""
    listed = report.thresholds
    held = setup.bundle.values
    for name in sorted(probe.threshold_uses):
        assert name in listed, (
            f"Learned_Threshold {name} was used during the run, but the report lists only {sorted(listed)}"
        )
    for name, use in listed.items():
        assert name in THRESHOLDS, f"the report lists the unregistered Learned_Threshold {name!r}"
        assert name in probe.threshold_reads, f"the report lists {name}, which the Model_Bundle never supplied"
        assert name in held, f"the report lists {name}, which bundle {setup.bundle_name} does not hold"
        assert use.value == held[name] and type(use.value) is type(held[name]), (
            f"{name} is listed as {use.value!r}; bundle {setup.bundle_name} holds {held[name]!r}"
        )
        assert use.bundle_id == setup.bundle.bundle_id, (
            f"{name} is listed as supplied by {use.bundle_id!r}, not {setup.bundle.bundle_id!r}"
        )
        assert any(setup.config.techniques[tid] for tid in THRESHOLDS[name].techniques), (
            f"{name} is listed, but none of its Techniques {THRESHOLDS[name].techniques} is enabled"
        )
        for value in probe.threshold_reads[name] + probe.threshold_uses.get(name, []):
            assert value == held[name], f"{name} was read as {value!r}; the bundle holds {held[name]!r}"
        event(f"Learned_Threshold {name} listed")
    if not listed:
        event("no Learned_Threshold read")


def thresholds_at(tid: str, kind: str) -> list[str]:
    """The Learned_Thresholds Technique ``tid`` reads at Decision_Point ``kind``."""
    return [name for name in REGISTRY.technique(tid).thresholds if THRESHOLDS[name].decision_point == kind]


def score_json(scores: Any) -> list[Any]:
    return [score.to_json() for score in scores]


def check_cutoff(
    kind: str,
    resolution: Resolution,
    cutoff: Cutoff | None,
    details: Mapping[str, Any],
    report: InferenceReport,
    where: str,
) -> None:
    """20.1, 20.7 for one resolved Decision_Point (see the module docstring, Checks)."""
    spec = REGISTRY.decision_point(kind)
    method = resolution.method
    selected = resolution.selected_candidate
    if cutoff is None:
        if method == "fallback":
            assert isinstance(spec.fallback, ConstantOutcome), f"{where}: a Fallback_Behavior that is not constant"
            event("no cut-off: constant Fallback_Behavior")
            return
        assert method == "priority", f"{where}: a {method!r} resolution records no cut-off"
        tid = selected.technique
        assert tid is not None, f"{where}: a priority selection of no Technique's candidate"
        applied = thresholds_at(tid, kind)
        assert not applied, f"{where}: {tid} applies {applied} at {kind}, but the record says no cut-off was applied"
        criteria = [name for name, tids in MODEL_SELECTION_CRITERIA.items() if tid in tids]
        assert not criteria, f"{where}: {tid} selects by {criteria}, but the record says no cut-off was applied"
        event("no cut-off: priority selection without a Learned_Threshold")
        return

    source = cutoff.source
    kinds = [
        source == MDL_COMPARISON,
        source == DECISION_MODEL_COMPARISON,
        source in MODEL_SELECTION_CRITERIA,
        source in THRESHOLDS,
    ]
    assert sum(kinds) == 1, f"{where}: cut-off source {source!r} is not exactly one of the four sources of 20.1"
    assert source not in RESOURCE_PARAMETERS, f"{where}: the Resource_Parameter {source} is used as a cut-off"
    assert method != "fallback", f"{where}: a constant Fallback_Behavior records the cut-off {source!r}"
    compared = score_json(cutoff.scores)
    candidates = score_json(resolution.candidates)

    if source == MDL_COMPARISON:
        assert cutoff.value is None and cutoff.scores and cutoff.artifact is None, f"{where}: {cutoff.to_json()}"
        for score in cutoff.scores:
            assert score.score is not None and math.isfinite(score.score) and score.score >= 0.0, (
                f"{where}: an MDL comparison with a description length of {score.score!r}"
            )
        if method == "mdl":
            assert compared == candidates, f"{where}: the MDL comparison is not over the resolved candidates"
        else:  # 22.12: equal calibrated probabilities, resolved by MDL among the tied outcomes
            assert method == "decision-model", f"{where}: a {method!r} resolution with an MDL comparison"
            assert details.get("tie_break") == MDL_COMPARISON, f"{where}: an MDL comparison without a tie"
            best = max(c.score for c in resolution.candidates if c.score is not None)
            tied = sorted(c.outcome for c in resolution.candidates if c.score == best)
            assert sorted(s.outcome for s in cutoff.scores) == tied, f"{where}: not the tied outcomes {tied}"
            assert selected.outcome in tied, f"{where}: selected {selected.outcome!r}, not a tied outcome"
        event(f"MDL comparison ({method})")
    elif source == DECISION_MODEL_COMPARISON:
        assert method == "decision-model", f"{where}: a {method!r} resolution with a Decision_Model comparison"
        artifacts = {decision_model_artifact(kind, model_source) for model_source in DECISION_MODEL_SOURCES}
        assert kind in DECISION_MODEL_TECHNIQUES and cutoff.artifact in artifacts, (
            f"{where}: Decision_Model artifact {cutoff.artifact!r}, expected one of {sorted(artifacts)}"
        )
        assert compared == candidates, f"{where}: the probabilities are not those of the resolved candidates"
        assert selected.score == max(s.score for s in cutoff.scores if s.score is not None), (
            f"{where}: the selected outcome does not have the highest calibrated probability"
        )
        event("Decision_Model comparison")
    elif source in MODEL_SELECTION_CRITERIA:
        assert method == "priority", f"{where}: a {method!r} resolution with the criterion {source}"
        assert cutoff.value is None and cutoff.scores and cutoff.artifact is None, f"{where}: {cutoff.to_json()}"
        assert all(s.score is not None and math.isfinite(s.score) for s in cutoff.scores), (
            f"{where}: {source} scores {compared}"
        )
        assert selected.technique in MODEL_SELECTION_CRITERIA[source], (
            f"{where}: {source} cut-off for {selected.technique}, which does not select by it"
        )
        event(f"model-selection criterion {source}")
    else:
        threshold = THRESHOLDS[source]
        assert method == "priority", f"{where}: a {method!r} resolution with the Learned_Threshold {source}"
        assert cutoff.value is not None and not cutoff.scores and cutoff.artifact is None, (
            f"{where}: {cutoff.to_json()}"
        )
        assert threshold.decision_point == kind and selected.technique in threshold.techniques, (
            f"{where}: {source} serves {threshold.decision_point} for {threshold.techniques}; "
            f"selected {selected.technique} at {kind}"
        )
        listed = report.thresholds.get(source)
        assert listed is not None, f"{where}: the cut-off {source} is not listed in the report"
        assert listed.value == cutoff.value, f"{where}: cut-off {source}={cutoff.value!r}, listed as {listed.value!r}"
        event(f"Learned_Threshold cut-off {source}")


def check_decisions(result: PipelineResult) -> None:
    """20.1, 20.7 over every Decision_Point the run resolved."""
    report = result.report
    assert report.decisions, "a run resolves at least its run-level Decision_Points"
    for entry in report.decisions:
        where = f"the {entry.kind} decision at {entry.path} ({entry.resolution.method})"
        check_cutoff(entry.kind, entry.resolution, entry.cutoff, entry.details, report, where)
        event(f"{entry.kind} by {entry.resolution.method}")


def key(resolution: Resolution, cutoff: Cutoff | None, fallback: FallbackInfo | None) -> str:
    parts = [part if part is None else part.to_json() for part in (resolution, cutoff, fallback)]
    return json.dumps(parts, sort_keys=True, ensure_ascii=False)


def check_records(result: PipelineResult) -> None:
    """20.7: each Constraint's record carries the cut-off of the decision that produced it."""
    decisions = {key(e.resolution, e.cutoff, e.fallback) for e in result.report.decisions}
    relaxations = {key(e.resolution, None, None) for e in result.report.decisions if e.kind == "relaxation"}
    for cid, record in result.model.provenance.items():
        where = f"the record at {result.pointers.get(cid, cid)}"
        for change in record.changes:
            if change.resolution is not None:
                assert key(change.resolution, None, None) in relaxations, f"{where}: an unreported relaxation"
                event("relaxation recorded under changes")
        if record.resolution is None:
            assert record.cutoff is None, f"{where}: a cut-off {record.cutoff} without a resolved Decision_Point"
            continue
        assert key(record.resolution, record.cutoff, record.fallback) in decisions, (
            f"{where}: its resolution and cut-off {None if record.cutoff is None else record.cutoff.to_json()} "
            "are not those of a reported decision"
        )
        event(f"record with cut-off {'none' if record.cutoff is None else record.cutoff.source}")


def check_rejected(report: InferenceReport) -> None:
    """20.7: rejected co-occurrence candidates name the Learned_Threshold that rejected them, with its value."""
    for entry in report.rejected:
        if entry.category != RULES_CATEGORY:
            continue
        name, value = entry.details.get("threshold"), entry.details.get("threshold_value")
        where = f"the rejected rule {entry.candidate} at {entry.path}"
        assert name in COOCCURRENCE_THRESHOLDS, f"{where}: rejected by {name!r}"
        listed = report.thresholds.get(name)
        assert listed is not None, f"{where}: rejected by {name}, which the report does not list"
        assert value == listed.value, f"{where}: {name}={value!r}, listed as {listed.value!r}"
        event(f"rejected rule with cut-off {name}")


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("bundle", ["default", *STUB_THRESHOLDS])
@given(data=st.data())
def test_every_cutoff_and_every_threshold_and_resource_read_is_recorded(
    bundles: dict[str, Bundle], bundle: str, data: st.DataObject
) -> None:
    setup = data.draw(setups(bundles, bundle), label="setup")
    docs = data.draw(collections(), label="documents")
    note(f"flags {setup.flags}; file {setup.file}")
    event(f"MDL selection {'on' if setup.config.techniques[MDL] else 'off'}")

    probe = Probe()
    with observed(probe, setup.config) as run_config, mock.patch.object(
        StructureAnalyzer, "infer_model", narrowing(data)
    ):
        result = infer_documents(docs, run_config)
    note(f"schema: {result.schema.decode()}")

    check_resources(setup, probe, result.report)
    check_thresholds(setup, probe, result.report)
    check_decisions(result)
    check_records(result)
    check_rejected(result.report)
