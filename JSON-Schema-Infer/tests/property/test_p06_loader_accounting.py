"""Property 6: Loader accounting (task 6.7).

For any supported schema into which unsupported constructs, unknown
non-vocabulary keywords, annotations, and ``x-`` keywords are injected at
random pointers, the loaded model omits exactly the unsupported and unknown
keywords and records each with its pointer. It retains the annotations and
``x-`` keywords, is marked partially supported if and only if an unsupported
construct was injected, and records an assumed dialect if and only if no
recognized ``$schema`` is declared.

Each example starts from a fully supported draft 2020-12 Schema_Model drawn by
:func:`tests.strategies.schema_models` (local and recursive ``$ref`` included),
takes its emitted form without ``$schema`` as the base document (the drawn
values are not canonical, and the emitter's are ones the meta-schema accepts),
and turns that into a source document by injecting, at drawn schema objects and
at drawn member positions:

- unknown keywords (``markdownDescription``, ``id``, ``additionalItems``,
  ``X-upper``, random names, ...), expected as ignored records (2.10), and
  ``$schema`` below the root, which the loader documents as ignored;
- the keywords draft 2020-12 defines outside the supported set (2.6), such as
  ``multipleOf``, ``dependentSchemas``, ``unevaluatedProperties``,
  ``minContains``, ``contentEncoding``, ``$dynamicRef``, ``$anchor``,
  ``definitions``, and ``dependencies``, expected as unsupported records;
- ``$id`` below the root (unsupported). One that is not a bare fragment starts
  an embedded resource, so every ``$ref`` in its subtree, from the base or
  injected, is unsupported too;
- ``$ref`` values that are remote, relative, or plain-name fragments, or local
  pointers that do not resolve in the loaded model (into an omitted construct,
  at a container, at a non-schema keyword, at a missing entry, or malformed),
  expected as unsupported; and local ``$ref`` values to a schema position of
  the base, expected to load as written;
- the retained annotations of 2.10, a root ``$id``, and ``x-`` keywords,
  expected to be kept;
- at the root, no ``$schema``, a recognized draft 2020-12 URI in one of its
  spellings, or an unrecognized URI (unsupported, dialect assumed; 2.6, 2.11).

The values of omitted constructs hold keywords that would be recorded at a
schema position (``multipleOf``, ``$id``, a remote ``$ref``, ...), so the test
also checks that nothing below an omitted keyword is recorded. Every injected
document passes the draft 2020-12 meta-schema; a ``SchemaLoadError`` would fail
the test. Older declared drafts are translated first and are the subject of
Property 5, so a recognized ``$schema`` here is always draft 2020-12.

The expected records come from tables written from Requirements 2.4, 2.6, and
2.10 and the draft 2020-12 meta-schemas, not from ``io/load.py``. The loaded
model must equal, under the equality of 2.7, the base model with the kept
injections added and the omitted constructs (and the base ``$ref`` values
inside an embedded resource) left out.

**Validates: Requirements 2.6, 2.10, 2.11**
"""

# Feature: json-schema-inference, Property 6: Loader accounting

from __future__ import annotations

import copy
from dataclasses import dataclass
from typing import Any
from urllib.parse import quote

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.schema_model import SchemaModel, SchemaNode, join_pointer, split_pointer
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoader
from tests.strategies import schema_documents, schema_json_values, schema_models, schema_names

EMITTER = SchemaEmitter()
LOADER = SchemaLoader()

# ---------------------------------------------------------------------------
# Keyword tables (from the requirements and the draft 2020-12 meta-schemas)
# ---------------------------------------------------------------------------

# Requirement 2.4: the supported keywords (local $ref included).
SUPPORTED: frozenset[str] = frozenset(
    {
        "type", "properties", "required", "additionalProperties", "patternProperties", "propertyNames", "items",
        "prefixItems", "contains", "enum", "const", "format", "pattern", "minimum", "maximum", "exclusiveMinimum",
        "exclusiveMaximum", "minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties",
        "uniqueItems", "anyOf", "oneOf", "allOf", "not", "if", "then", "else", "dependentRequired", "$defs", "$ref",
    }
)  # fmt: skip

