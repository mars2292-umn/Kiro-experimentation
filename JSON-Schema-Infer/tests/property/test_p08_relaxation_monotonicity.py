"""Property 8: Relaxation monotonicity (task 15.2).

For any schema, any set of documents, and any sequence of operations from
the relaxation table (including edits inside ``oneOf`` branches and removals
of ``not``), every document that validated before an operation still
validates after it (Requirement 3.5).

Inputs
    Schemas are drawn over a small vocabulary (property names ``a``, ``b``,
    ``c``, and ``d/~é``, which needs JSON Pointer escapes and
    percent-encoding in a ``$ref``; small numbers; short strings, dates,
    emails, and IPv4 addresses), so the instance pool often validates. Each
    test run focuses its root on one group of rows of the relaxation table:

    - ``values``: ``type``, ``enum``, ``const``, ``format``, ``pattern``
      (safe-AST Patterns the profile can hull, and other regexes that become
      ``(?:old)|new``), and the ten bounds, inclusive and exclusive;
    - ``objects``: ``properties``, ``required``, ``dependentRequired``,
      ``propertyNames``, ``additionalProperties: false`` (with and without
      ``patternProperties``), and key-count bounds;
    - ``arrays``: ``prefixItems`` with ``items: false``, ``items``,
      ``contains``, ``uniqueItems``, and item-count bounds;
    - ``combinators``: ``anyOf``, ``oneOf`` (nested, and beside an
      ``anyOf``), ``allOf``, ``not``, and ``if``/``then``/``else``;
    - ``references``: root ``$defs`` entries referenced from positive
      sites, from ``oneOf`` branches, and from inside ``not`` and ``if``.

    Children mix every row, and about one subschema in seven is boolean
    (mostly ``false``). A second pass adds ``$ref`` values to drawn nodes,
    targeting ``$defs`` entries and arbitrary subschemas, preferring ones
    under a keyword whose removal relocates them (``not``, ``anyOf``,
    ``oneOf``, ``propertyNames``, ``contains``, ``items``,
    ``additionalProperties``, ``if``, ``then``, ``else``). References may
    recurse through an instance-consuming keyword (``properties``,
    ``items``, ...), but never form a cycle of in-place applicators, so
    validation terminates.

    Documents are a pool of distinct instances, some valid and some not:
    witnesses built to satisfy the root (const and enum members, a branch's
    witness, required and optional properties, keys matching
    ``patternProperties``, prefixes and extra elements, Pattern strings and
    near-misses, Format examples), which a heuristic sometimes gets wrong,
    and arbitrary instances over the vocabulary.

    Failures are built in two ways. From rejected values, as the soundness
    loop does (task 15.3): the Validator's errors on the rejected documents
    are mapped to ``(node, keyword, item)`` targets following the hooks in
    the ``decide.relax`` docstring and grouped per target; ``anyOf``/``oneOf``
    failures may carry branch-local failures, obtained by validating the
    values against each branch alone. Or arbitrary: any relaxable keyword of
    any node (inside ``not``, ``if``, and ``$defs`` too), with values drawn
    from the pool's sub-values (plus near-misses for a safe-AST ``pattern``)
    that the node may or may not reject, and arbitrary branch-local
    failures. Either way the row (keyword) is drawn first, so rows with many
    targets (``required`` entries, ``false`` nodes) do not crowd out the rest.

Checks
    Each example runs one to four operations. At each step every candidate
    of :func:`relaxations` is applied with :meth:`Relaxation.applied`, and:

    - monotonicity: every pool document valid before is valid after, with
      ``format`` asserted (the project's Validator). For ``re-induce``, the
      new ``pattern`` alone also accepts every string the old one accepts:
      pool strings, and strings drawn from each old safe-AST alternative;
    - ``applied()`` leaves its input model unchanged (JSON, nids, and
      equality), and applying in place to a copy gives the same model;
    - the relaxed model compiles, its nids are distinct, and its local
      ``$ref`` values resolve;
    - Changes account for the ConstraintIds 15.3 moves records along: each
      Change names a node of the input, each ``before_ids`` entry existed
      before (in the input or an earlier Change's ``after_ids``), each
      ``after_ids`` entry exists afterwards unless a later Change replaced
      it or its node was removed, and no Constraint of a surviving node
      appears or disappears without a Change that names it (the owner's
      ``allOf`` of a conversion beside ``anyOf`` included);
    - the guard contract of the docstring: ``guard_conversions`` and
      ``guard_removals`` name keywords the input held and the result does
      not, and in the relaxed model no path from the root to an edited node
      passes through ``not``, ``if``, or a ``oneOf`` branch;
    - :func:`removal` is the last candidate.

    The sequence continues with a drawn candidate, computed on the model it
    is applied to. ``event()`` shows the rows, actions, guards, conversions
    into ``allOf``, relocations to ``$defs``, rewritten references, and
    Pattern widenings exercised.

**Validates: Requirements 3.5**
"""

# Feature: json-schema-inference, Property 8: Relaxation monotonicity

from __future__ import annotations

import copy
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from typing import Any
from urllib.parse import quote, unquote

import pytest
from hypothesis import assume, event, given, note
from hypothesis import strategies as st

from jsinfer.core.jsonvalue import JSON
from jsinfer.core.provenance import REMOVED
from jsinfer.core.schema_model import (
    ITEMIZED_KEYWORDS,
    NON_CONSTRAINT_KEYWORDS,
    ConstraintId,
    SchemaModel,
    SchemaNode,
    join_pointer,
    split_pointer,
)
from jsinfer.decide.relax import FALSE_SCHEMA, RELAXABLE_KEYWORDS, Change, Failure, Relaxation, relaxations, removal
from jsinfer.io.validator import Compiled, Validator
from jsinfer.values.patterns import Alternative, CharClass, Pattern, Quantifier, Token
from tests.strategies import alternative_strings, pattern_strings

# One Validator for the module: compiled schemas and per-string verdicts are cached across examples.
VALIDATOR = Validator()

