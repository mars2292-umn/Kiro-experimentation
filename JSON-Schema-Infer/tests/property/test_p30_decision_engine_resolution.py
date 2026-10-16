"""Property 30: DecisionEngine resolution order (tasks 11.2 and 11.3).

For any registered Decision_Point kind, candidate list with forced ties,
stub Decision_Models, Model_Bundle thresholds, and Technique states,
``DecisionEngine.resolve`` agrees with an independent reference model of the
resolution order. The reference reads only registry data (serving
Techniques in priority order, serving gates, tie rules, Fallback_Behaviors,
and the Learned_Thresholds each Technique reads at the kind).

Scenarios
    Technique states are drawn for every serving Technique, its gates,
    ``decide.mdl``, ``decide.weak_supervision``, and
    ``robust.mdl_exceptions``, independently of the ``implemented`` flag
    (the engine takes states directly). Each Learned_Threshold of the kind is
    present or missing. At a Judgment_Call kind, ``dm.<kind>.synthetic`` and
    ``dm.<kind>.weak`` are each present or missing, with classes in either
    column order and calibrated probabilities ``(p, 1 - p)`` that are often
    equal. The candidates are one per outcome class at a Judgment_Call kind
    (Technique-less, or ``values.enum_estimation`` at Closed-or-Open-Enum);
    elsewhere one to three per producing serving Technique plus
    Technique-less structural alternatives; and at any kind up to two
    exception-bearing candidates of ``robust.mdl_exceptions`` or a producing
    Technique. Scores are precomputed in half bits from a small range, and
    each candidate either takes a shared description length (a forced tie)
    or draws its own, so description-length ties, model-bit ties, and
    tie-rule ties are frequent; labels include the tie
    rule's favored outcomes and the Fallback_Behavior's outcome; tie ranks
    are drawn for every criterion. The caller may name an MDL-selecting
    serving Technique (``mdl_by``) and pass a cut-off.

Reference resolution order
    1. Decision_Model (22.12, 23.8, 23.9): at a Judgment_Call kind whose
       Decision_Model Technique is enabled with its gate (15.7) and has a
       model (weak when weak supervision is enabled and present, else
       synthetic), the class with the higher calibrated probability wins,
       with a "Decision_Model comparison" cut-off naming the artifact (20.7);
       equal probabilities go to the MDL rule among the tied classes, with an
       "MDL comparison" cut-off.
    2. MDL (5.3, 6.2, 7.8, 9.4, 10.3, 13.8, 14.2, 19.3, 19.4, 22.13, 24.7,
       28.6): when ``decide.mdl`` is enabled or ``mdl_by`` can act, and some
       serving Technique can act (28.4), every eligible candidate is ranked
       by description length, then widening over exception-bearing (17.2,
       17.3), then the kind's mandated tie rule favor by favor, then fewer
       model bits, then registry priority, then the documented candidate
       order. This is one lexicographic key rather than the engine's
       successive filters.
    3. Priority (19.7, 28.7): otherwise the first produced candidate by
       priority, then candidate order; a Fallback_Behavior of the disabled
       ``decide.mdl`` when there are two or more.
    4. Fallback_Behavior (28.4): otherwise the kind's constant outcome,
       naming the first serving Technique that is disabled, missing its
       model, or missing a Learned_Threshold.

    A candidate is eligible when its Technique can act (enabled, gate
    enabled, Learned_Thresholds present) or it has none, and, when it rejects
    rows, only while ``robust.mdl_exceptions`` is enabled; the Decision_Model
    and priority steps never select an exception-bearing candidate.

    Exception check (17.2, 17.3): after step 1 or 3 picks a widening winner
    W, if eligible exception-bearing candidates exist, W and those
    candidates are ranked by the MDL key above, so one replaces W only on a
    strictly smaller description length. A replacement is an MDL decision
    over that set (posterior, "MDL comparison" cut-off, its rows flagged)
    that keeps the earlier step's fallback indicator and details and names
    the replaced method; otherwise the earlier decision stands with the
    comparison recorded in ``details["exception_check"]``. Step 4 runs no
    check.

Checks, for the drawn input order and for the identity order:

- the method, the selected candidate (by identity), label, outcome, and the
  Provenance_Record Technique;
- the Resolution: every considered candidate in canonical order with its
  score (description length with the 19.5 breakdown, calibrated
  probability, or none) and the selected index (19.5, 26.3);
- the Confidence: the code-length posterior ``2^-DL / Σ 2^-DL`` for MDL, the
  calibrated probability for a Decision_Model, none for priority and
  fallback; and ``Decision.provenance`` uses it, or the Laplace default;
- the cut-off (a Decision_Model comparison with its artifact, an MDL
  comparison, the caller's, or none), the fallback indicator, and the
  details (the Decision_Model artifact consulted, an MDL tie-break, the
  flagged count, the replaced method or the exception check's comparison,
  or the Fallback_Behavior);
- a winning exception-bearing candidate flags exactly its rows, and no
  other resolution flags anything;
- features are computed exactly when a Decision_Model is consulted;
- the report holds the decision's entry and a Substitution for each missing
  Decision_Model the engine consulted;
- no record names a disabled Technique as a candidate's or the decision's
  Technique (28.11);
- the Inference_Report entries of both input orders are byte-identical.

**Validates: Requirements 5.3, 6.2, 7.8, 9.4, 10.3, 13.8, 14.2, 17.2, 17.3, 19.3, 19.4, 19.5, 19.7, 22.12, 22.13, 23.8, 23.9, 24.7, 26.3, 28.6, 28.7**
"""

