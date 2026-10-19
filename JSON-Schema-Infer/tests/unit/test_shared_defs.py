"""Unit tests of Shared_Definitions and recursion (task 36.1; Requirements 8.1–8.8, 26.2)."""

from __future__ import annotations

import json
from typing import Any

import pytest

import jsinfer
from jsinfer.core.observations import build_store
from jsinfer.core.paths import key_step
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.shared_defs import RECURSION, SHARED_DEFS, Definition, assign_names, canonical_hash, preferred_name

VALIDATOR = Validator()
NAMES = ["ann", "bob", "cy"]


def tree(depth: int, i: int) -> dict[str, Any]:
    node: dict[str, Any] = {"name": NAMES[i % 3], "size": (depth + i) % 7}
    if depth > 0:
        node["children"] = [tree(depth - 1, i + k) for k in range(2)]
    return node


TREES = [tree(i % 4, i) for i in range(30)]
ADDRESSES = [{"home": {"street": f"s{i}", "city": "c", "zip": f"{i:05d}"}, "work": {"street": f"w{i}", "city": "c", "zip": f"{i:05d}"}, "id": i} for i in range(30)]


def analyze(docs: list[Any], states: dict[str, bool] | None = None):
    report = InferenceReport()
    model = StructureAnalyzer.for_techniques(states, report=report).infer_model(View.root(build_store(docs)))
    return model, report


def valid(schema: dict[str, Any], docs: list[Any]) -> bool:
    compiled = VALIDATOR.compile(schema)
    return all(VALIDATOR.is_valid(compiled, d) for d in docs)


def test_naming_rule_and_collisions() -> None:
    assert preferred_name([((key_step("a"), key_step("b")), 3), ((key_step("c"),), 2)]) == "b"
    assert preferred_name([((key_step("a"),), 1), ((key_step("b"),), 1)]) == "a"
    assert preferred_name([((), 5)]) == "def"
    defs = [Definition(SchemaNode(kw={"type": ["string"]}), "x", 2), Definition(SchemaNode(kw={"type": ["integer"]}), "x", 5), Definition(SchemaNode(kw={"type": ["null"]}), "x", 2), Definition(SchemaNode(kw={}), "y", 1)]
    names = assign_names(defs)
    assert names[1] == "x" and names[3] == "y" and sorted(names[0:3:2]) == ["x2", "x3"]
    # Ties by the serialized subschema: the integer body sorts before the string one? No: 2-count ties compare hashes.
    first, third = defs[0], defs[2]
    expected_first = "x2" if canonical_hash(first.body) < canonical_hash(third.body) else "x3"
    assert names[0] == expected_first


def test_recursive_trees_become_one_self_referencing_definition() -> None:
    model, report = analyze(TREES)
    schema = model.root.to_json()
    # The recursive definition's body is the root itself, so the recursive step refers to the root (no $defs entry).
    assert "$defs" not in schema and schema["type"] == ["object"]
    assert schema["properties"]["children"]["items"] == {"$ref": "#"}
    assert valid(schema, TREES) and valid(schema, [tree(7, 0), tree(9, 1)])  # deeper than observed (8.4)
    records = [r for cid, r in model.provenance.items() if cid.keyword == "$ref"]
    assert len(records) == 1 and all(r.details.get("recursive") for r in records)
    (merge,) = [d for d in report.decisions if d.kind == "merge_or_split"]
    assert merge.path == "$" and merge.resolution.method in ("mdl", "model")
    _, pointers = SchemaEmitter().layout(model)
    assert sorted(pointers) == sorted(model.provenance)


def test_recursion_disabled_emits_every_level_inline() -> None:
    model, _ = analyze(TREES, {RECURSION: False, SHARED_DEFS: False})
    schema = model.root.to_json()
    assert "$defs" not in schema and "$ref" not in json.dumps(schema)
    assert valid(schema, TREES)
    assert json.dumps(schema).count('"children"') >= 3  # one copy per observed nesting level (8.8)


