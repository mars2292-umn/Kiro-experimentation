"""Field_Groups by FP-growth with Krimp-style selection (task 33.1; Requirements 6.2, 6.3, 6.6–6.9). BMF is task 47.1.

Transactions (6.3)
    :func:`transactions` lists the distinct key sets of a Variant's
    non-flagged objects restricted to the optional fields (every field that
    is not a Base_Field, 6.1), with the number of objects holding each.
    Objects whose optional key set is empty make no transaction.

FP-growth (6.3)
    :func:`frequent_itemsets` is an in-house FP-growth over weighted
    transactions: an FP-tree with a header table, mined by recursive
    conditional pattern bases, returning every itemset whose support (the
    number of objects holding all its fields) is at least ``min_support``,
    in code-point order of the itemset. Property 17 compares it with the
    brute-force enumeration.

Krimp-style code table (6.3)
    :func:`code_table` accepts a frequent itemset into the code table when
    the total description length ``L(CT) + L(D | CT)`` drops. Candidates
    are visited in Krimp's standard candidate order (support descending,
    size descending, then code-point order), each transaction is covered
    greedily in the standard cover order (size descending, support
    descending, then code-point order), codes are Shannon codes of the
    usage counts, and the code table's own cost codes each itemset's
    elements with the singleton table (the "no pruning" Krimp variant). The
    accepted itemsets of two or more fields are the Technique's candidate
    Field_Groups.

Filters and selection (6.2, 6.7, 6.8)
    :class:`FieldGroupMiner` applies the all-or-none filter (a candidate
    with an object that holds some but not all of its fields is excluded
    and reported with the Path, the fields, the Technique, and the number of
    violating objects, 6.7), then the disjointness filter, which keeps
    candidates greedily by their description-length gain (the drop in the
    Variant subschema's score when the candidate's ``dependentRequired``
    clique is added) and drops any that shares a field with a kept one
    (6.8). The ``field_groups`` Decision_Point then compares the empty set
    with the kept set by MDL (6.2): the Variant subschema with every kept
    clique, scored on the Variant's objects, against the subschema without
    them. Mining may run on a seeded sample of ``sample.fpgrowth`` objects
    (recorded in the Provenance_Records); the filters and the selection use
    every object. With every serving Technique disabled, the
    Fallback_Behavior selects no Field_Groups (6.9).

Output (6.6)
    :class:`FieldGroups` holds the selected groups and their
    ``dependentRequired`` cliques (one entry per member listing every other
    member), with one Provenance_Record per entry.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Collection, Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final

import numpy as np

from jsinfer.config.registry import OUTCOME_NONE, REGISTRY
from jsinfer.config.resources import ResourceReader
from jsinfer.core.determinism import codepoint_key, rng, sorted_codepoint
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import ProvenanceRecord, SampleInfo, Support
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.sampling import sample_rows
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.deps.presence import Presence
from jsinfer.mdl.scorer import MDLScorer

__all__ = [
    "FIELD_GROUPS",
    "FPGROWTH",
    "FieldGroupMiner",
    "FieldGroups",
    "clique_entries",
    "code_table",
    "frequent_itemsets",
    "transactions",
    "violations",
]

FIELD_GROUPS: Final = "field_groups"
FPGROWTH: Final = "groups.fpgrowth_slim"
BMF: Final = "groups.bmf"
REJECTED_CATEGORY: Final = "field_groups"
MIN_SUPPORT: Final = "fpgrowth.min_support"
SAMPLE: Final = "sample.fpgrowth"
DEPENDENT_REQUIRED: Final = "dependentRequired"
OUTCOME_FPGROWTH: Final = "fp-growth"

Itemset = frozenset[str]


# ---------------------------------------------------------------------------
# Transactions and FP-growth (6.3)
# ---------------------------------------------------------------------------


def _presence_matrix(p: Presence) -> np.ndarray:
    """A bool matrix ``fields × objects`` from the packed bitsets."""
    if not p.n_fields:
        return np.zeros((0, p.n_objects), dtype=bool)
    return np.stack([p.mask(name) for name in p.names])


def transactions(p: Presence, base: Collection[str] = (), objects: np.ndarray | None = None) -> dict[Itemset, int]:
    """The distinct optional key sets of the objects (those of ``objects``' positions when given) with their counts (6.3)."""
    excluded = set(base) | set(p.required)
    matrix = _presence_matrix(p)
    if objects is not None:
        matrix = matrix[:, objects]
    names = p.names
    rows = [j for j, name in enumerate(names) if name not in excluded]
    found: Counter[Itemset] = Counter()
    for column in range(matrix.shape[1]):
        key_set = frozenset(names[j] for j in rows if matrix[j, column])
        if key_set:
            found[key_set] += 1
    return {key_set: found[key_set] for key_set in sorted(found, key=lambda s: tuple(sorted_codepoint(s)))}


class _Node:
    __slots__ = ("children", "count", "item", "parent")

    def __init__(self, item: str | None, parent: _Node | None) -> None:
        self.item = item
        self.parent = parent
        self.count = 0
        self.children: dict[str, _Node] = {}


def _fp_growth(weighted: Sequence[tuple[tuple[str, ...], int]], min_support: int, suffix: Itemset, out: dict[Itemset, int]) -> None:
    support: Counter[str] = Counter()
    for items, count in weighted:
        for item in items:
            support[item] += count
    frequent = {item: s for item, s in support.items() if s >= min_support}
    if not frequent:
        return
    order = sorted(frequent, key=lambda item: (-frequent[item], codepoint_key(item)))
    rank = {item: i for i, item in enumerate(order)}
    root = _Node(None, None)
    header: dict[str, list[_Node]] = {item: [] for item in order}
    for items, count in weighted:
        node = root
        for item in sorted((i for i in items if i in rank), key=lambda i: rank[i]):
            child = node.children.get(item)
            if child is None:
                child = _Node(item, node)
                node.children[item] = child
                header[item].append(child)
            child.count += count
            node = child
    for item in reversed(order):  # least frequent first, as FP-growth does
        itemset = suffix | {item}
        out[itemset] = frequent[item]
        bases: list[tuple[tuple[str, ...], int]] = []
        for node in header[item]:
            path: list[str] = []
            parent = node.parent
            while parent is not None and parent.item is not None:
                path.append(parent.item)
                parent = parent.parent
            if path:
                bases.append((tuple(path), node.count))
        if bases:
            _fp_growth(bases, min_support, itemset, out)


def frequent_itemsets(weighted: Mapping[Itemset, int], min_support: int) -> dict[Itemset, int]:
    """Every itemset with support at least ``min_support`` (objects holding all its fields), in code-point order (6.3)."""
    if isinstance(min_support, bool) or not isinstance(min_support, int) or min_support < 1:
        raise ValueError(f"min_support must be a positive int, got {min_support!r}")
    found: dict[Itemset, int] = {}
    _fp_growth([(tuple(sorted_codepoint(items)), count) for items, count in weighted.items() if count > 0], min_support, frozenset(), found)
    return {itemset: found[itemset] for itemset in sorted(found, key=lambda s: (len(s), tuple(sorted_codepoint(s))))}


# ---------------------------------------------------------------------------
# Krimp-style code table (6.3)
# ---------------------------------------------------------------------------


def _cover_order(table: Sequence[Itemset], support: Mapping[Itemset, int]) -> list[Itemset]:
    return sorted(table, key=lambda s: (-len(s), -support[s], tuple(sorted_codepoint(s))))


def _cover(transaction: Itemset, ordered: Sequence[Itemset]) -> list[Itemset]:
    remaining = set(transaction)
    cover: list[Itemset] = []
    for itemset in ordered:
        if itemset <= remaining:
            cover.append(itemset)
            remaining -= itemset
            if not remaining:
                break
    return cover


def _total_length(weighted: Mapping[Itemset, int], table: Sequence[Itemset], support: Mapping[Itemset, int], singleton_bits: Mapping[str, float]) -> float:
    ordered = _cover_order(table, support)
    usage: Counter[Itemset] = Counter()
    covers: dict[Itemset, list[Itemset]] = {}
    for transaction, count in weighted.items():
        cover = _cover(transaction, ordered)
        covers[transaction] = cover
        for itemset in cover:
            usage[itemset] += count
    total = sum(usage.values())
    if total == 0:
        return 0.0
    code = {itemset: -math.log2(usage[itemset] / total) for itemset in usage}
    data = math.fsum(count * math.fsum(code[itemset] for itemset in covers[transaction]) for transaction, count in weighted.items())
    model = math.fsum(code[itemset] + math.fsum(singleton_bits[item] for item in itemset) for itemset in usage)
    return model + data


def code_table(weighted: Mapping[Itemset, int], candidates: Mapping[Itemset, int]) -> list[Itemset]:
    """The itemsets of two or more fields Krimp accepts (see the module docstring), in acceptance order."""
    items = sorted({item for transaction in weighted for item in transaction}, key=codepoint_key)
    if not items or not candidates:
        return []
    item_support: Counter[str] = Counter()
    for transaction, count in weighted.items():
        for item in transaction:
            item_support[item] += count
    total_items = sum(item_support.values())
    singleton_bits = {item: -math.log2(item_support[item] / total_items) for item in items}
    support: dict[Itemset, int] = {frozenset([item]): item_support[item] for item in items}
    support.update(candidates)
    table: list[Itemset] = [frozenset([item]) for item in items]
    best = _total_length(weighted, table, support, singleton_bits)
    accepted: list[Itemset] = []
    ordered = sorted((c for c in candidates if len(c) >= 2), key=lambda s: (-candidates[s], -len(s), tuple(sorted_codepoint(s))))
    for candidate in ordered:
        trial = [*table, candidate]
        length = _total_length(weighted, trial, support, singleton_bits)
        if length < best:
            best = length
            table = trial
            accepted.append(candidate)
    return accepted


# ---------------------------------------------------------------------------
# Filters, selection, and output (6.2, 6.6–6.9)
# ---------------------------------------------------------------------------


def violations(p: Presence, group: Collection[str]) -> int:
    """The number of non-flagged objects holding at least one but not all fields of ``group`` (6.7)."""
    names = sorted_codepoint(group)
    masks = [p.mask(name) if name in p else np.zeros(p.n_objects, dtype=bool) for name in names]
    any_held = np.logical_or.reduce(masks)
    all_held = np.logical_and.reduce(masks)
    return int(np.count_nonzero(any_held & ~all_held))


def clique_entries(groups: Iterable[Collection[str]]) -> dict[str, list[str]]:
    """The ``dependentRequired`` entries of the groups: each member lists every other member (6.6)."""
    entries: dict[str, list[str]] = {}
    for group in groups:
        members = sorted_codepoint(group)
        for member in members:
            entries[member] = [other for other in members if other != member]
    return {name: entries[name] for name in sorted_codepoint(entries)}


def _with_groups(record: SchemaNode, groups: Iterable[Collection[str]]) -> SchemaNode:
    """``record`` plus the cliques of ``groups`` merged into its ``dependentRequired`` (children shared)."""
    entries: dict[str, list[str]] = {name: list(listed) for name, listed in record.kw.get(DEPENDENT_REQUIRED, {}).items()}
    for antecedent, consequents in clique_entries(groups).items():
        listed = entries.setdefault(antecedent, [])
        listed.extend(c for c in consequents if c not in listed)
    merged = {name: sorted_codepoint(entries[name]) for name in sorted_codepoint(entries)}
    kw = {k: v for k, v in record.kw.items() if k != DEPENDENT_REQUIRED}
    if merged:
        kw[DEPENDENT_REQUIRED] = merged
    return SchemaNode(kw=kw)


@dataclass(frozen=True)
class GroupCandidate:
    """One candidate Field_Group with its support and the all-or-none violations (6.7)."""

    fields: tuple[str, ...]
    support: int
    technique: str
    violators: int = 0
    gain: float | None = None  # description-length drop when its clique is added (set by the disjointness filter)

    @property
    def excluded(self) -> bool:
        return self.violators > 0

    def to_json(self) -> dict[str, JSON]:
        return {"fields": list(self.fields), "support": self.support, "technique": self.technique, "violators": self.violators, "gain": self.gain}


@dataclass(frozen=True)
class FieldGroups:
    """The Field_Groups of one Variant: the selected groups, their cliques, and the records (see the module docstring)."""

    presence: Presence
    variant: int = 0
    candidates: tuple[GroupCandidate, ...] = ()
    selected: tuple[tuple[str, ...], ...] = ()
    decision: Decision | None = None
    sample: SampleInfo | None = None
    thresholds: Mapping[str, int | float] = field(default_factory=dict)
    _records: Mapping[str, ProvenanceRecord] = field(init=False, repr=False)

    def __post_init__(self) -> None:
        object.__setattr__(self, "thresholds", MappingProxyType(dict(self.thresholds)))
        object.__setattr__(self, "_records", MappingProxyType(self._build_records()))

    @property
    def path(self) -> Path:
        return self.presence.path

    @property
    def groups(self) -> tuple[frozenset[str], ...]:
        return tuple(frozenset(group) for group in self.selected)

    @property
    def is_empty(self) -> bool:
        return not self.selected

    def entries(self) -> dict[str, list[str]]:
        """The ``dependentRequired`` cliques of the selected groups (6.6)."""
        return clique_entries(self.selected)

    def keywords(self) -> dict[str, dict[str, list[str]]]:
        entries = self.entries()
        return {DEPENDENT_REQUIRED: entries} if entries else {}

    def _build_records(self) -> dict[str, ProvenanceRecord]:
        if self.decision is None or not self.selected:
            return {}
        n = self.presence.n_objects
        by_member = {member: group for group in self.selected for member in group}
        records: dict[str, ProvenanceRecord] = {}
        for member in sorted_codepoint(by_member):
            group = by_member[member]
            candidate = next((c for c in self.candidates if c.fields == group), None)
            details: dict[str, JSON] = {
                "group": list(group),
                "member": member,
                "support_count": None if candidate is None else candidate.support,
                "gain": None if candidate is None else candidate.gain,
                "thresholds": dict(self.thresholds),
                "variant": self.variant,
            }
            records[member] = self.decision.provenance(Support(n, n), sample=self.sample, details=details)
        return records

    @property
    def provenance(self) -> Mapping[str, ProvenanceRecord]:
        """The Provenance_Record of each clique entry, by member."""
        return self._records

    def constraint_provenance(self, node: SchemaNode) -> dict[ConstraintId, ProvenanceRecord]:
        """The entries' records keyed by ``(node.nid, "dependentRequired", member)``; ``node`` must list every clique entry."""
        listed = node.kw.get(DEPENDENT_REQUIRED, {}) if node.boolean is None else {}
        for member, others in self.entries().items():
            entry = listed.get(member) if isinstance(listed, dict) else None
            if not isinstance(entry, list) or any(name not in entry for name in others):
                raise ValueError(f"the node's 'dependentRequired' must list {others} under {member!r} for {render_path(self.path)}")
        return {ConstraintId(node.nid, DEPENDENT_REQUIRED, member): record for member, record in self._records.items()}

    def rejected(self) -> tuple[RejectedCandidate, ...]:
        """The report entries of the candidates the all-or-none filter excluded (6.7)."""
        return tuple(
            RejectedCandidate(
                REJECTED_CATEGORY,
                render_path(self.path),
                {"fields": list(c.fields)},
                c.technique,
                {"violators": c.violators, "support_count": c.support, "variant": self.variant},
            )
            for c in self.candidates
            if c.excluded
        )


class FieldGroupMiner:
    """Mines, filters, and selects the Field_Groups of a Variant (see the module docstring)."""

    __slots__ = ("_engine", "_reader", "_report", "_scorer", "_seed")

    def __init__(
        self,
        engine: DecisionEngine,
        *,
        scorer: MDLScorer | None = None,
        report: InferenceReport | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = 0,
    ) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        self._engine = engine
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._report = report
        self._reader = ResourceReader.of(resources)
        if isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
            raise ValueError(f"seed must be a non-negative int, got {seed!r}")
        self._seed = seed

    @classmethod
    def for_techniques(cls, techniques: Mapping[str, bool] | None = None, *, report: InferenceReport | None = None, resources: ResourceReader | Mapping[str, Any] | None = None, seed: int = 0) -> FieldGroupMiner:
        scorer = MDLScorer()
        engine = DecisionEngine(techniques, report=report, scorer=scorer)
        return cls(engine, scorer=scorer, report=report, resources=resources, seed=seed)

    @property
    def engine(self) -> DecisionEngine:
        return self._engine

    def runs(self, tid: str) -> bool:
        return REGISTRY.technique(tid).implemented and self._engine.enabled(tid)

    def in_effect(self) -> bool:
        """Whether any serving Technique runs (6.9)."""
        return any(self.runs(tid) for tid in REGISTRY.decision_point(FIELD_GROUPS).served_by)

    def mine(self, source: View | Presence, *, record: SchemaNode | None = None, variant: int = 0) -> FieldGroups:
        """The Field_Groups of one Variant (6.2, 6.3, 6.6–6.9); ``record`` is the Variant subschema to score against."""
        p = source if isinstance(source, Presence) else Presence.of(source)
        if isinstance(variant, bool) or not isinstance(variant, (int, np.integer)) or variant < 0:
            raise ValueError(f"variant must be a non-negative integer, got {variant!r}")
        variant = int(variant)
        dp = DecisionPoint(FIELD_GROUPS, p.path)
        if not self.in_effect():
            servers = REGISTRY.decision_point(FIELD_GROUPS).served_by
            if any(self._engine.enabled(tid) for tid in servers):
                return FieldGroups(p, variant)  # enabled outside a validated run, where it is not implemented: nothing offered
            decision = self._engine.resolve(dp, [], p.objects)  # 6.9: no Field_Groups, the Fallback_Behavior recorded
            return FieldGroups(p, variant, decision=decision)
        base = record if record is not None else _naive_record(p)
        candidates: list[GroupCandidate] = []
        thresholds: dict[str, int | float] = {}
        sample: SampleInfo | None = None
        if self.runs(FPGROWTH) and p.optional:  # a Variant without an optional field has no key set to mine
            min_support = int(self._reader.read(MIN_SUPPORT))
            sample_size = int(self._reader.read(SAMPLE))
            thresholds[MIN_SUPPORT] = min_support
            rows: np.ndarray | None = None
            if p.n_objects > sample_size:
                rows = np.asarray(sorted(sample_rows(list(range(p.n_objects)), sample_size, rng(FPGROWTH, render_path(p.path), seed=self._seed))))
                sample = SampleInfo(sample_size, p.n_objects)
            weighted = transactions(p, (), rows)
            frequent = frequent_itemsets(weighted, min_support) if weighted else {}
            for itemset in code_table(weighted, {s: c for s, c in frequent.items() if len(s) >= 2}):
                fields = tuple(sorted_codepoint(itemset))
                candidates.append(GroupCandidate(fields, _support(p, fields), FPGROWTH, violations(p, fields)))
        # BMF (6.4) joins in task 47.1.
        kept = self._disjoint(p, base, [c for c in candidates if not c.excluded])
        candidates = [next((k for k in kept if k.fields == c.fields), c) for c in candidates]
        selected = tuple(c.fields for c in kept)
        none = Candidate(technique=FPGROWTH, outcome=OUTCOME_NONE, order_key=(0,), node=base, label=OUTCOME_NONE)
        offered = [none]
        if selected:
            offered.append(Candidate(technique=FPGROWTH, outcome=[list(g) for g in selected], order_key=(1,), node=_with_groups(base, selected), label=OUTCOME_FPGROWTH))
        decision = self._engine.resolve(dp, offered, p.objects, mdl_by=FPGROWTH, scorer=self._scorer)
        chosen = selected if decision.label == OUTCOME_FPGROWTH else ()
        outcome = FieldGroups(p, variant, tuple(candidates), chosen, decision, sample, thresholds)
        if self._report is not None:
            for entry in outcome.rejected():
                self._report.record_rejected(entry)
        return outcome

    def _disjoint(self, p: Presence, base: SchemaNode, candidates: Sequence[GroupCandidate]) -> list[GroupCandidate]:
        """The disjointness filter (6.8): greedy by description-length gain, dropping overlaps."""
        if not candidates:
            return []
        without = self._scorer.score(base, p.objects).total
        scored = [GroupCandidate(c.fields, c.support, c.technique, c.violators, without - self._scorer.score(_with_groups(base, [c.fields]), p.objects).total) for c in candidates]
        scored.sort(key=lambda c: (-(c.gain or 0.0), c.fields))
        kept: list[GroupCandidate] = []
        taken: set[str] = set()
        for candidate in scored:
            if len(candidate.fields) < 2 or taken & set(candidate.fields):
                continue
            kept.append(candidate)
            taken.update(candidate.fields)
        return kept


def _support(p: Presence, fields: Sequence[str]) -> int:
    masks = [p.mask(name) for name in fields if name in p]
    return int(np.count_nonzero(np.logical_and.reduce(masks))) if masks else 0


def _naive_record(p: Presence) -> SchemaNode:
    from jsinfer.pipeline.naive_merge import naive_merge_node

    return naive_merge_node(p.objects)


def merge_records(group: ProvenanceRecord, rule: ProvenanceRecord) -> ProvenanceRecord:
    """One record for a ``dependentRequired`` entry that a Field_Group clique and a Co_Occurrence_Rule share (6.6, 9.4).

    The Field_Group's record stands, with the rule's Technique, Confidence,
    and details kept under ``cooccurrence``; the Confidence is the lower of
    the two, since both must hold for the merged entry.
    """
    details: dict[str, JSON] = dict(group.details)
    details["cooccurrence"] = {"technique": rule.technique, "confidence": rule.confidence, "details": dict(rule.details)}
    return ProvenanceRecord(
        group.technique,
        group.support,
        min(group.confidence, rule.confidence),
        fallback=group.fallback,
        resolution=group.resolution,
        cutoff=group.cutoff,
        sample=group.sample,
        changes=group.changes,
        details=details,
    )


__all__ += ["GroupCandidate", "merge_records"]
