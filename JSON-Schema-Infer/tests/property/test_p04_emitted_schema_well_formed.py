"""Property 4: Emitted schemas are well-formed and canonical (task 6.5).

For any Schema_Model, the emitted document validates against the draft
2020-12 meta-schema, carries ``$schema`` only at the root, uses an ``x-``
prefix for every keyword outside the 2020-12 vocabularies, and is
byte-identical across every permutation of the order in which its
Constraints were added.

Models come from :func:`tests.strategies.schema_models` (task 6.4): every
supported keyword, local and recursive ``$ref``, boolean schemas at every
position and at the root, annotations, a root ``$id``, and ``x-`` keywords,
with values in non-canonical form. The test checks the emitted bytes:

- ``$schema`` (2.1): the root schema object starts with ``$schema`` set to
  the draft 2020-12 URI, and no subschema holds it. A boolean root is
  emitted as ``true`` or ``false`` with no ``$schema``, as ``io/emit.py``
  documents: it has no schema object to hold one, and wrapping it would
  change the model under 2.7.
- Meta-schema (2.1): the parsed document has no errors against the draft
  2020-12 meta-schema under the Validator (formats asserted).
- ``x-`` prefix (2.2): every keyword of every schema object is in the draft
  2020-12 vocabularies (``VOCABULARY_KEYWORDS``, pinned to an independent
  list in ``tests/unit/test_emit.py``) or starts with ``x-``. Names under
  ``properties``, ``patternProperties``, ``dependentRequired``, and
  ``$defs`` are not keywords, nor are keys inside JSON values such as a
  ``const`` object, so they are not checked.
- Layout (2.3): the bytes are strict UTF-8 and equal the canonical encoder's
  output for their own parsed content (two-space indent, non-ASCII kept,
  one trailing newline).
- Order (2.3): keywords follow ``keyword_rank``; ``properties``,
  ``patternProperties``, ``$defs``, and ``dependentRequired`` names are in
  code-point order; ``type`` lists at least two distinct names in the order
  null, boolean, integer, number, string, array, object, or is a single
  name; ``required`` and each ``dependentRequired`` entry list distinct
  names in code-point order; ``enum`` lists distinct members in canonical
  value order; and every JSON value is its canonical representative (no
  integral floats, object keys in code-point order).
- Insertion-order independence (2.3): a twin of the model that is equal
  under 2.7 is rebuilt with every content-free choice redrawn. Keywords and
  map entries are inserted in a shuffled order; nodes are made bottom-up, so
  the twin's nids follow no tree order; set-compared lists are shuffled and
  get repeats; a lone ``type`` name switches between string and list form;
  integral numbers switch between ``int`` and ``float`` (``0`` and ``-0.0``
  included); object members inside JSON values are shuffled; ``$schema``
  and ``x-provenance``, which 2.7 excludes, are added, changed, or dropped at
  any node; and the loader records (ignored keywords, assumed dialect)
  change. The twin's bytes must equal the model's. The order of
  ``prefixItems``, ``anyOf``, ``oneOf``, ``allOf``, and ``examples`` is
  content (2.7 compares them by index) and is kept.

**Validates: Requirements 2.1, 2.2, 2.3**
"""

# Feature: json-schema-inference, Property 4: Emitted schemas are well-formed and canonical

from __future__ import annotations

import json
import random
from collections.abc import Iterable, Iterator
from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.determinism import encode_canonical, value_sort_key
from jsinfer.core.schema_model import (
    EXCLUDED_KEYWORDS,
    SUBSCHEMA_KEYWORDS,
    SchemaModel,
    SchemaNode,
    join_pointer,
)
from jsinfer.io.emit import VOCABULARY_KEYWORDS, SchemaEmitter, keyword_rank
from jsinfer.io.validator import Draft, Validator
from tests.strategies import SCHEMA_TYPE_NAMES, schema_models

EMITTER = SchemaEmitter()
VALIDATOR = Validator()

# Requirement 2.1, written out rather than taken from the emitter.
DRAFT_2020_12_URI = "https://json-schema.org/draft/2020-12/schema"

# $schema values a twin may hold at any node: 2.7 excludes $schema, and the
# emitter writes its own at the root and drops it everywhere else.
_TWIN_SCHEMA_URIS = (DRAFT_2020_12_URI, "http://json-schema.org/draft-07/schema#", "https://example.com/unknown")

# Loader records a twin may carry; 2.7 excludes them.
_TWIN_IGNORED = (("", "markdownDescription"), ("/properties/a", "deprecationMessage"))


# ---------------------------------------------------------------------------
# The twin: equal under 2.7, built with every content-free choice redrawn
# ---------------------------------------------------------------------------


def _shuffled(items: Iterable[Any], rnd: random.Random) -> list[Any]:
    out = list(items)
    rnd.shuffle(out)
    return out


