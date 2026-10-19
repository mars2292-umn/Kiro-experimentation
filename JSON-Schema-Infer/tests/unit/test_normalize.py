"""Unit tests for field-name normalization and normalized Path sets (task 21.1; Requirements 24.2, 25.1)."""

from __future__ import annotations

import pytest

from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, key_step, render_path
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.corpus.normalize import (
    join_tokens,
    normalize_name,
    normalize_path,
    normalized_paths,
    ref_target,
    sorted_normalized_paths,
    walk_paths,
)


@pytest.mark.parametrize(
    "name",
    ["emailAddress", "email_address", "email-address", "EMAIL_ADDRESS", "EmailAddress", "Email-Address", "email__address"],
)
def test_the_spellings_of_24_2_share_one_normalized_name(name: str) -> None:
    assert normalize_name(name) == ("email", "address")


@pytest.mark.parametrize(
    ("name", "tokens"),
    [
        ("", ()),
        ("_", ()),
        ("-_-", ()),
        ("a", ("a",)),
        ("A", ("a",)),
        ("a1B", ("a1", "b")),
        ("a1b2", ("a1b2",)),
        ("HTTPServer", ("httpserver",)),  # no lower/digit -> upper transition inside "HTTPS"
        ("userID", ("user", "id")),
        ("v2Beta", ("v2", "beta")),
        ("x__y-", ("x", "y")),
        ("ß", ("ss",)),  # casefold, not lower
        ("Straße", ("strasse",)),
        ("é", ("é",)),
        ("a b", ("a b",)),  # spaces are not separators
        ("1st", ("1st",)),
    ],
)
def test_normalization_examples(name: str, tokens: tuple[str, ...]) -> None:
    assert normalize_name(name) == tokens


def test_normalization_is_idempotent_on_its_rendering_and_on_each_token() -> None:
    for name in ["emailAddress", "HTTPServer", "a1B", "Straße", "x__y", "userID2Go"]:
        tokens = normalize_name(name)
        assert normalize_name(join_tokens(tokens)) == tokens
        assert normalize_name("-".join(tokens)) == tokens
        for token in tokens:
            assert normalize_name(token) == (token,)


def test_normalize_name_rejects_non_strings() -> None:
    with pytest.raises(TypeError):
        normalize_name(b"abc")  # type: ignore[arg-type]


def test_normalize_path_replaces_key_steps_only() -> None:
    path = (key_step("userName"), ELEM_STEP, key_step("Email-Address"), MAPVAL_STEP)
    assert normalize_path(path) == (key_step("user_name"), ELEM_STEP, key_step("email_address"), MAPVAL_STEP)


@pytest.mark.parametrize(
    ("ref", "pointer"),
    [
        ("#", ""),
        ("#/$defs/a", "/$defs/a"),
        ("#/$defs/a%20b", "/$defs/a b"),
        ("#anchor", None),
        ("other.json#/x", None),
        ("#/bad~2", None),
        (3, None),
    ],
)
def test_ref_target(ref: object, pointer: str | None) -> None:
    assert ref_target(ref) == pointer  # type: ignore[arg-type]


def _model(doc: object) -> SchemaModel:
    return SchemaModel(SchemaNode.from_json(doc))


def test_normalized_paths_step_like_the_inferrer_and_normalize_keys() -> None:
    model = _model(
        {
            "properties": {
                "userName": {"type": "string"},
                "tagList": {"items": {"type": "string"}},
                "pairs": {"prefixItems": [{"type": "integer"}]},
                "lookUp": {"additionalProperties": {"type": "number"}},
                "byPattern": {"patternProperties": {"^x": {"type": "null"}}},
                "closed": {"additionalProperties": False},
                "open": {"additionalProperties": True},
            },
            "not": {"properties": {"hidden": {}}},
            "propertyNames": {"pattern": "^a"},
            "contains": {"properties": {"alsoHidden": {}}},
        }
    )
    assert normalized_paths(model) == frozenset(
        {
            "$",
            "$.user_name",
            "$.tag_list",
            "$.tag_list[*]",
            "$.pairs",
            "$.pairs[*]",
            "$.look_up",
            "$.look_up{*}",
            "$.by_pattern",
            "$.by_pattern{*}",
            "$.closed",
            "$.open",
        }
    )


def test_branches_contribute_paths_at_their_owners_path() -> None:
    model = _model(
        {
            "anyOf": [{"properties": {"A": {}}}, {"properties": {"B": {}}}],
            "oneOf": [{"properties": {"C": {}}}],
            "allOf": [{"properties": {"D": {}}}],
            "if": {"properties": {"E": {}}},
            "then": {"properties": {"F": {}}},
            "else": {"properties": {"G": {}}},
        }
    )
    assert normalized_paths(model) == frozenset({"$", "$.a", "$.b", "$.c", "$.d", "$.e", "$.f", "$.g"})


def test_refs_expand_up_to_the_path_depth() -> None:
    model = _model({"$ref": "#/$defs/node", "$defs": {"node": {"properties": {"next": {"$ref": "#/$defs/node"}}}}})
    assert sorted(normalized_paths(model, ref_depth=0)) == ["$", "$.next"]
    assert sorted(normalized_paths(model, ref_depth=2)) == ["$", "$.next", "$.next.next", "$.next.next.next"]
    rendered = [render_path(p) for p in sorted_normalized_paths(model, ref_depth=1)]
    assert rendered == ["$", "$.next", "$.next.next"]


def test_step_less_reference_cycles_terminate() -> None:
    model = _model({"allOf": [{"$ref": "#"}], "properties": {"a": {"$ref": "#/$defs/b"}}, "$defs": {"b": {"$ref": "#/$defs/c"}, "c": {"$ref": "#/$defs/b", "properties": {"z": {}}}}})
    assert normalized_paths(model, ref_depth=1) == frozenset({"$", "$.a", "$.a.z"})


def test_walk_paths_yields_ref_targets_and_rejects_bad_depths() -> None:
    model = _model({"$ref": "#/$defs/x", "$defs": {"x": {"type": "string"}}})
    found = [(render_path(path), node.kw.get("type")) for path, node in walk_paths(model)]
    assert found == [("$", None), ("$", "string")]
    with pytest.raises(ValueError):
        list(walk_paths(model, ref_depth=-1))


def test_unresolvable_and_remote_refs_are_skipped() -> None:
    model = _model({"properties": {"a": {"$ref": "#/$defs/missing"}, "b": {"$ref": "https://example.com/x"}}})
    assert normalized_paths(model) == frozenset({"$", "$.a", "$.b"})
