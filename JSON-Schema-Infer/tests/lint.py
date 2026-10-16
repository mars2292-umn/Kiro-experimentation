"""Static lints over the ``src/`` tree (Requirements 27.5, 29.3, 29.5).

Each check parses one module with :mod:`ast` and returns sorted
:class:`Violation` records. ``tests/unit/test_source_lint.py`` runs the checks
over every module under ``src/jsinfer`` and over known-good and known-bad
snippets, so a lint that silently stops matching is caught too.

Import lint (29.3, 29.5)
    Collects absolute ``import`` and ``from ... import`` statements and
    ``importlib.import_module(...)`` / ``__import__(...)`` calls with a literal
    module name. Importing a known JSON Schema validator other than
    ``jsonschema_rs`` (:data:`FOREIGN_VALIDATORS`) is a violation. Every other
    top-level module must be in the standard library, be ``jsinfer`` itself, or
    belong to a declared runtime dependency, so a validator missing from the
    list (or any package that ``uv.lock`` does not pin) is caught as well.

Set-iteration lint (27.5)
    Set iteration order depends on ``PYTHONHASHSEED`` (for str, bytes, and
    containers of them), so the lint flags each order-sensitive use of a
    *set-like expression*:

    - ``for`` loops and ``yield from``;
    - list, dict, and generator comprehensions. A list or generator
      comprehension that is the only positional argument of an
      order-insensitive consumer is allowed: ``sorted``, ``set``,
      ``frozenset``, ``any``, ``all``, ``fsum``, and ``min``/``max`` without
      ``key=``. Set comprehensions are allowed;
    - passing it to ``list``, ``tuple``, ``enumerate``, ``iter``,
      ``reversed``, ``zip``, ``map``, ``filter``, ``sum`` (float sums depend on
      order), ``str``, ``repr``, ``ascii``, ``format``, ``print``, ``min``/``max``
      with ``key=`` (ties), an ``itertools`` function, or a ``.join``,
      ``.extend``, ``.fromkeys``, ``.fromiter``, or ``.writelines`` method;
    - star-unpacking outside a set display, tuple or list unpacking
      assignment, f-string interpolation, and argument-less ``.pop()``.

    An expression is set-like when it is a set display or set comprehension;
    a call of builtin ``set`` or ``frozenset``; ``|``, ``&``, ``-``, or ``^``
    with a set-like or dict-view (``.keys()``, ``.items()``) operand;
    ``union``, ``intersection``, ``difference``, ``symmetric_difference``, or
    ``copy`` on a set-like receiver; a call of a function, method, or property
    defined in the same module and annotated to return a set; a conditional
    expression with a set-like branch; a name whose annotation in its scope is
    a set type, or whose every binding in its scope is set-like;
    ``self.attr`` where the enclosing class annotates or assigns ``attr`` as a
    set; or ``d[k]``, ``d.get(k)``, ``d.pop(k)``, or ``d.setdefault(k, ...)``
    where ``d`` is annotated as a mapping to sets or bound to
    ``defaultdict(set)`` or a dict of sets. Names resolve by Python's scope
    rules within one module.

    Out of scope, to keep the lint free of false positives: values that cross
    a module boundary (imported functions and constants), attributes of objects
    other than ``self``/``cls``, unannotated parameters, flow-sensitive
    rebinding (a name bound to a set and to something else is not tracked),
    and ``sorted(..., key=...)``, whose key is trusted to be a total order. The
    byte-identical reproducibility checks under several ``PYTHONHASHSEED``
    values cover what the lint cannot see.

``hash()`` lint (27.5)
    Builtin ``hash`` is salted per process for str and bytes. Any load of the
    builtin name ``hash`` (a call, or ``key=hash``) and any ``.__hash__``
    attribute load is a violation unless it occurs inside a ``__hash__``
    method. There, hashes only serve set and dict membership. Dict order is
    insertion order, and the set-iteration lint guards set order, so they
    never reach output. A local, enclosing, or module binding named ``hash``
    shadows the builtin.
"""

from __future__ import annotations

import ast
import sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from typing import Final, Literal, TypeAlias

__all__ = [
    "FOREIGN_VALIDATORS",
    "Violation",
    "check_hash",
    "check_imports",
    "check_set_iteration",
    "imported_modules",
]


