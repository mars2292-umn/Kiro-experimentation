"""Pipeline orchestration: one in-memory inference run from documents to the emitted layout (Requirement 27).

:func:`run_pipeline` runs every stage of the design's inference pipeline in
order, with one :class:`InferenceContext` per run, and returns a
:class:`PipelineResult`: the Schema_Model after the soundness check, its
emitted layout and bytes, the explanation file, the Inference_Report, and
the bookkeeping behind them. It writes no file. The public API
(``jsinfer.infer``, ``jsinfer.infer_files``) wraps this module and returns
an ``InferenceResult`` holding every output file's bytes, which the CLI
(task 18.2) writes atomically under ``--out``
(``core.determinism.atomic_outputs``).

Stages
    Each stage runs inside :meth:`InferenceContext.stage`, which times it
    with ``InferenceReport.stage`` under the name of the component that
    executes it (27.8). :data:`STAGES` lists them in run order:

    ======================  ====================================================
    ``Run_Configuration``   The effective Run_Configuration: the one supplied,
                            or every default when None (27.2). It is
                            validated on construction, so invalid settings
                            never reach a run (27.11).
    ``Model_Bundle``        Load the bundle named by ``model_bundle`` and
                            check it against the Technique states
                            (``learn.bundle.open_bundle``); record its
                            Substitutions (20.6, 21.10). Build the run's
                            Structure_Analyzer, whose DecisionEngine every
                            later stage shares, and record the run metadata
                            (``run`` section; below).
    ``Ingestor``            Read the source into a DocumentStore
                            (``io.ingest``); record every skipped document or
                            file (1.7).
    ``ObservationStore``    Flatten the documents in fixed chunks of
                            ``ingest.chunk_size`` documents, in at most
                            ``workers`` joblib processes, and merge the chunks
                            in order (``core.observations``).
    ``Outlier_Detector``    Document flagging hook (below).
    ``Version_Detector``    Change-point hook (below).
    ``Structure_Analyzer``  ``StructureAnalyzer.infer_model`` on the root view
                            of the segment's non-flagged documents.
    ``Soundness``           ``pipeline.soundness.check_soundness`` over every
                            document of the segment that is not a
                            Flagged_Document (3.2), before anything is
                            emitted; then provenance finalization (below).
    ``Schema_Emitter``      ``SchemaEmitter.layout`` (the document and the
                            ConstraintId → JSON Pointer map), with
                            ``x-provenance`` when ``output.inline_provenance``
                            is on (26.6), its canonical bytes, and the
                            explanation file
                            (``SchemaEmitter.explanation_file``; 26.5, 26.7).
                            These are the outputs the CLI writes and the
                            public API returns, so an EmitError here is a
                            stage error like any other.
    ======================  ====================================================

    The report's ``status`` stays ``ok`` when every stage completes.

Run metadata and Resource_Parameters (20.5, 27.8)
    The ``run`` section, recorded in the Model_Bundle stage, holds the
    effective Run_Configuration, the seed, the Model_Bundle id, the jsinfer
    and CPython versions, the installed version of each runtime package of
    the lock file (:data:`LOCKED_PACKAGES`, read with ``importlib.metadata``;
    :func:`locked_package_versions`), and the run's start timestamp, taken
    before the Run_Configuration stage. The end timestamp is added when the
    run ends, completed or failed (:func:`run_timestamp`, UTC). Timestamps
    and stage runtimes appear only in ``run`` and ``timings``, which 27.12
    exempts with the worker count (``InferenceReport.comparable_json``).

    Every Resource_Parameter read during the run is listed in the report
    with its value and origin (default, configuration file, or flag): the
    stages read through :meth:`InferenceContext.resource` (``workers``,
    ``ingest.chunk_size``, ``soundness.max_passes``), and the
    Structure_Analyzer's components through the ``ResourceReader`` the
    analyzer is built with (the VariantAnalyzer's ``minhash.perm``,
    ``lsh.bands``, ``sample.dbscan``, ``sample.hdbscan``, and ``variants.*``,
    each read only when its Technique runs).

InferenceContext
    The per-run state the design calls ``Ctx``: the Run_Configuration, the
    registry, the Inference_Report, the Validator, the resolved Model_Bundle,
    the Structure_Analyzer and its DecisionEngine, the random-stream factory
    (:meth:`InferenceContext.rng`, ``core.determinism.rng`` with the run
    seed), and the stage outputs (ingestion result, ObservationStore,
    segments). Stages fill it in order.

Sources
    A source says how the Ingestor reads the documents:
    :class:`PathSource` (files and directories, 1.1–1.3, 1.10),
    :class:`ValueSource` (parsed Python values, ``Ingestor.ingest_values``),
    and :class:`JsonlSource` (the bytes of one ``.jsonl`` file,
    ``Ingestor.ingest_bytes``). In-memory documents are ingested as the lines
    of a ``.jsonl`` file, one per document, so every later stage sees the
    same DocumentStore it would see for that file (27.9).
    ``io.ingest.documents_to_jsonl`` writes the lines and converts nothing:
    a value outside the JSON domain raises ``TypeError`` (``ValueError`` for
    a circular reference), while NaN, infinities, and unpaired surrogates
    are written as a file would hold them and are skipped and recorded at
    their lines (see ``io.ingest``, In-memory documents).
    :func:`infer_documents` writes them before any stage runs, so those
    errors leave the call as they are; a :class:`ValueSource` writes them
    inside the Ingestor stage, where they become an InferenceError.

In-memory runs (Properties 3, 7, 9 and other full-run tests)
    ``infer_documents(documents, config)`` runs the pipeline on a list (or
    any iterable) of JSON values, or on JSONL ``bytes``, and returns the
    :class:`PipelineResult`:

    - ``result.model``: the Schema_Model after soundness, with its
      Provenance_Records keyed by ConstraintId;
    - ``result.document`` and ``result.pointers``: the emitted layout (with
      ``x-provenance`` when ``output.inline_provenance`` is on) and the
      ConstraintId → JSON Pointer map, which is the same either way;
      ``result.schema``: the document's canonical bytes;
      ``result.explanation``: the explanation file
      (``ExplanationFile.dumps`` gives its bytes);
    - ``result.report``: the Inference_Report;
    - ``result.soundness``: the SoundnessResult (counts, change records,
      validated, flagged, and first-pass failing document indices);
    - ``result.documents``, ``result.origins``, ``result.store``, and
      ``result.flagged``: the DocumentStore, the document origins, the
      ObservationStore, and the Flagged_Documents.

    ``config`` defaults to every default setting. Full-run tests should pin
    costly Resource_Parameters, for example ``workers = 1``. Document ``i``
    of the result is the ``i``-th value that could be ingested; values that
    fail to parse as their JSONL lines (NaN, ...) are skipped and recorded
    in the report. The public API (``jsinfer.infer``) wraps this function.

Errors (1.8, 27.10, 27.13)
    - :class:`~jsinfer.config.errors.ConfigError` (an unloadable
      Model_Bundle, a rejected input path) propagates unchanged, before any
      document is ingested (27.11, 1.10).
    - :class:`~jsinfer.core.errors.NoDocumentsError` is raised when the
      source yields no document that can be ingested. Its errors are in the
      report's ``ingestion`` section and its ``partial_report`` is the run's
      report, with ``status`` failed at ``Ingestor``.
    - Any other exception that leaves a stage is unrecoverable: it becomes
      :class:`~jsinfer.core.errors.InferenceError` with ``stage`` the
      component's name and ``partial_report`` the run's report, which names
      the stage and the error and lists each completed stage with its
      runtime (27.10). The original exception is the ``__cause__``. Errors a
      requirement resolves by skipping, falling back, or recording and
      continuing are handled inside the stages. No output exists yet when a
      stage fails, so nothing needs removing; the CLI deletes its temp
      output directory.
    - ``TypeError`` and ``ValueError`` for documents that are not JSON
      values come from :func:`infer_documents` before any stage runs (see
      Sources); they are usage errors, not stage errors.

Hooks
    - **Outlier detection** (``Outlier_Detector``; 17.1, 17.6, 17.7, 17.9;
      task 38.1). While Isolation Forest and LOF are disabled, the
      ``document_outlier`` Decision_Point takes its Fallback_Behavior (every
      document included, 17.7), recorded in the report's ``decisions``
      (28.4), and no document is flagged. With MDL exception encoding
      disabled too (17.6), no Observation is flagged during analysis, and
      the report lists zero Flagged_Items. A serving Technique registered as
      implemented raises ``NotImplementedError`` here until its task fills
      the hook in, and so does a flagging Technique for the 17.9 check.
    - **Change points** (``Version_Detector``; 18.1–18.11; tasks 48.1,
      48.2). While ``robust.change_point`` is disabled, the
      ``version_split`` Decision_Point takes its Fallback_Behavior (one
      schema), the report records that change-point detection was not
      applied (18.10), and the run has one segment holding every document.
      Analysis, soundness, and emission run per segment; with one segment
      the result is the single schema.
    - **Provenance finalization** (26.1, 26.2, 26.4, 26.10, 26.13; task
      17.1) runs after the soundness check, inside the ``Soundness`` stage
      (its Support walk is the soundness check's), before anything is
      emitted; see below.

Provenance finalization (:func:`finalize_provenance`)
    1. Lay out the model; its pointer map is the set of emitted
       Constraints. ``core.provenance.check_records`` checks that they and
       the Provenance_Records correspond one to one, and raises
       ``ProvenanceError`` (an internal error, so the stage fails) naming
       every Constraint without a record and every record without a
       Constraint. No emitted Constraint may legitimately lack one: the
       components record every Constraint they emit (Fallback_Behavior and
       Baseline_Rule Constraints included), the soundness loop moves records
       with renamed Constraints, gives Constraints created under 3.3 the
       relaxation decision's record, and drops the records of what it
       removes. The one case the soundness loop leaves without a record, an
       ``allOf`` created beside an existing ``anyOf`` by a removal-loop
       guard (3.7, which resolves no decision), needs a node holding both
       ``anyOf`` and ``oneOf``, which no component emits yet.
    2. Recompute the Support of every emitted Constraint
       (``pipeline.soundness.count_instances``, the soundness check's rule)
       over the non-flagged Observations of the segment: the documents that
       are not Flagged_Items, walked along the final schema without
       entering their Flagged_Items (17.4). Constraints inside ``$defs``
       are reached through every reference, so their totals cover every
       referencing Path. ``anyOf``/``oneOf`` branches count the instances
       they accept among every instance at the node, which for the
       mixed-type rule's branches is every row at the Path. Constraints
       inside a branch count the instances that branch accepts. The
       Constraints of a ``propertyNames`` subschema share the
       ``propertyNames`` Constraint's record (design, Open Decisions), so
       all of them take one Support over the Map's keys: every non-flagged
       key, consistent when the subschema accepts it.
    3. ``core.provenance.finalize_record`` installs each Support and
       refreshes the Laplace default Confidences from it (Baseline_Rule
       records, ``x-latent-type``, priority and Fallback_Behavior records
       without a kind-specific Confidence); decision Confidences stay.

    The walk is serial, one document at a time, so its result does not
    depend on the worker count.

Determinism (27.5, 27.6, 27.12)
    Every stage's result depends on the documents in index order and on
    the settings, never on the worker count or the chunk size: the work
    units (parsing, flattening, and validation chunks) are fixed-size
    chunks of ``ingest.chunk_size`` documents, merged in submission order,
    each task runs with OpenMP and BLAS limited to one thread
    (``core.workers.single_threaded``), and every later choice is
    canonical. Equal runs that differ only in ``workers`` give
    byte-identical schemas and reports equal under
    ``InferenceReport.comparable_json``.

Standard input (27.1; design, Open Decisions)
    A run with more than one worker holds ``os.devnull`` on descriptor 0
    while it runs when the calling program has descriptor 0 closed
    (``core.workers.reserved_stdin``), and closes it again when the run
    ends, so ``jsinfer.infer`` and ``jsinfer.infer_files`` work in such a
    program as the CLI does.
"""

