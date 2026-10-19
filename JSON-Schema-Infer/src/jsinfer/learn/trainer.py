"""Oracle-resolver featurization and Learned_Threshold fitting (Requirements 20.3, 20.4, 20.8, 22.6, 22.10).

Featurization (22.6, 22.10; design, Featurization)
    :func:`featurize_collection` runs the real pipeline on a generated
    Document_Collection with an :class:`OracleEngine`: a DecisionEngine
    that, at every Judgment_Call, computes the features the Inferrer would
    compute (the ``features`` callable of the call site, so the featurizers
    of ``learn.features`` run unchanged), aligns the Judgment_Call's Path to
    the source schema with ``learn.labels.align``, and records an
    :class:`Example` with the ground-truth label. Upstream Judgment_Calls
    take their ground-truth label when one is known, so the contexts of
    later calls match the true structure, and the MDL resolution otherwise
    (the Decision_Model Techniques are off in the featurization
    configuration). The ``variant_grouping`` Decision_Point gates
    Merge-or-Split: at a Path whose Merge-or-Split label is known, the
    oracle takes the lowest-description-length grouping with two or more
    candidate Variants when one was offered, so the labeled call is reached
    and featurized (a ``merge`` example is exactly a grouping a Technique
    proposed that the schema says to merge). Examples are excluded and
    counted per kind and cause (:data:`EXCLUSION_CAUSES`):
    ``unknown-label`` (22.5 recorded unknown), ``no-label`` (the kind does
    not apply at the aligned Path), ``unaligned-path`` (the inferred Path
    has no counterpart in the schema), and ``no-observation`` (a known
    label at a Path where no Judgment_Call of the kind was resolved: no
    Observation, or a rule such as 15.5 decided without the engine).

Threshold fitting (20.3, 20.4, 20.8; design, Learned_Thresholds)
    :func:`fit_threshold` is the deterministic grid search: the spec's
    candidate grid over the observed statistic, the objective evaluated at
    each candidate, the maximum taken, ties going to the smallest value,
    and the fitted value, objective value, example count, and
    Training_Data id stored in a ThresholdRecord. :func:`fit_m2_thresholds`
    fits the M2 thresholds on validation-split examples built by
    :func:`grouping_examples` and :func:`cooccurrence_examples`:

    - ``lsh.keyset_jaccard``, ``dbscan.eps`` with ``dbscan.min_samples``
      (jointly), and ``hdbscan.min_cluster_size`` maximize the mean
      adjusted Rand index of the grouping Technique's candidate Variants
      against the ground-truth branches, over the Paths the source schema
      splits (``merge_or_split`` label ``split``). Each Object_Signature is
      assigned to the branch whose ``required`` fields it holds and whose
      ``properties`` cover most of its keys; ``dbscan.min_samples`` is
      fitted in objects and ``hdbscan.min_cluster_size`` in distinct
      signatures, the units the Techniques use.
    - ``cooccur.min_support`` and ``cooccur.min_confidence`` (jointly)
      maximize the F1 of the Co_Occurrence_Rule candidates kept by both cuts
      against the ``dependentRequired`` entries of the source schema at the
      aligned Paths, over every Record Path; ``cooccur.min_confidence`` is
      fitted on the Confidence ``(count(a) + 1) / (count(a) + 2)``, so it is
      a minimum Support count in disguise.

    A threshold with no validation example is left out with the reason
    (:class:`OmittedThreshold`, 20.8).
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass, field, replace
from typing import Any, Final

import numpy as np
from sklearn.metrics import adjusted_rand_score

from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS, ResourceReader
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord, ThresholdSpec
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import ObservationStore
from jsinfer.core.paths import ROOT, Path, render_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.corpus.normalize import ref_target, walk_paths
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.deps.cooccurrence import rule_candidates
from jsinfer.deps.presence import presence
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import DECISION_MODEL_TECHNIQUES, ResolvedBundle
from jsinfer.learn.features import FEATURE_SPEC, closed_enum_features
from jsinfer.learn.labels import DEFAULT_MAX_DEPTH, UNKNOWN, Labels, SchemaPaths, derive_labels
from jsinfer.learn.synth import GeneratedCollection
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.record_map import object_rows
from jsinfer.structure.variants import (
    GroupingScorer,
    ObjectSignatures,
    dbscan_grouping,
    hdbscan_grouping,
    minhash_lsh_grouping,
)
from jsinfer.values.inferrer import ValueInferrer

__all__ = [
    "EXCLUSION_CAUSES",
    "M2_THRESHOLDS",
    "CooccurrenceExample",
    "Example",
    "ExampleSet",
    "FeaturizedCollection",
    "GroupingExample",
    "OmittedThreshold",
    "OracleEngine",
    "cooccurrence_examples",
    "featurization_config",
    "featurize_collection",
    "featurize_collections",
    "fit_m2_thresholds",
    "fit_threshold",
    "grouping_examples",
]

JUDGMENT_KINDS: Final[tuple[str, ...]] = tuple(JUDGMENT_OUTCOMES)
EXCLUSION_CAUSES: Final[tuple[str, ...]] = ("unknown-label", "no-label", "unaligned-path", "no-observation")
M2_THRESHOLDS: Final[tuple[str, ...]] = (
    "lsh.keyset_jaccard",
    "dbscan.eps",
    "dbscan.min_samples",
    "hdbscan.min_cluster_size",
    "cooccur.min_support",
    "cooccur.min_confidence",
)


# ---------------------------------------------------------------------------
# Examples
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class Example:
    """One featurized Judgment_Call with its ground-truth label."""

    kind: str
    path: Path  # the inferred Path
    schema_path: Path  # the aligned Path of the source schema
    features: np.ndarray
    label: str
    schema_id: str
    split: str | None
    k: int

    def to_json(self) -> dict[str, JSON]:
        return {
            "kind": self.kind,
            "path": render_path(self.path),
            "schema_path": render_path(self.schema_path),
            "features": self.features.tolist(),
            "label": self.label,
            "schema_id": self.schema_id,
            "split": self.split,
            "k": self.k,
        }


@dataclass(eq=False)
class ExampleSet:
    """Examples with the exclusion counts per kind and cause (22.6)."""

    examples: list[Example] = field(default_factory=list)
    exclusions: Counter[tuple[str, str]] = field(default_factory=Counter)

    def extend(self, other: ExampleSet) -> None:
        self.examples.extend(other.examples)
        self.exclusions.update(other.exclusions)

    def of_kind(self, kind: str, split: str | None = None) -> list[Example]:
        return [e for e in self.examples if e.kind == kind and (split is None or e.split == split)]

    def matrix(self, kind: str, split: str | None = None) -> tuple[np.ndarray, list[str]]:
        """``(X, y)`` of the examples of ``kind`` (and ``split``): features in spec order, labels."""
        chosen = self.of_kind(kind, split)
        dim = len(FEATURE_SPEC[kind])
        if not chosen:
            return np.zeros((0, dim), dtype=np.float64), []
        return np.stack([e.features for e in chosen]), [e.label for e in chosen]

    def exclusions_json(self) -> dict[str, dict[str, int]]:
        """``{kind: {cause: count}}`` with every kind and cause listed (zero when none)."""
        return {
            kind: {cause: int(self.exclusions.get((kind, cause), 0)) for cause in EXCLUSION_CAUSES}
            for kind in JUDGMENT_KINDS
        }

    def counts(self) -> dict[str, int]:
        return {kind: len(self.of_kind(kind)) for kind in JUDGMENT_KINDS}


# ---------------------------------------------------------------------------
# The oracle engine
# ---------------------------------------------------------------------------


class OracleEngine(DecisionEngine):
    """A DecisionEngine that records features and forces known labels at Judgment_Calls (see the module docstring)."""

    __slots__ = ("_examples", "_k", "_labels", "_paths", "_schema_id", "_seen", "_split")

    def __init__(
        self,
        techniques: Mapping[str, bool] | None,
        *,
        labels: Labels,
        paths: SchemaPaths,
        examples: ExampleSet,
        schema_id: str = "schema",
        split: str | None = None,
        k: int = 0,
        bundle: Any = None,
        report: InferenceReport | None = None,
        scorer: MDLScorer | None = None,
    ) -> None:
        super().__init__(techniques, bundle=bundle, report=report, scorer=scorer)
        self._labels = labels
        self._paths = paths
        self._examples = examples
        self._schema_id = schema_id
        self._split = split
        self._k = k
        self._seen: set[tuple[Path, str]] = set()

    @property
    def seen(self) -> frozenset[tuple[Path, str]]:
        """The ``(schema Path, kind)`` pairs that received an example."""
        return frozenset(self._seen)

    def resolve(
        self,
        dp: DecisionPoint,
        candidates: Iterable[Candidate],
        view: View | None = None,
        features: Callable[[], np.ndarray] | None = None,
        *,
        mdl_by: str | None = None,
        cutoff: Any = None,
        scorer: MDLScorer | None = None,
    ) -> Decision:
        offered = list(candidates)
        label: str | None = None
        if dp.kind in JUDGMENT_KINDS and features is not None and offered:
            path = dp.path if dp.path is not None else (view.path if view is not None else ROOT)
            aligned = self._paths.align(path)
            if aligned is None:
                self._examples.exclusions[dp.kind, "unaligned-path"] += 1
            else:
                truth = self._labels.get((aligned, dp.kind))
                if truth is None:
                    self._examples.exclusions[dp.kind, "no-label"] += 1
                elif truth == UNKNOWN:
                    self._examples.exclusions[dp.kind, "unknown-label"] += 1
                else:
                    x = np.asarray(features(), dtype=np.float64).reshape(-1)
                    self._examples.examples.append(
                        Example(dp.kind, path, aligned, x, truth, self._schema_id, self._split, self._k)
                    )
                    self._seen.add((aligned, dp.kind))
                    label = truth
        decision = super().resolve(dp, offered, view, features, mdl_by=mdl_by, cutoff=cutoff, scorer=scorer)
        if dp.kind == "variant_grouping":
            return self._expose_merge_or_split(dp, offered, view, decision)
        if label is not None and decision.label != label:
            forced = next((c for c in offered if c.label == label and not c.exceptional), None)
            if forced is not None:  # the oracle: upstream Judgment_Calls take the ground-truth label
                decision = replace(
                    decision,
                    label=forced.label,
                    outcome=forced.outcome,
                    candidate=forced,
                    details={**decision.details, "oracle": label},
                )
        return decision


    def _expose_merge_or_split(
        self, dp: DecisionPoint, offered: list[Candidate], view: View | None, decision: Decision
    ) -> Decision:
        """Prefer an offered multi-Variant grouping where the schema labels Merge-or-Split (see the module docstring)."""
        path = dp.path if dp.path is not None else (view.path if view is not None else ROOT)
        aligned = self._paths.align(path)
        label = None if aligned is None else self._labels.get((aligned, "merge_or_split"))
        if label is None or label == UNKNOWN or getattr(decision.outcome, "n_variants", 1) >= 2:
            return decision
        split = [c for c in offered if getattr(c.outcome, "n_variants", 1) >= 2 and c.score is not None]
        if not split:
            return decision
        best = min(split, key=lambda c: (c.score.total, c.order_key))  # type: ignore[union-attr]
        return replace(
            decision, label=best.label, outcome=best.outcome, candidate=best, details={**decision.details, "oracle": "merge_or_split"}
        )


# ---------------------------------------------------------------------------
# Featurization
# ---------------------------------------------------------------------------


def featurization_config(flags: Mapping[str, Any] | None = None) -> RunConfiguration:
    """The Run_Configuration of an oracle run: one worker, the Decision_Models off (MDL resolves), plus ``flags``."""
    settings: dict[str, Any] = {"workers": 1, **(flags or {})}
    for tid in DECISION_MODEL_TECHNIQUES.values():
        settings[technique_setting(tid)] = False
    return RunConfiguration.resolve(settings)


@dataclass(frozen=True, eq=False)
class FeaturizedCollection:
    """One collection's examples, its pipeline result, and the source schema's labels and Paths."""

    schema_id: str
    split: str | None
    k: int
    examples: ExampleSet
    result: PipelineResult
    labels: Labels
    paths: SchemaPaths
    model: SchemaModel

    @property
    def store(self) -> ObservationStore:
        return self.result.store


def featurize_collection(
    collection: GeneratedCollection,
    model: SchemaModel,
    *,
    config: RunConfiguration | None = None,
    max_depth: int = DEFAULT_MAX_DEPTH,
) -> FeaturizedCollection:
    """Run the pipeline on ``collection`` with the oracle resolver and collect its examples (22.6, 22.10)."""
    if not isinstance(collection, GeneratedCollection):
        raise TypeError(f"collection must be a GeneratedCollection, got {type(collection).__name__}")
    if not isinstance(model, SchemaModel):
        raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
    config = config if config is not None else featurization_config()
    for tid in DECISION_MODEL_TECHNIQUES.values():
        if config.techniques.get(tid, False):
            raise ValueError(f"the featurization configuration must disable {tid} (the oracle resolves by label or MDL)")
    labels = collection.labels or derive_labels(model, max_depth)
    paths = SchemaPaths.of(model, max_depth)
    examples = ExampleSet()
    engines: list[OracleEngine] = []

    def factory(cfg: RunConfiguration, bundle: ResolvedBundle, report: InferenceReport, validator: Validator) -> StructureAnalyzer:
        scorer = MDLScorer(validator=validator)
        engine = OracleEngine(
            cfg.techniques,
            labels=labels,
            paths=paths,
            examples=examples,
            schema_id=collection.schema_id,
            split=collection.split,
            k=collection.k,
            bundle=bundle,
            report=report,
            scorer=scorer,
        )
        engines.append(engine)
        values = ValueInferrer(engine, validator=validator, report=report, scorer=scorer, enum_features=closed_enum_features)
        return StructureAnalyzer(values, scorer=scorer, resources=ResourceReader.for_run(cfg, report), seed=cfg.seed)

    result = infer_documents(list(collection.documents), config, analyzer_factory=factory)
    seen = engines[0].seen if engines else frozenset()
    for (path, kind), label in labels.items():
        if label != UNKNOWN and (path, kind) not in seen:
            examples.exclusions[kind, "no-observation"] += 1
    return FeaturizedCollection(
        collection.schema_id, collection.split, collection.k, examples, result, labels, paths, model
    )


# ---------------------------------------------------------------------------
# Threshold fitting
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class OmittedThreshold:
    """A Learned_Threshold left out of the bundle, with the reason (20.8)."""

    name: str
    reason: str

    def to_json(self) -> dict[str, JSON]:
        return {"name": self.name, "reason": self.reason}


def fit_threshold(
    spec: ThresholdSpec,
    statistic: Any,
    objective: Callable[[float], float],
    *,
    n_examples: int,
    training_data: str,
) -> ThresholdRecord:
    """Grid-search ``spec`` over its candidates for ``statistic``, maximizing ``objective``; ties to the smallest value."""
    grid = spec.candidates(statistic)
    scores = [float(objective(float(value))) for value in grid.tolist()]
    if any(math.isnan(score) for score in scores):
        raise ValueError(f"the objective of {spec.name!r} is NaN at some candidate")
    best = max(scores)
    index = scores.index(best)  # the first maximum: ties go to the smallest candidate
    value = grid[index].item()
    return ThresholdRecord.from_spec(
        spec,
        value=int(value) if spec.integer else float(value),
        objective_value=best,
        n_examples=n_examples,
        training_data=training_data,
    )


def _fit_joint(
    specs: Sequence[ThresholdSpec],
    statistics: Sequence[Any],
    objective: Callable[..., float],
    *,
    n_examples: int,
    training_data: str,
) -> dict[str, ThresholdRecord]:
    """A joint grid search over several specs (the product of their grids); ties to the lexicographically smallest."""
    grids = [spec.candidates(statistic).tolist() for spec, statistic in zip(specs, statistics)]
    best: tuple[float, tuple[float, ...]] | None = None
    index = [0] * len(grids)
    while True:
        values = tuple(float(grid[i]) for grid, i in zip(grids, index))
        score = float(objective(*values))
        if math.isnan(score):
            raise ValueError(f"the joint objective of {[s.name for s in specs]!r} is NaN at {values!r}")
        if best is None or score > best[0]:
            best = (score, values)
        # advance the odometer, the last grid fastest
        position = len(index) - 1
        while position >= 0:
            index[position] += 1
            if index[position] < len(grids[position]):
                break
            index[position] = 0
            position -= 1
        if position < 0:
            break
    assert best is not None
    return {
        spec.name: ThresholdRecord.from_spec(
            spec,
            value=int(value) if spec.integer else float(value),
            objective_value=best[0],
            n_examples=n_examples,
            training_data=training_data,
        )
        for spec, value in zip(specs, best[1])
    }


# -- grouping examples (ARI) ---------------------------------------------------


@dataclass(frozen=True, eq=False)
class GroupingExample:
    """The Object_Signatures of one split Path and the ground-truth branch of each signature."""

    schema_id: str
    path: Path
    signatures: ObjectSignatures
    truth: np.ndarray  # branch index per signature

    def object_truth(self) -> np.ndarray:
        return self.truth[self.signatures.assignment]

    def pairwise_similarities(self) -> np.ndarray:
        """Key-set Jaccard similarity of every signature pair (the statistic of the Jaccard thresholds)."""
        keys = [frozenset(code >> 3 for code in codes) for codes in self.signatures.codes]
        out: list[float] = []
        for i in range(len(keys)):
            for j in range(i + 1, len(keys)):
                union = len(keys[i] | keys[j])
                out.append(len(keys[i] & keys[j]) / union if union else 1.0)
        return np.asarray(out, dtype=np.float64)


# An object branch of a split Path: its required fields, its property names, and whether it is closed.
_Branch = tuple[frozenset[str], frozenset[str], bool]


def _branch_schemas(model: SchemaModel, path: Path, max_depth: int) -> list[_Branch]:
    """The object branches of the ``anyOf``/``oneOf`` subschemas at ``path`` (a bare ``$ref`` branch is its target)."""
    branches: list[_Branch] = []
    for found, node in walk_paths(model, max_depth):
        if found != path or node.boolean is not None:
            continue
        for keyword in ("anyOf", "oneOf"):
            for branch in node.kw.get(keyword, ()):
                target = branch
                ref = branch.kw.get("$ref") if branch.boolean is None else None
                if isinstance(ref, str) and len(branch.kw) == 1:
                    pointer = ref_target(ref)
                    try:
                        target = model.resolve(pointer) if pointer is not None else branch
                    except (KeyError, ValueError):
                        target = branch
                if target.boolean is not None or "properties" not in target.kw:
                    continue
                required = frozenset(n for n in target.kw.get("required", ()) if isinstance(n, str))
                closed = isinstance(target.kw.get("additionalProperties"), SchemaNode) and target.kw[
                    "additionalProperties"
                ].boolean is False
                branches.append((required, frozenset(target.kw["properties"]), closed))
    return branches


def _assign_branches(signatures: ObjectSignatures, branches: Sequence[_Branch]) -> np.ndarray:
    """The branch of each signature: ``required`` held, then most keys covered (closed branches pay for extra keys), then the first."""
    store = signatures.objects.store
    truth = np.zeros(signatures.n_signatures, dtype=np.int64)
    for s, codes in enumerate(signatures.codes):
        keys = {store.key_name(code >> 3) for code in codes}
        best: tuple[int, int, int] | None = None
        for index, (required, names, closed) in enumerate(branches):
            holds = required <= keys
            covered = len(keys & names)
            extra = len(keys - names) if closed else 0
            score = (int(holds), covered - extra, -index)
            if best is None or score > best:
                best = score
                truth[s] = index
    return truth


def grouping_examples(featurized: Iterable[FeaturizedCollection], max_depth: int = DEFAULT_MAX_DEPTH) -> list[GroupingExample]:
    """One example per Path the source schema splits into two or more object branches, with its signatures."""
    out: list[GroupingExample] = []
    for item in featurized:
        root = View.root(item.store)
        done: set[Path] = set()
        for example in item.examples.of_kind("merge_or_split"):
            if example.label != "split" or example.path in done:
                continue
            done.add(example.path)
            branches = _branch_schemas(item.model, example.schema_path, max_depth)
            if len(branches) < 2:
                continue
            try:
                view = object_rows(root.descend(example.path))
            except (KeyError, ValueError):
                continue
            if view.is_empty:
                continue
            signatures = ObjectSignatures.of(view)
            if signatures.n_signatures < 2:
                continue
            out.append(GroupingExample(item.schema_id, example.path, signatures, _assign_branches(signatures, branches)))
    return out


def featurize_collections(
    collections: Iterable[tuple[GeneratedCollection, SchemaModel]],
    *,
    config: RunConfiguration | None = None,
    max_depth: int = DEFAULT_MAX_DEPTH,
) -> list[FeaturizedCollection]:
    """:func:`featurize_collection` for each ``(collection, model)``, in order."""
    return [featurize_collection(c, m, config=config, max_depth=max_depth) for c, m in collections]


def _mean_ari(examples: Sequence[GroupingExample], grouping: Callable[[GroupingExample], Sequence[Sequence[int]]]) -> float:
    scores: list[float] = []
    for example in examples:
        groups = grouping(example)
        predicted = np.zeros(example.signatures.n_signatures, dtype=np.int64)
        for index, group in enumerate(groups):
            predicted[np.asarray(group, dtype=np.int64)] = index
        scores.append(float(adjusted_rand_score(example.object_truth(), predicted[example.signatures.assignment])))
    return math.fsum(scores) / len(scores) if scores else 0.0


# -- co-occurrence examples (F1) -------------------------------------------------


@dataclass(frozen=True, eq=False)
class CooccurrenceExample:
    """The rule candidates of one Record Path with their Support counts, and the schema's ``dependentRequired`` pairs."""

    schema_id: str
    path: Path
    candidates: tuple[tuple[str, str, int], ...]  # (antecedent, consequent, support count)
    truth: frozenset[tuple[str, str]]


