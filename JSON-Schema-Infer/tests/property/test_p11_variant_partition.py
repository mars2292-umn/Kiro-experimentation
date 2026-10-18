"""Property 11: Variant partition invariant (task 13.5).

For any object Path with two or more Object_Signatures and every enabled
grouping Technique, the candidate Variants partition the non-flagged
objects. Objects with equal signatures share a Variant, the Variant count
lies in [1, #signatures], and clustering noise never produces Flagged_Items.

Reference
    The non-flagged objects at the Path are collected from the raw
    documents, independently of the ObservationStore, and identified by
    ``(document index, array position)`` (position ``-1`` outside arrays),
    which is what a view row's ``doc`` and ``pos`` fields hold. An object's
    reference Object_Signature follows the glossary: the set of its keys,
    each with the JSON type of its value, where integers and other numbers
    are one type (``number``) and booleans are not numbers. A non-flagged
    object whose member value is a Flagged_Item keeps that key and type in
    its signature (the item is part of the object, not the other way round).
    The test first checks that :class:`ObjectSignatures` finds exactly the
    reference signatures, with their object counts, so ``#signatures`` in
    the bound is the reference's.

Inputs
    Objects come from one to three planted families: a base of up to three
    ``(key, type)`` entries, varied by dropping a base key and by adding or
    retyping up to two keys, so signatures are similar within a family and
    the clustering Techniques find clusters and noise. Values of each type
    include integral floats (``1.0``, ``1e3``, ``-0.0``, ``2**70``) for
    ``integer`` and non-integral ones for ``number``, so a key holding ``1``
    in one object and ``2.5`` in another gives one signature. Keys come
    from :data:`tests.strategies.KEYS` (the empty key, astral and
    non-ASCII keys). The objects sit at ``$``, ``$.v``, or ``$.v[*]`` (each
    test runs once per layout, so each gets the full example budget), next
    to rows at the Path that are not objects (scalars, arrays, and arrays of
    objects one level deeper) and documents without ``v``.

    Flags: outlier documents with a signature found nowhere else are
    flagged, as are up to two other documents; up to two objects at the
    Path are flagged as items; up to two member values of non-flagged
    objects are flagged as items; and at ``$.v[*]`` a whole ``v`` array may
    be flagged, which excludes its elements.

Checks
    ``test_every_grouping_technique_partitions_the_non_flagged_objects``
    calls the four grouping Techniques directly with drawn Learned_Thresholds
    and Resource_Parameters, including sample sizes far below the signature
    count (so DBSCAN and HDBSCAN cluster a sample and extend it), radii and
    minimum sizes that leave every signature as noise, and BIC capped at one
    component.

    ``test_selected_variants_partition_the_non_flagged_objects`` runs
    :meth:`VariantAnalyzer.select` with Technique states from
    ``run_configs()`` and a stub Model_Bundle of hand-set Learned_Thresholds
    (some missing). Scenarios force the priority path (``decide.mdl`` off),
    the one-Variant fallback with every grouping Technique disabled, and the
    fallback with only threshold-reading Techniques enabled and no
    thresholds; otherwise the states are drawn. Every offered candidate
    grouping and the selected one (via :meth:`VariantGrouping.views`) are
    checked, and the offered Techniques are exactly those in effect, so no
    enabled Technique escapes the check.

    In both tests the store's FlagSet (epoch, documents, and items) is the
    same before and after, so noise objects never become Flagged_Items, and
    they are still in exactly one Variant.

**Validates: Requirements 5.1, 5.2, 5.9**
"""

# Feature: json-schema-inference, Property 11: Variant partition invariant

from __future__ import annotations

from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Any

import numpy as np
import pytest
from hypothesis import assume, event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core.observations import ObservationStore, build_store
from jsinfer.core.paths import parse_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.structure.variants import (
    BERNOULLI_MIXTURE,
    GROUPING_TECHNIQUES,
    Grouping,
    GroupingScorer,
    ObjectSignatures,
    VariantAnalyzer,
    bernoulli_mixture_grouping,
    dbscan_grouping,
    hdbscan_grouping,
    minhash_lsh_grouping,
)
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, run_configs

# One Validator and MDL_Scorer for the module, so compiled schemas are cached across examples.
VALIDATOR = Validator()
SCORER = MDLScorer(validator=VALIDATOR)
MDL = "decide.mdl"

