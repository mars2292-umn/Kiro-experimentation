"""Property 29: MDL_Scorer is well-formed and invariant (task 10.4).

For any candidate schema node and view, the MDL_Scorer's model bits, data
bits, and their sum are finite and non-negative; each rejected Observation
adds a positive, finite exception cost; and the result is bit-identical for
every permutation of the view's rows and every chunking of the store.

Collections come from :func:`tests.strategies.json_documents` with
:data:`tests.strategies.decimal_scalars` leaves (``0.1`` and ``0.3`` are not
binary fractions, so a bound equal to one of them sits off its own decimal
grid). Some Observations are flagged (``core.stats.FlagSet``): documents by
index, and as Flagged_Items the members under one key of the root objects or
the first elements of the root arrays. Item flags are content-defined and
document flags follow their documents, so every reordering flags the same
multiset. Candidates cover the scorer's vocabulary:

- the Naive_Merge node of the view, and a tightened copy (a bound, a
  length, a short ``enum``, a closed record, or a single type at some
  nodes) that rejects some Observations;
- data-shaped candidates built from the values at each position: ``type``
  (sometimes missing a type or with an extra one), numeric bounds (often an
  observed value), string lengths, safe ``pattern`` values that match the
  strings or not, ``format``, ``enum``/``const``, Lists, Tuples with
  ``items: false`` or ``items``, ``contains``, ``uniqueItems``, Records with
  ``required``, ``dependentRequired`` cliques (Field_Groups), closed or open
  extras, ``patternProperties``, Maps with ``propertyNames`` and key counts,
  hybrid Record/Map nodes, ``anyOf``/``oneOf``, ``not``, ``allOf``,
  ``if``/``then``/``else``, and ``$ref`` into root ``$defs`` (alone, or next
  to other keywords);
- a root ``anyOf``/``oneOf`` with one branch per JSON type of the documents,
  scored with or without ``branch_views`` that hold the rows of each type
  (the analyzer's Variants);
- an arbitrary supported Schema_Model (:func:`tests.strategies.schema_models`,
  with local and recursive ``$ref``). It is skipped when the scorer reports
  a ``$ref`` or branch cycle that consumes no instance: such a schema does
  not terminate under validation, so it is not a candidate.

Checks:

- 19.1: the model bits, data bits, exception bits, and total are finite
  floats ≥ 0; the total is the exactly rounded sum of model and data bits;
  the model bits are ``M(node)``; the exception bits are at most the data
  bits; and ``n_scored`` equals an independent count of the non-flagged
  Observations (so it is the same for every candidate), with
  ``n_rejected ≤ n_scored``.
- 19.2: each rejected Observation costs at least the type tag of its
  universal code, so ``n_rejected > 0`` gives positive exception bits.
  Appending a document that the candidate's root rejects (by ``type``,
  ``enum``, ``const``, or ``false``) adds exactly one rejected Observation
  and a finite exception cost of at least that tag, leaves the model bits
  and the data bits outside the exceptions unchanged, and adds the
  document's non-flagged Observations to ``n_scored``.
- 19.6: the Score is ``==`` (bit-identical) on stores built from four drawn
  permutations of the documents with chunk sizes 1, 3, 17, and 2000.

**Validates: Requirements 19.1, 19.2, 19.6**
"""

# Feature: json-schema-inference, Property 29: MDL_Scorer is well-formed and invariant

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Any

from hypothesis import event, given, note
from hypothesis import strategies as st

from jsinfer.core.jsonvalue import JType, canonical_key, jtype_of
from jsinfer.core.observations import build_store
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.mdl.codes import UNI
from jsinfer.mdl.scorer import FORMAT_ALPHABET_SIZES, MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.values.patterns import Alternative, CharClass, Pattern, Quantifier, Token
from tests.strategies import (
    KEYS,
    SCHEMA_TYPE_NAMES,
    decimal_scalars,
    json_documents,
    json_values,
    safe_patterns,
    schema_models,
)