def _respell(value: Any, rnd: random.Random) -> Any:
    """``value`` with integral numbers respelled (``int`` and ``float``, ``0`` and ``-0.0``) and object members shuffled."""
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, int):
        if rnd.random() < 0.5:
            return value
        if value == 0:
            return rnd.choice((0.0, -0.0))
        try:
            spelled = float(value)
        except OverflowError:
            return value
        return spelled if int(spelled) == value else value
    if isinstance(value, float):
        return int(value) if value.is_integer() and rnd.random() < 0.5 else value
    if isinstance(value, list):  # array order is content
        return [_respell(item, rnd) for item in value]
    return {key: _respell(item, rnd) for key, item in _shuffled(value.items(), rnd)}


def _as_set(values: list[Any], rnd: random.Random) -> list[Any]:
    """A set-compared list respelled, shuffled, and sometimes with a repeated member."""
    out = [_respell(item, rnd) for item in values]
    if values and rnd.random() < 0.5:
        out.append(_respell(rnd.choice(values), rnd))
    rnd.shuffle(out)
    return out


def _excluded_value(keyword: str, rnd: random.Random) -> Any:
    if keyword == "$schema":
        return rnd.choice(_TWIN_SCHEMA_URIS)
    return {"technique": rnd.choice(("a", "b")), "consistent": rnd.randint(0, 9)}  # x-provenance


def _twin_value(keyword: str, value: Any, rnd: random.Random) -> Any:
    shape = SUBSCHEMA_KEYWORDS.get(keyword)
    if shape == "single":
        return _twin_node(value, rnd)
    if shape == "list":  # each element is its own position
        return [_twin_node(child, rnd) for child in value]
    if shape == "map":
        return {name: _twin_node(child, rnd) for name, child in _shuffled(value.items(), rnd)}
    if keyword == "type":
        names = [value] if isinstance(value, str) else list(value)
        if len(set(names)) == 1 and rnd.random() < 0.5:
            return names[0]
        return _as_set(names, rnd)
    if keyword in ("enum", "required"):
        return _as_set(value, rnd)
    if keyword == "dependentRequired":
        return {name: _as_set(entry, rnd) for name, entry in _shuffled(value.items(), rnd)}
    return _respell(value, rnd)


def _twin_node(node: SchemaNode, rnd: random.Random) -> SchemaNode:
    """A node equal to ``node`` under 2.7. Its children are made first, so they get the smaller nids."""
    if node.boolean is not None:
        return SchemaNode(boolean=node.boolean)
    items: list[tuple[str, Any]] = []
    for keyword, value in node.kw.items():
        if keyword in EXCLUDED_KEYWORDS:
            if rnd.random() < 0.5:
                continue
            value = _excluded_value(keyword, rnd)
        items.append((keyword, value))
    for keyword in sorted(EXCLUDED_KEYWORDS):
        if keyword not in node.kw and rnd.random() < 0.25:
            items.append((keyword, _excluded_value(keyword, rnd)))
    kw = {keyword: _twin_value(keyword, value, rnd) for keyword, value in _shuffled(items, rnd)}
    return SchemaNode(kw=kw)


def _twin_model(model: SchemaModel, rnd: random.Random) -> SchemaModel:
    return SchemaModel(
        _twin_node(model.root, rnd),
        dialect_assumed=rnd.random() < 0.5,
        ignored=rnd.sample(_TWIN_IGNORED, rnd.randint(0, len(_TWIN_IGNORED))),
    )


# ---------------------------------------------------------------------------
# Checks on the emitted document
# ---------------------------------------------------------------------------


def _reject_constant(name: str) -> Any:
    raise ValueError(f"the emitted bytes hold the non-JSON constant {name}")


def _schema_positions(doc: Any) -> Iterator[tuple[str, Any]]:
    """``(pointer, schema)`` for every schema position of an emitted document, following the subschema keywords."""
    stack: list[tuple[str, Any]] = [("", doc)]
    while stack:
        pointer, schema = stack.pop()
        yield pointer, schema
        if not isinstance(schema, dict):
            continue
        for keyword, value in schema.items():
            shape = SUBSCHEMA_KEYWORDS.get(keyword)
            if shape == "single":
                stack.append((join_pointer(pointer, keyword), value))
            elif shape == "list":
                stack.extend((join_pointer(pointer, keyword, index), child) for index, child in enumerate(value))
            elif shape == "map":
                stack.extend((join_pointer(pointer, keyword, name), child) for name, child in value.items())


def _non_canonical(value: Any) -> Any:
    """The first integral float or object with unsorted keys inside ``value``, or None when it is canonical."""
    stack = [value]
    while stack:
        item = stack.pop()
        if isinstance(item, float) and item.is_integer():
            return item
        if isinstance(item, dict):
            if list(item) != sorted(item):
                return item
            stack.extend(item.values())
        elif isinstance(item, list):
            stack.extend(item)
    return None


