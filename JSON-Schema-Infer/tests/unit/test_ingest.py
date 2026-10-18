"""Unit tests for enumeration, line splitting, and the DocumentStore in io/ingest.py (task 4.2).

Requirements 1.1 (newline-delimited files), 1.2 (directory enumeration), 1.3
(array unwrapping), 1.5 (deterministic indices), 1.7 (skipped documents and
files), 1.8 (no documents), 1.9 (whole ``.json`` files), 1.10 (rejected
supplied paths), and 29.4 (joblib workers). Property 1 (ingestion matches a
reference model) is task 4.3.
"""

from __future__ import annotations

import enum
import json
import os
import pickle
import sys
from collections import OrderedDict
from pathlib import Path
from typing import Any

import numpy as np
import pytest

from jsinfer.config.errors import ConfigError
from jsinfer.config.run_config import InputOptions
from jsinfer.core.determinism import encode_canonical
from jsinfer.core.errors import InferenceError, NoDocumentsError
from jsinfer.core.report import IngestError
from jsinfer.io.ingest import (
    MAX_NESTING_DEPTH,
    UTF8_BOM,
    DocOrigin,
    DocumentStore,
    IngestResult,
    Ingestor,
    documents_to_jsonl,
)

POSIX_PERMISSIONS = hasattr(os, "geteuid") and os.geteuid() != 0  # root can read anything


def write(path: Path, data: bytes | str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data.encode() if isinstance(data, str) else data)
    return path


def ingest(*inputs: Path | str, unwrap: bool = False, workers: int = 1, chunk_size: int = 2000) -> IngestResult:
    return Ingestor(InputOptions(unwrap_arrays=unwrap), workers, chunk_size).ingest(list(inputs))


def strict(value: Any) -> str:
    """A spelling that tells apart what Python's == conflates (True/1, 1/1.0) and keeps member order."""
    return json.dumps(value, ensure_ascii=False)


def docs(result: IngestResult) -> list[tuple[DocOrigin, str]]:
    return [(origin, strict(value)) for origin, value in zip(result.origins, result.store.values())]


def sources(result: IngestResult) -> list[str]:
    return [origin.source for origin in result.origins]


class Level(enum.IntEnum):
    HIGH = 3


class Name(str):
    pass


# ---------------------------------------------------------------------------
# Enumeration (1.2, 1.5)
# ---------------------------------------------------------------------------


def test_directory_files_are_ordered_by_relative_path_in_code_point_order(tmp_path: Path) -> None:
    # No two names differ only by case, since macOS file systems are usually case-insensitive.
    for name in ["c.json", "a/x.json", "a.json", "B.json", "\u00e9.json", "z.json", "a/b/c.json", "a-b.json"]:
        write(tmp_path / name, "1")
    result = ingest(tmp_path)
    # '-' < '.' < '/' < 'B' < 'a' < 'c' < 'z' < 'é', comparing whole relative paths.
    assert sources(result) == ["B.json", "a-b.json", "a.json", "a/b/c.json", "a/x.json", "c.json", "z.json", "\u00e9.json"]
    assert all(origin.line is None and origin.element is None for origin in result.origins)


def test_inputs_are_processed_in_argument_order(tmp_path: Path) -> None:
    second = write(tmp_path / "two" / "doc.json", '"two"').parent
    first = write(tmp_path / "one" / "doc.json", '"one"').parent
    single = write(tmp_path / "single.json", '"single"')
    result = ingest(second, single, first)
    assert [value for _, value in docs(result)] == ['"two"', '"single"', '"one"']
    # Directory files are named relative to their directory; supplied files as supplied.
    assert sources(result) == ["doc.json", str(single), "doc.json"]


def test_dotfiles_and_dot_directories_are_kept(tmp_path: Path) -> None:
    write(tmp_path / ".hidden.json", "1")
    write(tmp_path / ".dir" / "inner.json", "2")
    write(tmp_path / ".json", "3")  # no extension, as os.path.splitext sees it
    assert sources(ingest(tmp_path)) == [".dir/inner.json", ".hidden.json"]


def test_symbolic_links_are_not_followed_inside_directories(tmp_path: Path) -> None:
    root = tmp_path / "root"
    write(root / "real.json", '"real"')
    outside = write(tmp_path / "outside" / "out.json", '"outside"')
    (root / "link.json").symlink_to(root / "real.json")
    (root / "dangling.json").symlink_to(root / "missing.json")
    (root / "linkdir").symlink_to(outside.parent, target_is_directory=True)
    assert sources(ingest(root)) == ["real.json"]
    # A supplied symbolic link is followed.
    supplied = tmp_path / "supplied.json"
    supplied.symlink_to(root / "real.json")
    assert [value for _, value in docs(ingest(supplied))] == ['"real"']


