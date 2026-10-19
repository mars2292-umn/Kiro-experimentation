"""Unit tests for structure/variants.py: signatures, groupings, Merge-or-Split, and branches (tasks 13.2, 13.3).

Requirements 5.1 (signature partitions), 5.2 (Variant counts by DL or BIC,
none read from the configuration), 5.7 (MinHash/LSH groups), 5.8 (DBSCAN,
HDBSCAN, and Bernoulli-mixture groupings passed to the MDL_Scorer), 5.9
(noise assigned by DL), 5.10 (fallback one Variant), 5.11 (priority order
without MDL, and split without the Merge-or-Split Decision_Model and MDL),
5.3 (Merge-or-Split by Decision_Model or MDL), 5.4 (one ``anyOf`` branch per
Variant with only its observed fields), 5.5 (``oneOf`` for pairwise
disjoint branches), 5.6 (the merged Variant's schema), and 29.8 (sampling
of the super-linear Techniques).
"""

from __future__ import annotations

import math
import random
from typing import Any

import numpy as np
import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration
from jsinfer.config.thresholds import THRESHOLDS
from jsinfer.core.determinism import canonical_bytes
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import FallbackInfo, SampleInfo, laplace_confidence
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import ConstraintId, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate, DecisionEngine, mdl_posterior
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer, Score
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.variants import (
    ANY_OF,
    BERNOULLI_MIXTURE,
    DBSCAN_TECHNIQUE,
    DM_MERGE_SPLIT,
    GROUPING_TECHNIQUES,
    HDBSCAN_TECHNIQUE,
    MERGE,
    MERGE_OR_SPLIT,
    MINHASH_LSH,
    ONE_OF,
    ONE_VARIANT,
    SPLIT,
    VARIANT_GROUPING,
    Grouping,
    GroupingScorer,
    MergeSplitOutcome,
    ObjectSignatures,
    VariantAnalyzer,
    VariantGrouping,
    VariantNode,
    assign_by_description_length,
    bernoulli_mixture_grouping,
    branches_disjoint,
    canonical_groups,
    dbscan_grouping,
    groups_from_labels,
    hdbscan_grouping,
    merge_or_split,
    merge_split_features,
    minhash_lsh_grouping,
    pairwise_disjoint,
    permits_field,
    signature_distances,
    variant_branches,
)

VALIDATOR = Validator()
SCORER = MDLScorer(validator=VALIDATOR)
MDL = "decide.mdl"

USERS = [{"kind": "user", "name": f"u{i}", "age": 20 + i % 30} for i in range(24)]
USERS_WITH_EMAIL = [
    {"kind": "user", "name": f"v{i}", "age": 30 + i % 9, "email": f"v{i}@example.org"} for i in range(8)
]
ITEMS = [{"sku": f"s{i}", "price": i + 0.25, "tags": ["a", "b"][: i % 3]} for i in range(20)]
ODD = [{"kind": 7}]  # the same key as the users, with another type
FAMILIES = [*USERS, *USERS_WITH_EMAIL, *ITEMS, *ODD]

ALL_THRESHOLDS = {
    "lsh.keyset_jaccard": 0.5,
    "dbscan.eps": 0.5,
    "dbscan.min_samples": 3,
    "hdbscan.min_cluster_size": 2,
}


class StubBundle:
    """Learned_Thresholds by name, like ResolvedBundle.threshold; no Decision_Models."""

    def __init__(self, thresholds: dict[str, int | float]) -> None:
        self.thresholds = thresholds

    def model(self, name: str) -> Any | None:
        return None

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def signatures_of(docs: list[Any]) -> ObjectSignatures:
    return ObjectSignatures.of(root(docs))


def analyzer(
    states: dict[str, bool] | None = None,
    thresholds: dict[str, int | float] | None = None,
    **kwargs: Any,
) -> tuple[VariantAnalyzer, InferenceReport]:
    report = InferenceReport()
    bundle = None if thresholds is None else StubBundle(thresholds)
    return VariantAnalyzer.for_techniques(states, bundle=bundle, report=report, validator=VALIDATOR, **kwargs), report


def only(*technique: str) -> dict[str, bool]:
    """Technique states with exactly the given grouping Techniques enabled."""
    return {tid: tid in technique for tid in GROUPING_TECHNIQUES}


def entries(sigs: ObjectSignatures) -> list[tuple[tuple[str, str], ...]]:
    return [sigs.entries(s) for s in range(sigs.n_signatures)]


def object_count(view: View) -> int:
    return view.stats().n_objects


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_grouping_techniques_are_built_with_their_thresholds_and_objectives() -> None:
    assert GROUPING_TECHNIQUES == (MINHASH_LSH, DBSCAN_TECHNIQUE, HDBSCAN_TECHNIQUE, BERNOULLI_MIXTURE)
    expected = {
        MINHASH_LSH: ("lsh.keyset_jaccard",),
        DBSCAN_TECHNIQUE: ("dbscan.eps", "dbscan.min_samples"),
        HDBSCAN_TECHNIQUE: ("hdbscan.min_cluster_size",),
        BERNOULLI_MIXTURE: (),
    }
    for tid, thresholds in expected.items():
        spec = REGISTRY.technique(tid)
        assert spec.implemented and spec.default_enabled and spec.serves == (VARIANT_GROUPING,), tid
        assert spec.thresholds == thresholds
        for name in thresholds:  # registered with the Trainer's validation objective (20.3)
            assert THRESHOLDS[name].decision_point == VARIANT_GROUPING
            assert THRESHOLDS[name].objective == "grouping_ari"
            assert THRESHOLDS[name].techniques == (tid,)
    # 29.8: the clustering Techniques are super-linear and sample.
    assert REGISTRY.technique(DBSCAN_TECHNIQUE).sample_param == "sample.dbscan"
    assert REGISTRY.technique(HDBSCAN_TECHNIQUE).sample_param == "sample.hdbscan"
    assert not REGISTRY.technique(MINHASH_LSH).superlinear and not REGISTRY.technique(BERNOULLI_MIXTURE).superlinear
    assert REGISTRY.fallback(VARIANT_GROUPING)() == ONE_VARIANT


# ---------------------------------------------------------------------------
# Object_Signatures (5.1)
# ---------------------------------------------------------------------------


def test_signatures_are_weighted_and_canonically_sorted() -> None:
    docs: list[Any] = [{"b": 1, "a": "x"}, {"a": "y", "b": 2.5}, {"a": None}, {"a": "z"}, {"a": "w"}, {}, 3, [1]]
    sigs = signatures_of(docs)
    # integer and number are both "number"; scalars and arrays at the Path are not objects.
    assert entries(sigs) == [(), (("a", "null"),), (("a", "string"),), (("a", "string"), ("b", "number"))]
    assert sigs.counts.tolist() == [1, 1, 2, 2]
    assert sigs.n_objects == 6 and sigs.n_signatures == 4
    assert sigs.describe(3) == {"entries": [["a", "string"], ["b", "number"]], "objects": 2}
    assert sigs.key_presence().tolist() == [[False, False], [True, False], [True, False], [True, True]]
    assert sigs.features().shape == (4, 3)


