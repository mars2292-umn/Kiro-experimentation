"""Ingestor: file enumeration, parsing (orjson, then stdlib fallback), and DocumentStore.

:class:`Ingestor` turns the supplied paths into a :class:`DocumentStore`
with one :class:`DocOrigin` per document and an
:class:`~jsinfer.core.report.IngestError` per skipped document or file
(Requirement 1). :func:`parse_document` parses one document.

Supplied paths (1.10)
    Every supplied path is checked before any document is read. A path that
    does not exist, cannot be read, is a regular file whose extension is not
    ``.json``, ``.jsonl``, or ``.ndjson``, or is neither a regular file nor a
    directory is rejected; a :class:`~jsinfer.config.errors.ConfigError`
    then names every rejected path and its reason, and nothing is ingested.
    Supplied paths are followed when they are symbolic links.

Enumeration (1.2, 1.5)
    Inputs are processed in argument order. A supplied file is one source,
    named by its path as supplied. A supplied directory is walked with
    ``os.walk(followlinks=False)``: symbolic links (to files or directories)
    and other non-regular files are skipped, dotfiles and dot-directories are
    kept, and files with other extensions are ignored. Its files are named by
    their path relative to the directory, with ``/`` separators, and are
    processed in code-point order of that name. Extensions are compared
    after ``casefold()``; the extension is that of :func:`os.path.splitext`,
    so a name that is only a dot and an extension (``.json``) has none. A
    file that cannot be read, or a subdirectory that cannot be listed, is
    recorded as an IngestError without a line, at the position of its name
    in the order. Names are reported as valid Unicode: bytes that do not
    decode (surrogate escapes) are written as backslash escapes.

Records (1.1, 1.3, 1.9)
    A ``.json`` file is one record. A ``.jsonl`` or ``.ndjson`` file is split
    on LF, one trailing CR is stripped from each line, and each non-blank
    line is a record; blank lines (empty, or only spaces and tabs) are
    skipped and consume no index. Lines are numbered from 1. Each record is
    parsed with :func:`parse_document`, so each one may carry its own BOM.
    With array unwrapping enabled (``InputOptions.unwrap_arrays``), a
    ``.json`` record whose top-level value is an array contributes one
    document per element, in element order, and none for ``[]``. Unwrapping
    never applies to nested arrays or to lines.

    Before parsing, a record nested deeper than :data:`MAX_NESTING_DEPTH`
    containers fails with kind ``depth``. RFC 8259 allows such a limit. The
    stdlib fallback parser and the canonical encoder recurse once per level,
    so without a fixed limit whether a deep document is accepted would
    depend on the call stack, which differs between the main process and the
    workers.

Skipped documents and files (1.7, 1.8)
    A record that fails to parse is recorded as an IngestError with its
    source, the 1-based line at which the error was detected (the
    DocumentParseError line, offset by the record's starting line; the
    starting line when the parser gives none), and the reason, followed by
    ``(column N)`` when the parser gives a column. Errors are kept in
    ingestion order, and ``skipped`` is their count. When no document
    parses, :class:`~jsinfer.core.errors.NoDocumentsError` is raised with the
    errors.

Parallel parsing and the DocumentStore (1.5, 29.4)
    Records are grouped in chunks of ``chunk_size`` records (the
    Resource_Parameter ``ingest.chunk_size``), whatever the worker count.
    Each chunk is parsed and re-serialized by one joblib task, in at most
    ``workers`` processes, and results are collected in submission order.
    Indices are assigned in the main process, consecutively from 0 in that
    order, so they depend on neither the worker count nor the chunk size.

    The :class:`DocumentStore` holds the canonical encoding
    (``core.determinism.encode_canonical``) of every document, concatenated
    in index order, and an int64 offsets array with ``n + 1`` entries. The
    encoding is stable for equal values, keeps every integer exact, and
    keeps member order and the spelling of integral floats (``1.0``), so
    :meth:`DocumentStore.value` gives back the parsed value.

Parsing policy (Requirements 1.4, 1.6, 1.11, 29.2)
    :func:`parse_document` strips one leading UTF-8 byte order mark and
    parses the rest with orjson. It uses the stdlib :mod:`json` decoder only
    when orjson rejects the document. orjson counts as rejecting a document
    when it raises, or when its result holds an integral ``float`` of
    magnitude at least 2**63. orjson keeps integers in [-2**63, 2**64 - 1]
    exact, but turns larger integer literals into rounded floats instead of
    raising, so such a float may be a lossy integer. The stdlib decoder keeps
    every integer exact.

    The stdlib path decodes strict UTF-8 itself, with no encoding detection
    and no BOM skipping. It rejects the constants that :mod:`json` accepts by
    default but RFC 8259 forbids (``NaN``, ``Infinity``, ``-Infinity``)
    through ``parse_constant``. It also rejects two RFC 8259 literals that
    the Validator cannot judge (design, Open Decisions): number literals
    that overflow an IEEE 754 double, and strings with an unpaired UTF-16
    surrogate escape. The jsonschema-rs smoke tests
    (``tests/unit/test_validator_edge_cases.py``, task 5.2) confirmed this
    policy. jsonschema-rs reads an infinite float as a value of no JSON type
    and raises ``UnicodeEncodeError`` instead of giving a verdict for a lone
    surrogate. It does handle ``Decimal``, but nothing else in jsinfer
    does. RFC 8259 allows a limit on the range of numbers (section 9) and
    calls unpaired surrogates unpredictable (section 8.2). The same tests
    confirmed that jsonschema-rs compares integers of any size exactly, so
    big integers are kept as exact ``int`` (1.6).
    Integer literals longer than the interpreter's integer string conversion
    limit (``sys.get_int_max_str_digits()``) fail, because the canonical
    encoder could not write them back out. Nesting deeper than the stdlib
    decoder's recursion limit also fails.

    Comments, trailing commas, single-quoted strings, more than one top-level
    value, a second BOM, and unescaped control characters are syntax errors
    for both parsers (1.11).

Failures
    Every failure raises :class:`DocumentParseError`. It carries a ``kind``
    from :data:`PARSE_FAILURE_KINDS`, a ``reason`` that describes the error
    without its position, and the 1-based ``line`` and ``column`` at which it
    was detected, counted within the bytes passed in (after the BOM, which
    holds no newline). The Ingestor adds a record's starting line for
    ``.jsonl`` and ``.ndjson`` files. Lines are counted by LF, as in
    :class:`json.JSONDecodeError`. Columns count characters. They are
    ``None`` for invalid UTF-8, whose reason gives the byte offset instead.
    The location comes from:

    - syntax errors: ``JSONDecodeError.lineno`` and ``colno`` of the stdlib
      decoder;
    - invalid UTF-8: the newline count before the offending byte;
    - constants, overflowing doubles, and over-long integers: the first token
      outside a string equal to the rejected literal. The stdlib decoder
      reads left to right and stops at the first rejection, so that token is
      the one it rejected;
    - unpaired surrogates: the first unpaired ``\\uD800``-``\\uDFFF``
      escape, paired the way the stdlib decoder pairs them;
    - nesting depth: orjson's error position when orjson raised, else
      ``None``.
"""

