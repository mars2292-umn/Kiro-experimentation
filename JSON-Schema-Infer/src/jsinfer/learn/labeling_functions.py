"""Labeling_Functions for weak supervision (task 39.1; Requirements 23.1, 23.5).

A :class:`LabelingFunction` votes one outcome of its Judgment_Call kind, as
the class index in :data:`~jsinfer.decide.engine.JUDGMENT_OUTCOMES` order,
or abstains (:data:`~jsinfer.learn.label_model.ABSTAIN`). It reads the
Decision_Model feature vector of the Judgment_Call (``learn.features``), so
the same function votes on synthetic examples, where its thresholds are
fitted and its accuracy measured, and on the Judgment_Calls of unlabeled
collections. Every numeric cut-off is a Learned_Threshold ``lf.<id>.<name>``
(``config.thresholds``) or the sign of the MDL difference.

The functions (the design's table; ``source`` names the heuristic):

=====================  ==========================  ============================================================
kind                   id                          rule
=====================  ==========================  ============================================================
Record-or-Map          ``rm-mdl``                  the cheaper encoding under the MDL_Scorer
                       ``rm-key-entropy``          JXPlain: key-frequency entropy at or above a cut-off → Map
                       ``rm-key-pattern``          integer/UUID/date/hex keys at or above a share → Map
                       ``rm-dynamic``              JSONoid dynamic objects: singleton keys at or above a share → Map
Closed-or-Open-Record  ``cr-mdl``                  MDL
                       ``cr-keyset-stability``     key-set stability at or above a cut-off → closed
                       ``cr-good-turing``          Good–Turing unseen-key mass at or below a cut-off → closed
Tuple-or-List          ``tl-mdl``                  MDL
                       ``tl-fixed-length``         JXPlain: modal length share at or above a cut-off and
                                                   two or more element types → Tuple
                       ``tl-positional-entropy``   pooled minus positional type entropy at or above a cut-off → Tuple
Closed-or-Open-Enum    ``ce-mdl``                  MDL
                       ``ce-distinct-ratio``       S_obs/n at or below a cut-off → closed
                       ``ce-chao1``                Chao1/S_obs at or below a cut-off → closed
                       ``ce-jsonoid``              JSONoid: few distinct values, none seen once → closed
Merge-or-Split         ``ms-mdl``                  MDL
                       ``ms-jaccard``              mean key-set Jaccard at or above a cut-off → merge
                       ``ms-discriminator``        a scalar field informative about the groups → split
=====================  ==========================  ============================================================

Functions marked "→ X" with no alternative abstain otherwise; the others
vote the other outcome. The MDL functions abstain on an exact tie.

Thresholds (23.1)
    :func:`fit_lf_thresholds` fits each cut-off on the validation-split
    synthetic examples of its kind by grid search over the quantiles of the
    feature it reads, maximizing the votes that match the label minus the
    votes that do not, over all examples: abstaining is never rewarded for
    its own sake. Ties go to the smallest candidate.

Statistics (23.5)
    :func:`lf_statistics` gives, per function, the coverage and the conflict
    rate over the unlabeled examples, the label model's estimated accuracy,
    and the measured accuracy on the validation-split synthetic examples
    (undefined when the function casts no vote there).
"""

from __future__ import annotations

import math
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Final

import numpy as np

from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.jsonvalue import JSON
from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.learn.features import FEATURE_SPEC
from jsinfer.learn.label_model import ABSTAIN, DawidSkene

__all__ = [
    "LABELING_FUNCTIONS",
    "LFStatistics",
    "LabelingFunction",
    "conflict_rates",
    "coverage",
    "fit_lf_thresholds",
    "functions_of",
    "lf_statistics",
    "measured_accuracy",
    "vote_matrix",
]

Rule = Callable[[Mapping[str, float], Mapping[str, float]], int]
FIRST, SECOND = 0, 1


@dataclass(frozen=True)
class LabelingFunction:
    """One Labeling_Function (see the module docstring)."""

    id: str
    kind: str
    source: str
    thresholds: tuple[str, ...]
    statistic: Callable[[Mapping[str, float]], float] | None
    rule: Rule

    def vote(self, features: np.ndarray, thresholds: Mapping[str, float]) -> int:
        """The class index this function votes for one feature vector, or ABSTAIN."""
        named = dict(zip(FEATURE_SPEC[self.kind], np.asarray(features, dtype=np.float64).tolist()))
        missing = [name for name in self.thresholds if name not in thresholds]
        if missing:
            raise KeyError(f"{self.id}: no value for {missing}")
        return int(self.rule(named, thresholds))


