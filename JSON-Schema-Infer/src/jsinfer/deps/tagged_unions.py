"""Per-field decision trees and Discriminator selection (task 34.1; Requirements 10.1–10.8).

Features (10.2)
    :func:`sibling_features` reads, for the non-flagged objects of a Record
    Path, the value of every property whose values are all JSON strings,
    integers, or booleans (:data:`DISCRIMINATOR_TYPES`) as a categorical
    feature, provided the property is present in every object (a property
    some objects lack would predict its own presence, and the candidates
    of 10.1 are the properties present everywhere). Only the
    ``trees.max_values`` most frequent values of a feature are kept; the
    others become :data:`OTHER`. Every feature is a Discriminator candidate.

Trees (10.2)
    :func:`grow_tree` is an in-house categorical tree with binary splits
    ``P == v``. The description length of a target at a node is ``CAT`` of
    its class counts; a split costs ``log2(#features) + log2(#values) + 1``
    bits and is taken greedily when the children's length plus the cost is
    below the node's, down to ``trees.max_depth``. The tree is then
    post-pruned bottom-up: a split whose subtree is no shorter than the leaf
    collapses. One tree is grown per field-presence target (an optional
    field) and one per Variant membership when the Path has two or more
    Variants; the retained splits of every tree are the Path's splits.

Value groups and candidates (10.1, 10.3)
    The value groups of a candidate ``P`` are the partition of its observed
    values induced by its retained splits: every split value is a group of
    its own and the values no split separates form one group. The
    candidate's description length is the MDL_Scorer's length of the
    objects partitioned by the groups: each group's objects under their
    Naive_Merge subschema with ``P`` fixed to the group (``const`` or
    ``enum``), plus ``UNI(#groups)`` per object for the branch choice. A
    candidate is eligible when some group's Support ratio reaches the
    ``tagged.branch_support`` Learned_Threshold; the eligible candidate of
    the lowest length wins, ties to the smallest name (10.3), through the
    ``discriminator`` Decision_Point, and the others are recorded in the
    report (category ``rules``) and in the winner's details.

Branches (10.4, 10.5, 10.8)
    :class:`Discriminator` lists the dedicated groups (ratio at or above the
    threshold) and the fallback: the objects of the groups below it. The
    Structure_Analyzer builds the branch schemas (a dedicated branch
    requires the fields common to its group's objects, which include the
    Base_Fields; the fallback requires only the Base_Fields) and
    constrains ``P`` with ``const``/``enum`` in dedicated branches and, in
    the fallback, with ``enum`` of the below-threshold values, or, when the
    Value_Inferrer does not judge ``P``'s Path a Closed_Enum, with
    ``not: {enum: <dedicated values>}`` (10.8). Every object must then
    validate against exactly one branch (10.7); the analyzer checks it.

Disabled (10.6)
    With ``rules.decision_trees`` disabled, or no usable threshold in the
    Model_Bundle, :meth:`DiscriminatorMiner.mine` reports no candidate and
    the Fallback_Behavior (none) is recorded.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Any, Final

import numpy as np

from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import ResourceReader
from jsinfer.core.determinism import codepoint_key, rng
from jsinfer.core.jsonvalue import JSON, JType, canonical_key
from jsinfer.core.paths import Path, render_path
from jsinfer.core.provenance import SampleInfo
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.sampling import sample_rows
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.deps.presence import Presence, object_rows
from jsinfer.mdl.codes import CAT, UNI
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node

__all__ = [
    "ABSENT",
    "BRANCH_SUPPORT",
    "DISCRIMINATOR",
    "DISCRIMINATOR_TYPES",
    "OTHER",
    "TREES",
    "Discriminator",
    "DiscriminatorCandidate",
    "DiscriminatorMiner",
    "Features",
    "Split",
    "Tree",
    "ValueGroup",
    "grow_tree",
    "sibling_features",
    "value_groups",
]

TREES: Final = "rules.decision_trees"
DISCRIMINATOR: Final = "discriminator"
BRANCH_SUPPORT: Final = "tagged.branch_support"
MAX_DEPTH: Final = "trees.max_depth"
MAX_VALUES: Final = "trees.max_values"
SAMPLE: Final = "sample.trees"
REJECTED_CATEGORY: Final = "rules"
DISCRIMINATOR_TYPES: Final = frozenset({JType.STRING, JType.INTEGER, JType.BOOLEAN})


class _Marker:
    __slots__ = ("name",)

    def __init__(self, name: str) -> None:
        self.name = name

    def __repr__(self) -> str:
        return self.name


ABSENT: Final = _Marker("ABSENT")  # the object lacks the property
OTHER: Final = _Marker("OTHER")  # a value outside the kept most frequent ones
Key = Any  # a canonical value key, ABSENT, or OTHER


# ---------------------------------------------------------------------------
# Features (10.1, 10.2)
# ---------------------------------------------------------------------------


def _parent_positions(child: View, parent: View) -> np.ndarray:
    """For each row of ``child`` (one step below ``parent``), the position of its parent among ``parent``'s rows."""
    index: dict[Any, tuple[int, np.ndarray]] = {}
    offset = 0
    for part in parent.parts:
        index[part.column] = (offset, part.rows)
        offset += len(part.rows)
    found: list[np.ndarray] = []
    columns = child.store.columns
    for part in child.parts:
        start, rows = index[part.column[:-1]]
        parents = columns[part.column].parent[part.rows]
        found.append(start + np.searchsorted(rows, parents))
    return np.concatenate(found) if found else np.zeros(0, dtype=np.int64)