SCORER = MDLScorer()
CHUNK_SIZES = (1, 3, 17, 2000)
DEFAULT_CHUNK_SIZE = 2000
# Every universal code U(v) starts with a type tag over the seven JSON types,
# so each rejected Observation costs at least this many bits.
TYPE_TAG_BITS = UNI(len(JType))
# Relative slack for comparing sums of the same terms grouped differently.
REL_TOL = 1e-9
# Root values for the appended-document check, one per JSON type.
PROBES: tuple[Any, ...] = (None, False, 7, 0.5, "zz", [1, "a"], {"q": None})
FORMATS = (*sorted(FORMAT_ALPHABET_SIZES), "regex", "not-a-format")
NUMERIC_BOUNDS = ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum")
# Bound values beyond the observed numbers: decimals off the binary grid, and integers.
BOUND_VALUES = (0.1, 0.3, -0.5, 2.5, 0, 1, -3, 1e3)


# ---------------------------------------------------------------------------
# Collections, flags, and views
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Flags:
    """Flags every reordering keeps: document indices (in the drawn order) and content-defined items."""

    docs: frozenset[int]
    key: str | None  # the members under this key of the root objects are Flagged_Items
    first_elements: bool  # the first elements of the root arrays are Flagged_Items


@st.composite
def flagged_collections(draw: st.DrawFn) -> tuple[list[Any], Flags]:
    docs = draw(json_documents(max_docs=8, scalars=decimal_scalars), label="docs")
    indices = st.frozensets(st.integers(0, len(docs) - 1), max_size=2) if docs else st.just(frozenset[int]())
    flagged = draw(indices, label="flagged docs")
    key = draw(st.sampled_from(KEYS), label="flagged key") if draw(st.integers(0, 3)) == 0 else None
    first = draw(st.integers(0, 3), label="flag first elements") == 0
    return docs, Flags(flagged, key, first)


def make_view(docs: Sequence[Any], order: Sequence[int], chunk_size: int, flags: Flags) -> View:
    """The root view of ``docs`` taken in ``order``, flattened in chunks of ``chunk_size``, with ``flags``."""
    store = build_store([docs[index] for index in order], chunk_size)
    flagged = [position for position, index in enumerate(order) if index in flags.docs]
    if flagged:
        store.flags.flag_documents(flagged)
    root = View.root(store)
    if flags.key is not None:
        store.flags.flag_view(root.child_key(flags.key))
    if flags.first_elements:
        store.flags.flag_view(root.elements(0))
    return View.root(store)


def observations(value: Any) -> int:
    """The number of Observations in a value's subtree (the value, its members, and its elements)."""
    total, pending = 0, [value]
    while pending:
        item = pending.pop()
        total += 1
        if isinstance(item, list):
            pending.extend(item)
        elif isinstance(item, dict):
            pending.extend(item.values())
    return total


def reference_n_scored(docs: Sequence[Any], flags: Flags) -> int:
    """The non-flagged Observations of ``docs``: flagged documents and flagged items drop their subtrees."""
    total = 0
    for index, doc in enumerate(docs):
        if index in flags.docs:
            continue
        total += observations(doc)
        if flags.key is not None and isinstance(doc, dict) and flags.key in doc:
            total -= observations(doc[flags.key])
        if flags.first_elements and isinstance(doc, list) and doc:
            total -= observations(doc[0])
    return total


# ---------------------------------------------------------------------------
# Candidates
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class Candidate:
    kind: str
    node: SchemaNode
    # For a root anyOf/oneOf scored with branch_views: the JType rank whose rows each branch codes.
    groups: tuple[int, ...] | None = None

    def branch_views(self, view: View) -> Mapping[int, Sequence[View]] | None:
        if self.groups is None:
            return None
        jtypes = view.gather("jtype")
        return {self.node.nid: [view.subset(jtypes == rank) for rank in self.groups]}

    def score(self, view: View) -> Score:
        return SCORER.score(self.node, view, branch_views=self.branch_views(view))


def _type_names(values: Sequence[Any]) -> list[str]:
    """The ``type`` names of ``values``, with ``number`` replacing ``integer`` as Naive_Merge does."""
    kinds = {jtype_of(value) for value in values}
    if JType.NUMBER in kinds:
        kinds.discard(JType.INTEGER)
    return [kind.keyword for kind in sorted(kinds)]


def _atom(ch: str) -> CharClass | str:
    if "a" <= ch <= "z":
        return CharClass.LOWER
    if "A" <= ch <= "Z":
        return CharClass.UPPER
    if "0" <= ch <= "9":
        return CharClass.DIGIT
    return ch


