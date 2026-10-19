"""Trainer integration tests (task 23.5; Requirements 21.1, 21.2, 21.7, 21.8, 21.9).

``tiny_bundle`` is trained once per session from the fixture corpus
(``tests/fixtures/corpus``, task 23.4) and cached by content hash. The
tests check what the bundle holds, that two runs agree (identical
Learned_Threshold values, identical predicted classes and calibrated
probabilities on a probe set of validation examples, and manifests that
differ only in the bundle id and timestamp, 21.7), that a held-out or
unknown schema is rejected with no bundle written (21.8), and that
``jsinfer train`` drives the same run.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
import pytest

from jsinfer import cli
from jsinfer.config.registry import REGISTRY
from jsinfer.config.thresholds import THRESHOLDS
from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.learn.bundle import (
    DECISION_MODEL_TECHNIQUES,
    FEATURE_SPEC_FILE,
    MANIFEST_FILE,
    THRESHOLDS_FILE,
    decision_model_artifact,
    load_bundle,
    sha256_of,
)
from jsinfer.learn.decision_models import DecisionModel
from jsinfer.learn.features import FEATURE_SPEC, feature_spec_json
from jsinfer.learn.trainer import M2_THRESHOLDS, Trainer, TrainingConfig, TrainingError
from jsinfer.pipeline.infer import infer_documents
from jsinfer.config.run_config import RunConfiguration
from tests.conftest import FIXTURE_MANIFEST, FIXTURE_SPLIT, TINY_TRAINING

ALL_ON = {tid: True for tid in REGISTRY.ids()}


def training_config(**overrides: Any) -> TrainingConfig:
    return TrainingConfig(str(FIXTURE_MANIFEST), str(FIXTURE_SPLIT), **{**TINY_TRAINING, **overrides})  # type: ignore[arg-type]


def manifest_of(bundle: str) -> dict[str, Any]:
    return json.loads((Path(bundle) / MANIFEST_FILE).read_text(encoding="utf-8"))


def test_tiny_bundle_holds_a_decision_model_per_kind_and_the_m2_thresholds(tiny_bundle: str) -> None:
    bundle = load_bundle(tiny_bundle)
    assert bundle.bundle_id.startswith("bundle-") and bundle.feature_spec == feature_spec_json()
    assert sorted(bundle.artifacts) == sorted(decision_model_artifact(kind, "synthetic") for kind in JUDGMENT_OUTCOMES)
    for name, relative in bundle.artifacts.items():
        assert bundle.digests[name] == sha256_of(bundle.root / relative)
        model = bundle.load_artifact(name)
        assert isinstance(model, DecisionModel) and model.feature_dim == len(FEATURE_SPEC[model.kind])
    assert sorted(bundle.threshold_values()) == sorted(M2_THRESHOLDS)
    manifest = manifest_of(tiny_bundle)
    assert manifest["schema_split"] == {"id": "fixture-split", "seed": 0}
    assert len(manifest["schemas"]["train"]) == 14 and len(manifest["schemas"]["val"]) == 12
    assert manifest["excluded_schemas"] == [] and manifest["training_config"]["seed"] == 0
    assert {c["split"] for c in manifest["collections"]} == {"train", "val"} and len(manifest["collections"]) == 52
    assert set(manifest["metrics"]) == set(bundle.artifacts)
    for metrics in manifest["metrics"].values():
        assert set(metrics) == {"n_val", "acc", "macro_f1", "ece_before", "ece_after"} and metrics["n_val"] >= 2
    assert "normalized_mdl_difference" in manifest["feature_signals"][decision_model_artifact("closed_or_open_record", "synthetic")]
    omitted = {o["artifact"]: o["reason"] for o in manifest["omitted_artifacts"]}
    assert "corpus.index" in omitted and "semantic_type" in omitted and "label_model.record_or_map" in omitted
    assert all("not implemented" in reason for reason in omitted.values())
    left_out = {o["name"] for o in manifest["omitted_thresholds"]}
    assert left_out == set(THRESHOLDS) - set(M2_THRESHOLDS)
    assert set(manifest["example_exclusions"]) == set(JUDGMENT_OUTCOMES)
    # The run's Technique states resolve every model (nothing substituted for the Decision_Models).
    resolved = bundle.resolve(ALL_ON)
    assert not any(s.artifact.startswith("dm.") and s.artifact.endswith(".synthetic") for s in resolved.substitutions)


def test_two_training_runs_agree(tiny_bundle: str, tmp_path: Path) -> None:
    again = tmp_path / "again"
    Trainer().run(training_config(), again)
    first, second = load_bundle(tiny_bundle), load_bundle(again)
    assert first.bundle_id != second.bundle_id
    assert first.threshold_values() == second.threshold_values()
    assert (Path(tiny_bundle) / THRESHOLDS_FILE).read_bytes() == (again / THRESHOLDS_FILE).read_bytes()
    assert (Path(tiny_bundle) / FEATURE_SPEC_FILE).read_bytes() == (again / FEATURE_SPEC_FILE).read_bytes()
    a, b = manifest_of(tiny_bundle), manifest_of(str(again))
    for key in ("bundle_id", "created", "artifacts"):
        a.pop(key), b.pop(key)
    assert a == b
    # A probe set drawn with a fixed seed from validation-like vectors gives identical classes and probabilities.
    rng = np.random.default_rng(21)
    for name in first.artifacts:
        one, two = first.load_artifact(name), second.load_artifact(name)
        probe = rng.normal(size=(16, one.feature_dim))
        assert one.predict(probe) == two.predict(probe)
        assert np.array_equal(one.predict_proba(probe), two.predict_proba(probe))


def test_held_out_and_unknown_schemas_are_rejected_without_a_bundle(tmp_path: Path) -> None:
    out = tmp_path / "bundle"
    with pytest.raises(TrainingError, match=r"held-out split: \['heldout/person'\]") as held:
        Trainer().run(training_config(schemas=("train/order", "heldout/person")), out)
    assert held.value.stage == "inputs" and not out.exists()
    with pytest.raises(TrainingError, match="not in the corpus manifest"):
        Trainer().run(training_config(schemas=("train/order", "nope")), out)
    assert not out.exists() and not any(tmp_path.iterdir())


def test_the_train_command_writes_a_bundle_and_reports_failures(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    config = tmp_path / "train.toml"
    config.write_text(
        "\n".join(
            [
                "[training]",
                f'corpus_manifest = "{FIXTURE_MANIFEST}"',
                f'schema_split = "{FIXTURE_SPLIT}"',
                'schemas = ["train/order", "train/user-profile", "val/account", "val/invoice"]',
                "seed = 3",
                "[resources]",
                '"synth.collections_per_schema" = 1',
                '"synth.max_depth" = 2',
                "[generator]",
                "size_min = 6",
                "size_max = 10",
            ]
        ),
        encoding="utf-8",
    )
    out = tmp_path / "out"
    assert cli.main(["train", "-c", str(config), "--out", str(out)]) == cli.EXIT_OK
    assert "wrote Model_Bundle bundle-" in capsys.readouterr().err
    bundle = load_bundle(out)
    assert manifest_of(str(out))["schemas"] == {"train": ["train/order", "train/user-profile"], "val": ["val/account", "val/invoice"]}
    assert manifest_of(str(out))["training_config"]["resources"] == {"synth.collections_per_schema": 1, "synth.max_depth": 2}
    # The bundle serves a run.
    result = infer_documents([{"a": i, "b": [i, "x"]} for i in range(6)], RunConfiguration.resolve({"workers": 1, "model_bundle": str(out)}))
    assert result.report.ok and result.report.run.bundle_id == bundle.bundle_id
    # A held-out schema in the configuration: non-zero exit, no bundle.
    config.write_text(config.read_text(encoding="utf-8").replace('"val/invoice"', '"heldout/person"'), encoding="utf-8")
    assert cli.main(["train", "-c", str(config), "--out", str(tmp_path / "rejected")]) == cli.EXIT_STAGE_ERROR
    assert "held-out split" in capsys.readouterr().err and not (tmp_path / "rejected").exists()
    # An unreadable or invalid configuration is a usage error.
    assert cli.main(["train", "-c", str(tmp_path / "missing.toml"), "--out", str(out)]) == cli.EXIT_USAGE
    config.write_text(f'[training]\ncorpus_manifest = "{FIXTURE_MANIFEST}"\nschema_split = "{FIXTURE_SPLIT}"\nseed = -1\n', encoding="utf-8")
    assert cli.main(["train", "-c", str(config), "--out", str(out)]) == cli.EXIT_USAGE
    assert "seed must be" in capsys.readouterr().err
