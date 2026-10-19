"""Subschema_Checker: inclusion between two JSON Schemas with jsonsubschema (Requirements 28.5, 31.7, 31.8).

:meth:`SubschemaChecker.relation` compares an inferred schema with a
ground-truth schema, both draft 2020-12 documents (dicts or booleans, as the
Schema_Emitter writes them and as a loaded Schema_Model emits). It returns a
:class:`Relation` when both inclusion checks complete (31.7), and an
:class:`Unsupported` with its cause otherwise (31.8). The steps, in order:

1. **Keyword scan** (:func:`unsupported_keywords`). Each schema object of both
   documents is scanned; every keyword that the checker cannot take is
   recorded with the JSON Pointer of its schema object and a reason. Any
   finding makes the result ``Unsupported(cause="keywords")`` listing the
   keyword names, and no check runs.
2. **Meta-schema check**. Each document must be valid against the draft
   2020-12 meta-schema (:meth:`Validator.meta_errors
   <jsinfer.io.validator.Validator.meta_errors>`), so that the shim below
   only meets 2020-12 shapes (a boolean ``exclusiveMinimum`` or an array-form
   ``items`` means something else in draft-04). A failure is
   ``Unsupported(cause="error")``.
3. **Shim** (:func:`to_draft4`): annotations and ``x-`` keywords are
   stripped and the document is rewritten into the draft-04 shape that
   jsonsubschema reads.
4. **Checks**. ``inferred ⊆ truth`` and then ``truth ⊆ inferred`` each run
   through the checker's runner with the per-check time limit
   ``subschema.timeout_s`` (a Resource_Parameter, default 30 s). The default
   runner, :class:`SubprocessRunner`, runs jsonsubschema's ``isSubschema``
   in a fresh child process per check. A check that errors, times out, or
   cannot run makes the result ``Unsupported`` with that cause, and the
   second check is skipped.

Supported keywords
    :data:`SUPPORTED_KEYWORDS` lists the draft 2020-12 keywords that the shim
    maps onto draft-04 keywords jsonsubschema decides: ``type``, ``enum``,
    ``const``, ``allOf``, ``anyOf``, ``oneOf``, ``not``, the string, number,
    array, and object bounds, ``multipleOf``, ``pattern``, ``items``,
    ``prefixItems``, ``properties``, ``patternProperties``,
    ``additionalProperties``, ``required``, ``$defs``, and, with the
    conditions below, ``$ref`` and ``uniqueItems``. :data:`STRIPPED_KEYWORDS`
    (the annotations the Schema_Loader retains, ``$anchor``, and
    ``$dynamicAnchor``), ``x-`` keywords, and the root ``$schema`` and
    ``$id`` assert nothing and are removed. Every other keyword is
    unsupported, whatever its draft or vocabulary, with these reasons:

    - a keyword outside both lists (:data:`NOT_SUPPORTED`). This covers the
      design's initial list (``format``, ``contains``, ``propertyNames``,
      ``if``/``then``/``else``) and every keyword without a draft-04
      counterpart (``unevaluatedProperties``, ``minContains``,
      ``$dynamicRef``, ...). It also covers ``dependentRequired`` and
      ``dependentSchemas``: their draft-04 form, ``dependencies``, is
      accepted by jsonsubschema 0.0.8 but ignored, so that ``{"type":
      "object"}`` is reported a subschema of ``{"type": "object",
      "dependencies": {"a": ["b"]}}``, which is wrong. The same goes for
      ``format``, which jsonsubschema ignores while the Validator asserts it;
    - ``$schema`` at the root declaring a dialect other than draft 2020-12
      (:data:`OTHER_DIALECT`), and ``$schema`` below the root
      (:data:`BELOW_ROOT`);
    - ``$id`` below the root, which starts an embedded schema resource
      (:data:`EMBEDDED_RESOURCE`), as for the Schema_Loader;
    - a ``$ref`` that is not ``#`` plus a JSON Pointer (:data:`NOT_LOCAL`:
      remote, relative, and anchor references), or whose pointer is not a
      schema position of the document (:data:`UNRESOLVED`), or whose
      expansion never ends (:data:`RECURSIVE`). A schema position is the
      root or a subschema reached through the subschema keywords of draft
      2020-12. A reference is recursive when its target holds, after any
      number of references followed, a reference back into a target already
      being expanded; references that only lead into such a cycle are
      recursive too, since their expansion does not end either;
    - an ``enum`` or ``const`` holding an object or array value
      (:data:`CONTAINER_VALUES`): jsonsubschema 0.0.8 raises
      ``UnsupportedEnumCanonicalization`` on such an enum (probed in
      ``tests/integration/test_subschema_probe.py``, task 25.6);
    - a ``not`` whose subschema holds object or array keywords
      (:data:`NEGATED_COMPOUND`: ``properties``, ``patternProperties``,
      ``additionalProperties``, ``required``, ``minProperties``,
      ``maxProperties``, ``items``, ``prefixItems``, ``minItems``,
      ``maxItems``, ``uniqueItems``), which jsonsubschema 0.0.8 cannot negate
      (``UnsupportedNegatedObject``/``UnsupportedNegatedArray``); a negated
      plain type (``{"not": {"type": "object"}}``) is fine, except a
      negated numeric type (:data:`NEGATED_NUMBER`): jsonsubschema 0.0.8
      reports ``{"not": {"type": "integer"}}`` and ``{"not": {"type":
      "number"}}`` equivalent, although 1.5 satisfies only the first;
    - ``uniqueItems: true`` on an array whose items may be objects or arrays
      (:data:`NON_SCALAR_ITEMS`). Items are known to be scalars when every
      ``prefixItems`` entry and ``items`` (which must then be present) admit
      only scalars: a ``false`` schema; a ``type`` without ``object`` and
      ``array``; an ``enum`` or ``const`` of scalars; an ``allOf`` with such
      a branch, an ``anyOf`` or ``oneOf`` whose branches all are; or a local
      ``$ref`` to such a schema. ``uniqueItems`` on a schema whose ``type``
      excludes ``array`` has no effect and is accepted.

    The keyword scan walks the subschemas of supported and unsupported
    keywords alike, so a finding nested under another (a ``format`` inside
    ``contains``) is listed as well. Task 25.6 probed jsonsubschema's actual
    keyword coverage: ``dependencies`` and ``format`` are ignored (so stay
    unsupported), ``multipleOf``, negated plain types, and integer-valued
    enums are decided correctly, and the two cases above are the ones that
    raise; the probe test records the results.

The draft-04 shim
    :func:`to_draft4` builds a new document; the input is never changed.

    - ``$defs`` becomes ``definitions``; ``prefixItems`` becomes array-form
      ``items``, and ``items`` next to it becomes ``additionalItems``.
    - ``const`` becomes a one-value ``enum``; next to an ``enum`` it becomes
      an ``{"enum": [c]}`` entry appended to ``allOf``. ``enum`` lists keep
      their distinct values under JSON equality (draft-04 requires unique
      entries), and every keyword value has integral numbers written as
      integers (``1.0`` as ``1``), which draft 2020-12 counts as integers.
    - Numeric ``exclusiveMinimum``/``exclusiveMaximum`` become draft-04
      boolean flags on ``minimum``/``maximum``: the tighter of the two lower
      (upper) bounds is kept, and the flag is set when it is the exclusive
      one.
    - ``true`` and ``false`` subschemas become ``{}`` and ``{"not": {}}``,
      except under ``additionalProperties`` and ``additionalItems``, where
      draft-04 takes booleans.
    - A ``$ref`` with siblings moves into an ``{"$ref": ...}`` entry
      appended to ``allOf``: in 2020-12 its siblings apply too, while the
      draft-04 reference resolver (jsonref) replaces the whole object.
      Appending keeps every other position where it was, so the only
      position changes are the renames above. Every ``$ref`` is rewritten
      through the map from source positions to new ones, as ``#`` plus the
      percent-encoded pointer.

Running the checks
    :class:`SubprocessRunner` sends the two draft-04 documents as JSON to a
    child process, ``python -I -c <script>``, whose script imports
    jsonsubschema, calls ``isSubschema(lhs, rhs)``, and writes one JSON
    reply. jsonsubschema returns ``None`` instead of ``False`` on some paths
    where inclusion is not shown (``{"type": "number"}`` against
    ``{"type": "integer"}``); like jsonsubschema's own ``isEquivalent``, the
    reply reads ``None`` as "does not hold". The child runs in its own
    session, and a check that exceeds the time limit has its process group
    killed (cause ``timeout``). An exception in jsonsubschema (its own
    unsupported cases, such as an ``enum`` of objects or a negated object
    schema, included), a crash, or an unreadable reply is cause ``error``.
    An interpreter that cannot be started or cannot import jsonsubschema is
    cause ``unavailable``.

    jsonsubschema 0.0.8 is a pinned runtime dependency in ``uv.lock``. Its
    requirements resolve with the project's pins, so by default the child is
    this interpreter (``sys.executable``) in the project environment, and no
    isolated environment is needed. ``SubprocessRunner(python=...)`` points
    the checks at another interpreter, such as a ``uv tool`` environment,
    should the pins ever conflict. jsinfer itself never imports jsonsubschema
    (or the jsonschema package it uses), so the import lint of
    ``tests/lint.py`` still holds.

    A runner is any callable ``runner(lhs, rhs, timeout_s)`` (:data:`CheckRunner`)
    that takes two draft-04 documents and returns whether ``lhs ⊆ rhs``, or a
    :class:`CheckFailure`. Tests inject a stub to exercise the mapping
    without jsonsubschema.

Outcome mapping
    With ``included`` = inferred ⊆ truth and ``includes`` = truth ⊆ inferred:
    both hold, :attr:`Relation.EQUIVALENT`; only ``included``,
    :attr:`Relation.SUBSET`; only ``includes``, :attr:`Relation.SUPERSET`;
    neither, :attr:`Relation.INCOMPARABLE` (31.7). Otherwise the result is
    :class:`Unsupported` with ``cause`` ``"keywords"`` (with the keyword
    names and their uses in each document), ``"error"``, ``"timeout"``, or
    ``"unavailable"`` (31.8), and a message describing it.
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
from bisect import bisect_left
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from enum import Enum
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias
from urllib.parse import quote, unquote

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import canonical_value, distinct_values
from jsinfer.core.schema_model import SUBSCHEMA_KEYWORDS, SubschemaShape, join_pointer, split_pointer
from jsinfer.io.load import ANNOTATION_KEYWORDS
from jsinfer.io.validator import Draft, Validator, dialect_of

__all__ = [
    "BELOW_ROOT",
    "EMBEDDED_RESOURCE",
    "CONTAINER_VALUES",
    "NEGATED_COMPOUND",
    "NEGATED_NUMBER",
    "NON_SCALAR_ITEMS",
    "NOT_LOCAL",
    "NOT_SUPPORTED",
    "OTHER_DIALECT",
    "RECURSIVE",
    "STRIPPED_KEYWORDS",
    "SUPPORTED_KEYWORDS",
    "TIMEOUT_PARAMETER",
    "UNRESOLVED",
    "CheckFailure",
    "CheckRunner",
    "FailureKind",
    "KeywordUse",
    "Relation",
    "SubprocessRunner",
    "SubschemaChecker",
    "Unsupported",
    "UnsupportedCause",
    "to_draft4",
    "unsupported_keywords",
]

# The Resource_Parameter holding the per-check time limit in seconds (31.8).
TIMEOUT_PARAMETER: Final = "subschema.timeout_s"

# ---------------------------------------------------------------------------
# Keyword lists (see "Supported keywords" in the module docstring)
# ---------------------------------------------------------------------------

# Draft 2020-12 keywords the shim maps onto draft-04 keywords that jsonsubschema
# decides. "$ref" and "uniqueItems" are supported only under the conditions of
# the module docstring.
SUPPORTED_KEYWORDS: Final = frozenset(
    {
        "$defs",
        "$ref",
        "additionalProperties",
        "allOf",
        "anyOf",
        "const",
        "enum",
        "exclusiveMaximum",
        "exclusiveMinimum",
        "items",
        "maxItems",
        "maxLength",
        "maxProperties",
        "maximum",
        "minItems",
        "minLength",
        "minProperties",
        "minimum",
        "multipleOf",
        "not",
        "oneOf",
        "pattern",
        "patternProperties",
        "prefixItems",
        "properties",
        "required",
        "type",
        "uniqueItems",
    }
)

# Keywords removed at every schema object: they assert nothing. x- keywords and
# the root $schema (draft 2020-12) and $id are removed too.
STRIPPED_KEYWORDS: Final = frozenset(ANNOTATION_KEYWORDS | {"$anchor", "$dynamicAnchor"})

# Reasons recorded with an unsupported keyword.
NOT_SUPPORTED: Final = "not supported"
OTHER_DIALECT: Final = "declares a dialect other than draft 2020-12"
BELOW_ROOT: Final = "below the root"
EMBEDDED_RESOURCE: Final = "starts an embedded schema resource"
NOT_LOCAL: Final = "not a local JSON Pointer reference"
UNRESOLVED: Final = "target is not a schema position of the document"
RECURSIVE: Final = "recursive reference"
NON_SCALAR_ITEMS: Final = "items may be objects or arrays"
CONTAINER_VALUES: Final = "holds an object or array value, which jsonsubschema cannot canonicalize"
NEGATED_COMPOUND: Final = "negates an object or array schema, which jsonsubschema cannot negate"
NEGATED_NUMBER: Final = "negates a numeric type, which jsonsubschema decides wrongly"
_NUMERIC_TYPES: Final = frozenset({"integer", "number"})
_COMPOUND_KEYWORDS: Final = frozenset(
    {
        "properties", "patternProperties", "additionalProperties", "required", "minProperties", "maxProperties",
        "dependentRequired", "items", "prefixItems", "minItems", "maxItems", "uniqueItems", "contains",
    }
)

# The subschema positions of draft 2020-12, by how each keyword holds them. The
# scan walks all of them; the shim meets only those of supported keywords.
_POSITIONS: Final[Mapping[str, SubschemaShape]] = MappingProxyType(
    {
        **SUBSCHEMA_KEYWORDS,
        "contentSchema": "single",
        "dependentSchemas": "map",
        "unevaluatedItems": "single",
        "unevaluatedProperties": "single",
    }
)

_X_PREFIX: Final = "x-"
_NON_SCALAR_TYPES: Final = frozenset({"array", "object"})
_LIST_KEYWORDS: Final = frozenset({"allOf", "anyOf", "oneOf"})
_MAP_KEYWORDS: Final = frozenset({"patternProperties", "properties"})
_BOUNDS: Final = (("minimum", "exclusiveMinimum", True), ("maximum", "exclusiveMaximum", False))
_BOUND_KEYWORDS: Final = frozenset({"exclusiveMaximum", "exclusiveMinimum", "maximum", "minimum"})
# How deep the scalar-items test looks through allOf/anyOf/oneOf and $ref before giving up.
_SCALAR_DEPTH: Final = 64

# Characters a rewritten "$ref" keeps unescaped: the RFC 3986 fragment
# characters beyond the unreserved ones, which quote() never escapes.
_FRAGMENT_SAFE: Final = "/!$&'()*+,;=:@"

_Tokens: TypeAlias = tuple[str, ...]
_MISSING: Final = object()


# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------


class Relation(Enum):
    """The inclusion relation of an inferred schema to a ground-truth schema (31.7)."""

    EQUIVALENT = "equivalent"  # both inclusions hold
    SUBSET = "subset"  # only inferred ⊆ truth
    SUPERSET = "superset"  # only truth ⊆ inferred
    INCOMPARABLE = "incomparable"  # neither holds

    @classmethod
    def of(cls, included: bool, includes: bool) -> Relation:
        """The relation when inferred ⊆ truth is ``included`` and truth ⊆ inferred is ``includes``."""
        if included:
            return cls.EQUIVALENT if includes else cls.SUBSET
        return cls.SUPERSET if includes else cls.INCOMPARABLE


@dataclass(frozen=True, order=True)
class KeywordUse:
    """One unsupported keyword: the JSON Pointer of its schema object (``""`` for the root), its name, and why."""

    pointer: str
    keyword: str
    reason: str

    def __str__(self) -> str:
        return f"{self.keyword} at {self.pointer or '(root)'} ({self.reason})"


UnsupportedCause: TypeAlias = Literal["keywords", "error", "timeout", "unavailable"]
FailureKind: TypeAlias = Literal["error", "timeout", "unavailable"]
_CAUSES: Final = ("keywords", "error", "timeout", "unavailable")


@dataclass(frozen=True)
class Unsupported:
    """An inclusion relation that cannot be reported, with its cause (31.8).

    ``cause`` is ``"keywords"`` when a schema holds keywords the checker
    does not support: ``keywords`` lists their distinct names in code-point
    order, and ``inferred`` and ``truth`` hold every use in each schema,
    sorted. ``"error"`` means a check raised an error or a schema is not a
    valid draft 2020-12 schema, ``"timeout"`` that a check exceeded the time
    limit, and ``"unavailable"`` that jsonsubschema could not be run.
    ``message`` describes the cause.
    """

    cause: UnsupportedCause
    message: str
    keywords: tuple[str, ...] = ()
    inferred: tuple[KeywordUse, ...] = ()
    truth: tuple[KeywordUse, ...] = ()

    def __post_init__(self) -> None:
        if self.cause not in _CAUSES:
            raise ValueError(f"Unsupported cause must be one of {_CAUSES}, got {self.cause!r}")
        if not isinstance(self.message, str) or not self.message:
            raise ValueError("Unsupported message must be a non-empty str")
        if (self.cause == "keywords") != bool(self.keywords):
            raise ValueError("Unsupported lists keywords exactly when its cause is 'keywords'")

    @classmethod
    def from_uses(cls, inferred: list[KeywordUse], truth: list[KeywordUse]) -> Unsupported:
        """The result for the unsupported keywords found in each schema (at least one)."""
        names = sorted({use.keyword for use in [*inferred, *truth]}, key=codepoint_key)
        parts = [f"unsupported keywords {', '.join(names)}"]
        for side, uses in (("inferred", inferred), ("truth", truth)):
            if uses:
                parts.append(f"{side} schema: {', '.join(str(use) for use in sorted(uses))}")
        return cls("keywords", "; ".join(parts), tuple(names), tuple(sorted(inferred)), tuple(sorted(truth)))


@dataclass(frozen=True)
class CheckFailure:
    """A check that did not complete: ``kind`` is ``"error"``, ``"timeout"``, or ``"unavailable"``."""

    kind: FailureKind
    message: str

    def __post_init__(self) -> None:
        if self.kind not in _CAUSES[1:]:
            raise ValueError(f"CheckFailure kind must be one of {_CAUSES[1:]}, got {self.kind!r}")
        if not isinstance(self.message, str) or not self.message:
            raise ValueError("CheckFailure message must be a non-empty str")


# runner(lhs, rhs, timeout_s): whether lhs ⊆ rhs for two draft-04 documents, or why the check did not complete.
CheckRunner: TypeAlias = Callable[[Any, Any, float], bool | CheckFailure]


# ---------------------------------------------------------------------------
# Pointers
# ---------------------------------------------------------------------------


def _local_target(ref: Any) -> _Tokens | None:
    """The pointer tokens of a local ``$ref`` (``#`` plus a JSON Pointer), or None for any other value."""
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None  # not a string, or remote or relative
    pointer = unquote(ref[1:])
    if pointer and not pointer.startswith("/"):
        return None  # a plain-name fragment (an anchor)
    try:
        return tuple(split_pointer(pointer))
    except ValueError:
        return None


def _resolve(doc: Any, tokens: _Tokens) -> Any:
    """The value at ``tokens`` in ``doc``, or ``_MISSING``; array indices must be canonical decimals."""
    node = doc
    for token in tokens:
        if isinstance(node, dict):
            if token not in node:
                return _MISSING
            node = node[token]
        elif isinstance(node, list):
            if not (token.isascii() and token.isdigit() and (token == "0" or token[0] != "0")):
                return _MISSING
            index = int(token)
            if index >= len(node):
                return _MISSING
            node = node[index]
        else:
            return _MISSING
    return node


def _children(keyword: str, value: Any) -> list[tuple[str, Any]]:
    """The ``(token, subschema)`` pairs a subschema keyword holds; none when the value has another shape."""
    shape = _POSITIONS.get(keyword)
    if shape == "single":
        return [("", value)] if isinstance(value, (dict, bool)) else []
    if shape == "list" and isinstance(value, list):
        return [(str(i), item) for i, item in enumerate(value) if isinstance(item, (dict, bool))]
    if shape == "map" and isinstance(value, dict):
        return [(name, item) for name, item in value.items() if isinstance(item, (dict, bool))]
    return []


def _extend(tokens: _Tokens, keyword: str, token: str) -> _Tokens:
    shape = _POSITIONS[keyword]
    return (*tokens, keyword) if shape == "single" else (*tokens, keyword, token)


# ---------------------------------------------------------------------------
# Keyword scan
# ---------------------------------------------------------------------------


def _is_2020_12(uri: Any) -> bool:
    draft, assumed = dialect_of({"$schema": uri})
    return draft is Draft.DRAFT202012 and not assumed


def _keyword_reason(keyword: str, value: Any, root: bool) -> str | None:
    """Why a keyword is unsupported regardless of the rest of the document, or None."""
    if keyword.startswith(_X_PREFIX) or keyword in STRIPPED_KEYWORDS:
        return None
    if keyword == "$schema":
        if not root:
            return BELOW_ROOT
        return None if _is_2020_12(value) else OTHER_DIALECT
    if keyword == "$id":
        return None if root else EMBEDDED_RESOURCE
    return None if keyword in SUPPORTED_KEYWORDS else NOT_SUPPORTED


def _numeric_type(declared: Any) -> bool:
    names = [declared] if isinstance(declared, str) else declared if isinstance(declared, list) else []
    return any(name in _NUMERIC_TYPES for name in names)


def unsupported_keywords(schema: Any) -> list[KeywordUse]:
    """Every keyword of a draft 2020-12 document that the checker does not support, sorted.

    See "Supported keywords" in the module docstring. Values of an
    unexpected shape are not walked (the meta-schema check reports them).
    """
    found: list[KeywordUse] = []
    positions: set[_Tokens] = set()
    refs: list[tuple[_Tokens, Any]] = []  # (holder, $ref value)
    unique: list[tuple[_Tokens, dict[str, Any]]] = []  # holders of uniqueItems: true
    stack: list[tuple[Any, _Tokens]] = [(schema, ())]
    while stack:
        value, tokens = stack.pop()
        positions.add(tokens)
        if not isinstance(value, dict):
            continue
        pointer = join_pointer("", *tokens)
        for keyword, item in value.items():
            reason = _keyword_reason(keyword, item, not tokens)
            if reason is not None:
                found.append(KeywordUse(pointer, keyword, reason))
            elif keyword == "$ref":
                refs.append((tokens, item))
            elif keyword == "uniqueItems" and item is True:
                unique.append((tokens, value))
            elif keyword == "const" and isinstance(item, (dict, list)):
                found.append(KeywordUse(pointer, keyword, CONTAINER_VALUES))
            elif keyword == "enum" and isinstance(item, list) and any(isinstance(v, (dict, list)) for v in item):
                found.append(KeywordUse(pointer, keyword, CONTAINER_VALUES))
            elif keyword == "not" and isinstance(item, dict) and any(k in _COMPOUND_KEYWORDS for k in item):
                found.append(KeywordUse(pointer, keyword, NEGATED_COMPOUND))
            elif keyword == "not" and isinstance(item, dict) and _numeric_type(item.get("type")):
                found.append(KeywordUse(pointer, keyword, NEGATED_NUMBER))
            if keyword in _POSITIONS:
                stack.extend((child, _extend(tokens, keyword, token)) for token, child in _children(keyword, item))

    resolved: list[tuple[_Tokens, _Tokens]] = []
    for holder, ref in refs:
        target = _local_target(ref)
        pointer = join_pointer("", *holder)
        if target is None:
            found.append(KeywordUse(pointer, "$ref", NOT_LOCAL))
        elif target not in positions:
            found.append(KeywordUse(pointer, "$ref", UNRESOLVED))
        else:
            resolved.append((holder, target))
    found.extend(KeywordUse(join_pointer("", *holder), "$ref", RECURSIVE) for holder in _recursive(resolved))

    for holder, value in unique:
        if not _scalar_items(value, schema):
            found.append(KeywordUse(join_pointer("", *holder), "uniqueItems", NON_SCALAR_ITEMS))
    return sorted(found)


def _recursive(refs: list[tuple[_Tokens, _Tokens]]) -> list[_Tokens]:
    """The holders of the references whose expansion never ends.

    Reference ``a`` leads to reference ``b`` when ``b`` sits inside ``a``'s
    target (the target itself included). Peeling references that lead to no
    unpeeled one leaves exactly those that lead into a cycle.
    """
    order = sorted(range(len(refs)), key=lambda i: refs[i][0])
    holders = [refs[i][0] for i in order]
    leads_to: list[list[int]] = [[] for _ in refs]  # positions in ``order``
    led_from: list[list[int]] = [[] for _ in refs]
    for a, i in enumerate(order):
        target = refs[i][1]
        # Holders inside the target are contiguous in sorted tuple order, starting at the target itself.
        b = bisect_left(holders, target)
        while b < len(holders) and holders[b][: len(target)] == target:
            leads_to[a].append(b)
            led_from[b].append(a)
            b += 1
    pending = [len(edges) for edges in leads_to]
    queue = [a for a, count in enumerate(pending) if count == 0]
    while queue:
        b = queue.pop()
        for a in led_from[b]:
            pending[a] -= 1
            if pending[a] == 0:
                queue.append(a)
    return [holders[a] for a, count in enumerate(pending) if count > 0]


def _scalar_items(array: dict[str, Any], doc: Any) -> bool:
    """Whether every item an array schema admits is a scalar (see the module docstring)."""
    declared = array.get("type")
    if isinstance(declared, str):
        declared = [declared]
    if isinstance(declared, list) and "array" not in declared:
        return True  # uniqueItems has no effect on a schema that admits no array
    entries = array.get("prefixItems", [])
    if not isinstance(entries, list) or "items" not in array:
        return False  # without items, any value may follow the prefix
    return all(_only_scalars(entry, doc, 0) for entry in [*entries, array["items"]])


def _only_scalars(schema: Any, doc: Any, depth: int) -> bool:
    """Whether ``schema`` admits only scalars; False when that cannot be shown within ``_SCALAR_DEPTH`` steps."""
    if schema is False:
        return True
    if not isinstance(schema, dict) or depth >= _SCALAR_DEPTH:
        return False
    declared = schema.get("type")
    if isinstance(declared, str) and declared not in _NON_SCALAR_TYPES:
        return True
    if isinstance(declared, list) and declared and not any(t in _NON_SCALAR_TYPES for t in declared):
        return True
    if "const" in schema and not isinstance(schema["const"], (dict, list)):
        return True
    values = schema.get("enum")
    if isinstance(values, list) and not any(isinstance(v, (dict, list)) for v in values):
        return True
    branches = schema.get("allOf")
    if isinstance(branches, list) and any(_only_scalars(b, doc, depth + 1) for b in branches):
        return True
    for keyword in ("anyOf", "oneOf"):
        branches = schema.get(keyword)
        if isinstance(branches, list) and branches and all(_only_scalars(b, doc, depth + 1) for b in branches):
            return True
    target = _local_target(schema.get("$ref"))
    if target is not None:
        referent = _resolve(doc, target)
        return referent is not _MISSING and _only_scalars(referent, doc, depth + 1)
    return False


# ---------------------------------------------------------------------------
# The draft-04 shim
# ---------------------------------------------------------------------------


def _number(keyword: str, value: Any) -> int | float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{keyword} must be a number, got {value!r}")
    return value


def _bound(schema: dict[str, Any], inclusive: str, exclusive: str, lower: bool) -> tuple[Any, bool] | None:
    """The tighter of a 2020-12 inclusive and exclusive bound, as ``(value, exclusive)``, or None."""
    candidates = [
        (_number(keyword, schema[keyword]), keyword == exclusive) for keyword in (inclusive, exclusive) if keyword in schema
    ]
    if not candidates:
        return None
    if len(candidates) == 1:
        return candidates[0]
    (inc, _), (exc, _) = candidates
    tighter = exc >= inc if lower else exc <= inc
    return (exc, True) if tighter else (inc, False)


# A schema position waiting to be translated: (value, source tokens, new tokens,
# the container to fill, its slot, whether a boolean stays a boolean there).
_Task: TypeAlias = tuple[Any, _Tokens, _Tokens, Any, Any, bool]


def to_draft4(schema: dict[str, Any] | bool) -> dict[str, Any]:
    """The draft-04 shape of a supported draft 2020-12 document (see "The draft-04 shim").

    Raises ``ValueError`` for a keyword outside :data:`SUPPORTED_KEYWORDS`
    and the stripped ones, a ``$ref`` that is not a local JSON Pointer to a
    schema position, and a value whose shape draft 2020-12 does not allow
    where the shim needs it; :func:`unsupported_keywords` and the meta-schema
    check rule these out.
    """
    if not isinstance(schema, (dict, bool)):
        raise TypeError(f"a schema must be a dict or a bool, got {type(schema).__name__}")
    moved: dict[_Tokens, _Tokens] = {}  # source position -> new position
    refs: list[tuple[dict[str, Any], _Tokens, str]] = []  # ({"$ref": ...} to fill, source target, holder pointer)
    holder: list[Any] = [None]
    stack: list[_Task] = [(schema, (), (), holder, 0, False)]
    while stack:
        value, source, new, container, slot, keep_bool = stack.pop()
        moved[source] = new
        if isinstance(value, bool):
            container[slot] = value if keep_bool else ({} if value else {"not": {}})
        elif isinstance(value, dict):
            container[slot] = _translate_object(value, source, new, stack, refs)
        else:
            where = join_pointer("", *source) or "(root)"
            raise ValueError(f"the subschema at {where} is neither an object nor a boolean")
    for ref, target, pointer in refs:
        if target not in moved:
            raise ValueError(f"the $ref at {pointer or '(root)'} does not point to a schema position")
        ref["$ref"] = "#" + quote(join_pointer("", *moved[target]), safe=_FRAGMENT_SAFE)
    return holder[0]


def _translate_object(
    value: dict[str, Any],
    source: _Tokens,
    new: _Tokens,
    stack: list[_Task],
    refs: list[tuple[dict[str, Any], _Tokens, str]],
) -> dict[str, Any]:
    """One schema object in draft-04 shape; its subschemas are queued on ``stack``."""
    pointer = join_pointer("", *source)
    out: dict[str, Any] = {}
    conjuncts: list[dict[str, Any]] = []  # appended to allOf
    has_prefix = "prefixItems" in value

    def single(keyword: str, item: Any, name: str, keep_bool: bool) -> None:
        out[name] = None
        stack.append((item, (*source, keyword), (*new, name), out, name, keep_bool))

    def many(keyword: str, item: Any, name: str) -> None:
        if isinstance(item, list):
            children: Any = [None] * len(item)
            entries = [(str(i), child) for i, child in enumerate(item)]
        elif isinstance(item, dict):
            children = dict.fromkeys(item)
            entries = list(item.items())
        else:
            raise ValueError(f"{keyword} at {pointer or '(root)'} must be an array or an object")
        out[name] = children
        for token, child in entries:
            slot: Any = int(token) if isinstance(children, list) else token
            stack.append((child, (*source, keyword, token), (*new, name, token), children, slot, False))

    for keyword, item in value.items():
        if keyword.startswith(_X_PREFIX) or keyword in STRIPPED_KEYWORDS:
            continue
        if keyword in ("$schema", "$id") and not source:
            continue
        if keyword not in SUPPORTED_KEYWORDS:
            raise ValueError(f"{keyword} at {pointer or '(root)'} is not a supported keyword")
        if keyword in _BOUND_KEYWORDS or keyword in ("$ref", "const"):
            continue  # written below
        if keyword == "$defs":
            many(keyword, item, "definitions")
        elif keyword == "prefixItems":
            many(keyword, item, "items")
        elif keyword == "items":
            single(keyword, item, "additionalItems" if has_prefix else "items", has_prefix)
        elif keyword == "additionalProperties":
            single(keyword, item, keyword, True)
        elif keyword == "not":
            single(keyword, item, keyword, False)
        elif keyword in _LIST_KEYWORDS or keyword in _MAP_KEYWORDS:
            many(keyword, item, keyword)
        elif keyword == "enum":
            if not isinstance(item, list):
                raise ValueError(f"enum at {pointer or '(root)'} must be an array")
            out[keyword] = distinct_values(item)
        else:
            out[keyword] = canonical_value(item)

    for inclusive, exclusive, lower in _BOUNDS:
        bound = _bound(value, inclusive, exclusive, lower)
        if bound is not None:
            out[inclusive] = canonical_value(bound[0])
            if bound[1]:
                out[exclusive] = True
    if "const" in value:
        enum = [canonical_value(value["const"])]
        if "enum" in value:
            conjuncts.append({"enum": enum})
        else:
            out["enum"] = enum
    if "$ref" in value:
        target = _local_target(value["$ref"])
        if target is None:
            raise ValueError(f"the $ref at {pointer or '(root)'} is not a local JSON Pointer reference")
        ref: dict[str, Any] = {"$ref": None}
        refs.append((ref, target, pointer))
        if not out and not conjuncts:
            return ref
        conjuncts.append(ref)
    if conjuncts:
        out.setdefault("allOf", []).extend(conjuncts)
    return out


# ---------------------------------------------------------------------------
# Running isSubschema in a child process
# ---------------------------------------------------------------------------

# The child's script: read {"lhs", "rhs"} from stdin, write one JSON reply to stdout.
_CHILD_SCRIPT: Final = r"""
import json
import sys


def main():
    reply_to = sys.stdout
    sys.stdout = sys.stderr  # whatever jsonsubschema prints stays out of the reply
    request = json.load(sys.stdin)
    try:
        from jsonsubschema import isSubschema
    except Exception as exc:
        reply = {"unavailable": f"{type(exc).__name__}: {exc}"}
    else:
        try:
            result = isSubschema(request["lhs"], request["rhs"])
        except (Exception, SystemExit) as exc:
            name = f"{type(exc).__module__}.{type(exc).__qualname__}"
            reply = {"error": f"{name}: {exc}"}
        else:
            if result is True or result is False or result is None:
                reply = {"holds": result is True}
            else:
                reply = {"error": f"isSubschema returned a {type(result).__name__}, not a boolean"}
    reply_to.write(json.dumps(reply) + "\n")
    reply_to.flush()


main()
"""

# Longest error text kept from a child, in characters.
_MAX_MESSAGE: Final = 1_000


def _clip(text: str) -> str:
    text = text.strip()
    return text if len(text) <= _MAX_MESSAGE else text[: _MAX_MESSAGE - 1] + "…"


class SubprocessRunner:
    """Runs each ``isSubschema(lhs, rhs)`` in a fresh child process (see "Running the checks").

    ``python`` is the interpreter to run, by default this one
    (``sys.executable``), whose environment holds the pinned jsonsubschema.
    """

    __slots__ = ("python",)

    def __init__(self, python: str | os.PathLike[str] | None = None) -> None:
        self.python = sys.executable if python is None else os.fspath(python)
        if not self.python:
            raise ValueError("python must name an interpreter")

    def __repr__(self) -> str:
        return f"SubprocessRunner(python={self.python!r})"

    def __call__(self, lhs: Any, rhs: Any, timeout_s: float) -> bool | CheckFailure:
        try:
            payload = json.dumps({"lhs": lhs, "rhs": rhs}, allow_nan=False).encode("ascii")
        except (TypeError, ValueError) as exc:
            return CheckFailure("error", f"the schemas cannot be sent to the check process: {exc}")
        try:
            process = subprocess.Popen(
                [self.python, "-I", "-c", _CHILD_SCRIPT],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except OSError as exc:
            return CheckFailure("unavailable", f"cannot start {self.python}: {exc}")
        try:
            stdout, stderr = process.communicate(payload, timeout=timeout_s)
        except subprocess.TimeoutExpired:
            _kill(process)
            return CheckFailure("timeout", f"the check did not finish within {timeout_s:g} s")
        except BaseException:
            _kill(process)
            raise
        return self._reply(process.returncode, stdout, stderr)

    def _reply(self, returncode: int, stdout: bytes, stderr: bytes) -> bool | CheckFailure:
        lines = stdout.decode("utf-8", "replace").strip().splitlines()
        try:
            reply = json.loads(lines[-1]) if lines else None
        except ValueError:
            reply = None
        if returncode == 0 and isinstance(reply, dict) and len(reply) == 1:
            if isinstance(reply.get("holds"), bool):
                return reply["holds"]
            if isinstance(reply.get("error"), str):
                return CheckFailure("error", _clip(reply["error"]) or "jsonsubschema raised an error")
            if isinstance(reply.get("unavailable"), str):
                detail = _clip(reply["unavailable"])
                return CheckFailure("unavailable", f"{self.python} cannot import jsonsubschema: {detail}")
        tail = _clip(stderr.decode("utf-8", "replace")[-_MAX_MESSAGE:])
        status = f"the check process exited with status {returncode}"
        return CheckFailure("error", f"{status}: {tail}" if tail else status)


def _kill(process: subprocess.Popen[bytes]) -> None:
    """Kill a check process and its process group, and reap it."""
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except (OSError, AttributeError):  # already gone, or no process groups on this platform
        process.kill()
    process.communicate()


# ---------------------------------------------------------------------------
# The checker
# ---------------------------------------------------------------------------


class SubschemaChecker:
    """Decides the inclusion relation between an inferred and a ground-truth schema (see the module docstring).

    ``runner`` runs one inclusion check (:data:`CheckRunner`), by default a
    :class:`SubprocessRunner`; ``validator`` performs the meta-schema check.
    """

    __slots__ = ("_runner", "_validator")

    def __init__(self, runner: CheckRunner | None = None, *, validator: Validator | None = None) -> None:
        if runner is not None and not callable(runner):
            raise TypeError("runner must be callable")
        self._runner: CheckRunner = SubprocessRunner() if runner is None else runner
        self._validator = Validator() if validator is None else validator

    @property
    def runner(self) -> CheckRunner:
        return self._runner

    def relation(
        self, inferred: dict[str, Any] | bool, truth: dict[str, Any] | bool, timeout_s: float | None = None
    ) -> Relation | Unsupported:
        """The relation of ``inferred`` to ``truth``, or why it cannot be reported (31.7, 31.8).

        ``timeout_s`` is the per-check time limit in seconds, by default the
        ``subschema.timeout_s`` Resource_Parameter's default; an invalid one
        raises ``ValueError``. A root that is neither an object nor a boolean
        raises ``TypeError``.
        """
        spec = RESOURCE_PARAMETERS[TIMEOUT_PARAMETER]
        limit = float(spec.default_value() if timeout_s is None else spec.normalize(timeout_s))
        schemas = (("inferred", inferred), ("truth", truth))
        for side, schema in schemas:
            if not isinstance(schema, (dict, bool)):
                raise TypeError(f"the {side} schema must be a dict or a bool, got {type(schema).__name__}")
        found = [unsupported_keywords(schema) for _, schema in schemas]
        if found[0] or found[1]:
            return Unsupported.from_uses(found[0], found[1])
        for side, schema in schemas:
            errors = self._validator.meta_errors(schema, Draft.DRAFT202012)
            if errors:
                more = f" (and {len(errors) - 1} more)" if len(errors) > 1 else ""
                return Unsupported("error", f"the {side} schema is not a valid draft 2020-12 schema: {errors[0]}{more}")
        lhs, rhs = to_draft4(inferred), to_draft4(truth)
        included = self._check(lhs, rhs, limit, "inferred", "truth")
        if isinstance(included, Unsupported):
            return included
        includes = self._check(rhs, lhs, limit, "truth", "inferred")
        if isinstance(includes, Unsupported):
            return includes
        return Relation.of(included, includes)

    def _check(self, lhs: Any, rhs: Any, limit: float, sub: str, sup: str) -> bool | Unsupported:
        outcome = self._runner(lhs, rhs, limit)
        if isinstance(outcome, CheckFailure):
            check = f"checking that the {sub} schema is a subschema of the {sup} schema"
            return Unsupported(outcome.kind, f"{check}: {outcome.message}")
        if not isinstance(outcome, bool):
            raise TypeError(f"the runner must return a bool or a CheckFailure, got {type(outcome).__name__}")
        return outcome
