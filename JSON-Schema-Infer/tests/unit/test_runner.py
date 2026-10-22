"""Tests of the isolated system runner and the Baseline_Adapters (task 28.3; Requirements 34.1–34.8, 34.11).

Stub adapters are small Python scripts run through :class:`CommandAdapter`,
one per outcome: a draft 2020-12 schema, a draft-07 schema (translated and
recorded, 34.5), a non-zero exit (34.8), a run past the time limit whose
grandchild must die with it (34.7), an unavailable system (34.6), and
outputs that are not JSON, not a schema, or fail their meta-schema (34.8).
The Naive_Merge and Inferrer adapters run for real; the JSONoid adapter is
checked for its availability rules and configuration, and its real run is
skipped unless ``JSONOID_JAR``, ``JSONOID_SHA256``, and a Java runtime are
present.
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import psutil
import pytest

from jsinfer.bench.baselines.base import Availability, BaselineAdapter, BaselineInfo
from jsinfer.bench.baselines.inferrer import InferrerAdapter
from jsinfer.bench.baselines.jsonoid import JsonoidAdapter, java_version, jsonoid_adapters, jsonoid_from_config
from jsinfer.bench.baselines.naive import NaiveMergeAdapter
from jsinfer.bench.runner import CommandAdapter, run_system, run_systems, write_jsonl
from jsinfer.io.validator import Draft, Validator

DOCS = [{"id": 1, "name": "a", "tags": ["x"]}, {"id": 2, "name": "b", "tags": []}, {"id": 3, "name": "c"}]


@pytest.fixture
def docs_jsonl(tmp_path: Path) -> Path:
    path = tmp_path / "docs.jsonl"
    assert write_jsonl(DOCS, path) == 3
    return path


def _script(tmp_path: Path, name: str, body: str) -> Path:
    path = tmp_path / f"{name}.py"
    path.write_text(body, encoding="utf-8")
    return path


def _adapter(script: Path, name: str = "stub", **kwargs) -> CommandAdapter:
    return CommandAdapter(name, name, (sys.executable, "-I", str(script), "{docs}", "{out}", "{seed}"), **kwargs)


WRITES = """
import json, sys
docs, out, seed = sys.argv[1:4]
json.dump(json.loads({doc!r}), open(out + "/schema.json", "w"))
"""


def test_write_jsonl_keeps_order_and_one_document_per_line(docs_jsonl: Path) -> None:
    lines = docs_jsonl.read_text(encoding="utf-8").splitlines()
    assert [json.loads(line) for line in lines] == DOCS


def test_schema_outcome_records_measurements_and_info(tmp_path: Path, docs_jsonl: Path) -> None:
    script = _script(tmp_path, "ok", WRITES.format(doc='{"type": "object", "properties": {"id": {"type": "integer"}}}'))
    run = run_system(_adapter(script), docs_jsonl, tmp_path / "out", seed=7, timeout_s=60)
    assert run.kind == "schema" and run.ok
    assert run.exit_status == 0 and run.seconds is not None and run.seconds > 0
    assert run.peak_rss_mb is not None and run.peak_rss_mb >= 0
    assert run.translation is None
    assert run.document == {"$schema": Draft.DRAFT202012.uri, "type": "object", "properties": {"id": {"type": "integer"}}}
    assert run.model is not None and not run.model.unsupported
    assert run.command[-1] == "7"
    assert run.info == BaselineInfo("stub", "stub", "unknown", "unknown", {}, _adapter(script).argv)
    assert json.loads((tmp_path / "out" / "schema-2020-12.json").read_text()) == run.document
    assert isinstance(_adapter(script), BaselineAdapter)
    assert run.to_json()["kind"] == "schema"


def test_older_draft_output_is_translated_and_recorded(tmp_path: Path, docs_jsonl: Path) -> None:
    doc = (
        '{"$schema": "http://json-schema.org/draft-07/schema#", "type": "object", '
        '"definitions": {"n": {"type": "string"}}, "properties": {"name": {"$ref": "#/definitions/n"}}, '
        '"required": ["name"]}'
    )
    script = _script(tmp_path, "d7", WRITES.format(doc=doc))
    run = run_system(_adapter(script), docs_jsonl, tmp_path / "out", timeout_s=60)
    assert run.kind == "schema", run.reason
    assert run.translation is not None
    assert run.translation.source_draft == "DRAFT7"
    assert "definitions" in run.translation.translated_keywords
    assert run.document["$schema"] == Draft.DRAFT202012.uri
    assert "definitions" not in run.document
    assert run.to_json()["translation"]["source_draft"] == "DRAFT7"


def test_output_without_dollar_schema_uses_the_adapter_draft(tmp_path: Path, docs_jsonl: Path) -> None:
    # Draft-04 exclusiveMaximum is boolean; as draft 2020-12 it would fail the meta-schema.
    doc = '{"type": "integer", "maximum": 5, "exclusiveMaximum": true}'
    script = _script(tmp_path, "d4", WRITES.format(doc=doc))
    run = run_system(_adapter(script, output_draft=Draft.DRAFT4), docs_jsonl, tmp_path / "out", timeout_s=60)
    assert run.kind == "schema", run.reason
    assert run.translation is not None and run.translation.source_draft == "DRAFT4"
    assert run.document.get("exclusiveMaximum") == 5
    bad = run_system(_adapter(script), docs_jsonl, tmp_path / "out2", timeout_s=60)
    assert bad.kind == "failure" and "meta-schema" in (bad.reason or "")


def test_failure_records_exit_status_and_stderr(tmp_path: Path, docs_jsonl: Path) -> None:
    script = _script(tmp_path, "boom", "import sys\nsys.stderr.write('kaboom\\n')\nsys.exit(3)\n")
    run = run_system(_adapter(script), docs_jsonl, tmp_path / "out", timeout_s=60)
    assert run.kind == "failure" and not run.ok
    assert run.exit_status == 3
    assert "kaboom" in run.stderr
    assert run.reason == "exit status 3"
    assert run.model is None and run.document is None


@pytest.mark.parametrize(
    ("body", "fragment"),
    [
        ("open(sys.argv[2] + '/schema.json', 'w').write('{not json')", "not a JSON document"),
        ("open(sys.argv[2] + '/schema.json', 'w').write('[1, 2]')", "not a JSON Schema document"),
        ("json.dump({'type': 'nonsense'}, open(sys.argv[2] + '/schema.json', 'w'))", "meta-schema"),
        ("pass", "no output file"),
    ],
)
def test_bad_outputs_are_failures_with_reasons(tmp_path: Path, docs_jsonl: Path, body: str, fragment: str) -> None:
    script = _script(tmp_path, "bad", f"import json, sys\n{body}\n")
    run = run_system(_adapter(script), docs_jsonl, tmp_path / "out", timeout_s=60)
    assert run.kind == "failure"
    assert run.exit_status == 0
    assert fragment in (run.reason or "")


def test_unavailable_system_is_recorded_without_a_run(tmp_path: Path, docs_jsonl: Path) -> None:
    script = _script(tmp_path, "never", "raise SystemExit('must not run')\n")
    adapter = _adapter(script, availability=Availability(False, "no runtime"))
    run = run_system(adapter, docs_jsonl, tmp_path / "out", timeout_s=60)
    assert run.kind == "unavailable" and run.reason == "no runtime"
    assert run.seconds is None and run.command == ()
    assert not (tmp_path / "out").exists()


def test_timeout_kills_the_whole_process_tree(tmp_path: Path, docs_jsonl: Path) -> None:
    body = """
