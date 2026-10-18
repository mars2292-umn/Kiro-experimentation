"""Unit tests for the command-line interface in ``jsinfer/cli.py`` (task 18.2).

Requirements 1.8 (no valid documents: exit 3), 1.10 (rejected input paths:
exit 2 before any read), 20.2 (``list-thresholds``), 26.9 and 26.12
(``explain``), 27.1 (no prompts; existing outputs replaced), 27.3 (flag >
file > default), 27.7 (the four output files), 27.10 (stage errors: exit 1,
a partial report, no schema or explanation file, the temp directory
removed), 27.11 (every configuration problem in one message, exit 2), and
28.10 (``list-techniques`` reads no data).

The tests call :func:`jsinfer.cli.main` in process, which returns the exit
status. Faults are injected into a pipeline stage and into the file writer
the same way ``tests/unit/test_api.py`` injects them. End-to-end subprocess
runs (closed stdin, API/CLI equality across processes) belong to task 18.4;
the subprocess tests here only check the import footprint of the light
commands and that the console script exits with ``main``'s status.
"""

from __future__ import annotations

import errno
import io
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

import pytest

import jsinfer
from jsinfer import cli
from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration
from jsinfer.config.thresholds import THRESHOLDS, ThresholdRecord
from jsinfer.core import determinism
from jsinfer.core.report import InferenceReport
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE, load_bundle
from jsinfer.pipeline.infer import ANALYZER_STAGE, INGESTOR_STAGE
from jsinfer.structure.analyzer import StructureAnalyzer

OUTPUTS = ["effective-config.toml", "explanation.json", "report.json", "schema.json"]
ONE_WORKER = ("--set", "workers=1")
DOCS: list[Any] = [{"id": i, "name": f"n{i}", **({"tag": "x"} if i % 2 else {})} for i in range(8)]


def run(capsys: pytest.CaptureFixture[str], *argv: str) -> tuple[int, str, str]:
    """``main(argv)`` and what it wrote to standard output and standard error."""
    status = cli.main(list(argv))
    captured = capsys.readouterr()
    return status, captured.out, captured.err


def write_jsonl(path: Path, documents: list[Any]) -> Path:
    path.write_text("".join(json.dumps(document) + "\n" for document in documents), encoding="utf-8")
    return path


def read_json(path: Path) -> Any:
    return json.loads(path.read_bytes())


def effective(out: Path) -> dict[str, Any]:
    return tomllib.loads((out / "effective-config.toml").read_text(encoding="utf-8"))


