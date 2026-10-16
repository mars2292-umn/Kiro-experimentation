"""Source lints over ``src/jsinfer`` (task 1.3; Requirements 27.5, 29.3, 29.5).

The checks live in :mod:`tests.lint`, whose docstring defines what each one
covers. This module runs them over every source module and pins their
behavior on small known-bad and known-good snippets, so a lint that stops
matching fails here instead of passing vacuously.
"""

from __future__ import annotations

import importlib.metadata
import re
import tomllib
from pathlib import Path

import pytest

from tests.lint import FOREIGN_VALIDATORS, check_hash, check_imports, check_set_iteration

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src" / "jsinfer"
SOURCES = sorted(SRC.rglob("*.py"))


def _normalize(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def _requirement_name(requirement: str) -> str:
    match = re.match(r"\s*([A-Za-z0-9][A-Za-z0-9._-]*)", requirement)
    assert match, f"unparseable requirement {requirement!r}"
    return _normalize(match.group(1))


def _runtime_distributions() -> list[str]:
    pyproject = tomllib.loads((ROOT / "pyproject.toml").read_text("utf-8"))
    return sorted(_requirement_name(r) for r in pyproject["project"]["dependencies"])


def _runtime_modules() -> frozenset[str]:
    """Top-level modules provided by the declared runtime dependencies."""
    declared = _runtime_distributions()
    provided: dict[str, list[str]] = {name: [] for name in declared}
    for module, distributions in sorted(importlib.metadata.packages_distributions().items()):
        for distribution in distributions:
            if _normalize(distribution) in provided:
                provided[_normalize(distribution)].append(module)
    missing = sorted(name for name, modules in provided.items() if not modules)
    assert not missing, f"declared runtime dependencies are not installed: {missing}"
    return frozenset(module for modules in provided.values() for module in modules)


def _report(violations: list[object]) -> str:
    return "\n".join(str(v) for v in violations)


def _relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


# ---------------------------------------------------------------------------
# The source tree


def test_source_tree_is_found() -> None:
    names = {_relative(path) for path in SOURCES}
    assert "src/jsinfer/core/determinism.py" in names
    assert "src/jsinfer/cli.py" in names


def test_jsonschema_rs_is_a_declared_runtime_dependency() -> None:
    assert "jsonschema-rs" in _runtime_distributions()
    assert "jsonschema_rs" in _runtime_modules()


def test_src_imports_only_stdlib_jsinfer_and_declared_dependencies() -> None:
    """No validator other than jsonschema_rs, and nothing that uv.lock does not pin (29.3, 29.5)."""
    allowed = _runtime_modules()
    violations = [
        v for path in SOURCES for v in check_imports(path.read_text("utf-8"), _relative(path), allowed)
    ]
    assert not violations, _report(violations)


def test_src_never_iterates_a_set_in_hash_order() -> None:
    violations = [v for path in SOURCES for v in check_set_iteration(path.read_text("utf-8"), _relative(path))]
    assert not violations, _report(violations)


def test_src_never_uses_builtin_hash() -> None:
    violations = [v for path in SOURCES for v in check_hash(path.read_text("utf-8"), _relative(path))]
    assert not violations, _report(violations)


# ---------------------------------------------------------------------------
# Import lint behavior

ALLOWED = frozenset({"jsonschema_rs", "numpy"})


@pytest.mark.parametrize(
    "source",
    [
        "import jsonschema",
        "from jsonschema.validators import Draft202012Validator",
        "import fastjsonschema as fjs",
        "from jschon import JSONSchema",
        "import importlib\nimportlib.import_module('jsonschema')",
        "__import__('fastjsonschema')",
    ],
)
def test_import_lint_flags_foreign_validators(source: str) -> None:
    violations = check_imports(source, "m.py", ALLOWED)
    assert len(violations) == 1
    assert "29.3" in violations[0].message
    assert "other than jsonschema_rs" in violations[0].message


@pytest.mark.parametrize("source", ["import yaml", "from pydantic import BaseModel", "import hypothesis"])
def test_import_lint_flags_undeclared_packages(source: str) -> None:
    violations = check_imports(source, "m.py", ALLOWED)
    assert len(violations) == 1
    assert "declared runtime dependency" in violations[0].message


@pytest.mark.parametrize(
    "source",
    [
        "import jsonschema_rs",
        "from __future__ import annotations\nimport json, os.path\nfrom collections.abc import Mapping",
        "from jsinfer.core import paths\nfrom . import sibling\nfrom ..core import determinism",
        "import numpy as np\nfrom numpy.random import SeedSequence",
        "import importlib\nimportlib.import_module('.relative', 'jsinfer')",
    ],
)
def test_import_lint_accepts_stdlib_jsinfer_and_allowed(source: str) -> None:
    assert check_imports(source, "m.py", ALLOWED) == []


def test_foreign_validators_exclude_jsonschema_rs() -> None:
    assert "jsonschema_rs" not in FOREIGN_VALIDATORS
    assert "jsonschema" in FOREIGN_VALIDATORS


# ---------------------------------------------------------------------------
# Set-iteration lint behavior

# Each snippet has one violation, on the line marked with "# <-".
BAD_SET_USES = {
    "for over a set literal": "for x in {1, 2}:  # <-\n    pass",
    "for over set()": "for x in set(ys):  # <-\n    pass",
    "for over a bound frozenset": "xs = frozenset(ys)\nfor x in xs:  # <-\n    pass",
    "annotated parameter": "def f(xs: set[str]) -> None:\n    for x in xs:  # <-\n        pass",
    "optional annotated parameter": "def f(xs: 'frozenset[str] | None') -> list[str]:\n    return [x for x in xs or ()]  # <-",
    "list comprehension": "out = [x for x in set(ys)]  # <-",
    "dict comprehension": "out = {k: 1 for k in set(ys)}  # <-",
    "nested comprehension iterable": "out = [y for x in xs for y in set(x)]  # <-",
    "list()": "out = list(set(ys))  # <-",
    "str.join": "out = ', '.join(set(ys))  # <-",
    "set algebra": "a = {1}\nb = a | other\nfor x in b:  # <-\n    pass",
    "dict-view difference": "for k in d.keys() - other:  # <-\n    pass",
    "union method": "out = tuple(frozenset(a).union(b))  # <-",
    "module constant in a function": (
        "from typing import Final\nNAMES: Final = frozenset({'a'})\ndef f():\n    return tuple(NAMES)  # <-"
    ),
    "self attribute declared on the class": (
        "class C:\n    items: frozenset[str]\n    def f(self):\n        return [x for x in self.items]  # <-"
    ),
    "self attribute assigned in __init__": (
        "class C:\n    def __init__(self, ys):\n        self.seen = set(ys)\n"
        "    def f(self):\n        for x in self.seen:  # <-\n            pass"
    ),
    "property returning a set": (
        "class C:\n    @property\n    def keys_(self) -> set[str]:\n        return set()\n"
        "    def f(self):\n        return list(self.keys_)  # <-"
    ),
    "function returning a set": "def f() -> set[str]:\n    return set()\nfor x in f():  # <-\n    pass",
    "defaultdict(set) lookup": (
        "from collections import defaultdict\ng = defaultdict(set)\nfor x in g['k']:  # <-\n    pass"
    ),
    "mapping-to-set annotation .get": (
        "def f(g: dict[str, set[int]]) -> list[int]:\n    return list(g.get('k', ()))  # <-"
    ),
    "conditional expression": "s = set(ys) if c else ()\nfor x in s:  # <-\n    pass",
    "walrus": "if (s := set(ys)):\n    out = list(s)  # <-",
    "pop": "s = set(ys)\nfirst = s.pop()  # <-",
    "f-string": "s = set(ys)\nmsg = f'got {s!r}'  # <-",
    "star-unpacking": "s = set(ys)\nprint(*s)  # <-",
    "unpacking assignment": "a, b = {x, y}  # <-",
    "sum": "total = sum(x for x in set(ys))  # <-",
    "min with key": "best = min(set(ys), key=len)  # <-",
    "itertools": "import itertools\nfor p in itertools.combinations(set(ys), 2):  # <-\n    pass",
    "imported itertools function": "from itertools import chain\nout = list(chain(set(ys), zs))  # <-",
    "yield from": "def g():\n    yield from set(ys)  # <-",
    "closure over an enclosing set": "def f(ys):\n    s = set(ys)\n    def g():\n        return list(s)  # <-\n    return g",
}


def _marked_line(source: str) -> int:
    lines = [i for i, line in enumerate(source.splitlines(), start=1) if line.endswith("# <-")]
    assert len(lines) == 1, "each bad snippet marks exactly one line"
    return lines[0]


@pytest.mark.parametrize("source", list(BAD_SET_USES.values()), ids=list(BAD_SET_USES))
def test_set_lint_flags_order_sensitive_uses(source: str) -> None:
    violations = check_set_iteration(source, "m.py")
    assert [v.line for v in violations] == [_marked_line(source)], _report(violations)
    assert "27.5" in violations[0].message


GOOD_SET_USES = {
    "sorted first": "for x in sorted(set(ys)):\n    pass",
    "membership and size": "s = frozenset(ys)\nok = 'a' in s and len(s) > 1 and s <= other",
    "set comprehension": "u = {x.lower() for x in set(ys)}",
    "order-insensitive consumers": (
        "import math\ns = set(ys)\n"
        "a = sorted(x for x in s)\nb = max(x for x in s)\nc = any(x > 1 for x in s)\n"
        "d = frozenset(x for x in s)\ne = math.fsum(x for x in s)\nf = min(s)"
    ),
    "rebound to a list": "xs = set(ys)\nxs = sorted(xs)\nfor x in xs:\n    pass",
    "dict views iterate in insertion order": "for k in d.keys():\n    pass\nfor k, v in d.items():\n    pass",
    "list parameter": "def f(xs: list[str]):\n    for x in xs:\n        pass",
    "parameter shadows a module set": "s = set(ys)\ndef f(s):\n    for x in s:\n        pass",
    "other attribute type": (
        "class C:\n    items: tuple[str, ...]\n    def f(self):\n        return [x for x in self.items]"
    ),
    "class attribute name is not a local": (
        "class C:\n    items = frozenset({1})\n    def f(self, items):\n        return list(items)"
    ),
    "unpack sorted": "a, b = sorted({x, y})",
    "set display unpacking": "s = set(ys)\nt = {*s, 1}",
    "f-string of sorted": "s = set(ys)\nmsg = f'{sorted(s)}'",
    "list.pop is not set.pop": "xs = list(ys)\nlast = xs.pop()",
    "sorted then join": "out = ', '.join(sorted(set(ys)))",
    "integer arithmetic is not set algebra": "n = len(xs) - 1\nfor i in range(n | 1):\n    pass",
}


@pytest.mark.parametrize("source", list(GOOD_SET_USES.values()), ids=list(GOOD_SET_USES))
def test_set_lint_accepts_order_insensitive_uses(source: str) -> None:
    assert check_set_iteration(source, "m.py") == []


# ---------------------------------------------------------------------------
# hash() lint behavior


@pytest.mark.parametrize(
    "source",
    [
        "k = hash('a')",
        "out = sorted(xs, key=hash)",
        "def f(x):\n    return hash(x) % 8",
        "v = obj.__hash__()",
        "class C:\n    def key(self):\n        return hash(self.name)",
    ],
)
def test_hash_lint_flags_builtin_hash(source: str) -> None:
    violations = check_hash(source, "m.py")
    assert len(violations) == 1
    assert "PYTHONHASHSEED" in violations[0].message


@pytest.mark.parametrize(
    "source",
    [
        "class C:\n    def __hash__(self):\n        return hash((self.a, self.b))",
        "class C:\n    __hash__ = None",
        "class C:\n    def __hash__(self):\n        return object.__hash__(self)",
        "def f(hash):\n    return hash(1)",
        "from mylib import hash\nk = hash('a')",
        "import hashlib\nd = hashlib.blake2b(b'x', digest_size=4).digest()",
    ],
)
def test_hash_lint_accepts_dunder_hash_and_shadowing(source: str) -> None:
    assert check_hash(source, "m.py") == []
