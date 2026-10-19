"""Shared_Definition extraction and recursion detection (task 36.1; Requirements 8.1–8.8, 26.2).

Recursion (8.3, 8.4, 8.8)
    At an object Path ``A`` whose instances are all objects, the
    Structure_Analyzer looks for a relative step sequence ``S`` (a property,
    or a property's array elements) such that the objects at ``A + S`` share
    a property name with those at ``A``; the levels ``A, A + S, A + 2S, …``
    are the candidate recursion. :func:`level_grouping` turns the union of
    the levels into a cross-Path Merge-or-Split grouping (each Object
    Signature joins the level that holds most of its objects), which the
    ``merge_or_split`` Decision_Point resolves with its usual Decision_Model
    and MDL candidates (8.3). A merge makes one Shared_Definition inferred
    from the union of every level, and the analysis of that union emits a
    ``$ref`` to the definition at the recursive step ``A + S`` instead of
    recursing (8.4), so the schema holds one copy of the subschema and
    accepts deeper trees. With ``structure.recursion`` disabled no
    prefix-related pair is considered (8.8).

Extraction (8.1, 8.2, 8.6, 8.7)
    :class:`SharedDefinitionPass` walks the finished model, hashes every
    keyword subschema canonically (``canonical_bytes`` of its JSON, which
    holds no provenance), and for each hash at two or more distinct sites
    compares the model bits of the model with the sites inlined against the
    model with the subschema extracted once under ``$defs`` and a ``$ref``
    at every site (the data bits are the same). The ``shared_definition``
    Decision_Point resolves the two by MDL: extraction wins only with
    strictly fewer bits (8.2), otherwise the candidate, its sites, and both
    lengths are reported as rejected (8.6). With ``structure.shared_defs``
    disabled the pass does nothing (8.7).

Names (8.5)
    A definition is named after the object key with the most non-flagged
    Observations among its sites (the last Key step of each site's Path),
    ties to the smallest key, ``def`` when no site has a Key step.
    :func:`assign_names` resolves collisions: the colliding definition with
    the most Observations keeps the name and the others get the suffixes
    2, 3, … in descending Observation count, ties by the lexicographic
    order of their serialized subschemas.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping, Sequence
from urllib.parse import quote
from dataclasses import dataclass, field
from typing import Any, Final

import numpy as np

from jsinfer.config.registry import OUTCOME_INLINE, REGISTRY
from jsinfer.core.determinism import canonical_bytes, codepoint_key
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import Path, Step, render_path
from jsinfer.core.provenance import CandidateScore, Resolution
from jsinfer.core.report import InferenceReport, RejectedCandidate
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.mdl.scorer import MDLScorer, Score, model_bits
from jsinfer.structure.variants import VARIANT_GROUPING, Grouping, GroupingScorer, ObjectSignatures, VariantGrouping

__all__ = [
    "DEFAULT_NAME",
    "RECURSION",
    "SHARED_DEFINITION",
    "SHARED_DEFS",
    "Definition",
    "SharedDefinitionPass",
    "assign_names",
    "canonical_hash",
    "level_grouping",
    "preferred_name",
    "ref_node",
    "ref_string",
]

SHARED_DEFS: Final = "structure.shared_defs"
RECURSION: Final = "structure.recursion"
SHARED_DEFINITION: Final = "shared_definition"
OUTCOME_EXTRACT: Final = "extract"
DEFAULT_NAME: Final = "def"
REJECTED_CATEGORY: Final = "defs"
DEFS: Final = "$defs"
REF: Final = "$ref"
_MAX_SITES: Final = 64  # occurrences one extraction candidate considers (the rest stay inline)
_MAX_ROUNDS: Final = 32  # extraction rounds per model


def canonical_hash(node: SchemaNode) -> bytes:
    """The canonical bytes of a subschema's JSON, which carries no Provenance_Record (8.1)."""
    return canonical_bytes(node.to_json())


def ref_string(name: str) -> str:
    """``#/$defs/<name>`` as a URI fragment: the name as a JSON Pointer token (``~0``, ``~1``), percent-encoded."""
    token = name.replace("~", "~0").replace("/", "~1")
    return f"#/{DEFS}/{quote(token, safe=_FRAGMENT_SAFE)}"


def ref_node(name: str) -> SchemaNode:
    """A fresh ``{"$ref": "#/$defs/<name>"}`` node."""
    return SchemaNode(kw={REF: ref_string(name)})


_FRAGMENT_SAFE: Final = "!$&'()*+,;=:@"  # the characters a URI fragment may hold raw besides unreserved ones


