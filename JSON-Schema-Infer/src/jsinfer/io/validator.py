"""Validator: the one wrapper over jsonschema-rs (Requirements 3.2, 13.4, 29.3, 29.10).

Every accept or reject verdict for a JSON document against a JSON Schema
comes from :class:`Validator` (29.3). That covers the soundness loop, Format
and Pattern checks, the Schema_Loader's meta-schema check, the
Synthetic_Generator, and the Evaluation_Harness. No other validator is
imported anywhere under ``src/`` (``tests/lint.py`` checks this).

Dialects
    :func:`dialect_of` picks the draft from the root ``$schema``. URIs are
    compared after mapping ``http://`` to ``https://`` and dropping one
    trailing ``#``. With no ``$schema``, a non-string one, an unrecognized
    URI, or a boolean root, the draft is 2020-12 and is marked as assumed
    (2.11). :meth:`Validator.compile` builds that draft's jsonschema-rs class
    directly, so an unrecognized ``$schema`` never makes jsonschema-rs look
    for a custom meta-schema.

Options
    Every jsonschema-rs validator is built with ``validate_formats=True``
    (``format`` is an assertion, 3.2 and 13.4), the default regex engine
    (fancy-regex, which supports lookaround; no ``pattern_options``), and
    ``offline=True``, so nothing is ever fetched over the network: a remote
    ``$ref`` is a compile failure, and results do not depend on what a server
    returns. Unknown ``format`` names are ignored, which is jsonschema-rs's
    default.

Numbers
    jsonschema-rs compares Python integers of any size exactly, but it
    compares an integral float with an integer by the float's shortest
    round-trip decimal (``repr``), not by its exact value. Below 2**53 in
    magnitude that decimal is exact; above it, ``float(2**62)`` does not
    equal ``2**62`` while ``1e40`` equals ``10**40``
    (``tests/unit/test_validator_edge_cases.py`` pins this). From draft-06
    on, JSON Schema compares numbers by mathematical value and counts every
    number with a zero fractional part as an integer, as
    :mod:`jsinfer.core.jsonvalue` does. For those drafts the Validator hands
    jsonschema-rs every integral float in a schema, and every integral float
    of magnitude at least 2**53 in an instance, as the exact ``int`` it
    equals. The verdicts then follow mathematical equality, and error
    messages print those numbers as integers. An instance without such a
    float is passed as is; a scan of its orjson encoding tells most such
    instances apart without walking them in Python.

    Draft-04 counts only numbers written without a fraction or exponent as
    integers, and jsonschema-rs follows it (``1.0`` is not an ``integer``,
    and ``"maxLength": 2.0`` does not compile). Draft-04 schemas and their
    instances are therefore passed exactly as given.

Compilation and caching
    :meth:`Validator.compile` keys compiled validators by the SHA-256 digest of
    ``core.determinism.canonical_bytes(schema)``, so schemas that are equal
    JSON values (whatever their key order, ``1`` or ``1.0``) share one
    compiled validator. Under draft-04 the key also tells apart how
    integral numbers are written, because jsonschema-rs compiles ``1`` and
    ``1.0`` differently there. Either way, schemas with one key compile to
    the same behavior, so a verdict never depends on which schema was
    compiled first. The cache is LRU-bounded, and failures are cached as
    well. A schema that jsonschema-rs cannot compile raises
    :class:`CompileError`, which names the schema and the cause and gives no
    verdict for any document (29.10).

    jsonschema-rs validators cannot be shared across processes. A
    :class:`Compiled` therefore pickles as its schema and is compiled again
    when it is unpickled, and a :class:`Validator` pickles with empty caches,
    so each worker rebuilds the validators it uses.

Error locations
    For each error, jsonschema-rs reports an *evaluation path*: the keywords
    traversed from the root, with one ``$ref`` token per reference followed.
    Its ``schema_path`` is the failing keyword's location relative to the
    schema *resource* that holds it, which differs from the location in the
    document when the keyword sits in an embedded resource (a subschema with
    its own ``$id``). :meth:`Validator.errors` converts the evaluation path
    into the absolute keyword location, a JSON Pointer from the root of the
    compiled document. It walks the document along the path and, at each
    ``$ref`` token, follows the reference to its target:

    - the reference is resolved against the current base URI. ``$id`` (``id``
      in draft-04) on a subschema starts a new resource with a new base URI.
      For drafts 04 through 07, an ``$id`` next to ``$ref`` is ignored, as
      the drafts require;
    - a fragment that is empty or starts with ``/`` is a JSON Pointer
      (percent-decoded, then RFC 6901) from the root of the target resource.
      Any other fragment is a plain-name anchor: ``$anchor`` (and
      ``$dynamicAnchor`` in 2020-12), or a fragment-only ``$id`` in drafts 04
      through 07.

    When the walk cannot be followed inside the document (a ``$dynamicRef`` or
    ``$recursiveRef`` token, whose target depends on the dynamic scope, or a
    path that does not lead to a location in the document), the error keeps
    jsonschema-rs's ``schema_path``. That location is exact for documents
    without embedded resources.

Format and Pattern checks
    :meth:`Validator.satisfies_format` is the verdict of the compiled schema
    ``{"format": fmt}`` for one string, and :meth:`Validator.matches_pattern`
    that of ``{"pattern": pattern}``, so they agree with the verdicts on an
    emitted schema that holds the keyword. Results are cached per distinct
    ``(format or pattern, string)``, LRU-bounded.

Meta-schema checks
    :meth:`Validator.meta_errors` validates a schema document against the
    meta-schema of a given draft. jsonschema-rs bundles the five meta-schemas;
    each check uses that draft's class on ``{"$ref": <meta-schema URI>}``,
    with the options above. Formats are asserted in the meta-schema check
    too, which matches what compilation enforces in every draft (``pattern``
    must be a valid regex; ``$id``, ``$ref``, and ``$schema`` must be valid
    URIs or URI references). ``jsonschema_rs.meta.validate`` is not used: it
    reports only the first error, does not assert formats for 2019-09 and
    2020-12, and fetches an unrecognized ``$schema`` URI over the network.
"""

