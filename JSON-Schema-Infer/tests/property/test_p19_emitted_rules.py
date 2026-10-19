"""Property 19: Emitted rules hold and are irredundant (task 16.7).

For any collection:

- every emitted ``dependentRequired`` entry, Exclusion_Rule (``oneOf`` or
  ``not`` form), ``x-key``, ``x-references``, and ``x-functional-dependency``
  holds on all non-flagged Observations;
- no emitted rule has Support or Confidence below its Learned_Threshold;
- the emitted Co_Occurrence_Rules derive every selected rule, and none is
  derivable from the others plus ``required``;
- ``uniqueItems`` appears only on Lists of scalars that carry a selected
  Key_Constraint;
- an Exclusion_Rule schema rejects a generated object that combines two
  members.

Co-occurrence mining (``rules.cooccurrence``) is the only rule Technique
implemented so far. Every clause is written over what the schema holds, not
over which Technique is implemented, so the clauses about Exclusion_Rules
(task 35.1), Key_Constraints, Reference_Constraints, and Functional_Dependencies
(task 44.1) start checking as soon as those Techniques emit anything. Until
then each is an asserted absence (see Gated clauses).

How a run is driven
    The test runs the two pipeline stages that produce the emitted schema,
    as ``pipeline.infer`` does: :meth:`StructureAnalyzer.infer_model` on the
    root view, then :func:`check_soundness` with the same DecisionEngine,
    Validator, and Inference_Report, the Flagged_Documents of
    :func:`flagged_documents`, and the run's ``soundness.max_passes``. The
    emitted layout is ``SchemaEmitter.layout`` of the model after soundness.
    It does not call ``infer_documents``, for two reasons. The pipeline loads
    its Model_Bundle from disk with ``open_bundle``, and co-occurrence needs
    hand-set Learned_Thresholds; here a stub bundle supplies them without
    patching the pipeline. And the pipeline flags nothing yet (the outlier
    and MDL-exception Techniques are unimplemented), so "non-flagged" would
    never be exercised; here the test sets FlagSet flags on the store before
    the analysis, as Properties 11, 12, and 18 do.

Inputs
    Documents come from a random template whose Records plant co-occurrence.
    Each object of a Record Variant draws one level ``u`` in ``0..7``, and a
    field is ``always`` present, present when ``u`` is below its cut (``2``,
    ``4``, or ``6``), or present on a coin flip. Fields with the same cut
    travel together (a strongly connected component of mutual rules, like
    the ADDRESSES fixture of ``tests/unit/test_cooccurrence.py``), and
    fields with smaller cuts imply those with larger ones (one-way chains,
    like CHAIN). A Variant's fields draw from at most two cuts. Records have
    one to three Variants with disjoint key sets, sometimes plus one shared
    key, and sit at the root, in properties, in List and Tuple elements, in
    Map values, and beside scalars. Member values are ``null`` in about one
    draw in five. Keys come from :data:`tests.strategies.KEYS` (the empty
    key, astral and non-ASCII keys). Collections have 10 to 40 documents,
    and 20 to 60 under MDL, where a rule must save more bits than its entry
    costs, so MDL selects some rules and rejects others.

    Flags: up to two spoiler documents with arbitrary field presence (which
    break the planted rules) are always flagged, and one more document may
    be. When documents are arrays of Records, a spoiled Record may be
    appended to one array and flagged as an item. Up to two other objects
    and up to two member values are flagged as items; an object whose member
    value is flagged keeps the key (17.4), so it still holds the field.

    Technique states come from ``run_configs()`` (implemented Techniques
    only) with small Resource_Parameters and ``soundness.max_passes`` in
    {1, 2, 5}. Each scenario is its own test, so each gets the full example
    budget:

    - ``mdl``: ``decide.mdl`` and ``rules.cooccurrence`` on, so MDL selects
      (9.4);
    - ``threshold``: ``decide.mdl`` off, so ``cooccur.min_confidence``
      selects (9.8) and split Records keep their Variants (5.11). A grouping
      resolved by priority often gives each key set its own Variant, which
      leaves no optional field, so the Variants come from no grouping
      Technique (one Variant, 5.10), from LSH alone at τ ∈ {0, 0.05}, or
      from drawn states;
    - ``drawn``: every implemented Technique drawn, with the co-occurrence
      thresholds both present, both absent, or one missing (the Technique is
      then out of effect, 20.6).

    A stub Model_Bundle holds hand-set Learned_Thresholds within their
    admissible ranges, each Technique's present or absent together.
    ``cooccur.min_confidence`` is often exactly a Laplace Confidence
    ``(k + 1) / (k + 2)``, so the "at or above" boundary is hit. The bundle
    also holds the thresholds of the unimplemented rule Techniques, so their
    clauses run once those Techniques are enabled.

Reference objects (holds on all non-flagged Observations)
    Every node of every document has a location ``(document index, key or
    position, …)``. The reference reads presence from the raw Python
    objects, independently of the ObservationStore: an object holds a field
    when it has the key, whatever its value (``null`` and Flagged_Item values
    included), and an object is non-flagged when neither its document nor it
    nor an ancestor is flagged.

    A ``dependentRequired`` entry sits in the object schema of one final
    Variant (``StructureAnalyzer._variant_node``). The test walks the model
    after analysis (before soundness) from the root, carrying the
    non-flagged raw instances that reach each subschema, as Property 18
    does: ``properties`` take the values of their key, a Map's
    ``additionalProperties`` every member value, ``items`` every element,
    ``prefixItems`` the elements at its position, and the branches of a
    mixed-type ``anyOf`` the instances of their JSON types. At a split Record
    (two or more object branches) ``anyOf`` cannot assign objects to
    branches, so the walk asks the run's VariantAnalyzer for the final
    Variants of exactly the objects that reach the node
    (``merge_or_split(select(view))``) and maps each Variant's rows to raw
    locations through the store's ``parent`` links. The store is used only
    for this routing; every presence fact comes from the raw objects. The
    walk checks that every non-flagged object reaches exactly one Record or
    Map, and that the flags are unchanged. It records, per node id, the
    instances that reached it and its Path; nids survive the soundness check
    (it edits nodes in place), which the test asserts for every node that
    still holds a rule keyword.

    An entry ``a: [b, …]`` holds when every non-flagged object of its
    Variant that holds ``a`` holds every ``b``. The test checks this for the
    entries after soundness and, more strongly, for the analyzer's entries
    before it: soundness validates only documents without any flag, so an
    object in a document that holds a Flagged_Item elsewhere is checked only
    here.

Readings
    Selected rules. A selected rule is a Co_Occurrence_Rule chosen under 9.4
    or 9.8. Each one is its own ``cooccurrence_rule`` Decision_Point, so the
    report lists one decision per selected rule (selected candidate produced
    by ``rules.cooccurrence``, outcome label ``{"a":["b"]}``). Per Variant,
    the selected rules are also in the Provenance_Records of the emitted
    entries: ``details.rules`` (the emitted rules of the entry, each with its
    Confidence, method, resolution, and cut-off) and ``details.implied`` (the
    selected rules with that antecedent that the reduction left out). The
    test reads the per-Variant selection from the records and checks it
    against the report: per Path, the multiset of rules selected in the
    records equals the multiset of rule-selecting decisions. Without
    Field_Group entries every selected rule has an emitted entry for its
    antecedent (the chain that derives it starts with an emitted rule out of
    the antecedent), so the two multisets must be equal. Field_Group cliques
    (task 33.1) are given rules that may start a chain; then a decision
    without a record must be derivable in some Variant at its Path.

    Derivation and irredundancy (9.5). A rule ``a → b`` is derivable from a
    set of rules when ``b`` is reachable from ``a`` by chaining them. The
    ``required`` list takes part as the implications ``x → r`` for every
    field ``x`` and required field ``r`` (``r`` is in every object), so a
    chain may start from ``a`` or from any required field. A chain through a
    required field cannot derive an emitted rule, because a rule ``r → y``
    out of a required ``r`` would make ``y`` required, and consequents are
    optional (9.3); the test still includes ``required``, as the design
    states, and asserts that no emitted rule has a required antecedent or
    consequent. (a) Every selected rule is derivable from the analyzer's
    emitted rules, the Field_Group entries (6.6), and ``required``. This is
    checked before soundness: 9.5 constrains the subset the Schema_Emitter
    is given, and the soundness check only removes whole entries, each
    recorded (3.6). (b) No emitted co-occurrence rule is derivable from the
    other ``dependentRequired`` pairs of its node and ``required``, checked
    both before and after soundness (a subset of an irredundant set stays
    irredundant, so removals cannot break it).

    Thresholds. ``cooccur.min_support`` applies to the Support count of every
    candidate, whichever selection method runs (9.6): each emitted and
    selected rule has an antecedent held by at least that many non-flagged
    objects of the Variant, counted from the raw objects and equal to the
    ``support_count`` in the record. ``cooccur.min_confidence`` is the
    selection cut-off only while ``decide.mdl`` is off (9.8): then each
    selected rule's Confidence is the Laplace value ``(count(a) + 1) /
    (count(a) + 2)`` (design, Open Decisions), at or above the threshold,
    with that threshold recorded as the cut-off, and the test checks that
    exactly the reference candidates meeting both thresholds are selected.
    While MDL selects (9.4) there is no Learned_Threshold on Confidence: the
    rule's cut-off is the "MDL comparison", the rule's description length is
    strictly below that of the subschema without it, and its Confidence is
    the MDL posterior of the two recorded scores, so at least 1/2. The
    selected rules are then a subset of the reference candidates that pass
    the Support filter. The entry's record carries the lowest Confidence of
    its rules and Support ``(n, n)`` over the Variant's ``n`` non-flagged
    objects.

    Soundness. After the check, every node that holds ``dependentRequired``
    held it before, with the same consequents per antecedent (entries are
    only removed whole). Each entry that disappeared is named in the
    ``before_ids`` of an applied change, or its node left the model with an
    enclosing keyword. Every remaining entry keeps its record, and its
    emitted pointer resolves to its consequents in the emitted document.

Gated clauses (activate when the producing Technique emits)
    - Exclusion_Rule, ``not`` form (11.6): every ``not`` keyword in the
      schema. It must carry a Provenance_Record from ``rules.exclusion``,
      which must be implemented and enabled. Holds: every non-flagged
      object reaching the node is valid against ``{"not": …}``. Rejects a
      combination: for each ``required`` set in the ``not`` subschema (or in
      its ``anyOf``/``oneOf``/``allOf`` branches), the first object reaching
      the node, extended with those fields (values taken from objects that
      hold them), is invalid.
    - Exclusion_Rule, ``oneOf`` form (11.5): the ``oneOf`` branch Constraints
      whose records name ``rules.exclusion``. Holds: every non-flagged
      object reaching the node is valid against the ``oneOf``. Rejects a
      combination: for each pair of branches with ``required`` lists, an
      object valid against the first, extended with the second's fields, is
      invalid against the ``oneOf``. If task 35.1 places an Exclusion
      ``oneOf`` at a node without ``type`` (beside the Variant branches), the
      routing walk must learn that placement.
    - Exclusion thresholds (11.4): the cut-off is ``exclusion.min_confidence``
      with the Confidence at or above it, or an MDL comparison the selected
      candidate wins strictly.
    - ``x-key`` (12.1, 12.2, 12.8): on a node whose instances are scalars;
      holds when the values are distinct (JSON Schema instance equality)
      across the collection (scope ``collection``) or within each nearest
      enclosing array (scope ``array``). The scope is read from the
      annotation (a string, or a ``scope`` member); an unreadable value
      fails with a message naming :func:`key_scope`.
    - ``x-references`` (12.3, 12.8): the referenced Path (a Path string, or a
      ``path`` member) holds a node with ``x-key``, and every value at the
      referencing node occurs at the referenced Path.
    - ``x-functional-dependency`` (12.4, 12.8): one or more
      ``{"determinant": P, "dependent": Q}`` objects with absolute Path
      strings below the annotated object node, through Key steps only; holds
      when, over the non-flagged objects holding both, each determinant
      value co-occurs with one dependent value.
    - Key, Reference, and FD thresholds (12.6): the record names
      ``rules.keys`` or ``rules.fd_ind`` and its Confidence is at or above
      ``keys.min_confidence``, ``ind.min_confidence``, or
      ``fd.min_confidence``.
    - ``uniqueItems`` (12.7, 12.8): only on a List (``items`` without
      ``prefixItems``) whose ``items`` subschema carries ``x-key`` and whose
      non-flagged elements are all scalars.

    While ``rules.exclusion``, ``rules.keys``, and ``rules.fd_ind`` are
    unimplemented, the test also asserts that no Provenance_Record names
    them and that the schema holds none of their keywords (``not``,
    ``x-key``, ``uniqueItems``, ``x-references``,
    ``x-functional-dependency``). Field_Group entries (``dependentRequired``
    consequents not listed in a co-occurrence record) are rejected until a
    Field_Group Technique is implemented and enabled, and are then treated
    as given rules.

**Validates: Requirements 9.5, 9.6, 11.5, 11.6, 12.6, 12.7, 12.8**
"""

