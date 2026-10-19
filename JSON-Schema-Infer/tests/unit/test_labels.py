"""Unit tests for label derivation and Path alignment (task 21.5; Requirements 22.4, 22.5, 31.9, 31.10)."""

from __future__ import annotations

import pytest

from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, key_step, render_path
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.learn.labels import KINDS, UNKNOWN, SchemaPaths, align, derive_labels, judgment_accuracy, schema_paths


def _model(doc: object) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(doc))


def _rendered(labels: dict) -> dict[tuple[str, str], str]:
    return {(render_path(path), kind): label for (path, kind), label in labels.items()}


def test_kinds_are_the_five_judgment_call_kinds() -> None:
    assert KINDS == ("record_or_map", "closed_or_open_record", "tuple_or_list", "closed_or_open_enum", "merge_or_split")


def test_the_keyword_rules_of_22_4() -> None:
    labels = _rendered(
        derive_labels(
            _model(
                {
                    "type": "object",
                    "properties": {
                        "closed_enum": {"type": "string", "enum": ["x", "y"]},
                        "const": {"const": 3},
                        "open_enum": {"type": "integer"},
                        "bool_only": {"type": "boolean"},
                        "map": {"type": "object", "additionalProperties": {"type": "number"}},
                        "pattern_map": {"patternProperties": {"^x": {}}},
                        "true_map": {"type": "object", "additionalProperties": True},
                        "tuple": {"prefixItems": [{"type": "string"}], "items": {"type": "string"}},
                        "list": {"type": "array", "items": {"type": "string"}},
                        "bare_array": {"type": "array"},
                        "closed": {"properties": {"z": {}}, "additionalProperties": False},
                        "open": {"properties": {"z": {}}, "additionalProperties": {"type": "string"}},
                        "open_true": {"properties": {"z": {}}, "additionalProperties": True},
                        "split": {"anyOf": [{"properties": {"k": {}}}, {"properties": {"j": {}}}]},
                        "one_branch": {"oneOf": [{"properties": {"k": {}}}, {"type": "string"}]},
                    },
                }
            )
        )
    )
    assert labels[("$", "record_or_map")] == "record"
    assert labels[("$", "closed_or_open_record")] == UNKNOWN  # no additionalProperties
    assert labels[("$", "merge_or_split")] == "merge"
    assert labels[("$.closed_enum", "closed_or_open_enum")] == "closed"
    assert labels[("$.const", "closed_or_open_enum")] == "closed"
    assert labels[("$.open_enum", "closed_or_open_enum")] == "open"
    assert labels[("$.bool_only", "closed_or_open_enum")] == UNKNOWN
    assert labels[("$.map", "record_or_map")] == "map"
    assert labels[("$.pattern_map", "record_or_map")] == "map"
    assert labels[("$.true_map", "record_or_map")] == UNKNOWN  # true is not a schema-valued additionalProperties
    assert ("$.map", "closed_or_open_record") not in labels  # not a Record
    assert labels[("$.tuple", "tuple_or_list")] == "tuple"
    assert labels[("$.list", "tuple_or_list")] == "list"
    assert labels[("$.bare_array", "tuple_or_list")] == UNKNOWN
    assert labels[("$.closed", "closed_or_open_record")] == "closed"
    assert labels[("$.open", "closed_or_open_record")] == "open"
    assert labels[("$.open_true", "closed_or_open_record")] == "open"
    assert labels[("$.split", "merge_or_split")] == "split"
    assert labels[("$.split", "record_or_map")] == "record"
    assert labels[("$.one_branch", "merge_or_split")] == "merge"  # exactly one object subschema
    assert labels[("$.one_branch", "closed_or_open_enum")] == "open"
    # Kinds that do not apply have no entry.
    assert ("$.closed_enum", "record_or_map") not in labels
    assert ("$", "tuple_or_list") not in labels


def test_conflicting_branches_are_unknown() -> None:
    labels = _rendered(
        derive_labels(
            _model(
                {
                    "anyOf": [
                        {"prefixItems": [{}], "properties": {"a": {}}, "additionalProperties": False},
                        {"items": {}, "properties": {"a": {}}, "additionalProperties": True},
                    ]
                }
            )
        )
    )
    assert labels[("$", "tuple_or_list")] == UNKNOWN
    assert labels[("$", "closed_or_open_record")] == UNKNOWN
    assert labels[("$", "record_or_map")] == "record"  # both branches agree
    assert labels[("$", "merge_or_split")] == "split"