from __future__ import annotations

import importlib.metadata
import platform
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from contextlib import AbstractContextManager, contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from os import PathLike
from typing import Any, Final, Protocol

import numpy as np
from joblib import Parallel, delayed

from jsinfer.config.errors import ConfigError
from jsinfer.config.registry import REGISTRY, Registry
from jsinfer.config.resources import WORKERS, ResourceReader
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.determinism import rng
from jsinfer.core.errors import INGESTOR_STAGE, InferenceError, NoDocumentsError
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.observations import ObservationStore, PartialStore, flatten_chunk, merge_partials
from jsinfer.core.provenance import check_records, finalize_record
from jsinfer.core.report import ErrorInfo, InferenceReport, RunInfo, VersionsInfo
from jsinfer.core.schema_model import ConstraintId, SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.core.workers import reserved_stdin, single_threaded
from jsinfer.decide.engine import DecisionEngine, DecisionPoint
from jsinfer.io.emit import ExplanationFile, PointerMap, SchemaEmitter
from jsinfer.io.ingest import (
    BYTES_SOURCE,
    VALUES_SOURCE,
    DocOrigin,
    DocumentStore,
    IngestResult,
    Ingestor,
    documents_to_jsonl,
    parse_document,
)
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import ResolvedBundle, open_bundle
from jsinfer.pipeline.soundness import (
    CHUNK_SIZE,
    MAX_PASSES,
    SoundnessResult,
    check_soundness,
    count_instances,
    flagged_documents,
    flagged_item_locations,
)
from jsinfer.structure.analyzer import StructureAnalyzer

