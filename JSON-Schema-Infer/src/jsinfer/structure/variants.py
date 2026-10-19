"""Variants of an object Path: candidate groupings, their selection, Merge-or-Split, and the branches.

Requirements 5.1–5.11 and 29.8. Grouping and its selection are task 13.2;
Merge-or-Split and branch emission (5.3–5.6, 5.11) are task 13.3.

Object_Signatures (5.1)
    :class:`ObjectSignatures` reads the Object_Signature of every non-flagged
    object row of a view (:func:`~jsinfer.structure.record_map.object_rows`):
    its keys together with the JSON type of the value under each key, where
    ``integer`` and ``number`` are both ``number`` (the glossary's six types).
    Each distinct signature is weighted by the number of objects that have
    it, and the signatures are sorted canonically: as sequences of
    ``(key, type)`` entries in code-point order of the keys, compared entry by
    entry (a prefix first). Key ids follow code-point order, so the sort runs
    on ``(key id, type rank)`` codes. As in ``core.stats``, a member whose
    value is a Flagged_Item still makes its key, with the value's type, part
    of the signature of its non-flagged parent object.

    Every grouping Technique partitions the *signatures*, so objects with
    equal signatures always share a candidate Variant and every object
    belongs to exactly one (5.1). A candidate Variant's view is the subset of
    the object rows with its signatures (:meth:`ObjectSignatures.view`).

Groupings and their description length (5.2, 5.9)
    A :class:`Grouping` is a canonical partition of the signature indices:
    each group sorted, groups ordered by their first index. It is encoded as
    the design's MDL_Scorer encoding for Variants: one candidate Variant is
    the Naive_Merge subschema of the objects, and two or more are an
    ``anyOf`` of the Naive_Merge subschemas of the candidate Variants, scored
    with ``branch_views`` assigning each object to its own branch
    (:meth:`GroupingScorer.score`). Descendants are represented by their
    Naive_Merge subschemas (local scoring).

    That description length decomposes exactly: with ``L(g)`` the
    description length of the Naive_Merge subschema of group ``g`` on its
    own objects, a grouping of ``G >= 2`` groups with object counts ``c``
    costs ``LN0(1) + UNI(|KW|) + LN0(G)`` model bits for the ``anyOf``
    keyword, the 1-bit exception flag and ``CAT(c)`` data bits for the
    branch choice, and ``Σ L(g)``; one group costs ``L(g)``. Every branch
    accepts all of its own objects, so no exception arises.
    :class:`GroupingScorer` caches ``L(g)`` per group and evaluates
    :meth:`GroupingScorer.bits` from the cache, so the searches below cost
    one scoring per group they look at, not one per grouping. Merging two
    groups changes the length by a term shared by every merge of the same
    grouping (the ``anyOf`` overhead and the ``CAT`` normalizer) plus
    :meth:`GroupingScorer.merge_term`, ``L(a ∪ b) − L(a) − L(b)`` and the
    change of the two ``CAT`` categories; the noise assignment ranks the
    groups a signature could join by that term and checks the best join
    against a new group with the exact length.

The grouping Techniques (5.7, 5.8)
    ``variants.minhash_lsh`` (:func:`minhash_lsh_grouping`)
        A datasketch ``MinHash`` of each signature's key set, seeded with a
        32-bit draw from ``rng("variants.minhash_lsh", path)``, with
        ``minhash.perm`` permutations, indexed in a ``MinHashLSH`` with
        ``params=(lsh.bands, minhash.perm // lsh.bands)``. datasketch needs
        two or more bands, so with ``lsh.bands = 1`` (which the
        Resource_Parameter admits) the single band of all permutations is
        applied directly: the pairs with equal sketches. Every pair the
        index retrieves whose estimated Jaccard similarity is at or above the
        Learned_Threshold τ = ``lsh.keyset_jaccard`` is a graph edge; the
        connected components (``scipy.sparse.csgraph``) are the initial
        groups. Groups are then merged greedily while the total description
        length decreases. The merge candidates are the links of the maximum
        spanning forest of the retrieved pairs, weighted by estimated Jaccard
        similarity (Kruskal, ties by signature pair): the strongest LSH link
        between every two parts of the retrieval graph. A pass walks the
        links strongest first and merges the two groups a link joins when
        that lowers the exact description length of the whole grouping;
        passes repeat until one merges nothing. The merge search therefore
        stays within the candidate structure LSH produced (5.7) and scores
        about one union per link and pass, instead of one per pair of
        groups per merge, so a strict τ that leaves many components stays
        tractable.
    ``variants.dbscan`` (:func:`dbscan_grouping`)
        ``DBSCAN(eps, min_samples, metric="precomputed")`` on the Jaccard
        distance matrix of the signatures, each a set of ``(key, type)``
        entries, with ``sample_weight`` the signature counts, so
        ``dbscan.min_samples`` counts objects. ``dbscan.eps`` and
        ``dbscan.min_samples`` are Learned_Thresholds.
    ``variants.hdbscan`` (:func:`hdbscan_grouping`)
        ``HDBSCAN(min_cluster_size, metric="precomputed")`` on the same
        matrix. scikit-learn's HDBSCAN takes no ``sample_weight``, so each
        distinct signature is one point and ``hdbscan.min_cluster_size``
        counts signatures; the Trainer fits it in those units. When the
        minimum cluster size exceeds the number of points, no cluster can
        form and every signature is noise.
    ``variants.bernoulli_mixture`` (:func:`bernoulli_mixture_grouping`)
        In-house weighted EM for a mixture of independent Bernoulli
        distributions over the key-presence bits of the signatures, weighted
        by their counts, for each component count k = 1 … min(#signatures,
        ``variants.max_components``). Parameters are smoothed with the
        Jeffreys (KT) prior, the estimator of the MDL_Scorer's categorical
        codes, so no log of zero arises. Each k starts from k distinct
        signatures drawn with probability proportional to their counts from
        ``rng("variants.bernoulli_mixture", path, k)``, each signature
        assigned to its nearest start by Hamming distance, and runs until the
        mean log-likelihood gain per object falls below ``variants.em_tol``
        nats or ``variants.em_max_iter`` iterations. BIC
        ``−2 ln L + (k·K + k − 1) ln N`` (K keys, N objects) selects k, ties
        to the smaller k (5.2); each signature joins its most probable
        component, and empty components are dropped. The BIC of every k is
        the grouping's cut-off.

    Noise (5.9): DBSCAN and HDBSCAN label some signatures as noise. They are
    taken in canonical order, and each signature's objects join the existing
    or new group (existing groups first, in canonical order, then the new
    one) with the lowest description length, so noise never becomes a
    Flagged_Item. A group created by an earlier noise signature is an
    existing group for later ones. Each noise signature costs one scoring
    per existing group (the union with it) plus one for itself.

Sampling (29.8)
    DBSCAN and HDBSCAN are super-linear in the points they cluster, which are
    the distinct signatures (the design's ``sample.dbscan`` and
    ``sample.hdbscan`` count signatures). When a Path has more signatures than
    the Technique's sample size, the Technique clusters a uniform sample of
    exactly that many, drawn without replacement with ``sample_rows`` and
    ``rng(technique, path)``. Every signature left out takes the label of its
    nearest sampled signature (smallest Jaccard distance, ties to the first),
    the usual out-of-sample extension, so it is noise exactly when that
    signature is; the grouping still partitions every object. The grouping
    records the sample as :class:`~jsinfer.core.provenance.SampleInfo`
    (sampled and total signatures) for the Provenance_Records it produces
    (26.4).

Selection (5.2, 5.8, 5.10, 5.11)
    :meth:`VariantAnalyzer.select` resolves the ``variant_grouping``
    Decision_Point of a view with two or more signatures. Every Technique in
    effect (implemented, enabled, and with its Learned_Thresholds in the
    Model_Bundle) offers its grouping as one candidate labeled
    ``one-variant`` or ``<n>-variants``, with the MDL score of its encoding
    precomputed (5.8). The DecisionEngine then applies its order: MDL while
    ``decide.mdl`` is enabled (the lowest description length; no tie rule is
    mandated), else the registry priority order (``variants.minhash_lsh``,
    ``variants.dbscan``, ``variants.hdbscan``, ``variants.bernoulli_mixture``;
    5.11), recording the cut-off the first produced grouping applied: its
    Learned_Threshold (``dbscan.eps`` for DBSCAN, whose ``min_samples`` is in
    the grouping's details) or the BIC comparison. With no Technique in
    effect the Fallback_Behavior places every object in one Variant (5.10),
    and the fallback names the first serving Technique that is disabled or
    missing a Learned_Threshold. A Technique that is missing a
    Learned_Threshold produces no grouping (20.6). A view whose objects
    share one signature is one Variant without a Decision_Point.

    No number of Variants is read from the Run_Configuration (5.2): it comes
    from the DL comparisons of the greedy merge and noise assignment, from
    BIC, or from clustering with Learned_Thresholds.

Merge-or-Split (5.3, 5.6, 5.11)
    :func:`merge_or_split` resolves the ``merge_or_split`` Judgment_Call of
    a :class:`VariantGrouping` with two or more candidate Variants; with one
    there is nothing to merge and no Decision_Point. The Judgment_Call is
    binary, as its Decision_Model's classes are: the candidate Variants are
    all merged into one Variant, or all kept as separate Variants. Partial
    merges are the grouping step's business: its DL-driven merges (the LSH
    greedy merge, the noise assignment, BIC) already chose which signatures
    share a candidate Variant. Both candidates are structural alternatives
    that no Technique produces (``technique=None``), labeled with the kind's
    outcomes, and scored on the Path's object rows with the design's MDL
    comparison, both through the :class:`GroupingScorer` of the grouping
    (whose ``score`` is cached per grouping, so the selected grouping is not
    scored twice):

    - merge: one object schema, the Naive_Merge subschema of every object
      (the union of the fields, ``required`` the fields every object holds,
      and the union of the observed types per field);
    - split: the grouping's encoding, an ``anyOf`` of the Naive_Merge
      subschemas of the candidate Variants, whose data bits include
      ``CAT(group sizes)`` for the branch choice.

    The DecisionEngine resolves the point in its order: the Merge-or-Split
    Decision_Model (``decide.dm_merge_split``) when it is in effect and the
    Model_Bundle holds it, on :func:`merge_split_features` (the
    caller's ``features`` when given); else MDL while ``decide.mdl`` is
    enabled (no tie rule is mandated, so a tie goes to fewer model bits,
    then to merge by candidate order); else the Fallback_Behavior split,
    which keeps the candidate Variants of the grouping that the registry
    priority order selected (5.11).

Branches (5.4, 5.5, 5.6)
    :func:`variant_branches` builds the schema of a
    :class:`MergeSplitOutcome`'s final Variants. Each Variant's object schema
    is ``variant_schema(view)`` of its objects, by default
    :func:`~jsinfer.pipeline.naive_merge.naive_merge_node`, which lists under
    ``properties`` exactly the Variant's observed fields (5.4) and carries
    ``type: ["object"]``, so a branch never accepts a non-object.

    - One final Variant (merged, or a single candidate Variant): the node is
      that Variant's object schema. For a merge it is built from the union of
      the merged candidate Variants' objects, so the union of the fields, the
      fields every object holds as ``required``, and the union of the
      observed types per field follow (5.6).
    - Two or more: one branch per Variant, in the grouping's canonical order,
      under ``oneOf`` when every pair of branches is disjoint
      (:func:`branches_disjoint`, 5.5) and under ``anyOf`` otherwise (5.4).
      Each branch is a Constraint, ``(nid, keyword, i)`` at
      ``…/anyOf/<i>``, whose Provenance_Record
      (:meth:`MergeSplitOutcome.branch_provenance`) comes from the
      Merge-or-Split decision: its Technique, method, candidates with scores,
      cut-off, fallback indicator, and Confidence (the MDL posterior or
      calibrated probability; the Laplace default for a fallback). The
      details hold the grouping facts (:meth:`VariantGrouping.details`),
      the branch index, the combinator, and the ``variant_grouping``
      decision (Technique, method, candidates with scores, cut-off, and
      fallback indicator), so both Decision_Points that produced the branch
      are on record (20.7, 26.3); the grouping's sample goes in ``sample``
      (26.4). Support counts the Variant's objects among the Path's objects;
      provenance finalization (task 17.1) recomputes Support after the
      soundness check.

    A merged outcome produces no Constraint of its own (its object schema's
    Constraints come from Record-or-Map and the Baseline_Rules), so the
    engine's Inference_Report entry is the decision's record (20.7), and
    :meth:`MergeSplitOutcome.details` gives the facts the analyzer may merge
    into the record of the merged Record's ``properties``.

Disjoint branches (5.5)
    Two branches are disjoint when both require a common property and
    constrain it to non-overlapping value sets, or when one requires a field
    the other does not permit. Both tests read the branches' own keywords:

    - A property's value set is that of the ``const`` and ``enum`` keywords
      of its subschema under ``properties`` (both: their intersection), with
      values compared under JSON Schema equality (``canonical_key``); a
      property whose subschema has neither is unconstrained.
    - A keyword schema permits a field unless its subschema under
      ``properties`` is ``false``, the subschema of a matching
      ``patternProperties`` Pattern is ``false``, the field is neither listed
      nor matched and ``additionalProperties`` is ``false``, or
      ``propertyNames`` rejects the name (checked with the Validator). A
      boolean schema permits every field (``true``) or none (``false``).

    Anything the tests cannot read (a ``$ref`` branch, a Pattern or a
    ``propertyNames`` schema the Validator cannot compile) counts as
    overlapping, so ``oneOf`` is only emitted when every object matches at
    most one branch; every object matches its own Variant's branch, so it
    then matches exactly one. With the default open branches, the second
    test never applies; a closed Record (``additionalProperties: false`` in
    each branch, task 16.1) makes Variants with a required field the other
    does not list disjoint.

Decision_Model features
    A Decision_Model in effect scores the two candidates on
    :func:`merge_split_features`: the number of candidate Variants; the
    minimum, mean, and maximum pairwise Jaccard similarity of their key
    sets; the share of keys held by two or more candidate Variants with the
    same observed types in each (type agreement on shared keys); the
    entropy in bits of the candidate Variants' object counts; and the MDL
    difference of merge and split normalized by the Observations scored.
    Task 22.1 replaces them with the versioned featurizer of
    ``learn/features.py`` (adding the design's scalar-field mutual
    information).

Hooks
    The analyzer walk (task 16.1) calls :meth:`VariantAnalyzer.select` on
    the object rows of a Record after Closed-or-Open-Record (design walk,
    step 2.3), then :meth:`VariantAnalyzer.merge_or_split` and
    :meth:`VariantAnalyzer.branches` (or :meth:`VariantAnalyzer.variant_node`
    for all three), passing a ``variant_schema`` that builds each Variant's
    Record node: ``type``, the Record keywords of
    :meth:`~jsinfer.structure.record_map.RecordMapAnalyzer.record_node` over
    the Variant's objects with its ``analyze`` as ``child_schema``,
    ``required`` from :func:`~jsinfer.deps.presence.required_fields` of the
    Variant's objects, and rules from the Dependency_Miner. A closed Record puts
    ``additionalProperties: false`` inside each branch: next to an ``anyOf``
    without ``properties`` it would reject every key. When the Path also
    holds other JSON types, the analyzer's node is an ``anyOf`` of the
    Variant branches plus one branch for those types (design walk, step 4):
    :meth:`MergeSplitOutcome.branch_provenance` then takes ``total``, the
    instances at the Path, and :meth:`MergeSplitOutcome.remaining_provenance`
    gives the other-types branch its record from the same decision. A
    Discriminator (task 34.1) replaces the Variant branches with its value
    groups. The cross-Path Merge-or-Split of Shared_Definitions (8.3, task
    36.1) uses the same kind, Decision_Model, and candidate labels over
    union views.
    MDL exception encoding (task 38.2) may add exception-bearing grouping
    and Merge-or-Split candidates.

Determinism
    Signatures, groups, and candidate pairs are handled in canonical order;
    random streams are keyed by the Technique, the Path, and the component
    count (never by execution order); and description lengths depend only on
    the multiset of Observations. Groupings therefore do not depend on
    document order, chunking, or worker count. Sets are never iterated: the
    code keeps insertion-ordered dicts and sorted lists.
"""

