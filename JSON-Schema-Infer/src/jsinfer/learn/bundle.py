"""The Model_Bundle reader (Requirements 20.6, 21.10, 22.14, 23.9, 24.9, 27.2, 27.11).

A Model_Bundle is a directory of trained artifacts (design, "Model_Bundle and
training manifest")::

    <bundle>/
      manifest.json      format, bundle id, artifact locations, training metadata
      thresholds.json    Learned_Threshold name -> ThresholdRecord JSON (20.4)
      models/...         Decision_Models, Semantic_Type_Classifier, label models
      corpus/...         Corpus_Index

The Trainer writes bundles (task 23.1). This module finds, loads, and checks
them, and turns every missing or unusable artifact that an enabled Technique
needs into a report :class:`~jsinfer.core.report.Substitution`.

Identifiers (27.2, 27.11)
    The ``model_bundle`` setting holds an identifier or a path.
    ``bundles/<name>`` names a bundle shipped inside the ``jsinfer`` package
    (``jsinfer/bundles/<name>/``, package data). A shipped bundle takes
    precedence over a directory with the same relative name, so a default
    run never depends on the working directory; write ``./bundles/<name>``
    to select such a directory. Any other value is a filesystem path,
    relative to the working directory. Shipped bundles are only found when
    the package is installed as files (a source tree, an editable install,
    or an unpacked wheel), not from a zip archive.

    The shipped default, ``bundles/default``
    (:data:`~jsinfer.config.run_config.DEFAULT_MODEL_BUNDLE`), is empty: its
    manifest lists no artifacts and its ``thresholds.json`` is ``{}``. A
    default run therefore completes, and every enabled Technique that needs a
    Learned_Threshold or a model applies its Fallback_Behavior and records
    the substitution.

Loading (:func:`load_bundle`)
    The identifier must name a directory holding ``manifest.json`` and
    ``thresholds.json``. Both are strict UTF-8 JSON, and an object may not
    repeat a member. Otherwise the bundle cannot be loaded, and
    :class:`~jsinfer.config.errors.ConfigError` is raised before any document
    is ingested. It lists every problem found, each with reason ``"bundle"``,
    the identifier as its subject, and a message naming the identifier.

    ``manifest.json`` is an object. The reader requires three members and
    keeps every other member (the training metadata of 21.2, 21.5, and 21.9)
    unchanged in :attr:`ModelBundle.manifest`:

    - ``format``: :data:`BUNDLE_FORMAT`. Other format numbers are rejected.
    - ``bundle_id``: a non-empty string. It is the Model_Bundle identifier
      that the report gives for every Learned_Threshold read (20.5) and in its
      ``run`` section (27.8).
    - ``artifacts``: an object that maps artifact names (lowercase dotted,
      for example ``dm.record_or_map.synthetic``) to ``{"path": <file path
      relative to the bundle directory>, "sha256": <hex digest of the file's
      bytes>}``, as the Trainer writes them (design, Model_Bundle). A plain
      path string is accepted too, for hand-written bundles; such an
      artifact has no digest to verify.

    ``feature_spec.json``, when present, is the featurizer spec the bundle's
    Decision_Models were trained with (``learn.features.feature_spec_json``);
    it must be a JSON object.

    Artifact paths are relative POSIX paths whose segments are letters,
    digits, ``_``, ``.``, and ``-``, and do not start with ``.``. A path must
    also stay inside the bundle directory after symbolic links are resolved.
    ``manifest.json`` and ``thresholds.json`` must stay inside it too. A
    violation makes the bundle unloadable, so no file outside the bundle is
    ever read.

    ``thresholds.json`` is an object that maps Learned_Threshold names to
    ThresholdRecord JSON. Entries are checked when a Technique reads them
    (``config.thresholds.check_thresholds``), and entries with unregistered
    names are ignored. The tokens ``NaN``, ``Infinity``, and ``-Infinity``
    are accepted here, so a non-finite stored value becomes an invalid
    Learned_Threshold (20.6), not an unloadable bundle.

Artifacts
    ===============================  ============================================  =====================
    Artifact name                    Contents                                      Needed by
    ===============================  ============================================  =====================
    ``dm.<kind>.synthetic``          Decision_Model trained on synthetic labels    its ``decide.dm_*``
    ``dm.<kind>.weak``               Decision_Model trained with weak labels       ``decide.weak_supervision``
    ``semantic_type``                Semantic_Type_Classifier                      ``values.semantic_type``
    ``semantic_mapping``             semantic type -> Format or Pattern (13.5)     ``values.semantic_type``
    ``corpus.index``                 Corpus_Index                                  the three ``corpus.*``
    ``label_model.<kind>``           label model (training only)                   none at inference
    ===============================  ============================================  =====================

    ``<kind>`` is a Judgment_Call kind (:data:`DECISION_MODEL_TECHNIQUES`).
    :meth:`ModelBundle.load_artifact` loads a file by its suffix:
    ``.joblib`` with ``joblib.load``, ``.json`` as strict JSON, and any other
    suffix (``.sqlite``, ``.jsonl``, ``.npz``) as its :class:`~pathlib.Path`,
    which the consuming component opens. A listed file that does not exist
    is missing. A file that cannot be loaded is invalid.

    Digests (21.10): before an artifact file is unpickled or read, its
    SHA-256 is compared with the manifest's; a mismatch makes the artifact
    unusable with cause ``checksum``, and the Fallback_Behavior applies. The
    digest catches corrupted or swapped files, not a malicious bundle whose
    manifest matches its files: joblib files are pickles, and loading one
    can run arbitrary code, so load only bundles you trust.

    Feature spec (22.10): a Decision_Model is usable only when the bundle's
    ``feature_spec.json`` equals the running featurizer spec and the model's
    own ``feature_version`` and ``feature_dim`` match the kind's; otherwise
    it is treated as a missing model (cause ``invalid``, the detail naming
    the mismatch) and the next resolution step applies.

Writing (:func:`write_bundle`)
    The Trainer writes a bundle with :func:`write_bundle`: the artifact
    files (joblib for objects, bytes as given) under ``models/`` or the
    given relative paths, ``thresholds.json``, ``feature_spec.json``, and
    ``manifest.json`` with the artifact digests and the training metadata,
    all through ``core.determinism.atomic_directory`` (a temp sibling, then
    one rename), so a failed write leaves no bundle behind.

Resolution against a run (:meth:`ModelBundle.resolve`, :func:`open_bundle`)
    Given the run's Technique states, the reader checks the Learned_Thresholds
    and loads the models that the Techniques in effect need. A Technique is
    in effect when it is enabled and serves at least one Decision_Point kind
    under its serving gates (15.7), or when it serves none. The result is a
    :class:`ResolvedBundle` with the usable threshold records, the loaded
    models, and one Substitution per problem and affected Technique, sorted
    as the report sorts them:

    - Missing or invalid Learned_Threshold (20.6): fallback reason
      ``missing-threshold``, cause ``missing`` or ``invalid``. One
      Substitution for each Technique in effect that reads the threshold.
      ``applied`` names the Fallback_Behavior of the threshold's
      Decision_Point kind.
    - Missing synthetic Decision_Model (22.14): the next resolution step is
      applied, which is the next serving Technique in effect for the kind
      (``decide.mdl`` for every kind but ``closed_or_open_enum``, where it is
      ``values.enum_estimation``), or else the kind's Fallback_Behavior.
    - Missing weak Decision_Model, while ``decide.weak_supervision`` and the
      kind's Decision_Model are both in effect (23.9): the synthetic-only
      Decision_Model, which is the ``judgment_model`` Fallback_Behavior.
    - Missing Semantic_Type_Classifier or semantic mapping (21.10): the
      ``string_constraint`` Fallback_Behavior.
    - Missing Corpus_Index (21.10, 24.9): the ``suggestion``
      Fallback_Behavior for each suggestion Technique in effect. For
      ``corpus.nearest_schema``, which serves no Decision_Point, the lookup is
      skipped as for an empty Corpus_Index (25.6).

    Each artifact is loaded at most once per resolution. Errors surface
    there, before ingestion.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from importlib import resources
from pathlib import Path
from types import MappingProxyType
from typing import Any, Final, Literal, NoReturn, TypeAlias

from jsinfer.config.errors import ConfigError, ConfigProblem
from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE
from jsinfer.config.thresholds import (
    THRESHOLDS,
    WEAK_SUPERVISION,
    ThresholdCheck,
    ThresholdProblem,
    ThresholdRecord,
    ThresholdSpec,
    check_thresholds,
    thresholds_used_by,
)
from jsinfer.core.determinism import atomic_directory, codepoint_key, encode_canonical, sorted_codepoint
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.provenance import FallbackInfo
from jsinfer.core.report import SUBSTITUTION_CAUSES, Substitution, SubstitutionCause

__all__ = [
    "BUILTIN_PREFIX",
    "BUNDLE_FORMAT",
    "CORPUS_INDEX",
    "FEATURE_SPEC_FILE",
    "BundleArtifact",
    "DECISION_MODEL_SOURCES",
    "DECISION_MODEL_TECHNIQUES",
    "DEFAULT_MODEL_BUNDLE",
    "MANIFEST_FILE",
    "NEAREST_SCHEMA",
    "SEMANTIC_MAPPING",
    "SEMANTIC_TYPE",
    "SEMANTIC_TYPE_CLASSIFIER",
    "SUGGESTION_TECHNIQUES",
    "THRESHOLDS_FILE",
    "ArtifactError",
    "ArtifactProblem",
    "DecisionModelSource",
    "ModelBundle",
    "ModelRequirement",
    "ResolvedBundle",
    "builtin_bundles",
    "decision_model_artifact",
    "label_model_artifact",
    "load_bundle",
    "open_bundle",
    "required_models",
    "sha256_of",
    "write_bundle",
]

# The manifest format this reader understands.
BUNDLE_FORMAT: Final = 1

MANIFEST_FILE: Final = "manifest.json"
THRESHOLDS_FILE: Final = "thresholds.json"
FEATURE_SPEC_FILE: Final = "feature_spec.json"
_SHA256_RE: Final = re.compile(r"[0-9a-f]{64}")

# Identifiers of bundles shipped as package data: "bundles/<name>".
BUILTIN_PREFIX: Final = "bundles/"

DecisionModelSource: TypeAlias = Literal["synthetic", "weak"]
DECISION_MODEL_SOURCES: Final[tuple[DecisionModelSource, ...]] = ("synthetic", "weak")

# The Decision_Model Technique of each Judgment_Call kind, in registration order (22.10, 22.14).
DECISION_MODEL_TECHNIQUES: Final[Mapping[str, str]] = MappingProxyType(
    {
        "record_or_map": "decide.dm_record_map",
        "closed_or_open_record": "decide.dm_closed_record",
        "tuple_or_list": "decide.dm_tuple_list",
        "closed_or_open_enum": "decide.dm_closed_enum",
        "merge_or_split": "decide.dm_merge_split",
    }
)

# Artifact names of the Semantic_Type_Classifier (13.5, 21.6) and the Corpus_Index (24, 25).
SEMANTIC_TYPE_CLASSIFIER: Final = "semantic_type"
SEMANTIC_MAPPING: Final = "semantic_mapping"
CORPUS_INDEX: Final = "corpus.index"

# The Techniques that read those artifacts.
SEMANTIC_TYPE: Final = "values.semantic_type"
SUGGESTION_TECHNIQUES: Final[tuple[str, ...]] = ("corpus.name_suggest", "corpus.cf_suggest")
NEAREST_SCHEMA: Final = "corpus.nearest_schema"

_JUDGMENT_MODEL: Final = "judgment_model"
_STRING_CONSTRAINT: Final = "string_constraint"
_SUGGESTION: Final = "suggestion"
_NEAREST_SKIPPED: Final = "nearest-known-schema lookup skipped, as for an empty Corpus_Index (25.6)"

_BUILTIN_RE: Final = re.compile(r"bundles/([a-z0-9][a-z0-9_-]*)")
_ARTIFACT_RE: Final = re.compile(r"[a-z][a-z0-9_]*(?:\.[a-z0-9][a-z0-9_-]*)*")
_SEGMENT_RE: Final = re.compile(r"[A-Za-z0-9_][A-Za-z0-9_.-]*")
_SURROGATE_RE: Final = re.compile(r"[\ud800-\udfff]")
_REQUIRED_MEMBERS: Final = ("artifacts", "bundle_id", "format")


def decision_model_artifact(kind: str, source: DecisionModelSource) -> str:
    """The artifact name of the ``source`` Decision_Model of Judgment_Call ``kind``: ``dm.<kind>.<source>``."""
    if kind not in DECISION_MODEL_TECHNIQUES:
        raise ValueError(f"{kind!r} is not a Judgment_Call kind; the kinds are {list(DECISION_MODEL_TECHNIQUES)}")
    if source not in DECISION_MODEL_SOURCES:
        raise ValueError(f"Decision_Model source must be one of {list(DECISION_MODEL_SOURCES)}, got {source!r}")
    return f"dm.{kind}.{source}"


def label_model_artifact(kind: str) -> str:
    """The artifact name of the label model of Judgment_Call ``kind`` (23.2): ``label_model.<kind>``."""
    if kind not in DECISION_MODEL_TECHNIQUES:
        raise ValueError(f"{kind!r} is not a Judgment_Call kind; the kinds are {list(DECISION_MODEL_TECHNIQUES)}")
    return f"label_model.{kind}"


# ---------------------------------------------------------------------------
# Artifact problems
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ArtifactProblem:
    """A model artifact the bundle lacks (``cause="missing"``) or holds in an unusable form (``"invalid"``)."""

    artifact: str
    cause: SubstitutionCause
    detail: str

    def __post_init__(self) -> None:
        if self.cause not in SUBSTITUTION_CAUSES:
            raise ValueError(f"ArtifactProblem cause must be one of {list(SUBSTITUTION_CAUSES)}, got {self.cause!r}")

    def __str__(self) -> str:
        return f"artifact {self.artifact!r} is {self.cause}: {self.detail}"


class ArtifactError(ValueError):
    """Raised by :meth:`ModelBundle.load_artifact`; ``problem`` says which artifact and why."""

    def __init__(self, problem: ArtifactProblem) -> None:
        super().__init__(problem)  # args survive pickling
        self.problem = problem

    def __str__(self) -> str:
        return str(self.problem)


# ---------------------------------------------------------------------------
# JSON files
# ---------------------------------------------------------------------------


class _JsonProblem(ValueError):
    """A JSON file that cannot be parsed; the message says why."""


def _kind(value: Any) -> str:
    """How a JSON value reads in a message."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "a boolean"
    if isinstance(value, (int, float)):
        return "a number"
    if isinstance(value, str):
        return "a string"
    if isinstance(value, list):
        return "an array"
    if isinstance(value, dict):
        return "an object"
    return f"a {type(value).__name__}"