from __future__ import annotations

import json
import math
import os
import re
import stat
import sys
from collections import deque
from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass
from typing import Any, Final, Literal, TypeAlias

import numpy as np
import orjson
from joblib import Parallel, delayed

from jsinfer.config.errors import ConfigError, ConfigProblem
from jsinfer.config.resources import RESOURCE_PARAMETERS, WORKERS
from jsinfer.config.run_config import InputOptions
from jsinfer.core.determinism import codepoint_key, encode_canonical
from jsinfer.core.errors import NoDocumentsError
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.report import IngestError

__all__ = [
    "CHUNK_SIZE",
    "LINE_EXTENSIONS",
    "MAX_NESTING_DEPTH",
    "PARSE_FAILURE_KINDS",
    "SUPPORTED_EXTENSIONS",
    "UTF8_BOM",
    "DocOrigin",
    "DocumentParseError",
    "DocumentStore",
    "IngestResult",
    "Ingestor",
    "ParseFailureKind",
    "Parser",
    "parse_document",
]

Parser: TypeAlias = Literal["orjson", "stdlib"]

ParseFailureKind: TypeAlias = Literal[
    "syntax",
    "encoding",
    "constant",
    "number_overflow",
    "integer_digits",
    "surrogate",
    "depth",
]
# Why a document failed to parse with both parsers:
#   syntax           not RFC 8259 JSON text: a comment, a trailing comma, a single-quoted
#                    string, more than one top-level value, a second BOM, an empty document, ...
#   encoding         a byte sequence that is not valid UTF-8
#   constant         NaN, Infinity, or -Infinity
#   number_overflow  a number literal outside the range of an IEEE 754 double
#   integer_digits   an integer literal longer than the integer string conversion limit
#   surrogate        a string with an unpaired UTF-16 surrogate escape
#   depth            nesting deeper than the fallback parser's recursion limit
PARSE_FAILURE_KINDS: Final[tuple[ParseFailureKind, ...]] = (
    "syntax",
    "encoding",
    "constant",
    "number_overflow",
    "integer_digits",
    "surrogate",
    "depth",
)

UTF8_BOM: Final = b"\xef\xbb\xbf"