from __future__ import annotations

import math
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

import numpy as np
from datasketch import MinHash, MinHashLSH
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from scipy.special import logsumexp
from sklearn.cluster import DBSCAN, HDBSCAN

from jsinfer.config.registry import OUTCOME_ONE_VARIANT, REGISTRY
from jsinfer.config.resources import ResourceReader
from jsinfer.core.determinism import DEFAULT_SEED, random_state, rng
from jsinfer.core.jsonvalue import JSON, CanonicalKey, JType, canonical_key
from jsinfer.core.observations import isin_sorted, ranges_to_rows
from jsinfer.core.paths import MAPVAL_STEP, Path, render_path
from jsinfer.core.provenance import CandidateScore, Cutoff, ProvenanceRecord, SampleInfo, Support
from jsinfer.core.report import InferenceReport
from jsinfer.core.sampling import sample_rows
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import (
    JUDGMENT_OUTCOMES,
    ArtifactSource,
    Candidate,
    Decision,
    DecisionEngine,
    DecisionPoint,
)
from jsinfer.io.validator import CompileError, Validator
from jsinfer.mdl.codes import CAT, LN0, UNI
from jsinfer.mdl.scorer import EXCEPTION_FLAG_BITS, KEYWORD_VOCABULARY_SIZE, MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.record_map import object_rows

__all__ = [
    "ANY_OF",
    "BERNOULLI_MIXTURE",
    "DBSCAN_TECHNIQUE",
    "DM_MERGE_SPLIT",
    "GROUPING_TECHNIQUES",
    "HDBSCAN_TECHNIQUE",
    "MERGE",
    "MERGE_OR_SPLIT",
    "MINHASH_LSH",
    "ONE_OF",
    "ONE_VARIANT",
    "SPLIT",
    "VARIANT_GROUPING",
    "VARIANT_RESOURCES",
    "GroupingScorer",
    "Grouping",
    "MergeSplitFeatures",
    "MergeSplitOutcome",
    "ObjectSignatures",
    "VariantAnalyzer",
    "VariantGrouping",
    "VariantNode",
    "VariantSchema",
    "assign_by_description_length",
    "bernoulli_mixture_grouping",
    "branches_disjoint",
    "canonical_groups",
    "dbscan_grouping",
    "groups_from_labels",
    "hdbscan_grouping",
    "merge_or_split",
    "merge_split_features",
    "minhash_lsh_grouping",
    "pairwise_disjoint",
    "permits_field",
    "signature_distances",
    "variant_branches",
]

VARIANT_GROUPING: Final = "variant_grouping"
MINHASH_LSH: Final = "variants.minhash_lsh"
DBSCAN_TECHNIQUE: Final = "variants.dbscan"
HDBSCAN_TECHNIQUE: Final = "variants.hdbscan"
BERNOULLI_MIXTURE: Final = "variants.bernoulli_mixture"
ONE_VARIANT: Final = OUTCOME_ONE_VARIANT

# The Merge-or-Split Judgment_Call, its Decision_Model, and its two outcomes (5.3).
MERGE_OR_SPLIT: Final = "merge_or_split"
DM_MERGE_SPLIT: Final = "decide.dm_merge_split"
MERGE, SPLIT = JUDGMENT_OUTCOMES[MERGE_OR_SPLIT]

# The combinators of two or more Variant branches (5.4, 5.5).
ANY_OF: Final = "anyOf"
ONE_OF: Final = "oneOf"
Combinator: TypeAlias = Literal["anyOf", "oneOf"]

# The grouping Techniques in registry priority order (5.11).
GROUPING_TECHNIQUES: Final[tuple[str, ...]] = REGISTRY.decision_point(VARIANT_GROUPING).served_by

# Learned_Thresholds read by the grouping Techniques.
_LSH_THRESHOLD: Final = "lsh.keyset_jaccard"
_DBSCAN_EPS: Final = "dbscan.eps"
_DBSCAN_MIN_SAMPLES: Final = "dbscan.min_samples"
_HDBSCAN_MIN_CLUSTER_SIZE: Final = "hdbscan.min_cluster_size"
_BIC: Final = "BIC"

# The Resource_Parameters this module reads.
VARIANT_RESOURCES: Final[tuple[str, ...]] = (
    "lsh.bands",
    "minhash.perm",
    "sample.dbscan",
    "sample.hdbscan",
    "variants.em_max_iter",
    "variants.em_tol",
    "variants.max_components",
)

