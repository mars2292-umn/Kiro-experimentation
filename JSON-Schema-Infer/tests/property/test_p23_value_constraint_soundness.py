"""Property 23: Value Constraints are sound (task 12.9).

For any collection and Run_Configuration, every Constraint the
Value_Inferrer emits at a scalar Path (``format``, ``pattern``, ``enum`` or
``const``, ``minimum``/``maximum``, ``minLength``/``maxLength``, and any
accepted corpus suggestion) accepts every non-flagged value at the Path,
and every ``propertyNames`` Constraint (and ``patternProperties`` key) at a
Map Path accepts every non-flagged key, under the Validator with ``format``
asserted. At most one Format or Pattern from Requirement 13 is emitted per
Path, and no induced Pattern (Requirement 14) appears at a Path with a
selected Format or a Closed_Enum.

The test also checks the two rules soundness rests on: a candidate is
withheld exactly when the Validator rejects a non-flagged string (or key),
with the rejected count (13.6), and when a candidate survives, one survivor
is selected with the lowest description length (13.8).

Inputs
    Scalar mixes follow a theme: one of the ten Formats of 13.4
    (generated dates, date-times, times, durations, emails, hostnames,
    IPv4/IPv6 addresses, URIs, UUIDs), Latent_Type strings (integer,
    decimal, boolean, and non-RFC-3339 dates and times), code-like strings,
    numbers (big integers, integral floats, ``-0.0``, subnormals, huge
    doubles; dense counts; two-decimal amounts), or anything. Some elements
    are near-misses (a theme string edited by one character, or any number
    for a numeric theme) or other scalars, and values are drawn with
    repetition from a small pool, or mostly distinct, so both Closed_Enums
    and open Paths occur. The values sit at ``$`` (scalar documents),
    ``$.v`` (with some documents lacking ``v``), or ``$.v[*]``. Outlier
    documents with near-misses are added and flagged
    (``store.flags.flag_documents``), sometimes with a few clean documents,
    so "non-flagged" matters. Map keys are drawn from the string themes the
    same way, at ``$`` or ``$.m``.

    The reference values and keys are collected from the raw documents,
    independently of the ObservationStore. The Technique states come from
    ``run_configs()``, which enables only implemented Techniques.

Registry gating
    Clauses about pattern induction (``values.pattern``), corpus
    suggestions (``corpus.*``), and ``patternProperties`` keys are driven by
    the emitted keywords and their Provenance_Records, so they hold
    vacuously until those Techniques are implemented and then apply
    unchanged. While a Technique is unimplemented or disabled, no emitted
    keyword may name it.

**Validates: Requirements 13.6, 13.8, 14.1, 14.3, 14.7, 15.3, 16.6, 24.5**
"""

# Feature: json-schema-inference, Property 23: Value Constraints are sound

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from datetime import date, timedelta, timezone
from typing import Any

from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.observations import build_store
from jsinfer.core.paths import parse_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.views import View
from jsinfer.io.validator import Validator
from jsinfer.values.inferrer import (
    CORPUS_TECHNIQUES,
    FORMAT,
    LATENT_TYPE,
    PATTERN,
    SEMANTIC_TYPE,
    StringConstraint,
    ValueInferrer,
)
from tests.strategies import TECHNIQUE_SETTINGS, run_configs

# One Validator for the module: compiled schemas and per-string verdicts are cached across examples.
VALIDATOR = Validator()

# The Techniques whose Formats and Patterns are "from Requirement 13" (13.3–13.5).
REQ13_TECHNIQUES = frozenset({LATENT_TYPE, FORMAT, SEMANTIC_TYPE})
# Techniques whose clauses are gated on the registry: their records appear only once implemented and enabled.
GATED_TECHNIQUES = (PATTERN, *CORPUS_TECHNIQUES)

# Only Technique toggles matter to the Value_Inferrer; the other settings keep their defaults.
CONFIGS = run_configs(vary=TECHNIQUE_SETTINGS)


# ---------------------------------------------------------------------------
# Strings rich in Formats, Latent_Types, and near-misses
# ---------------------------------------------------------------------------

_OFFSETS = st.sampled_from([timezone.utc, timezone(timedelta(hours=5, minutes=30)), timezone(timedelta(hours=-8))])
_LABELS = st.from_regex(r"[a-z0-9]([a-z0-9-]{0,4}[a-z0-9])?", fullmatch=True)
_HOSTNAMES = st.one_of(
    st.lists(_LABELS, min_size=1, max_size=3).map(".".join),
    st.sampled_from(["example.com", "localhost", "xn--bcher-kva.example", "a-b.c"]),
)

