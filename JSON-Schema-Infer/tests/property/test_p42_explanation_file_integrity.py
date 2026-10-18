"""Property 42: Explanation file integrity (task 17.5).

For any run, every key of the explanation file resolves to exactly one
Constraint in the top-level emitted schema and never to ``x-provenance``, and
every Constraint is the target of exactly one key. Loading the file and
writing it again produces identical bytes.

Explanation files
    The pipeline result carries no explanation file yet, so each check builds
    it as the CLI (task 18.2) will: ``SchemaEmitter().explanation(model,
    pointers)`` over the Schema_Model after soundness and provenance
    finalization (``result.model``) and the pointer map of its layout
    (``result.pointers``). The top-level emitted schema is ``result.schema``,
    re-parsed. While change-point detection is unimplemented every run emits
    one schema; once task 48.2 makes ``result.schema`` the combined schema,
    the same checks apply to it.

Inputs
    - Full runs (``pipeline.infer.infer_documents``) on generated
      collections whose field names need pointer escapes (``~``, ``/``,
      ``a/b~c``, ``~1``), are empty, look like keywords (``x-provenance``,
      ``$ref``, ``type``, ``required``), or are non-ASCII and astral; Records
      with optional and co-occurring fields (``required`` entries,
      ``dependentRequired`` keys), Variants (``anyOf``/``oneOf`` branches,
      some tagged), Maps with Format keys (``propertyNames/format``), Lists,
      Tuples, and mixed types. Technique states come from ``run_configs()``
      (``decide.mdl`` forced off in about one run in three), with
      ``workers = 1``, small Resource_Parameters, ``soundness.max_passes`` of
      1, 2, or 5, and the empty default Model_Bundle or one of two stub
      bundles of hand-set Learned_Thresholds. In about three runs in four,
      edits are injected into the analyzer's model before soundness, as in
      Property 3 (``StructureAnalyzer.infer_model`` patched, records completed
      with ``tests.records.complete_records``): a ``not`` of an accept-all
      ``type`` whose removal moves the ``$ref`` target, with its ``type``
      Constraint, into the root ``$defs``; ``false`` subschemas (replaced by
      a Naive_Merge or by ``true``); a ``oneOf`` that matches twice
      (converted to ``anyOf``, or to an ``allOf`` entry beside an
      ``anyOf``); a closed Record missing a field; ``required`` lists
      reversed with a repeat (deduplicated and sorted at emission); a
      ``required`` entry that sorts first and fails (its removal shifts every
      index); and rarely a ``false`` root (a ``true`` root under removal
      only: a boolean root with no records). Hypothesis favors small draws,
      so the injection kinds and sites are offset by the documents' size.
      Explicit examples reach each pointer kind on every run.
    - Generated Schema_Models in the supported grammar
      (``tests.strategies.schema_models``: every 2.4 keyword, local and
      recursive ``$ref``, ``$defs``, boolean subschemas and roots, retained
      annotations, ``$id``, ``x-`` keywords including a held
      ``x-provenance``, names from ``schema_names``, repeats in ``required``),
      with one synthetic record per Constraint of the layout
      (``tests.strategies.provenance_records``, made distinct per
      Constraint), so the check is not limited to what the Inferrer emits
      today.

Checks
    The emitted document is read independently of ``io.emit``: a walk of the
    JSON document (not the model) enumerates its Constraints at the design's
    granularity (every keyword except ``$schema``, ``$id``, ``$defs``,
    ``x-provenance``, and the retained annotations; one Constraint per
    ``required`` index, ``dependentRequired`` key, and ``anyOf``/``oneOf``
    branch; no Constraint for container entries such as
    ``properties/<name>`` or for boolean subschemas), and each key is
    resolved by RFC 6901 with the schema read at each step.

    - The file is ``{"format": 1, "records": {...}}`` with unique members,
      keys in code-point order, and the canonical encoder's bytes (26.5,
      26.8).
    - 26.7: every key is a non-empty JSON Pointer that resolves in the
      emitted schema to a Constraint (never to ``x-provenance``, ``$schema``,
      ``$defs``, ``$id``, an annotation, a container, a position, a schema
      object, or inside a keyword value), and every enumerated Constraint is
      the target of exactly one key.
    - Each key's Constraint, found through the model's own node at the
      owner's pointer (``SchemaModel.resolve``), is a distinct ConstraintId
      with a record in ``model.provenance``, the file holds that record, and
      every record of the model is in the file. The derived ConstraintId →
      pointer map equals the emitter's (``result.pointers``).
    - 26.8: ``load_explanation`` of the bytes, the text, and the parsed
      value each write the same bytes again, and the loaded file equals
      ``SchemaEmitter.explanation_file``.
    - Inline provenance: the pointer map is unchanged, the inline document
      has the same Constraints, every key still names the same Constraint in
      it, and each schema object holds ``x-provenance`` exactly when it holds
      a Constraint, with one entry per Constraint (the object's pointer plus
      the relative key) equal to the file's record.

    Hypothesis events list the awkward pointer kinds reached: names that are
    empty, need ``~0``/``~1``, are ``x-provenance``, look like keywords, or
    are non-ASCII or astral; ``required`` entries reordered or deduplicated
    at emission; ``dependentRequired`` keys; branches; ``propertyNames``
    contents; Constraints inside ``$defs``; ``$ref``; boolean subschemas and
    roots; and the injections applied.

**Validates: Requirements 26.5, 26.7, 26.8**
"""

