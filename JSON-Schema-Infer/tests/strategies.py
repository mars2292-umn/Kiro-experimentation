"""Shared Hypothesis strategies (JSON values, collections, Schema_Models, run_configs, ...).

Strategies are added by the tasks that introduce the components they exercise.
"""

from __future__ import annotations

import json
import math
import sys
from collections.abc import Iterable, Mapping
from typing import Any

from hypothesis import assume
from hypothesis import strategies as st

from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import BOUNDED_BY, RESOURCE_PARAMETERS, SettingSpec
from jsinfer.config.run_config import (
    SETTING_NAMES,
    SETTINGS,
    TECHNIQUE_PREFIX,
    RunConfiguration,
    technique_setting,
)
from jsinfer.core.paths import ELEM_STEP, KEY, MAPVAL_STEP, key_step, render_step
from jsinfer.core.provenance import (
    BASELINE_PREFIX,
    BASELINE_RULES,
    DECISION_MODEL_COMPARISON,
    FALLBACK_REASONS,
    MDL_COMPARISON,
    REMOVED,
    RESOLUTION_METHODS,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    ProvenanceRecord,
    RelaxationRecord,
    Resolution,
    SampleInfo,
    Support,
    baseline_technique,
    fallback_technique,
)
from jsinfer.io.ingest import UTF8_BOM

# A small key alphabet makes keys repeat across objects and documents. It
# includes the empty key, a key that needs bracket rendering, and keys whose
# code-point order differs from UTF-16 order ("\uffff" < "😀").
KEYS = ("", "a", "b", "B", "id", "é", "\uffff", "😀", "a b")

# Numbers include integral floats (one value, several spellings), -0.0, a big
# integer, and non-integral floats.
_NUMBERS = st.one_of(
    st.integers(min_value=-3, max_value=3),
    st.sampled_from([0.0, -0.0, 1.0, 2.5, -0.5, 1e3, 2**70]),
)

json_scalars = st.one_of(
    st.none(),
    st.booleans(),
    _NUMBERS,
    st.text(alphabet="ab😀", max_size=3),
)


# Decimals whose shortest spelling is not their binary value (0.1 is
# 0.1000000000000000055…), next to the binary-exact json_scalars numbers.
decimal_scalars = st.one_of(json_scalars, st.sampled_from([0.1, 0.3, 1.1, -2.675, 1e-7, 123.456]))


def json_values(max_leaves: int = 12, scalars: st.SearchStrategy = json_scalars) -> st.SearchStrategy:
    """Nested JSON values (dicts with keys from ``KEYS``, lists, and ``scalars``)."""
    return st.recursive(
        scalars,
        lambda children: st.one_of(
            st.lists(children, max_size=4),
            st.dictionaries(st.sampled_from(KEYS), children, max_size=4),
        ),
        max_leaves=max_leaves,
    )


def json_documents(max_docs: int = 8, scalars: st.SearchStrategy = json_scalars) -> st.SearchStrategy:
    """Document_Collections: lists of JSON values (leaves from ``scalars``), mostly objects."""
    doc = st.one_of(
        st.dictionaries(st.sampled_from(KEYS), json_values(scalars=scalars), max_size=4),
        json_values(scalars=scalars),
    )
    return st.lists(doc, max_size=max_docs)


# ---------------------------------------------------------------------------
# JSON texts for ingestion (io/ingest.py, task 4.3)
# ---------------------------------------------------------------------------

# Numbers that take each parsing path: 64-bit integers (orjson), integers
# beyond 64 bits and integral floats from 2**63 (the stdlib fallback), and
# floats whose spelling must survive (1.0, -0.0).
_TEXT_NUMBERS = st.one_of(
    st.integers(min_value=-3, max_value=3),
    st.sampled_from([2**63 - 1, 2**64 - 1, 2**64, -(2**63) - 1, 10**30, 0.0, -0.0, 1.0, 2.5, 1e19, -1.5e300]),
)
# Strings that need escapes, with raw U+2028 and DEL, brackets (which a nesting
# scan must skip), and non-BMP text (a surrogate pair under ensure_ascii).
_TEXT_STRINGS = st.text(alphabet='ab"\\/\n\t\x00\x1f\x7f\u00e9\u2028\U0001f600[{', max_size=4)


def text_values(max_leaves: int = 8) -> st.SearchStrategy:
    """JSON values to serialize as texts: big numbers, and strings and keys that need escapes."""
    return st.recursive(
        st.one_of(st.none(), st.booleans(), _TEXT_NUMBERS, _TEXT_STRINGS),
        lambda children: st.one_of(
            st.lists(children, max_size=4),
            st.dictionaries(st.sampled_from(KEYS) | _TEXT_STRINGS, children, max_size=4),
        ),
        max_leaves=max_leaves,
    )


@st.composite
def json_texts(draw: st.DrawFn, values: st.SearchStrategy | None = None, multiline: bool = True) -> bytes:
    """RFC 8259 texts of ``values`` (default :func:`text_values`) in UTF-8, maybe with one leading BOM.

    Formatting varies: indentation, separators, ASCII escapes, and JSON
    whitespace around the value. With ``multiline=False`` the text holds no
    LF, so it fits on one line of a ``.jsonl`` file (it may hold CR).
    """
    value = draw(text_values() if values is None else values)
    indent = draw(st.sampled_from([None, 0, 2, "\t"])) if multiline else None
    separators = draw(st.sampled_from([(", ", ": "), (",", ":"), (" ,\t", " : ")]))
    text = json.dumps(value, ensure_ascii=draw(st.booleans()), indent=indent, separators=separators)
    space = " \t\r\n" if multiline else " \t\r"
    data = (draw(st.text(space, max_size=2)) + text + draw(st.text(space, max_size=2))).encode("utf-8")
    return UTF8_BOM + data if draw(st.booleans()) else data


