"""Safe regex AST, printer, and matcher (Requirements 14.4, 14.5).

Pattern induction (task 32.1) builds its candidates from this AST, the
Emitter writes them with the printer, and the MDL_Scorer reads pattern data
bits off the matcher's decomposition.

AST
    A :class:`Pattern` is a non-empty disjunction of :class:`Alternative`
    sub-patterns, and an Alternative is a sequence of :class:`Token` atoms. A
    Token is one of the five ASCII classes of :class:`CharClass` (``[0-9]``,
    ``[a-z]``, ``[A-Z]``, ``[A-Za-z]``, ``[0-9A-Za-z]``) or one literal code
    point, repeated as its :class:`Quantifier` says. A Quantifier repeats at
    least once and at most ``high`` times, or without bound when ``high`` is
    ``None``. These are FlashProfile's atoms: an exact run length is
    ``{n}``, a range ``{n,m}``, and the most general form ``+``.

    A literal is never an ASCII letter or digit (those characters belong to
    the classes, as in FlashProfile tokenization) and never a surrogate code
    point (jsonschema-rs takes patterns as Rust strings, which cannot hold
    one, and a UTF-16 engine would see half of a pair).

Normal form and unique decomposition
    The five classes overlap only by containment: ``[0-9]``, ``[a-z]``, and
    ``[A-Z]`` are disjoint, ``[A-Za-z]`` is the union of the last two, and
    ``[0-9A-Za-z]`` holds all three. Building an Alternative merges adjacent
    tokens that share a character, left to right until no adjacent pair
    does: two classes become the larger class with the quantifiers added
    (``[a-z]{2}[A-Za-z]+`` becomes ``[A-Za-z]{3,}``, which may accept more
    strings), and two runs of one literal become one run (``--`` becomes
    ``(?:-){2}``, which accepts the same strings). A literal never shares a
    character with a class. Duplicate alternatives of a Pattern are dropped,
    keeping the first.

    In normal form every token matches at least one character and adjacent
    tokens are disjoint. The character after a token's run therefore belongs
    to the next token and not to this one, so each token consumes exactly
    the maximal run of its characters that starts where the previous token
    stopped. A string matched by an alternative thus has exactly one split
    into token runs. Among the alternatives, the decomposition uses the first
    one that matches, which makes it unique for the whole Pattern.

Printer
    :attr:`Pattern.source` is ``^(?:alt1|…|altk)$(?!\\n)``. The anchors make
    every engine require a match of the whole string. Python's ``$`` also
    matches before a final newline, and the lookahead rules that position
    out; ECMA-262 and jsonschema-rs's default fancy-regex engine (which
    supports lookaround) match ``$`` only at the end. Inside an alternative:

    - a class prints as written above;
    - a literal prints raw, except that the ECMA-262 syntax characters
      ``^$\\.*+?()[]{}|/`` get a backslash. Nothing else is escaped, because
      ECMA-262 in Unicode mode rejects identity escapes of other characters
      (``\\-`` is a syntax error there);
    - a quantifier prints as nothing (exactly once), ``{n}``, ``{n,m}``,
      ``+``, or ``{n,}``. It follows a class directly and wraps a literal in a
      group, so ``(?:😀){2}`` repeats the whole astral character in UTF-16
      engines too, where ``😀{2}`` would repeat only its low surrogate.

    An alternative without tokens prints as the empty string and matches
    only ``""``.

Parser
    :meth:`Pattern.from_source` inverts the printer: it returns the Pattern
    whose :attr:`Pattern.source` is the given string, and raises
    ``ValueError`` for every other string, including regexes that are
    equivalent but printed differently (``[a-z][a-z]``, ``{1}``, ``{1,}``,
    a needless escape, repeated alternatives). The MDL_Scorer uses it to
    read the AST back from a ``pattern`` value or a ``patternProperties``
    key.

Matcher
    :meth:`Pattern.match` returns a :class:`Decomposition`: the index of the
    first matching alternative and, per token, the code-point span of its
    run. Its repetition count is the span's length, since each repetition is
    one code point. Matching scans each run once, so it is linear in the
    string length per alternative, and it never compiles the printed
    pattern. A string matches exactly when the unanchored body matches the
    entire string, which is what all three engines report for the anchored
    source (Property 24).
"""

