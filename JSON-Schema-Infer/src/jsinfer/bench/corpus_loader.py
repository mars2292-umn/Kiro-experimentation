"""The Schema_Corpus loader (task 25.1; Requirements 30.1, 30.9).

:func:`load_schemastore` reads a SchemaStore checkout at a pinned commit:
schemas under ``src/schemas/json/<name>.json``, positive tests under
``src/test/<name>/``, negative tests under ``src/negative_test/<name>/``
(``.json`` and ``.toml`` test files are read, the latter through
``tomllib``; ``.yaml``/``.yml`` test files are omitted with a reason, since
no YAML reader is a dependency; other files are not test files),
and the catalog ``src/api/json/catalog.json``, whose entries list the
versions of one schema under ``versions`` (name → URL). Every schema gets the
id ``schemastore/<name>``, the source ``schemastore``, the ``snapshot``
(the commit), its linked ``tests`` (``positive`` and ``negative`` lists of
file paths relative to the manifest), and, for a file that a catalog entry
lists among its versions, the ``catalog_group`` (the entry's name), which
the split builder groups (30.2).

:func:`load_jsonschemabench` reads a JSONSchemaBench checkout at a pinned
revision: every ``*.json`` file under the configured directory, recursively,
with the id ``jsonschemabench/<relative path without the extension>`` and the
source ``jsonschemabench``.

A schema file that cannot be read or is not valid JSON, or whose root is not
an object or a boolean, is omitted with its reason, and so are its test
files; an unreadable or invalid test file is omitted on its own (30.9). The
omissions are recorded in the manifest's ``omitted`` member as
``{"id", "reason"}`` entries, where a test file's id is its path relative
to the checkout, and the snapshots in ``snapshots``. :func:`build_corpus`
assembles the manifest from the configured sources and
:func:`write_corpus_manifest` writes it where the Trainer and the split
builder read it (``bench.splits.CorpusManifest``).
"""

from __future__ import annotations

import json
import os
import tomllib
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Final
from urllib.parse import urlparse

from jsinfer.bench.splits import CorpusEntry, CorpusManifest
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON

__all__ = ["CorpusSource", "Omission", "build_corpus", "load_jsonschemabench", "load_schemastore", "write_corpus_manifest"]

SCHEMASTORE_SCHEMAS: Final = "src/schemas/json"
SCHEMASTORE_TESTS: Final = "src/test"
SCHEMASTORE_NEGATIVE_TESTS: Final = "src/negative_test"
SCHEMASTORE_CATALOG: Final = "src/api/json/catalog.json"


@dataclass(frozen=True)
class Omission:
    """A file left out of the corpus (30.9): its id (a schema id or a checkout-relative path) and the reason."""

    id: str
    reason: str

    def to_json(self) -> dict[str, JSON]:
        return {"id": self.id, "reason": self.reason}


@dataclass(frozen=True)
class CorpusSource:
    """What one loader produced: entries (paths relative to ``root``), omissions, and the snapshot."""

    source: str
    snapshot: str
    root: Path
    entries: list[CorpusEntry] = field(default_factory=list)
    omitted: list[Omission] = field(default_factory=list)


def _read_json(path: Path) -> tuple[Any, str | None]:
    try:
        data = path.read_bytes()
    except OSError as error:
        return None, f"cannot be read: {error.strerror or type(error).__name__}"
    try:
        return json.loads(data.decode("utf-8")), None
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return None, f"not valid JSON: {error}"


def _relative(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def _json_files(directory: Path) -> list[Path]:
    if not directory.is_dir():
        return []
    return sorted((p for p in directory.iterdir() if p.is_file() and p.suffix == ".json"), key=lambda p: codepoint_key(p.name))


_TEST_SUFFIXES: Final = (".json", ".toml", ".yaml", ".yml")


def _test_files(directory: Path) -> list[Path]:
    """The test documents of a SchemaStore test directory, in code-point order of their names."""
    if not directory.is_dir():
        return []
    return sorted((p for p in directory.iterdir() if p.is_file() and p.suffix in _TEST_SUFFIXES), key=lambda p: codepoint_key(p.name))


def _read_test(path: Path) -> tuple[Any, str | None]:
    """A test document as JSON, or None with the reason it is omitted (30.9)."""
    if path.suffix == ".json":
        return _read_json(path)
    if path.suffix == ".toml":
        try:
            return tomllib.loads(path.read_text(encoding="utf-8")), None
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
            return None, f"not valid TOML: {error}"
    return None, "YAML test files are not read (no YAML reader is a dependency)"


def _catalog_groups(root: Path, omitted: list[Omission]) -> dict[str, str]:
    """Schema file stem → catalog entry name, for every file a catalog entry lists among its versions."""
    catalog = root / SCHEMASTORE_CATALOG
    if not catalog.is_file():
        return {}
    document, reason = _read_json(catalog)
    if reason is not None:
        omitted.append(Omission(_relative(catalog, root), f"catalog {reason}; no catalog version groups"))
        return {}
    groups: dict[str, str] = {}
    entries = document.get("schemas", []) if isinstance(document, dict) else []
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("name"), str):
            continue
        urls = [entry.get("url")]
        versions = entry.get("versions")
        if isinstance(versions, dict):
            urls.extend(versions.values())
        stems = [Path(urlparse(url).path).stem for url in urls if isinstance(url, str) and url.endswith(".json")]
        if len(set(stems)) >= 2:
            for stem in stems:
                groups.setdefault(stem, entry["name"])
    return groups


