"""The synthetic round-trip benchmark (task 29.1; Requirements 30.6, 31.1–31.14).

Data preparation (31.1, 31.11, 31.12)
    :func:`prepare` builds, for one Held_Out_Schema, the :class:`SyntheticData`
    of 31.1: one training Document_Collection per configured training size
    (:attr:`SyntheticConfig.sizes`), drawn by the Synthetic_Generator from
    the run seed with collection index ``k`` = the size's position, and the
    held-out Positive_Documents, drawn from collection index ``len(sizes)``
    and kept only when, as a parsed JSON value, they equal no training
    document. Candidates are drawn until :attr:`SyntheticConfig.held_out`
    distinct documents are found or the budget of ``held_out × synth.doc_attempts``
    candidates is spent; a schema the generator excludes, or whose budget
    runs out, is excluded from the benchmark at every size with its reason
    (31.12, as an :class:`Exclusion`). The retained Negative_Documents come
    from the Mutation_Operators (``learn.mutations.negatives``). The truth
    schema is loaded and re-emitted as draft 2020-12 so that every check
    (the Validator with ``format`` asserted, the Subschema_Checker) sees the
    same document.

Evaluation (30.6, 31.2–31.10, 31.13, 31.14)
    :func:`evaluate` runs every system (a ``bench.baselines`` adapter) on
    each training collection through ``bench.runner`` and scores the
    result against the same held-out positives and negatives at every size
    (31.11). The Inferrer adapter gets ``corpus.exclude_ids`` set to the
    Held_Out_Schema's whole near-duplicate group (30.6). Each
    :class:`SyntheticResult` holds recall, negative rejection, their
    harmonic mean, the per-operator rejection rates, the inclusion relation
    (or why it is unsupported), the Judgment_Call accuracies, and the
    Constraint precision and recall; a run that failed, timed out, was
    unavailable, or produced no loadable schema leaves every metric missing
    with the failure kind (31.13). Undefined metrics are ``None`` (31.14).

:func:`run_synthetic` ties the two together over a Schema_Split: every
held-out schema not excluded by the split (30.5) is prepared and evaluated,
and :class:`SyntheticResults` aggregates per system and size with the
counts of missing and undefined values (31.13, 31.14).
"""

from __future__ import annotations

import dataclasses
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from jsinfer.bench.baselines.base import BaselineAdapter
from jsinfer.bench.baselines.inferrer import InferrerAdapter
from jsinfer.bench.leakage import held_out_groups
from jsinfer.bench.metrics import (
    Aggregate,
    ConstraintScores,
    Metric,
    OperatorRate,
    aggregate,
    constraint_scores,
    harmonic_mean,
    judgment_accuracies,
    operator_rejection_rates,
    recall,
    rejection_rate,
)
from jsinfer.bench.runner import SystemRun, run_system, write_jsonl
from jsinfer.bench.splits import CorpusManifest, SchemaSplit
from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON, canonical_key
from jsinfer.core.schema_model import SchemaModel
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoadError, SchemaLoader
from jsinfer.io.subschema import Relation, SubschemaChecker, Unsupported
from jsinfer.io.validator import CompileError, Validator
from jsinfer.learn.mutations import Negative, negatives
from jsinfer.learn.synth import GeneratorConfig, SchemaExcluded, SyntheticGenerator

__all__ = [
    "Exclusion",
    "SyntheticConfig",
    "SyntheticData",
    "SyntheticResult",
    "SyntheticResults",
    "evaluate",
    "prepare",
    "run_synthetic",
    "split_excluded_ids",
]

_DOC_ATTEMPTS = int(RESOURCE_PARAMETERS["synth.doc_attempts"].default)  # type: ignore[arg-type]


