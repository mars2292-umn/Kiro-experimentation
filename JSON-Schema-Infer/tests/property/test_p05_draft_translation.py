"""Property 5: Draft translation preserves verdicts (task 6.6).

For any generated draft-04, -06, -07, or 2019-09 schema in the translatable
grammar and any generated instance, the Validator's verdict against the
original schema under its declared draft equals its verdict against the
emitted draft 2020-12 form of the loaded schema, with ``format`` asserted for
both (Requirements 2.5, 2.9, 34.5). Every generated schema is also fully
supported after loading.

The schemas and instances come from strategies local to this module.

Schemas
    Each document declares one of the four drafts and uses only keywords that
    the draft defines, with values its meta-schema accepts: ``type``,
    ``enum``, ``format``, ``pattern``, the bounds (draft-04 boolean
    ``exclusiveMinimum``/``exclusiveMaximum`` next to their bound, numeric
    ones from draft-06), ``uniqueItems``, ``required``, array-valued
    ``dependencies`` (or ``dependentRequired`` in 2019-09), ``properties``,
    ``patternProperties``, ``additionalProperties``, single and array-form
    ``items``, ``additionalItems``, ``allOf``/``anyOf``/``oneOf``, ``not``,
    and from later drafts ``const``, ``contains``, ``propertyNames``,
    ``if``/``then``/``else``, and boolean schemas. The root may carry an id
    (draft-04 ``id``, ``$id`` otherwise) and a definitions container
    (``definitions``, sometimes ``$defs``), and keywords may be followed by a
    supported keyword the declared draft does not define (``const`` in
    draft-04, ``prefixItems`` before 2020-12, ...), which neither reading
    applies. ``additionalItems`` also appears without array-form ``items``,
    where it has no effect. Formats include ones that jsonschema-rs asserts
    only from a later draft (``uuid`` in draft-07, ``uri-reference`` in
    draft-04).

    About a quarter of the schema objects get a local ``$ref``, placed
    among their keywords, so that drafts 04 to 07 ignore those keywords. A
    reference targets a schema position inside a definitions entry (the entry
    itself, or deeper: ``#/definitions/a/items/0`` and
    ``#/definitions/a/additionalItems`` exercise the pointer-rewrite map),
    percent-encoded with JSON Pointer escapes. Below the root, under an
    instance-consuming keyword (``properties``, ``items``, ...), it may also
    be ``#``. Targets never pass through keywords that ``$ref`` hides or that
    the draft does not define, so every reference survives translation.

    ``$ref`` cycles that consume no instance would not terminate under
    validation, so the references are kept acyclic except through ``#``:
    a definitions entry refers only to positions in earlier entries and never
    to ``#``, and a ``#`` reference sits in the root's own tree below an
    instance-consuming keyword. Every cycle therefore passes through ``#``
    and descends into the instance.

Instances
    Instances are drawn by a guide that walks the original schema as the
    declared draft reads it: it follows ``$ref`` (in 2019-09, sometimes the
    siblings instead), picks ``enum``/``const`` members and combinator
    branches, picks numbers at, just inside, and just outside each bound,
    string lengths around ``minLength``/``maxLength``, valid and invalid
    examples for each format and pattern, array lengths around the prefix
    and ``minItems``/``maxItems`` (with ``contains`` matches and duplicates),
    and objects with required, optional, pattern-matching, dependent, and
    extra properties. At each step it may emit an unrelated JSON value
    instead, so the instances mix accepted and rejected ones; Hypothesis
    events report the balance.

    Draft-04 counts only numbers written without a fraction as integers,
    while draft 2020-12 counts ``1.0`` as one too: that verdict difference is
    a property of the drafts, not of the translation, so draft-04 instances
    hold no integral floats (each is written as the integer it equals).
    Draft-04 schemas hold none either (jsonschema-rs compiles ``1.0`` and
    ``1`` differently there). Instances of the later drafts include
    integral floats.

SchemaStore test documents (Requirement 2.9 on the corpus) are added in task 25.7.

**Validates: Requirements 2.5, 2.9, 34.5**
"""

# Feature: json-schema-inference, Property 5: Draft translation preserves verdicts

