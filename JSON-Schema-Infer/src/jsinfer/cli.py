"""Command-line interface for jsinfer (Requirements 1.8, 1.10, 20.2, 26.9, 26.12, 27.1, 27.3, 27.7, 27.10, 27.11, 28.10).

Commands
    ===========================================================  ==========================================
    ``jsinfer infer INPUT... [-c FILE] [--set KEY=VALUE]...``    a full run; writes the four output files
    ``[--technique ID=on|off]... [--out DIR]``                   (27.1–27.3, 27.7, 27.10)
    ``jsinfer explain SCHEMA EXPLANATION POINTER``               one Provenance_Record as text (26.9, 26.12)
    ``jsinfer list-techniques``                                  the Technique registry (28.10)
    ``jsinfer list-thresholds [--bundle ID]``                    the Learned_Thresholds and their values
                                                                 in a Model_Bundle (20.2)
    ``jsinfer --version``                                        ``jsinfer <version>``
    ===========================================================  ==========================================

    ``jsinfer train -c FILE --out DIR``                          train a Model_Bundle (21.1, 21.8); stage progress on stderr
    ``jsinfer split -c FILE``                                    build the Schema_Split (30.1–30.5)
    ``jsinfer bench {synthetic,real,ablation} -c FILE [--out DIR] [--fresh]``  run a benchmark (30.7, 30.8, 31, 33, 35.3),
                                                                 resuming an interrupted run in DIR unless --fresh (35.9)

    ``bench`` is added by task 30.5. :func:`main` takes the arguments (default ``sys.argv[1:]``)
    and returns the exit status instead of exiting, so tests call it in
    process; the console script ``jsinfer`` (``jsinfer.cli:main``) exits
    with it.

Exit statuses
    ===  ====================================================================
    0    success (``--help`` and ``--version`` included)
    1    an unrecoverable stage error (27.10), or outputs that cannot be
         written
    2    a configuration or usage error: an argparse usage error, any
         :class:`~jsinfer.config.errors.ConfigError` (invalid settings,
         unknown or unregistered names, unmet prerequisites, unimplemented
         Techniques, an unreadable configuration file, an unloadable
         Model_Bundle, a rejected input path (1.10), an unusable ``--out``),
         and every ``explain`` failure (26.12)
    3    no valid documents (1.8)
    ===  ====================================================================

Streams (27.1)
    No command reads standard input or prompts, so a run with standard
    input closed or not attached to a terminal behaves as one started from
    a terminal. When descriptor 0 is closed, :func:`main` first opens
    ``os.devnull`` on it: otherwise the next file or pipe the process opens
    takes number 0, and the joblib worker processes of a run with more than
    one worker inherit it as their standard input and fail at interpreter
    startup. Results (``explain`` text and the listings) go to standard
    output. Errors, warnings, and the ``infer`` summary go to standard
    error, each first line prefixed ``jsinfer:``. Text a stream cannot
    encode is written with backslash escapes rather than failing.

``infer``: settings (27.3, 27.11, 28.8, 28.9)
    Each setting resolves to its flag value, else its ``-c FILE`` value,
    else its default (``RunConfiguration.resolve``). Flags:

    - ``--set KEY=VALUE``: ``KEY`` is a flat setting name, as in
      ``config.run_config`` (``seed``, ``workers``, ``model_bundle``,
      ``input.unwrap_arrays``, ``output.inline_provenance``,
      ``soundness.max_passes``, ``techniques.decide.mdl``). The value of a
      string setting (``model_bundle``, ``input.ordering_key``) is taken
      verbatim. Any other value is read as a TOML value (``4``, ``0.5``,
      ``true``, ``["a", "b"]``) and, when it is not one, kept as the string
      it is, so the type error names it (``--set workers=four``).
    - ``--technique ID=on|off`` is ``--set techniques.ID=true|false``. An
      unregistered ``ID`` is reported as unregistered, with every registered
      id, whatever its value (28.9).
    - A setting given more than once takes its last value, in command-line
      order across ``--set`` and ``--technique``.

    Every problem is reported in one message, before anything is read
    (27.11): malformed flags, then an unreadable configuration file, then
    the problems ``RunConfiguration.resolve`` finds, then an unusable
    ``--out``. When the file cannot be read, the flags are still checked on
    their own, leaving out prerequisite problems, which depend on the
    file's Technique states. An unloadable Model_Bundle and rejected input
    paths (1.10) are found by the pipeline's Model_Bundle and Ingestor
    stages, which raise ``ConfigError`` before any document is read.

``infer``: outputs (27.1, 27.7, 27.10)
    ``--out DIR`` (default: the working directory) is checked first: it must
    be a writable directory, or a path whose nearest existing ancestor is
    one; nothing is created before the run succeeds. The run is
    ``jsinfer.infer_files(INPUT..., config)``, which writes nothing, so the
    CLI and the API give the same bytes for the same documents (27.9). On
    success, ``InferenceResult.files()`` (``schema.json``, with
    ``x-provenance`` when ``output.inline_provenance`` is on;
    ``explanation.json``; ``report.json``; ``effective-config.toml``) is
    written into a temp directory under ``DIR``
    (``core.determinism.atomic_outputs``), and each file is renamed into
    ``DIR``, replacing an existing file of that name without confirmation.
    Other entries of ``DIR`` are kept. If writing fails, the temp directory
    is deleted and the exit status is 1.

    A failed run writes no schema, explanation, or configuration file. When
    a stage fails (exit 1) or no document can be ingested (exit 3), the
    partial Inference_Report (27.10: the failed stage, the error, and every
    completed stage with its runtime; for exit 3 also every skipped
    document or file) is written as ``DIR/report.json`` the same atomic
    way, and standard error names the stage and the error. The files of an
    earlier run in ``DIR`` other than ``report.json`` are left as they are.
    A configuration or usage error (exit 2) writes nothing.

``explain`` (26.9, 26.12)
    ``SCHEMA`` and ``EXPLANATION`` are file paths (passed to
    ``io.explain.explain`` as :class:`pathlib.Path`, since a ``str`` is
    content there). The record's text goes to standard output with exit
    status 0. On an ``ExplainError`` (a file that cannot be read or parsed,
    a malformed pointer, a pointer that is not a Constraint of the schema,
    or one without an entry), its message, which names the failed input and
    the cause, goes to standard error, nothing goes to standard output, and
    the exit status is 2.

``list-techniques`` (28.10)
    Prints every registered Technique in registry order: its id and title,
    area, tier, whether it is implemented, its default state, its
    prerequisites, each served Decision_Point with the Technique's rank in
    that kind's priority order and the kind's Fallback_Behavior, the
    Learned_Thresholds it reads, and the sample-size Resource_Parameter of a
    super-linear Technique. It reads only the registry: no Document_Collection
    and no Model_Bundle, and it does not import the pipeline.

``list-thresholds`` (20.2)
    Prints every registered Learned_Threshold in name order: its name, the
    Techniques and the Decision_Point kind that use it, its admissible
    range, and its value in the Model_Bundle ``--bundle ID`` (default
    ``bundles/default``, the empty shipped bundle), or ``none`` with the
    reason (``missing`` or ``invalid``, 20.6). An unloadable bundle is a
    ``ConfigError`` (exit 2). The library equivalent is
    ``config.thresholds.list_thresholds(load_bundle(ID).threshold_values())``.

Imports
    Only ``infer``, ``explain``, and ``list-thresholds`` import what they
    run, inside their handlers; ``import jsinfer`` itself is light (see its
    module docstring), so ``--version`` and ``list-techniques`` start fast.
"""

