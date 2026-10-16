"""The Run_Configuration: settings, resolution (flag > file > default), validation, and TOML round trip.

A :class:`RunConfiguration` is the complete set of settings of one run
(27.2): the random seed, the Model_Bundle identifier, the worker count and
every other Resource_Parameter, the input and output options, the corpus
options, and the enabled-or-disabled state of every registered Technique.
Every setting has a default, so ``RunConfiguration.resolve()`` with nothing
supplied is a complete configuration (27.2). The seed defaults to
:data:`~jsinfer.core.determinism.DEFAULT_SEED`, never the clock or system
entropy; ``workers`` defaults to the logical CPU count; every Technique
defaults to its registered state (28.2).

Configuration-file layout (``effective-config.toml``)
    ======================  ====================================================
    ``[run]``               ``seed``, ``model_bundle``, ``workers``
    ``[input]``             ``unwrap_arrays`` (1.3), ``ordering_key`` (18.2)
    ``[output]``            ``inline_provenance`` (26.6)
    ``[corpus]``            ``exclude_ids`` (30.6)
    ``[techniques]``        one boolean per registered Technique id
    ``[resources]``         every Resource_Parameter other than ``workers``
    ======================  ====================================================

    Technique ids and Resource_Parameter names contain dots, so they are
    quoted TOML keys (``"decide.mdl" = true``); an unquoted dotted key in
    ``[techniques]`` or ``[resources]``, which TOML reads as nested tables,
    is accepted as the same name.

Setting names
    Flags, :attr:`RunConfiguration.provenance_of`, and error messages name
    settings by a flat name: the bare key for ``[run]`` and ``[resources]``
    (``seed``, ``workers``, ``soundness.max_passes``), ``<table>.<key>`` for
    ``[input]``, ``[output]``, and ``[corpus]`` (``input.unwrap_arrays``),
    and ``techniques.<id>`` for a Technique toggle. :data:`SETTINGS` maps the
    non-Technique names to their :class:`Setting`; :data:`SETTING_NAMES`
    lists every name, Technique toggles last in registry order.

Resolution (27.3, 28.2)
    ``RunConfiguration.resolve(flags, file)`` takes ``flags`` as a mapping of
    flat setting names to values (the CLI's ``--set`` and ``--technique``
    pairs, or a library caller's overrides) and ``file`` as a parsed
    configuration file in the layout above (:func:`load_toml`,
    :func:`read_config_file`). Each setting resolves to its flag value if one
    is given, otherwise its file value, otherwise its default, and
    ``provenance_of`` records which (``"flag"``, ``"file"``, ``"default"``;
    20.5). Setting one Technique changes no other Technique's state.

Validation (27.11, 28.8, 28.9, 29.11)
    ``resolve`` checks every supplied entry of both sources, including file
    entries a flag overrides, and raises one
    :class:`~jsinfer.config.errors.ConfigError` listing every problem:

    - an unknown setting, table, or flag (with a hint when the name belongs
      elsewhere), and a setting given twice in the file;
    - a value of the wrong type or out of range, which covers the worker
      count and every sample size below 1 (29.11); messages name the setting
      and the accepted values; ``lsh.bands`` above ``minhash.perm`` is out of
      range too;
    - every unregistered Technique id, in one problem that also lists every
      registered id;
    - every enabled Technique that is registered but not implemented yet;
    - every enabled Technique with a disabled prerequisite, one problem per
      pair.

    Problems come in that order: flags before the file within each group,
    and settings in :data:`SETTING_NAMES` order. Value types follow
    :class:`~jsinfer.config.resources.SettingSpec`: booleans are never
    numbers, ``int`` settings take ints only, and ``float`` settings take
    ints and store floats. Technique toggles take booleans only.

    Strings must be Unicode scalar values, so that the configuration file
    can be written as UTF-8. ``model_bundle`` must be non-empty.
    ``input.ordering_key`` is ``""`` (choose automatically, 18.3, 18.4) or a
    Path string (``core.paths``), stored in its canonical rendering
    (``$["a"]`` becomes ``$.a``). ``corpus.exclude_ids`` is an array of
    non-empty strings, stored sorted in code-point order without duplicates.
    The seed is an integer in [0, 2**63 - 1], the non-negative TOML integers.

Equality and the TOML round trip (27.4)
    Configurations are equal when every setting value is equal;
    ``provenance_of`` is excluded, since it records where values came from.
    :meth:`RunConfiguration.to_toml` writes every setting with tomli-w, and
    :meth:`RunConfiguration.from_toml` reads it with tomllib, so
    ``from_toml(c.to_toml()) == c`` (with every origin ``"file"``).
    :meth:`RunConfiguration.to_json` is the same content as a JSON object,
    the ``config`` of the Inference_Report's ``run`` section (27.8).

Direct construction validates the fully resolved values the same way and
raises ``ConfigError``. ``techniques``, ``resources`` (which holds ``workers``
too), and ``provenance_of`` must cover every registered Technique,
Resource_Parameter, and setting name; prefer :meth:`RunConfiguration.resolve`.
"""

