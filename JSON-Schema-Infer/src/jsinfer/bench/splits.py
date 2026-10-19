"""The corpus manifest and Schema_Split file models (tasks 23.2 and 25.2; Requirements 21.2, 21.8, 30.4).

Corpus manifest (:class:`CorpusManifest`)
    A JSON file ``{"format": 1, "schemas": [{"id", "source", "path", ...}]}``
    listing the Schema_Corpus: per schema its source identifier ``id``
    (unique), its ``source`` (``schemastore``, ``jsonschemabench``, or
    ``fixture``), and the ``path`` of its schema file relative to the
    manifest's directory. Task 25.1 writes the same model from the pinned
    checkouts, extended with test-file paths and catalog versions, which
    this reader keeps in :attr:`CorpusEntry.extra`. The Trainer reads the
    training-split and validation-split schemas by id from it (design, Open
    Decisions, Corpus manifest and training fixtures).

Schema_Split (:class:`SchemaSplit`)
    A JSON file ``{"format": 1, "id", "seed", "splits": {"train": [...],
    "val": [...], "held_out": [...]}, ...}`` assigning every schema id to one
    split. The optional members of 30.4 (``groups`` mapping ids to
    near-duplicate groups, ``excluded`` entries with reasons, ``cutoff``,
    ``proportions``, ``snapshots``) are kept as given; the split builder of
    task 25.2 writes them. Ids are listed in code-point order in each split
    and no id appears twice.

Both models load strict UTF-8 JSON and raise :class:`SplitFormatError` (a
``ValueError``) naming the file and the problem, and save canonical JSON
through the atomic writer.
"""

from __future__ import annotations

import json
import os
from collections import Counter
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

from jsinfer.core.determinism import atomic_write_bytes, codepoint_key, encode_canonical, sorted_codepoint
from jsinfer.core.jsonvalue import JSON

__all__ = [
    "CORPUS_FORMAT",
    "SOURCES",
    "SPLITS",
    "SPLIT_FORMAT",
    "CorpusEntry",
    "CorpusManifest",
    "SchemaSplit",
    "SplitFormatError",
    "SplitName",
]

CORPUS_FORMAT: Final = 1
SPLIT_FORMAT: Final = 1
SOURCES: Final[tuple[str, ...]] = ("schemastore", "jsonschemabench", "fixture")
SplitName: TypeAlias = Literal["train", "val", "held_out"]
SPLITS: Final[tuple[SplitName, ...]] = ("train", "val", "held_out")


class SplitFormatError(ValueError):
    """A corpus manifest or Schema_Split file that cannot be read; the message names the file and the problem."""


def _read_json(path: Path, what: str) -> Any:
    try:
        data = path.read_bytes()
    except OSError as error:
        raise SplitFormatError(f"{what} {os.fspath(path)!r} cannot be read: {error.strerror or type(error).__name__}") from None
    try:
        return json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SplitFormatError(f"{what} {os.fspath(path)!r} is not valid UTF-8 JSON: {error}") from None


