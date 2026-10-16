"""``InferenceError`` and ``NoDocumentsError``: errors that end a run after its configuration was accepted.

Configuration and usage errors, including rejected input paths (1.10), are
:class:`~jsinfer.config.errors.ConfigError` and are raised before any
document is ingested. The errors here come later, from a pipeline stage:

- :class:`InferenceError` is an unrecoverable stage error (27.10, 27.13). It
  names the failed ``stage`` (a component name, as in
  ``InferenceReport.stage``) and carries the ``partial_report``. A stage
  that raises does not hold the run's report, so the pipeline attaches it
  (``error.partial_report = report``) before the error leaves the public
  API. The CLI maps it to exit status 1.
- :class:`NoDocumentsError` is raised by the Ingestor when ingestion ends
  with zero successfully parsed documents (1.8). It carries the ingestion
  errors (:class:`~jsinfer.core.report.IngestError`, in ingestion order) so
  the pipeline can record them in the report. The CLI maps it to exit
  status 3.

The public API re-exports both as ``jsinfer.InferenceError`` and
``jsinfer.NoDocumentsError``. Both pickle.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import Any, Final

from jsinfer.core.report import IngestError, InferenceReport

__all__ = ["INGESTOR_STAGE", "InferenceError", "NoDocumentsError"]

# The component name under which the Ingestor is timed and reported.
INGESTOR_STAGE: Final = "Ingestor"


class InferenceError(Exception):
    """An unrecoverable error of one pipeline stage (27.10, 27.13).

    ``stage`` is the failed component's name; ``partial_report`` is the
    report of the run up to the failure, or ``None`` until the pipeline
    attaches it.
    """

    def __init__(self, message: str, stage: str, partial_report: InferenceReport | None = None) -> None:
        if not isinstance(message, str) or not message:
            raise ValueError("InferenceError message must be a non-empty str")
        if not isinstance(stage, str) or not stage:
            raise ValueError("InferenceError stage must be a non-empty str")
        if partial_report is not None and not isinstance(partial_report, InferenceReport):
            raise TypeError(f"partial_report must be an InferenceReport or None, got {type(partial_report).__name__}")
        super().__init__(message, stage, partial_report)  # args survive pickling
        self.message = message
        self.stage = stage
        self.partial_report = partial_report

    def __str__(self) -> str:
        return self.message


class NoDocumentsError(InferenceError):
    """Ingestion ended with zero successfully parsed documents (1.8).

    ``errors`` holds every skipped document or file in ingestion order, and
    ``skipped`` is their count. The message states that no valid documents
    were found.
    """

    def __init__(self, errors: Iterable[IngestError] = (), partial_report: InferenceReport | None = None) -> None:
        found = tuple(errors)
        for error in found:
            if not isinstance(error, IngestError):
                raise TypeError(f"NoDocumentsError errors must be IngestError values, got {type(error).__name__}")
        message = "no valid documents were found"
        if found:
            noun = "document or file" if len(found) == 1 else "documents or files"
            message = f"{message}; skipped {len(found)} {noun} that could not be parsed or read"
        super().__init__(message, INGESTOR_STAGE, partial_report)
        self.errors: tuple[IngestError, ...] = found

    @property
    def skipped(self) -> int:
        """The number of skipped documents and files."""
        return len(self.errors)

    def __reduce__(self) -> tuple[Any, tuple[Any, ...]]:
        return (type(self), (self.errors, self.partial_report))
