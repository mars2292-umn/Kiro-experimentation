"""Property 43: Annotation and report neutrality (task 17.6).

For any collection: the schemas emitted with inline provenance on and off
accept and reject the same generated documents and load to equal
Schema_Models; each inline record equals its explanation-file entry;
disabling nearest-schema lookup leaves the schema and explanation bytes
unchanged.

Each example runs the whole in-memory pipeline
(``pipeline.infer.infer_documents``, run A) on a generated collection. The
plain schema is ``A.schema`` (``output.inline_provenance`` is off in run
A); the inline schema is
``SchemaEmitter().emit(A.model, inline_provenance=True)``, which run B,
with the setting on, must reproduce. The explanation file is
``SchemaEmitter().explanation(A.model, A.pointers)``.

Annotation neutrality (26.11)
    - Verdicts: a pool of documents is validated against the plain and the
      inline schema with the project's Validator (``format`` asserted), and
      every verdict must agree. The pool holds the collection's own
      documents (all accepted, by soundness), mutants of them that retype,
      drop, add, grow, or shrink one value (which break inferred ``type``,
      ``required``, closed Records, ``items: false``, bounds, ``enum``,
      ``format``, and ``pattern``), arbitrary ``json_values``, and one probe
      of each JSON type. Non-vacuity: the pool always holds accepted
      documents, and it must hold a rejected one whenever the root schema
      object's ``type`` misses a probe's type (the probe then fails). Events
      record the accepted/rejected mix.
    - Loading: both schemas load fully supported, with no ignored keyword,
      to Schema_Models equal under 2.7, and emitting the loaded inline model
      gives the plain bytes.
    - Removing every ``x-provenance`` member from the schema objects of the
      inline document gives the plain document byte for byte, and the
      inline layout's pointer map is the plain one, so the explanation
      file's pointers address the inline schema too.

Inline records (26.6)
    The schema objects of the emitted document are found by walking the
    2020-12 applicator keywords (a property named ``x-provenance`` is a
    subschema, not the keyword). Each explanation key belongs to the deepest
    schema object whose pointer is a proper prefix of it, and the next token
    must be a keyword of that object. Then, exactly: the schema objects that
    hold ``x-provenance`` are the owners of at least one key, and each
    owner's ``x-provenance`` maps the relative pointer of every key it owns
    to that key's record JSON in the explanation file, and nothing else. So
    every explanation entry appears inline exactly once, each inline record
    is identical in content to its entry, and schema objects without
    Constraints (and boolean schemas) get no ``x-provenance``.

``output.inline_provenance`` in the pipeline
    Run B repeats run A with ``output.inline_provenance = true``. Its model
    equals A's under 2.7, its explanation file (``B.explanation``, as the
    pipeline's Schema_Emitter stage built it) equals A's byte for byte, and
    its schema bytes are exactly the inline bytes (the stage honors the
    setting since task 18.1).

Nearest-schema clause (25.5; gated on ``corpus.nearest_schema``)
    - Active once the Technique is implemented (task 40.4): run C repeats
      run A with the Technique's state flipped, and its schema and
      explanation bytes must equal A's (report sections may differ).
    - Until then the gate is checked to be honest: the registry lists the
      Technique as unimplemented and disabled by default, every drawn
      configuration has it disabled (``setting_flags`` draws toggles of
      implemented Techniques only), and enabling it is a ``ConfigError``
      with an ``unimplemented`` problem naming it, raised when the
      Run_Configuration is resolved, before any document is ingested
      (28.8). Disabling it explicitly resolves to the same Technique states
      as leaving it out, and run B also toggles whether the ``false`` flag
      is supplied, so its equal explanation bytes (and schema bytes) show
      that the flag has no effect.

Inputs
    Collections come from random templates, as in Property 3: Records with
    one to three Variants (maybe with a ``const`` Discriminator field, so
    split branches are disjoint and become ``oneOf``), fields always
    present, sometimes present, or present together (``dependentRequired``),
    Maps with data-like or Format keys (``propertyNames``), Lists, Tuples
    (``prefixItems``, ``items: false``), mixed types, and scalars: bounded
    integers, decimals, big integers, booleans, nulls, small vocabularies
    (``enum``/``const``), free text, the ten Formats, and latent integer and
    decimal strings (``x-latent-type``, ``pattern``). Property names include
    :data:`tests.strategies.KEYS` (the empty name among them) and names that
    look like keywords or need pointer escapes: ``x-provenance``, ``$ref``,
    ``properties``, ``required``, ``~1/``, ``%25``.

    Technique states come from ``setting_flags`` (implemented Techniques
    only, prerequisites respected) with ``workers = 1``, small
    Resource_Parameters, and ``soundness.max_passes`` of 1, 2, or 5; the
    ``priority`` scenario turns ``decide.mdl`` off. A real Model_Bundle
    directory with hand-set Learned_Thresholds is written per example.

    Over-tight injections edit the analyzer's model before the soundness
    check, as Property 3 does (``StructureAnalyzer.infer_model`` patched,
    ``tests.records.complete_records`` restoring the record invariant), so
    relaxation and removal outputs reach both schemas: loosened bounds,
    ``const`` widened to ``enum``, added properties under
    ``additionalProperties: false``, replaced ``items: false``, ``true`` and
    ``false`` subschemas, an ``allOf`` entry from a converted ``oneOf``, and
    a ``$ref`` whose target is relocated into the root ``$defs``. Explicit
    examples (:data:`EXPLICIT_CASES`) reach ``oneOf``, ``allOf``,
    ``dependentRequired``, ``propertyNames`` with ``format``, relaxed and
    removed Constraints, ``$defs``/``$ref``, and the awkward property names
    on every run.

**Validates: Requirements 25.5, 26.6, 26.11**
"""