def preferred_name(sites: Iterable[tuple[Path, int]]) -> str:
    """The key with the most Observations among the sites' last Key steps, ties to the smallest key; ``def`` without one (8.5)."""
    counts: dict[str, int] = {}
    for path, count in sites:
        keys = [step.key for step in path if step.kind == 0]
        if keys:
            counts[keys[-1]] = counts.get(keys[-1], 0) + count
    if not counts:
        return DEFAULT_NAME
    return min(counts, key=lambda k: (-counts[k], codepoint_key(k)))


@dataclass
class Definition:
    """A Shared_Definition before naming: its body, preferred name, Observation count, and the ``$ref`` nodes to it."""

    body: SchemaNode
    preferred: str
    count: int
    refs: list[SchemaNode] = field(default_factory=list)
    name: str | None = None
    decision: Decision | None = None  # the merge decision of a recursive definition (its refs' records)
    supports: dict[int, int] = field(default_factory=dict)  # ref node nid -> instances it covers
    at_root: bool = False  # a recursive definition whose body is the schema root: its refs are ``#``, no ``$defs`` entry


def assign_names(definitions: Sequence[Definition]) -> list[str]:
    """The final names (8.5): collisions keep the unsuffixed name for the most observed, suffixes 2, 3, … for the rest."""
    by_name: dict[str, list[int]] = {}
    for i, definition in enumerate(definitions):
        by_name.setdefault(definition.preferred, []).append(i)
    names = [""] * len(definitions)
    taken: set[str] = set()
    for preferred in sorted(by_name, key=codepoint_key):
        order = sorted(by_name[preferred], key=lambda i: (-definitions[i].count, canonical_hash(definitions[i].body)))
        for rank, i in enumerate(order):
            candidate = preferred if rank == 0 else f"{preferred}{rank + 1}"
            while candidate in taken:  # a suffixed name may collide with another preferred name
                rank += 1
                candidate = f"{preferred}{rank + 1}"
            taken.add(candidate)
            names[i] = candidate
    return names


# ---------------------------------------------------------------------------
# Cross-Path Merge-or-Split groupings (8.3, 8.4)
# ---------------------------------------------------------------------------


def _positions_of(member: View, union: View) -> np.ndarray:
    """The positions of ``member``'s rows among ``union``'s rows (``member`` is a subset of the same columns)."""
    index: dict[Any, tuple[int, np.ndarray]] = {}
    offset = 0
    for part in union.parts:
        index[part.column] = (offset, part.rows)
        offset += len(part.rows)
    found: list[np.ndarray] = []
    for part in member.live().parts:
        if part.column not in index:
            continue
        start, rows = index[part.column]
        found.append(start + np.searchsorted(rows, part.rows))
    return np.concatenate(found) if found else np.zeros(0, dtype=np.int64)


def level_grouping(
    union: View, members: Sequence[View], *, origin: str, engine: DecisionEngine, scorer: MDLScorer
) -> VariantGrouping | None:
    """A Merge-or-Split grouping of ``union`` whose candidate Variants are the member views (8.3).

    Each Object_Signature joins the member holding most of its objects (ties
    to the first member). The grouping is the only candidate of a
    ``variant_grouping`` Decision_Point at the union's Path, which
    ``engine`` records. None when fewer than two members keep a signature,
    in which case the members have the same shapes and merge without a
    judgment.
    """
    signatures = ObjectSignatures.of(union)
    n = signatures.n_signatures
    tallies = np.zeros((len(members), n), dtype=np.int64)
    for m, member in enumerate(members):
        for position in _positions_of(member, union).tolist():
            tallies[m, int(signatures.assignment[position])] += 1
    groups: dict[int, list[int]] = {}
    for s in range(n):
        column = tallies[:, s]
        groups.setdefault(int(np.argmax(column)), []).append(s)
    if len(groups) < 2:
        return None
    grouping = Grouping(None, tuple(tuple(g) for _, g in sorted(groups.items())), details={"origin": origin, "members": len(members)})
    grouping_scorer = GroupingScorer(signatures, scorer)
    label = f"{origin}:{grouping.label}"
    candidate = Candidate(technique=None, outcome=grouping, label=label, score=grouping_scorer.score(grouping.groups))
    # The grouping is given by the members, not chosen among grouping Techniques: a priority-style Decision
    # built here, outside the engine (the ``merge_or_split`` judgment that follows is the engine's).
    decision = Decision(
        kind=VARIANT_GROUPING,
        path=union.path,
        method="priority",
        label=label,
        outcome=grouping,
        candidate=candidate,
        technique=origin,
        confidence=None,
        resolution=Resolution("priority", 0, (CandidateScore(origin, label, None),)),
        details={"origin": origin, "members": len(members)},
    )
    return VariantGrouping(union.path, signatures, grouping, decision, (grouping,), scorer=grouping_scorer)


