"""Property 7: Soundness on non-flagged documents (task 16.8).

For any Document_Collection and any valid Run_Configuration (random
Technique toggles that respect prerequisites, small
``soundness.max_passes``, and sampling enabled), every parsed document that
is not a Flagged_Document validates, with ``format`` asserted, against the
schema of its covering Schema_Version (or the single schema), and against
the combined schema when versions are emitted.

The test runs the whole in-memory pipeline (``pipeline.infer.infer_documents``)
and validates the emitted bytes, re-parsed, with the project's Validator
(which always asserts ``format``).

Reference
    The documents checked are the raw generated values, not the pipeline's
    DocumentStore. A value is parsed exactly when ``json.dumps`` can write it
    and RFC 8259 allows the text: no NaN or infinite float and no unpaired
    surrogate in a string or key (:func:`ingestible`, written from the
    Ingestor's documented rules, not from its bookkeeping). Collections
    sometimes hold such values, so document ``i`` of the run is the ``i``-th
    parsed value. ``result.flagged`` (Flagged_Documents) is excluded. No
    outlier Technique is implemented yet, so nothing is flagged now; the
    exclusion is already in place for task 38.1.

Checks
    - Single schema (3.1; ``versions.applied`` false): every parsed
      non-flagged document validates against the emitted schema, and the
      report holds exactly one soundness summary, for the single schema.
    - 3.2: the soundness stage validated exactly the non-flagged documents,
      every one of them, whatever the ``sample.*`` sizes left out of a
      sample (``result.soundness.validated``).
    - 3.3 and 3.7: the final schema is valid for every such document after
      at most ``max_passes`` relaxation passes followed by the removal loop;
      ``max_passes`` is drawn from 1 to 3, so the removal loop runs alone
      (``max_passes = 1``) or after relaxation.
    - 8.3: validation follows ``$defs`` and local ``$ref``; every ``$ref`` of
      the emitted schema must resolve inside it. Shared_Definitions are not
      implemented yet (task 36.1), so no ``$ref`` is emitted now; the inputs
      already hold object subschemas that share property names at distinct
      Paths and recursive trees, so the clause is exercised when they land.
    - Schema_Versions (gated on ``report.versions.applied``, active after
      task 48.2): every such document validates against the combined schema
      and against its covering Schema_Version's schema. Following the
      design's Version outputs, the combined schema is the emitted
      ``result.schema``; Schema_Version ``i`` is the combined root's
      ``anyOf`` branch ``i`` with the hoisted ``$defs``, and covers the
      documents of segment ``i`` in Ordering_Key order (18.2–18.4, 18.6).

Inputs
    Documents come from a random template: Records with one to three
    Variants and optional fields, Maps with data-like, date, or UUID keys,
    Lists, Tuples, mixed types, recursive trees, two properties holding the
    same Record template, and free-form JSON values
    (``tests.strategies.json_values``). Scalar fields come in families that
    stress value Constraints: Format-looking strings (dates, date-times,
    times, durations, emails, hostnames, IPv4/IPv6, URIs, UUIDs) with rare
    near misses, numbers near bound-ladder rungs (9/10/11, 99/100/101,
    0.99/1.01, 2**31, 2**53, 2**63, 2**64, 2**70), integral floats (``1.0``,
    ``2.0 ** 70``), ``-0.0``, latent integer and decimal strings, and
    Closed_Enum-prone categories with a rare extra value.

    Configurations come from ``setting_flags`` (implemented Techniques only,
    prerequisites respected) with ``workers = 1``, ``soundness.max_passes``
    in 1–3, a varied ``ingest.chunk_size``, small ``sample.*`` sizes (so
    DBSCAN and HDBSCAN sample signatures; the other sizes take effect as
    their Techniques land), and small MinHash and EM settings. A real
    Model_Bundle directory with hand-set Learned_Thresholds (each
    Technique's thresholds present together or absent) is written once per
    distinct threshold set, so LSH, DBSCAN, HDBSCAN, and co-occurrence rules
    run under soundness as well as their fallbacks. Scenarios turn
    ``decide.mdl`` off (priority resolution keeps Variants split, 5.11) or
    put DBSCAN first, which makes ``anyOf``/``oneOf`` Variant branches
    common.

    Over-tight models: in about half the runs the Structure_Analyzer's
    model is tightened before the soundness stage, by adding value
    Constraints that some observed instance violates (``type``, ``enum``,
    ``const``, ``format``, ``pattern``, the ten bounds, ``required``,
    ``uniqueItems``). The analyzer infers from every non-flagged
    Observation, so its own models rarely fail the first pass; the
    tightening drives the relaxation passes and the removal loop through
    the real pipeline. The claim is the same: whatever the model was, every
    non-flagged document validates against what is emitted.

**Validates: Requirements 3.1, 3.2, 3.3, 3.7, 8.3**
"""