# orjson's integers are exact in [-2**63, 2**64 - 1]; a float at or beyond 2**63
# in magnitude that is integral may be a rounded integer literal.
_LOSSY_FLOAT_MIN: Final = float(2**63)

# A necessary condition for such a float: a literal with at least 19 integer
# digits (2**63 has 19), or one with an exponent. Checked on the raw bytes so
# most documents skip the walk over the parsed value.
_MAY_HOLD_LOSSY_FLOAT: Final = re.compile(rb"[0-9]{19}|[eE][-+]?[0-9]")

# A necessary condition for an unpaired surrogate: an escape in \uD800-\uDFFF.
_SURROGATE_ESCAPE: Final = re.compile(r"\\u[dD][89a-fA-F]")

# One escape sequence. In a document the stdlib decoder accepted, every
# backslash starts an escape inside a string, so successive matches walk the
# escapes in order.
_ESCAPE: Final = re.compile(r"\\(?:u([0-9a-fA-F]{4})|.)", re.DOTALL)

# Literals a decoder hook can reject. Strings match whole, so literals inside
# strings are skipped.
_TOKEN: Final = re.compile(
    r'"[^"\\]*(?:\\.[^"\\]*)*"'
    r"|-?Infinity|NaN"
    r"|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?",
    re.DOTALL,
)

_LITERAL_PREVIEW: Final = 40


class DocumentParseError(ValueError):
    """A document that failed to parse with both parsers (Requirements 1.7, 1.11).

    ``kind`` is one of :data:`PARSE_FAILURE_KINDS`; ``reason`` describes the
    error without its position; ``line`` and ``column`` are 1-based within
    the parsed bytes, or ``None`` when unknown (see the module docstring).
    """

    def __init__(
        self,
        kind: ParseFailureKind,
        reason: str,
        line: int | None = None,
        column: int | None = None,
    ) -> None:
        if kind not in PARSE_FAILURE_KINDS:
            raise ValueError(f"DocumentParseError kind must be one of {list(PARSE_FAILURE_KINDS)}, got {kind!r}")
        if not isinstance(reason, str) or not reason:
            raise ValueError("DocumentParseError reason must be a non-empty str")
        for name, number in (("line", line), ("column", column)):
            if number is not None and (type(number) is not int or number < 1):
                raise ValueError(f"DocumentParseError {name} must be None or an int >= 1, got {number!r}")
        super().__init__(kind, reason, line, column)  # args survive pickling
        self.kind: ParseFailureKind = kind
        self.reason: str = reason
        self.line: int | None = line
        self.column: int | None = column

    def __str__(self) -> str:
        if self.line is None:
            return self.reason
        if self.column is None:
            return f"{self.reason} (line {self.line})"
        return f"{self.reason} (line {self.line}, column {self.column})"


class _Rejected(Exception):
    """Raised by a stdlib decoder hook for a literal that the Ingestor does not accept."""

    def __init__(self, kind: ParseFailureKind, literal: str, reason: str) -> None:
        super().__init__(kind, literal, reason)
        self.kind: ParseFailureKind = kind
        self.literal = literal
        self.reason = reason


def _preview(literal: str) -> str:
    if len(literal) <= _LITERAL_PREVIEW:
        return literal
    return f"{literal[: _LITERAL_PREVIEW - 3]}..."


def _reject_constant(name: str) -> None:
    raise _Rejected("constant", name, f"{name} is not a JSON value under RFC 8259")


def _parse_float(literal: str) -> float:
    value = float(literal)
    if math.isinf(value):
        raise _Rejected(
            "number_overflow", literal, f"number {_preview(literal)} is outside the range of an IEEE 754 double"
        )
    return value


def _parse_int(literal: str) -> int:
    try:
        return int(literal)
    except ValueError:
        digits = len(literal) - literal.startswith("-")
        limit = sys.get_int_max_str_digits()
        raise _Rejected(
            "integer_digits",
            literal,
            f"integer literal has {digits} digits, more than the integer string conversion limit of {limit}",
        ) from None


_STDLIB_DECODER: Final = json.JSONDecoder(
    parse_float=_parse_float,
    parse_int=_parse_int,
    parse_constant=_reject_constant,
    strict=True,
)