# Feature: json-schema-inference, Property 30: DecisionEngine resolution order

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field, replace
from typing import Any, Literal, TypeAlias, cast

import numpy as np
from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.config.registry import OUTCOME_CANDIDATE_ORDER, REGISTRY, TIE_CRITERIA, DecisionPointSpec
from jsinfer.config.thresholds import THRESHOLDS
from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import ObservationStore, build_store
from jsinfer.core.paths import ROOT, Path, parse_path
from jsinfer.core.provenance import (
    DECISION_MODEL_COMPARISON,
    MDL_COMPARISON,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    FallbackReason,
    Resolution,
    Support,
    laplace_confidence,
)
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.learn.bundle import DECISION_MODEL_TECHNIQUES
from jsinfer.mdl.scorer import Score

MDL = "decide.mdl"
WEAK = "decide.weak_supervision"
EXCEPTIONS = "robust.mdl_exceptions"
DM_TECHNIQUES = frozenset(DECISION_MODEL_TECHNIQUES.values())
# Why a serving Technique cannot act: a fallback reason, or "gated" (enabled, but its serving gate is not).
Reason: TypeAlias = FallbackReason | Literal["gated"]
# Every candidate is scored on the same N_DOCS scalar documents (one Observation each).
N_DOCS = 8
PROBABILITIES = (0.0, 0.2, 0.5, 0.5, 0.8, 1.0)
CALLER_CUTOFF = Cutoff("semantic.min_probability", value=0.5)
FEATURES = np.array([0.25, 1.0, 4.0])
SUPPORT = Support(3, 4)
POSTERIOR_REL_TOL = 1e-12
# Decision_Point Paths; None resolves to the root, as there is no view.
PATHS: tuple[Path | None, ...] = (None, parse_path("$.x"), parse_path("$.users[*].id"))


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Spec:
    """A candidate as plain data; bits are counted in halves so every sum is exact."""

    technique: str | None
    label: str
    order_key: tuple[int, ...]
    model_halves: int
    data_halves: int
    exception_halves: int = 0
    rejected: tuple[int, ...] = ()  # document indices; non-empty for an exception-bearing candidate
    tie_ranks: tuple[tuple[str, float], ...] = ()

    @property
    def model_bits(self) -> float:
        return self.model_halves / 2

    @property
    def data_bits(self) -> float:
        return self.data_halves / 2

    @property
    def exception_bits(self) -> float:
        return self.exception_halves / 2

    @property
    def total(self) -> float:
        return (self.model_halves + self.data_halves) / 2

    @property
    def exceptional(self) -> bool:
        return bool(self.rejected)

    def tie_rank(self, criterion: str) -> float:
        return dict(self.tie_ranks).get(criterion, math.inf)


