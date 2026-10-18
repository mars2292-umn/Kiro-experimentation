"""Unit tests for core/report.py (task 2.6): the Inference_Report builder, stage timer, status, and JSON form."""

from __future__ import annotations

import copy
import json
import math
import pickle
import random
import time
from collections.abc import Callable
from typing import Any

import numpy as np
import pytest

import jsinfer.core.report as report_module
from jsinfer.core.paths import ELEM_STEP, key_step
from jsinfer.core.provenance import (
    MDL_COMPARISON,
    REMOVED,
    CandidateScore,
    Cutoff,
    FallbackInfo,
    RelaxationRecord,
    Resolution,
)
from jsinfer.core.report import (
    REJECTED_CATEGORIES,
    SECTIONS,
    ChangePointCriterion,
    DecisionEntry,
    ErrorInfo,
    FlaggedItem,
    FlagScore,
    InferenceReport,
    IngestError,
    NearestInfo,
    NearestResult,
    Neighbor,
    OrderingKey,
    RejectedCandidate,
    RemovedConstraint,
    ReportFormatError,
    ResourceUse,
    RunInfo,
    Segment,
    SoundnessSummary,
    Status,
    Substitution,
    VersionsInfo,
)

# ---------------------------------------------------------------------------
# Fixtures and a fully populated report
# ---------------------------------------------------------------------------


class FakeClock:
    """A stage clock the test advances by hand, so timings are exact."""

    def __init__(self) -> None:
        self.now = 100.0

    def __call__(self) -> float:
        return self.now


@pytest.fixture
def clock(monkeypatch: pytest.MonkeyPatch) -> FakeClock:
    fake = FakeClock()
    monkeypatch.setattr(report_module, "_clock", fake)
    return fake


def run_stage(report: InferenceReport, clock: FakeClock, component: str, seconds: float) -> None:
    with report.stage(component):
        clock.now += seconds


MDL_RESOLUTION = Resolution(
    "mdl",
    0,
    (
        CandidateScore(
            "decide.mdl", "record", 10.5, model_bits=2.0, data_bits=8.5, exception_bits=0.0, n_rejected=0, n_scored=4
        ),
        CandidateScore(
            "decide.mdl", "map", 12.25, model_bits=6.0, data_bits=6.25, exception_bits=0.0, n_rejected=0, n_scored=4
        ),
    ),
)
FALLBACK_RESOLUTION = Resolution("fallback", 0, (CandidateScore(None, "omit", None),))
SUB_THRESHOLD = Substitution(
    "cooccur.min_support", FallbackInfo("rules.cooccurrence", "missing-threshold"), "missing", "no rules"
)
SUB_MODEL = Substitution("dm.record_or_map", FallbackInfo("decide.dm_record_map", "missing-model"), "missing", "decide.mdl")
REMOVAL = RemovedConstraint("/properties/age/maximum", RelaxationRecord({"maximum": 10}, REMOVED, (7,)))


def run_info(*, seed: int = 0, workers: int = 8, started: str | None = "2026-01-01T00:00:00Z", finished: str | None = None) -> RunInfo:
    return RunInfo(
        config={
            "resources": {"soundness.max_passes": 5},
            "run": {"model_bundle": "bundles/default", "seed": seed, "workers": workers},
        },
        seed=seed,
        bundle_id="bundle-1",
        jsinfer_version="0.1.0",
        python_version="3.11.9",
        packages={"numpy": "2.4.6", "jsonschema-rs": "0.58.6"},
        started=started,
        finished=finished,
    )


def recordings(clock: FakeClock, *, workers: int = 8, scale: float = 1.0) -> list[Callable[[InferenceReport], None]]:
    """Every order-insensitive recording of the full report, one callable each."""
    return [
        lambda r: r.record_threshold("lsh.keyset_jaccard", 0.4, "bundle-1"),
        lambda r: r.record_threshold("dbscan.min_samples", 5, "bundle-1"),
        lambda r: r.record_threshold("dbscan.min_samples", 5, "bundle-1"),  # a repeated read
        lambda r: r.record_resource("soundness.max_passes", 5, "default"),
        lambda r: r.record_resource("workers", workers, "flag"),
        lambda r: r.record_substitution(SUB_MODEL),
        lambda r: r.record_substitution(SUB_THRESHOLD),
        lambda r: r.record_substitution(SUB_THRESHOLD),  # recorded at a second Decision_Point
        lambda r: r.record_flagged_item(FlaggedItem(3, "$", (FlagScore("robust.isolation_forest", 0.71, 0.6),))),
        lambda r: r.record_flagged_item(
            FlaggedItem(1, "$.age", (FlagScore("robust.mdl_exceptions", 40.0, 52.5),), ("/properties/age/maximum",))
        ),
        lambda r: r.record_flagged_item(
            FlaggedItem(
                1, "$", (FlagScore("robust.lof", 1.9, 1.5), FlagScore("robust.isolation_forest", 0.8, 0.6))
            )
        ),
        lambda r: r.record_decision(
            DecisionEntry("record_or_map", "$.user", MDL_RESOLUTION, Cutoff(MDL_COMPARISON, scores=MDL_RESOLUTION.candidates))
        ),
        lambda r: r.record_decision(
            DecisionEntry("record_or_map", "$", MDL_RESOLUTION, Cutoff(MDL_COMPARISON, scores=MDL_RESOLUTION.candidates))
        ),
        lambda r: r.record_decision(
            DecisionEntry(
                "bound", "$.age", FALLBACK_RESOLUTION, fallback=FallbackInfo("values.bounds", "disabled"),
                details={"side": "maximum"},
            )
        ),
        lambda r: r.record_rejected(
            RejectedCandidate("formats", "$.email", {"format": "email"}, "values.format", {"rejected": 2})
        ),
        lambda r: r.record_rejected(
            RejectedCandidate(
                "rules", "$", {"antecedent": "a", "consequent": "b"}, "rules.cooccurrence", {"support": 1, "threshold": 2}
            )
        ),
        lambda r: r.record_rejected(RejectedCandidate("formats", "$.a", {"format": "date"}, "values.format", {"rejected": 1})),
        lambda r: r.record_soundness(SoundnessSummary(0, 5, 2, 1, 1, 5, True, removed=(REMOVAL,), version=1)),
        lambda r: r.record_soundness(SoundnessSummary(1, 4, 1, 1, 0, 2, False, version=0)),
        lambda r: run_stage(r, clock, "Structure_Analyzer", 1.0 * scale),
        lambda r: run_stage(r, clock, "Structure_Analyzer", 0.5 * scale),
        lambda r: run_stage(r, clock, "Ingestor", 0.5 * scale),
        lambda r: run_stage(r, clock, "Flatten", 0.25 * scale),
    ]