@dataclass(frozen=True, order=True)
class Violation:
    """One lint finding, ordered by location."""

    path: str
    line: int
    col: int
    message: str

    def __str__(self) -> str:
        return f"{self.path}:{self.line}:{self.col + 1}: {self.message}"


def _at(path: str, node: ast.AST, message: str) -> Violation:
    return Violation(path, getattr(node, "lineno", 0), getattr(node, "col_offset", 0), message)


# ---------------------------------------------------------------------------
# Import lint (Requirements 29.3, 29.5)

# Top-level modules of JSON Schema validators other than jsonschema-rs.
FOREIGN_VALIDATORS: Final = frozenset(
    {
        "fastjsonschema",
        "jschon",
        "jsonschema",
        "jsonspec",
        "openapi_schema_validator",
        "validictory",
    }
)


def _is_dynamic_import(func: ast.expr) -> bool:
    if isinstance(func, ast.Name):
        return func.id in ("__import__", "import_module")
    return (
        isinstance(func, ast.Attribute)
        and func.attr == "import_module"
        and isinstance(func.value, ast.Name)
        and func.value.id == "importlib"
    )


def imported_modules(tree: ast.Module) -> list[tuple[str, ast.AST]]:
    """Absolute module names that ``tree`` imports, each with its importing node."""
    found: list[tuple[str, ast.AST]] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            found.extend((alias.name, node) for alias in node.names)
        elif isinstance(node, ast.ImportFrom):
            if node.level == 0 and node.module:
                found.append((node.module, node))
        elif isinstance(node, ast.Call) and _is_dynamic_import(node.func) and node.args:
            first = node.args[0]
            if isinstance(first, ast.Constant) and isinstance(first.value, str) and not first.value.startswith("."):
                found.append((first.value, node))
    return found


def check_imports(source: str, path: str, allowed_modules: Iterable[str]) -> list[Violation]:
    """Imports of foreign validators, and of modules outside stdlib, jsinfer, and ``allowed_modules``."""
    allowed = frozenset(allowed_modules)
    found: list[Violation] = []
    for name, node in imported_modules(ast.parse(source, path)):
        top = name.partition(".")[0]
        if top in FOREIGN_VALIDATORS:
            message = f"imports {name!r}, a JSON Schema validator other than jsonschema_rs (Requirement 29.3)"
        elif top == "jsinfer" or top in sys.stdlib_module_names or top in allowed:
            continue
        else:
            message = (
                f"imports {name!r}, which is neither the standard library nor a declared runtime "
                "dependency pinned in uv.lock (Requirements 29.3, 29.5)"
            )
        found.append(_at(path, node, message))
    return sorted(found)


# ---------------------------------------------------------------------------
# Scope analysis shared by the set-iteration and hash() lints

Tag: TypeAlias = Literal["other", "set", "set_map"]
OTHER: Final[Tag] = "other"
SET: Final[Tag] = "set"
SET_MAP: Final[Tag] = "set_map"  # a mapping whose values are sets

# A binding is ("value", expr, scope the expr is evaluated in), ("def", the
# FunctionDef, None), or ("opaque", None, None) for anything else.
_Binding: TypeAlias = tuple[str, "ast.AST | None", "_Scope | None"]
_OPAQUE: Final[_Binding] = ("opaque", None, None)

_SET_TYPES: Final = frozenset({"AbstractSet", "FrozenSet", "MutableSet", "Set", "frozenset", "set"})
_MAPPING_TYPES: Final = frozenset(
    {"DefaultDict", "Dict", "Mapping", "MappingProxyType", "MutableMapping", "OrderedDict", "defaultdict", "dict"}
)
_TRANSPARENT_TYPES: Final = frozenset({"Annotated", "ClassVar", "Final", "NotRequired", "Optional", "ReadOnly", "Required"})
_SET_OPS: Final = (ast.BitOr, ast.BitAnd, ast.Sub, ast.BitXor)
_SET_RETURNING_METHODS: Final = frozenset({"copy", "difference", "intersection", "symmetric_difference", "union"})
_MAP_LOOKUP_METHODS: Final = frozenset({"get", "pop", "setdefault"})
_PROPERTY_DECORATORS: Final = frozenset({"cached_property", "property"})

