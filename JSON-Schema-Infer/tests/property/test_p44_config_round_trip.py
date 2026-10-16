"""Property 44: Configuration round trip and precedence (task 3.5).

For any Run_Configuration, writing it to TOML and reading it back yields an
equal configuration. For any assignment of each setting to a flag, the file,
or neither, the resolved value comes from the flag, otherwise the file,
otherwise the default.

**Validates: Requirements 27.3, 27.4**
"""

# Feature: json-schema-inference, Property 44: Configuration round trip and precedence

from __future__ import annotations

import os
import tempfile
from collections.abc import Mapping
from typing import Any

import tomli_w
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.run_config import (
    SETTING_NAMES,
    SETTINGS,
    TECHNIQUE_PREFIX,
    RunConfiguration,
    load_toml,
    read_config_file,
)
from tests.strategies import effective_value, run_configs, setting_flags, setting_values

# Where a supplied setting is given. "both" also puts a different valid value
# in the file, which the flag must shadow.
SOURCES = ("flag", "file", "both")


def same(a: Any, b: Any) -> bool:
    """Equal and of the same type, so an int setting read back as a float (1 == 1.0) is caught."""
    return type(a) is type(b) and a == b


def stored(name: str, raw: Any) -> Any:
    """The stored form of a supplied raw value (canonical Path, sorted ids, float for float settings)."""
    return raw if name.startswith(TECHNIQUE_PREFIX) else SETTINGS[name].normalize(raw)


def file_layout(entries: Mapping[str, Any]) -> dict[str, dict[str, Any]]:
    """Flat setting names and raw values arranged in the configuration-file tables."""
    tables: dict[str, dict[str, Any]] = {}
    for name, value in entries.items():
        if name.startswith(TECHNIQUE_PREFIX):
            table, key = "techniques", name[len(TECHNIQUE_PREFIX) :]
        else:
            table, key = SETTINGS[name].table, SETTINGS[name].key
        tables.setdefault(table, {})[key] = value
    return tables


@given(run_configs())
def test_writing_to_toml_and_reading_back_yields_an_equal_configuration(config: RunConfiguration) -> None:
    text = config.to_toml()
    back = RunConfiguration.from_toml(text)

    assert back == config
    for name in SETTING_NAMES:
        assert same(back.value(name), config.value(name)), name
    assert set(back.provenance_of.values()) == {"file"}
    assert back.to_toml() == text

    # Through a UTF-8 file on disk, as effective-config.toml is written and read with -c.
    with tempfile.TemporaryDirectory() as directory:
        path = os.path.join(directory, "effective-config.toml")
        with open(path, "wb") as handle:
            handle.write(text.encode("utf-8"))
        assert RunConfiguration.resolve(None, read_config_file(path)) == config


@given(setting_flags(), st.data())
def test_each_setting_resolves_to_its_flag_else_its_file_value_else_its_default(
    supplied: dict[str, Any], data: st.DataObject
) -> None:
    # Split the supplied settings between the two sources; the rest are given by neither.
    flags: dict[str, Any] = {}
    file_entries: dict[str, Any] = {}
    for name, value in supplied.items():
        source = data.draw(st.sampled_from(SOURCES), label=f"source of {name}")
        if source in ("flag", "both"):
            flags[name] = value
        if source == "file":
            file_entries[name] = value
        if source == "both":
            # Valid on its own; Technique states and BOUNDED_BY relations are
            # checked over the resolved values only, so it may break them.
            file_entries[name] = data.draw(setting_values(name), label=f"shadowed file value of {name}")
    file = file_layout(file_entries)
    if data.draw(st.booleans(), label="file read from TOML text"):
        file = load_toml(tomli_w.dumps(file))

    config = RunConfiguration.resolve(flags, file)

    for name in SETTING_NAMES:
        if name in flags:
            origin, expected = "flag", stored(name, flags[name])
        elif name in file_entries:
            origin, expected = "file", stored(name, file_entries[name])
        else:
            origin, expected = "default", effective_value({}, name)
        assert config.provenance_of[name] == origin, name
        assert same(config.value(name), expected), (name, config.value(name), expected)
    origins = set(config.provenance_of.values())
    event(f"origins: {', '.join(sorted(origins))}")
