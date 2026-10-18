"""Unit tests for config/run_config.py and config/errors.py (task 3.3).

Requirements 27.2, 27.3, 27.4, 27.11, 28.2, 28.8, 28.9, 29.11. The
round-trip and precedence property (Property 44) is task 3.5.
"""

from __future__ import annotations

import dataclasses
import pickle
import tomllib
from pathlib import Path
from typing import Any

import pytest

from jsinfer.config.errors import CONFIG_PROBLEM_REASONS, ConfigError, ConfigProblem
from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS, SAMPLE_SIZES, WORKERS, logical_cpu_count, resource_defaults
from jsinfer.config.run_config import (
    DEFAULT_MODEL_BUNDLE,
    SEED_MAX,
    SETTING_NAMES,
    SETTINGS,
    TABLES,
    CorpusOptions,
    InputOptions,
    OutputOptions,
    RunConfiguration,
    load_toml,
    read_config_file,
    technique_setting,
)
from jsinfer.core.determinism import DEFAULT_SEED
from jsinfer.core.paths import parse_path

HYBRID = "structure.hybrid_record_map"
MDL = "decide.mdl"


def problems_of(flags: Any = None, file: Any = None) -> tuple[ConfigProblem, ...]:
    with pytest.raises(ConfigError) as caught:
        RunConfiguration.resolve(flags, file)
    return caught.value.problems


def only(problems: tuple[ConfigProblem, ...]) -> ConfigProblem:
    assert len(problems) == 1, [p.message for p in problems]
    return problems[0]


# ---------------------------------------------------------------------------
# Defaults (27.2, 28.2)


def test_every_setting_has_a_default() -> None:
    config = RunConfiguration.resolve()
    assert config == RunConfiguration.default() == RunConfiguration.resolve({}, {})
    assert config.seed == DEFAULT_SEED == 0
    assert config.model_bundle == DEFAULT_MODEL_BUNDLE
    assert config.workers == logical_cpu_count()
    assert config.input == InputOptions(unwrap_arrays=False, ordering_key="")
    assert config.input.ordering_path is None
    assert config.output == OutputOptions(inline_provenance=False)
    assert config.corpus == CorpusOptions(exclude_ids=())
    assert dict(config.techniques) == REGISTRY.default_states()
    assert list(config.techniques) == list(REGISTRY.ids())
    assert dict(config.resources) == resource_defaults()
    assert list(config.provenance_of) == list(SETTING_NAMES)
    assert set(config.provenance_of.values()) == {"default"}


def test_setting_names() -> None:
    assert SETTING_NAMES[:7] == (
        "seed",
        "model_bundle",
        WORKERS,
        "input.unwrap_arrays",
        "input.ordering_key",
        "output.inline_provenance",
        "corpus.exclude_ids",
    )
    resources = [name for name in SETTINGS if SETTINGS[name].table == "resources"]
    assert sorted([*resources, WORKERS]) == sorted(RESOURCE_PARAMETERS)
    assert SETTING_NAMES[len(SETTINGS) :] == tuple(technique_setting(tid) for tid in REGISTRY.ids())
    assert len(set(SETTING_NAMES)) == len(SETTING_NAMES)


def test_value_reads_every_setting() -> None:
    config = RunConfiguration.resolve({"seed": 5, "input.unwrap_arrays": True, "sample.lof": 9})
    assert config.value("seed") == 5
    assert config.value("input.unwrap_arrays") is True
    assert config.value("sample.lof") == 9 == config.resources["sample.lof"]
    assert config.value(WORKERS) == config.workers
    assert config.value(technique_setting(MDL)) is REGISTRY.technique(MDL).default_enabled
    tables = config.to_json()
    for name in SETTING_NAMES:
        if name.startswith("techniques."):
            assert tables["techniques"][name.removeprefix("techniques.")] == config.value(name)
        else:
            setting = SETTINGS[name]
            assert tables[setting.table][setting.key] == setting.file_value(config.value(name))
    with pytest.raises(KeyError):
        config.value("nope")


# ---------------------------------------------------------------------------
# Precedence (27.3, 28.2)