def test_signature_views_partition_the_objects() -> None:
    sigs = signatures_of(FAMILIES)
    counts = [object_count(sigs.view((s,))) for s in range(sigs.n_signatures)]
    assert counts == sigs.counts.tolist() and sum(counts) == len(FAMILIES)
    assert sigs.view(tuple(range(sigs.n_signatures))) is sigs.objects
    with pytest.raises(ValueError, match="partition"):
        sigs.check_partition(((0,),))


def test_signatures_do_not_depend_on_document_order() -> None:
    reference = signatures_of(FAMILIES)
    shuffled = list(FAMILIES)
    random.Random(3).shuffle(shuffled)
    for order in (FAMILIES[::-1], shuffled):
        sigs = signatures_of(order)
        assert sigs.codes == reference.codes and sigs.counts.tolist() == reference.counts.tolist()


def test_flagged_documents_are_excluded() -> None:
    store = build_store([*USERS[:3], {"zzz": 1}])
    store.flags.flag_documents([3])
    sigs = ObjectSignatures.of(View.root(store))
    assert sigs.n_signatures == 1 and sigs.n_objects == 3


def test_a_view_without_objects_is_an_error() -> None:
    with pytest.raises(ValueError, match="no non-flagged object"):
        signatures_of([1, "a"])


def test_signature_distances_are_exact_jaccard_over_key_type_entries() -> None:
    sigs = signatures_of([{}, {"a": 1}, {"a": 1, "b": 2}, {"a": "s", "b": 2}])
    d = signature_distances(sigs)
    assert d.shape == (4, 4) and np.all(np.diag(d) == 0.0)
    assert d[0, 1] == 1.0  # the empty signature shares nothing
    assert d[1, 2] == 1 - 1 / 2  # {a:number} vs {a:number, b:number}
    assert d[2, 3] == 1 - 1 / 3  # a differs in type: one shared entry of three
    assert d[1, 3] == 1.0
    assert np.array_equal(d, d.T)
    assert np.array_equal(signature_distances(sigs, np.array([1, 3])), d[np.ix_([1, 3], [1, 3])])


# ---------------------------------------------------------------------------
# Groupings and their description lengths (5.2)
# ---------------------------------------------------------------------------


def test_groupings_are_canonical_partitions() -> None:
    assert canonical_groups([[3, 1], [2], [0]]) == ((0,), (1, 3), (2,))
    assert groups_from_labels(np.array([1, -1, 1, 0])) == ((0, 2), (3,))
    assert groups_from_labels(np.array([0, 0]), np.array([4, 2])) == ((2, 4),)
    with pytest.raises(ValueError, match="two candidate Variants"):
        canonical_groups([[0, 1], [1]])
    with pytest.raises(ValueError, match="at least one signature"):
        canonical_groups([[0], []])
    grouping = Grouping(DBSCAN_TECHNIQUE, ((2,), (0, 1)))
    assert grouping.groups == ((0, 1), (2,)) and grouping.label == "2-variants"
    assert Grouping.one(3).label == ONE_VARIANT and Grouping.one(3).groups == ((0, 1, 2),)
    with pytest.raises(ValueError, match="not a Variant grouping Technique"):
        Grouping("decide.mdl", ((0,),))


def test_grouping_bits_decompose_the_scorers_encoding() -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    n = sigs.n_signatures
    for groups in [((tuple(range(n))),), tuple((s,) for s in range(n)), ((0, 1), tuple(range(2, n)))]:
        groups = canonical_groups(groups)
        exact = scorer.score(groups)
        assert math.isclose(scorer.bits(list(groups)), exact.total, rel_tol=1e-12, abs_tol=0.0)
        assert exact.n_scored == SCORER.score(naive_merge_node(sigs.objects), sigs.objects).n_scored
        assert exact.n_rejected == 0
    one, hints = scorer.encoding(((tuple(range(n))),))
    assert hints is None and one.to_json() == naive_merge_node(sigs.objects).to_json()
    split, hints = scorer.encoding(((0,), tuple(range(1, n))))
    assert hints is not None and list(hints) == [split.nid] and len(split.kw["anyOf"]) == 2


def test_merge_terms_rank_merges_like_their_exact_description_lengths() -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    groups = [(0,), (1,), (2,), (3,)]
    current = scorer.bits(groups)
    shared = None
    for a in range(4):
        for b in range(a + 1, 4):
            merged = canonical_groups([g for g in groups if g[0] not in (a, b)] + [(a, b)])
            change = scorer.bits(list(merged)) - current
            offset = change - scorer.merge_term((a,), (b,))
            if shared is None:
                shared = offset
            assert math.isclose(offset, shared, rel_tol=0.0, abs_tol=1e-9)  # one term shared by every merge


def test_noise_joins_the_existing_or_new_group_with_the_lowest_description_length() -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    n = sigs.n_signatures
    clusters = canonical_groups([(0,), tuple(range(1, n - 1))])
    pending = n - 1
    options = [
        canonical_groups([(0, pending), tuple(range(1, n - 1))]),
        canonical_groups([(0,), tuple(range(1, n))]),
        canonical_groups([(0,), tuple(range(1, n - 1)), (pending,)]),
    ]
    best = min(options, key=lambda groups: scorer.bits(list(groups)))
    assert assign_by_description_length(clusters, [pending], scorer) == best
    # With no group yet, the first pending signature opens one.
    assert assign_by_description_length((), [0], scorer) == ((0,),)


# ---------------------------------------------------------------------------
# variants.minhash_lsh (5.7)
# ---------------------------------------------------------------------------


def groups_holding(sigs: ObjectSignatures, groups: tuple[tuple[int, ...], ...], key: str) -> list[int]:
    """The indices of the groups with a signature that holds ``key``."""
    return [g for g, group in enumerate(groups) if any(key in dict(sigs.entries(s)) for s in group)]


def test_lsh_separates_dissimilar_key_sets() -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    grouping = minhash_lsh_grouping(sigs, scorer, threshold=0.5, perm=128, bands=32)
    sigs.check_partition(grouping.groups)
    users, items = groups_holding(sigs, grouping.groups, "age"), groups_holding(sigs, grouping.groups, "sku")
    assert len(users) == 1 and len(items) == 1 and users != items
    assert grouping.cutoff.source == "lsh.keyset_jaccard" and grouping.cutoff.value == 0.5
    details = grouping.details
    assert (details["permutations"], details["bands"], details["rows_per_band"]) == (128, 32, 4)


def test_identical_key_sets_always_share_a_group() -> None:
    # {kind: string} and {kind: number} are two signatures with one key set: estimated Jaccard 1 >= τ.
    docs: list[Any] = [{"kind": "a"}] * 3 + [{"kind": 1}] * 2 + [{"other": True}] * 4
    sigs = signatures_of(docs)
    grouping = minhash_lsh_grouping(sigs, GroupingScorer(sigs, SCORER), threshold=1.0, perm=64, bands=16)
    kind = [s for s in range(sigs.n_signatures) if sigs.entries(s)[0][0] == "kind"]
    assert len(kind) == 2 and any(set(kind) <= set(group) for group in grouping.groups)