def matching_pattern(strings: Sequence[str]) -> Pattern:
    """A safe Pattern with one alternative per distinct string: its class and literal runs."""
    alternatives = []
    for s in sorted(set(strings)):
        runs: list[list[Any]] = []
        for ch in s:
            atom = _atom(ch)
            if runs and runs[-1][0] == atom:
                runs[-1][1] += 1
            else:
                runs.append([atom, 1])
        alternatives.append(Alternative(tuple(Token(atom, Quantifier.exactly(count)) for atom, count in runs)))
    return Pattern(tuple(alternatives) or (Alternative(()),))


def _rare(draw: st.DrawFn, n: int = 4) -> bool:
    """True one time in ``n``; shrinks to False."""
    return draw(st.integers(0, n - 1)) == n - 1


_MODES = ("typed",) * 8 + ("true", "false", "branches", "branches", "wrap", "wrap", "ref", "ref")


def _shape(draw: st.DrawFn, values: list[Any], defs: dict[str, Any]) -> Any:
    """A candidate subschema for the values observed at one position (definitions go to ``defs``)."""
    if not values:
        return draw(st.sampled_from([True, False, {}, {"type": "integer"}]))
    mode = draw(st.sampled_from(_MODES), label="mode")
    if mode == "true":
        return draw(st.sampled_from([True, {}]))
    if mode == "false":
        return False
    if mode == "branches":
        groups: dict[JType, list[Any]] = {}
        for value in values:
            groups.setdefault(jtype_of(value), []).append(value)
        branches = [_shape(draw, group, defs) for _, group in sorted(groups.items())]
        return {draw(st.sampled_from(["anyOf", "oneOf"])): draw(st.permutations(branches))}
    schema = _typed(draw, values, defs)
    if mode == "wrap":
        other = {"type": draw(st.sampled_from(SCHEMA_TYPE_NAMES))}
        kind = draw(st.sampled_from(["not", "allOf", "if"]))
        if kind == "not":
            schema["not"] = other
        elif kind == "allOf":
            schema = {"allOf": [schema, draw(st.sampled_from([{}, other]))]}
        else:
            schema = {"if": other, "then": schema, "else": _shape(draw, values, defs)}
    elif mode == "ref":
        name = f"d{len(defs)}"
        defs[name] = schema
        schema = {"$ref": f"#/$defs/{name}"}
        if _rare(draw, 3):  # a $ref next to other keywords only rejects
            schema["type"] = _type_names(values)
    return schema


def _typed(draw: st.DrawFn, values: list[Any], defs: dict[str, Any]) -> dict[str, Any]:
    """A keyword schema shaped by ``values``, with drawn constraints that may reject some of them."""
    schema: dict[str, Any] = {}
    names = _type_names(values)
    change = draw(st.sampled_from(["keep", "keep", "keep", "drop", "add", "absent"]), label="type change")
    if change == "drop" and len(names) > 1:
        names.remove(draw(st.sampled_from(names)))
    elif change == "add":
        extra = draw(st.sampled_from(SCHEMA_TYPE_NAMES))
        if extra not in names:
            names.append(extra)
    if change != "absent":
        schema["type"] = names[0] if len(names) == 1 and draw(st.booleans()) else names

    numbers = [value for value in values if jtype_of(value) in (JType.INTEGER, JType.NUMBER)]
    if bool(numbers) and draw(st.booleans()):
        for keyword in draw(st.lists(st.sampled_from(NUMERIC_BOUNDS), unique=True, min_size=1, max_size=2)):
            schema[keyword] = draw(st.sampled_from([*numbers, *BOUND_VALUES]), label=keyword)

    strings = [value for value in values if isinstance(value, str)]
    if strings:
        longest = max(len(s) for s in strings)
        if _rare(draw):
            schema["minLength"] = draw(st.integers(0, longest + 1))
        if _rare(draw):
            schema["maxLength"] = draw(st.integers(0, longest + 1))
        text = draw(st.sampled_from(["none", "none", "match", "safe", "format"]), label="string code")
        if text == "match":
            schema["pattern"] = matching_pattern(strings).source
        elif text == "safe":
            schema["pattern"] = draw(safe_patterns()).source
        elif text == "format":
            schema["format"] = draw(st.sampled_from(FORMATS))

    if _rare(draw, 6):
        if draw(st.booleans()):
            schema["const"] = draw(st.sampled_from(values))
        else:
            members = draw(st.lists(st.sampled_from(values), max_size=4))
            schema["enum"] = members + draw(st.lists(decimal_scalars, max_size=1))

    arrays = [value for value in values if isinstance(value, list)]
    if arrays:
        elements = [element for array in arrays for element in array]
        longest = max(len(array) for array in arrays)
        layout = draw(st.sampled_from(["list", "list", "tuple", "none"]), label="array layout")
        if layout == "list" and elements:
            schema["items"] = _shape(draw, elements, defs)
        elif layout == "tuple":
            k = draw(st.integers(1, longest + 1))
            schema["prefixItems"] = [_shape(draw, [a[i] for a in arrays if len(a) > i], defs) for i in range(k)]
            rest = [element for array in arrays for element in array[k:]]
            schema["items"] = False if draw(st.booleans()) else _shape(draw, rest, defs)
        if _rare(draw):
            schema["minItems"] = draw(st.integers(0, longest + 1))
        if _rare(draw):
            schema["maxItems"] = draw(st.integers(0, longest + 1))
        if _rare(draw):
            schema["uniqueItems"] = draw(st.booleans())
        if _rare(draw):
            schema["contains"] = _shape(draw, elements, defs)

    objects = [value for value in values if isinstance(value, dict)]
    if objects:
        _object_keywords(draw, schema, objects, defs)
    return schema


