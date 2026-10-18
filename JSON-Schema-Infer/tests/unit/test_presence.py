"""Unit tests for deps/presence.py: presence bitsets, Base_Fields, and ``required`` (task 14.1).

Requirements 6.1 (Base_Fields per Variant and per family), 6.5 (every
Base_Field under ``required`` in every branch), 9.1 (required or optional,
from the Variant's own non-flagged objects), 9.2 (present with any value,
``null`` included), 9.7 (``required`` lists the required fields and no
optional one), and 26.13 (each entry's record names ``baseline:required``
with the Laplace Confidence).
"""

from __future__ import annotations

import random
from typing import Any

import numpy as np
import pytest

from jsinfer.core.observations import build_store
from jsinfer.core.paths import MAPVAL_STEP
from jsinfer.core.provenance import laplace_confidence
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.deps.presence import (
    OPTIONAL,
    REQUIRED,
    REQUIRED_PRODUCER,
    Presence,
    RequiredFields,
    family_base_fields,
    popcount,
    presence,
    required_fields,
)
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.validator import Validator
from jsinfer.pipeline.naive_merge import naive_merge_node
from jsinfer.structure.variants import VariantAnalyzer

VALIDATOR = Validator()

USERS = [{"id": i, "kind": "user", "name": f"u{i}", "age": 20 + i % 30} for i in range(24)]
ITEMS = [{"id": 100 + i, "sku": f"s{i}", "price": i + 0.25, **({"tags": ["a"]} if i % 2 else {})} for i in range(20)]


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


def unpack(p: Presence, row: int) -> list[bool]:
    return np.unpackbits(p.bits[row], count=p.n_objects, bitorder="little").astype(bool).tolist()


def reference(objects: list[dict[str, Any]]) -> dict[str, list[bool]]:
    """Brute-force presence: for each key in code-point order, which objects hold it."""
    keys = sorted({key for obj in objects for key in obj})
    return {key: [key in obj for obj in objects] for key in keys}


# ---------------------------------------------------------------------------
# Presence and classification (9.1, 9.2)
# ---------------------------------------------------------------------------


def test_fields_are_classified_from_the_objects() -> None:
    docs: list[Any] = [{"a": 1, "b": None}, {"a": 2}, {"a": 3, "b": 4, "c": 5}]
    p = presence(root(docs))
    assert p.names == ("a", "b", "c") and p.n_objects == 3 and p.n_fields == 3
    assert p.counts.tolist() == [3, 2, 1] and p.count("b") == 2 and p.count("zzz") == 0
    assert p.required == p.base_fields == ("a",)
    assert p.optional == ("b", "c")
    assert p.classification() == {"a": REQUIRED, "b": OPTIONAL, "c": OPTIONAL}
    assert p.is_required("a") and not p.is_required("b") and not p.is_required("zzz")
    assert "a" in p and "zzz" not in p and 1 not in p
    assert p.path == () and p.epoch == p.objects.store.flags.epoch
    with pytest.raises(KeyError, match="holds 'zzz'"):
        p.classify("zzz")


def test_null_values_count_as_present() -> None:
    p = presence(root([{"a": None, "b": 1}, {"a": 0}, {"a": None}]))
    assert p.required == ("a",) and p.optional == ("b",)  # 9.2: any value, including null


def test_bitsets_are_packed_little_endian_and_match_the_stats() -> None:
    docs: list[Any] = [{"k": i} if i % 3 == 0 or i == 17 else {"other": i} for i in range(21)]
    view = root(docs)
    p = presence(view)
    assert p.n_bytes == 3 and p.bits.shape == (2, 3) and p.bits.dtype == np.uint8
    assert not p.bits.flags.writeable and not p.counts.flags.writeable
    assert unpack(p, p.index("k")) == [i % 3 == 0 or i == 17 for i in range(21)]
    stats = view.stats()
    np.testing.assert_array_equal(p.bitset("k"), stats.bitset(view.store.key_id("k")))
    assert p.mask("k").tolist() == unpack(p, p.index("k"))
    # The masks select objects of the covered view.
    assert p.objects.subset(p.mask("k")).stats().n_objects == p.count("k")
    assert popcount(p.bits) == int(p.counts.sum())
    with pytest.raises(TypeError, match="uint8"):
        popcount(np.ones(2, dtype=np.int64))


