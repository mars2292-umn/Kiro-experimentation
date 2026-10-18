"""Property 54: Inclusion relation mapping (task 19.3).

For any pair of inclusion outcomes, given by a stubbed check runner, the
Subschema_Checker reports equivalent, subset, superset, or incomparable as
Requirement 31.7 defines them, and a check that fails makes the relation
unsupported with the failure's cause. Any schema holding a keyword the checker
does not support yields ``Unsupported(cause="keywords")`` listing that keyword,
and no check runs (31.8). The stub replaces jsonsubschema, so no child process
starts.

The schemas come from strategies local to this module.

Supported schemas
    Draft 2020-12 documents, valid against the meta-schema (the checker's
    second step), built from the checker's supported keywords: ``type``,
    ``enum``, ``const``, the numeric, string, array, and object bounds,
    ``multipleOf``, ``pattern``, ``required``, ``properties``,
    ``patternProperties``, ``additionalProperties``, ``items``,
    ``prefixItems``, ``allOf``/``anyOf``/``oneOf``, ``not``, a root ``$defs``,
    and boolean subschemas. The root may declare ``$schema`` (one of the
    accepted spellings of the 2020-12 URI) and ``$id``. About a quarter of
    the schema objects get a local ``$ref`` to a position inside a ``$defs``
    entry, percent-encoded with JSON Pointer escapes; an entry refers only to
    earlier entries, so no reference is recursive. ``uniqueItems: true``
    appears only where the items are known scalars (a scalar ``type`` on the
    array schema, or scalar ``items`` and ``prefixItems`` entries).

    Decoys that must never be findings are spread over the schema objects:
    annotations (``title``, ``default``, ``examples``, ``$comment``, ...),
    ``$anchor``, ``$dynamicAnchor``, and ``x-`` keywords whose values hold
    unsupported keywords and ``$ref``; property, pattern, ``$defs``, and
    ``required`` names that spell keywords (``format``, ``if``, ``$ref``,
    ``x-a``).

Planted keywords
    One to six unsupported keyword uses are planted, on either side or both,
    at schema objects drawn from every depth of a supported document, each
    with the findings it must produce (JSON Pointer of the schema object,
    keyword, reason):

    - keywords outside both lists (``format``, ``contains``,
      ``propertyNames``, ``if``/``then``/``else``, ``dependentRequired``,
      ``dependentSchemas``, ``unevaluated*``, ``min/maxContains``,
      ``content*``, ``$dynamicRef``, ``$vocabulary``, legacy
      ``definitions``/``dependencies``/``additionalItems``, unknown names);
      some hold unsupported keywords in their own subschemas, which are
      findings too, and some are supported keywords (``not``,
      ``properties``, ``allOf``) whose subschemas hold one;
    - ``$schema`` declaring another dialect at the root, ``$schema`` below
      the root, and ``$id`` below the root;
    - a non-local, unresolvable, or recursive (``#``, or the schema object's
      own pointer) ``$ref``;
    - ``uniqueItems: true`` on an array schema without ``items``.

    A recursive ``$ref`` makes the document's own references recursive when
    their expansion leads to it, so those are expected findings as well; a
    naive reachability fixed point over the references finds them. Nothing
    else is a finding: the reported uses per side must equal the expected ones
    exactly.

**Validates: Requirements 31.7, 31.8**
"""

# Feature: json-schema-inference, Property 54: Inclusion relation mapping

from __future__ import annotations

import copy
from typing import Any, TypeAlias
from urllib.parse import quote, unquote

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.schema_model import join_pointer, split_pointer
from jsinfer.io.emit import SCHEMA_DIALECT
from jsinfer.io.subschema import (
    BELOW_ROOT,
    EMBEDDED_RESOURCE,
    NON_SCALAR_ITEMS,
    NOT_LOCAL,
    NOT_SUPPORTED,
    OTHER_DIALECT,
    RECURSIVE,
    UNRESOLVED,
    CheckFailure,
    KeywordUse,
    Relation,
    SubschemaChecker,
    Unsupported,
    to_draft4,
    unsupported_keywords,
)

Tokens: TypeAlias = tuple[str, ...]
# A planted keyword: (keyword, value, findings as (tokens relative to the schema object, keyword, reason)).
Plant: TypeAlias = tuple[str, Any, tuple[tuple[Tokens, str, str], ...]]