# Feature: json-schema-inference, Property 19: Emitted rules hold and are irredundant

from __future__ import annotations

import json
from collections import Counter, defaultdict
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any

import numpy as np
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.determinism import sorted_codepoint
from jsinfer.core.jsonvalue import canonical_key
from jsinfer.core.observations import ObservationStore, ShapePath, build_store
from jsinfer.core.paths import ELEM_STEP, KEY, MAPVAL_STEP, ROOT, Path, key_step, parse_path, render_path
from jsinfer.core.provenance import MDL_COMPARISON, ProvenanceRecord
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode, split_pointer
from jsinfer.core.views import View
from jsinfer.decide.engine import mdl_posterior
from jsinfer.deps.cooccurrence import (
    COOCCURRENCE,
    COOCCURRENCE_RULE,
    DEPENDENT_REQUIRED,
    MIN_CONFIDENCE,
    MIN_SUPPORT,
    NO_RULE,
    rule_confidence,
)
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.pipeline.soundness import MAX_PASSES, SoundnessResult, check_soundness, flagged_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.structure.record_map import object_rows
from jsinfer.structure.variants import ANY_OF, GROUPING_TECHNIQUES, MINHASH_LSH, ONE_OF, MergeSplitOutcome
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, run_configs

# One Validator for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
MDL = "decide.mdl"

# The rule Techniques of Requirements 6, 11, and 12, and their Learned_Thresholds.
EXCLUSION = "rules.exclusion"
KEY_TECHNIQUE = "rules.keys"
FD_IND = "rules.fd_ind"
EXCLUSION_THRESHOLD = "exclusion.min_confidence"
FIELD_GROUP_TECHNIQUES = REGISTRY.decision_point("field_groups").served_by
# Annotation keyword → (producing Technique, Learned_Threshold on Confidence).
ANNOTATIONS: dict[str, tuple[str, str]] = {
    "x-key": (KEY_TECHNIQUE, "keys.min_confidence"),
    "x-references": (FD_IND, "ind.min_confidence"),
    "x-functional-dependency": (FD_IND, "fd.min_confidence"),
}
# The keywords each unimplemented rule Technique would emit (asserted absent until it is implemented).
EMITTED_BY: dict[str, tuple[str, ...]] = {
    EXCLUSION: ("not",),
    KEY_TECHNIQUE: ("x-key", "uniqueItems"),
    FD_IND: ("x-references", "x-functional-dependency"),
}
for _tid in (EXCLUSION, KEY_TECHNIQUE, FD_IND, *FIELD_GROUP_TECHNIQUES):
    REGISTRY.technique(_tid)  # fails at import if an id is not registered

Loc = tuple[Any, ...]  # (document index, key or position, …)
Rule = tuple[str, str]
OBJECT = "object"
ARRAY = "array"
REQUIRED = "required"

# ---------------------------------------------------------------------------
# Templates and documents
# ---------------------------------------------------------------------------

ALWAYS, LEVEL, COIN = "always", "level", "coin"
LEVELS = 8  # an object's level u is drawn from 0..LEVELS-1
CUTS = (2, 4, 6)  # a LEVEL field is present when u < cut


@dataclass(frozen=True)
class Field:
    """One field of a Record Variant: its key, presence mode (and cut), and value template."""

    key: str
    mode: str
    cut: int
    child: Template


@dataclass(frozen=True)
class Template:
    """A generator of JSON values (see the module docstring, Inputs).

    ``record``: ``variants`` holds the fields of each Variant. ``map``,
    ``list``: ``children`` is the value template. ``tuple``: one template per
    position. ``mix``: the alternatives.
    """

    kind: str
    variants: tuple[tuple[Field, ...], ...] = ()
    children: tuple[Template, ...] = ()


SCALAR = Template("scalar")
KEYLESS = Template("record", variants=((),))

SCALAR_VALUES = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(-3, 3),
    st.sampled_from([1.0, 2.5, -0.0, 2**70]),
    st.text("ab😀", max_size=3),
)
# Data-like Map keys, so description length often prefers a Map; plus the shared alphabet.
MAP_KEYS = st.one_of(st.integers(0, 999).map(lambda i: f"u{i}"), st.sampled_from(KEYS))

_KINDS = ("scalar", "scalar", "scalar", "scalar", "record", "record", "list", "keyless", "map", "tuple", "mix")
_LEAF_KINDS = ("scalar", "scalar", "scalar", "keyless")


def child_templates(depth: int) -> st.SearchStrategy[Template]:
    return templates(depth - 1) if depth > 0 else st.just(SCALAR)