# Single-line texts that RFC 8259 forbids (Requirement 1.11), or that the
# parsing policy of io/ingest.py rejects: doubles that overflow, unpaired
# surrogate escapes, and integers beyond the integer string conversion limit.
FORBIDDEN_JSON_LINES: tuple[bytes, ...] = (
    b"NaN",
    b"[1, Infinity]",
    b"-Infinity",
    b'{"a": NaN}',
    b"// comment",
    b"1 /* comment */",
    b"[1, 2,]",
    b'{"a": 1,}',
    b"{'a': 1}",
    b"'text'",
    b"1 2",
    b"{}{}",
    b'"a" "b"',
    b'"\xff"',
    b'["\xc3"]',
    b'"\xed\xa0\x80"',
    b"\xe9",
    UTF8_BOM + UTF8_BOM + b"1",
    b"1e400",
    b"[-1e400]",
    b'"\\ud800"',
    b'{"k\\udc00": 1}',
    b'["\\ud83d\\ud83d\\ude00"]',
    b'"\x01"',
    b"tru",
    b"[1",
    b"]",
    b'{"a" 1}',
    b"\x00",
    *((b"9" * (sys.get_int_max_str_digits() + 1),) if sys.get_int_max_str_digits() else ()),
)

# Multi-line texts that fail, so errors are found past the first line.
FORBIDDEN_JSON_DOCUMENTS: tuple[bytes, ...] = (
    b"",
    b" \n\t",
    b"[1,\n 2,\n x]",
    b'{\n  "a": NaN\n}',
    b'{\n"a": 1,\n}',
    b"1\n2",
    b"\n\n// comment\n1",
    b'["ok",\n"\xff"]',
    b'[\n"\\ud800"]',
    b"[0,\n   1e999]",
)


def forbidden_json_texts(multiline: bool = True) -> st.SearchStrategy[bytes]:
    """Texts that fail to parse with both parsers; with ``multiline=False``, without LF."""
    lines = st.sampled_from(FORBIDDEN_JSON_LINES)
    if not multiline:
        return lines
    return st.one_of(
        lines,
        st.sampled_from(FORBIDDEN_JSON_DOCUMENTS),
        # A valid text followed by a forbidden line: more than one top-level value, or invalid UTF-8.
        st.tuples(json_texts(), lines).map(lambda pair: pair[0] + b"\n" + pair[1]),
        st.tuples(st.integers(1, 3), lines).map(lambda pair: b"\n" * pair[0] + pair[1]),
    )



# ---------------------------------------------------------------------------
# Provenance_Records (core/provenance.py, task 2.5)
# ---------------------------------------------------------------------------

_TECHNIQUES = st.sampled_from(["decide.mdl", "values.bounds", "variants.dbscan", "rules.cooccurrence", "a b😀"])
_KINDS = st.sampled_from(["record_or_map", "bound", "closed_or_open_enum", "relaxation"])
# Finite floats (subnormals, -0.0, huge magnitudes) and ints, which records store as floats.
_FINITE = st.one_of(st.floats(allow_nan=False, allow_infinity=False), st.integers(-(2**53), 2**53))
_BITS = st.one_of(st.floats(min_value=0.0, allow_infinity=False), st.integers(0, 10**6))
_UNIT = st.one_of(st.floats(min_value=0.0, max_value=1.0), st.sampled_from([0, 1]))
_OUTCOMES = st.text(alphabet="ab=😀 ", max_size=6)
# The score strategy of each resolution method (see the core.provenance docstring).
_METHOD_SCORES = {"mdl": _BITS, "decision-model": _UNIT, "priority": st.none(), "fallback": st.none()}


@st.composite
def supports(draw: st.DrawFn) -> Support:
    total = draw(st.integers(0, 2**40))
    return Support(draw(st.integers(0, total)), total)


@st.composite
def candidate_scores(draw: st.DrawFn, score: st.SearchStrategy | None = None) -> CandidateScore:
    """CandidateScores whose score comes from ``score`` (default: absent or any finite number)."""
    n_scored = draw(st.none() | st.integers(0, 1000))
    n_rejected = draw(st.none() | st.integers(0, 1000 if n_scored is None else n_scored))
    return CandidateScore(
        technique=draw(st.none() | _TECHNIQUES),
        outcome=draw(_OUTCOMES),
        score=draw((st.none() | _FINITE) if score is None else score),
        model_bits=draw(st.none() | _BITS),
        data_bits=draw(st.none() | _BITS),
        exception_bits=draw(st.none() | _BITS),
        n_rejected=n_rejected,
        n_scored=n_scored,
    )


@st.composite
def resolutions(draw: st.DrawFn, methods: tuple[str, ...] = RESOLUTION_METHODS) -> Resolution:
    method = draw(st.sampled_from(methods))
    candidates = draw(st.lists(candidate_scores(_METHOD_SCORES[method]), min_size=1, max_size=4))
    return Resolution(method, draw(st.integers(0, len(candidates) - 1)), tuple(candidates))


def fallback_infos() -> st.SearchStrategy[FallbackInfo]:
    return st.builds(FallbackInfo, _TECHNIQUES, st.sampled_from(FALLBACK_REASONS))


def cutoffs() -> st.SearchStrategy[Cutoff]:
    return st.one_of(
        st.builds(Cutoff, st.sampled_from(["lsh.keyset_jaccard", "dbscan.min_samples"]), value=_FINITE),
        st.builds(
            Cutoff,
            st.sampled_from([MDL_COMPARISON, "BIC"]),
            scores=st.lists(candidate_scores(), min_size=1, max_size=3).map(tuple),
        ),
        st.builds(
            Cutoff,
            st.just(DECISION_MODEL_COMPARISON),
            scores=st.lists(candidate_scores(_UNIT), min_size=1, max_size=3).map(tuple),
            artifact=st.sampled_from(["dm.record_or_map.synthetic", "dm.tuple_or_list.weak"]),
        ),
    )


@st.composite
def sample_infos(draw: st.DrawFn) -> SampleInfo:
    total = draw(st.integers(1, 10**9))
    return SampleInfo(draw(st.integers(1, total)), total)


@st.composite
def relaxation_records(draw: st.DrawFn) -> RelaxationRecord:
    return RelaxationRecord(
        before=draw(json_values(max_leaves=4)),
        after=draw(st.just(REMOVED) | json_values(max_leaves=4)),
        documents=draw(st.lists(st.integers(0, 10**6), min_size=1, max_size=5)),
        resolution=draw(st.none() | resolutions(("mdl", "fallback"))),
    )