from __future__ import annotations

import math
from typing import Any
from urllib.parse import quote, unquote

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.schema_model import join_pointer, split_pointer
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoader
from jsinfer.io.validator import Draft, Validator
from tests.strategies import json_values

VALIDATOR = Validator()
LOADER = SchemaLoader(VALIDATOR)
EMITTER = SchemaEmitter()

D4, D6, D7, D2019 = Draft.DRAFT4, Draft.DRAFT6, Draft.DRAFT7, Draft.DRAFT201909
DRAFTS = (D4, D6, D7, D2019)
DRAFT_NAMES = {D4: "draft-04", D6: "draft-06", D7: "draft-07", D2019: "2019-09"}

# Drafts in which "$ref" overrides its siblings.
REF_HIDES_SIBLINGS = frozenset({D4, D6, D7})

# ---------------------------------------------------------------------------
# The translatable grammar, per draft
# ---------------------------------------------------------------------------

_VALUE_KEYWORDS_04 = (
    "type", "enum", "format", "pattern", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum",
    "minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties", "uniqueItems",
    "required", "dependencies",
)  # fmt: skip
_SUBSCHEMA_KEYWORDS_04 = (
    "properties", "patternProperties", "additionalProperties", "items", "additionalItems",
    "allOf", "anyOf", "oneOf", "not",
)  # fmt: skip

VALUE_KEYWORDS: dict[Draft, tuple[str, ...]] = {
    D4: _VALUE_KEYWORDS_04,
    D6: (*_VALUE_KEYWORDS_04, "const"),
    D7: (*_VALUE_KEYWORDS_04, "const"),
    D2019: (*_VALUE_KEYWORDS_04, "const", "dependentRequired"),
}
SUBSCHEMA_KEYWORDS: dict[Draft, tuple[str, ...]] = {
    D4: _SUBSCHEMA_KEYWORDS_04,
    D6: (*_SUBSCHEMA_KEYWORDS_04, "contains", "propertyNames"),
    D7: (*_SUBSCHEMA_KEYWORDS_04, "contains", "propertyNames", "if", "then", "else"),
    D2019: (*_SUBSCHEMA_KEYWORDS_04, "contains", "propertyNames", "if", "then", "else"),
}
DEFINED: dict[Draft, frozenset[str]] = {
    draft: frozenset(VALUE_KEYWORDS[draft]) | frozenset(SUBSCHEMA_KEYWORDS[draft]) for draft in DRAFTS
}

# Supported keywords the declared draft does not define, with values that
# would change verdicts if either reading applied them.
_IF_THEN = {"if": {"type": "string"}, "then": False}
_PREFIX_ITEMS = {"prefixItems": [{"type": "null"}]}
_DEPENDENT_REQUIRED = {"dependentRequired": {"a": ["b"]}}
FOREIGN: dict[Draft, tuple[dict[str, Any], ...]] = {
    D4: (
        {"const": "a"},
        {"contains": {"type": "null"}},
        {"propertyNames": {"maxLength": 0}},
        _IF_THEN,
        _PREFIX_ITEMS,
        _DEPENDENT_REQUIRED,
    ),
    D6: (_IF_THEN, _PREFIX_ITEMS, _DEPENDENT_REQUIRED),
    D7: (_PREFIX_ITEMS, _DEPENDENT_REQUIRED),
    D2019: (_PREFIX_ITEMS,),
}

SCHEMA_TYPES = ("null", "boolean", "integer", "number", "string", "array", "object")