def test_ref_targets_count_once_and_bare_ref_branches_resolve() -> None:
    model = _model(
        {
            "$ref": "#/$defs/rec",
            "properties": {"u": {"oneOf": [{"$ref": "#/$defs/rec"}, {"$ref": "#/$defs/other"}]}},
            "$defs": {
                "rec": {"type": "object", "properties": {"x": {"type": "string"}}, "additionalProperties": False},
                "other": {"properties": {"y": {}}},
            },
        }
    )
    labels = _rendered(derive_labels(model))
    # The root's own keywords and the target are both object subschemas at "$": not exactly one, so unknown.
    assert labels[("$", "merge_or_split")] == UNKNOWN
    assert labels[("$", "record_or_map")] == "record"
    assert labels[("$", "closed_or_open_record")] == "closed"
    assert labels[("$.u", "merge_or_split")] == "split"
    assert labels[("$.u.x", "closed_or_open_enum")] == "open"


def test_recursion_is_capped_by_max_depth() -> None:
    model = _model({"$ref": "#/$defs/n", "$defs": {"n": {"properties": {"next": {"$ref": "#/$defs/n"}}}}})
    assert sorted(render_path(path) for path in schema_paths(model, max_depth=1)) == ["$", "$.next", "$.next.next"]
    # At the cap the node is a bare $ref whose target is not expanded, so no kind applies there.
    labeled = sorted({render_path(path) for (path, _) in derive_labels(model, max_depth=1)})
    assert labeled == ["$", "$.next"]
    assert sorted({render_path(path) for (path, _) in derive_labels(model, max_depth=2)}) == ["$", "$.next", "$.next.next"]


def test_boolean_schemas_and_non_models_are_handled() -> None:
    assert derive_labels(_model(True)) == {}
    assert derive_labels(_model(False)) == {}
    with pytest.raises(TypeError):
        derive_labels({"type": "object"})  # type: ignore[arg-type]


def test_judgment_accuracy_counts_known_labels_only_and_missing_paths_as_wrong() -> None:
    a, b = (key_step("a"),), (key_step("b"),)
    truth = {(a, "record_or_map"): "record", (b, "record_or_map"): "map", (a, "tuple_or_list"): UNKNOWN}
    assert judgment_accuracy(truth, truth) == {
        "record_or_map": 1.0,
        "closed_or_open_record": None,
        "tuple_or_list": None,
        "closed_or_open_enum": None,
        "merge_or_split": None,
    }
    inferred = {(a, "record_or_map"): "record", (a, "tuple_or_list"): "list"}
    assert judgment_accuracy(truth, inferred)["record_or_map"] == 0.5
    assert judgment_accuracy(truth, {})["record_or_map"] == 0.0


def test_align_follows_schema_steps_and_maps_keys_to_map_values() -> None:
    model = _model(
        {
            "properties": {
                "rec": {"properties": {"x": {}}},
                "map": {"additionalProperties": {"properties": {"inner": {}}}},
                "arr": {"items": {"properties": {"e": {}}}},
            }
        }
    )
    paths = SchemaPaths.of(model)
    assert paths.paths == schema_paths(model)
    assert align(model, ()) == ()
    assert align(paths, (key_step("rec"), key_step("x"))) == (key_step("rec"), key_step("x"))
    assert align(paths, (key_step("map"), key_step("anything"))) == (key_step("map"), MAPVAL_STEP)
    assert align(paths, (key_step("map"), key_step("k"), key_step("inner"))) == (key_step("map"), MAPVAL_STEP, key_step("inner"))
    assert align(paths, (key_step("map"), MAPVAL_STEP)) == (key_step("map"), MAPVAL_STEP)
    assert align(paths, (key_step("arr"), ELEM_STEP, key_step("e"))) == (key_step("arr"), ELEM_STEP, key_step("e"))
    assert align(paths, (key_step("rec"), key_step("missing"))) is None
    assert align(paths, (key_step("rec"), MAPVAL_STEP)) is None  # a Map where the schema has a Record
    assert align(paths, (ELEM_STEP,)) is None
    assert (key_step("rec"),) in paths
    with pytest.raises(TypeError):
        paths.align(("rec",))  # type: ignore[arg-type]