@dataclass(frozen=True)
class SyntheticConfig:
    """The ``[bench]`` settings of the synthetic benchmark (31.1, 31.11).

    ``sizes`` are the training collection sizes (proposed default 10, 100,
    and 1,000) and ``held_out`` the count of held-out Positive_Documents
    (proposed default 1,000). ``seed`` is the run seed; ``timeout_s`` the
    per-run time limit (``bench.run_timeout_s`` when None); ``workers`` the
    Inferrer's worker count.
    """

    sizes: tuple[int, ...] = (10, 100, 1000)
    held_out: int = 1000
    seed: int = 0
    timeout_s: float | None = None
    subschema_timeout_s: float | None = None
    generator: GeneratorConfig = field(default_factory=GeneratorConfig)

    def __post_init__(self) -> None:
        if not self.sizes or any(isinstance(s, bool) or not isinstance(s, int) or s < 1 for s in self.sizes):
            raise ValueError("sizes must be one or more positive integers")
        if isinstance(self.held_out, bool) or not isinstance(self.held_out, int) or self.held_out < 1:
            raise ValueError("held_out must be a positive integer")
        if isinstance(self.seed, bool) or not isinstance(self.seed, int) or self.seed < 0:
            raise ValueError("seed must be a non-negative integer")

    @classmethod
    def from_mapping(cls, table: Mapping[str, Any], seed: int = 0) -> SyntheticConfig:
        """From a ``[bench]`` table: ``sizes``, ``held_out``, ``timeout_s``, ``subschema_timeout_s``."""
        unknown = sorted(k for k in table if k not in ("sizes", "held_out", "timeout_s", "subschema_timeout_s", "folds", "real_manifest"))
        if unknown:
            raise ValueError(f"unknown [bench] settings {unknown}")
        sizes = table.get("sizes", list(cls.sizes))
        if not isinstance(sizes, list):
            raise ValueError("[bench] sizes must be a list of positive integers")
        return cls(
            tuple(sizes),
            table.get("held_out", cls.held_out),
            seed,
            table.get("timeout_s"),
            table.get("subschema_timeout_s"),
        )

    def to_json(self) -> dict[str, JSON]:
        return {
            "sizes": list(self.sizes),
            "held_out": self.held_out,
            "seed": self.seed,
            "timeout_s": self.timeout_s,
            "subschema_timeout_s": self.subschema_timeout_s,
            "generator": self.generator.to_json(),
        }


@dataclass(frozen=True)
class Exclusion:
    """A Held_Out_Schema left out of the benchmark at every size, with the reason (31.12)."""

    schema_id: str
    reason: str

    def to_json(self) -> dict[str, JSON]:
        return {"schema_id": self.schema_id, "reason": self.reason}


@dataclass(frozen=True)
class SyntheticData:
    """The evaluation data of one Held_Out_Schema (31.1): see the module docstring."""

    schema_id: str
    truth: SchemaModel
    truth_document: dict[str, Any] | bool
    training: Mapping[int, tuple[JSON, ...]]  # size -> the training Document_Collection
    positives: tuple[JSON, ...]
    negatives: tuple[Negative, ...]
    mutation_skipped: tuple[str, ...] = ()
    candidates_drawn: int = 0

    def to_json(self) -> dict[str, JSON]:
        return {
            "schema_id": self.schema_id,
            "sizes": sorted(self.training),
            "positives": len(self.positives),
            "negatives": len(self.negatives),
            "mutation_skipped": list(self.mutation_skipped),
            "candidates_drawn": self.candidates_drawn,
        }


def _emit(model: SchemaModel) -> dict[str, Any] | bool:
    document, _ = SchemaEmitter().layout(model)
    return document