@pytest.mark.parametrize("perm", [1, 8])
def test_lsh_with_one_band_retrieves_the_pairs_with_equal_sketches(perm: int) -> None:
    # lsh.bands = 1 is admissible, but datasketch's MinHashLSH needs two or more bands.
    docs: list[Any] = [{"kind": "a"}] * 3 + [{"kind": 1}] * 2 + [{"other": True}] * 4
    sigs = signatures_of(docs)
    grouping = minhash_lsh_grouping(sigs, GroupingScorer(sigs, SCORER), threshold=1.0, perm=perm, bands=1)
    sigs.check_partition(grouping.groups)
    assert grouping.details["candidate_pairs"] == 1  # the two signatures with key set {kind}
    assert grouping.details["rows_per_band"] == perm
    kind = [s for s in range(sigs.n_signatures) if sigs.entries(s)[0][0] == "kind"]
    assert any(set(kind) <= set(group) for group in grouping.groups)


def test_lsh_greedy_merging_only_lowers_the_description_length() -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    # τ = 1: the components are the distinct key sets; merging then runs on the LSH candidate pairs.
    grouping = minhash_lsh_grouping(sigs, scorer, threshold=1.0, perm=128, bands=32)
    components = grouping.details["components"]
    assert grouping.n_variants == components - grouping.details["merges"]
    by_key_set: dict[tuple[int, ...], list[int]] = {}
    for s in range(sigs.n_signatures):
        by_key_set.setdefault(sigs.key_ids(s), []).append(s)
    assert components == len(by_key_set)
    initial = canonical_groups(list(by_key_set.values()))
    assert scorer.bits(list(grouping.groups)) <= scorer.bits(list(initial))


def test_lsh_does_not_depend_on_document_order() -> None:
    results = set()
    for order in (FAMILIES, FAMILIES[::-1]):
        sigs = signatures_of(order)
        grouping = minhash_lsh_grouping(sigs, GroupingScorer(sigs, SCORER), threshold=0.6, perm=128, bands=32)
        results.add(canonical_bytes([list(map(list, grouping.groups)), dict(grouping.details)]))
    assert len(results) == 1


# ---------------------------------------------------------------------------
# variants.dbscan and variants.hdbscan (5.8, 5.9, 29.8)
# ---------------------------------------------------------------------------


def test_dbscan_min_samples_counts_objects() -> None:
    # Far-apart signatures with counts 5, 1, and 1: with min_samples = 3 only the first is a core point.
    docs: list[Any] = [{"a": 1}] * 5 + [{"b": 1}] + [{"c": 1}]
    sigs = signatures_of(docs)
    grouping = dbscan_grouping(
        sigs, GroupingScorer(sigs, SCORER), eps=0.1, min_samples=3, sample_size=5_000
    )
    sigs.check_partition(grouping.groups)
    assert grouping.details["clusters"] == 1 and grouping.details["noise_signatures"] == 2
    assert grouping.details["noise_objects"] == 2 and grouping.sample is None
    assert grouping.cutoff.source == "dbscan.eps" and grouping.details["min_samples"] == 3


def test_noise_never_becomes_flagged() -> None:
    store = build_store(FAMILIES)
    sigs = ObjectSignatures.of(View.root(store))
    grouping = dbscan_grouping(sigs, GroupingScorer(sigs, SCORER), eps=0.01, min_samples=1_000, sample_size=5_000)
    assert grouping.details["clusters"] == 0 and grouping.details["noise_signatures"] == sigs.n_signatures
    sigs.check_partition(grouping.groups)
    assert store.flags.is_empty


def test_hdbscan_with_a_minimum_cluster_size_above_the_signatures_is_all_noise() -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    grouping = hdbscan_grouping(sigs, scorer, min_cluster_size=sigs.n_signatures + 1, sample_size=5_000)
    assert grouping.details["clusters"] == 0 and grouping.details["noise_signatures"] == sigs.n_signatures
    assert grouping.groups == assign_by_description_length((), range(sigs.n_signatures), scorer)
    assert grouping.cutoff.source == "hdbscan.min_cluster_size"
    clustered = hdbscan_grouping(sigs, scorer, min_cluster_size=2, sample_size=5_000)
    sigs.check_partition(clustered.groups)


@pytest.mark.parametrize("technique", [DBSCAN_TECHNIQUE, HDBSCAN_TECHNIQUE])
def test_clustering_samples_signatures_beyond_the_sample_size(technique: str) -> None:
    sigs = signatures_of(FAMILIES)
    scorer = GroupingScorer(sigs, SCORER)
    n = sigs.n_signatures
    assert n > 3

    def run() -> Grouping:
        if technique == DBSCAN_TECHNIQUE:
            return dbscan_grouping(sigs, scorer, eps=0.5, min_samples=1, sample_size=3, seed=11)
        return hdbscan_grouping(sigs, scorer, min_cluster_size=2, sample_size=3, seed=11)

    grouping = run()
    assert grouping.sample == SampleInfo(3, n)
    assert grouping.details["unsampled_signatures"] == n - 3
    sigs.check_partition(grouping.groups)  # the unsampled signatures are assigned too
    assert run().groups == grouping.groups  # the sample is seeded


def test_unsampled_signatures_follow_their_nearest_sampled_signature() -> None:
    # Two far-apart families of three signatures each; DBSCAN clusters each family it samples.
    docs: list[Any] = []
    for extra in ("x", "y", "z"):
        docs += [{"a": 1, "b": 1, "c": 1, extra: 1}] * 4
        docs += [{"p": "s", "q": "s", "r": "s", f"{extra}{extra}": "s"}] * 4
    sigs = signatures_of(docs)
    assert sigs.n_signatures == 6
    grouping = dbscan_grouping(sigs, GroupingScorer(sigs, SCORER), eps=0.5, min_samples=1, sample_size=4, seed=2)
    assert grouping.sample == SampleInfo(4, 6)
    for group in grouping.groups:  # no group mixes the families
        families = {"a" in dict(sigs.entries(s)) for s in group}
        assert len(families) == 1
    sigs.check_partition(grouping.groups)


# ---------------------------------------------------------------------------
# variants.bernoulli_mixture (5.2)
# ---------------------------------------------------------------------------


def test_bernoulli_mixture_selects_the_component_count_by_bic() -> None:
    sigs = signatures_of(FAMILIES)
    grouping = bernoulli_mixture_grouping(sigs, max_components=10, max_iter=100, tol=1e-6)
    sigs.check_partition(grouping.groups)
    table = grouping.details["bic"]
    assert [row["components"] for row in table] == list(range(1, min(sigs.n_signatures, 10) + 1))
    best = min(table, key=lambda row: (row["bic"], row["components"]))
    assert grouping.details["components"] == best["components"] and grouping.details["selected_bic"] == best["bic"]
    assert grouping.cutoff.source == "BIC"
    assert [score.score for score in grouping.cutoff.scores] == [row["bic"] for row in table]
    # The user and item families never share a component.
    for group in grouping.groups:
        keys = {name for s in group for name, _ in sigs.entries(s)}
        assert not ({"age", "sku"} <= keys)
    assert 1 <= grouping.n_variants <= sigs.n_signatures


