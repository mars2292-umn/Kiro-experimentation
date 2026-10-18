"""Property 47: Order-insensitivity (tasks 19.5 and 19.10).

For any Document_Collection and any permutation of its documents, with no
sampling and change-point detection off, the inferred Schema_Models are
equal (27.6).

Each example runs the whole in-memory pipeline
(``pipeline.infer.infer_documents``, the function the public
``jsinfer.infer`` wraps) twice under one Run_Configuration: run A on the
collection in its drawn order and run B on a drawn permutation of it
(``st.permutations``). Document ``j`` of run B is document ``order[j]`` of
run A.

Preconditions (27.6)
    - Change-point detection is off: ``techniques.robust.change_point`` is
      pinned to false in every configuration, and both reports say that
      Schema_Versions were not applied (18.10).
    - No Technique operates on a sample (29.8): every ``sample.*``
      Resource_Parameter is drawn at or above the number of JSON values plus
      object members in the collection, which bounds what any Technique can
      be handed (documents, objects, Object_Signatures, distinct strings), or
      left at its default, which is larger still. The sizes vary, so the
      check also shows that a sample size that does not bind changes
      nothing. Neither run may record a SampleInfo in any
      Provenance_Record, which confirms the precondition held.

Checks
    - The Structure_Analyzer's models, captured before the soundness stage,
      are equal under 2.7, which locates a difference in analysis or in the
      soundness stage.
    - The property: the final Schema_Models are equal under 2.7 (every
      keyword but ``$schema`` and ``x-provenance``; ``anyOf``/``oneOf``
      branch order and ``prefixItems`` order count; ``enum``, ``required``,
      and ``type`` compare as sets).
    - The emitted schema bytes are identical. Equal models need not give
      equal bytes in general (2.7 compares ``enum`` as a set), but the
      Schema_Emitter writes canonical bytes, so they must here.
    - The explanation files are equal once the document indices that
      Provenance_Records carry are renumbered. The only such indices are a
      RelaxationRecord's triggering ``documents`` (3.6), which name
      documents by position and so legitimately change with the order; they
      are mapped from run B's numbering to run A's and re-sorted. Everything
      else (Supports, Confidences, candidate scores in bits, cut-offs,
      details) is a function of multisets and must be equal as it is.
    - The Inference_Reports are equal after the 27.12 exemptions
      (``InferenceReport.comparable_json``) and the same renumbering of the
      removal records in ``soundness[].removed``, which are then re-sorted
      (the report orders them by pointer, then by their record bytes, which
      hold the indices). This covers the ``decisions`` section (every
      resolved Decision_Point, its candidates, and its scores), the
      ``rejected`` candidates, the soundness counts, and the
      Resource_Parameters read.

Inputs
    Collections come from random templates: Records with one to three
    Variants (some tagged with a ``type`` constant, so split branches can be
    disjoint ``oneOf`` branches), fields always present, sometimes present,
    or present together with the other "together" fields of their Variant
    (Co_Occurrence_Rules, ``dependentRequired``), Maps with data-like, date,
    UUID, or :data:`tests.strategies.KEYS` keys, Lists, Tuples, mixed types,
    and recursive trees. Field names include :data:`~tests.strategies.KEYS`
    (the empty key, and keys whose code-point and UTF-16 orders differ), so
    key ids sorted in code-point order matter. Scalars come in families:
    small and bound-ladder integers (``2**63``, ``2**64``, ``-0.0``,
    integral floats), decimals, Closed_Enum-prone vocabularies with a rare
    extra value, Format strings, latent integer and decimal strings, free
    text, and any :data:`~tests.strategies.json_scalars`. Some collections
    repeat documents (multisets with counts above one) or add free-form
    :func:`~tests.strategies.json_values` documents.

    Configurations come from :func:`tests.strategies.setting_flags` over
    every implemented Technique (prerequisites respected) and the seed, with
    ``workers = 1``, varied ``ingest.chunk_size``, ``soundness.max_passes``,
    MinHash, LSH, and EM settings, and the sample sizes above. A real
    Model_Bundle directory with hand-set Learned_Thresholds (each
    Technique's thresholds present together or absent) is written once per
    distinct threshold set, so LSH, DBSCAN, HDBSCAN, and co-occurrence rules
    run as well as their fallbacks. Scenarios turn ``decide.mdl`` off
    (priority resolution) and, in turn, put DBSCAN, HDBSCAN, or the
    Bernoulli mixture first in the ``variant_grouping`` priority order, so
    each grouping Technique's result is the one emitted in some runs.

    Collections hold at least two documents and the permutation is never the
    identity (a drawn identity becomes a rotation by one), so every example
    reorders something.

The soundness stage under a permutation
    The analyzer infers from every Observation, so its own models do not
    fail a document (no untightened run of this test has failed a first
    pass; the event ``first pass failed without tightening`` would show
    one), and the soundness stage would never relax anything. In about half
    of the examples the analyzer's model is therefore tightened before the
    soundness stage, as in Properties 3 and 7, with one to three over-tight
    value Constraints that some observed instance violates, possibly
    several at one node (``tests.records.complete_records`` restores the
    record invariant). The tightenings are drawn once, in run A, as
    (JSON Pointer, keyword, value) edits, and replayed unchanged and in the
    same order on run B's model, which the first check shows is equal to
    run A's. Both runs then hand the soundness stage the same model and the
    same multiset of documents in different orders.

    Several failing targets in one pass are processed in target order (the
    node's JSON Pointer, then keyword, then item; design Open Decisions,
    Soundness processing order; ``pipeline.soundness``), a function of the
    model as relaxed so far, and each relaxation decision is scored on its
    target's Observations and failing values (multisets), so the
    relaxations, removals, scores, cut-offs, and records must agree as
    well. Before task 19.10 the groups of a pass came in order of their
    first failing document, and the order of the documents decided which
    target was relaxed first: task 19.5 found 23 lists whose root the
    tightening gave ``maxItems: 0`` and ``minItems: 2`` that ended with
    ``maxItems`` removed in one order and loosened to 3 when the first
    document moved to the end (now a regression test in
    ``tests/unit/test_soundness.py``).

**Validates: Requirements 27.6**
"""