@dataclass(frozen=True)
class Scenario:
    kind: str
    path: Path | None
    states: Mapping[str, bool]
    models: Mapping[str, tuple[tuple[str, ...], tuple[float, ...]]]  # artifact -> (classes, probabilities)
    thresholds: Mapping[str, float]
    specs: tuple[Spec, ...]
    mdl_by: str | None
    cutoff: Cutoff | None
    order: tuple[int, ...]  # the input order of the candidates


def thresholds_at(kind: str) -> list[str]:
    """The Learned_Thresholds whose Decision_Point is ``kind``."""
    return sorted(name for name, spec in THRESHOLDS.items() if spec.decision_point == kind)


def thresholds_read(tid: str, kind: str) -> list[str]:
    """The Learned_Thresholds Technique ``tid`` reads at ``kind``."""
    return [name for name in thresholds_at(kind) if tid in THRESHOLDS[name].techniques]


def producers(dp: DecisionPointSpec) -> list[str]:
    """The serving Techniques that produce candidates (not a Decision_Model, not generic MDL selection)."""
    return [tid for tid in dp.served_by if tid != MDL and tid not in DM_TECHNIQUES]


def label_pool(dp: DecisionPointSpec) -> tuple[str, ...]:
    """Candidate labels: two plain ones, the Fallback_Behavior's outcome, and the tie rule's favored outcomes."""
    labels = {"a", "b"}
    if dp.fallback.outcome != OUTCOME_CANDIDATE_ORDER:
        labels.add(dp.fallback.outcome)
    if dp.tie_rule is not None:
        labels.update(favor for favor in dp.tie_rule.favors if favor not in TIE_CRITERIA)
    return tuple(sorted(labels))


# Judgment_Call kinds are listed twice, so Decision_Models resolve often enough.
KINDS = (*REGISTRY.kinds(), *JUDGMENT_OUTCOMES)
# Enabled three times in four (shrinking toward disabled), so most scenarios get past the fallback.
ENABLED = st.integers(0, 3).map(bool)


@st.composite
def scenarios(draw: st.DrawFn) -> Scenario:
    kind = draw(st.sampled_from(KINDS), label="kind")
    dp = REGISTRY.decision_point(kind)
    toggled = sorted({*dp.served_by, *dp.serves_only_with.values(), MDL, WEAK, EXCEPTIONS})
    states = {tid: draw(ENABLED, label=tid) for tid in toggled}
    thresholds = {name: 1.0 for name in thresholds_at(kind) if draw(st.integers(0, 3), label=f"{name} present") > 0}

    models: dict[str, tuple[tuple[str, ...], tuple[float, ...]]] = {}
    outcomes = JUDGMENT_OUTCOMES.get(kind)
    if outcomes is not None:
        for source in ("synthetic", "weak"):
            if draw(st.integers(0, 2), label=f"{source} model present") > 0:
                classes = tuple(draw(st.permutations(outcomes), label=f"{source} classes"))
                p = draw(st.sampled_from(PROBABILITIES), label=f"{source} probability")
                models[f"dm.{kind}.{source}"] = (classes, (p, 1.0 - p))

    makers = producers(dp)
    drafts: list[tuple[str | None, str, bool]] = []  # (technique, label, exception-bearing)
    if outcomes is not None:
        # One candidate per class: Technique-less, or values.enum_estimation's at closed_or_open_enum.
        owner = makers[0] if makers else None
        drafts.extend((owner, label, False) for label in outcomes)
        labels: tuple[str, ...] = (*outcomes, "a")
    else:
        labels = label_pool(dp)
        for maker in makers:
            for _ in range(draw(st.integers(1, 3), label=f"{maker} candidates")):
                drafts.append((maker, draw(st.sampled_from(labels), label="label"), False))
        for _ in range(draw(st.integers(0 if makers else 1, 2), label="structural candidates")):
            drafts.append((None, draw(st.sampled_from(labels), label="label"), False))
    for _ in range(draw(st.integers(0, 2), label="exception-bearing candidates")):
        flagger = draw(st.sampled_from([EXCEPTIONS, *makers]), label="exception technique")
        drafts.append((flagger, draw(st.sampled_from(labels), label="label"), True))

    # The documented candidate order: a drawn permutation within each Technique.
    members: dict[str | None, list[int]] = {}
    for index, draft in enumerate(drafts):
        members.setdefault(draft[0], []).append(index)
    order_keys: dict[int, tuple[int, ...]] = {}
    for owner_id, indices in members.items():
        ranks = draw(st.permutations(range(len(indices))), label=f"{owner_id} candidate order")
        order_keys.update((index, (rank,)) for index, rank in zip(indices, ranks))

    # Forced ties: each candidate takes the shared description length or draws its own.
    tied_total = draw(st.integers(0, 4), label="shared description length (half bits)")
    specs: list[Spec] = []
    for index, (technique, label, exceptional) in enumerate(drafts):
        if draw(st.booleans(), label="own description length"):
            total = draw(st.integers(0, 6), label="description length (half bits)")
        else:
            total = tied_total
        model = draw(st.integers(0, total), label="model bits (half bits)")
        rejected: tuple[int, ...] = ()
        exception = 0
        if exceptional:
            rejected = tuple(sorted(draw(st.sets(st.integers(0, N_DOCS - 1), min_size=1, max_size=3))))
            exception = draw(st.integers(0, total - model), label="exception bits (half bits)")
        tie_ranks = tuple(
            (criterion, float(draw(st.integers(0, 2), label=criterion)))
            for criterion in TIE_CRITERIA
            if draw(st.booleans(), label=f"has {criterion}")
        )
        specs.append(Spec(technique, label, order_keys[index], model, total - model, exception, rejected, tie_ranks))

    mdl_by = draw(st.none() | st.sampled_from(makers), label="mdl_by") if makers else None
    cutoff = draw(st.sampled_from([None, CALLER_CUTOFF]), label="caller cut-off")
    path = draw(st.sampled_from(PATHS), label="path")
    order = tuple(draw(st.permutations(range(len(specs))), label="input order"))
    return Scenario(kind, path, states, models, thresholds, tuple(specs), mdl_by, cutoff, order)


