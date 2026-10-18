"""jsinfer: infer draft 2020-12 JSON Schemas from collections of JSON documents.

The public API (Requirement 27.9, 27.13; design, Public API and CLI)
    ====================================  =============================================
    :func:`infer`                         a full run on parsed documents (Python values)
    :func:`infer_files`                   a full run on files and directories (1.1–1.3)
    :func:`load_schema`                   the Schema_Loader: a schema document → its
                                          Schema_Model (2.4–2.6, 2.10–2.12)
    :func:`emit_schema`                   the Schema_Emitter: a Schema_Model → canonical
                                          bytes, optionally with ``x-provenance`` (2.1–2.3,
                                          26.6)
    :func:`load_explanation`              read an explanation file back (26.8)
    :func:`explain`                       one Provenance_Record as plain text (26.9)
    :class:`InferenceResult`              what a run returns
    :class:`RunConfiguration`             the settings of a run (27.2–27.4)
    ====================================  =============================================

    Both inference functions run the pipeline of ``pipeline.infer`` and
    write nothing. ``config`` None means every default setting (27.2),
    ``workers`` = the CPU count included; build others with
    ``RunConfiguration.resolve(flags)``, for example
    ``RunConfiguration.resolve({"workers": 1, "output.inline_provenance": True})``.
    They read no standard input, and work in a program whose descriptor 0
    is closed: a run with more than one worker holds ``os.devnull`` there
    while it runs and closes it afterwards (27.1; ``core.workers``).

In-memory documents (27.9)
    :func:`infer` reads ``documents`` once, in full, and writes value ``k``
    of the iterable as line ``k + 1`` of a virtual ``.jsonl`` file named
    ``<documents>`` (``io.ingest.documents_to_jsonl``) before any stage
    runs. The Ingestor then parses those lines and stores each document's
    canonical encoding in the DocumentStore, exactly as for a ``.jsonl``
    file on disk, so every later stage is the one the CLI runs: for the
    same documents in the same order under the same Run_Configuration,
    :func:`infer` and ``jsinfer infer`` on a ``.jsonl`` file holding them
    give equal Schema_Models and the same schema and explanation bytes
    (27.9). Array unwrapping never applies to lines.

    Non-JSON values are rejected, never converted. Each value must be, at
    any depth, a ``dict`` with ``str`` keys, a ``list``, a ``str``, an
    ``int``, a ``float``, a ``bool``, or ``None`` (the types 27.9 lists;
    subclasses are written as their base type, as :mod:`json` writes them).
    Anything else raises ``TypeError`` naming the document's position in
    the iterable and the JSON Pointer of the value: tuples, sets, bytes,
    ``Decimal``, and non-``str`` keys among them. A container that contains
    itself raises ``ValueError``. Both are raised before any stage runs and
    carry no report. ``json.dumps``'s conversions (tuples to arrays,
    ``int``/``float``/``bool``/``None`` keys to strings) are not applied:
    they can merge keys (``{1: "a", "1": "b"}``) and change a document
    without any record of it, and the documents 27.9 describes never need
    them.

    Values inside that domain that RFC 8259 cannot carry, or that jsinfer
    rejects as parse failures, are handled as the CLI handles the same
    documents in a ``.jsonl`` file (1.7, 1.11): NaN and the infinities
    (which Python's :mod:`json` reads and writes by default), strings with
    an unpaired surrogate, integers past the integer string conversion
    limit, and nesting deeper than ``io.ingest.MAX_NESTING_DEPTH``. Each
    such document is skipped and recorded in the report's ``ingestion``
    section with source ``<documents>``, line ``k + 1``, and the reason the
    file's line would get; the run continues. When no document remains,
    :class:`NoDocumentsError` is raised (1.8).

InferenceResult and the output files (27.7, 27.9)
    ``schema`` is the Schema_Model, ``provenance`` the explanation file
    (every Provenance_Record keyed by the JSON Pointer of its Constraint;
    ``schema.provenance`` holds the same records keyed by ConstraintId),
    ``report`` the Inference_Report, ``config`` the effective
    Run_Configuration, ``document`` the emitted schema document and
    ``pointers`` its ConstraintId → JSON Pointer map, and ``schema_bytes``
    the canonical bytes of ``document``. ``document`` and ``schema_bytes``
    carry ``x-provenance`` exactly when ``output.inline_provenance`` is on
    (26.6); everything else is the same either way. All of them come from
    the pipeline's Schema_Emitter stage, so an emission error is a stage
    error.

    :meth:`InferenceResult.files` returns each output file's name and bytes,
    in this order, so the CLI writes them without recomputing anything:

    =========================  ================================================
    ``schema.json``            ``schema_bytes``
    ``explanation.json``       :meth:`~InferenceResult.explanation_bytes`
                               (``ExplanationFile.dumps``; 26.5, 26.8)
    ``report.json``            :meth:`~InferenceResult.report_bytes`
                               (``InferenceReport.to_bytes``; 27.8)
    ``effective-config.toml``  :meth:`~InferenceResult.config_bytes`
                               (``RunConfiguration.to_toml`` as UTF-8; 27.7)
    =========================  ================================================

    ``versions``, ``combined``, and ``diffs`` hold the Schema_Versions, the
    combined schema, and their diffs once change points exist (Requirement
    18, task 48.2); until then they are empty and ``schema`` is the single
    schema. A result does not keep the DocumentStore or the
    ObservationStore, so their memory is freed when the call returns.

Errors (27.10, 27.11, 27.13)
    - :class:`ConfigError`: an invalid Run_Configuration (raised by
      ``RunConfiguration.resolve``), an unloadable Model_Bundle, or a
      rejected supplied path (:func:`infer_files`); raised before any
      document is ingested (1.10, 27.11, 28.8, 28.9, 29.11).
    - :class:`NoDocumentsError`: no document could be ingested, because the
      iterable was empty or every document or file was skipped (1.8). It is
      an :class:`InferenceError` of stage ``Ingestor``.
    - :class:`InferenceError`: an unrecoverable stage error. ``stage`` names
      the component, ``partial_report`` is the run's report up to the
      failure, and the original exception is the ``__cause__`` (27.10,
      27.13).
    - ``TypeError`` and ``ValueError``: documents that are not JSON values
      (above), and arguments of the wrong type.
    - :class:`SchemaLoadError`: :func:`load_schema` input that is not JSON,
      has a root that is neither an object nor a boolean, or fails its
      draft's meta-schema (2.12).
    - :class:`EmitError`: :func:`emit_schema` of a model that cannot be
      emitted, or with ``inline_provenance`` when its records do not match
      its Constraints one to one (a loaded model has no records).
    - :class:`ExplanationLoadError`: :func:`load_explanation` input that is
      not an explanation file.
    - :class:`ExplainError`: an :func:`explain` request that prints no
      record (26.12).

    Every error leaves the calling process running (27.13).

Imports
    ``import jsinfer`` loads only this module. The re-exported names load
    their defining modules on first access (PEP 562 ``__getattr__``), and
    the functions import the pipeline when called, so commands that never
    run it (``jsinfer --version``, ``jsinfer list-techniques``) start
    without importing numpy, scikit-learn, or jsonschema-rs.
"""

