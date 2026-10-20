"""The Structure_Analyzer walk: one top-down pass from a view to a Schema_Model (Requirements 4–9, 26.13).

:meth:`StructureAnalyzer.analyze` dispatches on the JSON types of the
non-flagged rows of a view (design, Per-segment inference walk) and recurses
into child views through the components' ``child_schema`` and
``variant_schema`` callbacks. :meth:`StructureAnalyzer.infer_model` runs the
walk from a root view and returns the :class:`~jsinfer.core.schema_model.SchemaModel`
with the Provenance_Record of every Constraint it emitted, keyed by
:class:`~jsinfer.core.schema_model.ConstraintId`.

One run, one set of components
    The analyzer shares one :class:`~jsinfer.decide.engine.DecisionEngine`
    (Technique states, Model_Bundle, and the Inference_Report that receives
    every decision), one :class:`~jsinfer.io.validator.Validator`, and one
    :class:`~jsinfer.mdl.scorer.MDLScorer` across its components: the
    :class:`~jsinfer.values.inferrer.ValueInferrer`, the
    :class:`~jsinfer.structure.record_map.RecordMapAnalyzer`, the
    :class:`~jsinfer.structure.variants.VariantAnalyzer`, the
    :class:`~jsinfer.structure.arrays.ArrayAnalyzer`, and the
    :class:`~jsinfer.deps.cooccurrence.CooccurrenceMiner`. Build it with
    :meth:`StructureAnalyzer.for_run` from a validated Run_Configuration
    (the entry point of ``pipeline.infer``, task 16.2), or with
    :meth:`StructureAnalyzer.for_techniques` from Technique states (tests).
    Components passed to the constructor must use the ValueInferrer's
    engine, which lets callers supply Decision_Model featurizers (task 22.1)
    or stub scorers.

The walk (:meth:`StructureAnalyzer.analyze`)
    A view with no non-flagged row gets ``{}``, as under Naive_Merge. Otherwise
    each JSON-type family present builds one *part*, a schema object with its
    own ``type``:

    1. Scalars (null, boolean, integer, number, string rows):
       :meth:`ValueInferrer.infer_scalar` on the scalar rows, which emits
       ``type``, a Closed_Enum, the Format or Pattern, the bounds, and
       ``x-latent-type``.
    2. Objects (design walk, step 2), on the object rows:

       1. Record-or-Map (:meth:`RecordMapAnalyzer.record_or_map`), then the
          hybrid partition hook (4.5).
       2. A Map is :meth:`RecordMapAnalyzer.map_node` with ``analyze`` as
          ``child_schema``: one ``additionalProperties`` value schema,
          ``propertyNames``, and the key-count bounds (4.3, 4.4, 16.5). It
          has no Variants, ``required``, or rules.
       3. A Record resolves Closed-or-Open-Record over the Path's objects
          (4.7), selects a Variant grouping
          (:meth:`VariantAnalyzer.select`), resolves Merge-or-Split
          (:meth:`VariantAnalyzer.merge_or_split`), passes the Discriminator
          hook (10), and emits the final Variants with
          :meth:`VariantAnalyzer.branches`. Each final Variant's object
          schema is built by :meth:`StructureAnalyzer._variant_node`:
          :meth:`RecordMapAnalyzer.record_node` over the Variant's objects
          with ``analyze`` as ``child_schema`` (the per-branch recursion,
          4.2), ``type: ["object"]``, ``required`` from
          :func:`~jsinfer.deps.presence.required_fields` (9.7), the
          Field_Group hook (6), and :meth:`CooccurrenceMiner.rules` scored
          against that node (9.3–9.6, 9.8, 9.9). A closed Record therefore
          puts ``additionalProperties: false`` inside each Variant branch,
          never beside the ``anyOf``/``oneOf`` (design, Open Decisions). The
          Exclusion_Rule hook (11) follows the branches.
    3. Arrays (design walk, step 3): :meth:`ArrayAnalyzer.array_node` with
       ``analyze`` as ``child_schema`` (Tuple-or-List, element or position
       recursion, the array grammar step, item-count bounds) plus
       ``type: ["array"]``.

Combining the parts (design walk, step 4)
    - One part: it is the node.
    - Several parts and no more than one object branch: the parts' keywords
      coexist in one schema object, as in Naive_Merge, with one ``type``
      listing every family's names in emitted order (the integer rule
      applies among the scalars). No two families emit the same keyword:
      scalar, array, and object keywords apply only to their own types.
    - Two or more object branches (a split Record): with objects only, the
      node is the Variant node as built, ``anyOf`` or ``oneOf`` by 5.5.
      With other types too, the node is an ``anyOf`` of the Variant
      branches followed by one branch holding the other families' parts,
      merged as above. The other-types branch holds no object keywords, so
      it requires nothing and permits every field: no pair with it is
      disjoint under the reading of 5.5 in the design's Open Decisions, and
      ``anyOf`` is what that reading gives for the emitted branches.
    - A Closed_Enum beside other types. ``enum`` and ``const`` are the only
      scalar keywords that apply to every instance type, and they list only
      scalar values (15.3), so in a shared schema object they would reject
      the Path's arrays and objects. The scalar part then becomes its own
      ``anyOf`` branch, placed after any Variant branches and before the
      branch of the remaining families (JSON type order). The scalar branch
      requires nothing and permits every field, so ``anyOf`` again agrees
      with 5.5. This extends the design's step 4, which does not name the
      case.

Records
    Every Constraint the walk emits gets exactly one Provenance_Record,
    collected by node nid while the walk runs and keyed by ConstraintId in
    the returned model (26.1). When parts merge into one schema object,
    their records move to the merged node's nid.

    - ``type`` on scalar, object, array, and mixed nodes (and on each
      Variant branch) names the ``type`` Baseline_Rule, ``baseline:type``,
      with the Laplace Confidence over the node's rows and no fallback or
      resolution (26.13). A Record with no keys (4.11) merges
      :meth:`ObjectNode.type_details` (``"keys_available": false``) into
      it, and a Path with only empty arrays (7.9) merges
      :meth:`ArrayNode.type_details` (``"elements_observed": false``).
    - Scalar, object, array, ``required``, and ``dependentRequired``
      records are the components' own (``constraint_provenance``).
    - Variant branches carry the Merge-or-Split record of
      :meth:`MergeSplitOutcome.branch_provenance` with the node's
      combinator; in a mixed node their Support total is every row at the
      Path. The other-types branch of a split Path carries
      :meth:`MergeSplitOutcome.remaining_provenance`, and the branches made
      by a Closed_Enum beside other types carry the Closed-or-Open-Enum
      decision, with the branch's rows among the Path's rows as Support and
      ``"branch"`` (``"closed_enum"`` or ``"remaining_types"``) and
      ``"types"`` in the details. Provenance finalization (task 17.1)
      recomputes every Support after the soundness check.

Every Technique off (28.5, Property 49)
    Each Decision_Point then takes its Fallback_Behavior: Record, open, one
    Variant, List, no enum, Format, bounds, or rules. The walk then emits,
    view by view, exactly the Naive_Merge subschema: ``type`` with the
    integer rule, ``properties`` and ``required`` over every object,
    ``items`` over every element (none for only empty arrays), all in one
    schema object, and ``{}`` for an empty view.

Hooks (Techniques the walk reaches that are not implemented yet)
    Each hook is skipped by registry state, as in the components. While
    every serving Technique is disabled, the hook resolves its
    Decision_Point with no candidates, so the engine records the
    Fallback_Behavior in the report (28.4); a serving Technique enabled
    outside a validated run (where it is not implemented) offers nothing;
    and once the registry marks one implemented, the hook raises
    ``NotImplementedError`` until its task fills it in.

    - :meth:`StructureAnalyzer._record_map_partition` (4.5, 4.6,
      ``structure.hybrid_record_map``, task 37.1), at an object Path with
      keys: the partition replaces the Record-or-Map outcome, its Record
      fields go to :meth:`RecordMapAnalyzer.closed_or_open` as ``record``,
      and its Map entries to ``patternProperties`` or
      ``additionalProperties``.
    - :meth:`StructureAnalyzer._field_groups` (6, ``groups.fpgrowth_slim``
      through :class:`~jsinfer.structure.field_groups.FieldGroupMiner`;
      ``groups.bmf`` joins in task 47.1), per final Variant with a field:
      the selected Field_Groups go to :meth:`CooccurrenceMiner.rules` as
      ``groups``, their ``dependentRequired`` cliques merge with the
      Co_Occurrence_Rule entries of the same antecedent into one entry, and
      an entry both produced gets one Provenance_Record
      (:func:`~jsinfer.structure.field_groups.merge_records`).
    - :meth:`StructureAnalyzer._discriminator` (10, ``rules.decision_trees``,
      task 34.1), at a Record Path with keys after Merge-or-Split: a
      selected Discriminator's value groups replace the Variant branches.
    - :meth:`StructureAnalyzer._exclusions` (11, ``rules.exclusion``, task
      35.1), at a Record Path with keys after the branches.
    - :meth:`StructureAnalyzer._reference` (8, ``structure.shared_defs`` and
      ``structure.recursion``, task 36.1): at a recursive step, ``analyze``
      emits a ``$ref`` instead of recursing. The ``shared_definition``
      Decision_Point belongs to the shared-definition pass, so this hook
      records nothing.

    The array grammar (7.5–7.8, task 43.1) is the ArrayAnalyzer's own step,
    MDL exception encoding (17.2, 17.3, task 38.2) adds candidates inside
    the components, and the flags it raises affect only the flagged item's
    subtree because every step reads live views.

Recursion
    The walk recurses through the components' callbacks, about ten Python
    frames per object level. :meth:`StructureAnalyzer.infer_model` raises
    the interpreter's recursion limit by a bound proportional to the
    store's deepest column for the duration of the walk and restores it
    afterwards, so documents nested up to the Ingestor's limit
    (``io.ingest.MAX_NESTING_DEPTH``) are analyzed. On CPython 3.11 and
    later, calls between Python functions do not consume the C stack.

Determinism
    Child views are visited in key-id (code-point) order, positions in
    index order, and Variants in the grouping's canonical order; every
    choice is the components', which depend only on the multiset of
    non-flagged Observations. The emitted schema therefore does not depend
    on document order, chunking, or worker count. Nids come from the
    process-wide counter and are distinct within the model;
    ``SchemaModel.renumber`` makes them canonical when needed.
"""