@st.composite
def provenance_records(draw: st.DrawFn) -> ProvenanceRecord:
    """Valid ProvenanceRecords, each optional part present or absent."""
    fallback = draw(st.none() | fallback_infos())
    if fallback is None:
        # Fallback markers and fallback resolutions require fallback information;
        # Baseline_Rule markers allow neither fallback information nor a resolution (26.13).
        technique = draw(_TECHNIQUES | st.sampled_from(BASELINE_RULES).map(baseline_technique))
        methods: tuple[str, ...] = ("mdl", "decision-model", "priority")
    else:
        technique = draw(_TECHNIQUES | _KINDS.map(fallback_technique))
        methods = RESOLUTION_METHODS
    baseline = technique.startswith(BASELINE_PREFIX)
    return ProvenanceRecord(
        technique=technique,
        support=draw(supports()),
        confidence=draw(_UNIT),
        fallback=fallback,
        resolution=None if baseline else draw(st.none() | resolutions(methods)),
        cutoff=draw(st.none() | cutoffs()),
        sample=draw(st.none() | sample_infos()),
        changes=tuple(draw(st.lists(relaxation_records(), max_size=2))),
        details=draw(st.dictionaries(st.sampled_from(KEYS), json_values(max_leaves=4), max_size=3)),
    )


# ---------------------------------------------------------------------------
# Run_Configurations (config/run_config.py, task 3.3)
# ---------------------------------------------------------------------------

# The flat names of every Technique toggle, in registry order.
TECHNIQUE_SETTINGS: tuple[str, ...] = tuple(technique_setting(tid) for tid in REGISTRY.ids())

# Characters a UTF-8 configuration file can hold (Unicode scalar values), with
# TOML and Path escapes, quotes, dots, and non-ASCII made common.
_SCALARS = st.characters(exclude_categories=("Cs",))
_CONFIG_CHARS = st.sampled_from('ab "\\.\n\t\x00\x7fé😀\ufffe') | _SCALARS
# Path keys: identifiers (rendered ``.name``), the empty key, and keys that need brackets.
_PATH_KEYS = st.sampled_from(["a", "users", "_x1", "", "a b", "é", "😀", '"', "\\", "\n"]) | st.text(
    _CONFIG_CHARS, max_size=4
)


def _number_values(spec: SettingSpec, low: int | float | None = None, high: int | float | None = None) -> st.SearchStrategy:
    """Values of the numeric setting ``spec`` inside its range, narrowed to [low, high] when given.

    ``float`` settings also get ints and ``-0.0``, which they accept and store as floats.
    """
    assert spec.range is not None
    lo, lo_open, hi, hi_open = spec.range.low, spec.range.low_open, spec.range.high, spec.range.high_open
    if low is not None and (lo is None or low > lo):
        lo, lo_open = low, False
    if high is not None and (hi is None or high < hi):
        hi, hi_open = high, False
    int_lo = None if lo is None else math.ceil(lo) + (1 if lo_open and math.ceil(lo) == lo else 0)
    int_hi = None if hi is None else math.floor(hi) - (1 if hi_open and math.floor(hi) == hi else 0)
    if int_lo is not None and int_hi is not None and int_lo > int_hi:
        ints: st.SearchStrategy = st.nothing()
    else:
        ints = st.integers(int_lo, int_hi)
    if spec.type is float:
        floats = st.floats(
            min_value=lo,
            max_value=hi,
            exclude_min=lo_open,
            exclude_max=hi_open,
            allow_nan=False,
            allow_infinity=False,
        )
        candidates = floats | ints
    else:
        candidates = ints

    def admissible(value: int | float) -> bool:
        return (
            spec.check(value) is None
            and (low is None or value >= low)
            and (high is None or value <= high)
        )

    if spec.type is float and admissible(-0.0):
        candidates |= st.just(-0.0)
    return candidates.filter(admissible)


@st.composite
def path_strings(draw: st.DrawFn) -> str:
    """Path strings in canonical or non-canonical spelling (bracketed identifiers, ``\\u`` escapes)."""
    text = "$"
    for _ in range(draw(st.integers(0, 4))):
        kind = draw(st.sampled_from(["key", "elem", "mapval"]))
        if kind == "elem":
            text += render_step(ELEM_STEP)
        elif kind == "mapval":
            text += render_step(MAPVAL_STEP)
        else:
            name = draw(_PATH_KEYS)
            spelling = draw(st.sampled_from(["canonical", "bracket", "ascii"]))
            if spelling == "canonical":
                text += render_step(key_step(name))
            else:
                text += f"[{json.dumps(name, ensure_ascii=spelling == 'ascii')}]"
    return text


def setting_values(name: str) -> st.SearchStrategy[Any]:
    """Raw values that pass the checks of the setting with flat ``name``, as a flag or file supplies them.

    Values are not normalized: ``float`` settings also get ints, ``input.ordering_key``
    gets non-canonical Path spellings, and ``corpus.exclude_ids`` gets unsorted lists
    with duplicates. A Technique toggle gets any boolean; whether a Technique may be
    enabled depends on the whole configuration (see :func:`setting_flags`).
    """
    if name.startswith(TECHNIQUE_PREFIX):
        if name not in TECHNIQUE_SETTINGS:
            raise ValueError(f"unregistered Technique toggle {name!r}")
        return st.booleans()
    setting = SETTINGS[name]
    if setting.kind == "text":
        return st.text(_CONFIG_CHARS, min_size=1, max_size=8)
    if setting.kind == "path":
        return st.just("") | path_strings()
    if setting.kind == "ids":
        item = st.text(st.sampled_from("ab😀 ") | _SCALARS, min_size=1, max_size=4)
        return st.lists(item, max_size=5)
    assert setting.spec is not None
    if setting.spec.type is bool:
        return st.booleans()
    if setting.spec.type is str:
        return st.text(_CONFIG_CHARS, max_size=8)
    return _number_values(setting.spec)


def _toggle_values(name: str) -> st.SearchStrategy[bool]:
    """States a Technique toggle may take on its own: an unimplemented Technique stays disabled."""
    tid = name[len(TECHNIQUE_PREFIX) :]
    return st.booleans() if REGISTRY.technique(tid).implemented else st.just(False)