import subprocess, sys, time
out = sys.argv[2]
child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"])
open(out + "/grandchild.pid", "w").write(str(child.pid))
time.sleep(60)
"""
    script = _script(tmp_path, "slow", body)
    out = tmp_path / "out"
    started = time.monotonic()
    run = run_system(_adapter(script), docs_jsonl, out, timeout_s=1.0)
    assert run.kind == "timeout"
    assert run.exit_status is None and run.reason == "exceeded 1.0 s"
    assert run.seconds is not None and 1.0 <= run.seconds < 20
    assert time.monotonic() - started < 30
    pid_file = out / "grandchild.pid"
    assert pid_file.is_file()
    grandchild = int(pid_file.read_text())
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline and psutil.pid_exists(grandchild) and psutil.Process(grandchild).status() != psutil.STATUS_ZOMBIE:
        time.sleep(0.05)
    assert not psutil.pid_exists(grandchild) or psutil.Process(grandchild).status() == psutil.STATUS_ZOMBIE


def test_timeout_must_be_a_valid_resource_parameter(tmp_path: Path, docs_jsonl: Path) -> None:
    with pytest.raises(ValueError, match="bench.run_timeout_s"):
        run_system(_adapter(_script(tmp_path, "x", "pass")), docs_jsonl, tmp_path / "out", timeout_s=-1)


def test_run_systems_is_sequential_over_the_same_file(tmp_path: Path, docs_jsonl: Path) -> None:
    body = """