# Object_Signature entries are coded as key id << 3 | type rank (ranks are 0..6).
_TYPE_BITS: Final = 3
_TYPE_MASK: Final = (1 << _TYPE_BITS) - 1

# The anyOf keyword of a grouping with two or more groups: LN0(#kw = 1) + UNI(|KW|), then LN0(G).
_ANY_OF_KEYWORD_BITS: Final = math.fsum((LN0(1), UNI(KEYWORD_VOCABULARY_SIZE)))

# Jeffreys (KT) pseudo-count of the Bernoulli-mixture estimates.
_KT: Final = 0.5

Groups = tuple[tuple[int, ...], ...]


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


# ---------------------------------------------------------------------------
# Object_Signatures
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class ObjectSignatures:
    """The weighted, canonically sorted Object_Signatures of the non-flagged objects of a Path.

    ``objects`` are the object rows (see the module docstring). ``codes[s]``
    is signature ``s`` as ascending ``key id << 3 | type rank`` codes, and
    ``counts[s]`` the number of objects that have it. ``assignment[i]`` is
    the signature index of the ``i``-th object row of ``objects`` in view
    order. Signatures are in canonical order, so indices do not depend on
    document order.
    """

    objects: View
    codes: tuple[tuple[int, ...], ...]
    counts: np.ndarray
    assignment: np.ndarray

    @classmethod
    def of(cls, view: View) -> ObjectSignatures:
        """The signatures of the non-flagged object rows of ``view``; ``ValueError`` when it has none."""
        objects = object_rows(view)
        if objects.is_empty:
            raise ValueError(f"no non-flagged object at {render_path(objects.path)}")
        store = objects.store
        owners: list[np.ndarray] = []
        entries: list[np.ndarray] = []
        offset = 0
        for part in objects.parts:
            members = store.columns.get(part.column + (MAPVAL_STEP,))
            if members is not None:
                lo, hi = members.runs(part.rows)
                rows = ranges_to_rows(lo, hi)
                if len(rows):
                    ranks = members.jtype[rows].astype(np.int64)
                    ranks[ranks == JType.INTEGER] = JType.NUMBER
                    entries.append((members.key[rows].astype(np.int64) << _TYPE_BITS) | ranks)
                    owners.append(np.repeat(np.arange(offset, offset + len(part.rows), dtype=np.int64), hi - lo))
            offset += len(part.rows)
        n = offset
        if entries:
            code = np.concatenate(entries)
            owner = np.concatenate(owners)
            order = np.lexsort((code, owner))
            code, owner = np.ascontiguousarray(code[order], dtype="<i8"), owner[order]
            ends = np.cumsum(np.bincount(owner, minlength=n))
        else:
            code = np.empty(0, dtype="<i8")
            ends = np.zeros(n, dtype=np.int64)
        buffer = code.tobytes()
        width = code.itemsize
        provisional: dict[bytes, int] = {}
        first = np.empty(n, dtype=np.int64)
        start = 0
        for index, end in enumerate(ends.tolist()):
            first[index] = provisional.setdefault(buffer[start * width : end * width], len(provisional))
            start = end
        decoded = [tuple(np.frombuffer(raw, dtype="<i8").tolist()) for raw in provisional]
        order = sorted(range(len(decoded)), key=decoded.__getitem__)
        rank = np.empty(len(decoded), dtype=np.int64)
        rank[np.array(order, dtype=np.int64)] = np.arange(len(decoded), dtype=np.int64)
        assignment = rank[first]
        counts = np.bincount(assignment, minlength=len(decoded)).astype(np.int64)
        for arr in (assignment, counts):
            arr.setflags(write=False)
        return cls(objects, tuple(decoded[s] for s in order), counts, assignment)

    # -- inspection ---------------------------------------------------------

    @property
    def path(self) -> Path:
        return self.objects.path

    @property
    def n_signatures(self) -> int:
        return len(self.codes)

    @property
    def n_objects(self) -> int:
        return len(self.assignment)

    def key_ids(self, index: int) -> tuple[int, ...]:
        """The key ids of signature ``index``, ascending (code-point order of the names)."""
        return tuple(code >> _TYPE_BITS for code in self.codes[index])

    def entries(self, index: int) -> tuple[tuple[str, str], ...]:
        """Signature ``index`` as ``(key, type)`` pairs in code-point order of the keys."""
        store = self.objects.store
        return tuple(
            (store.key_name(code >> _TYPE_BITS), JType(code & _TYPE_MASK).keyword) for code in self.codes[index]
        )

    def describe(self, index: int) -> dict[str, JSON]:
        """Signature ``index`` as JSON: its ``(key, type)`` entries and its object count."""
        return {"entries": [list(entry) for entry in self.entries(index)], "objects": int(self.counts[index])}

    def count_of(self, members: Sequence[int]) -> int:
        """The number of objects whose signature is in ``members``."""
        return int(self.counts[np.asarray(members, dtype=np.int64)].sum())

    def _matrix(self, keyed: Sequence[tuple[int, ...]]) -> np.ndarray:
        columns = np.unique(np.array([value for row in keyed for value in row], dtype=np.int64))
        out = np.zeros((len(keyed), len(columns)), dtype=bool)
        for row, values in enumerate(keyed):
            if values:
                out[row, np.searchsorted(columns, np.array(values, dtype=np.int64))] = True
        return out

    def key_presence(self) -> np.ndarray:
        """``(#signatures, #keys)`` bool matrix: which keys each signature holds, keys in code-point order."""
        return self._matrix([self.key_ids(s) for s in range(self.n_signatures)])

    def features(self) -> np.ndarray:
        """``(#signatures, #entries)`` bool matrix over the distinct ``(key, type)`` entries."""
        return self._matrix(self.codes)

    def view(self, members: Sequence[int]) -> View:
        """The objects whose signature is in ``members`` (sorted signature indices): a candidate Variant."""
        wanted = np.asarray(members, dtype=np.int64)
        if len(wanted) == self.n_signatures:
            return self.objects
        return self.objects.subset(isin_sorted(self.assignment, wanted))

    def check_partition(self, groups: Groups) -> None:
        """``ValueError`` unless ``groups`` (canonical) partition the signature indices 0 … S − 1."""
        flat = [s for group in groups for s in group]
        if sorted(flat) != list(range(self.n_signatures)):
            raise ValueError(
                f"a grouping must partition the {self.n_signatures} signatures at {render_path(self.path)}, "
                f"got {list(groups)!r}"
            )


