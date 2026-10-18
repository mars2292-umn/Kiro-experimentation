"""Property 45: Invalid configurations stop before ingestion (task 18.3).

For any configuration containing unknown settings, wrongly typed or
out-of-range values (including worker or sample sizes < 1), or unregistered
Technique ids, the Inferrer exits non-zero before reading any document. Its
error names every offending setting and lists the registered ids when an id
is unknown.

Each example plants one to five faults among zero to three valid settings.
Every fault and every valid setting goes into the flags or the configuration
file. The faults are:

- an unknown flag, an unknown configuration-file table, or an unknown key
  in a known table;
- a value of the wrong type for a setting or a Technique toggle;
- a value out of range: worker counts and sample sizes below 1, other
  numbers outside their ranges (NaN and the infinities included), an empty
  Model_Bundle identifier, an invalid Ordering_Key Path, an empty Schema_Corpus
  id, or ``lsh.bands`` above ``minhash.perm``;
- an unregistered Technique id.

Two entry points are checked:

- The public API. ``RunConfiguration.resolve(flags, file)`` raises one
  ``ConfigError`` before ``jsinfer.infer`` or ``jsinfer.infer_files`` can
  start. The document iterable is never iterated, and the pipeline and the
  Ingestor never run. For the API, "exits" means "raises".
- The CLI. ``jsinfer infer DATA [-c FILE] [--set ...] [--technique ...] --out
  DIR`` returns exit status 2 and writes nothing to standard output.
  Standard error gets ``jsinfer: error:`` followed by the error. The CLI never
  enters ``jsinfer.infer_files``, the pipeline, or the Ingestor, and it
  creates neither ``DIR`` nor any other file.

The error holds exactly one problem per fault, except that all unregistered
ids share one problem, which also lists every registered id (28.9). Each
problem names its setting and the source of the value (flag or
configuration file). For a wrong type or an out-of-range value, the problem
also names the accepted values (29.11).

**Validates: Requirements 27.11, 28.9, 29.11**
"""

# Feature: json-schema-inference, Property 45: Invalid configurations stop before ingestion

from __future__ import annotations

import io
import itertools
import json
import math
import os
import tempfile
import tomllib
from collections.abc import Callable, Iterator
from contextlib import ExitStack, contextmanager, redirect_stderr, redirect_stdout
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal, NoReturn
from unittest.mock import patch

import pytest
import tomli_w
from hypothesis import event, given
from hypothesis import strategies as st

import jsinfer
from jsinfer import cli
from jsinfer.config.errors import ConfigError, ConfigProblem
from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS, SAMPLE_SIZES, WORKERS
from jsinfer.config.run_config import (
    SETTINGS,
    TABLES,
    TECHNIQUE_PREFIX,
    RunConfiguration,
    load_toml,
    technique_setting,
)
from jsinfer.io.ingest import Ingestor
from jsinfer.pipeline import infer as pipeline
from tests.strategies import TECHNIQUE_SETTINGS, setting_values

Target = Literal["api", "cli"]
Source = Literal["flag", "file"]

SOURCES: tuple[Source, ...] = ("flag", "file")
# How a problem's message starts, and how the unregistered-id problem tags each id, per source.
SOURCE_PREFIX = {"flag": "flag: ", "file": "configuration file: "}
SOURCE_WORD = {"flag": "flag", "file": "configuration file"}