# Feature: json-schema-inference, Property 7: Soundness on non-flagged documents

from __future__ import annotations

import hashlib
import json
import math
from collections.abc import Iterator, Mapping, Sequence
from dataclasses import dataclass
from datetime import date, datetime, time, timezone
from pathlib import Path
from typing import Any
from unittest import mock
from urllib.parse import unquote

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import ThresholdRecord, threshold_spec
from jsinfer.core.paths import ELEM, KEY, parse_path
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import MANIFEST_FILE, THRESHOLDS_FILE
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.variants import DBSCAN_TECHNIQUE as DBSCAN, MINHASH_LSH
from tests.records import complete_records
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, decimal_scalars, json_values, setting_flags

# One Validator for the module, so compiled schemas and format verdicts are cached across examples.
VALIDATOR = Validator()
MDL = "decide.mdl"

# ---------------------------------------------------------------------------
# Scalar families
# ---------------------------------------------------------------------------

_DATES = st.dates(min_value=date(1990, 1, 1), max_value=date(2030, 12, 31))


def _date_time(d: date, t: time) -> str:
    return datetime.combine(d, t).isoformat().replace("+00:00", "Z")


# Per Format: values that satisfy it, and near misses one edit away (which the Validator may reject).
FORMAT_VALUES: dict[str, tuple[st.SearchStrategy[str], tuple[str, ...]]] = {
    "date": (_DATES.map(date.isoformat), ("2024-02-30", "2024-1-31", "2024-01-31 ", "20240131", "2024-13-01")),
    "date-time": (
        st.builds(_date_time, _DATES, st.times(timezones=st.just(timezone.utc))),
        ("2024-01-31T12:00:00", "2024-01-31 12:00:00Z", "2024-01-31T24:00:00Z", "2024-01-31T12:00Z"),
    ),
    "time": (
        st.times(timezones=st.just(timezone.utc)).map(lambda t: t.isoformat().replace("+00:00", "Z")),
        ("12:00:00", "25:00:00Z", "12:60:00Z", "1:00:00Z"),
    ),
    "duration": (st.sampled_from(["P1D", "PT1H30M", "P2W", "P1Y2M3DT4H5M6S", "PT0S"]), ("P", "1D", "PT", "P1H")),
    "email": (
        st.sampled_from(["a@example.com", "b.c@example.org", "x+y@sub.example.net", "u_1@example.io"]),
        ("a@", "a@@example.com", "@example.com", "a b@example.com"),
    ),
    "hostname": (
        st.sampled_from(["example.com", "a-b.example", "localhost", "xn--bcher-kva.example"]),
        ("-bad.example", "a_b.example", "a..b", "ex ample.com"),
    ),
    "ipv4": (st.ip_addresses(v=4).map(str), ("256.0.0.1", "1.2.3", "01.2.3.4", "1.2.3.4 ")),
    "ipv6": (st.ip_addresses(v=6).map(str), (":::1", "2001:db8::g", "1:2:3:4:5:6:7:8:9")),
    "uri": (
        st.sampled_from(["https://example.com/a?b=1", "urn:isbn:0451450523", "mailto:a@example.com", "ftp://h/p"]),
        ("//example.com", "not a uri", "example.com", "http://exa mple.com"),
    ),
    "uuid": (
        st.uuids().map(str),
        ("123e4567e89b12d3a456426614174000", "123e4567-e89b-12d3-a456-42661417400g", "{123e4567-e89b-12d3-a456-426614174000}"),
    ),
}
FORMAT_NAMES = tuple(FORMAT_VALUES)

