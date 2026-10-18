"""Record-or-Map, Closed-or-Open-Record, and their object nodes (Requirements 4.1–4.4, 4.7–4.11, 16.5).

Only the non-flagged object rows of a view count (:func:`object_rows`);
scalars and arrays at the same Path belong to the Value_Inferrer and the
array analysis. Both Judgment_Calls are resolved per Path, before Variant
grouping (design walk, steps 2.1 and 2.2).

Record-or-Map (4.1, 4.10, 4.11)
    :func:`record_or_map` resolves the ``record_or_map`` Decision_Point of a
    view whose objects hold at least one key. Both candidates are
    structural alternatives that no Technique produces (``technique=None``),
    labeled with the kind's outcomes, and scored by the MDL_Scorer on the
    same object rows (19.3), with descendants represented by their
    Naive_Merge subschemas (local scoring):

    - Record (:func:`record_encoding`): the Naive_Merge subschema of the
      objects, ``properties`` for every observed key and ``required`` for
      the keys every object holds. The key literals are paid once in the
      model bits; each optional key pays its presence bits.
    - Map (:func:`map_encoding`): ``additionalProperties`` holding the
      Naive_Merge subschema of every member value, so the key literals and
      key counts are paid in the data bits.

    The DecisionEngine resolves the point in its order: the Record-or-Map
    Decision_Model (``decide.dm_record_map``) when it is in effect and the
    Model_Bundle holds it, else MDL while ``decide.mdl`` is enabled (the
    lower description length wins, ties go to Record by the registry's
    mandated tie rule), else the Fallback_Behavior Record (4.10). The
    Provenance_Record of the outcome (:meth:`RecordMapOutcome.provenance`)
    names the resolving Technique (``decide.mdl``, the Decision_Model, or
    ``fallback:record_or_map``), its Confidence (the MDL posterior or the
    calibrated probability; the Laplace default for a fallback), and every
    candidate with its score, the rejected alternative included (4.1). It
    belongs to the object-kind Constraint: ``properties`` of a Record and
    ``additionalProperties`` of a Map.

    When no non-flagged object holds a key, the Path is a Record with no
    ``properties`` and no Decision_Point is resolved (4.11). The object-kind
    Constraint then sits on ``type`` (design granularity table), whose
    record comes from the ``type`` Baseline_Rule; the analyzer merges
    :meth:`RecordMapOutcome.details` (``"keys_available": false``) into it,
    which records that no keys were available to evaluate.

Closed-or-Open-Record (4.7–4.9)
    :func:`closed_or_open` resolves the ``closed_or_open_record``
    Decision_Point of a Record Path. The open candidate is the Record
    encoding (or the caller's ``record``, for the hybrid partition of 4.5)
    without ``additionalProperties``, whose data bits include the
    ``CAT(no extras, extras)`` flag over the objects. The closed candidate
    adds ``additionalProperties: false`` (its keyword slot and flag in the
    model bits) plus the Good–Turing unseen-key charge
    (:func:`unseen_key_charge`), added to its data and exception bits:
    the expected cost of the exceptions that keys not seen yet would cause.
    The engine resolves the point as above: the Decision_Model
    (``decide.dm_closed_record``), else MDL while ``decide.mdl`` is enabled
    (no tie rule is mandated, so a tie goes to fewer model bits, which is
    open), else the Fallback_Behavior open, which matches Naive_Merge. A
    closed Record emits ``additionalProperties: false`` (4.8) with the
    decision's Provenance_Record; an open Record emits nothing (4.9), and the
    engine's Inference_Report entry is the decision's only record (20.7).

The Good–Turing unseen-key charge
    Let ``f1`` be the number of keys listed under the Record's
    ``properties`` that exactly one object holds, and ``m`` the number of
    objects that hold at least one of them, among ``n`` objects. If those
    keys had been unseen, a closed Record would have rejected the ``m``
    objects, paying the 19.2 exception cost ``CAT(n − m, m) + Σ U(o)`` over
    them (the node's 1-bit exception flag is paid anyway). The design's
    charge ``f1 × mean EXC(singleton-key objects)`` is that cost divided by
    ``m`` and multiplied by ``f1``, the Good–Turing estimate of the number
    of unseen keys a further sample of the same size brings. When every
    such object holds one singleton key, ``f1 = m`` and the charge is the
    exception cost itself, as for Closed-or-Open-Enum (15.6). With no
    singleton key the charge is 0. ``U(o)`` is computed by
    :meth:`~jsinfer.mdl.scorer.MDLScorer.universal_bits`, the same code a
    real exception pays.

Object nodes (4.2, 4.3, 4.4, 16.5)
    :class:`RecordMapAnalyzer` builds the object keywords of a node from an
    outcome; the analyzer (task 16.1) adds ``type``, ``required``
    (:func:`~jsinfer.deps.presence.required_fields`), rule keywords, and
    branches, and recurses through ``child_schema``:

    - Record (:meth:`RecordMapAnalyzer.record_node`): ``properties`` lists
      exactly the keys of the view's non-flagged objects, each with
      ``child_schema(view.child_key(name))`` (4.2), and a closed Record adds
      ``additionalProperties: false`` (4.8).
    - Map (:meth:`RecordMapAnalyzer.map_node`): exactly one value schema,
      ``child_schema(view.child_map())``, under ``additionalProperties``, and
      no ``properties`` or ``required`` (4.3); ``propertyNames`` from
      :meth:`~jsinfer.values.inferrer.ValueInferrer.key_constraint` when a
      key Format (or, after task 32.1, a Pattern) is selected (4.4); and
      ``minProperties``/``maxProperties`` from
      :meth:`~jsinfer.values.inferrer.ValueInferrer.decide_bound` over the
      key counts of the non-flagged objects (16.5), which omits them while
      ``values.bounds`` is disabled (16.8).

    :meth:`RecordMapAnalyzer.object_node` runs both Judgment_Calls and builds
    the node of a single Variant. ``child_schema`` defaults to
    :func:`~jsinfer.pipeline.naive_merge.naive_merge_node`; the
    Structure_Analyzer passes its own ``analyze`` (task 16.1).

Decision_Model features
    A Decision_Model in effect scores the candidates on ``features``. The
    defaults are small vectors of the statistics at hand plus the MDL
    difference of the two outcomes normalized by the Observations scored
    (:func:`record_map_features`, :func:`closed_record_features`); task 22.1
    replaces them with the versioned featurizers of ``learn/features.py``.

Hooks
    The hybrid Record/Map partition (4.5, 4.6, task 37.1) replaces the
    Record-or-Map outcome when it beats both pure encodings, and passes the
    partitioned Record to :func:`closed_or_open` as ``record``. MDL
    exception encoding (17.3, task 38.2) adds exception-bearing candidates.

Determinism
    Scores depend only on the multiset of non-flagged Observations (19.6),
    candidates are offered in a fixed order, and keys are handled in key-id
    order, which is code-point order, so outcomes do not depend on document
    order, chunking, or worker count. No cut-off is hard-coded: each outcome
    is an MDL comparison, a calibrated Decision_Model comparison, or a
    constant Fallback_Behavior.
"""

