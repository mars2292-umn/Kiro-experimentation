"""Exclusion_Rule mining (task 35.1; Requirements 11.1–11.7).

Members and candidates (11.1)
    The members at a Record Path are its Field_Groups and every field in
    no Field_Group, each present in at least one non-flagged object
    (:func:`members_of`). Two members are *exclusive* when no object holds
    fields of both; the maximal sets of pairwise exclusive members are the
    maximal cliques of that graph (:func:`maximal_cliques`, Bron–Kerbosch
    with pivoting over members in canonical order), and each clique of two
    or more members is one candidate.

Confidence (11.2)
    ``E`` is the number of objects expected to hold two or more members if
    the members' presences were independent, from their observed presence
    rates (:func:`expected_overlap`), and the Confidence is ``1 − exp(−E)``,
    which lies in [0, 1) and increases strictly with ``E``. ``E`` goes into
    the Provenance_Record of an emitted rule and into the report entry of a
    discarded candidate.

Already enforced (11.3)
    For each pair of members, a witness object is built from an observed
    object holding the first member and the second member's fields taken
    from objects that hold them (:func:`witnesses`). When no emitted branch
    accepts any witness, the candidate is already enforced, discarded, and
    recorded with the enforcing branches. When no witness can be built the
    rule is kept (redundant but sound).

Selection (11.4)
    A candidate reduces the total description length when the branch node
    with the rule scores strictly below the node without it; with
    ``decide.mdl`` enabled that is the ``exclusion_rule`` Decision_Point's
    MDL resolution. A candidate that does not reduce the length is still
    emitted when its Confidence reaches ``exclusion.min_confidence``
    (recorded as the cut-off); otherwise it is rejected and reported with
    its members and Confidence.

Emission (11.5, 11.6)
    When every object holds exactly one member and the rule's node is a
    plain object schema, the rule is a ``oneOf`` with one ``required``
    branch per member. Otherwise it is one ``not``: ``{"required": a ∪ b}``
    for two members, ``{"anyOf": [{"required": …} per pair]}`` for more;
    several not-form rules at one node share one ``not`` over all their
    pairs (one Constraint, whose record lists every candidate).
    :class:`ExclusionRules` holds the keywords and one Provenance_Record
    per emitted Constraint (the ``not`` or each ``oneOf`` branch, and the
    ``required`` entries inside them, which share the rule's record).

Disabled (11.7)
    With ``rules.exclusion`` disabled, or without a usable
    ``exclusion.min_confidence`` in the Model_Bundle, no candidate is
    reported and the Fallback_Behavior (none) is recorded.
"""

from __future__ import annotations

import math
from collections.abc import Collection, Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from itertools import combinations
from typing import Any, Final

import numpy as np

from jsinfer.config.registry import OUTCOME_NONE, REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS, ResourceReader
from jsinfer.core.determinism import codepoint_key, sorted_codepoint
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import Cutoff, ProvenanceRecord, Support
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import MDL_SELECTION, Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.deps.presence import Presence, object_rows
from jsinfer.io.validator import CompileError, Validator
from jsinfer.mdl.scorer import MDLScorer

__all__ = [
    "EXCLUSION",
    "EXCLUSION_RULE",
    "MIN_CONFIDENCE",
    "ExclusionCandidate",
    "ExclusionMiner",
    "ExclusionRules",
    "Member",
    "confidence_of",
    "expected_overlap",
    "exclusion_candidates",
    "maximal_cliques",
    "members_of",
    "witnesses",
]

EXCLUSION: Final = "rules.exclusion"
EXCLUSION_RULE: Final = "exclusion_rule"
MIN_CONFIDENCE: Final = "exclusion.min_confidence"
REJECTED_CATEGORY: Final = "rules"
ONE_OF: Final = "oneOf"
NOT: Final = "not"

Member = tuple[str, ...]  # the fields of a member, in code-point order (one field, or a Field_Group)


