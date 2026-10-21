"""ValueInferrer: the Value_Inferrer's steps at scalar views and Map keys (Requirements 4.4, 13–16).

Context
    The design's signatures take ``ctx``, the per-run ``InferenceContext``
    of ``pipeline.infer`` (task 16.2), which does not exist yet. The
    ValueInferrer therefore holds the per-run pieces it needs itself: the
    :class:`~jsinfer.decide.engine.DecisionEngine` (the Technique states,
    the Model_Bundle, and the report that receives every decision), the
    :class:`~jsinfer.io.validator.Validator`, the
    :class:`~jsinfer.core.report.InferenceReport` that receives rejected
    candidates, and an optional :class:`~jsinfer.mdl.scorer.MDLScorer`.
    ``InferenceContext`` builds one per run,
    ``ValueInferrer(ctx.engine, validator=ctx.validator, report=ctx.report)``,
    and the analyzers call ``infer_scalar(view)`` and ``key_constraint(view)``
    on it. :meth:`ValueInferrer.for_techniques` builds the whole set from
    Technique states, which is the entry point for tests and property tests.
    Pass the engine's own report as ``report`` so decisions and rejected
    candidates land in one Inference_Report.

Order at a scalar view (:meth:`ValueInferrer.infer_scalar`)
    Only the non-flagged scalar rows of the view count; arrays and objects
    at the same Path belong to the Structure_Analyzer.

    1. ``type`` (13.1, 13.2): :func:`~jsinfer.values.types.scalar_types`.
    2. Closed-or-Open-Enum (15): :func:`~jsinfer.values.enums.judge_enum`,
       whose open candidate uses type-only coding. A Closed_Enum emits
       ``const`` or ``enum``; an open judgment is reported by ``judge_enum``.
    3. String candidates, at a Path with at least one non-flagged string, in
       the registry priority order of ``string_constraint``: the Latent_Type
       candidate (13.3, ``values.latent``), one candidate per Format that at
       least one string satisfies (13.4, ``values.formats``), the
       Semantic_Type_Classifier's mapped Format or Pattern (13.5, 13.7), and
       Corpus_Advisor suggestions (24). The ``x-latent-type`` annotation is
       recorded whenever a Latent_Type is found, whichever candidate wins.
    4. Withholding (13.6): every candidate is checked against every distinct
       non-flagged string with the Validator, format assertion on (Format
       candidates reuse the detector's per-string verdicts). A candidate that
       rejects a string is withheld, and a ``formats`` entry with the Path,
       the candidate, the producing Technique, and the rejected counts (with
       multiplicity and distinct) goes to the Inference_Report.
    5. Selection (13.8, 24.7): the surviving candidates are resolved at one
       ``string_constraint`` Decision_Point by MDL. 13.8 defines the
       selection as an MDL comparison, so the engine is called with
       ``mdl_by`` set to the first surviving candidate's Technique (in effect
       by construction), whatever the state of ``decide.mdl``, as
       ``values.bounds`` and ``values.enum_estimation`` do. Every candidate
       is scored on the Path's string rows under ``{"type": "string"}`` plus
       its Constraint, so the scores differ only in how the candidates code
       the strings. The winner is the only Format or Pattern from these
       steps at the Path. Its Provenance_Record holds the resolution with
       every candidate and its score (26.3) and, in ``details``, the other
       satisfying candidates as ``alternatives``. Two Techniques may offer
       the same Constraint (the ``date`` Format from latent types and from
       Format detection). Only the first in registry priority order is
       offered to the engine, so identical Constraints do not split the MDL
       posterior; the others are listed as ``also_offered_by``.
    6. Pattern induction (14), at a Path with no selected Format that is not
       a Closed_Enum: FlashProfile-style candidates
       (``values.patterns.Induction``) over at most ``sample.patterns``
       distinct strings, each checked against every string and discarded
       with a report entry when it rejects one (14.7), at most
       ``patterns.attempt_budget`` of them (14.8, reported when reached).
       Each surviving rung is then checked across document folds
       (Document_Support, 14.9, :meth:`ValueInferrer._fold_refutations`):
       the Pattern induced at that rung from the other folds' strings must
       match every string only the fold holds, else the candidate is
       withheld with the facts in its report entry. With one document
       every induced candidate is withheld, and so it is when no fold
       holds a string of its own, since the induction cannot then be
       tested on any string it was not induced from (14.10, reported as
       ``untested``). The survivors compete by MDL
       with the option of no induced Pattern,
       ties going to no Pattern and then fewer sub-patterns (14.2). An
       induced Pattern that wins replaces a Latent_Type Pattern; otherwise
       the earlier selection stands.
    7. Bounds (16): ``minimum``/``maximum`` over the non-flagged numbers
       unless the Path is a Closed_Enum, and ``minLength``/``maxLength`` over
       the code-point lengths of the non-flagged strings unless the Path is
       a Closed_Enum or has a selected Format
       (:func:`~jsinfer.values.bounds.decide_bound`), each with the
       quantities' documents, so rungs that fail leave-one-document-out are
       withheld (Document_Support, 16.9).

Map keys (:meth:`ValueInferrer.key_constraint`, 4.4)
    The keys of the view's non-flagged objects, each counted once per object
    that has it, go through steps 3–5 with the Format candidates only
    (Latent_Types describe values, not keys). The candidate nodes are
    ``base`` (default ``{"type": "object"}``) plus
    ``propertyNames: {"format": f}``, scored on the view's object rows, so
    the MDL_Scorer codes the keys under each Format as it codes Map keys.
    Withheld key Formats are reported like value Formats, with
    ``"target": "keys"`` in their details. The selected Format becomes the
    Map's ``propertyNames`` subschema, which every key satisfies. Induced
    key Patterns (14.1) join here once pattern induction exists.

Technique states (13.9, 28.11)
    A step runs only when its Technique is registered as implemented and is
    enabled in the engine: a disabled Technique produces no candidate and,
    for latent types, no ``x-latent-type``, and the other steps run
    unchanged. ``values.semantic_type``, ``corpus.name_suggest``, and
    ``corpus.cf_suggest`` are not implemented yet, so their steps are
    skipped by registry state; each has a hook method
    (:meth:`ValueInferrer._semantic_candidates`,
    :meth:`ValueInferrer._corpus_candidates`) that the implementing task
    fills in, and which raises ``NotImplementedError`` if the registry
    marks the Technique implemented before the hook is written.

    The ``string_constraint`` Decision_Point exists at a Path with strings
    (or keys) when a candidate survives. When no serving Technique is
    enabled at all, it is resolved with no candidates, so the engine records
    the Fallback_Behavior "none" (28.4). When some serving Technique is
    enabled but every candidate was withheld or none was produced, no
    Decision_Point is resolved: the enabled Techniques ran and had nothing
    to offer, and the withheld candidates are in the report.

Results and provenance
    :class:`ScalarNode` holds the emitted keywords as a fresh
    :class:`~jsinfer.core.schema_model.SchemaNode` and one
    Provenance_Record per emitted keyword: ``type`` with the Laplace
    default Confidence (it has no alternative), ``enum``/``const`` from the
    Closed-or-Open-Enum decision with its statistics (15.8), ``format`` or
    ``pattern`` from the ``string_constraint`` decision, each bound from its
    ``bound`` decision, and ``x-latent-type`` with the Laplace default.
    :meth:`ScalarNode.constraint_provenance` keys them by ConstraintId for
    the node's nid, ready for provenance finalization (task 17.1). The
    step results (types, enum judgment, Latent_Type, string selection, bound
    decisions) stay available for inspection.

    ``type`` comes from the ``type`` Baseline_Rule of 13.1 and 13.2, which
    every run applies and no Technique serves, so its records name the
    Baseline_Rule marker :data:`TYPE_PRODUCER` (``baseline:type``, 26.13)
    with no resolution and no fallback indicator.

Determinism
    Every step reads integer counts over value and key ids, processes
    distinct strings in code-point order, and offers candidates in registry
    priority order, then documented candidate order, so results depend only
    on the multiset of non-flagged Observations. No decision cut-off is
    hard-coded here: candidates are kept or withheld by Validator verdicts
    and selected by the DecisionEngine.
"""

