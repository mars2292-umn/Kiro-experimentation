"""Smoke tests for the project scaffolding (task 1.1)."""

from __future__ import annotations

import importlib
import pkgutil

import pytest
from hypothesis import settings

import jsinfer
from jsinfer import cli

SUBPACKAGES = [
    "config",
    "core",
    "io",
    "mdl",
    "decide",
    "structure",
    "deps",
    "values",
    "robust",
    "corpus",
    "learn",
    "pipeline",
    "bench",
    "bench.baselines",
]


@pytest.mark.parametrize("name", SUBPACKAGES)
def test_subpackage_and_modules_import(name: str) -> None:
    package = importlib.import_module(f"jsinfer.{name}")
    modules = [m.name for m in pkgutil.iter_modules(package.__path__)]
    assert modules, f"jsinfer.{name} has no modules"
    for module in modules:
        importlib.import_module(f"jsinfer.{name}.{module}")


def test_version_is_installed_metadata() -> None:
    assert jsinfer.__version__ == "0.1.0"


def test_cli_version_exits_zero(capsys: pytest.CaptureFixture[str]) -> None:
    with pytest.raises(SystemExit) as exc:
        cli.main(["--version"])
    assert exc.value.code == 0
    assert "jsinfer 0.1.0" in capsys.readouterr().out


def test_cli_without_command_is_usage_error() -> None:
    assert cli.main([]) == cli.EXIT_USAGE


def test_hypothesis_profiles_registered() -> None:
    ci = settings.get_profile("ci")
    assert ci.max_examples == 100
    assert ci.derandomize is True
    assert settings.get_profile("nightly").max_examples == 500