# ---------------------------------------------------------------------------
# Members, the graph, and the cliques (11.1)
# ---------------------------------------------------------------------------


def members_of(p: Presence, groups: Iterable[Collection[str]] = ()) -> list[Member]:
    """The members at a Path: the Field_Groups and the fields in none of them, in canonical order (11.1)."""
    grouped: set[str] = set()
    found: list[Member] = []
    for group in groups:
        member = tuple(sorted_codepoint(group))
        if len(member) >= 2 and not grouped.intersection(member) and all(name in p for name in member):
            found.append(member)
            grouped.update(member)
    for name in p.names:
        if name not in grouped and p.count(name) > 0:
            found.append((name,))
    return sorted(found, key=lambda m: tuple(codepoint_key(n) for n in m))


def member_mask(p: Presence, member: Member) -> np.ndarray:
    """Which objects hold the member (any of its fields)."""
    masks = [p.mask(name) for name in member if name in p]
    return np.logical_or.reduce(masks) if masks else np.zeros(p.n_objects, dtype=bool)


MAX_MEMBERS: Final = "exclusion.max_members"
MAX_CLIQUES: Final = "exclusion.max_cliques"


class _Enough(Exception):
    """Raised inside the clique search once the cap is reached."""


def maximal_cliques(n: int, adjacent: Mapping[int, set[int]], *, max_cliques: int | None = None) -> list[tuple[int, ...]]:
    """The maximal cliques of the graph on ``0..n-1`` (Bron–Kerbosch with pivoting), each sorted, in canonical order.

    ``max_cliques`` stops the search once that many are found (the graph's
    maximal cliques can be exponential in ``n``); the result is then the
    first ones in search order, which is deterministic.
    """
    if n == 0:
        return []
    if max_cliques is not None and (isinstance(max_cliques, bool) or not isinstance(max_cliques, int) or max_cliques < 1):
        raise ValueError(f"max_cliques must be a positive int or None, got {max_cliques!r}")
    found: list[tuple[int, ...]] = []

    def expand(r: set[int], p: set[int], x: set[int]) -> None:
        if not p and not x:
            found.append(tuple(sorted(r)))
            if max_cliques is not None and len(found) >= max_cliques:
                raise _Enough
            return
        pivot = min(sorted(p | x), key=lambda v: (-len(adjacent[v] & p), v))
        for v in sorted(p - adjacent[pivot]):
            expand(r | {v}, p & adjacent[v], x & adjacent[v])
            p = p - {v}
            x = x | {v}

    try:
        expand(set(), set(range(n)), set())
    except _Enough:
        pass
    return sorted(found)


def expected_overlap(rates: Sequence[float], n: int) -> float:
    """The number of ``n`` objects expected to hold two or more members under independent presences (11.2)."""
    if not rates:
        return 0.0
    none = math.prod(1.0 - r for r in rates)
    exactly_one = 0.0
    for i, r in enumerate(rates):
        exactly_one += r * math.prod(1.0 - s for j, s in enumerate(rates) if j != i)
    return max(0.0, n * (1.0 - none - exactly_one))


def confidence_of(expected: float) -> float:
    """``1 − exp(−E)``: in [0, 1) and strictly increasing in ``E`` (11.2)."""
    if expected < 0.0 or math.isnan(expected):
        raise ValueError(f"the expected count must be non-negative, got {expected!r}")
    return -math.expm1(-expected)


@dataclass(frozen=True)
class ExclusionCandidate:
    """One maximal set of exclusive members with its expected overlap and Confidence (11.1, 11.2)."""

    members: tuple[Member, ...]
    expected: float
    confidence: float
    exactly_one: bool  # every object holds exactly one member (the oneOf form, 11.5)
    enforced_by: tuple[int, ...] = ()  # branches that reject every witness (11.3); empty when not enforced

    @property
    def label(self) -> str:
        return "exclusion=" + "|".join("+".join(m) for m in self.members)

    @property
    def fields(self) -> frozenset[str]:
        return frozenset(name for member in self.members for name in member)

    def to_json(self) -> dict[str, JSON]:
        return {"members": [list(m) for m in self.members], "expected_overlap": self.expected, "confidence": self.confidence, "exactly_one": self.exactly_one, "enforced_by": list(self.enforced_by)}


