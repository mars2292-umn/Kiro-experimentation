"""Field-name normalization and normalized Path sets (Requirements 24.2, 25.1).

:func:`normalize_name` splits a field name into tokens at underscores, at
hyphens, and at each transition from a lowercase letter or a digit to an
uppercase letter, then casefolds every token (24.2). ``emailAddress``,
``email_address``, ``email-address``, ``EmailAddress``, and ``EMAIL_ADDRESS``
all normalize to ``("email", "address")``. Empty tokens (leading, trailing, or
repeated separators) are dropped, so ``"_"`` normalizes to ``()``. The
normalization is idempotent: rendering the tokens with any separator and
normalizing again gives the same tokens, and each token normalizes to
itself. The Trainer applies it before indexing and the Corpus_Advisor before
lookup, so both sides agree.

:func:`normalized_paths` is the normalized Path set of a Schema_Model (25.1):
the Paths of the model taken across all branches, with each ``$ref`` expanded
up to a Path depth and each object-key step replaced by its normalized name.
The stepping is the one the Inferrer and the label derivation use
(``learn.labels``): ``properties`` adds a Key step, ``patternProperties`` and
a schema-valued ``additionalProperties`` add a MapVal step, and ``items`` and
``prefixItems`` add an Elem step. ``anyOf``, ``oneOf``, ``allOf``, ``if``,
``then``, and ``else`` subschemas stand at the Path of their owner; ``not``,
``propertyNames``, ``contains``, and ``$defs`` contribute no Paths (``$defs``
entries are reached through ``$ref``). A ``$ref`` is followed when the
referencing node stands at a Path of at most ``ref_depth`` steps, so a
recursive schema yields a finite set; reference chains that add no step are
followed once per chain. The set holds rendered Path strings (``$`` for the
root); the Key step of a normalized name is rendered with the tokens joined
by ``_``.
"""

from __future__ import annotations

from collections.abc import Iterable, Iterator
from typing import Final
from urllib.parse import unquote

from jsinfer.core.determinism import codepoint_key
from jsinfer.core.paths import ELEM_STEP, MAPVAL_STEP, Path, key_step, render_path, sorted_paths
from jsinfer.core.schema_model import SchemaModel, SchemaNode, split_pointer

__all__ = [
    "DEFAULT_REF_DEPTH",
    "SEPARATORS",
    "join_tokens",
    "normalize_name",
    "normalize_path",
    "normalized_paths",
    "ref_target",
    "sorted_normalized_paths",
    "walk_paths",
]

# The separators of 24.2.
SEPARATORS: Final = frozenset("_-")
# The Path depth default of nearest.ref_depth (config.resources); kept here so
# this module does not import the configuration.
DEFAULT_REF_DEPTH: Final = 8


def normalize_name(name: str) -> tuple[str, ...]:
    """Return the normalized tokens of field name ``name`` (Requirement 24.2)."""
    if not isinstance(name, str):
        raise TypeError(f"a field name is a str, got {type(name).__name__}")
    tokens: list[str] = []
    current: list[str] = []
    previous = ""
    for ch in name:
        if ch in SEPARATORS:
            if current:
                tokens.append("".join(current))
                current = []
        else:
            if current and ch.isupper() and (previous.islower() or previous.isdigit()):
                tokens.append("".join(current))
                current = []
            current.append(ch)
        previous = ch
    if current:
        tokens.append("".join(current))
    return tuple(token.casefold() for token in tokens)


def join_tokens(tokens: Iterable[str]) -> str:
    """Render normalized tokens as one key, joined by ``_`` (the inverse rendering used in Path sets)."""
    return "_".join(tokens)


def normalize_path(path: Path) -> Path:
    """Return ``path`` with every Key step replaced by its normalized, ``_``-joined name."""
    return tuple(key_step(join_tokens(normalize_name(step.key))) if step.kind == 0 else step for step in path)


def ref_target(ref: str) -> str | None:
    """The JSON Pointer of a local ``$ref`` (``#`` plus a pointer), or None for any other reference."""
    if not isinstance(ref, str) or not ref.startswith("#"):
        return None
    pointer = unquote(ref[1:])
    if pointer and not pointer.startswith("/"):
        return None
    try:
        split_pointer(pointer)
    except ValueError:
        return None
    return pointer


def _resolve_ref(model: SchemaModel, ref: str) -> SchemaNode | None:
    pointer = ref_target(ref)
    if pointer is None:
        return None
    try:
        return model.resolve(pointer)
    except (KeyError, ValueError):
        return None


def _branches(node: SchemaNode) -> Iterator[SchemaNode]:
    """The subschemas that stand at the same Path as ``node`` (applicators without a step)."""
    kw = node.kw
    for keyword in ("allOf", "anyOf", "oneOf"):
        for child in kw.get(keyword, ()):
            yield child
    for keyword in ("if", "then", "else"):
        child = kw.get(keyword)
        if child is not None:
            yield child