import json, sys, time
docs, out = sys.argv[1], sys.argv[2]
n = sum(1 for line in open(docs) if line.strip())
json.dump({"type": "object", "minProperties": n, "description": docs}, open(out + "/schema.json", "w"))
"""
    script = _script(tmp_path, "count", body)
    adapters = [_adapter(script, name="first"), _adapter(script, name="second")]
    runs = run_systems(adapters, docs_jsonl, tmp_path / "runs", timeout_s=60)
    assert [r.system for r in runs] == ["first", "second"]
    assert all(r.kind == "schema" for r in runs)
    assert all(r.document["minProperties"] == 3 for r in runs)
    assert {r.document["description"] for r in runs} == {str(docs_jsonl.resolve())}
    assert (tmp_path / "runs" / "first" / "stderr.txt").exists()
    assert (tmp_path / "runs" / "second" / "schema-2020-12.json").exists()


def test_naive_merge_adapter_runs_the_naive_merge_pipeline(tmp_path: Path, docs_jsonl: Path) -> None:
    from jsinfer.io.emit import SchemaEmitter
    from jsinfer.pipeline.naive_merge import naive_merge

    adapter = NaiveMergeAdapter()
    assert adapter.available().available
    assert adapter.output_draft is None
    run = run_system(adapter, docs_jsonl, tmp_path / "out", timeout_s=120)
    assert run.kind == "schema", (run.reason, run.stderr)
    expected, _ = SchemaEmitter().layout(naive_merge(DOCS))
    assert run.document == expected
    assert set(run.document) <= {"$schema", "type", "properties", "required", "items", "enum"}
    info = run.info
    assert info.name == "naive_merge" and info.runtime.startswith("python ")
    assert info.options["keywords"] == ["type", "properties", "required", "items", "enum"]


def test_naive_merge_main_usage(capsys: pytest.CaptureFixture[str]) -> None:
    from jsinfer.bench.baselines.naive import main

    assert main([]) == 2
    assert "usage" in capsys.readouterr().err


def test_inferrer_adapter_reports_a_missing_config_file(tmp_path: Path) -> None:
    adapter = InferrerAdapter(config=tmp_path / "missing.toml")
    availability = adapter.available()
    assert not availability.available and "missing.toml" in (availability.reason or "")


def test_jsonoid_adapter_availability_rules(tmp_path: Path) -> None:
    jar = tmp_path / "jsonoid.jar"
    jar.write_bytes(b"not really a jar")
    import hashlib

    digest = hashlib.sha256(b"not really a jar").hexdigest()
    no_java = JsonoidAdapter(jar, digest, "v0.0.0", java=str(tmp_path / "no-such-java"))
    assert not no_java.available().available and "Java" in no_java.available().reason
    assert no_java.output_draft is Draft.DRAFT201909
    info = no_java.describe()
    assert info.version == "v0.0.0" and info.options["jar_sha256"] == digest
    assert info.options["property_set"] == "default"
    command = list(no_java.command(Path("/d/docs.jsonl"), tmp_path / "out", 42))
    assert command[1:5] == ["-jar", str(jar.resolve()), "--random-seed", "42"]
    assert command[-2:] == ["-w", str(tmp_path / "out" / "jsonoid.json")]
    if java_version() is not None:
        assert JsonoidAdapter(jar, "0" * 64, "v").available().reason.startswith("jar digest")
        assert JsonoidAdapter(tmp_path / "none.jar", digest, "v").available().reason.startswith("no jar")
        assert JsonoidAdapter(jar, digest.upper(), "v").available().available


def test_a_java_that_exits_with_an_error_is_no_java_runtime(tmp_path: Path) -> None:
    """34.6: macOS's /usr/bin/java stub prints a message and exits 1; that is unavailability, not a version."""
    stub = tmp_path / "stub-java"
    stub.write_text("#!/bin/sh\necho 'The operation couldn’t be completed. Unable to locate a Java Runtime.' >&2\nexit 1\n")
    stub.chmod(0o755)
    real = tmp_path / "real-java"
    real.write_text("#!/bin/sh\necho 'openjdk version \"21.0.0\" 2024-01-01' >&2\nexit 0\n")
    real.chmod(0o755)
    assert java_version(str(stub)) is None
    assert java_version(str(real)) == 'openjdk version "21.0.0" 2024-01-01'
    jar = tmp_path / "jsonoid.jar"
    jar.write_bytes(b"jar")
    import hashlib

    digest = hashlib.sha256(b"jar").hexdigest()
    unavailable = JsonoidAdapter(jar, digest, "v", java=str(stub)).available()
    assert not unavailable.available and unavailable.reason == f"no Java runtime ({stub})"
    assert JsonoidAdapter(jar, digest, "v", java=str(real)).available().available