def test_bernoulli_mixture_component_count_is_capped_by_the_resource_parameter() -> None:
    sigs = signatures_of(FAMILIES)
    grouping = bernoulli_mixture_grouping(sigs, max_components=1, max_iter=100, tol=1e-6)
    assert grouping.groups == (tuple(range(sigs.n_signatures)),)
    assert len(grouping.details["bic"]) == 1 and grouping.details["max_components"] == 1


def test_bernoulli_mixture_is_seeded_and_order_independent() -> None:
    a = bernoulli_mixture_grouping(signatures_of(FAMILIES), max_components=4, max_iter=50, tol=1e-6, seed=5)
    b = bernoulli_mixture_grouping(signatures_of(FAMILIES[::-1]), max_components=4, max_iter=50, tol=1e-6, seed=5)
    assert a.groups == b.groups and canonical_bytes(dict(a.details)) == canonical_bytes(dict(b.details))


# ---------------------------------------------------------------------------
# Selection (5.2, 5.8, 5.10, 5.11)
# ---------------------------------------------------------------------------


def decisions(report: InferenceReport) -> list[Any]:
    return [entry for entry in report.decisions if entry.kind == VARIANT_GROUPING]


def test_the_default_bundle_leaves_only_the_bernoulli_mixture_in_effect() -> None:
    variants, report = analyzer()
    outcome = variants.select(root(FAMILIES))
    decision = outcome.decision
    assert decision.method == "mdl" and decision.technique == BERNOULLI_MIXTURE
    assert [g.technique for g in outcome.candidates] == [BERNOULLI_MIXTURE]
    assert outcome.grouping is outcome.candidates[0]
    (entry,) = decisions(report)
    assert entry.resolution.candidates[0].technique == BERNOULLI_MIXTURE
    assert entry.cutoff.source == "MDL comparison"


def test_mdl_selects_the_grouping_with_the_lowest_description_length() -> None:
    variants, report = analyzer(thresholds=ALL_THRESHOLDS)
    outcome = variants.select(root(FAMILIES))
    assert [g.technique for g in outcome.candidates] == list(GROUPING_TECHNIQUES)
    resolution = outcome.decision.resolution
    scores = [c.score for c in resolution.candidates]
    assert resolution.method == "mdl" and scores[resolution.selected] == min(scores)
    assert outcome.decision.confidence == mdl_posterior(scores)[resolution.selected]
    # 5.8: every grouping was scored by the MDL_Scorer on the same objects.
    scorer = outcome.scorer
    assert scores == [scorer.score(g.groups).total for g in outcome.candidates]
    assert len({c.n_scored for c in resolution.candidates}) == 1
    assert outcome.grouping.technique == resolution.candidates[resolution.selected].technique
    views = outcome.views()
    assert sum(object_count(v) for v in views) == len(FAMILIES) and len(views) == outcome.n_variants
    details = outcome.details()
    assert details["object_signatures"] == outcome.signatures.n_signatures
    assert sum(details["variant_objects"]) == len(FAMILIES)
    assert details["grouping"]["technique"] == outcome.grouping.technique


def test_without_mdl_the_priority_order_selects() -> None:
    variants, report = analyzer({MDL: False}, ALL_THRESHOLDS)
    outcome = variants.select(root(FAMILIES))
    decision = outcome.decision
    assert decision.method == "priority" and outcome.grouping.technique == MINHASH_LSH
    assert decision.fallback == FallbackInfo(MDL, "disabled")  # 19.7
    assert decision.cutoff.source == "lsh.keyset_jaccard" and decision.cutoff.value == 0.5
    assert [c.technique for c in decision.resolution.candidates] == list(GROUPING_TECHNIQUES)


def test_priority_skips_techniques_missing_a_learned_threshold() -> None:
    thresholds = {"dbscan.eps": 0.5, "dbscan.min_samples": 3}
    variants, _ = analyzer({MDL: False}, thresholds)
    outcome = variants.select(root(FAMILIES))
    assert [g.technique for g in outcome.candidates] == [DBSCAN_TECHNIQUE, BERNOULLI_MIXTURE]
    assert outcome.grouping.technique == DBSCAN_TECHNIQUE and outcome.decision.cutoff.source == "dbscan.eps"
    # With only the Bernoulli mixture left, its BIC comparison is the cut-off.
    variants, _ = analyzer({MDL: False})
    outcome = variants.select(root(FAMILIES))
    assert outcome.grouping.technique == BERNOULLI_MIXTURE and outcome.decision.cutoff.source == "BIC"
    assert outcome.decision.fallback is None


def test_all_grouping_techniques_disabled_is_one_variant() -> None:
    variants, report = analyzer(only(), ALL_THRESHOLDS)
    outcome = variants.select(root(FAMILIES))
    assert outcome.decision.method == "fallback" and outcome.decision.label == ONE_VARIANT
    assert outcome.decision.fallback == FallbackInfo(MINHASH_LSH, "disabled")
    assert outcome.decision.technique == "fallback:variant_grouping"
    assert outcome.grouping.groups == (tuple(range(outcome.signatures.n_signatures)),)
    assert outcome.views() == (outcome.signatures.objects,) and outcome.candidates == ()
    assert [entry.resolution.method for entry in decisions(report)] == ["fallback"]


def test_a_missing_threshold_is_named_by_the_fallback() -> None:
    variants, _ = analyzer(only(MINHASH_LSH))
    outcome = variants.select(root(FAMILIES))
    assert outcome.decision.method == "fallback"
    assert outcome.decision.fallback == FallbackInfo(MINHASH_LSH, "missing-threshold")
    assert outcome.n_variants == 1


def test_a_single_signature_needs_no_decision() -> None:
    variants, report = analyzer(thresholds=ALL_THRESHOLDS)
    outcome = variants.select(root(USERS))
    assert outcome.decision is None and outcome.n_variants == 1 and decisions(report) == []
    assert outcome.details() == {
        "candidate_variants": 1,
        "object_signatures": 1,
        "variant_objects": [len(USERS)],
        "variant_signatures": [1],
    }
    assert variants.variant_groupings(root(USERS)) == []


def test_variant_counts_stay_within_the_signatures() -> None:
    variants, _ = analyzer(thresholds=ALL_THRESHOLDS)
    view = root(FAMILIES)
    n = ObjectSignatures.of(view).n_signatures
    for grouping in variants.variant_groupings(view):
        assert 1 <= grouping.n_variants <= n


def test_selection_does_not_depend_on_document_order() -> None:
    seen = set()
    shuffled = list(FAMILIES)
    random.Random(8).shuffle(shuffled)
    for order in (FAMILIES, FAMILIES[::-1], shuffled):
        variants, report = analyzer(thresholds=ALL_THRESHOLDS)
        outcome = variants.select(root(order))
        seen.add(canonical_bytes([[entry.to_json() for entry in report.decisions], outcome.details()]))
    assert len(seen) == 1