from __future__ import annotations

import copy
import hashlib
import json
import re
from collections import OrderedDict
from collections.abc import Hashable, Sequence
from dataclasses import dataclass
from enum import Enum
from typing import Any, Final
from urllib.parse import unquote, urldefrag, urljoin

import jsonschema_rs
import orjson

from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.schema_model import join_pointer, split_pointer

__all__ = [
    "DEFAULT_MAX_COMPILED",
    "DEFAULT_MAX_VALUES",
    "CompileError",
    "Compiled",
    "Draft",
    "ErrorInfo",
    "Validator",
    "dialect_of",
]

# Cache bounds. They change speed only, never a verdict.
DEFAULT_MAX_COMPILED: Final = 1024
DEFAULT_MAX_VALUES: Final = 1 << 16


# ---------------------------------------------------------------------------
# Dialects
# ---------------------------------------------------------------------------


class Draft(Enum):
    """A recognized JSON Schema dialect; the value is its meta-schema URI."""

    DRAFT4 = "http://json-schema.org/draft-04/schema#"
    DRAFT6 = "http://json-schema.org/draft-06/schema#"
    DRAFT7 = "http://json-schema.org/draft-07/schema#"
    DRAFT201909 = "https://json-schema.org/draft/2019-09/schema"
    DRAFT202012 = "https://json-schema.org/draft/2020-12/schema"

    @property
    def uri(self) -> str:
        """The meta-schema URI, as the draft publishes it."""
        return self.value


_VALIDATOR_CLASSES: Final[dict[Draft, Any]] = {
    Draft.DRAFT4: jsonschema_rs.Draft4Validator,
    Draft.DRAFT6: jsonschema_rs.Draft6Validator,
    Draft.DRAFT7: jsonschema_rs.Draft7Validator,
    Draft.DRAFT201909: jsonschema_rs.Draft201909Validator,
    Draft.DRAFT202012: jsonschema_rs.Draft202012Validator,
}

