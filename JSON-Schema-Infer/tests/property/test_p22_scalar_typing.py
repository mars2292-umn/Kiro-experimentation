"""Property 22: Scalar typing (task 12.6).

For any scalar Path, :meth:`ValueInferrer.infer_scalar` gives:

- a ``type`` listing exactly the JSON types of the non-flagged values, with
  ``integer`` used if and only if every number has a zero fractional part
  (``1.0``, ``-0.0``, ``1e3``, and integers beyond 64 bits included) and
  never together with ``number`` (13.1, 13.2);
- an ``x-latent-type`` equal to the common Latent_Type of the strings under
  a reference classifier written here from the grammars of the
  ``values.latent`` module docstring, with a mix of integer and decimal
  strings reported as decimal (the per-string classification it is built
  from is compared with the reference too), together with the 13.3 candidate: the
  ``date``, ``date-time``, or ``time`` Format when the Validator accepts
  every string for it, and otherwise a Pattern the Validator matches against
  every string (13.3);
- Format candidates that are exactly the Formats of 13.4 that at least one
  string satisfies, by the Validator's verdict with format assertion on
  (13.4).

Documents are JSON texts parsed with :func:`parse_document`, so numbers keep
their spelling up to the parser (``1E+3``, ``100e-2``, ``-0``, ``1e300``,
``2**80``). Which numbers are integral is decided on the literal with
:class:`decimal.Decimal`. Float spellings have at most 12 significant digits
and magnitudes within the double range, so a literal is integral exactly when
its parsed value is, and the reference reads the requirement's "zero
fractional part" from the text itself. Strings come from generators of every
Latent_Type (RFC 3339 and looser spellings), near-misses and edited latent
strings, Format-shaped strings, and arbitrary text; the test runs once per
string kind, so each gets the profile's full example budget (a fixed-seed
run of 100 mixed examples reaches some kinds only a few times). Each collection holds one
to three independent scalar Paths, checked with one ValueInferrer (so one
FormatDetector serves them all). Some documents lack a Path and some are
flagged; only non-flagged values count. The two Techniques
of 13.3 and 13.4 are mostly enabled; their clauses apply when they are
enabled, and a disabled one contributes nothing (13.9).

The Format and Pattern verdicts of the reference come from a second
:class:`Validator`, so they share no cache with the inferrer's.

**Validates: Requirements 13.1, 13.2, 13.3, 13.4**
"""

# Feature: json-schema-inference, Property 22: Scalar typing

from __future__ import annotations

import calendar
import json
from collections.abc import Callable, Iterable
from decimal import Decimal
from typing import Any

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.core.observations import build_store
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.io.ingest import parse_document
from jsinfer.io.validator import Validator
from jsinfer.values.inferrer import FORMAT, LATENT_TYPE, ValueInferrer
from jsinfer.values.latent import latent_type_of

# The inferrer's Validator, shared across examples so each check compiles once.
VALIDATOR = Validator()
# The reference's Validator: real jsonschema-rs verdicts, with format assertion on.
REFERENCE = Validator()

# The ten Formats of Requirement 13.4.
TEN_FORMATS = ("date-time", "date", "time", "duration", "email", "hostname", "ipv4", "ipv6", "uri", "uuid")
FORMAT_SCHEMAS = {fmt: REFERENCE.compile({"format": fmt}) for fmt in TEN_FORMATS}
TEMPORAL = ("date", "date-time", "time")

ABSENT = None  # a document without the Path


def satisfies(fmt: str, s: str) -> bool:
    """The reference Format verdict: ``{"format": fmt}`` accepts ``s`` under the Validator."""
    return REFERENCE.is_valid(FORMAT_SCHEMAS[fmt], s)


# ---------------------------------------------------------------------------
# Reference Latent_Type classifier (grammars of the values.latent docstring)
# ---------------------------------------------------------------------------

_ASCII_DIGITS = frozenset("0123456789")