ObjectId = tuple[int, int]  # (document index, array position or -1)
Signature = frozenset[tuple[str, str]]

# ---------------------------------------------------------------------------
# Reference Object_Signatures
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


def signature_of(obj: Mapping[str, Any]) -> Signature:
    return frozenset((key, json_type(value)) for key, value in obj.items())


LAYOUTS = ("$", "$.v", "$.v[*]")


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


# ---------------------------------------------------------------------------
# Strategies
# ---------------------------------------------------------------------------

# Values by template type; "integer" and "number" are one signature type.
VALUES: dict[str, st.SearchStrategy[Any]] = {
    "null": st.none(),
    "boolean": st.booleans(),
    "integer": st.one_of(st.integers(-3, 3), st.sampled_from([1.0, 1e3, -0.0, 2**70])),
    "number": st.sampled_from([2.5, -0.5, 0.1]),
    "string": st.text("ab😀", max_size=3),
    "array": st.lists(st.none() | st.integers(0, 2) | st.just({"a": 1}), max_size=2),
    "object": st.dictionaries(st.sampled_from(KEYS[:3]), st.integers(0, 2), max_size=2),
}
TEMPLATE_TYPES = tuple(VALUES)

# Rows at the Path that are not objects, including arrays of objects one level deeper.
NON_OBJECTS: tuple[Any, ...] = (None, 3, 2.5, "s", True, [], [{"a": 1}], [1, [2]])

# The template of outlier documents: its key occurs in no other template.
OUTLIER_KEY = "outlier"
OUTLIER_TEMPLATE = {OUTLIER_KEY: "string", "a": "integer"}


def template_signature(template: Mapping[str, str]) -> Signature:
    return frozenset((key, "number" if kind == "integer" else kind) for key, kind in template.items())


@st.composite
def templates(draw: st.DrawFn) -> list[dict[str, str]]:
    """Object templates (key → type) in one to three families, with distinct signatures."""
    entries = st.dictionaries(st.sampled_from(KEYS), st.sampled_from(TEMPLATE_TYPES), max_size=2)
    out: dict[Signature, dict[str, str]] = {}
    for _ in range(draw(st.integers(1, 3), label="families")):
        base = draw(st.dictionaries(st.sampled_from(KEYS), st.sampled_from(TEMPLATE_TYPES), max_size=3), label="base")
        for _ in range(draw(st.integers(1, 3), label="family members")):
            member = dict(base)
            if base and draw(st.booleans(), label="drop a base key"):
                del member[draw(st.sampled_from(sorted(base)))]
            member.update(draw(entries, label="added or retyped keys"))
            out.setdefault(template_signature(member), member)
    found = list(out.values())
    assume(len(found) >= 2)
    return found


@st.composite
def instances(draw: st.DrawFn, template: Mapping[str, str]) -> dict[str, Any]:
    return {key: draw(VALUES[kind]) for key, kind in template.items()}


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
    """Documents, their flags, and the reference: each non-flagged object at the Path → its signature."""

    layout: str
    docs: tuple[Any, ...]
    flagged_docs: tuple[int, ...]
    flagged_objects: tuple[ObjectId, ...]  # objects at the Path flagged as items
    flagged_members: tuple[tuple[ObjectId, str], ...]  # member values of non-flagged objects flagged as items
    flagged_arrays: tuple[int, ...]  # documents whose $.v array is flagged ($.v[*] only)
    reference: Mapping[ObjectId, Signature]

    @property
    def n_signatures(self) -> int:
        return len(set(self.reference.values()))

    @property
    def flagged(self) -> bool:
        return bool(self.flagged_docs or self.flagged_objects or self.flagged_members or self.flagged_arrays)


