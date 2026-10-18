"""Unit tests for the explanation file and inline ``x-provenance`` (task 17.2; Requirements 26.5–26.8, 26.11).

Covers ``io/emit.py`` (``ExplanationFile``, ``SchemaEmitter.explanation_file``,
``SchemaEmitter.explanation``, and ``inline_provenance``) and ``io/load.py``
(``load_explanation`` and ``ExplanationLoadError``).
"""

from __future__ import annotations

import copy
import json
import pickle
from typing import Any

import pytest

from jsinfer.core.determinism import encode_canonical
from jsinfer.core.provenance import (
    MDL_COMPARISON,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    ProvenanceRecord,
    RelaxationRecord,
    Resolution,
    SampleInfo,
    Support,
    fallback_technique,
)
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode, new_nid, split_pointer
from jsinfer.io.emit import EXPLANATION_FORMAT, PROVENANCE_KEYWORD, EmitError, ExplanationFile, SchemaEmitter
from jsinfer.io.load import EXPLANATION_LOAD_CHECKS, ExplanationLoadError, SchemaLoader, load_explanation
from jsinfer.io.validator import Draft, Validator

EMITTER = SchemaEmitter()

# Property names that need pointer escapes ("a/b", "m~n"), the empty name, a
# name that looks like the inline keyword, and a node without Constraints.
SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "": {"type": "string", "minLength": 1},
        "a/b": {"type": "integer", "minimum": 0, "maximum": 9},
        "m~n": {"enum": ["y", "x"]},
        "x-provenance": {"type": "boolean"},
        "list": {"type": "array", "items": {"type": "number"}, "maxItems": 3},
        "pair": {"type": "array", "prefixItems": [{"type": "string"}, True], "items": False},
        "map": {"type": "object", "additionalProperties": {"type": "integer"}, "propertyNames": {"format": "uuid"}},
        "node": {"$ref": "#/$defs/node"},
        "free": {"title": "no Constraint here"},
    },
    "additionalProperties": False,
    "required": ["a/b", "", "a/b"],
    "dependentRequired": {"a/b": ["m~n"], "": ["a/b"]},
    "anyOf": [{"required": ["list"]}, True],
    "not": {"required": ["x-provenance", "pair"]},
    "$defs": {"node": {"type": "object", "properties": {"next": {"$ref": "#/$defs/node"}}, "x-latent-type": "tree"}},
}

RICH_RECORD = ProvenanceRecord(
    technique="decide.mdl",
    support=Support(7, 9),
    confidence=0.875,
    resolution=Resolution(
        "mdl",
        0,
        (
            CandidateScore("decide.mdl", "record", 12.5, model_bits=2.0, data_bits=10.5, n_rejected=0, n_scored=9),
            CandidateScore("decide.mdl", "map", 40.0),
        ),
    ),
    cutoff=Cutoff(MDL_COMPARISON, scores=(CandidateScore("decide.mdl", "record", 12.5),)),
    sample=SampleInfo(5, 9),
    changes=(RelaxationRecord({"maximum": 5}, {"maximum": 9}, (3, 1)),),
    # Awkward numbers: integral floats (stored as ints, a huge one included), -0.0, a subnormal, a big int.
    details={"é😀": [2.0, -0.0, 5e-324, 1.7976931348623157e308, 2**70], "a/b~": {"z": None, "a": True}},
)


def model(schema: Any) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(schema))


def record_for(index: int, cid: ConstraintId) -> ProvenanceRecord:
    """A distinct record per Constraint, covering baseline, fallback, and rich records."""
    support = Support(index, index + 1)
    if cid.keyword in ("type", "required"):
        return ProvenanceRecord.baseline(cid.keyword, support)
    if index % 3 == 0:
        return ProvenanceRecord(
            RICH_RECORD.technique,
            support,
            RICH_RECORD.confidence,
            resolution=RICH_RECORD.resolution,
            cutoff=RICH_RECORD.cutoff,
            sample=RICH_RECORD.sample,
            changes=RICH_RECORD.changes,
            details={**RICH_RECORD.details, "index": index},
        )
    if index % 3 == 1:
        return ProvenanceRecord.laplace(
            fallback_technique("record_or_map"),
            support,
            fallback=FallbackInfo("decide.mdl", "disabled"),
            resolution=Resolution("fallback", 0, (CandidateScore(None, "record", None),)),
        )
    return ProvenanceRecord("values.bounds", support, 0.25, details={"index": index})