# Feature: json-schema-inference, Property 43: Annotation and report neutrality

from __future__ import annotations

import copy
import json
import tempfile
from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from unittest import mock
from urllib.parse import quote

import pytest
from hypothesis import event, example, given
from hypothesis import strategies as st

from jsinfer.config.errors import ConfigError
from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.schema_model import SchemaModel, SchemaNode, split_pointer
from jsinfer.core.views import View
from jsinfer.io.emit import PROVENANCE_KEYWORD, SchemaEmitter
from jsinfer.io.load import SchemaLoader
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, NEAREST_SCHEMA, THRESHOLDS_FILE
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.values.formats import FORMATS
from tests.records import complete_records
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, decimal_scalars, json_values, setting_flags

EMITTER = SchemaEmitter()
LOADER = SchemaLoader()
# One Validator for the module, so compiled schemas and format verdicts are cached across examples.
VALIDATOR = Validator()

MDL = "decide.mdl"
INLINE = "output.inline_provenance"
NEAREST_SETTING = technique_setting(NEAREST_SCHEMA)
PROVENANCE = "x-provenance"  # the keyword 26.6 names
assert PROVENANCE_KEYWORD == PROVENANCE

# Property names: the shared awkward keys plus keyword-like names and names that need pointer escapes.
FIELD_KEYS = (*KEYS, PROVENANCE, "$ref", "properties", "required", "~1/", "%25")

# ---------------------------------------------------------------------------
# Schema objects of an emitted document (the 2020-12 applicator keywords)
# ---------------------------------------------------------------------------

_SINGLE = frozenset(
    {
        "additionalProperties",
        "contains",
        "contentSchema",
        "else",
        "if",
        "items",
        "not",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    }
)
_LISTS = frozenset({"allOf", "anyOf", "oneOf", "prefixItems"})
_MAPS = frozenset({"$defs", "dependentSchemas", "patternProperties", "properties"})


def _escape(token: str | int) -> str:
    return str(token).replace("~", "~0").replace("/", "~1")


def _tokens(pointer: str) -> list[str]:
    return [token.replace("~1", "/").replace("~0", "~") for token in pointer.split("/")[1:]]