JSONOID_EMPTY_ARRAYS = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
        "topics": {"type": "array", "minItems": 0, "items": [], "maxItems": 0, "lengthHistogram": {"bins": [[0.0, 4]]}},
        "tags": {"type": "array", "items": {"type": "string"}},
        "items": {"type": "array", "items": [], "maxItems": 0},  # a property named "items" is not the keyword
    },
}


def test_jsonoid_repair_removes_only_empty_items_arrays(tmp_path: Path) -> None:
    """34.12: "items": [] (beside maxItems: 0) goes; schema-valued items and properties named items stay."""
    adapter = JsonoidAdapter(tmp_path / "j.jar", "0" * 64, "v")
    repaired, removed = adapter.repair_output(JSONOID_EMPTY_ARRAYS)
    assert removed == ("/properties/topics/items", "/properties/items/items")
    assert "items" not in repaired["properties"]["topics"] and repaired["properties"]["topics"]["maxItems"] == 0
    assert repaired["properties"]["tags"]["items"] == {"type": "string"}
    assert "items" in repaired["properties"] and "items" not in repaired["properties"]["items"]
    assert JSONOID_EMPTY_ARRAYS["properties"]["topics"]["items"] == []  # the input is not modified
    assert adapter.repair_output({"type": "object"}) == ({"type": "object"}, ())


def test_a_repaired_jsonoid_output_loads_and_records_the_repairs(tmp_path: Path, docs_jsonl: Path) -> None:
    """34.8, 34.12: unrepaired, the output fails the 2020-12 meta-schema; through the JSONoid adapter it loads."""
    java = tmp_path / "java"
    java.write_text(
        "#!/bin/sh\n"
        'if [ "$1" = "-version" ]; then echo "openjdk version \\"21\\"" >&2; exit 0; fi\n'
        'while [ "$1" != "-w" ]; do shift; done\n'
        f"cat > \"$2\" <<'JSON'\n{json.dumps(JSONOID_EMPTY_ARRAYS)}\nJSON\n"
    )
    java.chmod(0o755)
    jar = tmp_path / "jsonoid.jar"
    jar.write_bytes(b"jar")
    import hashlib

    adapter = JsonoidAdapter(jar, hashlib.sha256(b"jar").hexdigest(), "v-test", java=str(java))
    assert Validator().meta_errors(JSONOID_EMPTY_ARRAYS, Draft.DRAFT202012)  # the raw output is invalid
    run = run_system(adapter, docs_jsonl, tmp_path / "out", seed=0, timeout_s=60)
    assert run.kind == "schema", (run.reason, run.stderr)
    assert run.repairs == ("/properties/topics/items", "/properties/items/items")
    assert run.to_json()["repairs"] == list(run.repairs)
    assert json.loads((tmp_path / "out" / "repairs.json").read_text()) == {"removed": list(run.repairs)}
    compiled = Validator().compile(run.document)
    assert Validator().is_valid(compiled, {"topics": [], "tags": ["a"]})
    assert not Validator().is_valid(compiled, {"topics": ["x"]})  # maxItems: 0 still holds


