"""Weak supervision: unlabeled collections, leakage filtering, and Judgment_Call recording (task 39.4; Requirements 23.2–23.7).

The Trainer (``learn.trainer``) runs weak supervision after the
Labeling_Function thresholds are fitted. This module supplies its inputs.

Unlabeled collections (23.2, 23.7)
    :func:`load_unlabeled` reads the manifest ``{"collections": [{"id",
    "path"}, ...]}``; a ``path`` is a JSONL file (one document per line) or a
    JSON file holding one document, relative to the manifest. An entry with a
    duplicate id, a file that cannot be read, or no parseable document is
    excluded with its reason; a line that does not parse is skipped.

Leakage (23.6)
    :func:`real_dataset_identity` collects the ids and the canonical bytes
    of every document and Negative_Document of the Real_Datasets the
    Evaluation_Harness defines: the manifest's (33.1) and SchemaStore's
    (33.2). A collection whose id is a Real_Dataset's is excluded, and every
    document equal, as a parsed JSON value, to a Real_Dataset document is
    removed before anything else reads it.

Slices
    Benchmark collections are mostly small, while a fetched collection holds
    hundreds or thousands of documents, so each collection also yields
    seeded random sub-collections of the configured sizes
    (``rng("ws.slice", id, size, i)``, documents in their original order).
    Every slice and the full collection are recorded separately.

Recording
    :func:`record_judgments` runs the pipeline on a collection with the
    oracle runs' configuration (``trainer.featurization_config``: the
    Decision_Models and the cross-Path Techniques off, MDL resolving) and a
    :class:`RecordingEngine`, which keeps the Decision_Model feature vector
    of every Judgment_Call. The features are therefore computed exactly as
    for the synthetic examples, but no label is read or forced.
"""

from __future__ import annotations

import json
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path as FilePath
from typing import Any

import numpy as np
from joblib import Parallel, delayed

from jsinfer.config.resources import ResourceReader
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.determinism import canonical_bytes, codepoint_key, rng
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.core.workers import single_threaded
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.io.ingest import DocumentParseError, parse_document
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import ResolvedBundle
from jsinfer.learn.features import closed_enum_features
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.pipeline.infer import infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.values.inferrer import ValueInferrer

__all__ = [
    "RealIdentity",
    "RecordingEngine",
    "UnlabeledCollection",
    "UnlabeledSet",
    "load_unlabeled",
    "real_dataset_identity",
    "record_judgments",
    "record_slices",
    "slices",
]


@dataclass(frozen=True)
class RealIdentity:
    """The Real_Dataset ids and the canonical bytes of their documents (23.6)."""

    ids: frozenset[str]
    documents: frozenset[bytes]


def real_dataset_identity(real_manifest: str | None, corpus_manifest: Any | None) -> RealIdentity:
    """The identity of every Real_Dataset of the manifest and of SchemaStore (see the module docstring)."""
    from jsinfer.bench.real import load_manifest, schemastore_datasets  # deferred: bench imports learn

    datasets = []
    if real_manifest is not None:
        found, _ = load_manifest(real_manifest)
        datasets.extend(found)
    if corpus_manifest is not None:
        found, _ = schemastore_datasets(corpus_manifest)
        datasets.extend(found)
    documents: set[bytes] = set()
    for dataset in datasets:
        for _, raw in (*dataset.documents, *dataset.negatives):
            try:
                value, _ = parse_document(raw)
            except DocumentParseError:
                continue
            documents.add(canonical_bytes(value))
    return RealIdentity(frozenset(d.id for d in datasets), frozenset(documents))


@dataclass(frozen=True)
class UnlabeledCollection:
    """One unlabeled Document_Collection after leakage filtering."""

    id: str
    documents: tuple[JSON, ...]
    removed: int  # documents equal to a Real_Dataset document (23.6)
    skipped: int  # lines that did not parse


@dataclass
class UnlabeledSet:
    """The collections kept, the excluded entries with reasons, and the removed-document count (23.6, 23.7)."""

    collections: list[UnlabeledCollection] = field(default_factory=list)
    excluded: list[dict[str, JSON]] = field(default_factory=list)

    @property
    def removed_documents(self) -> int:
        return sum(c.removed for c in self.collections)


def _read(path: FilePath) -> tuple[list[JSON], int]:
    data = path.read_bytes()
    if path.suffix in (".jsonl", ".ndjson"):
        values, skipped = [], 0
        for line in data.splitlines():
            if not line.strip():
                continue
            try:
                values.append(parse_document(line)[0])
            except DocumentParseError:
                skipped += 1
        return values, skipped
    try:
        return [parse_document(data)[0]], 0
    except DocumentParseError:
        return [], 1


