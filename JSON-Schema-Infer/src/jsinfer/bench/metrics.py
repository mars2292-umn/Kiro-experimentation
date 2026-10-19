"""Benchmark metrics (task 27.1; Requirements 31.3–31.5, 31.9, 31.10, 31.14, 32.8, 33.7, 33.9, 35.6).

Values and undefined (31.14)
    A metric is a ``float`` or ``None`` for undefined (a zero denominator:
    no held-out documents, no negatives, no labeled Path, no Constraint of
    a kind). :func:`recall` and :func:`rejection_rate` are counts over
    totals; :func:`harmonic_mean` is ``2RN / (R + N)``, 0 when both are 0,
    and undefined when either input is (31.3–31.5). :func:`aggregate` is
    the mean of the defined values with their count and the count of
    undefined ones (31.14, 35.12).

Per-operator rejection (32.8)
    :func:`operator_rejection_rates` gives, per Mutation_Operator, the
    number of Negative_Documents it produced and the fraction the inferred
    schema rejects, ``applicable=False`` (rate None) for an operator with no
    negatives.

Judgment accuracy (31.9)
    :func:`judgment_accuracies` derives labels from the inferred schema with
    the Synthetic_Generator's rules (``learn.labels.derive_labels``) and
    scores them against the ground truth's known labels; a labeled Path the
    inferred schema lacks is incorrect; a kind with no known label is
    undefined.

Constraints (31.10, 35.6)
    :func:`constraints_of` lists a schema's Constraints of the seven kinds,
    each keyed by its Path and what the kind compares: the field name for
    ``required`` entries, the Format value, the value set of a Closed_Enum
    (``const`` is a one-value set), the Discriminator and its value set for
    a Tagged_Union (an ``anyOf``/``oneOf`` whose branches each fix one
    common property to a distinct ``const`` or one-value ``enum``), the
    antecedent and its implied fields for a Co_Occurrence_Rule
    (``dependentRequired``), the member set for an Exclusion_Rule (``not:
    {required: [...]}``, or a ``oneOf`` of ``required``-only field groups,
    whose key is the set of groups), and the bound keyword for a bound,
    whatever its value. :func:`constraint_scores` aligns each inferred Path
    to the ground truth (``learn.labels.align``, a Record property over a
    Map becomes the Map step) and reports precision and recall per kind,
    undefined when the respective side has no Constraint of the kind.
    :func:`reliability` bins the emitted Constraints that carry a
    Provenance_Record by Confidence (``ece.bins`` equal-width bins), marks
    each correct when the ground truth holds the same Constraint at the
    aligned Path, and reports the bins (empty ones at count 0) with the
    expected calibration error of ``learn.decision_models``.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any, Final

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON, canonical_key
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import ProvenanceRecord
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.corpus.normalize import walk_paths
from jsinfer.learn.decision_models import expected_calibration_error
from jsinfer.learn.labels import DEFAULT_MAX_DEPTH, KINDS, SchemaPaths, derive_labels, judgment_accuracy
from jsinfer.learn.mutations import OPERATORS, Negative

__all__ = [
    "BOUND_KEYWORDS",
    "CONSTRAINT_KINDS",
    "Aggregate",
    "Constraint",
    "ConstraintScores",
    "OperatorRate",
    "ReliabilityBin",
    "ReliabilityCurve",
    "aggregate",
    "constraint_scores",
    "constraints_of",
    "harmonic_mean",
    "judgment_accuracies",
    "operator_rejection_rates",
    "recall",
    "rejection_rate",
    "reliability",
]

Metric = float | None

CONSTRAINT_KINDS: Final[tuple[str, ...]] = ("required", "format", "enum", "tagged_union", "cooccurrence", "exclusion", "bound")
BOUND_KEYWORDS: Final[tuple[str, ...]] = (
    "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf",
    "minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties",
)
_DEFAULT_BINS: Final[int] = int(RESOURCE_PARAMETERS["ece.bins"].default)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Rates (31.3–31.5, 31.14, 32.8)
# ---------------------------------------------------------------------------


def _rate(count: int, total: int) -> Metric:
    for value in (count, total):
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise ValueError(f"counts must be non-negative ints, got {value!r}")
    if count > total:
        raise ValueError(f"count {count} exceeds total {total}")
    return None if total == 0 else count / total


def recall(accepted: int, total: int) -> Metric:
    """Held-out positives accepted over all of them (31.3); undefined without positives."""
    return _rate(accepted, total)


def rejection_rate(rejected: int, total: int) -> Metric:
    """Negatives rejected over all of them (31.4); undefined without negatives."""
    return _rate(rejected, total)


def harmonic_mean(r: Metric, n: Metric) -> Metric:
    """``2RN / (R + N)``, 0 when both are 0, undefined when either is (31.5, 31.14)."""
    if r is None or n is None:
        return None
    if r == 0.0 and n == 0.0:
        return 0.0
    return 2.0 * r * n / (r + n)


@dataclass(frozen=True)
class Aggregate:
    """The mean of the defined values, how many contributed, and how many were undefined (31.14, 35.12)."""

    mean: Metric
    count: int
    undefined: int

    def to_json(self) -> dict[str, JSON]:
        return {"mean": self.mean, "count": self.count, "undefined": self.undefined}


def aggregate(values: Iterable[Metric]) -> Aggregate:
    items = list(values)
    defined = [float(v) for v in items if v is not None]
    return Aggregate(math.fsum(defined) / len(defined) if defined else None, len(defined), len(items) - len(defined))


@dataclass(frozen=True)
class OperatorRate:
    """Per Mutation_Operator: the negatives it produced and the fraction rejected; not applicable at zero (32.8)."""

    operator: str
    produced: int
    rejected: int

    @property
    def applicable(self) -> bool:
        return self.produced > 0

    @property
    def rate(self) -> Metric:
        return None if not self.applicable else self.rejected / self.produced

    def to_json(self) -> dict[str, JSON]:
        return {"operator": self.operator, "produced": self.produced, "rejected": self.rejected, "rate": self.rate, "applicable": self.applicable}


def operator_rejection_rates(negatives: Sequence[Negative], rejected: Sequence[bool]) -> dict[str, OperatorRate]:
    """Rejection per operator, from the negatives and whether the inferred schema rejected each (aligned by index)."""
    if len(negatives) != len(rejected):
        raise ValueError("negatives and rejected verdicts must have the same length")
    produced: Counter[str] = Counter()
    hits: Counter[str] = Counter()
    for negative, hit in zip(negatives, rejected):
        produced[negative.operator] += 1
        if hit:
            hits[negative.operator] += 1
    return {op: OperatorRate(op, produced[op], hits[op]) for op in OPERATORS}


# ---------------------------------------------------------------------------
# Judgment accuracy (31.9)
# ---------------------------------------------------------------------------


def judgment_accuracies(truth: SchemaModel, inferred: SchemaModel, max_depth: int = DEFAULT_MAX_DEPTH) -> dict[str, Metric]:
    """Per-kind accuracy of the inferred schema's labels against the ground truth's known labels (31.9)."""
    return judgment_accuracy(derive_labels(truth, max_depth), derive_labels(inferred, max_depth))


# ---------------------------------------------------------------------------
# Constraints (31.10, 35.6)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Constraint:
    """A Constraint of one of :data:`CONSTRAINT_KINDS` at a Path, keyed by what its kind compares (31.10)."""

    kind: str
    path: Path
    key: Any
    cid: ConstraintId | None = None  # the emitted Constraint's id, when the schema is an inferred model

    @property
    def identity(self) -> tuple[str, Path, Any]:
        return (self.kind, self.path, self.key)

    def to_json(self) -> dict[str, JSON]:
        return {"kind": self.kind, "path": render_path(self.path), "key": _json_key(self.key)}


def _json_key(key: Any) -> JSON:
    if isinstance(key, frozenset):
        return sorted((_json_key(k) for k in key), key=lambda v: str(v))
    if isinstance(key, tuple):
        return [_json_key(k) for k in key]
    return key


def _value_set(values: Iterable[Any]) -> frozenset[Any]:
    return frozenset(canonical_key(v) for v in values)


def _fixed_value(schema: Any) -> Any:
    if not isinstance(schema, SchemaNode) or schema.boolean is not None:
        return _MISSING
    if "const" in schema.kw:
        return schema.kw["const"]
    values = schema.kw.get("enum")
    if isinstance(values, list) and len(values) == 1:
        return values[0]
    return _MISSING


_MISSING: Final = object()


def _tagged_union(node: SchemaNode) -> tuple[str, str, frozenset[Any]] | None:
    """``(keyword, discriminator, values)`` when ``node`` holds a Tagged_Union (see the module docstring)."""
    for keyword in ("oneOf", "anyOf"):
        branches = node.kw.get(keyword)
        if not isinstance(branches, list) or len(branches) < 2:
            continue
        properties = [b.kw.get("properties") for b in branches if isinstance(b, SchemaNode) and b.boolean is None]
        if len(properties) != len(branches) or not all(isinstance(p, dict) for p in properties):
            continue
        common = set.intersection(*(set(p) for p in properties))  # type: ignore[arg-type]
        for key in sorted(common, key=codepoint_key):
            values = [_fixed_value(p[key]) for p in properties]  # type: ignore[index]
            if all(v is not _MISSING for v in values):
                keys = _value_set(values)
                if len(keys) == len(values):
                    return keyword, key, keys
    return None


def _exclusion(node: SchemaNode) -> tuple[str, int | None, frozenset[Any]] | None:
    negated = node.kw.get("not")
    if isinstance(negated, SchemaNode) and negated.boolean is None and set(negated.kw) - {"type"} in ({"required"}, {"anyOf"}):
        # One pair: {"required": [a, b]}; several: {"anyOf": [{"required": pair}, ...]}; an optional type guard.
        sets: list[Any] = [negated.kw["required"]] if "required" in negated.kw else [
            b.kw.get("required") for b in negated.kw["anyOf"] if isinstance(b, SchemaNode) and b.boolean is None and set(b.kw) == {"required"}
        ]
        if sets and all(isinstance(m, list) and len(m) >= 2 for m in sets) and len(sets) == len(negated.kw.get("anyOf", [None])):
            return "not", None, frozenset(name for m in sets for name in m)
    branches = node.kw.get("oneOf")
    if isinstance(branches, list) and len(branches) >= 2:
        groups = []
        for branch in branches:
            if not (isinstance(branch, SchemaNode) and branch.boolean is None and set(branch.kw) == {"required"}):
                return None
            required = branch.kw["required"]
            if not isinstance(required, list) or not required:
                return None
            groups.append(frozenset(required))
        return "oneOf", 0, frozenset(groups)
    return None


def constraints_of(model: SchemaModel, max_depth: int = DEFAULT_MAX_DEPTH) -> list[Constraint]:
    """The Constraints of ``model`` (31.10), Paths stepped as the Inferrer and the labels do, in Path order."""
    found: dict[tuple[str, Path, Any], Constraint] = {}

    def add(kind: str, path: Path, key: Any, cid: ConstraintId | None) -> None:
        found.setdefault((kind, path, key), Constraint(kind, path, key, cid))

    for path, node in walk_paths(model, max_depth):
        if node.boolean is not None:
            continue
        kw = node.kw
        required = kw.get("required")
        if isinstance(required, list):
            for name in required:
                if isinstance(name, str):
                    add("required", path, name, ConstraintId(node.nid, "required", name))
        fmt = kw.get("format")
        if isinstance(fmt, str):
            add("format", path, fmt, ConstraintId(node.nid, "format"))
        if "const" in kw:
            add("enum", path, _value_set([kw["const"]]), ConstraintId(node.nid, "const"))
        elif isinstance(kw.get("enum"), list):
            add("enum", path, _value_set(kw["enum"]), ConstraintId(node.nid, "enum"))
        union = _tagged_union(node)
        if union is not None:
            keyword, key, values = union
            add("tagged_union", path, (key, values), ConstraintId(node.nid, keyword, 0))
        dependent = kw.get("dependentRequired")
        if isinstance(dependent, dict):
            for name in sorted(dependent, key=codepoint_key):
                implied = dependent[name]
                if isinstance(implied, list) and implied:
                    add("cooccurrence", path, (name, frozenset(implied)), ConstraintId(node.nid, "dependentRequired", name))
        exclusion = _exclusion(node)
        if exclusion is not None:
            keyword, item, members = exclusion
            add("exclusion", path, members, ConstraintId(node.nid, keyword, item))
        for keyword in BOUND_KEYWORDS:
            if keyword in kw and not isinstance(kw[keyword], bool) and isinstance(kw[keyword], (int, float)):
                add("bound", path, keyword, ConstraintId(node.nid, keyword))
    return [found[key] for key in sorted(found, key=lambda k: (k[0], render_path(k[1]), str(_json_key(k[2]))))]


@dataclass(frozen=True)
class ConstraintScores:
    """Precision and recall per Constraint kind (31.10), undefined when a side has none of the kind (31.14)."""

    precision: Mapping[str, Metric]
    recall: Mapping[str, Metric]
    inferred: Mapping[str, int]
    truth: Mapping[str, int]
    matched: Mapping[str, int]

    def to_json(self) -> dict[str, JSON]:
        return {
            kind: {
                "precision": self.precision[kind],
                "recall": self.recall[kind],
                "inferred": self.inferred[kind],
                "truth": self.truth[kind],
                "matched": self.matched[kind],
            }
            for kind in CONSTRAINT_KINDS
        }


def _aligned(constraints: Iterable[Constraint], paths: SchemaPaths) -> list[Constraint]:
    out: list[Constraint] = []
    for c in constraints:
        aligned = paths.align(c.path)
        if aligned is not None:
            out.append(Constraint(c.kind, aligned, c.key, c.cid))
    return out


def constraint_scores(truth: SchemaModel, inferred: SchemaModel, max_depth: int = DEFAULT_MAX_DEPTH) -> ConstraintScores:
    """Constraint precision and recall of ``inferred`` against ``truth`` (31.10)."""
    truth_constraints = constraints_of(truth, max_depth)
    truth_keys = {c.identity for c in truth_constraints}
    paths = SchemaPaths.of(truth, max_depth)
    inferred_constraints = constraints_of(inferred, max_depth)
    aligned = _aligned(inferred_constraints, paths)
    inferred_keys = {c.identity for c in aligned}
    precision: dict[str, Metric] = {}
    rec: dict[str, Metric] = {}
    n_inferred: dict[str, int] = {}
    n_truth: dict[str, int] = {}
    n_matched: dict[str, int] = {}
    for kind in CONSTRAINT_KINDS:
        mine = {k for k in inferred_keys if k[0] == kind}
        theirs = {k for k in truth_keys if k[0] == kind}
        n_inferred[kind] = sum(1 for c in inferred_constraints if c.kind == kind)
        n_truth[kind] = len(theirs)
        n_matched[kind] = len(mine & theirs)
        precision[kind] = _rate(sum(1 for c in aligned if c.kind == kind and c.identity in theirs), n_inferred[kind])
        rec[kind] = _rate(n_matched[kind], n_truth[kind])
    return ConstraintScores(precision, rec, n_inferred, n_truth, n_matched)


# ---------------------------------------------------------------------------
# Reliability (35.6)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ReliabilityBin:
    low: float
    high: float
    count: int
    mean_confidence: Metric
    fraction_correct: Metric

    def to_json(self) -> dict[str, JSON]:
        return {"low": self.low, "high": self.high, "count": self.count, "mean_confidence": self.mean_confidence, "fraction_correct": self.fraction_correct}


@dataclass(frozen=True)
class ReliabilityCurve:
    bins: tuple[ReliabilityBin, ...]
    ece: Metric
    n: int

    def to_json(self) -> dict[str, JSON]:
        return {"bins": [b.to_json() for b in self.bins], "ece": self.ece, "n": self.n}


def reliability(
    truth: SchemaModel,
    inferred: SchemaModel,
    provenance: Mapping[ConstraintId, ProvenanceRecord],
    *,
    bins: int = _DEFAULT_BINS,
    max_depth: int = DEFAULT_MAX_DEPTH,
) -> ReliabilityCurve:
    """Reliability bins and ECE of the inferred schema's recorded Constraints against the ground truth (35.6)."""
    if isinstance(bins, bool) or not isinstance(bins, int) or bins < 1:
        raise ValueError(f"bins must be a positive integer, got {bins!r}")
    truth_keys = {c.identity for c in constraints_of(truth, max_depth)}
    paths = SchemaPaths.of(truth, max_depth)
    pairs: list[tuple[float, bool]] = []
    for constraint in constraints_of(inferred, max_depth):
        record = provenance.get(constraint.cid) if constraint.cid is not None else None
        if record is None:
            continue
        aligned = paths.align(constraint.path)
        correct = aligned is not None and (constraint.kind, aligned, constraint.key) in truth_keys
        pairs.append((float(record.confidence), correct))
    confidences = [c for c, _ in pairs]
    hits = [h for _, h in pairs]
    out: list[ReliabilityBin] = []
    for b in range(bins):
        low, high = b / bins, (b + 1) / bins
        members = [(c, h) for c, h in pairs if (low <= c < high) or (b == bins - 1 and c == 1.0)]
        out.append(
            ReliabilityBin(
                low,
                high,
                len(members),
                math.fsum(c for c, _ in members) / len(members) if members else None,
                sum(1 for _, h in members if h) / len(members) if members else None,
            )
        )
    ece = expected_calibration_error(confidences, hits, bins) if pairs else None
    return ReliabilityCurve(tuple(out), ece, len(pairs))


assert set(KINDS) == {"record_or_map", "closed_or_open_record", "tuple_or_list", "closed_or_open_enum", "merge_or_split"}
