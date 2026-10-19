"""The Inferrer as a Baseline_Adapter (task 28.1; Requirements 34.1, 34.3, 34.11).

The harness runs the Inferrer the way a user does, as the ``jsinfer infer``
command in its own process, so its time and memory are measured like every
other system's (34.2, 34.3, 35.7): ``jsinfer infer DOCS.jsonl --out OUT_DIR
--set seed=SEED [-c FILE] [--set KEY=VALUE]...`` writes ``schema.json``
(draft 2020-12) into ``OUT_DIR``. The console script next to the
interpreter is used when installed, else ``python -m jsinfer.cli``.
"""

from __future__ import annotations

import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path

import jsinfer
from jsinfer.bench.baselines.base import Availability, BaselineInfo
from jsinfer.core.jsonvalue import JSON
from jsinfer.io.validator import Draft

__all__ = ["InferrerAdapter", "jsinfer_command"]


def jsinfer_command(python: str = sys.executable) -> list[str]:
    """The installed ``jsinfer`` console script next to ``python``, else ``python -m jsinfer.cli``."""
    script = Path(python).with_name("jsinfer")
    return [str(script)] if script.exists() else [python, "-m", "jsinfer.cli"]


@dataclass(frozen=True)
class InferrerAdapter:
    """``jsinfer infer`` as a Baseline_Adapter (see the module docstring).

    ``config`` is an optional ``-c FILE``; ``settings`` are ``--set``
    overrides (flat names, as ``jsinfer infer --set`` takes them); the run
    seed is always passed as ``--set seed=SEED``.
    """

    config: Path | None = None
    settings: Mapping[str, JSON] = field(default_factory=dict)
    python: str = sys.executable
    name: str = "jsinfer"
    label: str = "Inferrer"

    @property
    def output_draft(self) -> Draft | None:
        return None

    def available(self) -> Availability:
        if self.config is not None and not self.config.is_file():
            return Availability(False, f"no configuration file {self.config}")
        return Availability(True) if Path(self.python).exists() else Availability(False, f"no interpreter {self.python}")

    def _flags(self, seed: int) -> list[str]:
        flags = ["--set", f"seed={seed}"]
        if self.config is not None:
            flags += ["-c", str(self.config)]
        for key in sorted(self.settings):
            flags += ["--set", f"{key}={_toml(self.settings[key])}"]
        return flags

    def command(self, docs_jsonl: Path, out_dir: Path, seed: int) -> Sequence[str]:
        return [*jsinfer_command(self.python), "infer", str(docs_jsonl), "--out", str(out_dir), *self._flags(seed)]

    def output_path(self, out_dir: Path) -> Path:
        return out_dir / "schema.json"

    def describe(self) -> BaselineInfo:
        options: dict[str, JSON] = dict(self.settings)
        if self.config is not None:
            options["config"] = str(self.config)
        return BaselineInfo(
            self.name, self.label, jsinfer.__version__, f"python {sys.version.split()[0]}", options, tuple(self._flags(0)[2:])
        )


def _toml(value: JSON) -> str:
    """A ``--set`` value: strings verbatim, everything else as a TOML literal."""
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, list):
        return "[" + ", ".join(_toml(v) if not isinstance(v, str) else f'"{v}"' for v in value) + "]"
    return str(value)
