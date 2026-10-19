"""The seeded Synthetic_Generator (Requirements 22.1–22.3, 22.7–22.9).

:class:`SyntheticGenerator` turns a Schema_Model into Document_Collections
whose every document the Validator accepts, with the ground-truth labels of
``learn.labels`` attached.

Seeding (22.9)
    Each document's random stream is ``SeedSequence([seed, h(schema_id), k,
    doc_index])`` with ``h`` the 32-bit BLAKE2b key of ``core.determinism``,
    and the collection's parameters come from ``SeedSequence([seed,
    h(schema_id), k])`` spawned with the key ``h("params")``. No stream is
    shared between documents, so the output does not depend on the number
    of workers or on the order in which documents are produced. A failed
    attempt continues the document's stream, so retries differ.

Collection parameters (22.2)
    Per Document_Collection the generator samples, from the distributions
    of the :class:`GeneratorConfig`, the collection size (at least 1), the
    presence rate of each optional field (in [0, 1], keyed by the field's
    rendered Path), the mean array length, and the enum value frequency skew
    (the exponent of a Zipf weighting over the declared ``enum`` order), and
    records them in the :class:`SampledParameters` of the collection.

Value sources (22.3)
    Each scalar is generated from the first applicable source: (a) a
    Format-conformant generator for the ten Formats of 13.4; (b) an
    ``examples``, ``default``, ``enum``, or ``const`` value, first from the
    field's own declarations and otherwise from fields with the same
    normalized name across the schemas the generator was given (a
    :class:`ValueIndex` built from the training and validation splits,
    never from held-out schemas); (c) a value from the type and constraints
    of the subschema. A source applies when it yields a value that
    satisfies the subschema's scalar constraints (type, ``enum``,
    ``const``, bounds, ``multipleOf``, lengths, every ``pattern``, and
    every ``format``). ``pattern`` values come from a sampler over the
    ``re._parser`` tree of the pattern after ECMA-262 translation (``\\d``,
    ``\\w``, and ``\\s`` read as their ASCII classes, named groups as
    Python named groups); patterns the sampler cannot handle fall back to
    rejection sampling of random strings against the Validator.

Structure
    ``allOf`` branches and ``$ref`` targets are merged into the constraints
    of their owner; one branch of each ``anyOf`` and ``oneOf`` is chosen at
    random and merged the same way. ``not``, ``if``/``then``/``else``,
    ``oneOf`` exclusivity, and every constraint the merge does not cover
    are enforced by validating the whole document, as is the final verdict
    on every document (22.7). A ``$ref`` is followed only at Paths of at
    most ``max_depth`` steps (``synth.max_depth``), and reference chains
    that add no step are followed once per chain; beyond the cap the
    generator can only choose other branches or leave optional fields out,
    so recursive schemas terminate. Optional fields and extra elements that
    cannot be generated are dropped, and a required one that cannot be
    generated fails the attempt.

Exclusions (22.8)
    A schema the Validator cannot compile, or one for which some document
    gets no accepted value within ``doc_attempts`` attempts
    (``synth.doc_attempts``), raises :class:`SchemaExcluded` with the schema
    id and the reason; :meth:`SyntheticGenerator.collections` catches it,
    records the exclusion, and continues with the remaining schemas.
"""

from __future__ import annotations

import copy
import math
import re
import re._parser as _sre_parse
from collections import defaultdict
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any, Final

import numpy as np
from joblib import Parallel, delayed

from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import canonical_bytes, codepoint_key, sorted_values, stable_hash32
from jsinfer.core.jsonvalue import JSON, canonical_key, canonical_value, jtype_of
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, Path, key_step, render_path, path_sort_key
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.corpus.normalize import normalize_name, ref_target, walk_paths
from jsinfer.io.validator import CompileError, Validator
from jsinfer.learn.labels import Labels, derive_labels
from jsinfer.values.formats import FORMATS

__all__ = [
    "GeneratedCollection",
    "GeneratorConfig",
    "SampledParameters",
    "SchemaExcluded",
    "SchemaSource",
    "SyntheticGenerator",
    "ValueIndex",
    "collection_digest",
    "sample_parameters",
    "sample_pattern",
]

SchemaSource = tuple[str, SchemaModel, str | None]  # (schema id, model, split tag)

_DEFAULT_MAX_DEPTH: Final[int] = int(RESOURCE_PARAMETERS["synth.max_depth"].default)  # type: ignore[arg-type]
_DEFAULT_DOC_ATTEMPTS: Final[int] = int(RESOURCE_PARAMETERS["synth.doc_attempts"].default)  # type: ignore[arg-type]

_SCALAR_NAMES: Final = ("null", "boolean", "integer", "number", "string")
_ALL_TYPES: Final = (*_SCALAR_NAMES, "array", "object")
_OBJECT_KEYWORDS: Final = (
    "properties",
    "patternProperties",
    "additionalProperties",
    "required",
    "propertyNames",
    "minProperties",
    "maxProperties",
    "dependentRequired",
)
_ARRAY_KEYWORDS: Final = ("prefixItems", "items", "contains", "minItems", "maxItems", "uniqueItems")
_STRING_KEYWORDS: Final = ("pattern", "format", "minLength", "maxLength")
_NUMBER_KEYWORDS: Final = ("minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf")

_WORD_CHARS: Final = "abcdefghijklmnopqrstuvwxyz"
_TEXT_CHARS: Final = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-.é😀"
_KEY_CHARS: Final = "abcdefghijklmnopqrstuvwxyz0123456789_"
_PRINTABLE: Final = "".join(chr(c) for c in range(0x20, 0x7F))
_DIGITS: Final = "0123456789"
_WORD_CLASS: Final = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_"
_SPACE_CLASS: Final = " \t\n\r\f\v"
_HEX: Final = "0123456789abcdef"
_TLDS: Final = ("com", "org", "net", "io", "dev", "example")


class SchemaExcluded(Exception):
    """A schema the generator excludes (22.8): ``schema_id`` and the ``reason``."""

    def __init__(self, schema_id: str, reason: str) -> None:
        super().__init__(schema_id, reason)
        self.schema_id = schema_id
        self.reason = reason

    def __str__(self) -> str:
        return f"schema {self.schema_id} excluded: {self.reason}"


class _Reject(Exception):
    """An attempt cannot produce a value here (an unsatisfiable or depth-capped subschema)."""


# ---------------------------------------------------------------------------
# Configuration, sampled parameters, and results
# ---------------------------------------------------------------------------


def _check_count(name: str, value: Any, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, (int, np.integer)) or value < minimum:
        raise ValueError(f"{name} must be an integer >= {minimum}, got {value!r}")
    return int(value)