# String settings, whose --set value the CLI takes verbatim, so a flag can never give them a wrong type.
VERBATIM = frozenset(name for name, setting in SETTINGS.items() if setting.kind in ("text", "path"))
# Configuration-file tables whose dotted keys are read as nested tables, so a table value there renames the key.
FLATTENED = ("techniques", "resources")
# Tables that can hold an unknown key ([techniques] keys are Technique ids, covered as unregistered ids).
KEYED_TABLES = tuple(table for table in TABLES if table != "techniques")
LOCATIONS = frozenset((setting.table, setting.key) for setting in SETTINGS.values())
RELATION = ("lsh.bands", "minhash.perm")  # lsh.bands must not exceed minhash.perm
SIZE_SETTINGS = (WORKERS, *SAMPLE_SIZES)  # 29.11: integers >= 1
RANGED = tuple(
    name
    for name, setting in SETTINGS.items()
    if setting.kind != "value" or (setting.spec is not None and setting.spec.type in (int, float))
)
TYPED = (*SETTINGS, *TECHNIQUE_SETTINGS)
KINDS = ("unknown", "type", "range", "relation", "unregistered")
DOCS = [{"id": 1, "name": "a"}, {"id": 2, "tags": ["x"]}]

# ---------------------------------------------------------------------------
# Values
# ---------------------------------------------------------------------------

_TEXT = st.text(st.characters(exclude_categories=("Cs",)), max_size=6)
_NONEMPTY_TEXT = st.text(st.characters(exclude_categories=("Cs",)), min_size=1, max_size=4)
STRINGS = st.sampled_from(["four", "1", "true", "on", "", "1,5", "a b"]) | _TEXT
BOOLS = st.booleans()
INTS = st.sampled_from([0, 1, -1]) | st.integers(-(2**70), 2**70)
FLOATS = st.sampled_from([0.0, 1.0, 2.0, -1.0, 1e3]) | st.floats()
LISTS = st.lists(st.one_of(BOOLS, st.integers(-5, 5), _TEXT), max_size=3)
TABLE_VALUES = st.dictionaries(st.sampled_from(["a", "x"]), st.integers(-5, 5), max_size=2)
SCALARS = st.one_of(STRINGS, BOOLS, INTS, FLOATS)
SURROGATE_TEXTS = st.sampled_from(["\ud800", "a\udfff"])
# --technique values other than on and off.
NOT_A_SWITCH = st.sampled_from(["ON", "Off", "true", "1", "yes", "", "maybe"]) | st.text(
    st.characters(codec="ascii", exclude_categories=("Cc",)), max_size=5
).filter(lambda raw: raw not in ("on", "off"))
INVALID_PATHS = ("a", "x.y", " $", "$.", "$..a", "$.a.", "$[", "$[*", "$[0]", "$a", '$["a"')

# Names that are not settings: near misses (with hints) and arbitrary dotted names.
_NAMES = st.from_regex(r"[a-z][a-z0-9_.-]{0,10}", fullmatch=True)
UNKNOWN_FLAG_NAMES = (
    st.sampled_from(
        ["worker", "seeds", "run.seed", "sample.lofs", "decide.mdl", "resources.sample.lof", "input.unwrap", "techniques"]
    )
    | _NAMES
).filter(lambda name: name not in SETTINGS and not name.startswith(TECHNIQUE_PREFIX))
UNKNOWN_TOPS = (st.sampled_from(["settings", "technique", "resource", "Run", "seed", "workers"]) | _NAMES).filter(
    lambda top: top not in TABLES
)
BARE_KEYS = st.sampled_from(["seeds", "worker", "unwrap", "workers", "bogus"]) | st.from_regex(
    r"[a-z][a-z0-9_-]{0,8}", fullmatch=True
)
UNREGISTERED_IDS = (
    st.sampled_from(["decide.MDL", "decide.mdl2", "variants.dbscan.x", "mdl", "decide", "nope.x"])
    | st.from_regex(r"[a-z][a-z0-9_.-]{0,12}", fullmatch=True)
).filter(lambda tid: tid not in REGISTRY.techniques)


def any_values(*, none: bool) -> st.SearchStrategy[Any]:
    """Values of every TOML type; ``None`` too where no TOML text carries them (API flags)."""
    pools = [STRINGS, BOOLS, INTS, FLOATS, LISTS, TABLE_VALUES]
    return st.one_of(*pools, *([st.none()] if none else []))


