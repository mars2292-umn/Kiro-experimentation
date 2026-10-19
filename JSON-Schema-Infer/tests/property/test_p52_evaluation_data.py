"""Property 52: Evaluation data preparation (task 29.3).

For any Held_Out_Schema and seed, the held-out Positive_Documents are
distinct, accepted by the schema, and disjoint as JSON values from every
training collection (31.1). For any Real_Dataset, exactly the documents
that fail parsing, the positives the known schema rejects, and the
negatives it accepts are excluded and recorded with their reasons (33.5).
Fold assignment is a deterministic partition into min(k, n) folds that does
not depend on the system (33.3).
"""

# Feature: json-schema-inference, Property 52: Evaluation data preparation

from __future__ import annotations

import json
from typing import Any

from hypothesis import assume, event, given
from hypothesis import strategies as st

from jsinfer.bench.real import CleanedDataset, DatasetExclusion, RealDataset, clean, folds
from jsinfer.bench.synthetic import Exclusion, SyntheticConfig, SyntheticData, prepare
from jsinfer.core.jsonvalue import canonical_key
from jsinfer.io.load import SchemaLoader
from jsinfer.io.validator import Validator
from jsinfer.learn.synth import GeneratorConfig
from tests.strategies import json_values, planted_schemas

VALIDATOR = Validator()


@given(schema=planted_schemas(max_depth=2), seed=st.integers(0, 2**31 - 1), sizes=st.lists(st.integers(1, 4), min_size=1, max_size=2), held=st.integers(1, 4))
def test_held_out_positives_are_distinct_from_training_documents(schema: dict[str, Any], seed: int, sizes: list[int], held: int) -> None:
    cfg = SyntheticConfig(sizes=tuple(sizes), held_out=held, seed=seed, generator=GeneratorConfig(size_min=1, size_max=4, doc_attempts=5))
    model = SchemaLoader().load(schema)
    outcome = prepare(model, "s", cfg, validator=VALIDATOR)
    if isinstance(outcome, Exclusion):
        event("excluded")
        assume(False)
    assert isinstance(outcome, SyntheticData)
    event("prepared")
    training = {canonical_key(d) for size in outcome.training for d in outcome.training[size]}
    assert [len(outcome.training[s]) for s in sizes] == sizes
    keys = [canonical_key(d) for d in outcome.positives]
    assert len(keys) == held and len(set(keys)) == held
    assert not training & set(keys)
    compiled = VALIDATOR.compile(outcome.truth_document)
    assert all(VALIDATOR.is_valid(compiled, d) for d in outcome.positives)
    assert all(not VALIDATOR.is_valid(compiled, n.document) for n in outcome.negatives)
    again = prepare(model, "s", cfg, validator=VALIDATOR)
    assert isinstance(again, SyntheticData) and again.positives == outcome.positives and again.training == outcome.training


SCHEMA = {"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"], "additionalProperties": False}
COMPILED = VALIDATOR.compile(SCHEMA)


@st.composite
def raw_documents(draw: st.DrawFn) -> list[tuple[str, bytes, str]]:
    """``(id, bytes, expected verdict)`` where the verdict is ``parse``, ``accept``, or ``reject``."""
    out = []
    for i in range(draw(st.integers(0, 8))):
        choice = draw(st.integers(0, 3))
        if choice == 0:
            out.append((f"d{i}", b"{" + draw(st.binary(max_size=4)), "parse"))
        elif choice == 1:
            out.append((f"d{i}", json.dumps({"id": draw(st.integers(-5, 5))}).encode(), "accept"))
        else:
            value = draw(json_values(max_leaves=4))
            verdict = "accept" if VALIDATOR.is_valid(COMPILED, value) else "reject"
            out.append((f"d{i}", json.dumps(value).encode(), verdict))
    return out


@given(positives=raw_documents(), negatives=raw_documents(), k=st.integers(2, 6), seed=st.integers(0, 1000))
def test_cleaning_excludes_exactly_the_contradicted_documents(positives, negatives, k: int, seed: int) -> None:
    dataset = RealDataset("ds", tuple((i, b) for i, b, _ in positives), SCHEMA, tuple(("n" + i, b) for i, b, _ in negatives))
    outcome = clean(dataset, VALIDATOR)
    expected_positives = [i for i, _, v in positives if v == "accept"]
    expected = [(i, "parse failure" if v == "parse" else "positive rejected") for i, _, v in positives if v != "accept"]
    expected += [("n" + i, "parse failure" if v == "parse" else "negative accepted") for i, _, v in negatives if v != "reject"]
    if len(expected_positives) < 2:
        event("dataset excluded")
        assert isinstance(outcome, DatasetExclusion) and outcome.remaining == len(expected_positives)
        return
    event("dataset kept")
    assert isinstance(outcome, CleanedDataset)
    assert [i for i, _ in outcome.positives] == expected_positives
    assert [i for i, _ in outcome.negatives] == ["n" + i for i, _, v in negatives if v == "reject"]
    assert [(e.document_id, e.reason) for e in outcome.excluded] == expected
    parts = folds(outcome.n, k, "ds", seed)
    assert len(parts) == min(k, outcome.n)
    assert sorted(i for p in parts for i in p) == list(range(outcome.n))
    assert parts == folds(outcome.n, k, "ds", seed)
    assert max(len(p) for p in parts) - min(len(p) for p in parts) <= 1