# ---------------------------------------------------------------------------
# Vocabulary
# ---------------------------------------------------------------------------

NAMES = ("a", "b", "c", "d/~é")
KEYS = (*NAMES, "e")
DEF_NAMES = ("D0", "x y")
STRINGS = ("", "a", "ab", "abc", "B", "0", "12", "-5", "2.5", "a-1", "A1b", "2024-01-02", "x@y.co", "10.0.0.1")
NUMBERS = (-2, -1, 0, 1, 2, 3, 5, 10, 1.0, 2.5, -0.5, 2**64)
SCALARS = st.one_of(st.none(), st.booleans(), st.sampled_from(NUMBERS), st.sampled_from(STRINGS))

instances = st.recursive(
    SCALARS,
    lambda children: st.one_of(
        st.lists(children, max_size=3),
        st.dictionaries(st.sampled_from(KEYS), children, max_size=3),
    ),
    max_leaves=6,
)

TYPE_NAMES = ("null", "boolean", "integer", "number", "string", "array", "object")
TYPES = st.one_of(
    st.sampled_from(TYPE_NAMES), st.lists(st.sampled_from(TYPE_NAMES), min_size=1, max_size=3, unique=True)
)
FORMATS = ("date", "email", "ipv4", "not-a-format")

# Safe-AST Patterns over the classes and a few literals, as the Value_Inferrer prints them.
_ATOMS = (CharClass.DIGIT, CharClass.LOWER, CharClass.UPPER, CharClass.ALPHA, CharClass.ALNUM, "-", ".", "@")
_QUANTIFIERS = (Quantifier(1, 1), Quantifier(1, 2), Quantifier(2, 2), Quantifier(1, None), Quantifier(2, 4))
SAFE_PATTERNS = st.lists(
    st.lists(
        st.builds(Token, st.sampled_from(_ATOMS), st.sampled_from(_QUANTIFIERS)), min_size=1, max_size=3
    ).map(Alternative),
    min_size=1,
    max_size=2,
).map(lambda alternatives: Pattern(tuple(alternatives)).source)
# Regexes the safe AST does not parse; re-induction alternates them with the new strings.
OTHER_PATTERNS = ("^a", "b$", "a|z", "^[0-9]+$", "\\d", "^(a|b)c?$", "^(?:ab)*$", "-")
PATTERNS = st.one_of(SAFE_PATTERNS, st.sampled_from(OTHER_PATTERNS))
PATTERN_PROPERTY_KEYS = ("^c", "é$", "^[ab]")

NUMBER_BOUNDS = ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum")
NUMBER_BOUND_VALUES = st.sampled_from((-2, 0, 1, 2, 3, 5, 10, 2.5, -0.5))
COUNT_VALUES = st.integers(0, 3)

# Characters a $ref fragment may keep unescaped, or only "/" to escape all of them.
REF_SAFE_SETS = ("/!$&'()*+,;=:@", "/")
REF_SAFE = REF_SAFE_SETS[0]

# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------

MAX_DEPTH = 2

VALUE_FEATURES = ("type", "enum", "const", "format", "pattern", "number-bound", "length-bound")
OBJECT_FEATURES = (
    "properties",
    "required",
    "dependentRequired",
    "propertyNames",
    "closed",
    "additionalProperties",
    "patternProperties",
    "key-count",
)
ARRAY_FEATURES = ("tuple", "items", "contains", "uniqueItems", "item-count")
COMBINATOR_FEATURES = ("anyOf", "oneOf", "oneOf+anyOf", "allOf", "not", "if")
REFERENCE_FEATURES = ("ref-positive", "ref-oneOf", "ref-not", "ref-if")
# Features without subschemas, for nodes at the depth limit.
LEAF_FEATURES = (*VALUE_FEATURES, "required", "dependentRequired", "key-count", "uniqueItems", "item-count")
CHILD_FEATURES = (*VALUE_FEATURES, *OBJECT_FEATURES, *ARRAY_FEATURES, *COMBINATOR_FEATURES)

FOCI = ("values", "objects", "arrays", "combinators", "references")
ROOT_FEATURES = {
    "values": (*VALUE_FEATURES, "number-bound", "number-bound", "length-bound"),
    "objects": OBJECT_FEATURES,
    "arrays": ARRAY_FEATURES,
    "combinators": COMBINATOR_FEATURES,
    "references": (*REFERENCE_FEATURES, *REFERENCE_FEATURES, "oneOf", "oneOf+anyOf", "not", "properties"),
}


def _merge(schema: dict[str, Any], part: dict[str, Any]) -> None:
    for keyword, value in part.items():
        if keyword == "properties" and isinstance(schema.get("properties"), dict):
            schema["properties"].update(value)
        else:
            schema[keyword] = value