from __future__ import annotations

from collections.abc import Callable, Collection, Mapping, Sequence
from dataclasses import dataclass, field, replace
from typing import Any, Final, Literal, TypeAlias

import numpy as np

from jsinfer.config.registry import REGISTRY, OUTCOME_NONE, TIE_FEWER_SUBPATTERNS
from jsinfer.core.determinism import RANK_INTEGER, RANK_NUMBER
from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.paths import Path, render_path
from jsinfer.config.resources import ResourceReader
from jsinfer.core.determinism import rng as _rng
from jsinfer.core.provenance import ProvenanceRecord, SampleInfo, Support, baseline_technique
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.sampling import sample_rows
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.stats import Histogram
from jsinfer.core.views import View
from jsinfer.decide.engine import ArtifactSource, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.decide.support import QuantitySupport, document_folds
from jsinfer.decide.support import withheld as _withheld_facts
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.values.bounds import NUMBER, STRING_LENGTH, BoundDecision, Number, QuantityKind, QuantitySet, decide_bound
from jsinfer.values.enums import EnumFeatures, EnumJudgment, judge_enum
from jsinfer.values.patterns import Induction
from jsinfer.values.formats import REJECTED_CATEGORY, FormatCheck, FormatDetector, format_candidates
from jsinfer.values.latent import LatentDetection, detect_latent, view_strings
from jsinfer.values.types import ScalarTypes, scalar_types

__all__ = [
    "CORPUS_TECHNIQUES",
    "FORMAT",
    "LATENT_TYPE",
    "PATTERN",
    "SEMANTIC_TYPE",
    "STRING_CONSTRAINT",
    "TYPE_PRODUCER",
    "KeyConstraint",
    "ScalarNode",
    "StringCandidate",
    "StringConstraint",
    "StringSet",
    "ValueInferrer",
]

# The Decision_Point of every Format or Pattern choice and the Techniques that serve it.
STRING_CONSTRAINT: Final = "string_constraint"
LATENT_TYPE: Final = "values.latent_type"
FORMAT: Final = "values.format"
SEMANTIC_TYPE: Final = "values.semantic_type"
CORPUS_TECHNIQUES: Final = ("corpus.name_suggest", "corpus.cf_suggest")
PATTERN: Final = "values.pattern"
PATTERN_CATEGORY: Final = "patterns"  # the Inference_Report category of discarded induced candidates (14.7, 14.8)

# The producer named in the Provenance_Record of ``type``: the ``type`` Baseline_Rule (13.1, 13.2, 26.13).
TYPE_PRODUCER: Final = baseline_technique("type")

ConstraintKeyword: TypeAlias = Literal["format", "pattern"]
Target: TypeAlias = Literal["values", "keys"]

_STRING_BASE: Final[Mapping[str, JSON]] = {"type": "string"}
_OBJECT_BASE: Final[Mapping[str, JSON]] = {"type": "object"}
_SERVERS: Final = REGISTRY.decision_point(STRING_CONSTRAINT).served_by