def with_records(schema: Any) -> tuple[SchemaModel, dict[ConstraintId, str]]:
    """A model of ``schema`` with one record per Constraint, and its pointer map."""
    m = model(schema)
    _, pointers = EMITTER.layout(m)
    m.provenance = {cid: record_for(index, cid) for index, cid in enumerate(pointers)}
    return m, pointers


def at(doc: Any, pointer: str) -> Any:
    """The value at a JSON Pointer in a plain JSON document (KeyError or IndexError when absent)."""
    value = doc
    for token in split_pointer(pointer):
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def strip_inline(value: Any) -> Any:
    """``value`` without any ``x-provenance`` member of a schema object (names under properties are kept)."""
    if isinstance(value, list):
        return [strip_inline(item) for item in value]
    if not isinstance(value, dict):
        return value
    out: dict[str, Any] = {}
    for key, item in value.items():
        if key == PROVENANCE_KEYWORD:
            continue
        if key in ("properties", "patternProperties", "$defs"):
            out[key] = {name: strip_inline(child) for name, child in item.items()}
        elif key in ("enum", "const", "required", "dependentRequired", "type", "x-latent-type", "title"):
            out[key] = item
        else:
            out[key] = strip_inline(item)
    return out


# ---------------------------------------------------------------------------
# The explanation file
# ---------------------------------------------------------------------------


def test_explanation_file_format_and_sorting() -> None:
    m, pointers = with_records(SCHEMA)
    data = EMITTER.explanation(m, pointers)
    doc = json.loads(data)
    assert list(doc) == ["format", "records"]
    assert doc["format"] == EXPLANATION_FORMAT == 1
    keys = list(doc["records"])
    assert keys == sorted(keys)  # code-point order (Python str order)
    assert sorted(keys) == sorted(pointers.values())
    for cid, pointer in pointers.items():
        assert doc["records"][pointer] == json.loads(encode_canonical(m.provenance[cid].to_json()))
    # Written by the canonical encoder, keeping the parsed key order at every level.
    assert data == encode_canonical(doc)
    assert data == EMITTER.explanation_file(m, pointers).dumps()


def test_explanation_keys_are_the_layout_pointers_with_escapes_and_empty_names() -> None:
    m, pointers = with_records(SCHEMA)
    doc, _ = EMITTER.layout(m)
    keys = set(EMITTER.explanation_file(m, pointers).records)
    expected = {
        "/properties//type",
        "/properties//minLength",
        "/properties/a~1b/type",
        "/properties/a~1b/maximum",
        "/properties/m~0n/enum",
        "/properties/x-provenance/type",
        "/properties/map/propertyNames",
        "/properties/map/propertyNames/format",
        "/properties/pair/prefixItems",
        "/properties/pair/items",
        "/properties/node/$ref",
        "/required/0",
        "/required/1",
        "/dependentRequired/",
        "/dependentRequired/a~1b",
        "/anyOf/0",
        "/anyOf/1",
        "/anyOf/0/required/0",
        "/not",
        "/not/required/1",
        "/additionalProperties",
        "/$defs/node/x-latent-type",
    }
    assert expected <= keys
    assert keys == set(pointers.values())
    assert at(doc, "/required/0") == "" and at(doc, "/required/1") == "a/b"
    # Every key resolves in the emitted document, and none to x-provenance or a container entry.
    for pointer in keys:
        at(doc, pointer)
        assert split_pointer(pointer)[-1] != PROVENANCE_KEYWORD or split_pointer(pointer)[-2] == "properties"
    assert not any(pointer.startswith("/properties/free") for pointer in keys)