def _unique_members(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    obj: dict[str, Any] = {}
    for key, value in pairs:
        if key in obj:
            raise _JsonProblem(f"an object has the member {key!r} twice")
        obj[key] = value
    return obj


def _reject_constant(name: str) -> NoReturn:
    raise _JsonProblem(f"{name} is not a JSON value")


def _parse_json(data: bytes, *, constants: bool) -> Any:
    """Strict UTF-8 JSON without repeated members; ``constants`` admits NaN and the infinities."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise _JsonProblem(f"it is not valid UTF-8 at byte {error.start}") from None
    try:
        return json.loads(
            text,
            object_pairs_hook=_unique_members,
            parse_constant=None if constants else _reject_constant,
        )
    except json.JSONDecodeError as error:
        raise _JsonProblem(f"it is not valid JSON: {error.msg} at line {error.lineno}, column {error.colno}") from None
    except RecursionError:
        raise _JsonProblem("it is nested too deeply") from None


# ---------------------------------------------------------------------------
# Locating and reading a bundle
# ---------------------------------------------------------------------------


def _builtin_root() -> Path | None:
    """The package-data directory of the shipped bundles, when the package is installed as files."""
    root = resources.files("jsinfer").joinpath("bundles")
    return root if isinstance(root, Path) and root.is_dir() else None


def builtin_bundles() -> tuple[str, ...]:
    """The identifiers of the bundles shipped with jsinfer, in code-point order."""
    root = _builtin_root()
    if root is None:
        return ()
    names = [
        BUILTIN_PREFIX + entry.name
        for entry in root.iterdir()
        if _BUILTIN_RE.fullmatch(BUILTIN_PREFIX + entry.name) and (entry / MANIFEST_FILE).is_file()
    ]
    return tuple(sorted_codepoint(names))


def _bundle_error(identifier: str, details: list[str], verb: str = "loaded") -> ConfigError:
    return ConfigError(
        ConfigProblem(identifier, "bundle", f"model_bundle: Model_Bundle {identifier!r} cannot be {verb}: {detail}")
        for detail in details
    )


def _locate(identifier: str) -> tuple[Path, bool]:
    """The bundle directory of ``identifier`` and whether it is shipped; ``ConfigError`` when there is none."""
    if not identifier:
        raise _bundle_error(identifier, ["the identifier is empty"], "found")
    if _BUILTIN_RE.fullmatch(identifier):
        root = _builtin_root()
        if root is not None:
            shipped = root / identifier[len(BUILTIN_PREFIX) :]
            if (shipped / MANIFEST_FILE).is_file():
                return shipped, True
    path = Path(identifier)
    try:
        exists, is_dir = path.exists(), path.is_dir()
    except (OSError, ValueError) as error:
        raise _bundle_error(identifier, [f"the path cannot be inspected: {error}"], "found") from None
    if not exists:
        shipped = ", ".join(builtin_bundles()) or "none"
        detail = f"it is neither a shipped bundle (shipped: {shipped}) nor an existing directory"
        raise _bundle_error(identifier, [detail], "found")
    if not is_dir:
        raise _bundle_error(identifier, [f"{identifier} is not a directory"])
    return path, False


def _inside(root: Path, path: Path) -> bool:
    """Whether ``path`` stays inside the resolved directory ``root`` after symbolic links are resolved."""
    try:
        return path.resolve().is_relative_to(root)
    except (OSError, RuntimeError):  # unreadable link, or a symlink loop
        return False


def _read_json_file(root: Path, resolved_root: Path, name: str, problems: list[str], *, constants: bool) -> Any:
    """Parse the bundle file ``name``; on failure append a problem and return None."""
    path = root / name
    if not path.exists() and not path.is_symlink():
        problems.append(f"{name} is missing")
        return None
    if not _inside(resolved_root, path):
        problems.append(f"{name} resolves outside the bundle directory")
        return None
    if not path.is_file():
        problems.append(f"{name} is not a regular file")
        return None
    try:
        data = path.read_bytes()
    except OSError as error:
        problems.append(f"{name} cannot be read: {error.strerror or type(error).__name__}")
        return None
    try:
        return _parse_json(data, constants=constants)
    except _JsonProblem as error:
        problems.append(f"{name}: {error}")
        return None


def _artifact_entry(name: str, value: Any) -> tuple[str, str | None] | str:
    """``(path, digest or None)`` of the manifest's entry for artifact ``name``, or the problem with it."""
    where = f"{MANIFEST_FILE}: artifacts[{name!r}]"
    digest: str | None = None
    if isinstance(value, dict):
        if "path" not in value:
            return f"{where} must hold a path"
        unknown = sorted(member for member in value if member not in ("path", "sha256"))
        if unknown:
            return f"{where} has unknown members {unknown}"
        digest = value.get("sha256")
        if digest is not None and (not isinstance(digest, str) or not _SHA256_RE.fullmatch(digest)):
            return f"{where} sha256 must be a 64-character lowercase hex digest, got {_kind(digest)}"
        value = value["path"]
    problem = _artifact_path_problem(name, value)
    if problem is not None:
        return problem
    return value, digest


def _artifact_path_problem(name: str, value: Any) -> str | None:
    """Why the manifest's path for artifact ``name`` is unusable, or None."""
    where = f"{MANIFEST_FILE}: artifacts[{name!r}]"
    if not isinstance(value, str):
        return f"{where} must be a path string, got {_kind(value)}"
    segments = value.split("/")
    if not value or not all(_SEGMENT_RE.fullmatch(segment) for segment in segments):
        return (
            f"{where} must be a relative path inside the bundle whose '/'-separated segments are letters, "
            f"digits, '_', '.', or '-' and do not start with '.', got {value!r}"
        )
    return None


def _check_manifest(manifest: Any, problems: list[str]) -> tuple[str, dict[str, str], dict[str, str | None]] | None:
    """The bundle id, artifact paths, and artifact digests of a parsed manifest; None (with problems) when unusable."""
    if not isinstance(manifest, dict):
        problems.append(f"{MANIFEST_FILE}: expected an object at the top level, got {_kind(manifest)}")
        return None
    missing = [member for member in _REQUIRED_MEMBERS if member not in manifest]
    if missing:
        problems.append(f"{MANIFEST_FILE}: missing members {missing}")
        return None
    version = manifest["format"]
    if isinstance(version, bool) or not isinstance(version, int):
        problems.append(f"{MANIFEST_FILE}: format must be an integer, got {_kind(version)}")
        return None
    if version != BUNDLE_FORMAT:
        problems.append(f"{MANIFEST_FILE}: format {version} is not supported; this jsinfer reads format {BUNDLE_FORMAT}")
        return None
    count = len(problems)
    bundle_id = manifest["bundle_id"]
    if not isinstance(bundle_id, str) or not bundle_id:
        problems.append(f"{MANIFEST_FILE}: bundle_id must be a non-empty string, got {_kind(bundle_id)}")
    elif _SURROGATE_RE.search(bundle_id):
        problems.append(f"{MANIFEST_FILE}: bundle_id contains an unpaired surrogate")
    listed = manifest["artifacts"]
    artifacts: dict[str, str] = {}
    digests: dict[str, str | None] = {}
    if not isinstance(listed, dict):
        problems.append(f"{MANIFEST_FILE}: artifacts must be an object, got {_kind(listed)}")
    else:
        for name in sorted_codepoint(listed):
            if not _ARTIFACT_RE.fullmatch(name):
                problems.append(f"{MANIFEST_FILE}: artifact name {name!r} is not a lowercase dotted name")
                continue
            entry = _artifact_entry(name, listed[name])
            if isinstance(entry, str):
                problems.append(entry)
            else:
                artifacts[name], digests[name] = entry
    if len(problems) > count:
        return None
    return bundle_id, artifacts, digests


# ---------------------------------------------------------------------------
# Model requirements of a run
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ModelRequirement:
    """A model artifact that a Technique in effect needs.

    When the artifact is missing or invalid, ``technique`` applies its
    Fallback_Behavior (or, for a synthetic Decision_Model, the next resolution
    step), as ``applied`` describes.
    """

    artifact: str
    technique: str
    applied: str


def _states(techniques: Mapping[str, bool]) -> dict[str, bool]:
    """The state of every registered Technique; ids absent from ``techniques`` take their default (28.2)."""
    if not isinstance(techniques, Mapping):
        raise TypeError(f"techniques must be a mapping of Technique id to bool, got {type(techniques).__name__}")
    unknown = [tid for tid in techniques if tid not in REGISTRY.techniques]
    if unknown:
        raise ValueError(f"unregistered Technique ids {sorted_codepoint(unknown)}")
    states: dict[str, bool] = {}
    for tid, spec in REGISTRY.techniques.items():
        state = techniques.get(tid, spec.default_enabled)
        if not isinstance(state, bool):
            raise TypeError(f"the state of {tid!r} must be a bool, got {type(state).__name__}")
        states[tid] = state
    return states


def _in_effect(tid: str, states: Mapping[str, bool]) -> bool:
    """Enabled, and serving at least one kind under its gates (15.7), or serving none."""
    if not states[tid]:
        return False
    serves = REGISTRY.technique(tid).serves
    return not serves or any(tid in REGISTRY.active_servers(kind, states) for kind in serves)


def _fallback_text(kind: str) -> str:
    return f"Fallback_Behavior of {kind}: {REGISTRY.decision_point(kind).fallback_doc}"


def _resolves_without(kind: str, states: Mapping[str, bool], broken: Iterable[str]) -> str:
    """What resolves ``kind`` when the Learned_Thresholds ``broken`` are unusable (design, Open Decisions).

    The serving Techniques of ``kind`` that are still in effect there (those
    enabled under their gates and reading no unusable threshold at the kind),
    in priority order, or the kind's Fallback_Behavior when none is.
    """
    unusable = set(broken)
    standing = [
        tid
        for tid in REGISTRY.active_servers(kind, states)
        if not any(
            name in unusable and THRESHOLDS[name].decision_point == kind for name in REGISTRY.technique(tid).thresholds
        )
    ]
    if standing:
        return f"{', '.join(standing)} at {kind}"
    return _fallback_text(kind)


def _next_step(kind: str, technique: str, states: Mapping[str, bool]) -> str:
    """What resolves ``kind`` when ``technique`` has no model: the next serving Technique in effect, or the fallback."""
    rank = REGISTRY.rank(technique, kind)
    for tid in REGISTRY.active_servers(kind, states):
        if REGISTRY.rank(tid, kind) > rank:
            return f"{tid} at {kind}"
    return _fallback_text(kind)


def required_models(techniques: Mapping[str, bool]) -> tuple[ModelRequirement, ...]:
    """The model artifacts needed by the Techniques in effect, sorted by artifact, then Technique."""
    states = _states(techniques)
    found: list[ModelRequirement] = []
    weak = _in_effect(WEAK_SUPERVISION, states)
    for kind, tid in DECISION_MODEL_TECHNIQUES.items():
        if not _in_effect(tid, states):
            continue
        found.append(ModelRequirement(decision_model_artifact(kind, "synthetic"), tid, _next_step(kind, tid, states)))
        if weak:
            found.append(
                ModelRequirement(decision_model_artifact(kind, "weak"), WEAK_SUPERVISION, _fallback_text(_JUDGMENT_MODEL))
            )
    if _in_effect(SEMANTIC_TYPE, states):
        for artifact in (SEMANTIC_TYPE_CLASSIFIER, SEMANTIC_MAPPING):
            found.append(ModelRequirement(artifact, SEMANTIC_TYPE, _fallback_text(_STRING_CONSTRAINT)))
    for tid in SUGGESTION_TECHNIQUES:
        if _in_effect(tid, states):
            found.append(ModelRequirement(CORPUS_INDEX, tid, _fallback_text(_SUGGESTION)))
    if _in_effect(NEAREST_SCHEMA, states):
        found.append(ModelRequirement(CORPUS_INDEX, NEAREST_SCHEMA, _NEAREST_SKIPPED))
    return tuple(sorted(found, key=lambda r: (codepoint_key(r.artifact), codepoint_key(r.technique))))


# ---------------------------------------------------------------------------
# Bundles
# ---------------------------------------------------------------------------


def _load_file(path: Path) -> Any:
    """Load an artifact file by suffix: joblib, strict JSON, or the path itself."""
    suffix = path.suffix.lower()
    if suffix == ".joblib":
        import joblib  # deferred: only runs that load a model pay for the import

        return joblib.load(path)
    if suffix == ".json":
        return _parse_json(path.read_bytes(), constants=False)
    return path


@dataclass(frozen=True, eq=False)
class ModelBundle:
    """A loaded Model_Bundle; see the module docstring.

    ``identifier`` is the configured identifier or path, ``root`` the bundle
    directory, and ``builtin`` whether the bundle ships with jsinfer.
    ``manifest`` is the whole parsed manifest, ``artifacts`` maps artifact
    names to their paths relative to ``root`` (in code-point order), and
    ``threshold_entries`` maps names to the raw ``thresholds.json`` entries.
    All of them are read-only.
    """

    identifier: str
    root: Path
    builtin: bool
    bundle_id: str
    manifest: Mapping[str, JSON]
    artifacts: Mapping[str, str]
    threshold_entries: Mapping[str, JSON]
    digests: Mapping[str, str | None] = field(default_factory=dict)
    feature_spec: Mapping[str, JSON] | None = None

    def has_artifact(self, name: str) -> bool:
        """Whether the manifest lists artifact ``name``."""
        return name in self.artifacts

    def artifact_path(self, name: str) -> Path | None:
        """The file of artifact ``name`` (inside ``root``), or None when the manifest lists none."""
        relative = self.artifacts.get(name)
        return None if relative is None else self.root / relative

    def load_artifact(self, name: str) -> Any:
        """Load artifact ``name``; :class:`ArtifactError` when it is missing or cannot be loaded."""
        path = self.artifact_path(name)
        if path is None:
            raise ArtifactError(ArtifactProblem(name, "missing", "the bundle manifest lists no such artifact"))
        relative = self.artifacts[name]
        if not path.exists():
            raise ArtifactError(ArtifactProblem(name, "missing", f"its file {relative} does not exist"))
        if not path.is_file():
            raise ArtifactError(ArtifactProblem(name, "invalid", f"its file {relative} is not a regular file"))
        expected = self.digests.get(name)
        if expected is not None:  # 21.10: verified before the file is unpickled or read
            try:
                actual = sha256_of(path)
            except OSError as error:
                reason = error.strerror or type(error).__name__
                raise ArtifactError(ArtifactProblem(name, "invalid", f"its file {relative} cannot be read: {reason}")) from None
            if actual != expected:
                raise ArtifactError(
                    ArtifactProblem(name, "checksum", f"the SHA-256 of its file {relative} is {actual}, the manifest says {expected}")
                )
        try:
            loaded = _load_file(path)
        except Exception as error:  # unpickling can raise anything; every failure means an unusable model
            reason = str(error) or type(error).__name__
            raise ArtifactError(ArtifactProblem(name, "invalid", f"its file {relative} cannot be loaded: {reason}")) from None
        mismatch = self._feature_mismatch(name, loaded)
        if mismatch is not None:
            raise ArtifactError(ArtifactProblem(name, "invalid", mismatch))
        return loaded

    def _feature_mismatch(self, name: str, model: Any) -> str | None:
        """Why Decision_Model ``name`` was not trained with the running featurizers (22.10), or None."""
        if not name.startswith("dm."):
            return None
        from jsinfer.learn.features import FEATURE_SPEC, FEATURE_VERSION, feature_spec_json  # deferred: import cycle

        if self.feature_spec is not None and dict(self.feature_spec) != feature_spec_json():
            return f"the bundle's {FEATURE_SPEC_FILE} differs from the running featurizer spec (version {FEATURE_VERSION})"
        kind = name.split(".")[1] if name.count(".") >= 2 else ""
        version = getattr(model, "feature_version", None)
        dim = getattr(model, "feature_dim", None)
        if version is not None and str(version) != FEATURE_VERSION:
            return f"it was trained with featurizer version {version!r}, the running version is {FEATURE_VERSION!r}"
        if dim is not None and kind in FEATURE_SPEC and int(dim) != len(FEATURE_SPEC[kind]):
            return f"it takes {dim} features, the running {kind} featurizer computes {len(FEATURE_SPEC[kind])}"
        return None

    def check_thresholds(self, specs: Iterable[ThresholdSpec] | None = None) -> ThresholdCheck:
        """Check the stored entries of ``specs`` (default: every registered Learned_Threshold) (20.6)."""
        return check_thresholds(THRESHOLDS.values() if specs is None else specs, self.threshold_entries)

    def threshold_values(self) -> dict[str, int | float]:
        """Name -> value of every registered Learned_Threshold the bundle holds a usable value for (20.2)."""
        return self.check_thresholds().values

    def resolve(self, techniques: Mapping[str, bool]) -> ResolvedBundle:
        """Check the thresholds and load the models the Techniques in effect need (see the module docstring).

        ``techniques`` maps Technique ids to their states. Ids it omits take
        their registered default, and an unregistered id is a ``ValueError``.
        """
        states = _states(techniques)
        in_effect = tuple(tid for tid in REGISTRY.ids() if _in_effect(tid, states))
        check = check_thresholds(thresholds_used_by(in_effect), self.threshold_entries)
        problems: list[ThresholdProblem | ArtifactProblem] = list(check.problems)
        substitutions: list[Substitution] = []
        broken = {problem.name for problem in check.problems}
        for threshold in check.problems:
            applied = _resolves_without(threshold.decision_point, states, broken)
            for tid in threshold.techniques:
                if tid in in_effect:
                    fallback = FallbackInfo(tid, "missing-threshold")
                    substitutions.append(Substitution(threshold.name, fallback, threshold.cause, applied))
        models: dict[str, Any] = {}
        failed: dict[str, ArtifactProblem] = {}
        for requirement in required_models(states):
            name = requirement.artifact
            if name not in models and name not in failed:
                try:
                    models[name] = self.load_artifact(name)
                except ArtifactError as error:
                    failed[name] = error.problem
                    problems.append(error.problem)
            if name in failed:
                fallback = FallbackInfo(requirement.technique, "missing-model")
                substitutions.append(Substitution(name, fallback, failed[name].cause, requirement.applied))
        return ResolvedBundle(
            bundle=self,
            thresholds=check.records,
            models=MappingProxyType(dict(sorted(models.items(), key=lambda item: codepoint_key(item[0])))),
            substitutions=tuple(sorted(substitutions, key=Substitution.sort_key)),
            problems=tuple(problems),
        )


@dataclass(frozen=True, eq=False)
class ResolvedBundle:
    """A Model_Bundle checked against one run's Technique states.

    ``thresholds`` holds the usable records of the Learned_Thresholds the
    Techniques in effect read, and ``models`` the loaded artifacts they need,
    both by name in code-point order. ``substitutions`` go to the report's
    ``substitutions`` section (20.6, 21.10, 22.14, 24.9). ``problems`` holds
    the detail behind each one: threshold problems in name order, then
    artifact problems in artifact order.
    """

    bundle: ModelBundle
    thresholds: Mapping[str, ThresholdRecord]
    models: Mapping[str, Any]
    substitutions: tuple[Substitution, ...]
    problems: tuple[ThresholdProblem | ArtifactProblem, ...]

    @property
    def bundle_id(self) -> str:
        """The bundle id, reported with every Learned_Threshold read (20.5)."""
        return self.bundle.bundle_id

    def threshold(self, name: str) -> int | float | None:
        """The value of Learned_Threshold ``name``, or None when its Technique must apply its fallback."""
        record = self.thresholds.get(name)
        return None if record is None else record.value

    def model(self, name: str) -> Any | None:
        """The loaded artifact ``name``, or None when it is unavailable or not needed by this run."""
        return self.models.get(name)

    def decision_model(self, kind: str) -> tuple[str, Any] | None:
        """The Decision_Model that resolves Judgment_Call ``kind``, as (artifact name, model), or None.

        The weakly supervised model wins when it was loaded, which happens
        only while ``decide.weak_supervision`` and the kind's Decision_Model
        are both in effect (23.8). Otherwise the synthetic-only model is used
        (23.9). None means the kind's Decision_Model is disabled or missing.
        """
        for source in ("weak", "synthetic"):
            name = decision_model_artifact(kind, source)
            if name in self.models:
                return name, self.models[name]
        return None


def load_bundle(identifier: str | os.PathLike[str]) -> ModelBundle:
    """Find and read the Model_Bundle named by ``identifier`` (27.2, 27.11).

    Raises :class:`~jsinfer.config.errors.ConfigError` listing every problem
    when the bundle cannot be found or loaded (see the module docstring).
    """
    name = os.fspath(identifier)
    if not isinstance(name, str):
        raise TypeError(f"a Model_Bundle identifier must be a str or path, got {type(name).__name__}")
    root, builtin = _locate(name)
    resolved_root = root.resolve()
    problems: list[str] = []
    manifest = _read_json_file(root, resolved_root, MANIFEST_FILE, problems, constants=False)
    checked = None if manifest is None else _check_manifest(manifest, problems)
    if checked is not None:
        for artifact, relative in checked[1].items():
            if not _inside(resolved_root, root / relative):
                problems.append(
                    f"{MANIFEST_FILE}: artifacts[{artifact!r}] ({relative}) resolves outside the bundle directory"
                )
    entries = _read_json_file(root, resolved_root, THRESHOLDS_FILE, problems, constants=True)
    if entries is not None and not isinstance(entries, dict):
        problems.append(f"{THRESHOLDS_FILE}: expected an object at the top level, got {_kind(entries)}")
    spec: Any = None
    if (root / FEATURE_SPEC_FILE).exists() or (root / FEATURE_SPEC_FILE).is_symlink():
        spec = _read_json_file(root, resolved_root, FEATURE_SPEC_FILE, problems, constants=False)
        if spec is not None and not isinstance(spec, dict):
            problems.append(f"{FEATURE_SPEC_FILE}: expected an object at the top level, got {_kind(spec)}")
    if problems or checked is None or not isinstance(entries, dict):
        raise _bundle_error(name, problems)
    bundle_id, artifacts, digests = checked
    return ModelBundle(
        identifier=name,
        root=root,
        builtin=builtin,
        bundle_id=bundle_id,
        manifest=MappingProxyType(manifest),
        artifacts=MappingProxyType(artifacts),
        threshold_entries=MappingProxyType(entries),
        digests=MappingProxyType(digests),
        feature_spec=None if spec is None else MappingProxyType(spec),
    )


def open_bundle(identifier: str | os.PathLike[str], techniques: Mapping[str, bool]) -> ResolvedBundle:
    """The bundle stage of a run: ``load_bundle(identifier).resolve(techniques)``."""
    return load_bundle(identifier).resolve(techniques)


# ---------------------------------------------------------------------------
# Writing a bundle (task 23.1)
# ---------------------------------------------------------------------------


def sha256_of(path: Path) -> str:
    """The hex SHA-256 of a file's bytes."""
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


@dataclass(frozen=True)
class BundleArtifact:
    """An artifact to write: its name, its path relative to the bundle, and bytes or a picklable object."""

    name: str
    path: str
    payload: Any

    def __post_init__(self) -> None:
        if not _ARTIFACT_RE.fullmatch(self.name):
            raise ValueError(f"artifact name {self.name!r} is not a lowercase dotted name")
        problem = _artifact_path_problem(self.name, self.path)
        if problem is not None:
            raise ValueError(problem)
        if not isinstance(self.payload, bytes) and not self.path.endswith(".joblib"):
            raise ValueError(f"artifact {self.name!r}: an object payload is written with joblib, so its path must end in .joblib")

    @classmethod
    def model(cls, name: str, payload: Any) -> BundleArtifact:
        """A joblib model artifact at ``models/<name>.joblib``."""
        return cls(name, f"models/{name}.joblib", payload)


def write_bundle(
    target: str | os.PathLike[str],
    *,
    bundle_id: str,
    artifacts: Iterable[BundleArtifact] = (),
    thresholds: Mapping[str, ThresholdRecord] | None = None,
    feature_spec: Mapping[str, JSON] | None = None,
    manifest: Mapping[str, JSON] | None = None,
) -> Path:
    """Write a Model_Bundle to ``target`` atomically and return its path (see the module docstring, Writing).

    ``manifest`` holds the training metadata; ``format``, ``bundle_id``, and
    ``artifacts`` are set here and may not be supplied. ``feature_spec``
    defaults to the running featurizer spec. Artifacts must have distinct
    names and paths.
    """
    if not isinstance(bundle_id, str) or not bundle_id or _SURROGATE_RE.search(bundle_id):
        raise ValueError("bundle_id must be a non-empty str without unpaired surrogates")
    listed = list(artifacts)
    names = [artifact.name for artifact in listed]
    paths = [artifact.path for artifact in listed]
    if len(set(names)) != len(names) or len(set(paths)) != len(paths):
        raise ValueError("artifact names and paths must be distinct")
    extra = dict(manifest or {})
    clash = sorted(member for member in extra if member in _REQUIRED_MEMBERS)
    if clash:
        raise ValueError(f"manifest members {clash} are written by write_bundle")
    if feature_spec is None:
        from jsinfer.learn.features import feature_spec_json  # deferred: import cycle

        feature_spec = feature_spec_json()
    import joblib  # deferred: only writers pay for the import

    Path(target).parent.mkdir(parents=True, exist_ok=True)  # the atomic rename needs the parent to exist
    with atomic_directory(target) as tmp:
        entries: dict[str, JSON] = {}
        for artifact in sorted(listed, key=lambda a: codepoint_key(a.name)):
            path = tmp / artifact.path
            path.parent.mkdir(parents=True, exist_ok=True)
            if isinstance(artifact.payload, bytes):
                path.write_bytes(artifact.payload)
            else:
                joblib.dump(artifact.payload, path)
            entries[artifact.name] = {"path": artifact.path, "sha256": sha256_of(path)}
        records = {name: record.to_json() for name, record in sorted((thresholds or {}).items(), key=lambda i: codepoint_key(i[0]))}
        (tmp / THRESHOLDS_FILE).write_bytes(encode_canonical(records))
        (tmp / FEATURE_SPEC_FILE).write_bytes(encode_canonical(dict(feature_spec)))
        document = {"format": BUNDLE_FORMAT, "bundle_id": bundle_id, "artifacts": entries, **extra}
        (tmp / MANIFEST_FILE).write_bytes(encode_canonical(document))
    return Path(target)