from __future__ import annotations

import math
from collections.abc import Callable, Collection, Mapping, Sequence
from dataclasses import dataclass, replace
from typing import Final, Literal, TypeAlias

import numpy as np

from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.observations import isin_sorted
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, ArtifactSource, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.io.validator import Validator
from jsinfer.mdl.codes import CAT
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.values.bounds import KEY_COUNT, BoundDecision
from jsinfer.values.inferrer import KeyConstraint, ValueInferrer

__all__ = [
    "CLOSED",
    "CLOSED_OR_OPEN_RECORD",
    "DM_CLOSED_RECORD",
    "DM_RECORD_MAP",
    "MAP",
    "OPEN",
    "RECORD",
    "RECORD_OR_MAP",
    "ChildSchema",
    "ClosedOpenOutcome",
    "JudgmentFeatures",
    "KeyStats",
    "ObjectNode",
    "RecordMapAnalyzer",
    "RecordMapOutcome",
    "closed_or_open",
    "closed_record_features",
    "map_encoding",
    "object_rows",
    "record_encoding",
    "record_map_features",
    "record_or_map",
    "unseen_key_charge",
]

RECORD_OR_MAP: Final = "record_or_map"
CLOSED_OR_OPEN_RECORD: Final = "closed_or_open_record"
DM_RECORD_MAP: Final = "decide.dm_record_map"
DM_CLOSED_RECORD: Final = "decide.dm_closed_record"
RECORD, MAP = JUDGMENT_OUTCOMES[RECORD_OR_MAP]
CLOSED, OPEN = JUDGMENT_OUTCOMES[CLOSED_OR_OPEN_RECORD]

