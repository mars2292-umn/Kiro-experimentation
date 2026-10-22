"""Property 36: Label model and LF statistics are well-formed (task 39.3).

For any vote matrix over any number of Labeling_Functions and classes, the
Dawid–Skene label model returns a probability vector per row with at least
one vote (non-negative, summing to 1), a NaN row where every function
abstains (23.3), and an estimated accuracy in [0, 1] per function that
votes. Coverage, conflict, and measured accuracy are fractions in [0, 1],
coverage is the share of non-abstain votes, and a function that never votes
has an undefined (NaN, written as null) conflict and accuracy (23.5).
Fitting twice gives identical output.

Inputs
    Vote matrices of 1–60 rows, 1–6 functions, 2–3 classes, with abstains
    drawn at any rate, and truth labels for the measured accuracy.

**Validates: Requirements 23.3, 23.5**
"""

# Feature: json-schema-inference, Property 36: Label model and LF statistics are well-formed

from __future__ import annotations

import numpy as np
from hypothesis import given
from hypothesis import strategies as st

from jsinfer.learn.label_model import ABSTAIN, DawidSkene
from jsinfer.learn.labeling_functions import conflict_rates, coverage, measured_accuracy


@st.composite
def vote_matrices(draw: st.DrawFn) -> tuple[np.ndarray, np.ndarray, int]:
    n_classes = draw(st.integers(2, 3))
    rows = draw(st.integers(1, 60))
    functions = draw(st.integers(1, 6))
    vote = st.integers(ABSTAIN, n_classes - 1)
    L = np.array(draw(st.lists(st.lists(vote, min_size=functions, max_size=functions), min_size=rows, max_size=rows)))
    truth = np.array(draw(st.lists(st.integers(0, n_classes - 1), min_size=rows, max_size=rows)))
    return L, truth, n_classes


@given(vote_matrices())
def test_label_model_output_is_well_formed(drawn: tuple[np.ndarray, np.ndarray, int]) -> None:
    L, _, n_classes = drawn
    model = DawidSkene().fit(L, n_classes)
    proba = model.predict_proba(L)
    assert proba.shape == (L.shape[0], n_classes)
    silent = (L == ABSTAIN).all(axis=1)
    assert np.isnan(proba[silent]).all()
    voted = proba[~silent]
    assert not np.isnan(voted).any()
    assert np.all(voted >= 0) and np.allclose(voted.sum(axis=1), 1.0)
    estimated = model.estimated_accuracy(L)
    never = (L == ABSTAIN).all(axis=0)
    assert estimated.shape == (L.shape[1],) and np.array_equal(np.isnan(estimated), never)
    assert np.all((estimated[~never] >= 0) & (estimated[~never] <= 1))
    again = DawidSkene().fit(L, n_classes).predict_proba(L)
    assert np.array_equal(np.nan_to_num(proba, nan=-1.0), np.nan_to_num(again, nan=-1.0))


@given(vote_matrices())
def test_lf_statistics_are_fractions(drawn: tuple[np.ndarray, np.ndarray, int]) -> None:
    L, truth, _ = drawn
    cov, conflict, acc = coverage(L), conflict_rates(L), measured_accuracy(L, truth)
    never = (L == ABSTAIN).all(axis=0)
    assert np.allclose(cov, (L != ABSTAIN).mean(axis=0))
    for stat in (conflict, acc):
        assert stat.shape == (L.shape[1],) and np.array_equal(np.isnan(stat), never)
        assert np.all((stat[~never] >= 0) & (stat[~never] <= 1))