from __future__ import annotations

import argparse
import os
import sys
import tomllib
from collections.abc import Callable, Iterable, Mapping, Sequence
from pathlib import Path
from typing import TYPE_CHECKING, Any, Final, TextIO

import jsinfer

if TYPE_CHECKING:
    from jsinfer.config.errors import ConfigError, ConfigProblem
    from jsinfer.config.registry import Registry
    from jsinfer.config.run_config import RunConfiguration
    from jsinfer.core.errors import InferenceError
    from jsinfer.learn.bundle import ModelBundle

__all__ = [
    "EXIT_NO_DOCUMENTS",
    "EXIT_OK",
    "EXIT_STAGE_ERROR",
    "EXIT_USAGE",
    "build_parser",
    "main",
    "techniques_text",
    "thresholds_text",
]

EXIT_OK: Final = 0
EXIT_STAGE_ERROR: Final = 1  # an unrecoverable stage error (27.10), or outputs that cannot be written
EXIT_USAGE: Final = 2  # a configuration or usage error (1.10, 26.12, 27.11, 28.8, 28.9)
EXIT_NO_DOCUMENTS: Final = 3  # no valid documents (1.8)

PROG: Final = "jsinfer"
SET_OPTION: Final = "--set"
TECHNIQUE_OPTION: Final = "--technique"