def test_presence_matches_a_brute_force_reference() -> None:
    gen = random.Random(14)
    keys = ["alpha", "b", "B", "é", "c-d", "z"]
    docs: list[Any] = []
    for i in range(150):
        if i % 17 == 0:
            docs.append(gen.choice([1, "s", [1, 2], None]))  # non-objects at the Path are ignored
            continue
        # "b" is held by every object, with values that include null.
        docs.append({k: gen.choice([None, 1, "x", [], {}]) for k in keys if k == "b" or gen.random() < 0.4})
    objects = [doc for doc in docs if isinstance(doc, dict)]
    expected = reference(objects)
    p = presence(root(docs))
    assert p.n_objects == len(objects)
    assert list(p.names) == list(expected)  # code-point order: "B" < "alpha" < "b" < "c-d" < "z" < "é"
    for j, name in enumerate(p.names):
        assert unpack(p, j) == expected[name]
        assert p.count(name) == sum(expected[name])
    assert p.required == tuple(k for k, row in expected.items() if all(row))
    assert p.optional == tuple(k for k, row in expected.items() if not all(row))

    joint = p.joint_counts()
    implied = p.implication_matrix()
    for i, a in enumerate(p.names):
        for j, b in enumerate(p.names):
            both = sum(x and y for x, y in zip(expected[a], expected[b]))
            subset = all(y for x, y in zip(expected[a], expected[b]) if x)
            assert joint[i, j] == both == p.joint_count(a, b)
            assert implied[i, j] == subset == p.implies(a, b)
    assert np.array_equal(np.diag(joint), p.counts) and np.diag(implied).all()
    assert "b" in p.required and implied[:, p.index("b")].all()  # every field implies a required one
    assert p.optional  # the other keys are optional


def test_implication_ignores_padding_bits() -> None:
    # 9 objects: the second byte has seven padding bits, which ~bits sets.
    docs: list[Any] = [{"a": 1, "b": 1}] * 2 + [{"b": 1}] * 6 + [{"c": 1}]
    p = presence(root(docs))
    assert p.n_bytes == 2
    assert p.implies("a", "b") and not p.implies("b", "a") and not p.implies("a", "c")
    assert p.implication_matrix().tolist() == [[True, True, False], [False, True, False], [False, False, True]]


def test_only_non_flagged_objects_count() -> None:
    store = build_store([{"a": 1, "b": 2}, {"a": 3}, {"a": 4, "b": 5}, "s"])
    assert presence(View.root(store)).required == ("a",)
    store.flags.flag_documents([1])
    p = presence(View.root(store))
    assert p.n_objects == 2 and p.required == ("a", "b") and p.optional == ()


def test_flagged_member_values_keep_their_key_present() -> None:
    store = build_store([{"a": 1, "b": 2}, {"a": 3, "b": 4}])
    b = View.root(store).child_key("b")
    store.flags.flag_items((MAPVAL_STEP,), b.parts[0].rows[:1])  # document 0's "b" value
    assert View.root(store).child_key("b").stats().n == 1
    assert presence(View.root(store)).required == ("a", "b")  # the parent object still holds "b"


def test_objects_without_keys_have_no_fields() -> None:
    p = presence(root([{}, {}, 3]))
    assert p.n_objects == 2 and p.names == () and p.bits.shape == (0, 1)
    assert p.required == () and p.joint_counts().shape == (0, 0) and p.implication_matrix().shape == (0, 0)
    assert required_fields(p).keywords() == {}


def test_a_view_without_objects_is_an_error() -> None:
    with pytest.raises(ValueError, match="no non-flagged object"):
        presence(root([1, "a", [2]]))
    with pytest.raises(TypeError):
        required_fields("not a view")  # type: ignore[arg-type]


def test_presence_checks_its_shape() -> None:
    p = presence(root([{"a": 1}, {"a": 2, "b": 3}]))
    with pytest.raises(ValueError, match="bits must be uint8"):
        Presence(p.objects, p.names, p.key_ids, p.counts, p.bits[:, :0])
    with pytest.raises(ValueError, match="one entry per field"):
        Presence(p.objects, p.names[:1], p.key_ids, p.counts, p.bits)
    with pytest.raises(ValueError, match="held by 1"):
        Presence(p.objects, p.names, p.key_ids, np.array([3, 1]), p.bits)


def test_presence_does_not_depend_on_document_order() -> None:
    docs = [*USERS, *ITEMS]
    reference_presence = presence(root(docs))
    shuffled = list(docs)
    random.Random(3).shuffle(shuffled)
    for order in (docs[::-1], shuffled):
        p = presence(root(order))
        assert p.names == reference_presence.names and p.required == reference_presence.required
        assert p.counts.tolist() == reference_presence.counts.tolist()
        assert np.array_equal(p.joint_counts(), reference_presence.joint_counts())
        assert np.array_equal(p.implication_matrix(), reference_presence.implication_matrix())


def test_presence_agrees_with_naive_merge_required() -> None:
    for docs in ([*USERS, *ITEMS], USERS, ITEMS, [{"a": None}, {"a": 1, "b": 2}]):
        view = root(docs)
        assert list(presence(view).required) == naive_merge_node(view).kw.get("required", [])


# ---------------------------------------------------------------------------
# The required keyword and its provenance (9.7, 26.13)
# ---------------------------------------------------------------------------


def test_required_keyword_lists_exactly_the_required_fields() -> None:
    view = root([{"b": 1, "a": None, "c": 1}, {"a": 2, "b": 3}, {"b": 4, "a": 5}])
    required = required_fields(view)
    assert required.names == ("a", "b") and not required.is_empty
    keywords = required.keywords()
    assert keywords == {"required": ["a", "b"]}
    keywords["required"].append("c")  # a fresh list each time
    assert required.keywords() == {"required": ["a", "b"]}