def test_run_configuration_resources_and_seed_are_accepted() -> None:
    config = RunConfiguration.default()
    engine = DecisionEngine(config.techniques, scorer=SCORER)
    variants = VariantAnalyzer(engine, scorer=SCORER, resources=config.resources, seed=config.seed)
    assert variants.resources["minhash.perm"] == 128 and variants.resources["variants.max_components"] == 10
    assert variants.seed == 0


@pytest.mark.parametrize(
    "resources",
    [{"lsh.bands": 256}, {"sample.dbscan": 0}, {"variants.em_tol": -1.0}, {"not.a.setting": 1}],
)
def test_invalid_resources_are_rejected(resources: dict[str, Any]) -> None:
    with pytest.raises(ValueError, match="invalid Resource_Parameters"):
        VariantAnalyzer(DecisionEngine(), resources=resources)


def test_resource_reads_are_listed_with_their_origin_when_the_technique_runs() -> None:
    """20.5 (task 19.1): each grouping Technique that runs lists the Resource_Parameters it reads."""
    from jsinfer.config.resources import ResourceReader

    config = RunConfiguration.resolve({"minhash.perm": 64, "sample.hdbscan": 9})
    report = InferenceReport()
    variants = VariantAnalyzer.for_techniques(
        {**only(MINHASH_LSH, DBSCAN_TECHNIQUE), MDL: True},
        bundle=StubBundle(ALL_THRESHOLDS),
        report=report,
        validator=VALIDATOR,
        resources=ResourceReader.for_run(config, report),
    )
    assert variants.resources["sample.hdbscan"] == 9 and report.resources == {}  # built, nothing read yet
    variants.select(root(USERS))  # one signature: no grouping runs
    assert report.resources == {}
    variants.select(root(FAMILIES))
    listed = {name: (use.value, use.origin) for name, use in report.resources.items()}
    # HDBSCAN and the Bernoulli mixture are disabled, so sample.hdbscan and variants.* are not read.
    assert listed == {
        "lsh.bands": (32, "default"),
        "minhash.perm": (64, "flag"),
        "sample.dbscan": (5_000, "default"),
    }
    plain, plain_report = analyzer(only(BERNOULLI_MIXTURE), resources=config.resources)
    plain.select(root(FAMILIES))
    assert plain_report.resources == {}  # a plain mapping is read without recording


# ===========================================================================
# Merge-or-Split and branch emission (task 13.3)
# ===========================================================================

CATS = [{"kind": "cat", "lives": 9 - i % 3} for i in range(15)]
DOGS = [{"kind": "dog", "bark": "loud" if i % 2 else "soft"} for i in range(12)]
PETS = [*CATS, *DOGS]
# Two families whose shared field "id" changes type; the Bernoulli mixture splits them.
NUMBERED = [{"id": i, "a": f"x{i}"} for i in range(10)]
NAMED = [{"id": f"n{i}", "b": i % 2 == 0} for i in range(10)]


class ModelBundle:
    """Decision_Models and Learned_Thresholds by name, like ResolvedBundle."""

    def __init__(self, models: dict[str, Any], thresholds: dict[str, int | float] | None = None) -> None:
        self.models = models
        self.thresholds = thresholds or {}

    def model(self, name: str) -> Any | None:
        return self.models.get(name)

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


class StubModel:
    """A Decision_Model with fixed calibrated probabilities; records the feature arrays it sees."""

    def __init__(self, classes: tuple[str, ...], proba: list[float]) -> None:
        self.classes = classes
        self.proba = np.array(proba, dtype=np.float64)
        self.seen: list[np.ndarray] = []

    def predict_proba(self, x: np.ndarray) -> np.ndarray:
        self.seen.append(np.array(x))
        return self.proba.reshape(1, -1)


def fixed(model_bits: float, data_bits: float) -> Score:
    return Score(float(model_bits), float(data_bits), 0.0, 0, 1)


class ShapeScorer(MDLScorer):
    """Scores a grouping encoding by its shape: an ``anyOf`` is the split, anything else the merge."""

    __slots__ = ("merge", "split")

    def __init__(self, *, merge: Score, split: Score) -> None:
        super().__init__(validator=VALIDATOR)
        self.merge = merge
        self.split = split

    def score(self, node: SchemaNode, view: View, **kwargs: Any) -> Score:
        return self.split if "anyOf" in node.kw else self.merge


def shaped(states: dict[str, bool] | None, scorer: MDLScorer) -> tuple[VariantAnalyzer, InferenceReport]:
    """An analyzer whose groupings and Merge-or-Split candidates are scored by ``scorer``."""
    report = InferenceReport()
    engine = DecisionEngine(states, report=report, scorer=scorer)
    return VariantAnalyzer(engine, scorer=scorer, validator=VALIDATOR), report


def merge_decisions(report: InferenceReport) -> list[Any]:
    return [entry for entry in report.decisions if entry.kind == MERGE_OR_SPLIT]


def validates(schema: Any, docs: list[Any]) -> bool:
    compiled = VALIDATOR.compile(schema)
    return all(VALIDATOR.is_valid(compiled, doc) for doc in docs)


def matching_branches(node: VariantNode, doc: Any) -> list[int]:
    return [i for i, branch in enumerate(node.branches) if validates(branch.to_json(), [doc])]


def kind_enum(view: View) -> SchemaNode:
    """A Variant schema whose ``kind`` property lists the Variant's observed kinds under ``enum``."""
    node = naive_merge_node(view)
    kinds = view.child_key("kind")
    values = [kinds.store.values[int(vid)] for vid in kinds.stats().value_ids.tolist()]
    node.kw["properties"]["kind"] = SchemaNode(kw={"enum": values})
    return node


def closed(view: View) -> SchemaNode:
    """A Variant schema of a closed Record: Naive_Merge plus ``additionalProperties: false``."""
    node = naive_merge_node(view)
    node.kw["additionalProperties"] = SchemaNode(boolean=False)
    return node


def schema(value: Any) -> SchemaNode:
    return SchemaNode.from_json(value)


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_merge_or_split_is_served_by_its_decision_model_then_mdl() -> None:
    spec = REGISTRY.decision_point(MERGE_OR_SPLIT)
    assert spec.served_by == (DM_MERGE_SPLIT, MDL)
    assert REGISTRY.fallback(MERGE_OR_SPLIT)() == SPLIT and spec.tie_rule is None
    assert JUDGMENT_OUTCOMES[MERGE_OR_SPLIT] == (MERGE, SPLIT)
    # Both serve by default; without a model in the bundle MDL selection resolves the point (22.14).
    assert REGISTRY.active_servers(MERGE_OR_SPLIT, REGISTRY.default_states()) == (DM_MERGE_SPLIT, MDL)


# ---------------------------------------------------------------------------
# Merge-or-Split (5.3, 5.11)
# ---------------------------------------------------------------------------


