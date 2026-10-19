"""Isolated system runs (task 28.1; Requirements 34.2, 34.3, 34.5–34.8, 34.11, 35.7).

:func:`run_system` runs one Baseline_Adapter (``bench.baselines.base``) on
one JSONL Document_Collection:

- the system runs in its own child process, started with
  ``start_new_session=True`` and standard input at ``/dev/null``, so a
  run past the time limit ``bench.run_timeout_s`` has its whole process
  group killed with ``os.killpg`` and is recorded as a timeout (34.7);
- runs are sequential, and every system reads the same JSONL file in the
  same order (:func:`write_jsonl`; 34.2, 34.3);
- wall-clock time is measured from the start of the child until it returns
  (34.3), and a psutil sampler sums the resident set size of the child and
  its descendants at :data:`SAMPLE_INTERVAL_S` intervals for the peak (35.7);
- a system that is not available is recorded as such without a run (34.6);
- a non-zero exit status, missing or unparseable output, or an output that
  fails the meta-schema of its declared draft (or the adapter's output
  draft when ``$schema`` is absent) is a failure with the status and the
  captured standard error (34.8);
- an output in draft-04, -06, -07, or 2019-09 is translated to draft
  2020-12 by the Schema_Loader, and the result records the source draft,
  the keywords the translation changed or left out, and the constructs it
  reported as unsupported (34.5).

The :class:`SystemRun` holds the outcome kind (``schema``, ``timeout``,
``failure``, or ``unavailable``), the loaded Schema_Model and its emitted
draft 2020-12 document when there is one, the measurements, and the
adapter's :class:`~jsinfer.bench.baselines.base.BaselineInfo` (34.11).
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import time
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Final, Literal

import psutil

from jsinfer.bench.baselines.base import Availability, BaselineAdapter, BaselineInfo
from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import encode_canonical
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.schema_model import SchemaModel
from jsinfer.io.emit import SchemaEmitter
from jsinfer.io.load import SchemaLoadError, SchemaLoader
from jsinfer.io.validator import Draft, Validator, dialect_of

__all__ = ["SAMPLE_INTERVAL_S", "OutcomeKind", "SystemRun", "Translation", "run_system", "run_systems", "write_jsonl"]

OutcomeKind = Literal["schema", "timeout", "failure", "unavailable"]
SAMPLE_INTERVAL_S: Final = 0.05
_STDERR_LIMIT: Final = 20_000  # characters of standard error kept with a failure


@dataclass(frozen=True)
class Translation:
    """How an older-draft output was translated to draft 2020-12 (34.5)."""

    source_draft: str
    translated_keywords: tuple[str, ...]  # keywords of the source document absent from the emitted one
    ignored: tuple[tuple[str, str], ...]
    unsupported: tuple[tuple[str, str], ...]

    def to_json(self) -> dict[str, JSON]:
        return {
            "source_draft": self.source_draft,
            "translated_keywords": list(self.translated_keywords),
            "ignored": [list(entry) for entry in self.ignored],
            "unsupported": [list(entry) for entry in self.unsupported],
        }


@dataclass(frozen=True)
class SystemRun:
    """The outcome of one system on one Document_Collection (see the module docstring)."""

    system: str
    kind: OutcomeKind
    info: BaselineInfo
    seconds: float | None = None
    peak_rss_mb: float | None = None
    exit_status: int | None = None
    stderr: str = ""
    reason: str | None = None
    model: SchemaModel | None = None
    document: dict[str, Any] | bool | None = None
    translation: Translation | None = None
    command: tuple[str, ...] = ()

    @property
    def ok(self) -> bool:
        return self.kind == "schema"

    def to_json(self) -> dict[str, JSON]:
        return {
            "system": self.system,
            "kind": self.kind,
            "info": self.info.to_json(),
            "seconds": self.seconds,
            "peak_rss_mb": self.peak_rss_mb,
            "exit_status": self.exit_status,
            "stderr": self.stderr,
            "reason": self.reason,
            "translation": None if self.translation is None else self.translation.to_json(),
            "command": list(self.command),
        }


def write_jsonl(documents: Iterable[JSON], path: Path) -> int:
    """Write ``documents`` as one JSON text per line, in order (34.2); returns the count."""
    count = 0
    with open(path, "w", encoding="utf-8") as handle:
        for document in documents:
            handle.write(json.dumps(document, ensure_ascii=False, allow_nan=False, separators=(",", ":")))
            handle.write("\n")
            count += 1
    return count


def _timeout(value: float | None) -> float:
    spec = RESOURCE_PARAMETERS["bench.run_timeout_s"]
    chosen = spec.default if value is None else value
    problem = spec.check(chosen)
    if problem is not None:
        raise ValueError(f"bench.run_timeout_s: {problem}")
    return float(chosen)  # type: ignore[arg-type]


def _tree_rss(process: psutil.Process) -> int:
    total = 0
    try:
        total += process.memory_info().rss
        for child in process.children(recursive=True):
            try:
                total += child.memory_info().rss
            except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
                continue
    except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
        pass
    return total


def _kill_group(child: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(os.getpgid(child.pid), signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass
    try:
        child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        pass


def _execute(command: Sequence[str], out_dir: Path, timeout_s: float, env: dict[str, str] | None) -> tuple[int | None, float, float, bool, str]:
    """Run ``command`` isolated: ``(exit status, seconds, peak RSS in MB, timed out, stderr)``."""
    stderr_path = out_dir / "stderr.txt"
    stdout_path = out_dir / "stdout.txt"
    peak = 0
    started = time.monotonic()
    with open(stdout_path, "wb") as out, open(stderr_path, "wb") as err:
        child = subprocess.Popen(
            list(command), cwd=out_dir, stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True, env=env
        )
        timed_out = False
        try:
            watched = psutil.Process(child.pid)
        except psutil.NoSuchProcess:
            watched = None
        while True:
            if watched is not None:
                peak = max(peak, _tree_rss(watched))
            try:
                child.wait(timeout=SAMPLE_INTERVAL_S)
                break
            except subprocess.TimeoutExpired:
                if time.monotonic() - started > timeout_s:
                    timed_out = True
                    _kill_group(child)
                    break
    seconds = time.monotonic() - started
    stderr = stderr_path.read_text(encoding="utf-8", errors="replace")[-_STDERR_LIMIT:]
    return (None if timed_out else child.returncode), seconds, peak / (1024 * 1024), timed_out, stderr


def _load_output(
    path: Path, adapter: BaselineAdapter, validator: Validator, loader: SchemaLoader
) -> tuple[SchemaModel, dict[str, Any] | bool, Translation | None] | str:
    """The loaded model, its 2020-12 document, and the translation record, or the failure reason (34.5, 34.8)."""
    if not path.is_file():
        return f"no output file {path.name}"
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        return f"the output is not a JSON document: {error}"
    if not isinstance(document, (dict, bool)):
        return "the output is not a JSON Schema document (object or boolean)"
    if isinstance(document, dict) and "$schema" not in document and adapter.output_draft is not None:
        document = {"$schema": adapter.output_draft.uri, **document}
    draft, _ = dialect_of(document)
    errors = validator.meta_errors(document, draft)
    if errors:
        return f"the output fails the {draft.name} meta-schema: {errors[0]}"
    try:
        model = loader.load(document)
    except SchemaLoadError as error:
        return f"the Schema_Loader rejects the output: {error}"
    emitted, _ = SchemaEmitter().layout(model)
    translation = None
    if draft is not Draft.DRAFT202012:
        source_keywords = _keywords(document)
        emitted_keywords = _keywords(emitted)
        translation = Translation(
            draft.name,
            tuple(sorted(k for k in source_keywords if k not in emitted_keywords)),
            tuple(model.ignored),
            tuple(model.unsupported),
        )
    return model, emitted, translation


def _keywords(document: Any) -> set[str]:
    found: set[str] = set()
    stack = [document]
    while stack:
        value = stack.pop()
        if isinstance(value, dict):
            found.update(value)
            stack.extend(value.values())
        elif isinstance(value, list):
            stack.extend(value)
    return found


def run_system(
    adapter: BaselineAdapter,
    docs_jsonl: Path,
    out_dir: Path,
    *,
    seed: int = 0,
    timeout_s: float | None = None,
    validator: Validator | None = None,
    loader: SchemaLoader | None = None,
    env: dict[str, str] | None = None,
) -> SystemRun:
    """Run ``adapter`` on ``docs_jsonl``, writing into ``out_dir`` (see the module docstring)."""
    limit = _timeout(timeout_s)
    info = adapter.describe()
    availability = adapter.available()
    if not availability.available:
        return SystemRun(adapter.name, "unavailable", info, reason=availability.reason)
    out_dir.mkdir(parents=True, exist_ok=True)
    command = tuple(str(part) for part in adapter.command(docs_jsonl.resolve(), out_dir.resolve(), seed))
    status, seconds, peak, timed_out, stderr = _execute(command, out_dir, limit, env)
    if timed_out:
        return SystemRun(adapter.name, "timeout", info, seconds=seconds, peak_rss_mb=peak, stderr=stderr, reason=f"exceeded {limit} s", command=command)
    if status != 0:
        return SystemRun(adapter.name, "failure", info, seconds=seconds, peak_rss_mb=peak, exit_status=status, stderr=stderr, reason=f"exit status {status}", command=command)
    loaded = _load_output(adapter.output_path(out_dir), adapter, validator or Validator(), loader or SchemaLoader())
    if isinstance(loaded, str):
        return SystemRun(adapter.name, "failure", info, seconds=seconds, peak_rss_mb=peak, exit_status=status, stderr=stderr, reason=loaded, command=command)
    model, document, translation = loaded
    (out_dir / "schema-2020-12.json").write_bytes(encode_canonical(document))
    return SystemRun(adapter.name, "schema", info, seconds=seconds, peak_rss_mb=peak, exit_status=status, stderr=stderr, model=model, document=document, translation=translation, command=command)


def run_systems(
    adapters: Sequence[BaselineAdapter], docs_jsonl: Path, out_root: Path, *, seed: int = 0, timeout_s: float | None = None, env: dict[str, str] | None = None
) -> list[SystemRun]:
    """Run every adapter in turn on the same JSONL file (34.2, 34.3), each in ``out_root/<system>``."""
    validator, loader = Validator(), SchemaLoader()
    return [run_system(a, docs_jsonl, out_root / a.name, seed=seed, timeout_s=timeout_s, validator=validator, loader=loader, env=env) for a in adapters]


@dataclass(frozen=True)
class CommandAdapter:
    """A Baseline_Adapter for any command that reads a JSONL file and writes one schema file (tests and wrappers).

    ``argv`` may contain ``{docs}``, ``{out}``, and ``{seed}`` placeholders.
    """

    name: str
    label: str
    argv: tuple[str, ...]
    output: str = "schema.json"
    output_draft: Draft | None = None
    info: BaselineInfo | None = None
    availability: Availability = field(default_factory=lambda: Availability(True))

    def available(self) -> Availability:
        return self.availability

    def command(self, docs_jsonl: Path, out_dir: Path, seed: int) -> Sequence[str]:
        return [part.format(docs=docs_jsonl, out=out_dir, seed=seed) for part in self.argv]

    def output_path(self, out_dir: Path) -> Path:
        return out_dir / self.output

    def describe(self) -> BaselineInfo:
        return self.info if self.info is not None else BaselineInfo(self.name, self.label, "unknown", "unknown", {}, self.argv)


__all__ += ["CommandAdapter"]