def test_extensions_are_compared_case_insensitively(tmp_path: Path) -> None:
    write(tmp_path / "a.JSON", "[1]")
    write(tmp_path / "b.JsonL", "1\n2\n")
    write(tmp_path / "c.NDJSON", "3\n")
    for ignored in ["d.txt", "e.json.bak", "README", "f.jsonx", "g.yaml"]:
        write(tmp_path / ignored, "not json")
    result = ingest(tmp_path)
    assert [(o.source, o.line) for o in result.origins] == [("a.JSON", None), ("b.JsonL", 1), ("b.JsonL", 2), ("c.NDJSON", 1)]
    assert result.skipped == 0


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="needs named pipes")
def test_non_regular_files_inside_directories_are_skipped(tmp_path: Path) -> None:
    write(tmp_path / "a.json", "1")
    os.mkfifo(tmp_path / "pipe.json")  # reading it would block
    assert sources(ingest(tmp_path)) == ["a.json"]


@pytest.mark.skipif(not POSIX_PERMISSIONS, reason="needs POSIX permissions and a non-root user")
def test_unreadable_files_and_directories_are_recorded_in_order(tmp_path: Path) -> None:
    write(tmp_path / "a.json", "1")
    locked_file = write(tmp_path / "b.json", "2")
    locked_dir = write(tmp_path / "c" / "inner.json", "3").parent
    write(tmp_path / "d.json", "4")
    locked_file.chmod(0)
    locked_dir.chmod(0)
    try:
        result = ingest(tmp_path, chunk_size=1)
    finally:
        locked_file.chmod(0o644)
        locked_dir.chmod(0o755)
    assert sources(result) == ["a.json", "d.json"]
    assert result.errors == (
        IngestError("b.json", "file cannot be read: Permission denied"),
        IngestError("c", "directory cannot be read: Permission denied"),
    )
    assert result.skipped == 2


# ---------------------------------------------------------------------------
# Records: .json files, lines, and array unwrapping (1.1, 1.3, 1.9)
# ---------------------------------------------------------------------------


def test_json_file_is_one_document_of_any_top_level_type(tmp_path: Path) -> None:
    write(tmp_path / "multi.json", '{\n  "a": [1,\n 2]\n}\n')
    write(tmp_path / "scalar.json", UTF8_BOM + b'"text"')
    assert [value for _, value in docs(ingest(tmp_path))] == ['{"a": [1, 2]}', '"text"']


def test_lines_split_on_lf_with_crlf_and_blank_lines(tmp_path: Path) -> None:
    data = b'{"a": 1}\r\n\n   \n\t \r\n[2]\n\r\n"three"'  # no final terminator
    result = ingest(write(tmp_path / "x.jsonl", data))
    assert [(o.line, value) for o, value in docs(result)] == [(1, '{"a": 1}'), (5, "[2]"), (7, '"three"')]
    assert result.skipped == 0


def test_a_bare_cr_is_not_a_blank_line(tmp_path: Path) -> None:
    # Only one trailing CR is stripped; the remaining "\r" is not a space or a tab.
    result = ingest(write(tmp_path / "x.ndjson", b"1\n\r\r\n2\n"))
    assert [o.line for o in result.origins] == [1, 3]
    # JSON whitespace includes CR, so the value is expected after it.
    assert [(e.line, e.reason) for e in result.errors] == [(2, "invalid JSON: Expecting value (column 2)")]


def test_each_line_may_carry_a_bom(tmp_path: Path) -> None:
    result = ingest(write(tmp_path / "x.jsonl", UTF8_BOM + b"1\n" + UTF8_BOM + b"2\n"))
    assert [value for _, value in docs(result)] == ["1", "2"]


