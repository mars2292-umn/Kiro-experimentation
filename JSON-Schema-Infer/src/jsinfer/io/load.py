"""Schema_Loader: parse a JSON Schema document into a Schema_Model (Requirements 2.4–2.6, 2.10–2.12, 26.8).

It also reads explanation files back (:func:`load_explanation`; see
Explanation files at the end).

:meth:`SchemaLoader.load` turns a schema document into a
:class:`~jsinfer.core.schema_model.SchemaModel` in draft 2020-12 form. It
runs these steps in order; the first three fail with :class:`SchemaLoadError`
and return no model (2.12):

1. **Parse** (check ``json``). ``bytes`` are parsed with
   :func:`~jsinfer.io.ingest.parse_document` (one UTF-8 BOM stripped, orjson
   first, exact big integers), and a ``str`` is its UTF-8 encoding. A syntax
   error carries the 1-based line and column of
   :class:`~jsinfer.io.ingest.DocumentParseError`. An already parsed value
   must be a JSON value: ``None``, ``bool``, ``int``, finite ``float``,
   ``str``, ``list``, or ``dict`` with ``str`` keys, and strings without
   unpaired surrogates.
2. **Root kind** (check ``root``). The root must be a JSON object or a
   boolean.
3. **Dialect and meta-schema** (check ``meta``). The dialect comes from the
   root ``$schema`` through :func:`~jsinfer.io.validator.dialect_of`, which
   maps ``http://`` to ``https://`` and drops one trailing ``#`` before
   comparing. With no ``$schema``, an unrecognized URI, or a boolean root,
   the draft is 2020-12 and ``SchemaModel.dialect_assumed`` is set (2.11).
   The document is then validated against that draft's meta-schema with
   :meth:`Validator.meta_errors <jsinfer.io.validator.Validator.meta_errors>`
   (formats asserted); every error is kept in ``SchemaLoadError.errors``.
4. **Draft translation** (:func:`_translate`, below). A declared draft-04,
   -06, -07, or 2019-09 document is rewritten into draft 2020-12 shape, with
   every local ``$ref`` rewritten to match (2.5). A draft 2020-12 document,
   declared or assumed, passes through unchanged.
5. **Keyword loading** (below). The result is renumbered
   (:meth:`SchemaModel.renumber`), so nids follow canonical pre-order.

Draft translation
    The rewrite walks the schema positions of the source document: those of
    the 2020-12 subschema keywords that the declared draft defines (and
    ``$defs``, see keyword loading), plus the translated ones below. It
    builds a new document, never changing the source. At each schema
    object:

    - **``$ref`` siblings** (drafts 04 to 07, where ``$ref`` overrides its
      siblings). Every keyword next to ``$ref`` is left out and recorded as
      ignored, ``$id`` and annotations included, except two kinds that the
      draft's ``$ref`` does not hide: ``$schema`` at the root, which selects
      the dialect, and the ``definitions`` and ``$defs`` containers. Those
      assert nothing, so keeping them changes no verdict, and a JSON Pointer
      ``$ref`` reaches into them under the draft (the common layout
      ``{"$ref": "#/definitions/Root", "definitions": {...}}`` relies on it).
      This applies first, so the rules below see only what is left.
    - ``definitions`` becomes ``$defs``. When the object also holds
      ``$defs``, ``definitions`` cannot take the name; it is left out and
      recorded as unsupported.
    - Array-form ``items`` becomes ``prefixItems``, and ``additionalItems``
      next to it becomes ``items``. ``additionalItems`` without array-form
      ``items`` has no effect under these drafts; it is left out and
      recorded as ignored. A ``prefixItems`` next to array-form ``items`` is
      not a keyword of the draft; it is left out and recorded as ignored
      (keyword loading would ignore it too).
    - ``dependencies``: its array-valued entries become ``dependentRequired``
      entries. Schema-valued entries (booleans included) are unsupported:
      they are left out and recorded once, as ``(object, "dependencies")``,
      however many there are, and the array-valued entries of the same
      object are still translated. ``dependentRequired`` is written when at
      least one entry is array-valued or ``dependencies`` is empty. A
      ``dependentRequired`` already in the object is left out and recorded
      as ignored in drafts 04 to 07, which do not define it. In 2019-09,
      which does, ``dependencies`` cannot be merged into it and is recorded
      as unsupported instead.
    - Draft-04 ``id`` becomes ``$id`` (keyword loading keeps it at the root
      only). A draft-04 ``$id`` next to it is left out and recorded as
      keyword loading would record it: ignored at the root, unsupported
      below.
    - Draft-04 boolean exclusive bounds. ``exclusiveMinimum: true`` with
      ``minimum: m`` becomes ``exclusiveMinimum: m`` and drops ``minimum``;
      ``exclusiveMinimum: false`` is dropped and ``minimum`` stays. The same
      holds for ``exclusiveMaximum`` and ``maximum``. Neither is recorded:
      the meaning is kept, not discarded.
    - Formats from later drafts. jsonschema-rs asserts some formats only
      from a later draft: ``uri-reference``, ``uri-template``, and
      ``json-pointer`` from draft-06; ``idn-hostname``, ``iri``,
      ``iri-reference``, and ``relative-json-pointer`` from draft-07;
      ``duration`` and ``uuid`` from 2019-09. Under an earlier declared draft
      such a ``format`` asserts nothing, while draft 2020-12 asserts it, so
      it is left out and recorded as ignored, as for the keywords a draft
      ignores (2.9).

    A value whose shape the rule does not expect (the meta-schema check
    rules most of these out) is passed on unchanged, and keyword loading
    records it.

    The rewrite records a pointer-rewrite map from the source tokens of every
    schema position it translates to that position's tokens in the new
    document; renaming replaces one token by one token, so only keyword
    tokens change (``/definitions/a`` to ``/$defs/a``, ``/items/0`` to
    ``/prefixItems/0``, ``/additionalItems`` to ``/items``). Each local
    ``$ref`` is then decoded as in keyword loading and looked up in the map.
    A ``$ref`` whose target maps to itself is kept as written. One whose
    target moved is written again as ``#`` plus the new pointer,
    percent-encoded so that it stays a URI reference (characters outside
    the RFC 3986 fragment set, ``%`` included, are escaped as UTF-8). A
    ``$ref`` whose pointer is no schema position of the translation (a
    container such as ``#/definitions``, a sibling that ``$ref`` hides, or
    a construct left out above) is left out and recorded as unsupported.
    Remote, relative, and plain-name references are left for keyword
    loading.

    The rewrite also returns, for keyword loading, the source name of every
    renamed keyword, so that records and the source pointers of children
    use source names (``/definitions/a``, ``/items/0``), and its own
    ``ignored`` and ``unsupported`` records, in source pointers.

Keyword loading
    Schema positions are those of the 2020-12 subschema keywords
    (``core.schema_model.SUBSCHEMA_KEYWORDS``), and boolean schemas load at
    every one of them, the root included. At each schema object, each
    keyword is handled by the first rule that applies. Here the *source
    keyword* is the name the keyword had in the source document; it differs
    only for keywords the rewrite renamed, and records name it.

    1. ``x-`` keywords and the annotations of :data:`ANNOTATION_KEYWORDS` are
       kept as JSON values at every node under every draft (2.10); an
       annotation whose value lacks its draft 2020-12 shape (``deprecated``
       not a boolean, ``examples`` not an array, ``title`` not a string) is
       recorded as ignored instead, since the emitted document must satisfy
       the 2020-12 meta-schema and the source draft left the value unchecked.
       This
       includes the inline ``x-provenance`` of the Schema_Emitter (26.6): it
       is kept, never recorded as ignored or unsupported, and 2.7 equality
       and the Schema_Emitter leave it out, so a schema emitted with inline
       provenance loads to a model equal to the one emitted without it
       (26.11). Its records are not read back into ``SchemaModel.provenance``;
       the explanation file is the source of records (below).
    2. ``$schema`` at the root selects the dialect and is not kept: the
       model is draft 2020-12 whatever the source declared, and the
       Schema_Emitter writes ``$schema``. An unrecognized ``$schema`` URI is
       recorded as unsupported (2.6). Below the root, ``$schema`` is
       recorded as ignored: without an embedded resource (an unsupported
       ``$id``) it has no effect.
    3. ``$id`` is kept at the root when the declared draft defines the source
       keyword (2.10). Below the root it is unsupported (2.6).
    4. A keyword of :data:`SUPPORTED_KEYWORDS` loads when the declared draft
       defines its source keyword (:data:`VOCABULARIES`). ``$defs`` loads
       under every draft: it holds no assertion, and a JSON Pointer ``$ref``
       reaches into it under every draft. A supported keyword that the
       declared draft does not define, such as ``prefixItems`` in a draft-07
       schema or ``const`` in a draft-04 schema, has no effect under that
       draft (jsonschema-rs ignores it too). It is left out and recorded as
       ignored, as Requirement 2.5 does for the keywords a draft ignores, so
       that the loaded model gives the source's verdicts (2.9). A renamed
       keyword is checked by its source keyword, so ``prefixItems`` from
       array-form ``items`` loads under draft-07. A supported keyword whose
       value still lacks its 2020-12 shape after translation (a non-string
       ``$ref`` in a draft-04 document, a boolean ``exclusiveMinimum``
       without ``minimum``) is unsupported.
    5. Every other keyword that draft 2020-12 or the declared draft defines
       is unsupported (2.6), for example ``multipleOf``,
       ``dependentSchemas``, ``unevaluatedProperties``, ``minContains``,
       ``contentEncoding``, ``$anchor``, ``$dynamicRef``, and
       ``$recursiveRef``. The vocabularies include the keywords that the
       2019-09 and 2020-12 meta-schemas keep for compatibility
       (``definitions``, ``dependencies``, ``$recursiveRef``,
       ``$recursiveAnchor``); jsonschema-rs still applies ``dependencies``
       under both drafts. Translation turns ``definitions`` and
       ``dependencies`` of a 2019-09 document into ``$defs`` and
       ``dependentRequired``; in a 2020-12 document, which Requirement 2.5
       does not translate, they are unsupported.
    6. Every other keyword is ignored (2.10), for example
       ``markdownDescription``. Ignored keywords do not make a model
       partially supported.

    An unsupported or ignored keyword is left out with its whole value, so
    nothing below it is loaded or recorded. Kept JSON values are deep copies,
    so the model never shares data with a parsed source.

References
    A ``$ref`` loads when it is local (2.4): its value is ``#`` or ``#``
    followed by a JSON Pointer (percent-decoded first, as a URI fragment),
    and the pointer resolves to a subschema of the loaded model. Recursive
    references (``#``, or a pointer to an ancestor) are just strings, so the
    model stays a tree. The value is kept as written. A ``$ref`` is
    unsupported and left out when it is remote or relative
    (``other.json#/a``, ``https://...``), when its fragment is a plain name
    (an anchor), when its pointer does not resolve in the model (a missing
    location, a location inside a construct that was left out, or one that
    is not a schema, such as ``#/$defs`` itself), or when it sits in an
    embedded schema resource. A root ``id``/``$id`` of drafts 04 to 07 whose
    value is a plain-name fragment (``#name``, an anchor for the root, which
    ``#`` already names) is left out and recorded as ignored, since draft
    2020-12 admits no such ``$id`` (the round trip of 2.8 would otherwise
    emit a document its own meta-schema rejects). An embedded resource
    starts at a subschema below the root whose draft id keyword (``id`` in
    draft-04, renamed to ``$id`` by translation; ``$id`` otherwise) is not a
    bare fragment; in
    drafts 04 to 07 translation has already left out an ``$id`` next to
    ``$ref``, which those drafts ignore. Its ``$id`` is unsupported, and
    every reference inside it is relative to that resource rather than to
    the document, so it is unsupported as well. For a translated document,
    the value kept is the one translation wrote.

Records
    ``SchemaModel.ignored`` and ``SchemaModel.unsupported`` hold
    ``(pointer, keyword)`` pairs: the JSON Pointer of the schema object in
    the source document (``""`` for the root) and the source keyword. Each
    list is sorted in code-point order without repeats, so it depends only on
    the document and not on its member order. A model with an unsupported
    record is partially supported (``SchemaModel.fully_supported`` is
    false, 2.6).

Explanation files
    :func:`load_explanation` reads the explanation file of
    ``io.emit.ExplanationFile`` (``{"format": 1, "records": {...}}``) back
    into an ``ExplanationFile``. It parses like the Schema_Loader, checks the
    format version first, then the members, the pointer keys, and every
    record, and raises :class:`ExplanationLoadError` with the check that
    failed and the JSON Pointer of the offending member. Writing a loaded
    file again gives the bytes it was read from (26.8).
"""

