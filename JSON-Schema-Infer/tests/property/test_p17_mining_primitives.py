"""Property 17: Mining primitives equal brute-force references (task 14.3).

For any small generated object set, the mining primitives match exhaustive
reference implementations (with exact Jaccard in place of MinHash when the
permutation count is maximal). This module covers the primitives
implemented so far: LSH candidate groups (5.7) and Co_Occurrence_Rule
candidates (9.3). Tasks 33.3 (FP-growth frequent itemsets), 35.3 (maximal
Exclusion_Rule candidates), and 44.2 (Key_Constraint, Reference_Constraint,
and Functional_Dependency candidates) add their primitives as further tests
at the end of this module. They can draw the same object sets with
:func:`cases` and read the reference from :attr:`Case.reference`.

Object sets and the reference
    Objects sit at ``$``, ``$.v``, or ``$.v[*]``. Each test runs once per
    layout, so each layout gets the full example budget. Next to the objects
    are rows at the Path that are not objects (scalars, arrays, and arrays
    of objects one level deeper) and documents without ``v``. The reference
    is built from the raw documents, independently of the ObservationStore.
    It holds every non-flagged object at the Path, keyed by ``(document
    index, array position)`` (position ``-1`` outside arrays), which are the
    ``doc`` and ``pos`` fields of a view row. From each object it derives
    the key set and the Object_Signature: the keys with the JSON type of
    each value, where integers and other numbers are one type (``number``).

    Key sets come from a pool with one of three shapes:

    - ``blocks``: pairwise identical or disjoint key sets, like Field_Groups
      (one to four blocks, plus maybe the empty key set);
    - ``nested``: prefix chains, where one-way implications are common, plus
      at most one key set off the chain;
    - ``overlapping``: two to five arbitrary subsets of up to five keys.

    The first two objects take different key sets from the pool, and the
    rest are drawn with repetition.

    Keys come from :data:`tests.strategies.KEYS`, which includes the empty
    key and astral and non-ASCII keys. Each key set in the pool has base
    types, and an object may retype one key, so one key set often has
    several signatures. ``null`` values count as present.

    Flags follow Property 11. Spoiler documents with arbitrary key sets are
    always flagged, and up to two other documents may be flagged too. Up to
    two objects at the Path are flagged as items. Up to two member values of
    non-flagged objects are flagged as items, and those objects keep the
    keys (17.4). At ``$.v[*]``, a whole ``v`` array may be flagged, which
    excludes its elements.

LSH candidate groups (5.7): the exact reading
    :func:`minhash_lsh_grouping` sketches the key set of each signature with
    ``minhash.perm`` datasketch hash functions. LSH retrieves candidate
    pairs from ``lsh.bands`` bands, or from equal sketches when
    ``lsh.bands = 1``. A retrieved pair whose estimated Jaccard similarity
    is at or above τ = ``lsh.keyset_jaccard`` is an edge, and the connected
    components are the initial groups. Groups are then merged greedily
    while the description length decreases, along the links of the maximum
    spanning forest of the retrieved pairs.

    The MinHash estimate equals the exact Jaccard similarity of two key sets
    only in the limit of every permutation, so it cannot be compared with
    the exact value directly. It is exact where it does not depend on the
    hash functions drawn. Each datasketch hash function applies an odd
    affine map modulo 2^32 to the MurmurHash3 finalizer of a 32-bit SHA-1
    key hash. Both maps are bijections, and the test checks that the key
    hashes of the alphabet are distinct (:data:`KEY_HASHES`). So every hash
    function orders the keys strictly, and two key sets agree on it exactly
    when the first key of their union is a common key. For every
    permutation count, band count, and seed, it follows that:

    - identical key sets (J = 1) have identical sketches, so they are
      retrieved in every band with estimate exactly 1 ≥ τ;
    - key sets with no common key (J = 0) agree on no hash value, so the
      estimate is 0 and LSH never retrieves them.

    Only pairs with 0 < J < 1 get an estimate, and a retrieval, that depends
    on the hash functions drawn. The tests compare in two ways:

    1. ``test_lsh_candidate_groups_equal_exact_jaccard_components``: when
       key sets are pairwise identical or disjoint (``blocks``), every
       estimate is exact. The reference is the brute-force connected
       components of the exact graph, whose edges are the signature pairs
       with J > 0 (LSH retrieval in this regime) and J ≥ τ. Every retrieved
       pair is then an edge, so no spanning-forest link joins two groups,
       and the greedy merge has nothing to merge. The grouping's groups
       must therefore equal the reference components for every τ, whatever
       ``minhash.perm``, ``lsh.bands``, and seed. The details must count
       the components, edges, retrieved pairs, links, and merges (none) to
       match.
    2. ``test_lsh_candidate_groups_lie_between_exact_key_set_classes_and_overlap_components``:
       for overlapping key sets (``nested`` and ``overlapping``), where
       estimates depend on the hash functions, the facts above still bound
       the groups exactly. Signatures with one key set share a group. A
       group never spans two components of the J > 0 graph, because edges
       and merge links are retrieved pairs. Each reported count lies
       between its two exact references. Every merge removes one group. At
       τ = 0 every retrieved pair is an edge, so nothing merges. Half the
       parameter draws are merge-prone (many bands of few rows, high τ), so
       greedy merges occur and the bound covers them.

    In both, groups are compared as partitions of object ids, through
    :meth:`ObjectSignatures.view`, and the signatures are first checked
    against the reference signatures. The grouping's groups and details
    expose every comparison point, so the code needs no test hook.

    One hash coincidence remains. The sketch of the empty key set holds the
    maximal hash value under every function. A one-key set matches it under
    a function only when its key hashes to that maximum, with probability
    2^-32 per function. The test treats the empty key set like any other,
    so such a coincidence would show as a failure.

Co_Occurrence_Rule candidates (9.3)
    ``test_cooccurrence_rule_candidates_equal_brute_force`` compares
    :func:`rule_candidates` with an exhaustive search over the reference
    objects, on three inputs: the Path's view (non-object and flagged rows
    are ignored), its :class:`Presence`, and a Variant (the view of a drawn
    subset of the objects). For each ordered pair of different fields
    ``a ≠ b`` of the objects, the pair is a candidate when every object
    holding ``a`` also holds ``b`` and some object lacks ``b`` (``b`` is
    optional). Its Support count is the number of objects holding ``a``,
    and candidates are in code-point order of ``(a, b)``.

**Validates: Requirements 5.7, 9.3**
"""

