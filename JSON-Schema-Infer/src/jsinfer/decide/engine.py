"""DecisionEngine: every Decision_Point is resolved here (Requirements 19, 20.7, 22, 23, 26.3, 28).

Techniques only generate :class:`Candidate` outcomes. :meth:`DecisionEngine.resolve`
picks one, records how in the Inference_Report (:class:`~jsinfer.core.report.DecisionEntry`),
and returns a :class:`Decision` whose :meth:`Decision.provenance` builds the
Provenance_Record of every Constraint the outcome produces.

Candidates
    A :class:`Candidate` names its producing Technique (or ``None`` for a
    structural alternative that no Technique produces, such as the Record and
    Map encodings of a Judgment_Call), its ``outcome`` and outcome ``label``,
    its ``order_key`` (the documented candidate order of 19.4), the partial
    schema ``node`` that expresses it, the ``rejects`` it would flag (an
    exception-bearing candidate, 17.2, 17.3), an optional precomputed MDL
    ``score`` (for scoring that needs more than ``MDLScorer.score(node, view)``,
    such as branch views or a Good–Turing charge), and ``tie_ranks`` for the
    ranking criteria of mandated tie rules (lower wins; a missing criterion
    ranks last).

    A candidate's Technique must serve the Decision_Point kind, except that
    ``robust.mdl_exceptions`` may offer exception-bearing candidates anywhere.
    No two candidates may share a Technique and an ``order_key``.

Canonical order
    Candidates are ordered by priority rank, then ``order_key``: Technique-less
    candidates first, then the serving Techniques in registry priority order,
    then ``robust.mdl_exceptions`` where it does not serve the kind. Every list
    the engine writes uses this order, and every choice it makes depends only
    on it, so results do not depend on the order in which candidates are
    supplied, on document order, or on the worker count (19.4, 27.5).

Eligibility (28.11, 20.6, 17.2)
    A candidate is eligible when its Technique is in effect: enabled, serving
    under its gate (15.7), and not missing a Learned_Threshold it reads at
    this kind (``threshold(name) is None`` in the Model_Bundle). Candidates of
    other Techniques are dropped, so disabled Techniques produce no
    candidates and no record names them. Exception-bearing candidates are
    eligible only while ``robust.mdl_exceptions`` is enabled. A
    Decision_Model or the priority order never selects one: they compete in
    the MDL step's pool and in the exception check that follows a
    Decision_Model or priority selection.

Resolution order
    1. **Decision_Model** (22.12, 23.8, 23.9). When the kind is a Judgment_Call
       kind and its ``decide.dm_*`` Technique is in effect, the model is
       ``dm.<kind>.weak`` while ``decide.weak_supervision`` is enabled and the
       bundle holds it, otherwise ``dm.<kind>.synthetic``. Each missing model
       is recorded as a report Substitution (22.14, 23.9), taken from the
       bundle's own substitutions when it lists one. The model scores the
       eligible widening candidates, whose labels must be its ``classes``,
       on ``features()``. The higher calibrated probability wins, with a
       "Decision_Model comparison" cut-off holding the calibrated
       probabilities and the artifact (20.7); equal probabilities are
       resolved by the MDL step's rule among the tied candidates, whatever
       the state of ``decide.mdl``, with an "MDL comparison" cut-off over
       the tied candidates (22.12, 20.7). Confidence is the selected
       probability. ``details["decision_model"]`` names the artifact that was
       consulted (23.9). The exception check follows.
    2. **MDL** (19.3, 19.4, 28.6). When ``decide.mdl`` is enabled, or when the
       caller passes ``mdl_by``, a serving Technique whose requirement defines
       its selection as an MDL comparison (13.8, 14.2, 15.6, 16.2, 7.8) and
       which is in effect, every eligible candidate is scored on the same
       view, provided some serving Technique can act. When none can (each
       is disabled, gated, or missing its model or a Learned_Threshold), the
       Fallback_Behavior applies even to Technique-less and exception-bearing
       candidates (28.4). The lowest description length wins. Ties go, in order, to the
       widening candidates (17.2, 17.3), the Decision_Point's mandated tie
       rule (:class:`~jsinfer.config.registry.TieRule`, applied favor by
       favor: an outcome label keeps the candidates with that label if any,
       a criterion keeps the lowest ``tie_rank``), fewer model bits, then
       the canonical order. Equal means equal floats: no tolerance, since a
       tolerance would be a hard-coded cut-off. Confidence is the code-length
       posterior ``2^-DL(c) / Σ 2^-DL(c')`` (:func:`mdl_posterior`). When an
       exception-bearing candidate wins, its rejected rows are flagged in the
       store (``FlagSet.flag_view``) and returned as ``Decision.flagged``; the
       caller records their report entries with the Constraints they would
       have loosened (17.5).
    3. **Priority** (19.7, 28.7). Otherwise the first produced candidate in
       canonical order wins. With two or more candidates this is the
       Fallback_Behavior of the disabled ``decide.mdl`` (19.7), recorded as
       such. Technique-less candidates are not ranked by priority. The
       exception check follows.
    4. **Fallback_Behavior** (28.4). Otherwise the kind's constant outcome
       applies, with no exception check: no serving Technique can act, so the
       Fallback_Behavior applies to every candidate, exception-bearing ones
       included. The Technique-less widening candidates are listed with it;
       one whose label equals the outcome is returned as
       ``Decision.candidate``. For the "documented candidate order" fallback
       of ``relaxation`` the first candidate in canonical order is selected.
       The fallback names the first serving Technique in priority order that
       is disabled, missing its model, or missing a Learned_Threshold, with
       that reason (26.10, 28.12). A serving Technique that is in effect and
       usable at this point means its candidates were never offered, which is
       a ``ValueError``.

    When a Judgment_Call is resolved by MDL (or later steps) because its
    Decision_Model is disabled or missing, that is the Decision_Model's
    Fallback_Behavior (22.13, 22.14) and is recorded with ``FallbackInfo``.

Exception check (17.2, 17.3)
    Exception encoding does not depend on ``decide.mdl``. When step 1 or 3
    selects a widening winner W and eligible exception-bearing candidates
    exist, W and those candidates are scored like the MDL step's pool (a
    precomputed ``score`` or ``MDLScorer.score(node, view)``) and the MDL
    step's rule picks among them, so the best exception-bearing candidate E
    (lowest description length, then the tie rule, fewer model bits, and
    canonical order) replaces W only when ``DL(E) < DL(W)``: ties go to
    widening.

    - If E wins, the decision is an MDL decision over W and the
      exception-bearing candidates, in canonical order: method ``mdl``,
      Confidence the code-length posterior over that set, an "MDL
      comparison" cut-off with their scores, and E's rejected rows flagged.
      The earlier step's fallback indicator and details are kept, and
      ``details["replaced"]`` names the earlier step's method.
    - If W survives, the earlier step's method, Confidence, cut-off, and
      fallback indicator stand, and ``details["exception_check"]`` holds the
      compared description lengths as a Resolution (method ``mdl``, W
      selected) in JSON form.

Records
    ``Decision.resolution`` lists every candidate the selecting step
    considered with its score (26.3): description length in bits with the
    model, data, and exception bits and counts for MDL (19.5); the calibrated
    probability for a Decision_Model; none for priority and fallback.
    ``Decision.cutoff`` is the cut-off (20.1, 20.7): the "MDL comparison"
    with the compared scores for MDL, for an MDL tie-break of equal
    probabilities, and for an exception-bearing winner of the exception
    check; the "Decision_Model comparison" with the calibrated probabilities
    and the artifact for a Decision_Model's argmax; the caller's ``cutoff``
    (a Learned_Threshold a producing Technique applied) for priority; and
    none for a constant fallback or a priority selection without a
    Learned_Threshold (no cut-off applied). The Provenance_Record's
    ``technique`` is the selected candidate's Technique, the resolving
    Technique for a Technique-less candidate (``robust.mdl_exceptions`` in
    the exception check), or ``fallback:<kind>``. Priority and fallback
    decisions carry no Confidence; :meth:`Decision.provenance` then uses the
    caller's or the Laplace default.

Technique states
    The engine takes the Technique states directly (ids it omits take their
    registered default) and does not require a Technique to be implemented,
    so resolution logic can be exercised before every Technique lands. A
    run builds it from the validated Run_Configuration with :meth:`DecisionEngine.for_run`.
"""