def _similarities(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Jaccard similarities between the 0/1 float rows of ``a`` and ``b``; two empty rows are equal (1).

    The counts are exact in float64, so each similarity is the exact ratio rounded once.
    """
    similarity = a @ b.T  # intersections
    union = np.add.outer(a.sum(axis=1), b.sum(axis=1))
    union -= similarity
    np.divide(similarity, union, out=similarity, where=union > 0)
    similarity[union == 0] = 1.0
    return similarity


def signature_distances(signatures: ObjectSignatures, indices: np.ndarray | None = None) -> np.ndarray:
    """Jaccard distances ``1 − |A ∩ B| / |A ∪ B|`` between signatures as sets of ``(key, type)`` entries.

    ``indices`` selects the signatures (all by default). Two empty signatures
    have distance 0.
    """
    x = signatures.features().astype(np.float64)
    if indices is not None:
        x = x[np.asarray(indices, dtype=np.int64)]
    distances = _similarities(x, x)
    np.subtract(1.0, distances, out=distances)
    np.fill_diagonal(distances, 0.0)
    return distances


# Rows per block when matching unsampled signatures and estimating LSH pair similarities;
# they bound memory and never change a result.
_NEAREST_BLOCK: Final = 1024
_PAIR_BLOCK: Final = 65_536


def _nearest(signatures: ObjectSignatures, sampled: np.ndarray, others: np.ndarray) -> np.ndarray:
    """For each signature in ``others``, the position in ``sampled`` of its nearest sampled signature.

    Nearest is the smallest Jaccard distance over ``(key, type)`` entries,
    ties to the first sampled signature.
    """
    x = signatures.features().astype(np.float64)
    reference = x[sampled]
    out = np.empty(len(others), dtype=np.int64)
    for start in range(0, len(others), _NEAREST_BLOCK):
        block = others[start : start + _NEAREST_BLOCK]
        out[start : start + len(block)] = _similarities(x[block], reference).argmax(axis=1)
    return out


# ---------------------------------------------------------------------------
# Groupings
# ---------------------------------------------------------------------------


def canonical_groups(groups: Sequence[Sequence[int]]) -> Groups:
    """``groups`` with each group sorted and the groups ordered by their first index.

    Raises ``ValueError`` for an empty group or an index in two groups.
    """
    normalized: list[tuple[int, ...]] = []
    seen: dict[int, None] = {}
    for group in groups:
        members = tuple(sorted(int(s) for s in group))
        if not members:
            raise ValueError("a candidate Variant needs at least one signature")
        for s in members:
            if s < 0:
                raise ValueError(f"signature indices are non-negative, got {s}")
            if s in seen:
                raise ValueError(f"signature {s} is in two candidate Variants")
            seen[s] = None
        normalized.append(members)
    return tuple(sorted(normalized))


def groups_from_labels(labels: np.ndarray, indices: np.ndarray | None = None) -> Groups:
    """The groups of signatures sharing a label; negative labels (noise) are left out.

    ``indices[i]`` is the signature index of ``labels[i]`` (``i`` by default).
    """
    labels = np.asarray(labels, dtype=np.int64)
    where = np.arange(len(labels), dtype=np.int64) if indices is None else np.asarray(indices, dtype=np.int64)
    by_label: dict[int, list[int]] = {}
    for label, s in zip(labels.tolist(), where.tolist()):
        if label >= 0:
            by_label.setdefault(label, []).append(s)
    return canonical_groups(list(by_label.values()))


@dataclass(frozen=True, eq=False)
class Grouping:
    """A candidate Variant grouping: a canonical partition of the signature indices.

    ``technique`` is the producing Technique (None for the one-Variant
    grouping of the Fallback_Behavior or of a single signature). ``cutoff``
    is the cut-off the Technique applied (a Learned_Threshold or the BIC
    comparison), ``sample`` its sample when it clustered one (29.8), and
    ``details`` the Technique's other facts as JSON.
    """

    technique: str | None
    groups: Groups
    cutoff: Cutoff | None = None
    sample: SampleInfo | None = None
    details: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if self.technique is not None and self.technique not in GROUPING_TECHNIQUES:
            raise ValueError(
                f"{self.technique!r} is not a Variant grouping Technique ({', '.join(GROUPING_TECHNIQUES)})"
            )
        groups = canonical_groups(self.groups)
        if not groups:
            raise ValueError("a grouping needs at least one candidate Variant")
        _set(self, "groups", groups)
        if self.cutoff is not None and not isinstance(self.cutoff, Cutoff):
            raise TypeError(f"cutoff must be a Cutoff or None, got {type(self.cutoff).__name__}")
        if self.sample is not None and not isinstance(self.sample, SampleInfo):
            raise TypeError(f"sample must be a SampleInfo or None, got {type(self.sample).__name__}")
        _set(self, "details", MappingProxyType(dict(self.details)))

    @classmethod
    def one(cls, n_signatures: int, technique: str | None = None, **kwargs: Any) -> Grouping:
        """Every signature in one candidate Variant."""
        return cls(technique, (tuple(range(n_signatures)),), **kwargs)

    @property
    def n_variants(self) -> int:
        return len(self.groups)

    @property
    def label(self) -> str:
        """The outcome label: ``one-variant`` or ``<n>-variants``."""
        return ONE_VARIANT if self.n_variants == 1 else f"{self.n_variants}-variants"


# ---------------------------------------------------------------------------
# Description lengths of groupings
# ---------------------------------------------------------------------------


def _branch_overhead(n_groups: int) -> float:
    """The bits a grouping of ``n_groups >= 2`` adds to its groups beyond the branch choice."""
    return math.fsum((_ANY_OF_KEYWORD_BITS, LN0(n_groups), EXCEPTION_FLAG_BITS))


_LOG2_GAMMA_HALF: Final = math.lgamma(0.5) / math.log(2)


def _kt_cell(count: int) -> float:
    """``log2 Γ(c + ½) − log2 Γ(½)``: the per-category term that ``CAT`` subtracts for a category of ``count``."""
    return math.lgamma(count + 0.5) / math.log(2) - _LOG2_GAMMA_HALF


class GroupingScorer:
    """Description lengths of groupings of one Path's signatures (see the module docstring).

    ``group_bits`` scores the Naive_Merge subschema of a group on its own
    objects and caches it; ``bits`` evaluates a grouping from the cache;
    ``score`` scores a grouping's encoding with the MDL_Scorer itself, as the
    DecisionEngine compares it, and caches it per grouping, so Merge-or-Split
    reuses the selected grouping's score. The caches belong to one Path's
    analysis at one flag epoch.
    """

    __slots__ = ("_cache", "_counts", "_scorer", "_scores", "_signatures")

    def __init__(self, signatures: ObjectSignatures, scorer: MDLScorer) -> None:
        if not isinstance(signatures, ObjectSignatures):
            raise TypeError(f"signatures must be ObjectSignatures, got {type(signatures).__name__}")
        if not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer, got {type(scorer).__name__}")
        self._signatures = signatures
        self._scorer = scorer
        self._cache: dict[tuple[int, ...], float] = {}
        self._counts: dict[tuple[int, ...], int] = {}
        self._scores: dict[Groups, Score] = {}

    @property
    def signatures(self) -> ObjectSignatures:
        return self._signatures

    @property
    def scorer(self) -> MDLScorer:
        return self._scorer

    def _count(self, members: tuple[int, ...]) -> int:
        count = self._counts.get(members)
        if count is None:
            count = self._counts[members] = self._signatures.count_of(members)
        return count

    def group_bits(self, members: tuple[int, ...]) -> float:
        """``L(g)``: the description length of the Naive_Merge subschema of group ``members`` on its objects."""
        bits = self._cache.get(members)
        if bits is None:
            view = self._signatures.view(members)
            bits = self._scorer.score(naive_merge_node(view), view).total
            self._cache[members] = bits
        return bits

    def bits(self, groups: Sequence[tuple[int, ...]]) -> float:
        """The description length of a grouping (canonical groups), from the cached group lengths."""
        if len(groups) == 1:
            return self.group_bits(groups[0])
        counts = [self._count(group) for group in groups]
        terms = [_branch_overhead(len(groups)), CAT(counts)]
        terms.extend(self.group_bits(group) for group in groups)
        return math.fsum(terms)

    def merge_term(self, a: tuple[int, ...], b: tuple[int, ...]) -> float:
        """The part of the change in description length from merging groups ``a`` and ``b`` that depends on them.

        ``L(a ∪ b) − L(a) − L(b)`` plus the change of the two categories in
        ``CAT``. The rest of the change (the ``anyOf`` overhead and the
        ``CAT`` normalizer, which depend only on the group count and the
        object total) is the same for every pair of one grouping, and for
        every group that one signature could join, so this term ranks the
        merges and the joins.
        """
        ca, cb = self._count(a), self._count(b)
        merged = tuple(sorted(a + b))
        return math.fsum(
            (
                self.group_bits(merged),
                -self.group_bits(a),
                -self.group_bits(b),
                _kt_cell(ca),
                _kt_cell(cb),
                -_kt_cell(ca + cb),
            )
        )

    def encoding(self, groups: Groups) -> tuple[SchemaNode, dict[int, list[View]] | None]:
        """The MDL encoding of a grouping and its ``branch_views`` (None for one group)."""
        views = [self._signatures.view(group) for group in groups]
        if len(views) == 1:
            return naive_merge_node(views[0]), None
        node = SchemaNode(kw={"anyOf": [naive_merge_node(view) for view in views]})
        return node, {node.nid: views}

    def score(self, groups: Groups) -> Score:
        """The MDL_Scorer's :class:`Score` of the grouping's encoding on the Path's objects (cached)."""
        key = canonical_groups(groups)
        cached = self._scores.get(key)
        if cached is None:
            node, branch_views = self.encoding(key)
            cached = self._scorer.score(node, self._signatures.objects, branch_views=branch_views)
            self._scores[key] = cached
        return cached


def _with_member(groups: list[tuple[int, ...]], at: int, s: int) -> list[tuple[int, ...]]:
    """``groups`` with signature ``s`` added to group ``at`` (``at == len(groups)``: a new group), canonical."""
    if at == len(groups):
        return sorted([*groups, (s,)])
    out = list(groups)
    out[at] = tuple(sorted((*groups[at], s)))
    return sorted(out)


def assign_by_description_length(groups: Groups, pending: Sequence[int], scorer: GroupingScorer) -> Groups:
    """Assign each pending signature, in the given order, to the existing or new group with the lowest DL (5.9).

    Existing groups are tried first, in canonical order, then a new group;
    the first lowest description length wins, and a group opened by an
    earlier pending signature is an existing group for later ones. The joins
    are ranked by :meth:`GroupingScorer.merge_term`, which differs from their
    description lengths by a term shared by every join, and the best join is
    compared exactly with the new group. Returns the canonical groups.
    """
    current = list(canonical_groups(groups))
    for s in pending:
        single = (int(s),)
        if not current:
            current = [single]
            continue
        at = min(range(len(current)), key=lambda index: scorer.merge_term(current[index], single))
        joined = _with_member(current, at, single[0])
        opened = _with_member(current, len(current), single[0])
        current = opened if scorer.bits(opened) < scorer.bits(joined) else joined
    return canonical_groups(current)


# ---------------------------------------------------------------------------
# variants.minhash_lsh (5.7)
# ---------------------------------------------------------------------------


def _spanning_links(n: int, pairs: Sequence[tuple[int, int, float]]) -> list[tuple[int, int]]:
    """The maximum spanning forest of the retrieved pairs ``(i, j, estimate)``: Kruskal, ties by ``(i, j)``."""
    parent = list(range(n))

    def find(x: int) -> int:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    links: list[tuple[int, int]] = []
    for _, i, j in sorted((-estimate, i, j) for i, j, estimate in pairs):
        ri, rj = find(i), find(j)
        if ri != rj:
            parent[max(ri, rj)] = min(ri, rj)
            links.append((i, j))
    return links


def _greedy_merge(
    groups: Groups, links: Sequence[tuple[int, int]], scorer: GroupingScorer
) -> tuple[Groups, int, int]:
    """Merge linked groups greedily while the description length decreases; see the module docstring.

    ``links`` are signature pairs, strongest first. Each pass walks them in
    order and merges the two groups a link joins when that lowers the exact
    description length of the grouping; passes repeat until one merges
    nothing. Returns the groups, the number of merges, and of passes.
    """
    owner = list(range(scorer.signatures.n_signatures))  # union-find over signatures; a root names its group
    for group in groups:
        for s in group:
            owner[s] = group[0]
    members: dict[int, tuple[int, ...]] = {group[0]: group for group in groups}

    def find(s: int) -> int:
        while owner[s] != s:
            owner[s] = owner[owner[s]]
            s = owner[s]
        return s

    # The length of a grouping depends only on the multiset of its groups, so lists need no sorting here.
    current_bits = scorer.bits(list(members.values()))
    merges = passes = 0
    merged_any = True
    while merged_any and len(members) > 1:
        merged_any = False
        passes += 1
        for i, j in links:
            a, b = find(i), find(j)
            if a == b:
                continue
            merged = tuple(sorted(members[a] + members[b]))
            trial = [group for name, group in members.items() if name != a and name != b]
            trial.append(merged)
            trial_bits = scorer.bits(trial)
            if trial_bits < current_bits:
                root, other = min(a, b), max(a, b)
                owner[other] = root
                del members[other]
                members[root] = merged
                current_bits = trial_bits
                merges += 1
                merged_any = True
    return canonical_groups(list(members.values())), merges, passes


def minhash_lsh_grouping(
    signatures: ObjectSignatures,
    scorer: GroupingScorer,
    *,
    threshold: float,
    perm: int,
    bands: int,
    seed: int = DEFAULT_SEED,
) -> Grouping:
    """The ``variants.minhash_lsh`` grouping (5.7); see the module docstring.

    ``threshold`` is ``lsh.keyset_jaccard``; ``perm`` and ``bands`` are
    ``minhash.perm`` and ``lsh.bands`` (``bands <= perm``).
    """
    if not 1 <= bands <= perm:
        raise ValueError(f"lsh.bands ({bands}) must lie in [1, minhash.perm ({perm})]")
    rows = perm // bands
    store = signatures.objects.store
    hash_seed = random_state(MINHASH_LSH, render_path(signatures.path), seed=seed)
    by_keys: dict[tuple[int, ...], MinHash] = {}
    sketches: list[MinHash] = []
    for s in range(signatures.n_signatures):
        kids = signatures.key_ids(s)
        sketch = by_keys.get(kids)
        if sketch is None:
            sketch = MinHash(num_perm=perm, seed=hash_seed)
            if kids:
                sketch.update_batch([store.key_name(kid).encode("utf-8", "surrogatepass") for kid in kids])
            by_keys[kids] = sketch
        sketches.append(sketch)
    pairs: list[tuple[int, int]] = []
    if bands == 1:
        # datasketch's MinHashLSH needs two or more bands. One band of all ``perm`` rows
        # retrieves exactly the pairs whose sketches are equal.
        buckets: dict[bytes, list[int]] = {}
        for s, sketch in enumerate(sketches):
            buckets.setdefault(np.ascontiguousarray(sketch.hashvalues, dtype="<u8").tobytes(), []).append(s)
        for members in buckets.values():
            pairs.extend((i, j) for a, i in enumerate(members) for j in members[a + 1 :])
        pairs.sort()
    else:
        index = MinHashLSH(num_perm=perm, params=(bands, rows))
        for s, sketch in enumerate(sketches):
            index.insert(s, sketch)
        for i, sketch in enumerate(sketches):
            pairs.extend((i, j) for j in sorted(int(j) for j in index.query(sketch)) if j > i)
    # MinHash.jaccard, vectorized: the share of equal hash values.
    hashes = np.stack([sketch.hashvalues for sketch in sketches])
    estimates: list[float] = []
    for start in range(0, len(pairs), _PAIR_BLOCK):
        block = np.array(pairs[start : start + _PAIR_BLOCK], dtype=np.int64).reshape(-1, 2)
        equal = np.count_nonzero(hashes[block[:, 0]] == hashes[block[:, 1]], axis=1)
        estimates.extend((equal / perm).tolist())
    retrieved = [(i, j, estimate) for (i, j), estimate in zip(pairs, estimates)]
    edges = [(i, j) for i, j, estimate in retrieved if estimate >= threshold]
    n = signatures.n_signatures
    heads = np.array([i for i, _ in edges], dtype=np.int64)
    tails = np.array([j for _, j in edges], dtype=np.int64)
    graph = coo_matrix((np.ones(len(edges)), (heads, tails)), shape=(n, n)).tocsr()
    n_components, labels = connected_components(graph, directed=False)
    links = _spanning_links(n, retrieved)
    groups, merges, passes = _greedy_merge(groups_from_labels(labels), links, scorer)
    details: dict[str, JSON] = {
        "bands": bands,
        "candidate_pairs": len(retrieved),
        "components": int(n_components),
        "edges": len(edges),
        "keyset_jaccard": threshold,
        "merge_passes": passes,
        "merges": merges,
        "permutations": perm,
        "rows_per_band": rows,
        "spanning_links": len(links),
    }
    return Grouping(MINHASH_LSH, groups, cutoff=Cutoff(_LSH_THRESHOLD, value=threshold), details=details)


# ---------------------------------------------------------------------------
# variants.dbscan and variants.hdbscan (5.8, 5.9, 29.8)
# ---------------------------------------------------------------------------


def _clustered(
    technique: str,
    signatures: ObjectSignatures,
    scorer: GroupingScorer,
    sample_size: int,
    seed: int,
    cluster: Any,
) -> tuple[Groups, SampleInfo | None, dict[str, JSON]]:
    """Cluster a (sampled) set of signatures with ``cluster(indices)``; see the module docstring.

    Unsampled signatures take the label of their nearest sampled signature;
    noise is then assigned by description length (5.9).
    """
    n = signatures.n_signatures
    everything = np.arange(n, dtype=np.int64)
    indices = sample_rows(everything, sample_size, rng(technique, render_path(signatures.path), seed=seed))
    labels = np.full(n, -1, dtype=np.int64)
    labels[indices] = np.asarray(cluster(indices), dtype=np.int64)
    unsampled = everything[~isin_sorted(everything, indices)]
    if len(unsampled):
        labels[unsampled] = labels[indices[_nearest(signatures, indices, unsampled)]]
    clusters = groups_from_labels(labels)
    noise = everything[labels < 0]
    groups = assign_by_description_length(clusters, noise.tolist(), scorer)
    sample = SampleInfo(len(indices), n) if len(indices) < n else None
    details: dict[str, JSON] = {
        "clusters": len(clusters),
        "noise_signatures": len(noise),
        "noise_objects": signatures.count_of(noise.tolist()),
    }
    if sample is not None:
        details["unsampled_signatures"] = len(unsampled)
    return groups, sample, details


def dbscan_grouping(
    signatures: ObjectSignatures,
    scorer: GroupingScorer,
    *,
    eps: float,
    min_samples: int,
    sample_size: int,
    seed: int = DEFAULT_SEED,
) -> Grouping:
    """The ``variants.dbscan`` grouping (5.8, 5.9, 29.8); see the module docstring."""
    min_samples = int(min_samples)

    def cluster(indices: np.ndarray) -> np.ndarray:
        model = DBSCAN(eps=float(eps), min_samples=min_samples, metric="precomputed")
        weights = signatures.counts[indices].astype(np.float64)
        return model.fit(signature_distances(signatures, indices), sample_weight=weights).labels_

    groups, sample, details = _clustered(DBSCAN_TECHNIQUE, signatures, scorer, sample_size, seed, cluster)
    details.update({"eps": float(eps), "min_samples": min_samples})
    cutoff = Cutoff(_DBSCAN_EPS, value=float(eps))
    return Grouping(DBSCAN_TECHNIQUE, groups, cutoff=cutoff, sample=sample, details=details)


def hdbscan_grouping(
    signatures: ObjectSignatures,
    scorer: GroupingScorer,
    *,
    min_cluster_size: int,
    sample_size: int,
    seed: int = DEFAULT_SEED,
) -> Grouping:
    """The ``variants.hdbscan`` grouping (5.8, 5.9, 29.8); see the module docstring."""
    min_cluster_size = int(min_cluster_size)

    def cluster(indices: np.ndarray) -> np.ndarray:
        if min_cluster_size > len(indices):  # no cluster can form: every signature is noise
            return np.full(len(indices), -1, dtype=np.int64)
        model = HDBSCAN(min_cluster_size=min_cluster_size, metric="precomputed", copy=True)
        return model.fit(signature_distances(signatures, indices)).labels_

    groups, sample, details = _clustered(HDBSCAN_TECHNIQUE, signatures, scorer, sample_size, seed, cluster)
    details["min_cluster_size"] = min_cluster_size
    return Grouping(
        HDBSCAN_TECHNIQUE,
        groups,
        cutoff=Cutoff(_HDBSCAN_MIN_CLUSTER_SIZE, value=float(min_cluster_size)),
        sample=sample,
        details=details,
    )


# ---------------------------------------------------------------------------
# variants.bernoulli_mixture (5.2)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class _MixtureFit:
    components: int
    log_likelihood: float
    iterations: int
    labels: np.ndarray

    def bic(self, n_keys: int, n_objects: float) -> float:
        parameters = self.components * n_keys + self.components - 1
        return -2.0 * self.log_likelihood + parameters * math.log(n_objects)


def _bernoulli_em(
    x: np.ndarray, w: np.ndarray, k: int, stream: np.random.Generator, max_iter: int, tol: float
) -> _MixtureFit:
    """Weighted EM for a k-component Bernoulli mixture with Jeffreys-smoothed estimates."""
    n_objects = float(w.sum())
    starts = stream.choice(len(x), size=k, replace=False, p=w / n_objects)
    centers = x[starts]
    hamming = x @ (1.0 - centers).T + (1.0 - x) @ centers.T
    resp = np.zeros((len(x), k))
    resp[np.arange(len(x)), hamming.argmin(axis=1)] = 1.0
    log_likelihood = -math.inf
    joint = np.zeros((len(x), k))
    iterations = 0
    for iterations in range(1, max_iter + 1):
        weighted = resp * w[:, None]
        nk = weighted.sum(axis=0)
        log_pi = np.log((nk + _KT) / (n_objects + _KT * k))
        theta = (weighted.T @ x + _KT) / (nk[:, None] + 2.0 * _KT)
        joint = x @ np.log(theta).T + (1.0 - x) @ np.log1p(-theta).T + log_pi
        norm = logsumexp(joint, axis=1)
        resp = np.exp(joint - norm[:, None])
        new = math.fsum((w * norm).tolist())
        converged = iterations > 1 and (new - log_likelihood) / n_objects < tol
        log_likelihood = new
        if converged:
            break
    return _MixtureFit(k, log_likelihood, iterations, joint.argmax(axis=1))


def bernoulli_mixture_grouping(
    signatures: ObjectSignatures,
    *,
    max_components: int,
    max_iter: int,
    tol: float,
    seed: int = DEFAULT_SEED,
) -> Grouping:
    """The ``variants.bernoulli_mixture`` grouping (5.2): weighted EM per k, BIC selection; see the module docstring."""
    x = signatures.key_presence().astype(np.float64)
    w = signatures.counts.astype(np.float64)
    n_objects = float(signatures.n_objects)
    path = render_path(signatures.path)
    fits: list[tuple[float, _MixtureFit]] = []
    for k in range(1, min(signatures.n_signatures, max_components) + 1):
        fit = _bernoulli_em(x, w, k, rng(BERNOULLI_MIXTURE, path, k, seed=seed), max_iter, tol)
        fits.append((fit.bic(x.shape[1], n_objects), fit))
    best_bic, best = min(fits, key=lambda item: (item[0], item[1].components))  # ties to the smaller k
    scores = tuple(CandidateScore(BERNOULLI_MIXTURE, f"{fit.components}-components", bic) for bic, fit in fits)
    details: dict[str, JSON] = {
        "bic": [
            {
                "bic": bic,
                "components": fit.components,
                "iterations": fit.iterations,
                "log_likelihood": fit.log_likelihood,
            }
            for bic, fit in fits
        ],
        "components": best.components,
        "max_components": max_components,
        "selected_bic": best_bic,
    }
    groups = groups_from_labels(best.labels)
    return Grouping(BERNOULLI_MIXTURE, groups, cutoff=Cutoff(_BIC, scores=scores), details=details)


# ---------------------------------------------------------------------------
# Selection
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False)
class VariantGrouping:
    """The candidate Variants of one object Path (see the module docstring).

    ``grouping`` is the selected grouping: a Technique's, or the one-Variant
    grouping of the Fallback_Behavior (``decision.method == "fallback"``) or
    of a single signature (``decision`` None, no Decision_Point).
    ``candidates`` are the groupings that were offered. ``scorer`` holds the
    cached description lengths of the candidate Variants, for Merge-or-Split.
    """

    path: Path
    signatures: ObjectSignatures
    grouping: Grouping
    decision: Decision | None
    candidates: tuple[Grouping, ...] = ()
    scorer: GroupingScorer | None = field(default=None, repr=False)

    def __post_init__(self) -> None:
        self.signatures.check_partition(self.grouping.groups)
        if self.decision is None and self.signatures.n_signatures > 1:
            raise ValueError("only a Path whose objects share one signature has no variant_grouping decision")

    @property
    def n_variants(self) -> int:
        return self.grouping.n_variants

    @property
    def sample(self) -> SampleInfo | None:
        """The sample the selected grouping's Technique clustered, if any (29.8, 26.4)."""
        return self.grouping.sample

    def views(self) -> tuple[View, ...]:
        """The candidate Variants' object views, in canonical order; together they partition the objects (5.1)."""
        return tuple(self.signatures.view(group) for group in self.grouping.groups)

    def details(self) -> dict[str, JSON]:
        """Facts for the Provenance_Records of the Variant Constraints: counts and the grouping's details."""
        details: dict[str, JSON] = {
            "candidate_variants": self.n_variants,
            "object_signatures": self.signatures.n_signatures,
            "variant_objects": [self.signatures.count_of(group) for group in self.grouping.groups],
            "variant_signatures": [len(group) for group in self.grouping.groups],
        }
        if self.grouping.technique is not None:
            details["grouping"] = {"technique": self.grouping.technique, **self.grouping.details}
        return details


