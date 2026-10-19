"""Stub Decision_Models for tests, and Model_Bundle directories that hold them.

:class:`FixedDecisionModel` satisfies ``decide.engine.DecisionModelLike`` with
fixed calibrated probabilities, so a test can make a Judgment_Call go one
way without training anything. It lives in this importable module so that
``joblib`` can pickle it into a bundle's ``models/`` directory and the
bundle reader can load it back (``tests.models.FixedDecisionModel``).

:data:`FIXED_PROBABILITIES` gives every kind a model that picks the first
class for Closed-or-Open-Record, Tuple-or-List, and Closed-or-Open-Enum
(closed, tuple, closed), the second for Record-or-Map (map), and a tie for
Merge-or-Split (resolved by the MDL_Scorer, 22.12), so the full-run
properties reach every Decision_Model branch of the engine.
"""

from __future__ import annotations

import json
from collections.abc import Mapping
from pathlib import Path
from typing import Any

import joblib
import numpy as np

from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.learn.bundle import THRESHOLDS_FILE, BundleArtifact, decision_model_artifact, write_bundle

__all__ = ["FIXED_PROBABILITIES", "FixedDecisionModel", "fixed_models", "write_model_bundle"]

FIXED_PROBABILITIES: dict[str, tuple[float, float]] = {
    "record_or_map": (0.3, 0.7),
    "closed_or_open_record": (0.8, 0.2),
    "tuple_or_list": (0.6, 0.4),
    "closed_or_open_enum": (0.55, 0.45),
    "merge_or_split": (0.5, 0.5),
}


class FixedDecisionModel:
    """A Decision_Model of ``kind`` that returns ``proba`` for every feature vector; records the vectors it saw."""

    def __init__(
        self, kind: str, proba: tuple[float, float], *, feature_version: str | None = None, feature_dim: int | None = None
    ) -> None:
        if kind not in JUDGMENT_OUTCOMES:
            raise ValueError(f"{kind!r} is not a Judgment_Call kind")
        if len(proba) != 2 or not all(0.0 <= p <= 1.0 for p in proba):
            raise ValueError("proba must be two probabilities")
        self.kind = kind
        self.proba = (float(proba[0]), float(proba[1]))
        # Declared only when given, so the bundle reader's feature-spec check can be exercised (task 23.1).
        if feature_version is not None:
            self.feature_version = feature_version
        if feature_dim is not None:
            self.feature_dim = feature_dim
        self.seen: list[np.ndarray] = []

    @property
    def classes(self) -> tuple[str, str]:
        return JUDGMENT_OUTCOMES[self.kind]

    def predict_proba(self, x: Any) -> np.ndarray:
        matrix = np.asarray(x, dtype=np.float64)
        if matrix.ndim == 1:
            matrix = matrix.reshape(1, -1)
        if matrix.ndim != 2 or not np.all(np.isfinite(matrix)):
            raise ValueError(f"expected one or more finite feature vectors, got shape {matrix.shape}")
        self.seen.append(np.array(matrix))
        return np.tile(np.asarray(self.proba, dtype=np.float64), (matrix.shape[0], 1))

    def __getstate__(self) -> dict[str, Any]:
        state = {"kind": self.kind, "proba": self.proba}  # the seen vectors stay in the process that saw them
        for name in ("feature_version", "feature_dim"):
            if hasattr(self, name):
                state[name] = getattr(self, name)
        return state

    def __setstate__(self, state: dict[str, Any]) -> None:
        self.__dict__.update(state)
        self.seen = []


def fixed_models(
    kinds: Mapping[str, tuple[float, float]] | None = None, source: str = "synthetic"
) -> dict[str, FixedDecisionModel]:
    """Artifact name -> :class:`FixedDecisionModel` for ``kinds`` (default :data:`FIXED_PROBABILITIES`)."""
    chosen = FIXED_PROBABILITIES if kinds is None else kinds
    return {decision_model_artifact(kind, source): FixedDecisionModel(kind, proba) for kind, proba in chosen.items()}  # type: ignore[arg-type]


def write_model_bundle(
    directory: Path,
    bundle_id: str,
    models: Mapping[str, Any],
    thresholds: Mapping[str, Any] | None = None,
) -> str:
    """Write a Model_Bundle holding ``models`` (artifact name -> picklable model) and raw threshold ``thresholds``.

    The models go through the real writer (digests, feature spec); the raw
    threshold entries are written as given, NaN tokens included.
    """
    write_bundle(directory, bundle_id=bundle_id, artifacts=[BundleArtifact.model(n, models[n]) for n in sorted(models)])
    (directory / THRESHOLDS_FILE).write_text(json.dumps(dict(thresholds or {})), encoding="utf-8")
    return str(directory)