def test_flag_beats_file_beats_default() -> None:
    file = {"run": {"seed": 2, "model_bundle": "from/file"}, "resources": {"sample.lof": 30, "ds.tol": 0.5}}
    config = RunConfiguration.resolve({"seed": 1, "sample.lof": 10}, file)
    assert (config.seed, config.provenance_of["seed"]) == (1, "flag")
    assert (config.model_bundle, config.provenance_of["model_bundle"]) == ("from/file", "file")
    assert (config.resources["sample.lof"], config.provenance_of["sample.lof"]) == (10, "flag")
    assert (config.resources["ds.tol"], config.provenance_of["ds.tol"]) == (0.5, "file")
    assert config.provenance_of["sample.fd"] == "default"
    assert config.resources["sample.fd"] == RESOURCE_PARAMETERS["sample.fd"].default


def test_technique_flag_beats_file_and_leaves_other_techniques_alone() -> None:
    # The file enables an unimplemented Technique; the flag disables it, so the effective state is valid.
    config = RunConfiguration.resolve(
        {technique_setting(HYBRID): False}, {"techniques": {HYBRID: True, "values.bounds": False}}
    )
    assert config.techniques[HYBRID] is False and config.techniques["values.bounds"] is False
    assert config.provenance_of[technique_setting(HYBRID)] == "flag"
    assert config.provenance_of[technique_setting("values.bounds")] == "file"
    defaults = REGISTRY.default_states()
    for tid in REGISTRY.ids():
        if tid not in (HYBRID, "values.bounds"):
            assert config.techniques[tid] == defaults[tid]
            assert config.provenance_of[technique_setting(tid)] == "default"


def test_workers_is_read_from_the_run_table() -> None:
    config = RunConfiguration.resolve(None, {"run": {WORKERS: 3}})
    assert config.workers == 3 == config.resources[WORKERS]
    assert config.provenance_of[WORKERS] == "file"


# ---------------------------------------------------------------------------
# Normalization


def test_values_are_stored_in_their_setting_types() -> None:
    config = RunConfiguration.resolve({"ds.tol": 0, "subschema.timeout_s": 3})
    assert type(config.resources["ds.tol"]) is float and config.resources["ds.tol"] == 0.0
    assert type(config.resources["subschema.timeout_s"]) is float


def test_ordering_key_is_stored_in_canonical_rendering() -> None:
    config = RunConfiguration.resolve({"input.ordering_key": '$["meta"]["created at"]'})
    assert config.input.ordering_key == '$.meta["created at"]'
    assert config.input.ordering_path == parse_path('$.meta["created at"]')


def test_exclude_ids_are_sorted_and_unique() -> None:
    config = RunConfiguration.resolve(None, {"corpus": {"exclude_ids": ["b", "a", "b", "é"]}})
    assert config.corpus.exclude_ids == ("a", "b", "é")


def test_dotted_keys_in_techniques_and_resources() -> None:
    text = "[techniques]\ndecide.mdl = false\n[resources]\nsoundness.max_passes = 3\n"
    config = RunConfiguration.resolve(None, load_toml(text))
    assert config.resources["soundness.max_passes"] == 3
    assert config.provenance_of["soundness.max_passes"] == "file"
    assert config.provenance_of[technique_setting(MDL)] == "file"


# ---------------------------------------------------------------------------
# TOML round trip (27.4)


def test_to_toml_writes_every_setting_in_the_file_layout() -> None:
    config = RunConfiguration.resolve({"seed": 3, "corpus.exclude_ids": ["x"], "ds.tol": 1e-9})
    data = tomllib.loads(config.to_toml())
    assert list(data) == list(TABLES)
    assert data["run"] == {"seed": 3, "model_bundle": DEFAULT_MODEL_BUNDLE, WORKERS: config.workers}
    assert data["input"] == {"unwrap_arrays": False, "ordering_key": ""}
    assert data["output"] == {"inline_provenance": False}
    assert data["corpus"] == {"exclude_ids": ["x"]}
    assert sorted(data["techniques"]) == sorted(REGISTRY.ids())
    assert sorted(data["resources"]) == sorted(name for name in RESOURCE_PARAMETERS if name != WORKERS)
    assert data["resources"]["ds.tol"] == 1e-9
    assert data == config.to_json()