def _dependent_required(model: SchemaModel, path: Path, max_depth: int) -> frozenset[tuple[str, str]]:
    pairs: set[tuple[str, str]] = set()
    for found, node in walk_paths(model, max_depth):
        if found != path or node.boolean is not None:
            continue
        dependent = node.kw.get("dependentRequired")
        if isinstance(dependent, dict):
            for name in sorted(dependent, key=codepoint_key):
                for other in dependent[name]:
                    if isinstance(other, str):
                        pairs.add((name, other))
    return frozenset(pairs)


def cooccurrence_examples(
    featurized: Iterable[FeaturizedCollection], max_depth: int = DEFAULT_MAX_DEPTH
) -> list[CooccurrenceExample]:
    """One example per Record Path with at least one rule candidate or one ``dependentRequired`` pair in the schema."""
    out: list[CooccurrenceExample] = []
    for item in featurized:
        root = View.root(item.store)
        done: set[Path] = set()
        for example in item.examples.of_kind("record_or_map"):
            if example.label != "record" or example.path in done:
                continue
            done.add(example.path)
            try:
                view = object_rows(root.descend(example.path))
            except (KeyError, ValueError):
                continue
            if view.is_empty:
                continue
            candidates = tuple((c.antecedent, c.consequent, c.support_count) for c in rule_candidates(presence(view)))
            truth = _dependent_required(item.model, example.schema_path, max_depth)
            if candidates or truth:
                out.append(CooccurrenceExample(item.schema_id, example.path, candidates, truth))
    return out