def _digits(s: str, min_len: int = 1, max_len: int | None = None) -> bool:
    """``s`` is ASCII digits only, with a length in ``[min_len, max_len]``."""
    if len(s) < min_len or (max_len is not None and len(s) > max_len):
        return False
    return all(ch in _ASCII_DIGITS for ch in s)


def _unsigned(s: str) -> str:
    return s[1:] if s[:1] in ("+", "-") else s


def ref_boolean(s: str) -> bool:
    """``true`` or ``false`` in any ASCII letter case."""
    return any(
        len(s) == len(word) and all(ch in (c, c.upper()) for ch, c in zip(s, word)) for word in ("true", "false")
    )


def ref_integer(s: str) -> bool:
    """``[+-]?(0|[1-9]D*)``: no zero padding."""
    body = _unsigned(s)
    return _digits(body) and (body == "0" or body[0] != "0")


def ref_decimal(s: str) -> bool:
    """An integer followed by ``.D+``, ``[eE][+-]?D+``, or both."""
    mantissa, exponent = s, None
    for i, ch in enumerate(s):
        if ch in ("e", "E"):
            mantissa, exponent = s[:i], s[i + 1 :]
            break
    whole, dot, fraction = mantissa.partition(".")
    if not ref_integer(whole) or (dot and not _digits(fraction)):
        return False
    if exponent is not None and not _digits(_unsigned(exponent)):
        return False
    return bool(dot) or exponent is not None


def _calendar_date(year: int, month: int, day: int) -> bool:
    """A date of the proleptic Gregorian calendar (``calendar`` maps year 0 to a leap year)."""
    return 1 <= month <= 12 and 1 <= day <= calendar.monthrange(year, month)[1]


def ref_date(s: str) -> bool:
    """``YYYY s M s D`` or ``A s B s YYYY`` (day-month or month-day), one separator from ``-/.``."""
    for sep in "-/.":
        parts = s.split(sep)
        if len(parts) != 3 or not all(_digits(part) for part in parts):
            continue
        a, b, c = parts
        if len(a) == 4 and len(b) <= 2 and len(c) <= 2:
            return _calendar_date(int(a), int(b), int(c))
        if len(c) == 4 and len(a) <= 2 and len(b) <= 2:
            return _calendar_date(int(c), int(b), int(a)) or _calendar_date(int(c), int(a), int(b))
    return False


def _offset(digits: str) -> bool:
    """``HH:MM`` or ``HHMM`` (after the sign), hours 00–23 and minutes 00–59."""
    if len(digits) == 5 and digits[2] == ":":
        hours, minutes = digits[:2], digits[3:]
    elif len(digits) == 4:
        hours, minutes = digits[:2], digits[2:]
    else:
        return False
    return _digits(hours) and _digits(minutes) and int(hours) <= 23 and int(minutes) <= 59


def ref_time(s: str) -> bool:
    """``H:MM[:SS[.D+]]`` then an optional ``Z``, ``z``, or ``±HH:MM``/``±HHMM`` offset."""
    clock = s
    if s.endswith(("Z", "z")):
        clock = s[:-1]
    else:
        signs = [i for i, ch in enumerate(s) if ch in "+-"]
        if signs:
            if not _offset(s[signs[0] + 1 :]):
                return False
            clock = s[: signs[0]]
    fields = clock.split(":")
    if len(fields) not in (2, 3):
        return False
    hour, minute = fields[0], fields[1]
    if not (_digits(hour, 1, 2) and int(hour) <= 23 and _digits(minute, 2, 2) and int(minute) <= 59):
        return False
    if len(fields) == 3:
        second, dot, fraction = fields[2].partition(".")
        if not (_digits(second, 2, 2) and int(second) <= 60) or (dot and not _digits(fraction)):
            return False
    return True


def ref_date_time(s: str) -> bool:
    """A date, one of ``T``, ``t``, or a space, and a time."""
    return any(ch in "Tt " and ref_date(s[:i]) and ref_time(s[i + 1 :]) for i, ch in enumerate(s))