# Drafts in which "$ref" overrides its sibling keywords, "$id" included.
_REF_IGNORES_SIBLINGS: Final = frozenset({Draft.DRAFT4, Draft.DRAFT6, Draft.DRAFT7})


def _normalize_dialect_uri(uri: str) -> str:
    if uri.startswith("http://"):
        uri = "https://" + uri[len("http://") :]
    return uri[:-1] if uri.endswith("#") else uri


_DRAFTS_BY_URI: Final[dict[str, Draft]] = {_normalize_dialect_uri(draft.uri): draft for draft in Draft}


def dialect_of(schema: dict[str, Any] | bool) -> tuple[Draft, bool]:
    """Return ``(draft, assumed)`` for a schema document (see the module docstring).

    ``assumed`` is ``True`` when the root declares no recognized ``$schema``,
    in which case the draft is 2020-12 (Requirement 2.11).
    """
    if isinstance(schema, dict):
        declared = schema.get("$schema")
        if isinstance(declared, str):
            draft = _DRAFTS_BY_URI.get(_normalize_dialect_uri(declared))
            if draft is not None:
                return draft, False
    return Draft.DRAFT202012, True


def _build(draft: Draft, schema: dict[str, Any] | bool) -> Any:
    """Build a jsonschema-rs validator with the Validator's fixed options."""
    return _VALIDATOR_CLASSES[draft](schema, validate_formats=True, offline=True)


# ---------------------------------------------------------------------------
# Numbers (see the module docstring)
# ---------------------------------------------------------------------------

# Below this magnitude the shortest decimal of an integral float is its exact
# value, so jsonschema-rs compares it exactly.
_INEXACT_FLOAT_MIN: Final = float(2**53)

# orjson writes floats as repr does: positionally, with a ".0" suffix when
# integral, below 1e16 in magnitude, and with an "e+" exponent from 1e16 up.
# A float of magnitude at least 2**53 (16 digits) therefore shows either a
# point after 16 digits or "e+". Checked on the bytes, so that most instances
# skip the walk in Python.
_SIXTEEN_DIGITS_THEN_POINT: Final = re.compile(rb"\.(?<=[0-9]{16}\.)")


def _may_hold_inexact_float(encoded: bytes) -> bool:
    return b"e+" in encoded or _SIXTEEN_DIGITS_THEN_POINT.search(encoded) is not None


def _rewrites(item: Any, minimum: float) -> bool:
    return isinstance(item, float) and abs(item) >= minimum and item.is_integer()


def _with_exact_integers(value: Any, minimum: float) -> Any:
    """``value`` with every integral float of magnitude at least ``minimum`` written as the int it equals.

    Returns ``value`` itself when there is no such float. Otherwise returns a
    copy that keeps member order and writes tuples as lists. Both walks use
    an explicit stack, so deep nesting cannot overflow.
    """
    pending: list[Any] = [value]
    while pending:
        item = pending.pop()
        if _rewrites(item, minimum):
            break
        if isinstance(item, (list, tuple)):
            pending.extend(item)
        elif isinstance(item, dict):
            pending.extend(item.values())
    else:
        return value
    root: list[Any] = [None]
    copying: list[tuple[Any, Any, Any]] = [(value, root, 0)]  # (item, its container in the copy, slot)
    while copying:
        item, target, slot = copying.pop()
        if _rewrites(item, minimum):
            item = int(item)
        elif isinstance(item, (list, tuple)):
            items: list[Any] = [None] * len(item)
            copying.extend((child, items, index) for index, child in enumerate(item))
            item = items
        elif isinstance(item, dict):
            members: dict[str, Any] = dict.fromkeys(item)
            copying.extend((child, members, key) for key, child in item.items())
            item = members
        target[slot] = item
    return root[0]


def _rs_schema(schema: dict[str, Any] | bool, draft: Draft) -> dict[str, Any] | bool:
    """The schema as jsonschema-rs gets it: every integral float exact, except under draft-04."""
    return schema if draft is Draft.DRAFT4 else _with_exact_integers(schema, 0.0)