def parse_document(data: bytes) -> tuple[JSON, Parser]:
    """Parse one JSON document and name the parser that produced the value.

    Strip one UTF-8 BOM. Try orjson. Fall back to stdlib :mod:`json` when
    orjson raises, or when the result contains an integral float with
    ``|x| >= 2**63`` (a possibly lossy big integer). The stdlib path decodes
    strict UTF-8 and uses ``parse_constant`` to reject NaN and ±Infinity.

    Raises :class:`DocumentParseError` when both parsers fail, including for
    every construct of Requirement 1.11, number literals that overflow a
    double, and unpaired surrogate escapes. Raises ``TypeError`` when
    ``data`` is not ``bytes``.
    """
    if type(data) is not bytes:
        raise TypeError(f"parse_document expects bytes, got {type(data).__name__}")
    offset = 0
    if data.startswith(UTF8_BOM):
        data = data[len(UTF8_BOM) :]
        offset = len(UTF8_BOM)
    try:
        value = orjson.loads(data)
    except orjson.JSONDecodeError as exc:
        orjson_error: orjson.JSONDecodeError | None = exc
    else:
        if _MAY_HOLD_LOSSY_FLOAT.search(data) is None or not _has_lossy_float(value):
            return value, "orjson"
        orjson_error = None
    return _parse_stdlib(data, offset, orjson_error), "stdlib"


def _has_lossy_float(value: JSON) -> bool:
    """Return whether ``value`` holds an integral float with magnitude at least 2**63."""
    pending: list[Any] = [value]  # an explicit stack, so deep nesting cannot overflow
    while pending:
        item = pending.pop()
        cls = type(item)
        if cls is float:
            if item.is_integer() and abs(item) >= _LOSSY_FLOAT_MIN:
                return True
        elif cls is list:
            pending.extend(item)
        elif cls is dict:
            pending.extend(item.values())
    return False