_ORDER_INSENSITIVE_CONSUMERS: Final = frozenset({"all", "any", "frozenset", "fsum", "set", "sorted"})
_ORDER_SENSITIVE_BUILTINS: Final = frozenset(
    {"ascii", "enumerate", "filter", "format", "iter", "list", "map", "print", "repr", "reversed", "str", "sum", "tuple", "zip"}
)
_ORDER_SENSITIVE_METHODS: Final = frozenset({"extend", "fromiter", "fromkeys", "join", "writelines"})

_COMPREHENSIONS: Final = (ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)


def _type_name(node: ast.expr) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return node.attr
    return None


def _decorator_name(node: ast.expr) -> str | None:
    if isinstance(node, ast.Call):
        node = node.func
    return _type_name(node)


def _is_none(node: ast.expr) -> bool:
    return (isinstance(node, ast.Constant) and node.value is None) or (isinstance(node, ast.Name) and node.id == "None")


def _union_members(node: ast.expr) -> list[ast.expr]:
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.BitOr):
        return [*_union_members(node.left), *_union_members(node.right)]
    return [node]


def annotation_tag(node: ast.expr | None) -> Tag:
    """Whether a type annotation names a set, a mapping to sets, or neither."""
    if node is None:
        return OTHER
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        try:
            return annotation_tag(ast.parse(node.value, mode="eval").body)
        except SyntaxError:
            return OTHER
    members = _union_members(node)
    if isinstance(node, ast.Subscript) and _type_name(node.value) == "Union":
        members = list(node.slice.elts) if isinstance(node.slice, ast.Tuple) else [node.slice]
    if len(members) > 1:
        tags = [annotation_tag(member) for member in members if not _is_none(member)]
        return tags[0] if tags and all(tag == tags[0] for tag in tags) else OTHER
    name = _type_name(node.value if isinstance(node, ast.Subscript) else node)
    if name in _SET_TYPES:
        return SET
    if not isinstance(node, ast.Subscript):
        return OTHER
    args = list(node.slice.elts) if isinstance(node.slice, ast.Tuple) else [node.slice]
    if name in _TRANSPARENT_TYPES:
        return annotation_tag(args[0])
    if name in _MAPPING_TYPES and len(args) == 2 and annotation_tag(args[1]) == SET:
        return SET_MAP
    return OTHER


class _Scope:
    """A module, class, function, lambda, or comprehension scope."""

    def __init__(self, node: ast.AST, parent: _Scope | None) -> None:
        self.node = node
        self.parent = parent
        if isinstance(node, ast.Module):
            self.kind = "module"
        elif isinstance(node, ast.ClassDef):
            self.kind = "class"
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
            self.kind = "function"
        else:
            self.kind = "comprehension"
        self.bindings: dict[str, list[_Binding]] = {}
        self.declared: dict[str, list[ast.expr]] = {}
        self.global_names: list[str] = []
        self.nonlocal_names: list[str] = []
        # Class scopes: instance attributes bound through the self parameter of a method.
        self.attr_bindings: dict[str, list[_Binding]] = {}
        self.attr_declared: dict[str, list[ast.expr]] = {}
        # Method scopes: the name of the self or cls parameter.
        self.self_name: str | None = None