VERSIONS = VersionsInfo(
    True,
    OrderingKey("selected", "$.ts"),
    ChangePointCriterion("MDL", 1, 120.5, 150.25),
    (Segment(0, "2020-01-01", 4, "2020-01-05"), Segment(5, "2020-01-06", 9, "2020-01-10")),
)
NEAREST = NearestInfo(
    "searched",
    (
        NearestResult(version=1),
        NearestResult(
            (Neighbor("schemastore:b", 0.5), Neighbor("schemastore:a", 0.5), Neighbor("schemastore:c", 0.75)),
            shared=("$.name", "$.id", "$.tags[*]"),
            only_inferred=("$.age",),
            version=0,
        ),
    ),
)


def full_report(
    clock: FakeClock,
    *,
    shuffle: int | None = None,
    seed: int = 0,
    workers: int = 8,
    scale: float = 1.0,
    started: str | None = "2026-01-01T00:00:00Z",
    finished: str | None = None,
) -> InferenceReport:
    """A report with every section populated; ``shuffle`` permutes the order-insensitive recordings."""
    report = InferenceReport()
    report.set_run(run_info(seed=seed, workers=workers, started=started, finished=finished))
    # Ingestion order is the Ingestor's (input) order, which the report keeps.
    report.record_ingest_error(IngestError("data/b.jsonl", "invalid JSON: Expecting value", 3))
    report.record_ingest_error(IngestError("data/a.json", "unreadable file: permission denied"))
    actions = recordings(clock, workers=workers, scale=scale)
    if shuffle is not None:
        random.Random(shuffle).shuffle(actions)
    for action in actions:
        action(report)
    report.set_versions(VERSIONS)
    report.set_nearest(NEAREST)
    return report


def loaded(report: InferenceReport) -> InferenceReport:
    """The report read back from its written bytes."""
    return InferenceReport.from_json(json.loads(report.to_bytes()))


# ---------------------------------------------------------------------------
# Whole-report form, round trip, and order-insensitivity
# ---------------------------------------------------------------------------


def test_empty_report_has_every_section() -> None:
    report = InferenceReport()
    data = report.to_json()
    assert list(data) == list(SECTIONS) == sorted(SECTIONS)
    assert len(SECTIONS) == 13
    assert data == {
        "decisions": [],
        "ingestion": {"errors": [], "skipped": 0},
        "nearest": {},
        "outliers": {"abandoned": False, "discarded": 0, "items": []},
        "rejected": {category: [] for category in REJECTED_CATEGORIES},
        "resources": {},
        "run": {},
        "soundness": [],
        "status": {"state": "ok"},
        "substitutions": [],
        "thresholds": {},
        "timings": {},
        "versions": {},
    }
    assert report.ok and report.status == Status() and report.run is None
    assert report.to_bytes().endswith(b"}\n")
    assert loaded(report) == report


def test_full_report_sections(clock: FakeClock) -> None:
    data = full_report(clock).to_json()
    assert data["ingestion"] == {
        "errors": [
            {"line": 3, "reason": "invalid JSON: Expecting value", "source": "data/b.jsonl"},
            {"reason": "unreadable file: permission denied", "source": "data/a.json"},
        ],
        "skipped": 2,
    }
    assert data["thresholds"] == {
        "dbscan.min_samples": {"bundle_id": "bundle-1", "value": 5},
        "lsh.keyset_jaccard": {"bundle_id": "bundle-1", "value": 0.4},
    }
    assert list(data["thresholds"]) == ["dbscan.min_samples", "lsh.keyset_jaccard"]
    assert data["resources"] == {
        "soundness.max_passes": {"origin": "default", "value": 5},
        "workers": {"origin": "flag", "value": 8},
    }
    assert data["substitutions"] == [
        {
            "applied": "no rules",
            "artifact": "cooccur.min_support",
            "cause": "missing",
            "fallback": {"reason": "missing-threshold", "technique": "rules.cooccurrence"},
        },
        {
            "applied": "decide.mdl",
            "artifact": "dm.record_or_map",
            "cause": "missing",
            "fallback": {"reason": "missing-model", "technique": "decide.dm_record_map"},
        },
    ]
    assert data["outliers"] == {
        "abandoned": False,
        "discarded": 0,
        "items": [
            {
                "document": 1,
                "effects": [],
                "flags": [
                    {"compared": 0.6, "score": 0.8, "technique": "robust.isolation_forest"},
                    {"compared": 1.5, "score": 1.9, "technique": "robust.lof"},
                ],
                "path": "$",
            },
            {
                "document": 1,
                "effects": ["/properties/age/maximum"],
                "flags": [{"compared": 52.5, "score": 40.0, "technique": "robust.mdl_exceptions"}],
                "path": "$.age",
            },
            {
                "document": 3,
                "effects": [],
                "flags": [{"compared": 0.6, "score": 0.71, "technique": "robust.isolation_forest"}],
                "path": "$",
            },
        ],
    }
    assert [(d["path"], d["kind"]) for d in data["decisions"]] == [
        ("$", "record_or_map"),
        ("$.age", "bound"),
        ("$.user", "record_or_map"),
    ]
    assert data["decisions"][1] == {
        "details": {"side": "maximum"},
        "fallback": {"reason": "disabled", "technique": "values.bounds"},
        "kind": "bound",
        "path": "$.age",
        "resolution": {"candidates": [{"outcome": "omit"}], "method": "fallback", "selected": 0},
    }
    assert data["decisions"][0]["cutoff"]["source"] == MDL_COMPARISON
    assert data["decisions"][0]["resolution"] == MDL_RESOLUTION.to_json()
    assert list(data["rejected"]) == list(REJECTED_CATEGORIES)
    assert data["rejected"]["formats"] == [
        {"candidate": {"format": "date"}, "details": {"rejected": 1}, "path": "$.a", "technique": "values.format"},
        {"candidate": {"format": "email"}, "details": {"rejected": 2}, "path": "$.email", "technique": "values.format"},
    ]
    assert [entry["path"] for entry in data["rejected"]["rules"]] == ["$"]
    assert [summary["version"] for summary in data["soundness"]] == [0, 1]
    assert data["soundness"][1] == {
        "failed_first_pass": 2,
        "flagged_documents": 0,
        "pass_limit_reached": True,
        "passes": 5,
        "relaxations": 1,
        "removals": 1,
        "removed": [
            {"change": {"before": {"maximum": 10}, "documents": [7], "removed": True}, "pointer": "/properties/age/maximum"}
        ],
        "validated": 5,
        "version": 1,
    }
    assert data["versions"] == {
        "applied": True,
        "criterion": {"change_points": 1, "name": "MDL", "selected": 120.5, "unsegmented": 150.25},
        "ordering_key": {"path": "$.ts", "source": "selected"},
        "segments": [
            {"first_document": 0, "first_key": "2020-01-01", "last_document": 4, "last_key": "2020-01-05"},
            {"first_document": 5, "first_key": "2020-01-06", "last_document": 9, "last_key": "2020-01-10"},
        ],
    }
    assert data["nearest"] == {
        "results": [
            {
                "found": True,
                "neighbors": [
                    {"similarity": 0.75, "source": "schemastore:c"},
                    {"similarity": 0.5, "source": "schemastore:a"},
                    {"similarity": 0.5, "source": "schemastore:b"},
                ],
                "only_inferred": ["$.age"],
                "only_known": [],
                "shared": ["$.id", "$.name", "$.tags[*]"],
                "version": 0,
            },
            {"found": False, "neighbors": [], "only_inferred": [], "only_known": [], "shared": [], "version": 1},
        ],
        "status": "searched",
    }
    assert data["timings"] == {"Flatten": 0.25, "Ingestor": 0.5, "Structure_Analyzer": 1.5}
    assert list(data["timings"]) == ["Flatten", "Ingestor", "Structure_Analyzer"]
    assert data["status"] == {"state": "ok"}
    assert data["run"] == {
        "bundle_id": "bundle-1",
        "config": {
            "resources": {"soundness.max_passes": 5},
            "run": {"model_bundle": "bundles/default", "seed": 0, "workers": 8},
        },
        "jsinfer_version": "0.1.0",
        "packages": {"jsonschema-rs": "0.58.6", "numpy": "2.4.6"},
        "python_version": "3.11.9",
        "seed": 0,
        "started": "2026-01-01T00:00:00Z",
    }