def _children(node: SchemaNode) -> Iterator[tuple[Path, SchemaNode]]:
    """The subschemas one step below ``node``, with the step, in canonical order."""
    kw = node.kw
    properties = kw.get("properties")
    if properties:
        for name in sorted(properties, key=codepoint_key):
            yield (key_step(name),), properties[name]
    patterns = kw.get("patternProperties")
    if patterns:
        for pattern in sorted(patterns, key=codepoint_key):
            yield (MAPVAL_STEP,), patterns[pattern]
    additional = kw.get("additionalProperties")
    if isinstance(additional, SchemaNode) and additional.boolean is None:
        yield (MAPVAL_STEP,), additional
    for child in kw.get("prefixItems", ()):
        yield (ELEM_STEP,), child
    items = kw.get("items")
    if items is not None:
        yield (ELEM_STEP,), items


def walk_paths(
    model: SchemaModel, ref_depth: int = DEFAULT_REF_DEPTH, max_paths: int | None = None
) -> Iterator[tuple[Path, SchemaNode]]:
    """Yield ``(Path, node)`` for every subschema the Path stepping reaches, ``$ref`` expanded up to ``ref_depth``.

    Every node yielded at a Path is one of the subschemas that constrain the
    values at that Path: the node itself and its ``allOf``/``anyOf``/``oneOf``/
    ``if``/``then``/``else`` branches, and the targets of followed ``$ref``.
    A reference is followed when its node's Path has at most ``ref_depth``
    steps; within one chain of references that adds no step, a target is
    expanded once. A node reached again at a Path it was already walked at
    (through another branch or reference) is not walked twice: its subtree
    yields the same Paths. With ``max_paths``, no new Path is opened once
    that many distinct Paths have been yielded (the ``nearest.max_paths``
    Resource_Parameter bounds the Path sets of recursive schemas, 25.1).
    Boolean schemas are yielded too. Paths are in canonical order, nodes at
    one Path in pre-order.
    """
    if isinstance(ref_depth, bool) or not isinstance(ref_depth, int) or ref_depth < 0:
        raise ValueError(f"ref_depth must be a non-negative int, got {ref_depth!r}")
    if max_paths is not None and (isinstance(max_paths, bool) or not isinstance(max_paths, int) or max_paths < 1):
        raise ValueError(f"max_paths must be a positive int or None, got {max_paths!r}")
    # (path, node, pointers expanded in the current step-less chain)
    stack: list[tuple[Path, SchemaNode, frozenset[str]]] = [((), model.root, frozenset())]
    walked: set[tuple[int, Path]] = set()  # (id(node), Path) pairs, for membership only
    opened: set[Path] = set()  # distinct Paths yielded, for the size cap only
    resolved: dict[str, SchemaNode | None] = {}  # $ref targets, resolved once per walk (resolve validates on every call)
    while stack:
        path, node, chain = stack.pop()
        marker = (id(node), path)
        if marker in walked:
            continue
        walked.add(marker)
        opened.add(path)
        yield path, node
        if node.boolean is not None:
            continue
        capped = max_paths is not None and len(opened) >= max_paths
        below: list[tuple[Path, SchemaNode, frozenset[str]]] = []
        ref = node.kw.get("$ref")
        if isinstance(ref, str) and len(path) <= ref_depth and not capped:
            pointer = ref_target(ref)
            if pointer is not None and pointer not in chain:
                if ref not in resolved:
                    resolved[ref] = _resolve_ref(model, ref)
                target = resolved[ref]
                if target is not None:
                    below.append((path, target, chain | {pointer}))
        for branch in _branches(node):
            below.append((path, branch, chain))
        for step, child in _children(node):
            if capped and path + step not in opened:
                continue
            below.append((path + step, child, frozenset()))
        stack.extend(reversed(below))


def normalized_paths(
    model: SchemaModel, ref_depth: int = DEFAULT_REF_DEPTH, max_paths: int | None = None
) -> frozenset[str]:
    """The normalized Path set of ``model`` (Requirement 25.1), as rendered Path strings."""
    found: set[str] = set()
    for path, _ in walk_paths(model, ref_depth, max_paths):
        found.add(render_path(normalize_path(path)))
    return frozenset(found)


def sorted_normalized_paths(model: SchemaModel, ref_depth: int = DEFAULT_REF_DEPTH) -> list[Path]:
    """The normalized Paths of ``model`` as Path tuples in canonical order, each once."""
    seen: dict[Path, None] = {}
    for path, _ in walk_paths(model, ref_depth):
        seen.setdefault(normalize_path(path), None)
    return sorted_paths(seen)