def _rs_instance(instance: Any, draft: Draft) -> Any:
    """The instance as jsonschema-rs gets it: integral floats from 2**53 up exact, except under draft-04."""
    if draft is Draft.DRAFT4:
        return instance
    try:
        encoded = orjson.dumps(instance)
    except TypeError:  # integers beyond 64 bits, nesting deeper than orjson allows, non-JSON types
        pass
    else:
        if not _may_hold_inexact_float(encoded):
            return instance
    return _with_exact_integers(instance, _INEXACT_FLOAT_MIN)


def _float_spellings(schema: dict[str, Any] | bool) -> list[list[str | int]]:
    """The locations of the integral floats in a schema, sorted (part of draft-04 cache keys)."""
    found: list[list[str | int]] = []
    pending: list[tuple[Any, list[str | int]]] = [(schema, [])]
    while pending:
        item, path = pending.pop()
        if _rewrites(item, 0.0):
            found.append(path)
        elif isinstance(item, (list, tuple)):
            pending.extend((child, [*path, index]) for index, child in enumerate(item))
        elif isinstance(item, dict):
            pending.extend((child, [*path, key]) for key, child in item.items())
    return sorted(found, key=canonical_bytes)


# ---------------------------------------------------------------------------
# Results and errors
# ---------------------------------------------------------------------------


class CompileError(ValueError):
    """jsonschema-rs could not compile a schema (Requirement 29.10).

    ``schema_id`` names the schema: the name passed to
    :meth:`Validator.compile`, else the root ``$id``, else a digest prefix.
    ``cause`` describes the failure. ``digest`` is the SHA-256 hex digest of
    the schema's canonical bytes, or ``None`` when the schema is not a JSON
    value.
    """

    def __init__(self, schema_id: str, cause: str, digest: str | None = None) -> None:
        if not isinstance(schema_id, str) or not schema_id:
            raise ValueError("CompileError schema_id must be a non-empty str")
        if not isinstance(cause, str) or not cause:
            raise ValueError("CompileError cause must be a non-empty str")
        super().__init__(schema_id, cause, digest)  # args survive pickling
        self.schema_id = schema_id
        self.cause = cause
        self.digest = digest

    def __str__(self) -> str:
        return f"cannot compile schema {self.schema_id}: {self.cause}"


@dataclass(frozen=True, order=True)
class ErrorInfo:
    """One validation error.

    ``keyword_location`` is the absolute keyword location, a JSON Pointer into
    the compiled schema document (for example ``/$defs/B/minimum``).
    ``instance_location`` is a JSON Pointer into the instance. ``keyword`` is
    jsonschema-rs's error kind, which usually names the failing keyword but
    not always (``falseSchema``; ``required`` for ``dependentRequired``).
    ``message`` is jsonschema-rs's message.
    """

    keyword_location: str
    instance_location: str
    keyword: str
    message: str


class Compiled:
    """A schema compiled by :meth:`Validator.compile`.

    ``schema`` is a private copy of the compiled document, ``digest`` the
    SHA-256 hex digest of its canonical bytes, and ``draft`` and
    ``dialect_assumed`` come from :func:`dialect_of`. Pickling keeps only
    the schema; unpickling compiles it again in the receiving process.
    """

    __slots__ = ("_locator", "_rs", "dialect_assumed", "digest", "draft", "schema")

    def __init__(
        self, schema: dict[str, Any] | bool, digest: str, draft: Draft, dialect_assumed: bool, rs: Any
    ) -> None:
        self.schema = schema
        self.digest = digest
        self.draft = draft
        self.dialect_assumed = dialect_assumed
        self._rs = rs
        self._locator: _Locator | None = None

    def _location_map(self) -> _Locator:
        """The evaluation-path walker for this document, built on first use."""
        if self._locator is None:
            self._locator = _Locator(self.schema, self.draft)
        return self._locator

    def __reduce__(self) -> tuple[Any, tuple[Any, ...]]:
        return (_recompile, (self.schema,))

    def __repr__(self) -> str:
        return f"Compiled(draft={self.draft.name}, digest={self.digest[:12]})"


