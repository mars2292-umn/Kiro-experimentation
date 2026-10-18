"""Co_Occurrence_Rule mining, selection, and canonical transitive reduction (Requirements 9.3–9.6, 9.8, 9.9).

Everything here works on one Variant: the non-flagged objects of one final
Variant at an object Path, through their :class:`~jsinfer.deps.presence.Presence`
(packed presence bitsets, task 14.1).

Candidates (9.3)
    A Co_Occurrence_Rule candidate ``a → b`` is a pair of fields ``a ≠ b``
    such that ``a`` is held by at least one object, every object that holds
    ``a`` also holds ``b`` (``rows(a) ⊆ rows(b)``, the packed-bitset test of
    :meth:`Presence.implication_matrix`), and ``b`` is optional for the
    Variant. Its Support count is ``count(a)``, the number of the Variant's
    non-flagged objects that hold the antecedent (9.6). An antecedent is
    then always optional too: a required antecedent would make its
    consequent required. :func:`rule_candidates` lists every candidate as a
    :class:`RuleCandidate`, ordered by antecedent and then consequent in
    code-point order, so it compares directly with a brute-force reference
    (Property 17, task 14.3).

Support filter (9.6)
    A candidate whose Support count is below the Learned_Threshold
    ``cooccur.min_support`` is discarded before selection. Each discarded
    candidate is recorded in the Inference_Report's ``rejected.rules`` group
    with the Path, the Variant (its index among the Path's final Variants),
    the antecedent and consequent, the Support count, and the threshold's
    name and value.

Selection (9.4, 9.8)
    Each remaining candidate is one ``cooccurrence_rule`` Decision_Point at
    the Variant's Path, resolved by the DecisionEngine.

    - While ``decide.mdl`` is enabled (9.4) the engine compares two
      candidates by MDL: ``none``, the Variant subschema as it is (no
      Technique produces it), and the rule, produced by
      ``rules.cooccurrence``, which adds the entry ``{a: [b]}`` to the
      subschema's ``dependentRequired`` and is labeled with that entry as
      compact JSON. The rule is selected only when it lowers the
      description length: adding an entry always costs model bits, so a tie
      goes to ``none`` by the engine's fewer-model-bits rule (no tie rule is
      mandated). The Confidence of a selected rule is its MDL posterior,
      and its cut-off is the "MDL comparison" with both scores.
    - Otherwise (9.8) the rule is selected when its Confidence is at or
      above the Learned_Threshold ``cooccur.min_confidence``. The engine
      then resolves the point by the registry priority order with the rule
      as its only candidate, recording ``cooccur.min_confidence`` and its
      value as the cut-off. A candidate below the threshold produces no
      Constraint and has no other candidate, so it is recorded in the
      Inference_Report's ``rejected.rules`` group with its Confidence and the
      threshold's name and value instead (20.7).

    The Confidence of a candidate (:attr:`RuleCandidate.confidence`) is the
    Laplace rule of succession over the objects that hold the antecedent,
    ``(count(a) + 1) / (count(a) + 2)``: every one of them holds the
    consequent, so this estimates the probability that the next object
    holding ``a`` also holds ``b``. It lies in (0, 1) and grows with the
    Support count.

Scoring (19.3)
    The Variant subschema is the caller's ``record`` (the Variant's Record
    node, with its ``required`` list and any Field_Group ``dependentRequired``
    entries), or by default the Naive_Merge subschema of the Variant's
    objects plus the Field_Group entries of ``groups``: descendants are
    represented by their Naive_Merge subschemas (local scoring). Every
    candidate is scored on the Variant's object rows. A candidate holds on
    every non-flagged object, so adding it rejects no row and leaves every
    member value's coding unchanged: its data terms are those of the
    subschema with the Record presence terms replaced by its own. The
    subschema is therefore scored once in full, and each rule's score is
    its own model bits plus the subschema's data bits with the presence
    bits (:meth:`~jsinfer.mdl.scorer.MDLScorer.presence_bits`) exchanged,
    which equals scoring the rule's node in full up to float rounding and
    costs O(objects × fields) per candidate instead of a full scoring.

Reduction (9.5)
    :func:`reduce_rules` turns the selected rules into the emitted subset,
    working on the implication graph of the selected rules plus the
    ``given`` rules (the ``dependentRequired`` cliques of the Variant's
    Field_Groups, 6.6), which are not emitted again:

    1. Strongly connected components (SCCs) of that graph, and the units
       inside them: the SCCs of the given rules alone (one Field_Group, or
       a single field). Every member of an SCC holds exactly the same
       objects.
    2. Each SCC with two or more units collapses to a canonical cycle over
       its units' representatives (their code-point-first members) in
       code-point order: ``u1 → u2 → … → ur → u1``. The members of an SCC
       have equal Support counts and, against a Record without other
       ``dependentRequired`` entries, equal description-length changes, so
       the miner selects their rules all or none and the cycle edges are
       selected rules. When one is not (next to Field_Group entries, or for
       a rule set given directly), the SCC keeps instead an irredundant
       subset of its selected rules: edges are dropped, cycle edges last and
       each group in descending code-point order, whenever the rest still
       derive them.
    3. Between SCCs, an edge of the condensation is dropped when the given
       rules state it or a path through a third SCC implies it (the
       transitive reduction of a DAG, which is unique). Each kept edge is
       emitted as the code-point-first selected rule between the two SCCs.

    The result derives every selected rule by transitive chaining together
    with the given rules (9.5 a), and no emitted rule follows from the other
    emitted rules and the given ones (9.5 b). The ``required`` list cannot
    take part in a derivation: a chain through a required field would make
    the optional consequent required. Rules are pairs of names, emitted in
    code-point order of antecedent, then consequent.

Emission and records
    :meth:`CooccurrenceRules.keywords` gives ``{"dependentRequired": {a: [b,
    …]}}`` with antecedents and consequents in code-point order, or ``{}``
    when nothing is emitted. Each entry is one Constraint,
    ``(nid, "dependentRequired", a)`` at ``…/dependentRequired/<a>`` (the
    design's granularity table). Its Provenance_Record comes from the
    decision of the entry's weakest rule (lowest Confidence, then the first
    consequent): its Technique (``rules.cooccurrence``), method, candidates
    with scores, and cut-off, with that rule's Confidence. Support is every
    non-flagged object of the Variant, each of which satisfies the entry;
    provenance finalization (task 17.1) recomputes it after the soundness
    check. The details hold the antecedent, its Support count, the Variant,
    the thresholds read, every emitted rule of the entry with its
    Confidence, resolution, and cut-off (so every Decision_Point that
    produced the entry is on record, 26.3), and the selected rules with this
    antecedent that the reduction left implied. Every decision is also in
    the engine's Inference_Report entries (20.7).

Fallback (9.9, 20.6)
    While ``rules.cooccurrence`` is disabled, or the Model_Bundle lacks one of
    its Learned_Thresholds (``cooccur.min_support`` and
    ``cooccur.min_confidence``, both registered at this Decision_Point, so
    the engine needs both before the Technique is in effect), no candidate
    is reported. The engine resolves the Variant's ``cooccurrence_rule``
    point with its Fallback_Behavior, "no rules", and records it with the
    fallback indicator and the reason. The Variant's ``required`` list
    (9.7, task 14.1) and Field_Group entries (6.6) are unaffected.

Hooks
    The analyzer walk (task 16.1, design walk step 2.5) calls
    :meth:`CooccurrenceMiner.rules` for each final Variant after
    :func:`~jsinfer.deps.presence.required_fields`, passing
    ``RequiredFields.presence``, the Variant's Record node as ``record``,
    the selected Field_Groups as ``groups``, and the Variant's branch index
    as ``variant``. It adds :meth:`CooccurrenceRules.keywords` to the node
    and collects :meth:`CooccurrenceRules.constraint_provenance`. Field_Group
    selection (task 33.1) emits its own ``dependentRequired`` cliques; when
    an antecedent has both kinds of entry, they merge into one entry (one
    Constraint), whose record that task combines. The Trainer (tasks 22.5,
    23.2) fits ``cooccur.min_support`` and ``cooccur.min_confidence`` for
    the registered objective ``cooccurrence_f1`` (F1 of the emitted
    ``dependentRequired`` entries versus the source schema), on the Support
    counts and Confidences of :func:`rule_candidates`.

Determinism
    Fields are in code-point order (key ids follow it), candidates and
    decisions follow that order, the reduction depends only on the sets of
    rules, and scores depend only on the multiset of non-flagged
    Observations, so the outcome does not depend on document order,
    chunking, or worker count. No cut-off is hard-coded: a rule is selected
    by an MDL comparison or by a Learned_Threshold. Sets are never iterated:
    the code keeps sorted lists and insertion-ordered dicts.
"""

