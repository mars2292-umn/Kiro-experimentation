"""Property 1: Ingestion matches the reference model (task 4.3).

For any generated input tree of ``.json``, ``.jsonl``, ``.ndjson``, and other
files (mixed-case extensions, dotfiles, symbolic links, FIFOs, unreadable
files and directories, nested directories, LF and CRLF, blank and
whitespace-only lines, invalid documents and constructs that RFC 8259
forbids), supplied as any mix of directories and files, with array unwrapping
on or off, the Ingestor's documents, indices, skipped entries (path, line,
reason), and skipped count equal those of a straightforward reference
enumerator, for every worker count and chunk size.

The reference below is written independently of io/ingest.py. It lists
directories recursively with ``os.scandir`` and ``os.lstat``, splits lines
with ``io.BytesIO.readlines``, scans nesting depth byte by byte, decides
validity with the stdlib :mod:`json` decoder plus the documented policy (no
overflowing doubles, no unpaired surrogates), and unwraps arrays itself. Only
the reason and in-record position of a parse error come from
``parse_document``, because reproducing its messages would mean
reimplementing it; the reference still decides on its own that the record
fails, and the test fails if ``parse_document`` accepts it.

**Validates: Requirements 1.1, 1.2, 1.3, 1.5, 1.7, 1.9, 1.11**
"""

# Feature: json-schema-inference, Property 1: Ingestion matches the reference model

from __future__ import annotations

import io
import json
import math
import os
import re
import stat
import tempfile
import unicodedata
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.run_config import InputOptions
from jsinfer.core.determinism import encode_canonical
from jsinfer.core.errors import NoDocumentsError
from jsinfer.core.report import IngestError
from jsinfer.io.ingest import MAX_NESTING_DEPTH, UTF8_BOM, DocumentParseError, Ingestor, parse_document
from tests.strategies import forbidden_json_texts, json_texts, text_values

POSIX_PERMISSIONS = hasattr(os, "geteuid") and os.geteuid() != 0  # root can read anything
FIFOS = hasattr(os, "mkfifo")

# ===========================================================================
# The reference enumerator
# ===========================================================================

JSON_EXTENSION = ".json"
LINE_EXTENSIONS = (".jsonl", ".ndjson")
DEPTH_REASON = f"nesting is deeper than the Ingestor's limit of {MAX_NESTING_DEPTH} levels"
_SURROGATE = re.compile("[\ud800-\udfff]")

# A document: ((source, line, element), canonical bytes).
Doc = tuple[tuple[str, int | None, int | None], bytes]


@dataclass(frozen=True)
class Outcome:
    """What ingestion produced: documents in index order, skipped entries in order, and the skipped count."""

    docs: list[Doc]
    errors: list[IngestError]
    skipped: int


def extension(name: str) -> str:
    """The case-folded extension of a file name: from its last dot, not counting leading dots."""
    stem = os.path.basename(name).casefold().lstrip(".")
    dot = stem.rfind(".")
    return stem[dot:] if dot >= 0 else ""


def listing(root: str) -> list[tuple[str, str, str | None]]:
    """``(name, path, reason)`` of each supported regular file and each unlistable directory under ``root``.

    Names are relative to ``root`` with ``/`` separators; the list is sorted
    by name (Python compares ``str`` by code point). ``reason`` is set for a
    directory that cannot be listed. Symbolic links are never followed.
    """
    found: list[tuple[str, str, str | None]] = []

    def visit(directory: str, prefix: str) -> None:
        try:
            with os.scandir(directory) as entries:
                children = [(entry.name, entry.path) for entry in entries]
        except OSError as error:
            found.append((prefix.removesuffix("/"), directory, f"directory cannot be read: {error.strerror}"))
            return
        for name, path in children:
            mode = os.lstat(path).st_mode
            if stat.S_ISDIR(mode):
                visit(path, f"{prefix}{name}/")
            elif stat.S_ISREG(mode) and extension(name) in (JSON_EXTENSION, *LINE_EXTENSIONS):
                found.append((prefix + name, path, None))
            # Symbolic links, FIFOs, and other extensions are not ingested.

    visit(root, "")
    return sorted(found, key=lambda entry: entry[0])