from __future__ import annotations

import math
import re
from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass
from enum import Enum
from typing import Final

__all__ = [
    "ONE",
    "PLUS",
    "SYNTAX_CHARACTERS",
    "Alternative",
    "CharClass",
    "Decomposition",
    "Pattern",
    "Quantifier",
    "Token",
    "escape",
]

# ECMA-262 SyntaxCharacter plus "/", the characters a literal escapes.
SYNTAX_CHARACTERS: Final = "^$\\.*+?()[]{}|/"


# ---------------------------------------------------------------------------
# Character classes
# ---------------------------------------------------------------------------

_DIGIT_BIT: Final = 1
_LOWER_BIT: Final = 2
_UPPER_BIT: Final = 4


def _base_bit(ch: str) -> int:
    """The base-class bit of one character: digit, lower, upper, or 0 for anything else."""
    if "0" <= ch <= "9":
        return _DIGIT_BIT
    if "a" <= ch <= "z":
        return _LOWER_BIT
    if "A" <= ch <= "Z":
        return _UPPER_BIT
    return 0


class CharClass(Enum):
    """One of the five ASCII classes; the value is its printed form."""

    DIGIT = "[0-9]"
    LOWER = "[a-z]"
    UPPER = "[A-Z]"
    ALPHA = "[A-Za-z]"
    ALNUM = "[0-9A-Za-z]"

    @property
    def size(self) -> int:
        """The number of characters in the class."""
        return _SIZES[self]

    def contains(self, ch: str) -> bool:
        """Whether the one-character string ``ch`` is in the class."""
        return bool(_MASKS[self] & _base_bit(ch))

    def overlaps(self, other: CharClass) -> bool:
        """Whether the two classes share a character (then one contains the other)."""
        return bool(_MASKS[self] & _MASKS[other])

    def issubset(self, other: CharClass) -> bool:
        return _MASKS[self] & ~_MASKS[other] == 0

    def join(self, other: CharClass) -> CharClass:
        """The smallest of the five classes that contains both (``[0-9]`` and ``[a-z]`` join at ``[0-9A-Za-z]``)."""
        union = _MASKS[self] | _MASKS[other]
        for candidate in _BY_SIZE:
            if _MASKS[candidate] & union == union:
                return candidate
        raise AssertionError("ALNUM contains every class")  # pragma: no cover

    @classmethod
    def base_of(cls, ch: str) -> CharClass | None:
        """``DIGIT``, ``LOWER``, or ``UPPER`` for an ASCII digit or letter, else None."""
        return _BASE_CLASSES.get(_base_bit(ch))


_MASKS: Final[dict[CharClass, int]] = {
    CharClass.DIGIT: _DIGIT_BIT,
    CharClass.LOWER: _LOWER_BIT,
    CharClass.UPPER: _UPPER_BIT,
    CharClass.ALPHA: _LOWER_BIT | _UPPER_BIT,
    CharClass.ALNUM: _DIGIT_BIT | _LOWER_BIT | _UPPER_BIT,
}
_SIZES: Final[dict[CharClass, int]] = {
    CharClass.DIGIT: 10,
    CharClass.LOWER: 26,
    CharClass.UPPER: 26,
    CharClass.ALPHA: 52,
    CharClass.ALNUM: 62,
}
_BY_SIZE: Final = (CharClass.DIGIT, CharClass.LOWER, CharClass.UPPER, CharClass.ALPHA, CharClass.ALNUM)
_BASE_CLASSES: Final = {_DIGIT_BIT: CharClass.DIGIT, _LOWER_BIT: CharClass.LOWER, _UPPER_BIT: CharClass.UPPER}

# Maximal-run scanners: run(s, pos).end() is where the class's run from pos stops.
_CLASS_RUNS: Final = {cls: re.compile(cls.value + "*").match for cls in _BY_SIZE}


# ---------------------------------------------------------------------------
# Quantifiers
# ---------------------------------------------------------------------------