def _scalar_rows(child: View) -> tuple[np.ndarray, list[Any]]:
    """``(jtypes, values)`` of the rows of ``child`` in view order (container rows get None)."""
    jtypes: list[np.ndarray] = []
    values: list[Any] = []
    table = child.store.values
    for part in child.parts:
        column = child.store.columns[part.column]
        jtypes.append(column.jtype[part.rows])
        ids = column.value[part.rows]
        values.extend(table[int(v)] if v >= 0 else None for v in ids.tolist())
    return (np.concatenate(jtypes) if jtypes else np.zeros(0, dtype=np.int64)), values


@dataclass(frozen=True)
class Features:
    """The categorical features of a Path's objects (see the module docstring).

    ``names`` are the usable properties in code-point order; ``columns[j]``
    holds object ``i``'s key for feature ``j`` at ``columns[j][i]``;
    ``everywhere`` marks the features present in every object (the
    Discriminator candidates, 10.1); ``originals[j]`` maps a kept key back
    to its JSON value.
    """

    n_objects: int
    names: tuple[str, ...]
    columns: tuple[tuple[Key, ...], ...]
    everywhere: tuple[bool, ...]
    originals: tuple[Mapping[Key, JSON], ...]

    def index(self, name: str) -> int:
        return self.names.index(name)

    def candidates(self) -> tuple[str, ...]:
        return tuple(name for name, always in zip(self.names, self.everywhere) if always)


def sibling_features(objects: View, max_values: int = 32) -> Features:
    """The features of the non-flagged objects of ``objects`` (10.1, 10.2); see the module docstring."""
    objects = object_rows(objects)
    n = len(objects)
    p = Presence.of(objects) if n else None
    names: list[str] = []
    columns: list[tuple[Key, ...]] = []
    everywhere: list[bool] = []
    originals: list[Mapping[Key, JSON]] = []
    for name in (p.names if p is not None else ()):
        child = objects.child_key(name)
        if child.is_empty:
            continue
        jtypes, values = _scalar_rows(child)
        if not all(int(j) in DISCRIMINATOR_TYPES for j in jtypes.tolist()):
            continue
        positions = _parent_positions(child, objects)
        column: list[Key] = [ABSENT] * n
        back: dict[Key, JSON] = {}
        for position, value in zip(positions.tolist(), values):
            key = canonical_key(value)
            column[position] = key
            back.setdefault(key, value)
        counts = Counter(k for k in column if k is not ABSENT)
        kept = {k for k, _ in sorted(counts.items(), key=lambda item: (-item[1], codepoint_key(str(item[0]))))[:max_values]}
        if len(kept) < len(counts):
            column = [k if k is ABSENT or k in kept else OTHER for k in column]
            back = {k: v for k, v in back.items() if k in kept}
        if ABSENT in column:
            continue  # only properties present in every object serve as features (and as candidates, 10.1)
        names.append(name)
        columns.append(tuple(column))
        everywhere.append(True)
        originals.append(back)
    return Features(n, tuple(names), tuple(columns), tuple(everywhere), tuple(originals))