@st.composite
def cases(draw: st.DrawFn, layout: str) -> Case:
    pool = draw(templates(), label="templates")
    n = draw(st.integers(2, 20), label="objects")
    # The first two objects have different signatures; the rest are drawn with repetition.
    first = draw(st.permutations(range(len(pool))), label="first templates")[:2]
    picks = [*first, *draw(st.lists(st.integers(0, len(pool) - 1), min_size=n - 2, max_size=n - 2), label="picks")]
    objects = [draw(instances(pool[i])) for i in picks]
    outliers = [draw(instances(OUTLIER_TEMPLATE)) for _ in range(draw(st.integers(0, 2), label="outliers"))]
    others = draw(st.lists(st.sampled_from(NON_OBJECTS), max_size=3), label="non-object rows")

    # Documents: (document, outlier?) pairs.
    if layout == "$":
        units = [(obj, False) for obj in objects] + [(x, False) for x in others] + [(obj, True) for obj in outliers]
    elif layout == "$.v":
        units = [({"v": x}, False) for x in [*objects, *others]] + [({"v": obj}, True) for obj in outliers]
        units += [({"w": 1}, False)] * draw(st.integers(0, 1), label="documents without v")
    else:
        mixed = draw(st.permutations([*objects, *others]), label="element order")
        units = [({"v": chunk}, False) for chunk in _chunks(draw, mixed)]
        units += [({"v": [obj]}, True) for obj in outliers]
        units += [({"v": 3}, False), ({"w": 1}, False)][: draw(st.integers(0, 2), label="documents without arrays")]
    units = [units[i] for i in draw(st.permutations(range(len(units))), label="document order")]
    docs = tuple(doc for doc, _ in units)

    extra = draw(st.sets(st.integers(0, len(docs) - 1), max_size=2), label="flagged clean documents")
    flagged_docs = tuple(i for i, (_, outlier) in enumerate(units) if outlier or i in extra)
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
    reference = {oid: signature_of(obj) for oid, obj in kept}
    case = Case(layout, docs, flagged_docs, flagged_objects, flagged_members, flagged_arrays, reference)
    assume(case.n_signatures >= 2)  # the property is about Paths with two or more signatures
    return case


# ---------------------------------------------------------------------------
# Building the store with its flags, and the checks
# ---------------------------------------------------------------------------


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


def flag_state(store: ObservationStore) -> tuple[Any, ...]:
    flags = store.flags
    return flags.epoch, flags.documents.tolist(), [(column, rows.tolist()) for column, rows in flags.items()]


def check_signatures(signatures: ObjectSignatures, case: Case) -> int:
    """``signatures`` are exactly the reference signatures, with their object counts; returns their number."""
    found: dict[Signature, int] = {}
    for s in range(signatures.n_signatures):
        entries = frozenset(signatures.entries(s))
        ids = object_ids(signatures.view((s,)))
        assert len(ids) == int(signatures.counts[s]), f"signature {sorted(entries)}: view and count disagree"
        wrong = [oid for oid in ids if case.reference.get(oid) != entries]
        assert not wrong, f"objects {wrong[:3]} are not non-flagged objects with signature {sorted(entries)}"
        assert entries not in found, f"signature {sorted(entries)} is listed twice"
        found[entries] = len(ids)
    assert found == dict(Counter(case.reference.values())), "the Object_Signatures differ from the reference"
    return signatures.n_signatures


def check_variants(views: Sequence[View], case: Case, where: str) -> None:
    """The candidate Variants' views partition the non-flagged objects, keep signatures whole, and are few enough."""
    owner: dict[ObjectId, int] = {}
    holders: dict[Signature, list[int]] = {}
    for v, view in enumerate(views):
        ids = object_ids(view)
        assert ids, f"candidate Variant {v} of {where} has no object"
        for oid in ids:
            assert oid in case.reference, f"{where}: {oid} in Variant {v} is not a non-flagged object at the Path"
            assert oid not in owner, f"{where}: object {oid} is in Variants {owner[oid]} and {v}"
            owner[oid] = v
            sig = case.reference[oid]
            if v not in holders.setdefault(sig, []):
                holders[sig].append(v)
    missing = [oid for oid in case.reference if oid not in owner]
    assert not missing, f"{where}: non-flagged objects {missing[:3]} are in no Variant"
    split = {tuple(sorted(sig)): vs for sig, vs in holders.items() if len(vs) > 1}
    assert not split, f"{where}: objects with one signature are in several Variants: {split}"
    assert 1 <= len(views) <= case.n_signatures, f"{where}: {len(views)} Variants for {case.n_signatures} signatures"


def note_case(case: Case) -> None:
    if case.flagged_docs:
        event("flagged documents")
    if case.flagged_members:
        event("flagged member values")
    if case.flagged_objects or case.flagged_arrays:
        event("flagged objects or arrays")


