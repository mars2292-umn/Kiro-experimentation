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
import shutil
import signal
import subprocess
import time
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Final, Literal

import psutil

from jsinfer.bench.baselines.base import Availability, BaselineAdapter, BaselineInfo
from jsinfer.bench.config import BenchConfig
from jsinfer.bench.leakage import LeakageError, LeakageReport, check_leakage
from jsinfer.bench.splits import CorpusManifest, SchemaSplit
from jsinfer.config.resources import RESOURCE_PARAMETERS
from jsinfer.core.determinism import codepoint_key, encode_canonical
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
    availability: Availability | None = None  # None: available when the executable can be found

    def available(self) -> Availability:
        if self.availability is not None:
            return self.availability
        executable = self.argv[0] if self.argv else ""
        if shutil.which(executable) is None and not Path(executable).is_file():
            return Availability(False, f"no executable {executable}")
        return Availability(True)

    def command(self, docs_jsonl: Path, out_dir: Path, seed: int) -> Sequence[str]:
        return [part.format(docs=docs_jsonl, out=out_dir, seed=seed) for part in self.argv]

    def output_path(self, out_dir: Path) -> Path:
        return out_dir / self.output

    def describe(self) -> BaselineInfo:
        return self.info if self.info is not None else BaselineInfo(self.name, self.label, "unknown", "unknown", {}, self.argv)


__all__ += ["CommandAdapter"]


# ---------------------------------------------------------------------------
# The Harness facade (task 30.5; Requirements 30.7, 30.8, 30.11, 34.3)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class HarnessSettings:
    """The Harness's own ``[bench]`` and ``[baselines]`` settings of ``bench.toml``.

    ``model_bundle`` is the Model_Bundle whose training manifest the leakage
    check reads (default ``bundles/default``); ``results`` the directory
    the results files go to; ``resamples`` the bootstrap resample count
    (``bootstrap.resamples`` when None); ``real_manifest`` the Real_Dataset
    manifest (33.1); ``schemastore_datasets`` whether SchemaStore test sets
    become Real_Datasets (33.2). ``[baselines]`` holds ``naive_merge``
    (default on), ``[baselines.jsonoid]`` (``bench.baselines.jsonoid``),
    and ``[baselines.command.<name>]`` tables with ``argv`` (placeholders
    ``{docs}``, ``{out}``, ``{seed}``), ``output``, and ``label`` for any
    other command-line system.
    """

    model_bundle: str = "bundles/default"
    results: str = "results"
    resamples: int | None = None
    real_manifest: str | None = None
    schemastore_datasets: bool = True
    naive_merge: bool = True
    jsonoid: Mapping[str, Any] | None = None
    commands: Mapping[str, Mapping[str, Any]] = field(default_factory=dict)
    inferrer_settings: Mapping[str, JSON] = field(default_factory=dict)
    ablations: tuple[str, ...] | None = None  # the Techniques to ablate (every enabled one when None)

    @classmethod
    def from_config(cls, cfg: BenchConfig) -> HarnessSettings:
        bench = cfg.extra.get("bench", {})
        baselines = cfg.extra.get("baselines", {})
        for name, table in (("bench", bench), ("baselines", baselines)):
            if not isinstance(table, Mapping):
                raise ValueError(f"[{name}] must be a table")
        unknown = sorted(k for k in baselines if k not in ("naive_merge", "jsonoid", "command", "inferrer"))
        if unknown:
            raise ValueError(f"unknown [baselines] settings {unknown}")
        commands = baselines.get("command", {})
        if not isinstance(commands, Mapping) or not all(isinstance(t, Mapping) and isinstance(t.get("argv"), list) for t in commands.values()):
            raise ValueError("[baselines.command.<name>] tables need an argv list")
        inferrer = baselines.get("inferrer", {})
        if not isinstance(inferrer, Mapping):
            raise ValueError("[baselines.inferrer] must be a table of --set settings")
        base = Path(cfg.manifest).parent.parent if cfg.manifest else Path(".")
        results = bench.get("results", "results")
        real = bench.get("real_manifest")
        ablations = bench.get("ablations")
        if ablations is not None and (not isinstance(ablations, list) or not all(isinstance(a, str) for a in ablations)):
            raise ValueError("[bench] ablations must be a list of Technique ids")
        bundle = str(bench.get("model_bundle", "bundles/default"))
        if (base / bundle).is_dir():  # a bundle directory under the project; otherwise an identifier load_bundle resolves
            bundle = str(base / bundle)
        return cls(
            bundle,
            str(_under(base, results)),
            bench.get("resamples"),
            None if real is None else str(_under(base, real)),
            bool(bench.get("schemastore_datasets", True)),
            bool(baselines.get("naive_merge", True)),
            baselines.get("jsonoid"),
            {name: dict(table) for name, table in commands.items()},
            dict(inferrer),
            None if ablations is None else tuple(ablations),
        )