def test_required_entries_record_the_baseline_rule() -> None:
    required = required_fields(root([{"a": 1, "b": 1}] * 4 + [{"a": 2}]))
    assert required.names == ("a",)
    record = required.provenance["a"]
    assert record.technique == REQUIRED_PRODUCER == "baseline:required"
    assert record.fallback is None and record.resolution is None  # a Baseline_Rule resolves no Decision_Point
    assert record.cutoff is None and record.sample is None
    assert (record.support.consistent, record.support.total) == (5, 5)
    assert record.confidence == laplace_confidence(record.support) == 6 / 7
    assert dict(required.provenance) == {"a": record}


def test_constraint_provenance_matches_the_emitted_pointers() -> None:
    view = root([{"z": 1, "a": 1, "é": 1, "m": 1}, {"z": 2, "a": 2, "é": 2}])
    required = required_fields(view)
    node = SchemaNode(kw={"type": ["object"], **required.keywords()})
    records = required.constraint_provenance(node)
    assert list(records) == [ConstraintId(node.nid, REQUIRED, name) for name in ("a", "z", "é")]
    _, pointers = SchemaEmitter().layout(SchemaModel(root=node))
    assert {pointers[cid] for cid in records} == {"/required/0", "/required/1", "/required/2"}
    for index, name in enumerate(("a", "z", "é")):
        assert pointers[ConstraintId(node.nid, REQUIRED, name)] == f"/required/{index}"
    # The node's required list may be in any order, but must hold exactly these names.
    assert required.constraint_provenance(SchemaNode(kw={"required": ["é", "a", "z"]}))
    for bad in ({"required": ["a", "z"]}, {"required": ["a", "z", "é", "m"]}, {}, {"required": ["a", "a", "z"]}):
        with pytest.raises(ValueError, match="must list exactly"):
            required.constraint_provenance(SchemaNode(kw=bad))
    with pytest.raises(ValueError, match="must list exactly"):
        required.constraint_provenance(SchemaNode(boolean=True))
    # No required field: no keyword and no records.
    none = required_fields(root([{"a": 1}, {"b": 1}]))
    assert none.is_empty and none.keywords() == {} and none.constraint_provenance(SchemaNode()) == {}


def test_required_fields_accepts_a_presence() -> None:
    p = presence(root(USERS))
    required = required_fields(p)
    assert isinstance(required, RequiredFields) and required.presence is p
    assert required.names == ("age", "id", "kind", "name")


# ---------------------------------------------------------------------------
# Base_Fields of Variants and families (6.1, 6.5)
# ---------------------------------------------------------------------------


def variant_record(view: View) -> SchemaNode:
    """A Variant schema as the analyzer builds it: Naive_Merge properties plus ``required`` from presence."""
    node = naive_merge_node(view)
    node.kw.pop("required", None)
    node.kw.update(required_fields(view).keywords())
    return node


def test_each_variant_uses_only_its_own_objects() -> None:
    variants = VariantAnalyzer.for_techniques(validator=VALIDATOR)
    view = root([*USERS, *ITEMS])
    outcome = variants.merge_or_split(variants.select(view))
    assert outcome.n_variants == 2
    by_kind = {"kind" in p: p for p in (presence(v) for v in outcome.views())}
    users, items = by_kind[True], by_kind[False]
    assert users.required == ("age", "id", "kind", "name") and users.optional == ()
    assert items.required == ("id", "price", "sku") and items.optional == ("tags",)
    assert presence(view).required == ("id",)  # the Path as a whole


def test_family_base_fields_are_the_intersection() -> None:
    variants = VariantAnalyzer.for_techniques(validator=VALIDATOR)
    view = root([*USERS, *ITEMS])
    node = variants.variant_node(view, variant_schema=variant_record)
    assert node.keyword is not None and len(node.branches) == 2
    family = family_base_fields(node.views)
    assert family == ("id",)
    # The final Variants partition the objects, so the intersection is the union's Base_Fields.
    assert family == presence(view).base_fields
    # 6.5: every family Base_Field is under required in every branch.
    for branch in node.branches:
        assert all(name in branch.kw["required"] for name in family)
    # Presences and views are interchangeable, and one Variant is its own family.
    assert family_base_fields([presence(v) for v in node.views]) == family
    assert family_base_fields([node.views[0]]) == presence(node.views[0]).base_fields


def test_family_base_fields_check_their_input() -> None:
    with pytest.raises(ValueError, match="at least one Variant"):
        family_base_fields([])
    with pytest.raises(ValueError, match="one ObservationStore"):
        family_base_fields([root(USERS), root(ITEMS)])
    a = root([{"x": 1, "y": 1}, {"x": 2}])
    b = a.child_key("y")  # not an object view
    with pytest.raises(ValueError, match="no non-flagged object"):
        family_base_fields([a, b])


def test_disjoint_variants_have_no_family_base_fields() -> None:
    store = build_store([{"a": 1}, {"a": 2}, {"b": 1}])
    objects = View.root(store)
    left = objects.subset(np.array([True, True, False]))
    right = objects.subset(np.array([False, False, True]))
    assert family_base_fields([left, right]) == ()
    assert presence(left).required == ("a",) and presence(right).required == ("b",)