def _check_unit(name: str, value: Any) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float, np.number)) or not 0.0 <= float(value) <= 1.0:
        raise ValueError(f"{name} must be a number in [0, 1], got {value!r}")
    return float(value)


def _check_nonneg(name: str, value: Any) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float, np.number)) or not float(value) >= 0.0:
        raise ValueError(f"{name} must be a non-negative number, got {value!r}")
    return float(value)


@dataclass(frozen=True)
class GeneratorConfig:
    """The generator configuration: the distributions of 22.2 and the generation limits.

    ``size_min``/``size_max`` bound the collection size (uniform);
    ``presence_min``/``presence_max`` the per-field presence rate (uniform
    in a subrange of [0, 1]); ``array_length_max`` the mean array length
    (uniform in [0, ``array_length_max``], lengths then Poisson around it);
    ``enum_skew_min``/``enum_skew_max`` the Zipf exponent (uniform).
    ``max_depth`` and ``doc_attempts`` are the ``synth.*``
    Resource_Parameters. ``string_max_length``, ``integer_span``, and
    ``map_keys_max`` bound type-based values.
    """

    size_min: int = 1
    size_max: int = 40
    presence_min: float = 0.0
    presence_max: float = 1.0
    array_length_max: float = 4.0
    enum_skew_min: float = 0.0
    enum_skew_max: float = 2.0
    max_depth: int = _DEFAULT_MAX_DEPTH
    doc_attempts: int = _DEFAULT_DOC_ATTEMPTS
    string_max_length: int = 12
    integer_span: int = 1000
    map_keys_max: int = 4

    def __post_init__(self) -> None:
        object.__setattr__(self, "size_min", _check_count("size_min", self.size_min, 1))
        object.__setattr__(self, "size_max", _check_count("size_max", self.size_max, self.size_min))
        object.__setattr__(self, "presence_min", _check_unit("presence_min", self.presence_min))
        object.__setattr__(self, "presence_max", _check_unit("presence_max", self.presence_max))
        if self.presence_max < self.presence_min:
            raise ValueError("presence_max must be at least presence_min")
        object.__setattr__(self, "array_length_max", _check_nonneg("array_length_max", self.array_length_max))
        object.__setattr__(self, "enum_skew_min", _check_nonneg("enum_skew_min", self.enum_skew_min))
        object.__setattr__(self, "enum_skew_max", _check_nonneg("enum_skew_max", self.enum_skew_max))
        if self.enum_skew_max < self.enum_skew_min:
            raise ValueError("enum_skew_max must be at least enum_skew_min")
        object.__setattr__(self, "max_depth", _check_count("max_depth", self.max_depth, 0))
        object.__setattr__(self, "doc_attempts", _check_count("doc_attempts", self.doc_attempts, 1))
        object.__setattr__(self, "string_max_length", _check_count("string_max_length", self.string_max_length, 1))
        object.__setattr__(self, "integer_span", _check_count("integer_span", self.integer_span, 1))
        object.__setattr__(self, "map_keys_max", _check_count("map_keys_max", self.map_keys_max, 0))

    def to_json(self) -> dict[str, JSON]:
        return {name: getattr(self, name) for name in self.__dataclass_fields__}


@dataclass(frozen=True)
class SampledParameters:
    """The parameters sampled for one Document_Collection (22.2), recorded in the training manifest."""

    size: int
    presence_rates: Mapping[str, float]  # rendered Path of each optional field -> rate in [0, 1]
    array_length_mean: float
    enum_skew: float

    def to_json(self) -> dict[str, JSON]:
        rates = {path: self.presence_rates[path] for path in sorted(self.presence_rates, key=codepoint_key)}
        return {
            "size": self.size,
            "presence_rates": rates,
            "array_length_mean": self.array_length_mean,
            "enum_skew": self.enum_skew,
        }

    @classmethod
    def from_json(cls, data: Mapping[str, Any]) -> SampledParameters:
        return cls(
            size=_check_count("size", data["size"], 1),
            presence_rates={str(k): _check_unit(f"presence_rates[{k!r}]", v) for k, v in data["presence_rates"].items()},
            array_length_mean=_check_nonneg("array_length_mean", data["array_length_mean"]),
            enum_skew=_check_nonneg("enum_skew", data["enum_skew"]),
        )


@dataclass(frozen=True)
class GeneratedCollection:
    """One generated Document_Collection: documents, sampled parameters, labels, and the source schema's tags."""

    schema_id: str
    k: int
    split: str | None
    documents: tuple[JSON, ...]
    params: SampledParameters
    labels: Labels
    attempts: int  # generation attempts over all documents


# ---------------------------------------------------------------------------
# Value index for source (b) of 22.3
# ---------------------------------------------------------------------------


def _declared_values(node: SchemaNode) -> list[JSON]:
    """The ``examples``, ``default``, ``enum``, and ``const`` values a node declares, in that order."""
    if node.boolean is not None:
        return []
    kw = node.kw
    values: list[JSON] = []
    examples = kw.get("examples")
    if isinstance(examples, list):
        values.extend(examples)
    if "default" in kw:
        values.append(kw["default"])
    enum = kw.get("enum")
    if isinstance(enum, list):
        values.extend(enum)
    if "const" in kw:
        values.append(kw["const"])
    return values


class ValueIndex:
    """Declared scalar values by normalized field name, across the schemas the generator is given."""

    __slots__ = ("_values",)

    def __init__(self, values: Mapping[tuple[str, ...], Sequence[JSON]] | None = None) -> None:
        self._values: dict[tuple[str, ...], tuple[JSON, ...]] = {}
        for name, items in (values or {}).items():
            self._values[tuple(name)] = tuple(sorted_values(_distinct(items)))

    @classmethod
    def build(cls, models: Iterable[tuple[str, SchemaModel]]) -> ValueIndex:
        """Index the ``examples``, ``default``, ``enum``, and ``const`` of every property of ``models``."""
        found: dict[tuple[str, ...], list[JSON]] = defaultdict(list)
        for _, model in models:
            for _, node in model.walk():
                properties = node.kw.get("properties") if node.boolean is None else None
                if not properties:
                    continue
                for name in sorted(properties, key=codepoint_key):
                    values = [v for v in _declared_values(properties[name]) if jtype_of(v).is_scalar]
                    if values:
                        found[normalize_name(name)].extend(values)
        return cls(found)

    def lookup(self, name: tuple[str, ...]) -> tuple[JSON, ...]:
        """The distinct scalar values declared for fields normalized to ``name``, in canonical value order."""
        return self._values.get(tuple(name), ())

    def __len__(self) -> int:
        return len(self._values)

    def names(self) -> list[tuple[str, ...]]:
        return sorted(self._values)