# ---------------------------------------------------------------------------
# Strings and candidates
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class StringSet:
    """The distinct non-flagged strings (values or Map keys) of a view, in code-point order, with counts.

    ``total`` counts the strings with multiplicity (keys once per object
    that has them).
    """

    strings: tuple[str, ...]
    counts: tuple[int, ...]

    @classmethod
    def of_values(cls, view: View) -> StringSet:
        """The string values of the non-flagged rows of ``view``."""
        strings, counts = view_strings(view)
        return cls(tuple(strings), tuple(counts))

    @classmethod
    def of_keys(cls, view: View) -> StringSet:
        """The keys of the non-flagged objects of ``view``, each once per object that has it."""
        stats = view.stats()
        store = view.store
        # Key ids follow code-point order, so the names are already sorted.
        names = tuple(store.key_name(int(kid)) for kid in stats.key_ids.tolist())
        return cls(names, tuple(int(count) for count in stats.key_counts.tolist()))

    @property
    def total(self) -> int:
        return sum(self.counts)

    @property
    def n_distinct(self) -> int:
        return len(self.strings)

    def __len__(self) -> int:
        return len(self.strings)

    def rejections(self, keyword: ConstraintKeyword, value: str, validator: Validator) -> tuple[int, int]:
        """``(rejected, distinct rejected)`` strings of the Constraint ``{keyword: value}`` under the Validator."""
        if keyword == "format":
            accepts = validator.satisfies_format
        elif keyword == "pattern":
            accepts = validator.matches_pattern
        else:
            raise ValueError(f"a string Constraint is 'format' or 'pattern', got {keyword!r}")
        rejected = distinct = 0
        for s, count in zip(self.strings, self.counts):
            if not accepts(value, s):
                rejected += count
                distinct += 1
        return rejected, distinct


@dataclass(frozen=True, eq=False, slots=True)
class StringCandidate:
    """A candidate Format or Pattern at ``string_constraint``: the Constraint ``{keyword: value}``.

    ``technique`` produced it and ``order_key`` is its documented position
    among that Technique's candidates. ``rejected`` and
    ``distinct_rejected`` are the Validator's rejections over the Path's
    strings (13.6); a candidate with ``rejected > 0`` is withheld, and so is
    one with ``refuted`` set: the Document_Support facts of an induced
    Pattern that failed the document folds (14.9). ``details`` are facts its
    Provenance_Record carries when it is selected.
    """

    technique: str
    keyword: ConstraintKeyword
    value: str
    order_key: tuple[Any, ...]
    rejected: int = 0
    distinct_rejected: int = 0
    details: Mapping[str, JSON] = field(default_factory=dict)
    refuted: Mapping[str, JSON] | None = None

    def __post_init__(self) -> None:
        if self.technique not in _SERVERS:
            raise ValueError(f"{self.technique!r} does not serve {STRING_CONSTRAINT!r}")
        if self.keyword not in ("format", "pattern"):
            raise ValueError(f"a string Constraint is 'format' or 'pattern', got {self.keyword!r}")
        if not isinstance(self.value, str):
            raise TypeError(f"the {self.keyword} value must be a str, got {type(self.value).__name__}")

    @property
    def label(self) -> str:
        """The outcome label in Decision records: ``format=<name>`` or ``pattern=<source>``."""
        return f"{self.keyword}={self.value}"

    @property
    def constraint(self) -> dict[str, JSON]:
        """The Constraint as schema keywords, ``{"format": …}`` or ``{"pattern": …}``."""
        return {self.keyword: self.value}

    @property
    def withheld(self) -> bool:
        """Whether the candidate is withheld: it rejects a non-flagged string (13.6) or failed the document folds (14.9)."""
        return self.rejected > 0 or self.refuted is not None

    def sort_key(self) -> tuple[int, tuple[Any, ...]]:
        """Registry priority rank at ``string_constraint``, then the documented candidate order."""
        return (REGISTRY.rank(self.technique, STRING_CONSTRAINT), self.order_key)

    def to_candidate(self, node: SchemaNode) -> Candidate:
        """The DecisionEngine candidate whose partial schema is ``node``."""
        return Candidate(
            technique=self.technique, outcome=self.constraint, order_key=self.order_key, node=node, label=self.label
        )

    def rejection(self, path: Path, target: Target) -> RejectedCandidate:
        """The Inference_Report entry of this withheld candidate (13.6)."""
        if not self.withheld:
            raise ValueError(f"{self.label} rejects no string, so it is not withheld")
        details: dict[str, JSON] = {"rejected": self.rejected, "distinct_rejected": self.distinct_rejected}
        if self.refuted is not None:
            details["withheld"] = dict(self.refuted)
        if target == "keys":
            details["target"] = "keys"
        category = PATTERN_CATEGORY if self.technique == PATTERN else REJECTED_CATEGORY  # induced Patterns (14.7)
        return RejectedCandidate(category, render_path(path), self.constraint, self.technique, details)


def _from_format(check: FormatCheck) -> StringCandidate:
    return StringCandidate(FORMAT, "format", check.fmt, check.order_key, check.rejected, check.distinct_rejected)


def _distinct(candidates: Sequence[StringCandidate]) -> tuple[StringCandidate, ...]:
    """The first candidate of each distinct Constraint, keeping the given (canonical) order."""
    first: dict[tuple[str, str], StringCandidate] = {}
    for candidate in candidates:
        first.setdefault((candidate.keyword, candidate.value), candidate)
    return tuple(first.values())