@st.composite
def record_templates(draw: st.DrawFn, depth: int) -> Template:
    """One to three Variants with disjoint key sets (maybe plus one shared key), presence planted by level cuts."""
    n_variants = draw(st.sampled_from([1, 1, 1, 2, 2, 3]), label="variants")
    order = draw(st.permutations(KEYS), label="key order")
    # A shared key joins the Variants' key sets, so LSH at a near-zero τ would group them all; keep it rarer.
    shared = order[:1] if n_variants > 1 and draw(st.integers(0, 2), label="a shared key") == 0 else ()
    rest = order[len(shared) :]
    room = min(len(rest) // n_variants, 6 if n_variants == 1 else 3)
    cuts = draw(st.lists(st.sampled_from(CUTS), min_size=1, max_size=2, unique=True), label="level cuts")
    variants: list[tuple[Field, ...]] = []
    start = 0
    for _ in range(n_variants):
        size = draw(st.integers(min(2, room), room), label="own keys")
        own = rest[start : start + size]
        start += size
        fields = []
        for key in (*shared, *own):
            mode = draw(st.sampled_from([ALWAYS, LEVEL, LEVEL, LEVEL, COIN]), label=f"{key!r} presence")
            cut = draw(st.sampled_from(cuts), label=f"{key!r} cut") if mode == LEVEL else 0
            fields.append(Field(key, mode, cut, draw(child_templates(depth), label=f"{key!r} values")))
        variants.append(tuple(fields))
    return Template("record", variants=tuple(variants))


@st.composite
def templates(draw: st.DrawFn, depth: int, mix: bool = True) -> Template:
    kinds = _KINDS if depth > 0 else _LEAF_KINDS
    kind = draw(st.sampled_from(kinds if mix else [k for k in kinds if k != "mix"]), label="kind")
    if kind == "scalar":
        return SCALAR
    if kind == "keyless":
        return KEYLESS
    if kind == "record":
        return draw(record_templates(depth - 1))
    if kind == "list":
        element = draw(st.one_of(child_templates(depth), record_templates(depth - 1)), label="elements")
        return Template("list", children=(element,))
    if kind == "map":
        return Template("map", children=(draw(child_templates(depth)),))
    if kind == "tuple":
        positions = draw(st.lists(child_templates(depth), min_size=1, max_size=3), label="positions")
        return Template("tuple", children=tuple(positions))
    assert kind == "mix"
    options = draw(st.lists(templates(depth, mix=False), min_size=2, max_size=3), label="alternatives")
    return Template("mix", children=tuple(options))


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    """A Record, a Record among other types, or an array of Records, with one level of nesting below."""
    record = draw(record_templates(2), label="root record")
    shape = draw(st.sampled_from(["record", "record", "record", "mixed", "array"]), label="root shape")
    if shape == "record":
        return record
    if shape == "mixed":
        return Template("mix", children=(record, SCALAR))
    return Template("list", children=(record,))


@st.composite
def instances(draw: st.DrawFn, template: Template, spoil: bool = False) -> Any:
    """A value of ``template``. A spoiler (``spoil``) draws every field's presence on a coin flip."""
    kind = template.kind
    if kind == "scalar":
        return draw(SCALAR_VALUES)
    if kind == "record":
        fields = template.variants[draw(st.integers(0, len(template.variants) - 1), label="variant")]
        level = draw(st.integers(0, LEVELS - 1), label="level")
        out: dict[str, Any] = {}
        for f in fields:
            if spoil or f.mode == COIN:
                present = draw(st.booleans(), label=f"include {f.key!r}")
            else:
                present = f.mode == ALWAYS or level < f.cut
            if present:
                null = draw(st.integers(0, 4), label=f"{f.key!r} null") == 0
                out[f.key] = None if null else draw(instances(f.child, spoil))
        return out
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(MAP_KEYS, max_size=3), label="map keys"))
        return {key: draw(instances(template.children[0], spoil)) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0], spoil), max_size=3), label="elements")
    if kind == "tuple":
        length = draw(st.integers(0, len(template.children)), label="tuple length")
        return [draw(instances(child, spoil)) for child in template.children[:length]]
    assert kind == "mix"
    return draw(instances(template.children[draw(st.integers(0, len(template.children) - 1))], spoil))


def raw_nodes(docs: Sequence[Any]) -> list[tuple[Loc, Any, bool]]:
    """Every node of the documents as ``(location, value, parent is an object)``, flags ignored, in pre-order."""
    out: list[tuple[Loc, Any, bool]] = []

    def visit(loc: Loc, value: Any, in_object: bool) -> None:
        out.append((loc, value, in_object))
        if isinstance(value, dict):
            for key, child in value.items():
                visit((*loc, key), child, True)
        elif isinstance(value, list):
            for i, child in enumerate(value):
                visit((*loc, i), child, False)

    for d, doc in enumerate(docs):
        visit((d,), doc, False)
    return out


@dataclass(frozen=True)
class Case:
    """Documents and their flags: flagged document indices and Flagged_Item locations."""

    docs: tuple[Any, ...]
    flagged_docs: frozenset[int]
    flagged_items: frozenset[Loc]
    spoiled_items: frozenset[Loc] = frozenset()

    def excluded(self, loc: Loc) -> bool:
        """Whether the node at ``loc`` is in a flagged document, or is or lies under a Flagged_Item."""
        return loc[0] in self.flagged_docs or any(loc[:i] in self.flagged_items for i in range(2, len(loc) + 1))

    def live_objects(self) -> list[Loc]:
        """Every non-flagged object of every document (the reference for routing coverage)."""
        return [loc for loc, value, _ in raw_nodes(self.docs) if isinstance(value, dict) and not self.excluded(loc)]


@st.composite
def cases(draw: st.DrawFn, min_docs: int = 10, max_docs: int = 40) -> Case:
    root = draw(root_templates(), label="root template")
    n_docs = draw(st.integers(min_docs, max_docs), label="documents")
    clean = [draw(instances(root), label="document") for _ in range(n_docs)]
    spoilers = draw(st.lists(instances(root, spoil=True), max_size=2), label="spoilers")
    units = [(doc, False) for doc in clean] + [(doc, True) for doc in spoilers]
    units = [units[i] for i in draw(st.permutations(range(len(units))), label="document order")]
    docs = [doc for doc, _ in units]
    extra = draw(st.sets(st.integers(0, len(docs) - 1), max_size=1), label="flagged clean document")
    flagged_docs = frozenset(i for i, (_, spoiler) in enumerate(units) if spoiler) | extra

    flagged: set[Loc] = set()
    spoiled: set[Loc] = set()
    if root.kind == "list":
        # A spoiled Record appended to one array and flagged as an item: it breaks the planted rules if counted.
        arrays = [i for i, doc in enumerate(docs) if isinstance(doc, list) and i not in flagged_docs]
        if arrays and draw(st.booleans(), label="spoiled element"):
            i = draw(st.sampled_from(arrays), label="array with a spoiled element")
            docs[i] = [*docs[i], draw(instances(root.children[0], spoil=True), label="spoiled element")]
            spoiled.add((i, len(docs[i]) - 1))
    nodes = [(loc, value, in_object) for loc, value, in_object in raw_nodes(docs) if len(loc) > 1]
    nodes = [node for node in nodes if node[0][0] not in flagged_docs and node[0] not in spoiled]
    objects = [loc for loc, value, _ in nodes if isinstance(value, dict)]
    members = [loc for loc, _, in_object in nodes if in_object]
    if objects:
        flagged |= draw(st.sets(st.sampled_from(objects), max_size=2), label="flagged objects")
    if members:
        flagged |= draw(st.sets(st.sampled_from(members), max_size=2), label="flagged member values")
    return Case(tuple(docs), flagged_docs, frozenset(flagged | spoiled), frozenset(spoiled))


# ---------------------------------------------------------------------------
# The store, its flags, and row locations
# ---------------------------------------------------------------------------


def store_locations(store: ObservationStore) -> dict[ShapePath, list[Loc]]:
    """The location of every row of every column, found by following ``parent`` links to the document row."""
    out: dict[ShapePath, list[Loc]] = {}
    for shape in sorted(store.columns, key=len):
        column = store.columns[shape]
        if not shape:
            out[shape] = [(row,) for row in range(len(column))]  # root row i is document i
            continue
        parents = out[shape[:-1]]
        if shape[-1] == MAPVAL_STEP:
            steps: list[Any] = [store.key_name(int(kid)) for kid in column.key.tolist()]
        else:
            steps = column.pos.tolist()
        out[shape] = [(*parents[parent], step) for parent, step in zip(column.parent.tolist(), steps)]
    return out


def build(case: Case) -> tuple[ObservationStore, dict[ShapePath, list[Loc]]]:
    """A fresh store of the documents with the case's flags, and its row locations."""
    store = build_store(list(case.docs))
    locations = store_locations(store)
    where = {loc: (shape, row) for shape, locs in locations.items() for row, loc in enumerate(locs)}
    raw = [loc for loc, _, _ in raw_nodes(case.docs)]
    assert len(where) == len(raw) and set(where) == set(raw), "row locations differ from the raw documents' locations"
    for loc in sorted(case.flagged_items, key=repr):
        shape, row = where[loc]
        store.flags.flag_items(shape, [row])
    if case.flagged_docs:
        store.flags.flag_documents(sorted(case.flagged_docs))
    return store, locations


