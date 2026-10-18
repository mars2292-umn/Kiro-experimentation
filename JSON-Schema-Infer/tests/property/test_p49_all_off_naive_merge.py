"""Property 49: All Techniques off equals Naive_Merge (task 19.7).

For any Document_Collection, the schema the Inferrer emits under a
Run_Configuration that disables every registered Technique is structurally
equal to the Naive_Merge output over the same parsed documents (28.5), the
Subschema_Checker reports each as a subschema of the other (28.5), and the
Naive_Merge output uses only ``$schema``, ``type``, ``properties``,
``items``, and ``required``, with the integer rule (34.4).

Each example runs the whole in-memory pipeline
(``pipeline.infer.infer_documents``) with ``techniques.<id> = false`` for
every id of the registry, implemented or not, and ``workers = 1``. The other
settings vary, since 28.5 holds for any such configuration:
``output.inline_provenance`` (on or off), ``ingest.chunk_size`` (1, 2, 3, or
the default 2000), and the seed.

Checks
    - **Structural equality (28.5, 2.7).** The Schema_Model after the
      soundness stage equals ``naive_merge`` of the run's parsed documents
      (``result.documents``, the Ingestor's DocumentStore) under Schema_Model
      equality, which ignores ``$schema`` and ``x-provenance``. The emitted
      bytes are the Naive_Merge file's bytes
      (``SchemaEmitter().emit(naive_merge(...))``) with inline provenance
      off; with it on, they are those bytes once every ``x-provenance``
      keyword is removed from the emitted document, so the annotation is the
      only difference.
    - **Naive_Merge is already sound.** The soundness stage finds no failing
      document on its first pass and makes no relaxation or removal, and no
      document is flagged: with every Technique off, nothing the soundness
      loop could do may account for the equality.
    - **Subschema_Checker (28.5).** ``SubschemaChecker().relation(emitted,
      naive)`` runs the pinned jsonsubschema in a child process, on the
      emitted document as it is (with ``x-provenance`` when inline provenance
      is on, which the checker strips) against the emitted Naive_Merge
      document, and must return ``Relation.EQUIVALENT``: both inclusion
      checks hold. An ``Unsupported`` result fails the example with its
      cause and message, whatever the cause. ``keywords`` cannot arise from
      the five Naive_Merge keywords (all supported); ``error`` (jsonsubschema
      raising on one of its own unsupported cases), ``timeout``, and
      ``unavailable`` mean the checker did not report the two inclusions
      28.5 asks for, so they are not accepted silently. None occurred while
      this test was written.
    - **Naive_Merge's keywords and the integer rule (34.4).** The emitted
      document (``x-provenance`` removed) equals an independent reference
      built here from the raw generated values (:func:`reference_naive`,
      written from 34.4 and 13.2, not from ``core.stats``): ``type`` is the
      union of the observed JSON types in the emitted order, with
      ``integer`` instead of ``number`` exactly when every number at the
      Path has a zero fractional part (``1.0``, ``-0.0``, ``1e3``, and
      ``1.5e300`` included); ``properties`` the union of the observed keys;
      ``items`` one subschema merging every element; ``required`` the keys
      present in every object. Every keyword in the document is one of the
      five, and ``$schema`` appears only at the root.

Inputs
    Collections of one to eight documents, sometimes repeated three times so
    values recur (the inputs on which enum estimation, bounds, and
    co-occurrence mining would otherwise emit Constraints). Documents mix
    free JSON values of every type (nested lists and objects, empty ones
    included), records sharing a few base keys with optional extras (so
    ``required`` is non-trivial), lists of records, Map-like objects keyed
    by UUIDs or dates (Record-or-Map's fallback is Record), and ``[int,
    str]`` pairs (Tuple-or-List's fallback is List). Scalars cover integers
    (beyond 64 bits too), integral floats, non-integral floats, and strings
    that look like Formats or latent numbers, which the disabled Format,
    latent-type, and pattern Techniques must leave as plain ``string``.
    Keys include the empty key, keys whose code-point order differs from
    UTF-16 order, JSON Pointer escapes, and keyword-like names such as
    ``type``, ``$ref``, and ``x-provenance``, which are property names, not
    keywords.

Runtime
    The Subschema_Checker starts two child processes per example (about
    0.15 s per example, about 15 s for the 100 examples of the ``ci``
    profile); the pipeline runs take a few seconds in total.

**Validates: Requirements 28.5, 34.4**
"""

# Feature: json-schema-inference, Property 49: All Techniques off equals Naive_Merge

from __future__ import annotations

import json
from datetime import date
from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import SEED_MAX, RunConfiguration, technique_setting
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.subschema import Relation, SubschemaChecker, Unsupported
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.pipeline.naive_merge import NAIVE_MERGE_KEYWORDS, naive_merge

DIALECT = "https://json-schema.org/draft/2020-12/schema"
EMITTER = SchemaEmitter()
CHECKER = SubschemaChecker()  # the real jsonsubschema, one child process per inclusion check

