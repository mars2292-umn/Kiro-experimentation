"""Unit tests for core/workers.py (task 19.1; Requirements 27.1, 27.5, 29.4).

``single_threaded`` limits OpenMP and BLAS to one thread in each task, in
worker processes and in the calling process, and pickles by reference.
``reserved_stdin`` holds ``os.devnull`` on a closed descriptor 0 for the
duration of a block and gives it back afterwards; those checks run in
subprocesses started with descriptor 0 closed, so the test process's own
descriptors are never touched.
"""

from __future__ import annotations

import os
import pickle
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest
import sklearn.utils._openmp_helpers  # noqa: F401  # loads the OpenMP runtime scikit-learn uses
from joblib import Parallel, delayed
from threadpoolctl import threadpool_info

from jsinfer.core.workers import single_threaded

ROOT = Path(__file__).resolve().parents[2]
TIMEOUT = 120


def _thread_counts() -> list[int]:
    """The thread count of every pool threadpoolctl sees in this process (module-level, so it pickles)."""
    return [pool["num_threads"] for pool in threadpool_info()]


def _add(a: int, b: int = 0) -> int:
    return a + b


# ---------------------------------------------------------------------------
# single_threaded
# ---------------------------------------------------------------------------


def test_single_threaded_limits_pools_during_the_call_and_restores_them() -> None:
    before = _thread_counts()
    if not before:
        pytest.skip("no thread pool that threadpoolctl controls is loaded")
    inside = single_threaded(_thread_counts)()
    assert inside == [1] * len(before)
    assert _thread_counts() == before


def test_single_threaded_limits_pools_in_worker_processes() -> None:
    if not _thread_counts():
        pytest.skip("no thread pool that threadpoolctl controls is loaded")
    # The workers import this module to unpickle the task, which loads the same pools.
    results = Parallel(n_jobs=2)(delayed(single_threaded(_thread_counts))() for _ in range(4))
    assert len(results) == 4
    assert all(counts and set(counts) == {1} for counts in results)


def test_single_threaded_passes_arguments_and_pickles_by_reference() -> None:
    task = single_threaded(_add)
    assert task(2, b=3) == 5
    again = pickle.loads(pickle.dumps(task))
    assert again(4) == 4 and again.function is _add  # type: ignore[attr-defined]
    assert "_add" in repr(task)
    with pytest.raises(TypeError):
        single_threaded(3)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# reserved_stdin (design, Open Decisions, Standard input)
# ---------------------------------------------------------------------------

_STATE = """
import os
from jsinfer.core.workers import reserved_stdin

NULL = os.stat(os.devnull)

def state():
    try:
        status = os.fstat(0)
    except OSError:
        return "closed"
    return "devnull" if (status.st_dev, status.st_ino) == (NULL.st_dev, NULL.st_ino) else "other"
"""

_NESTED = _STATE + """
out = [state()]
with reserved_stdin():
    out += [state(), str(os.get_inheritable(0))]
    with reserved_stdin():
        out.append(state())
    out.append(state())  # still held: the outer block is running
    r, w = os.pipe()
    out.append(str(min(r, w) > 0))  # a pipe opened meanwhile never takes descriptor 0
    os.close(r)
    os.close(w)
out.append(state())
with reserved_stdin(False):
    out.append(state())
print(" ".join(out))
"""

_REPLACED = _STATE + """
import tempfile
out = []
with reserved_stdin():
    os.close(0)
    handle = tempfile.TemporaryFile()  # the program puts its own file on descriptor 0
    out += [str(handle.fileno()), state()]
out.append(state())  # not ours, so not closed
print(" ".join(out))
"""


def _run(script: str, *, closed: bool) -> str:
    if os.name != "posix":
        pytest.skip("closed descriptors are a POSIX feature")
    command = [sys.executable, "-c", textwrap.dedent(script)]
    if closed:
        command = ["/bin/sh", "-c", 'exec "$@" <&-', "sh", *command]
    result = subprocess.run(
        command, cwd=ROOT, stdin=subprocess.PIPE, capture_output=True, text=True, timeout=TIMEOUT, check=False
    )
    assert result.returncode == 0, result.stderr
    return result.stdout.strip()


def test_a_closed_descriptor_0_is_held_during_the_block_and_closed_after() -> None:
    assert _run(_NESTED, closed=True) == "closed devnull True devnull devnull True closed closed"


def test_an_open_descriptor_0_is_left_alone() -> None:
    assert _run(_NESTED, closed=False) == "other other True other other True other other"


def test_a_descriptor_0_the_program_replaced_is_not_closed() -> None:
    assert _run(_REPLACED, closed=True) == "0 other other"