from __future__ import annotations

import importlib
import os
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _version
from types import MappingProxyType
from typing import TYPE_CHECKING, Any, Final

if TYPE_CHECKING:
    from jsinfer.config.errors import ConfigError
    from jsinfer.config.run_config import RunConfiguration
    from jsinfer.core.errors import InferenceError, NoDocumentsError
    from jsinfer.core.jsonvalue import JSON
    from jsinfer.core.provenance import ExplainError
    from jsinfer.core.report import InferenceReport
    from jsinfer.core.schema_model import ConstraintId, SchemaModel
    from jsinfer.io.emit import EmitError, ExplanationFile
    from jsinfer.io.explain import explain
    from jsinfer.io.load import ExplanationLoadError, SchemaLoadError, load_explanation
    from jsinfer.pipeline.infer import PipelineResult

# The re-exported names and the modules that define them. They are loaded on
# first access (PEP 562), so `import jsinfer` (and with it `import
# jsinfer.cli`, whose console script runs `jsinfer --version` and `jsinfer
# list-techniques`) does not import the pipeline and its scientific stack.
# `from jsinfer import X` and `jsinfer.X` work as for eager imports, and give
# the defining module's object.
_LAZY_EXPORTS: Final[Mapping[str, str]] = MappingProxyType(
    {
        "ConfigError": "jsinfer.config.errors",
        "EmitError": "jsinfer.io.emit",
        "ExplainError": "jsinfer.core.provenance",
        "ExplanationLoadError": "jsinfer.io.load",
        "InferenceError": "jsinfer.core.errors",
        "NoDocumentsError": "jsinfer.core.errors",
        "RunConfiguration": "jsinfer.config.run_config",
        "SchemaLoadError": "jsinfer.io.load",
        "explain": "jsinfer.io.explain",
        "load_explanation": "jsinfer.io.load",
    }
)