# Requirement 31.7, written out: (inferred ⊆ truth, truth ⊆ inferred) -> relation.
RELATIONS = {
    (True, True): Relation.EQUIVALENT,
    (True, False): Relation.SUBSET,
    (False, True): Relation.SUPERSET,
    (False, False): Relation.INCOMPARABLE,
}
DEFAULT_TIMEOUT_S = 30.0  # the subschema.timeout_s default
FAILURE_KINDS = ("error", "timeout", "unavailable")

# ---------------------------------------------------------------------------
# Supported grammar
# ---------------------------------------------------------------------------

SCHEMA_TYPES = ("null", "boolean", "integer", "number", "string", "array", "object")
SCALAR_TYPES = ("null", "boolean", "integer", "number", "string")
VALUE_KEYWORDS = (
    "type", "enum", "const", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "minLength", "maxLength",
    "minItems", "maxItems", "minProperties", "maxProperties", "multipleOf", "pattern", "required", "uniqueItems",
)  # fmt: skip
SUBSCHEMA_KEYWORDS = (
    "properties", "patternProperties", "additionalProperties", "items", "prefixItems", "allOf", "anyOf", "oneOf", "not",
)  # fmt: skip
_SINGLE = frozenset({"additionalProperties", "items", "not"})
_LIST = frozenset({"allOf", "anyOf", "oneOf", "prefixItems"})
_MAP = frozenset({"$defs", "patternProperties", "properties"})

# Names that spell keywords are not keywords under properties, patternProperties, $defs, and required.
PROPERTY_NAMES = ("a", "b c", "é", "", "format", "if", "$ref", "x-a", "title", "contains")
DEFINITION_NAMES = ("A", "b/c", "~", "%", "é", "", "format", "x-d", "$ref")
PROPERTY_PATTERNS = ("^[a-z]+$", "format", "^x-", "é|b")
PATTERNS = ("^[a-z]+$", "[0-9]", "^a")
SCALARS = (None, True, False, 0, 1, 1.0, -1, 2.5, "a", "", "é")
MEMBERS = (*SCALARS, [1], {"a": 1}, {"format": "email"})
NUMBERS = (-3, -1, 0, 1, 2, 5, 0.5, -1.5, 2.0)
COUNTS = (0, 1, 2, 4, 2.0)
MULTIPLES = (1, 2, 0.5, 2.0)

DIALECT_URIS = (SCHEMA_DIALECT, "http://json-schema.org/draft/2020-12/schema#", "https://json-schema.org/draft/2020-12/schema#")
ROOT_ID = "https://example.com/schemas/root.json"

# Characters a reference keeps unescaped: the RFC 3986 fragment characters beyond the unreserved ones.
FRAGMENT_SAFE = "/!$&'()*+,;=:@"

# Annotations, anchors, and x- keywords: never findings, whatever their values hold.
DECOYS: tuple[tuple[str, Any], ...] = (
    ("title", "format"),
    ("description", "uses if, then, and contains"),
    ("$comment", "unevaluatedProperties"),
    ("default", {"format": "email", "$ref": "#", "if": {"contains": {}}}),
    ("examples", [{"if": True}, "format", {"$ref": "other.json"}]),
    ("deprecated", True),
    ("readOnly", False),
    ("writeOnly", True),
    ("$anchor", "A_1"),
    ("$dynamicAnchor", "meta"),
    ("x-note", {"format": "email", "$ref": "#", "dependentSchemas": {"a": {}}}),
    ("x-provenance", {"/": {"decision": "format", "items": [{"contains": {}}]}}),
    ("x-latent-type", "date"),
    ("x-format", ["contains", {"unevaluatedProperties": False}]),
    ("x-$ref", "#"),
    ("x-schema", {"$schema": "http://json-schema.org/draft-07/schema#", "$id": "inner.json"}),
)


def _reference(tokens: Tokens) -> str:
    return "#" + quote(join_pointer("", *tokens), safe=FRAGMENT_SAFE)


def _target(ref: str) -> Tokens:
    return tuple(split_pointer(unquote(ref[1:])))


