"""Relaxation table: widening candidates for failing Constraints (Requirements 3.3, 3.5, 3.6).

The soundness loop (``pipeline.soundness``, task 15.3) validates every
non-flagged document against the inferred schema. For each Constraint the
Validator reports as failing, :func:`relaxations` lists the candidate
relaxations of the design's relaxation table, least widening first, and
:func:`removal` gives the removal alone (the removal loop of 3.7). Every
candidate only widens the schema (3.5): each document that validated before
it is applied still validates after.

Failures
    A :class:`Failure` names the failing Constraint by the ``nid`` of the
    node that holds the keyword, the ``keyword``, and the ``item`` of a
    ``required`` entry (the missing property) or of a ``dependentRequired``
    entry (the antecedent). ``values`` are the instances that node was
    evaluated on and rejected, and ``documents`` the indices of the
    documents they come from. Two kinds of failure are not a single
    ConstraintId (:attr:`Failure.constraint` is None for them):

    - ``anyOf`` / ``oneOf`` as a whole (no branch matches, or two or more
      ``oneOf`` branches match). ``branches`` may then hold, per branch
      index, the branch-local failures of the values (the Constraints inside
      the branch that rejected them); an empty tuple for a branch means it
      accepted the values.
    - :data:`FALSE_SCHEMA`: a boolean ``false`` subschema that rejected the
      values, at a position the table does not name (``properties/<name>``,
      ``prefixItems/<i>``, a branch, the root). ``false`` under ``items`` and
      ``additionalProperties`` are the ``items`` and ``additionalProperties``
      Constraints of the owner node instead.

Candidates (the design's relaxation table)
    ======================================  =====================================================
    Failing keyword                         Candidates, in documented order
    ======================================  =====================================================
    ``type``                                add the values' types (``integer`` becomes ``number``
                                            when a value is non-integral); remove
    ``enum`` / ``const``                    add the values (``const`` becomes ``enum``); remove
    ``format``                              remove
    ``pattern``                             re-induce on the union (Validator-checked); remove
    the ten bounds                          the next looser bound-ladder candidate that accepts
                                            the values; remove
    ``required[i]``, ``dependentRequired    remove
    [k]``, ``uniqueItems``, ``contains``,
    ``propertyNames``, ``not``
    ``additionalProperties: false``         add the new properties with Naive_Merge subschemas;
                                            remove (open)
    ``items: false``                        replace with the Naive_Merge ``items`` of the extra
                                            elements; remove
    ``anyOf``                               relax one branch (per branch with branch-local
                                            failures); add a branch inferred from the values;
                                            remove
    ``oneOf``                               convert to ``anyOf``; convert plus each ``anyOf``
                                            candidate; remove
    :data:`FALSE_SCHEMA`                    replace with the Naive_Merge of the values;
                                            replace with ``true`` (remove)
    ======================================  =====================================================

    Details:

    - ``add-types`` lists the old and the new JSON types in emitted order with
      the integer rule (``values.types.type_names``).
    - ``add-values`` keeps the old members and adds the distinct values under
      ``enum`` equality, in canonical value order. ``const`` becomes ``enum``
      only when the node holds no ``enum`` (otherwise only removal is
      offered).
    - ``loosen-bound`` builds the bound ladder of 16.2 (``values.bounds``)
      from the values' quantities (numbers; string lengths in code points;
      array lengths; object key counts) and takes the tightest rung that is
      strictly looser than the current bound. Every rung accepts every
      value; an exclusive bound also needs a rung strictly beyond the
      extreme. The Path's earlier Observations satisfy the current bound,
      so the extreme of the union is the values' extreme and the ladder
      needs no view.
    - ``re-induce`` widens the Pattern to also match the strings it rejected.
      They are profiled with the FlashProfile atoms of ``values.patterns``
      (maximal runs of ``[0-9]``, ``[a-z]``, ``[A-Z]``, or one literal), one
      alternative per atom sequence, with the observed run-length range per
      token, and unbounded digit runs when every string of the alternative
      encodes an integer or decimal number (as ``values.latent`` does). An
      old safe-AST Pattern keeps its alternatives: a new alternative with the
      atom sequence of an old one replaces it with their hull (each
      quantifier widened to cover both), and the others are appended. The
      result accepts every string the old Pattern accepted, which is what
      makes it monotone, and is the induction on the union for Patterns
      built this way. Any other old pattern is kept as ``(?:old)|new``.
      Either way the Validator checks that the new pattern compiles and
      matches every given string; otherwise the candidate is not offered.
    - ``add-properties`` adds, for each key of the values that is not in
      ``properties`` and matches no ``patternProperties`` pattern (the keys
      ``additionalProperties`` applies to), its Naive_Merge subschema over
      the member values (``pipeline.naive_merge.naive_merge_values``).
    - ``replace-items`` merges the elements past ``prefixItems``.
    - ``relax-branch`` applies, for each branch-local failure of the branch
      (merged per Constraint, in target order: the node's JSON Pointer, then
      keyword, then item), the first of its own candidates.
    - ``add-branch`` appends the Naive_Merge of the values.
    - ``convert`` alone is offered for ``oneOf`` unless ``branches`` shows
      that no branch matched, since converting cannot fix that.

    A candidate that would change nothing (for example values whose types
    are already listed) is not offered. Removal is always offered, last.

Monotonicity (3.5)
    Each primitive edit widens the node it touches: types, members, bounds,
    Patterns, properties, items, and branches only accept more, and removing
    an assertion or a positive applicator only drops a conjunct. A node is
    evaluated in a positive position unless a path from the root to it
    passes through ``not`` (negated) or ``if`` (whose result selects
    ``then`` or ``else``, so widening it can narrow), or through a ``oneOf``
    branch (where widening one branch can make a value match two). Paths
    follow subschema keywords and local ``$ref`` hops, but not the ``$defs``
    container, which is evaluated only through references. So an edit
    inside a ``$defs`` entry applies at every referencing site, and the
    guards below consider every such site. For the set E of nodes a
    candidate edits:

    - ``not`` and ``if`` contents are never edited: every ``not`` or ``if``
      whose owner is reachable from the root through positive edges and
      whose subschema reaches a node of E is removed as part of the
      candidate (``if`` together with its ``then`` and ``else``). Its owner
      joins E, and this repeats until no such keyword remains. Afterwards
      no path from the root to a node of E passes through ``not`` or ``if``.
      Validation errors are never reported inside a ``not`` or an ``if``,
      so this only happens for a ``$defs`` entry that a ``not`` or ``if``
      also references; the Inferrer emits no such schema.
    - Every ``oneOf`` reachable from the root with a branch that reaches a
      node of E is converted to ``anyOf``. A node that already holds
      ``anyOf`` gets the converted branches as a new ``allOf`` entry
      ``{"anyOf": [...]}``.

    These guard edits come after the candidate's own edits. An edit whose
    node an earlier edit of the candidate removed is skipped.

References
    Edits keep local ``$ref`` values resolving. A conversion rewrites the
    references into the converted branches. Removing (or replacing) a
    subschema-valued keyword first moves every subschema in it that a
    remaining reference targets into the root ``$defs`` (named
    ``relaxed``, ``relaxed-2``, ...) and rewrites those references, so their
    sites keep their meaning. Only loaded schemas hold such references.

Changes and records (3.6)
    :meth:`Relaxation.apply` edits a model in place and returns one
    :class:`Change` per applied edit: the node, the JSON fragments of the
    touched keywords before and after (``REMOVED`` when none remains), the
    ConstraintIds before and after (a ``oneOf`` branch ``(n, "oneOf", i)``
    becomes ``(n, "anyOf", i)``; an added branch is a new id), and whether
    it was a monotonicity guard. A conversion beside ``anyOf`` touches the
    owner's ``allOf``, a single Constraint ``(n, "allOf")``: its branches
    become ``(w, "anyOf", i)`` of the wrapper ``w``, ``after_ids`` lists
    ``(n, "allOf")`` too, and when the ``allOf`` already existed it is in
    ``before_ids`` as well, with the old ``allOf`` in ``before`` and the
    whole new one in ``after``. :meth:`Change.record` builds the
    RelaxationRecord. ``apply`` leaves ``SchemaModel.provenance`` alone.
    :meth:`Relaxation.applied` applies to a copy and leaves the input
    unchanged.

Hooks
    Soundness loop (task 15.3). Map each Validator error to a Failure: the
    keyword location resolves to the owner node and keyword. A
    ``falseSchema`` error at ``…/items`` or ``…/additionalProperties`` is
    that keyword of the owner, with the arrays or objects (the parent
    instance) as values; at other positions it is :data:`FALSE_SCHEMA` at
    the ``false`` node. The ``propertyNames`` and ``contains`` kinds are
    those keywords of the owner, whatever subschema keyword the location
    names. A ``required`` error gives the missing property as the item;
    jsonschema-rs reports ``dependentRequired`` with the ``required`` kind
    at ``…/dependentRequired``, and the item is the antecedent whose
    consequent is missing. ``anyOf`` and ``oneOf`` errors carry
    jsonschema-rs's per-branch ``context``, which becomes ``branches``.
    Group failures by ``(nid, keyword, item)`` and process the groups in
    target order (the node's JSON Pointer, then keyword, then item, in
    code-point order), never in document order (27.6), scoring each against
    the model as relaxed so far. Call :func:`relaxations`, and resolve the
    ``relaxation`` Decision_Point (:data:`RELAXATION`) with
    :meth:`Relaxation.candidate` for each candidate, using
    ``DecisionPoint(RELAXATION, path)`` and the Path's view (its
    Observations plus the failing values). With ``decide.mdl`` enabled the
    engine scores each candidate's ``node`` (:meth:`Relaxation.preview`)
    on the view, or a precomputed ``score`` (for example from an
    ``MDLScorer`` rooted at the relaxed model, so ``$ref`` hops see the
    edited ``$defs``); otherwise the Fallback_Behavior takes the first
    candidate in the documented order (``order_key`` is
    :attr:`Relaxation.order`). Keep only candidates that make the failing
    documents validate (3.3) by validating them against
    :meth:`Relaxation.applied`. Apply the winner
    (``decision.outcome.apply(model)``), then move or append Provenance_Records
    following each Change's ids: a removed Constraint's record goes to the
    report, a relaxed one gets ``change.record(documents,
    decision.resolution)`` in its ``changes`` and its Support recomputed.
    The removal loop of 3.7 applies :func:`removal`.

    Monotonicity property (task 15.2, Property 8). For any model, Failure,
    and candidate, every document valid against the model is valid against
    ``relaxation.applied(model)[0]``; sequences of candidates compose,
    since each one is computed on the model it is applied to.

    Pattern induction (task 32.1). ``re-induce`` uses the atom profile
    above. When ``values.pattern`` lands, its induction over the Path's
    strings plus the failing strings can replace the profile step, merged
    with the old Pattern by the same hull-or-alternation rule so the result
    still accepts every string the old Pattern accepted.

Determinism
    Candidates, edits, and guards follow the canonical pre-order of the
    model, code-point order of names and pointers, and canonical value
    order; no set is iterated, and no order depends on nid values, which
    follow the order in which nodes were made rather than the model. New
    nodes take fresh nids (Naive_Merge templates when the candidates are
    built, ``allOf`` wrappers of conversions), so applying one candidate to
    a model and to a copy gives equal results.
"""