def effective_value(flags: Mapping[str, Any], name: str) -> Any:
    """The raw value of setting ``name`` given the supplied ``flags``: the flag, else the default."""
    if name in flags:
        return flags[name]
    if name.startswith(TECHNIQUE_PREFIX):
        return REGISTRY.technique(name[len(TECHNIQUE_PREFIX) :]).default_enabled
    return SETTINGS[name].default_value()


@st.composite
def setting_flags(
    draw: st.DrawFn, vary: Iterable[str] | None = None, base: Mapping[str, Any] | None = None
) -> dict[str, Any]:
    """Flat setting names mapped to raw values that resolve to a valid Run_Configuration.

    ``base`` fixes some settings. Each other name in ``vary`` (default: every
    setting) is left out, taking its default, or drawn from
    :func:`setting_values`. Technique toggles are drawn only for implemented
    Techniques; an unimplemented one is left out or set to false, its only
    valid state, so the strategy follows the registry as Techniques land.
    Over the effective values, each :data:`BOUNDED_BY` relation is then
    repaired by redrawing a varied setting, and each enabled Technique with a
    disabled prerequisite by disabling it or enabling the prerequisite.
    """
    flags = dict(base or {})
    names = set(SETTING_NAMES if vary is None else vary)
    unknown = sorted(names.difference(SETTING_NAMES))
    if unknown:
        raise ValueError(f"unknown setting names {unknown}")
    varied = [name for name in SETTING_NAMES if name in names and name not in flags]
    mode = draw(st.sampled_from(["some", "all", "none"]), label="which settings are supplied")
    for name in varied:
        if mode == "none" or (mode == "some" and not draw(st.booleans(), label=f"supply {name}")):
            continue
        values = _toggle_values(name) if name.startswith(TECHNIQUE_PREFIX) else setting_values(name)
        flags[name] = draw(values, label=name)

    for name, bound in BOUNDED_BY:
        value, limit = effective_value(flags, name), effective_value(flags, bound)
        if value <= limit:
            continue
        if name in varied:
            flags[name] = draw(_number_values(RESOURCE_PARAMETERS[name], high=limit), label=name)
        elif bound in varied:
            flags[bound] = draw(_number_values(RESOURCE_PARAMETERS[bound], low=value), label=bound)
        else:
            assume(False)

    def enabled(tid: str) -> bool:
        return effective_value(flags, technique_setting(tid))

    def conflicts() -> list[tuple[str, str]]:
        return [
            (tid, prerequisite)
            for tid in REGISTRY.ids()
            if enabled(tid)
            for prerequisite in REGISTRY.technique(tid).prerequisites
            if not enabled(prerequisite)
        ]

    for _ in range(2 * len(REGISTRY.ids())):
        found = conflicts()
        if not found:
            break
        tid, prerequisite = found[0]
        if technique_setting(tid) in varied:
            flags[technique_setting(tid)] = False
        elif technique_setting(prerequisite) in varied and REGISTRY.technique(prerequisite).implemented:
            flags[technique_setting(prerequisite)] = True
        else:
            break
    assume(not conflicts())
    assume(all(REGISTRY.technique(tid).implemented or not enabled(tid) for tid in REGISTRY.ids()))
    return flags


def run_configs(
    vary: Iterable[str] | None = None, base: Mapping[str, Any] | None = None
) -> st.SearchStrategy[RunConfiguration]:
    """Valid Run_Configurations: :func:`setting_flags` resolved as flags.

    Full-run properties should pin costly Resource_Parameters, for example
    ``run_configs(vary=(*TECHNIQUE_SETTINGS, "seed"), base={"workers": 1})``.
    """
    vary = None if vary is None else tuple(vary)
    return setting_flags(vary, base).map(RunConfiguration.resolve)


# ---------------------------------------------------------------------------
# RFC 8259 JSON values the Ingestor accepts (io/ingest.py, task 4.4)
# ---------------------------------------------------------------------------
#
# The accepted domain (see the io.ingest module docstring): strings of Unicode
# scalar values (no unpaired surrogates), finite floats, integers of any size
# up to the integer string conversion limit, and at most MAX_NESTING_DEPTH
# nested containers. Imported here rather than at the top so this section
# stays self-contained.

import sys as _sys  # noqa: E402

from jsinfer.io.ingest import MAX_NESTING_DEPTH as _MAX_NESTING_DEPTH  # noqa: E402

# Digits in the longest integer literal the stdlib converts (0 means unlimited).
MAX_INT_DIGITS: int = _sys.get_int_max_str_digits() or 5000

# Integers around the ends of orjson's exact range [-2**63, 2**64 - 1], the
# largest integer a double holds exactly, and the largest double and the
# point at which an integer literal rounds to infinity.
_INT_EDGES = (
    2**63,
    2**64,
    -(2**63),
    -(2**64),
    2**53,
    2**1023,
    2**1024 - 2**970,
    2**1024,
    -(2**1024),
)
# Integers at the digit limit, with and without a sign.
_INT_DIGIT_LIMIT = (10**MAX_INT_DIGITS - 1, -(10**MAX_INT_DIGITS - 1), 10 ** (MAX_INT_DIGITS - 1))


# Indexing keeps the strategy reprs (in Hypothesis statistics) short.
def _one_of_constants(constants: tuple[Any, ...]) -> st.SearchStrategy:
    return st.integers(0, len(constants) - 1).map(constants.__getitem__)


rfc8259_integers = st.one_of(
    st.integers(),
    st.integers(-(2**200), 2**200),
    st.builds(lambda edge, delta: edge + delta, _one_of_constants(_INT_EDGES), st.integers(-2, 2)),
    _one_of_constants(_INT_DIGIT_LIMIT),
)

# Finite doubles, with subnormals, the extremes, and integral floats on both
# sides of 2**63 (orjson's results at or beyond it take the stdlib path).
rfc8259_floats = st.one_of(
    st.floats(allow_nan=False, allow_infinity=False),
    st.sampled_from(
        [
            0.0,
            -0.0,
            1.0,
            0.1,
            5e-324,
            -5e-324,
            2.2250738585072014e-308,
            1.7976931348623157e308,
            -1.7976931348623157e308,
            float(2**63 - 1024),
            float(2**63),
            -float(2**63),
            float(2**64),
            1e16,
            1e300,
        ]
    ),
)

