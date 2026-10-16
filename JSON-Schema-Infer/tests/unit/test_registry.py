"""Unit tests for config/registry.py (task 3.1; Requirements 28.1, 28.3, 29.8).

The expected tables below are transcribed from the design's Technique
registry section (Decision_Point table, tie rules, Learned_Threshold table,
and the super-linear sampling list), so a drifting registry entry fails here.
"""

from __future__ import annotations

import dataclasses
import pickle

import pytest

from jsinfer.config.registry import (
    REGISTRY,
    TIE_CRITERIA,
    TIE_FEWER_SUBPATTERNS,
    TIE_NEAREST_EXTREME,
    ConstantOutcome,
    DecisionPointSpec,
    Registry,
    RegistryError,
    TechniqueSpec,
    TieRule,
)

# Decision_Point kind -> (serving Techniques in priority order, fallback outcome).
EXPECTED_DECISION_POINTS = {
    "record_or_map": (("decide.dm_record_map", "decide.mdl"), "record"),
    "record_map_partition": (("structure.hybrid_record_map",), "none"),
    "closed_or_open_record": (("decide.dm_closed_record", "decide.mdl"), "open"),
    "variant_grouping": (
        ("variants.minhash_lsh", "variants.dbscan", "variants.hdbscan", "variants.bernoulli_mixture"),
        "one-variant",
    ),
    "merge_or_split": (("decide.dm_merge_split", "decide.mdl"), "split"),
    "field_groups": (("groups.fpgrowth_slim", "groups.bmf"), "none"),
    "tuple_or_list": (("decide.dm_tuple_list", "decide.mdl"), "list"),
    "array_pattern": (("structure.array_grammar",), "base-array-schema"),
    "shared_definition": (("structure.shared_defs", "structure.recursion"), "inline"),
    "cooccurrence_rule": (("rules.cooccurrence",), "none"),
    "discriminator": (("rules.decision_trees",), "none"),
    "exclusion_rule": (("rules.exclusion",), "none"),
    "key": (("rules.keys",), "none"),
    "reference": (("rules.fd_ind",), "none"),
    "fd": (("rules.fd_ind",), "none"),
    "string_constraint": (
        (
            "values.latent_type",
            "values.format",
            "values.semantic_type",
            "corpus.name_suggest",
            "corpus.cf_suggest",
            "values.pattern",
        ),
        "none",
    ),
    "closed_or_open_enum": (("decide.dm_closed_enum", "values.enum_estimation"), "open"),
    "bound": (("values.bounds",), "none"),
    "document_outlier": (("robust.isolation_forest", "robust.lof"), "not-flagged"),
    "value_exception": (("robust.mdl_exceptions",), "widen"),
    "version_split": (("robust.change_point",), "single-schema"),
    "suggestion": (("corpus.name_suggest", "corpus.cf_suggest"), "none"),
    "relaxation": (("decide.mdl",), "candidate-order"),
    "judgment_model": (("decide.weak_supervision",), "synthetic-only"),
}

EXPECTED_TIE_RULES = {
    "record_or_map": (("record",), ("4.10",)),
    "array_pattern": (("base-array-schema",), ("7.8",)),
    "string_constraint": (("none", TIE_FEWER_SUBPATTERNS), ("14.2",)),
    "closed_or_open_enum": (("open",), ("15.6",)),
    "bound": ((TIE_NEAREST_EXTREME,), ("16.7",)),
    "value_exception": (("widen",), ("17.2", "17.3")),
}

EXPECTED_TIERS = {
    "must": [
        "variants.minhash_lsh", "variants.dbscan", "variants.hdbscan", "variants.bernoulli_mixture",
        "rules.cooccurrence", "values.latent_type", "values.format", "values.enum_estimation", "values.bounds",
        "decide.mdl", "decide.dm_record_map", "decide.dm_closed_record", "decide.dm_tuple_list",
        "decide.dm_closed_enum", "decide.dm_merge_split",
    ],
    "should": [
        "groups.fpgrowth_slim", "structure.hybrid_record_map", "structure.shared_defs", "structure.recursion",
        "rules.decision_trees", "rules.exclusion", "values.pattern", "robust.isolation_forest", "robust.lof",
        "robust.mdl_exceptions", "decide.weak_supervision", "corpus.name_suggest", "corpus.nearest_schema",
    ],
    "could": [
        "groups.bmf", "structure.array_grammar", "rules.keys", "rules.fd_ind", "values.semantic_type",
        "robust.change_point", "corpus.cf_suggest",
    ],
}