# Numbers around bound-ladder rungs (16.2): extremes next to 1- and 2-significant-digit roundings, integral
# floats, the integer type limits, -0.0, and decimals whose shortest spelling is not their binary value.
NUMBER_FAMILIES: tuple[st.SearchStrategy[int | float], ...] = (
    st.integers(-12, 120),
    st.sampled_from([9, 10, 11, 99, 100, 101, 999, 1000, 1001, 1.0, 10.0, 1e3]),
    st.sampled_from([0.5, 0.99, 1.0, 1.01, 9.95, 99.5, 0.1, 0.3, 1.1, -2.675, 1e-7, 123.456]),
    st.sampled_from(
        [2**31 - 1, 2**31, 2**32, 2**53, 2**53 + 1, 2**63 - 1, 2**63, 2**64, 2**70, 2.0**70, -(2**63), -(2**63) - 1, 1e16]
    ),
    st.sampled_from([0, -0.0, 0.0, 1, -1, 1.0, -1.0]),
)

# Closed_Enum-prone categories, and the rare extra values outside them (15.3, 15.5).
ENUM_SETS: tuple[tuple[Any, ...], ...] = (
    ("red", "green", "blue"),
    (1, 2, 3),
    ("A", "B"),
    (True, False),
    ("only",),
    ("é", "😀", ""),
)
ENUM_EXTRAS = st.sampled_from(["purple", 4, 1.5, None, "Red", "", 2.0])

# Latent integer and decimal strings (13.3), with rare near misses.
LATENT_GOOD = st.sampled_from(["007", "42", "-3", "0", "12345", "1.50", "0.0", "-0.25", "10", "99"])
LATENT_NEAR = ("1e3", "12a", " 12", "+5", "\u0663", "4.", "", "1,5")

STRING_VALUES = st.text(alphabet="ab😀 é\n-", max_size=5)


def _rarely(common: st.SearchStrategy[Any], rare: st.SearchStrategy[Any], odds: int = 8) -> st.SearchStrategy[Any]:
    """``common``, except one draw in about ``odds`` from ``rare``.

    The rare branch is the largest roll, which Hypothesis draws less often than the smallest and shrinks away from.
    """
    return st.integers(0, odds - 1).flatmap(lambda roll: rare if roll == odds - 1 else common)


def scalar_values(family: str) -> st.SearchStrategy[Any]:
    kind, _, arg = family.partition(":")
    if kind == "format":
        good, near = FORMAT_VALUES[arg]
        return _rarely(good, st.sampled_from(near))
    if kind == "number":
        return NUMBER_FAMILIES[int(arg)]
    if kind == "enum":
        return _rarely(st.sampled_from(ENUM_SETS[int(arg)]), ENUM_EXTRAS, odds=12)
    if kind == "latent":
        return _rarely(LATENT_GOOD, st.sampled_from(LATENT_NEAR))
    if kind == "string":
        return STRING_VALUES
    if kind == "free":
        return json_values(max_leaves=6, scalars=decimal_scalars)
    assert kind == "any", family
    return decimal_scalars


SCALAR_FAMILIES = st.one_of(
    st.sampled_from(FORMAT_NAMES).map(lambda name: f"format:{name}"),
    st.integers(0, len(NUMBER_FAMILIES) - 1).map(lambda i: f"number:{i}"),
    st.integers(0, len(ENUM_SETS) - 1).map(lambda i: f"enum:{i}"),
    st.sampled_from(["latent", "string", "any", "free"]),
)

# ---------------------------------------------------------------------------
# Templates and documents
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Template:
    """A generator of JSON values.

    ``scalar``: ``family`` names the scalar family. ``record``: ``variants``
    holds, per Variant, its fields as ``(key, always, child)``. ``map``:
    ``family`` names the key family, ``children[0]`` the value template.
    ``list``: ``children[0]`` the element template. ``tuple``: one template
    per position. ``mix``: the alternatives. ``tree``: recursive nodes
    ``{"name", "children"}`` with ``children[0]`` the name template.
    ``shared``: properties ``billing`` and ``shipping`` both of the Record
    template ``children[0]``. A tagged Record (``tags``) gives each Variant
    a ``type`` member holding one constant per Variant, so its branches can
    be disjoint and emitted under ``oneOf`` (5.5).
    """

    kind: str
    family: str = ""
    variants: tuple[tuple[tuple[str, bool, Template], ...], ...] = ()
    children: tuple[Template, ...] = ()
    tags: tuple[Any, ...] = ()  # record: per Variant, the constant value of its TAG_KEY member (if any)


