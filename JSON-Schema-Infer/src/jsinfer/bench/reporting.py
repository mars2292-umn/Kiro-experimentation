"""Results files and summary tables (task 30.3; Requirements 33.6, 34.6, 34.9, 34.10, 35.1, 35.2, 35.7, 35.8, 35.12).

Rows (35.8)
    A :class:`Row` is one (system, dataset, Dataset_Group, Run_Configuration,
    seed) result: its status (``ok``, or the failure kind ``failure``,
    ``timeout``, or ``unavailable``), the metric values (None for an
    undefined one), the runtime, peak memory, schema size, and description
    length of 35.7, and dataset facts such as ``n`` and the fold count.
    :func:`synthetic_rows` and :func:`real_rows` flatten the benchmark
    results; the Dataset_Group of a synthetic row is ``synthetic@<size>``
    and of a real row ``real``.

Files (35.8)
    :func:`write_csv` and :func:`write_json` write the same rows, so the
    two files hold identical values (:func:`read_csv` reads the CSV back
    into the same rows for checks). Every row names its system, dataset,
    Run_Configuration, and seed.

Aggregates (34.9, 35.1, 35.2, 35.12)
    :func:`aggregate` gives, per system, Dataset_Group, and metric, the
    mean over the datasets with a value, the contributing count, the 95%
    bootstrap interval (``bootstrap.summarize``), the undefined count, and
    the missing count per failure kind; rows that are not ``ok`` contribute
    to no mean. Paired comparisons between the reference system (the
    Inferrer) and every other system use only the datasets where both are
    ``ok`` and defined (``bootstrap.paired_test``).

Tables (33.6, 34.6, 34.10)
    :func:`markdown` and :func:`latex` render the aggregates: the
    Real_Dataset tables first, labeled the primary comparison, with the
    per-dataset facts (n, folds, exclusions, negative source) and the
    dataset exclusion counts per reason, then the synthetic tables per
    training size, then the paired comparisons. A system that was
    unavailable is kept in every table and marked; systems are shown by
    their label, which carries the ReCG reimplementation label (34.10).
"""

from __future__ import annotations

import csv
import json
import math
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Final

from jsinfer.bench.bootstrap import PairedTest, Summary, paired_test, summarize
from jsinfer.bench.real import RealResults
from jsinfer.bench.synthetic import SyntheticResults
from jsinfer.core.determinism import codepoint_key
from jsinfer.core.jsonvalue import JSON

__all__ = [
    "FIXED_COLUMNS",
    "MEASURE_COLUMNS",
    "REAL_GROUP",
    "SYNTHETIC_METRICS",
    "REAL_METRICS",
    "Aggregates",
    "Comparison",
    "MetricSummary",
    "Row",
    "aggregate",
    "latex",
    "markdown",
    "read_csv",
    "real_rows",
    "synthetic_rows",
    "synthetic_group",
    "write_csv",
    "write_json",
    "write_report",
]

FIXED_COLUMNS: Final = ("system", "label", "dataset", "group", "config", "seed", "status", "reason")
MEASURE_COLUMNS: Final = ("seconds", "peak_rss_mb", "schema_size", "dl_bits")
REAL_GROUP: Final = "real"
SYNTHETIC_METRICS: Final = ("recall", "rejection", "harmonic")
REAL_METRICS: Final = ("recall", "rejection", "harmonic")
STATUSES: Final = ("ok", "failure", "timeout", "unavailable")


def synthetic_group(size: int) -> str:
    return f"synthetic@{size}"


@dataclass(frozen=True)
class Row:
    """One result (see the module docstring)."""

    system: str
    label: str
    dataset: str
    group: str
    config: str
    seed: int
    status: str
    reason: str | None = None
    metrics: Mapping[str, float | None] = field(default_factory=dict)
    seconds: float | None = None
    peak_rss_mb: float | None = None
    schema_size: float | None = None
    dl_bits: float | None = None
    facts: Mapping[str, JSON] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if self.status not in STATUSES:
            raise ValueError(f"status must be one of {STATUSES}, got {self.status!r}")

    @property
    def ok(self) -> bool:
        return self.status == "ok"

    def flat(self) -> dict[str, JSON]:
        """The row as one flat mapping: fixed columns, metrics, measures, then facts."""
        out: dict[str, JSON] = {name: getattr(self, name) for name in FIXED_COLUMNS}
        out.update({name: self.metrics[name] for name in sorted(self.metrics, key=codepoint_key)})
        out.update({name: getattr(self, name) for name in MEASURE_COLUMNS})
        out.update({name: self.facts[name] for name in sorted(self.facts, key=codepoint_key)})
        return out


