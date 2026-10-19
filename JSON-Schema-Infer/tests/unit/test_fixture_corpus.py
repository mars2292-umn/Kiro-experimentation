"""The hand-written fixture corpus (task 23.4): valid, compilable, consistent, and covering every label."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

import pytest

from jsinfer.bench.splits import CorpusManifest, SchemaSplit
from jsinfer.corpus.normalize import normalize_name
from jsinfer.decide.engine import JUDGMENT_OUTCOMES
from jsinfer.io.load import SchemaLoader
from jsinfer.io.validator import Draft, Validator
from jsinfer.learn.labels import KINDS, UNKNOWN, derive_labels
from jsinfer.learn.synth import ValueIndex
from jsinfer.values.formats import FORMATS
from tests.conftest import FIXTURE_MANIFEST, FIXTURE_SPLIT

VALIDATOR = Validator()
LOADER = SchemaLoader(VALIDATOR)


@pytest.fixture(scope="module")
def corpus() -> tuple[CorpusManifest, SchemaSplit, dict]:
    manifest = CorpusManifest.load(FIXTURE_MANIFEST)
    split = SchemaSplit.load(FIXTURE_SPLIT)
    models = {sid: LOADER.load(manifest.schema_path(sid).read_bytes()) for sid in manifest.ids()}
    return manifest, split, models


def test_thirty_fixture_schemas_pass_the_meta_schema_and_compile(corpus: tuple) -> None:
    manifest, _, models = corpus
    assert len(manifest.ids()) == 30 and all(entry.source == "fixture" for entry in manifest.entries.values())
    for sid in manifest.ids():
        raw = json.loads(manifest.schema_path(sid).read_text(encoding="utf-8"))
        assert VALIDATOR.meta_errors(raw, Draft.DRAFT202012) == [], sid
        VALIDATOR.compile(raw, name=sid)
        assert models[sid].fully_supported, sid


def test_the_manifest_and_the_split_agree(corpus: tuple) -> None:
    manifest, split, _ = corpus
    assigned = (*split.train, *split.val, *split.held_out)
    assert sorted(assigned) == sorted(manifest.ids())
    assert len(split.train) == 14 and len(split.val) == 12 and len(split.held_out) == 4
    assert split.id == "fixture-split" and split.seed == 0


def test_both_classes_of_every_kind_occur_in_the_training_and_validation_splits(corpus: tuple) -> None:
    _, split, models = corpus
    for name in ("train", "val"):
        found: Counter[tuple[str, str]] = Counter()
        for sid in split.ids(name):  # type: ignore[arg-type]
            for (_, kind), label in derive_labels(models[sid]).items():
                if label != UNKNOWN:
                    found[kind, label] += 1
        for kind in KINDS:
            for label in JUDGMENT_OUTCOMES[kind]:
                assert found[kind, label] > 0, f"{name}: no {kind} = {label} label"


def test_the_corpus_covers_formats_values_names_refs_applicators_and_patterns(corpus: tuple) -> None:
    manifest, _, models = corpus
    texts = {sid: manifest.schema_path(sid).read_text(encoding="utf-8") for sid in manifest.ids()}
    everything = "\n".join(texts.values())
    for fmt in FORMATS:
        assert f'"format": "{fmt}"' in everything, fmt
    for keyword in ('"examples"', '"default"', '"enum"', '"$ref"', '"allOf"', '"oneOf"', '"anyOf"', '"not"', '"if"', '"pattern"'):
        assert keyword in everything, keyword
    # Recursion: a $defs entry that refers to itself.
    assert any('"$ref": "#/$defs/node"' in t and '"node"' in t for t in texts.values())
    # Field names shared across schemas after normalization (source (b) of 22.3), in three spellings.
    index = ValueIndex.build(models.items())
    assert index.lookup(normalize_name("emailAddress")) == index.lookup(("email", "address")) != ()
    spellings = {name for t in texts.values() for name in ("emailAddress", "email_address") if f'"{name}"' in t}
    assert spellings == {"emailAddress", "email_address"}