@st.composite
def _lists_with_a_non_string(draw: st.DrawFn) -> list[Any]:
    items: list[Any] = draw(st.lists(_NONEMPTY_TEXT, max_size=2))
    bad = draw(st.one_of(st.integers(-5, 5), BOOLS, st.floats(allow_nan=False), st.lists(st.integers(-5, 5), max_size=2)))
    items.insert(draw(st.integers(0, len(items))), bad)
    return items


@st.composite
def _lists_with_a_bad_string(draw: st.DrawFn, surrogates: bool) -> list[str]:
    items: list[str] = draw(st.lists(_NONEMPTY_TEXT, max_size=2))
    bad = draw(st.just("") | (SURROGATE_TEXTS if surrogates else st.nothing()))
    items.insert(draw(st.integers(0, len(items))), bad)
    return items


def wrong_types(name: str, *, tables: bool, none: bool) -> st.SearchStrategy[Any]:
    """Values whose type setting ``name`` rejects; no table values where ``tables`` is False."""
    if name.startswith(TECHNIQUE_PREFIX):
        pools = [INTS, FLOATS, STRINGS, LISTS]
    else:
        setting = SETTINGS[name]
        if setting.kind == "ids":
            pools = [STRINGS, INTS, FLOATS, BOOLS, _lists_with_a_non_string()]
        elif setting.kind in ("text", "path"):
            pools = [INTS, FLOATS, BOOLS, LISTS]
        else:
            assert setting.spec is not None
            kind = setting.spec.type
            if kind is bool:
                pools = [INTS, FLOATS, STRINGS, LISTS]
            elif kind is int:
                pools = [STRINGS, BOOLS, FLOATS, LISTS]  # 2.0 is a float, so it has the wrong type
            else:
                pools = [STRINGS, BOOLS, LISTS]  # float settings take ints
    if tables:
        pools.append(TABLE_VALUES)
    if none:
        pools.append(st.none())
    return st.one_of(*pools)


def out_of_range(name: str, *, surrogates: bool) -> st.SearchStrategy[Any]:
    """Values of the right type outside the range of setting ``name``; unpaired surrogates only if ``surrogates``."""
    setting = SETTINGS[name]
    if setting.kind == "text":
        return st.just("") | (SURROGATE_TEXTS if surrogates else st.nothing())
    if setting.kind == "path":  # "" is valid: the Ordering_Key is then chosen automatically
        invalid = st.sampled_from(INVALID_PATHS) | _TEXT.map(lambda text: "x" + text)
        return invalid | (st.just("$.a\udc80") if surrogates else st.nothing())
    if setting.kind == "ids":
        return _lists_with_a_bad_string(surrogates)
    spec = setting.spec
    assert spec is not None and spec.range is not None
    low, high = spec.range.low, spec.range.high
    parts: list[st.SearchStrategy[Any]] = []
    if spec.type is int:
        if low is not None:
            first = math.ceil(low) + (1 if spec.range.low_open and math.ceil(low) == low else 0)
            parts += [st.just(first - 1), st.integers(first - 2**70, first - 1)]
        if high is not None:
            last = math.floor(high) - (1 if spec.range.high_open and math.floor(high) == high else 0)
            parts += [st.just(last + 1), st.integers(last + 1, last + 2**70)]
    else:
        parts.append(st.sampled_from([math.nan, math.inf, -math.inf]))
        if low is not None:
            # Below a closed bound: strictly less (nextafter keeps -0.0 out of [0, ...)); below an open one: at most.
            top = low if spec.range.low_open else math.nextafter(low, -math.inf)
            parts.append(st.floats(max_value=top, allow_nan=False, allow_infinity=False))
            below = math.floor(low) if spec.range.low_open else math.ceil(low) - 1
            parts.append(st.integers(below - 2**70, below))
        if high is not None:
            bottom = high if spec.range.high_open else math.nextafter(high, math.inf)
            parts.append(st.floats(min_value=bottom, allow_nan=False, allow_infinity=False))
            above = math.ceil(high) if spec.range.high_open else math.floor(high) + 1
            parts.append(st.integers(above, above + 2**70))
    return st.one_of(*parts)


