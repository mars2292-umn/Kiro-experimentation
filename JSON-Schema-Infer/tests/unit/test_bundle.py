"""Unit tests for the Model_Bundle reader (task 3.4; Requirements 20.6, 21.10, 27.2, 27.11)."""

from __future__ import annotations

import json
import os
from importlib import resources
from pathlib import Path
from typing import Any

import joblib
import pytest

from jsinfer.config.errors import ConfigError
from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord, threshold_spec
from jsinfer.core.determinism import encode_canonical
from jsinfer.core.report import InferenceReport
from jsinfer.learn.bundle import (
    CORPUS_INDEX,
    DECISION_MODEL_TECHNIQUES,
    DEFAULT_MODEL_BUNDLE,
    MANIFEST_FILE,
    SEMANTIC_MAPPING,
    SEMANTIC_TYPE_CLASSIFIER,
    THRESHOLDS_FILE,
    ArtifactError,
    builtin_bundles,
    decision_model_artifact,
    load_bundle,
    open_bundle,
    required_models,
)

ALL_ON = {tid: True for tid in REGISTRY.ids()}
ALL_OFF = {tid: False for tid in REGISTRY.ids()}


def _manifest(artifacts: dict[str, Any] | None = None, **extra: Any) -> dict[str, Any]:
    return {"artifacts": artifacts or {}, "bundle_id": "bundle-test", "format": 1, **extra}


def make_bundle(
    root: Path,
    *,
    manifest: Any = None,
    thresholds: Any = None,
    files: dict[str, bytes] | None = None,
    raw: dict[str, bytes] | None = None,
) -> Path:
    """Write a stub bundle: JSON ``manifest`` and ``thresholds``, extra ``files``, and ``raw`` overrides."""
    root.mkdir(parents=True, exist_ok=True)
    contents = {
        MANIFEST_FILE: json.dumps(_manifest() if manifest is None else manifest).encode(),
        THRESHOLDS_FILE: json.dumps({} if thresholds is None else thresholds).encode(),
        **(files or {}),
        **(raw or {}),
    }
    for name, data in contents.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return root


def record(name: str, value: Any) -> dict[str, Any]:
    spec = threshold_spec(name)
    return ThresholdRecord.from_spec(spec, value=value, objective_value=0.5, n_examples=7, training_data="td-1").to_json()


def triples(resolved: Any) -> list[tuple[str, str, str, str]]:
    return [(s.artifact, s.fallback.technique, s.fallback.reason, s.cause) for s in resolved.substitutions]


# ---------------------------------------------------------------------------
# The shipped default bundle (27.2)


def test_default_identifier_names_the_shipped_empty_bundle() -> None:
    bundle = load_bundle(DEFAULT_MODEL_BUNDLE)
    assert bundle.builtin and bundle.identifier == DEFAULT_MODEL_BUNDLE
    assert bundle.bundle_id == "default"
    assert dict(bundle.artifacts) == {} and dict(bundle.threshold_entries) == {}
    assert bundle.threshold_values() == {}
    assert DEFAULT_MODEL_BUNDLE in builtin_bundles()


def test_shipped_default_is_canonical_package_data() -> None:
    root = resources.files("jsinfer").joinpath("bundles").joinpath("default")
    for name in (MANIFEST_FILE, THRESHOLDS_FILE):
        data = root.joinpath(name).read_bytes()
        assert encode_canonical(json.loads(data)) == data, name
    assert json.loads(root.joinpath(THRESHOLDS_FILE).read_bytes()) == {}


def test_default_run_configuration_opens_the_default_bundle() -> None:
    config = RunConfiguration.resolve({}, None)
    resolved = open_bundle(config.model_bundle, config.techniques)
    assert resolved.bundle_id == "default"
    assert dict(resolved.thresholds) == {} and dict(resolved.models) == {}
    # Every Learned_Threshold read by a default-enabled Technique falls back (20.6).
    expected = sorted(
        (spec.name, tid)
        for spec in THRESHOLDS.values()
        for tid in spec.techniques
        if config.techniques[tid] and REGISTRY.technique(tid).implemented
    )
    found = sorted((s.artifact, s.fallback.technique) for s in resolved.substitutions if s.fallback.reason == "missing-threshold")
    assert found == expected
    assert all(s.cause == "missing" for s in resolved.substitutions)