ObjectKind: TypeAlias = Literal["record", "map"]

# Infers the subschema of a child view (a Record property or the Map values).
ChildSchema = Callable[[View], SchemaNode]

# Computes the Decision_Model features from the object rows and the candidates (task 22.1 supplies them).
JudgmentFeatures = Callable[[View, Sequence[Candidate]], np.ndarray]

# The keywords the object-kind Constraint sits on (design granularity table).
_PROPERTIES: Final = "properties"
_ADDITIONAL: Final = "additionalProperties"
_TYPE: Final = "type"


# ---------------------------------------------------------------------------
# Object rows and key statistics
# ---------------------------------------------------------------------------


def object_rows(view: View) -> View:
    """The non-flagged object rows of ``view``."""
    if not isinstance(view, View):
        raise TypeError(f"expected a View, got {type(view).__name__}")
    live = view.live()
    mask = live.gather("jtype") == JType.OBJECT
    return live if mask.all() else live.subset(mask)


def _objects_of(view: View) -> View:
    """The object rows of ``view``; ``ValueError`` when it has none (not an object Path)."""
    objects = object_rows(view)
    if objects.is_empty:
        raise ValueError(f"no non-flagged object at {render_path(view.path)}")
    return objects


@dataclass(frozen=True, slots=True)
class KeyStats:
    """Key statistics over the non-flagged objects of a Path.

    ``n_objects`` objects hold ``n_keys`` distinct keys with ``occurrences``
    key occurrences in total (each key once per object that holds it).
    ``f1`` and ``f2`` count the keys held by exactly one and exactly two
    objects, and ``singleton_objects`` the objects that hold at least one
    key that no other object holds.
    """

    n_objects: int
    n_keys: int
    occurrences: int
    f1: int
    f2: int
    singleton_objects: int

    @classmethod
    def of(cls, objects: View) -> KeyStats:
        """The statistics of the object rows of ``objects``."""
        stats = objects.stats()
        counts = stats.key_counts
        owners = stats.key_objects[np.repeat(counts == 1, np.diff(stats.key_indptr))]
        return cls(
            n_objects=stats.n_objects,
            n_keys=len(counts),
            occurrences=int(counts.sum()),
            f1=int((counts == 1).sum()),
            f2=int((counts == 2).sum()),
            singleton_objects=len(np.unique(owners)),
        )

    @property
    def good_turing(self) -> float:
        """The Good–Turing unseen-key mass ``f1 / occurrences`` (0 when no object holds a key)."""
        return self.f1 / self.occurrences if self.occurrences else 0.0

    def to_json(self) -> dict[str, JSON]:
        """The statistics as recorded in provenance."""
        return {
            "f1": self.f1,
            "f2": self.f2,
            "good_turing": self.good_turing,
            "n_keys": self.n_keys,
            "n_objects": self.n_objects,
            "occurrences": self.occurrences,
            "singleton_objects": self.singleton_objects,
        }


def unseen_key_charge(objects: View, scorer: MDLScorer, names: Collection[str] | None = None) -> float:
    """The closed candidate's Good–Turing unseen-key charge in bits (see the module docstring).

    ``objects`` are object rows; ``names`` restricts the singleton keys to
    the keys a Record lists under ``properties`` (every observed key by
    default). 0 when no listed key is held by exactly one object.
    """
    if not isinstance(scorer, MDLScorer):
        raise TypeError(f"scorer must be an MDLScorer, got {type(scorer).__name__}")
    live = objects.live()
    stats = live.stats()
    single = stats.key_counts == 1
    if names is not None:
        store = live.store
        listed = sorted(kid for kid in (store.key_id(name) for name in names) if kid is not None)
        single &= isin_sorted(stats.key_ids.astype(np.int64), np.array(listed, dtype=np.int64))
    f1 = int(single.sum())
    if not f1:
        return 0.0
    owners = np.unique(stats.key_objects[np.repeat(single, np.diff(stats.key_indptr))])
    m = len(owners)
    mask = np.zeros(len(live), dtype=bool)
    mask[stats.object_index[owners]] = True
    exceptions = math.fsum((CAT((stats.n_objects - m, m)), scorer.universal_bits(live.subset(mask))))
    return exceptions * (f1 / m)


