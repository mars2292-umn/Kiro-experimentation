"""Smoke tests for the project lock file (task 1.3; Requirement 29.5).

``uv.lock`` must pin every direct and transitive dependency, including the
dev group used by the Trainer and the Evaluation_Harness tests, to exactly one
version, and the environment the tests run in must be the one it describes.
"""

from __future__ import annotations

import importlib.metadata
import os
import re
import shutil
import subprocess
import sys
import sysconfig
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
LOCK: dict[str, Any] = tomllib.loads((ROOT / "uv.lock").read_text("utf-8"))
PYPROJECT: dict[str, Any] = tomllib.loads((ROOT / "pyproject.toml").read_text("utf-8"))
PACKAGES: list[dict[str, Any]] = LOCK["package"]
PROJECT = PYPROJECT["project"]["name"]

# A normalized PEP 440 version: one exact release, no ranges or wildcards.
EXACT_VERSION = re.compile(r"^(\d+!)?\d+(\.\d+)*((a|b|rc)\d+)?(\.post\d+)?(\.dev\d+)?(\+[a-z0-9]+(\.[a-z0-9]+)*)?$")
EXACT_PIN = re.compile(r"^\s*(?P<name>[A-Za-z0-9][A-Za-z0-9._-]*)(\[[^\]]*\])?\s*==\s*(?P<version>[^\s;]+)\s*(;.*)?$")
SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")


def _normalize(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def _locked_versions() -> dict[str, str]:
    return {_normalize(package["name"]): package["version"] for package in PACKAGES}


def _direct_requirements() -> list[str]:
    groups = PYPROJECT.get("dependency-groups", {})
    return [*PYPROJECT["project"]["dependencies"], *(r for name in sorted(groups) for r in groups[name])]


def _dependency_references(package: dict[str, Any]) -> list[dict[str, Any]]:
    references = list(package.get("dependencies", []))
    for table in ("optional-dependencies", "dev-dependencies"):
        groups = package.get(table, {})
        references.extend(r for name in sorted(groups) for r in groups[name])
    return references


def test_lock_lists_each_package_once() -> None:
    counts = Counter(_normalize(package["name"]) for package in PACKAGES)
    duplicated = sorted(name for name, count in counts.items() if count > 1)
    assert not duplicated, f"uv.lock resolves these packages to more than one version: {duplicated}"
    assert _normalize(PROJECT) in counts


def test_each_locked_package_pins_one_exact_version() -> None:
    inexact = sorted(
        f"{package['name']} {package.get('version')!r}"
        for package in PACKAGES
        if not isinstance(package.get("version"), str) or not EXACT_VERSION.match(package["version"])
    )
    assert not inexact, f"uv.lock entries without one exact version: {inexact}"


def test_registry_packages_pin_hashed_artifacts() -> None:
    problems: list[str] = []
    for package in PACKAGES:
        source = package.get("source", {})
        if "registry" in source:
            artifacts = [*([package["sdist"]] if "sdist" in package else []), *package.get("wheels", [])]
            if not artifacts:
                problems.append(f"{package['name']}: no sdist or wheel")
            problems.extend(
                f"{package['name']}: {artifact.get('url')} has no sha256 hash"
                for artifact in artifacts
                if not SHA256.match(artifact.get("hash", ""))
            )
        elif "git" in source:
            if not re.search(r"#[0-9a-f]{40}$", source["git"]):
                problems.append(f"{package['name']}: git source not pinned to a commit")
        elif _normalize(package["name"]) != _normalize(PROJECT):
            problems.append(f"{package['name']}: unpinned source {source}")
    assert not problems, "\n".join(problems)


def test_every_dependency_reference_is_locked_unambiguously() -> None:
    """The lock covers the transitive closure, and no reference needs a version to disambiguate."""
    locked = _locked_versions()
    problems: list[str] = []
    for package in PACKAGES:
        for reference in _dependency_references(package):
            name = _normalize(reference["name"])
            if name not in locked:
                problems.append(f"{package['name']} depends on {reference['name']}, which uv.lock does not pin")
            if "version" in reference or "source" in reference:
                problems.append(f"{package['name']} -> {reference['name']} is disambiguated by version or source")
    assert not problems, "\n".join(problems)


def test_direct_dependencies_are_exact_pins_matching_the_lock() -> None:
    locked = _locked_versions()
    problems: list[str] = []
    for requirement in [*_direct_requirements(), *PYPROJECT["build-system"]["requires"]]:
        match = EXACT_PIN.match(requirement)
        if not match or not EXACT_VERSION.match(match.group("version")):
            problems.append(f"{requirement!r} is not an exact '==' pin")
    for requirement in _direct_requirements():
        match = EXACT_PIN.match(requirement)
        if match:
            name, version = _normalize(match.group("name")), match.group("version")
            if locked.get(name) != version:
                problems.append(f"{requirement!r} but uv.lock has {name} {locked.get(name)!r}")
    assert not problems, "\n".join(problems)


def test_installed_distributions_match_the_lock() -> None:
    """The test environment was installed from uv.lock (Requirement 29.1)."""
    direct = {_normalize(m.group("name")) for r in _direct_requirements() if (m := EXACT_PIN.match(r))}
    problems: list[str] = []
    for name, version in sorted(_locked_versions().items()):
        try:
            installed = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            # Platform-specific transitive packages (colorama on Windows) may be absent.
            if name in direct:
                problems.append(f"{name} is a direct dependency but is not installed")
            continue
        if installed != version:
            problems.append(f"{name} {installed} is installed but uv.lock pins {version}")
    assert not problems, "\n".join(problems)


def _find_uv() -> str | None:
    """The uv executable: ``$UV``, then PATH, then the usual install locations that are often not on PATH.

    pip's ``--user`` install puts the binary in the user scripts directory of
    the interpreter that ran pip (``~/Library/Python/3.x/bin`` for macOS
    framework builds), and uv's standalone installer uses ``~/.local/bin`` or
    ``~/.cargo/bin``. The test environment's interpreter may be a different
    Python, so a pip-installed uv package is also located by asking each
    Python on PATH (``uv.find_uv_bin()``).
    """
    explicit = os.environ.get("UV")
    if explicit:
        return explicit
    found = shutil.which("uv")
    if found:
        return found
    home = Path.home()
    directories = [
        home / ".local" / "bin",
        home / ".cargo" / "bin",
        Path(sys.executable).parent,
        Path(sysconfig.get_path("scripts", sysconfig.get_preferred_scheme("user"))),
    ]
    found = shutil.which("uv", path=os.pathsep.join(str(directory) for directory in directories))
    if found:
        return found
    for name in ("python3", "python"):
        interpreter = shutil.which(name)
        if not interpreter:
            continue
        result = subprocess.run(
            [interpreter, "-c", "import uv; print(uv.find_uv_bin())"],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        candidate = result.stdout.strip()
        if result.returncode == 0 and candidate and os.access(candidate, os.X_OK):
            return candidate
    return None


def test_lock_is_current_with_pyproject() -> None:
    uv = _find_uv()
    if not uv:
        pytest.skip("uv is not available (install it, or set UV to its path)")
    result = subprocess.run(
        [uv, "lock", "--check", "--offline"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    assert result.returncode == 0, f"uv.lock is out of date with pyproject.toml:\n{result.stderr}"
