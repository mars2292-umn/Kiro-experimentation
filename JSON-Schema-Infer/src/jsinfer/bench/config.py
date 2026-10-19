"""The benchmark configuration ``bench.toml`` (tasks 25.1, 25.2; design, ``jsinfer split -c bench.toml``).

Sections and settings, every path relative to the file's directory:

``[corpus]``
    ``manifest``: the corpus manifest the loader writes and the split builder
    and Trainer read (required). ``schemastore`` and ``schemastore_commit``:
    a SchemaStore checkout and its pinned commit; ``jsonschemabench`` and
    ``jsonschemabench_revision``: a JSONSchemaBench checkout and its pinned
    revision. With no checkout configured, ``jsinfer split`` reads an
    existing manifest (the fixture corpus, for example) instead of writing one.
``[split]``
    ``out``: the Schema_Split file to write (required); ``id``: its
    identifier (default ``split-<seed>``); ``seed`` (default 0);
    ``known_schemas``: corpus ids of the Real_Datasets' known schemas, held
    out with their groups (30.3); ``known_schema_files``: a table of dataset
    id → schema file for known schemas outside the corpus, whose similar
    corpus schemas are held out.
``[resources]``
    Resource_Parameter overrides: the split proportions ``split.train``,
    ``split.val``, ``split.held_out`` (30.2, 30.10), ``minhash.perm``,
    ``lsh.bands``, and ``nearest.ref_depth``.

Other sections (``[datasets]``, ``[bench]``, ``[baselines]``) belong to later
tasks and are kept as given in ``extra``. Invalid settings raise
:class:`~jsinfer.config.errors.ConfigError`.
"""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Final

from jsinfer.config.errors import ConfigError, ConfigProblem
from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.config.run_config import read_config_file
from jsinfer.core.determinism import codepoint_key

__all__ = ["BENCH_RESOURCES", "BenchConfig"]

BENCH_RESOURCES: Final[tuple[str, ...]] = (
    "split.train",
    "split.val",
    "split.held_out",
    "minhash.perm",
    "lsh.bands",
    "nearest.ref_depth",
)
_CORPUS_KEYS: Final = ("manifest", "schemastore", "schemastore_commit", "jsonschemabench", "jsonschemabench_revision")
_SPLIT_KEYS: Final = ("out", "id", "seed", "known_schemas", "known_schema_files")
_KNOWN_SECTIONS: Final = ("corpus", "split", "resources")
_RESERVED_SECTIONS: Final = ("datasets", "bench", "baselines")


def _error(subject: str, message: str, reason: str = "type") -> ConfigError:
    return ConfigError([ConfigProblem(subject, reason, f"bench configuration: {message}")])  # type: ignore[arg-type]


def _text(table: Mapping[str, Any], key: str, section: str, *, required: bool = False) -> str | None:
    value = table.get(key)
    if value is None:
        if required:
            raise _error(f"{section}.{key}", f"[{section}] needs {key}")
        return None
    if not isinstance(value, str) or not value:
        raise _error(f"{section}.{key}", f"[{section}] {key} must be a non-empty string")
    return value


@dataclass(frozen=True)
class BenchConfig:
    """A parsed ``bench.toml`` (see the module docstring); paths are absolute."""

    manifest: str
    split_out: str
    split_id: str
    seed: int = 0
    schemastore: str | None = None
    schemastore_commit: str | None = None
    jsonschemabench: str | None = None
    jsonschemabench_revision: str | None = None
    known_schemas: tuple[str, ...] = ()
    known_schema_files: Mapping[str, str] = field(default_factory=dict)
    resources: Mapping[str, int | float] = field(default_factory=dict)
    extra: Mapping[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if isinstance(self.seed, bool) or not isinstance(self.seed, int) or self.seed < 0:
            raise _error("split.seed", f"[split] seed must be a non-negative integer, got {self.seed!r}")
        if (self.schemastore is None) != (self.schemastore_commit is None):
            raise _error("corpus.schemastore", "[corpus] schemastore and schemastore_commit go together")
        if (self.jsonschemabench is None) != (self.jsonschemabench_revision is None):
            raise _error("corpus.jsonschemabench", "[corpus] jsonschemabench and jsonschemabench_revision go together")
        for name, value in self.resources.items():
            if name not in BENCH_RESOURCES:
                raise _error(name, f"{name!r} is not a benchmark Resource_Parameter ({', '.join(BENCH_RESOURCES)})", "unknown")
            problem = RESOURCE_PARAMETERS[name].check(value)
            if problem is not None:
                raise _error(name, f"{name}: {problem}", "range")

    @classmethod
    def from_mapping(cls, document: Mapping[str, Any], base: str | os.PathLike[str] | None = None) -> BenchConfig:
        for section in document:
            if section not in _KNOWN_SECTIONS and section not in _RESERVED_SECTIONS:
                raise _error(section, f"unknown section [{section}]", "unknown")
        corpus = document.get("corpus", {})
        split = document.get("split", {})
        resources = document.get("resources", {})
        for name, table in (("corpus", corpus), ("split", split), ("resources", resources)):
            if not isinstance(table, Mapping):
                raise _error(name, f"[{name}] must be a table")
        for key in corpus:
            if key not in _CORPUS_KEYS:
                raise _error(f"corpus.{key}", f"unknown [corpus] setting {key!r}", "unknown")
        for key in split:
            if key not in _SPLIT_KEYS:
                raise _error(f"split.{key}", f"unknown [split] setting {key!r}", "unknown")

        def path(value: str | None) -> str | None:
            if value is None:
                return None
            if base is not None and not os.path.isabs(value):
                return os.path.join(os.fspath(base), value)
            return value

        known = split.get("known_schemas", [])
        if not isinstance(known, list) or not all(isinstance(sid, str) and sid for sid in known):
            raise _error("split.known_schemas", "[split] known_schemas must be a list of schema ids")
        files = split.get("known_schema_files", {})
        if not isinstance(files, Mapping) or not all(isinstance(v, str) and v for v in files.values()):
            raise _error("split.known_schema_files", "[split] known_schema_files must map dataset ids to file paths")
        seed = split.get("seed", 0)
        return cls(
            manifest=path(_text(corpus, "manifest", "corpus", required=True)),  # type: ignore[arg-type]
            split_out=path(_text(split, "out", "split", required=True)),  # type: ignore[arg-type]
            split_id=_text(split, "id", "split") or f"split-{seed}",
            seed=seed,
            schemastore=path(_text(corpus, "schemastore", "corpus")),
            schemastore_commit=_text(corpus, "schemastore_commit", "corpus"),
            jsonschemabench=path(_text(corpus, "jsonschemabench", "corpus")),
            jsonschemabench_revision=_text(corpus, "jsonschemabench_revision", "corpus"),
            known_schemas=tuple(sorted(set(known), key=codepoint_key)),
            known_schema_files={name: path(file) for name, file in sorted(files.items(), key=lambda i: codepoint_key(i[0]))},  # type: ignore[misc]
            resources=dict(resources),
            extra={name: document[name] for name in _RESERVED_SECTIONS if name in document},
        )

    @classmethod
    def from_toml(cls, file: str | os.PathLike[str]) -> BenchConfig:
        """Read ``bench.toml``; ``ConfigError`` when the file cannot be read or is invalid."""
        return cls.from_mapping(read_config_file(file), Path(file).parent)

    def resource(self, name: str) -> int | float:
        value = self.resources.get(name, RESOURCE_PARAMETERS[name].default)
        assert isinstance(value, (int, float))
        return value

    @property
    def has_checkouts(self) -> bool:
        return self.schemastore is not None or self.jsonschemabench is not None
