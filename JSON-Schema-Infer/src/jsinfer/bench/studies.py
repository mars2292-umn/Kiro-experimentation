"""The ablation study (task 30.1; Requirements 35.3, 35.10). Robustness, versioning, and calibration follow in 41.2, 41.4, and 48.4.

Configurations (35.3, 35.10)
    :func:`ablation_configurations` builds one :class:`AblatedConfig` per
    Technique enabled in the default Run_Configuration: that Technique is
    disabled and, transitively, every enabled Technique that would lose a
    prerequisite, and the complete disabled set is recorded. The function
    takes the default states and the prerequisite map, so the Registry is
    one source of them and property tests another.

Runs (35.3)
    :func:`run_ablation` runs the Inferrer under the default and every
    ablated Run_Configuration on the same synthetic Document_Collections
    with the same seed: each ablated configuration is the Inferrer adapter
    with ``techniques.<id>=false`` for its disabled set, named
    ``ablate:<id>``, and all of them run through ``bench.synthetic``
    together, so they share every training collection, held-out document,
    and negative. :meth:`AblationResults.differences` reports, per metric
    and training size, the paired bootstrap difference (ablated minus
    default) with its interval and p-value over the Held_Out_Schemas where
    both have a value (35.2, 35.3).
"""

from __future__ import annotations

import dataclasses
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

from jsinfer.bench.baselines.inferrer import InferrerAdapter
from jsinfer.bench.bootstrap import PairedTest, paired_test
from jsinfer.bench.splits import CorpusManifest, SchemaSplit
from jsinfer.bench.synthetic import SyntheticConfig, SyntheticResults, _metric_values, run_synthetic
from jsinfer.config.registry import REGISTRY, Registry
from jsinfer.config.run_config import TECHNIQUE_PREFIX
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON
from jsinfer.io.subschema import SubschemaChecker
from jsinfer.io.validator import Validator
from jsinfer.learn.synth import SyntheticGenerator

__all__ = ["ABLATION_PREFIX", "DEFAULT_SYSTEM", "AblatedConfig", "AblationResults", "ablation_configurations", "run_ablation"]

ABLATION_PREFIX = "ablate:"
DEFAULT_SYSTEM = "jsinfer"


@dataclass(frozen=True)
class AblatedConfig:
    """One ablated Run_Configuration: the Technique turned off and the complete disabled set (35.10)."""

    technique: str
    disabled: tuple[str, ...]
    states: Mapping[str, bool]

    @property
    def system(self) -> str:
        return ABLATION_PREFIX + self.technique

    def settings(self) -> dict[str, JSON]:
        """The ``techniques.<id>=false`` settings of the configuration."""
        return {TECHNIQUE_PREFIX + tid: False for tid in self.disabled}

    def to_json(self) -> dict[str, JSON]:
        return {"technique": self.technique, "disabled": list(self.disabled), "system": self.system}


def ablation_configurations(
    states: Mapping[str, bool], prerequisites: Mapping[str, Sequence[str]] | None = None
) -> list[AblatedConfig]:
    """One configuration per enabled Technique, with transitive dependents disabled (35.3, 35.10).

    ``states`` is the default enabled state per Technique id (registration
    order is kept), ``prerequisites`` the prerequisite ids per Technique
    (from the Registry when None).
    """
    prerequisites = prerequisites if prerequisites is not None else {tid: spec.prerequisites for tid, spec in REGISTRY.techniques.items()}
    configs: list[AblatedConfig] = []
    for tid, enabled in states.items():
        if not enabled:
            continue
        disabled = {tid}
        changed = True
        while changed:
            changed = False
            for other, on in states.items():
                if on and other not in disabled and any(p in disabled for p in prerequisites.get(other, ())):
                    disabled.add(other)
                    changed = True
        ordered = tuple(sorted(disabled, key=codepoint_key))
        configs.append(AblatedConfig(tid, ordered, {t: (on and t not in disabled) for t, on in states.items()}))
    return configs


def registry_configurations(registry: Registry = REGISTRY) -> list[AblatedConfig]:
    """:func:`ablation_configurations` for the Registry's default states."""
    return ablation_configurations(registry.default_states(), {tid: spec.prerequisites for tid, spec in registry.techniques.items()})


def _values_by_schema(results: SyntheticResults, system: str, size: int, metric: str) -> dict[str, float]:
    """The defined values of ``metric`` per Held_Out_Schema for one system and size (missing runs left out)."""
    out: dict[str, float] = {}
    for row in results.select(system, size):
        if row.missing is not None:
            continue
        [value] = _metric_values([row], metric)
        if value is not None:
            out[row.schema_id] = value
    return out


@dataclass
class AblationResults:
    """The synthetic results of the default and every ablated configuration on the same data (35.3)."""

    configs: list[AblatedConfig]
    results: SyntheticResults
    resamples: int | None = None

    def differences(self, metric: str, size: int) -> dict[str, PairedTest | None]:
        """Ablated minus default per ablation, paired over the schemas where both have a value (35.2, 35.3)."""
        default = _values_by_schema(self.results, DEFAULT_SYSTEM, size, metric)
        out: dict[str, PairedTest | None] = {}
        for config in self.configs:
            ablated = _values_by_schema(self.results, config.system, size, metric)
            shared = sorted(set(default) & set(ablated), key=codepoint_key)
            out[config.technique] = paired_test(
                [ablated[s] for s in shared], [default[s] for s in shared],
                purpose=f"ablation:{config.technique}:{metric}:{size}", seed=self.results.config.seed, resamples=self.resamples,
            )
        return out

    def to_json(self) -> dict[str, JSON]:
        sizes = list(self.results.config.sizes)
        return {
            "configs": [c.to_json() for c in self.configs],
            "results": self.results.to_json(),
            "differences": {
                f"{metric}@{size}": {tid: (None if test is None else test.to_json()) for tid, test in self.differences(metric, size).items()}
                for metric in ("recall", "rejection", "harmonic")
                for size in sizes
            },
        }


def run_ablation(
    manifest: CorpusManifest,
    split: SchemaSplit,
    inferrer: InferrerAdapter,
    cfg: SyntheticConfig,
    out_root: Path,
    *,
    configs: Sequence[AblatedConfig] | None = None,
    schema_ids: Sequence[str] | None = None,
    resamples: int | None = None,
    generator: SyntheticGenerator | None = None,
    validator: Validator | None = None,
    checker: SubschemaChecker | None = None,
    env: dict[str, str] | None = None,
) -> AblationResults:
    """Run the default and every ablated Inferrer configuration on the same collections and seed (35.3)."""
    configs = list(configs) if configs is not None else registry_configurations()
    default = dataclasses.replace(inferrer, name=DEFAULT_SYSTEM)
    systems = [default] + [
        dataclasses.replace(inferrer, name=c.system, label=f"Inferrer without {c.technique}", settings={**inferrer.settings, **c.settings()})
        for c in configs
    ]
    results = run_synthetic(manifest, split, systems, cfg, out_root, schema_ids=schema_ids, generator=generator, validator=validator, checker=checker, env=env)
    return AblationResults(configs, results, resamples)


# Robustness (35.4), versioning (35.5, 35.11), and calibration (35.6) are tasks 41.2, 41.4, and 48.4.
