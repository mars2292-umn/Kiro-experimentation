"""The real-data benchmark with cross-validation (task 29.2; Requirements 33.1–33.5, 33.7–33.9).

Real_Datasets (33.1, 33.2, 33.8)
    A :class:`RealDataset` is an identifier, the raw bytes of its positive
    documents and optional Negative_Documents (each with a document id),
    its known schema, and an optional Ordering_Key. :func:`load_manifest`
    reads definitions from a JSON manifest::

        {"datasets": [{"id": "my-dataset", "documents": "docs.jsonl" | "docs/",
                       "schema": "schema.json", "negatives": "neg.jsonl" | "neg/",
                       "ordering_key": "$.ts"}]}

    where a ``.jsonl`` file gives one document per line (id ``<file>#<line>``)
    and a directory gives one document per ``.json`` file. A definition
    without ``documents`` or ``schema``, with an identifier an earlier
    definition used, or naming a file that cannot be read is excluded with
    its reason (33.8, a :class:`DatasetExclusion`). :func:`schemastore_datasets`
    makes one Real_Dataset of every SchemaStore corpus schema that has at
    least one positive test file (33.2), the test files as documents.

Cleaning (33.5, 33.8)
    :func:`clean` parses every document, compiles the known schema with
    ``format`` asserted, and excludes, each with the dataset id, the
    document id, and the reason ``parse failure``, ``positive rejected``, or
    ``negative accepted``, the documents the schema's verdict contradicts.
    A dataset whose schema the Validator cannot compile, or with fewer than
    2 positives left, is excluded as a whole (33.8) with the remaining count.

Folds and metrics (33.3, 33.4, 33.7, 33.9)
    :func:`folds` partitions ``n`` documents into ``min(k, n)`` folds from
    a seeded permutation (``rng("real.folds", dataset id, seed=seed)``), the
    same for every system. :func:`evaluate` infers one schema per fold from
    the documents outside it, in their original order, through
    ``bench.runner``; recall is the fraction of positives the schema of
    their own fold accepts, the negative rejection rate the mean over folds
    of the fraction of Negative_Documents rejected, and the harmonic mean
    follows 31.5. A dataset with no Negative_Documents after cleaning gets
    them from the Mutation_Operators applied to the known schema (33.4);
    with none even then, its rejection rate and harmonic mean are not
    available and recorded as such (33.9). A system that fails, times out,
    is unavailable, or produces no loadable schema in any fold has every
    metric missing for the dataset with the failure kind.
"""

from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal

import numpy as np

from jsinfer.bench.baselines.base import BaselineAdapter
from jsinfer.bench.measure import description_length, schema_size
from jsinfer.bench.metrics import Aggregate, Metric, aggregate, harmonic_mean, recall, rejection_rate
from jsinfer.bench.runner import run_system, write_jsonl
from jsinfer.bench.splits import CorpusManifest
from jsinfer.core.determinism import codepoint_key, rng
from jsinfer.core.jsonvalue import JSON
from jsinfer.io.ingest import DocumentParseError, parse_document
from jsinfer.io.load import SchemaLoader
from jsinfer.io.validator import CompileError, Validator
from jsinfer.learn.mutations import negatives as mutation_negatives

__all__ = [
    "CleanedDataset",
    "DatasetExclusion",
    "DocumentExclusion",
    "RealConfig",
    "RealDataset",
    "RealResult",
    "RealResults",
    "clean",
    "evaluate",
    "folds",
    "load_manifest",
    "run_real",
    "schemastore_datasets",
]

NegativesSource = Literal["supplied", "generated", "none"]
DEFAULT_FOLDS = 5


@dataclass(frozen=True)
class RealConfig:
    """``[bench]`` settings of the real-data benchmark: ``folds`` (k ≥ 2, default 5), the seed, and the time limit."""

    folds: int = DEFAULT_FOLDS
    seed: int = 0
    timeout_s: float | None = None

    def __post_init__(self) -> None:
        if isinstance(self.folds, bool) or not isinstance(self.folds, int) or self.folds < 2:
            raise ValueError("folds must be an integer of at least 2")
        if isinstance(self.seed, bool) or not isinstance(self.seed, int) or self.seed < 0:
            raise ValueError("seed must be a non-negative integer")

    @classmethod
    def from_mapping(cls, table: Mapping[str, Any], seed: int = 0) -> RealConfig:
        """From a ``[bench]`` table: ``folds`` and ``timeout_s``."""
        return cls(table.get("folds", DEFAULT_FOLDS), seed, table.get("timeout_s"))

    def to_json(self) -> dict[str, JSON]:
        return {"folds": self.folds, "seed": self.seed, "timeout_s": self.timeout_s}