TAG_KEY = "type"
FIELD_KEYS = tuple(dict.fromkeys((*KEYS, "id", "name", "when", "tags", "kind", "value", "children", "meta")))
MAP_KEY_FAMILIES = ("data", "date", "uuid", "small")
_KINDS = ("scalar", "scalar", "scalar", "record", "record", "map", "list", "tuple", "mix", "tree", "shared")


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
        size = draw(st.integers(1 if n_variants > 1 else 0, 3), label="own keys")
        own = rest[start : start + size]
        start += size
        fields = tuple(
            (key, draw(st.integers(0, 2), label="always") != 0, draw(child_templates(depth), label=f"{key!r}"))
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
        options = draw(st.lists(child_templates(depth), min_size=2, max_size=3), label="alternatives")
        return Template("mix", children=tuple(options))
    if kind == "tree":
        return Template("tree", children=(Template("scalar", draw(SCALAR_FAMILIES, label="tree names")),))
    assert kind == "shared"
    return Template("shared", children=(draw(record_templates(depth - 1 if depth > 1 else 0)),))


def map_keys(family: str) -> st.SearchStrategy[str]:
    if family == "data":
        return st.integers(0, 999).map(lambda i: f"u{i}")
    if family == "date":
        return _rarely(_DATES.map(date.isoformat), st.sampled_from(FORMAT_VALUES["date"][1]))
    if family == "uuid":
        return _rarely(st.uuids().map(str), st.sampled_from(FORMAT_VALUES["uuid"][1]))
    return st.sampled_from(KEYS)


@st.composite
def instances(draw: st.DrawFn, template: Template, depth: int = 3) -> Any:
    """A value of ``template``; ``depth`` bounds the recursion of trees."""
    kind = template.kind
    if kind == "scalar":
        return draw(scalar_values(template.family))
    if kind == "record":
        variant = draw(st.integers(0, len(template.variants) - 1), label="variant")
        out = {
            key: draw(instances(child, depth))
            for key, always, child in template.variants[variant]
            if always or draw(st.booleans(), label=f"include {key!r}")
        }
        if template.tags:
            out[TAG_KEY] = template.tags[variant]
        return out
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(map_keys(template.family), max_size=4), label="map keys"))
        return {key: draw(instances(template.children[0], depth)) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0], depth), max_size=3), label="elements")
    if kind == "tuple":
        length = draw(st.integers(0, len(template.children)), label="tuple length")
        return [draw(instances(child, depth)) for child in template.children[:length]]
    if kind == "mix":
        return draw(instances(template.children[draw(st.integers(0, len(template.children) - 1))], depth))
    if kind == "tree":
        node: dict[str, Any] = {"name": draw(instances(template.children[0], depth))}
        if depth > 0 and draw(st.integers(0, 3), label="children present") != 0:
            node["children"] = draw(st.lists(instances(template, depth - 1), max_size=2), label="subtrees")
        if draw(st.integers(0, 5), label="meta") == 0:
            node["meta"] = draw(scalar_values("any"))
        return node
    assert kind == "shared"
    return {
        key: draw(instances(template.children[0], depth))
        for key in ("billing", "shipping")
        if draw(st.integers(0, 3), label=f"include {key}") != 0
    }


# Values the Ingestor skips: json.dumps cannot write non-finite floats with allow_nan=False, and an unpaired
# surrogate (written as a \\u escape) is not RFC 8259 text (1.6, 1.11).
REJECTED_VALUES: tuple[Any, ...] = (
    float("nan"),
    float("inf"),
    {"a": float("-inf")},
    [1, float("nan")],
    "\ud800",
    {"k\udc00": 1},
    ["x\udfff"],
)