# ---------------------------------------------------------------------------
# Evaluation path -> absolute keyword location
# ---------------------------------------------------------------------------

# Keywords whose value is one subschema, a list of subschemas, or a map from
# names to subschemas, across drafts 04 through 2020-12. Only these are
# walked when indexing resources and anchors, so JSON data under ``enum``,
# ``const``, ``default``, ``examples``, or ``x-*`` keywords is never taken
# for a subschema.
_SUBSCHEMA_KEYWORDS: Final = frozenset(
    {
        "additionalItems",
        "additionalProperties",
        "allOf",
        "anyOf",
        "contains",
        "contentSchema",
        "else",
        "if",
        "items",
        "not",
        "oneOf",
        "prefixItems",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    }
)
_SUBSCHEMA_MAP_KEYWORDS: Final = frozenset(
    {"$defs", "definitions", "dependencies", "dependentSchemas", "patternProperties", "properties"}
)

# The base URI of a document without a root $id. The .invalid TLD is
# reserved, so no real schema identifies itself with it.
_DEFAULT_BASE: Final = "https://json-schema.invalid/root"

_Token = str | int
_Hop = tuple[Any, list[_Token], str, list[_Token]]  # (node, pointer, base URI, resource root pointer)
_Index = tuple[dict[str, list[_Token]], dict[tuple[str, str], list[_Token]]]  # (resources, anchors)


def _step(node: Any, token: _Token) -> tuple[Any, _Token] | None:
    """Return ``(child, typed token)`` for one step into a schema document, or None."""
    if isinstance(node, dict):
        key = token if isinstance(token, str) else str(token)
        return (node[key], key) if key in node else None
    if isinstance(node, list):
        if isinstance(token, int) and not isinstance(token, bool):
            index = token
        elif isinstance(token, str) and token.isascii() and token.isdigit() and (token == "0" or token[0] != "0"):
            index = int(token)
        else:
            return None
        return (node[index], index) if 0 <= index < len(node) else None
    return None