@dataclass(frozen=True)
class RealDataset:
    """A Real_Dataset definition (33.1): raw documents with ids, the known schema, optional negatives and Ordering_Key."""

    id: str
    documents: tuple[tuple[str, bytes], ...]
    schema: dict[str, Any] | bool
    negatives: tuple[tuple[str, bytes], ...] = ()
    ordering_key: str | None = None
    source: str = "manifest"


@dataclass(frozen=True)
class DatasetExclusion:
    """A Real_Dataset left out of every metric (33.8)."""

    dataset_id: str
    reason: str
    remaining: int | None = None

    def to_json(self) -> dict[str, JSON]:
        return {"dataset_id": self.dataset_id, "reason": self.reason, "remaining": self.remaining}


@dataclass(frozen=True)
class DocumentExclusion:
    """A document excluded from a Real_Dataset (33.5)."""

    dataset_id: str
    document_id: str
    reason: Literal["parse failure", "positive rejected", "negative accepted"]
    detail: str = ""

    def to_json(self) -> dict[str, JSON]:
        return {"dataset_id": self.dataset_id, "document_id": self.document_id, "reason": self.reason, "detail": self.detail}


@dataclass(frozen=True)
class CleanedDataset:
    """A Real_Dataset after cleaning (33.5): parsed positives and negatives in their original order."""

    id: str
    schema: dict[str, Any] | bool
    positives: tuple[tuple[str, JSON], ...]
    negatives: tuple[tuple[str, JSON], ...]
    excluded: tuple[DocumentExclusion, ...]
    ordering_key: str | None = None
    source: str = "manifest"

    @property
    def n(self) -> int:
        return len(self.positives)


# ---------------------------------------------------------------------------
# Definitions (33.1, 33.2, 33.8)
# ---------------------------------------------------------------------------


def _read_documents(path: Path) -> list[tuple[str, bytes]]:
    """``(document id, bytes)`` of a ``.jsonl`` file (one per line) or of the ``.json`` files of a directory."""
    if path.is_dir():
        files = sorted((p for p in path.iterdir() if p.is_file() and p.suffix == ".json"), key=lambda p: codepoint_key(p.name))
        return [(str(p), p.read_bytes()) for p in files]
    out: list[tuple[str, bytes]] = []
    with open(path, "rb") as handle:
        for number, line in enumerate(handle, 1):
            if line.strip():
                out.append((f"{path}#{number}", line))
    return out


def load_manifest(path: str | Path) -> tuple[list[RealDataset], list[DatasetExclusion]]:
    """Read Real_Dataset definitions (33.1); invalid ones are excluded with their reasons (33.8)."""
    path = Path(path)
    document = json.loads(path.read_text(encoding="utf-8"))
    entries = document.get("datasets") if isinstance(document, dict) else None
    if not isinstance(entries, list):
        raise ValueError(f"{path}: the manifest must be an object with a 'datasets' list")
    base = path.parent
    datasets: list[RealDataset] = []
    exclusions: list[DatasetExclusion] = []
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        dataset_id = entry.get("id") if isinstance(entry, dict) else None
        if not isinstance(dataset_id, str) or not dataset_id:
            exclusions.append(DatasetExclusion(f"#{index}", "the definition has no identifier"))
            continue
        if dataset_id in seen:
            exclusions.append(DatasetExclusion(dataset_id, "the identifier repeats an earlier definition"))
            continue
        seen.add(dataset_id)
        missing = [key for key in ("documents", "schema") if not isinstance(entry.get(key), str)]
        if missing:
            exclusions.append(DatasetExclusion(dataset_id, f"the definition lacks {' and '.join(missing)}"))
            continue
        try:
            documents = _read_documents(base / entry["documents"])
            schema = json.loads((base / entry["schema"]).read_text(encoding="utf-8"))
            negatives = _read_documents(base / entry["negatives"]) if isinstance(entry.get("negatives"), str) else []
        except (OSError, ValueError) as error:
            exclusions.append(DatasetExclusion(dataset_id, f"a referenced file cannot be read: {error}"))
            continue
        if not isinstance(schema, (dict, bool)):
            exclusions.append(DatasetExclusion(dataset_id, "the known schema is not a JSON Schema document"))
            continue
        ordering = entry.get("ordering_key")
        datasets.append(RealDataset(dataset_id, tuple(documents), schema, tuple(negatives), ordering if isinstance(ordering, str) else None))
    return datasets, exclusions


