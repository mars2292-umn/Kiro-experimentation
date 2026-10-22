"""Tuple-or-List and array nodes (Requirements 7.1–7.4, 7.9, 7.10, 16.4); the array grammar hook (7.5–7.8).

Only the non-flagged array rows of a view count (:func:`array_rows`);
scalars and objects at the same Path belong to the Value_Inferrer and the
object analysis. The length of an array is its number of elements, flagged
ones included (``ViewStats.array_lengths``); its non-flagged elements are
the Observations at the element Path ``path + [*]``. A Tuple position is not
a Path step: position ``i`` is the element view filtered to index ``i``
(:meth:`View.elements`), whose Path is the element Path as well.

Tuple-or-List (7.1, 7.10)
    :func:`tuple_or_list` resolves the ``tuple_or_list`` Decision_Point of a
    view in which at least one non-flagged element is observed. Both
    candidates are structural alternatives that no Technique produces
    (``technique=None``), labeled with the kind's outcomes, and scored by
    the MDL_Scorer on the same array rows (19.3), with descendants
    represented by their Naive_Merge subschemas (local scoring):

    - Tuple (:func:`tuple_encoding`): ``prefixItems`` with one subschema per
      position from 0 to the maximum length minus one, each the Naive_Merge
      subschema of the elements at that position, plus ``items: false``.
    - List (:func:`list_encoding`): one ``items`` subschema, the Naive_Merge
      subschema of every element; this is the Naive_Merge subschema of the
      arrays themselves.

    The Tuple candidate is withheld (Document_Support, 7.11,
    ``decide.support``) when the arrays come from one document or the
    arrays of only one document reach the last Tuple position: the Tuple
    inferred without that document would be shorter, and its ``items:
    false`` would reject that document's arrays. The engine then resolves
    the point among the candidates left, without the Decision_Model; on the
    real-data benchmark this is what kept geojson-countries' polygon lists
    from becoming one Tuple position per polygon.

    The DecisionEngine resolves the point in its order: the Tuple-or-List
    Decision_Model (``decide.dm_tuple_list``) when it is in effect and the
    Model_Bundle holds it, else MDL while ``decide.mdl`` is enabled, else
    the Fallback_Behavior List. Under MDL the Tuple wins only with a
    strictly lower description length: the registry's mandated tie rule
    gives a tie to List (7.10), whatever the model bits. The
    Provenance_Record of the outcome (:meth:`ArrayOutcome.provenance`) names
    the resolving Technique (``decide.mdl``, the Decision_Model, or
    ``fallback:tuple_or_list``), its Confidence (the MDL posterior or the
    calibrated probability; the Laplace default for a fallback), every
    candidate with its score, and the length statistics (7.1). Its Support
    is every non-flagged array at the Path, empty ones included, each of
    which the outcome describes.

    When every non-flagged array is empty (or holds only flagged elements),
    no Decision_Point is resolved and the outcome has no kind (7.9). The
    node then emits neither ``items`` nor ``prefixItems``, and
    :meth:`ArrayNode.type_details` (``"elements_observed": false``) is what
    the analyzer merges into the record of the Path's ``type`` Constraint,
    which comes from the ``type`` Baseline_Rule.

Array nodes (7.2–7.4, 7.9, 16.4)
    :class:`ArrayAnalyzer` builds the array keywords of a node; the
    analyzer (task 16.1) adds ``type`` and recurses through
    ``child_schema``:

    - List: ``items`` holding ``child_schema(view.elements())``, inferred
      from every non-flagged element, and no ``prefixItems`` (7.2). The
      Tuple-or-List record belongs to ``items``.
    - Tuple: ``prefixItems`` with ``child_schema(view.elements(i))`` for
      each position ``i`` below the maximum length (7.3), and ``items:
      false`` when no non-flagged array is longer than the prefix (7.4),
      which holds for every Tuple of 7.3 because the prefix spans the
      maximum length. Both Constraints carry the Tuple-or-List record. A
      position whose elements are all flagged gets ``child_schema`` of an
      empty view (``{}`` under Naive_Merge), so it accepts them.
    - Only empty arrays: neither keyword (7.9).
    - ``minItems``/``maxItems`` from
      :meth:`~jsinfer.values.inferrer.ValueInferrer.decide_bound` over the
      lengths of every non-flagged array (16.4), empty ones included, which
      omits them while ``values.bounds`` is disabled (16.8).

    :meth:`ArrayAnalyzer.array_node` runs the steps in the design's order:
    Tuple-or-List, the element subschemas, the array grammar, then the
    item-count bounds. ``child_schema`` defaults to
    :func:`~jsinfer.pipeline.naive_merge.naive_merge_node`; the
    Structure_Analyzer passes its own ``analyze`` (task 16.1).

The array grammar (7.5–7.8, task 43.1)
    ``structure.array_grammar`` serves the ``array_pattern`` Decision_Point
    at every Path with at least one non-flagged element. While it is
    disabled, :meth:`ArrayAnalyzer.array_node` resolves the point with no
    candidates, so the engine records its Fallback_Behavior, the schema of
    7.2–7.4, in the report (28.4), and the node is left as built. Task 43.1
    fills in the step; until then a run in which the registry marks the
    Technique implemented raises ``NotImplementedError``.

Decision_Model features
    A Decision_Model in effect scores the candidates on ``features``. The
    default (:func:`tuple_list_features`) is the design's Tuple-or-List
    vector computed from the view: arrays, length mean and standard
    deviation, distinct-length ratio, modal-length share, positional type
    entropy minus pooled type entropy, distinct element types, maximum
    length, and the MDL difference of the two outcomes normalized by the
    Observations scored. Task 22.1 replaces it with the versioned
    featurizer of ``learn/features.py``, which also splits object elements
    by Variant.

Cost
    The Tuple candidate builds one position view and Naive_Merge subschema
    per position up to the maximum length, so its cost grows with the
    longest array (bounded by the design point of documents of at most
    2 KB).

Determinism
    Scores depend only on the multiset of non-flagged Observations (19.6),
    candidates are offered in a fixed order (Tuple, then List), and
    positions are visited in index order, so outcomes do not depend on
    document order, chunking, or worker count. No cut-off is hard-coded:
    each outcome is an MDL comparison, a calibrated Decision_Model
    comparison, or a constant Fallback_Behavior.
"""

