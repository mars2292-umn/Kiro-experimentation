"""The JSONoid adapter (task 28.2; Requirements 34.1, 34.5, 34.6, 34.11).

JSONoid (Mior, "JSONoid: Monoid-based Enrichment for Configurable and
Scalable Data-driven Schema Discovery") is a Java 17 CLI distributed as a
jar. The harness runs it as

    java -jar jsonoid.jar --random-seed SEED DOCS.jsonl -w OUT_DIR/jsonoid.json

with its default property set and equivalence relation, both recorded as
options (design: Baselines). The jar is pinned by release and SHA-256
(:class:`JsonoidAdapter` with ``jar``, ``sha256``, and ``version``), and
:meth:`JsonoidAdapter.available` reports a missing ``java``, a missing jar,
or a digest mismatch as unavailability rather than a failure (34.6). The
Java version is recorded as the runtime (34.11).

Release 0.40 declares draft 2020-12 in its output; an older release's
output declares 2019-09 (or nothing, when :attr:`JsonoidAdapter.output_draft`
says 2019-09), and the runner then translates it to draft 2020-12 and
records the translation (34.5). JSONoid also writes its own statistics as
keywords (``bloomFilter``, ``hll``, ``lengthHistogram``, ``fieldPresence``,
...), which the Schema_Loader records as ignored; they count in the schema
size of 35.7, which the report notes.

For an array it only saw empty, release 0.40 writes ``"items": []`` beside
``"maxItems": 0``, which is invalid in the draft 2020-12 it declares, so the
whole output would fail the meta-schema (34.8): on the real-data benchmark
that removed 41 of 187 datasets, every GitHub webhook dataset among them.
:meth:`JsonoidAdapter.repair_output` removes those keywords before the check
(34.12); the runner records where.

Configuration (``bench.toml``)::

    [baselines.jsonoid]
    jar = "checkouts/jsonoid-discovery.jar"
    sha256 = "..."
    version = "v0.x.y"
    java = "java"            # optional
    extra_args = []          # optional, recorded as options

    [baselines.jsonoid.variants.simple-open]   # optional: more JSONoid systems, same jar
    label = "JSONoid (Simple, open objects)"
    extra_args = ["--prop", "Simple", "--additional-properties"]

Each variant is one more system, named ``jsonoid-<variant>`` and run with
its own ``extra_args`` instead of the main table's (:func:`jsonoid_adapters`).
JSONoid's default property set, ``All``, records statistics (histograms,
Bloom filters, examples) and its output says it is not meant for
validation; every property set closes every object unless
``--additional-properties`` is given. :meth:`JsonoidAdapter.describe`
records the property set and the object closing each run used.
"""

from __future__ import annotations

import dataclasses
import hashlib
import shutil
import subprocess
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from functools import cache
from pathlib import Path
from typing import Any

from jsinfer.bench.baselines.base import Availability, BaselineInfo
from jsinfer.io.validator import Draft

__all__ = ["OUTPUT_FILE", "JsonoidAdapter", "java_version", "jsonoid_adapters", "jsonoid_from_config"]

OUTPUT_FILE = "jsonoid.json"