from __future__ import annotations

import dataclasses

import sys
from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Final

import numpy as np

from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import ResourceReader
from jsinfer.core.determinism import DEFAULT_SEED, sorted_codepoint
from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.paths import ELEM_STEP, Path, key_step, render_path
from jsinfer.core.views import View
from jsinfer.decide.engine import ArtifactSource, Decision, DecisionEngine, DecisionPoint
from jsinfer.deps.cooccurrence import CooccurrenceMiner
from jsinfer.deps.exclusions import ExclusionMiner, ExclusionRules
from jsinfer.deps.tagged_unions import TREES, Discriminator, DiscriminatorMiner
from jsinfer.structure.field_groups import DEPENDENT_REQUIRED, FieldGroupMiner, FieldGroups, merge_records
from jsinfer.deps.presence import Presence, RequiredFields, required_fields
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.structure.arrays import ArrayAnalyzer
from jsinfer.learn.features import (
    closed_enum_features,
    closed_record_features,
    merge_split_features,
    record_map_features,
    tuple_list_features,
)
from jsinfer.structure.record_map import (
    HYBRID,
    ClosedOpenOutcome,
    Partition,
    PartitionOutcome,
    RecordMapAnalyzer,
    RecordMapOutcome,
    entries_view,
    object_rows,
    partition_encoding,
    partition_keys,
)
from jsinfer.structure.shared_defs import (
    DEFS,
    RECURSION,
    SHARED_DEFS,
    Definition,
    SharedDefinitionPass,
    assign_names,
    level_grouping,
    preferred_name,
    ref_node,
    ref_string,
)
from jsinfer.structure.variants import ANY_OF, ONE_OF, MergeSplitOutcome, VariantAnalyzer, VariantNode
from jsinfer.values.inferrer import ScalarNode, ValueInferrer

if TYPE_CHECKING:
    from jsinfer.config.run_config import RunConfiguration

__all__ = [
    "BRANCH_CLOSED_ENUM",
    "BRANCH_REMAINING_TYPES",
    "DISCRIMINATOR",
    "EXCLUSION_RULE",
    "FIELD_GROUPS",
    "RECORD_MAP_PARTITION",
    "SHARED_DEFINITION_TECHNIQUES",
    "StructureAnalyzer",
]

# Decision_Points the walk reaches whose Techniques are not implemented yet (see the module docstring, Hooks).
RECORD_MAP_PARTITION: Final = "record_map_partition"
FIELD_GROUPS: Final = "field_groups"
DISCRIMINATOR: Final = "discriminator"
EXCLUSION_RULE: Final = "exclusion_rule"
SHARED_DEFINITION_TECHNIQUES: Final = REGISTRY.decision_point("shared_definition").served_by

# The ``branch`` detail of the branches the mixed-type rule adds beside Variant branches.
BRANCH_CLOSED_ENUM: Final = "closed_enum"
BRANCH_REMAINING_TYPES: Final = "remaining_types"

_TYPE: Final = "type"
_ARRAY_TYPE: Final = JType.ARRAY.keyword
_OBJECT_TYPE: Final = JType.OBJECT.keyword
_ADDITIONAL_PROPERTIES: Final = "additionalProperties"
_NON_SCALAR: Final = (int(JType.ARRAY), int(JType.OBJECT))

# A generous bound on the Python frames one nesting level of the walk uses (about ten for an object).
_FRAMES_PER_LEVEL: Final = 32


# ---------------------------------------------------------------------------
# Records by node
# ---------------------------------------------------------------------------


class _Records:
    """The walk's Provenance_Records by node nid, then ``(keyword, item)``: one per Constraint."""

    __slots__ = ("_by_node",)

    def __init__(self) -> None:
        self._by_node: dict[int, dict[tuple[str, str | int | None], ProvenanceRecord]] = {}

    def add(self, cid: ConstraintId, record: ProvenanceRecord) -> None:
        if not isinstance(record, ProvenanceRecord):
            raise TypeError(f"expected a ProvenanceRecord, got {type(record).__name__}")
        bucket = self._by_node.setdefault(cid.nid, {})
        key = (cid.keyword, cid.item)
        if key in bucket:
            raise ValueError(f"Constraint {cid} already has a Provenance_Record")
        bucket[key] = record

    def put(self, node: SchemaNode, keyword: str, record: ProvenanceRecord, item: str | int | None = None) -> None:
        self.add(ConstraintId(node.nid, keyword, item), record)

    def update(self, records: Mapping[ConstraintId, ProvenanceRecord]) -> None:
        for cid, record in records.items():
            self.add(cid, record)

    def discard_subtree(self, node: SchemaNode) -> None:
        """Forget the records of ``node`` and every subschema below it (a node replaced by a ``$ref``)."""
        for _, inner in node.walk():
            self._by_node.pop(inner.nid, None)

    def drop(self, node: SchemaNode, keyword: str, item: str | int | None = None) -> None:
        """Forget the record of one Constraint, if any (a replaced keyword)."""
        self._by_node.get(node.nid, {}).pop((keyword, item), None)

    def snapshot(self) -> dict[int, dict[tuple[str, str | int | None], ProvenanceRecord]]:
        return {nid: dict(bucket) for nid, bucket in self._by_node.items()}

    def restore(self, snapshot: dict[int, dict[tuple[str, str | int | None], ProvenanceRecord]]) -> None:
        self._by_node = snapshot

    def move(self, source: SchemaNode, target: SchemaNode, *, skip: str) -> None:
        """Re-key the records of ``source`` to ``target``, dropping those of keyword ``skip``."""
        for (keyword, item), record in self._by_node.pop(source.nid, {}).items():
            if keyword != skip:
                self.add(ConstraintId(target.nid, keyword, item), record)

    def constraint_ids(self) -> dict[ConstraintId, ProvenanceRecord]:
        """Every record keyed by its ConstraintId, in ConstraintId order."""
        out = {
            ConstraintId(nid, keyword, item): record
            for nid, bucket in self._by_node.items()
            for (keyword, item), record in bucket.items()
        }
        return {cid: out[cid] for cid in sorted(out)}


