"""MDL_Scorer: model bits, data bits with exception costs, and local scoring (Requirement 19).

``DL(candidate) = M(node) + D(node, view)``: :meth:`MDLScorer.score` returns
a :class:`Score` with the model bits, the data bits (exception bits
included), the exception bits, the number of rejected Observations, and the
number of Observations scored. The encoding follows "Key Algorithms > MDL
encoding" in the design; the details the design leaves open are fixed below.

Model bits (Requirement 19.1)
    ``M(node) = LN0(#kw) + Σ_kw [UNI(|KW|) + M_kw]`` over the coded keywords
    of a keyword schema (see "Key Algorithms > MDL encoding" in the design).
    The vocabulary KW is :data:`CODED_KEYWORDS`, the keywords the
    Schema_Loader supports (Requirement 2.4) plus ``x-latent-type`` and
    ``x-array-pattern``, and one shared slot for every other ``x-``
    annotation, so ``|KW|`` is :data:`KEYWORD_VOCABULARY_SIZE` (37).
    ``M_kw`` is:

    - ``type``: ``SUB(k, 7)`` for k distinct type names.
    - ``properties``: ``LN0(p) + Σ (STR(name) + M(sub))``.
    - ``required``: ``SUB(r, p)``, p the number of ``properties`` entries.
    - ``additionalProperties`` and ``items``: ``1 + [M(sub) if sub is a
      keyword schema]``.
    - ``patternProperties``: ``LN0(q) + Σ (M_re(key) + M(sub))``.
    - ``propertyNames``, ``contains``, ``not``, ``if``, ``then``, ``else``:
      ``M(sub)``.
    - ``prefixItems``: ``LN0(len) + Σ M(sub)``.
    - ``anyOf``, ``oneOf``, ``allOf``: ``LN0(b) + Σ M(branch)``.
    - ``enum``: ``LN0(m) + Σ U(v)`` over the m distinct members (JSON Schema
      equality, so ``1`` and ``1.0`` are one member).
    - ``const``: ``U(v)``. ``format``: ``UNI(10)``. ``pattern``: ``M_re``.
    - the ten bounds (:data:`BOUND_KEYWORDS`): ``BOUND(v)``.
    - ``uniqueItems``: 0.
    - ``dependentRequired``: ``LN0(e) + Σ [UNI(p) + SUB(k, p − 1)]``.
    - ``$ref``: ``UNI(D)``. ``$defs``: ``LN0(#defs) + Σ M(def)``, where the
      names are derived and cost 0 bits.
    - ``x-latent-type``: ``UNI(6)``. ``x-array-pattern``: ``U(value)``.
    - every other ``x-`` annotation: :data:`ANNOTATION_BITS` (0).

    ``if``/``then``/``else`` and ``allOf`` are not in the design's table;
    they are coded like ``not`` and ``anyOf``. ``x-array-pattern`` stands
    for the grammar size: its value is the grammar (task 43.1), and until
    that task fixes a grammar AST the universal value code of the stored
    grammar is its size. The other ``x-`` annotations (``x-key``,
    ``x-references``, ``x-functional-dependency``, and ``x-`` keywords
    loaded from a schema) pay their keyword slot plus a constant 0 bits for
    the value: the value is not coded, so an annotation never changes a
    comparison between candidates that carry it.

    Keywords that are not Constraints (:data:`UNCODED_KEYWORDS`: ``$schema``,
    ``$id``, ``$comment``, the retained annotations such as ``title`` and
    ``description``, and ``x-provenance``) are not coded and do not count in
    ``#kw``, so annotations and inline provenance never change model bits.
    Any other keyword (``multipleOf``, ``unevaluatedProperties``, ...,
    which the Schema_Loader never loads) raises ``ValueError``.

Pattern bits
    :func:`M_re` codes a safe-regex AST (``values.patterns.Pattern``):
    ``LN0(#alternatives)``, then per alternative ``LN0(#tokens)``, then per
    token ``UNI(2)`` for its kind (literal or class), ``8·bytes`` for a
    literal code point (its UTF-8 length; one code point needs no length
    prefix) or ``UNI(5)`` for a class, and the quantifier: a 1-bit bounded
    flag, ``LN0(min)``, and ``LN0(max − min)`` when bounded. The flag is
    always paid, so the quantifier code is prefix-free. :func:`regex_bits`
    costs a ``pattern`` value or a ``patternProperties`` key: ``M_re`` of
    the Pattern it is the printed form of (``Pattern.from_source``), or, for
    any other regex (only loaded schemas hold them), ``STR(source)``, the
    literal cost of the string.

Boolean schemas
    A boolean schema costs the same as its keyword equivalent: ``true`` is
    ``M({})`` = ``LN0(0)`` and ``false`` is ``M({"not": {}})`` =
    ``LN0(1) + UNI(|KW|) + LN0(0)``. Under ``items`` and
    ``additionalProperties`` the 1-bit flag already says "boolean", so a
    boolean there adds nothing to the flag, as in the table.

Names outside ``properties``
    ``required`` and ``dependentRequired`` code names as subsets of the p
    names of the same node's ``properties``. A loaded schema may also name
    other properties; each such name costs ``STR(name)``, plus ``LN0(x)``
    for their number x when there is at least one, and a
    ``dependentRequired`` antecedent outside ``properties`` costs
    ``STR(name)`` instead of ``UNI(p)`` and leaves all p names as possible
    consequents. A name repeated in a list counts once, and an antecedent
    listed among its own consequents counts as a name outside the subset.
    The Inferrer only names properties of the node, where these codes equal
    the table's.

``$ref`` and D
    ``$ref`` picks one of the D ``$defs`` entries of the model root. D is
    the ``n_defs`` given to :class:`MDLScorer` (or :func:`model_bits`);
    when none is given, D is the number of ``$defs`` entries of the scored
    node itself, which is right when a whole model is scored from its root,
    and 0 when it has none. Local scoring of a subschema of a model with
    definitions therefore passes the root's count.

Determinism
    Every term of ``M(node)`` (each keyword slot, length, literal, value
    code, and subschema term, flattened over the whole subtree) is summed
    once with ``math.fsum``, so the result does not depend on the order of
    keywords, of ``properties``, ``patternProperties``, ``$defs``, and
    ``dependentRequired`` entries, or of ``type``, ``enum``, and
    ``required`` lists. It is finite and non-negative. The walk is
    iterative, so deep nesting does not hit the recursion limit, and a node
    object reached twice (shared or cyclic structure) raises ``ValueError``,
    as ``SchemaNode.walk`` does.

Data bits (Requirements 19.1, 19.2)
    :meth:`MDLScorer.data_bits` codes the non-flagged rows of a view under a
    node, top-down: each node codes the rows of its own view and hands the
    rows of its children (member values, elements, branch rows) to their
    subschemas. At every node the rows split into accepted rows A and
    rejected rows R:

    - Exceptions: ``EXC = 1 + [CAT(|A|, |R|) + Σ_{r∈R} U(v_r) if R ≠ ∅]``.
      The 1-bit flag is paid at every node that codes at least one row.
      ``U(v_r)`` codes the whole subtree of a rejected row, which is not
      coded further, so every rejected Observation costs at least
      ``UNI(7) > 0`` and finitely many bits. These terms are the
      ``exception_bits``; they are also part of the data bits.
    - Type choice: ``CAT`` of the accepted rows' counts over the distinct
      ``type`` names (an integral number counts as ``integer`` when that is
      listed and as ``number`` otherwise), or over the seven JTypes when
      ``type`` is absent.
    - ``enum`` / ``const``: ``CAT`` of the member counts, with every distinct
      member a category; ``const`` costs 0. The member determines the whole
      value, so no type choice, no type-specific code, and no descendant is
      coded for those rows.
    - Integers: with [lo, hi] the integers the bounds allow (inclusive
      bounds rounded inward, exclusive bounds shifted by one),
      ``UNI(hi − lo + 1)`` with both sides, ``LN0(v − lo)`` or
      ``LN0(hi − v)`` with one, and ``LZ(v)`` with none.
    - Numbers (rows in the ``number`` category, where an integral number
      has d = 0): ``LN0(d)`` for the d decimal places of the shortest
      decimal ``m·10^−d`` (``codes.decimal_parts``), plus the integer code
      of m on the grid of multiples of ``10^−d`` that the bounds allow. With
      both bounds that is ``UNI`` of the number of grid points, which is the
      design's ``(max − min)·10^d + 1`` when the bounds lie on the grid, and
      without bounds it is ``LZ(m)``. A single bound uses the one-sided
      ``LN0`` of the integer code, so a lone bound can pay for itself on
      decimals as it does on integers. Bounds are read as decimals the same
      way as values (``minimum: 0.1`` is the grid point 1 at d = 1, not the
      double's binary value), so every value a bound admits lies on the
      grid it allows.
    - Strings: the length in code points (see Counts), then the characters.
      With a safe ``pattern`` (``Pattern.from_source``): ``UNI(b)`` for the
      alternative, and for each token of the matcher's unique decomposition
      ``log2 |class|`` per character (0 for a literal) plus the repetition
      count, ``UNI(high − low + 1)``, or ``LN0(count − low)`` for an
      unbounded token. Otherwise with a ``format`` of
      :data:`FORMAT_ALPHABET_SIZES`: ``len · log2 |Σ_f|``. Otherwise
      ``8 · utf8len``. The alphabet sizes are encoding constants, not
      decision cut-offs.
    - Booleans: ``CAT(#true, #false)``. Null: 0 bits.
    - Counts: string lengths, array lengths (``minItems``/``maxItems``), and
      Map key counts (``minProperties``/``maxProperties``) cannot be
      negative, so a missing minimum is 0: with a maximum the code is
      ``UNI(max − min + 1)``, with only a minimum ``LN0(n − min)``, and with
      neither ``LN0(n)``.
    - Arrays: the length, then element i under ``prefixItems[i]`` and every
      later element under ``items`` (the ``true`` schema when absent).
      ``items: false`` rejects arrays longer than the prefix.
    - Records (an object node that is not a Map): each property in
      ``properties`` and not in ``required`` has its presence coded as
      ``CAT(present, absent)``; ``required`` properties cost 0. Presence is
      coded per *presence unit*, a strongly connected component of the
      ``dependentRequired`` graph over the optional properties, so a
      Field_Group (a ``dependentRequired`` clique) is coded once, and only
      over the objects that hold none of the unit's antecedents outside it
      (the antecedents imply its presence). The other keys are extras: a
      closed Record (``additionalProperties: false``) rejects objects with
      an extra that no ``patternProperties`` pattern matches, and without
      ``patternProperties`` it codes no extras flag, since no accepted
      object can hold an extra (the design's closed candidate pays no
      ``CAT(n, 0)``); otherwise
      ``CAT(no extras, extras)`` over the objects, ``LN0(#extras)`` per
      object with extras, each extra key as ``STR(k)`` (under
      ``propertyNames`` when present, else under the first
      ``patternProperties`` Pattern matching it, as a ``pattern`` string
      code), and each extra value under the first matching
      ``patternProperties`` subschema in code-point order of the patterns,
      else under ``additionalProperties``, else as ``U(v)``.
    - Maps (an object node without ``properties`` that has a keyword-schema
      ``additionalProperties``, ``patternProperties``, or
      ``propertyNames``): per object the key count (Counts), every key as
      ``STR(k)`` or, under ``propertyNames``, by the string code of that
      subschema (no type choice), and every value under its subschema as
      for Record extras.
    - ``anyOf`` / ``oneOf``: ``CAT`` of the branch counts over the
      assignment of the accepted rows to branches, plus each branch's data
      bits on its rows (the sibling keywords only reject). A row goes to the
      branch whose view in the caller's ``branch_views`` for the node holds
      it (the analyzer's Variants; the branch then codes the row, with any
      exceptions inside it), and otherwise to the first branch that accepts
      it. A row that no branch takes is rejected at the node, and so is a
      ``oneOf`` row that two or more branches accept. With both keywords,
      ``anyOf`` codes and ``oneOf`` only rejects.
    - Keywords that only reject: ``not``, ``allOf``, ``if``/``then``/
      ``else``, ``contains``, ``uniqueItems``, ``required``,
      ``dependentRequired``, ``minProperties``/``maxProperties`` of a
      Record, ``propertyNames`` (as a check), and ``$ref`` next to other
      keywords. A ``$ref`` alone codes the rows under its target, resolved
      as a JSON Pointer fragment against the scorer's ``root`` (by default
      the scored node). Annotations, ``$defs``, and ``x-`` keywords code
      nothing.

    "Accepts" for branches, ``not``, ``allOf``, ``if``, ``contains``, and
    ``$ref`` checks means the whole subtree validates: the scorer evaluates
    the subschema over the view's columns, with the Validator deciding
    ``format`` and non-safe ``pattern`` per distinct string. Container
    values are rebuilt from the store only for ``enum``/``const`` members
    and ``uniqueItems`` comparisons of arrays and objects.

Local scoring and candidate encoding (Requirement 19.3)
    The scorer runs no inference: it codes exactly the node it is given, so
    every candidate at a Decision_Point is scored against the same view.
    A candidate represents the descendants it implies by their Naive_Merge
    subschemas, ``pipeline.naive_merge.naive_merge_node`` of the child views
    (``view.child_key(name)``, ``view.child_map()``, ``view.elements(i)``,
    a Variant's view), which keeps scoring O(rows) while still rewarding
    child type homogeneity. Outcomes are written as schema keywords:

    - Record: ``properties`` (and ``required``); closed with
      ``additionalProperties: false``, open without it.
    - Map: the value schema as ``additionalProperties`` without
      ``properties``, optionally with ``propertyNames`` and
      ``minProperties``/``maxProperties``.
    - Hybrid Record/Map (4.5): ``properties`` for the Record fields plus the
      Map part as ``additionalProperties`` or ``patternProperties``.
    - Field_Groups: ``dependentRequired`` cliques (each member requires every
      other), which Requirement 6 emits anyway.
    - Tuple / List: ``prefixItems`` plus ``items: false`` / one ``items``.
    - Variants and Merge-or-Split: an ``anyOf`` (or ``oneOf``) of the
      branch subschemas, with ``branch_views={node.nid: views}`` giving the
      rows of each branch.
    - Co_Occurrence_Rules: the Variant's Record plus one
      ``dependentRequired`` entry. A rule that every object satisfies
      changes only the model bits and the Record's own presence terms
      (:meth:`MDLScorer.presence_bits`), so many rules are scored against
      one full scoring of the Record.

Observations scored, flags, and determinism
    ``n_scored`` is the number of non-flagged Observations in the view and
    their subtrees, the same for every candidate scored on the view;
    ``n_rejected`` counts the rejected Observations at every depth, each
    paying an exception, so ``n_rejected <= n_scored``. Views exclude
    flagged documents and items (``core.stats.FlagSet``), and so does the
    scorer, except that rebuilt container values hold their flagged
    descendants as the document does. Every cost is computed once per
    distinct value, length, or key, multiplied by its integer count, and
    all terms are summed once with ``math.fsum``, so the result depends
    only on the multiset of Observations: it is bit-identical for every
    document order, chunking, and worker count (Requirement 19.6).
"""