def too_deep_line(record: bytes) -> int | None:
    """The 1-based line of the first container opened deeper than MAX_NESTING_DEPTH, outside strings."""
    if record.count(b"[") + record.count(b"{") <= MAX_NESTING_DEPTH:
        return None  # nesting cannot exceed the number of openers
    depth, line, in_string, escaped = 0, 1, False, False
    for byte in record:
        if byte == ord("\n"):
            line += 1
        if in_string:
            if escaped:
                escaped = False
            elif byte == ord("\\"):
                escaped = True
            elif byte == ord('"'):
                in_string = False
        elif byte == ord('"'):
            in_string = True
        elif byte in b"[{":
            depth += 1
            if depth > MAX_NESTING_DEPTH:
                return line
        elif byte in b"]}" and depth:
            depth -= 1
    return None


def _reject_constant(name: str) -> None:
    raise ValueError(f"{name} is not RFC 8259 JSON")


def _items(value: Any) -> Iterator[Any]:
    """Every value and object key inside ``value`` (iteratively, so deep nesting cannot overflow)."""
    pending = [value]
    while pending:
        item = pending.pop()
        yield item
        if isinstance(item, list):
            pending.extend(item)
        elif isinstance(item, dict):
            pending.extend(item)
            pending.extend(item.values())


def accepted_value(record: bytes) -> tuple[bool, Any]:
    """Whether ``record`` is an accepted document, and its value, decided with the stdlib decoder alone.

    One BOM is stripped; the rest must be strict UTF-8 and RFC 8259 JSON
    without NaN or infinities. The policy also rejects number literals that
    overflow a double and strings with an unpaired surrogate escape (which the
    stdlib decoder turns into lone surrogates).
    """
    body = record[len(UTF8_BOM) :] if record.startswith(UTF8_BOM) else record
    try:
        value = json.loads(body.decode("utf-8"), parse_constant=_reject_constant)
    except ValueError:  # invalid UTF-8, a syntax error, a constant, or an over-long integer
        return False, None
    for item in _items(value):
        if type(item) is float and math.isinf(item):
            return False, None
        if type(item) is str and _SURROGATE.search(item):
            return False, None
    return True, value


def parse_failure(source: str, start: int, record: bytes) -> IngestError:
    """The skipped entry of a record that starts at line ``start`` and that the reference rejects."""
    try:
        parse_document(record)
    except DocumentParseError as error:
        line = start if error.line is None else start + error.line - 1
        reason = error.reason if error.column is None else f"{error.reason} (column {error.column})"
        return IngestError(source, reason, line)
    raise AssertionError(f"the reference rejects record {record!r} of {source}, but parse_document accepts it")


def reference_ingest(inputs: Sequence[str | os.PathLike[str]], unwrap: bool) -> Outcome:
    """Ingest ``inputs`` in argument order, the straightforward way."""
    docs: list[Doc] = []
    errors: list[IngestError] = []

    def add_record(source: str, line: int | None, record: bytes, unwrap_here: bool) -> None:
        start = 1 if line is None else line
        deep = too_deep_line(record)
        if deep is not None:
            errors.append(IngestError(source, DEPTH_REASON, start + deep - 1))
            return
        ok, value = accepted_value(record)
        if not ok:
            errors.append(parse_failure(source, start, record))
        elif unwrap_here and isinstance(value, list):
            docs.extend(((source, line, position), encode_canonical(item)) for position, item in enumerate(value))
        else:
            docs.append(((source, line, None), encode_canonical(value)))

    def add_file(path: str, source: str) -> None:
        try:
            with open(path, "rb") as file:
                data = file.read()
        except OSError as error:
            errors.append(IngestError(source, f"file cannot be read: {error.strerror}"))
            return
        if extension(path) not in LINE_EXTENSIONS:
            add_record(source, None, data, unwrap)
            return
        for number, raw in enumerate(io.BytesIO(data).readlines(), start=1):
            text = raw.removesuffix(b"\n").removesuffix(b"\r")
            if any(byte not in b" \t" for byte in text):
                add_record(source, number, text, False)

    for item in inputs:
        supplied = os.fspath(item)
        if os.path.isdir(supplied):
            for name, path, reason in listing(supplied):
                if reason is None:
                    add_file(path, name)
                else:
                    errors.append(IngestError(name, reason))
        else:
            add_file(supplied, supplied)
    return Outcome(docs, errors, len(errors))