from __future__ import annotations

import json
import os
import re
import tomllib
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any, Final, Literal, TypeAlias

import tomli_w

from jsinfer.config.errors import ConfigError, ConfigProblem
from jsinfer.config.intervals import Interval
from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import (
    BOUNDED_BY,
    RESOURCE_PARAMETERS,
    WORKERS,
    SettingProblem,
    SettingSpec,
    check_resources,
    describe_value,
)
from jsinfer.core.determinism import DEFAULT_SEED, codepoint_key, sorted_codepoint
from jsinfer.core.jsonvalue import JSON
from jsinfer.core.paths import Path, PathSyntaxError, parse_path, render_path
from jsinfer.core.report import RESOURCE_ORIGINS, ResourceOrigin

__all__ = [
    "DEFAULT_MODEL_BUNDLE",
    "SEED_MAX",
    "SETTINGS",
    "SETTING_NAMES",
    "TABLES",
    "TECHNIQUE_PREFIX",
    "ConfigError",
    "ConfigProblem",
    "CorpusOptions",
    "InputOptions",
    "OutputOptions",
    "RunConfiguration",
    "Setting",
    "SettingKind",
    "load_toml",
    "read_config_file",
    "technique_setting",
]

# The default Model_Bundle identifier (27.2); the bundle reader resolves it.
DEFAULT_MODEL_BUNDLE: Final = "bundles/default"

# The largest seed: the largest TOML integer, so every seed round-trips (27.4).
SEED_MAX: Final = 2**63 - 1

# Configuration-file tables, in the order they are written.
TABLES: Final[tuple[str, ...]] = ("run", "input", "output", "corpus", "techniques", "resources")

# The flat-name prefix of a Technique toggle: "techniques.decide.mdl".
TECHNIQUE_PREFIX: Final = "techniques."

SettingKind: TypeAlias = Literal["value", "text", "path", "ids"]
_Source: TypeAlias = Literal["flag", "file"]

_TECHNIQUES: Final = "techniques"
_BARE_TABLES: Final = ("run", "resources")  # tables whose keys are flat names as they are
_FLATTENED_TABLES: Final = ("techniques", "resources")  # tables whose dotted TOML keys are joined back
_SOURCE_WORDS: Final[Mapping[str, str]] = MappingProxyType(
    {"flag": "flag", "file": "configuration file", "default": "default"}
)
_FILE: Final = "configuration file"
_BARE_KEY_RE: Final = re.compile(r"[A-Za-z0-9_-]+")
_SURROGATE_RE: Final = re.compile(r"[\ud800-\udfff]")

# Problem groups, in report order (see the module docstring).
_VALUES, _RELATIONS, _UNREGISTERED, _UNIMPLEMENTED, _PREREQUISITES = range(5)
_SOURCE_RANK: Final[Mapping[str, int]] = MappingProxyType({"flag": 0, "file": 1})


def _set(obj: object, name: str, value: Any) -> None:
    """Store a normalized field value on a frozen dataclass."""
    object.__setattr__(obj, name, value)


def _toml_key(key: str) -> str:
    """A TOML key as written in a file: bare when it can be, otherwise quoted."""
    return key if _BARE_KEY_RE.fullmatch(key) else json.dumps(key, ensure_ascii=False)


def _text_problem(text: str) -> str | None:
    """Why ``text`` cannot be written to a UTF-8 configuration file, or None."""
    bad = _SURROGATE_RE.search(text)
    if bad is None:
        return None
    return f"it contains the unpaired surrogate U+{ord(bad.group()):04X}, which UTF-8 cannot encode"


def technique_setting(tid: str) -> str:
    """The flat setting name of the toggle of Technique ``tid``: ``techniques.<tid>``."""
    return TECHNIQUE_PREFIX + tid


