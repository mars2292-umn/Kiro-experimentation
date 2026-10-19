"""The Naive_Merge adapter (task 28.2; Requirements 34.1, 34.4, 34.11).

Naive_Merge is ``pipeline.naive_merge``: the schema with the five-keyword
set of 34.4 (``type``, ``properties``, ``required``, ``items``, ``enum``)
and nothing else. So that it is timed and bounded like every other
system (34.2, 34.3, 34.7), the adapter runs it in a child process:

    python -m jsinfer.bench.baselines.naive DOCS.jsonl OUT_DIR

reads the JSONL file with the Ingestor's document parser (so the documents
are the ones the Inferrer sees, large integers included) and writes
``OUT_DIR/schema.json`` as the Schema_Emitter emits it, in draft 2020-12.
"""

from __future__ import annotations

import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

import jsinfer
from jsinfer.bench.baselines.base import Availability, BaselineInfo
from jsinfer.core.jsonvalue import JSON
from jsinfer.io.validator import Draft

__all__ = ["OUTPUT_FILE", "NaiveMergeAdapter", "main"]

OUTPUT_FILE = "schema.json"


def _read_jsonl(path: Path) -> list[JSON]:
    from jsinfer.io.ingest import parse_document

    documents: list[JSON] = []
    with open(path, "rb") as handle:
        for line in handle:
            if line.strip():
                documents.append(parse_document(line)[0])
    return documents


def main(argv: Sequence[str] | None = None) -> int:
    """``naive DOCS.jsonl OUT_DIR``: write the Naive_Merge schema of the collection (see the module docstring)."""
    from jsinfer.io.emit import SchemaEmitter
    from jsinfer.pipeline.naive_merge import naive_merge

    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) != 2:
        print("usage: python -m jsinfer.bench.baselines.naive DOCS.jsonl OUT_DIR", file=sys.stderr)
        return 2
    docs, out_dir = Path(args[0]), Path(args[1])
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / OUTPUT_FILE).write_bytes(SchemaEmitter().emit(naive_merge(_read_jsonl(docs))))
    return 0


@dataclass(frozen=True)
class NaiveMergeAdapter:
    """Naive_Merge as a Baseline_Adapter (see the module docstring)."""

    python: str = sys.executable
    name: str = "naive_merge"
    label: str = "Naive_Merge"

    @property
    def output_draft(self) -> Draft | None:
        return None

    def available(self) -> Availability:
        return Availability(True) if Path(self.python).exists() else Availability(False, f"no interpreter {self.python}")

    def command(self, docs_jsonl: Path, out_dir: Path, seed: int) -> Sequence[str]:
        return [self.python, "-I", "-m", "jsinfer.bench.baselines.naive", str(docs_jsonl), str(out_dir)]

    def output_path(self, out_dir: Path) -> Path:
        return out_dir / OUTPUT_FILE

    def describe(self) -> BaselineInfo:
        return BaselineInfo(
            self.name,
            self.label,
            jsinfer.__version__,
            f"python {sys.version.split()[0]}",
            {"keywords": ["type", "properties", "required", "items", "enum"]},
            ("-I", "-m", "jsinfer.bench.baselines.naive"),
        )


if __name__ == "__main__":
    sys.exit(main())