# Feature: json-schema-inference, Property 17: Mining primitives equal brute-force references

from __future__ import annotations

from collections import Counter
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from fractions import Fraction
from typing import Any

import numpy as np
import pytest
from datasketch.hashfunc import sha1_hash32
from hypothesis import assume, event, given
from hypothesis import strategies as st

from jsinfer.core.observations import ObservationStore, build_store
from jsinfer.core.paths import parse_path
from jsinfer.core.views import View
from jsinfer.deps.cooccurrence import RuleCandidate, rule_candidates
from jsinfer.deps.presence import Presence
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.structure.variants import MINHASH_LSH, Grouping, GroupingScorer, ObjectSignatures, minhash_lsh_grouping
from tests.strategies import KEYS

# One Validator and MDL_Scorer for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
SCORER = MDLScorer(validator=VALIDATOR)

ObjectId = tuple[int, int]  # (document index, array position or -1)
KeySet = frozenset[str]
Signature = frozenset[tuple[str, str]]
Partition = frozenset[frozenset[ObjectId]]

# The 32-bit key hashes that datasketch mixes per hash function (minhash_lsh_grouping encodes keys this way).
KEY_HASHES: dict[int, str] = {sha1_hash32(key.encode("utf-8", "surrogatepass")): key for key in KEYS}

# ---------------------------------------------------------------------------
# Object sets, flags, and the raw-document reference (shared by every primitive)
# ---------------------------------------------------------------------------


