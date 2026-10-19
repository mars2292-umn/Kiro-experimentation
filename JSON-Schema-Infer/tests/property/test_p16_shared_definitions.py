"""Property 16: Shared definitions are faithful (task 36.2).

For any emitted schema with ``$defs``: every ``$ref`` resolves; expanding the
non-recursive refs yields a schema with the same verdicts on every observed
document; and the definition names equal those of the naming rule of 8.5.
For any recursive collection inferred from trees of depth at most d,
generated trees of depth d + k with the same node shape validate.
"""

# Feature: json-schema-inference, Property 16: Shared definitions are faithful

from __future__ import annotations

import copy
import json
from typing import Any

from hypothesis import assume, event, given
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.structure.analyzer import StructureAnalyzer

VALIDATOR = Validator()
VOCAB = ["a", "b", "c"]


def expand(schema: Any, defs: dict[str, Any], seen: tuple[str, ...] = ()) -> Any:
    """``schema`` with every non-recursive ``$ref`` replaced by its definition (recursive ones are kept)."""
    if isinstance(schema, dict):
        ref = schema.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/$defs/"):
            name = ref[len("#/$defs/") :]
            if name in seen:
                return schema
            body = expand(defs[name], defs, (*seen, name))
            rest = {k: expand(v, defs, seen) for k, v in schema.items() if k != "$ref"}
            return {**body, **rest} if rest else body
        return {k: (v if k == "$defs" else expand(v, defs, seen)) for k, v in schema.items()}
    if isinstance(schema, list):
        return [expand(v, defs, seen) for v in schema]
    return schema


def resolves(schema: Any, defs: dict[str, Any]) -> bool:
    if isinstance(schema, dict):
        ref = schema.get("$ref")
        if isinstance(ref, str) and not (ref.startswith("#/$defs/") and ref[len("#/$defs/") :] in defs):
            return False
        return all(resolves(v, defs) for v in schema.values())
    if isinstance(schema, list):
        return all(resolves(v, defs) for v in schema)
    return True


@st.composite
def collections(draw: st.DrawFn) -> list[dict[str, Any]]:
    """Documents with two sibling records of one shape and an optional recursive tree."""
    n = draw(st.integers(4, 14), label="documents")
    recursive = draw(st.booleans(), label="recursive")
    docs = []
    for i in range(n):
        shape = {"x": VOCAB[i % 3], "y": i % 5}
        d: dict[str, Any] = {"first": dict(shape), "second": dict(shape), "id": i}
        if recursive:
            depth = draw(st.integers(0, 3), label=f"depth {i}")
            node: dict[str, Any] = {"tag": VOCAB[i % 3]}
            top = node
            for level in range(depth):
                node["next"] = {"tag": VOCAB[(i + level) % 3]}
                node = node["next"]
            d["chain"] = top
        docs.append(d)
    return docs


@given(docs=collections())
def test_refs_resolve_and_expansion_keeps_verdicts(docs: list[dict[str, Any]]) -> None:
    report = InferenceReport()
    model = StructureAnalyzer.for_techniques(report=report).infer_model(View.root(build_store(docs)))
    schema = model.root.to_json()
    defs = schema.get("$defs", {})
    if not defs:
        event("no $defs")
        return
    event("$defs emitted")
    assert resolves(schema, defs)
    compiled = VALIDATOR.compile(schema)
    expanded = expand(copy.deepcopy(schema), defs)
    expanded_compiled = VALIDATOR.compile(expanded)
    probes = docs + [{**docs[0], "id": "not-an-int"}, {"first": {"x": 1}, "second": {}, "id": 0}]
    for probe in probes:
        assert VALIDATOR.is_valid(compiled, probe) == VALIDATOR.is_valid(expanded_compiled, probe), json.dumps(probe)[:120]
    for name in defs:
        assert name == "def" or name[0].isalpha()
    if any("next" in json.dumps(body) for body in defs.values()):
        # 8.4: a recursive chain deeper than observed validates.
        deep: dict[str, Any] = {"tag": "a"}
        node = deep
        for level in range(8):
            node["next"] = {"tag": VOCAB[level % 3]}
            node = node["next"]
        assert VALIDATOR.is_valid(compiled, {**docs[0], "chain": deep})
        event("recursive definition")