class _Locator:
    """Maps jsonschema-rs evaluation paths to absolute keyword locations in one document."""

    __slots__ = ("_doc", "_draft", "_id_keyword", "_index_cache", "_root_base")

    def __init__(self, doc: dict[str, Any] | bool, draft: Draft) -> None:
        self._doc = doc
        self._draft = draft
        self._id_keyword = "id" if draft is Draft.DRAFT4 else "$id"
        self._root_base = self._new_base(_DEFAULT_BASE, doc) or _DEFAULT_BASE
        self._index_cache: _Index | None = None

    def locate(self, evaluation_path: Sequence[_Token]) -> str | None:
        """Return the absolute keyword location, or None when the path cannot be followed."""
        hop: _Hop | None = (self._doc, [], self._root_base, [])
        for token in evaluation_path:
            assert hop is not None
            node, pointer, base, resource = hop
            if token == "$ref" and isinstance(node, dict) and isinstance(node.get("$ref"), str):
                hop = self._follow(node["$ref"], base, resource)
            else:
                hop = self._descend(node, pointer, base, resource, (token,))
            if hop is None:
                return None
        assert hop is not None
        return join_pointer("", *hop[1])

    # -- resources and base URIs ------------------------------------------

    def _ident(self, node: Any) -> str | None:
        """The node's ``$id`` (``id`` in draft-04) when it applies, else None."""
        if not isinstance(node, dict):
            return None
        ident = node.get(self._id_keyword)
        if not isinstance(ident, str):
            return None
        if self._draft in _REF_IGNORES_SIBLINGS and "$ref" in node:
            return None
        return ident

    def _new_base(self, base: str, node: Any) -> str | None:
        """The base URI of the resource that ``node`` starts, or None when it starts none."""
        ident = self._ident(node)
        if ident is None or ident.startswith("#"):
            return None
        return urldefrag(urljoin(base, ident)).url

    def _anchor_names(self, node: dict[str, Any]) -> list[str]:
        names: list[str] = []
        if self._draft in _REF_IGNORES_SIBLINGS:
            ident = self._ident(node)
            if ident is not None and "#" in ident:
                fragment = unquote(ident.partition("#")[2])
                if fragment and not fragment.startswith("/"):
                    names.append(fragment)
            return names
        keywords = ("$anchor", "$dynamicAnchor") if self._draft is Draft.DRAFT202012 else ("$anchor",)
        for keyword in keywords:
            name = node.get(keyword)
            if isinstance(name, str) and name:
                names.append(name)
        return names

    def _index(self) -> _Index:
        """Build (once) the maps from resource URIs and from ``(resource URI, anchor)`` to document pointers."""
        if self._index_cache is not None:
            return self._index_cache
        resources: dict[str, list[_Token]] = {self._root_base: []}
        anchors: dict[tuple[str, str], list[_Token]] = {}
        stack: list[tuple[Any, list[_Token], str]] = [(self._doc, [], self._root_base)]
        while stack:
            node, pointer, base = stack.pop()
            if not isinstance(node, dict):
                continue
            if pointer:
                new_base = self._new_base(base, node)
                if new_base is not None:
                    base = new_base
                    resources.setdefault(base, pointer)
            for name in self._anchor_names(node):
                anchors.setdefault((base, name), pointer)
            children: list[tuple[Any, list[_Token], str]] = []
            for keyword, value in node.items():
                if keyword in _SUBSCHEMA_MAP_KEYWORDS and isinstance(value, dict):
                    children.extend((sub, [*pointer, keyword, name], base) for name, sub in value.items())
                elif keyword in _SUBSCHEMA_KEYWORDS and isinstance(value, list):
                    children.extend((sub, [*pointer, keyword, i], base) for i, sub in enumerate(value))
                elif keyword in _SUBSCHEMA_KEYWORDS:
                    children.append((value, [*pointer, keyword], base))
            stack.extend(reversed(children))  # pre-order: the first occurrence wins
        self._index_cache = (resources, anchors)
        return self._index_cache

    # -- walking -----------------------------------------------------------

    def _descend(
        self, node: Any, pointer: list[_Token], base: str, resource: list[_Token], tokens: Sequence[_Token]
    ) -> _Hop | None:
        for token in tokens:
            stepped = _step(node, token)
            if stepped is None:
                return None
            node, typed = stepped
            pointer = [*pointer, typed]
            new_base = self._new_base(base, node)
            if new_base is not None:
                base, resource = new_base, pointer
        return node, pointer, base, resource

    def _node_at(self, pointer: Sequence[_Token]) -> Any:
        node: Any = self._doc
        for token in pointer:
            stepped = _step(node, token)
            assert stepped is not None, "indexed pointers always resolve"
            node = stepped[0]
        return node

    def _follow(self, ref: str, base: str, resource: list[_Token]) -> _Hop | None:
        """Follow one ``$ref`` from a node whose base URI is ``base``."""
        if ref.startswith("#"):
            uri, fragment = base, ref[1:]
        else:
            uri, fragment = urldefrag(urljoin(base, ref))
            if uri != base:
                found = self._index()[0].get(uri)
                if found is None:
                    return None  # a remote resource, or one jsinfer cannot place
                resource = found
        fragment = unquote(fragment)
        if fragment and not fragment.startswith("/"):
            resources, anchors = self._index()
            target = anchors.get((uri, fragment))
            if target is None:
                return None
            # An anchor is registered under the base of the resource holding it.
            return self._node_at(target), list(target), uri, resources[uri]
        try:
            tokens = split_pointer(fragment)
        except ValueError:
            return None
        return self._descend(self._node_at(resource), list(resource), uri, resource, tokens)


# ---------------------------------------------------------------------------
# The Validator
# ---------------------------------------------------------------------------


class _LRU:
    """A small least-recently-used map."""

    __slots__ = ("_data", "_maxsize")

    def __init__(self, maxsize: int) -> None:
        self._data: OrderedDict[Hashable, Any] = OrderedDict()
        self._maxsize = maxsize

    def get(self, key: Hashable) -> Any:
        try:
            value = self._data[key]
        except KeyError:
            return None
        self._data.move_to_end(key)
        return value

    def put(self, key: Hashable, value: Any) -> None:
        self._data[key] = value
        self._data.move_to_end(key)
        if len(self._data) > self._maxsize:
            self._data.popitem(last=False)

    def __len__(self) -> int:
        return len(self._data)


