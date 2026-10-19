"""Shared Judgment_Call featurizers of the Trainer and the Inferrer (Requirements 15.2, 22.10).

One featurizer per Judgment_Call kind computes the Decision_Model feature
vector at a Judgment_Call from the Observations the Inferrer judges and the
two scored candidates. The same functions run at inference (the
``features`` callable the analyzers pass to the DecisionEngine) and in the
Trainer's oracle featurization, so both compute identical features for an
identical Document_Collection (22.10, Property 35).

Vectors (design, "Features per Judgment_Call kind"), in :data:`FEATURE_SPEC`
order; every kind ends with the normalized MDL difference ``(DL(first) −
DL(second)) / n_scored`` of its two candidates and the hashed name tokens of
the field and its parent (:data:`HASH_BUCKETS` buckets each: the count of
the normalized tokens of the last Key step of the Path, and of the Key step
before it, whose 32-bit BLAKE2b key falls in each bucket):

Record-or-Map (``record_map_features``)
    objects; distinct keys; keys per object mean and standard deviation;
    key-frequency entropy in bits; singleton-key fraction ``f1 / #keys``;
    ``Chao1(keys) / #keys``; the fractions of keys that look like integers,
    UUIDs, dates, and hex strings; key length mean and standard deviation;
    the JSON-type entropy of the member values; and the mean key-set Jaccard
    similarity of a seeded sample of object pairs (``rng("features.jaccard",
    path)``, at most :data:`JACCARD_PAIRS` pairs).
Closed-or-Open-Record (``closed_record_features``)
    objects; keys; ``f1`` and ``f2`` of the key occurrences; the Good–Turing
    unseen-key mass; mean optional-key presence; Path depth; and the key-set
    stability between the first and second halves of the objects in
    canonical order (the Jaccard similarity of the key sets seen in each).
Tuple-or-List (``tuple_list_features``)
    arrays; length mean, standard deviation, and distinct-length ratio;
    modal-length share; positional type entropy minus pooled type entropy;
    distinct element JSON types; maximum length.
Closed-or-Open-Enum (``closed_enum_features``)
    the six statistics of 15.1 in ``EnumStats.vector`` order (``n``,
    ``s_obs``, ``f1``, ``f2``, Good–Turing, Chao1); ``s_obs / n``;
    ``Chao1 / s_obs``; the JSON type rank of the most frequent type; mean
    string length; and the fraction of distinct string values that look like
    identifiers.
Merge-or-Split (``merge_split_features``)
    candidate Variants; minimum, mean, and maximum pairwise key-set Jaccard
    similarity; type agreement on shared keys; group-size entropy in bits;
    and the normalized mutual information between the candidate Variant of
    an object and the value of its best scalar field, computed among the
    objects that hold the field (the maximum over the fields; 0 without a
    scalar field or when a field's holders share one Variant, since mere
    presence is what the key-set similarities measure).

Every feature is a finite float. The vectors depend only on the multiset of
non-flagged Observations in canonical order and on the Path, not on
document order, chunking, or the worker count. :data:`FEATURE_SPEC` records
the version, the feature names per kind, and the class order of each kind,
and is what ``feature_spec.json`` stores (design, Model_Bundle), so a
bundle trained with other featurizers is caught (task 23.1).
"""

from __future__ import annotations

import math
import re
from collections.abc import Callable, Mapping, Sequence
from types import MappingProxyType
from typing import Any, Final

import numpy as np

from jsinfer.core.determinism import rng, stable_hash32
from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.observations import ranges_to_rows
from jsinfer.core.paths import KEY, MAPVAL_STEP, Path
from jsinfer.core.views import View
from jsinfer.corpus.normalize import normalize_name
from jsinfer.decide.engine import JUDGMENT_OUTCOMES, Candidate
from jsinfer.structure.arrays import LengthStats, array_rows, type_entropies
from jsinfer.structure.record_map import KeyStats, object_rows
from jsinfer.structure.variants import VariantGrouping
from jsinfer.values.enums import EnumStats, scalar_view

__all__ = [
    "FEATURE_SPEC",
    "FEATURE_VERSION",
    "FEATURIZERS",
    "HASH_BUCKETS",
    "JACCARD_PAIRS",
    "closed_enum_features",
    "closed_record_features",
    "feature_names",
    "feature_spec_json",
    "merge_split_features",
    "name_token_features",
    "normalized_mdl_difference",
    "record_map_features",
    "tuple_list_features",
]