TABLE_1_AREAS = {
    "variants": "Variant discovery",
    "groups": "Field groups",
    "structure": "Structure",
    "rules": "Inter-field rules",
    "values": "Values",
    "robust": "Robustness",
    "decide": "Decisions",
    "corpus": "Corpus",
}

# Super-linear Techniques and their sample-size Resource_Parameters (design "Sampling", 29.8).
EXPECTED_SAMPLE_PARAMS = {
    "variants.dbscan": "sample.dbscan",
    "variants.hdbscan": "sample.hdbscan",
    "robust.lof": "sample.lof",
    "groups.fpgrowth_slim": "sample.fpgrowth",
    "groups.bmf": "sample.bmf",
    "rules.decision_trees": "sample.trees",
    "rules.fd_ind": "sample.fd",
    "values.pattern": "sample.patterns",
    "robust.change_point": "sample.changepoint",
}

# Learned_Thresholds read at inference (design Learned_Threshold table).
EXPECTED_THRESHOLDS = {
    "variants.minhash_lsh": {"lsh.keyset_jaccard"},
    "variants.dbscan": {"dbscan.eps", "dbscan.min_samples"},
    "variants.hdbscan": {"hdbscan.min_cluster_size"},
    "rules.cooccurrence": {"cooccur.min_support", "cooccur.min_confidence"},
    "rules.decision_trees": {"tagged.branch_support"},
    "rules.exclusion": {"exclusion.min_confidence"},
    "rules.keys": {"keys.min_confidence"},
    "rules.fd_ind": {"ind.min_confidence", "fd.min_confidence"},
    "values.semantic_type": {"semantic.min_probability"},
    "robust.isolation_forest": {"iforest.score"},
    "robust.lof": {"lof.score", "lof.n_neighbors"},
    "robust.change_point": {"changepoint.penalty", "changepoint.min_segment"},
    "corpus.name_suggest": {"corpus.suggest_min_confidence"},
    "corpus.cf_suggest": {"corpus.suggest_min_confidence"},
}


# ---------------------------------------------------------------------------
# The shipped registry matches the design
# ---------------------------------------------------------------------------


def test_decision_points_match_design_table() -> None:
    assert REGISTRY.kinds() == tuple(EXPECTED_DECISION_POINTS)
    for kind, (served_by, outcome) in EXPECTED_DECISION_POINTS.items():
        dp = REGISTRY.decision_point(kind)
        assert dp.served_by == served_by, kind
        assert dp.fallback == ConstantOutcome(outcome), kind
        assert dp.fallback_doc and dp.requirements, kind


def test_tie_rules_match_design() -> None:
    rules = {kind: dp.tie_rule for kind, dp in REGISTRY.decision_points.items() if dp.tie_rule is not None}
    assert {kind: (rule.favors, rule.requirements) for kind, rule in rules.items()} == EXPECTED_TIE_RULES
    assert set(TIE_CRITERIA) == {TIE_FEWER_SUBPATTERNS, TIE_NEAREST_EXTREME}


def test_technique_entries_match_design() -> None:
    assert len(REGISTRY.ids()) == 35
    assert sorted(REGISTRY.ids()) == sorted(id_ for ids in EXPECTED_TIERS.values() for id_ in ids)
    for tier, ids in EXPECTED_TIERS.items():
        for tid in ids:
            spec = REGISTRY.technique(tid)
            assert spec.tier == tier, tid
            assert spec.area == TABLE_1_AREAS[tid.partition(".")[0]], tid
            # Nothing is built yet: every entry is unimplemented and off by default.
            assert spec.implemented is False and spec.default_enabled is False, tid
            assert spec.prerequisites == (("decide.mdl",) if tid == "structure.hybrid_record_map" else ()), tid
            assert spec.sample_param == EXPECTED_SAMPLE_PARAMS.get(tid), tid
            assert spec.superlinear is (tid in EXPECTED_SAMPLE_PARAMS), tid
            assert set(spec.thresholds) == EXPECTED_THRESHOLDS.get(tid, set()), tid
    assert REGISTRY.default_states() == {tid: False for tid in REGISTRY.ids()}