# ---------------------------------------------------------------------------
# Command-line rendering
# ---------------------------------------------------------------------------


def toml_inline(value: Any) -> str:
    """``value`` as one inline TOML value, the form ``--set KEY=VALUE`` reads."""
    if isinstance(value, list):
        return "[" + ", ".join(toml_inline(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{" + ", ".join(f"{json.dumps(key)} = {toml_inline(item)}" for key, item in value.items()) + "}"
    return tomli_w.dumps({"v": value}).removeprefix("v = ").removesuffix("\n")


def kept_as_string(raw: str) -> bool:
    """Whether ``--set KEY=raw`` passes ``raw`` on as a string because it is not one TOML value."""
    try:
        parsed = tomllib.loads(f"value = {raw}")
    except tomllib.TOMLDecodeError:
        return True
    return list(parsed) != ["value"]


def cli_raw(draw: st.DrawFn, name: str, value: Any) -> str:
    """The text after ``KEY=`` that gives setting ``name`` the value ``value``."""
    if name in VERBATIM:
        assert isinstance(value, str)
        return value
    if isinstance(value, str) and kept_as_string(value) and draw(st.booleans(), label="bare string"):
        return value
    return toml_inline(value)


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Entry:
    """One supplied entry: a flag (flat ``name``), or a file entry (``name`` is the key in ``table``, or top-level)."""

    source: Source
    name: str
    value: Any
    table: str | None = None
    cli: tuple[str, str] | None = None  # the CLI's (option, text) for a flag entry


@dataclass(frozen=True)
class Fault:
    """One planted fault and the problem that must report it."""

    kind: str
    source: Source
    subject: str  # the ConfigProblem subject: the offending setting (or file location)
    reason: str
    fragments: tuple[str, ...]  # text the problem's message must contain
    entries: tuple[Entry, ...]

    def matches(self, problem: ConfigProblem) -> bool:
        return (
            problem.subject == self.subject
            and problem.reason == self.reason
            and problem.message.startswith(SOURCE_PREFIX[self.source])
            and all(fragment in problem.message for fragment in self.fragments)
        )


@dataclass(frozen=True)
class Scenario:
    faults: tuple[Fault, ...]
    background: tuple[Entry, ...]  # valid settings

    def entries(self) -> list[Entry]:
        return [entry for fault in self.faults for entry in fault.entries] + list(self.background)

    def flags(self) -> dict[str, Any]:
        return {entry.name: entry.value for entry in self.entries() if entry.source == "flag"}

    def file(self) -> dict[str, Any] | None:
        file: dict[str, Any] = {}
        for entry in self.entries():
            if entry.source != "file":
                continue
            if entry.table is None:
                file[entry.name] = entry.value
            else:
                file.setdefault(entry.table, {})[entry.name] = entry.value
        return file or None

    def cli_flags(self) -> list[tuple[str, str]]:
        flags = []
        for entry in self.entries():
            if entry.source == "flag":
                assert entry.cli is not None
                flags.append(entry.cli)
        return flags


def file_location(name: str) -> tuple[str, str]:
    """The (table, key) of setting ``name`` in a configuration file."""
    if name.startswith(TECHNIQUE_PREFIX):
        return "techniques", name[len(TECHNIQUE_PREFIX) :]
    return SETTINGS[name].table, SETTINGS[name].key


def flag_entry(draw: st.DrawFn, target: Target, name: str, value: Any) -> Entry:
    cli_flag = (cli.SET_OPTION, f"{name}={cli_raw(draw, name, value)}") if target == "cli" else None
    return Entry("flag", name, value, cli=cli_flag)


def _unknown(draw: st.DrawFn, target: Target, source: Source, used: set[Any]) -> Fault:
    if source == "flag":
        name = draw(UNKNOWN_FLAG_NAMES.filter(lambda n: ("flag", n) not in used), label="unknown flag")
        used.add(("flag", name))
        value = draw(any_values(none=target == "api"), label=f"value of {name}")
        return Fault("unknown", source, name, "unknown", (repr(name),), (flag_entry(draw, target, name, value),))
    if draw(st.booleans(), label="unknown top-level entry"):
        top = draw(UNKNOWN_TOPS.filter(lambda t: ("top", t) not in used), label="unknown table")
        used.add(("top", top))
        value = draw(SCALARS | st.just({"x": 1}), label=f"value of {top}")
        return Fault("unknown", source, top, "unknown", (top,), (Entry("file", top, value),))
    table = draw(st.sampled_from(KEYED_TABLES), label="table")
    key = draw(
        BARE_KEYS.filter(lambda k: (table, k) not in LOCATIONS and ("key", table, k) not in used), label="unknown key"
    )
    used.add(("key", table, key))
    value = draw(SCALARS, label=f"value of {key}")  # no table: in [resources] it would rename the key
    return Fault("unknown", source, f"[{table}] {key}", "unknown", (key, f"[{table}]"), (Entry("file", key, value, table),))


def _value_fault(kind: str) -> Callable[[st.DrawFn, Target, Source, set[Any]], Fault]:
    def draw_fault(draw: st.DrawFn, target: Target, source: Source, used: set[Any]) -> Fault:
        if kind == "type":
            candidates = [name for name in TYPED if name not in used]
        else:
            sizes = draw(st.booleans(), label="a worker count or sample size")
            candidates = [name for name in (SIZE_SETTINGS if sizes else RANGED) if name not in used]
        name = draw(st.sampled_from(candidates), label=f"{kind} fault on")
        used.add(name)
        if kind == "type" and target == "cli" and source == "flag" and name in VERBATIM:
            source = "file"  # a string setting's flag value is a string
        table, key = file_location(name)
        if kind == "type":
            tables = not (source == "file" and table in FLATTENED)
            value = draw(wrong_types(name, tables=tables, none=target == "api" and source == "flag"), label=name)
        else:
            surrogates = source == "flag" and (target == "api" or name in VERBATIM)
            value = draw(out_of_range(name, surrogates=surrogates), label=name)
            if name in SIZE_SETTINGS:
                event("a worker count or sample size below 1")
        toggle = name.startswith(TECHNIQUE_PREFIX)
        # 29.11: a type or range problem names the setting and the accepted values.
        fragments = (name,) if toggle else (name, SETTINGS[name].accepted())
        if source == "file":
            entry = Entry("file", key, value, table)
        elif toggle and target == "cli" and draw(st.booleans(), label="as --technique"):
            raw = draw(NOT_A_SWITCH, label="--technique value")
            entry = Entry("flag", name, raw, cli=(cli.TECHNIQUE_OPTION, f"{key}={raw}"))
            fragments = (key,)
        else:
            entry = flag_entry(draw, target, name, value)
        return Fault(kind, source, name, kind, fragments, (entry,))

    return draw_fault


def _relation(draw: st.DrawFn, target: Target, source: Source, used: set[Any]) -> Fault:
    used.update(RELATION)
    perm_default = RESOURCE_PARAMETERS["minhash.perm"].default
    bands_default = RESOURCE_PARAMETERS["lsh.bands"].default
    assert isinstance(perm_default, int) and isinstance(bands_default, int)
    variant = draw(st.sampled_from(["both", "bands", "perm"]), label="relation variant")
    if variant == "both":
        perm = draw(st.integers(1, 64), label="minhash.perm")
        values = {"lsh.bands": draw(st.integers(perm + 1, 256), label="lsh.bands"), "minhash.perm": perm}
        subject = "lsh.bands"
    elif variant == "bands":  # above the default minhash.perm
        values, subject = {"lsh.bands": draw(st.integers(perm_default + 1, 10**6), label="lsh.bands")}, "lsh.bands"
    else:  # below the default lsh.bands
        values, subject = {"minhash.perm": draw(st.integers(1, bands_default - 1), label="minhash.perm")}, "minhash.perm"
    entries = tuple(
        Entry("file", name, value, "resources") if source == "file" else flag_entry(draw, target, name, value)
        for name, value in values.items()
    )
    return Fault("relation", source, subject, "range", RELATION, entries)


def _unregistered(draw: st.DrawFn, target: Target, source: Source, used: set[Any]) -> Fault:
    tid = draw(UNREGISTERED_IDS.filter(lambda t: technique_setting(t) not in used), label="unregistered id")
    name = technique_setting(tid)
    used.add(name)
    if source == "file":
        entry = Entry("file", tid, draw(SCALARS, label=f"value of {tid}"), "techniques")  # a table would rename the id
    elif target == "cli" and draw(st.booleans(), label="as --technique"):
        raw = draw(st.sampled_from(["on", "off"]) | NOT_A_SWITCH, label="--technique value")
        entry = Entry("flag", name, raw, cli=(cli.TECHNIQUE_OPTION, f"{tid}={raw}"))
    else:
        entry = flag_entry(draw, target, name, draw(BOOLS | any_values(none=target == "api"), label=f"value of {tid}"))
    fragment = f"{tid!r} ({SOURCE_WORD[source]})"
    return Fault("unregistered", source, "techniques", "unregistered", (fragment,), (entry,))


DRAWERS = {
    "unknown": _unknown,
    "type": _value_fault("type"),
    "range": _value_fault("range"),
    "relation": _relation,
    "unregistered": _unregistered,
}


@st.composite
def scenarios(draw: st.DrawFn, target: Target) -> Scenario:
    """One to five faults, each on its own setting, plus up to three valid settings, split between the sources."""
    used: set[Any] = set()
    faults = []
    for index in range(draw(st.integers(1, 5), label="number of faults")):
        kinds = [kind for kind in KINDS if kind != "relation" or not used.intersection(RELATION)]
        kind = draw(st.sampled_from(kinds), label=f"kind of fault {index}")
        source: Source = draw(st.sampled_from(SOURCES), label=f"source of fault {index}")
        fault = DRAWERS[kind](draw, target, source, used)
        event(f"fault: {fault.kind} ({fault.source})")
        faults.append(fault)
    background = []
    for _ in range(draw(st.integers(0, 3), label="number of valid settings")):
        candidates = [name for name in SETTINGS if name not in used and name not in RELATION]
        name = draw(st.sampled_from(candidates), label="valid setting")
        used.add(name)
        value = draw(setting_values(name), label=name)
        if draw(st.sampled_from(SOURCES), label=f"source of {name}") == "file":
            table, key = file_location(name)
            background.append(Entry("file", key, value, table))
        else:
            background.append(flag_entry(draw, target, name, value))
    event(f"faults: {len(faults)}")
    return Scenario(tuple(faults), tuple(background))


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


@contextmanager
def no_ingestion(*extra: tuple[object, str]) -> Iterator[list[str]]:
    """Make the pipeline, every Ingestor entry point, and ``extra`` record any call and fail."""
    calls: list[str] = []

    def refuse(label: str) -> Callable[..., NoReturn]:
        def refused(*args: Any, **kwargs: Any) -> NoReturn:
            calls.append(label)
            raise AssertionError(f"{label} was called: documents would be read")

        return refused

    targets = [(Ingestor, name) for name in ("ingest", "ingest_bytes", "ingest_values")]
    targets += [(pipeline, "run_pipeline"), *extra]
    with ExitStack() as stack:
        for owner, name in targets:
            stack.enter_context(patch.object(owner, name, refuse(f"{getattr(owner, '__name__', owner)}.{name}")))
        yield calls


def check_names_every_fault(error: ConfigError, scenario: Scenario) -> None:
    """One problem per fault naming its setting; all unregistered ids in one problem listing the registered ids."""
    text = str(error)
    remaining = list(error.problems)
    unregistered = [fault for fault in scenario.faults if fault.kind == "unregistered"]
    others = [fault for fault in scenario.faults if fault.kind != "unregistered"]
    assert len(remaining) == len(others) + (1 if unregistered else 0), text
    for fault in others:
        match = next((problem for problem in remaining if fault.matches(problem)), None)
        assert match is not None, f"no problem reports the {fault.kind} fault on {fault.subject!r} ({fault.source}):\n{text}"
        remaining.remove(match)
    if unregistered:
        [problem] = remaining
        assert (problem.subject, problem.reason) == ("techniques", "unregistered"), text
        rest = problem.message
        for fault in unregistered:
            for fragment in fault.fragments:
                assert fragment in rest, f"the unregistered id {fragment} is not named:\n{text}"
                rest = rest.replace(fragment, "")
        missing = [tid for tid in REGISTRY.ids() if tid not in rest]
        assert not missing, f"registered ids not listed: {missing}\n{text}"
    for problem in error.problems:
        assert problem.message in text


def write_documents(directory: Path) -> Path:
    path = directory / "data.jsonl"
    path.write_text("".join(json.dumps(document) + "\n" for document in DOCS), encoding="utf-8")
    return path


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


@given(scenario=scenarios("api"), entry=st.sampled_from(["infer", "infer_files"]), via_toml=st.booleans())
def test_the_api_rejects_an_invalid_configuration_before_any_document_is_read(
    scenario: Scenario, entry: str, via_toml: bool
) -> None:
    flags, file = scenario.flags(), scenario.file()
    if via_toml and file is not None:
        file = load_toml(tomli_w.dumps(file))
    iterated: list[bool] = []

    def documents() -> Iterator[Any]:
        iterated.append(True)
        yield from DOCS

    with tempfile.TemporaryDirectory() as directory, no_ingestion() as calls:
        data = write_documents(Path(directory))
        with pytest.raises(ConfigError) as caught:
            config = RunConfiguration.resolve(flags, file)
            if entry == "infer":
                jsinfer.infer(documents(), config)
            else:
                jsinfer.infer_files([data], config)

    assert iterated == [] and calls == []
    check_names_every_fault(caught.value, scenario)


@given(scenario=scenarios("cli"), data=st.data())
def test_the_cli_exits_2_before_any_document_is_read_and_writes_nothing(scenario: Scenario, data: st.DataObject) -> None:
    flags = data.draw(st.permutations(scenario.cli_flags()), label="flag order")
    errors: list[ConfigError] = []
    report = cli._config_failure

    def recording(error: ConfigError) -> int:
        errors.append(error)
        return report(error)

    stdout, stderr = io.StringIO(), io.StringIO()
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        argv = ["infer", str(write_documents(root))]
        file = scenario.file()
        if file is not None:
            config_file = root / "run.toml"
            config_file.write_text(tomli_w.dumps(file), encoding="utf-8")
            argv += ["-c", str(config_file)]
        out = root / "out"
        argv += [*itertools.chain.from_iterable(flags), "--out", str(out)]
        before = sorted(os.listdir(root))
        with (
            no_ingestion((jsinfer, "infer_files"), (jsinfer, "infer")) as calls,
            patch.object(cli, "_config_failure", recording),
            redirect_stdout(stdout),
            redirect_stderr(stderr),
        ):
            status = cli.main(argv)
        after = sorted(os.listdir(root))

    assert status == cli.EXIT_USAGE, stderr.getvalue()
    assert calls == []
    assert after == before  # no output directory, no temp directory, no other file
    assert stdout.getvalue() == ""
    assert len(errors) == 1, stderr.getvalue()
    assert stderr.getvalue() == f"jsinfer: error: {errors[0]}\n"
    check_names_every_fault(errors[0], scenario)