class _Analysis:
    """Scopes, bindings, and parents of every node of one module."""

    def __init__(self, tree: ast.Module) -> None:
        self.parent: dict[ast.AST, ast.AST] = {}
        self.scope_of: dict[ast.AST, _Scope] = {}  # node -> scope the node is evaluated in
        self.opened: dict[ast.AST, _Scope] = {}  # scope node -> the scope it opens
        self.itertools_names: list[str] = []
        self._busy: list[tuple[object, ...]] = []
        self.module = _Scope(tree, None)
        self.opened[tree] = self.module
        for child in ast.iter_child_nodes(tree):
            self._visit(child, tree, self.module)

    # -- construction ------------------------------------------------------

    def _open(self, node: ast.AST, scope: _Scope) -> _Scope:
        inner = _Scope(node, scope)
        self.opened[node] = inner
        return inner

    def _visit(self, node: ast.AST, parent: ast.AST, scope: _Scope) -> None:
        self.parent[node] = parent
        self.scope_of[node] = scope
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            self._bind(scope, node.name, ("def", node, None))
            for decorator in node.decorator_list:
                self._visit(decorator, node, scope)
            inner = self._open(node, scope)
            self._visit_arguments(node.args, node, scope, inner)
            if node.returns is not None:
                self._visit(node.returns, node, scope)
            static = any(_decorator_name(d) == "staticmethod" for d in node.decorator_list)
            params = [*node.args.posonlyargs, *node.args.args]
            if scope.kind == "class" and not static and params:
                inner.self_name = params[0].arg
            for statement in node.body:
                self._visit(statement, node, inner)
        elif isinstance(node, ast.Lambda):
            inner = self._open(node, scope)
            self._visit_arguments(node.args, node, scope, inner)
            self._visit(node.body, node, inner)
        elif isinstance(node, ast.ClassDef):
            self._bind(scope, node.name, _OPAQUE)
            for child in [*node.decorator_list, *node.bases, *node.keywords]:
                self._visit(child, node, scope)
            inner = self._open(node, scope)
            for statement in node.body:
                self._visit(statement, node, inner)
        elif isinstance(node, _COMPREHENSIONS):
            inner = self._open(node, scope)
            for i, generator in enumerate(node.generators):
                self.parent[generator] = node
                self.scope_of[generator] = inner
                # The first iterable is evaluated in the enclosing scope.
                self._visit(generator.iter, generator, scope if i == 0 else inner)
                self._bind_target(generator.target, None, inner)
                self._visit(generator.target, generator, inner)
                for condition in generator.ifs:
                    self._visit(condition, generator, inner)
            parts = [node.key, node.value] if isinstance(node, ast.DictComp) else [node.elt]
            for part in parts:
                self._visit(part, node, inner)
        else:
            self._record_bindings(node, scope)
            for child in ast.iter_child_nodes(node):
                self._visit(child, node, scope)

    def _visit_arguments(self, args: ast.arguments, owner: ast.AST, outer: _Scope, inner: _Scope) -> None:
        self.parent[args] = owner
        self.scope_of[args] = outer
        for arg in [*args.posonlyargs, *args.args, *args.kwonlyargs]:
            self._bind(inner, arg.arg, _OPAQUE)
            if arg.annotation is not None:
                inner.declared.setdefault(arg.arg, []).append(arg.annotation)
        for arg in (args.vararg, args.kwarg):
            if arg is not None:
                self._bind(inner, arg.arg, _OPAQUE)
        # Annotations and defaults are evaluated in the enclosing scope.
        for child in ast.iter_child_nodes(args):
            self._visit(child, args, outer)

    def _record_bindings(self, node: ast.AST, scope: _Scope) -> None:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                self._bind_target(target, node.value, scope)
        elif isinstance(node, ast.AnnAssign):
            target = node.target
            if isinstance(target, ast.Name):
                self._declare(scope, target.id, node.annotation)
                if node.value is not None:
                    self._bind(scope, target.id, ("value", node.value, scope))
            elif isinstance(target, ast.Attribute):
                owner = self._method_class(scope, target.value)
                if owner is not None:
                    owner.attr_declared.setdefault(target.attr, []).append(node.annotation)
                    if node.value is not None:
                        owner.attr_bindings.setdefault(target.attr, []).append(("value", node.value, scope))
        elif isinstance(node, (ast.For, ast.AsyncFor)):
            self._bind_target(node.target, None, scope)
        elif isinstance(node, ast.withitem) and node.optional_vars is not None:
            self._bind_target(node.optional_vars, None, scope)
        elif isinstance(node, ast.ExceptHandler) and node.name:
            self._bind(scope, node.name, _OPAQUE)
        elif isinstance(node, (ast.Import, ast.ImportFrom)):
            for alias in node.names:
                if alias.name != "*":
                    self._bind(scope, alias.asname or alias.name.partition(".")[0], _OPAQUE)
                    if isinstance(node, ast.ImportFrom) and node.module == "itertools":
                        self.itertools_names.append(alias.asname or alias.name)
        elif isinstance(node, ast.NamedExpr) and isinstance(node.target, ast.Name):
            owner = scope
            while owner.kind == "comprehension" and owner.parent is not None:
                owner = owner.parent
            self._bind(owner, node.target.id, ("value", node.value, scope))
        elif isinstance(node, ast.Global):
            scope.global_names.extend(node.names)
        elif isinstance(node, ast.Nonlocal):
            scope.nonlocal_names.extend(node.names)
        elif isinstance(node, (ast.MatchAs, ast.MatchStar)) and node.name:
            self._bind(scope, node.name, _OPAQUE)
        elif isinstance(node, ast.MatchMapping) and node.rest:
            self._bind(scope, node.rest, _OPAQUE)

    def _bind_target(self, target: ast.expr, value: ast.expr | None, scope: _Scope) -> None:
        if isinstance(target, ast.Name):
            self._bind(scope, target.id, ("value", value, scope) if value is not None else _OPAQUE)
        elif isinstance(target, (ast.Tuple, ast.List)):
            for element in target.elts:
                self._bind_target(element, None, scope)
        elif isinstance(target, ast.Starred):
            self._bind_target(target.value, None, scope)
        elif isinstance(target, ast.Attribute):
            owner = self._method_class(scope, target.value)
            if owner is not None:
                binding = ("value", value, scope) if value is not None else _OPAQUE
                owner.attr_bindings.setdefault(target.attr, []).append(binding)

    def _target_scope(self, scope: _Scope, name: str) -> _Scope:
        if name in scope.global_names:
            return self.module
        if name in scope.nonlocal_names and scope.parent is not None:
            owner = self.resolve(name, scope.parent)
            if owner is not None:
                return owner
        return scope

    def _bind(self, scope: _Scope, name: str, binding: _Binding) -> None:
        self._target_scope(scope, name).bindings.setdefault(name, []).append(binding)

    def _declare(self, scope: _Scope, name: str, annotation: ast.expr) -> None:
        self._target_scope(scope, name).declared.setdefault(name, []).append(annotation)

    # -- resolution ---------------------------------------------------------

    def resolve(self, name: str, scope: _Scope) -> _Scope | None:
        """The scope that binds ``name`` as seen from ``scope``; None for builtins and unknowns."""
        current: _Scope | None = scope
        first = True
        while current is not None:
            if name in current.global_names:
                return self.module if name in self.module.bindings or name in self.module.declared else None
            if name in current.nonlocal_names:
                current, first = current.parent, False
                continue
            # A class scope is visible only from its own body.
            if (first or current.kind != "class") and (name in current.bindings or name in current.declared):
                return current
            current, first = current.parent, False
        return None

    def _method_class(self, scope: _Scope, receiver: ast.expr) -> _Scope | None:
        """The class scope when ``receiver`` is the self/cls parameter of an enclosing method."""
        if not isinstance(receiver, ast.Name):
            return None
        current: _Scope | None = scope
        # Comprehensions and lambdas inside a method still see its self parameter.
        while current is not None and (current.kind == "comprehension" or isinstance(current.node, ast.Lambda)):
            current = current.parent
        if current is None or current.self_name != receiver.id or current.parent is None:
            return None
        return current.parent if current.parent.kind == "class" else None

    def inside_function(self, scope: _Scope, name: str) -> bool:
        current: _Scope | None = scope
        while current is not None:
            if isinstance(current.node, (ast.FunctionDef, ast.AsyncFunctionDef)) and current.node.name == name:
                return True
            current = current.parent
        return False

    # -- classification -----------------------------------------------------

    def tag(self, node: ast.AST | None, scope: _Scope) -> Tag | None:
        """SET, SET_MAP, OTHER, or None while the value depends on a binding being classified."""
        if isinstance(node, (ast.Set, ast.SetComp)):
            return SET
        if isinstance(node, ast.DictComp):
            return SET_MAP if self.tag(node.value, self.opened[node]) == SET else OTHER
        if isinstance(node, ast.Dict):
            values = node.values
            if values and all(key is not None for key in node.keys) and all(self.tag(v, scope) == SET for v in values):
                return SET_MAP
            return OTHER
        if isinstance(node, ast.Call):
            return self._call_tag(node, scope)
        if isinstance(node, ast.BinOp) and isinstance(node.op, _SET_OPS):
            sides = [self.tag(node.left, scope), self.tag(node.right, scope)]
            if SET in sides or _is_dict_view(node.left) or _is_dict_view(node.right):
                return SET
            if isinstance(node.op, ast.BitOr) and sides == [SET_MAP, SET_MAP]:
                return SET_MAP
            return None if None in sides else OTHER
        if isinstance(node, (ast.IfExp, ast.BoolOp)):
            # Either branch (``a if c else b``) or operand (``a or b``) may be the result.
            operands = [node.body, node.orelse] if isinstance(node, ast.IfExp) else node.values
            branches = [self.tag(operand, scope) for operand in operands]
            for wanted in (SET, SET_MAP, None):
                if wanted in branches:
                    return wanted
            return OTHER
        if isinstance(node, ast.NamedExpr):
            return self.tag(node.value, scope)
        if isinstance(node, ast.Name):
            owner = self.resolve(node.id, scope)
            if owner is None:
                return OTHER
            key = ("name", id(owner), node.id)
            return self._symbol_tag(key, owner.declared.get(node.id, []), owner.bindings.get(node.id, []), attr=False)
        if isinstance(node, ast.Attribute):
            owner = self._method_class(scope, node.value)
            if owner is None:
                return OTHER
            name = node.attr
            declared = [*owner.declared.get(name, []), *owner.attr_declared.get(name, [])]
            bindings = [*owner.bindings.get(name, []), *owner.attr_bindings.get(name, [])]
            return self._symbol_tag(("attr", id(owner), name), declared, bindings, attr=True)
        if isinstance(node, ast.Subscript):
            return SET if self.tag(node.value, scope) == SET_MAP else OTHER
        return OTHER

    def is_set(self, node: ast.AST, scope: _Scope) -> bool:
        return self.tag(node, scope) == SET

    def _symbol_tag(
        self, key: tuple[object, ...], declared: Sequence[ast.expr], bindings: Sequence[_Binding], *, attr: bool
    ) -> Tag | None:
        # An annotation declares the type of every binding in the scope.
        declared_tags = [annotation_tag(annotation) for annotation in declared]
        if declared_tags and all(t == declared_tags[0] for t in declared_tags) and declared_tags[0] != OTHER:
            return declared_tags[0]
        if key in self._busy:
            return None  # a binding that refers to itself (x = x | y) does not decide the tag
        self._busy.append(key)
        try:
            tags = [self._binding_tag(binding, attr=attr) for binding in bindings]
        finally:
            self._busy.pop()
        known = [t for t in tags if t is not None]
        if not known:
            return None if tags else OTHER
        return known[0] if all(t == known[0] for t in known) else OTHER

    def _binding_tag(self, binding: _Binding, *, attr: bool) -> Tag | None:
        kind, node, scope = binding
        if kind == "value" and scope is not None:
            return self.tag(node, scope)
        if kind == "def" and attr and isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            names = [_decorator_name(d) for d in node.decorator_list]
            if any(name in _PROPERTY_DECORATORS for name in names):
                return annotation_tag(node.returns)
            if any(name in ("setter", "deleter") for name in names):
                return None  # the property's getter decides
        return OTHER

    def _returns_tag(self, bindings: Sequence[_Binding]) -> Tag:
        """The annotated return tag when every binding of a callee is a ``def``."""
        defs = [node for kind, node, _ in bindings if kind == "def" and isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))]
        if not defs or len(defs) != len(bindings):
            return OTHER
        tags = [annotation_tag(fn.returns) for fn in defs]
        return tags[0] if all(t == tags[0] for t in tags) else OTHER

    def _call_tag(self, node: ast.Call, scope: _Scope) -> Tag | None:
        func = node.func
        if _type_name(func) == "defaultdict" and node.args:
            factory = node.args[0]
            if isinstance(factory, ast.Name) and factory.id in ("set", "frozenset"):
                return SET_MAP
            if isinstance(factory, ast.Lambda) and self.tag(factory.body, self.opened[factory]) == SET:
                return SET_MAP
            return OTHER
        if isinstance(func, ast.Name):
            owner = self.resolve(func.id, scope)
            if owner is None:
                return SET if func.id in ("set", "frozenset") else OTHER
            return self._returns_tag(owner.bindings.get(func.id, []))
        if isinstance(func, ast.Attribute):
            receiver = self.tag(func.value, scope)
            if receiver == SET and func.attr in _SET_RETURNING_METHODS:
                return SET
            if receiver == SET_MAP:
                if func.attr in _MAP_LOOKUP_METHODS:
                    return SET
                if func.attr == "copy":
                    return SET_MAP
            owner = self._method_class(scope, func.value)
            if owner is not None:
                return self._returns_tag(owner.bindings.get(func.attr, []))
        return OTHER