# ---------------------------------------------------------------------------
# Settings
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Setting:
    """One non-Technique setting: its flat ``name``, its file location, and how values are checked.

    ``kind`` selects the checks: ``"value"`` uses ``spec`` as it is (numbers
    and booleans); ``"text"`` is a non-empty string; ``"path"`` is ``""`` or
    a Path string; ``"ids"`` is an array of non-empty strings. ``spec``
    gives the type, range, default, and description; for ``"ids"`` it is
    None and the default is the empty tuple.
    """

    name: str
    table: str
    key: str
    kind: SettingKind
    spec: SettingSpec | None
    description: str = ""

    def __post_init__(self) -> None:
        if self.table not in TABLES or self.table == _TECHNIQUES:
            raise ValueError(f"setting {self.name!r} has table {self.table!r}, not a settings table")
        expected = self.key if self.table in _BARE_TABLES else f"{self.table}.{self.key}"
        if self.name != expected:
            raise ValueError(f"the setting at [{self.table}] {self.key} is named {expected!r}, got {self.name!r}")
        if (self.kind == "ids") != (self.spec is None):
            raise ValueError(f"setting {self.name!r}: only kind 'ids' has no SettingSpec")
        if self.spec is not None:
            if self.spec.name != self.name:
                raise ValueError(f"setting {self.name!r} has a SettingSpec named {self.spec.name!r}")
            if self.kind in ("text", "path") and self.spec.type is not str:
                raise ValueError(f"setting {self.name!r} of kind {self.kind!r} needs a str SettingSpec")
            if not self.description:
                _set(self, "description", self.spec.description)

    @property
    def location(self) -> str:
        """Where the setting goes in a configuration file, for example ``[resources] "sample.lof"``."""
        return f"[{self.table}] {_toml_key(self.key)}"

    def accepted(self) -> str:
        """The accepted values in words."""
        if self.kind == "text":
            return "a non-empty string"
        if self.kind == "path":
            return '"" (choose automatically) or a Path string such as "$.created_at"'
        if self.kind == "ids":
            return "an array of non-empty strings"
        assert self.spec is not None
        return self.spec.accepted()

    def _problem(self, reason: Literal["range", "type"], value: Any, detail: str | None = None) -> SettingProblem:
        message = f"{self.name}: expected {self.accepted()}, got {describe_value(value)}"
        return SettingProblem(self.name, reason, message if detail is None else f"{message} ({detail})")

    def check(self, value: Any) -> SettingProblem | None:
        """The problem with ``value`` for this setting (wrong type or out of range), or None."""
        if self.kind == "value":
            assert self.spec is not None
            return self.spec.check(value)
        if self.kind == "ids":
            if not isinstance(value, (list, tuple)):
                return self._problem("type", value)
            for index, item in enumerate(value):
                if not isinstance(item, str):
                    return self._problem("type", value, f"item {index} is {describe_value(item)}")
                detail = "it is empty" if not item else _text_problem(item)
                if detail is not None:
                    return self._problem("range", value, f"item {index}: {detail}")
            return None
        if not isinstance(value, str):
            return self._problem("type", value)
        detail = _text_problem(value)
        if detail is not None:
            return self._problem("range", value, detail)
        if self.kind == "text" and not value:
            return self._problem("range", value)
        if self.kind == "path" and value:
            try:
                parse_path(value)
            except PathSyntaxError as error:
                return self._problem("range", value, str(error))
        return None

    def normalize(self, value: Any) -> Any:
        """``value`` in its stored form; raises ``ValueError`` with the problem message when it is invalid."""
        problem = self.check(value)
        if problem is not None:
            raise ValueError(problem.message)
        if self.kind == "ids":
            return tuple(sorted(set(value), key=codepoint_key))
        if self.kind == "path":
            return render_path(parse_path(value)) if value else ""
        assert self.spec is not None
        return self.spec.normalize(value)

    def default_value(self) -> Any:
        """The default (``workers``: the logical CPU count)."""
        if self.spec is None:
            return ()
        return self.spec.default_value()

    def file_value(self, value: Any) -> JSON:
        """A stored value as written to a configuration file (tuples become arrays)."""
        return list(value) if self.kind == "ids" else value


def _settings() -> Mapping[str, Setting]:
    run = (
        Setting(
            "seed",
            "run",
            "seed",
            "value",
            SettingSpec(
                "seed",
                int,
                Interval(0, SEED_MAX),
                DEFAULT_SEED,
                "seed of every random stream (29.8); a fixed default, never the clock or system entropy (27.2)",
            ),
        ),
        Setting(
            "model_bundle",
            "run",
            "model_bundle",
            "text",
            SettingSpec("model_bundle", str, None, DEFAULT_MODEL_BUNDLE, "Model_Bundle identifier or path"),
        ),
        Setting(WORKERS, "run", WORKERS, "value", RESOURCE_PARAMETERS[WORKERS]),
    )
    options = (
        Setting(
            "input.unwrap_arrays",
            "input",
            "unwrap_arrays",
            "value",
            SettingSpec(
                "input.unwrap_arrays",
                bool,
                None,
                False,
                "treat each element of a top-level array in a .json file as one document (1.3)",
            ),
        ),
        Setting(
            "input.ordering_key",
            "input",
            "ordering_key",
            "path",
            SettingSpec(
                "input.ordering_key",
                str,
                None,
                "",
                'timestamp Path that orders documents for versioning (18.2); "" chooses one (18.3, 18.4)',
            ),
        ),
        Setting(
            "output.inline_provenance",
            "output",
            "inline_provenance",
            "value",
            SettingSpec(
                "output.inline_provenance",
                bool,
                None,
                False,
                "embed x-provenance annotations in the emitted schema (26.6)",
            ),
        ),
        Setting(
            "corpus.exclude_ids",
            "corpus",
            "exclude_ids",
            "ids",
            None,
            "Schema_Corpus schema identifiers the Corpus_Advisor ignores (30.6)",
        ),
    )
    resources = tuple(
        Setting(name, "resources", name, "value", spec) for name, spec in RESOURCE_PARAMETERS.items() if name != WORKERS
    )
    table: dict[str, Setting] = {}
    for setting in (*run, *options, *resources):
        if setting.name in table:
            raise ValueError(f"setting {setting.name!r} is defined twice")
        if setting.name.startswith(TECHNIQUE_PREFIX):
            raise ValueError(f"setting {setting.name!r} collides with the Technique toggle names")
        table[setting.name] = setting
    return MappingProxyType(table)