def test_explanation_bytes_do_not_depend_on_construction_order() -> None:
    first, pointers = with_records(SCHEMA)
    by_pointer = {pointer: first.provenance[cid] for cid, pointer in pointers.items()}

    def reordered(value: Any) -> Any:
        if isinstance(value, dict):
            return {key: reordered(value[key]) for key in reversed(list(value))}
        return value

    second = model(reordered(copy.deepcopy(SCHEMA)))
    _, second_pointers = EMITTER.layout(second)
    # Records inserted in the reverse order of the first model's.
    second.provenance = {cid: by_pointer[second_pointers[cid]] for cid in reversed(list(second_pointers))}
    data = EMITTER.explanation(first, pointers)
    assert EMITTER.explanation(second, second_pointers) == data
    assert EMITTER.explanation(first, pointers) == data
    assert EMITTER.emit(second, inline_provenance=True) == EMITTER.emit(first, inline_provenance=True)


def test_explanation_round_trips_through_load_explanation() -> None:
    m, pointers = with_records(SCHEMA)
    explanation = EMITTER.explanation_file(m, pointers)
    data = explanation.dumps()
    for source in (data, data.decode("utf-8"), json.loads(data)):
        loaded = load_explanation(source)
        assert loaded == explanation
        assert loaded.dumps() == data
    loaded = load_explanation(data)
    for cid, pointer in pointers.items():
        assert loaded.records[pointer] == m.provenance[cid]
    # Member order and number spelling in the file do not matter to the loaded records.
    respelled = json.loads(data)
    respelled["records"] = dict(reversed(list(respelled["records"].items())))
    respelled = {"records": respelled["records"], "format": 1}
    assert load_explanation(json.dumps(respelled)).dumps() == data


def test_boolean_root_has_an_empty_explanation() -> None:
    m = model(True)
    doc, pointers = EMITTER.layout(m, inline_provenance=True)
    assert (doc, pointers) == (True, {})
    data = EMITTER.explanation(m, pointers)
    assert data == b'{\n  "format": 1,\n  "records": {}\n}\n'
    assert load_explanation(data) == ExplanationFile({})


def test_explanation_file_checks_its_keys_and_compares_by_content() -> None:
    record = ProvenanceRecord.baseline("type", Support(1, 1))
    with pytest.raises(ValueError, match="root schema object"):
        ExplanationFile({"": record})
    with pytest.raises(ValueError, match="start with '/'"):
        ExplanationFile({"type": record})
    with pytest.raises(ValueError, match="'~' not followed"):
        ExplanationFile({"/a~2": record})
    with pytest.raises(TypeError, match="str JSON Pointers"):
        ExplanationFile({1: record})  # type: ignore[dict-item]
    with pytest.raises(TypeError, match="ProvenanceRecord"):
        ExplanationFile({"/type": record.to_json()})  # type: ignore[dict-item]
    with pytest.raises(TypeError, match="mapping"):
        ExplanationFile([("/type", record)])  # type: ignore[arg-type]

    other = ProvenanceRecord.baseline("type", Support(0, 1))
    a = ExplanationFile({"/type": record, "/items/type": other})
    b = ExplanationFile({"/items/type": other, "/type": record})
    assert a == b and a.dumps() == b.dumps()
    assert list(a.records) == ["/items/type", "/type"]
    assert a != ExplanationFile({"/type": other, "/items/type": record})
    assert pickle.loads(pickle.dumps(a)) == a
    with pytest.raises(TypeError):
        hash(a)


# ---------------------------------------------------------------------------
# Record checks: exactly one record per Constraint (26.1, 26.7)
# ---------------------------------------------------------------------------


def test_a_constraint_without_a_record_cannot_be_written() -> None:
    m, pointers = with_records(SCHEMA)
    missing = next(cid for cid, pointer in pointers.items() if pointer == "/properties/a~1b/maximum")
    del m.provenance[missing]
    for build in (
        lambda: EMITTER.explanation(m, pointers),
        lambda: EMITTER.layout(m, inline_provenance=True),
        lambda: EMITTER.emit(m, inline_provenance=True),
    ):
        with pytest.raises(EmitError, match="has no Provenance_Record") as info:
            build()
        assert info.value.pointer == "/properties/a~1b/maximum"
    # Inline provenance off needs no records at all.
    assert EMITTER.emit(m) == EMITTER.emit(model(SCHEMA))


