"""The Baseline_Adapter protocol and the shared run records (task 28.1; Requirements 34.1, 34.5, 34.6, 34.11).

A :class:`BaselineAdapter` describes one benchmarked system: the Inferrer,
Naive_Merge, JSONoid, or ReCG. The harness never calls a system in
process. An adapter gives the command line that produces a schema file from
a JSONL Document_Collection, and ``bench.runner`` runs it in an isolated
child process, times it, samples its memory, enforces the time limit, and
reads the output back (34.2, 34.3, 34.7, 34.8, 35.7).

- :meth:`BaselineAdapter.available` says whether the system can be started
  at all (its executable, runtime, or jar is present), with the reason when
  not (34.6).
- :meth:`BaselineAdapter.command` is the argv that reads ``docs_jsonl`` and
  writes the schema into ``out_dir``; :meth:`BaselineAdapter.output_path`
  names the file it writes. ``seed`` is the run seed for systems that take
  one.
- :attr:`BaselineAdapter.output_draft` is the draft the system's output is
  in when its ``$schema`` is absent (34.5); None means draft 2020-12.
- :meth:`BaselineAdapter.describe` is the :class:`BaselineInfo` recorded
  with every result: version or commit, options, and runtime (34.11).

:class:`Availability` and :class:`BaselineInfo` are plain records; the run
outcome records live in ``bench.runner``.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Protocol, runtime_checkable

from jsinfer.core.jsonvalue import JSON
from jsinfer.io.validator import Draft

__all__ = ["Availability", "BaselineAdapter", "BaselineInfo"]


@dataclass(frozen=True)
class BaselineInfo:
    """What 34.11 records with every result: the system's identity, version, options, and runtime."""

    name: str
    label: str
    version: str
    runtime: str
    options: Mapping[str, JSON] = field(default_factory=dict)
    arguments: tuple[str, ...] = ()

    def to_json(self) -> dict[str, JSON]:
        return {
            "name": self.name,
            "label": self.label,
            "version": self.version,
            "runtime": self.runtime,
            "options": dict(self.options),
            "arguments": list(self.arguments),
        }


@dataclass(frozen=True)
class Availability:
    """Whether a system can be started, and why not (34.6)."""

    available: bool
    reason: str | None = None

    def to_json(self) -> dict[str, JSON]:
        return {"available": self.available, "reason": self.reason}


@runtime_checkable
class BaselineAdapter(Protocol):
    """One benchmarked system (see the module docstring)."""

    @property
    def name(self) -> str: ...

    @property
    def label(self) -> str: ...

    @property
    def output_draft(self) -> Draft | None: ...

    def available(self) -> Availability: ...

    def command(self, docs_jsonl: Path, out_dir: Path, seed: int) -> Sequence[str]: ...

    def output_path(self, out_dir: Path) -> Path: ...

    def describe(self) -> BaselineInfo: ...
