"""Provenance bookkeeping for tests that edit an inferred Schema_Model before the soundness check.

Several full-run tests (Properties 3, 7, and 9) inject over-tight Constraints
into the Structure_Analyzer's model to drive the soundness loop. Provenance
finalization (``pipeline.infer.finalize_provenance``, task 17.1) requires the
emitted Constraints and the Provenance_Records to correspond one to one
(26.1) and rejects anything else as an internal error, as it should for a
real component. :func:`complete_records` restores that invariant for an
edited model the way a component would: every emitted Constraint without a
record gets a placeholder record, and records of Constraints the edit took
out of the model are dropped. Finalization then recomputes every Support.
"""

from __future__ import annotations

from typing import Final

from jsinfer.core.provenance import ProvenanceRecord, Support
from jsinfer.core.schema_model import SchemaModel
from jsinfer.io.emit import SchemaEmitter

__all__ = ["PLACEHOLDER_TECHNIQUE", "complete_records"]

# The producer named by placeholder records: a registered, implemented Technique id.
PLACEHOLDER_TECHNIQUE: Final = "decide.mdl"

_EMITTER: Final = SchemaEmitter()


def complete_records(model: SchemaModel, tag: str = "injected") -> int:
    """Give each emitted Constraint of ``model`` without a record a placeholder; drop records of no Constraint.

    Placeholders carry the Laplace Confidence of an empty Support and
    ``details[tag] = True``. Returns the number of records added or dropped.
    """
    _, pointers = _EMITTER.layout(model)
    changed = 0
    for cid in pointers:
        if cid not in model.provenance:
            model.provenance[cid] = ProvenanceRecord.laplace(PLACEHOLDER_TECHNIQUE, Support(0, 0), details={tag: True})
            changed += 1
    for cid in [cid for cid in model.provenance if cid not in pointers]:
        del model.provenance[cid]
        changed += 1
    return changed