def observe(ingestor: Ingestor, inputs: Sequence[str | os.PathLike[str]]) -> Outcome:
    """The Ingestor's result in the reference's terms (no documents: the error's skipped entries)."""
    try:
        result = ingestor.ingest(inputs)
    except NoDocumentsError as error:
        return Outcome([], list(error.errors), error.skipped)
    docs = [((origin.source, origin.line, origin.element), result.store[index]) for index, origin in enumerate(result.origins)]
    return Outcome(docs, list(result.errors), result.skipped)


# ===========================================================================
# Input trees
# ===========================================================================

JSON_SPELLINGS = (".json", ".JSON", ".Json")
LINE_SPELLINGS = (".jsonl", ".JSONL", ".JsonL", ".ndjson", ".NDJSON", ".nDjSoN")
OTHER_SPELLINGS = ("", ".txt", ".json.bak", ".jsonx", ".JSON~", ".yaml")
# "" and "." stems give ".json" and "..json", which have no extension.
STEMS = ("a", "B", "c", "\u00e9", "a-b", "x y", "_", ".hidden", "", ".")
DIR_NAMES = ("d", "E", ".dot", "sub dir", "\u00fc", "x.json", "y.JSONL")
# Lines that are blank (empty, spaces, tabs), and a bare CR, which is blank only when a CR LF does not follow.
BLANK_LINES = (b"", b" ", b"\t", b" \t ", b"\r")
WORKER_COUNTS = (1, 2)
CHUNK_SIZES = (1, 2, 3, 7, 2000)


@dataclass(frozen=True)
class Node:
    """A non-directory entry of the tree.

    ``kind`` is ``file``, ``locked`` (a file nobody may read), ``fifo``, or
    ``link``. A link's ``target`` is ``file:<path>`` or ``dir:<path>`` (in the
    tree), ``outside-file``, ``outside-dir``, or ``missing``.
    """

    path: str
    kind: str
    data: bytes = b""
    target: str = ""


@dataclass(frozen=True)
class Tree:
    """Directories (parents first), locked directories, other entries, and the supplied inputs.

    Paths are relative to the tree root with ``/`` separators; ``""`` is the
    root. An input is ``(kind, path, flag)``: a ``dir`` is supplied with a
    trailing ``/`` when ``flag`` is set, a ``file`` as a ``pathlib.Path``.
    """

    dirs: tuple[str, ...]
    locked_dirs: tuple[str, ...]
    nodes: tuple[Node, ...]
    inputs: tuple[tuple[str, str, bool], ...]


def fold(path: str) -> str:
    """A key shared by names that a case- and normalization-insensitive file system (macOS) treats as one."""
    return unicodedata.normalize("NFD", unicodedata.normalize("NFD", path).casefold())


def join(parent: str, name: str) -> str:
    return f"{parent}/{name}" if parent else name


@st.composite
def deep_texts(draw: st.DrawFn, multiline: bool) -> bytes:
    """Texts nested MAX_NESTING_DEPTH or one more levels deep, or with that many brackets inside a string."""
    depth = draw(st.sampled_from([MAX_NESTING_DEPTH, MAX_NESTING_DEPTH + 1]))
    form = draw(st.sampled_from(["arrays", "objects", "mixed", "in_string", "unclosed"]))
    if form == "in_string":
        # One level deep: the brackets sit in a string after an escaped quote.
        return b'["\\"' + b"[{" * depth + b'", 0]'
    opens = [b'{"k":' if form == "objects" or (form == "mixed" and level % 2) else b"[" for level in range(depth)]
    closes = [b"}" if opener.startswith(b"{") else b"]" for opener in reversed(opens)]
    # Line breaks between openers put the first one too deep on the line of its depth.
    separator = b"\n" if multiline and draw(st.booleans()) else b""
    prefix = b"\n" * draw(st.integers(0, 2)) if multiline else b""
    text = prefix + separator.join(opens)
    if form != "unclosed":
        text += (b"0" if opens[-1].startswith(b"{") else b"") + b"".join(closes)
    return UTF8_BOM + text if draw(st.booleans()) else text


def json_file_contents() -> st.SearchStrategy[bytes]:
    return st.one_of(
        json_texts(),
        # Top-level arrays, for unwrapping: empty or not.
        json_texts(st.lists(text_values(), max_size=4)),
        json_texts(st.lists(text_values(), min_size=1, max_size=4)),
        forbidden_json_texts(),
        deep_texts(multiline=True),
    )