def __getattr__(name: str) -> Any:
    """Load a re-exported name from its module on first access (see ``_LAZY_EXPORTS``)."""
    module = _LAZY_EXPORTS.get(name)
    if module is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    value = getattr(importlib.import_module(module), name)
    globals()[name] = value  # later accesses skip this hook
    return value


def __dir__() -> list[str]:
    return sorted([*globals(), *(name for name in _LAZY_EXPORTS if name not in globals())])


# The pipeline reads __version__ when a run records its metadata, not at import
# time (pipeline.infer._jsinfer_version imports this package lazily).
try:
    __version__ = _version("jsinfer")
except PackageNotFoundError:  # running from a source tree without an install
    __version__ = "0+unknown"

__all__ = [
    "ConfigError",
    "EmitError",
    "ExplainError",
    "ExplanationLoadError",
    "InferenceError",
    "InferenceResult",
    "NoDocumentsError",
    "RunConfiguration",
    "SchemaLoadError",
    "__version__",
    "emit_schema",
    "explain",
    "infer",
    "infer_files",
    "load_explanation",
    "load_schema",
]

# The output files of a run, in the order InferenceResult.files lists them (27.7).
SCHEMA_FILE: Final = "schema.json"
EXPLANATION_FILE: Final = "explanation.json"
REPORT_FILE: Final = "report.json"
CONFIG_FILE: Final = "effective-config.toml"


@dataclass(frozen=True, eq=False)
class InferenceResult:
    """The outputs of one inference run (see the module docstring, InferenceResult and the output files).

    Treat the mappings and the model as read-only. Results compare by
    identity; compare ``schema`` (2.7 equality) or the bytes instead.
    """

    schema: SchemaModel
    provenance: ExplanationFile
    report: InferenceReport
    config: RunConfiguration
    document: dict[str, JSON] | bool
    pointers: Mapping[ConstraintId, str]
    schema_bytes: bytes
    versions: tuple[()] = ()
    combined: None = None
    diffs: tuple[()] = ()

    def explanation_bytes(self) -> bytes:
        """The bytes of ``explanation.json``: ``provenance.dumps()`` (26.5, 26.8)."""
        return self.provenance.dumps()

    def report_bytes(self) -> bytes:
        """The bytes of ``report.json``: the canonical JSON of the Inference_Report (27.8)."""
        return self.report.to_bytes()

    def config_bytes(self) -> bytes:
        """The bytes of ``effective-config.toml``: the effective Run_Configuration as TOML in UTF-8 (27.4, 27.7)."""
        return self.config.to_toml().encode("utf-8")

    def files(self) -> dict[str, bytes]:
        """Each output file's name and bytes, in a fixed order (see the module docstring)."""
        return {
            SCHEMA_FILE: self.schema_bytes,
            EXPLANATION_FILE: self.explanation_bytes(),
            REPORT_FILE: self.report_bytes(),
            CONFIG_FILE: self.config_bytes(),
        }

    def __repr__(self) -> str:
        status = "ok" if self.report.ok else "failed"
        return (
            f"<InferenceResult: {len(self.schema_bytes)} schema bytes, "
            f"{len(self.provenance.records)} Provenance_Records, status {status}>"
        )