# Characters that need escapes or are otherwise awkward: quotes, backslashes,
# control characters, line and paragraph separators, a BOM, noncharacters,
# astral characters, and bracket and number characters.
_AWKWARD_CHARS = st.sampled_from(
    ['"', "\\", "/", "\x00", "\x1f", "\n", "\r", "\t", "\x7f", "\x80", "\u2028", "\u2029", "\ufeff",
     "\ufffe", "\uffff", "\U0001f600", "\U0010ffff", "[", "{", "]", "}", "e", "9"]
)
# Unicode scalar values: every code point except the surrogates U+D800-U+DFFF.
_SCALAR_CHARS = st.characters(exclude_categories=("Cs",))

# Whole strings that look like things the parser checks for: surrogate escape
# text (a backslash, then "ud800"), rejected constants, an overflowing or long
# number, and enough brackets to reach the nesting pre-check.
_AWKWARD_STRINGS = _one_of_constants(
    ("", "\ufeff", "\\ud800", "\\uDC00x", "\\\\ud83d", "NaN", "-Infinity", "1e400", "9" * 25,
     "[" * (_MAX_NESTING_DEPTH + 1), "{" * _MAX_NESTING_DEPTH + "}" * _MAX_NESTING_DEPTH)
)

rfc8259_strings = st.one_of(st.text(_SCALAR_CHARS | _AWKWARD_CHARS, max_size=12), _AWKWARD_STRINGS)

rfc8259_scalars = st.one_of(st.none(), st.booleans(), rfc8259_integers, rfc8259_floats, rfc8259_strings)


def json_depth(value: Any) -> int:
    """The number of nested containers in ``value`` (0 for a scalar)."""
    depth, level = 0, [(value, 0)]
    while level:
        item, above = level.pop()
        if isinstance(item, (list, dict)):
            depth = max(depth, above + 1)
            level.extend((child, above + 1) for child in (item.values() if isinstance(item, dict) else item))
    return depth


@st.composite
def rfc8259_values(draw: st.DrawFn, max_leaves: int = 16) -> Any:
    """JSON values of any top-level type in the domain the Ingestor accepts.

    A drawn value is sometimes wrapped in further lists or single-member
    objects, up to exactly :data:`~jsinfer.io.ingest.MAX_NESTING_DEPTH`
    levels in total.
    """
    value = draw(
        st.recursive(
            rfc8259_scalars,
            lambda children: st.one_of(
                st.lists(children, max_size=4),
                st.dictionaries(rfc8259_strings, children, max_size=4),
            ),
            max_leaves=max_leaves,
        )
    )
    room = _MAX_NESTING_DEPTH - json_depth(value)
    extra = draw(st.one_of(st.just(0), st.integers(0, room), st.just(room)), label="extra nesting")
    if extra:
        kinds = draw(st.sampled_from([("list",), ("dict",), ("list", "dict")]), label="wrapper kinds")
        key = draw(rfc8259_strings, label="wrapper key")
        for level in range(extra):
            value = [value] if kinds[level % len(kinds)] == "list" else {key: value}
    return value


# ---------------------------------------------------------------------------
# Fully supported draft 2020-12 Schema_Models (io/emit.py, io/load.py; tasks 6.4, 6.5)
# ---------------------------------------------------------------------------
#
# :func:`schema_models` draws Schema_Models in the keyword grammar that the
# Schema_Loader supports fully (Requirements 2.4 and 2.10), with every value
# one that the Schema_Emitter and the draft 2020-12 meta-schema accept (with
# formats asserted, as in ``Validator.meta_errors``). Imported here rather
# than at the top so this section stays self-contained.

from urllib.parse import quote as _quote  # noqa: E402

from jsinfer.core.schema_model import (  # noqa: E402
    SUBSCHEMA_KEYWORDS as _SUBSCHEMA_KEYWORDS,
    SchemaModel as _SchemaModel,
    SchemaNode as _SchemaNode,
    split_pointer as _split_pointer,
)

SCHEMA_TYPE_NAMES: tuple[str, ...] = ("null", "boolean", "integer", "number", "string", "array", "object")

# Names under properties, $defs, dependentRequired, and required. They are not
# keywords, so keyword-like names, JSON Pointer escapes ("/", "~"), and
# characters a URI fragment must percent-encode ("%", "#", " ", non-ASCII)
# all appear; "\uffff" and "😀" sort differently in code-point and UTF-16 order.
_SCHEMA_NAME_CONSTANTS = (
    "", "a", "b", "B", "id", "a b", "é", "😀", "\uffff", "/", "~", "a/b~c", "~1", "%", "%25", "#", "?", "[x]",
    "$ref", "$schema", "type", "x-provenance", "x-a",
)
schema_names = st.one_of(_one_of_constants(_SCHEMA_NAME_CONSTANTS), st.text(_SCALAR_CHARS, max_size=4))

# Regex fragments that jsonschema-rs accepts as format "regex" (ECMA-262)
# and that stay valid when concatenated: none starts with a quantifier. They
# include the anchoring and lookahead of the safe regex AST (design, Value_Inferrer).
_REGEX_FRAGMENTS = (
    "a", "Z", "0", "é", "😀", " ", "-", "_", "/", ".", "\\.", "\\$", "\\d", "\\w", "\\s", "\\D", "[0-9]",
    "[a-z]+", "[A-Za-z]", "[^\\n]", "[0-9A-Za-z]{2,4}", "x?", "b*", "\\d{3}", "(?:ab|c)*", "(a|b)", "(?!\\n)",
    "^", "$",
)
regex_patterns = st.lists(_one_of_constants(_REGEX_FRAGMENTS), max_size=4).map("".join)

# Format names: the Formats the Value_Inferrer detects, other 2020-12
# formats, and unknown names (the meta-schema allows any string).
_FORMAT_NAMES = (
    "date-time", "date", "time", "duration", "email", "hostname", "ipv4", "ipv6", "uri", "uuid", "regex",
    "uri-reference", "json-pointer", "not-a-format",
)
_FORMATS = st.one_of(_one_of_constants(_FORMAT_NAMES), st.text(_SCALAR_CHARS, max_size=6))

