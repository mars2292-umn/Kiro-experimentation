"""Technique and Decision_Point registry with Fallback_Behaviors (Requirements 28.1, 28.3, 29.8).

Every Technique of Table 1 (Requirement 28) is registered here under a unique
identifier, whether or not it is built yet. A :class:`TechniqueSpec` records
the Technique's Table 1 entry and area, its delivery tier, its default state,
whether it is implemented, the Decision_Point kinds it serves with its rank in
each kind's priority order, its prerequisite Techniques, whether it is
super-linear together with its sample-size Resource_Parameter (29.8), and the
names of the Learned_Thresholds it reads.

Every Decision_Point kind is a :class:`DecisionPointSpec`: its serving
Techniques in priority order (rank 1 first), exactly one Fallback_Behavior,
and any tie rule a requirement mandates. A Fallback_Behavior is a
:class:`ConstantOutcome`. It returns the same outcome label for every view and
invokes no Technique (28.3), so with every serving Technique disabled the
Decision_Point is still resolved (28.4).

Conventions
    - Unimplemented Techniques stay registered with ``implemented=False`` and
      ``default_enabled=False``. Enabling one is a ``ConfigError`` raised by
      the Run_Configuration (task 3.3), not here. A Technique task flips its
      entry to ``implemented=True``, which also makes it enabled by default.
    - The only prerequisite is ``structure.hybrid_record_map`` →
      ``decide.mdl``, because the hybrid partition (4.5) is defined purely as
      an MDL comparison. Other Techniques use the MDL_Scorer internally and do
      not depend on the ``decide.mdl`` toggle, which governs only generic
      Decision_Point selection (19.3).
    - ``decide.dm_closed_enum`` serves ``closed_or_open_enum`` only while
      ``values.enum_estimation`` is enabled (15.7). This is a serving gate
      (:attr:`DecisionPointSpec.serves_only_with`), not a prerequisite:
      enabling the Decision_Model alone is allowed and leaves the fallback in
      force.
    - Learned_Threshold and Resource_Parameter names are plain strings, so
      this module does not depend on ``config.thresholds`` or
      ``config.resources``. No cut-off value lives here.
    - Registration order is the Table 1 order for Techniques and the design's
      Decision_Point table order for kinds. Both are fixed, so every listing
      is deterministic.

:data:`REGISTRY` is the registry of the running Inferrer. It is validated
when this module is imported; an inconsistent table raises
:class:`RegistryError` listing every problem.
"""

from __future__ import annotations

import re
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import TYPE_CHECKING, Any, Final, Literal, TypeAlias

if TYPE_CHECKING:
    from jsinfer.core.views import View

__all__ = [
    "OUTCOME_BASE_ARRAY_SCHEMA",
    "OUTCOME_CANDIDATE_ORDER",
    "OUTCOME_INLINE",
    "OUTCOME_LIST",
    "OUTCOME_NONE",
    "OUTCOME_NOT_FLAGGED",
    "OUTCOME_ONE_VARIANT",
    "OUTCOME_OPEN",
    "OUTCOME_RECORD",
    "OUTCOME_SINGLE_SCHEMA",
    "OUTCOME_SPLIT",
    "OUTCOME_SYNTHETIC_ONLY",
    "OUTCOME_WIDEN",
    "REGISTRY",
    "TIERS",
    "TIE_CRITERIA",
    "TIE_FEWER_SUBPATTERNS",
    "TIE_NEAREST_EXTREME",
    "ConstantOutcome",
    "DecisionPointSpec",
    "Outcome",
    "Registry",
    "RegistryError",
    "TechniqueSpec",
    "Tier",
    "TieRule",
]

Tier: TypeAlias = Literal["must", "should", "could"]
TIERS: Final[tuple[Tier, ...]] = ("must", "should", "could")

# The outcome label of a Decision_Point resolution, as in Candidate.outcome
# and CandidateScore.outcome.
Outcome: TypeAlias = str

# Outcome labels of the Fallback_Behaviors and tie rules. "none" is the
# absence of the Constraint, rule, partition, or suggestion in question.
OUTCOME_RECORD: Final = "record"
OUTCOME_OPEN: Final = "open"
OUTCOME_ONE_VARIANT: Final = "one-variant"
OUTCOME_SPLIT: Final = "split"
OUTCOME_LIST: Final = "list"
OUTCOME_BASE_ARRAY_SCHEMA: Final = "base-array-schema"  # the array schema of 7.2–7.4
OUTCOME_INLINE: Final = "inline"
OUTCOME_NONE: Final = "none"
OUTCOME_NOT_FLAGGED: Final = "not-flagged"
OUTCOME_WIDEN: Final = "widen"
OUTCOME_SINGLE_SCHEMA: Final = "single-schema"
OUTCOME_CANDIDATE_ORDER: Final = "candidate-order"  # first candidate in the documented order
OUTCOME_SYNTHETIC_ONLY: Final = "synthetic-only"