def ingestible(value: Any) -> bool:
    """Whether the Ingestor parses ``value``: finite floats only, and no surrogate in any string or key."""
    stack = [value]
    while stack:
        item = stack.pop()
        if isinstance(item, float) and not math.isfinite(item):
            return False
        if isinstance(item, str) and any("\ud800" <= ch <= "\udfff" for ch in item):
            return False
        if isinstance(item, dict):
            stack.extend(item.keys())
            stack.extend(item.values())
        elif isinstance(item, list):
            stack.extend(item)
    return True


@dataclass(frozen=True)
class Collection:
    """The raw inputs, in order, and the template they come from (shown in failure reports)."""

    inputs: tuple[Any, ...]
    root: Template

    def parsed(self) -> list[Any]:
        """The inputs the Ingestor accepts, in order: document ``i`` of the run is ``parsed()[i]``."""
        return [value for value in self.inputs if ingestible(value)]


@st.composite
def collections(draw: st.DrawFn) -> Collection:
    shape = draw(st.sampled_from(["record", "record", "record", "array", "any"]), label="root shape")
    if shape == "record":
        root = draw(record_templates(3), label="root template")
    elif shape == "array":
        root = Template("list", children=(draw(record_templates(2)),))
    else:
        root = draw(templates(3), label="root template")
    docs = [draw(instances(root), label="document") for _ in range(draw(st.integers(1, 30), label="documents"))]
    if draw(st.integers(0, 7), label="odd documents") == 7:
        docs += draw(st.lists(json_values(max_leaves=8, scalars=decimal_scalars), min_size=1, max_size=2))
    if draw(st.integers(0, 4), label="rejected inputs") == 4:
        docs += draw(st.lists(st.sampled_from(REJECTED_VALUES), min_size=1, max_size=2))
    order = draw(st.permutations(range(len(docs))), label="document order")
    return Collection(tuple(docs[i] for i in order), root)


# ---------------------------------------------------------------------------
# Configurations and the stub Model_Bundle
# ---------------------------------------------------------------------------

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
# Every sample-size Resource_Parameter (29.8): those of unimplemented Techniques take effect as they land.
SAMPLE_PARAMETERS = tuple(sorted(name for name in RESOURCE_PARAMETERS if name.startswith("sample.")))
SCENARIOS = ("drawn", "priority", "dbscan first")