def _object_keywords(draw: st.DrawFn, schema: dict[str, Any], objects: list[dict[str, Any]], defs: dict[str, Any]) -> None:
    keys = sorted({key for obj in objects for key in obj})
    layout = draw(st.sampled_from(["record", "record", "map", "hybrid", "none"]), label="object layout")
    if layout == "map":
        schema["additionalProperties"] = _shape(draw, [v for obj in objects for v in obj.values()], defs)
        names = draw(st.sampled_from(["none", "length", "pattern", "enum"]), label="propertyNames")
        if names == "length":
            schema["propertyNames"] = {"maxLength": draw(st.integers(0, 3))}
        elif names == "pattern":
            schema["propertyNames"] = {"pattern": matching_pattern(keys).source}
        elif names == "enum":
            schema["propertyNames"] = {"enum": [*keys, "zz"]}
        largest = max(len(obj) for obj in objects)
        if _rare(draw):
            schema["minProperties"] = draw(st.integers(0, largest + 1))
        if _rare(draw):
            schema["maxProperties"] = draw(st.integers(0, largest + 1))
        return
    if layout == "none":
        return
    if layout == "record":
        fields = [key for key in keys if not _rare(draw, 6)]
    else:  # hybrid: the first fields are a Record, the rest a Map part
        fields = keys[: draw(st.integers(0, len(keys)))]
    schema["properties"] = {key: _shape(draw, [obj[key] for obj in objects if key in obj], defs) for key in fields}
    always = [key for key in fields if all(key in obj for obj in objects)]
    required = [key for key in always if not _rare(draw)]
    if _rare(draw, 5):
        required.append(draw(st.sampled_from([*keys, "zz"])))  # may reject objects
    if required:
        schema["required"] = list(dict.fromkeys(required))
    optional = [key for key in fields if key not in required]
    if len(optional) >= 2 and draw(st.booleans()):
        group = draw(st.lists(st.sampled_from(optional), unique=True, min_size=2, max_size=3))
        schema["dependentRequired"] = {key: [other for other in group if other != key] for key in group}
    extras = [key for key in keys if key not in fields]
    extra_values = [obj[key] for obj in objects for key in extras if key in obj]
    mode = draw(st.sampled_from(["open", "closed", "typed", "patterned"]), label="extras")
    if layout == "hybrid" and mode in ("open", "closed"):
        mode = "typed"
    if mode == "closed":
        schema["additionalProperties"] = False
    elif mode == "typed":
        schema["additionalProperties"] = _shape(draw, extra_values, defs)
    elif mode == "patterned":
        pattern = matching_pattern(extras) if bool(extras) and draw(st.booleans()) else draw(safe_patterns())
        matched = [obj[key] for obj in objects for key in extras if key in obj and pattern.matches(key)]
        schema["patternProperties"] = {pattern.source: _shape(draw, matched, defs)}
        if draw(st.booleans()):
            schema["additionalProperties"] = False


