"""The public API with descriptor 0 closed and more than one worker (task 19.1; Requirement 27.1).

A program that calls ``jsinfer.infer`` or ``jsinfer.infer_files`` with
descriptor 0 closed gets the same outputs as with one worker, and finds
descriptor 0 closed again after the call (design, Open Decisions, Standard
input). Each check runs in a subprocess started with descriptor 0 closed
(``<&-``), as the CLI's closed-stdin test does.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
TIMEOUT = 300

_SCRIPT = """
import json, os, sys
import jsinfer
from jsinfer import RunConfiguration

def closed():
    try:
        os.fstat(0)
    except OSError:
        return True
    return False

path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    docs = [json.loads(line) for line in handle]
start = closed()
outputs = {}
for workers in (2, 1):
    config = RunConfiguration.resolve({"workers": workers, "ingest.chunk_size": 4})
    files = jsinfer.infer_files([path], config).files()
    memory = jsinfer.infer(docs, config)
    outputs[workers] = (files["schema.json"], files["explanation.json"], memory.schema_bytes)
    print(json.dumps({"workers": workers, "closed_after": closed()}))
print(json.dumps({"closed_at_start": start, "equal": outputs[1] == outputs[2]}))
"""


@pytest.mark.skipif(os.name != "posix", reason="closed descriptors are a POSIX feature")
def test_infer_and_infer_files_cope_with_descriptor_0_closed(tmp_path: Path) -> None:
    data = tmp_path / "docs.jsonl"
    docs = [{"id": i, "kind": ["a", "b"][i % 2], **({"x": i} if i % 3 else {"y": [i]})} for i in range(24)]
    data.write_text("".join(json.dumps(doc) + "\n" for doc in docs), encoding="utf-8")
    command = ["/bin/sh", "-c", 'exec "$@" <&-', "sh", sys.executable, "-c", textwrap.dedent(_SCRIPT), str(data)]
    result = subprocess.run(
        command, cwd=tmp_path, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=TIMEOUT, check=False
    )
    assert result.returncode == 0, result.stderr
    lines = [json.loads(line) for line in result.stdout.splitlines()]
    assert lines == [
        {"workers": 2, "closed_after": True},
        {"workers": 1, "closed_after": True},
        {"closed_at_start": True, "equal": True},
    ], result.stderr