# Requirement 2.10: the annotations retained at every node.
ANNOTATIONS: frozenset[str] = frozenset(
    {"title", "description", "default", "examples", "deprecated", "readOnly", "writeOnly", "$comment"}
)

# Every keyword draft 2020-12 defines: the core, applicator, unevaluated,
# validation, meta-data, format-annotation, and content vocabularies, plus the
# compatibility keywords of the top-level meta-schema.
DRAFT_2020_12: frozenset[str] = frozenset(
    {
        # core
        "$id", "$schema", "$ref", "$anchor", "$dynamicRef", "$dynamicAnchor", "$vocabulary", "$comment", "$defs",
        # applicator
        "prefixItems", "items", "contains", "additionalProperties", "properties", "patternProperties",
        "dependentSchemas", "propertyNames", "if", "then", "else", "allOf", "anyOf", "oneOf", "not",
        # unevaluated
        "unevaluatedItems", "unevaluatedProperties",
        # validation
        "type", "const", "enum", "multipleOf", "maximum", "exclusiveMaximum", "minimum", "exclusiveMinimum",
        "maxLength", "minLength", "pattern", "maxItems", "minItems", "uniqueItems", "maxContains", "minContains",
        "maxProperties", "minProperties", "required", "dependentRequired",
        # meta-data
        "title", "description", "default", "deprecated", "readOnly", "writeOnly", "examples",
        # format-annotation
        "format",
        # content
        "contentEncoding", "contentMediaType", "contentSchema",
        # compatibility keywords of the top-level meta-schema
        "definitions", "dependencies", "$recursiveAnchor", "$recursiveRef",
    }
)  # fmt: skip

# Requirement 2.6: the 2020-12 keywords outside the supported set. $schema and
# $id have their own rules (dialect detection, kept at the root only).
UNSUPPORTED: frozenset[str] = DRAFT_2020_12 - SUPPORTED - ANNOTATIONS - {"$schema", "$id"}

D2020_SPELLINGS = (
    "https://json-schema.org/draft/2020-12/schema",
    "https://json-schema.org/draft/2020-12/schema#",
    "http://json-schema.org/draft/2020-12/schema",
    "http://json-schema.org/draft/2020-12/schema#",
)
UNRECOGNIZED_DIALECTS = (
    "https://example.com/custom-meta",
    "http://json-schema.org/schema#",
    "https://json-schema.org/draft/2020-12/hyper-schema",
    "https://json-schema.org/draft/2021-01/schema",
    "urn:example:meta-schema",
)

# $id values below the root (the 2020-12 meta-schema allows no non-empty
# fragment). All but the bare fragment start an embedded resource.
RESOURCE_IDS = ("item.json", "https://example.com/sub.json", "urn:example:sub", "sub/")
FRAGMENT_IDS = ("#",)
ROOT_IDS = ("https://example.com/schemas/injected.json", "urn:example:root", "https://example.com/root#")

REMOTE_REFS = (
    "https://example.com/other.json",
    "https://example.com/other.json#/$defs/a",
    "other.json#/$defs/a",
    "../up.json",
    "urn:example:other",
    "#a-name",  # a plain-name fragment (an anchor)
    "#Name_1",
)
# Pointer tokens for local refs that cannot resolve.
CONTAINER_KEYWORDS = ("properties", "patternProperties", "$defs", "anyOf", "oneOf", "allOf", "prefixItems")
LIST_KEYWORDS = ("anyOf", "oneOf", "allOf", "prefixItems")
VALUE_KEYWORDS = ("type", "minimum", "required", "enum", "const", "title", "x-a", "nope", "multipleOf", "$ref", "$id")
BAD_INDICES = ("99", "00", "-1", "x")
MALFORMED_POINTERS = ("/a~2", "/$defs/~x", "/~")
# Characters a fragment may keep unescaped, or only "/".
REF_SAFE_SETS = ("/!$&'()*+,;=:@", "/")

# ---------------------------------------------------------------------------
# Value strategies
# ---------------------------------------------------------------------------