def _under(base: Path, value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else base / path


class Harness:
    """The Evaluation_Harness facade over ``bench.toml`` (see the design's Harness interface).

    :meth:`systems` builds the Baseline_Adapters: the Inferrer first, then
    Naive_Merge, JSONoid when configured, and the ``[baselines.command.*]``
    systems. :meth:`check_leakage` runs before any system runs (30.7, 30.8,
    30.11). :meth:`synthetic`, :meth:`real`, and :meth:`ablation` run the
    benchmarks sequentially (34.3) and write the results files (35.8).
    """

    def __init__(self, cfg: BenchConfig, *, env: dict[str, str] | None = None) -> None:
        from jsinfer.bench.synthetic import SyntheticConfig
        from jsinfer.bench.real import RealConfig

        self.cfg = cfg
        self.settings = HarnessSettings.from_config(cfg)
        bench = cfg.extra.get("bench", {})
        self.synthetic_config = SyntheticConfig.from_mapping(bench, cfg.seed)
        self.real_config = RealConfig.from_mapping(bench, cfg.seed)
        self.env = env
        self._manifest = None
        self._split = None

    # -- inputs --------------------------------------------------------------

    @property
    def manifest(self):
        if self._manifest is None:
            self._manifest = CorpusManifest.load(self.cfg.manifest)
        return self._manifest

    @property
    def split(self):
        if self._split is None:
            self._split = SchemaSplit.load(self.cfg.split_out)
        return self._split

    def build_split(self):
        """The Schema_Split of the configured corpus (task 25; ``jsinfer split`` writes it)."""
        from jsinfer.bench.splits import SplitBuilder, SplitInputs

        cfg = self.cfg
        inputs = SplitInputs.from_manifest(self.manifest, known_ids=cfg.known_schemas, known_files=cfg.known_schema_files, ref_depth=int(cfg.resource("nearest.ref_depth")), max_paths=int(cfg.resource("nearest.max_paths")))
        builder = SplitBuilder(perm=int(cfg.resource("minhash.perm")), bands=int(cfg.resource("lsh.bands")))
        proportions = {name: cfg.resource(name) for name in ("split.train", "split.val", "split.held_out")}
        self._split = builder.build(inputs, split_id=cfg.split_id, seed=cfg.seed, proportions=proportions, max_cutoff=float(cfg.resource("split.max_cutoff")))
        return self._split

    def systems(self) -> list[BaselineAdapter]:
        from jsinfer.bench.baselines.inferrer import InferrerAdapter
        from jsinfer.bench.baselines.jsonoid import jsonoid_from_config
        from jsinfer.bench.baselines.naive import NaiveMergeAdapter

        settings = {"model_bundle": self.settings.model_bundle, **self.settings.inferrer_settings}
        out: list[BaselineAdapter] = [InferrerAdapter(settings=settings)]
        if self.settings.naive_merge:
            out.append(NaiveMergeAdapter())
        if self.settings.jsonoid is not None:
            out.append(jsonoid_from_config(self.settings.jsonoid, Path(self.cfg.manifest).parent.parent))
        for name, table in sorted(self.settings.commands.items(), key=lambda i: codepoint_key(i[0])):
            out.append(CommandAdapter(name, str(table.get("label", name)), tuple(str(a) for a in table["argv"]), str(table.get("output", "schema.json"))))
        return out

    def real_datasets(self):
        """The Real_Datasets of the manifest (33.1) and of SchemaStore (33.2), with the definition-level exclusions."""
        from jsinfer.bench.real import load_manifest, schemastore_datasets

        datasets, exclusions = [], []
        if self.settings.real_manifest is not None:
            found, excluded = load_manifest(self.settings.real_manifest)
            datasets.extend(found)
            exclusions.extend(excluded)
        if self.settings.schemastore_datasets:
            found, excluded = schemastore_datasets(self.manifest)
            datasets.extend(found)
            exclusions.extend(excluded)
        return datasets, exclusions

    # -- leakage (30.7, 30.8, 30.11) ----------------------------------------------

    def check_leakage(self, real_collections: Iterable[str] = ()) -> LeakageReport:
        """Compare the Model_Bundle's training manifest with the held-out groups and Real_Datasets; raises :class:`LeakageError`."""
        from jsinfer.config.errors import ConfigError
        from jsinfer.learn.bundle import load_bundle

        try:
            bundle = load_bundle(self.settings.model_bundle)
        except ConfigError as error:
            raise LeakageError(missing_manifest=f"the Model_Bundle {self.settings.model_bundle} cannot be loaded: {error}") from None
        return check_leakage(bundle.manifest, self.split, self.manifest.ids(), list(real_collections))

    # -- benchmarks ----------------------------------------------------------------

    def _labels(self, systems: Sequence[BaselineAdapter]) -> tuple[dict[str, str], list[str]]:
        labels = {s.name: s.label for s in systems}
        unavailable = [s.name for s in systems if not s.available().available]
        return labels, unavailable

    def synthetic(self, out_dir: Path | None = None):
        """Run the synthetic benchmark (31) and write its results files; returns ``(results, aggregates)``."""
        from jsinfer.bench.reporting import synthetic_rows, write_report
        from jsinfer.bench.synthetic import run_synthetic

        self.check_leakage()
        out = Path(out_dir) if out_dir is not None else Path(self.settings.results) / "synthetic"
        systems = self.systems()
        results = run_synthetic(self.manifest, self.split, systems, self.synthetic_config, out / "runs", env=self.env)
        labels, unavailable = self._labels(systems)
        rows = synthetic_rows(results, labels, unavailable=unavailable)
        agg = write_report(rows, out, seed=self.cfg.seed, resamples=self.settings.resamples, extra={"synthetic": results.to_json()})
        return results, agg

    def real(self, out_dir: Path | None = None):
        """Run the real-data benchmark (33) and write its results files; returns ``(results, aggregates)``."""
        from jsinfer.bench.real import run_real
        from jsinfer.bench.reporting import real_rows, write_report

        datasets, exclusions = self.real_datasets()
        self.check_leakage([d.id for d in datasets])
        out = Path(out_dir) if out_dir is not None else Path(self.settings.results) / "real"
        systems = self.systems()
        results = run_real(datasets, systems, self.real_config, out / "runs", exclusions=exclusions, env=self.env)
        labels, unavailable = self._labels(systems)
        rows = real_rows(results, labels, unavailable=unavailable)
        agg = write_report(rows, out, real=results, seed=self.cfg.seed, resamples=self.settings.resamples, extra={"real": results.to_json()})
        return results, agg

    def ablation(self, out_dir: Path | None = None):
        """Run the ablation study (35.3) and write its results files; returns ``(results, aggregates)``."""
        from jsinfer.bench.baselines.inferrer import InferrerAdapter
        from jsinfer.bench.reporting import synthetic_rows, write_report
        from jsinfer.bench.studies import DEFAULT_SYSTEM, registry_configurations, run_ablation

        self.check_leakage()
        out = Path(out_dir) if out_dir is not None else Path(self.settings.results) / "ablation"
        inferrer = InferrerAdapter(settings={"model_bundle": self.settings.model_bundle, **self.settings.inferrer_settings})
        configs = registry_configurations()
        if self.settings.ablations is not None:
            known = {c.technique for c in configs}
            unknown = [t for t in self.settings.ablations if t not in known]
            if unknown:
                raise ValueError(f"[bench] ablations names Techniques that are not enabled by default: {unknown}")
            configs = [c for c in configs if c.technique in self.settings.ablations]
        results = run_ablation(self.manifest, self.split, inferrer, self.synthetic_config, out / "runs", configs=configs, resamples=self.settings.resamples, env=self.env)
        labels = {DEFAULT_SYSTEM: "Inferrer", **{c.system: f"Inferrer without {c.technique}" for c in results.configs}}
        rows = synthetic_rows(results.results, labels)
        agg = write_report(rows, out, reference=DEFAULT_SYSTEM, seed=self.cfg.seed, resamples=self.settings.resamples, extra={"ablation": results.to_json()})
        return results, agg


__all__ += ["Harness", "HarnessSettings"]