@dataclass(frozen=True, eq=False)
class StringConstraint:
    """Steps 3–5 at one Path: the candidates, the withheld ones, and the ``string_constraint`` decision.

    ``candidates`` lists every candidate produced, in canonical order, and
    ``withheld`` the report entries of those that reject a string (13.6).
    ``decision`` is None when no Decision_Point was resolved (see the module
    docstring); ``selected`` is the winning candidate, None when nothing is
    emitted (no survivor, or the Fallback_Behavior "none").
    """

    path: Path
    target: Target
    strings: StringSet
    candidates: tuple[StringCandidate, ...]
    withheld: tuple[RejectedCandidate, ...]
    decision: Decision | None
    selected: StringCandidate | None
    sample: SampleInfo | None = None  # the induction sample behind a selected induced Pattern (14.7, 26.4)

    @property
    def survivors(self) -> tuple[StringCandidate, ...]:
        """The candidates that accept every non-flagged string, in canonical order."""
        return tuple(candidate for candidate in self.candidates if not candidate.withheld)

    @property
    def resolved(self) -> tuple[StringCandidate, ...]:
        """The survivors offered to the engine: the first, in canonical order, of each distinct Constraint."""
        return _distinct(self.survivors)

    def duplicates(self) -> tuple[StringCandidate, ...]:
        """Survivors that offered the selected Constraint after a higher-priority Technique did."""
        selected = self.selected
        if selected is None:
            return ()
        return tuple(
            candidate
            for candidate in self.survivors
            if candidate is not selected and candidate.constraint == selected.constraint
        )

    @property
    def keyword(self) -> ConstraintKeyword | None:
        return None if self.selected is None else self.selected.keyword

    @property
    def value(self) -> str | None:
        return None if self.selected is None else self.selected.value

    def keywords(self) -> dict[str, JSON]:
        """The selected Constraint, or ``{}``."""
        return {} if self.selected is None else self.selected.constraint

    def alternatives(self) -> list[dict[str, JSON]]:
        """The other surviving candidates (13.8), in canonical order, as ``{"outcome", "technique"}``."""
        return [
            {"outcome": candidate.label, "technique": candidate.technique}
            for candidate in self.resolved
            if candidate is not self.selected
        ]

    def provenance(self) -> ProvenanceRecord:
        """The Provenance_Record of the selected Constraint (13.8, 26.3).

        Support is every string, all of which the Constraint accepts;
        Confidence is the MDL posterior. ``details`` lists the alternatives,
        the lower-priority Techniques that offered the same Constraint
        (``also_offered_by``), and the selected candidate's own facts (for a
        Latent_Type candidate, its statistics).
        """
        if self.selected is None or self.decision is None:
            raise ValueError(f"no Format or Pattern was selected at {render_path(self.path)}")
        total = self.strings.total
        details: dict[str, JSON] = dict(self.selected.details)
        details["alternatives"] = self.alternatives()
        duplicates = self.duplicates()
        if duplicates:
            details["also_offered_by"] = [candidate.technique for candidate in duplicates]
        if self.target == "keys":
            details["target"] = "keys"
        return self.decision.provenance(Support(total, total), sample=self.sample, details=details)


# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class ScalarNode:
    """The Value_Inferrer's result at one scalar view (see the module docstring).

    ``node`` is a fresh SchemaNode with the emitted keywords and
    ``provenance`` maps each of them to its Provenance_Record. ``enum`` is
    the Closed-or-Open-Enum judgment; ``latent``, ``string``,
    ``number_bounds``, and ``length_bounds`` are None when their step did not
    apply (no strings, no numbers, or the Technique disabled for
    ``latent``).
    """

    path: Path
    node: SchemaNode
    provenance: Mapping[str, ProvenanceRecord]
    types: ScalarTypes
    enum: EnumJudgment
    latent: LatentDetection | None = None
    string: StringConstraint | None = None
    number_bounds: BoundDecision | None = None
    length_bounds: BoundDecision | None = None

    def keywords(self) -> dict[str, JSON]:
        """The emitted keywords as plain JSON."""
        schema = self.node.to_json()
        assert isinstance(schema, dict)
        return schema

    def constraint_provenance(self) -> dict[ConstraintId, ProvenanceRecord]:
        """The Provenance_Records keyed by the ConstraintIds of ``node``."""
        return {ConstraintId(self.node.nid, keyword): record for keyword, record in self.provenance.items()}


@dataclass(frozen=True, eq=False)
class KeyConstraint:
    """The ``propertyNames`` Constraint of a Map Path (4.4): a Format (or, after 14.1, a Pattern) every key satisfies."""

    selection: StringConstraint

    def __post_init__(self) -> None:
        if self.selection.selected is None or self.selection.target != "keys":
            raise ValueError("a KeyConstraint needs a selected Map-key candidate")

    @property
    def path(self) -> Path:
        return self.selection.path

    @property
    def candidate(self) -> StringCandidate:
        selected = self.selection.selected
        assert selected is not None
        return selected

    def subschema(self) -> SchemaNode:
        """A fresh ``propertyNames`` subschema, ``{"format": …}`` or ``{"pattern": …}``."""
        return SchemaNode(kw=self.candidate.constraint)

    def keywords(self) -> dict[str, SchemaNode]:
        """``{"propertyNames": subschema}`` for the Map node."""
        return {"propertyNames": self.subschema()}

    def provenance(self) -> ProvenanceRecord:
        """The Provenance_Record of the ``propertyNames`` Constraint."""
        return self.selection.provenance()


# ---------------------------------------------------------------------------
# The ValueInferrer
# ---------------------------------------------------------------------------


def _rows_of(view: View, jtype: JType) -> View:
    """The live rows of ``view`` of one JType."""
    live = view.live()
    mask = live.gather("jtype") == jtype
    return live if mask.all() else live.subset(mask)


def _numbers(view: View) -> tuple[list[Number], list[int]]:
    """The distinct non-flagged JSON numbers of ``view`` with their counts (canonical value order)."""
    stats = view.stats()
    table = view.store.values
    numbers: list[Number] = []
    counts: list[int] = []
    for vid, count in zip(stats.value_ids.tolist(), stats.value_counts.tolist()):
        rank, _ = table.key(vid)
        if rank == RANK_INTEGER or rank == RANK_NUMBER:
            value = table[vid]
            assert isinstance(value, (int, float)) and not isinstance(value, bool)
            numbers.append(value)
            counts.append(int(count))
    return numbers, counts