def _feature(draw: st.DrawFn, feature: str, depth: int, defs: Sequence[str]) -> dict[str, Any]:
    """The keywords one feature adds to a schema at ``depth``."""

    def child() -> JSON:
        return draw(schemas(depth + 1))

    def children(low: int, high: int) -> list[JSON]:
        return [child() for _ in range(draw(st.integers(low, high)))]

    def names(low: int = 1, high: int = 2) -> list[str]:
        return draw(st.lists(st.sampled_from(NAMES), min_size=low, max_size=high, unique=True))

    def ref() -> dict[str, Any]:
        pointer = join_pointer("", "$defs", draw(st.sampled_from(defs)))
        return {"$ref": "#" + quote(pointer, safe=draw(st.sampled_from(REF_SAFE_SETS)))}

    if feature == "type":
        return {"type": draw(TYPES)}
    if feature == "enum":
        return {"enum": draw(st.lists(SCALARS, min_size=1, max_size=3))}
    if feature == "const":
        return {"const": draw(SCALARS)}
    if feature == "format":
        return {"format": draw(st.sampled_from(FORMATS))}
    if feature == "pattern":
        return {"pattern": draw(PATTERNS)}
    if feature == "number-bound":
        keywords = draw(st.lists(st.sampled_from(NUMBER_BOUNDS), min_size=1, max_size=2, unique=True))
        return {keyword: draw(NUMBER_BOUND_VALUES) for keyword in keywords}
    if feature in ("length-bound", "key-count", "item-count"):
        stem = {"length-bound": "Length", "key-count": "Properties", "item-count": "Items"}[feature]
        keywords = draw(st.lists(st.sampled_from([f"min{stem}", f"max{stem}"]), min_size=1, max_size=2, unique=True))
        return {keyword: draw(COUNT_VALUES) for keyword in keywords}
    if feature == "properties":
        return {"properties": {name: child() for name in names()}}
    if feature == "required":
        return {"required": names()}
    if feature == "dependentRequired":
        return {"dependentRequired": {name: names() for name in names()}}
    if feature == "propertyNames":
        return {"propertyNames": child()}
    if feature == "closed":
        part: dict[str, Any] = {"additionalProperties": False}
        if draw(st.booleans()):
            part["properties"] = {name: child() for name in names()}
        return part
    if feature == "additionalProperties":
        return {"additionalProperties": child()}
    if feature == "patternProperties":
        part = {"patternProperties": {draw(st.sampled_from(PATTERN_PROPERTY_KEYS)): child()}}
        if draw(st.booleans()):
            part["additionalProperties"] = False
        return part
    if feature == "tuple":
        part = {"prefixItems": children(1, 2)}
        if draw(st.integers(0, 3)):
            part["items"] = False
        return part
    if feature == "items":
        return {"items": child()}
    if feature == "contains":
        return {"contains": child()}
    if feature == "uniqueItems":
        return {"uniqueItems": True}
    if feature in ("anyOf", "oneOf", "allOf"):
        return {feature: children(1, 3)}
    if feature == "oneOf+anyOf":
        return {"anyOf": children(1, 2), "oneOf": children(1, 3)}
    if feature == "not":
        return {"not": child()}
    if feature == "if":
        part = {"if": child()}
        for keyword in draw(st.lists(st.sampled_from(["then", "else"]), max_size=2, unique=True)):
            part[keyword] = child()
        return part
    # References to root $defs entries (the "references" focus).
    if feature == "ref-positive":
        site = draw(st.sampled_from(["properties", "allOf", "items"]))
        if site == "properties":
            return {"properties": {draw(st.sampled_from(NAMES)): ref()}}
        return {site: [ref()] if site == "allOf" else ref()}
    if feature == "ref-oneOf":
        branches = [ref(), *children(1, 2)]
        return {"oneOf": draw(st.permutations(branches))}
    if feature == "ref-not":
        return {"not": ref()}
    if feature == "ref-if":
        part = {"if": ref(), "then": child()}
        if draw(st.booleans()):
            part["else"] = child()
        return part
    raise AssertionError(f"unknown feature {feature!r}")


@st.composite
def schemas(draw: st.DrawFn, depth: int = 0, focus: str = "mixed") -> JSON:
    """A schema at ``depth``: at the root, features of ``focus``; below it, every feature."""
    if depth > 0 and draw(st.integers(0, 6), label="boolean subschema") == 0:
        return draw(st.sampled_from([False, False, True]), label="boolean")
    defs: list[str] = []
    schema: dict[str, Any] = {}
    if depth == 0 and focus == "references":
        defs = draw(st.lists(st.sampled_from(DEF_NAMES), min_size=1, max_size=2, unique=True), label="$defs")
        schema["$defs"] = {name: draw(schemas(1), label=f"$defs/{name}") for name in defs}
    if depth >= MAX_DEPTH:
        pool = LEAF_FEATURES
    elif depth == 0:
        pool = ROOT_FEATURES[focus]
    else:
        pool = CHILD_FEATURES
    features = draw(st.lists(st.sampled_from(pool), min_size=1, max_size=3 if depth == 0 else 2), label="features")
    for feature in features:
        _merge(schema, _feature(draw, feature, depth, defs))
    return schema


# Subschema keywords whose evaluation descends into a part of the instance: a
# reference cycle through one of them terminates.
CONSUMING = frozenset(
    {"properties", "patternProperties", "additionalProperties", "items", "prefixItems", "contains", "propertyNames"}
)
# Keywords whose removal relocates referenced subschemas to $defs.
RELOCATING = frozenset(
    {"not", "anyOf", "oneOf", "propertyNames", "contains", "items", "additionalProperties", "if", "then", "else"}
)


def local_ref_tokens(node: SchemaNode) -> list[str] | None:
    """The JSON Pointer tokens of a node's local ``$ref``, or None."""
    if node.boolean is not None:
        return None
    ref = node.kw.get("$ref")
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None
    return split_pointer(unquote(ref[1:]))


def ref_target(model: SchemaModel, node: SchemaNode) -> SchemaNode | None:
    tokens = local_ref_tokens(node)
    return None if tokens is None else model.resolve(join_pointer("", *tokens))


def ref_string(tokens: Sequence[str], safe: str = REF_SAFE) -> str:
    return "#" + quote(join_pointer("", *tokens), safe=safe)


def _in_place_edges(model: SchemaModel) -> dict[int, list[int]]:
    """Edges that apply a subschema to the same instance: applicator children and references."""
    edges: dict[int, list[int]] = {}
    for _, node in model.walk():
        out = edges.setdefault(node.nid, [])
        out.extend(child.nid for keyword, _, child in node.children() if keyword not in CONSUMING and keyword != "$defs")
        target = ref_target(model, node)
        if target is not None:
            out.append(target.nid)
    return edges


def _reaches(edges: dict[int, list[int]], start: int, goal: int) -> bool:
    stack, seen = [start], {start}
    while stack:
        nid = stack.pop()
        if nid == goal:
            return True
        for nxt in edges.get(nid, ()):
            if nxt not in seen:
                seen.add(nxt)
                stack.append(nxt)
    return False


