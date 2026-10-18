"""Unit tests for values/types.py: scalar ``type`` Constraints with the integer rule (task 12.1, 13.1, 13.2)."""

from __future__ import annotations

from typing import Any

import pytest

from jsinfer.core.jsonvalue import JType
from jsinfer.core.observations import build_store
from jsinfer.core.provenance import Support
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.values.types import SCALAR_JTYPES, ScalarTypes, scalar_type_names, scalar_types, type_names

VALIDATOR = Validator()


def root(docs: list[Any]) -> View:
    return View.root(build_store(docs))


@pytest.mark.parametrize(
    ("values", "names", "counts"),
    [
        # Zero-fraction numbers are integers however they are written (13.2).
        ([1, 1.0, 1e3, -0.0, 2**70], ("integer",), (5,)),
        # One non-integral number makes every number a "number", never both (13.1).
        ([1, 2.5, 3.0], ("number",), (3,)),
        ([0.5], ("number",), (1,)),
        # Booleans are never numbers; emitted order is null, boolean, integer|number, string.
        (["s", True, None, 7, False], ("null", "boolean", "integer", "string"), (1, 2, 1, 1)),
        (["a", 1.5, None, 2], ("null", "number", "string"), (1, 2, 1)),
        (["2024-01-01", "12"], ("string",), (2,)),  # latent types stay strings (13.3)
    ],
)
def test_type_lists_exactly_the_observed_scalar_types(
    values: list[Any], names: tuple[str, ...], counts: tuple[int, ...]
) -> None:
    view = root(values)
    found = scalar_types(view)
    assert found == ScalarTypes(names, counts)
    assert found.n == len(values) and found.support == Support(len(values), len(values))
    assert scalar_type_names(view) == list(names) == found.keyword_value
    assert found.keyword_value is not found.keyword_value  # a fresh list each time
    assert not ("integer" in names and "number" in names)
    compiled = VALIDATOR.compile({"type": found.keyword_value})
    assert all(VALIDATOR.is_valid(compiled, value) for value in values)
    assert scalar_types(view.stats()) == found  # ViewStats works too


def test_count_by_name() -> None:
    found = scalar_types(root([1, 2.5, "x", "y"]))
    assert (found.count("number"), found.count("string"), found.count("integer"), found.count("null")) == (2, 2, 0, 0)


def test_arrays_and_objects_are_not_scalar_types() -> None:
    found = scalar_types(root([[1], {"a": 1}, "s", 3]))
    assert found.names == ("integer", "string") and found.n == 2
    assert scalar_types(root([[1], {}])).is_empty


def test_empty_view_has_no_type() -> None:
    found = scalar_types(root([]))
    assert found.is_empty and found.names == () and found.n == 0
    assert found.support == Support(0, 0)


def test_flagged_values_are_excluded() -> None:
    store = build_store([{"v": 1}, {"v": 2.5}, {"v": "s"}])
    view = View.root(store).child_key("v")
    assert scalar_type_names(view) == ["number", "string"]
    store.flags.flag_documents([1])  # the only non-integral number
    assert scalar_type_names(view) == ["integer", "string"]
    store.flags.flag_documents([0])
    assert scalar_types(view) == ScalarTypes(("string",), (1,))


def test_type_names_from_jtypes() -> None:
    assert type_names([JType.STRING, JType.INTEGER, JType.NULL, JType.INTEGER]) == ["null", "integer", "string"]
    assert type_names([JType.NUMBER, JType.INTEGER, JType.BOOLEAN]) == ["boolean", "number"]
    assert type_names([]) == []
    assert type_names([JType.OBJECT, JType.ARRAY]) == ["array", "object"]
    assert [jtype.keyword for jtype in SCALAR_JTYPES] == ["null", "boolean", "integer", "number", "string"]


def test_rejects_other_inputs() -> None:
    with pytest.raises(TypeError):
        scalar_types([1, 2])  # type: ignore[arg-type]
