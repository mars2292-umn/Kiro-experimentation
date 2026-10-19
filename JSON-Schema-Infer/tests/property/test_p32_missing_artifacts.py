"""Property 32: Missing artifacts degrade to fallbacks (task 23.3).

For any Model_Bundle with a random subset of its Decision_Models, its
Semantic_Type_Classifier, its Corpus_Index, or its Learned_Thresholds
removed, corrupted (the file's bytes changed, so its SHA-256 no longer
matches the manifest), or set out of range, the run completes and its
schema accepts every document. Each affected Decision_Point uses its
Fallback_Behavior, or the next resolution step for a Decision_Model (MDL,
or enum estimation for Closed-or-Open-Enum), and each substitution is
recorded with its cause: ``missing`` for a removed artifact or threshold,
``checksum`` for a corrupted file, and ``invalid`` for an out-of-range or
non-finite threshold value (20.6, 21.10, 22.14, 24.9).

Inputs
    A complete bundle holds a stub Decision_Model per Judgment_Call kind
    (``tests.models``), stub Semantic_Type_Classifier and Corpus_Index
    artifacts, and every registered Learned_Threshold at an admissible
    value. Each example draws, per artifact and threshold, whether to keep,
    remove, corrupt, or (thresholds only) invalidate it, a planted
    Document_Collection, and the Technique toggles of a run.

**Validates: Requirements 20.6, 21.10, 22.14, 24.9**
"""

# Feature: json-schema-inference, Property 32: Missing artifacts degrade to fallbacks

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any

import pytest
from hypothesis import event, given, settings
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.provenance import FallbackInfo
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import (
    CORPUS_INDEX,
    DECISION_MODEL_TECHNIQUES,
    SEMANTIC_MAPPING,
    SEMANTIC_TYPE_CLASSIFIER,
    THRESHOLDS_FILE,
    BundleArtifact,
    decision_model_artifact,
    load_bundle,
    write_bundle,
)
from jsinfer.pipeline.infer import infer_documents
from tests.models import fixed_models
from tests.strategies import TECHNIQUE_SETTINGS, collections, run_configs

VALIDATOR = Validator()
GOOD_VALUES: dict[str, int | float] = {
    "lsh.keyset_jaccard": 0.6,
    "dbscan.eps": 0.3,
    "dbscan.min_samples": 2,
    "hdbscan.min_cluster_size": 2,
    "cooccur.min_support": 2,
    "cooccur.min_confidence": 0.7,
    "tagged.branch_support": 0.1,
    "exclusion.min_confidence": 0.5,
    "keys.min_confidence": 0.5,
    "ind.min_confidence": 0.5,
    "fd.min_confidence": 0.5,
    "semantic.min_probability": 0.5,
    "iforest.score": 0.6,
    "lof.score": 1.5,
    "lof.n_neighbors": 5,
    "changepoint.penalty": 3.0,
    "changepoint.min_segment": 5,
    "corpus.suggest_min_confidence": 0.5,
}
assert set(GOOD_VALUES) == set(THRESHOLDS)
MODEL_ARTIFACTS = tuple(decision_model_artifact(kind, "synthetic") for kind in DECISION_MODEL_TECHNIQUES)
OTHER_ARTIFACTS = (SEMANTIC_TYPE_CLASSIFIER, SEMANTIC_MAPPING, CORPUS_INDEX)
FATES = ("keep", "remove", "corrupt")
THRESHOLD_FATES = ("keep", "remove", "invalid")
# The Techniques whose artifacts are drawn here; the unimplemented ones cannot be enabled, so their
# substitutions are never recorded by a run (28.11), only by the bundle reader.
ARTIFACT_TECHNIQUE = {
    **{decision_model_artifact(kind, "synthetic"): tid for kind, tid in DECISION_MODEL_TECHNIQUES.items()},
    SEMANTIC_TYPE_CLASSIFIER: "values.semantic_type",
    SEMANTIC_MAPPING: "values.semantic_type",
    CORPUS_INDEX: "corpus.name_suggest",
}