_TEXT = st.text(max_size=6)
_ANCHORS = st.from_regex(r"[A-Za-z_][-A-Za-z0-9._]{0,5}", fullmatch=True)

# Keywords that would be recorded at a schema position. Inside an omitted
# construct they must not be (nothing below an omitted keyword is recorded).
_NOISE: dict[str, st.SearchStrategy] = {
    "markdownDescription": st.just("m"),
    "multipleOf": st.just(3),
    "$id": st.just("inner.json"),
    "$ref": st.sampled_from(["https://example.com/r.json", "#/$defs/nowhere"]),
    "$anchor": st.just("inner"),
    "$schema": st.just("https://example.com/custom-meta"),
}


def emitted_document(root: SchemaNode) -> dict[str, Any] | bool:
    """The Schema_Emitter's document for ``root`` without ``$schema``: canonical values the meta-schema accepts."""
    document, _ = EMITTER.layout(SchemaModel(root))
    if isinstance(document, dict):
        document.pop("$schema", None)
    return document


@st.composite
def noisy_schemas(draw: st.DrawFn) -> dict[str, Any] | bool:
    """Small valid 2020-12 schemas that may hold keywords the loader would record at a schema position."""
    schema = emitted_document(SchemaNode.from_json(draw(schema_documents(max_leaves=2))))
    if isinstance(schema, bool):
        return schema
    for keyword in draw(st.lists(st.sampled_from(sorted(_NOISE)), unique=True, max_size=3)):
        schema[keyword] = draw(_NOISE[keyword], label=f"noise {keyword}")
    return schema


_SMALL_SCHEMAS = noisy_schemas()

UNSUPPORTED_VALUES: dict[str, st.SearchStrategy] = {
    "$anchor": _ANCHORS,
    "$dynamicAnchor": _ANCHORS,
    "$recursiveAnchor": _ANCHORS,
    "$dynamicRef": st.sampled_from(["#meta", "#", "https://example.com/s#node", "other.json"]),
    "$recursiveRef": st.just("#"),
    "$vocabulary": st.just({"https://json-schema.org/draft/2020-12/vocab/core": True}),
    "dependentSchemas": st.dictionaries(schema_names, _SMALL_SCHEMAS, max_size=2),
    "unevaluatedItems": _SMALL_SCHEMAS,
    "unevaluatedProperties": _SMALL_SCHEMAS,
    "contentSchema": _SMALL_SCHEMAS,
    "multipleOf": st.one_of(st.integers(1, 10), st.sampled_from([0.5, 2.5, 1e-3, 2**64])),
    "minContains": st.integers(0, 5),
    "maxContains": st.integers(0, 5),
    "contentEncoding": st.sampled_from(["base64", "7bit", "quoted-printable"]),
    "contentMediaType": st.sampled_from(["application/json", "text/plain", "image/png"]),
    "definitions": st.dictionaries(schema_names, _SMALL_SCHEMAS, max_size=2),
    "dependencies": st.dictionaries(
        schema_names, st.one_of(st.lists(schema_names, unique=True, max_size=2), _SMALL_SCHEMAS), max_size=2
    ),
}
assert set(UNSUPPORTED_VALUES) == UNSUPPORTED, sorted(set(UNSUPPORTED_VALUES) ^ UNSUPPORTED)

ANNOTATION_VALUES: dict[str, st.SearchStrategy] = {
    "title": _TEXT,
    "description": _TEXT,
    "$comment": _TEXT,
    "default": schema_json_values,
    "examples": st.lists(schema_json_values, max_size=3),
    "deprecated": st.booleans(),
    "readOnly": st.booleans(),
    "writeOnly": st.booleans(),
}
assert set(ANNOTATION_VALUES) == ANNOTATIONS


def is_unknown(name: str) -> bool:
    """A keyword that neither draft 2020-12 nor the x- convention defines (ignored, 2.10)."""
    return not name.startswith("x-") and name not in DRAFT_2020_12


