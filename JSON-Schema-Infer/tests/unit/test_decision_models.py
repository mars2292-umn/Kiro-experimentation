"""Unit tests for Decision_Model training and calibration (task 22.4; Requirements 21.3–21.5, 22.11)."""

from __future__ import annotations

import joblib
import numpy as np
import pytest

from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.learn.decision_models import (
    DecisionModel,
    OmittedModel,
    TrainedModel,
    class_counts,
    expected_calibration_error,
    feature_signal,
    macro_f1,
    train_decision_model,
)
from jsinfer.learn.features import FEATURE_SPEC


def examples(kind: str, n: int, seed: int, noise: float = 0.0) -> tuple[np.ndarray, list[str]]:
    """``n`` separable examples: the label follows the sign of the MDL-difference feature (plus ``noise``)."""
    dim = len(FEATURE_SPEC[kind])
    rng = np.random.default_rng(seed)
    x = rng.normal(size=(n, dim))
    index = FEATURE_SPEC[kind].index("normalized_mdl_difference")
    first, second = JUDGMENT_OUTCOMES[kind]
    signal = x[:, index] + noise * rng.normal(size=n)
    return x, [first if value < 0 else second for value in signal.tolist()]


def test_a_separable_kind_trains_calibrates_and_predicts_in_class_order() -> None:
    kind = "closed_or_open_record"
    x_train, y_train = examples(kind, 300, 1)
    x_val, y_val = examples(kind, 120, 2)
    trained = train_decision_model(kind, x_train, y_train, x_val, y_val, seed=0, signal_features=("normalized_mdl_difference", "depth"))
    assert isinstance(trained, TrainedModel)
    model = trained.model
    assert model.classes == ("closed", "open") and model.feature_dim == len(FEATURE_SPEC[kind])
    proba = model.predict_proba(x_val)
    assert proba.shape == (120, 2) and np.allclose(proba.sum(axis=1), 1.0) and np.all((proba >= 0) & (proba <= 1))
    assert model.predict(x_val[:1]) == [JUDGMENT_OUTCOMES[kind][int(np.argmax(proba[0]))]]
    assert trained.metrics.n_val == 120 and trained.metrics.accuracy > 0.9 and trained.metrics.macro_f1 > 0.9
    assert 0.0 <= trained.metrics.ece_after <= 1.0 and 0.0 <= trained.metrics.ece_before <= 1.0
    assert set(trained.metrics.to_json()) == {"n_val", "acc", "macro_f1", "ece_before", "ece_after"}
    # The MDL-difference feature carries the signal; an unused feature does not.
    signal = trained.signals["normalized_mdl_difference"]
    assert signal.importance > 0.2 and signal.spread > 0.5 and signal.to_json()["feature"] == "normalized_mdl_difference"
    assert trained.signals["depth"].importance <= 0.05
    assert repr(model).startswith("DecisionModel('closed_or_open_record'")


def test_training_is_deterministic_and_the_model_survives_joblib() -> None:
    kind = "record_or_map"
    x_train, y_train = examples(kind, 200, 3, noise=0.3)
    x_val, y_val = examples(kind, 80, 4, noise=0.3)
    first = train_decision_model(kind, x_train, y_train, x_val, y_val, seed=5)
    second = train_decision_model(kind, x_train, y_train, x_val, y_val, seed=5)
    assert isinstance(first, TrainedModel) and isinstance(second, TrainedModel)
    assert np.array_equal(first.model.predict_proba(x_val), second.model.predict_proba(x_val))
    assert first.metrics == second.metrics
    other = train_decision_model(kind, x_train, y_train, x_val, y_val, seed=6)
    assert isinstance(other, TrainedModel)  # another seed trains another model, still a valid one
    restored = joblib.load(joblib_dump(first.model))
    assert isinstance(restored, DecisionModel) and restored.classes == first.model.classes
    assert np.array_equal(restored.predict_proba(x_val), first.model.predict_proba(x_val))


def joblib_dump(model: DecisionModel) -> str:
    import tempfile

    path = tempfile.NamedTemporaryFile(suffix=".joblib", delete=False).name
    joblib.dump(model, path)
    return path


@pytest.mark.parametrize(
    ("train_labels", "val_labels", "reason"),
    [
        ([], [], "no training example"),
        (["tuple"] * 10, ["tuple", "list"] * 5, "training examples of one class only"),
        (["tuple", "list"] * 5, [], "no validation example"),
        (["tuple", "list"] * 5, ["list"] * 10, "validation examples of one class only or fewer than two"),
        (["tuple", "list"] * 5, ["list"], "validation examples of one class only or fewer than two"),
    ],
)
def test_a_kind_without_both_classes_in_a_split_is_omitted_with_counts(train_labels: list[str], val_labels: list[str], reason: str) -> None:
    kind = "tuple_or_list"
    dim = len(FEATURE_SPEC[kind])
    rng = np.random.default_rng(0)
    omitted = train_decision_model(
        kind, rng.normal(size=(len(train_labels), dim)), train_labels, rng.normal(size=(len(val_labels), dim)), val_labels
    )
    assert isinstance(omitted, OmittedModel)
    assert omitted.kind == kind and omitted.reason == reason
    assert omitted.class_counts == {"train": class_counts(train_labels), "val": class_counts(val_labels)}
    assert omitted.to_json()["class_counts"]["train"] == class_counts(train_labels)


def test_bad_inputs_are_rejected() -> None:
    kind = "merge_or_split"
    x, y = examples(kind, 20, 0)
    with pytest.raises(ValueError, match="not classes"):
        train_decision_model(kind, x, ["bogus"] * 20, x, y)
    with pytest.raises(ValueError, match="columns"):
        train_decision_model(kind, x[:, :3], y, x, y)
    with pytest.raises(ValueError):
        train_decision_model("bound", x, y, x, y)
    trained = train_decision_model(kind, *examples(kind, 200, 1), *examples(kind, 80, 2))
    assert isinstance(trained, TrainedModel)
    with pytest.raises(ValueError, match="2-d|columns"):
        trained.model.predict_proba(np.zeros(3))
    with pytest.raises(ValueError, match="finite"):
        trained.model.predict_proba(np.full((1, trained.model.feature_dim), np.nan))
    with pytest.raises(ValueError):
        feature_signal(trained.model, x, y, "no_such_feature")


def test_expected_calibration_error_and_macro_f1_match_hand_computations() -> None:
    confidence = [0.9, 0.8, 0.3, 0.2]
    correct = [True, False, False, True]
    # Two bins hold examples: [0.8, 0.9] mean 0.85 vs accuracy 0.5, [0.2, 0.3] mean 0.25 vs accuracy 0.5.
    assert expected_calibration_error(confidence, correct, bins=2) == pytest.approx((2 * 0.35 + 2 * 0.25) / 4)
    assert expected_calibration_error([], [], bins=5) == 0.0
    assert expected_calibration_error([1.0], [True], bins=10) == 0.0
    with pytest.raises(ValueError):
        expected_calibration_error([1.5], [True])
    with pytest.raises(ValueError):
        expected_calibration_error([0.5], [True], bins=0)
    truth = ["a", "a", "b", "b"]
    assert macro_f1(truth, truth, ("a", "b")) == 1.0
    assert macro_f1(truth, ["a", "a", "a", "a"], ("a", "b")) == pytest.approx((2 / 3) / 2)
    assert macro_f1(truth, ["b", "b", "a", "a"], ("a", "b")) == 0.0
    assert class_counts(["b", "a", "b"]) == {"a": 1, "b": 2}
