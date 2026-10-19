"""Mutation_Operators and Negative_Document generation (task 26.1; Requirement 32).

Operators (32.1)
    :data:`OPERATORS` lists the fourteen Mutation_Operators by name with their
    applicable site. :func:`sites` enumerates the applicable sites of one
    operator in a ground-truth schema (draft 2020-12 JSON, as the
    Schema_Emitter writes it), each a JSON Pointer into the schema, and
    :func:`mutate` applies the operator at one site, returning a new schema
    that differs from the original only at that site's schema object (32.2;
    design, Mutations):

    ==========================  ==========================================  =================================================
    Operator                    Site                                        Violation form at the site
    ==========================  ==========================================  =================================================
    ``remove_required_field``   ``.../required/<i>``                        the field's property becomes ``false``, the entry is removed
    ``add_undeclared_property`` object schema with ``additionalProperties: false``  a fresh property is declared and required
    ``change_type``             ``.../type``                                a disjoint type set (``integer`` and ``number`` excluded together)
    ``enum_non_member``         ``.../enum`` or ``.../const``               ``const`` of a fresh value of the first member's type
    ``violate_pattern``         ``.../pattern``                             ``not: {pattern: p}``
    ``violate_format``          ``.../format``                              ``type: string`` and ``not: {format: f}``
    ``violate_numeric_bound``   a numeric bound keyword                     the bound flipped (``maximum: m`` → ``exclusiveMinimum: m``, ...); ``multipleOf: k`` → ``not: {multipleOf: k}``
    ``violate_length_bound``    ``minLength``/``maxLength``                 ``maxLength: L` → ``minLength: L+1``; ``minLength: L`` (L ≥ 1) → ``maxLength: L-1``
    ``violate_count_bound``     ``minItems``/``maxItems``/``minProperties``/``maxProperties``  the analogues of the length bounds
    ``reorder_tuple``           ``.../prefixItems`` with two differing positions  the first two differing entries swapped
    ``mismatch_discriminator``  a Tagged_Union branch's Discriminator        the branch's ``const`` → the next branch's value
    ``break_cooccurrence``      ``.../dependentRequired/<a>``               ``required: [a]`` and ``not: {required: [b…]}``
    ``combine_exclusive``       ``.../not`` over a ``required`` set, or a ``oneOf`` of field groups  the members required together
    ``duplicate_element``       ``.../uniqueItems`` (true)                  ``not: {uniqueItems: true}`` (and ``minItems: 2``)
    ==========================  ==========================================  =================================================

    A ``not`` added next to an existing ``not`` goes into ``allOf`` so the
    original ``not`` stays. A pointer that would be invalid in draft 2020-12
    (``minLength: 0`` cannot be violated; an ``enum`` of nulls has no fresh
    value) is not an applicable site.

Negatives (32.3–32.7, 32.9, 32.10)
    :func:`negatives` runs every operator on one ground-truth schema: it
    enumerates sites (a seeded subset of ``mutations.site_limit`` when
    there are more), builds one mutated schema per site, generates
    candidate documents from it with the Synthetic_Generator, validates each
    against the ground truth with ``format`` asserted, and retains the
    distinct rejected ones (JSON value equality) until
    ``mutations.negatives_per_schema`` are kept or ``mutations.doc_attempts``
    candidates were generated. Each :class:`Negative` carries its operator,
    schema id, and site pointer (32.5). Operators without an applicable site
    are recorded as skips (32.6); operators none of whose mutated schemas
    yields a negative (the generator excluding the mutated schema included)
    are recorded as unproductive with the candidates attempted (32.7). All
    randomness is keyed by schema id, operator, and site, so reruns are
    identical (32.10).
"""

from __future__ import annotations

import copy
from collections.abc import Iterator, Sequence
from dataclasses import dataclass, field
from typing import Any, Final

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import codepoint_key, rng, stable_hash32
from jsinfer.core.jsonvalue import JSON, canonical_key
from jsinfer.core.schema_model import SUBSCHEMA_KEYWORDS, SchemaModel, SchemaNode, join_pointer, split_pointer
from jsinfer.io.validator import CompileError, Validator
from jsinfer.learn.synth import GeneratorConfig, SchemaExcluded, SyntheticGenerator, sample_parameters