GRAMMARS: tuple[tuple[str, Callable[[str], bool]], ...] = (
    ("boolean", ref_boolean),
    ("integer", ref_integer),
    ("decimal", ref_decimal),
    ("date", ref_date),
    ("time", ref_time),
    ("date-time", ref_date_time),
)


def ref_latent_type(s: str) -> str | None:
    found = [name for name, grammar in GRAMMARS if grammar(s)]
    assert len(found) <= 1, f"the reference grammars overlap on {s!r}: {found}"
    return found[0] if found else None


def ref_common_latent_type(strings: Iterable[str]) -> str | None:
    """The Latent_Type every string encodes; integer and decimal strings together are decimal (13.3)."""
    kinds = {ref_latent_type(s) for s in strings}
    if not kinds or None in kinds:
        return None
    if kinds <= {"integer", "decimal"}:
        return "decimal" if "decimal" in kinds else "integer"
    return kinds.pop() if len(kinds) == 1 else None


# ---------------------------------------------------------------------------
# Strategies: number literals
# ---------------------------------------------------------------------------

_DIGIT_CHARS = "0123456789"

_INTEGER_LITERALS = st.one_of(
    st.integers(min_value=-1000, max_value=1000).map(str),
    st.integers(min_value=-(2**80), max_value=2**80).map(str),
    st.sampled_from(["-0", str(2**63), str(2**64), str(-(2**63) - 1), str(10**30)]),
)


@st.composite
def _float_literals(draw: st.DrawFn, integral: bool) -> str:
    """Number literals with a fraction or an exponent: at most 12 significant digits, |x| < 1e306."""
    sign = draw(st.sampled_from(["", "-"]))
    whole = draw(st.integers(min_value=0, max_value=999_999))
    if integral:
        form = draw(st.sampled_from(["zero fraction", "positive exponent", "trailing zeros"]))
        if form == "zero fraction":  # 1.0, -0.00, 7.0e2
            text = f"{whole}.{'0' * draw(st.integers(1, 4))}" + draw(st.sampled_from(["", "e0", "E+2", "e5"]))
        elif form == "positive exponent":  # 1e3, 1.25E+2
            fraction = draw(st.text(_DIGIT_CHARS, max_size=4))
            exponent = draw(st.integers(min_value=len(fraction), max_value=300))
            marker = draw(st.sampled_from(["e", "E", "e+", "E+"]))
            text = f"{whole}" + (f".{fraction}" if fraction else "") + f"{marker}{exponent}"
        else:  # 100e-2
            k = draw(st.integers(0, 6))
            text = f"{whole * 10**k}e-{draw(st.integers(0, k))}"
    else:
        form = draw(st.sampled_from(["fraction", "negative exponent"]))
        if form == "fraction":  # 2.5, -0.05, 1.25e1, 3.5E-300: the last fraction digit stays fractional
            fraction = draw(st.text(_DIGIT_CHARS, max_size=5)) + draw(st.sampled_from("123456789"))
            text = f"{whole}.{fraction}"
            if draw(st.booleans()):
                exponent = draw(st.integers(min_value=-300, max_value=len(fraction) - 1))
                plus = "+" if exponent >= 0 and draw(st.booleans()) else ""
                text += draw(st.sampled_from(["e", "E"])) + plus + str(exponent)
        else:  # 5e-1, 1234E-2: the last digit is not 0
            mantissa = 10 * draw(st.integers(min_value=0, max_value=99_999)) + draw(st.integers(1, 9))
            text = f"{mantissa}" + draw(st.sampled_from(["e-", "E-"])) + str(draw(st.integers(1, 300)))
    return sign + text


def number_literals(integral_only: bool) -> st.SearchStrategy[str]:
    if integral_only:
        return st.one_of(_INTEGER_LITERALS, _float_literals(True))
    return st.one_of(_INTEGER_LITERALS, _float_literals(True), _float_literals(False))


def is_integral(literal: str) -> bool:
    """Whether a JSON number literal has a zero fractional part (exact decimal arithmetic)."""
    value = Decimal(literal)
    return value == value.to_integral_value()


