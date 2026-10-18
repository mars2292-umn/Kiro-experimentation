"""Soundness check and relaxation loop (Requirement 3).

After Constraint inference, and before the Schema_Emitter writes anything,
:func:`check_soundness` validates every non-flagged document of the covering
schema's segment and widens the schema until all of them validate. It edits
the Schema_Model in place, updates its Provenance_Records, records the 3.4
counts in the Inference_Report, and returns a :class:`SoundnessResult` with
the bookkeeping that Properties 7 and 9 check.

Entry point (for ``pipeline.infer``, task 16.2)
    ``check_soundness(model, documents, engine=..., validator=...,
    report=..., flagged=flagged_documents(store), segment=None,
    version=None, max_passes=config["soundness.max_passes"],
    workers=config["workers"], chunk_size=config["ingest.chunk_size"])``.
    ``documents`` is the run's DocumentStore (or a sequence of JSON values),
    ``flagged`` the Flagged_Documents (:func:`flagged_documents` derives them
    from an ObservationStore's FlagSet: flagged documents plus every document
    that holds a Flagged_Item), and ``segment`` the document indices the
    model covers (all documents by default; one Schema_Version's segment
    once change points exist, with ``version`` its index). Call it once per
    covering schema. The engine is the run's DecisionEngine; it records one
    DecisionEntry per resolved ``relaxation`` Decision_Point in its report.

Validation (3.2)
    The covering schema is the emitted layout of the model
    (``SchemaEmitter.layout``), compiled by the Validator with ``format``
    asserted. Every document of the segment that is not a Flagged_Document
    is validated, including documents that any sample of 29.8 left out:
    sampling only limits what super-linear Techniques look at, never what is
    validated. Documents are validated in fixed chunks of ``chunk_size``
    documents, in at most ``workers`` joblib processes; results are merged
    by document index, so nothing depends on the chunking or the worker
    count (27.5). A schema that does not compile raises ``CompileError``.

Failures (the hooks of ``decide.relax``)
    Each error of a failing document becomes an occurrence: a Failure target
    ``(nid, keyword, item)`` and the instance it rejected, located in the
    document. The keyword location resolves to the owner node and keyword;
    a ``falseSchema`` error at ``…/items`` or ``…/additionalProperties`` (in
    keyword position) is that keyword of the owner with the array or object
    holding the rejected element or member (jsonschema-rs reports a false
    ``items`` at the element but a false ``additionalProperties`` at the
    object), and elsewhere :data:`~jsinfer.decide.relax.FALSE_SCHEMA` at the ``false``
    node. ``propertyNames`` and ``contains`` errors are those keywords of the
    node that holds them, found through the evaluation path (or, when
    jsonschema-rs reports a location behind a ``$ref`` without the keyword,
    the nodes whose keyword subschema reaches the location by ``$ref`` hops
    and rejects the instance). A ``required`` error names the missing
    property as its item; at ``…/dependentRequired`` the item is each
    antecedent present in the object whose consequents include the missing
    property. ``anyOf`` and ``oneOf`` errors carry their per-branch errors
    (``Validator.errors(details=True)``), mapped the same way into
    ``Failure.branches`` when there is one entry per branch.

    Occurrences are grouped by target, one per (document, instance
    location). The groups of a pass, and of each removal round, are
    processed in target order only: the JSON Pointer of the target's node
    in the layout at the start of the pass (token by token), then keyword,
    then item, all in code-point order. This order is a function of the
    model, never of the document order, so the stage is order-insensitive
    (27.6; design Open Decisions, Soundness processing order). Nids are not
    used: they follow the order in which nodes were made (the analyzer's
    walk, then every relaxation candidate built), not the model. A group
    processed after the model changed in the same pass is re-derived from
    the failing documents against the current model, so each relaxation is
    scored against the model as relaxed so far, and skipped when its target
    no longer fails.

Relaxation passes (3.3, 3.5)
    One pass is one validation of every validated document. Passes 1 to
    ``max_passes − 1`` are each followed by relaxation; when pass
    ``max_passes`` still finds failures, the removal loop runs instead
    (3.7), so with ``max_passes = 1`` nothing is relaxed. For each group,
    :func:`~jsinfer.decide.relax.relaxations` lists the candidates; a
    candidate is kept when, validating the group's documents against the
    relaxed copy, no error remains at the failing target, at the targets the
    candidate's Changes turn it into (a ``oneOf`` becoming ``anyOf``, a
    ``const`` becoming ``enum``), or at a node the candidate created (other
    than a guard's ``allOf`` wrapper). Removal is always offered last and
    stands in when nothing else is kept. The ``relaxation`` Decision_Point
    ``DecisionPoint(RELAXATION, path)`` is resolved by the DecisionEngine:
    with ``decide.mdl`` enabled, every kept candidate is scored by an
    MDL_Scorer rooted at its relaxed copy (so ``$ref`` hops see edited
    ``$defs``), on the Failure node's Observations plus the failing values
    (below); otherwise the Fallback_Behavior takes the first candidate in the
    documented order. Every candidate only widens (decide.relax), so each
    document valid before a change stays valid after it.

    ``path`` is the logical Path of the Failure's node: properties add Key
    steps, ``additionalProperties`` and ``patternProperties`` add ``{*}``,
    ``items``, ``prefixItems``, and ``contains`` add ``[*]``, and
    combinators, conditionals, and ``$ref`` hops add nothing. A node reached
    several ways (a ``$defs`` entry) takes the Path with the fewest steps,
    then the first in Path order; an unreached node takes ``$``.

The Failure node's Observations
    The instances a node is evaluated on, over every validated document: a
    walk of the documents along the schema (``properties``,
    ``patternProperties``, ``additionalProperties``, ``propertyNames`` (the
    keys), ``prefixItems``, ``items``, ``contains``, ``allOf``, ``anyOf``,
    ``oneOf``, ``if``/``then``/``else``, and local ``$ref``; ``$defs`` only
    through references; ``not`` contents are skipped). An ``anyOf`` or
    ``oneOf`` branch, a ``contains`` subschema, ``then``, and ``else`` see
    only the instances they apply to: those the branch or ``contains``
    subschema accepts, and those ``if`` accepts or rejects. Each (document,
    instance location) counts once per node. The failing values are added
    when the walk does not reach them, and the instances are flattened into
    an ObservationStore of their own, whose root view the candidates are
    scored on.

Records (3.6)
    Each applied Change yields one RelaxationRecord: the fragments before
    and after, the triggering documents (the Failure's), and, under 3.3, the
    relaxation decision's Resolution with every candidate and its score.

    - A Change that removes Constraints (``after`` is REMOVED, or a ``false``
      subschema replaced by ``true``, which the relaxation table calls its
      removal) goes to the report as one :class:`RemovedConstraint`. Its
      pointer is the removed Constraint's emitted pointer when it removes
      one Constraint (``…/required/<i>`` indexes the emitted list), else the
      pointer of the keyword or, for several keywords, of the schema object,
      all taken in the layout just before the change. The removed
      Constraints' Provenance_Records leave the model.
    - Any other Change appends its record to the Provenance_Record of each
      Constraint it leaves (its ``after_ids``); a renamed Constraint (a
      ``oneOf`` branch now an ``anyOf`` branch, ``const`` now ``enum``) takes
      its old record along, pairing gone and new ids in order among those
      with the same kind of item. Under 3.3 a Constraint without a record (a
      new branch, a new ``properties``, the owner's new ``allOf`` of a
      conversion beside ``anyOf``, a replaced ``false`` subschema's
      keywords, or a Constraint that had none) gets one from the relaxation
      decision (``Decision.provenance``); under 3.7 only existing records are
      extended.
    - Under 3.3, the Constraints inside a subschema the change created (a
      Naive_Merge subschema for new properties, extra elements, a new
      branch, or a replaced ``false`` subschema) are listed by no Change;
      each gets the relaxation decision's record too, without a change
      record, since nothing of theirs was changed (26.1). Its Support stays
      ``(0, 0)`` until provenance finalization.
    - Provenance_Records of nodes that a change removed with their parent
      keyword are dropped.

    At the end, the Support of every Constraint whose record received a
    change is recomputed over the final model and the validated documents
    (:func:`count_instances`, below). Confidences are left as they are;
    provenance finalization (``pipeline.infer.finalize_provenance``, task
    17.1) recomputes every Support with the same rule and refreshes the
    Laplace defaults.

Support (3.6, 26.2; :func:`count_instances`)
    ``total`` counts the instances at the Constraint's node (the walk above)
    that its keyword applies to (numbers for numeric bounds; strings for
    string bounds, ``pattern``, ``format``, and ``x-latent-type``; arrays
    for array keywords; objects for object keywords, ``required`` entries,
    and ``dependentRequired`` entries; every instance otherwise), and
    ``consistent`` those it accepts. In a valid document, an instance that
    reaches a node satisfies its assertions, so ``consistent = total``
    except for an ``anyOf``/``oneOf`` branch Constraint, whose consistent
    count is the instances that branch accepts. A ``$defs`` entry is reached
    only through references, so its counts cover every referencing site.

    Flagged_Items (17.4). :func:`count_instances` can also count documents
    that hold Flagged_Items (``flagged_items``: per document, the instance
    locations of its Flagged_Items, :func:`flagged_item_locations`). The
    walk then never enters a Flagged_Item, and a Map key counts only when
    its member is not flagged (a flagged member's key belongs to the
    member's Observation). Such documents were not validated, so their
    verdicts are taken *modulo* their Flagged_Items: a validation error is
    disregarded when it lies inside a Flagged_Item, or when its keyword's
    verdict depends on the instance's descendants (anything but ``type``,
    ``required``, the length, size, and numeric bounds, ``pattern``,
    ``format``, and ``false`` subschemas) and a Flagged_Item lies below its
    instance; an ``anyOf`` error is disregarded when some branch passes
    modulo the flags, and a ``oneOf`` error when exactly one does. Branch,
    ``contains``, and ``if`` checks of the walk use this verdict, and an
    instance of such a document counts as consistent with a Constraint
    unless an error that is not disregarded names that Constraint at that
    instance. With no Flagged_Item every document counted is valid and the
    rule above applies unchanged.

Counts (3.4)
    ``validated``: documents of the segment that are not Flagged_Documents;
    ``flagged_documents``: the Flagged_Documents of the segment;
    ``failed_first_pass``: documents with an error in pass 1; ``passes``:
    every validation performed, removal-loop validations included (0 when
    there is nothing to validate); ``pass_limit_reached``: whether pass
    ``max_passes`` still failed. ``relaxations`` counts the records written
    under 3.3, plus the non-removal guard records of the removal loop (a
    ``oneOf`` converted so that a removal inside a branch stays monotone);
    ``removals`` counts the removal records of the removal loop. So the
    change records of :attr:`SoundnessResult.changes` number exactly
    ``relaxations + removals``, and ``summary.removed`` holds one entry per
    removal record of either loop.

Removal loop (3.7)
    While documents fail, each failing group's Constraint is removed, in
    target order, with :func:`~jsinfer.decide.relax.removal` (its
    monotonicity guards included), then every document is validated again. Each round removes at least one
    keyword or ``false`` subschema, so the loop ends, with every validated
    document valid.

Determinism
    Groups follow target order (above); the occurrences of a group, its
    Failure's values, and records follow document index; candidates follow
    canonical pre-order and code-point order; no set is iterated.
    Validation results do not depend on chunking or workers; MDL scores
    depend only on the multiset of instances (19.6), and candidates on the
    multiset of failing values. So a permutation of the documents changes
    only the document indices that records carry (3.6).
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field, replace
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias
from urllib.parse import quote, unquote

import numpy as np
from joblib import Parallel, delayed

from jsinfer.config.resources import RESOURCE_PARAMETERS, WORKERS
from jsinfer.core.determinism import codepoint_key, sorted_codepoint
from jsinfer.core.jsonvalue import JSON, JType, jtype_of
from jsinfer.core.observations import ObservationStore, build_store
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, ROOT, Path, fewest_steps_key, key_step
from jsinfer.core.provenance import REMOVED, RelaxationRecord, Resolution, Support
from jsinfer.core.report import InferenceReport, RemovedConstraint, SoundnessSummary
from jsinfer.core.schema_model import (
    SUBSCHEMA_KEYWORDS,
    ConstraintId,
    SchemaModel,
    SchemaNode,
    join_pointer,
    split_pointer,
)
from jsinfer.core.views import View
from jsinfer.core.workers import single_threaded
from jsinfer.decide.engine import MDL_SELECTION, Decision, DecisionEngine, DecisionPoint
from jsinfer.decide.relax import (
    FALSE_SCHEMA,
    RELAXABLE_KEYWORDS,
    RELAXATION,
    Change,
    Failure,
    Relaxation,
    relaxations,
    removal,
)
from jsinfer.io.emit import EmitError, PointerMap, SchemaEmitter
from jsinfer.io.ingest import DocumentStore, parse_document
from jsinfer.io.validator import CompileError, Compiled, ErrorInfo, Validator
from jsinfer.mdl.scorer import MDLScorer

__all__ = [
    "CHUNK_SIZE",
    "MAX_PASSES",
    "AppliedChange",
    "InstanceCounts",
    "SoundnessError",
    "SoundnessResult",
    "check_soundness",
    "count_instances",
    "flagged_documents",
    "flagged_item_locations",
]

# The Resource_Parameters this stage reads.
MAX_PASSES: Final = "soundness.max_passes"
CHUNK_SIZE: Final = "ingest.chunk_size"

Phase: TypeAlias = Literal["relaxation", "removal"]
_Target: TypeAlias = tuple[int, str, "str | None"]
_Key: TypeAlias = tuple[str, "str | None"]  # a target without its nid: (keyword, item)
_Location: TypeAlias = tuple[Any, ...]
# One visit of the walk: document, node nid, instance location, instance, and the branch verdicts of the descent.
_Visit: TypeAlias = tuple[int, int, _Location, Any, dict[tuple[str, int], bool]]

_COMBINATORS: Final = ("anyOf", "oneOf")

# The base URI under which a model's document is embedded to validate one of its subschemas.
_EMBED_URI: Final = "urn:jsinfer:soundness"
# Characters a $ref fragment may hold unescaped besides letters, digits, and "_.-~" (as decide.relax writes them).
_FRAGMENT_SAFE: Final = "/!$&'()*+,;=:@"

_NUMBERS: Final = frozenset({JType.INTEGER, JType.NUMBER})
_STRINGS: Final = frozenset({JType.STRING})
_ARRAYS: Final = frozenset({JType.ARRAY})
_OBJECTS: Final = frozenset({JType.OBJECT})

# The JSON types each Constraint keyword applies to (for Support totals); absent keywords apply to every instance.
_KEYWORDS_BY_TYPES: Final = (
    (("exclusiveMaximum", "exclusiveMinimum", "maximum", "minimum", "multipleOf"), _NUMBERS),
    (("format", "maxLength", "minLength", "pattern", "x-latent-type"), _STRINGS),
    (("contains", "items", "maxItems", "minItems", "prefixItems", "uniqueItems"), _ARRAYS),
    (
        (
            "additionalProperties",
            "dependentRequired",
            "maxProperties",
            "minProperties",
            "patternProperties",
            "properties",
            "propertyNames",
            "required",
        ),
        _OBJECTS,
    ),
)
_APPLIES_TO: Final[Mapping[str, frozenset[JType]]] = MappingProxyType(
    {keyword: types for keywords, types in _KEYWORDS_BY_TYPES for keyword in keywords}
)

# Error kinds whose verdict depends only on the instance itself (its type, keys, size, or scalar value), never
# on the values below it; an error of any other kind may stem from a Flagged_Item below its instance.
_LOCAL_KINDS: Final = frozenset(
    {
        "exclusiveMaximum",
        "exclusiveMinimum",
        "falseSchema",
        "format",
        "maxItems",
        "maxLength",
        "maxProperties",
        "maximum",
        "minItems",
        "minLength",
        "minProperties",
        "minimum",
        "multipleOf",
        "pattern",
        "required",
        "type",
    }
)

_EMITTER: Final = SchemaEmitter()


class SoundnessError(RuntimeError):
    """A validation error the soundness loop cannot map to a Constraint of the model (an internal inconsistency)."""


# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------


def flagged_documents(store: ObservationStore) -> np.ndarray:
    """The Flagged_Documents of a store: flagged documents and every document holding a Flagged_Item (ascending)."""
    if not isinstance(store, ObservationStore):
        raise TypeError(f"store must be an ObservationStore, got {type(store).__name__}")
    flags = store.flags
    parts = [np.asarray(flags.documents, dtype=np.int64)]
    for column, rows in flags.items():
        parts.append(np.asarray(store.column(column).doc[rows], dtype=np.int64))
    return np.unique(np.concatenate(parts))


def flagged_item_locations(store: ObservationStore) -> dict[int, tuple[_Location, ...]]:
    """The instance locations of a store's Flagged_Items, by document (see Support in the module docstring).

    A location is the tuple of the item's reference tokens from the document
    root (member names, and array indices as decimal strings); ``()`` is a
    flagged root row. Documents are ascending, and each document's locations
    are distinct and sorted. Flagged documents themselves are not listed.
    """
    if not isinstance(store, ObservationStore):
        raise TypeError(f"store must be an ObservationStore, got {type(store).__name__}")
    found: dict[int, dict[_Location, None]] = {}
    for column, rows in store.flags.items():
        for row in rows.tolist():
            tokens: list[str] = []
            shape, current = column, row
            while shape:
                cells = store.columns[shape]
                if shape[-1] == MAPVAL_STEP:
                    tokens.append(str(store.keys[int(cells.key[current])]))
                else:
                    tokens.append(str(int(cells.pos[current])))
                current = int(cells.parent[current])
                shape = shape[:-1]
            doc = int(store.columns[ROOT].doc[current])
            found.setdefault(doc, {})[tuple(reversed(tokens))] = None
    return {doc: tuple(sorted(found[doc], key=_location_key)) for doc in sorted(found)}


def _location_key(location: _Location) -> tuple[Any, ...]:
    return tuple(codepoint_key(token) for token in location)


def _under(masked: Sequence[_Location], location: _Location) -> tuple[_Location, ...]:
    """The flagged locations strictly below ``location``, relative to it."""
    if not masked:
        return ()
    depth = len(location)
    return tuple(m[depth:] for m in masked if len(m) > depth and m[:depth] == location)


def _passes(errors: Sequence[ErrorInfo], masked: Sequence[_Location]) -> bool:
    """Whether an instance with these (detailed) errors is valid modulo the Flagged_Items at ``masked``."""
    return all(_disregarded(error, masked) for error in errors)


def _disregarded(error: ErrorInfo, masked: Sequence[_Location]) -> bool:
    """Whether a validation error is explained by the Flagged_Items at ``masked`` (see Support in the docstring)."""
    if not masked:
        return False
    where = tuple(split_pointer(error.instance_location))
    if any(where[: len(m)] == m for m in masked):
        return True  # inside a Flagged_Item
    if error.keyword in _COMBINATORS and error.context:
        passing = sum(1 for branch in error.context if _passes(branch, masked))
        return passing >= 1 if error.keyword == "anyOf" else passing == 1
    if error.keyword in _LOCAL_KINDS:
        return False
    return any(len(m) > len(where) and m[: len(where)] == where for m in masked)


class _Corpus:
    """The documents by index: a DocumentStore (parsed on demand) or a sequence of JSON values."""

    __slots__ = ("_store", "_values")

    def __init__(self, documents: DocumentStore | Sequence[JSON]) -> None:
        if isinstance(documents, DocumentStore):
            self._store: DocumentStore | None = documents
            self._values: Sequence[JSON] | None = None
        elif isinstance(documents, (str, bytes, dict)) or not isinstance(documents, Sequence):
            raise TypeError(
                f"documents must be a DocumentStore or a sequence of JSON values, got {type(documents).__name__}"
            )
        else:
            self._store = None
            self._values = documents

    def __len__(self) -> int:
        return len(self._store) if self._store is not None else len(self._values or ())

    @property
    def parse(self) -> bool:
        """Whether payloads are canonical bytes that workers parse."""
        return self._store is not None

    def payload(self, index: int) -> Any:
        if self._store is not None:
            return self._store[index]
        assert self._values is not None
        return self._values[index]

    def value(self, index: int) -> JSON:
        if self._store is not None:
            return self._store.value(index)
        assert self._values is not None
        return self._values[index]


def _indices(values: Iterable[int] | np.ndarray, what: str, n_docs: int) -> list[int]:
    """Distinct document indices in ascending order, checked against the corpus size."""
    if isinstance(values, (str, bytes)) or not isinstance(values, Iterable):
        raise TypeError(f"{what} must be document indices, got {type(values).__name__}")
    found: dict[int, None] = {}
    for value in values.tolist() if isinstance(values, np.ndarray) else values:
        if isinstance(value, (bool, np.bool_)) or not isinstance(value, (int, np.integer)):
            raise TypeError(f"{what} must be integers, got {value!r}")
        index = int(value)
        if not 0 <= index < n_docs:
            raise ValueError(f"{what}: document index {index} is out of range for {n_docs} documents")
        found[index] = None
    return sorted(found)


def _setting(name: str, value: Any) -> int:
    """A Resource_Parameter value, or its default when ``value`` is None; raises ``ValueError`` when invalid."""
    spec = RESOURCE_PARAMETERS[name]
    return int(spec.normalize(spec.default_value() if value is None else value))


# ---------------------------------------------------------------------------
# Parallel validation
# ---------------------------------------------------------------------------

_chunk_validator: Validator | None = None


def _validate_chunk(compiled: Compiled, items: list[tuple[int, Any]], parse: bool) -> list[tuple[int, list[ErrorInfo]]]:
    """The detailed errors of each failing document of one chunk (runs in a joblib worker)."""
    global _chunk_validator
    if _chunk_validator is None:
        _chunk_validator = Validator()
    validator = _chunk_validator
    failing: list[tuple[int, list[ErrorInfo]]] = []
    for index, payload in items:
        value = parse_document(payload)[0] if parse else payload
        if not validator.is_valid(compiled, value):
            failing.append((index, validator.errors(compiled, value, details=True)))
    return failing


# ---------------------------------------------------------------------------
# Schema helpers
# ---------------------------------------------------------------------------


def _keyword_indices(tokens: Sequence[Any]) -> list[int]:
    """The positions of keywords in a schema location or evaluation path (entries of list and map keywords skipped)."""
    found: list[int] = []
    i = 0
    while i < len(tokens):
        found.append(i)
        shape = SUBSCHEMA_KEYWORDS.get(tokens[i]) if isinstance(tokens[i], str) else None
        i += 2 if shape in ("list", "map") else 1
    return found


def _ref_target(model: SchemaModel, node: SchemaNode) -> SchemaNode | None:
    """The node a local JSON Pointer ``$ref`` of ``node`` names, or None."""
    if node.boolean is not None:
        return None
    ref = node.kw.get("$ref")
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None
    pointer = unquote(ref[1:])
    if pointer and not pointer.startswith("/"):
        return None
    try:
        return model.resolve(pointer)
    except (KeyError, ValueError):
        return None


def _instance_at(value: JSON, tokens: Sequence[str]) -> JSON:
    """The instance at a JSON Pointer (as tokens) into a document."""
    current: Any = value
    for token in tokens:
        if isinstance(current, dict):
            current = current[token]
        elif isinstance(current, list):
            current = current[int(token)]
        else:
            raise KeyError(f"instance location token {token!r} does not lead into a container")
    return current


def _norm(nid: int, keyword: str, item: Any) -> _Target:
    """A target with the branch index of a combinator dropped (the whole ``anyOf``/``oneOf``)."""
    if keyword in _COMBINATORS or not isinstance(item, str):
        return (nid, keyword, None)
    return (nid, keyword, item)


def _target_key(target: _Target, node_pointer: Mapping[int, str]) -> tuple[tuple[str, ...], str, str]:
    """The processing order of a target: its node's JSON Pointer token by token, keyword, item (code-point order).

    The pointer is the node's position in the model, which the emitted layout
    keeps, so the order is a function of the model alone; nids are not (they
    follow the order in which nodes were made).
    """
    tokens = tuple(codepoint_key(token) for token in split_pointer(node_pointer[target[0]]))
    return (tokens, codepoint_key(target[1]), codepoint_key(target[2] or ""))


def _renames(gone: Sequence[ConstraintId], new: Sequence[ConstraintId]) -> dict[ConstraintId, ConstraintId]:
    """Pair each new id with the gone id it renames: in order, among ids with the same kind of item.

    So ``oneOf`` branch i becomes ``anyOf`` branch i and ``const`` becomes
    ``enum``, while a conversion beside ``anyOf`` pairs the branches with the
    wrapper's and leaves the owner's new ``allOf`` without a predecessor.
    """
    pairs: dict[ConstraintId, ConstraintId] = {}
    for kind in (type(None), int, str):
        pairs.update(
            zip(
                [cid for cid in new if type(cid.item) is kind],
                [cid for cid in gone if type(cid.item) is kind],
            )
        )
    return pairs


def _edges(model: SchemaModel, node: SchemaNode) -> list[tuple[str, Any, SchemaNode]]:
    """How instances flow from ``node`` to its subschemas (see the module docstring), in canonical order."""
    if node.boolean is not None:
        return []
    kw = node.kw
    out: list[tuple[str, Any, SchemaNode]] = []
    properties = kw.get("properties")
    names: dict[str, Any] = properties if isinstance(properties, dict) else {}
    for name in sorted_codepoint(names):
        out.append(("key", name, names[name]))
    patterns = kw.get("patternProperties")
    pattern_names = sorted_codepoint(patterns) if isinstance(patterns, dict) else []
    for pattern in pattern_names:
        out.append(("pattern", pattern, patterns[pattern]))
    if isinstance(kw.get("additionalProperties"), SchemaNode):
        out.append(("additional", (names, pattern_names), kw["additionalProperties"]))
    if isinstance(kw.get("propertyNames"), SchemaNode):
        out.append(("names", None, kw["propertyNames"]))
    prefix = kw.get("prefixItems")
    prefix = prefix if isinstance(prefix, list) else []
    for index, child in enumerate(prefix):
        out.append(("prefix", index, child))
    if isinstance(kw.get("items"), SchemaNode):
        out.append(("items", len(prefix), kw["items"]))
    if isinstance(kw.get("contains"), SchemaNode):
        out.append(("contains", None, kw["contains"]))
    for child in kw.get("allOf") or ():
        out.append(("same", None, child))
    for keyword in _COMBINATORS:
        for index, child in enumerate(kw.get(keyword) or ()):
            out.append(("branch", (keyword, index), child))
    condition = kw.get("if")
    if isinstance(condition, SchemaNode):
        out.append(("same", None, condition))
        if isinstance(kw.get("then"), SchemaNode):
            out.append(("then", condition, kw["then"]))
        if isinstance(kw.get("else"), SchemaNode):
            out.append(("else", condition, kw["else"]))
    target = _ref_target(model, node)
    if target is not None:
        out.append(("same", None, target))
    return out


def _node_paths(model: SchemaModel) -> dict[int, Path]:
    """The logical Path of every reachable node: the fewest steps, then Path order (see the module docstring)."""
    best: dict[int, Path] = {}
    stack: list[tuple[SchemaNode, Path]] = [(model.root, ROOT)]
    while stack:
        node, path = stack.pop()
        known = best.get(node.nid)
        if known is not None and fewest_steps_key(known) <= fewest_steps_key(path):
            continue
        best[node.nid] = path
        for keyword, entry, child in node.children():
            if keyword == "$defs":
                continue
            if keyword == "properties":
                stack.append((child, (*path, key_step(str(entry)))))
            elif keyword in ("additionalProperties", "patternProperties"):
                stack.append((child, (*path, MAPVAL_STEP)))
            elif keyword in ("contains", "items", "prefixItems"):
                stack.append((child, (*path, ELEM_STEP)))
            else:
                stack.append((child, path))
        target = _ref_target(model, node)
        if target is not None:
            stack.append((target, path))
    return best


# ---------------------------------------------------------------------------
# Occurrences and groups
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class _Occurrence:
    """One instance that a Failure target rejected, at ``location`` in document ``doc``."""

    target: _Target
    doc: int
    location: _Location
    value: JSON
    branches: tuple[tuple[Failure, ...], ...] = ()

    def failure(self) -> Failure:
        nid, keyword, item = self.target
        return Failure(nid, keyword, item, (self.value,), (self.doc,), self.branches)


def _merge(target: _Target, occurrences: Sequence[_Occurrence]) -> Failure:
    """One Failure for a target from its occurrences, in their order."""
    width = max((len(occurrence.branches) for occurrence in occurrences), default=0)
    branches = tuple(
        tuple(
            failure
            for occurrence in occurrences
            if index < len(occurrence.branches)
            for failure in occurrence.branches[index]
        )
        for index in range(width)
    )
    nid, keyword, item = target
    return Failure(
        nid,
        keyword,
        item,
        tuple(occurrence.value for occurrence in occurrences),
        tuple(occurrence.doc for occurrence in occurrences),
        branches,
    )


@dataclass(frozen=True, eq=False, slots=True)
class _Group:
    """The occurrences of one target in one validation pass."""

    target: _Target
    occurrences: tuple[_Occurrence, ...]

    @property
    def docs(self) -> list[int]:
        return sorted(dict.fromkeys(occurrence.doc for occurrence in self.occurrences))


def _groups(
    found: dict[_Target, dict[tuple[int, _Location], _Occurrence]], node_pointer: Mapping[int, str]
) -> list[_Group]:
    """The groups of one pass in target order (:func:`_target_key`), never in document order (27.6)."""
    groups = [_Group(target, tuple(occurrences.values())) for target, occurrences in found.items()]
    groups.sort(key=lambda group: _target_key(group.target, node_pointer))
    return groups


# ---------------------------------------------------------------------------
# Snapshots: one state of the model
# ---------------------------------------------------------------------------


class _Snapshot:
    """A model state: its emitted document and pointers, its compiled schema, and checks against its subschemas."""

    __slots__ = ("_checks", "_paths", "compiled", "doc", "model", "node_pointer", "nodes", "pointers", "validator")

    def __init__(self, model: SchemaModel, validator: Validator, name: str) -> None:
        self.model = model
        self.validator = validator
        self.doc, self.pointers = _EMITTER.layout(model)
        self.compiled = validator.compile(self.doc, name=name)
        self.nodes: dict[int, SchemaNode] = {}
        self.node_pointer: dict[int, str] = {}
        for pointer, node in model.walk():
            self.nodes[node.nid] = node
            self.node_pointer[node.nid] = pointer
        self._paths: dict[int, Path] | None = None
        self._checks: dict[int, Compiled] = {}

    # -- nodes ----------------------------------------------------------------

    def path_of(self, nid: int) -> Path:
        if self._paths is None:
            self._paths = _node_paths(self.model)
        return self._paths.get(nid, ROOT)

    def accepts(self, node: SchemaNode, value: JSON, masked: Sequence[_Location] = ()) -> bool:
        """Whether ``value`` validates against the subschema ``node`` of the model (its ``$ref`` hops resolved).

        ``masked`` holds the locations, relative to ``value``, of Flagged_Items
        inside it; the verdict is then taken modulo them (see Support in the
        module docstring).
        """
        if node.boolean is not None:
            return node.boolean
        compiled = self._checks.get(node.nid)
        if compiled is None:
            pointer = self.node_pointer[node.nid]
            if pointer == "":
                schema: Any = self.doc
            else:
                embedded = {**self.doc, "$id": _EMBED_URI} if isinstance(self.doc, dict) else self.doc
                schema = {"$defs": {"model": embedded}, "$ref": f"{_EMBED_URI}#{quote(pointer, safe=_FRAGMENT_SAFE)}"}
            compiled = self._checks[node.nid] = self.validator.compile(schema, name=f"the subschema at {pointer!r}")
        if self.validator.is_valid(compiled, value):
            return True
        return bool(masked) and _passes(self.validator.errors(compiled, value, details=True), masked)

    def rejections(self, doc: int, value: JSON, masked: Sequence[_Location]) -> dict[tuple[int, _Location], list[_Key]]:
        """The Constraints a document rejects modulo its Flagged_Items: (nid, location) → (keyword, item) targets."""
        found: dict[tuple[int, _Location], dict[_Key, None]] = {}
        if self.validator.is_valid(self.compiled, value):
            return {}
        for error in self.validator.errors(self.compiled, value, details=True):
            if _disregarded(error, masked):
                continue
            for occurrence in self.occurrences(doc, value, error, branches=False):
                nid, keyword, item = _norm(*occurrence.target)
                found.setdefault((nid, occurrence.location), {})[(keyword, item)] = None
        return {spot: list(targets) for spot, targets in found.items()}

    def _node(self, tokens: Sequence[str], error: ErrorInfo) -> SchemaNode:
        try:
            return self.model.resolve(join_pointer("", *tokens))
        except (KeyError, ValueError):
            raise SoundnessError(
                f"the {error.keyword!r} error at {error.keyword_location!r} names no subschema of the model"
            ) from None

    # -- errors to occurrences -------------------------------------------------

    def occurrences(self, doc: int, value: JSON, error: ErrorInfo, *, branches: bool = True) -> list[_Occurrence]:
        """The occurrences of one detailed error of document ``doc`` (see Failures in the module docstring)."""
        kind = error.keyword
        tokens = split_pointer(error.keyword_location)
        where = tuple(split_pointer(error.instance_location))
        last = len(tokens) - 1
        keyword_last = bool(tokens) and last in _keyword_indices(tokens)
        if kind == "falseSchema":
            if keyword_last and tokens[-1] in ("additionalProperties", "items"):
                # jsonschema-rs reports a false items at the rejected element, a false
                # additionalProperties at the object itself.
                parent = where if tokens[-1] == "additionalProperties" else where[:-1]
                return [self._occurrence(tokens[:-1], tokens[-1], None, doc, value, parent, error)]
            return [self._occurrence(tokens, FALSE_SCHEMA, None, doc, value, where, error)]
        if kind in ("contains", "propertyNames"):
            instance = _instance_at(value, where)
            return [
                self._occurrence(owner, kind, None, doc, value, where, error)
                for owner in self._owners(kind, error, instance)
            ]
        if not keyword_last:
            raise SoundnessError(f"the {kind!r} error at {error.keyword_location!r} does not end at a keyword")
        keyword = tokens[-1]
        if kind == "required" and keyword == "dependentRequired":
            return self._antecedents(tokens[:-1], doc, value, where, error)
        item = None
        if keyword == "required":
            if error.property is None:
                raise SoundnessError(f"the required error at {error.keyword_location!r} names no missing property")
            item = error.property
        inner: tuple[tuple[Failure, ...], ...] = ()
        if keyword in _COMBINATORS and branches and error.context:
            listed = self._node(tokens[:-1], error).kw.get(keyword)
            if isinstance(listed, list) and len(listed) == len(error.context):
                try:
                    inner = tuple(
                        tuple(
                            occurrence.failure()
                            for branch_error in branch_errors
                            for occurrence in self.occurrences(doc, value, branch_error)
                        )
                        for branch_errors in error.context
                    )
                except SoundnessError:
                    inner = ()  # a branch error names no Constraint: offer the combinator's own candidates only
        return [self._occurrence(tokens[:-1], keyword, item, doc, value, where, error, inner)]

    def _occurrence(
        self,
        owner: Sequence[str],
        keyword: str,
        item: str | None,
        doc: int,
        value: JSON,
        where: _Location,
        error: ErrorInfo,
        branches: tuple[tuple[Failure, ...], ...] = (),
    ) -> _Occurrence:
        if keyword not in RELAXABLE_KEYWORDS:
            raise SoundnessError(
                f"the {error.keyword!r} error at {error.keyword_location!r} names {keyword!r}, "
                "which the relaxation table has no row for"
            )
        node = self._node(owner, error)
        return _Occurrence((node.nid, keyword, item), doc, where, _instance_at(value, where), branches)

    def _antecedents(
        self, owner: Sequence[str], doc: int, value: JSON, where: _Location, error: ErrorInfo
    ) -> list[_Occurrence]:
        """A ``dependentRequired`` error: one occurrence per present antecedent whose consequent is missing."""
        node = self._node(owner, error)
        entries = node.kw.get("dependentRequired")
        instance = _instance_at(value, where)
        missing = error.property
        if not isinstance(entries, dict) or not isinstance(instance, dict) or missing is None:
            raise SoundnessError(f"cannot read the dependentRequired error at {error.keyword_location!r}")
        found = [
            name
            for name in sorted_codepoint(entries)
            if name in instance and missing not in instance and missing in (entries[name] or ())
        ]
        if not found:
            raise SoundnessError(
                f"no dependentRequired entry at {error.keyword_location!r} explains the missing {missing!r}"
            )
        return [_Occurrence((node.nid, "dependentRequired", name), doc, where, instance) for name in found]

    def _owners(self, kind: str, error: ErrorInfo, instance: JSON) -> list[list[str]]:
        """The pointers of the nodes whose ``propertyNames``/``contains`` reported ``error``."""
        path = error.evaluation_path
        spots = [i for i in _keyword_indices(path) if path[i] == kind]
        if spots:
            located = self.validator.locate(self.compiled, path[: spots[-1] + 1])
            if located is not None:
                return [split_pointer(located)[:-1]]
        tokens = split_pointer(error.keyword_location)
        spots = [i for i in _keyword_indices(tokens) if tokens[i] == kind]
        if spots:
            return [tokens[: spots[-1]]]
        # The keyword's subschema is a $ref and the location lies behind it.
        found: list[list[str]] = []
        for nid, node in self.nodes.items():
            sub = None if node.boolean is not None else node.kw.get(kind)
            if isinstance(sub, SchemaNode) and self._reaches(sub, error.keyword_location):
                if self._rejects(kind, sub, instance):
                    found.append(split_pointer(self.node_pointer[nid]))
        if not found:
            raise SoundnessError(f"no node holding {kind!r} explains the error at {error.keyword_location!r}")
        return found

    def _reaches(self, sub: SchemaNode, location: str) -> bool:
        node: SchemaNode | None = sub
        for _ in range(len(self.nodes) + 1):
            if node is None:
                return False
            pointer = self.node_pointer.get(node.nid)
            if pointer is not None and (location == pointer or location.startswith(pointer + "/")):
                return True
            node = _ref_target(self.model, node)
        return False

    def _rejects(self, kind: str, sub: SchemaNode, instance: JSON) -> bool:
        if kind == "propertyNames":
            return isinstance(instance, dict) and any(not self.accepts(sub, key) for key in instance)
        return isinstance(instance, list) and not any(self.accepts(sub, element) for element in instance)


# ---------------------------------------------------------------------------
# Instances at nodes
# ---------------------------------------------------------------------------


class _Walker:
    """Walks documents along the schema to the instances that target nodes are evaluated on (module docstring).

    ``masked`` maps a document to the locations of its Flagged_Items
    (:func:`flagged_item_locations`): the walk does not enter them, and its
    branch, ``contains``, and ``if`` checks are taken modulo them.
    """

    __slots__ = ("_edges", "_leads", "_masked", "_snapshot", "_targets")

    def __init__(
        self, snapshot: _Snapshot, targets: Iterable[int], masked: Mapping[int, Sequence[_Location]] | None = None
    ) -> None:
        self._snapshot = snapshot
        self._targets = dict.fromkeys(targets)
        self._masked = masked or {}
        self._edges = {nid: _edges(snapshot.model, node) for nid, node in snapshot.nodes.items()}
        reverse: dict[int, list[int]] = {}
        for nid, out in self._edges.items():
            for _, _, child in out:
                reverse.setdefault(child.nid, []).append(nid)
        leads = dict.fromkeys(self._targets)
        stack = list(leads)
        while stack:
            for parent in reverse.get(stack.pop(), ()):
                if parent not in leads:
                    leads[parent] = None
                    stack.append(parent)
        self._leads = leads

    def masked_under(self, doc: int, location: _Location) -> tuple[_Location, ...]:
        """The Flagged_Items of document ``doc`` strictly below ``location``, relative to it."""
        return _under(self._masked.get(doc, ()), location)

    def collect(self, documents: Iterable[tuple[int, JSON]]) -> dict[int, dict[tuple[int, _Location], JSON]]:
        """Per target nid, the instances keyed by (document, instance location), in walk order."""
        found: dict[int, dict[tuple[int, _Location], JSON]] = {nid: {} for nid in self._targets}
        for doc, nid, location, instance, _ in self.visits(documents):
            found[nid][(doc, location)] = instance
        return found

    def visits(self, documents: Iterable[tuple[int, JSON]]) -> Iterator[_Visit]:
        """Each visit of a target node, once per (document, node, instance location), in walk order.

        A visit carries the verdicts of the node's ``anyOf``/``oneOf``
        branches that the descent computed, keyed by (keyword, index).
        """
        root = self._snapshot.model.root
        if root.nid not in self._leads:
            return
        for doc, value in documents:
            masked = self._masked.get(doc, ())
            if () in masked:
                continue  # the whole document is a Flagged_Item
            seen: set[tuple[int, _Location]] = set()  # membership only
            stack: list[tuple[SchemaNode, Any, _Location]] = [(root, value, ())]
            while stack:
                node, instance, location = stack.pop()
                key = (node.nid, location)
                if key in seen:
                    continue
                seen.add(key)
                verdicts: dict[tuple[str, int], bool] = {}
                for kind, data, child in self._edges.get(node.nid, ()):
                    if child.nid in self._leads:
                        self._descend(kind, data, child, instance, location, stack, masked, verdicts)
                if node.nid in self._targets:
                    yield doc, node.nid, location, instance, verdicts

    def _descend(
        self,
        kind: str,
        data: Any,
        child: SchemaNode,
        instance: Any,
        location: _Location,
        stack: list[Any],
        masked: Sequence[_Location],
        verdicts: dict[tuple[str, int], bool],
    ) -> None:
        snapshot = self._snapshot
        if kind == "same":
            stack.append((child, instance, location))
        elif kind == "branch":
            accepted = verdicts[data] = snapshot.accepts(child, instance, _under(masked, location))
            if accepted:
                stack.append((child, instance, location))
        elif kind in ("then", "else"):
            if snapshot.accepts(data, instance, _under(masked, location)) == (kind == "then"):
                stack.append((child, instance, location))
        elif isinstance(instance, dict):
            validator = snapshot.validator
            if kind == "key":
                if data in instance:
                    self._push(stack, child, instance[data], (*location, data), masked)
            elif kind == "pattern":
                for key, member in instance.items():
                    if validator.matches_pattern(data, key):
                        self._push(stack, child, member, (*location, key), masked)
            elif kind == "additional":
                names, patterns = data
                for key, member in instance.items():
                    if key not in names and not any(validator.matches_pattern(p, key) for p in patterns):
                        self._push(stack, child, member, (*location, key), masked)
            elif kind == "names":
                for key in instance:
                    if not masked or (*location, key) not in masked:  # a flagged member's key is flagged with it
                        stack.append((child, key, (*location, (key,))))
        elif isinstance(instance, list):
            if kind == "prefix":
                if data < len(instance):
                    self._push(stack, child, instance[data], (*location, str(data)), masked)
            elif kind == "items":
                for index in range(data, len(instance)):
                    self._push(stack, child, instance[index], (*location, str(index)), masked)
            elif kind == "contains":
                for index, element in enumerate(instance):
                    spot = (*location, str(index))
                    if masked and spot in masked:
                        continue
                    if snapshot.accepts(child, element, _under(masked, spot)):
                        stack.append((child, element, spot))

    @staticmethod
    def _push(
        stack: list[Any], child: SchemaNode, instance: Any, location: _Location, masked: Sequence[_Location]
    ) -> None:
        if not masked or location not in masked:
            stack.append((child, instance, location))


# ---------------------------------------------------------------------------
# Support counts
# ---------------------------------------------------------------------------


@dataclass(eq=False, slots=True)
class _NodeCounts:
    """The JSON types of the instances visited at one node, split as the Support rule needs them."""

    types: Counter[JType] = field(default_factory=Counter)
    branches: dict[tuple[str, int], Counter[JType]] = field(default_factory=dict)  # those each branch accepts
    rejected: dict[_Key, Counter[JType]] = field(default_factory=dict)  # those a Constraint rejects
    accepted: Counter[JType] | None = None  # those the node accepts as a whole (``whole`` nodes only)


def _count_of(counter: Mapping[JType, int] | None, applies: frozenset[JType] | None) -> int:
    if not counter:
        return 0
    return sum(n for jtype, n in counter.items() if applies is None or jtype in applies)


class InstanceCounts:
    """Counts of the instances each node of a model is evaluated on, for Support (see the module docstring).

    Built by :func:`count_instances`. ``pointers`` is the model's emitted
    layout pointer map (ConstraintId → JSON Pointer), the set of its
    Constraints.
    """

    __slots__ = ("_nodes", "_whole", "pointers")

    def __init__(self, nodes: dict[int, _NodeCounts], pointers: PointerMap, whole: Iterable[int]) -> None:
        self._nodes = nodes
        self._whole = dict.fromkeys(whole)
        self.pointers = pointers

    def support(self, cid: ConstraintId) -> Support:
        """The Support of Constraint ``cid`` (Support in the module docstring)."""
        counts = self._nodes.get(cid.nid)
        if counts is None:
            return Support(0, 0)
        applies = _APPLIES_TO.get(cid.keyword)
        total = _count_of(counts.types, applies)
        if cid.keyword in _COMBINATORS and isinstance(cid.item, int):
            branch = counts.branches.get((cid.keyword, cid.item))
            if branch is not None:
                return Support(_count_of(branch, applies), total)
        _, keyword, item = _norm(cid.nid, cid.keyword, cid.item)
        return Support(total - _count_of(counts.rejected.get((keyword, item)), applies), total)

    def node_support(self, nid: int) -> Support:
        """Every instance at node ``nid``, and as consistent those the node accepts (``whole`` nodes only)."""
        if nid not in self._whole:
            raise KeyError(f"node {nid} was not counted as a whole")
        counts = self._nodes.get(nid)
        if counts is None:
            return Support(0, 0)
        return Support(_count_of(counts.accepted, None), _count_of(counts.types, None))


def _count(
    snapshot: _Snapshot,
    targets: Iterable[int],
    documents: Iterable[tuple[int, JSON]],
    masked: Mapping[int, Sequence[_Location]] | None = None,
    whole: Iterable[int] = (),
) -> InstanceCounts:
    """Walk ``documents`` and count the instances at the ``targets`` (and accepted by the ``whole`` nodes)."""
    masked = masked or {}
    wholes = dict.fromkeys(nid for nid in whole if nid in snapshot.nodes)
    counted = dict.fromkeys(nid for nid in targets if nid in snapshot.nodes)
    counted.update(wholes)
    walker = _Walker(snapshot, counted, masked)
    nodes: dict[int, _NodeCounts] = {}
    branches: dict[int, list[tuple[str, int, SchemaNode]]] = {}
    for nid in counted:
        node = snapshot.nodes[nid]
        nodes[nid] = counts = _NodeCounts(accepted=Counter() if nid in wholes else None)
        for keyword in _COMBINATORS:
            listed = None if node.boolean is not None else node.kw.get(keyword)
            for index, child in enumerate(listed if isinstance(listed, list) else ()):
                branches.setdefault(nid, []).append((keyword, index, child))
                counts.branches[(keyword, index)] = Counter()
    rejected: dict[int, dict[tuple[int, _Location], list[_Key]]] = {}

    def tracked(items: Iterable[tuple[int, JSON]]) -> Iterator[tuple[int, JSON]]:
        for doc, value in items:
            if masked.get(doc):
                rejected[doc] = snapshot.rejections(doc, value, masked[doc])
            yield doc, value

    for doc, nid, location, instance, verdicts in walker.visits(tracked(documents)):
        jtype = jtype_of(instance)
        counts = nodes[nid]
        counts.types[jtype] += 1
        flagged = doc in rejected
        for keyword, index, child in branches.get(nid, ()):
            accepted = verdicts.get((keyword, index))
            if accepted is None:
                accepted = snapshot.accepts(child, instance, walker.masked_under(doc, location))
            if accepted:
                counts.branches[(keyword, index)][jtype] += 1
        if flagged:
            for key in rejected[doc].get((nid, location), ()):
                counts.rejected.setdefault(key, Counter())[jtype] += 1
        if counts.accepted is not None:
            node = snapshot.nodes[nid]
            if not flagged or snapshot.accepts(node, instance, walker.masked_under(doc, location)):
                counts.accepted[jtype] += 1
    return InstanceCounts(nodes, snapshot.pointers, wholes)


def count_instances(
    model: SchemaModel,
    documents: DocumentStore | Sequence[JSON],
    *,
    docs: Iterable[int] | np.ndarray | None = None,
    flagged_items: Mapping[int, Sequence[Sequence[str]]] | None = None,
    nodes: Iterable[int] | None = None,
    whole: Iterable[int] = (),
    validator: Validator | None = None,
) -> InstanceCounts:
    """Count the instances the nodes of ``model`` are evaluated on, for Support (see the module docstring).

    ``docs`` are the document indices to walk (all by default, ascending);
    ``flagged_items`` maps documents to the instance locations of their
    Flagged_Items (:func:`flagged_item_locations`); ``nodes`` restricts the
    counted nodes (every node by default); ``whole`` names nodes whose
    accepted instances are counted too (:meth:`InstanceCounts.node_support`).
    Every walked document without a Flagged_Item must be valid against the
    model (as after :func:`check_soundness`): its instances count as
    consistent without being checked. The model is laid out and compiled
    once; raises ``CompileError`` when it does not compile.
    """
    if not isinstance(model, SchemaModel):
        raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
    corpus = _Corpus(documents)
    if validator is None:
        validator = Validator()
    elif not isinstance(validator, Validator):
        raise TypeError(f"validator must be a Validator, got {type(validator).__name__}")
    walked = list(range(len(corpus))) if docs is None else _indices(docs, "docs", len(corpus))
    masked: dict[int, tuple[_Location, ...]] = {}
    for doc, locations in (flagged_items or {}).items():
        distinct = dict.fromkeys(tuple(str(token) for token in spot) for spot in locations)
        spots = tuple(sorted(distinct, key=_location_key))
        if spots:
            masked[int(doc)] = spots
    snapshot = _Snapshot(model, validator, "the covering schema (Support)")
    targets = list(snapshot.nodes) if nodes is None else list(nodes)
    return _count(snapshot, targets, ((doc, corpus.value(doc)) for doc in walked), masked, whole)


# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class AppliedChange:
    """One applied Change and its RelaxationRecord (3.6).

    ``phase`` is the loop that applied it (``"relaxation"`` for 3.3,
    ``"removal"`` for 3.7) and ``pass_index`` the validation pass whose
    failures triggered it (1-based). ``constraints`` are the ConstraintIds
    whose Provenance_Records received ``record``; ``removed`` is the report
    entry of a removal (then ``constraints`` is empty).
    """

    phase: Phase
    pass_index: int
    failure: Failure
    change: Change
    record: RelaxationRecord
    constraints: tuple[ConstraintId, ...]
    removed: RemovedConstraint | None

    @property
    def counted_as(self) -> Literal["relaxation", "removal"]:
        """The 3.4 count this record belongs to (see Counts in the module docstring)."""
        return "removal" if self.phase == "removal" and self.removed is not None else "relaxation"


@dataclass(frozen=True, eq=False, slots=True)
class SoundnessResult:
    """The outcome of :func:`check_soundness` for one covering schema.

    ``summary`` is the SoundnessSummary recorded in the report; ``changes``
    lists every change record in the order applied; ``validated``,
    ``flagged``, and ``failed_first_pass`` are the document indices behind
    the counts.
    """

    summary: SoundnessSummary
    changes: tuple[AppliedChange, ...]
    validated: tuple[int, ...]
    flagged: tuple[int, ...]
    failed_first_pass: tuple[int, ...]


# ---------------------------------------------------------------------------
# The loop
# ---------------------------------------------------------------------------


class _Loop:
    """The state of one soundness run over one covering schema."""

    def __init__(
        self,
        model: SchemaModel,
        corpus: _Corpus,
        docs: list[int],
        engine: DecisionEngine,
        validator: Validator,
        workers: int,
        chunk_size: int,
        name: str,
    ) -> None:
        self.model = model
        self.corpus = corpus
        self.docs = docs
        self.engine = engine
        self.validator = validator
        self.workers = workers
        self.chunk_size = chunk_size
        self.name = name
        self.mdl = engine.enabled(MDL_SELECTION)
        self.log: list[AppliedChange] = []
        self.removed: list[RemovedConstraint] = []
        self.relaxed: dict[ConstraintId, None] = {}  # Constraints whose records received a change
        self._snapshot: _Snapshot | None = None
        self._values: dict[int, JSON] = {}

    # -- state ------------------------------------------------------------------

    def snapshot(self) -> _Snapshot:
        if self._snapshot is None:
            self._snapshot = _Snapshot(self.model, self.validator, self.name)
        return self._snapshot

    def value(self, doc: int) -> JSON:
        found = self._values.get(doc)
        if found is None and doc not in self._values:
            found = self._values[doc] = self.corpus.value(doc)
        return found

    def documents(self) -> Iterator[tuple[int, JSON]]:
        for doc in self.docs:
            yield doc, self.corpus.value(doc)

    # -- validation ---------------------------------------------------------------

    def validate(self) -> dict[int, list[ErrorInfo]]:
        """One pass: the detailed errors of every failing validated document, by index."""
        compiled = self.snapshot().compiled
        self._values = {}
        size = self.chunk_size
        chunks = [self.docs[start : start + size] for start in range(0, len(self.docs), size)]
        parse = self.corpus.parse
        task = delayed(single_threaded(_validate_chunk))
        tasks = (task(compiled, [(doc, self.corpus.payload(doc)) for doc in chunk], parse) for chunk in chunks)
        failing: dict[int, list[ErrorInfo]] = {}
        for result in Parallel(n_jobs=self.workers)(tasks):
            for doc, errors in result:
                failing[doc] = errors
        return {doc: failing[doc] for doc in sorted(failing)}

    def groups(self, failing: dict[int, list[ErrorInfo]]) -> list[_Group]:
        snapshot = self.snapshot()
        found: dict[_Target, dict[tuple[int, _Location], _Occurrence]] = {}
        for doc, errors in failing.items():
            value = self.value(doc)
            for error in errors:
                for occurrence in snapshot.occurrences(doc, value, error):
                    found.setdefault(occurrence.target, {}).setdefault((doc, occurrence.location), occurrence)
        return _groups(found, snapshot.node_pointer)

    def refresh(self, group: _Group) -> tuple[Failure, tuple[_Occurrence, ...]] | None:
        """The group's Failure against the current model, or None when its target no longer fails."""
        snapshot = self.snapshot()
        found: dict[tuple[int, _Location], _Occurrence] = {}
        for doc in group.docs:
            value = self.value(doc)
            if self.validator.is_valid(snapshot.compiled, value):
                continue
            for error in self.validator.errors(snapshot.compiled, value, details=True):
                for occurrence in snapshot.occurrences(doc, value, error):
                    if occurrence.target == group.target:
                        found.setdefault((doc, occurrence.location), occurrence)
        if not found:
            return None
        occurrences = tuple(found.values())
        return _merge(group.target, occurrences), occurrences

    # -- relaxation (3.3) ---------------------------------------------------------

    def relax_pass(self, failing: dict[int, list[ErrorInfo]], pass_index: int) -> None:
        changed = False
        for group in self.groups(failing):
            if changed:
                current = self.refresh(group)
                if current is None:
                    continue
                failure, occurrences = current
            else:
                failure, occurrences = _merge(group.target, group.occurrences), group.occurrences
            if self.relax(failure, occurrences, pass_index):
                changed = True

    def relax(self, failure: Failure, occurrences: Sequence[_Occurrence], pass_index: int) -> bool:
        snapshot = self.snapshot()
        try:
            options = relaxations(self.model, failure, validator=self.validator)
        except (KeyError, ValueError):
            return False  # an earlier change of this pass removed the node or the keyword
        kept: list[tuple[Relaxation, SchemaModel]] = []
        for option in options:
            relaxed, changes = option.applied(self.model)
            if self.fixes(failure, relaxed, changes):
                kept.append((option, relaxed))
        if not kept:
            last = options[-1]
            kept.append((last, last.applied(self.model)[0]))  # removal, the widest candidate
        view: View | None = None
        if self.mdl:
            view = self.view_of(failure.nid, occurrences)
            candidates = []
            for option, relaxed in kept:
                node = relaxed.index().get(failure.nid)
                if node is None:
                    continue  # the node sat inside a removed not/if; nothing to score it on
                scorer = MDLScorer(root=relaxed.root, validator=self.validator)
                candidates.append(option.candidate(score=scorer.score(node, view)))
            if not candidates:
                return False
        else:
            candidates = [option.candidate() for option, _ in kept]
        decision = self.engine.resolve(DecisionPoint(RELAXATION, snapshot.path_of(failure.nid)), candidates, view)
        winner = decision.outcome
        assert isinstance(winner, Relaxation)
        self.apply(winner, failure, "relaxation", pass_index, decision)
        return True

    def fixes(self, failure: Failure, relaxed: SchemaModel, changes: Sequence[Change]) -> bool:
        """Whether the failing documents no longer fail at the Failure's target or what it became."""
        try:
            after = _Snapshot(relaxed, self.validator, f"{self.name} (relaxation candidate)")
        except (CompileError, EmitError):
            return False
        successors: dict[_Target, None] = {_norm(*failure.target): None}
        for change in changes:
            touches = any(_norm(cid.nid, cid.keyword, cid.item) in successors for cid in change.before_ids)
            if touches or (failure.keyword == FALSE_SCHEMA and change.nid == failure.nid):
                for cid in change.after_ids:
                    successors[_norm(cid.nid, cid.keyword, cid.item)] = None
        wrappers = {
            cid.nid for change in changes if change.guard and change.edit == "convert" for cid in change.after_ids
        }
        original = self.snapshot().nodes
        created = {nid for nid in after.nodes if nid not in original and nid not in wrappers}
        for doc in failure.documents:
            value = self.value(doc)
            if self.validator.is_valid(after.compiled, value):
                continue
            for error in self.validator.errors(after.compiled, value, details=True):
                try:
                    found = after.occurrences(doc, value, error, branches=False)
                except SoundnessError:
                    return False
                for occurrence in found:
                    nid = occurrence.target[0]
                    if (
                        _norm(*occurrence.target) in successors
                        or nid in created
                        or (failure.keyword == FALSE_SCHEMA and nid == failure.nid)
                    ):
                        return False
        return True

    def view_of(self, nid: int, occurrences: Sequence[_Occurrence]) -> View:
        """The Failure node's Observations plus the failing values, as the root view of a store of their own."""
        found = _Walker(self.snapshot(), [nid]).collect(self.documents())[nid]
        for occurrence in occurrences:
            found.setdefault((occurrence.doc, occurrence.location), occurrence.value)
        return View.root(build_store(list(found.values())))

    # -- removal (3.7) ------------------------------------------------------------

    def removal_round(self, failing: dict[int, list[ErrorInfo]], pass_index: int) -> None:
        applied = 0
        for group in self.groups(failing):
            if applied:
                current = self.refresh(group)
                if current is None:
                    continue
                failure = current[0]
            else:
                failure = _merge(group.target, group.occurrences)
            try:
                option = removal(self.model, failure)
            except (KeyError, ValueError):
                continue
            if self.apply(option, failure, "removal", pass_index, None):
                applied += 1
        if not applied:
            raise SoundnessError("a removal round removed nothing although documents still fail")

    # -- applying and recording (3.6) ---------------------------------------------

    def apply(
        self, option: Relaxation, failure: Failure, phase: Phase, pass_index: int, decision: Decision | None
    ) -> int:
        before = self.snapshot()
        changes = option.apply(self.model)
        self._snapshot = None
        resolution = None if decision is None else decision.resolution
        for change in changes:
            self.record(change, failure, phase, pass_index, resolution, decision, before)
        live = self.model.index()
        gone = {nid for nid in before.nodes if nid not in live}
        if gone:
            for cid in [cid for cid in self.model.provenance if cid.nid in gone]:
                del self.model.provenance[cid]
                self.relaxed.pop(cid, None)
        if decision is not None:
            self.record_created(before, live, decision)
        return len(changes)

    def record_created(self, before: _Snapshot, live: Mapping[int, SchemaNode], decision: Decision) -> None:
        """Give the Constraints inside the subschemas a 3.3 change created the relaxation decision's record (26.1).

        A Naive_Merge subschema added for new properties, extra elements, a
        new branch, or a replaced ``false`` subschema holds Constraints of
        its own (``type``, ``properties``, ``items``, ``required``) that no
        Change lists. They are produced by the relaxation decision, so each
        gets its record without a change record (nothing of theirs was
        changed). Like every record that received no change, its Support is
        left to provenance finalization (task 17.1), which recomputes every
        Support and, from it, the Laplace default Confidence.
        """
        created = dict.fromkeys(nid for nid in live if nid not in before.nodes)
        if not created:
            return
        provenance = self.model.provenance
        _, pointers = _EMITTER.layout(self.model)
        for cid in pointers:
            if cid.nid in created and cid not in provenance:
                provenance[cid] = decision.provenance(Support(0, 0))

    def record(
        self,
        change: Change,
        failure: Failure,
        phase: Phase,
        pass_index: int,
        resolution: Resolution | None,
        decision: Decision | None,
        before: _Snapshot,
    ) -> None:
        provenance = self.model.provenance
        documents = failure.documents
        if change.removed or (change.edit == "replace" and change.before is False and change.after is True):
            record = RelaxationRecord(change.before, REMOVED, documents, resolution)
            entry = RemovedConstraint(self._removed_pointer(change, before), record)
            self.removed.append(entry)
            for cid in change.before_ids:
                provenance.pop(cid, None)
                self.relaxed.pop(cid, None)
            self.log.append(AppliedChange(phase, pass_index, failure, change, record, (), entry))
            return
        record = change.record(documents, resolution)
        kept = dict.fromkeys(change.after_ids)
        old = dict.fromkeys(change.before_ids)
        gone = [cid for cid in change.before_ids if cid not in kept]
        new = [cid for cid in change.after_ids if cid not in old]
        renamed = _renames(gone, new)
        paired = dict.fromkeys(renamed.values())
        for cid in gone:
            if cid not in paired:
                provenance.pop(cid, None)
                self.relaxed.pop(cid, None)
        holders: list[ConstraintId] = []
        for cid in change.after_ids:
            source = cid if cid in old else renamed.get(cid)
            existing = provenance.pop(source, None) if source is not None else None
            if source is not None:
                self.relaxed.pop(source, None)
            if existing is None:
                if decision is None:
                    continue  # the removal loop extends existing records only
                existing = decision.provenance(Support(0, 0))
            provenance[cid] = replace(existing, changes=(*existing.changes, record))
            self.relaxed[cid] = None
            holders.append(cid)
        self.log.append(AppliedChange(phase, pass_index, failure, change, record, tuple(holders), None))

    @staticmethod
    def _removed_pointer(change: Change, before: _Snapshot) -> str:
        """The emitted pointer of what a removal removed, in the layout just before it."""
        ids = change.before_ids
        if len(ids) == 1 and ids[0] in before.pointers:
            return before.pointers[ids[0]]
        node = before.node_pointer.get(change.nid, "")
        fragment = change.before
        if isinstance(fragment, dict) and len(fragment) == 1:
            return join_pointer(node, next(iter(fragment)))
        return node

    # -- Support (3.6) ------------------------------------------------------------

    def recompute_support(self) -> None:
        provenance = self.model.provenance
        ids = [cid for cid in self.relaxed if cid in provenance]
        if not ids:
            return
        snapshot = self.snapshot()
        ids = [cid for cid in ids if cid.nid in snapshot.nodes]
        counts = _count(snapshot, (cid.nid for cid in ids), self.documents())
        for cid in sorted(ids):
            provenance[cid] = replace(provenance[cid], support=counts.support(cid))

    # -- summary ------------------------------------------------------------------

    def counts(self) -> tuple[int, int]:
        relaxed = sum(1 for entry in self.log if entry.counted_as == "relaxation")
        return relaxed, len(self.log) - relaxed


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def check_soundness(
    model: SchemaModel,
    documents: DocumentStore | Sequence[JSON],
    *,
    engine: DecisionEngine | None = None,
    validator: Validator | None = None,
    report: InferenceReport | None = None,
    flagged: Iterable[int] | np.ndarray = (),
    segment: Iterable[int] | np.ndarray | None = None,
    version: int | None = None,
    max_passes: int | None = None,
    workers: int = 1,
    chunk_size: int | None = None,
) -> SoundnessResult:
    """Validate, relax, and remove until every validated document is valid (see the module docstring).

    ``model`` is edited in place, Provenance_Records included. ``engine``
    resolves the ``relaxation`` Decision_Points (a default DecisionEngine
    when omitted); ``report`` receives the SoundnessSummary. ``max_passes``
    and ``chunk_size`` default to their Resource_Parameter defaults, and
    ``workers`` to 1. Raises ``CompileError`` when the covering schema does
    not compile and :class:`SoundnessError` for a validation error that
    names no Constraint of the model.
    """
    if not isinstance(model, SchemaModel):
        raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
    corpus = _Corpus(documents)
    if engine is None:
        engine = DecisionEngine()
    elif not isinstance(engine, DecisionEngine):
        raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
    if validator is None:
        validator = Validator()
    elif not isinstance(validator, Validator):
        raise TypeError(f"validator must be a Validator, got {type(validator).__name__}")
    if report is not None and not isinstance(report, InferenceReport):
        raise TypeError(f"report must be an InferenceReport or None, got {type(report).__name__}")
    passes_limit = _setting(MAX_PASSES, max_passes)
    size = _setting(CHUNK_SIZE, chunk_size)
    workers = _setting(WORKERS, workers)
    n_docs = len(corpus)
    covered = list(range(n_docs)) if segment is None else _indices(segment, "segment", n_docs)
    excluded = dict.fromkeys(_indices(flagged, "flagged", n_docs))
    docs = [doc for doc in covered if doc not in excluded]
    flagged_in = tuple(doc for doc in covered if doc in excluded)
    name = "the covering schema" if version is None else f"the schema of Schema_Version {version}"

    loop = _Loop(model, corpus, docs, engine, validator, workers, size, name)
    loop.snapshot()  # compile the covering schema, even with nothing to validate
    passes = 0
    first: tuple[int, ...] = ()
    limit_reached = False
    if docs:
        failing = loop.validate()
        passes = 1
        first = tuple(failing)
        while failing and passes < passes_limit:
            loop.relax_pass(failing, passes)
            failing = loop.validate()
            passes += 1
        if failing:
            limit_reached = True
            while failing:
                loop.removal_round(failing, passes)
                failing = loop.validate()
                passes += 1
        loop.recompute_support()
    relaxed, removed = loop.counts()
    summary = SoundnessSummary(
        flagged_documents=len(flagged_in),
        validated=len(docs),
        failed_first_pass=len(first),
        relaxations=relaxed,
        removals=removed,
        passes=passes,
        pass_limit_reached=limit_reached,
        removed=tuple(loop.removed),
        version=version,
    )
    if report is not None:
        report.record_soundness(summary)
    return SoundnessResult(summary, tuple(loop.log), tuple(docs), flagged_in, first)