def test_a_record_that_names_no_constraint_cannot_be_written() -> None:
    m, pointers = with_records(SCHEMA)
    stale = ConstraintId(new_nid(), "maximum")
    m.provenance[stale] = ProvenanceRecord("values.bounds", Support(1, 1), 1.0)
    for build in (lambda: EMITTER.explanation(m, pointers), lambda: EMITTER.layout(m, inline_provenance=True)):
        with pytest.raises(EmitError, match="names no Constraint of the emitted schema") as info:
            build()
        assert repr(stale) in str(info.value)


def test_record_values_and_pointer_maps_are_checked() -> None:
    m, pointers = with_records(SCHEMA)
    cid = next(iter(pointers))
    m.provenance[cid] = {"technique": "decide.mdl"}  # type: ignore[assignment]
    with pytest.raises(EmitError, match="must be a ProvenanceRecord") as info:
        EMITTER.explanation(m, pointers)
    assert info.value.pointer == pointers[cid]
    with pytest.raises(EmitError, match="must be a ProvenanceRecord"):
        EMITTER.layout(m, inline_provenance=True)

    m, pointers = with_records(SCHEMA)
    first, second = list(pointers)[:2]
    shared = {first: "/type", second: "/type"} | {c: p for c, p in pointers.items() if c not in (first, second)}
    with pytest.raises(EmitError, match="share this pointer"):
        EMITTER.explanation(m, shared)
    with pytest.raises(TypeError):
        EMITTER.explanation(m, {"/type": first})  # type: ignore[dict-item]
    with pytest.raises(TypeError):
        EMITTER.layout(m, inline_provenance=1)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Inline provenance (26.6, 26.11)
# ---------------------------------------------------------------------------


def test_inline_provenance_matches_the_explanation_entries() -> None:
    m, pointers = with_records(SCHEMA)
    explanation = EMITTER.explanation_file(m, pointers).to_json()["records"]
    inline_doc, inline_pointers = EMITTER.layout(m, inline_provenance=True)
    assert list(inline_pointers.items()) == list(pointers.items())  # same map, same order

    pointer_of_nid = {cid.nid: pointer for cid, pointer in pointers.items()}
    seen_inline = 0
    for base, node in m.walk():  # the layout keeps every subschema at its model pointer
        emitted = at(inline_doc, base)
        if node.boolean is not None:
            assert emitted is node.boolean
            continue
        expected = {
            pointer[len(base) :]: explanation[pointer] for cid, pointer in pointers.items() if cid.nid == node.nid
        }
        if not expected:
            assert PROVENANCE_KEYWORD not in emitted, base
            continue
        assert node.nid in pointer_of_nid
        inline = emitted[PROVENANCE_KEYWORD]
        assert list(emitted)[-1] == PROVENANCE_KEYWORD  # last keyword
        assert list(inline) == sorted(inline)
        assert inline == expected
        for key, record in inline.items():
            assert key.startswith("/")
            assert explanation[base + key] == record  # the object's pointer + key is the explanation key
        seen_inline += 1
    assert seen_inline >= 10
    assert PROVENANCE_KEYWORD not in at(inline_doc, "/properties/free")
    named = at(inline_doc, "/properties/x-provenance")  # a property named like the keyword
    assert named[PROVENANCE_KEYWORD] == {"/type": explanation["/properties/x-provenance/type"]}
    root_keys = set(at(inline_doc, "")[PROVENANCE_KEYWORD])
    assert {"/required/0", "/anyOf/1", "/dependentRequired/a~1b", "/dependentRequired/", "/not"} <= root_keys