def _rule_f1(examples: Sequence[CooccurrenceExample], min_support: float, min_confidence: float) -> float:
    tp = fp = fn = 0
    for example in examples:
        predicted = {
            (a, b)
            for a, b, count in example.candidates
            if count >= min_support and (count + 1) / (count + 2) >= min_confidence
        }
        tp += len(predicted & example.truth)
        fp += len(predicted - example.truth)
        fn += len(example.truth - predicted)
    return 2 * tp / (2 * tp + fp + fn) if (2 * tp + fp + fn) else 0.0


# -- the M2 thresholds -----------------------------------------------------------


def fit_m2_thresholds(
    featurized: Sequence[FeaturizedCollection],
    *,
    training_data: str,
    seed: int = 0,
    resources: Mapping[str, int | float] | None = None,
    max_depth: int = DEFAULT_MAX_DEPTH,
) -> tuple[dict[str, ThresholdRecord], list[OmittedThreshold]]:
    """Fit the M2 Learned_Thresholds on validation-split collections (see the module docstring).

    ``resources`` overrides the Resource_Parameters the grouping Techniques
    read (``minhash.perm``, ``lsh.bands``, ``sample.dbscan``,
    ``sample.hdbscan``); the registered defaults apply otherwise.
    """
    validation = [item for item in featurized if item.split in (None, "val", "validation")]

    def resource(name: str) -> int | float:
        value = (resources or {}).get(name, RESOURCE_PARAMETERS[name].default)
        assert isinstance(value, (int, float))
        return value

    records: dict[str, ThresholdRecord] = {}
    omitted: list[OmittedThreshold] = []
    groups = grouping_examples(validation, max_depth)
    if not groups:
        reason = "no validation-split example of a split object Path (variant_grouping)"
        omitted.extend(OmittedThreshold(name, reason) for name in M2_THRESHOLDS[:4])
    else:
        similarities = np.concatenate([g.pairwise_similarities() for g in groups])
        counts = np.concatenate([g.signatures.counts for g in groups]).astype(np.float64)
        sizes = np.asarray([g.signatures.n_signatures for g in groups], dtype=np.float64)
        scorers = {id(g): GroupingScorer(g.signatures, MDLScorer()) for g in groups}
        perm, bands = int(resource("minhash.perm")), int(resource("lsh.bands"))
        n = len(groups)

        def lsh(threshold: float) -> float:
            return _mean_ari(
                groups,
                lambda g: minhash_lsh_grouping(
                    g.signatures, scorers[id(g)], threshold=threshold, perm=perm, bands=bands, seed=seed
                ).groups,
            )

        records["lsh.keyset_jaccard"] = fit_threshold(
            THRESHOLDS["lsh.keyset_jaccard"], similarities, lsh, n_examples=n, training_data=training_data
        )
        sample_dbscan = int(resource("sample.dbscan"))

        def dbscan(eps: float, min_samples: float) -> float:
            return _mean_ari(
                groups,
                lambda g: dbscan_grouping(
                    g.signatures,
                    scorers[id(g)],
                    eps=eps,
                    min_samples=int(min_samples),
                    sample_size=sample_dbscan,
                    seed=seed,
                ).groups,
            )

        records.update(
            _fit_joint(
                (THRESHOLDS["dbscan.eps"], THRESHOLDS["dbscan.min_samples"]),
                (1.0 - similarities, counts),
                dbscan,
                n_examples=n,
                training_data=training_data,
            )
        )
        sample_hdbscan = int(resource("sample.hdbscan"))

        def hdbscan(min_cluster_size: float) -> float:
            return _mean_ari(
                groups,
                lambda g: hdbscan_grouping(
                    g.signatures,
                    scorers[id(g)],
                    min_cluster_size=int(min_cluster_size),
                    sample_size=sample_hdbscan,
                    seed=seed,
                ).groups,
            )

        records["hdbscan.min_cluster_size"] = fit_threshold(
            THRESHOLDS["hdbscan.min_cluster_size"], sizes, hdbscan, n_examples=n, training_data=training_data
        )

    rules = cooccurrence_examples(validation, max_depth)
    if not rules:
        reason = "no validation-split Record Path with a rule candidate or a dependentRequired entry (cooccurrence_rule)"
        omitted.extend(OmittedThreshold(name, reason) for name in M2_THRESHOLDS[4:])
    else:
        supports = np.asarray([count for r in rules for _, _, count in r.candidates], dtype=np.float64)
        confidences = (supports + 1.0) / (supports + 2.0)
        records.update(
            _fit_joint(
                (THRESHOLDS["cooccur.min_support"], THRESHOLDS["cooccur.min_confidence"]),
                (supports, confidences),
                lambda s, c: _rule_f1(rules, s, c),
                n_examples=len(rules),
                training_data=training_data,
            )
        )
    assert all(name in records or any(o.name == name for o in omitted) for name in M2_THRESHOLDS)
    return records, omitted


