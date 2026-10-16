"""Unit tests for core/determinism.py (task 1.2)."""

from __future__ import annotations

import hashlib
import itertools
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import pytest

from jsinfer.core import determinism as det

# ---------------------------------------------------------------------------
# canonical_bytes and stable_hash32
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _Step:
    """Stand-in for a Path step that opts into canonical hashing."""

    kind: int
    key: str = ""

    def __canonical__(self) -> list[object]:
        return [self.kind, self.key]


def test_canonical_bytes_is_compact_sorted_json() -> None:
    assert det.canonical_bytes({"b": [1, "é"], "a": None}) == '{"a":null,"b":[1,"é"]}'.encode()


def test_canonical_bytes_ignores_key_order_and_integral_float_form() -> None:
    assert det.canonical_bytes({"x": 1.0, "y": [2e0]}) == det.canonical_bytes({"y": [2], "x": 1})
    assert det.canonical_bytes(-0.0) == det.canonical_bytes(0)
    assert det.canonical_bytes(1.5) != det.canonical_bytes(1)


def test_canonical_bytes_keeps_booleans_apart_from_numbers() -> None:
    assert det.canonical_bytes(True) != det.canonical_bytes(1)
    assert det.canonical_bytes(False) != det.canonical_bytes(0)
    assert det.canonical_bytes("1") != det.canonical_bytes(1)


def test_canonical_bytes_tuples_numpy_scalars_and_hook() -> None:
    assert det.canonical_bytes(("a", 1)) == det.canonical_bytes(["a", 1])
    assert det.canonical_bytes(np.int64(7)) == b"7"
    assert det.canonical_bytes(np.float64(2.0)) == b"2"
    assert det.canonical_bytes(np.bool_(True)) == b"true"
    assert det.canonical_bytes((_Step(0, "users"), _Step(1))) == b'[[0,"users"],[1,""]]'


def test_canonical_bytes_keeps_big_integers_exact() -> None:
    big = 2**80 + 1
    assert det.canonical_bytes(big) == str(big).encode()


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), -float("inf"), [1, float("nan")]])
def test_canonical_bytes_rejects_non_finite_numbers(bad: object) -> None:
    with pytest.raises(ValueError):
        det.canonical_bytes(bad)


@pytest.mark.parametrize("bad", [{1, 2}, object(), {1: "x"}, b"raw"])
def test_canonical_bytes_rejects_values_without_canonical_form(bad: object) -> None:
    with pytest.raises(TypeError):
        det.canonical_bytes(bad)


def test_stable_hash32_is_blake2b_of_canonical_bytes() -> None:
    expected = int.from_bytes(hashlib.blake2b(b'"variants.dbscan"', digest_size=4).digest(), "big")
    assert det.stable_hash32("variants.dbscan") == expected
    for value in ["", "a", 0, 2**40, ("$", "users"), {"k": [1, 2]}]:
        assert 0 <= det.stable_hash32(value) < 2**32


# ---------------------------------------------------------------------------
# rng streams
# ---------------------------------------------------------------------------


def _draw(gen: np.random.Generator) -> list[int]:
    return gen.integers(0, 2**62, size=8).tolist()


def test_rng_is_reproducible_and_uses_default_seed_zero() -> None:
    assert det.DEFAULT_SEED == 0
    first = _draw(det.rng("variants.dbscan", ("$", "users")))
    assert first == _draw(det.rng("variants.dbscan", ("$", "users")))
    assert first == _draw(det.rng("variants.dbscan", ("$", "users"), seed=0))


def test_rng_matches_documented_seed_sequence() -> None:
    keys = ("$", "users")
    expected = np.random.default_rng(
        np.random.SeedSequence(5, spawn_key=(det.stable_hash32("lof"), det.stable_hash32(keys)))
    )
    assert _draw(det.rng("lof", keys, seed=5)) == _draw(expected)


def test_rng_streams_differ_by_purpose_keys_key_order_and_seed() -> None:
    streams = [
        det.rng("a"),
        det.rng("b"),
        det.rng("a", "x"),
        det.rng("a", "y"),
        det.rng("a", "x", "y"),
        det.rng("a", "y", "x"),
        det.rng("a", seed=1),
    ]
    draws = [tuple(_draw(s)) for s in streams]
    assert len(set(draws)) == len(draws)


def test_rng_keys_follow_json_equality() -> None:
    # 1.0 and 1 are the same JSON value, so they key the same stream.
    assert _draw(det.rng("p", 1.0)) == _draw(det.rng("p", 1))
    assert _draw(det.rng("p", True)) != _draw(det.rng("p", 1))


@pytest.mark.parametrize("seed", [-1, 1.5, "0", True, None])
def test_rng_rejects_invalid_seeds(seed: object) -> None:
    with pytest.raises(ValueError):
        det.rng("p", seed=seed)  # type: ignore[arg-type]


