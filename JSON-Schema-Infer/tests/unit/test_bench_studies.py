"""Tests of the ablation study (task 30.1; Requirements 35.3, 35.10)."""

from __future__ import annotations

from pathlib import Path

from jsinfer.bench.baselines.inferrer import InferrerAdapter
from jsinfer.bench.studies import ABLATION_PREFIX, AblatedConfig, AblationResults, ablation_configurations, registry_configurations
from jsinfer.bench.synthetic import SyntheticConfig, SyntheticResult, SyntheticResults
from jsinfer.config.registry import REGISTRY


def test_each_enabled_technique_gets_one_configuration_with_dependents_disabled() -> None:
    states = {"a": True, "b": True, "c": True, "d": False, "e": True}
    prerequisites = {"b": ["a"], "c": ["b"], "d": ["a"], "e": []}
    configs = ablation_configurations(states, prerequisites)
    assert [c.technique for c in configs] == ["a", "b", "c", "e"]
    by = {c.technique: c for c in configs}
    assert by["a"].disabled == ("a", "b", "c")  # d is off already, so it is not recorded
    assert by["b"].disabled == ("b", "c")
    assert by["c"].disabled == ("c",) and by["e"].disabled == ("e",)
    assert by["a"].states == {"a": False, "b": False, "c": False, "d": False, "e": True}
    assert by["a"].settings() == {"techniques.a": False, "techniques.b": False, "techniques.c": False}
    assert by["a"].system == ABLATION_PREFIX + "a" and by["a"].to_json()["disabled"] == ["a", "b", "c"]


def test_registry_configurations_cover_every_default_enabled_technique() -> None:
    configs = registry_configurations()
    enabled = [tid for tid, on in REGISTRY.default_states().items() if on]
    assert [c.technique for c in configs] == enabled
    for config in configs:
        assert config.technique in config.disabled
        for tid, on in config.states.items():
            if on:
                assert not any(p in config.disabled for p in REGISTRY.technique(tid).prerequisites), (config.technique, tid)


def test_differences_pair_schemas_where_both_have_values() -> None:
    cfg = SyntheticConfig(sizes=(5,), held_out=2, seed=3)
    configs = [AblatedConfig("x", ("x",), {"x": False})]
    results = SyntheticResults(cfg)
    for sid, default, ablated in (("s1", 1.0, 0.5), ("s2", 0.8, 0.6), ("s3", 0.9, None), ("s4", 0.7, 0.7)):
        results.results.append(SyntheticResult(sid, 5, "jsinfer", recall=default, rejection=1.0, harmonic=default))
        if ablated is None:
            results.results.append(SyntheticResult(sid, 5, "ablate:x", missing="timeout"))
        else:
            results.results.append(SyntheticResult(sid, 5, "ablate:x", recall=ablated, rejection=1.0, harmonic=ablated))
    study = AblationResults(configs, results, resamples=200)
    test = study.differences("recall", 5)["x"]
    assert test is not None and test.n_pairs == 3
    assert abs(test.mean_difference - ((0.5 - 1.0) + (0.6 - 0.8) + (0.7 - 0.7)) / 3) < 1e-12
    assert test.ci.low <= test.mean_difference <= test.ci.high
    assert study.to_json()["differences"]["recall@5"]["x"]["n_pairs"] == 3


def test_run_ablation_builds_one_inferrer_per_configuration(monkeypatch, tmp_path: Path) -> None:
    import jsinfer.bench.studies as studies

    seen: list[list[str]] = []

    def fake_run_synthetic(manifest, split, systems, cfg, out_root, **kwargs):
        seen.append([s.name for s in systems])
        assert all(isinstance(s, InferrerAdapter) for s in systems)
        assert systems[1].settings == {"workers": 1, "techniques.a": False, "techniques.b": False}
        return SyntheticResults(cfg)

    monkeypatch.setattr(studies, "run_synthetic", fake_run_synthetic)
    configs = [AblatedConfig("a", ("a", "b"), {"a": False, "b": False}), AblatedConfig("b", ("b",), {"a": True, "b": False})]
    study = studies.run_ablation(None, None, InferrerAdapter(settings={"workers": 1}), SyntheticConfig(sizes=(2,), held_out=1), tmp_path, configs=configs)
    assert seen == [["jsinfer", "ablate:a", "ablate:b"]]
    assert study.configs == configs