@st.composite
def schema_models(draw: st.DrawFn, focus: str) -> SchemaModel:
    """A model of :func:`schemas` with ``$ref`` values added to drawn nodes (see Inputs), or rarely ``false``."""
    if draw(st.integers(0, 24), label="false root") == 0:
        return SchemaModel(SchemaNode(boolean=False))
    model = SchemaModel(SchemaNode.from_json(draw(schemas(0, focus), label="schema")))
    if model.root.boolean is not None:
        return model
    rate = 3 if focus == "references" else 7
    positions = list(model.walk())
    edges = _in_place_edges(model)
    for pointer, node in positions:
        if node.boolean is not None or "$ref" in node.kw or draw(st.integers(0, rate), label=f"ref at {pointer!r}"):
            continue
        # A reference must not close a cycle of in-place edges (validation would not terminate).
        allowed = [(p, n) for p, n in positions if n is not node and not _reaches(edges, n.nid, node.nid)]
        relocatable = [(p, n) for p, n in allowed if RELOCATING.intersection(split_pointer(p))]
        if not allowed:
            continue
        pool = relocatable if relocatable and draw(st.booleans(), label="prefer relocatable target") else allowed
        target_pointer, target = draw(st.sampled_from(pool), label="ref target")
        node.kw["$ref"] = "#" + quote(target_pointer, safe=draw(st.sampled_from(REF_SAFE_SETS)))
        edges[node.nid].append(target.nid)
    return model


# ---------------------------------------------------------------------------
# Documents
# ---------------------------------------------------------------------------

FORMAT_EXAMPLES = {"date": "2024-01-02", "email": "x@y.co", "ipv4": "10.0.0.1"}
OBJECT_KEYWORDS = frozenset(
    {"properties", "required", "dependentRequired", "propertyNames", "additionalProperties", "patternProperties",
     "minProperties", "maxProperties"}
)
ARRAY_KEYWORDS = frozenset({"prefixItems", "items", "contains", "uniqueItems", "minItems", "maxItems"})
STRING_KEYWORDS = frozenset({"format", "pattern", "minLength", "maxLength"})


def _kinds_of(node: SchemaNode) -> list[str]:
    """The instance kinds a witness of ``node`` may take: its ``type`` names, else those its keywords suggest."""
    listed = node.kw.get("type")
    if isinstance(listed, str):
        return [listed]
    if isinstance(listed, list) and listed:
        return list(listed)
    kinds = [
        kind
        for kind, keywords in (("object", OBJECT_KEYWORDS), ("array", ARRAY_KEYWORDS), ("string", STRING_KEYWORDS))
        if keywords.intersection(node.kw)
    ]
    if any(keyword in node.kw for keyword in NUMBER_BOUNDS):
        kinds.append("number")
    return kinds or list(TYPE_NAMES)


@st.composite
def witnesses(draw: st.DrawFn, model: SchemaModel, node: SchemaNode, depth: int = 0) -> JSON:
    """An instance built to satisfy ``node``: a heuristic, so it may be rejected (the pool needs both)."""
    if depth > 3 or node.boolean is not None or draw(st.integers(0, 9), label="random instead") == 0:
        return draw(instances)
    kw = node.kw
    target = ref_target(model, node)
    if target is not None and draw(st.booleans(), label="follow $ref"):
        return draw(witnesses(model, target, depth + 1))
    if "const" in kw:
        return copy.deepcopy(kw["const"])
    if kw.get("enum"):
        return copy.deepcopy(draw(st.sampled_from(kw["enum"]), label="enum member"))
    for keyword in ("oneOf", "anyOf", "allOf"):
        if keyword in kw and draw(st.booleans(), label=f"witness of a {keyword} branch"):
            return draw(witnesses(model, draw(st.sampled_from(kw[keyword])), depth + 1))
    if "then" in kw and draw(st.booleans(), label="witness of then"):
        return draw(witnesses(model, kw["then"], depth + 1))
    kind = draw(st.sampled_from(_kinds_of(node)), label="kind")
    if kind == "object":
        properties = kw.get("properties", {})
        obj: dict[str, JSON] = {}
        names = [*kw.get("required", []), *(n for n in properties if draw(st.booleans(), label=f"include {n!r}"))]
        for name in names:
            if name not in obj:
                child = properties.get(name)
                obj[name] = draw(witnesses(model, child, depth + 1)) if child is not None else draw(SCALARS)
        for regex, child in kw.get("patternProperties", {}).items():
            matching = [key for key in KEYS if key not in obj and VALIDATOR.matches_pattern(regex, key)]
            if matching and draw(st.booleans(), label=f"include a key matching {regex!r}"):
                obj[draw(st.sampled_from(matching))] = draw(witnesses(model, child, depth + 1))
        for antecedent, consequents in kw.get("dependentRequired", {}).items():
            if antecedent in obj:
                for name in consequents:
                    obj.setdefault(name, draw(SCALARS))
        return obj
    if kind == "array":
        prefix = kw.get("prefixItems", [])
        arr = [draw(witnesses(model, child, depth + 1)) for child in prefix[: draw(st.integers(0, len(prefix)))]]
        items = kw.get("items")
        if len(arr) == len(prefix) and not (isinstance(items, SchemaNode) and items.boolean is False):
            for _ in range(draw(st.integers(0, 2), label="extra elements")):
                arr.append(draw(witnesses(model, items, depth + 1)) if items is not None else draw(SCALARS))
        if "contains" in kw and draw(st.booleans(), label="add a contains witness"):
            arr.append(draw(witnesses(model, kw["contains"], depth + 1)))
        return arr
    if kind == "string":
        pattern = kw.get("pattern")
        if isinstance(pattern, str) and draw(st.booleans(), label="pattern string"):
            try:
                return draw(pattern_strings(Pattern.from_source(pattern)))
            except ValueError:
                pass
        if kw.get("format") in FORMAT_EXAMPLES and draw(st.booleans(), label="format example"):
            return FORMAT_EXAMPLES[kw["format"]]
        return draw(st.sampled_from(STRINGS))
    if kind in ("integer", "number"):
        return draw(st.sampled_from(NUMBERS))
    if kind == "boolean":
        return draw(st.booleans())
    return None