def _with_defs(schema: Any, defs: dict[str, Any]) -> Any:
    if not defs:
        return schema
    if isinstance(schema, bool):
        schema = {"allOf": [schema]}
    return {**schema, "$defs": defs}


@st.composite
def shaped_candidates(draw: st.DrawFn, docs: list[Any]) -> Candidate:
    defs: dict[str, Any] = {}
    schema = _shape(draw, docs, defs)
    return Candidate("shaped", SchemaNode.from_json(_with_defs(schema, defs)))


@st.composite
def variant_candidates(draw: st.DrawFn, docs: list[Any]) -> Candidate:
    """A root anyOf/oneOf with one branch per JSON type, maybe with branch_views of the rows of each type."""
    groups: dict[int, list[Any]] = {}
    for doc in docs:
        groups.setdefault(int(jtype_of(doc)), []).append(doc)
    ranks = tuple(draw(st.permutations(sorted(groups)), label="branch order"))
    defs: dict[str, Any] = {}
    branches = [_shape(draw, groups[rank], defs) for rank in ranks] or [True]
    keyword = draw(st.sampled_from(["anyOf", "oneOf"]))
    node = SchemaNode.from_json(_with_defs({keyword: branches}, defs))
    hinted = bool(ranks) and draw(st.booleans(), label="branch views")
    return Candidate(f"variants ({keyword}{', hinted' if hinted else ''})", node, ranks if hinted else None)


@st.composite
def tightened_candidates(draw: st.DrawFn, merged: SchemaNode) -> Candidate:
    return Candidate("tightened naive merge", SchemaNode.from_json(_tighten(draw, merged.to_json())))


def _tighten(draw: st.DrawFn, schema: Any) -> Any:
    """A copy of a Naive_Merge schema with one constraint added at some of its nodes."""
    if not isinstance(schema, dict):
        return schema
    out: dict[str, Any] = {}
    for keyword, value in schema.items():
        if keyword == "properties":
            out[keyword] = {name: _tighten(draw, child) for name, child in value.items()}
        elif keyword == "items":
            out[keyword] = _tighten(draw, value)
        else:
            out[keyword] = value
    if _rare(draw, 3):
        types = out.get("type", [])
        names = [types] if isinstance(types, str) else list(types)
        change = draw(st.sampled_from(["maxLength", "maximum", "maxItems", "closed", "single", "enum"]))
        if change == "maxLength":
            out["maxLength"] = draw(st.integers(0, 2))
        elif change == "maximum":
            out["maximum"] = draw(st.sampled_from(BOUND_VALUES))
        elif change == "maxItems":
            out["maxItems"] = draw(st.integers(0, 2))
        elif change == "closed" and out.get("properties"):
            properties = dict(out["properties"])
            del properties[draw(st.sampled_from(sorted(properties)))]
            out["properties"] = properties
            out["additionalProperties"] = False
        elif change == "single" and len(names) > 1:
            out["type"] = draw(st.sampled_from(names))
        elif change == "enum":
            out["enum"] = draw(st.lists(decimal_scalars, max_size=3))
    return out


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def root_rejects(schema: Any, value: Any) -> bool:
    """Whether the root of ``schema`` surely rejects ``value`` by ``false``, ``type``, ``enum``, or ``const``."""
    if schema is False:
        return True
    if not isinstance(schema, dict):
        return False
    types = schema.get("type")
    if types is not None:
        names = {types} if isinstance(types, str) else set(types)
        name = jtype_of(value).keyword
        if name not in names and not (name == "integer" and "number" in names):
            return True
    if "const" in schema and canonical_key(schema["const"]) != canonical_key(value):
        return True
    return "enum" in schema and canonical_key(value) not in {canonical_key(member) for member in schema["enum"]}