@pytest.mark.parametrize(
    "flags",
    [
        {},
        {
            "seed": SEED_MAX,
            "model_bundle": 'dir with "quotes"\\and é\n',
            WORKERS: 1,
            "input.unwrap_arrays": True,
            "input.ordering_key": '$["a\\u0000b"][*]{*}',
            "output.inline_provenance": True,
            "corpus.exclude_ids": ["z", "a b", "😀"],
            "ds.tol": 5e-324,
            "bench.run_timeout_s": 1.7976931348623157e308,
            "nearest.next": 0,
            "minhash.perm": 7,
            "lsh.bands": 7,
            technique_setting(MDL): False,
        },
    ],
)
def test_toml_round_trip(flags: dict[str, Any]) -> None:
    config = RunConfiguration.resolve(flags)
    text = config.to_toml()
    text.encode("utf-8")  # the file is writable as UTF-8
    back = RunConfiguration.from_toml(text)
    assert back == config
    for name in SETTING_NAMES:
        assert back.value(name) == config.value(name), name
    assert set(back.provenance_of.values()) == {"file"}
    assert back.to_toml() == text


def test_equality_ignores_origins() -> None:
    assert RunConfiguration.resolve({"seed": 0}) == RunConfiguration.resolve()
    assert RunConfiguration.resolve({"seed": 1}) != RunConfiguration.resolve()


# ---------------------------------------------------------------------------
# One ConfigError lists every problem (27.11, 28.8, 28.9, 29.11)


@pytest.mark.parametrize("name", [WORKERS, *SAMPLE_SIZES])
@pytest.mark.parametrize("value", [0, -3, 2.0, True, "4"])
def test_worker_count_and_sample_sizes_below_one_or_not_integers(name: str, value: Any) -> None:
    table = "run" if name == WORKERS else "resources"
    for flags, file, word in (({name: value}, None, "flag"), (None, {table: {name: value}}, "configuration file")):
        problem = only(problems_of(flags, file))
        assert problem.subject == name
        assert problem.reason == ("range" if type(value) is int else "type")
        # Names the setting and the accepted range (29.11).
        assert problem.message.startswith(f"{word}: {name}: expected an integer in [1, inf), got ")


@pytest.mark.parametrize(
    ("name", "value", "reason"),
    [
        ("seed", -1, "range"),
        ("seed", SEED_MAX + 1, "range"),
        ("seed", 1.0, "type"),
        ("model_bundle", "", "range"),
        ("model_bundle", 3, "type"),
        ("model_bundle", "a\udc80", "range"),
        ("input.unwrap_arrays", 1, "type"),
        ("input.ordering_key", "created_at", "range"),
        ("input.ordering_key", "$.a b", "range"),
        ("output.inline_provenance", "true", "type"),
        ("corpus.exclude_ids", "a", "type"),
        ("corpus.exclude_ids", ["a", 1], "type"),
        ("corpus.exclude_ids", [""], "range"),
        ("ds.tol", -1e-9, "range"),
        ("subschema.timeout_s", 0, "range"),
        ("bootstrap.resamples", None, "type"),
    ],
)
def test_wrong_type_or_out_of_range(name: str, value: Any, reason: str) -> None:
    problem = only(problems_of({name: value}))
    assert (problem.subject, problem.reason) == (name, reason)
    assert problem.message.startswith(f"flag: {name}: expected ")


def test_unknown_settings_name_the_entry_and_suggest_the_right_place() -> None:
    problems = problems_of(
        {"run.seed": 1, "max_passes": 2, MDL: True},
        {"seed": 1, "extra": {}, "resources": {WORKERS: 2, "nope": 1}, "input": {"unwrap": True}},
    )
    assert all(p.reason == "unknown" for p in problems)
    messages = [p.message for p in problems]
    assert "flag: unknown setting 'run.seed' (did you mean 'seed'?)" in messages
    assert "flag: unknown setting 'max_passes'" in messages
    assert f"flag: unknown setting {MDL!r} (did you mean {technique_setting(MDL)!r}?)" in messages
    assert "configuration file: unknown setting [resources] workers (did you mean [run] workers?)" in messages
    assert "configuration file: unknown setting [resources] nope" in messages
    assert "configuration file: unknown setting [input] unwrap" in messages
    assert any(m.startswith("configuration file: unknown setting seed (did you mean [run] seed?)") for m in messages)
    assert any(m.startswith("configuration file: unknown table [extra]") for m in messages)
    assert len(problems) == 8


