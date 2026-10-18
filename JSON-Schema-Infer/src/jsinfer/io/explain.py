"""``explain``: the Provenance_Record of one Constraint as plain text (Requirements 26.9, 26.12; task 17.3).

:func:`explain` takes a schema, the explanation file written with it, and a
JSON Pointer, and returns the text of
:func:`~jsinfer.core.provenance.render_record` for the record keyed by the
pointer. ``core.provenance`` re-exports it. ``jsinfer explain SCHEMA
EXPLANATION POINTER`` (task 18.2) passes the two files as
:class:`pathlib.Path` values, writes the text to standard output, and
exits with status 0; on an :class:`~jsinfer.core.provenance.ExplainError`
it writes the error's message to standard error, prints no record, and
exits with status 2. The function returns the text instead of printing
it, so the CLI owns the streams.

Inputs
    ``schema`` and ``explanation`` are each one of:

    - an ``os.PathLike`` (such as :class:`pathlib.Path`): the file is read
      as bytes, and messages name it (``the schema file 'out/schema.json'``);
    - ``bytes``, the content of the file, or a ``str``, its text, as for
      ``SchemaLoader.load`` and ``load_explanation``. A ``str`` is never
      taken for a path, so the two cannot be confused;
    - a parsed JSON value, or for ``explanation`` an
      :class:`~jsinfer.io.emit.ExplanationFile`.

    In-memory inputs are named ``the schema`` and ``the explanation file``.
    Any other Python type raises ``TypeError``, as does a ``pointer`` that
    is not a ``str``.

Checks
    They run in argument order and stop at the first failure
    (``core.provenance.EXPLAIN_CHECKS``). Each raises an ``ExplainError``
    whose message names the failed input and the cause:

    1. ``read``: a path cannot be read (missing, a directory, no
       permission). The reason is the OS's.
    2. ``parse``: the schema fails the Schema_Loader (not JSON, a root that
       is neither an object nor a boolean, or a meta-schema violation,
       2.12), or the explanation file fails ``load_explanation``. The reason
       is the loader's message, with the line and column of a JSON syntax
       error.
    3. ``syntax``: the pointer is not an RFC 6901 JSON Pointer: it is
       neither empty nor starts with ``/``, or holds a ``~`` not followed by
       ``0`` or ``1``. The URI fragment form (``#/type``) gets a hint.
    4. ``resolve``: the pointer must name a location in the schema document
       as given, and that location must be a Constraint (below).
    5. ``entry``: the explanation file must hold a record keyed by the
       pointer. Keys are compared as strings, in the escaped form the
       Schema_Emitter writes (``/properties/a~1b/type``).

Constraint locations
    The Constraints of a schema are those of the Schema_Emitter's layout of
    the loaded schema (``SchemaEmitter.layout``), at the granularity of the
    design's table (26.5): a keyword such as ``…/type``, ``…/maximum``, or
    ``…/properties``; each ``…/required/<i>`` and ``…/dependentRequired/<k>``
    entry; and each ``…/anyOf/<i>`` and ``…/oneOf/<i>`` branch. These are
    exactly the keys that the explanation file of the schema holds (26.7).
    For a schema written by the Schema_Emitter, with or without inline
    provenance, the layout is the document itself (the 2.7 round trip), so
    a pointer means the same in both. A schema from elsewhere is read in
    the canonical form the Schema_Emitter would write it in (``required``
    sorted, older drafts translated), and the pointer must also name a
    location of the document as given.

    A pointer to a location that is not a Constraint fails, and the reason
    says what it names instead:

    - a schema object: the root, ``…/properties/<name>``,
      ``…/$defs/<name>``, ``…/prefixItems/<i>``, and so on. 26.9 asks for a
      pointer that resolves to a Constraint and 26.12 rejects any other, so
      a schema object is an error rather than a request for all of its
      records. The reason lists the pointers of the Constraints the schema
      object holds, so one can be picked;
    - ``x-provenance``, or a location inside it: the inline copy of the
      records, which is not a Constraint (26.7);
    - a keyword that is never a Constraint (``$defs``, ``$schema``,
      ``$id``, and annotations such as ``title``);
    - a ``required``, ``dependentRequired``, ``anyOf``, or ``oneOf``
      keyword as a whole: its entries are the Constraints, and the reason
      lists them;
    - a keyword that the Schema_Loader does not support or ignores, which
      the loaded schema leaves out (2.6, 2.10);
    - a location inside a keyword's value (``…/enum/0``): the reason names
      the Constraint that holds it.
"""