def _parse_stdlib(data: bytes, offset: int, orjson_error: orjson.JSONDecodeError | None) -> JSON:
    """Parse with the stdlib decoder; ``offset`` is the length of the stripped BOM."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        line = data.count(b"\n", 0, exc.start) + 1
        raise DocumentParseError(
            "encoding", f"invalid UTF-8 at byte offset {exc.start + offset}: {exc.reason}", line
        ) from None
    if text.startswith("\ufeff"):
        raise DocumentParseError("syntax", "invalid JSON: more than one leading byte order mark", 1, 1)
    try:
        value = _STDLIB_DECODER.decode(text)
    except json.JSONDecodeError as exc:
        # "Invalid control character at" expects a position after it.
        message = exc.msg.removesuffix(" at")
        raise DocumentParseError("syntax", f"invalid JSON: {message}", exc.lineno, exc.colno) from None
    except _Rejected as exc:
        line, column = _line_column(text, _locate_literal(text, exc.literal))
        raise DocumentParseError(exc.kind, exc.reason, line, column) from None
    except RecursionError:
        line = column = None
        if orjson_error is not None:
            line, column = orjson_error.lineno, orjson_error.colno
        raise DocumentParseError(
            "depth", "nesting is deeper than the recursion limit of the fallback parser", line, column
        ) from None
    if _SURROGATE_ESCAPE.search(text) is not None:
        position = _unpaired_surrogate(text)
        if position is not None:
            escape = text[position : position + 6]
            line, column = _line_column(text, position)
            raise DocumentParseError("surrogate", f"string has an unpaired UTF-16 surrogate escape {escape}", line, column)
    return value


def _line_column(text: str, position: int | None) -> tuple[int | None, int | None]:
    """1-based line and column of ``position``, counted as :class:`json.JSONDecodeError` does."""
    if position is None:
        return None, None
    return text.count("\n", 0, position) + 1, position - text.rfind("\n", 0, position)


def _locate_literal(text: str, literal: str) -> int | None:
    """Return the offset of the first token outside a string that equals ``literal``."""
    for match in _TOKEN.finditer(text):
        if match.group() == literal:
            return match.start()
    return None


def _unpaired_surrogate(text: str) -> int | None:
    """Return the offset of the first unpaired surrogate escape, or ``None``.

    A high-surrogate escape (``\\uD800``-``\\uDBFF``) pairs with a
    low-surrogate escape (``\\uDC00``-``\\uDFFF``) that follows it directly,
    as in the stdlib decoder. Every other surrogate escape is unpaired.
    """
    high_start = high_end = -1  # the pending high-surrogate escape, if any
    for match in _ESCAPE.finditer(text):
        hexits = match.group(1)
        unit = int(hexits, 16) if hexits is not None else -1
        if high_start >= 0:
            if match.start() == high_end and 0xDC00 <= unit <= 0xDFFF:
                high_start = -1
                continue
            return high_start
        if 0xD800 <= unit <= 0xDBFF:
            high_start, high_end = match.start(), match.end()
        elif 0xDC00 <= unit <= 0xDFFF:
            return match.start()
    return high_start if high_start >= 0 else None


# ===========================================================================
# Enumeration, records, parallel parsing, and the DocumentStore (task 4.2)
# ===========================================================================

# The Resource_Parameter that sets the number of records per parsing chunk.
CHUNK_SIZE: Final = "ingest.chunk_size"

# Supported extensions (after casefold()), and those read as newline-delimited JSON (1.1, 1.2).
SUPPORTED_EXTENSIONS: Final[tuple[str, ...]] = (".json", ".jsonl", ".ndjson")
LINE_EXTENSIONS: Final[tuple[str, ...]] = (".jsonl", ".ndjson")

# The deepest container nesting a record may have (see the module docstring, Records).
MAX_NESTING_DEPTH: Final = 512

_EXPECTED_EXTENSIONS: Final = ".json, .jsonl, or .ndjson"

# A string (matched whole, so brackets inside it are skipped) or one bracket.
_STRUCTURE: Final = re.compile(rb'"[^"\\]*(?:\\.[^"\\]*)*"|[\[\]{}]', re.DOTALL)
_OPENERS: Final = (ord("["), ord("{"))
_CLOSERS: Final = (ord("]"), ord("}"))


@dataclass(frozen=True, slots=True)
class DocOrigin:
    """Where a document came from (1.5).

    ``source`` is the file's path relative to the supplied directory (with
    ``/`` separators), or the supplied file path. ``line`` is the 1-based
    line of a ``.jsonl`` or ``.ndjson`` document, else None. ``element`` is
    the array position of a document produced by array unwrapping, else None.
    """

    source: str
    line: int | None = None
    element: int | None = None


class DocumentStore:
    """The canonical bytes of every document, concatenated in index order, plus offsets.

    Document ``i`` is ``buffer[offsets[i]:offsets[i + 1]]``, the canonical
    encoding (``core.determinism.encode_canonical``) of its value.
    ``offsets`` is a read-only int64 array of ``len(store) + 1``
    non-decreasing entries from 0 to ``len(buffer)``. Stores are equal when
    their buffers and offsets are equal. Stores pickle.
    """

    __slots__ = ("_buffer", "_offsets")

    def __init__(self, buffer: bytes, offsets: Any) -> None:
        if type(buffer) is not bytes:
            raise TypeError(f"DocumentStore buffer must be bytes, got {type(buffer).__name__}")
        array = np.array(offsets)  # a copy: later changes to the argument cannot reach the store
        if array.ndim != 1 or len(array) == 0 or not np.issubdtype(array.dtype, np.integer):
            raise ValueError("DocumentStore offsets must be a 1-D integer array with at least one entry")
        array = array.astype(np.int64, copy=False)
        if array[0] != 0 or array[-1] != len(buffer) or bool(np.any(np.diff(array) < 0)):
            raise ValueError("DocumentStore offsets must be non-decreasing from 0 to len(buffer)")
        array.flags.writeable = False
        self._buffer = buffer
        self._offsets = array

    @property
    def buffer(self) -> bytes:
        """The concatenated canonical bytes of all documents."""
        return self._buffer

    @property
    def offsets(self) -> np.ndarray:
        """The read-only int64 offsets: document ``i`` spans ``offsets[i]`` to ``offsets[i + 1]``."""
        return self._offsets

    @property
    def nbytes(self) -> int:
        """The size of the buffer in bytes."""
        return len(self._buffer)

    def __len__(self) -> int:
        return len(self._offsets) - 1

    def _span(self, index: int) -> tuple[int, int]:
        if isinstance(index, (bool, np.bool_)) or not isinstance(index, (int, np.integer)):
            raise TypeError(f"document index must be an int, got {type(index).__name__}")
        count = len(self)
        position = int(index) + count if index < 0 else int(index)
        if not 0 <= position < count:
            raise IndexError(f"document index {int(index)} is out of range for {count} documents")
        return int(self._offsets[position]), int(self._offsets[position + 1])

    def __getitem__(self, index: int) -> bytes:
        """The canonical bytes of document ``index`` (negative indices count from the end)."""
        start, stop = self._span(index)
        return self._buffer[start:stop]

    def __iter__(self) -> Iterator[bytes]:
        offsets = self._offsets.tolist()
        for start, stop in zip(offsets, offsets[1:]):
            yield self._buffer[start:stop]

    def value(self, index: int) -> JSON:
        """The value of document ``index``, parsed from its canonical bytes."""
        return parse_document(self[index])[0]

    def values(self, start: int = 0, stop: int | None = None) -> Iterator[JSON]:
        """The values of the documents in ``range(start, stop)`` (``stop`` defaults to ``len(store)``)."""
        for index in range(*slice(start, stop).indices(len(self))):
            yield self.value(index)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, DocumentStore):
            return NotImplemented
        return self._buffer == other._buffer and bool(np.array_equal(self._offsets, other._offsets))

    __hash__ = None  # type: ignore[assignment]

    def __reduce__(self) -> tuple[Any, tuple[Any, ...]]:
        return (type(self), (self._buffer, np.asarray(self._offsets)))

    def __repr__(self) -> str:
        return f"DocumentStore({len(self)} documents, {self.nbytes} bytes)"


@dataclass(frozen=True, slots=True)
class IngestResult:
    """The outcome of :meth:`Ingestor.ingest`.

    ``origins[i]`` is the origin of document ``i`` of ``store``. ``errors``
    holds the skipped documents and files in ingestion order, and
    ``skipped`` is their count (1.7).
    """

    store: DocumentStore
    origins: tuple[DocOrigin, ...]
    errors: tuple[IngestError, ...]
    skipped: int

    def __post_init__(self) -> None:
        if len(self.origins) != len(self.store):
            raise ValueError(f"IngestResult has {len(self.origins)} origins for {len(self.store)} documents")
        if self.skipped != len(self.errors):
            raise ValueError(f"IngestResult skipped ({self.skipped}) must equal the number of errors ({len(self.errors)})")


@dataclass(frozen=True, slots=True)
class _Input:
    """An accepted supplied path: as supplied (to open) and as reported."""

    path: str
    display: str
    is_dir: bool


@dataclass(frozen=True, slots=True)
class _Record:
    """One document to parse: its source, starting line (None for a ``.json`` file), bytes, and unwrapping."""

    source: str
    line: int | None
    data: bytes
    unwrap: bool


# What the main process keeps of a chunk: (source, line) per record, or the
# IngestError of an unreadable file or directory, in ingestion order.
_Meta: TypeAlias = tuple[str, int | None] | IngestError

# A record's outcome in a worker: None for one document, the element count of
# an unwrapped array, or the parse error.
_Outcome: TypeAlias = int | None | DocumentParseError

# A parsed chunk: the concatenated canonical bytes of its documents, the size
# of each document, and one outcome per record.
_ChunkResult: TypeAlias = tuple[bytes, np.ndarray, list[_Outcome]]


def _display(text: str) -> str:
    """``text`` as valid Unicode: unpaired surrogates (undecodable file name bytes) become backslash escapes."""
    return text.encode("utf-8", "backslashreplace").decode("utf-8")


def _extension(name: str) -> str:
    """The extension of ``name`` as :func:`os.path.splitext` finds it, after ``casefold()``."""
    return os.path.splitext(name)[1].casefold()


def _os_reason(error: OSError) -> str:
    """The OS's description of ``error``, without the file name (which may be absolute)."""
    return _display(error.strerror or str(error) or type(error).__name__)


