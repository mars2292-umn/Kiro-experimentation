"""The corpus manifest and Schema_Split file models (tasks 23.2 and 25.2; Requirements 21.2, 21.8, 30.4).

Corpus manifest (:class:`CorpusManifest`)
    A JSON file ``{"format": 1, "schemas": [{"id", "source", "path", ...}]}``
    listing the Schema_Corpus: per schema its source identifier ``id``
    (unique), its ``source`` (``schemastore``, ``jsonschemabench``, or
    ``fixture``), and the ``path`` of its schema file relative to the
    manifest's directory. Task 25.1 writes the same model from the pinned
    checkouts, extended with test-file paths and catalog versions, which
    this reader keeps in :attr:`CorpusEntry.extra`. The Trainer reads the
    training-split and validation-split schemas by id from it (design, Open
    Decisions, Corpus manifest and training fixtures).

Schema_Split (:class:`SchemaSplit`)
    A JSON file ``{"format": 1, "id", "seed", "splits": {"train": [...],
    "val": [...], "held_out": [...]}, ...}`` assigning every schema id to one
    split. The optional members of 30.4 (``groups`` mapping ids to
    near-duplicate groups, ``excluded`` entries with reasons, ``cutoff``,
    ``proportions``, ``snapshots``) are kept as given; the split builder of
    task 25.2 writes them. Ids are listed in code-point order in each split
    and no id appears twice.

Both models load strict UTF-8 JSON and raise :class:`SplitFormatError` (a
``ValueError``) naming the file and the problem, and save canonical JSON
through the atomic writer.
"""

from __future__ import annotations

import json
import os
from collections import Counter
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

from jsinfer.core.determinism import atomic_write_bytes, codepoint_key, encode_canonical, sorted_codepoint
from jsinfer.core.jsonvalue import JSON

__all__ = [
    "CORPUS_FORMAT",
    "SOURCES",
    "SPLITS",
    "SPLIT_FORMAT",
    "CorpusEntry",
    "CorpusManifest",
    "SchemaSplit",
    "SplitFormatError",
    "SplitName",
]

CORPUS_FORMAT: Final = 1
SPLIT_FORMAT: Final = 1
SOURCES: Final[tuple[str, ...]] = ("schemastore", "jsonschemabench", "fixture")
SplitName: TypeAlias = Literal["train", "val", "held_out"]
SPLITS: Final[tuple[SplitName, ...]] = ("train", "val", "held_out")


class SplitFormatError(ValueError):
    """A corpus manifest or Schema_Split file that cannot be read; the message names the file and the problem."""


def _read_json(path: Path, what: str) -> Any:
    try:
        data = path.read_bytes()
    except OSError as error:
        raise SplitFormatError(f"{what} {os.fspath(path)!r} cannot be read: {error.strerror or type(error).__name__}") from None
    try:
        return json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SplitFormatError(f"{what} {os.fspath(path)!r} is not valid UTF-8 JSON: {error}") from None


