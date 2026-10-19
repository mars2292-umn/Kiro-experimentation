"""Property 34: Label derivation is consistent (task 21.6).

For any supported schema, ``derive_labels`` assigns each (Path, kind) the
label dictated by the keyword rules of 22.4, or unknown per 22.5, and no
label to a kind that does not apply at a Path. The reference below reads the
rules independently over the plain JSON of the schema: it walks the document
with the Inferrer's Path stepping (``properties`` adds a Key step,
``patternProperties`` and a schema-valued ``additionalProperties`` a MapVal
step, ``items`` and ``prefixItems`` an Elem step), keeps ``allOf``,
``anyOf``, ``oneOf``, ``if``, ``then``, and ``else`` branches and the targets
of local ``$ref`` at their owner's Path (references followed at Paths of at
most ``max_depth`` steps, once per step-less chain), and combines the
per-subschema readings: one common label, else unknown.

Evaluating Judgment_Call accuracy of a schema against itself yields 1 for
every kind with at least one known label (31.9), and the labels survive the
Schema_Emitter/Schema_Loader round trip, so a schema evaluated against its
emitted self scores 1 as well.

Inputs
    Fully supported Schema_Models from ``tests.strategies.schema_models``
    (with ``$ref`` to ancestors and ``$defs`` entries), and the planted
    schemas of ``tests.strategies.planted_schemas``.

**Validates: Requirements 22.4, 22.5, 31.9**
"""

# Feature: json-schema-inference, Property 34: Label derivation is consistent

from __future__ import annotations

from collections import defaultdict
from typing import Any
from urllib.parse import unquote

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, Path, key_step
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoader
from jsinfer.learn.labels import KINDS, UNKNOWN, derive_labels, judgment_accuracy
from tests.strategies import planted_schemas, schema_models

MAX_DEPTH = 3
SCALARS = frozenset({"null", "boolean", "integer", "number", "string"})
OPEN_ENUM = frozenset({"integer", "number", "string"})


# -- the reference ---------------------------------------------------------------


def _types(schema: dict[str, Any]) -> frozenset[str]:
    declared = schema.get("type")
    if isinstance(declared, str):
        return frozenset({declared})
    if isinstance(declared, list):
        return frozenset(n for n in declared if isinstance(n, str))
    return frozenset()


def _pointer(ref: Any) -> str | None:
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None
    pointer = unquote(ref[1:])
    if pointer and not pointer.startswith("/"):
        return None
    return pointer


def _resolve(root: Any, pointer: str) -> Any:
    node = root
    for token in pointer.split("/")[1:]:
        token = token.replace("~1", "/").replace("~0", "~")
        if isinstance(node, dict) and token in node:
            node = node[token]
        elif isinstance(node, list) and token.isdigit() and int(token) < len(node):
            node = node[int(token)]
        else:
            return None
    return node if isinstance(node, (dict, bool)) else None


def _subschemas_by_path(root: Any) -> dict[Path, list[Any]]:
    by_path: dict[Path, list[Any]] = defaultdict(list)
    stack: list[tuple[Path, Any, frozenset[str]]] = [((), root, frozenset())]
    while stack:
        path, schema, chain = stack.pop()
        by_path[path].append(schema)
        if not isinstance(schema, dict):
            continue
        pointer = _pointer(schema.get("$ref"))
        if pointer is not None and len(path) <= MAX_DEPTH and pointer not in chain:
            target = _resolve(root, pointer)
            if target is not None:
                stack.append((path, target, chain | {pointer}))
        for keyword in ("allOf", "anyOf", "oneOf"):
            for branch in schema.get(keyword, []):
                stack.append((path, branch, chain))
        for keyword in ("if", "then", "else"):
            if keyword in schema:
                stack.append((path, schema[keyword], chain))
        for name, child in schema.get("properties", {}).items():
            stack.append((path + (key_step(name),), child, frozenset()))
        for child in schema.get("patternProperties", {}).values():
            stack.append((path + (MAPVAL_STEP,), child, frozenset()))
        additional = schema.get("additionalProperties")
        if isinstance(additional, dict):
            stack.append((path + (MAPVAL_STEP,), additional, frozenset()))
        for child in schema.get("prefixItems", []):
            stack.append((path + (ELEM_STEP,), child, frozenset()))
        if "items" in schema:
            stack.append((path + (ELEM_STEP,), schema["items"], frozenset()))
    return by_path