# Root $id values: URI references without a non-empty fragment (2020-12 meta-schema).
_ROOT_IDS = (
    "https://example.com/schemas/root.json",
    "https://example.com/root#",
    "http://example.org/a%20b/c.json",
    "urn:example:schema",
    "tag:example.com,2024:s",
)

# JSON values held by const, enum, default, examples, and x- keywords: big
# integers, integral floats (2.0 is the integer 2; 1e300 an exact 301-digit
# integer), -0.0, subnormals, and strings with awkward characters.
_SCHEMA_JSON_SCALARS = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(-5, 5),
    _one_of_constants((2**64, -(2**70), 0.0, -0.0, 1.0, 2.0, 2.5, -0.5, 1e3, 5e-324, 1e300)),
    st.floats(allow_nan=False, allow_infinity=False),
    st.text(_SCALAR_CHARS | _AWKWARD_CHARS, max_size=5),
)
schema_json_values = st.recursive(
    _SCHEMA_JSON_SCALARS,
    lambda children: st.one_of(
        st.lists(children, max_size=3),
        st.dictionaries(schema_names, children, max_size=3),
    ),
    max_leaves=6,
)

# Numeric bounds take any number; length and count bounds non-negative integers
# (3.0 is an integer in draft 2020-12).
_NUMERIC_BOUNDS = st.one_of(
    st.integers(-10, 10),
    _one_of_constants((2**64, -(2**70), 0.5, -0.0, 1.0, 1e300, -2.5e-8)),
    st.floats(allow_nan=False, allow_infinity=False),
)
_COUNT_BOUNDS = st.one_of(st.integers(0, 10), _one_of_constants((0.0, 3.0, 2**64)))
_SCHEMA_TEXT = st.text(_SCALAR_CHARS | _AWKWARD_CHARS, max_size=6)

# Non-subschema keywords and their values. Lists that 2.7 compares as sets
# (type, enum, required, dependentRequired entries) may hold repeats in any
# order: the Schema_Emitter writes their distinct members in canonical order.
_SCHEMA_VALUE_KEYWORDS: dict[str, st.SearchStrategy] = {
    "type": st.one_of(
        st.sampled_from(SCHEMA_TYPE_NAMES),
        st.lists(st.sampled_from(SCHEMA_TYPE_NAMES), min_size=1, max_size=4),
    ),
    "enum": st.lists(schema_json_values, max_size=4),
    "const": schema_json_values,
    "format": _FORMATS,
    "pattern": regex_patterns,
    "minimum": _NUMERIC_BOUNDS,
    "maximum": _NUMERIC_BOUNDS,
    "exclusiveMinimum": _NUMERIC_BOUNDS,
    "exclusiveMaximum": _NUMERIC_BOUNDS,
    "minLength": _COUNT_BOUNDS,
    "maxLength": _COUNT_BOUNDS,
    "minItems": _COUNT_BOUNDS,
    "maxItems": _COUNT_BOUNDS,
    "minProperties": _COUNT_BOUNDS,
    "maxProperties": _COUNT_BOUNDS,
    "uniqueItems": st.booleans(),
    "required": st.lists(schema_names, max_size=4),
    "dependentRequired": st.dictionaries(schema_names, st.lists(schema_names, max_size=3), max_size=3),
    # The annotations the Schema_Loader retains at every node (2.10).
    "title": _SCHEMA_TEXT,
    "description": _SCHEMA_TEXT,
    "$comment": _SCHEMA_TEXT,
    "default": schema_json_values,
    "examples": st.lists(schema_json_values, max_size=3),
    "deprecated": st.booleans(),
    "readOnly": st.booleans(),
    "writeOnly": st.booleans(),
}
_SCHEMA_VALUE_KEYWORD_NAMES = tuple(_SCHEMA_VALUE_KEYWORDS)
_SCHEMA_SUBSCHEMA_KEYWORD_NAMES = tuple(_SUBSCHEMA_KEYWORDS)

# x- keyword names. "x-provenance" is dropped by the Schema_Emitter and left
# out of the 2.7 equality, so models that hold it still round-trip.
_X_KEYWORDS = st.one_of(
    _one_of_constants(("x-latent-type", "x-array-pattern", "x-key", "x-", "x-😀", "x-provenance")),
    st.text(_SCALAR_CHARS, max_size=4).map(lambda suffix: "x-" + suffix),
)

# Characters a rewritten fragment may keep unescaped (the RFC 3986 fragment
# characters beyond the unreserved ones), or only "/" to escape all of them.
_REF_SAFE_SETS = ("/!$&'()*+,;=:@", "/")


def _subschema_values(keyword: str, children: st.SearchStrategy) -> st.SearchStrategy:
    shape = _SUBSCHEMA_KEYWORDS[keyword]
    if shape == "single":
        return children
    if shape == "list":
        return st.lists(children, min_size=1, max_size=3)  # schemaArray: minItems 1
    keys = regex_patterns if keyword == "patternProperties" else schema_names
    return st.dictionaries(keys, children, max_size=3)


@st.composite
def _keyword_schema(draw: st.DrawFn, children: st.SearchStrategy | None) -> dict[str, Any]:
    """A schema object: a few value and annotation keywords, maybe an x- keyword, and with
    ``children`` one to three subschema keywords; keywords in a drawn order."""
    keywords = draw(st.lists(st.sampled_from(_SCHEMA_VALUE_KEYWORD_NAMES), unique=True, max_size=4))
    if children is not None:
        keywords += draw(st.lists(st.sampled_from(_SCHEMA_SUBSCHEMA_KEYWORD_NAMES), unique=True, min_size=1, max_size=3))
    keywords += draw(st.lists(_X_KEYWORDS, unique=True, max_size=1))
    schema: dict[str, Any] = {}
    for keyword in draw(st.permutations(keywords)):
        if keyword in _SCHEMA_VALUE_KEYWORDS:
            strategy = _SCHEMA_VALUE_KEYWORDS[keyword]
        elif keyword in _SUBSCHEMA_KEYWORDS:
            assert children is not None
            strategy = _subschema_values(keyword, children)
        else:
            strategy = schema_json_values
        schema[keyword] = draw(strategy, label=keyword)
    return schema