from __future__ import annotations

import math
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass, field, replace
from types import MappingProxyType
from typing import TYPE_CHECKING, Any, Final, Literal, Protocol, TypeAlias

import numpy as np

from jsinfer.config.registry import (
    OUTCOME_CANDIDATE_ORDER,
    REGISTRY,
    TIE_CRITERIA,
    DecisionPointSpec,
    TieRule,
)
from jsinfer.config.thresholds import THRESHOLDS, WEAK_SUPERVISION
from jsinfer.core.determinism import sorted_codepoint
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import ROOT, Path, Step, parse_path, render_path
from jsinfer.core.provenance import (
    DECISION_MODEL_COMPARISON,
    MDL_COMPARISON,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    ProvenanceRecord,
    Resolution,
    ResolutionMethod,
    SampleInfo,
    Support,
    fallback_technique,
    laplace_confidence,
)
from jsinfer.core.report import DecisionEntry, InferenceReport, Substitution
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.learn.bundle import DECISION_MODEL_TECHNIQUES, decision_model_artifact, required_models
from jsinfer.mdl.scorer import MDLScorer, Score

if TYPE_CHECKING:
    from jsinfer.config.run_config import RunConfiguration

__all__ = [
    "JUDGMENT_OUTCOMES",
    "MDL_EXCEPTIONS",
    "MDL_SELECTION",
    "ArtifactSource",
    "Candidate",
    "Decision",
    "DecisionEngine",
    "DecisionModelLike",
    "DecisionPoint",
    "mdl_posterior",
]

# The generic MDL selection Technique (19.3) and MDL exception encoding (17.2, 17.3).
MDL_SELECTION: Final = "decide.mdl"
MDL_EXCEPTIONS: Final = "robust.mdl_exceptions"

# The two outcome labels of each Judgment_Call kind (22.4). Decision_Models
# name these as their classes, and the analyzers label their candidates with them.
JUDGMENT_OUTCOMES: Final[Mapping[str, tuple[str, str]]] = MappingProxyType(
    {
        "record_or_map": ("record", "map"),
        "closed_or_open_record": ("closed", "open"),
        "tuple_or_list": ("tuple", "list"),
        "closed_or_open_enum": ("closed", "open"),
        "merge_or_split": ("merge", "split"),
    }
)

# A serving Technique that is enabled but whose serving gate is disabled (15.7).
_GATED: Final = "gated"
_Unavailable: TypeAlias = Literal["disabled", "missing-threshold", "missing-model", "gated"]