# Every non-Technique setting by flat name, in file order ([run], [input],
# [output], [corpus], then [resources] in code-point order).
SETTINGS: Final[Mapping[str, Setting]] = _settings()

# Every setting name: SETTINGS, then each Technique toggle in registry order.
SETTING_NAMES: Final[tuple[str, ...]] = (*SETTINGS, *(technique_setting(tid) for tid in REGISTRY.ids()))

_LOCATIONS: Final[Mapping[tuple[str, str], Setting]] = MappingProxyType(
    {(setting.table, setting.key): setting for setting in SETTINGS.values()}
)
_POSITION: Final[Mapping[str, int]] = MappingProxyType({name: i for i, name in enumerate(SETTING_NAMES)})


# ---------------------------------------------------------------------------
# Options
# ---------------------------------------------------------------------------


def _normalize_fields(obj: object, names: Iterable[tuple[str, str]]) -> None:
    """Check and normalize the fields of an options object in place; ``ConfigError`` lists every problem."""
    problems: list[ConfigProblem] = []
    for attr, name in names:
        setting = SETTINGS[name]
        value = getattr(obj, attr)
        problem = setting.check(value)
        if problem is None:
            _set(obj, attr, setting.normalize(value))
        else:
            problems.append(ConfigProblem(name, problem.reason, problem.message))
    if problems:
        raise ConfigError(problems)


@dataclass(frozen=True, slots=True)
class InputOptions:
    """Input options: array unwrapping (1.3) and the configured Ordering_Key Path (18.2)."""

    unwrap_arrays: bool = False
    ordering_key: str = ""  # canonical Path rendering; "" = choose automatically (18.3, 18.4)

    def __post_init__(self) -> None:
        _normalize_fields(self, (("unwrap_arrays", "input.unwrap_arrays"), ("ordering_key", "input.ordering_key")))

    @property
    def ordering_path(self) -> Path | None:
        """The configured Ordering_Key as a Path, or None when it is chosen automatically."""
        return parse_path(self.ordering_key) if self.ordering_key else None


@dataclass(frozen=True, slots=True)
class OutputOptions:
    """Output options: inline ``x-provenance`` annotations (26.6)."""

    inline_provenance: bool = False

    def __post_init__(self) -> None:
        _normalize_fields(self, (("inline_provenance", "output.inline_provenance"),))


@dataclass(frozen=True, slots=True)
class CorpusOptions:
    """Corpus options: Schema_Corpus identifiers excluded from the Corpus_Index (30.6), sorted and unique."""

    exclude_ids: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        _normalize_fields(self, (("exclude_ids", "corpus.exclude_ids"),))


# ---------------------------------------------------------------------------
# Problem collection
# ---------------------------------------------------------------------------


class _Problems:
    """Problems with a report-order sort key: (group, source, setting position, label)."""

    def __init__(self) -> None:
        self._items: list[tuple[tuple[int, int, int, str], ConfigProblem]] = []

    def __len__(self) -> int:
        return len(self._items)

    def add(self, group: int, source: str | None, position: int, label: str, problem: ConfigProblem) -> None:
        rank = -1 if source is None else _SOURCE_RANK[source]
        self._items.append(((group, rank, position, codepoint_key(label)), problem))

    def setting(self, group: int, source: str, problem: SettingProblem) -> None:
        """A setting problem of one source, its message prefixed with the source."""
        position = _POSITION.get(problem.name, len(_POSITION))
        message = f"{_SOURCE_WORDS[source]}: {problem.message}"
        self.add(group, source, position, problem.name, ConfigProblem(problem.name, problem.reason, message))

    def unknown(self, source: _Source, subject: str, message: str) -> None:
        self.add(_VALUES, source, len(_POSITION), subject, ConfigProblem(subject, "unknown", message))

    def sorted(self) -> list[ConfigProblem]:
        return [problem for _, problem in sorted(self._items, key=lambda item: item[0])]


def _hint(places: list[str]) -> str:
    return f" (did you mean {' or '.join(places)}?)" if places else ""


def _flag_suggestions(name: str) -> list[str]:
    """Flat names a mistyped flag may have meant: ``run.seed`` → ``seed``, ``decide.mdl`` → ``techniques.decide.mdl``."""
    places = [
        repr(setting.name)
        for setting in SETTINGS.values()
        if name in (setting.key, f"{setting.table}.{setting.key}") and name != setting.name
    ]
    if name in REGISTRY.techniques:
        places.append(repr(technique_setting(name)))
    return places


