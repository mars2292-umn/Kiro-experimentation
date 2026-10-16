"""Command-line interface for jsinfer.

The subcommands ``infer``, ``explain``, ``list-techniques``, ``list-thresholds``,
``train``, ``split``, and ``bench`` are added as their components land. Exit
codes: 0 success, 1 unrecoverable stage error, 2 configuration or usage error,
3 no valid documents.
"""

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence

import jsinfer

EXIT_USAGE = 2


def build_parser() -> argparse.ArgumentParser:
    """Build the top-level argument parser."""
    parser = argparse.ArgumentParser(
        prog="jsinfer",
        description="Infer a draft 2020-12 JSON Schema from a collection of JSON documents.",
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {jsinfer.__version__}")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """Run the CLI and return the process exit status."""
    parser = build_parser()
    parser.parse_args(argv)
    # No subcommands exist yet, so any invocation without --help/--version is a usage error.
    parser.print_help(sys.stderr)
    return EXIT_USAGE


if __name__ == "__main__":
    sys.exit(main())