# Tie-rule ranking criteria that are not a single outcome label.
TIE_FEWER_SUBPATTERNS: Final = "fewer-subpatterns"  # the Pattern with fewer sub-patterns (14.2)
TIE_NEAREST_EXTREME: Final = "nearest-observed-extreme"  # the bound nearest the observed extreme (16.7)
TIE_CRITERIA: Final[tuple[str, ...]] = (TIE_FEWER_SUBPATTERNS, TIE_NEAREST_EXTREME)

_ID_RE: Final = re.compile(r"[a-z]+\.[a-z][a-z0-9_]*")
_KIND_RE: Final = re.compile(r"[a-z][a-z_]*")
_REQUIREMENT_RE: Final = re.compile(r"[1-9][0-9]*\.[1-9][0-9]*")
_LABEL_RE: Final = re.compile(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*")
_SAMPLE_PREFIX: Final = "sample."


class RegistryError(ValueError):
    """An inconsistent Technique or Decision_Point table; ``problems`` lists every finding."""

    def __init__(self, problems: Iterable[str]) -> None:
        self.problems: tuple[str, ...] = tuple(problems)
        super().__init__("invalid Technique registry:\n  " + "\n  ".join(self.problems))


# ---------------------------------------------------------------------------
# Construction checks
# ---------------------------------------------------------------------------


def _text(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a str, got {type(value).__name__}")
    if not value:
        raise ValueError(f"{what} must be non-empty")
    return value


def _matching(value: Any, pattern: re.Pattern[str], what: str) -> str:
    text = _text(value, what)
    if pattern.fullmatch(text) is None:
        raise ValueError(f"{what} {text!r} does not match {pattern.pattern!r}")
    return text


def _flag(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise TypeError(f"{what} must be a bool, got {type(value).__name__}")
    return value


def _texts(value: Any, what: str, pattern: re.Pattern[str] | None = None) -> tuple[str, ...]:
    """A tuple of distinct non-empty strings, in the given order."""
    if isinstance(value, (str, bytes)) or not isinstance(value, (tuple, list)):
        raise TypeError(f"{what} must be a tuple of str, got {type(value).__name__}")
    entry = f"{what} entry"
    items = tuple(_text(item, entry) if pattern is None else _matching(item, pattern, entry) for item in value)
    duplicates = sorted({item for item in items if items.count(item) > 1})
    if duplicates:
        raise ValueError(f"{what} has duplicate entries {duplicates}")
    return items


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


# ---------------------------------------------------------------------------
# Specs
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ConstantOutcome:
    """A Fallback_Behavior: the same ``outcome`` for every view, invoking no Technique (28.3).

    Calling it with a view (or with none) returns ``outcome``; the view is
    never inspected. A plain dataclass rather than a closure, so the
    registry pickles into worker processes.
    """

    outcome: Outcome

    def __post_init__(self) -> None:
        _matching(self.outcome, _LABEL_RE, "fallback outcome")

    def __call__(self, view: View | None = None) -> Outcome:
        return self.outcome


@dataclass(frozen=True, slots=True)
class TieRule:
    """A tie rule a requirement mandates, applied before the generic 19.4 chain.

    Among candidates with equal description length, ``favors`` is applied in
    order. Each entry is either an outcome label (the tie goes to the
    candidate with that outcome) or a ranking criterion from
    :data:`TIE_CRITERIA`. ``doc`` is the rule as the design states it, and
    ``requirements`` the criteria that mandate it.
    """

    favors: tuple[str, ...]
    doc: str
    requirements: tuple[str, ...]

    def __post_init__(self) -> None:
        favors = _texts(self.favors, "tie rule favors", _LABEL_RE)
        if not favors:
            raise ValueError("a tie rule must favor at least one outcome or criterion")
        _set(self, "favors", favors)
        _text(self.doc, "tie rule doc")
        requirements = _texts(self.requirements, "tie rule requirements", _REQUIREMENT_RE)
        if not requirements:
            raise ValueError("a tie rule must name the requirement that mandates it")
        _set(self, "requirements", requirements)


@dataclass(frozen=True, slots=True)
class DecisionPointSpec:
    """One Decision_Point kind, its serving Techniques, and its Fallback_Behavior.

    ``served_by`` lists the serving Technique ids in priority order (rank 1
    first; 19.7, 28.6, 28.7). ``fallback`` is the kind's single
    Fallback_Behavior (28.3) and ``fallback_doc`` its description for
    ``list-techniques`` (28.10). ``serves_only_with`` maps a serving Technique
    to another Technique that must also be enabled for it to serve this kind
    (15.7). ``notes`` documents how candidates are selected where the
    requirements say more than the generic resolution order.

    ``serves_only_with`` is stored as a new dict in ``served_by`` order; treat
    it as read-only.
    """

    kind: str
    title: str
    served_by: tuple[str, ...]
    fallback_doc: str
    fallback: ConstantOutcome
    requirements: tuple[str, ...]
    tie_rule: TieRule | None = None
    serves_only_with: Mapping[str, str] = field(default_factory=dict)
    notes: str = ""

    def __post_init__(self) -> None:
        kind = _matching(self.kind, _KIND_RE, "Decision_Point kind")
        _text(self.title, f"{kind} title")
        served_by = _texts(self.served_by, f"{kind} served_by", _ID_RE)
        if not served_by:
            raise ValueError(f"Decision_Point {kind!r} must be served by at least one Technique")
        _set(self, "served_by", served_by)
        _text(self.fallback_doc, f"{kind} fallback_doc")
        if not isinstance(self.fallback, ConstantOutcome):
            raise TypeError(
                f"the Fallback_Behavior of {kind!r} must be a ConstantOutcome, which invokes no Technique (28.3), "
                f"got {type(self.fallback).__name__}"
            )
        requirements = _texts(self.requirements, f"{kind} requirements", _REQUIREMENT_RE)
        if not requirements:
            raise ValueError(f"Decision_Point {kind!r} must name the requirements that define its fallback")
        _set(self, "requirements", requirements)
        if self.tie_rule is not None and not isinstance(self.tie_rule, TieRule):
            raise TypeError(f"{kind} tie_rule must be a TieRule or None, got {type(self.tie_rule).__name__}")
        if not isinstance(self.serves_only_with, Mapping):
            raise TypeError(f"{kind} serves_only_with must be a mapping, got {type(self.serves_only_with).__name__}")
        gates = dict(self.serves_only_with)
        for technique in sorted(gates):
            gate = _matching(gates[technique], _ID_RE, f"{kind} serves_only_with[{technique!r}]")
            if technique not in served_by:
                raise ValueError(f"{kind} serves_only_with names {technique!r}, which does not serve {kind!r}")
            if gate == technique:
                raise ValueError(f"{kind} serves_only_with gates {technique!r} on itself")
        _set(self, "serves_only_with", {t: gates[t] for t in served_by if t in gates})
        if not isinstance(self.notes, str):
            raise TypeError(f"{kind} notes must be a str, got {type(self.notes).__name__}")

    def rank(self, technique: str) -> int:
        """The 1-based rank of ``technique`` in this kind's priority order."""
        try:
            return self.served_by.index(technique) + 1
        except ValueError:
            raise KeyError(f"Technique {technique!r} does not serve Decision_Point {self.kind!r}") from None


@dataclass(frozen=True, slots=True)
class TechniqueSpec:
    """One registered Technique (28.1, 29.8).

    ``title`` and ``area`` are the Technique's Table 1 entry and area.
    ``serves`` lists the Decision_Point kinds it serves, and ``priority`` maps
    each to the Technique's 1-based rank in that kind's priority order (lower
    first). ``superlinear`` marks a Technique whose runtime or memory grows
    faster than linearly in its Observations. Exactly those Techniques name a
    sample-size Resource_Parameter in ``sample_param`` (29.8). ``thresholds``
    names the Learned_Thresholds the Technique reads at inference.

    ``priority`` is stored as a new dict in ``serves`` order; treat it as
    read-only.
    """

    id: str
    title: str
    area: str
    tier: Tier
    default_enabled: bool
    implemented: bool
    serves: tuple[str, ...]
    priority: Mapping[str, int]
    prerequisites: tuple[str, ...]
    superlinear: bool
    sample_param: str | None
    thresholds: tuple[str, ...]

    def __post_init__(self) -> None:
        tid = _matching(self.id, _ID_RE, "Technique id")
        _text(self.title, f"{tid} title")
        _text(self.area, f"{tid} area")
        if self.tier not in TIERS:
            raise ValueError(f"{tid} tier must be one of {list(TIERS)}, got {self.tier!r}")
        default_enabled = _flag(self.default_enabled, f"{tid} default_enabled")
        implemented = _flag(self.implemented, f"{tid} implemented")
        if default_enabled and not implemented:
            raise ValueError(f"{tid} is not implemented, so it cannot be enabled by default")
        serves = _texts(self.serves, f"{tid} serves", _KIND_RE)
        _set(self, "serves", serves)
        if not isinstance(self.priority, Mapping):
            raise TypeError(f"{tid} priority must be a mapping, got {type(self.priority).__name__}")
        priority = dict(self.priority)
        if sorted(priority) != sorted(serves):
            raise ValueError(
                f"{tid} priority must rank exactly the served kinds {sorted(serves)}, got {sorted(priority)}"
            )
        for kind in serves:
            rank = priority[kind]
            if isinstance(rank, bool) or not isinstance(rank, int) or rank < 1:
                raise ValueError(f"{tid} priority[{kind!r}] must be an int >= 1, got {rank!r}")
        _set(self, "priority", {kind: priority[kind] for kind in serves})
        prerequisites = _texts(self.prerequisites, f"{tid} prerequisites", _ID_RE)
        if tid in prerequisites:
            raise ValueError(f"{tid} lists itself as a prerequisite")
        _set(self, "prerequisites", prerequisites)
        superlinear = _flag(self.superlinear, f"{tid} superlinear")
        if self.sample_param is None:
            if superlinear:
                raise ValueError(f"{tid} is super-linear and must name its sample-size Resource_Parameter (29.8)")
        else:
            name = _text(self.sample_param, f"{tid} sample_param")
            if not superlinear:
                raise ValueError(f"{tid} names sample_param {name!r} but is not marked super-linear")
            if not name.startswith(_SAMPLE_PREFIX) or name == _SAMPLE_PREFIX:
                raise ValueError(f"{tid} sample_param must be a {_SAMPLE_PREFIX}* Resource_Parameter, got {name!r}")
        _set(self, "thresholds", _texts(self.thresholds, f"{tid} thresholds"))


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


class Registry:
    """A validated set of Technique and Decision_Point specs with cross-references checked.

    Construction raises :class:`RegistryError` listing every problem: a
    duplicate id or kind; a served kind, serving Technique, gate, or
    prerequisite that is not registered; a ``serves``/``priority`` entry that
    disagrees with the kind's ``served_by`` order; a prerequisite cycle; and a
    Technique enabled by default whose prerequisite is not.
    """

    __slots__ = ("_decision_points", "_techniques")

    def __init__(self, techniques: Iterable[TechniqueSpec], decision_points: Iterable[DecisionPointSpec]) -> None:
        problems: list[str] = []
        self._techniques: dict[str, TechniqueSpec] = {}
        for spec in techniques:
            if not isinstance(spec, TechniqueSpec):
                raise TypeError(f"techniques must be TechniqueSpec instances, got {type(spec).__name__}")
            if spec.id in self._techniques:
                problems.append(f"duplicate Technique id {spec.id!r}")
            else:
                self._techniques[spec.id] = spec
        self._decision_points: dict[str, DecisionPointSpec] = {}
        for dp in decision_points:
            if not isinstance(dp, DecisionPointSpec):
                raise TypeError(f"decision_points must be DecisionPointSpec instances, got {type(dp).__name__}")
            if dp.kind in self._decision_points:
                problems.append(f"duplicate Decision_Point kind {dp.kind!r}")
            else:
                self._decision_points[dp.kind] = dp
        problems.extend(self._cross_reference_problems())
        if problems:
            raise RegistryError(problems)

    def _cross_reference_problems(self) -> list[str]:
        techniques, kinds = self._techniques, self._decision_points
        problems: list[str] = []
        for kind, dp in kinds.items():
            for rank, tid in enumerate(dp.served_by, start=1):
                spec = techniques.get(tid)
                if spec is None:
                    problems.append(f"{kind!r} is served by unregistered Technique {tid!r}")
                elif spec.priority.get(kind) != rank:
                    problems.append(
                        f"{tid!r} must serve {kind!r} at rank {rank} (its served_by position), "
                        f"got {spec.priority.get(kind)!r}"
                    )
            for tid, gate in dp.serves_only_with.items():
                if gate not in techniques:
                    problems.append(f"{kind!r} gates {tid!r} on unregistered Technique {gate!r}")
        for tid, spec in techniques.items():
            for kind in spec.serves:
                dp = kinds.get(kind)
                if dp is None:
                    problems.append(f"{tid!r} serves unregistered Decision_Point kind {kind!r}")
                elif tid not in dp.served_by:
                    problems.append(f"{tid!r} claims to serve {kind!r}, which does not list it in served_by")
            for prerequisite in spec.prerequisites:
                if prerequisite not in techniques:
                    problems.append(f"{tid!r} has unregistered prerequisite {prerequisite!r}")
                elif spec.default_enabled and not techniques[prerequisite].default_enabled:
                    problems.append(
                        f"{tid!r} is enabled by default but its prerequisite {prerequisite!r} is not (28.8)"
                    )
        problems.extend(self._cycle_problems())
        return problems

    def _cycle_problems(self) -> list[str]:
        """One problem per prerequisite cycle, found by depth-first search in registration order."""
        state: dict[str, int] = {}  # 1 = on the stack, 2 = done
        problems: list[str] = []

        def visit(tid: str, stack: list[str]) -> None:
            state[tid] = 1
            stack.append(tid)
            for prerequisite in self._techniques[tid].prerequisites:
                if prerequisite not in self._techniques:
                    continue
                if state.get(prerequisite) == 1:
                    cycle = [*stack[stack.index(prerequisite) :], prerequisite]
                    problems.append("prerequisite cycle " + " -> ".join(cycle))
                elif prerequisite not in state:
                    visit(prerequisite, stack)
            stack.pop()
            state[tid] = 2

        for tid in self._techniques:
            if tid not in state:
                visit(tid, [])
        return problems

    # -- lookup -------------------------------------------------------------

    @property
    def techniques(self) -> Mapping[str, TechniqueSpec]:
        """Every registered Technique by id, in registration (Table 1) order."""
        return MappingProxyType(self._techniques)

    @property
    def decision_points(self) -> Mapping[str, DecisionPointSpec]:
        """Every Decision_Point kind, in registration order."""
        return MappingProxyType(self._decision_points)

    def ids(self) -> tuple[str, ...]:
        """Every registered Technique id, in registration order."""
        return tuple(self._techniques)

    def kinds(self) -> tuple[str, ...]:
        """Every registered Decision_Point kind, in registration order."""
        return tuple(self._decision_points)

    def technique(self, tid: str) -> TechniqueSpec:
        """The spec of ``tid``; ``KeyError`` naming every registered id when it is not registered."""
        try:
            return self._techniques[tid]
        except KeyError:
            raise KeyError(f"unregistered Technique {tid!r}; registered: {', '.join(self._techniques)}") from None

    def decision_point(self, kind: str) -> DecisionPointSpec:
        """The spec of Decision_Point ``kind``; ``KeyError`` when it is not registered."""
        try:
            return self._decision_points[kind]
        except KeyError:
            raise KeyError(f"unregistered Decision_Point kind {kind!r}") from None

    def default_states(self) -> dict[str, bool]:
        """The registered default state of every Technique, in registration order (28.2)."""
        return {tid: spec.default_enabled for tid, spec in self._techniques.items()}

    def servers(self, kind: str) -> tuple[str, ...]:
        """The Techniques that serve ``kind``, in priority order (rank 1 first)."""
        return self.decision_point(kind).served_by

    def rank(self, tid: str, kind: str) -> int:
        """The 1-based rank of ``tid`` in the priority order of ``kind``."""
        return self.decision_point(kind).rank(tid)

    def fallback(self, kind: str) -> ConstantOutcome:
        """The Fallback_Behavior of ``kind`` (28.3)."""
        return self.decision_point(kind).fallback

    # -- effective serving --------------------------------------------------

    def _enabled(self, tid: str, enabled: Mapping[str, bool]) -> bool:
        return bool(enabled[tid]) if tid in enabled else self.technique(tid).default_enabled

    def active_servers(self, kind: str, enabled: Mapping[str, bool]) -> tuple[str, ...]:
        """The serving Techniques of ``kind`` that are in effect under ``enabled``, in priority order.

        A serving Technique is in effect when it is enabled and, if the kind
        gates it, its gate is enabled too (15.7). Ids missing from
        ``enabled`` take their registered default state (28.2).
        """
        dp = self.decision_point(kind)
        active: list[str] = []
        for tid in dp.served_by:
            gate = dp.serves_only_with.get(tid)
            if self._enabled(tid, enabled) and (gate is None or self._enabled(gate, enabled)):
                active.append(tid)
        return tuple(active)

    def fallback_applies(self, kind: str, enabled: Mapping[str, bool]) -> bool:
        """Whether every serving Technique of ``kind`` is out of effect, so its Fallback_Behavior resolves it (28.4)."""
        return not self.active_servers(kind, enabled)


# ---------------------------------------------------------------------------
# Decision_Point table (design: "Decision_Point kinds, serving Techniques in
# priority order, and Fallback_Behaviors"; tie rules from the DecisionEngine
# section)
# ---------------------------------------------------------------------------

_DECISION_POINTS: Final[tuple[DecisionPointSpec, ...]] = (
    DecisionPointSpec(
        kind="record_or_map",
        title="Record-or-Map",
        served_by=("decide.dm_record_map", "decide.mdl"),
        fallback_doc="Record",
        fallback=ConstantOutcome(OUTCOME_RECORD),
        requirements=("4.10",),
        tie_rule=TieRule((OUTCOME_RECORD,), "Record", ("4.10",)),
    ),
    DecisionPointSpec(
        kind="record_map_partition",
        title="Hybrid Record/Map partition",
        served_by=("structure.hybrid_record_map",),
        fallback_doc="no partition",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("4.5",),
    ),
    DecisionPointSpec(
        kind="closed_or_open_record",
        title="Closed-or-Open-Record",
        served_by=("decide.dm_closed_record", "decide.mdl"),
        fallback_doc="open (matches Naive_Merge)",
        fallback=ConstantOutcome(OUTCOME_OPEN),
        requirements=("4.7",),
    ),
    DecisionPointSpec(
        kind="variant_grouping",
        title="Variant grouping",
        served_by=("variants.minhash_lsh", "variants.dbscan", "variants.hdbscan", "variants.bernoulli_mixture"),
        fallback_doc="one Variant",
        fallback=ConstantOutcome(OUTCOME_ONE_VARIANT),
        requirements=("5.10",),
        notes="The MDL_Scorer selects among the groupings when decide.mdl is enabled, else the priority order (5.11).",
    ),
    DecisionPointSpec(
        kind="merge_or_split",
        title="Merge-or-Split (at one Path, or across Paths for $defs)",
        served_by=("decide.dm_merge_split", "decide.mdl"),
        fallback_doc="keep separate (split)",
        fallback=ConstantOutcome(OUTCOME_SPLIT),
        requirements=("5.11", "8.3"),
    ),
    DecisionPointSpec(
        kind="field_groups",
        title="Field_Group selection",
        served_by=("groups.fpgrowth_slim", "groups.bmf"),
        fallback_doc="no Field_Groups",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("6.9",),
    ),
    DecisionPointSpec(
        kind="tuple_or_list",
        title="Tuple-or-List",
        served_by=("decide.dm_tuple_list", "decide.mdl"),
        fallback_doc="List",
        fallback=ConstantOutcome(OUTCOME_LIST),
        requirements=("7.10",),
        # 7.10: Tuple only with a strictly lower description length.
        tie_rule=TieRule((OUTCOME_LIST,), "List", ("7.10",)),
    ),
    DecisionPointSpec(
        kind="array_pattern",
        title="Array_Pattern",
        served_by=("structure.array_grammar",),
        fallback_doc="the array schema of 7.2–7.4",
        fallback=ConstantOutcome(OUTCOME_BASE_ARRAY_SCHEMA),
        requirements=("7.5",),
        tie_rule=TieRule((OUTCOME_BASE_ARRAY_SCHEMA,), "the schema of 7.2–7.4", ("7.8",)),
    ),
    DecisionPointSpec(
        kind="shared_definition",
        title="Shared_Definition extraction",
        served_by=("structure.shared_defs", "structure.recursion"),
        fallback_doc="inline",
        fallback=ConstantOutcome(OUTCOME_INLINE),
        requirements=("8.7", "8.8"),
    ),
    DecisionPointSpec(
        kind="cooccurrence_rule",
        title="Co_Occurrence_Rule inclusion",
        served_by=("rules.cooccurrence",),
        fallback_doc="no rules",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("9.9",),
        notes="A rule is included by MDL when decide.mdl is enabled, else by its Confidence threshold (9.4, 9.8).",
    ),
    DecisionPointSpec(
        kind="discriminator",
        title="Discriminator selection",
        served_by=("rules.decision_trees",),
        fallback_doc="none",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("10.6",),
    ),
    DecisionPointSpec(
        kind="exclusion_rule",
        title="Exclusion_Rule inclusion",
        served_by=("rules.exclusion",),
        fallback_doc="none",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("11.7",),
    ),
    DecisionPointSpec(
        kind="key",
        title="Key_Constraint",
        served_by=("rules.keys",),
        fallback_doc="none",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("12.9",),
    ),
    DecisionPointSpec(
        kind="reference",
        title="Reference_Constraint",
        served_by=("rules.fd_ind",),
        fallback_doc="none",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("12.10",),
    ),
    DecisionPointSpec(
        kind="fd",
        title="Functional_Dependency",
        served_by=("rules.fd_ind",),
        fallback_doc="none",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("12.10",),
    ),
    DecisionPointSpec(
        kind="string_constraint",
        title="String Format or Pattern",
        served_by=(
            "values.latent_type",
            "values.format",
            "values.semantic_type",
            "corpus.name_suggest",
            "corpus.cf_suggest",
            "values.pattern",
        ),
        fallback_doc="none",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("13.9", "14.6"),
        # Pattern selection offers "no Pattern" as a candidate (14.2); Format
        # selection (13.8) does not, so the first preference only matters there.
        tie_rule=TieRule((OUTCOME_NONE, TIE_FEWER_SUBPATTERNS), "no Pattern, then fewer sub-patterns", ("14.2",)),
    ),
    DecisionPointSpec(
        kind="closed_or_open_enum",
        title="Closed-or-Open-Enum",
        served_by=("decide.dm_closed_enum", "values.enum_estimation"),
        fallback_doc="open",
        fallback=ConstantOutcome(OUTCOME_OPEN),
        requirements=("15.7",),
        tie_rule=TieRule((OUTCOME_OPEN,), "open", ("15.6",)),
        serves_only_with={"decide.dm_closed_enum": "values.enum_estimation"},
        notes="decide.dm_closed_enum serves only while values.enum_estimation is enabled (15.7).",
    ),
    DecisionPointSpec(
        kind="bound",
        title="Bound generalization",
        served_by=("values.bounds",),
        fallback_doc="omit",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("16.8",),
        tie_rule=TieRule(
            (TIE_NEAREST_EXTREME,),
            "the candidate nearest the observed extreme, with omission as the farthest",
            ("16.7",),
        ),
    ),
    DecisionPointSpec(
        kind="document_outlier",
        title="Document outlier flagging",
        served_by=("robust.isolation_forest", "robust.lof"),
        fallback_doc="not flagged",
        fallback=ConstantOutcome(OUTCOME_NOT_FLAGGED),
        requirements=("17.7",),
    ),
    DecisionPointSpec(
        kind="value_exception",
        title="MDL exception encoding",
        served_by=("robust.mdl_exceptions",),
        fallback_doc="widen",
        fallback=ConstantOutcome(OUTCOME_WIDEN),
        requirements=("17.8",),
        tie_rule=TieRule((OUTCOME_WIDEN,), "widening", ("17.2", "17.3")),
    ),
    DecisionPointSpec(
        kind="version_split",
        title="Schema_Version split",
        served_by=("robust.change_point",),
        fallback_doc="single schema",
        fallback=ConstantOutcome(OUTCOME_SINGLE_SCHEMA),
        requirements=("18.10",),
    ),
    DecisionPointSpec(
        kind="suggestion",
        title="Corpus suggestion",
        served_by=("corpus.name_suggest", "corpus.cf_suggest"),
        fallback_doc="no suggestion",
        fallback=ConstantOutcome(OUTCOME_NONE),
        requirements=("24.8",),
    ),
    DecisionPointSpec(
        kind="relaxation",
        title="Soundness relaxation",
        served_by=("decide.mdl",),
        fallback_doc=(
            "documented candidate order (least widening first: add value/type, "
            "loosen to next bound candidate, re-induce, then remove)"
        ),
        fallback=ConstantOutcome(OUTCOME_CANDIDATE_ORDER),
        requirements=("3.3",),
    ),
    DecisionPointSpec(
        kind="judgment_model",
        title="Decision_Model training source",
        served_by=("decide.weak_supervision",),
        fallback_doc="synthetic-only Decision_Model",
        fallback=ConstantOutcome(OUTCOME_SYNTHETIC_ONLY),
        requirements=("23.9",),
    ),
)

# ---------------------------------------------------------------------------
# Technique table (Table 1 of Requirement 28, design "Technique registry")
# ---------------------------------------------------------------------------

# Table 1 area of each id prefix.
_AREAS: Final[Mapping[str, str]] = MappingProxyType(
    {
        "variants": "Variant discovery",
        "groups": "Field groups",
        "structure": "Structure",
        "rules": "Inter-field rules",
        "values": "Values",
        "robust": "Robustness",
        "decide": "Decisions",
        "corpus": "Corpus",
    }
)


def _technique(
    tid: str,
    title: str,
    tier: Tier,
    *,
    implemented: bool,
    prerequisites: tuple[str, ...] = (),
    sample_param: str | None = None,
    thresholds: tuple[str, ...] = (),
) -> TechniqueSpec:
    """A table row. Served kinds and ranks come from ``_DECISION_POINTS``, so the two tables cannot disagree.

    Every implemented Technique is enabled by default, and only super-linear
    Techniques have a sample-size parameter.
    """
    serving = [dp for dp in _DECISION_POINTS if tid in dp.served_by]
    return TechniqueSpec(
        id=tid,
        title=title,
        area=_AREAS[tid.partition(".")[0]],
        tier=tier,
        default_enabled=implemented,
        implemented=implemented,
        serves=tuple(dp.kind for dp in serving),
        priority={dp.kind: dp.rank(tid) for dp in serving},
        prerequisites=prerequisites,
        superlinear=sample_param is not None,
        sample_param=sample_param,
        thresholds=thresholds,
    )


_TECHNIQUES: Final[tuple[TechniqueSpec, ...]] = (
    # Variant discovery (task 13.2, structure/variants.py)
    _technique("variants.minhash_lsh", "MinHash/LSH candidate grouping", "must",
               implemented=True, thresholds=("lsh.keyset_jaccard",)),
    _technique("variants.dbscan", "DBSCAN", "must",
               implemented=True, sample_param="sample.dbscan", thresholds=("dbscan.eps", "dbscan.min_samples")),
    _technique("variants.hdbscan", "HDBSCAN", "must",
               implemented=True, sample_param="sample.hdbscan", thresholds=("hdbscan.min_cluster_size",)),
    _technique("variants.bernoulli_mixture", "Bernoulli mixture with BIC", "must",
               implemented=True),
    # Field groups
    _technique("groups.fpgrowth_slim", "FP-growth with SLIM/Krimp-style MDL selection", "should",
               implemented=True, sample_param="sample.fpgrowth"),
    _technique("groups.bmf", "Boolean matrix factorization", "could",
               implemented=False, sample_param="sample.bmf"),
    # Structure
    _technique("structure.hybrid_record_map", "Hybrid Record/Map partitioning", "should",
               implemented=True, prerequisites=("decide.mdl",)),
    _technique("structure.shared_defs", "Shared_Definition extraction", "should",
               implemented=True),
    _technique("structure.recursion", "recursion detection", "should",
               implemented=True),
    _technique("structure.array_grammar", "array grammar inference", "could",
               implemented=False),
    # Inter-field rules (co-occurrence: task 14.2, deps/cooccurrence.py)
    _technique("rules.cooccurrence", "Co-occurrence mining", "must",
               implemented=True, thresholds=("cooccur.min_confidence", "cooccur.min_support")),
    _technique("rules.decision_trees", "per-field decision trees for Tagged_Unions", "should",
               implemented=True, sample_param="sample.trees", thresholds=("tagged.branch_support",)),
    _technique("rules.exclusion", "exclusion mining", "should",
               implemented=True, thresholds=("exclusion.min_confidence",)),
    _technique("rules.keys", "key discovery", "could",
               implemented=False, thresholds=("keys.min_confidence",)),
    _technique("rules.fd_ind", "functional and inclusion dependency discovery", "could",
               implemented=False, sample_param="sample.fd", thresholds=("fd.min_confidence", "ind.min_confidence")),
    # Values
    _technique("values.latent_type", "Latent-type detection", "must",
               implemented=True),
    _technique("values.format", "Format detection", "must",
               implemented=True),
    _technique("values.semantic_type", "Semantic_Type_Classifier", "could",
               implemented=False, thresholds=("semantic.min_probability",)),
    _technique("values.pattern", "FlashProfile-style pattern induction", "should",
               implemented=True, sample_param="sample.patterns"),
    _technique("values.enum_estimation", "Good–Turing/Chao1 enum estimation", "must",
               implemented=True),
    _technique("values.bounds", "bound generalization", "must",
               implemented=True),
    # Robustness
    _technique("robust.isolation_forest", "Isolation Forest", "should",
               implemented=False, thresholds=("iforest.score",)),
    _technique("robust.lof", "Local Outlier Factor", "should",
               implemented=False, sample_param="sample.lof", thresholds=("lof.n_neighbors", "lof.score")),
    _technique("robust.mdl_exceptions", "MDL exception encoding", "should",
               implemented=False),
    _technique("robust.change_point", "change-point detection", "could",
               implemented=False, sample_param="sample.changepoint",
               thresholds=("changepoint.min_segment", "changepoint.penalty")),
    # Decisions
    # Generic MDL selection lives in the DecisionEngine (task 11); Record-or-Map and
    # Closed-or-Open-Record (task 13.1) are its first Decision_Points.
    _technique("decide.mdl", "MDL selection", "must",
               implemented=True),
    _technique("decide.dm_record_map", "Record-or-Map Decision_Model", "must",
               implemented=True),
    _technique("decide.dm_closed_record", "Closed-or-Open-Record Decision_Model", "must",
               implemented=True),
    _technique("decide.dm_tuple_list", "Tuple-or-List Decision_Model", "must",
               implemented=True),
    _technique("decide.dm_closed_enum", "Closed-or-Open-Enum Decision_Model", "must",
               implemented=True),
    _technique("decide.dm_merge_split", "Merge-or-Split Decision_Model", "must",
               implemented=True),
    # The Labeling_Function thresholds lf.<id>.* are read by the Trainer, not at inference.
    _technique("decide.weak_supervision", "weak supervision", "should",
               implemented=True),
    # Corpus
    _technique("corpus.name_suggest", "Name-based suggestions", "should",
               implemented=False, thresholds=("corpus.suggest_min_confidence",)),
    _technique("corpus.cf_suggest", "collaborative-filtering suggestions", "could",
               implemented=False, thresholds=("corpus.suggest_min_confidence",)),
    # Serves no Decision_Point: it only reports the Nearest_Known_Schema (25.5).
    _technique("corpus.nearest_schema", "nearest-known-schema lookup", "should",
               implemented=False),
)

REGISTRY: Final = Registry(_TECHNIQUES, _DECISION_POINTS)