def test_jsonoid_variants_share_the_jar_and_record_their_property_set(tmp_path: Path) -> None:
    """34.1, 34.11: each [baselines.jsonoid.variants.<name>] is one more system with its own arguments, recorded."""
    table = {
        "jar": "j.jar", "sha256": "ab", "version": "v1",
        "variants": {"simple-open": {"label": "JSONoid (Simple, open objects)", "extra_args": ["--prop", "Simple", "--additional-properties"]}},
    }
    main, variant = jsonoid_adapters(table, tmp_path)
    assert (main.name, main.label, main.extra_args) == ("jsonoid", "JSONoid", ())
    assert (variant.name, variant.label) == ("jsonoid-simple-open", "JSONoid (Simple, open objects)")
    assert variant.jar == main.jar == tmp_path / "j.jar" and variant.sha256 == "ab"
    assert main.describe().options["property_set"] == "default" and main.describe().options["additional_properties"] == "closed"
    options = variant.describe().options
    assert (options["property_set"], options["additional_properties"]) == ("Simple", "open")
    assert options["extra_args"] == ["--prop", "Simple", "--additional-properties"]
    assert list(variant.command(Path("/d.jsonl"), tmp_path, 3))[5:8] == ["--prop", "Simple", "--additional-properties"]
    assert jsonoid_adapters({"jar": "j.jar", "sha256": "ab", "version": "v1"}, tmp_path)[0].name == "jsonoid"
    with pytest.raises(ValueError, match="extra_args"):
        jsonoid_adapters({**table, "variants": {"x": {"extra_args": "-a"}}}, tmp_path)


def test_jsonoid_from_config(tmp_path: Path) -> None:
    adapter = jsonoid_from_config({"jar": "tools/j.jar", "sha256": "ab", "version": "v1", "extra_args": ["-p", "Min"]}, tmp_path)
    assert adapter.jar == tmp_path / "tools" / "j.jar"
    assert adapter.extra_args == ("-p", "Min")
    assert adapter.describe().options["extra_args"] == ["-p", "Min"]
    with pytest.raises(ValueError, match="sha256"):
        jsonoid_from_config({"jar": "x", "version": "v"}, tmp_path)
    with pytest.raises(ValueError, match="extra_args"):
        jsonoid_from_config({"jar": "x", "sha256": "a", "version": "v", "extra_args": "-p"}, tmp_path)


@pytest.mark.skipif(
    java_version() is None or not os.environ.get("JSONOID_JAR") or not os.environ.get("JSONOID_SHA256"),
    reason="needs a Java runtime and JSONOID_JAR/JSONOID_SHA256",
)
def test_jsonoid_adapter_runs_the_jar(tmp_path: Path, docs_jsonl: Path) -> None:
    adapter = JsonoidAdapter(Path(os.environ["JSONOID_JAR"]), os.environ["JSONOID_SHA256"], os.environ.get("JSONOID_VERSION", "env"))
    assert adapter.available().available, adapter.available().reason
    run = run_system(adapter, docs_jsonl, tmp_path / "out", seed=1, timeout_s=600)
    assert run.kind == "schema", (run.reason, run.stderr[-2000:])
    # Release 0.40 declares draft 2020-12 itself; an older jar's 2019-09 output is translated and recorded.
    assert run.translation is None or run.translation.source_draft == "DRAFT201909"
    assert run.document["$schema"] == Draft.DRAFT202012.uri
    assert run.model is not None and not run.model.unsupported  # JSONoid's own keywords (bloomFilter, hll, ...) are ignored
    assert run.info.runtime.startswith("openjdk") or "java" in run.info.runtime.lower()


def test_harness_settings_make_a_project_bundle_directory_absolute(tmp_path: Path) -> None:
    """The Inferrer runs in each fold's directory, so a bundle under the project must be passed as an absolute path."""
    from jsinfer.bench.config import BenchConfig
    from jsinfer.bench.runner import HarnessSettings

    (tmp_path / "corpus").mkdir()
    (tmp_path / "corpus" / "manifest.json").write_text("{}", encoding="utf-8")
    (tmp_path / "bundles" / "mine").mkdir(parents=True)
    document = {"corpus": {"manifest": "corpus/manifest.json"}, "split": {"out": "split.json"}, "bench": {"model_bundle": "bundles/mine"}}
    settings = HarnessSettings.from_config(BenchConfig.from_mapping(document, tmp_path))
    assert Path(settings.model_bundle).is_absolute() and Path(settings.model_bundle) == (tmp_path / "bundles" / "mine").resolve()
    shipped = HarnessSettings.from_config(BenchConfig.from_mapping({**document, "bench": {"model_bundle": "bundles/default"}}, tmp_path))
    assert shipped.model_bundle == "bundles/default"  # not a directory under the project: an identifier load_bundle resolves