def _check_input(path: str) -> tuple[bool, str | None]:
    """Whether ``path`` is a directory, and why it is rejected (None when it is accepted; 1.10)."""
    try:
        mode = os.stat(path).st_mode
    except (FileNotFoundError, NotADirectoryError):
        return False, "does not exist"
    except OSError as error:
        return False, f"cannot be read: {_os_reason(error)}"
    except ValueError:  # an embedded NUL character
        return False, "is not a valid path"
    if stat.S_ISDIR(mode):
        try:
            with os.scandir(path):
                pass
        except OSError as error:
            return True, f"is a directory that cannot be read: {_os_reason(error)}"
        return True, None
    if not stat.S_ISREG(mode):
        return False, "is neither a regular file nor a directory"
    if _extension(path) not in SUPPORTED_EXTENSIONS:
        suffix = os.path.splitext(path)[1]
        found = f"has the extension {_display(suffix)!r}" if suffix else "has no extension"
        return False, f"{found}; expected {_EXPECTED_EXTENSIONS}"
    try:
        with open(path, "rb"):
            pass
    except OSError as error:
        return False, f"cannot be read: {_os_reason(error)}"
    return False, None


def _check_inputs(inputs: Sequence[str | os.PathLike[str]]) -> list[_Input]:
    """The accepted inputs in argument order; a ConfigError names every rejected path (1.10)."""
    if isinstance(inputs, (str, bytes, os.PathLike)):
        raise TypeError("inputs must be a sequence of paths, not a single path")
    accepted: list[_Input] = []
    problems: list[ConfigProblem] = []
    for item in inputs:
        path = os.fspath(item)
        if not isinstance(path, str):
            raise TypeError(f"input paths must be str or os.PathLike[str], got {type(item).__name__}")
        display = _display(path)
        is_dir, reason = _check_input(path)
        if reason is None:
            accepted.append(_Input(path, display, is_dir))
        else:
            problems.append(ConfigProblem(display, "path", f"input path {display!r} {reason}"))
    if problems:
        raise ConfigError(problems)
    return accepted