def test_a_table_that_is_not_a_table() -> None:
    problem = only(problems_of(None, {"run": 3}))
    assert (problem.subject, problem.reason) == ("run", "type")
    problem = only(problems_of(None, [1]))
    assert problem.reason == "type"


def test_quoted_and_dotted_key_for_one_setting_is_a_duplicate() -> None:
    file = load_toml('[resources]\n"sample.lof" = 5\nsample.lof = 6\n')
    problem = only(problems_of(None, file))
    assert (problem.subject, problem.reason) == ("sample.lof", "duplicate")


def test_invalid_file_values_are_reported_even_when_a_flag_overrides_them() -> None:
    problem = only(problems_of({"seed": 1}, {"run": {"seed": "one"}}))
    assert problem.message == "configuration file: seed: expected an integer in [0, 9223372036854775807], got the string 'one'"


def test_lsh_bands_must_not_exceed_minhash_perm() -> None:
    problem = only(problems_of({"lsh.bands": 64}, {"resources": {"minhash.perm": 32}}))
    assert (problem.subject, problem.reason) == ("lsh.bands", "range")
    assert problem.message == "flag: lsh.bands: lsh.bands (64) must not exceed minhash.perm (32)"
    # An invalid bound is reported once, without a relation problem built on it.
    problem = only(problems_of({"lsh.bands": 64, "minhash.perm": 0}))
    assert problem.subject == "minhash.perm"
    assert RunConfiguration.resolve({"lsh.bands": 64, "minhash.perm": 64}).resources["lsh.bands"] == 64


def test_unregistered_technique_ids_are_named_with_every_registered_id() -> None:
    problem = only(problems_of({"techniques.no.such": True}, {"techniques": {"also.missing": False}}))
    assert (problem.subject, problem.reason) == ("techniques", "unregistered")
    assert "'no.such' (flag)" in problem.message
    assert "'also.missing' (configuration file)" in problem.message
    listed = problem.message.split("the registered Technique ids are: ", 1)[1]
    assert listed.split(", ") == list(REGISTRY.ids())


def test_technique_toggles_take_booleans() -> None:
    problem = only(problems_of({technique_setting(MDL): "off"}))
    assert (problem.subject, problem.reason) == (technique_setting(MDL), "type")
    problem = only(problems_of(None, {"techniques": {MDL: 0}}))
    assert problem.message.startswith(f"configuration file: {technique_setting(MDL)}: expected a boolean")


def test_enabling_an_unimplemented_technique() -> None:
    # Any Technique that is not built yet and has no prerequisite (which would add a second problem).
    tid = next(t for t, spec in REGISTRY.techniques.items() if not spec.implemented and not spec.prerequisites)
    assert not REGISTRY.technique(tid).implemented
    problem = only(problems_of({technique_setting(tid): True}))
    assert (problem.subject, problem.reason) == (tid, "unimplemented")
    assert "(flag)" in problem.message


def test_enabled_technique_with_a_disabled_prerequisite() -> None:
    assert REGISTRY.technique(HYBRID).prerequisites == (MDL,)
    problems = problems_of({technique_setting(MDL): False}, {"techniques": {HYBRID: True}})
    assert [(p.subject, p.reason) for p in problems] == [(HYBRID, "unimplemented"), (HYBRID, "prerequisite")]
    message = problems[1].message
    assert f"{HYBRID!r} is enabled (configuration file)" in message
    assert f"prerequisite {MDL!r} is disabled (flag)" in message
    # With the prerequisite enabled too (decide.mdl is built, so on by default), only the unimplemented problem remains.
    assert REGISTRY.technique(MDL).implemented and REGISTRY.technique(MDL).default_enabled
    for flags in ({technique_setting(HYBRID): True}, {technique_setting(MDL): True, technique_setting(HYBRID): True}):
        assert [(p.subject, p.reason) for p in problems_of(flags)] == [(HYBRID, "unimplemented")]