# ---------------------------------------------------------------------------
# Extraction (8.1, 8.2, 8.6)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _Site:
    parent: SchemaNode | None  # None for a $defs body's own parent (the root's $defs map)
    container: Any  # the dict or list holding the node, or the parent's kw
    key: Any
    node: SchemaNode
    pointer: str


def _sites(root: SchemaNode) -> list[_Site]:
    """Every keyword subschema below the root with its holder (walk order)."""
    out: list[_Site] = []

    def visit(node: SchemaNode, pointer: str) -> None:
        if node.boolean is not None:
            return
        for keyword, value in node.kw.items():
            if isinstance(value, SchemaNode):
                out.append(_Site(node, node.kw, keyword, value, f"{pointer}/{keyword}"))
                visit(value, f"{pointer}/{keyword}")
            elif isinstance(value, dict):
                for name, child in value.items():
                    if isinstance(child, SchemaNode):
                        token = name.replace("~", "~0").replace("/", "~1")
                        out.append(_Site(node, value, name, child, f"{pointer}/{keyword}/{token}"))
                        visit(child, f"{pointer}/{keyword}/{token}")
            elif isinstance(value, list):
                for i, child in enumerate(value):
                    if isinstance(child, SchemaNode):
                        out.append(_Site(node, value, i, child, f"{pointer}/{keyword}/{i}"))
                        visit(child, f"{pointer}/{keyword}/{i}")

    visit(root, "")
    return out


def _path_of_pointer(pointer: str) -> Path:
    """A Path-like tuple of Key steps from a JSON Pointer (``properties/<name>`` segments only), for naming."""
    tokens = pointer.split("/")[1:]
    steps: list[Step] = []
    for i, token in enumerate(tokens):
        if i and tokens[i - 1] == "properties":
            steps.append(Step(0, token.replace("~1", "/").replace("~0", "~")))
    return tuple(steps)


def _extractable(node: SchemaNode) -> bool:
    return node.boolean is None and len(node.kw) >= 2 and REF not in node.kw and DEFS not in node.kw