__all__ = [
    "OPERATORS",
    "MutatedSchema",
    "MutationError",
    "MutationReport",
    "Negative",
    "holder_of",
    "mutate",
    "negatives",
    "sites",
]

OPERATORS: Final[tuple[str, ...]] = (
    "remove_required_field",
    "add_undeclared_property",
    "change_type",
    "enum_non_member",
    "violate_pattern",
    "violate_format",
    "violate_numeric_bound",
    "violate_length_bound",
    "violate_count_bound",
    "reorder_tuple",
    "mismatch_discriminator",
    "break_cooccurrence",
    "combine_exclusive",
    "duplicate_element",
)

_TYPES: Final = ("null", "boolean", "integer", "number", "string", "array", "object")
_NUMERIC_BOUNDS: Final = ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf")
_LENGTH_BOUNDS: Final = ("minLength", "maxLength")
_COUNT_BOUNDS: Final = ("minItems", "maxItems", "minProperties", "maxProperties")
_FRESH_PROPERTY: Final = "undeclared_property"


class MutationError(ValueError):
    """An operator applied where it is not applicable."""


# ---------------------------------------------------------------------------
# Schema positions
# ---------------------------------------------------------------------------


_NOT_DESCENDED: Final = frozenset({"not", "if"})


def _positions(schema: Any) -> Iterator[tuple[str, dict[str, Any]]]:
    """``(pointer, schema object)`` for every keyword schema object of the document, in pre-order.

    ``not`` and ``if`` subschemas are not entered: a violation planted
    inside a negation or a condition is no violation of the document.
    """
    stack: list[tuple[str, Any]] = [("", schema)]
    while stack:
        pointer, value = stack.pop()
        if not isinstance(value, dict):
            continue
        yield pointer, value
        below: list[tuple[str, Any]] = []
        for keyword, item in value.items():
            shape = SUBSCHEMA_KEYWORDS.get(keyword)
            if shape is None or keyword in _NOT_DESCENDED:
                continue
            if shape == "single":
                below.append((join_pointer(pointer, keyword), item))
            elif shape == "list" and isinstance(item, list):
                below.extend((join_pointer(pointer, keyword, i), child) for i, child in enumerate(item))
            elif shape == "map" and isinstance(item, dict):
                below.extend((join_pointer(pointer, keyword, name), child) for name, child in item.items())
        stack.extend(reversed(below))


def holder_of(schema: Any, site: str) -> str:
    """The pointer of the schema object a site lies in: the longest schema position that properly prefixes ``site``.

    A mutation at ``site`` changes that object and its subtree only (32.2),
    so the original and the mutated schema are equal everywhere else.
    """
    tokens = split_pointer(site)
    positions = {pointer for pointer, _ in _positions(schema)}
    for n in range(len(tokens) - 1, -1, -1):
        candidate = join_pointer("", *tokens[:n])
        if candidate in positions:
            return candidate
    raise MutationError(f"{site!r} lies in no schema object")


def _resolve(schema: Any, pointer: str) -> Any:
    node = schema
    for token in split_pointer(pointer):
        node = node[int(token)] if isinstance(node, list) else node[token]
    return node