@st.composite
def resource_values(draw: st.DrawFn) -> dict[str, int]:
    """Small passes, varied chunks, small samples, and small MinHash and EM settings."""
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    values = {
        "workers": 1,
        "soundness.max_passes": draw(st.sampled_from([1, 2, 3]), label="soundness.max_passes"),
        "ingest.chunk_size": draw(st.sampled_from([1, 2, 3, 7, 2000]), label="ingest.chunk_size"),
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "variants.max_components": draw(st.integers(1, 6), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
    }
    for name in SAMPLE_PARAMETERS:  # mostly 1–3, so samples leave signatures out; sometimes the default
        default = st.just(RESOURCE_PARAMETERS[name].default_value())
        values[name] = draw(_rarely(st.integers(1, 3), default, odds=4), label=name)
    return values


@dataclass(frozen=True)
class Setup:
    scenario: str
    flags: Mapping[str, Any]
    thresholds: Mapping[str, int | float]
    tighten: bool

    def config(self, bundles: BundleDir) -> RunConfiguration:
        return RunConfiguration.resolve({**self.flags, "model_bundle": str(bundles.path(self.thresholds))})


@st.composite
def setups(draw: st.DrawFn) -> Setup:
    """Technique states, resources, thresholds present per Technique or absent, and whether to tighten.

    ``drawn`` varies every implemented Technique. ``priority`` turns ``decide.mdl`` off, so Judgment_Calls
    resolve by priority and Variant groupings stay split (5.11). ``dbscan first`` also takes LSH out and
    gives DBSCAN a small radius with ``min_samples`` 1, so its grouping (and its sample) is the one used.
    """
    scenario = draw(st.sampled_from(SCENARIOS), label="scenario")
    base: dict[str, Any] = dict(draw(resource_values(), label="resources"))
    if scenario != "drawn":
        base[technique_setting(MDL)] = False
    if scenario == "dbscan first":
        base[technique_setting(MINHASH_LSH)] = False
        base[technique_setting(DBSCAN)] = True
    flags = draw(setting_flags(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="flags")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    if scenario == "dbscan first":
        present[DBSCAN] = True
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    if scenario == "dbscan first":
        thresholds["dbscan.eps"] = draw(st.sampled_from([1e-9, 0.34, 0.5]), label="small dbscan.eps")
        thresholds["dbscan.min_samples"] = 1
    tighten = draw(st.booleans(), label="tighten")
    return Setup(scenario, flags, thresholds, tighten)


class BundleDir:
    """Model_Bundle directories of hand-set Learned_Thresholds, one per distinct threshold set."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.written: dict[str, Path] = {}

    def path(self, thresholds: Mapping[str, int | float]) -> Path:
        entries = {
            name: ThresholdRecord.from_spec(
                threshold_spec(name), value=value, objective_value=0.5, n_examples=1, training_data="p7-stub"
            ).to_json()
            for name, value in sorted(thresholds.items())
        }
        text = json.dumps(entries, sort_keys=True)
        digest = hashlib.blake2b(text.encode(), digest_size=8).hexdigest()
        if digest not in self.written:
            bundle = self.root / digest
            bundle.mkdir()
            manifest = {"artifacts": {}, "bundle_id": f"p7-{digest}", "format": 1}
            (bundle / MANIFEST_FILE).write_text(json.dumps(manifest))
            (bundle / THRESHOLDS_FILE).write_text(text)
            self.written[digest] = bundle
        return self.written[digest]


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> BundleDir:
    return BundleDir(tmp_path_factory.mktemp("p7-bundles"))


# ---------------------------------------------------------------------------
# Over-tight models
# ---------------------------------------------------------------------------


def _schema_objects(model: SchemaModel) -> list[SchemaNode]:
    """The keyword schema objects of the model, in pre-order (``$defs`` included)."""
    return [node for _, node in model.walk() if node.boolean is None]


def _types_of(node: SchemaNode) -> list[str]:
    listed = node.kw.get("type", [])
    return list(listed) if isinstance(listed, list) else [listed]


# Constraints that reject something an instance of the given JSON type usually is, by keyword.
_TIGHTENINGS: dict[str, tuple[tuple[str, Any], ...]] = {
    "string": (
        ("maxLength", 0),
        ("minLength", 4),
        ("format", "email"),
        ("format", "date"),
        ("pattern", "^[a-z]{2}$"),
        ("enum", ["zz-no-such-value"]),
    ),
    "integer": (("maximum", -5), ("minimum", 3), ("exclusiveMaximum", 0), ("const", 7), ("type", ["string"])),
    "number": (("maximum", 0.25), ("exclusiveMinimum", 1), ("enum", [0.5]), ("type", ["integer"])),
    "array": (("maxItems", 0), ("minItems", 2), ("uniqueItems", True)),
    "object": (("maxProperties", 0), ("minProperties", 3), ("required", ["zz-missing"])),
    "boolean": (("const", True), ("type", ["null"])),
    "null": (("type", ["boolean"]),),
}


@dataclass
class Tightener:
    """Wraps ``StructureAnalyzer.infer_model`` to add over-tight value Constraints to the inferred model."""

    data: st.DataObject
    applied: list[str]

    def install(self) -> Any:
        original = StructureAnalyzer.infer_model
        tightener = self

        def over_tight(analyzer: StructureAnalyzer, view: View) -> SchemaModel:
            model = original(analyzer, view)
            tightener.tighten(model)
            complete_records(model)  # 26.1: the injected Constraints get records, as a component's would
            return model

        return mock.patch.object(StructureAnalyzer, "infer_model", over_tight)

    def tighten(self, model: SchemaModel) -> None:
        """Add one to three over-tight value Constraints at typed schema objects of ``model``."""
        nodes = [node for node in _schema_objects(model) if any(t in _TIGHTENINGS for t in _types_of(node))]
        if not nodes:
            return
        for _ in range(self.data.draw(st.integers(1, 3), label="tightenings")):
            node = self.data.draw(st.sampled_from(nodes), label="tightened node")
            options = [option for t in _types_of(node) if t in _TIGHTENINGS for option in _TIGHTENINGS[t]]
            keyword, value = self.data.draw(st.sampled_from(options), label="tightening")
            if keyword == "required":
                value = sorted({*node.kw.get("required", []), *value})
            node.kw[keyword] = value
            self.applied.append(keyword)


# ---------------------------------------------------------------------------
# Schema_Versions (gated on versions being emitted; task 48.2)
# ---------------------------------------------------------------------------


def _instant(value: Any) -> datetime | None:
    """A date or date-time value as an aware UTC instant (``date`` values at midnight UTC), else None."""
    if not isinstance(value, str):
        return None
    try:
        if "T" in value or "t" in value:
            parsed = datetime.fromisoformat(value.replace("z", "Z"))
            return parsed.astimezone(timezone.utc) if parsed.tzinfo is not None else None
        return datetime.combine(date.fromisoformat(value), time(), timezone.utc)
    except ValueError:
        return None


def _value_at(doc: Any, path_text: str) -> Any:
    """The single Observation of ``doc`` at ``path_text`` (the Ordering_Key has one in every document)."""
    value = doc
    for step in parse_path(path_text):
        if step.kind == KEY:
            assert isinstance(value, dict) and step.name in value, f"no Observation at {path_text}"
            value = value[step.name]
        else:
            members = value if step.kind == ELEM else list(value.values())
            assert len(members) == 1, f"{len(members)} Observations at {path_text}"
            value = members[0]
    return value


def ordering(result: PipelineResult, parsed: Sequence[Any]) -> list[int]:
    """The document indices in Ordering_Key order (18.2–18.4): by UTC instant, then by index."""
    versions = result.report.versions
    assert versions is not None and versions.ordering_key is not None
    key = versions.ordering_key
    if key.path is None:
        return list(range(len(parsed)))
    instants = [_instant(_value_at(doc, key.path)) for doc in parsed]
    assert all(instant is not None for instant in instants), "an Ordering_Key value is not a date or date-time"
    return sorted(range(len(parsed)), key=lambda i: (instants[i], i))


def version_checks(result: PipelineResult, schema: Any, parsed: Sequence[Any]) -> Iterator[tuple[str, Any, int]]:
    """(label, schema, document index) for the combined schema and each covering Schema_Version (3.1)."""
    versions = result.report.versions
    assert versions is not None and versions.applied
    segments = versions.segments
    if not segments:  # no change point accepted: a single schema (18.9)
        yield from (("single schema", schema, i) for i in range(len(parsed)))
        return
    assert isinstance(schema, dict) and len(schema.get("anyOf", ())) == len(segments), (
        "the combined schema has one top-level anyOf branch per Schema_Version (18.8)"
    )
    order = ordering(result, parsed)
    position = {doc: rank for rank, doc in enumerate(order)}
    hoisted = {"$defs": schema["$defs"]} if "$defs" in schema else {}
    for v, segment in enumerate(segments):
        version_schema = {"$schema": schema["$schema"], **hoisted, **schema["anyOf"][v]}
        first, last = position[segment.first_document], position[segment.last_document]
        for doc in order[first : last + 1]:
            yield f"Schema_Version {v}", version_schema, doc
    yield from (("combined schema", schema, i) for i in range(len(parsed)))


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


def _refs(model: SchemaModel) -> Iterator[str]:
    """The ``$ref`` values of the model's schema objects (not ``$ref`` members of ``enum`` or ``const`` data)."""
    for _, node in model.walk():
        if node.boolean is None and isinstance(node.kw.get("$ref"), str):
            yield node.kw["$ref"]


def _resolve(schema: Any, ref: str) -> Any:
    """The subschema a local ``$ref`` names (``#`` plus a percent-encoded JSON Pointer)."""
    assert ref.startswith("#"), f"$ref {ref!r} is not local"
    target = schema
    pointer = unquote(ref[1:])
    for token in pointer.split("/")[1:] if pointer else ():
        token = token.replace("~1", "/").replace("~0", "~")
        target = target[int(token)] if isinstance(target, list) else target[token]
    return target


_EVENT_KEYWORDS = {
    "format": "format",
    "enum": "enum/const",
    "const": "enum/const",
    "pattern": "pattern",
    "type": "type",
    "required": "required",
    "dependentRequired": "dependentRequired",
    "additionalProperties": "additionalProperties",
    "items": "items",
    "anyOf": "anyOf/oneOf",
    "oneOf": "anyOf/oneOf",
}
_BOUND_KEYWORDS = frozenset(
    {"minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "minLength", "maxLength", "minItems", "maxItems",
     "minProperties", "maxProperties"}
)


def note(result: PipelineResult, setup: Setup, collection: Collection, applied: Sequence[str]) -> None:
    summary = result.soundness.summary
    event(f"scenario {setup.scenario}")
    event(f"soundness.max_passes {result.config.resources['soundness.max_passes']}")
    if applied:
        event("model tightened before soundness")
    if summary.failed_first_pass:
        event("first pass failed")
        if not applied:
            event("first pass failed without tightening")
    if summary.relaxations:
        event("relaxations applied")
    if any(change.phase == "removal" for change in result.soundness.changes):
        event("removal loop reached")
    if summary.pass_limit_reached:
        event("pass limit reached")
    for change in result.soundness.changes:
        keyword = change.failure.keyword
        kind = "bounds" if keyword in _BOUND_KEYWORDS else _EVENT_KEYWORDS.get(keyword, "other")
        event(f"{change.phase}: {kind}")
    if any(record.sample is not None for record in result.model.provenance.values()):
        event("sampled grouping emitted")
    if len(collection.parsed()) < len(collection.inputs):
        event("unparseable inputs skipped")
    keywords = {keyword for _, node in result.model.walk() if node.boolean is None for keyword in node.kw}
    for keyword in ("format", "enum", "const", "oneOf", "anyOf", "prefixItems", "dependentRequired", "$defs"):
        if keyword in keywords:
            event(f"emitted {keyword}")
    if "additionalProperties" in keywords:
        event("emitted additionalProperties")


@given(setup=setups(), collection=collections(), data=st.data())
def test_every_parsed_non_flagged_document_validates_against_its_covering_schema(
    bundles: BundleDir, setup: Setup, collection: Collection, data: st.DataObject
) -> None:
    config = setup.config(bundles)
    applied: list[str] = []
    if setup.tighten:
        with Tightener(data, applied).install():
            result = infer_documents(collection.inputs, config)
    else:
        result = infer_documents(collection.inputs, config)

    parsed = collection.parsed()
    n = len(parsed)
    assert len(result.documents) == n, f"{len(result.documents)} documents ingested, {n} parseable inputs"
    flagged = frozenset(result.flagged)
    assert all(0 <= i < n for i in flagged)
    schema = json.loads(result.schema)
    # 8.3: every $ref is local and resolves inside the emitted schema (validation follows them).
    for ref in _refs(result.model):
        target = _resolve(schema, ref)
        assert isinstance(target, (dict, bool)), f"$ref {ref!r} names no subschema"
        event("emitted $ref")

    # The Validator asserts format (3.1, 3.2): a non-email string fails {"format": "email"}.
    assert not VALIDATOR.is_valid(VALIDATOR.compile({"format": "email"}), "not an email")

    versions = result.report.versions
    if versions is not None and versions.applied:
        checks = list(version_checks(result, schema, parsed))
    else:
        # 18.10: one segment, so one soundness summary, for the single schema.
        (summary,) = result.report.soundness
        assert summary.version is None and summary == result.soundness.summary
        # 3.2: the stage validated every non-flagged document, whether or not a sample held it, and no flagged one.
        live = tuple(i for i in range(n) if i not in flagged)
        assert result.soundness.validated == live, "the soundness stage skipped or added documents"
        assert result.soundness.flagged == tuple(sorted(flagged))
        checks = [("single schema", schema, i) for i in range(n)]

    compiled: dict[int, Any] = {}
    for label, covering, i in checks:
        if i in flagged:
            continue
        key = id(covering)
        if key not in compiled:
            compiled[key] = VALIDATOR.compile(covering)
        errors = VALIDATOR.errors(compiled[key], parsed[i])
        assert not errors, (
            f"document {i} is not a Flagged_Document but fails the {label}:\n"
            f"document: {parsed[i]!r}\nerrors: {errors}\nschema: {result.schema.decode()}\n"
            f"soundness: {result.soundness.summary.to_json()}\ntightened: {applied}"
        )

    note(result, setup, collection, applied)
