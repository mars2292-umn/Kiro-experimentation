"""Tests of benchmark resume (task 41.5; Requirement 35.9).

The store itself (keys, atomic records, completion markers, discarding),
the fingerprint, the round trip of stored results, and resume through the
real and synthetic evaluations with stub systems that count their runs.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from jsinfer.bench.real import CleanedDataset, RealConfig, RealDataset, RealResult, clean, evaluate
from jsinfer.bench.resume import RECORD_FORMAT, ResumeStore, content_digest, fingerprint
from jsinfer.bench.runner import CommandAdapter
from jsinfer.bench.synthetic import SyntheticConfig, SyntheticData, SyntheticResult, prepare
from jsinfer.bench.synthetic import evaluate as evaluate_synthetic
from jsinfer.io.load import SchemaLoader
from jsinfer.io.subschema import SubschemaChecker
from jsinfer.learn.synth import GeneratorConfig

SCHEMA = {"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"]}
POSITIVES = [{"id": i} for i in range(6)]
# Appends one line to the counter file (argv[4]) per run, then writes a schema that accepts everything.
COUNTED = (
    "import json, sys\n"
    "open(sys.argv[4], 'a').write('run\\n')\n"
    "json.dump({}, open(sys.argv[2] + '/schema.json', 'w'))\n"
)
FAIL = "import sys\nopen(sys.argv[4], 'a').write('run\\n')\nsys.exit(3)\n"


def _adapter(tmp_path: Path, name: str, body: str) -> CommandAdapter:
    script = tmp_path / f"{name}.py"
    script.write_text(body, encoding="utf-8")
    return CommandAdapter(name, name, (sys.executable, "-I", str(script), "{docs}", "{out}", "{seed}", str(tmp_path / f"{name}.count")))


def _runs(tmp_path: Path, name: str) -> int:
    path = tmp_path / f"{name}.count"
    return path.read_text().count("\n") if path.exists() else 0


def _dataset() -> CleanedDataset:
    raw = tuple((f"d{i}", json.dumps(d).encode()) for i, d in enumerate(POSITIVES))
    cleaned = clean(RealDataset("ds", raw, SCHEMA, (("n0", b'{"id": "no"}'),)))
    assert isinstance(cleaned, CleanedDataset)
    return cleaned


# ---------------------------------------------------------------------------
# The store
# ---------------------------------------------------------------------------


def test_keys_are_canonical_and_cover_the_fingerprint(tmp_path: Path) -> None:
    store = ResumeStore(tmp_path, {"code": "a"})
    assert store.key("real", b=1, a=2).digest == store.key("real", a=2, b=1).digest
    assert store.key("real", a=1).digest != store.key("synthetic", a=1).digest
    assert store.key("real", a=1).digest != ResumeStore(tmp_path, {"code": "b"}).key("real", a=1).digest
    assert store.key("real", a=1).payload["format"] == RECORD_FORMAT
    assert content_digest([1, {"b": 2, "a": 1}]) == content_digest([1, {"a": 1, "b": 2}])


def test_records_round_trip_and_are_counted(tmp_path: Path) -> None:
    store = ResumeStore(tmp_path / "resume", {"code": "a"})
    key = store.key("real", dataset="x")
    assert store.load(key) is None and not (tmp_path / "resume").exists()
    store.save(key, {"recall": 0.5})
    assert store.load(key) == {"recall": 0.5}
    record = json.loads(store.path(key).read_bytes())
    assert record["complete"] is True and record["key"] == dict(key.payload)
    assert [p.name for p in (tmp_path / "resume").iterdir()] == [f"{key.digest}.json"]  # no temp file left
    assert store.summary() == {"reused": 1, "stored": 1, "discarded": 0}
    store.clear()
    assert not (tmp_path / "resume").exists() and store.load(key) is None


@pytest.mark.parametrize(
    "content",
    [b'{"format": 1, "key"', b"", b"[]", "incomplete", "other key", "no result"],
    ids=["truncated", "empty", "not an object", "incomplete", "other key", "no result"],
)
def test_partial_or_foreign_records_are_discarded(tmp_path: Path, content: bytes | str) -> None:
    store = ResumeStore(tmp_path, {"code": "a"})
    key = store.key("real", dataset="x")
    store.save(key, {"recall": 0.5})
    record = json.loads(store.path(key).read_bytes())
    if content == "incomplete":
        record["complete"] = False
        content = json.dumps(record).encode()
    elif content == "other key":
        record["key"] = dict(store.key("real", dataset="y").payload)
        content = json.dumps(record).encode()
    elif content == "no result":
        del record["result"]
        content = json.dumps(record).encode()
    assert isinstance(content, bytes)
    store.path(key).write_bytes(content)
    assert store.load(key) is None
    assert not store.path(key).exists() and store.discarded == 1


def test_the_fingerprint_follows_the_code_and_the_bundle(tmp_path: Path) -> None:
    plain = fingerprint()
    assert set(plain) == {"code"} and len(plain["code"]) == 64
    assert fingerprint() == plain  # deterministic
    assert fingerprint(tmp_path / "missing") == {**plain, "bundle": None}
    (tmp_path / "manifest.json").write_text('{"bundle_id": "one"}')
    one = fingerprint(tmp_path)
    (tmp_path / "manifest.json").write_text('{"bundle_id": "two"}')
    assert one["bundle"] and fingerprint(tmp_path)["bundle"] != one["bundle"]


# ---------------------------------------------------------------------------
# Results round trip
# ---------------------------------------------------------------------------


def test_real_results_round_trip() -> None:
    result = RealResult("ds", "sys", 6, 3, 1, "generated", 4, recall=0.5, rejection=None, harmonic=None, fold_rejection=(1.0, None, 0.5), seconds=2.5, peak_rss_mb=10.0, schema_size=3.0, dl_bits=12.5)
    assert RealResult.from_json(result.to_json()) == result
    missing = RealResult("ds", "sys", 6, 3, 0, "none", 0, missing="timeout", reason="exceeded 1 s", seconds=1.0, peak_rss_mb=0.0)
    assert RealResult.from_json(json.loads(json.dumps(missing.to_json()))) == missing


@pytest.fixture(scope="module")
def synthetic_data() -> SyntheticData:
    truth = {"type": "object", "properties": {"id": {"type": "integer"}, "email": {"type": "string", "format": "email"}}, "required": ["id"]}
    cfg = SyntheticConfig(sizes=(3,), held_out=4, seed=1, timeout_s=120, generator=GeneratorConfig(size_min=1, size_max=4))
    data = prepare(SchemaLoader().load(truth), "s1", cfg)
    assert isinstance(data, SyntheticData), data
    return data


def test_synthetic_results_round_trip(tmp_path: Path, synthetic_data: SyntheticData) -> None:
    cfg = SyntheticConfig(sizes=(3,), held_out=4, seed=1, timeout_s=120, generator=GeneratorConfig(size_min=1, size_max=4))
    copy = tmp_path / "copy.py"
    copy.write_text("import shutil, sys\nshutil.copy(sys.argv[4], sys.argv[2] + '/schema.json')\n")
    truth = tmp_path / "truth.json"
    truth.write_text(json.dumps(synthetic_data.truth_document))
    systems = [
        CommandAdapter("copy", "copy", (sys.executable, "-I", str(copy), "{docs}", "{out}", "{seed}", str(truth))),
        _adapter(tmp_path, "fail", FAIL),
    ]
    results = evaluate_synthetic(synthetic_data, systems, tmp_path / "runs", cfg, checker=SubschemaChecker(lambda a, b, t: True))
    assert any(r.relation is not None and r.constraints is not None for r in results) and any(r.missing for r in results)
    for result in results:
        again = SyntheticResult.from_json(json.loads(json.dumps(result.to_json())))
        assert again.to_json() == result.to_json()


# ---------------------------------------------------------------------------
# Resume through the evaluations
# ---------------------------------------------------------------------------


def test_a_resumed_real_evaluation_runs_nothing_and_gives_the_same_results(tmp_path: Path) -> None:
    dataset = _dataset()
    systems = [_adapter(tmp_path, "a", COUNTED), _adapter(tmp_path, "b", FAIL)]
    cfg = RealConfig(folds=3, seed=1, timeout_s=120)
    first = evaluate(dataset, systems, tmp_path / "runs", cfg, store=ResumeStore(tmp_path / "resume", {"code": "x"}))
    assert (_runs(tmp_path, "a"), _runs(tmp_path, "b")) == (3, 1)  # the failure stops after its first fold
    store = ResumeStore(tmp_path / "resume", {"code": "x"})
    second = evaluate(dataset, systems, tmp_path / "runs2", cfg, store=store)
    assert (_runs(tmp_path, "a"), _runs(tmp_path, "b")) == (3, 1)  # nothing ran again
    assert [r.to_json() for r in second] == [r.to_json() for r in first]
    assert store.summary() == {"reused": 2, "stored": 0, "discarded": 0}
    assert not (tmp_path / "runs2").exists()  # every system was stored, so no fold was even written


def test_only_absent_or_partial_combinations_are_recomputed(tmp_path: Path) -> None:
    dataset = _dataset()
    systems = [_adapter(tmp_path, "a", COUNTED), _adapter(tmp_path, "b", COUNTED), _adapter(tmp_path, "c", COUNTED)]
    cfg = RealConfig(folds=2, seed=1, timeout_s=120)
    store = ResumeStore(tmp_path / "resume", {"code": "x"})
    first = evaluate(dataset, systems, tmp_path / "runs", cfg, store=store)
    keys = {s.name: store.key("real", dataset="ds", content=content_digest([[list(p) for p in dataset.positives], [list(n) for n in dataset.negatives], dataset.schema, 0]), config=cfg.to_json(), system=s.describe().to_json()) for s in systems}
    assert all(store.path(k).is_file() for k in keys.values())
    store.path(keys["b"]).unlink()  # interrupted before b's record was written
    store.path(keys["c"]).write_bytes(store.path(keys["c"]).read_bytes()[:20])  # a partially written record
    again = ResumeStore(tmp_path / "resume", {"code": "x"})
    second = evaluate(dataset, systems, tmp_path / "runs", cfg, store=again)
    assert (_runs(tmp_path, "a"), _runs(tmp_path, "b"), _runs(tmp_path, "c")) == (2, 4, 4)
    assert again.summary() == {"reused": 1, "stored": 2, "discarded": 1}
    assert [r.to_json()["recall"] for r in second] == [r.to_json()["recall"] for r in first]


def test_a_changed_fingerprint_or_configuration_reruns_everything(tmp_path: Path) -> None:
    dataset = _dataset()
    systems = [_adapter(tmp_path, "a", COUNTED)]
    evaluate(dataset, systems, tmp_path / "runs", RealConfig(folds=2, seed=1, timeout_s=120), store=ResumeStore(tmp_path / "resume", {"code": "x"}))
    evaluate(dataset, systems, tmp_path / "runs", RealConfig(folds=2, seed=1, timeout_s=120), store=ResumeStore(tmp_path / "resume", {"code": "y"}))
    evaluate(dataset, systems, tmp_path / "runs", RealConfig(folds=2, seed=2, timeout_s=120), store=ResumeStore(tmp_path / "resume", {"code": "y"}))
    assert _runs(tmp_path, "a") == 6


def test_unavailable_systems_are_not_stored(tmp_path: Path) -> None:
    gone = CommandAdapter("gone", "gone", ("/nonexistent/system", "{docs}", "{out}"))
    store = ResumeStore(tmp_path / "resume", {"code": "x"})
    [result] = evaluate(_dataset(), [gone], tmp_path / "runs", RealConfig(folds=2, seed=1, timeout_s=120), store=store)
    assert result.missing == "unavailable" and store.stored == 0


def test_a_resumed_synthetic_evaluation_runs_nothing(tmp_path: Path, synthetic_data: SyntheticData) -> None:
    cfg = SyntheticConfig(sizes=(3,), held_out=4, seed=1, timeout_s=120, generator=GeneratorConfig(size_min=1, size_max=4))
    systems = [_adapter(tmp_path, "a", COUNTED)]
    checker = SubschemaChecker(lambda a, b, t: True)
    first = evaluate_synthetic(synthetic_data, systems, tmp_path / "runs", cfg, checker=checker, store=ResumeStore(tmp_path / "resume", {"code": "x"}))
    store = ResumeStore(tmp_path / "resume", {"code": "x"})
    second = evaluate_synthetic(synthetic_data, systems, tmp_path / "runs2", cfg, checker=checker, store=store)
    assert _runs(tmp_path, "a") == 1 and store.reused == 1
    assert [r.to_json() for r in second] == [r.to_json() for r in first]
    assert not (tmp_path / "runs2").exists()