def _positions(value: Any, tokens: Tokens = ()) -> list[tuple[Tokens, Any]]:
    """The schema positions at and below ``value`` (at ``tokens``) reached through supported subschema keywords."""
    found = [(tokens, value)]
    if isinstance(value, dict):
        for keyword, sub in value.items():
            if keyword in _SINGLE:
                found.extend(_positions(sub, (*tokens, keyword)))
            elif keyword in _LIST:
                for index, child in enumerate(sub):
                    found.extend(_positions(child, (*tokens, keyword, str(index))))
            elif keyword in _MAP:
                for name, child in sub.items():
                    found.extend(_positions(child, (*tokens, keyword, name)))
    return found


class _Documents:
    """Draws one supported draft 2020-12 document (see the module docstring)."""

    def __init__(self, draw: st.DrawFn) -> None:
        self.draw = draw
        self.targets: list[Tokens] = []  # positions inside $defs entries drawn so far

    def document(self, boolean_root: bool) -> dict[str, Any] | bool:
        draw = self.draw
        if boolean_root and draw(st.integers(0, 9), label="boolean root") == 0:
            return draw(st.booleans())
        names = draw(st.lists(st.sampled_from(DEFINITION_NAMES), unique=True, max_size=3), label="definitions")
        definitions: dict[str, Any] = {}
        for name in names:
            # An entry refers only to earlier entries, so no reference is recursive.
            entry = self.node(depth=2)
            definitions[name] = entry
            self.targets.extend(tokens for tokens, _ in _positions(entry, ("$defs", name)))
        root = self.node(depth=3, boolean=False)
        assert isinstance(root, dict)
        extras: list[tuple[str, Any]] = []
        if names or draw(st.integers(0, 7), label="empty $defs") == 0:
            extras.append(("$defs", definitions))
        if draw(st.integers(0, 2), label="root $schema") == 0:
            extras.append(("$schema", draw(st.sampled_from(DIALECT_URIS))))
        if draw(st.integers(0, 3), label="root $id") == 0:
            extras.append(("$id", ROOT_ID))
        items = list(root.items())
        for extra in extras:
            items.insert(draw(st.integers(0, len(items)), label="extra position"), extra)
        return dict(items)

    def node(self, depth: int, boolean: bool = True) -> dict[str, Any] | bool:
        draw = self.draw
        if boolean and draw(st.integers(0, 9), label="boolean schema") == 0:
            return draw(st.booleans())
        keywords = VALUE_KEYWORDS + (SUBSCHEMA_KEYWORDS if depth > 0 else ())
        chosen = draw(st.lists(st.sampled_from(keywords), unique=True, max_size=4), label="keywords")
        out: dict[str, Any] = {}
        for keyword in chosen:
            self._keyword(out, keyword, depth)
        if out.get("uniqueItems") is True:
            self._scalar_items(out, depth)
        if draw(st.integers(0, 3), label="decoys") == 0:
            for key, value in draw(st.lists(st.sampled_from(DECOYS), unique_by=lambda d: d[0], min_size=1, max_size=2)):
                out[key] = copy.deepcopy(value)
        if self.targets and draw(st.integers(0, 3), label="ref") == 0:
            items = list(out.items())
            ref = _reference(draw(st.sampled_from(self.targets), label="ref target"))
            items.insert(draw(st.integers(0, len(items)), label="ref position"), ("$ref", ref))
            out = dict(items)
        return out

    def _types(self, choices: tuple[str, ...]) -> str | list[str]:
        return self.draw(
            st.one_of(st.sampled_from(choices), st.lists(st.sampled_from(choices), unique=True, min_size=1, max_size=3))
        )

    def _keyword(self, out: dict[str, Any], keyword: str, depth: int) -> None:
        draw = self.draw
        if keyword == "type":
            out[keyword] = self._types(SCHEMA_TYPES)
        elif keyword == "enum":
            out[keyword] = draw(st.lists(st.sampled_from(MEMBERS), min_size=1, max_size=4))
        elif keyword == "const":
            out[keyword] = copy.deepcopy(draw(st.sampled_from(MEMBERS)))
        elif keyword in ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"):
            out[keyword] = draw(st.sampled_from(NUMBERS))
        elif keyword in ("minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties"):
            out[keyword] = draw(st.sampled_from(COUNTS))
        elif keyword == "multipleOf":
            out[keyword] = draw(st.sampled_from(MULTIPLES))
        elif keyword == "pattern":
            out[keyword] = draw(st.sampled_from(PATTERNS))
        elif keyword == "required":
            out[keyword] = draw(st.lists(st.sampled_from(PROPERTY_NAMES), unique=True, max_size=3))
        elif keyword == "uniqueItems":
            out[keyword] = draw(st.booleans())
        elif keyword in ("properties", "patternProperties"):
            names = PROPERTY_NAMES if keyword == "properties" else PROPERTY_PATTERNS
            chosen = draw(st.lists(st.sampled_from(names), unique=True, max_size=2))
            out[keyword] = {name: self.node(depth - 1) for name in chosen}
        elif keyword in ("prefixItems", "allOf", "anyOf", "oneOf"):
            out[keyword] = [self.node(depth - 1) for _ in range(draw(st.integers(1, 2)))]
        else:  # additionalProperties, items, not
            out[keyword] = self.node(depth - 1)

    def _scalar_items(self, out: dict[str, Any], depth: int) -> None:
        """Make the items of an array schema with ``uniqueItems: true`` known scalars."""
        if self.draw(st.booleans(), label="scalar type on the array schema"):
            out["type"] = self._types(SCALAR_TYPES)  # admits no array: uniqueItems has no effect
            return
        out["items"] = self._scalar(depth)
        if "prefixItems" in out:
            out["prefixItems"] = [self._scalar(depth) for _ in out["prefixItems"]]

    def _scalar(self, depth: int) -> dict[str, Any] | bool:
        """A subschema that admits only scalars."""
        draw = self.draw
        kind = draw(st.integers(0, 5), label="scalar kind")
        if kind == 0:
            return False
        if kind == 1:
            return {"enum": draw(st.lists(st.sampled_from(SCALARS), min_size=1, max_size=3))}
        if kind == 2:
            return {"const": draw(st.sampled_from(SCALARS))}
        if kind == 3:
            return {"anyOf": [self._typed(depth) for _ in range(draw(st.integers(1, 2)))]}
        if kind == 4:
            return {"allOf": [self.node(max(depth - 1, 0)), self._typed(depth)]}
        return self._typed(depth)

    def _typed(self, depth: int) -> dict[str, Any]:
        child = self.node(max(depth - 1, 0), boolean=False)
        assert isinstance(child, dict)
        child["type"] = self._types(SCALAR_TYPES)
        return child