def test_mdl_splits_unrelated_families_and_records_both_candidates() -> None:
    variants, report = analyzer()
    grouping = variants.select(root([*USERS, *ITEMS]))
    assert grouping.n_variants == 2
    outcome = variants.merge_or_split(grouping)
    assert outcome.judgment == SPLIT and not outcome.merged and outcome.n_variants == 2
    decision = outcome.decision
    assert decision.method == "mdl" and decision.technique == MDL and decision.cutoff.source == "MDL comparison"
    assert decision.fallback == FallbackInfo(DM_MERGE_SPLIT, "missing-model")  # 22.14: no model in the bundle
    merge, split = decision.resolution.candidates
    assert (merge.outcome, split.outcome) == (MERGE, SPLIT) and merge.technique is None
    # The design's comparison: one Naive_Merge object schema vs the anyOf of the candidate Variants.
    scorer = grouping.scorer
    everything = (tuple(range(grouping.signatures.n_signatures)),)
    assert merge.score == scorer.score(everything).total
    assert merge.score == SCORER.score(naive_merge_node(grouping.signatures.objects), grouping.signatures.objects).total
    assert split.score == scorer.score(grouping.grouping.groups).total
    assert split.score < merge.score and merge.n_scored == split.n_scored
    assert decision.confidence == mdl_posterior([merge.score, split.score])[1]
    (entry,) = merge_decisions(report)
    assert entry.resolution.method == "mdl" and entry.path == "$"


def test_branch_records_beside_other_types_count_every_instance() -> None:
    # A split Path with 5 more instances of other types (the analyzer's mixed-type rule, task 16.1).
    variants, _ = analyzer()
    outcome = variants.merge_or_split(variants.select(root([*USERS, *ITEMS])))
    plain, wide = outcome.branch_provenance(0), outcome.branch_provenance(0, total=49)
    assert (plain.support.consistent, plain.support.total) == (24, 44)
    assert (wide.support.consistent, wide.support.total) == (24, 49)
    assert wide.resolution == plain.resolution and wide.details == plain.details
    rest = outcome.remaining_provenance(5, 49, details={"branch": "remaining_types"})
    assert (rest.support.consistent, rest.support.total) == (5, 49) and rest.technique == plain.technique
    assert "variant" not in rest.details and rest.details["variant_grouping"] == plain.details["variant_grouping"]
    assert rest.details["branch"] == "remaining_types" and rest.resolution == plain.resolution
    with pytest.raises(ValueError, match="total"):
        outcome.branch_provenance(0, total=43)
    with pytest.raises(ValueError, match="total"):
        outcome.remaining_provenance(5, 48)
    with pytest.raises(ValueError, match="consistent"):
        outcome.remaining_provenance(0, 49)


def test_the_selected_groupings_score_is_reused() -> None:
    variants, _ = analyzer()
    grouping = variants.select(root([*USERS, *ITEMS]))
    selected = grouping.decision.candidate.score
    assert grouping.scorer.score(grouping.grouping.groups) is selected  # cached per grouping
    outcome = variants.merge_or_split(grouping)
    assert outcome.decision.resolution.candidates[1].score == selected.total


def test_mdl_merges_when_one_schema_is_shorter() -> None:
    variants, report = shaped(None, ShapeScorer(merge=fixed(10, 10), split=fixed(50, 50)))
    outcome = variants.merge_or_split(variants.select(root(PETS)))
    assert outcome.variants.n_variants == 2
    assert outcome.merged and outcome.judgment == MERGE and outcome.n_variants == 1
    assert outcome.decision.method == "mdl" and outcome.views() == (outcome.variants.signatures.objects,)
    assert outcome.groups == (tuple(range(outcome.variants.signatures.n_signatures)),)
    assert outcome.details()["judgment"] == MERGE and outcome.details()["variants"] == 1
    assert outcome.details()["candidate_variants"] == 2
    with pytest.raises(ValueError, match="not split"):
        outcome.branch_provenance(0)
    assert [entry.resolution.method for entry in merge_decisions(report)] == ["mdl"]


def test_ties_go_to_merge_by_fewer_model_bits() -> None:
    variants, _ = shaped(None, ShapeScorer(merge=fixed(5, 20), split=fixed(20, 5)))
    outcome = variants.merge_or_split(variants.select(root(PETS)))
    assert outcome.merged and outcome.decision.method == "mdl" and outcome.decision.confidence == 0.5


def test_without_the_decision_model_and_mdl_the_priority_grouping_stays_split() -> None:
    # 5.11: MDL would merge, but with it disabled the Fallback_Behavior keeps the candidate Variants.
    variants, report = shaped({MDL: False}, ShapeScorer(merge=fixed(1, 1), split=fixed(50, 50)))
    grouping = variants.select(root(PETS))
    assert grouping.decision.method == "priority" and grouping.n_variants == 2
    outcome = variants.merge_or_split(grouping)
    assert not outcome.merged and outcome.groups == grouping.grouping.groups
    decision = outcome.decision
    assert decision.method == "fallback" and decision.label == SPLIT
    assert decision.technique == "fallback:merge_or_split"
    assert decision.fallback == FallbackInfo(DM_MERGE_SPLIT, "missing-model")
    assert decision.cutoff is None and decision.confidence is None
    assert [entry.resolution.method for entry in merge_decisions(report)] == ["fallback"]


def test_without_mdl_the_lsh_grouping_selected_by_priority_is_kept() -> None:
    variants, _ = analyzer({MDL: False}, ALL_THRESHOLDS)
    grouping = variants.select(root(FAMILIES))
    assert grouping.grouping.technique == MINHASH_LSH and grouping.n_variants >= 2
    outcome = variants.merge_or_split(grouping)
    assert outcome.judgment == SPLIT and outcome.groups == grouping.grouping.groups
    assert [v.digest for v in outcome.views()] == [v.digest for v in grouping.views()]


def test_a_decision_model_in_effect_decides_on_the_default_features() -> None:
    model = StubModel((SPLIT, MERGE), [0.3, 0.7])
    bundle = ModelBundle({"dm.merge_or_split.synthetic": model})
    report = InferenceReport()
    variants = VariantAnalyzer.for_techniques({DM_MERGE_SPLIT: True}, bundle=bundle, report=report, validator=VALIDATOR)
    outcome = variants.merge_or_split(variants.select(root([*USERS, *ITEMS])))  # MDL would split
    assert outcome.merged and outcome.decision.method == "decision-model"
    assert outcome.decision.technique == DM_MERGE_SPLIT and outcome.decision.confidence == 0.7
    assert outcome.decision.cutoff.source == "Decision_Model comparison"
    (x,) = model.seen
    assert x.shape == (1, 7) and np.all(np.isfinite(x))
    # Two candidate Variants with disjoint key sets (Jaccard 0) and no shared key (agreement 1).
    entropy = -math.fsum(p * math.log2(p) for p in (24 / 44, 20 / 44))
    assert x[0, :5].tolist() == [2.0, 0.0, 0.0, 0.0, 1.0] and math.isclose(x[0, 5], entropy)
    merge, split = outcome.decision.resolution.candidates
    assert (merge.outcome, merge.score, split.score) == (MERGE, 0.7, 0.3)


def test_a_missing_decision_model_falls_back_to_mdl() -> None:
    report = InferenceReport()
    variants = VariantAnalyzer.for_techniques(
        {DM_MERGE_SPLIT: True}, bundle=ModelBundle({}), report=report, validator=VALIDATOR
    )
    outcome = variants.merge_or_split(variants.select(root([*USERS, *ITEMS])))
    assert outcome.judgment == SPLIT and outcome.decision.method == "mdl"
    assert outcome.decision.fallback == FallbackInfo(DM_MERGE_SPLIT, "missing-model")  # 22.14
    assert any(sub.artifact == "dm.merge_or_split.synthetic" for sub in report.substitutions)