def test_one_error_lists_every_problem_in_report_order() -> None:
    problems = problems_of(
        {
            WORKERS: 0,
            "sample.lof": 0,
            "bogus": 1,
            "lsh.bands": 512,
            "techniques.not.registered": True,
            technique_setting(MDL): False,
            technique_setting(HYBRID): True,
        },
        {"run": {"seed": -5}, "resources": {"sample.fd": 2.5}},
    )
    assert [(p.subject, p.reason) for p in problems] == [
        (WORKERS, "range"),
        ("sample.lof", "range"),
        ("bogus", "unknown"),
        ("seed", "range"),
        ("sample.fd", "type"),
        ("lsh.bands", "range"),
        ("techniques", "unregistered"),
        (HYBRID, "unimplemented"),
        (HYBRID, "prerequisite"),
    ]
    assert {p.reason for p in problems} <= set(CONFIG_PROBLEM_REASONS)


def test_flags_must_be_a_mapping() -> None:
    with pytest.raises(TypeError):
        RunConfiguration.resolve([("seed", 1)])  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Configuration files (27.11)


def test_load_toml_reports_unparseable_text() -> None:
    with pytest.raises(ConfigError) as caught:
        load_toml("[run\nseed = 1")
    problem = only(caught.value.problems)
    assert problem.reason == "parse"
    assert problem.message.startswith("configuration file cannot be parsed as TOML: ")
    with pytest.raises(ConfigError):
        RunConfiguration.from_toml("seed = = 1")


def test_read_config_file(tmp_path: Path) -> None:
    good = tmp_path / "good.toml"
    good.write_text(RunConfiguration.resolve({"seed": 4}).to_toml(), "utf-8")
    assert RunConfiguration.resolve(None, read_config_file(good)).seed == 4
    for name, data in (("bad.toml", b"[run\n"), ("latin1.toml", b'[run]\nmodel_bundle = "\xe9"\n')):
        path = tmp_path / name
        path.write_bytes(data)
        with pytest.raises(ConfigError) as caught:
            read_config_file(path)
        assert (caught.value.subjects, caught.value.problems[0].reason) == ((str(path),), "parse")
    with pytest.raises(ConfigError) as caught:
        read_config_file(tmp_path / "missing.toml")
    assert "cannot be read" in str(caught.value)


# ---------------------------------------------------------------------------
# Direct construction


def test_direct_construction_is_validated() -> None:
    config = RunConfiguration.default()
    assert dataclasses.replace(config, seed=7).seed == 7
    with pytest.raises(ConfigError) as caught:
        dataclasses.replace(config, seed=-1, techniques={**config.techniques, MDL: False, HYBRID: True})
    assert [(p.subject, p.reason) for p in caught.value.problems] == [
        ("seed", "range"),
        (HYBRID, "unimplemented"),
        (HYBRID, "prerequisite"),
    ]
    resources = dict(config.resources)
    del resources["sample.lof"]
    with pytest.raises(ConfigError, match="missing entries for 'sample.lof'"):
        dataclasses.replace(config, resources=resources)
    with pytest.raises(ConfigError):
        dataclasses.replace(config, provenance_of={})
    with pytest.raises(ConfigError):
        dataclasses.replace(config, input={"unwrap_arrays": True})
    with pytest.raises(ConfigError):
        InputOptions(ordering_key="nope")


def test_configurations_pickle() -> None:
    config = RunConfiguration.resolve({"seed": 3, "corpus.exclude_ids": ["a"]})
    back = pickle.loads(pickle.dumps(config))
    assert back == config
    assert dict(back.provenance_of) == dict(config.provenance_of)


# ---------------------------------------------------------------------------
# ConfigError


def test_config_error_messages_and_pickling() -> None:
    one = ConfigError([ConfigProblem("seed", "range", "seed: bad")])
    assert str(one) == "seed: bad"
    two = ConfigError([ConfigProblem("a", "type", "a: bad"), ConfigProblem("b", "unknown", "b: unknown")])
    assert str(two) == "2 configuration problems:\n  - a: bad\n  - b: unknown"
    assert two.subjects == ("a", "b")
    back = pickle.loads(pickle.dumps(two))
    assert back.problems == two.problems and str(back) == str(two)
    with pytest.raises(ValueError):
        ConfigError([])
    with pytest.raises(ValueError):
        ConfigProblem("a", "nonsense", "a")  # type: ignore[arg-type]