def prepare(
    model: SchemaModel,
    schema_id: str,
    cfg: SyntheticConfig,
    *,
    generator: SyntheticGenerator | None = None,
    validator: Validator | None = None,
) -> SyntheticData | Exclusion:
    """The :class:`SyntheticData` of ``model``, or its :class:`Exclusion` (31.1, 31.12; see the module docstring)."""
    validator = validator if validator is not None else Validator()
    generator = generator if generator is not None else SyntheticGenerator(validator)
    truth_document = _emit(model)
    try:
        validator.compile(truth_document, name=schema_id)
    except CompileError as error:
        return Exclusion(schema_id, f"the truth schema cannot be compiled: {error}")
    gen_cfg = cfg.generator
    training: dict[int, tuple[JSON, ...]] = {}
    seen: set[Any] = set()
    try:
        for k, size in enumerate(cfg.sizes):
            documents = tuple(generator.documents(model, size, (cfg.seed, schema_id, k), gen_cfg))
            training[size] = documents
            seen.update(canonical_key(document) for document in documents)
        positives: list[JSON] = []
        kept: set[Any] = set()
        budget = cfg.held_out * max(1, gen_cfg.doc_attempts)
        drawn = 0
        stream = generator.documents(model, budget, (cfg.seed, schema_id, len(cfg.sizes)), gen_cfg)
        for document in stream:
            drawn += 1
            key = canonical_key(document)
            if key in seen or key in kept:
                continue
            kept.add(key)
            positives.append(document)
            if len(positives) == cfg.held_out:
                break
    except SchemaExcluded as error:
        return Exclusion(schema_id, error.reason)
    if len(positives) < cfg.held_out:
        return Exclusion(
            schema_id,
            f"only {len(positives)} of {cfg.held_out} held-out documents distinct from the training documents "
            f"within {budget} candidates",
        )
    report = negatives(
        truth_document, schema_id, seed=cfg.seed, generator=generator, validator=validator, generator_config=dataclasses.replace(gen_cfg, size_min=1, size_max=1)
    )
    return SyntheticData(
        schema_id, model, truth_document, training, tuple(positives), tuple(report.negatives), tuple(report.skipped), drawn
    )


@dataclass(frozen=True)
class SyntheticResult:
    """One (system, Held_Out_Schema, training size) result (31.2–31.10, 31.13, 31.14).

    ``missing`` is the failure kind (``failure``, ``timeout``, or
    ``unavailable``) when the system produced no usable schema; every metric
    is then None.
    """

    schema_id: str
    size: int
    system: str
    missing: str | None = None
    reason: str | None = None
    recall: Metric = None
    rejection: Metric = None
    harmonic: Metric = None
    operators: Mapping[str, OperatorRate] = field(default_factory=dict)
    relation: Relation | Unsupported | None = None
    judgments: Mapping[str, Metric] = field(default_factory=dict)
    constraints: ConstraintScores | None = None
    seconds: float | None = None
    peak_rss_mb: float | None = None

    @property
    def ok(self) -> bool:
        return self.missing is None

    def to_json(self) -> dict[str, JSON]:
        relation: JSON
        if self.relation is None:
            relation = None
        elif isinstance(self.relation, Relation):
            relation = {"relation": self.relation.value}
        else:
            relation = {"relation": "unsupported", "cause": self.relation.cause, "message": self.relation.message, "keywords": list(self.relation.keywords)}
        return {
            "schema_id": self.schema_id,
            "size": self.size,
            "system": self.system,
            "missing": self.missing,
            "reason": self.reason,
            "recall": self.recall,
            "rejection": self.rejection,
            "harmonic": self.harmonic,
            "operators": {name: rate.to_json() for name, rate in self.operators.items()},
            "relation": relation,
            "judgments": dict(self.judgments),
            "constraints": None if self.constraints is None else self.constraints.to_json(),
            "seconds": self.seconds,
            "peak_rss_mb": self.peak_rss_mb,
        }


def _with_exclusions(adapter: BaselineAdapter, excluded: Sequence[str]) -> BaselineAdapter:
    """The Inferrer adapter with ``corpus.exclude_ids`` set to the held-out group (30.6); other adapters unchanged."""
    if isinstance(adapter, InferrerAdapter):
        return dataclasses.replace(adapter, settings={**adapter.settings, "corpus.exclude_ids": list(excluded)})
    return adapter