# ---------------------------------------------------------------------------
# Strategies: strings
# ---------------------------------------------------------------------------

_SIGNS = st.sampled_from(["", "+", "-"])


@st.composite
def integer_strings(draw: st.DrawFn) -> str:
    n = draw(st.integers(0, 999) | st.integers(0, 10**25))
    return draw(_SIGNS) + str(n)


@st.composite
def decimal_strings(draw: st.DrawFn) -> str:
    whole = draw(integer_strings())
    fraction = draw(st.none() | st.text(_DIGIT_CHARS, min_size=1, max_size=4))
    exponent = draw(
        st.none()
        | st.tuples(st.sampled_from(["e", "E"]), _SIGNS, st.text(_DIGIT_CHARS, min_size=1, max_size=3)).map("".join)
    )
    if fraction is None and exponent is None:
        fraction = "0"
    return whole + (f".{fraction}" if fraction is not None else "") + (exponent or "")


@st.composite
def boolean_strings(draw: st.DrawFn) -> str:
    word = draw(st.sampled_from(["true", "false"]))
    upper = draw(st.lists(st.booleans(), min_size=len(word), max_size=len(word)))
    return "".join(c.upper() if u else c for c, u in zip(word, upper))


@st.composite
def date_strings(draw: st.DrawFn, rfc3339: bool | None = None) -> str:
    year = draw(st.integers(0, 9999))
    month = draw(st.integers(1, 12))
    day = draw(st.integers(1, calendar.monthrange(year, month)[1]))
    if rfc3339 is None:
        rfc3339 = draw(st.booleans())
    if rfc3339:
        return f"{year:04d}-{month:02d}-{day:02d}"
    sep = draw(st.sampled_from("-/."))
    m = draw(st.sampled_from([str(month), f"{month:02d}"]))
    d = draw(st.sampled_from([str(day), f"{day:02d}"]))
    y = f"{year:04d}"
    parts = draw(st.sampled_from([(y, m, d), (d, m, y), (m, d, y)]))
    return sep.join(parts)


@st.composite
def time_strings(draw: st.DrawFn, rfc3339: bool | None = None) -> str:
    hour, minute, second = draw(st.integers(0, 23)), draw(st.integers(0, 59)), draw(st.integers(0, 60))
    fraction = draw(st.none() | st.text(_DIGIT_CHARS, min_size=1, max_size=3))
    offset_hour, offset_minute = draw(st.integers(0, 23)), draw(st.integers(0, 59))
    sign = draw(st.sampled_from("+-"))
    if rfc3339 is None:
        rfc3339 = draw(st.booleans())
    if rfc3339:
        offset = draw(st.sampled_from(["Z", "z", f"{sign}{offset_hour:02d}:{offset_minute:02d}"]))
        return f"{hour:02d}:{minute:02d}:{second:02d}" + (f".{fraction}" if fraction else "") + offset
    text = draw(st.sampled_from([str(hour), f"{hour:02d}"])) + f":{minute:02d}"
    if draw(st.booleans()):
        text += f":{second:02d}" + (f".{fraction}" if fraction else "")
    offset = draw(
        st.sampled_from(
            ["", "Z", "z", f"{sign}{offset_hour:02d}:{offset_minute:02d}", f"{sign}{offset_hour:02d}{offset_minute:02d}"]
        )
    )
    return text + offset


@st.composite
def date_time_strings(draw: st.DrawFn, rfc3339: bool | None = None) -> str:
    if rfc3339 is None:
        rfc3339 = draw(st.booleans())
    sep = draw(st.sampled_from("Tt")) if rfc3339 else draw(st.sampled_from("Tt "))
    return draw(date_strings(rfc3339)) + sep + draw(time_strings(rfc3339))