from __future__ import annotations

import math
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from typing import Final, Literal, TypeAlias

import numpy as np

from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, ArtifactSource, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.decide.support import QuantitySupport, withheld
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.values.bounds import ITEM_COUNT, BoundDecision
from jsinfer.values.inferrer import ValueInferrer

__all__ = [
    "ARRAY_GRAMMAR",
    "ARRAY_PATTERN",
    "DM_TUPLE_LIST",
    "LIST",
    "TUPLE",
    "TUPLE_OR_LIST",
    "ArrayAnalyzer",
    "ArrayNode",
    "ArrayOutcome",
    "ChildSchema",
    "JudgmentFeatures",
    "LengthStats",
    "array_rows",
    "list_encoding",
    "tuple_encoding",
    "tuple_list_features",
    "tuple_or_list",
    "type_entropies",
]

TUPLE_OR_LIST: Final = "tuple_or_list"
ARRAY_PATTERN: Final = "array_pattern"
DM_TUPLE_LIST: Final = "decide.dm_tuple_list"
ARRAY_GRAMMAR: Final = "structure.array_grammar"
TUPLE, LIST = JUDGMENT_OUTCOMES[TUPLE_OR_LIST]

ArrayKind: TypeAlias = Literal["tuple", "list"]

# Infers the subschema of an element view (all elements, or one Tuple position).
ChildSchema = Callable[[View], SchemaNode]

# Computes the Decision_Model features from the array rows and the candidates (task 22.1 supplies them).
JudgmentFeatures = Callable[[View, Sequence[Candidate]], np.ndarray]

# The keywords the Tuple-or-List Constraint sits on (design granularity table).
_ITEMS: Final = "items"
_PREFIX_ITEMS: Final = "prefixItems"
_TYPE: Final = "type"
_ARRAY_TYPE: Final = JType.ARRAY.keyword
_N_JTYPES: Final = len(JType)


# ---------------------------------------------------------------------------
# Array rows and length statistics
# ---------------------------------------------------------------------------


def array_rows(view: View) -> View:
    """The non-flagged array rows of ``view``."""
    if not isinstance(view, View):
        raise TypeError(f"expected a View, got {type(view).__name__}")
    live = view.live()
    mask = live.gather("jtype") == JType.ARRAY
    return live if mask.all() else live.subset(mask)


def _arrays_of(view: View) -> View:
    """The array rows of ``view``; ``ValueError`` when it has none (not an array Path)."""
    arrays = array_rows(view)
    if arrays.is_empty:
        raise ValueError(f"no non-flagged array at {render_path(view.path)}")
    return arrays