@cache
def java_version(java: str = "java") -> str | None:
    """``java -version``'s first line, or None when ``java`` cannot be run or exits with an error.

    The exit status counts: macOS ships ``/usr/bin/java`` as a stub that
    prints "Unable to locate a Java Runtime" and exits 1, which is no Java
    runtime (34.6), not a version string.
    """
    executable = shutil.which(java)
    if executable is None:
        return None
    try:
        done = subprocess.run([executable, "-version"], capture_output=True, text=True, timeout=60, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if done.returncode != 0:
        return None
    text = (done.stderr or done.stdout).strip().splitlines()
    return text[0] if text else None


def _digest(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


@dataclass(frozen=True)
class JsonoidAdapter:
    """JSONoid as a Baseline_Adapter (see the module docstring)."""

    jar: Path
    sha256: str
    version: str
    java: str = "java"
    extra_args: tuple[str, ...] = ()
    options: Mapping[str, Any] = field(default_factory=dict)
    name: str = "jsonoid"
    label: str = "JSONoid"

    @property
    def output_draft(self) -> Draft | None:
        return Draft.DRAFT201909

    def available(self) -> Availability:
        if java_version(self.java) is None:
            return Availability(False, f"no Java runtime ({self.java})")
        if not self.jar.is_file():
            return Availability(False, f"no jar {self.jar}")
        actual = _digest(self.jar)
        if actual.lower() != self.sha256.lower():
            return Availability(False, f"jar digest {actual} is not the pinned {self.sha256}")
        return Availability(True)

    def command(self, docs_jsonl: Path, out_dir: Path, seed: int) -> Sequence[str]:
        return [
            shutil.which(self.java) or self.java,
            "-jar",
            str(self.jar.resolve()),
            "--random-seed",
            str(seed),
            *self.extra_args,
            str(docs_jsonl),
            "-w",
            str(out_dir / OUTPUT_FILE),
        ]

    def output_path(self, out_dir: Path) -> Path:
        return out_dir / OUTPUT_FILE

    def repair_output(self, document: dict[str, Any]) -> tuple[dict[str, Any], tuple[str, ...]]:
        """JSONoid's output with every ``"items": []`` removed, and the JSON Pointers of the removals (34.12).

        For an array it saw only empty, JSONoid writes ``"items": []`` beside
        ``"maxItems": 0``. An empty ``items`` array is the pre-2020-12
        spelling of a zero-length tuple, which constrains no array in any
        draft, yet draft 2020-12, the draft the output declares, requires
        ``items`` to be a schema, so the document fails its meta-schema.
        Removing the keyword changes no verdict; ``maxItems: 0`` keeps the
        meaning. Every other keyword is left as it is.
        """
        removed: list[str] = []

        def walk(value: Any, pointer: str) -> Any:
            if isinstance(value, dict):
                out = {}
                for key, child in value.items():
                    where = f"{pointer}/{key.replace('~', '~0').replace('/', '~1')}"
                    if key == "items" and child == []:
                        removed.append(where)
                        continue
                    out[key] = walk(child, where)
                return out
            if isinstance(value, list):
                return [walk(child, f"{pointer}/{i}") for i, child in enumerate(value)]
            return value

        repaired = walk(document, "")
        return repaired, tuple(removed)

    def describe(self) -> BaselineInfo:
        args = list(self.extra_args)
        prop = next((args[i + 1] for i, a in enumerate(args[:-1]) if a in ("-p", "--prop")), "default")
        relation = next((args[i + 1] for i, a in enumerate(args[:-1]) if a in ("-e", "--equivalence-relation")), "default")
        options = {
            "property_set": prop,
            "equivalence_relation": relation,
            "additional_properties": "open" if {"-a", "--additional-properties"} & set(args) else "closed",
            "jar_sha256": self.sha256,
            **dict(self.options),
        }
        return BaselineInfo(
            self.name,
            self.label,
            self.version,
            java_version(self.java) or "java (not found)",
            options,
            ("-jar", self.jar.name, "--random-seed", "SEED", *self.extra_args, "DOCS", "-w", OUTPUT_FILE),
        )


def jsonoid_from_config(table: Mapping[str, Any], base: Path) -> JsonoidAdapter:
    """A :class:`JsonoidAdapter` from a ``[baselines.jsonoid]`` table; relative paths are under ``base``."""
    for key in ("jar", "sha256", "version"):
        if not isinstance(table.get(key), str) or not table[key]:
            raise ValueError(f"[baselines.jsonoid] needs a non-empty string {key}")
    extra = table.get("extra_args", [])
    if not isinstance(extra, list) or not all(isinstance(a, str) for a in extra):
        raise ValueError("[baselines.jsonoid] extra_args must be a list of strings")
    java = table.get("java", "java")
    if not isinstance(java, str) or not java:
        raise ValueError("[baselines.jsonoid] java must be a non-empty string")
    jar = Path(table["jar"])
    return JsonoidAdapter(
        jar if jar.is_absolute() else base / jar,
        table["sha256"],
        table["version"],
        java,
        tuple(extra),
        {"extra_args": list(extra)} if extra else {},
    )


def jsonoid_adapters(table: Mapping[str, Any], base: Path) -> list[JsonoidAdapter]:
    """The main JSONoid system of a ``[baselines.jsonoid]`` table and one per ``variants`` entry, in name order."""
    main = jsonoid_from_config(table, base)
    variants = table.get("variants", {})
    if not isinstance(variants, Mapping):
        raise ValueError("[baselines.jsonoid.variants] must be a table of variant tables")
    out = [main]
    for name in sorted(variants):
        spec = variants[name]
        if not isinstance(name, str) or not name or not isinstance(spec, Mapping):
            raise ValueError(f"[baselines.jsonoid.variants.{name}] must be a table")
        extra = spec.get("extra_args", [])
        if not isinstance(extra, list) or not all(isinstance(a, str) for a in extra):
            raise ValueError(f"[baselines.jsonoid.variants.{name}] extra_args must be a list of strings")
        label = spec.get("label", f"JSONoid ({name})")
        if not isinstance(label, str) or not label:
            raise ValueError(f"[baselines.jsonoid.variants.{name}] label must be a non-empty string")
        out.append(
            dataclasses.replace(
                main, name=f"jsonoid-{name}", label=label, extra_args=tuple(extra), options={"extra_args": list(extra)} if extra else {}
            )
        )
    return out
