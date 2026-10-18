"""Latent_Type detection in strings (Requirements 13.3, 13.9).

A Latent_Type is the type a string value encodes: integer, decimal number,
boolean, date, date-time, or time (:class:`LatentType`). When a Path has at
least one non-flagged string and every non-flagged string at the Path
encodes the same Latent_Type, the Value_Inferrer records it in an
``x-latent-type`` annotation, keeps ``string`` as the JSON type of those
values (``values.types``), and produces one ``string_constraint`` candidate
(13.3):

- the ``date``, ``date-time``, or ``time`` Format of the same name, when the
  Latent_Type is one of those three and the Validator, with format
  assertion on, accepts every such string for that Format;
- otherwise a Pattern from the safe regex AST (``values.patterns``) that
  matches every such string.

Integer and decimal-number strings at one Path count as the Latent_Type
decimal number. Any other mix, or one string that encodes no Latent_Type,
means the Path has none: no annotation and no candidate.

Classifier
    :func:`latent_type_of` gives each string at most one Latent_Type. Every
    grammar below is ASCII and must match the whole string (no surrounding
    whitespace, no trailing newline); ``D`` is an ASCII digit ``[0-9]``. The
    six grammars are disjoint, so the classification does not depend on the
    order in which they are tried.

    ``boolean``
        ``true`` or ``false`` in any ASCII letter case (``True``, ``FALSE``).
    ``integer``
        An optional sign ``+`` or ``-``, then ``0`` or a non-zero digit
        followed by digits: ``[+-]?(0|[1-9]D*)``. A zero-padded string such
        as ``007`` or ``02134`` is a code, not an integer, and encodes no
        Latent_Type.
    ``decimal``
        The integer grammar followed by a fraction ``.D+``, an exponent
        ``[eE][+-]?D+``, or both, as in JSON number syntax with an optional
        ``+``. The written form decides: ``"1.0"`` and ``"1e3"`` are decimal
        strings, unlike the JSON numbers ``1.0`` and ``1e3`` (13.2).
    ``date``
        ``YYYY s M s D`` or ``A s B s YYYY``, where ``s`` is one of ``-``,
        ``/``, ``.`` (the same separator twice), ``YYYY`` is four digits,
        and ``M``, ``D``, ``A``, ``B`` are one or two digits. The first form
        is year, month, day; the second is day-month-year or
        month-day-year, whichever is a calendar date (``15/03/2024`` and
        ``03/15/2024`` both qualify). The date must exist in the proleptic
        Gregorian calendar, years 0000 to 9999 (``2023-02-29`` does not).
    ``time``
        ``H:MM``, optionally followed by ``:SS`` and then optionally by a
        fraction ``.D+``, then optionally an offset ``Z``, ``z``,
        ``+HH:MM``, ``-HH:MM``, ``+HHMM``, or ``-HHMM``. ``H`` is one or two
        digits with value 0 to 23, minutes 00 to 59, seconds 00 to 60 (a
        leap second), offset hours 00 to 23 and minutes 00 to 59.
    ``date-time``
        A date, one separator ``T``, ``t``, or a space, and a time.

    Strings that satisfy the RFC 3339 Formats are a subset of these
    spellings; the others (``2024-3-15``, ``03/15/2024``, ``12:30``,
    ``2024-03-15 12:30:00``) still encode the Latent_Type and lead to a
    Pattern candidate. Whether a string satisfies a Format is decided by the
    Validator alone (:meth:`Validator.satisfies_format`, one cached check per
    distinct string), never by these grammars.

Pattern candidate
    :func:`latent_pattern` builds the Pattern from the distinct strings in
    code-point order. Each string is split into maximal runs of one atom:
    ``[0-9]``, ``[a-z]``, ``[A-Z]``, or one literal character (the
    FlashProfile atoms of ``values.patterns``). Strings with the same atom
    sequence form one alternative, and the alternatives come in the order of
    their first string. Each run becomes a token whose quantifier is the
    range of run lengths observed at that position, except that digit runs
    of integer and decimal-number strings are unbounded (``[0-9]+``),
    because magnitudes and precisions are open-ended. So ``2024-03-15`` and
    ``2023-12-01`` give ``^(?:[0-9]{4}-[0-9]{2}-[0-9]{2})$(?!\\n)``, and
    ``-5``, ``12``, ``7`` give ``^(?:-[0-9]+|[0-9]+)$(?!\\n)``. Adjacent runs
    have disjoint atoms, so a string's runs are its unique decomposition
    under its alternative and every string matches the Pattern by
    construction; the ValueInferrer still checks every candidate with the
    Validator (13.6).

Annotation and candidate
    The ``x-latent-type`` annotation is recorded whenever a Latent_Type is
    found, whichever ``string_constraint`` candidate the DecisionEngine
    later selects (13.3, 13.8), so it is not part of the candidate.
    :meth:`LatentDetection.to_candidate` builds the
    :class:`~jsinfer.decide.engine.Candidate` of the ``string_constraint``
    Decision_Point, with outcome ``(keyword, value)`` and label
    ``format=<name>`` or ``pattern=<source>``.

Technique state (13.9)
    While ``values.latent_type`` is disabled, :func:`detect_latent` and
    :func:`detect_latent_view` return None: no annotation and no candidate,
    the Fallback_Behavior. ``type`` (13.1, 13.2) and the other enabled
    string Techniques are unaffected. The Technique reads no
    Learned_Threshold: every decision here is a grammar match or a Validator
    verdict.

Determinism
    Results depend only on the multiset of non-flagged strings: distinct
    strings are processed in code-point order, counts are integers, and no
    order depends on ``hash()``.
"""