from __future__ import annotations

import json
import math
from collections.abc import Collection, Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final

import numpy as np

from jsinfer.config.registry import OUTCOME_NONE
from jsinfer.core.determinism import sorted_codepoint
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import Cutoff, ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import MDL_SELECTION, ArtifactSource, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.deps.presence import Presence
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node

__all__ = [
    "COOCCURRENCE",
    "COOCCURRENCE_RULE",
    "DEPENDENT_REQUIRED",
    "MIN_CONFIDENCE",
    "MIN_SUPPORT",
    "NO_RULE",
    "REJECTED_CATEGORY",
    "CooccurrenceMiner",
    "CooccurrenceRules",
    "Rule",
    "RuleCandidate",
    "RuleEvaluation",
    "group_rules",
    "reduce_rules",
    "rule_candidates",
    "rule_confidence",
]

COOCCURRENCE: Final = "rules.cooccurrence"
COOCCURRENCE_RULE: Final = "cooccurrence_rule"
MIN_SUPPORT: Final = "cooccur.min_support"
MIN_CONFIDENCE: Final = "cooccur.min_confidence"
DEPENDENT_REQUIRED: Final = "dependentRequired"
NO_RULE: Final = OUTCOME_NONE  # the label of the subschema without the rule, and of the Fallback_Behavior
REJECTED_CATEGORY: Final = "rules"

