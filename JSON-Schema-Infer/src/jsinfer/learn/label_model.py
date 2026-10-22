"""The Dawid–Skene label model of weak supervision (task 39.2; Requirement 23.3).

:class:`DawidSkene` combines the votes of the Labeling_Functions of one
Judgment_Call kind into one probabilistic label per example. Its input is a
vote matrix ``L`` of shape (examples, Labeling_Functions) whose entries are a
class index or :data:`ABSTAIN` (-1); it reads no ground-truth label.

Model
    Each example has a latent true class drawn from the class prior ``π``;
    given the true class ``c``, Labeling_Function ``j`` votes class ``k``
    with probability ``θ_j[c, k]``, independently of the other functions
    (abstentions carry no information and are skipped). The posterior of an
    example is proportional to ``π_c · Π_j θ_j[c, L_ij]`` over its
    non-abstaining votes.

Fitting (EM)
    The posteriors start from majority vote: each example's vote counts per
    class, with Laplace smoothing, normalized. The M-step re-estimates ``π``
    and every ``θ_j`` from the posterior-weighted vote counts with Laplace
    smoothing (one pseudo-count per cell), the E-step recomputes the
    posteriors in log space, and iteration stops when no posterior moves by
    more than ``tol`` or after ``max_iter`` rounds (the Resource_Parameters
    ``ds.tol`` and ``ds.max_iter``). Examples where every Labeling_Function
    abstains take no part in fitting.

Outputs
    :meth:`DawidSkene.predict_proba` returns one row per example that sums
    to 1, and a row of NaN where every Labeling_Function abstains (no
    label, 23.3). :meth:`DawidSkene.estimated_accuracy` is, per
    Labeling_Function, the mean posterior probability of the class it voted
    over the examples it voted on (23.5); NaN for a function that never
    votes.

Determinism
    Everything is closed-form arithmetic in a fixed order, with no random
    initialization, so the fit depends only on ``L``.
"""

from __future__ import annotations

from typing import Final

import numpy as np

__all__ = ["ABSTAIN", "DawidSkene"]

ABSTAIN: Final = -1


def _votes(L: np.ndarray, n_classes: int, n_lfs: int | None = None) -> np.ndarray:
    matrix = np.asarray(L)
    if matrix.ndim != 2:
        raise ValueError(f"the vote matrix must be 2-d (examples × Labeling_Functions), got shape {matrix.shape}")
    if not np.issubdtype(matrix.dtype, np.integer):
        if matrix.size and not np.all(np.equal(np.mod(matrix, 1), 0)):
            raise ValueError("votes must be class indices or -1 (abstain)")
        matrix = matrix.astype(np.int64)
    if matrix.size and (matrix.min() < ABSTAIN or matrix.max() >= n_classes):
        raise ValueError(f"votes must lie in -1 … {n_classes - 1}")
    if n_lfs is not None and matrix.shape[1] != n_lfs:
        raise ValueError(f"the model was fitted on {n_lfs} Labeling_Functions, got {matrix.shape[1]}")
    return matrix.astype(np.int64, copy=False)


