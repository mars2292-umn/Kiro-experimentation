"""Integration tests for the ``jsinfer`` command-line interface (task 18.4).

Requirements 26.9 and 26.12 (``explain``), 27.1 (no prompts, no reads of
standard input, existing outputs replaced), 27.9 (the API gives what the CLI
writes), and 28.10 (``list-techniques`` reads no data), with the
``list-thresholds`` listing (20.2) and an end-to-end ``infer`` run that
supplies only a Document_Collection (27.2, 27.7).

``tests/unit/test_cli.py`` calls :func:`jsinfer.cli.main` in process and
covers argument parsing, settings, and every error path one by one. These
tests run the installed console script (``python -m jsinfer.cli`` when it
is not installed) as a real process on a multi-file Document_Collection,
with standard input closed, at ``/dev/null``, an open pipe, or a terminal,
and compare what it writes with the public API.

API/CLI equality (27.9; design, Public API and CLI)
    ``schema.json``, ``explanation.json``, and ``effective-config.toml``
    are byte-identical to ``InferenceResult.files()`` of ``infer_files`` on
    the same inputs and of ``infer`` on the same documents in the same
    order, and the loaded schema equals the API's Schema_Model.
    ``report.json`` holds per-stage runtimes, so it is compared after
    removing what 27.12 exempts (``InferenceReport.comparable_json``), and
    only with ``infer_files``: an in-memory run names its input
    ``<documents>``.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest

import jsinfer
from jsinfer import cli
from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE, RunConfiguration, read_config_file
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core.report import InferenceReport
from jsinfer.io.validator import Draft, Validator
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE, load_bundle
from jsinfer.pipeline.infer import INGESTOR_STAGE

OUTPUTS = ["effective-config.toml", "explanation.json", "report.json", "schema.json"]
# The outputs that 27.1 and 27.9 require to be the same bytes in every run of the same configuration.
REPRODUCIBLE = ["schema.json", "explanation.json", "effective-config.toml"]
WROTE = "jsinfer: wrote schema.json, explanation.json, report.json, effective-config.toml to "

# Seconds one CLI process may take. Generous, so a slow machine is not a failure; a CLI that
# waits on standard input never finishes and is still caught.
TIMEOUT = 300


def _command() -> list[str]:
    """The console script installed next to this interpreter (pyproject.toml), else the module."""
    script = Path(sys.executable).with_name("jsinfer")
    return [str(script)] if script.exists() else [sys.executable, "-m", "jsinfer.cli"]


JSINFER = _command()


def _env(hash_seed: str = "0") -> dict[str, str]:
    # A fixed hash seed per process (the test process has its own), and UTF-8 standard streams,
    # so the listings (which hold non-ASCII titles) compare with the in-process text exactly.
    return {**os.environ, "PYTHONHASHSEED": hash_seed, "PYTHONIOENCODING": "utf-8"}


def run_jsinfer(*args: str, cwd: Path | None = None, hash_seed: str = "0") -> subprocess.CompletedProcess[str]:
    """Run ``jsinfer ARGS`` with standard input at ``/dev/null`` and capture its streams."""
    return subprocess.run(
        [*JSINFER, *args],
        cwd=cwd,
        env=_env(hash_seed),
        stdin=subprocess.DEVNULL,
        capture_output=True,
        encoding="utf-8",
        timeout=TIMEOUT,
        check=False,
    )


# ---------------------------------------------------------------------------
# The Document_Collection
# ---------------------------------------------------------------------------


def _document(i: int) -> dict[str, Any]:
    return {
        "id": i,
        "kind": ("order", "refund", "quote")[i % 3],
        "amount": round(i * 2.25, 2),
        "customer": {"name": f"c{i % 7}", "vip": i % 4 == 0},
        "items": [{"sku": f"s{j}", "qty": j + 1} for j in range(i % 3)],
        "unit/size": "kg",  # a name that its JSON Pointers escape (/properties/unit~1size)
        **({"note": f"note {i}"} if i % 2 else {}),
    }


DOCUMENTS: list[dict[str, Any]] = [_document(i) for i in range(36)]


@dataclass(frozen=True)
class Collection:
    root: Path  # a directory: the Ingestor reads its files in code-point order of their names
    documents: list[dict[str, Any]]  # every document, in the order the Ingestor reads them


def _lines(documents: Sequence[Any]) -> str:
    return "".join(json.dumps(document) + "\n" for document in documents)


@pytest.fixture(scope="module")
def collection(tmp_path_factory: pytest.TempPathFactory) -> Collection:
    """One directory with a .json file, a .jsonl file, and a .ndjson file in a subdirectory."""
    root = tmp_path_factory.mktemp("collection")
    (root / "1-single.json").write_text(json.dumps(DOCUMENTS[0]), encoding="utf-8")
    (root / "2-lines.jsonl").write_text(_lines(DOCUMENTS[1:20]), encoding="utf-8")
    (root / "3-more").mkdir()
    (root / "3-more" / "lines.ndjson").write_text(_lines(DOCUMENTS[20:]), encoding="utf-8")
    (root / "README.txt").write_text("not a document", encoding="utf-8")  # other extensions are ignored
    return Collection(root, DOCUMENTS)


Run = tuple[subprocess.CompletedProcess[str], Path]  # a finished CLI process and its output directory


@pytest.fixture(scope="module")
def default_run(collection: Collection, tmp_path_factory: pytest.TempPathFactory) -> Run:
    """``jsinfer infer DIR`` and nothing else, in an empty working directory (27.2)."""
    out = tmp_path_factory.mktemp("default-run")
    return run_jsinfer("infer", str(collection.root), cwd=out), out


def read_json(path: Path) -> Any:
    return json.loads(path.read_bytes())


def comparable_report(path: Path) -> dict[str, Any]:
    return InferenceReport.from_json(read_json(path)).comparable_json()


# ---------------------------------------------------------------------------
# infer: end to end (27.1, 27.2, 27.7)
# ---------------------------------------------------------------------------


def test_infer_with_only_a_document_collection(
    default_run: Run, collection: Collection
) -> None:
    result, out = default_run
    assert (result.returncode, result.stdout) == (cli.EXIT_OK, ""), result.stderr
    assert result.stderr == f"{WROTE}'.'\n"
    assert sorted(os.listdir(out)) == OUTPUTS  # the four outputs, and no temp directory left behind

    # The schema is a draft 2020-12 schema that accepts every document of the collection.
    schema = read_json(out / "schema.json")
    validator = Validator()
    assert validator.meta_errors(schema, Draft.DRAFT202012) == []
    compiled = validator.compile(schema)
    assert all(validator.is_valid(compiled, document) for document in collection.documents)
    assert set(schema["properties"]) == {"id", "kind", "amount", "customer", "items", "unit/size", "note"}

    # Every setting took its default, and the configuration file reads back as that configuration.
    written = RunConfiguration.from_toml((out / "effective-config.toml").read_text(encoding="utf-8"))
    assert written == RunConfiguration.default()

    report = InferenceReport.from_json(read_json(out / "report.json"))
    assert report.ok
    data = report.to_json()
    assert data["ingestion"] == {"errors": [], "skipped": 0}
    assert INGESTOR_STAGE in data["timings"]

    # Each key of the explanation file is a Constraint of the schema (26.7).
    explanation = jsinfer.load_explanation((out / "explanation.json").read_bytes())
    assert explanation.records and all(isinstance(key, str) for key in explanation.records)


# ---------------------------------------------------------------------------
# API/CLI equality (27.9)
# ---------------------------------------------------------------------------

CONFIG_FILE_TEXT = '[run]\nseed = 7\n[techniques]\n"values.format" = false\n'

# (CLI options after INPUT, the same settings as API flags); "-c FILE" stands for CONFIG_FILE_TEXT.
CONFIGURATIONS: dict[str, tuple[list[str], dict[str, Any]]] = {
    "defaults": (["--set", "workers=1"], {"workers": 1}),
    "file and flags": (
        [
            "-c", "FILE", "--technique", "decide.mdl=off", "--technique", "structure.hybrid_record_map=off",
            "--set", "output.inline_provenance=true", "--set", "workers=1",
        ],  # the hybrid partition needs decide.mdl (28.8), so both go off
        {
            "techniques.decide.mdl": False, "techniques.structure.hybrid_record_map": False,
            "output.inline_provenance": True, "workers": 1,
        },
    ),
}


@pytest.mark.parametrize("name", list(CONFIGURATIONS))
def test_the_api_gives_what_the_cli_writes(collection: Collection, tmp_path: Path, name: str) -> None:
    options, flags = CONFIGURATIONS[name]
    config_file = tmp_path / "run.toml"
    config_file.write_text(CONFIG_FILE_TEXT, encoding="utf-8")
    file_settings = read_config_file(config_file) if "FILE" in options else None
    options = [str(config_file) if option == "FILE" else option for option in options]
    out = tmp_path / "out"

    result = run_jsinfer("infer", str(collection.root), *options, "--out", str(out), hash_seed="12345")
    assert result.returncode == cli.EXIT_OK, result.stderr
    written = {name: (out / name).read_bytes() for name in OUTPUTS}

    config = RunConfiguration.resolve(flags, file_settings)
    from_files = jsinfer.infer_files([collection.root], config)
    from_documents = jsinfer.infer(collection.documents, config)
    assert RunConfiguration.from_toml(written["effective-config.toml"].decode("utf-8")) == config
    for api in (from_files, from_documents):
        files = api.files()
        for output in REPRODUCIBLE:
            assert written[output] == files[output], output
        assert jsinfer.load_schema(written["schema.json"]) == api.schema  # the Schema_Model of 27.9
    assert comparable_report(out / "report.json") == from_files.report.comparable_json()

    inline = b'"x-provenance"' in written["schema.json"]
    assert inline is config.output.inline_provenance
    if file_settings is not None:  # the file's settings reached the run, under the flags
        assert (config.seed, config.techniques["values.format"], config.techniques["decide.mdl"]) == (7, False, False)


# ---------------------------------------------------------------------------
# Standard input (27.1)
# ---------------------------------------------------------------------------

STDIN_MODES = ["terminal", "closed", "devnull", "open pipe"]


@dataclass(frozen=True)
class Finished:
    returncode: int
    stdout: str
    stderr: str


def launch(argv: Sequence[str], stdin: str, cwd: Path, logs: Path) -> Finished:
    """Run ``argv`` with standard input ``stdin`` (a STDIN_MODES entry) and wait for it to finish.

    ``terminal`` is the slave side of a new pseudo-terminal, ``closed`` is no
    descriptor 0 at all (``<&-``), and ``open pipe`` is a pipe that stays
    open, never written, until the process ends: a process that read
    standard input would wait on it forever. Output goes to files, so
    nothing here reads or closes the process's standard input early.
    """
    logs.mkdir(parents=True, exist_ok=True)
    command = list(argv)
    keep_open: list[int] = []
    pipe = None
    if stdin == "terminal":
        import pty

        master, slave = pty.openpty()
        keep_open.append(master)
        source: Any = slave
    elif stdin == "closed":
        command = ["/bin/sh", "-c", 'exec "$@" <&-', "sh", *command]
        source = subprocess.DEVNULL
    elif stdin == "devnull":
        source = subprocess.DEVNULL
    else:
        source = subprocess.PIPE
    with (
        open(logs / "stdout", "w+", encoding="utf-8") as stdout,
        open(logs / "stderr", "w+", encoding="utf-8") as stderr,
    ):
        process = subprocess.Popen(command, cwd=cwd, env=_env(), stdin=source, stdout=stdout, stderr=stderr)
        pipe = process.stdin
        if stdin == "terminal":
            os.close(source)  # the child holds the slave side; the master stays open until the end
        try:
            returncode = process.wait(timeout=TIMEOUT)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            pytest.fail(f"jsinfer did not finish with standard input {stdin!r}: is it waiting on input?")
        finally:
            if pipe is not None:
                pipe.close()
            for fd in keep_open:
                os.close(fd)
        stdout.seek(0)
        stderr.seek(0)
        return Finished(returncode, stdout.read(), stderr.read())


_PROBE = "import sys; print(sys.stdin is None, sys.stdin is not None and sys.stdin.isatty())"
_PROBED = {"terminal": "False True", "closed": "True False", "devnull": "False False", "open pipe": "False False"}


@pytest.fixture(params=STDIN_MODES)
def stdin_mode(request: pytest.FixtureRequest, tmp_path: Path) -> str:
    """A STDIN_MODES entry, after checking that a process launched with it sees that standard input."""
    mode: str = request.param
    if os.name != "posix":
        pytest.skip("closed descriptors and pseudo-terminals are POSIX features")
    probed = launch([sys.executable, "-c", _PROBE], mode, tmp_path, tmp_path / "probe")
    assert (probed.returncode, probed.stdout.strip()) == (0, _PROBED[mode]), probed.stderr
    return mode


@pytest.fixture(scope="module")
def terminal_run(collection: Collection, tmp_path_factory: pytest.TempPathFactory) -> Path:
    """The outputs of ``jsinfer infer DIR`` started from a terminal, the reference of 27.1."""
    if os.name != "posix":
        pytest.skip("pseudo-terminals are a POSIX feature")
    out = tmp_path_factory.mktemp("terminal-run")
    finished = launch([*JSINFER, "infer", str(collection.root)], "terminal", out, out.parent / f"{out.name}-logs")
    assert finished.returncode == cli.EXIT_OK, finished.stderr
    return out


def test_runs_do_not_depend_on_standard_input(
    collection: Collection, terminal_run: Path, tmp_path: Path, stdin_mode: str
) -> None:
    # The working directory holds the outputs of an earlier run and a file of the user's. With a
    # terminal on standard input, a prompt before replacing them would wait on it.
    out = tmp_path / "out"
    out.mkdir()
    for name in OUTPUTS:
        (out / name).write_text("from an earlier run", encoding="utf-8")
    (out / "notes.txt").write_text("mine", encoding="utf-8")

    # The default worker count: the workers are processes, which inherit standard input.
    finished = launch([*JSINFER, "infer", str(collection.root)], stdin_mode, out, tmp_path / "logs")
    assert (finished.returncode, finished.stdout) == (cli.EXIT_OK, ""), finished.stderr
    assert finished.stderr == f"{WROTE}'.'\n"
    assert sorted(os.listdir(out)) == sorted([*OUTPUTS, "notes.txt"])
    assert (out / "notes.txt").read_text(encoding="utf-8") == "mine"
    for name in REPRODUCIBLE:
        assert (out / name).read_bytes() == (terminal_run / name).read_bytes(), name
    assert comparable_report(out / "report.json") == comparable_report(terminal_run / "report.json")


# ---------------------------------------------------------------------------
# Failed runs across a real process (1.8, 27.10, 27.11)
# ---------------------------------------------------------------------------


def test_failed_runs_exit_with_their_status_and_keep_earlier_outputs(
    default_run: Run, tmp_path: Path
) -> None:
    _, earlier = default_run
    out = tmp_path / "out"
    shutil.copytree(earlier, out)
    bad = tmp_path / "bad.jsonl"
    bad.write_text("{bad\n[1,]\n", encoding="utf-8")

    # No valid documents (1.8): exit 3, and only report.json is replaced, by the partial report.
    result = run_jsinfer("infer", str(bad), "--out", str(out))
    assert (result.returncode, result.stdout) == (cli.EXIT_NO_DOCUMENTS, "")
    assert result.stderr.startswith("jsinfer: error: no valid documents were found; skipped 2 documents or files")
    assert sorted(os.listdir(out)) == OUTPUTS
    for name in REPRODUCIBLE:
        assert (out / name).read_bytes() == (earlier / name).read_bytes(), name
    report = read_json(out / "report.json")
    assert report["status"]["state"] == "failed" and report["status"]["stage"] == INGESTOR_STAGE

    # A configuration error (27.11): exit 2, before any document is read, and nothing is created.
    fresh = tmp_path / "fresh"
    result = run_jsinfer("infer", str(bad), "--set", "workers=0", "--out", str(fresh))
    assert (result.returncode, result.stdout) == (cli.EXIT_USAGE, "")
    assert result.stderr.startswith("jsinfer: error: ") and "workers" in result.stderr
    assert not fresh.exists()


# ---------------------------------------------------------------------------
# explain (26.9, 26.12)
# ---------------------------------------------------------------------------


def _number(value: int | float) -> str:
    return str(value) if isinstance(value, int) else repr(value)


def _record_with(records: dict[str, Any], member: str) -> str:
    return next(pointer for pointer, record in records.items() if member in record)


def test_explain_prints_every_field_of_a_record(default_run: Run) -> None:
    _, out = default_run
    schema, explanation = out / "schema.json", out / "explanation.json"
    records: dict[str, Any] = read_json(explanation)["records"]
    pointers = ["/properties/unit~1size/type", _record_with(records, "resolution"), _record_with(records, "fallback")]
    for pointer in pointers:
        result = run_jsinfer("explain", str(schema), str(explanation), pointer)
        assert (result.returncode, result.stderr) == (cli.EXIT_OK, ""), pointer
        assert result.stdout == jsinfer.explain(schema, explanation, pointer)
        lines = result.stdout.splitlines()
        record = records[pointer]
        assert lines[:2] == [f"constraint: {pointer}", f"technique: {record['technique']}"]
        support = [f"  {name}: {_number(record['support'][name])}" for name in ("consistent", "total", "ratio")]
        assert lines[2:6] == ["support:", *support]
        assert lines[6] == f"confidence: {_number(record['confidence'])}"
        if "fallback" in record:
            fallback = record["fallback"]
            at = lines.index("fallback:")
            assert lines[at + 1] == f"  technique: {fallback['technique']}"
            assert lines[at + 2] == f"  reason: {fallback['reason']}"
        if "resolution" in record:
            resolution = record["resolution"]
            at = lines.index("resolution:")
            assert lines[at + 1] == f"  method: {resolution['method']}"
            assert lines[at + 2] == f"  selected: {resolution['selected']}"
            for candidate in resolution["candidates"]:
                if "score" in candidate:
                    assert f"      score: {_number(candidate['score'])}" in lines, pointer


def test_explain_accepts_every_key_of_the_explanation_file(
    default_run: Run, capsys: pytest.CaptureFixture[str]
) -> None:
    _, out = default_run
    schema, explanation = str(out / "schema.json"), str(out / "explanation.json")
    records: dict[str, Any] = read_json(out / "explanation.json")["records"]
    for pointer, record in records.items():
        status = cli.main(["explain", schema, explanation, pointer])
        captured = capsys.readouterr()
        assert (status, captured.err) == (cli.EXIT_OK, ""), pointer
        assert captured.out.startswith(f"constraint: {pointer}\ntechnique: {record['technique']}\n"), pointer


# (failed input, failed check) for each way 26.12 lists.
EXPLAIN_FAILURES: dict[str, tuple[str, str]] = {
    "schema missing": ("schema", "read"),
    "schema is a directory": ("schema", "read"),
    "schema is not JSON": ("schema", "parse"),
    "schema is the explanation file": ("schema", "parse"),
    "explanation is the schema file": ("explanation", "parse"),
    "explanation truncated": ("explanation", "parse"),
    "pointer without a leading slash": ("pointer", "syntax"),
    "pointer as a URI fragment": ("pointer", "syntax"),
    "pointer to nothing": ("pointer", "resolve"),
    "pointer to a schema object": ("pointer", "resolve"),
    "pointer without an entry": ("pointer", "entry"),
}


def _explain_arguments(out: Path, tmp_path: Path, case: str) -> tuple[Path, Path, str]:
    schema, explanation, pointer = out / "schema.json", out / "explanation.json", "/properties/kind/type"
    if case == "schema missing":
        schema = tmp_path / "missing.json"
    elif case == "schema is a directory":
        schema = out
    elif case == "schema is not JSON":
        schema = out / "effective-config.toml"
    elif case == "schema is the explanation file":
        schema = explanation
    elif case == "explanation is the schema file":
        explanation = schema
    elif case == "explanation truncated":
        data = explanation.read_bytes()
        explanation = tmp_path / "explanation.json"
        explanation.write_bytes(data[: len(data) // 2])
    elif case == "pointer without a leading slash":
        pointer = "properties/kind/type"
    elif case == "pointer as a URI fragment":
        pointer = "#/properties/kind/type"
    elif case == "pointer to nothing":
        pointer = "/properties/missing/type"
    elif case == "pointer to a schema object":
        pointer = "/properties/customer"
    elif case == "pointer without an entry":
        data = read_json(explanation)
        del data["records"][pointer]
        explanation = tmp_path / "explanation.json"
        explanation.write_text(json.dumps(data), encoding="utf-8")
    return schema, explanation, pointer


@pytest.mark.parametrize("case", list(EXPLAIN_FAILURES))
def test_explain_failures_print_no_record_and_exit_2(
    default_run: Run, tmp_path: Path, case: str
) -> None:
    _, out = default_run
    schema, explanation, pointer = _explain_arguments(out, tmp_path, case)
    with pytest.raises(jsinfer.ExplainError) as caught:
        jsinfer.explain(schema, explanation, pointer)
    error = caught.value
    assert (error.input, error.check) == EXPLAIN_FAILURES[case]

    result = run_jsinfer("explain", str(schema), str(explanation), pointer)
    assert (result.returncode, result.stdout) == (cli.EXIT_USAGE, "")
    assert result.stderr == f"jsinfer: error: {error}\n"
    # The message names the failed input: a file by its path, the pointer as given.
    named = {"schema": repr(str(schema)), "explanation": repr(str(explanation)), "pointer": repr(pointer)}
    assert named[error.input] in result.stderr


# ---------------------------------------------------------------------------
# list-techniques (28.10) and list-thresholds (20.2)
# ---------------------------------------------------------------------------


def test_list_techniques_prints_the_registry(tmp_path: Path) -> None:
    (tmp_path / "data.jsonl").write_text(_lines(DOCUMENTS[:3]), encoding="utf-8")  # there to be ignored
    listings = []
    for hash_seed in ("0", "4242"):
        result = run_jsinfer("list-techniques", cwd=tmp_path, hash_seed=hash_seed)
        assert (result.returncode, result.stderr) == (cli.EXIT_OK, "")
        listings.append(result.stdout)
    assert listings[0] == listings[1] == cli.techniques_text(REGISTRY)
    header, *blocks = listings[0].rstrip("\n").split("\n\n")
    assert header == f"{len(REGISTRY.techniques)} registered Techniques, in registry order:"
    assert [block.split(":", 1)[0] for block in blocks] == list(REGISTRY.ids())
    assert sorted(os.listdir(tmp_path)) == ["data.jsonl"]


# Runs list-techniques under an audit hook that records every path the process opens or lists, and
# prints the ones in the working directory. A hook's exception would reach the audited call, so the
# hook only appends; ``-P`` keeps the working directory off sys.path, so imports do not list it.
_AUDITED = """
import os, sys
seen = []
def hook(event, args):
    if event in ("open", "os.listdir", "os.scandir") and args:
        path = "." if args[0] is None else args[0]
        if isinstance(path, (str, bytes, os.PathLike)):
            seen.append(path)