from __future__ import annotations

import copy
import math
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias
from urllib.parse import quote, unquote

from jsinfer.core.determinism import codepoint_key
from jsinfer.core.provenance import ProvenanceFormatError, ProvenanceRecord
from jsinfer.core.schema_model import SUBSCHEMA_KEYWORDS, SchemaModel, SchemaNode, join_pointer, split_pointer
from jsinfer.io.emit import EXPLANATION_FORMAT, ExplanationFile
from jsinfer.io.ingest import DocumentParseError, parse_document
from jsinfer.io.validator import Draft, Validator, dialect_of

__all__ = [
    "ANNOTATION_KEYWORDS",
    "EXPLANATION_LOAD_CHECKS",
    "SCHEMA_LOAD_CHECKS",
    "SUPPORTED_KEYWORDS",
    "VOCABULARIES",
    "ExplanationLoadCheck",
    "ExplanationLoadError",
    "SchemaLoadCheck",
    "SchemaLoadError",
    "SchemaLoader",
    "load_explanation",
]

# ---------------------------------------------------------------------------
# Keyword sets
# ---------------------------------------------------------------------------

# The keywords of Requirement 2.4 (local $ref included). Root $id, $schema,
# annotations, and x- keywords are handled by their own rules.
SUPPORTED_KEYWORDS: Final = frozenset(
    {
        "$defs",
        "$ref",
        "additionalProperties",
        "allOf",
        "anyOf",
        "const",
        "contains",
        "dependentRequired",
        "else",
        "enum",
        "exclusiveMaximum",
        "exclusiveMinimum",
        "format",
        "if",
        "items",
        "maxItems",
        "maxLength",
        "maxProperties",
        "maximum",
        "minItems",
        "minLength",
        "minProperties",
        "minimum",
        "not",
        "oneOf",
        "pattern",
        "patternProperties",
        "prefixItems",
        "properties",
        "propertyNames",
        "required",
        "then",
        "type",
        "uniqueItems",
    }
)