@st.composite
def line_contents(draw: st.DrawFn) -> bytes:
    kind = draw(st.sampled_from(["valid", "valid", "blank", "forbidden", "deep"]))
    if kind == "valid":
        return draw(json_texts(multiline=False))
    if kind == "blank":
        return draw(st.sampled_from(BLANK_LINES))
    if kind == "deep":
        return draw(deep_texts(multiline=False))
    text = draw(forbidden_json_texts(multiline=False))
    return UTF8_BOM + text if draw(st.booleans()) else text  # each line may carry its own BOM


@st.composite
def line_file_contents(draw: st.DrawFn) -> bytes:
    """Lines joined by LF or CR LF, with or without a final terminator."""
    lines = draw(st.lists(line_contents(), min_size=1, max_size=6))
    parts = []
    for number, line in enumerate(lines, start=1):
        terminators = [b"\n", b"\r\n", b""] if number == len(lines) else [b"\n", b"\r\n"]
        parts.append(line + draw(st.sampled_from(terminators)))
    return b"".join(parts)


def contents(spelling: str) -> st.SearchStrategy[bytes]:
    """File contents suited to an extension spelling (contents of ignored files are arbitrary)."""
    folded = spelling.casefold()
    if folded == JSON_EXTENSION:
        return json_file_contents()
    if folded in LINE_EXTENSIONS:
        return line_file_contents()
    return st.sampled_from([b"", b"1", b"[1, 2]", b"not json"]) | st.binary(max_size=6)


def rare(draw: st.DrawFn) -> bool:
    """True one time in six; shrinks to False."""
    return draw(st.integers(0, 5)) == 5


@st.composite
def trees(draw: st.DrawFn) -> Tree:
    taken: set[str] = set()

    def claim(path: str) -> bool:
        key = fold(path)
        if key in taken:
            return False
        taken.add(key)
        return True

    dirs: list[str] = []
    for _ in range(draw(st.integers(0, 3), label="directories")):
        path = join(draw(st.sampled_from(["", *dirs])), draw(st.sampled_from(DIR_NAMES)))
        if claim(path):
            dirs.append(path)
    locked_dirs = tuple(d for d in dirs if POSIX_PERMISSIONS and rare(draw))

    # Mostly regular files; the other kinds are rarer.
    kinds = ["file"] * 6 + ["link"] + (["locked"] if POSIX_PERMISSIONS else []) + (["fifo"] if FIFOS else [])
    entries: list[tuple[str, str, bytes]] = []
    for _ in range(draw(st.integers(1, 10), label="entries")):
        parent = draw(st.sampled_from(["", *dirs]))
        family = OTHER_SPELLINGS if rare(draw) else JSON_SPELLINGS if draw(st.booleans()) else LINE_SPELLINGS
        spelling = draw(st.sampled_from(family))
        name = draw(st.sampled_from(STEMS)) + spelling
        if name in ("", ".", "..") or not claim(join(parent, name)):
            continue
        kind = draw(st.sampled_from(kinds))
        data = draw(contents(spelling)) if kind in ("file", "locked") else b""
        entries.append((join(parent, name), kind, data))

    files = [path for path, kind, _ in entries if kind in ("file", "locked")]
    nodes: list[Node] = []
    for path, kind, data in entries:
        target = ""
        if kind == "link":
            options = ["outside-file", "outside-dir", "missing", "dir"] + (["file"] if files else [])
            target = draw(st.sampled_from(options))
            if target == "file":
                target = f"file:{draw(st.sampled_from(files))}"
            elif target == "dir":
                target = f"dir:{draw(st.sampled_from(['', *dirs]))}"
        nodes.append(Node(path, kind, data, target))

    def searchable(path: str) -> bool:
        """Whether every directory enclosing ``path`` can be entered."""
        parts = path.split("/")
        return not any("/".join(parts[:end]) in locked_dirs for end in range(1, len(parts)))

    def openable_dir(path: str) -> bool:
        return path == "" or (searchable(path) and path not in locked_dirs)

    kind_of = {node.path: node.kind for node in nodes}

    def supported(path: str) -> bool:
        return extension(path) in (JSON_EXTENSION, *LINE_EXTENSIONS)

    # Inputs that the Ingestor accepts (rejected inputs are Requirement 1.10, not this property).
    candidates: list[tuple[str, str]] = [("dir", d) for d in ["", *dirs] if openable_dir(d)]
    for node in nodes:
        if not searchable(node.path):
            continue
        if node.kind == "file" and supported(node.path):
            candidates.append(("file", node.path))
        elif node.kind == "link":
            kind, _, where = node.target.partition(":")
            if kind == "outside-dir" or (kind == "dir" and openable_dir(where)):
                candidates.append(("dir", node.path))  # a supplied link is followed
            elif supported(node.path) and (
                kind == "outside-file" or (kind == "file" and kind_of[where] == "file" and searchable(where))
            ):
                candidates.append(("file", node.path))
    # Usually the whole tree, then up to two more inputs of any accepted kind.
    picked = [] if rare(draw) else [("dir", "")]
    picked += draw(st.lists(st.sampled_from(candidates), min_size=0 if picked else 1, max_size=2), label="inputs")
    inputs = tuple((kind, path, draw(st.booleans())) for kind, path in picked)
    return Tree(tuple(dirs), locked_dirs, tuple(nodes), inputs)