from __future__ import annotations

import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from enum import Enum
from typing import Any, Final, Literal, TypeAlias

from jsinfer.core.determinism import sorted_codepoint
from jsinfer.core.jsonvalue import JSON, JType
from jsinfer.core.provenance import Support
from jsinfer.core.schema_model import SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate
from jsinfer.io.validator import Validator
from jsinfer.values.patterns import PLUS, Alternative, CharClass, Pattern, Quantifier, Token

__all__ = [
    "ANNOTATION_KEYWORD",
    "STRING_CONSTRAINT",
    "TECHNIQUE",
    "LatentDetection",
    "LatentType",
    "common_latent_type",
    "detect_latent",
    "detect_latent_view",
    "latent_pattern",
    "latent_type_of",
    "view_strings",
]

TECHNIQUE: Final = "values.latent_type"
STRING_CONSTRAINT: Final = "string_constraint"  # the Decision_Point kind the candidate is offered at
ANNOTATION_KEYWORD: Final = "x-latent-type"

ConstraintKeyword: TypeAlias = Literal["format", "pattern"]


class LatentType(Enum):
    """The six Latent_Types; the value is the ``x-latent-type`` annotation value."""

    INTEGER = "integer"
    DECIMAL = "decimal"
    BOOLEAN = "boolean"
    DATE = "date"
    DATE_TIME = "date-time"
    TIME = "time"

    @property
    def format(self) -> str | None:
        """The Format of the same name for date, date-time, and time; None for the others."""
        return self.value if self in _TEMPORAL else None

    @property
    def numeric(self) -> bool:
        """True for integer and decimal number."""
        return self is LatentType.INTEGER or self is LatentType.DECIMAL


_TEMPORAL: Final = (LatentType.DATE, LatentType.DATE_TIME, LatentType.TIME)


# ---------------------------------------------------------------------------
# Classifier
# ---------------------------------------------------------------------------