def _mdl(kind: str) -> LabelingFunction:
    def rule(f: Mapping[str, float], t: Mapping[str, float]) -> int:
        d = f["normalized_mdl_difference"]
        return FIRST if d < 0 else SECOND if d > 0 else ABSTAIN

    prefix = {"record_or_map": "rm", "closed_or_open_record": "cr", "tuple_or_list": "tl", "closed_or_open_enum": "ce", "merge_or_split": "ms"}[kind]
    return LabelingFunction(f"{prefix}-mdl", kind, "MDL comparison", (), None, rule)


def _key_pattern_share(f: Mapping[str, float]) -> float:
    return f["integer_key_fraction"] + f["uuid_key_fraction"] + f["date_key_fraction"] + f["hex_key_fraction"]


LABELING_FUNCTIONS: Final[tuple[LabelingFunction, ...]] = (
    # Record-or-Map: record = 0, map = 1
    _mdl("record_or_map"),
    LabelingFunction(
        "rm-key-entropy", "record_or_map", "JXPlain key-entropy rule", ("lf.rm-key-entropy.min_entropy",),
        lambda f: f["key_frequency_entropy"],
        lambda f, t: SECOND if f["key_frequency_entropy"] >= t["lf.rm-key-entropy.min_entropy"] else FIRST,
    ),
    LabelingFunction(
        "rm-key-pattern", "record_or_map", "key-pattern regularity", ("lf.rm-key-pattern.min_fraction",),
        _key_pattern_share,
        lambda f, t: SECOND if _key_pattern_share(f) >= t["lf.rm-key-pattern.min_fraction"] > 0 else ABSTAIN,
    ),
    LabelingFunction(
        "rm-dynamic", "record_or_map", "JSONoid dynamic-object rule", ("lf.rm-dynamic.min_singleton_fraction",),
        lambda f: f["singleton_key_fraction"],
        lambda f, t: SECOND if f["singleton_key_fraction"] >= t["lf.rm-dynamic.min_singleton_fraction"] > 0 else ABSTAIN,
    ),
    # Closed-or-Open-Record: closed = 0, open = 1
    _mdl("closed_or_open_record"),
    LabelingFunction(
        "cr-keyset-stability", "closed_or_open_record", "key-set stability", ("lf.cr-keyset-stability.min_stability",),
        lambda f: f["keyset_stability"],
        lambda f, t: FIRST if f["keyset_stability"] >= t["lf.cr-keyset-stability.min_stability"] else SECOND,
    ),
    LabelingFunction(
        "cr-good-turing", "closed_or_open_record", "Good–Turing unseen-key mass", ("lf.cr-good-turing.max_unseen_mass",),
        lambda f: f["good_turing_unseen_mass"],
        lambda f, t: FIRST if f["good_turing_unseen_mass"] <= t["lf.cr-good-turing.max_unseen_mass"] else SECOND,
    ),
    # Tuple-or-List: tuple = 0, list = 1
    _mdl("tuple_or_list"),
    LabelingFunction(
        "tl-fixed-length", "tuple_or_list", "JXPlain fixed-length heterogeneity rule", ("lf.tl-fixed-length.min_modal_share",),
        lambda f: f["modal_length_share"],
        lambda f, t: FIRST if f["modal_length_share"] >= t["lf.tl-fixed-length.min_modal_share"] and f["distinct_element_types"] >= 2 else SECOND,
    ),
    LabelingFunction(
        "tl-positional-entropy", "tuple_or_list", "positional type entropy", ("lf.tl-positional-entropy.min_drop",),
        lambda f: max(0.0, -f["positional_minus_pooled_type_entropy"]),
        lambda f, t: FIRST if -f["positional_minus_pooled_type_entropy"] >= t["lf.tl-positional-entropy.min_drop"] > 0 else SECOND,
    ),
    # Closed-or-Open-Enum: closed = 0, open = 1
    _mdl("closed_or_open_enum"),
    LabelingFunction(
        "ce-distinct-ratio", "closed_or_open_enum", "distinct-value ratio", ("lf.ce-distinct-ratio.max_ratio",),
        lambda f: f["s_obs_ratio"],
        lambda f, t: FIRST if f["s_obs_ratio"] <= t["lf.ce-distinct-ratio.max_ratio"] else SECOND,
    ),
    LabelingFunction(
        "ce-chao1", "closed_or_open_enum", "Chao1 richness ratio", ("lf.ce-chao1.max_ratio",),
        lambda f: f["chao1_ratio"],
        lambda f, t: FIRST if f["chao1_ratio"] <= t["lf.ce-chao1.max_ratio"] else SECOND,
    ),
    LabelingFunction(
        "ce-jsonoid", "closed_or_open_enum", "JSONoid enum rule", ("lf.ce-jsonoid.max_values",),
        lambda f: f["s_obs"],
        lambda f, t: FIRST if f["s_obs"] <= t["lf.ce-jsonoid.max_values"] and f["f1"] == 0 else ABSTAIN,
    ),
    # Merge-or-Split: merge = 0, split = 1
    _mdl("merge_or_split"),
    LabelingFunction(
        "ms-jaccard", "merge_or_split", "key-set Jaccard", ("lf.ms-jaccard.min_mean",),
        lambda f: f["keyset_jaccard_mean"],
        lambda f, t: FIRST if f["keyset_jaccard_mean"] >= t["lf.ms-jaccard.min_mean"] else SECOND,
    ),
    LabelingFunction(
        "ms-discriminator", "merge_or_split", "discriminator presence", ("lf.ms-discriminator.min_information",),
        lambda f: f["scalar_field_information"],
        lambda f, t: SECOND if f["scalar_field_information"] >= t["lf.ms-discriminator.min_information"] > 0 else ABSTAIN,
    ),
)