def test_empty_bundle_records_every_fallback_with_every_technique_enabled() -> None:
    resolved = load_bundle(DEFAULT_MODEL_BUNDLE).resolve(ALL_ON)
    expected = [(spec.name, tid, "missing-threshold") for spec in THRESHOLDS.values() for tid in spec.techniques]
    for kind, tid in DECISION_MODEL_TECHNIQUES.items():
        expected.append((decision_model_artifact(kind, "synthetic"), tid, "missing-model"))
        expected.append((decision_model_artifact(kind, "weak"), "decide.weak_supervision", "missing-model"))
    expected += [(name, "values.semantic_type", "missing-model") for name in (SEMANTIC_TYPE_CLASSIFIER, SEMANTIC_MAPPING)]
    expected += [
        (CORPUS_INDEX, tid, "missing-model") for tid in ("corpus.name_suggest", "corpus.cf_suggest", "corpus.nearest_schema")
    ]
    assert sorted(triples(resolved)) == sorted((*entry, "missing") for entry in expected)
    by_artifact = {(s.artifact, s.fallback.technique): s.applied for s in resolved.substitutions}
    assert by_artifact[("dbscan.eps", "variants.dbscan")] == "Fallback_Behavior of variant_grouping: one Variant"
    assert by_artifact[("dm.record_or_map.synthetic", "decide.dm_record_map")] == "decide.mdl at record_or_map"
    assert by_artifact[("dm.record_or_map.weak", "decide.weak_supervision")] == (
        "Fallback_Behavior of judgment_model: synthetic-only Decision_Model"
    )
    # Substitutions go into the report unchanged and in its canonical order.
    report = InferenceReport()
    for substitution in resolved.substitutions:
        report.record_substitution(substitution)
    assert report.substitutions == resolved.substitutions
    assert InferenceReport.from_json(report.to_json()) == report


def test_nothing_enabled_needs_nothing() -> None:
    resolved = load_bundle(DEFAULT_MODEL_BUNDLE).resolve(ALL_OFF)
    assert resolved.substitutions == () and resolved.problems == ()
    assert required_models(ALL_OFF) == ()


# ---------------------------------------------------------------------------
# Identifiers and paths