# Each format with a valid and an invalid example (checked against
# jsonschema-rs 0.58). Formats from "uri-reference" down are asserted by
# jsonschema-rs only from a later draft; "not-a-format" by none.
FORMATS: dict[str, tuple[str, ...]] = {
    "date": ("2024-02-29", "2023-02-29"),
    "date-time": ("2024-01-31T12:00:00Z", "2024-01-31T25:00:00Z"),
    "email": ("a@example.com", "a.example.com"),
    "hostname": ("example.com", "-a-.com"),
    "ipv4": ("192.168.0.1", "256.1.1.1"),
    "uri": ("https://example.com/a?b", "//no-scheme"),
    "regex": ("^a+$", "(a"),
    "uri-reference": ("../a#b", "http://[x"),
    "uri-template": ("https://e.com/{id}", "https://e.com/{id"),
    "json-pointer": ("/a~1b", "a/b"),
    "relative-json-pointer": ("1/a", "/a"),
    "idn-hostname": ("例え.jp", "-a-"),
    "iri": ("https://例え.jp/π", "π"),
    "iri-reference": ("π/a", "http://[x"),
    "duration": ("P1DT2H", "P1H"),
    "uuid": ("123e4567-e89b-12d3-a456-426614174000", "123e4567"),
    "not-a-format": ("x",),
}
LATER_FORMATS = (
    "uri-reference", "uri-template", "json-pointer", "relative-json-pointer", "idn-hostname", "iri", "iri-reference",
    "duration", "uuid",
)  # fmt: skip

# Each pattern with strings that match it and strings that do not.
PATTERNS: dict[str, tuple[str, ...]] = {
    "^[a-z]+$": ("abc", "aBc", ""),
    "[0-9]": ("a1", "ab"),
    "^a": ("ab", "ba"),
    "é|😀": ("x😀", "e"),
    "^.{2}$": ("😀😀", "abc"),
}

PROPERTY_NAMES = ("a", "b", "c d", "é", "")
# Definition names need JSON Pointer escapes ("~", "/") and percent-encoding
# (" ", "%", non-ASCII) in a reference; "" gives the pointer "/definitions/".
DEFINITION_NAMES = ("A", "b c", "~/", "%", "é", "")
ROOT_ID = "https://example.com/schemas/root.json"

# Characters a reference keeps unescaped: the RFC 3986 fragment characters
# beyond the unreserved ones.
FRAGMENT_SAFE = "/!$&'()*+,;=:@"

# Keywords whose subschemas apply to parts of the instance, so that a "$ref"
# below one of them descends into the instance.
CONSUMING = frozenset(
    {"properties", "patternProperties", "additionalProperties", "items", "additionalItems", "contains", "propertyNames"}
)

# enum/const members: distinct under JSON Schema equality (no 1 next to 1.0).
_MEMBERS = (None, True, False, 0, 1, -1, 2.5, "a", "b", "", "é", [1], {"a": 1}, 2**64)
_MEMBERS_LATER = (*_MEMBERS, 2.0)  # an integral float: the integer 2 from draft-06 on

_NOISE = json_values(max_leaves=4)
_STRING_ALPHABET = "aZ0é😀 "


def _is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _member_key(value: Any) -> tuple[str, str]:
    return (type(value).__name__, repr(value))


def _without_integral_floats(value: Any) -> Any:
    """``value`` with every integral float written as the int it equals (for draft-04)."""
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, list):
        return [_without_integral_floats(item) for item in value]
    if isinstance(value, dict):
        return {key: _without_integral_floats(item) for key, item in value.items()}
    return value


def _reference(tokens: tuple[str, ...]) -> str:
    return "#" + quote(join_pointer("", *tokens), safe=FRAGMENT_SAFE)


def _positions(value: Any, tokens: tuple[str, ...], draft: Draft) -> list[tuple[str, ...]]:
    """The schema positions at and below ``value`` (at ``tokens``) that the declared draft reads.

    A position whose object holds ``$ref`` in drafts 04 to 07 has no readable
    positions below it, and keywords the draft does not define, or
    ``additionalItems`` without array-form ``items``, hold none either.
    Draft-04 has no boolean schemas as reference targets.
    """
    found = [tokens] if isinstance(value, dict) or draft is not D4 else []
    if not isinstance(value, dict) or ("$ref" in value and draft in REF_HIDES_SIBLINGS):
        return found
    subschemas = SUBSCHEMA_KEYWORDS[draft]
    for keyword, sub in value.items():
        if keyword not in subschemas:
            continue
        if keyword in ("properties", "patternProperties"):
            children = [((keyword, name), child) for name, child in sub.items()]
        elif keyword in ("allOf", "anyOf", "oneOf") or (keyword == "items" and isinstance(sub, list)):
            children = [((keyword, str(index)), child) for index, child in enumerate(sub)]
        elif keyword == "additionalItems" and not isinstance(value.get("items"), list):
            continue
        else:
            children = [((keyword,), sub)]
        for extra, child in children:
            found.extend(_positions(child, (*tokens, *extra), draft))
    return found


# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------


class _Schemas:
    """Draws one older-draft schema document (see the module docstring)."""

    def __init__(self, draw: st.DrawFn, draft: Draft) -> None:
        self.draw = draw
        self.draft = draft
        self.targets: list[tuple[str, ...]] = []  # source tokens of the positions a $ref may target

    def document(self) -> dict[str, Any]:
        draw, draft = self.draw, self.draft
        later = draw(st.integers(0, 1 if draft is D2019 else 5), label="container") == 0
        container = "$defs" if later else "definitions"
        names = draw(st.lists(st.sampled_from(DEFINITION_NAMES), unique=True, max_size=3), label="definitions")
        definitions: dict[str, Any] = {}
        for name in names:
            # An entry refers only to earlier entries, so entries hold no cycle.
            entry = self.node(depth=2, consuming=False, root_refs=False)
            definitions[name] = entry
            self.targets.extend(_positions(entry, (container, name), draft))
        root = self.node(depth=3, consuming=False, root_refs=True, boolean=False)
        extras: list[tuple[str, Any]] = [("$schema", draft.uri)]
        if names or draw(st.integers(0, 7), label="empty container") == 0:
            extras.append((container, definitions))
        if draw(st.integers(0, 3), label="root id") == 0:
            extras.append(("id" if draft is D4 else "$id", ROOT_ID))
        items = list(root.items())
        for extra in extras:
            items.insert(draw(st.integers(0, len(items)), label="extra position"), extra)
        return dict(items)

    def node(self, depth: int, consuming: bool, root_refs: bool, boolean: bool = True) -> dict[str, Any] | bool:
        draw, draft = self.draw, self.draft
        if boolean and draft is not D4 and draw(st.integers(0, 9), label="boolean schema") == 0:
            return draw(st.booleans())
        keywords = list(VALUE_KEYWORDS[draft]) + (list(SUBSCHEMA_KEYWORDS[draft]) if depth > 0 else [])
        # Mostly at least one keyword: empty schemas accept everything (and "not": {} nothing).
        least = 0 if draw(st.integers(0, 7), label="empty schema") == 0 else 1
        chosen = draw(st.lists(st.sampled_from(keywords), unique=True, min_size=least, max_size=4), label="keywords")
        if "format" not in chosen and draw(st.integers(0, 4), label="add format") == 0:
            chosen.append("format")  # formats are where the drafts' assertions differ most
        out: dict[str, Any] = {}
        for keyword in chosen:
            self._keyword(out, keyword, depth, consuming, root_refs)
        if draw(st.integers(0, 7), label="foreign keyword") == 0:
            out.update(draw(st.sampled_from(FOREIGN[draft])))
        ref = self._ref(consuming, root_refs)
        if ref is not None:
            items = list(out.items())
            items.insert(draw(st.integers(0, len(items)), label="ref position"), ("$ref", ref))
            out = dict(items)
        return out

    def _ref(self, consuming: bool, root_refs: bool) -> str | None:
        options = list(self.targets)
        if root_refs and consuming:
            options.append(())
        if not options or self.draw(st.integers(0, 3), label="ref") != 0:
            return None
        return _reference(self.draw(st.sampled_from(options), label="ref target"))

    def _child(self, depth: int, consuming: bool, root_refs: bool) -> dict[str, Any] | bool:
        return self.node(depth - 1, consuming, root_refs)

    def _number(self) -> int | float:
        later = (2.0, -0.0) if self.draft is not D4 else ()
        return self.draw(st.one_of(st.integers(-3, 5), st.sampled_from((0.5, -1.5, 2**64, *later))))

    def _count(self) -> int | float:
        later = (2.0,) if self.draft is not D4 else ()
        return self.draw(st.one_of(st.integers(0, 4), st.sampled_from((0, *later))))

    def _names(self, min_size: int) -> list[str]:
        return self.draw(st.lists(st.sampled_from(PROPERTY_NAMES), unique=True, min_size=min_size, max_size=3))

    def _keyword(self, out: dict[str, Any], keyword: str, depth: int, consuming: bool, root_refs: bool) -> None:
        draw, draft = self.draw, self.draft
        nonempty = 1 if draft is D4 else 0  # draft-04 string arrays have minItems 1
        if keyword == "type":
            out[keyword] = draw(
                st.one_of(
                    st.sampled_from(SCHEMA_TYPES),
                    st.lists(st.sampled_from(SCHEMA_TYPES), unique=True, min_size=1, max_size=3),
                )
            )
        elif keyword in ("enum", "const"):
            members = _MEMBERS if draft is D4 else _MEMBERS_LATER
            if keyword == "const":
                out[keyword] = draw(st.sampled_from(members))
            else:
                out[keyword] = draw(
                    st.lists(st.sampled_from(members), unique_by=_member_key, min_size=1, max_size=4)
                )
        elif keyword == "format":
            # Half the time a format jsonschema-rs asserts only from a later draft.
            names = LATER_FORMATS if draw(st.booleans()) else tuple(FORMATS)
            out[keyword] = draw(st.sampled_from(names))
        elif keyword == "pattern":
            out[keyword] = draw(st.sampled_from(tuple(PATTERNS)))
        elif keyword in ("minimum", "maximum"):
            out[keyword] = self._number()
        elif keyword in ("exclusiveMinimum", "exclusiveMaximum"):
            if draft is D4:
                bound = "minimum" if keyword == "exclusiveMinimum" else "maximum"
                out.setdefault(bound, self._number())  # the draft-04 meta-schema requires the bound
                out[keyword] = draw(st.booleans())
            else:
                out[keyword] = self._number()
        elif keyword in ("minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties"):
            out[keyword] = self._count()
        elif keyword == "uniqueItems":
            out[keyword] = draw(st.booleans())
        elif keyword == "required":
            out[keyword] = self._names(nonempty)
        elif keyword in ("dependencies", "dependentRequired"):
            if draft is D2019 and ({"dependencies", "dependentRequired"} & out.keys()):
                return  # 2019-09 cannot merge dependencies into a dependentRequired next to it
            names = draw(st.lists(st.sampled_from(PROPERTY_NAMES), unique=True, max_size=2))
            out[keyword] = {name: self._names(nonempty) for name in names}
        elif keyword in ("properties", "patternProperties"):
            names = PROPERTY_NAMES if keyword == "properties" else tuple(PATTERNS)
            chosen = draw(st.lists(st.sampled_from(names), unique=True, max_size=3))
            out[keyword] = {name: self._child(depth, True, root_refs) for name in chosen}
        elif keyword in ("additionalProperties", "additionalItems"):
            if draw(st.booleans()):
                out[keyword] = draw(st.booleans())
            else:
                out[keyword] = self._child(depth, True, root_refs)
        elif keyword == "items":
            if draw(st.booleans()):
                count = draw(st.integers(1, 3))  # schemaArray: minItems 1
                out[keyword] = [self._child(depth, True, root_refs) for _ in range(count)]
            else:
                out[keyword] = self._child(depth, True, root_refs)
        elif keyword in ("contains", "propertyNames"):
            out[keyword] = self._child(depth, True, root_refs)
        elif keyword in ("allOf", "anyOf", "oneOf"):
            count = draw(st.integers(1, 3))
            out[keyword] = [self._child(depth, consuming, root_refs) for _ in range(count)]
        else:  # not, if, then, else
            out[keyword] = self._child(depth, consuming, root_refs)