def _is_dict_view(node: ast.expr) -> bool:
    return (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and node.func.attr in ("keys", "items")
        and not node.args
        and not node.keywords
    )


# ---------------------------------------------------------------------------
# Set-iteration lint (Requirement 27.5)


def _has_key(call: ast.Call) -> bool:
    return any(keyword.arg == "key" for keyword in call.keywords)


def _order_insensitive_consumer(analysis: _Analysis, comprehension: ast.AST) -> bool:
    call = analysis.parent.get(comprehension)
    if not isinstance(call, ast.Call) or len(call.args) != 1 or call.args[0] is not comprehension:
        return False
    name = _type_name(call.func)
    return name in _ORDER_INSENSITIVE_CONSUMERS or (name in ("min", "max") and not _has_key(call))


def check_set_iteration(source: str, path: str) -> list[Violation]:
    """Order-sensitive uses of set-like expressions (see the module docstring for the scope)."""
    analysis = _Analysis(ast.parse(source, path))
    found: list[Violation] = []

    def check(node: ast.AST, scope: _Scope, construct: str) -> None:
        if analysis.is_set(node, scope):
            found.append(
                _at(path, node, f"{construct} iterates a set in hash order; iterate sorted(...) instead (Requirement 27.5)")
            )

    for node, scope in analysis.scope_of.items():
        if isinstance(node, (ast.For, ast.AsyncFor)):
            check(node.iter, scope, "a for loop")
        elif isinstance(node, (ast.ListComp, ast.DictComp, ast.GeneratorExp)):
            if isinstance(node, ast.DictComp) or not _order_insensitive_consumer(analysis, node):
                inner = analysis.opened[node]
                for i, generator in enumerate(node.generators):
                    check(generator.iter, scope if i == 0 else inner, "a comprehension")
        elif isinstance(node, ast.YieldFrom):
            check(node.value, scope, "yield from")
        elif isinstance(node, ast.Starred) and isinstance(node.ctx, ast.Load):
            if not isinstance(analysis.parent.get(node), ast.Set):
                check(node.value, scope, "star-unpacking")
        elif isinstance(node, ast.Assign) and any(isinstance(t, (ast.Tuple, ast.List)) for t in node.targets):
            check(node.value, scope, "unpacking assignment")
        elif isinstance(node, ast.FormattedValue):
            check(node.value, scope, "an f-string")
        elif isinstance(node, ast.Call):
            _check_call(analysis, node, scope, check)
    return sorted(found)