from __future__ import annotations

import os
import re
from collections.abc import Mapping
from typing import Any, Final, TypeAlias

from jsinfer.core.jsonvalue import JSON
from jsinfer.core.provenance import ExplainError, ExplainInput, ProvenanceRecord, render_record
from jsinfer.core.schema_model import (
    ITEMIZED_KEYWORDS,
    NON_CONSTRAINT_KEYWORDS,
    SUBSCHEMA_KEYWORDS,
    SchemaModel,
    join_pointer,
    split_pointer,
)
from jsinfer.io.emit import PROVENANCE_KEYWORD, ExplanationFile, PointerMap, SchemaEmitter
from jsinfer.io.ingest import parse_document
from jsinfer.io.load import ExplanationLoadError, SchemaLoader, SchemaLoadError, load_explanation

__all__ = ["ExplanationSource", "SchemaSource", "explain"]

SchemaSource: TypeAlias = "os.PathLike[str] | bytes | str | JSON"
ExplanationSource: TypeAlias = "os.PathLike[str] | bytes | str | JSON | ExplanationFile"

_IN_MEMORY_NAMES: Final[Mapping[str, str]] = {"schema": "the schema", "explanation": "the explanation file"}
_FILE_NAMES: Final[Mapping[str, str]] = {"schema": "the schema file", "explanation": "the explanation file"}

# A '~' that does not start one of the two RFC 6901 escapes.
_BAD_ESCAPE: Final = re.compile(r"~(?![01])")


def explain(schema: SchemaSource, explanation: ExplanationSource, pointer: str) -> str:
    """The plain text of the Provenance_Record of the Constraint at ``pointer`` (26.9).

    ``schema`` is the emitted schema and ``explanation`` its explanation
    file, each a path, bytes, text, or a parsed value (see Inputs in the
    module docstring). The pointer must resolve to a Constraint of the
    schema and have an entry in the explanation file. Returns
    ``render_record(record, pointer)``: a first line naming the
    Constraint, then every field of the record. Raises
    :class:`~jsinfer.core.provenance.ExplainError` for each failure of 26.12
    (see Checks in the module docstring).
    """
    if not isinstance(pointer, str):
        raise TypeError(f"pointer must be a str JSON Pointer, got {type(pointer).__name__}")
    raw, model, schema_name = _load_schema(schema)
    records, explanation_name = _load_explanation(explanation)
    name = f"the JSON Pointer {pointer!r}"
    tokens = _tokens(pointer, name)
    _resolve(raw, model, tokens, pointer, name, schema_name)
    record = records.get(pointer)
    if record is None:
        raise ExplainError("pointer", "entry", name, f"has no entry in {explanation_name}")
    return render_record(record, pointer)


# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------


def _is_in_memory(source: Any) -> bool:
    return source is None or isinstance(source, (bytes, str, bool, int, float, list, dict))


def _source(source: Any, input: ExplainInput) -> tuple[Any, str]:
    """The bytes of a path, or the in-memory source as it is, with the name messages use for it."""
    if isinstance(source, os.PathLike):
        name = f"{_FILE_NAMES[input]} {os.fsdecode(source)!r}"
        try:
            with open(source, "rb") as handle:
                return handle.read(), name
        except OSError as error:
            reason = error.strerror or str(error) or type(error).__name__
            raise ExplainError(input, "read", name, f"cannot be read: {reason}") from None
    if _is_in_memory(source):
        return source, _IN_MEMORY_NAMES[input]
    kinds = "a path, bytes, str, or a parsed JSON value" + (", or an ExplanationFile" if input == "explanation" else "")
    raise TypeError(f"{input} must be {kinds}, got {type(source).__name__}")