__all__ = [
    "ANALYZER_STAGE",
    "BUNDLE_STAGE",
    "CHANGE_POINT_TECHNIQUES",
    "CONFIG_STAGE",
    "DOCUMENT_OUTLIER",
    "EMITTER_STAGE",
    "FLATTEN_STAGE",
    "INGESTOR_STAGE",
    "LOCKED_PACKAGES",
    "OUTLIER_STAGE",
    "OUTLIER_TECHNIQUES",
    "SOUNDNESS_STAGE",
    "STAGES",
    "VERSION_SPLIT",
    "VERSION_STAGE",
    "InferenceContext",
    "JsonlSource",
    "PathSource",
    "PipelineResult",
    "Source",
    "ValueSource",
    "finalize_provenance",
    "infer_documents",
    "infer_paths",
    "locked_package_versions",
    "run_pipeline",
    "run_timestamp",
]

# Stage names: the component that executes each stage (27.8), in run order.
CONFIG_STAGE: Final = "Run_Configuration"
BUNDLE_STAGE: Final = "Model_Bundle"
FLATTEN_STAGE: Final = "ObservationStore"
OUTLIER_STAGE: Final = "Outlier_Detector"
VERSION_STAGE: Final = "Version_Detector"
ANALYZER_STAGE: Final = "Structure_Analyzer"
SOUNDNESS_STAGE: Final = "Soundness"
EMITTER_STAGE: Final = "Schema_Emitter"
STAGES: Final = (
    CONFIG_STAGE,
    BUNDLE_STAGE,
    INGESTOR_STAGE,
    FLATTEN_STAGE,
    OUTLIER_STAGE,
    VERSION_STAGE,
    ANALYZER_STAGE,
    SOUNDNESS_STAGE,
    EMITTER_STAGE,
)

