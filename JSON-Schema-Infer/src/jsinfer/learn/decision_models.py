"""Decision_Models: calibrated gradient-boosted classifiers per Judgment_Call kind (Requirements 21.3–21.5, 22.11).

:func:`train_decision_model` fits a
``HistGradientBoostingClassifier(random_state=…, early_stopping=False)`` on
the training-split examples of one kind, calibrates it with
``CalibratedClassifierCV(FrozenEstimator(m), method="sigmoid")`` on the
validation-split examples (21.3, 21.4), and wraps the result in a
:class:`DecisionModel`, the artifact ``dm.<kind>.<source>`` of the
Model_Bundle. The wrapper satisfies ``decide.engine.DecisionModelLike``:
``classes`` are the kind's two outcome labels in
:data:`~jsinfer.decide.engine.JUDGMENT_OUTCOMES` order and ``predict_proba``
returns one calibrated probability per class in that order, whatever order
scikit-learn sorted the labels into. It also records the featurizer version
and dimension it was trained with, so a feature vector of another shape is
refused.

Omission (21.9, 22.11)
    A kind whose training examples hold one class only, or whose validation
    examples hold one class only or fewer than two examples, gets no model:
    :func:`train_decision_model` returns an :class:`OmittedModel` with the
    reason and the per-class example counts of both splits, which the
    training manifest records.

Metrics (21.5)
    :class:`ModelMetrics` holds, on the validation split, the example count,
    accuracy, macro-F1, and the expected calibration error before and after
    calibration, with :func:`expected_calibration_error` the count-weighted
    mean absolute gap between mean Confidence and accuracy over the
    non-empty of ``bins`` equal-width Confidence bins on [0, 1]
    (``ece.bins``; design, Evaluation). Confidence is the probability of the
    predicted class.

Feature signal (task 22.4)
    :func:`feature_signal` reports whether one feature carries signal on the
    validation examples: its spread (standard deviation) and its permutation
    importance (the mean accuracy drop over seeded shuffles of that column).
    The Trainer records it for the normalized MDL-difference feature of
    Closed-or-Open-Record (design, Open Decisions).

Determinism (21.7)
    Fitting and prediction run under ``threadpoolctl`` with one thread, the
    random state comes from the run seed through ``core.determinism``, and
    the model has no early stopping, so two training runs give identical
    predictions.
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any, Final

import warnings

import numpy as np
from sklearn.calibration import CalibratedClassifierCV
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.frozen import FrozenEstimator
from threadpoolctl import threadpool_limits

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import random_state, rng
from jsinfer.core.jsonvalue import JSON
from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.learn.features import FEATURE_SPEC, FEATURE_VERSION

__all__ = [
    "DEFAULT_ECE_BINS",
    "DecisionModel",
    "FeatureSignal",
    "ModelMetrics",
    "OmittedModel",
    "TrainedModel",
    "class_counts",
    "expected_calibration_error",
    "feature_signal",
    "macro_f1",
    "train_decision_model",
]

DEFAULT_ECE_BINS: Final[int] = int(RESOURCE_PARAMETERS["ece.bins"].default)  # type: ignore[arg-type]
_PERMUTATION_REPEATS: Final = 5


# ---------------------------------------------------------------------------
# Metrics
# ---------------------------------------------------------------------------


def _labels(y: Sequence[str], what: str) -> np.ndarray:
    out = np.asarray(list(y), dtype=object)
    if out.ndim != 1:
        raise ValueError(f"{what} labels must be one-dimensional")
    if not all(isinstance(label, str) for label in out.tolist()):
        raise TypeError(f"{what} labels must be str")
    return out


def _matrix(x: Any, what: str, dim: int | None = None) -> np.ndarray:
    out = np.asarray(x, dtype=np.float64)
    if out.ndim == 1 and dim is not None and out.shape[0] == dim:
        out = out.reshape(1, -1)
    if out.ndim != 2:
        raise ValueError(f"{what} features must be a 2-d array, got shape {out.shape}")
    if dim is not None and out.shape[1] != dim:
        raise ValueError(f"{what} features must have {dim} columns, got {out.shape[1]}")
    if not np.all(np.isfinite(out)):
        raise ValueError(f"{what} features must be finite")
    return out


def class_counts(y: Sequence[str]) -> dict[str, int]:
    """Examples per label, in code-point order of the labels."""
    counts = Counter(_labels(y, "the").tolist())
    return {label: counts[label] for label in sorted(counts)}


def macro_f1(y_true: Sequence[str], y_pred: Sequence[str], classes: Sequence[str]) -> float:
    """The unweighted mean F1 over ``classes`` (a class with no true or predicted example scores 0)."""
    true = _labels(y_true, "true")
    pred = _labels(y_pred, "predicted")
    if true.shape != pred.shape:
        raise ValueError("y_true and y_pred must have the same length")
    scores: list[float] = []
    for label in classes:
        tp = int(np.sum((true == label) & (pred == label)))
        fp = int(np.sum((true != label) & (pred == label)))
        fn = int(np.sum((true == label) & (pred != label)))
        scores.append(2 * tp / (2 * tp + fp + fn) if (2 * tp + fp + fn) else 0.0)
    return float(np.mean(scores)) if scores else 0.0


def expected_calibration_error(confidence: Sequence[float], correct: Sequence[bool], bins: int = DEFAULT_ECE_BINS) -> float:
    """The count-weighted mean |mean Confidence − accuracy| over the non-empty of ``bins`` equal-width bins."""
    if isinstance(bins, bool) or not isinstance(bins, (int, np.integer)) or bins < 1:
        raise ValueError(f"bins must be a positive integer, got {bins!r}")
    conf = np.asarray(confidence, dtype=np.float64)
    hit = np.asarray(correct, dtype=bool)
    if conf.shape != hit.shape or conf.ndim != 1:
        raise ValueError("confidence and correct must be one-dimensional and of the same length")
    if not len(conf):
        return 0.0
    if np.any((conf < 0.0) | (conf > 1.0)):
        raise ValueError("confidences must lie in [0, 1]")
    index = np.minimum((conf * bins).astype(np.int64), bins - 1)
    total = 0.0
    for b in range(int(bins)):
        mask = index == b
        count = int(mask.sum())
        if not count:
            continue
        total += count * abs(float(conf[mask].mean()) - float(hit[mask].mean()))
    return total / len(conf)


@dataclass(frozen=True)
class ModelMetrics:
    """Validation-split metrics of one Decision_Model (21.5)."""

    n_val: int
    accuracy: float
    macro_f1: float
    ece_before: float
    ece_after: float

    def to_json(self) -> dict[str, JSON]:
        return {
            "n_val": self.n_val,
            "acc": self.accuracy,
            "macro_f1": self.macro_f1,
            "ece_before": self.ece_before,
            "ece_after": self.ece_after,
        }


@dataclass(frozen=True)
class FeatureSignal:
    """Whether one feature carries signal on the validation examples (see the module docstring)."""

    feature: str
    index: int
    spread: float
    importance: float

    def to_json(self) -> dict[str, JSON]:
        return {"feature": self.feature, "index": self.index, "spread": self.spread, "importance": self.importance}


@dataclass(frozen=True)
class OmittedModel:
    """A Decision_Model left out of the bundle, with the reason and the per-class counts (21.9, 22.11)."""

    kind: str
    reason: str
    class_counts: Mapping[str, Mapping[str, int]]  # split -> label -> count

    def to_json(self) -> dict[str, JSON]:
        return {
            "kind": self.kind,
            "reason": self.reason,
            "class_counts": {split: dict(counts) for split, counts in self.class_counts.items()},
        }


# ---------------------------------------------------------------------------
# The model
# ---------------------------------------------------------------------------


class DecisionModel:
    """A calibrated Decision_Model of one Judgment_Call kind (see the module docstring)."""

    def __init__(
        self,
        kind: str,
        estimator: Any,
        *,
        trained_with: str = "synthetic",
        feature_version: str = FEATURE_VERSION,
        feature_dim: int | None = None,
    ) -> None:
        if kind not in JUDGMENT_OUTCOMES:
            raise ValueError(f"{kind!r} is not a Judgment_Call kind; the kinds are {list(JUDGMENT_OUTCOMES)}")
        if trained_with not in ("synthetic", "synthetic+weak"):
            raise ValueError(f"trained_with must be 'synthetic' or 'synthetic+weak', got {trained_with!r}")
        if not hasattr(estimator, "predict_proba") or not hasattr(estimator, "classes_"):
            raise TypeError("estimator must be a fitted scikit-learn classifier with predict_proba and classes_")
        self.kind = kind
        self.estimator = estimator
        self.trained_with = trained_with
        self.feature_version = str(feature_version)
        self.feature_dim = int(feature_dim) if feature_dim is not None else len(FEATURE_SPEC[kind])
        learned = [str(label) for label in np.asarray(estimator.classes_).tolist()]
        if sorted(learned) != sorted(JUDGMENT_OUTCOMES[kind]):
            raise ValueError(f"the estimator predicts {learned!r}, not the classes {list(JUDGMENT_OUTCOMES[kind])!r}")
        self._columns = [learned.index(label) for label in JUDGMENT_OUTCOMES[kind]]

    @property
    def classes(self) -> tuple[str, str]:
        """The outcome labels, in the column order of :meth:`predict_proba`."""
        return JUDGMENT_OUTCOMES[self.kind]

    def predict_proba(self, x: Any) -> np.ndarray:
        """Calibrated probabilities, one row per feature vector, columns in ``classes`` order."""
        matrix = _matrix(x, "the", self.feature_dim)
        with threadpool_limits(limits=1):
            proba = np.asarray(self.estimator.predict_proba(matrix), dtype=np.float64)
        return proba[:, self._columns]

    def predict(self, x: Any) -> list[str]:
        """The class with the higher probability per row (the first class on a tie, as the engine would start)."""
        proba = self.predict_proba(x)
        return [self.classes[int(np.argmax(row))] for row in proba]

    def __repr__(self) -> str:
        return f"DecisionModel({self.kind!r}, trained_with={self.trained_with!r}, features=v{self.feature_version}/{self.feature_dim})"


@dataclass(frozen=True)
class TrainedModel:
    """A trained Decision_Model with its validation metrics and feature signals."""

    model: DecisionModel
    metrics: ModelMetrics
    signals: Mapping[str, FeatureSignal] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------


def _accuracy(model: Any, x: np.ndarray, y: np.ndarray, columns: Sequence[str]) -> tuple[float, np.ndarray, np.ndarray]:
    """Accuracy, the per-row confidence, and the per-row correctness of ``model`` on ``x``."""
    with threadpool_limits(limits=1):
        proba = np.asarray(model.predict_proba(x), dtype=np.float64)
    learned = [str(label) for label in np.asarray(model.classes_).tolist()]
    predicted = np.asarray([learned[int(i)] for i in np.argmax(proba, axis=1)], dtype=object)
    confidence = proba.max(axis=1)
    correct = predicted == y
    return float(correct.mean()), confidence, correct


def feature_signal(model: DecisionModel, x: Any, y: Sequence[str], feature: str, seed: int = 0) -> FeatureSignal:
    """The spread and permutation importance of ``feature`` for ``model`` on the examples ``(x, y)``."""
    names = FEATURE_SPEC[model.kind]
    if feature not in names:
        raise ValueError(f"{feature!r} is not a {model.kind} feature")
    index = names.index(feature)
    matrix = _matrix(x, "the", model.feature_dim)
    labels = _labels(y, "the")
    if len(labels) != matrix.shape[0]:
        raise ValueError("x and y must have the same length")
    spread = float(matrix[:, index].std()) if len(labels) else 0.0
    if not len(labels):
        return FeatureSignal(feature, index, spread, 0.0)
    baseline = float(np.mean(np.asarray(model.predict(matrix), dtype=object) == labels))
    stream = rng("dm.permutation", model.kind, feature, seed=seed)
    drops: list[float] = []
    for _ in range(_PERMUTATION_REPEATS):
        shuffled = matrix.copy()
        shuffled[:, index] = stream.permutation(shuffled[:, index])
        accuracy = float(np.mean(np.asarray(model.predict(shuffled), dtype=object) == labels))
        drops.append(baseline - accuracy)
    return FeatureSignal(feature, index, spread, float(np.mean(drops)))


def train_decision_model(
    kind: str,
    x_train: Any,
    y_train: Sequence[str],
    x_val: Any,
    y_val: Sequence[str],
    *,
    seed: int = 0,
    ece_bins: int = DEFAULT_ECE_BINS,
    trained_with: str = "synthetic",
    signal_features: Sequence[str] = (),
) -> TrainedModel | OmittedModel:
    """Fit, calibrate, and evaluate the Decision_Model of ``kind`` (see the module docstring).

    ``x_train``/``y_train`` are the training-split examples and
    ``x_val``/``y_val`` the validation-split examples (features in
    ``FEATURE_SPEC[kind]`` order, labels among the kind's classes).
    ``signal_features`` names the features whose :func:`feature_signal` is
    reported. Returns an :class:`OmittedModel` when a split cannot support
    the model (21.9, 22.11).
    """
    if kind not in JUDGMENT_OUTCOMES:
        raise ValueError(f"{kind!r} is not a Judgment_Call kind; the kinds are {list(JUDGMENT_OUTCOMES)}")
    classes = JUDGMENT_OUTCOMES[kind]
    dim = len(FEATURE_SPEC[kind])
    train_y = _labels(y_train, "training")
    val_y = _labels(y_val, "validation")
    train_x = _matrix(x_train, "training", dim) if len(train_y) else np.zeros((0, dim))
    val_x = _matrix(x_val, "validation", dim) if len(val_y) else np.zeros((0, dim))
    if train_x.shape[0] != len(train_y) or val_x.shape[0] != len(val_y):
        raise ValueError("features and labels must have the same length")
    for split, labels in (("train", train_y), ("val", val_y)):
        unknown = sorted({label for label in labels.tolist() if label not in classes})
        if unknown:
            raise ValueError(f"{split} labels {unknown!r} are not classes of {kind!r} ({list(classes)})")
    counts = {"train": class_counts(train_y.tolist()), "val": class_counts(val_y.tolist())}
    if len(counts["train"]) < 2:
        reason = "no training example" if not len(train_y) else "training examples of one class only"
        return OmittedModel(kind, reason, counts)
    if len(val_y) < 2 or len(counts["val"]) < 2:
        reason = "no validation example" if not len(val_y) else "validation examples of one class only or fewer than two"
        return OmittedModel(kind, reason, counts)

    base = HistGradientBoostingClassifier(random_state=random_state("dm", kind, seed=seed), early_stopping=False)
    with threadpool_limits(limits=1), warnings.catch_warnings():
        # CalibratedClassifierCV still inspects its (unused) cross-validation splits for a frozen estimator.
        warnings.filterwarnings("ignore", message="The least populated class", category=UserWarning)
        base.fit(train_x, train_y)
        # cv is ignored for a frozen estimator, but still validated against the sample count; 2 always fits.
        calibrated = CalibratedClassifierCV(FrozenEstimator(base), method="sigmoid", cv=2)
        calibrated.fit(val_x, val_y)
    accuracy_before, confidence_before, correct_before = _accuracy(base, val_x, val_y, classes)
    accuracy, confidence_after, correct_after = _accuracy(calibrated, val_x, val_y, classes)
    model = DecisionModel(kind, calibrated, trained_with=trained_with, feature_dim=dim)
    predicted = model.predict(val_x)
    metrics = ModelMetrics(
        n_val=int(len(val_y)),
        accuracy=float(np.mean(np.asarray(predicted, dtype=object) == val_y)),
        macro_f1=macro_f1(val_y.tolist(), predicted, classes),
        ece_before=expected_calibration_error(confidence_before, correct_before, ece_bins),
        ece_after=expected_calibration_error(confidence_after, correct_after, ece_bins),
    )
    del accuracy_before, accuracy  # the reported accuracy is that of the calibrated model's argmax
    signals = {name: feature_signal(model, val_x, val_y.tolist(), name, seed) for name in signal_features}
    return TrainedModel(model, metrics, signals)