# Feature: json-schema-inference, Property 47: Order-insensitivity

from __future__ import annotations

import hashlib
import json
from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Any
from unittest import mock

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS, SAMPLE_SIZES
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import ThresholdRecord, threshold_spec
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE
from jsinfer.pipeline.infer import CHANGE_POINT_TECHNIQUES, PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.variants import (
    BERNOULLI_MIXTURE,
    DBSCAN_TECHNIQUE as DBSCAN,
    HDBSCAN_TECHNIQUE as HDBSCAN,
    MINHASH_LSH,
)
from tests.records import complete_records
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_scalars, json_values, setting_flags

MDL = "decide.mdl"

# ---------------------------------------------------------------------------
# Scalar families
# ---------------------------------------------------------------------------

_DATES = st.dates(min_value=date(1990, 1, 1), max_value=date(2030, 12, 31))

ENUM_SETS: tuple[tuple[Any, ...], ...] = (
    ("red", "green", "blue"),
    (1, 2, 3),
    ("A", "B"),
    (True, False),
    ("only",),
    ("é", "😀", ""),
)
ENUM_EXTRAS = st.sampled_from(["purple", 4, 1.5, None, "Red", 2.0])

FORMAT_VALUES: dict[str, st.SearchStrategy[str]] = {
    "date": _DATES.map(date.isoformat),
    "date-time": _DATES.map(lambda d: f"{d.isoformat()}T12:30:00Z"),
    "email": st.sampled_from(["a@example.com", "b.c@example.org", "x+y@sub.example.net"]),
    "uuid": st.uuids().map(str),
    "ipv4": st.ip_addresses(v=4).map(str),
    "uri": st.sampled_from(["https://example.com/a?b=1", "urn:isbn:0451450523", "ftp://h/p"]),
}
FORMAT_NEAR_MISSES = ("2024-02-30", "a@@example.com", "1.2.3", "not a uri", "")

NUMBER_FAMILIES: tuple[st.SearchStrategy[int | float], ...] = (
    st.integers(-12, 120),
    st.sampled_from([9, 10, 11, 99, 100, 101, 999, 1000, 1001, 1.0, 10.0, 1e3]),
    st.sampled_from([0.5, 0.99, 1.01, 9.95, 0.1, 0.3, 1.1, -2.675, 1e-7, 123.456]),
    st.sampled_from([2**31 - 1, 2**31, 2**53, 2**53 + 1, 2**63 - 1, 2**63, 2**64, 2**70, 2.0**70, -(2**63) - 1]),
    st.sampled_from([0, -0.0, 0.0, 1, -1, 1.0, -1.0]),
)

