"""Unit tests for the leakage aborts (task 25.5; Requirements 30.7, 30.8, 30.11)."""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.bench.leakage import LeakageError, LeakageReport, check_leakage, held_out_groups
from jsinfer.bench.splits import SchemaSplit

SPLIT = SchemaSplit(
    "s",
    0,
    ("a", "b", "c"),
    ("d",),
    ("h1", "h2"),
    {"groups": {"a": "a", "b": "a", "c": "c", "d": "d", "h1": "h1", "h2": "h2", "z": "h2"}},
)
CORPUS = ("a", "b", "c", "d", "h1", "h2", "z")


def manifest(train: list[str], val: list[str], collections: list[Any] = []) -> dict[str, Any]:
    return {"schemas": {"train": train, "val": val}, "unlabeled_collections": collections}


def test_held_out_groups_cover_every_member_of_a_held_out_group() -> None:
    assert held_out_groups(SPLIT) == {"h1": "h1", "h2": "h2", "z": "h2"}
    bare = SchemaSplit("s", 0, ("a",), ("b",), ("h",))
    assert held_out_groups(bare) == {"h": "h"}


def test_a_clean_manifest_passes_with_a_report() -> None:
    report = check_leakage(manifest(["a", "b"], ["d"], [{"id": "col-1"}, "col-2"]), SPLIT, CORPUS, real_collections=["real-1"])
    assert report == LeakageReport(("a", "b", "d"), ("col-1", "col-2"), ("h1", "h2"))
    assert report.to_json()["held_out_groups"] == ["h1", "h2"]


def test_a_missing_manifest_aborts() -> None:
    with pytest.raises(LeakageError, match="no training manifest") as info:
        check_leakage(None, SPLIT, CORPUS)
    assert info.value.missing_manifest is not None and info.value.unknown == ()
    with pytest.raises(LeakageError, match="no training schema lists"):
        check_leakage({"bundle_id": "x"}, SPLIT, CORPUS)
    with pytest.raises(LeakageError, match="no training schema lists"):
        check_leakage({"schemas": {"train": "a"}}, SPLIT, CORPUS)


def test_every_offender_is_listed() -> None:
    bad = manifest(["a", "h1", "nope", "z"], ["other", "d"], ["real-1", {"id": "col"}, "real-2"])
    with pytest.raises(LeakageError) as info:
        check_leakage(bad, SPLIT, CORPUS, real_collections=["real-2", "real-1", "unused"])
    error = info.value
    assert error.unknown == ("nope", "other")
    assert error.held_out == {"h1": "h1", "z": "h2"}  # z shares h2's near-duplicate group
    assert error.collections == ("real-1", "real-2")
    message = str(error)
    assert "absent from the corpus: ['nope', 'other']" in message
    assert "z (group h2)" in message and "h1 (group h1)" in message
    assert "Real_Dataset collections used for training: ['real-1', 'real-2']" in message


@pytest.mark.parametrize(
    ("train", "val", "collections", "field"),
    [
        (["a", "nope"], [], [], "unknown"),
        (["h2"], [], [], "held_out"),
        (["z"], [], [], "held_out"),
        ([], ["c"], ["real-1"], "collections"),
    ],
)
def test_each_offense_alone_aborts(train: list[str], val: list[str], collections: list[str], field: str) -> None:
    with pytest.raises(LeakageError) as info:
        check_leakage(manifest(train, val, collections), SPLIT, CORPUS, real_collections=["real-1"])
    assert getattr(info.value, field)
    for other in ("unknown", "held_out", "collections"):
        if other != field:
            assert not getattr(info.value, other)