def _pointer(tokens: Sequence[_Token]) -> str:
    return join_pointer("", *tokens)


def _describe(exc: Exception) -> str:
    """A one-line cause for a jsonschema-rs compile failure."""
    if isinstance(exc, jsonschema_rs.ValidationError):
        where = _pointer(exc.instance_path)
        return f"{exc.message} (at {where})" if where else exc.message
    if isinstance(exc, jsonschema_rs.ReferencingError):
        return exc.message
    text = str(exc).strip()
    return f"{type(exc).__name__}: {text}" if text else type(exc).__name__


def _schema_id(schema: dict[str, Any] | bool, name: str | None, digest: str | None) -> str:
    if name is not None:
        return name
    if isinstance(schema, dict):
        keyword = "id" if dialect_of(schema)[0] is Draft.DRAFT4 else "$id"
        ident = schema.get(keyword)
        if isinstance(ident, str) and ident:
            return ident
    return f"sha256:{digest[:12]}" if digest is not None else "(unnamed)"


def _check_bound(name: str, value: Any) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ValueError(f"{name} must be an int >= 1, got {value!r}")
    return value


class Validator:
    """Compiles schemas and gives every accept or reject verdict (see the module docstring)."""

    def __init__(self, max_compiled: int = DEFAULT_MAX_COMPILED, max_values: int = DEFAULT_MAX_VALUES) -> None:
        self._max_compiled = _check_bound("max_compiled", max_compiled)
        self._max_values = _check_bound("max_values", max_values)
        self._compiled = _LRU(self._max_compiled)  # digest -> Compiled, or the cause of a failure
        self._values = _LRU(self._max_values)  # (keyword, value, string) -> bool
        self._meta: dict[Draft, Any] = {}

    def __reduce__(self) -> tuple[Any, tuple[Any, ...]]:
        # Caches hold jsonschema-rs validators, which do not cross processes.
        return (Validator, (self._max_compiled, self._max_values))

    # -- compiling and validating ------------------------------------------

    def compile(self, schema: dict[str, Any] | bool, *, name: str | None = None) -> Compiled:
        """Compile ``schema`` under its declared or assumed draft.

        ``name`` identifies the schema in a :class:`CompileError` (for
        example a corpus id or a file name). Raises :class:`CompileError`
        when jsonschema-rs cannot compile the schema or the schema is not a
        JSON value, and ``TypeError`` when the root is neither an object nor
        a boolean.
        """
        if not isinstance(schema, (dict, bool)):
            raise TypeError(f"a schema must be a dict or a bool, got {type(schema).__name__}")
        if name is not None and (not isinstance(name, str) or not name):
            raise ValueError("name must be None or a non-empty str")
        try:
            data = canonical_bytes(schema)
        except (TypeError, ValueError) as exc:
            raise CompileError(_schema_id(schema, name, None), f"the schema is not a JSON value: {exc}") from exc
        digest = hashlib.sha256(data).hexdigest()
        draft, assumed = dialect_of(schema)
        key = digest
        if draft is Draft.DRAFT4:
            # jsonschema-rs compiles 1 and 1.0 differently here, so their spelling is part of the key.
            spellings = _float_spellings(schema)
            if spellings:
                key = hashlib.sha256(data + b"\0" + canonical_bytes(spellings)).hexdigest()
        cached = self._compiled.get(key)
        if isinstance(cached, Compiled):
            return cached
        if isinstance(cached, str):
            raise CompileError(_schema_id(schema, name, digest), cached, digest)
        try:
            rs = _build(draft, _rs_schema(schema, draft))
        except Exception as exc:  # any failure inside jsonschema-rs is a compile failure (29.10)
            cause = _describe(exc)
            self._compiled.put(key, cause)
            raise CompileError(_schema_id(schema, name, digest), cause, digest) from exc
        # The private copy keeps error locations right if the caller mutates its schema.
        compiled = Compiled(copy.deepcopy(schema), digest, draft, assumed, rs)
        self._compiled.put(key, compiled)
        return compiled

    def is_valid(self, compiled: Compiled, instance: JSON) -> bool:
        """Return whether ``instance`` validates against ``compiled``, with ``format`` asserted."""
        compiled = _require_compiled(compiled)
        return bool(compiled._rs.is_valid(_rs_instance(instance, compiled.draft)))

    def errors(self, compiled: Compiled, instance: JSON) -> list[ErrorInfo]:
        """Return every validation error, sorted; empty exactly when ``instance`` is valid.

        Keyword locations are absolute (see the module docstring).
        """
        locator = _require_compiled(compiled)._location_map()
        found: list[ErrorInfo] = []
        for error in compiled._rs.iter_errors(_rs_instance(instance, compiled.draft)):
            location = locator.locate(error.evaluation_path)
            if location is None:
                location = _pointer(error.schema_path)
            found.append(ErrorInfo(location, _pointer(error.instance_path), error.kind.name, error.message))
        found.sort()
        return found

    # -- meta-schema checks ------------------------------------------------

    def meta_errors(self, schema: dict[str, Any] | bool, draft: Draft) -> list[str]:
        """Return the errors of ``schema`` against the meta-schema of ``draft``, sorted.

        Each entry is ``"<JSON Pointer into the schema>: <message>"``, with
        ``(root)`` for the root. The list is empty exactly when the schema
        is valid against the meta-schema.
        """
        if not isinstance(draft, Draft):
            raise TypeError(f"draft must be a Draft, got {type(draft).__name__}")
        if not isinstance(schema, (dict, bool)):
            raise TypeError(f"a schema must be a dict or a bool, got {type(schema).__name__}")
        meta = self._meta.get(draft)
        if meta is None:
            meta = self._meta[draft] = _build(draft, {"$ref": draft.uri})
        errors = meta.iter_errors(_rs_instance(schema, draft))
        found = sorted((_pointer(error.instance_path), error.message) for error in errors)
        return [f"{where or '(root)'}: {message}" for where, message in found]

    # -- Format and Pattern checks -----------------------------------------

    def satisfies_format(self, fmt: str, s: str) -> bool:
        """Return whether ``s`` satisfies the Format ``fmt`` (Requirement 13.4).

        This is the verdict of ``{"format": fmt}`` with format assertion on,
        so an unknown format name accepts every string, as it does in a
        schema.
        """
        return self._keyword_check("format", fmt, s)

    def matches_pattern(self, pattern: str, s: str) -> bool:
        """Return whether ``s`` matches ``pattern`` (unanchored, as the ``pattern`` keyword).

        Raises :class:`CompileError` when jsonschema-rs does not accept the
        pattern.
        """
        return self._keyword_check("pattern", pattern, s)

    def _keyword_check(self, keyword: str, value: str, s: str) -> bool:
        if not isinstance(value, str):
            raise TypeError(f"{keyword} must be a str, got {type(value).__name__}")
        if not isinstance(s, str):
            raise TypeError(f"the checked value must be a str, got {type(s).__name__}")
        key = (keyword, value, s)
        cached = self._values.get(key)
        if cached is not None:
            return bool(cached)
        schema = {keyword: value}
        compiled = self.compile(schema, name=json.dumps(schema, ensure_ascii=False))
        result = self.is_valid(compiled, s)
        self._values.put(key, result)
        return result


def _require_compiled(compiled: Any) -> Compiled:
    if not isinstance(compiled, Compiled):
        raise TypeError(f"expected a Compiled schema, got {type(compiled).__name__}")
    return compiled


# One Validator per process compiles unpickled Compiled values, so a worker
# that receives many of them compiles each distinct schema once.
_process_validator: Validator | None = None


def _recompile(schema: dict[str, Any] | bool) -> Compiled:
    global _process_validator
    if _process_validator is None:
        _process_validator = Validator()
    return _process_validator.compile(schema)