_BOOLEAN_WORDS: Final = ("true", "false")
_INTEGER_RE: Final = re.compile(r"[+-]?(?:0|[1-9][0-9]*)")
_NUMBER_RE: Final = re.compile(r"[+-]?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")
_YMD_RE: Final = re.compile(r"([0-9]{4})([-/.])([0-9]{1,2})\2([0-9]{1,2})")
_XYY_RE: Final = re.compile(r"([0-9]{1,2})([-/.])([0-9]{1,2})\2([0-9]{4})")
_TIME_RE: Final = re.compile(
    r"([0-9]{1,2}):([0-9]{2})(?::([0-9]{2})(?:\.[0-9]+)?)?(?:[Zz]|[+-]([0-9]{2}):?([0-9]{2}))?"
)
# A date holds no T, t, or space and a time holds none either, so the separator is unique.
_DATE_TIME_RE: Final = re.compile(r"([0-9./-]+)[Tt ]([0-9:.+Zz-]+)")

_DAYS_IN_MONTH: Final = (31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)


def _calendar_date(year: int, month: int, day: int) -> bool:
    """Whether the date exists in the proleptic Gregorian calendar (year 0 is a leap year)."""
    if not 1 <= month <= 12 or day < 1:
        return False
    if month == 2 and year % 4 == 0 and (year % 100 != 0 or year % 400 == 0):
        return day <= 29
    return day <= _DAYS_IN_MONTH[month - 1]


def _is_date(s: str) -> bool:
    found = _YMD_RE.fullmatch(s)
    if found is not None:
        return _calendar_date(int(found[1]), int(found[3]), int(found[4]))
    found = _XYY_RE.fullmatch(s)
    if found is not None:
        a, b, year = int(found[1]), int(found[3]), int(found[4])
        return _calendar_date(year, b, a) or _calendar_date(year, a, b)
    return False


def _is_time(s: str) -> bool:
    found = _TIME_RE.fullmatch(s)
    if found is None:
        return False
    hour, minute, second, offset_hour, offset_minute = found.groups()
    if int(hour) > 23 or int(minute) > 59 or (second is not None and int(second) > 60):
        return False
    return offset_hour is None or (int(offset_hour) <= 23 and int(offset_minute) <= 59)


def _is_date_time(s: str) -> bool:
    found = _DATE_TIME_RE.fullmatch(s)
    return found is not None and _is_date(found[1]) and _is_time(found[2])


def latent_type_of(s: str) -> LatentType | None:
    """The Latent_Type the string ``s`` encodes, or None (see Classifier in the module docstring)."""
    if not isinstance(s, str):
        raise TypeError(f"latent_type_of takes a str, got {type(s).__name__}")
    if not s.isascii():
        return None  # every grammar is ASCII
    if s.lower() in _BOOLEAN_WORDS:
        return LatentType.BOOLEAN
    if _NUMBER_RE.fullmatch(s):
        return LatentType.INTEGER if _INTEGER_RE.fullmatch(s) else LatentType.DECIMAL
    if _is_time(s):
        return LatentType.TIME
    if _is_date(s):
        return LatentType.DATE
    if _is_date_time(s):
        return LatentType.DATE_TIME
    return None


def common_latent_type(strings: Iterable[str]) -> LatentType | None:
    """The Latent_Type every string encodes, with integer and decimal strings together as decimal (13.3).

    None when there are no strings, a string encodes no Latent_Type, or the
    strings encode different Latent_Types (other than integer and decimal).
    """
    seen: dict[LatentType, None] = {}
    for s in strings:
        latent = latent_type_of(s)
        if latent is None:
            return None
        seen[latent] = None
    if not seen:
        return None
    if all(latent.numeric for latent in seen):
        return LatentType.DECIMAL if LatentType.DECIMAL in seen else LatentType.INTEGER
    return next(iter(seen)) if len(seen) == 1 else None


# ---------------------------------------------------------------------------
# Pattern candidate
# ---------------------------------------------------------------------------

Atom: TypeAlias = CharClass | str