@pytest.mark.parametrize("unwrap", [False, True])
def test_array_unwrapping(tmp_path: Path, unwrap: bool) -> None:
    write(tmp_path / "a.json", '[1, [2, 3], {"k": []}]')
    write(tmp_path / "b.json", "[]")
    write(tmp_path / "c.json", '{"not": "an array"}')
    write(tmp_path / "d.jsonl", "[4, 5]\n")
    result = ingest(tmp_path, unwrap=unwrap)
    if unwrap:
        expected = [
            (DocOrigin("a.json", None, 0), "1"),
            (DocOrigin("a.json", None, 1), "[2, 3]"),  # nested arrays stay whole
            (DocOrigin("a.json", None, 2), '{"k": []}'),
            # b.json ([]) contributes no document
            (DocOrigin("c.json"), '{"not": "an array"}'),
            (DocOrigin("d.jsonl", 1), "[4, 5]"),  # lines are never unwrapped
        ]
    else:
        expected = [
            (DocOrigin("a.json"), '[1, [2, 3], {"k": []}]'),
            (DocOrigin("b.json"), "[]"),
            (DocOrigin("c.json"), '{"not": "an array"}'),
            (DocOrigin("d.jsonl", 1), "[4, 5]"),
        ]
    assert docs(result) == expected


# ---------------------------------------------------------------------------
# Skipped documents (1.7, 1.11)
# ---------------------------------------------------------------------------


def test_parse_errors_are_recorded_with_file_lines(tmp_path: Path) -> None:
    write(tmp_path / "a.jsonl", b'1\n\n{"x": NaN}\n2\n[1 2]\n\xff\n')
    write(tmp_path / "b.json", '{\n  "a": 1,\n  "b": \n}\n')
    write(tmp_path / "c.json", "")
    write(tmp_path / "d.jsonl", '{"ok": true}\n// comment\n')
    result = ingest(tmp_path)
    assert [(o.source, o.line) for o in result.origins] == [("a.jsonl", 1), ("a.jsonl", 4), ("d.jsonl", 1)]
    assert result.errors == (
        IngestError("a.jsonl", "NaN is not a JSON value under RFC 8259 (column 7)", 3),
        IngestError("a.jsonl", "invalid JSON: Expecting ',' delimiter (column 4)", 5),
        IngestError("a.jsonl", "invalid UTF-8 at byte offset 0: invalid start byte", 6),
        IngestError("b.json", "invalid JSON: Expecting value (column 1)", 4),
        IngestError("c.json", "invalid JSON: Expecting value (column 1)", 1),
        IngestError("d.jsonl", "invalid JSON: Expecting value (column 1)", 2),
    )
    assert result.skipped == len(result.errors) == 6


def test_an_unwrapped_array_that_fails_to_parse_is_one_skipped_document(tmp_path: Path) -> None:
    write(tmp_path / "bad.json", "[1, 2,, 3]")
    write(tmp_path / "good.json", "[1]")
    result = ingest(tmp_path, unwrap=True)
    assert docs(result) == [(DocOrigin("good.json", None, 0), "1")]
    assert result.errors == (IngestError("bad.json", "invalid JSON: Expecting value (column 7)", 1),)


def test_nesting_beyond_the_limit_is_a_depth_error(tmp_path: Path) -> None:
    deepest = b"[" * MAX_NESTING_DEPTH + b"]" * MAX_NESTING_DEPTH
    too_deep = b"[" * (MAX_NESTING_DEPTH + 1) + b"]" * (MAX_NESTING_DEPTH + 1)
    in_strings = b'["' + b"[" * 2000 + b'", "\\"' + b"{" * 2000 + b'"]'
    data = b"\n".join([deepest, too_deep, in_strings])
    path = write(tmp_path / "x.jsonl", data)
    result = ingest(path)
    assert [o.line for o in result.origins] == [1, 3]
    assert result.errors == (
        IngestError(str(path), f"nesting is deeper than the Ingestor's limit of {MAX_NESTING_DEPTH} levels", 2),
    )


# ---------------------------------------------------------------------------
# Rejected supplied paths (1.10) and no documents (1.8)
# ---------------------------------------------------------------------------


def test_rejected_supplied_paths_raise_config_error_naming_each_path(tmp_path: Path) -> None:
    good = write(tmp_path / "good.json", "1")
    missing = tmp_path / "missing.json"
    text = write(tmp_path / "notes.TXT", "x")
    bare = write(tmp_path / "README", "x")
    with pytest.raises(ConfigError) as info:
        ingest(good, missing, text, bare, tmp_path)
    error = info.value
    assert error.subjects == (str(missing), str(text), str(bare))
    assert all(problem.reason == "path" for problem in error.problems)
    messages = [problem.message for problem in error.problems]
    assert messages == [
        f"input path {str(missing)!r} does not exist",
        f"input path {str(text)!r} has the extension '.TXT'; expected .json, .jsonl, or .ndjson",
        f"input path {str(bare)!r} has no extension; expected .json, .jsonl, or .ndjson",
    ]


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="needs named pipes")
def test_a_supplied_non_regular_file_is_rejected(tmp_path: Path) -> None:
    pipe = tmp_path / "pipe.jsonl"
    os.mkfifo(pipe)
    with pytest.raises(ConfigError, match="is neither a regular file nor a directory"):
        ingest(pipe)