# Every registered Technique disabled, implemented or not (28.5).
ALL_OFF: dict[str, bool] = {technique_setting(tid): False for tid in REGISTRY.ids()}

# The JSON types in the order the emitter lists them (io.emit, Layout).
TYPE_ORDER = ("null", "boolean", "integer", "number", "string", "array", "object")
PROVENANCE = "x-provenance"

# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------

# Keys repeat across objects. They include the empty key, keys whose
# code-point order differs from UTF-16 order ("\uffff" < "😀"), JSON Pointer
# escapes, and property names that look like keywords.
KEYS = ("", "a", "b", "B", "id", "é", "\uffff", "😀", "a b", "~1", "a/b", "type", "items", "required", "$ref", PROVENANCE)

INTEGERS = st.integers(-3, 3) | st.sampled_from([2**31, 2**53 + 1, 2**63, 2**64, 2**70, -(2**63) - 1])
# Floats with a zero fractional part count as integers (13.2); every double beyond 2**53 is one.
INTEGRAL_FLOATS = st.sampled_from([0.0, -0.0, 1.0, -2.0, 1e3, 2.0**70, 1.5e300])
FRACTIONAL_FLOATS = st.sampled_from([2.5, -0.5, 0.1, 1e-7, 123.456, 1.0000000000000002])
# Strings that Format detection, latent-type detection, or pattern induction would describe.
FORMAT_LIKE = st.sampled_from(
    [
        "2024-01-31",
        "2024-01-31T12:00:00Z",
        "12:00:00",
        "P1D",
        "a@b.co",
        "example.com",
        "192.168.0.1",
        "::1",
        "https://example.com/a",
        "550e8400-e29b-41d4-a716-446655440000",
        "123",
        "-1.5",
    ]
)
STRINGS = st.text(alphabet="ab 😀", max_size=3) | FORMAT_LIKE
SCALARS = st.one_of(st.none(), st.booleans(), INTEGERS, INTEGRAL_FLOATS, FRACTIONAL_FLOATS, STRINGS)


def values(max_leaves: int) -> st.SearchStrategy[Any]:
    """Nested JSON values: scalars, lists, and objects (empty ones included) with keys from KEYS."""
    return st.recursive(
        SCALARS,
        lambda children: st.lists(children, max_size=4) | st.dictionaries(st.sampled_from(KEYS), children, max_size=4),
        max_leaves=max_leaves,
    )


VALUES = values(12)
SMALL_VALUES = values(4)

# Map-like objects: many distinct data-like keys, which Record-or-Map would call a Map.
MAP_KEYS = st.uuids().map(str) | st.dates(date(2020, 1, 1), date(2030, 12, 31)).map(date.isoformat)
MAPS = st.dictionaries(MAP_KEYS, SCALARS, min_size=4, max_size=10)
# [int, str] pairs, which Tuple-or-List would call a Tuple.
PAIRS = st.tuples(st.integers(-3, 3), STRINGS).map(list)


@st.composite
def collections(draw: st.DrawFn) -> list[Any]:
    """A Document_Collection of one to eight documents, sometimes repeated three times."""
    base = draw(st.lists(st.sampled_from(KEYS), unique=True, max_size=3), label="record keys")
    extra = draw(st.lists(st.sampled_from([k for k in KEYS if k not in base]), unique=True, max_size=3), label="optional keys")
    records = st.fixed_dictionaries(
        {key: SMALL_VALUES for key in base}, optional={key: SMALL_VALUES for key in extra}
    )
    document = st.one_of(
        VALUES,
        records,
        st.lists(records, max_size=3),
        MAPS,
        st.dictionaries(st.sampled_from(KEYS), MAPS | st.lists(PAIRS, max_size=3), max_size=2),
    )
    docs = draw(st.lists(document, min_size=1, max_size=8))
    copies = draw(st.sampled_from([1, 1, 3]), label="copies")
    return docs * copies


@st.composite
def all_off_configs(draw: st.DrawFn) -> RunConfiguration:
    """Run_Configurations with every Technique disabled and one worker; the other settings vary."""
    return RunConfiguration.resolve(
        {
            "workers": 1,
            "output.inline_provenance": draw(st.booleans(), label="inline provenance"),
            "ingest.chunk_size": draw(st.sampled_from([1, 2, 3, 2000]), label="chunk size"),
            "seed": draw(st.just(0) | st.integers(0, SEED_MAX), label="seed"),
            **ALL_OFF,
        }
    )


# ---------------------------------------------------------------------------
# The reference Naive_Merge (Requirement 34.4) and helpers
# ---------------------------------------------------------------------------