def _check_call(analysis: _Analysis, node: ast.Call, scope: _Scope, check: Callable[[ast.AST, _Scope, str], None]) -> None:
    func = node.func
    arguments = [arg for arg in node.args if not isinstance(arg, ast.Starred)]
    if isinstance(func, ast.Name):
        builtin = analysis.resolve(func.id, scope) is None
        if not (
            (builtin and func.id in _ORDER_SENSITIVE_BUILTINS)
            or (builtin and func.id in ("min", "max") and _has_key(node))
            or func.id in analysis.itertools_names
        ):
            return
        construct = f"{func.id}(...)"
    elif isinstance(func, ast.Attribute):
        if func.attr == "pop" and not node.args and not node.keywords:
            if analysis.is_set(func.value, scope):
                check(func.value, scope, ".pop()")
            return
        is_itertools = isinstance(func.value, ast.Name) and func.value.id == "itertools"
        if func.attr not in _ORDER_SENSITIVE_METHODS and not is_itertools:
            return
        construct = f".{func.attr}(...)"
    else:
        return
    for argument in arguments:
        check(argument, scope, construct)


# ---------------------------------------------------------------------------
# hash() lint (Requirement 27.5)


def check_hash(source: str, path: str) -> list[Violation]:
    """Loads of builtin ``hash`` or of ``.__hash__`` outside ``__hash__`` methods."""
    analysis = _Analysis(ast.parse(source, path))
    found: list[Violation] = []
    for node, scope in analysis.scope_of.items():
        if not isinstance(getattr(node, "ctx", None), ast.Load) or analysis.inside_function(scope, "__hash__"):
            continue
        if isinstance(node, ast.Name) and node.id == "hash" and analysis.resolve("hash", scope) is None:
            what = "builtin hash()"
        elif isinstance(node, ast.Attribute) and node.attr == "__hash__":
            what = ".__hash__"
        else:
            continue
        found.append(
            _at(path, node, f"uses {what}, which PYTHONHASHSEED salts per process; use canonical bytes or BLAKE2b (Requirement 27.5)")
        )
    return sorted(found)