def json_type(value: Any) -> str:
    """The glossary's JSON type of ``value``: integers and other numbers are both ``number``."""
    if value is None:
        return "null"
    if isinstance(value, bool):  # before numbers: bool is an int subclass
        return "boolean"
    if isinstance(value, (int, float)):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    assert isinstance(value, dict), value
    return "object"


LAYOUTS = ("$", "$.v", "$.v[*]")
POOL_SHAPES = ("blocks", "nested", "overlapping")


def objects_at(layout: str, docs: Sequence[Any]) -> list[tuple[ObjectId, dict[str, Any]]]:
    """Every object at the Path ``layout`` in the raw documents, with its id, flags ignored."""
    found: list[tuple[ObjectId, dict[str, Any]]] = []
    for d, doc in enumerate(docs):
        if layout == "$":
            if isinstance(doc, dict):
                found.append(((d, -1), doc))
        elif not isinstance(doc, dict):
            continue
        elif layout == "$.v":
            if isinstance(doc.get("v"), dict):
                found.append(((d, -1), doc["v"]))
        elif isinstance(doc.get("v"), list):
            found.extend(((d, i), element) for i, element in enumerate(doc["v"]) if isinstance(element, dict))
    return found


# Values by template type; "integer" (with integral floats) and "number" are one signature type.
VALUES: dict[str, st.SearchStrategy[Any]] = {
    "null": st.none(),
    "boolean": st.booleans(),
    "integer": st.one_of(st.integers(-3, 3), st.sampled_from([1.0, 1e3, -0.0, 2**70])),
    "number": st.sampled_from([2.5, -0.5, 0.1]),
    "string": st.text("ab😀", max_size=3),
    "array": st.lists(st.none() | st.integers(0, 2) | st.just({"a": 1}), max_size=2),
    "object": st.dictionaries(st.sampled_from(KEYS[:3]), st.integers(0, 2), max_size=2),
}
TYPES = tuple(VALUES)

# Rows at the Path that are not objects, including arrays of objects one level deeper.
NON_OBJECTS: tuple[Any, ...] = (None, 3, 2.5, "s", True, [], [{"a": 1}], [1, [2]])

# Objects of always-flagged spoiler documents: any key set, so counting them would change the results.
SPOILERS = st.dictionaries(st.sampled_from(KEYS), st.none() | st.integers(0, 2), max_size=4)


@st.composite
def key_set_pools(draw: st.DrawFn, shape: str) -> list[KeySet]:
    """Distinct key sets of the given shape (see the module docstring)."""
    order = draw(st.permutations(KEYS), label="key order")
    if shape == "blocks":
        # One to four blocks of the keys used; a lone block always comes with the empty key set.
        used = order[: draw(st.integers(1, len(order)), label="keys used")]
        n_blocks = draw(st.integers(1, min(4, len(used))), label="blocks")
        cuts = sorted(draw(st.permutations(range(1, len(used))), label="block cuts")[: n_blocks - 1])
        bounds = [0, *cuts, len(used)]
        pool = [frozenset(used[a:b]) for a, b in zip(bounds, bounds[1:])]
        if n_blocks == 1 or draw(st.booleans(), label="the empty key set"):
            pool.append(frozenset())
    elif shape == "nested":
        depth = draw(st.integers(1, 5), label="chain length")
        lengths = draw(st.sets(st.integers(0, depth), min_size=min(3, depth + 1)), label="prefix lengths")
        pool = [frozenset(order[:i]) for i in sorted(lengths)]
        off_chain = st.frozensets(st.sampled_from(order[: depth + 1]))
        pool += draw(st.lists(off_chain, max_size=1), label="off-chain key set")
    else:
        assert shape == "overlapping", shape
        alphabet = order[: draw(st.integers(1, 5), label="alphabet size")]
        pool = draw(st.lists(st.frozensets(st.sampled_from(alphabet)), min_size=2, max_size=5), label="key sets")
    return list(dict.fromkeys(pool))