# The run-level Decision_Points of the hooks, and the Techniques that serve them.
DOCUMENT_OUTLIER: Final = "document_outlier"
VERSION_SPLIT: Final = "version_split"
OUTLIER_TECHNIQUES: Final = REGISTRY.decision_point(DOCUMENT_OUTLIER).served_by
CHANGE_POINT_TECHNIQUES: Final = REGISTRY.decision_point(VERSION_SPLIT).served_by

# The Techniques whose flags count for the abandonment check of 17.9.
_FLAGGING_TECHNIQUES: Final = (*OUTLIER_TECHNIQUES, *REGISTRY.decision_point("value_exception").served_by)

# The key Constraint keyword, whose subschema's Constraints share its Provenance_Record (design, Open Decisions).
_PROPERTY_NAMES: Final = "propertyNames"

# The runtime packages of the project lock file (uv.lock), by their normalized
# names in code-point order: the dependencies of jsinfer and theirs under any
# marker, without jsinfer itself (``run.jsinfer_version``) and without the dev
# group (pytest, hypothesis, regress, and their dependencies), which only the
# test suite imports. tests/unit/test_run_metadata.py checks this list against
# uv.lock, so a dependency change that does not update it fails there.
LOCKED_PACKAGES: Final[tuple[str, ...]] = (
    "attrs",
    "cloudpickle",
    "datasketch",
    "greenery",
    "joblib",
    "jsonref",
    "jsonschema",
    "jsonschema-rs",
    "jsonschema-specifications",
    "jsonsubschema",
    "narwhals",
    "numpy",
    "orjson",
    "portion",
    "psutil",
    "referencing",
    "rpds-py",
    "ruptures",
    "scikit-learn",
    "scipy",
    "sortedcontainers",
    "threadpoolctl",
    "tomli-w",
    "typing-extensions",
)

# The wall clock of the run timestamps; module-level so tests can substitute a fake one.
_wall_clock: Callable[[], datetime] = lambda: datetime.now(timezone.utc)


# ---------------------------------------------------------------------------
# Sources
# ---------------------------------------------------------------------------


class Source(Protocol):
    """How the Ingestor reads a run's documents."""

    def ingest(self, ingestor: Ingestor) -> IngestResult: ...