def _children(doc: dict[str, Any]) -> Iterator[tuple[str, Any]]:
    """``(relative pointer, subschema)`` of each subschema directly under the schema object ``doc``."""
    for keyword, value in doc.items():
        if keyword in _SINGLE:
            yield f"/{_escape(keyword)}", value
        elif keyword in _LISTS:
            for index, child in enumerate(value):
                yield f"/{_escape(keyword)}/{index}", child
        elif keyword in _MAPS:
            for name, child in value.items():
                yield f"/{_escape(keyword)}/{_escape(name)}", child


def schema_objects(doc: Any, pointer: str = "") -> Iterator[tuple[str, dict[str, Any]]]:
    """``(pointer, schema object)`` for every schema object of an emitted document, in pre-order."""
    if not isinstance(doc, dict):
        return  # a boolean schema
    yield pointer, doc
    for relative, child in _children(doc):
        yield from schema_objects(child, pointer + relative)


def strip_provenance(doc: Any) -> Any:
    """``doc`` without the ``x-provenance`` keyword of any schema object (property names are kept)."""
    if not isinstance(doc, dict):
        return doc
    out: dict[str, Any] = {}
    for keyword, value in doc.items():
        if keyword == PROVENANCE:
            continue
        if keyword in _SINGLE:
            out[keyword] = strip_provenance(value)
        elif keyword in _LISTS:
            out[keyword] = [strip_provenance(child) for child in value]
        elif keyword in _MAPS:
            out[keyword] = {name: strip_provenance(child) for name, child in value.items()}
        else:
            out[keyword] = value
    return out


def owner(key: str, objects: Mapping[str, dict[str, Any]]) -> str:
    """The pointer of the deepest schema object that is a proper prefix of the explanation key ``key``."""
    tokens = _tokens(key)
    assert tokens, "an explanation key is a non-empty pointer"
    for n in range(len(tokens) - 1, -1, -1):
        pointer = "".join(f"/{_escape(token)}" for token in tokens[:n])
        if pointer in objects:
            return pointer
    raise AssertionError(f"explanation key {key!r} lies under no schema object")


# ---------------------------------------------------------------------------
# Collections (templates as in Property 3)
# ---------------------------------------------------------------------------

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
_KINDS = ("scalar", "scalar", "map", "record", "scalar", "record", "map", "list", "tuple", "mix")
PRESENCE = ("always", "always", "sometimes", "group", "group")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values.

    ``scalar``: ``family`` and its fixed ``params``. ``record``: ``variants``
    holds, per Variant, its fields as ``(key, presence, child)`` with presence
    ``always``, ``sometimes``, or ``group`` (the ``group`` fields of a Variant
    are present or absent together), and ``tags`` the Discriminator
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
    """Mostly Records; also Records among other types, arrays of Records, Tuples, and any template."""
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
    return [draw(instances(root), label="document") for _ in range(draw(st.integers(5, 40), label="documents"))]


# ---------------------------------------------------------------------------
# Over-tight injections (as in Property 3)
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# The document pool: mutants, arbitrary values, and type probes
# ---------------------------------------------------------------------------

# One value of each JSON type (integer and non-integral number apart).
PROBES: tuple[Any, ...] = (None, True, 0, 0.5, "s", [], {})
MUTATIONS = ("retype", "drop", "extra", "grow", "shrink")