def test_shipped_identifier_wins_over_the_working_directory(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    make_bundle(tmp_path / "bundles" / "default", manifest=_manifest(bundle_id="local"))
    monkeypatch.chdir(tmp_path)
    assert load_bundle("bundles/default").bundle_id == "default"
    local = load_bundle("./bundles/default")
    assert (local.bundle_id, local.builtin) == ("local", False)
    assert load_bundle(tmp_path / "bundles" / "default").bundle_id == "local"  # an absolute path is a path


def test_path_bundle_supplies_usable_thresholds(tmp_path: Path) -> None:
    root = make_bundle(
        tmp_path / "b",
        thresholds={"dbscan.eps": record("dbscan.eps", 0.25), "dbscan.min_samples": record("dbscan.min_samples", 3)},
    )
    bundle = load_bundle(root)
    assert bundle.threshold_values() == {"dbscan.eps": 0.25, "dbscan.min_samples": 3}
    resolved = bundle.resolve({**ALL_OFF, "variants.dbscan": True})
    assert resolved.substitutions == ()
    assert (resolved.threshold("dbscan.eps"), resolved.threshold("dbscan.min_samples")) == (0.25, 3)
    assert resolved.bundle_id == "bundle-test"


@pytest.mark.parametrize(
    ("raw_entry", "cause"),
    [
        ('{"value": 1.5}', "invalid"),  # outside (0, 1]
        ('{"value": NaN}', "invalid"),  # non-finite values are invalid thresholds, not unloadable bundles
        ('{"value": Infinity}', "invalid"),
        ('{"value": "0.5"}', "invalid"),
        ('{"value": 0.5}', "invalid"),  # value fine, but not a well-formed record
        ("[0.5]", "invalid"),
        ('{"objective": "grouping_ari"}', "missing"),  # no value
    ],
)
def test_unusable_threshold_entries_become_substitutions(tmp_path: Path, raw_entry: str, cause: str) -> None:
    text = '{"dbscan.eps": %s, "dbscan.min_samples": %s}' % (raw_entry, json.dumps(record("dbscan.min_samples", 2)))
    root = make_bundle(tmp_path / "b", raw={THRESHOLDS_FILE: text.encode()})
    resolved = load_bundle(root).resolve({**ALL_OFF, "variants.dbscan": True})
    assert triples(resolved) == [("dbscan.eps", "variants.dbscan", "missing-threshold", cause)]
    assert resolved.threshold("dbscan.eps") is None and resolved.threshold("dbscan.min_samples") == 2
    assert [p.name for p in resolved.problems] == ["dbscan.eps"]


def test_record_of_a_different_spec_is_invalid(tmp_path: Path) -> None:
    wrong = record("dbscan.eps", 0.5) | {"name": "lsh.keyset_jaccard"}
    root = make_bundle(tmp_path / "b", thresholds={"lsh.keyset_jaccard": wrong})
    resolved = load_bundle(root).resolve({**ALL_OFF, "variants.minhash_lsh": True})
    assert triples(resolved) == [("lsh.keyset_jaccard", "variants.minhash_lsh", "missing-threshold", "invalid")]


def test_thresholds_of_disabled_techniques_are_not_checked(tmp_path: Path) -> None:
    root = make_bundle(tmp_path / "b", thresholds={"dbscan.eps": {"value": 7}, "lf.x.cut": {"value": "ignored"}})
    resolved = load_bundle(root).resolve({**ALL_OFF, "variants.hdbscan": True})
    assert triples(resolved) == [("hdbscan.min_cluster_size", "variants.hdbscan", "missing-threshold", "missing")]


def test_shared_threshold_substitutes_each_technique_in_effect(tmp_path: Path) -> None:
    resolved = load_bundle(make_bundle(tmp_path / "b")).resolve({**ALL_OFF, "corpus.cf_suggest": True})
    assert triples(resolved) == [
        (CORPUS_INDEX, "corpus.cf_suggest", "missing-model", "missing"),
        ("corpus.suggest_min_confidence", "corpus.cf_suggest", "missing-threshold", "missing"),
    ]


# ---------------------------------------------------------------------------
# Unloadable bundles (27.11)


def _bad(name: str, data: bytes) -> dict[str, bytes]:
    return {name: data}


UNLOADABLE: dict[str, tuple[dict[str, Any], str]] = {
    "manifest not JSON": ({"raw": _bad(MANIFEST_FILE, b"{")}, "manifest.json: it is not valid JSON"),
    "manifest not UTF-8": ({"raw": _bad(MANIFEST_FILE, b'{"a": "\xff"}')}, "not valid UTF-8 at byte 7"),
    "manifest array": ({"manifest": []}, "expected an object at the top level, got an array"),
    "manifest NaN": ({"raw": _bad(MANIFEST_FILE, b'{"x": NaN}')}, "NaN is not a JSON value"),
    "manifest repeats a member": (
        {"raw": _bad(MANIFEST_FILE, b'{"format": 1, "format": 1}')},
        "the member 'format' twice",
    ),
    "unsupported format": ({"manifest": _manifest() | {"format": 2}}, "format 2 is not supported"),
    "boolean format": ({"manifest": _manifest() | {"format": True}}, "format must be an integer, got a boolean"),
    "missing members": ({"manifest": {"format": 1}}, "missing members ['artifacts', 'bundle_id']"),
    "empty bundle_id": ({"manifest": _manifest() | {"bundle_id": ""}}, "bundle_id must be a non-empty string"),
    "artifacts not an object": ({"manifest": _manifest() | {"artifacts": []}}, "artifacts must be an object"),
    "bad artifact name": ({"manifest": _manifest({"DM": "models/dm.joblib"})}, "artifact name 'DM'"),
    "artifact path not a string": ({"manifest": _manifest({"semantic_type": 3})}, "must be a path string"),
    "parent segment": ({"manifest": _manifest({"semantic_type": "../x.joblib"})}, "relative path inside the bundle"),
    "absolute path": ({"manifest": _manifest({"semantic_type": "/etc/x.joblib"})}, "relative path inside the bundle"),
    "inner parent segment": ({"manifest": _manifest({"semantic_type": "models/../../x"})}, "relative path inside"),
    "backslash": ({"manifest": _manifest({"semantic_type": "models\\x.joblib"})}, "relative path inside the bundle"),
    "hidden segment": ({"manifest": _manifest({"semantic_type": "models/.x"})}, "relative path inside the bundle"),
    "thresholds not an object": ({"thresholds": []}, "thresholds.json: expected an object at the top level"),
    "thresholds repeat a member": (
        {"raw": _bad(THRESHOLDS_FILE, b'{"dbscan.eps": {}, "dbscan.eps": {}}')},
        "the member 'dbscan.eps' twice",
    ),
}


@pytest.mark.parametrize(("setup", "fragment"), list(UNLOADABLE.values()), ids=list(UNLOADABLE))
def test_unloadable_bundles_raise_config_error(tmp_path: Path, setup: dict[str, Any], fragment: str) -> None:
    root = make_bundle(tmp_path / "b", **setup)
    with pytest.raises(ConfigError) as caught:
        load_bundle(root)
    (problem,) = caught.value.problems
    assert (problem.subject, problem.reason) == (str(root), "bundle")
    assert repr(str(root)) in problem.message and fragment in problem.message, problem.message


@pytest.mark.parametrize("missing", [MANIFEST_FILE, THRESHOLDS_FILE])
def test_bundle_files_are_required(tmp_path: Path, missing: str) -> None:
    root = make_bundle(tmp_path / "b")
    (root / missing).unlink()
    with pytest.raises(ConfigError, match=f"{missing} is missing"):
        load_bundle(root)


@pytest.mark.parametrize(
    ("identifier", "fragment"),
    [
        ("", "cannot be found: the identifier is empty"),
        ("bundles/nonexistent", "neither a shipped bundle (shipped: bundles/default"),
        ("no/such/dir", "nor an existing directory"),
    ],
)
def test_unknown_identifiers_cannot_be_found(identifier: str, fragment: str) -> None:
    with pytest.raises(ConfigError) as caught:
        load_bundle(identifier)
    (problem,) = caught.value.problems
    assert (problem.subject, problem.reason) == (identifier, "bundle")
    assert fragment in problem.message


def test_a_file_is_not_a_bundle(tmp_path: Path) -> None:
    path = tmp_path / "bundle.json"
    path.write_text("{}")
    with pytest.raises(ConfigError, match="is not a directory"):
        load_bundle(path)


def test_every_problem_is_reported_at_once(tmp_path: Path) -> None:
    root = make_bundle(tmp_path / "b", manifest=_manifest() | {"bundle_id": 5, "artifacts": "x"}, thresholds=[1])
    with pytest.raises(ConfigError) as caught:
        load_bundle(root)
    messages = [p.message for p in caught.value.problems]
    assert len(messages) == 3, messages
    assert "bundle_id" in messages[0] and "artifacts" in messages[1] and THRESHOLDS_FILE in messages[2]


@pytest.mark.skipif(not hasattr(os, "symlink"), reason="needs symbolic links")
def test_symlinks_out_of_the_bundle_are_rejected(tmp_path: Path) -> None:
    outside = tmp_path / "secret.joblib"
    joblib.dump({"secret": True}, outside)
    root = make_bundle(tmp_path / "b", manifest=_manifest({"semantic_type": "models/semantic_type.joblib"}))
    (root / "models").mkdir()
    (root / "models" / "semantic_type.joblib").symlink_to(outside)
    with pytest.raises(ConfigError, match=r"artifacts\['semantic_type'\] .* resolves outside the bundle directory"):
        load_bundle(root)
    (root / THRESHOLDS_FILE).unlink()
    (root / THRESHOLDS_FILE).symlink_to(outside)
    with pytest.raises(ConfigError, match="thresholds.json resolves outside the bundle directory"):
        load_bundle(root)


# ---------------------------------------------------------------------------
# Model artifacts (21.10, 22.14, 23.8, 23.9)


def test_models_load_by_suffix_and_weak_models_win(tmp_path: Path) -> None:
    artifacts = {
        "dm.record_or_map.synthetic": "models/dm.record_or_map.synthetic.joblib",
        "dm.record_or_map.weak": "models/dm.record_or_map.weak.joblib",
        "semantic_type": "models/semantic_type.joblib",
        "semantic_mapping": "models/semantic_mapping.json",
        "corpus.index": "corpus/index.sqlite",
    }
    root = make_bundle(
        tmp_path / "b",
        manifest=_manifest(artifacts),
        thresholds={"semantic.min_probability": record("semantic.min_probability", 0.9)},
        files={"models/semantic_mapping.json": b'{"email": "email"}', "corpus/index.sqlite": b"sqlite"},
    )
    for name in ("dm.record_or_map.synthetic", "dm.record_or_map.weak", "semantic_type"):
        joblib.dump({"artifact": name}, root / artifacts[name])
    bundle = load_bundle(root)
    states = {**ALL_OFF, "decide.dm_record_map": True, "values.semantic_type": True, "corpus.name_suggest": True}

    synthetic_only = bundle.resolve(states)
    assert synthetic_only.decision_model("record_or_map") == (
        "dm.record_or_map.synthetic",
        {"artifact": "dm.record_or_map.synthetic"},
    )
    assert "dm.record_or_map.weak" not in synthetic_only.models  # not needed without weak supervision
    assert synthetic_only.model("semantic_type") == {"artifact": "semantic_type"}
    assert synthetic_only.model("semantic_mapping") == {"email": "email"}
    assert synthetic_only.model("corpus.index") == root / "corpus" / "index.sqlite"
    assert synthetic_only.decision_model("tuple_or_list") is None
    # Only the missing threshold of corpus.name_suggest is substituted.
    assert triples(synthetic_only) == [
        ("corpus.suggest_min_confidence", "corpus.name_suggest", "missing-threshold", "missing")
    ]

    weak = bundle.resolve({**states, "decide.weak_supervision": True})
    assert weak.decision_model("record_or_map") == ("dm.record_or_map.weak", {"artifact": "dm.record_or_map.weak"})


@pytest.mark.parametrize(
    ("files", "cause"),
    [
        ({}, "missing"),  # listed, but no file
        ({"models/dm.joblib": b"not a pickle"}, "invalid"),
        ({"models/dm.joblib/inner": b"x"}, "invalid"),  # a directory, not a file
    ],
)
def test_unusable_model_files_become_substitutions(tmp_path: Path, files: dict[str, bytes], cause: str) -> None:
    root = make_bundle(
        tmp_path / "b", manifest=_manifest({"dm.record_or_map.synthetic": "models/dm.joblib"}), files=files
    )
    bundle = load_bundle(root)
    with pytest.raises(ArtifactError) as caught:
        bundle.load_artifact("dm.record_or_map.synthetic")
    assert caught.value.problem.cause == cause
    resolved = bundle.resolve({**ALL_OFF, "decide.dm_record_map": True})
    assert triples(resolved) == [("dm.record_or_map.synthetic", "decide.dm_record_map", "missing-model", cause)]
    assert resolved.decision_model("record_or_map") is None
    # With decide.mdl disabled no serving Technique remains, so the kind's Fallback_Behavior applies.
    assert resolved.substitutions[0].applied == "Fallback_Behavior of record_or_map: Record"


def test_invalid_json_artifact_is_invalid(tmp_path: Path) -> None:
    root = make_bundle(
        tmp_path / "b",
        manifest=_manifest({"semantic_type": "m/s.joblib", "semantic_mapping": "m/map.json"}),
        files={"m/map.json": b"{NaN"},
    )
    joblib.dump({}, root / "m" / "s.joblib")
    resolved = load_bundle(root).resolve({**ALL_OFF, "values.semantic_type": True})
    assert ("semantic_mapping", "values.semantic_type", "missing-model", "invalid") in triples(resolved)
    assert resolved.model("semantic_type") == {}


def test_unlisted_artifact_is_missing() -> None:
    with pytest.raises(ArtifactError, match="lists no such artifact") as caught:
        load_bundle(DEFAULT_MODEL_BUNDLE).load_artifact(CORPUS_INDEX)
    assert caught.value.problem.cause == "missing"


def test_gated_decision_model_is_not_required() -> None:
    # decide.dm_closed_enum serves only while values.enum_estimation is enabled (15.7).
    assert required_models({**ALL_OFF, "decide.dm_closed_enum": True}) == ()
    (requirement,) = required_models({**ALL_OFF, "decide.dm_closed_enum": True, "values.enum_estimation": True})
    assert requirement.artifact == "dm.closed_or_open_enum.synthetic"
    assert requirement.applied == "values.enum_estimation at closed_or_open_enum"


def test_decision_model_techniques_match_the_registry() -> None:
    for kind, tid in DECISION_MODEL_TECHNIQUES.items():
        assert REGISTRY.technique(tid).serves == (kind,)
    assert sorted(DECISION_MODEL_TECHNIQUES.values()) == sorted(t for t in REGISTRY.ids() if t.startswith("decide.dm_"))


def test_resolve_rejects_unregistered_technique_ids() -> None:
    bundle = load_bundle(DEFAULT_MODEL_BUNDLE)
    with pytest.raises(ValueError, match="unregistered Technique ids"):
        bundle.resolve({"decide.nope": True})
    with pytest.raises(TypeError):
        bundle.resolve({"decide.mdl": 1})  # type: ignore[dict-item]