def test_full_report_round_trip(clock: FakeClock) -> None:
    report = full_report(clock)
    again = loaded(report)
    assert again == report
    assert again.to_bytes() == report.to_bytes()
    assert InferenceReport.from_json(report.to_json()) == report
    # The accessors of the loaded report see the same records.
    assert again.flagged_items == report.flagged_items
    assert again.substitutions == report.substitutions
    assert again.soundness == report.soundness
    assert again.versions == report.versions and again.nearest == report.nearest
    assert again.run == report.run and again.timings == report.timings


def test_written_bytes_are_canonical(clock: FakeClock) -> None:
    data = full_report(clock).to_bytes()
    text = data.decode("utf-8")
    assert text == json.dumps(json.loads(text), indent=2, ensure_ascii=False, sort_keys=True) + "\n"


@pytest.mark.parametrize("shuffle", range(6))
def test_recording_order_does_not_change_bytes(clock: FakeClock, shuffle: int) -> None:
    assert full_report(clock, shuffle=shuffle).to_bytes() == full_report(clock).to_bytes()


def test_report_equality_and_hashing(clock: FakeClock) -> None:
    report = full_report(clock)
    other = full_report(clock)
    assert report == other and report is not other
    other.record_decision(DecisionEntry("tuple_or_list", "$.tags", MDL_RESOLUTION))
    assert report != other
    with pytest.raises(TypeError):
        hash(report)
    assert report != "report"


def test_report_pickles(clock: FakeClock) -> None:
    report = full_report(clock)
    report.fail("Soundness", ValueError("boom"))
    again = pickle.loads(pickle.dumps(report))
    assert again == report and not again.ok
    assert copy.deepcopy(report) == report


# ---------------------------------------------------------------------------
# run
# ---------------------------------------------------------------------------


def test_run_info_normalizes_config_and_packages() -> None:
    info = RunInfo(
        config={"run": {"seed": 1.0, "b": True}, "a": [1.0, (2, 3)]},
        seed=np.int64(1),
        bundle_id="b",
        jsinfer_version="0.1.0",
        python_version="3.12.1",
        packages={"z": "1", "a": "2"},
    )
    assert info.config == {"a": [1, [2, 3]], "run": {"b": True, "seed": 1}}
    assert list(info.config["run"]) == ["b", "seed"]  # type: ignore[call-overload]
    assert list(info.packages) == ["a", "z"]
    assert type(info.seed) is int
    assert "started" not in info.to_json() and "finished" not in info.to_json()
    assert RunInfo.from_json(info.to_json()) == info


@pytest.mark.parametrize(
    ("change", "error"),
    [
        ({"seed": -1}, ValueError),
        ({"seed": True}, TypeError),
        ({"bundle_id": ""}, ValueError),
        ({"config": [1]}, TypeError),
        ({"config": {"x": float("nan")}}, ValueError),
        ({"config": {1: 2}}, TypeError),
        ({"packages": {"numpy": 2}}, TypeError),
        ({"started": ""}, ValueError),
    ],
)
def test_run_info_rejects_invalid_fields(change: dict[str, Any], error: type[Exception]) -> None:
    fields: dict[str, Any] = {
        "config": {}, "seed": 0, "bundle_id": "b", "jsinfer_version": "v", "python_version": "p",
    }
    fields.update(change)
    with pytest.raises(error):
        RunInfo(**fields)


def test_run_is_recorded_once() -> None:
    report = InferenceReport()
    report.set_run(run_info())
    with pytest.raises(ValueError):
        report.set_run(run_info())
    with pytest.raises(TypeError):
        InferenceReport().set_run({"seed": 0})  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# ingestion, thresholds, resources, substitutions
# ---------------------------------------------------------------------------


def test_ingest_errors_keep_ingestion_order_and_count() -> None:
    report = InferenceReport()
    errors = [IngestError("z.jsonl", "bad", 9), IngestError("a.jsonl", "bad", 2), IngestError("dir/x.json", "unreadable")]
    for error in errors:
        report.record_ingest_error(error)
    assert report.ingest_errors == tuple(errors)
    assert report.skipped == 3
    assert report.to_json()["ingestion"]["skipped"] == 3  # type: ignore[index]
    assert errors[2].to_json() == {"reason": "unreadable", "source": "dir/x.json"}


