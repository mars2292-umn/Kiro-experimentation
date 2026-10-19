"""Tests of the synthetic round-trip benchmark (task 29.1; Requirements 30.6, 31.1–31.14).

Stub systems through ``CommandAdapter``: one copies a given schema file
(so a copy of the truth scores perfectly), one writes the accept-everything
schema, one fails. The Inferrer adapter is checked for the
``corpus.exclude_ids`` setting without a run (30.6).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from jsinfer.bench.baselines.inferrer import InferrerAdapter
from jsinfer.bench.runner import CommandAdapter
from jsinfer.bench.splits import CorpusEntry, CorpusManifest, SchemaSplit
from jsinfer.bench.synthetic import Exclusion, SyntheticConfig, SyntheticData, _with_exclusions, evaluate, prepare, run_synthetic, split_excluded_ids
from jsinfer.io.load import SchemaLoader
from jsinfer.io.subschema import CheckFailure, Relation, SubschemaChecker, Unsupported
from jsinfer.io.validator import Validator
from jsinfer.learn.synth import GeneratorConfig

TRUTH = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
        "id": {"type": "integer", "minimum": 0},
        "kind": {"enum": ["a", "b"]},
        "email": {"type": "string", "format": "email"},
        "tags": {"type": "array", "items": {"type": "string"}},
    },
    "required": ["id", "kind"],
    "additionalProperties": False,
}
CFG = SyntheticConfig(sizes=(3, 6), held_out=5, seed=1, timeout_s=120, generator=GeneratorConfig(size_min=1, size_max=4))
COPY = "import shutil, sys\nshutil.copy(sys.argv[4], sys.argv[2] + '/schema.json')\n"
ACCEPT_ALL = "import json, sys\njson.dump({}, open(sys.argv[2] + '/schema.json', 'w'))\n"
FAIL = "import sys\nsys.exit(2)\n"


def _script(tmp_path: Path, name: str, body: str) -> Path:
    path = tmp_path / f"{name}.py"
    path.write_text(body, encoding="utf-8")
    return path


def _adapter(tmp_path: Path, name: str, body: str, *extra: str) -> CommandAdapter:
    script = _script(tmp_path, name, body)
    return CommandAdapter(name, name, (sys.executable, "-I", str(script), "{docs}", "{out}", "{seed}", *extra))


def _checker(relation: bool = True) -> SubschemaChecker:
    def runner(lhs, rhs, timeout_s):
        return relation if relation is not None else CheckFailure("error", "no checker")

    return SubschemaChecker(runner)


@pytest.fixture(scope="module")
def data() -> SyntheticData:
    prepared = prepare(SchemaLoader().load(TRUTH), "s1", CFG)
    assert isinstance(prepared, SyntheticData), prepared
    return prepared


def test_prepare_gives_training_collections_distinct_held_out_positives_and_negatives(data: SyntheticData) -> None:
    assert sorted(data.training) == [3, 6]
    assert [len(data.training[s]) for s in (3, 6)] == [3, 6]
    assert len(data.positives) == 5
    training = {json.dumps(d, sort_keys=True) for s in data.training for d in data.training[s]}
    held = [json.dumps(d, sort_keys=True) for d in data.positives]
    assert len(set(held)) == 5 and not training & set(held)
    assert data.negatives, "the mutation operators found no negatives"
    assert data.truth_document["$schema"].endswith("2020-12/schema")
    assert data.to_json()["positives"] == 5
    validator = Validator()
    compiled = validator.compile(data.truth_document)
    assert all(validator.is_valid(compiled, d) for d in data.positives)
    assert not any(validator.is_valid(compiled, n.document) for n in data.negatives)


def test_prepare_is_deterministic_and_seed_sensitive() -> None:
    model = SchemaLoader().load(TRUTH)
    a, b = prepare(model, "s1", CFG), prepare(model, "s1", CFG)
    assert isinstance(a, SyntheticData) and isinstance(b, SyntheticData)
    assert a.positives == b.positives and a.training == b.training
    other = prepare(model, "s1", SyntheticConfig(sizes=(3, 6), held_out=5, seed=2, generator=CFG.generator))
    assert isinstance(other, SyntheticData) and other.positives != a.positives


def test_prepare_excludes_a_schema_without_enough_distinct_documents() -> None:
    model = SchemaLoader().load({"type": "boolean"})
    cfg = SyntheticConfig(sizes=(2,), held_out=3, seed=0, generator=GeneratorConfig(doc_attempts=3))
    outcome = prepare(model, "bool", cfg)
    assert isinstance(outcome, Exclusion)
    assert "distinct from the training documents" in outcome.reason


def test_evaluate_scores_each_system_at_each_size(tmp_path: Path, data: SyntheticData) -> None:
    truth_file = tmp_path / "truth.json"
    truth_file.write_text(json.dumps(data.truth_document))
    systems = [
        _adapter(tmp_path, "copy", COPY, str(truth_file)),
        _adapter(tmp_path, "accept", ACCEPT_ALL),
        _adapter(tmp_path, "broken", FAIL),
    ]
    results = evaluate(data, systems, tmp_path / "runs", CFG, checker=_checker(True))
    assert [(r.size, r.system) for r in results] == [(3, "copy"), (3, "accept"), (3, "broken"), (6, "copy"), (6, "accept"), (6, "broken")]
    by = {(r.size, r.system): r for r in results}
    copy = by[(3, "copy")]
    assert copy.ok and copy.recall == 1.0 and copy.rejection == 1.0 and copy.harmonic == 1.0
    # format is outside the Subschema_Checker's coverage (31.8); the relation says so
    assert isinstance(copy.relation, Unsupported) and copy.relation.keywords == ("format",)
    assert copy.constraints is not None and copy.constraints.precision["required"] == 1.0 and copy.constraints.recall["format"] == 1.0
    assert all(v in (1.0, None) for v in copy.judgments.values())
    assert set(copy.operators) and all(rate.rate in (1.0, None) for rate in copy.operators.values())
    accept = by[(6, "accept")]
    assert accept.ok and accept.recall == 1.0 and accept.rejection == 0.0 and accept.harmonic == 0.0
    assert accept.constraints is not None and accept.constraints.precision["required"] is None and accept.constraints.recall["required"] == 0.0
    broken = by[(3, "broken")]
    assert not broken.ok and broken.missing == "failure" and broken.recall is None and broken.relation is None
    assert broken.reason == "exit status 2"
    assert (tmp_path / "runs" / "3" / "docs.jsonl").read_text().count("\n") == 3
    assert copy.to_json()["relation"]["keywords"] == ["format"]
    assert broken.to_json()["missing"] == "failure"


def test_relations_equivalent_and_unsupported(tmp_path: Path) -> None:
    truth = {k: v for k, v in TRUTH.items() if k != "properties"}
    truth["properties"] = {k: v for k, v in TRUTH["properties"].items() if k != "email"}
    data = prepare(SchemaLoader().load(truth), "plain", CFG)
    assert isinstance(data, SyntheticData)
    truth_file = tmp_path / "truth.json"
    truth_file.write_text(json.dumps(data.truth_document))
    systems = [_adapter(tmp_path, "copy", COPY, str(truth_file))]
    [first, _] = evaluate(data, systems, tmp_path / "runs", CFG, checker=_checker(True))
    assert first.relation is Relation.EQUIVALENT and first.to_json()["relation"] == {"relation": "equivalent"}
    [first, _] = evaluate(data, systems, tmp_path / "runs2", CFG, checker=_checker(None))
    assert isinstance(first.relation, Unsupported) and first.relation.cause == "error"
    assert first.to_json()["relation"]["relation"] == "unsupported"


def test_inferrer_gets_the_held_out_group_as_corpus_exclude_ids() -> None:
    adapter = InferrerAdapter(settings={"workers": 1})
    excluded = _with_exclusions(adapter, ["schemastore/a", "schemastore/a-v2"])
    assert isinstance(excluded, InferrerAdapter)
    assert excluded.settings == {"workers": 1, "corpus.exclude_ids": ["schemastore/a", "schemastore/a-v2"]}
    command = list(excluded.command(Path("/d.jsonl"), Path("/o"), 0))
    assert 'corpus.exclude_ids=["schemastore/a", "schemastore/a-v2"]' in command
    other = CommandAdapter("x", "x", ("x",))
    assert _with_exclusions(other, ["a"]) is other


def test_run_synthetic_over_a_split(tmp_path: Path) -> None:
    root = tmp_path / "corpus"
    root.mkdir()
    (root / "s1.json").write_text(json.dumps(TRUTH))
    (root / "s2.json").write_text(json.dumps({"type": "boolean"}))
    (root / "s3.json").write_text(json.dumps(TRUTH))
    (root / "s4.json").write_text("{not json")
    manifest = CorpusManifest.of([CorpusEntry(f"s{i}", "schemastore", f"s{i}.json") for i in range(1, 5)], root)
    split = SchemaSplit(
        "t", 0, ("t1",), ("v1",), ("s1", "s2", "s3", "s4"),
        {"groups": {"s1": "g1", "s3": "g1", "s2": "g2", "s4": "g4"}, "excluded": [{"id": "s3", "reason": "partially supported"}]},
    )
    assert split_excluded_ids(split) == {"s3": "partially supported"}
    accept = _adapter(tmp_path, "accept", ACCEPT_ALL)
    cfg = SyntheticConfig(sizes=(2,), held_out=3, seed=0, timeout_s=120, generator=GeneratorConfig(size_min=1, size_max=2, doc_attempts=3))
    out = run_synthetic(manifest, split, [accept], cfg, tmp_path / "runs", checker=_checker(True))
    assert {e.schema_id: e.reason.split(":")[0] for e in out.exclusions} == {
        "s2": "only 0 of 3 held-out documents distinct from the training documents within 9 candidates",
        "s3": "excluded by the Schema_Split",
        "s4": "the schema cannot be loaded",
    }
    assert [r.schema_id for r in out.results] == ["s1"]
    assert out.systems == ["accept"]
    assert out.aggregate("accept", 2, "recall").mean == 1.0
    assert out.aggregate("accept", 2, "rejection").mean == 0.0
    assert out.aggregate("accept", 2, "precision:required").undefined == 1
    assert out.missing("accept") == {}
    assert out.relations("accept", 2) == {"unsupported": 1}
    assert out.to_json()["data"]["s1"]["sizes"] == [2]


def test_config_from_mapping_and_validation() -> None:
    cfg = SyntheticConfig.from_mapping({"sizes": [5, 50], "held_out": 20, "timeout_s": 10, "folds": 3}, seed=4)
    assert cfg.sizes == (5, 50) and cfg.held_out == 20 and cfg.seed == 4 and cfg.timeout_s == 10
    assert SyntheticConfig().sizes == (10, 100, 1000) and SyntheticConfig().held_out == 1000
    with pytest.raises(ValueError, match="unknown"):
        SyntheticConfig.from_mapping({"size": [1]})
    with pytest.raises(ValueError, match="sizes"):
        SyntheticConfig(sizes=(0,))
    with pytest.raises(ValueError, match="held_out"):
        SyntheticConfig(held_out=0)