FORMAT_STRINGS: dict[str, st.SearchStrategy[str]] = {
    "date": st.dates().map(date.isoformat),
    "date-time": st.datetimes(timezones=_OFFSETS).map(lambda d: d.isoformat()),
    # Times without an offset are near-misses of the RFC 3339 ``time`` Format.
    "time": st.times(timezones=_OFFSETS | st.none()).map(lambda t: t.isoformat()),
    "duration": st.sampled_from(["P1D", "PT1H30M", "P3Y6M4DT12H30M5S", "P2W", "PT0S", "P1M", "PT36H", "P1H"]),
    "email": st.builds(
        "{}@{}".format, st.sampled_from(["a", "first.last", "x_y+z", "ü", "a..b"]), _HOSTNAMES
    ),
    "hostname": _HOSTNAMES,
    "ipv4": st.ip_addresses(v=4).map(str),
    "ipv6": st.ip_addresses(v=6).map(str),
    "uri": st.one_of(
        st.sampled_from(["https://example.com/a?b=c#d", "urn:isbn:0451450523", "mailto:a@example.com", "ftp://h/p"]),
        st.builds("https://{}/{}".format, _HOSTNAMES, st.text("ab/%é ", max_size=4)),
    ),
    "uuid": st.uuids().map(str) | st.uuids().map(lambda u: str(u).upper()),
}

# Strings that encode a Latent_Type (integer, decimal, boolean, date, time,
# date-time), most outside the RFC 3339 spellings, and zero-padded codes that encode none.
_LATENT_CONSTANTS = (
    "0", "42", "-7", "+3", "007", "3.14", "-0.5", "1e3", "2.5E-3", "1.0", "true", "False", "TRUE",
    "2024-3-15", "15/03/2024", "03.15.2024", "2023-12-1", "12:30", "9:05:07.5", "23:59:60Z", "12:30+0530",
    "2024-03-15 12:30:00", "2024-3-15T9:05",
)
LATENT_STRINGS = st.one_of(
    st.sampled_from(_LATENT_CONSTANTS),
    st.integers(-(10**20), 10**20).map(str),
    st.floats(allow_nan=False, allow_infinity=False).map(repr),
)

# Identifier-like strings that a Pattern could describe, and arbitrary short text.
CODE_STRINGS = st.one_of(
    st.from_regex(r"[A-Z]{2}-[0-9]{3}", fullmatch=True),
    st.text("aB1-_ é😀\n", max_size=6),
)

# One-character edits that break (or sometimes keep) a Format or Latent_Type.
_EDIT_PIECES = ("", " ", "\n", "\u2028", "x", "Z", "0", "9", "-", ":", ".", "@", "/", "é", "😀", "\uff10")


@st.composite
def near_misses(draw: st.DrawFn, base: st.SearchStrategy[str]) -> str:
    """A string from ``base`` with one piece appended, prepended, or inserted, or one character replaced or deleted."""
    s = draw(base)
    edit = draw(st.sampled_from(["append", "prepend", "insert", "replace", "delete"]))
    piece = draw(st.sampled_from(_EDIT_PIECES))
    if edit == "append":
        return s + piece
    if edit == "prepend":
        return piece + s
    at = draw(st.integers(0, max(len(s) - 1, 0)))
    if edit == "insert":
        return s[:at] + piece + s[at:]
    return s[:at] + ("" if edit == "delete" else piece) + s[at + 1 :]


NUMBERS = st.one_of(
    st.integers(-1000, 1000),
    st.sampled_from([2**63 - 1, 2**64, -(2**63) - 1, 2**70, -(10**20), 255, 256, -128, 65535]),
    st.sampled_from([0.0, -0.0, 0.5, 1.0, 1e3, 2.5, -2.675, 0.1, 1e-7, 5e-324, 1e300, -1.7976931348623157e308]),
    st.floats(allow_nan=False, allow_infinity=False),
)

ANY_STRING = st.one_of(*FORMAT_STRINGS.values(), LATENT_STRINGS, CODE_STRINGS)
ANY_SCALAR = st.one_of(st.none(), st.booleans(), NUMBERS, ANY_STRING)