@st.composite
def supported_documents(draw: st.DrawFn, boolean_root: bool = True) -> dict[str, Any] | bool:
    """A draft 2020-12 document over the checker's supported keywords, with decoys."""
    return _Documents(draw).document(boolean_root)


# ---------------------------------------------------------------------------
# Planted unsupported keywords
# ---------------------------------------------------------------------------


def _plain(keyword: str, value: Any, *nested: tuple[Tokens, str]) -> Plant:
    """An unsupported keyword, plus the unsupported keywords nested in its subschemas."""
    return (keyword, value, (((), keyword, NOT_SUPPORTED), *((rel, kw, NOT_SUPPORTED) for rel, kw in nested)))


def _nested(keyword: str, value: Any, *nested: tuple[Tokens, str]) -> Plant:
    """A supported keyword whose subschemas hold unsupported keywords."""
    return (keyword, value, tuple((rel, kw, NOT_SUPPORTED) for rel, kw in nested))


PLAIN: tuple[Plant, ...] = (
    _plain("format", "email"),
    _plain("format", "date"),
    _plain("contains", {"type": "integer"}),
    _plain("contains", {"format": "uuid", "x-f": {"format": "x"}}, (("contains",), "format")),
    _plain("propertyNames", {"maxLength": 3}),
    _plain("propertyNames", {"pattern": "^a", "contentMediaType": "text/plain"}, (("propertyNames",), "contentMediaType")),
    _plain("if", {"contains": {"format": "ipv4"}}, (("if",), "contains"), (("if", "contains"), "format")),
    _plain("then", {"minProperties": 1, "title": "t"}),
    _plain("else", False),
    _plain("dependentRequired", {"a": ["b"]}),
    _plain("dependentSchemas", {"a": {"unevaluatedItems": False}}, (("dependentSchemas", "a"), "unevaluatedItems")),
    _plain("unevaluatedProperties", False),
    _plain("unevaluatedItems", {"type": "string"}),
    _plain("minContains", 1),
    _plain("maxContains", 2),
    _plain("contentEncoding", "base64"),
    _plain("contentMediaType", "application/json"),
    _plain("contentSchema", {"type": "object"}),
    _plain("$dynamicRef", "#meta"),
    _plain("$vocabulary", {"https://example.com/vocab": True}),
    _plain("definitions", {"a": {}}),
    _plain("dependencies", {"a": ["b"]}),
    _plain("additionalItems", False),
    _plain("madeUpKeyword", 1),
    _plain("markdownDescription", "d"),
    _nested("not", {"format": "email"}, (("not",), "format")),
    _nested("properties", {"format": {"format": "date"}, "x-b": {"title": "t"}}, (("properties", "format"), "format")),
    _nested("allOf", [{"x-c": {"format": "x"}}, {"dependentRequired": {"a": []}}], (("allOf", "1"), "dependentRequired")),
)
OTHER_DIALECTS = ("http://json-schema.org/draft-07/schema#", "https://json-schema.org/draft/2019-09/schema", "https://example.com/dialect", 7)
BELOW_ROOT_DIALECTS = (SCHEMA_DIALECT, "http://json-schema.org/draft-07/schema#")
INNER_IDS = ("https://example.com/inner.json", "inner.json")
NOT_LOCAL_REFS = ("other.json#/a", "#A", "https://example.com/schemas/root.json#/$defs/A", "#a/b", "#/a~2b")
UNRESOLVED_REFS = ("#/$defs/zz-missing", "#/x-decoy/a", "#/properties", "#/$defs")