def _object(value: Any, what: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SplitFormatError(f"{what} must be a JSON object")
    return value


def _format(document: dict[str, Any], expected: int, what: str) -> None:
    if document.get("format") != expected:
        raise SplitFormatError(f"{what} format {document.get('format')!r} is not supported; expected {expected}")


def _ids(value: Any, what: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise SplitFormatError(f"{what} must be an array of non-empty strings")
    return tuple(sorted_codepoint(value))


# ---------------------------------------------------------------------------
# Corpus manifest
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class CorpusEntry:
    """One schema of the Schema_Corpus: its id, source, schema file path (relative to the manifest), and extras."""

    id: str
    source: str
    path: str
    extra: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id:
            raise ValueError("a corpus entry id must be a non-empty str")
        if self.source not in SOURCES:
            raise ValueError(f"a corpus entry source must be one of {list(SOURCES)}, got {self.source!r}")
        if not isinstance(self.path, str) or not self.path or self.path.startswith("/") or ".." in self.path.split("/"):
            raise ValueError(f"a corpus entry path must be a relative path without '..', got {self.path!r}")

    def to_json(self) -> dict[str, JSON]:
        return {**dict(self.extra), "id": self.id, "source": self.source, "path": self.path}


@dataclass(frozen=True)
class CorpusManifest:
    """The Schema_Corpus listing: entries by id, and the directory paths are relative to."""

    entries: Mapping[str, CorpusEntry]
    root: Path

    @classmethod
    def of(cls, entries: Iterable[CorpusEntry], root: str | os.PathLike[str]) -> CorpusManifest:
        found: dict[str, CorpusEntry] = {}
        for entry in entries:
            if entry.id in found:
                raise ValueError(f"the corpus lists schema {entry.id!r} twice")
            found[entry.id] = entry
        ordered = {sid: found[sid] for sid in sorted_codepoint(found)}
        return cls(MappingProxyType(ordered), Path(root))

    @classmethod
    def load(cls, path: str | os.PathLike[str]) -> CorpusManifest:
        """Read a corpus manifest file; entries' paths resolve against the file's directory."""
        file = Path(path)
        document = _object(_read_json(file, "corpus manifest"), "the corpus manifest")
        _format(document, CORPUS_FORMAT, "corpus manifest")
        listed = document.get("schemas")
        if not isinstance(listed, list):
            raise SplitFormatError("the corpus manifest needs a 'schemas' array")
        entries: list[CorpusEntry] = []
        for index, item in enumerate(listed):
            obj = _object(item, f"schemas[{index}]")
            try:
                entries.append(
                    CorpusEntry(
                        obj.get("id"),  # type: ignore[arg-type]
                        obj.get("source"),  # type: ignore[arg-type]
                        obj.get("path"),  # type: ignore[arg-type]
                        {k: v for k, v in obj.items() if k not in ("id", "source", "path")},
                    )
                )
            except ValueError as error:
                raise SplitFormatError(f"schemas[{index}]: {error}") from None
        try:
            return cls.of(entries, file.parent)
        except ValueError as error:
            raise SplitFormatError(str(error)) from None

    def save(self, path: str | os.PathLike[str]) -> None:
        """Write the manifest as canonical JSON (paths stay as recorded)."""
        document = {"format": CORPUS_FORMAT, "schemas": [entry.to_json() for entry in self.entries.values()]}
        atomic_write_bytes(path, encode_canonical(document))

    def ids(self) -> tuple[str, ...]:
        return tuple(self.entries)

    def schema_path(self, schema_id: str) -> Path:
        """The schema file of ``schema_id``; ``KeyError`` for an unknown id."""
        return self.root / self.entries[schema_id].path


# ---------------------------------------------------------------------------
# Schema_Split
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SchemaSplit:
    """A Schema_Split: ids per split, its identifier and seed, and the other persisted members of 30.4."""

    id: str
    seed: int
    train: tuple[str, ...]
    val: tuple[str, ...]
    held_out: tuple[str, ...]
    extra: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id:
            raise ValueError("a Schema_Split id must be a non-empty str")
        if isinstance(self.seed, bool) or not isinstance(self.seed, int) or self.seed < 0:
            raise ValueError("a Schema_Split seed must be a non-negative int")
        for name in SPLITS:
            object.__setattr__(self, name, tuple(sorted_codepoint(getattr(self, name))))
        counts = Counter([*self.train, *self.val, *self.held_out])
        twice = sorted_codepoint([sid for sid, count in counts.items() if count > 1])
        if twice:
            raise ValueError(f"schema ids assigned to more than one split: {twice}")
        unknown = sorted(member for member in self.extra if member in ("format", "id", "seed", "splits"))
        if unknown:
            raise ValueError(f"extra members {unknown} are written by SchemaSplit itself")

    def split_of(self, schema_id: str) -> SplitName | None:
        """The split of ``schema_id``, or None when the split does not assign it."""
        for name in SPLITS:
            if schema_id in getattr(self, name):
                return name
        return None

    def ids(self, split: SplitName) -> tuple[str, ...]:
        if split not in SPLITS:
            raise ValueError(f"split must be one of {list(SPLITS)}, got {split!r}")
        return getattr(self, split)

    @classmethod
    def load(cls, path: str | os.PathLike[str]) -> SchemaSplit:
        file = Path(path)
        document = _object(_read_json(file, "Schema_Split"), "the Schema_Split")
        _format(document, SPLIT_FORMAT, "Schema_Split")
        splits = _object(document.get("splits"), "'splits'")
        try:
            return cls(
                document.get("id"),  # type: ignore[arg-type]
                document.get("seed"),  # type: ignore[arg-type]
                _ids(splits.get("train", []), "splits.train"),
                _ids(splits.get("val", []), "splits.val"),
                _ids(splits.get("held_out", []), "splits.held_out"),
                {k: v for k, v in document.items() if k not in ("format", "id", "seed", "splits")},
            )
        except ValueError as error:
            raise SplitFormatError(f"Schema_Split {os.fspath(file)!r}: {error}") from None

    def to_json(self) -> dict[str, JSON]:
        return {
            "format": SPLIT_FORMAT,
            "id": self.id,
            "seed": self.seed,
            "splits": {"train": list(self.train), "val": list(self.val), "held_out": list(self.held_out)},
            **{k: self.extra[k] for k in sorted(self.extra, key=codepoint_key)},
        }

    def save(self, path: str | os.PathLike[str]) -> None:
        atomic_write_bytes(path, encode_canonical(self.to_json()))