# Unknown keywords, including draft-04 "id", the 2019-09 "additionalItems",
# names that only look like x- keywords or 2020-12 keywords, and random names.
_UNKNOWN_CONSTANTS = (
    "markdownDescription", "errorMessage", "deprecationMessage", "nullable", "discriminator", "example", "id",
    "additionalItems", "$foo", "$Ref", "", "X-upper", "x", "xa", "x_", "Type", "properties ", "😀",
)  # fmt: skip
assert all(is_unknown(name) for name in _UNKNOWN_CONSTANTS)
UNKNOWN_NAMES = st.one_of(st.sampled_from(_UNKNOWN_CONSTANTS), _TEXT.filter(is_unknown))
UNKNOWN_VALUES = st.one_of(schema_json_values, _SMALL_SCHEMAS)

X_NAMES = st.one_of(st.sampled_from(("x-", "x-latent-type", "x-😀", "x-provenance")), _TEXT.map(lambda s: "x-" + s))

# ---------------------------------------------------------------------------
# Injection
# ---------------------------------------------------------------------------

UNKNOWN = "unknown keyword"
NESTED_SCHEMA = "$schema below the root"
UNSUPPORTED_KEYWORD = "unsupported keyword"
ID_BELOW_ROOT = "$id below the root"
KEPT = "annotation, x-, or root $id"
LOCAL_REF = "resolving local $ref"
BAD_REF = "remote or unresolved $ref"
CATEGORIES = (UNKNOWN, NESTED_SCHEMA, UNSUPPORTED_KEYWORD, ID_BELOW_ROOT, KEPT, LOCAL_REF, BAD_REF)
REF_CATEGORIES = (LOCAL_REF, BAD_REF)

_Record = tuple[str, str]


@dataclass
class LoaderCase:
    """A source document and what loading it must give."""

    document: dict[str, Any] | bool
    expected_root: SchemaNode
    ignored: set[_Record]
    unsupported: set[_Record]
    unsupported_injected: bool  # an unsupported construct (or an unrecognized $schema) was injected
    dialect: str  # "missing", "recognized", or "unrecognized"
    kept: list[_Record]
    categories: tuple[str, ...]
    scoped_base_refs: int
    bad_ref_kinds: tuple[str, ...]


def _insert(node: SchemaNode, keyword: str, value: Any, index: int) -> None:
    items = list(node.kw.items())
    items.insert(index, (keyword, value))
    node.kw.clear()
    node.kw.update(items)


def _is_prefix(prefix: list[str], tokens: list[str]) -> bool:
    return tokens[: len(prefix)] == prefix


@st.composite
def bad_refs(
    draw: st.DrawFn, positions: list[tuple[str, list[str], SchemaNode]], omitted: list[_Record]
) -> tuple[str, str]:
    """``(kind, value)`` of a ``$ref`` that is not local, or a local pointer that cannot resolve in the loaded model."""
    kinds = ["remote", "container", "value", "index", "missing entry", "malformed"]
    if omitted:
        kinds.append("into omitted")
    kind = draw(st.sampled_from(kinds), label="bad $ref kind")
    if kind == "remote":
        return kind, draw(st.sampled_from(REMOTE_REFS))
    if kind == "malformed":
        pointer = draw(st.sampled_from(MALFORMED_POINTERS))
    elif kind == "into omitted":
        where, keyword = draw(st.sampled_from(omitted), label="omitted target")
        pointer = join_pointer(where, keyword) + draw(st.sampled_from(("", "/a", "/0")))
    else:
        at, _, node = draw(st.sampled_from(positions), label="near")
        if kind == "container":
            pointer = join_pointer(at, draw(st.sampled_from(CONTAINER_KEYWORDS)))
        elif kind == "value":
            pointer = join_pointer(at, draw(st.sampled_from(VALUE_KEYWORDS)))
        elif kind == "index":
            pointer = join_pointer(at, draw(st.sampled_from(LIST_KEYWORDS)), draw(st.sampled_from(BAD_INDICES)))
        else:
            defs = node.kw.get("$defs", {}) if node.boolean is None else {}
            name = draw(schema_names.filter(lambda n: n not in defs), label="missing $defs name")
            pointer = join_pointer(at, "$defs", name)
    return kind, "#" + quote(pointer, safe=draw(st.sampled_from(REF_SAFE_SETS)))