def _runs(s: str) -> list[tuple[Atom, int]]:
    """The maximal runs of one atom (``[0-9]``, ``[a-z]``, ``[A-Z]``, or one literal character) in ``s``."""
    runs: list[tuple[Atom, int]] = []
    for ch in s:
        atom: Atom = CharClass.base_of(ch) or ch
        if runs and runs[-1][0] == atom:
            runs[-1] = (atom, runs[-1][1] + 1)
        else:
            runs.append((atom, 1))
    return runs


def latent_pattern(strings: Iterable[str], latent: LatentType) -> Pattern:
    """The safe-AST Pattern candidate for strings of Latent_Type ``latent`` (see the module docstring).

    The Pattern matches every given string. Repeated strings and their order
    do not matter. Raises ``ValueError`` when there are no strings.
    """
    if not isinstance(latent, LatentType):
        raise TypeError(f"latent must be a LatentType, got {type(latent).__name__}")
    # Per atom sequence, the [low, high] run lengths at each position.
    groups: dict[tuple[Atom, ...], list[list[int]]] = {}
    for s in sorted_codepoint(dict.fromkeys(strings)):
        runs = _runs(s)
        profile = tuple(atom for atom, _ in runs)
        ranges = groups.get(profile)
        if ranges is None:
            groups[profile] = [[length, length] for _, length in runs]
            continue
        for bounds, (_, length) in zip(ranges, runs):
            bounds[0] = min(bounds[0], length)
            bounds[1] = max(bounds[1], length)
    if not groups:
        raise ValueError("a latent Pattern needs at least one string")
    alternatives = []
    for profile, ranges in groups.items():
        tokens = []
        for atom, (low, high) in zip(profile, ranges):
            open_ended = latent.numeric and atom is CharClass.DIGIT
            tokens.append(Token(atom, PLUS if open_ended else Quantifier(low, high)))
        alternatives.append(Alternative(tuple(tokens)))
    return Pattern(tuple(alternatives))


# ---------------------------------------------------------------------------
# Detection
# ---------------------------------------------------------------------------


@dataclass(frozen=True, eq=False, slots=True)
class LatentDetection:
    """The Latent_Type of a Path and the ``string_constraint`` candidate it produces (13.3).

    ``keyword`` and ``value`` are the candidate Constraint: ``format`` with
    the Format name, or ``pattern`` with the printed Pattern (``pattern``
    holds its AST). ``n_strings`` counts the non-flagged string
    Observations and ``n_distinct`` the distinct strings. For a date,
    date-time, or time Latent_Type with a Pattern candidate,
    ``format_rejects`` is the number of string Observations the Format
    rejected; it is 0 otherwise.
    """

    latent_type: LatentType
    keyword: ConstraintKeyword
    value: str
    pattern: Pattern | None
    n_strings: int
    n_distinct: int
    format_rejects: int = 0

    @property
    def annotation(self) -> tuple[str, str]:
        """The ``x-latent-type`` annotation as ``(keyword, value)``."""
        return ANNOTATION_KEYWORD, self.latent_type.value

    @property
    def constraint(self) -> dict[str, str]:
        """The candidate Constraint as schema keywords, ``{"format": …}`` or ``{"pattern": …}``."""
        return {self.keyword: self.value}

    @property
    def label(self) -> str:
        """The candidate's outcome label: ``format=<name>`` or ``pattern=<source>``."""
        return f"{self.keyword}={self.value}"

    @property
    def support(self) -> Support:
        """Every non-flagged string encodes the Latent_Type and satisfies the candidate (26.2)."""
        return Support(self.n_strings, self.n_strings)

    def details(self) -> dict[str, JSON]:
        """Provenance details: the Latent_Type, string counts, and any Format rejections."""
        out: dict[str, JSON] = {
            "latent_type": self.latent_type.value,
            "strings": self.n_strings,
            "distinct_strings": self.n_distinct,
        }
        fmt = self.latent_type.format
        if fmt is not None and self.keyword == "pattern":
            out["format"] = fmt
            out["format_rejects"] = self.format_rejects
        return out

    def node(self, base: Mapping[str, Any] | None = None) -> SchemaNode:
        """A fresh node holding the ``base`` keywords (for example ``type``) and the candidate Constraint."""
        kw = dict(base or {})
        kw[self.keyword] = self.value
        return SchemaNode(kw=kw)

    def to_candidate(self, base: Mapping[str, Any] | None = None, *, order_key: tuple[Any, ...] = (0,)) -> Candidate:
        """The ``string_constraint`` Candidate of ``values.latent_type`` (outcome ``(keyword, value)``)."""
        return Candidate(
            technique=TECHNIQUE,
            outcome=(self.keyword, self.value),
            label=self.label,
            order_key=order_key,
            node=self.node(base),
        )


