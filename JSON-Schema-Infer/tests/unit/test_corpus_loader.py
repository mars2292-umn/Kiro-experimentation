"""Unit tests for the corpus loader (task 25.1; Requirements 30.1, 30.9) on fake checkouts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from jsinfer.bench.corpus_loader import (
    SCHEMASTORE_CATALOG,
    SCHEMASTORE_NEGATIVE_TESTS,
    SCHEMASTORE_SCHEMAS,
    SCHEMASTORE_TESTS,
    build_corpus,
    load_jsonschemabench,
    load_schemastore,
    write_corpus_manifest,
)
from jsinfer.bench.splits import CorpusManifest


def write(path: Path, content: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content if isinstance(content, str) else json.dumps(content), encoding="utf-8")


@pytest.fixture
def schemastore(tmp_path: Path) -> Path:
    root = tmp_path / "schemastore"
    schemas = root / SCHEMASTORE_SCHEMAS
    write(schemas / "package.json", {"type": "object", "properties": {"name": {"type": "string"}}})
    write(schemas / "package-1.0.json", {"type": "object", "properties": {"name": {"type": "string"}, "v": {"const": 1}}})
    write(schemas / "tsconfig.json", {"type": "object"})
    write(schemas / "broken.json", "{not json")
    write(schemas / "scalar.json", 42)
    write(root / SCHEMASTORE_TESTS / "package" / "good.json", {"name": "a"})
    write(root / SCHEMASTORE_TESTS / "package" / "bad-file.json", "[")
    write(root / SCHEMASTORE_TESTS / "package" / "notes.txt", "ignored: not a test document")
    write(root / SCHEMASTORE_TESTS / "package" / "config.toml", 'name = "a"\n')
    write(root / SCHEMASTORE_TESTS / "package" / "config.yaml", "name: a\n")
    write(root / SCHEMASTORE_NEGATIVE_TESTS / "package" / "wrong.json", {"name": 1})
    write(root / SCHEMASTORE_TESTS / "broken" / "orphan.json", {})
    write(
        root / SCHEMASTORE_CATALOG,
        {
            "schemas": [
                {
                    "name": "package.json",
                    "url": "https://json.schemastore.org/package.json",
                    "versions": {"1.0": "https://json.schemastore.org/package-1.0.json"},
                },
                {"name": "tsconfig", "url": "https://json.schemastore.org/tsconfig.json"},
                {"name": "no url"},
            ]
        },
    )
    return root


def test_schemastore_schemas_tests_catalog_groups_and_omissions(schemastore: Path) -> None:
    source = load_schemastore(schemastore, "abc123")
    assert source.source == "schemastore" and source.snapshot == "abc123"
    by_id = {entry.id: entry for entry in source.entries}
    assert list(by_id) == ["schemastore/package-1.0", "schemastore/package", "schemastore/tsconfig"]  # code-point order
    package = by_id["schemastore/package"]
    assert package.path == f"{SCHEMASTORE_SCHEMAS}/package.json" and package.source == "schemastore"
    assert package.extra["snapshot"] == "abc123" and package.extra["catalog_group"] == "package.json"
    assert package.extra["tests"] == {
        "positive": [f"{SCHEMASTORE_TESTS}/package/config.toml", f"{SCHEMASTORE_TESTS}/package/good.json"],
        "negative": [f"{SCHEMASTORE_NEGATIVE_TESTS}/package/wrong.json"],
    }
    assert by_id["schemastore/package-1.0"].extra["catalog_group"] == "package.json"
    tsconfig = by_id["schemastore/tsconfig"]
    assert "catalog_group" not in tsconfig.extra and tsconfig.extra["tests"] == {"positive": [], "negative": []}
    omitted = {o.id: o.reason for o in source.omitted}
    assert "not valid JSON" in omitted["schemastore/broken"]
    assert omitted["schemastore/scalar"] == "the root is not a schema object or boolean"
    assert omitted[f"{SCHEMASTORE_TESTS}/broken/orphan.json"] == "its schema schemastore/broken was omitted"
    assert "not valid JSON" in omitted[f"{SCHEMASTORE_TESTS}/package/bad-file.json"]
    assert omitted[f"{SCHEMASTORE_TESTS}/package/config.yaml"] == "YAML test files are not read (no YAML reader is a dependency)"
    assert len(omitted) == 5


def test_schemastore_without_schemas_or_commit_is_an_error(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        load_schemastore(tmp_path, "abc")
    with pytest.raises(ValueError):
        load_schemastore(tmp_path, "")


def test_schemastore_without_a_catalog_or_with_a_broken_one(schemastore: Path) -> None:
    catalog = schemastore / SCHEMASTORE_CATALOG
    catalog.write_text("{", encoding="utf-8")
    source = load_schemastore(schemastore, "c")
    assert all("catalog_group" not in e.extra for e in source.entries)
    assert any(o.id == SCHEMASTORE_CATALOG and "catalog" in o.reason for o in source.omitted)
    catalog.unlink()
    assert all("catalog_group" not in e.extra for e in load_schemastore(schemastore, "c").entries)


def test_jsonschemabench_reads_every_json_file_recursively(tmp_path: Path) -> None:
    root = tmp_path / "jsb"
    write(root / "Github_easy" / "a.json", {"type": "string"})
    write(root / "Kaggle" / "deep" / "b.json", True)
    write(root / "Kaggle" / "c.json", [1])
    write(root / "readme.md", "x")
    source = load_jsonschemabench(root, "rev-1")
    assert [e.id for e in source.entries] == ["jsonschemabench/Github_easy/a", "jsonschemabench/Kaggle/deep/b"]
    assert source.entries[1].path == "Kaggle/deep/b.json" and source.entries[1].extra == {"snapshot": "rev-1"}
    assert [(o.id, o.reason) for o in source.omitted] == [("jsonschemabench/Kaggle/c", "the root is not a schema object or boolean")]
    with pytest.raises(FileNotFoundError):
        load_jsonschemabench(tmp_path / "missing", "r")


def test_the_manifest_is_written_with_paths_relative_to_it(schemastore: Path, tmp_path: Path) -> None:
    jsb = tmp_path / "jsb"
    write(jsb / "x.json", {"type": "integer"})
    manifest_path = tmp_path / "corpus" / "manifest.json"
    manifest = write_corpus_manifest([load_schemastore(schemastore, "abc"), load_jsonschemabench(jsb, "r")], manifest_path)
    loaded = CorpusManifest.load(manifest_path)
    assert loaded.ids() == manifest.ids() == ("jsonschemabench/x", "schemastore/package", "schemastore/package-1.0", "schemastore/tsconfig")
    assert loaded.schema_path("schemastore/package").resolve() == (schemastore / SCHEMASTORE_SCHEMAS / "package.json").resolve()
    assert loaded.schema_path("jsonschemabench/x").resolve() == (jsb / "x.json").resolve()
    tests = loaded.entries["schemastore/package"].extra["tests"]
    assert (manifest_path.parent / tests["positive"][1]).resolve() == (schemastore / SCHEMASTORE_TESTS / "package" / "good.json").resolve()
    assert loaded.extra["snapshots"] == {"jsonschemabench": "r", "schemastore": "abc"}
    assert [o["id"] for o in loaded.extra["omitted"]] == sorted(o["id"] for o in loaded.extra["omitted"])
    assert len(loaded.extra["omitted"]) == 5
    assert build_corpus([], manifest_path).ids() == ()


def test_jsonschemabench_jsonl_exports_are_read_row_by_row(tmp_path: Path) -> None:
    root = tmp_path / "jsb"
    root.mkdir()
    rows = [
        {"unique_id": "o1", "json_schema": json.dumps({"type": "object"}), "split": "test"},
        {"unique_id": "o1", "json_schema": json.dumps({"type": "string"}), "split": "train"},  # a repeated id
        {"json_schema": "{broken"},
        {"json_schema": json.dumps([1])},
        {"other": 1},
        "",
    ]
    (root / "Github_easy.jsonl").write_text("\n".join(r if isinstance(r, str) else json.dumps(r) for r in rows) + "\nnot json\n", encoding="utf-8")
    source = load_jsonschemabench(root, "sha-1")
    ids = [e.id for e in source.entries]
    assert ids == ["jsonschemabench/Github_easy/o1", "jsonschemabench/Github_easy/o1~2"]
    assert source.entries[0].extra["split"] == "test" and source.entries[0].extra["row"] == 1
    reasons = {o.id: o.reason for o in source.omitted}
    assert "json_schema is not valid JSON" in reasons["Github_easy.jsonl#3"]
    assert reasons["jsonschemabench/Github_easy/4"] == "the root is not a schema object or boolean"
    assert reasons["Github_easy.jsonl#5"] == "no json_schema member"
    assert "not valid JSON" in reasons["Github_easy.jsonl#7"]
    manifest_path = tmp_path / "corpus" / "manifest.json"
    manifest = write_corpus_manifest([source], manifest_path)
    loaded = CorpusManifest.load(manifest_path)
    assert loaded.ids() == manifest.ids() == ("jsonschemabench/Github_easy/o1", "jsonschemabench/Github_easy/o1~2")
    assert json.loads(loaded.schema_path("jsonschemabench/Github_easy/o1~2").read_text()) == {"type": "string"}
    assert "schema_text" not in loaded.entries["jsonschemabench/Github_easy/o1"].extra