def _file_suggestions(key: str) -> list[str]:
    """File locations a misplaced key may have meant: ``seed`` → ``[run] seed``."""
    places = [setting.location for setting in SETTINGS.values() if key in (setting.key, setting.name)]
    if key in REGISTRY.techniques:
        places.append(f"[{_TECHNIQUES}] {_toml_key(key)}")
    return places


def _label(key: Any) -> str:
    return key if isinstance(key, str) else repr(key)


def _unregistered_problem(found: list[tuple[str, str | None]]) -> ConfigProblem:
    """One problem naming every unregistered Technique id (with its source) and every registered id (28.9)."""
    named = ", ".join(repr(tid) if source is None else f"{tid!r} ({_SOURCE_WORDS[source]})" for tid, source in found)
    noun = "id" if len(found) == 1 else "ids"
    return ConfigProblem(
        _TECHNIQUES,
        "unregistered",
        f"unregistered Technique {noun} {named}; the registered Technique ids are: {', '.join(REGISTRY.ids())}",
    )


def _technique_problems(
    states: Mapping[str, bool], origins: Mapping[str, str] | None, problems: _Problems
) -> None:
    """Enabled unimplemented Techniques and enabled Techniques with a disabled prerequisite (28.8)."""

    def state(tid: str) -> str:
        if origins is None:
            return ""
        return f" ({_SOURCE_WORDS[origins[technique_setting(tid)]]})"

    for position, tid in enumerate(REGISTRY.ids()):
        if not states[tid]:
            continue
        spec = REGISTRY.technique(tid)
        if not spec.implemented:
            problems.add(
                _UNIMPLEMENTED,
                None,
                position,
                tid,
                ConfigProblem(
                    tid,
                    "unimplemented",
                    f"Technique {tid!r} is enabled{state(tid)} but is not implemented yet; disable it",
                ),
            )
        for prerequisite in spec.prerequisites:
            if not states[prerequisite]:
                problems.add(
                    _PREREQUISITES,
                    None,
                    position,
                    prerequisite,
                    ConfigProblem(
                        tid,
                        "prerequisite",
                        f"Technique {tid!r} is enabled{state(tid)} but its prerequisite {prerequisite!r} is "
                        f"disabled{state(prerequisite)}; enable {prerequisite!r} or disable {tid!r}",
                    ),
                )


# ---------------------------------------------------------------------------
# Reading the two sources
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class _Entries:
    """The raw entries of one source: settings by flat name, and Technique toggles by id."""

    source: _Source
    settings: dict[str, Any] = field(default_factory=dict)
    techniques: dict[str, Any] = field(default_factory=dict)


def _flag_entries(flags: Mapping[str, Any] | None, problems: _Problems) -> _Entries:
    entries = _Entries("flag")
    if flags is None:
        return entries
    if not isinstance(flags, Mapping):
        raise TypeError(f"flags must be a mapping of setting names to values, got {type(flags).__name__}")
    for name, value in flags.items():
        if isinstance(name, str) and name in SETTINGS:
            entries.settings[name] = value
        elif isinstance(name, str) and name.startswith(TECHNIQUE_PREFIX):
            entries.techniques[name[len(TECHNIQUE_PREFIX) :]] = value
        else:
            hint = _hint(_flag_suggestions(name)) if isinstance(name, str) else ""
            problems.unknown("flag", _label(name), f"flag: unknown setting {name!r}{hint}")
    return entries


def _flatten(table: Mapping[Any, Any], prefix: str = "") -> Iterable[tuple[Any, Any]]:
    """(dotted key, value) pairs of a table whose dotted TOML keys became nested tables."""
    for key, value in table.items():
        if not isinstance(key, str):
            yield key, value
        elif isinstance(value, Mapping) and value:
            yield from _flatten(value, f"{prefix}{key}.")
        else:
            yield prefix + key, value


