"""Worker processes for per-chunk work (Requirements 27.1, 27.5, 29.4).

The stages hand independent work units to joblib, ``Parallel(n_jobs=workers)``
with at most ``workers`` processes (29.4). The work units are defined by the
data, never by the worker count: fixed-size chunks of ``ingest.chunk_size``
documents for parsing, flattening, and validation. joblib returns results in
submission order and the stages reduce them exactly, so the results do not
depend on the worker count (design, Determinism and reproducibility). This
module holds the two pieces of process plumbing that the worker launch needs.

:func:`single_threaded`
    Wraps a task function so that each call runs with every thread pool that
    threadpoolctl controls (OpenMP and BLAS) limited to one thread. joblib's
    loky workers otherwise get ``cpu_count // n_jobs`` threads each, so a
    task whose floating-point reductions run on such a pool could give
    results that depend on the worker count. The limit also applies with one
    worker, when joblib runs the tasks in the calling process, so a task
    sees one thread whatever the worker count; the calling process's limits
    are restored when the task returns. The wrapper pickles as a reference
    to the wrapped function, which must be a module-level function.

:func:`reserved_stdin` (design, Open Decisions, Standard input)
    When descriptor 0 is closed, the next file or pipe the process opens
    takes number 0. loky's worker launch opens pipes, and a worker started
    while one of them is descriptor 0 fails at interpreter startup
    ("init_sys_streams: can't initialize sys standard streams"). The CLI
    (``cli.main``) opens ``os.devnull`` on descriptor 0 for the rest of its
    process. A program calling ``jsinfer.infer`` or ``jsinfer.infer_files``
    owns its process, so the pipeline holds ``os.devnull`` on descriptor 0
    only while a run that can launch workers is in progress, and closes it
    again afterwards. Workers launched meanwhile keep ``os.devnull`` as their
    standard input; the loky executor they belong to keeps no reference to
    descriptor 0.

    The reservation never takes over a descriptor someone else holds: it
    acts only when ``fstat(0)`` reports a closed descriptor, opens
    ``os.devnull`` with ``os.open`` (which returns the lowest free number)
    instead of ``dup2``, and gives the descriptor back when another thread
    took number 0 first. Concurrent runs share one reservation, counted
    under a lock, and the last run to finish closes it, but only while
    descriptor 0 is still the ``os.devnull`` it opened. It does nothing
    outside POSIX.
"""

from __future__ import annotations

import errno
import os
import threading
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from typing import Any, Final, Generic, ParamSpec, TypeVar

from threadpoolctl import threadpool_limits

__all__ = ["reserved_stdin", "single_threaded"]

P = ParamSpec("P")
R = TypeVar("R")

_STDIN_FD: Final = 0


# ---------------------------------------------------------------------------
# Single-threaded tasks
# ---------------------------------------------------------------------------


class _SingleThreaded(Generic[P, R]):
    """A task function run with its thread pools limited to one thread (see :func:`single_threaded`)."""

    __slots__ = ("function",)

    def __init__(self, function: Callable[P, R]) -> None:
        self.function = function

    def __call__(self, *args: P.args, **kwargs: P.kwargs) -> R:
        with threadpool_limits(limits=1):
            return self.function(*args, **kwargs)

    def __reduce__(self) -> tuple[Any, ...]:
        return (_SingleThreaded, (self.function,))

    def __repr__(self) -> str:
        return f"single_threaded({getattr(self.function, '__qualname__', self.function)!r})"


def single_threaded(function: Callable[P, R]) -> Callable[P, R]:
    """``function`` as a joblib task that runs with OpenMP and BLAS limited to one thread (see the module docstring)."""
    if not callable(function):
        raise TypeError(f"single_threaded takes a function, got {type(function).__name__}")
    return _SingleThreaded(function)


# ---------------------------------------------------------------------------
# Descriptor 0 while workers can be launched
# ---------------------------------------------------------------------------

_lock = threading.Lock()
_holders = 0
_held: tuple[int, int] | None = None  # (st_dev, st_ino) of the os.devnull this module put on descriptor 0


def _hold() -> tuple[int, int] | None:
    """Open ``os.devnull`` on descriptor 0 when it is closed; its identity, or None when nothing was opened."""
    try:
        os.fstat(_STDIN_FD)
        return None  # open: standard input, or whatever the program put there
    except OSError as error:
        if error.errno != errno.EBADF:
            return None
    fd = os.open(os.devnull, os.O_RDONLY)  # the lowest free descriptor: 0, unless another thread took it
    if fd != _STDIN_FD:
        os.close(fd)
        return None
    os.set_inheritable(fd, True)  # as standard input is; os.open gives a non-inheritable descriptor
    status = os.fstat(fd)
    return (status.st_dev, status.st_ino)


def _give_back(identity: tuple[int, int]) -> None:
    """Close descriptor 0 when it is still the ``os.devnull`` that :func:`_hold` opened."""
    try:
        status = os.fstat(_STDIN_FD)
    except OSError:
        return  # already closed by someone else
    if (status.st_dev, status.st_ino) == identity:
        os.close(_STDIN_FD)


@contextmanager
def reserved_stdin(active: bool = True) -> Iterator[None]:
    """Hold ``os.devnull`` on descriptor 0 while the block runs, when descriptor 0 is closed (see the module docstring).

    ``active`` False (a run with one worker, which launches no process)
    makes the block run unchanged.
    """
    global _holders, _held
    if not active or os.name != "posix":
        yield
        return
    with _lock:
        if _holders == 0:
            _held = _hold()
        _holders += 1
    try:
        yield
    finally:
        with _lock:
            _holders -= 1
            if _holders == 0 and _held is not None:
                identity, _held = _held, None
                _give_back(identity)
