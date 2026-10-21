"""Good–Turing/Chao1 enum estimation and the Closed-or-Open-Enum Judgment_Call (Requirement 15).

Statistics (15.1)
    :class:`EnumStats` holds, over the non-flagged scalar Observations of a
    scalar Path, the observation count ``n``, the distinct value count
    ``s_obs``, the counts ``f1`` and ``f2`` of values observed exactly once
    and exactly twice, the Good–Turing unseen-mass estimate ``f1 / n``, and
    the Chao1 estimate ``s_obs + f1² / (2·f2)``, or
    ``s_obs + f1·(f1 − 1) / 2`` when ``f2`` is 0. Distinct values are
    distinct under draft 2020-12 ``enum`` equality: the store interns scalars
    by canonical key (``core.jsonvalue``), so ``1`` and ``1.0`` share a value
    id and ``true`` never equals ``1``. :func:`enum_statistics` reads the
    value-id counts of a view's ViewStats; :meth:`EnumStats.of_values` computes
    the same statistics for loose JSON values.

Judgment (15.2, 15.5, 15.6, 15.7)
    :func:`judge_enum` resolves the Closed-or-Open-Enum Decision_Point of a
    scalar Path through the DecisionEngine, in this order:

    1. No non-flagged scalar Observation: open (15.5). There is no
       Decision_Point to resolve and no statistics to record.
    2. ``values.enum_estimation`` disabled: the engine resolves the point
       with no candidates, so the Fallback_Behavior "open" applies and is
       recorded with its fallback indicator, whatever the state of
       ``decide.dm_closed_enum`` (15.7; the registry gates the Decision_Model
       on enum estimation). No statistics are computed.
    3. Every distinct value observed exactly once (``f1 == s_obs``): open
       (15.5). The rule overrides criteria 2 and 6, so the engine is not
       consulted; the judgment is recorded as a rejected enum candidate.
    4. Otherwise the engine resolves the closed and open candidates, both
       produced by ``values.enum_estimation``. The closed candidate is
       withheld (Document_Support, 15.9, ``decide.support``) when the
       Observations come from one document or some distinct value occurs in
       only one document: the enum inferred without that document would
       reject it. The engine then resolves the point without the
       Decision_Model, among the candidates left, and records the withheld
       candidate in the decision's details. A Decision_Model in effect
       (``decide.dm_closed_enum``, with enum estimation on and a model in the
       Model_Bundle) decides on ``features`` (15.2); the default features are
       the six statistics of 15.1, in :meth:`EnumStats.vector` order, which
       every Closed-or-Open-Enum feature vector includes. Otherwise enum
       estimation selects by MDL itself (``mdl_by``), whatever the state of
       ``decide.mdl`` (15.6), and a missing or disabled Decision_Model is
       recorded as its Fallback_Behavior (22.13, 22.14). DL ties go to open
       (the registry's mandated tie rule, 15.6).

Candidates and the Good–Turing charge (15.6)
    Both candidates are scored on the view of the Path's scalar
    Observations. The open candidate uses type-only coding (``{"type": …}``
    with the integer rule, ``pipeline.naive_merge.naive_type_names``), or
    the caller's ``open_node``. The closed candidate is ``{"enum": […]}``
    (``{"const": v}`` for one distinct value), whose data bits are the
    ``CAT`` of the member counts, plus the expected exception cost of unseen
    values (:func:`good_turing_charge`): ``f1 × mean EXC(singleton values)``
    in the design's terms, the 19.2 exception cost of ``f1`` exceptions
    among ``n`` rows, ``CAT(n − f1, f1) + Σ U(v)`` over the singleton values
    (the 1-bit exception flag is already paid by the node). Its rate
    ``f1 / n`` is the Good–Turing estimate, so the unseen values charged for
    have total probability ``f1 / n``; the charge is 0 when ``f1`` is 0. It
    is added to the closed candidate's data and exception bits.

Emission and records (15.3, 15.4, 15.8)
    A Closed_Enum emits ``const`` holding the single distinct value when
    ``s_obs`` is 1 and otherwise ``enum`` listing each distinct non-flagged
    scalar value exactly once, as canonical representatives in canonical
    value order (:meth:`EnumJudgment.keywords`). An open Path emits no
    ``enum`` or ``const``; its type, Format, Pattern, and bounds are the
    ValueInferrer's other steps. Whenever statistics were computed, the six
    statistics, the outcome, and the resolving Technique are recorded: in
    the Provenance_Record of the ``enum`` or ``const`` Constraint for a
    Closed_Enum (:meth:`EnumJudgment.provenance`, whose ``technique`` is the
    resolving Technique), and otherwise as a rejected ``enums`` candidate in
    the Inference_Report (:meth:`EnumJudgment.rejected`, recorded by
    :func:`judge_enum` when given the report). The rejected candidate is
    described by its keyword and distinct-value count rather than by its
    values, so the report stays small at Paths with many distinct values.

Determinism
    Statistics are integer counts over value ids, and costs are summed with
    ``math.fsum`` over distinct values, so every result depends only on the
    multiset of non-flagged Observations (19.6, 27.6). No cut-off is
    hard-coded: closure is an MDL comparison or a calibrated Decision_Model
    comparison.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, replace
from typing import Final, Literal

import numpy as np

from jsinfer.core.jsonvalue import JSON, CanonicalKey, canonical_key, canonical_value
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import FALLBACK_PREFIX, ProvenanceRecord, SampleInfo, Support
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.decide.support import documents_of, value_documents, withheld
from jsinfer.mdl.codes import CAT, U
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_type_names

__all__ = [
    "CLOSED",
    "CLOSED_OR_OPEN_ENUM",
    "DM_CLOSED_ENUM",
    "ENUM_ESTIMATION",
    "OPEN",
    "EnumFeatures",
    "EnumJudgment",
    "EnumStats",
    "enum_keywords",
    "enum_statistics",
    "good_turing_charge",
    "judge_enum",
    "scalar_view",
]

ENUM_ESTIMATION: Final = "values.enum_estimation"
DM_CLOSED_ENUM: Final = "decide.dm_closed_enum"
CLOSED_OR_OPEN_ENUM: Final = "closed_or_open_enum"
CLOSED, OPEN = JUDGMENT_OUTCOMES[CLOSED_OR_OPEN_ENUM]

# The requirement whose rule judged a Path open without the engine.
_RULE_ALL_SINGLETONS: Final = "15.5"

# Computes the Decision_Model features from the scalar view and the candidates (task 22.1 supplies one).
EnumFeatures = Callable[[View, Sequence[Candidate]], np.ndarray]

EnumKeyword = Literal["const", "enum"]


# ---------------------------------------------------------------------------
# Statistics (15.1)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class EnumStats:
    """The six statistics of 15.1 over a multiset of scalar values (see the module docstring)."""

    n: int
    s_obs: int
    f1: int
    f2: int
    good_turing: float
    chao1: float

    @classmethod
    def from_counts(cls, counts: Iterable[int] | np.ndarray) -> EnumStats:
        """The statistics of the per-distinct-value occurrence counts ``counts`` (each >= 1, at least one)."""
        values: list[int] = []
        for count in np.asarray(counts).tolist() if isinstance(counts, np.ndarray) else counts:
            if isinstance(count, (bool, np.bool_)) or not isinstance(count, (int, np.integer)):
                raise TypeError(f"occurrence counts must be ints, got {type(count).__name__}")
            if count < 1:
                raise ValueError(f"every distinct value occurs at least once, got a count of {count}")
            values.append(int(count))
        if not values:
            raise ValueError("enum statistics need at least one Observation")
        n = sum(values)
        s_obs = len(values)
        f1 = sum(1 for count in values if count == 1)
        f2 = sum(1 for count in values if count == 2)
        chao1 = s_obs + (f1 * f1) / (2 * f2) if f2 else s_obs + (f1 * (f1 - 1)) / 2
        return cls(n, s_obs, f1, f2, f1 / n, float(chao1))

    @classmethod
    def of_values(cls, values: Iterable[JSON]) -> EnumStats:
        """The statistics of loose JSON values, distinct under ``enum`` equality (``1`` equals ``1.0``)."""
        counts = Counter(canonical_key(value) for value in values)
        return cls.from_counts(counts.values())

    @property
    def all_singletons(self) -> bool:
        """Whether every distinct value was observed exactly once (``f1 == s_obs``), which forces open (15.5)."""
        return self.f1 == self.s_obs

    def vector(self) -> np.ndarray:
        """``[n, s_obs, f1, f2, good_turing, chao1]`` as float64: the 15.1 part of every feature vector (15.2)."""
        return np.array([self.n, self.s_obs, self.f1, self.f2, self.good_turing, self.chao1], dtype=np.float64)

    def to_json(self) -> dict[str, JSON]:
        """The statistics as recorded in provenance and the report (15.8)."""
        return {
            "chao1": self.chao1,
            "f1": self.f1,
            "f2": self.f2,
            "good_turing": self.good_turing,
            "n": self.n,
            "s_obs": self.s_obs,
        }


def scalar_view(view: View) -> View:
    """The non-flagged scalar Observations of ``view`` (null, boolean, number, and string rows)."""
    if not isinstance(view, View):
        raise TypeError(f"expected a View, got {type(view).__name__}")
    live = view.live()
    scalar = live.gather("value") >= 0
    return live if scalar.all() else live.subset(scalar)


def enum_statistics(view: View) -> EnumStats | None:
    """The 15.1 statistics of the non-flagged scalar Observations of ``view``; None when it has none."""
    stats = scalar_view(view).stats()
    if not len(stats.value_counts):
        return None
    return EnumStats.from_counts(stats.value_counts)


def good_turing_charge(view: View) -> float:
    """The closed candidate's expected exception cost of unseen values (15.6), in bits.

    ``CAT(n − f1, f1) + Σ U(v)`` over the values of the scalar Observations
    of ``view`` that occur exactly once: ``f1 × mean EXC(singleton values)``,
    the exception cost of ``f1`` exceptions among ``n`` rows at the
    Good–Turing rate ``f1 / n``. 0 when no value is a singleton.
    """
    scalars = scalar_view(view)
    stats = scalars.stats()
    counts = stats.value_counts
    singletons = stats.value_ids[counts == 1].tolist()
    if not singletons:
        return 0.0
    n = int(counts.sum())
    table = scalars.store.values
    terms = [CAT((n - len(singletons), len(singletons)))]
    terms.extend(U(table[vid]) for vid in singletons)
    return math.fsum(terms)


def enum_keywords(values: Sequence[JSON]) -> dict[str, JSON]:
    """The Closed_Enum Constraint of the distinct ``values`` (15.3): ``const`` for one, else ``enum``.

    Values are deduplicated under ``enum`` equality and listed as canonical
    representatives in canonical value order.
    """
    reps: dict[CanonicalKey, JSON] = {}
    for value in values:
        reps.setdefault(canonical_key(value), canonical_value(value))
    if not reps:
        raise ValueError("a Closed_Enum holds at least one value")
    ordered = [reps[key] for key in sorted(reps)]
    return {"const": ordered[0]} if len(ordered) == 1 else {"enum": ordered}


# ---------------------------------------------------------------------------
# Judgments
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class EnumJudgment:
    """The resolved Closed-or-Open-Enum Judgment_Call of one Path (see the module docstring).

    ``stats`` is None when no statistics were computed (no scalar
    Observation, or enum estimation disabled). ``values`` are the distinct
    values of a Closed_Enum in canonical order (empty when open).
    ``technique`` is the resolving Technique, ``fallback:closed_or_open_enum``
    for a Fallback_Behavior, or None when there was no Decision_Point.
    ``decision`` is the engine's Decision, None when the engine was not
    consulted; ``rule`` names the requirement that judged the Path open
    without it (15.5).
    """

    path: Path
    closed: bool
    stats: EnumStats | None
    values: tuple[JSON, ...]
    technique: str | None
    decision: Decision | None = None
    rule: str | None = None

    @property
    def outcome(self) -> str:
        return CLOSED if self.closed else OPEN

    @property
    def keyword(self) -> EnumKeyword | None:
        """``const`` or ``enum`` for a Closed_Enum, None when open."""
        if not self.closed:
            return None
        return "const" if len(self.values) == 1 else "enum"

    def keywords(self) -> dict[str, JSON]:
        """The keywords to emit: ``{"const": v}``, ``{"enum": [...]}``, or ``{}`` when open (15.3, 15.4)."""
        return enum_keywords(self.values) if self.closed else {}

    def details(self) -> dict[str, JSON]:
        """The 15.8 facts: the statistics and the outcome (the Technique is the record's own field)."""
        if self.stats is None:
            return {}
        details: dict[str, JSON] = {"enum_statistics": self.stats.to_json(), "outcome": self.outcome}
        if self.rule is not None:
            details["rule"] = self.rule
        return details

    def provenance(self, *, sample: SampleInfo | None = None) -> ProvenanceRecord:
        """The Provenance_Record of the emitted ``enum`` or ``const`` Constraint of a Closed_Enum (15.8).

        Support is every scalar Observation, all of which the Constraint
        accepts; Confidence is the decision's (MDL posterior or calibrated
        probability).
        """
        if not self.closed or self.stats is None or self.decision is None:
            raise ValueError(f"only a Closed_Enum has an enum Constraint; {render_path(self.path)} is open")
        support = Support(self.stats.n, self.stats.n)
        return self.decision.provenance(support, sample=sample, details=self.details())

    def rejected(self) -> RejectedCandidate | None:
        """The report entry of an open judgment with statistics (15.8); None otherwise."""
        if self.closed or self.stats is None:
            return None
        technique = self.technique
        if technique is not None and technique.startswith(FALLBACK_PREFIX):
            technique = None  # a Fallback_Behavior invokes no Technique (28.3)
        details = self.details()
        if self.decision is not None:
            details["method"] = self.decision.method
        candidate: dict[str, JSON] = {
            "distinct_values": self.stats.s_obs,
            "keyword": "const" if self.stats.s_obs == 1 else "enum",
        }
        return RejectedCandidate("enums", render_path(self.path), candidate, technique, details)


def _check_open_node(node: SchemaNode | None) -> None:
    if node is None:
        return
    if not isinstance(node, SchemaNode):
        raise TypeError(f"open_node must be a SchemaNode or None, got {type(node).__name__}")
    if node.boolean is not None:
        raise ValueError("open_node must be a keyword schema")
    if "enum" in node.kw or "const" in node.kw:
        raise ValueError("open_node is the open candidate and must not hold 'enum' or 'const'")


def _charged(score: Score, charge: float) -> Score:
    """``score`` with the Good–Turing charge added to its data and exception bits."""
    if not charge:
        return score
    return replace(
        score,
        data_bits=math.fsum((score.data_bits, charge)),
        exception_bits=math.fsum((score.exception_bits, charge)),
    )


def judge_enum(
    view: View,
    engine: DecisionEngine,
    *,
    report: InferenceReport | None = None,
    open_node: SchemaNode | None = None,
    features: EnumFeatures | None = None,
    scorer: MDLScorer | None = None,
) -> EnumJudgment:
    """Resolve the Closed-or-Open-Enum Judgment_Call of the scalar Path of ``view`` (15.2, 15.5–15.8).

    ``engine`` resolves the Decision_Point and records it in its own report.
    ``report`` receives the rejected enum candidate of an open judgment with
    statistics. ``open_node`` is the open candidate (type-only coding by
    default). ``features`` computes the Decision_Model features from the
    scalar view and the candidates (the six statistics by default).
    ``scorer`` scores both candidates (a default MDL_Scorer when None). See
    the module docstring for the order of the rules.
    """
    if not isinstance(engine, DecisionEngine):
        raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
    if report is not None and not isinstance(report, InferenceReport):
        raise TypeError(f"report must be an InferenceReport or None, got {type(report).__name__}")
    if scorer is not None and not isinstance(scorer, MDLScorer):
        raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
    _check_open_node(open_node)
    scalars = scalar_view(view)
    path = scalars.path
    value_stats = scalars.stats()
    if not len(value_stats.value_counts):  # 15.5: no non-flagged scalar Observation
        return EnumJudgment(path, False, None, (), None)

    dp = DecisionPoint(CLOSED_OR_OPEN_ENUM, path)
    if not engine.enabled(ENUM_ESTIMATION):  # 15.7: the Fallback_Behavior, whatever the Decision_Model's state
        decision = engine.resolve(dp, [], scalars)
        return EnumJudgment(path, False, None, (), decision.technique, decision)

    stats = EnumStats.from_counts(value_stats.value_counts)
    if stats.all_singletons:  # 15.5 overrides criteria 2 and 6
        judgment = EnumJudgment(path, False, stats, (), ENUM_ESTIMATION, rule=_RULE_ALL_SINGLETONS)
        _record(judgment, report)
        return judgment

    table = scalars.store.values
    values = tuple(table[vid] for vid in value_stats.value_ids.tolist())
    closed_node = SchemaNode(kw=enum_keywords(values))
    if open_node is None:
        open_node = SchemaNode(kw={"type": naive_type_names(value_stats)})
    scorer = scorer if scorer is not None else MDLScorer()
    closed_score = _charged(scorer.score(closed_node, scalars), good_turing_charge(scalars))
    open_score = scorer.score(open_node, scalars)
    # 15.9: the Closed_Enum must survive leave-one-document-out; a value only one document holds refutes it.
    documents = documents_of(scalars)
    single = int((value_documents(scalars) < 2).sum())
    refuted = withheld(documents, single_document_values=single) if documents < 2 or single else None
    candidates = (
        Candidate(
            technique=ENUM_ESTIMATION, outcome=CLOSED, order_key=(0,), node=closed_node, score=closed_score, withheld=refuted
        ),
        Candidate(technique=ENUM_ESTIMATION, outcome=OPEN, order_key=(1,), node=open_node, score=open_score),
    )

    def featurize() -> np.ndarray:
        return stats.vector() if features is None else features(scalars, candidates)

    decision = engine.resolve(dp, candidates, scalars, featurize, mdl_by=ENUM_ESTIMATION)
    closed = decision.label == CLOSED
    judgment = EnumJudgment(path, closed, stats, values if closed else (), decision.technique, decision)
    _record(judgment, report)
    return judgment


def _record(judgment: EnumJudgment, report: InferenceReport | None) -> None:
    """Record an open judgment with statistics as a rejected enum candidate (15.8)."""
    if report is None:
        return
    entry = judgment.rejected()
    if entry is not None:
        report.record_rejected(entry)