def _types_of(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [t for t in value if isinstance(t, str)]
    return []


def _discriminators(obj: dict[str, Any]) -> list[tuple[str, list[int]]]:
    """``(key, branch indices)`` of ``oneOf``/``anyOf`` Tagged_Unions in ``obj``: every branch fixes ``key`` to a distinct value."""
    found: list[tuple[str, list[int]]] = []
    for keyword in ("oneOf", "anyOf"):
        branches = obj.get(keyword)
        if not isinstance(branches, list) or len(branches) < 2:
            continue
        if not all(isinstance(b, dict) and isinstance(b.get("properties"), dict) for b in branches):
            continue
        keys = set.intersection(*(set(b["properties"]) for b in branches))
        for key in sorted(keys, key=codepoint_key):
            values = [_fixed_value(b["properties"][key]) for b in branches]
            if all(v is not _NO_VALUE for v in values) and len({canonical_key(v) for v in values}) == len(values):
                found.append((f"{keyword}:{key}", list(range(len(branches)))))
                break  # one Discriminator per union
    return found


_NO_VALUE: Final = object()


def _fixed_value(schema: Any) -> Any:
    if not isinstance(schema, dict):
        return _NO_VALUE
    if "const" in schema:
        return schema["const"]
    values = schema.get("enum")
    if isinstance(values, list) and len(values) == 1:
        return values[0]
    return _NO_VALUE


def _is_field_group_union(branches: Any) -> bool:
    return (
        isinstance(branches, list)
        and len(branches) >= 2
        and all(isinstance(b, dict) and isinstance(b.get("required"), list) and b["required"] and set(b) <= {"required"} for b in branches)
    )


# ---------------------------------------------------------------------------
# Sites (32.1, 32.6)
# ---------------------------------------------------------------------------


def sites(operator: str, schema: Any) -> list[str]:
    """The applicable sites of ``operator`` in ``schema``, as JSON Pointers in pre-order (32.1)."""
    if operator not in OPERATORS:
        raise ValueError(f"unknown Mutation_Operator {operator!r}; the operators are {list(OPERATORS)}")
    found: list[str] = []
    for pointer, obj in _positions(schema):
        if operator == "remove_required_field":
            required = obj.get("required")
            if isinstance(required, list):
                found.extend(join_pointer(pointer, "required", i) for i, name in enumerate(required) if isinstance(name, str))
        elif operator == "add_undeclared_property":
            if obj.get("additionalProperties") is False and isinstance(obj.get("properties", {}), dict):
                found.append(join_pointer(pointer, "additionalProperties"))
        elif operator == "change_type":
            if _types_of(obj.get("type")) and set(_types_of(obj["type"])) < set(_TYPES):
                found.append(join_pointer(pointer, "type"))
        elif operator == "enum_non_member":
            for keyword in ("enum", "const"):
                if keyword in obj and _fresh_value(obj, keyword) is not _NO_VALUE:
                    found.append(join_pointer(pointer, keyword))
        elif operator == "violate_pattern":
            if isinstance(obj.get("pattern"), str):
                found.append(join_pointer(pointer, "pattern"))
        elif operator == "violate_format":
            if isinstance(obj.get("format"), str):
                found.append(join_pointer(pointer, "format"))
        elif operator == "violate_numeric_bound":
            found.extend(join_pointer(pointer, k) for k in _NUMERIC_BOUNDS if _is_number(obj.get(k)))
        elif operator == "violate_length_bound":
            found.extend(join_pointer(pointer, k) for k in _LENGTH_BOUNDS if _is_count(obj.get(k)) and (k == "maxLength" or obj[k] >= 1))
        elif operator == "violate_count_bound":
            found.extend(join_pointer(pointer, k) for k in _COUNT_BOUNDS if _is_count(obj.get(k)) and (k.startswith("max") or obj[k] >= 1))
        elif operator == "reorder_tuple":
            prefix = obj.get("prefixItems")
            if isinstance(prefix, list) and _differing_pair(prefix) is not None:
                found.append(join_pointer(pointer, "prefixItems"))
        elif operator == "mismatch_discriminator":
            for union, branches in _discriminators(obj):
                keyword, key = union.split(":", 1)
                found.extend(join_pointer(pointer, keyword, i, "properties", key) for i in branches)
        elif operator == "break_cooccurrence":
            dependent = obj.get("dependentRequired")
            if isinstance(dependent, dict):
                found.extend(
                    join_pointer(pointer, "dependentRequired", name)
                    for name in dependent
                    if isinstance(dependent[name], list) and dependent[name]
                )
        elif operator == "combine_exclusive":
            negated = obj.get("not")
            if isinstance(negated, dict) and isinstance(negated.get("required"), list) and len(negated["required"]) >= 2 and set(negated) <= {"required"}:
                found.append(join_pointer(pointer, "not"))
            if _is_field_group_union(obj.get("oneOf")):
                found.append(join_pointer(pointer, "oneOf"))
        elif operator == "duplicate_element":
            if obj.get("uniqueItems") is True:
                found.append(join_pointer(pointer, "uniqueItems"))
    return found


def _is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _is_count(value: Any) -> bool:
    return _is_number(value) and value == int(value) and value >= 0


def _differing_pair(prefix: list[Any]) -> tuple[int, int] | None:
    for i in range(len(prefix)):
        for j in range(i + 1, len(prefix)):
            if canonical_key(prefix[i]) != canonical_key(prefix[j]):
                return i, j
    return None


def _fresh_value(obj: dict[str, Any], keyword: str) -> Any:
    """A value of the first member's JSON type that is no member of the ``enum``/``const`` (``_NO_VALUE`` when none)."""
    members = obj[keyword] if keyword == "enum" else [obj[keyword]]
    if not isinstance(members, list) or not members:
        return _NO_VALUE
    keys = {canonical_key(m) for m in members}
    first = members[0]
    candidates: list[Any]
    if isinstance(first, bool):
        candidates = [not first]
    elif isinstance(first, (int, float)):
        top = max(m for m in members if _is_number(m))
        candidates = [top + 1, top + 2]
    elif isinstance(first, str):
        candidates = [f"{first}-x", "not-a-member", "zz"]
    elif isinstance(first, list):
        candidates = [[*first, "x"], []]
    elif isinstance(first, dict):
        candidates = [{**first, "undeclared": 1}, {}]
    else:
        return _NO_VALUE  # null has one value
    for candidate in candidates:
        if canonical_key(candidate) not in keys:
            return candidate
    return _NO_VALUE


# ---------------------------------------------------------------------------
# Mutations (32.2)
# ---------------------------------------------------------------------------


def _add_not(obj: dict[str, Any], negated: dict[str, Any]) -> None:
    """Add ``not: negated`` to ``obj``, beside an existing ``not`` through ``allOf``."""
    if "not" not in obj:
        obj["not"] = negated
    else:
        obj.setdefault("allOf", []).append({"not": negated})


def mutate(operator: str, schema: Any, site: str) -> dict[str, Any] | bool:
    """A copy of ``schema`` with ``operator`` applied at ``site`` (32.2); ``MutationError`` for an inapplicable site."""
    if site not in sites(operator, schema):
        raise MutationError(f"{operator} is not applicable at {site!r}")
    mutated = copy.deepcopy(schema)
    tokens = split_pointer(site)
    if operator == "remove_required_field":
        obj = _resolve(mutated, join_pointer("", *tokens[:-2]))
        name = obj["required"][int(tokens[-1])]
        obj["required"] = [n for n in obj["required"] if n != name]
        if not obj["required"]:
            del obj["required"]
        obj.setdefault("properties", {})[name] = False
    elif operator == "add_undeclared_property":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        properties = obj.setdefault("properties", {})
        fresh = _FRESH_PROPERTY
        n = 1
        while fresh in properties:
            n += 1
            fresh = f"{_FRESH_PROPERTY}_{n}"
        properties[fresh] = {"type": "string"}
        obj["required"] = [*obj.get("required", []), fresh]
    elif operator == "change_type":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        declared = set(_types_of(obj["type"]))
        if declared & {"integer", "number"}:
            declared |= {"integer", "number"}
        obj["type"] = [t for t in _TYPES if t not in declared]
        for keyword in ("enum", "const"):
            obj.pop(keyword, None)  # values of the old types would contradict the new type
    elif operator == "enum_non_member":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        fresh = _fresh_value(obj, tokens[-1])
        obj.pop("enum", None)
        obj.pop("const", None)
        obj["const"] = fresh
    elif operator == "violate_pattern":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        _add_not(obj, {"pattern": obj.pop("pattern")})
    elif operator == "violate_format":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        _add_not(obj, {"format": obj.pop("format")})
        obj["type"] = "string"
        obj.pop("enum", None)
        obj.pop("const", None)
    elif operator == "violate_numeric_bound":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        keyword = tokens[-1]
        value = obj.pop(keyword)
        if keyword == "multipleOf":
            _add_not(obj, {"multipleOf": value})
        else:
            for other in _NUMERIC_BOUNDS[:4]:
                obj.pop(other, None)
            flipped = {"maximum": "exclusiveMinimum", "minimum": "exclusiveMaximum", "exclusiveMinimum": "maximum", "exclusiveMaximum": "minimum"}
            obj[flipped[keyword]] = value
    elif operator in ("violate_length_bound", "violate_count_bound"):
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        keyword = tokens[-1]
        value = int(obj.pop(keyword))
        partner = keyword.replace("min", "max") if keyword.startswith("min") else keyword.replace("max", "min")
        obj.pop(partner, None)
        obj[partner] = value - 1 if keyword.startswith("min") else value + 1
    elif operator == "reorder_tuple":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        pair = _differing_pair(obj["prefixItems"])
        assert pair is not None
        i, j = pair
        obj["prefixItems"][i], obj["prefixItems"][j] = obj["prefixItems"][j], obj["prefixItems"][i]
    elif operator == "mismatch_discriminator":
        union = _resolve(mutated, join_pointer("", *tokens[:-4]))
        keyword, index, key = tokens[-4], int(tokens[-3]), tokens[-1]
        branches = union[keyword]
        other = branches[(index + 1) % len(branches)]["properties"][key]
        value = _fixed_value(other)
        target = branches[index]["properties"][key]
        target.pop("enum", None)
        target["const"] = value
    elif operator == "break_cooccurrence":
        obj = _resolve(mutated, join_pointer("", *tokens[:-2]))
        name = tokens[-1]
        dependents = obj["dependentRequired"].pop(name)
        if not obj["dependentRequired"]:
            del obj["dependentRequired"]
        obj["required"] = [*[n for n in obj.get("required", []) if n != name], name]
        _add_not(obj, {"required": list(dependents)})
    elif operator == "combine_exclusive":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        if tokens[-1] == "not":
            members = obj.pop("not")["required"]
        else:
            groups = obj.pop("oneOf")
            members = [*groups[0]["required"], *[n for n in groups[1]["required"] if n not in groups[0]["required"]]]
        obj["required"] = [*[n for n in obj.get("required", []) if n not in members], *members]
    elif operator == "duplicate_element":
        obj = _resolve(mutated, join_pointer("", *tokens[:-1]))
        del obj["uniqueItems"]
        _add_not(obj, {"uniqueItems": True})
        obj["minItems"] = max(2, int(obj.get("minItems", 0)))
        if "maxItems" in obj and obj["maxItems"] < 2:
            obj["maxItems"] = 2
    return mutated


# ---------------------------------------------------------------------------
# Negatives (32.3–32.7, 32.9, 32.10)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class MutatedSchema:
    operator: str
    site: str
    schema: dict[str, Any] | bool


@dataclass(frozen=True)
class Negative:
    """A Negative_Document with its label (32.5)."""

    document: JSON
    operator: str
    schema_id: str
    site: str

    def to_json(self) -> dict[str, JSON]:
        return {"document": self.document, "operator": self.operator, "schema_id": self.schema_id, "site": self.site}


@dataclass
class MutationReport:
    """What one schema's mutation run produced and recorded (32.6, 32.7)."""

    schema_id: str
    mutated: list[MutatedSchema] = field(default_factory=list)
    negatives: list[Negative] = field(default_factory=list)
    skipped: list[str] = field(default_factory=list)  # operators without an applicable site
    unproductive: dict[str, int] = field(default_factory=dict)  # operator -> candidates attempted
    attempts: dict[str, int] = field(default_factory=dict)  # operator -> candidates attempted

    def per_operator(self) -> dict[str, int]:
        counts = dict.fromkeys(OPERATORS, 0)
        for negative in self.negatives:
            counts[negative.operator] += 1
        return counts


def _resource(name: str, value: int | None) -> int:
    chosen = RESOURCE_PARAMETERS[name].default if value is None else value
    problem = RESOURCE_PARAMETERS[name].check(chosen)
    if problem is not None:
        raise ValueError(f"{name}: {problem}")
    return int(chosen)  # type: ignore[arg-type]


def negatives(
    schema: dict[str, Any] | bool,
    schema_id: str,
    *,
    seed: int = 0,
    target: int | None = None,
    attempts: int | None = None,
    site_limit: int | None = None,
    operators: Sequence[str] = OPERATORS,
    generator: SyntheticGenerator | None = None,
    generator_config: GeneratorConfig | None = None,
    validator: Validator | None = None,
) -> MutationReport:
    """Mutate ``schema`` with every operator and generate its Negative_Documents (see the module docstring)."""
    if not isinstance(schema_id, str) or not schema_id:
        raise ValueError("schema_id must be a non-empty str")
    target_n = _resource("mutations.negatives_per_schema", target)
    budget = _resource("mutations.doc_attempts", attempts)
    limit = _resource("mutations.site_limit", site_limit)
    unknown = [op for op in operators if op not in OPERATORS]
    if unknown:
        raise ValueError(f"unknown Mutation_Operators {unknown}")
    validator = validator if validator is not None else Validator()
    generator = generator if generator is not None else SyntheticGenerator(validator)
    cfg = generator_config if generator_config is not None else GeneratorConfig(size_min=1, size_max=1, doc_attempts=5)
    report = MutationReport(schema_id)
    try:
        truth = validator.compile(schema, name=schema_id)
    except CompileError as error:
        raise ValueError(f"the ground-truth schema {schema_id!r} cannot be compiled: {error}") from None
    for operator in operators:
        found = sites(operator, schema)
        if not found:
            report.skipped.append(operator)
            continue
        if len(found) > limit:  # 32.9: a seeded subset
            stream = rng("mutations.sites", schema_id, operator, seed=seed)
            chosen = sorted(stream.choice(len(found), size=limit, replace=False).tolist())
            found = [found[i] for i in chosen]
        used = 0
        produced = 0
        for site in found:
            mutated = mutate(operator, schema, site)
            report.mutated.append(MutatedSchema(operator, site, mutated))
            kept, spent = _generate(mutated, schema_id, operator, site, truth, validator, generator, cfg, target_n, budget, seed)
            report.negatives.extend(kept)
            produced += len(kept)
            used += spent
        report.attempts[operator] = used
        if not produced:
            report.unproductive[operator] = used
    return report


def _generate(
    mutated: Any,
    schema_id: str,
    operator: str,
    site: str,
    truth: Any,
    validator: Validator,
    generator: SyntheticGenerator,
    cfg: GeneratorConfig,
    target: int,
    budget: int,
    seed: int,
) -> tuple[list[Negative], int]:
    """Candidates from the mutated schema that the ground truth rejects, distinct, up to ``target`` or ``budget``."""
    mutated_id = f"{schema_id}#{operator}{site}"
    try:
        model = SchemaModel(SchemaNode.from_json(mutated))
    except TypeError:
        return [], 0
    site_seed = stable_hash32([seed, schema_id, operator, site])
    try:
        compiled = validator.compile(mutated, name=mutated_id)
    except CompileError:
        return [], 0  # the generator would exclude it (32.7)
    params = sample_parameters(model, cfg, rng("mutations.params", schema_id, operator, site, seed=seed))
    kept: list[Negative] = []
    seen: set[Any] = set()
    spent = 0
    index = 0
    while spent < budget and len(kept) < target:
        try:
            document, used = generator.document(model, mutated_id, 0, index, cfg, params, site_seed, compiled)
        except SchemaExcluded:
            spent += cfg.doc_attempts
            index += 1
            continue
        spent += used
        index += 1
        if validator.is_valid(truth, document):
            continue
        key = canonical_key(document)
        if key in seen:
            continue
        seen.add(key)
        kept.append(Negative(document, operator, schema_id, site))
    return kept, spent