def load_unlabeled(manifest_path: str, identity: RealIdentity) -> UnlabeledSet:
    """The collections of ``manifest_path`` with the exclusions and removals of 23.6 and 23.7."""
    out = UnlabeledSet()
    manifest_file = FilePath(manifest_path)
    try:
        document = json.loads(manifest_file.read_text(encoding="utf-8"))
        entries = document["collections"]
        if not isinstance(entries, list):
            raise TypeError("collections must be a list")
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise ValueError(f"the unlabeled-collection manifest {manifest_path!r} cannot be read: {error}") from None
    seen: set[str] = set()
    for entry in entries:
        cid = entry.get("id") if isinstance(entry, Mapping) else None
        if not isinstance(cid, str) or not cid:
            out.excluded.append({"id": None, "reason": "an entry without an id"})
            continue
        if cid in seen:
            out.excluded.append({"id": cid, "reason": "duplicate identifier"})
            continue
        seen.add(cid)
        if cid in identity.ids:
            out.excluded.append({"id": cid, "reason": "the identifier of a Real_Dataset (23.6)"})
            continue
        try:
            values, skipped = _read(manifest_file.parent / str(entry.get("path", "")))
        except OSError as error:
            out.excluded.append({"id": cid, "reason": f"the documents cannot be read: {error.strerror or error}"})
            continue
        kept = [v for v in values if canonical_bytes(v) not in identity.documents]
        if not kept:
            reason = "no parseable document" if not values else "every document equals a Real_Dataset document"
            out.excluded.append({"id": cid, "reason": reason})
            continue
        out.collections.append(UnlabeledCollection(cid, tuple(kept), len(values) - len(kept), skipped))
    out.collections.sort(key=lambda c: codepoint_key(c.id))
    return out


def slices(
    collection: UnlabeledCollection, sizes: Sequence[int], per_size: int, seed: int
) -> list[tuple[str, tuple[JSON, ...]]]:
    """The full collection and its seeded sub-collections of each size below its length (see the module docstring)."""
    out = [(collection.id, collection.documents)]
    n = len(collection.documents)
    for size in sorted(set(sizes)):
        if size >= n:
            continue
        for i in range(per_size):
            chosen = np.sort(rng("ws.slice", collection.id, size, i, seed=seed).choice(n, size=size, replace=False))
            out.append((f"{collection.id}@{size}#{i}", tuple(collection.documents[j] for j in chosen.tolist())))
    return out


class RecordingEngine(DecisionEngine):
    """A DecisionEngine that keeps the feature vector of every Judgment_Call it resolves, reading no label."""

    __slots__ = ("records",)

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.records: list[tuple[str, np.ndarray]] = []

    def resolve(
        self,
        dp: DecisionPoint,
        candidates: Iterable[Candidate],
        view: View | None = None,
        features: Callable[[], np.ndarray] | None = None,
        **kwargs: Any,
    ) -> Decision:
        offered = list(candidates)
        if dp.kind in JUDGMENT_OUTCOMES and features is not None and len(offered) == 2 and all(c.score is not None for c in offered):
            self.records.append((dp.kind, np.asarray(features(), dtype=np.float64).reshape(-1)))
        return super().resolve(dp, offered, view, features, **kwargs)


def record_judgments(documents: Sequence[JSON], config: RunConfiguration) -> list[tuple[str, np.ndarray]]:
    """The ``(kind, features)`` of every Judgment_Call of one collection (see the module docstring)."""
    engines: list[RecordingEngine] = []

    def factory(cfg: RunConfiguration, bundle: ResolvedBundle, report: InferenceReport, validator: Validator) -> StructureAnalyzer:
        scorer = MDLScorer(validator=validator)
        engine = RecordingEngine(cfg.techniques, bundle=bundle, report=report, scorer=scorer)
        engines.append(engine)
        values = ValueInferrer(engine, validator=validator, report=report, scorer=scorer, enum_features=closed_enum_features)
        return StructureAnalyzer(values, scorer=scorer, resources=ResourceReader.for_run(cfg, report), seed=cfg.seed)

    infer_documents(list(documents), config, analyzer_factory=factory)
    return [record for engine in engines for record in engine.records]


def record_slices(
    items: Sequence[tuple[str, tuple[JSON, ...]]], config: RunConfiguration, workers: int = 1
) -> list[tuple[str, list[tuple[str, np.ndarray]]]]:
    """:func:`record_judgments` per ``(slice id, documents)``, in order; joblib processes when ``workers`` > 1."""
    if workers == 1:
        return [(sid, record_judgments(docs, config)) for sid, docs in items]
    task = single_threaded(record_judgments)
    results = Parallel(n_jobs=workers, batch_size=1)(delayed(task)(docs, config) for _, docs in items)
    return [(sid, records) for (sid, _), records in zip(items, results)]
