"""Property 51: Split integrity (task 25.3).

For any corpus (schema ids with normalized Path sets, catalog version
groups, and 30.5 exclusions), seed, valid proportions, and Real_Dataset
known schemas (in the corpus or given as Path sets):

- every schema is assigned to exactly one split, and every near-duplicate
  group (catalog versions, and pairs whose Path-set Jaccard reaches the
  recorded cut-off, transitively) lies in one split;
- every known schema's group, and every corpus schema whose similarity to a
  known schema outside the corpus reaches the cut-off, is held out;
- two builds with equal inputs give identical splits;
- the deviation flags mark exactly the splits whose realized share differs
  from its configured proportion by more than 0.05, and the recorded
  proportions, groups, exclusions, and cut-off are what the build used;
- invalid proportions raise ``SplitError`` (30.10).

Inputs
    Corpora of 3 to 24 schemas whose Path sets are drawn from a small key
    pool, so near-duplicates occur; a few schemas get the Path set of another
    (exact duplicates) or share a catalog entry; one to three are known
    schemas or excluded. Proportions are drawn to sum to 1.

**Validates: Requirements 30.2, 30.3, 30.4, 30.10**
"""

# Feature: json-schema-inference, Property 51: Split integrity

from __future__ import annotations

import pytest
from hypothesis import assume, event, given
from hypothesis import strategies as st

from jsinfer.bench.splits import DEVIATION, SPLITS, SchemaSplit, SplitBuilder, SplitError, SplitInputs

KEYS = ("id", "name", "email", "tags", "created", "value", "kind", "meta", "x", "y")
BUILDER = SplitBuilder(perm=32, bands=8)


def path_set(keys: frozenset[str]) -> frozenset[str]:
    return frozenset({"$", *(f"$.{k}" for k in keys)})


@st.composite
def corpora(draw: st.DrawFn) -> tuple[SplitInputs, dict[str, float]]:
    n = draw(st.integers(3, 24), label="schemas")
    ids = [f"s{i:02d}" for i in range(n)]
    paths: dict[str, frozenset[str]] = {}
    for sid in ids:
        if paths and draw(st.integers(0, 3), label=f"{sid} duplicate") == 0:
            paths[sid] = paths[draw(st.sampled_from(sorted(paths)), label=f"{sid} copies")]
        else:
            paths[sid] = path_set(frozenset(draw(st.lists(st.sampled_from(KEYS), min_size=1, max_size=5), label=f"{sid} keys")))
    catalog = tuple(
        tuple(group)
        for group in draw(st.lists(st.lists(st.sampled_from(ids), min_size=2, max_size=3, unique=True), max_size=2), label="catalog")
    )
    known_ids = tuple(draw(st.lists(st.sampled_from(ids), max_size=2, unique=True), label="known ids"))
    known_paths = {
        f"real{i}": path_set(frozenset(draw(st.lists(st.sampled_from(KEYS), min_size=1, max_size=4), label=f"real{i}")))
        for i in range(draw(st.integers(0, 2), label="known files"))
    }
    excluded = tuple((sid, "partially supported") for sid in draw(st.lists(st.sampled_from(ids), max_size=2, unique=True), label="excluded"))
    train = draw(st.floats(0.2, 0.8), label="train")
    val = draw(st.floats(0.05, 1 - train - 0.05), label="val")
    proportions = {"split.train": train, "split.val": val, "split.held_out": 1 - train - val}
    return SplitInputs(paths, excluded, catalog, known_ids, known_paths), proportions


def jaccard(a: frozenset[str], b: frozenset[str]) -> float:
    union = len(a | b)
    return len(a & b) / union if union else 1.0


@given(case=corpora(), seed=st.integers(0, 2**31 - 1))
def test_groups_lie_in_one_split_known_schemas_are_held_out_and_builds_agree(case: tuple[SplitInputs, dict[str, float]], seed: int) -> None:
    inputs, proportions = case
    try:
        split = BUILDER.build(inputs, split_id="p51", seed=seed, proportions=proportions)
    except SplitError as error:
        assert "without a schema" in str(error)
        event("a split would be empty")
        return
    assert isinstance(split, SchemaSplit)
    ids = sorted(inputs.paths)
    assignment = {sid: split.split_of(sid) for sid in ids}
    assert all(assignment[sid] is not None for sid in ids) and len(split.train) + len(split.val) + len(split.held_out) == len(ids)
    groups = split.extra["groups"]
    cutoff = split.extra["cutoff"]["value"]
    assert 0.0 < cutoff <= 1.0
    # Grouping: catalog versions and pairs at or above the cut-off share a group, and a group shares a split.
    for versions in inputs.catalog_groups:
        assert len({groups[sid] for sid in versions}) == 1
    for i, a in enumerate(ids):
        for b in ids[i + 1 :]:
            if jaccard(inputs.paths[a], inputs.paths[b]) >= cutoff:
                assert groups[a] == groups[b], f"{a} and {b} reach the cut-off {cutoff} but are not grouped"
            if groups[a] == groups[b]:
                assert assignment[a] == assignment[b], f"group {groups[a]} is split between {assignment[a]} and {assignment[b]}"
    # Known schemas (30.3).
    for sid in inputs.known_ids:
        assert assignment[sid] == "held_out"
    for name, known in inputs.known_paths.items():
        for sid in ids:
            if jaccard(known, inputs.paths[sid]) >= cutoff:
                assert assignment[sid] == "held_out", f"{sid} is similar to the known schema {name} but not held out"
                assert sid in split.extra["known_schemas"]["similar"].get(name, [])
    # Determinism (30.4).
    assert BUILDER.build(inputs, split_id="p51", seed=seed, proportions=proportions) == split
    # Deviation flags and the recorded members (30.4).
    recorded = split.extra["proportions"]
    for name, param in zip(SPLITS, ("split.train", "split.val", "split.held_out")):
        realized = len(split.ids(name)) / len(ids)
        assert recorded["realized"][name] == realized and recorded["configured"][name] == proportions[param]
        assert recorded["deviation"][name] == (abs(realized - proportions[param]) > DEVIATION)
    assert split.extra["excluded"] == [{"id": sid, "reason": reason} for sid, reason in sorted(inputs.excluded)]
    assert split.extra["known_schemas"]["ids"] == list(inputs.known_ids)
    event(f"cut-off {'one' if cutoff == 1.0 else 'below one'}")
    event(f"deviation flagged: {'yes' if any(recorded['deviation'].values()) else 'no'}")


@given(case=corpora(), seed=st.integers(0, 2**31 - 1), bad=st.sampled_from(["zero", "one", "sum"]))
def test_invalid_proportions_are_rejected(case: tuple[SplitInputs, dict[str, float]], seed: int, bad: str) -> None:
    inputs, proportions = case
    broken = dict(proportions)
    if bad == "zero":
        broken["split.val"] = 0.0
    elif bad == "one":
        broken["split.train"] = 1.0
    else:
        broken["split.held_out"] = proportions["split.held_out"] + 0.01
    assume(bad != "sum" or broken["split.held_out"] < 1.0)
    with pytest.raises(SplitError, match="invalid split proportions"):
        BUILDER.build(inputs, split_id="p51", seed=seed, proportions=broken)

