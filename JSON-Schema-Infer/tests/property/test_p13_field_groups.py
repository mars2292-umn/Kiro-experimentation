"""Property 13: Field_Groups are well-formed and hold (task 33.2).

For any Variant (here: any object set with planted add-on groups, base
fields, and loose optional fields), every selected Field_Group has at least
two fields, contains no Base_Field, is disjoint from the other selected
groups, and is all-or-none in every non-flagged object. It is emitted as a
complete ``dependentRequired`` clique, and the Base_Fields are ``required``
in the Variant's schema.
"""

# Feature: json-schema-inference, Property 13: Field_Groups are well-formed and hold

from __future__ import annotations

from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.views import View
from jsinfer.deps.presence import Presence, required_fields
from jsinfer.io.validator import Validator
from jsinfer.structure.field_groups import FieldGroupMiner, clique_entries

VALIDATOR = Validator()
BASE = ("id", "kind")
GROUPS = (("street", "city", "zip"), ("phone", "ext"), ("lat", "lon"))
LOOSE = ("note", "tag", "extra")


@st.composite
def variants(draw: st.DrawFn) -> list[dict[str, Any]]:
    n = draw(st.integers(2, 30), label="objects")
    n_base = draw(st.integers(0, len(BASE)), label="base fields")
    docs = []
    for i in range(n):
        d: dict[str, Any] = {name: i for name in BASE[:n_base]}
        for group in GROUPS:
            roll = draw(st.integers(0, 9), label=f"object {i} {group[0]}")
            if roll < 4:
                d.update({name: 1 for name in group})
            elif roll == 4:  # a partial group: the all-or-none filter must drop it
                d[group[0]] = 1
        for name in LOOSE:
            if draw(st.booleans(), label=f"object {i} {name}"):
                d[name] = 0
        docs.append(d)
    return docs


@given(docs=variants(), seed=st.integers(0, 50))
def test_selected_groups_are_well_formed_cliques_that_hold(docs: list[dict[str, Any]], seed: int) -> None:
    p = Presence.of(View.root(build_store(docs)))
    out = FieldGroupMiner.for_techniques(seed=seed).mine(p)
    base = set(p.required)
    seen: set[str] = set()
    for group in out.groups:
        assert len(group) >= 2
        assert not group & base
        assert not group & seen
        seen |= group
        for d in docs:
            held = sum(1 for name in group if name in d)
            assert held in (0, len(group)), (group, d)
    entries = out.entries()
    assert entries == clique_entries(out.selected)
    for group in out.groups:
        for member in group:
            assert set(entries[member]) == group - {member}
    for candidate in out.candidates:
        assert candidate.excluded == any(0 < sum(1 for name in candidate.fields if name in d) < len(candidate.fields) for d in docs)
    schema = {"type": "object", "required": list(required_fields(p).presence.required), **out.keywords()}
    compiled = VALIDATOR.compile(schema)
    assert all(VALIDATOR.is_valid(compiled, d) for d in docs)
    event(f"{len(out.groups)} groups selected")
    if any(c.excluded for c in out.candidates):
        event("a candidate was excluded")