def test_inline_provenance_off_leaves_the_bytes_unchanged() -> None:
    m, pointers = with_records(SCHEMA)
    plain = EMITTER.emit(model(SCHEMA))  # a model without any records
    assert EMITTER.emit(m) == plain
    assert EMITTER.emit(m, inline_provenance=False) == plain
    assert EMITTER.layout(m)[0] == EMITTER.layout(m, inline_provenance=False)[0]
    inline_doc, _ = EMITTER.layout(m, inline_provenance=True)
    assert encode_canonical(strip_inline(inline_doc)) == plain
    # The inline document still validates against the 2020-12 meta-schema.
    assert Validator().meta_errors(inline_doc, Draft.DRAFT202012) == []


VALID = {"": "s", "a/b": 3, "m~n": "x", "list": [1.5]}
INSTANCES: list[Any] = [
    VALID,
    VALID | {"map": {"123e4567-e89b-12d3-a456-426614174000": 1}},
    VALID | {"map": {"not-a-uuid": 1}},
    VALID | {"node": {"next": {"next": {}}}},
    VALID | {"node": {"next": 1}},
    VALID | {"pair": ["a", 1]},
    VALID | {"pair": ["a", 1, 2]},
    VALID | {"pair": [1]},
    VALID | {"x-provenance": True},
    VALID | {"x-provenance": True, "pair": []},
    VALID | {"x-provenance": "no"},
    VALID | {"a/b": 10},
    VALID | {"": ""},
    VALID | {"zz": 1},
    VALID | {"list": [1, 2, 3, 4]},
    VALID | {"m~n": "z"},
    {"": "s", "a/b": 3},
    {"a/b": 3, "m~n": "x"},
    {},
    5,
    None,
    [VALID],
]


def test_inline_provenance_does_not_change_verdicts() -> None:
    m, _ = with_records(SCHEMA)
    validator = Validator()
    plain = validator.compile(EMITTER.layout(m)[0])
    inline = validator.compile(EMITTER.layout(m, inline_provenance=True)[0])
    verdicts = [validator.is_valid(plain, instance) for instance in INSTANCES]
    assert [validator.is_valid(inline, instance) for instance in INSTANCES] == verdicts
    assert True in verdicts and False in verdicts  # not vacuous
    assert verdicts[:2] == [True, True] and verdicts[2] is False


def test_loader_ignores_inline_provenance() -> None:
    m, pointers = with_records(SCHEMA)
    inline_bytes = EMITTER.emit(m, inline_provenance=True)
    plain_bytes = EMITTER.emit(m)
    explanation_bytes = EMITTER.explanation(m, pointers)
    loader = SchemaLoader()
    loaded_inline = loader.load(inline_bytes)
    loaded_plain = loader.load(plain_bytes)
    assert loaded_inline == loaded_plain == m
    for loaded in (loaded_inline, loaded_plain):
        assert (loaded.ignored, loaded.unsupported, loaded.dialect_assumed) == ([], [], False)
        assert loaded.fully_supported
        assert loaded.provenance == {}
    # Kept as an x- keyword (2.10), but never emitted from the node.
    assert PROVENANCE_KEYWORD in loaded_inline.root.kw
    assert EMITTER.emit(loaded_inline) == plain_bytes

    # Attaching the loaded explanation file to the loaded model reproduces both outputs.
    explanation = load_explanation(explanation_bytes)
    _, loaded_pointers = EMITTER.layout(loaded_inline)
    loaded_inline.provenance = {cid: explanation.records[pointer] for cid, pointer in loaded_pointers.items()}
    assert EMITTER.emit(loaded_inline, inline_provenance=True) == inline_bytes
    assert EMITTER.explanation(loaded_inline, loaded_pointers) == explanation_bytes


# ---------------------------------------------------------------------------
# Malformed explanation files
# ---------------------------------------------------------------------------

RECORD_JSON = json.dumps(ProvenanceRecord.baseline("type", Support(1, 2)).to_json())
EMPTY_SUPPORT = '"support": {"consistent": 0, "ratio": 1.0, "total": 0}'


def _file(records: str) -> bytes:
    return ('{"format": 1, "records": %s}' % records).encode()


def _one(pointer: str, record: str) -> bytes:
    return _file(json.dumps({pointer: None}).replace("null", record))


