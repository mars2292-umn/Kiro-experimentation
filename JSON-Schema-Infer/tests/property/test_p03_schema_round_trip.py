"""Property 3: Schema emit/load round trip (task 6.4).

For any Schema_Model in the supported keyword grammar, emitting it with the
Schema_Emitter and loading the emitted document with the Schema_Loader yields
a fully supported Schema_Model equal to the original under the equality of
Requirement 2.7. Emitting the loaded model again gives the same bytes, so the
emitted form is a fixed point of load-then-emit.

Models come from :func:`tests.strategies.schema_models`: every keyword of
Requirement 2.4 (local and recursive ``$ref`` into ``$defs`` and ancestors
included), boolean schemas at every subschema position and at the root, the
retained annotations, a root ``$id``, and ``x-`` keywords (2.10), with values
in non-canonical form (repeats in set-compared lists, integral floats,
``-0.0``, big integers, and keywords in a drawn order). Loading the emitted
bytes also runs the draft 2020-12 meta-schema check with formats asserted, so
an emitted document the meta-schema rejects fails this test as a
``SchemaLoadError``.

Schema_Models produced by the Inferrer are added in task 16.10, and Schema_Corpus
schemas (Requirement 2.8) in task 25.7.

**Validates: Requirements 2.4, 2.7, 2.8**
"""

# Feature: json-schema-inference, Property 3: Schema emit/load round trip

from __future__ import annotations

from hypothesis import event, given

from jsinfer.core.schema_model import SchemaModel
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoader
from tests.strategies import schema_models

EMITTER = SchemaEmitter()
LOADER = SchemaLoader()


def _describe(model: SchemaModel) -> None:
    """Hypothesis events showing which structural cases the examples reached."""
    if model.root.boolean is not None:
        event("root: boolean")
        return
    nodes = list(model.walk())
    event(f"nodes: {'1' if len(nodes) == 1 else '2-9' if len(nodes) < 10 else '10+'}")
    keywords = {keyword for _, node in nodes for keyword in node.kw}
    for feature in ("$ref", "$defs", "$id"):
        if feature in keywords:
            event(f"has {feature}")
    if any(node.boolean is not None for _, node in nodes):
        event("has boolean subschemas")


@given(schema_models())
def test_emit_then_load_gives_an_equal_fully_supported_model(model: SchemaModel) -> None:
    _describe(model)
    emitted = EMITTER.emit(model)
    loaded = LOADER.load(emitted)

    assert loaded.fully_supported, f"unsupported constructs after loading: {loaded.unsupported}"
    assert loaded == model, (
        "the loaded model differs from the original under 2.7 equality\n"
        f"emitted:\n{emitted.decode('utf-8')}"
    )
    assert EMITTER.emit(loaded) == emitted, "emitting the loaded model changed the bytes"