def _score(data: SyntheticData, size: int, run: SystemRun, validator: Validator, checker: SubschemaChecker, subschema_timeout: float | None) -> SyntheticResult:
    if run.kind != "schema" or run.model is None or run.document is None:
        return SyntheticResult(data.schema_id, size, run.system, run.kind, run.reason, seconds=run.seconds, peak_rss_mb=run.peak_rss_mb)
    try:
        compiled = validator.compile(run.document, name=f"{run.system}:{data.schema_id}:{size}")
    except CompileError as error:
        return SyntheticResult(data.schema_id, size, run.system, "failure", f"the Validator cannot load the schema: {error}", seconds=run.seconds, peak_rss_mb=run.peak_rss_mb)
    accepted = sum(1 for document in data.positives if validator.is_valid(compiled, document))
    rejected = [not validator.is_valid(compiled, negative.document) for negative in data.negatives]
    r = recall(accepted, len(data.positives))
    n = rejection_rate(sum(rejected), len(data.negatives))
    return SyntheticResult(
        data.schema_id,
        size,
        run.system,
        None,
        None,
        r,
        n,
        harmonic_mean(r, n),
        operator_rejection_rates(list(data.negatives), rejected),
        checker.relation(run.document, data.truth_document, subschema_timeout),
        judgment_accuracies(data.truth, run.model),
        constraint_scores(data.truth, run.model),
        run.seconds,
        run.peak_rss_mb,
    )


def evaluate(
    data: SyntheticData,
    systems: Sequence[BaselineAdapter],
    out_dir: Path,
    cfg: SyntheticConfig,
    *,
    excluded_ids: Sequence[str] = (),
    validator: Validator | None = None,
    checker: SubschemaChecker | None = None,
    env: dict[str, str] | None = None,
) -> list[SyntheticResult]:
    """Run every system at every size on ``data`` and score it (31.2–31.11, 31.13); see the module docstring.

    ``excluded_ids`` is the Held_Out_Schema's near-duplicate group, given to
    the Inferrer as ``corpus.exclude_ids`` (30.6). Results come in size then
    system order; the runs of size ``s`` and system ``x`` live in
    ``out_dir/<s>/<x>``.
    """
    validator = validator if validator is not None else Validator()
    checker = checker if checker is not None else SubschemaChecker(validator=validator)
    loader = SchemaLoader()
    excluded = list(excluded_ids) if excluded_ids else [data.schema_id]
    results: list[SyntheticResult] = []
    for size in cfg.sizes:
        size_dir = out_dir / str(size)
        size_dir.mkdir(parents=True, exist_ok=True)
        docs = size_dir / "docs.jsonl"
        write_jsonl(data.training[size], docs)
        for system in systems:
            adapter = _with_exclusions(system, excluded)
            run = run_system(adapter, docs, size_dir / system.name, seed=cfg.seed, timeout_s=cfg.timeout_s, validator=validator, loader=loader, env=env)
            results.append(_score(data, size, run, validator, checker, cfg.subschema_timeout_s))
    return results


def split_excluded_ids(split: SchemaSplit) -> dict[str, str]:
    """The schemas the Schema_Split excludes from synthetic generation, with their reasons (30.5)."""
    excluded = split.extra.get("excluded")
    out: dict[str, str] = {}
    if isinstance(excluded, list):
        for entry in excluded:
            if isinstance(entry, Mapping) and isinstance(entry.get("id"), str):
                out[entry["id"]] = str(entry.get("reason", ""))
    return out