def _distinct(values: Iterable[JSON]) -> list[JSON]:
    seen: dict[Any, None] = {}
    out: list[JSON] = []
    for value in values:
        try:
            key = canonical_key(value)
        except (TypeError, ValueError):
            continue
        if key in seen:
            continue
        seen[key] = None
        out.append(value)
    return out


# ---------------------------------------------------------------------------
# Format generators (source (a) of 22.3)
# ---------------------------------------------------------------------------


def _pad(n: int, width: int = 2) -> str:
    return str(int(n)).zfill(width)


def _gen_date(rng: np.random.Generator) -> str:
    year = int(rng.integers(1970, 2070))
    month = int(rng.integers(1, 13))
    days = (31, 29 if (year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)) else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)
    day = int(rng.integers(1, days[month - 1] + 1))
    return f"{year:04d}-{_pad(month)}-{_pad(day)}"


def _gen_offset(rng: np.random.Generator) -> str:
    if rng.random() < 0.5:
        return "Z"
    sign = "+" if rng.random() < 0.5 else "-"
    return f"{sign}{_pad(rng.integers(0, 13))}:{_pad(rng.choice([0, 30, 45]))}"


def _gen_time(rng: np.random.Generator) -> str:
    base = f"{_pad(rng.integers(0, 24))}:{_pad(rng.integers(0, 60))}:{_pad(rng.integers(0, 60))}"
    if rng.random() < 0.3:
        base += "." + _pad(rng.integers(0, 1000), 3)
    return base + _gen_offset(rng)


def _gen_date_time(rng: np.random.Generator) -> str:
    return f"{_gen_date(rng)}T{_gen_time(rng)}"


def _gen_duration(rng: np.random.Generator) -> str:
    """An RFC 3339 Appendix A duration: ``P<n>W``, or date units Y[M[D]] and/or time units H[M[S]] in one run each."""
    if rng.random() < 0.2:
        return f"P{int(rng.integers(1, 53))}W"
    date_chain, time_chain = "YMD", "HMS"
    date_units = date_chain[int(rng.integers(0, 3)) :][: int(rng.integers(1, 4))] if rng.random() < 0.7 else ""
    time_units = time_chain[int(rng.integers(0, 3)) :][: int(rng.integers(1, 4))] if rng.random() < 0.7 else ""
    if not date_units and not time_units:
        time_units = "S"
    text = "P" + "".join(f"{int(rng.integers(1, 60))}{u}" for u in date_units)
    if time_units:
        text += "T" + "".join(f"{int(rng.integers(1, 60))}{u}" for u in time_units)
    return text


def _gen_word(rng: np.random.Generator, low: int = 1, high: int = 8, chars: str = _WORD_CHARS) -> str:
    n = int(rng.integers(low, high + 1))
    return "".join(rng.choice(list(chars), size=n)) if n else ""


def _gen_hostname(rng: np.random.Generator) -> str:
    labels = [_gen_word(rng, 1, 10, _WORD_CHARS + _DIGITS) for _ in range(int(rng.integers(1, 4)))]
    labels.append(str(rng.choice(_TLDS)))
    return ".".join(labels)


def _gen_email(rng: np.random.Generator) -> str:
    local = _gen_word(rng, 1, 10, _WORD_CHARS + _DIGITS + ".-_+")
    local = local.strip(".") or "a"
    return f"{local}@{_gen_hostname(rng)}"


def _gen_ipv4(rng: np.random.Generator) -> str:
    return ".".join(str(int(x)) for x in rng.integers(0, 256, size=4))


def _gen_ipv6(rng: np.random.Generator) -> str:
    if rng.random() < 0.2:
        return "::1" if rng.random() < 0.5 else "::"
    groups = ["".join(rng.choice(list(_HEX), size=int(rng.integers(1, 5)))) for _ in range(8)]
    return ":".join(groups)


def _gen_uri(rng: np.random.Generator) -> str:
    scheme = str(rng.choice(["https", "http", "ftp", "urn"]))
    if scheme == "urn":
        return f"urn:{_gen_word(rng)}:{_gen_word(rng, 1, 12, _WORD_CHARS + _DIGITS)}"
    text = f"{scheme}://{_gen_hostname(rng)}"
    for _ in range(int(rng.integers(0, 3))):
        text += "/" + _gen_word(rng, 1, 8, _WORD_CHARS + _DIGITS)
    if rng.random() < 0.3:
        text += f"?{_gen_word(rng)}={_gen_word(rng, 1, 6, _WORD_CHARS + _DIGITS)}"
    if rng.random() < 0.2:
        text += "#" + _gen_word(rng)
    return text


def _gen_uuid(rng: np.random.Generator) -> str:
    hexes = "".join(rng.choice(list(_HEX), size=32))
    variant = str(rng.choice(list("89ab")))
    return f"{hexes[:8]}-{hexes[8:12]}-4{hexes[13:16]}-{variant}{hexes[17:20]}-{hexes[20:32]}"


FORMAT_GENERATORS: Final[Mapping[str, Callable[[np.random.Generator], str]]] = {
    "date-time": _gen_date_time,
    "date": _gen_date,
    "time": _gen_time,
    "duration": _gen_duration,
    "email": _gen_email,
    "hostname": _gen_hostname,
    "ipv4": _gen_ipv4,
    "ipv6": _gen_ipv6,
    "uri": _gen_uri,
    "uuid": _gen_uuid,
}
assert tuple(FORMAT_GENERATORS) == FORMATS


# ---------------------------------------------------------------------------
# Pattern sampler over re._parser (ECMA-262 translated)
# ---------------------------------------------------------------------------

_NAMED_GROUP_RE: Final = re.compile(r"\(\?<(?![=!])")
_MAX_UNBOUNDED_EXTRA: Final = 3


class _Unsupported(Exception):
    """A pattern construct the sampler does not handle (rejection sampling takes over)."""


def _category_chars(category: Any, negate: bool = False) -> str:
    name = str(category)
    if name.endswith("CATEGORY_DIGIT"):
        chars = _DIGITS
    elif name.endswith("CATEGORY_NOT_DIGIT"):
        chars, negate = _DIGITS, not negate
    elif name.endswith("CATEGORY_WORD"):
        chars = _WORD_CLASS
    elif name.endswith("CATEGORY_NOT_WORD"):
        chars, negate = _WORD_CLASS, not negate
    elif name.endswith("CATEGORY_SPACE"):
        chars = _SPACE_CLASS
    elif name.endswith("CATEGORY_NOT_SPACE"):
        chars, negate = _SPACE_CLASS, not negate
    else:
        raise _Unsupported(name)
    if negate:
        return "".join(ch for ch in _PRINTABLE if ch not in chars)
    return chars


