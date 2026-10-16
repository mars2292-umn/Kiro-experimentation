"""Seeded RNG streams, canonical ordering, canonical encoders, and atomic writers.

Everything that can reach an emitted file goes through this module, so that
equal inputs and equal Run_Configurations give byte-identical outputs
(Requirements 2.3, 27.2, 27.5).

Two byte forms are defined here and must not be confused:

``canonical_bytes(value)``
    A compact, normalized form used for hashing and ordering (never written to
    an output file by itself). It is
    ``json.dumps(normalize(value), ensure_ascii=False, allow_nan=False,
    sort_keys=True, separators=(",", ":"))`` encoded as UTF-8 with
    ``surrogatepass``, where ``normalize``:

    - maps every finite float with a zero fractional part to ``int`` (so ``1.0``
      and ``1`` give the same bytes, matching JSON Schema instance equality),
      and rejects NaN and infinities;
    - keeps ``bool`` apart from numbers (``true`` versus ``1``);
    - encodes tuples as arrays, and numpy scalars as the matching Python scalar;
    - sorts object keys by code point (keys must be ``str``);
    - encodes an object that defines ``__canonical__()`` as the canonical form
      of the value that method returns. Path steps use this hook, so
      Path-like keys hash by content and never by ``repr`` or ``hash()``.

    Two values have equal canonical bytes exactly when they are equal as JSON
    values under JSON Schema equality (with tuples treated as arrays).

``encode_canonical(obj)``
    The output encoder for schemas, explanation files, and reports:
    ``json.dumps(obj, indent=2, ensure_ascii=False, allow_nan=False) + "\\n"`` in
    strict UTF-8. It does not sort keys. Callers (the Schema_Emitter, the report
    builder) supply dicts whose insertion order is already canonical, for
    example the emitter's fixed keyword rank order.

Random streams come only from :func:`rng`, keyed by purpose (a Technique or
component id) and by data keys such as Paths, never by execution order.
Python's ``hash()`` is never used, so ``PYTHONHASHSEED`` cannot change results.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import secrets
import shutil
from collections.abc import Callable, Iterable, Iterator, Mapping
from contextlib import contextmanager
from pathlib import Path
from typing import IO, Any, TypeVar

import numpy as np

__all__ = [
    "DEFAULT_SEED",
    "RANK_ARRAY",
    "RANK_BOOLEAN",
    "RANK_INTEGER",
    "RANK_NULL",
    "RANK_NUMBER",
    "RANK_OBJECT",
    "RANK_STRING",
    "atomic_directory",
    "atomic_file",
    "atomic_outputs",
    "atomic_write_bytes",
    "atomic_write_text",
    "canonical_bytes",
    "codepoint_key",
    "dumps_canonical",
    "encode_canonical",
    "json_type_rank",
    "random_state",
    "rng",
    "seed_sequence",
    "sorted_codepoint",
    "sorted_items",
    "sorted_values",
    "stable_hash32",
    "value_sort_key",
]

T = TypeVar("T")
V = TypeVar("V")
StrPath = str | os.PathLike[str]

# The fixed default seed (Requirement 27.2). It is a constant, never derived
# from the clock or from system entropy.
DEFAULT_SEED = 0

# ---------------------------------------------------------------------------
# Canonical bytes and stable hashing
# ---------------------------------------------------------------------------


def _normalize(value: Any) -> Any:
    """Return a JSON-encodable equivalent of ``value`` in canonical form."""
    # bool first: bool is a subclass of int, and numpy bools are not bools.
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, np.bool_):
        return bool(value)
    if isinstance(value, (int, np.integer)):
        return int(value)
    if isinstance(value, (float, np.floating)):
        x = float(value)
        if not math.isfinite(x):
            raise ValueError(f"non-finite number has no canonical form: {x!r}")
        return int(x) if x.is_integer() else x
    if isinstance(value, (list, tuple)):
        return [_normalize(item) for item in value]
    if isinstance(value, dict):
        out: dict[str, Any] = {}
        for key, item in value.items():
            if not isinstance(key, str):
                raise TypeError(f"object keys must be str for canonical form, got {type(key).__name__}")
            out[key] = _normalize(item)
        return out
    hook = getattr(type(value), "__canonical__", None)
    if hook is not None:
        return _normalize(hook(value))
    raise TypeError(f"{type(value).__name__} has no canonical form")


def canonical_bytes(value: Any) -> bytes:
    """Return the compact canonical bytes of ``value`` (see the module docstring).

    Raises ``TypeError`` for values without a canonical form (sets, arbitrary
    objects, non-``str`` keys) and ``ValueError`` for NaN or infinities.
    """
    text = json.dumps(
        _normalize(value),
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    )
    # surrogatepass keeps hashing total even for strings with lone surrogates;
    # these bytes are internal and never written as output.
    return text.encode("utf-8", "surrogatepass")


def stable_hash32(value: Any) -> int:
    """Return a 32-bit BLAKE2b digest of ``canonical_bytes(value)`` as an int.

    This is the ``h`` of the design's seeding scheme. It is independent of the
    process, the platform, and ``PYTHONHASHSEED``.
    """
    digest = hashlib.blake2b(canonical_bytes(value), digest_size=4).digest()
    return int.from_bytes(digest, "big")


# ---------------------------------------------------------------------------
# Seeded random streams
# ---------------------------------------------------------------------------


def _check_seed(seed: int) -> int:
    if isinstance(seed, bool) or not isinstance(seed, (int, np.integer)) or seed < 0:
        raise ValueError(f"seed must be a non-negative integer, got {seed!r}")
    return int(seed)


def seed_sequence(purpose: str, *keys: Any, seed: int = DEFAULT_SEED) -> np.random.SeedSequence:
    """Return ``SeedSequence(seed, spawn_key=(h(purpose), *map(h, keys)))``.

    ``purpose`` names the consumer (a Technique or component id). ``keys``
    identify the data the stream is for (Paths, schema ids, indices), so the
    stream does not depend on the order in which work is executed.
    """
    if not isinstance(purpose, str) or not purpose:
        raise ValueError(f"purpose must be a non-empty str, got {purpose!r}")
    spawn_key = (stable_hash32(purpose), *(stable_hash32(key) for key in keys))
    return np.random.SeedSequence(_check_seed(seed), spawn_key=spawn_key)


def rng(purpose: str, *keys: Any, seed: int = DEFAULT_SEED) -> np.random.Generator:
    """Return the random stream for ``(seed, purpose, *keys)``.

    Equal arguments give identical streams in every process. Different
    purposes, keys, key orders, or seeds give independent streams (up to
    32-bit collisions of the key hashes).
    """
    return np.random.default_rng(seed_sequence(purpose, *keys, seed=seed))


def random_state(purpose: str, *keys: Any, seed: int = DEFAULT_SEED) -> int:
    """Return a 32-bit integer drawn from the stream, for sklearn ``random_state``."""
    return int(seed_sequence(purpose, *keys, seed=seed).generate_state(1, np.uint32)[0])


# ---------------------------------------------------------------------------
# Canonical ordering
# ---------------------------------------------------------------------------


def codepoint_key(text: str) -> str:
    """Sort key for code-point order.

    Python compares ``str`` by code point (not by locale and not by UTF-16
    code unit), so the key is the string itself. Use it so that every
    code-point sort in the code base reads the same way.
    """
    return text


def sorted_codepoint(strings: Iterable[str]) -> list[str]:
    """Return ``strings`` sorted in code-point order."""
    return sorted(strings, key=codepoint_key)


def sorted_items(mapping: Mapping[str, V]) -> list[tuple[str, V]]:
    """Return the items of a ``str``-keyed mapping in code-point key order."""
    return sorted(mapping.items(), key=lambda item: item[0])


# Type ranks of the canonical value order. They equal the ``JType`` values of
# the design (null, boolean, integer, number, string, array, object), which
# is also the order of emitted ``type`` lists.
RANK_NULL = 0
RANK_BOOLEAN = 1
RANK_INTEGER = 2
RANK_NUMBER = 3
RANK_STRING = 4
RANK_ARRAY = 5
RANK_OBJECT = 6


def json_type_rank(value: Any) -> int:
    """Return the type rank of a JSON value.

    Numbers with a zero fractional part (``1``, ``1.0``, ``1e3``) rank as
    integers. Booleans rank apart from numbers. Tuples rank as arrays.
    """
    if value is None:
        return RANK_NULL
    if isinstance(value, bool):
        return RANK_BOOLEAN
    if isinstance(value, int):
        return RANK_INTEGER
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"non-finite number is not a JSON value: {value!r}")
        return RANK_INTEGER if value.is_integer() else RANK_NUMBER
    if isinstance(value, str):
        return RANK_STRING
    if isinstance(value, (list, tuple)):
        return RANK_ARRAY
    if isinstance(value, dict):
        return RANK_OBJECT
    raise TypeError(f"{type(value).__name__} is not a JSON value")


def value_sort_key(value: Any) -> tuple[int, Any]:
    """Return the canonical value-order key ``(type_rank, payload)`` of a JSON value.

    The payload is ``None`` for null, ``0``/``1`` for booleans, the value as an
    ``int`` for integral numbers, the ``float`` for other numbers, the string
    itself (code-point order), and ``canonical_bytes`` for arrays and objects.
    Keys are equal exactly when the values are equal under JSON Schema
    equality, so the key also serves as an interning key. Ranks are compared
    first, so payloads of different Python types are never compared.
    """
    rank = json_type_rank(value)
    if rank == RANK_NULL:
        return (rank, None)
    if rank == RANK_BOOLEAN:
        return (rank, int(value))
    if rank == RANK_INTEGER:
        return (rank, int(value))
    if rank in (RANK_NUMBER, RANK_STRING):
        return (rank, value)
    return (rank, canonical_bytes(value))


def sorted_values(values: Iterable[Any]) -> list[Any]:
    """Return JSON values in canonical value order (the emitted ``enum`` order)."""
    return sorted(values, key=value_sort_key)


# ---------------------------------------------------------------------------
# Canonical output encoder
# ---------------------------------------------------------------------------


def dumps_canonical(obj: Any) -> str:
    """Return ``json.dumps(obj, indent=2, ensure_ascii=False, allow_nan=False) + "\\n"``.

    Keys keep their insertion order; callers supply canonically ordered dicts.
    NaN and infinities raise ``ValueError``.
    """
    return json.dumps(obj, indent=2, ensure_ascii=False, allow_nan=False) + "\n"


def encode_canonical(obj: Any) -> bytes:
    """Return :func:`dumps_canonical` encoded as strict UTF-8 (the output bytes)."""
    return dumps_canonical(obj).encode("utf-8")


# ---------------------------------------------------------------------------
# Atomic writers
# ---------------------------------------------------------------------------
#
# Every output goes to a temporary sibling first and is then renamed into
# place with os.replace, which is atomic within one filesystem. A failure or
# interruption therefore never leaves a partial file at the target. Temporary
# names start with "." and use random suffixes; they never reach any output.

_TEMP_ATTEMPTS = 100


def _create_unique(directory: Path, stem: str, suffix: str, make: Callable[[Path], T]) -> tuple[Path, T]:
    """Create a fresh hidden path in ``directory`` with ``make`` and return it."""
    stem = stem[:32]  # keep temp names well under NAME_MAX
    for _ in range(_TEMP_ATTEMPTS):
        candidate = directory / f".{stem}.{secrets.token_hex(6)}{suffix}"
        try:
            return candidate, make(candidate)
        except FileExistsError:
            continue
    raise FileExistsError(f"could not create a temporary path in {directory}")


def _open_new_file(path: Path) -> int:
    # Mode 0o666 lets the process umask apply, as for an ordinary open().
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    return os.open(path, flags, 0o666)


def _make_dir(path: Path) -> None:
    os.mkdir(path, 0o777)  # the process umask applies


def _require_absent(path: Path) -> None:
    if os.path.lexists(path):
        raise FileExistsError(path)


def _fsync_dir(directory: Path) -> None:
    """Flush a directory entry update to disk where the platform allows it."""
    try:
        fd = os.open(directory, os.O_RDONLY)
    except OSError:
        return
    try:
        os.fsync(fd)
    except OSError:
        pass
    finally:
        os.close(fd)


def _unlink_quietly(path: Path) -> None:
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass


@contextmanager
def atomic_file(path: StrPath, *, text: bool = False) -> Iterator[IO[Any]]:
    """Open a temporary sibling of ``path`` for writing and rename it into place on success.

    Yields a binary file, or with ``text=True`` a UTF-8 text file without
    newline translation. On normal exit the data is flushed and fsynced, then
    the temp file replaces ``path`` (an existing file is overwritten without
    confirmation, Requirement 27.1). If the block raises, the temp file is
    removed, ``path`` is left untouched, and the exception propagates. The
    parent directory must exist.
    """
    target = Path(path)
    tmp, fd = _create_unique(target.parent, target.name, ".tmp", _open_new_file)
    try:
        handle: IO[Any] = open(fd, "w", encoding="utf-8", newline="") if text else open(fd, "wb")
    except BaseException:
        os.close(fd)
        _unlink_quietly(tmp)
        raise
    try:
        with handle:
            yield handle
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, target)
    except BaseException:
        _unlink_quietly(tmp)
        raise
    _fsync_dir(target.parent)


def atomic_write_bytes(path: StrPath, data: bytes) -> None:
    """Write ``data`` to ``path`` atomically (temp file, fsync, rename)."""
    with atomic_file(path) as handle:
        handle.write(data)


def atomic_write_text(path: StrPath, text: str) -> None:
    """Write ``text`` to ``path`` atomically as strict UTF-8 without newline translation."""
    atomic_write_bytes(path, text.encode("utf-8"))


def _replace_entry(src: Path, dst: Path) -> None:
    """Move ``src`` to ``dst``, replacing an existing file or directory of that name."""
    if src.is_dir() and not src.is_symlink() and dst.is_dir() and not dst.is_symlink():
        # rename() cannot replace a non-empty directory: move the old one
        # aside, rename the new one in, then delete the old one.
        old, _ = _create_unique(dst.parent, dst.name, ".old", _require_absent)
        os.replace(dst, old)
        try:
            os.replace(src, dst)
        except BaseException:
            os.replace(old, dst)
            raise
        shutil.rmtree(old)
    else:
        os.replace(src, dst)


@contextmanager
def atomic_directory(target: StrPath) -> Iterator[Path]:
    """Build a directory in a temp sibling of ``target`` and rename it into place on success.

    Used for whole-directory artifacts such as a Model_Bundle. On normal exit
    the temp directory becomes ``target``; an existing ``target`` directory is
    replaced as a whole. If the block raises, the temp directory is deleted,
    an existing ``target`` is left untouched, and the exception propagates. The
    parent directory must exist.
    """
    final = Path(target)
    tmp, _ = _create_unique(final.parent, final.name, ".tmp", _make_dir)
    try:
        yield tmp
        _replace_entry(tmp, final)
    except BaseException:
        shutil.rmtree(tmp, ignore_errors=True)
        raise
    _fsync_dir(final.parent)


@contextmanager
def atomic_outputs(out_dir: StrPath) -> Iterator[Path]:
    """Stage output files in a temp directory under ``out_dir`` and move them in on success.

    ``out_dir`` is created (with parents) if needed. On normal exit each
    top-level entry of the temp directory is renamed into ``out_dir`` in
    code-point order, replacing existing entries of the same names without
    confirmation (Requirement 27.1); other entries of ``out_dir`` are kept.
    The temp directory is then removed. If the block raises, the temp
    directory and everything in it are deleted, so no output of the failed
    run remains (Requirement 27.10), and the exception propagates.

    Each rename is atomic, so every file in ``out_dir`` is whole. If a rename
    itself fails part-way, entries renamed before it stay in place.
    """
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    tmp, _ = _create_unique(out, "jsinfer-out", ".tmp", _make_dir)
    try:
        yield tmp
        for name in sorted_codepoint(os.listdir(tmp)):
            _replace_entry(tmp / name, out / name)
        os.rmdir(tmp)
    except BaseException:
        shutil.rmtree(tmp, ignore_errors=True)
        raise
    _fsync_dir(out)