@dataclass(frozen=True, slots=True)
class PathSource:
    """Files and directories, in argument order (1.1–1.3, 1.5, 1.10)."""

    paths: tuple[str | PathLike[str], ...]

    def __post_init__(self) -> None:
        if isinstance(self.paths, (str, bytes, PathLike)):
            raise TypeError("paths must be a sequence of paths, not a single path")
        object.__setattr__(self, "paths", tuple(self.paths))

    def ingest(self, ingestor: Ingestor) -> IngestResult:
        return ingestor.ingest(self.paths)


@dataclass(frozen=True, slots=True)
class ValueSource:
    """Parsed documents (Python values) in iteration order, read once (``Ingestor.ingest_values``)."""

    documents: Iterable[JSON]
    name: str = VALUES_SOURCE

    def ingest(self, ingestor: Ingestor) -> IngestResult:
        return ingestor.ingest_values(self.documents, self.name)


@dataclass(frozen=True, slots=True)
class JsonlSource:
    """The bytes of one ``.jsonl`` file named ``name`` (``Ingestor.ingest_bytes``)."""

    data: bytes
    name: str = BYTES_SOURCE

    def ingest(self, ingestor: Ingestor) -> IngestResult:
        return ingestor.ingest_bytes(self.data, self.name)


def _source(documents: Iterable[JSON] | bytes | bytearray | memoryview) -> Source:
    """A JsonlSource: the bytes as they are, or an iterable of values written as JSONL (``<documents>``).

    The values are written here, before any stage runs, so a value outside
    the JSON domain raises ``TypeError`` or ``ValueError`` from the caller's
    call rather than failing the Ingestor stage (see Sources).
    """
    if isinstance(documents, (bytes, bytearray, memoryview)):
        return JsonlSource(bytes(documents))
    if isinstance(documents, str):
        raise TypeError("documents must be an iterable of JSON values or JSONL bytes, not a str")
    return JsonlSource(documents_to_jsonl(documents), VALUES_SOURCE)


# ---------------------------------------------------------------------------
# The context and the result
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class _Segment:
    """The documents one schema covers: every document (``docs`` None) or one Schema_Version's."""

    version: int | None = None
    docs: np.ndarray | None = None


@dataclass(eq=False)
class InferenceContext:
    """The state of one run (see the module docstring); stages fill the optional fields in order."""

    config: RunConfiguration
    report: InferenceReport = field(default_factory=InferenceReport)
    validator: Validator = field(default_factory=Validator)
    registry: Registry = field(default_factory=lambda: REGISTRY)
    bundle: ResolvedBundle | None = None
    analyzer: StructureAnalyzer | None = None
    ingested: IngestResult | None = None
    store: ObservationStore | None = None
    segments: tuple[_Segment, ...] = ()
    started: str | None = None
    resources: ResourceReader = field(init=False, repr=False)

    def __post_init__(self) -> None:
        self.resources = ResourceReader.for_run(self.config, self.report)

    @property
    def engine(self) -> DecisionEngine:
        """The run's DecisionEngine (available from the Model_Bundle stage on)."""
        if self.analyzer is None:
            raise RuntimeError("the DecisionEngine is built in the Model_Bundle stage")
        return self.analyzer.engine

    @property
    def documents(self) -> DocumentStore:
        if self.ingested is None:
            raise RuntimeError("the documents are read in the Ingestor stage")
        return self.ingested.store

    def rng(self, purpose: str, *keys: Any) -> np.random.Generator:
        """The random stream for ``(seed, purpose, *keys)`` (``core.determinism.rng``)."""
        return rng(purpose, *keys, seed=self.config.seed)

    def resource(self, name: str) -> int:
        """Read integer Resource_Parameter ``name`` and list it in the report with its origin (20.5)."""
        return int(self.resources.read(name))

    def runs(self, tid: str) -> bool:
        """Whether Technique ``tid`` runs: registered as implemented and enabled."""
        return self.registry.technique(tid).implemented and self.engine.enabled(tid)

    def unwritten(self, tids: Iterable[str], component: str) -> None:
        """Raise ``NotImplementedError`` for a running Technique whose step this pipeline lacks."""
        for tid in tids:
            if self.runs(tid):
                raise NotImplementedError(f"{tid} is registered as implemented, but the {component} has no step for it")

    def stage(self, component: str) -> AbstractContextManager[None]:
        """Run one stage: timed under ``component``; unrecoverable errors become InferenceError (27.10, 27.13)."""
        return _stage(self.report, component)