def _result(run: PipelineResult) -> InferenceResult:
    return InferenceResult(
        schema=run.model,
        provenance=run.explanation,
        report=run.report,
        config=run.config,
        document=run.document,
        pointers=run.pointers,
        schema_bytes=run.schema,
    )


def _check_config(config: RunConfiguration | None) -> None:
    from jsinfer.config.run_config import RunConfiguration

    if config is not None and not isinstance(config, RunConfiguration):
        raise TypeError(
            f"config must be a RunConfiguration or None, got {type(config).__name__}; "
            "build one with RunConfiguration.resolve(flags)"
        )


def infer(documents: Iterable[JSON], config: RunConfiguration | None = None) -> InferenceResult:
    """Infer a schema from parsed documents, in iteration order, under ``config`` (27.9).

    ``documents`` is an iterable of JSON values (see the module docstring,
    In-memory documents); it is read once, before any stage runs. Raises
    ``TypeError`` or ``ValueError`` for documents that are not JSON values,
    :class:`ConfigError` for an unloadable Model_Bundle,
    :class:`NoDocumentsError` when no document can be ingested, and
    :class:`InferenceError` for an unrecoverable stage error (27.13).
    """
    from jsinfer.pipeline.infer import infer_documents

    _check_config(config)
    if isinstance(documents, (bytes, bytearray, memoryview)):
        raise TypeError(
            f"documents must be an iterable of JSON values, got {type(documents).__name__}; "
            "use infer_files for JSONL files"
        )
    return _result(infer_documents(documents, config))


def infer_files(paths: Sequence[str | os.PathLike[str]], config: RunConfiguration | None = None) -> InferenceResult:
    """Infer a schema from the documents of files and directories, in argument order (1.1–1.3, 1.5).

    Raises :class:`ConfigError` naming every rejected supplied path before
    anything is read (1.10), and otherwise as :func:`infer` does.
    """
    from jsinfer.pipeline.infer import infer_paths

    _check_config(config)
    return _result(infer_paths(paths, config))


def load_schema(source: bytes | str | dict[str, JSON] | bool) -> SchemaModel:
    """Load a JSON Schema document into a Schema_Model (the Schema_Loader; 2.4–2.6, 2.10–2.12).

    ``source`` is the document's bytes, its text (a ``str`` is never a
    path), or its parsed value. Documents of drafts 04 to 2019-09 are
    translated to draft 2020-12 (2.5). The model has no Provenance_Records.
    Raises :class:`SchemaLoadError` when the document is not JSON, its root
    is neither an object nor a boolean, or it fails its draft's meta-schema
    (2.12), and ``TypeError`` for another type of ``source``.
    """
    from jsinfer.io.load import SchemaLoader

    return SchemaLoader().load(source)


def emit_schema(model: SchemaModel, *, inline_provenance: bool = False) -> bytes:
    """The canonical draft 2020-12 bytes of ``model`` (the Schema_Emitter; 2.1–2.3).

    With ``inline_provenance``, each schema object holding a Constraint gets
    an ``x-provenance`` keyword built from ``model.provenance`` (26.6), so
    every Constraint needs exactly one record. Raises :class:`EmitError`
    when the model cannot be emitted or its records do not match its
    Constraints.
    """
    from jsinfer.io.emit import SchemaEmitter

    return SchemaEmitter().emit(model, inline_provenance=inline_provenance)
