"""``ConfigError``: configuration and usage errors raised before any document is ingested.

This module imports nothing from ``jsinfer``, so every layer can raise or
catch the error without import cycles: the Run_Configuration
(``config.run_config``; 27.11, 28.8, 28.9, 29.11), the Model_Bundle reader
(``learn.bundle``; an unloadable bundle, 27.11), the Ingestor (rejected
supplied paths, 1.10), and the public API, which re-exports it as
``jsinfer.ConfigError``. The CLI maps it to exit status 2.

A :class:`ConfigError` carries every finding at once as :class:`ConfigProblem`
values, so one error reports all offending settings. Each problem has a
``subject`` (the offending setting, Technique id, Model_Bundle identifier, or
path), a ``reason`` from :data:`CONFIG_PROBLEM_REASONS`, and a ``message``
that names the subject and says what is wrong.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from typing import Final, Literal, TypeAlias

__all__ = ["CONFIG_PROBLEM_REASONS", "ConfigError", "ConfigProblem", "ConfigProblemReason"]

ConfigProblemReason: TypeAlias = Literal[
    "bundle",
    "duplicate",
    "parse",
    "path",
    "prerequisite",
    "range",
    "type",
    "unimplemented",
    "unknown",
    "unregistered",
]

# Why a configuration is rejected:
#   bundle         the Model_Bundle cannot be found or loaded (27.11)
#   duplicate      one setting is given twice in one source (a quoted and a dotted TOML key)
#   parse          the configuration file cannot be read or parsed (27.11)
#   path           a supplied input path is missing, unreadable, or has the wrong extension (1.10)
#   prerequisite   an enabled Technique has a disabled prerequisite (28.8)
#   range          a value lies outside the setting's allowed range (27.11, 29.11)
#   type           a value has the wrong type (27.11)
#   unimplemented  an enabled Technique is registered but not implemented yet
#   unknown        an entry or flag names an unknown setting (27.11)
#   unregistered   a Technique id that is not registered (28.9)
CONFIG_PROBLEM_REASONS: Final[tuple[ConfigProblemReason, ...]] = (
    "bundle",
    "duplicate",
    "parse",
    "path",
    "prerequisite",
    "range",
    "type",
    "unimplemented",
    "unknown",
    "unregistered",
)


@dataclass(frozen=True, slots=True)
class ConfigProblem:
    """One finding of a :class:`ConfigError`; ``message`` names ``subject`` and the cause."""

    subject: str
    reason: ConfigProblemReason
    message: str

    def __post_init__(self) -> None:
        if not isinstance(self.subject, str):
            raise TypeError(f"ConfigProblem subject must be a str, got {type(self.subject).__name__}")
        if self.reason not in CONFIG_PROBLEM_REASONS:
            raise ValueError(f"ConfigProblem reason must be one of {list(CONFIG_PROBLEM_REASONS)}, got {self.reason!r}")
        if not isinstance(self.message, str) or not self.message:
            raise ValueError("ConfigProblem message must be a non-empty str")

    def __str__(self) -> str:
        return self.message


class ConfigError(Exception):
    """An invalid configuration or usage, detected before ingestion (1.10, 27.11, 28.8, 28.9, 29.11).

    ``problems`` holds every finding in a deterministic order (at least one).
    ``str(error)`` is the message of a single problem, or a count followed by
    one message per line.
    """

    def __init__(self, problems: Iterable[ConfigProblem]) -> None:
        found = tuple(problems)
        if not found:
            raise ValueError("a ConfigError needs at least one ConfigProblem")
        for problem in found:
            if not isinstance(problem, ConfigProblem):
                raise TypeError(f"ConfigError problems must be ConfigProblem values, got {type(problem).__name__}")
        super().__init__(found)  # args survive pickling
        self.problems: tuple[ConfigProblem, ...] = found

    @property
    def subjects(self) -> tuple[str, ...]:
        """The subject of each problem, in problem order."""
        return tuple(problem.subject for problem in self.problems)

    def __str__(self) -> str:
        if len(self.problems) == 1:
            return self.problems[0].message
        lines = [f"{len(self.problems)} configuration problems:"]
        lines.extend(f"  - {problem.message}" for problem in self.problems)
        return "\n".join(lines)