def latent_strings(rfc3339: bool | None = None) -> dict[str, st.SearchStrategy[str]]:
    """String strategies per Latent_Type; ``rfc3339=True`` keeps temporal strings in RFC 3339 form."""
    return {
        "integer": integer_strings(),
        "decimal": decimal_strings(),
        "boolean": boolean_strings(),
        "date": date_strings(rfc3339),
        "time": time_strings(rfc3339),
        "date-time": date_time_strings(rfc3339),
        "numeric": st.one_of(integer_strings(), decimal_strings()),  # the integer–decimal mix
    }


LATENT_STRINGS = latent_strings()

_NEAR_MISSES = [
    "", " 1", "1 ", "1\n", "007", "00", "-", "+", "1.", ".5", "1e", "1,000", "NaN", "Infinity", "0x1F",
    "\u0661\u0662", "\uff11\uff12", "yes", "t", "truee", "fal\u017fe", "true\n",
    "2023-02-29", "1900-02-29", "2024-13-01", "2024-00-10", "2024-01-32", "13/13/2024", "2024-03/15", "24-03-15",
    "24:00", "12:60", "12:5", "12:30:61", "12:30.5", "12:30:00+24:00", "12:30:00+01:60", "12:30:00+1:00",
    "2024-03-15X12:30", "2024-03-15T", "2024-03-15T25:00", "2024-03-15  12:30", "2024-02-30T12:30:00Z",
]


@st.composite
def edited_latent_strings(draw: st.DrawFn) -> str:
    """A latent string with one edit, which usually (not always) breaks its grammar."""
    return edit_string(draw, draw(st.one_of(*LATENT_STRINGS.values())))


def edit_string(draw: st.DrawFn, s: str) -> str:
    """``s`` with one edit: padding, a trailing newline or letter, a non-ASCII digit, or a dropped character."""
    edit = draw(st.sampled_from(["zero pad", "lead space", "trail space", "newline", "non-ASCII digit", "drop", "x"]))
    if edit == "lead space":
        return " " + s
    if edit == "trail space":
        return s + " "
    if edit == "newline":
        return s + "\n"
    if edit == "zero pad":
        return "0" + s
    if edit == "non-ASCII digit":
        i = next((i for i, ch in enumerate(s) if ch in _ASCII_DIGITS), None)
        return s if i is None else s[:i] + "\u0663" + s[i + 1 :]
    if edit == "drop":
        return s[:-1]
    return s + "x"


_FORMAT_SAMPLES = [
    "a@example.com", "user.name+tag@sub.example.org", "not@", "example.com", "localhost", "a-b.example",
    "-bad.example", "192.168.0.1", "0.0.0.0", "256.1.1.1", "::1", "2001:db8::8a2e:370:7334", "fe80::1%eth0",
    "https://example.com/a?b=c#d", "urn:isbn:0451450523", "mailto:a@example.com", "/relative/path",
    "P1D", "PT1H30M", "P3Y6M4DT12H30M5S", "P1W", "PT", "P", "123e4567-e89b-12d3-a456-426614174000",
    "123E4567-E89B-12D3-A456-426614174000", "123e4567e89b12d3a456426614174000",
]

FORMAT_STRINGS = st.one_of(
    st.sampled_from(_FORMAT_SAMPLES),
    st.ip_addresses().map(str),
    st.uuids().map(str),
)

OTHER_STRINGS = {
    "near-miss": st.sampled_from(_NEAR_MISSES),
    "edited": edited_latent_strings(),
    "format": FORMAT_STRINGS,
    "text": st.text(max_size=4),
}
# Per Path, temporal strings are either all RFC 3339 (so the latent Format can win) or mixed spellings.
STRING_KINDS = {rfc3339: {**latent_strings(rfc3339), **OTHER_STRINGS} for rfc3339 in (True, None)}


