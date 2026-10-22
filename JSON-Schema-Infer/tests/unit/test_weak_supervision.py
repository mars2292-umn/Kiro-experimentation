"""Tests of weak supervision (task 39; Requirements 23.1–23.9).

The label model on simulated votes, the Labeling_Functions and their
statistics, the leakage filter and manifest handling, and one tiny training
run with weak supervision on the fixture corpus.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
import pytest

from jsinfer.config.thresholds import THRESHOLDS
from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.learn.bundle import decision_model_artifact, label_model_artifact, load_bundle
from jsinfer.learn.features import FEATURE_SPEC
from jsinfer.learn.label_model import ABSTAIN, DawidSkene
from jsinfer.learn.labeling_functions import (
    LABELING_FUNCTIONS,
    conflict_rates,
    coverage,
    fit_lf_thresholds,
    functions_of,
    measured_accuracy,
    vote_matrix,
)
from jsinfer.learn.trainer import Trainer, TrainingConfig
from jsinfer.learn.weak_supervision import RealIdentity, load_unlabeled, slices
from jsinfer.core.determinism import canonical_bytes
from tests.conftest import FIXTURE_MANIFEST, FIXTURE_SPLIT, TINY_TRAINING


def simulated_votes(accuracies, coverages, n=1500, seed=0):
    rng = np.random.default_rng(seed)
    truth = rng.integers(0, 2, n)
    L = np.full((n, len(accuracies)), ABSTAIN)
    for j, (a, c) in enumerate(zip(accuracies, coverages)):
        vote = rng.random(n) < c
        right = rng.random(n) < a
        L[vote, j] = np.where(right[vote], truth[vote], 1 - truth[vote])
    return L, truth


# ---------------------------------------------------------------------------
# Label model (23.3)
# ---------------------------------------------------------------------------


def test_dawid_skene_beats_majority_vote_and_estimates_accuracies() -> None:
    L, truth = simulated_votes([0.9, 0.75, 0.6], [0.9, 0.6, 0.8])
    model = DawidSkene().fit(L, 2)
    proba = model.predict_proba(L)
    voted = ~np.isnan(proba[:, 0])
    assert np.allclose(proba[voted].sum(axis=1), 1.0) and np.all((proba[voted] >= 0) & (proba[voted] <= 1))
    accuracy = float((proba[voted].argmax(axis=1) == truth[voted]).mean())
    assert accuracy > 0.84
    estimated = model.estimated_accuracy(L)
    assert estimated[0] > estimated[1] > estimated[2]


def test_rows_where_every_function_abstains_get_no_label() -> None:
    L = np.array([[0, 1, ABSTAIN], [ABSTAIN, ABSTAIN, ABSTAIN], [1, 1, 1]])
    model = DawidSkene().fit(L, 2)
    proba = model.predict_proba(L)
    assert np.isnan(proba[1]).all() and not np.isnan(proba[[0, 2]]).any()
    empty = DawidSkene().fit(np.full((3, 2), ABSTAIN), 2)
    assert np.isnan(empty.predict_proba(np.full((3, 2), ABSTAIN))).all()
    with pytest.raises(ValueError):
        DawidSkene().fit(np.array([[2, 0]]), 2)  # a vote outside the classes
    with pytest.raises(ValueError):
        DawidSkene().predict_proba(L)  # not fitted


# ---------------------------------------------------------------------------
# Labeling_Functions (23.1, 23.5)
# ---------------------------------------------------------------------------


def test_every_kind_has_three_functions_with_registered_cutoffs() -> None:
    for kind in JUDGMENT_OUTCOMES:
        functions = functions_of(kind)
        assert len(functions) >= 3, kind
        assert any(lf.id.endswith("-mdl") for lf in functions)
    for lf in LABELING_FUNCTIONS:
        assert lf.source and all(name in THRESHOLDS and name.startswith(f"lf.{lf.id}.") for name in lf.thresholds)


def test_the_mdl_function_votes_the_cheaper_outcome_and_abstains_on_a_tie() -> None:
    names = FEATURE_SPEC["record_or_map"]
    rows = np.zeros((3, len(names)))
    rows[:, names.index("normalized_mdl_difference")] = [-0.5, 0.5, 0.0]
    mdl = functions_of("record_or_map")[0]
    assert [mdl.vote(r, {}) for r in rows] == [0, 1, ABSTAIN]


def test_statistics_match_their_definitions() -> None:
    L = np.array([[0, 0, ABSTAIN], [0, 1, 1], [ABSTAIN, 1, 1], [1, ABSTAIN, ABSTAIN]])
    assert coverage(L).tolist() == [0.75, 0.75, 0.5]
    assert conflict_rates(L).tolist() == [1 / 3, 1 / 3, 0.5]
    assert measured_accuracy(L, np.array([0, 0, 1, 0])).tolist() == [2 / 3, 2 / 3, 0.5]


def test_thresholds_are_fitted_to_validation_labels() -> None:
    names = FEATURE_SPEC["closed_or_open_enum"]
    rng = np.random.default_rng(1)
    ratio = rng.random(200)
    rows = np.zeros((200, len(names)))
    rows[:, names.index("s_obs_ratio")] = ratio
    rows[:, names.index("chao1_ratio")] = 1 + ratio
    rows[:, names.index("s_obs")] = 3
    labels = ["closed" if r < 0.3 else "open" for r in ratio]
    records, omitted = fit_lf_thresholds("closed_or_open_enum", rows, labels, training_data="t")
    assert not omitted
    assert abs(records["lf.ce-distinct-ratio.max_ratio"].value - 0.3) < 0.06
    votes = vote_matrix("closed_or_open_enum", rows, {n: r.value for n, r in records.items()})
    assert votes.shape == (200, 4)
    _, omitted = fit_lf_thresholds("closed_or_open_enum", rows[:5], ["open"] * 5, training_data="t")
    assert {name for name, _ in omitted} == {n for lf in functions_of("closed_or_open_enum") for n in lf.thresholds}


# ---------------------------------------------------------------------------
# Unlabeled collections and leakage (23.2, 23.6, 23.7)
# ---------------------------------------------------------------------------


def write_unlabeled(root: Path) -> Path:
    def jsonl(name: str, docs: list[Any]) -> str:
        (root / name).write_text("".join(json.dumps(d) + "\n" for d in docs))
        return name

    users = [{"id": i, "name": f"u{i}", "tags": ["a", "b"][: i % 3]} for i in range(40)]
    maps = [{f"k{j}x{i}": {"v": j} for j in range(i % 4 + 1)} for i in range(40)]
    (root / "bad.jsonl").write_text("not json\n")
    manifest = {"collections": [
        {"id": "users", "path": jsonl("users.jsonl", users + [{"leaked": True}])},
        {"id": "maps", "path": jsonl("maps.jsonl", maps)},
        {"id": "users", "path": "users.jsonl"},
        {"id": "benchmark-set", "path": "users.jsonl"},
        {"id": "garbage", "path": "bad.jsonl"},
        {"id": "gone", "path": "missing.jsonl"},
    ]}
    (root / "manifest.json").write_text(json.dumps(manifest))
    return root / "manifest.json"


def test_leakage_filter_and_manifest_exclusions(tmp_path: Path) -> None:
    identity = RealIdentity(frozenset({"benchmark-set"}), frozenset({canonical_bytes({"leaked": True})}))
    loaded = load_unlabeled(str(write_unlabeled(tmp_path)), identity)
    assert [c.id for c in loaded.collections] == ["maps", "users"]
    users = next(c for c in loaded.collections if c.id == "users")
    assert users.removed == 1 and len(users.documents) == 40 and loaded.removed_documents == 1
    assert {e["id"]: e["reason"].split(" (")[0] for e in loaded.excluded} == {
        "users": "duplicate identifier",
        "benchmark-set": "the identifier of a Real_Dataset",
        "garbage": "no parseable document",
        "gone": loaded.excluded[-1]["reason"],
    }
    cut = slices(users, (2, 5, 100), 2, seed=0)
    assert [sid for sid, _ in cut] == ["users", "users@2#0", "users@2#1", "users@5#0", "users@5#1"]
    assert cut == slices(users, (2, 5, 100), 2, seed=0) and cut != slices(users, (2, 5, 100), 2, seed=1)


# ---------------------------------------------------------------------------
# Training with weak supervision (23.4, 23.5, 23.8)
# ---------------------------------------------------------------------------


def test_training_with_weak_supervision_adds_weak_models_and_statistics(tmp_path: Path) -> None:
    (tmp_path / "unlabeled").mkdir()
    manifest = write_unlabeled(tmp_path / "unlabeled")
    real = tmp_path / "real"
    real.mkdir()
    (real / "docs.jsonl").write_text('{"leaked": true}\n{"other": 1}\n')
    (real / "schema.json").write_text("{}")
    (real / "manifest.json").write_text(json.dumps(
        {"datasets": [{"id": "benchmark-set", "documents": "docs.jsonl", "schema": "schema.json"}]}))
    config = TrainingConfig(
        str(FIXTURE_MANIFEST), str(FIXTURE_SPLIT), **TINY_TRAINING,
        weak_supervision={"manifest": str(manifest), "real_manifest": str(real / "manifest.json"),
                          "slice_sizes": (2, 10), "slices_per_size": 2, "schemastore_datasets": False},
    )
    bundle_dir = tmp_path / "bundle"
    Trainer().run(config, bundle_dir)
    document = json.loads((bundle_dir / "manifest.json").read_text())
    assert [c["id"] for c in document["unlabeled_collections"]] == ["maps", "users"]
    assert document["unlabeled_collections"][0]["slices"] == 4
    assert {e["id"] for e in document["ws_excluded"]} == {"users", "benchmark-set", "garbage", "gone"}
    assert document["ws_removed_docs"] == 1
    stats = document["labeling_functions"]
    assert {s["id"] for s in stats} == {lf.id for lf in LABELING_FUNCTIONS if any(s2["kind"] == lf.kind for s2 in stats)}
    assert all(set(s) == {"id", "kind", "source", "coverage", "conflict", "est_acc", "measured_acc"} for s in stats)
    assert sum(document["ws_examples"].values()) > 0
    bundle = load_bundle(bundle_dir)
    weak = [k for k in JUDGMENT_OUTCOMES if bundle.has_artifact(decision_model_artifact(k, "weak"))]
    assert weak, document.get("omitted_artifacts")
    for kind in weak:
        assert bundle.has_artifact(label_model_artifact(kind))
        assert bundle.load_artifact(decision_model_artifact(kind, "weak")) is not None
    assert any(name.startswith("lf.") for name in bundle.threshold_entries)