def _type_record(rows: int, details: Mapping[str, JSON] | None = None) -> ProvenanceRecord:
    """The ``type`` Baseline_Rule's record over ``rows`` rows, every one of which has a listed type (26.13)."""
    return ProvenanceRecord.baseline(_TYPE, Support(rows, rows), details=dict(details or {}))


# ---------------------------------------------------------------------------
# Parts of a node
# ---------------------------------------------------------------------------


@dataclass(eq=False, slots=True)
class _Part:
    """One JSON-type family of a node: a schema object with its own ``type`` and records.

    ``types`` are the family's type names in emitted order, ``rows`` its
    non-flagged rows, and ``details`` what its ``type`` record notes (4.11,
    7.9). ``scalar`` is the ValueInferrer's result for the scalar family.
    """

    node: SchemaNode
    types: tuple[str, ...]
    rows: int
    details: dict[str, JSON] = field(default_factory=dict)
    scalar: ScalarNode | None = None
    exclusions: ExclusionRules | None = None  # the object family's Exclusion_Rules (11), applied at assembly

    @property
    def closed_enum(self) -> bool:
        """Whether the part holds a Closed_Enum (``enum`` or ``const``), which applies to every instance type."""
        return self.scalar is not None and self.scalar.enum.closed


@dataclass(eq=False, slots=True)
class _Split:
    """The object family of a Record Path emitted as two or more Variant branches."""

    variants: VariantNode
    rows: int
    exclusions: ExclusionRules | None = None


# ---------------------------------------------------------------------------
# Recursion room
# ---------------------------------------------------------------------------


_MAX_MERGES: Final = 16  # cross-Path merges (8.3) one model performs


def _property_names(objects: View) -> set[str]:
    """The property names of the non-flagged objects of a view (empty when it has none)."""
    if objects.is_empty:
        return set()
    stats = objects.stats()
    return {objects.store.key_name(int(kid)) for kid in stats.key_ids.tolist()}


def _levels(view: View) -> int:
    """An upper bound on the walk's nesting levels below ``view``: the steps of its store's deepest column, plus one."""
    return max((len(column) for column in view.store.columns), default=0) + 1


@contextmanager
def _recursion_room(levels: int) -> Iterator[None]:
    """Raise the recursion limit by ``levels`` walk levels for the duration of the block, then restore it."""
    previous = sys.getrecursionlimit()
    sys.setrecursionlimit(previous + levels * _FRAMES_PER_LEVEL)
    try:
        yield
    finally:
        sys.setrecursionlimit(previous)


# ---------------------------------------------------------------------------
# The analyzer
# ---------------------------------------------------------------------------


def _check(component: Any, kind: type, name: str) -> None:
    if component is not None and not isinstance(component, kind):
        raise TypeError(f"{name} must be a {kind.__name__} or None, got {type(component).__name__}")