# Annotations retained at every node under every draft (Requirement 2.10).
ANNOTATION_KEYWORDS: Final = frozenset(
    {"$comment", "default", "deprecated", "description", "examples", "readOnly", "title", "writeOnly"}
)

_DRAFT4_VOCABULARY: Final = frozenset(
    {
        "$ref",
        "$schema",
        "additionalItems",
        "additionalProperties",
        "allOf",
        "anyOf",
        "default",
        "definitions",
        "dependencies",
        "description",
        "enum",
        "exclusiveMaximum",
        "exclusiveMinimum",
        "format",
        "id",
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
        "properties",
        "required",
        "title",
        "type",
        "uniqueItems",
    }
)
_DRAFT6_VOCABULARY: Final = (_DRAFT4_VOCABULARY - {"id"}) | {"$id", "const", "contains", "examples", "propertyNames"}
_DRAFT7_VOCABULARY: Final = _DRAFT6_VOCABULARY | {
    "$comment",
    "contentEncoding",
    "contentMediaType",
    "else",
    "if",
    "readOnly",
    "then",
    "writeOnly",
}
# Keywords shared by the 2019-09 and 2020-12 vocabularies (validation,
# meta-data, format, content, and most applicators) and their meta-schemas.
_MODERN_VOCABULARY: Final = frozenset(
    {
        "$anchor",
        "$comment",
        "$defs",
        "$id",
        "$recursiveAnchor",
        "$recursiveRef",
        "$ref",
        "$schema",
        "$vocabulary",
        "additionalProperties",
        "allOf",
        "anyOf",
        "const",
        "contains",
        "contentEncoding",
        "contentMediaType",
        "contentSchema",
        "default",
        "definitions",
        "dependencies",
        "dependentRequired",
        "dependentSchemas",
        "deprecated",
        "description",
        "else",
        "enum",
        "examples",
        "exclusiveMaximum",
        "exclusiveMinimum",
        "format",
        "if",
        "items",
        "maxContains",
        "maxItems",
        "maxLength",
        "maxProperties",
        "maximum",
        "minContains",
        "minItems",
        "minLength",
        "minProperties",
        "minimum",
        "multipleOf",
        "not",
        "oneOf",
        "pattern",
        "patternProperties",
        "properties",
        "propertyNames",
        "readOnly",
        "required",
        "then",
        "title",
        "type",
        "unevaluatedItems",
        "unevaluatedProperties",
        "uniqueItems",
        "writeOnly",
    }
)

# The keywords each draft defines, including those its meta-schema keeps for
# compatibility. Membership only; never iterated.
VOCABULARIES: Final[Mapping[Draft, frozenset[str]]] = MappingProxyType(
    {
        Draft.DRAFT4: _DRAFT4_VOCABULARY,
        Draft.DRAFT6: _DRAFT6_VOCABULARY,
        Draft.DRAFT7: _DRAFT7_VOCABULARY,
        Draft.DRAFT201909: _MODERN_VOCABULARY | {"additionalItems"},
        Draft.DRAFT202012: _MODERN_VOCABULARY | {"$dynamicAnchor", "$dynamicRef", "prefixItems"},
    }
)

# Formats that jsonschema-rs asserts only from a later draft (as of
# jsonschema-rs 0.58; tests/unit/test_load.py pins the table). Under the
# drafts listed, a "format" naming one asserts nothing, while draft 2020-12
# asserts it. Membership only; never iterated.
_LATER_FORMATS: Final[Mapping[Draft, frozenset[str]]] = MappingProxyType(
    {
        Draft.DRAFT4: frozenset(
            {
                "duration",
                "idn-hostname",
                "iri",
                "iri-reference",
                "json-pointer",
                "relative-json-pointer",
                "uri-reference",
                "uri-template",
                "uuid",
            }
        ),
        Draft.DRAFT6: frozenset({"duration", "idn-hostname", "iri", "iri-reference", "relative-json-pointer", "uuid"}),
        Draft.DRAFT7: frozenset({"duration", "uuid"}),
        Draft.DRAFT201909: frozenset(),
    }
)

_DRAFT_NAMES: Final[Mapping[Draft, str]] = MappingProxyType(
    {
        Draft.DRAFT4: "draft-04",
        Draft.DRAFT6: "draft-06",
        Draft.DRAFT7: "draft-07",
        Draft.DRAFT201909: "draft 2019-09",
        Draft.DRAFT202012: "draft 2020-12",
    }
)

# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------

SchemaLoadCheck: TypeAlias = Literal["json", "root", "meta"]
# The checks of Requirement 2.12:
#   json  the document is not valid JSON (or a parsed source is not a JSON value)
#   root  the root is neither a JSON object nor a boolean
#   meta  the document fails the meta-schema of its declared or assumed draft
SCHEMA_LOAD_CHECKS: Final[tuple[SchemaLoadCheck, ...]] = ("json", "root", "meta")