def _set_chars(items: Sequence[tuple[Any, Any]]) -> str:
    """The characters of an ``IN`` set, drawn from the printable ASCII alphabet plus the set's own literals."""
    negate = False
    members: list[str] = []
    for op, arg in items:
        name = str(op)
        if name.endswith("NEGATE"):
            negate = True
        elif name.endswith("LITERAL"):
            members.append(chr(arg))
        elif name.endswith("RANGE"):
            lo, hi = arg
            members.extend(chr(c) for c in range(lo, min(hi, lo + 64) + 1))
        elif name.endswith("CATEGORY"):
            members.extend(_category_chars(arg))
        else:
            raise _Unsupported(name)
    if negate:
        excluded = set(members)
        return "".join(ch for ch in _PRINTABLE if ch not in excluded)
    return "".join(dict.fromkeys(members))


def _sample_tree(items: Sequence[tuple[Any, Any]], rng: np.random.Generator, out: list[str]) -> None:
    for op, arg in items:
        name = str(op)
        if name.endswith("LITERAL") and not name.endswith("NOT_LITERAL"):
            out.append(chr(arg))
        elif name.endswith("NOT_LITERAL"):
            choices = [ch for ch in _PRINTABLE if ch != chr(arg)]
            out.append(str(rng.choice(choices)))
        elif name.endswith("ANY"):
            out.append(str(rng.choice([ch for ch in _PRINTABLE])))
        elif name.endswith("IN"):
            chars = _set_chars(arg)
            if not chars:
                raise _Unsupported("empty set")
            out.append(str(rng.choice(list(chars))))
        elif name.endswith("CATEGORY"):
            out.append(str(rng.choice(list(_category_chars(arg)))))
        elif name.endswith("BRANCH"):
            _, branches = arg
            _sample_tree(branches[int(rng.integers(0, len(branches)))], rng, out)
        elif name.endswith("SUBPATTERN"):
            _sample_tree(arg[-1], rng, out)
        elif name.endswith("ATOMIC_GROUP"):
            _sample_tree(arg, rng, out)
        elif name.endswith("REPEAT"):
            low, high, sub = arg
            if high == _sre_parse.MAXREPEAT:
                high = low + _MAX_UNBOUNDED_EXTRA
            count = int(rng.integers(low, high + 1))
            for _ in range(count):
                _sample_tree(sub, rng, out)
        elif name.endswith("AT") or name.endswith("ASSERT") or name.endswith("ASSERT_NOT"):
            continue  # anchors and lookarounds produce no text; the match check catches a violated one
        else:
            raise _Unsupported(name)


_SAMPLER_TRIES: Final = 6


def sample_pattern(pattern: str, rng: np.random.Generator, validator: Validator) -> str | None:
    """A string that matches ``pattern`` (ECMA-262 ``pattern`` keyword), or None when the sampler cannot make one.

    Up to :data:`_SAMPLER_TRIES` samples are checked against the Validator
    (lookarounds and misplaced anchors produce no text, so a sample may
    violate them); backreferences and other unsupported constructs give
    None at once.
    """
    translated = _NAMED_GROUP_RE.sub("(?P<", pattern)
    try:
        tree = list(_sre_parse.parse(translated))
    except (re.error, ValueError, OverflowError, RecursionError):
        return None
    for _ in range(_SAMPLER_TRIES):
        out: list[str] = []
        try:
            _sample_tree(tree, rng, out)
        except _Unsupported:
            return None
        text = "".join(out)
        try:
            if validator.matches_pattern(pattern, text):
                return text
        except CompileError:
            return None
    return None


# ---------------------------------------------------------------------------
# Effective constraints of one position (conjuncts merged)
# ---------------------------------------------------------------------------


@dataclass
class _Effective:
    """The keywords of one position after merging its conjuncts (see the module docstring)."""

    types: frozenset[str] | None = None
    enum: list[JSON] | None = None
    formats: list[str] = field(default_factory=list)
    patterns: list[str] = field(default_factory=list)
    minimum: float | None = None
    maximum: float | None = None
    exclusive_minimum: float | None = None
    exclusive_maximum: float | None = None
    multiple_of: list[float] = field(default_factory=list)
    min_length: int = 0
    max_length: int | None = None
    min_items: int = 0
    max_items: int | None = None
    unique_items: bool = False
    min_properties: int = 0
    max_properties: int | None = None
    properties: dict[str, list[SchemaNode]] = field(default_factory=dict)
    required: set[str] = field(default_factory=set)
    additional: list[SchemaNode] = field(default_factory=list)
    pattern_properties: dict[str, list[SchemaNode]] = field(default_factory=dict)
    property_names: list[SchemaNode] = field(default_factory=list)
    dependent_required: dict[str, set[str]] = field(default_factory=dict)
    prefix_items: list[list[SchemaNode]] = field(default_factory=list)
    items: list[SchemaNode] = field(default_factory=list)
    contains: list[SchemaNode] = field(default_factory=list)
    min_contains: int = 1
    duplicate: bool = False  # a ``not: {uniqueItems: true}`` conjunct: the array must repeat an element
    declared: list[JSON] = field(default_factory=list)  # examples and default, for source (b)
    enum_declared: list[JSON] | None = None  # the first enum, in declared order, for the skew
    keywords: set[str] = field(default_factory=set)

    def has_any(self, names: Sequence[str]) -> bool:
        return any(name in self.keywords for name in names)


def _number(value: Any) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return float(value)


def _count(value: Any) -> int | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0 or value != int(value):
        return None
    return int(value)


