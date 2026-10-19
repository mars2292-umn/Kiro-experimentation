"""Property 57: Study construction, ablation clause (task 30.2). The contamination clause is added in 41.3.

For any default Run_Configuration (technique states and an acyclic
prerequisite map), the ablation set holds one configuration per enabled
Technique; each disables that Technique and, transitively, exactly the
enabled Techniques that would lose a prerequisite; the disabled set is
recorded and consistent with the configuration's states.
"""

# Feature: json-schema-inference, Property 57: Study construction

from __future__ import annotations

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.bench.studies import ablation_configurations


@st.composite
def registries(draw: st.DrawFn) -> tuple[dict[str, bool], dict[str, list[str]]]:
    n = draw(st.integers(1, 9), label="techniques")
    ids = [f"t{i}" for i in range(n)]
    states = {tid: draw(st.booleans(), label=f"{tid} enabled") for tid in ids}
    prerequisites = {tid: draw(st.lists(st.sampled_from(ids[:i]), max_size=3, unique=True), label=f"{tid} prerequisites") if i else [] for i, tid in enumerate(ids)}
    return states, prerequisites


def closure(start: str, states: dict[str, bool], prerequisites: dict[str, list[str]]) -> set[str]:
    out = {start}
    changed = True
    while changed:
        changed = False
        for tid, on in states.items():
            if on and tid not in out and any(p in out for p in prerequisites[tid]):
                out.add(tid)
                changed = True
    return out


@given(case=registries())
def test_one_configuration_per_enabled_technique_with_transitive_dependents(case: tuple[dict[str, bool], dict[str, list[str]]]) -> None:
    states, prerequisites = case
    configs = ablation_configurations(states, prerequisites)
    enabled = [tid for tid, on in states.items() if on]
    assert [c.technique for c in configs] == enabled
    event(f"{len(enabled)} enabled")
    for config in configs:
        expected = closure(config.technique, states, prerequisites)
        assert set(config.disabled) == expected
        assert config.disabled == tuple(sorted(expected))
        assert all(states[t] for t in config.disabled)  # only enabled Techniques are recorded as disabled
        for tid, on in config.states.items():
            assert on == (states[tid] and tid not in expected)
            if on:
                assert not any(p in expected for p in prerequisites[tid])
        assert set(config.settings()) == {f"techniques.{t}" for t in expected}