# ---------------------------------------------------------------------------
# Merge-or-Split (5.3, 5.6, 5.11)
# ---------------------------------------------------------------------------

# Computes the Decision_Model features from the grouping and the merge and split candidates (task 22.1).
MergeSplitFeatures = Callable[[VariantGrouping, Sequence[Candidate]], np.ndarray]


def _group_types(signatures: ObjectSignatures, group: tuple[int, ...]) -> dict[int, tuple[int, ...]]:
    """Key id → the sorted type ranks observed under it, over the signatures of ``group`` (keys ascending)."""
    types: dict[int, dict[int, None]] = {}
    for s in group:
        for code in signatures.codes[s]:
            types.setdefault(code >> _TYPE_BITS, {})[code & _TYPE_MASK] = None
    return {kid: tuple(sorted(types[kid])) for kid in sorted(types)}


def merge_split_features(variants: VariantGrouping, candidates: Sequence[Candidate]) -> np.ndarray:
    """Default Merge-or-Split features (see the module docstring).

    ``candidates`` are the merge and split candidates, in that order, with
    their scores. The vector is: candidate Variants; minimum, mean, and
    maximum pairwise key-set Jaccard similarity; type agreement on shared
    keys; group-size entropy in bits; and the normalized MDL difference
    ``(DL(merge) − DL(split)) / n_scored``.
    """
    signatures = variants.signatures
    groups = variants.grouping.groups
    if len(groups) < 2:
        raise ValueError("Merge-or-Split features need two or more candidate Variants")
    typed = [_group_types(signatures, group) for group in groups]
    similarities: list[float] = []
    for i in range(len(typed)):
        for j in range(i + 1, len(typed)):
            shared = sum(1 for kid in typed[i] if kid in typed[j])
            union = len(typed[i]) + len(typed[j]) - shared
            similarities.append(shared / union if union else 1.0)  # two empty key sets are equal
    holders: dict[int, list[tuple[int, ...]]] = {}
    for types in typed:
        for kid, ranks in types.items():
            holders.setdefault(kid, []).append(ranks)
    shared_keys = [ranks for ranks in holders.values() if len(ranks) >= 2]
    agreeing = sum(1 for ranks in shared_keys if all(r == ranks[0] for r in ranks))
    agreement = agreeing / len(shared_keys) if shared_keys else 1.0
    counts = [signatures.count_of(group) for group in groups]
    total = sum(counts)
    entropy = -math.fsum((c / total) * math.log2(c / total) for c in counts)
    merged, split = (candidate.score for candidate in candidates)
    if merged is None or split is None:
        raise ValueError("Merge-or-Split features need the scores of both candidates")
    difference = (merged.total - split.total) / max(merged.n_scored, 1)
    return np.array(
        [
            len(groups),
            min(similarities),
            math.fsum(similarities) / len(similarities),
            max(similarities),
            agreement,
            entropy,
            difference,
        ],
        dtype=np.float64,
    )