@contextmanager
def _stage(report: InferenceReport, component: str) -> Iterator[None]:
    """Time one stage in ``report``; an exception that leaves it is unrecoverable unless it is a ConfigError."""
    try:
        with report.stage(component):
            yield
    except ConfigError:
        raise  # 27.11, 1.10: a configuration or usage error, reported as such
    except InferenceError as error:
        if error.partial_report is None:
            error.partial_report = report
        raise
    except Exception as error:
        info = ErrorInfo.from_exception(error)
        message = f"stage {component} failed: {info.type}: {info.message}"
        raise InferenceError(message, component, report) from error


@dataclass(frozen=True, eq=False)
class PipelineResult:
    """The outputs of one run (see the module docstring, In-memory runs)."""

    config: RunConfiguration
    model: SchemaModel
    document: dict[str, JSON] | bool
    pointers: PointerMap
    schema: bytes
    explanation: ExplanationFile
    report: InferenceReport
    soundness: SoundnessResult
    documents: DocumentStore
    origins: tuple[DocOrigin, ...]
    store: ObservationStore
    flagged: tuple[int, ...]


# ---------------------------------------------------------------------------
# Stages
# ---------------------------------------------------------------------------


def _jsinfer_version() -> str:
    import jsinfer  # deferred: the package's public API imports this module

    return jsinfer.__version__


def locked_package_versions() -> dict[str, str]:
    """The installed version of each :data:`LOCKED_PACKAGES` entry (``importlib.metadata``; 27.8).

    A package that is not installed has no version to report and is left
    out. The mapping is in code-point order of the names.
    """
    versions: dict[str, str] = {}
    for name in LOCKED_PACKAGES:
        try:
            versions[name] = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            continue
    return versions


def run_timestamp() -> str:
    """The current UTC time as an RFC 3339 timestamp with microseconds, ``2026-01-31T12:00:00.000000Z``."""
    return _wall_clock().astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _open_bundle(ctx: InferenceContext) -> None:
    """Model_Bundle stage: load and resolve the bundle, build the analyzer, record the run metadata."""
    config = ctx.config
    bundle = open_bundle(config.model_bundle, config.techniques)
    for substitution in bundle.substitutions:
        ctx.report.record_substitution(substitution)
    ctx.bundle = bundle
    ctx.analyzer = StructureAnalyzer.for_run(config, bundle, ctx.report, validator=ctx.validator)
    ctx.report.set_run(
        RunInfo(
            config=config.to_json(),
            seed=config.seed,
            bundle_id=bundle.bundle_id,
            jsinfer_version=_jsinfer_version(),
            python_version=platform.python_version(),
            packages=locked_package_versions(),
            started=ctx.started,
        )
    )


def _ingest(ctx: InferenceContext, source: Source) -> None:
    """Ingestor stage: read the documents and record every skipped document or file (1.7, 1.8)."""
    ingestor = Ingestor(ctx.config.input, ctx.resource(WORKERS), ctx.resource(CHUNK_SIZE))
    try:
        ingested = source.ingest(ingestor)
    except NoDocumentsError as error:
        for skipped in error.errors:
            ctx.report.record_ingest_error(skipped)
        raise
    for skipped in ingested.errors:
        ctx.report.record_ingest_error(skipped)
    ctx.ingested = ingested


def _flatten_chunk(payload: bytes, offsets: np.ndarray, first: int) -> PartialStore:
    """Parse one chunk's canonical documents and flatten them (runs in a joblib worker)."""
    bounds = offsets.tolist()
    docs = [parse_document(payload[start:stop])[0] for start, stop in zip(bounds, bounds[1:])]
    return flatten_chunk(docs, first)


def _flatten(ctx: InferenceContext) -> None:
    """ObservationStore stage: flatten fixed-size chunks in at most ``workers`` processes, merged in order."""
    documents = ctx.documents
    size = ctx.resource(CHUNK_SIZE)
    offsets = documents.offsets
    n_docs = len(documents)
    firsts = range(0, n_docs, size)
    task = delayed(single_threaded(_flatten_chunk))
    tasks = []
    for first in firsts:
        last = min(first + size, n_docs)
        low, high = int(offsets[first]), int(offsets[last])
        tasks.append(task(documents.buffer[low:high], offsets[first : last + 1] - low, first))
    workers = min(ctx.resource(WORKERS), max(len(tasks), 1))
    parts = Parallel(n_jobs=workers)(tasks)
    ctx.store = merge_partials(parts)