@pytest.mark.skipif(not POSIX_PERMISSIONS, reason="needs POSIX permissions and a non-root user")
def test_unreadable_supplied_paths_are_rejected(tmp_path: Path) -> None:
    locked_file = write(tmp_path / "locked.json", "1")
    locked_dir = write(tmp_path / "dir" / "a.json", "1").parent
    locked_file.chmod(0)
    locked_dir.chmod(0)
    try:
        with pytest.raises(ConfigError) as info:
            ingest(locked_file, locked_dir)
    finally:
        locked_file.chmod(0o644)
        locked_dir.chmod(0o755)
    assert [problem.message for problem in info.value.problems] == [
        f"input path {str(locked_file)!r} cannot be read: Permission denied",
        f"input path {str(locked_dir)!r} is a directory that cannot be read: Permission denied",
    ]


@pytest.mark.parametrize(
    ("files", "unwrap", "skipped"),
    [
        ({}, False, 0),  # an empty directory
        ({"a.txt": "1", "b.yaml": "1"}, False, 0),  # no supported extension
        ({"a.jsonl": "\n  \n\t\r\n", "b.ndjson": ""}, False, 0),  # every line blank
        ({"a.json": "[]"}, True, 0),  # an unwrapped empty array
        ({"a.json": "{", "b.jsonl": "x\n"}, False, 2),  # every document fails
    ],
)
def test_zero_documents_raise_no_documents_error(tmp_path: Path, files: dict[str, str], unwrap: bool, skipped: int) -> None:
    for name, text in files.items():
        write(tmp_path / name, text)
    with pytest.raises(NoDocumentsError) as info:
        ingest(tmp_path, unwrap=unwrap)
    error = info.value
    assert isinstance(error, InferenceError)
    assert error.stage == "Ingestor"
    assert error.skipped == len(error.errors) == skipped
    assert str(error).startswith("no valid documents were found")


def test_no_inputs_raise_no_documents_error() -> None:
    with pytest.raises(NoDocumentsError):
        ingest()


def test_errors_pickle() -> None:
    error = NoDocumentsError([IngestError("a.json", "invalid JSON: Expecting value", 1)])
    copy = pickle.loads(pickle.dumps(error))
    assert type(copy) is NoDocumentsError
    assert (copy.errors, copy.stage, str(copy)) == (error.errors, "Ingestor", str(error))
    assert str(error) == "no valid documents were found; skipped 1 document or file that could not be parsed or read"
    stage_error = pickle.loads(pickle.dumps(InferenceError("boom", "Flatten")))
    assert (stage_error.stage, str(stage_error), stage_error.partial_report) == ("Flatten", "boom", None)


# ---------------------------------------------------------------------------
# Determinism across workers and chunk sizes (1.5, 29.4)
# ---------------------------------------------------------------------------


def _mixed_tree(root: Path) -> None:
    for i in range(7):
        write(root / f"part{i % 3}" / f"f{i}.jsonl", "".join(f'{{"i": {i}, "j": {j}}}\n' for j in range(5)) + "bad\n\n")
        write(root / f"arr{i}.json", json.dumps([{"k": i}, i, None]))
    write(root / "big.json", "123456789012345678901234567890")
    write(root / "broken.json", "{")


@pytest.mark.parametrize(("workers", "chunk_size"), [(1, 1), (1, 4), (2, 3), (3, 1000)])
def test_indices_do_not_depend_on_workers_or_chunk_size(tmp_path: Path, workers: int, chunk_size: int) -> None:
    _mixed_tree(tmp_path)
    for unwrap in (False, True):
        reference = ingest(tmp_path, unwrap=unwrap)
        result = ingest(tmp_path, unwrap=unwrap, workers=workers, chunk_size=chunk_size)
        assert result == reference
        assert result.store == reference.store
        assert len(result.store) == len(result.origins) == (57 if unwrap else 43)
        assert result.skipped == 8


# ---------------------------------------------------------------------------
# DocumentStore
# ---------------------------------------------------------------------------