def test_features_measure_overlap_type_agreement_and_balance() -> None:
    docs: list[Any] = [{"k": 1, "a": 1}] * 3 + [{"k": "s", "b": 1}]
    variants, _ = analyzer()
    selected = variants.select(root(docs))
    sigs = selected.signatures
    assert sigs.n_signatures == 2
    split = VariantGrouping(
        selected.path, sigs, Grouping(BERNOULLI_MIXTURE, ((0,), (1,))), selected.decision, scorer=selected.scorer
    )
    candidates = [
        Candidate(technique=None, outcome=MERGE, score=Score(4.0, 6.0, 0.0, 0, 8)),
        Candidate(technique=None, outcome=SPLIT, score=Score(3.0, 3.0, 0.0, 0, 8)),
    ]
    x = merge_split_features(split, candidates)
    # {a, k} vs {b, k}: Jaccard 1/3; the shared key k is a number in one and a string in the other.
    entropy = -math.fsum(p * math.log2(p) for p in (3 / 4, 1 / 4))
    assert x.tolist()[:5] == [2.0, 1 / 3, 1 / 3, 1 / 3, 0.0]
    assert math.isclose(x[5], entropy) and x[6] == (10.0 - 6.0) / 8
    one = VariantGrouping(selected.path, sigs, Grouping.one(2), selected.decision)
    with pytest.raises(ValueError, match="two or more"):
        merge_split_features(one, candidates)


def test_one_candidate_variant_has_no_judgment_call() -> None:
    for view, states in ((root(USERS), None), (root(PETS), only())):
        variants, report = analyzer(states)
        outcome = variants.merge_or_split(variants.select(view))
        assert outcome.decision is None and outcome.judgment is None and not outcome.merged
        assert outcome.n_variants == 1 and merge_decisions(report) == []
        assert "judgment" not in outcome.details()
        node = variants.branches(outcome)
        assert node.keyword is None and node.branches == () and node.constraint_provenance() == {}
        assert node.keywords() == naive_merge_node(view).to_json() and node.branch_views() is None


def test_merge_or_split_without_a_grouping_scorer_scores_with_the_given_scorer() -> None:
    variants, _ = analyzer()
    selected = variants.select(root([*USERS, *ITEMS]))
    bare = VariantGrouping(selected.path, selected.signatures, selected.grouping, selected.decision)
    engine = DecisionEngine(scorer=SCORER)
    outcome = merge_or_split(bare, engine, scorer=SCORER)
    expected = variants.merge_or_split(selected).decision.resolution.candidates
    assert [c.score for c in outcome.decision.resolution.candidates] == [c.score for c in expected]


def test_outcomes_check_their_decision() -> None:
    variants, _ = analyzer()
    selected = variants.select(root([*USERS, *ITEMS]))
    with pytest.raises(ValueError, match="need a merge_or_split decision"):
        MergeSplitOutcome(selected, False)
    with pytest.raises(ValueError, match="merge_or_split"):
        MergeSplitOutcome(selected, False, selected.decision)  # a variant_grouping decision
    single = variants.select(root(USERS))
    with pytest.raises(ValueError, match="one candidate Variant"):
        MergeSplitOutcome(single, True)


# ---------------------------------------------------------------------------
# Branches (5.4, 5.5, 5.6)
# ---------------------------------------------------------------------------


def test_split_variants_become_one_anyof_branch_each() -> None:
    variants, _ = analyzer()
    view = root([*USERS, *ITEMS, 5, "s"])
    node = variants.variant_node(view)
    outcome = node.outcome
    assert outcome.judgment == SPLIT and node.keyword == ANY_OF  # open branches overlap (5.5)
    assert list(node.node.kw) == [ANY_OF] and len(node.branches) == outcome.n_variants == 2
    for branch, variant in zip(node.branches, node.views):
        # 5.4: each branch lists exactly the fields observed in its Variant's objects.
        observed = sorted(variant.store.key_name(int(kid)) for kid in variant.stats().key_ids.tolist())
        assert sorted(branch.kw["properties"]) == observed
        assert branch.to_json() == naive_merge_node(variant).to_json()
        assert branch.kw["type"] == ["object"]
    users, items = node.views
    assert object_count(users) + object_count(items) == len(USERS) + len(ITEMS)
    assert validates(node.keywords(), [*USERS, *ITEMS])
    assert all(len(matching_branches(node, doc)) == 1 for doc in [*USERS, *ITEMS])
    assert node.branch_views() == {node.node.nid: list(node.views)}


def test_branch_records_carry_both_decisions() -> None:
    variants, _ = analyzer()
    node = variants.variant_node(root([*USERS, *ITEMS]))
    records = node.constraint_provenance()
    assert list(records) == [ConstraintId(node.node.nid, ANY_OF, 0), ConstraintId(node.node.nid, ANY_OF, 1)]
    grouping = node.outcome.variants
    decision = node.outcome.decision
    for index, record in enumerate(records.values()):
        assert record.technique == MDL and record.resolution == decision.resolution
        assert record.cutoff == decision.cutoff and record.confidence == decision.confidence
        assert record.support.consistent == grouping.signatures.count_of(grouping.grouping.groups[index])
        assert record.support.total == len(USERS) + len(ITEMS)
        details = record.details
        assert details["variant"] == index and details["combinator"] == ANY_OF and details["judgment"] == SPLIT
        assert details["grouping"]["technique"] == BERNOULLI_MIXTURE  # the grouping facts
        upstream = details["variant_grouping"]  # 20.7, 26.3: the decision that formed the Variants
        assert upstream["technique"] == BERNOULLI_MIXTURE and upstream["method"] == "mdl"
        assert upstream["resolution"] == grouping.decision.resolution.to_json()
        assert upstream["cutoff"]["source"] == "MDL comparison"
        assert record.sample is None
    assert sum(r.support.consistent for r in records.values()) == len(USERS) + len(ITEMS)


def test_fallback_branch_records_name_the_fallback() -> None:
    variants, _ = analyzer({MDL: False})
    node = variants.variant_node(root(PETS))
    assert node.outcome.decision.method == "fallback"
    for record in node.provenance.values():
        assert record.technique == "fallback:merge_or_split"
        assert record.fallback == FallbackInfo(DM_MERGE_SPLIT, "missing-model")
        assert record.cutoff is None and record.confidence == laplace_confidence(record.support)
        assert record.details["variant_grouping"]["method"] == "priority"


def test_sampled_groupings_record_their_sample() -> None:
    thresholds = {"dbscan.eps": 0.5, "dbscan.min_samples": 1}
    variants, _ = analyzer({**only(DBSCAN_TECHNIQUE), MDL: False}, thresholds, resources={"sample.dbscan": 3})
    node = variants.variant_node(root(FAMILIES))
    grouping = node.outcome.variants
    assert grouping.sample == SampleInfo(3, grouping.signatures.n_signatures)
    assert node.keyword == ANY_OF and len(node.provenance) == grouping.n_variants >= 2
    assert all(record.sample == grouping.sample for record in node.provenance.values())  # 26.4