def schemastore_datasets(manifest: CorpusManifest) -> tuple[list[RealDataset], list[DatasetExclusion]]:
    """One Real_Dataset per SchemaStore schema with a positive test file (33.2), named by the schema id."""
    datasets: list[RealDataset] = []
    exclusions: list[DatasetExclusion] = []
    for schema_id in manifest.ids():
        entry = manifest.entries[schema_id]
        if entry.source != "schemastore":
            continue
        tests = entry.extra.get("tests")
        positive = tests.get("positive") if isinstance(tests, Mapping) else None
        if not isinstance(positive, list) or not positive:
            continue
        negative = tests.get("negative") if isinstance(tests, Mapping) else None
        try:
            schema = json.loads(manifest.schema_path(schema_id).read_text(encoding="utf-8"))
            documents = tuple((str(f), (manifest.root / str(f)).read_bytes()) for f in positive)
            negatives = tuple((str(f), (manifest.root / str(f)).read_bytes()) for f in (negative if isinstance(negative, list) else []))
        except (OSError, ValueError) as error:
            exclusions.append(DatasetExclusion(schema_id, f"a referenced file cannot be read: {error}"))
            continue
        if not isinstance(schema, (dict, bool)):
            exclusions.append(DatasetExclusion(schema_id, "the known schema is not a JSON Schema document"))
            continue
        datasets.append(RealDataset(schema_id, documents, schema, negatives, None, "schemastore"))
    return datasets, exclusions


# ---------------------------------------------------------------------------
# Cleaning (33.5, 33.8)
# ---------------------------------------------------------------------------


def clean(dataset: RealDataset, validator: Validator | None = None) -> CleanedDataset | DatasetExclusion:
    """Apply the exclusions of 33.5 to ``dataset``; the whole-dataset exclusions of 33.8 come back as such."""
    validator = validator if validator is not None else Validator()
    try:
        compiled = validator.compile(dataset.schema, name=dataset.id)
    except CompileError as error:
        return DatasetExclusion(dataset.id, f"the known schema cannot be used for validation: {error}")
    excluded: list[DocumentExclusion] = []
    positives: list[tuple[str, JSON]] = []
    negatives: list[tuple[str, JSON]] = []
    for kind, raw, keep in (("positive", dataset.documents, positives), ("negative", dataset.negatives, negatives)):
        for document_id, data in raw:
            try:
                value, _ = parse_document(data)
            except DocumentParseError as error:
                excluded.append(DocumentExclusion(dataset.id, document_id, "parse failure", error.reason))
                continue
            accepted = validator.is_valid(compiled, value)
            if kind == "positive" and not accepted:
                excluded.append(DocumentExclusion(dataset.id, document_id, "positive rejected"))
            elif kind == "negative" and accepted:
                excluded.append(DocumentExclusion(dataset.id, document_id, "negative accepted"))
            else:
                keep.append((document_id, value))
    if len(positives) < 2:
        return DatasetExclusion(dataset.id, f"fewer than 2 positive documents remain ({len(positives)})", len(positives))
    return CleanedDataset(dataset.id, dataset.schema, tuple(positives), tuple(negatives), tuple(excluded), dataset.ordering_key, dataset.source)


# ---------------------------------------------------------------------------
# Folds and evaluation (33.3, 33.4, 33.7, 33.9)
# ---------------------------------------------------------------------------


def folds(n: int, k: int, dataset_id: str, seed: int = 0) -> list[list[int]]:
    """``min(k, n)`` folds of the document indices ``0..n-1`` from a seeded permutation (33.3); each is sorted."""
    if isinstance(n, bool) or not isinstance(n, int) or n < 1:
        raise ValueError("n must be a positive integer")
    if isinstance(k, bool) or not isinstance(k, int) or k < 2:
        raise ValueError("k must be an integer of at least 2")
    order = rng("real.folds", dataset_id, seed=seed).permutation(n)
    parts = np.array_split(order, min(k, n))
    return [sorted(int(i) for i in part) for part in parts]