@st.composite
def object_sets(draw: st.DrawFn, shape: str) -> list[dict[str, Any]]:
    """Two to sixteen objects with key sets from a pool; an object may retype one key of its key set.

    The first two objects have different key sets when the pool has two or
    more. Flags can still leave a single object or a single key set.
    """
    pool = draw(key_set_pools(shape), label="key-set pool")
    base = [{key: draw(st.sampled_from(TYPES), label="base type") for key in sorted(keys)} for keys in pool]
    n = draw(st.integers(2, 16), label="objects")
    first = draw(st.permutations(range(len(pool))), label="first key sets")[: min(n, 2)]
    rest = st.lists(st.integers(0, len(pool) - 1), min_size=n - len(first), max_size=n - len(first))
    out: list[dict[str, Any]] = []
    for pick in [*first, *draw(rest, label="key sets")]:
        template = dict(base[pick])
        if template and draw(st.booleans(), label="retype a key"):
            template[draw(st.sampled_from(sorted(template)), label="retyped key")] = draw(st.sampled_from(TYPES))
        out.append({key: draw(VALUES[kind]) for key, kind in template.items()})
    return out


def _chunks(draw: st.DrawFn, items: list[Any]) -> list[list[Any]]:
    """``items`` split in order into arrays of one to four elements."""
    out: list[list[Any]] = []
    i = 0
    while i < len(items):
        size = draw(st.integers(1, 4))
        out.append(items[i : i + size])
        i += size
    return out


@dataclass(frozen=True)
class Case:
    """Documents, their flags, and the reference: each non-flagged object at the Path, by id."""

    layout: str
    shape: str
    docs: tuple[Any, ...]
    flagged_docs: tuple[int, ...]
    flagged_objects: tuple[ObjectId, ...]  # objects at the Path flagged as items
    flagged_members: tuple[tuple[ObjectId, str], ...]  # member values of non-flagged objects flagged as items
    flagged_arrays: tuple[int, ...]  # documents whose $.v array is flagged ($.v[*] only)
    reference: Mapping[ObjectId, Mapping[str, Any]]  # the raw non-flagged objects; flagged members keep their keys

    def key_set(self, oid: ObjectId) -> KeySet:
        return frozenset(self.reference[oid])

    def signature(self, oid: ObjectId) -> Signature:
        return frozenset((key, json_type(value)) for key, value in self.reference[oid].items())

    @property
    def flagged(self) -> bool:
        return bool(self.flagged_docs or self.flagged_objects or self.flagged_members or self.flagged_arrays)


@st.composite
def cases(draw: st.DrawFn, layout: str, shapes: Sequence[str] = POOL_SHAPES) -> Case:
    """Objects at the Path ``layout`` with key sets of a drawn shape, among other rows, with flags."""
    shape = draw(st.sampled_from(shapes), label="key-set shape")
    objects = draw(object_sets(shape), label="objects")
    spoilers = draw(st.lists(SPOILERS, max_size=2), label="spoilers")
    others = draw(st.lists(st.sampled_from(NON_OBJECTS), max_size=3), label="non-object rows")

    # Documents: (document, spoiler?) pairs.
    if layout == "$":
        units = [(obj, False) for obj in objects] + [(x, False) for x in others] + [(obj, True) for obj in spoilers]
    elif layout == "$.v":
        units = [({"v": x}, False) for x in [*objects, *others]] + [({"v": obj}, True) for obj in spoilers]
        units += [({"w": 1}, False)] * draw(st.integers(0, 1), label="documents without v")
    else:
        mixed = draw(st.permutations([*objects, *others]), label="element order")
        units = [({"v": chunk}, False) for chunk in _chunks(draw, mixed)]
        units += [({"v": [obj]}, True) for obj in spoilers]
        units += [({"v": 3}, False), ({"w": 1}, False)][: draw(st.integers(0, 2), label="documents without arrays")]
    units = [units[i] for i in draw(st.permutations(range(len(units))), label="document order")]
    docs = tuple(doc for doc, _ in units)

    extra = draw(st.sets(st.integers(0, len(docs) - 1), max_size=2), label="flagged clean documents")
    flagged_docs = tuple(i for i, (_, spoiler) in enumerate(units) if spoiler or i in extra)
    skipped = set(flagged_docs)
    flagged_arrays: tuple[int, ...] = ()
    if layout == "$.v[*]":
        arrays = [i for i, doc in enumerate(docs) if i not in skipped and isinstance(doc.get("v"), list)]
        if arrays and draw(st.integers(0, 3), label="flag an array") == 0:
            flagged_arrays = (draw(st.sampled_from(arrays), label="flagged array"),)
    live = [(oid, obj) for oid, obj in objects_at(layout, docs) if oid[0] not in skipped and oid[0] not in flagged_arrays]
    ids = [oid for oid, _ in live]
    flagged_objects = tuple(sorted(draw(st.sets(st.sampled_from(ids), max_size=2), label="flagged objects"))) if ids else ()
    kept = [(oid, obj) for oid, obj in live if oid not in set(flagged_objects)]
    members = [(oid, key) for oid, obj in kept for key in obj]
    flagged_members: tuple[tuple[ObjectId, str], ...] = ()
    if members:
        chosen = draw(st.sets(st.integers(0, len(members) - 1), max_size=2), label="flagged members")
        flagged_members = tuple(members[i] for i in sorted(chosen))
    assume(kept)  # every primitive works on at least one non-flagged object
    return Case(layout, shape, docs, flagged_docs, flagged_objects, flagged_members, flagged_arrays, dict(kept))