FEATURE_VERSION: Final = "1"
HASH_BUCKETS: Final = 8
JACCARD_PAIRS: Final = 64

_N_JTYPES: Final = len(JType)
_INTEGER_KEY_RE: Final = re.compile(r"-?[0-9]+")
_UUID_KEY_RE: Final = re.compile(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
_DATE_KEY_RE: Final = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}(?:T.*)?")
_HEX_KEY_RE: Final = re.compile(r"[0-9a-fA-F]{4,}")
_IDENTIFIER_RE: Final = re.compile(r"[A-Za-z_][A-Za-z0-9_\-.]*")


# ---------------------------------------------------------------------------
# Shared pieces
# ---------------------------------------------------------------------------


def normalized_mdl_difference(candidates: Sequence[Candidate]) -> float:
    """``(DL(first) − DL(second)) / n_scored`` of two scored candidates: the MDL feature of every kind."""
    if len(candidates) != 2:
        raise ValueError(f"a Judgment_Call has two candidates, got {len(candidates)}")
    first, second = (candidate.score for candidate in candidates)
    if first is None or second is None:
        raise ValueError("Judgment_Call features need the scores of both candidates")
    return (first.total - second.total) / max(first.n_scored, 1)


def _bucket_counts(tokens: Sequence[str]) -> list[float]:
    counts = [0.0] * HASH_BUCKETS
    for token in tokens:
        counts[stable_hash32(token) % HASH_BUCKETS] += 1.0
    return counts


def name_token_features(path: Path) -> list[float]:
    """Hashed normalized-token counts of the field (the last Key step) and its parent (the Key step before it)."""
    keys = [step.key for step in path if step.kind == KEY]
    field = normalize_name(keys[-1]) if keys else ()
    parent = normalize_name(keys[-2]) if len(keys) >= 2 else ()
    return _bucket_counts(field) + _bucket_counts(parent)


def _entropy_bits(counts: Sequence[int] | np.ndarray) -> float:
    positive = [int(count) for count in np.asarray(counts).tolist() if count]
    n = sum(positive)
    if not n:
        return 0.0
    return max(0.0, math.log2(n) - math.fsum(count * math.log2(count) for count in positive) / n)


def _mean_std(values: Sequence[float]) -> tuple[float, float]:
    if not values:
        return 0.0, 0.0
    mean = math.fsum(values) / len(values)
    variance = math.fsum((value - mean) ** 2 for value in values) / len(values)
    return mean, math.sqrt(variance)


def _finish(kind: str, body: Sequence[float], candidates: Sequence[Candidate], path: Path) -> np.ndarray:
    vector = [*body, normalized_mdl_difference(candidates), *name_token_features(path)]
    out = np.asarray(vector, dtype=np.float64)
    if out.shape != (len(FEATURE_SPEC[kind]),):
        raise AssertionError(f"{kind} features have {out.shape[0]} entries, the spec {len(FEATURE_SPEC[kind])}")
    if not np.all(np.isfinite(out)):
        raise ValueError(f"{kind} features are not finite: {out!r}")
    return out


def _presence_matrix(stats: Any) -> np.ndarray:
    """``(objects, keys)`` bool matrix of which keys each object holds."""
    packed = stats.presence  # (keys, bytes)
    if not len(stats.key_ids):
        return np.zeros((stats.n_objects, 0), dtype=bool)
    bits = np.unpackbits(packed, axis=1, bitorder="little")[:, : stats.n_objects]
    return bits.T.astype(bool)


def _pair_jaccard(matrix: np.ndarray, pairs: np.ndarray) -> float:
    if not len(pairs):
        return 1.0
    a = matrix[pairs[:, 0]]
    b = matrix[pairs[:, 1]]
    shared = np.logical_and(a, b).sum(axis=1).astype(np.float64)
    union = np.logical_or(a, b).sum(axis=1).astype(np.float64)
    similarity = np.where(union > 0, shared / np.maximum(union, 1.0), 1.0)
    return float(math.fsum(similarity.tolist()) / len(similarity))


# ---------------------------------------------------------------------------
# Record-or-Map and Closed-or-Open-Record
# ---------------------------------------------------------------------------