class DawidSkene:
    """The Dawid–Skene label model (see the module docstring)."""

    def __init__(self, max_iter: int = 100, tol: float = 1e-6) -> None:
        if isinstance(max_iter, bool) or not isinstance(max_iter, int) or max_iter < 1:
            raise ValueError(f"max_iter must be a positive int, got {max_iter!r}")
        if not isinstance(tol, (int, float)) or tol < 0:
            raise ValueError(f"tol must be a non-negative number, got {tol!r}")
        self.max_iter = max_iter
        self.tol = float(tol)
        self.n_classes: int | None = None
        self.prior: np.ndarray | None = None  # (classes,)
        self.confusion: np.ndarray | None = None  # (LFs, classes, classes): row c = P(vote | true c)
        self.iterations = 0

    # -- fitting -------------------------------------------------------------

    def fit(self, L: np.ndarray, n_classes: int) -> DawidSkene:
        """Fit the prior and the confusion matrices on the votes ``L`` (see the module docstring)."""
        if isinstance(n_classes, bool) or not isinstance(n_classes, int) or n_classes < 2:
            raise ValueError(f"n_classes must be an int of at least 2, got {n_classes!r}")
        votes = _votes(L, n_classes)
        self.n_classes = n_classes
        n_lfs = votes.shape[1]
        active = votes[(votes != ABSTAIN).any(axis=1)]
        if not len(active):
            self.prior = np.full(n_classes, 1.0 / n_classes)
            self.confusion = np.full((n_lfs, n_classes, n_classes), 1.0 / n_classes)
            self.iterations = 0
            return self
        one_hot = self._one_hot(active)  # (examples, LFs, classes)
        counts = one_hot.sum(axis=1) + 1.0
        posterior = counts / counts.sum(axis=1, keepdims=True)  # majority vote, Laplace-smoothed
        for iteration in range(1, self.max_iter + 1):
            self._m_step(posterior, one_hot)
            updated = self._e_step(one_hot)
            moved = float(np.abs(updated - posterior).max())
            posterior = updated
            self.iterations = iteration
            if moved <= self.tol:
                break
        self._m_step(posterior, one_hot)
        return self

    def _one_hot(self, votes: np.ndarray) -> np.ndarray:
        assert self.n_classes is not None
        out = np.zeros((*votes.shape, self.n_classes), dtype=np.float64)
        rows, cols = np.nonzero(votes != ABSTAIN)
        out[rows, cols, votes[rows, cols]] = 1.0
        return out

    def _m_step(self, posterior: np.ndarray, one_hot: np.ndarray) -> None:
        self.prior = (posterior.sum(axis=0) + 1.0) / (posterior.shape[0] + posterior.shape[1])
        # confusion[j, c, k] ∝ 1 + Σ_i posterior[i, c] · [L_ij = k]
        weighted = np.einsum("ic,ijk->jck", posterior, one_hot) + 1.0
        self.confusion = weighted / weighted.sum(axis=2, keepdims=True)

    def _e_step(self, one_hot: np.ndarray) -> np.ndarray:
        assert self.prior is not None and self.confusion is not None
        log_conf = np.log(self.confusion)  # (LFs, classes, classes)
        # log P(votes | c) = Σ_j Σ_k one_hot[i, j, k] · log θ_j[c, k]
        log_like = np.einsum("ijk,jck->ic", one_hot, log_conf)
        log_post = log_like + np.log(self.prior)
        log_post -= log_post.max(axis=1, keepdims=True)
        post = np.exp(log_post)
        return post / post.sum(axis=1, keepdims=True)

    # -- prediction ----------------------------------------------------------

    def _fitted(self) -> None:
        if self.prior is None or self.confusion is None or self.n_classes is None:
            raise ValueError("the label model is not fitted")

    def predict_proba(self, L: np.ndarray) -> np.ndarray:
        """Posterior class probabilities per example; NaN rows where every Labeling_Function abstains (23.3)."""
        self._fitted()
        assert self.confusion is not None and self.n_classes is not None
        votes = _votes(L, self.n_classes, self.confusion.shape[0])
        out = np.full((votes.shape[0], self.n_classes), np.nan)
        voted = (votes != ABSTAIN).any(axis=1)
        if voted.any():
            out[voted] = self._e_step(self._one_hot(votes[voted]))
        return out

    def estimated_accuracy(self, L: np.ndarray) -> np.ndarray:
        """Per Labeling_Function: the mean posterior of the class it voted, over its votes (23.5); NaN without votes."""
        proba = self.predict_proba(L)
        assert self.confusion is not None and self.n_classes is not None
        votes = _votes(L, self.n_classes, self.confusion.shape[0])
        out = np.full(votes.shape[1], np.nan)
        for j in range(votes.shape[1]):
            rows = np.flatnonzero(votes[:, j] != ABSTAIN)
            if len(rows):
                out[j] = float(proba[rows, votes[rows, j]].mean())
        return out

    def to_json(self) -> dict[str, object]:
        """The fitted parameters (for the training manifest)."""
        self._fitted()
        assert self.prior is not None and self.confusion is not None
        return {"prior": self.prior.tolist(), "confusion": self.confusion.tolist(), "iterations": self.iterations}