def _tally(strings: Iterable[str], counts: Sequence[int] | None) -> dict[str, int]:
    tally: dict[str, int] = {}
    if counts is None:
        for s in strings:
            if not isinstance(s, str):
                raise TypeError(f"strings must be str, got {type(s).__name__}")
            tally[s] = tally.get(s, 0) + 1
        return tally
    strings = list(strings)
    if len(strings) != len(counts):
        raise ValueError(f"counts must align with strings: {len(counts)} counts for {len(strings)} strings")
    for s, count in zip(strings, counts):
        if not isinstance(s, str):
            raise TypeError(f"strings must be str, got {type(s).__name__}")
        if isinstance(count, bool) or not isinstance(count, int) or count < 1:
            raise ValueError(f"each count must be an int >= 1, got {count!r}")
        tally[s] = tally.get(s, 0) + count
    return tally


def detect_latent(
    strings: Iterable[str],
    validator: Validator,
    *,
    counts: Sequence[int] | None = None,
    enabled: bool = True,
) -> LatentDetection | None:
    """Detect the common Latent_Type of non-flagged strings and produce its candidate (13.3, 13.9).

    ``strings`` are the non-flagged strings at a Path, repeated as often as
    they occur, or distinct with their occurrence ``counts``. Returns None
    when ``values.latent_type`` is disabled (``enabled`` false), when there
    are no strings, or when they share no Latent_Type.
    """
    if not isinstance(validator, Validator):
        raise TypeError(f"validator must be a Validator, got {type(validator).__name__}")
    if not enabled:
        return None  # 13.9: the Fallback_Behavior produces no annotation and no candidate
    tally = _tally(strings, counts)
    distinct = sorted_codepoint(tally)
    latent = common_latent_type(distinct)
    if latent is None:
        return None
    n_strings = sum(tally[s] for s in distinct)
    fmt = latent.format
    rejects = 0
    if fmt is not None:
        rejects = sum(tally[s] for s in distinct if not validator.satisfies_format(fmt, s))
        if rejects == 0:
            return LatentDetection(latent, "format", fmt, None, n_strings, len(distinct))
    pattern = latent_pattern(distinct, latent)
    return LatentDetection(latent, "pattern", pattern.source, pattern, n_strings, len(distinct), rejects)


def view_strings(view: View) -> tuple[list[str], list[int]]:
    """The distinct non-flagged strings of a view in code-point order, with their occurrence counts."""
    if not isinstance(view, View):
        raise TypeError(f"view_strings takes a View, got {type(view).__name__}")
    stats = view.stats()
    table = view.store.values
    strings: list[str] = []
    counts: list[int] = []
    # Value ids follow canonical value order, which orders strings by code point.
    for vid, count in zip(stats.value_ids.tolist(), stats.value_counts.tolist()):
        rank, payload = table.key(vid)
        if rank == JType.STRING:
            assert isinstance(payload, str)
            strings.append(payload)
            counts.append(int(count))
    return strings, counts


def detect_latent_view(view: View, validator: Validator, *, enabled: bool = True) -> LatentDetection | None:
    """:func:`detect_latent` over the non-flagged strings of a scalar view."""
    if not enabled:
        return None
    strings, counts = view_strings(view)
    return detect_latent(strings, validator, counts=counts)