STRING_THEMES: dict[str, st.SearchStrategy[str]] = {**FORMAT_STRINGS, "latent": LATENT_STRINGS, "codes": CODE_STRINGS}
SCALAR_THEMES: dict[str, st.SearchStrategy[Any]] = {
    **STRING_THEMES,
    "numbers": NUMBERS,
    # Dense counts and two-decimal amounts, whose ranges MDL tends to bound.
    "counts": st.integers(0, 300),
    "amounts": st.integers(-(10**5), 10**5).map(lambda cents: cents / 100),
    "mixed": ANY_SCALAR,
}
NUMERIC_THEMES = frozenset({"numbers", "counts", "amounts"})


@st.composite
def themed_values(
    draw: st.DrawFn, themes: dict[str, st.SearchStrategy[Any]], other: st.SearchStrategy[Any], max_size: int = 40
) -> tuple[str, list[Any], st.SearchStrategy[Any]]:
    """``(theme, values, outliers)``: values mostly from one theme, some near-misses or ``other`` values.

    In ``repeat`` mode the values are drawn with repetition from a pool of
    up to six (Closed_Enums are likely); in ``distinct`` mode each value is
    a fresh draw (mostly singletons, so the Path is open). ``outliers``
    draws values meant for flagged documents.
    """
    # First a group: Formats (weighted double), numbers, or the rest, so that
    # every group is common and the ten Formats do not crowd out the others.
    formats = sorted(t for t in themes if t in FORMAT_STRINGS)
    groups = [
        group
        for group in (
            formats,
            formats,
            sorted(t for t in themes if t in NUMERIC_THEMES),
            sorted(t for t in themes if t not in FORMAT_STRINGS and t not in NUMERIC_THEMES),
        )
        if group
    ]
    theme = draw(st.sampled_from(draw(st.sampled_from(groups), label="theme group")), label="theme")
    main = themes[theme]
    # Near-misses: an edited theme string, or for a numeric theme any number (extremes included).
    if theme in STRING_THEMES:
        near = near_misses(main)
    elif theme in NUMERIC_THEMES:
        near = NUMBERS
    else:
        near = near_misses(ANY_STRING)
    impurity = draw(st.sampled_from([0, 0, 0, 1, 3]), label="impurity (out of 10)")

    @st.composite
    def element(draw2: st.DrawFn) -> Any:
        roll = draw2(st.integers(0, 9))
        if roll >= impurity:
            return draw2(main)
        return draw2(near if roll % 2 == 0 else other)

    # An explicit size, so long collections (where MDL starts to pay for bounds and enums) are common.
    n = draw(st.integers(1, max_size), label="number of values")
    if draw(st.sampled_from(["repeat", "distinct"]), label="mode") == "repeat":
        pool = draw(st.lists(element(), min_size=1, max_size=6), label="pool")
        values = draw(st.lists(st.sampled_from(pool), min_size=n, max_size=n), label="values")
    else:
        values = draw(st.lists(element(), min_size=n, max_size=n), label="values")
    outliers = st.one_of(near, other)
    return theme, values, outliers


# ---------------------------------------------------------------------------
# Collections with flagged documents
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Case:
    """Documents, the flagged document indices, the Path, and the reference values (or objects) at the Path."""

    theme: str
    docs: list[Any]
    flagged: list[int]
    path: str
    reference: list[Any]


@st.composite
def _assemble(draw: st.DrawFn, clean: list[Any], outliers: list[Any]) -> tuple[list[Any], list[int]]:
    """Shuffle clean and outlier documents; flag every outlier and maybe a few clean documents.

    Returns the documents and the flagged document indices.
    """
    tagged = [(doc, False) for doc in clean] + [(doc, True) for doc in outliers]
    order = draw(st.permutations(range(len(tagged))), label="document order")
    tagged = [tagged[i] for i in order]
    extra = draw(st.sets(st.integers(0, len(tagged) - 1), max_size=2), label="flagged clean documents")
    flagged = [i for i, (_, outlier) in enumerate(tagged) if outlier or i in extra]
    return [doc for doc, _ in tagged], flagged


def _chunks(draw: st.DrawFn, values: list[Any]) -> list[list[Any]]:
    """``values`` split in order into arrays of one to four elements."""
    out: list[list[Any]] = []
    i = 0
    while i < len(values):
        size = draw(st.integers(1, 4))
        out.append(values[i : i + size])
        i += size
    return out


