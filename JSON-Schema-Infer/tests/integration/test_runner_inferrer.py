"""Integration test of the Inferrer adapter: ``jsinfer infer`` in a child process (task 28.3; Requirement 34.1)."""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from jsinfer.bench.baselines.inferrer import InferrerAdapter
from jsinfer.bench.runner import run_system, write_jsonl

DOCS = [{"id": 1, "name": "a", "tags": ["x"]}, {"id": 2, "name": "b", "tags": []}, {"id": 3, "name": "c"}]


@pytest.fixture
def docs_jsonl(tmp_path: Path) -> Path:
    path = tmp_path / "docs.jsonl"
    write_jsonl(DOCS, path)
    return path


def test_inferrer_adapter_runs_jsinfer_infer(tmp_path: Path, docs_jsonl: Path) -> None:
    adapter = InferrerAdapter(settings={"workers": 1})
    assert adapter.available().available
    command = list(adapter.command(docs_jsonl, tmp_path / "out", 5))
    assert "infer" in command and "--set" in command and "seed=5" in command and "workers=1" in command
    run = run_system(adapter, docs_jsonl, tmp_path / "out", seed=5, timeout_s=600, env={**os.environ, "PYTHONHASHSEED": "0"})
    assert run.kind == "schema", (run.reason, run.stderr[-2000:])
    assert run.document["type"] == "object"
    assert (tmp_path / "out" / "report.json").exists()
    assert run.info.name == "jsinfer" and run.info.options == {"workers": 1}