def _load_schema(source: Any) -> tuple[Any, SchemaModel, str]:
    """The parsed schema document, its Schema_Model, and its name."""
    data, name = _source(source, "schema")
    try:
        model = SchemaLoader().load(data)
    except SchemaLoadError as error:
        raise ExplainError("schema", "parse", name, f"cannot be parsed: {error}") from None
    if isinstance(data, str):
        data = data.encode("utf-8")  # the loader parsed this text, so it is valid Unicode
    raw = parse_document(data)[0] if isinstance(data, bytes) else data
    return raw, model, name


def _load_explanation(source: Any) -> tuple[Mapping[str, ProvenanceRecord], str]:
    """The records of the explanation file, keyed by pointer, and its name."""
    if isinstance(source, ExplanationFile):
        return source.records, _IN_MEMORY_NAMES["explanation"]
    data, name = _source(source, "explanation")
    try:
        loaded = load_explanation(data)
    except ExplanationLoadError as error:
        raise ExplainError("explanation", "parse", name, f"cannot be parsed: {error}") from None
    return loaded.records, name


# ---------------------------------------------------------------------------
# The pointer
# ---------------------------------------------------------------------------


def _tokens(pointer: str, name: str) -> list[str]:
    """The reference tokens of ``pointer``; a ``syntax`` ExplainError when it is not a JSON Pointer."""
    if pointer and not pointer.startswith("/"):
        reason = "is not a valid JSON Pointer: a JSON Pointer is empty or starts with '/'"
        if pointer.startswith("#"):
            reason += "; write it without the '#' of the URI fragment form"
        raise ExplainError("pointer", "syntax", name, reason)
    bad = _BAD_ESCAPE.search(pointer)
    if bad is not None:
        raise ExplainError(
            "pointer",
            "syntax",
            name,
            f"is not a valid JSON Pointer: the '~' at index {bad.start()} is not followed by '0' or '1' "
            "(write '~0' for '~' and '~1' for '/')",
        )
    return split_pointer(pointer)


def _index(token: str, length: int) -> int | None:
    """The array index an RFC 6901 token names (no sign, no leading zero), or None when it names none."""
    if not token.isascii() or not token.isdigit() or (len(token) > 1 and token[0] == "0"):
        return None
    index = int(token)
    return index if index < length else None