def thresholds_json(records: Mapping[str, ThresholdRecord]) -> dict[str, JSON]:
    """The ``thresholds.json`` content of ``records``, names in code-point order."""
    return {name: records[name].to_json() for name in sorted(records, key=codepoint_key)}


# Every M2 threshold is registered with the objective this module fits (a registry drift check).
assert all(THRESHOLDS[name].objective in ("grouping_ari", "cooccurrence_f1") for name in M2_THRESHOLDS)
assert all(name in REGISTRY.technique(THRESHOLDS[name].techniques[0]).thresholds for name in M2_THRESHOLDS)


# ---------------------------------------------------------------------------
# The training run (task 23.2; Requirements 21.1, 21.2, 21.3, 21.7, 21.8, 21.9, 22.1, 22.6)
# ---------------------------------------------------------------------------
#
# ``Trainer.run`` follows the design's training order with the stages that
# exist: inputs, schemas, generation, featurization, Decision_Models,
# Learned_Thresholds, manifest, and the atomic write. The Corpus_Index, the
# Labeling_Function thresholds, weak supervision, and the
# Semantic_Type_Classifier belong to Techniques that are not implemented (or
# are disabled in the training configuration), so their artifacts are
# omitted and the reasons recorded (21.9). A stage error raises
# ``TrainingError`` naming the stage, and no bundle is left behind (21.8).