class SchemaLoadError(ValueError):
    """A document that the Schema_Loader cannot load (Requirement 2.12).

    ``check`` names the failed check (:data:`SCHEMA_LOAD_CHECKS`), ``reason``
    describes the failure, and ``line`` and ``col`` are the 1-based location
    of a JSON syntax error (``None`` when unknown or not applicable).
    ``errors`` holds every meta-schema error for the ``meta`` check, each as
    ``"<JSON Pointer>: <message>"``, and is empty otherwise.
    """

    def __init__(
        self,
        check: SchemaLoadCheck,
        reason: str,
        line: int | None = None,
        col: int | None = None,
        errors: Iterable[str] = (),
    ) -> None:
        if check not in SCHEMA_LOAD_CHECKS:
            raise ValueError(f"SchemaLoadError check must be one of {list(SCHEMA_LOAD_CHECKS)}, got {check!r}")
        if not isinstance(reason, str) or not reason:
            raise ValueError("SchemaLoadError reason must be a non-empty str")
        for name, number in (("line", line), ("col", col)):
            if number is not None and (type(number) is not int or number < 1):
                raise ValueError(f"SchemaLoadError {name} must be None or an int >= 1, got {number!r}")
        found = tuple(errors)
        for error in found:
            if not isinstance(error, str):
                raise TypeError(f"SchemaLoadError errors must be str, got {type(error).__name__}")
        super().__init__(check, reason, line, col, found)  # args survive pickling
        self.check: SchemaLoadCheck = check
        self.reason = reason
        self.line = line
        self.col = col
        self.errors: tuple[str, ...] = found

    def __str__(self) -> str:
        if self.line is None:
            return self.reason
        if self.col is None:
            return f"{self.reason} (line {self.line})"
        return f"{self.reason} (line {self.line}, column {self.col})"


# ---------------------------------------------------------------------------
# Steps 1 and 2: parsing and the root kind
# ---------------------------------------------------------------------------


def _encodes(text: str) -> bool:
    """Whether ``text`` is valid Unicode (no unpaired surrogate), so it can be written as UTF-8."""
    try:
        text.encode("utf-8")
    except UnicodeEncodeError:
        return False
    return True


def _parse_text(text: str) -> Any:
    try:
        data = text.encode("utf-8")
    except UnicodeEncodeError as exc:
        position = exc.start
        line = text.count("\n", 0, position) + 1
        col = position - text.rfind("\n", 0, position)
        unit = ord(text[position])
        raise SchemaLoadError("json", f"invalid JSON text: unpaired surrogate U+{unit:04X}", line, col) from None
    return _parse_bytes(data)


def _parse_bytes(data: bytes) -> Any:
    try:
        value, _parser = parse_document(data)
    except DocumentParseError as exc:
        raise SchemaLoadError("json", exc.reason, exc.line, exc.column) from None
    return value


def _check_json_value(value: Any, what: str = "the schema") -> None:
    """Raise a ``json`` SchemaLoadError unless ``value`` is a JSON value (see the module docstring)."""
    pending: list[tuple[Any, str]] = [(value, "")]  # an explicit stack, so deep nesting cannot overflow
    while pending:
        item, pointer = pending.pop()
        if item is None or isinstance(item, (bool, int)):
            continue
        if isinstance(item, float):
            if math.isfinite(item):
                continue
            problem = f"the number {item!r} is not finite"
        elif isinstance(item, str):
            if _encodes(item):
                continue
            problem = "a string holds an unpaired surrogate code point"
        elif isinstance(item, list):
            pending.extend((child, join_pointer(pointer, index)) for index, child in enumerate(item))
            continue
        elif isinstance(item, dict):
            problem = ""
            for key, child in item.items():
                if not isinstance(key, str):
                    problem = f"an object key is {type(key).__name__}, not str"
                    break
                if not _encodes(key):
                    problem = "an object key holds an unpaired surrogate code point"
                    break
                pending.append((child, join_pointer(pointer, key)))
            if not problem:
                continue
        else:
            problem = f"{type(item).__name__} is not a JSON type"
        raise SchemaLoadError("json", f"{what} is not a JSON value: {problem} at {pointer or '(root)'}")


def _parse(source: Any, what: str = "the schema") -> Any:
    """Step 1: the parsed JSON value of ``source``."""
    if isinstance(source, str):
        return _parse_text(source)
    if isinstance(source, bytes):
        return _parse_bytes(bytes(source))
    if source is None or isinstance(source, (bool, int, float, list, dict)):
        _check_json_value(source, what)
        return source
    raise TypeError(f"a schema source must be bytes, str, or a parsed JSON value, got {type(source).__name__}")


def _kind(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, str):
        return "a string"
    if isinstance(value, list):
        return "an array"
    return "a number"


# ---------------------------------------------------------------------------
# Step 4: draft translation
# ---------------------------------------------------------------------------

_Record: TypeAlias = tuple[str, str]  # (JSON Pointer of the schema object in the source, source keyword)


@dataclass(frozen=True, slots=True)
class _Prepared:
    """A schema document in draft 2020-12 shape, ready for keyword loading.

    ``doc`` is the document. ``renamed`` maps ``(pointer in doc of a schema
    object, keyword)`` to the keyword's name in the source document, for
    every keyword the rewrite renamed; unlisted keywords keep their name.
    Child schema positions take their source pointers from the source
    keyword, so a renamed container (``definitions`` to ``$defs``) needs no
    other entry. ``ignored`` and ``unsupported`` are the records the rewrite
    itself made, in source pointers.
    """

    doc: dict[str, Any] | bool
    renamed: Mapping[tuple[str, str], str] = field(default_factory=dict)
    ignored: tuple[_Record, ...] = ()
    unsupported: tuple[_Record, ...] = ()


_Tokens: TypeAlias = tuple[str, ...]  # unescaped JSON Pointer tokens; array indices as decimal strings

# Drafts whose documents are translated (Requirement 2.5). A draft 2020-12
# document, declared or assumed, passes through unchanged.
_TRANSLATED_DRAFTS: Final = frozenset({Draft.DRAFT4, Draft.DRAFT6, Draft.DRAFT7, Draft.DRAFT201909})

# Drafts in which "$ref" overrides its siblings, "$id" included.
_REF_OVERRIDES_SIBLINGS: Final = frozenset({Draft.DRAFT4, Draft.DRAFT6, Draft.DRAFT7})

# Siblings of "$ref" kept in those drafts: they assert nothing, and a JSON
# Pointer "$ref" still reaches into them.
_REF_TARGET_CONTAINERS: Final = frozenset({"$defs", "definitions"})