LATENT_VALUES = st.sampled_from(["007", "42", "-3", "0", "12345", "1.50", "0.0", "-0.25", "10", "99"])
LATENT_NEAR_MISSES = ("1e3", "12a", " 12", "+5", "4.")


def _rarely(common: st.SearchStrategy[Any], rare: st.SearchStrategy[Any], odds: int = 8) -> st.SearchStrategy[Any]:
    """``common``, except one draw in about ``odds`` from ``rare``."""
    return st.integers(0, odds - 1).flatmap(lambda roll: rare if roll == odds - 1 else common)


def scalar_values(family: str) -> st.SearchStrategy[Any]:
    kind, _, arg = family.partition(":")
    if kind == "format":
        return _rarely(FORMAT_VALUES[arg], st.sampled_from(FORMAT_NEAR_MISSES))
    if kind == "number":
        return NUMBER_FAMILIES[int(arg)]
    if kind == "enum":
        return _rarely(st.sampled_from(ENUM_SETS[int(arg)]), ENUM_EXTRAS, odds=12)
    if kind == "latent":
        return _rarely(LATENT_VALUES, st.sampled_from(LATENT_NEAR_MISSES))
    if kind == "text":
        return st.text(alphabet="ab😀 é\n-", max_size=5)
    assert kind == "any", family
    return json_scalars


SCALAR_FAMILIES = st.one_of(
    st.sampled_from(sorted(FORMAT_VALUES)).map(lambda name: f"format:{name}"),
    st.integers(0, len(NUMBER_FAMILIES) - 1).map(lambda i: f"number:{i}"),
    st.integers(0, len(ENUM_SETS) - 1).map(lambda i: f"enum:{i}"),
    st.sampled_from(["latent", "text", "any"]),
)

# ---------------------------------------------------------------------------
# Templates and documents
# ---------------------------------------------------------------------------