def record_map_features(objects: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """The Record-or-Map vector of the non-flagged objects of ``objects`` (see the module docstring)."""
    rows = object_rows(objects)
    stats = rows.stats()
    keys = KeyStats.of(rows)
    store = rows.store
    names = [store.key_name(int(kid)) for kid in stats.key_ids.tolist()]
    n_keys = keys.n_keys
    counts = stats.key_counts
    sizes = stats.object_sizes
    per_object = np.repeat(sizes.values, sizes.counts).astype(np.float64).tolist() if sizes.total else []
    size_mean, size_std = _mean_std(per_object)
    chao1 = EnumStats.from_counts(counts).chao1 if n_keys else 0.0
    lengths = [float(len(name)) for name in names]
    length_mean, length_std = _mean_std(lengths)
    members = rows.child_map().stats() if n_keys else None
    value_type_entropy = _entropy_bits(members.type_counts) if members is not None else 0.0
    matrix = _presence_matrix(stats)
    n_objects = keys.n_objects
    if n_objects >= 2:
        stream = rng("features.jaccard", rows.path)
        pairs = stream.integers(0, n_objects, size=(JACCARD_PAIRS, 2))
        pairs = pairs[pairs[:, 0] != pairs[:, 1]]
        jaccard = _pair_jaccard(matrix, pairs)
    else:
        jaccard = 1.0

    def fraction(pattern: re.Pattern[str]) -> float:
        return sum(1 for name in names if pattern.fullmatch(name)) / n_keys if n_keys else 0.0

    body = [
        float(n_objects),
        float(n_keys),
        size_mean,
        size_std,
        _entropy_bits(counts),
        keys.f1 / n_keys if n_keys else 0.0,
        chao1 / n_keys if n_keys else 0.0,
        fraction(_INTEGER_KEY_RE),
        fraction(_UUID_KEY_RE),
        fraction(_DATE_KEY_RE),
        fraction(_HEX_KEY_RE),
        length_mean,
        length_std,
        value_type_entropy,
        jaccard,
    ]
    return _finish("record_or_map", body, candidates, rows.path)


def closed_record_features(objects: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """The Closed-or-Open-Record vector of the non-flagged objects of ``objects`` (see the module docstring)."""
    rows = object_rows(objects)
    stats = rows.stats()
    keys = KeyStats.of(rows)
    n_objects = keys.n_objects
    counts = stats.key_counts.astype(np.float64)
    optional = counts[counts < n_objects] if n_objects else counts[:0]
    optional_presence = float(optional.mean() / n_objects) if len(optional) else 1.0
    matrix = _presence_matrix(stats)
    half = n_objects // 2
    if half and keys.n_keys:
        first = matrix[:half].any(axis=0)
        second = matrix[half:].any(axis=0)
        union = int(np.logical_or(first, second).sum())
        stability = float(np.logical_and(first, second).sum() / union) if union else 1.0
    else:
        stability = 1.0
    body = [
        float(n_objects),
        float(keys.n_keys),
        float(keys.f1),
        float(keys.f2),
        keys.good_turing,
        optional_presence,
        float(len(rows.path)),
        stability,
    ]
    return _finish("closed_or_open_record", body, candidates, rows.path)


# ---------------------------------------------------------------------------
# Tuple-or-List
# ---------------------------------------------------------------------------


def tuple_list_features(arrays: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """The Tuple-or-List vector of the non-flagged arrays of ``arrays`` (see the module docstring)."""
    rows = array_rows(arrays)
    lengths = LengthStats.of(rows)
    pooled, positional = type_entropies(rows)
    elements = rows.elements()
    kinds = int(np.count_nonzero(np.bincount(elements.gather("jtype").astype(np.int64), minlength=_N_JTYPES)))
    body = [
        float(lengths.n_arrays),
        lengths.mean,
        lengths.std,
        lengths.distinct_ratio,
        lengths.modal_share,
        positional - pooled,
        float(kinds),
        float(lengths.max_length),
    ]
    return _finish("tuple_or_list", body, candidates, rows.path)


# ---------------------------------------------------------------------------
# Closed-or-Open-Enum
# ---------------------------------------------------------------------------


def closed_enum_features(scalars: View, candidates: Sequence[Candidate]) -> np.ndarray:
    """The Closed-or-Open-Enum vector of the non-flagged scalars of ``scalars`` (see the module docstring)."""
    rows = scalar_view(scalars)
    stats = rows.stats()
    if not len(stats.value_counts):
        raise ValueError("Closed-or-Open-Enum features need at least one scalar Observation")
    enum = EnumStats.from_counts(stats.value_counts)
    type_counts = stats.type_counts
    dominant = int(np.argmax(type_counts))
    strings = stats.string_lengths
    mean_length = float(np.dot(strings.values, strings.counts) / strings.total) if strings.total else 0.0
    table = rows.store.values
    distinct_strings = [table[int(vid)] for vid in stats.value_ids.tolist()]
    distinct_strings = [value for value in distinct_strings if isinstance(value, str)]
    identifiers = (
        sum(1 for value in distinct_strings if _IDENTIFIER_RE.fullmatch(value)) / len(distinct_strings)
        if distinct_strings
        else 0.0
    )
    body = [
        *enum.vector().tolist(),
        enum.s_obs / enum.n,
        enum.chao1 / enum.s_obs,
        float(dominant),
        mean_length,
        identifiers,
    ]
    return _finish("closed_or_open_enum", body, candidates, rows.path)


# ---------------------------------------------------------------------------
# Merge-or-Split
# ---------------------------------------------------------------------------

_TYPE_BITS: Final = 3
_TYPE_MASK: Final = (1 << _TYPE_BITS) - 1


def _group_key_types(variants: VariantGrouping) -> list[dict[int, tuple[int, ...]]]:
    signatures = variants.signatures
    out: list[dict[int, tuple[int, ...]]] = []
    for group in variants.grouping.groups:
        types: dict[int, dict[int, None]] = {}
        for s in group:
            for code in signatures.codes[s]:
                types.setdefault(code >> _TYPE_BITS, {})[code & _TYPE_MASK] = None
        out.append({kid: tuple(sorted(types[kid])) for kid in sorted(types)})
    return out


def _scalar_field_information(variants: VariantGrouping) -> float:
    """``max_k I(group; value of k) / H(group)`` over the scalar fields ``k``, among the objects holding ``k``."""
    signatures = variants.signatures
    groups = variants.grouping.groups
    group_of = np.empty(signatures.n_signatures, dtype=np.int64)
    for index, group in enumerate(groups):
        group_of[np.asarray(group, dtype=np.int64)] = index
    object_group = group_of[signatures.assignment]
    counts = np.bincount(object_group, minlength=len(groups))
    h_group = _entropy_bits(counts)
    if h_group == 0.0:
        return 0.0
    objects = signatures.objects
    store = objects.store
    owners: list[np.ndarray] = []
    keys: list[np.ndarray] = []
    values: list[np.ndarray] = []
    offset = 0
    for part in objects.parts:
        members = store.columns.get(part.column + (MAPVAL_STEP,))
        if members is not None:
            lo, hi = members.runs(part.rows)
            rows = ranges_to_rows(lo, hi)
            if len(rows):
                owners.append(np.repeat(np.arange(offset, offset + len(part.rows), dtype=np.int64), hi - lo))
                keys.append(members.key[rows].astype(np.int64))
                values.append(members.value[rows].astype(np.int64))
        offset += len(part.rows)
    if not owners:
        return 0.0
    owner = np.concatenate(owners)
    key = np.concatenate(keys)
    value = np.concatenate(values)
    scalar = value >= 0
    owner, key, value = owner[scalar], key[scalar], value[scalar]
    if not len(owner):
        return 0.0
    best = 0.0
    for kid in np.unique(key).tolist():
        mask = key == kid
        groups_k = object_group[owner[mask]]
        h_holders = _entropy_bits(np.bincount(groups_k, minlength=len(groups)))
        if h_holders == 0.0:
            continue  # the holders share one Variant: presence, not value, separates them
        _, inverse = np.unique(value[mask], return_inverse=True)
        joint = np.zeros((len(groups), int(inverse.max()) + 1), dtype=np.int64)
        np.add.at(joint, (groups_k, inverse), 1)
        h_value = _entropy_bits(joint.sum(axis=0))
        h_joint = _entropy_bits(joint.reshape(-1))
        information = max(0.0, h_holders + h_value - h_joint)
        best = max(best, information / h_holders)
    return min(best, 1.0)


def merge_split_features(variants: VariantGrouping, candidates: Sequence[Candidate]) -> np.ndarray:
    """The Merge-or-Split vector of the candidate Variants of ``variants`` (see the module docstring)."""
    if not isinstance(variants, VariantGrouping):
        raise TypeError(f"variants must be a VariantGrouping, got {type(variants).__name__}")
    groups = variants.grouping.groups
    if len(groups) < 2:
        raise ValueError("Merge-or-Split features need two or more candidate Variants")
    typed = _group_key_types(variants)
    similarities: list[float] = []
    for i in range(len(typed)):
        for j in range(i + 1, len(typed)):
            shared = sum(1 for kid in typed[i] if kid in typed[j])
            union = len(typed[i]) + len(typed[j]) - shared
            similarities.append(shared / union if union else 1.0)
    holders: dict[int, list[tuple[int, ...]]] = {}
    for types in typed:
        for kid, ranks in types.items():
            holders.setdefault(kid, []).append(ranks)
    shared_keys = [ranks for ranks in holders.values() if len(ranks) >= 2]
    agreeing = sum(1 for ranks in shared_keys if all(r == ranks[0] for r in ranks))
    agreement = agreeing / len(shared_keys) if shared_keys else 1.0
    counts = [variants.signatures.count_of(group) for group in groups]
    body = [
        float(len(groups)),
        min(similarities),
        math.fsum(similarities) / len(similarities),
        max(similarities),
        agreement,
        _entropy_bits(counts),
        _scalar_field_information(variants),
    ]
    return _finish("merge_or_split", body, candidates, variants.path)


# ---------------------------------------------------------------------------
# The feature spec
# ---------------------------------------------------------------------------


def _token_names() -> list[str]:
    return [f"field_token_bucket_{i}" for i in range(HASH_BUCKETS)] + [
        f"parent_token_bucket_{i}" for i in range(HASH_BUCKETS)
    ]


_BODY_NAMES: Final[Mapping[str, tuple[str, ...]]] = MappingProxyType(
    {
        "record_or_map": (
            "n_objects",
            "n_keys",
            "keys_per_object_mean",
            "keys_per_object_std",
            "key_frequency_entropy",
            "singleton_key_fraction",
            "chao1_key_ratio",
            "integer_key_fraction",
            "uuid_key_fraction",
            "date_key_fraction",
            "hex_key_fraction",
            "key_length_mean",
            "key_length_std",
            "value_type_entropy",
            "pairwise_keyset_jaccard",
        ),
        "closed_or_open_record": (
            "n_objects",
            "n_keys",
            "f1",
            "f2",
            "good_turing_unseen_mass",
            "optional_key_presence",
            "depth",
            "keyset_stability",
        ),
        "tuple_or_list": (
            "n_arrays",
            "length_mean",
            "length_std",
            "distinct_length_ratio",
            "modal_length_share",
            "positional_minus_pooled_type_entropy",
            "distinct_element_types",
            "max_length",
        ),
        "closed_or_open_enum": (
            "n",
            "s_obs",
            "f1",
            "f2",
            "good_turing",
            "chao1",
            "s_obs_ratio",
            "chao1_ratio",
            "dominant_type_rank",
            "string_length_mean",
            "identifier_fraction",
        ),
        "merge_or_split": (
            "n_variants",
            "keyset_jaccard_min",
            "keyset_jaccard_mean",
            "keyset_jaccard_max",
            "shared_key_type_agreement",
            "group_size_entropy",
            "scalar_field_information",
        ),
    }
)

FEATURE_SPEC: Final[Mapping[str, tuple[str, ...]]] = MappingProxyType(
    {kind: (*names, "normalized_mdl_difference", *_token_names()) for kind, names in _BODY_NAMES.items()}
)
assert tuple(FEATURE_SPEC) == tuple(JUDGMENT_OUTCOMES)

FEATURIZERS: Final[Mapping[str, Callable[..., np.ndarray]]] = MappingProxyType(
    {
        "record_or_map": record_map_features,
        "closed_or_open_record": closed_record_features,
        "tuple_or_list": tuple_list_features,
        "closed_or_open_enum": closed_enum_features,
        "merge_or_split": merge_split_features,
    }
)


def feature_names(kind: str) -> tuple[str, ...]:
    """The feature names of Judgment_Call ``kind``, in vector order."""
    if kind not in FEATURE_SPEC:
        raise ValueError(f"{kind!r} is not a Judgment_Call kind; the kinds are {list(FEATURE_SPEC)}")
    return FEATURE_SPEC[kind]


def feature_spec_json() -> dict[str, JSON]:
    """The ``feature_spec.json`` content: version, per-kind feature names and dimension, and class order."""
    return {
        "version": FEATURE_VERSION,
        "hash_buckets": HASH_BUCKETS,
        "kinds": {
            kind: {"dim": len(names), "features": list(names), "classes": list(JUDGMENT_OUTCOMES[kind])}
            for kind, names in FEATURE_SPEC.items()
        },
    }