# Draft-04 boolean exclusive bounds and the bound each one qualifies, both ways.
_BOUND_OF: Final[Mapping[str, str]] = MappingProxyType({"exclusiveMaximum": "maximum", "exclusiveMinimum": "minimum"})
_EXCLUSIVE_OF: Final[Mapping[str, str]] = MappingProxyType({v: k for k, v in _BOUND_OF.items()})

# Characters a rewritten "$ref" keeps unescaped: the RFC 3986 fragment
# characters beyond the unreserved ones, which quote() never escapes.
_FRAGMENT_SAFE: Final = "/!$&'()*+,;=:@"

# A schema position waiting to be translated: (value, source tokens, doc
# tokens, the container that receives the translation, and its slot there).
_Step: TypeAlias = tuple[Any, _Tokens, _Tokens, Any, Any]


def _is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _translate(schema: dict[str, Any] | bool, draft: Draft) -> _Prepared:
    """Bring a document of ``draft`` into draft 2020-12 shape (Requirement 2.5; see the module docstring).

    A draft 2020-12 document is returned unchanged. The source document is
    never modified.
    """
    if draft not in _TRANSLATED_DRAFTS:
        return _Prepared(schema)
    return _Translator(draft).run(schema)


class _Translator:
    """One translation pass over a draft-04, -06, -07, or 2019-09 document (see the module docstring)."""

    __slots__ = (
        "_draft",
        "_id_keyword",
        "_ignored",
        "_later_formats",
        "_positions",
        "_refs",
        "_renamed",
        "_unsupported",
        "_vocabulary",
    )

    def __init__(self, draft: Draft) -> None:
        self._draft = draft
        self._id_keyword = "id" if draft is Draft.DRAFT4 else "$id"
        self._vocabulary = VOCABULARIES[draft]
        self._later_formats = _LATER_FORMATS[draft]
        # The pointer-rewrite map: source tokens -> doc tokens of every translated schema position.
        self._positions: dict[_Tokens, _Tokens] = {}
        self._renamed: dict[tuple[str, str], str] = {}
        self._ignored: list[_Record] = []
        self._unsupported: list[_Record] = []
        # (translated object holding a string $ref, source pointer of the object), rewritten once the map is complete.
        self._refs: list[tuple[dict[str, Any], str]] = []

    def run(self, schema: dict[str, Any] | bool) -> _Prepared:
        holder: list[Any] = [None]
        stack: list[_Step] = [(schema, (), (), holder, 0)]
        while stack:  # an explicit stack, so deep nesting cannot overflow
            value, source, doc, target, slot = stack.pop()
            self._positions[source] = doc
            target[slot] = self._object(value, source, doc, stack) if isinstance(value, dict) else value
        self._rewrite_refs()
        return _Prepared(holder[0], self._renamed, tuple(self._ignored), tuple(self._unsupported))

    def _object(self, obj: dict[str, Any], source: _Tokens, doc: _Tokens, stack: list[_Step]) -> dict[str, Any]:
        here = join_pointer("", *source)
        draft4 = self._draft is Draft.DRAFT4
        if "$ref" in obj and self._draft in _REF_OVERRIDES_SIBLINGS:
            obj = self._without_ref_siblings(obj, here, root=not source)
        array_items = "items" in obj and _has_2020_12_shape("prefixItems", obj["items"])
        dependent, dependencies_done = self._dependencies(obj, here)
        out: dict[str, Any] = {}
        for keyword, value in obj.items():
            if keyword == "definitions":
                if "$defs" in obj:
                    self._unsupported.append((here, keyword))  # it cannot take the name $defs
                elif _has_2020_12_shape("$defs", value):
                    self._descend(out, "$defs", keyword, value, (source, doc), stack)
                else:
                    out[keyword] = value
            elif keyword == "items" and array_items:
                self._descend(out, "prefixItems", keyword, value, (source, doc), stack)
            elif keyword == "prefixItems" and array_items:
                self._ignored.append((here, keyword))  # no draft translated here defines it
            elif keyword == "additionalItems":
                if not array_items:
                    self._ignored.append((here, keyword))
                elif _is_schema(value):
                    self._descend(out, "items", keyword, value, (source, doc), stack)
                else:
                    out[keyword] = value
            elif keyword == "dependencies" and dependencies_done:
                if dependent is not None:
                    out["dependentRequired"] = dependent
                    self._renamed[(join_pointer("", *doc), "dependentRequired")] = keyword
            elif keyword == "dependentRequired" and dependent is not None:
                self._ignored.append((here, keyword))  # drafts 04 to 07 do not define it
            elif (
                keyword == self._id_keyword
                and not source
                and self._draft in _REF_OVERRIDES_SIBLINGS
                and _is_anchor_fragment(value)
            ):
                # A root id that is a fragment alone ("#name" or "#/pointer", drafts 04 to 07) only names the
                # document, which "#" already does; draft 2020-12 forbids such an $id, so it is left out and
                # recorded (2.5).
                self._ignored.append((here, keyword))
            elif draft4 and keyword == "id":
                out["$id"] = value
                self._renamed[(join_pointer("", *doc), "$id")] = keyword
            elif draft4 and keyword == "$id" and "id" in obj:
                # Draft-04 does not define $id; this is what keyword loading would record.
                (self._unsupported if source else self._ignored).append((here, keyword))
            elif draft4 and keyword in _BOUND_OF and isinstance(value, bool):
                bound = obj.get(_BOUND_OF[keyword])
                if value and _is_number(bound):
                    out[keyword] = bound
                elif value:
                    out[keyword] = value  # no numeric bound to make exclusive: left for keyword loading
            elif draft4 and keyword in _EXCLUSIVE_OF and obj.get(_EXCLUSIVE_OF[keyword]) is True and _is_number(value):
                continue  # the exclusive keyword carries this bound
            elif keyword == "format" and isinstance(value, str) and value in self._later_formats:
                self._ignored.append((here, keyword))  # asserted from a later draft only: no effect here
            elif keyword == "$ref" and isinstance(value, str):
                out[keyword] = value
                self._refs.append((out, here))
            elif (
                keyword in SUBSCHEMA_KEYWORDS
                and (keyword in self._vocabulary or keyword == "$defs")
                and _has_2020_12_shape(keyword, value)
            ):
                self._descend(out, keyword, keyword, value, (source, doc), stack)
            else:
                out[keyword] = value
        return out

    def _without_ref_siblings(self, obj: dict[str, Any], here: str, root: bool) -> dict[str, Any]:
        """Drafts 04 to 07: leave out, as ignored, every keyword next to ``$ref`` but ref-target containers."""
        kept: dict[str, Any] = {}
        for keyword, value in obj.items():
            if keyword == "$ref" or keyword in _REF_TARGET_CONTAINERS or (root and keyword == "$schema"):
                kept[keyword] = value
            else:
                self._ignored.append((here, keyword))
        return kept

    def _dependencies(self, obj: dict[str, Any], here: str) -> tuple[dict[str, Any] | None, bool]:
        """Split ``dependencies``: ``(dependentRequired entries or None, whether dependencies is consumed)``.

        Array-valued entries become ``dependentRequired``. Schema-valued
        entries are unsupported, recorded once as ``(object, "dependencies")``.
        A ``dependencies`` of another shape is left for keyword loading.
        """
        value = obj.get("dependencies")
        if not isinstance(value, dict) or not all(
            isinstance(entry, list) or _is_schema(entry) for entry in value.values()
        ):
            return None, False
        if "dependentRequired" in obj and "dependentRequired" in self._vocabulary:
            self._unsupported.append((here, "dependencies"))  # 2019-09 already has dependentRequired here
            return None, True
        arrays = {name: entry for name, entry in value.items() if isinstance(entry, list)}
        if len(arrays) < len(value):
            self._unsupported.append((here, "dependencies"))
        return (arrays if arrays or not value else None), True

    def _descend(
        self,
        out: dict[str, Any],
        keyword: str,
        source_keyword: str,
        value: Any,
        where: tuple[_Tokens, _Tokens],
        stack: list[_Step],
    ) -> None:
        """Queue the subschemas of ``value`` (shape-checked by the caller) under ``keyword`` in ``out``."""
        source, doc = where
        if keyword != source_keyword:
            self._renamed[(join_pointer("", *doc), keyword)] = source_keyword
        shape = SUBSCHEMA_KEYWORDS[keyword]
        # Placeholders are replaced when the children are translated, before run() returns.
        if shape == "single":
            out[keyword] = None
            stack.append((value, (*source, source_keyword), (*doc, keyword), out, keyword))
        elif shape == "list":
            items: list[Any] = [None] * len(value)
            out[keyword] = items
            for index, child in enumerate(value):
                token = str(index)
                stack.append((child, (*source, source_keyword, token), (*doc, keyword, token), items, index))
        else:
            entries: dict[str, Any] = dict.fromkeys(value)
            out[keyword] = entries
            for name, child in value.items():
                stack.append((child, (*source, source_keyword, name), (*doc, keyword, name), entries, name))

    def _rewrite_refs(self) -> None:
        """Rewrite every local ``$ref`` through the pointer-rewrite map (see the module docstring)."""
        for obj, here in self._refs:
            ref = obj["$ref"]
            pointer = _local_pointer(ref)
            if pointer is None:
                continue  # remote, relative, or a plain name: keyword loading records it
            tokens = tuple(split_pointer(pointer))
            target = self._positions.get(tokens)
            if target is None:
                del obj["$ref"]  # its target is no translated schema position
                self._unsupported.append((here, "$ref"))
            elif target != tokens:
                obj["$ref"] = "#" + quote(join_pointer("", *target), safe=_FRAGMENT_SAFE)