@st.composite
def document_pools(draw: st.DrawFn, model: SchemaModel) -> list[JSON]:
    """Distinct documents: witnesses of the root (often valid) and arbitrary instances (often not)."""
    pool = draw(st.lists(witnesses(model, model.root), min_size=2, max_size=8), label="witnesses")
    pool += draw(st.lists(instances, min_size=2, max_size=6), label="instances")
    return list({repr(doc): doc for doc in pool}.values())


# ---------------------------------------------------------------------------
# Failures
# ---------------------------------------------------------------------------


def instance_at(doc: JSON, tokens: Sequence[str]) -> JSON:
    value = doc
    for token in tokens:
        value = value[int(token)] if isinstance(value, list) else value[token]  # type: ignore[index]
    return value


def sub_values(docs: Iterable[JSON]) -> list[JSON]:
    """Every value in the documents (the documents too), in pre-order."""
    found: list[JSON] = []
    stack = list(reversed(list(docs)))
    while stack:
        value = stack.pop()
        found.append(value)
        if isinstance(value, list):
            stack.extend(reversed(value))
        elif isinstance(value, dict):
            stack.extend(reversed(list(value.values())))
    return found


@dataclass
class Group:
    """The values and documents of one target, with the owner's pointer tokens."""

    nid: int
    keyword: str
    item: str | None
    tokens: list[str]
    values: list[JSON] = field(default_factory=list)
    documents: list[int] = field(default_factory=list)
    seen: set[tuple[int, str]] = field(default_factory=set)

    def add(self, value: JSON, document: int, where: str) -> None:
        if (document, where) not in self.seen:
            self.seen.add((document, where))
            self.values.append(value)
            self.documents.append(document)

    def failure(self, branches: tuple[tuple[Failure, ...], ...] = ()) -> Failure:
        return Failure(self.nid, self.keyword, self.item, self.values, self.documents, branches)


def _resolve(model: SchemaModel, tokens: Sequence[str]) -> SchemaNode:
    return model.resolve(join_pointer("", *tokens))


def _targets_of_error(
    model: SchemaModel, tokens: list[str], kind: str, doc: JSON, where: list[str]
) -> list[tuple[SchemaNode, list[str], str, str | None, JSON]]:
    """``(node, owner tokens, keyword, item, value)`` for one Validator error (the hooks in ``decide.relax``)."""
    value = instance_at(doc, where)
    if kind == "falseSchema":
        if tokens and tokens[-1] in ("items", "additionalProperties"):
            owner = _resolve(model, tokens[:-1])
            child = None if owner.boolean is not None else owner.kw.get(tokens[-1])
            if isinstance(child, SchemaNode) and child.boolean is False:
                return [(owner, tokens[:-1], tokens[-1], None, instance_at(doc, where[:-1]))]
        node = _resolve(model, tokens)
        return [(node, tokens, FALSE_SCHEMA, None, value)] if node.boolean is False else []
    if kind in ("propertyNames", "contains"):
        for i in range(len(tokens) - 1, -1, -1):
            if tokens[i] == kind:
                owner = _resolve(model, tokens[:i])
                if owner.boolean is None and kind in owner.kw:
                    return [(owner, tokens[:i], kind, None, value)]
        return []
    if not tokens:
        return []
    keyword = tokens[-1]
    owner = _resolve(model, tokens[:-1])
    if owner.boolean is not None or keyword not in owner.kw or keyword not in RELAXABLE_KEYWORDS:
        return []
    if keyword == "required":
        names = owner.kw["required"]
        missing = [n for n in dict.fromkeys(names) if isinstance(value, dict) and n not in value]
        return [(owner, tokens[:-1], keyword, name, value) for name in missing]
    if keyword == "dependentRequired":
        entries = owner.kw["dependentRequired"]
        if not isinstance(value, dict):
            return []
        antecedents = [k for k, deps in entries.items() if k in value and any(dep not in value for dep in deps)]
        return [(owner, tokens[:-1], keyword, k, value) for k in antecedents]
    return [(owner, tokens[:-1], keyword, None, value)]


# The rebased document holds the model under this $defs entry (see branch_failures).
REBASE = ["$defs", "__model"]


def rejected_groups(
    model: SchemaModel, compiled: Compiled, docs: Sequence[tuple[int, JSON]], strip: Sequence[str] = ()
) -> list[Group]:
    """The Validator's errors on ``docs`` grouped per target, in first-seen order."""
    groups: dict[tuple[int, str, str | None], Group] = {}
    for d, doc in docs:
        for error in VALIDATOR.errors(compiled, doc):
            tokens = split_pointer(error.keyword_location)
            if tokens[: len(strip)] != list(strip):
                continue
            tokens = tokens[len(strip) :]
            where = split_pointer(error.instance_location)
            try:
                found = _targets_of_error(model, tokens, error.keyword, doc, where)
            except (KeyError, ValueError, IndexError, TypeError):
                continue
            for node, owner_tokens, keyword, item, value in found:
                key = (node.nid, keyword, item)
                group = groups.get(key)
                if group is None:
                    group = groups[key] = Group(node.nid, keyword, item, owner_tokens)
                group.add(value, d, error.instance_location)
    return list(groups.values())


def rebased(model: SchemaModel, tokens: Sequence[str]) -> dict[str, Any]:
    """A document that validates instances against the subschema at ``tokens`` alone, references intact."""
    twin = SchemaModel(copy.deepcopy(model.root))
    for _, node in twin.walk():
        ref = local_ref_tokens(node)
        if ref is not None:
            node.kw["$ref"] = ref_string([*REBASE, *ref])
    return {"$defs": {REBASE[1]: twin.root.to_json()}, "$ref": ref_string([*REBASE, *tokens])}