@dataclass(frozen=True, eq=False)
class MergeSplitOutcome:
    """The Merge-or-Split judgment of one object Path (see the module docstring).

    ``decision`` is None exactly when the grouping has one candidate
    Variant, so there was nothing to merge (5.3). ``merged`` says whether
    the candidate Variants were merged into one Variant.
    """

    variants: VariantGrouping
    merged: bool
    decision: Decision | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.variants, VariantGrouping):
            raise TypeError(f"variants must be a VariantGrouping, got {type(self.variants).__name__}")
        several = self.variants.n_variants >= 2
        if several and self.decision is None:
            raise ValueError("two or more candidate Variants need a merge_or_split decision (5.3)")
        if not several and (self.decision is not None or self.merged):
            raise ValueError("one candidate Variant is neither merged nor split, so it has no decision")
        if self.decision is not None and self.decision.kind != MERGE_OR_SPLIT:
            raise ValueError(f"the decision must be a {MERGE_OR_SPLIT!r} decision, got {self.decision.kind!r}")

    @property
    def path(self) -> Path:
        return self.variants.path

    @property
    def judgment(self) -> str | None:
        """``merge`` or ``split``; None without a Judgment_Call (one candidate Variant)."""
        if self.decision is None:
            return None
        return MERGE if self.merged else SPLIT

    @property
    def groups(self) -> Groups:
        """The final Variants as signature groups: every signature in one group after a merge."""
        if self.merged:
            return (tuple(range(self.variants.signatures.n_signatures)),)
        return self.variants.grouping.groups

    @property
    def n_variants(self) -> int:
        """The number of final Variants."""
        return len(self.groups)

    def views(self) -> tuple[View, ...]:
        """The final Variants' object views, in canonical order; together they partition the objects."""
        signatures = self.variants.signatures
        return tuple(signatures.view(group) for group in self.groups)

    def details(self) -> dict[str, JSON]:
        """The judgment, the final Variant count, and the grouping facts (:meth:`VariantGrouping.details`)."""
        details: dict[str, JSON] = dict(self.variants.details())
        details["variants"] = self.n_variants
        if self.judgment is not None:
            details["judgment"] = self.judgment
        return details

    def _grouping_record(self) -> dict[str, JSON]:
        """The ``variant_grouping`` decision that produced the candidate Variants, as JSON (20.7, 26.3)."""
        decision = self.variants.decision
        assert decision is not None  # two or more candidate Variants come from a resolved grouping
        record: dict[str, JSON] = {
            "method": decision.method,
            "resolution": decision.resolution.to_json(),
            "technique": decision.technique,
        }
        if decision.confidence is not None:
            record["confidence"] = decision.confidence
        if decision.cutoff is not None:
            record["cutoff"] = decision.cutoff.to_json()
        if decision.fallback is not None:
            record["fallback"] = decision.fallback.to_json()
        return record

    def _check_split(self) -> None:
        if self.decision is None or self.merged:
            raise ValueError(
                f"the Variants at {render_path(self.path)} were not split, so they have no branch Constraints"
            )

    def _total(self, total: int | None) -> int:
        """The Support total of a branch: the Path's objects, or ``total`` instances when it is given."""
        n = self.variants.signatures.n_objects
        if total is None:
            return n
        if isinstance(total, bool) or not isinstance(total, (int, np.integer)) or total < n:
            raise ValueError(f"total must be an integer of at least the Path's {n} objects, got {total!r}")
        return int(total)

    def branch_provenance(
        self, index: int, *, details: Mapping[str, JSON] | None = None, total: int | None = None
    ) -> ProvenanceRecord:
        """The Provenance_Record of branch ``index`` of a split (see the module docstring).

        Support counts the Variant's objects among the Path's objects, or
        among ``total`` instances when the node also holds a branch for the
        Path's other JSON types (the analyzer's mixed-type rule, task 16.1);
        Confidence is the decision's (the Laplace default after a
        Fallback_Behavior). ``details`` are added to the record's.
        """
        self._check_split()
        groups = self.groups
        if isinstance(index, bool) or not isinstance(index, int) or not 0 <= index < len(groups):
            raise ValueError(f"branch index must lie in [0, {len(groups)}), got {index!r}")
        signatures = self.variants.signatures
        support = Support(signatures.count_of(groups[index]), self._total(total))
        extra: dict[str, JSON] = {**self.details(), "variant": index, "variant_grouping": self._grouping_record()}
        extra.update(details or {})
        return self.decision.provenance(support, sample=self.variants.sample, details=extra)

    def remaining_provenance(
        self, consistent: int, total: int, *, details: Mapping[str, JSON] | None = None
    ) -> ProvenanceRecord:
        """The Provenance_Record of the branch that holds a split Path's other JSON types (design walk, step 4).

        A Path with two or more Variants and values of other types is an
        ``anyOf`` of the Variant branches plus one branch for the other
        types; the split decision produced that branch too. Support counts
        its ``consistent`` instances among the ``total`` at the Path, and
        the record holds the same decision facts as the Variant branches.
        """
        self._check_split()
        n = self.variants.signatures.n_objects
        if isinstance(consistent, bool) or not isinstance(consistent, (int, np.integer)) or consistent < 1:
            raise ValueError(f"consistent must be a positive integer, got {consistent!r}")
        if self._total(total) < n + consistent:
            raise ValueError(f"total must count the Path's {n} objects and the {consistent} other instances")
        extra: dict[str, JSON] = {**self.details(), "variant_grouping": self._grouping_record()}
        extra.update(details or {})
        return self.decision.provenance(
            Support(int(consistent), int(total)), sample=self.variants.sample, details=extra
        )


