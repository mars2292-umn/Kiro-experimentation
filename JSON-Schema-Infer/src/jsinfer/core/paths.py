"""Paths and their steps: the model, canonical order, rendering, and parsing.

A Path is a tuple of :class:`Step` from the document root; ``()`` is the root.
There are three kinds of step:

``Key``
    The value under one object key (a Record property).
``Elem``, rendered ``[*]``
    Every element of an array. All elements of a List share this one
    wildcard step (Requirement 7.2). Tuple positions are not steps: a Tuple's
    ``prefixItems[i]`` is inferred from the element view filtered to position i.
``MapVal``, rendered ``{*}``
    Every value of a Map, the wildcard step of Requirement 4.3.

Order
    Steps order Key < Elem < MapVal, and Key steps among themselves by the
    code-point order of their keys. Paths order by their step tuples, so a
    Path sorts directly before its extensions. :func:`fewest_steps_key` orders
    by step count first and breaks ties with this order (Requirement 18.3).

Rendering
    ``$`` for the root, then for each step ``.name`` when the key is an ASCII
    identifier (``[A-Za-z_][A-Za-z0-9_]*``) and ``["name"]`` otherwise, ``[*]``,
    and ``{*}``, for example ``$.users[*].tags{*}``. In the bracket form the
    key is a JSON string literal that escapes only ``"``, ``\\``, and the
    control characters U+0000 to U+001F; all other characters appear as
    themselves, so the rendering is valid UTF-8 text.

Parsing
    :func:`parse_path` reads Path strings from configuration (for example a
    timestamp Ordering_Key, Requirements 18.2 and 18.11). The grammar is::

        path = "$" *step
        step = "." name / "[*]" / "{*}" / "[" json-string "]"
        name = (ALPHA / "_") *(ALPHA / DIGIT / "_")    ; ASCII only

    ``json-string`` is an RFC 8259 string literal, so every JSON escape is
    accepted, and a surrogate pair escape decodes to one code point. Input is
    taken as written: no whitespace, single quotes, or numeric indices.
    ``render_path(parse_path(s)) == s`` holds for every rendered string, and
    ``parse_path(render_path(p)) == p`` for every Path. Non-canonical inputs
    such as ``$["users"]`` parse to the same Path as ``$.users``.

Keys must be strings of Unicode scalar values. A key with an unpaired
surrogate is rejected, as unpaired surrogates in documents are parse
failures; this keeps rendering valid UTF-8 and the round trip total.

Steps define ``__canonical__``, so a Path has the canonical bytes of the
nested array ``[[kind, key], ...]`` and can key random streams in
``core.determinism`` by content.
"""

from __future__ import annotations

import json
import re
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from typing import Final, Literal, TypeAlias

from jsinfer.core.determinism import codepoint_key

__all__ = [
    "ELEM",
    "ELEM_STEP",
    "KEY",
    "MAPVAL",
    "MAPVAL_STEP",
    "ROOT",
    "Path",
    "PathSyntaxError",
    "Step",
    "fewest_steps_key",
    "key_step",
    "parse_path",
    "path_sort_key",
    "render_path",
    "render_step",
    "sorted_paths",
]

# Step kinds. Their numeric values define the step order Key < Elem < MapVal.
KEY: Final = 0
ELEM: Final = 1
MAPVAL: Final = 2

StepKind: TypeAlias = Literal[0, 1, 2]

_SURROGATE_RE: Final = re.compile(r"[\ud800-\udfff]")


@dataclass(frozen=True, order=True, slots=True)
class Step:
    """One step of a Path: ``Step(KEY, name)``, ``Step(ELEM)``, or ``Step(MAPVAL)``.

    The dataclass order compares ``(kind, key)``, which is the canonical step
    order. Use :func:`key_step`, :data:`ELEM_STEP`, and :data:`MAPVAL_STEP` to
    build steps.
    """

    kind: StepKind
    key: str = ""  # only for Key steps; "" is a valid (empty) object key

    def __post_init__(self) -> None:
        kind = self.kind
        if isinstance(kind, bool) or not isinstance(kind, int) or kind not in (KEY, ELEM, MAPVAL):
            raise ValueError(f"step kind must be KEY (0), ELEM (1), or MAPVAL (2), got {kind!r}")
        if not isinstance(self.key, str):
            raise TypeError(f"step key must be str, got {type(self.key).__name__}")
        if kind != KEY:
            if self.key:
                raise ValueError(f"only Key steps carry a key, got kind {kind} with key {self.key!r}")
        else:
            bad = _SURROGATE_RE.search(self.key)
            if bad is not None:
                raise ValueError(
                    f"step key contains the surrogate code point U+{ord(bad.group()):04X} at index "
                    f"{bad.start()}; keys must be valid Unicode (no unpaired surrogates)"
                )

    def __canonical__(self) -> list[int | str]:
        return [int(self.kind), self.key]

    def __str__(self) -> str:
        return render_step(self)


Path: TypeAlias = tuple[Step, ...]

ROOT: Final[Path] = ()
ELEM_STEP: Final = Step(ELEM)
MAPVAL_STEP: Final = Step(MAPVAL)


def key_step(name: str) -> Step:
    """Return the Key step for object key ``name``."""
    return Step(KEY, name)