@pytest.mark.parametrize(
    ("data", "check", "location", "match"),
    [
        pytest.param(b"{", "json", "", "Expecting", id="truncated"),
        pytest.param(b'{"format": 1, "records": {}} {}', "json", "", "Extra data", id="two-values"),
        pytest.param(b'{"format": NaN, "records": {}}', "json", "", "NaN", id="nan"),
        pytest.param(b"[]", "structure", "", "got an array", id="array-root"),
        pytest.param(b"true", "structure", "", "got a boolean", id="boolean-root"),
        pytest.param(b'{"records": {}}', "format", "", "missing member 'format'", id="no-format"),
        pytest.param(b'{"format": 2, "records": {}}', "format", "/format", "unsupported .* format 2", id="v2"),
        pytest.param(b'{"format": true, "records": {}}', "format", "/format", "got a boolean", id="format-bool"),
        pytest.param(b'{"format": "1", "records": {}}', "format", "/format", "got a string", id="format-str"),
        pytest.param(b'{"format": 1.5, "records": {}}', "format", "/format", "got a number", id="format-float"),
        pytest.param(b'{"format": 2}', "format", "/format", "unsupported", id="v2-checked-first"),
        pytest.param(b'{"format": 1}', "structure", "", "missing member 'records'", id="no-records"),
        pytest.param(b'{"format": 1, "records": {}, "x": 0}', "structure", "", "unknown member 'x'", id="extra"),
        pytest.param(b'{"format": 1, "records": []}', "structure", "/records", "got an array", id="records-array"),
        pytest.param(_one("type", RECORD_JSON), "structure", "/records/type", "not a JSON Pointer", id="no-slash"),
        pytest.param(_one("/a~2", RECORD_JSON), "structure", "/records/~1a~02", "not a JSON Pointer", id="bad-tilde"),
        pytest.param(_one("", RECORD_JSON), "structure", "/records/", "root schema object", id="root-key"),
        pytest.param(_one("/type", "[]"), "structure", "/records/~1type", "expected an object", id="record-array"),
        pytest.param(
            _one("/type", '{"confidence": 0.5, %s}' % EMPTY_SUPPORT),
            "structure",
            "/records/~1type",
            "missing member 'technique'",
            id="no-technique",
        ),
        pytest.param(
            _one("/type", '{"confidence": 2, %s, "technique": "t"}' % EMPTY_SUPPORT),
            "structure",
            "/records/~1type",
            r"in \[0, 1\]",
            id="confidence-out-of-range",
        ),
        pytest.param(
            _one("/type", '{"confidence": 0.5, "support": {"consistent": 1, "ratio": 0.25, "total": 2}, "technique": "t"}'),
            "structure",
            "/records/~1type/support/ratio",
            "does not equal",
            id="wrong-ratio",
        ),
    ],
)
def test_malformed_explanation_files_raise_located_errors(data: bytes, check: str, location: str, match: str) -> None:
    with pytest.raises(ExplanationLoadError, match=match) as info:
        load_explanation(data)
    error = info.value
    assert error.check == check and check in EXPLANATION_LOAD_CHECKS
    assert error.location == location
    if check == "json":
        assert error.line is not None and error.col is not None
        assert f"line {error.line}" in str(error)
    else:
        assert (error.line, error.col) == (None, None)
        assert (repr(location) if location else "the top level") in str(error)
    again = pickle.loads(pickle.dumps(error))
    assert (again.check, again.reason, again.location, str(again)) == (check, error.reason, location, str(error))
    assert isinstance(error, ValueError)


def test_parsed_sources_must_be_json_values() -> None:
    with pytest.raises(ExplanationLoadError, match="the explanation file is not a JSON value") as info:
        load_explanation({"format": 1, "records": {"/type": {"confidence": float("nan")}}})
    assert info.value.check == "json"
    with pytest.raises(TypeError, match="explanation file source"):
        load_explanation(bytearray(b"{}"))  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="explanation file source"):
        load_explanation(object())  # type: ignore[arg-type]