# ---------------------------------------------------------------------------
# Protocols for bundle-supplied artifacts
# ---------------------------------------------------------------------------


class DecisionModelLike(Protocol):
    """What the engine needs from a Decision_Model (task 22.4 supplies calibrated ones).

    ``classes`` are the outcome labels in the column order of
    ``predict_proba``, which maps a ``(1, n_features)`` array to a
    ``(1, len(classes))`` array of calibrated probabilities.
    """

    @property
    def classes(self) -> Sequence[str]: ...

    def predict_proba(self, x: np.ndarray) -> np.ndarray: ...


class ArtifactSource(Protocol):
    """What the engine reads from a Model_Bundle; ``learn.bundle.ResolvedBundle`` satisfies it.

    ``model`` returns a loaded artifact or None, ``threshold`` a usable
    Learned_Threshold value or None. A source may also expose
    ``substitutions`` (the bundle stage's Substitutions), which the engine
    records verbatim for missing Decision_Models, and ``bundle_id``, which
    :meth:`DecisionEngine.threshold` reports with every value read (20.5).
    """

    def model(self, name: str) -> Any | None: ...

    def threshold(self, name: str) -> int | float | None: ...


# ---------------------------------------------------------------------------
# Construction checks
# ---------------------------------------------------------------------------


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _name(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a str, got {type(value).__name__}")
    if not value:
        raise ValueError(f"{what} must be non-empty")
    return value


def _states(techniques: Mapping[str, bool]) -> dict[str, bool]:
    """The state of every registered Technique; ids absent from ``techniques`` take their default (28.2)."""
    if not isinstance(techniques, Mapping):
        raise TypeError(f"techniques must map Technique ids to bools, got {type(techniques).__name__}")
    unknown = [tid for tid in techniques if tid not in REGISTRY.techniques]
    if unknown:
        raise ValueError(f"unregistered Technique ids {sorted_codepoint(unknown)}")
    states: dict[str, bool] = {}
    for tid, spec in REGISTRY.techniques.items():
        state = techniques.get(tid, spec.default_enabled)
        if not isinstance(state, bool):
            raise TypeError(f"the state of {tid!r} must be a bool, got {type(state).__name__}")
        states[tid] = state
    return states


def mdl_posterior(lengths: Sequence[float]) -> list[float]:
    """The code-length posteriors ``2^-DL(c) / Σ 2^-DL(c')`` of description lengths in bits.

    Computed relative to the shortest length (log-sum-exp in base 2), so
    lengths of millions of bits neither overflow nor underflow the winner,
    and summed with ``math.fsum``, so the result does not depend on order.
    """
    if not lengths:
        raise ValueError("the posterior needs at least one description length")
    low = min(lengths)
    weights = [math.exp2(low - length) for length in lengths]  # each in [0, 1]; the shortest is 1
    total = math.fsum(weights)
    return [weight / total for weight in weights]


# ---------------------------------------------------------------------------
# Decision_Points and candidates
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class DecisionPoint:
    """A Decision_Point of a registered ``kind`` at ``path``.

    ``path`` is a Path or a Path string. When it is None, ``resolve`` uses the
    view's Path, or the root for a run-level decision without a view.
    """

    kind: str
    path: Path | None = None

    def __post_init__(self) -> None:
        kind = _name(self.kind, "Decision_Point kind")
        if kind not in REGISTRY.decision_points:
            raise ValueError(f"unregistered Decision_Point kind {kind!r}; registered: {', '.join(REGISTRY.kinds())}")
        path = self.path
        if isinstance(path, str):
            _set(self, "path", parse_path(path))
        elif path is not None:
            if not isinstance(path, (tuple, list)) or not all(isinstance(step, Step) for step in path):
                raise TypeError(f"a Decision_Point path must be a Path or a Path string, got {type(path).__name__}")
            _set(self, "path", tuple(path))


@dataclass(frozen=True, eq=False, slots=True, kw_only=True)
class Candidate:
    """One candidate outcome at a Decision_Point (see the module docstring).

    ``label`` defaults to ``outcome`` when that is a str and is required
    otherwise; it is the ``CandidateScore.outcome`` and what tie rules and
    Decision_Model classes match. ``order_key`` is a tuple whose values are
    comparable with those of the same Technique's other candidates.
    ``tie_ranks`` maps :data:`~jsinfer.config.registry.TIE_CRITERIA` to
    numbers (lower wins, ``inf`` allowed).
    """

    technique: str | None
    outcome: Any
    order_key: tuple[Any, ...] = ()
    node: SchemaNode | None = None
    label: str = ""
    rejects: View | None = None
    score: Score | None = None
    tie_ranks: Mapping[str, float] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if self.technique is not None:
            _name(self.technique, "Candidate technique")
        if not isinstance(self.order_key, tuple):
            raise TypeError(f"Candidate order_key must be a tuple, got {type(self.order_key).__name__}")
        if self.node is not None and not isinstance(self.node, SchemaNode):
            raise TypeError(f"Candidate node must be a SchemaNode or None, got {type(self.node).__name__}")
        label = self.label
        if label == "":
            if not isinstance(self.outcome, str):
                raise TypeError(
                    f"a Candidate whose outcome is a {type(self.outcome).__name__} needs a str label"
                )
            label = self.outcome
        _set(self, "label", _name(label, "Candidate label"))
        if self.rejects is not None and not isinstance(self.rejects, View):
            raise TypeError(f"Candidate rejects must be a View or None, got {type(self.rejects).__name__}")
        if self.score is not None and not isinstance(self.score, Score):
            raise TypeError(f"Candidate score must be a Score or None, got {type(self.score).__name__}")
        if not isinstance(self.tie_ranks, Mapping):
            raise TypeError(f"Candidate tie_ranks must be a mapping, got {type(self.tie_ranks).__name__}")
        ranks: dict[str, float] = {}
        for criterion in TIE_CRITERIA:
            if criterion not in self.tie_ranks:
                continue
            value = self.tie_ranks[criterion]
            if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, float, np.integer, np.floating)):
                raise TypeError(f"Candidate tie_ranks[{criterion!r}] must be a number, got {type(value).__name__}")
            if math.isnan(float(value)):
                raise ValueError(f"Candidate tie_ranks[{criterion!r}] must not be NaN")
            ranks[criterion] = float(value)
        extra = [key for key in self.tie_ranks if key not in ranks]
        if extra:
            raise ValueError(
                f"Candidate tie_ranks names unknown criteria {extra!r}; the criteria are {list(TIE_CRITERIA)}"
            )
        _set(self, "tie_ranks", MappingProxyType(ranks))

    @property
    def exceptional(self) -> bool:
        """Whether the candidate rejects rows, which would become Flagged_Items (17.2, 17.3)."""
        return self.rejects is not None and len(self.rejects) > 0

    def tie_rank(self, criterion: str) -> float:
        """The candidate's value for a tie criterion; ``inf`` (last) when it has none."""
        return self.tie_ranks.get(criterion, math.inf)