def test_serves_and_priority_agree_with_decision_points() -> None:
    for tid, spec in REGISTRY.techniques.items():
        expected = [kind for kind, (served_by, _) in EXPECTED_DECISION_POINTS.items() if tid in served_by]
        assert list(spec.serves) == expected, tid
        for kind in spec.serves:
            assert spec.priority[kind] == REGISTRY.rank(tid, kind) == REGISTRY.servers(kind).index(tid) + 1
    assert REGISTRY.technique("corpus.nearest_schema").serves == ()
    assert REGISTRY.technique("decide.mdl").priority == {
        "record_or_map": 2, "closed_or_open_record": 2, "merge_or_split": 2, "tuple_or_list": 2, "relaxation": 1,
    }


# ---------------------------------------------------------------------------
# Fallback_Behaviors are constant and invoke no Technique (28.3, 28.4)
# ---------------------------------------------------------------------------


def test_fallbacks_are_constant_and_ignore_the_view() -> None:
    class Exploding:
        def __getattr__(self, name: str) -> object:
            raise AssertionError(f"a Fallback_Behavior inspected the view ({name})")

    for kind in REGISTRY.kinds():
        fallback = REGISTRY.fallback(kind)
        assert isinstance(fallback, ConstantOutcome)
        assert fallback(None) == fallback(Exploding()) == fallback() == fallback.outcome


def test_fallback_must_be_a_constant_outcome() -> None:
    with pytest.raises(TypeError, match="ConstantOutcome"):
        DecisionPointSpec("k", "K", ("a.b",), "doc", lambda view: "record", ("1.1",))  # type: ignore[arg-type]


def test_active_servers_follow_toggles_priority_and_defaults() -> None:
    off = dict.fromkeys(REGISTRY.ids(), False)
    for kind in REGISTRY.kinds():
        assert REGISTRY.active_servers(kind, off) == ()
        assert REGISTRY.fallback_applies(kind, off)
        assert REGISTRY.active_servers(kind, {}) == ()  # unset ids take the registered default (off)
    on = dict.fromkeys(REGISTRY.ids(), True)
    assert REGISTRY.active_servers("record_or_map", on) == ("decide.dm_record_map", "decide.mdl")
    assert REGISTRY.active_servers("record_or_map", {"decide.mdl": True}) == ("decide.mdl",)
    assert not REGISTRY.fallback_applies("record_or_map", {"decide.mdl": True})


def test_closed_enum_decision_model_serves_only_with_enum_estimation() -> None:
    kind = "closed_or_open_enum"
    assert REGISTRY.decision_point(kind).serves_only_with == {"decide.dm_closed_enum": "values.enum_estimation"}
    assert REGISTRY.active_servers(kind, {"decide.dm_closed_enum": True}) == ()
    assert REGISTRY.fallback_applies(kind, {"decide.dm_closed_enum": True})  # 15.7: open whatever the model says
    both = {"decide.dm_closed_enum": True, "values.enum_estimation": True}
    assert REGISTRY.active_servers(kind, both) == ("decide.dm_closed_enum", "values.enum_estimation")
    assert REGISTRY.active_servers(kind, {"values.enum_estimation": True}) == ("values.enum_estimation",)


def test_registry_pickles_for_worker_processes() -> None:
    copy = pickle.loads(pickle.dumps(REGISTRY))
    assert copy.ids() == REGISTRY.ids() and copy.kinds() == REGISTRY.kinds()
    assert list(copy.techniques.values()) == list(REGISTRY.techniques.values())
    assert list(copy.decision_points.values()) == list(REGISTRY.decision_points.values())


def test_lookups_name_unregistered_entries() -> None:
    with pytest.raises(KeyError, match=r"unregistered Technique 'nope\.x'; registered: variants\.minhash_lsh"):
        REGISTRY.technique("nope.x")
    with pytest.raises(KeyError, match="unregistered Decision_Point kind"):
        REGISTRY.decision_point("nope")
    with pytest.raises(KeyError, match="does not serve"):
        REGISTRY.rank("values.bounds", "record_or_map")


# ---------------------------------------------------------------------------
# Validation of hand-built specs and registries
# ---------------------------------------------------------------------------