def check_well_formed(candidate: Candidate, score: Score, n_scored: int) -> None:
    """19.1 and 19.2 on one Score."""
    assert isinstance(score, Score)
    for name in ("model_bits", "data_bits", "exception_bits"):
        bits = getattr(score, name)
        assert isinstance(bits, float) and math.isfinite(bits) and bits >= 0.0, f"{name} = {bits!r}"
    total = score.total
    assert math.isfinite(total) and total >= 0.0, f"total = {total!r}"
    assert total == math.fsum((score.model_bits, score.data_bits))
    assert score.model_bits == SCORER.model_bits(candidate.node)
    assert score.exception_bits <= score.data_bits
    assert score.n_scored == n_scored, f"n_scored {score.n_scored} != {n_scored} non-flagged Observations"
    assert 0 <= score.n_rejected <= score.n_scored
    if score.n_rejected:
        floor = score.n_rejected * TYPE_TAG_BITS
        assert score.exception_bits >= floor * (1 - REL_TOL), (
            f"{score.n_rejected} rejected Observations cost {score.exception_bits} < {floor} bits"
        )


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------

_NO_PROBE = object()


@given(collection=flagged_collections(), data=st.data())
def test_mdl_scorer_is_well_formed_and_invariant(collection: tuple[list[Any], Flags], data: st.DataObject) -> None:
    docs, flags = collection
    identity = tuple(range(len(docs)))
    view = make_view(docs, identity, DEFAULT_CHUNK_SIZE, flags)
    merged = naive_merge_node(view)
    candidates = [
        Candidate("naive merge", merged),
        data.draw(tightened_candidates(merged), label="tightened naive merge"),
        data.draw(shaped_candidates(docs), label="shaped"),
        data.draw(shaped_candidates(docs), label="shaped"),
        data.draw(variant_candidates(docs), label="variants"),
        Candidate("schema model", data.draw(schema_models(max_leaves=6), label="schema model").root),
    ]
    expected_scored = reference_n_scored(docs, flags)
    if flags.docs or flags.key is not None or flags.first_elements:
        event("flags")

    # 19.1, 19.2: every Score is well-formed.
    scored: list[tuple[Candidate, Score]] = []
    for candidate in candidates:
        note(f"{candidate.kind}: {candidate.node.to_json()!r}")
        try:
            score = candidate.score(view)
        except ValueError as error:
            if candidate.kind == "schema model" and "cycle" in str(error):
                event("schema model skipped: a cycle that consumes no instance")
                continue
            raise
        note(f"{candidate.kind}: {score}")
        check_well_formed(candidate, score, expected_scored)
        event(f"{candidate.kind.split(' (')[0]}: {'rejects' if score.n_rejected else 'accepts every'} Observation")
        scored.append((candidate, score))

    # 19.6: bit-identical for every document order and chunking.
    orders = data.draw(
        st.lists(st.permutations(identity), min_size=len(CHUNK_SIZES), max_size=len(CHUNK_SIZES)), label="orders"
    )
    for chunk_size, order in zip(CHUNK_SIZES, orders):
        other = make_view(docs, order, chunk_size, flags)
        for candidate, score in scored:
            again = candidate.score(other)
            assert again == score, (
                f"{candidate.kind}: order {order} with chunk size {chunk_size} gives {again}, not {score}"
            )

    # 19.2: a document the root rejects adds one rejected Observation and a positive, finite exception cost.
    drawn = data.draw(json_values(max_leaves=4, scalars=decimal_scalars), label="probe")
    for candidate, score in scored:
        schema = candidate.node.to_json()
        probe = next((value for value in (drawn, *PROBES) if root_rejects(schema, value)), _NO_PROBE)
        if probe is _NO_PROBE:
            continue
        event(f"{candidate.kind.split(' (')[0]}: appended a rejected document")
        extended = [*docs, probe]
        bigger = candidate.score(make_view(extended, tuple(range(len(extended))), DEFAULT_CHUNK_SIZE, flags))
        note(f"{candidate.kind} with {probe!r} appended: {bigger}")
        check_well_formed(candidate, bigger, reference_n_scored(extended, flags))
        assert bigger.n_rejected == score.n_rejected + 1
        added = bigger.exception_bits - score.exception_bits
        assert math.isfinite(added) and added >= TYPE_TAG_BITS * (1 - REL_TOL), f"the exception added {added} bits"
        assert bigger.model_bits == score.model_bits
        # The accepted rows are coded as before: the whole increase is exception cost.
        assert math.isclose(
            bigger.data_bits - bigger.exception_bits,
            score.data_bits - score.exception_bits,
            rel_tol=REL_TOL,
            abs_tol=REL_TOL,
        )