@st.composite
def scalar_paths(draw: st.DrawFn, kind: str) -> list[str | None]:
    """The values at one scalar Path, in document order: a JSON literal or ABSENT per document.

    Strings come from ``kind``, and at one Path in four also from a second
    kind. Numbers follow a theme: none, integral only, at least one
    fractional, or any mix.
    """
    kinds = {kind}
    if not draw(st.integers(0, 3)):
        kinds.add(draw(st.sampled_from(sorted(STRING_KINDS[True]))))
    kinds = sorted(kinds)
    by_kind = STRING_KINDS[draw(st.sampled_from([True, None]))]
    strings: list[str] = []
    if draw(st.integers(0, 4)):  # one Path in five has no strings
        strings = draw(st.lists(st.one_of(*(by_kind[kind] for kind in kinds)), min_size=1, max_size=6))
        if not draw(st.integers(0, 2)):  # a near-miss of one of the Path's own strings
            strings.append(edit_string(draw, draw(st.sampled_from(strings))))
    theme = draw(st.sampled_from(["integral", "fractional", "mixed", "integral", "fractional", "none"]))
    numbers: list[str] = []
    if theme == "integral":  # integer unless flags leave no number
        numbers = draw(st.lists(number_literals(integral_only=True), min_size=1, max_size=4))
    elif theme == "fractional":  # at least one number with a fractional part
        numbers = draw(st.lists(number_literals(integral_only=False), max_size=3)) + [draw(_float_literals(False))]
    elif theme == "mixed":
        numbers = draw(st.lists(number_literals(integral_only=False), max_size=4))
    booleans = draw(st.lists(st.sampled_from(["true", "false"]), max_size=2))
    nulls = draw(st.lists(st.just("null"), max_size=2))
    absent = [ABSENT] * draw(st.integers(0, 2))
    literals: list[str | None] = [json.dumps(s) for s in strings] + numbers + booleans + nulls + absent
    return draw(st.permutations(literals))


@st.composite
def collections(draw: st.DrawFn, kind: str) -> tuple[list[list[str | None]], list[bool]]:
    """One to three independent scalar Paths ``$.p0``, ``$.p1``, … and a flag per document.

    Document ``i`` holds the ``i``-th literal of each Path that has one.
    """
    paths = draw(st.lists(scalar_paths(kind), min_size=1, max_size=3))
    n_docs = max(len(literals) for literals in paths)
    flagged = draw(st.lists(st.sampled_from([False, False, False, True]), min_size=n_docs, max_size=n_docs))
    return paths, flagged


TECHNIQUE_STATES = st.sampled_from([True, True, True, False])


# ---------------------------------------------------------------------------
# The property
# ---------------------------------------------------------------------------


def literal_type(literal: str) -> str:
    """The JSON type of a literal, with every number as ``number``."""
    if literal == "null":
        return "null"
    if literal in ("true", "false"):
        return "boolean"
    if literal.startswith('"'):
        return "string"
    return "number"


def type_list(kw: dict[str, Any]) -> list[str]:
    value = kw.get("type", [])
    return [value] if isinstance(value, str) else list(value)


def document_text(paths: list[list[str | None]], i: int) -> bytes:
    """The JSON text of document ``i``: one member per Path with a literal there."""
    members = [
        f'"p{k}": {literals[i]}' for k, literals in enumerate(paths) if i < len(literals) and literals[i] is not ABSENT
    ]
    return ("{" + ", ".join(members) + "}").encode()


# One run per string kind, so every Latent_Type (and every kind of near-miss) gets the full example budget.
@pytest.mark.parametrize("kind", sorted(STRING_KINDS[True]))
@given(data=st.data(), latent_on=TECHNIQUE_STATES, format_on=TECHNIQUE_STATES)
def test_scalar_typing(kind: str, data: st.DataObject, latent_on: bool, format_on: bool) -> None:
    paths, flagged = data.draw(collections(kind), label="collection")
    docs = [parse_document(document_text(paths, i))[0] for i in range(len(flagged))]
    store = build_store(docs)
    if any(flagged):
        store.flags.flag_documents([i for i, is_flagged in enumerate(flagged) if is_flagged])
    root = View.root(store)

    states = {LATENT_TYPE: latent_on, FORMAT: format_on}
    inferrer = ValueInferrer.for_techniques(states, report=InferenceReport(), validator=VALIDATOR)
    for k, literals in enumerate(paths):
        key = f"p{k}"
        # The non-flagged values at the Path: (literal, parsed value).
        live = [
            (literal, docs[i][key])
            for i, literal in enumerate(literals)
            if literal is not ABSENT and not flagged[i]
        ]
        check_path(inferrer, root.child_key(key), live, latent_on=latent_on, format_on=format_on)