def object_ids(view: View) -> list[ObjectId]:
    return list(zip(view.gather("doc").tolist(), view.gather("pos").tolist()))


def _rows_where(view: View, wanted: Sequence[ObjectId]) -> View:
    targets = set(wanted)
    return view.subset(np.array([oid in targets for oid in object_ids(view)], dtype=bool))


def build(case: Case) -> tuple[ObservationStore, View]:
    """A fresh store of the documents with the case's flags, and the view of its Path."""
    store = build_store(list(case.docs))
    root = View.root(store)
    path = parse_path(case.layout)
    at = root.descend(path)
    targets = [_rows_where(at, case.flagged_objects)]
    targets += [_rows_where(at, [oid]).child_key(key) for oid, key in case.flagged_members]
    if case.flagged_arrays:
        arrays = root.child_key("v")
        targets.append(arrays.subset(np.isin(arrays.gather("doc"), case.flagged_arrays)))
    for target in targets:
        store.flags.flag_view(target)
    if case.flagged_docs:
        store.flags.flag_documents(case.flagged_docs)
    return store, View.root(store).descend(path)


def note_case(case: Case) -> None:
    event(f"key sets: {case.shape}")
    if len(case.reference) == 1:
        event("one non-flagged object")
    elif len({case.key_set(oid) for oid in case.reference}) == 1:
        event("one key set")
    if case.flagged_docs:
        event("flagged documents")
    if case.flagged_members:
        event("flagged member values")
    if case.flagged_objects or case.flagged_arrays:
        event("flagged objects or arrays")


# ---------------------------------------------------------------------------
# LSH candidate groups (5.7)
# ---------------------------------------------------------------------------


def jaccard(a: KeySet, b: KeySet) -> Fraction:
    """The exact Jaccard similarity ``|a ∩ b| / |a ∪ b|``; two empty key sets are equal (1)."""
    union = a | b
    return Fraction(len(a & b), len(union)) if union else Fraction(1)


def brute_components(
    items: Sequence[Signature], linked: Callable[[Signature, Signature], bool]
) -> list[list[Signature]]:
    """Connected components of the graph on ``items`` with an edge between every linked pair, by exhaustive search."""
    label = list(range(len(items)))
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            if label[i] != label[j] and linked(items[i], items[j]):
                old, new = max(label[i], label[j]), min(label[i], label[j])
                label = [new if x == old else x for x in label]
    groups: dict[int, list[Signature]] = {}
    for item, x in zip(items, label):
        groups.setdefault(x, []).append(item)
    return list(groups.values())