def json_type(value: Any) -> str:
    """The JSON Schema type name of ``value``, with ``integer`` for zero-fraction numbers."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int) or (isinstance(value, float) and value.is_integer()):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    return "array" if isinstance(value, list) else "object"


def _locations(value: Any, path: tuple[Any, ...] = ()) -> Iterator[tuple[Any, ...]]:
    yield path
    if isinstance(value, dict):
        for key, child in value.items():
            yield from _locations(child, (*path, key))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from _locations(child, (*path, index))


@dataclass(frozen=True)
class Mutation:
    """Edit the ``pick``-th value (pre-order, modulo their count) of document ``doc`` with ``op``."""

    doc: int
    pick: int
    op: str
    arg: int

    def apply(self, docs: Sequence[Any]) -> tuple[str, Any]:
        """The edit's label and the mutated copy of the document (the original is unchanged)."""
        root = copy.deepcopy(docs[self.doc % len(docs)])
        paths = list(_locations(root))
        path = paths[self.pick % len(paths)]
        parent, target = None, root
        for step in path:
            parent, target = target, target[step]

        def put(new: Any) -> Any:
            if not path:
                return new
            parent[path[-1]] = new
            return root

        op = self.op
        probe = copy.deepcopy(PROBES[self.arg % len(PROBES)])
        if op == "drop" and isinstance(parent, dict):
            del parent[path[-1]]  # breaks `required`
            return op, root
        if op == "extra" and isinstance(target, dict):
            target["zz-extra"] = probe  # breaks closed Records and key bounds
            return op, root
        if op == "extra" and isinstance(target, list):
            target.append(copy.deepcopy(target[-1]) if target else probe)  # `items: false`, maxItems
            return op, root
        if op == "grow" and json_type(target) == "string":
            return op, put(target + "~" * 41)  # length bounds, enum, format, pattern
        if op == "grow" and json_type(target) in ("integer", "number"):
            return op, put(target * 1000 + 0.5)  # bounds, integer type, enum
        if op == "shrink" and isinstance(target, (list, dict, str)) and target:
            return op, put(type(target)())  # required, minItems, minLength, prefix positions
        others = [other for other in PROBES if json_type(other) != json_type(target)]
        return "retype", put(copy.deepcopy(others[self.arg % len(others)]))


# ---------------------------------------------------------------------------
# Cases: Technique states, thresholds, injections, and the pool
# ---------------------------------------------------------------------------

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
ALL_THRESHOLDS: Mapping[str, int | float] = {
    "lsh.keyset_jaccard": 0.5,
    "dbscan.eps": 0.34,
    "dbscan.min_samples": 1,
    "hdbscan.min_cluster_size": 2,
    "cooccur.min_support": 1,
    "cooccur.min_confidence": 0.5,
}