def _kind(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "a boolean"
    if isinstance(value, str):
        return "a string"
    return "a number"


def _location(pointer: str) -> str:
    return "the root" if not pointer else f"the location {pointer!r}"


def _resolve(raw: Any, model: SchemaModel, tokens: list[str], pointer: str, name: str, schema_name: str) -> None:
    """Check that ``pointer`` names a location of the schema document, and that it is a Constraint."""
    value = raw
    for depth, token in enumerate(tokens):
        here = join_pointer("", *tokens[:depth])
        if isinstance(value, dict):
            found = token in value
            detail = f"{_location(here)} has no member {token!r}"
            value = value.get(token)
        elif isinstance(value, list):
            index = _index(token, len(value))
            found = index is not None
            detail = f"{_location(here)} is an array of length {len(value)}, which has no element {token!r}"
            value = value[index] if index is not None else None
        else:
            found = False
            detail = f"{_location(here)} holds {_kind(value)}, which has no members"
        if not found:
            raise ExplainError("pointer", "resolve", name, f"does not resolve to a location in {schema_name}: {detail}")
    _, layout = SchemaEmitter().layout(model)
    constraints = list(layout.values())  # in document order
    if pointer not in constraints:
        reason = _not_a_constraint(raw, model, layout, constraints, tokens)
        raise ExplainError("pointer", "resolve", name, f"does not resolve to a Constraint in {schema_name}: {reason}")


# ---------------------------------------------------------------------------
# Why a location is not a Constraint
# ---------------------------------------------------------------------------


def _not_a_constraint(raw: Any, model: SchemaModel, layout: PointerMap, constraints: list[str], tokens: list[str]) -> str:
    """What the location at ``tokens`` (which exists and is no Constraint) names instead."""
    node = raw  # the schema object the walk is at
    base = ""  # its pointer
    i = 0
    while i < len(tokens):
        keyword = tokens[i]
        # The location exists, so the schema object is a JSON object (a boolean schema has no members).
        value = node[keyword]
        here = join_pointer(base, keyword)
        last = i + 1 == len(tokens)
        if keyword == PROVENANCE_KEYWORD:
            verb = "names" if last else "points inside"
            return (
                f"it {verb} the inline provenance annotation at {here!r}, which is not a Constraint: "
                "it holds copies of the records of the Constraints of its schema object"
            )
        if last:
            return _keyword_reason(model, constraints, base, keyword, here)
        shape = SUBSCHEMA_KEYWORDS.get(keyword)
        child: Any = None
        step = 1
        if shape == "single":
            child = value
        elif shape == "list" and isinstance(value, list):
            child, step = value[_index(tokens[i + 1], len(value))], 2  # type: ignore[index]  # it resolved
        elif shape == "map" and isinstance(value, dict):
            child, step = value[tokens[i + 1]], 2
        if not isinstance(child, (dict, bool)):
            return _inside(constraints, tokens, keyword, here)
        node, base, i = child, join_pointer(base, *tokens[i : i + step]), i + step
    return _schema_object_reason(model, layout, base)


def _listing(pointers: list[str]) -> str:
    return ", ".join(repr(pointer) for pointer in pointers)


def _keyword_reason(model: SchemaModel, constraints: list[str], base: str, keyword: str, here: str) -> str:
    if keyword in ITEMIZED_KEYWORDS:
        prefix = here + "/"
        entries = [p for p in constraints if p.startswith(prefix) and "/" not in p[len(prefix) :]]
        if not entries:
            return f"each entry of the keyword {keyword!r} at {here!r} is its own Constraint, and it has none"
        return f"each entry of the keyword {keyword!r} at {here!r} is its own Constraint: {_listing(entries)}"
    if keyword in NON_CONSTRAINT_KEYWORDS:
        return (
            f"the keyword {keyword!r} at {here!r} is not a Constraint "
            "($defs, $schema, $id, and annotation keywords such as title are none)"
        )
    if (base, keyword) in model.unsupported:
        return f"the Schema_Loader does not support the keyword {keyword!r} at {here!r}, so it is no Constraint"
    if (base, keyword) in model.ignored:
        return f"the Schema_Loader ignores the keyword {keyword!r} at {here!r}, so it is no Constraint"
    return f"the keyword {keyword!r} at {here!r} is not a Constraint of the loaded schema"


def _inside(constraints: list[str], tokens: list[str], keyword: str, here: str) -> str:
    known = set(constraints)  # membership only
    prefixes = (join_pointer("", *tokens[:end]) for end in range(len(tokens) - 1, 0, -1))
    holder = next((prefix for prefix in prefixes if prefix in known), None)
    reason = f"it points inside the value of the keyword {keyword!r} at {here!r}"
    if holder == here:
        reason += ", which is one Constraint as a whole"
    elif holder is not None:
        reason += f", which belongs to the Constraint at {holder!r}"
    return reason


def _schema_object_reason(model: SchemaModel, layout: PointerMap, base: str) -> str:
    what = "the root schema object" if not base else f"the schema object at {base!r}"
    try:
        node = model.resolve(base)
    except (KeyError, ValueError):
        return f"it names {what}, which is not a Constraint"
    if node.boolean is not None:
        return f"it names {what}, the boolean schema {str(node.boolean).lower()}, which holds no Constraint"
    owned = [pointer for cid, pointer in layout.items() if cid.nid == node.nid]
    if not owned:
        return f"it names {what}, which is not a Constraint and holds none"
    return f"it names {what}, which is not a Constraint; the Constraints it holds are {_listing(owned)}"