def note_grouping(grouping: Grouping) -> None:
    if grouping.details.get("noise_signatures", 0):
        event(f"{grouping.technique}: noise assigned by description length")
    if grouping.sample is not None:
        event(f"{grouping.technique}: clustered a sample")
    event(f"{grouping.technique}: {'one' if grouping.n_variants == 1 else 'several'} Variants")


# ---------------------------------------------------------------------------
# Learned_Thresholds and Resource_Parameters
# ---------------------------------------------------------------------------

# Hand-set Learned_Thresholds inside their admissible ranges, with the extremes common.
THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    "lsh.keyset_jaccard": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
    "dbscan.eps": st.one_of(st.sampled_from([1e-9, 0.34, 0.5, 1.0]), st.floats(0.0, 1.0, exclude_min=True)),
    "dbscan.min_samples": st.one_of(st.integers(1, 4), st.integers(5, 40)),
    "hdbscan.min_cluster_size": st.one_of(st.integers(2, 4), st.integers(5, 12)),
}

# Small sample sizes make DBSCAN and HDBSCAN cluster a sample of the signatures (29.8).
SAMPLE_SIZES = st.one_of(st.integers(1, 4), st.just(5_000))


@st.composite
def resource_values(draw: st.DrawFn) -> dict[str, int | float]:
    """The Resource_Parameters the grouping Techniques read, kept small so MinHash and EM stay fast."""
    perm = draw(st.sampled_from([1, 8, 32, 128]), label="minhash.perm")
    return {
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "sample.dbscan": draw(SAMPLE_SIZES, label="sample.dbscan"),
        "sample.hdbscan": draw(SAMPLE_SIZES, label="sample.hdbscan"),
        "variants.max_components": draw(st.integers(1, 10), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
        "variants.em_tol": draw(st.sampled_from([0.0, 1e-6, 1e-2]), label="variants.em_tol"),
    }


@st.composite
def all_thresholds(draw: st.DrawFn) -> dict[str, int | float]:
    return {name: draw(values, label=name) for name, values in THRESHOLD_VALUES.items()}


class StubBundle:
    """A Model_Bundle of hand-set Learned_Thresholds by name; no Decision_Models."""

    def __init__(self, thresholds: Mapping[str, int | float]) -> None:
        self.thresholds = dict(thresholds)

    def model(self, name: str) -> Any | None:
        return None

    def threshold(self, name: str) -> int | float | None:
        return self.thresholds.get(name)


# ---------------------------------------------------------------------------
# Every grouping Technique, called directly
# ---------------------------------------------------------------------------


# One run per layout, so each Path shape gets the profile's full example budget.
@pytest.mark.parametrize("layout", LAYOUTS)
@given(
    data=st.data(),
    thresholds=all_thresholds(),
    resources=resource_values(),
    seed=st.integers(0, 2**63 - 1),
)
def test_every_grouping_technique_partitions_the_non_flagged_objects(
    layout: str,
    data: st.DataObject,
    thresholds: dict[str, int | float],
    resources: dict[str, int | float],
    seed: int,
) -> None:
    case = data.draw(cases(layout), label="case")
    store, view = build(case)
    before = flag_state(store)
    signatures = ObjectSignatures.of(view)
    check_signatures(signatures, case)
    scorer = GroupingScorer(signatures, SCORER)

    groupings = [
        minhash_lsh_grouping(
            signatures,
            scorer,
            threshold=float(thresholds["lsh.keyset_jaccard"]),
            perm=int(resources["minhash.perm"]),
            bands=int(resources["lsh.bands"]),
            seed=seed,
        ),
        dbscan_grouping(
            signatures,
            scorer,
            eps=float(thresholds["dbscan.eps"]),
            min_samples=int(thresholds["dbscan.min_samples"]),
            sample_size=int(resources["sample.dbscan"]),
            seed=seed,
        ),
        hdbscan_grouping(
            signatures,
            scorer,
            min_cluster_size=int(thresholds["hdbscan.min_cluster_size"]),
            sample_size=int(resources["sample.hdbscan"]),
            seed=seed,
        ),
        bernoulli_mixture_grouping(
            signatures,
            max_components=int(resources["variants.max_components"]),
            max_iter=int(resources["variants.em_max_iter"]),
            tol=float(resources["variants.em_tol"]),
            seed=seed,
        ),
    ]
    assert [g.technique for g in groupings] == list(GROUPING_TECHNIQUES)
    for grouping in groupings:
        where = f"{grouping.technique} at {case.layout}"
        signatures.check_partition(grouping.groups)
        check_variants([signatures.view(group) for group in grouping.groups], case, where)
        note_grouping(grouping)

    # Clustering noise was assigned to Variants, never flagged (5.9).
    assert flag_state(store) == before, "a grouping Technique changed the flags"
    note_case(case)


# ---------------------------------------------------------------------------
# VariantAnalyzer.select under drawn Run_Configurations
# ---------------------------------------------------------------------------

SCENARIOS = ("drawn", "priority", "all disabled", "missing thresholds")


@dataclass(frozen=True)
class Setup:
    scenario: str
    config: RunConfiguration
    thresholds: Mapping[str, int | float]


@st.composite
def setups(draw: st.DrawFn) -> Setup:
    """Technique states from ``run_configs()`` (implemented Techniques only), resources, and a stub bundle."""
    scenario = draw(st.sampled_from(SCENARIOS), label="scenario")
    base: dict[str, Any] = dict(draw(resource_values(), label="resources"))
    thresholds = draw(all_thresholds(), label="thresholds")
    if scenario == "priority":
        base[technique_setting(MDL)] = False
    elif scenario == "all disabled":
        base.update({technique_setting(tid): False for tid in GROUPING_TECHNIQUES})
    elif scenario == "missing thresholds":
        base[technique_setting(BERNOULLI_MIXTURE)] = False
        thresholds = {}
    if scenario in ("drawn", "priority"):
        # Each Technique's thresholds are present together or absent (20.6: a missing one disables it).
        present = {
            tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0
            for tid in GROUPING_TECHNIQUES
            if REGISTRY.technique(tid).thresholds
        }
        thresholds = {
            name: value
            for name, value in thresholds.items()
            if all(present[tid] for tid in present if name in REGISTRY.technique(tid).thresholds)
        }
    config = draw(run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base=base), label="config")
    return Setup(scenario, config, thresholds)