def branch_failures(model: SchemaModel, group: Group) -> tuple[tuple[Failure, ...], ...]:
    """Per branch of an ``anyOf``/``oneOf`` group: the failures of the group's values against the branch alone."""
    owner = _resolve(model, group.tokens)
    out: list[tuple[Failure, ...]] = []
    for index in range(len(owner.kw[group.keyword])):
        compiled = VALIDATOR.compile(rebased(model, [*group.tokens, group.keyword, str(index)]))
        pairs = list(zip(group.documents, group.values))
        out.append(tuple(g.failure() for g in rejected_groups(model, compiled, pairs, REBASE)))
    return tuple(out)


def relaxable_targets(model: SchemaModel, root: SchemaNode | None = None) -> list[tuple[SchemaNode, str, str | None]]:
    """Every ``(node, keyword, item)`` a Failure may name in the subtree of ``root`` (default: the whole model)."""
    found: list[tuple[SchemaNode, str, str | None]] = []
    for _, node in (root or model.root).walk():
        if node.boolean is False:
            found.append((node, FALSE_SCHEMA, None))
        if node.boolean is not None:
            continue
        for keyword in sorted(node.kw):
            if keyword not in RELAXABLE_KEYWORDS or keyword == FALSE_SCHEMA:
                continue
            if keyword == "required":
                found.extend((node, keyword, name) for name in dict.fromkeys(node.kw[keyword]))
            elif keyword == "dependentRequired":
                found.extend((node, keyword, name) for name in node.kw[keyword])
            else:
                found.append((node, keyword, None))
    return found


def by_row(options: Sequence[Any], row: Any) -> st.SearchStrategy[Any]:
    """One of ``options``: a row (keyword) first, uniformly, so rows with many targets do not crowd out the others."""
    rows: dict[str, list[Any]] = {}
    for option in options:
        rows.setdefault(row(option), []).append(option)
    names = sorted(rows)
    return st.sampled_from(names).flatmap(lambda name: st.sampled_from(rows[name]))


@st.composite
def arbitrary_failures(
    draw: st.DrawFn, model: SchemaModel, values: Sequence[JSON], n_docs: int, root: SchemaNode | None = None, local: bool = False
) -> Failure | None:
    """A Failure for any relaxable target of the subtree, with drawn values the node may or may not reject."""
    targets = relaxable_targets(model, root)
    if not targets:
        return None
    node, keyword, item = draw(by_row(targets, lambda target: target[1]), label="local target" if local else "target")
    chosen = draw(st.lists(st.sampled_from(values), max_size=3), label="values") if values else []
    if keyword == "pattern":
        # Near-misses of a safe-AST Pattern often share an alternative's atoms, so re-induction takes their hull.
        try:
            safe = Pattern.from_source(node.kw["pattern"])
        except ValueError:
            pass
        else:
            chosen += draw(st.lists(pattern_strings(safe), max_size=3), label="near-miss strings")
    documents = draw(st.lists(st.integers(0, max(n_docs - 1, 0)), max_size=3), label="documents")
    branches: tuple[tuple[Failure, ...], ...] = ()
    if keyword in ("anyOf", "oneOf") and not local and draw(st.booleans(), label="branch-local failures"):
        per_branch: list[tuple[Failure, ...]] = []
        for branch in node.kw[keyword]:
            inner: list[Failure] = []
            for _ in range(draw(st.integers(0, 2), label="failures in branch")):
                found = draw(arbitrary_failures(model, values, n_docs, branch, local=True))
                if found is not None:
                    inner.append(found)
            per_branch.append(tuple(inner))
        branches = tuple(per_branch)
    return Failure(node.nid, keyword, item, chosen, documents, branches)


@st.composite
def failures(draw: st.DrawFn, model: SchemaModel, compiled: Compiled, docs: Sequence[JSON], valid: Sequence[bool]) -> tuple[Failure, str]:
    rejected = [(d, doc) for d, doc in enumerate(docs) if not valid[d]]
    if rejected and draw(st.integers(0, 4), label="failure from rejected values") < 3:
        groups = rejected_groups(model, compiled, rejected)
        if groups:
            group = draw(by_row(groups, lambda g: g.keyword), label="failing Constraint")
            if group.keyword in ("anyOf", "oneOf") and draw(st.booleans(), label="with branch-local failures"):
                return group.failure(branch_failures(model, group)), "rejected values with branch failures"
            return group.failure(), "rejected values"
    found = draw(arbitrary_failures(model, sub_values(docs), len(docs)))
    assume(found is not None)  # a model of only true subschemas and containers has nothing to relax
    assert found is not None
    return found, "arbitrary"


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def snapshot(model: SchemaModel) -> tuple[Any, ...]:
    """The model's JSON in keyword order, its nids by pointer, and its equality key."""
    return (repr(model.root.to_json()), [(p, n.nid) for p, n in model.walk()], model.equality_key())


def compile_model(model: SchemaModel, what: str) -> Compiled:
    return VALIDATOR.compile(model.root.to_json(), name=what)  # type: ignore[arg-type]


def constraint_ids(model: SchemaModel) -> set[ConstraintId]:
    """Every ConstraintId the model holds, at the design's granularity."""
    found: set[ConstraintId] = set()
    for _, node in model.walk():
        if node.boolean is not None:
            continue
        for keyword, value in node.kw.items():
            if keyword in NON_CONSTRAINT_KEYWORDS:
                continue
            kind = ITEMIZED_KEYWORDS.get(keyword)
            if kind is None:
                found.add(ConstraintId(node.nid, keyword))
            elif kind is int:
                found.update(ConstraintId(node.nid, keyword, i) for i in range(len(value)))
            else:
                found.update(ConstraintId(node.nid, keyword, name) for name in value)
    return found


def evaluation_edges(model: SchemaModel) -> list[tuple[int, str, int]]:
    """``(owner, keyword, child)`` for every subschema keyword but ``$defs``, and ``(owner, "$ref", target)``."""
    edges: list[tuple[int, str, int]] = []
    for _, node in model.walk():
        edges.extend((node.nid, keyword, child.nid) for keyword, _, child in node.children() if keyword != "$defs")
        target = ref_target(model, node)
        if target is not None:
            edges.append((node.nid, "$ref", target.nid))
    return edges