def exclusion_candidates(
    p: Presence, groups: Iterable[Collection[str]] = (), *, max_members: int | None = None, max_cliques: int | None = None
) -> list[ExclusionCandidate]:
    """The Exclusion_Rule candidates of a Path (11.1, 11.2), in canonical order; the enforcement check is separate.

    ``max_members`` (``exclusion.max_members``) keeps the most frequent
    members (ties in canonical order) and ``max_cliques``
    (``exclusion.max_cliques``) stops the clique search; both default to
    the registered Resource_Parameters.
    """
    members = members_of(p, groups)
    masks = [member_mask(p, m) for m in members]
    keep = max_members if max_members is not None else int(RESOURCE_PARAMETERS[MAX_MEMBERS].default)  # type: ignore[arg-type]
    if keep < 1:
        raise ValueError(f"max_members must be a positive int, got {max_members!r}")
    if len(members) > keep:
        order = sorted(range(len(members)), key=lambda i: (-int(masks[i].sum()), i))[:keep]
        chosen = sorted(order)
        members = [members[i] for i in chosen]
        masks = [masks[i] for i in chosen]
    cap = max_cliques if max_cliques is not None else int(RESOURCE_PARAMETERS[MAX_CLIQUES].default)  # type: ignore[arg-type]
    n = p.n_objects
    adjacent: dict[int, set[int]] = {i: set() for i in range(len(members))}
    for i, j in combinations(range(len(members)), 2):
        if not np.any(masks[i] & masks[j]):
            adjacent[i].add(j)
            adjacent[j].add(i)
    out: list[ExclusionCandidate] = []
    for clique in maximal_cliques(len(members), adjacent, max_cliques=cap):
        if len(clique) < 2:
            continue
        rates = [float(masks[i].sum()) / n for i in clique]
        expected = expected_overlap(rates, n)
        held = np.sum([masks[i] for i in clique], axis=0)
        out.append(ExclusionCandidate(tuple(members[i] for i in clique), expected, confidence_of(expected), bool(np.all(held == 1))))
    return out


# ---------------------------------------------------------------------------
# Witnesses (11.3)
# ---------------------------------------------------------------------------


def witnesses(candidate: ExclusionCandidate, objects: Sequence[Mapping[str, Any]]) -> list[dict[str, Any]]:
    """One object per ordered pair of members: an object holding the first, extended with the second's fields."""
    out: list[dict[str, Any]] = []
    for a, b in combinations(candidate.members, 2):
        for first, second in ((a, b), (b, a)):
            base = next((o for o in objects if any(name in o for name in first)), None)
            if base is None:
                continue
            extended = dict(base)
            complete = True
            for name in second:
                if name in extended:
                    continue
                donor = next((o for o in objects if name in o), None)
                if donor is None:
                    complete = False
                    break
                extended[name] = donor[name]
            if complete:
                out.append(extended)
    return out


# ---------------------------------------------------------------------------
# Rules and the miner (11.3–11.7)
# ---------------------------------------------------------------------------


def _pairs(candidate: ExclusionCandidate) -> list[list[str]]:
    """The combined ``required`` set of each pair of members (11.6)."""
    return [sorted_codepoint(set(a) | set(b)) for a, b in combinations(candidate.members, 2)]