# ---------------------------------------------------------------------------
# Trees (10.2)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Split:
    """A retained split ``feature == value``."""

    feature: str
    value: Key

    def to_json(self) -> dict[str, JSON]:
        return {"feature": self.feature, "value": repr(self.value) if isinstance(self.value, _Marker) else self.value}


@dataclass(frozen=True)
class Tree:
    """A grown and pruned tree: its target, the retained splits (preorder), and its length against the leaf's."""

    target: str
    splits: tuple[Split, ...]
    leaf_bits: float
    tree_bits: float

    def to_json(self) -> dict[str, JSON]:
        return {"target": self.target, "splits": [s.to_json() for s in self.splits], "leaf_bits": self.leaf_bits, "tree_bits": self.tree_bits}


def _class_bits(labels: Sequence[int], rows: Sequence[int], n_classes: int) -> float:
    counts = [0] * n_classes
    for i in rows:
        counts[labels[i]] += 1
    return CAT(counts)


def _split_cost(features: Features, feature: int) -> float:
    values = len(set(features.columns[feature]))
    return math.log2(len(features.names)) + math.log2(max(values, 1)) + 1.0


class _Node:
    __slots__ = ("left", "right", "rows", "split")

    def __init__(self, rows: list[int]) -> None:
        self.rows = rows
        self.split: tuple[int, Key] | None = None
        self.left: _Node | None = None
        self.right: _Node | None = None


def _grow(node: _Node, features: Features, labels: Sequence[int], n_classes: int, depth: int, max_depth: int) -> None:
    if depth >= max_depth or len(node.rows) < 2:
        return
    here = _class_bits(labels, node.rows, n_classes)
    best: tuple[float, int, Key, list[int], list[int]] | None = None
    for j in range(len(features.names)):
        column = features.columns[j]
        cost = _split_cost(features, j)
        for value in sorted({column[i] for i in node.rows}, key=lambda k: codepoint_key(str(k))):
            inside = [i for i in node.rows if column[i] == value]
            outside = [i for i in node.rows if column[i] != value]
            if not inside or not outside:
                continue
            length = cost + _class_bits(labels, inside, n_classes) + _class_bits(labels, outside, n_classes)
            if length < here and (best is None or length < best[0]):
                best = (length, j, value, inside, outside)
    if best is None:
        return
    _, j, value, inside, outside = best
    node.split = (j, value)
    node.left = _Node(inside)
    node.right = _Node(outside)
    _grow(node.left, features, labels, n_classes, depth + 1, max_depth)
    _grow(node.right, features, labels, n_classes, depth + 1, max_depth)


def _prune(node: _Node, features: Features, labels: Sequence[int], n_classes: int) -> float:
    """Collapse splits whose subtree is no shorter than the leaf; returns the node's length."""
    leaf = _class_bits(labels, node.rows, n_classes)
    if node.split is None:
        return leaf
    assert node.left is not None and node.right is not None
    subtree = _split_cost(features, node.split[0]) + _prune(node.left, features, labels, n_classes) + _prune(node.right, features, labels, n_classes)
    if subtree >= leaf:
        node.split = node.left = node.right = None
        return leaf
    return subtree


def _splits(node: _Node, features: Features, out: list[Split]) -> None:
    if node.split is None:
        return
    j, value = node.split
    out.append(Split(features.names[j], value))
    assert node.left is not None and node.right is not None
    _splits(node.left, features, out)
    _splits(node.right, features, out)