from __future__ import annotations

import math
import operator
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from fractions import Fraction
from types import MappingProxyType
from typing import Any, Final
from urllib.parse import unquote

import numpy as np

from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON, CanonicalKey, JType, canonical_key, distinct_values
from jsinfer.core.observations import ObservationStore, ShapePath, isin_sorted, ranges_to_rows
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP
from jsinfer.core.schema_model import NON_CONSTRAINT_KEYWORDS, SchemaNode
from jsinfer.core.stats import ViewStats
from jsinfer.core.views import View
from jsinfer.io.load import SUPPORTED_KEYWORDS
from jsinfer.io.validator import Validator
from jsinfer.mdl.codes import BOUND, CAT, LN0, LZ, NUM, STR, SUB, UNI, U, decimal_parts, utf8_length
from jsinfer.values.patterns import CharClass, Pattern

__all__ = [
    "ANNOTATION_BITS",
    "BOUND_KEYWORDS",
    "CODED_KEYWORDS",
    "EXCEPTION_FLAG_BITS",
    "FORMAT_ALPHABET_SIZES",
    "FORMAT_COUNT",
    "KEYWORD_VOCABULARY_SIZE",
    "LATENT_TYPE_COUNT",
    "UNCODED_KEYWORDS",
    "M_re",
    "MDLScorer",
    "Score",
    "model_bits",
    "regex_bits",
]

# Inferrer annotations with their own value code.
_LATENT_TYPE_KEYWORD: Final = "x-latent-type"
_ARRAY_PATTERN_KEYWORD: Final = "x-array-pattern"
_ANNOTATION_PREFIX: Final = "x-"

# The keywords M codes by name: the supported keywords and the two Inferrer
# annotations with their own value code.
CODED_KEYWORDS: Final = frozenset(SUPPORTED_KEYWORDS | {_LATENT_TYPE_KEYWORD, _ARRAY_PATTERN_KEYWORD})

# |KW|: the coded keywords plus one slot shared by every other x- annotation.
KEYWORD_VOCABULARY_SIZE: Final = len(CODED_KEYWORDS) + 1

# Keywords that are not Constraints and cost nothing ($defs is not a
# Constraint but holds subschemas, so it is coded).
UNCODED_KEYWORDS: Final = NON_CONSTRAINT_KEYWORDS - {"$defs"}

# The value cost of an x- annotation other than x-latent-type and x-array-pattern.
ANNOTATION_BITS: Final = 0.0

# Alphabet sizes of the constant-cost keyword values: the Formats the
# Format detector proposes, and the six Latent_Types (integer, decimal
# number, boolean, date, date-time, time).
FORMAT_COUNT: Final = 10
LATENT_TYPE_COUNT: Final = 6

BOUND_KEYWORDS: Final = frozenset(
    {
        "exclusiveMaximum",
        "exclusiveMinimum",
        "maxItems",
        "maxLength",
        "maxProperties",
        "maximum",
        "minItems",
        "minLength",
        "minProperties",
        "minimum",
    }
)

_TYPE_NAMES: Final = frozenset(jtype.keyword for jtype in JType)
_KEYWORD_SLOT: Final = UNI(KEYWORD_VOCABULARY_SIZE)
_SCHEMA_FLAG: Final = 1.0  # items / additionalProperties: boolean or keyword schema

# Boolean schemas cost what their keyword equivalents cost: true is {} and false is {"not": {}}.
_TRUE_TERMS: Final = (LN0(0),)
_FALSE_TERMS: Final = (LN0(1), _KEYWORD_SLOT, LN0(0))

# M_re constants: two token kinds (literal, class), five classes, and the bounded flag.
_TOKEN_KIND_BITS: Final = UNI(2)
_CLASS_BITS: Final = UNI(len(CharClass))
_BOUNDED_FLAG_BITS: Final = 1.0
_BITS_PER_BYTE: Final = 8.0


# ---------------------------------------------------------------------------
# Pattern bits
# ---------------------------------------------------------------------------


def _pattern_terms(pattern: Pattern, terms: list[float]) -> None:
    alternatives = pattern.alternatives
    terms.append(LN0(len(alternatives)))
    for alternative in alternatives:
        tokens = alternative.tokens
        terms.append(LN0(len(tokens)))
        for token in tokens:
            terms.append(_TOKEN_KIND_BITS)
            atom = token.atom
            if isinstance(atom, CharClass):
                terms.append(_CLASS_BITS)
            else:
                terms.append(_BITS_PER_BYTE * utf8_length(atom))
            quantifier = token.quantifier
            terms.append(_BOUNDED_FLAG_BITS)
            terms.append(LN0(quantifier.low))
            if quantifier.high is not None:
                terms.append(LN0(quantifier.high - quantifier.low))


def M_re(pattern: Pattern) -> float:
    """Model bits of a safe-regex AST (see "Pattern bits" in the module docstring)."""
    if not isinstance(pattern, Pattern):
        raise TypeError(f"M_re codes a Pattern, got {type(pattern).__name__}")
    terms: list[float] = []
    _pattern_terms(pattern, terms)
    return math.fsum(terms)


def regex_bits(source: str) -> float:
    """Model bits of a regex string: ``M_re`` of its safe Pattern, else ``STR(source)``."""
    if not isinstance(source, str):
        raise TypeError(f"a regex is a str, got {type(source).__name__}")
    try:
        pattern = Pattern.from_source(source)
    except ValueError:
        return STR(source)
    return M_re(pattern)


# ---------------------------------------------------------------------------
# Model bits
# ---------------------------------------------------------------------------


def _count_of(value: Any, what: str) -> int:
    if isinstance(value, bool):
        raise TypeError(f"{what} must be an int, not a boolean")
    try:
        count = operator.index(value)
    except TypeError:
        raise TypeError(f"{what} must be an int, got {type(value).__name__}") from None
    if count < 0:
        raise ValueError(f"{what} must be non-negative, got {count}")
    return count


def _defs_of(node: SchemaNode) -> int:
    """The number of ``$defs`` entries of a keyword schema (0 when it has none)."""
    if node.boolean is not None:
        return 0
    defs = node.kw.get("$defs")
    return len(defs) if isinstance(defs, dict) else 0


def _distinct_names(value: Any, keyword: str) -> list[str]:
    """The distinct names of a list of strings, first occurrence first."""
    if not isinstance(value, (list, tuple)):
        raise TypeError(f"{keyword!r} must be an array of strings, got {type(value).__name__}")
    for name in value:
        if not isinstance(name, str):
            raise TypeError(f"{keyword!r} entries must be strings, got {type(name).__name__}")
    return list(dict.fromkeys(value))