# ---------------------------------------------------------------------------
# Step 5: keyword loading
# ---------------------------------------------------------------------------

_Action: TypeAlias = Literal["load", "skip", "ignore", "unsupported"]

# A schema position waiting to be built: (value, pointer in the prepared
# document, pointer in the source, inside an embedded resource, the container
# that receives the node, and its slot there).
_Task: TypeAlias = tuple[Any, str, str, bool, Any, Any]


def _is_schema(value: Any) -> bool:
    return isinstance(value, (dict, bool))


_ANNOTATION_SHAPES: Final[dict[str, type | tuple[type, ...]]] = {
    "$comment": str,
    "deprecated": bool,
    "description": str,
    "examples": list,
    "readOnly": bool,
    "title": str,
    "writeOnly": bool,
}


def _is_anchor_fragment(value: Any) -> bool:
    """Whether an id value is a fragment alone: a plain name (``#name``), which drafts 04 to 07 read as an
    anchor, or a JSON Pointer (``#/definitions/x``), which only names the document the id already sits in."""
    return isinstance(value, str) and value.startswith("#") and len(value) > 1


def _has_2020_12_shape(keyword: str, value: Any) -> bool:
    """Whether a supported keyword's value has its draft 2020-12 shape (the meta-schema checked the rest)."""
    shape = SUBSCHEMA_KEYWORDS.get(keyword)
    if shape == "single":
        return _is_schema(value)
    if shape == "list":
        return isinstance(value, list) and all(_is_schema(child) for child in value)
    if shape == "map":
        return isinstance(value, dict) and all(_is_schema(child) for child in value.values())
    if keyword in ("exclusiveMaximum", "exclusiveMinimum"):
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if keyword == "$ref":
        return isinstance(value, str)
    return True


def _local_pointer(ref: str) -> str | None:
    """The JSON Pointer of a local ``$ref`` (``#`` plus a pointer), or None for any other reference."""
    if not ref.startswith("#"):
        return None  # remote or relative
    pointer = unquote(ref[1:])
    if pointer and not pointer.startswith("/"):
        return None  # a plain-name fragment (an anchor)
    try:
        split_pointer(pointer)
    except ValueError:
        return None
    return pointer


def _sorted_records(records: Iterable[_Record]) -> list[_Record]:
    return sorted(set(records), key=lambda record: (codepoint_key(record[0]), codepoint_key(record[1])))