import datetime as _datetime  # noqa: E402
import os as _os  # noqa: E402
import tempfile as _tempfile  # noqa: E402
import uuid as _uuid  # noqa: E402
from types import MappingProxyType  # noqa: E402
from pathlib import Path as _Path  # noqa: E402

from jsinfer.bench.splits import SPLITS, CorpusManifest, SchemaSplit, SplitFormatError  # noqa: E402
from jsinfer.config.errors import ConfigError, ConfigProblem  # noqa: E402
from jsinfer.config.run_config import read_config_file  # noqa: E402
from jsinfer.core.determinism import rng  # noqa: E402
from jsinfer.io.load import SchemaLoadError, SchemaLoader  # noqa: E402
from jsinfer.learn.bundle import (  # noqa: E402
    CORPUS_INDEX,
    SEMANTIC_MAPPING,
    SEMANTIC_TYPE,
    SEMANTIC_TYPE_CLASSIFIER,
    SUGGESTION_TECHNIQUES,
    BundleArtifact,
    decision_model_artifact,
    label_model_artifact,
    write_bundle,
)
from jsinfer.learn.decision_models import DEFAULT_ECE_BINS, OmittedModel, TrainedModel, train_decision_model  # noqa: E402
from jsinfer.learn.features import FEATURE_VERSION  # noqa: E402
from jsinfer.learn.synth import GeneratorConfig, SyntheticGenerator, ValueIndex  # noqa: E402

__all__ += ["FEATURIZATION_THRESHOLDS", "TRAINING_RESOURCES", "TrainingConfig", "TrainingError", "Trainer"]

# Provisional Learned_Threshold values of the oracle runs (design, Featurization): the featurization
# pipeline needs values for the grouping and rule Techniques to reach their Judgment_Calls at all. They
# are training-run settings recorded in the manifest, not the fitted values the bundle ships.
FEATURIZATION_THRESHOLDS: Final[Mapping[str, int | float]] = MappingProxyType(
    {
        "lsh.keyset_jaccard": 0.9,
        "dbscan.eps": 0.1,
        "dbscan.min_samples": 1,
        "hdbscan.min_cluster_size": 2,
        "cooccur.min_support": 1,
        "cooccur.min_confidence": 0.5,
    }
)