def _detect_outliers(ctx: InferenceContext) -> None:
    """Outlier_Detector stage hook (17.1, 17.6, 17.7; task 38.1): no document is flagged yet."""
    ctx.unwritten(_FLAGGING_TECHNIQUES, "Outlier_Detector")  # flagging needs the 17.9 abandonment check
    if not any(ctx.engine.enabled(tid) for tid in OUTLIER_TECHNIQUES):
        ctx.engine.resolve(DecisionPoint(DOCUMENT_OUTLIER), [])  # 17.7, 28.4: every document is included


def _detect_versions(ctx: InferenceContext) -> None:
    """Version_Detector stage hook (18.1–18.11; tasks 48.1, 48.2): one segment holding every document."""
    ctx.unwritten(CHANGE_POINT_TECHNIQUES, "Version_Detector")
    if not any(ctx.engine.enabled(tid) for tid in CHANGE_POINT_TECHNIQUES):
        ctx.engine.resolve(DecisionPoint(VERSION_SPLIT), [])  # 18.10, 28.4: a single schema
        ctx.report.set_versions(VersionsInfo.not_applied())
    ctx.segments = (_Segment(),)


def _analyze(ctx: InferenceContext, segment: _Segment) -> SchemaModel:
    """Structure_Analyzer stage: the Schema_Model of the segment's non-flagged documents."""
    assert ctx.analyzer is not None and ctx.store is not None
    view = View.root(ctx.store) if segment.docs is None else View.root(ctx.store, segment.docs)
    return ctx.analyzer.infer_model(view)


def _check_soundness(ctx: InferenceContext, segment: _Segment, model: SchemaModel) -> SoundnessResult:
    """Soundness stage: validate and widen until every non-flagged document of the segment validates (3.2–3.7)."""
    assert ctx.store is not None
    return check_soundness(
        model,
        ctx.documents,
        engine=ctx.engine,
        validator=ctx.validator,
        report=ctx.report,
        flagged=flagged_documents(ctx.store),
        segment=segment.docs,
        version=segment.version,
        max_passes=ctx.resource(MAX_PASSES),
        workers=ctx.resource(WORKERS),
        chunk_size=ctx.resource(CHUNK_SIZE),
    )


def _key_constraints(model: SchemaModel) -> dict[int, int]:
    """The ``propertyNames`` subschemas of the model: their nid → the nid of the node that holds them."""
    found: dict[int, int] = {}
    for _, node in model.walk():
        names = None if node.boolean is not None else node.kw.get(_PROPERTY_NAMES)
        if isinstance(names, SchemaNode):
            found[names.nid] = node.nid
    return found


def finalize_provenance(
    model: SchemaModel,
    documents: DocumentStore | Sequence[JSON],
    *,
    docs: Iterable[int] | np.ndarray | None = None,
    flagged_items: Mapping[int, Sequence[Sequence[str]]] | None = None,
    validator: Validator | None = None,
) -> PointerMap:
    """Give every emitted Constraint of ``model`` its final Support and Confidence (see the module docstring).

    ``docs`` are the documents the model covers that are not Flagged_Items
    themselves (all by default), and ``flagged_items`` the instance
    locations of the Flagged_Items inside them
    (``pipeline.soundness.flagged_item_locations``); every walked document
    without one must be valid against the model, as after the soundness
    check. Edits ``model.provenance`` in place and returns the emitted
    pointer map whose Constraints it finalized. Raises
    :class:`~jsinfer.core.provenance.ProvenanceError` when the records and
    the emitted Constraints do not correspond one to one.
    """
    if not isinstance(model, SchemaModel):
        raise TypeError(f"model must be a SchemaModel, got {type(model).__name__}")
    names = _key_constraints(model)
    counts = count_instances(
        model, documents, docs=docs, flagged_items=flagged_items, whole=names, validator=validator
    )
    pointers = counts.pointers
    check_records(pointers, model.provenance)
    shared: dict[ConstraintId, int] = {ConstraintId(owner, _PROPERTY_NAMES): nid for nid, owner in names.items()}
    for cid in pointers:
        if cid.nid in names:
            shared[cid] = cid.nid
    provenance = model.provenance
    for cid in pointers:
        node = shared.get(cid)
        support = counts.support(cid) if node is None else counts.node_support(node)
        provenance[cid] = finalize_record(provenance[cid], support)
    return pointers