def _walk(root: str) -> list[tuple[str, str, str | None]]:
    """The supported files and unreadable items under ``root``, in code-point order of their names (1.2, 1.5).

    Each entry is (name relative to ``root`` with ``/`` separators, path to
    open, reason when the item cannot be read). The root itself, when it
    cannot be listed, has the name ``""``.
    """
    found: list[tuple[str, str, str | None]] = []

    def relative(path: str) -> str:
        # os.walk joins names onto root, so every path it reports starts with root.
        return path[len(root) :].lstrip(os.sep).replace(os.sep, "/")

    def unlisted(error: OSError) -> None:
        where = error.filename if isinstance(error.filename, str) else root
        found.append((relative(where), where, f"directory cannot be read: {_os_reason(error)}"))

    for dirpath, _dirnames, filenames in os.walk(root, followlinks=False, onerror=unlisted):
        prefix = relative(dirpath)
        prefix = f"{prefix}/" if prefix else ""
        for name in filenames:
            if _extension(name) not in SUPPORTED_EXTENSIONS:
                continue
            path = os.path.join(dirpath, name)
            try:
                mode = os.lstat(path).st_mode
            except OSError as error:
                found.append((prefix + name, path, f"file cannot be read: {_os_reason(error)}"))
                continue
            if stat.S_ISREG(mode):  # symbolic links and other non-regular files are skipped
                found.append((prefix + name, path, None))
    found.sort(key=lambda entry: codepoint_key(entry[0]))
    return found


def _chunks(items: Iterable[_Record | IngestError], size: int) -> Iterator[tuple[list[_Meta], list[tuple[bytes, bool]]]]:
    """Group items into chunks of ``size`` records; unreadable items ride along in order."""
    meta: list[_Meta] = []
    payload: list[tuple[bytes, bool]] = []
    for item in items:
        if isinstance(item, IngestError):
            meta.append(item)
            continue
        meta.append((item.source, item.line))
        payload.append((item.data, item.unwrap))
        if len(payload) == size:
            yield meta, payload
            meta, payload = [], []
    if meta:
        yield meta, payload


def _check_nesting(data: bytes) -> None:
    """Raise a ``depth`` DocumentParseError when ``data`` nests more than MAX_NESTING_DEPTH containers."""
    if data.count(b"[") + data.count(b"{") <= MAX_NESTING_DEPTH:
        return  # too few brackets to nest that deep
    depth = 0
    for match in _STRUCTURE.finditer(data):
        byte = data[match.start()]
        if byte in _OPENERS:
            depth += 1
            if depth > MAX_NESTING_DEPTH:
                line = data.count(b"\n", 0, match.start()) + 1
                raise DocumentParseError(
                    "depth", f"nesting is deeper than the Ingestor's limit of {MAX_NESTING_DEPTH} levels", line
                )
        elif byte in _CLOSERS and depth:
            depth -= 1


def _encode(value: JSON) -> bytes:
    try:
        return encode_canonical(value)
    except RecursionError:
        # Below MAX_NESTING_DEPTH this needs a nearly full call stack; fail the document, not the run.
        raise DocumentParseError("depth", "nesting is too deep for the canonical encoder") from None


def _parse_chunk(records: Sequence[tuple[bytes, bool]]) -> _ChunkResult:
    """Parse and canonically re-encode one chunk of ``(bytes, unwrap)`` records (runs in a joblib worker)."""
    parts: list[bytes] = []
    outcomes: list[_Outcome] = []
    for data, unwrap in records:
        outcome: _Outcome
        try:
            _check_nesting(data)
            value, _parser = parse_document(data)
            if unwrap and type(value) is list:
                encoded = [_encode(element) for element in value]
                outcome = len(encoded)
            else:
                encoded = [_encode(value)]
                outcome = None
        except DocumentParseError as error:
            outcomes.append(error)
            continue
        parts.extend(encoded)
        outcomes.append(outcome)
    sizes = np.fromiter((len(part) for part in parts), dtype=np.int64, count=len(parts))
    return b"".join(parts), sizes, outcomes


def _parse_failure(source: str, line: int | None, error: DocumentParseError) -> IngestError:
    """The IngestError of a record starting at ``line`` (None: a ``.json`` file) that failed to parse (1.7)."""
    start = 1 if line is None else line
    at = start if error.line is None else start + error.line - 1
    reason = error.reason if error.column is None else f"{error.reason} (column {error.column})"
    return IngestError(source, _display(reason), at)


def _build_store(buffers: list[bytes], sizes: list[np.ndarray]) -> DocumentStore:
    lengths = np.concatenate(sizes) if sizes else np.empty(0, dtype=np.int64)
    offsets = np.zeros(len(lengths) + 1, dtype=np.int64)
    np.cumsum(lengths, out=offsets[1:])
    return DocumentStore(b"".join(buffers), offsets)