def spec(tid: str, ranks: dict[str, int] | None = None, **fields: object) -> TechniqueSpec:
    """A TechniqueSpec serving the kinds of ``ranks`` at those ranks; ``fields`` override any field."""
    values: dict[str, object] = dict(
        id=tid, title=tid, area="Test", tier="must", default_enabled=False, implemented=False,
        serves=tuple(ranks or {}), priority=dict(ranks or {}), prerequisites=(), superlinear=False,
        sample_param=None, thresholds=(),
    )
    values.update(fields)
    return TechniqueSpec(**values)  # type: ignore[arg-type]


def point(kind: str, served_by: tuple[str, ...], **fields: object) -> DecisionPointSpec:
    return DecisionPointSpec(kind, kind, served_by, "none", ConstantOutcome("none"), ("1.1",), **fields)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("fields", "message"),
    [
        ({"default_enabled": True}, "not implemented"),
        ({"superlinear": True}, "sample-size Resource_Parameter"),
        ({"sample_param": "sample.x"}, "not marked super-linear"),
        ({"superlinear": True, "sample_param": "trees.max_depth"}, r"sample\.\* Resource_Parameter"),
        ({"prerequisites": ("a.x",)}, "itself"),
        ({"tier": "may"}, "tier"),
        ({"thresholds": ("t", "t")}, "duplicate"),
        ({"serves": ("k",), "priority": {"k": 0}}, ">= 1"),
        ({"serves": ("k",), "priority": {}}, "rank exactly"),
        ({"id": "NoDot"}, "Technique id"),
    ],
)
def test_technique_spec_rejects_inconsistent_fields(fields: dict[str, object], message: str) -> None:
    with pytest.raises(ValueError, match=message):
        spec("a.x", **fields)


def test_registry_reports_every_cross_reference_problem() -> None:
    with pytest.raises(RegistryError) as exc:
        Registry(
            [
                spec("a.x", {"k": 2}),  # wrong rank
                spec("a.x"),  # duplicate id
                spec("a.y", {"missing": 1}),  # unregistered kind
                spec("a.z", prerequisites=("a.nope",)),
                spec("a.on", default_enabled=True, implemented=True, prerequisites=("a.y",)),
            ],
            [point("k", ("a.x", "a.ghost")), point("k", ("a.x",))],
        )
    problems = exc.value.problems
    assert "duplicate Technique id 'a.x'" in problems
    assert "duplicate Decision_Point kind 'k'" in problems
    assert any("'a.x' must serve 'k' at rank 1" in p for p in problems)
    assert any("unregistered Technique 'a.ghost'" in p for p in problems)
    assert any("unregistered Decision_Point kind 'missing'" in p for p in problems)
    assert any("unregistered prerequisite 'a.nope'" in p for p in problems)
    assert any("'a.on' is enabled by default but its prerequisite 'a.y' is not" in p for p in problems)


def test_registry_rejects_one_sided_serving_and_cycles() -> None:
    with pytest.raises(RegistryError, match="does not list it in served_by"):
        Registry([spec("a.x", {"k": 1}), spec("a.y", {"k": 1})], [point("k", ("a.x",))])
    with pytest.raises(RegistryError, match=r"prerequisite cycle a\.x -> a\.y -> a\.x"):
        Registry([spec("a.x", prerequisites=("a.y",)), spec("a.y", prerequisites=("a.x",))], [])


def test_decision_point_spec_checks_gates_and_tie_rules() -> None:
    with pytest.raises(ValueError, match="does not serve"):
        point("k", ("a.x",), serves_only_with={"a.y": "a.x"})
    with pytest.raises(ValueError, match="on itself"):
        point("k", ("a.x",), serves_only_with={"a.x": "a.x"})
    with pytest.raises(RegistryError, match="unregistered Technique 'a.g'"):
        Registry([spec("a.x", {"k": 1})], [point("k", ("a.x",), serves_only_with={"a.x": "a.g"})])
    with pytest.raises(ValueError, match="at least one"):
        TieRule((), "doc", ("1.1",))
    with pytest.raises(ValueError, match="at least one Technique"):
        point("k", ())


def test_specs_are_frozen() -> None:
    with pytest.raises(dataclasses.FrozenInstanceError):
        REGISTRY.technique("decide.mdl").implemented = True  # type: ignore[misc]
    with pytest.raises(TypeError):
        REGISTRY.techniques["decide.mdl"] = REGISTRY.technique("decide.mdl")  # type: ignore[index]