def json_type(value: Any) -> str:
    """The JSON type of ``value``, ``integer`` for a number with a zero fractional part (13.2)."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "integer" if value.is_integer() else "number"
    if isinstance(value, str):
        return "string"
    return "array" if isinstance(value, list) else "object"


def reference_naive(observed: list[Any]) -> dict[str, Any]:
    """The Naive_Merge subschema of the values observed at one Path, laid out as the emitter writes it.

    ``type`` lists the observed JSON types in emitted order, ``number``
    replacing ``integer`` when some number is non-integral, written as a
    string when there is one; ``properties`` holds every observed key in
    code-point order; ``required`` the keys present in every object;
    ``items`` the merge of every element of every array. No values give
    ``{}``.
    """
    if not observed:
        return {}
    present = {json_type(value) for value in observed}
    if "number" in present:
        present.discard("integer")
    names = [name for name in TYPE_ORDER if name in present]
    schema: dict[str, Any] = {"type": names[0] if len(names) == 1 else names}
    objects = [value for value in observed if isinstance(value, dict)]
    keys = sorted({key for obj in objects for key in obj})  # str order is code-point order
    if keys:
        schema["properties"] = {key: reference_naive([obj[key] for obj in objects if key in obj]) for key in keys}
        required = [key for key in keys if all(key in obj for obj in objects)]
        if required:
            schema["required"] = required
    elements = [element for value in observed if isinstance(value, list) for element in value]
    if elements:
        schema["items"] = reference_naive(elements)
    return schema


def subschemas(schema: dict[str, Any]) -> list[dict[str, Any]]:
    """The subschemas of a Naive_Merge-shaped schema object: ``items`` and each ``properties`` entry."""
    found = list(schema.get("properties", {}).values())
    if "items" in schema:
        found.append(schema["items"])
    return found


def without_provenance(schema: dict[str, Any]) -> dict[str, Any]:
    """A copy of ``schema`` without the ``x-provenance`` keyword of any schema object.

    Only keywords are removed: a property named ``x-provenance`` stays.
    """
    copy = {keyword: value for keyword, value in schema.items() if keyword != PROVENANCE}
    if "properties" in copy:
        copy["properties"] = {name: without_provenance(sub) for name, sub in copy["properties"].items()}
    if "items" in copy:
        copy["items"] = without_provenance(copy["items"])
    return copy


def keyword_uses(schema: dict[str, Any], pointer: str = "") -> list[tuple[str, str]]:
    """Every ``(pointer, keyword)`` of a Naive_Merge-shaped document, walking ``properties`` and ``items``."""
    uses = [(pointer, keyword) for keyword in schema]
    for name, sub in schema.get("properties", {}).items():
        escaped = name.replace("~", "~0").replace("/", "~1")
        uses.extend(keyword_uses(sub, f"{pointer}/properties/{escaped}"))
    if "items" in schema:
        uses.extend(keyword_uses(schema["items"], f"{pointer}/items"))
    return uses


def describe(relation: Relation | Unsupported) -> str:
    if isinstance(relation, Relation):
        return relation.value
    return f"Unsupported({relation.cause}): {relation.message}"


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@given(collections(), all_off_configs())
def test_every_technique_off_emits_the_naive_merge_schema(docs: list[Any], config: RunConfiguration) -> None:
    result: PipelineResult = infer_documents(docs, config)
    inline = config.output.inline_provenance

    # The run's parsed documents: every generated value is ingestible, in order.
    parsed = list(result.documents.values())
    assert len(parsed) == len(docs)
    assert not result.report.to_json()["ingestion"]["errors"]
    expected = naive_merge(parsed)
    expected_doc, _ = EMITTER.layout(expected)
    expected_bytes = EMITTER.dumps(expected_doc)

    # Naive_Merge is sound, so the soundness stage changes nothing, and no document is flagged.
    summary = result.soundness.summary
    assert (summary.failed_first_pass, summary.relaxations, summary.removals) == (0, 0, 0)
    assert summary.validated == len(docs) and result.flagged == ()

    # 28.5, structurally: Schema_Model equality (2.7) and the emitted bytes.
    assert result.model == expected
    emitted = json.loads(result.schema)
    assert emitted == result.document
    assert isinstance(emitted, dict)
    stripped = without_provenance(emitted)
    assert EMITTER.dumps(stripped) == expected_bytes
    if inline:
        event("inline provenance")
    else:
        assert result.schema == expected_bytes and stripped == emitted

    # 34.4: the five keywords, $schema only at the root, and the integer rule, from the raw values.
    uses = keyword_uses(stripped)
    assert {keyword for _, keyword in uses} <= set(NAIVE_MERGE_KEYWORDS)
    assert [pointer for pointer, keyword in uses if keyword == "$schema"] == [""]
    assert stripped == {"$schema": DIALECT, **reference_naive(docs)}
    assert expected_doc == stripped

    # 28.5, with the Subschema_Checker: each is a subschema of the other.
    relation = CHECKER.relation(result.document, expected_doc)
    event(f"relation: {relation.value if isinstance(relation, Relation) else relation.cause}")
    assert relation is Relation.EQUIVALENT, f"the Subschema_Checker reported {describe(relation)}"