def _number_support(view: View) -> QuantitySupport:
    """The Document_Support of the non-flagged JSON numbers of ``view`` (16.9): rows ranked by exact value."""
    live = view.live()
    jtypes = live.gather("jtype")
    mask = (jtypes == JType.INTEGER) | (jtypes == JType.NUMBER)
    distinct, inverse = np.unique(live.gather("value")[mask], return_inverse=True)
    table = live.store.values
    values = [table[int(vid)] for vid in distinct.tolist()]
    order = sorted(range(len(values)), key=lambda i: values[i])  # exact int/float comparison
    rank_of = np.empty(len(values), dtype=np.int64)
    rank_of[order] = np.arange(len(values), dtype=np.int64)
    return QuantitySupport.of([values[i] for i in order], rank_of[inverse.reshape(-1)], live.gather("doc")[mask])


def _string_documents(view: View, target: Target, wanted: Collection[str]) -> dict[str, set[int]]:
    """The documents holding each of the ``wanted`` strings: string values of ``view``, or its objects' keys."""
    live = view.live()
    store = live.store
    if target == "keys":
        members = live.child_map()
        ids, docs = members.gather("key"), members.gather("doc")
        name: Callable[[int], Any] = store.key_name
    else:
        rows = _rows_of(live, JType.STRING)
        ids, docs = rows.gather("value"), rows.gather("doc")
        name = store.values.__getitem__
    found: dict[str, set[int]] = {}
    if len(ids):
        wanted = set(wanted)
        span = max(store.n_docs, int(docs.max()) + 1)
        pairs = np.unique(ids.astype(np.int64) * span + docs.astype(np.int64))
        for sid, doc in zip((pairs // span).tolist(), (pairs % span).tolist()):
            s = name(int(sid))
            if s in wanted:
                found.setdefault(s, set()).add(int(doc))
    return found


def _base_keywords(base: Mapping[str, Any] | None, default: Mapping[str, JSON], forbidden: str) -> dict[str, Any]:
    if base is None:
        return dict(default)
    if not isinstance(base, Mapping):
        raise TypeError(f"base must be a mapping of schema keywords, got {type(base).__name__}")
    if forbidden in base:
        raise ValueError(f"base must not hold {forbidden!r}, which is being decided")
    return dict(base)


class ValueInferrer:
    """The Value_Inferrer of one run (see the module docstring).

    ``engine`` resolves every Decision_Point and records it in its report.
    ``validator`` decides every Format and Pattern check (a new Validator by
    default). ``report`` receives withheld candidates and open enum
    judgments; pass the engine's report. ``scorer`` scores candidates (the
    engine's MDL_Scorer by default).
    """

    __slots__ = ("_detector", "_engine", "_enum_features", "_report", "_resources", "_scorer", "_seed", "_validator")

    def __init__(
        self,
        engine: DecisionEngine,
        *,
        validator: Validator | None = None,
        report: InferenceReport | None = None,
        scorer: MDLScorer | None = None,
        enum_features: EnumFeatures | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = 0,
    ) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        if validator is not None and not isinstance(validator, Validator):
            raise TypeError(f"validator must be a Validator or None, got {type(validator).__name__}")
        if report is not None and not isinstance(report, InferenceReport):
            raise TypeError(f"report must be an InferenceReport or None, got {type(report).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        self._engine = engine
        self._validator = validator if validator is not None else Validator()
        self._report = report
        self._scorer = scorer
        if enum_features is not None and not callable(enum_features):
            raise TypeError("enum_features must be callable or None")
        self._enum_features = enum_features
        self._detector: FormatDetector | None = None
        self._resources = ResourceReader.of(resources)
        if isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
            raise ValueError(f"seed must be a non-negative int, got {seed!r}")
        self._seed = seed

    @property
    def enum_features(self) -> EnumFeatures | None:
        """The Closed-or-Open-Enum featurizer passed to ``judge_enum`` (the six statistics when None)."""
        return self._enum_features

    @classmethod
    def for_techniques(
        cls,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = 0,
    ) -> ValueInferrer:
        """An inferrer whose engine has the Technique states ``techniques`` (omitted ids take their default).

        The engine, the MDL_Scorer, and the inferrer share ``validator`` and
        ``report``; ``bundle`` supplies Decision_Models and Learned_Thresholds;
        ``resources`` and ``seed`` feed pattern induction (14.8, 29.8).
        """
        validator = validator if validator is not None else Validator()
        scorer = MDLScorer(validator=validator)
        engine = DecisionEngine(techniques, bundle=bundle, report=report, scorer=scorer)
        return cls(engine, validator=validator, report=report, scorer=scorer, resources=resources, seed=seed)

    @property
    def engine(self) -> DecisionEngine:
        return self._engine

    @property
    def validator(self) -> Validator:
        return self._validator

    @property
    def resources(self) -> ResourceReader:
        return self._resources

    @property
    def seed(self) -> int:
        return self._seed

    @property
    def report(self) -> InferenceReport | None:
        return self._report

    # -- Technique states ---------------------------------------------------

    def runs(self, tid: str) -> bool:
        """Whether Technique ``tid``'s step runs: registered as implemented and enabled in the engine."""
        return REGISTRY.technique(tid).implemented and self._engine.enabled(tid)

    def _any_server_enabled(self) -> bool:
        return any(self._engine.enabled(tid) for tid in _SERVERS)

    def _planned(self, tid: str) -> list[StringCandidate]:
        """The hook of a Technique whose step is not written yet: no candidates while it is unimplemented."""
        if not self.runs(tid):
            return []
        raise NotImplementedError(f"{tid} is registered as implemented, but the ValueInferrer has no step for it")

    def _detector_for(self, view: View) -> FormatDetector:
        """The FormatDetector of the view's store (one per store, reused across its views)."""
        detector = self._detector
        if detector is None or detector.store is not view.store:
            detector = self._detector = FormatDetector(view.store, self._validator)
        return detector

    # -- scalar views -------------------------------------------------------

    def infer_scalar(self, view: View) -> ScalarNode:
        """The keywords and Provenance_Records of the scalar rows of ``view``, in design order."""
        if not isinstance(view, View):
            raise TypeError(f"infer_scalar takes a View, got {type(view).__name__}")
        view = view.live()
        path = view.path
        kw: dict[str, Any] = {}
        provenance: dict[str, ProvenanceRecord] = {}

        # 1. type (13.1, 13.2): no alternative, so the Laplace default Confidence.
        types = scalar_types(view)
        if not types.is_empty:
            kw["type"] = types.keyword_value
            provenance["type"] = ProvenanceRecord.laplace(TYPE_PRODUCER, types.support)

        # 2. Closed-or-Open-Enum (15); judge_enum reports an open judgment itself.
        judgment = judge_enum(
            view, self._engine, report=self._report, scorer=self._scorer, features=self._enum_features
        )
        if judgment.closed:
            record = judgment.provenance()
            for keyword, value in judgment.keywords().items():
                kw[keyword] = value
                provenance[keyword] = record

        # 3–6. Latent_Type, string candidates, withholding, selection, pattern induction.
        latent: LatentDetection | None = None
        string: StringConstraint | None = None
        if types.count("string"):
            strings = StringSet.of_values(view)
            latent = detect_latent(strings.strings, self._validator, counts=strings.counts, enabled=self.runs(LATENT_TYPE))
            offered = self._value_candidates(view, strings, latent)
            string = self._select(view, _rows_of(view, JType.STRING), strings, offered, "values", self._value_node)
            if string.keyword != "format" and not judgment.closed:
                string = self._pattern_step(  # 6. (14.1)
                    view, strings, string, _rows_of(view, JType.STRING), self._value_node, SchemaNode.from_json({**_STRING_BASE, **string.keywords()})
                )
            for keyword, value in string.keywords().items():
                kw[keyword] = value
                provenance[keyword] = string.provenance()

        # 7. Bounds (16).
        number_bounds: BoundDecision | None = None
        numbers, counts = _numbers(view)
        if numbers:
            number_bounds = self.decide_bound(
                numbers, NUMBER, counts=counts, path=path, closed_enum=judgment.closed, support=_number_support(view)
            )
            self._emit_bounds(number_bounds, kw, provenance)
        length_bounds: BoundDecision | None = None
        lengths = view.stats().string_lengths
        if lengths.total:
            string_rows = _rows_of(view, JType.STRING)
            length_bounds = self.decide_bound(
                lengths,
                STRING_LENGTH,
                path=path,
                closed_enum=judgment.closed,
                format_selected=string is not None and string.keyword == "format",
                support=QuantitySupport.of_array(string_rows.gather("size"), string_rows.gather("doc")),
            )
            self._emit_bounds(length_bounds, kw, provenance)

        # The x-latent-type annotation, whichever candidate won (13.3).
        if latent is not None:
            keyword, value = latent.annotation
            kw[keyword] = value
            provenance[keyword] = ProvenanceRecord.laplace(LATENT_TYPE, latent.support, details=latent.details())

        return ScalarNode(
            path=path,
            node=SchemaNode(kw=kw),
            provenance=provenance,
            types=types,
            enum=judgment,
            latent=latent,
            string=string,
            number_bounds=number_bounds,
            length_bounds=length_bounds,
        )

    @staticmethod
    def _emit_bounds(decision: BoundDecision, kw: dict[str, Any], provenance: dict[str, ProvenanceRecord]) -> None:
        records = decision.provenance()
        for keyword, value in decision.keywords().items():
            kw[keyword] = value
            provenance[keyword] = records[keyword]

    def _value_candidates(
        self, view: View, strings: StringSet, latent: LatentDetection | None
    ) -> list[StringCandidate]:
        """Step 3 at string values (13.3–13.5, 24), each candidate with its Validator rejections (13.6)."""
        offered: list[StringCandidate] = []
        if latent is not None:
            rejected, distinct = strings.rejections(latent.keyword, latent.value, self._validator)
            offered.append(
                StringCandidate(
                    LATENT_TYPE, latent.keyword, latent.value, (0,), rejected, distinct, latent.details()
                )
            )
        if self.runs(FORMAT):
            checks = format_candidates(self._detector_for(view), view, self._engine.techniques)
            offered.extend(_from_format(check) for check in checks)
        offered.extend(self._semantic_candidates(view, strings))
        offered.extend(self._corpus_candidates(view, strings))
        return offered

    @staticmethod
    def _value_node(candidate: StringCandidate) -> SchemaNode:
        return SchemaNode.from_json({**_STRING_BASE, **candidate.constraint})

    # -- hooks of unimplemented Techniques ----------------------------------

    def _semantic_candidates(self, view: View, strings: StringSet) -> list[StringCandidate]:
        """13.5, 13.7 (task 45.1): the Format or Pattern mapped from the predicted semantic type."""
        return self._planned(SEMANTIC_TYPE)

    def _corpus_candidates(self, view: View, strings: StringSet) -> list[StringCandidate]:
        """24 (task 40.2): accepted Corpus_Advisor ``format``/``pattern`` suggestions."""
        found: list[StringCandidate] = []
        for tid in CORPUS_TECHNIQUES:
            found.extend(self._planned(tid))
        return found

    def _pattern_step(
        self,
        view: View,
        strings: StringSet,
        selection: StringConstraint,
        scored: View,
        make_node: Callable[[StringCandidate], SchemaNode],
        none_node: SchemaNode,
    ) -> StringConstraint:
        """Step 6 (14): Pattern induction at a Path (or Map keys) with no selected Format.

        The caller has checked the Path is not a Closed_Enum. Candidates
        come from :class:`~jsinfer.values.patterns.Induction` over at most
        ``sample.patterns`` distinct strings (sampled with
        ``rng("values.pattern", path)``), each checked against every
        non-flagged string with the Validator: a rejecting candidate is
        discarded and reported with the number of non-matching strings
        (14.7). Induction stops at ``patterns.attempt_budget`` candidates,
        which the report records (14.8). The survivors and the option of
        no induced Pattern (``none_node``, the current selection) are
        resolved by MDL on ``scored``; ties favor no Pattern, then fewer
        sub-patterns (14.2). ``selection`` stands when nothing replaces it.
        """
        if not self.runs(PATTERN) or not strings.strings:
            return selection
        path = view.path
        max_alternatives = int(self._resources.read("patterns.max_alternatives"))
        budget = int(self._resources.read("patterns.attempt_budget"))
        sample_size = int(self._resources.read("sample.patterns"))
        sample: SampleInfo | None = None
        induced_from = strings.strings
        induced_counts: tuple[int, ...] = strings.counts
        if strings.n_distinct > sample_size:
            indices = sorted(sample_rows(list(range(strings.n_distinct)), sample_size, _rng(PATTERN, render_path(path), seed=self._seed)))
            induced_from = tuple(strings.strings[i] for i in indices)
            induced_counts = tuple(strings.counts[i] for i in indices)
            sample = SampleInfo(sample_size, strings.n_distinct)
        offered: list[StringCandidate] = []
        evaluated = 0
        exhausted = False
        for index, candidate in enumerate(Induction(induced_from, induced_counts, max_alternatives=max_alternatives).candidates()):
            if evaluated >= budget:
                exhausted = True
                break
            evaluated += 1
            rejected, distinct = strings.rejections("pattern", candidate.pattern.source, self._validator)
            details: dict[str, JSON] = {"rung": candidate.rung, "sub_patterns": candidate.sub_patterns, "merged_profiles": candidate.merged}
            offered.append(StringCandidate(PATTERN, "pattern", candidate.pattern.source, (index,), rejected, distinct, details))
        # 14.9: Document_Support. A candidate's rung must generalize across the document folds.
        refutations = self._fold_refutations(view, selection.target, induced_from, induced_counts, offered, max_alternatives)
        if refutations:
            offered = [
                c if c.withheld or str(c.details["rung"]) not in refutations else replace(c, refuted=refutations[str(c.details["rung"])])
                for c in offered
            ]
        withheld: list[RejectedCandidate] = list(selection.withheld)
        survivors: list[StringCandidate] = []
        for candidate in offered:
            if candidate.withheld:
                entry = candidate.rejection(path, selection.target)
                withheld.append(entry)
                if self._report is not None:
                    self._report.record_rejected(entry)
            else:
                survivors.append(candidate)
        if exhausted and self._report is not None:
            budget_entry = RejectedCandidate(
                PATTERN_CATEGORY, render_path(path), {"attempt_budget": budget}, PATTERN,
                {"budget_exhausted": True, "evaluated": evaluated, **({"target": "keys"} if selection.target == "keys" else {})},
            )
            self._report.record_rejected(budget_entry)
            withheld.append(budget_entry)
        candidates = (*selection.candidates, *offered)
        if not survivors:
            return StringConstraint(path, selection.target, strings, candidates, tuple(withheld), selection.decision, selection.selected, selection.sample)
        none = Candidate(technique=PATTERN, outcome=OUTCOME_NONE, order_key=(-1,), node=none_node, label=OUTCOME_NONE, tie_ranks={TIE_FEWER_SUBPATTERNS: 0})
        pairs = [(sc, Candidate(technique=PATTERN, outcome=sc.constraint, order_key=sc.order_key, node=make_node(sc), label=sc.label, tie_ranks={TIE_FEWER_SUBPATTERNS: int(sc.details["sub_patterns"])})) for sc in _distinct(survivors)]
        decision = self._engine.resolve(DecisionPoint(STRING_CONSTRAINT, path), [none, *(ec for _, ec in pairs)], scored, mdl_by=PATTERN, scorer=self._scorer)
        selected = next((sc for sc, ec in pairs if ec is decision.candidate), None)
        if selected is None:  # no induced Pattern (14.2): the earlier selection stands, with the induced candidates recorded
            kept = selection.decision if selection.selected is not None else decision
            return StringConstraint(path, selection.target, strings, candidates, tuple(withheld), kept, selection.selected, selection.sample)
        return StringConstraint(path, selection.target, strings, candidates, tuple(withheld), decision, selected, sample)

    def _fold_refutations(
        self,
        view: View,
        target: Target,
        strings: Sequence[str],
        counts: Sequence[int],
        offered: Sequence[StringCandidate],
        max_alternatives: int,
    ) -> dict[str, dict[str, JSON]]:
        """14.9, 14.10: per rung of the surviving induced candidates, the Document_Support facts that refute it.

        The documents behind ``strings`` (the induction set) are dealt into
        ``document_folds``. For each fold, the strings only that fold holds
        are held out and the ladder is induced from the strings some other
        fold holds; a rung is refuted when its induced Pattern is missing or
        fails to match a held-out string. With one document nothing can be
        held out and every rung is refuted. When no fold holds a string of
        its own, every string recurs across folds and no rung can be tested
        on a string it was not induced from, so every rung is refuted as
        ``untested`` (14.10). Rungs that already reject a string are not
        checked.
        """
        rungs = sorted({str(c.details["rung"]) for c in offered if not c.withheld})
        if not rungs:
            return {}
        by_string = _string_documents(view, target, strings)
        held_by: dict[int, list[str]] = {}
        for s, docs in by_string.items():
            for d in docs:
                held_by.setdefault(d, []).append(s)
        if len(held_by) < 2:
            facts = _withheld_facts(len(held_by))
            return {rung: facts for rung in rungs}
        # Folds are dealt in content order, so they do not depend on document order (27.6).
        documents = sorted(held_by, key=lambda d: (sorted(held_by[d]), d))
        fold_of = dict(zip(documents, document_folds(len(documents)).tolist()))
        count_of = dict(zip(strings, counts))
        folds_of = {s: {fold_of[d] for d in by_string.get(s, ())} for s in strings}
        if not any(len(folds) == 1 for folds in folds_of.values()):  # 14.10: nothing can be held out
            facts = _withheld_facts(len(documents), held_out=0, untested=True)
            return {rung: facts for rung in rungs}
        refuted: dict[str, dict[str, JSON]] = {}
        for fold in range(max(fold_of.values()) + 1):
            held = [s for s in strings if folds_of[s] == {fold}]
            if not held:
                continue
            train = [s for s in strings if any(f != fold for f in folds_of[s])]
            ladder: dict[str, str] = {}
            if train:
                induction = Induction(tuple(train), tuple(count_of[s] for s in train), max_alternatives=max_alternatives)
                for candidate in induction.candidates():
                    ladder.setdefault(candidate.rung, candidate.pattern.source)
            for rung in rungs:
                if rung in refuted:
                    continue
                source = ladder.get(rung)
                if source is None:
                    unmatched = len(held)
                else:
                    unmatched = sum(1 for s in held if not self._validator.matches_pattern(source, s))
                if unmatched:
                    refuted[rung] = _withheld_facts(len(documents), fold=fold, held_out=len(held), unmatched=unmatched)
        return refuted

    # -- selection (13.6, 13.8) ---------------------------------------------

    def _select(
        self,
        view: View,
        scored: View,
        strings: StringSet,
        offered: Sequence[StringCandidate],
        target: Target,
        make_node: Callable[[StringCandidate], SchemaNode],
    ) -> StringConstraint:
        """Withhold rejecting candidates (13.6) and resolve the survivors by MDL (13.8) on ``scored``."""
        path = view.path
        ordered = tuple(sorted(offered, key=StringCandidate.sort_key))
        withheld: list[RejectedCandidate] = []
        survivors: list[StringCandidate] = []
        for candidate in ordered:
            if candidate.withheld:
                entry = candidate.rejection(path, target)
                withheld.append(entry)
                if self._report is not None:
                    self._report.record_rejected(entry)
            else:
                survivors.append(candidate)

        dp = DecisionPoint(STRING_CONSTRAINT, path)
        decision: Decision | None = None
        selected: StringCandidate | None = None
        if survivors:
            pairs = [(candidate, candidate.to_candidate(make_node(candidate))) for candidate in _distinct(survivors)]
            decision = self._engine.resolve(
                dp,
                [engine_candidate for _, engine_candidate in pairs],
                scored,
                mdl_by=survivors[0].technique,  # 13.8 selects by MDL; the first survivor's Technique is in effect
                scorer=self._scorer,
            )
            selected = next((sc for sc, ec in pairs if ec is decision.candidate), None)
        elif not self._any_server_enabled():
            decision = self._engine.resolve(dp, [], scored)  # 28.4: the Fallback_Behavior "none" is recorded
        return StringConstraint(path, target, strings, ordered, tuple(withheld), decision, selected)

    # -- Map keys (4.4) -----------------------------------------------------

    def key_constraint(
        self,
        view: View,
        *,
        base: Mapping[str, Any] | None = None,
        keys: Collection[str] | None = None,
        make_node: Callable[[StringCandidate], SchemaNode] | None = None,
        none_node: SchemaNode | None = None,
    ) -> KeyConstraint | None:
        """The ``propertyNames`` Constraint of the keys of the objects of the Map view ``view`` (4.4).

        ``base`` holds the Map node's other keywords for scoring (default
        ``{"type": "object"}``). Returns None when the objects have no keys
        or no candidate is selected; withheld candidates and any
        Fallback_Behavior are recorded as for string values.

        With ``keys``, only those keys are the strings (the Map entries of a
        hybrid partition, 4.6): Formats are not offered, since the Pattern
        keys ``patternProperties``, and ``make_node`` / ``none_node`` give
        the nodes scored with and without a candidate Pattern.
        """
        if not isinstance(view, View):
            raise TypeError(f"key_constraint takes a View, got {type(view).__name__}")
        view = view.live()
        base_kw = _base_keywords(base, _OBJECT_BASE, "propertyNames")
        strings = StringSet.of_keys(view)
        if keys is not None:
            wanted = set(keys)
            pairs = [(s, c) for s, c in zip(strings.strings, strings.counts) if s in wanted]
            strings = StringSet(tuple(s for s, _ in pairs), tuple(c for _, c in pairs))
        if not strings.total:
            return None
        offered: list[StringCandidate] = []
        if self.runs(FORMAT) and keys is None:
            checks = format_candidates(self._detector_for(view), view, self._engine.techniques, keys=True)
            offered.extend(_from_format(check) for check in checks)

        def names_node(candidate: StringCandidate) -> SchemaNode:
            return SchemaNode.from_json({**base_kw, "propertyNames": candidate.constraint})

        build = make_node if make_node is not None else names_node
        without = none_node if none_node is not None else SchemaNode.from_json(dict(base_kw))
        selection = self._select(view, _rows_of(view, JType.OBJECT), strings, offered, "keys", build)
        if selection.keyword != "format":
            selection = self._pattern_step(view, strings, selection, _rows_of(view, JType.OBJECT), build, without)  # 14.1
        return None if selection.selected is None else KeyConstraint(selection)

    # -- bounds (16) --------------------------------------------------------

    def decide_bound(
        self,
        quantities: Histogram | Sequence[Number] | np.ndarray | QuantitySet,
        kind: QuantityKind,
        *,
        counts: Sequence[int] | np.ndarray | None = None,
        path: Path | str | None = None,
        base: Mapping[str, Any] | SchemaNode | None = None,
        closed_enum: bool = False,
        format_selected: bool = False,
        support: QuantitySupport | None = None,
    ) -> BoundDecision:
        """Both bounds of ``quantities`` through this run's engine (16.2, 16.7, 16.9); see ``values.bounds.decide_bound``."""
        return decide_bound(
            quantities,
            kind,
            self._engine,
            counts=counts,
            path=path,
            base=base,
            closed_enum=closed_enum,
            format_selected=format_selected,
            support=support,
        )