def grow_tree(features: Features, labels: Sequence[int], target: str, *, max_depth: int = 4) -> Tree:
    """Grow and prune a tree for ``labels`` (class ids per object) over ``features`` (10.2); see the module docstring."""
    if len(labels) != features.n_objects:
        raise ValueError("one label per object")
    n_classes = (max(labels) + 1) if labels else 0
    root = _Node(list(range(features.n_objects)))
    leaf = _class_bits(labels, root.rows, n_classes) if labels else 0.0
    if features.names and labels:
        _grow(root, features, labels, n_classes, 0, max_depth)
        tree_bits = _prune(root, features, labels, n_classes)
    else:
        tree_bits = leaf
    found: list[Split] = []
    _splits(root, features, found)
    return Tree(target, tuple(found), leaf, tree_bits)


# ---------------------------------------------------------------------------
# Value groups and candidates (10.1, 10.3)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ValueGroup:
    """A maximal set of observed values of a Discriminator that no retained split separates (10.4)."""

    values: tuple[JSON, ...]  # in code-point order of their canonical keys
    keys: frozenset[Key]
    support: int
    ratio: float

    @property
    def single(self) -> bool:
        return len(self.values) == 1

    def constraint(self) -> dict[str, JSON]:
        """``{"const": v}`` for one value, else ``{"enum": [...]}``."""
        return {"const": self.values[0]} if self.single else {"enum": list(self.values)}

    def to_json(self) -> dict[str, JSON]:
        return {"values": list(self.values), "support": self.support, "ratio": self.ratio}


def value_groups(features: Features, name: str, splits: Sequence[Split]) -> list[ValueGroup]:
    """The value groups of candidate ``name`` under the retained ``splits`` (10.4); singletons first, then the rest."""
    j = features.index(name)
    column = features.columns[j]
    originals = features.originals[j]
    counts = Counter(column)
    n = features.n_objects
    separated = {s.value for s in splits if s.feature == name and s.value in counts}
    groups: list[ValueGroup] = []
    for key in sorted(separated, key=lambda k: codepoint_key(str(k))):
        if key is OTHER:
            continue
        groups.append(ValueGroup((originals[key],), frozenset([key]), counts[key], counts[key] / n))
    rest = [k for k in counts if k not in separated or k is OTHER]
    if rest:
        keys = frozenset(rest)
        support = sum(counts[k] for k in rest)
        values = tuple(originals[k] for k in sorted((k for k in rest if k is not OTHER), key=lambda k: codepoint_key(str(k))))
        groups.append(ValueGroup(values, keys, support, support / n))
    return groups


@dataclass(frozen=True)
class DiscriminatorCandidate:
    """A Discriminator candidate (10.1): its value groups, description length, and eligibility (10.3)."""

    name: str
    groups: tuple[ValueGroup, ...]
    score: Score
    eligible: bool

    @property
    def bits(self) -> float:
        return self.score.total

    def to_json(self) -> dict[str, JSON]:
        return {"name": self.name, "groups": [g.to_json() for g in self.groups], "description_length": self.bits, "eligible": self.eligible}


def _fix(node: SchemaNode, name: str, constraint: Mapping[str, JSON]) -> SchemaNode:
    """``node`` with property ``name`` constrained by ``constraint`` (its enum/const replaced)."""
    properties = dict(node.kw.get("properties", {}))
    child = properties.get(name)
    kw = {k: v for k, v in (child.kw.items() if isinstance(child, SchemaNode) and child.boolean is None else ()) if k not in ("enum", "const")}
    properties[name] = SchemaNode(kw={**kw, **constraint})
    return SchemaNode(kw={**node.kw, "properties": properties})


