"""Property 3: Schema emit/load round trip (tasks 6.4 and 16.10).

For any Schema_Model in the supported keyword grammar, and for any
Schema_Model produced by the Inferrer, emitting it with the Schema_Emitter and
loading the emitted document with the Schema_Loader yields a fully supported
Schema_Model equal to the original under the equality of Requirement 2.7.
Emitting the loaded model again gives the same bytes, so the emitted form is a
fixed point of load-then-emit.

Generated Schema_Models (task 6.4)
    Models come from :func:`tests.strategies.schema_models`: every keyword of
    Requirement 2.4 (local and recursive ``$ref`` into ``$defs`` and ancestors
    included), boolean schemas at every subschema position and at the root,
    the retained annotations, a root ``$id``, and ``x-`` keywords (2.10),
    with values in non-canonical form (repeats in set-compared lists,
    integral floats, ``-0.0``, big integers, and keywords in a drawn order).
    Loading the emitted bytes also runs the draft 2020-12 meta-schema check
    with formats asserted, so an emitted document the meta-schema rejects
    fails this test as a ``SchemaLoadError``.

Schema_Models produced by the Inferrer (task 16.10)
    Each example runs the whole pipeline (``pipeline.infer.infer_documents``)
    on a generated collection and round-trips ``result.model``, the
    Schema_Model after the soundness check. Checks:

    - emitting the model gives ``result.schema``, the pipeline's own bytes;
    - the loaded model is fully supported, records no ignored keyword, and
      has a declared dialect (the emitter writes ``$schema`` on every
      keyword root; a boolean root is read with an assumed dialect, which
      2.7 ignores);
    - the loaded model equals ``result.model`` under 2.7, and the two
      layouts put Constraints at the same JSON Pointers (the explanation
      file of 26.5 keys records by these pointers);
    - emitting the loaded model gives the same bytes again. Equal bytes
      compile to the same validator, so the loaded model accepts and rejects
      exactly the documents the inferred one does.

    "Including each Schema_Version": while change-point detection is
    unimplemented every run emits one schema. The per-Version loop of
    :func:`emitted_models` turns on when the report's ``versions`` section
    says detection was applied with segments (task 48.2), and then needs the
    per-Version Schema_Models on the result.

    Collections come from random templates (:func:`collections`): Records
    with one to three Variants (optionally with a ``const`` Discriminator
    field per Variant, so split branches are disjoint and become ``oneOf``),
    fields always present, sometimes present, or present together (so
    ``dependentRequired`` can be mined), Maps whose keys are data-like or
    satisfy a Format (``propertyNames``), Lists, Tuples (``prefixItems``,
    ``items: false``), mixed types, and scalars: bounded integers, decimals,
    big integers, booleans, nulls, small vocabularies (``enum``/``const``),
    free text, strings of each of the ten Formats, and latent integer and
    decimal strings (``x-latent-type``, ``pattern``). Property names include
    the awkward ones of :data:`tests.strategies.KEYS` plus names that look
    like keywords or need pointer escapes (``x-provenance``, ``$ref``,
    ``~1/``, ``%25``).

    Technique states come from ``run_configs()`` with ``workers = 1``,
    small Resource_Parameters, and ``soundness.max_passes`` of 1 (removal
    only), 2, or the default. In the ``priority`` scenario ``decide.mdl`` is
    off, so Judgment_Calls resolve by priority and groupings stay split
    (5.11). A real Model_Bundle directory is written per example with
    hand-set Learned_Thresholds, each Technique's thresholds present
    together or absent, so the grouping Techniques, co-occurrence mining,
    and their fallbacks all occur.

    Over-tight injections (:class:`Injection`) edit the analyzer's model
    before the soundness check (``StructureAnalyzer.infer_model`` is
    patched, as ``tests/unit/test_pipeline_infer.py`` does), so relaxation
    edits reach the output: loosened and exclusive bounds, ``const`` turned
    into ``enum``, widened types, re-induced Patterns, removed Formats and
    ``required`` entries, properties added under ``additionalProperties:
    false``, ``items: false`` replaced, ``false`` subschemas replaced by
    their Naive_Merge or by ``true``, an always-failing ``oneOf`` converted
    to ``anyOf`` (or to an ``allOf`` entry beside an existing ``anyOf``), and
    a ``not`` whose removal moves a ``$ref`` target into the root ``$defs``.

    Explicit examples (:data:`EXPLICIT_CASES`) reach the rarer outputs on
    every run, whatever the generated examples reach: ``oneOf`` from tagged
    Variants, ``allOf`` from a converted ``oneOf``, ``dependentRequired``,
    ``propertyNames`` with ``format``, relaxed Constraints, and a removal-only
    run with ``true`` subschemas and a relocated ``$ref`` target. Hypothesis
    events list the keywords and boolean subschemas each round-tripped model
    held, the injections applied, and whether soundness relaxed or removed
    Constraints.

Schema_Corpus schemas (Requirement 2.8) are added in task 25.7.

**Validates: Requirements 2.4, 2.7, 2.8**
"""

