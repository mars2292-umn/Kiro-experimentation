"""Leakage checks before a benchmark run (task 25.4; Requirements 30.7, 30.8, 30.11).

:func:`check_leakage` compares the training manifest of a Model_Bundle with
the Schema_Split and the Real_Datasets before any inference or Baseline
runs. It raises :class:`LeakageError` listing every offender when:

- the bundle has no training manifest (no ``schemas`` member with
  ``train``/``val`` lists), 30.11;
- the manifest lists a schema id that the loaded corpus does not hold, 30.11;
- the manifest lists a schema that belongs to a near-duplicate group of the
  held-out split (a held-out schema, or one sharing its group as the split's
  ``groups`` member records), 30.8;
- the manifest's ``unlabeled_collections`` names the Document_Collection of
  a Real_Dataset, 30.8.

Otherwise it returns the :class:`LeakageReport` of what was compared, so a
clean run can record it.
"""

from __future__ import annotations

from collections.abc import Collection, Mapping
from dataclasses import dataclass
from typing import Any

from jsinfer.bench.splits import SchemaSplit
from jsinfer.core.determinism import codepoint_key, sorted_codepoint
from jsinfer.core.jsonvalue import JSON

__all__ = ["LeakageError", "LeakageReport", "check_leakage", "held_out_groups"]


@dataclass(frozen=True)
class LeakageReport:
    """What a clean check compared: the manifest's schema and collection ids and the held-out groups."""

    schemas: tuple[str, ...]
    collections: tuple[str, ...]
    held_out_groups: tuple[str, ...]

    def to_json(self) -> dict[str, JSON]:
        return {
            "schemas": list(self.schemas),
            "collections": list(self.collections),
            "held_out_groups": list(self.held_out_groups),
        }


class LeakageError(Exception):
    """The benchmark must abort: ``missing_manifest``, or the offending ids per category (30.8, 30.11)."""

    def __init__(
        self,
        *,
        missing_manifest: str | None = None,
        unknown: Collection[str] = (),
        held_out: Mapping[str, str] | None = None,
        collections: Collection[str] = (),
    ) -> None:
        self.missing_manifest = missing_manifest
        self.unknown = tuple(sorted_codepoint(unknown))
        self.held_out = dict(sorted((held_out or {}).items(), key=lambda i: codepoint_key(i[0])))
        self.collections = tuple(sorted_codepoint(collections))
        super().__init__(str(self))

    def __str__(self) -> str:
        if self.missing_manifest is not None:
            return f"leakage check: {self.missing_manifest}"
        parts: list[str] = []
        if self.unknown:
            parts.append(f"training schemas absent from the corpus: {list(self.unknown)}")
        if self.held_out:
            listed = ", ".join(f"{sid} (group {group})" for sid, group in self.held_out.items())
            parts.append(f"training schemas in held-out near-duplicate groups: {listed}")
        if self.collections:
            parts.append(f"Real_Dataset collections used for training: {list(self.collections)}")
        return "leakage check: " + "; ".join(parts)


def held_out_groups(split: SchemaSplit) -> dict[str, str]:
    """Every corpus schema in a held-out near-duplicate group → its group name (the split's ``groups`` member)."""
    groups = split.extra.get("groups")
    mapping: dict[str, str] = {}
    if isinstance(groups, dict):
        mapping = {str(sid): str(group) for sid, group in groups.items()}
    held = set(split.held_out)
    held_group_names = {mapping.get(sid, sid) for sid in held}
    out = {sid: group for sid, group in mapping.items() if group in held_group_names}
    for sid in sorted(held, key=codepoint_key):
        out.setdefault(sid, mapping.get(sid, sid))
    return out


def _manifest_schemas(manifest: Mapping[str, Any]) -> list[str] | None:
    schemas = manifest.get("schemas")
    if not isinstance(schemas, Mapping):
        return None
    found: list[str] = []
    for split_name in ("train", "val"):
        ids = schemas.get(split_name)
        if not isinstance(ids, list) or not all(isinstance(sid, str) for sid in ids):
            return None
        found.extend(ids)
    return found


def check_leakage(
    manifest: Mapping[str, Any] | None,
    split: SchemaSplit,
    corpus_ids: Collection[str],
    real_collections: Collection[str] = (),
) -> LeakageReport:
    """Abort with :class:`LeakageError` on any overlap or unresolved id (see the module docstring)."""
    if manifest is None:
        raise LeakageError(missing_manifest="the Model_Bundle has no training manifest")
    schemas = _manifest_schemas(manifest)
    if schemas is None:
        raise LeakageError(missing_manifest="the Model_Bundle's manifest has no training schema lists")
    listed = manifest.get("unlabeled_collections", [])
    collections = [str(c.get("id", c)) if isinstance(c, Mapping) else str(c) for c in listed] if isinstance(listed, list) else []
    known = set(corpus_ids)
    unknown = [sid for sid in schemas if sid not in known]
    groups = held_out_groups(split)
    overlap = {sid: groups[sid] for sid in schemas if sid in groups}
    real = set(real_collections)
    leaked = [cid for cid in collections if cid in real]
    if unknown or overlap or leaked:
        raise LeakageError(unknown=unknown, held_out=overlap, collections=leaked)
    return LeakageReport(
        tuple(sorted_codepoint(schemas)),
        tuple(sorted_codepoint(collections)),
        tuple(sorted(set(groups.values()), key=codepoint_key)),
    )