@pytest.mark.parametrize("purpose", ["", 3, None])
def test_rng_rejects_invalid_purposes(purpose: object) -> None:
    with pytest.raises(ValueError):
        det.rng(purpose)  # type: ignore[arg-type]


def test_random_state_is_stable_32_bit_int() -> None:
    value = det.random_state("iforest", "$")
    assert value == det.random_state("iforest", "$")
    assert value != det.random_state("iforest", "$", seed=1)
    assert 0 <= value < 2**32


_SUBPROCESS_SNIPPET = """
import json
from jsinfer.core import determinism as det
gen = det.rng("variants.dbscan", ("$", "users"), {"b": 1, "a": 2}, seed=7)
print(json.dumps({
    "draws": gen.integers(0, 2**62, size=8).tolist(),
    "hash": det.stable_hash32(["x", 1.5, None]),
    "state": det.random_state("lof", "$.a"),
}))
"""


@pytest.mark.parametrize("hash_seed", ["0", "12345", "random"])
def test_rng_is_identical_across_processes_and_hash_seeds(hash_seed: str) -> None:
    env = {**os.environ, "PYTHONHASHSEED": hash_seed}
    out = subprocess.run(
        [sys.executable, "-c", _SUBPROCESS_SNIPPET],
        check=True,
        capture_output=True,
        text=True,
        env=env,
    ).stdout
    gen = det.rng("variants.dbscan", ("$", "users"), {"a": 2, "b": 1}, seed=7)
    expected = {
        "draws": _draw(gen),
        "hash": det.stable_hash32(["x", 1.5, None]),
        "state": det.random_state("lof", "$.a"),
    }
    assert json.loads(out) == expected


# ---------------------------------------------------------------------------
# Canonical ordering
# ---------------------------------------------------------------------------


def test_codepoint_order_is_not_utf16_or_locale_order() -> None:
    # U+FF5E sorts before U+1F600 by code point but after it by UTF-16 code unit.
    strings = ["\U0001f600", "\uff5e", "b", "B", "a", "é"]
    assert det.sorted_codepoint(strings) == ["B", "a", "b", "é", "\uff5e", "\U0001f600"]
    assert det.codepoint_key("x") == "x"


def test_sorted_items_orders_by_key() -> None:
    assert det.sorted_items({"b": 1, "a": 2, "B": 3}) == [("B", 3), ("a", 2), ("b", 1)]


def test_json_type_rank_matches_design_type_order() -> None:
    cases = [(None, 0), (False, 1), (3, 2), (3.0, 2), (1e3, 2), (2.5, 3), ("s", 4), ([], 5), ((), 5), ({}, 6)]
    for value, rank in cases:
        assert det.json_type_rank(value) == rank


def test_value_sort_key_keeps_true_and_one_distinct() -> None:
    assert det.value_sort_key(True) != det.value_sort_key(1)
    assert det.value_sort_key(False) != det.value_sort_key(0)
    assert det.value_sort_key(1.0) == det.value_sort_key(1)
    assert det.value_sort_key([1.0, {"b": 0, "a": 1}]) == det.value_sort_key([1, {"a": 1, "b": 0}])


def test_sorted_values_follows_canonical_value_order() -> None:
    # Containers compare by canonical bytes: b"[1,2]" < b"[1]" because "," < "]".
    expected = [None, False, True, -3, 1, 2, 0.5, 2.5, "", "a", "é", [1, 2], [1], {"a": 1}]
    values = [{"a": 1}, "é", 2.5, [1, 2], True, 1, None, "a", -3, False, 0.5, [1], "", 2]
    assert det.sorted_values(values) == expected


def test_sorted_values_is_independent_of_input_order() -> None:
    values = [None, True, 0, 1, 1.5, "x", [0], {"k": None}]
    results = {tuple(map(det.canonical_bytes, det.sorted_values(p))) for p in itertools.permutations(values)}
    assert len(results) == 1


def test_value_sort_key_rejects_non_json_values() -> None:
    with pytest.raises(ValueError):
        det.value_sort_key(float("nan"))
    with pytest.raises(TypeError):
        det.value_sort_key({1, 2})


# ---------------------------------------------------------------------------
# Canonical encoder
# ---------------------------------------------------------------------------


def test_encode_canonical_matches_stdlib_form() -> None:
    doc = {"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object", "enum": [2**70, "é"]}
    expected = json.dumps(doc, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    assert det.dumps_canonical(doc) == expected
    assert det.encode_canonical(doc) == expected.encode("utf-8")


def test_encode_canonical_emits_non_ascii_raw_with_trailing_newline() -> None:
    data = det.encode_canonical({"name": "café ☃"})
    assert data == '{\n  "name": "café ☃"\n}\n'.encode()
    assert b"\\u" not in data


def test_encode_canonical_preserves_insertion_order() -> None:
    assert det.dumps_canonical({"b": 1, "a": 2}) == '{\n  "b": 1,\n  "a": 2\n}\n'


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), {"x": -float("inf")}])
def test_encode_canonical_rejects_nan_and_infinity(bad: object) -> None:
    with pytest.raises(ValueError):
        det.encode_canonical(bad)