@dataclass(frozen=True)
class SignatureReference:
    """Brute-force facts about the distinct reference signatures of a case and their key sets."""

    case: Case
    signatures: tuple[Signature, ...]

    @classmethod
    def of(cls, case: Case) -> SignatureReference:
        return cls(case, tuple(dict.fromkeys(case.signature(oid) for oid in case.reference)))

    @staticmethod
    def keys(signature: Signature) -> KeySet:
        return frozenset(key for key, _ in signature)

    def similarity(self, a: Signature, b: Signature) -> Fraction:
        return jaccard(self.keys(a), self.keys(b))

    def components(self, linked: Callable[[Signature, Signature], bool]) -> list[list[Signature]]:
        return brute_components(self.signatures, linked)

    def pairs(self, linked: Callable[[Signature, Signature], bool]) -> int:
        s = self.signatures
        return sum(1 for i in range(len(s)) for j in range(i + 1, len(s)) if linked(s[i], s[j]))

    # The three graphs: exact LSH edges at τ, equal key sets (J = 1), and a common key (J > 0).
    def exact_edge(self, threshold: float) -> Callable[[Signature, Signature], bool]:
        tau = Fraction(threshold)
        return lambda a, b: 0 < self.similarity(a, b) and self.similarity(a, b) >= tau

    def same_keys(self, a: Signature, b: Signature) -> bool:
        return self.keys(a) == self.keys(b)

    def overlap(self, a: Signature, b: Signature) -> bool:
        return self.similarity(a, b) > 0

    def partition(self, groups: Sequence[Sequence[Signature]]) -> Partition:
        """The partition of the case's objects that puts each object in its signature's group."""
        where = {sig: g for g, group in enumerate(groups) for sig in group}
        blocks: dict[int, set[ObjectId]] = {}
        for oid in self.case.reference:
            blocks.setdefault(where[self.case.signature(oid)], set()).add(oid)
        return frozenset(frozenset(block) for block in blocks.values())


def check_signatures(signatures: ObjectSignatures, case: Case) -> None:
    """``signatures`` are exactly the reference signatures of the case's objects, with their object counts."""
    found: Counter[Signature] = Counter()
    for s in range(signatures.n_signatures):
        entries = frozenset(signatures.entries(s))
        ids = object_ids(signatures.view((s,)))
        assert ids, f"signature {sorted(entries)} has no object"
        wrong = [oid for oid in ids if oid not in case.reference or case.signature(oid) != entries]
        assert not wrong, f"objects {wrong[:3]} are not non-flagged objects with signature {sorted(entries)}"
        assert entries not in found, f"signature {sorted(entries)} is listed twice"
        found[entries] = len(ids)
    assert found == Counter(case.signature(oid) for oid in case.reference), "the Object_Signatures differ"


def grouped_objects(signatures: ObjectSignatures, grouping: Grouping) -> Partition:
    """The grouping as a partition of object ids."""
    return frozenset(frozenset(object_ids(signatures.view(group))) for group in grouping.groups)


def refines(fine: Partition, coarse: Partition) -> bool:
    """Every block of ``fine`` lies inside one block of ``coarse``."""
    return all(any(block <= big for big in coarse) for block in fine)


@dataclass(frozen=True)
class LshParameters:
    """τ = ``lsh.keyset_jaccard``, ``minhash.perm``, ``lsh.bands``, and the run seed."""

    threshold: float
    perm: int
    bands: int
    seed: int

    def run(self, signatures: ObjectSignatures) -> Grouping:
        grouping = minhash_lsh_grouping(
            signatures,
            GroupingScorer(signatures, SCORER),
            threshold=self.threshold,
            perm=self.perm,
            bands=self.bands,
            seed=self.seed,
        )
        assert grouping.technique == MINHASH_LSH
        assert grouping.cutoff is not None and grouping.cutoff.value == self.threshold
        signatures.check_partition(grouping.groups)
        return grouping


