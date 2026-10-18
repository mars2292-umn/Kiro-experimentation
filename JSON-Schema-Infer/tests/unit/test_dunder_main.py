"""``python -m jsinfer`` (``src/jsinfer/__main__.py``): the command-line interface as a module."""

from __future__ import annotations

import importlib
import json
import os
import subprocess
import sys
from pathlib import Path

import jsinfer
from jsinfer import cli
from jsinfer.config.run_config import RunConfiguration

TIMEOUT = 300
OUTPUTS = ["effective-config.toml", "explanation.json", "report.json", "schema.json"]


def run_module(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-m", "jsinfer", *args],
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        stdin=subprocess.DEVNULL,
        capture_output=True,
        encoding="utf-8",
        timeout=TIMEOUT,
        check=False,
    )


def test_python_m_jsinfer_is_the_cli() -> None:
    result = run_module("--version")
    assert (result.returncode, result.stdout, result.stderr) == (cli.EXIT_OK, f"jsinfer {jsinfer.__version__}\n", "")
    assert run_module().returncode == cli.EXIT_USAGE  # no command, as cli.main([])
    module = importlib.import_module("jsinfer.__main__")  # importing it runs nothing
    assert module.main is cli.main


def test_python_m_jsinfer_runs_with_worker_processes(tmp_path: Path) -> None:
    # A run that launches loky worker processes from a ``-m`` main module, with the API's bytes.
    data = tmp_path / "docs.jsonl"
    data.write_text("".join(json.dumps({"id": i, "kind": "ab"[i % 2]}) + "\n" for i in range(8)), encoding="utf-8")
    out = tmp_path / "out"
    flags = {"workers": 2, "ingest.chunk_size": 1}
    result = run_module("infer", str(data), *(f"--set={name}={value}" for name, value in flags.items()), "--out", str(out))
    assert result.returncode == cli.EXIT_OK, result.stderr
    assert sorted(os.listdir(out)) == OUTPUTS
    files = jsinfer.infer_files([data], RunConfiguration.resolve(flags)).files()
    for name in ("schema.json", "explanation.json", "effective-config.toml"):
        assert (out / name).read_bytes() == files[name], name