def flag_state(store: ObservationStore) -> tuple[Any, ...]:
    flags = store.flags
    return flags.epoch, flags.documents.tolist(), [(column, rows.tolist()) for column, rows in flags.items()]


# ---------------------------------------------------------------------------
# Technique states, Resource_Parameters, and the stub Model_Bundle
# ---------------------------------------------------------------------------

# Hand-set Learned_Thresholds inside their admissible ranges, with the extremes common. The co-occurrence
# Confidence threshold is often exactly a Laplace Confidence, the "at or above" boundary of 9.8.
_UNIT = st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0))
LSH_THRESHOLD = "lsh.keyset_jaccard"
THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    LSH_THRESHOLD: _UNIT,
    "dbscan.eps": st.one_of(st.sampled_from([1e-9, 0.34, 0.5, 1.0]), st.floats(0.0, 1.0, exclude_min=True)),
    "dbscan.min_samples": st.integers(1, 6),
    "hdbscan.min_cluster_size": st.integers(2, 6),
    MIN_SUPPORT: st.one_of(st.just(1), st.integers(1, 8)),
    MIN_CONFIDENCE: st.one_of(_UNIT, st.integers(1, 12).map(rule_confidence)),
    EXCLUSION_THRESHOLD: _UNIT,
    **{threshold: _UNIT for _, threshold in ANNOTATIONS.values()},
    "tagged.branch_support": st.one_of(st.sampled_from([0.0, 0.1, 0.3, 1.0]), st.floats(0.0, 1.0)),
}
THRESHOLD_TECHNIQUES = tuple(
    tid for tid in REGISTRY.ids() if any(name in THRESHOLD_VALUES for name in REGISTRY.technique(tid).thresholds)
)


