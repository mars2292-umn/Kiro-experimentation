"""Benchmark smoke tests (task 30.6; Requirements 30.8, 34.6, 34.7, 34.8, 35.8).

``jsinfer bench`` runs end to end on the fixture corpus (two of its held-out
schemas, small collections) with the real Inferrer, Naive_Merge, and stub
command systems that exercise the failure, timeout, and unavailable paths,
and every output format is checked. The leakage check aborts a run whose
Model_Bundle has no training manifest (30.8, 30.11).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from jsinfer import cli
from jsinfer.bench.reporting import read_csv
from jsinfer.bench.splits import CorpusManifest, SchemaSplit
from jsinfer.io.load import SchemaLoader
from jsinfer.learn.synth import GeneratorConfig, SyntheticGenerator
from tests.conftest import FIXTURE_MANIFEST, FIXTURE_SPLIT

SCRIPTS = {
    "ok": "import json, sys\njson.dump({'type': 'object'}, open(sys.argv[2] + '/schema.json', 'w'))\n",
    "fail": "import sys\nsys.stderr.write('boom\\n')\nsys.exit(4)\n",
    "slow": "import time\ntime.sleep(60)\n",
}


def _toml_list(items: list[str]) -> str:
    return "[" + ", ".join(json.dumps(i) for i in items) + "]"


@pytest.fixture(scope="module")
def workspace(tmp_path_factory: pytest.TempPathFactory, tiny_bundle: str) -> Path:
    root = tmp_path_factory.mktemp("bench")
    for name, body in SCRIPTS.items():
        (root / f"{name}.py").write_text(body, encoding="utf-8")
    split = SchemaSplit.load(FIXTURE_SPLIT)
    manifest = CorpusManifest.load(FIXTURE_MANIFEST)
    held = list(split.held_out[:2])
    # A Real_Dataset: generated documents of a held-out fixture schema with its schema as the known schema.
    real = root / "real"
    real.mkdir()
    schema_id = held[0]
    model = SchemaLoader().load(manifest.schema_path(schema_id).read_bytes())
    docs = list(SyntheticGenerator().documents(model, 6, (0, schema_id, 9), GeneratorConfig(size_min=1, size_max=3)))
    (real / "docs.jsonl").write_text("".join(json.dumps(d) + "\n" for d in docs), encoding="utf-8")
    (real / "schema.json").write_text(manifest.schema_path(schema_id).read_text(encoding="utf-8"), encoding="utf-8")
    (real / "manifest.json").write_text(json.dumps({"datasets": [{"id": "fixture-real", "documents": "docs.jsonl", "schema": "schema.json"}, {"id": "broken"}]}), encoding="utf-8")
    python = json.dumps(sys.executable)

    def command(name: str, argv0: str = python) -> str:
        return f'[baselines.command.{name}]\nargv = [{argv0}, "-I", {json.dumps(str(root / f"{name}.py"))}, "{{docs}}", "{{out}}", "{{seed}}"]\n'

    (root / "bench.toml").write_text(
        f'[corpus]\nmanifest = {json.dumps(str(FIXTURE_MANIFEST))}\n\n[split]\nout = {json.dumps(str(FIXTURE_SPLIT))}\nseed = 0\n\n'
        f'[bench]\nsizes = [4]\nheld_out = 3\nschemas = {_toml_list(held)}\ntimeout_s = 3\nmodel_bundle = {json.dumps(tiny_bundle)}\n'
        f'results = {json.dumps(str(root / "results"))}\nreal_manifest = {json.dumps(str(real / "manifest.json"))}\nschemastore_datasets = false\n'
        f'resamples = 50\nfolds = 2\nablations = ["variants.minhash_lsh"]\n[bench.generator]\nsize_min = 1\nsize_max = 4\ndoc_attempts = 5\n\n'
        f'[baselines]\nnaive_merge = true\n[baselines.inferrer]\nworkers = 1\n'
        + command("ok") + command("fail") + command("slow") + command("gone", json.dumps(str(root / "no-such-python"))),
        encoding="utf-8",
    )
    (root / "leaky.toml").write_text(
        f'[corpus]\nmanifest = {json.dumps(str(FIXTURE_MANIFEST))}\n\n[split]\nout = {json.dumps(str(FIXTURE_SPLIT))}\n\n'
        f'[bench]\nsizes = [4]\nheld_out = 3\nschemas = {_toml_list(held)}\nmodel_bundle = "bundles/default"\nresults = {json.dumps(str(root / "leaky"))}\n',
        encoding="utf-8",
    )
    return root


def _statuses(out: Path) -> dict[str, set[str]]:
    rows = read_csv(out / "results.csv")
    found: dict[str, set[str]] = {}
    for row in rows:
        found.setdefault(str(row["system"]), set()).add(str(row["status"]))
    return found


def _check_outputs(out: Path) -> None:
    assert {p.name for p in out.iterdir() if p.is_file()} == {"results.csv", "results.json", "aggregates.csv", "aggregates.json", "summary.md", "summary.tex"}
    assert read_csv(out / "results.csv") == json.loads((out / "results.json").read_text(encoding="utf-8"))
    summary = (out / "summary.md").read_text(encoding="utf-8")
    assert "GONE (unavailable)" not in summary and "gone (unavailable)" in summary
    assert "\\begin{tabular}" in (out / "summary.tex").read_text(encoding="utf-8")
    aggregates = json.loads((out / "aggregates.json").read_text(encoding="utf-8"))
    assert aggregates["unavailable"] == ["gone"]


def test_synthetic_benchmark_end_to_end(workspace: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert cli.main(["bench", "synthetic", "-c", str(workspace / "bench.toml")]) == 0
    err = capsys.readouterr().err
    assert "synthetic:" in err and "unavailable: gone" in err
    out = workspace / "results" / "synthetic"
    _check_outputs(out)
    statuses = _statuses(out)
    assert statuses["jsinfer"] == {"ok"} and statuses["naive_merge"] == {"ok"} and statuses["ok"] == {"ok"}
    assert statuses["fail"] == {"failure"} and statuses["slow"] == {"timeout"} and statuses["gone"] == {"unavailable"}
    rows = read_csv(out / "results.csv")
    inferrer = [r for r in rows if r["system"] == "jsinfer"]
    assert len(inferrer) == 2 and all(r["recall"] is not None and r["schema_size"] and r["seconds"] for r in inferrer)
    fail = next(r for r in rows if r["system"] == "fail")
    assert fail["reason"] == "exit status 4" and fail["recall"] is None
    assert (out / "runs").is_dir()


def test_real_benchmark_end_to_end(workspace: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert cli.main(["bench", "real", "-c", str(workspace / "bench.toml")]) == 0
    assert "real:" in capsys.readouterr().err
    out = workspace / "results" / "real"
    _check_outputs(out)
    statuses = _statuses(out)
    assert statuses["jsinfer"] == {"ok"} and statuses["fail"] == {"failure"} and statuses["slow"] == {"timeout"} and statuses["gone"] == {"unavailable"}
    rows = read_csv(out / "results.csv")
    mine = next(r for r in rows if r["system"] == "jsinfer")
    assert mine["n"] == 6 and mine["folds"] == 2 and mine["negatives_source"] in ("generated", "none")
    summary = (out / "summary.md").read_text(encoding="utf-8")
    assert summary.index("primary comparison") < summary.index("Paired comparisons")
    assert "the definition lacks" in summary  # the "broken" definition is reported as excluded


def test_a_restarted_real_benchmark_resumes_and_writes_the_same_results(workspace: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """35.9: the second run of the same configuration reuses every stored result; --fresh reruns them."""
    out = workspace / "results" / "real"
    if not (out / "results.json").exists():
        assert cli.main(["bench", "real", "-c", str(workspace / "bench.toml")]) == 0
        capsys.readouterr()
    before = (out / "results.json").read_bytes()
    assert cli.main(["bench", "real", "-c", str(workspace / "bench.toml")]) == 0
    assert "resumed" in capsys.readouterr().err
    assert (out / "results.json").read_bytes() == before
    assert cli.main(["bench", "real", "-c", str(workspace / "bench.toml"), "--fresh"]) == 0
    assert "resumed" not in capsys.readouterr().err
    assert any((out / "resume").iterdir())  # the fresh run stored its results again


def test_ablation_study_end_to_end(workspace: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert cli.main(["bench", "ablation", "-c", str(workspace / "bench.toml")]) == 0
    assert "ablation:" in capsys.readouterr().err
    out = workspace / "results" / "ablation"
    assert read_csv(out / "results.csv") == json.loads((out / "results.json").read_text(encoding="utf-8"))
    statuses = _statuses(out)
    assert set(statuses) == {"jsinfer", "ablate:variants.minhash_lsh"} and all(v == {"ok"} for v in statuses.values())
    aggregates = json.loads((out / "aggregates.json").read_text(encoding="utf-8"))
    assert aggregates["extra"]["ablation"]["configs"][0]["technique"] == "variants.minhash_lsh"
    assert "recall@4" in aggregates["extra"]["ablation"]["differences"]


def test_leakage_check_aborts_before_any_run(workspace: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert cli.main(["bench", "synthetic", "-c", str(workspace / "leaky.toml")]) == 1
    err = capsys.readouterr().err
    assert "aborted before any run" in err and "manifest" in err
    assert not (workspace / "leaky").exists()