def build(tree: Tree, base: Path) -> tuple[Path, list[Path]]:
    """Create ``tree`` under ``base``; return its root and the paths to lock, in the order to lock them."""
    root, outside = base / "root", base / "outside"
    (outside / "dir").mkdir(parents=True)
    (outside / "out.json").write_bytes(b'"outside"')
    (outside / "dir" / "in.json").write_bytes(b'"outside dir"')
    root.mkdir()
    for directory in tree.dirs:
        (root / directory).mkdir()
    targets = {"outside-file": outside / "out.json", "outside-dir": outside / "dir", "missing": root / "missing.json"}
    for node in tree.nodes:
        path = root / node.path
        if node.kind == "fifo":
            os.mkfifo(path)
        elif node.kind == "link":
            kind, _, where = node.target.partition(":")
            path.symlink_to(targets[kind] if kind in targets else root / where)
        else:
            path.write_bytes(node.data)
    # Files first, then directories deepest first, so every path can still be reached when it is locked.
    locked = [root / node.path for node in tree.nodes if node.kind == "locked"]
    locked += [root / d for d in sorted(tree.locked_dirs, key=lambda d: d.count("/"), reverse=True)]
    return root, locked


def supplied(tree: Tree, root: Path) -> list[str | Path]:
    paths: list[str | Path] = []
    for kind, relative, flag in tree.inputs:
        path = root / relative if relative else root
        if kind == "dir":
            paths.append(f"{path}/" if flag else str(path))
        else:
            paths.append(path if flag else str(path))
    return paths


# ===========================================================================
# The property
# ===========================================================================


@given(
    tree=trees(),
    unwrap=st.booleans(),
    chunk_size=st.sampled_from(CHUNK_SIZES),
    workers=st.sampled_from(WORKER_COUNTS),
    other_chunk_size=st.sampled_from(CHUNK_SIZES),
)
def test_ingestion_matches_the_reference_model(
    tree: Tree, unwrap: bool, chunk_size: int, workers: int, other_chunk_size: int
) -> None:
    opts = InputOptions(unwrap_arrays=unwrap)
    # A fresh directory per example: Hypothesis does not reset function-scoped fixtures.
    with tempfile.TemporaryDirectory(prefix="jsinfer-p1-") as tmp:
        root, locked = build(tree, Path(tmp))
        inputs = supplied(tree, root)
        try:
            for path in locked:
                path.chmod(0)
            expected = reference_ingest(inputs, unwrap)
            actual = observe(Ingestor(opts, 1, chunk_size), inputs)
            other = observe(Ingestor(opts, workers, other_chunk_size), inputs)
        finally:
            for path in reversed(locked):
                path.chmod(0o700)

    event("unwrap" if unwrap else "no unwrap")
    event(f"workers={workers}")
    event("no documents" if not expected.docs else "documents")
    if any(origin[1] is not None for origin, _ in expected.docs):
        event("documents from lines")
    if any(origin[2] is not None for origin, _ in expected.docs):
        event("documents from unwrapped arrays")
    for kind in sorted({node.kind for node in tree.nodes}):
        event(f"entry kind: {kind}")
    if tree.locked_dirs:
        event("locked directory")
    for reason in sorted({re.sub(r"\d+|\\u\w+", "N", error.reason.split(":")[0]) for error in expected.errors}):
        event(f"skipped: {reason}")

    assert actual.docs == expected.docs
    assert actual.errors == expected.errors
    assert actual.skipped == expected.skipped
    # Neither the worker count nor the chunk size changes the result.
    assert other == actual