@st.composite
def resource_values(draw: st.DrawFn) -> dict[str, int | float]:
    """Small Resource_Parameters for the grouping Techniques, and the soundness pass limit."""
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    return {
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.dbscan"),
        "sample.hdbscan": draw(st.one_of(st.integers(1, 4), st.just(5_000)), label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(1, 6), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
        MAX_PASSES: draw(st.sampled_from([1, 2, 5]), label=MAX_PASSES),
    }


def _tagged_union(branches: Sequence[SchemaNode]) -> bool:
    """Whether the branches fix one common property with const/enum/not (a Discriminator's value groups, 10.4)."""
    properties = [b.kw.get("properties", {}) for b in branches]
    common = set.intersection(*(set(p) for p in properties)) if properties else set()
    return any(all(isinstance(p[name], SchemaNode) and {"const", "enum", "not"} & set(p[name].kw) for p in properties) for name in common)


class StubBundle:
    """A Model_Bundle of hand-set Learned_Thresholds by name; no Decision_Models."""

    def __init__(self, thresholds: Mapping[str, int | float]) -> None:
        self.thresholds = dict(thresholds)

    def model(self, name: str) -> Any | None:
        return None

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


SCENARIOS = ("mdl", "threshold", "drawn")


@dataclass(frozen=True)
class Setup:
    scenario: str
    config: RunConfiguration
    thresholds: Mapping[str, int | float]

    @property
    def mdl(self) -> bool:
        return bool(self.config.techniques[MDL])

    def runs(self, tid: str) -> bool:
        """Whether Technique ``tid`` is implemented and enabled."""
        return REGISTRY.technique(tid).implemented and bool(self.config.techniques[tid])

    @property
    def cooccurrence(self) -> bool:
        """Whether co-occurrence mining is in effect: enabled, with both Learned_Thresholds (20.6)."""
        return self.runs(COOCCURRENCE) and MIN_SUPPORT in self.thresholds and MIN_CONFIDENCE in self.thresholds

    @property
    def field_groups(self) -> bool:
        return any(self.runs(tid) for tid in FIELD_GROUP_TECHNIQUES)


@st.composite
def setups(draw: st.DrawFn, scenario: str) -> Setup:
    """Technique states from ``run_configs()``, small resources, and thresholds present per Technique or absent.

    Without MDL, a grouping Technique resolved by priority often puts each key set in its own Variant, which
    leaves no optional field and so no candidate. The ``threshold`` scenario therefore groups in one of three
    ways: every grouping Technique off (one Variant, 5.10), LSH alone at a near-zero τ (one Variant per set of
    overlapping key sets), or drawn like the rest.
    """
    assert scenario in SCENARIOS
    base: dict[str, Any] = dict(draw(resource_values(), label="resources"))
    grouping = "drawn"
    if scenario != "drawn":
        base[technique_setting(MDL)] = scenario == "mdl"
        base[technique_setting(COOCCURRENCE)] = True
    if scenario == "threshold":
        grouping = draw(st.sampled_from(["one Variant", "LSH near zero", "drawn"]), label="grouping")
        if grouping != "drawn":
            for tid in GROUPING_TECHNIQUES:
                base[technique_setting(tid)] = grouping == "LSH near zero" and tid == MINHASH_LSH
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    if scenario != "drawn":
        present[COOCCURRENCE] = True
    if grouping == "LSH near zero":
        present[MINHASH_LSH] = True
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    if grouping == "LSH near zero":
        thresholds[LSH_THRESHOLD] = draw(st.sampled_from([0.0, 0.05]), label="near-zero τ")
    if scenario == "drawn" and present[COOCCURRENCE] and draw(st.integers(0, 5), label="drop one") == 0:
        del thresholds[draw(st.sampled_from([MIN_SUPPORT, MIN_CONFIDENCE]), label="dropped threshold")]
    return Setup(scenario, config, thresholds)


# ---------------------------------------------------------------------------
# Routing raw instances through the model (before soundness)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Inst:
    """A non-flagged raw node that reaches a subschema."""

    loc: Loc
    value: Any


@dataclass(frozen=True)
class Site:
    """The object schema of one final Variant: its non-flagged objects, Path, and branch index."""

    objects: tuple[Inst, ...]
    path: Path
    index: int
    n_variants: int


def _types(node: SchemaNode) -> list[str]:
    listed = node.kw.get("type", []) if node.boolean is None else []
    return list(listed) if isinstance(listed, list) else [listed]


def _json_type(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int) or (isinstance(value, float) and value.is_integer()):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    return ARRAY if isinstance(value, list) else OBJECT


def _admits(types: Sequence[str], value: Any) -> bool:
    kind = _json_type(value)
    return kind in types or (kind == "integer" and "number" in types)


@dataclass
class Router:
    """Walks the model with the raw instances that reach each subschema (see the module docstring)."""

    case: Case
    store: ObservationStore
    locations: Mapping[ShapePath, list[Loc]]
    analyzer: StructureAnalyzer
    reach: dict[int, list[Inst]] = field(default_factory=dict)
    paths: dict[int, Path] = field(default_factory=dict)
    sites: dict[int, Site] = field(default_factory=dict)
    reached: list[Loc] = field(default_factory=list)  # objects that reached a Record or a Map

    def roots(self) -> list[Inst]:
        return [Inst((d,), doc) for d, doc in enumerate(self.case.docs) if not self.case.excluded((d,))]

    def members(self, objects: Sequence[Inst], key: str) -> list[Inst]:
        """The non-flagged values of ``key`` (an object whose value is flagged still holds the key)."""
        flagged = self.case.flagged_items
        return [Inst((*o.loc, key), o.value[key]) for o in objects if key in o.value and (*o.loc, key) not in flagged]

    def all_members(self, objects: Sequence[Inst]) -> list[Inst]:
        flagged = self.case.flagged_items
        return [Inst((*o.loc, k), v) for o in objects for k, v in o.value.items() if (*o.loc, k) not in flagged]

    def elements(self, arrays: Sequence[Inst]) -> list[Inst]:
        flagged = self.case.flagged_items
        return [Inst((*a.loc, i), v) for a in arrays for i, v in enumerate(a.value) if (*a.loc, i) not in flagged]

    def row_locations(self, view: View) -> list[Loc]:
        return [self.locations[part.column][row] for part in view.parts for row in part.rows.tolist()]

    def final_variants(self, objects: Sequence[Inst], path: Path) -> MergeSplitOutcome:
        """The run's final Variants of exactly ``objects`` at ``path``, from the view of those objects."""
        view = object_rows(View.root(self.store).descend(path))
        wanted = {o.loc for o in objects}
        mask = np.array([loc in wanted for loc in self.row_locations(view)], dtype=bool)
        assert int(mask.sum()) == len(wanted) == len(objects), f"{render_path(path)}: objects missing from the view"
        variants = self.analyzer.variants
        return variants.merge_or_split(variants.select(view.subset(mask)))

    def mark(self, node: SchemaNode, insts: Sequence[Inst], path: Path) -> None:
        assert node.nid not in self.reach, f"{render_path(path)}: node {node.nid} reached twice"
        self.reach[node.nid] = list(insts)
        self.paths[node.nid] = path

    def walk(self, node: SchemaNode, insts: Sequence[Inst], path: Path) -> None:
        self.mark(node, insts, path)
        if not insts or node.boolean is not None:
            return
        objects = [i for i in insts if isinstance(i.value, dict)]
        arrays = [i for i in insts if isinstance(i.value, list)]
        if "type" in node.kw:
            self.typed(node, objects, arrays, path)
            return
        combinator = ANY_OF if ANY_OF in node.kw else ONE_OF
        assert combinator in node.kw, f"{render_path(path)}: neither 'type' nor a combinator in {sorted(node.kw)}"
        branches: list[SchemaNode] = node.kw[combinator]
        holders = [b for b in branches if OBJECT in _types(b)]
        rest = branches
        others = insts
        if len(holders) >= 2:
            # A split Record: its Variant branches come first, then any branch for the other types.
            assert all(a is b for a, b in zip(branches, holders)), f"{render_path(path)}: Variant branches not first"
            self.split(holders, objects, path)
            rest = branches[len(holders) :]
            others = [i for i in insts if not isinstance(i.value, dict)]
        for branch in rest:
            types = _types(branch)
            mine = [i for i in others if _admits(types, i.value)]
            self.mark(branch, mine, path)
            branch_objects = [i for i in mine if isinstance(i.value, dict)]
            self.typed(branch, branch_objects, [i for i in mine if isinstance(i.value, list)], path)

    def typed(self, node: SchemaNode, objects: Sequence[Inst], arrays: Sequence[Inst], path: Path) -> None:
        if objects:
            if any(o.value for o in objects) and "properties" not in node.kw:
                self.map(node, objects, path)
            else:
                outcome = self.final_variants(objects, path)
                assert outcome.n_variants == 1, f"{render_path(path)}: {outcome.n_variants} Variants, one object schema"
                self.record(node, objects, path, 0, 1)
        if arrays:
            self.array(node, arrays, path)

    def split(self, branches: Sequence[SchemaNode], objects: Sequence[Inst], path: Path) -> None:
        """A split Record: assign the objects to branches by the final Variants' views, or by a Discriminator (10)."""
        by_loc = {o.loc: o for o in objects}
        if _tagged_union(branches):
            compiled = [VALIDATOR.compile(branch.to_json()) for branch in branches]
            groups = [[o for o in objects if VALIDATOR.is_valid(c, o.value)] for c in compiled]
            views = tuple(groups)  # only the count matters below
        else:
            views = self.final_variants(objects, path).views()
            assert len(views) == len(branches), f"{render_path(path)}: {len(views)} Variants for {len(branches)} branches"
            groups = [[by_loc[loc] for loc in self.row_locations(view)] for view in views]
        assigned = Counter(o.loc for g in groups for o in g)
        assert assigned == Counter(o.loc for o in objects), f"{render_path(path)}: Variants ≠ a partition"
        for i, (branch, group) in enumerate(zip(branches, groups)):
            self.mark(branch, group, path)
            self.record(branch, group, path, i, len(views))

    def record(self, node: SchemaNode, objects: Sequence[Inst], path: Path, index: int, n_variants: int) -> None:
        self.sites[node.nid] = Site(tuple(objects), path, index, n_variants)
        self.reached.extend(o.loc for o in objects)
        properties: Mapping[str, SchemaNode] = node.kw.get("properties", {})
        keys = frozenset().union(*(frozenset(o.value) for o in objects))
        assert set(properties) == keys, f"{render_path(path)}: properties {sorted(properties)} ≠ keys {sorted(keys)}"
        for key in sorted(keys):
            self.walk(properties[key], self.members(objects, key), (*path, key_step(key)))

    def map(self, node: SchemaNode, objects: Sequence[Inst], path: Path) -> None:
        values = node.kw.get("additionalProperties")
        assert isinstance(values, SchemaNode), f"{render_path(path)}: a Map without a value schema"
        self.reached.extend(o.loc for o in objects)
        self.walk(values, self.all_members(objects), (*path, MAPVAL_STEP))

    def array(self, node: SchemaNode, arrays: Sequence[Inst], path: Path) -> None:
        elements = self.elements(arrays)
        child = (*path, ELEM_STEP)
        prefix = node.kw.get("prefixItems")
        items = node.kw.get("items")
        if prefix is not None:
            for position, schema in enumerate(prefix):
                self.walk(schema, [e for e in elements if e.loc[-1] == position], child)
            beyond = [e for e in elements if e.loc[-1] >= len(prefix)]
            if isinstance(items, SchemaNode):
                self.walk(items, beyond, child)
            else:
                assert not beyond, f"{render_path(path)}: elements beyond the prefix"
        elif isinstance(items, SchemaNode):
            self.walk(items, elements, child)
        else:
            assert not elements, f"{render_path(path)}: {len(elements)} elements without 'items' or 'prefixItems'"


# ---------------------------------------------------------------------------
# Snapshots of the analyzer's output (before soundness)
# ---------------------------------------------------------------------------

Entries = dict[str, tuple[str, ...]]  # antecedent → consequents


def node_entries(model: SchemaModel) -> dict[int, Entries]:
    """Every ``dependentRequired`` of the model, by node id."""
    out: dict[int, Entries] = {}
    for _, node in model.walk():
        if node.boolean is None and DEPENDENT_REQUIRED in node.kw:
            listed = node.kw[DEPENDENT_REQUIRED]
            assert isinstance(listed, dict) and listed, f"node {node.nid}: 'dependentRequired' is {listed!r}"
            out[node.nid] = {a: tuple(bs) for a, bs in listed.items()}
    return out


def node_required(model: SchemaModel) -> dict[int, frozenset[str]]:
    return {
        node.nid: frozenset(node.kw.get(REQUIRED, ()))
        for _, node in model.walk()
        if node.boolean is None and OBJECT in _types(node)
    }


def rule_label(rule: Rule) -> str:
    """The outcome label of a rule's decision: its entry as compact JSON."""
    return json.dumps({rule[0]: [rule[1]]}, ensure_ascii=False, separators=(",", ":"))


def selected_decisions(report: InferenceReport) -> Counter[tuple[str, str, str]]:
    """The rules the report's ``cooccurrence_rule`` decisions selected, as ``(Path, antecedent, consequent)``."""
    out: Counter[tuple[str, str, str]] = Counter()
    for entry in report.decisions:
        if entry.kind != COOCCURRENCE_RULE:
            continue
        resolution = entry.resolution.to_json()
        candidates = resolution.get("candidates") or []
        if not candidates or resolution.get("selected") is None:
            continue
        chosen = candidates[resolution["selected"]]
        if chosen.get("technique") != COOCCURRENCE:
            continue
        ((a, bs),) = json.loads(chosen["outcome"]).items()
        (b,) = bs
        out[(entry.path, a, b)] += 1
    return out


@dataclass(frozen=True)
class Snapshot:
    """The analyzer's rule output, taken before soundness edits the model in place."""

    entries: dict[int, Entries]
    required: dict[int, frozenset[str]]
    records: dict[ConstraintId, ProvenanceRecord]
    decisions: Counter[tuple[str, str, str]]

    @classmethod
    def of(cls, model: SchemaModel, report: InferenceReport) -> Snapshot:
        records = {cid: r for cid, r in model.provenance.items() if cid.keyword == DEPENDENT_REQUIRED}
        return cls(node_entries(model), node_required(model), records, selected_decisions(report))


# ---------------------------------------------------------------------------
# Rule algebra
# ---------------------------------------------------------------------------


def closure(start: Iterable[str], edges: Iterable[Rule]) -> set[str]:
    """Every field reachable from ``start`` by chaining ``edges``."""
    successors: dict[str, list[str]] = defaultdict(list)
    for a, b in edges:
        successors[a].append(b)
    seen = set(start)
    todo = list(seen)
    while todo:
        for nxt in successors[todo.pop()]:
            if nxt not in seen:
                seen.add(nxt)
                todo.append(nxt)
    return seen


def derivable(rule: Rule, edges: Iterable[Rule], required: Iterable[str]) -> bool:
    """Whether ``rule`` follows by chaining ``edges`` plus ``x → r`` for every required ``r`` (see Readings)."""
    a, b = rule
    return b in closure({a, *required}, edges)


def reference_candidates(keysets: Sequence[frozenset[str]]) -> dict[Rule, int]:
    """Brute-force 9.3 candidates of a Variant with their Support counts: a ≠ b, rows(a) ⊆ rows(b), b optional."""
    n = len(keysets)
    count = Counter(k for ks in keysets for k in ks)
    out: dict[Rule, int] = {}
    for a in count:
        holders = [ks for ks in keysets if a in ks]
        for b in count:
            if a != b and count[b] < n and all(b in ks for ks in holders):
                out[(a, b)] = count[a]
    return out


def pairs_of(entries: Mapping[str, Sequence[str]]) -> list[Rule]:
    return [(a, b) for a, bs in entries.items() for b in bs]


# ---------------------------------------------------------------------------
# The checks
# ---------------------------------------------------------------------------


def resolve(document: Any, pointer: str) -> Any:
    value = document
    for token in split_pointer(pointer):
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def key_scope(value: Any) -> str:
    """The scope an ``x-key`` annotation states: the value itself or its ``scope`` member (12.8)."""
    scope = value if isinstance(value, str) else value.get("scope") if isinstance(value, dict) else None
    assert scope in ("collection", "array"), (
        f"Property 19 cannot read the scope of x-key {value!r}; update key_scope for the emitted form"
    )
    return scope


def referenced_path(value: Any) -> Path:
    """The Path an ``x-references`` annotation identifies: a Path string or its ``path`` member (12.8)."""
    text = value if isinstance(value, str) else value.get("path") if isinstance(value, dict) else None
    assert isinstance(text, str), f"Property 19 cannot read x-references {value!r}; update referenced_path"
    return parse_path(text)


def dependency_paths(value: Any, base: Path) -> list[tuple[tuple[str, ...], tuple[str, ...]]]:
    """The (determinant, dependent) key paths below ``base`` of an ``x-functional-dependency`` annotation."""
    items = value if isinstance(value, list) else [value]
    out = []
    for item in items:
        ends = [item.get(name) if isinstance(item, dict) else None for name in ("determinant", "dependent")]
        assert all(isinstance(end, str) and end.startswith("$") for end in ends), (
            f"Property 19 cannot read x-functional-dependency {value!r}; update dependency_paths"
        )
        steps = [parse_path(end) for end in ends]
        assert all(path[: len(base)] == base and all(s.kind == KEY for s in path[len(base) :]) for path in steps), (
            f"x-functional-dependency {value!r} names Paths outside the object at {render_path(base)}"
        )
        out.append(tuple(tuple(s.key for s in path[len(base) :]) for path in steps))  # type: ignore[arg-type]
    return out


def required_sets(node: SchemaNode) -> list[list[str]]:
    """The ``required`` lists of a ``not`` subschema and of its combinator branches (11.6)."""
    if node.boolean is not None:
        return []
    found = [list(node.kw[REQUIRED])] if isinstance(node.kw.get(REQUIRED), list) else []
    for keyword in (ANY_OF, ONE_OF, "allOf"):
        for branch in node.kw.get(keyword, []):
            if branch.boolean is None and isinstance(branch.kw.get(REQUIRED), list):
                found.append(list(branch.kw[REQUIRED]))
    return found


def combine(base: Mapping[str, Any], names: Iterable[str], donors: Sequence[Inst]) -> dict[str, Any]:
    """``base`` extended with ``names``, each taking the value of the first donor object that holds it."""
    out = dict(base)
    for name in names:
        if name not in out:
            out[name] = next((d.value[name] for d in donors if name in d.value), None)
    return out


@dataclass
class RuleChecker:
    """The Property 19 clauses over one run (see the module docstring)."""

    setup: Setup
    case: Case
    router: Router
    pre: Snapshot
    model: SchemaModel
    soundness: SoundnessResult
    document: Any
    pointers: Mapping[ConstraintId, str]
    notes: set[str] = field(default_factory=set)
    # Per Variant node: the pairs of its entries before soundness, its given (Field_Group) pairs, and required.
    edges: dict[int, tuple[list[Rule], frozenset[str]]] = field(default_factory=dict)

    @property
    def thresholds(self) -> Mapping[str, int | float]:
        return self.setup.thresholds

    def check(self) -> None:
        final = node_entries(self.model)
        for nid in (*self.pre.entries, *final):
            assert nid in self.router.sites, f"node {nid}: 'dependentRequired' outside a Variant's object schema"
        if not self.setup.cooccurrence:
            assert not any(r.technique == COOCCURRENCE for r in self.pre.records.values()), "rules while out of effect"
            assert not self.pre.decisions, "rules selected while co-occurrence mining is out of effect (9.9, 20.6)"
        final_required = node_required(self.model)
        selected: Counter[tuple[str, str, str]] = Counter()
        for nid, site in sorted(self.router.sites.items()):
            selected.update(self.check_variant(nid, site, final.get(nid, {}), final_required.get(nid, frozenset())))
        self.check_report(selected)
        self.check_soundness(final)
        self.check_exclusions()
        self.check_annotations()
        self.check_unique_items()
        self.check_absences()

    # -- co-occurrence rules (9.3–9.6, 9.8) -------------------------------------

    def check_variant(
        self, nid: int, site: Site, final: Entries, final_required: frozenset[str]
    ) -> Counter[tuple[str, str, str]]:
        where = f"{render_path(site.path)} (Variant {site.index} of {site.n_variants})"
        keysets = [frozenset(o.value) for o in site.objects]
        n = len(keysets)
        count = Counter(k for ks in keysets for k in ks)
        always = frozenset.intersection(*keysets) if keysets else frozenset()
        entries = self.pre.entries.get(nid, {})
        required = self.pre.required.get(nid, frozenset())
        assert required == always, f"{where}: required {sorted(required)} ≠ always present {sorted(always)}"
        rules, given, selected = self.read_entries(where, nid, site, entries, count)

        # Holds on every non-flagged object of the Variant, before and after soundness.
        for label, listed in (("analyzer", entries), ("emitted", final)):
            for a, bs in listed.items():
                for o, ks in zip(site.objects, keysets):
                    missing = [b for b in bs if b not in ks] if a in ks else []
                    assert not missing, f"{where}: {label} entry {a!r} → {list(bs)} fails on {o.loc}, lacks {missing}"

        # Every rule relates optional fields; the antecedent's Support meets cooccur.min_support (9.6).
        for a, b in [*rules, *selected]:
            assert a != b and count[a] < n and count[b] < n, f"{where}: rule {a!r} → {b!r} names a required field"
            assert count[a] >= self.thresholds[MIN_SUPPORT], f"{where}: Support {count[a]} of {a!r} below the threshold"

        # The selection against the brute-force candidates.
        reference = reference_candidates(keysets)
        if reference:
            self.notes.add("a Variant with candidates")
        if self.setup.cooccurrence:
            kept = {rule for rule, support in reference.items() if support >= self.thresholds[MIN_SUPPORT]}
            assert set(selected) <= kept, f"{where}: selected {sorted(set(selected) - kept)} are no kept candidates"
            if not self.setup.mdl and not given:
                expected = {r for r in kept if rule_confidence(reference[r]) >= self.thresholds[MIN_CONFIDENCE]}
                assert set(selected) == expected, (
                    f"{where}: selected {sorted(selected)} ≠ candidates meeting both thresholds {sorted(expected)}"
                )
        else:
            assert not rules and not selected, f"{where}: rules while co-occurrence mining is out of effect"

        # 9.5 (a): the analyzer's emitted rules, the given entries, and required derive every selected rule.
        edges = [*rules, *given]
        for rule in selected:
            assert derivable(rule, edges, required), f"{where}: selected {rule} is not derived by {edges}"
        # 9.5 (b): no emitted rule follows from the other pairs of its node and required, before and after soundness.
        for label, listed, req in (("analyzer", entries, required), ("emitted", final, final_required)):
            pairs = pairs_of(listed)
            for rule in (r for r in rules if r in pairs):
                others = [p for p in pairs if p != rule]
                assert not derivable(rule, others, req), f"{where}: {label} rule {rule} follows from {others}"
        self.edges[nid] = (edges, required)
        self.note_variant(site, rules, selected, entries, final)
        return Counter((render_path(site.path), a, b) for a, b in selected)

    def read_entries(
        self, where: str, nid: int, site: Site, entries: Entries, count: Counter[str]
    ) -> tuple[list[Rule], list[Rule], list[Rule]]:
        """The co-occurrence rules, given (Field_Group) pairs, and selected rules of a node, from its records."""
        rules: list[Rule] = []
        given: list[Rule] = []
        selected: list[Rule] = []
        n = len(site.objects)
        for a, bs in entries.items():
            record = self.pre.records.get(ConstraintId(nid, DEPENDENT_REQUIRED, a))
            assert record is not None, f"{where}: no Provenance_Record for the entry {a!r}"
            details = record.details
            listed = list(details.get("rules", [])) if record.technique == COOCCURRENCE else []
            consequents = [item["consequent"] for item in listed]
            assert len(set(consequents)) == len(consequents) and set(consequents) <= set(bs), (
                f"{where}: the record of {a!r} lists rules {consequents} for the entry {list(bs)}"
            )
            others = [b for b in bs if b not in consequents]
            if others:
                assert self.setup.field_groups, (
                    f"{where}: {a!r} → {others} come from {record.technique}, not co-occurrence, "
                    "and no Field_Group Technique runs"
                )
                given.extend((a, b) for b in others)
            if record.technique != COOCCURRENCE:
                continue
            assert details["antecedent"] == a and details["variant"] == site.index, f"{where}: details of {a!r}"
            assert details["support_count"] == count[a], f"{where}: Support count of {a!r} ≠ {count[a]} objects"
            expected = {MIN_SUPPORT: self.thresholds[MIN_SUPPORT]}
            if not self.setup.mdl:
                expected[MIN_CONFIDENCE] = self.thresholds[MIN_CONFIDENCE]
            read = dict(details["thresholds"])
            assert read == expected, f"{where}: thresholds read {read} ≠ {expected}"
            assert (record.support.consistent, record.support.total) == (n, n), f"{where}: Support {record.support}"
            confidences = [self.check_rule(where, (a, item["consequent"]), item, count[a]) for item in listed]
            assert record.confidence == min(confidences), f"{where}: entry {a!r} Confidence ≠ its weakest rule's"
            assert record.cutoff is not None and record.cutoff.source == (
                MDL_COMPARISON if self.setup.mdl else MIN_CONFIDENCE
            ), f"{where}: entry {a!r} cut-off {record.cutoff}"
            implied = list(details.get("implied", []))
            assert not set(implied) & set(bs), f"{where}: implied rules of {a!r} are emitted"
            rules.extend((a, item["consequent"]) for item in listed)
            selected.extend((a, b) for b in [*consequents, *implied])
        return rules, given, selected

    def check_rule(self, where: str, rule: Rule, item: Mapping[str, Any], support: int) -> float:
        """The cut-off of one emitted rule (see Readings, Thresholds); returns its Confidence."""
        label = rule_label(rule)
        resolution = item["resolution"]
        chosen = resolution["candidates"][resolution["selected"]]
        assert (chosen["outcome"], chosen.get("technique")) == (label, COOCCURRENCE), f"{where}: {rule} not selected"
        confidence = item["confidence"]
        cutoff = item["cutoff"]
        if self.setup.mdl:
            assert item["method"] == "mdl" and cutoff["source"] == MDL_COMPARISON, f"{where}: {rule} cut-off {cutoff}"
            scores = [score["score"] for score in cutoff["scores"]]
            outcomes = [score["outcome"] for score in cutoff["scores"]]
            assert outcomes == [NO_RULE, label], f"{where}: {rule} compared {outcomes}"
            assert scores[1] < scores[0], f"{where}: {rule} does not lower the description length: {scores}"
            assert confidence == mdl_posterior(scores)[1] >= 0.5, f"{where}: {rule} Confidence {confidence}"
        else:
            threshold = self.thresholds[MIN_CONFIDENCE]
            assert item["method"] == "priority", f"{where}: {rule} resolved by {item['method']}"
            assert cutoff == {"source": MIN_CONFIDENCE, "value": threshold}, f"{where}: {rule} cut-off {cutoff}"
            assert confidence == rule_confidence(support) >= threshold, f"{where}: {rule} Confidence {confidence}"
        return confidence

    def check_report(self, selected: Counter[tuple[str, str, str]]) -> None:
        """Per Path, the rules selected in the records are the report's rule-selecting decisions."""
        extra = selected - self.pre.decisions
        assert not extra, f"selected rules without a decision in the report: {sorted(extra)}"
        leftover = self.pre.decisions - selected
        if not self.setup.field_groups:
            assert not leftover, f"decisions that select rules no record lists: {sorted(leftover)}"
        for path, a, b in sorted(leftover):
            sites = [nid for nid, site in self.router.sites.items() if render_path(site.path) == path]
            assert any(derivable((a, b), *self.edges[nid]) for nid in sites if nid in self.edges), (
                f"{path}: the selected rule {a!r} → {b!r} is derived in no Variant"
            )

    def check_soundness(self, final: dict[int, Entries]) -> None:
        """Soundness only removes whole entries, each recorded; remaining entries keep records and pointers."""
        index = self.model.index()
        named = {cid for applied in self.soundness.changes for cid in applied.change.before_ids}
        for nid, entries in final.items():
            assert nid in self.pre.entries, f"node {nid}: soundness added 'dependentRequired'"
            for a, bs in entries.items():
                before = self.pre.entries[nid].get(a)
                assert before is not None and sorted(bs) == sorted(before), f"node {nid}: entry {a!r} was edited"
                cid = ConstraintId(nid, DEPENDENT_REQUIRED, a)
                assert cid in self.model.provenance, f"{cid}: the emitted entry lost its Provenance_Record"
                assert resolve(self.document, self.pointers[cid]) == sorted_codepoint(bs), f"{cid}: emitted entry"
        for nid, entries in self.pre.entries.items():
            for a in entries:
                if a in final.get(nid, {}):
                    continue
                self.notes.add("rule entry removed by soundness")
                if nid in index:
                    assert ConstraintId(nid, DEPENDENT_REQUIRED, a) in named, f"node {nid}: {a!r} removed silently"
                else:
                    assert self.soundness.changes, f"node {nid} left the model without a soundness change"

    # -- gated clauses (Exclusion_Rules, keys, references, FDs, uniqueItems) ------------------------

    def objects_at(self, node: SchemaNode, what: str) -> list[Inst]:
        assert node.nid in self.router.reach, f"{what} on node {node.nid}, which the analyzer did not emit"
        return [i for i in self.router.reach[node.nid] if isinstance(i.value, dict)]

    def check_cutoff(self, what: str, record: ProvenanceRecord, threshold: str, mdl: bool) -> None:
        """The record meets its Learned_Threshold on Confidence, or (``mdl``) won an MDL comparison strictly."""
        cutoff = record.cutoff
        if mdl and cutoff is not None and cutoff.source == MDL_COMPARISON:
            assert record.resolution is not None, f"{what}: an MDL cut-off without a resolution"
            scores = [c.score for c in record.resolution.candidates]
            won = scores[record.resolution.selected]
            assert all(won < s for i, s in enumerate(scores) if i != record.resolution.selected), f"{what}: {scores}"
            return
        assert threshold in self.thresholds, f"{what}: emitted without its Learned_Threshold {threshold}"
        assert record.confidence >= self.thresholds[threshold], (
            f"{what}: Confidence {record.confidence} below {threshold} = {self.thresholds[threshold]}"
        )

    def check_exclusions(self) -> None:
        """11.5, 11.6: every Exclusion_Rule holds and rejects a combination of two members."""
        exclusion_branches: dict[int, list[int]] = defaultdict(list)
        for cid, record in self.model.provenance.items():
            if record.technique == EXCLUSION and cid.keyword == ONE_OF:
                exclusion_branches[cid.nid].append(int(cid.item))  # type: ignore[arg-type]
        for pointer, node in self.model.walk():
            if node.boolean is not None:
                continue
            if "not" in node.kw:
                record = self.model.provenance.get(ConstraintId(node.nid, "not"))
                assert record is not None and record.technique == EXCLUSION, f"{pointer}: a 'not' from {record}"
                assert self.setup.runs(EXCLUSION), f"{pointer}: an Exclusion_Rule while {EXCLUSION} does not run"
                self.check_cutoff(f"{pointer}/not", record, EXCLUSION_THRESHOLD, mdl=self.setup.mdl)
                self.check_not_form(pointer, node)
            if node.nid in exclusion_branches:
                assert self.setup.runs(EXCLUSION), f"{pointer}: an Exclusion_Rule while {EXCLUSION} does not run"
                for i in exclusion_branches[node.nid]:
                    record = self.model.provenance[ConstraintId(node.nid, ONE_OF, i)]
                    self.check_cutoff(f"{pointer}/oneOf/{i}", record, EXCLUSION_THRESHOLD, mdl=self.setup.mdl)
                self.check_one_of_form(pointer, node)

    def check_not_form(self, pointer: str, node: SchemaNode) -> None:
        objects = self.objects_at(node, f"{pointer}/not")
        sub = node.kw["not"]
        compiled = VALIDATOR.compile({"not": sub.to_json()})
        for o in objects:
            assert VALIDATOR.is_valid(compiled, o.value), f"{pointer}: the Exclusion_Rule fails on {o.loc}"
        sets = required_sets(sub)
        assert sets, f"Property 19 cannot read the members of the 'not' at {pointer}: {sub.to_json()}"
        for names in sets:
            if objects:
                combined = combine(objects[0].value, names, objects)
                assert not VALIDATOR.is_valid(compiled, combined), f"{pointer}: accepts {combined} (members {names})"
        self.notes.add("Exclusion_Rule (not)")

    def check_one_of_form(self, pointer: str, node: SchemaNode) -> None:
        objects = self.objects_at(node, f"{pointer}/oneOf")
        branches = node.kw[ONE_OF]
        compiled = VALIDATOR.compile({ONE_OF: [b.to_json() for b in branches]})
        for o in objects:
            assert VALIDATOR.is_valid(compiled, o.value), f"{pointer}: {o.loc} is not in exactly one member"
        members = [list(b.kw.get(REQUIRED, [])) if b.boolean is None else [] for b in branches]
        for i, first in enumerate(members):
            alone = VALIDATOR.compile(branches[i].to_json())
            base = next((o for o in objects if VALIDATOR.is_valid(alone, o.value)), None)
            for j, second in enumerate(members):
                if i == j or not first or not second or base is None:
                    continue
                combined = combine(base.value, second, objects)
                assert not VALIDATOR.is_valid(compiled, combined), f"{pointer}: accepts {combined} (members {i}, {j})"
        self.notes.add("Exclusion_Rule (oneOf)")

    def check_annotations(self) -> None:
        """12.1–12.6, 12.8: every ``x-key``, ``x-references``, and ``x-functional-dependency`` holds."""
        index = self.model.index()
        for pointer, node in self.model.walk():
            if node.boolean is not None:
                continue
            for keyword, (technique, threshold) in ANNOTATIONS.items():
                if keyword not in node.kw:
                    continue
                what = f"{pointer}/{keyword}"
                record = self.model.provenance.get(ConstraintId(node.nid, keyword))
                assert record is not None and record.technique == technique, f"{what}: produced by {record}"
                assert self.setup.runs(technique), f"{what}: emitted while {technique} does not run"
                self.check_cutoff(what, record, threshold, mdl=False)
                assert node.nid in self.router.reach, f"{what}: on a node the analyzer did not emit"
                insts = self.router.reach[node.nid]
                value = node.kw[keyword]
                if keyword == "x-key":
                    self.check_key(what, insts, key_scope(value))
                elif keyword == "x-references":
                    self.check_reference(what, insts, referenced_path(value), index)
                else:
                    self.check_dependency(what, insts, dependency_paths(value, self.router.paths[node.nid]))
                self.notes.add(keyword)

    def check_key(self, what: str, insts: Sequence[Inst], scope: str) -> None:
        groups: dict[Loc, list[Any]] = defaultdict(list)
        for inst in insts:
            assert not isinstance(inst.value, (dict, list)), f"{what}: a Key_Constraint on a non-scalar at {inst.loc}"
            if scope == "collection":
                group: Loc = ()
            else:
                positions = [j for j in range(1, len(inst.loc)) if isinstance(inst.loc[j], int)]
                assert positions, f"{what}: an array-scope key outside arrays at {inst.loc}"
                group = inst.loc[: positions[-1]]
            groups[group].append(canonical_key(inst.value))
        for group, keys in groups.items():
            assert len(keys) == len(set(keys)), f"{what}: repeated values within {group or 'the collection'}"

    def check_reference(self, what: str, insts: Sequence[Inst], target: Path, index: Mapping[int, SchemaNode]) -> None:
        nids = [nid for nid, path in self.router.paths.items() if path == target and nid in index]
        assert any("x-key" in index[nid].kw for nid in nids if index[nid].boolean is None), (
            f"{what}: {render_path(target)} carries no Key_Constraint"
        )
        values = {canonical_key(i.value) for nid in nids for i in self.router.reach[nid]}
        for inst in insts:
            assert canonical_key(inst.value) in values, f"{what}: {inst.value!r} at {inst.loc} is not referenced"

    def check_dependency(self, what: str, insts: Sequence[Inst], pairs: Sequence[tuple[tuple[str, ...], ...]]) -> None:
        def value_at(o: Inst, keys: tuple[str, ...]) -> tuple[bool, Any]:
            value, loc = o.value, o.loc
            for key in keys:
                if not isinstance(value, dict) or key not in value:
                    return False, None
                value, loc = value[key], (*loc, key)
            return not self.case.excluded(loc), value

        for determinant, dependent in pairs:
            seen: dict[Any, set[Any]] = defaultdict(set)
            for o in (i for i in insts if isinstance(i.value, dict)):
                (has_a, a), (has_b, b) = value_at(o, determinant), value_at(o, dependent)
                if has_a and has_b:
                    seen[canonical_key(a)].add(canonical_key(b))
            assert all(len(bs) == 1 for bs in seen.values()), f"{what}: {determinant} does not determine {dependent}"

    def check_unique_items(self) -> None:
        """12.7, 12.8: ``uniqueItems`` only on a List of scalars whose elements carry ``x-key``."""
        for pointer, node in self.model.walk():
            if node.boolean is not None or "uniqueItems" not in node.kw:
                continue
            items = node.kw.get("items")
            assert "prefixItems" not in node.kw and isinstance(items, SchemaNode), f"{pointer}: not a List"
            assert items.boolean is None and "x-key" in items.kw, f"{pointer}: no Key_Constraint on the elements"
            assert items.nid in self.router.reach, f"{pointer}: elements of a List the analyzer did not emit"
            elements = self.router.reach[items.nid]
            assert all(not isinstance(e.value, (dict, list)) for e in elements), f"{pointer}: non-scalar elements"
            self.notes.add("uniqueItems")

    def check_absences(self) -> None:
        """Unimplemented rule Techniques produce no record and none of their keywords."""
        for tid, keywords in EMITTED_BY.items():
            if REGISTRY.technique(tid).implemented:
                continue
            assert not any(r.technique == tid for r in self.model.provenance.values()), f"a record names {tid}"
            for pointer, node in self.model.walk():
                found = [k for k in keywords if node.boolean is None and k in node.kw]
                assert not found, f"{pointer}: {found} although {tid} is unimplemented"

    # -- statistics -----------------------------------------------------------

    def note_variant(
        self, site: Site, rules: Sequence[Rule], selected: Sequence[Rule], entries: Entries, final: Entries
    ) -> None:
        if not rules:
            return
        self.notes.add("rules selected by MDL" if self.setup.mdl else "rules selected by the Confidence threshold")
        self.notes.add("1 rule emitted" if len(rules) == 1 else "2 or more rules emitted")
        if len(selected) > len(rules):
            self.notes.add("implied rules dropped")
        if any(derivable((b, a), rules, ()) for a, b in rules):
            self.notes.add("cycle emitted (strongly connected fields)")
        if site.n_variants > 1:
            self.notes.add("rules in a split Record's branch")
        if site.path:
            self.notes.add("rules in a nested Record")
        flagged = self.case.flagged_items
        fields = {name for rule in rules for name in rule}
        if any((*o.loc, key) in flagged for o in site.objects for key in o.value if key in fields):
            self.notes.add("a rule field holds a flagged value")
        if entries != final:
            self.notes.add("entries changed by soundness")

    def note_case(self) -> None:
        if not self.setup.cooccurrence:
            event("co-occurrence out of effect")
        elif not self.pre.records:
            event("co-occurrence in effect, no rule emitted")
        if self.case.spoiled_items:
            event("flagged spoiled element")
        if self.soundness.changes:
            event("soundness changed the schema")
        for note in sorted(self.notes):
            event(note)


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


@given(setup=setups("mdl"), case=cases(20, 60))
def test_rules_selected_by_mdl_hold_and_are_irredundant(setup: Setup, case: Case) -> None:
    """9.4: MDL selects; larger collections, since a rule must save more bits than its entry costs."""
    run_property(setup, case)


@given(setup=setups("threshold"), case=cases())
def test_rules_selected_by_the_confidence_threshold_hold_and_are_irredundant(setup: Setup, case: Case) -> None:
    """9.8: ``decide.mdl`` off, so ``cooccur.min_confidence`` selects and the selection is checked exactly."""
    run_property(setup, case)


@given(setup=setups("drawn"), case=cases())
def test_emitted_rules_hold_and_are_irredundant_under_any_technique_states(setup: Setup, case: Case) -> None:
    """Every implemented Technique drawn, the co-occurrence thresholds present, absent, or one of them missing."""
    run_property(setup, case)


def run_property(setup: Setup, case: Case) -> None:
    store, locations = build(case)
    config = setup.config
    report = InferenceReport()
    analyzer = StructureAnalyzer.for_techniques(
        config.techniques,
        bundle=StubBundle(setup.thresholds),
        report=report,
        validator=VALIDATOR,
        resources=config.resources,
        seed=config.seed,
    )
    before = flag_state(store)
    model = analyzer.infer_model(View.root(store))
    pre = Snapshot.of(model, report)

    router = Router(case, store, locations, analyzer)
    router.walk(model.root, router.roots(), ROOT)
    assert flag_state(store) == before, "the walk changed the flags, so the Variant views would differ"
    assert Counter(router.reached) == Counter(case.live_objects()), "objects reached ≠ non-flagged objects"

    soundness = check_soundness(
        model,
        list(case.docs),
        engine=analyzer.engine,
        validator=VALIDATOR,
        report=report,
        flagged=flagged_documents(store),
        max_passes=config.resources[MAX_PASSES],
    )
    document, pointers = SchemaEmitter().layout(model)

    checker = RuleChecker(setup, case, router, pre, model, soundness, document, pointers)
    checker.check()
    checker.note_case()