def in_effect(setup: Setup) -> list[str]:
    """The grouping Techniques that run: implemented, enabled, and with every Learned_Threshold they read."""
    return [
        tid
        for tid in GROUPING_TECHNIQUES
        if REGISTRY.technique(tid).implemented
        and setup.config.techniques[tid]
        and all(name in setup.thresholds for name in REGISTRY.technique(tid).thresholds)
    ]


@pytest.mark.parametrize("layout", LAYOUTS)
@given(data=st.data(), setup=setups())
def test_selected_variants_partition_the_non_flagged_objects(layout: str, data: st.DataObject, setup: Setup) -> None:
    case = data.draw(cases(layout), label="case")
    store, view = build(case)
    before = flag_state(store)
    config = setup.config
    analyzer = VariantAnalyzer.for_techniques(
        config.techniques,
        bundle=StubBundle(setup.thresholds),
        report=InferenceReport(),
        validator=VALIDATOR,
        resources=config.resources,
        seed=config.seed,
    )
    outcome = analyzer.select(view)
    signatures = outcome.signatures
    check_signatures(signatures, case)

    # Every grouping Technique in effect offered a grouping, and each one partitions the objects.
    expected = in_effect(setup)
    assert [g.technique for g in outcome.candidates] == expected
    for grouping in outcome.candidates:
        check_variants([signatures.view(group) for group in grouping.groups], case, f"{grouping.technique} candidate")
        note_grouping(grouping)

    # The selected candidate Variants partition the objects too.
    decision = outcome.decision
    assert decision is not None, "a Path with two or more signatures resolves variant_grouping"
    where = f"selected grouping ({decision.method}) at {case.layout}"
    check_variants(list(outcome.views()), case, where)
    if expected:
        assert any(outcome.grouping is g for g in outcome.candidates), f"{where} was not offered"
        if setup.scenario == "priority":
            assert decision.method == "priority" and outcome.grouping is outcome.candidates[0]
    else:
        assert decision.method == "fallback" and outcome.n_variants == 1, f"{where}: expected the one-Variant fallback"

    assert flag_state(store) == before, "Variant selection changed the flags"
    note_case(case)
    event(f"scenario {setup.scenario}: resolved by {decision.method}")