def _check_name_set(names: Any, where: str) -> None:
    assert isinstance(names, list) and all(isinstance(name, str) for name in names), f"{where}: not a list of names"
    assert names == sorted(set(names)), f"{where}: names are not distinct and in code-point order: {names!r}"


def _check_schema_object(pointer: str, schema: dict[str, Any]) -> None:
    where = pointer or "(root)"
    keywords = list(schema)

    # 2.1: $schema on the root schema object only.
    if pointer == "":
        assert schema.get("$schema") == DRAFT_2020_12_URI, f"root $schema is {schema.get('$schema')!r}"
    else:
        assert "$schema" not in schema, f"{where}: a subschema holds $schema"

    # 2.2: every keyword is a 2020-12 vocabulary keyword or has the x- prefix.
    for keyword in keywords:
        assert keyword in VOCABULARY_KEYWORDS or keyword.startswith("x-"), (
            f"{where}: keyword {keyword!r} is outside the 2020-12 vocabularies and lacks the x- prefix"
        )

    # 2.3: keyword rank order, then the content-determined order of each value.
    assert keywords == sorted(keywords, key=keyword_rank), f"{where}: keywords out of rank order: {keywords}"
    for keyword, value in schema.items():
        at = join_pointer(pointer, keyword)
        shape = SUBSCHEMA_KEYWORDS.get(keyword)
        if shape == "map":
            assert list(value) == sorted(value), f"{at}: names not in code-point order: {list(value)}"
        elif shape is not None:
            continue  # subschemas are checked at their own positions
        elif keyword == "type":
            names = [value] if isinstance(value, str) else value
            assert isinstance(value, str) or len(value) >= 2, f"{at}: a single type is not written as a string"
            assert set(names) <= set(SCHEMA_TYPE_NAMES), f"{at}: unknown type names in {names!r}"
            assert names == sorted(set(names), key=SCHEMA_TYPE_NAMES.index), f"{at}: types out of order: {names!r}"
        elif keyword == "required":
            _check_name_set(value, at)
        elif keyword == "dependentRequired":
            assert list(value) == sorted(value), f"{at}: keys not in code-point order: {list(value)}"
            for key, entry in value.items():
                _check_name_set(entry, join_pointer(at, key))
        elif keyword == "enum":
            keys = [value_sort_key(member) for member in value]
            assert len(set(keys)) == len(keys), f"{at}: repeated members: {value!r}"
            assert keys == sorted(keys), f"{at}: members not in canonical value order: {value!r}"
            assert _non_canonical(value) is None, f"{at}: non-canonical member part {_non_canonical(value)!r}"
        else:
            assert _non_canonical(value) is None, f"{at}: non-canonical value part {_non_canonical(value)!r}"


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@given(model=schema_models(), rnd=st.randoms(use_true_random=False))
def test_emitted_schemas_are_well_formed_and_canonical(model: SchemaModel, rnd: random.Random) -> None:
    doc, _ = EMITTER.layout(model)
    data = EMITTER.emit(model)
    assert EMITTER.dumps(doc) == data

    # 2.3: strict UTF-8 in the canonical encoder's fixed layout.
    text = data.decode("utf-8")
    parsed = json.loads(text, parse_constant=_reject_constant)
    assert encode_canonical(parsed) == data, f"the bytes are not in the canonical layout:\n{text}"

    # 2.1: valid against the draft 2020-12 meta-schema.
    meta_errors = VALIDATOR.meta_errors(parsed, Draft.DRAFT202012)
    assert meta_errors == [], f"meta-schema errors {meta_errors}\nemitted:\n{text}"

    positions = list(_schema_positions(parsed))
    # The checks below reach every schema position of the model.
    assert sorted(pointer for pointer, _ in positions) == sorted(pointer for pointer, _ in model.walk())
    if model.root.boolean is not None:
        event("root: boolean")
        assert parsed is model.root.boolean, f"boolean root emitted as {text!r}"
    else:
        event(f"schema objects: {'1' if len(positions) == 1 else '2-9' if len(positions) < 10 else '10+'}")
    for pointer, schema in positions:
        if isinstance(schema, dict):
            _check_schema_object(pointer, schema)
        else:
            assert isinstance(schema, bool), f"{pointer}: a schema is an object or a boolean, got {schema!r}"

    # 2.3: a 2.7-equal twin built in another order emits the same bytes.
    twin = _twin_model(model, rnd)
    assert twin == model, "test bug: the twin differs from the model under 2.7"
    twin_data = EMITTER.emit(twin)
    assert twin_data == data, (
        f"a 2.7-equal model built in another order emits different bytes\nmodel:\n{text}\n"
        f"twin:\n{twin_data.decode('utf-8')}"
    )