def _object(value: Any, what: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SplitFormatError(f"{what} must be a JSON object")
    return value


def _format(document: dict[str, Any], expected: int, what: str) -> None:
    if document.get("format") != expected:
        raise SplitFormatError(f"{what} format {document.get('format')!r} is not supported; expected {expected}")


def _ids(value: Any, what: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise SplitFormatError(f"{what} must be an array of non-empty strings")
    return tuple(sorted_codepoint(value))


# ---------------------------------------------------------------------------
# Corpus manifest
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class CorpusEntry:
    """One schema of the Schema_Corpus: its id, source, schema file path (relative to the manifest), and extras."""

    id: str
    source: str
    path: str
    extra: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id:
            raise ValueError("a corpus entry id must be a non-empty str")
        if self.source not in SOURCES:
            raise ValueError(f"a corpus entry source must be one of {list(SOURCES)}, got {self.source!r}")
        if not isinstance(self.path, str) or not self.path or self.path.startswith("/"):
            raise ValueError(f"a corpus entry path must be a relative path, got {self.path!r}")

    def to_json(self) -> dict[str, JSON]:
        return {**dict(self.extra), "id": self.id, "source": self.source, "path": self.path}


@dataclass(frozen=True)
class CorpusManifest:
    """The Schema_Corpus listing: entries by id, and the directory paths are relative to."""

    entries: Mapping[str, CorpusEntry]
    root: Path
    extra: Mapping[str, JSON] = field(default_factory=dict)  # omitted files, snapshots, ... (30.1, 30.9)

    @classmethod
    def of(
        cls, entries: Iterable[CorpusEntry], root: str | os.PathLike[str], extra: Mapping[str, JSON] | None = None
    ) -> CorpusManifest:
        found: dict[str, CorpusEntry] = {}
        for entry in entries:
            if entry.id in found:
                raise ValueError(f"the corpus lists schema {entry.id!r} twice")
            found[entry.id] = entry
        ordered = {sid: found[sid] for sid in sorted_codepoint(found)}
        unknown = sorted(member for member in (extra or {}) if member in ("format", "schemas"))
        if unknown:
            raise ValueError(f"extra members {unknown} are written by CorpusManifest itself")
        return cls(MappingProxyType(ordered), Path(root), dict(extra or {}))

    @classmethod
    def load(cls, path: str | os.PathLike[str]) -> CorpusManifest:
        """Read a corpus manifest file; entries' paths resolve against the file's directory."""
        file = Path(path)
        document = _object(_read_json(file, "corpus manifest"), "the corpus manifest")
        _format(document, CORPUS_FORMAT, "corpus manifest")
        listed = document.get("schemas")
        if not isinstance(listed, list):
            raise SplitFormatError("the corpus manifest needs a 'schemas' array")
        entries: list[CorpusEntry] = []
        for index, item in enumerate(listed):
            obj = _object(item, f"schemas[{index}]")
            try:
                entries.append(
                    CorpusEntry(
                        obj.get("id"),  # type: ignore[arg-type]
                        obj.get("source"),  # type: ignore[arg-type]
                        obj.get("path"),  # type: ignore[arg-type]
                        {k: v for k, v in obj.items() if k not in ("id", "source", "path")},
                    )
                )
            except ValueError as error:
                raise SplitFormatError(f"schemas[{index}]: {error}") from None
        try:
            return cls.of(entries, file.parent, {k: v for k, v in document.items() if k not in ("format", "schemas")})
        except ValueError as error:
            raise SplitFormatError(str(error)) from None

    def save(self, path: str | os.PathLike[str]) -> None:
        """Write the manifest as canonical JSON (paths stay as recorded)."""
        document = {
            "format": CORPUS_FORMAT,
            "schemas": [entry.to_json() for entry in self.entries.values()],
            **{k: self.extra[k] for k in sorted(self.extra, key=codepoint_key)},
        }
        atomic_write_bytes(path, encode_canonical(document))

    def ids(self) -> tuple[str, ...]:
        return tuple(self.entries)

    def schema_path(self, schema_id: str) -> Path:
        """The schema file of ``schema_id``; ``KeyError`` for an unknown id."""
        return self.root / self.entries[schema_id].path


# ---------------------------------------------------------------------------
# Schema_Split
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SchemaSplit:
    """A Schema_Split: ids per split, its identifier and seed, and the other persisted members of 30.4."""

    id: str
    seed: int
    train: tuple[str, ...]
    val: tuple[str, ...]
    held_out: tuple[str, ...]
    extra: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id:
            raise ValueError("a Schema_Split id must be a non-empty str")
        if isinstance(self.seed, bool) or not isinstance(self.seed, int) or self.seed < 0:
            raise ValueError("a Schema_Split seed must be a non-negative int")
        for name in SPLITS:
            object.__setattr__(self, name, tuple(sorted_codepoint(getattr(self, name))))
        counts = Counter([*self.train, *self.val, *self.held_out])
        twice = sorted_codepoint([sid for sid, count in counts.items() if count > 1])
        if twice:
            raise ValueError(f"schema ids assigned to more than one split: {twice}")
        unknown = sorted(member for member in self.extra if member in ("format", "id", "seed", "splits"))
        if unknown:
            raise ValueError(f"extra members {unknown} are written by SchemaSplit itself")

    def split_of(self, schema_id: str) -> SplitName | None:
        """The split of ``schema_id``, or None when the split does not assign it."""
        for name in SPLITS:
            if schema_id in getattr(self, name):
                return name
        return None

    def ids(self, split: SplitName) -> tuple[str, ...]:
        if split not in SPLITS:
            raise ValueError(f"split must be one of {list(SPLITS)}, got {split!r}")
        return getattr(self, split)

    @classmethod
    def load(cls, path: str | os.PathLike[str]) -> SchemaSplit:
        file = Path(path)
        document = _object(_read_json(file, "Schema_Split"), "the Schema_Split")
        _format(document, SPLIT_FORMAT, "Schema_Split")
        splits = _object(document.get("splits"), "'splits'")
        try:
            return cls(
                document.get("id"),  # type: ignore[arg-type]
                document.get("seed"),  # type: ignore[arg-type]
                _ids(splits.get("train", []), "splits.train"),
                _ids(splits.get("val", []), "splits.val"),
                _ids(splits.get("held_out", []), "splits.held_out"),
                {k: v for k, v in document.items() if k not in ("format", "id", "seed", "splits")},
            )
        except ValueError as error:
            raise SplitFormatError(f"Schema_Split {os.fspath(file)!r}: {error}") from None

    def to_json(self) -> dict[str, JSON]:
        return {
            "format": SPLIT_FORMAT,
            "id": self.id,
            "seed": self.seed,
            "splits": {"train": list(self.train), "val": list(self.val), "held_out": list(self.held_out)},
            **{k: self.extra[k] for k in sorted(self.extra, key=codepoint_key)},
        }

    def save(self, path: str | os.PathLike[str]) -> None:
        atomic_write_bytes(path, encode_canonical(self.to_json()))


# ---------------------------------------------------------------------------
# The split builder (task 25.2; Requirements 30.2, 30.3, 30.4, 30.5, 30.10)
# ---------------------------------------------------------------------------
#
# Near-duplicate grouping and split assignment (design, Evaluation harness
# details):
#
# 1. Every schema's normalized Path set (``corpus.normalize.normalized_paths``,
#    ``$ref`` expanded to ``nearest.ref_depth``) is MinHashed with
#    ``minhash.perm`` permutations seeded from the split seed; LSH with
#    ``lsh.bands`` bands retrieves candidate pairs, whose exact Jaccard
#    similarity is computed.
# 2. The selected cut-off is where the posterior of the highest-mean component
#    of a 1–3 component Gaussian mixture on the logit of the candidate
#    similarities, selected by BIC, reaches 0.5; it is 1.0 when BIC selects one
#    component or there are too few candidate pairs to fit a mixture. The
#    effective cut-off is the selected one capped at ``split.max_cutoff``, so
#    pairs at or above the cap always group (30.2).
# 3. Union-find over catalog version groups and over pairs at or above the
#    cut-off gives the near-duplicate groups; a group is named by its smallest id.
# 4. Forced groups go to the held-out split first: the groups of the
#    Real_Datasets' known schemas that are in the corpus, and every corpus
#    schema whose similarity to a known schema outside the corpus reaches
#    the cut-off (30.3). The other groups are shuffled by ``rng("split.assign")``,
#    stably sorted by descending size, and each goes to the split with the
#    largest deficit against its target count (ties: train, val, held_out).
# 5. The proportions must each lie in (0, 1) and sum to 1 within 0.001, and
#    each split must receive a schema; otherwise ``SplitError`` and nothing is
#    written (30.10). Each split's realized share is flagged when it deviates
#    from its proportion by more than 0.05 (30.4).
#
# A schema the Validator rejects against its meta-schema, that declares an
# unrecognized dialect, or that the Schema_Loader marks partially supported
# is excluded from synthetic generation with its reason but keeps its group
# and split (30.5); its Path set is then the root alone, so only catalog
# versions group it.

import math  # noqa: E402
import warnings  # noqa: E402
from collections.abc import Sequence  # noqa: E402

import numpy as np  # noqa: E402
from datasketch import MinHash, MinHashLSH  # noqa: E402
from sklearn.mixture import GaussianMixture  # noqa: E402

from jsinfer.config.resources import RESOURCE_PARAMETERS  # noqa: E402
from jsinfer.core.determinism import random_state, rng  # noqa: E402
from jsinfer.core.schema_model import SchemaModel  # noqa: E402
from jsinfer.corpus.normalize import normalized_paths  # noqa: E402
from jsinfer.io.load import SchemaLoadError, SchemaLoader  # noqa: E402

__all__ += ["DEVIATION", "PROPORTION_PARAMETERS", "SplitBuilder", "SplitError", "SplitInputs", "near_duplicate_cutoff"]

PROPORTION_PARAMETERS: Final[tuple[str, ...]] = ("split.train", "split.val", "split.held_out")
DEVIATION: Final = 0.05
_LOGIT_EPS: Final = 1e-6
_MIN_PAIRS: Final = 6  # fewer candidate pairs than this cannot support a mixture; the cut-off is then 1.0
_GRID: Final = 2001


class SplitError(ValueError):
    """The Schema_Split cannot be built; the message lists the offending values (30.10)."""


@dataclass(frozen=True)
class SplitInputs:
    """What a split is derived from (30.4): Path sets, catalog version groups, and the known schemas.

    ``paths`` maps each corpus schema id to its normalized Path set;
    ``excluded`` holds the 30.5 exclusions; ``catalog_groups`` lists ids
    that are versions of one catalog entry; ``known_ids`` are the
    Real_Datasets' known schemas in the corpus and ``known_paths`` the Path
    sets of those outside it, by dataset id; ``snapshots`` records the
    source snapshot versions.
    """

    paths: Mapping[str, frozenset[str]]
    excluded: tuple[tuple[str, str], ...] = ()
    catalog_groups: tuple[tuple[str, ...], ...] = ()
    known_ids: tuple[str, ...] = ()
    known_paths: Mapping[str, frozenset[str]] = field(default_factory=dict)
    snapshots: Mapping[str, str] = field(default_factory=dict)

    @classmethod
    def from_manifest(
        cls,
        manifest: CorpusManifest,
        *,
        known_ids: Iterable[str] = (),
        known_files: Mapping[str, str | os.PathLike[str]] | None = None,
        ref_depth: int | None = None,
        max_paths: int | None = None,
        loader: SchemaLoader | None = None,
    ) -> SplitInputs:
        """Load every corpus schema and known schema file into Path sets, recording the 30.5 exclusions.

        ``ref_depth`` and ``max_paths`` default to ``nearest.ref_depth`` and ``nearest.max_paths``.
        """
        loader = loader if loader is not None else SchemaLoader()
        depth = int(RESOURCE_PARAMETERS["nearest.ref_depth"].default if ref_depth is None else ref_depth)  # type: ignore[arg-type]
        cap = int(RESOURCE_PARAMETERS["nearest.max_paths"].default if max_paths is None else max_paths)  # type: ignore[arg-type]
        paths: dict[str, frozenset[str]] = {}
        excluded: list[tuple[str, str]] = []
        for sid, entry in manifest.entries.items():
            model, reason = _load_for_paths(loader, manifest.schema_path(sid))
            if reason is not None:
                excluded.append((sid, reason))
            paths[sid] = frozenset({"$"}) if model is None else normalized_paths(model, depth, cap)
        groups: dict[str, list[str]] = {}
        for sid, entry in manifest.entries.items():
            group = entry.extra.get("catalog_group")
            if isinstance(group, str) and group:
                groups.setdefault(group, []).append(sid)
        catalog = tuple(tuple(ids) for _, ids in sorted(groups.items(), key=lambda i: codepoint_key(i[0])) if len(ids) >= 2)
        unknown = sorted((sid for sid in known_ids if sid not in manifest.entries), key=codepoint_key)
        if unknown:
            raise SplitError(f"known schemas not in the corpus: {unknown}")
        known_paths: dict[str, frozenset[str]] = {}
        for name, file in sorted((known_files or {}).items(), key=lambda i: codepoint_key(i[0])):
            model, reason = _load_for_paths(loader, Path(file))
            if model is None:
                raise SplitError(f"known schema file {os.fspath(file)!r} of {name!r}: {reason}")
            known_paths[name] = normalized_paths(model, depth, cap)
        snapshots = {}
        for entry in manifest.entries.values():
            snapshot = entry.extra.get("snapshot")
            if isinstance(snapshot, str):
                snapshots.setdefault(entry.source, snapshot)
        return cls(paths, tuple(excluded), catalog, tuple(sorted(set(known_ids), key=codepoint_key)), known_paths, snapshots)


def _load_for_paths(loader: SchemaLoader, file: Path) -> tuple[SchemaModel | None, str | None]:
    """The loaded model, or None with the 30.5 exclusion reason (partially supported models come with a reason too)."""
    try:
        data = file.read_bytes()
    except OSError as error:
        return None, f"unreadable: {error.strerror or type(error).__name__}"
    try:
        model = loader.load(data)
    except SchemaLoadError as error:
        return None, f"{error.check}: {error}" if hasattr(error, "check") else str(error)
    if not model.fully_supported:
        return model, f"partially supported: {[f'{p} {k}' for p, k in model.unsupported][:5]}"
    return model, None


def _sigmoid(x: np.ndarray) -> np.ndarray:
    return 1.0 / (1.0 + np.exp(-x))


def near_duplicate_cutoff(similarities: Sequence[float], seed: int = 0) -> tuple[float, dict[str, JSON]]:
    """The cut-off of 30.2 from candidate-pair ``similarities``, and how it was selected (see the section comment)."""
    values = np.asarray(similarities, dtype=np.float64)
    if np.any((values < 0.0) | (values > 1.0)):
        raise ValueError("similarities must lie in [0, 1]")
    detail: dict[str, JSON] = {"criterion": "BIC", "candidate_pairs": int(values.size)}
    if values.size < _MIN_PAIRS:
        detail.update({"components": 0, "note": f"fewer than {_MIN_PAIRS} candidate pairs; the cut-off is 1.0"})
        return 1.0, detail
    clipped = np.clip(values, _LOGIT_EPS, 1.0 - _LOGIT_EPS)
    logits = np.log(clipped / (1.0 - clipped)).reshape(-1, 1)
    best: tuple[float, int, GaussianMixture] | None = None
    bics: dict[str, float] = {}
    for k in (1, 2, 3):
        if k > values.size:
            break
        mixture = GaussianMixture(n_components=k, random_state=random_state("split.gmm", k, seed=seed), n_init=1)
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")  # duplicate similarities make k-means init warn; BIC still decides
                mixture.fit(logits)
        except ValueError:  # degenerate data (all equal) for k > 1
            continue
        bic = float(mixture.bic(logits))
        bics[str(k)] = bic
        if best is None or bic < best[0]:
            best = (bic, k, mixture)
    assert best is not None
    _, k, mixture = best
    detail["components"] = k
    detail["bic"] = bics
    if k == 1:
        return 1.0, detail
    top = int(np.argmax(mixture.means_.reshape(-1)))
    grid = np.linspace(float(logits.min()), float(logits.max()), _GRID).reshape(-1, 1)
    posterior = mixture.predict_proba(grid)[:, top]
    above = np.flatnonzero(posterior >= 0.5)
    if not len(above):
        return 1.0, {**detail, "note": "the top component never reaches posterior 0.5; the cut-off is 1.0"}
    cutoff = float(_sigmoid(grid[int(above[0]), 0]))
    cutoff = min(max(cutoff, _LOGIT_EPS), 1.0)
    return cutoff, detail


class _UnionFind:
    def __init__(self, items: Iterable[str]) -> None:
        self.parent = {item: item for item in items}

    def find(self, item: str) -> str:
        root = item
        while self.parent[root] != root:
            root = self.parent[root]
        while self.parent[item] != root:
            self.parent[item], item = root, self.parent[item]
        return root

    def union(self, a: str, b: str) -> None:
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            # the smaller id (code-point order) becomes the root, so group names are canonical
            if codepoint_key(rb) < codepoint_key(ra):
                ra, rb = rb, ra
            self.parent[rb] = ra


def _check_proportions(proportions: Mapping[str, float]) -> dict[str, float]:
    problems: list[str] = []
    values: dict[str, float] = {}
    for name in PROPORTION_PARAMETERS:
        value = proportions.get(name, RESOURCE_PARAMETERS[name].default)
        problem = RESOURCE_PARAMETERS[name].check(value)
        if problem is not None:
            problems.append(f"{name}: {problem}")
        else:
            values[name] = float(value)  # type: ignore[arg-type]
    if not problems and abs(math.fsum(values.values()) - 1.0) > 0.001:
        problems.append(f"the proportions sum to {math.fsum(values.values())!r}, not 1 within 0.001")
    if problems:
        raise SplitError("invalid split proportions: " + "; ".join(problems))
    return values


class SplitBuilder:
    """Builds the Schema_Split of a corpus (see the section comment)."""

    def __init__(self, *, perm: int | None = None, bands: int | None = None) -> None:
        self.perm = int(RESOURCE_PARAMETERS["minhash.perm"].default if perm is None else perm)  # type: ignore[arg-type]
        self.bands = int(RESOURCE_PARAMETERS["lsh.bands"].default if bands is None else bands)  # type: ignore[arg-type]
        if not 1 <= self.bands <= self.perm:
            raise ValueError(f"lsh.bands ({self.bands}) must lie in [1, minhash.perm ({self.perm})]")

    def candidate_similarities(self, paths: Mapping[str, frozenset[str]], seed: int) -> dict[tuple[str, str], float]:
        """Exact Jaccard similarity of every LSH candidate pair of Path sets, keyed by ``(smaller id, larger id)``."""
        ids = sorted(paths, key=codepoint_key)
        hash_seed = random_state("split.minhash", seed=seed)
        sketches: dict[str, MinHash] = {}
        for sid in ids:
            sketch = MinHash(num_perm=self.perm, seed=hash_seed)
            sketch.update_batch([p.encode("utf-8", "surrogatepass") for p in sorted(paths[sid], key=codepoint_key)])
            sketches[sid] = sketch
        pairs: set[tuple[str, str]] = set()
        if self.bands == 1:
            buckets: dict[bytes, list[str]] = {}
            for sid in ids:
                buckets.setdefault(np.ascontiguousarray(sketches[sid].hashvalues, dtype="<u8").tobytes(), []).append(sid)
            for members in buckets.values():
                pairs.update((a, b) for i, a in enumerate(members) for b in members[i + 1 :])
        else:
            index = MinHashLSH(num_perm=self.perm, params=(self.bands, self.perm // self.bands))
            for sid in ids:
                index.insert(sid, sketches[sid])
            for sid in ids:
                for other in index.query(sketches[sid]):
                    if other != sid:
                        pairs.add((sid, other) if codepoint_key(sid) < codepoint_key(other) else (other, sid))
        out: dict[tuple[str, str], float] = {}
        for a, b in sorted(pairs, key=lambda p: (codepoint_key(p[0]), codepoint_key(p[1]))):
            union = len(paths[a] | paths[b])
            out[a, b] = len(paths[a] & paths[b]) / union if union else 1.0
        return out

    def build(
        self,
        inputs: SplitInputs,
        *,
        split_id: str,
        seed: int,
        proportions: Mapping[str, float] | None = None,
        max_cutoff: float | None = None,
    ) -> SchemaSplit:
        """The Schema_Split of ``inputs`` (30.2–30.5, 30.10); raises :class:`SplitError` without writing anything.

        ``max_cutoff`` caps the selected cut-off (``split.max_cutoff`` by default).
        """
        if isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
            raise SplitError(f"the seed must be a non-negative integer, got {seed!r}")
        targets = _check_proportions(proportions or {})
        ceiling_spec = RESOURCE_PARAMETERS["split.max_cutoff"]
        ceiling = ceiling_spec.default if max_cutoff is None else max_cutoff
        problem = ceiling_spec.check(ceiling)
        if problem is not None:
            raise SplitError(f"invalid split.max_cutoff: {problem}")
        ids = sorted(inputs.paths, key=codepoint_key)
        if not ids:
            raise SplitError("the corpus holds no schema")
        similarities = self.candidate_similarities(inputs.paths, seed)
        selected, detail = near_duplicate_cutoff(list(similarities.values()), seed)
        cutoff = min(selected, float(ceiling))  # type: ignore[arg-type]
        detail = {**detail, "selected": selected, "max_cutoff": float(ceiling)}  # type: ignore[arg-type]
        groups = _UnionFind(ids)
        for versions in inputs.catalog_groups:
            members = [sid for sid in versions if sid in groups.parent]
            for other in members[1:]:
                groups.union(members[0], other)
        for (a, b), similarity in similarities.items():
            if similarity >= cutoff:
                groups.union(a, b)
        group_of = {sid: groups.find(sid) for sid in ids}
        members: dict[str, list[str]] = {}
        for sid in ids:
            members.setdefault(group_of[sid], []).append(sid)
        # Forced held-out groups (30.3).
        forced: set[str] = {group_of[sid] for sid in inputs.known_ids}
        similar: dict[str, list[str]] = {}
        for name in sorted(inputs.known_paths, key=codepoint_key):
            known = inputs.known_paths[name]
            for sid in ids:
                union = len(known | inputs.paths[sid])
                if (len(known & inputs.paths[sid]) / union if union else 1.0) >= cutoff:
                    forced.add(group_of[sid])
                    similar.setdefault(name, []).append(sid)
        # Assignment (30.2, 30.4).
        assignment: dict[str, str] = {}
        counts = {name: 0 for name in SPLITS}
        total = len(ids)
        for group in sorted(forced, key=codepoint_key):
            for sid in members[group]:
                assignment[sid] = "held_out"
            counts["held_out"] += len(members[group])
        remaining = [group for group in sorted(members, key=codepoint_key) if group not in forced]
        stream = rng("split.assign", seed=seed)
        order = stream.permutation(len(remaining)).tolist()
        shuffled = [remaining[i] for i in order]
        shuffled.sort(key=lambda group: -len(members[group]))  # stable: the shuffle breaks size ties
        target = {name: targets[param] * total for name, param in zip(SPLITS, PROPORTION_PARAMETERS)}
        for group in shuffled:
            deficits = [(target[name] - counts[name], -SPLITS.index(name), name) for name in SPLITS]
            chosen = max(deficits)[2]
            for sid in members[group]:
                assignment[sid] = chosen
            counts[chosen] += len(members[group])
        empty = [name for name in SPLITS if counts[name] == 0]
        if empty:
            raise SplitError(f"the assignment leaves {empty} without a schema (proportions {targets}, {total} schemas)")
        realized = {name: counts[name] / total for name in SPLITS}
        configured = {name: targets[param] for name, param in zip(SPLITS, PROPORTION_PARAMETERS)}
        deviation = {name: abs(realized[name] - configured[name]) > DEVIATION for name in SPLITS}
        extra: dict[str, JSON] = {
            "groups": {sid: group_of[sid] for sid in ids},
            "excluded": [{"id": sid, "reason": reason} for sid, reason in sorted(inputs.excluded, key=lambda e: codepoint_key(e[0]))],
            "cutoff": {"value": cutoff, **detail},
            "proportions": {"configured": configured, "realized": realized, "deviation": deviation},
            "snapshots": dict(inputs.snapshots),
            "known_schemas": {"ids": list(inputs.known_ids), "similar": similar},
        }
        return SchemaSplit(
            split_id,
            seed,
            tuple(sid for sid in ids if assignment[sid] == "train"),
            tuple(sid for sid in ids if assignment[sid] == "val"),
            tuple(sid for sid in ids if assignment[sid] == "held_out"),
            extra,
        )
