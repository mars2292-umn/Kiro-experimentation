"""Unit tests for core/paths.py (task 2.1)."""

from __future__ import annotations

import dataclasses
import pickle

import pytest

from jsinfer.core import determinism as det
from jsinfer.core.paths import (
    ELEM,
    ELEM_STEP,
    KEY,
    MAPVAL,
    MAPVAL_STEP,
    ROOT,
    Path,
    PathSyntaxError,
    Step,
    fewest_steps_key,
    key_step,
    parse_path,
    path_sort_key,
    render_path,
    render_step,
    sorted_paths,
)


def k(name: str) -> Step:
    return key_step(name)


E, M = ELEM_STEP, MAPVAL_STEP

# ---------------------------------------------------------------------------
# Step model
# ---------------------------------------------------------------------------


def test_step_kinds_and_constants() -> None:
    assert (KEY, ELEM, MAPVAL) == (0, 1, 2)
    assert k("a") == Step(KEY, "a")
    assert E == Step(ELEM) and E.key == ""
    assert M == Step(MAPVAL) and M.key == ""
    assert ROOT == ()


@pytest.mark.parametrize("kind", [3, -1, True, "0", 1.0, None])
def test_step_rejects_bad_kind(kind: object) -> None:
    with pytest.raises(ValueError, match="step kind"):
        Step(kind)  # type: ignore[arg-type]


def test_step_rejects_bad_keys() -> None:
    with pytest.raises(TypeError, match="must be str"):
        Step(KEY, 5)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="only Key steps"):
        Step(ELEM, "x")
    with pytest.raises(ValueError, match="only Key steps"):
        Step(MAPVAL, "x")
    with pytest.raises(ValueError, match="surrogate code point U\\+D800 at index 1"):
        k("a\ud800")


def test_step_is_frozen_hashable_and_pickles() -> None:
    step = k("users")
    with pytest.raises(dataclasses.FrozenInstanceError):
        step.key = "other"  # type: ignore[misc]
    path: Path = (k("users"), E, k("tags"), M)
    assert {path: 1}[(k("users"), E, k("tags"), M)] == 1
    assert pickle.loads(pickle.dumps(path)) == path


def test_paths_hash_by_content_in_canonical_bytes() -> None:
    assert det.canonical_bytes((k("users"), E, M)) == b'[[0,"users"],[1,""],[2,""]]'
    assert det.canonical_bytes(ROOT) == b"[]"
    # Equal Paths give equal streams; a Key named "" differs from Elem.
    a = det.rng("t", (k("a"), E)).integers(0, 2**32, 4).tolist()
    assert a == det.rng("t", (k("a"), E)).integers(0, 2**32, 4).tolist()
    assert det.canonical_bytes((k(""),)) != det.canonical_bytes((E,))


# ---------------------------------------------------------------------------
# Order
# ---------------------------------------------------------------------------


def test_step_order_key_elem_mapval() -> None:
    assert k("zzz") < E < M
    assert sorted([M, E, k("b"), k("a")]) == [k("a"), k("b"), E, M]


def test_keys_order_by_code_point() -> None:
    # U+FFFF precedes U+1F600 in code-point order (but not in UTF-16 order).
    names = ["\U0001f600", "\uffff", "é", "a", "Z", "_", ""]
    assert sorted(names, key=lambda n: k(n)) == ["", "Z", "_", "a", "é", "\uffff", "\U0001f600"]


def test_paths_sort_by_step_tuples() -> None:
    paths: list[Path] = [
        (M,),
        (k("b"),),
        (E, k("a")),
        (k("a"), M),
        (k("a"), E),
        (k("a"),),
        ROOT,
        (E,),
    ]
    expected: list[Path] = [ROOT, (k("a"),), (k("a"), E), (k("a"), M), (k("b"),), (E,), (E, k("a")), (M,)]
    assert sorted_paths(paths) == expected
    assert sorted(paths) == expected
    assert sorted(paths, key=path_sort_key) == expected


def test_sorted_paths_accepts_lists_and_returns_tuples() -> None:
    assert sorted_paths([[k("b")], [k("a"), E]]) == [(k("a"), E), (k("b"),)]


def test_fewest_steps_then_lexicographic() -> None:
    # Requirement 18.3: fewest steps first, ties by step order.
    candidates: list[Path] = [(k("a"), k("ts")), (E, k("ts")), (k("updated"),), (k("created"),)]
    ranked = sorted(candidates, key=fewest_steps_key)
    assert ranked == [(k("created"),), (k("updated"),), (k("a"), k("ts")), (E, k("ts"))]
    assert min(candidates, key=fewest_steps_key) == (k("created"),)


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------


def test_render_examples() -> None:
    assert render_path(ROOT) == "$"
    assert render_path((k("users"), E, k("tags"), M)) == "$.users[*].tags{*}"
    assert render_path((E, E, M, M)) == "$[*][*]{*}{*}"
    assert render_path((k("_id"), k("a1"), k("A_b2"))) == "$._id.a1.A_b2"
    assert str(k("x")) == ".x" and str(E) == "[*]" and str(M) == "{*}"
    assert render_step(k("a b")) == '["a b"]'