@dataclass(frozen=True, slots=True)
class LengthStats:
    """Length statistics over the non-flagged arrays of a Path.

    ``lengths`` are the distinct array lengths, ascending, and ``counts``
    the number of arrays of each length; a length counts every element,
    flagged ones included. ``n_elements`` is the number of non-flagged
    elements, the Observations at the element Path.
    """

    n_arrays: int
    n_elements: int
    lengths: tuple[int, ...]
    counts: tuple[int, ...]

    @classmethod
    def of(cls, arrays: View) -> LengthStats:
        """The statistics of the array rows of ``arrays``."""
        histogram = arrays.stats().array_lengths
        return cls(
            n_arrays=histogram.total,
            n_elements=len(arrays.elements()),
            lengths=tuple(histogram.values.tolist()),
            counts=tuple(histogram.counts.tolist()),
        )

    @property
    def elements_observed(self) -> bool:
        """Whether some non-flagged array holds a non-flagged element, so Tuple-or-List is resolved (7.1, 7.9)."""
        return self.n_elements > 0

    @property
    def max_length(self) -> int:
        """The maximum array length (0 when there is no array)."""
        return self.lengths[-1] if self.lengths else 0

    @property
    def min_length(self) -> int:
        """The minimum array length (0 when there is no array)."""
        return self.lengths[0] if self.lengths else 0

    @property
    def mean(self) -> float:
        """The mean array length (0 when there is no array)."""
        if not self.n_arrays:
            return 0.0
        return math.fsum(length * count for length, count in zip(self.lengths, self.counts)) / self.n_arrays

    @property
    def std(self) -> float:
        """The population standard deviation of the array lengths (0 when there is no array)."""
        if not self.n_arrays:
            return 0.0
        mean = self.mean
        variance = math.fsum(count * (length - mean) ** 2 for length, count in zip(self.lengths, self.counts))
        return math.sqrt(variance / self.n_arrays)

    @property
    def distinct_ratio(self) -> float:
        """Distinct lengths per array (0 when there is no array)."""
        return len(self.lengths) / self.n_arrays if self.n_arrays else 0.0

    @property
    def modal_share(self) -> float:
        """The share of arrays that have the most frequent length (0 when there is no array)."""
        return max(self.counts) / self.n_arrays if self.n_arrays else 0.0

    def to_json(self) -> dict[str, JSON]:
        """The statistics as recorded in provenance."""
        return {
            "max_length": self.max_length,
            "mean_length": self.mean,
            "min_length": self.min_length,
            "n_arrays": self.n_arrays,
            "n_distinct_lengths": len(self.lengths),
            "n_elements": self.n_elements,
        }


def _entropy_bits(counts: Iterable[int]) -> float:
    """The Shannon entropy in bits of the empirical distribution with these counts (0 for no counts)."""
    positive = [count for count in counts if count]
    n = sum(positive)
    if not n:
        return 0.0
    return max(0.0, math.log2(n) - math.fsum(count * math.log2(count) for count in positive) / n)


def type_entropies(arrays: View) -> tuple[float, float]:
    """``(pooled, positional)`` JType entropy in bits of the non-flagged elements of ``arrays``.

    Pooled is the entropy over every element; positional is the average,
    weighted by element counts, of the entropy at each position. A Tuple
    whose positions each hold one type has positional entropy 0. Both are
    0 when no element is observed.
    """
    elements = array_rows(arrays).elements()
    if elements.is_empty:
        return 0.0, 0.0
    jtype = elements.gather("jtype").astype(np.int64)
    position = elements.gather("pos").astype(np.int64)
    pooled = _entropy_bits(np.bincount(jtype, minlength=_N_JTYPES).tolist())
    _, pair_counts = np.unique(position * _N_JTYPES + jtype, return_counts=True)
    _, position_counts = np.unique(position, return_counts=True)
    n = len(jtype)
    # Σ_i (n_i / n) H_i = (Σ_i n_i log2 n_i − Σ_{i,t} c_it log2 c_it) / n
    terms = [count * math.log2(count) for count in position_counts.tolist()]
    terms.extend(-count * math.log2(count) for count in pair_counts.tolist())
    positional = max(0.0, math.fsum(terms) / n)
    return pooled, positional


# ---------------------------------------------------------------------------
# Candidate encodings
# ---------------------------------------------------------------------------


def list_encoding(arrays: View) -> SchemaNode:
    """The List candidate: one ``items`` subschema merging every element (the Naive_Merge subschema of the arrays)."""
    return naive_merge_node(array_rows(arrays))