class _ModelCoder:
    """One model-bits walk: the flat list of terms and the subschemas still to visit."""

    __slots__ = ("_n_defs", "_pending", "_seen", "terms")

    def __init__(self, n_defs: int) -> None:
        self._n_defs = n_defs
        self.terms: list[float] = []
        self._pending: list[SchemaNode] = []
        self._seen: set[int] = set()  # id()s, for membership only; never iterated

    def run(self, root: SchemaNode) -> float:
        self._push(root, "the scored node")
        while self._pending:
            self._node(self._pending.pop())
        return math.fsum(self.terms)

    def _push(self, node: Any, where: str) -> None:
        if not isinstance(node, SchemaNode):
            raise TypeError(f"{where} must be a SchemaNode, got {type(node).__name__}")
        marker = id(node)
        if marker in self._seen:
            raise ValueError(f"node nid {node.nid} is reached twice (again under {where}); a Schema_Model is a tree")
        self._seen.add(marker)
        self._pending.append(node)

    # -- nodes ---------------------------------------------------------------

    def _node(self, node: SchemaNode) -> None:
        terms = self.terms
        if node.boolean is not None:
            if not isinstance(node.boolean, bool):
                raise TypeError(f"boolean must be True, False, or None, got {type(node.boolean).__name__}")
            if node.kw:
                raise ValueError(f"boolean schema nid {node.nid} has keywords")
            terms.extend(_TRUE_TERMS if node.boolean else _FALSE_TERMS)
            return
        kw = node.kw
        if not isinstance(kw, dict):
            raise TypeError(f"kw must be a dict, got {type(kw).__name__}")
        coded: list[str] = []
        for keyword in kw:
            if not isinstance(keyword, str):
                raise TypeError(f"schema keywords must be str, got {type(keyword).__name__}")
            if keyword not in UNCODED_KEYWORDS:
                coded.append(keyword)
        terms.append(LN0(len(coded)))
        for keyword in coded:
            terms.append(_KEYWORD_SLOT)
            self._keyword(node, keyword, kw[keyword])

    def _keyword(self, node: SchemaNode, keyword: str, value: Any) -> None:
        handler = _HANDLERS.get(keyword)
        if handler is not None:
            handler(self, node, keyword, value)
        elif keyword in BOUND_KEYWORDS:
            self.terms.append(BOUND(value))
        elif keyword.startswith(_ANNOTATION_PREFIX):
            self.terms.append(ANNOTATION_BITS)
        else:
            raise ValueError(f"model bits are not defined for keyword {keyword!r} (node nid {node.nid})")

    # -- subschema keywords --------------------------------------------------

    def _single(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self._push(value, repr(keyword))

    def _flagged(self, node: SchemaNode, keyword: str, value: Any) -> None:
        """``items`` and ``additionalProperties``: a 1-bit flag, plus the subschema when it has keywords."""
        if not isinstance(value, SchemaNode):
            raise TypeError(f"{keyword!r} must hold a SchemaNode, got {type(value).__name__}")
        self.terms.append(_SCHEMA_FLAG)
        if value.boolean is None:
            self._push(value, repr(keyword))
        elif not isinstance(value.boolean, bool) or value.kw:
            raise ValueError(f"{keyword!r} holds a malformed boolean schema (nid {value.nid})")

    def _list(self, node: SchemaNode, keyword: str, value: Any) -> None:
        if not isinstance(value, (list, tuple)):
            raise TypeError(f"{keyword!r} must hold a list of SchemaNodes, got {type(value).__name__}")
        self.terms.append(LN0(len(value)))
        for index, child in enumerate(value):
            self._push(child, f"{keyword!r} entry {index}")

    def _map(self, node: SchemaNode, keyword: str, value: Any) -> dict[str, Any]:
        if not isinstance(value, dict):
            raise TypeError(f"{keyword!r} must hold a dict of SchemaNodes, got {type(value).__name__}")
        self.terms.append(LN0(len(value)))
        for name, child in value.items():
            if not isinstance(name, str):
                raise TypeError(f"{keyword!r} names must be str, got {type(name).__name__}")
            self._push(child, f"{keyword!r} entry {name!r}")
        return value

    def _properties(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self.terms.extend(STR(name) for name in self._map(node, keyword, value))

    def _pattern_properties(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self.terms.extend(regex_bits(key) for key in self._map(node, keyword, value))

    def _defs(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self._map(node, keyword, value)  # definition names are derived: 0 bits

    # -- value keywords ------------------------------------------------------

    def _type(self, node: SchemaNode, keyword: str, value: Any) -> None:
        names = _distinct_names([value] if isinstance(value, str) else value, keyword)
        for name in names:
            if name not in _TYPE_NAMES:
                raise ValueError(f"{name!r} is not a JSON Schema type (node nid {node.nid})")
        self.terms.append(SUB(len(names), len(_TYPE_NAMES)))

    def _enum(self, node: SchemaNode, keyword: str, value: Any) -> None:
        if not isinstance(value, (list, tuple)):
            raise TypeError(f"'enum' must be an array, got {type(value).__name__}")
        members = distinct_values(value)
        self.terms.append(LN0(len(members)))
        self.terms.extend(U(member) for member in members)

    def _const(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self.terms.append(U(value))

    def _format(self, node: SchemaNode, keyword: str, value: Any) -> None:
        if not isinstance(value, str):
            raise TypeError(f"'format' must be a string, got {type(value).__name__}")
        self.terms.append(UNI(FORMAT_COUNT))

    def _pattern(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self.terms.append(regex_bits(value))

    def _unique_items(self, node: SchemaNode, keyword: str, value: Any) -> None:
        if not isinstance(value, bool):
            raise TypeError(f"'uniqueItems' must be a boolean, got {type(value).__name__}")

    def _ref(self, node: SchemaNode, keyword: str, value: Any) -> None:
        if not isinstance(value, str):
            raise TypeError(f"'$ref' must be a string, got {type(value).__name__}")
        self.terms.append(UNI(self._n_defs))

    def _latent_type(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self.terms.append(UNI(LATENT_TYPE_COUNT))

    def _array_pattern(self, node: SchemaNode, keyword: str, value: Any) -> None:
        self.terms.append(U(value))

    # -- name sets over properties --------------------------------------------

    def _subset(self, names: list[str], universe: Mapping[str, Any], size: int, excluded: str | None) -> None:
        """``SUB(k, size)`` for the names in ``universe`` (but not ``excluded``), literals for the rest."""
        inside = 0
        outside: list[str] = []
        for name in names:
            if name in universe and name != excluded:
                inside += 1
            else:
                outside.append(name)
        self.terms.append(SUB(inside, size))
        if outside:
            self.terms.append(LN0(len(outside)))
            self.terms.extend(STR(name) for name in outside)

    @staticmethod
    def _property_names(node: SchemaNode) -> Mapping[str, Any]:
        properties = node.kw.get("properties")
        return properties if isinstance(properties, dict) else {}

    def _required(self, node: SchemaNode, keyword: str, value: Any) -> None:
        properties = self._property_names(node)
        self._subset(_distinct_names(value, keyword), properties, len(properties), None)

    def _dependent_required(self, node: SchemaNode, keyword: str, value: Any) -> None:
        if not isinstance(value, dict):
            raise TypeError(f"'dependentRequired' must be an object, got {type(value).__name__}")
        properties = self._property_names(node)
        p = len(properties)
        self.terms.append(LN0(len(value)))
        for antecedent, consequents in value.items():
            if not isinstance(antecedent, str):
                raise TypeError(f"'dependentRequired' keys must be str, got {type(antecedent).__name__}")
            if antecedent in properties:
                self.terms.append(UNI(p))
                size = p - 1
            else:
                self.terms.append(STR(antecedent))
                size = p
            self._subset(_distinct_names(consequents, keyword), properties, size, antecedent)


_Handler = Callable[[_ModelCoder, SchemaNode, str, Any], None]

_HANDLERS: Final[Mapping[str, _Handler]] = MappingProxyType(
    {
        "$defs": _ModelCoder._defs,
        "$ref": _ModelCoder._ref,
        "additionalProperties": _ModelCoder._flagged,
        "allOf": _ModelCoder._list,
        "anyOf": _ModelCoder._list,
        "const": _ModelCoder._const,
        "contains": _ModelCoder._single,
        "dependentRequired": _ModelCoder._dependent_required,
        "else": _ModelCoder._single,
        "enum": _ModelCoder._enum,
        "format": _ModelCoder._format,
        "if": _ModelCoder._single,
        "items": _ModelCoder._flagged,
        "not": _ModelCoder._single,
        "oneOf": _ModelCoder._list,
        "pattern": _ModelCoder._pattern,
        "patternProperties": _ModelCoder._pattern_properties,
        "prefixItems": _ModelCoder._list,
        "properties": _ModelCoder._properties,
        "propertyNames": _ModelCoder._single,
        "required": _ModelCoder._required,
        "then": _ModelCoder._single,
        "type": _ModelCoder._type,
        "uniqueItems": _ModelCoder._unique_items,
        _ARRAY_PATTERN_KEYWORD: _ModelCoder._array_pattern,
        _LATENT_TYPE_KEYWORD: _ModelCoder._latent_type,
    }
)


def model_bits(node: SchemaNode, *, n_defs: int | None = None) -> float:
    """``M(node)``: the model bits of a schema subtree (see the module docstring).

    ``n_defs`` is D, the number of ``$defs`` entries at the model root that
    ``$ref`` chooses from; by default the number of ``$defs`` entries of
    ``node`` itself.
    """
    if not isinstance(node, SchemaNode):
        raise TypeError(f"model_bits takes a SchemaNode, got {type(node).__name__}")
    defs = _defs_of(node) if n_defs is None else _count_of(n_defs, "n_defs")
    return _ModelCoder(defs).run(node)


# ---------------------------------------------------------------------------
# Data bits: constants and keyword parsing
# ---------------------------------------------------------------------------

# |Σ_f|, the characters a string of each Format is written with. These are
# encoding constants (the design's examples), not decision cut-offs.
FORMAT_ALPHABET_SIZES: Final[Mapping[str, int]] = MappingProxyType(
    {
        "date": 16,  # digits and - : T Z . + (RFC 3339)
        "date-time": 16,
        "time": 16,
        "duration": 18,  # digits and P Y M W D T H S
        "email": 64,
        "hostname": 64,
        "ipv4": 11,  # digits and .
        "ipv6": 18,  # hexadecimal digits and : .
        "uri": 64,
        "uuid": 17,  # hexadecimal digits and -
    }
)

# The flag "this node rejects rows" in EXC, paid at every node that codes a row.
EXCEPTION_FLAG_BITS: Final = 1.0

_FORMAT_LOG2: Final = MappingProxyType({name: math.log2(size) for name, size in FORMAT_ALPHABET_SIZES.items()})
_CLASS_LOG2: Final = MappingProxyType({cls: math.log2(cls.size) for cls in CharClass})
_TYPE_TAG_BITS: Final = UNI(len(JType))  # the type tag of U(v)

_NULL: Final = int(JType.NULL)
_BOOLEAN: Final = int(JType.BOOLEAN)
_INTEGER: Final = int(JType.INTEGER)
_NUMBER: Final = int(JType.NUMBER)
_STRING: Final = int(JType.STRING)
_ARRAY: Final = int(JType.ARRAY)
_OBJECT: Final = int(JType.OBJECT)

_EMPTY_IDS: Final = np.empty(0, dtype=np.int64)
_EMPTY_IDS.setflags(write=False)

# The true and false schemas, for absent subschemas and for items: false overflow.
_TRUE_NODE: Final = SchemaNode(boolean=True)
_FALSE_NODE: Final = SchemaNode(boolean=False)


def _length_of(value: Any, keyword: str) -> int:
    """A length or count keyword value: a non-negative integer (an integral float is allowed)."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{keyword!r} must be a non-negative integer, got {type(value).__name__}")
    if isinstance(value, float) and not value.is_integer():
        raise ValueError(f"{keyword!r} must be a non-negative integer, got {value!r}")
    length = int(value)
    if length < 0:
        raise ValueError(f"{keyword!r} must be a non-negative integer, got {value!r}")
    return length


def _bound_of(value: Any, keyword: str) -> int | float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{keyword!r} must be a number, got {type(value).__name__}")
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"{keyword!r} must be finite, got {value!r}")
        return float(value)
    return int(value)


def _subschema(value: Any, keyword: str) -> SchemaNode:
    if not isinstance(value, SchemaNode):
        raise TypeError(f"{keyword!r} must hold a SchemaNode, got {type(value).__name__}")
    return value


def _subschemas(value: Any, keyword: str) -> tuple[SchemaNode, ...]:
    if not isinstance(value, (list, tuple)):
        raise TypeError(f"{keyword!r} must hold a list of SchemaNodes, got {type(value).__name__}")
    return tuple(_subschema(child, keyword) for child in value)


def _keyword_schema(value: Any, keyword: str) -> tuple[SchemaNode | None, bool]:
    """``items`` / ``additionalProperties``: (the keyword schema or None for true, whether it is false)."""
    node = _subschema(value, keyword)
    if node.boolean is None:
        return node, False
    return None, node.boolean is False


@dataclass(eq=False, slots=True)
class _Spec:
    """The keywords of one node, parsed and checked once per scoring call.

    ``target`` is set for a node whose only assertion is ``$ref``: the node
    at the end of its ``$ref`` chain. ``ref`` is a ``$ref`` next to other
    keywords, which only rejects.
    """

    node: SchemaNode
    false: bool = False
    target: SchemaNode | None = None
    ref: SchemaNode | None = None
    types: frozenset[str] | None = None
    members: dict[CanonicalKey, int] | None = None
    const: bool = False
    minimum: int | float | None = None
    exclusive_minimum: int | float | None = None
    maximum: int | float | None = None
    exclusive_maximum: int | float | None = None
    min_length: int | None = None
    max_length: int | None = None
    pattern: Pattern | None = None
    pattern_source: str | None = None  # a pattern that is not a safe Pattern
    fmt: str | None = None
    min_items: int | None = None
    max_items: int | None = None
    prefix: tuple[SchemaNode, ...] = ()
    items: SchemaNode | None = None  # keyword schema; None for absent or true
    items_false: bool = False
    contains: SchemaNode | None = None
    unique: bool = False
    properties: dict[str, SchemaNode] = field(default_factory=dict)
    required: tuple[str, ...] = ()
    dependent: tuple[tuple[str, tuple[str, ...]], ...] = ()
    additional: SchemaNode | None = None  # keyword schema; None for absent or true
    additional_false: bool = False
    pattern_properties: tuple[tuple[str, Pattern | None, SchemaNode], ...] = ()
    property_names: SchemaNode | None = None
    min_properties: int | None = None
    max_properties: int | None = None
    not_: SchemaNode | None = None
    all_of: tuple[SchemaNode, ...] = ()
    any_of: tuple[SchemaNode, ...] | None = None
    one_of: tuple[SchemaNode, ...] | None = None
    if_: SchemaNode | None = None
    then: SchemaNode | None = None
    else_: SchemaNode | None = None
    resolved: bool = False
    grids: dict[int, tuple[int | None, int | None]] = field(default_factory=dict)
    key_patterns: dict[str, tuple[int, ...]] = field(default_factory=dict)

    @property
    def is_map(self) -> bool:
        return not self.properties and (
            self.additional is not None or bool(self.pattern_properties) or self.property_names is not None
        )

    @property
    def checks_arrays(self) -> bool:
        return (
            self.min_items is not None
            or self.max_items is not None
            or self.items_false
            or self.unique
            or self.contains is not None
        )

    @property
    def checks_objects(self) -> bool:
        return bool(
            self.required
            or self.dependent
            or self.additional_false
            or self.property_names is not None
            or self.min_properties is not None
            or self.max_properties is not None
        )

    @property
    def has_structure(self) -> bool:
        """Whether the node has subschemas for elements or member values."""
        return bool(
            self.properties
            or self.prefix
            or self.items is not None
            or self.pattern_properties
            or self.additional is not None
        )

    def deep(self, skip: str | None) -> bool:
        """Whether the node has checks that evaluate subschemas on the row itself."""
        return (
            self.not_ is not None
            or bool(self.all_of)
            or self.ref is not None
            or (self.any_of is not None and skip != "anyOf")
            or (self.one_of is not None and skip != "oneOf")
            or (self.if_ is not None and (self.then is not None or self.else_ is not None))
        )

    def grid(self, d: int) -> tuple[int | None, int | None]:
        """The lowest and highest multiples of ``10^−d`` (as integers m) that the numeric bounds allow."""
        cached = self.grids.get(d)
        if cached is None:
            scale = 10**d
            lows: list[int] = []
            highs: list[int] = []
            if self.minimum is not None:
                lows.append(math.ceil(_decimal_value(self.minimum) * scale))
            if self.exclusive_minimum is not None:
                lows.append(math.floor(_decimal_value(self.exclusive_minimum) * scale) + 1)
            if self.maximum is not None:
                highs.append(math.floor(_decimal_value(self.maximum) * scale))
            if self.exclusive_maximum is not None:
                highs.append(math.ceil(_decimal_value(self.exclusive_maximum) * scale) - 1)
            cached = self.grids[d] = (max(lows) if lows else None, min(highs) if highs else None)
        return cached


def _decimal_value(x: int | float) -> Fraction:
    """A bound as the decimal ``m·10^e`` that values are coded by (``codes.decimal_parts``).

    That is the exact value of an integral number and the shortest decimal
    of any other, so ``0.1`` is 1/10, not the double's binary value. The map
    is strictly increasing (each decimal lies in its double's rounding
    interval), so every value a bound admits lies on the admitted side of
    the bound's decimal, and its grid point is inside the grid.
    """
    m, e = decimal_parts(x)
    return Fraction(m * 10**e) if e >= 0 else Fraction(m, 10**-e)


def _assertions(kw: Mapping[str, Any], node: SchemaNode) -> list[str]:
    """The keywords of ``kw`` that constrain values (no annotations, ``$defs``, or ``x-`` keywords)."""
    found: list[str] = []
    for keyword in kw:
        if not isinstance(keyword, str):
            raise TypeError(f"schema keywords must be str, got {type(keyword).__name__}")
        if keyword in UNCODED_KEYWORDS or keyword == "$defs" or keyword.startswith(_ANNOTATION_PREFIX):
            continue
        if keyword not in CODED_KEYWORDS:
            raise ValueError(f"data bits are not defined for keyword {keyword!r} (node nid {node.nid})")
        found.append(keyword)
    return found


def _parse(node: SchemaNode, resolve: Callable[[Any], SchemaNode]) -> _Spec:
    """Parse and check the keywords of one node (see :class:`_Spec`)."""
    if not isinstance(node, SchemaNode):
        raise TypeError(f"expected a SchemaNode, got {type(node).__name__}")
    spec = _Spec(node)
    if node.boolean is not None:
        if not isinstance(node.boolean, bool) or node.kw:
            raise ValueError(f"malformed boolean schema (nid {node.nid})")
        spec.false = node.boolean is False
        return spec
    kw = node.kw
    if not isinstance(kw, dict):
        raise TypeError(f"kw must be a dict, got {type(kw).__name__}")
    assertions = _assertions(kw, node)
    for keyword in assertions:
        value = kw[keyword]
        if keyword == "$ref":
            target = resolve(value)
            if len(assertions) == 1:
                spec.target = target
            else:
                spec.ref = target
        elif keyword == "type":
            names = [value] if isinstance(value, str) else _distinct_names(value, keyword)
            for name in names:
                if name not in _TYPE_NAMES:
                    raise ValueError(f"{name!r} is not a JSON Schema type (node nid {node.nid})")
            spec.types = frozenset(names)
        elif keyword in ("enum", "const"):
            continue  # below, since the two combine
        elif keyword in ("minimum", "exclusiveMinimum", "maximum", "exclusiveMaximum"):
            setattr(spec, _BOUND_ATTRIBUTES[keyword], _bound_of(value, keyword))
        elif keyword in _LENGTH_ATTRIBUTES:
            setattr(spec, _LENGTH_ATTRIBUTES[keyword], _length_of(value, keyword))
        elif keyword == "pattern":
            if not isinstance(value, str):
                raise TypeError(f"'pattern' must be a string, got {type(value).__name__}")
            try:
                spec.pattern = Pattern.from_source(value)
            except ValueError:
                spec.pattern_source = value
        elif keyword == "format":
            if not isinstance(value, str):
                raise TypeError(f"'format' must be a string, got {type(value).__name__}")
            spec.fmt = value
        elif keyword == "prefixItems":
            spec.prefix = _subschemas(value, keyword)
        elif keyword == "items":
            spec.items, spec.items_false = _keyword_schema(value, keyword)
        elif keyword == "contains":
            spec.contains = _subschema(value, keyword)
        elif keyword == "uniqueItems":
            if not isinstance(value, bool):
                raise TypeError(f"'uniqueItems' must be a boolean, got {type(value).__name__}")
            spec.unique = value
        elif keyword == "properties":
            if not isinstance(value, dict):
                raise TypeError(f"'properties' must hold a dict of SchemaNodes, got {type(value).__name__}")
            spec.properties = {name: _subschema(child, keyword) for name, child in value.items()}
        elif keyword == "required":
            spec.required = tuple(_distinct_names(value, keyword))
        elif keyword == "dependentRequired":
            if not isinstance(value, dict):
                raise TypeError(f"'dependentRequired' must be an object, got {type(value).__name__}")
            entries = []
            for antecedent in sorted(value, key=codepoint_key):
                if not isinstance(antecedent, str):
                    raise TypeError(f"'dependentRequired' keys must be str, got {type(antecedent).__name__}")
                entries.append((antecedent, tuple(_distinct_names(value[antecedent], keyword))))
            spec.dependent = tuple(entries)
        elif keyword == "additionalProperties":
            spec.additional, spec.additional_false = _keyword_schema(value, keyword)
        elif keyword == "patternProperties":
            if not isinstance(value, dict):
                raise TypeError(f"'patternProperties' must hold a dict of SchemaNodes, got {type(value).__name__}")
            entries_pp: list[tuple[str, Pattern | None, SchemaNode]] = []
            for source in sorted(value, key=codepoint_key):
                child = _subschema(value[source], keyword)
                try:
                    entries_pp.append((source, Pattern.from_source(source), child))
                except ValueError:
                    entries_pp.append((source, None, child))
            spec.pattern_properties = tuple(entries_pp)
        elif keyword == "propertyNames":
            spec.property_names = _subschema(value, keyword)
        elif keyword == "not":
            spec.not_ = _subschema(value, keyword)
        elif keyword == "allOf":
            spec.all_of = _subschemas(value, keyword)
        elif keyword == "anyOf":
            spec.any_of = _subschemas(value, keyword)
        elif keyword == "oneOf":
            spec.one_of = _subschemas(value, keyword)
        elif keyword == "if":
            spec.if_ = _subschema(value, keyword)
        elif keyword == "then":
            spec.then = _subschema(value, keyword)
        elif keyword == "else":
            spec.else_ = _subschema(value, keyword)
        # x-latent-type and x-array-pattern start with "x-" and never get here.
    if "const" in kw:
        const_key = canonical_key(kw["const"])
        spec.const = True
        allowed = True
        if "enum" in kw:
            if not isinstance(kw["enum"], (list, tuple)):
                raise TypeError(f"'enum' must be an array, got {type(kw['enum']).__name__}")
            allowed = const_key in {canonical_key(member) for member in kw["enum"]}
        spec.members = {const_key: 0} if allowed else {}
    elif "enum" in kw:
        if not isinstance(kw["enum"], (list, tuple)):
            raise TypeError(f"'enum' must be an array, got {type(kw['enum']).__name__}")
        spec.members = {canonical_key(member): index for index, member in enumerate(distinct_values(kw["enum"]))}
    return spec


_BOUND_ATTRIBUTES: Final = MappingProxyType(
    {
        "minimum": "minimum",
        "exclusiveMinimum": "exclusive_minimum",
        "maximum": "maximum",
        "exclusiveMaximum": "exclusive_maximum",
    }
)
_LENGTH_ATTRIBUTES: Final = MappingProxyType(
    {
        "minLength": "min_length",
        "maxLength": "max_length",
        "minItems": "min_items",
        "maxItems": "max_items",
        "minProperties": "min_properties",
        "maxProperties": "max_properties",
    }
)


def _type_allows(types: frozenset[str] | None, rank: int) -> bool:
    if types is None:
        return True
    name = JType(rank).keyword
    return name in types or (rank == _INTEGER and "number" in types)


def _category(types: frozenset[str], rank: int) -> str:
    """The ``type`` name an accepted row of type ``rank`` is coded under."""
    name = JType(rank).keyword
    return name if name in types else "number"  # an integral number under "number"


# ---------------------------------------------------------------------------
# Data bits: primitive per-value codes
# ---------------------------------------------------------------------------


def _count_bits(n: int, low: int | None, high: int | None) -> float:
    """A length or count n: uniform with a maximum (the minimum defaults to 0), else LN0 above the minimum."""
    if high is not None:
        return UNI(high - (low or 0) + 1)
    if low is not None:
        return LN0(n - low)
    return LN0(n)


def _grid_bits(m: int, low: int | None, high: int | None) -> float:
    """The integer code of m in [low, high]: uniform, one-sided LN0, or LZ."""
    if low is not None and high is not None:
        return UNI(high - low + 1)
    if low is not None:
        return LN0(m - low)
    if high is not None:
        return LN0(high - m)
    return LZ(m)


def _integer_bits(spec: _Spec, value: int) -> float:
    low, high = spec.grid(0)
    return _grid_bits(value, low, high)


def _number_bits(spec: _Spec, value: int | float) -> float:
    """``LN0(d)`` plus the integer code of m for ``value = m·10^−d`` on the bounded grid."""
    m, e = decimal_parts(value)
    if e < 0:
        d = -e
    else:
        d, m = 0, m * 10**e
    low, high = spec.grid(d)
    return LN0(d) + _grid_bits(m, low, high)


def _char_bits(spec: _Spec, s: str) -> float:
    pattern = spec.pattern
    if pattern is not None:
        decomposition = pattern.match(s)
        if decomposition is None:  # pragma: no cover - accepted strings match
            raise AssertionError(f"accepted string {s!r} does not match {pattern.source!r}")
        alternative = pattern.alternatives[decomposition.alternative]
        terms = [UNI(len(pattern.alternatives))]
        for token, count in zip(alternative.tokens, decomposition.counts):
            atom = token.atom
            if isinstance(atom, CharClass):
                terms.append(count * _CLASS_LOG2[atom])
            quantifier = token.quantifier
            if quantifier.high is None:
                terms.append(LN0(count - quantifier.low))
            else:
                terms.append(UNI(quantifier.high - quantifier.low + 1))
        return math.fsum(terms)
    if spec.fmt is not None:
        log2_alphabet = _FORMAT_LOG2.get(spec.fmt)
        if log2_alphabet is not None:
            return len(s) * log2_alphabet
    return _BITS_PER_BYTE * utf8_length(s)


def _string_bits(spec: _Spec, s: str) -> float:
    return _count_bits(len(s), spec.min_length, spec.max_length) + _char_bits(spec, s)


# ---------------------------------------------------------------------------
# Data bits: view helpers
# ---------------------------------------------------------------------------


def _mask(n: int, positions: np.ndarray) -> np.ndarray:
    out = np.zeros(n, dtype=bool)
    out[positions] = True
    return out


def _parent_positions(child: View, parent: View) -> np.ndarray:
    """For each row of ``child`` (derived from ``parent`` by one step), the position of its parent in ``parent``."""
    index: dict[ShapePath, tuple[int, np.ndarray]] = {}
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
    if not found:
        return _EMPTY_IDS
    return found[0] if len(found) == 1 else np.concatenate(found)


def _members_of(view: View, other: View) -> np.ndarray:
    """Bool mask of the rows of ``view`` that are also rows of ``other``."""
    index = {part.column: part.rows for part in other.live().parts}
    masks = []
    for part in view.parts:
        rows = index.get(part.column)
        masks.append(isin_sorted(part.rows, rows) if rows is not None else np.zeros(len(part.rows), dtype=bool))
    return np.concatenate(masks) if masks else np.zeros(0, dtype=bool)


def _rebuild_rows(store: ObservationStore, column: ShapePath, rows: np.ndarray) -> list[JSON]:
    """The JSON values of ``rows`` of ``column``, rebuilt from the columns (flags not applied)."""
    col = store.columns[column]
    jtype = col.jtype[rows]
    values = col.value[rows]
    out: list[JSON] = [None] * len(rows)
    table = store.values
    for i in np.flatnonzero(values >= 0).tolist():
        out[i] = table[int(values[i])]
    for kind, step in ((_ARRAY, ELEM_STEP), (_OBJECT, MAPVAL_STEP)):
        where = np.flatnonzero(jtype == kind)
        if not len(where):
            continue
        child = store.columns.get(column + (step,))
        if child is None:
            for i in where.tolist():
                out[i] = [] if kind == _ARRAY else {}
            continue
        lo, hi = child.runs(rows[where])
        child_rows = ranges_to_rows(lo, hi)
        items = _rebuild_rows(store, column + (step,), child_rows)
        keys = child.key[child_rows].tolist() if kind == _OBJECT else []
        start = 0
        for i, length in zip(where.tolist(), (hi - lo).tolist()):
            stop = start + length
            if kind == _ARRAY:
                out[i] = items[start:stop]
            else:
                out[i] = {store.key_name(kid): item for kid, item in zip(keys[start:stop], items[start:stop])}
            start = stop
    return out


def _rebuild(view: View, positions: np.ndarray) -> list[JSON]:
    """The JSON values of the rows at ascending ``positions`` of ``view``."""
    out: list[JSON] = []
    offset = 0
    for part in view.parts:
        stop = offset + len(part.rows)
        picked = positions[(positions >= offset) & (positions < stop)] - offset
        if len(picked):
            out.extend(_rebuild_rows(view.store, part.column, part.rows[picked]))
        offset = stop
    return out


def _subtree_size(view: View) -> int:
    """The number of non-flagged Observations in ``view`` and their subtrees."""
    total = 0
    pending = [view]
    while pending:
        current = pending.pop()
        stats = current.stats()
        total += stats.n
        if stats.count(JType.ARRAY):
            pending.append(current.elements())
        if stats.n_objects:
            pending.append(current.child_map())
    return total


def _presence_units(optional: list[str], successors: Mapping[str, list[str]]) -> list[list[str]]:
    """The strongly connected components of the ``dependentRequired`` graph over ``optional`` (in its order)."""
    reach: dict[str, set[str]] = {}
    for name in optional:
        if name in successors:
            seen: set[str] = set()
            todo = list(successors[name])
            while todo:
                other = todo.pop()
                if other not in seen:
                    seen.add(other)
                    todo.extend(successors.get(other, []))
            reach[name] = seen
    units: list[list[str]] = []
    assigned: set[str] = set()
    for name in optional:
        if name in assigned:
            continue
        unit = [name]
        reached = reach.get(name)
        if reached:
            unit.extend(other for other in optional if other != name and other in reached and name in reach.get(other, ()))
        assigned.update(unit)
        units.append(unit)
    return units


# ---------------------------------------------------------------------------
# Data bits: the coder
# ---------------------------------------------------------------------------


class _DataCoder:
    """One data-bits computation: parsed nodes, caches, and the collected terms."""

    __slots__ = (
        "_active",
        "_branch_views",
        "_key_bits",
        "_pattern_key_specs",
        "_payloads",
        "_root",
        "_specs",
        "_store",
        "_validator",
        "data_terms",
        "exception_terms",
        "n_rejected",
    )

    def __init__(
        self,
        store: ObservationStore,
        root: SchemaNode,
        validator: Validator,
        branch_views: Mapping[int, Sequence[View]],
    ) -> None:
        self._store = store
        self._root = root
        self._validator = validator
        self._branch_views = branch_views
        self._specs: dict[int, tuple[SchemaNode, _Spec]] = {}
        self._active: set[tuple[int, bytes]] = set()  # evaluations in progress; membership only
        self._payloads: dict[int, float] = {}
        self._key_bits: dict[int, float] = {}
        self._pattern_key_specs: dict[str, tuple[SchemaNode, _Spec]] = {}  # patternProperties source -> its key code
        self.data_terms: list[float] = []
        self.exception_terms: list[float] = []
        self.n_rejected = 0

    # -- nodes ---------------------------------------------------------------

    def resolve(self, ref: Any) -> SchemaNode:
        """The node a local ``$ref`` (a JSON Pointer fragment) names under the root."""
        if not isinstance(ref, str):
            raise TypeError(f"'$ref' must be a string, got {type(ref).__name__}")
        if not ref.startswith("#"):
            raise ValueError(f"only local $ref fragments resolve, got {ref!r}")
        try:
            return self._root.resolve(unquote(ref[1:]))
        except KeyError as exc:
            raise ValueError(f"$ref {ref!r} does not resolve: {exc}") from None

    def _raw_spec(self, node: SchemaNode) -> _Spec:
        entry = self._specs.get(id(node))
        if entry is not None and entry[0] is node:
            return entry[1]
        spec = _parse(node, self.resolve)
        self._specs[id(node)] = (node, spec)
        return spec

    def spec(self, node: SchemaNode) -> _Spec:
        """The parsed node; a ``$ref``-only node's ``target`` is the end of its chain."""
        spec = self._raw_spec(node)
        if spec.target is not None and not spec.resolved:
            chain = {id(node)}
            current = spec.target
            while True:
                if id(current) in chain:
                    raise ValueError(f"$ref cycle without a step through node nid {current.nid}")
                chain.add(id(current))
                following = self._raw_spec(current).target
                if following is None:
                    break
                current = following
            spec.target = current
        spec.resolved = True
        return spec

    def _enter(self, node: SchemaNode, token: bytes) -> tuple[int, bytes]:
        key = (id(node), token)
        if key in self._active:
            raise ValueError(f"schema cycle without a step through node nid {node.nid}")
        self._active.add(key)
        return key

    # -- per-value caches ----------------------------------------------------

    def _payload(self, vid: int) -> float:
        """``U(v) − UNI(7)`` for the scalar with value id ``vid``."""
        bits = self._payloads.get(vid)
        if bits is None:
            rank = self._store.values.key(vid)[0]
            if rank == _NULL:
                bits = 0.0
            elif rank == _BOOLEAN:
                bits = 1.0
            elif rank == _STRING:
                value = self._store.values[vid]
                assert isinstance(value, str)
                bits = STR(value)
            else:
                number = self._store.values[vid]
                assert isinstance(number, (int, float)) and not isinstance(number, bool)
                bits = NUM(number)
            self._payloads[vid] = bits
        return bits

    def _pattern_key_spec(self, source: str) -> _Spec:
        """The spec of ``{"pattern": source}``: the string code of the keys a ``patternProperties`` Pattern matches."""
        found = self._pattern_key_specs.get(source)
        if found is None:
            node = SchemaNode(kw={"pattern": source})
            found = self._pattern_key_specs[source] = (node, self.spec(node))
        return found[1]

    def _key_str(self, kid: int) -> float:
        bits = self._key_bits.get(kid)
        if bits is None:
            bits = self._key_bits[kid] = STR(self._store.key_name(kid))
        return bits

    def _key_patterns(self, spec: _Spec, name: str) -> tuple[int, ...]:
        """Indices of the ``patternProperties`` patterns that match key ``name``."""
        found = spec.key_patterns.get(name)
        if found is None:
            hits = []
            for index, (source, pattern, _) in enumerate(spec.pattern_properties):
                if pattern.matches(name) if pattern is not None else self._validator.matches_pattern(source, name):
                    hits.append(index)
            found = spec.key_patterns[name] = tuple(hits)
        return found

    def _property_kids(self, spec: _Spec) -> np.ndarray:
        """Sorted key ids of the ``properties`` names that occur in the store."""
        kids = [kid for kid in (self._store.key_id(name) for name in spec.properties) if kid is not None]
        return np.array(sorted(kids), dtype=np.int64) if kids else _EMPTY_IDS

    # -- acceptance ----------------------------------------------------------

    def _scalar_ok(self, spec: _Spec, value: Any) -> bool:
        """The checks of the node's own keywords on one scalar value."""
        key = canonical_key(value)
        rank = key[0]
        if not _type_allows(spec.types, rank):
            return False
        if spec.members is not None and key not in spec.members:
            return False
        if rank == _INTEGER or rank == _NUMBER:
            if spec.minimum is not None and value < spec.minimum:
                return False
            if spec.exclusive_minimum is not None and value <= spec.exclusive_minimum:
                return False
            if spec.maximum is not None and value > spec.maximum:
                return False
            if spec.exclusive_maximum is not None and value >= spec.exclusive_maximum:
                return False
        elif rank == _STRING:
            length = len(value)
            if spec.min_length is not None and length < spec.min_length:
                return False
            if spec.max_length is not None and length > spec.max_length:
                return False
            if spec.pattern is not None:
                if not spec.pattern.matches(value):
                    return False
            elif spec.pattern_source is not None and not self._validator.matches_pattern(spec.pattern_source, value):
                return False
            if spec.fmt is not None and not self._validator.satisfies_format(spec.fmt, value):
                return False
        return True

    def _deep_reject(
        self, spec: _Spec, n: int, evaluate: Callable[[SchemaNode, np.ndarray], np.ndarray], skip: str | None
    ) -> np.ndarray:
        """Items (0 … n−1) rejected by the subschema checks of ``spec``.

        ``evaluate(sub, positions)`` is the acceptance mask of ``sub`` on the
        items at ascending ``positions``.
        """
        rejected = np.zeros(n, dtype=bool)
        alive = np.arange(n)

        def drop(bad: np.ndarray) -> np.ndarray:
            rejected[alive[bad]] = True
            kept: np.ndarray = alive[~bad]
            return kept

        if spec.not_ is not None and len(alive):
            alive = drop(evaluate(spec.not_, alive))
        for sub in spec.all_of:
            if len(alive):
                alive = drop(~evaluate(sub, alive))
        if spec.ref is not None and len(alive):
            alive = drop(~evaluate(spec.ref, alive))
        if spec.any_of is not None and skip != "anyOf" and len(alive):
            hit = np.zeros(len(alive), dtype=bool)
            for branch in spec.any_of:
                todo = np.flatnonzero(~hit)
                if not len(todo):
                    break
                hit[todo[evaluate(branch, alive[todo])]] = True
            alive = drop(~hit)
        if spec.one_of is not None and skip != "oneOf" and len(alive):
            count = np.zeros(len(alive), dtype=np.int64)
            for branch in spec.one_of:
                count += evaluate(branch, alive)
            alive = drop(count != 1)
        if spec.if_ is not None and (spec.then is not None or spec.else_ is not None) and len(alive):
            condition = evaluate(spec.if_, alive)
            bad = np.zeros(len(alive), dtype=bool)
            if spec.then is not None:
                where = np.flatnonzero(condition)
                if len(where):
                    bad[where[~evaluate(spec.then, alive[where])]] = True
            if spec.else_ is not None:
                where = np.flatnonzero(~condition)
                if len(where):
                    bad[where[~evaluate(spec.else_, alive[where])]] = True
            alive = drop(bad)
        return rejected

    def values_accept(self, node: SchemaNode, values: list[Any], skip: str | None = None) -> np.ndarray:
        """Whether each scalar value (or object key) validates against ``node``."""
        n = len(values)
        if not n:
            return np.ones(0, dtype=bool)
        spec = self.spec(node)
        if spec.false:
            return np.zeros(n, dtype=bool)
        key = self._enter(node, b"")
        try:
            if spec.target is not None:
                return self.values_accept(spec.target, values)
            ok = np.fromiter((self._scalar_ok(spec, value) for value in values), dtype=bool, count=n)
            if spec.deep(skip):
                alive = np.flatnonzero(ok)
                if len(alive):

                    def evaluate(sub: SchemaNode, positions: np.ndarray) -> np.ndarray:
                        return self.values_accept(sub, [values[i] for i in alive[positions].tolist()])

                    ok[alive[self._deep_reject(spec, len(alive), evaluate, skip)]] = False
            return ok
        finally:
            self._active.discard(key)

    def accept_mask(self, node: SchemaNode, view: View) -> np.ndarray:
        """Whether each row of ``view`` validates against ``node`` (its whole subtree included)."""
        n = len(view)
        if not n:
            return np.ones(0, dtype=bool)
        key = self._enter(node, view.digest)
        try:
            return self._accept_walk([(node, view, np.arange(n))], n)
        finally:
            self._active.discard(key)

    def _accept_walk(self, tasks: list[tuple[SchemaNode, View, np.ndarray]], n: int) -> np.ndarray:
        """Acceptance of n top rows; each task's ``owner`` maps its rows to the top row they descend from."""
        accepted = np.ones(n, dtype=bool)
        stack = list(tasks)
        while stack:
            node, view, owner = stack.pop()
            if not len(view):
                continue
            alive = accepted[owner]
            if not alive.all():
                if not alive.any():
                    continue
                view, owner = view.subset(alive), owner[alive]
            spec = self.spec(node)
            if spec.target is not None:
                stack.append((spec.target, view, owner))
                continue
            rejected = self.local_reject(spec, view, None)
            if rejected.any():
                accepted[owner[rejected]] = False
                if rejected.all():
                    continue
                view, owner = view.subset(~rejected), owner[~rejected]
            for child, child_view in self._structure(spec, view):
                stack.append((child, child_view, owner[_parent_positions(child_view, view)]))
        return accepted

    def _structure(self, spec: _Spec, view: View) -> list[tuple[SchemaNode, View]]:
        """The element and member-value views of ``view`` with the subschemas that must accept them."""
        found: list[tuple[SchemaNode, View]] = []
        stats = view.stats()
        if stats.count(JType.ARRAY) and (spec.prefix or spec.items is not None):
            elements = view.elements()
            if len(elements):
                k = len(spec.prefix)
                if k:
                    position = elements.gather("pos")
                    for index, child in enumerate(spec.prefix):
                        at = position == index
                        if at.any():
                            found.append((child, elements.subset(at)))
                    later = position >= k
                    if spec.items is not None and later.any():
                        found.append((spec.items, elements.subset(later)))
                elif spec.items is not None:
                    found.append((spec.items, elements))
        if stats.n_objects:
            for name in sorted(spec.properties, key=codepoint_key):
                child_view = view.child_key(name)
                if len(child_view):
                    found.append((spec.properties[name], child_view))
            if spec.pattern_properties or spec.additional is not None:
                members = view.child_map()
                if len(members):
                    keys = members.gather("key").astype(np.int64)
                    # Every matching pattern applies, to property keys too; additionalProperties to the rest.
                    unmatched = ~isin_sorted(keys, self._property_kids(spec))
                    if spec.pattern_properties:
                        distinct = np.unique(keys)
                        hits = [self._key_patterns(spec, self._store.key_name(kid)) for kid in distinct.tolist()]
                        for index, (_, _, child) in enumerate(spec.pattern_properties):
                            kids = distinct[np.fromiter((index in h for h in hits), dtype=bool, count=len(hits))]
                            rows = isin_sorted(keys, kids)
                            if rows.any():
                                found.append((child, members.subset(rows)))
                                unmatched &= ~rows
                    if spec.additional is not None and unmatched.any():
                        found.append((spec.additional, members.subset(unmatched)))
        return found

    def local_reject(self, spec: _Spec, view: View, skip: str | None) -> np.ndarray:
        """Rows of ``view`` that the node's own keywords reject (subschemas of members and elements excluded)."""
        n = len(view)
        if spec.false:
            return np.ones(n, dtype=bool)
        if spec.target is not None:
            return self.local_reject(self.spec(spec.target), view, skip)
        values = view.gather("value")
        rejected = np.zeros(n, dtype=bool)
        scalar = values >= 0
        if scalar.any():
            vids = np.unique(values[scalar])
            table = self._store.values
            ok = self.values_accept(spec.node, [table[vid] for vid in vids.tolist()], skip)
            if not ok.all():
                rejected[scalar] = isin_sorted(values[scalar], vids[~ok])
        if not scalar.all():
            rejected |= self._container_reject(spec, view, scalar, skip)
        return rejected

    def _container_reject(self, spec: _Spec, view: View, scalar: np.ndarray, skip: str | None) -> np.ndarray:
        n = len(view)
        jtype = view.gather("jtype")
        rejected = np.zeros(n, dtype=bool)
        is_array = jtype == _ARRAY
        is_object = jtype == _OBJECT
        if spec.types is not None:
            if "array" not in spec.types:
                rejected |= is_array
            if "object" not in spec.types:
                rejected |= is_object
        if spec.members is not None:
            where = np.flatnonzero(~scalar & ~rejected)
            if len(where):
                members = spec.members
                for position, value in zip(where.tolist(), _rebuild(view, where)):
                    if canonical_key(value) not in members:
                        rejected[position] = True
        arrays = is_array & ~rejected
        if spec.checks_arrays and arrays.any():
            rejected |= self._array_reject(spec, view, arrays)
        objects = is_object & ~rejected
        if spec.checks_objects and objects.any():
            rejected |= self._object_reject(spec, view, objects)
        if spec.deep(skip):
            alive = np.flatnonzero(~scalar & ~rejected)
            if len(alive):

                def evaluate(sub: SchemaNode, positions: np.ndarray) -> np.ndarray:
                    return self.accept_mask(sub, view.subset(_mask(n, alive[positions])))

                rejected[alive[self._deep_reject(spec, len(alive), evaluate, skip)]] = True
        return rejected

    def _array_reject(self, spec: _Spec, view: View, arrays: np.ndarray) -> np.ndarray:
        n = len(view)
        where = np.flatnonzero(arrays)
        sizes = view.gather("size")[where]
        bad = np.zeros(len(where), dtype=bool)
        if spec.min_items is not None:
            bad |= sizes < spec.min_items
        if spec.max_items is not None:
            bad |= sizes > spec.max_items
        if spec.items_false:
            bad |= sizes > len(spec.prefix)
        if spec.unique or spec.contains is not None:
            keep = np.flatnonzero(~bad)
            if len(keep):
                arrays_view = view.subset(_mask(n, where[keep]))
                elements = arrays_view.elements()
                failed = np.zeros(len(keep), dtype=bool)
                if len(elements):
                    parents = _parent_positions(elements, arrays_view)
                    if spec.unique:
                        pairs = np.stack([parents, self._identities(elements)], axis=1)
                        distinct, counts = np.unique(pairs, axis=0, return_counts=True)
                        failed[np.unique(distinct[counts > 1, 0])] = True
                    if spec.contains is not None:
                        holds = np.zeros(len(keep), dtype=bool)
                        holds[parents[self.accept_mask(spec.contains, elements)]] = True
                        failed |= ~holds
                elif spec.contains is not None:
                    failed[:] = True
                bad[keep[failed]] = True
        out = np.zeros(n, dtype=bool)
        out[where[bad]] = True
        return out

    def _identities(self, view: View) -> np.ndarray:
        """Integer ids that are equal exactly when the rows' values are JSON-equal."""
        ids = view.gather("value").astype(np.int64)
        containers = np.flatnonzero(ids < 0)
        if len(containers):
            base = len(self._store.values)
            seen: dict[CanonicalKey, int] = {}
            for position, value in zip(containers.tolist(), _rebuild(view, containers)):
                ids[position] = base + seen.setdefault(canonical_key(value), len(seen))
        return ids

    def _presence(self, stats: ViewStats, name: str, cache: dict[str, np.ndarray]) -> np.ndarray:
        """Bool mask over the view's objects of those that have key ``name``."""
        found = cache.get(name)
        if found is None:
            found = np.zeros(stats.n_objects, dtype=bool)
            kid = self._store.key_id(name)
            if kid is not None:
                found[stats.objects_with(kid)] = True
            cache[name] = found
        return found

    def _object_reject(self, spec: _Spec, view: View, objects: np.ndarray) -> np.ndarray:
        stats = view.stats()
        rows = stats.object_index
        bad = np.zeros(stats.n_objects, dtype=bool)
        if spec.min_properties is not None or spec.max_properties is not None:
            sizes = view.gather("size")[rows]
            if spec.min_properties is not None:
                bad |= sizes < spec.min_properties
            if spec.max_properties is not None:
                bad |= sizes > spec.max_properties
        cache: dict[str, np.ndarray] = {}
        for name in spec.required:
            bad |= ~self._presence(stats, name, cache)
        for antecedent, consequents in spec.dependent:
            holds = self._presence(stats, antecedent, cache)
            if holds.any():
                for consequent in consequents:
                    bad |= holds & ~self._presence(stats, consequent, cache)
        if (spec.additional_false or spec.property_names is not None) and len(stats.key_ids):
            names = [self._store.key_name(kid) for kid in stats.key_ids.tolist()]
            bad_key = np.zeros(len(names), dtype=bool)
            if spec.additional_false:
                properties = spec.properties
                bad_key |= np.fromiter(
                    (name not in properties and not self._key_patterns(spec, name) for name in names),
                    dtype=bool,
                    count=len(names),
                )
            if spec.property_names is not None:
                bad_key |= ~self.values_accept(spec.property_names, names)
            if bad_key.any():
                bad[stats.key_objects[np.repeat(bad_key, np.diff(stats.key_indptr))]] = True
        out = np.zeros(len(view), dtype=bool)
        out[rows[bad]] = True
        out &= objects
        return out

    # -- coding --------------------------------------------------------------

    def code(self, node: SchemaNode, view: View) -> None:
        """Collect the data and exception terms of ``view`` under ``node``."""
        # (node, view, chain): chain holds the nodes passed through without a
        # step ($ref targets and branches), to catch cycles.
        stack: list[tuple[SchemaNode, View, tuple[int, ...]]] = [(node, view, ())]
        while stack:
            current, rows, chain = stack.pop()
            if not len(rows):
                continue
            spec = self.spec(current)
            if spec.target is not None:
                stack.append(self._same_level(current, spec.target, rows, chain))
                continue
            n = len(rows)
            branch_keyword = "anyOf" if spec.any_of is not None else "oneOf" if spec.one_of is not None else None
            labels: np.ndarray | None = None
            if spec.false:
                rejected = np.ones(n, dtype=bool)
            else:
                rejected = self.local_reject(spec, rows, branch_keyword)
                if branch_keyword is not None:
                    if spec.has_structure and not rejected.all():
                        alive = np.flatnonzero(~rejected)
                        sub = rows.subset(~rejected)
                        tasks = [
                            (child, child_view, _parent_positions(child_view, sub))
                            for child, child_view in self._structure(spec, sub)
                        ]
                        rejected[alive[~self._accept_walk(tasks, len(sub))]] = True
                    labels = self._assign(spec, branch_keyword, rows, rejected)
                    rejected |= labels < 0
            self._exceptions(rows, rejected)
            if rejected.all():
                continue
            if labels is not None:
                branches = spec.any_of if branch_keyword == "anyOf" else spec.one_of
                assert branches is not None
                counts = np.bincount(labels[labels >= 0], minlength=len(branches)).tolist()
                self.data_terms.append(CAT(counts))
                for index, branch in enumerate(branches):
                    if counts[index]:
                        stack.append(self._same_level(current, branch, rows.subset(labels == index), chain))
                continue
            accepted = rows.subset(~rejected) if rejected.any() else rows
            self._code_values(spec, accepted, stack)

    @staticmethod
    def _same_level(
        node: SchemaNode, child: SchemaNode, view: View, chain: tuple[int, ...]
    ) -> tuple[SchemaNode, View, tuple[int, ...]]:
        """A task for ``child`` on rows of ``node``'s own view; a node met again on the way is a cycle."""
        chain = chain + (id(node),)
        if id(child) in chain:
            raise ValueError(f"schema cycle without a step through node nid {child.nid}")
        return (child, view, chain)

    def _assign(self, spec: _Spec, keyword: str, view: View, rejected: np.ndarray) -> np.ndarray:
        """The branch index of each row (−1: rejected or taken by no branch)."""
        branches = spec.any_of if keyword == "anyOf" else spec.one_of
        assert branches is not None
        n = len(view)
        labels = np.full(n, -1, dtype=np.int64)
        alive = ~rejected
        hints = self._branch_views.get(spec.node.nid)
        if hints is not None:
            if len(hints) != len(branches):
                raise ValueError(
                    f"branch_views for node nid {spec.node.nid} has {len(hints)} views for {len(branches)} branches"
                )
            for index, hint in enumerate(hints):
                labels[alive & (labels < 0) & _members_of(view, hint)] = index
        if keyword == "oneOf":
            where = np.flatnonzero(alive)
            if len(where) and branches:
                sub = view.subset(_mask(n, where))
                accepts = np.stack([self.accept_mask(branch, sub) for branch in branches])
                count = accepts.sum(axis=0)
                free = (labels[where] < 0) & (count == 1)
                labels[where[free]] = accepts.argmax(axis=0)[free]
                labels[where[count >= 2]] = -1
        else:
            todo = np.flatnonzero(alive & (labels < 0))
            for index, branch in enumerate(branches):
                if not len(todo):
                    break
                hit = self.accept_mask(branch, view.subset(_mask(n, todo)))
                labels[todo[hit]] = index
                todo = todo[~hit]
        return labels

    def _exceptions(self, view: View, rejected: np.ndarray) -> None:
        """``EXC = 1 + [CAT(|A|, |R|) + Σ U(v_r) if R ≠ ∅]``."""
        self.exception_terms.append(EXCEPTION_FLAG_BITS)
        n_rejected = int(rejected.sum())
        if n_rejected:
            self.exception_terms.append(CAT((len(view) - n_rejected, n_rejected)))
            self._universal_terms(view.subset(rejected), self.exception_terms)
            self.n_rejected += n_rejected

    def _universal_terms(self, view: View, terms: list[float]) -> None:
        """``Σ U(v)`` over the rows of ``view``, from per-distinct-value costs and counts."""
        pending = [view]
        while pending:
            current = pending.pop()
            stats = current.stats()
            if not stats.n:
                continue
            terms.append(_TYPE_TAG_BITS * stats.n)
            for vid, count in zip(stats.value_ids.tolist(), stats.value_counts.tolist()):
                payload = self._payload(vid)
                if payload:
                    terms.append(payload * count)
            for histogram in (stats.array_lengths, stats.object_sizes):
                for length, count in zip(histogram.values.tolist(), histogram.counts.tolist()):
                    terms.append(LN0(length) * count)
            for kid, count in zip(stats.key_ids.tolist(), stats.key_counts.tolist()):
                terms.append(self._key_str(kid) * count)
            if stats.count(JType.ARRAY):
                pending.append(current.elements())
            if stats.n_objects:
                pending.append(current.child_map())

    def _code_values(self, spec: _Spec, view: View, stack: list[tuple[SchemaNode, View, tuple[int, ...]]]) -> None:
        """Data terms of accepted rows under a node without branches; pushes the children."""
        store = self._store
        terms = self.data_terms
        stats = view.stats()
        if spec.const:
            return
        if spec.members is not None:
            counts = [0] * len(spec.members)
            for vid, count in zip(stats.value_ids.tolist(), stats.value_counts.tolist()):
                counts[spec.members[store.values.key(vid)]] += count
            containers = np.flatnonzero(view.gather("value") < 0)
            if len(containers):
                for value in _rebuild(view, containers):
                    counts[spec.members[canonical_key(value)]] += 1
            terms.append(CAT(counts))
            return

        type_counts = stats.type_counts.tolist()
        if spec.types is None:
            terms.append(CAT(type_counts))
        else:
            by_name = dict.fromkeys(sorted(spec.types), 0)
            for rank, count in enumerate(type_counts):
                if count:
                    by_name[_category(spec.types, rank)] += count
            terms.append(CAT(list(by_name.values())))

        integer_code = spec.types is None or "integer" in spec.types
        n_true = n_false = 0
        for vid, count in zip(stats.value_ids.tolist(), stats.value_counts.tolist()):
            rank, payload = store.values.key(vid)
            if rank == _NULL:
                continue
            if rank == _BOOLEAN:
                if payload:
                    n_true += count
                else:
                    n_false += count
                continue
            value = store.values[vid]
            if rank == _STRING:
                assert isinstance(value, str)
                bits = _string_bits(spec, value)
            else:
                assert isinstance(value, (int, float)) and not isinstance(value, bool)
                bits = _integer_bits(spec, int(value)) if rank == _INTEGER and integer_code else _number_bits(spec, value)
            if bits:
                terms.append(bits * count)
        if n_true or n_false:
            terms.append(CAT((n_true, n_false)))

        if stats.count(JType.ARRAY):
            histogram = stats.array_lengths
            for length, count in zip(histogram.values.tolist(), histogram.counts.tolist()):
                bits = _count_bits(length, spec.min_items, spec.max_items)
                if bits:
                    terms.append(bits * count)
            self._push_elements(spec, view, stack)
        if stats.n_objects:
            self._code_objects(spec, view, stats, stack)

    @staticmethod
    def _push_elements(spec: _Spec, view: View, stack: list[tuple[SchemaNode, View, tuple[int, ...]]]) -> None:
        elements = view.elements()
        if not len(elements):
            return
        later_schema = _FALSE_NODE if spec.items_false else spec.items if spec.items is not None else _TRUE_NODE
        k = len(spec.prefix)
        if not k:
            stack.append((later_schema, elements, ()))
            return
        position = elements.gather("pos")
        for index, child in enumerate(spec.prefix):
            at = position == index
            if at.any():
                stack.append((child, elements.subset(at), ()))
        later = position >= k
        if later.any():
            stack.append((later_schema, elements.subset(later), ()))

    def _code_objects(
        self, spec: _Spec, view: View, stats: ViewStats, stack: list[tuple[SchemaNode, View, tuple[int, ...]]]
    ) -> None:
        terms = self.data_terms
        n_objects = stats.n_objects
        if spec.properties:
            self._presence_terms(spec, stats)
            for name in sorted(spec.properties, key=codepoint_key):
                child_view = view.child_key(name)
                if len(child_view):
                    stack.append((spec.properties[name], child_view, ()))

        kids = stats.key_ids.astype(np.int64)
        extra = ~isin_sorted(kids, self._property_kids(spec))
        if spec.is_map:
            histogram = stats.object_sizes
            for size, count in zip(histogram.values.tolist(), histogram.counts.tolist()):
                bits = _count_bits(size, spec.min_properties, spec.max_properties)
                if bits:
                    terms.append(bits * count)
        elif spec.additional_false and not spec.pattern_properties:
            pass  # a closed Record admits no extras (objects with one were rejected), so no extras flag is coded
        else:
            owners = stats.key_objects[np.repeat(extra, np.diff(stats.key_indptr))]
            per_object = np.bincount(owners, minlength=n_objects)
            holds = per_object > 0
            n_with = int(holds.sum())
            terms.append(CAT((n_objects - n_with, n_with)))
            if n_with:
                sizes, counts = np.unique(per_object[holds], return_counts=True)
                for size, count in zip(sizes.tolist(), counts.tolist()):
                    terms.append(LN0(size) * count)
        if not extra.any():
            return

        extra_kids = kids[extra]
        extra_counts = stats.key_counts[extra]
        if spec.property_names is None and spec.pattern_properties:
            # A key a patternProperties Pattern matches is coded under that Pattern (the first matching one).
            plain: list[int] = []
            for kid, count in zip(extra_kids.tolist(), extra_counts.tolist()):
                name = self._store.key_name(kid)
                hits = self._key_patterns(spec, name)
                if not hits:
                    plain.append(kid)
                    continue
                bits = _string_bits(self._pattern_key_spec(spec.pattern_properties[hits[0]][0]), name)
                if bits:
                    terms.append(bits * count)
            if plain:
                keep = isin_sorted(extra_kids, np.array(sorted(plain), dtype=np.int64))
                self._key_terms(None, extra_kids[keep], extra_counts[keep])
        else:
            self._key_terms(spec.property_names, extra_kids, extra_counts)
        members = view.child_map()
        if not len(members):
            return
        keys = members.gather("key").astype(np.int64)
        rows = isin_sorted(keys, extra_kids)
        unmatched = rows
        if spec.pattern_properties:
            first = np.array(
                [
                    (hits[0] if (hits := self._key_patterns(spec, self._store.key_name(kid))) else -1)
                    for kid in extra_kids.tolist()
                ],
                dtype=np.int64,
            )
            label = np.full(len(keys), -1, dtype=np.int64)
            label[rows] = first[np.searchsorted(extra_kids, keys[rows])]
            for index, (_, _, child) in enumerate(spec.pattern_properties):
                at = label == index
                if at.any():
                    stack.append((child, members.subset(at), ()))
            unmatched = rows & (label < 0)
        if unmatched.any():
            values = members.subset(unmatched)
            if spec.additional is not None:
                stack.append((spec.additional, values, ()))
            elif spec.additional_false:
                stack.append((_FALSE_NODE, values, ()))  # only flagged edge cases: closed objects were rejected
            else:
                self._universal_terms(values, terms)

    def _presence_terms(self, spec: _Spec, stats: ViewStats) -> None:
        """``CAT(present, absent)`` per presence unit over the objects without its antecedents."""
        required = set(spec.required)
        optional = [name for name in sorted(spec.properties, key=codepoint_key) if name not in required]
        if not optional:
            return
        is_optional = set(optional)
        antecedents: dict[str, list[str]] = {name: [] for name in optional}
        successors: dict[str, list[str]] = {}
        for antecedent, consequents in spec.dependent:
            for consequent in consequents:
                if consequent in is_optional and consequent != antecedent:
                    antecedents[consequent].append(antecedent)
                    if antecedent in is_optional:
                        successors.setdefault(antecedent, []).append(consequent)
        cache: dict[str, np.ndarray] = {}
        for unit in _presence_units(optional, successors):
            inside = set(unit)
            outside = sorted({a for member in unit for a in antecedents[member] if a not in inside}, key=codepoint_key)
            coded = np.ones(stats.n_objects, dtype=bool)
            for antecedent in outside:
                coded &= ~self._presence(stats, antecedent, cache)
            n_coded = int(coded.sum())
            if n_coded:
                n_present = int((self._presence(stats, unit[0], cache) & coded).sum())
                self.data_terms.append(CAT((n_present, n_coded - n_present)))

    def _key_terms(self, names_schema: SchemaNode | None, kids: np.ndarray, counts: np.ndarray) -> None:
        """Bits of the extra (or Map) keys: ``STR(k)``, or the string code of ``propertyNames``."""
        terms = self.data_terms
        if names_schema is None:
            for kid, count in zip(kids.tolist(), counts.tolist()):
                terms.append(self._key_str(kid) * count)
            return
        spec = self.spec(names_schema)
        if spec.target is not None:
            spec = self.spec(spec.target)
        if spec.const:
            return
        names = [self._store.key_name(kid) for kid in kids.tolist()]
        if spec.members is not None:
            tally = [0] * len(spec.members)
            for name, count in zip(names, counts.tolist()):
                tally[spec.members[canonical_key(name)]] += count
            terms.append(CAT(tally))
            return
        for name, count in zip(names, counts.tolist()):
            bits = _string_bits(spec, name)
            if bits:
                terms.append(bits * count)


# ---------------------------------------------------------------------------
# MDL_Scorer
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Score:
    """The description length of one candidate on one view (Requirements 19.1, 19.2, 19.5).

    ``data_bits`` includes ``exception_bits``. ``n_rejected`` is the number
    of Observations the candidate rejects (each pays an exception) and
    ``n_scored`` the number of Observations scored. All bit counts are
    finite and non-negative.
    """

    model_bits: float
    data_bits: float
    exception_bits: float
    n_rejected: int
    n_scored: int

    def __post_init__(self) -> None:
        for name in ("model_bits", "data_bits", "exception_bits"):
            bits = getattr(self, name)
            if not isinstance(bits, float) or not math.isfinite(bits) or bits < 0.0:
                raise ValueError(f"Score {name} must be a finite, non-negative float, got {bits!r}")
        if self.exception_bits > self.data_bits:
            raise ValueError("Score exception_bits are part of data_bits and cannot exceed them")
        for name in ("n_rejected", "n_scored"):
            _count_of(getattr(self, name), f"Score {name}")
        if self.n_rejected > self.n_scored:
            raise ValueError("Score n_rejected cannot exceed n_scored")

    @property
    def total(self) -> float:
        """The description length ``model_bits + data_bits``."""
        return math.fsum((self.model_bits, self.data_bits))


_shared_validator: Validator | None = None


def _default_validator() -> Validator:
    """One Validator per process for scorers built without one, so compiled checks are shared."""
    global _shared_validator
    if _shared_validator is None:
        _shared_validator = Validator()
    return _shared_validator


class MDLScorer:
    """Description lengths of candidate schema nodes on views (Requirement 19).

    ``root`` is the root of the model the scored nodes belong to: ``$ref``
    fragments resolve against it (by default against the scored node).
    ``n_defs`` is the number of ``$defs`` entries at that root (D in
    ``UNI(D)`` for ``$ref``); it defaults to the count of ``root`` when one
    is given, and otherwise each scored node supplies its own count, which
    is right for a model root. ``validator`` decides ``format`` and
    non-safe ``pattern`` checks; by default a Validator shared by the
    process. See the module docstring for the encoding.
    """

    __slots__ = ("_n_defs", "_root", "_validator")

    def __init__(
        self, *, n_defs: int | None = None, root: SchemaNode | None = None, validator: Validator | None = None
    ) -> None:
        if root is not None and not isinstance(root, SchemaNode):
            raise TypeError(f"root must be a SchemaNode, got {type(root).__name__}")
        if validator is not None and not isinstance(validator, Validator):
            raise TypeError(f"validator must be a Validator, got {type(validator).__name__}")
        if n_defs is None and root is not None:
            n_defs = _defs_of(root)
        self._n_defs = None if n_defs is None else _count_of(n_defs, "n_defs")
        self._root = root
        self._validator = validator

    @property
    def n_defs(self) -> int | None:
        return self._n_defs

    @property
    def root(self) -> SchemaNode | None:
        return self._root

    def model_bits(self, node: SchemaNode) -> float:
        """``M(node)`` in bits: finite, non-negative, and independent of keyword and entry order."""
        return model_bits(node, n_defs=self._n_defs)

    def data_bits(
        self, node: SchemaNode, view: View, *, branch_views: Mapping[int, Sequence[View]] | None = None
    ) -> tuple[float, float, int]:
        """``(data bits, exception bits, rejected Observations)`` of ``view`` under ``node``.

        The data bits include the exception bits. ``branch_views`` maps the
        nid of an ``anyOf``/``oneOf`` node to one view per branch: the rows
        each branch codes (see the module docstring).
        """
        coder = self._code(node, view, branch_views)
        data = math.fsum([*coder.data_terms, *coder.exception_terms])
        return data, math.fsum(coder.exception_terms), coder.n_rejected

    def score(
        self, node: SchemaNode, view: View, *, branch_views: Mapping[int, Sequence[View]] | None = None
    ) -> Score:
        """The :class:`Score` of ``node`` on the non-flagged Observations of ``view``."""
        model = self.model_bits(node)
        data, exceptions, n_rejected = self.data_bits(node, view, branch_views=branch_views)
        return Score(model, data, exceptions, n_rejected, _subtree_size(view.live()))

    def presence_bits(self, node: SchemaNode, view: View) -> float:
        """The Record presence bits that :meth:`data_bits` charges at ``node`` itself, over the rows it accepts.

        These are the ``CAT(present, absent)`` terms of the node's optional
        ``properties``, one per presence unit (see the module docstring:
        Records), over the non-flagged object rows of ``view`` that the
        node's own keywords accept. They depend only on the node's
        ``properties`` names, ``required``, and ``dependentRequired``, and on
        which keys those objects hold. So a candidate that differs from
        ``node`` only in ``required`` or ``dependentRequired`` entries that
        every non-flagged object of ``view`` satisfies has exactly the data
        terms of ``node`` with these presence terms replaced by its own, and
        a Decision_Point can score many such candidates with one full
        scoring (Co_Occurrence_Rule inclusion, ``deps.cooccurrence``).

        0 when the node codes no presence: no ``properties``, ``const`` or
        ``enum`` (the member codes the whole value), the ``false`` schema, or
        no accepted object. Raises ``ValueError`` for an ``anyOf``/``oneOf``
        node or a ``$ref``-only node, whose rows other nodes code.
        """
        if not isinstance(node, SchemaNode):
            raise TypeError(f"the scored node must be a SchemaNode, got {type(node).__name__}")
        if not isinstance(view, View):
            raise TypeError(f"the scored view must be a View, got {type(view).__name__}")
        rows = view.live()
        coder = _DataCoder(rows.store, self._root or node, self._validator or _default_validator(), {})
        spec = coder.spec(node)
        if spec.target is not None or spec.any_of is not None or spec.one_of is not None:
            raise ValueError("presence_bits codes a Record node itself, not an anyOf/oneOf or $ref-only node")
        if not len(rows) or spec.false or spec.const or spec.members is not None or not spec.properties:
            return 0.0
        rejected = coder.local_reject(spec, rows, None)
        accepted = rows.subset(~rejected) if rejected.any() else rows
        stats = accepted.stats()
        if not stats.n_objects:
            return 0.0
        coder._presence_terms(spec, stats)
        return math.fsum(coder.data_terms)

    def universal_bits(self, view: View) -> float:
        """``Σ U(v)`` over the non-flagged rows of ``view``: their exception cost beyond ``CAT(|A|, |R|)`` (19.2).

        Each row's whole non-flagged subtree is coded with the universal
        value code, exactly as an exception codes a rejected row, so expected
        exception costs (such as the Good–Turing charges of 15.6 and of the
        Closed-or-Open-Record comparison) use the same bits as real ones.
        """
        if not isinstance(view, View):
            raise TypeError(f"universal_bits takes a View, got {type(view).__name__}")
        view = view.live()
        coder = _DataCoder(view.store, self._root or _TRUE_NODE, self._validator or _default_validator(), {})
        terms: list[float] = []
        coder._universal_terms(view, terms)
        return math.fsum(terms)

    def _code(
        self, node: SchemaNode, view: View, branch_views: Mapping[int, Sequence[View]] | None
    ) -> _DataCoder:
        if not isinstance(node, SchemaNode):
            raise TypeError(f"the scored node must be a SchemaNode, got {type(node).__name__}")
        if not isinstance(view, View):
            raise TypeError(f"the scored view must be a View, got {type(view).__name__}")
        view = view.live()
        hints = _check_branch_views(branch_views, view.store)
        coder = _DataCoder(view.store, self._root or node, self._validator or _default_validator(), hints)
        coder.code(node, view)
        return coder


def _check_branch_views(
    branch_views: Mapping[int, Sequence[View]] | None, store: ObservationStore
) -> Mapping[int, Sequence[View]]:
    if branch_views is None:
        return {}
    if not isinstance(branch_views, Mapping):
        raise TypeError(f"branch_views must map node nids to views, got {type(branch_views).__name__}")
    for nid, views in branch_views.items():
        if isinstance(nid, bool) or not isinstance(nid, int):
            raise TypeError(f"branch_views keys are node nids (int), got {type(nid).__name__}")
        if not isinstance(views, Sequence) or isinstance(views, (str, bytes)):
            raise TypeError(f"branch_views[{nid}] must be a sequence of Views, got {type(views).__name__}")
        for item in views:
            if not isinstance(item, View):
                raise TypeError(f"branch_views[{nid}] entries must be Views, got {type(item).__name__}")
            if item.store is not store:
                raise ValueError(f"branch_views[{nid}] holds a view of another ObservationStore")
    return branch_views