def synthetic_rows(results: SyntheticResults, labels: Mapping[str, str] | None = None, config: str = "default", *, unavailable: Iterable[str] = ()) -> list[Row]:
    """One :class:`Row` per synthetic result, plus an ``unavailable`` row per listed system and group (34.6)."""
    labels = labels or {}
    rows: list[Row] = []
    for r in results.results:
        metrics: dict[str, float | None] = {"recall": r.recall, "rejection": r.rejection, "harmonic": r.harmonic}
        for kind, value in r.judgments.items():
            metrics[f"judgment:{kind}"] = value
        if r.constraints is not None:
            for kind, value in r.constraints.precision.items():
                metrics[f"precision:{kind}"] = value
            for kind, value in r.constraints.recall.items():
                metrics[f"recall:{kind}"] = value
        if not r.ok:
            metrics = dict.fromkeys(metrics)
        rows.append(Row(
            r.system, labels.get(r.system, r.system), r.schema_id, synthetic_group(r.size), config, results.config.seed,
            "ok" if r.ok else str(r.missing), r.reason, metrics, r.seconds, r.peak_rss_mb, r.schema_size, r.dl_bits,
            {"relation": None if r.relation is None else (r.relation.value if hasattr(r.relation, "value") else "unsupported")},
        ))
    for system in sorted(set(unavailable), key=codepoint_key):
        for size in results.config.sizes:
            rows.append(Row(system, labels.get(system, system), "*", synthetic_group(size), config, results.config.seed, "unavailable", "unavailable", dict.fromkeys(SYNTHETIC_METRICS)))
    return rows


def real_rows(results: RealResults, labels: Mapping[str, str] | None = None, config: str = "default", *, unavailable: Iterable[str] = ()) -> list[Row]:
    """One :class:`Row` per real-data result with the dataset facts of 33.6, plus ``unavailable`` rows (34.6)."""
    labels = labels or {}
    rows: list[Row] = []
    for r in results.results:
        metrics: dict[str, float | None] = {"recall": r.recall, "rejection": r.rejection, "harmonic": r.harmonic}
        if not r.ok:
            metrics = dict.fromkeys(metrics)
        rows.append(Row(
            r.system, labels.get(r.system, r.system), r.dataset_id, REAL_GROUP, config, results.config.seed,
            "ok" if r.ok else str(r.missing), r.reason, metrics, r.seconds, r.peak_rss_mb, r.schema_size, r.dl_bits,
            {"n": r.n, "folds": r.fold_count, "excluded": r.excluded, "negatives_source": r.negatives_source, "negatives": r.negatives},
        ))
    for system in sorted(set(unavailable), key=codepoint_key):
        rows.append(Row(system, labels.get(system, system), "*", REAL_GROUP, config, results.config.seed, "unavailable", "unavailable", dict.fromkeys(REAL_METRICS)))
    return rows


# ---------------------------------------------------------------------------
# Files (35.8)
# ---------------------------------------------------------------------------


def _columns(rows: Sequence[Row]) -> list[str]:
    seen: dict[str, None] = {}
    for row in rows:
        for name in row.flat():
            seen.setdefault(name, None)
    fixed = [c for c in FIXED_COLUMNS if c in seen]
    rest = sorted((c for c in seen if c not in FIXED_COLUMNS and c not in MEASURE_COLUMNS), key=codepoint_key)
    return [*fixed, *rest, *(c for c in MEASURE_COLUMNS if c in seen)]