def tuple_encoding(arrays: View) -> SchemaNode:
    """The Tuple candidate: ``prefixItems`` over every position below the maximum length, plus ``items: false``."""
    rows = array_rows(arrays)
    k = LengthStats.of(rows).max_length
    return SchemaNode(
        kw={
            _TYPE: [_ARRAY_TYPE],
            _PREFIX_ITEMS: [naive_merge_node(rows.elements(position)) for position in range(k)],
            _ITEMS: SchemaNode(boolean=False),
        }
    )


def _normalized_difference(candidates: Sequence[Candidate]) -> float:
    """``(DL(first) − DL(second)) / n_scored`` of two scored candidates: the MDL feature of every kind."""
    first, second = (candidate.score for candidate in candidates)
    assert first is not None and second is not None
    return (first.total - second.total) / max(first.n_scored, 1)


def tuple_list_features(arrays: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """Default Tuple-or-List features (see the module docstring), the MDL difference last."""
    rows = array_rows(arrays)
    lengths = LengthStats.of(rows)
    pooled, positional = type_entropies(rows)
    kinds = int(np.count_nonzero(np.bincount(rows.elements().gather("jtype"), minlength=_N_JTYPES)))
    return np.array(
        [
            lengths.n_arrays,
            lengths.mean,
            lengths.std,
            lengths.distinct_ratio,
            lengths.modal_share,
            positional - pooled,
            kinds,
            lengths.max_length,
            _normalized_difference(candidates),
        ],
        dtype=np.float64,
    )


# ---------------------------------------------------------------------------
# The outcome
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class ArrayOutcome:
    """The Tuple-or-List classification of one Path (see the module docstring).

    ``kind`` and ``decision`` are None exactly when no non-flagged element
    is observed, so the Judgment_Call is not resolved (7.9).
    """

    path: Path
    kind: ArrayKind | None
    lengths: LengthStats
    decision: Decision | None = None

    def __post_init__(self) -> None:
        if self.kind not in (TUPLE, LIST, None):
            raise ValueError(f"an array kind is {TUPLE!r}, {LIST!r}, or None, got {self.kind!r}")
        resolved = self.lengths.elements_observed
        if (self.kind is None) == resolved or (self.decision is None) == resolved:
            raise ValueError(
                "a Path is classified, with a decision, exactly when a non-flagged element is observed (7.1, 7.9)"
            )

    @property
    def is_tuple(self) -> bool:
        return self.kind == TUPLE

    @property
    def is_list(self) -> bool:
        return self.kind == LIST

    @property
    def elements_observed(self) -> bool:
        """Whether some non-flagged element is observed, so the Judgment_Call was resolved."""
        return self.lengths.elements_observed

    @property
    def constraint_keyword(self) -> str:
        """The keyword of the Tuple-or-List Constraint: ``prefixItems``, ``items``, or ``type`` (7.9)."""
        if self.kind is None:
            return _TYPE
        return _PREFIX_ITEMS if self.is_tuple else _ITEMS

    def details(self) -> dict[str, JSON]:
        """The judgment and the length statistics, or for 7.9 that no element was observed."""
        if self.kind is None:
            return {"elements_observed": False}
        return {"judgment": self.kind, "length_statistics": self.lengths.to_json()}

    def provenance(self) -> ProvenanceRecord:
        """The Provenance_Record of the Tuple-or-List Constraint of a resolved Path (7.1).

        Support is every non-flagged array at the Path, each of which the
        outcome describes; Confidence is the decision's (the Laplace default
        after a Fallback_Behavior).
        """
        if self.decision is None:
            raise ValueError(
                f"no element is observed at {render_path(self.path)}, so the Tuple-or-List Judgment_Call was not "
                "resolved (7.9); merge details() into the record of its type Constraint"
            )
        n = self.lengths.n_arrays
        return self.decision.provenance(Support(n, n), details=self.details())


# ---------------------------------------------------------------------------
# The Judgment_Call
# ---------------------------------------------------------------------------


def _check_engine(engine: DecisionEngine, scorer: MDLScorer | None) -> MDLScorer:
    if not isinstance(engine, DecisionEngine):
        raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
    if scorer is not None and not isinstance(scorer, MDLScorer):
        raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
    return scorer if scorer is not None else MDLScorer()


def _tuple_refutation(arrays: View) -> dict[str, JSON] | None:
    """The Document_Support facts that withhold the Tuple candidate (7.11), or None when it survives.

    A Tuple spans the maximum array length and closes it with ``items:
    false``. Inferred without a document, it would be shorter by the
    positions only that document's arrays reach, and reject them; and since
    an array reaching a position reaches every lower one, it is enough that
    two documents reach the last position. With one document nothing can
    be held out.
    """
    sizes = arrays.gather("size").astype(np.int64)
    docs = arrays.gather("doc").astype(np.int64)
    documents = int(len(np.unique(docs)))
    if documents < 2:
        return withheld(documents)
    # Each document's longest array; positions at or past the second-longest document's are reached by one document.
    order = np.argsort(docs, kind="stable")
    starts = np.flatnonzero(np.r_[True, np.diff(docs[order]) != 0])
    per_document = np.sort(np.maximum.reduceat(sizes[order], starts))
    single = int(per_document[-1] - per_document[-2])
    return withheld(documents, single_document_positions=single) if single else None


def tuple_or_list(
    view: View,
    engine: DecisionEngine,
    *,
    scorer: MDLScorer | None = None,
    features: JudgmentFeatures | None = None,
) -> ArrayOutcome:
    """Classify the arrays of ``view`` as Tuple or List (7.1, 7.10, 7.9); see the module docstring.

    ``engine`` resolves the Decision_Point and records it in its report.
    ``scorer`` scores both candidates (a default MDL_Scorer when None).
    ``features`` computes the Decision_Model features
    (:func:`tuple_list_features` by default). Raises ``ValueError`` when the
    view has no non-flagged array.
    """
    scorer = _check_engine(engine, scorer)
    arrays = _arrays_of(view)
    lengths = LengthStats.of(arrays)
    if not lengths.elements_observed:  # 7.9: neither keyword, without the Judgment_Call
        return ArrayOutcome(arrays.path, None, lengths)
    encodings = ((TUPLE, tuple_encoding(arrays)), (LIST, list_encoding(arrays)))
    refuted = _tuple_refutation(arrays)  # 7.11: Document_Support
    candidates = tuple(
        Candidate(
            technique=None,
            outcome=label,
            order_key=(index,),
            node=node,
            score=scorer.score(node, arrays),
            withheld=refuted if label == TUPLE else None,
        )
        for index, (label, node) in enumerate(encodings)
    )

    def featurize() -> np.ndarray:
        return (features or tuple_list_features)(arrays, candidates)

    decision = engine.resolve(DecisionPoint(TUPLE_OR_LIST, arrays.path), candidates, arrays, featurize)
    kind: ArrayKind = TUPLE if decision.label == TUPLE else LIST
    return ArrayOutcome(arrays.path, kind, lengths, decision)


# ---------------------------------------------------------------------------
# Array nodes
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class ArrayNode:
    """The array keywords of one node and their Provenance_Records (see the module docstring).

    ``node`` holds only array keywords; ``provenance`` maps each emitted
    Constraint keyword to its record. ``item_bounds`` is the item-count
    bound decision, and ``pattern`` the ``array_pattern`` resolution (the
    recorded Fallback_Behavior while the array grammar is disabled; None
    when no element is observed).
    """

    path: Path
    node: SchemaNode
    provenance: Mapping[str, ProvenanceRecord]
    outcome: ArrayOutcome
    item_bounds: BoundDecision
    pattern: Decision | None = None

    def keywords(self) -> dict[str, JSON]:
        """The emitted keywords as plain JSON."""
        schema = self.node.to_json()
        assert isinstance(schema, dict)
        return schema

    def constraint_provenance(self) -> dict[ConstraintId, ProvenanceRecord]:
        """The Provenance_Records keyed by the ConstraintIds of ``node``."""
        return {ConstraintId(self.node.nid, keyword): record for keyword, record in self.provenance.items()}

    def type_details(self) -> dict[str, JSON]:
        """What the ``type`` record of a Path with only empty arrays notes (7.9); ``{}`` otherwise.

        The analyzer merges these details into the ``type`` record of the
        ``type`` Baseline_Rule.
        """
        return {} if self.outcome.elements_observed else self.outcome.details()


class ArrayAnalyzer:
    """Tuple-or-List and array nodes for one run (see the module docstring).

    ``values`` is the run's ValueInferrer: its engine resolves every
    Decision_Point, and it supplies the item-count bounds. ``scorer``
    scores the Judgment_Call candidates (a new MDL_Scorer sharing the
    inferrer's Validator by default). ``tuple_list_features`` computes the
    Decision_Model features.
    """

    __slots__ = ("_features", "_scorer", "_values")

    def __init__(
        self,
        values: ValueInferrer,
        *,
        scorer: MDLScorer | None = None,
        tuple_list_features: JudgmentFeatures | None = None,
    ) -> None:
        if not isinstance(values, ValueInferrer):
            raise TypeError(f"values must be a ValueInferrer, got {type(values).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        self._values = values
        self._scorer = scorer if scorer is not None else MDLScorer(validator=values.validator)
        self._features = tuple_list_features

    @classmethod
    def for_techniques(
        cls,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
    ) -> ArrayAnalyzer:
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

    # -- steps --------------------------------------------------------------

    def tuple_or_list(self, view: View) -> ArrayOutcome:
        """:func:`tuple_or_list` with this run's engine and scorer."""
        return tuple_or_list(view, self.engine, scorer=self._scorer, features=self._features)

    def item_bounds(self, view: View) -> BoundDecision:
        """``minItems``/``maxItems`` over the lengths of the non-flagged arrays of ``view`` (16.4, 16.8)."""
        arrays = _arrays_of(view)
        support = QuantitySupport.of_array(arrays.gather("size"), arrays.gather("doc"))
        return self._values.decide_bound(arrays.stats().array_lengths, ITEM_COUNT, path=arrays.path, support=support)

    def _array_pattern(self, arrays: View, outcome: ArrayOutcome) -> Decision | None:
        """The ``array_pattern`` step (7.5–7.8, task 43.1) at a Path with a non-flagged element.

        While ``structure.array_grammar`` is disabled, the engine records
        the Fallback_Behavior, the schema of 7.2–7.4 (28.4). Task 43.1 adds
        the grammar candidates; a Technique enabled outside a validated
        run, where it is not implemented, offers none.
        """
        if not outcome.elements_observed:
            return None
        if self._values.runs(ARRAY_GRAMMAR):
            raise NotImplementedError(
                f"{ARRAY_GRAMMAR} is registered as implemented, but the ArrayAnalyzer has no step for it"
            )
        if self.engine.enabled(ARRAY_GRAMMAR):
            return None
        return self.engine.resolve(DecisionPoint(ARRAY_PATTERN, arrays.path), [], arrays)

    # -- nodes --------------------------------------------------------------

    def array_node(
        self,
        view: View,
        outcome: ArrayOutcome | None = None,
        *,
        child_schema: ChildSchema = naive_merge_node,
    ) -> ArrayNode:
        """The array keywords of the arrays of ``view`` (7.2–7.4, 7.9, 16.4), in the design walk's order.

        ``outcome`` is the Path's Tuple-or-List classification (resolved
        here when None). A List gets ``items`` from ``child_schema`` of every
        element; a Tuple gets ``prefixItems`` from ``child_schema`` of each
        position and ``items: false``; a Path with only empty arrays gets
        neither. The array grammar step and the item-count bounds follow.
        """
        arrays = _arrays_of(view)
        path = arrays.path
        if outcome is None:
            outcome = self.tuple_or_list(arrays)
        elif not isinstance(outcome, ArrayOutcome):
            raise TypeError(f"outcome must be an ArrayOutcome or None, got {type(outcome).__name__}")
        elif outcome.path != path:
            raise ValueError(f"the outcome is for {render_path(outcome.path)}, not for {render_path(path)}")
        kw: dict[str, object] = {}
        provenance: dict[str, ProvenanceRecord] = {}
        if outcome.is_list:
            kw[_ITEMS] = child_schema(arrays.elements())  # 7.2
            provenance[_ITEMS] = outcome.provenance()
        elif outcome.is_tuple:
            record = outcome.provenance()
            prefix = [child_schema(arrays.elements(position)) for position in range(outcome.lengths.max_length)]
            kw[_PREFIX_ITEMS] = prefix  # 7.3
            provenance[_PREFIX_ITEMS] = record
            if outcome.lengths.max_length <= len(prefix):  # 7.4: no array is longer than the prefix
                kw[_ITEMS] = SchemaNode(boolean=False)
                provenance[_ITEMS] = record
        pattern = self._array_pattern(arrays, outcome)
        bounds = self.item_bounds(arrays)
        records = bounds.provenance()
        for keyword, value in bounds.keywords().items():
            kw[keyword] = value
            provenance[keyword] = records[keyword]
        return ArrayNode(path, SchemaNode(kw=kw), provenance, outcome, bounds, pattern)