def _file_entries(file: Mapping[str, Any] | None, problems: _Problems) -> _Entries:
    entries = _Entries("file")
    if file is None:
        return entries
    if not isinstance(file, Mapping):
        problems.add(
            _VALUES,
            "file",
            -1,
            "",
            ConfigProblem(_FILE, "type", f"{_FILE}: expected a table at the top level, got {describe_value(file)}"),
        )
        return entries
    for top, table in file.items():
        if not isinstance(top, str) or top not in TABLES:
            label = _label(top)
            what = "table" if isinstance(table, Mapping) else "setting"
            shown = f"[{label}]" if what == "table" else _toml_key(label)
            hint = _hint(_file_suggestions(top)) if isinstance(top, str) else ""
            tables = ", ".join(f"[{name}]" for name in TABLES)
            problems.unknown("file", label, f"{_FILE}: unknown {what} {shown}{hint}; the tables are {tables}")
            continue
        if not isinstance(table, Mapping):
            problems.add(
                _VALUES,
                "file",
                -1,
                top,
                ConfigProblem(top, "type", f"{_FILE}: [{top}] must be a table, got {describe_value(table)}"),
            )
            continue
        pairs = _flatten(table) if top in _FLATTENED_TABLES else table.items()
        for key, value in pairs:
            label = _label(key)
            where = f"[{top}] {_toml_key(label)}"
            if top == _TECHNIQUES:
                target, name = entries.techniques, label
            else:
                setting = _LOCATIONS.get((top, key)) if isinstance(key, str) else None
                if setting is None:
                    hint = _hint([p for p in _file_suggestions(label) if p != where])
                    problems.unknown("file", where, f"{_FILE}: unknown setting {where}{hint}")
                    continue
                target, name = entries.settings, setting.name
            if name in target:
                subject = technique_setting(name) if top == _TECHNIQUES else name
                problems.add(
                    _VALUES,
                    "file",
                    _POSITION.get(subject, len(_POSITION)),
                    subject,
                    ConfigProblem(
                        subject,
                        "duplicate",
                        f"{_FILE}: {where} is set twice (as a quoted key and as a dotted key)",
                    ),
                )
                continue
            target[name] = value
    return entries


def _checked(
    entries: _Entries, problems: _Problems, unregistered: list[tuple[str, str | None]]
) -> tuple[dict[str, Any], dict[str, bool], list[str]]:
    """Valid normalized setting values and Technique states of one source, plus the names it got wrong."""
    values: dict[str, Any] = {}
    states: dict[str, bool] = {}
    bad: list[str] = []
    for name, setting in SETTINGS.items():
        if name not in entries.settings:
            continue
        problem = setting.check(entries.settings[name])
        if problem is None:
            values[name] = setting.normalize(entries.settings[name])
        else:
            problems.setting(_VALUES, entries.source, problem)
            bad.append(name)
    for tid in sorted_codepoint(entries.techniques):
        value = entries.techniques[tid]
        if tid not in REGISTRY.techniques:
            unregistered.append((tid, entries.source))
        elif isinstance(value, bool):
            states[tid] = value
        else:
            name = technique_setting(tid)
            message = f"{name}: expected a boolean (true enables the Technique), got {describe_value(value)}"
            problems.setting(_VALUES, entries.source, SettingProblem(name, "type", message))
    return values, states, bad


# ---------------------------------------------------------------------------
# Direct-construction checks
# ---------------------------------------------------------------------------


def _missing(kind: str, names: Iterable[str]) -> str:
    return f"{kind}: missing entries for {', '.join(repr(name) for name in names)}"


def _checked_states(value: Any, problems: _Problems) -> dict[str, bool] | None:
    if not isinstance(value, Mapping):
        message = f"techniques: expected a mapping of Technique id to boolean, got {describe_value(value)}"
        problems.add(_VALUES, None, len(_POSITION), "", ConfigProblem(_TECHNIQUES, "type", message))
        return None
    count = len(problems)
    unregistered = sorted_codepoint(_label(tid) for tid in value if tid not in REGISTRY.techniques)
    if unregistered:
        problems.add(_UNREGISTERED, None, 0, "", _unregistered_problem([(tid, None) for tid in unregistered]))
    missing = [tid for tid in REGISTRY.ids() if tid not in value]
    if missing:
        problems.add(_VALUES, None, len(_POSITION), "", ConfigProblem(_TECHNIQUES, "type", _missing(_TECHNIQUES, missing)))
    for tid in REGISTRY.ids():
        if tid in value and not isinstance(value[tid], bool):
            name = technique_setting(tid)
            message = f"{name}: expected a boolean, got {describe_value(value[tid])}"
            problems.add(_VALUES, None, _POSITION[name], name, ConfigProblem(name, "type", message))
    if len(problems) > count:
        return None
    return {tid: value[tid] for tid in REGISTRY.ids()}


def _checked_resources(value: Any, problems: _Problems) -> dict[str, int | float] | None:
    if not isinstance(value, Mapping):
        message = f"resources: expected a mapping of Resource_Parameter name to number, got {describe_value(value)}"
        problems.add(_VALUES, None, len(_POSITION), "", ConfigProblem("resources", "type", message))
        return None
    count = len(problems)
    for key in value:
        if not isinstance(key, str):
            message = f"unknown Resource_Parameter {key!r}"
            problems.add(_VALUES, None, len(_POSITION), repr(key), ConfigProblem(repr(key), "unknown", message))
    named = {key: item for key, item in value.items() if isinstance(key, str)}
    for problem in check_resources(named):
        problems.add(
            _VALUES,
            None,
            _POSITION.get(problem.name, len(_POSITION)),
            problem.name,
            ConfigProblem(problem.name, problem.reason, problem.message),
        )
    missing = [name for name in RESOURCE_PARAMETERS if name not in named]
    if missing:
        problems.add(_VALUES, None, len(_POSITION), "", ConfigProblem("resources", "type", _missing("resources", missing)))
    if len(problems) > count:
        return None
    return {name: spec.normalize(named[name]) for name, spec in RESOURCE_PARAMETERS.items()}


