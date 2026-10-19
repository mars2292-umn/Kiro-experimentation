"""Unit tests for ``bench.toml``, the split builder, and ``jsinfer split`` (task 25.2; Requirements 30.2–30.5, 30.10)."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from jsinfer import cli
from jsinfer.bench.config import BenchConfig
from jsinfer.bench.corpus_loader import SCHEMASTORE_CATALOG, SCHEMASTORE_SCHEMAS
from jsinfer.bench.splits import SPLITS, CorpusManifest, SchemaSplit, SplitBuilder, SplitError, SplitInputs, near_duplicate_cutoff
from jsinfer.config.errors import ConfigError
from tests.conftest import FIXTURE_MANIFEST


def write_toml(path: Path, text: str) -> Path:
    path.write_text(text, encoding="utf-8")
    return path


def test_bench_config_reads_paths_relative_to_the_file(tmp_path: Path) -> None:
    config = BenchConfig.from_toml(
        write_toml(
            tmp_path / "bench.toml",
            '[corpus]\nmanifest = "corpus/manifest.json"\nschemastore = "ss"\nschemastore_commit = "c1"\n'
            '[split]\nout = "split.json"\nseed = 4\nknown_schemas = ["b", "a", "a"]\n'
            '[split.known_schema_files]\nreal = "known.json"\n'
            '[resources]\n"split.train" = 0.6\n"split.val" = 0.2\n"split.held_out" = 0.2\n"minhash.perm" = 16\n[datasets]\nx = 1\n',
        )
    )
    assert config.manifest == str(tmp_path / "corpus" / "manifest.json") and config.split_out == str(tmp_path / "split.json")
    assert config.schemastore == str(tmp_path / "ss") and config.schemastore_commit == "c1" and config.jsonschemabench is None
    assert config.split_id == "split-4" and config.seed == 4 and config.known_schemas == ("a", "b")
    assert config.known_schema_files == {"real": str(tmp_path / "known.json")} and config.has_checkouts
    assert config.resource("split.train") == 0.6 and config.resource("lsh.bands") == 32
    assert config.extra == {"datasets": {"x": 1}}


@pytest.mark.parametrize(
    ("text", "match"),
    [
        ("[split]\nout = 'x'\n", "needs manifest"),
        ("[corpus]\nmanifest = 'm'\n", "needs out"),
        ("[corpus]\nmanifest = 'm'\nschemastore = 's'\n[split]\nout = 'x'\n", "go together"),
        ("[corpus]\nmanifest = 'm'\n[split]\nout = 'x'\nseed = -1\n", "seed must be"),
        ("[corpus]\nmanifest = 'm'\n[split]\nout = 'x'\n[resources]\n'split.train' = 1.5\n", "split.train"),
        ("[corpus]\nmanifest = 'm'\n[split]\nout = 'x'\n[resources]\n'ece.bins' = 3\n", "not a benchmark Resource_Parameter"),
        ("[corpus]\nmanifest = 'm'\n[split]\nout = 'x'\nbogus = 1\n", "unknown \\[split\\] setting"),
        ("[corpus]\nmanifest = 'm'\n[split]\nout = 'x'\n[nope]\n", "unknown section"),
        ("[corpus]\nmanifest = 'm'\n[split]\nout = 'x'\nknown_schemas = 'a'\n", "list of schema ids"),
    ],
)
def test_invalid_bench_configurations_are_config_errors(tmp_path: Path, text: str, match: str) -> None:
    with pytest.raises(ConfigError, match=match):
        BenchConfig.from_toml(write_toml(tmp_path / "bench.toml", text))


# -- the split builder ------------------------------------------------------------


def paths(*names: str) -> frozenset[str]:
    return frozenset({"$", *(f"$.{name}" for name in names)})


def test_near_duplicate_cutoff_falls_back_to_one_without_enough_pairs() -> None:
    cutoff, detail = near_duplicate_cutoff([0.9, 0.1], seed=0)
    assert cutoff == 1.0 and detail["components"] == 0 and detail["candidate_pairs"] == 2
    cutoff, detail = near_duplicate_cutoff([0.5] * 10, seed=0)
    assert cutoff == 1.0 and detail["criterion"] == "BIC" and detail["components"] == 1
    with pytest.raises(ValueError):
        near_duplicate_cutoff([1.5])


def test_a_bimodal_similarity_distribution_gives_a_cutoff_between_the_modes() -> None:
    similarities = [0.05, 0.08, 0.1, 0.12, 0.15, 0.1, 0.07, 0.09] * 3 + [0.9, 0.92, 0.95, 0.97, 0.93, 0.96] * 3
    cutoff, detail = near_duplicate_cutoff(similarities, seed=0)
    assert 0.2 < cutoff < 0.9 and detail["components"] >= 2
    assert near_duplicate_cutoff(similarities, seed=0) == (cutoff, detail)


def test_groups_follow_catalog_versions_and_similar_path_sets() -> None:
    inputs = SplitInputs(
        {
            "a": paths("x", "y", "z"),
            "a2": paths("x", "y", "z"),  # identical to a
            "b": paths("p", "q"),
            "b-v1": paths("p", "q", "r"),  # a catalog version of b
            "c": paths("m"),
            "d": paths("n", "o"),
            "e": paths("k"),
            "f": paths("k", "l"),
        },
        excluded=(("f", "partially supported"),),
        catalog_groups=(("b", "b-v1"),),
        known_ids=("c",),
        known_paths={"real": paths("n", "o")},
    )
    split = SplitBuilder(perm=32, bands=8).build(inputs, split_id="t", seed=1, proportions={"split.train": 0.5, "split.val": 0.25, "split.held_out": 0.25})
    groups = split.extra["groups"]
    assert groups["a"] == groups["a2"] and groups["b"] == groups["b-v1"] == "b"
    assert split.split_of("c") == "held_out" and split.split_of("d") == "held_out"  # known in corpus; similar to the known file
    assert split.extra["known_schemas"] == {"ids": ["c"], "similar": {"real": ["d"]}}
    assert split.extra["excluded"] == [{"id": "f", "reason": "partially supported"}]
    assert set(split.extra["proportions"]["deviation"]) == set(SPLITS)
    for name in SPLITS:
        members = split.ids(name)
        assert all(groups[other] != groups[sid] or other in members for sid in members for other in groups)
    assert SplitBuilder(perm=32, bands=8).build(inputs, split_id="t", seed=1, proportions={"split.train": 0.5, "split.val": 0.25, "split.held_out": 0.25}) == split


@pytest.mark.parametrize(
    ("proportions", "match"),
    [
        ({"split.train": 1.0, "split.val": 0.1, "split.held_out": 0.2}, "split.train"),
        ({"split.train": 0.5, "split.val": 0.1, "split.held_out": 0.2}, "sum to"),
        ({"split.train": 0.98, "split.val": 0.01, "split.held_out": 0.01}, "without a schema"),
    ],
)
def test_invalid_proportions_or_empty_splits_stop_without_a_split(proportions: dict[str, float], match: str) -> None:
    inputs = SplitInputs({f"s{i}": paths(f"k{i}") for i in range(6)})
    with pytest.raises(SplitError, match=match):
        SplitBuilder(perm=16, bands=4).build(inputs, split_id="t", seed=0, proportions=proportions)
    with pytest.raises(SplitError, match="seed"):
        SplitBuilder(perm=16, bands=4).build(inputs, split_id="t", seed=-1)
    with pytest.raises(SplitError, match="known schemas not in the corpus"):
        SplitInputs.from_manifest(CorpusManifest.load(FIXTURE_MANIFEST), known_ids=["nope"])


# -- jsinfer split ----------------------------------------------------------------


def test_the_split_command_writes_the_corpus_manifest_and_the_split(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    store = tmp_path / "store"
    for name in ("a", "a-1", "b", "c", "d", "e"):
        (store / SCHEMASTORE_SCHEMAS).mkdir(parents=True, exist_ok=True)
        (store / SCHEMASTORE_SCHEMAS / f"{name}.json").write_text(json.dumps({"type": "object", "properties": {name: {}, "shared": {}}}), encoding="utf-8")
    (store / SCHEMASTORE_SCHEMAS / "bad.json").write_text("{", encoding="utf-8")
    catalog = store / SCHEMASTORE_CATALOG
    catalog.parent.mkdir(parents=True)
    catalog.write_text(json.dumps({"schemas": [{"name": "a", "url": "https://x/a.json", "versions": {"1": "https://x/a-1.json"}}]}), encoding="utf-8")
    config = write_toml(
        tmp_path / "bench.toml",
        f'[corpus]\nmanifest = "corpus/manifest.json"\nschemastore = "store"\nschemastore_commit = "c0ffee"\n'
        f'[split]\nout = "split.json"\nseed = 2\nknown_schemas = ["schemastore/c"]\n'
        f'[resources]\n"split.train" = 0.5\n"split.val" = 0.25\n"split.held_out" = 0.25\n"minhash.perm" = 16\n"lsh.bands" = 4\n',
    )
    assert cli.main(["split", "-c", str(config)]) == cli.EXIT_OK
    err = capsys.readouterr().err
    assert "wrote the corpus manifest (6 schemas, 1 omitted files)" in err and "wrote Schema_Split split-2" in err
    manifest = CorpusManifest.load(tmp_path / "corpus" / "manifest.json")
    assert len(manifest.ids()) == 6 and manifest.extra["snapshots"] == {"schemastore": "c0ffee"}
    split = SchemaSplit.load(tmp_path / "split.json")
    assert split.split_of("schemastore/c") == "held_out" and split.extra["snapshots"] == {"schemastore": "c0ffee"}
    assert split.extra["groups"]["schemastore/a"] == split.extra["groups"]["schemastore/a-1"]
    assert {"candidate_pairs", "criterion", "value"} <= set(split.extra["cutoff"])
    # Invalid proportions: exit 1 and no split written (30.10).
    bad = write_toml(tmp_path / "bad.toml", config.read_text(encoding="utf-8").replace('"split.val" = 0.25', '"split.val" = 0.5').replace('out = "split.json"', 'out = "nope.json"'))
    assert cli.main(["split", "-c", str(bad)]) == cli.EXIT_STAGE_ERROR
    assert "invalid split proportions" in capsys.readouterr().err and not (tmp_path / "nope.json").exists()
    # A configuration error: exit 2.
    assert cli.main(["split", "-c", str(tmp_path / "missing.toml")]) == cli.EXIT_USAGE
    # Without checkouts, an existing manifest (the fixture corpus) is read.
    fixture = write_toml(tmp_path / "fixture.toml", f'[corpus]\nmanifest = "{FIXTURE_MANIFEST}"\n[split]\nout = "fixture-split.json"\n[resources]\n"minhash.perm" = 16\n"lsh.bands" = 4\n')
    assert cli.main(["split", "-c", str(fixture)]) == cli.EXIT_OK
    assert len(SchemaSplit.load(tmp_path / "fixture-split.json").train) >= 10