KINDS = (NOT_SUPPORTED, OTHER_DIALECT, BELOW_ROOT, EMBEDDED_RESOURCE, NOT_LOCAL, UNRESOLVED, RECURSIVE, NON_SCALAR_ITEMS)


def _may_be_array(declared: Any) -> bool:
    return declared is None or declared == "array" or (isinstance(declared, list) and "array" in declared)


def _options(kind: str, tokens: Tokens, holder: dict[str, Any]) -> list[Plant]:
    """The plants of one kind (a reason) that the schema object at ``tokens`` can take.

    A plant adds a keyword the object does not hold yet; only the root
    ``$schema`` may be replaced (a ground truth declaring another dialect).
    """
    root = not tokens
    if kind == NOT_SUPPORTED:
        return [plant for plant in PLAIN if plant[0] not in holder]
    if kind == OTHER_DIALECT and root and holder.get("$schema", SCHEMA_DIALECT) in DIALECT_URIS:
        return [("$schema", uri, (((), "$schema", OTHER_DIALECT),)) for uri in OTHER_DIALECTS]
    if kind == BELOW_ROOT and not root and "$schema" not in holder:
        return [("$schema", uri, (((), "$schema", BELOW_ROOT),)) for uri in BELOW_ROOT_DIALECTS]
    if kind == EMBEDDED_RESOURCE and not root and "$id" not in holder:
        return [("$id", uri, (((), "$id", EMBEDDED_RESOURCE),)) for uri in INNER_IDS]
    if kind in (NOT_LOCAL, UNRESOLVED, RECURSIVE) and "$ref" not in holder:
        refs = {NOT_LOCAL: NOT_LOCAL_REFS, UNRESOLVED: UNRESOLVED_REFS, RECURSIVE: ("#", _reference(tokens))}[kind]
        return [("$ref", ref, (((), "$ref", kind),)) for ref in refs]
    if kind == NON_SCALAR_ITEMS and "uniqueItems" not in holder and "items" not in holder:
        if _may_be_array(holder.get("type")):
            return [("uniqueItems", True, (((), "uniqueItems", NON_SCALAR_ITEMS),))]
    return []


def _site(draw: st.DrawFn, sites: list[tuple[Tokens, dict[str, Any]]]) -> tuple[Tokens, dict[str, Any]]:
    """A schema object, drawn by pointer length first so that deep objects are as likely as the root."""
    lengths = sorted({len(tokens) for tokens, _ in sites})
    length = draw(st.sampled_from(lengths), label="plant depth")
    return draw(st.sampled_from([site for site in sites if len(site[0]) == length]), label="plant site")