def merge_or_split(
    variants: VariantGrouping,
    engine: DecisionEngine,
    *,
    scorer: MDLScorer | None = None,
    features: MergeSplitFeatures | None = None,
    details: Mapping[str, JSON] | None = None,
) -> MergeSplitOutcome:
    """Resolve the Merge-or-Split Judgment_Call of ``variants`` (5.3, 5.11); see the module docstring.

    ``engine`` resolves the Decision_Point and records it in its report.
    The candidates are scored with the grouping's :class:`GroupingScorer`,
    or, when ``variants`` carries none, with one over ``scorer`` (a default
    MDL_Scorer when None). ``features`` computes the Decision_Model features
    (:func:`merge_split_features` by default).
    """
    if not isinstance(variants, VariantGrouping):
        raise TypeError(f"variants must be a VariantGrouping, got {type(variants).__name__}")
    if not isinstance(engine, DecisionEngine):
        raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
    if scorer is not None and not isinstance(scorer, MDLScorer):
        raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
    if variants.n_variants < 2:  # nothing to merge (5.3)
        return MergeSplitOutcome(variants, False)
    signatures = variants.signatures
    grouping_scorer = variants.scorer
    if grouping_scorer is None:
        grouping_scorer = GroupingScorer(signatures, scorer if scorer is not None else MDLScorer())
    everything: Groups = (tuple(range(signatures.n_signatures)),)
    candidates = (
        Candidate(technique=None, outcome=MERGE, order_key=(0,), score=grouping_scorer.score(everything)),
        Candidate(
            technique=None, outcome=SPLIT, order_key=(1,), score=grouping_scorer.score(variants.grouping.groups)
        ),
    )

    def featurize() -> np.ndarray:
        return (features or merge_split_features)(variants, candidates)

    decision = engine.resolve(
        DecisionPoint(MERGE_OR_SPLIT, variants.path), candidates, signatures.objects, featurize, details=details
    )
    return MergeSplitOutcome(variants, decision.label == MERGE, decision)


# ---------------------------------------------------------------------------
# Disjoint branches (5.5)
# ---------------------------------------------------------------------------

_shared_validator: Validator | None = None


def _validator(validator: Validator | None) -> Validator:
    """``validator``, or one Validator per process for callers without one, so compiled checks are shared."""
    global _shared_validator
    if validator is not None:
        return validator
    if _shared_validator is None:
        _shared_validator = Validator()
    return _shared_validator


def _required_names(branch: SchemaNode) -> dict[str, None]:
    """The names ``branch`` lists under ``required``, in order."""
    if branch.boolean is not None:
        return {}
    listed = branch.kw.get("required")
    if not isinstance(listed, (list, tuple)):
        return {}
    return {name: None for name in listed if isinstance(name, str)}


def _value_set(branch: SchemaNode, name: str) -> dict[CanonicalKey, None] | None:
    """The values ``const``/``enum`` allow for property ``name`` of ``branch``; None when unconstrained."""
    if branch.boolean is not None:
        return None
    properties = branch.kw.get("properties")
    if not isinstance(properties, dict):
        return None
    sub = properties.get(name)
    if not isinstance(sub, SchemaNode) or sub.boolean is not None:
        return None
    allowed: dict[CanonicalKey, None] | None = None
    if "const" in sub.kw:
        allowed = {canonical_key(sub.kw["const"]): None}
    listed = sub.kw.get("enum")
    if isinstance(listed, (list, tuple)):
        members = {canonical_key(value): None for value in listed}
        allowed = members if allowed is None else {key: None for key in allowed if key in members}
    return allowed


def _accepts_name(validator: Validator, schema: SchemaNode, name: str) -> bool:
    """Whether the ``propertyNames`` schema accepts ``name``; True when the Validator cannot compile it."""
    if schema.boolean is not None:
        return schema.boolean
    document = schema.to_json()
    assert isinstance(document, dict)
    try:
        return validator.is_valid(validator.compile(document), name)
    except CompileError:
        return True


def permits_field(branch: SchemaNode, name: str, *, validator: Validator | None = None) -> bool:
    """Whether ``branch`` permits an object to hold key ``name`` (5.5); see the module docstring.

    False only when the branch's keywords forbid the key: ``properties[name]``
    is ``false``, a matching ``patternProperties`` subschema is ``false``,
    the key is neither listed nor matched under ``additionalProperties:
    false``, or ``propertyNames`` rejects it. Patterns and ``propertyNames``
    are checked with ``validator`` (one shared by the process when None).
    """
    if not isinstance(branch, SchemaNode):
        raise TypeError(f"branch must be a SchemaNode, got {type(branch).__name__}")
    if not isinstance(name, str):
        raise TypeError(f"name must be a str, got {type(name).__name__}")
    if branch.boolean is not None:
        return branch.boolean
    kw = branch.kw
    properties = kw.get("properties")
    listed = isinstance(properties, dict) and name in properties
    if listed and properties[name].boolean is False:
        return False
    matched = False
    patterns = kw.get("patternProperties")
    if isinstance(patterns, dict):
        for pattern, sub in patterns.items():
            try:
                hit = _validator(validator).matches_pattern(pattern, name)
            except CompileError:  # unreadable: assume it matches and permits
                matched = True
                continue
            if hit:
                matched = True
                if sub.boolean is False:
                    return False
    if not listed and not matched:
        additional = kw.get("additionalProperties")
        if isinstance(additional, SchemaNode) and additional.boolean is False:
            return False
    names = kw.get("propertyNames")
    if isinstance(names, SchemaNode) and not _accepts_name(_validator(validator), names, name):
        return False
    return True


def branches_disjoint(a: SchemaNode, b: SchemaNode, *, validator: Validator | None = None) -> bool:
    """Whether no object can match both branches, by the definition of 5.5 (see the module docstring).

    True when both require a common property and constrain it to
    non-overlapping ``const``/``enum`` value sets, or when one requires a
    field the other does not permit (:func:`permits_field`).
    """
    for branch in (a, b):
        if not isinstance(branch, SchemaNode):
            raise TypeError(f"branches must be SchemaNodes, got {type(branch).__name__}")
    required_a, required_b = _required_names(a), _required_names(b)
    for name in required_a:
        if name not in required_b:
            continue
        values_a, values_b = _value_set(a, name), _value_set(b, name)
        if values_a is not None and values_b is not None and not any(key in values_b for key in values_a):
            return True
    if any(not permits_field(b, name, validator=validator) for name in required_a):
        return True
    return any(not permits_field(a, name, validator=validator) for name in required_b)


def pairwise_disjoint(branches: Sequence[SchemaNode], *, validator: Validator | None = None) -> bool:
    """Whether every pair of ``branches`` is disjoint (5.5); vacuously True for fewer than two."""
    items = list(branches)
    return all(
        branches_disjoint(items[i], items[j], validator=validator)
        for i in range(len(items))
        for j in range(i + 1, len(items))
    )


# ---------------------------------------------------------------------------
# Branches (5.4, 5.5, 5.6)
# ---------------------------------------------------------------------------

# Infers the object schema of one final Variant from its object rows.
VariantSchema = Callable[[View], SchemaNode]


@dataclass(frozen=True, eq=False)
class VariantNode:
    """The schema of an object Path's final Variants and its branch records (see the module docstring).

    With one final Variant, ``node`` is that Variant's object schema and
    ``keyword`` is None. With two or more, ``node`` holds only the
    combinator ``keyword`` (``anyOf`` or ``oneOf``) with one branch per
    Variant, in the order of ``views``, and ``provenance`` maps each branch
    index to its record.
    """

    path: Path
    node: SchemaNode
    outcome: MergeSplitOutcome
    views: tuple[View, ...]
    keyword: Combinator | None = None
    provenance: Mapping[int, ProvenanceRecord] = field(default_factory=dict)
    kind: str = "variants"  # "variants": one branch per final Variant; "tagged_union": Discriminator value groups (10)

    def __post_init__(self) -> None:
        if self.kind not in ("variants", "tagged_union"):
            raise ValueError(f"kind must be 'variants' or 'tagged_union', got {self.kind!r}")
        if self.keyword is None:
            if len(self.views) != 1 or self.provenance:
                raise ValueError("one final Variant has no combinator and no branch records")
        else:
            if self.keyword not in (ANY_OF, ONE_OF):
                raise ValueError(f"the combinator is {ANY_OF!r} or {ONE_OF!r}, got {self.keyword!r}")
            branches = self.node.kw.get(self.keyword)
            if not isinstance(branches, list) or len(branches) != len(self.views) or len(branches) < 2:
                raise ValueError("a combinator holds one branch per final Variant, two or more")
            if sorted(self.provenance) != list(range(len(branches))):
                raise ValueError("every branch needs exactly one Provenance_Record")
        _set(self, "provenance", MappingProxyType(dict(self.provenance)))

    @property
    def branches(self) -> tuple[SchemaNode, ...]:
        """The branch schemas (none for one final Variant)."""
        if self.keyword is None:
            return ()
        return tuple(self.node.kw[self.keyword])

    def keywords(self) -> JSON:
        """The node as plain JSON."""
        return self.node.to_json()

    def constraint_provenance(self) -> dict[ConstraintId, ProvenanceRecord]:
        """The branch Provenance_Records keyed by their ConstraintIds ``(nid, keyword, i)``."""
        if self.keyword is None:
            return {}
        return {ConstraintId(self.node.nid, self.keyword, index): record for index, record in self.provenance.items()}

    def branch_views(self) -> dict[int, list[View]] | None:
        """The rows each branch codes, as the MDL_Scorer's ``branch_views``; None for one final Variant."""
        if self.keyword is None:
            return None
        return {self.node.nid: list(self.views)}


def _variant_schema(variant_schema: VariantSchema, view: View) -> SchemaNode:
    node = variant_schema(view)
    if not isinstance(node, SchemaNode):
        raise TypeError(f"variant_schema must return a SchemaNode, got {type(node).__name__}")
    return node


