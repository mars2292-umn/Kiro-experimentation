"""Unit tests for the run metadata of the Inference_Report (task 19.1).

Requirements 27.8 (the ``run`` section: effective Run_Configuration, seed,
Model_Bundle id, jsinfer and CPython versions, the exact version of every
locked package), 27.12 (timestamps exempt from the reproducibility
comparison), and 20.5 (every Resource_Parameter read, with its origin,
components' reads included).
"""

from __future__ import annotations

import importlib.metadata
import platform
import re
import tomllib
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

import pytest

import jsinfer
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.errors import InferenceError
from jsinfer.core.report import InferenceReport, RunInfo
from jsinfer.pipeline import infer as pipeline
from jsinfer.pipeline.infer import LOCKED_PACKAGES, infer_documents, locked_package_versions, run_timestamp
from jsinfer.structure.analyzer import StructureAnalyzer

ROOT = Path(__file__).resolve().parents[2]
LOCK: dict[str, Any] = tomllib.loads((ROOT / "uv.lock").read_text("utf-8"))
TIMESTAMP = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z$")

# Two record families, so the Variant grouping Techniques run at the root.
DOCS: list[Any] = [
    {"id": i, "kind": "a", "x": i} if i % 2 else {"id": i, "kind": "b", "y": f"s{i}", "z": [i]} for i in range(30)
]


def config(**flags: Any) -> RunConfiguration:
    return RunConfiguration.resolve({"workers": 1, **flags})