# The Resource_Parameters a training run reads, with their stage.
TRAINING_RESOURCES: Final[tuple[str, ...]] = (
    "train.max_schemas",
    "synth.collections_per_schema",
    "synth.doc_attempts",
    "synth.max_depth",
    "ece.bins",
    "minhash.perm",
    "lsh.bands",
    "sample.dbscan",
    "sample.hdbscan",
)
_WEAK_SUPERVISION: Final = "decide.weak_supervision"
_GENERATOR_FIELDS: Final = (
    "size_min",
    "size_max",
    "presence_min",
    "presence_max",
    "array_length_max",
    "enum_skew_min",
    "enum_skew_max",
    "string_max_length",
    "integer_span",
    "map_keys_max",
)
_STAGES: Final = ("inputs", "schemas", "generate", "featurize", "models", "thresholds", "manifest", "write")


class TrainingError(Exception):
    """An unrecoverable training failure: the failed ``stage`` and a message (21.8)."""

    def __init__(self, stage: str, message: str) -> None:
        super().__init__(stage, message)
        self.stage = stage
        self.message = message

    def __str__(self) -> str:
        return f"training stage {self.stage} failed: {self.message}"


def _config_error(subject: str, message: str) -> ConfigError:
    """A one-problem ConfigError: unknown names and settings read as ``unknown``, everything else as ``type``."""
    reason = "unknown" if "unknown" in message or "unregistered" in message or "is not a" in message else "type"
    return ConfigError([ConfigProblem(subject, reason, message)])


@dataclass(frozen=True)
class TrainingConfig:
    """A training configuration (21.2): inputs, seed, and the settings of each stage.

    ``corpus_manifest`` and ``schema_split`` are file paths. ``schemas``
    lists the schema ids to train on (default: every training-split and
    validation-split id of the Schema_Split); each must be in the corpus
    and assigned to the training or validation split (21.8).
    ``resources`` overrides :data:`TRAINING_RESOURCES`, ``techniques`` the
    states of the artifact-producing Techniques, ``generator`` the
    :class:`~jsinfer.learn.synth.GeneratorConfig` fields, and
    ``featurization`` the flat Run_Configuration settings of the oracle
    runs (``seed``, ``model_bundle``, and the Decision_Model toggles are set
    by the Trainer), and ``featurization_thresholds`` the provisional
    Learned_Threshold values those runs read
    (:data:`FEATURIZATION_THRESHOLDS` by default; a name set to None leaves
    the threshold out, so its Technique falls back).
    ``training_data`` names the Training_Data in the threshold records
    (default ``<split id>@<seed>``).
    """

    corpus_manifest: str
    schema_split: str
    seed: int = 0
    schemas: tuple[str, ...] | None = None
    training_data: str | None = None
    resources: Mapping[str, int | float] = field(default_factory=dict)
    techniques: Mapping[str, bool] = field(default_factory=dict)
    generator: Mapping[str, int | float] = field(default_factory=dict)
    featurization: Mapping[str, Any] = field(default_factory=dict)
    featurization_thresholds: Mapping[str, int | float | None] = field(default_factory=lambda: dict(FEATURIZATION_THRESHOLDS))

    def __post_init__(self) -> None:
        for name in ("corpus_manifest", "schema_split"):
            if not isinstance(getattr(self, name), str) or not getattr(self, name):
                raise _config_error(name, f"training configuration: {name} must be a non-empty path")
        if isinstance(self.seed, bool) or not isinstance(self.seed, int) or self.seed < 0:
            raise _config_error("seed", f"training configuration: seed must be a non-negative integer, got {self.seed!r}")
        if self.schemas is not None:
            if not all(isinstance(sid, str) and sid for sid in self.schemas):
                raise _config_error("schemas", "training configuration: schemas must be non-empty strings")
            object.__setattr__(self, "schemas", tuple(sorted(set(self.schemas), key=codepoint_key)))
        if self.training_data is not None and (not isinstance(self.training_data, str) or not self.training_data):
            raise _config_error("training_data", "training configuration: training_data must be a non-empty string")
        for name, value in self.resources.items():
            if name not in TRAINING_RESOURCES:
                raise _config_error(name, f"training configuration: {name!r} is not a training Resource_Parameter ({', '.join(TRAINING_RESOURCES)})")
            problem = RESOURCE_PARAMETERS[name].check(value)
            if problem is not None:
                raise _config_error(name, f"training configuration: {name}: {problem}")
        for tid, state in self.techniques.items():
            if tid not in REGISTRY.techniques:
                raise _config_error(tid, f"training configuration: unregistered Technique {tid!r}")
            if not isinstance(state, bool):
                raise _config_error(tid, f"training configuration: techniques.{tid} must be true or false")
        unknown = sorted(name for name in self.generator if name not in _GENERATOR_FIELDS)
        if unknown:
            raise _config_error("generator", f"training configuration: unknown generator settings {unknown}")
        if "model_bundle" in self.featurization:
            raise _config_error("featurization", "training configuration: the featurization bundle is built by the Trainer")
        for name, value in self.featurization_thresholds.items():
            if name not in THRESHOLDS:
                raise _config_error(name, f"training configuration: unregistered Learned_Threshold {name!r}")
            problem = None if value is None else THRESHOLDS[name].check_value(value)
            if problem is not None:
                raise _config_error(name, f"training configuration: featurization_thresholds.{name}: {problem}")
        try:
            self.generator_config()
        except ValueError as error:
            raise _config_error("generator", f"training configuration: generator: {error}") from None

    @classmethod
    def from_mapping(cls, document: Mapping[str, Any], base: str | _os.PathLike[str] | None = None) -> TrainingConfig:
        """A configuration from a parsed TOML document; relative paths resolve against ``base``."""
        training = document.get("training", {})
        if not isinstance(training, Mapping):
            raise _config_error("training", "training configuration: [training] must be a table")
        known = {"corpus_manifest", "schema_split", "seed", "schemas", "training_data"}
        unknown = sorted(k for k in training if k not in known)
        if unknown:
            raise _config_error("training", f"training configuration: unknown [training] settings {unknown}")
        sections = {}
        for section in ("resources", "techniques", "generator", "featurization"):
            table = document.get(section, {})
            if not isinstance(table, Mapping):
                raise _config_error(section, f"training configuration: [{section}] must be a table")
            sections[section] = dict(table)
        if "featurization_thresholds" in document:
            table = document["featurization_thresholds"]
            if not isinstance(table, Mapping):
                raise _config_error("featurization_thresholds", "training configuration: [featurization_thresholds] must be a table")
            sections["featurization_thresholds"] = {**FEATURIZATION_THRESHOLDS, **table}
        known_sections = ("training", "resources", "techniques", "generator", "featurization", "featurization_thresholds")
        extra = sorted(k for k in document if k not in known_sections)
        if extra:
            raise _config_error("training", f"training configuration: unknown sections {extra}")

        def path_of(name: str) -> Any:
            value = training.get(name)
            if isinstance(value, str) and base is not None and not _os.path.isabs(value):
                return _os.path.join(_os.fspath(base), value)
            return value

        schemas = training.get("schemas")
        return cls(
            corpus_manifest=path_of("corpus_manifest"),
            schema_split=path_of("schema_split"),
            seed=training.get("seed", 0),
            schemas=None if schemas is None else tuple(schemas) if isinstance(schemas, list) else schemas,
            training_data=training.get("training_data"),
            **sections,
        )

    @classmethod
    def from_toml(cls, path: str | _os.PathLike[str]) -> TrainingConfig:
        """Read ``jsinfer train -c FILE``; ``ConfigError`` when the file cannot be read or is invalid."""
        document = read_config_file(path)
        return cls.from_mapping(document, _Path(path).parent)

    def resource(self, name: str) -> int | float:
        value = self.resources.get(name, RESOURCE_PARAMETERS[name].default)
        assert isinstance(value, (int, float))
        return value

    def technique_enabled(self, tid: str) -> bool:
        return bool(self.techniques.get(tid, REGISTRY.technique(tid).default_enabled))

    def generator_config(self) -> GeneratorConfig:
        return GeneratorConfig(
            max_depth=int(self.resource("synth.max_depth")),
            doc_attempts=int(self.resource("synth.doc_attempts")),
            **{name: value for name, value in self.generator.items()},  # type: ignore[arg-type]
        )

    def to_json(self) -> dict[str, JSON]:
        return {
            "corpus_manifest": self.corpus_manifest,
            "schema_split": self.schema_split,
            "seed": self.seed,
            "schemas": None if self.schemas is None else list(self.schemas),
            "training_data": self.training_data,
            "resources": {name: self.resources[name] for name in sorted(self.resources, key=codepoint_key)},
            "techniques": {tid: self.techniques[tid] for tid in sorted(self.techniques, key=codepoint_key)},
            "generator": {name: self.generator[name] for name in sorted(self.generator, key=codepoint_key)},
            "featurization": {name: self.featurization[name] for name in sorted(self.featurization, key=codepoint_key)},
            "featurization_thresholds": {
                name: self.featurization_thresholds[name] for name in sorted(self.featurization_thresholds, key=codepoint_key)
            },
        }