# ---------------------------------------------------------------------------
# Instances
# ---------------------------------------------------------------------------


class _Instances:
    """Draws instances guided by the original schema, as its declared draft reads it."""

    def __init__(self, draw: st.DrawFn, draft: Draft, doc: dict[str, Any]) -> None:
        self.draw = draw
        self.draft = draft
        self.doc = doc
        self.defined = DEFINED[draft]

    def instance(self) -> Any:
        value = self.value(self.doc, depth=5)
        return _without_integral_floats(value) if self.draft is D4 else value

    def _resolve(self, ref: str) -> Any:
        node: Any = self.doc
        for token in split_pointer(unquote(ref[1:])):
            node = node[int(token)] if isinstance(node, list) else node[token]
        return node

    def _has(self, schema: dict[str, Any], keyword: str) -> bool:
        return keyword in schema and keyword in self.defined

    def value(self, schema: Any, depth: int) -> Any:
        draw = self.draw
        if depth <= 0 or not isinstance(schema, dict) or draw(st.integers(0, 6), label="noise") == 0:
            return draw(_NOISE)
        if "$ref" in schema and (self.draft in REF_HIDES_SIBLINGS or draw(st.integers(0, 2), label="follow") != 0):
            return self.value(self._resolve(schema["$ref"]), depth - 1)
        guides = [keyword for keyword in ("enum", "const", "allOf", "anyOf", "oneOf") if self._has(schema, keyword)]
        if guides and draw(st.booleans()):
            keyword = draw(st.sampled_from(guides))
            if keyword == "enum":
                return draw(st.sampled_from(schema["enum"]))
            if keyword == "const":
                return schema["const"]
            return self.value(draw(st.sampled_from(schema[keyword])), depth - 1)
        kind = self._kind(schema)
        if kind == "null":
            return None
        if kind == "boolean":
            return draw(st.booleans())
        if kind in ("integer", "number"):
            return self._number(schema, kind == "integer")
        if kind == "string":
            return self._string(schema)
        if kind == "array":
            return self._array(schema, depth)
        return self._object(schema, depth)

    def _kind(self, schema: dict[str, Any]) -> str:
        draw = self.draw
        declared = schema.get("type")
        if declared is not None and draw(st.integers(0, 7), label="other type") != 0:
            return draw(st.sampled_from([declared] if isinstance(declared, str) else declared))
        hinted = [
            kind
            for kind, keywords in (
                (
                    "object",
                    (
                        "properties", "patternProperties", "additionalProperties", "propertyNames", "required",
                        "dependencies", "dependentRequired", "minProperties", "maxProperties",
                    ),
                ),
                ("array", ("items", "additionalItems", "minItems", "maxItems", "contains", "uniqueItems")),
                ("string", ("format", "pattern", "minLength", "maxLength")),
                ("number", ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum")),
            )
            if any(self._has(schema, keyword) for keyword in keywords)
        ]
        return draw(st.sampled_from(hinted or SCHEMA_TYPES))

    def _number(self, schema: dict[str, Any], integer: bool) -> int | float:
        draw = self.draw
        candidates: list[int | float] = [0, 1, -1]
        for keyword in ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"):
            bound = schema.get(keyword)
            if _is_number(bound):
                candidates += [bound, bound - 1, bound + 1]
                if not integer:
                    candidates += [bound - 0.5, bound + 0.5]
        value = draw(st.sampled_from(candidates))
        if integer and isinstance(value, float):
            value = math.floor(value)
        if self.draft is not D4 and isinstance(value, int) and abs(value) < 2**53 and draw(st.booleans()):
            value = float(value)  # an integral float: an integer from draft-06 on
        return value

    def _string(self, schema: dict[str, Any]) -> str:
        draw = self.draw
        for keyword, examples in (("format", FORMATS), ("pattern", PATTERNS)):
            if self._has(schema, keyword) and schema[keyword] in examples and draw(st.integers(0, 2)) != 0:
                return draw(st.sampled_from(examples[schema[keyword]]))
        lengths = [0, 1, 2, 3]
        for keyword in ("minLength", "maxLength"):
            if keyword in schema:
                lengths += [int(schema[keyword]) + delta for delta in (-1, 0, 1)]
        size = max(0, draw(st.sampled_from(lengths)))
        return draw(st.text(_STRING_ALPHABET, min_size=size, max_size=size))

    def _array(self, schema: dict[str, Any], depth: int) -> list[Any]:
        draw = self.draw
        items = schema.get("items")
        prefix = items if isinstance(items, list) else []
        rest = schema.get("additionalItems") if prefix else items
        lengths = [0, 1, 2, len(prefix), len(prefix) + 1]
        for keyword in ("minItems", "maxItems"):
            if keyword in schema:
                lengths += [int(schema[keyword]) + delta for delta in (-1, 0, 1)]
        size = min(5, max(0, draw(st.sampled_from(lengths))))
        out = []
        for index in range(size):
            sub = prefix[index] if index < len(prefix) else rest
            out.append(self.value(sub, depth - 1) if isinstance(sub, dict) else draw(_NOISE))
        if out and self._has(schema, "contains") and draw(st.booleans()):
            out[draw(st.integers(0, len(out) - 1))] = self.value(schema["contains"], depth - 1)
        if out and schema.get("uniqueItems") is True and draw(st.booleans()):
            out.append(out[0])
        return out

    def _object(self, schema: dict[str, Any], depth: int) -> dict[str, Any]:
        draw = self.draw
        properties = schema.get("properties", {})
        patterns = schema.get("patternProperties", {})
        keys: dict[str, Any] = {}  # key -> the subschema guiding its value (None for none)
        for name in schema.get("required", []):
            if draw(st.integers(0, 7), label="drop required") != 0:
                keys[name] = properties.get(name)
        for name, sub in properties.items():
            if draw(st.booleans()):
                keys[name] = sub
        if patterns and draw(st.booleans()):
            pattern = draw(st.sampled_from(sorted(patterns)))
            keys.setdefault(draw(st.sampled_from(PATTERNS[pattern])), patterns[pattern])
        if draw(st.integers(0, 3), label="extra key") == 0:
            keys.setdefault(draw(st.sampled_from(PROPERTY_NAMES)), schema.get("additionalProperties"))
        for keyword in ("dependencies", "dependentRequired"):
            if self._has(schema, keyword):
                for name, needed in schema[keyword].items():
                    if name in keys and draw(st.booleans()):
                        for other in needed:
                            keys.setdefault(other, properties.get(other))
        return {
            key: self.value(sub, depth - 1) if isinstance(sub, dict) else draw(_NOISE) for key, sub in keys.items()
        }


@st.composite
def older_draft_cases(draw: st.DrawFn) -> tuple[Draft, dict[str, Any], list[Any]]:
    """``(draft, schema document, instances)``: a translatable older-draft schema and guided instances."""
    draft = draw(st.sampled_from(DRAFTS), label="draft")
    schema = _Schemas(draw, draft).document()
    guide = _Instances(draw, draft, schema)
    count = draw(st.integers(2, 8), label="instances")
    return draft, schema, [guide.instance() for _ in range(count)]


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


def _describe(draft: Draft, verdicts: list[bool], ignored: list[tuple[str, str]], emitted: Any) -> None:
    """Hypothesis events: the draft, the verdict balance, and the translation cases reached."""
    event(f"draft: {DRAFT_NAMES[draft]}")
    event("verdicts: mixed" if len(set(verdicts)) == 2 else f"verdicts: all {'accept' if verdicts[0] else 'reject'}")
    for keyword in sorted({keyword for _, keyword in ignored}):
        event(f"ignored: {keyword}")
    text = repr(emitted)
    for marker, label in (
        ("'$ref'", "has $ref"),
        ("'prefixItems'", "has prefixItems"),
        ("'dependentRequired'", "has dependentRequired"),
        ("/prefixItems/", "has $ref into prefixItems"),
    ):
        if marker in text:
            event(label)


@given(older_draft_cases())
def test_translated_schemas_keep_their_verdicts(case: tuple[Draft, dict[str, Any], list[Any]]) -> None:
    draft, schema, instances = case
    model = LOADER.load(schema)
    assert model.fully_supported, f"unsupported constructs after loading: {model.unsupported}"
    emitted, _ = EMITTER.layout(model)
    original = VALIDATOR.compile(schema)
    translated = VALIDATOR.compile(emitted)

    verdicts = [VALIDATOR.is_valid(original, instance) for instance in instances]
    _describe(draft, verdicts, model.ignored, emitted)
    for instance, expected in zip(instances, verdicts, strict=True):
        assert VALIDATOR.is_valid(translated, instance) == expected, (
            f"{DRAFT_NAMES[draft]} verdict {expected} differs from the emitted 2020-12 schema's\n"
            f"instance: {instance!r}\nemitted: {emitted!r}"
        )