def _normalize(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def _runtime_closure() -> dict[str, str]:
    """The packages uv.lock pins for jsinfer's runtime dependencies under any marker (no dev group), with versions."""
    packages = {_normalize(package["name"]): package for package in LOCK["package"]}

    def references(dependencies: list[dict[str, Any]]) -> list[tuple[str, tuple[str, ...]]]:
        return [(_normalize(dep["name"]), tuple(dep.get("extra", ()))) for dep in dependencies]

    seen: set[tuple[str, tuple[str, ...]]] = set()
    versions: dict[str, str] = {}
    pending = references(packages["jsinfer"].get("dependencies", []))
    while pending:
        name, extras = pending.pop()
        if (name, extras) in seen:
            continue
        seen.add((name, extras))
        package = packages[name]
        versions[name] = package["version"]
        pending.extend(references(package.get("dependencies", [])))
        for extra in extras:  # a dependency on name[extra] also needs the extra's packages
            pending.extend(references(package.get("optional-dependencies", {}).get(extra, [])))
    return dict(sorted(versions.items()))


def _installed(name: str) -> str | None:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return None


# ---------------------------------------------------------------------------
# Locked package versions (27.8)
# ---------------------------------------------------------------------------


def test_locked_packages_are_the_runtime_closure_of_the_lock_file() -> None:
    assert list(LOCKED_PACKAGES) == sorted(LOCKED_PACKAGES)
    assert list(LOCKED_PACKAGES) == list(_runtime_closure())
    dev = {_normalize(name) for name in ("pytest", "hypothesis", "regress")}
    assert "jsinfer" not in LOCKED_PACKAGES and not dev & set(LOCKED_PACKAGES)


def test_locked_package_versions_are_the_installed_locked_versions() -> None:
    versions = locked_package_versions()
    # The test environment is installed from uv.lock; a package needed only under another
    # marker (typing-extensions from Python 3.13 on) may be absent, and is then left out.
    expected = {name: version for name, version in _runtime_closure().items() if _installed(name) is not None}
    assert versions == expected
    assert list(versions) == sorted(versions)
    assert {"numpy", "scikit-learn", "jsonschema-rs", "joblib", "threadpoolctl"} <= set(versions)


def test_packages_that_are_not_installed_are_left_out(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(pipeline, "LOCKED_PACKAGES", ("no-such-package-for-jsinfer", "numpy"))
    assert locked_package_versions() == {"numpy": importlib.metadata.version("numpy")}


# ---------------------------------------------------------------------------
# The run section (27.8) and its timestamps (27.12)
# ---------------------------------------------------------------------------


def test_a_run_records_its_metadata() -> None:
    effective = config(seed=3)
    result = infer_documents(DOCS, effective)
    run = result.report.run
    assert run is not None
    assert run.config == effective.to_json() and run.seed == 3 and run.bundle_id == "default"
    assert run.jsinfer_version == jsinfer.__version__
    assert run.python_version == platform.python_version()
    assert dict(run.packages) == locked_package_versions()
    assert run.started is not None and run.finished is not None
    assert TIMESTAMP.match(run.started) and TIMESTAMP.match(run.finished)
    assert run.started <= run.finished
    data = result.report.to_json()["run"]
    members = ("bundle_id", "config", "finished", "jsinfer_version", "packages", "python_version", "seed", "started")
    assert list(data) == list(members)  # type: ignore[arg-type]
    assert RunInfo.from_json(data) == run


class FakeWallClock:
    """A wall clock that advances one minute per reading."""

    def __init__(self, start: datetime) -> None:
        self.now = start

    def __call__(self) -> datetime:
        self.now += timedelta(minutes=1)
        return self.now


def test_timestamps_bracket_the_run_and_are_exempt(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(pipeline, "_wall_clock", FakeWallClock(datetime(2026, 1, 1, tzinfo=timezone.utc)))
    first = infer_documents(DOCS, config())
    run = first.report.run
    assert run is not None
    assert (run.started, run.finished) == ("2026-01-01T00:01:00.000000Z", "2026-01-01T00:02:00.000000Z")
    later = timezone(timedelta(hours=5))
    monkeypatch.setattr(pipeline, "_wall_clock", FakeWallClock(datetime(2027, 6, 1, 5, 0, tzinfo=later)))
    second = infer_documents(DOCS, config())
    assert second.report.run is not None and second.report.run.started == "2027-06-01T00:01:00.000000Z"  # in UTC
    assert first.report.to_json()["run"] != second.report.to_json()["run"]
    assert first.report.comparable_json() == second.report.comparable_json()  # 27.12
    # Timestamps live only in the run section, and runtimes only in timings.
    stripped = first.report.comparable_json()
    assert "2026-01-01" not in str(stripped) and "timings" not in stripped


def test_a_failed_run_records_when_it_ended(monkeypatch: pytest.MonkeyPatch) -> None:
    def fail(self: StructureAnalyzer, view: Any) -> Any:
        raise RuntimeError("injected failure")

    monkeypatch.setattr(StructureAnalyzer, "infer_model", fail)
    with pytest.raises(InferenceError) as caught:
        infer_documents(DOCS, config())
    report = caught.value.partial_report
    assert report is not None and report.run is not None
    assert report.run.started is not None and report.run.finished is not None


def test_run_timestamps_are_utc() -> None:
    stamp = run_timestamp()
    assert TIMESTAMP.match(stamp)
    parsed = datetime.strptime(stamp, "%Y-%m-%dT%H:%M:%S.%fZ").replace(tzinfo=timezone.utc)
    assert abs(parsed - datetime.now(timezone.utc)) < timedelta(minutes=5)


def test_finish_run_completes_the_recorded_run_once() -> None:
    report = InferenceReport()
    with pytest.raises(ValueError, match="not recorded"):
        report.finish_run("2026-01-01T00:00:00Z")
    report.set_run(RunInfo({"seed": 0}, 0, "b", "v", "p", {"numpy": "2.4.6"}, started="2026-01-01T00:00:00Z"))
    report.finish_run("2026-01-01T00:05:00Z")
    assert report.run is not None and report.run.finished == "2026-01-01T00:05:00Z"
    assert report.run.started == "2026-01-01T00:00:00Z" and dict(report.run.packages) == {"numpy": "2.4.6"}
    with pytest.raises(ValueError, match="already recorded"):
        report.finish_run("2026-01-01T00:06:00Z")
    unfinished = InferenceReport()
    unfinished.set_run(RunInfo({"seed": 0}, 0, "b", "v", "p"))
    with pytest.raises(ValueError, match="non-empty"):
        unfinished.finish_run("")
    with pytest.raises(TypeError):
        unfinished.finish_run(None)  # type: ignore[arg-type]
    assert unfinished.run is not None and unfinished.run.finished is None


# ---------------------------------------------------------------------------
# Resource_Parameters read during the run (20.5)
# ---------------------------------------------------------------------------


def test_a_run_lists_the_resource_parameters_its_components_read() -> None:
    result = infer_documents(DOCS, config(**{"variants.max_components": 3}))
    listed = {name: (use.value, use.origin) for name, use in result.report.resources.items()}
    assert listed == {
        "ingest.chunk_size": (2_000, "default"),
        "soundness.max_passes": (5, "default"),
        # The Bernoulli mixture runs at the root (the default bundle has no Learned_Thresholds, so the
        # other grouping Techniques do not run and read nothing).
        "variants.em_max_iter": (100, "default"),
        "variants.em_tol": (1e-6, "default"),
        "variants.max_components": (3, "flag"),
        "workers": (1, "flag"),
        # Field_Group mining (6) runs at a Variant with optional fields.
        "fpgrowth.min_support": (2, "default"),
        "fpgrowth.max_itemsets": (2_000, "default"),
        "sample.fpgrowth": (50_000, "default"),
    }


def test_a_run_without_variant_groupings_reads_no_variant_resources() -> None:
    result = infer_documents([{"a": 1}, {"a": 2}, [1, 2], "x"], config())
    assert not any(name.startswith(("variants.", "minhash.", "lsh.", "sample.")) for name in result.report.resources)