@st.composite
def lsh_parameters(draw: st.DrawFn) -> LshParameters:
    """Hand-set τ in [0, 1] and permutation and band counts, with the extremes common.

    Half the draws are merge-prone: many bands of at most four rows retrieve
    most pairs that share a key, and a high τ leaves some of them below it,
    so the greedy merge has links to try.
    """
    if draw(st.booleans(), label="merge-prone"):
        perm = draw(st.sampled_from([8, 32, 128]), label="minhash.perm")
        bands = draw(st.integers(max(2, perm // 4), perm), label="lsh.bands")
        threshold = draw(st.floats(0.75, 1.0), label="lsh.keyset_jaccard")
    else:
        perm = draw(st.sampled_from([1, 2, 8, 32, 128]), label="minhash.perm")
        bands = draw(st.one_of(st.just(1), st.just(perm), st.integers(1, perm)), label="lsh.bands")
        threshold = draw(st.one_of(st.sampled_from([0.0, 1.0]), st.floats(0.0, 1.0)), label="lsh.keyset_jaccard")
    return LshParameters(threshold, perm, bands, draw(st.integers(0, 2**63 - 1), label="seed"))


def note_lsh(params: LshParameters, grouping: Grouping, reference: SignatureReference) -> None:
    if params.bands == 1:
        event("lsh: one band (equal sketches)")
    elif params.perm // params.bands == 1:
        event("lsh: one row per band")
    if grouping.details["merges"]:
        event("lsh: greedy merges")
    if len(reference.signatures) > len(reference.components(reference.same_keys)):
        event("lsh: a key set with several signatures")


def check_key_hashes() -> None:
    assert len(KEY_HASHES) == len(KEYS), "two alphabet keys share a 32-bit hash, so J = 0 estimates are not exact"


@pytest.mark.parametrize("layout", LAYOUTS)
@given(data=st.data(), params=lsh_parameters())
def test_lsh_candidate_groups_equal_exact_jaccard_components(
    layout: str, data: st.DataObject, params: LshParameters
) -> None:
    check_key_hashes()
    case = data.draw(cases(layout, ("blocks",)), label="case")
    _, view = build(case)
    signatures = ObjectSignatures.of(view)
    check_signatures(signatures, case)
    reference = SignatureReference.of(case)
    sigs = reference.signatures
    assert all(reference.similarity(a, b) in (0, 1) for a in sigs for b in sigs), "key sets are not blocks"

    grouping = params.run(signatures)
    expected = reference.components(reference.exact_edge(params.threshold))
    assert grouped_objects(signatures, grouping) == reference.partition(expected), (
        f"LSH groups differ from the exact Jaccard components at τ = {params.threshold}"
    )
    details = grouping.details
    identical = reference.pairs(reference.same_keys)
    assert details["components"] == len(expected) == grouping.n_variants
    assert details["merges"] == 0, "a retrieved pair joined two exact components"
    assert details["candidate_pairs"] == details["edges"] == identical
    assert details["spanning_links"] == len(sigs) - len(expected)
    note_lsh(params, grouping, reference)
    note_case(case)


@pytest.mark.parametrize("layout", LAYOUTS)
@given(data=st.data(), params=lsh_parameters())
def test_lsh_candidate_groups_lie_between_exact_key_set_classes_and_overlap_components(
    layout: str, data: st.DataObject, params: LshParameters
) -> None:
    check_key_hashes()
    # Blocks are compared exactly above; here key sets overlap, where estimates depend on the hash functions.
    case = data.draw(cases(layout, ("nested", "overlapping")), label="case")
    _, view = build(case)
    signatures = ObjectSignatures.of(view)
    check_signatures(signatures, case)
    reference = SignatureReference.of(case)
    n_signatures = len(reference.signatures)

    grouping = params.run(signatures)
    found = grouped_objects(signatures, grouping)
    classes = reference.components(reference.same_keys)
    overlap = reference.components(reference.overlap)
    assert refines(reference.partition(classes), found), "signatures with one key set are in different groups"
    assert refines(found, reference.partition(overlap)), "a group joins key sets with no common key"

    details = grouping.details
    assert details["components"] - details["merges"] == grouping.n_variants
    assert len(overlap) <= details["components"] <= len(classes)
    assert reference.pairs(reference.same_keys) <= details["edges"] <= details["candidate_pairs"]
    assert details["candidate_pairs"] <= reference.pairs(reference.overlap)
    assert n_signatures - len(classes) <= details["spanning_links"] <= n_signatures - len(overlap)
    if params.threshold == 0:
        # Every retrieved pair is an edge, so every spanning-forest link lies inside one component.
        assert details["edges"] == details["candidate_pairs"] and details["merges"] == 0
    note_lsh(params, grouping, reference)
    note_case(case)


# ---------------------------------------------------------------------------
# Co_Occurrence_Rule candidates (9.3)
# ---------------------------------------------------------------------------


def reference_rule_candidates(objects: Sequence[KeySet]) -> list[tuple[str, str, int]]:
    """The 9.3 candidates by exhaustive search: (antecedent, consequent, Support count) in code-point order."""
    fields = sorted({key for keys in objects for key in keys})  # str order is code-point order
    out: list[tuple[str, str, int]] = []
    for a in fields:
        holders = [keys for keys in objects if a in keys]
        for b in fields:
            optional = not all(b in keys for keys in objects)
            if a != b and optional and all(b in keys for keys in holders):
                out.append((a, b, len(holders)))
    return out


def triples(candidates: Sequence[RuleCandidate]) -> list[tuple[str, str, int]]:
    return [(c.antecedent, c.consequent, c.support_count) for c in candidates]


@pytest.mark.parametrize("layout", LAYOUTS)
@given(data=st.data())
def test_cooccurrence_rule_candidates_equal_brute_force(layout: str, data: st.DataObject) -> None:
    case = data.draw(cases(layout), label="case")
    _, view = build(case)
    ids = sorted(case.reference)

    # The Path's objects, through the view and through its Presence.
    found = rule_candidates(view)
    expected = reference_rule_candidates([case.key_set(oid) for oid in ids])
    assert triples(found) == expected
    assert rule_candidates(Presence.of(view)) == found

    # A Variant: the objects of a drawn subset.
    chosen = sorted(data.draw(st.sets(st.sampled_from(ids), min_size=1), label="Variant objects"))
    variant = _rows_where(view, chosen)
    assert sorted(object_ids(variant)) == chosen
    assert triples(rule_candidates(variant)) == reference_rule_candidates([case.key_set(oid) for oid in chosen])

    event("candidates found" if expected else "no candidates")
    if any((b, a, n) in expected for a, b, n in expected):
        event("mutual implications")
    note_case(case)


# Tasks 35.3 and 44.2 add their primitives' tests below, reusing cases() and Case.reference.


# ---------------------------------------------------------------------------
# FP-growth frequent itemsets (6.3, task 33.3)
# ---------------------------------------------------------------------------


def reference_itemsets(key_sets: Sequence[KeySet], min_support: int) -> dict[frozenset[str], int]:
    """Every non-empty subset of the keys held by at least ``min_support`` of the key sets, with its count."""
    from itertools import combinations

    keys = sorted({key for key_set in key_sets for key in key_set})
    found: dict[frozenset[str], int] = {}
    for size in range(1, len(keys) + 1):
        for combo in combinations(keys, size):
            itemset = frozenset(combo)
            count = sum(1 for key_set in key_sets if itemset <= key_set)
            if count >= min_support:
                found[itemset] = count
    return found


@given(data=st.data(), min_support=st.integers(1, 4))
def test_fpgrowth_frequent_itemsets_equal_brute_force(data: st.DataObject, min_support: int) -> None:
    from jsinfer.structure.field_groups import frequent_itemsets, transactions

    case = data.draw(cases("$"), label="case")
    _, view = build(case)
    ids = sorted(case.reference)
    p = Presence.of(view)
    # Transactions are the optional (non-required) key sets of the non-flagged objects, with counts.
    weighted = transactions(p)
    required = set(p.required)
    key_sets = [case.key_set(oid) - required for oid in ids]
    assert sum(weighted.values()) == sum(1 for key_set in key_sets if key_set)
    found = frequent_itemsets(weighted, min_support)
    expected = reference_itemsets(key_sets, min_support)
    assert found == expected
    assert list(found) == sorted(found, key=lambda s: (len(s), tuple(sorted(s))))
    event("frequent itemsets" if expected else "none frequent")
    note_case(case)