# Field presence within a Variant: always; drawn per object; or with every other "together" field of the Variant.
PRESENCE = ("always", "always", "sometimes", "together")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values.

    ``scalar``: ``family`` names the scalar family. ``record``: ``variants``
    holds, per Variant, its fields as ``(key, presence, child)``, and
    ``tags`` (if any) one ``type`` constant per Variant. ``map``: ``family``
    names the key family and ``children[0]`` the value template. ``list``:
    ``children[0]`` the element template. ``tuple``: one template per
    position. ``mix``: the alternatives. ``tree``: recursive nodes
    ``{"name", "children"}`` with ``children[0]`` the name template.
    """

    kind: str
    family: str = ""
    variants: tuple[tuple[tuple[str, str, Template], ...], ...] = ()
    children: tuple[Template, ...] = ()
    tags: tuple[Any, ...] = ()


TAG_KEY = "type"
FIELD_KEYS = tuple(dict.fromkeys((*KEYS, "name", "kind", "value", "when", "tags", "meta", "count")))
MAP_KEY_FAMILIES = ("data", "date", "uuid", "small")
_KINDS = ("scalar", "scalar", "scalar", "record", "record", "map", "list", "tuple", "mix", "tree")


def child_templates(depth: int) -> st.SearchStrategy[Template]:
    return templates(depth - 1) if depth > 0 else SCALAR_FAMILIES.map(lambda family: Template("scalar", family))


@st.composite
def record_templates(draw: st.DrawFn, depth: int) -> Template:
    """One to three Variants of mostly disjoint key sets, maybe with one key shared by all of them."""
    n_variants = draw(st.sampled_from([1, 1, 2, 3]), label="variants")
    order = draw(st.permutations(FIELD_KEYS), label="key order")
    shared = order[:1] if n_variants > 1 and draw(st.booleans(), label="a shared key") else ()
    rest = order[len(shared) :]
    variants = []
    start = 0
    for _ in range(n_variants):
        size = draw(st.integers(1 if n_variants > 1 else 0, 4), label="own keys")
        own = rest[start : start + size]
        start += size
        fields = tuple(
            (key, draw(st.sampled_from(PRESENCE), label="presence"), draw(child_templates(depth), label=repr(key)))
            for key in (*shared, *own)
        )
        variants.append(fields)
    tags: tuple[Any, ...] = ()
    if n_variants > 1 and draw(st.booleans(), label="tagged"):
        values = st.lists(st.sampled_from(["a", "b", "c", 1, 2, True]), min_size=n_variants, max_size=n_variants, unique=True)
        tags = tuple(draw(values, label="tags"))
    return Template("record", variants=tuple(variants), tags=tags)


@st.composite
def templates(draw: st.DrawFn, depth: int) -> Template:
    kind = draw(st.sampled_from(_KINDS if depth > 0 else ("scalar",)), label="kind")
    if kind == "scalar":
        return Template("scalar", draw(SCALAR_FAMILIES, label="family"))
    if kind == "record":
        return draw(record_templates(depth))
    if kind == "map":
        family = draw(st.sampled_from(MAP_KEY_FAMILIES), label="map keys")
        return Template("map", family, children=(draw(child_templates(depth)),))
    if kind == "list":
        return Template("list", children=(draw(child_templates(depth)),))
    if kind == "tuple":
        return Template("tuple", children=tuple(draw(st.lists(child_templates(depth), min_size=1, max_size=3))))
    if kind == "mix":
        return Template("mix", children=tuple(draw(st.lists(child_templates(depth), min_size=2, max_size=3))))
    assert kind == "tree"
    return Template("tree", children=(Template("scalar", draw(SCALAR_FAMILIES, label="tree names")),))


def map_keys(family: str) -> st.SearchStrategy[str]:
    if family == "data":
        return st.integers(0, 999).map(lambda i: f"u{i}")
    if family == "date":
        return _DATES.map(date.isoformat)
    if family == "uuid":
        return st.uuids().map(str)
    return st.sampled_from(KEYS)


@st.composite
def instances(draw: st.DrawFn, template: Template, depth: int = 3) -> Any:
    """A value of ``template``; ``depth`` bounds the recursion of trees."""
    kind = template.kind
    if kind == "scalar":
        return draw(scalar_values(template.family))
    if kind == "record":
        variant = draw(st.integers(0, len(template.variants) - 1), label="variant")
        together = draw(st.booleans(), label="together fields present")
        out: dict[str, Any] = {}
        for key, presence, child in template.variants[variant]:
            if presence == "always" or (presence == "together" and together) or (
                presence == "sometimes" and draw(st.booleans(), label=f"include {key!r}")
            ):
                out[key] = draw(instances(child, depth))
        if template.tags:
            out[TAG_KEY] = template.tags[variant]
        return out
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(map_keys(template.family), max_size=4), label="map keys"))
        return {key: draw(instances(template.children[0], depth)) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0], depth), max_size=3), label="elements")
    if kind == "tuple":
        full = len(template.children)
        length = draw(_rarely(st.just(full), st.integers(0, full), odds=4), label="tuple length")
        return [draw(instances(child, depth)) for child in template.children[:length]]
    if kind == "mix":
        return draw(instances(template.children[draw(st.integers(0, len(template.children) - 1))], depth))
    assert kind == "tree"
    node: dict[str, Any] = {"name": draw(instances(template.children[0], depth))}
    if depth > 0 and draw(st.integers(0, 3), label="children present") != 0:
        node["children"] = draw(st.lists(instances(template, depth - 1), max_size=2), label="subtrees")
    return node


@st.composite
def collections(draw: st.DrawFn) -> list[Any]:
    """Documents of one random template, sometimes with repeats and free-form documents."""
    shape = draw(st.sampled_from(["record", "record", "record", "array", "any"]), label="root shape")
    if shape == "record":
        root = draw(record_templates(3), label="root template")
    elif shape == "array":
        root = Template("list", children=(draw(record_templates(2)),))
    else:
        root = draw(templates(3), label="root template")
    docs = [draw(instances(root), label="document") for _ in range(draw(st.integers(2, 30), label="documents"))]
    if draw(st.integers(0, 3), label="repeated documents") == 0:
        picks = st.lists(st.integers(0, len(docs) - 1), min_size=1, max_size=6)
        docs += [docs[i] for i in draw(picks, label="repeats")]
    if draw(st.integers(0, 7), label="free-form documents") == 0:
        docs += draw(st.lists(json_values(max_leaves=8), min_size=1, max_size=2), label="free-form")
    return docs


def observation_bound(docs: Sequence[Any]) -> int:
    """JSON values plus object members over every document: at least what any Technique could sample (29.8)."""
    count = 0
    stack = list(docs)
    while stack:
        value = stack.pop()
        count += 1
        if isinstance(value, dict):
            count += len(value)
            stack.extend(value.values())
        elif isinstance(value, list):
            stack.extend(value)
    return count


# ---------------------------------------------------------------------------
# Configurations and the stub Model_Bundle
# ---------------------------------------------------------------------------

CHANGE_POINT = technique_setting("robust.change_point")
assert CHANGE_POINT_TECHNIQUES == ("robust.change_point",)

# Hand-set Learned_Thresholds inside their admissible ranges, with the extremes common.
THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    "lsh.keyset_jaccard": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
    "dbscan.eps": st.one_of(st.sampled_from([1e-9, 0.34, 0.5, 1.0]), st.floats(0.0, 1.0, exclude_min=True)),
    "dbscan.min_samples": st.integers(1, 6),
    "hdbscan.min_cluster_size": st.integers(2, 6),
    "cooccur.min_support": st.integers(1, 4),
    "cooccur.min_confidence": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
}
THRESHOLD_TECHNIQUES = tuple(
    tid for tid in REGISTRY.ids() if any(name in THRESHOLD_VALUES for name in REGISTRY.technique(tid).thresholds)
)

# Scenario → (Techniques disabled, the Technique enabled first in variant_grouping priority order, if any).
# In drawing order: "drawn" is the one Hypothesis shrinks towards.
SCENARIOS: dict[str, tuple[tuple[str, ...], str | None]] = {
    "drawn": ((), None),
    "priority": ((MDL,), None),
    "dbscan first": ((MDL, MINHASH_LSH), DBSCAN),
    "hdbscan first": ((MDL, MINHASH_LSH, DBSCAN), HDBSCAN),
    "mixture first": ((MDL, MINHASH_LSH, DBSCAN, HDBSCAN), BERNOULLI_MIXTURE),
}
assert REGISTRY.decision_point("variant_grouping").served_by == (MINHASH_LSH, DBSCAN, HDBSCAN, BERNOULLI_MIXTURE)


@dataclass(frozen=True)
class Setup:
    scenario: str
    flags: Mapping[str, Any]
    thresholds: Mapping[str, int | float]

    def config(self, bundles: BundleDir) -> RunConfiguration:
        return RunConfiguration.resolve({**self.flags, "model_bundle": str(bundles.path(self.thresholds))})


@st.composite
def setups(draw: st.DrawFn, bound: int) -> Setup:
    """Technique states, small resources, non-binding sample sizes (at least ``bound``), and thresholds."""
    scenario = draw(st.sampled_from(tuple(SCENARIOS)), label="scenario")
    disabled, first = SCENARIOS[scenario]
    perm = draw(st.sampled_from([1, 8, 32, 128]), label="minhash.perm")
    base: dict[str, Any] = {
        "workers": 1,
        CHANGE_POINT: False,
        "ingest.chunk_size": draw(st.sampled_from([1, 2, 3, 7, 2000]), label="ingest.chunk_size"),
        "soundness.max_passes": draw(st.sampled_from([1, 2, 5]), label="soundness.max_passes"),
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "variants.max_components": draw(st.integers(1, 6), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
    }
    for name in SAMPLE_SIZES:
        default = int(RESOURCE_PARAMETERS[name].default_value())
        base[name] = draw(st.one_of(st.integers(bound, max(bound, default)), st.just(default)), label=name)
    for tid in disabled:
        base[technique_setting(tid)] = False
    if first is not None:
        base[technique_setting(first)] = True
    flags = draw(setting_flags(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="flags")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    if first in present:
        present[first] = True
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    return Setup(scenario, flags, thresholds)


@dataclass(frozen=True)
class Case:
    """A collection, a permutation of it, the Run_Configuration both runs share, and whether to tighten."""

    documents: tuple[Any, ...]
    order: tuple[int, ...]  # document j of run B is document order[j] of run A
    setup: Setup
    tighten: bool

    def permuted(self) -> list[Any]:
        return [self.documents[i] for i in self.order]


@st.composite
def cases(draw: st.DrawFn) -> Case:
    docs = draw(collections(), label="collection")
    identity = list(range(len(docs)))
    order = draw(st.permutations(identity), label="permutation")
    if order == identity:  # reorder something: rotate by one (len(docs) >= 2)
        order = order[1:] + order[:1]
    setup = draw(setups(observation_bound(docs)), label="setup")
    tighten = draw(st.booleans(), label="tighten")
    return Case(tuple(docs), tuple(order), setup, tighten)


class BundleDir:
    """Model_Bundle directories of hand-set Learned_Thresholds, one per distinct threshold set."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.written: dict[str, Path] = {}

    def path(self, thresholds: Mapping[str, int | float]) -> Path:
        entries = {
            name: ThresholdRecord.from_spec(
                threshold_spec(name), value=value, objective_value=0.5, n_examples=1, training_data="p47-stub"
            ).to_json()
            for name, value in sorted(thresholds.items())
        }
        text = json.dumps(entries, sort_keys=True)
        digest = hashlib.blake2b(text.encode(), digest_size=8).hexdigest()
        if digest not in self.written:
            bundle = self.root / digest
            bundle.mkdir()
            manifest = {"artifacts": {}, "bundle_id": f"p47-{digest}", "format": BUNDLE_FORMAT}
            (bundle / MANIFEST_FILE).write_text(json.dumps(manifest))
            (bundle / THRESHOLDS_FILE).write_text(text)
            self.written[digest] = bundle
        return self.written[digest]


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> BundleDir:
    return BundleDir(tmp_path_factory.mktemp("p47-bundles"))