class _Builder:
    """Builds the Schema_Model of one prepared document (see the module docstring)."""

    __slots__ = (
        "_assumed",
        "_defined",
        "_id_keyword",
        "_ignored",
        "_prepared",
        "_refs",
        "_unsupported",
        "_vocabulary",
    )

    def __init__(self, prepared: _Prepared, draft: Draft, dialect_assumed: bool) -> None:
        self._prepared = prepared
        self._assumed = dialect_assumed
        self._vocabulary = VOCABULARIES[draft]
        self._defined = VOCABULARIES[Draft.DRAFT202012] | self._vocabulary
        self._id_keyword = "id" if draft is Draft.DRAFT4 else "$id"
        self._ignored: list[_Record] = list(prepared.ignored)
        self._unsupported: list[_Record] = list(prepared.unsupported)
        # (node holding a local $ref, source pointer of the node, the $ref's pointer), checked once the tree is built.
        self._refs: list[tuple[SchemaNode, str, str]] = []

    def build(self) -> SchemaModel:
        holder: list[Any] = [None]
        stack: list[_Task] = [(self._prepared.doc, "", "", False, holder, 0)]
        while stack:
            value, pointer, source, scoped, target, slot = stack.pop()
            target[slot] = self._node(value, pointer, source, scoped, stack)
        root: SchemaNode = holder[0]
        self._check_refs(root)
        model = SchemaModel(
            root,
            dialect_assumed=self._assumed,
            ignored=_sorted_records(self._ignored),
            unsupported=_sorted_records(self._unsupported),
        )
        model.renumber()
        return model

    def _source_keyword(self, pointer: str, keyword: str) -> str:
        return self._prepared.renamed.get((pointer, keyword), keyword)

    def _starts_resource(self, value: dict[str, Any], pointer: str) -> bool:
        """Whether a schema object below the root starts an embedded schema resource.

        Translation has already renamed draft-04 ``id`` to ``$id`` and left out
        an ``$id`` next to ``$ref`` in drafts 04 to 07.
        """
        if "$id" not in value or self._source_keyword(pointer, "$id") != self._id_keyword:
            return False
        ident = value["$id"]
        return isinstance(ident, str) and not ident.startswith("#")

    def _classify(self, keyword: str, source_keyword: str, value: Any, root: bool) -> _Action:
        if keyword.startswith("x-"):
            return "load"
        if keyword in ANNOTATION_KEYWORDS:
            # An annotation whose value lacks its draft 2020-12 shape (a string "deprecated", a scalar
            # "examples") would fail the 2020-12 meta-schema when emitted; the source draft either did not
            # define it or did not check it there, so it is left out and recorded (2.5).
            shape = _ANNOTATION_SHAPES.get(keyword)
            return "load" if shape is None or isinstance(value, shape) else "ignore"
        if keyword == "$schema":
            if root:
                # A $schema at the root is consumed by dialect detection; when the
                # dialect was assumed despite it, its URI is not recognized.
                return "unsupported" if self._assumed else "skip"
            return "ignore"
        if keyword == "$id":
            if not root:
                return "unsupported"
            return "load" if source_keyword in self._vocabulary else "ignore"
        if keyword in SUPPORTED_KEYWORDS:
            if source_keyword not in self._vocabulary and keyword != "$defs":
                return "ignore"  # the declared draft does not define it, so it has no effect there
            return "load" if _has_2020_12_shape(keyword, value) else "unsupported"
        if source_keyword in self._defined:
            return "unsupported"
        return "ignore"

    def _node(self, value: Any, pointer: str, source: str, scoped: bool, stack: list[_Task]) -> SchemaNode:
        if isinstance(value, bool):
            return SchemaNode(boolean=value)
        node = SchemaNode()
        root = not pointer
        if not root and not scoped:
            scoped = self._starts_resource(value, pointer)
        for keyword, item in value.items():
            source_keyword = self._source_keyword(pointer, keyword)
            action = self._classify(keyword, source_keyword, item, root)
            if action == "ignore":
                self._ignored.append((source, source_keyword))
            elif action == "unsupported":
                self._unsupported.append((source, source_keyword))
            elif action == "load":
                self._load(node, keyword, source_keyword, item, (pointer, source, scoped), stack)
        return node

    def _load(
        self,
        node: SchemaNode,
        keyword: str,
        source_keyword: str,
        value: Any,
        where: tuple[str, str, bool],
        stack: list[_Task],
    ) -> None:
        pointer, source, scoped = where
        shape = SUBSCHEMA_KEYWORDS.get(keyword)
        if shape is None:
            if keyword == "$ref":
                target = None if scoped else _local_pointer(value)
                if target is None:
                    self._unsupported.append((source, source_keyword))
                    return
                self._refs.append((node, source, target))
            node.kw[keyword] = copy.deepcopy(value)
            return
        child_pointer = join_pointer(pointer, keyword)
        child_source = join_pointer(source, source_keyword)
        # Placeholders are replaced when the children are built, before build() returns.
        if shape == "single":
            node.kw[keyword] = None
            stack.append((value, child_pointer, child_source, scoped, node.kw, keyword))
        elif shape == "list":
            items: list[Any] = [None] * len(value)
            node.kw[keyword] = items
            for index, child in enumerate(value):
                stack.append(
                    (child, join_pointer(child_pointer, index), join_pointer(child_source, index), scoped, items, index)
                )
        else:
            entries: dict[str, Any] = dict.fromkeys(value)
            node.kw[keyword] = entries
            for name, child in value.items():
                stack.append(
                    (child, join_pointer(child_pointer, name), join_pointer(child_source, name), scoped, entries, name)
                )

    def _check_refs(self, root: SchemaNode) -> None:
        """Leave out, as unsupported, every local ``$ref`` whose pointer does not lead to a subschema."""
        for node, source, target in self._refs:
            try:
                root.resolve(target)
            except (KeyError, ValueError):
                del node.kw["$ref"]
                self._unsupported.append((source, "$ref"))


# ---------------------------------------------------------------------------
# The Schema_Loader
# ---------------------------------------------------------------------------


class SchemaLoader:
    """Parses JSON Schema documents into Schema_Models (see the module docstring).

    ``validator`` runs the meta-schema checks; a new
    :class:`~jsinfer.io.validator.Validator` is made when none is given.
    """

    __slots__ = ("_validator",)

    def __init__(self, validator: Validator | None = None) -> None:
        if validator is not None and not isinstance(validator, Validator):
            raise TypeError(f"validator must be a Validator or None, got {type(validator).__name__}")
        self._validator = validator if validator is not None else Validator()

    def load(self, source: bytes | str | dict[str, Any] | bool) -> SchemaModel:
        """Load one schema document.

        ``source`` is the document's bytes, its text, or its parsed value.
        Raises :class:`SchemaLoadError` when the document is not valid JSON,
        has a root that is neither an object nor a boolean, or fails the
        meta-schema of its declared or assumed draft (2.12), and
        ``TypeError`` when ``source`` is of another Python type.
        """
        schema = _parse(source)
        if not isinstance(schema, (dict, bool)):
            raise SchemaLoadError(
                "root", f"the schema root is {_kind(schema)}; a schema must be a JSON object or a boolean"
            )
        draft, assumed = dialect_of(schema)
        errors = list(dict.fromkeys(self._validator.meta_errors(schema, draft)))
        if errors:
            how = "assumed" if assumed else "declared"
            more = f" (and {len(errors) - 1} more)" if len(errors) > 1 else ""
            raise SchemaLoadError(
                "meta",
                f"the schema is not valid against the {_DRAFT_NAMES[draft]} meta-schema ({how}): {errors[0]}{more}",
                errors=errors,
            )
        return _Builder(_translate(schema, draft), draft, assumed).build()