# ---------------------------------------------------------------------------
# Stubs and materialization
# ---------------------------------------------------------------------------


class StubModel:
    """A Decision_Model with fixed calibrated probabilities."""

    def __init__(self, classes: tuple[str, ...], proba: tuple[float, ...]) -> None:
        self.classes = classes
        self.proba = np.asarray(proba, dtype=np.float64)

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        assert x.shape == (1, FEATURES.size) and np.array_equal(x[0], FEATURES), f"features {x!r}"
        return self.proba.reshape(1, -1)


class StubBundle:
    """Models and Learned_Thresholds by name, like ResolvedBundle.model and .threshold."""

    def __init__(self, scenario: Scenario) -> None:
        self.models = {name: StubModel(*entry) for name, entry in scenario.models.items()}
        self.thresholds = dict(scenario.thresholds)

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


def materialize(specs: Sequence[Spec]) -> tuple[list[Candidate], ObservationStore]:
    """Fresh Candidates (in spec order) whose rejects are rows of a fresh store."""
    store = build_store(list(range(N_DOCS)))
    made = [
        Candidate(
            technique=spec.technique,
            outcome=("outcome", index),
            label=spec.label,
            order_key=spec.order_key,
            rejects=View.root(store, spec.rejected) if spec.rejected else None,
            score=Score(spec.model_bits, spec.data_bits, spec.exception_bits, len(spec.rejected), N_DOCS),
            tie_ranks=dict(spec.tie_ranks),
        )
        for index, spec in enumerate(specs)
    ]
    return made, store


# ---------------------------------------------------------------------------
# The reference model
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Expected:
    method: str
    selected: int | None  # the spec index of the selected candidate; None when no candidate expresses a fallback
    label: str
    technique: str
    confidence: float | None
    resolution: Resolution
    cutoff: Cutoff | None = None
    fallback: FallbackInfo | None = None
    details: Mapping[str, Any] = field(default_factory=dict)
    decided_by: str = ""