def load_schemastore(root: str | os.PathLike[str], commit: str) -> CorpusSource:
    """Load a SchemaStore checkout at ``commit`` (see the module docstring)."""
    base = Path(root)
    if not isinstance(commit, str) or not commit:
        raise ValueError("the SchemaStore commit must be a non-empty str")
    schemas = base / SCHEMASTORE_SCHEMAS
    if not schemas.is_dir():
        raise FileNotFoundError(f"no SchemaStore schemas at {schemas}")
    source = CorpusSource("schemastore", commit, base)
    groups = _catalog_groups(base, source.omitted)
    for file in _json_files(schemas):
        stem = file.stem
        sid = f"schemastore/{stem}"
        document, reason = _read_json(file)
        if reason is None and not isinstance(document, (dict, bool)):
            reason = "the root is not a schema object or boolean"
        if reason is not None:
            source.omitted.append(Omission(sid, reason))
            for kind in (SCHEMASTORE_TESTS, SCHEMASTORE_NEGATIVE_TESTS):
                for test in _test_files(base / kind / stem):
                    source.omitted.append(Omission(_relative(test, base), f"its schema {sid} was omitted"))
            continue
        tests: dict[str, list[str]] = {"positive": [], "negative": []}
        for kind, directory in (("positive", SCHEMASTORE_TESTS), ("negative", SCHEMASTORE_NEGATIVE_TESTS)):
            for test in _test_files(base / directory / stem):
                _, problem = _read_test(test)
                if problem is not None:
                    source.omitted.append(Omission(_relative(test, base), problem))
                else:
                    tests[kind].append(_relative(test, base))
        extra: dict[str, JSON] = {"snapshot": commit, "tests": tests}
        if stem in groups:
            extra["catalog_group"] = groups[stem]
        source.entries.append(CorpusEntry(sid, "schemastore", _relative(file, base), extra))
    return source


def load_jsonschemabench(root: str | os.PathLike[str], revision: str) -> CorpusSource:
    """Load a JSONSchemaBench checkout at ``revision``: every ``*.json`` under ``root`` (see the module docstring)."""
    base = Path(root)
    if not isinstance(revision, str) or not revision:
        raise ValueError("the JSONSchemaBench revision must be a non-empty str")
    if not base.is_dir():
        raise FileNotFoundError(f"no JSONSchemaBench checkout at {base}")
    source = CorpusSource("jsonschemabench", revision, base)
    files = sorted((p for p in base.rglob("*.json") if p.is_file()), key=lambda p: codepoint_key(_relative(p, base)))
    for file in files:
        relative = _relative(file, base)
        sid = f"jsonschemabench/{relative[: -len('.json')]}"
        document, reason = _read_json(file)
        if reason is None and not isinstance(document, (dict, bool)):
            reason = "the root is not a schema object or boolean"
        if reason is not None:
            source.omitted.append(Omission(sid, reason))
            continue
        source.entries.append(CorpusEntry(sid, "jsonschemabench", relative, {"snapshot": revision}))
    return source


def build_corpus(sources: Iterable[CorpusSource], manifest_path: str | os.PathLike[str]) -> CorpusManifest:
    """The corpus manifest of ``sources`` for a manifest at ``manifest_path`` (entry paths made relative to its directory)."""
    directory = Path(manifest_path).resolve().parent
    entries: list[CorpusEntry] = []
    omitted: list[dict[str, JSON]] = []
    snapshots: dict[str, str] = {}
    for source in sources:
        snapshots[source.source] = source.snapshot
        root = source.root.resolve()
        for entry in source.entries:
            path = Path(os.path.relpath(root / entry.path, directory)).as_posix()
            extra = dict(entry.extra)
            tests = extra.get("tests")
            if isinstance(tests, dict):
                extra["tests"] = {
                    kind: [Path(os.path.relpath(root / p, directory)).as_posix() for p in files]  # type: ignore[union-attr]
                    for kind, files in tests.items()
                }
            entries.append(CorpusEntry(entry.id, entry.source, path, extra))
        omitted.extend(o.to_json() for o in source.omitted)
    omitted.sort(key=lambda o: codepoint_key(str(o["id"])))
    return CorpusManifest.of(entries, directory, {"omitted": omitted, "snapshots": snapshots})


def write_corpus_manifest(sources: Iterable[CorpusSource], manifest_path: str | os.PathLike[str]) -> CorpusManifest:
    """Build the manifest and write it to ``manifest_path`` (the directory is created)."""
    path = Path(manifest_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    manifest = build_corpus(sources, path)
    manifest.save(path)
    return manifest