# A Co_Occurrence_Rule as (antecedent, consequent).
Rule = tuple[str, str]


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _as_presence(source: View | Presence) -> Presence:
    if isinstance(source, Presence):
        return source
    if isinstance(source, View):
        return Presence.of(source)
    raise TypeError(f"expected a View or a Presence, got {type(source).__name__}")


def _field_name(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a str, got {type(value).__name__}")
    return value


# ---------------------------------------------------------------------------
# Candidates (9.3)
# ---------------------------------------------------------------------------


def rule_confidence(support_count: int) -> float:
    """The Confidence of a candidate held by ``support_count`` antecedent objects: ``(n + 1) / (n + 2)``."""
    if isinstance(support_count, bool) or not isinstance(support_count, (int, np.integer)) or support_count < 1:
        raise ValueError(f"a Support count is a positive integer, got {support_count!r}")
    return (int(support_count) + 1) / (int(support_count) + 2)


@dataclass(frozen=True, slots=True)
class RuleCandidate:
    """The candidate ``antecedent → consequent`` of 9.3, with its Support count ``count(antecedent)``."""

    antecedent: str
    consequent: str
    support_count: int

    def __post_init__(self) -> None:
        _field_name(self.antecedent, "the antecedent")
        _field_name(self.consequent, "the consequent")
        if self.antecedent == self.consequent:
            raise ValueError(f"a Co_Occurrence_Rule relates two different fields, got {self.antecedent!r} twice")
        rule_confidence(self.support_count)  # checks the count
        _set(self, "support_count", int(self.support_count))

    @property
    def rule(self) -> Rule:
        return (self.antecedent, self.consequent)

    @property
    def confidence(self) -> float:
        """The Laplace rule of succession over the antecedent's objects (see the module docstring)."""
        return rule_confidence(self.support_count)

    @property
    def entry(self) -> dict[str, list[str]]:
        """The ``dependentRequired`` entry that expresses the rule."""
        return {self.antecedent: [self.consequent]}

    @property
    def label(self) -> str:
        """The outcome label in Decision records: the entry as compact JSON, such as ``{"a":["b"]}``."""
        return json.dumps(self.entry, ensure_ascii=False, separators=(",", ":"))

    def to_json(self) -> dict[str, JSON]:
        """The candidate as recorded in the report: antecedent and consequent."""
        return {"antecedent": self.antecedent, "consequent": self.consequent}


def rule_candidates(source: View | Presence) -> tuple[RuleCandidate, ...]:
    """Every Co_Occurrence_Rule candidate of a Variant (9.3), by antecedent, then consequent, in code-point order.

    ``source`` is the Variant's object view or its :class:`Presence`. A pair
    ``a ≠ b`` is a candidate when every object holding ``a`` holds ``b`` and
    ``b`` is optional.
    """
    p = _as_presence(source)
    if not p.n_fields:
        return ()
    implied = p.implication_matrix()
    optional = p.counts < p.n_objects
    names = p.names
    out: list[RuleCandidate] = []
    for i, antecedent in enumerate(names):
        support = int(p.counts[i])
        for j in np.flatnonzero(implied[i] & optional).tolist():
            if j != i:
                out.append(RuleCandidate(antecedent, names[j], support))
    return tuple(out)


# ---------------------------------------------------------------------------
# Reduction (9.5)
# ---------------------------------------------------------------------------


def _rules(rules: Iterable[Rule], what: str) -> list[Rule]:
    """Distinct rules without self-loops, in code-point order of antecedent, then consequent."""
    found: dict[Rule, None] = {}
    for item in rules:
        if not isinstance(item, (tuple, list)) or len(item) != 2:
            raise TypeError(f"{what} must be (antecedent, consequent) pairs, got {item!r}")
        a, b = (_field_name(name, f"{what} names") for name in item)
        if a != b:
            found[(a, b)] = None
    return sorted(found, key=lambda rule: (rule[0], rule[1]))


def _closure(adjacency: np.ndarray) -> np.ndarray:
    """The reflexive transitive closure of a square bool adjacency matrix (Warshall, one vectorized row pass per node)."""
    reach = adjacency.copy()
    np.fill_diagonal(reach, True)
    for k in range(len(reach)):
        reach |= reach[:, k : k + 1] & reach[k : k + 1, :]
    return reach


def _reaches(source: int, target: int, edges: Iterable[tuple[int, int]]) -> bool:
    """Whether ``target`` is reachable from ``source`` over ``edges``."""
    successors: dict[int, list[int]] = {}
    for a, b in edges:
        successors.setdefault(a, []).append(b)
    seen = {source: None}
    todo = [source]
    while todo:
        node = todo.pop()
        for nxt in successors.get(node, ()):
            if nxt == target:
                return True
            if nxt not in seen:
                seen[nxt] = None
                todo.append(nxt)
    return False


def _irredundant(
    selected: list[tuple[int, int]], given: list[tuple[int, int]], preferred: Collection[tuple[int, int]]
) -> list[tuple[int, int]]:
    """An irredundant subset of ``selected`` that, with ``given``, keeps every reachability of ``selected``.

    Edges are tried for removal in descending order, the ``preferred`` ones last.
    """
    kept: dict[tuple[int, int], None] = dict.fromkeys(selected)
    others = sorted((edge for edge in selected if edge not in preferred), reverse=True)
    last = sorted((edge for edge in selected if edge in preferred), reverse=True)
    for edge in [*others, *last]:
        del kept[edge]
        if not _reaches(edge[0], edge[1], [*kept, *given]):
            kept[edge] = None
    return sorted(kept)


def reduce_rules(selected: Iterable[Rule], given: Iterable[Rule] = ()) -> tuple[Rule, ...]:
    """The canonical emitted subset of the ``selected`` rules (9.5); see the module docstring.

    ``given`` are rules stated anyway (the Field_Group cliques of 6.6). The
    result is a subset of ``selected`` that derives every selected rule
    together with ``given``, in which no rule follows from the others and
    ``given``; it is in code-point order of antecedent, then consequent.
    """
    chosen = _rules(selected, "selected rules")
    stated = _rules(given, "given rules")
    if not chosen:
        return ()
    names = sorted_codepoint(dict.fromkeys(name for rule in [*chosen, *stated] for name in rule))
    index = {name: i for i, name in enumerate(names)}
    sel = [(index[a], index[b]) for a, b in chosen]
    giv = [(index[a], index[b]) for a, b in stated]
    n = len(names)
    graph = np.zeros((n, n), dtype=bool)
    given_graph = np.zeros((n, n), dtype=bool)
    for a, b in sel:
        graph[a, b] = True
    for a, b in giv:
        graph[a, b] = given_graph[a, b] = True
    reach = _closure(graph)
    # Names are in code-point order, so the first mutually reachable index is the component's first member.
    component = (reach & reach.T).argmax(axis=1)
    given_reach = _closure(given_graph)
    unit = (given_reach & given_reach.T).argmax(axis=1)
    selected_edges = dict.fromkeys(sel)

    emitted: list[tuple[int, int]] = []
    # 2. Inside each SCC: a canonical cycle over its units.
    for comp in dict.fromkeys(component.tolist()):
        members = np.flatnonzero(component == comp).tolist()
        reps = list(dict.fromkeys(int(unit[m]) for m in members))
        if len(reps) < 2:
            continue
        cycle = [(reps[k], reps[(k + 1) % len(reps)]) for k in range(len(reps))]
        if all(edge in selected_edges for edge in cycle):
            emitted.extend(cycle)
            continue
        inside = dict.fromkeys(members)
        internal = [(a, b) for a, b in sel if a in inside and b in inside and unit[a] != unit[b]]
        stated_inside = [(a, b) for a, b in giv if a in inside and b in inside]
        emitted.extend(_irredundant(internal, stated_inside, dict.fromkeys(cycle)))

    # 3. Between SCCs: the transitive reduction of the condensation.
    reps_of = list(dict.fromkeys(component.tolist()))
    position = {comp: k for k, comp in enumerate(reps_of)}
    condensed = reach[np.ix_(reps_of, reps_of)].copy()
    np.fill_diagonal(condensed, False)
    stated_between = np.zeros_like(condensed)
    for a, b in giv:
        if component[a] != component[b]:
            stated_between[position[int(component[a])], position[int(component[b])]] = True
    first: dict[tuple[int, int], tuple[int, int]] = {}
    for a, b in sel:  # code-point order, so the first rule between two SCCs is kept
        x, y = position[int(component[a])], position[int(component[b])]
        if x != y:
            first.setdefault((x, y), (a, b))
    for (x, y), edge in first.items():
        if stated_between[x, y] or bool(np.any(condensed[x] & condensed[:, y])):
            continue
        emitted.append(edge)
    return tuple((names[a], names[b]) for a, b in sorted(emitted))


def group_rules(groups: Iterable[Collection[str]]) -> tuple[Rule, ...]:
    """The rules a set of Field_Groups states (6.6): each member requires every other member."""
    out: dict[Rule, None] = {}
    for group in groups:
        if isinstance(group, (str, bytes)) or not isinstance(group, Collection):
            raise TypeError(f"a Field_Group is a collection of field names, got {type(group).__name__}")
        members = sorted_codepoint(dict.fromkeys(_field_name(name, "Field_Group members") for name in group))
        if len(members) < 2:
            raise ValueError(f"a Field_Group has at least two fields, got {members!r}")
        for a in members:
            for b in members:
                if a != b:
                    out[(a, b)] = None
    return tuple(sorted(out))


# ---------------------------------------------------------------------------
# Outcomes
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class RuleEvaluation:
    """The ``cooccurrence_rule`` Decision_Point of one candidate that passed the support filter.

    ``decision`` is the engine's Decision, or None for a candidate below
    ``cooccur.min_confidence`` while MDL selection is disabled (no Constraint
    and no other candidate: it is recorded as a rejected rule). ``confidence``
    is the selected rule's Confidence (its MDL posterior, or the candidate's
    Confidence under the threshold), and the candidate's Confidence otherwise.
    """

    candidate: RuleCandidate
    selected: bool
    confidence: float
    decision: Decision | None = None

    def __post_init__(self) -> None:
        if self.selected and self.decision is None:
            raise ValueError("a selected rule is the outcome of a Decision")

    def to_json(self) -> dict[str, JSON]:
        """The rule's facts as recorded in the details of its entry's Provenance_Record."""
        out: dict[str, JSON] = {"confidence": self.confidence, "consequent": self.candidate.consequent}
        if self.decision is not None:
            out["method"] = self.decision.method
            out["resolution"] = self.decision.resolution.to_json()
            if self.decision.cutoff is not None:
                out["cutoff"] = self.decision.cutoff.to_json()
        return out


@dataclass(frozen=True, eq=False)
class CooccurrenceRules:
    """The Co_Occurrence_Rules of one Variant (see the module docstring).

    ``candidates`` are every 9.3 candidate (none under the Fallback_Behavior),
    ``discarded`` those below ``cooccur.min_support``, ``evaluations`` the
    Decision_Points of the others in candidate order, and ``emitted`` the
    9.5 reduction of the selected rules given ``given`` (the Field_Group
    rules). ``thresholds`` holds the Learned_Thresholds read, and
    ``fallback`` the Fallback_Behavior's Decision when the Technique was out
    of effect.
    """

    presence: Presence
    variant: int = 0
    candidates: tuple[RuleCandidate, ...] = ()
    discarded: tuple[RuleCandidate, ...] = ()
    evaluations: tuple[RuleEvaluation, ...] = ()
    emitted: tuple[Rule, ...] = ()
    given: tuple[Rule, ...] = ()
    thresholds: Mapping[str, int | float] = field(default_factory=dict)
    fallback: Decision | None = None
    _records: Mapping[str, ProvenanceRecord] = field(init=False, repr=False)

    def __post_init__(self) -> None:
        _set(self, "thresholds", MappingProxyType(dict(self.thresholds)))
        selected = dict.fromkeys(self.selected)
        if any(rule not in selected for rule in self.emitted):
            raise ValueError("the emitted rules are a subset of the selected ones (9.5)")
        _set(self, "_records", MappingProxyType(self._build_records()))

    # -- inspection ---------------------------------------------------------

    @property
    def path(self) -> Path:
        return self.presence.path

    @property
    def selected(self) -> tuple[Rule, ...]:
        """The rules selected by MDL or by the Confidence threshold (9.4, 9.8), in candidate order."""
        return tuple(e.candidate.rule for e in self.evaluations if e.selected)

    @property
    def implied(self) -> tuple[Rule, ...]:
        """The selected rules that the reduction left out because the emitted and given rules derive them."""
        emitted = dict.fromkeys(self.emitted)
        return tuple(rule for rule in self.selected if rule not in emitted)

    @property
    def is_empty(self) -> bool:
        """True when no ``dependentRequired`` entry is emitted."""
        return not self.emitted

    def entries(self) -> dict[str, list[str]]:
        """The emitted ``dependentRequired`` entries: antecedent → consequents, both in code-point order."""
        out: dict[str, list[str]] = {}
        for antecedent, consequent in self.emitted:  # already in code-point order
            out.setdefault(antecedent, []).append(consequent)
        return out

    def keywords(self) -> dict[str, dict[str, list[str]]]:
        """``{"dependentRequired": {...}}`` with fresh lists, or ``{}`` when no rule is emitted (9.4, 9.8)."""
        entries = self.entries()
        return {DEPENDENT_REQUIRED: entries} if entries else {}

    # -- records ------------------------------------------------------------

    def _build_records(self) -> dict[str, ProvenanceRecord]:
        evaluations = {e.candidate.rule: e for e in self.evaluations}
        implied: dict[str, list[str]] = {}
        for antecedent, consequent in self.implied:
            implied.setdefault(antecedent, []).append(consequent)
        n = self.presence.n_objects
        records: dict[str, ProvenanceRecord] = {}
        for antecedent, consequents in self.entries().items():
            rules = [evaluations[(antecedent, consequent)] for consequent in consequents]
            weakest = min(rules, key=lambda e: e.confidence)  # min keeps the first consequent on ties
            assert weakest.decision is not None
            details: dict[str, JSON] = {
                "antecedent": antecedent,
                "rules": [e.to_json() for e in rules],
                "support_count": weakest.candidate.support_count,
                "thresholds": dict(self.thresholds),
                "variant": self.variant,
            }
            if antecedent in implied:
                details["implied"] = implied[antecedent]
            records[antecedent] = weakest.decision.provenance(
                Support(n, n), confidence=weakest.confidence, details=details
            )
        return records

    @property
    def provenance(self) -> Mapping[str, ProvenanceRecord]:
        """The Provenance_Record of each emitted entry, by antecedent."""
        return self._records

    def constraint_provenance(self, node: SchemaNode) -> dict[ConstraintId, ProvenanceRecord]:
        """The entries' records keyed by ``(node.nid, "dependentRequired", antecedent)``.

        ``node`` is the Variant's object schema, whose ``dependentRequired``
        must hold every emitted entry with at least its emitted consequents
        (Field_Group consequents may share an entry). Raises ``ValueError``
        otherwise.
        """
        if not isinstance(node, SchemaNode):
            raise TypeError(f"node must be a SchemaNode, got {type(node).__name__}")
        listed = node.kw.get(DEPENDENT_REQUIRED, {}) if node.boolean is None else {}
        for antecedent, consequents in self.entries().items():
            entry = listed.get(antecedent) if isinstance(listed, dict) else None
            if not isinstance(entry, list) or any(name not in entry for name in consequents):
                raise ValueError(
                    f"the node's 'dependentRequired' must list {consequents} under {antecedent!r} "
                    f"for {render_path(self.path)}"
                )
        return {ConstraintId(node.nid, DEPENDENT_REQUIRED, a): record for a, record in self._records.items()}

    def rejected(self) -> tuple[RejectedCandidate, ...]:
        """The report entries of the discarded (9.6) and below-Confidence (9.8) candidates."""
        path = render_path(self.path)
        out: list[RejectedCandidate] = []
        for candidate in self.discarded:
            details: dict[str, JSON] = {
                "support_count": candidate.support_count,
                "threshold": MIN_SUPPORT,
                "threshold_value": self.thresholds[MIN_SUPPORT],
                "variant": self.variant,
            }
            out.append(RejectedCandidate(REJECTED_CATEGORY, path, candidate.to_json(), COOCCURRENCE, details))
        for evaluation in self.evaluations:
            if evaluation.decision is not None:
                continue
            candidate = evaluation.candidate
            details = {
                "confidence": evaluation.confidence,
                "support_count": candidate.support_count,
                "threshold": MIN_CONFIDENCE,
                "threshold_value": self.thresholds[MIN_CONFIDENCE],
                "variant": self.variant,
            }
            out.append(RejectedCandidate(REJECTED_CATEGORY, path, candidate.to_json(), COOCCURRENCE, details))
        return tuple(out)


# ---------------------------------------------------------------------------
# Mining
# ---------------------------------------------------------------------------


def _check_record(record: SchemaNode, p: Presence) -> SchemaNode:
    if not isinstance(record, SchemaNode):
        raise TypeError(f"record must be a SchemaNode or None, got {type(record).__name__}")
    if record.boolean is not None:
        raise ValueError("record is the Variant's Record node, a keyword schema")
    properties = record.kw.get("properties", {})
    missing = [name for name in p.names if name not in properties]
    if missing:
        raise ValueError(f"record must list every field of the Variant under 'properties'; missing {missing}")
    entries = record.kw.get(DEPENDENT_REQUIRED, {})
    if not isinstance(entries, dict):
        raise TypeError(f"'dependentRequired' must be an object, got {type(entries).__name__}")
    return record


def _default_record(p: Presence, given: Sequence[Rule]) -> SchemaNode:
    """The Naive_Merge subschema of the Variant's objects plus the given (Field_Group) entries."""
    node = naive_merge_node(p.objects)
    if given:
        entries: dict[str, list[str]] = {}
        for antecedent, consequent in given:
            entries.setdefault(antecedent, []).append(consequent)
        node.kw[DEPENDENT_REQUIRED] = entries
    return node


def _with_rule(record: SchemaNode, rule: Rule) -> SchemaNode:
    """``record`` plus the entry ``{a: [b]}`` merged into its ``dependentRequired`` (children are shared)."""
    antecedent, consequent = rule
    entries: dict[str, list[str]] = {
        name: list(listed) for name, listed in record.kw.get(DEPENDENT_REQUIRED, {}).items()
    }
    listed = entries.setdefault(antecedent, [])
    if consequent not in listed:
        listed.append(consequent)
    merged = {name: sorted_codepoint(entries[name]) for name in sorted_codepoint(entries)}
    return SchemaNode(kw={**record.kw, DEPENDENT_REQUIRED: merged})


class _Scoring:
    """Scores of the Variant subschema with and without one rule (see the module docstring: Scoring)."""

    __slots__ = ("_base", "_objects", "_presence", "_record", "_scorer")

    def __init__(self, record: SchemaNode, objects: View, scorer: MDLScorer) -> None:
        self._record = record
        self._objects = objects
        self._scorer = scorer
        self._base: Score | None = None
        self._presence = 0.0

    @property
    def record(self) -> SchemaNode:
        return self._record

    @property
    def objects(self) -> View:
        return self._objects

    def base(self) -> Score:
        """The score of the Variant subschema as it is (computed once)."""
        if self._base is None:
            self._base = self._scorer.score(self._record, self._objects)
            self._presence = self._scorer.presence_bits(self._record, self._objects)
        return self._base

    def with_rule(self, node: SchemaNode) -> Score:
        """The score of ``node``: the subschema plus one rule that every object satisfies."""
        base = self.base()
        presence = self._scorer.presence_bits(node, self._objects)
        data = math.fsum((base.data_bits, presence, -self._presence))
        # In exact arithmetic data >= exception bits (the rule rejects nothing); guard against rounding.
        data = max(data, base.exception_bits)
        return Score(self._scorer.model_bits(node), data, base.exception_bits, base.n_rejected, base.n_scored)


class CooccurrenceMiner:
    """Co-occurrence mining for one run (see the module docstring).

    ``engine`` resolves the Decision_Points and supplies the Technique states
    and Learned_Thresholds. ``scorer`` scores the candidates (a default
    MDL_Scorer when None). ``report`` receives the rejected candidates of
    9.6 and 9.8 (the engine records the decisions in its own report).
    """

    __slots__ = ("_engine", "_report", "_scorer")

    def __init__(
        self, engine: DecisionEngine, *, scorer: MDLScorer | None = None, report: InferenceReport | None = None
    ) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        if report is not None and not isinstance(report, InferenceReport):
            raise TypeError(f"report must be an InferenceReport or None, got {type(report).__name__}")
        self._engine = engine
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._report = report

    @classmethod
    def for_techniques(
        cls,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
    ) -> CooccurrenceMiner:
        """A miner whose engine has the Technique states ``techniques`` (omitted ids take their default)."""
        scorer = MDLScorer(validator=validator if validator is not None else Validator())
        engine = DecisionEngine(techniques, bundle=bundle, report=report, scorer=scorer)
        return cls(engine, scorer=scorer, report=report)

    @property
    def engine(self) -> DecisionEngine:
        return self._engine

    def in_effect(self) -> bool:
        """Whether ``rules.cooccurrence`` is enabled and the Model_Bundle holds both of its Learned_Thresholds."""
        if not self._engine.enabled(COOCCURRENCE):
            return False
        bundle = self._engine.bundle
        return bundle is not None and all(bundle.threshold(name) is not None for name in (MIN_SUPPORT, MIN_CONFIDENCE))

    def rules(
        self,
        source: View | Presence,
        *,
        record: SchemaNode | None = None,
        groups: Iterable[Collection[str]] = (),
        variant: int = 0,
    ) -> CooccurrenceRules:
        """Mine, select, and reduce the Co_Occurrence_Rules of one Variant (9.3–9.6, 9.8, 9.9).

        ``source`` is the Variant's object view or its Presence. ``record`` is
        the Variant subschema the candidates are scored against (by default
        the Naive_Merge subschema of its objects plus the ``groups`` entries).
        ``groups`` are the Variant's selected Field_Groups, whose rules are
        given (6.6), and ``variant`` the Variant's index among the Path's
        final Variants, recorded with the rejected candidates. Raises
        ``ValueError`` when the view has no non-flagged object.
        """
        p = _as_presence(source)
        if isinstance(variant, bool) or not isinstance(variant, (int, np.integer)) or variant < 0:
            raise ValueError(f"variant must be a non-negative integer, got {variant!r}")
        variant = int(variant)
        given = group_rules(groups)
        if record is not None:
            _check_record(record, p)
        dp = DecisionPoint(COOCCURRENCE_RULE, p.path)
        if not self.in_effect():  # 9.9, 20.6: report no candidates
            decision = self._engine.resolve(dp, [], p.objects)
            return CooccurrenceRules(p, variant, given=given, fallback=decision)

        candidates = rule_candidates(p)
        if not candidates:
            return CooccurrenceRules(p, variant, given=given)
        min_support = self._engine.threshold(MIN_SUPPORT)  # recorded in the report (20.5)
        assert min_support is not None
        thresholds: dict[str, int | float] = {MIN_SUPPORT: min_support}
        discarded = tuple(c for c in candidates if c.support_count < min_support)
        kept = [c for c in candidates if c.support_count >= min_support]

        evaluations: list[RuleEvaluation] = []
        if kept and self._engine.enabled(MDL_SELECTION):  # 9.4
            base = record if record is not None else _default_record(p, given)
            scoring = _Scoring(base, p.objects, self._scorer)
            evaluations = [self._by_mdl(dp, candidate, scoring) for candidate in kept]
        elif kept:  # 9.8
            min_confidence = self._engine.threshold(MIN_CONFIDENCE)
            assert min_confidence is not None
            thresholds[MIN_CONFIDENCE] = min_confidence
            cutoff = Cutoff(MIN_CONFIDENCE, value=float(min_confidence))
            evaluations = [self._by_confidence(dp, candidate, p.objects, min_confidence, cutoff) for candidate in kept]

        selected = [e.candidate.rule for e in evaluations if e.selected]
        outcome = CooccurrenceRules(
            p,
            variant,
            candidates,
            discarded,
            tuple(evaluations),
            reduce_rules(selected, given),
            given,
            thresholds,
        )
        if self._report is not None:
            for entry in outcome.rejected():
                self._report.record_rejected(entry)
        return outcome

    def _by_mdl(self, dp: DecisionPoint, candidate: RuleCandidate, scoring: _Scoring) -> RuleEvaluation:
        """9.4: the rule against the Variant subschema without it, by MDL."""
        node = _with_rule(scoring.record, candidate.rule)
        options = (
            Candidate(technique=None, outcome=NO_RULE, order_key=(0,), node=scoring.record, score=scoring.base()),
            Candidate(
                technique=COOCCURRENCE,
                outcome=candidate,
                label=candidate.label,
                order_key=(0,),
                node=node,
                score=scoring.with_rule(node),
            ),
        )
        decision = self._engine.resolve(dp, options, scoring.objects)
        selected = decision.candidate is options[1]
        confidence = decision.confidence if selected and decision.confidence is not None else candidate.confidence
        return RuleEvaluation(candidate, selected, confidence, decision)

    def _by_confidence(
        self, dp: DecisionPoint, candidate: RuleCandidate, objects: View, threshold: int | float, cutoff: Cutoff
    ) -> RuleEvaluation:
        """9.8: the rule when its Confidence reaches ``cooccur.min_confidence``."""
        confidence = candidate.confidence
        if confidence < threshold:
            return RuleEvaluation(candidate, False, confidence)
        option = Candidate(technique=COOCCURRENCE, outcome=candidate, label=candidate.label, order_key=(0,))
        decision = self._engine.resolve(dp, (option,), objects, cutoff=cutoff)
        return RuleEvaluation(candidate, decision.candidate is option, confidence, decision)