from __future__ import annotations

import copy
import json
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, replace
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias
from urllib.parse import quote, unquote

from jsinfer.core.determinism import codepoint_key, sorted_codepoint
from jsinfer.core.jsonvalue import JSON, JType, distinct_values, jtype_of
from jsinfer.core.provenance import REMOVED, RelaxationRecord, Removed, Resolution
from jsinfer.core.schema_model import (
    ITEMIZED_KEYWORDS,
    NON_CONSTRAINT_KEYWORDS,
    SUBSCHEMA_KEYWORDS,
    ConstraintId,
    SchemaModel,
    SchemaNode,
    join_pointer,
    new_nid,
    split_pointer,
)
from jsinfer.decide.engine import Candidate
from jsinfer.io.validator import CompileError, Validator
from jsinfer.mdl.scorer import Score
from jsinfer.pipeline.naive_merge import naive_merge_values
from jsinfer.values.bounds import (
    ITEM_COUNT,
    KEY_COUNT,
    MAXIMUM,
    MINIMUM,
    NUMBER,
    STRING_LENGTH,
    QuantityKind,
    QuantitySet,
    Side,
    bound_ladder,
)
from jsinfer.values.latent import latent_type_of
from jsinfer.values.patterns import PLUS, Alternative, CharClass, Pattern, Quantifier, Token
from jsinfer.values.types import type_names

__all__ = [
    "ACTIONS",
    "BOUND_SPECS",
    "FALSE_SCHEMA",
    "RELAXABLE_KEYWORDS",
    "RELAXATION",
    "Action",
    "Change",
    "Failure",
    "Relaxation",
    "relaxations",
    "removal",
]

# The Decision_Point kind that selects among the candidates (served by decide.mdl).
RELAXATION: Final = "relaxation"

# Failure.keyword of a boolean false subschema that rejected values.
FALSE_SCHEMA: Final = "false"

Action: TypeAlias = Literal[
    "add-types",
    "add-values",
    "add-properties",
    "replace-items",
    "replace",
    "convert",
    "loosen-bound",
    "re-induce",
    "relax-branch",
    "add-branch",
    "remove",
]