def _pairs_of(sub: SchemaNode) -> list[list[str]]:
    """The ``required`` sets an existing ``not`` form holds."""
    if sub.boolean is not None:
        return []
    if isinstance(sub.kw.get("required"), list):
        return [list(sub.kw["required"])]
    return [list(b.kw["required"]) for b in sub.kw.get("anyOf", []) if b.boolean is None and isinstance(b.kw.get("required"), list)]


def _not_form(candidate: ExclusionCandidate, existing: SchemaNode | None = None) -> SchemaNode:
    pairs = [list(p) for p in _pairs_of(existing)] if existing is not None else []
    pairs += [pair for pair in _pairs(candidate) if pair not in pairs]
    if len(pairs) == 1:
        return SchemaNode(kw={"required": pairs[0]})
    return SchemaNode(kw={"anyOf": [SchemaNode(kw={"required": pair}) for pair in pairs]})


def _one_of_form(candidate: ExclusionCandidate) -> list[SchemaNode]:
    return [SchemaNode(kw={"required": list(member)}) for member in candidate.members]


@dataclass(frozen=True)
class ExclusionRules:
    """The Exclusion_Rules of one Record Path: the emitted candidates with their forms and records (11.5, 11.6)."""

    path: Path
    candidates: tuple[ExclusionCandidate, ...] = ()
    emitted: tuple[tuple[ExclusionCandidate, str, Decision], ...] = ()  # (candidate, form, decision)
    enforced: tuple[ExclusionCandidate, ...] = ()
    rejected_candidates: tuple[tuple[ExclusionCandidate, Decision | None], ...] = ()
    threshold: float | None = None
    fallback: Decision | None = None
    n_objects: int = 0

    @property
    def is_empty(self) -> bool:
        return not self.emitted

    def apply(self, node: SchemaNode, *, guard: bool = False) -> dict[ConstraintId, ProvenanceRecord]:
        """Add the emitted rules to ``node``: one ``oneOf`` at most, and one ``not`` over every not-form rule.

        With ``guard`` (a node whose instances are not all objects) the
        ``not`` subschema carries ``type: object``, so a non-object instance,
        which ``required`` would accept vacuously, is not rejected.
        """
        records: dict[ConstraintId, ProvenanceRecord] = {}
        nots: list[tuple[ExclusionCandidate, Decision]] = []
        for candidate, form, decision in self.emitted:
            if form == ONE_OF and ONE_OF not in node.kw and not guard:
                branches = _one_of_form(candidate)
                node.kw[ONE_OF] = branches
                record = self._record(candidate, decision, ONE_OF)
                for i, branch in enumerate(branches):
                    records[ConstraintId(node.nid, ONE_OF, i)] = record
                    for name in branch.kw["required"]:
                        records[ConstraintId(branch.nid, "required", name)] = record
            else:
                nots.append((candidate, decision))
        if not nots:
            return records
        # Every not-form rule of the Path is one ``not`` over the pairs of all its candidates (one Constraint).
        pairs: list[list[str]] = [list(p) for p in _pairs_of(node.kw[NOT])] if NOT in node.kw else []
        for candidate, _ in nots:
            for pair in _pairs(candidate):
                if pair not in pairs:
                    pairs.append(pair)
        sub = SchemaNode(kw={"required": pairs[0]}) if len(pairs) == 1 else SchemaNode(kw={"anyOf": [SchemaNode(kw={"required": pair}) for pair in pairs]})
        if guard:
            sub = SchemaNode(kw={"type": ["object"], **sub.kw})
        node.kw[NOT] = sub
        candidate, decision = nots[0]
        record = self._record(candidate, decision, NOT)
        if len(nots) > 1:
            details = {**record.details, "candidates": [c.to_json() for c, _ in nots]}
            record = decision.provenance(record.support, confidence=min(c.confidence for c, _ in nots), details=details)
        records[ConstraintId(node.nid, NOT)] = record
        records.update(self._inner(sub, record))
        return records

    @staticmethod
    def _inner(sub: SchemaNode, record: ProvenanceRecord) -> dict[ConstraintId, ProvenanceRecord]:
        """The ``required`` entries inside a ``not`` form share the rule's record (their Support is settled as (0, 0))."""
        out: dict[ConstraintId, ProvenanceRecord] = {}
        if "type" in sub.kw:
            out[ConstraintId(sub.nid, "type")] = record
        if "required" in sub.kw:
            for name in sub.kw["required"]:
                out[ConstraintId(sub.nid, "required", name)] = record
        for i, branch in enumerate(sub.kw.get("anyOf", [])):
            out[ConstraintId(sub.nid, "anyOf", i)] = record
            for name in branch.kw["required"]:
                out[ConstraintId(branch.nid, "required", name)] = record
        return out

    def _record(self, candidate: ExclusionCandidate, decision: Decision, form: str) -> ProvenanceRecord:
        n = self.n_objects
        details: dict[str, JSON] = {**candidate.to_json(), "form": form, "threshold": self.threshold}
        return decision.provenance(Support(n, n), confidence=candidate.confidence, details=details)

    def rejected(self) -> tuple[RejectedCandidate, ...]:
        """Report entries: candidates already enforced (11.3) and candidates rejected by selection (11.4)."""
        where = render_path(self.path)
        out: list[RejectedCandidate] = []
        for candidate in self.enforced:
            out.append(RejectedCandidate(REJECTED_CATEGORY, where, {"exclusion": [list(m) for m in candidate.members]}, EXCLUSION, {**candidate.to_json(), "reason": "already enforced", "branches": list(candidate.enforced_by)}))
        for candidate, decision in self.rejected_candidates:
            details: dict[str, JSON] = {**candidate.to_json(), "reason": "neither shorter nor confident", "threshold": self.threshold}
            if decision is not None:
                details["method"] = decision.method
            out.append(RejectedCandidate(REJECTED_CATEGORY, where, {"exclusion": [list(m) for m in candidate.members]}, EXCLUSION, details))
        return tuple(out)