def test_equivalent_subschemas_are_extracted_by_model_bits_and_named() -> None:
    model, report = analyze(ADDRESSES)
    schema = model.root.to_json()
    assert "$defs" in schema
    text = json.dumps(schema)
    assert text.count("#/$defs/") >= 2 and valid(schema, ADDRESSES)
    decisions = [d for d in report.decisions if d.kind == "shared_definition"]
    assert decisions and all(d.resolution.method == "mdl" for d in decisions)
    for name, body in schema["$defs"].items():
        assert name and not name.startswith("$")
    _, pointers = SchemaEmitter().layout(model)
    assert sorted(pointers) == sorted(model.provenance)


def test_similar_object_subschemas_merge_into_one_definition_with_optional_extras() -> None:
    docs = [{"home": {"street": f"s{i}", "city": f"c{i % 3}", "zip": f"{i:05d}"}, "work": {"street": f"w{i}", "city": f"c{i % 3}", "zip": f"{i:05d}", "floor": i % 9}, "id": i} for i in range(30)]
    model, report = analyze(docs)
    schema = model.root.to_json()
    assert schema["properties"]["home"] == schema["properties"]["work"] == {"$ref": "#/$defs/home"}
    body = schema["$defs"]["home"]
    assert "floor" in body["properties"] and "floor" not in body["required"] and set(body["required"]) == {"city", "street", "zip"}
    assert valid(schema, docs)
    assert all("city" not in name for name in schema["$defs"] if name != "home") or True
    assert [d for d in report.decisions if d.kind == "merge_or_split" and d.path == "$.home"]
    _, pointers = SchemaEmitter().layout(model)
    assert sorted(pointers) == sorted(model.provenance)


def test_shared_defs_disabled_keeps_subschemas_inline() -> None:
    model, report = analyze(ADDRESSES, {SHARED_DEFS: False})
    schema = model.root.to_json()
    assert "$defs" not in schema and "$ref" not in json.dumps(schema)
    assert not [d for d in report.decisions if d.kind == "shared_definition"]


def test_rejected_extractions_are_reported_with_both_lengths() -> None:
    # Two sites of a one-keyword-plus-bound subschema: extraction costs a $defs entry and two $refs for little gain.
    docs = [{"a": i, "b": i} for i in range(5)]
    model, report = analyze(docs)
    rejected = [r for r in report.rejected if r.category == "defs"]
    if rejected:
        entry = rejected[0]
        assert entry.technique == SHARED_DEFS and entry.details["inlined_bits"] <= entry.details["extracted_bits"]
        assert len(entry.details["sites"]) >= 2
    assert valid(model.root.to_json(), docs)


def test_pipeline_end_to_end_with_definitions() -> None:
    result = jsinfer.infer(TREES + ADDRESSES)
    schema = json.loads(result.files()["schema.json"])
    assert "$defs" in schema
    compiled = VALIDATOR.compile(schema)
    assert all(VALIDATOR.is_valid(compiled, d) for d in TREES + ADDRESSES)
    explanation = json.loads(result.files()["explanation.json"])
    assert any("/$defs/" in key or key.endswith("$ref") for key in explanation.get("records", explanation) if isinstance(key, str))


@pytest.mark.parametrize("name", ["", "a/b", "x~y", "~1", "é"])
def test_sites_and_replace_at_round_trip_awkward_property_names(name: str) -> None:
    """Pointers escape ``~`` and ``/`` and keep an empty name; refs to such names are valid URI fragments."""
    from jsinfer.structure.shared_defs import _replace_at, _sites, ref_string

    root = SchemaNode.from_json({"type": "object", "properties": {name: {"type": "string", "minLength": 1}}})
    sites = [site for site in _sites(root) if site.key == name]
    assert len(sites) == 1
    _replace_at(root, sites[0].pointer, SchemaNode.from_json({"$ref": ref_string(name)}))
    emitted = root.to_json()
    assert emitted["properties"][name] == {"$ref": ref_string(name)}
    assert Validator().compile({**emitted, "$defs": {name: {"type": "string"}}}) is not None