# The candidate actions with their rank in the documented order (least
# widening first: add value or type, loosen to the next bound candidate,
# re-induce, then remove; the structural edits of the table in between).
_ACTION_RANKS: Final[Mapping[str, int]] = MappingProxyType(
    {
        "add-types": 0,
        "add-values": 0,
        "add-properties": 0,
        "replace-items": 0,
        "replace": 0,
        "convert": 0,
        "loosen-bound": 1,
        "re-induce": 2,
        "relax-branch": 3,
        "add-branch": 4,
        "remove": 5,
    }
)
ACTIONS: Final[tuple[str, ...]] = tuple(_ACTION_RANKS)

# Each bound keyword: the kind of quantity it bounds, its side, and whether it is exclusive.
BOUND_SPECS: Final[Mapping[str, tuple[QuantityKind, Side, bool]]] = MappingProxyType(
    {
        "minimum": (NUMBER, MINIMUM, False),
        "exclusiveMinimum": (NUMBER, MINIMUM, True),
        "maximum": (NUMBER, MAXIMUM, False),
        "exclusiveMaximum": (NUMBER, MAXIMUM, True),
        "minLength": (STRING_LENGTH, MINIMUM, False),
        "maxLength": (STRING_LENGTH, MAXIMUM, False),
        "minItems": (ITEM_COUNT, MINIMUM, False),
        "maxItems": (ITEM_COUNT, MAXIMUM, False),
        "minProperties": (KEY_COUNT, MINIMUM, False),
        "maxProperties": (KEY_COUNT, MAXIMUM, False),
    }
)

# The keywords a Failure may name: the rows of the relaxation table.
RELAXABLE_KEYWORDS: Final = frozenset(
    {
        "additionalProperties",
        "anyOf",
        "const",
        "contains",
        "dependentRequired",
        "enum",
        "format",
        "items",
        "not",
        "oneOf",
        "pattern",
        "propertyNames",
        "required",
        "type",
        "uniqueItems",
        FALSE_SCHEMA,
        *BOUND_SPECS,
    }
)

_ENTRY_KEYWORDS: Final = ("dependentRequired", "required")
_COMBINATORS: Final = ("anyOf", "oneOf")

# Subschema keywords whose contents are not in a positive position, and the
# keywords removed together when such a keyword must go.
_BLOCKED: Final[Mapping[str, tuple[str, ...]]] = MappingProxyType({"if": ("else", "if", "then"), "not": ("not",)})

# Characters a $ref fragment may hold unescaped besides letters, digits, and "_.-~" (RFC 3986 pchar and "/").
_FRAGMENT_SAFE: Final = "/!$&'()*+,;=:@"
_RELOCATED_NAME: Final = "relaxed"


def _label_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


# ---------------------------------------------------------------------------
# Failures
# ---------------------------------------------------------------------------