# ---------------------------------------------------------------------------
# Order
# ---------------------------------------------------------------------------


def path_sort_key(path: Sequence[Step]) -> tuple[tuple[int, str], ...]:
    """Return the canonical sort key of a Path: its ``(kind, key)`` step tuples.

    Keys compare in code-point order. The result orders Paths exactly as the
    step tuples themselves compare.
    """
    return tuple((int(step.kind), codepoint_key(step.key)) for step in path)


def sorted_paths(paths: Iterable[Sequence[Step]]) -> list[Path]:
    """Return ``paths`` as tuples in canonical Path order."""
    return sorted((tuple(path) for path in paths), key=path_sort_key)


def fewest_steps_key(path: Sequence[Step]) -> tuple[int, tuple[tuple[int, str], ...]]:
    """Sort key for "fewest steps, then lexicographic order of the steps" (Requirement 18.3)."""
    return (len(path), path_sort_key(path))


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------

_IDENT_RE: Final = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_ESCAPE_RE: Final = re.compile(r'["\\\x00-\x1f]')
_SHORT_ESCAPES: Final = {'"': '\\"', "\\": "\\\\", "\b": "\\b", "\f": "\\f", "\n": "\\n", "\r": "\\r", "\t": "\\t"}


def _escape(match: re.Match[str]) -> str:
    char = match.group()
    return _SHORT_ESCAPES.get(char) or f"\\u{ord(char):04x}"


def _quote(name: str) -> str:
    """Return ``name`` as a JSON string literal with minimal escaping."""
    return '"' + _ESCAPE_RE.sub(_escape, name) + '"'


def render_step(step: Step) -> str:
    """Render one step as ``.name``, ``["name"]``, ``[*]``, or ``{*}``."""
    if not isinstance(step, Step):
        raise TypeError(f"expected a Step, got {type(step).__name__}")
    if step.kind == ELEM:
        return "[*]"
    if step.kind == MAPVAL:
        return "{*}"
    if _IDENT_RE.fullmatch(step.key):
        return "." + step.key
    return "[" + _quote(step.key) + "]"


def render_path(path: Sequence[Step]) -> str:
    """Render a Path, for example ``$.users[*].tags{*}``; the root renders as ``$``."""
    return "$" + "".join(render_step(step) for step in path)


# ---------------------------------------------------------------------------
# Parsing
# ---------------------------------------------------------------------------


class PathSyntaxError(ValueError):
    """A Path string that does not follow the Path grammar.

    ``text`` is the input, ``position`` the 0-based code-point index where
    parsing failed, and ``reason`` what was expected there.
    """

    def __init__(self, text: str, position: int, reason: str) -> None:
        # Keep the parts in args so the exception pickles (joblib workers).
        super().__init__(text, position, reason)
        self.text = text
        self.position = position
        self.reason = reason

    def __str__(self) -> str:
        return f"invalid Path {self.text!r} at position {self.position}: {self.reason}"


_STRING_DECODER: Final = json.JSONDecoder(strict=True)
_EXPECTED_STEP: Final = "expected '.name', '[\"key\"]', '[*]', or '{*}'"
_NAME_CHARS: Final = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_")
_NAME_HINT: Final = (
    "a '.name' holds only ASCII letters, digits, and '_', and does not start with a digit "
    '(write other keys as ["key"])'
)


def parse_path(text: str) -> Path:
    """Parse a Path string (see the module docstring for the grammar).

    Raises :class:`PathSyntaxError` for malformed input and ``TypeError`` when
    ``text`` is not a ``str``.
    """
    if not isinstance(text, str):
        raise TypeError(f"Path string must be str, got {type(text).__name__}")
    if not text.startswith("$"):
        raise PathSyntaxError(text, 0, "a Path starts with '$'")
    steps: list[Step] = []
    i, n = 1, len(text)
    while i < n:
        char = text[i]
        if char == ".":
            match = _IDENT_RE.match(text, i + 1)
            if match is None:
                raise PathSyntaxError(text, i + 1, f"expected a name after '.'; {_NAME_HINT}")
            steps.append(Step(KEY, match.group()))
            i = match.end()
        elif text.startswith("[*]", i):
            steps.append(ELEM_STEP)
            i += 3
        elif text.startswith("{*}", i):
            steps.append(MAPVAL_STEP)
            i += 3
        elif char == "[" and text.startswith('"', i + 1):
            try:
                name, end = _STRING_DECODER.raw_decode(text, i + 1)
            except json.JSONDecodeError as exc:
                raise PathSyntaxError(text, exc.pos, f"invalid JSON string literal: {exc.msg}") from None
            if not text.startswith("]", end):
                raise PathSyntaxError(text, end, "expected ']' after the quoted key")
            try:
                steps.append(Step(KEY, name))
            except ValueError as exc:
                raise PathSyntaxError(text, i + 1, str(exc)) from None
            i = end + 1
        else:
            # Only a '.name' token ends in a name character, so this is a
            # '.name' running into a character it cannot hold.
            ends_name = text[i - 1] in _NAME_CHARS
            raise PathSyntaxError(text, i, f"{_EXPECTED_STEP}; {_NAME_HINT}" if ends_name else _EXPECTED_STEP)
    return tuple(steps)