def _count(value: object, what: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{what} must be an int, got {type(value).__name__}")
    return value


@dataclass(frozen=True, slots=True)
class Quantifier:
    """A repetition count from ``low`` (at least 1) to ``high``, or unbounded when ``high`` is None."""

    low: int
    high: int | None

    def __post_init__(self) -> None:
        low = _count(self.low, "Quantifier low")
        if low < 1:
            raise ValueError(f"Quantifier low must be at least 1 (every token matches a character), got {low}")
        if self.high is not None and _count(self.high, "Quantifier high") < low:
            raise ValueError(f"Quantifier high ({self.high}) must not be below low ({low})")

    @classmethod
    def exactly(cls, n: int) -> Quantifier:
        return cls(n, n)

    @classmethod
    def at_least(cls, n: int) -> Quantifier:
        return cls(n, None)

    @property
    def bounded(self) -> bool:
        return self.high is not None

    def accepts(self, count: int) -> bool:
        """Whether a run of ``count`` repetitions is allowed."""
        return self.low <= count and (self.high is None or count <= self.high)

    def __add__(self, other: Quantifier) -> Quantifier:
        """The counts of two consecutive runs of one atom, as one run."""
        if not isinstance(other, Quantifier):
            return NotImplemented
        high = None if self.high is None or other.high is None else self.high + other.high
        return Quantifier(self.low + other.low, high)

    @property
    def source(self) -> str:
        """``""`` (once), ``{n}``, ``{n,m}``, ``+``, or ``{n,}``."""
        if self.high is None:
            return "+" if self.low == 1 else f"{{{self.low},}}"
        if self.high == self.low:
            return "" if self.low == 1 else f"{{{self.low}}}"
        return f"{{{self.low},{self.high}}}"

    def __str__(self) -> str:
        return self.source


ONE: Final = Quantifier(1, 1)
PLUS: Final = Quantifier(1, None)


# ---------------------------------------------------------------------------
# Tokens
# ---------------------------------------------------------------------------


def escape(ch: str) -> str:
    """One literal character as printed: a backslash before an ECMA-262 syntax character, else raw."""
    return "\\" + ch if ch in SYNTAX_CHARACTERS else ch


def _check_literal(ch: str) -> str:
    if len(ch) != 1:
        raise ValueError(f"a literal is exactly one code point, got {ch!r}")
    if _base_bit(ch):
        raise ValueError(f"the literal {ch!r} is an ASCII letter or digit; use a CharClass for it")
    if "\ud800" <= ch <= "\udfff":
        raise ValueError(f"the literal {ch!r} is a surrogate code point")
    return ch


@dataclass(frozen=True, slots=True)
class Token:
    """One atom, a :class:`CharClass` or a literal code point, repeated as ``quantifier`` says."""

    atom: CharClass | str
    quantifier: Quantifier = ONE

    def __post_init__(self) -> None:
        if isinstance(self.atom, str):
            _check_literal(self.atom)
        elif not isinstance(self.atom, CharClass):
            raise TypeError(f"a token atom must be a CharClass or a one-character str, got {type(self.atom).__name__}")
        if not isinstance(self.quantifier, Quantifier):
            raise TypeError(f"a token quantifier must be a Quantifier, got {type(self.quantifier).__name__}")

    @property
    def is_literal(self) -> bool:
        return isinstance(self.atom, str)

    @property
    def alphabet_size(self) -> int:
        """The number of characters one repetition can be: the class size, or 1 for a literal."""
        return self.atom.size if isinstance(self.atom, CharClass) else 1

    def accepts(self, ch: str) -> bool:
        """Whether one repetition can be the character ``ch``."""
        if isinstance(self.atom, CharClass):
            return self.atom.contains(ch)
        return ch == self.atom

    def overlaps(self, other: Token) -> bool:
        """Whether a repetition of each can be the same character."""
        if isinstance(self.atom, CharClass):
            return isinstance(other.atom, CharClass) and self.atom.overlaps(other.atom)
        return self.atom == other.atom

    def merge(self, other: Token) -> Token:
        """The token for this run followed by ``other``'s, when the two overlap."""
        if not self.overlaps(other):
            raise ValueError(f"cannot merge disjoint tokens {self.source!r} and {other.source!r}")
        if isinstance(self.atom, CharClass) and isinstance(other.atom, CharClass):
            return Token(self.atom.join(other.atom), self.quantifier + other.quantifier)
        return Token(self.atom, self.quantifier + other.quantifier)

    def run_end(self, s: str, pos: int) -> int:
        """Where the maximal run of this token's characters that starts at ``pos`` ends."""
        if isinstance(self.atom, CharClass):
            found = _CLASS_RUNS[self.atom](s, pos)
            assert found is not None, "a starred run always matches"
            return found.end()
        end, n = pos, len(s)
        while end < n and s[end] == self.atom:
            end += 1
        return end

    @property
    def source(self) -> str:
        """The printed token: a class with its quantifier, or a literal (in a group when quantified)."""
        if isinstance(self.atom, CharClass):
            return self.atom.value + self.quantifier.source
        if self.quantifier == ONE:
            return escape(self.atom)
        return f"(?:{escape(self.atom)}){self.quantifier.source}"


# ---------------------------------------------------------------------------
# Alternatives and Patterns
# ---------------------------------------------------------------------------

Span = tuple[int, int]

# The anchors around the printed disjunction (see Printer in the module docstring).
_SOURCE_PREFIX: Final = "^(?:"
_SOURCE_SUFFIX: Final = ")$(?!\\n)"


def _normal_form(tokens: Iterable[Token]) -> tuple[Token, ...]:
    """Merge overlapping adjacent tokens until adjacent tokens are disjoint."""
    merged: list[Token] = []
    for token in tokens:
        if not isinstance(token, Token):
            raise TypeError(f"an alternative holds Tokens, got {type(token).__name__}")
        while merged and merged[-1].overlaps(token):
            token = merged.pop().merge(token)
        merged.append(token)
    return tuple(merged)


@dataclass(frozen=True, slots=True)
class Alternative:
    """One sub-pattern: a sequence of tokens, stored in normal form (see the module docstring)."""

    tokens: tuple[Token, ...] = ()

    def __post_init__(self) -> None:
        if isinstance(self.tokens, (str, Token)):
            raise TypeError("Alternative takes an iterable of Tokens")
        object.__setattr__(self, "tokens", _normal_form(self.tokens))

    def match(self, s: str) -> tuple[Span, ...] | None:
        """The code-point span of each token's run when ``s`` matches as a whole, else None."""
        spans: list[Span] = []
        pos = 0
        for token in self.tokens:
            end = token.run_end(s, pos)
            if not token.quantifier.accepts(end - pos):
                return None
            spans.append((pos, end))
            pos = end
        return tuple(spans) if pos == len(s) else None

    @property
    def source(self) -> str:
        return "".join(token.source for token in self.tokens)


@dataclass(frozen=True, slots=True)
class Decomposition:
    """How a string matches a Pattern: the first matching alternative and each token's run."""

    alternative: int
    spans: tuple[Span, ...]

    @property
    def counts(self) -> tuple[int, ...]:
        """The repetition count of each token of the alternative (its run length in code points)."""
        return tuple(end - start for start, end in self.spans)


@dataclass(frozen=True, slots=True)
class Pattern:
    """A disjunction of one or more alternatives, matched as a whole string."""

    alternatives: tuple[Alternative, ...]

    def __post_init__(self) -> None:
        if isinstance(self.alternatives, (str, Alternative)):
            raise TypeError("Pattern takes an iterable of Alternatives")
        alternatives = tuple(self.alternatives)
        for alternative in alternatives:
            if not isinstance(alternative, Alternative):
                raise TypeError(f"a pattern holds Alternatives, got {type(alternative).__name__}")
        if not alternatives:
            raise ValueError("a pattern needs at least one alternative")
        object.__setattr__(self, "alternatives", tuple(dict.fromkeys(alternatives)))

    def match(self, s: str) -> Decomposition | None:
        """The unique decomposition of ``s``, or None when no alternative matches all of it."""
        for index, alternative in enumerate(self.alternatives):
            spans = alternative.match(s)
            if spans is not None:
                return Decomposition(index, spans)
        return None

    def matches(self, s: str) -> bool:
        return self.match(s) is not None

    @property
    def body(self) -> str:
        """The unanchored disjunction ``alt1|…|altk``."""
        return "|".join(alternative.source for alternative in self.alternatives)

    @property
    def source(self) -> str:
        """The emitted ``pattern`` keyword value: ``^(?:alt1|…|altk)$(?!\\n)``."""
        return f"{_SOURCE_PREFIX}{self.body}{_SOURCE_SUFFIX}"

    @classmethod
    def from_source(cls, source: str) -> Pattern:
        """The Pattern whose :attr:`source` is ``source``; ``ValueError`` for any other string."""
        if not isinstance(source, str):
            raise TypeError(f"a pattern source is a str, got {type(source).__name__}")
        if not (
            source.startswith(_SOURCE_PREFIX)
            and source.endswith(_SOURCE_SUFFIX)
            and len(source) >= len(_SOURCE_PREFIX) + len(_SOURCE_SUFFIX)
        ):
            raise ValueError(f"{source!r} is not anchored as ^(?:…)$(?!\\n)")
        pattern = cls(_parse_body(source[len(_SOURCE_PREFIX) : len(source) - len(_SOURCE_SUFFIX)]))
        if pattern.source != source:
            raise ValueError(f"{source!r} is not the printed form of a safe pattern (it prints as {pattern.source!r})")
        return pattern

    def __str__(self) -> str:
        return self.source

    def __canonical__(self) -> str:
        return self.source


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------

_CLASS_SOURCES: Final = tuple((cls.value, cls) for cls in _BY_SIZE)  # no class source is a prefix of another
_GROUP_OPEN: Final = "(?:"


def _parse_body(body: str) -> tuple[Alternative, ...]:
    """Read the alternatives of an unanchored printed disjunction (the printer's output, leniently).

    Accepts every string the printer writes and raises ``ValueError`` on
    input it cannot read as classes, literals, groups, and quantifiers.
    :meth:`Pattern.from_source` rejects the remaining non-canonical inputs by
    printing the result again.
    """
    alternatives: list[Alternative] = []
    tokens: list[Token] = []
    i, n = 0, len(body)
    while i < n:
        ch = body[i]
        if ch == "|":
            alternatives.append(Alternative(tuple(tokens)))
            tokens = []
            i += 1
        elif ch == "[":
            for text, cls in _CLASS_SOURCES:
                if body.startswith(text, i):
                    quantifier, i = _parse_quantifier(body, i + len(text))
                    tokens.append(Token(cls, quantifier))
                    break
            else:
                raise ValueError(f"unknown character class at index {i} of {body!r}")
        elif body.startswith(_GROUP_OPEN, i):
            literal, i = _parse_literal(body, i + len(_GROUP_OPEN))
            if not body.startswith(")", i):
                raise ValueError(f"a group holds one literal; expected ')' at index {i} of {body!r}")
            quantifier, i = _parse_quantifier(body, i + 1)
            tokens.append(Token(literal, quantifier))
        else:
            literal, i = _parse_literal(body, i)
            tokens.append(Token(literal))
    alternatives.append(Alternative(tuple(tokens)))
    return tuple(alternatives)


def _parse_literal(body: str, i: int) -> tuple[str, int]:
    """One printed literal at ``i`` (raw, or a backslash before a syntax character) and the index after it."""
    if i >= len(body):
        raise ValueError(f"expected a literal at the end of {body!r}")
    ch = body[i]
    if ch == "\\":
        if i + 1 < len(body) and body[i + 1] in SYNTAX_CHARACTERS:
            return body[i + 1], i + 2
        raise ValueError(f"only syntax characters are escaped; bad escape at index {i} of {body!r}")
    if ch in SYNTAX_CHARACTERS:
        raise ValueError(f"unescaped syntax character {ch!r} at index {i} of {body!r}")
    return ch, i + 1


def _parse_quantifier(body: str, i: int) -> tuple[Quantifier, int]:
    """The quantifier printed at ``i`` (``ONE`` when there is none) and the index after it."""
    if body.startswith("+", i):
        return PLUS, i + 1
    if not body.startswith("{", i):
        return ONE, i
    close = body.find("}", i)
    if close < 0:
        raise ValueError(f"unterminated quantifier at index {i} of {body!r}")
    low_text, comma, high_text = body[i + 1 : close].partition(",")
    low = _parse_count(low_text, body)
    if not comma:
        return Quantifier.exactly(low), close + 1
    if not high_text:
        return Quantifier.at_least(low), close + 1
    return Quantifier(low, _parse_count(high_text, body)), close + 1


def _parse_count(text: str, body: str) -> int:
    if not text or not text.isascii() or not text.isdigit():
        raise ValueError(f"bad repetition count {text!r} in {body!r}")
    return int(text)


# ---------------------------------------------------------------------------
# Induction (task 32.1; Requirements 14.1, 14.2, 14.7, 14.8)
# ---------------------------------------------------------------------------
#
# FlashProfile-style, as the design describes it:
#
# - :func:`tokenize` splits a string into atoms: maximal runs of ``[0-9]``,
#   ``[a-z]``, and ``[A-Z]``, and single literal characters (a run of one
#   literal is one token after normal form).
# - Strings are grouped into *profiles* by their atom sequence
#   (:func:`profiles`); a profile keeps every string's run lengths.
# - A profile yields one alternative per rung of the generality ladder
#   (:data:`LADDER`): exact run lengths when every string shares them,
#   ``{min,max}`` ranges, class widening (``[0-9]``/``[a-z]``/``[A-Z]`` to
#   ``[A-Za-z]`` to ``[0-9A-Za-z]``), and ``+``.
# - :class:`Induction` builds one candidate Pattern per rung: the
#   alternatives of every profile at that rung, merged greedily by the
#   smallest estimated description-length increase (:func:`estimate_bits`,
#   the MDL_Scorer's pattern coding without a view) until at most
#   ``max_alternatives`` remain. Two alternatives merge token by token
#   (classes join, equal literals stay, quantifiers take the union); a
#   pair whose atom sequences cannot be aligned does not merge, and a rung
#   that cannot reach the cap yields no candidate.
#
# The candidates are yielded from the most specific rung to the most
# general; duplicates (the same source) are yielded once. The caller
# checks each against all strings with the Validator (14.7) and stops at
# the attempt budget (14.8).


Atom = CharClass | str


@dataclass(frozen=True, slots=True)
class Profile:
    """The strings that share one atom sequence, with each string's run lengths per atom."""

    atoms: tuple[Atom, ...]
    runs: tuple[tuple[int, ...], ...]  # one tuple per string, in first-seen order
    count: int  # occurrences of the profile's strings, with multiplicity

    def low(self, i: int) -> int:
        return min(run[i] for run in self.runs)

    def high(self, i: int) -> int:
        return max(run[i] for run in self.runs)


def tokenize(s: str) -> tuple[Token, ...]:
    """The atoms of ``s`` with exact run lengths: maximal class runs and single literal characters."""
    tokens: list[Token] = []
    pos, n = 0, len(s)
    while pos < n:
        base = CharClass.base_of(s[pos])
        if base is not None:
            end = _CLASS_RUNS[base](s, pos).end()  # type: ignore[union-attr]
            tokens.append(Token(base, Quantifier.exactly(end - pos)))
            pos = end
        else:
            ch = s[pos]
            end = pos + 1
            while end < n and s[end] == ch:
                end += 1
            tokens.append(Token(ch, Quantifier.exactly(end - pos)))
            pos = end
    return tuple(tokens)


def profiles(strings: Iterable[str], counts: Iterable[int] | None = None) -> list[Profile]:
    """Group ``strings`` by atom sequence, in first-seen order (see the induction notes)."""
    counts_list = list(counts) if counts is not None else None
    found: dict[tuple[Atom, ...], tuple[list[tuple[int, ...]], int]] = {}
    for index, s in enumerate(strings):
        tokens = tokenize(s)
        key = tuple(token.atom for token in tokens)
        runs, count = found.setdefault(key, ([], 0))
        runs.append(tuple(token.quantifier.low for token in tokens))
        found[key] = (runs, count + (counts_list[index] if counts_list is not None else 1))
    return [Profile(atoms, tuple(runs), count) for atoms, (runs, count) in found.items()]


LADDER: Final = ("exact", "range", "widen", "plus")
_WIDEN: Final = {CharClass.DIGIT: CharClass.ALNUM, CharClass.LOWER: CharClass.ALPHA, CharClass.UPPER: CharClass.ALPHA}


def alternative_at(profile: Profile, rung: str) -> Alternative:
    """The alternative of ``profile`` at a rung of :data:`LADDER`."""
    if rung not in LADDER:
        raise ValueError(f"rung must be one of {LADDER}, got {rung!r}")
    tokens: list[Token] = []
    for i, atom in enumerate(profile.atoms):
        low, high = profile.low(i), profile.high(i)
        if rung == "exact":
            quantifier = Quantifier(low, high)  # exact when every string agrees, else the observed range
        elif rung == "range":
            quantifier = Quantifier(low, high)
        elif rung == "widen":
            quantifier = Quantifier(low, high)
            if isinstance(atom, CharClass):
                atom = _WIDEN.get(atom, atom)
        elif isinstance(atom, CharClass):
            quantifier = PLUS
            atom = _WIDEN.get(atom, atom)
        else:
            quantifier = Quantifier(low, high)  # a literal keeps its observed run lengths at every rung
        tokens.append(Token(atom, quantifier))
    return Alternative(tokens)


def _unique_lengths(profile: Profile) -> bool:
    return all(profile.low(i) == profile.high(i) for i in range(len(profile.atoms)))


def merge_alternatives(a: Alternative, b: Alternative) -> Alternative | None:
    """Merge two alternatives token by token (classes join, equal literals stay, quantifiers union), or None."""
    if len(a.tokens) != len(b.tokens):
        return None
    tokens: list[Token] = []
    for x, y in zip(a.tokens, b.tokens):
        if isinstance(x.atom, CharClass) and isinstance(y.atom, CharClass):
            atom: Atom = x.atom.join(y.atom)
        elif x.atom == y.atom:
            atom = x.atom
        else:
            return None
        low = min(x.quantifier.low, y.quantifier.low)
        high = None if x.quantifier.high is None or y.quantifier.high is None else max(x.quantifier.high, y.quantifier.high)
        tokens.append(Token(atom, Quantifier(low, high)))
    return Alternative(tokens)


def _log2(n: int) -> float:
    return math.log2(n) if n > 1 else 0.0


def _item_bits(alternative: Alternative, profile: Profile) -> float:
    """The bits of one alternative and its profile's strings, without the per-string alternative-choice term."""
    tokens = alternative.tokens
    model = 2.0 + sum(4.0 + _log2(t.quantifier.low + 1) + (0.0 if t.quantifier.high is None else _log2(t.quantifier.high - t.quantifier.low + 1)) for t in tokens)
    if not tokens:
        return model
    last = len(tokens) - 1
    class_bits = [_log2(t.atom.size) if isinstance(t.atom, CharClass) else 0.0 for t in tokens]
    bounded = [None if t.quantifier.high is None else _log2(t.quantifier.high - t.quantifier.low + 1) for t in tokens]
    lows = [t.quantifier.low for t in tokens]
    data = 0.0
    for run in profile.runs:
        # Normal form may fuse adjacent tokens after a class join, so a run is charged under the token
        # at its index, the last token absorbing the rest.
        for i, length in enumerate(run):
            j = i if i < last else last
            data += length * class_bits[j]
            fixed = bounded[j]
            data += fixed if fixed is not None else _log2(length - lows[j] + 2)
    return model + data


def estimate_bits(alternatives: Sequence[tuple[Alternative, Profile]]) -> float:
    """A description-length estimate of a Pattern over its profiles (the scorer's pattern coding, without a view).

    Model bits follow ``mdl.scorer.M_re``'s shape (a constant per token plus
    the quantifier counts); data bits are, per string, ``log2`` of the
    number of alternatives plus, per token, the run length times
    ``log2 |class|`` (0 for a literal) plus the count code of its quantifier.
    """
    k = len(alternatives)
    strings = sum(len(profile.runs) for _, profile in alternatives)
    return _log2(k + 1) + strings * _log2(k) + math.fsum(_item_bits(a, p) for a, p in alternatives)


_MAX_PROFILES: Final = 64  # profiles a rung may hold before the greedy merge is skipped (free text, not a pattern)


@dataclass(frozen=True, slots=True)
class Induced:
    """One induced candidate: the Pattern, its rung, and the number of profiles it merged into alternatives."""

    pattern: Pattern
    rung: str
    merged: int

    @property
    def sub_patterns(self) -> int:
        return len(self.pattern.alternatives)


class Induction:
    """Candidate Patterns of a set of strings, most specific first (see the induction notes)."""

    __slots__ = ("_max_alternatives", "_profiles")

    def __init__(self, strings: Iterable[str], counts: Iterable[int] | None = None, *, max_alternatives: int = 4) -> None:
        if isinstance(max_alternatives, bool) or not isinstance(max_alternatives, int) or max_alternatives < 1:
            raise ValueError(f"max_alternatives must be a positive int, got {max_alternatives!r}")
        self._profiles = profiles(strings, counts)
        self._max_alternatives = max_alternatives

    @property
    def profiles(self) -> list[Profile]:
        return list(self._profiles)

    def _merge_down(self, items: list[tuple[Alternative, Profile]]) -> list[tuple[Alternative, Profile]] | None:
        """Greedily merge the pair with the smallest estimated bit increase until the cap is met, or None.

        The increase of merging two items is the merged item's bits minus
        the two items' bits (the alternative-count terms change equally for
        every pair), so each pair is evaluated once and only the pairs of a
        newly merged item are evaluated again. More than
        :data:`_MAX_PROFILES` profiles is free text, not a pattern: None.
        """
        if len(items) > _MAX_PROFILES:
            return None
        live: dict[int, tuple[Alternative, Profile, float]] = {i: (a, p, _item_bits(a, p)) for i, (a, p) in enumerate(items)}
        order = list(range(len(items)))  # canonical order of the live items, for deterministic ties
        pairs: dict[tuple[int, int], tuple[float, Alternative] | None] = {}

        def evaluate(i: int, j: int) -> tuple[float, Alternative] | None:
            key = (i, j)
            if key not in pairs:
                a, pa, bits_a = live[i]
                b, pb, bits_b = live[j]
                merged = merge_alternatives(a, b)
                if merged is None:
                    pairs[key] = None
                else:
                    joint = Profile(pa.atoms, pa.runs + pb.runs, pa.count + pb.count)
                    pairs[key] = (_item_bits(merged, joint) - bits_a - bits_b, merged)
            return pairs[key]

        next_id = len(items)
        while len(order) > self._max_alternatives:
            best: tuple[float, int, int] | None = None
            for x in range(len(order)):
                for y in range(x + 1, len(order)):
                    found = evaluate(order[x], order[y])
                    if found is not None and (best is None or (found[0], x, y) < best):
                        best = (found[0], x, y)
            if best is None:
                return None
            _, x, y = best
            i, j = order[x], order[y]
            increase, merged = pairs[(i, j)]  # type: ignore[misc]
            pa, pb = live[i][1], live[j][1]
            joint = Profile(pa.atoms, pa.runs + pb.runs, pa.count + pb.count)
            live[next_id] = (merged, joint, _item_bits(merged, joint))
            del live[i], live[j]
            order = [o for o in order if o not in (i, j)] + [next_id]
            next_id += 1
        return [(live[o][0], live[o][1]) for o in order]

    def candidates(self) -> Iterator[Induced]:
        """Yield one candidate per rung of :data:`LADDER` that reaches the alternatives cap, distinct sources once."""
        if not self._profiles:
            return
        seen: set[str] = set()
        for rung in LADDER:
            if rung == "exact" and not all(_unique_lengths(p) for p in self._profiles):
                continue  # exact lengths exist only when every string of every profile agrees
            items = [(alternative_at(p, rung), p) for p in self._profiles]
            merged = self._merge_down(items)
            if merged is None:
                continue
            pattern = Pattern([alternative for alternative, _ in merged])
            if pattern.source in seen:
                continue
            seen.add(pattern.source)
            yield Induced(pattern, rung, len(self._profiles) - len(merged))


__all__ += ["LADDER", "Induced", "Induction", "Profile", "alternative_at", "estimate_bits", "merge_alternatives", "profiles", "tokenize"]