def _merge(eff: _Effective, node: SchemaNode) -> None:
    kw = node.kw
    eff.keywords.update(kw)
    declared_type = kw.get("type")
    if isinstance(declared_type, str):
        types = frozenset({declared_type})
    elif isinstance(declared_type, list):
        types = frozenset(name for name in declared_type if isinstance(name, str))
    else:
        types = None
    if types is not None:
        eff.types = types if eff.types is None else eff.types & types
    values: list[JSON] | None = None
    if "const" in kw:
        values = [kw["const"]]
    if isinstance(kw.get("enum"), list):
        values = list(kw["enum"]) if values is None else [v for v in values if _in(v, kw["enum"])]
        if eff.enum_declared is None:
            eff.enum_declared = list(kw["enum"])
    if values is not None:
        eff.enum = values if eff.enum is None else [v for v in eff.enum if _in(v, values)]
    fmt = kw.get("format")
    if isinstance(fmt, str) and fmt not in eff.formats:
        eff.formats.append(fmt)
    pattern = kw.get("pattern")
    if isinstance(pattern, str) and pattern not in eff.patterns:
        eff.patterns.append(pattern)
    minimum = _number(kw.get("minimum"))
    if minimum is not None:
        eff.minimum = minimum if eff.minimum is None else max(eff.minimum, minimum)
    maximum = _number(kw.get("maximum"))
    if maximum is not None:
        eff.maximum = maximum if eff.maximum is None else min(eff.maximum, maximum)
    exclusive_minimum = _number(kw.get("exclusiveMinimum"))
    if exclusive_minimum is not None:
        eff.exclusive_minimum = (
            exclusive_minimum if eff.exclusive_minimum is None else max(eff.exclusive_minimum, exclusive_minimum)
        )
    exclusive_maximum = _number(kw.get("exclusiveMaximum"))
    if exclusive_maximum is not None:
        eff.exclusive_maximum = (
            exclusive_maximum if eff.exclusive_maximum is None else min(eff.exclusive_maximum, exclusive_maximum)
        )
    multiple = _number(kw.get("multipleOf"))
    if multiple is not None and multiple > 0:
        eff.multiple_of.append(multiple)
    for keyword, attr, bigger in (
        ("minLength", "min_length", True),
        ("maxLength", "max_length", False),
        ("minItems", "min_items", True),
        ("maxItems", "max_items", False),
        ("minProperties", "min_properties", True),
        ("maxProperties", "max_properties", False),
    ):
        count = _count(kw.get(keyword))
        if count is None:
            continue
        current = getattr(eff, attr)
        if current is None:
            setattr(eff, attr, count)
        else:
            setattr(eff, attr, max(current, count) if bigger else min(current, count))
    if kw.get("uniqueItems") is True:
        eff.unique_items = True
    negated = kw.get("not")
    if isinstance(negated, SchemaNode) and negated.boolean is None and negated.kw.get("uniqueItems") is True:
        eff.duplicate = True  # mutation negatives (32.1 n): repeat an element
    properties = kw.get("properties")
    if isinstance(properties, dict):
        for name in sorted(properties, key=codepoint_key):
            eff.properties.setdefault(name, []).append(properties[name])
    required = kw.get("required")
    if isinstance(required, list):
        eff.required.update(name for name in required if isinstance(name, str))
    additional = kw.get("additionalProperties")
    if isinstance(additional, SchemaNode):
        eff.additional.append(additional)
    pattern_properties = kw.get("patternProperties")
    if isinstance(pattern_properties, dict):
        for name in sorted(pattern_properties, key=codepoint_key):
            eff.pattern_properties.setdefault(name, []).append(pattern_properties[name])
    property_names = kw.get("propertyNames")
    if isinstance(property_names, SchemaNode):
        eff.property_names.append(property_names)
    dependent = kw.get("dependentRequired")
    if isinstance(dependent, dict):
        for name in sorted(dependent, key=codepoint_key):
            if isinstance(dependent[name], list):
                eff.dependent_required.setdefault(name, set()).update(
                    d for d in dependent[name] if isinstance(d, str)
                )
    prefix = kw.get("prefixItems")
    if isinstance(prefix, list):
        for index, child in enumerate(prefix):
            if index < len(eff.prefix_items):
                eff.prefix_items[index].append(child)
            else:
                eff.prefix_items.append([child])
    items = kw.get("items")
    if isinstance(items, SchemaNode):
        eff.items.append(items)
    contains = kw.get("contains")
    if isinstance(contains, SchemaNode):
        eff.contains.append(contains)
        min_contains = _count(kw.get("minContains"))
        if min_contains is not None:
            eff.min_contains = min(eff.min_contains, min_contains)
    examples = kw.get("examples")
    if isinstance(examples, list):
        eff.declared.extend(examples)
    if "default" in kw:
        eff.declared.append(kw["default"])


def _in(value: JSON, values: Sequence[JSON]) -> bool:
    try:
        key = canonical_key(value)
        return any(canonical_key(v) == key for v in values)
    except (TypeError, ValueError):
        return False


# ---------------------------------------------------------------------------
# The generator
# ---------------------------------------------------------------------------

_BRANCH_RETRIES: Final = 3
# Lengths and counts above this are not generated (a document that large is no training example).
_MAX_COUNT: Final = 1000
_SCALAR_RETRIES: Final = 8
_REJECTION_SAMPLES: Final = 24