@pytest.mark.parametrize(
    ("name", "rendered"),
    [
        ("", '[""]'),
        ("1a", '["1a"]'),
        ("first-name", '["first-name"]'),
        ("a.b", '["a.b"]'),
        ("$ref", '["$ref"]'),
        ("*", '["*"]'),
        ("café", '["café"]'),
        ("a/b", '["a/b"]'),
        ('say "hi"', '["say \\"hi\\""]'),
        ("back\\slash", '["back\\\\slash"]'),
        ("\b\f\n\r\t", '["\\b\\f\\n\\r\\t"]'),
        ("\x00\x1f\x7f", '["\\u0000\\u001f\x7f"]'),
        ("\U0001f600", '["\U0001f600"]'),
    ],
)
def test_render_bracket_form_for_non_identifiers(name: str, rendered: str) -> None:
    assert render_step(k(name)) == rendered
    assert parse_path("$" + rendered) == (k(name),)


def test_render_rejects_non_steps() -> None:
    with pytest.raises(TypeError, match="expected a Step"):
        render_path(("users",))  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Parsing and round trip
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "text",
    [
        "$",
        "$.users[*].tags{*}",
        "$[*]",
        "${*}",
        '$[""]',
        '$.a["b.c"][*]["1"]{*}._',
        '$["a\\"b\\\\c\\n\\u0001"]',
        '$["é"].x["\U0001f600"]',
    ],
)
def test_canonical_strings_round_trip(text: str) -> None:
    assert render_path(parse_path(text)) == text


@pytest.mark.parametrize(
    "path",
    [
        ROOT,
        (k("users"), E, k("tags"), M),
        (k(""), k(" "), k("\u2028"), k("\x00")),
        (M, E, k("[*]"), k("{*}"), k('"]'), k("$")),
    ],
)
def test_paths_round_trip(path: Path) -> None:
    assert parse_path(render_path(path)) == path


def test_parse_examples() -> None:
    assert parse_path("$") == ROOT
    assert parse_path("$.users[*].tags{*}") == (k("users"), E, k("tags"), M)
    assert parse_path("$.created_at") == (k("created_at"),)


@pytest.mark.parametrize(
    ("text", "path"),
    [
        ('$["users"]', (k("users"),)),
        ('$["\\u0041"]', (k("A"),)),
        ('$["a\\/b"]', (k("a/b"),)),
        ('$["\\ud83d\\ude00"]', (k("\U0001f600"),)),
        ('$["\\u00E9"]', (k("é"),)),
    ],
)
def test_parse_accepts_non_canonical_spellings(text: str, path: Path) -> None:
    assert parse_path(text) == path


@pytest.mark.parametrize(
    ("text", "position"),
    [
        ("", 0),
        ("users", 0),
        (" $", 0),
        ("$ ", 1),
        ("$.", 2),
        ("$..a", 2),
        ("$.1a", 2),
        ("$.café", 5),
        ("$.a-b", 3),
        ("$.a.", 4),
        ("$[", 1),
        ("$[*", 1),
        ("$[0]", 1),
        ("$['a']", 1),
        ("${}", 1),
        ("$*", 1),
        ("$$", 1),
        ('$["a"', 5),
        ('$["a]', 2),
        ('$["a"x]', 5),
        ('$["\\x"]', 3),
        ('$["\\u12"]', 4),
        ('$["a\nb"]', 4),
        ('$["\\ud800"]', 2),
        ('$["\ud800"]', 2),
    ],
)
def test_parse_rejects_malformed_input(text: str, position: int) -> None:
    with pytest.raises(PathSyntaxError) as info:
        parse_path(text)
    err = info.value
    assert (err.text, err.position) == (text, position)
    assert str(err).startswith(f"invalid Path {text!r} at position {position}: ")


def test_parse_error_messages_and_pickling() -> None:
    with pytest.raises(PathSyntaxError, match="starts with '\\$'"):
        parse_path("users")
    for text in ("$.first-name.x", "$.café", "$.1a"):
        with pytest.raises(PathSyntaxError, match='write other keys as \\["key"\\]'):
            parse_path(text)
    with pytest.raises(PathSyntaxError) as info:
        parse_path("$[*]-")
    assert "write other keys" not in str(info.value)
    with pytest.raises(PathSyntaxError, match="unpaired surrogate"):
        parse_path('$["\\udc00"]')
    with pytest.raises(PathSyntaxError, match="Unterminated string"):
        parse_path('$["a')
    err = PathSyntaxError("$.", 2, "reason")
    assert isinstance(err, ValueError)
    clone = pickle.loads(pickle.dumps(err))
    assert (clone.text, clone.position, clone.reason, str(clone)) == ("$.", 2, "reason", str(err))


def test_parse_rejects_non_str() -> None:
    with pytest.raises(TypeError, match="must be str"):
        parse_path(b"$")  # type: ignore[arg-type]