def _cell(value: JSON) -> str:
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float, str)):
        return str(value) if not isinstance(value, float) else repr(value)
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def write_csv(rows: Sequence[Row], path: Path) -> list[str]:
    """Write ``rows`` as CSV (one row each, the columns of :func:`_columns`); returns the column names."""
    columns = _columns(rows)
    with open(path, "w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(columns)
        for row in rows:
            flat = row.flat()
            writer.writerow([_cell(flat.get(name)) for name in columns])
    return columns


def write_json(rows: Sequence[Row], path: Path) -> None:
    """Write ``rows`` as a JSON array of flat objects over the same columns and values as :func:`write_csv`."""
    columns = _columns(rows)
    records = [{name: row.flat().get(name) for name in columns} for row in rows]
    path.write_text(json.dumps(records, ensure_ascii=False, indent=1, allow_nan=False), encoding="utf-8")


def _parse(text: str) -> JSON:
    if text == "":
        return None
    if text in ("true", "false"):
        return text == "true"
    try:
        return int(text)
    except ValueError:
        pass
    try:
        value = float(text)
    except ValueError:
        pass
    else:
        if math.isfinite(value):
            return value
    if text[:1] in "[{":
        try:
            return json.loads(text)
        except ValueError:
            pass
    return text


def read_csv(path: Path) -> list[dict[str, JSON]]:
    """Read a results CSV back into flat mappings with the JSON file's values (numbers, booleans, None)."""
    with open(path, encoding="utf-8", newline="") as handle:
        reader = csv.reader(handle)
        columns = next(reader)
        return [{name: _parse(cell) for name, cell in zip(columns, record)} for record in reader]


# ---------------------------------------------------------------------------
# Aggregates (34.9, 35.1, 35.2, 35.12)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class MetricSummary:
    system: str
    group: str
    metric: str
    summary: Summary
    missing: Mapping[str, int]  # failure kind -> count of datasets without a result

    @property
    def missing_total(self) -> int:
        return sum(self.missing.values())

    def to_json(self) -> dict[str, JSON]:
        return {"system": self.system, "group": self.group, "metric": self.metric, **self.summary.to_json(), "missing": dict(self.missing)}


@dataclass(frozen=True)
class Comparison:
    """``reference`` minus ``system`` over the datasets where both are ok and defined (34.9, 35.2)."""

    reference: str
    system: str
    group: str
    metric: str
    test: PairedTest | None
    n_pairs: int

    def to_json(self) -> dict[str, JSON]:
        return {"reference": self.reference, "system": self.system, "group": self.group, "metric": self.metric, "test": None if self.test is None else self.test.to_json(), "n_pairs": self.n_pairs}


@dataclass
class Aggregates:
    summaries: list[MetricSummary]
    comparisons: list[Comparison]
    reference: str
    labels: dict[str, str]
    unavailable: list[str]

    def summary(self, system: str, group: str, metric: str) -> MetricSummary | None:
        for s in self.summaries:
            if (s.system, s.group, s.metric) == (system, group, metric):
                return s
        return None

    @property
    def systems(self) -> list[str]:
        return sorted({s.system for s in self.summaries}, key=codepoint_key)

    @property
    def groups(self) -> list[str]:
        return sorted({s.group for s in self.summaries}, key=lambda g: (g != REAL_GROUP, _group_key(g)))

    def to_json(self) -> dict[str, JSON]:
        return {
            "reference": self.reference,
            "summaries": [s.to_json() for s in self.summaries],
            "comparisons": [c.to_json() for c in self.comparisons],
            "unavailable": list(self.unavailable),
        }


def _group_key(group: str) -> tuple[int, str]:
    if group.startswith("synthetic@"):
        try:
            return (int(group.split("@", 1)[1]), "")
        except ValueError:
            pass
    return (0, group)


def aggregate(rows: Sequence[Row], *, reference: str = "jsinfer", metrics: Sequence[str] | None = None, seed: int = 0, resamples: int | None = None) -> Aggregates:
    """Per-system, per-group, per-metric summaries and paired comparisons to ``reference`` (see the module docstring)."""
    names = list(metrics) if metrics is not None else sorted({m for row in rows for m in row.metrics}, key=codepoint_key)
    systems = sorted({row.system for row in rows}, key=codepoint_key)
    groups = sorted({row.group for row in rows}, key=lambda g: (g != REAL_GROUP, _group_key(g)))
    labels = {row.system: row.label for row in rows}
    unavailable = sorted({row.system for row in rows if row.status == "unavailable"}, key=codepoint_key)
    summaries: list[MetricSummary] = []
    comparisons: list[Comparison] = []
    by_cell: dict[tuple[str, str], list[Row]] = {}
    for row in rows:
        by_cell.setdefault((row.system, row.group), []).append(row)
    for system in systems:
        for group in groups:
            cell = by_cell.get((system, group), [])
            for metric in names:
                values = [row.metrics.get(metric) for row in cell if row.ok]
                missing: dict[str, int] = {}
                for row in cell:
                    if not row.ok:
                        missing[row.status] = missing.get(row.status, 0) + 1
                summaries.append(MetricSummary(system, group, metric, summarize(values, purpose=f"{system}:{group}:{metric}", seed=seed, resamples=resamples), dict(sorted(missing.items()))))
    ref_values: dict[tuple[str, str], dict[str, float]] = {}
    for group in groups:
        for metric in names:
            ref_values[(group, metric)] = {row.dataset: row.metrics[metric] for row in by_cell.get((reference, group), []) if row.ok and row.metrics.get(metric) is not None}  # type: ignore[misc]
    for system in systems:
        if system == reference:
            continue
        for group in groups:
            for metric in names:
                mine = {row.dataset: row.metrics[metric] for row in by_cell.get((system, group), []) if row.ok and row.metrics.get(metric) is not None}
                shared = sorted(set(mine) & set(ref_values[(group, metric)]), key=codepoint_key)
                test = paired_test([ref_values[(group, metric)][d] for d in shared], [mine[d] for d in shared], purpose=f"{reference}-{system}:{group}:{metric}", seed=seed, resamples=resamples)  # type: ignore[arg-type]
                comparisons.append(Comparison(reference, system, group, metric, test, len(shared)))
    return Aggregates(summaries, comparisons, reference, labels, unavailable)


# ---------------------------------------------------------------------------
# Tables (33.6, 34.6, 34.10)
# ---------------------------------------------------------------------------


def _fmt(value: float | None, digits: int = 3) -> str:
    return "—" if value is None else f"{value:.{digits}f}"


def _summary_cell(s: MetricSummary) -> str:
    if s.summary.mean is None:
        text = "n/a"
    else:
        ci = s.summary.ci
        text = _fmt(s.summary.mean) + ("" if ci is None else f" [{_fmt(ci.low)}, {_fmt(ci.high)}]")
    notes = []
    if s.summary.count:
        notes.append(f"n={s.summary.count}")
    if s.summary.undefined:
        notes.append(f"undef={s.summary.undefined}")
    if s.missing_total:
        notes.append("missing=" + "+".join(f"{k}:{v}" for k, v in s.missing.items()))
    return text + (f" ({', '.join(notes)})" if notes else "")


def _system_name(agg: Aggregates, system: str) -> str:
    name = agg.labels.get(system, system)
    return f"{name} (unavailable)" if system in agg.unavailable else name


def _tables(agg: Aggregates, rows: Sequence[Row], real: RealResults | None, metrics: Sequence[str]) -> list[tuple[str, list[str], list[list[str]]]]:
    """``(title, header, body)`` tables in report order: real first and labeled primary, then synthetic, then comparisons."""
    tables: list[tuple[str, list[str], list[list[str]]]] = []
    for group in agg.groups:
        shown = [m for m in metrics if any(s.group == group and s.metric == m for s in agg.summaries)]
        title = "Real datasets (primary comparison)" if group == REAL_GROUP else f"Synthetic benchmark, training size {group.split('@', 1)[-1]}"
        header = ["System", *shown]
        body = []
        for system in agg.systems:
            cells = [_system_name(agg, system)]
            for metric in shown:
                s = agg.summary(system, group, metric)
                cells.append("n/a" if s is None else _summary_cell(s))
            body.append(cells)
        tables.append((title, header, body))
        if group == REAL_GROUP:
            facts: dict[str, Mapping[str, JSON]] = {}
            for row in rows:
                if row.group == REAL_GROUP and row.dataset != "*" and row.dataset not in facts:
                    facts[row.dataset] = row.facts
            if facts:
                body = [[d, str(f.get("n")), str(f.get("folds")), str(f.get("excluded")), str(f.get("negatives_source"))] for d, f in sorted(facts.items(), key=lambda i: codepoint_key(i[0]))]
                tables.append(("Real datasets: documents, folds, exclusions, negatives", ["Dataset", "n", "folds", "excluded documents", "negatives"], body))
            if real is not None and real.dataset_exclusions:
                counts = real.exclusion_reasons()
                tables.append(("Real datasets excluded", ["Reason", "datasets"], [[reason, str(count)] for reason, count in counts.items()]))
    if agg.comparisons:
        body = []
        for c in agg.comparisons:
            if c.metric not in metrics:
                continue
            if c.test is None:
                body.append([_system_name(agg, c.system), c.group, c.metric, "not computable", "", "", str(c.n_pairs)])
            else:
                body.append([_system_name(agg, c.system), c.group, c.metric, _fmt(c.test.mean_difference), f"[{_fmt(c.test.ci.low)}, {_fmt(c.test.ci.high)}]", _fmt(c.test.p_value), str(c.n_pairs)])
        if body:
            tables.append((f"Paired comparisons: {agg.labels.get(agg.reference, agg.reference)} minus system", ["System", "group", "metric", "Δ", "95% CI", "p", "pairs"], body))
    return tables


def markdown(agg: Aggregates, rows: Sequence[Row], real: RealResults | None = None, metrics: Sequence[str] = SYNTHETIC_METRICS) -> str:
    out: list[str] = ["# Benchmark summary", ""]
    for title, header, body in _tables(agg, rows, real, metrics):
        out.append(f"## {title}")
        out.append("")
        out.append("| " + " | ".join(header) + " |")
        out.append("|" + "|".join("---" for _ in header) + "|")
        for cells in body:
            out.append("| " + " | ".join(c.replace("|", "\\|") for c in cells) + " |")
        out.append("")
    return "\n".join(out)


def _tex(text: str) -> str:
    return text.replace("\\", "\\textbackslash{}").replace("&", "\\&").replace("%", "\\%").replace("_", "\\_").replace("#", "\\#").replace("Δ", "$\\Delta$").replace("—", "--")


def latex(agg: Aggregates, rows: Sequence[Row], real: RealResults | None = None, metrics: Sequence[str] = SYNTHETIC_METRICS) -> str:
    out: list[str] = []
    for title, header, body in _tables(agg, rows, real, metrics):
        out.append("\\begin{table}[ht]")
        out.append("\\centering")
        out.append(f"\\caption{{{_tex(title)}}}")
        out.append("\\begin{tabular}{" + "l" * len(header) + "}")
        out.append("\\toprule")
        out.append(" & ".join(_tex(h) for h in header) + " \\\\")
        out.append("\\midrule")
        for cells in body:
            out.append(" & ".join(_tex(c) for c in cells) + " \\\\")
        out.append("\\bottomrule")
        out.append("\\end{tabular}")
        out.append("\\end{table}")
        out.append("")
    return "\n".join(out)


def write_report(rows: Sequence[Row], out_dir: Path, *, real: RealResults | None = None, reference: str = "jsinfer", metrics: Sequence[str] = SYNTHETIC_METRICS, seed: int = 0, resamples: int | None = None, extra: Mapping[str, JSON] | None = None) -> Aggregates:
    """Write ``results.csv``/``results.json``, ``aggregates.json``/``aggregates.csv``, ``summary.md``, and ``summary.tex`` (35.8)."""
    out_dir.mkdir(parents=True, exist_ok=True)
    write_csv(rows, out_dir / "results.csv")
    write_json(rows, out_dir / "results.json")
    agg = aggregate(rows, reference=reference, seed=seed, resamples=resamples)
    document: dict[str, Any] = agg.to_json()
    if extra:
        document["extra"] = dict(extra)
    (out_dir / "aggregates.json").write_text(json.dumps(document, ensure_ascii=False, indent=1, allow_nan=False), encoding="utf-8")
    with open(out_dir / "aggregates.csv", "w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(["system", "group", "metric", "mean", "count", "undefined", "ci_low", "ci_high", "missing"])
        for s in agg.summaries:
            ci = s.summary.ci
            writer.writerow([s.system, s.group, s.metric, _cell(s.summary.mean), s.summary.count, s.summary.undefined, _cell(None if ci is None else ci.low), _cell(None if ci is None else ci.high), _cell(dict(s.missing))])
    (out_dir / "summary.md").write_text(markdown(agg, rows, real, metrics), encoding="utf-8")
    (out_dir / "summary.tex").write_text(latex(agg, rows, real, metrics), encoding="utf-8")
    return agg