# Feature: json-schema-inference, Property 42: Explanation file integrity

from __future__ import annotations

import json
import re
import tempfile
from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Any
from unittest import mock
from urllib.parse import quote

from hypothesis import event, example, given, note
from hypothesis import strategies as st

from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.provenance import ProvenanceRecord
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode, split_pointer
from jsinfer.core.views import View
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import load_explanation
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from tests.records import complete_records
from tests.strategies import TECHNIQUE_SETTINGS, provenance_records, run_configs, schema_models

EMITTER = SchemaEmitter()
MDL = "decide.mdl"
PROVENANCE = "x-provenance"

# ---------------------------------------------------------------------------
# An independent reading of emitted schema documents
# ---------------------------------------------------------------------------

# Keywords that are never Constraints (design, Constraint granularity): $schema,
# the root $id, $defs, the inline keyword, and the retained annotations.
NOT_CONSTRAINTS = frozenset(
    {
        "$schema", "$id", "$defs", PROVENANCE,
        "title", "description", "$comment", "default", "examples", "deprecated", "readOnly", "writeOnly",
    }
)  # fmt: skip
# Draft 2020-12 keywords holding one subschema, a list of subschemas, or named subschemas.
SINGLE = frozenset({"additionalProperties", "propertyNames", "items", "contains", "not", "if", "then", "else"})
LISTS = frozenset({"prefixItems", "allOf", "anyOf", "oneOf"})
MAPS = frozenset({"properties", "patternProperties", "$defs"})
# Keywords whose Constraints are their elements (branches) or their entries.
BRANCHES = frozenset({"anyOf", "oneOf"})
ENTRIES = frozenset({"required", "dependentRequired"})
# Keywords that are one Constraint each and hold a JSON value (the 12 bounds included).
VALUE_CONSTRAINTS = frozenset(
    {
        "$ref", "type", "enum", "const", "format", "pattern", "uniqueItems",
        "minimum", "exclusiveMinimum", "maximum", "exclusiveMaximum", "minLength", "maxLength",
        "minProperties", "maxProperties", "minItems", "maxItems", "minContains", "maxContains",
    }
)  # fmt: skip
KEYWORD_LIKE = NOT_CONSTRAINTS | SINGLE | LISTS | MAPS | ENTRIES | VALUE_CONSTRAINTS

_BAD_TILDE = re.compile(r"~(?![01])")


def escape(token: str | int) -> str:
    """One RFC 6901 reference token."""
    return str(token).replace("~", "~0").replace("/", "~1")


def pointer_tokens(key: Any) -> list[str]:
    """The unescaped tokens of a non-empty JSON Pointer (the root schema object is not a Constraint)."""
    assert isinstance(key, str) and key.startswith("/"), f"{key!r} is not a non-empty JSON Pointer"
    assert _BAD_TILDE.search(key) is None, f"{key!r} has a '~' not followed by '0' or '1'"
    return [token.replace("~1", "/").replace("~0", "~") for token in key[1:].split("/")]


def array_index(token: str, length: int) -> int | None:
    """The array index an RFC 6901 token names, or None (malformed, a leading zero, or out of range)."""
    if not (token.isascii() and token.isdigit()) or (len(token) > 1 and token[0] == "0"):
        return None
    index = int(token)
    return index if index < length else None


@dataclass(frozen=True)
class Located:
    """A Constraint of an emitted document: the pointer of the schema object holding it, its keyword, its entry."""

    owner: str
    keyword: str
    item: str | int | None = None


@dataclass
class Reading:
    """The schema objects, boolean subschemas, and Constraints (pointer -> Constraint) of an emitted document."""

    constraints: dict[str, Located] = field(default_factory=dict)
    objects: dict[str, dict[str, Any]] = field(default_factory=dict)
    booleans: list[str] = field(default_factory=list)


def read_schema(doc: Any) -> Reading:
    """Enumerate the Constraints of a draft 2020-12 schema document by walking the JSON, not a model."""
    reading = Reading()
    stack: list[tuple[str, Any]] = [("", doc)]
    while stack:
        pointer, schema = stack.pop()
        if isinstance(schema, bool):
            reading.booleans.append(pointer)
            continue
        assert isinstance(schema, dict), f"{pointer or '(root)'}: a subschema must be an object or a boolean"
        reading.objects[pointer] = schema
        children: list[tuple[str, Any]] = []
        for keyword, value in schema.items():
            at = f"{pointer}/{escape(keyword)}"
            if keyword in SINGLE:
                children.append((at, value))
            elif keyword in LISTS:
                assert isinstance(value, list), f"{at}: expected an array of subschemas"
                children += [(f"{at}/{index}", child) for index, child in enumerate(value)]
            elif keyword in MAPS:
                assert isinstance(value, dict), f"{at}: expected an object of subschemas"
                children += [(f"{at}/{escape(name)}", child) for name, child in value.items()]
            if keyword in NOT_CONSTRAINTS:
                continue
            entries: list[tuple[str, str | int | None]]
            if keyword == "required":
                assert isinstance(value, list) and all(isinstance(name, str) for name in value), at
                entries = [(f"{at}/{index}", name) for index, name in enumerate(value)]
            elif keyword == "dependentRequired":
                assert isinstance(value, dict), at
                entries = [(f"{at}/{escape(name)}", name) for name in value]
            elif keyword in BRANCHES:
                entries = [(f"{at}/{index}", index) for index in range(len(value))]
            else:
                classified = keyword in SINGLE | LISTS | MAPS | VALUE_CONSTRAINTS or keyword.startswith("x-")
                assert classified, f"{at}: keyword {keyword!r} has no place in the Constraint granularity table"
                entries = [(at, None)]
            for where, item in entries:
                assert where not in reading.constraints, f"{where}: two Constraints at one pointer"
                reading.constraints[where] = Located(pointer, keyword, item)
        stack.extend(reversed(children))
    return reading