@dataclass
class SyntheticResults:
    """Every result of a synthetic run, with the exclusions and the data summaries (31.12–31.14)."""

    config: SyntheticConfig
    results: list[SyntheticResult] = field(default_factory=list)
    exclusions: list[Exclusion] = field(default_factory=list)
    data: dict[str, dict[str, JSON]] = field(default_factory=dict)

    @property
    def systems(self) -> list[str]:
        return sorted({r.system for r in self.results}, key=codepoint_key)

    def select(self, system: str, size: int | None = None) -> list[SyntheticResult]:
        return [r for r in self.results if r.system == system and (size is None or r.size == size)]

    def missing(self, system: str, size: int | None = None) -> dict[str, int]:
        """Missing combinations of ``system`` per failure kind (31.13)."""
        counts: dict[str, int] = {}
        for r in self.select(system, size):
            if r.missing is not None:
                counts[r.missing] = counts.get(r.missing, 0) + 1
        return dict(sorted(counts.items()))

    def aggregate(self, system: str, size: int, metric: str) -> Aggregate:
        """The aggregate of ``metric`` (``recall``, ``rejection``, ``harmonic``, ``judgment:<kind>``, ``precision:<kind>``, ``recall:<kind>``) over the defined values (31.14)."""
        return aggregate(_metric_values(self.select(system, size), metric))

    def relations(self, system: str, size: int) -> dict[str, int]:
        """Counts of each inclusion relation and of ``unsupported`` (31.7, 31.8)."""
        counts: dict[str, int] = {}
        for r in self.select(system, size):
            if r.relation is None:
                continue
            name = r.relation.value if isinstance(r.relation, Relation) else "unsupported"
            counts[name] = counts.get(name, 0) + 1
        return dict(sorted(counts.items()))

    def to_json(self) -> dict[str, JSON]:
        return {
            "config": self.config.to_json(),
            "results": [r.to_json() for r in self.results],
            "exclusions": [e.to_json() for e in self.exclusions],
            "data": dict(self.data),
        }


def _metric_values(results: Iterable[SyntheticResult], metric: str) -> list[Metric]:
    values: list[Metric] = []
    for r in results:
        if r.missing is not None:
            continue
        if metric in ("recall", "rejection", "harmonic"):
            values.append(getattr(r, metric))
        elif metric.startswith("judgment:"):
            values.append(r.judgments.get(metric[len("judgment:") :]))
        elif metric.startswith(("precision:", "recall:")):
            side, kind = metric.split(":", 1)
            scores = r.constraints
            values.append(None if scores is None else getattr(scores, side).get(kind))
        else:
            raise ValueError(f"unknown metric {metric!r}")
    return values


def run_synthetic(
    manifest: CorpusManifest,
    split: SchemaSplit,
    systems: Sequence[BaselineAdapter],
    cfg: SyntheticConfig,
    out_root: Path,
    *,
    schema_ids: Sequence[str] | None = None,
    generator: SyntheticGenerator | None = None,
    validator: Validator | None = None,
    checker: SubschemaChecker | None = None,
    env: dict[str, str] | None = None,
) -> SyntheticResults:
    """Prepare and evaluate every eligible Held_Out_Schema of ``split`` (or of ``schema_ids``); see the module docstring."""
    validator = validator if validator is not None else Validator()
    generator = generator if generator is not None else SyntheticGenerator(validator)
    loader = SchemaLoader()
    groups = held_out_groups(split)
    by_group: dict[str, list[str]] = {}
    for sid, group in groups.items():
        by_group.setdefault(group, []).append(sid)
    excluded_by_split = split_excluded_ids(split)
    ids = list(schema_ids) if schema_ids is not None else list(split.held_out)
    out = SyntheticResults(cfg)
    for schema_id in ids:
        if schema_id in excluded_by_split:
            out.exclusions.append(Exclusion(schema_id, f"excluded by the Schema_Split: {excluded_by_split[schema_id]}"))
            continue
        try:
            model = loader.load(manifest.schema_path(schema_id).read_bytes())
        except (OSError, SchemaLoadError) as error:
            out.exclusions.append(Exclusion(schema_id, f"the schema cannot be loaded: {error}"))
            continue
        prepared = prepare(model, schema_id, cfg, generator=generator, validator=validator)
        if isinstance(prepared, Exclusion):
            out.exclusions.append(prepared)
            continue
        out.data[schema_id] = prepared.to_json()
        group_members = sorted(by_group.get(groups.get(schema_id, schema_id), [schema_id]), key=codepoint_key)
        out.results.extend(
            evaluate(prepared, systems, out_root / _safe(schema_id), cfg, excluded_ids=group_members, validator=validator, checker=checker, env=env)
        )
    return out


def _safe(schema_id: str) -> str:
    return "".join(c if c.isalnum() or c in "-._" else "_" for c in schema_id)