# ---------------------------------------------------------------------------
# The analyzer's models, and over-tight models for the soundness stage
# ---------------------------------------------------------------------------

# Constraints that reject something an instance of the given JSON type usually is, by keyword (as in Property 7).
TIGHTENINGS: dict[str, tuple[tuple[str, Any], ...]] = {
    "string": (("maxLength", 0), ("minLength", 4), ("format", "email"), ("pattern", "^[a-z]{2}$"), ("enum", ["zz"])),
    "integer": (("maximum", -5), ("minimum", 3), ("exclusiveMaximum", 0), ("const", 7), ("type", ["string"])),
    "number": (("maximum", 0.25), ("exclusiveMinimum", 1), ("enum", [0.5]), ("type", ["integer"])),
    "array": (("maxItems", 0), ("minItems", 2), ("uniqueItems", True)),
    "object": (("maxProperties", 0), ("minProperties", 3), ("required", ["zz-missing"])),
    "boolean": (("const", True), ("type", ["null"])),
    "null": (("type", ["boolean"]),),
}


def _nested(pointer: str, other: str) -> bool:
    """Whether one of two node pointers is the other or lies inside it."""
    return pointer == other or pointer.startswith(other + "/") or other.startswith(pointer + "/")


def _types_of(node: SchemaNode) -> list[str]:
    listed = node.kw.get("type", [])
    return list(listed) if isinstance(listed, list) else [listed]


