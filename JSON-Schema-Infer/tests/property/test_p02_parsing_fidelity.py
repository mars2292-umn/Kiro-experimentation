"""Property 2: Parsing fidelity (task 4.4).

For any JSON value, including integers outside the 64-bit range and
arbitrary Unicode strings, serialized as UTF-8 with or without a single
leading BOM, ingestion yields a value equal to the original with every
integer preserved exactly.

Values come from :func:`tests.strategies.rfc8259_values`, the domain the
Ingestor accepts: strings of Unicode scalar values, finite floats, integers
up to the integer string conversion limit, and nesting up to
``MAX_NESTING_DEPTH``. They are serialized by the stdlib encoder with and
without ``ensure_ascii`` (so astral characters also appear as surrogate-pair
escapes), with several whitespace layouts. Fidelity is checked for
:func:`parse_document` directly, with and without a BOM, and through the
whole Ingestor: a ``.json`` file and a ``.jsonl`` line, each read back with
``DocumentStore.value``, which re-parses the canonical bytes.

Equality is strict: types must match (``True`` is not ``1`` and ``1`` is not
``1.0``), floats must have the same bits (``-0.0`` is not ``0.0``), and object
members must keep their order.

**Validates: Requirements 1.4, 1.6**
"""

# Feature: json-schema-inference, Property 2: Parsing fidelity

from __future__ import annotations

import json
import os
import tempfile
from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.run_config import InputOptions
from jsinfer.io.ingest import UTF8_BOM, Ingestor, parse_document
from tests.strategies import json_depth, rfc8259_values

# JSON whitespace (RFC 8259, section 2). A .jsonl line holds no line terminator.
_WHITESPACE = " \t\r\n"
_LINE_WHITESPACE = " \t"


def _short(value: Any, limit: int = 120) -> str:
    text = repr(value)
    return text if len(text) <= limit else f"{text[: limit - 3]}..."


def assert_same_json(actual: Any, expected: Any) -> None:
    """Assert that ``actual`` equals ``expected`` strictly: same types, float bits, and member order."""
    pending: list[tuple[tuple[Any, ...], Any, Any]] = [((), actual, expected)]
    while pending:  # an explicit stack, since values nest up to MAX_NESTING_DEPTH levels
        where, got, want = pending.pop()
        if type(got) is not type(want):
            same = False
        elif type(want) is float:
            same = got.hex() == want.hex()
        elif type(want) is list:
            same = len(got) == len(want)
            pending.extend((where + (index,), a, b) for index, (a, b) in enumerate(zip(got, want)))
        elif type(want) is dict:
            same = list(got) == list(want)
            pending.extend((where + (key,), got[key], want[key]) for key in want if key in got)
        else:
            same = got == want
        # The message (a repr of possibly huge values) is built only on failure.
        assert same, (
            f"at depth {len(where)}, path ending {_short(list(where[-3:]))}: "
            f"got {type(got).__name__} {_short(got)}, expected {type(want).__name__} {_short(want)}"
        )


@st.composite
def serializations(draw: st.DrawFn, value: Any, one_line: bool = False) -> bytes:
    """``value`` as UTF-8 JSON text (no BOM) in a drawn layout; on one line when ``one_line``."""
    ensure_ascii = draw(st.booleans(), label="ensure_ascii")
    indent = None if one_line else draw(st.sampled_from([None, 0, 2, "\t"]), label="indent")
    separators = draw(st.sampled_from([None, (",", ":"), (" , ", " :\t")]), label="separators")
    padding = st.text(st.sampled_from(_LINE_WHITESPACE if one_line else _WHITESPACE), max_size=3)
    text = json.dumps(value, ensure_ascii=ensure_ascii, indent=indent, separators=separators, allow_nan=False)
    return (draw(padding, label="leading whitespace") + text + draw(padding, label="trailing whitespace")).encode()


@given(rfc8259_values(), st.data())
def test_parse_document_returns_the_serialized_value_with_or_without_a_bom(value: Any, data: st.DataObject) -> None:
    text = data.draw(serializations(value), label="document")
    for prefix in (b"", UTF8_BOM):
        parsed, parser = parse_document(prefix + text)
        assert_same_json(parsed, value)
        event(f"parser: {parser}")
    event(f"nesting: {'deep' if json_depth(value) > 64 else 'shallow'}")


@given(rfc8259_values(), st.data())
def test_ingesting_a_json_file_and_a_jsonl_line_returns_the_value(value: Any, data: st.DataObject) -> None:
    files = {
        "doc.json": serializations(value),
        "doc.jsonl": st.tuples(serializations(value, one_line=True), st.sampled_from([b"", b"\n", b"\r\n"])).map(
            b"".join
        ),
    }
    with tempfile.TemporaryDirectory() as directory:  # one directory per example
        paths = []
        for name, content in files.items():
            bom = UTF8_BOM if data.draw(st.booleans(), label=f"{name} starts with a BOM") else b""
            path = os.path.join(directory, name)
            with open(path, "wb") as handle:
                handle.write(bom + data.draw(content, label=name))
            paths.append(path)
        result = Ingestor(InputOptions(unwrap_arrays=False), workers=1, chunk_size=1).ingest(paths)

    assert result.errors == ()
    assert len(result.store) == len(files)
    for index in range(len(files)):
        assert_same_json(result.store.value(index), value)