@dataclass(frozen=True, eq=False, slots=True)
class _Entry:
    """A validated candidate with its canonical sort key."""

    candidate: Candidate
    key: tuple[int, tuple[Any, ...]]


@dataclass(frozen=True, eq=False, slots=True)
class _Scored:
    entry: _Entry
    score: Score

    @property
    def candidate(self) -> Candidate:
        return self.entry.candidate

    def to_score(self) -> CandidateScore:
        """The 19.5 record: producing Technique, description length, and its breakdown."""
        s = self.score
        return CandidateScore(
            self.candidate.technique,
            self.candidate.label,
            s.total,
            model_bits=s.model_bits,
            data_bits=s.data_bits,
            exception_bits=s.exception_bits,
            n_rejected=s.n_rejected,
            n_scored=s.n_scored,
        )


def _position(items: Sequence[Any], item: Any) -> int:
    """The index of ``item`` in ``items`` by identity."""
    for index, candidate in enumerate(items):
        if candidate is item:
            return index
    raise ValueError("item is not in the sequence")  # pragma: no cover - internal invariant


def _mdl_winner(scored: Sequence[_Scored], tie_rule: TieRule | None) -> _Scored:
    """The lowest description length; ties by widening, the tie rule, model bits, then canonical order."""
    best = min(item.score.total for item in scored)
    tied = [item for item in scored if item.score.total == best]
    widening = [item for item in tied if not item.candidate.exceptional]
    if widening:
        tied = widening  # 17.2, 17.3: ties favor widening
    if tie_rule is not None:
        for favor in tie_rule.favors:
            if favor in TIE_CRITERIA:
                low = min(item.candidate.tie_rank(favor) for item in tied)
                tied = [item for item in tied if item.candidate.tie_rank(favor) == low]
            else:
                matching = [item for item in tied if item.candidate.label == favor]
                if matching:
                    tied = matching
    return min(tied, key=lambda item: (item.score.model_bits, item.entry.key))


# ---------------------------------------------------------------------------
# Decisions
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class Decision:
    """The outcome of one :meth:`DecisionEngine.resolve` call (see the module docstring).

    ``label`` and ``outcome`` are the selected candidate's, or the
    Fallback_Behavior's label for a fallback no candidate expresses
    (``candidate`` is then None). ``technique`` is the Provenance_Record's
    Technique. ``confidence`` is None for priority and fallback decisions.
    ``flagged`` holds the rows a winning exception-bearing candidate flagged.
    """

    kind: str
    path: Path
    method: ResolutionMethod
    label: str
    outcome: Any
    candidate: Candidate | None
    technique: str
    confidence: float | None
    resolution: Resolution
    cutoff: Cutoff | None = None
    fallback: FallbackInfo | None = None
    flagged: View | None = None
    details: Mapping[str, JSON] = field(default_factory=dict)

    @property
    def entry(self) -> DecisionEntry:
        """The Inference_Report entry of this decision (19.5, 20.7, 26.3)."""
        return DecisionEntry(
            self.kind, render_path(self.path), self.resolution, self.cutoff, self.fallback, dict(self.details)
        )

    def provenance(
        self,
        support: Support,
        *,
        confidence: float | None = None,
        sample: SampleInfo | None = None,
        details: Mapping[str, JSON] | None = None,
    ) -> ProvenanceRecord:
        """The Provenance_Record of a Constraint this decision produced, with its ``support``.

        Confidence is ``confidence`` when given (a kind-specific value), else
        the decision's (MDL posterior or calibrated probability), else the
        Laplace default of ``support``. ``details`` are added to the decision's.
        """
        value = confidence if confidence is not None else self.confidence
        if value is None:
            value = laplace_confidence(support)
        merged = {**self.details, **(details or {})}
        return ProvenanceRecord(
            self.technique,
            support,
            value,
            fallback=self.fallback,
            resolution=self.resolution,
            cutoff=self.cutoff,
            sample=sample,
            details=merged,
        )