def _closure(seeds: Iterable[int], adjacency: dict[int, list[int]]) -> set[int]:
    stack = list(seeds)
    seen = set(stack)
    while stack:
        for nxt in adjacency.get(stack.pop(), ()):
            if nxt not in seen:
                seen.add(nxt)
                stack.append(nxt)
    return seen


def guard_violations(model: SchemaModel, edited: Iterable[int]) -> list[tuple[int, str, int]]:
    """Edges ``not``, ``if``, or ``oneOf`` on a path from the root to an edited node (the guard contract)."""
    edges = evaluation_edges(model)
    forward: dict[int, list[int]] = {}
    reverse: dict[int, list[int]] = {}
    for owner, keyword, child in edges:
        reverse.setdefault(child, []).append(owner)
        if keyword not in ("not", "if"):
            forward.setdefault(owner, []).append(child)
    positive = _closure([model.root.nid], forward)
    reaching = _closure(edited, reverse)
    return [
        (owner, keyword, child)
        for owner, keyword, child in edges
        if keyword in ("not", "if", "oneOf") and owner in positive and child in reaching
    ]


def check_monotone(relaxation: Relaxation, before: SchemaModel, after: SchemaModel, docs: Sequence[JSON], valid: Sequence[bool]) -> Compiled:
    compiled = compile_model(after, f"relaxed by {relaxation.label}")
    for d, doc in enumerate(docs):
        if valid[d]:
            assert VALIDATOR.is_valid(compiled, doc), (
                f"{relaxation.label} rejects document {d} {doc!r}, which validated before.\n"
                f"before: {before.root.to_json()!r}\nafter:  {after.root.to_json()!r}"
            )
    return compiled


@st.composite
def accepted_strings(draw: st.DrawFn, old: str, pool: Sequence[JSON]) -> list[str]:
    """Strings the Pattern ``old`` accepts: drawn from each safe-AST alternative, plus the pool's matching strings."""
    found = [s for s in pool if isinstance(s, str) and VALIDATOR.matches_pattern(old, s)]
    try:
        alternatives = Pattern.from_source(old).alternatives
    except ValueError:
        return found
    for alternative in alternatives:
        found += draw(st.lists(alternative_strings(alternative), min_size=1, max_size=4), label="accepted strings")
    return found


def check_pattern_widens(relaxation: Relaxation, before: SchemaModel, after: SchemaModel, strings: Sequence[str]) -> None:
    """Every string the old ``pattern`` accepted, the new one accepts (the keyword alone, as re-induction promises)."""
    nid = relaxation.failure.nid
    old = before.index()[nid].kw["pattern"]
    new = after.index()[nid].kw["pattern"]
    lost = [s for s in strings if not VALIDATOR.matches_pattern(new, s)]
    assert not lost, f"{relaxation.label}: {new!r} rejects {lost[:3]!r}, which {old!r} accepts"


def check_references(model: SchemaModel, where: str) -> None:
    for pointer, node in model.walk():
        tokens = local_ref_tokens(node)
        if tokens is None:
            continue
        try:
            model.resolve(join_pointer("", *tokens))
        except KeyError as exc:
            raise AssertionError(f"{where}: $ref {node.kw['$ref']!r} at {pointer!r} does not resolve") from exc


def check_changes(relaxation: Relaxation, before: SchemaModel, after: SchemaModel, changes: Sequence[Change]) -> None:
    """The Changes account for the ConstraintIds the soundness loop moves records along."""
    label = relaxation.label
    before_nodes = before.index()
    after_nodes = after.index()  # raises on repeated nids
    before_ids = constraint_ids(before)
    after_ids = constraint_ids(after)
    named_before: set[ConstraintId] = set()
    named_after: set[ConstraintId] = set()
    produced: set[ConstraintId] = set()
    for position, change in enumerate(changes):
        assert change.nid in before_nodes, f"{label}: Change {position} names nid {change.nid}, not a node of the input"
        unknown = [cid for cid in change.before_ids if cid not in before_ids and cid not in produced]
        assert not unknown, f"{label}: Change {position} ({change.edit}) lists ids that did not exist: {unknown}"
        assert list(change.before_ids) == sorted(set(change.before_ids)), f"{label}: before_ids not sorted"
        assert list(change.after_ids) == sorted(set(change.after_ids)), f"{label}: after_ids not sorted"
        assert change.removed == (change.after is REMOVED)
        assert not (change.removed and change.after_ids), f"{label}: a removal lists after_ids {change.after_ids}"
        later = {cid for other in changes[position + 1 :] for cid in other.before_ids}
        stale = [
            cid
            for cid in change.after_ids
            if cid not in after_ids and cid not in later and cid.nid in after_nodes
        ]
        assert not stale, f"{label}: Change {position} ({change.edit}) lists after_ids that do not exist: {stale}"
        produced.update(change.after_ids)
        named_before.update(change.before_ids)
        named_after.update(change.after_ids)
        change.record(relaxation.failure.documents or (0,))
    lost = sorted(cid for cid in before_ids - after_ids if cid.nid in after_nodes and cid not in named_before)
    assert not lost, f"{label}: Constraints disappeared without a Change: {lost}"
    gained = sorted(cid for cid in after_ids - before_ids if cid.nid in before_nodes and cid not in named_after)
    assert not gained, f"{label}: Constraints appeared without a Change: {gained}"
    flags = [c.guard for c in changes]
    assert flags == sorted(flags), f"{label}: guard Changes do not all follow the candidate's own Changes"

    # The guards name keywords the input held, and the result no longer holds them.
    for nid in relaxation.guard_conversions:
        node = before_nodes[nid]
        assert node.boolean is None and "oneOf" in node.kw, f"{label}: guard converts nid {nid}, which has no oneOf"
        if nid in after_nodes:
            assert "oneOf" not in after_nodes[nid].kw, f"{label}: guard left the oneOf of nid {nid}"
    for nid, keyword in relaxation.guard_removals:
        node = before_nodes[nid]
        assert node.boolean is None and keyword in node.kw, f"{label}: guard removes {keyword!r} absent at nid {nid}"
        if nid in after_nodes:
            assert keyword not in after_nodes[nid].kw, f"{label}: guard left {keyword!r} at nid {nid}"

    # No path from the root to an edited node passes through not, if, or a oneOf branch.
    edited = [c.nid for c in changes if c.nid in after_nodes]
    violations = guard_violations(after, edited)
    assert not violations, f"{label}: edited nodes {edited} are reached through {violations} in {after.root.to_json()!r}"