sys.addaudithook(hook)
from jsinfer.cli import main
status = main(["list-techniques"])
paths = list(seen)
here = os.path.realpath(os.getcwd())
def inside(path):
    real = os.path.realpath(os.fsdecode(path))
    return real == here or real.startswith(here + os.sep)
touched = {os.fsdecode(p) for p in paths if inside(p)}
print(status, sorted(touched), file=sys.stderr)
"""


def test_list_techniques_reads_no_document_collection(tmp_path: Path) -> None:
    # A Document_Collection in the working directory: the command opens and lists nothing there.
    (tmp_path / "data.jsonl").write_text(_lines(DOCUMENTS[:3]), encoding="utf-8")
    result = subprocess.run(
        [sys.executable, "-P", "-c", _AUDITED],
        cwd=tmp_path,
        env=_env(),
        stdin=subprocess.DEVNULL,
        capture_output=True,
        encoding="utf-8",
        timeout=TIMEOUT,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout == cli.techniques_text(REGISTRY)
    assert result.stderr.splitlines()[-1] == "0 []"


def _stub_bundle(root: Path) -> Path:
    """A Model_Bundle with one valid Learned_Threshold, one invalid one, and no models."""
    spec = THRESHOLDS["cooccur.min_support"]
    record = ThresholdRecord.from_spec(spec, value=3, objective_value=0.5, n_examples=4, training_data="cli").to_json()
    root.mkdir()
    manifest = {"format": BUNDLE_FORMAT, "bundle_id": "integration-stub", "artifacts": {}}
    (root / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
    entries = {spec.name: record, "dbscan.eps": {"value": "wide"}}
    (root / THRESHOLDS_FILE).write_text(json.dumps(entries), encoding="utf-8")
    return root


def _blocks(listing: str) -> dict[str, str]:
    return {block.splitlines()[0]: block for block in listing.rstrip("\n").split("\n\n")[1:]}


def test_list_thresholds_prints_the_bundle_values(tmp_path: Path) -> None:
    for hash_seed in ("0", "4242"):
        result = run_jsinfer("list-thresholds", cwd=tmp_path, hash_seed=hash_seed)
        assert (result.returncode, result.stderr) == (cli.EXIT_OK, "")
        assert result.stdout == cli.thresholds_text(load_bundle(DEFAULT_MODEL_BUNDLE))
    blocks = _blocks(result.stdout)
    assert list(blocks) == list(THRESHOLDS)
    missing = "\n  value: none (missing: the Model_Bundle has no record of it)"
    assert all(block.endswith(missing) for block in blocks.values())

    bundle = _stub_bundle(tmp_path / "bundle")
    result = run_jsinfer("list-thresholds", "--bundle", str(bundle), cwd=tmp_path)
    assert (result.returncode, result.stderr) == (cli.EXIT_OK, "")
    assert result.stdout == cli.thresholds_text(load_bundle(str(bundle)))
    assert "(bundle id 'integration-stub') holds a usable value for 1:" in result.stdout.splitlines()[0]
    blocks = _blocks(result.stdout)
    assert list(blocks) == list(THRESHOLDS)
    assert blocks["cooccur.min_support"].endswith("\n  value: 3")
    assert "\n  value: none (invalid: " in blocks["dbscan.eps"]


def test_list_thresholds_with_an_unloadable_bundle_exits_2(tmp_path: Path) -> None:
    result = run_jsinfer("list-thresholds", "--bundle", str(tmp_path / "none"), cwd=tmp_path)
    assert (result.returncode, result.stdout) == (cli.EXIT_USAGE, "")
    expected = f"jsinfer: error: model_bundle: Model_Bundle {str(tmp_path / 'none')!r} cannot be found"
    assert result.stderr.startswith(expected)