def test_store_holds_canonical_bytes_that_round_trip(tmp_path: Path) -> None:
    big = 2**100 + 1
    write(tmp_path / "a.jsonl", f'{{"z": 1.0, "a": [{big}, -0.0, 1e300]}}\n{{ "z" : 1.0 ,"a":[ {big},-0.0,1E300]}}\n')
    write(tmp_path / "b.json", UTF8_BOM + '"\\u00e9\\ud83d\\ude00"'.encode())
    store = ingest(tmp_path).store
    first = store.value(0)
    assert strict(first) == f'{{"z": 1.0, "a": [{big}, -0.0, 1e+300]}}'  # member order and 1.0 kept
    assert first["a"][0] == big
    assert store[0] == store[1] == encode_canonical(first)  # equal values, equal bytes
    assert store.value(2) == "\u00e9\U0001f600"
    assert store[-1] == store[2]
    assert list(store) == [store[0], store[1], store[2]]
    assert store.buffer == b"".join(store)
    assert store.offsets.dtype == np.int64 and not store.offsets.flags.writeable
    assert store.offsets.tolist()[0] == 0 and store.offsets.tolist()[-1] == store.nbytes
    assert pickle.loads(pickle.dumps(store)) == store


def test_store_validates_its_arguments() -> None:
    store = DocumentStore(b"1\n22\n", [0, 2, 5])
    assert len(store) == 2 and store[1] == b"22\n"
    assert list(store.values(1)) == [22]
    with pytest.raises(IndexError):
        store[2]
    with pytest.raises(TypeError):
        store[True]
    assert len(DocumentStore(b"", [0])) == 0
    for offsets in ([], [1, 5], [0, 3], [0, 4, 2, 5], [[0, 5]], [0.0, 5.0]):
        with pytest.raises(ValueError):
            DocumentStore(b"1\n22\n", offsets)
    with pytest.raises(TypeError):
        DocumentStore(bytearray(b"1"), [0, 1])  # type: ignore[arg-type]


def test_ingestor_validates_its_arguments(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="workers"):
        Ingestor(InputOptions(), 0, 10)
    with pytest.raises(ValueError, match="ingest.chunk_size"):
        Ingestor(InputOptions(), 1, 0)
    with pytest.raises(TypeError):
        Ingestor({"unwrap_arrays": True}, 1, 10)  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="single path"):
        Ingestor(InputOptions(), 1, 10).ingest(str(tmp_path))  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# In-memory documents (Ingestor.ingest_bytes, Ingestor.ingest_values; task 16.2)
# ---------------------------------------------------------------------------


def test_ingest_bytes_reads_a_jsonl_file_content(tmp_path: Path) -> None:
    data = b'{"a": 1}\r\n\n \t\n[1, 2]\n{bad\n"\\ud800"\n7'
    ingestor = Ingestor(InputOptions(), 1, 2)
    from_bytes = ingestor.ingest_bytes(data, "data.jsonl")
    path = write(tmp_path / "data.jsonl", data)
    from_file = ingest(path, chunk_size=2)
    assert from_bytes.origins == tuple(DocOrigin("data.jsonl", origin.line) for origin in from_file.origins)
    assert from_bytes.errors == tuple(IngestError("data.jsonl", e.reason, e.line) for e in from_file.errors)
    assert from_bytes.store == from_file.store
    assert [origin.line for origin in from_bytes.origins] == [1, 4, 7]
    assert [error.line for error in from_bytes.errors] == [5, 6]
    assert sources(Ingestor(InputOptions(), 1, 2).ingest_bytes(b"1\n")) == ["<jsonl>"]


def test_ingest_values_matches_a_jsonl_file_with_one_document_per_line(tmp_path: Path) -> None:
    values: list[Any] = [{"b": 1, "a": [1.0, 2**70, None]}, "x\u2028y\n", True, -0.0, "\U0001f600"]
    ingestor = Ingestor(InputOptions(unwrap_arrays=True), 1, 2)  # unwrapping never applies to lines
    from_values = ingestor.ingest_values(iter(values))
    lines = "\n".join(json.dumps(value, ensure_ascii=False) for value in values)
    from_file = ingest(write(tmp_path / "docs.jsonl", lines), unwrap=True)
    assert [strict(value) for value in from_values.store.values()] == [strict(value) for value in values]
    assert from_values.store == from_file.store
    assert from_values.origins == tuple(DocOrigin("<documents>", line) for line in range(1, 6))