@st.composite
def loader_cases(draw: st.DrawFn) -> LoaderCase:
    # The base document is the emitted form of a drawn model (without $schema),
    # so its values are canonical and pass the meta-schema.
    model: SchemaModel = draw(schema_models(max_leaves=8), label="base model")
    base_root = SchemaNode.from_json(emitted_document(model.root))
    positions = [(pointer, split_pointer(pointer), node) for pointer, node in base_root.walk()]
    objects = [(pointer, tokens) for pointer, tokens, node in positions if node.boolean is None]
    categories = tuple(category for category in CATEGORIES if draw(st.booleans(), label=category))

    doc_root = copy.deepcopy(base_root)  # the source document, with every injection
    expected_root = copy.deepcopy(base_root)  # the model loading must give
    used = {pointer: set(doc_root.resolve(pointer).kw) for pointer, _ in objects}
    ignored: set[_Record] = set()
    unsupported: set[_Record] = set()
    omitted: list[_Record] = []  # injected keywords left out of the model
    kept: list[_Record] = []
    resources: list[list[str]] = []  # tokens of each schema object starting an embedded resource
    injected_unsupported = False

    def inject(pointer: str, keyword: str, value: Any) -> None:
        node = doc_root.resolve(pointer)
        index = draw(st.integers(0, len(node.kw)), label=f"member position of {keyword!r}")
        _insert(node, keyword, value, index)
        used[pointer].add(keyword)

    def keep(pointer: str, keyword: str, value: Any) -> None:
        inject(pointer, keyword, copy.deepcopy(value))
        expected_root.resolve(pointer).kw[keyword] = copy.deepcopy(value)
        kept.append((pointer, keyword))

    # Pass 1: keywords other than $ref and the root $schema.
    for pointer, tokens in objects:
        root = not tokens
        choices = [
            category
            for category in categories
            if category not in REF_CATEGORIES and not (root and category in (NESTED_SCHEMA, ID_BELOW_ROOT))
        ]
        if not choices:
            continue
        for _ in range(draw(st.integers(0, 3), label=f"injections at {pointer!r}")):
            category = draw(st.sampled_from(choices), label="category")
            if category == UNKNOWN:
                name = draw(UNKNOWN_NAMES.filter(lambda n: n not in used[pointer]), label="unknown keyword")
                inject(pointer, name, draw(UNKNOWN_VALUES, label="unknown value"))
                ignored.add((pointer, name))
                omitted.append((pointer, name))
            elif category == NESTED_SCHEMA:
                if "$schema" in used[pointer]:
                    continue
                inject(pointer, "$schema", draw(st.sampled_from(D2020_SPELLINGS)))
                ignored.add((pointer, "$schema"))
                omitted.append((pointer, "$schema"))
            elif category == UNSUPPORTED_KEYWORD:
                available = sorted(UNSUPPORTED - used[pointer])
                if not available:
                    continue
                keyword = draw(st.sampled_from(available), label="unsupported keyword")
                inject(pointer, keyword, draw(UNSUPPORTED_VALUES[keyword], label=keyword))
                unsupported.add((pointer, keyword))
                omitted.append((pointer, keyword))
                injected_unsupported = True
            elif category == ID_BELOW_ROOT:
                if "$id" in used[pointer]:
                    continue
                ident = draw(st.sampled_from(RESOURCE_IDS + FRAGMENT_IDS), label="$id")
                inject(pointer, "$id", ident)
                unsupported.add((pointer, "$id"))
                omitted.append((pointer, "$id"))
                injected_unsupported = True
                if not ident.startswith("#"):
                    resources.append(tokens)
            else:  # KEPT
                options = sorted(ANNOTATIONS - used[pointer]) + ["x-"]
                if root and "$id" not in used[pointer]:
                    options.append("$id")
                option = draw(st.sampled_from(options), label="kept keyword")
                if option == "x-":
                    name = draw(X_NAMES.filter(lambda n: n not in used[pointer]), label="x- keyword")
                    keep(pointer, name, draw(schema_json_values, label="x- value"))
                elif option == "$id":
                    keep(pointer, "$id", draw(st.sampled_from(ROOT_IDS), label="root $id"))
                else:
                    keep(pointer, option, draw(ANNOTATION_VALUES[option], label=option))

    def scoped(tokens: list[str]) -> bool:
        return any(_is_prefix(resource, tokens) for resource in resources)

    # Every $ref of the base inside an embedded resource is unsupported.
    scoped_base_refs = 0
    for pointer, tokens in objects:
        node = expected_root.resolve(pointer)
        if "$ref" in node.kw and scoped(tokens):
            del node.kw["$ref"]
            unsupported.add((pointer, "$ref"))
            scoped_base_refs += 1

    # Pass 2: $ref, at schema objects that hold none.
    bad_ref_kinds: list[str] = []
    ref_categories = [category for category in REF_CATEGORIES if category in categories]
    if ref_categories:
        for pointer, tokens in objects:
            if "$ref" in used[pointer] or not draw(st.booleans(), label=f"$ref at {pointer!r}"):
                continue
            category = draw(st.sampled_from(ref_categories), label="$ref category")
            if category == LOCAL_REF:
                target = draw(st.sampled_from([where for where, _, _ in positions]), label="$ref target")
                ref = "#" + quote(target, safe=draw(st.sampled_from(REF_SAFE_SETS)))
                inject(pointer, "$ref", ref)
                if scoped(tokens):
                    unsupported.add((pointer, "$ref"))  # relative to the resource, not the document
                else:
                    expected_root.resolve(pointer).kw["$ref"] = ref
            else:
                kind, ref = draw(bad_refs(positions, omitted), label="bad $ref")
                inject(pointer, "$ref", ref)
                unsupported.add((pointer, "$ref"))
                injected_unsupported = True
                bad_ref_kinds.append(kind)

    # The root $schema.
    dialect = "missing"
    if base_root.boolean is None:
        dialect = draw(st.sampled_from(("missing", "recognized", "unrecognized")), label="dialect")
        if dialect == "recognized":
            inject("", "$schema", draw(st.sampled_from(D2020_SPELLINGS)))
        elif dialect == "unrecognized":
            inject("", "$schema", draw(st.sampled_from(UNRECOGNIZED_DIALECTS)))
            unsupported.add(("", "$schema"))
            injected_unsupported = True

    return LoaderCase(
        document=doc_root.to_json(),  # type: ignore[arg-type]
        expected_root=expected_root,
        ignored=ignored,
        unsupported=unsupported,
        unsupported_injected=injected_unsupported,
        dialect=dialect,
        kept=kept,
        categories=categories,
        scoped_base_refs=scoped_base_refs,
        bad_ref_kinds=tuple(bad_ref_kinds),
    )


