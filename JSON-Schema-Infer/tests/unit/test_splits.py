"""Unit tests for the corpus manifest and Schema_Split file models (task 23.2; Requirements 21.2, 21.8, 30.4)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from jsinfer.bench.splits import CorpusEntry, CorpusManifest, SchemaSplit, SplitFormatError


def test_corpus_manifest_round_trips_and_resolves_paths(tmp_path: Path) -> None:
    manifest = CorpusManifest.of(
        [CorpusEntry("b", "fixture", "schemas/b.json"), CorpusEntry("a", "schemastore", "x/a.json", {"tests": ["t.json"]})],
        tmp_path,
    )
    assert manifest.ids() == ("a", "b") and manifest.schema_path("b") == tmp_path / "schemas" / "b.json"
    manifest.save(tmp_path / "manifest.json")
    again = CorpusManifest.load(tmp_path / "manifest.json")
    assert again.ids() == ("a", "b") and again.entries["a"].extra == {"tests": ["t.json"]}
    assert again.entries["a"].to_json() == {"tests": ["t.json"], "id": "a", "source": "schemastore", "path": "x/a.json"}
    with pytest.raises(KeyError):
        manifest.schema_path("zzz")


@pytest.mark.parametrize(
    ("document", "match"),
    [
        ("not json", "not valid UTF-8 JSON"),
        (json.dumps([]), "must be a JSON object"),
        (json.dumps({"format": 2, "schemas": []}), "format 2 is not supported"),
        (json.dumps({"format": 1}), "'schemas' array"),
        (json.dumps({"format": 1, "schemas": [{"id": "a", "source": "nope", "path": "a.json"}]}), "source must be"),
        (json.dumps({"format": 1, "schemas": [{"id": "a", "source": "fixture", "path": "../a.json"}]}), "relative path"),
        (json.dumps({"format": 1, "schemas": [{"id": "a", "source": "fixture", "path": "a"}] * 2}), "twice"),
    ],
)
def test_corpus_manifest_rejects_malformed_files(tmp_path: Path, document: str, match: str) -> None:
    (tmp_path / "m.json").write_text(document, encoding="utf-8")
    with pytest.raises(SplitFormatError, match=match):
        CorpusManifest.load(tmp_path / "m.json")
    with pytest.raises(SplitFormatError, match="cannot be read"):
        CorpusManifest.load(tmp_path / "missing.json")


def test_schema_split_round_trips_and_assigns_each_id_once(tmp_path: Path) -> None:
    split = SchemaSplit("s1", 7, ("b", "a"), ("c",), ("d",), {"groups": {"a": "g1"}, "excluded": []})
    assert split.train == ("a", "b") and split.split_of("c") == "val" and split.split_of("d") == "held_out"
    assert split.split_of("zzz") is None and split.ids("held_out") == ("d",)
    split.save(tmp_path / "split.json")
    again = SchemaSplit.load(tmp_path / "split.json")
    assert again == split and again.to_json()["groups"] == {"a": "g1"}
    assert list(again.to_json())[:4] == ["format", "id", "seed", "splits"]
    with pytest.raises(ValueError, match="more than one split"):
        SchemaSplit("s1", 0, ("a",), ("a",), ())
    with pytest.raises(ValueError, match="written by SchemaSplit"):
        SchemaSplit("s1", 0, ("a",), (), (), {"seed": 3})
    with pytest.raises(ValueError):
        split.ids("test")  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("document", "match"),
    [
        (json.dumps({"format": 1, "id": "s", "seed": 0}), "'splits' must be a JSON object"),
        (json.dumps({"format": 1, "id": "s", "seed": -1, "splits": {}}), "seed must be"),
        (json.dumps({"format": 1, "id": "", "seed": 0, "splits": {}}), "id must be"),
        (json.dumps({"format": 1, "id": "s", "seed": 0, "splits": {"train": [1]}}), "array of non-empty strings"),
    ],
)
def test_schema_split_rejects_malformed_files(tmp_path: Path, document: str, match: str) -> None:
    (tmp_path / "s.json").write_text(document, encoding="utf-8")
    with pytest.raises(SplitFormatError, match=match):
        SchemaSplit.load(tmp_path / "s.json")
