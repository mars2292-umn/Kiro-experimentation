"""Property 33: Generator correctness and determinism (task 21.4).

For any compilable supported schema, seed, and worker count, every document
the Synthetic_Generator emits validates against the source schema (22.7).
Two runs with the same seed produce byte-identical documents and labels in
the same order (22.9); documents generated one at a time in any order are
the same documents, which is what makes the output independent of the
number of workers (the unit tests run the joblib path with two workers).
The sampled collection parameters stay in the configured ranges, the
collection has exactly the sampled size, and the labels are those of
``derive_labels``. A schema the generator excludes (22.8) is either one
the Validator cannot compile, which the reason names, or one for which some
document exhausted the attempt budget.

Inputs
    Planted schemas from ``tests.strategies.planted_schemas`` and fully
    supported Schema_Models from ``tests.strategies.schema_models`` without
    ``$ref`` (reference cycles that consume no instance may not terminate
    under validation; recursion is covered by the planted schemas). Seeds
    are arbitrary non-negative integers; the value index is built from the
    schema itself and a second schema with same-name fields.

**Validates: Requirements 22.7, 22.9**
"""

# Feature: json-schema-inference, Property 33: Generator correctness and determinism

from __future__ import annotations

from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.io.validator import CompileError, Validator
from jsinfer.learn.labels import derive_labels
from jsinfer.learn.synth import GeneratorConfig, SchemaExcluded, SyntheticGenerator, ValueIndex, collection_digest
from tests.strategies import planted_schemas, schema_models

CFG = GeneratorConfig(size_min=1, size_max=6, max_depth=3, doc_attempts=12)
VALIDATOR = Validator()

# A schema whose fields share normalized names with the planted ones, for source (b).
OTHER = SchemaModel(
    SchemaNode.from_json(
        {
            "properties": {
                "Name": {"examples": ["Ada", "Grace"]},
                "e-mail": {"examples": ["ada@example.com"]},
                "createdAt": {"default": "2020-01-02T03:04:05Z"},
                "count": {"enum": [1, 2, 3]},
            }
        }
    )
)


def models() -> st.SearchStrategy[SchemaModel]:
    return st.one_of(
        planted_schemas(max_depth=2).map(lambda doc: SchemaModel(SchemaNode.from_json(doc))),
        planted_schemas(max_depth=3).map(lambda doc: SchemaModel(SchemaNode.from_json(doc))),
        schema_models(max_leaves=6, refs=False),
    )


@given(model=models(), seed=st.integers(0, 2**32 - 1), k=st.integers(0, 3), data=st.data())
def test_generated_documents_validate_and_runs_are_identical(
    model: SchemaModel, seed: int, k: int, data: st.DataObject
) -> None:
    schema: Any = model.root.to_json()
    index = ValueIndex.build([("self", model), ("other", OTHER)])
    generator = SyntheticGenerator(index=index)
    try:
        first = generator.collection(model, "schema", k, CFG, seed, split="train")
    except SchemaExcluded as exc:
        assert exc.schema_id == "schema"
        if exc.reason.startswith("uncompilable: "):
            event("excluded: uncompilable")
            try:
                VALIDATOR.compile(schema)
            except CompileError:
                return
            raise AssertionError("the Validator compiles a schema the generator called uncompilable")
        assert exc.reason.startswith(f"no accepted document within {CFG.doc_attempts} attempts")
        event("excluded: attempt budget")
        return
    event("generated")

    # Correctness (22.7): every document validates against the source schema.
    compiled = VALIDATOR.compile(schema)
    assert first.documents and all(VALIDATOR.is_valid(compiled, doc) for doc in first.documents)
    assert len(first.documents) == first.params.size >= 1
    assert all(0.0 <= rate <= 1.0 for rate in first.params.presence_rates.values())
    assert 0.0 <= first.params.array_length_mean <= CFG.array_length_max
    assert CFG.enum_skew_min <= first.params.enum_skew <= CFG.enum_skew_max
    assert first.labels == derive_labels(model, CFG.max_depth)
    assert (first.schema_id, first.k, first.split) == ("schema", k, "train")

    # Determinism (22.9): a fresh generator with its own Validator gives the same bytes and labels.
    second = SyntheticGenerator(Validator(), ValueIndex.build([("self", model), ("other", OTHER)])).collection(
        model, "schema", k, CFG, seed, split="train"
    )
    assert collection_digest(second) == collection_digest(first)
    assert second.labels == first.labels and second.params == first.params
    assert canonical_bytes(second.params.to_json()) == canonical_bytes(first.params.to_json())

    # Order and worker independence: documents generated one by one, in a drawn order, are the same.
    order = data.draw(st.permutations(range(first.params.size)), label="order")
    single = {i: generator.document(model, "schema", k, i, CFG, first.params, seed)[0] for i in order}
    assert [single[i] for i in range(first.params.size)] == list(first.documents)