def _is_object(s: Any) -> bool:
    return isinstance(s, dict) and ("object" in _types(s) or any(k in s for k in ("properties", "patternProperties", "additionalProperties")))


def _is_array(s: Any) -> bool:
    return isinstance(s, dict) and ("array" in _types(s) or any(k in s for k in ("prefixItems", "items", "contains")))


def _is_scalar(s: Any) -> bool:
    return isinstance(s, dict) and ("enum" in s or "const" in s or bool(_types(s) & SCALARS))


def _one(votes: list[str | None]) -> str:
    found = sorted({v for v in votes if v is not None})
    return found[0] if len(found) == 1 else UNKNOWN


def reference_labels(root: Any) -> dict[tuple[Path, str], str]:
    labels: dict[tuple[Path, str], str] = {}
    for path, schemas in _subschemas_by_path(root).items():
        own = [s for s in schemas if not (isinstance(s, dict) and set(s) == {"$ref"})]
        objects = [s for s in own if _is_object(s)]
        if objects:
            labels[path, "record_or_map"] = _one(
                [
                    "record" if "properties" in s else "map"
                    if "patternProperties" in s or isinstance(s.get("additionalProperties"), dict) else None
                    for s in objects
                ]
            )
            if any("properties" in s for s in objects):
                labels[path, "closed_or_open_record"] = _one(
                    [
                        None if "additionalProperties" not in s else "closed" if s["additionalProperties"] is False else "open"
                        for s in objects
                    ]
                )
            split = False
            for s in own:
                if not isinstance(s, dict):
                    continue
                for keyword in ("anyOf", "oneOf"):
                    branches = []
                    for b in s.get(keyword, []):
                        pointer = _pointer(b.get("$ref")) if isinstance(b, dict) and set(b) == {"$ref"} else None
                        branches.append(_resolve(root, pointer) if pointer is not None else b)
                    if sum(1 for b in branches if _is_object(b)) >= 2:
                        split = True
            labels[path, "merge_or_split"] = "split" if split else "merge" if len(objects) == 1 else UNKNOWN
        arrays = [s for s in own if _is_array(s)]
        if arrays:
            labels[path, "tuple_or_list"] = _one(
                ["tuple" if "prefixItems" in s else "list" if "items" in s else None for s in arrays]
            )
        scalars = [s for s in own if _is_scalar(s)]
        if scalars:
            labels[path, "closed_or_open_enum"] = _one(
                [
                    "closed" if "enum" in s or "const" in s else "open" if _types(s) & OPEN_ENUM else None
                    for s in scalars
                ]
            )
    return labels


# -- the property -----------------------------------------------------------------


def models() -> st.SearchStrategy[SchemaModel]:
    return st.one_of(
        schema_models(max_leaves=8),
        planted_schemas(max_depth=2).map(lambda doc: SchemaModel(SchemaNode.from_json(doc))),
    )


@given(model=models())
def test_labels_follow_the_keyword_rules(model: SchemaModel) -> None:
    labels = derive_labels(model, MAX_DEPTH)
    expected = reference_labels(model.root.to_json())
    assert labels == expected
    assert all(kind in KINDS for (_, kind) in labels)
    known = sum(1 for label in labels.values() if label != UNKNOWN)
    event(f"known labels: {'none' if not known else 'some'}")
    event(f"unknown labels: {'none' if known == len(labels) else 'some'}")


@given(model=models())
def test_a_schema_scores_one_against_itself(model: SchemaModel) -> None:
    labels = derive_labels(model, MAX_DEPTH)
    accuracy = judgment_accuracy(labels, labels)
    for kind in KINDS:
        has_known = any(k == kind and label != UNKNOWN for (_, k), label in labels.items())
        assert accuracy[kind] == (1.0 if has_known else None), kind
    # The labels survive emitting and loading the schema (the harness compares emitted schemas, 31.9).
    document, _ = SchemaEmitter().layout(model)
    reloaded = SchemaLoader().load(document)
    assert derive_labels(reloaded, MAX_DEPTH) == labels
    assert judgment_accuracy(labels, derive_labels(reloaded, MAX_DEPTH)) == accuracy