def schema_documents(max_leaves: int = 10) -> st.SearchStrategy[dict[str, Any] | bool]:
    """Plain draft 2020-12 schema documents (no ``$schema``, ``$id``, or ``$ref``), boolean schemas at any position."""
    leaves = st.one_of(st.booleans(), _keyword_schema(None))
    return st.recursive(leaves, _keyword_schema, max_leaves=max_leaves)


def _is_proper_prefix(prefix: list[str], tokens: list[str]) -> bool:
    return len(prefix) < len(tokens) and tokens[: len(prefix)] == prefix


@st.composite
def schema_models(draw: st.DrawFn, max_leaves: int = 10, refs: bool = True) -> _SchemaModel:
    """Fully supported draft 2020-12 Schema_Models (Requirements 2.4 and 2.10).

    The grammar holds every keyword of Requirement 2.4: the subschema
    keywords (``properties``, ``patternProperties``, ``additionalProperties``,
    ``propertyNames``, ``items``, ``prefixItems``, ``contains``, ``anyOf``,
    ``oneOf``, ``allOf``, ``not``, ``if``, ``then``, ``else``, ``$defs``),
    the value keywords (``type``, ``enum``, ``const``, ``format``,
    ``pattern``, the twelve bounds, ``uniqueItems``, ``required``,
    ``dependentRequired``), and local ``$ref``. Boolean schemas stand at
    every subschema position, and at the root in about one model in ten
    (the boolean case of the Schema_Emitter). Every node may also hold the
    retained annotations and one ``x-`` keyword (2.10), and the root of a
    keyword schema may hold an ``$id``.

    With ``refs``, about a quarter of the keyword schemas get a ``$ref``
    (inserted at a drawn position among their keywords) to a proper
    ancestor, ``#`` included, or to any ``$defs`` entry of the model. The
    reference is ``#`` plus the target's JSON Pointer, percent-encoded
    (sometimes more than needed), so it is a valid URI reference. Targets are
    never inside an embedded resource, so every reference is local and
    resolves. A reference may form a cycle that consumes no instance (a
    ``$ref`` to an ancestor through ``allOf``, or ``$defs`` entries that
    refer to each other); that is fine for emitting, loading, and meta-schema
    checks, but such a schema may not terminate under validation.

    Values are ones the Schema_Emitter and the draft 2020-12 meta-schema
    accept: regex-valid ``pattern`` values and ``patternProperties`` keys,
    non-negative integer length and count bounds, at least one valid
    ``type`` name, and at least one entry in ``prefixItems``, ``anyOf``,
    ``oneOf``, and ``allOf``. Values are not canonical: set-compared lists
    may repeat members in any order, numbers include integral floats,
    ``-0.0``, and big integers, and keywords are in a drawn order. Nodes have
    distinct nids in pre-order. The model is not renumbered, and its loader
    records (``ignored``, ``unsupported``, ``dialect_assumed``) are empty.
    """
    if draw(st.integers(0, 9), label="boolean root") == 0:
        return _SchemaModel(_SchemaNode(boolean=draw(st.booleans(), label="root value")))
    root = draw(_keyword_schema(schema_documents(max_leaves)), label="root")
    if draw(st.booleans(), label="root $id"):
        items = list(root.items())
        items.insert(draw(st.integers(0, len(items))), ("$id", draw(st.sampled_from(_ROOT_IDS))))
        root = dict(items)
    model = _SchemaModel(_SchemaNode.from_json(root))
    if not refs:
        return model

    positions = [(pointer, _split_pointer(pointer), node) for pointer, node in model.walk()]
    defs_entries = [pointer for pointer, tokens, _ in positions if len(tokens) >= 2 and tokens[-2] == "$defs"]
    for pointer, tokens, node in positions:
        if node.boolean is not None or draw(st.integers(0, 3), label=f"ref at {pointer!r}") != 0:
            continue
        ancestors = [other for other, prefix, _ in positions if _is_proper_prefix(prefix, tokens)]
        targets = ancestors + [entry for entry in defs_entries if entry not in ancestors]
        if not targets:
            continue  # the root, in a model without $defs
        target = draw(st.sampled_from(targets), label="ref target")
        ref = "#" + _quote(target, safe=draw(st.sampled_from(_REF_SAFE_SETS)))
        items = list(node.kw.items())
        items.insert(draw(st.integers(0, len(items)), label="ref position"), ("$ref", ref))
        node.kw.clear()
        node.kw.update(items)
    return model


# ---------------------------------------------------------------------------
# Safe regex ASTs paired with adversarial strings (values/patterns.py; task 9.2)
# ---------------------------------------------------------------------------

from jsinfer.values.patterns import (  # noqa: E402
    ONE as _ONE,
    SYNTAX_CHARACTERS as _SYNTAX_CHARACTERS,
    Alternative as _Alternative,
    CharClass as _CharClass,
    Pattern as _Pattern,
    Quantifier as _Quantifier,
    Token as _Token,
)

_CHAR_CLASSES = tuple(_CharClass)
# The ASCII members of each class, in code-point order.
_CLASS_MEMBERS: dict[_CharClass, str] = {
    cls: "".join(ch for ch in map(chr, range(128)) if cls.contains(ch)) for cls in _CHAR_CLASSES
}
_ASCII_ALNUM = _CLASS_MEMBERS[_CharClass.ALNUM]

# ECMA-262's line terminators (LF, CR, U+2028, U+2029) and CRLF. Python's "$"
# also matches before a final LF, which the printed lookahead rules out.
LINE_TERMINATORS: tuple[str, ...] = ("\n", "\r", "\r\n", "\u2028", "\u2029")