@dataclass(frozen=True)
class RealResult:
    """One (system, Real_Dataset) result (33.7, 33.9): see the module docstring."""

    dataset_id: str
    system: str
    n: int
    fold_count: int
    excluded: int
    negatives_source: NegativesSource
    negatives: int
    missing: str | None = None
    reason: str | None = None
    recall: Metric = None
    rejection: Metric = None
    harmonic: Metric = None
    fold_rejection: tuple[Metric, ...] = ()
    seconds: float | None = None
    peak_rss_mb: float | None = None
    schema_size: float | None = None  # mean over folds
    dl_bits: float | None = None  # mean over folds

    @property
    def ok(self) -> bool:
        return self.missing is None

    def to_json(self) -> dict[str, JSON]:
        return {
            "dataset_id": self.dataset_id,
            "system": self.system,
            "n": self.n,
            "folds": self.fold_count,
            "excluded": self.excluded,
            "negatives_source": self.negatives_source,
            "negatives": self.negatives,
            "missing": self.missing,
            "reason": self.reason,
            "recall": self.recall,
            "rejection": self.rejection,
            "harmonic": self.harmonic,
            "fold_rejection": list(self.fold_rejection),
            "seconds": self.seconds,
            "peak_rss_mb": self.peak_rss_mb,
            "schema_size": self.schema_size,
            "dl_bits": self.dl_bits,
        }


def _generated_negatives(dataset: CleanedDataset, seed: int, validator: Validator) -> list[tuple[str, JSON]]:
    report = mutation_negatives(dataset.schema, dataset.id, seed=seed, validator=validator)
    return [(f"mutation:{neg.operator}:{neg.site}:{i}", neg.document) for i, neg in enumerate(report.negatives)]


def evaluate(
    dataset: CleanedDataset,
    systems: Sequence[BaselineAdapter],
    out_dir: Path,
    cfg: RealConfig,
    *,
    validator: Validator | None = None,
    env: dict[str, str] | None = None,
) -> list[RealResult]:
    """Cross-validate every system on ``dataset`` (33.3, 33.4, 33.7, 33.9); runs live in ``out_dir/fold<i>/<system>``."""
    validator = validator if validator is not None else Validator()
    loader = SchemaLoader()
    negatives = list(dataset.negatives)
    source: NegativesSource = "supplied"
    if not negatives:
        negatives = _generated_negatives(dataset, cfg.seed, validator)
        source = "generated" if negatives else "none"
    assignment = folds(dataset.n, cfg.folds, dataset.id, cfg.seed)
    documents = [value for _, value in dataset.positives]
    fold_dirs: list[tuple[list[int], Path]] = []
    for i, fold in enumerate(assignment):
        fold_dir = out_dir / f"fold{i}"
        fold_dir.mkdir(parents=True, exist_ok=True)
        held = set(fold)
        write_jsonl([documents[j] for j in range(dataset.n) if j not in held], fold_dir / "docs.jsonl")
        fold_dirs.append((fold, fold_dir))
    results: list[RealResult] = []
    for system in systems:
        accepted = 0
        fold_rates: list[Metric] = []
        sizes: list[int] = []
        lengths: list[float] = []
        seconds = 0.0
        peak = 0.0
        missing: tuple[str, str | None] | None = None
        for fold, fold_dir in fold_dirs:
            run = run_system(system, fold_dir / "docs.jsonl", fold_dir / system.name, seed=cfg.seed, timeout_s=cfg.timeout_s, validator=validator, loader=loader, env=env)
            if run.seconds is not None:
                seconds += run.seconds
            if run.peak_rss_mb is not None:
                peak = max(peak, run.peak_rss_mb)
            if run.kind != "schema" or run.document is None:
                missing = (run.kind, run.reason)
                break
            try:
                compiled = validator.compile(run.document, name=f"{system.name}:{dataset.id}")
            except CompileError as error:
                missing = ("failure", f"the Validator cannot load the schema: {error}")
                break
            accepted += sum(1 for j in fold if validator.is_valid(compiled, documents[j]))
            fold_rates.append(rejection_rate(sum(1 for _, neg in negatives if not validator.is_valid(compiled, neg)), len(negatives)))
            sizes.append(schema_size(run.document))
            held = set(fold)
            bits = description_length(run.model, [documents[j] for j in range(dataset.n) if j not in held]) if run.model is not None else None
            if bits is not None:
                lengths.append(bits)
        base = dict(dataset_id=dataset.id, system=system.name, n=dataset.n, fold_count=len(assignment), excluded=len(dataset.excluded), negatives_source=source, negatives=len(negatives))
        if missing is not None:
            results.append(RealResult(**base, missing=missing[0], reason=missing[1], seconds=seconds, peak_rss_mb=peak))
            continue
        r = recall(accepted, dataset.n)
        defined = [rate for rate in fold_rates if rate is not None]
        n_rate: Metric = None if not defined else sum(defined) / len(defined)
        results.append(RealResult(
            **base, recall=r, rejection=n_rate, harmonic=harmonic_mean(r, n_rate), fold_rejection=tuple(fold_rates), seconds=seconds, peak_rss_mb=peak,
            schema_size=sum(sizes) / len(sizes) if sizes else None, dl_bits=sum(lengths) / len(lengths) if lengths else None,
        ))
    return results