# ---------------------------------------------------------------------------
# Atomic writers
# ---------------------------------------------------------------------------


class _Boom(Exception):
    pass


def test_atomic_write_bytes_creates_and_replaces(tmp_path: Path) -> None:
    target = tmp_path / "schema.json"
    det.atomic_write_bytes(target, b"first")
    det.atomic_write_bytes(target, b"second")
    assert target.read_bytes() == b"second"
    assert sorted(os.listdir(tmp_path)) == ["schema.json"]


def test_atomic_write_text_is_utf8_without_newline_translation(tmp_path: Path) -> None:
    target = tmp_path / "config.toml"
    det.atomic_write_text(target, "a = \"é\"\r\nb = 1\n")
    assert target.read_bytes() == 'a = "é"\r\nb = 1\n'.encode()


def test_atomic_file_failure_leaves_existing_file_and_no_temp(tmp_path: Path) -> None:
    target = tmp_path / "report.json"
    target.write_bytes(b"old")
    with pytest.raises(_Boom), det.atomic_file(target) as handle:
        handle.write(b"partial")
        raise _Boom
    assert target.read_bytes() == b"old"
    assert os.listdir(tmp_path) == ["report.json"]


def test_atomic_file_failure_creates_nothing(tmp_path: Path) -> None:
    with pytest.raises(_Boom), det.atomic_file(tmp_path / "new.json", text=True) as handle:
        handle.write("partial")
        raise _Boom
    assert os.listdir(tmp_path) == []


def test_atomic_file_honours_umask_like_open(tmp_path: Path) -> None:
    det.atomic_write_bytes(tmp_path / "a", b"")
    (tmp_path / "b").write_bytes(b"")
    assert (tmp_path / "a").stat().st_mode == (tmp_path / "b").stat().st_mode


def test_atomic_directory_replaces_whole_directory(tmp_path: Path) -> None:
    bundle = tmp_path / "bundle"
    with det.atomic_directory(bundle) as staging:
        (staging / "manifest.json").write_text("v1")
        (staging / "stale.json").write_text("x")
    with det.atomic_directory(bundle) as staging:
        (staging / "manifest.json").write_text("v2")
    assert sorted(os.listdir(bundle)) == ["manifest.json"]
    assert (bundle / "manifest.json").read_text() == "v2"
    assert sorted(os.listdir(tmp_path)) == ["bundle"]


def test_atomic_directory_failure_keeps_old_directory(tmp_path: Path) -> None:
    bundle = tmp_path / "bundle"
    bundle.mkdir()
    (bundle / "manifest.json").write_text("v1")
    with pytest.raises(_Boom), det.atomic_directory(bundle) as staging:
        (staging / "manifest.json").write_text("v2")
        raise _Boom
    assert (bundle / "manifest.json").read_text() == "v1"
    assert sorted(os.listdir(tmp_path)) == ["bundle"]


def test_atomic_outputs_moves_files_and_replaces_same_names(tmp_path: Path) -> None:
    out = tmp_path / "out" / "run"
    out.mkdir(parents=True)
    (out / "schema.json").write_text("old")
    (out / "unrelated.txt").write_text("keep")
    (out / "versions").mkdir()
    (out / "versions" / "stale.json").write_text("old")
    with det.atomic_outputs(out) as staging:
        (staging / "schema.json").write_text("new")
        (staging / "report.json").write_text("r")
        (staging / "versions").mkdir()
        (staging / "versions" / "v1.json").write_text("v1")
    assert sorted(os.listdir(out)) == ["report.json", "schema.json", "unrelated.txt", "versions"]
    assert (out / "schema.json").read_text() == "new"
    assert (out / "unrelated.txt").read_text() == "keep"
    assert os.listdir(out / "versions") == ["v1.json"]


def test_atomic_outputs_creates_missing_out_dir(tmp_path: Path) -> None:
    out = tmp_path / "a" / "b"
    with det.atomic_outputs(out) as staging:
        (staging / "schema.json").write_text("s")
    assert os.listdir(out) == ["schema.json"]


def test_atomic_outputs_failure_leaves_no_new_outputs(tmp_path: Path) -> None:
    out = tmp_path / "out"
    out.mkdir()
    (out / "schema.json").write_text("previous")
    with pytest.raises(_Boom), det.atomic_outputs(out) as staging:
        (staging / "schema.json").write_text("partial")
        (staging / "explanation.json").write_text("partial")
        raise _Boom
    assert os.listdir(out) == ["schema.json"]
    assert (out / "schema.json").read_text() == "previous"