def check_path(
    inferrer: ValueInferrer, view: View, live: list[tuple[str, Any]], *, latent_on: bool, format_on: bool
) -> None:
    """Property 22 at one scalar Path whose non-flagged values are ``live``."""
    result = inferrer.infer_scalar(view)
    kw = result.keywords()

    # 13.1, 13.2: exactly the observed types; integer iff every number has a zero fractional part.
    numbers = [literal for literal, _ in live if literal_type(literal) == "number"]
    expected_types = {literal_type(literal) for literal, _ in live} - {"number"}
    if numbers:
        expected_types.add("integer" if all(is_integral(literal) for literal in numbers) else "number")
    names = type_list(kw)
    assert len(names) == len(set(names)), f"type lists a name twice: {names}"
    assert set(names) == expected_types, f"type {names} for the literals {[literal for literal, _ in live]}"
    assert not {"integer", "number"} <= set(names)
    if not live:
        assert "type" not in kw
    else:
        # The Validator agrees: the list accepts every value, and each listed name is the type of some value.
        listed = REFERENCE.compile({"type": names})
        assert all(REFERENCE.is_valid(listed, value) for _, value in live)
        for name in names:
            one = REFERENCE.compile({"type": name})
            assert any(REFERENCE.is_valid(one, value) for _, value in live), f"no value of type {name!r}"
        if "number" in names:
            integer = REFERENCE.compile({"type": "integer"})
            assert any(
                not REFERENCE.is_valid(integer, value) for literal, value in live if literal_type(literal) == "number"
            )
        event("numbers: " + ("none" if not numbers else "integer" if "integer" in names else "number"))

    # 13.3: x-latent-type is the reference's common Latent_Type, and its candidate.
    strings = [value for literal, value in live if literal_type(literal) == "string"]
    for s in strings:  # the per-string classification it is built from
        found = latent_type_of(s)
        assert (None if found is None else found.value) == ref_latent_type(s), f"Latent_Type of {s!r}"
    expected_latent = ref_common_latent_type(strings) if latent_on else None
    assert kw.get("x-latent-type") == expected_latent, f"x-latent-type for {strings!r}"
    if {ref_latent_type(s) for s in strings} == {"integer", "decimal"}:
        event("integer and decimal strings")
    if expected_latent is None:
        assert result.latent is None
    else:
        latent = result.latent
        assert latent is not None and latent.latent_type.value == expected_latent
        assert "string" in names  # the Latent_Type never replaces the JSON type
        if expected_latent in TEMPORAL and all(satisfies(expected_latent, s) for s in strings):
            assert latent.constraint == {"format": expected_latent}
        else:
            assert latent.keyword == "pattern"
            pattern = REFERENCE.compile({"pattern": latent.value})
            unmatched = [s for s in strings if not REFERENCE.is_valid(pattern, s)]
            assert not unmatched, f"pattern {latent.value!r} does not match {unmatched!r}"
        event(f"latent type {expected_latent} -> {latent.keyword}")

    # 13.4: the Format candidates are exactly the Formats at least one string satisfies; 13.10: for hostname,
    # one satisfying string must contain a "." (a single-label hostname is any short word).
    expected_formats = {
        fmt for fmt in TEN_FORMATS if any(satisfies(fmt, s) and (fmt != "hostname" or "." in s) for s in strings)
    } if format_on else set()
    offered = [] if result.string is None else [c for c in result.string.candidates if c.technique == FORMAT]
    assert all(c.keyword == "format" for c in offered)
    assert len(offered) == len({c.value for c in offered})
    assert {c.value for c in offered} == expected_formats, f"Format candidates for {strings!r}"
    if strings:
        event(f"Format candidates: {len(expected_formats)}")