@dataclass(frozen=True, eq=False, slots=True)
class _Context:
    """The per-call inputs every step needs."""

    spec: DecisionPointSpec
    path: Path
    view: View | None
    scorer: MDLScorer
    cutoff: Cutoff | None
    active: tuple[str, ...]


# ---------------------------------------------------------------------------
# The engine
# ---------------------------------------------------------------------------


class DecisionEngine:
    """Resolves Decision_Points for one run (see the module docstring).

    ``techniques`` maps Technique ids to their states; omitted ids take their
    registered default. ``bundle`` supplies Decision_Models and
    Learned_Thresholds (None reads as an empty bundle). ``report`` receives a
    DecisionEntry per resolution and a Substitution per missing model.
    ``scorer`` is the default MDL_Scorer.
    """

    __slots__ = ("_bundle", "_extra", "_models", "_report", "_requirements", "_scorer", "_states")

    def __init__(
        self,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        scorer: MDLScorer | None = None,
    ) -> None:
        self._states = _states({} if techniques is None else techniques)
        self._extra: dict[str, JSON] | None = None
        if report is not None and not isinstance(report, InferenceReport):
            raise TypeError(f"report must be an InferenceReport or None, got {type(report).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        self._bundle = bundle
        self._report = report
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._requirements = {(r.artifact, r.technique): r for r in required_models(self._states)}
        self._models: dict[str, tuple[str, Any] | None] = {}

    @classmethod
    def for_run(
        cls,
        config: RunConfiguration,
        bundle: ArtifactSource | None,
        report: InferenceReport | None,
        *,
        scorer: MDLScorer | None = None,
    ) -> DecisionEngine:
        """The engine of a run with a validated Run_Configuration."""
        return cls(config.techniques, bundle=bundle, report=report, scorer=scorer)

    @property
    def techniques(self) -> Mapping[str, bool]:
        """The state of every registered Technique, in registration order."""
        return MappingProxyType(self._states)

    def enabled(self, tid: str) -> bool:
        """Whether Technique ``tid`` is enabled."""
        try:
            return self._states[tid]
        except KeyError:
            raise KeyError(f"unregistered Technique {tid!r}") from None

    @property
    def bundle(self) -> ArtifactSource | None:
        """The Model_Bundle the engine reads (None reads as an empty bundle)."""
        return self._bundle

    def threshold(self, name: str) -> int | float | None:
        """The value of Learned_Threshold ``name`` in the Model_Bundle, or None when it holds no usable one.

        Techniques read their Learned_Thresholds here, so each value read is
        listed in the report with the bundle's ``bundle_id`` (20.5) when the
        bundle names one. None means the reading Technique applies its
        Fallback_Behavior (20.6), as the engine's eligibility check does.
        """
        if name not in THRESHOLDS:
            raise KeyError(f"unregistered Learned_Threshold {name!r}")
        value = None if self._bundle is None else self._bundle.threshold(name)
        if value is not None and self._report is not None:
            bundle_id = getattr(self._bundle, "bundle_id", None)
            if isinstance(bundle_id, str) and bundle_id:
                self._report.record_threshold(name, value, bundle_id)
        return value

    # -- resolution ---------------------------------------------------------

    def resolve(
        self,
        dp: DecisionPoint,
        candidates: Iterable[Candidate],
        view: View | None = None,
        features: Callable[[], np.ndarray] | None = None,
        *,
        mdl_by: str | None = None,
        cutoff: Cutoff | None = None,
        scorer: MDLScorer | None = None,
        details: Mapping[str, JSON] | None = None,
    ) -> Decision:
        """Resolve ``dp`` among ``candidates`` (see the module docstring for the order).

        ``details`` are added to the Decision's (and so to its report entry
        and the records it produces): what the caller knows about the point,
        for example the origin of a cross-Path grouping.

        ``view`` holds the Observations MDL scores candidates on (19.3); it
        may be None when every candidate carries a ``score`` or no MDL step
        runs. ``features`` computes the Decision_Model features, only when a
        model is used. ``mdl_by`` names a serving Technique that selects by
        MDL itself; ``cutoff`` is the cut-off a producing Technique applied,
        recorded for a priority resolution; ``scorer`` overrides the default
        MDL_Scorer (for example one rooted at a model with ``$defs``).
        """
        if not isinstance(dp, DecisionPoint):
            raise TypeError(f"dp must be a DecisionPoint, got {type(dp).__name__}")
        if view is not None and not isinstance(view, View):
            raise TypeError(f"view must be a View or None, got {type(view).__name__}")
        if cutoff is not None and not isinstance(cutoff, Cutoff):
            raise TypeError(f"cutoff must be a Cutoff or None, got {type(cutoff).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        self._extra = dict(details) if details else None
        try:
            return self._resolve(dp, candidates, view, features, mdl_by=mdl_by, cutoff=cutoff, scorer=scorer)
        finally:
            self._extra = None

    def _resolve(
        self,
        dp: DecisionPoint,
        candidates: Iterable[Candidate],
        view: View | None,
        features: Callable[[], np.ndarray] | None,
        *,
        mdl_by: str | None,
        cutoff: Cutoff | None,
        scorer: MDLScorer | None,
    ) -> Decision:
        spec = REGISTRY.decision_point(dp.kind)
        if mdl_by is not None and mdl_by not in spec.served_by:
            raise ValueError(f"mdl_by {mdl_by!r} does not serve Decision_Point {spec.kind!r}")
        path = dp.path if dp.path is not None else (view.path if view is not None else ROOT)
        ctx = _Context(
            spec=spec,
            path=path,
            view=view,
            scorer=scorer if scorer is not None else self._scorer,
            cutoff=cutoff,
            active=REGISTRY.active_servers(spec.kind, self._states),
        )
        ordered = self._ordered(spec, candidates)
        eligible = [entry for entry in ordered if self._eligible(entry.candidate, ctx)]
        widening = [entry for entry in eligible if not entry.candidate.exceptional]

        # 1. Decision_Model (22.12, 23.8, 23.9, 22.14).
        dm_fallback: FallbackInfo | None = None
        dm = DECISION_MODEL_TECHNIQUES.get(spec.kind)
        if dm is not None:
            reason = self._unavailable(dm, ctx)
            if reason is None and widening:
                chosen = self._by_model(ctx, dm, widening, features)
                return self._finish(self._exception_check(ctx, eligible, chosen))
            if reason == _GATED:  # enabled, but its serving gate is disabled (15.7)
                dm_fallback = FallbackInfo(spec.serves_only_with[dm], "disabled")
            elif reason is not None:
                dm_fallback = FallbackInfo(dm, reason)

        # 2. MDL (19.3, 19.4, 28.6), while a serving Technique can act; otherwise
        # the Fallback_Behavior resolves the point (28.4), whatever else was offered.
        resolver: str | None = None
        if mdl_by is not None and self._unavailable(mdl_by, ctx) is None:
            resolver = mdl_by
        elif self._states[MDL_SELECTION]:
            resolver = MDL_SELECTION
        serving = any(self._unavailable(tid, ctx) is None for tid in spec.served_by)
        if resolver is not None and eligible and serving:
            return self._finish(self._by_mdl(ctx, resolver, eligible, dm_fallback))

        # 3. Priority (19.7, 28.7).
        produced = [entry for entry in widening if entry.candidate.technique is not None]
        if produced:
            return self._finish(self._exception_check(ctx, eligible, self._by_priority(ctx, produced)))

        # 4. Fallback_Behavior (28.4), with no exception check.
        structural = [entry for entry in widening if entry.candidate.technique is None]
        return self._finish(self._by_fallback(ctx, structural))

    # -- steps --------------------------------------------------------------

    def _by_model(
        self, ctx: _Context, dm: str, widening: list[_Entry], features: Callable[[], np.ndarray] | None
    ) -> Decision:
        found = self._model(ctx.spec.kind, dm)
        assert found is not None  # _unavailable checked it
        artifact, model = found
        classes = tuple(model.classes)
        distinct = len(dict.fromkeys(classes)) == len(classes)
        if not classes or not all(isinstance(name, str) for name in classes) or not distinct:
            raise ValueError(f"Decision_Model {artifact!r} must name distinct str classes, got {classes!r}")
        labels = [entry.candidate.label for entry in widening]
        if sorted_codepoint(labels) != sorted_codepoint(classes):
            raise ValueError(
                f"the candidates at {ctx.spec.kind!r} ({render_path(ctx.path)}) are labeled {labels!r}, but "
                f"Decision_Model {artifact!r} predicts the classes {list(classes)!r}; offer one candidate per class"
            )
        if features is None:
            raise ValueError(
                f"Decision_Model {artifact!r} resolves {ctx.spec.kind!r}, which needs the features callable"
            )
        x = np.asarray(features(), dtype=np.float64)
        if x.ndim == 1:
            x = x.reshape(1, -1)
        if x.ndim != 2 or x.shape[0] != 1 or not np.all(np.isfinite(x)):
            raise ValueError(f"features must be one finite feature vector, got an array of shape {x.shape}")
        proba = np.asarray(model.predict_proba(x), dtype=np.float64)
        if proba.ndim == 2 and proba.shape[0] == 1:
            proba = proba[0]
        if proba.shape != (len(classes),) or not np.all(np.isfinite(proba)) or np.any((proba < 0) | (proba > 1)):
            raise ValueError(
                f"Decision_Model {artifact!r} must return one probability in [0, 1] per class, got {proba!r}"
            )
        by_class = {name: float(p) for name, p in zip(classes, proba)}
        probs = [by_class[label] for label in labels]
        best = max(probs)
        tied = [entry for entry, p in zip(widening, probs) if p == best]
        scores = tuple(
            CandidateScore(entry.candidate.technique, entry.candidate.label, p)
            for entry, p in zip(widening, probs)
        )
        details: dict[str, JSON] = {"decision_model": artifact}
        if len(tied) == 1:  # 20.7: the argmax of the calibrated probabilities
            winner = tied[0]
            cutoff = Cutoff(DECISION_MODEL_COMPARISON, scores=scores, artifact=artifact)
        else:  # equal probabilities go to the MDL_Scorer (22.12, 20.7)
            scored = self._score_all(ctx, tied)
            winner = _mdl_winner(scored, ctx.spec.tie_rule).entry
            cutoff = Cutoff(MDL_COMPARISON, scores=tuple(item.to_score() for item in scored))
            details["tie_break"] = MDL_COMPARISON
        return self._decision(
            ctx,
            "decision-model",
            winner.candidate,
            Resolution("decision-model", _position(widening, winner), scores),
            technique=dm,
            confidence=best,
            cutoff=cutoff,
            details=details,
        )

    def _by_mdl(self, ctx: _Context, resolver: str, pool: list[_Entry], fallback: FallbackInfo | None) -> Decision:
        scored = self._score_all(ctx, pool)
        return self._mdl_decision(ctx, scored, _mdl_winner(scored, ctx.spec.tie_rule), resolver, fallback, {})

    def _mdl_decision(
        self,
        ctx: _Context,
        scored: list[_Scored],
        winner: _Scored,
        resolver: str,
        fallback: FallbackInfo | None,
        details: Mapping[str, JSON],
    ) -> Decision:
        """An MDL decision for ``winner`` among ``scored``; a winning exception-bearing candidate flags its rows."""
        index = _position(scored, winner)
        posterior = mdl_posterior([item.score.total for item in scored])
        scores = tuple(item.to_score() for item in scored)
        candidate = winner.candidate
        recorded = dict(details)
        flagged: View | None = None
        if candidate.exceptional:
            assert candidate.rejects is not None
            flagged = candidate.rejects
            flagged.store.flags.flag_view(flagged)  # 17.2, 17.3: the rejected rows become Flagged_Items
            recorded["flagged"] = len(flagged)
        return self._decision(
            ctx,
            "mdl",
            candidate,
            Resolution("mdl", index, scores),
            technique=candidate.technique or resolver,
            confidence=posterior[index],
            cutoff=Cutoff(MDL_COMPARISON, scores=scores),
            fallback=fallback,
            flagged=flagged,
            details=recorded,
        )

    def _exception_check(self, ctx: _Context, eligible: list[_Entry], chosen: Decision) -> Decision:
        """Compare a Decision_Model or priority winner in bits with the eligible exception-bearing candidates.

        The best exception-bearing candidate replaces the widening winner only on
        a strictly smaller description length (17.2, 17.3); see the module docstring.
        """
        if not any(entry.candidate.exceptional for entry in eligible):
            return chosen
        pool = [entry for entry in eligible if entry.candidate is chosen.candidate or entry.candidate.exceptional]
        scored = self._score_all(ctx, pool)
        winner = _mdl_winner(scored, ctx.spec.tie_rule)
        if not winner.candidate.exceptional:  # W survives: ties go to widening
            compared = Resolution("mdl", _position(scored, winner), tuple(item.to_score() for item in scored))
            return replace(chosen, details={**chosen.details, "exception_check": compared.to_json()})
        details = {**chosen.details, "replaced": chosen.method}
        return self._mdl_decision(ctx, scored, winner, MDL_EXCEPTIONS, chosen.fallback, details)

    def _by_priority(self, ctx: _Context, produced: list[_Entry]) -> Decision:
        winner = produced[0].candidate
        assert winner.technique is not None
        scores = tuple(CandidateScore(entry.candidate.technique, entry.candidate.label, None) for entry in produced)
        # 19.7: with two or more candidates, priority is the Fallback_Behavior of the disabled MDL selection.
        fallback = FallbackInfo(MDL_SELECTION, "disabled") if len(produced) >= 2 else None
        return self._decision(
            ctx,
            "priority",
            winner,
            Resolution("priority", 0, scores),
            technique=winner.technique,
            confidence=None,
            cutoff=ctx.cutoff,
            fallback=fallback,
        )

    def _by_fallback(self, ctx: _Context, structural: list[_Entry]) -> Decision:
        spec = ctx.spec
        info = self._fallback_info(ctx)
        label = spec.fallback(ctx.view)
        chosen: _Entry | None = None
        if label == OUTCOME_CANDIDATE_ORDER and structural:
            chosen = structural[0]
        else:
            chosen = next((entry for entry in structural if entry.candidate.label == label), None)
        scores = [CandidateScore(None, entry.candidate.label, None) for entry in structural]
        if chosen is None:
            scores.insert(0, CandidateScore(None, label, None))
            selected = 0
        else:
            selected = _position(structural, chosen)
        candidate = None if chosen is None else chosen.candidate
        return Decision(
            kind=spec.kind,
            path=ctx.path,
            method="fallback",
            label=label if candidate is None else candidate.label,
            outcome=label if candidate is None else candidate.outcome,
            candidate=candidate,
            technique=fallback_technique(spec.kind),
            confidence=None,
            resolution=Resolution("fallback", selected, tuple(scores)),
            fallback=info,
            details={"fallback_behavior": spec.fallback_doc},
        )

    # -- helpers ------------------------------------------------------------

    def _decision(
        self,
        ctx: _Context,
        method: ResolutionMethod,
        candidate: Candidate,
        resolution: Resolution,
        *,
        technique: str,
        confidence: float | None,
        cutoff: Cutoff | None = None,
        fallback: FallbackInfo | None = None,
        flagged: View | None = None,
        details: Mapping[str, JSON] | None = None,
    ) -> Decision:
        return Decision(
            kind=ctx.spec.kind,
            path=ctx.path,
            method=method,
            label=candidate.label,
            outcome=candidate.outcome,
            candidate=candidate,
            technique=technique,
            confidence=confidence,
            resolution=resolution,
            cutoff=cutoff,
            fallback=fallback,
            flagged=flagged,
            details=dict(details or {}),
        )

    def _finish(self, decision: Decision) -> Decision:
        if self._extra:
            decision = replace(decision, details={**decision.details, **self._extra})
        if self._report is not None:
            self._report.record_decision(decision.entry)
        return decision

    def _ordered(self, spec: DecisionPointSpec, candidates: Iterable[Candidate]) -> list[_Entry]:
        """Validate the candidates and sort them into canonical order."""
        if isinstance(candidates, (str, bytes)) or not isinstance(candidates, Iterable):
            raise TypeError(f"candidates must be an iterable of Candidate, got {type(candidates).__name__}")
        entries: list[_Entry] = []
        last_rank = len(spec.served_by) + 1
        for candidate in candidates:
            if not isinstance(candidate, Candidate):
                raise TypeError(f"candidates must be Candidate instances, got {type(candidate).__name__}")
            tid = candidate.technique
            if tid is None:
                rank = 0
            elif tid in spec.served_by:
                rank = spec.rank(tid)
            elif tid == MDL_EXCEPTIONS and candidate.exceptional:
                rank = last_rank
            elif tid not in REGISTRY.techniques:
                raise ValueError(f"candidate Technique {tid!r} is not registered")
            else:
                raise ValueError(
                    f"candidate Technique {tid!r} does not serve Decision_Point {spec.kind!r} "
                    f"(served by {', '.join(spec.served_by)})"
                )
            entries.append(_Entry(candidate, (rank, candidate.order_key)))
        try:
            entries.sort(key=lambda entry: entry.key)
        except TypeError as error:
            raise TypeError(f"candidate order keys of one Technique must be mutually comparable: {error}") from None
        for before, after in zip(entries, entries[1:]):
            if before.key == after.key and before.candidate.technique == after.candidate.technique:
                raise ValueError(
                    f"two candidates of {before.candidate.technique!r} share the order key {before.key[1]!r}; "
                    "the documented candidate order must be total (19.4)"
                )
        return entries

    def _eligible(self, candidate: Candidate, ctx: _Context) -> bool:
        """Whether the candidate's Technique is in effect and usable (28.11, 20.6, 17.2)."""
        if candidate.exceptional and not self._states[MDL_EXCEPTIONS]:
            return False
        tid = candidate.technique
        if tid is None or tid not in ctx.spec.served_by:
            return True  # Technique-less, or robust.mdl_exceptions (its state is checked above)
        return self._unavailable(tid, ctx) is None

    def _unavailable(self, tid: str, ctx: _Context) -> _Unavailable | None:
        """Why serving Technique ``tid`` cannot act at this kind, or None when it can."""
        if not self._states[tid]:
            return "disabled"
        if tid not in ctx.active:
            return _GATED
        kind = ctx.spec.kind
        if DECISION_MODEL_TECHNIQUES.get(kind) == tid and self._model(kind, tid) is None:
            return "missing-model"
        for name in REGISTRY.technique(tid).thresholds:
            spec = THRESHOLDS[name]
            if spec.decision_point == kind and (self._bundle is None or self._bundle.threshold(name) is None):
                return "missing-threshold"
        return None

    def _fallback_info(self, ctx: _Context) -> FallbackInfo:
        """The Technique whose Fallback_Behavior applies, and why (26.10, 28.12)."""
        named: FallbackInfo | None = None
        gated: str | None = None  # the disabled gate of the first gated server (15.7)
        usable: list[str] = []
        for tid in ctx.spec.served_by:
            reason = self._unavailable(tid, ctx)
            if reason is None:
                usable.append(tid)
            elif reason == _GATED:
                gated = gated or ctx.spec.serves_only_with[tid]
            elif named is None:
                named = FallbackInfo(tid, reason)
        if usable:
            raise ValueError(
                f"no eligible candidate at {ctx.spec.kind!r} ({render_path(ctx.path)}) although "
                f"{', '.join(usable)} is in effect; offer its candidates"
            )
        if named is None:
            assert gated is not None  # every kind has a serving Technique
            named = FallbackInfo(gated, "disabled")
        return named

    def _model(self, kind: str, dm: str) -> tuple[str, Any] | None:
        """The Decision_Model of ``kind`` as (artifact, model): weak when enabled and present, else synthetic."""
        if kind in self._models:
            return self._models[kind]
        found: tuple[str, Any] | None = None
        sources: list[tuple[Literal["synthetic", "weak"], str]] = []
        if self._states[WEAK_SUPERVISION]:
            sources.append(("weak", WEAK_SUPERVISION))
        sources.append(("synthetic", dm))
        for source, tid in sources:
            artifact = decision_model_artifact(kind, source)
            model = None if self._bundle is None else self._bundle.model(artifact)
            if model is not None:
                found = (artifact, model)
                break
            self._substitute(artifact, tid)
        self._models[kind] = found
        return found

    def _substitute(self, artifact: str, tid: str) -> None:
        """Record the substitution of a missing Decision_Model (22.14, 23.9)."""
        if self._report is None:
            return
        listed = [
            sub
            for sub in getattr(self._bundle, "substitutions", ())
            if isinstance(sub, Substitution) and sub.artifact == artifact and sub.fallback.technique == tid
        ]
        if not listed:
            applied = self._requirements[(artifact, tid)].applied
            listed = [Substitution(artifact, FallbackInfo(tid, "missing-model"), "missing", applied)]
        for sub in listed:
            self._report.record_substitution(sub)

    def _score_all(self, ctx: _Context, entries: Sequence[_Entry]) -> list[_Scored]:
        """MDL scores of the entries on the Decision_Point's view (19.3)."""
        scored: list[_Scored] = []
        for entry in entries:
            candidate = entry.candidate
            score = candidate.score
            if score is None:
                if candidate.node is None or ctx.view is None:
                    raise ValueError(
                        f"MDL scoring of {candidate.label!r} at {ctx.spec.kind!r} needs the candidate's node and "
                        "the Decision_Point's view, or a precomputed score"
                    )
                score = ctx.scorer.score(candidate.node, ctx.view)
            scored.append(_Scored(entry, score))
        return scored
