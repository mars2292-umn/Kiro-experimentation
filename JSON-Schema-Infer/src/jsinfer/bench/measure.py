"""Per-run size measures (Requirement 35.7): schema size and description length.

:func:`schema_size` counts the JSON Schema keyword occurrences of an emitted
document, ``$defs`` included: every member name of every schema object,
walking the values of subschema-valued keywords and the entries of
``properties``, ``patternProperties``, ``$defs``, ``definitions``,
``dependentSchemas``, and the schema arrays. :func:`description_length`
is the MDL_Scorer's description length in bits of a Schema_Model on its
training Document_Collection: the model bits of the root plus the data
bits of the root view of the collection's Observations.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from typing import Any, Final

from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import build_store
from jsinfer.core.schema_model import SchemaModel
from jsinfer.core.views import View
from jsinfer.mdl.scorer import MDLScorer

__all__ = ["description_length", "schema_size"]

_SINGLE: Final = frozenset({"additionalProperties", "items", "contains", "propertyNames", "not", "if", "then", "else", "unevaluatedProperties", "unevaluatedItems", "additionalItems"})
_MANY: Final = frozenset({"prefixItems", "allOf", "anyOf", "oneOf"})
_NAMED: Final = frozenset({"properties", "patternProperties", "$defs", "definitions", "dependentSchemas"})


def schema_size(document: Any) -> int:
    """The number of keyword occurrences in ``document`` (35.7); a boolean schema has none."""
    count = 0
    stack: list[Any] = [document]
    while stack:
        node = stack.pop()
        if not isinstance(node, dict):
            continue
        count += len(node)
        for keyword, value in node.items():
            if keyword in _SINGLE:
                stack.append(value)
            elif keyword in _MANY and isinstance(value, list):
                stack.extend(value)
            elif keyword in _NAMED and isinstance(value, dict):
                stack.extend(value.values())
            elif keyword == "items" and isinstance(value, list):
                stack.extend(value)
    return count


def description_length(model: SchemaModel, documents: Iterable[JSON] | Sequence[JSON]) -> float | None:
    """``DL`` in bits of ``model`` on ``documents`` (35.7), or None when the scorer cannot code the schema."""
    store = build_store(documents)
    try:
        return MDLScorer(root=model.root).score(model.root, View.root(store)).total
    except (ValueError, TypeError, KeyError):
        return None