def _finalize_provenance(ctx: InferenceContext, segment: _Segment, model: SchemaModel) -> None:
    """Provenance finalization (26.1, 26.2, 26.4, 26.10, 26.13; task 17.1) over the segment's documents."""
    assert ctx.store is not None
    store = ctx.store
    excluded = dict.fromkeys(store.flags.documents.tolist())
    covered = range(len(ctx.documents)) if segment.docs is None else segment.docs.tolist()
    finalize_provenance(
        model,
        ctx.documents,
        docs=[doc for doc in covered if doc not in excluded],
        flagged_items=flagged_item_locations(store),
        validator=ctx.validator,
    )


def _emit(
    ctx: InferenceContext, model: SchemaModel
) -> tuple[dict[str, JSON] | bool, PointerMap, bytes, ExplanationFile]:
    """Schema_Emitter stage: the layout, its pointer map, the canonical bytes, and the explanation file.

    The layout carries ``x-provenance`` when ``output.inline_provenance`` is
    on (2.1–2.3, 26.5–26.7).
    """
    emitter = SchemaEmitter()
    document, pointers = emitter.layout(model, inline_provenance=ctx.config.output.inline_provenance)
    return document, pointers, emitter.dumps(document), emitter.explanation_file(model, pointers)


# ---------------------------------------------------------------------------
# Entry points
# ---------------------------------------------------------------------------


def run_pipeline(source: Source, config: RunConfiguration | None = None) -> PipelineResult:
    """Run every stage on the documents of ``source`` under ``config`` (see the module docstring).

    ``config`` None means every default setting (27.2). Raises
    ``ConfigError``, ``NoDocumentsError``, or ``InferenceError`` as the
    module docstring describes.
    """
    if config is not None and not isinstance(config, RunConfiguration):
        raise TypeError(f"config must be a RunConfiguration or None, got {type(config).__name__}")
    if not callable(getattr(source, "ingest", None)):
        raise TypeError(f"source must be a PathSource, ValueSource, or JsonlSource, got {type(source).__name__}")
    started = run_timestamp()
    report = InferenceReport()
    with _stage(report, CONFIG_STAGE):
        effective = RunConfiguration.default() if config is None else config
    ctx = InferenceContext(effective, report, started=started)

    try:
        # Workers are launched only with more than one (design, Open Decisions, Standard input).
        with reserved_stdin(effective.workers > 1):
            with ctx.stage(BUNDLE_STAGE):
                _open_bundle(ctx)
            with ctx.stage(INGESTOR_STAGE):
                _ingest(ctx, source)
            with ctx.stage(FLATTEN_STAGE):
                _flatten(ctx)
            with ctx.stage(OUTLIER_STAGE):
                _detect_outliers(ctx)
            with ctx.stage(VERSION_STAGE):
                _detect_versions(ctx)
            (segment,) = ctx.segments  # one segment until change points exist (task 48.2)
            with ctx.stage(ANALYZER_STAGE):
                model = _analyze(ctx, segment)
            with ctx.stage(SOUNDNESS_STAGE):
                soundness = _check_soundness(ctx, segment, model)
                _finalize_provenance(ctx, segment, model)
            with ctx.stage(EMITTER_STAGE):
                document, pointers, schema, explanation = _emit(ctx, model)
    finally:
        if report.run is not None:  # recorded in the Model_Bundle stage; a failed run ends here too
            report.finish_run(run_timestamp())

    assert ctx.ingested is not None and ctx.store is not None
    return PipelineResult(
        config=effective,
        model=model,
        document=document,
        pointers=pointers,
        schema=schema,
        explanation=explanation,
        report=report,
        soundness=soundness,
        documents=ctx.ingested.store,
        origins=ctx.ingested.origins,
        store=ctx.store,
        flagged=tuple(int(doc) for doc in flagged_documents(ctx.store)),
    )


def infer_documents(
    documents: Iterable[JSON] | bytes | bytearray | memoryview, config: RunConfiguration | None = None
) -> PipelineResult:
    """Run the pipeline on in-memory documents: JSON values, or the bytes of a JSONL file (see the module docstring)."""
    return run_pipeline(_source(documents), config)


def infer_paths(paths: Sequence[str | PathLike[str]], config: RunConfiguration | None = None) -> PipelineResult:
    """Run the pipeline on files and directories, in argument order (1.1–1.3, 1.10)."""
    return run_pipeline(PathSource(paths), config)  # type: ignore[arg-type]  # PathSource takes any sequence