# Characters outside the ASCII classes that some engine might treat
# specially: the escaped ECMA-262 syntax characters; ASCII punctuation that
# is syntax in Rust or Python regexes ("-", "&", "~", "#", " "); controls and
# line terminators (NEL is one for Python's str.splitlines, not for "$");
# digits and letters outside the ASCII classes (superscript two,
# Arabic-Indic three, fullwidth zero and A, mathematical bold zero), which a
# Unicode "\d" or "\w" would accept; characters that case-fold to ASCII
# letters (Kelvin sign, long s, dotless i); a combining mark; a BOM,
# noncharacters, and astral characters (two UTF-16 code units each).
PATTERN_ADVERSARIAL_CHARS: tuple[str, ...] = (
    *_SYNTAX_CHARACTERS,
    "-", "_", "#", "&", "~", " ", ",", ":", ";", "=", "!", "<", ">", "'", '"', "`", "@", "%",
    "\x00", "\t", "\n", "\x0b", "\x0c", "\r", "\x1f", "\x7f", "\x85", "\xa0", "\u2028", "\u2029",
    "\u00b2", "\u0663", "\uff10", "\uff21", "\U0001d7ce", "\u212a", "\u017f", "\u0131", "\u00e9", "\u0301",
    "\u00df", "\u4e2d", "\ufeff", "\ufffe", "\uffff", "\U00010000", "\U0001f600", "\U0010ffff",
)

# Literal atoms: never an ASCII letter or digit (those belong to the classes)
# and never a surrogate (Token rejects both).
pattern_literals = st.one_of(
    _one_of_constants(PATTERN_ADVERSARIAL_CHARS),
    st.characters(exclude_categories=("Cs",), exclude_characters=_ASCII_ALNUM),
)

# Small counts keep every compiled pattern far below the engines' size limits,
# even after normal form adds up the counts of merged neighbours.
_MAX_PATTERN_COUNT = 4
pattern_quantifiers = st.one_of(
    st.just(_ONE),
    st.integers(1, _MAX_PATTERN_COUNT).map(_Quantifier.exactly),
    st.builds(
        lambda low, extra: _Quantifier(low, low + extra),
        st.integers(1, _MAX_PATTERN_COUNT),
        st.integers(1, _MAX_PATTERN_COUNT),
    ),
    st.integers(1, _MAX_PATTERN_COUNT).map(_Quantifier.at_least),
)
pattern_tokens = st.builds(_Token, st.one_of(_one_of_constants(_CHAR_CLASSES), pattern_literals), pattern_quantifiers)
# Alternatives may be empty (they then match only ""); Alternative merges
# overlapping neighbours into normal form.
pattern_alternatives = st.lists(pattern_tokens, max_size=6).map(_Alternative)


def safe_patterns(max_alternatives: int = 3) -> st.SearchStrategy[_Pattern]:
    """Patterns of the safe regex AST with one to ``max_alternatives`` alternatives (before deduplication)."""
    return st.lists(pattern_alternatives, min_size=1, max_size=max_alternatives).map(_Pattern)


@st.composite
def alternative_strings(draw: st.DrawFn, alternative: _Alternative) -> str:
    """A string that ``alternative`` matches: a run of each token's characters, of a length its quantifier accepts.

    Adjacent tokens are disjoint in normal form, so each run stops exactly
    where the next token's run starts, and the whole string matches.
    """
    parts: list[str] = []
    for token in alternative.tokens:
        quantifier = token.quantifier
        high = quantifier.high if quantifier.high is not None else quantifier.low + _MAX_PATTERN_COUNT
        count = draw(st.integers(quantifier.low, high))
        if isinstance(token.atom, _CharClass):
            members = st.sampled_from(_CLASS_MEMBERS[token.atom])
            parts.append("".join(draw(st.lists(members, min_size=count, max_size=count))))
        else:
            parts.append(token.atom * count)
    return "".join(parts)


_PATTERN_EDITS = ("append", "prepend", "insert", "delete", "replace")
_ADVERSARIAL_TEXT_CHARS = st.one_of(
    _SCALAR_CHARS, _one_of_constants(PATTERN_ADVERSARIAL_CHARS), st.sampled_from(_ASCII_ALNUM)
)


@st.composite
def pattern_strings(draw: st.DrawFn, pattern: _Pattern) -> str:
    """An adversarial string for ``pattern``.

    In three draws of four, a string built from one of the pattern's
    alternatives (so it matches), then edited zero to two times: a piece
    appended (a line terminator in about half the cases) or prepended, or
    inserted, or a character deleted or replaced by a piece. Pieces are line
    terminators, :data:`PATTERN_ADVERSARIAL_CHARS`, ASCII letters and
    digits, and the pattern's own literals, so an edit may keep the string
    matching, move it to another alternative, or break the match by one
    character. Otherwise, arbitrary text of Unicode scalar values (no
    surrogates, since jsonschema-rs takes Rust strings) mixed with the same
    adversarial characters.
    """
    if draw(st.integers(0, 3), label="arbitrary text") == 0:
        return draw(st.text(_ADVERSARIAL_TEXT_CHARS, max_size=10))
    literals = tuple(
        dict.fromkeys(token.atom for alt in pattern.alternatives for token in alt.tokens if token.is_literal)
    )
    pieces = st.one_of(
        _one_of_constants(LINE_TERMINATORS),
        _one_of_constants(PATTERN_ADVERSARIAL_CHARS),
        st.sampled_from(_ASCII_ALNUM),
        *([st.sampled_from(literals)] if literals else []),
    )
    suffixes = st.one_of(_one_of_constants(LINE_TERMINATORS), pieces)
    s = draw(alternative_strings(draw(st.sampled_from(pattern.alternatives), label="alternative")))
    for _ in range(draw(st.integers(0, 2), label="edits")):
        edit = draw(st.sampled_from(_PATTERN_EDITS), label="edit")
        if edit == "append":
            s += draw(suffixes)
        elif edit == "prepend":
            s = draw(pieces) + s
        elif edit == "insert":
            at = draw(st.integers(0, len(s)))
            s = s[:at] + draw(pieces) + s[at:]
        elif s:
            at = draw(st.integers(0, len(s) - 1))
            s = s[:at] + ("" if edit == "delete" else draw(pieces)) + s[at + 1 :]
    return s


@st.composite
def patterns_with_strings(draw: st.DrawFn, max_strings: int = 6) -> tuple[_Pattern, list[str]]:
    """A safe-AST Pattern and one to ``max_strings`` :func:`pattern_strings` for it."""
    pattern = draw(safe_patterns(), label="pattern")
    strings = draw(st.lists(pattern_strings(pattern), min_size=1, max_size=max_strings), label="strings")
    return pattern, strings