@pytest.mark.parametrize(
    ("args", "error"),
    [
        (("", "bad", 1), ValueError),
        (("a.json", "", 1), ValueError),
        (("a.json", "bad", 0), ValueError),
        (("a.json", "bad", 1.0), TypeError),
        (("a.json", "bad", True), TypeError),
        (("a\ud800.json", "bad", 1), ValueError),
    ],
)
def test_ingest_error_rejects_invalid_fields(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        IngestError(*args)


def test_thresholds_keep_value_type_and_reject_conflicts() -> None:
    report = InferenceReport()
    report.record_threshold("dbscan.min_samples", np.int32(5), "bundle-1")
    report.record_threshold("dbscan.eps", np.float64(0.25), "bundle-1")
    report.record_threshold("dbscan.min_samples", 5.0, "bundle-1")  # the same value read again
    values = {name: use.value for name, use in report.thresholds.items()}
    assert values == {"dbscan.eps": 0.25, "dbscan.min_samples": 5}
    assert type(values["dbscan.min_samples"]) is int and type(values["dbscan.eps"]) is float
    with pytest.raises(ValueError, match="read as"):
        report.record_threshold("dbscan.min_samples", 6, "bundle-1")
    with pytest.raises(ValueError, match="read as"):
        report.record_threshold("dbscan.min_samples", 5, "bundle-2")
    for bad, error in [(float("nan"), ValueError), (math.inf, ValueError), (True, TypeError), ("5", TypeError)]:
        with pytest.raises(error):
            report.record_threshold("x", bad, "bundle-1")
    with pytest.raises(ValueError):
        report.record_threshold("", 1, "bundle-1")


def test_resources_record_value_and_origin() -> None:
    report = InferenceReport()
    report.record_resource("soundness.max_passes", 5, "default")
    report.record_resource("ingest.chunk_size", 500, "file")
    report.record_resource("soundness.max_passes", 5, "default")
    assert list(report.resources) == ["ingest.chunk_size", "soundness.max_passes"]
    assert report.resources["ingest.chunk_size"].user_supplied
    assert not report.resources["soundness.max_passes"].user_supplied
    assert ResourceUse(2, "flag").user_supplied
    with pytest.raises(ValueError, match="read as"):
        report.record_resource("soundness.max_passes", 5, "flag")
    with pytest.raises(ValueError):
        report.record_resource("x", 1, "user")  # type: ignore[arg-type]


def test_substitutions_are_unique_and_sorted() -> None:
    report = InferenceReport()
    for substitution in (SUB_MODEL, SUB_THRESHOLD, SUB_MODEL):
        report.record_substitution(substitution)
    assert report.substitutions == (SUB_THRESHOLD, SUB_MODEL)
    invalid = Substitution("lof.score", FallbackInfo("robust.lof", "missing-threshold"), "invalid", "not flagged")
    assert invalid.to_json()["cause"] == "invalid"


@pytest.mark.parametrize(
    ("args", "error"),
    [
        (("x", FallbackInfo("robust.lof", "disabled"), "missing", "none"), ValueError),
        (("x", FallbackInfo("robust.lof", "missing-threshold"), "absent", "none"), ValueError),
        (("x", ("robust.lof", "missing-threshold"), "missing", "none"), TypeError),
        (("", FallbackInfo("robust.lof", "missing-threshold"), "missing", "none"), ValueError),
        (("x", FallbackInfo("robust.lof", "missing-threshold"), "missing", ""), ValueError),
    ],
)
def test_substitution_rejects_invalid_fields(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        Substitution(*args)


# ---------------------------------------------------------------------------
# outliers
# ---------------------------------------------------------------------------


def test_flagged_item_normalizes_paths_flags_and_effects() -> None:
    item = FlaggedItem(
        np.int64(2),
        (key_step("tags"), ELEM_STEP),
        [FlagScore("robust.lof", 2, 1), FlagScore("robust.isolation_forest", -0.0, 0.5)],
        ["b", "a", "b"],
    )
    assert item.path == "$.tags[*]" and type(item.document) is int
    assert [flag.technique for flag in item.flags] == ["robust.isolation_forest", "robust.lof"]
    assert item.flags[0].score == 0.0 and math.copysign(1.0, item.flags[0].score) == 1.0
    assert item.effects == ("a", "b")
    assert FlaggedItem(0, '$["tags"]', (FlagScore("robust.lof", 1, 0),)).path == "$.tags"
    assert FlaggedItem.from_json(item.to_json()) == item


@pytest.mark.parametrize(
    ("args", "error"),
    [
        ((0, "$", ()), ValueError),
        ((0, "$", (FlagScore("robust.lof", 1, 0), FlagScore("robust.lof", 2, 0))), ValueError),
        ((0, "user", (FlagScore("robust.lof", 1, 0),)), ValueError),
        ((0, 5, (FlagScore("robust.lof", 1, 0),)), TypeError),
        ((-1, "$", (FlagScore("robust.lof", 1, 0),)), ValueError),
        ((0, "$", (FlagScore("robust.lof", 1, 0),), "effect"), TypeError),
    ],
)
def test_flagged_item_rejects_invalid_fields(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        FlaggedItem(*args)


@pytest.mark.parametrize(
    ("args", "error"),
    [
        (("fallback:bound", 1.0, 0.0), ValueError),
        (("baseline:type", 1.0, 0.0), ValueError),
        (("robust.lof", float("nan"), 0.0), ValueError),
        (("robust.lof", 1.0, True), TypeError),
    ],
)
def test_flag_score_rejects_invalid_fields(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        FlagScore(*args)


def test_one_entry_per_flagged_item() -> None:
    report = InferenceReport()
    report.record_flagged_item(FlaggedItem(4, "$.a", (FlagScore("robust.mdl_exceptions", 1, 2),)))
    with pytest.raises(ValueError, match="already recorded"):
        report.record_flagged_item(FlaggedItem(4, '$["a"]', (FlagScore("robust.lof", 1, 2),)))
    report.record_flagged_item(FlaggedItem(4, "$", (FlagScore("robust.lof", 1, 2),)))
    assert [(item.document, item.path) for item in report.flagged_items] == [(4, "$"), (4, "$.a")]


def test_abandoning_outlier_exclusion() -> None:
    report = InferenceReport()
    with pytest.raises(ValueError, match="at least one"):
        report.abandon_outliers()
    for document in range(3):
        report.record_flagged_item(FlaggedItem(document, "$", (FlagScore("robust.isolation_forest", 0.9, 0.5),)))
    assert report.abandon_outliers() == 3
    assert report.flagged_items == () and report.outliers_abandoned and report.discarded_flags == 3
    assert report.to_json()["outliers"] == {"abandoned": True, "discarded": 3, "items": []}
    with pytest.raises(ValueError, match="already abandoned"):
        report.abandon_outliers()
    # Value-level flags raised later in the run are still recorded.
    report.record_flagged_item(FlaggedItem(1, "$.x", (FlagScore("robust.mdl_exceptions", 3.0, 4.0),)))
    assert loaded(report) == report


# ---------------------------------------------------------------------------
# versions
# ---------------------------------------------------------------------------


def test_versions_not_applied() -> None:
    report = InferenceReport()
    report.set_versions(VersionsInfo.not_applied())
    assert report.to_json()["versions"] == {"applied": False}
    assert loaded(report) == report
    with pytest.raises(ValueError, match="already recorded"):
        report.set_versions(VERSIONS)


def test_versions_without_accepted_change_points_have_no_segments() -> None:
    info = VersionsInfo(True, OrderingKey("document-index"), ChangePointCriterion("MDL", 0, 80.0, 80.0))
    assert info.to_json()["segments"] == []
    assert VersionsInfo.from_json(info.to_json()) == info


@pytest.mark.parametrize(
    "build",
    [
        lambda: VersionsInfo(False, OrderingKey("document-index")),
        lambda: VersionsInfo(True, None, ChangePointCriterion("MDL", 0, 1.0, 1.0)),
        lambda: VersionsInfo(True, OrderingKey("document-index"), None),
        lambda: VersionsInfo(True, OrderingKey("document-index"), ChangePointCriterion("MDL", 1, 1.0, 2.0)),
        lambda: VersionsInfo(
            True, OrderingKey("document-index"), ChangePointCriterion("MDL", 0, 1.0, 1.0), (Segment(0, 0, 3, 3),)
        ),
        lambda: OrderingKey("document-index", "$.ts"),
        lambda: OrderingKey("configured"),
        lambda: OrderingKey("timestamp", "$.ts"),  # type: ignore[arg-type]
        lambda: OrderingKey("configured", "$.ts", "$.ts", 2),
        lambda: OrderingKey("document-index", None, "$.ts"),
        lambda: OrderingKey("document-index", None, "$.ts", 0),
        lambda: ChangePointCriterion("MDL", -1, 1.0, 1.0),
        lambda: Segment(0, float("inf"), 1, 1),
    ],
)
def test_versions_reject_inconsistent_records(build: Callable[[], Any]) -> None:
    with pytest.raises((TypeError, ValueError)):
        build()


def test_ordering_key_fallback() -> None:
    key = OrderingKey("document-index", fallback_from='$["ts"]', fallback_documents=3)
    assert key.to_json() == {"fallback_documents": 3, "fallback_from": "$.ts", "source": "document-index"}
    assert OrderingKey.from_json(key.to_json()) == key


# ---------------------------------------------------------------------------
# decisions and rejected candidates
# ---------------------------------------------------------------------------


def test_decision_entry_requires_fallback_info_for_fallback_resolutions() -> None:
    with pytest.raises(ValueError, match="fallback"):
        DecisionEntry("bound", "$.age", FALLBACK_RESOLUTION)
    entry = DecisionEntry("bound", "$.age", FALLBACK_RESOLUTION, fallback=FallbackInfo("values.bounds", "disabled"))
    assert "cutoff" not in entry.to_json() and "details" not in entry.to_json()
    with pytest.raises(TypeError):
        DecisionEntry("bound", "$.age", MDL_RESOLUTION.to_json())  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        DecisionEntry("", "$", MDL_RESOLUTION)


def test_decision_entries_compare_as_json() -> None:
    a = DecisionEntry("bound", "$", MDL_RESOLUTION, details={"flag": True})
    b = DecisionEntry("bound", "$", MDL_RESOLUTION, details={"flag": 1})
    assert a != b and a == DecisionEntry("bound", "$", MDL_RESOLUTION, details={"flag": True})
    with pytest.raises(TypeError):
        hash(a)


def test_decisions_sort_by_path_then_kind() -> None:
    report = InferenceReport()
    for kind, path in [("tuple_or_list", "$.b"), ("bound", "$.b"), ("record_or_map", "$.a{*}"), ("record_or_map", "$.a")]:
        report.record_decision(DecisionEntry(kind, path, MDL_RESOLUTION))
    assert [(d.path, d.kind) for d in report.decisions] == [
        ("$.a", "record_or_map"),
        ("$.a{*}", "record_or_map"),
        ("$.b", "bound"),
        ("$.b", "tuple_or_list"),
    ]


def test_rejected_candidates_group_by_category() -> None:
    report = InferenceReport()
    report.record_rejected(RejectedCandidate("defs", "$.a", None, "structure.shared_defs", {"paths": ["$.a", "$.b"]}))
    report.record_rejected(RejectedCandidate("enums", "$.color", ["red", "blue"], "values.enum_estimation"))
    groups = report.to_json()["rejected"]
    assert groups["defs"] == [  # type: ignore[index]
        {"candidate": None, "details": {"paths": ["$.a", "$.b"]}, "path": "$.a", "technique": "structure.shared_defs"}
    ]
    assert groups["enums"] == [  # type: ignore[index]
        {"candidate": ["red", "blue"], "path": "$.color", "technique": "values.enum_estimation"}
    ]
    assert all(not groups[c] for c in REJECTED_CATEGORIES if c not in ("defs", "enums"))  # type: ignore[index]
    assert loaded(report) == report


@pytest.mark.parametrize(
    ("args", "error"),
    [
        (("misc", "$", 1), ValueError),
        (("enums", "$", {1, 2}), TypeError),
        (("enums", "$", 1, "fallback:enum"), ValueError),
        (("enums", "$", 1, "baseline:type"), ValueError),
        (("enums", "$", 1, None, {"x": float("nan")}), ValueError),
        (("enums", "$", "\udc80"), ValueError),
    ],
)
def test_rejected_candidate_rejects_invalid_fields(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        RejectedCandidate(*args)


def test_rejected_candidates_of_different_categories_differ() -> None:
    assert RejectedCandidate("enums", "$", 1) != RejectedCandidate("formats", "$", 1)
    assert RejectedCandidate("enums", "$", 1) == RejectedCandidate("enums", "$", 1.0)


# ---------------------------------------------------------------------------
# soundness
# ---------------------------------------------------------------------------


def test_soundness_summary_for_the_single_schema() -> None:
    summary = SoundnessSummary(0, 10, 0, 0, 0, 1, False)
    assert summary.to_json() == {
        "failed_first_pass": 0,
        "flagged_documents": 0,
        "pass_limit_reached": False,
        "passes": 1,
        "relaxations": 0,
        "removals": 0,
        "removed": [],
        "validated": 10,
    }
    report = InferenceReport()
    report.record_soundness(summary)
    with pytest.raises(ValueError, match="single schema"):
        report.record_soundness(summary)
    report.record_soundness(SoundnessSummary(0, 0, 0, 0, 0, 0, False, version=0))
    assert [s.version for s in report.soundness] == [None, 0]


def test_soundness_removals_from_relaxation_and_from_the_pass_limit() -> None:
    relaxed_away = RemovedConstraint(
        "/properties/a/format",
        RelaxationRecord({"format": "email"}, REMOVED, (2,), FALLBACK_RESOLUTION),
    )
    summary = SoundnessSummary(0, 4, 2, 2, 1, 5, True, removed=(REMOVAL, relaxed_away))
    assert [entry.pointer for entry in summary.removed] == ["/properties/a/format", "/properties/age/maximum"]
    assert SoundnessSummary.from_json(summary.to_json()) == summary


@pytest.mark.parametrize(
    "args",
    [
        (0, 3, 4, 0, 0, 1, False),  # failed_first_pass > validated
        (0, 3, 0, 1, 0, 1, False),  # relaxations without failures
        (0, 3, 0, 0, 0, 1, True),  # pass limit without failures
        (0, 3, 0, 0, 0, 0, False),  # no pass for validated documents
        (0, 3, 1, 0, 1, 5, False),  # removals before the pass limit
        (0, 3, 1, 1, 1, 5, True),  # a 3.7 removal not listed
        (0, 3, 1, 1, 0, 2, False, (REMOVAL, RemovedConstraint("/maxLength", RelaxationRecord({"maxLength": 1}, REMOVED, (0,))))),
        (0, 3, 1, 0, 0, 2, 1),  # pass_limit_reached is a bool
        (-1, 3, 0, 0, 0, 1, False),
    ],
)
def test_soundness_summary_rejects_inconsistent_counts(args: tuple[Any, ...]) -> None:
    with pytest.raises((TypeError, ValueError)):
        SoundnessSummary(*args)


@pytest.mark.parametrize(
    ("args", "error"),
    [
        (("/a", RelaxationRecord({"maximum": 1}, {"maximum": 2}, (0,))), ValueError),
        (("a", RelaxationRecord({"maximum": 1}, REMOVED, (0,))), ValueError),
        (("/a~2", RelaxationRecord({"maximum": 1}, REMOVED, (0,))), ValueError),
        (("/a", {"before": {"maximum": 1}}), TypeError),
    ],
)
def test_removed_constraint_rejects_invalid_fields(args: tuple[Any, ...], error: type[Exception]) -> None:
    with pytest.raises(error):
        RemovedConstraint(*args)


# ---------------------------------------------------------------------------
# nearest
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("status", ["disabled", "empty-index"])
def test_nearest_lookup_not_run(status: str) -> None:
    info = NearestInfo(status)  # type: ignore[arg-type]
    assert info.to_json() == {"status": status}
    with pytest.raises(ValueError):
        NearestInfo(status, (NearestResult(),))  # type: ignore[arg-type]
    report = InferenceReport()
    report.set_nearest(info)
    assert loaded(report) == report
    with pytest.raises(ValueError, match="already recorded"):
        report.set_nearest(info)


def test_nearest_not_found_is_distinct_from_disabled() -> None:
    searched = NearestInfo("searched", (NearestResult(),))
    assert searched.to_json() == {
        "results": [{"found": False, "neighbors": [], "only_inferred": [], "only_known": [], "shared": []}],
        "status": "searched",
    }
    assert not searched.results[0].found
    assert searched != NearestInfo("disabled")


@pytest.mark.parametrize(
    "build",
    [
        lambda: Neighbor("a", 0.0),
        lambda: Neighbor("a", 1.5),
        lambda: Neighbor("", 0.5),
        lambda: NearestResult((Neighbor("a", 0.5), Neighbor("a", 0.25))),
        lambda: NearestResult(shared=("$.a",)),
        lambda: NearestResult((Neighbor("a", 0.5),), shared=("$.a",), only_known=("$.a",)),
        lambda: NearestResult((Neighbor("a", 0.5),), shared="$.a"),
        lambda: NearestInfo("searched", (NearestResult(version=0), NearestResult(version=0))),
        lambda: NearestInfo("lost"),  # type: ignore[arg-type]
    ],
)
def test_nearest_rejects_invalid_records(build: Callable[[], Any]) -> None:
    with pytest.raises((TypeError, ValueError)):
        build()


def test_nearest_results_are_ranked_and_ordered() -> None:
    result = NearestResult((Neighbor("b", 0.5), Neighbor("c", 1), Neighbor("a", 0.5)), shared=("$.y", "$.x", "$.x"))
    assert [(n.source, n.similarity) for n in result.neighbors] == [("c", 1.0), ("a", 0.5), ("b", 0.5)]
    assert result.shared == ("$.x", "$.y") and result.found
    info = NearestInfo("searched", (NearestResult(version=2), NearestResult(version=0)))
    assert [r.version for r in info.results] == [0, 2]


# ---------------------------------------------------------------------------
# Stage timer and status
# ---------------------------------------------------------------------------


def test_stage_clock_is_perf_counter() -> None:
    assert report_module._clock is time.perf_counter


def test_stage_records_wall_clock_seconds(clock: FakeClock) -> None:
    report = InferenceReport()
    with report.stage("Ingestor") as timer:
        assert report.current_stage == "Ingestor" and timer.component == "Ingestor"
        clock.now += 2.5
    run_stage(report, clock, "Flatten", 0.75)
    assert report.current_stage is None
    assert report.timings == {"Flatten": 0.75, "Ingestor": 2.5}
    assert report.ok


def test_repeated_stage_reports_the_sum_of_its_runs(clock: FakeClock) -> None:
    report = InferenceReport()
    for seconds in (0.125, 0.25, 0.5):  # dyadic, so the fake clock's differences are exact
        run_stage(report, clock, "Structure_Analyzer", seconds)
    assert report.timings == {"Structure_Analyzer": 0.875}


def test_nested_stages_are_timed_inclusively(clock: FakeClock) -> None:
    report = InferenceReport()
    with report.stage("Structure_Analyzer"):
        clock.now += 1.0
        with report.stage("Value_Inferrer"):
            assert report.current_stage == "Value_Inferrer"
            clock.now += 2.0
        assert report.current_stage == "Structure_Analyzer"
    assert report.timings == {"Structure_Analyzer": 3.0, "Value_Inferrer": 2.0}


def test_running_stage_cannot_be_reentered(clock: FakeClock) -> None:
    report = InferenceReport()
    with report.stage("Ingestor"):
        with pytest.raises(ValueError, match="already running"):
            with report.stage("Ingestor"):
                pass  # pragma: no cover
    assert report.ok and list(report.timings) == ["Ingestor"]
    with pytest.raises(ValueError):
        report.stage("")


def test_failed_stage_records_status_and_reraises(clock: FakeClock) -> None:
    report = InferenceReport()
    run_stage(report, clock, "Ingestor", 1.0)
    error = RuntimeError("worker crashed")
    with pytest.raises(RuntimeError) as raised:
        with report.stage("Flatten"):
            clock.now += 5.0
            raise error
    assert raised.value is error
    assert not report.ok and report.current_stage is None
    assert report.status == Status("failed", "Flatten", ErrorInfo("RuntimeError", "worker crashed"))
    # The partial report lists only the completed stages (27.10).
    assert report.timings == {"Ingestor": 1.0}
    assert report.to_json()["status"] == {
        "error": {"message": "worker crashed", "type": "RuntimeError"},
        "stage": "Flatten",
        "state": "failed",
    }
    assert loaded(report) == report
    assert repr(report) == "<InferenceReport failed at 'Flatten'>"


def test_nested_failure_names_the_innermost_stage(clock: FakeClock) -> None:
    report = InferenceReport()
    with pytest.raises(KeyError):
        with report.stage("Structure_Analyzer"):
            run_stage(report, clock, "Dependency_Miner", 0.5)
            with report.stage("Value_Inferrer"):
                raise KeyError("x")
    assert report.status.stage == "Value_Inferrer"
    assert report.status.error == ErrorInfo("KeyError", "'x'")
    assert report.timings == {"Dependency_Miner": 0.5}


def test_first_failure_is_kept(clock: FakeClock) -> None:
    report = InferenceReport()
    with pytest.raises(ValueError):
        with report.stage("Soundness"):
            raise ValueError("first")
    with pytest.raises(OSError):
        with report.stage("Schema_Emitter"):
            raise OSError("second")
    report.fail("Schema_Emitter", ErrorInfo("OSError", "third"))
    assert report.status == Status("failed", "Soundness", ErrorInfo("ValueError", "first"))


def test_interrupts_are_recorded_as_failures(clock: FakeClock) -> None:
    report = InferenceReport()
    with pytest.raises(KeyboardInterrupt):
        with report.stage("Ingestor"):
            raise KeyboardInterrupt
    assert report.status.stage == "Ingestor" and report.status.error == ErrorInfo("KeyboardInterrupt", "")


def test_real_clock_timings_are_finite_and_non_negative() -> None:
    report = InferenceReport()
    with report.stage("Ingestor"):
        sum(range(1000))
    seconds = report.timings["Ingestor"]
    assert math.isfinite(seconds) and seconds >= 0.0


def test_fail_without_a_stage_timer() -> None:
    report = InferenceReport()
    report.fail("Schema_Emitter", OSError("disk full"))
    assert report.status == Status("failed", "Schema_Emitter", ErrorInfo("OSError", "disk full"))
    with pytest.raises(TypeError):
        InferenceReport().fail("Schema_Emitter", "disk full")  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        InferenceReport().fail("", OSError())


class CustomError(Exception):
    pass


class Unprintable(Exception):
    def __str__(self) -> str:
        raise RuntimeError("no")


def test_error_info_from_exception() -> None:
    assert ErrorInfo.from_exception(ValueError("bad value")) == ErrorInfo("ValueError", "bad value")
    assert ErrorInfo.from_exception(CustomError("x")).type == f"{__name__}.CustomError"
    assert ErrorInfo.from_exception(Unprintable()).message == f"<unprintable {__name__}.Unprintable>"
    escaped = ErrorInfo.from_exception(OSError("name \udcff here"))
    assert escaped.message == "name \\udcff here"
    with pytest.raises(ValueError):
        ErrorInfo("OSError", "name \udcff here")
    with pytest.raises(TypeError):
        ErrorInfo.from_exception("boom")  # type: ignore[arg-type]


@pytest.mark.parametrize(
    "args",
    [("failed",), ("failed", "Ingestor"), ("failed", None, ErrorInfo("E", "m")), ("ok", "Ingestor"), ("done",)],
)
def test_status_rejects_inconsistent_fields(args: tuple[Any, ...]) -> None:
    with pytest.raises((TypeError, ValueError)):
        Status(*args)


# ---------------------------------------------------------------------------
# Reproducibility comparison (27.12)
# ---------------------------------------------------------------------------


def test_comparable_json_drops_only_the_exempt_contents(clock: FakeClock) -> None:
    first = full_report(clock)
    second = full_report(clock, workers=2, scale=3.0, started="2026-02-02T10:00:00Z", finished="2026-02-02T10:05:00Z")
    assert first.to_json() != second.to_json()
    assert first.comparable_json() == second.comparable_json()
    data = first.comparable_json()
    assert "timings" not in data and "workers" not in data["resources"]  # type: ignore[operator]
    assert "started" not in data["run"]  # type: ignore[operator]
    assert data["run"]["config"]["run"] == {"model_bundle": "bundles/default", "seed": 0}  # type: ignore[index]
    # Comparison leaves the report itself unchanged.
    assert "workers" in first.to_json()["resources"]  # type: ignore[operator]
    # Every other content difference still counts.
    assert full_report(clock, seed=1).comparable_json() != data
    failed = full_report(clock)
    failed.fail("Soundness", ValueError("x"))
    assert failed.comparable_json() != data


def test_comparable_json_handles_flat_configs_and_unset_runs() -> None:
    report = InferenceReport()
    assert "timings" not in report.comparable_json()
    report.set_run(RunInfo({"workers": 4, "seed": 0}, 0, "b", "v", "p"))
    assert report.comparable_json()["run"]["config"] == {"seed": 0}  # type: ignore[index]


# ---------------------------------------------------------------------------
# Malformed report JSON
# ---------------------------------------------------------------------------


def _set_path(doc: dict[str, Any], pointer: tuple[Any, ...], value: Any) -> dict[str, Any]:
    holder: Any = doc
    for token in pointer[:-1]:
        holder = holder[token]
    holder[pointer[-1]] = value
    return doc


def _delete(doc: dict[str, Any], pointer: tuple[Any, ...]) -> dict[str, Any]:
    holder: Any = doc
    for token in pointer[:-1]:
        holder = holder[token]
    del holder[pointer[-1]]
    return doc


def _reverse(doc: dict[str, Any], pointer: tuple[Any, ...]) -> dict[str, Any]:
    holder: Any = doc
    for token in pointer:
        holder = holder[token]
    holder.reverse()
    return doc


def _append_copy(doc: dict[str, Any], pointer: tuple[Any, ...], index: int) -> dict[str, Any]:
    holder: Any = doc
    for token in pointer:
        holder = holder[token]
    holder.append(copy.deepcopy(holder[index]))
    return doc


MALFORMED: list[tuple[str, Callable[[dict[str, Any]], Any], str]] = [
    ("not an object", lambda d: [d], ""),
    ("missing section", lambda d: _delete(d, ("status",)), ""),
    ("unknown section", lambda d: _set_path(d, ("format",), 1), ""),
    ("section of the wrong type", lambda d: _set_path(d, ("run",), []), "/run"),
    ("skipped of the wrong type", lambda d: _set_path(d, ("ingestion", "skipped"), "2"), "/ingestion/skipped"),
    ("skipped disagrees with the errors", lambda d: _set_path(d, ("ingestion", "skipped"), 5), "/ingestion/skipped"),
    ("line 0", lambda d: _set_path(d, ("ingestion", "errors", 0, "line"), 0), "/ingestion/errors/0"),
    ("surrogate text", lambda d: _set_path(d, ("ingestion", "errors", 0, "reason"), "bad \ud800"), "/ingestion/errors/0"),
    (
        "boolean threshold value",
        lambda d: _set_path(d, ("thresholds", "dbscan.min_samples", "value"), True),
        "/thresholds/dbscan.min_samples/value",
    ),
    ("unknown origin", lambda d: _set_path(d, ("resources", "workers", "origin"), "user"), "/resources/workers"),
    ("disabled substitution", lambda d: _set_path(d, ("substitutions", 0, "fallback", "reason"), "disabled"), "/substitutions/0"),
    ("bad fallback reason", lambda d: _set_path(d, ("substitutions", 0, "fallback", "reason"), "lost"), "/substitutions/0/fallback"),
    ("unknown item member", lambda d: _set_path(d, ("outliers", "items", 0, "note"), "x"), "/outliers/items/0"),
    ("repeated Flagged_Item", lambda d: _append_copy(d, ("outliers", "items"), 0), "/outliers/items/3"),
    ("discarded without abandonment", lambda d: _set_path(d, ("outliers", "discarded"), 2), "/outliers/discarded"),
    ("abandonment discarding nothing", lambda d: _set_path(d, ("outliers", "abandoned"), True), "/outliers/discarded"),
    ("non-canonical Path", lambda d: _set_path(d, ("decisions", 1, "path"), '$["age"]'), "/decisions/1/path"),
    ("bad resolution method", lambda d: _set_path(d, ("decisions", 0, "resolution", "method"), "vote"), "/decisions/0/resolution"),
    ("empty details", lambda d: _set_path(d, ("decisions", 1, "details"), {}), "/decisions/1/details"),
    ("fallback resolution without fallback", lambda d: _delete(d, ("decisions", 1, "fallback")), "/decisions/1"),
    ("unknown category", lambda d: _set_path(d, ("rejected", "misc"), []), "/rejected"),
    ("NaN candidate", lambda d: _set_path(d, ("rejected", "formats", 0, "candidate"), float("nan")), "/rejected/formats/0"),
    ("inconsistent counts", lambda d: _set_path(d, ("soundness", 0, "failed_first_pass"), 10), "/soundness/0"),
    (
        "relaxation listed as removal",
        lambda d: _set_path(
            d, ("soundness", 1, "removed", 0, "change"), {"after": {"maximum": 20}, "before": {"maximum": 10}, "documents": [7]}
        ),
        "/soundness/1/removed/0",
    ),
    ("repeated version", lambda d: _set_path(d, ("soundness", 1, "version"), 0), "/soundness/1"),
    ("extra member when not applied", lambda d: _set_path(d, ("versions",), {"applied": False, "segments": []}), "/versions"),
    ("missing segment", lambda d: d["versions"]["segments"].pop() and d, "/versions"),
    ("found disagrees with neighbors", lambda d: _set_path(d, ("nearest", "results", 1, "found"), True), "/nearest/results/1/found"),
    ("results of a disabled lookup", lambda d: _set_path(d, ("nearest", "status"), "disabled"), "/nearest"),
    ("NaN runtime", lambda d: _set_path(d, ("timings", "Ingestor"), float("nan")), "/timings/Ingestor"),
    ("negative runtime", lambda d: _set_path(d, ("timings", "Ingestor"), -1.0), "/timings/Ingestor"),
    ("string runtime", lambda d: _set_path(d, ("timings", "Ingestor"), "1s"), "/timings/Ingestor"),
    ("ok status with a stage", lambda d: _set_path(d, ("status", "stage"), "Ingestor"), "/status"),
    ("failed status without an error", lambda d: _set_path(d, ("status",), {"stage": "Ingestor", "state": "failed"}), "/status"),
    ("negative seed", lambda d: _set_path(d, ("run", "seed"), -1), "/run"),
    ("package version not a string", lambda d: _set_path(d, ("run", "packages", "numpy"), 2), "/run/packages/numpy"),
]


@pytest.mark.parametrize(("mutate", "location"), [(m, loc) for _, m, loc in MALFORMED], ids=[name for name, _, _ in MALFORMED])
def test_malformed_report_json_is_rejected(clock: FakeClock, mutate: Callable[[dict[str, Any]], Any], location: str) -> None:
    doc = mutate(json.loads(full_report(clock).to_bytes()))
    with pytest.raises(ReportFormatError) as raised:
        InferenceReport.from_json(doc)
    assert raised.value.location == location, str(raised.value)


@pytest.mark.parametrize(
    ("pointer", "location"),
    [
        (("substitutions",), "/substitutions/0"),
        (("outliers", "items"), "/outliers/items/0"),
        (("nearest", "results", 0, "neighbors"), "/nearest/results/0/neighbors/0"),
        (("ingestion", "errors"), None),  # ingestion order is not canonicalized
    ],
)
def test_entries_out_of_canonical_order_are_rejected(clock: FakeClock, pointer: tuple[Any, ...], location: str | None) -> None:
    doc = _reverse(json.loads(full_report(clock).to_bytes()), pointer)
    if location is None:
        assert InferenceReport.from_json(doc).ingest_errors[0].source == "data/a.json"
        return
    with pytest.raises(ReportFormatError) as raised:
        InferenceReport.from_json(doc)
    assert raised.value.location.startswith(location), str(raised.value)


def test_report_format_error_reports_its_location() -> None:
    error = ReportFormatError("/timings/Ingestor", "expected a number")
    assert str(error) == "invalid report JSON at '/timings/Ingestor': expected a number"
    assert str(ReportFormatError("", "x")) == "invalid report JSON at the top level: x"
    assert isinstance(error, ValueError)
    again = pickle.loads(pickle.dumps(error))
    assert (again.location, again.reason) == (error.location, error.reason)


def test_from_json_location_prefix(clock: FakeClock) -> None:
    doc = json.loads(full_report(clock).to_bytes())
    doc["timings"]["Ingestor"] = -2
    with pytest.raises(ReportFormatError) as raised:
        InferenceReport.from_json(doc, location="/report")
    assert raised.value.location == "/report/timings/Ingestor"