class _DocumentGenerator:
    """Generates one document from a Schema_Model under one random stream."""

    __slots__ = ("cfg", "index", "model", "params", "rng", "validator")

    def __init__(
        self,
        model: SchemaModel,
        cfg: GeneratorConfig,
        params: SampledParameters,
        index: ValueIndex,
        validator: Validator,
        rng: np.random.Generator,
    ) -> None:
        self.model = model
        self.cfg = cfg
        self.params = params
        self.index = index
        self.validator = validator
        self.rng = rng

    # -- conjuncts -----------------------------------------------------------

    def _resolve(self, ref: str) -> SchemaNode | None:
        pointer = ref_target(ref)
        if pointer is None:
            return None
        try:
            return self.model.resolve(pointer)
        except (KeyError, ValueError):
            return None

    def _effective(self, nodes: Sequence[SchemaNode], path: Path, chain: frozenset[str] = frozenset()) -> _Effective:
        """Merge ``nodes``, their ``allOf`` branches, their ``$ref`` targets, and one chosen branch of each ``anyOf``/``oneOf``."""
        eff = _Effective()
        stack: list[tuple[SchemaNode, frozenset[str]]] = [(node, chain) for node in reversed(nodes)]
        while stack:
            node, active = stack.pop()
            if node.boolean is False:
                raise _Reject
            if node.boolean is not None:
                continue
            _merge(eff, node)
            kw = node.kw
            more: list[tuple[SchemaNode, frozenset[str]]] = []
            ref = kw.get("$ref")
            if isinstance(ref, str):
                pointer = ref_target(ref)
                if pointer is not None and pointer not in active:
                    if len(path) > self.cfg.max_depth:
                        raise _Reject  # beyond the depth cap: only other branches or absences terminate
                    target = self._resolve(ref)
                    if target is not None:
                        more.append((target, active | {pointer}))
            for branch in kw.get("allOf", ()):
                more.append((branch, active))
            for keyword in ("anyOf", "oneOf"):
                branches = kw.get(keyword)
                if branches:
                    more.append((branches[int(self.rng.integers(0, len(branches)))], active))
            stack.extend(reversed(more))
        if eff.types is not None and not eff.types:
            raise _Reject
        if eff.enum is not None and not eff.enum:
            raise _Reject
        return eff

    # -- values ----------------------------------------------------------------

    def generate(self, nodes: Sequence[SchemaNode], path: Path) -> JSON:
        """A value for the position constrained by ``nodes`` at ``path``; raises ``_Reject``."""
        for attempt in range(_BRANCH_RETRIES):
            try:
                return self._value(self._effective(nodes, path), path)
            except _Reject:
                if attempt + 1 == _BRANCH_RETRIES:
                    raise
        raise _Reject  # unreachable

    def _candidate_types(self, eff: _Effective) -> list[str]:
        if eff.types is not None:
            return [name for name in _ALL_TYPES if name in eff.types]
        if eff.enum is not None:
            names = sorted({jtype_of(v).keyword for v in eff.enum})
            return [name for name in _ALL_TYPES if name in names]
        found: list[str] = []
        if eff.has_any(_OBJECT_KEYWORDS):
            found.append("object")
        if eff.has_any(_ARRAY_KEYWORDS):
            found.append("array")
        if eff.has_any(_STRING_KEYWORDS):
            found.append("string")
        if eff.has_any(_NUMBER_KEYWORDS):
            found.extend(("integer", "number"))
        return found or list(_SCALAR_NAMES)

    def _value(self, eff: _Effective, path: Path) -> JSON:
        types = self._candidate_types(eff)
        chosen = types[int(self.rng.integers(0, len(types)))]
        if chosen == "object":
            return self._object(eff, path)
        if chosen == "array":
            return self._array(eff, path)
        return self._scalar(eff, chosen, path)

    # -- scalars (22.3) ----------------------------------------------------------

    def _scalar(self, eff: _Effective, chosen: str, path: Path) -> JSON:
        rng = self.rng
        # (a) a Format generator
        if chosen == "string":
            for fmt in eff.formats:
                generator = FORMAT_GENERATORS.get(fmt)
                if generator is None:
                    continue
                for _ in range(_SCALAR_RETRIES):
                    value = generator(rng)
                    if self._scalar_ok(eff, chosen, value):
                        return value
                break
        # (b) declared values: the field's own, then same-name fields of the given schemas
        own = [v for v in eff.declared if jtype_of(v).is_scalar and self._scalar_ok(eff, chosen, v)]
        if own:
            return own[int(rng.integers(0, len(own)))]
        if eff.enum is not None:
            allowed = [v for v in eff.enum if self._scalar_ok(eff, chosen, v)]
            if allowed:
                return self._skewed(allowed, eff.enum_declared or allowed)
        if path and path[-1].kind == 0:
            pool = [v for v in self.index.lookup(normalize_name(path[-1].key)) if self._scalar_ok(eff, chosen, v)]
            if pool:
                return pool[int(rng.integers(0, len(pool)))]
        # (c) the type and constraints
        for _ in range(_SCALAR_RETRIES):
            value = self._typed(eff, chosen)
            if self._scalar_ok(eff, chosen, value):
                return value
        raise _Reject

    def _skewed(self, allowed: Sequence[JSON], declared: Sequence[JSON]) -> JSON:
        """One of ``allowed`` with Zipf weights by rank in the declared ``enum`` order."""
        ranks: list[int] = []
        for value in allowed:
            rank = next((i for i, d in enumerate(declared) if _in(value, [d])), len(declared))
            ranks.append(rank)
        weights = np.array([(rank + 1.0) ** -self.params.enum_skew for rank in ranks], dtype=float)
        weights /= weights.sum()
        return allowed[int(self.rng.choice(len(allowed), p=weights))]

    def _typed(self, eff: _Effective, chosen: str) -> JSON:
        rng = self.rng
        if chosen == "null":
            return None
        if chosen == "boolean":
            return bool(rng.random() < 0.5)
        if chosen in ("integer", "number"):
            return self._number(eff, chosen)
        return self._string(eff)

    def _number(self, eff: _Effective, chosen: str) -> JSON:
        rng = self.rng
        span = float(self.cfg.integer_span)
        low = -span if eff.minimum is None else eff.minimum
        high = span if eff.maximum is None else eff.maximum
        if eff.exclusive_minimum is not None:
            low = max(low, eff.exclusive_minimum)
        if eff.exclusive_maximum is not None:
            high = min(high, eff.exclusive_maximum)
        if low > high:
            raise _Reject
        step = max(eff.multiple_of) if eff.multiple_of else None
        if chosen == "integer" or (step is not None and step == int(step)) or rng.random() < 0.3:
            unit = step if step is not None else 1.0
            lo_k = math.ceil(low / unit)
            hi_k = math.floor(high / unit)
            if lo_k > hi_k:
                raise _Reject
            k = int(rng.integers(lo_k, hi_k + 1))
            value = k * unit
            if value == int(value) and abs(value) < 2**53:
                return int(value)
            return float(value)
        if step is not None:
            lo_k, hi_k = math.ceil(low / step), math.floor(high / step)
            if lo_k > hi_k:
                raise _Reject
            return float(int(rng.integers(lo_k, hi_k + 1)) * step)
        value = float(rng.uniform(low, high))
        return round(value, 3)

    def _string(self, eff: _Effective) -> str:
        rng = self.rng
        if eff.patterns:
            sampled = sample_pattern(eff.patterns[0], rng, self.validator)
            if sampled is not None:
                return sampled
            for _ in range(_REJECTION_SAMPLES):
                text = self._random_text(eff)
                try:
                    if self.validator.matches_pattern(eff.patterns[0], text):
                        return text
                except CompileError:
                    raise _Reject from None
            raise _Reject
        return self._random_text(eff)

    def _random_text(self, eff: _Effective) -> str:
        rng = self.rng
        low = eff.min_length
        if low > _MAX_COUNT:
            raise _Reject
        high = max(low, min(self.cfg.string_max_length, low + self.cfg.string_max_length))
        if eff.max_length is not None:
            high = min(high, eff.max_length)
        if low > high:
            raise _Reject
        n = int(rng.integers(low, high + 1))
        return "".join(rng.choice(list(_TEXT_CHARS), size=n)) if n else ""

    def _scalar_ok(self, eff: _Effective, chosen: str, value: JSON) -> bool:
        jtype = jtype_of(value).keyword
        if jtype == "integer":
            if chosen not in ("integer", "number"):
                return False
        elif jtype != chosen:
            return False
        if eff.enum is not None and not _in(value, eff.enum):
            return False
        if jtype in ("integer", "number"):
            assert isinstance(value, (int, float))
            if eff.minimum is not None and value < eff.minimum:
                return False
            if eff.maximum is not None and value > eff.maximum:
                return False
            if eff.exclusive_minimum is not None and value <= eff.exclusive_minimum:
                return False
            if eff.exclusive_maximum is not None and value >= eff.exclusive_maximum:
                return False
            for step in eff.multiple_of:
                quotient = value / step
                if abs(quotient - round(quotient)) > 1e-9:
                    return False
        elif jtype == "string":
            assert isinstance(value, str)
            if len(value) < eff.min_length or (eff.max_length is not None and len(value) > eff.max_length):
                return False
            try:
                for pattern in eff.patterns:
                    if not self.validator.matches_pattern(pattern, value):
                        return False
            except CompileError:
                return False
            for fmt in eff.formats:
                if not self.validator.satisfies_format(fmt, value):
                    return False
        return True

    # -- objects -----------------------------------------------------------------

    def _presence(self, path: Path) -> float:
        rate = self.params.presence_rates.get(render_path(path))
        return 0.5 if rate is None else rate

    def _key(self, eff: _Effective, taken: Mapping[str, Any]) -> str | None:
        """A fresh member name from ``propertyNames``, a ``patternProperties`` pattern, or random letters."""
        rng = self.rng
        for _ in range(_SCALAR_RETRIES):
            if eff.property_names:
                try:
                    name = self.generate(eff.property_names, ())
                except _Reject:
                    return None
                if not isinstance(name, str):
                    continue
            elif eff.pattern_properties:
                patterns = sorted(eff.pattern_properties, key=codepoint_key)
                sampled = sample_pattern(patterns[int(rng.integers(0, len(patterns)))], rng, self.validator)
                if sampled is None:
                    return None
                name = sampled
            else:
                name = _gen_word(rng, 1, 8, _KEY_CHARS)
            if name not in taken and name not in eff.properties:
                return name
        return None

    def _pattern_nodes(self, eff: _Effective, name: str) -> list[SchemaNode]:
        nodes: list[SchemaNode] = []
        for pattern in sorted(eff.pattern_properties, key=codepoint_key):
            try:
                if self.validator.matches_pattern(pattern, name):
                    nodes.extend(eff.pattern_properties[pattern])
            except CompileError:
                raise _Reject from None
        return nodes

    def _object(self, eff: _Effective, path: Path) -> dict[str, JSON]:
        rng = self.rng
        if eff.min_properties > _MAX_COUNT:
            raise _Reject
        out: dict[str, JSON] = {}
        required = sorted(eff.required, key=codepoint_key)
        for name in sorted(eff.properties, key=codepoint_key):
            child = path + (key_step(name),)
            wanted = name in eff.required or rng.random() < self._presence(child)
            if not wanted:
                continue
            try:
                out[name] = self.generate(eff.properties[name], child)
            except _Reject:
                if name in eff.required:
                    raise
        for name in required:
            if name not in out:
                nodes = eff.properties.get(name) or self._pattern_nodes(eff, name) or eff.additional
                if any(node.boolean is False for node in nodes):
                    raise _Reject
                out[name] = self.generate(nodes, path + (key_step(name),)) if nodes else self._typed(_Effective(), "string")
        # dependentRequired: close over the present members
        changed = True
        while changed:
            changed = False
            for name in sorted(eff.dependent_required, key=codepoint_key):
                if name not in out:
                    continue
                for dependent in sorted(eff.dependent_required[name], key=codepoint_key):
                    if dependent in out:
                        continue
                    nodes = eff.properties.get(dependent) or self._pattern_nodes(eff, dependent) or eff.additional
                    if any(node.boolean is False for node in nodes):
                        raise _Reject
                    out[dependent] = (
                        self.generate(nodes, path + (key_step(dependent),)) if nodes else self._typed(_Effective(), "string")
                    )
                    changed = True
        # extra members: patternProperties and a schema-valued or true additionalProperties
        closed = any(node.boolean is False for node in eff.additional)
        extra_allowed = bool(eff.pattern_properties) or (not closed and (bool(eff.additional) or not eff.properties))
        if extra_allowed:
            if eff.properties and not eff.pattern_properties:
                target = int(rng.integers(0, 2))
            else:
                target = int(rng.integers(0, self.cfg.map_keys_max + 1))
            target = max(target, eff.min_properties - len(out))
            if eff.max_properties is not None:
                target = min(target, eff.max_properties - len(out))
            for _ in range(max(target, 0)):
                name = self._key(eff, out)
                if name is None:
                    break
                nodes = self._pattern_nodes(eff, name)
                if not nodes:
                    if closed:
                        continue
                    nodes = eff.additional
                try:
                    out[name] = self.generate(nodes, path + (MAPVAL_STEP,)) if nodes else self._typed(_Effective(), "string")
                except _Reject:
                    continue
        if len(out) < eff.min_properties:
            raise _Reject
        if eff.max_properties is not None and len(out) > eff.max_properties:
            raise _Reject
        return out

    # -- arrays ------------------------------------------------------------------

    def _array(self, eff: _Effective, path: Path) -> list[JSON]:
        rng = self.rng
        child = path + (ELEM_STEP,)
        items_closed = any(node.boolean is False for node in eff.items)
        prefix_count = len(eff.prefix_items)
        low = eff.min_items
        high = eff.max_items
        if low > _MAX_COUNT:
            raise _Reject
        if items_closed:
            high = prefix_count if high is None else min(high, prefix_count)
        if high is not None and low > high:
            raise _Reject
        extra = int(rng.poisson(self.params.array_length_mean))
        total = max(low, prefix_count + extra)
        if high is not None:
            total = min(total, high)
        out: list[JSON] = []
        for index in range(min(prefix_count, total)):
            out.append(self.generate(eff.prefix_items[index], child))
        while len(out) < total:
            try:
                out.append(self.generate(eff.items, child) if eff.items else self._typed(_Effective(), "string"))
            except _Reject:
                break
        if eff.contains and eff.min_contains > 0:
            element = self.generate([*eff.contains, *eff.items], child)
            if high is not None and len(out) >= high and out:
                out[int(rng.integers(0, len(out)))] = element
            else:
                out.insert(int(rng.integers(0, len(out) + 1)), element)
        if eff.unique_items:
            out = _distinct(out)
        if eff.duplicate and out:
            if high is not None and len(out) >= high:
                out[-1] = copy.deepcopy(out[0])
            else:
                out.append(copy.deepcopy(out[0]))
        if len(out) < low:
            raise _Reject
        return out