def write_bundle(root: Path, entries: dict[str, Any], bundle_id: str = "cli-stub") -> Path:
    """A Model_Bundle directory holding the raw ``thresholds.json`` ``entries`` and no models."""
    root.mkdir(parents=True)
    manifest = {"format": BUNDLE_FORMAT, "bundle_id": bundle_id, "artifacts": {}}
    (root / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
    (root / THRESHOLDS_FILE).write_text(json.dumps(entries), encoding="utf-8")
    return root


def threshold_record(name: str, value: int | float) -> dict[str, Any]:
    spec = THRESHOLDS[name]
    return ThresholdRecord.from_spec(spec, value=value, objective_value=0.5, n_examples=4, training_data="cli").to_json()


@pytest.fixture
def data(tmp_path: Path) -> Path:
    return write_jsonl(tmp_path / "data.jsonl", DOCS)


# ---------------------------------------------------------------------------
# Parsing and exit statuses
# ---------------------------------------------------------------------------


def test_help_and_version_return_zero(capsys: pytest.CaptureFixture[str]) -> None:
    status, out, _ = run(capsys, "--help")
    assert status == cli.EXIT_OK and "infer" in out and "list-thresholds" in out
    status, out, _ = run(capsys, "infer", "--help")
    assert status == cli.EXIT_OK and "--technique ID=on|off" in out and "--out DIR" in out
    status, out, _ = run(capsys, "--version")
    assert (status, out) == (cli.EXIT_OK, f"jsinfer {jsinfer.__version__}\n")


@pytest.mark.parametrize(
    "argv",
    [
        [],  # no command
        ["bogus"],  # unknown command
        ["infer"],  # no INPUT
        ["infer", "a.jsonl", "--outdir", "x"],  # unknown option (no abbreviations)
        ["explain", "schema.json", "explanation.json"],  # no POINTER
        ["list-techniques", "extra"],
    ],
)
def test_usage_errors_exit_2(capsys: pytest.CaptureFixture[str], argv: list[str]) -> None:
    status, out, err = run(capsys, *argv)
    assert status == cli.EXIT_USAGE and out == "" and "usage: jsinfer" in err


def test_set_and_technique_keep_command_line_order() -> None:
    args = cli.build_parser().parse_args(
        ["infer", "a.jsonl", "--set", "seed=1", "--technique", "decide.mdl=off", "--set", "workers=2", "-c", "f.toml"]
    )
    assert args.inputs == ["a.jsonl"] and args.config == "f.toml" and args.out == "."
    assert args.overrides == [("--set", "seed=1"), ("--technique", "decide.mdl=off"), ("--set", "workers=2")]


# ---------------------------------------------------------------------------
# infer: a successful run and its outputs (27.1, 27.7, 27.10)
# ---------------------------------------------------------------------------


def test_infer_writes_the_api_outputs(capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path) -> None:
    out = tmp_path / "out" / "nested"  # created on success, with its parents
    status, stdout, err = run(capsys, "infer", str(data), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_OK and stdout == ""
    assert err == f"jsinfer: wrote schema.json, explanation.json, report.json, effective-config.toml to {str(out)!r}\n"
    assert sorted(os.listdir(out)) == OUTPUTS  # no temp directory is left behind

    api = jsinfer.infer_files([str(data)], RunConfiguration.resolve({"workers": 1}))
    files = api.files()
    for name in ("schema.json", "explanation.json", "effective-config.toml"):
        assert (out / name).read_bytes() == files[name], name
    written = InferenceReport.from_json(read_json(out / "report.json"))
    assert written.ok and written.comparable_json() == api.report.comparable_json()
    assert b"x-provenance" not in (out / "schema.json").read_bytes()  # output.inline_provenance is off by default


def test_inline_provenance_setting_reaches_the_schema_file(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path
) -> None:
    out = tmp_path / "out"
    status, _, _ = run(capsys, "infer", str(data), *ONE_WORKER, "--set", "output.inline_provenance=true", "--out", str(out))
    assert status == cli.EXIT_OK
    config = RunConfiguration.resolve({"workers": 1, "output.inline_provenance": True})
    assert (out / "schema.json").read_bytes() == jsinfer.infer_files([str(data)], config).schema_bytes
    assert b'"x-provenance"' in (out / "schema.json").read_bytes()
    assert effective(out)["output"]["inline_provenance"] is True


def test_existing_outputs_are_replaced_without_prompting(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    out = tmp_path / "out"
    out.mkdir()
    for name in OUTPUTS:
        (out / name).write_text("stale", encoding="utf-8")
    (out / "keep.txt").write_text("mine", encoding="utf-8")
    monkeypatch.setattr(sys, "stdin", io.StringIO(""))  # nothing to read: a prompt would see end of input
    status, _, _ = run(capsys, "infer", str(data), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_OK
    assert sorted(os.listdir(out)) == sorted([*OUTPUTS, "keep.txt"])
    assert (out / "keep.txt").read_text(encoding="utf-8") == "mine"
    for name in OUTPUTS:
        assert (out / name).read_bytes() != b"stale", name
    assert read_json(out / "schema.json")["$schema"] == "https://json-schema.org/draft/2020-12/schema"


def test_out_defaults_to_the_working_directory(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    work = tmp_path / "work"
    work.mkdir()
    monkeypatch.chdir(work)
    status, _, err = run(capsys, "infer", str(data), *ONE_WORKER)
    assert status == cli.EXIT_OK and err.endswith("to '.'\n")
    assert sorted(os.listdir(work)) == OUTPUTS


def test_skipped_documents_are_summarized(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    data = tmp_path / "data.jsonl"
    data.write_text('{"a": 1}\n{bad\n{"a": 2}\n', encoding="utf-8")
    out = tmp_path / "out"
    status, _, err = run(capsys, "infer", str(data), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_OK
    assert "jsinfer: skipped 1 document or file that could not be parsed or read; report.json lists them\n" in err
    assert read_json(out / "report.json")["ingestion"]["skipped"] == 1


# ---------------------------------------------------------------------------
# infer: settings (27.3, 27.4, 27.11, 28.8, 28.9)
# ---------------------------------------------------------------------------


def test_flags_override_the_file_and_the_file_overrides_defaults(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path
) -> None:
    config_file = tmp_path / "run.toml"
    config_file.write_text(
        '[run]\nseed = 11\nworkers = 3\n[resources]\n"soundness.max_passes" = 3\n'
        '[techniques]\n"decide.mdl" = false\n"values.bounds" = false\n',
        encoding="utf-8",
    )
    out = tmp_path / "out"
    argv = ["infer", str(data), "-c", str(config_file), "--set", "soundness.max_passes=2"]
    argv += ["--technique", "values.bounds=on", *ONE_WORKER, "--out", str(out)]
    status, _, _ = run(capsys, *argv)
    assert status == cli.EXIT_OK

    settings = effective(out)
    assert settings["run"]["seed"] == 11  # file
    assert settings["run"]["workers"] == 1  # flag over file
    assert settings["resources"]["soundness.max_passes"] == 2  # flag over file
    assert settings["resources"]["ingest.chunk_size"] == 2_000  # default
    assert settings["techniques"]["decide.mdl"] is False  # file
    assert settings["techniques"]["values.bounds"] is True  # flag over file
    origins = {name: use["origin"] for name, use in read_json(out / "report.json")["resources"].items()}
    assert {name: origins[name] for name in ("ingest.chunk_size", "soundness.max_passes", "workers")} == {
        "ingest.chunk_size": "default",
        "soundness.max_passes": "flag",
        "workers": "flag",
    }

    # The effective configuration, read back as a configuration file, is the same run (27.4, 27.7).
    again = tmp_path / "again"
    status, _, _ = run(capsys, "infer", str(data), "-c", str(out / "effective-config.toml"), "--out", str(again))
    assert status == cli.EXIT_OK
    for name in ("schema.json", "explanation.json", "effective-config.toml"):
        assert (again / name).read_bytes() == (out / name).read_bytes(), name


def test_set_values_are_toml_except_for_string_settings(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # A bundle directory named like a TOML integer: model_bundle is taken verbatim, not as 2024.
    monkeypatch.chdir(tmp_path)
    write_bundle(tmp_path / "2024", {}, bundle_id="year-bundle")
    out = tmp_path / "out"
    argv = ["infer", str(data), *ONE_WORKER, "--set", "model_bundle=2024", "--set", 'corpus.exclude_ids=["b", "a"]']
    argv += ["--set", "seed=1", "--technique", "decide.mdl=off", "--set", "seed=7", "--set", "techniques.decide.mdl=true"]
    argv += ["--set", "input.ordering_key=$[\"ts\"]", "--out", str(out)]
    status, _, err = run(capsys, *argv)
    assert status == cli.EXIT_OK, err
    settings = effective(out)
    assert settings["run"]["model_bundle"] == "2024"
    assert settings["corpus"]["exclude_ids"] == ["a", "b"]
    assert settings["input"]["ordering_key"] == "$.ts"
    assert settings["run"]["seed"] == 7  # the last value of a setting wins
    assert settings["techniques"]["decide.mdl"] is True  # across --set and --technique
    assert read_json(out / "report.json")["run"]["bundle_id"] == "year-bundle"


def test_every_configuration_problem_is_reported_at_once(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    out = tmp_path / "out"
    argv = ["infer", str(tmp_path / "missing.jsonl"), "--set", "workers=four", "--set", "bogus=1", "--set", "novalue"]
    argv += ["--technique", "nope.x=on", "--technique", "decide.mdl=maybe", "--set", "sample.lof=0", "--out", str(out)]
    status, stdout, err = run(capsys, *argv)
    assert status == cli.EXIT_USAGE and stdout == ""
    lines = err.splitlines()
    assert lines[0] == "jsinfer: error: 6 configuration problems:"
    expected = [
        "flag: --set 'novalue': expected KEY=VALUE",
        "flag: --technique 'decide.mdl=maybe': expected on or off after '=', got 'maybe'",
        "workers: expected",
        "'four'",
        "unknown setting 'bogus'",
        "sample.lof",
        "unregistered Technique id 'nope.x'",
        "variants.minhash_lsh",  # the registered ids are listed (28.9)
    ]
    for fragment in expected:
        assert fragment in err, fragment
    assert "missing.jsonl" not in err  # configuration problems stop the run before the inputs are checked
    assert not out.exists()  # nothing is written, not even the output directory


def test_an_unregistered_technique_id_is_reported_whatever_its_switch(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path
) -> None:
    # 28.9: the id is named as unregistered with every registered id, as for --set techniques.ID=VALUE,
    # not reported as a bad on/off value.
    out = tmp_path / "out"
    status, stdout, err = run(capsys, "infer", str(data), "--technique", "nope.x=maybe", "--out", str(out))
    assert (status, stdout) == (cli.EXIT_USAGE, "")
    assert err == (
        "jsinfer: error: unregistered Technique id 'nope.x' (flag); "
        f"the registered Technique ids are: {', '.join(REGISTRY.ids())}\n"
    )
    assert not out.exists()


def test_prerequisite_and_unimplemented_problems(capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path) -> None:
    flags = ["--technique", "decide.mdl=off", "--technique", "structure.hybrid_record_map=on"]
    unimplemented = "  - Technique 'structure.hybrid_record_map' is enabled (flag) but is not implemented yet; disable it"
    prerequisite = "but its prerequisite 'decide.mdl' is disabled (flag)"  # 28.8: the Technique and its prerequisite
    status, _, err = run(capsys, "infer", str(data), *flags, "--out", str(tmp_path / "out"))
    assert status == cli.EXIT_USAGE
    assert err.splitlines()[0] == "jsinfer: error: 2 configuration problems:"
    assert unimplemented in err.splitlines() and prerequisite in err

    # With an unreadable configuration file the effective Technique states are unknown, so no
    # prerequisite problem is claimed; the file problem and the flags' own problems are reported.
    missing = tmp_path / "missing.toml"
    status, _, err = run(capsys, "infer", str(data), "-c", str(missing), *flags, "--out", str(tmp_path / "out"))
    assert status == cli.EXIT_USAGE
    assert err.splitlines() == [
        "jsinfer: error: 2 configuration problems:",
        f"  - configuration file {str(missing)!r} cannot be read: {os.strerror(errno.ENOENT)}",
        unimplemented,
    ]
    assert not (tmp_path / "out").exists()


@pytest.mark.parametrize(
    ("content", "fragment"),
    [
        (b"[run\nseed = 1\n", "cannot be parsed as TOML"),
        (b"\xff\xfe", "is not valid UTF-8"),
        (b"[run]\nseeds = 1\n", "unknown setting [run] seeds"),
        (b'[run]\nseed = "one"\n', "seed: expected"),
    ],
)
def test_configuration_file_problems_exit_2(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path, content: bytes, fragment: str
) -> None:
    config_file = tmp_path / "run.toml"
    config_file.write_bytes(content)
    status, _, err = run(capsys, "infer", str(data), "-c", str(config_file), "--out", str(tmp_path / "out"))
    assert status == cli.EXIT_USAGE and err.startswith("jsinfer: error: ") and fragment in err
    assert not (tmp_path / "out").exists()


def test_unusable_out_is_a_configuration_error(capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path) -> None:
    blocker = tmp_path / "file.txt"
    blocker.write_text("x", encoding="utf-8")
    status, _, err = run(capsys, "infer", str(data), "--out", str(blocker))
    assert status == cli.EXIT_USAGE and f"--out {str(blocker)!r} is not a directory" in err
    status, _, err = run(capsys, "infer", str(data), "--out", str(blocker / "sub"), "--set", "workers=0")
    assert status == cli.EXIT_USAGE
    assert f"--out {str(blocker / 'sub')!r} cannot be created: {str(blocker)!r} is not a directory" in err
    assert "workers: expected" in err  # reported together with the settings' problems
    assert blocker.read_text(encoding="utf-8") == "x"


def test_unloadable_model_bundle_exits_2(capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path) -> None:
    bundle = tmp_path / "no-bundle"
    status, _, err = run(capsys, "infer", str(data), "--set", f"model_bundle={bundle}", "--out", str(tmp_path / "out"))
    assert status == cli.EXIT_USAGE and repr(str(bundle)) in err and "cannot be found" in err
    assert not (tmp_path / "out").exists()


def test_rejected_input_paths_exit_2_before_any_read(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path
) -> None:
    text = tmp_path / "data.txt"
    text.write_text("{}", encoding="utf-8")
    missing = tmp_path / "missing.jsonl"
    out = tmp_path / "out"
    status, _, err = run(capsys, "infer", str(data), str(missing), str(text), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_USAGE
    assert err.splitlines() == [
        "jsinfer: error: 2 configuration problems:",
        f"  - input path {str(missing)!r} does not exist",
        f"  - input path {str(text)!r} has the extension '.txt'; expected .json, .jsonl, or .ndjson",
    ]
    assert not out.exists()  # no schema and no report: nothing was ingested (1.10)


# ---------------------------------------------------------------------------
# infer: failed runs (1.8, 27.10)
# ---------------------------------------------------------------------------


def test_no_valid_documents_exit_3_with_a_partial_report(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    data = tmp_path / "data.jsonl"
    data.write_text('\n  \n{bad\n[1,]\n', encoding="utf-8")
    out = tmp_path / "out"
    status, stdout, err = run(capsys, "infer", str(data), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_NO_DOCUMENTS and stdout == ""
    lines = err.splitlines()
    assert lines[0].startswith("jsinfer: error: no valid documents were found; skipped 2 documents or files")
    assert lines[1].startswith(f"  {data} line 3: ") and lines[2].startswith(f"  {data} line 4: ")
    assert lines[3] == f"jsinfer: the partial Inference_Report is in {str(out / 'report.json')!r}; " + (
        "no schema or explanation file was written"
    )
    assert sorted(os.listdir(out)) == ["report.json"]
    report = read_json(out / "report.json")
    assert report["status"]["state"] == "failed" and report["status"]["stage"] == INGESTOR_STAGE
    assert report["ingestion"]["skipped"] == 2


def test_an_empty_directory_has_no_valid_documents(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    empty = tmp_path / "empty"
    empty.mkdir()
    status, _, err = run(capsys, "infer", str(empty), *ONE_WORKER, "--out", str(tmp_path / "out"))
    assert status == cli.EXIT_NO_DOCUMENTS and "jsinfer: error: no valid documents were found\n" in err


def test_stage_error_exit_1_names_the_stage_and_keeps_no_schema(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    out = tmp_path / "out"
    out.mkdir()
    (out / "schema.json").write_text("from an earlier run", encoding="utf-8")
    (out / "report.json").write_text("from an earlier run", encoding="utf-8")

    def fail(self: StructureAnalyzer, view: Any) -> Any:
        raise RuntimeError("injected failure")

    monkeypatch.setattr(StructureAnalyzer, "infer_model", fail)
    status, stdout, err = run(capsys, "infer", str(data), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_STAGE_ERROR and stdout == ""
    assert err.splitlines()[0] == f"jsinfer: error: stage {ANALYZER_STAGE} failed: RuntimeError: injected failure"
    # The failed run's partial report replaces report.json; it wrote no schema, explanation, or config file.
    assert sorted(os.listdir(out)) == ["report.json", "schema.json"]
    assert (out / "schema.json").read_text(encoding="utf-8") == "from an earlier run"
    report = read_json(out / "report.json")
    assert report["status"]["state"] == "failed" and report["status"]["stage"] == ANALYZER_STAGE
    assert "injected failure" in json.dumps(report["status"]["error"])
    assert INGESTOR_STAGE in report["timings"]  # completed stages with their runtimes (27.10)


def test_a_failed_write_removes_the_temp_directory(
    capsys: pytest.CaptureFixture[str], data: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    out = tmp_path / "out"
    out.mkdir()
    (out / "schema.json").write_text("from an earlier run", encoding="utf-8")
    real = determinism.atomic_write_bytes

    def write(path: str | os.PathLike[str], payload: bytes) -> None:
        if Path(path).name == "report.json":  # after schema.json and explanation.json are staged
            raise OSError(errno.ENOSPC, os.strerror(errno.ENOSPC))
        real(path, payload)

    monkeypatch.setattr(determinism, "atomic_write_bytes", write)
    status, _, err = run(capsys, "infer", str(data), *ONE_WORKER, "--out", str(out))
    assert status == cli.EXIT_STAGE_ERROR
    assert err == f"jsinfer: error: the outputs cannot be written to {str(out)!r}: {os.strerror(errno.ENOSPC)}\n"
    assert os.listdir(out) == ["schema.json"]  # no staged file and no temp directory remain
    assert (out / "schema.json").read_text(encoding="utf-8") == "from an earlier run"


# ---------------------------------------------------------------------------
# explain (26.9, 26.12)
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def outputs(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """The four output files of a run on DOCS."""
    out = tmp_path_factory.mktemp("explain")
    for name, payload in jsinfer.infer(DOCS, RunConfiguration.resolve({"workers": 1})).files().items():
        (out / name).write_bytes(payload)
    return out


def test_explain_prints_the_record(capsys: pytest.CaptureFixture[str], outputs: Path) -> None:
    schema, explanation = outputs / "schema.json", outputs / "explanation.json"
    status, stdout, err = run(capsys, "explain", str(schema), str(explanation), "/properties/id/type")
    assert (status, err) == (cli.EXIT_OK, "")
    assert stdout == jsinfer.explain(schema, explanation, "/properties/id/type")
    assert stdout.startswith("constraint: /properties/id/type\ntechnique: baseline:type\n")


def _explain_case(outputs: Path, tmp_path: Path, case: str) -> tuple[Path, Path, str]:
    schema, explanation, pointer = outputs / "schema.json", outputs / "explanation.json", "/properties/id/type"
    if case == "schema unreadable":
        schema = tmp_path / "missing.json"
    elif case == "schema not JSON":
        schema = tmp_path / "schema.json"
        schema.write_text("{not json", encoding="utf-8")
    elif case == "explanation unreadable":
        explanation = tmp_path / "missing.json"
    elif case == "explanation not an explanation file":
        explanation = tmp_path / "explanation.json"
        explanation.write_text("[]", encoding="utf-8")
    elif case == "pointer syntax":
        pointer = "properties/id"
    elif case == "pointer unresolved":
        pointer = "/properties/nope/type"
    elif case == "pointer to a schema object":
        pointer = "/properties/id"
    elif case == "pointer without an entry":
        data = read_json(explanation)
        del data["records"][pointer]
        explanation = tmp_path / "explanation.json"
        explanation.write_text(json.dumps(data), encoding="utf-8")
    return schema, explanation, pointer


@pytest.mark.parametrize(
    ("case", "failed", "check"),
    [
        ("schema unreadable", "schema", "read"),
        ("schema not JSON", "schema", "parse"),
        ("explanation unreadable", "explanation", "read"),
        ("explanation not an explanation file", "explanation", "parse"),
        ("pointer syntax", "pointer", "syntax"),
        ("pointer unresolved", "pointer", "resolve"),
        ("pointer to a schema object", "pointer", "resolve"),
        ("pointer without an entry", "pointer", "entry"),
    ],
)
def test_explain_errors_exit_2_and_print_no_record(
    capsys: pytest.CaptureFixture[str], outputs: Path, tmp_path: Path, case: str, failed: str, check: str
) -> None:
    schema, explanation, pointer = _explain_case(outputs, tmp_path, case)
    with pytest.raises(jsinfer.ExplainError) as caught:
        jsinfer.explain(schema, explanation, pointer)
    assert (caught.value.input, caught.value.check) == (failed, check)
    status, stdout, err = run(capsys, "explain", str(schema), str(explanation), pointer)
    assert (status, stdout) == (cli.EXIT_USAGE, "")
    assert err == f"jsinfer: error: {caught.value}\n"
    if failed != "pointer":  # the message names the failed file by its path
        assert repr(str(schema if failed == "schema" else explanation)) in err


# ---------------------------------------------------------------------------
# list-techniques (28.10) and list-thresholds (20.2)
# ---------------------------------------------------------------------------


def test_list_techniques_prints_every_registry_entry(
    capsys: pytest.CaptureFixture[str], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.chdir(tmp_path)  # an empty working directory: there is nothing to read
    status, stdout, err = run(capsys, "list-techniques")
    assert (status, err) == (cli.EXIT_OK, "")
    assert stdout == cli.techniques_text(REGISTRY)
    header, *blocks = stdout.rstrip("\n").split("\n\n")
    assert header == f"{len(REGISTRY.techniques)} registered Techniques, in registry order:"
    assert [block.split(":", 1)[0] for block in blocks] == list(REGISTRY.ids())
    for block, (tid, spec) in zip(blocks, REGISTRY.techniques.items()):
        lines = block.splitlines()
        assert f"  default: {'enabled' if spec.default_enabled else 'disabled'}" in lines, tid
        assert f"  prerequisites: {', '.join(spec.prerequisites) or 'none'}" in lines, tid
        for kind in spec.serves:
            dp = REGISTRY.decision_point(kind)
            assert f"    {kind} ({dp.title}): rank {spec.priority[kind]} of {len(dp.served_by)}" in lines, tid
            assert f"      Fallback_Behavior: {dp.fallback_doc}" in lines, tid
    assert os.listdir(tmp_path) == []


def test_list_thresholds_with_the_default_bundle(capsys: pytest.CaptureFixture[str]) -> None:
    status, stdout, err = run(capsys, "list-thresholds")
    assert (status, err) == (cli.EXIT_OK, "")
    assert stdout.splitlines()[0] == (
        f"{len(THRESHOLDS)} registered Learned_Thresholds; Model_Bundle 'bundles/default' "
        "(bundle id 'default') holds a usable value for 0:"
    )
    blocks = stdout.rstrip("\n").split("\n\n")[1:]
    assert [block.splitlines()[0] for block in blocks] == list(THRESHOLDS)
    for block, spec in zip(blocks, THRESHOLDS.values()):
        assert f"  Techniques: {', '.join(spec.techniques)}" in block
        assert f"  Decision_Point: {spec.decision_point}" in block
        assert "  value: none (missing: the Model_Bundle has no record of it)" in block


def test_list_thresholds_with_a_stub_bundle(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    entries = {"cooccur.min_support": threshold_record("cooccur.min_support", 3), "dbscan.eps": {"value": "wide"}}
    bundle = write_bundle(tmp_path / "bundle", entries)
    status, stdout, err = run(capsys, "list-thresholds", "--bundle", str(bundle))
    assert (status, err) == (cli.EXIT_OK, "")
    assert stdout == cli.thresholds_text(load_bundle(str(bundle)))
    blocks = {block.splitlines()[0]: block for block in stdout.rstrip("\n").split("\n\n")[1:]}
    assert "(bundle id 'cli-stub') holds a usable value for 1:" in stdout.splitlines()[0]
    assert blocks["cooccur.min_support"].endswith("\n  value: 3")
    assert "\n  value: none (invalid: " in blocks["dbscan.eps"]
    assert "\n  value: none (missing: " in blocks["dbscan.min_samples"]


def test_list_thresholds_with_an_unloadable_bundle(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    status, stdout, err = run(capsys, "list-thresholds", "--bundle", str(tmp_path / "none"))
    assert (status, stdout) == (cli.EXIT_USAGE, "")
    assert err.startswith(f"jsinfer: error: model_bundle: Model_Bundle {str(tmp_path / 'none')!r} cannot be found")


# ---------------------------------------------------------------------------
# Streams, imports, and the console script
# ---------------------------------------------------------------------------


def test_text_a_stream_cannot_encode_is_escaped() -> None:
    raw = io.BytesIO()
    stream = io.TextIOWrapper(raw, encoding="ascii")
    cli._write(stream, "caf\u00e9 \U0001f600\n")
    assert raw.getvalue() == b"caf\\xe9 \\U0001f600\n"


_FOOTPRINT = """
import sys
from jsinfer.cli import main
status = main(sys.argv[1:])
heavy = [m for m in ("numpy", "sklearn", "jsonschema_rs", "jsinfer.pipeline.infer", "jsinfer.learn.bundle")
         if m in sys.modules]
print(status, heavy, file=sys.stderr)
"""


@pytest.mark.parametrize("argv", [["--version"], ["list-techniques"]])
def test_light_commands_do_not_import_the_pipeline(tmp_path: Path, argv: list[str]) -> None:
    result = subprocess.run(
        [sys.executable, "-c", _FOOTPRINT, *argv],
        cwd=tmp_path,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        check=True,
        timeout=60,
    )
    assert result.stderr.splitlines()[-1] == "0 []"


def test_console_script_exits_with_the_status_of_main(tmp_path: Path) -> None:
    script = Path(sys.executable).with_name("jsinfer")
    if not script.exists():
        pytest.skip("the jsinfer console script is not installed next to this interpreter")
    ok = subprocess.run([script, "--version"], capture_output=True, text=True, timeout=60, stdin=subprocess.DEVNULL)
    assert (ok.returncode, ok.stdout) == (0, f"jsinfer {jsinfer.__version__}\n")
    bad = subprocess.run(
        [script, "list-thresholds", "--bundle", str(tmp_path / "none")],
        capture_output=True,
        text=True,
        timeout=60,
        stdin=subprocess.DEVNULL,
    )
    assert bad.returncode == cli.EXIT_USAGE and bad.stdout == "" and "cannot be found" in bad.stderr
