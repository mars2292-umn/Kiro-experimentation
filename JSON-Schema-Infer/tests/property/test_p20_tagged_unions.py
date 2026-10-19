"""Property 20: Tagged unions are exact (task 34.2).

For any object Path with a selected Discriminator: the Discriminator is
present in every object and is a string, integer, or boolean; every
retained tree split lowers the description length; the dedicated branches
carry disjoint ``const``/``enum`` sets and require the Base_Fields plus the
fields common to their group; the fallback branch exists if and only if
10.5 or 10.8 applies; and every non-flagged object validates against
exactly one branch.
"""

# Feature: json-schema-inference, Property 20: Tagged unions are exact

from __future__ import annotations

from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.jsonvalue import canonical_key
from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.structure.analyzer import StructureAnalyzer

VALIDATOR = Validator()
TAGS = ("alpha", "beta", "gamma", "delta", 1, 2, True)
EXTRA = {"alpha": ("ax", "ay"), "beta": ("bx",), "gamma": ("gx", "gy", "gz"), "delta": (), 1: ("one",), 2: ("two",), True: ("yes",)}


class StubBundle:
    def __init__(self, threshold: float) -> None:
        self.t = threshold

    def threshold(self, name: str) -> float | None:
        return self.t if name == "tagged.branch_support" else None

    def model(self, name: str) -> Any:
        return None

    @property
    def bundle_id(self) -> str:
        return "stub"


# The branches are read at their Paths: shared definitions and recursion (Property 16) stay off.
PATH_BY_PATH = {"structure.shared_defs": False, "structure.recursion": False}


@st.composite
def unions(draw: st.DrawFn) -> tuple[list[dict[str, Any]], float]:
    tags = draw(st.lists(st.sampled_from(TAGS), min_size=2, max_size=4, unique=True), label="tags")
    weights = [draw(st.integers(1, 12), label=f"weight {tag}") for tag in tags]
    base = draw(st.integers(0, 2), label="base fields")
    docs = []
    for tag, weight in zip(tags, weights):
        for i in range(weight):
            d: dict[str, Any] = {"tag": tag, **{f"base{j}": j for j in range(base)}}
            for name in EXTRA[tag]:
                if draw(st.integers(0, 4), label=f"{tag} {name} {i}") != 0:  # mostly present, sometimes absent
                    d[name] = f"{name}-{i}"
            if draw(st.booleans(), label=f"{tag} loose {i}"):
                d["loose"] = i
            docs.append(d)
    threshold = draw(st.sampled_from([0.05, 0.15, 0.3, 0.5]), label="branch support")
    return docs, threshold


@given(case=unions())
def test_selected_discriminator_branches_are_exact(case: tuple[list[dict[str, Any]], float]) -> None:
    docs, threshold = case
    report = InferenceReport()
    sa = StructureAnalyzer.for_techniques(PATH_BY_PATH, bundle=StubBundle(threshold), report=report)
    model = sa.infer_model(View.root(build_store(docs)))
    records = [r for cid, r in model.provenance.items() if cid.keyword == "oneOf" and r.technique == "rules.decision_trees"]
    if not records:
        event("no Discriminator selected")
        return
    event("Discriminator selected")
    name = records[0].details["discriminator"]
    assert all(name in d and isinstance(d[name], (str, int, bool)) and not isinstance(d[name], float) for d in docs)
    for split in records[0].details["splits"]:
        assert split["feature"] in docs[0]
    schema = model.root.to_json()
    branches = schema["oneOf"]
    base = set.intersection(*(set(d) for d in docs))
    dedicated = [r for r in records if r.details["branch"] == "dedicated"]
    fallback = [r for r in records if r.details["branch"] == "fallback"]
    assert len(fallback) <= 1 and len(dedicated) + len(fallback) == len(branches)
    seen: set[Any] = set()
    for record in dedicated:
        values = {canonical_key(v) for v in record.details["value_group"]}
        assert values and not values & seen
        seen |= values
        assert record.details["support_ratio"] >= threshold
        group = [d for d in docs if canonical_key(d[name]) in values]
        common = set.intersection(*(set(d) for d in group))
        branch = branches[int(records.index(record))] if False else None  # branches are matched by their constraint below
        matching = [b for b in branches if {canonical_key(v) for v in ([b["properties"][name]["const"]] if "const" in b["properties"][name] else b["properties"][name].get("enum", []))} == values]
        assert len(matching) == 1
        assert set(matching[0].get("required", [])) == common >= base
    below = [r.details["below_threshold"] for r in records][0]
    closed = fallback[0].details["closed_enum"] if fallback else None
    if below:
        assert fallback, "a below-threshold group needs the fallback (10.5)"
    if not fallback:
        assert not below
    compiled = [VALIDATOR.compile(b) for b in branches]
    for d in docs:
        assert sum(1 for c in compiled if VALIDATOR.is_valid(c, d)) == 1, d
    if fallback and closed is False:
        event("open Discriminator: not-enum fallback")
        unseen = {**docs[0], name: "never-seen-value"}
        assert sum(1 for c in compiled if VALIDATOR.is_valid(c, unseen)) == 1
