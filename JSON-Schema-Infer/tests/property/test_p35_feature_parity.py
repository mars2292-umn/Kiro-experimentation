"""Property 35: Feature parity (task 22.3).

For any planted Document_Collection, the Trainer and the Inferrer compute
identical features for the same Judgment_Call (22.10). The Trainer's oracle
featurization (``learn.trainer.featurize_collection``) records the feature
vector at each Judgment_Call it reaches; an Inferrer run under the same
configuration, with a recording DecisionEngine that keeps every ``features``
callable's output, is compared with it. Both run the real pipeline and the
featurizers of ``learn.features``; they differ only in how upstream
Judgment_Calls are resolved (the oracle forces known labels), so a
Judgment_Call is compared when its context is the same in both runs: every
decision at a proper prefix of its Path, and every other decision at its
Path, selected the same outcome. The root Record-or-Map call, which has no
upstream decision, is always compared. The oracle run is also
byte-identical when repeated.

Inputs
    Planted collections from ``tests.strategies.collections`` without
    outliers (outliers are not labeled), and a drawn seed.

**Validates: Requirements 22.10**
"""

# Feature: json-schema-inference, Property 35: Feature parity

from __future__ import annotations

from collections.abc import Callable, Iterable
from typing import Any

import numpy as np
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer.config.resources import ResourceReader
from jsinfer.config.run_config import RunConfiguration
from jsinfer.core.paths import ROOT, Path, render_path
from jsinfer.core.report import InferenceReport
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.core.views import View
from jsinfer.decide.engine import Candidate, Decision, DecisionEngine, DecisionPoint
from jsinfer.io.validator import Validator
from jsinfer.learn.bundle import ResolvedBundle
from jsinfer.learn.features import closed_enum_features
from jsinfer.learn.synth import GeneratorConfig, SchemaExcluded, SyntheticGenerator
from jsinfer.learn.trainer import JUDGMENT_KINDS, featurization_config, featurize_collection
from jsinfer.mdl.scorer import MDLScorer
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer
from jsinfer.values.inferrer import ValueInferrer
from tests.strategies import planted_schemas

MAX_DEPTH = 3


class RecordingEngine(DecisionEngine):
    """The Inferrer's engine, keeping the feature vector of every Judgment_Call it resolves."""

    __slots__ = ("recorded",)

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.recorded: dict[tuple[Path, str], list[np.ndarray]] = {}

    def resolve(
        self,
        dp: DecisionPoint,
        candidates: Iterable[Candidate],
        view: View | None = None,
        features: Callable[[], np.ndarray] | None = None,
        **kwargs: Any,
    ) -> Decision:
        offered = list(candidates)
        if dp.kind in JUDGMENT_KINDS and features is not None and offered:
            path = dp.path if dp.path is not None else (view.path if view is not None else ROOT)
            self.recorded.setdefault((path, dp.kind), []).append(np.asarray(features(), dtype=np.float64).reshape(-1))
        return super().resolve(dp, offered, view, features, **kwargs)


def inferrer_run(documents: list[Any], config: RunConfiguration) -> tuple[PipelineResult, dict[tuple[Path, str], list[np.ndarray]]]:
    engines: list[RecordingEngine] = []

    def factory(cfg: RunConfiguration, bundle: ResolvedBundle, report: InferenceReport, validator: Validator) -> StructureAnalyzer:
        scorer = MDLScorer(validator=validator)
        engine = RecordingEngine(cfg.techniques, bundle=bundle, report=report, scorer=scorer)
        engines.append(engine)
        values = ValueInferrer(engine, validator=validator, report=report, scorer=scorer, enum_features=closed_enum_features)
        return StructureAnalyzer(values, scorer=scorer, resources=ResourceReader.for_run(cfg, report), seed=cfg.seed)

    result = infer_documents(documents, config, analyzer_factory=factory)
    return result, engines[0].recorded


def outcomes(result: PipelineResult) -> dict[tuple[str, str], str]:
    """``(Path, kind) -> selected outcome`` of every report decision (the context of later Judgment_Calls)."""
    found: dict[tuple[str, str], str] = {}
    for entry in result.report.decisions:
        selected = entry.resolution.candidates[entry.resolution.selected]
        found[entry.path, entry.kind] = selected.outcome
    return found


def same_context(path: Path, kind: str, a: dict[tuple[str, str], str], b: dict[tuple[str, str], str]) -> bool:
    rendered = render_path(path)
    prefixes = {render_path(path[:n]) for n in range(len(path))}
    for (where, what), outcome in a.items():
        if where in prefixes or (where == rendered and what != kind):
            if b.get((where, what)) != outcome:
                return False
    return True


@given(schema=planted_schemas(max_depth=2), seed=st.integers(0, 2**31 - 1), size=st.integers(4, 14))
def test_the_trainer_and_the_inferrer_compute_identical_features(schema: dict[str, Any], seed: int, size: int) -> None:
    model = SchemaModel(SchemaNode.from_json(schema))
    cfg = GeneratorConfig(size_min=size, size_max=size, max_depth=MAX_DEPTH, doc_attempts=12)
    try:
        collection = SyntheticGenerator().collection(model, "planted", 0, cfg, seed)
    except SchemaExcluded:
        event("schema excluded by the generator")
        return
    config = featurization_config({"seed": seed})
    trainer = featurize_collection(collection, model, config=config, max_depth=MAX_DEPTH)
    again = featurize_collection(collection, model, config=config, max_depth=MAX_DEPTH)
    assert [e.to_json() for e in again.examples.examples] == [e.to_json() for e in trainer.examples.examples]
    assert again.examples.exclusions == trainer.examples.exclusions

    result, recorded = inferrer_run(list(collection.documents), config)
    # The oracle's context is what it forced: the report entry holds the engine's choice before the override.
    oracle, plain = outcomes(trainer.result), outcomes(result)
    for example in trainer.examples.examples:
        oracle[render_path(example.path), example.kind] = example.label
    # Several calls at one (Path, kind) (Tuple positions share the element Path) are compared in order.
    expected: dict[tuple[Path, str], list[np.ndarray]] = {}
    for example in trainer.examples.examples:
        expected.setdefault((example.path, example.kind), []).append(example.features)
    compared = 0
    for (path, kind), vectors in expected.items():
        if not same_context(path, kind, oracle, plain):
            continue
        found = recorded.get((path, kind), [])
        assert len(found) == len(vectors), f"{kind} at {render_path(path)}: {len(found)} calls, the Trainer saw {len(vectors)}"
        for mine, theirs in zip(found, vectors):
            assert np.array_equal(mine, theirs), f"{kind} at {render_path(path)}"
        compared += len(vectors)
    if (ROOT, "record_or_map") in expected:
        assert np.array_equal(recorded[ROOT, "record_or_map"][0], expected[ROOT, "record_or_map"][0])
    total = len(trainer.examples.examples)
    event(f"compared: {'none' if not compared else 'some' if compared < total else 'all'}")