def _checked_origins(value: Any, problems: _Problems) -> dict[str, ResourceOrigin] | None:
    if not isinstance(value, Mapping):
        message = f"provenance_of: expected a mapping of setting name to origin, got {describe_value(value)}"
        problems.add(_VALUES, None, len(_POSITION), "", ConfigProblem("provenance_of", "type", message))
        return None
    count = len(problems)
    unknown = sorted_codepoint(_label(name) for name in value if name not in _POSITION)
    missing = [name for name in SETTING_NAMES if name not in value]
    invalid = [name for name in SETTING_NAMES if name in value and value[name] not in RESOURCE_ORIGINS]
    for what, names in (("unknown setting names", unknown), ("missing entries for", missing)):
        if names:
            message = f"provenance_of: {what} {', '.join(repr(name) for name in names)}"
            problems.add(_VALUES, None, len(_POSITION), what, ConfigProblem("provenance_of", "type", message))
    for name in invalid:
        message = f"provenance_of[{name!r}]: expected one of {list(RESOURCE_ORIGINS)}, got {value[name]!r}"
        problems.add(_VALUES, None, _POSITION[name], name, ConfigProblem(name, "type", message))
    if len(problems) > count:
        return None
    return {name: value[name] for name in SETTING_NAMES}


# ---------------------------------------------------------------------------
# The Run_Configuration
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class RunConfiguration:
    """The fully resolved, validated Run_Configuration of one run; see the module docstring.

    ``techniques`` maps every registered Technique id to its state, in
    registry order. ``resources`` maps every Resource_Parameter, ``workers``
    included, to its value, in code-point order. ``provenance_of`` maps every
    name in :data:`SETTING_NAMES` to ``"default"``, ``"file"``, or ``"flag"``
    and takes no part in equality.
    """

    seed: int
    model_bundle: str
    input: InputOptions
    output: OutputOptions
    corpus: CorpusOptions
    techniques: Mapping[str, bool]
    resources: Mapping[str, int | float]
    provenance_of: Mapping[str, ResourceOrigin] = field(compare=False)

    def __post_init__(self) -> None:
        problems = _Problems()
        for name in ("seed", "model_bundle"):
            setting = SETTINGS[name]
            value = getattr(self, name)
            problem = setting.check(value)
            if problem is None:
                _set(self, name, setting.normalize(value))
            else:
                problems.add(_VALUES, None, _POSITION[name], name, ConfigProblem(name, problem.reason, problem.message))
        for name, kind in (("input", InputOptions), ("output", OutputOptions), ("corpus", CorpusOptions)):
            value = getattr(self, name)
            if not isinstance(value, kind):
                message = f"{name}: expected {kind.__name__}, got {describe_value(value)}"
                problems.add(_VALUES, None, len(_POSITION), name, ConfigProblem(name, "type", message))
        states = _checked_states(self.techniques, problems)
        resources = _checked_resources(self.resources, problems)
        origins = _checked_origins(self.provenance_of, problems)
        if states is not None:
            _technique_problems(states, origins, problems)
        found = problems.sorted()
        if found:
            raise ConfigError(found)
        _set(self, "techniques", MappingProxyType(states))
        _set(self, "resources", MappingProxyType(resources))
        _set(self, "provenance_of", MappingProxyType(origins))

    def __reduce__(self) -> tuple[Any, tuple[Any, ...]]:
        # Read-only mapping proxies do not pickle; rebuild from plain dicts (joblib workers).
        return (
            type(self),
            (
                self.seed,
                self.model_bundle,
                self.input,
                self.output,
                self.corpus,
                dict(self.techniques),
                dict(self.resources),
                dict(self.provenance_of),
            ),
        )

    # -- construction -------------------------------------------------------

    @classmethod
    def resolve(
        cls, flags: Mapping[str, Any] | None = None, file: Mapping[str, Any] | None = None
    ) -> RunConfiguration:
        """Resolve every setting to its flag value, else its file value, else its default (27.3).

        ``flags`` maps flat setting names to values; ``file`` is a parsed
        configuration file. Raises ``ConfigError`` listing every problem in
        either source and every Technique conflict in the result (27.11,
        28.8, 28.9, 29.11).
        """
        problems = _Problems()
        sources = (_flag_entries(flags, problems), _file_entries(file, problems))
        unregistered: list[tuple[str, str | None]] = []
        checked = [_checked(entries, problems, unregistered) for entries in sources]
        bad = [name for _, _, names in checked for name in names]

        values: dict[str, Any] = {}
        origins: dict[str, ResourceOrigin] = {}
        for name, setting in SETTINGS.items():
            for entries, (valid, _, _) in zip(sources, checked):
                if name in valid:
                    values[name], origins[name] = valid[name], entries.source
                    break
            else:
                values[name], origins[name] = setting.default_value(), "default"
        states: dict[str, bool] = {}
        for tid in REGISTRY.ids():
            name = technique_setting(tid)
            for entries, (_, valid_states, _) in zip(sources, checked):
                if tid in valid_states:
                    states[tid], origins[name] = valid_states[tid], entries.source
                    break
            else:
                states[tid], origins[name] = REGISTRY.technique(tid).default_enabled, "default"

        # Ranges bounded by another setting, over the effective values; a
        # relation with an invalid value is skipped, as that value is reported.
        for pair in BOUNDED_BY:
            if any(name in bad for name in pair):
                continue
            supplied = {name: values[name] for name in pair if origins[name] != "default"}
            for problem in check_resources(supplied):
                origin = origins[problem.name]
                assert origin != "default"
                problems.setting(_RELATIONS, origin, problem)
        if unregistered:
            problems.add(_UNREGISTERED, None, 0, "", _unregistered_problem(unregistered))
        _technique_problems(states, origins, problems)
        found = problems.sorted()
        if found:
            raise ConfigError(found)

        return cls(
            seed=values["seed"],
            model_bundle=values["model_bundle"],
            input=InputOptions(values["input.unwrap_arrays"], values["input.ordering_key"]),
            output=OutputOptions(values["output.inline_provenance"]),
            corpus=CorpusOptions(values["corpus.exclude_ids"]),
            techniques=states,
            resources={name: values[name] for name in RESOURCE_PARAMETERS},
            provenance_of={name: origins[name] for name in SETTING_NAMES},
        )

    @classmethod
    def default(cls) -> RunConfiguration:
        """The configuration with every setting at its default (27.2)."""
        return cls.resolve()

    # -- access -------------------------------------------------------------

    @property
    def workers(self) -> int:
        """The worker-count Resource_Parameter (29.4), written under ``[run]``."""
        return int(self.resources[WORKERS])

    def value(self, name: str) -> Any:
        """The value of the setting with flat ``name`` (see :data:`SETTING_NAMES`)."""
        if name.startswith(TECHNIQUE_PREFIX) and name in _POSITION:
            return self.techniques[name[len(TECHNIQUE_PREFIX) :]]
        setting = SETTINGS.get(name)
        if setting is None:
            raise KeyError(f"unknown setting {name!r}")
        if name in RESOURCE_PARAMETERS:
            return self.resources[name]
        if setting.table == "run":
            return getattr(self, setting.key)
        return getattr(getattr(self, setting.table), setting.key)

    # -- serialization ------------------------------------------------------

    def to_json(self) -> dict[str, JSON]:
        """Every setting in the configuration-file layout, as a JSON object (27.7, 27.8)."""
        tables: dict[str, dict[str, JSON]] = {name: {} for name in TABLES}
        for name, setting in SETTINGS.items():
            tables[setting.table][setting.key] = setting.file_value(self.value(name))
        tables[_TECHNIQUES] = {tid: self.techniques[tid] for tid in sorted_codepoint(self.techniques)}
        return dict(tables)

    def to_toml(self) -> str:
        """The configuration file of this configuration, written with tomli-w (27.4, 27.7)."""
        return tomli_w.dumps(self.to_json())

    @classmethod
    def from_toml(cls, text: str) -> RunConfiguration:
        """Read a configuration file's text with tomllib and resolve it with no flags (27.4)."""
        return cls.resolve(None, load_toml(text))