# ---------------------------------------------------------------------------
# Parameters and the public generator
# ---------------------------------------------------------------------------


def _optional_fields(model: SchemaModel, max_depth: int) -> list[str]:
    """The rendered Paths of the optional properties of ``model`` (every property not required at its Path)."""
    by_path: dict[Path, list[SchemaNode]] = defaultdict(list)
    for path, node in walk_paths(model, max_depth):
        by_path[path].append(node)
    found: dict[str, None] = {}
    for path in sorted(by_path, key=path_sort_key):
        nodes = by_path[path]
        required: set[str] = set()
        names: dict[str, None] = {}
        for node in nodes:
            if node.boolean is not None:
                continue
            declared = node.kw.get("required")
            if isinstance(declared, list):
                required.update(name for name in declared if isinstance(name, str))
            properties = node.kw.get("properties")
            if isinstance(properties, dict):
                for name in sorted(properties, key=codepoint_key):
                    names[name] = None
        for name in names:
            if name not in required:
                found[render_path(path + (key_step(name),))] = None
    return list(found)


def _params_seed(seed: int, schema_id: str, k: int) -> np.random.SeedSequence:
    return np.random.SeedSequence([seed, stable_hash32(schema_id), k], spawn_key=(stable_hash32("params"),))


def _document_seed(seed: int, schema_id: str, k: int, doc_index: int) -> np.random.SeedSequence:
    return np.random.SeedSequence([seed, stable_hash32(schema_id), k, doc_index])