def variant_branches(
    outcome: MergeSplitOutcome,
    *,
    variant_schema: VariantSchema = naive_merge_node,
    validator: Validator | None = None,
) -> VariantNode:
    """The schema of the final Variants of ``outcome`` (5.4, 5.5, 5.6); see the module docstring.

    ``variant_schema`` builds each Variant's object schema from its object
    rows (Naive_Merge by default; the Structure_Analyzer passes its Record
    builder). ``validator`` checks Patterns and ``propertyNames`` in the
    disjointness test (one shared by the process when None).
    """
    if not isinstance(outcome, MergeSplitOutcome):
        raise TypeError(f"outcome must be a MergeSplitOutcome, got {type(outcome).__name__}")
    views = outcome.views()
    if len(views) == 1:
        return VariantNode(outcome.path, _variant_schema(variant_schema, views[0]), outcome, views)
    branches = [_variant_schema(variant_schema, view) for view in views]
    keyword: Combinator = ONE_OF if pairwise_disjoint(branches, validator=validator) else ANY_OF
    node = SchemaNode(kw={keyword: branches})
    provenance = {
        index: outcome.branch_provenance(index, details={"combinator": keyword}) for index in range(len(branches))
    }
    return VariantNode(outcome.path, node, outcome, views, keyword, provenance)


def _resource_values(reader: ResourceReader) -> dict[str, int | float]:
    values: dict[str, int | float] = {}
    for name in VARIANT_RESOURCES:
        value = reader.value(name)
        assert isinstance(value, (int, float))
        values[name] = value
    return values


class VariantAnalyzer:
    """Variant grouping for one run (see the module docstring).

    ``engine`` resolves the Decision_Points and supplies the Technique states
    and Learned_Thresholds. ``scorer`` scores groupings (the engine's default
    MDL_Scorer when None). ``resources`` gives the Resource_Parameters: a
    ``config.resources.ResourceReader`` (a run passes
    ``ResourceReader.for_run``, which lists each value a grouping Technique
    reads in the report with its origin, 20.5), or a mapping of names to
    values whose reads are not recorded; absent names take their defaults.
    ``seed`` is the run's random seed. ``validator``
    checks Patterns and ``propertyNames`` in the branch disjointness test
    (one shared by the process when None), and ``merge_split_features``
    computes the Merge-or-Split Decision_Model features
    (:func:`merge_split_features` when None).
    """

    __slots__ = ("_engine", "_merge_features", "_reader", "_resources", "_scorer", "_seed", "_validator")

    def __init__(
        self,
        engine: DecisionEngine,
        *,
        scorer: MDLScorer | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = DEFAULT_SEED,
        validator: Validator | None = None,
        merge_split_features: MergeSplitFeatures | None = None,
    ) -> None:
        if not isinstance(engine, DecisionEngine):
            raise TypeError(f"engine must be a DecisionEngine, got {type(engine).__name__}")
        if scorer is not None and not isinstance(scorer, MDLScorer):
            raise TypeError(f"scorer must be an MDLScorer or None, got {type(scorer).__name__}")
        if isinstance(seed, bool) or not isinstance(seed, (int, np.integer)) or seed < 0:
            raise ValueError(f"seed must be a non-negative integer, got {seed!r}")
        if validator is not None and not isinstance(validator, Validator):
            raise TypeError(f"validator must be a Validator or None, got {type(validator).__name__}")
        self._engine = engine
        self._scorer = scorer if scorer is not None else MDLScorer()
        self._reader = ResourceReader.of(resources)
        self._resources = MappingProxyType(_resource_values(self._reader))
        self._seed = int(seed)
        self._validator = validator
        self._merge_features = merge_split_features

    @classmethod
    def for_techniques(
        cls,
        techniques: Mapping[str, bool] | None = None,
        *,
        bundle: ArtifactSource | None = None,
        report: InferenceReport | None = None,
        validator: Validator | None = None,
        resources: ResourceReader | Mapping[str, Any] | None = None,
        seed: int = DEFAULT_SEED,
    ) -> VariantAnalyzer:
        """An analyzer whose engine has the Technique states ``techniques`` (omitted ids take their default)."""
        validator = validator if validator is not None else Validator()
        scorer = MDLScorer(validator=validator)
        engine = DecisionEngine(techniques, bundle=bundle, report=report, scorer=scorer)
        return cls(engine, scorer=scorer, resources=resources, seed=seed, validator=validator)

    @property
    def engine(self) -> DecisionEngine:
        return self._engine

    @property
    def resources(self) -> Mapping[str, int | float]:
        """The Resource_Parameters this analyzer can read, with their effective values (not listed as reads)."""
        return self._resources

    def _read(self, name: str) -> int | float:
        """Read Resource_Parameter ``name`` for a grouping Technique that runs, listing it in the report (20.5)."""
        value = self._reader.read(name)
        assert isinstance(value, (int, float))
        return value

    @property
    def seed(self) -> int:
        return self._seed

    # -- Technique states ---------------------------------------------------

    def runs(self, tid: str) -> bool:
        """Whether grouping Technique ``tid`` is implemented and enabled."""
        return REGISTRY.technique(tid).implemented and self._engine.enabled(tid)

    def _thresholds(self, tid: str) -> dict[str, int | float] | None:
        """The Learned_Thresholds of ``tid``; None when the Model_Bundle lacks one, so the Technique does not run."""
        names = REGISTRY.technique(tid).thresholds
        bundle = self._engine.bundle
        if any(bundle is None or bundle.threshold(name) is None for name in names):
            return None
        values: dict[str, int | float] = {}
        for name in names:
            value = self._engine.threshold(name)  # recorded in the report (20.5)
            assert value is not None
            values[name] = value
        return values

    # -- groupings ----------------------------------------------------------

    def signatures(self, view: View) -> ObjectSignatures:
        """The Object_Signatures of the non-flagged objects of ``view``."""
        return ObjectSignatures.of(view)

    def groupings(self, signatures: ObjectSignatures, scorer: GroupingScorer) -> list[Grouping]:
        """The grouping of every grouping Technique in effect, in priority order (5.7, 5.8)."""
        out: list[Grouping] = []
        read = self._read  # each Technique reads its Resource_Parameters only when it runs (20.5)
        for tid in GROUPING_TECHNIQUES:
            if not self.runs(tid):
                continue
            thresholds = self._thresholds(tid)
            if thresholds is None:
                continue
            if tid == MINHASH_LSH:
                grouping = minhash_lsh_grouping(
                    signatures,
                    scorer,
                    threshold=float(thresholds[_LSH_THRESHOLD]),
                    perm=int(read("minhash.perm")),
                    bands=int(read("lsh.bands")),
                    seed=self._seed,
                )
            elif tid == DBSCAN_TECHNIQUE:
                grouping = dbscan_grouping(
                    signatures,
                    scorer,
                    eps=float(thresholds[_DBSCAN_EPS]),
                    min_samples=int(thresholds[_DBSCAN_MIN_SAMPLES]),
                    sample_size=int(read("sample.dbscan")),
                    seed=self._seed,
                )
            elif tid == HDBSCAN_TECHNIQUE:
                grouping = hdbscan_grouping(
                    signatures,
                    scorer,
                    min_cluster_size=int(thresholds[_HDBSCAN_MIN_CLUSTER_SIZE]),
                    sample_size=int(read("sample.hdbscan")),
                    seed=self._seed,
                )
            else:
                assert tid == BERNOULLI_MIXTURE
                grouping = bernoulli_mixture_grouping(
                    signatures,
                    max_components=int(read("variants.max_components")),
                    max_iter=int(read("variants.em_max_iter")),
                    tol=float(read("variants.em_tol")),
                    seed=self._seed,
                )
            signatures.check_partition(grouping.groups)
            out.append(grouping)
        return out

    def variant_groupings(self, view: View) -> list[Grouping]:
        """The candidate groupings of the objects of ``view`` (5.1, 5.7–5.9); none for a single signature."""
        signatures = self.signatures(view)
        if signatures.n_signatures <= 1:
            return []
        return self.groupings(signatures, GroupingScorer(signatures, self._scorer))

    def select(self, view: View) -> VariantGrouping:
        """Resolve the ``variant_grouping`` Decision_Point of ``view`` (5.2, 5.8, 5.10, 5.11); see the module docstring.

        Raises ``ValueError`` when the view has no non-flagged object.
        """
        signatures = self.signatures(view)
        path = signatures.path
        scorer = GroupingScorer(signatures, self._scorer)
        if signatures.n_signatures == 1:
            return VariantGrouping(path, signatures, Grouping.one(1), None, scorer=scorer)
        offered = self.groupings(signatures, scorer)
        candidates = [
            Candidate(technique=g.technique, outcome=g, label=g.label, score=scorer.score(g.groups)) for g in offered
        ]
        # A priority resolution records the cut-off of the first produced grouping (19.7, 20.1).
        cutoff = offered[0].cutoff if offered else None
        decision = self._engine.resolve(
            DecisionPoint(VARIANT_GROUPING, path), candidates, signatures.objects, cutoff=cutoff, scorer=self._scorer
        )
        if decision.candidate is None:  # the Fallback_Behavior: one Variant (5.10)
            grouping = Grouping.one(signatures.n_signatures)
        else:
            grouping = decision.outcome
            assert isinstance(grouping, Grouping)
        return VariantGrouping(path, signatures, grouping, decision, tuple(offered), scorer=scorer)

    # -- Merge-or-Split and branches ----------------------------------------

    def merge_or_split(self, variants: VariantGrouping, *, details: Mapping[str, JSON] | None = None) -> MergeSplitOutcome:
        """:func:`merge_or_split` with this run's engine, scorer, and features (5.3, 5.11)."""
        return merge_or_split(variants, self._engine, scorer=self._scorer, features=self._merge_features, details=details)

    def branches(self, outcome: MergeSplitOutcome, *, variant_schema: VariantSchema = naive_merge_node) -> VariantNode:
        """:func:`variant_branches` with this run's Validator (5.4, 5.5, 5.6)."""
        return variant_branches(outcome, variant_schema=variant_schema, validator=self._validator)

    def variant_node(self, view: View, *, variant_schema: VariantSchema = naive_merge_node) -> VariantNode:
        """Grouping selection, Merge-or-Split, and branch emission for the objects of ``view`` (design walk, 2.3).

        Raises ``ValueError`` when the view has no non-flagged object.
        """
        return self.branches(self.merge_or_split(self.select(view)), variant_schema=variant_schema)
