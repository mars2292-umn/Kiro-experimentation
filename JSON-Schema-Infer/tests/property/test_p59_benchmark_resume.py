"""Property 59: Benchmark resume (task 41.6; Requirement 35.9).

For any interruption point in a benchmark run, restarting with the same
configuration and seed reuses exactly the combinations whose results were
fully written and recomputes the rest; the final results equal those of an
uninterrupted run.

Setup
    One uninterrupted real-data run (``bench.real.run_real``) over two
    Real_Datasets and two stub systems that count their runs records the
    order in which its four (system, dataset) results were stored.

Each example
    An interruption after ``k`` stored combinations is reconstructed in a
    fresh store: the first ``k`` records, and for the ``k + 1``-th either no
    record, a truncated one (interrupted mid-write), or one without the
    completion marker. The run is restarted with the same configuration and
    seed. Checks: exactly ``k`` records are reused and the partial one (if
    any) discarded; the stub systems run exactly ``folds`` times for each
    combination that was not reused; and the results equal the
    uninterrupted run's in every field but the measured ``seconds`` and
    ``peak_rss_mb`` of the recomputed combinations, which are new
    measurements.
"""

# Feature: json-schema-inference, Property 59: Benchmark resume

from __future__ import annotations

import json
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.bench.real import RealConfig, RealDataset, run_real
from jsinfer.bench.resume import ResumeKey, ResumeStore
from jsinfer.bench.runner import CommandAdapter

SCHEMA = {"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"]}
CFG = RealConfig(folds=2, seed=3, timeout_s=120)
FINGERPRINT = {"code": "p59"}
COUNTED = (
    "import json, sys\n"
    "open(sys.argv[4], 'a').write('run\\n')\n"
    "json.dump({'type': 'object', 'required': ['id']} if sys.argv[5] == 'strict' else {}, open(sys.argv[2] + '/schema.json', 'w'))\n"
)
MEASURED = ("seconds", "peak_rss_mb")


@dataclass
class LoggingStore(ResumeStore):
    """A ResumeStore that remembers the order in which records were saved."""

    order: list[ResumeKey] | None = None

    def save(self, key: ResumeKey, result) -> None:  # type: ignore[no-untyped-def]
        super().save(key, result)
        if self.order is None:
            self.order = []
        self.order.append(key)


@dataclass(frozen=True)
class Baseline:
    root: Path
    datasets: list[RealDataset]
    systems: list[CommandAdapter]
    store: Path
    order: list[ResumeKey]
    results: list[dict]


def _runs(root: Path) -> int:
    return sum(p.read_text().count("\n") for p in root.glob("*.count"))


def _raw(documents: list) -> tuple[tuple[str, bytes], ...]:
    return tuple((f"d{i}", json.dumps(d).encode()) for i, d in enumerate(documents))


@pytest.fixture(scope="module")
def baseline(tmp_path_factory: pytest.TempPathFactory) -> Baseline:
    root = tmp_path_factory.mktemp("p59")
    script = root / "counted.py"
    script.write_text(COUNTED, encoding="utf-8")
    systems = [
        CommandAdapter(name, name, (sys.executable, "-I", str(script), "{docs}", "{out}", "{seed}", str(root / f"{name}.count"), mode))
        for name, mode in (("loose", "loose"), ("strict", "strict"))
    ]
    datasets = [
        RealDataset("alpha", _raw([{"id": i} for i in range(4)]), SCHEMA, (("n0", b'{"id": "x"}'),)),
        RealDataset("beta", _raw([{"id": i, "x": i} for i in range(5)] + [{"x": 0}]), {"type": "object"}),
    ]
    store = LoggingStore(root / "resume", FINGERPRINT)
    results = run_real(datasets, systems, CFG, root / "runs", store=store)
    assert store.order is not None and len(store.order) == 4
    return Baseline(root, datasets, systems, root / "resume", list(store.order), [r.to_json() for r in results.results])


def _comparable(result: dict) -> dict:
    return {k: v for k, v in result.items() if k not in MEASURED}


@given(k=st.integers(0, 4), partial=st.sampled_from(["absent", "truncated", "incomplete"]))
def test_a_restarted_run_reuses_exactly_the_completed_combinations(baseline: Baseline, k: int, partial: str) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        store_dir = Path(tmp) / "resume"
        store_dir.mkdir()
        for key in baseline.order[:k]:
            name = f"{key.digest}.json"
            shutil.copy(baseline.store / name, store_dir / name)
        discarded = 0
        if k < len(baseline.order) and partial != "absent":
            source = (baseline.store / f"{baseline.order[k].digest}.json").read_bytes()
            if partial == "truncated":
                damaged = source[: len(source) // 2]
            else:
                record = json.loads(source)
                record.pop("complete")
                damaged = json.dumps(record).encode()
            (store_dir / f"{baseline.order[k].digest}.json").write_bytes(damaged)
            discarded = 1
        event(f"interrupted after {k} of {len(baseline.order)} ({partial})")
        before = _runs(baseline.root)
        store = ResumeStore(store_dir, FINGERPRINT)
        results = run_real(baseline.datasets, baseline.systems, CFG, Path(tmp) / "runs", store=store)
        ran = _runs(baseline.root) - before
    assert store.reused == k and store.discarded == discarded and store.stored == len(baseline.order) - k
    assert ran == CFG.folds * (len(baseline.order) - k)
    got = [r.to_json() for r in results.results]
    assert [_comparable(r) for r in got] == [_comparable(r) for r in baseline.results]
    for i, (mine, theirs) in enumerate(zip(got, baseline.results)):
        position = [o.digest for o in baseline.order].index(_digest_of(baseline, mine))
        if position < k:
            assert mine == theirs, f"result {i} was reused, so even its measurements are the stored ones"


def _digest_of(baseline: Baseline, result: dict) -> str:
    """The digest of the baseline record holding ``result``'s (dataset, system) combination."""
    for key in baseline.order:
        payload = key.payload
        if payload["dataset"] == result["dataset_id"] and payload["system"]["name"] == result["system"]:
            return key.digest
    raise AssertionError(f"no record for {result['dataset_id']} / {result['system']}")