# Feature: json-schema-inference, Property 3: Schema emit/load round trip

from __future__ import annotations

import json
import tempfile
from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any
from unittest import mock
from urllib.parse import quote

from hypothesis import event, example, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.schema_model import SchemaModel, SchemaNode, split_pointer
from jsinfer.core.views import View
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoader
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.values.formats import FORMATS
from tests.records import complete_records
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, run_configs, schema_models

EMITTER = SchemaEmitter()
LOADER = SchemaLoader()


def _describe(model: SchemaModel) -> None:
    """Hypothesis events showing which structural cases the examples reached."""
    if model.root.boolean is not None:
        event("root: boolean")
        return
    nodes = list(model.walk())
    event(f"nodes: {'1' if len(nodes) == 1 else '2-9' if len(nodes) < 10 else '10+'}")
    keywords = {keyword for _, node in nodes for keyword in node.kw}
    for feature in ("$ref", "$defs", "$id"):
        if feature in keywords:
            event(f"has {feature}")
    if any(node.boolean is not None for _, node in nodes):
        event("has boolean subschemas")


@given(schema_models())
def test_emit_then_load_gives_an_equal_fully_supported_model(model: SchemaModel) -> None:
    _describe(model)
    emitted = EMITTER.emit(model)
    loaded = LOADER.load(emitted)

    assert loaded.fully_supported, f"unsupported constructs after loading: {loaded.unsupported}"
    assert loaded == model, (
        "the loaded model differs from the original under 2.7 equality\n"
        f"emitted:\n{emitted.decode('utf-8')}"
    )
    assert EMITTER.emit(loaded) == emitted, "emitting the loaded model changed the bytes"


# ---------------------------------------------------------------------------
# Schema_Models produced by the Inferrer (task 16.10)
# ---------------------------------------------------------------------------

MDL = "decide.mdl"

# Property names: the shared awkward keys plus keyword-like names and names that need pointer escapes.
FIELD_KEYS = (*KEYS, "x-provenance", "$ref", "~1/", "%25")

# -- scalar value families ----------------------------------------------------

# One example string per index for each of the ten Formats (values.formats.FORMATS).
_FORMAT_VALUES: Mapping[str, Any] = {
    "date-time": lambda i: f"2024-{i % 12 + 1:02d}-{i % 28 + 1:02d}T{i % 24:02d}:{i % 60:02d}:00Z",
    "date": lambda i: f"20{i % 100:02d}-{i % 12 + 1:02d}-{i % 28 + 1:02d}",
    "time": lambda i: f"{i % 24:02d}:{i % 60:02d}:{7 * i % 60:02d}Z",
    "duration": lambda i: f"P{i % 30 + 1}D" if i % 2 else f"PT{i % 50 + 1}H",
    "email": lambda i: f"user{i}@example.com",
    "hostname": lambda i: f"host{i}.example.org",
    "ipv4": lambda i: f"10.{i % 256}.{3 * i % 256}.1",
    "ipv6": lambda i: f"2001:db8::{i:x}",
    "uri": lambda i: f"https://example.com/p/{i}",
    "uuid": lambda i: f"{i:08x}-1234-4abc-8def-{7919 * i:012x}",
}
assert tuple(_FORMAT_VALUES) == FORMATS