class ExclusionMiner:
    """Mines, filters, selects, and forms the Exclusion_Rules of a Record Path (see the module docstring)."""

    __slots__ = ("_engine", "_report", "_resources", "_scorer", "_validator")

    def __init__(
        self,
        engine: DecisionEngine,
        *,
        scorer: MDLScorer | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
    ) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        self._engine = engine
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._report = report
        self._validator = validator if validator is not None else Validator()
        self._resources = ResourceReader.of(resources)

    @property
    def engine(self) -> DecisionEngine:
        return self._engine

    def in_effect(self) -> bool:
        """Whether ``rules.exclusion`` runs and the Model_Bundle holds ``exclusion.min_confidence`` (11.7)."""
        if not (REGISTRY.technique(EXCLUSION).implemented and self._engine.enabled(EXCLUSION)):
            return False
        bundle = self._engine.bundle
        return bundle is not None and bundle.threshold(MIN_CONFIDENCE) is not None

    def mine(
        self,
        objects: View,
        *,
        target: SchemaNode,
        branches: Sequence[SchemaNode] = (),
        groups: Iterable[Collection[str]] = (),
        documents: Sequence[Mapping[str, Any]] | None = None,
    ) -> ExclusionRules:
        """The Exclusion_Rules of the objects of ``objects`` (11.1–11.7).

        ``target`` is the node the rules are added to (the object schema,
        or the combinator node of a split Path) and is also scored with and
        without each rule; ``branches`` are the emitted branches that may
        already enforce a candidate (``target`` itself when none);
        ``groups`` the Path's Field_Groups; ``documents`` the objects as
        JSON (rebuilt from the view when None).
        """
        objects = object_rows(objects)
        dp = DecisionPoint(EXCLUSION_RULE, objects.path)
        if not self.in_effect():
            if self._engine.enabled(EXCLUSION) and not REGISTRY.technique(EXCLUSION).implemented:
                return ExclusionRules(objects.path)
            decision = self._engine.resolve(dp, [], objects)  # 11.7: no candidates, the Fallback_Behavior recorded
            return ExclusionRules(objects.path, fallback=decision)
        if objects.is_empty:
            return ExclusionRules(objects.path)
        p = Presence.of(objects)
        candidates = exclusion_candidates(
            p, groups, max_members=int(self._resources.read(MAX_MEMBERS)), max_cliques=int(self._resources.read(MAX_CLIQUES))
        )
        if not candidates:
            return ExclusionRules(objects.path, n_objects=p.n_objects)
        threshold = self._engine.threshold(MIN_CONFIDENCE)
        assert threshold is not None
        if documents is None:
            from jsinfer.mdl.scorer import _rebuild

            documents = [d for d in _rebuild(objects, np.arange(len(objects))) if isinstance(d, dict)]
        checked = list(branches) if branches else [target]
        compiled = []
        for branch in checked:
            try:
                compiled.append(self._validator.compile(branch.to_json()))
            except CompileError:
                compiled.append(None)
        enforced: list[ExclusionCandidate] = []
        emitted: list[tuple[ExclusionCandidate, str, Decision]] = []
        rejected: list[tuple[ExclusionCandidate, Decision | None]] = []
        use_mdl = self._engine.enabled(MDL_SELECTION)
        base_score = self._scorer.score(target, objects) if use_mdl else None
        for candidate in candidates:
            found = witnesses(candidate, documents)
            if found:
                accepting = [i for i, c in enumerate(compiled) if c is not None and any(self._validator.is_valid(c, w) for w in found)]
                if not accepting:  # 11.3: every branch rejects every witness
                    enforced.append(ExclusionCandidate(candidate.members, candidate.expected, candidate.confidence, candidate.exactly_one, tuple(range(len(checked)))))
                    continue
            form = ONE_OF if candidate.exactly_one and _plain_object(target) and ONE_OF not in target.kw else NOT
            trial = _with_rule(target, candidate, form)
            decision: Decision | None = None
            if use_mdl and base_score is not None:
                options = (
                    Candidate(technique=None, outcome=OUTCOME_NONE, order_key=(0,), node=target, score=base_score),
                    Candidate(technique=EXCLUSION, outcome=candidate.label, label=candidate.label, order_key=(0,), node=trial, score=self._scorer.score(trial, objects)),
                )
                decision = self._engine.resolve(dp, options, objects)
                if decision.candidate is options[1]:
                    emitted.append((candidate, form, decision))
                    continue
            if candidate.confidence >= threshold:  # 11.4: the Confidence threshold
                option = Candidate(technique=EXCLUSION, outcome=candidate.label, label=candidate.label, order_key=(0,), node=trial)
                decision = self._engine.resolve(dp, (option,), objects, cutoff=Cutoff(MIN_CONFIDENCE, value=float(threshold)))
                if decision.candidate is option:
                    emitted.append((candidate, form, decision))
                    continue
            rejected.append((candidate, decision))
        outcome = ExclusionRules(objects.path, tuple(candidates), tuple(emitted), tuple(enforced), tuple(rejected), float(threshold), None, p.n_objects)
        if self._report is not None:
            for entry in outcome.rejected():
                self._report.record_rejected(entry)
        return outcome


def _plain_object(node: SchemaNode) -> bool:
    return node.boolean is None and "type" in node.kw and not any(k in node.kw for k in ("anyOf", "oneOf"))


def _with_rule(target: SchemaNode, candidate: ExclusionCandidate, form: str) -> SchemaNode:
    """``target`` plus the candidate's rule (children shared), for scoring."""
    kw = dict(target.kw)
    if form == ONE_OF:
        kw[ONE_OF] = _one_of_form(candidate)
    else:
        kw[NOT] = _not_form(candidate, kw.get(NOT))
    return SchemaNode(kw=kw)