@dataclass
class AnalyzerTap:
    """Wraps ``StructureAnalyzer.infer_model``: keeps each run's analyzer model, and tightens it when asked.

    ``analyzed`` holds the 2.7 equality key of the model of each run, taken
    before any tightening. With ``data``, the first run draws a plan of one
    to three (JSON Pointer, keyword, value) edits at typed schema objects of
    its model, and every run applies that same plan in the same order.
    """

    data: st.DataObject | None
    analyzed: list[bytes] = field(default_factory=list)
    plan: list[tuple[str, str, Any]] | None = None

    @contextmanager
    def installed(self) -> Iterator[None]:
        original = StructureAnalyzer.infer_model
        tap = self

        def infer_model(analyzer: StructureAnalyzer, view: View) -> SchemaModel:
            model = original(analyzer, view)
            tap.analyzed.append(model.equality_key())
            if tap.data is not None:
                tap.tighten(model)
            return model

        with mock.patch.object(StructureAnalyzer, "infer_model", infer_model):
            yield

    def _draw_plan(self, model: SchemaModel) -> list[tuple[str, str, Any]]:
        """One to three edits, drawn on the untouched model (see the module docstring).

        After the first edit, each edit is at the first edit's node, at a node
        above or below it, or anywhere, so that relaxing one target often
        changes how another is scored.
        """
        assert self.data is not None
        nodes = [
            (pointer, node)
            for pointer, node in model.walk()
            if node.boolean is None and any(t in TIGHTENINGS for t in _types_of(node))
        ]
        if not nodes:
            return []
        plan: list[tuple[str, str, Any]] = []
        for _ in range(self.data.draw(st.integers(1, 3), label="tightenings")):
            pool = nodes
            if plan:
                first = plan[0][0]
                where = self.data.draw(st.sampled_from(["same node", "above or below", "anywhere"]), label="where")
                if where == "same node":
                    pool = [(pointer, node) for pointer, node in nodes if pointer == first]
                elif where == "above or below":
                    pool = [(pointer, node) for pointer, node in nodes if _nested(pointer, first)]
            pointer, node = self.data.draw(st.sampled_from(pool), label="tightened node")
            options = [option for t in _types_of(node) if t in TIGHTENINGS for option in TIGHTENINGS[t]]
            keyword, value = self.data.draw(st.sampled_from(options), label="tightening")
            plan.append((pointer, keyword, value))
        return plan

    def tighten(self, model: SchemaModel) -> None:
        if self.plan is None:
            self.plan = self._draw_plan(model)
        elif self.analyzed[-1] != self.analyzed[0]:
            return  # the models already differ, which the analyzer check reports
        for pointer, keyword, value in self.plan:
            node = model.resolve(pointer)
            if keyword == "required":
                value = sorted({*node.kw.get("required", []), *value})
            node.kw[keyword] = json.loads(json.dumps(value))  # a fresh copy per run: soundness edits in place
        complete_records(model)  # 26.1: the injected Constraints get records, as a component's would