def sample_parameters(model: SchemaModel, cfg: GeneratorConfig, rng: np.random.Generator) -> SampledParameters:
    """Sample the collection parameters of 22.2 from ``cfg`` with ``rng``."""
    size = int(rng.integers(cfg.size_min, cfg.size_max + 1))
    array_length_mean = float(rng.uniform(0.0, cfg.array_length_max)) if cfg.array_length_max > 0 else 0.0
    enum_skew = float(rng.uniform(cfg.enum_skew_min, cfg.enum_skew_max))
    rates = {
        path: float(rng.uniform(cfg.presence_min, cfg.presence_max)) for path in _optional_fields(model, cfg.max_depth)
    }
    return SampledParameters(size, rates, array_length_mean, enum_skew)


class SyntheticGenerator:
    """Generates labeled Document_Collections from Schema_Models (see the module docstring).

    ``index`` supplies source (b) of 22.3; build it with :meth:`ValueIndex.build`
    from the training-split and validation-split schemas, never held-out ones.
    """

    def __init__(self, validator: Validator | None = None, index: ValueIndex | None = None) -> None:
        self.validator = validator if validator is not None else Validator()
        self.index = index if index is not None else ValueIndex()

    def _compiled(self, model: SchemaModel, schema_id: str) -> Any:
        try:
            return self.validator.compile(model.root.to_json(), name=schema_id)
        except CompileError as exc:
            raise SchemaExcluded(schema_id, f"uncompilable: {exc.cause}") from exc

    def document(
        self,
        model: SchemaModel,
        schema_id: str,
        k: int,
        doc_index: int,
        cfg: GeneratorConfig,
        params: SampledParameters,
        seed: int,
        compiled: Any = None,
    ) -> tuple[JSON, int]:
        """Document ``doc_index`` of collection ``k`` and the attempts it took; raises :class:`SchemaExcluded` (22.8)."""
        compiled = self._compiled(model, schema_id) if compiled is None else compiled
        rng = np.random.default_rng(_document_seed(seed, schema_id, k, doc_index))
        generator = _DocumentGenerator(model, cfg, params, self.index, self.validator, rng)
        for attempt in range(1, cfg.doc_attempts + 1):
            try:
                value = generator.generate([model.root], ())
            except _Reject:
                continue
            if self.validator.is_valid(compiled, value):
                return canonical_value(value), attempt
        raise SchemaExcluded(
            schema_id, f"no accepted document within {cfg.doc_attempts} attempts (collection {k}, document {doc_index})"
        )

    def documents(
        self,
        model: SchemaModel,
        n: int,
        seed_key: tuple[int, str, int],
        cfg: GeneratorConfig | None = None,
        params: SampledParameters | None = None,
    ) -> Iterator[JSON]:
        """Yield ``n`` documents for ``seed_key = (seed, schema_id, k)``; parameters are sampled when not given."""
        seed, schema_id, k = seed_key
        cfg = cfg if cfg is not None else GeneratorConfig()
        if params is None:
            params = sample_parameters(model, cfg, np.random.default_rng(_params_seed(seed, schema_id, k)))
        compiled = self._compiled(model, schema_id)
        for doc_index in range(n):
            yield self.document(model, schema_id, k, doc_index, cfg, params, seed, compiled)[0]

    def collection(
        self,
        model: SchemaModel,
        schema_id: str,
        k: int,
        cfg: GeneratorConfig,
        seed: int,
        split: str | None = None,
    ) -> GeneratedCollection:
        """Document_Collection ``k`` of ``schema_id``: documents, sampled parameters, and labels (22.1–22.4)."""
        if not isinstance(schema_id, str) or not schema_id:
            raise ValueError("schema_id must be a non-empty str")
        k = _check_count("k", k)
        seed = _check_count("seed", seed)
        compiled = self._compiled(model, schema_id)
        params = sample_parameters(model, cfg, np.random.default_rng(_params_seed(seed, schema_id, k)))
        documents: list[JSON] = []
        attempts = 0
        for doc_index in range(params.size):
            value, used = self.document(model, schema_id, k, doc_index, cfg, params, seed, compiled)
            documents.append(value)
            attempts += used
        labels = derive_labels(model, cfg.max_depth)
        return GeneratedCollection(schema_id, k, split, tuple(documents), params, labels, attempts)

    def collections(
        self,
        schemas: Sequence[SchemaSource],
        cfg: GeneratorConfig,
        seed: int,
        per_schema: int,
        workers: int = 1,
    ) -> tuple[list[GeneratedCollection], list[tuple[str, str]]]:
        """``per_schema`` collections per schema, in schema then ``k`` order, and the ``(schema id, reason)`` exclusions.

        Every collection of an excluded schema is discarded (22.8). With
        ``workers`` above 1 the schemas are generated in joblib processes;
        the output is the same for every worker count (22.9).
        """
        workers = _check_count("workers", workers, 1)
        per_schema = _check_count("per_schema", per_schema, 0)
        ids = [schema_id for schema_id, _, _ in schemas]
        if len(set(ids)) != len(ids):
            raise ValueError("schema ids must be distinct")
        tasks = [
            delayed(_generate_schema)(self.index, model, schema_id, split, cfg, seed, per_schema)
            for schema_id, model, split in schemas
        ]
        results = Parallel(n_jobs=workers)(tasks) if workers > 1 else [task[0](*task[1], **task[2]) for task in tasks]
        collections: list[GeneratedCollection] = []
        excluded: list[tuple[str, str]] = []
        for result in results:
            if isinstance(result, SchemaExcluded):
                excluded.append((result.schema_id, result.reason))
            else:
                collections.extend(result)
        return collections, excluded


def _generate_schema(
    index: ValueIndex,
    model: SchemaModel,
    schema_id: str,
    split: str | None,
    cfg: GeneratorConfig,
    seed: int,
    per_schema: int,
) -> list[GeneratedCollection] | SchemaExcluded:
    """Every collection of one schema (runs in a joblib worker); the exclusion instead of a partial result."""
    generator = SyntheticGenerator(Validator(), index)
    found: list[GeneratedCollection] = []
    try:
        for k in range(per_schema):
            found.append(generator.collection(model, schema_id, k, cfg, seed, split))
    except SchemaExcluded as exc:
        return exc
    return found


def collection_digest(collection: GeneratedCollection) -> bytes:
    """Canonical bytes of a collection's documents, for determinism checks."""
    return canonical_bytes(list(collection.documents))