def test_a_merged_variant_has_the_union_of_fields_and_types() -> None:
    variants, _ = shaped(None, ShapeScorer(merge=fixed(1, 1), split=fixed(50, 50)))
    view = root([*NUMBERED, *NAMED])
    grouping = variants.select(view)
    assert grouping.n_variants == 2
    node = variants.branches(variants.merge_or_split(grouping))
    assert node.outcome.merged and node.keyword is None and node.provenance == {}
    kw = node.keywords()
    # 5.6: the union of the fields, required only where every candidate Variant has it, the union of types.
    assert sorted(kw["properties"]) == ["a", "b", "id"]
    assert kw["required"] == ["id"]
    assert kw["properties"]["id"]["type"] == ["integer", "string"]
    assert kw == naive_merge_node(view).to_json()
    assert validates(kw, [*NUMBERED, *NAMED])


def test_branches_requiring_one_property_with_disjoint_enums_are_oneof() -> None:
    variants, _ = analyzer({MDL: False})
    node = variants.variant_node(root(PETS), variant_schema=kind_enum)
    assert node.keyword == ONE_OF and len(node.branches) == 2
    kinds = sorted(branch.kw["properties"]["kind"].kw["enum"][0] for branch in node.branches)
    assert kinds == ["cat", "dog"]
    assert validates(node.keywords(), PETS)
    assert all(len(matching_branches(node, doc)) == 1 for doc in PETS)
    assert all(record.details["combinator"] == ONE_OF for record in node.provenance.values())
    assert list(node.constraint_provenance())[0] == ConstraintId(node.node.nid, ONE_OF, 0)


def test_overlapping_enums_stay_anyof() -> None:
    docs = [*PETS, {"kind": "cat", "bark": "meow"}]  # a cat that barks joins the dogs' Variant
    variants, _ = analyzer({MDL: False})
    node = variants.variant_node(root(docs), variant_schema=kind_enum)
    assert node.outcome.n_variants == 2 and node.keyword == ANY_OF
    assert validates(node.keywords(), docs)


def test_closed_branches_with_unlisted_required_fields_are_oneof() -> None:
    variants, _ = analyzer()
    docs = [*USERS, *ITEMS]
    open_node = variants.variant_node(root(docs))
    closed_node = variants.variant_node(root(docs), variant_schema=closed)
    assert open_node.keyword == ANY_OF and closed_node.keyword == ONE_OF
    assert validates(closed_node.keywords(), docs)


# ---------------------------------------------------------------------------
# The disjointness test (5.5)
# ---------------------------------------------------------------------------


def test_a_required_field_the_other_branch_forbids_makes_branches_disjoint() -> None:
    needs_a = schema({"type": "object", "properties": {"a": {}}, "required": ["a"]})
    open_ = schema({"type": "object", "properties": {"b": {}}})
    closed_ = schema({"type": "object", "properties": {"b": {}}, "additionalProperties": False})
    forbids = schema({"type": "object", "properties": {"a": False}})
    pattern_false = schema({"type": "object", "patternProperties": {"^a$": False}})
    pattern_true = schema({"type": "object", "patternProperties": {"^a": {}}, "additionalProperties": False})
    short_names = schema({"type": "object", "propertyNames": {"maxLength": 0}})
    assert not branches_disjoint(needs_a, open_)
    for other in (closed_, forbids, pattern_false, short_names, SchemaNode(boolean=False)):
        assert branches_disjoint(needs_a, other) and branches_disjoint(other, needs_a)
    assert not branches_disjoint(needs_a, pattern_true)  # matched by a Pattern, so permitted
    assert not branches_disjoint(needs_a, SchemaNode(boolean=True))
    assert not permits_field(closed_, "a") and permits_field(closed_, "b") and permits_field(open_, "a")


def test_common_required_properties_with_disjoint_value_sets() -> None:
    def tagged(values: Any, key: str = "enum", required: bool = True) -> SchemaNode:
        doc: dict[str, Any] = {"type": "object", "properties": {"t": {key: values}}}
        if required:
            doc["required"] = ["t"]
        return schema(doc)

    assert branches_disjoint(tagged(["a", "b"]), tagged(["c"]))
    assert branches_disjoint(tagged("a", "const"), tagged(["b", "c"]))
    assert not branches_disjoint(tagged(["a", "b"]), tagged(["b", "c"]))
    assert not branches_disjoint(tagged([1]), tagged([1.0]))  # JSON Schema equality: 1 == 1.0
    assert branches_disjoint(tagged([1]), tagged([True]))  # booleans never equal numbers
    assert not branches_disjoint(tagged(["a"]), tagged(["b"], required=False))  # both must require it
    assert not branches_disjoint(tagged(["a"]), schema({"type": "object", "required": ["t"]}))  # unconstrained
    both = schema({"type": "object", "properties": {"t": {"const": "a", "enum": ["a", "b"]}}, "required": ["t"]})
    assert branches_disjoint(both, tagged(["b"]))  # const and enum together allow only "a"
    assert not branches_disjoint(schema({"$ref": "#/$defs/x"}), tagged(["a"]))  # unreadable: overlapping


def test_every_pair_must_be_disjoint() -> None:
    a = schema({"properties": {"t": {"const": 1}}, "required": ["t"]})
    b = schema({"properties": {"t": {"const": 2}}, "required": ["t"]})
    c = schema({"properties": {"t": {"enum": [2, 3]}}, "required": ["t"]})
    assert pairwise_disjoint([a, b]) and pairwise_disjoint([a, c])
    assert not pairwise_disjoint([a, b, c])  # b and c overlap on 2
    assert pairwise_disjoint([a]) and pairwise_disjoint([])


def test_variant_nodes_check_their_shape() -> None:
    variants, _ = analyzer()
    outcome = variants.merge_or_split(variants.select(root([*USERS, *ITEMS])))
    node = variant_branches(outcome)
    with pytest.raises(ValueError, match="every branch"):
        VariantNode(node.path, node.node, outcome, node.views, node.keyword, {0: node.provenance[0]})
    with pytest.raises(ValueError, match="one branch per final Variant"):
        VariantNode(node.path, node.node, outcome, node.views[:1], node.keyword, node.provenance)
    with pytest.raises(TypeError, match="SchemaNode"):
        variant_branches(outcome, variant_schema=lambda view: {"type": "object"})  # type: ignore[arg-type,return-value]


def test_merge_or_split_and_branches_do_not_depend_on_document_order() -> None:
    docs = [*USERS, *ITEMS, *PETS]
    shuffled = list(docs)
    random.Random(5).shuffle(shuffled)
    seen = set()
    for order in (docs, docs[::-1], shuffled):
        variants, report = analyzer(thresholds=ALL_THRESHOLDS)
        node = variants.variant_node(root(order))
        records = [record.to_json() for record in node.provenance.values()]
        seen.add(canonical_bytes([node.keywords(), records, [entry.to_json() for entry in merge_decisions(report)]]))
    assert len(seen) == 1