class Reference:
    """The resolution order of Property 30, computed from the scenario and registry data alone."""

    def __init__(self, scenario: Scenario) -> None:
        self.sc = scenario
        self.specs = scenario.specs
        self.kind = scenario.kind
        self.dp = REGISTRY.decision_point(scenario.kind)
        self.favors = self.dp.tie_rule.favors if self.dp.tie_rule is not None else ()

    # -- Technique states ----------------------------------------------------

    def state(self, tid: str) -> bool:
        return self.sc.states.get(tid, REGISTRY.technique(tid).default_enabled)

    def model(self) -> str | None:
        """The Decision_Model artifact in use: weak when enabled and present, else synthetic, else none."""
        weak, synthetic = f"dm.{self.kind}.weak", f"dm.{self.kind}.synthetic"
        if self.state(WEAK) and weak in self.sc.models:
            return weak
        return synthetic if synthetic in self.sc.models else None

    def reason(self, tid: str) -> Reason | None:
        """Why serving Technique ``tid`` cannot act at this kind ("gated" for a disabled gate), or None."""
        if not self.state(tid):
            return "disabled"
        gate = self.dp.serves_only_with.get(tid)
        if gate is not None and not self.state(gate):
            return "gated"
        if DECISION_MODEL_TECHNIQUES.get(self.kind) == tid and self.model() is None:
            return "missing-model"
        if any(name not in self.sc.thresholds for name in thresholds_read(tid, self.kind)):
            return "missing-threshold"
        return None

    def eligible(self, spec: Spec) -> bool:
        if spec.exceptional and not self.state(EXCEPTIONS):
            return False
        if spec.technique is None or spec.technique not in self.dp.served_by:
            return True  # Technique-less, or robust.mdl_exceptions where it does not serve the kind
        return self.reason(spec.technique) is None

    def substitutions(self) -> frozenset[tuple[str, FallbackInfo]]:
        """The missing Decision_Models the engine consults: only while the Decision_Model Technique is in effect."""
        dm = DECISION_MODEL_TECHNIQUES.get(self.kind)
        if dm is None or self.reason(dm) not in (None, "missing-model"):
            return frozenset()
        weak, synthetic = f"dm.{self.kind}.weak", f"dm.{self.kind}.synthetic"
        missing: set[tuple[str, FallbackInfo]] = set()
        if self.state(WEAK):
            if weak in self.sc.models:
                return frozenset()
            missing.add((weak, FallbackInfo(WEAK, "missing-model")))
        if synthetic not in self.sc.models:
            missing.add((synthetic, FallbackInfo(dm, "missing-model")))
        return frozenset(missing)

    # -- orders ----------------------------------------------------------------

    def rank(self, spec: Spec) -> int:
        """Technique-less first, then the serving Techniques by priority, then robust.mdl_exceptions."""
        if spec.technique is None:
            return 0
        if spec.technique in self.dp.served_by:
            return self.dp.served_by.index(spec.technique) + 1
        return len(self.dp.served_by) + 1

    def canonical(self, indices: Sequence[int]) -> list[int]:
        return sorted(indices, key=lambda i: (self.rank(self.specs[i]), self.specs[i].order_key))

    def mdl_key(self, index: int) -> tuple[Any, ...]:
        """DL, widening first, the tie rule favor by favor, model bits, priority, candidate order."""
        spec = self.specs[index]
        favors = [
            spec.tie_rank(favor) if favor in TIE_CRITERIA else (0 if spec.label == favor else 1)
            for favor in self.favors
        ]
        return (spec.total, spec.exceptional, *favors, spec.model_bits, self.rank(spec), spec.order_key)

    def mdl_names(self) -> list[str]:
        return ["description length", "widening", *(f"tie rule: {f}" for f in self.favors),
                "model bits", "priority", "candidate order"]

    def decided_by(self, pool: Sequence[int], winner: int) -> str:
        """The first component of the MDL key that separates the winner from every rival."""
        rivals = [i for i in pool if i != winner]
        if not rivals:
            return "single candidate"
        key = self.mdl_key(winner)
        for depth, name in enumerate(self.mdl_names(), start=1):
            rivals = [i for i in rivals if self.mdl_key(i)[:depth] == key[:depth]]
            if not rivals:
                return name
        raise AssertionError("two candidates share a Technique and an order key")

    def mdl_score(self, index: int) -> CandidateScore:
        spec = self.specs[index]
        return CandidateScore(
            spec.technique,
            spec.label,
            spec.total,
            model_bits=spec.model_bits,
            data_bits=spec.data_bits,
            exception_bits=spec.exception_bits,
            n_rejected=len(spec.rejected),
            n_scored=N_DOCS,
        )

    # -- resolution --------------------------------------------------------------

    def expect(self) -> Expected:
        order = self.canonical(range(len(self.specs)))
        eligible = [i for i in order if self.eligible(self.specs[i])]
        widening = [i for i in eligible if not self.specs[i].exceptional]

        dm_fallback: FallbackInfo | None = None
        dm = DECISION_MODEL_TECHNIQUES.get(self.kind)
        if dm is not None:
            why = self.reason(dm)
            if why is None:
                assert len(widening) == 2, "a usable Decision_Model sees one widening candidate per class"
                return self.exception_check(self.by_model(dm, widening), eligible)
            # 22.13, 22.14: the Decision_Model's Fallback_Behavior is the next step.
            dm_fallback = self.fallback_of(dm, why)

        can_act = [tid for tid in self.dp.served_by if self.reason(tid) is None]
        resolver = None
        if self.sc.mdl_by is not None and self.reason(self.sc.mdl_by) is None:
            resolver = self.sc.mdl_by
        elif self.state(MDL):
            resolver = MDL
        if resolver is not None and eligible and can_act:
            return self.by_mdl(resolver, eligible, dm_fallback)

        produced = [i for i in widening if self.specs[i].technique is not None]
        if produced:
            return self.exception_check(self.by_priority(produced), eligible)
        # No exception check at the constant Fallback_Behavior (28.4).
        assert not can_act, f"{can_act} can act, so the scenario must offer their candidates"
        return self.by_fallback(widening)

    def exception_check(self, earlier: Expected, eligible: list[int]) -> Expected:
        """17.2, 17.3: an exception-bearing candidate replaces a Decision_Model or priority winner on fewer bits."""
        if not any(self.specs[i].exceptional for i in eligible):
            return earlier
        assert earlier.selected is not None and not self.specs[earlier.selected].exceptional
        pool = [i for i in eligible if i == earlier.selected or self.specs[i].exceptional]  # canonical order
        winner = min(pool, key=self.mdl_key)  # the widening component sends ties to W
        scores = tuple(self.mdl_score(i) for i in pool)
        decided = f"{earlier.decided_by}; exception check by {self.decided_by(pool, winner)}"
        if winner == earlier.selected:
            compared = Resolution("mdl", pool.index(winner), scores)
            return replace(
                earlier,
                details={**earlier.details, "exception_check": compared.to_json()},
                decided_by=f"{decided}: kept",
            )
        spec = self.specs[winner]
        lengths = [self.specs[i].total for i in pool]
        weights = [2.0 ** (min(lengths) - length) for length in lengths]
        return Expected(
            "mdl",
            winner,
            spec.label,
            spec.technique or EXCEPTIONS,
            weights[pool.index(winner)] / math.fsum(weights),
            Resolution("mdl", pool.index(winner), scores),
            cutoff=Cutoff(MDL_COMPARISON, scores=scores),
            fallback=earlier.fallback,  # the earlier step's fallback indicator is kept
            details={**earlier.details, "replaced": earlier.method, "flagged": len(spec.rejected)},
            decided_by=f"{decided}: replaced {earlier.method}",
        )

    def by_model(self, dm: str, widening: list[int]) -> Expected:
        artifact = self.model()
        assert artifact is not None
        classes, proba = self.sc.models[artifact]
        probability = dict(zip(classes, proba))
        probs = [probability[self.specs[i].label] for i in widening]
        best = max(probs)
        tied = [i for i, p in zip(widening, probs) if p == best]
        listed = tuple(CandidateScore(self.specs[i].technique, self.specs[i].label, p) for i, p in zip(widening, probs))
        details: dict[str, Any] = {"decision_model": artifact}
        # 20.7: the argmax is a Decision_Model comparison of the calibrated probabilities, naming the artifact.
        cutoff = Cutoff(DECISION_MODEL_COMPARISON, scores=listed, artifact=artifact)
        winner = tied[0]
        decided = "probability"
        if len(tied) > 1:  # 22.12: equal probabilities go to the MDL_Scorer
            winner = min(tied, key=self.mdl_key)
            cutoff = Cutoff(MDL_COMPARISON, scores=tuple(self.mdl_score(i) for i in tied))
            details["tie_break"] = MDL_COMPARISON
            decided = f"equal probabilities, then {self.decided_by(tied, winner)}"
        return Expected(
            "decision-model",
            winner,
            self.specs[winner].label,
            dm,
            best,
            Resolution("decision-model", widening.index(winner), listed),
            cutoff=cutoff,
            details=details,
            decided_by=f"{artifact.rsplit('.', 1)[1]} model: {decided}",
        )

    def by_mdl(self, resolver: str, pool: list[int], fallback: FallbackInfo | None) -> Expected:
        winner = min(pool, key=self.mdl_key)
        scores = tuple(self.mdl_score(i) for i in pool)
        lengths = [self.specs[i].total for i in pool]
        weights = [2.0 ** (min(lengths) - length) for length in lengths]
        spec = self.specs[winner]
        return Expected(
            "mdl",
            winner,
            spec.label,
            spec.technique or resolver,
            weights[pool.index(winner)] / math.fsum(weights),
            Resolution("mdl", pool.index(winner), scores),
            cutoff=Cutoff(MDL_COMPARISON, scores=scores),
            fallback=fallback,
            details={"flagged": len(spec.rejected)} if spec.exceptional else {},
            decided_by=self.decided_by(pool, winner),
        )

    def by_priority(self, produced: list[int]) -> Expected:
        winner = produced[0]
        listed = tuple(CandidateScore(self.specs[i].technique, self.specs[i].label, None) for i in produced)
        if len(produced) == 1:
            decided = "single candidate"
        elif self.rank(self.specs[produced[0]]) < self.rank(self.specs[produced[1]]):
            decided = "priority"
        else:
            decided = "candidate order"
        spec = self.specs[winner]
        assert spec.technique is not None
        return Expected(
            "priority",
            winner,
            spec.label,
            spec.technique,
            None,
            Resolution("priority", 0, listed),
            cutoff=self.sc.cutoff,
            # 19.7: priority among two or more candidates is the Fallback_Behavior of the disabled MDL selection.
            fallback=FallbackInfo(MDL, "disabled") if len(produced) >= 2 else None,
            decided_by=decided,
        )

    def by_fallback(self, widening: list[int]) -> Expected:
        structural = [i for i in widening if self.specs[i].technique is None]
        outcome = self.dp.fallback.outcome
        chosen: int | None
        if outcome == OUTCOME_CANDIDATE_ORDER and structural:
            chosen = structural[0]
        else:
            chosen = next((i for i in structural if self.specs[i].label == outcome), None)
        listed = [CandidateScore(None, self.specs[i].label, None) for i in structural]
        if chosen is None:
            listed.insert(0, CandidateScore(None, outcome, None))
            selected = 0
        else:
            selected = structural.index(chosen)
        return Expected(
            "fallback",
            chosen,
            outcome if chosen is None else self.specs[chosen].label,
            f"fallback:{self.kind}",
            None,
            Resolution("fallback", selected, tuple(listed)),
            fallback=self.fallback_info(),
            details={"fallback_behavior": self.dp.fallback_doc},
            decided_by="fallback" if chosen is None else "fallback candidate",
        )

    def fallback_of(self, tid: str, why: Reason) -> FallbackInfo:
        """The fallback indicator for serving Technique ``tid``: a gated one names its disabled gate (15.7)."""
        if why == "gated":
            return FallbackInfo(self.dp.serves_only_with[tid], "disabled")
        return FallbackInfo(tid, cast(FallbackReason, why))

    def fallback_info(self) -> FallbackInfo:
        """The first serving Technique that is disabled or missing an artifact, else the first disabled gate."""
        gated: list[str] = []
        for tid in self.dp.served_by:
            why = self.reason(tid)
            assert why is not None
            if why != "gated":
                return self.fallback_of(tid, why)
            gated.append(tid)
        return self.fallback_of(gated[0], "gated")


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def run(scenario: Scenario, order: Sequence[int]) -> tuple[Decision, list[Candidate], ObservationStore, InferenceReport, int]:
    """Resolve the scenario with its candidates supplied in ``order``; returns the feature-call count too."""
    candidates, store = materialize(scenario.specs)
    report = InferenceReport()
    calls: list[None] = []

    def features() -> np.ndarray:
        calls.append(None)
        return FEATURES

    engine = DecisionEngine(dict(scenario.states), bundle=StubBundle(scenario), report=report)
    decision = engine.resolve(
        DecisionPoint(scenario.kind, scenario.path),
        [candidates[i] for i in order],
        features=features,
        mdl_by=scenario.mdl_by,
        cutoff=scenario.cutoff,
    )
    return decision, candidates, store, report, len(calls)