# The --technique values and the Technique states they select.
_SWITCHES: Final[Mapping[str, bool]] = {"on": True, "off": False}

# Settings whose --set value is taken verbatim (config.run_config Setting kinds).
_VERBATIM_KINDS: Final = ("text", "path")

# How many skipped documents or files a no-documents error lists on standard error.
_SKIPPED_SHOWN: Final = 10

# The descriptor of standard input.
_STDIN_FD: Final = 0


# ---------------------------------------------------------------------------
# Streams
# ---------------------------------------------------------------------------


def _write(stream: TextIO, text: str) -> None:
    """Write ``text``; characters the stream cannot encode become backslash escapes."""
    try:
        stream.write(text)
    except UnicodeEncodeError:
        encoding = getattr(stream, "encoding", None) or "ascii"
        stream.write(text.encode(encoding, "backslashreplace").decode(encoding))
    stream.flush()


def _say(message: str) -> None:
    """A diagnostic line (or block) on standard error, its first line prefixed ``jsinfer:``."""
    _write(sys.stderr, f"{PROG}: {message}\n")


def _error(message: str) -> None:
    _say(f"error: {message}")


def _quote(path: str | os.PathLike[str]) -> str:
    return repr(os.fspath(path))


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


def _set_flag(text: str) -> tuple[str, str]:
    return (SET_OPTION, text)


def _technique_flag(text: str) -> tuple[str, str]:
    return (TECHNIQUE_OPTION, text)