@dataclass
class RealResults:
    """Every result of a real-data run with the dataset and document exclusions (33.5, 33.6, 33.8)."""

    config: RealConfig
    results: list[RealResult] = field(default_factory=list)
    dataset_exclusions: list[DatasetExclusion] = field(default_factory=list)
    document_exclusions: list[DocumentExclusion] = field(default_factory=list)

    @property
    def systems(self) -> list[str]:
        return sorted({r.system for r in self.results}, key=codepoint_key)

    def select(self, system: str) -> list[RealResult]:
        return [r for r in self.results if r.system == system]

    def missing(self, system: str) -> dict[str, int]:
        counts: dict[str, int] = {}
        for r in self.select(system):
            if r.missing is not None:
                counts[r.missing] = counts.get(r.missing, 0) + 1
        return dict(sorted(counts.items()))

    def aggregate(self, system: str, metric: str) -> Aggregate:
        """The aggregate of ``recall``, ``rejection``, or ``harmonic`` over the datasets where it is available (33.9)."""
        if metric not in ("recall", "rejection", "harmonic"):
            raise ValueError(f"unknown metric {metric!r}")
        return aggregate(getattr(r, metric) for r in self.select(system) if r.missing is None)

    def exclusion_reasons(self) -> dict[str, int]:
        """How many Real_Datasets each reason excluded (33.6)."""
        counts: dict[str, int] = {}
        for exclusion in self.dataset_exclusions:
            reason = exclusion.reason.split(":")[0].split(" (")[0]
            counts[reason] = counts.get(reason, 0) + 1
        return dict(sorted(counts.items()))

    def to_json(self) -> dict[str, JSON]:
        return {
            "config": self.config.to_json(),
            "results": [r.to_json() for r in self.results],
            "dataset_exclusions": [e.to_json() for e in self.dataset_exclusions],
            "document_exclusions": [e.to_json() for e in self.document_exclusions],
        }


def run_real(
    datasets: Sequence[RealDataset],
    systems: Sequence[BaselineAdapter],
    cfg: RealConfig,
    out_root: Path,
    *,
    exclusions: Sequence[DatasetExclusion] = (),
    validator: Validator | None = None,
    env: dict[str, str] | None = None,
) -> RealResults:
    """Clean and evaluate every Real_Dataset (see the module docstring); ``exclusions`` are the definition-level ones."""
    validator = validator if validator is not None else Validator()
    out = RealResults(cfg, dataset_exclusions=list(exclusions))
    for dataset in datasets:
        cleaned = clean(dataset, validator)
        if isinstance(cleaned, DatasetExclusion):
            out.dataset_exclusions.append(cleaned)
            continue
        out.document_exclusions.extend(cleaned.excluded)
        out.results.extend(evaluate(cleaned, systems, out_root / _safe(dataset.id), cfg, validator=validator, env=env))
    return out


def _safe(dataset_id: str) -> str:
    return "".join(c if c.isalnum() or c in "-._" else "_" for c in dataset_id)