# ---------------------------------------------------------------------------
# Comparison helpers
# ---------------------------------------------------------------------------


def renumber_change(change: Mapping[str, Any], to_a: Sequence[int]) -> dict[str, Any]:
    """A RelaxationRecord's JSON with its triggering ``documents`` mapped to run A's numbering and sorted."""
    return {**change, "documents": sorted(to_a[doc] for doc in change["documents"])}


def renumbered_explanation(result: PipelineResult, to_a: Sequence[int]) -> dict[str, Any]:
    """The explanation file's JSON with every record's change documents in run A's numbering."""
    data = result.explanation.to_json()
    for record in data["records"].values():
        if "changes" in record:
            record["changes"] = [renumber_change(change, to_a) for change in record["changes"]]
    return data


def _removal_key(entry: Mapping[str, Any]) -> tuple[str, str]:
    return entry["pointer"], json.dumps(entry["change"], sort_keys=True)


def renumbered_report(result: PipelineResult, to_a: Sequence[int]) -> dict[str, Any]:
    """``comparable_json`` with removal records in run A's numbering, re-sorted by pointer and record."""
    data = result.report.comparable_json()
    for summary in data["soundness"]:
        removed = [{**entry, "change": renumber_change(entry["change"], to_a)} for entry in summary["removed"]]
        summary["removed"] = sorted(removed, key=_removal_key)
    return data


def first_difference(a: Any, b: Any, where: str = "") -> str | None:
    """The JSON Pointer of the first place where ``a`` and ``b`` differ, with both values; None when equal."""
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            token = where + "/" + str(key).replace("~", "~0").replace("/", "~1")
            if key not in a or key not in b:
                return f"{token}: only in run {'A' if key in a else 'B'}"
            found = first_difference(a[key], b[key], token)
            if found is not None:
                return found
        return None
    if isinstance(a, list) and isinstance(b, list):
        for index, (x, y) in enumerate(zip(a, b)):
            found = first_difference(x, y, f"{where}/{index}")
            if found is not None:
                return found
        return None if len(a) == len(b) else f"{where}: {len(a)} entries in run A, {len(b)} in run B"
    if a == b and type(a) is type(b):
        return None
    return f"{where or '(root)'}: run A {json.dumps(a)[:300]} vs run B {json.dumps(b)[:300]}"