def check(scenario: Scenario, reference: Reference, expected: Expected, order: Sequence[int]) -> bytes:
    decision, candidates, store, report, feature_calls = run(scenario, order)
    note(f"order {tuple(order)}: {decision.entry.to_json()!r}")

    assert decision.kind == scenario.kind
    assert decision.path == (ROOT if scenario.path is None else scenario.path)
    assert decision.method == expected.method
    if expected.selected is None:
        assert decision.candidate is None and decision.outcome == expected.label
    else:
        selected = candidates[expected.selected]
        assert decision.candidate is selected, (
            f"selected {decision.candidate!r}, expected spec {expected.selected}: {scenario.specs[expected.selected]}"
        )
        assert decision.outcome == selected.outcome
    assert decision.label == expected.label
    assert decision.technique == expected.technique

    # 19.5, 26.3: every candidate considered, with its score, and the selection.
    assert decision.resolution == expected.resolution

    # Confidence: the MDL posterior, the calibrated probability, or none.
    if expected.method == "mdl":
        assert decision.confidence is not None and expected.confidence is not None
        assert math.isclose(decision.confidence, expected.confidence, rel_tol=POSTERIOR_REL_TOL)
    else:
        assert decision.confidence == expected.confidence
    record = decision.provenance(SUPPORT)
    assert record.technique == expected.technique
    assert record.confidence == (decision.confidence if decision.confidence is not None else laplace_confidence(SUPPORT))
    assert (record.resolution, record.cutoff, record.fallback) == (
        decision.resolution,
        decision.cutoff,
        decision.fallback,
    )

    assert decision.cutoff == expected.cutoff
    assert decision.fallback == expected.fallback
    assert decision.details == expected.details

    # 17.2, 17.3: only a winning exception-bearing candidate flags rows, exactly its own.
    if expected.selected is not None and scenario.specs[expected.selected].exceptional:
        assert decision.flagged is candidates[expected.selected].rejects
        assert store.flags.n_items == len(scenario.specs[expected.selected].rejected)
    else:
        assert decision.flagged is None and store.flags.is_empty

    # Features are computed exactly when a Decision_Model is consulted, even if the exception check replaces its pick.
    assert feature_calls == (1 if "decision_model" in expected.details else 0)
    assert report.decisions == (decision.entry,)
    assert {(sub.artifact, sub.fallback) for sub in report.substitutions} == reference.substitutions()

    # 28.11: no candidate record and no producing Technique names a disabled Technique.
    named = [score.technique for score in decision.resolution.candidates]
    if decision.cutoff is not None:
        named.extend(score.technique for score in decision.cutoff.scores)
    if decision.method != "fallback":
        named.append(decision.technique)
    for tid in named:
        assert tid is None or reference.state(tid), f"the record names disabled Technique {tid}"

    return canonical_bytes(decision.entry.to_json())


@given(scenario=scenarios())
def test_decision_engine_follows_the_resolution_order(scenario: Scenario) -> None:
    note(repr(scenario))
    reference = Reference(scenario)
    expected = reference.expect()
    note(repr(expected))
    event(f"method: {expected.method}")
    event(f"{expected.method} decided by: {expected.decided_by}")
    if expected.selected is not None and scenario.specs[expected.selected].exceptional:
        event("an exception-bearing candidate wins")
    if expected.fallback is not None:
        event(f"fallback reason: {expected.fallback.reason}")

    identity = tuple(range(len(scenario.specs)))
    in_order = check(scenario, reference, expected, identity)
    shuffled = check(scenario, reference, expected, scenario.order)
    assert in_order == shuffled, "the Inference_Report entry depends on the candidate input order"