# ---------------------------------------------------------------------------
# Events
# ---------------------------------------------------------------------------


def note_model(model: SchemaModel) -> None:
    for pointer, node in model.walk():
        tokens = local_ref_tokens(node)
        if tokens is None:
            continue
        path = set(split_pointer(pointer))
        if "not" in path or "if" in path:
            event("model: $ref inside not or if")
        elif "oneOf" in path:
            event("model: $ref in a oneOf branch")
        else:
            event("model: $ref at a positive site")
        if tokens[:1] == ["$defs"]:
            event("model: $ref to a $defs entry")
        if RELOCATING.intersection(tokens):
            event("model: $ref into a relocatable subschema")


def note_candidate(relaxation: Relaxation, before: SchemaModel, after: SchemaModel, changes: Sequence[Change]) -> None:
    event(f"candidate: {relaxation.action}")
    if relaxation.guard_conversions:
        event("guard: oneOf converted to anyOf")
        if len(relaxation.guard_conversions) > 1:
            event("guard: nested oneOf conversions")
    for _, keyword in relaxation.guard_removals:
        event(f"guard: {keyword} removed")
    if any(c.edit == "convert" and isinstance(c.after, dict) and "allOf" in c.after for c in changes):
        event("conversion beside anyOf wrapped in allOf")
    old_defs = _relocated(before)
    new_defs = _relocated(after)
    if len(new_defs) > len(old_defs):
        event("relocated to $defs")
        if any(name != "relaxed" for name in new_defs if name not in old_defs):
            event("relocated with a numbered name")
    old_refs = {node.nid: node.kw["$ref"] for _, node in before.walk() if local_ref_tokens(node) is not None}
    if any(
        local_ref_tokens(node) is not None and node.nid in old_refs and node.kw["$ref"] != old_refs[node.nid]
        for _, node in after.walk()
    ):
        event("references rewritten")
    if relaxation.action == "re-induce" and relaxation.failure.nid in after.index():
        old = before.index()[relaxation.failure.nid].kw["pattern"]
        try:
            old_alternatives = Pattern.from_source(old).alternatives
        except ValueError:
            event("re-induce: alternation with another regex")
        else:
            new_alternatives = Pattern.from_source(after.index()[relaxation.failure.nid].kw["pattern"]).alternatives
            if any(alternative not in new_alternatives for alternative in old_alternatives):
                event("re-induce: hull of an alternative with the same atoms")
            else:
                event("re-induce: safe-AST alternatives appended")
    if any(c.nid not in after.index() for c in changes):
        event("edited node removed by a guard")


def _relocated(model: SchemaModel) -> list[str]:
    defs = None if model.root.boolean is not None else model.root.kw.get("$defs")
    return [name for name in (defs or {}) if name.startswith("relaxed")]


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


# One run per focus, so each group of rows gets the profile's full example budget.
@pytest.mark.parametrize("focus", FOCI)
@given(data=st.data())
def test_relaxations_only_widen(focus: str, data: st.DataObject) -> None:
    model = data.draw(schema_models(focus), label="model")
    docs = data.draw(document_pools(model), label="documents")
    note_model(model)
    compiled = compile_model(model, "drawn model")
    check_references(model, "drawn model")

    for step in range(data.draw(st.integers(1, 4), label="operations")):
        valid = [VALIDATOR.is_valid(compiled, doc) for doc in docs]
        event(f"valid documents: {'none' if not any(valid) else 'all' if all(valid) else 'some'}")
        failure, how = data.draw(failures(model, compiled, docs, valid), label=f"failure {step}")
        event(f"failure from {how}")
        event(f"row: {failure.keyword}")
        note(f"step {step}: {failure!r} on {model.root.to_json()!r}")

        candidates = relaxations(model, failure, validator=VALIDATOR)
        assert candidates and candidates[-1].removes, "removal is always offered, last"
        alone = removal(model, failure)
        assert (alone.label, alone.order, alone.action) == (
            candidates[-1].label,
            candidates[-1].order,
            candidates[-1].action,
        )

        state = snapshot(model)
        results: list[tuple[SchemaModel, tuple[Change, ...], Compiled]] = []
        for relaxation in candidates:
            relaxed, changes = relaxation.applied(model)
            assert snapshot(model) == state, f"{relaxation.label}: applied() changed its input model"
            check_references(relaxed, relaxation.label)
            after = check_monotone(relaxation, model, relaxed, docs, valid)
            check_changes(relaxation, model, relaxed, changes)
            if relaxation.action == "re-induce" and failure.nid in relaxed.index():  # a guard may remove the node
                old = model.index()[failure.nid].kw["pattern"]
                strings = data.draw(accepted_strings(old, sub_values(docs)), label="strings the old pattern accepts")
                check_pattern_widens(relaxation, model, relaxed, strings)
            note_candidate(relaxation, model, relaxed, changes)
            results.append((relaxed, changes, after))

        # Continue with a drawn candidate, applied in place to a copy of the model.
        pick = data.draw(st.integers(0, len(candidates) - 1), label=f"applied candidate {step}")
        relaxed, changes, compiled = results[pick]
        twin = SchemaModel(copy.deepcopy(model.root))
        in_place = candidates[pick].apply(twin)
        assert snapshot(twin) == snapshot(relaxed), f"{candidates[pick].label}: apply() and applied() differ"
        assert [(c.edit, c.nid, c.before_ids, c.after_ids) for c in in_place] == [
            (c.edit, c.nid, c.before_ids, c.after_ids) for c in changes
        ]
        model = twin