@st.composite
def scalar_cases(draw: st.DrawFn) -> Case:
    theme, values, outlier_values = draw(themed_values(SCALAR_THEMES, ANY_SCALAR))
    outliers = draw(st.lists(outlier_values, max_size=3), label="outliers")
    layout = draw(st.sampled_from(["root", "field", "elements"]), label="layout")
    if layout == "root":
        clean, bad, path = list(values), list(outliers), "$"
    elif layout == "field":
        missing = draw(st.integers(0, 2), label="documents without v")
        clean = [{"v": v} for v in values] + [{"w": i} for i in range(missing)]
        bad, path = [{"v": v} for v in outliers], "$.v"
    else:
        clean = [{"v": chunk} for chunk in _chunks(draw, values)]
        bad = [{"v": chunk} for chunk in _chunks(draw, outliers)]
        path = "$.v[*]"
    docs, flagged = draw(_assemble(clean, bad))
    live = [doc for i, doc in enumerate(docs) if i not in set(flagged)]
    if layout == "root":
        reference = live
    elif layout == "field":
        reference = [doc["v"] for doc in live if "v" in doc]
    else:
        reference = [element for doc in live for element in doc["v"]]
    return Case(theme, docs, flagged, path, reference)


@st.composite
def key_cases(draw: st.DrawFn) -> Case:
    theme, keys, outlier_keys = draw(themed_values(STRING_THEMES, ANY_STRING, max_size=12))

    def objects(pool: Sequence[str] | st.SearchStrategy[str], n: int) -> list[dict[str, int]]:
        keys_of = st.lists(st.sampled_from(pool) if isinstance(pool, Sequence) else pool, max_size=4)
        return [{k: i for i, k in enumerate(draw(keys_of))} for _ in range(n)]

    clean = objects(keys, draw(st.integers(1, 6), label="objects"))
    bad = objects(outlier_keys, draw(st.integers(0, 2), label="outlier objects"))
    layout = draw(st.sampled_from(["root", "field"]), label="layout")
    if layout == "field":
        others = draw(st.lists(st.sampled_from([3, "x", None, [1]]), max_size=2), label="non-object values")
        clean = [{"m": obj} for obj in clean] + [{"m": value} for value in others] + [{"w": 1}]
        bad = [{"m": obj} for obj in bad]
    docs, flagged = draw(_assemble(clean, bad))
    live = [doc for i, doc in enumerate(docs) if i not in set(flagged)]
    if layout == "field":
        reference = [doc["m"] for doc in live if isinstance(doc.get("m"), dict)]
        path = "$.m"
    else:
        reference = live
        path = "$"
    return Case(theme, docs, flagged, path, reference)


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def _inferrer(config: RunConfiguration) -> ValueInferrer:
    return ValueInferrer.for_techniques(config.techniques, report=InferenceReport(), validator=VALIDATOR)


def _view(case: Case) -> View:
    store = build_store(case.docs)
    if case.flagged:
        store.flags.flag_documents(case.flagged)
    return View.root(store).descend(parse_path(case.path))


def _accepts(schema: dict[str, Any], instance: Any) -> bool:
    return VALIDATOR.is_valid(VALIDATOR.compile(schema), instance)


def _assert_accepts_all(schema: dict[str, Any], instances: Sequence[Any], where: str) -> None:
    rejected = [instance for instance in instances if not _accepts(schema, instance)]
    assert not rejected, f"{schema!r} at {where} rejects non-flagged {rejected[:3]!r}"


def _check_selection(selection: StringConstraint, strings: Sequence[str], where: str) -> None:
    """13.6: withheld exactly when a string is rejected (with the count); 13.8: a survivor is selected by MDL."""
    for candidate in selection.candidates:
        schema = {candidate.keyword: candidate.value}
        rejected = sum(1 for s in strings if not _accepts(schema, s))
        assert candidate.rejected == rejected, (
            f"{candidate.label} by {candidate.technique} at {where}: {candidate.rejected} rejections, Validator {rejected}"
        )
        assert candidate.withheld == (rejected > 0)
    selected = selection.selected
    step3 = [candidate for candidate in selection.survivors if candidate.technique != PATTERN]
    if step3:
        assert selected is not None and selected in selection.survivors, f"a candidate survives at {where}, none selected"
    if selection.survivors:
        decision = selection.decision
        assert decision is not None
        if selected is None:  # only induced Patterns survived and no Pattern won (14.2)
            assert decision.label == "none" and decision.method == "mdl", f"induced survivors at {where} lost without an MDL decision"
            event("no induced Pattern chosen")
        elif decision.method == "mdl":
            scores = [c.score for c in decision.resolution.candidates]
            assert scores[decision.resolution.selected] == min(scores), f"{selected.label} is not the shortest at {where}"
    else:
        assert selected is None, f"{selected!r} was selected at {where}, but every candidate rejects a string"
    if selected is not None:
        event(f"selected {selected.keyword}={selected.value if selected.keyword == 'format' else '<pattern>'}")
    if any(candidate.withheld for candidate in selection.candidates):
        event("a candidate was withheld")