def check_preconditions(result: PipelineResult, run: str) -> None:
    """27.6's conditions held: no change-point detection and no Technique on a sample (29.8)."""
    assert not result.config.techniques[CHANGE_POINT_TECHNIQUES[0]], f"run {run}: change-point detection is on"
    versions = result.report.versions
    assert versions is not None and not versions.applied, f"run {run}: Schema_Versions were applied"
    sampled = sorted(str(cid) for cid, record in result.model.provenance.items() if record.sample is not None)
    assert not sampled, f"run {run}: Constraints derived from a sample: {sampled}"


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------

_EVENT_KEYWORDS = (
    "anyOf",
    "oneOf",
    "dependentRequired",
    "prefixItems",
    "propertyNames",
    "additionalProperties",
    "enum",
    "const",
    "format",
    "pattern",
    "x-latent-type",
)


def note(case: Case, a: PipelineResult, tap: AnalyzerTap) -> None:
    event(f"scenario {case.setup.scenario}")
    if len({json.dumps(doc, sort_keys=True) for doc in case.documents}) < len(case.documents):
        event("repeated documents")
    for entry in a.report.to_json()["decisions"]:
        resolution = entry["resolution"]
        if entry["kind"] == "variant_grouping":
            chosen = resolution["candidates"][resolution["selected"]].get("technique")
            event(f"variant grouping by {chosen} ({resolution['method']})")
        elif entry["kind"] == "merge_or_split" and resolution["selected"] == 1:
            event("Variants split")
    for tid in sorted({record.technique for record in a.model.provenance.values()}):
        event(f"emitted record by {tid}")
    keywords = {keyword for _, node in a.model.walk() if node.boolean is None for keyword in node.kw}
    for keyword in _EVENT_KEYWORDS:
        if keyword in keywords:
            event(f"emitted {keyword}")
    if tap.plan:
        event("model tightened before soundness")
        if len(tap.plan) > 1:
            event("several Constraints tightened")
        if len({pointer for pointer, _, _ in tap.plan}) < len(tap.plan):
            event("one node tightened more than once")
    summary = a.soundness.summary
    if summary.failed_first_pass:
        event("first pass failed" + ("" if tap.plan else " without tightening"))
    if summary.relaxations:
        event("relaxations applied")
    targets: dict[tuple[str, int], set[tuple[Any, ...]]] = {}
    for applied in a.soundness.changes:
        targets.setdefault((applied.phase, applied.pass_index), set()).add(applied.failure.target)
    if any(len(found) > 1 for found in targets.values()):
        event("several targets changed in one pass")
    if summary.removed:
        event("Constraints removed")


@given(case=cases(), data=st.data())
def test_permuting_the_documents_gives_an_equal_schema_model(
    bundles: BundleDir, case: Case, data: st.DataObject
) -> None:
    config = case.setup.config(bundles)
    tap = AnalyzerTap(data if case.tighten else None)
    with tap.installed():
        a = infer_documents(list(case.documents), config)
        b = infer_documents(case.permuted(), config)
    n = len(case.documents)
    assert len(a.documents) == len(b.documents) == n, "every generated document is valid JSON and is ingested"
    check_preconditions(a, "A")
    check_preconditions(b, "B")

    schemas = f"run A schema:\n{a.schema.decode()}\nrun B schema:\n{b.schema.decode()}\norder: {case.order}"
    analyzed_a, analyzed_b = (json.loads(key) for key in tap.analyzed)
    assert analyzed_a == analyzed_b, (
        "the Structure_Analyzer's models differ under a permutation of the documents (27.6): "
        f"{first_difference(analyzed_a, analyzed_b)}\n{schemas}"
    )
    assert a.model == b.model, (
        "the Schema_Models differ under a permutation of the documents (27.6): "
        f"{first_difference(json.loads(a.schema), json.loads(b.schema))}\n{schemas}"
    )
    assert a.schema == b.schema, f"equal Schema_Models emitted different bytes\n{schemas}"

    to_a = case.order
    explanation_a = renumbered_explanation(a, range(n))
    explanation_b = renumbered_explanation(b, to_a)
    assert explanation_a == explanation_b, (
        "the explanation files differ beyond document numbering: "
        f"{first_difference(explanation_a, explanation_b)}\n{schemas}"
    )
    report_a = renumbered_report(a, range(n))
    report_b = renumbered_report(b, to_a)
    assert report_a == report_b, (
        f"the Inference_Reports differ beyond document numbering: {first_difference(report_a, report_b)}\n{schemas}"
    )

    note(case, a, tap)