# ---------------------------------------------------------------------------
# Candidate encodings
# ---------------------------------------------------------------------------


def record_encoding(objects: View) -> SchemaNode:
    """The open Record candidate: the Naive_Merge subschema of the object rows (``properties`` and ``required``)."""
    return naive_merge_node(object_rows(objects))


def map_encoding(objects: View) -> SchemaNode:
    """The Map candidate: ``additionalProperties`` holding the Naive_Merge subschema of every member value."""
    rows = object_rows(objects)
    return SchemaNode(kw={"type": ["object"], _ADDITIONAL: naive_merge_node(rows.child_map())})


def _charged(score: Score, charge: float) -> Score:
    """``score`` with ``charge`` added to its data and exception bits."""
    if not charge:
        return score
    return replace(
        score,
        data_bits=math.fsum((score.data_bits, charge)),
        exception_bits=math.fsum((score.exception_bits, charge)),
    )


def _normalized_difference(candidates: Sequence[Candidate]) -> float:
    """``(DL(first) − DL(second)) / n_scored`` of two scored candidates: the MDL feature of every kind."""
    first, second = (candidate.score for candidate in candidates)
    assert first is not None and second is not None
    return (first.total - second.total) / max(first.n_scored, 1)


def record_map_features(objects: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """Default Record-or-Map features: objects, keys, keys per object, singleton-key share, and the MDL difference."""
    keys = KeyStats.of(objects)
    per_object = keys.occurrences / keys.n_objects if keys.n_objects else 0.0
    singleton = keys.f1 / keys.n_keys if keys.n_keys else 0.0
    return np.array(
        [keys.n_objects, keys.n_keys, per_object, singleton, _normalized_difference(candidates)], dtype=np.float64
    )


def closed_record_features(objects: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """Default Closed-or-Open-Record features: objects, keys, f1, f2, the unseen-key mass, and the MDL difference."""
    keys = KeyStats.of(objects)
    return np.array(
        [keys.n_objects, keys.n_keys, keys.f1, keys.f2, keys.good_turing, _normalized_difference(candidates)],
        dtype=np.float64,
    )


# ---------------------------------------------------------------------------
# Outcomes
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class RecordMapOutcome:
    """The Record-or-Map classification of one Path (see the module docstring).

    ``decision`` is None exactly when no non-flagged object holds a key, so
    the Path is a Record without resolving the Judgment_Call (4.11).
    """

    path: Path
    kind: ObjectKind
    keys: KeyStats
    decision: Decision | None = None

    def __post_init__(self) -> None:
        if self.kind not in (RECORD, MAP):
            raise ValueError(f"an object kind is {RECORD!r} or {MAP!r}, got {self.kind!r}")
        if self.decision is None and (self.kind != RECORD or self.keys.n_keys):
            raise ValueError("only a Path whose objects hold no key is classified without a decision (4.11)")

    @property
    def is_record(self) -> bool:
        return self.kind == RECORD

    @property
    def is_map(self) -> bool:
        return self.kind == MAP

    @property
    def keys_available(self) -> bool:
        """Whether some non-flagged object holds a key, so the Judgment_Call was resolved."""
        return self.keys.n_keys > 0

    @property
    def constraint_keyword(self) -> str:
        """The keyword of the object-kind Constraint: ``properties``, ``additionalProperties``, or ``type`` (4.11)."""
        if not self.keys_available:
            return _TYPE
        return _PROPERTIES if self.is_record else _ADDITIONAL

    def details(self) -> dict[str, JSON]:
        """The judgment, and for 4.11 that no keys were available to evaluate."""
        details: dict[str, JSON] = {"judgment": self.kind}
        if not self.keys_available:
            details["keys_available"] = False
        return details

    def provenance(self) -> ProvenanceRecord:
        """The Provenance_Record of the object-kind Constraint of a resolved Path (4.1).

        Support is every non-flagged object at the Path, each of which the
        outcome describes; Confidence is the decision's (the Laplace default
        after a Fallback_Behavior).
        """
        if self.decision is None:
            raise ValueError(
                f"{render_path(self.path)} has no keys, so the Record-or-Map Judgment_Call was not resolved (4.11); "
                "merge details() into the record of its type Constraint"
            )
        n = self.keys.n_objects
        return self.decision.provenance(Support(n, n), details=self.details())


@dataclass(frozen=True, eq=False)
class ClosedOpenOutcome:
    """The Closed-or-Open-Record judgment of one Record Path (see the module docstring).

    ``charge`` is the Good–Turing unseen-key charge added to the closed
    candidate, in bits.
    """

    path: Path
    closed: bool
    keys: KeyStats
    charge: float
    decision: Decision

    @property
    def outcome(self) -> str:
        return CLOSED if self.closed else OPEN

    def keywords(self) -> dict[str, SchemaNode]:
        """``{"additionalProperties": false}`` for a closed Record (4.8), ``{}`` for an open one (4.9)."""
        return {_ADDITIONAL: SchemaNode(boolean=False)} if self.closed else {}

    def details(self) -> dict[str, JSON]:
        """The judgment, the key statistics, and the unseen-key charge."""
        return {"judgment": self.outcome, "key_statistics": self.keys.to_json(), "unseen_key_charge": self.charge}

    def provenance(self) -> ProvenanceRecord:
        """The Provenance_Record of the ``additionalProperties: false`` Constraint of a closed Record.

        Every non-flagged object holds only listed keys, so Support is every
        object; Confidence is the decision's.
        """
        if not self.closed:
            raise ValueError(f"the Record at {render_path(self.path)} is open, so it has no closing Constraint")
        n = self.keys.n_objects
        return self.decision.provenance(Support(n, n), details=self.details())


# ---------------------------------------------------------------------------
# The two Judgment_Calls
# ---------------------------------------------------------------------------


def _check_engine(engine: DecisionEngine, scorer: MDLScorer | None) -> MDLScorer:
    if not isinstance(engine, DecisionEngine):
        raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
    if scorer is not None and not isinstance(scorer, MDLScorer):
        raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
    return scorer if scorer is not None else MDLScorer()


def record_or_map(
    view: View,
    engine: DecisionEngine,
    *,
    scorer: MDLScorer | None = None,
    features: JudgmentFeatures | None = None,
) -> RecordMapOutcome:
    """Classify the objects of ``view`` as Record or Map (4.1, 4.10, 4.11); see the module docstring.

    ``engine`` resolves the Decision_Point and records it in its report.
    ``scorer`` scores both candidates (a default MDL_Scorer when None).
    ``features`` computes the Decision_Model features (:func:`record_map_features`
    by default). Raises ``ValueError`` when the view has no non-flagged object.
    """
    scorer = _check_engine(engine, scorer)
    objects = _objects_of(view)
    keys = KeyStats.of(objects)
    if not keys.n_keys:  # 4.11: a Record with no properties, without the Judgment_Call
        return RecordMapOutcome(objects.path, RECORD, keys)
    candidates = tuple(
        Candidate(technique=None, outcome=label, order_key=(index,), node=node, score=scorer.score(node, objects))
        for index, (label, node) in enumerate(((RECORD, record_encoding(objects)), (MAP, map_encoding(objects))))
    )

    def featurize() -> np.ndarray:
        return (features or record_map_features)(objects, candidates)

    decision = engine.resolve(DecisionPoint(RECORD_OR_MAP, objects.path), candidates, objects, featurize)
    kind: ObjectKind = MAP if decision.label == MAP else RECORD
    return RecordMapOutcome(objects.path, kind, keys, decision)


def _open_record(record: SchemaNode | None, objects: View) -> SchemaNode:
    if record is None:
        return record_encoding(objects)
    if not isinstance(record, SchemaNode):
        raise TypeError(f"record must be a SchemaNode or None, got {type(record).__name__}")
    if record.boolean is not None or _ADDITIONAL in record.kw:
        raise ValueError("record is the open Record candidate: a keyword schema without 'additionalProperties'")
    return record


def closed_or_open(
    view: View,
    engine: DecisionEngine,
    *,
    record: SchemaNode | None = None,
    scorer: MDLScorer | None = None,
    features: JudgmentFeatures | None = None,
) -> ClosedOpenOutcome:
    """Judge the Record at the Path of ``view`` closed or open (4.7–4.9); see the module docstring.

    ``record`` is the open Record candidate (by default :func:`record_encoding`
    of the objects; the hybrid partition of 4.5 passes its partitioned
    Record). ``engine``, ``scorer``, and ``features``
    (:func:`closed_record_features` by default) are as for :func:`record_or_map`.
    """
    scorer = _check_engine(engine, scorer)
    objects = _objects_of(view)
    open_node = _open_record(record, objects)
    closed_json = open_node.to_json()
    assert isinstance(closed_json, dict)
    closed_node = SchemaNode.from_json({**closed_json, _ADDITIONAL: False})
    listed = open_node.kw.get(_PROPERTIES, {})
    charge = unseen_key_charge(objects, scorer, listed)
    candidates = (
        Candidate(
            technique=None,
            outcome=CLOSED,
            order_key=(0,),
            node=closed_node,
            score=_charged(scorer.score(closed_node, objects), charge),
        ),
        Candidate(technique=None, outcome=OPEN, order_key=(1,), node=open_node, score=scorer.score(open_node, objects)),
    )

    def featurize() -> np.ndarray:
        return (features or closed_record_features)(objects, candidates)

    decision = engine.resolve(DecisionPoint(CLOSED_OR_OPEN_RECORD, objects.path), candidates, objects, featurize)
    return ClosedOpenOutcome(objects.path, decision.label == CLOSED, KeyStats.of(objects), charge, decision)


# ---------------------------------------------------------------------------
# Object nodes
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class ObjectNode:
    """The object keywords of one node and their Provenance_Records (see the module docstring).

    ``node`` holds only object keywords; ``provenance`` maps each emitted
    Constraint keyword to its record. ``closed`` is the Closed-or-Open
    judgment of a Record (None for a Map); ``key_constraint`` and
    ``key_bounds`` are a Map's ``propertyNames`` selection and key-count
    bound decision.
    """

    path: Path
    node: SchemaNode
    provenance: Mapping[str, ProvenanceRecord]
    kind: RecordMapOutcome
    closed: ClosedOpenOutcome | None = None
    key_constraint: KeyConstraint | None = None
    key_bounds: BoundDecision | None = None

    def keywords(self) -> dict[str, JSON]:
        """The emitted keywords as plain JSON."""
        schema = self.node.to_json()
        assert isinstance(schema, dict)
        return schema

    def constraint_provenance(self) -> dict[ConstraintId, ProvenanceRecord]:
        """The Provenance_Records keyed by the ConstraintIds of ``node``."""
        return {ConstraintId(self.node.nid, keyword): record for keyword, record in self.provenance.items()}

    def type_details(self) -> dict[str, JSON]:
        """What the ``type`` record of a Record without ``properties`` notes (4.11); ``{}`` otherwise.

        Such a node's object-kind Constraint is its ``type`` (design
        granularity table), so the analyzer merges these details into the
        ``type`` record of the ``type`` Baseline_Rule.
        """
        if self.kind.is_map or _PROPERTIES in self.node.kw:
            return {}
        return {"judgment": RECORD, "keys_available": False}


class RecordMapAnalyzer:
    """Record-or-Map, Closed-or-Open-Record, and object nodes for one run (see the module docstring).

    ``values`` is the run's ValueInferrer: its engine resolves every
    Decision_Point, and it supplies the Map-key Constraint and the key-count
    bounds. ``scorer`` scores the Judgment_Call candidates (a new MDL_Scorer
    sharing the inferrer's Validator by default). ``record_map_features`` and
    ``closed_record_features`` compute the Decision_Model features.
    """

    __slots__ = ("_closed_features", "_kind_features", "_scorer", "_values")

    def __init__(
        self,
        values: ValueInferrer,
        *,
        scorer: MDLScorer | None = None,
        record_map_features: JudgmentFeatures | None = None,
        closed_record_features: JudgmentFeatures | None = None,
    ) -> None:
        if not isinstance(values, ValueInferrer):
            raise TypeError(f"values must be a ValueInferrer, got {type(values).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        self._values = values
        self._scorer = scorer if scorer is not None else MDLScorer(validator=values.validator)
        self._kind_features = record_map_features
        self._closed_features = closed_record_features

    @classmethod
    def for_techniques(
        cls,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
    ) -> RecordMapAnalyzer:
        """An analyzer whose engine has the Technique states ``techniques`` (omitted ids take their default)."""
        validator = validator if validator is not None else Validator()
        values = ValueInferrer.for_techniques(techniques, bundle=bundle, report=report, validator=validator)
        return cls(values, scorer=MDLScorer(validator=validator))

    @property
    def values(self) -> ValueInferrer:
        return self._values

    @property
    def engine(self) -> DecisionEngine:
        return self._values.engine

    # -- Judgment_Calls -----------------------------------------------------

    def record_or_map(self, view: View) -> RecordMapOutcome:
        """:func:`record_or_map` with this run's engine and scorer."""
        return record_or_map(view, self.engine, scorer=self._scorer, features=self._kind_features)

    def closed_or_open(self, view: View, *, record: SchemaNode | None = None) -> ClosedOpenOutcome:
        """:func:`closed_or_open` with this run's engine and scorer."""
        return closed_or_open(view, self.engine, record=record, scorer=self._scorer, features=self._closed_features)

    # -- nodes --------------------------------------------------------------

    def record_node(
        self,
        view: View,
        outcome: RecordMapOutcome,
        closed: ClosedOpenOutcome | None = None,
        *,
        child_schema: ChildSchema = naive_merge_node,
    ) -> ObjectNode:
        """The keywords of a Record node over the objects of ``view`` (the Path's, or a Variant's).

        ``properties`` lists exactly the keys of the view's non-flagged
        objects with ``child_schema`` of each key's values (4.2), and carries
        the Record-or-Map record; ``closed`` adds ``additionalProperties:
        false`` when it judged the Record closed (4.8).
        """
        if not isinstance(outcome, RecordMapOutcome) or not outcome.is_record:
            raise ValueError("record_node needs a Record outcome")
        objects = _objects_of(view)
        store = objects.store
        kw: dict[str, object] = {}
        provenance: dict[str, ProvenanceRecord] = {}
        # Key ids follow code-point order, so the names are already sorted.
        names = [store.key_name(int(kid)) for kid in objects.stats().key_ids.tolist()]
        if names:
            kw[_PROPERTIES] = {name: child_schema(objects.child_key(name)) for name in names}
            if outcome.decision is not None:
                provenance[_PROPERTIES] = outcome.provenance()
        if closed is not None and closed.closed:
            kw.update(closed.keywords())
            provenance[_ADDITIONAL] = closed.provenance()
        return ObjectNode(objects.path, SchemaNode(kw=kw), provenance, outcome, closed=closed)

    def map_node(
        self, view: View, outcome: RecordMapOutcome, *, child_schema: ChildSchema = naive_merge_node
    ) -> ObjectNode:
        """The keywords of a Map node (4.3, 4.4, 16.5).

        One value schema, ``child_schema(view.child_map())``, under
        ``additionalProperties`` with the Record-or-Map record; ``propertyNames``
        when a key Constraint is selected; and the key-count bounds.
        """
        if not isinstance(outcome, RecordMapOutcome) or not outcome.is_map:
            raise ValueError("map_node needs a Map outcome")
        objects = _objects_of(view)
        path = objects.path
        kw: dict[str, object] = {_ADDITIONAL: child_schema(objects.child_map())}
        provenance: dict[str, ProvenanceRecord] = {_ADDITIONAL: outcome.provenance()}
        key_constraint = self._values.key_constraint(objects)
        if key_constraint is not None:
            kw.update(key_constraint.keywords())
            provenance["propertyNames"] = key_constraint.provenance()
        key_bounds = self._values.decide_bound(objects.stats().object_sizes, KEY_COUNT, path=path)
        records = key_bounds.provenance()
        for keyword, value in key_bounds.keywords().items():
            kw[keyword] = value
            provenance[keyword] = records[keyword]
        return ObjectNode(
            path, SchemaNode(kw=kw), provenance, outcome, key_constraint=key_constraint, key_bounds=key_bounds
        )

    def object_node(self, view: View, *, child_schema: ChildSchema = naive_merge_node) -> ObjectNode:
        """Both Judgment_Calls and the object keywords of a single-Variant Path (design walk, steps 2.1 and 2.2).

        The Structure_Analyzer (task 16.1) calls the steps separately when
        Variants split the Record, and passes its own ``analyze`` as
        ``child_schema``.
        """
        outcome = self.record_or_map(view)
        if outcome.is_map:
            return self.map_node(view, outcome, child_schema=child_schema)
        return self.record_node(view, outcome, self.closed_or_open(view), child_schema=child_schema)