def _recursive_holders(refs: list[tuple[Tokens, Tokens]]) -> set[Tokens]:
    """The holders of the ``(holder, target)`` references whose expansion reaches a cycle.

    Reference ``a`` leads to ``b`` when ``b``'s holder lies inside ``a``'s
    target (the target itself included).
    """
    leads_to = [[j for j, (holder, _) in enumerate(refs) if holder[: len(target)] == target] for _, target in refs]
    reached: list[set[int]] = []
    for i in range(len(refs)):
        seen: set[int] = set()
        stack = list(leads_to[i])
        while stack:
            j = stack.pop()
            if j not in seen:
                seen.add(j)
                stack.extend(leads_to[j])
        reached.append(seen)
    on_cycle = {i for i in range(len(refs)) if i in reached[i]}
    return {refs[i][0] for i in range(len(refs)) if i in on_cycle or reached[i] & on_cycle}


def _plant(draw: st.DrawFn, doc: dict[str, Any] | bool, count: int) -> tuple[dict[str, Any] | bool, list[KeywordUse]]:
    """Plant ``count`` unsupported keyword uses in ``doc``; return it with every finding it must produce, sorted."""
    if count == 0:
        return doc, []
    if isinstance(doc, bool):
        doc = {}
    objects = [(tokens, node) for tokens, node in _positions(doc) if isinstance(node, dict)]
    refs = [(tokens, _target(node["$ref"])) for tokens, node in objects if "$ref" in node]
    base_refs = len(refs)
    expected: list[KeywordUse] = []
    for _ in range(count):
        kind = draw(st.sampled_from(KINDS), label="plant kind")
        sites = [(tokens, holder) for tokens, holder in objects if _options(kind, tokens, holder)]
        if not sites:  # no schema object takes this kind; every object takes some unsupported keyword
            kind = NOT_SUPPORTED
            sites = [(tokens, holder) for tokens, holder in objects if _options(kind, tokens, holder)]
        if kind == RECURSIVE and draw(st.integers(0, 3), label="inside a referenced target") != 0:
            # A recursive reference inside another reference's target makes that one recursive too.
            inside = [site for site in sites if any(site[0][: len(target)] == target for _, target in refs[:base_refs])]
            sites = inside or sites
        tokens, holder = _site(draw, sites)
        keyword, value, findings = draw(st.sampled_from(_options(kind, tokens, holder)), label="plant")
        holder[keyword] = copy.deepcopy(value)
        expected += [KeywordUse(join_pointer("", *tokens, *rel), kw, reason) for rel, kw, reason in findings]
        if kind == RECURSIVE:
            refs.append((tokens, _target(value)))
        event(f"plant reason: {kind}")
        event(f"plant depth: {min(len(tokens), 4)}{'+' if len(tokens) > 4 else ''}")
    recursive = _recursive_holders(refs)
    assert all(holder in recursive for holder, _ in refs[base_refs:])
    derived = [KeywordUse(join_pointer("", *holder), "$ref", RECURSIVE) for holder, _ in refs[:base_refs] if holder in recursive]
    if derived:
        event("recursion spreads to the document's own references")
    return doc, sorted([*expected, *derived])


@st.composite
def planted_cases(draw: st.DrawFn) -> tuple[Any, Any, list[KeywordUse], list[KeywordUse]]:
    """``(inferred, truth, expected inferred findings, expected truth findings)`` with at least one finding."""
    inferred = draw(supported_documents(), label="inferred")
    truth = draw(supported_documents(), label="truth")
    on_inferred_count = draw(st.integers(0, 3), label="plants in inferred")
    on_truth_count = draw(st.integers(0 if on_inferred_count else 1, 3), label="plants in truth")
    inferred, on_inferred = _plant(draw, inferred, on_inferred_count)
    truth, on_truth = _plant(draw, truth, on_truth_count)
    return inferred, truth, on_inferred, on_truth


# ---------------------------------------------------------------------------
# The stubbed runner and outcomes
# ---------------------------------------------------------------------------


class _Script:
    """A stub runner: returns the scripted outcomes in order and records each call."""

    def __init__(self, *outcomes: bool | CheckFailure) -> None:
        self.outcomes = list(outcomes)
        self.calls: list[tuple[Any, Any, float]] = []

    def __call__(self, lhs: Any, rhs: Any, timeout_s: float) -> bool | CheckFailure:
        self.calls.append((lhs, rhs, timeout_s))
        assert self.outcomes, "the checker ran more checks than scripted"
        return self.outcomes.pop(0)