def _describe(case: LoaderCase) -> None:
    """Hypothesis events showing which cases the examples reached."""
    event(f"dialect: {case.dialect}")
    for category in case.categories:
        event(f"category enabled: {category}")
    event("fully supported" if not case.unsupported else "partially supported")
    if case.ignored:
        event("has ignored records")
    if case.kept:
        event("has kept injections")
    if case.scoped_base_refs:
        event("base $ref inside an embedded resource")
    for kind in sorted(set(case.bad_ref_kinds)):
        event(f"bad $ref: {kind}")


@given(loader_cases())
def test_loader_records_exactly_the_injected_constructs(case: LoaderCase) -> None:
    _describe(case)
    loaded = LOADER.load(case.document)

    # Records are sorted in code-point order (Python str order) without repeats.
    assert loaded.unsupported == sorted(case.unsupported), "unsupported records differ"
    assert loaded.ignored == sorted(case.ignored), "ignored records differ"
    assert loaded.fully_supported is (not case.unsupported_injected)
    assert loaded.dialect_assumed is (case.dialect != "recognized")
    for pointer, keyword in case.kept:
        assert keyword in loaded.resolve(pointer).kw, f"{keyword!r} at {pointer!r} was not retained"
    assert loaded.root == case.expected_root, (
        "the loaded model differs from the base with the kept injections added and the omitted constructs left out"
    )
    assert loaded == SchemaModel(case.expected_root, unsupported=sorted(case.unsupported))