@pytest.fixture(scope="module")
def complete(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """The complete bundle, written once; each example derives a damaged copy of its files."""
    root = tmp_path_factory.mktemp("p32") / "complete"
    artifacts = [BundleArtifact.model(name, model) for name, model in fixed_models().items()]
    artifacts.append(BundleArtifact.model(SEMANTIC_TYPE_CLASSIFIER, {"stub": "classifier"}))
    artifacts.append(BundleArtifact(SEMANTIC_MAPPING, "models/semantic_mapping.json", b'{"email": "email"}'))
    artifacts.append(BundleArtifact(CORPUS_INDEX, "corpus/index.sqlite", b"not really sqlite"))
    records = {
        name: ThresholdRecord.from_spec(THRESHOLDS[name], value=value, objective_value=0.0, n_examples=1, training_data="p32")
        for name, value in GOOD_VALUES.items()
    }
    write_bundle(root, bundle_id="p32-complete", artifacts=artifacts, thresholds=records)
    return root


def invalid_value(name: str) -> Any:
    """A stored value outside the admissible range, or NaN."""
    spec = THRESHOLDS[name]
    low = spec.admissible.low
    if low is not None and (low > 0 or spec.integer):
        return low - 1 if spec.integer else low - 0.5
    return math.nan


def damaged_copy(complete: Path, target: Path, fates: dict[str, str]) -> None:
    """``complete`` with each artifact or threshold kept, removed, corrupted, or invalidated per ``fates``."""
    manifest = json.loads((complete / "manifest.json").read_text(encoding="utf-8"))
    thresholds = json.loads((complete / THRESHOLDS_FILE).read_text(encoding="utf-8"))
    target.mkdir()
    for name, entry in list(manifest["artifacts"].items()):
        fate = fates[name]
        source = complete / entry["path"]
        if fate == "remove":
            del manifest["artifacts"][name]
            continue
        destination = target / entry["path"]
        destination.parent.mkdir(parents=True, exist_ok=True)
        data = source.read_bytes()
        destination.write_bytes(data + b"\0" if fate == "corrupt" else data)
    for name in list(thresholds):
        fate = fates[name]
        if fate == "remove":
            del thresholds[name]
        elif fate == "invalid":
            thresholds[name]["value"] = invalid_value(name)
    (target / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
    (target / THRESHOLDS_FILE).write_text(json.dumps(thresholds), encoding="utf-8")  # NaN as the token
    (target / "feature_spec.json").write_bytes((complete / "feature_spec.json").read_bytes())


@st.composite
def damages(draw: st.DrawFn) -> dict[str, str]:
    fates = {name: draw(st.sampled_from(FATES), label=name) for name in (*MODEL_ARTIFACTS, *OTHER_ARTIFACTS)}
    fates.update({name: draw(st.sampled_from(THRESHOLD_FATES), label=name) for name in GOOD_VALUES})
    return fates


@settings(deadline=None)
@given(fates=damages(), docs=collections(max_docs=10, outliers=False), config=run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base={"workers": 1}), data=st.data())
def test_missing_or_corrupted_artifacts_degrade_to_fallbacks(
    complete: Path, tmp_path_factory: pytest.TempPathFactory, fates: dict[str, str], docs: list[Any], config: RunConfiguration, data: st.DataObject
) -> None:
    target = tmp_path_factory.mktemp("damaged") / "bundle"
    damaged_copy(complete, target, fates)
    flags = {technique_setting(tid): config.techniques[tid] for tid in REGISTRY.ids()}
    run_config = RunConfiguration.resolve({**flags, "workers": 1, "seed": config.seed, "model_bundle": str(target)})
    result = infer_documents(docs, run_config)
    assert result.report.ok
    compiled = VALIDATOR.compile(result.document)
    assert all(VALIDATOR.is_valid(compiled, doc) for doc in docs)

    bundle = load_bundle(target)
    resolved = bundle.resolve(run_config.techniques)
    by_artifact = {(s.artifact, s.fallback.technique): s for s in result.report.substitutions}
    recorded = {(s.artifact, s.fallback.technique): s for s in resolved.substitutions}
    assert set(by_artifact) >= set(recorded), "the run drops a substitution the bundle stage found"
    expected_cause = {"remove": "missing", "corrupt": "checksum", "invalid": "invalid"}
    for name, fate in fates.items():
        if fate == "keep":
            continue
        tid = ARTIFACT_TECHNIQUE.get(name) or THRESHOLDS[name].techniques[0]
        in_effect = run_config.techniques[tid] and REGISTRY.technique(tid).implemented
        gate = next((dp.serves_only_with.get(tid) for dp in REGISTRY.decision_points.values() if tid in dp.serves_only_with), None)
        if gate is not None and not run_config.techniques[gate]:
            in_effect = False
        found = [s for (artifact, technique), s in by_artifact.items() if artifact == name and technique == tid]
        if not in_effect:
            assert not found, f"{name}: substituted although {tid} is not in effect"
            continue
        assert found, f"{name} ({fate}): no substitution recorded for {tid}"
        assert found[0].cause == expected_cause[fate], f"{name}: cause {found[0].cause!r} for {fate}"
        reason = "missing-model" if name in ARTIFACT_TECHNIQUE else "missing-threshold"
        assert found[0].fallback == FallbackInfo(tid, reason)
        event(f"substitution: {fate}")
        # A substituted Decision_Model never resolves a Judgment_Call; its next step does.
        if name in MODEL_ARTIFACTS:
            kind = name.split(".")[1]
            for entry in result.report.decisions:
                if entry.kind == kind:
                    assert entry.resolution.method != "decision-model", f"{kind} resolved by the {fate} model"
                    assert entry.fallback is not None and entry.fallback.technique == tid
    kept_models = [name for name in MODEL_ARTIFACTS if fates[name] == "keep" and run_config.techniques[ARTIFACT_TECHNIQUE[name]]]
    if any(e.resolution.method == "decision-model" for e in result.report.decisions):
        assert kept_models
        event("a kept Decision_Model resolved")
