"""Benchmark resume (task 41.5; Requirement 35.9, Property 59).

A benchmark run is a sequence of *combinations*: one system on one
Real_Dataset (``bench.real``), or one system on one Held_Out_Schema at one
training size (``bench.synthetic``, and through it the ablation study).
:class:`ResumeStore` keeps the result of every completed combination, so a
restart after an interruption runs only what is missing.

Records
    Each combination is identified by a :class:`ResumeKey`: the study, the
    system's :class:`~jsinfer.bench.baselines.base.BaselineInfo` (name,
    version, runtime, options), the identity and a content digest of its
    input (the cleaned documents, negatives, and known schema; or the
    training documents and the ground-truth schema), the study's
    configuration and seed, and the store's *fingerprint*. Its record is
    ``<root>/<sha256 of the key>.json`` holding the key, the result as JSON,
    and ``"complete": true``. Records are written by
    :func:`~jsinfer.core.determinism.atomic_write_bytes` (temporary file,
    fsync, rename), so a record is either absent or whole; the completion
    marker and the stored key are checked on every read as well.

Fingerprint
    :func:`fingerprint` digests the Python sources of the ``jsinfer``
    package and the Model_Bundle's ``manifest.json`` (which lists every
    artifact with its SHA-256). A code change or a retrained bundle under the
    same path changes every key, so stale results are never reused: "the
    same configuration" (35.9) includes the code and the bundle that produce
    and score the results. Baselines' versions are in their BaselineInfo.

Reuse and recomputation (35.9)
    :meth:`ResumeStore.load` returns the stored result of a key whose record
    is complete, without running any system; the caller rebuilds the result
    object from it (``RealResult.from_json``, ``SyntheticResult.from_json``),
    whose ``to_json`` gives back the stored JSON, so the results files equal
    those of an uninterrupted run. A record that is unreadable, lacks the
    completion marker, or stores another key is deleted and reported as
    discarded, and the combination is recomputed. Results of systems that
    were ``unavailable`` are not stored: no system ran, and the system may
    be available on the restart. Run directories (``runs/…``) are
    overwritten as before; they are diagnostics, not inputs to resume.
"""

from __future__ import annotations

import hashlib
import json
import shutil
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Final

import jsinfer
from jsinfer.core.determinism import atomic_write_bytes, canonical_bytes, encode_canonical
from jsinfer.core.jsonvalue import JSON

__all__ = ["RECORD_FORMAT", "ResumeKey", "ResumeStore", "content_digest", "fingerprint"]

RECORD_FORMAT: Final = 1


def content_digest(value: Any) -> str:
    """The SHA-256 of the canonical bytes of ``value`` (hex)."""
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def fingerprint(model_bundle: str | Path | None = None) -> dict[str, JSON]:
    """The digest of the ``jsinfer`` sources and of the Model_Bundle manifest (see the module docstring)."""
    package = Path(jsinfer.__file__).resolve().parent
    h = hashlib.sha256()
    for path in sorted(package.rglob("*.py"), key=lambda p: p.relative_to(package).as_posix()):
        if "__pycache__" in path.parts:
            continue
        h.update(path.relative_to(package).as_posix().encode("utf-8") + b"\0")
        h.update(path.read_bytes() + b"\0")
    out: dict[str, JSON] = {"code": h.hexdigest()}
    if model_bundle is not None:
        manifest = Path(model_bundle) / "manifest.json"
        out["bundle"] = hashlib.sha256(manifest.read_bytes()).hexdigest() if manifest.is_file() else None
    return out


@dataclass(frozen=True)
class ResumeKey:
    """One combination's identity: the canonical ``payload`` and its SHA-256 ``digest``."""

    payload: Mapping[str, JSON]
    digest: str


@dataclass
class ResumeStore:
    """Completed combination results under ``root`` (see the module docstring).

    ``reused``, ``stored``, and ``discarded`` count this run's reads that
    returned a result, records written, and invalid records removed.
    """

    root: Path
    fingerprint: Mapping[str, JSON] = field(default_factory=dict)
    reused: int = 0
    stored: int = 0
    discarded: int = 0

    def __post_init__(self) -> None:
        self.root = Path(self.root)
        self.fingerprint = dict(self.fingerprint)
        canonical_bytes(self.fingerprint)  # raises early on a value without a canonical form

    def key(self, study: str, **parts: JSON) -> ResumeKey:
        """The key of one combination of ``study`` from its ``parts`` and the store's fingerprint."""
        payload: dict[str, JSON] = {"format": RECORD_FORMAT, "study": study, "fingerprint": dict(self.fingerprint), **parts}
        return ResumeKey(payload, content_digest(payload))

    def path(self, key: ResumeKey) -> Path:
        return self.root / f"{key.digest}.json"

    def load(self, key: ResumeKey) -> dict[str, Any] | None:
        """The stored result of ``key``, or None when it is absent; an invalid record is deleted (35.9)."""
        path = self.path(key)
        if not path.is_file():
            return None
        try:
            record = json.loads(path.read_bytes())
            valid = (
                isinstance(record, dict)
                and record.get("complete") is True
                and record.get("format") == RECORD_FORMAT
                and canonical_bytes(record.get("key")) == canonical_bytes(key.payload)
                and isinstance(record.get("result"), dict)
            )
        except (OSError, ValueError, TypeError):
            valid = False
        if not valid:
            path.unlink(missing_ok=True)
            self.discarded += 1
            return None
        self.reused += 1
        return record["result"]

    def save(self, key: ResumeKey, result: Mapping[str, JSON]) -> None:
        """Store the result of a completed combination atomically, with its key and completion marker."""
        self.root.mkdir(parents=True, exist_ok=True)
        record = {"format": RECORD_FORMAT, "key": dict(key.payload), "result": dict(result), "complete": True}
        atomic_write_bytes(self.path(key), encode_canonical(record))
        self.stored += 1

    def clear(self) -> None:
        """Remove every stored record (a fresh run)."""
        if self.root.exists():
            shutil.rmtree(self.root)

    def summary(self) -> dict[str, int]:
        return {"reused": self.reused, "stored": self.stored, "discarded": self.discarded}