def test_ingest_values_skips_documents_a_jsonl_file_could_not_carry(tmp_path: Path) -> None:
    deep: Any = 0
    for _ in range(MAX_NESTING_DEPTH + 1):
        deep = [deep]
    huge = 10 ** (sys.get_int_max_str_digits() + 5)
    values: list[Any] = [1, float("nan"), {"s": "\ud800"}, [float("-inf")], deep, {"n": huge}, 5]
    result = Ingestor(InputOptions(), 1, 3).ingest_values(values, "api")
    assert [strict(value) for value in result.store.values()] == ["1", "5"]
    assert [origin.line for origin in result.origins] == [1, 7]
    assert [(error.source, error.line) for error in result.errors] == [("api", n) for n in (2, 3, 4, 5, 6)]
    reasons = [error.reason for error in result.errors]
    assert reasons[0].startswith("NaN is not a JSON value") and reasons[2].startswith("-Infinity is not")
    assert "surrogate" in reasons[1] and "nesting" in reasons[3] and "conversion limit" in reasons[4]
    # The same failures, reasons, and lines as a .jsonl file holding the same lines.
    from_file = ingest(write(tmp_path / "api.jsonl", documents_to_jsonl(values)), chunk_size=3)
    assert from_file.errors == tuple(IngestError(str(tmp_path / "api.jsonl"), e.reason, e.line) for e in result.errors)
    assert from_file.store == result.store


def test_ingest_values_rejects_values_outside_the_json_domain() -> None:
    circular: list[Any] = []
    circular.append(circular)
    ingestor = Ingestor(InputOptions(), 1, 3)
    for bad, where in [
        ((2, 3), "tuple at (root)"),
        ({"a": [1, {"b": {1, 2}}]}, "set at /a/1/b"),
        ([b"x"], "bytes at /0"),
        ({"a/b": {"~": object()}}, "object at /a~1b/~0"),
        ({4: "four"}, "key of type int"),
        ({"a": {None: 1}}, "the object at /a has a key of type NoneType"),
    ]:
        with pytest.raises(TypeError, match="document 1 of the iterable") as caught:
            ingestor.ingest_values([1, bad, 2])
        assert where in str(caught.value)
    with pytest.raises(ValueError, match="the array at /k/0 contains itself"):
        ingestor.ingest_values([{"k": circular}])


def test_documents_to_jsonl_writes_the_lines_json_dumps_writes() -> None:
    shared = [1]
    values: list[Any] = [
        {"b": 1, "a": [1.0, 2**70, None, -0.0, 1e300, "\u00e9\U0001f600\u2028\n\"\\"]},
        {"": {}, "x": [[], [[]]], "y": [shared, shared]},  # a value reached twice is not a cycle
        True, False, None, "", 0, -5, 3.25, float("nan"), float("inf"), "\ud800",
        # Subclasses of the JSON types are written as their base type, as json.dumps writes them.
        OrderedDict([("z", 1), ("a", 2)]), Level.HIGH, np.float64(0.5), Name("text"),
    ]
    data = documents_to_jsonl(iter(values))
    assert data == b"".join(
        json.dumps(value, ensure_ascii=True, separators=(",", ":")).encode("ascii") + b"\n" for value in values
    )
    assert documents_to_jsonl([]) == b""
    # Integers past the integer string conversion limit are written in full (and then fail to parse).
    limit = sys.get_int_max_str_digits()
    assert documents_to_jsonl([-(10 ** (limit + 1)) - 7]) == b"-1" + b"0" * limit + b"7\n"
    assert documents_to_jsonl([10 ** (3 * limit)]) == b"1" + b"0" * (3 * limit) + b"\n"
    for bad in ("text", b"bytes", {"a": 1}, 5):
        with pytest.raises(TypeError, match="iterable of JSON values"):
            documents_to_jsonl(bad)  # type: ignore[arg-type]


def test_ingest_values_without_documents_raises() -> None:
    ingestor = Ingestor(InputOptions(), 1, 10)
    with pytest.raises(NoDocumentsError) as caught:
        ingestor.ingest_values([])
    assert caught.value.errors == ()
    with pytest.raises(NoDocumentsError) as caught:
        ingestor.ingest_values([float("inf")])
    assert [error.line for error in caught.value.errors] == [1]
    with pytest.raises(NoDocumentsError):
        ingestor.ingest_bytes(b"\n  \n")
    for bad in ("text", b"bytes", {"a": 1}, 5):
        with pytest.raises(TypeError):
            ingestor.ingest_values(bad)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        ingestor.ingest_bytes("text")  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        ingestor.ingest_values([1], "")