@st.composite
def resource_values(draw: st.DrawFn) -> dict[str, int]:
    """Small Resource_Parameters for the grouping Techniques, and the soundness pass limit."""
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    return {
        "workers": 1,
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
    """One example: documents, setting flags of run A, Learned_Thresholds, injections, and the extra pool."""

    docs: tuple[Any, ...]
    scenario: str
    flags: Mapping[str, Any]
    thresholds: Mapping[str, int | float]
    injections: tuple[Injection, ...]
    mutations: tuple[Mutation, ...]
    extras: tuple[Any, ...]


# Hypothesis draws 0 often; the sum of two wide draws, taken modulo the choices, spreads them evenly.
_WIDE = st.tuples(st.integers(0, 2**16), st.integers(0, 2**16)).map(sum)


@st.composite
def cases(draw: st.DrawFn) -> Case:
    docs = tuple(draw(collections(), label="documents"))
    scenario = draw(st.sampled_from(["priority", "drawn"]), label="scenario")
    base: dict[str, Any] = dict(draw(resource_values(), label="resources"))
    if scenario == "priority":
        base[technique_setting(MDL)] = False  # groupings with two or more Variants stay split (5.11)
    # Run A has inline provenance off: the setting left out or supplied as false.
    if draw(st.booleans(), label="supply inline false"):
        base[INLINE] = False
    flags = draw(setting_flags(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="flags")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    injection = st.builds(Injection, _WIDE.map(lambda k: INJECTIONS[k % len(INJECTIONS)]), _WIDE, _WIDE)
    some = st.lists(injection, min_size=1, max_size=4)
    injections = draw(st.one_of(some, some, st.just([])), label="injections")  # two runs in three are injected
    # References go in last, so no later edit moves or drops their targets.
    injections.sort(key=lambda injection: injection.kind == "relocated-ref")
    mutation = st.builds(Mutation, _WIDE, _WIDE, st.sampled_from(MUTATIONS), _WIDE)
    mutations = draw(st.lists(mutation, min_size=3, max_size=12), label="mutations")
    extras = draw(st.lists(json_values(max_leaves=8, scalars=decimal_scalars), max_size=4), label="extras")
    return Case(docs, scenario, flags, thresholds, tuple(injections), tuple(mutations), tuple(extras))


# Every mutation kind at spread-out positions of the first documents, for the explicit cases.
_EXPLICIT_MUTATIONS = tuple(Mutation(i, 7 * i + 3, MUTATIONS[i % len(MUTATIONS)], i) for i in range(10))


def _explicit(docs: Sequence[Any], *injections: Injection, mdl: bool = True, **flags: Any) -> Case:
    """An explicit case under the default Technique states (``decide.mdl`` as given) with every M2 threshold."""
    flags = {"workers": 1, technique_setting(MDL): mdl, **flags}
    return Case(
        tuple(docs),
        "drawn" if mdl else "priority",
        flags,
        ALL_THRESHOLDS,
        injections,
        _EXPLICIT_MUTATIONS,
        (None, {"a": [1, "b"]}, [{"": 0.1}]),
    )


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
    # Property names that look like keywords or are empty: `x-provenance` beside the inline keyword, a Tuple
    # under the empty name (`prefixItems`, `items: false`), latent integer strings (`x-latent-type`,
    # `pattern`) under `$ref`, an `enum` under `required`, and a nested Record under `properties`.
    "awkward names": _explicit(
        [
            {
                PROVENANCE: i * 3,
                "$ref": f"-{i * 7 + 1}",  # a leading hyphen keeps the strings from being hostnames
                "": [i, "t", i % 2 == 0],
                "required": "ab"[i % 2],
                "properties": {"type": f"host{i}.example.org", PROVENANCE: None},
            }
            for i in range(16)
        ]
    ),
}


def write_bundle(root: Path, thresholds: Mapping[str, int | float]) -> str:
    """A Model_Bundle directory with ``thresholds`` as fitted Learned_Thresholds and no models."""
    manifest = {"format": BUNDLE_FORMAT, "bundle_id": "p43-stub", "artifacts": {}}
    records = {
        name: ThresholdRecord.from_spec(
            THRESHOLDS[name], value=value, objective_value=0.0, n_examples=1, training_data="p43-stub"
        ).to_json()
        for name, value in thresholds.items()
    }
    (root / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
    (root / THRESHOLDS_FILE).write_text(json.dumps(records), encoding="utf-8")
    return str(root)


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def describe(model: SchemaModel, result: PipelineResult, applied: Sequence[str]) -> None:
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


def check_verdicts(plain_doc: Any, inline_doc: Any, pool: Sequence[tuple[str, Any]]) -> None:
    """26.11: both schemas accept and reject the same documents of ``pool`` (``(source, document)`` pairs)."""
    plain_c, inline_c = VALIDATOR.compile(plain_doc), VALIDATOR.compile(inline_doc)
    accepted: set[str] = set()
    rejected: set[str] = set()
    for source, doc in pool:
        verdict = VALIDATOR.is_valid(plain_c, doc)
        assert VALIDATOR.is_valid(inline_c, doc) == verdict, (
            f"the plain and inline schemas disagree on a {source} document: plain {'accepts' if verdict else 'rejects'}"
            f" {doc!r}"
        )
        (accepted if verdict else rejected).add(source)
    assert "collection" in accepted, "the collection's own documents must validate (soundness)"
    # Non-vacuity: a root `type` that misses a probe's type rejects that probe.
    if isinstance(plain_doc, dict) and "type" in plain_doc:
        names = plain_doc["type"] if isinstance(plain_doc["type"], list) else [plain_doc["type"]]
        covered = {*names, "integer"} if "number" in names else set(names)
        if any(json_type(probe) not in covered for probe in PROBES):
            assert rejected, "the root `type` rejects a probe, so the pool must hold a rejected document"
    event(f"pool: {'accepted and rejected' if rejected else 'accepted only'}")
    for source in sorted(rejected):
        event(f"rejected: {source}")


def check_loading(plain: bytes, inline: bytes) -> None:
    """26.11: both schemas load fully supported to equal Schema_Models; the loaded inline model emits ``plain``."""
    loaded_plain, loaded_inline = LOADER.load(plain), LOADER.load(inline)
    for label, loaded in (("plain", loaded_plain), ("inline", loaded_inline)):
        assert loaded.fully_supported, f"{label}: unsupported constructs after loading: {loaded.unsupported}"
        assert loaded.ignored == [], f"{label}: ignored keywords after loading: {loaded.ignored}"
    assert loaded_inline == loaded_plain, "the inline and plain schemas load to different Schema_Models (2.7)"
    assert EMITTER.emit(loaded_inline) == plain, "emitting the loaded inline model does not give the plain bytes"


def check_inline_records(plain_doc: Any, inline_doc: Any, explanation_doc: Mapping[str, Any]) -> None:
    """26.6: each schema object's ``x-provenance`` holds exactly the explanation entries of its Constraints."""
    assert EMITTER.dumps(strip_provenance(inline_doc)) == EMITTER.dumps(plain_doc), (
        "the inline schema differs from the plain one by more than its x-provenance keywords"
    )
    objects = dict(schema_objects(inline_doc))
    expected: dict[str, dict[str, Any]] = {}
    for key, record in explanation_doc["records"].items():
        at = owner(key, objects)
        assert key.startswith(f"{at}/"), f"explanation key {key!r} is not spelled under {at!r} (RFC 6901 escapes)"
        keyword = _tokens(key)[len(_tokens(at))]
        assert keyword in objects[at] and keyword != PROVENANCE, (
            f"explanation key {key!r} names no keyword of its schema object {at or '(root)'}"
        )
        expected.setdefault(at, {})[key[len(at) :]] = record
    actual = {at: obj[PROVENANCE] for at, obj in objects.items() if PROVENANCE in obj}
    for at in sorted(set(actual) | set(expected)):
        assert actual.get(at) == expected.get(at), (
            f"x-provenance of the schema object at {at or '(root)'} does not match its explanation entries:\n"
            f"inline: {json.dumps(actual.get(at), ensure_ascii=False)}\n"
            f"expected: {json.dumps(expected.get(at), ensure_ascii=False)}"
        )
    held = len(actual)
    event(f"x-provenance objects: {'0' if not held else '1' if held == 1 else '2-9' if held < 10 else '10+'}")
    if any(PROVENANCE in obj.get("properties", {}) for obj in objects.values()):
        event("property named x-provenance")


def check_nearest_schema_gate(flags: Mapping[str, Any], config: RunConfiguration) -> None:
    """While ``corpus.nearest_schema`` is unimplemented: it is off, enabling it is a ConfigError, off is a no-op."""
    spec = REGISTRY.technique(NEAREST_SCHEMA)
    assert not spec.implemented and not spec.default_enabled
    assert config.techniques[NEAREST_SCHEMA] is False
    with pytest.raises(ConfigError) as caught:
        RunConfiguration.resolve({**flags, NEAREST_SETTING: True})
    problems = [(problem.subject, problem.reason) for problem in caught.value.problems]
    assert (NEAREST_SCHEMA, "unimplemented") in problems, problems
    disabled = RunConfiguration.resolve({**flags, NEAREST_SETTING: False})
    assert disabled.techniques == config.techniques
    event("nearest-schema: unimplemented (ConfigError when enabled)")


def flags_b(flags: Mapping[str, Any]) -> dict[str, Any]:
    """Run B: run A with inline provenance on; while the lookup is unimplemented, its ``false`` flag toggled."""
    out = {**flags, INLINE: True}
    if not REGISTRY.technique(NEAREST_SCHEMA).implemented:
        if NEAREST_SETTING in out:
            del out[NEAREST_SETTING]
        else:
            out[NEAREST_SETTING] = False
    return out


# Feature: json-schema-inference, Property 43: Annotation and report neutrality
@given(cases())
@example(EXPLICIT_CASES["tagged Variants"])
@example(EXPLICIT_CASES["oneOf beside anyOf"])
@example(EXPLICIT_CASES["co-occurrence"])
@example(EXPLICIT_CASES["Maps"])
@example(EXPLICIT_CASES["relaxations"])
@example(EXPLICIT_CASES["removals"])
@example(EXPLICIT_CASES["awkward names"])
def test_inline_provenance_and_nearest_schema_lookup_are_neutral(case: Case) -> None:
    applied: list[str] = []
    with tempfile.TemporaryDirectory(prefix="p43-bundle-") as directory:
        bundle = write_bundle(Path(directory), case.thresholds)

        def run(flags: Mapping[str, Any], labels: list[str]) -> PipelineResult:
            with injected(case.injections, labels):
                return infer_documents(list(case.docs), RunConfiguration.resolve({**flags, "model_bundle": bundle}))

        a = run(case.flags, applied)
        b = run(flags_b(case.flags), [])
        c = None
        if REGISTRY.technique(NEAREST_SCHEMA).implemented:  # task 40.4
            on = a.config.techniques[NEAREST_SCHEMA]
            c = run({**case.flags, NEAREST_SETTING: not on}, [])

    event(f"scenario: {case.scenario}")
    describe(a.model, a, applied)
    assert a.config.output.inline_provenance is False and b.config.output.inline_provenance is True

    plain = a.schema
    inline = EMITTER.emit(a.model, inline_provenance=True)
    explanation = EMITTER.explanation(a.model, a.pointers)
    assert EMITTER.emit(a.model) == plain, "the pipeline's bytes are not the plain emission of its model"
    assert EMITTER.layout(a.model, inline_provenance=True)[1] == a.pointers, "inline provenance moved Constraints"
    plain_doc, inline_doc = json.loads(plain), json.loads(inline)
    explanation_doc = json.loads(explanation)

    # 26.11: verdicts and loading.
    docs = list(case.docs)
    pool = [("collection", doc) for doc in docs]
    for mutation in case.mutations:
        label, mutant = mutation.apply(docs)
        pool.append((f"mutant ({label})", mutant))
    pool += [("arbitrary", value) for value in case.extras]
    pool += [("probe", probe) for probe in PROBES]
    check_verdicts(plain_doc, inline_doc, pool)
    check_loading(plain, inline)

    # 26.6: inline records equal their explanation entries, one to one.
    check_inline_records(plain_doc, inline_doc, explanation_doc)

    # The output.inline_provenance setting leaves the model and the explanation file as they are.
    assert b.model == a.model, "output.inline_provenance changed the inferred Schema_Model"
    assert EMITTER.explanation(b.model, b.pointers) == explanation, "output.inline_provenance changed the explanation"
    assert a.explanation.dumps() == b.explanation.dumps() == explanation, "the pipeline's explanation file differs"
    assert b.schema == inline, "with output.inline_provenance the pipeline did not emit the inline schema"
    assert json.loads(b.schema) == b.document, "the pipeline's inline document is not its schema bytes"

    # 25.5: the nearest-known-schema lookup leaves the schema and explanation bytes unchanged.
    if c is None:
        check_nearest_schema_gate(case.flags, a.config)
    else:
        assert c.schema == plain, "toggling corpus.nearest_schema changed the schema bytes (25.5)"
        assert EMITTER.explanation(c.model, c.pointers) == explanation, (
            "toggling corpus.nearest_schema changed the explanation bytes (25.5)"
        )
        event(f"nearest-schema: compared on and off (run A {'on' if a.config.techniques[NEAREST_SCHEMA] else 'off'})")