# ---------------------------------------------------------------------------
# Explanation files (Requirements 26.5, 26.8)
# ---------------------------------------------------------------------------

ExplanationLoadCheck: TypeAlias = Literal["json", "format", "structure"]
# The checks of load_explanation, in the order they run:
#   json       the file is not valid JSON (or a parsed source is not a JSON value)
#   format     the "format" member is missing or is not the supported version
#   structure  anything else: the root is not an object, a member is unknown or
#              missing, "records" is not an object, a key is not a non-empty
#              JSON Pointer, or a record is malformed
EXPLANATION_LOAD_CHECKS: Final[tuple[ExplanationLoadCheck, ...]] = ("json", "format", "structure")

_EXPLANATION: Final = "the explanation file"
_EXPLANATION_MEMBERS: Final = ("format", "records")


class ExplanationLoadError(ValueError):
    """An explanation file that :func:`load_explanation` cannot read.

    ``check`` names the failed check (:data:`EXPLANATION_LOAD_CHECKS`) and
    ``reason`` the cause. ``line`` and ``col`` are the 1-based location of a
    JSON syntax error (``None`` otherwise). ``location`` is the JSON Pointer,
    within the explanation document, of the offending member (``""`` for
    the top level), for example ``/records/~1type/confidence``. The message
    does not name the file; a caller that read it from a path adds that.
    """

    def __init__(
        self,
        check: ExplanationLoadCheck,
        reason: str,
        line: int | None = None,
        col: int | None = None,
        location: str = "",
    ) -> None:
        if check not in EXPLANATION_LOAD_CHECKS:
            raise ValueError(
                f"ExplanationLoadError check must be one of {list(EXPLANATION_LOAD_CHECKS)}, got {check!r}"
            )
        if not isinstance(reason, str) or not reason:
            raise ValueError("ExplanationLoadError reason must be a non-empty str")
        for name, number in (("line", line), ("col", col)):
            if number is not None and (type(number) is not int or number < 1):
                raise ValueError(f"ExplanationLoadError {name} must be None or an int >= 1, got {number!r}")
        if not isinstance(location, str):
            raise TypeError(f"ExplanationLoadError location must be a str, got {type(location).__name__}")
        super().__init__(check, reason, line, col, location)  # args survive pickling
        self.check: ExplanationLoadCheck = check
        self.reason = reason
        self.line = line
        self.col = col
        self.location = location

    def __str__(self) -> str:
        if self.line is not None:
            if self.col is None:
                return f"{self.reason} (line {self.line})"
            return f"{self.reason} (line {self.line}, column {self.col})"
        where = repr(self.location) if self.location else "the top level"
        return f"invalid explanation file at {where}: {self.reason}"


def _explanation_kind(value: Any) -> str:
    if isinstance(value, bool):
        return "a boolean"
    if isinstance(value, dict):
        return "an object"
    return _kind(value)


def load_explanation(source: bytes | str | dict[str, Any]) -> ExplanationFile:
    """Read an explanation file written by the Schema_Emitter (26.5, 26.8).

    ``source`` is the file's bytes, its text, or its parsed value. Bytes are
    parsed with :func:`~jsinfer.io.ingest.parse_document`, as for schemas.
    The document must be ``{"format": 1, "records": {...}}`` with exactly
    those members (``io.emit.EXPLANATION_FORMAT``); each key of ``records``
    must be a non-empty JSON Pointer and each value a record in the form of
    ``ProvenanceRecord.to_json`` (``ProvenanceRecord.from_json``, which
    accepts any spelling of numbers and any member order). Whether the
    pointers resolve to Constraints of a schema is not checked here; the
    ``explain`` command resolves them against its schema.

    For every file ``data`` written by ``ExplanationFile.dumps``,
    ``load_explanation(data).dumps() == data``. Raises
    :class:`ExplanationLoadError` for a file that fails a check (in the order
    of :data:`EXPLANATION_LOAD_CHECKS`, so a file of another format version
    reports the version), and ``TypeError`` when ``source`` is of another
    Python type.
    """
    if not (source is None or isinstance(source, (bytes, str, bool, int, float, list, dict))):
        raise TypeError(
            f"an explanation file source must be bytes, str, or a parsed JSON value, got {type(source).__name__}"
        )
    try:
        value = _parse(source, _EXPLANATION)
    except SchemaLoadError as exc:
        raise ExplanationLoadError("json", exc.reason, exc.line, exc.col) from None
    if not isinstance(value, dict):
        raise ExplanationLoadError(
            "structure",
            f"expected an object with the members {list(_EXPLANATION_MEMBERS)}, got {_explanation_kind(value)}",
        )

    if "format" not in value:
        raise ExplanationLoadError("format", "missing member 'format' (the explanation file format version)")
    version = value["format"]
    if isinstance(version, bool) or not isinstance(version, int):
        raise ExplanationLoadError(
            "format",
            f"expected the integer {EXPLANATION_FORMAT}, got {_explanation_kind(version)}",
            location="/format",
        )
    if version != EXPLANATION_FORMAT:
        raise ExplanationLoadError(
            "format",
            f"unsupported explanation file format {version}; the supported format is {EXPLANATION_FORMAT}",
            location="/format",
        )

    for member in value:
        if member not in _EXPLANATION_MEMBERS:
            raise ExplanationLoadError(
                "structure", f"unknown member {member!r}; the members are {list(_EXPLANATION_MEMBERS)}"
            )
    if "records" not in value:
        raise ExplanationLoadError("structure", "missing member 'records'")
    entries = value["records"]
    if not isinstance(entries, dict):
        raise ExplanationLoadError(
            "structure", f"expected an object, got {_explanation_kind(entries)}", location="/records"
        )

    records: dict[str, ProvenanceRecord] = {}
    for pointer, entry in entries.items():
        location = join_pointer("/records", pointer)
        try:
            split_pointer(pointer)
        except ValueError as exc:
            raise ExplanationLoadError("structure", f"the key is not a JSON Pointer: {exc}", location=location) from None
        if not pointer:
            raise ExplanationLoadError(
                "structure",
                "the empty JSON Pointer names the root schema object, which is not a Constraint",
                location=location,
            )
        try:
            records[pointer] = ProvenanceRecord.from_json(entry, location=location)
        except ProvenanceFormatError as exc:
            raise ExplanationLoadError("structure", exc.reason, location=exc.location) from None
    return ExplanationFile(records)