def _nid(value: Any, what: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{what} must be an int, got {type(value).__name__}")
    if value < 0:
        raise ValueError(f"{what} must be non-negative, got {value}")
    return value


@dataclass(frozen=True, eq=False, slots=True)
class Failure:
    """One failing Constraint and the instances it rejected (see the module docstring).

    ``values`` and ``documents`` are stored as tuples; ``documents`` sorted
    without repeats. ``branches`` is only for ``anyOf`` and ``oneOf``.
    """

    nid: int
    keyword: str
    item: str | None = None
    values: tuple[JSON, ...] = ()
    documents: tuple[int, ...] = ()
    branches: tuple[tuple[Failure, ...], ...] = ()

    def __post_init__(self) -> None:
        _nid(self.nid, "Failure nid")
        keyword = self.keyword
        if not isinstance(keyword, str) or keyword not in RELAXABLE_KEYWORDS:
            raise ValueError(
                f"Failure keyword {keyword!r} is not in the relaxation table; "
                f"expected one of {sorted(RELAXABLE_KEYWORDS, key=codepoint_key)}"
            )
        if keyword in _ENTRY_KEYWORDS:
            if not isinstance(self.item, str):
                raise ValueError(f"a {keyword!r} Failure names its entry; item must be a str, got {self.item!r}")
        elif self.item is not None:
            raise ValueError(f"a {keyword!r} Failure is keyword-level; item must be None, got {self.item!r}")
        values = self.values
        if isinstance(values, (str, bytes, dict)) or not isinstance(values, Iterable):
            raise TypeError(f"Failure values must be a sequence of JSON values, got {type(values).__name__}")
        object.__setattr__(self, "values", tuple(values))
        documents = self.documents
        if isinstance(documents, (str, bytes)) or not isinstance(documents, Iterable):
            raise TypeError(f"Failure documents must be a sequence of ints, got {type(documents).__name__}")
        object.__setattr__(
            self, "documents", tuple(sorted(dict.fromkeys(_nid(d, "Failure document index") for d in documents)))
        )
        branches = self.branches
        if isinstance(branches, (str, bytes)) or not isinstance(branches, Iterable):
            raise TypeError(f"Failure branches must be a sequence of Failure tuples, got {type(branches).__name__}")
        normalized: list[tuple[Failure, ...]] = []
        for branch in branches:
            if isinstance(branch, Failure) or not isinstance(branch, Iterable):
                raise TypeError("each Failure branch entry is a sequence of the branch's Failures")
            inner = tuple(branch)
            for failure in inner:
                if not isinstance(failure, Failure):
                    raise TypeError(f"branch failures must be Failures, got {type(failure).__name__}")
            normalized.append(inner)
        if normalized and keyword not in _COMBINATORS:
            raise ValueError(f"only anyOf and oneOf Failures carry branch failures, not {keyword!r}")
        object.__setattr__(self, "branches", tuple(normalized))

    @property
    def target(self) -> tuple[int, str, str | None]:
        """``(nid, keyword, item)``: what the Failure names."""
        return (self.nid, self.keyword, self.item)

    @property
    def constraint(self) -> ConstraintId | None:
        """The failing ConstraintId; None for an ``anyOf``/``oneOf`` as a whole and for :data:`FALSE_SCHEMA`."""
        if self.keyword in _COMBINATORS or self.keyword == FALSE_SCHEMA:
            return None
        return ConstraintId(self.nid, self.keyword, self.item)


def _target_key(failure: Failure, tokens: Mapping[int, Sequence[str]]) -> tuple[tuple[str, ...], str, str]:
    """Target order: the node's JSON Pointer tokens, keyword, item, in code-point order (a function of the model).

    An nid outside the model sorts with the root; ``_Context.node`` rejects it afterwards.
    """
    pointer = tuple(codepoint_key(token) for token in tokens.get(failure.nid, ()))
    return (pointer, codepoint_key(failure.keyword), codepoint_key(failure.item or ""))


def _merged(failures: Sequence[Failure], tokens: Mapping[int, Sequence[str]]) -> list[Failure]:
    """One Failure per target with the values, documents, and branch failures of all of them, in target order.

    ``tokens`` holds the pointer tokens of every node by nid (``_Context.tokens``).
    """
    groups: dict[tuple[int, str, str | None], list[Failure]] = {}
    for failure in failures:
        groups.setdefault(failure.target, []).append(failure)
    merged: list[Failure] = []
    for group in groups.values():
        first = group[0]
        width = max(len(f.branches) for f in group)
        branches = tuple(
            tuple(inner for f in group if index < len(f.branches) for inner in f.branches[index])
            for index in range(width)
        )
        merged.append(
            Failure(
                first.nid,
                first.keyword,
                first.item,
                tuple(v for f in group for v in f.values),
                tuple(d for f in group for d in f.documents),
                branches,
            )
        )
    merged.sort(key=lambda failure: _target_key(failure, tokens))
    return merged


# ---------------------------------------------------------------------------
# Changes
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class Change:
    """One applied edit (see Changes and records in the module docstring).

    ``edit`` names the primitive edit (``rewrite``, ``delete-entry``,
    ``add-properties``, ``append-branch``, ``convert``, ``replace``).
    ``before`` and ``after`` are JSON fragments of the edited node (``after``
    is :data:`~jsinfer.core.provenance.REMOVED` when none of the touched
    keywords remains). ``guard`` marks a monotonicity guard edit.
    """

    edit: str
    nid: int
    before: JSON
    after: JSON | Removed
    before_ids: tuple[ConstraintId, ...]
    after_ids: tuple[ConstraintId, ...]
    guard: bool = False

    @property
    def removed(self) -> bool:
        """True when the edit removed the Constraints it touched."""
        return self.after is REMOVED

    def record(self, documents: Iterable[int], resolution: Resolution | None = None) -> RelaxationRecord:
        """The RelaxationRecord of this change, triggered by ``documents`` and chosen by ``resolution`` (3.6)."""
        return RelaxationRecord(self.before, self.after, tuple(documents), resolution)


def _value_json(keyword: str, value: Any) -> JSON:
    shape = SUBSCHEMA_KEYWORDS.get(keyword)
    if shape == "single":
        return value.to_json()
    if shape == "list":
        return [child.to_json() for child in value]
    if shape == "map":
        return {name: child.to_json() for name, child in value.items()}
    return copy.deepcopy(value)


def _fragment(node: SchemaNode, keywords: Iterable[str]) -> dict[str, JSON]:
    if node.boolean is not None:
        return {}
    return {keyword: _value_json(keyword, node.kw[keyword]) for keyword in keywords if keyword in node.kw}


def _ids(node: SchemaNode, keywords: Iterable[str]) -> tuple[ConstraintId, ...]:
    """The ConstraintIds the given keywords of ``node`` hold, in ConstraintId order."""
    if node.boolean is not None:
        return ()
    found: list[ConstraintId] = []
    for keyword in keywords:
        if keyword not in node.kw or keyword in NON_CONSTRAINT_KEYWORDS:
            continue
        value = node.kw[keyword]
        kind = ITEMIZED_KEYWORDS.get(keyword)
        if kind is None:
            found.append(ConstraintId(node.nid, keyword))
        elif kind is int:
            found.extend(ConstraintId(node.nid, keyword, index) for index in range(len(value)))
        else:
            names = value if isinstance(value, list) else list(value) if isinstance(value, dict) else []
            found.extend(ConstraintId(node.nid, keyword, name) for name in dict.fromkeys(names) if isinstance(name, str))
    return tuple(sorted(dict.fromkeys(found)))


# ---------------------------------------------------------------------------
# References
# ---------------------------------------------------------------------------


def _ref_tokens(node: SchemaNode) -> list[str] | None:
    """The JSON Pointer tokens of a node's local ``$ref``, or None when it has none."""
    if node.boolean is not None:
        return None
    ref = node.kw.get("$ref")
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None
    pointer = unquote(ref[1:])
    if pointer and not pointer.startswith("/"):
        return None  # a plain-name fragment (an anchor)
    try:
        return split_pointer(pointer)
    except ValueError:
        return None


def _ref_string(tokens: Sequence[str]) -> str:
    return "#" + quote(join_pointer("", *tokens), safe=_FRAGMENT_SAFE)


def _starts(tokens: Sequence[str], prefix: Sequence[str]) -> bool:
    return len(tokens) >= len(prefix) and list(tokens[: len(prefix)]) == list(prefix)


def _rewrite_refs(model: SchemaModel, old: Sequence[str], new: Sequence[str]) -> None:
    """Point every local ``$ref`` whose target starts with ``old`` at the same place under ``new``."""
    for _, node in model.walk():
        tokens = _ref_tokens(node)
        if tokens is not None and _starts(tokens, old):
            node.kw["$ref"] = _ref_string([*new, *tokens[len(old) :]])


def _minimal(prefixes: Iterable[list[str]]) -> list[list[str]]:
    """The prefixes no other one is a prefix of, shortest first, then in code-point order of their tokens."""
    ordered = sorted(prefixes, key=lambda tokens: (len(tokens), [codepoint_key(t) for t in tokens]))
    tops: list[list[str]] = []
    for tokens in ordered:
        if not any(_starts(tokens, top) for top in tops):
            tops.append(tokens)
    return tops


def _drop(model: SchemaModel, tokens: list[str], node: SchemaNode, keyword: str) -> None:
    """Delete ``node.kw[keyword]``, keeping every subschema in it that a remaining ``$ref`` targets.

    Such subschemas move into the root ``$defs`` and the references follow
    them (References in the module docstring). References from a moved
    subschema count as remaining, so the moved set is closed.
    """
    prefix = [*tokens, keyword]
    if keyword in SUBSCHEMA_KEYWORDS:
        holders = [(split_pointer(pointer), holder) for pointer, holder in model.walk()]
        tops: list[list[str]] = []
        while True:
            targets = [
                target
                for where, holder in holders
                if not _starts(where, prefix) or any(_starts(where, top) for top in tops)
                if (target := _ref_tokens(holder)) is not None and _starts(target, prefix)
            ]
            grown = _minimal([*tops, *targets])
            if grown == tops:
                break
            tops = grown
    else:
        tops = []
    value = node.kw.pop(keyword)
    if not tops:
        return
    detached = SchemaNode(kw={keyword: value})
    defs = model.root.kw.setdefault("$defs", {})
    moves: list[tuple[list[str], str]] = []
    for top in tops:
        try:
            moved = detached.resolve(join_pointer("", *top[len(tokens) :]))
        except KeyError:
            continue  # the reference never resolved; leave it as it was
        name = _RELOCATED_NAME
        suffix = 2
        while name in defs:
            name = f"{_RELOCATED_NAME}-{suffix}"
            suffix += 1
        defs[name] = moved
        moves.append((top, name))
    # Rewrite once every moved subschema is in the model, so references from one moved subschema to another follow too.
    for top, name in moves:
        _rewrite_refs(model, top, ["$defs", name])


# ---------------------------------------------------------------------------
# Primitive edits
# ---------------------------------------------------------------------------


class _Edit:
    """A primitive widening edit of the node with ``nid``; ``apply`` returns its Change, or None when it does nothing."""

    __slots__ = ()
    nid: int

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        raise NotImplementedError


@dataclass(frozen=True, eq=False, slots=True)
class _Rewrite(_Edit):
    """Delete keywords and set others (JSON values or SchemaNode templates, copied on apply)."""

    nid: int
    delete: tuple[str, ...] = ()
    assign: tuple[tuple[str, Any], ...] = ()

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        if node.boolean is not None:
            return None
        keys = sorted_codepoint(dict.fromkeys([*self.delete, *(keyword for keyword, _ in self.assign)]))
        before = _fragment(node, keys)
        if not before and not self.assign:
            return None
        before_ids = _ids(node, keys)
        for keyword in self.delete:
            if keyword in node.kw:
                _drop(model, tokens, node, keyword)
        for keyword, value in self.assign:
            if keyword in node.kw:
                _drop(model, tokens, node, keyword)
            node.kw[keyword] = copy.deepcopy(value)
        after = _fragment(node, keys)
        return Change("rewrite", self.nid, before, after or REMOVED, before_ids, _ids(node, keys))


@dataclass(frozen=True, eq=False, slots=True)
class _DeleteEntry(_Edit):
    """Delete one ``required`` name or one ``dependentRequired`` entry (the keyword goes when it empties)."""

    nid: int
    keyword: str
    name: str

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        value = None if node.boolean is not None else node.kw.get(self.keyword)
        if self.keyword == "required":
            if not isinstance(value, list) or self.name not in value:
                return None
            before: JSON = {"required": [self.name]}
            rest = [name for name in value if name != self.name]
            if rest:
                node.kw["required"] = rest
            else:
                del node.kw["required"]
        else:
            if not isinstance(value, dict) or self.name not in value:
                return None
            before = {"dependentRequired": {self.name: copy.deepcopy(value[self.name])}}
            del value[self.name]
            if not value:
                del node.kw["dependentRequired"]
        cid = ConstraintId(self.nid, self.keyword, self.name)
        return Change("delete-entry", self.nid, before, REMOVED, (cid,), ())


@dataclass(frozen=True, eq=False, slots=True)
class _AddProperties(_Edit):
    """Add ``properties`` entries for keys a closed Record rejected (names already present are left alone)."""

    nid: int
    members: tuple[tuple[str, SchemaNode], ...]

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        if node.boolean is not None:
            return None
        properties = node.kw.get("properties")
        created = properties is None
        present = {} if properties is None else properties
        new = [(name, template) for name, template in self.members if name not in present]
        if not new:
            return None
        before = _fragment(node, ["additionalProperties"])
        before_ids = _ids(node, ["additionalProperties"])
        if properties is None:
            properties = node.kw["properties"] = {}
        for name, template in new:
            properties[name] = copy.deepcopy(template)
        after = {
            **_fragment(node, ["additionalProperties"]),
            "properties": {name: properties[name].to_json() for name, _ in new},
        }
        after_keys = ["additionalProperties", "properties"] if created else ["additionalProperties"]
        return Change("add-properties", self.nid, before, after, before_ids, _ids(node, after_keys))


@dataclass(frozen=True, eq=False, slots=True)
class _AppendBranch(_Edit):
    """Append a copy of ``template`` to the node's ``anyOf`` or ``oneOf``."""

    nid: int
    keyword: str
    template: SchemaNode

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        branches = None if node.boolean is not None else node.kw.get(self.keyword)
        if not isinstance(branches, list):
            return None
        before = _fragment(node, [self.keyword])
        before_ids = _ids(node, [self.keyword])
        branches.append(copy.deepcopy(self.template))
        after = _fragment(node, [self.keyword])
        return Change("append-branch", self.nid, before, after, before_ids, _ids(node, [self.keyword]))


@dataclass(frozen=True, eq=False, slots=True)
class _Convert(_Edit):
    """Turn the node's ``oneOf`` into ``anyOf`` (into an ``allOf`` entry ``{"anyOf": …}`` with nid ``wrapper`` when ``anyOf`` exists)."""

    nid: int
    wrapper: int

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        if node.boolean is not None or "oneOf" not in node.kw:
            return None
        if "anyOf" not in node.kw:
            before = _fragment(node, ["oneOf"])
            before_ids = _ids(node, ["oneOf"])
            node.kw["anyOf"] = node.kw.pop("oneOf")
            _rewrite_refs(model, [*tokens, "oneOf"], [*tokens, "anyOf"])
            return Change("convert", self.nid, before, _fragment(node, ["anyOf"]), before_ids, _ids(node, ["anyOf"]))
        # Beside anyOf, the branches go into a new allOf entry, which creates the owner's allOf
        # Constraint or modifies an existing one; either way the Change lists it.
        before = _fragment(node, ["allOf", "oneOf"])
        before_ids = _ids(node, ["allOf", "oneOf"])
        wrapper = SchemaNode(kw={"anyOf": node.kw.pop("oneOf")}, nid=self.wrapper)
        all_of = node.kw.get("allOf")
        if not isinstance(all_of, list):
            all_of = node.kw["allOf"] = []
        all_of.append(wrapper)
        _rewrite_refs(model, [*tokens, "oneOf"], [*tokens, "allOf", str(len(all_of) - 1), "anyOf"])
        after_ids = tuple(sorted(dict.fromkeys([*_ids(node, ["allOf"]), *_ids(wrapper, ["anyOf"])])))
        return Change("convert", self.nid, before, _fragment(node, ["allOf"]), before_ids, after_ids)


@dataclass(frozen=True, eq=False, slots=True)
class _ReplaceFalse(_Edit):
    """Turn a boolean ``false`` node into a copy of ``template`` (``True`` for the ``true`` schema), keeping its nid."""

    nid: int
    template: SchemaNode | bool

    def apply(self, model: SchemaModel, tokens: list[str], node: SchemaNode) -> Change | None:
        if node.boolean is not False:
            return None
        if self.template is True:
            node.boolean = True
            node.kw = {}
        else:
            assert isinstance(self.template, SchemaNode)
            node.boolean = self.template.boolean
            node.kw = copy.deepcopy(self.template.kw)
        return Change("replace", self.nid, False, node.to_json(), (), _ids(node, sorted_codepoint(node.kw)))


# ---------------------------------------------------------------------------
# Relaxations
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class Relaxation:
    """One candidate relaxation of a Failure (see the module docstring).

    ``edits`` are the candidate's own edits (the first ``n_primary``) followed
    by its monotonicity guards. ``order`` is its position in the documented
    candidate order: ``(action rank, index)``.
    """

    failure: Failure
    action: Action
    label: str
    order: tuple[int, int]
    edits: tuple[_Edit, ...]
    n_primary: int

    @property
    def removes(self) -> bool:
        """True for the removal candidate (3.3, 3.7)."""
        return self.action == "remove"

    @property
    def guard_conversions(self) -> tuple[int, ...]:
        """The nids of the ``oneOf`` nodes the guards convert to ``anyOf``."""
        return tuple(edit.nid for edit in self.edits[self.n_primary :] if isinstance(edit, _Convert))

    @property
    def guard_removals(self) -> tuple[tuple[int, str], ...]:
        """The ``(nid, keyword)`` of each ``not`` or ``if`` the guards remove."""
        found: list[tuple[int, str]] = []
        for edit in self.edits[self.n_primary :]:
            if isinstance(edit, _Rewrite):
                found.append((edit.nid, "if" if "if" in edit.delete else "not"))
        return tuple(found)

    def apply(self, model: SchemaModel) -> tuple[Change, ...]:
        """Apply the edits to ``model`` in place and return their Changes (provenance is left to the caller)."""
        if not isinstance(model, SchemaModel):
            raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
        changes: list[Change] = []
        for position, edit in enumerate(self.edits):
            located = _locate(model).get(edit.nid)
            if located is None:
                continue  # an earlier edit of this candidate removed the node
            change = edit.apply(model, *located)
            if change is not None:
                changes.append(replace(change, guard=position >= self.n_primary))
        return tuple(changes)

    def applied(self, model: SchemaModel) -> tuple[SchemaModel, tuple[Change, ...]]:
        """A relaxed copy of ``model`` and the Changes; ``model`` is not modified."""
        relaxed = _copy_model(model)
        return relaxed, self.apply(relaxed)

    def preview(self, model: SchemaModel) -> SchemaNode:
        """The Failure's node after the relaxation, in a relaxed copy of ``model`` (for local MDL scoring).

        Raises ``KeyError`` when the node is gone (it sat inside a removed ``not`` or ``if``).
        """
        relaxed, _ = self.applied(model)
        return relaxed.index()[self.failure.nid]

    def candidate(self, *, node: SchemaNode | None = None, score: Score | None = None) -> Candidate:
        """The ``relaxation`` Candidate: Technique-less, outcome this Relaxation, ``order_key`` its order."""
        return Candidate(technique=None, outcome=self, label=self.label, order_key=self.order, node=node, score=score)


def _copy_model(model: SchemaModel) -> SchemaModel:
    return SchemaModel(
        root=copy.deepcopy(model.root),
        dialect_assumed=model.dialect_assumed,
        ignored=list(model.ignored),
        unsupported=list(model.unsupported),
        provenance=dict(model.provenance),
    )


def _locate(model: SchemaModel) -> dict[int, tuple[list[str], SchemaNode]]:
    """Every node by nid with its pointer tokens; raises ``ValueError`` on a repeated nid."""
    located: dict[int, tuple[list[str], SchemaNode]] = {}
    for pointer, node in model.walk():
        if node.nid in located:
            raise ValueError(f"nid {node.nid} appears twice in the model")
        located[node.nid] = (split_pointer(pointer), node)
    return located


# ---------------------------------------------------------------------------
# Monotonicity guards
# ---------------------------------------------------------------------------


def _reach(seeds: Iterable[int], adjacency: Mapping[int, list[int]]) -> set[int]:
    """The nids reachable from ``seeds`` (included) along ``adjacency``; a set for membership only."""
    stack = list(seeds)
    seen = set(stack)
    while stack:
        for nxt in adjacency.get(stack.pop(), ()):
            if nxt not in seen:
                seen.add(nxt)
                stack.append(nxt)
    return seen


@dataclass(frozen=True, slots=True)
class _Graph:
    """Evaluation edges ``(owner, keyword, child)`` in canonical order, and the nids reachable positively from the root."""

    edges: tuple[tuple[int, str, int], ...]
    positive: frozenset[int]


def _graph(model: SchemaModel) -> _Graph:
    edges: list[tuple[int, str, int]] = []
    for _, node in model.walk():
        for keyword, _entry, child in node.children():
            if keyword != "$defs":  # definitions are evaluated only through $ref
                edges.append((node.nid, keyword, child.nid))
        tokens = _ref_tokens(node)
        if tokens is not None:
            try:
                target = model.resolve(join_pointer("", *tokens))
            except (KeyError, ValueError):
                continue
            edges.append((node.nid, "$ref", target.nid))
    positive: dict[int, list[int]] = {}
    for owner, keyword, child in edges:
        if keyword not in _BLOCKED:
            positive.setdefault(owner, []).append(child)
    return _Graph(tuple(edges), frozenset(_reach([model.root.nid], positive)))


def _guard(graph: _Graph, edited: Iterable[int]) -> tuple[list[tuple[int, str]], list[int]]:
    """The ``not``/``if`` keywords to remove and the ``oneOf`` nodes to convert for edits at ``edited``."""
    targets = list(dict.fromkeys(edited))
    removed: dict[tuple[int, str], None] = {}
    while True:
        reverse: dict[int, list[int]] = {}
        for owner, keyword, child in graph.edges:
            if (owner, keyword) not in removed:
                reverse.setdefault(child, []).append(owner)
        reaching = _reach(targets, reverse)
        found = [
            (owner, keyword)
            for owner, keyword, child in graph.edges
            if keyword in _BLOCKED and (owner, keyword) not in removed
            if owner in graph.positive and child in reaching
        ]
        if not found:
            break
        for key in found:
            removed[key] = None
            if key[0] not in targets:
                targets.append(key[0])
    conversions = dict.fromkeys(
        owner
        for owner, keyword, child in graph.edges
        if keyword == "oneOf" and owner in graph.positive and child in reaching
    )
    return list(removed), list(conversions)


# ---------------------------------------------------------------------------
# Candidate generation
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class _Proposal:
    action: Action
    label: str
    index: int
    edits: tuple[_Edit, ...]


class _Context:
    """The model a call works on, its nodes by nid, and the lazily built Validator and evaluation graph."""

    __slots__ = ("_graph", "_validator", "model", "nodes", "tokens")

    def __init__(self, model: SchemaModel, validator: Validator | None) -> None:
        if not isinstance(model, SchemaModel):
            raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
        if validator is not None and not isinstance(validator, Validator):
            raise TypeError(f"validator must be a Validator or None, got {type(validator).__name__}")
        self.model = model
        located = _locate(model)
        self.nodes = {nid: node for nid, (_, node) in located.items()}
        self.tokens = {nid: tokens for nid, (tokens, _) in located.items()}
        self._validator = validator
        self._graph: _Graph | None = None

    @property
    def validator(self) -> Validator:
        if self._validator is None:
            self._validator = Validator()
        return self._validator

    @property
    def graph(self) -> _Graph:
        if self._graph is None:
            self._graph = _graph(self.model)
        return self._graph

    def node(self, failure: Failure) -> SchemaNode:
        """The Failure's node, checked to hold what the Failure names."""
        node = self.nodes.get(failure.nid)
        if node is None:
            raise KeyError(f"no node with nid {failure.nid} in the model")
        keyword = failure.keyword
        if keyword == FALSE_SCHEMA:
            if node.boolean is not False:
                raise ValueError(f"a {FALSE_SCHEMA!r} Failure names a boolean false node; nid {failure.nid} is not one")
            return node
        if node.boolean is not None or keyword not in node.kw:
            raise ValueError(f"node {failure.nid} has no {keyword!r} keyword")
        value = node.kw[keyword]
        if keyword == "required" and (not isinstance(value, list) or failure.item not in value):
            raise ValueError(f"node {failure.nid} does not require {failure.item!r}")
        if keyword == "dependentRequired" and (not isinstance(value, dict) or failure.item not in value):
            raise ValueError(f"node {failure.nid} has no dependentRequired entry {failure.item!r}")
        if keyword in _COMBINATORS and failure.branches and len(failure.branches) != len(value):
            raise ValueError(
                f"the Failure lists {len(failure.branches)} branches, but node {failure.nid} has {len(value)}"
            )
        return node


def _removal(failure: Failure) -> _Proposal:
    keyword, nid = failure.keyword, failure.nid
    if keyword in _ENTRY_KEYWORDS:
        assert failure.item is not None
        return _Proposal("remove", f"remove:{keyword}/{failure.item}", 0, (_DeleteEntry(nid, keyword, failure.item),))
    if keyword == FALSE_SCHEMA:
        return _Proposal("remove", "remove:false", 0, (_ReplaceFalse(nid, True),))
    return _Proposal("remove", f"remove:{keyword}", 0, (_Rewrite(nid, delete=(keyword,)),))


def _add_types(node: SchemaNode, failure: Failure) -> list[_Proposal]:
    listed = node.kw["type"]
    names = [listed] if isinstance(listed, str) else list(listed) if isinstance(listed, list) else []
    try:
        old = [JType.from_keyword(name) for name in names]
    except ValueError:
        return []
    new = type_names([*old, *(jtype_of(value) for value in failure.values)])
    if new == type_names(old):
        return []
    edit = _Rewrite(failure.nid, assign=(("type", new),))
    return [_Proposal("add-types", f"add-types:type={_label_json(new)}", 0, (edit,))]


def _add_values(node: SchemaNode, failure: Failure) -> list[_Proposal]:
    if failure.keyword == "enum":
        members = node.kw["enum"]
        if not isinstance(members, list):
            return []
        old = distinct_values(members)
        new = distinct_values([*members, *failure.values])
        if len(new) == len(old):
            return []
        edit = _Rewrite(failure.nid, assign=(("enum", new),))
        return [_Proposal("add-values", f"add-values:enum+{len(new) - len(old)}", 0, (edit,))]
    if "enum" in node.kw:
        return []  # const beside enum: the enum constrains the values on its own, so only removal is offered
    new = distinct_values([node.kw["const"], *failure.values])
    if len(new) == 1:
        return []
    edit = _Rewrite(failure.nid, delete=("const",), assign=(("enum", new),))
    return [_Proposal("add-values", f"add-values:const->enum+{len(new) - 1}", 0, (edit,))]


def _quantities(kind: QuantityKind, values: Iterable[JSON]) -> list[Any]:
    if kind == NUMBER:
        return [value for value in values if jtype_of(value) in (JType.INTEGER, JType.NUMBER)]
    if kind == STRING_LENGTH:
        return [len(value) for value in values if isinstance(value, str)]
    if kind == ITEM_COUNT:
        return [len(value) for value in values if isinstance(value, (list, tuple))]
    return [len(value) for value in values if isinstance(value, dict)]


def _loosen(node: SchemaNode, failure: Failure) -> list[_Proposal]:
    keyword = failure.keyword
    kind, side, exclusive = BOUND_SPECS[keyword]
    current = node.kw[keyword]
    if isinstance(current, bool) or not isinstance(current, (int, float)):
        return []
    quantities = _quantities(kind, failure.values)
    if not quantities:
        return []
    q = QuantitySet.of(kind, quantities)
    rungs = [rung.value for rung in bound_ladder(q, side) if rung.value is not None]
    if side == MAXIMUM:
        looser = [r for r in rungs if r > current and (not exclusive or r > q.high)]
        chosen = min(looser) if looser else None
    else:
        looser = [r for r in rungs if r < current and (not exclusive or r < q.low)]
        chosen = max(looser) if looser else None
    if chosen is None:
        return []
    edit = _Rewrite(failure.nid, assign=((keyword, chosen),))
    return [_Proposal("loosen-bound", f"loosen-bound:{keyword}={_label_json(chosen)}", 0, (edit,))]


def _runs(s: str) -> list[tuple[CharClass | str, int]]:
    """The maximal runs of one FlashProfile atom in ``s``: ``[0-9]``, ``[a-z]``, ``[A-Z]``, or one literal."""
    runs: list[tuple[CharClass | str, int]] = []
    for ch in s:
        atom: CharClass | str = CharClass.base_of(ch) or ch
        if runs and runs[-1][0] == atom:
            runs[-1] = (atom, runs[-1][1] + 1)
        else:
            runs.append((atom, 1))
    return runs


def _induce(strings: Sequence[str]) -> Pattern:
    """One alternative per atom sequence, with observed run-length ranges (see ``re-induce``)."""
    groups: dict[tuple[CharClass | str, ...], tuple[list[list[int]], list[bool]]] = {}
    for s in strings:
        runs = _runs(s)
        profile = tuple(atom for atom, _ in runs)
        latent = latent_type_of(s)
        numeric = latent is not None and latent.numeric
        entry = groups.get(profile)
        if entry is None:
            groups[profile] = ([[n, n] for _, n in runs], [numeric])
            continue
        ranges, flags = entry
        flags[0] = flags[0] and numeric
        for bounds, (_, n) in zip(ranges, runs):
            bounds[0] = min(bounds[0], n)
            bounds[1] = max(bounds[1], n)
    alternatives = []
    for profile, (ranges, flags) in groups.items():
        tokens = [
            Token(atom, PLUS if flags[0] and atom is CharClass.DIGIT else Quantifier(low, high))
            for atom, (low, high) in zip(profile, ranges)
        ]
        alternatives.append(Alternative(tuple(tokens)))
    return Pattern(tuple(alternatives))


def _shape(alternative: Alternative) -> tuple[CharClass | str, ...]:
    return tuple(token.atom for token in alternative.tokens)


def _hull(a: Alternative, b: Alternative) -> Alternative:
    """The alternative with ``a``'s atoms whose quantifiers cover both ``a``'s and ``b``'s (same atom sequence)."""
    tokens = []
    for x, y in zip(a.tokens, b.tokens):
        p, q = x.quantifier, y.quantifier
        high = None if p.high is None or q.high is None else max(p.high, q.high)
        tokens.append(Token(x.atom, Quantifier(min(p.low, q.low), high)))
    return Alternative(tuple(tokens))


def _widened_pattern(old: str, rejected: Sequence[str]) -> str | None:
    """A pattern accepting every string ``old`` accepts plus ``rejected`` (None when no literal can hold a character)."""
    try:
        induced = _induce(rejected)
    except ValueError:
        return None
    try:
        safe = Pattern.from_source(old)
    except ValueError:
        return f"(?:{old})|{induced.source}"
    alternatives = list(safe.alternatives)
    for alternative in induced.alternatives:
        shape = _shape(alternative)
        for index, existing in enumerate(alternatives):
            if _shape(existing) == shape:
                alternatives[index] = _hull(existing, alternative)
                break
        else:
            alternatives.append(alternative)
    return Pattern(tuple(alternatives)).source


def _reinduce(ctx: _Context, node: SchemaNode, failure: Failure) -> list[_Proposal]:
    old = node.kw["pattern"]
    if not isinstance(old, str):
        return []
    strings = sorted_codepoint(dict.fromkeys(value for value in failure.values if isinstance(value, str)))
    validator = ctx.validator
    try:
        rejected = [s for s in strings if not validator.matches_pattern(old, s)]
        if not rejected:
            return []
        source = _widened_pattern(old, rejected)
        if source is None or not all(validator.matches_pattern(source, s) for s in strings):
            return []
    except CompileError:
        return []
    edit = _Rewrite(failure.nid, assign=(("pattern", source),))
    return [_Proposal("re-induce", "re-induce:pattern", 0, (edit,))]


def _is_false(value: Any) -> bool:
    return isinstance(value, SchemaNode) and value.boolean is False


def _add_properties(ctx: _Context, node: SchemaNode, failure: Failure) -> list[_Proposal]:
    if not _is_false(node.kw["additionalProperties"]):
        return []
    properties = node.kw.get("properties") or {}
    patterns = list(node.kw.get("patternProperties") or {})
    extras: dict[str, list[JSON]] = {}
    for value in failure.values:
        if not isinstance(value, dict):
            continue
        for key, member in value.items():
            if key in properties:
                continue
            if key not in extras and any(ctx.validator.matches_pattern(p, key) for p in patterns):
                continue  # patternProperties covers the key, so additionalProperties does not apply to it
            extras.setdefault(key, []).append(member)
    if not extras:
        return []
    names = sorted_codepoint(extras)
    members = tuple((name, naive_merge_values(extras[name])) for name in names)
    edit = _AddProperties(failure.nid, members)
    return [_Proposal("add-properties", f"add-properties:+{len(names)}", 0, (edit,))]


def _replace_items(node: SchemaNode, failure: Failure) -> list[_Proposal]:
    if not _is_false(node.kw["items"]):
        return []
    prefix = node.kw.get("prefixItems")
    start = len(prefix) if isinstance(prefix, list) else 0
    extra = [element for value in failure.values if isinstance(value, (list, tuple)) for element in value[start:]]
    if not extra:
        return []
    edit = _Rewrite(failure.nid, assign=(("items", naive_merge_values(extra)),))
    return [_Proposal("replace-items", "replace-items:items", 0, (edit,))]


def _branch_proposals(ctx: _Context, failure: Failure) -> list[_Proposal]:
    keyword = failure.keyword
    convert: tuple[_Edit, ...] = (_Convert(failure.nid, new_nid()),) if keyword == "oneOf" else ()
    out: list[_Proposal] = []
    if convert and (not failure.branches or any(not inner for inner in failure.branches)):
        out.append(_Proposal("convert", "convert:oneOf->anyOf", 0, convert))
    for index, inner in enumerate(failure.branches):
        if not inner:
            continue  # the branch accepted the values
        edits: list[_Edit] = []
        for local in _merged(inner, ctx.tokens):
            edits.extend(_proposals(ctx, local)[0].edits)
        out.append(_Proposal("relax-branch", f"relax-branch:{keyword}/{index}", index, (*edits, *convert)))
    if failure.values:
        template = naive_merge_values(failure.values)
        out.append(
            _Proposal("add-branch", f"add-branch:{keyword}", 0, (_AppendBranch(failure.nid, keyword, template), *convert))
        )
    return out


def _proposals(ctx: _Context, failure: Failure) -> list[_Proposal]:
    """The candidates' own edits, in the documented order, removal last."""
    node = ctx.node(failure)
    keyword = failure.keyword
    out: list[_Proposal] = []
    if keyword == FALSE_SCHEMA:
        if failure.values:
            edit = _ReplaceFalse(failure.nid, naive_merge_values(failure.values))
            out.append(_Proposal("replace", "replace:false", 0, (edit,)))
    elif keyword == "type":
        out.extend(_add_types(node, failure))
    elif keyword in ("enum", "const"):
        out.extend(_add_values(node, failure))
    elif keyword in BOUND_SPECS:
        out.extend(_loosen(node, failure))
    elif keyword == "pattern":
        out.extend(_reinduce(ctx, node, failure))
    elif keyword == "additionalProperties":
        out.extend(_add_properties(ctx, node, failure))
    elif keyword == "items":
        out.extend(_replace_items(node, failure))
    elif keyword in _COMBINATORS:
        out.extend(_branch_proposals(ctx, failure))
    out.append(_removal(failure))
    return out


def _finish(ctx: _Context, failure: Failure, proposal: _Proposal) -> Relaxation:
    """The Relaxation of a proposal: its edits plus the monotonicity guards for the nodes they edit."""
    primary = proposal.edits
    removals, conversions = _guard(ctx.graph, (edit.nid for edit in primary))
    converted = {edit.nid for edit in primary if isinstance(edit, _Convert)}
    guards: list[_Edit] = [_Convert(nid, new_nid()) for nid in conversions if nid not in converted]
    guards.extend(_Rewrite(nid, delete=_BLOCKED[keyword]) for nid, keyword in removals)
    return Relaxation(
        failure=failure,
        action=proposal.action,
        label=proposal.label,
        order=(_ACTION_RANKS[proposal.action], proposal.index),
        edits=(*primary, *guards),
        n_primary=len(primary),
    )


def relaxations(model: SchemaModel, failure: Failure, *, validator: Validator | None = None) -> tuple[Relaxation, ...]:
    """The candidate relaxations of ``failure`` in ``model``, in the documented order, removal last.

    ``validator`` checks re-induced Patterns and ``patternProperties``
    matches (a default Validator when omitted). Raises ``KeyError`` when the
    Failure's node is not in the model and ``ValueError`` when the node does
    not hold what the Failure names.
    """
    if not isinstance(failure, Failure):
        raise TypeError(f"failure must be a Failure, got {type(failure).__name__}")
    ctx = _Context(model, validator)
    return tuple(_finish(ctx, failure, proposal) for proposal in _proposals(ctx, failure))


def removal(model: SchemaModel, failure: Failure) -> Relaxation:
    """The removal of the failing Constraint with its monotonicity guards (3.7); the last of :func:`relaxations`."""
    if not isinstance(failure, Failure):
        raise TypeError(f"failure must be a Failure, got {type(failure).__name__}")
    ctx = _Context(model, None)
    ctx.node(failure)
    return _finish(ctx, failure, _removal(failure))