class StructureAnalyzer:
    """The Structure_Analyzer of one run (see the module docstring).

    ``values`` is the run's ValueInferrer; its engine, Validator, and report
    are shared by every component. ``scorer`` is the MDL_Scorer of the
    components built here (a new one sharing the Validator when None).
    ``resources`` and ``seed`` configure the default VariantAnalyzer (a run
    passes a ``ResourceReader`` of its Run_Configuration, which lists each
    Resource_Parameter read in the report, and ``seed``). ``record_map``,
    ``variants``, ``arrays``, and ``cooccurrence`` replace the default
    components; each must use ``values.engine``.
    """

    __slots__ = (
        "_arrays", "_building", "_cooccurrence", "_definitions", "_discriminators", "_exclusion_miner",
        "_entries_by_nid", "_field_groups_miner", "_groups_by_nid", "_record_map", "_records", "_recursive",
        "_scorer", "_shared_defs", "_values", "_variants", "_views_by_nid",
    )

    def __init__(
        self,
        values: ValueInferrer,
        *,
        scorer: MDLScorer | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = DEFAULT_SEED,
        record_map: RecordMapAnalyzer | None = None,
        variants: VariantAnalyzer | None = None,
        arrays: ArrayAnalyzer | None = None,
        cooccurrence: CooccurrenceMiner | None = None,
        field_groups: FieldGroupMiner | None = None,
        discriminators: DiscriminatorMiner | None = None,
        exclusions: ExclusionMiner | None = None,
    ) -> None:
        if not isinstance(values, ValueInferrer):
            raise TypeError(f"values must be a ValueInferrer, got {type(values).__name__}")
        _check(scorer, MDLScorer, "scorer")
        _check(record_map, RecordMapAnalyzer, "record_map")
        _check(variants, VariantAnalyzer, "variants")
        _check(arrays, ArrayAnalyzer, "arrays")
        _check(cooccurrence, CooccurrenceMiner, "cooccurrence")
        _check(field_groups, FieldGroupMiner, "field_groups")
        _check(discriminators, DiscriminatorMiner, "discriminators")
        _check(exclusions, ExclusionMiner, "exclusions")
        if variants is not None and (resources is not None or seed != DEFAULT_SEED):
            raise ValueError("resources and seed configure the default VariantAnalyzer and FieldGroupMiner; pass them to those instead")
        engine = values.engine
        validator = values.validator
        scorer = scorer if scorer is not None else MDLScorer(validator=validator)
        self._values = values
        self._scorer = scorer
        # The default components judge with the versioned featurizers of learn.features (22.10).
        self._record_map = (
            record_map
            if record_map is not None
            else RecordMapAnalyzer(
                values,
                scorer=scorer,
                record_map_features=record_map_features,
                closed_record_features=closed_record_features,
            )
        )
        self._variants = (
            variants
            if variants is not None
            else VariantAnalyzer(
                engine,
                scorer=scorer,
                resources=resources,
                seed=seed,
                validator=validator,
                merge_split_features=merge_split_features,
            )
        )
        self._arrays = (
            arrays if arrays is not None else ArrayAnalyzer(values, scorer=scorer, tuple_list_features=tuple_list_features)
        )
        self._cooccurrence = (
            cooccurrence if cooccurrence is not None else CooccurrenceMiner(engine, scorer=scorer, report=values.report)
        )
        self._field_groups_miner = (
            field_groups
            if field_groups is not None
            else FieldGroupMiner(engine, scorer=scorer, report=values.report, resources=resources, seed=seed)
        )
        self._discriminators = (
            discriminators
            if discriminators is not None
            else DiscriminatorMiner(engine, scorer=scorer, report=values.report, resources=resources, seed=seed)
        )
        self._exclusion_miner = (
            exclusions
            if exclusions is not None
            else ExclusionMiner(engine, scorer=scorer, report=values.report, validator=validator, resources=resources)
        )
        self._groups_by_nid: dict[int, tuple[frozenset[str], ...]] = {}
        self._shared_defs = SharedDefinitionPass(engine, scorer=scorer, report=values.report)
        self._definitions: list[Definition] = []
        self._recursive: dict[Path, tuple[int, Decision]] = {}  # recursive step Path -> (definition index, decision)
        self._building: set[Path] = set()
        self._views_by_nid: dict[int, View] = {}  # node nid -> the view it was inferred from (8.1-8.3 unions)
        self._entries_by_nid: dict[int, tuple[int, int]] = {}  # node nid -> the report entries its analysis recorded
        for name, component in (
            ("record_map", self._record_map),
            ("variants", self._variants),
            ("arrays", self._arrays),
            ("cooccurrence", self._cooccurrence),
            ("field_groups", self._field_groups_miner),
            ("discriminators", self._discriminators),
            ("exclusions", self._exclusion_miner),
        ):
            if component.engine is not engine:
                raise ValueError(f"{name} must use the ValueInferrer's DecisionEngine")
        self._records = _Records()

    @classmethod
    def for_techniques(
        cls,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = DEFAULT_SEED,
    ) -> StructureAnalyzer:
        """An analyzer whose engine has the Technique states ``techniques`` (omitted ids take their default).

        Every component shares one engine, Validator, MDL_Scorer, and ``report``.
        """
        validator = validator if validator is not None else Validator()
        scorer = MDLScorer(validator=validator)
        engine = DecisionEngine(techniques, bundle=bundle, report=report, scorer=scorer)
        values = ValueInferrer(
            engine, validator=validator, report=report, scorer=scorer, enum_features=closed_enum_features,
            resources=resources, seed=seed,
        )
        return cls(values, scorer=scorer, resources=resources, seed=seed)

    @classmethod
    def for_run(
        cls,
        config: RunConfiguration,
        bundle: ArtifactSource | None,
        report: InferenceReport | None,
        *,
        validator: Validator | None = None,
    ) -> StructureAnalyzer:
        """The analyzer of a run with a validated Run_Configuration (``pipeline.infer``, task 16.2).

        The engine takes the configuration's Technique states and reads
        ``bundle``; the VariantAnalyzer takes its Resource_Parameters, through
        a ``ResourceReader`` that lists each one read in ``report`` with its
        origin (20.5), and its seed.
        """
        validator = validator if validator is not None else Validator()
        scorer = MDLScorer(validator=validator)
        engine = DecisionEngine.for_run(config, bundle, report, scorer=scorer)
        resources = ResourceReader.for_run(config, report)
        values = ValueInferrer(
            engine, validator=validator, report=report, scorer=scorer, enum_features=closed_enum_features,
            resources=resources, seed=config.seed,
        )
        return cls(values, scorer=scorer, resources=resources, seed=config.seed)

    # -- components ---------------------------------------------------------

    @property
    def values(self) -> ValueInferrer:
        return self._values

    @property
    def engine(self) -> DecisionEngine:
        return self._values.engine

    @property
    def validator(self) -> Validator:
        return self._values.validator

    @property
    def report(self) -> InferenceReport | None:
        return self._values.report

    @property
    def record_map(self) -> RecordMapAnalyzer:
        return self._record_map

    @property
    def variants(self) -> VariantAnalyzer:
        return self._variants

    @property
    def arrays(self) -> ArrayAnalyzer:
        return self._arrays

    @property
    def cooccurrence(self) -> CooccurrenceMiner:
        return self._cooccurrence

    # -- entry points -------------------------------------------------------

    def infer_model(self, view: View) -> SchemaModel:
        """The Schema_Model of the non-flagged rows of ``view``, with one Provenance_Record per Constraint.

        ``view`` is normally a root view (``View.root(store)``, or one
        segment's documents). The model's ``provenance`` holds the record of
        every Constraint the walk emitted, keyed by ConstraintId.
        """
        if not isinstance(view, View):
            raise TypeError(f"infer_model takes a View, got {type(view).__name__}")
        self._records = _Records()
        self._definitions = []
        self._recursive = {}
        self._building = set()
        self._views_by_nid = {}
        self._entries_by_nid = {}
        with _recursion_room(_levels(view)):
            root = self.analyze(view)
        root = self._hoist_root_definition(root)
        self._fill_references()  # 8.4
        self._merge_similar(root)  # 8.3 across non-prefix Paths
        root = self._extract_definitions(root)  # 8.1, 8.2, 8.6, 8.7
        self._prune_definitions(root)
        self._name_definitions(root)  # 8.5
        model = SchemaModel(root=root, provenance=self._records.constraint_ids())
        nodes = model.index()  # also checks that every node holds one position with a distinct nid
        stray = [cid for cid in model.provenance if cid.nid not in nodes]
        if stray:
            raise RuntimeError(f"Provenance_Records of nodes outside the schema: {stray[:3]}")
        return model

    def analyze(self, view: View) -> SchemaNode:
        """The subschema of the non-flagged rows of ``view`` (see the module docstring, The walk).

        Records of the Constraints emitted go to the walk's collection, which
        :meth:`infer_model` returns in the model. Components call this as
        their ``child_schema``.
        """
        if not isinstance(view, View):
            raise TypeError(f"analyze takes a View, got {type(view).__name__}")
        start = self._mark()
        node = self._analyze(view)
        self._views_by_nid[node.nid] = view  # the view a node was inferred from (8.1-8.3 unions)
        self._entries_by_nid[node.nid] = (start, self._mark())
        return node

    def _mark(self) -> int:
        return self.report.mark() if self.report is not None else 0

    def _forget_subtree(self, node: SchemaNode) -> None:
        """Drop the records of ``node``'s subtree and the report entries its analysis recorded (it left the model)."""
        self._records.discard_subtree(node)
        span = self._entries_by_nid.get(node.nid)
        if span is not None and self.report is not None:
            self.report.forget_between(*span)

    def _analyze(self, view: View) -> SchemaNode:
        live = view.live()
        stats = live.stats()
        if stats.n == 0:
            return SchemaNode()  # nothing observed: {} accepts every value, as under Naive_Merge
        reference = self._reference(live)
        if reference is not None:
            return reference
        n_arrays = stats.count(JType.ARRAY)
        n_objects = stats.count(JType.OBJECT)
        if n_objects == stats.n and self.runs(RECURSION) and live.path not in self._building:
            recursive = self._recursion(live)  # 8.3, 8.4: a self-referencing Shared_Definition
            if recursive is not None:
                return recursive
        scalar = self._scalar_part(live) if stats.n > n_arrays + n_objects else None
        array = self._array_part(live) if n_arrays else None
        obj = self._object_part(live) if n_objects else None
        return self._assemble(scalar, array, obj, stats.n)

    # -- the type families --------------------------------------------------

    def _scalar_part(self, view: View) -> _Part:
        """Design walk, step 1: the ValueInferrer on the scalar rows."""
        jtypes = view.gather("jtype")
        mask = ~np.isin(jtypes, _NON_SCALAR)
        scalars = view if mask.all() else view.subset(mask)
        result = self._values.infer_scalar(scalars)
        self._records.update(result.constraint_provenance())
        return _Part(result.node, result.types.names, result.types.n, scalar=result)

    def _array_part(self, view: View) -> _Part:
        """Design walk, step 3: Tuple-or-List, the element recursion, and bounds, plus ``type``."""
        result = self._arrays.array_node(view, child_schema=self.analyze)
        node = result.node
        node.kw[_TYPE] = [_ARRAY_TYPE]
        rows = result.outcome.lengths.n_arrays
        details = result.type_details()  # 7.9: no element observed
        self._records.update(result.constraint_provenance())
        self._records.put(node, _TYPE, _type_record(rows, details))
        return _Part(node, (_ARRAY_TYPE,), rows, details)

    def _object_part(self, view: View) -> _Part | _Split:
        """Design walk, step 2: Record or Map, then a Record's Variants and their branches."""
        objects = object_rows(view)
        rows = len(objects)
        outcome = self._record_map.record_or_map(objects)
        hybrid = self._record_map_partition(objects, outcome)  # 4.5: replaces the outcome when it wins
        if hybrid is not None:
            return self._hybrid_part(objects, hybrid)
        if outcome.is_map:
            result = self._record_map.map_node(objects, outcome, child_schema=self.analyze)
            node = result.node
            node.kw[_TYPE] = [_OBJECT_TYPE]
            self._records.update(result.constraint_provenance())
            self._records.put(node, _TYPE, _type_record(rows))
            if result.key_constraint is not None:
                # The key Format (or Pattern) is also a Constraint of the propertyNames subschema, from one decision.
                names = node.kw["propertyNames"]
                for keyword in names.kw:
                    self._records.put(names, keyword, result.provenance["propertyNames"])
            return _Part(node, (_OBJECT_TYPE,), rows)

        closed = self._record_map.closed_or_open(objects)
        split = self._variants.merge_or_split(self._variants.select(objects))
        tagged = self._discriminator(objects, outcome, split, closed)  # 10: value groups replace the Variants
        if tagged is not None:
            return _Split(tagged, rows, self._exclusions(objects, outcome, tagged))
        # Each final Variant's index among the Path's Variants, by its rows (the views partition the objects).
        index = {variant.digest: position for position, variant in enumerate(split.views())}
        type_details: dict[int, dict[str, JSON]] = {}

        def variant_schema(variant: View) -> SchemaNode:
            node, details = self._variant_node(variant, index[variant.digest], outcome, closed)
            type_details[node.nid] = details
            return node

        emitted = self._variants.branches(split, variant_schema=variant_schema)
        for branch, branch_view in zip(emitted.branches or (emitted.node,), emitted.views):
            self._views_by_nid[branch.nid] = branch_view
        exclusions = self._exclusions(objects, outcome, emitted)
        if emitted.keyword is None:
            return _Part(emitted.node, (_OBJECT_TYPE,), rows, type_details[emitted.node.nid], exclusions=exclusions)
        return _Split(emitted, rows, exclusions)

    def _variant_node(
        self, variant: View, index: int, outcome: RecordMapOutcome, closed: ClosedOpenOutcome
    ) -> tuple[SchemaNode, dict[str, JSON]]:
        """The object schema of one final Variant (design walk, step 2.5) and its ``type`` record's details."""
        start = self._mark()
        result = self._record_map.record_node(variant, outcome, closed, child_schema=self.analyze)
        node = result.node
        node.kw[_TYPE] = [_OBJECT_TYPE]
        self._records.update(result.constraint_provenance())
        required = required_fields(variant)
        details = result.type_details() if not outcome.keys_available else {}  # 4.11
        self._records.put(node, _TYPE, _type_record(required.presence.n_objects, details))
        node.kw.update(required.keywords())
        self._records.update(required.constraint_provenance(node))
        groups = self._field_groups(variant, required, node, index)  # 6
        self._groups_by_nid[node.nid] = groups.groups  # for the Exclusion_Rule members (11.1)
        rules = self._cooccurrence.rules(required.presence, record=node, groups=groups.groups, variant=index)
        # 6.6, 9.4: the cliques and the rules share one dependentRequired; an antecedent in both is one Constraint.
        entries: dict[str, list[str]] = {}
        for source in (groups.entries(), rules.entries()):
            for antecedent, consequents in source.items():
                listed = entries.setdefault(antecedent, [])
                listed.extend(c for c in consequents if c not in listed)
        if entries:
            node.kw[DEPENDENT_REQUIRED] = {a: sorted_codepoint(entries[a]) for a in sorted_codepoint(entries)}
        group_records = groups.constraint_provenance(node)
        rule_records = rules.constraint_provenance(node)
        for cid, record in rule_records.items():
            if cid in group_records:
                group_records[cid] = merge_records(group_records[cid], record)
            else:
                group_records[cid] = record
        self._records.update(group_records)
        self._entries_by_nid[node.nid] = (start, self._mark())
        return node, details

    # -- combining the families (design walk, step 4) -------------------------

    def _merge(self, parts: Sequence[_Part]) -> SchemaNode:
        """The parts' keywords in one schema object with one ``type``; a single part is returned as it is."""
        if len(parts) == 1:
            return parts[0].node
        node = SchemaNode()
        types: list[str] = []
        details: dict[str, JSON] = {}
        for part in parts:
            for keyword, value in part.node.kw.items():
                if keyword == _TYPE:
                    continue
                if keyword in node.kw:
                    raise ValueError(f"two JSON-type families both emit {keyword!r}")
                node.kw[keyword] = value
            self._records.move(part.node, node, skip=_TYPE)
            types.extend(part.types)
            details.update(part.details)
        node.kw[_TYPE] = types
        self._records.put(node, _TYPE, _type_record(sum(part.rows for part in parts), details))
        return node

    def _assemble(self, scalar: _Part | None, array: _Part | None, obj: _Part | _Split | None, rows: int) -> SchemaNode:
        """Combine the families of one view (see the module docstring, Combining the parts)."""
        split = obj if isinstance(obj, _Split) else None
        others = [part for part in (array, obj if isinstance(obj, _Part) else None) if part is not None]
        isolated = scalar is not None and scalar.closed_enum and (bool(others) or split is not None)
        coexisting = ([scalar] if scalar is not None and not isolated else []) + others
        exclusions = obj.exclusions if obj is not None else None
        if split is None and not isolated:
            merged = self._merge(coexisting)
            if exclusions is not None:  # 11: on the object schema, guarded when other types share the node
                self._records.update(exclusions.apply(merged, guard=len(coexisting) > 1))
            return merged
        if split is not None and not coexisting and not isolated:
            self._records.update(split.variants.constraint_provenance())
            if exclusions is not None:
                self._records.update(exclusions.apply(split.variants.node))
            return split.variants.node

        branches: list[SchemaNode] = []
        records: list[ProvenanceRecord] = []
        if split is not None:
            outcome = split.variants.outcome
            for position, branch in enumerate(split.variants.branches):
                branches.append(branch)
                if split.variants.kind == "tagged_union":
                    records.append(split.variants.provenance[position])
                else:
                    records.append(outcome.branch_provenance(position, details={"combinator": ANY_OF}, total=rows))
        if isolated:
            assert scalar is not None
            branches.append(scalar.node)
            records.append(self._enum_branch_record(scalar, scalar.rows, rows, BRANCH_CLOSED_ENUM, scalar.types))
        if coexisting:
            rest = self._merge(coexisting)
            consistent = sum(part.rows for part in coexisting)
            types = [name for part in coexisting for name in part.types]
            details: dict[str, JSON] = {"branch": BRANCH_REMAINING_TYPES, "combinator": ANY_OF, "types": types}
            if split is not None and split.variants.kind == "tagged_union":
                # The Discriminator's decision produced the branches (its Variants may be one): its record, resupported.
                first = split.variants.provenance[0]
                kept = {k: v for k, v in first.details.items() if k not in ("branch", "value_group", "support_ratio", "closed_enum")}
                record = dataclasses.replace(first, support=Support(consistent, rows), details={**kept, **details})
            elif split is not None:
                record = split.variants.outcome.remaining_provenance(consistent, rows, details=details)
            else:
                assert scalar is not None
                record = self._enum_branch_record(scalar, consistent, rows, BRANCH_REMAINING_TYPES, types)
            branches.append(rest)
            records.append(record)
        node = SchemaNode(kw={ANY_OF: branches})
        for position, record in enumerate(records):
            self._records.put(node, ANY_OF, record, position)
        if exclusions is not None:  # 11: the mixed node's objects, guarded against its other instances
            self._records.update(exclusions.apply(node, guard=True))
        return node

    @staticmethod
    def _enum_branch_record(
        scalar: _Part, consistent: int, total: int, branch: str, types: Sequence[str]
    ) -> ProvenanceRecord:
        """The record of a branch the Closed_Enum beside other types produced: the Closed-or-Open-Enum decision."""
        assert scalar.scalar is not None
        judgment = scalar.scalar.enum
        decision = judgment.decision
        assert decision is not None  # a Closed_Enum comes from a resolved decision
        details: dict[str, JSON] = {**judgment.details(), "branch": branch, "combinator": ANY_OF, "types": list(types)}
        return decision.provenance(Support(consistent, total), details=details)

    # -- hooks of unimplemented Techniques ------------------------------------

    def runs(self, tid: str) -> bool:
        """Whether Technique ``tid``'s step runs: registered as implemented and enabled in the engine."""
        return self._values.runs(tid)

    def _record_map_partition(self, objects: View, outcome: RecordMapOutcome) -> PartitionOutcome | None:
        """4.5: the hybrid Record/Map partition of an object Path with keys, when it beats both pure encodings.

        Records the Fallback_Behavior (no partition) while the Technique is
        not in effect (28.4). The partition of the keys the induced Pattern
        matches uses a Pattern induced over every key without report
        entries; the winner's Map entries get their own, recorded,
        induction in :meth:`_hybrid_part` (4.6).
        """
        if not outcome.keys_available:
            return None
        if not self.runs(HYBRID):
            self.engine.resolve(DecisionPoint(RECORD_MAP_PARTITION, objects.path), [], objects)
            return None
        result = partition_keys(
            objects,
            outcome,
            self.engine,
            validator=self._values.validator,
            scorer=self._scorer,
            induce=lambda partition: self._entry_pattern(objects, partition, side=True),
        )
        return result if result.partition is not None else None

    def _hybrid_part(self, objects: View, hybrid: PartitionOutcome) -> _Part:
        """4.5–4.7: the partitioned object node: Record fields under ``properties``, the Map entries' value schema
        under ``patternProperties`` (a Pattern matching every entry and no field, 4.6) or ``additionalProperties``,
        ``required`` over the fields, and Closed-or-Open-Record in the ``patternProperties`` form (4.7).

        A partitioned Path has one object schema: Variants, Discriminators,
        Field_Groups, Co_Occurrence_Rules, and Exclusion_Rules (which are
        defined over a Record's fields) do not run on it.
        """
        partition = hybrid.partition
        assert partition is not None
        rows = len(objects)
        node = SchemaNode(kw={_TYPE: [_OBJECT_TYPE]})
        node.kw["properties"] = {name: self.analyze(objects.child_key(name)) for name in partition.fields}
        presence = Presence.of(objects)
        required = [name for name in partition.fields if presence.is_required(name)]
        if required:
            node.kw["required"] = required
        value_node = self.analyze(entries_view(objects, partition.entries))
        # 4.6: the Pattern partition's Pattern matches every entry and no field by construction; the run's
        # Value_Inferrer now records its selection (the same induction, on the same keys). A top-j partition
        # has no such Pattern: were there one, the Pattern partition would coincide with it and encode cheaper.
        pattern = self._entry_pattern(objects, partition) if partition.pattern is not None else None
        if pattern is not None:
            node.kw["patternProperties"] = {pattern: value_node}
            map_keyword = "patternProperties"
        else:
            node.kw[_ADDITIONAL_PROPERTIES] = value_node
            map_keyword = _ADDITIONAL_PROPERTIES
        provenance = hybrid.provenance(rows, pattern=pattern)
        self._records.put(node, "properties", provenance)
        self._records.put(node, map_keyword, provenance)
        if required:
            record = required_fields(presence).record
            for name in required:
                self._records.put(node, "required", record, item=name)
        if pattern is not None:  # 4.7: Closed-or-Open applies to the partitioned Record
            closed = self._record_map.closed_or_open(objects, record=node)
            if closed.closed:
                node.kw.update(closed.keywords())
                self._records.put(node, _ADDITIONAL_PROPERTIES, closed.provenance())
        self._records.put(node, _TYPE, _type_record(rows))
        return _Part(node, (_OBJECT_TYPE,), rows)

    def _entry_pattern(self, objects: View, partition: Partition, *, side: bool = False) -> str | None:
        """4.6: the Pattern induced over the Map entries of ``partition``, or None.

        The candidates are scored as the partition's encoding with and
        without the Pattern. With ``side``, a side engine judges without
        report entries (the exploratory induction of 4.5, which only needs
        a Pattern of the entries); otherwise the run's Value_Inferrer
        records the selection, and the Pattern must match every entry and
        no Record field (4.6).
        """
        values = self._values
        if side:
            engine = self.engine
            values = ValueInferrer(
                DecisionEngine(engine.techniques, bundle=engine.bundle, scorer=self._scorer),
                validator=values.validator, scorer=self._scorer, resources=values.resources, seed=values.seed,
            )

        def with_pattern(candidate: Any) -> SchemaNode:
            return partition_encoding(objects, dataclasses.replace(partition, pattern=str(candidate.value)))

        found = values.key_constraint(
            objects,
            keys=partition.entries,
            make_node=with_pattern,
            none_node=partition_encoding(objects, dataclasses.replace(partition, pattern=None)),
        )
        if found is None or found.candidate.keyword != "pattern":
            return None
        pattern = str(found.candidate.value)
        matches = self._values.validator.matches_pattern
        if not all(matches(pattern, name) for name in partition.entries):
            return None
        if not side and any(matches(pattern, name) for name in partition.fields):
            return None
        return pattern

    def _unwritten(self, tids: Sequence[str]) -> None:
        for tid in tids:
            if self.runs(tid):
                raise NotImplementedError(
                    f"{tid} is registered as implemented, but the Structure_Analyzer has no step for it"
                )

    def _planned(self, kind: str, view: View) -> None:
        """Reach Decision_Point ``kind`` whose serving Techniques have no step yet (see the module docstring, Hooks)."""
        servers = REGISTRY.decision_point(kind).served_by
        engine = self.engine
        self._unwritten(servers)
        if any(engine.enabled(tid) for tid in servers):
            return  # enabled outside a validated run, where it is not implemented: it offers nothing
        engine.resolve(DecisionPoint(kind, view.path), [], view)  # 28.4: the Fallback_Behavior is recorded

    def _field_groups(self, variant: View, required: RequiredFields, node: SchemaNode, index: int) -> FieldGroups:
        """6: the Field_Groups of final Variant ``index`` (``groups.fpgrowth_slim``; BMF joins in task 47.1).

        A Variant without a field has nothing to group and reaches no
        Decision_Point. ``node`` is the Variant subschema the candidates
        are scored against.
        """
        if not required.presence.n_fields:
            return FieldGroups(required.presence, index)
        return self._field_groups_miner.mine(required.presence, record=node, variant=index)

    def _discriminator(
        self, objects: View, outcome: RecordMapOutcome, split: MergeSplitOutcome, closed: ClosedOpenOutcome
    ) -> VariantNode | None:
        """10: the Discriminator of a Record Path and its branches, or None when the Variants stand.

        The miner selects the Discriminator (10.1–10.3, 10.6). Each dedicated
        value group becomes a branch built like a Variant (so ``required``
        lists the fields common to its objects, the Base_Fields among them,
        10.4) with the Discriminator fixed by ``const`` or ``enum``; the
        fallback branch (10.5, 10.8) requires only the Path's Base_Fields.
        Every object must validate against exactly one branch (10.7); if
        not, the branches are discarded, the failure recorded, and the
        Variants stand.
        """
        if not outcome.keys_available:
            return None
        found = self._discriminators.mine(objects, split.views())
        if found is None:
            return None
        name = found.name
        base = list(required_fields(objects).presence.required)
        enum_closed = self._closed_enum(objects.child_key(name))
        n = len(objects)
        snapshot = self._records.snapshot()
        views: list[View] = []
        branches: list[SchemaNode] = []
        details: list[dict[str, JSON]] = []
        for index, (group, mask) in enumerate(zip(found.dedicated, found.masks)):
            view = objects.subset(mask)
            node, _ = self._variant_node(view, index, outcome, closed)
            self._fix_discriminator(node, name, group.constraint(), found)
            views.append(view)
            branches.append(node)
            details.append({"branch": "dedicated", "value_group": list(group.values), "support_ratio": group.ratio})
        if found.below or not enum_closed:
            fallback_view = objects.subset(found.fallback_mask)
            if not fallback_view.is_empty:
                node, _ = self._variant_node(fallback_view, len(branches), outcome, closed)
                self._restrict_required(node, base)
            else:
                # No object fell outside the dedicated groups: the fallback branch is the Discriminator's decision alone.
                node = SchemaNode(kw={_TYPE: [_OBJECT_TYPE], "properties": {name: SchemaNode(kw={})}})
                empty = found.decision.provenance(Support(0, n), sample=found.sample, details={"branch": "fallback", "objects": 0})
                self._records.put(node, _TYPE, _type_record(0))
                self._records.put(node, "properties", empty)
                if base:
                    node.kw["required"] = list(base)
                    for field_name in base:
                        self._records.put(node, "required", empty, item=field_name)
            constraint: dict[str, JSON] = {"enum": found.fallback_values} if enum_closed else {"not": {"enum": found.dedicated_values}}
            self._fix_discriminator(node, name, constraint, found)
            views.append(fallback_view)
            branches.append(node)
            details.append({"branch": "fallback", "value_group": found.fallback_values, "closed_enum": enum_closed})
        if len(branches) < 2 or not self._exactly_one(objects, branches, name, found):
            self._records.restore(snapshot)
            return None
        combined = SchemaNode(kw={ONE_OF: branches})
        provenance = {
            i: found.decision.provenance(
                Support(len(view), n), sample=found.sample, details={**found.details(), **extra, "combinator": ONE_OF}
            )
            for i, (view, extra) in enumerate(zip(views, details))
        }
        return VariantNode(objects.path, combined, split, tuple(views), ONE_OF, provenance, kind="tagged_union")

    def _closed_enum(self, child: View) -> bool:
        """Whether the Value_Inferrer judges the Discriminator's Path a Closed_Enum (10.8), without report entries."""
        from jsinfer.values.enums import judge_enum

        engine = self._values.engine
        side = DecisionEngine(engine.techniques, bundle=engine.bundle, scorer=self._scorer_of())
        return judge_enum(child.live(), side, scorer=self._scorer_of(), features=self._values.enum_features).closed

    def _scorer_of(self) -> MDLScorer:
        return self._scorer

    def _fix_discriminator(self, node: SchemaNode, name: str, constraint: Mapping[str, JSON], found: Discriminator) -> None:
        """Constrain property ``name`` of a branch with ``constraint``, replacing any enum/const the values gave it."""
        properties = node.kw.setdefault("properties", {})
        child = properties.get(name)
        if not isinstance(child, SchemaNode) or child.boolean is not None:
            child = SchemaNode(kw={})
            properties[name] = child
        for keyword in ("enum", "const", "not"):
            if keyword in child.kw:
                del child.kw[keyword]
                self._records.drop(child, keyword)
        for keyword, value in constraint.items():
            child.kw[keyword] = SchemaNode.from_json(value) if keyword == "not" else value
            n = found.candidate.score.n_scored
            extra: dict[str, JSON] = {"discriminator": name, "value_group": value} if keyword != "not" else {"discriminator": name, "not_values": value["enum"]}
            record = found.decision.provenance(Support(n, n), sample=found.sample, details=extra)
            self._records.put(child, keyword, record)
            if keyword == "not":  # the enum inside the not is a Constraint of its own node, from the same decision
                self._records.put(child.kw[keyword], "enum", record)

    def _restrict_required(self, node: SchemaNode, base: Sequence[str]) -> None:
        """The fallback branch requires only the Base_Fields (10.5): drop the other required entries and their records."""
        listed = node.kw.get("required")
        if not isinstance(listed, list):
            return
        for name in listed:
            if name not in base:
                self._records.drop(node, "required", name)
        kept = [name for name in listed if name in base]
        if kept:
            node.kw["required"] = kept
        else:
            del node.kw["required"]

    def _exactly_one(self, objects: View, branches: Sequence[SchemaNode], name: str, found: Discriminator) -> bool:
        """10.7: every non-flagged object validates against exactly one branch; a failure is recorded."""
        from jsinfer.mdl.scorer import _rebuild

        validator = self._values.validator
        compiled = [validator.compile({**branch.to_json()}) for branch in branches]
        documents = _rebuild(objects, np.arange(len(objects)))
        bad = sum(1 for document in documents if sum(1 for c in compiled if validator.is_valid(c, document)) != 1)
        if bad and self._values.report is not None:
            self._values.report.record_rejected(
                RejectedCandidate("rules", render_path(objects.path), {"discriminator": name}, TREES, {"reason": "not exactly one branch", "objects": bad})
            )
        return bad == 0

    def _exclusions(self, objects: View, outcome: RecordMapOutcome, emitted: VariantNode) -> ExclusionRules | None:
        """11: the Exclusion_Rules of a Record Path, given its branches (see ``deps.exclusions``).

        The rules go on the Path's final node (:meth:`_assemble`): the object
        schema of one final Variant, or the combinator node of a split (where
        only the ``not`` form applies), guarded with ``type: object`` when
        the node also admits other types. Members are the fields and the
        Field_Groups of the branches; the branches are the ones that may
        already enforce a candidate (11.3).
        """
        if not outcome.keys_available:
            return None
        target = emitted.node
        branches = list(emitted.branches) if emitted.keyword is not None else []
        groups: list[frozenset[str]] = []
        for node in branches or [target]:
            for group in self._groups_by_nid.get(node.nid, ()):
                if not any(group & g for g in groups):
                    groups.append(group)
        rules = self._exclusion_miner.mine(objects, target=target, branches=branches, groups=groups)
        return None if rules.is_empty else rules  # applied by _assemble to the node the Path finally gets

    def _reference(self, view: View) -> SchemaNode | None:
        """8.4: at the recursive step of a Shared_Definition being built, a ``$ref`` to it instead of recursing."""
        found = self._recursive.get(view.path)
        if found is None:
            return None
        index, _ = found
        # A placeholder ({} while the body is built, so scoring and validation see an accept-all subschema);
        # _fill_references turns it into the $ref once the whole walk is done.
        node = SchemaNode()
        definition = self._definitions[index]
        definition.refs.append(node)
        definition.supports[node.nid] = len(view)
        return node

    def _recursion(self, objects: View) -> SchemaNode | None:
        """8.3, 8.4, 8.8: the levels ``A, A + S, A + 2S, …`` merged into one self-referencing definition, or None.

        ``S`` is a property step, or a property's array elements, whose
        objects share a property name with the objects at ``A``. The levels
        form a cross-Path Merge-or-Split grouping over their union
        (``shared_defs.level_grouping``); a merge analyzes the union once as
        the definition's body, with the step ``A + S`` registered so that the
        body's analysis emits a ``$ref`` there, and returns a ``$ref`` for
        ``A`` itself.
        """
        names = _property_names(objects)
        if not names:
            return None
        path = objects.path
        for name in sorted_codepoint(names):
            child = objects.child_key(name)
            for steps, candidate in (((key_step(name),), child), ((key_step(name), ELEM_STEP), child.elements())):
                level = object_rows(candidate)
                if level.is_empty or not (_property_names(level) & names):
                    continue
                levels = [objects, level]
                while len(levels) < 64:
                    deeper = level
                    for step in steps:
                        deeper = deeper.child_key(step.key) if step.kind == 0 else deeper.elements()
                    deeper = object_rows(deeper)
                    if deeper.is_empty:
                        break
                    levels.append(deeper)
                    level = deeper
                union = View.union(levels, path=path)
                grouping = level_grouping(union, levels, origin=RECURSION, engine=self.engine, scorer=self._scorer)
                if grouping is None:
                    continue  # the levels show no distinct shapes: nothing a judgment could separate
                outcome = self._variants.merge_or_split(
                    grouping, details={"origin": RECURSION, "sites": [render_path(v.path) for v in levels]}
                )
                if not outcome.merged:
                    continue
                assert outcome.decision is not None
                index = len(self._definitions)
                sites = [(view.path, len(view)) for view in levels]
                definition = Definition(SchemaNode(), preferred_name(sites), sum(len(v) for v in levels))
                self._definitions.append(definition)
                recursive_path = path + steps
                self._recursive[recursive_path] = (index, outcome.decision)
                self._building.add(path)
                try:
                    body = self.analyze(union)
                finally:
                    self._building.discard(path)
                    del self._recursive[recursive_path]
                definition.body = body
                definition.decision = outcome.decision
                node = SchemaNode()  # the placeholder for A itself; _fill_references makes it the $ref
                definition.refs.append(node)
                definition.supports[node.nid] = len(objects)
                return node
        return None

    def _hoist_root_definition(self, root: SchemaNode) -> SchemaNode:
        """A recursive definition whose ``A`` is the root is the root itself: its body stands there, its refs are ``#``."""
        for definition in self._definitions:
            if definition.decision is not None and any(ref is root for ref in definition.refs):
                definition.refs = [ref for ref in definition.refs if ref is not root]
                definition.supports.pop(root.nid, None)
                definition.at_root = True
                return definition.body
        return root

    def _fill_references(self) -> None:
        """Turn the placeholders of the recursive definitions into ``$ref`` nodes with their records (8.4)."""
        for index, definition in enumerate(self._definitions):
            if definition.decision is None:
                continue
            for node in definition.refs:
                if "$ref" in node.kw:
                    continue
                node.kw["$ref"] = "#" if definition.at_root else f"#/{DEFS}/{index}"
                n = definition.supports.get(node.nid, 0)
                self._records.put(node, "$ref", definition.decision.provenance(Support(n, n), details={"definition": index, "recursive": True}))

    def _extract_definitions(self, root: SchemaNode) -> SchemaNode:
        """8.1, 8.2, 8.6, 8.7: the shared-definition pass over the finished root."""
        if not self._shared_defs.in_effect():
            return root
        counts: dict[int, int] = {}
        for cid, record in self._records.constraint_ids().items():
            if cid.keyword == _TYPE:
                counts[cid.nid] = record.support.consistent
        records = self._records

        def on_extract(body: SchemaNode, nodes: Sequence[SchemaNode], refs: Sequence[SchemaNode], decision: Decision) -> SchemaNode:
            # The definition is inferred once from the union of the sites' observations, so its records (and
            # their totals) count every referencing site; the sites' own records go with their subschemas.
            index = len(self._definitions) - 1  # this definition; the union's analysis may add recursive ones
            views = [self._views_by_nid[node.nid] for node in nodes]
            union = View.union(views, path=views[0].path)
            sites = [render_path(view.path) for view in views]
            self._building.add(union.path)
            try:
                new_body = self.analyze(union)
            finally:
                self._building.discard(union.path)
            self._fill_references()
            for node in nodes:
                self._forget_subtree(node)
            for ref, view in zip(refs, views):
                n = len(view.live())
                records.put(ref, "$ref", decision.provenance(Support(n, n), details={"definition": index, "sites": sites}))
            return new_body

        self._shared_defs.run(root, self._definitions, counts, views=self._views_by_nid, on_extract=on_extract)
        return root

    def _merge_similar(self, root: SchemaNode) -> None:
        """8.3, 8.7: Merge-or-Split for object subschemas at distinct, non-prefix Paths that share a property name.

        Candidates are object schema sites with recorded views whose
        property-name sets overlap (Jaccard at least 1/2) and that are not
        structurally equivalent (those are 8.1's). A merge re-infers one
        definition from the union of the sites' object views and replaces
        both sites with ``$ref``; the sites' records are replaced by the
        definition's. At most :data:`_MAX_MERGES` merges per model.
        """
        if not self._shared_defs.in_effect():
            return
        from jsinfer.structure.shared_defs import _sites, canonical_hash

        merges = 0
        done: set[tuple[int, int]] = set()
        while merges < _MAX_MERGES:
            sites = [s for s in _sites(root) if s.node.nid in self._views_by_nid and s.node.boolean is None and "properties" in s.node.kw and "$ref" not in s.node.kw]
            pair = None
            for i in range(len(sites)):
                for j in range(i + 1, len(sites)):
                    a, b = sites[i], sites[j]
                    key = (a.node.nid, b.node.nid)
                    if key in done or a.pointer.startswith(b.pointer + "/") or b.pointer.startswith(a.pointer + "/"):
                        continue
                    pa, pb = self._views_by_nid[a.node.nid].path, self._views_by_nid[b.node.nid].path
                    if pa[: len(pb)] == pb or pb[: len(pa)] == pa:
                        continue  # prefix-related Paths are recursion's (8.4, 8.8)
                    names_a, names_b = set(a.node.kw["properties"]), set(b.node.kw["properties"])
                    if not names_a & names_b or len(names_a & names_b) / len(names_a | names_b) < 0.5:
                        continue
                    if canonical_hash(a.node) == canonical_hash(b.node):
                        continue  # equivalent: 8.1 decides those
                    pair = (a, b)
                    break
                if pair is not None:
                    break
            if pair is None:
                return
            a, b = pair
            done.add((a.node.nid, b.node.nid))
            # The sites' object rows: a node's view may hold other JSON types beside its objects (mixed Paths).
            view_a, view_b = object_rows(self._views_by_nid[a.node.nid]), object_rows(self._views_by_nid[b.node.nid])
            union = View.union([view_a, view_b], path=view_a.path)
            grouping = level_grouping(union, [view_a, view_b], origin=SHARED_DEFS, engine=self.engine, scorer=self._scorer)
            if grouping is None:
                continue
            outcome = self._variants.merge_or_split(
                grouping, details={"origin": SHARED_DEFS, "sites": [render_path(view_a.path), render_path(view_b.path)]}
            )
            if not outcome.merged:
                continue
            assert outcome.decision is not None
            index = len(self._definitions)
            self._building.add(union.path)
            try:
                body = self.analyze(union)
            finally:
                self._building.discard(union.path)
            self._fill_references()
            self._forget_subtree(a.node)
            self._forget_subtree(b.node)
            definition = Definition(body, preferred_name([(view_a.path, len(view_a)), (view_b.path, len(view_b))]), len(view_a) + len(view_b), decision=outcome.decision)
            self._definitions.append(definition)
            for site, view in ((a, view_a), (b, view_b)):
                ref = ref_node(f"{index}")
                site.container[site.key] = ref
                definition.refs.append(ref)
                n = len(view)
                self._records.put(ref, "$ref", outcome.decision.provenance(Support(n, n), details={"definition": index, "merged_paths": [render_path(view_a.path), render_path(view_b.path)]}))
            merges += 1

    def _prune_definitions(self, root: SchemaNode) -> None:
        """Drop definitions whose every ``$ref`` left the schema (a site replaced by a later merge), with their records."""
        reachable = {inner.nid for _, inner in root.walk()}
        for definition in self._definitions:
            for body in (definition.body,):
                for _, inner in body.walk():
                    reachable.add(inner.nid)
        kept: list[Definition] = []
        for definition in self._definitions:
            definition.refs = [ref for ref in definition.refs if ref.nid in reachable]
            kept.append(definition)
        # A definition is live when a ref to it sits in the root or in a live definition's body (fixed point).
        live = [False] * len(kept)
        root_nids = {inner.nid for _, inner in root.walk()}
        changed = True
        while changed:
            changed = False
            body_nids = [{inner.nid for _, inner in d.body.walk()} for d in kept]
            for i, definition in enumerate(kept):
                if live[i]:
                    continue
                if definition.at_root:
                    live[i] = True
                    changed = True
                    continue
                for ref in definition.refs:
                    if ref.nid in root_nids or any(live[j] and ref.nid in body_nids[j] for j in range(len(kept))):
                        live[i] = True
                        changed = True
                        break
        for i, definition in enumerate(kept):
            if not live[i]:
                self._records.discard_subtree(definition.body)
        self._definitions = [d for i, d in enumerate(kept) if live[i]]

    def _name_definitions(self, root: SchemaNode) -> None:
        """8.5: the final names, the ``$defs`` map on the root, and the rewritten ``$ref`` strings."""
        if not self._definitions:
            return
        if root.boolean is not None:
            raise RuntimeError("Shared_Definitions need a keyword schema at the root")
        listed = [d for d in self._definitions if not d.at_root]
        for definition in self._definitions:
            if definition.at_root:
                for ref in definition.refs:
                    ref.kw["$ref"] = "#"
        if not listed:
            return
        names = assign_names(listed)
        defs: dict[str, SchemaNode] = dict(root.kw.get(DEFS, {}))
        for definition, name in zip(listed, names):
            definition.name = name
            defs[name] = definition.body
            for ref in definition.refs:
                ref.kw["$ref"] = ref_string(name)
        root.kw[DEFS] = {name: defs[name] for name in sorted_codepoint(defs)}