Steps = list[tuple[str, str | int | None]]  # (keyword, entry) steps from the root to a Constraint's owner


def locate(doc: Any, key: str) -> tuple[Located, Steps]:
    """The Constraint that the explanation key ``key`` names in ``doc``, and the steps leading to its owner.

    Resolves ``key`` by RFC 6901 with the schema read at each step; fails when
    the key does not resolve or resolves to anything other than a Constraint.
    """
    tokens = pointer_tokens(key)
    schema, owner, steps, i = doc, "", [], 0
    while True:
        assert isinstance(schema, dict), f"{key!r}: {owner or '(root)'} is a boolean schema, which holds nothing"
        assert i < len(tokens), f"{key!r} names a schema object, not a Constraint"
        keyword, rest = tokens[i], tokens[i + 1 :]
        assert keyword in schema, f"{key!r} does not resolve: no keyword {keyword!r} at {owner or '(root)'}"
        assert keyword != PROVENANCE, f"{key!r} resolves into an x-provenance keyword"
        value, at = schema[keyword], f"{owner}/{escape(keyword)}"
        if keyword in MAPS:
            if not rest:
                assert keyword != "$defs", f"{key!r} names $defs, which is not a Constraint"
                return Located(owner, keyword), steps
            name = rest[0]
            assert name in value, f"{key!r} does not resolve: {keyword!r} has no entry {name!r}"
            assert len(rest) > 1, f"{key!r} names the container entry {keyword}/{name}, not a Constraint"
            steps.append((keyword, name))
            schema, owner, i = value[name], f"{at}/{escape(name)}", i + 2
        elif keyword in LISTS:
            if not rest:
                assert keyword not in BRANCHES, f"{key!r} names the whole {keyword!r}; its branches are Constraints"
                return Located(owner, keyword), steps
            index = array_index(rest[0], len(value))
            assert index is not None, f"{key!r} does not resolve: {keyword!r} has no position {rest[0]!r}"
            if len(rest) == 1:
                assert keyword in BRANCHES, f"{key!r} names a {keyword!r} position, not a Constraint"
                return Located(owner, keyword, index), steps
            steps.append((keyword, index))
            schema, owner, i = value[index], f"{at}/{index}", i + 2
        elif keyword in SINGLE:
            if not rest:
                return Located(owner, keyword), steps
            steps.append((keyword, None))
            schema, owner, i = value, at, i + 1
        else:
            assert keyword not in NOT_CONSTRAINTS, f"{key!r} names {keyword!r}, which is not a Constraint"
            if keyword == "required":
                assert len(rest) == 1, f"{key!r} must name one 'required' entry"
                index = array_index(rest[0], len(value))
                assert index is not None, f"{key!r} does not resolve: 'required' has no position {rest[0]!r}"
                return Located(owner, keyword, value[index]), steps
            if keyword == "dependentRequired":
                assert len(rest) == 1, f"{key!r} must name one 'dependentRequired' entry"
                assert rest[0] in value, f"{key!r} does not resolve: no 'dependentRequired' entry {rest[0]!r}"
                return Located(owner, keyword, rest[0]), steps
            classified = keyword in VALUE_CONSTRAINTS or keyword.startswith("x-")
            assert classified, f"{key!r}: keyword {keyword!r} has no place in the Constraint granularity table"
            assert not rest, f"{key!r} points inside the value of {keyword!r}"
            return Located(owner, keyword), steps


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