# ---------------------------------------------------------------------------
# Configuration files
# ---------------------------------------------------------------------------


def load_toml(text: str, *, source: str = _FILE) -> dict[str, Any]:
    """Parse configuration-file text with tomllib; ``ConfigError`` (reason ``"parse"``) when it is not TOML (27.11)."""
    if not isinstance(text, str):
        raise TypeError(f"configuration text must be a str, got {type(text).__name__}")
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise ConfigError([ConfigProblem(source, "parse", f"{source} cannot be parsed as TOML: {error}")]) from None


def read_config_file(path: str | os.PathLike[str]) -> dict[str, Any]:
    """Read and parse a UTF-8 TOML configuration file; ``ConfigError`` naming the file when that fails (27.11)."""
    where = f"configuration file {os.fspath(path)!r}"
    try:
        with open(path, "rb") as handle:
            data = handle.read()
    except OSError as error:
        reason = error.strerror or type(error).__name__
        raise ConfigError([ConfigProblem(os.fspath(path), "parse", f"{where} cannot be read: {reason}")]) from None
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        message = f"{where} is not valid UTF-8 at byte {error.start}"
        raise ConfigError([ConfigProblem(os.fspath(path), "parse", message)]) from None
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        message = f"{where} cannot be parsed as TOML: {error}"
        raise ConfigError([ConfigProblem(os.fspath(path), "parse", message)]) from None