_WORDS = ("new", "paid", "sent", "é", "😀", "a b", "")
_TEXT = st.text(alphabet='ab"\\\n\t😀é~/', max_size=6)
SCALAR_FAMILIES = (
    "int",
    "int",
    "float",
    "bigint",
    "bool",
    "null",
    "vocab",
    "vocab",
    "int-vocab",
    "text",
    "format",
    "format",
    "latent-int",
    "latent-dec",
)
MAP_KEY_FAMILIES = ("uuid", "data", "date", "data", "email", "ipv4")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values.

    ``scalar``: ``family`` and its fixed ``params``. ``record``: ``variants``
    holds, per Variant, its fields as ``(key, presence, child)`` with presence
    ``always``, ``sometimes``, or ``group`` (every ``group`` field of the
    Variant is present or absent together), and ``tags`` the Discriminator
    ``(key, value)`` of each Variant, if any. ``map``: ``family`` names the
    key family and ``children`` the value template. ``list``: the element
    template. ``tuple``: one template per position. ``mix``: the
    alternatives.
    """

    kind: str
    family: str = ""
    params: tuple[Any, ...] = ()
    variants: tuple[tuple[tuple[str, str, Template], ...], ...] = ()
    tags: tuple[tuple[str, str], ...] = ()
    children: tuple[Template, ...] = ()


@st.composite
def scalar_templates(draw: st.DrawFn) -> Template:
    family = draw(st.sampled_from(SCALAR_FAMILIES), label="scalar family")
    if family == "int":
        low = draw(st.sampled_from([0, 1, -5, 100, -1000]), label="low")
        return Template("scalar", family, (low, draw(st.sampled_from([0, 3, 50, 999]), label="span")))
    if family in ("vocab", "int-vocab"):
        members = st.sampled_from(_WORDS if family == "vocab" else (0, 1, 2, 7))
        return Template("scalar", family, tuple(draw(st.lists(members, min_size=1, max_size=3, unique=True))))
    if family == "format":
        return Template("scalar", family, (draw(st.sampled_from(FORMATS), label="format"),))
    return Template("scalar", family)


def scalar_values(template: Template) -> st.SearchStrategy[Any]:
    family, params = template.family, template.params
    if family == "int":
        low, span = params
        return st.integers(low, low + span)
    if family == "float":
        return st.integers(-40, 400).map(lambda k: k / 8)  # mostly non-integral decimals
    if family == "bigint":
        return st.integers(2**64 - 2, 2**64 + 2)
    if family == "bool":
        return st.booleans()
    if family == "null":
        return st.none()
    if family in ("vocab", "int-vocab"):
        return st.sampled_from(params)
    if family == "text":
        return _TEXT
    if family == "format":
        return st.integers(0, 500).map(_FORMAT_VALUES[params[0]])
    if family == "latent-int":
        return st.integers(-50, 5000).map(str)
    assert family == "latent-dec"
    return st.tuples(st.integers(0, 999), st.integers(0, 99)).map(lambda p: f"{p[0]}.{p[1]:02d}")


def map_keys(family: str) -> st.SearchStrategy[str]:
    if family == "data":
        return st.integers(0, 999).map(lambda i: f"u{i}")
    return st.integers(0, 500).map(_FORMAT_VALUES[family])


# -- structural templates -----------------------------------------------------

_KINDS = ("scalar", "scalar", "map", "record", "scalar", "record", "map", "list", "tuple", "mix")
PRESENCE = ("always", "always", "sometimes", "group", "group")


def child_templates(depth: int) -> st.SearchStrategy[Template]:
    return templates(depth - 1) if depth > 0 else scalar_templates()


@st.composite
def record_templates(draw: st.DrawFn, depth: int) -> Template:
    """One to three Variants with disjoint key sets, maybe a key shared by all, maybe a Discriminator."""
    n_variants = draw(st.sampled_from([1, 2, 2, 3]), label="variants")
    order = draw(st.permutations(FIELD_KEYS), label="key order")
    tagged = n_variants > 1 and draw(st.integers(0, 2), label="tagged") != 0
    tag_key, order = (order[0], order[1:]) if tagged else ("", order)
    shared = order[:1] if n_variants > 1 and draw(st.booleans(), label="a shared key") else ()
    rest = order[len(shared) :]
    variants: list[tuple[tuple[str, str, Template], ...]] = []
    start = 0
    for _ in range(n_variants):
        size = draw(st.integers(1 if n_variants > 1 else 0, 4), label="own keys")
        own = rest[start : start + size]
        start += size
        fields = []
        for key in (*shared, *own):
            presence = draw(st.sampled_from(PRESENCE), label=f"{key!r} presence")
            fields.append((key, presence, draw(child_templates(depth), label=f"{key!r} values")))
        variants.append(tuple(fields))
    tags = tuple((tag_key, f"v{i}") for i in range(n_variants)) if tagged else ()
    return Template("record", variants=tuple(variants), tags=tags)


@st.composite
def templates(draw: st.DrawFn, depth: int, mix: bool = True) -> Template:
    kinds = _KINDS if depth > 0 else ("scalar",)
    kind = draw(st.sampled_from(kinds if mix else [k for k in kinds if k != "mix"]), label="kind")
    if kind == "scalar":
        return draw(scalar_templates())
    if kind == "record":
        return draw(record_templates(depth))
    if kind == "map":
        family = draw(st.sampled_from(MAP_KEY_FAMILIES), label="map key family")
        return Template("map", family=family, children=(draw(child_templates(depth)),))
    if kind == "list":
        return Template("list", children=(draw(child_templates(depth)),))
    if kind == "tuple":
        positions = draw(st.lists(child_templates(depth), min_size=1, max_size=3), label="positions")
        return Template("tuple", children=tuple(positions))
    assert kind == "mix"
    options = draw(st.lists(templates(depth, mix=False), min_size=2, max_size=3), label="alternatives")
    return Template("mix", children=tuple(options))


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    """Mostly Records; also Records among other types, arrays of Records, and any template."""
    shape = draw(st.sampled_from(["record", "record", "record", "mixed", "array", "tuple", "any"]), label="root shape")
    if shape == "any":
        return draw(templates(2))
    if shape == "tuple":
        positions = draw(st.lists(templates(1), min_size=2, max_size=3), label="root positions")
        return Template("tuple", children=tuple(positions))
    record = draw(record_templates(2), label="root record")
    if shape == "record":
        return record
    if shape == "mixed":
        return Template("mix", children=(record, draw(scalar_templates()), draw(templates(1, mix=False))))
    return Template("list", children=(record,))


@st.composite
def instances(draw: st.DrawFn, template: Template) -> Any:
    kind = template.kind
    if kind == "scalar":
        return draw(scalar_values(template))
    if kind == "record":
        index = draw(st.integers(0, len(template.variants) - 1), label="variant")
        group = draw(st.booleans(), label="group present")
        out: dict[str, Any] = {}
        if template.tags:
            key, value = template.tags[index]
            out[key] = value
        for key, presence, child in template.variants[index]:
            present = presence == "always" or (group if presence == "group" else draw(st.booleans(), label=f"{key!r}"))
            if present:
                out[key] = draw(instances(child))
        return out
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(map_keys(template.family), min_size=1, max_size=5), label="map keys"))
        return {key: draw(instances(template.children[0])) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0]), max_size=4), label="elements")
    if kind == "tuple":
        full = draw(st.integers(0, 3), label="full tuple") != 0  # mostly full, so Tuples beat Lists
        length = len(template.children) if full else draw(st.integers(0, len(template.children)), label="tuple length")
        return [draw(instances(child)) for child in template.children[:length]]
    assert kind == "mix"
    return draw(instances(template.children[draw(st.integers(0, len(template.children) - 1))]))


@st.composite
def collections(draw: st.DrawFn) -> list[Any]:
    root = draw(root_templates(), label="root template")
    return [draw(instances(root), label="document") for _ in range(draw(st.integers(5, 50), label="documents"))]


# -- over-tight injections ----------------------------------------------------

# Over-tight bounds (each applies only to instances of its type, so any node may hold it).
_TIGHT_BOUNDS: tuple[tuple[str, int], ...] = (
    ("exclusiveMinimum", 10**6),
    ("maxProperties", 0),
    ("minProperties", 30),
    ("maxItems", 0),
    ("minItems", 9),
    ("maxLength", 0),
    ("minLength", 40),
    ("maximum", -(10**6)),
    ("minimum", 10**6),
    ("exclusiveMaximum", -(10**6)),
)
_FRAGMENT_SAFE = "/!$&'()*+,;=:@"  # RFC 3986 fragment characters besides unreserved ones
# The structural edits, which find an eligible node less often, are listed twice.
INJECTIONS = (
    "bound",
    "bound",
    "const",
    "format",
    "pattern",
    "type",
    "required",
    "closed",
    "closed",
    "items-false",
    "items-false",
    "false-subschema",
    "false-subschema",
    "one-of",
    "one-of",
    "relocated-ref",
)


def editable(pointer: str, node: SchemaNode) -> bool:
    """Whether injections may edit ``node``: a keyword schema outside every injected ``not``."""
    return node.boolean is None and "not" not in split_pointer(pointer)  # no property is named "not"


@dataclass(frozen=True)
class Injection:
    """One over-tight edit of the analyzer's model, at the ``pick``-th eligible node (modulo their count)."""

    kind: str
    pick: int
    arg: int

    def eligible(self, pointer: str, node: SchemaNode) -> bool:
        kw = node.kw
        if not editable(pointer, node):
            return False
        if self.kind == "type":
            return "type" in kw
        if self.kind == "closed":
            return bool(kw.get("properties")) and "additionalProperties" not in kw
        if self.kind == "items-false":  # a Tuple of two or more positions, or a List
            items = kw.get("items")
            listed = items is not None and items.boolean is None and "prefixItems" not in kw
            return len(kw.get("prefixItems", [])) >= 2 or listed
        if self.kind == "false-subschema":
            return bool(kw.get("properties") or kw.get("prefixItems") or kw.get("anyOf"))
        if self.kind == "one-of":
            return "oneOf" not in kw and ("anyOf" in kw or self.arg % 2 == 0)
        if self.kind == "relocated-ref":
            return "not" not in kw
        return True

    def apply(self, model: SchemaModel) -> str | None:
        """Edit ``model`` in place; the label of the edit, or None when no node is eligible."""
        nodes = [(pointer, node) for pointer, node in model.walk() if self.eligible(pointer, node)]
        if not nodes:
            return None
        pointer, node = nodes[self.pick % len(nodes)]
        kw, arg, kind = node.kw, self.arg, self.kind
        if kind == "bound":
            name, value = _TIGHT_BOUNDS[arg % len(_TIGHT_BOUNDS)]
            kw[name] = value
            return f"bound {name}"
        if kind == "const":
            kw.pop("enum", None)
            kw["const"] = "\uffffsentinel"
        elif kind == "format":
            kw["format"] = FORMATS[arg % len(FORMATS)]
        elif kind == "pattern":
            kw["pattern"] = r"^(?:[a-z])$(?!\n)"
        elif kind == "type":
            kw["type"] = "null"
        elif kind == "required":
            kw["required"] = [*kw.get("required", []), "zz-missing"]
        elif kind == "closed":
            names = sorted(kw["properties"])
            del kw["properties"][names[arg % len(names)]]
            kw["additionalProperties"] = SchemaNode(boolean=False)
        elif kind == "items-false":
            if "prefixItems" in kw:
                kw["prefixItems"] = kw["prefixItems"][:-1]
            else:
                kw["prefixItems"] = [kw["items"]]
            kw["items"] = SchemaNode(boolean=False)
        elif kind == "false-subschema":
            slots: list[tuple[Any, Any]] = [(kw["properties"], name) for name in sorted(kw.get("properties", {}))]
            slots += [(kw[key], i) for key in ("prefixItems", "anyOf") for i in range(len(kw.get(key, [])))]
            container, slot = slots[arg % len(slots)]
            container[slot] = SchemaNode(boolean=False)
        elif kind == "one-of":
            kw["oneOf"] = [SchemaNode(), SchemaNode()]  # two matching branches reject every instance
            return "one-of beside anyOf" if "anyOf" in kw else "one-of"
        else:
            assert kind == "relocated-ref"
            # `not {}` rejects every instance, so it is removed, which moves the `$ref` target into the root `$defs`.
            # The site is chosen before the target exists, so no target references itself.
            sites = [site for at, site in model.walk() if editable(at, site) and "$ref" not in site.kw]
            kw["not"] = SchemaNode()
            sites[arg % len(sites)].kw["$ref"] = "#" + quote(f"{pointer}/not", safe=_FRAGMENT_SAFE)
        return kind