class Ingestor:
    """Reads the supplied paths into a :class:`DocumentStore` (Requirement 1; see the module docstring).

    ``opts`` sets array unwrapping. ``workers`` (the ``workers``
    Resource_Parameter) bounds the joblib worker processes, and
    ``chunk_size`` (``ingest.chunk_size``) is the number of records per
    chunk. Neither affects the result. Invalid values raise ``ValueError``
    naming the setting.
    """

    __slots__ = ("_chunk_size", "_opts", "_workers")

    def __init__(self, opts: InputOptions, workers: int, chunk_size: int) -> None:
        if not isinstance(opts, InputOptions):
            raise TypeError(f"opts must be an InputOptions, got {type(opts).__name__}")
        self._opts = opts
        self._workers = int(RESOURCE_PARAMETERS[WORKERS].normalize(workers))
        self._chunk_size = int(RESOURCE_PARAMETERS[CHUNK_SIZE].normalize(chunk_size))

    @property
    def opts(self) -> InputOptions:
        return self._opts

    @property
    def workers(self) -> int:
        return self._workers

    @property
    def chunk_size(self) -> int:
        return self._chunk_size

    def ingest(self, inputs: Sequence[str | os.PathLike[str]]) -> IngestResult:
        """Ingest ``inputs`` in argument order.

        Raises :class:`~jsinfer.config.errors.ConfigError` naming every
        rejected supplied path before anything is read (1.10), and
        :class:`~jsinfer.core.errors.NoDocumentsError` when no document
        parses (1.8).
        """
        accepted = _check_inputs(inputs)
        buffers: list[bytes] = []
        sizes: list[np.ndarray] = []
        origins: list[DocOrigin] = []
        errors: list[IngestError] = []
        for meta, (data, chunk_sizes, outcomes) in self._parse(_chunks(self._items(accepted), self._chunk_size)):
            buffers.append(data)
            sizes.append(chunk_sizes)
            results = iter(outcomes)
            for entry in meta:
                if isinstance(entry, IngestError):
                    errors.append(entry)
                    continue
                source, line = entry
                outcome = next(results)
                if outcome is None:
                    origins.append(DocOrigin(source, line))
                elif isinstance(outcome, DocumentParseError):
                    errors.append(_parse_failure(source, line, outcome))
                else:
                    origins.extend(DocOrigin(source, line, element) for element in range(outcome))
        if not origins:
            raise NoDocumentsError(errors)
        return IngestResult(_build_store(buffers, sizes), tuple(origins), tuple(errors), len(errors))

    def _items(self, accepted: list[_Input]) -> Iterator[_Record | IngestError]:
        """Records and unreadable items of every input, in ingestion order."""
        for supplied in accepted:
            if not supplied.is_dir:
                yield from self._read(supplied.path, supplied.display)
                continue
            for name, path, unreadable in _walk(supplied.path):
                source = _display(name) if name else supplied.display
                if unreadable is not None:
                    yield IngestError(source, unreadable)
                else:
                    yield from self._read(path, source)

    def _read(self, path: str, source: str) -> Iterator[_Record | IngestError]:
        """The records of one file (1.1, 1.9), or its IngestError when it cannot be read (1.7)."""
        try:
            with open(path, "rb") as file:
                data = file.read()
        except OSError as error:
            yield IngestError(source, f"file cannot be read: {_os_reason(error)}")
            return
        if _extension(path) not in LINE_EXTENSIONS:
            yield _Record(source, None, data, self._opts.unwrap_arrays)
            return
        for number, line in enumerate(data.split(b"\n"), start=1):
            if line.endswith(b"\r"):
                line = line[:-1]
            if line.strip(b" \t"):  # blank lines consume no index
                yield _Record(source, number, line, False)

    def _parse(
        self, chunks: Iterator[tuple[list[_Meta], list[tuple[bytes, bool]]]]
    ) -> Iterator[tuple[list[_Meta], _ChunkResult]]:
        """Parse chunks with joblib in at most ``workers`` processes; yield them in submission order (29.4)."""
        # joblib may draw tasks from another thread. Each chunk's metadata is
        # queued before its task is handed out, and results arrive in
        # submission order, so the oldest queued metadata belongs to each result.
        pending: deque[list[_Meta]] = deque()

        def tasks() -> Iterator[Any]:
            for meta, payload in chunks:
                pending.append(meta)
                yield delayed(_parse_chunk)(payload)

        results = Parallel(n_jobs=self._workers, return_as="generator")(tasks())
        try:
            for result in results:
                yield pending.popleft(), result
        finally:
            results.close()