def _trainer_version() -> str:
    import jsinfer  # deferred: the package's public API imports the pipeline

    return jsinfer.__version__


def _timestamp() -> str:
    return _datetime.datetime.now(_datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _subset(ids: Sequence[str], count: int, split: str, seed: int) -> tuple[str, ...]:
    """``count`` of the sorted ``ids``, drawn with ``rng("train.subset", split)`` (21.2, train.max_schemas)."""
    if count >= len(ids):
        return tuple(ids)
    stream = rng("train.subset", split, seed=seed)
    chosen = stream.choice(len(ids), size=count, replace=False)
    return tuple(sorted((ids[int(i)] for i in chosen.tolist()), key=codepoint_key))


class Trainer:
    """Runs a training run and writes the Model_Bundle (see the section comment above).

    ``loader`` reads the corpus schemas and ``validator`` validates the
    generated documents; new ones are made when None.
    """

    def __init__(self, loader: SchemaLoader | None = None, validator: Validator | None = None) -> None:
        self.validator = validator if validator is not None else Validator()
        self.loader = loader if loader is not None else SchemaLoader(self.validator)

    def run(self, cfg: TrainingConfig, out_dir: str | _os.PathLike[str]) -> str:
        """Train on ``cfg`` and write the Model_Bundle to ``out_dir``; returns the bundle id (21.1)."""
        if not isinstance(cfg, TrainingConfig):
            raise TypeError(f"cfg must be a TrainingConfig, got {type(cfg).__name__}")
        started = _timestamp()
        # 1. inputs (21.8)
        try:
            manifest = CorpusManifest.load(cfg.corpus_manifest)
            split = SchemaSplit.load(cfg.schema_split)
        except SplitFormatError as error:
            raise TrainingError("inputs", str(error)) from None
        requested = cfg.schemas if cfg.schemas is not None else (*split.train, *split.val)
        missing = [sid for sid in requested if sid not in manifest.entries]
        unassigned = [sid for sid in requested if split.split_of(sid) is None]
        held = [sid for sid in requested if split.split_of(sid) == "held_out"]
        problems = []
        if missing:
            problems.append(f"not in the corpus manifest: {sorted(missing, key=codepoint_key)}")
        if unassigned:
            problems.append(f"absent from the Schema_Split: {sorted(unassigned, key=codepoint_key)}")
        if held:
            problems.append(f"assigned to the held-out split: {sorted(held, key=codepoint_key)}")
        if problems:
            raise TrainingError("inputs", "; ".join(problems))
        by_split: dict[str, list[str]] = {"train": [], "val": []}
        for sid in requested:
            by_split[split.split_of(sid)].append(sid)  # type: ignore[index]
        for ids in by_split.values():
            ids.sort(key=codepoint_key)
        # 2. schemas, capped by train.max_schemas (21.2)
        cap = int(cfg.resource("train.max_schemas"))
        total = len(by_split["train"]) + len(by_split["val"])
        used: dict[str, tuple[str, ...]] = {}
        if total > cap:
            n_train = round(cap * len(by_split["train"]) / total)
            used["train"] = _subset(by_split["train"], n_train, "train", cfg.seed)
            used["val"] = _subset(by_split["val"], cap - len(used["train"]), "val", cfg.seed)
        else:
            used = {name: tuple(ids) for name, ids in by_split.items()}
        excluded: list[dict[str, JSON]] = []
        models: list[tuple[str, SchemaModel, str]] = []
        for name in ("train", "val"):
            for sid in used[name]:
                try:
                    models.append((sid, self.loader.load(manifest.schema_path(sid).read_bytes()), name))
                except (OSError, SchemaLoadError) as error:
                    excluded.append({"id": sid, "reason": f"unloadable schema: {error}"})
        loaded = {name: [sid for sid, _, s in models if s == name] for name in ("train", "val")}
        if not loaded["train"] or not loaded["val"]:
            raise TrainingError("schemas", "the training and validation splits each need at least one loadable schema")
        # 3. generate (22.1, 22.8)
        generator = SyntheticGenerator(self.validator, ValueIndex.build((sid, m) for sid, m, _ in models))
        per_schema = int(cfg.resource("synth.collections_per_schema"))
        try:
            collections, generation_excluded = generator.collections(models, cfg.generator_config(), cfg.seed, per_schema)
        except Exception as error:  # noqa: BLE001 - any failure here is a stage error (21.8)
            raise TrainingError("generate", f"{type(error).__name__}: {error}") from error
        excluded.extend({"id": sid, "reason": reason} for sid, reason in generation_excluded)
        model_of = {sid: m for sid, m, _ in models}
        # 4. featurize (22.6, 22.10), with the provisional thresholds in a temporary bundle
        max_depth = int(cfg.resource("synth.max_depth"))
        try:
            with _tempfile.TemporaryDirectory(prefix="jsinfer-featurization-") as scratch:
                provisional = {
                    name: ThresholdRecord.from_spec(THRESHOLDS[name], value=value, objective_value=0.0, n_examples=1, training_data="featurization")
                    for name, value in cfg.featurization_thresholds.items()
                    if value is not None
                }
                bundle_dir = write_bundle(_Path(scratch) / "bundle", bundle_id="featurization", thresholds=provisional)
                flags = {**dict(cfg.featurization), "seed": cfg.seed, "model_bundle": str(bundle_dir)}
                featurized = featurize_collections(
                    ((c, model_of[c.schema_id]) for c in collections), config=featurization_config(flags), max_depth=max_depth
                )
        except Exception as error:  # noqa: BLE001
            raise TrainingError("featurize", f"{type(error).__name__}: {error}") from error
        examples = ExampleSet()
        for item in featurized:
            examples.extend(item.examples)
        # 5. Decision_Models (21.3, 21.4, 21.5, 22.11), calibrated on the validation split
        artifacts: list[BundleArtifact] = []
        metrics: dict[str, JSON] = {}
        signals: dict[str, JSON] = {}
        omitted: list[dict[str, JSON]] = []
        ece_bins = int(cfg.resource("ece.bins"))
        for kind in JUDGMENT_KINDS:
            x_train, y_train = examples.matrix(kind, "train")
            x_val, y_val = examples.matrix(kind, "val")
            artifact = decision_model_artifact(kind, "synthetic")
            try:
                outcome = train_decision_model(
                    kind, x_train, y_train, x_val, y_val, seed=cfg.seed, ece_bins=ece_bins,
                    signal_features=("normalized_mdl_difference",),
                )
            except Exception as error:  # noqa: BLE001
                raise TrainingError("models", f"{artifact}: {type(error).__name__}: {error}") from error
            if isinstance(outcome, OmittedModel):
                omitted.append({"artifact": artifact, "reason": outcome.reason, "class_counts": outcome.to_json()["class_counts"]})
                continue
            assert isinstance(outcome, TrainedModel)
            artifacts.append(BundleArtifact.model(artifact, outcome.model))
            metrics[artifact] = outcome.metrics.to_json()
            signals[artifact] = {name: signal.to_json() for name, signal in outcome.signals.items()}
        # Artifacts of Techniques that are not implemented or are disabled (21.9)
        for kind in JUDGMENT_KINDS:
            for artifact, tid in ((decision_model_artifact(kind, "weak"), _WEAK_SUPERVISION), (label_model_artifact(kind), _WEAK_SUPERVISION)):
                omitted.append({"artifact": artifact, "reason": self._not_produced(cfg, tid), "class_counts": None})
        for artifact in (SEMANTIC_TYPE_CLASSIFIER, SEMANTIC_MAPPING):
            omitted.append({"artifact": artifact, "reason": self._not_produced(cfg, SEMANTIC_TYPE), "class_counts": None})
        omitted.append({"artifact": CORPUS_INDEX, "reason": self._not_produced(cfg, SUGGESTION_TECHNIQUES[0]), "class_counts": None})
        # 6. Learned_Thresholds on validation-split objectives (20.3, 20.4, 20.8)
        training_data = cfg.training_data if cfg.training_data is not None else f"{split.id}@{cfg.seed}"
        try:
            records, omitted_thresholds = fit_m2_thresholds(
                featurized, training_data=training_data, seed=cfg.seed,
                resources={name: cfg.resource(name) for name in ("minhash.perm", "lsh.bands", "sample.dbscan", "sample.hdbscan")},
                max_depth=max_depth,
            )
        except Exception as error:  # noqa: BLE001
            raise TrainingError("thresholds", f"{type(error).__name__}: {error}") from error
        for spec in THRESHOLDS.values():
            if spec.name not in records and not any(o.name == spec.name for o in omitted_thresholds):
                omitted_thresholds.append(OmittedThreshold(spec.name, "no training stage fits it yet (its Technique is not implemented)"))
        # 7. the manifest (21.2, 21.5, 22.2, 22.6, 22.8)
        bundle_id = f"bundle-{_uuid.uuid4()}"
        document: dict[str, JSON] = {
            "created": started,
            "trainer_version": _trainer_version(),
            "feature_version": FEATURE_VERSION,
            "training_config": cfg.to_json(),
            "seed": cfg.seed,
            "schema_split": {"id": split.id, "seed": split.seed},
            "schemas": {"train": list(loaded["train"]), "val": list(loaded["val"])},
            "schemas_available": {"train": len(by_split["train"]), "val": len(by_split["val"])},
            "excluded_schemas": sorted(excluded, key=lambda e: codepoint_key(str(e["id"]))),
            "unlabeled_collections": [],
            "ws_excluded": [],
            "ws_removed_docs": 0,
            "collections": [
                {"schema_id": c.schema_id, "k": c.k, "split": c.split, "sampled_params": c.params.to_json()} for c in collections
            ],
            "examples": examples.counts(),
            "example_exclusions": examples.exclusions_json(),
            "omitted_artifacts": omitted,
            "omitted_thresholds": [o.to_json() for o in omitted_thresholds],
            "metrics": metrics,
            "feature_signals": signals,
            "labeling_functions": [],
            "ws_examples": {},
        }
        # 8. the atomic write (21.1, 21.8)
        try:
            write_bundle(out_dir, bundle_id=bundle_id, artifacts=artifacts, thresholds=records, manifest=document)
        except OSError as error:
            raise TrainingError("write", f"the bundle cannot be written to {_os.fspath(out_dir)!r}: {error.strerror or error}") from None
        return bundle_id

    @staticmethod
    def _not_produced(cfg: TrainingConfig, tid: str) -> str:
        if not REGISTRY.technique(tid).implemented:
            return f"{tid} is not implemented"
        if not cfg.technique_enabled(tid):
            return f"{tid} is disabled in the training configuration"
        return f"{tid} produces no artifact in this trainer version"