@contextmanager
def injected(injections: Sequence[Injection], applied: list[str]) -> Iterator[None]:
    """Patch ``StructureAnalyzer.infer_model`` to apply ``injections``; their labels go to ``applied``."""
    if not injections:
        yield
        return
    original = StructureAnalyzer.infer_model

    def over_tight(self: StructureAnalyzer, view: View) -> SchemaModel:
        model = original(self, view)
        for injection in injections:
            label = injection.apply(model)
            if label is not None:
                applied.append(label)
        complete_records(model)  # 26.1: the injected Constraints get records, as a component's would
        return model

    with mock.patch.object(StructureAnalyzer, "infer_model", over_tight):
        yield


# -- Technique states, Resource_Parameters, and the Model_Bundle ---------------

# Hand-set Learned_Thresholds inside their admissible ranges, with the extremes common.
THRESHOLD_VALUES: Mapping[str, st.SearchStrategy[int | float]] = {
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


@st.composite
def resource_values(draw: st.DrawFn) -> dict[str, int]:
    """Small Resource_Parameters for the grouping Techniques, and the soundness pass limit."""
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    return {
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.dbscan"),
        "sample.hdbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(1, 6), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
        "soundness.max_passes": draw(st.sampled_from([1, 2, 5]), label="soundness.max_passes"),
    }


@dataclass(frozen=True)
class Case:
    """One full run: documents, Technique states, Learned_Thresholds, and injections."""

    docs: tuple[Any, ...]
    scenario: str
    config: RunConfiguration
    thresholds: Mapping[str, int | float]
    injections: tuple[Injection, ...]


@st.composite
def cases(draw: st.DrawFn) -> Case:
    docs = tuple(draw(collections(), label="documents"))
    scenario = draw(st.sampled_from(["priority", "drawn"]), label="scenario")
    base: dict[str, Any] = {"workers": 1, **draw(resource_values(), label="resources")}
    if scenario == "priority":
        base[technique_setting(MDL)] = False  # groupings with two or more Variants stay split (5.11)
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    # Hypothesis draws 0 often; the sum of two wide draws, taken modulo the choices, spreads them evenly.
    wide = st.tuples(st.integers(0, 2**16), st.integers(0, 2**16)).map(sum)
    injection = st.builds(Injection, wide.map(lambda k: INJECTIONS[k % len(INJECTIONS)]), wide, wide)
    some = st.lists(injection, min_size=1, max_size=4)
    injections = draw(st.one_of(some, some, st.just([])), label="injections")  # two runs in three are injected
    # References go in last, so no later edit moves or drops their targets.
    injections.sort(key=lambda injection: injection.kind == "relocated-ref")
    return Case(docs, scenario, config, thresholds, tuple(injections))


def _explicit(docs: Sequence[Any], *injections: Injection, mdl: bool = True, **flags: Any) -> Case:
    """An explicit case under the default Technique states (``decide.mdl`` as given) with every M2 threshold."""
    flags = {"workers": 1, technique_setting(MDL): mdl, **flags}
    thresholds = {
        "lsh.keyset_jaccard": 0.5,
        "dbscan.eps": 0.34,
        "dbscan.min_samples": 1,
        "hdbscan.min_cluster_size": 2,
        "cooccur.min_support": 1,
        "cooccur.min_confidence": 0.5,
    }
    return Case(tuple(docs), "drawn" if mdl else "priority", RunConfiguration.resolve(flags), thresholds, injections)


def _uuid(i: int) -> str:
    return _FORMAT_VALUES["uuid"](i)


# Explicit examples that reach the rarer keywords whatever the generated examples reach.
EXPLICIT_CASES: Mapping[str, Case] = {
    # Tagged Variants resolved by priority stay split, and their branches are disjoint: `oneOf`.
    "tagged Variants": _explicit(
        [
            {"kind": "a", "x": i, "when": f"2024-01-{i % 28 + 1:02d}"} if i % 2 else {"kind": "b", "y": [i]}
            for i in range(20)
        ],
        mdl=False,
    ),
    # Untagged Variants stay split under `anyOf`; an always-failing `oneOf` beside it becomes an `allOf` entry.
    "oneOf beside anyOf": _explicit(
        [{"t": i % 4, "p": "s"} if i % 2 else {"u": [i % 3], "w": None} for i in range(20)],
        Injection("one-of", 0, 1),
        mdl=False,
    ),
    # Fields present together give `dependentRequired`.
    "co-occurrence": _explicit(
        [{"a": i, **({"b": 1, "c": 2} if i % 3 == 0 else {}), **({"c": 3} if i % 3 == 1 else {})} for i in range(90)]
    ),
    # uuid-keyed Maps give `additionalProperties` with a value schema and `propertyNames` with `format`.
    "Maps": _explicit([{"m": {_uuid(7 * i + j): f"{j}.5" for j in range(i % 4 + 1)}} for i in range(20)]),
    # Relaxation by the first candidate (MDL off): a property added under `additionalProperties: false`,
    # `items: false` replaced by a Naive_Merge `items`, a looser exclusive bound, and `const` widened to `enum`.
    "relaxations": _explicit(
        [{"pair": [i, f"s{i}"], "n": i, "o": {"k": i, "j": "x"}, "l": [i, i + 1]} for i in range(12)],
        Injection("closed", 0, 3),
        Injection("items-false", 0, 0),
        Injection("bound", 2, 0),
        Injection("const", 5, 0),
        mdl=False,
    ),
    # Removal only: a `false` property becomes `true`, and removing `not` moves the `$ref` target into `$defs`.
    "removals": _explicit(
        [{"a": {"b": i, "c": "x"}, "d": [str(i)]} for i in range(10)],
        Injection("false-subschema", 1, 1),
        Injection("relocated-ref", 2, 3),
        **{"soundness.max_passes": 1},
    ),
}


def write_bundle(root: Path, thresholds: Mapping[str, int | float]) -> str:
    """A Model_Bundle directory with ``thresholds`` as fitted Learned_Thresholds and no models."""
    manifest = {"format": BUNDLE_FORMAT, "bundle_id": "p03-stub", "artifacts": {}}
    records = {
        name: ThresholdRecord.from_spec(
            THRESHOLDS[name], value=value, objective_value=0.0, n_examples=1, training_data="p03-stub"
        ).to_json()
        for name, value in thresholds.items()
    }
    (root / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
    (root / THRESHOLDS_FILE).write_text(json.dumps(records), encoding="utf-8")
    return str(root)


def run(case: Case, applied: list[str]) -> PipelineResult:
    with tempfile.TemporaryDirectory(prefix="p03-bundle-") as directory, injected(case.injections, applied):
        config = replace(case.config, model_bundle=write_bundle(Path(directory), case.thresholds))
        return infer_documents(list(case.docs), config)


# -- the round trip -------------------------------------------------------------


def emitted_models(result: PipelineResult) -> list[tuple[str, SchemaModel]]:
    """The run's Schema_Models: the emitted schema, plus each Schema_Version once task 48.2 emits them."""
    models = [("schema", result.model)]
    versions = result.report.versions
    if versions is not None and versions.applied and versions.segments:
        per_version = getattr(result, "versions", None)
        assert per_version is not None, "task 48.2: expose each Schema_Version's Schema_Model on the result"
        models += [(f"version {i}", version.model) for i, version in enumerate(per_version)]
    return models


def describe_inferred(model: SchemaModel, result: PipelineResult, applied: Sequence[str]) -> None:
    """Hypothesis events: keywords and boolean subschemas of the model, injections, and soundness edits."""
    nodes = [node for _, node in model.walk()]
    for keyword in sorted({keyword for node in nodes for keyword in node.kw}):
        event(f"keyword: {keyword}")
    for value in (True, False):
        if any(node.boolean is value for node in nodes[1:]):
            event(f"subschema: {str(value).lower()}")
    if model.root.boolean is not None:
        event("root: boolean")
    for label in sorted(set(applied)):
        event(f"injected: {label}")
    summary = result.soundness.summary
    if summary.relaxations:
        event("soundness: relaxed")
    if summary.removals:
        event("soundness: removed")


def check_round_trip(label: str, model: SchemaModel, expected_bytes: bytes | None) -> None:
    emitted = EMITTER.emit(model)
    text = emitted.decode("utf-8")
    if expected_bytes is not None:
        assert emitted == expected_bytes, f"{label}: emitting the model differs from the pipeline's schema bytes"
    loaded = LOADER.load(emitted)
    assert loaded.fully_supported, f"{label}: unsupported constructs after loading: {loaded.unsupported}\n{text}"
    assert loaded.ignored == [], f"{label}: ignored keywords after loading: {loaded.ignored}\n{text}"
    assert loaded.dialect_assumed == (model.root.boolean is not None), f"{label}: dialect assumed\n{text}"
    assert loaded == model, f"{label}: the loaded model differs from the inferred one under 2.7 equality\n{text}"
    assert sorted(EMITTER.layout(loaded)[1].values()) == sorted(EMITTER.layout(model)[1].values()), (
        f"{label}: the loaded model's Constraints sit at other pointers\n{text}"
    )
    assert EMITTER.emit(loaded) == emitted, f"{label}: emitting the loaded model changed the bytes\n{text}"


# Feature: json-schema-inference, Property 3: Schema emit/load round trip
@given(cases())
@example(EXPLICIT_CASES["tagged Variants"])
@example(EXPLICIT_CASES["oneOf beside anyOf"])
@example(EXPLICIT_CASES["co-occurrence"])
@example(EXPLICIT_CASES["Maps"])
@example(EXPLICIT_CASES["relaxations"])
@example(EXPLICIT_CASES["removals"])
def test_inferrer_outputs_round_trip_through_emit_and_load(case: Case) -> None:
    applied: list[str] = []
    result = run(case, applied)
    event(f"scenario: {case.scenario}")
    for label, model in emitted_models(result):
        describe_inferred(model, result, applied)
        check_round_trip(label, model, result.schema if label == "schema" else None)


# ---------------------------------------------------------------------------
# Corpus-parametrized cases (task 25.7; Requirement 2.8)
# ---------------------------------------------------------------------------
#
# The fixture corpus (``tests/fixtures/corpus``) runs in CI. The full
# Schema_Corpus runs when ``JSINFER_CORPUS_MANIFEST`` names a corpus manifest
# written by ``jsinfer split`` (the nightly job); every fully supported schema
# of it is a case, and the others are skipped with the loader's reason.

import os  # noqa: E402

import pytest  # noqa: E402

from jsinfer.bench.splits import CorpusManifest  # noqa: E402
from jsinfer.io.load import SchemaLoadError  # noqa: E402
from tests.conftest import FIXTURE_MANIFEST  # noqa: E402


def corpus_cases() -> list[tuple[str, CorpusManifest]]:
    manifests = [CorpusManifest.load(FIXTURE_MANIFEST)]
    nightly = os.environ.get("JSINFER_CORPUS_MANIFEST")
    if nightly:
        manifests.append(CorpusManifest.load(nightly))
    return [(sid, manifest) for manifest in manifests for sid in manifest.ids()]


CORPUS_CASES = corpus_cases()


@pytest.mark.parametrize(("schema_id", "manifest"), CORPUS_CASES, ids=[sid for sid, _ in CORPUS_CASES])
def test_corpus_schemas_round_trip(schema_id: str, manifest: CorpusManifest) -> None:
    try:
        model = LOADER.load(manifest.schema_path(schema_id).read_bytes())
    except SchemaLoadError as error:
        pytest.skip(f"{schema_id} does not load: {error}")
    if not model.fully_supported:
        pytest.skip(f"{schema_id} is partially supported: {model.unsupported[:3]}")
    emitted = EMITTER.emit(model)
    loaded = LOADER.load(emitted)
    assert loaded.fully_supported and loaded == model, f"{schema_id}: the round trip changed the model"
    assert EMITTER.emit(loaded) == emitted