def _check_gated(provenance: Any, config: RunConfiguration, where: str) -> None:
    """Keywords from pattern induction or corpus suggestions only once their Technique is implemented and enabled."""
    for keyword, record in provenance.items():
        if record.technique in GATED_TECHNIQUES:
            assert REGISTRY.technique(record.technique).implemented and config.techniques[record.technique], (
                f"{keyword} at {where} names {record.technique}, which is unimplemented or disabled"
            )
            event(f"keyword from {record.technique}")


@given(case=scalar_cases(), config=CONFIGS)
def test_scalar_value_constraints_accept_every_non_flagged_value(case: Case, config: RunConfiguration) -> None:
    result = _inferrer(config).infer_scalar(_view(case))
    kw = result.keywords()
    where = f"{case.path} (theme {case.theme})"
    scalars = [value for value in case.reference if not isinstance(value, (dict, list))]
    strings = [value for value in scalars if isinstance(value, str)]

    # Soundness: each emitted Constraint (annotations aside) accepts every non-flagged value, format asserted.
    for keyword, value in kw.items():
        if not keyword.startswith("x-"):
            _assert_accepts_all({keyword: value}, scalars, where)
    _assert_accepts_all(kw, scalars, where)

    # At most one Format or Pattern from Requirement 13 per Path (13.8).
    req13 = [k for k in ("format", "pattern") if k in kw and result.provenance[k].technique in REQ13_TECHNIQUES]
    assert len(req13) <= 1, f"{req13} from Requirement 13 at {where}"

    # No induced Pattern at a Path with a selected Format or a Closed_Enum (14.1).
    if "pattern" in kw and result.provenance["pattern"].technique == PATTERN:
        assert "format" not in kw and not result.enum.closed, f"induced Pattern at {where} with a Format or Closed_Enum"
    _check_gated(result.provenance, config, where)

    if result.string is not None:
        _check_selection(result.string, strings, where)
        if result.string.selected is not None and result.string.selected.technique in REQ13_TECHNIQUES:
            selected = result.string.selected
            assert kw.get(selected.keyword) == selected.value

    if case.flagged:
        event("flagged documents")
    if result.enum.closed:
        event("closed enum (const)" if "const" in kw else "closed enum (enum)")
    for keyword in ("minimum", "maximum", "minLength", "maxLength"):
        if keyword in kw:
            event(f"emitted {keyword}")
    if case.theme in NUMERIC_THEMES:
        event("numeric theme")


@given(case=key_cases(), config=CONFIGS)
def test_key_constraints_accept_every_non_flagged_key(case: Case, config: RunConfiguration) -> None:
    vi = _inferrer(config)
    found = vi.key_constraint(_view(case))
    where = f"keys of {case.path} (theme {case.theme})"
    objects = case.reference
    keys = [key for obj in objects for key in obj]  # once per object that has the key

    if found is None:
        # 13.8 at Map keys: with Format detection running, no Format may accept every key.
        if keys and vi.runs(FORMAT):
            satisfied = [f for f in FORMAT_STRINGS if all(VALIDATOR.satisfies_format(f, k) for k in keys)]
            assert not satisfied, f"{satisfied} accept every key at {where}, but no key Constraint was emitted"
        event("no key Constraint")
        return
    schema = {name: node.to_json() for name, node in found.keywords().items()}

    # Soundness: every non-flagged object validates, so its keys satisfy propertyNames (format asserted).
    _assert_accepts_all(schema, objects, where)
    names = schema.get("propertyNames")
    if isinstance(names, dict):
        _assert_accepts_all(names, keys, f"propertyNames of {where}")
        # At most one key Format or Pattern; none induced next to a selected key Format (14.1).
        assert not ("format" in names and "pattern" in names), f"propertyNames {names!r} at {where}"
    # patternProperties keys (14.1, 14.3): every key matches one of them, once key Patterns are emitted there.
    pattern_keys = list(schema.get("patternProperties", {}))
    if pattern_keys:
        unmatched = [k for k in keys if not any(VALIDATOR.matches_pattern(p, k) for p in pattern_keys)]
        assert not unmatched, f"patternProperties {pattern_keys} at {where} miss {unmatched[:3]!r}"

    _check_selection(found.selection, keys, where)
    _check_gated({"propertyNames": found.provenance()}, config, where)
    if case.flagged:
        event("flagged documents")