@dataclass(frozen=True)
class Discriminator:
    """The selected Discriminator of a Path (10.3–10.5, 10.8); see the module docstring."""

    path: Path
    name: str
    candidate: DiscriminatorCandidate
    dedicated: tuple[ValueGroup, ...]
    below: tuple[ValueGroup, ...]
    masks: tuple[np.ndarray, ...]  # one bool mask over the objects per dedicated group
    fallback_mask: np.ndarray
    threshold: float
    decision: Decision
    candidates: tuple[DiscriminatorCandidate, ...]
    trees: tuple[Tree, ...]
    sample: SampleInfo | None = None

    @property
    def dedicated_values(self) -> list[JSON]:
        return [v for g in self.dedicated for v in g.values]

    @property
    def fallback_values(self) -> list[JSON]:
        return [v for g in self.below for v in g.values]

    def details(self) -> dict[str, JSON]:
        return {
            "discriminator": self.name,
            "value_groups": [g.to_json() for g in self.dedicated],
            "below_threshold": [g.to_json() for g in self.below],
            "threshold": self.threshold,
            "description_length": self.candidate.bits,
            "other_candidates": [c.to_json() for c in self.candidates if c.name != self.name],
            "splits": [s.to_json() for t in self.trees for s in t.splits],
        }


class DiscriminatorMiner:
    """Grows the trees of a Record Path and selects its Discriminator (see the module docstring)."""

    __slots__ = ("_engine", "_reader", "_report", "_scorer", "_seed")

    def __init__(self, engine: DecisionEngine, *, scorer: MDLScorer | None = None, report: InferenceReport | None = None, resources: ResourceReader | Mapping[str, Any] | None = None, seed: int = 0) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        self._engine = engine
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._report = report
        self._reader = ResourceReader.of(resources)
        if isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
            raise ValueError(f"seed must be a non-negative int, got {seed!r}")
        self._seed = seed

    @property
    def engine(self) -> DecisionEngine:
        return self._engine

    def in_effect(self) -> bool:
        """Whether ``rules.decision_trees`` runs and the Model_Bundle holds ``tagged.branch_support`` (10.6)."""
        if not (REGISTRY.technique(TREES).implemented and self._engine.enabled(TREES)):
            return False
        bundle = self._engine.bundle
        return bundle is not None and bundle.threshold(BRANCH_SUPPORT) is not None

    def trees(self, features: Features, p: Presence, variants: Sequence[View], objects: View, *, max_depth: int) -> list[Tree]:
        """One tree per optional field and one for Variant membership (10.2)."""
        found: list[Tree] = []
        for name in p.optional:
            labels = [1 if held else 0 for held in p.mask(name).tolist()]
            found.append(grow_tree(features, labels, f"presence:{name}", max_depth=max_depth))
        if len(variants) >= 2:
            labels = [0] * features.n_objects
            for index, variant in enumerate(variants):
                for position in _parent_positions_of(variant, objects).tolist():
                    labels[position] = index
            found.append(grow_tree(features, labels, "variant", max_depth=max_depth))
        return found

    def mine(self, objects: View, variants: Sequence[View] = ()) -> Discriminator | None:
        """The Path's Discriminator, or None (10.1–10.3, 10.6); ``variants`` are the final Variant views."""
        objects = object_rows(objects)
        dp = DecisionPoint(DISCRIMINATOR, objects.path)
        if not self.in_effect():
            if self._engine.enabled(TREES) and not REGISTRY.technique(TREES).implemented:
                return None
            self._engine.resolve(dp, [], objects)  # 10.6: no candidates, the Fallback_Behavior recorded
            return None
        n = len(objects)
        if n < 2:
            return None
        p = Presence.of(objects)
        max_depth = int(self._reader.read(MAX_DEPTH))
        max_values = int(self._reader.read(MAX_VALUES))
        sample_size = int(self._reader.read(SAMPLE))
        threshold = self._engine.threshold(BRANCH_SUPPORT)
        assert threshold is not None
        features = sibling_features(objects, max_values)
        if not features.candidates():
            return None
        fitted = features
        sample: SampleInfo | None = None
        if n > sample_size:
            picked = sorted(sample_rows(list(range(n)), sample_size, rng(TREES, render_path(objects.path), seed=self._seed)))
            fitted = _restrict(features, picked)
            sample = SampleInfo(sample_size, n)
            p_fit = Presence.of(objects.subset(np.isin(np.arange(n), picked)))
            trees = self.trees(fitted, p_fit, (), objects, max_depth=max_depth)  # the Variant target needs every object
        else:
            trees = self.trees(features, p, variants, objects, max_depth=max_depth)
        splits = [s for t in trees for s in t.splits]
        candidates: list[DiscriminatorCandidate] = []
        for name in features.candidates():
            if not any(s.feature == name for s in splits):
                continue  # 10.1: a candidate is a property some retained split tests
            groups = value_groups(features, name, splits)
            if len(groups) < 2:
                continue
            score = self._partition_score(objects, features, name, groups)
            eligible = any(g.ratio >= threshold for g in groups)
            candidates.append(DiscriminatorCandidate(name, tuple(groups), score, eligible))
        candidates.sort(key=lambda c: codepoint_key(c.name))
        eligible = [c for c in candidates if c.eligible]
        winner: DiscriminatorCandidate | None = None
        decision: Decision | None = None
        if eligible:
            offered = [Candidate(technique=TREES, outcome=c.name, order_key=(c.name,), score=c.score, label=c.name) for c in eligible]
            decision = self._engine.resolve(dp, offered, objects, mdl_by=TREES, scorer=self._scorer)
            winner = next((c for c in eligible if c.name == decision.label), None)
        if self._report is not None:  # 10.1, 10.3: every candidate with its length; the non-selected ones are rejected
            for c in candidates:
                if winner is not None and c.name == winner.name:
                    continue
                self._report.record_rejected(RejectedCandidate(REJECTED_CATEGORY, render_path(objects.path), {"discriminator": c.name}, TREES, {"description_length": c.bits, "eligible": c.eligible, "groups": [g.to_json() for g in c.groups]}))
        if winner is None or decision is None:
            return None
        column = features.columns[features.index(winner.name)]
        dedicated = tuple(g for g in winner.groups if g.ratio >= threshold)
        below = tuple(g for g in winner.groups if g.ratio < threshold)
        masks = tuple(np.array([k in g.keys for k in column], dtype=bool) for g in dedicated)
        fallback = np.array([any(k in g.keys for g in below) for k in column], dtype=bool)
        return Discriminator(objects.path, winner.name, winner, dedicated, below, masks, fallback, float(threshold), decision, tuple(candidates), tuple(trees), sample)

    def _partition_score(self, objects: View, features: Features, name: str, groups: Sequence[ValueGroup]) -> Score:
        """The description length of the objects partitioned by ``groups`` (10.1), as one Score."""
        column = features.columns[features.index(name)]
        model = data = exceptions = 0.0
        rejected = 0
        choice = UNI(len(groups))
        for group in groups:
            mask = np.array([k in group.keys for k in column], dtype=bool)
            view = objects.subset(mask)
            node = _fix(naive_merge_node(view), name, group.constraint())
            score = self._scorer.score(node, view)
            model += score.model_bits
            data += score.data_bits + choice * len(view)
            exceptions += score.exception_bits
            rejected += score.n_rejected
        return Score(model, data, exceptions, rejected, len(objects))


def _parent_positions_of(variant: View, objects: View) -> np.ndarray:
    """The positions among ``objects``' rows of the rows of ``variant`` (a subset view of the same Path)."""
    index: dict[Any, tuple[int, np.ndarray]] = {}
    offset = 0
    for part in objects.parts:
        index[part.column] = (offset, part.rows)
        offset += len(part.rows)
    found: list[np.ndarray] = []
    for part in variant.parts:
        start, rows = index[part.column]
        found.append(start + np.searchsorted(rows, part.rows))
    return np.concatenate(found) if found else np.zeros(0, dtype=np.int64)


def _restrict(features: Features, rows: Sequence[int]) -> Features:
    columns = tuple(tuple(column[i] for i in rows) for column in features.columns)
    return Features(len(rows), features.names, columns, tuple(ABSENT not in c for c in columns), features.originals)