for _lf in LABELING_FUNCTIONS:
    assert _lf.kind in JUDGMENT_OUTCOMES and all(name in THRESHOLDS for name in _lf.thresholds), _lf.id
assert len({lf.id for lf in LABELING_FUNCTIONS}) == len(LABELING_FUNCTIONS)


def functions_of(kind: str) -> tuple[LabelingFunction, ...]:
    """The Labeling_Functions of Judgment_Call ``kind``, in table order (at least three, 23.1)."""
    if kind not in JUDGMENT_OUTCOMES:
        raise ValueError(f"{kind!r} is not a Judgment_Call kind")
    return tuple(lf for lf in LABELING_FUNCTIONS if lf.kind == kind)


def vote_matrix(kind: str, x: np.ndarray, thresholds: Mapping[str, float]) -> np.ndarray:
    """The votes of the functions of ``kind`` on the feature rows ``x``: an (examples × functions) int matrix."""
    rows = np.asarray(x, dtype=np.float64).reshape(-1, len(FEATURE_SPEC[kind])) if len(x) else np.zeros((0, len(FEATURE_SPEC[kind])))
    functions = functions_of(kind)
    out = np.full((rows.shape[0], len(functions)), ABSTAIN, dtype=np.int64)
    for j, lf in enumerate(functions):
        for i, row in enumerate(rows):
            out[i, j] = lf.vote(row, thresholds)
    return out


# ---------------------------------------------------------------------------
# Thresholds (23.1)
# ---------------------------------------------------------------------------


def _net_accuracy(lf: LabelingFunction, rows: np.ndarray, truth: np.ndarray, thresholds: Mapping[str, float]) -> float:
    votes = np.array([lf.vote(row, thresholds) for row in rows], dtype=np.int64)
    voted = votes != ABSTAIN
    return float(((votes == truth) & voted).sum() - ((votes != truth) & voted).sum()) / max(len(rows), 1)