FAILURES = st.builds(CheckFailure, st.sampled_from(FAILURE_KINDS), st.text(min_size=1, max_size=12))
INCLUSIONS = st.sampled_from((True, False))
OUTCOMES = st.one_of(INCLUSIONS, FAILURES)
# (first check, second check): both complete half the time, each relation equally often.
OUTCOME_PAIRS = st.one_of(
    st.sampled_from(tuple(RELATIONS)),
    st.sampled_from(tuple(RELATIONS)),
    st.tuples(FAILURES, OUTCOMES),
    st.tuples(INCLUSIONS, FAILURES),
)
TIMEOUTS = st.one_of(st.none(), st.floats(min_value=0.001, max_value=1e4), st.integers(1, 120))


def _label(outcome: bool | CheckFailure) -> str:
    return outcome.kind if isinstance(outcome, CheckFailure) else str(outcome).lower()


_DECOY_KEYS = frozenset(key for key, _ in DECOYS)


def _describe(*docs: Any) -> None:
    """Hypothesis events: the supported constructs and decoys the documents reach."""
    nodes = [node for doc in docs for _, node in _positions(doc) if isinstance(node, dict)]
    if any(isinstance(doc, bool) for doc in docs):
        event("has a boolean root")
    for label, found in (
        ("has $ref", any("$ref" in node for node in nodes)),
        ("has uniqueItems: true", any(node.get("uniqueItems") is True for node in nodes)),
        ("has decoys", any(_DECOY_KEYS & node.keys() for node in nodes)),
    ):
        if found:
            event(label)


# ---------------------------------------------------------------------------
# The properties
# ---------------------------------------------------------------------------


@given(supported_documents(), supported_documents(), OUTCOME_PAIRS, TIMEOUTS)
def test_check_outcomes_map_to_relations(
    inferred: Any, truth: Any, outcomes: tuple[bool | CheckFailure, bool | CheckFailure], timeout_s: float | None
) -> None:
    """31.7: completed checks give the defined relation; 31.8: a failed check gives its cause."""
    first, second = outcomes
    assert unsupported_keywords(inferred) == [] and unsupported_keywords(truth) == []
    runner = _Script(first, second)
    result = SubschemaChecker(runner).relation(inferred, truth, timeout_s)
    event(f"outcomes: {_label(first)}, {_label(second)}")
    _describe(inferred, truth)

    limit = DEFAULT_TIMEOUT_S if timeout_s is None else float(timeout_s)
    lhs, rhs = to_draft4(inferred), to_draft4(truth)
    forward, backward = (lhs, rhs, limit), (rhs, lhs, limit)  # inferred ⊆ truth first, then truth ⊆ inferred
    if isinstance(first, CheckFailure):
        failure, direction, calls = first, "the inferred schema is a subschema of the truth schema", [forward]
    elif isinstance(second, CheckFailure):
        failure, direction, calls = second, "the truth schema is a subschema of the inferred schema", [forward, backward]
    else:
        failure, direction, calls = None, "", [forward, backward]
    assert runner.calls == calls

    if failure is None:
        assert result is RELATIONS[first, second]
    else:
        assert isinstance(result, Unsupported), result
        assert result.cause == failure.kind
        assert (result.keywords, result.inferred, result.truth) == ((), (), ())
        assert direction in result.message
        assert result.message.endswith(failure.message)


@given(planted_cases(), OUTCOMES, OUTCOMES)
def test_unsupported_keywords_are_listed(
    case: tuple[Any, Any, list[KeywordUse], list[KeywordUse]], first: bool | CheckFailure, second: bool | CheckFailure
) -> None:
    """31.8: any unsupported keyword, on either side and at any depth, is listed, and no check runs."""
    inferred, truth, on_inferred, on_truth = case
    runner = _Script(first, second)
    result = SubschemaChecker(runner).relation(inferred, truth)
    event(f"sides: {'inferred' if on_inferred else ''}{' and ' if on_inferred and on_truth else ''}{'truth' if on_truth else ''}")
    _describe(inferred, truth)

    assert runner.calls == []
    assert isinstance(result, Unsupported), result
    assert result.cause == "keywords"
    assert list(result.inferred) == on_inferred
    assert list(result.truth) == on_truth
    names = sorted({use.keyword for use in [*on_inferred, *on_truth]})  # str order is code-point order
    assert list(result.keywords) == names
    assert all(name in result.message for name in names)