def unique_members(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """``object_pairs_hook`` that rejects an object with a repeated member name."""
    names = [name for name, _ in pairs]
    repeated = sorted({name for name in names if names.count(name) > 1})
    assert not repeated, f"repeated members in the explanation file: {repeated}"
    return dict(pairs)


def canonical(value: Any) -> bytes:
    """The design's canonical encoding (2.3, 26.8)."""
    return (json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")


def describe(model: SchemaModel, schema: Any, reading: Reading, located: Mapping[str, tuple[Located, Steps]]) -> None:
    """Hypothesis events: the awkward pointer kinds the checked keys reached."""
    if isinstance(schema, bool):
        event("root: boolean (no records)")
    if any(pointer for pointer in reading.booleans):
        event("boolean subschemas (no keys)")
    if not located:
        event("no Constraints")
    for constraint, steps in located.values():
        keyword = constraint.keyword
        names = [entry for step, entry in steps if step in MAPS]
        if keyword == "dependentRequired":
            names.append(constraint.item)
        for name in names:
            assert isinstance(name, str)
            if name == "":
                event("name: empty")
            if "~" in name or "/" in name:
                event("name: needs ~0 or ~1")
            if name == PROVENANCE:
                event("name: x-provenance")
            if name in KEYWORD_LIKE:
                event("name: keyword-like")
            if any(ord(ch) > 0x7F for ch in name):
                event("name: non-ASCII")
            if any(ord(ch) > 0xFFFF for ch in name):
                event("name: astral")
        if keyword == "required":
            event("constraint: required entry")
            raw = model.resolve(constraint.owner).kw["required"]
            if list(raw) != reading.objects[constraint.owner]["required"]:
                event("required: reordered or deduplicated at emission")
        elif keyword in ("dependentRequired", "allOf", "not", "$ref", "propertyNames", "patternProperties"):
            event(f"constraint: {keyword}")
        elif keyword in BRANCHES:
            event(f"constraint: {keyword} branch")
        elif keyword == "additionalProperties" and reading.objects[constraint.owner][keyword] is False:
            event("constraint: additionalProperties false")
        elif keyword.startswith("x-"):
            event("constraint: x- annotation")
        if steps and steps[-1][0] == "propertyNames":
            event("constraint: propertyNames/format" if keyword == "format" else "constraint: inside propertyNames")
        if any(step == "$defs" for step, _ in steps):
            event("constraint: inside $defs")


def check_explanation(model: SchemaModel, schema_bytes: bytes, pointers: Mapping[ConstraintId, str]) -> None:
    """Property 42 for the explanation file of ``model``, whose top-level emitted schema is ``schema_bytes``."""
    data = EMITTER.explanation(model, pointers)
    schema = json.loads(schema_bytes.decode("utf-8"))
    note(f"schema:\n{schema_bytes.decode('utf-8')}")
    note(f"explanation:\n{data.decode('utf-8')}")
    reading = read_schema(schema)

    # The file's form: {"format": 1, "records": {...}}, unique members, sorted keys, canonical bytes (26.5, 26.8).
    parsed = json.loads(data.decode("utf-8"), object_pairs_hook=unique_members)
    assert isinstance(parsed, dict) and list(parsed) == ["format", "records"], f"members: {list(parsed)}"
    assert parsed["format"] == 1 and not isinstance(parsed["format"], bool)
    records = parsed["records"]
    assert isinstance(records, dict)
    assert list(records) == sorted(records), "explanation keys are not in code-point order"
    assert data == canonical(parsed), "the explanation file is not in the canonical encoding"

    # 26.7: each key names exactly one Constraint, and every Constraint is the target of exactly one key.
    located = {key: locate(schema, key) for key in records}
    for key, (constraint, _) in located.items():
        assert reading.constraints.get(key) == constraint, f"{key!r}: the walk found {reading.constraints.get(key)}"
    untargeted = [pointer for pointer in reading.constraints if pointer not in records]
    assert not untargeted, f"Constraints without an explanation key: {untargeted}"

    # The records: one per Constraint of the model, found through the model's own nodes.
    loaded = load_explanation(data)
    assert list(loaded.records) == list(records)
    derived: dict[ConstraintId, str] = {}
    for key, constraint in reading.constraints.items():
        cid = ConstraintId(model.resolve(constraint.owner).nid, constraint.keyword, constraint.item)
        assert cid not in derived, f"{key!r} and {derived.get(cid)!r} name the same Constraint {cid}"
        derived[cid] = key
        record = model.provenance.get(cid)
        assert isinstance(record, ProvenanceRecord), f"{key!r}: no record for {cid} in the model"
        assert loaded.records[key] == record, f"{key!r}: the file's record differs from the model's"
        assert records[key] == record.to_json(), f"{key!r}: the file's record JSON differs from the model's"
    assert set(derived) == set(model.provenance), (
        f"records of the model missing from the file: {sorted(set(model.provenance) - set(derived))}"
    )
    assert derived == dict(pointers), "the emitter's pointer map differs from the independent reading"

    # 26.8: loading and writing again gives the same bytes, from bytes, text, and the parsed value.
    for source in (data, data.decode("utf-8"), json.loads(data)):
        assert load_explanation(source).dumps() == data, f"round trip from {type(source).__name__} changed the bytes"
    assert loaded == EMITTER.explanation_file(model, pointers)

    # Inline provenance: same pointer map and Constraints, one x-provenance entry per Constraint, equal records.
    inline_doc, inline_pointers = EMITTER.layout(model, inline_provenance=True)
    assert inline_pointers == pointers, "inline provenance changed the pointer map"
    inline = json.loads(EMITTER.dumps(inline_doc).decode("utf-8"))
    inline_reading = read_schema(inline)
    assert inline_reading.constraints == reading.constraints, "inline provenance changed the Constraints"
    assert list(inline_reading.objects) == list(reading.objects)
    for key, (constraint, _) in located.items():
        assert locate(inline, key)[0] == constraint, f"{key!r} names another Constraint in the inline document"
    owned: dict[str, set[str]] = {}
    for key, constraint in reading.constraints.items():
        owned.setdefault(constraint.owner, set()).add(key)
    for pointer, obj in inline_reading.objects.items():
        entries = obj.get(PROVENANCE)
        if pointer not in owned:
            assert entries is None, f"{pointer or '(root)'} holds no Constraint but has x-provenance"
            continue
        assert isinstance(entries, dict), f"{pointer or '(root)'} holds Constraints but has no x-provenance object"
        assert {pointer + relative for relative in entries} == owned[pointer], f"{pointer or '(root)'}: entries"
        for relative, record in entries.items():
            assert relative.startswith("/"), f"{pointer or '(root)'}: relative key {relative!r}"
            assert record == records[pointer + relative], f"{pointer + relative!r}: inline record differs"
    if any(PROVENANCE in obj for obj in inline_reading.objects.values()):
        event("inline: x-provenance present")

    describe(model, schema, reading, located)


# ---------------------------------------------------------------------------
# Generated Schema_Models with synthetic records
# ---------------------------------------------------------------------------


@st.composite
def models_with_records(draw: st.DrawFn) -> SchemaModel:
    """A Schema_Model in the supported grammar with one distinct synthetic record per Constraint of its layout."""
    model = draw(schema_models(), label="model")
    _, pointers = EMITTER.layout(model)
    if pointers:
        base = draw(st.lists(provenance_records(), min_size=1, max_size=4), label="records")
        model.provenance = {
            cid: replace(base[i % len(base)], details={**base[i % len(base)].details, "constraint": i})
            for i, cid in enumerate(pointers)
        }
    return model


# Feature: json-schema-inference, Property 42: Explanation file integrity
@given(models_with_records())
def test_explanation_files_of_generated_models_resolve_and_round_trip(model: SchemaModel) -> None:
    _, pointers = EMITTER.layout(model)
    check_explanation(model, EMITTER.emit(model), pointers)


# ---------------------------------------------------------------------------
# Full runs
# ---------------------------------------------------------------------------

# Field names that need pointer escapes, are empty, look like keywords, or are non-ASCII or astral.
FIELD_NAMES = (
    "", "a", "B", "~", "/", "a/b~c", "~1", "x-provenance", "$ref", "type", "required", "é", "😀", "\uffff", "a b",
)  # fmt: skip

_FORMAT_VALUES: Mapping[str, Any] = {
    "date": lambda i: f"20{i % 100:02d}-{i % 12 + 1:02d}-{i % 28 + 1:02d}",
    "date-time": lambda i: f"2024-{i % 12 + 1:02d}-{i % 28 + 1:02d}T{i % 24:02d}:{i % 60:02d}:00Z",
    "email": lambda i: f"user{i}@example.com",
    "uuid": lambda i: f"{i:08x}-1234-4abc-8def-{7919 * i:012x}",
}
_WORDS = ("new", "paid", "~/", "😀", "")
SCALAR_FAMILIES = ("int", "int", "float", "bool", "null", "vocab", "vocab", "text", "format", "latent-int")
MAP_KEY_FAMILIES = ("data", "uuid", "date", "email")
KINDS = ("scalar", "scalar", "record", "record", "map", "list", "tuple", "mix")
PRESENCE = ("always", "always", "sometimes", "group", "group")


@dataclass(frozen=True)
class Shape:
    """A generator of JSON values.

    ``scalar``: ``family`` and its ``params``. ``record``: per Variant, its
    fields as ``(name, presence, child)`` with presence ``always``,
    ``sometimes``, or ``group`` (the Variant's group fields are present
    together), and an optional Discriminator field ``tag``. ``map``: the key
    ``family`` and the value shape. ``list``: the element shape. ``tuple``:
    one shape per position. ``mix``: the alternatives.
    """

    kind: str
    family: str = ""
    params: tuple[Any, ...] = ()
    variants: tuple[tuple[tuple[str, str, Shape], ...], ...] = ()
    tag: str = ""
    children: tuple[Shape, ...] = ()


@st.composite
def scalar_shapes(draw: st.DrawFn) -> Shape:
    family = draw(st.sampled_from(SCALAR_FAMILIES), label="scalar family")
    if family == "int":
        return Shape("scalar", family, (draw(st.sampled_from([0, -5, 100])), draw(st.sampled_from([0, 9, 999]))))
    if family == "vocab":
        members = draw(st.lists(st.sampled_from(_WORDS), min_size=1, max_size=3, unique=True), label="members")
        return Shape("scalar", family, tuple(members))
    if family == "format":
        return Shape("scalar", family, (draw(st.sampled_from(tuple(_FORMAT_VALUES))),))
    return Shape("scalar", family)


def scalar_values(shape: Shape) -> st.SearchStrategy[Any]:
    family, params = shape.family, shape.params
    if family == "int":
        return st.integers(params[0], params[0] + params[1])
    if family == "float":
        return st.integers(-40, 400).map(lambda k: k / 8)
    if family == "bool":
        return st.booleans()
    if family == "null":
        return st.none()
    if family == "vocab":
        return st.sampled_from(params)
    if family == "text":
        return st.text(alphabet='ab~/"😀é', max_size=5)
    if family == "format":
        return st.integers(0, 500).map(_FORMAT_VALUES[params[0]])
    assert family == "latent-int"
    return st.integers(-50, 5000).map(str)


def child_shapes(depth: int) -> st.SearchStrategy[Shape]:
    return shapes(depth - 1) if depth > 0 else scalar_shapes()


@st.composite
def record_shapes(draw: st.DrawFn, depth: int) -> Shape:
    """One to three Variants with disjoint field names, maybe a Discriminator field."""
    n_variants = draw(st.sampled_from([1, 1, 2, 3]), label="variants")
    names = list(draw(st.permutations(FIELD_NAMES), label="name order"))
    tag = names.pop(0) if n_variants > 1 and draw(st.booleans(), label="tagged") else ""
    variants = []
    for _ in range(n_variants):
        size = draw(st.integers(1 if n_variants > 1 else 0, 4), label="fields")
        own, names = names[:size], names[size:]
        variants.append(
            tuple(
                (name, draw(st.sampled_from(PRESENCE), label=f"{name!r} presence"), draw(child_shapes(depth)))
                for name in own
            )
        )
    return Shape("record", variants=tuple(variants), tag=tag)


@st.composite
def shapes(draw: st.DrawFn, depth: int, mix: bool = True) -> Shape:
    kinds = KINDS if depth > 0 else ("scalar",)
    kind = draw(st.sampled_from(kinds if mix else tuple(k for k in kinds if k != "mix")), label="kind")
    if kind == "scalar":
        return draw(scalar_shapes())
    if kind == "record":
        return draw(record_shapes(depth))
    if kind == "map":
        family = draw(st.sampled_from(MAP_KEY_FAMILIES), label="map key family")
        return Shape("map", family=family, children=(draw(child_shapes(depth)),))
    if kind == "list":
        return Shape("list", children=(draw(child_shapes(depth)),))
    if kind == "tuple":
        return Shape("tuple", children=tuple(draw(st.lists(child_shapes(depth), min_size=1, max_size=3))))
    assert kind == "mix"
    return Shape("mix", children=tuple(draw(st.lists(shapes(depth, mix=False), min_size=2, max_size=3))))


@st.composite
def root_shapes(draw: st.DrawFn) -> Shape:
    """Mostly Records; also arrays of Records, Records among other types, and any shape."""
    root = draw(st.sampled_from(["record", "record", "record", "list", "mix", "any"]), label="root shape")
    if root == "any":
        return draw(shapes(2))
    record = draw(record_shapes(2), label="root record")
    if root == "record":
        return record
    if root == "list":
        return Shape("list", children=(record,))
    return Shape("mix", children=(record, draw(scalar_shapes())))


@st.composite
def instances(draw: st.DrawFn, shape: Shape) -> Any:
    kind = shape.kind
    if kind == "scalar":
        return draw(scalar_values(shape))
    if kind == "record":
        index = draw(st.integers(0, len(shape.variants) - 1), label="variant")
        group = draw(st.booleans(), label="group present")
        out: dict[str, Any] = {shape.tag: f"v{index}"} if shape.tag else {}
        for name, presence, child in shape.variants[index]:
            if presence == "always" or (group if presence == "group" else draw(st.booleans(), label=repr(name))):
                out[name] = draw(instances(child))
        return out
    if kind == "map":
        keys = draw(st.lists(_map_keys(shape.family), min_size=1, max_size=4, unique=True), label="map keys")
        return {key: draw(instances(shape.children[0])) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(shape.children[0]), max_size=4), label="elements")
    if kind == "tuple":
        full = draw(st.integers(0, 3), label="full tuple") != 0
        length = len(shape.children) if full else draw(st.integers(0, len(shape.children)), label="tuple length")
        return [draw(instances(child)) for child in shape.children[:length]]
    assert kind == "mix"
    return draw(instances(shape.children[draw(st.integers(0, len(shape.children) - 1), label="alternative")]))


def _map_keys(family: str) -> st.SearchStrategy[str]:
    if family == "data":
        return st.integers(0, 999).map(lambda i: f"u{i}")
    return st.integers(0, 500).map(_FORMAT_VALUES[family])


@st.composite
def collections(draw: st.DrawFn) -> list[Any]:
    root = draw(root_shapes(), label="root shape")
    return [draw(instances(root), label="document") for _ in range(draw(st.integers(5, 40), label="documents"))]


# -- over-tight injections (as in Property 3) -----------------------------------

_FRAGMENT_SAFE = "/!$&'()*+,;=:@"  # RFC 3986 fragment characters besides unreserved ones
ALL_TYPES = ("null", "boolean", "integer", "number", "string", "array", "object")
FIRST_NAME = "!missing"  # sorts before every field name except "", so its removal shifts the other indices
# A run applies none to three consecutive kinds of this cycle (none in about one run in four).
INJECTIONS = ("false-subschema", "one-of", "closed", "relocated-ref", "required-order", "required-first", "false-root")
# Applied last, in this order: no later edit moves or drops a reference's target, and a false root ends the edits.
_LAST = ("relocated-ref", "false-root")


def editable(pointer: str, node: SchemaNode) -> bool:
    """A keyword schema outside every injected ``not`` (no field is named "not")."""
    return node.boolean is None and "not" not in split_pointer(pointer)


@dataclass(frozen=True)
class Injection:
    """One edit of the analyzer's model, at the ``pick``-th eligible node (modulo their count)."""

    kind: str
    pick: int
    arg: int

    def eligible(self, pointer: str, node: SchemaNode) -> bool:
        kw = node.kw
        if not editable(pointer, node):
            return False
        if self.kind == "false-subschema":
            return bool(kw.get("properties") or kw.get("prefixItems") or kw.get("anyOf"))
        if self.kind == "one-of":
            return "oneOf" not in kw and ("anyOf" in kw or self.arg % 2 == 0)
        if self.kind == "closed":
            return bool(kw.get("properties")) and "additionalProperties" not in kw
        if self.kind == "required-order":
            return len(set(kw.get("required", ()))) >= 2
        if self.kind == "required-first":
            return "properties" in kw and FIRST_NAME not in kw.get("required", ())
        if self.kind == "relocated-ref":
            return "not" not in kw
        assert self.kind == "false-root"
        return pointer == "" and self.arg % 4 == 0  # rarely: it replaces the whole inferred schema

    def apply(self, model: SchemaModel) -> str | None:
        """Edit ``model`` in place; the label of the edit, or None when no node is eligible."""
        nodes = [(pointer, node) for pointer, node in model.walk() if self.eligible(pointer, node)]
        if not nodes:
            return None
        pointer, node = nodes[self.pick % len(nodes)]
        kw, arg, kind = node.kw, self.arg, self.kind
        if kind == "false-subschema":
            slots: list[tuple[Any, Any]] = [(kw["properties"], name) for name in sorted(kw.get("properties", {}))]
            slots += [(kw[key], i) for key in ("prefixItems", "anyOf") for i in range(len(kw.get(key, [])))]
            container, slot = slots[arg % len(slots)]
            container[slot] = SchemaNode(boolean=False)
        elif kind == "one-of":
            kw["oneOf"] = [SchemaNode(), SchemaNode()]  # two matching branches reject every instance
            return "one-of beside anyOf" if "anyOf" in kw else "one-of"
        elif kind == "closed":
            names = sorted(kw["properties"])
            del kw["properties"][names[arg % len(names)]]
            kw["additionalProperties"] = SchemaNode(boolean=False)
        elif kind == "required-order":  # holds as before; the emitter deduplicates and sorts
            names = list(kw["required"])
            kw["required"] = [*reversed(names), names[arg % len(names)]]
        elif kind == "required-first":  # fails on every object, so soundness removes it
            kw["required"] = [*kw.get("required", []), FIRST_NAME]
        elif kind == "relocated-ref":
            # A `not` of a schema that accepts every instance rejects every instance, so soundness removes it,
            # which moves the `$ref` target, and its `type` Constraint, into the root `$defs`.
            # The site is chosen before the target exists, so no target references itself.
            sites = [site for at, site in model.walk() if editable(at, site) and "$ref" not in site.kw]
            kw["not"] = SchemaNode.from_json({"type": list(ALL_TYPES)})
            sites[arg % len(sites)].kw["$ref"] = "#" + quote(f"{pointer}/not", safe=_FRAGMENT_SAFE)
        else:
            assert kind == "false-root"
            model.root = SchemaNode(boolean=False)
            model.provenance.clear()
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


# -- Model_Bundles, configurations, and cases -----------------------------------

# Hand-set Learned_Thresholds of the implemented Techniques, inside their admissible ranges.
STUB_THRESHOLDS: Mapping[str, Mapping[str, int | float]] = {
    "loose": {
        "lsh.keyset_jaccard": 0.5,
        "dbscan.eps": 0.34,
        "dbscan.min_samples": 1,
        "hdbscan.min_cluster_size": 2,
        "cooccur.min_support": 1,
        "cooccur.min_confidence": 0.5,
    },
    "tight": {
        "lsh.keyset_jaccard": 1.0,
        "dbscan.eps": 1e-9,
        "dbscan.min_samples": 2,
        "hdbscan.min_cluster_size": 3,
        "cooccur.min_support": 2,
        "cooccur.min_confidence": 0.0,
    },
}


def write_bundle(root: Path, name: str) -> str:
    """A Model_Bundle directory with the ``name`` stub thresholds and no models."""
    manifest = {"format": BUNDLE_FORMAT, "bundle_id": f"p42-{name}", "artifacts": {}}
    records = {
        threshold: ThresholdRecord.from_spec(
            THRESHOLDS[threshold], value=value, objective_value=0.0, n_examples=1, training_data="p42-stub"
        ).to_json()
        for threshold, value in STUB_THRESHOLDS[name].items()
    }
    (root / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
    (root / THRESHOLDS_FILE).write_text(json.dumps(records), encoding="utf-8")
    return str(root)


@dataclass(frozen=True)
class Case:
    """One full run: documents, the configuration, the Model_Bundle, and the injections."""

    docs: tuple[Any, ...]
    config: RunConfiguration
    bundle: str  # "default" or a STUB_THRESHOLDS name
    injections: tuple[Injection, ...]


@st.composite
def cases(draw: st.DrawFn) -> Case:
    docs = tuple(draw(collections(), label="documents"))
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    base: dict[str, Any] = {
        "workers": 1,
        "soundness.max_passes": draw(st.sampled_from([1, 2, 5]), label="soundness.max_passes"),
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(st.sampled_from([2, 5_000]), label="sample.dbscan"),
        "sample.hdbscan": draw(st.sampled_from([2, 5_000]), label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(1, 4), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([5, 100]), label="variants.em_max_iter"),
    }
    # Hypothesis favors small draws (and the first of sampled choices); offsets taken from the documents' size,
    # which varies from run to run, spread the scenario, the injection kinds, and the injection sites.
    size = len(json.dumps(docs))
    spread = st.integers(0, 2**16).map(lambda k: k + size)
    if draw(spread, label="scenario") % 3 == 0:
        base[technique_setting(MDL)] = False  # Judgment_Calls by priority; groupings stay split (5.11)
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    bundle = draw(st.sampled_from(["loose", "loose", "tight", "default"]), label="bundle")
    count, first = draw(spread, label="injections") % 4, draw(spread, label="first injection")
    kinds = sorted({INJECTIONS[(first + i) % len(INJECTIONS)] for i in range(count)}, key=_injection_order)
    injections = tuple(Injection(kind, draw(spread, label=f"{kind} site"), draw(spread, label=kind)) for kind in kinds)
    return Case(docs, config, bundle, injections)


def _injection_order(kind: str) -> int:
    return _LAST.index(kind) + 1 if kind in _LAST else 0


def _explicit(docs: Sequence[Any], *injections: Injection, mdl: bool = True, **flags: Any) -> Case:
    """An explicit case under the default Technique states (``decide.mdl`` as given) and the loose thresholds."""
    config = RunConfiguration.resolve({"workers": 1, technique_setting(MDL): mdl, **flags})
    return Case(tuple(docs), config, "loose", injections)


def _uuid(i: int) -> str:
    return _FORMAT_VALUES["uuid"](i)


# Explicit examples that reach every awkward pointer kind whatever the generated examples reach.
EXPLICIT_CASES: Mapping[str, Case] = {
    # Names that need escapes, are empty, look like keywords, or are non-ASCII and astral.
    "awkward names": _explicit(
        [
            {
                **{name: i for name in ("", "~", "/", "a/b~c", "~1")},
                "x-provenance": f"s{i}",
                "$ref": [i, i + 1],
                "type": {"required": i % 2 == 0, "é": None},
                **({"😀": 1.5} if i % 3 == 0 else {}),
            }
            for i in range(30)
        ]
    ),
    # Tagged Variants (Discriminator "$ref") resolved by priority stay split and disjoint: `oneOf` branches.
    "tagged Variants": _explicit(
        [
            {"$ref": "a", "~": i, "when": _FORMAT_VALUES["date"](i)} if i % 2 else {"$ref": "b", "/": [i]}
            for i in range(20)
        ],
        mdl=False,
    ),
    # Fields present together give `dependentRequired` entries keyed by the empty name and by a name needing ~0.
    "co-occurrence: empty key": _explicit(
        [
            {"x-provenance": i, **({"": 1, "😀": 2} if i % 3 == 0 else {}), **({"😀": 3} if i % 3 == 1 else {})}
            for i in range(90)
        ]
    ),
    "co-occurrence: escaped key": _explicit(
        [{"a": i, **({"~": 1, "/": 2} if i % 3 == 0 else {}), **({"/": 3} if i % 3 == 1 else {})} for i in range(90)]
    ),
    # uuid-keyed Maps: `additionalProperties` with a value schema and `propertyNames` with `format`.
    "Maps": _explicit([{"m/~": {_uuid(7 * i + j): f"{j}.5" for j in range(i % 4 + 1)}} for i in range(20)]),
    # Untagged Variants stay split under `anyOf`; an always-failing `oneOf` beside it becomes an `allOf` entry.
    "oneOf beside anyOf": _explicit(
        [{"t": i % 4, "": "s"} if i % 2 else {"u": [i % 3], "😀": None} for i in range(20)],
        Injection("one-of", 0, 1),
        mdl=False,
    ),
    # Removal only: a `false` property becomes `true`, and removing `not` moves the `$ref` target into `$defs`.
    "removals": _explicit(
        [{"a": {"b": i, "c": "x"}, "d": [str(i)]} for i in range(10)],
        Injection("false-subschema", 1, 1),
        Injection("relocated-ref", 2, 3),
        **{"soundness.max_passes": 1},
    ),
    # Required lists reversed with a repeat, and an entry sorting first whose removal shifts the indices.
    "required indices": _explicit(
        [{"b": i, "a/b": str(i), "": None, "~": [i]} for i in range(12)],
        Injection("required-order", 0, 1),
        Injection("required-first", 0, 0),
    ),
    # A false root under removal only becomes `true`: a boolean root, whose explanation file has no records.
    "boolean root": _explicit(
        [{"a": i} for i in range(5)], Injection("false-root", 0, 0), **{"soundness.max_passes": 1}
    ),
}


def run(case: Case, applied: list[str]) -> PipelineResult:
    with tempfile.TemporaryDirectory(prefix="p42-bundle-") as directory, injected(case.injections, applied):
        bundle = DEFAULT_MODEL_BUNDLE if case.bundle == "default" else write_bundle(Path(directory), case.bundle)
        return infer_documents(list(case.docs), replace(case.config, model_bundle=bundle))


# Feature: json-schema-inference, Property 42: Explanation file integrity
@given(cases())
@example(EXPLICIT_CASES["awkward names"])
@example(EXPLICIT_CASES["tagged Variants"])
@example(EXPLICIT_CASES["co-occurrence: empty key"])
@example(EXPLICIT_CASES["co-occurrence: escaped key"])
@example(EXPLICIT_CASES["Maps"])
@example(EXPLICIT_CASES["oneOf beside anyOf"])
@example(EXPLICIT_CASES["removals"])
@example(EXPLICIT_CASES["required indices"])
@example(EXPLICIT_CASES["boolean root"])
def test_explanation_files_of_inferred_schemas_resolve_and_round_trip(case: Case) -> None:
    applied: list[str] = []
    result = run(case, applied)
    for label in sorted(set(applied)):
        event(f"injected: {label}")
    summary = result.soundness.summary
    if summary.relaxations:
        event("soundness: relaxed")
    if summary.removals:
        event("soundness: removed")
    event(f"bundle: {case.bundle}")
    # The explanation file is keyed by the layout of the final model, which is what the pipeline emitted.
    assert EMITTER.layout(result.model) == (result.document, result.pointers)
    assert EMITTER.dumps(result.document) == result.schema
    check_explanation(result.model, result.schema, result.pointers)