def fit_lf_thresholds(
    kind: str, x_val: np.ndarray, y_val: Sequence[str], *, training_data: str
) -> tuple[dict[str, ThresholdRecord], list[tuple[str, str]]]:
    """Fit the cut-offs of the functions of ``kind`` on validation examples; ``(records, [(name, reason)])`` omitted."""
    classes = JUDGMENT_OUTCOMES[kind]
    rows = np.asarray(x_val, dtype=np.float64).reshape(-1, len(FEATURE_SPEC[kind])) if len(y_val) else np.zeros((0, len(FEATURE_SPEC[kind])))
    truth = np.array([classes.index(label) for label in y_val], dtype=np.int64)
    records: dict[str, ThresholdRecord] = {}
    omitted: list[tuple[str, str]] = []
    for lf in functions_of(kind):
        for name in lf.thresholds:
            if len(truth) < 2 or len(set(truth.tolist())) < 2:
                omitted.append((name, f"validation examples of {kind} hold fewer than two classes"))
                continue
            spec = THRESHOLDS[name]
            assert lf.statistic is not None
            statistic = np.array([lf.statistic(dict(zip(FEATURE_SPEC[kind], row.tolist()))) for row in rows])
            grid = spec.candidates(statistic)
            scores = [_net_accuracy(lf, rows, truth, {name: float(v)}) for v in grid.tolist()]
            best = max(scores)
            value = grid[scores.index(best)].item()
            records[name] = ThresholdRecord.from_spec(
                spec, value=int(value) if spec.integer else float(value), objective_value=best,
                n_examples=int(len(truth)), training_data=training_data,
            )
    return records, omitted


# ---------------------------------------------------------------------------
# Statistics (23.5)
# ---------------------------------------------------------------------------


def coverage(L: np.ndarray) -> np.ndarray:
    """Per function: the fraction of examples it votes on (NaN with no example)."""
    L = np.asarray(L)
    return (L != ABSTAIN).mean(axis=0) if len(L) else np.full(L.shape[1], np.nan)


def conflict_rates(L: np.ndarray) -> np.ndarray:
    """Per function: among the examples it votes on, the fraction where another function votes another class."""
    L = np.asarray(L)
    out = np.full(L.shape[1], np.nan)
    for j in range(L.shape[1]):
        rows = L[L[:, j] != ABSTAIN]
        if len(rows):
            others = np.delete(rows, j, axis=1)
            out[j] = float(((others != ABSTAIN) & (others != rows[:, [j]])).any(axis=1).mean())
    return out


def measured_accuracy(L: np.ndarray, truth: np.ndarray) -> np.ndarray:
    """Per function: the fraction of its votes that match ``truth`` (class indices); NaN when it never votes."""
    L = np.asarray(L)
    truth = np.asarray(truth)
    out = np.full(L.shape[1], np.nan)
    for j in range(L.shape[1]):
        voted = L[:, j] != ABSTAIN
        if voted.any():
            out[j] = float((L[voted, j] == truth[voted]).mean())
    return out


@dataclass(frozen=True)
class LFStatistics:
    """The 23.5 statistics of one Labeling_Function; None for an undefined value."""

    id: str
    kind: str
    source: str
    coverage: float | None
    conflict: float | None
    est_acc: float | None
    measured_acc: float | None

    def to_json(self) -> dict[str, JSON]:
        return {
            "id": self.id, "kind": self.kind, "source": self.source, "coverage": self.coverage,
            "conflict": self.conflict, "est_acc": self.est_acc, "measured_acc": self.measured_acc,
        }


def _value(x: float) -> float | None:
    return None if math.isnan(x) else float(x)


def lf_statistics(
    kind: str, L_unlabeled: np.ndarray, model: DawidSkene | None, L_val: np.ndarray, y_val: Sequence[str]
) -> list[LFStatistics]:
    """The statistics of the functions of ``kind`` (see the module docstring)."""
    functions = functions_of(kind)
    classes = JUDGMENT_OUTCOMES[kind]
    cov = coverage(L_unlabeled) if len(L_unlabeled) else np.full(len(functions), np.nan)
    con = conflict_rates(L_unlabeled) if len(L_unlabeled) else np.full(len(functions), np.nan)
    est = model.estimated_accuracy(L_unlabeled) if model is not None and len(L_unlabeled) else np.full(len(functions), np.nan)
    truth = np.array([classes.index(label) for label in y_val], dtype=np.int64)
    meas = measured_accuracy(L_val, truth) if len(truth) else np.full(len(functions), np.nan)
    return [
        LFStatistics(lf.id, kind, lf.source, _value(cov[j]), _value(con[j]), _value(est[j]), _value(meas[j]))
        for j, lf in enumerate(functions)
    ]