def build_parser() -> argparse.ArgumentParser:
    """Build the top-level argument parser and its subcommands."""
    parser = argparse.ArgumentParser(
        prog=PROG,
        description="Infer a draft 2020-12 JSON Schema from a collection of JSON documents.",
        allow_abbrev=False,
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {jsinfer.__version__}")
    commands = parser.add_subparsers(dest="command", metavar="COMMAND")

    infer = commands.add_parser(
        "infer",
        help="infer a schema from JSON documents",
        description=(
            "Infer a schema from .json, .jsonl, and .ndjson files and directories, and write schema.json, "
            "explanation.json, report.json, and effective-config.toml to the output directory. Settings "
            "resolve to their flag value, else their configuration-file value, else their default."
        ),
        allow_abbrev=False,
    )
    infer.add_argument("inputs", nargs="+", metavar="INPUT", help="a .json, .jsonl, or .ndjson file, or a directory")
    infer.add_argument("-c", "--config", metavar="FILE", help="a TOML configuration file (as effective-config.toml)")
    infer.add_argument(
        SET_OPTION,
        dest="overrides",
        action="append",
        type=_set_flag,
        metavar="KEY=VALUE",
        help="set one setting, e.g. workers=4 or output.inline_provenance=true (repeatable)",
    )
    infer.add_argument(
        TECHNIQUE_OPTION,
        dest="overrides",
        action="append",
        type=_technique_flag,
        metavar="ID=on|off",
        help="enable or disable one Technique, e.g. decide.mdl=off (repeatable)",
    )
    infer.add_argument(
        "--out", default=".", metavar="DIR", help="the output directory (default: the working directory)"
    )
    infer.set_defaults(handler=_infer)

    explain = commands.add_parser(
        "explain",
        help="print the Provenance_Record of one Constraint",
        description="Print every field of the Provenance_Record of the Constraint at POINTER.",
        allow_abbrev=False,
    )
    explain.add_argument("schema", metavar="SCHEMA", help="the schema file (schema.json)")
    explain.add_argument("explanation", metavar="EXPLANATION", help="its explanation file (explanation.json)")
    explain.add_argument("pointer", metavar="POINTER", help="the JSON Pointer of a Constraint, e.g. /properties/id/type")
    explain.set_defaults(handler=_explain)

    techniques = commands.add_parser(
        "list-techniques",
        help="list the registered Techniques",
        description="List every registered Technique with its default state, Decision_Points, and fallbacks.",
        allow_abbrev=False,
    )
    techniques.set_defaults(handler=_list_techniques)

    thresholds = commands.add_parser(
        "list-thresholds",
        help="list the Learned_Thresholds and their values in a Model_Bundle",
        description="List every registered Learned_Threshold and its value in a Model_Bundle.",
        allow_abbrev=False,
    )
    thresholds.add_argument(
        "--bundle", metavar="ID", help="a Model_Bundle identifier or path (default: bundles/default)"
    )
    thresholds.set_defaults(handler=_list_thresholds)

    train = commands.add_parser(
        "train",
        help="train a Model_Bundle from a schema corpus",
        description=(
            "Generate labeled collections from the training and validation schemas of a Schema_Split, train and "
            "calibrate the Decision_Models, fit the Learned_Thresholds, and write the Model_Bundle to DIR."
        ),
        allow_abbrev=False,
    )
    train.add_argument("-c", "--config", required=True, metavar="FILE", help="the TOML training configuration")
    train.add_argument("--out", required=True, metavar="DIR", help="the Model_Bundle directory to write")
    train.set_defaults(handler=_train)

    split = commands.add_parser(
        "split",
        help="build the Schema_Split of a schema corpus",
        description=(
            "Load the configured SchemaStore and JSONSchemaBench checkouts into the corpus manifest (when "
            "configured), group near-duplicate schemas, assign the groups to the training, validation, and held-out "
            "splits, and write the Schema_Split file."
        ),
        allow_abbrev=False,
    )
    split.add_argument("-c", "--config", required=True, metavar="FILE", help="the TOML benchmark configuration")
    split.set_defaults(handler=_split)

    bench = commands.add_parser(
        "bench",
        help="run a benchmark",
        description=(
            "Check the Model_Bundle's training manifest against the held-out split and the Real_Datasets, then run "
            "the synthetic benchmark, the real-data benchmark, or the ablation study, and write the results files "
            "(CSV, JSON, Markdown, and LaTeX)."
        ),
        allow_abbrev=False,
    )
    bench.add_argument("study", choices=("synthetic", "real", "ablation"), help="the benchmark to run")
    bench.add_argument("-c", "--config", required=True, metavar="FILE", help="the TOML benchmark configuration")
    bench.add_argument("--out", metavar="DIR", help="the results directory (default: [bench] results/<study>)")
    bench.add_argument(
        "--fresh",
        action="store_true",
        help="discard the stored results of an earlier run in DIR instead of resuming it (35.9)",
    )
    bench.set_defaults(handler=_bench)
    return parser


def _exit_status(code: Any) -> int:
    """The status of an argparse exit: ``--help`` and ``--version`` give 0, usage errors 2."""
    if code is None:
        return EXIT_OK
    return code if isinstance(code, int) else EXIT_USAGE


def _reserve_stdin() -> None:
    """Open ``os.devnull`` on descriptor 0 when standard input is closed (see Streams in the module docstring)."""
    try:
        os.fstat(_STDIN_FD)
    except OSError:
        null = os.open(os.devnull, os.O_RDONLY)  # the lowest free descriptor, so 0
        if null != _STDIN_FD:
            os.dup2(null, _STDIN_FD)
            os.close(null)
        os.set_inheritable(_STDIN_FD, True)  # as standard input is; os.open gives a non-inheritable descriptor


def main(argv: Sequence[str] | None = None) -> int:
    """Run the CLI on ``argv`` (default ``sys.argv[1:]``) and return the process exit status."""
    _reserve_stdin()
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as stop:  # argparse exits for --help, --version, and usage errors
        return _exit_status(stop.code)
    handler: Callable[[argparse.Namespace], int] | None = getattr(args, "handler", None)
    if handler is None:
        parser.print_help(sys.stderr)
        return EXIT_USAGE
    return handler(args)


# ---------------------------------------------------------------------------
# infer: settings
# ---------------------------------------------------------------------------


def _setting_value(name: str, raw: str) -> Any:
    """The value of ``--set name=raw``: verbatim for string settings, else TOML (see the module docstring)."""
    from jsinfer.config.run_config import SETTINGS

    setting = SETTINGS.get(name)
    if setting is not None and setting.kind in _VERBATIM_KINDS:
        return raw
    try:
        parsed = tomllib.loads(f"value = {raw}")
    except tomllib.TOMLDecodeError:
        return raw
    return parsed["value"] if list(parsed) == ["value"] else raw


def _flag_values(overrides: Iterable[tuple[str, str]]) -> tuple[dict[str, Any], list[ConfigProblem]]:
    """The flat-name flags of ``--set`` and ``--technique``, last one winning, and the malformed ones."""
    from jsinfer.config.errors import ConfigProblem
    from jsinfer.config.registry import REGISTRY
    from jsinfer.config.run_config import technique_setting

    flags: dict[str, Any] = {}
    problems: list[ConfigProblem] = []
    for option, text in overrides:
        key, separator, raw = text.partition("=")
        if not separator or not key:
            expected = "KEY=VALUE" if option == SET_OPTION else "ID=on or ID=off"
            problems.append(ConfigProblem(text, "parse", f"flag: {option} {text!r}: expected {expected}"))
            continue
        if option == TECHNIQUE_OPTION:
            name = technique_setting(key)
            if raw not in _SWITCHES and key in REGISTRY.techniques:
                message = f"flag: {option} {text!r}: expected on or off after '=', got {raw!r}"
                problems.append(ConfigProblem(name, "type", message))
                continue
            # An unregistered id is passed on whatever its value, so resolve reports it
            # with every registered id (28.9), as for --set techniques.ID=VALUE.
            flags[name] = _SWITCHES.get(raw, raw)
        else:
            flags[key] = _setting_value(key, raw)
    return flags, problems


def _output_problem(out: str) -> ConfigProblem | None:
    """Why ``--out`` cannot receive the outputs, or None; nothing is created (see the module docstring)."""
    from jsinfer.config.errors import ConfigProblem

    path = Path(out)
    probe = path
    while not os.path.lexists(probe) and probe.parent != probe:
        probe = probe.parent
    where = f"--out {_quote(out)}"
    if probe != path:
        where = f"{where} cannot be created: {_quote(probe)}"
    try:
        if not probe.is_dir():
            reason = "is not a directory"
        elif not os.access(probe, os.W_OK | os.X_OK):
            reason = "is not writable"
        else:
            return None
    except OSError as error:
        reason = f"cannot be inspected: {error.strerror or type(error).__name__}"
    return ConfigProblem(out, "path", f"{where} {reason}")


def _resolve_config(config_file: str | None, overrides: Iterable[tuple[str, str]], out: str) -> RunConfiguration:
    """The effective Run_Configuration; one ``ConfigError`` lists every problem (see the module docstring)."""
    from jsinfer.config.errors import ConfigError
    from jsinfer.config.run_config import RunConfiguration, read_config_file

    flags, problems = _flag_values(overrides)
    file: dict[str, Any] | None = None
    file_failed = False
    if config_file is not None:
        try:
            file = read_config_file(config_file)
        except ConfigError as error:
            problems.extend(error.problems)
            file_failed = True
    config: RunConfiguration | None = None
    try:
        config = RunConfiguration.resolve(flags, file)
    except ConfigError as error:
        # Without the file, the effective Technique states are unknown, so a
        # prerequisite problem of the flags alone may not be one.
        problems.extend(p for p in error.problems if not (file_failed and p.reason == "prerequisite"))
    out_problem = _output_problem(out)
    if out_problem is not None:
        problems.append(out_problem)
    if problems or config is None:
        raise ConfigError(problems)
    return config


def _config_failure(error: ConfigError) -> int:
    _error(str(error))
    return EXIT_USAGE


# ---------------------------------------------------------------------------
# infer: the run and its outputs
# ---------------------------------------------------------------------------


def _os_reason(error: OSError) -> str:
    return error.strerror or str(error) or type(error).__name__


def _write_outputs(out: str, files: Mapping[str, bytes]) -> str | None:
    """Write ``files`` into ``out`` through a temp directory (27.10); the reason when that fails, else None."""
    from jsinfer.core.determinism import atomic_outputs, atomic_write_bytes

    try:
        with atomic_outputs(out) as staging:
            for name, data in files.items():
                atomic_write_bytes(staging / name, data)
    except OSError as error:
        return _os_reason(error)
    return None


def _write_partial_report(out: str, error: InferenceError) -> None:
    """Write the partial Inference_Report of a failed run as ``out/report.json`` (27.10)."""
    from jsinfer import REPORT_FILE

    if error.partial_report is None:  # the pipeline always attaches it; nothing to write otherwise
        return
    target = os.path.join(out, REPORT_FILE)
    reason = _write_outputs(out, {REPORT_FILE: error.partial_report.to_bytes()})
    if reason is None:
        _say(f"the partial Inference_Report is in {_quote(target)}; no schema or explanation file was written")
    else:
        _say(f"the partial Inference_Report cannot be written to {_quote(target)}: {reason}")


def _skipped_lines(errors: Sequence[Any]) -> list[str]:
    """Up to ``_SKIPPED_SHOWN`` skipped documents or files (IngestErrors), one per line."""
    lines = []
    for skipped in errors[:_SKIPPED_SHOWN]:
        line = "" if skipped.line is None else f" line {skipped.line}"
        lines.append(f"  {skipped.source}{line}: {skipped.reason}")
    if len(errors) > _SKIPPED_SHOWN:
        lines.append(f"  ... and {len(errors) - _SKIPPED_SHOWN} more")
    return lines


def _infer(args: argparse.Namespace) -> int:
    from jsinfer.config.errors import ConfigError
    from jsinfer.core.errors import InferenceError, NoDocumentsError

    out: str = args.out
    try:
        config = _resolve_config(args.config, args.overrides or (), out)
        result = jsinfer.infer_files(args.inputs, config)
    except ConfigError as error:  # settings, the Model_Bundle, or rejected input paths (1.10, 27.11)
        return _config_failure(error)
    except NoDocumentsError as error:  # 1.8
        _error("\n".join([str(error), *_skipped_lines(error.errors)]))
        _write_partial_report(out, error)
        return EXIT_NO_DOCUMENTS
    except InferenceError as error:  # 27.10
        message = str(error)
        if error.stage not in message:  # the pipeline's messages name the stage; keep that for any other
            message = f"stage {error.stage} failed: {message}"
        _error(message)
        _write_partial_report(out, error)
        return EXIT_STAGE_ERROR

    files = result.files()
    reason = _write_outputs(out, files)
    if reason is not None:
        _error(f"the outputs cannot be written to {_quote(out)}: {reason}")
        return EXIT_STAGE_ERROR
    skipped = result.report.skipped
    if skipped:  # 1.7: the run continued without them
        noun = "document or file" if skipped == 1 else "documents or files"
        _say(f"skipped {skipped} {noun} that could not be parsed or read; report.json lists them")
    _say(f"wrote {', '.join(files)} to {_quote(out)}")
    return EXIT_OK


# ---------------------------------------------------------------------------
# explain
# ---------------------------------------------------------------------------


def _explain(args: argparse.Namespace) -> int:
    from jsinfer.core.provenance import ExplainError
    from jsinfer.io.explain import explain

    try:
        text = explain(Path(args.schema), Path(args.explanation), args.pointer)
    except ExplainError as error:  # 26.12
        _error(str(error))
        return EXIT_USAGE
    _write(sys.stdout, text)
    return EXIT_OK


# ---------------------------------------------------------------------------
# list-techniques
# ---------------------------------------------------------------------------


def techniques_text(registry: Registry) -> str:
    """The ``list-techniques`` listing of ``registry`` (see the module docstring; 28.10)."""
    lines = [f"{len(registry.techniques)} registered Techniques, in registry order:"]
    for tid, spec in registry.techniques.items():
        lines.append("")
        lines.append(f"{tid}: {spec.title}")
        lines.append(f"  area: {spec.area}")
        lines.append(f"  tier: {spec.tier}")
        lines.append(f"  implemented: {'yes' if spec.implemented else 'no'}")
        lines.append(f"  default: {'enabled' if spec.default_enabled else 'disabled'}")
        lines.append(f"  prerequisites: {', '.join(spec.prerequisites) or 'none'}")
        if spec.serves:
            lines.append("  Decision_Points:")
            for kind in spec.serves:
                dp = registry.decision_point(kind)
                lines.append(f"    {kind} ({dp.title}): rank {spec.priority[kind]} of {len(dp.served_by)}")
                gate = dp.serves_only_with.get(tid)
                if gate is not None:
                    lines.append(f"      serves it only while {gate} is enabled")
                lines.append(f"      Fallback_Behavior: {dp.fallback_doc}")
        else:
            lines.append("  Decision_Points: none")
        lines.append(f"  Learned_Thresholds: {', '.join(spec.thresholds) or 'none'}")
        if spec.sample_param is not None:
            lines.append(f"  sample size: {spec.sample_param}")
    return "\n".join(lines) + "\n"


def _list_techniques(args: argparse.Namespace) -> int:
    from jsinfer.config.registry import REGISTRY

    _write(sys.stdout, techniques_text(REGISTRY))
    return EXIT_OK


# ---------------------------------------------------------------------------
# list-thresholds
# ---------------------------------------------------------------------------


def thresholds_text(bundle: ModelBundle) -> str:
    """The ``list-thresholds`` listing of the Learned_Thresholds in ``bundle`` (see the module docstring; 20.2)."""
    from jsinfer.config.intervals import format_number
    from jsinfer.config.thresholds import THRESHOLDS, list_thresholds

    check = bundle.check_thresholds()
    problems = {problem.name: problem for problem in check.problems}
    rows = list_thresholds(check.values)
    lines = [
        f"{len(rows)} registered Learned_Thresholds; Model_Bundle {bundle.identifier!r} "
        f"(bundle id {bundle.bundle_id!r}) holds a usable value for {len(check.records)}:"
    ]
    for row in rows:
        if row.value is not None:
            value = format_number(row.value)
        else:
            problem = problems[row.name]
            value = f"none ({problem.cause}: {problem.detail})"
        lines.append("")
        lines.append(row.name)
        lines.append(f"  Techniques: {', '.join(row.techniques)}")
        lines.append(f"  Decision_Point: {row.decision_point}")
        lines.append(f"  admissible: {THRESHOLDS[row.name].accepted()}")
        lines.append(f"  value: {value}")
    return "\n".join(lines) + "\n"


def _list_thresholds(args: argparse.Namespace) -> int:
    from jsinfer.config.errors import ConfigError
    from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE
    from jsinfer.learn.bundle import load_bundle

    identifier = DEFAULT_MODEL_BUNDLE if args.bundle is None else args.bundle
    try:
        bundle = load_bundle(identifier)
    except ConfigError as error:  # 27.11
        return _config_failure(error)
    _write(sys.stdout, thresholds_text(bundle))
    return EXIT_OK


# ---------------------------------------------------------------------------
# train (21.1, 21.8)
# ---------------------------------------------------------------------------


def _train(args: argparse.Namespace) -> int:
    from jsinfer.config.errors import ConfigError
    from jsinfer.learn.trainer import Trainer, TrainingConfig, TrainingError

    try:
        config = TrainingConfig.from_toml(args.config)
    except ConfigError as error:  # 27.11
        return _config_failure(error)
    try:
        bundle_id = Trainer(progress=lambda line: _say(f"train: {line}")).run(config, args.out)
    except TrainingError as error:  # 21.8: no bundle is left at the output location
        _error(str(error))
        return EXIT_STAGE_ERROR
    _say(f"wrote Model_Bundle {bundle_id} to {_quote(args.out)}")
    return EXIT_OK


# ---------------------------------------------------------------------------
# split (30.1–30.5, 30.10)
# ---------------------------------------------------------------------------


def _split(args: argparse.Namespace) -> int:
    from jsinfer.bench.config import BenchConfig
    from jsinfer.bench.corpus_loader import CorpusSource, load_jsonschemabench, load_schemastore, write_corpus_manifest
    from jsinfer.bench.splits import CorpusManifest, SplitBuilder, SplitError, SplitFormatError, SplitInputs
    from jsinfer.config.errors import ConfigError

    try:
        config = BenchConfig.from_toml(args.config)
    except ConfigError as error:  # 27.11
        return _config_failure(error)
    try:
        if config.has_checkouts:
            sources: list[CorpusSource] = []
            if config.schemastore is not None:
                assert config.schemastore_commit is not None
                sources.append(load_schemastore(config.schemastore, config.schemastore_commit))
            if config.jsonschemabench is not None:
                assert config.jsonschemabench_revision is not None
                sources.append(load_jsonschemabench(config.jsonschemabench, config.jsonschemabench_revision))
            manifest = write_corpus_manifest(sources, config.manifest)
            omitted = len(manifest.extra.get("omitted", []))  # type: ignore[arg-type]
            _say(f"wrote the corpus manifest ({len(manifest.ids())} schemas, {omitted} omitted files) to {_quote(config.manifest)}")
        else:
            manifest = CorpusManifest.load(config.manifest)
        inputs = SplitInputs.from_manifest(
            manifest,
            known_ids=config.known_schemas,
            known_files=config.known_schema_files,
            ref_depth=int(config.resource("nearest.ref_depth")),
            max_paths=int(config.resource("nearest.max_paths")),
        )
        builder = SplitBuilder(perm=int(config.resource("minhash.perm")), bands=int(config.resource("lsh.bands")))
        proportions = {name: config.resource(name) for name in ("split.train", "split.val", "split.held_out")}
        split = builder.build(
            inputs,
            split_id=config.split_id,
            seed=config.seed,
            proportions=proportions,
            max_cutoff=float(config.resource("split.max_cutoff")),
        )
        split.save(config.split_out)
    except (SplitError, SplitFormatError, OSError) as error:  # 30.10: nothing is written
        _error(f"split: {error}")
        return EXIT_STAGE_ERROR
    cutoff: Any = split.extra["cutoff"]
    groups: Any = split.extra["groups"]
    _say(
        f"wrote Schema_Split {split.id} ({len(split.train)} train, {len(split.val)} val, {len(split.held_out)} held out; "
        f"{len(set(groups.values()))} near-duplicate groups; cut-off {cutoff['value']:.3f}, selected {cutoff['selected']:.3f} "
        f"from {cutoff['components']} mixture components over {cutoff['candidate_pairs']} candidate pairs, "
        f"capped at {cutoff['max_cutoff']:.2f}) to {_quote(config.split_out)}"
    )
    return EXIT_OK


# ---------------------------------------------------------------------------
# bench (30.7, 30.8, 30.11, 34.3, 35.8)
# ---------------------------------------------------------------------------


def _bench(args: argparse.Namespace) -> int:
    from jsinfer.bench.config import BenchConfig
    from jsinfer.bench.leakage import LeakageError
    from jsinfer.bench.runner import Harness
    from jsinfer.bench.splits import SplitFormatError
    from jsinfer.config.errors import ConfigError

    try:
        config = BenchConfig.from_toml(args.config)
        harness = Harness(config, fresh=args.fresh)
    except ConfigError as error:  # 27.11
        return _config_failure(error)
    except (ValueError, TypeError) as error:
        _error(f"bench: {error}")
        return EXIT_USAGE
    out = None if args.out is None else Path(args.out)
    try:
        if args.study == "synthetic":
            results, agg = harness.synthetic(out)
            done = f"{len(results.results)} results over {len(results.data)} held-out schemas ({len(results.exclusions)} excluded)"
        elif args.study == "real":
            results, agg = harness.real(out)
            done = f"{len(results.results)} results over {len({r.dataset_id for r in results.results})} Real_Datasets ({len(results.dataset_exclusions)} excluded)"
        else:
            results, agg = harness.ablation(out)
            done = f"{len(results.results.results)} results over {len(results.configs)} ablations"
    except LeakageError as error:  # 30.8, 30.11: nothing ran
        _error(f"bench: aborted before any run: {error}")
        return EXIT_STAGE_ERROR
    except (SplitFormatError, OSError, ValueError) as error:
        _error(f"bench: {error}")
        return EXIT_STAGE_ERROR
    where = out if out is not None else Path(harness.settings.results) / args.study
    unavailable = f"; unavailable: {', '.join(agg.unavailable)}" if agg.unavailable else ""
    store = harness.resume
    resumed = f"; resumed {store.reused} stored results" if store is not None and store.reused else ""
    _say(f"{args.study}: {done}{unavailable}{resumed}; wrote results.csv, results.json, aggregates.csv, aggregates.json, summary.md, summary.tex to {_quote(where)}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