class SharedDefinitionPass:
    """Extracts Shared_Definitions from a finished model (see the module docstring)."""

    __slots__ = ("_engine", "_report", "_scorer")

    def __init__(self, engine: DecisionEngine, *, scorer: MDLScorer | None = None, report: InferenceReport | None = None) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        self._engine = engine
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._report = report

    def in_effect(self) -> bool:
        return REGISTRY.technique(SHARED_DEFS).implemented and self._engine.enabled(SHARED_DEFS)

    def run(
        self,
        root: SchemaNode,
        definitions: list[Definition],
        counts: Mapping[int, int],
        *,
        views: Mapping[int, View] | None = None,
        on_extract: Any = None,
    ) -> list[tuple[Definition, Decision]]:
        """Extract equivalent subschemas of ``root`` into ``definitions`` (8.1, 8.2, 8.6).

        ``counts`` maps a node nid to its non-flagged Observations (for the
        names of 8.5; 1 when absent). ``views`` maps a node nid to the view
        it was inferred from; when given, only sites with a view are
        candidates (the definition is re-inferred from their union).
        ``on_extract(body, nodes, refs, decision)`` is told the body node,
        every site's node (the body first) and the ``$ref`` node that took
        each site's place, and may return a node to stand as the body
        instead. Returns the extracted definitions with their decisions;
        the ``$ref`` nodes are in each definition's ``refs``.
        """
        extracted: list[tuple[Definition, Decision]] = []
        if not self.in_effect():
            return extracted
        rejected: set[bytes] = set()
        for _ in range(_MAX_ROUNDS):
            groups: dict[bytes, list[_Site]] = {}
            for site in _sites(root):
                if _extractable(site.node) and (views is None or site.node.nid in views):
                    groups.setdefault(canonical_hash(site.node), []).append(site)
            candidates = [sites for digest, sites in groups.items() if len(sites) >= 2 and digest not in rejected]
            if not candidates:
                break
            candidates.sort(key=lambda sites: (-len(sites), sites[0].pointer))
            sites = candidates[0][:_MAX_SITES]
            digest = canonical_hash(sites[0].node)
            nested = {id(s.node) for s in sites}
            if any(id(inner) in nested for s in sites for _, inner in s.node.walk() if inner is not s.node):
                rejected.add(digest)  # a site inside another site of the same candidate: not extracted together
                continue
            decision = self._decide(root, sites, definitions, views)
            if decision.label != OUTCOME_EXTRACT:
                rejected.add(digest)
                if self._report is not None:
                    self._report.record_rejected(RejectedCandidate(REJECTED_CATEGORY, render_path(_path_of_pointer(sites[0].pointer)), None, SHARED_DEFS, {**decision.details, "sites": [s.pointer for s in sites]}))
                continue
            body = sites[0].node
            weighted = [(_path_of_pointer(s.pointer), max(1, int(counts.get(s.node.nid, 1)))) for s in sites]
            definition = Definition(body, preferred_name(weighted), sum(c for _, c in weighted))
            for site in sites:
                ref = ref_node(f"{len(definitions)}")  # a provisional name; assign_names settles it
                site.container[site.key] = ref
                definition.refs.append(ref)
            definitions.append(definition)
            extracted.append((definition, decision))
            if on_extract is not None:
                new_body = on_extract(body, [s.node for s in sites], definition.refs, decision)
                if isinstance(new_body, SchemaNode):
                    definition.body = new_body
        return extracted

    def _decide(
        self, root: SchemaNode, sites: Sequence[_Site], definitions: Sequence[Definition], views: Mapping[int, View] | None
    ) -> Decision:
        """The ``shared_definition`` Decision_Point: the model inlined against extracted, by model bits (8.1, 8.2).

        Both are scored on copies that carry the definitions extracted so
        far under ``$defs`` (provisional names), so the ``$ref`` costs see
        the right definition count.
        """
        inlined = _copy_with_defs(root, definitions)
        extracted = _copy_with_defs(root, definitions)
        name = f"{len(definitions)}"
        body = SchemaNode.from_json(sites[0].node.to_json())
        for site in sites:
            _replace_at(extracted, site.pointer, ref_node(name))
        extracted.kw.setdefault(DEFS, {})[name] = body
        inlined_bits, extracted_bits = model_bits(inlined), model_bits(extracted)
        path = _path_of_pointer(sites[0].pointer)
        n = len(sites)
        options = (
            Candidate(technique=None, outcome=OUTCOME_INLINE, order_key=(0,), score=Score(inlined_bits, 0.0, 0.0, 0, n), label=OUTCOME_INLINE),
            Candidate(technique=SHARED_DEFS, outcome=OUTCOME_EXTRACT, order_key=(1,), score=Score(extracted_bits, 0.0, 0.0, 0, n), label=OUTCOME_EXTRACT),
        )
        details: dict[str, JSON] = {"inlined_bits": inlined_bits, "extracted_bits": extracted_bits, "sites": [s.pointer for s in sites]}
        if views is not None:
            details["site_paths"] = [render_path(views[s.node.nid].path) for s in sites]
        return self._engine.resolve(
            DecisionPoint(SHARED_DEFINITION, path), options, None, mdl_by=SHARED_DEFS, scorer=self._scorer, details=details
        )


def _copy_with_defs(root: SchemaNode, definitions: Sequence[Definition]) -> SchemaNode:
    """A fresh copy of ``root`` with the definitions extracted so far under ``$defs`` (provisional names)."""
    copy = SchemaNode.from_json(root.to_json())
    if definitions:
        defs = dict(copy.kw.get(DEFS, {}))
        for i, definition in enumerate(definitions):
            defs[f"{i}"] = SchemaNode.from_json(definition.body.to_json())
        copy.kw[DEFS] = defs
    return copy


def _replace_at(root: SchemaNode, pointer: str, new: SchemaNode) -> None:
    """Replace the subschema at JSON Pointer ``pointer`` (as :func:`_sites` prints them) with ``new``."""
    # ``split`` rather than ``strip``: an empty property name is a trailing empty token, not nothing.
    tokens = [t.replace("~1", "/").replace("~0", "~") for t in pointer.split("/")[1:]]
    node = root
    i = 0
    while i < len(tokens):
        keyword = tokens[i]
        value = node.kw[keyword]
        if isinstance(value, SchemaNode):
            if i + 1 == len(tokens):
                node.kw[keyword] = new
                return
            node = value
            i += 1
        elif isinstance(value, dict):
            entry = tokens[i + 1]
            if i + 2 == len(tokens):
                value[entry] = new
                return
            node = value[entry]
            i += 2
        else:
            index = int(tokens[i + 1])
            if i + 2 == len(tokens):
                value[index] = new
                return
            node = value[index]
            i += 2
    raise KeyError(pointer)


def _root_defs(root: SchemaNode) -> dict[str, SchemaNode]:
    defs = root.kw.get(DEFS) if root.boolean is None else None
    return defs if isinstance(defs, dict) else {}
