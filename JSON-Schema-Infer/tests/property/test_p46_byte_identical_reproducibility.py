"""Property 46: Byte-identical reproducibility (task 19.4).

For any collection and Run_Configuration, runs with worker counts in
{1, 2, 4}, different ``PYTHONHASHSEED`` values, and different output
directories produce byte-identical schemas (or versions, diffs, and
combined schema) and explanation files. Their reports are equal after
removing timings, timestamps, and the worker count (27.5, 27.12).

Runs
    Each example writes a generated Document_Collection and a configuration
    file (the drawn Run_Configuration, ``config.to_toml()``), then runs
    ``jsinfer infer INPUT -c FILE --set workers=W --out DIR`` once in each
    of three driver subprocesses:

    ======  ===================  =======  ===============================
    driver  ``PYTHONHASHSEED``   workers  ``--out``, under the example
    ======  ===================  =======  ===============================
    ``h0``  ``0``                1        ``out-0``
    ``h1``  ``1``                2        ``nested/é 😀/out-1``
    ``hr``  ``random``           4        ``deeper/a/b/c/out-random``
    ======  ===================  =======  ===============================

    Every pair of runs differs in all three respects, so each worker count,
    each hash seed, and each output location is compared with every other.
    A full 3×3 grid would help attribute a difference but would not find
    more of them: missing one would need outputs that depend on the hash
    seed and the worker count jointly yet agree on all three pairs here.

    A driver (:data:`DRIVER`) is a Python process that calls
    ``jsinfer.cli.main(argv)``, the console script's entry point, once per
    request, and keeps running between examples. A fresh process costs one
    to three seconds (the interpreter, the NumPy and scikit-learn imports,
    and with workers the loky worker launch), against a fraction of a
    second for a run on these collections. Each driver keeps one worker
    count, so joblib's reusable loky executor keeps its worker processes
    (changing ``n_jobs`` restarts them). The workers inherit the driver's
    ``PYTHONHASHSEED``; under ``random`` each one draws its own. Drivers
    restart after staggered numbers of runs (:data:`RESTART_EVERY`), so cold
    first runs in a new process are compared with warm runs throughout, and
    the ``random`` driver gets a new hash seed each time.

Non-vacuity
    - Hash seeds: each driver reports ``hash()`` of a fixed string when it
      starts, and the live drivers' values must differ.
    - Workers: each collection holds at least ``4 × ingest.chunk_size``
      valid documents, so every stage has at least four chunks and a run
      with W workers hands them to W processes. After each run, a driver
      with W > 1 must have at least W live loky worker processes, and the
      one-worker driver none.
    - Each run used its own worker count: ``effective-config.toml`` and the
      report's ``resources`` section hold W.

Comparison
    - Every run exits 0 and writes the same file names: ``schema.json``,
      ``explanation.json``, ``report.json``, ``effective-config.toml``, and
      the Schema_Version, diff, and combined-schema files once task 48.2
      emits them, which are then compared like the schema.
    - Every file other than the report and the configuration file is
      byte-identical across the runs.
    - ``report.json`` is read with ``InferenceReport.from_json``, which also
      checks its canonical form. Its ``comparable_json()`` (the report
      without the ``timings`` section, the run timestamps, and the worker
      count, exactly what 27.12 exempts) must encode to the same canonical
      bytes in every run.
    - ``effective-config.toml`` differs only in ``[run] workers``.

Inputs
    Collections (4 to 40 documents, nesting depth at most 4) come from
    random templates: Records with one to three Variants (maybe with a
    ``const`` Discriminator) and always, sometimes, and jointly present
    fields, Maps with data-like or Format keys, Lists, Tuples, mixed types,
    flat free-form values (``tests.strategies.json_scalars`` and lists and
    objects of them), and scalar families that reach the value Techniques (bounded integers, decimals, big integers, small
    vocabularies, Formats, latent numbers, awkward strings). Property names
    include :data:`tests.strategies.KEYS` and other non-ASCII names, so many
    distinct strings are hashed and set orders differ between hash seeds.
    The collection is one ``.jsonl`` file or a directory of ``.json``,
    ``.jsonl``, and ``.ndjson`` files (one in a subdirectory), written with
    ``\\u`` escapes or as raw UTF-8, sometimes with unparseable lines (skipped
    records in the report).

    Configurations come from ``setting_flags``: every implemented
    Technique's toggle, ``seed``, ``input.unwrap_arrays``, and
    ``output.inline_provenance``, with a small ``ingest.chunk_size`` and
    small ``soundness.max_passes``, MinHash, EM, and ``sample.*`` sizes (so
    DBSCAN and HDBSCAN draw seeded samples). A Model_Bundle directory with
    hand-set Learned_Thresholds is written per distinct threshold set, so
    MinHash/LSH, DBSCAN, HDBSCAN, and co-occurrence rules run as well as
    their fallbacks. The scenarios are those of Property 7: ``drawn``,
    ``priority`` (``decide.mdl`` off), and ``dbscan first``.

Cost
    The ``ci`` profile's 100 examples, as for every property (design,
    Testing Strategy): three runs per example, in parallel, plus about ten
    driver starts of one to five seconds each. About a minute in all on an
    8–14 core laptop; ``nightly`` (500 examples) about five.

**Validates: Requirements 27.5, 27.12**
"""

# Feature: json-schema-inference, Property 46: Byte-identical reproducibility

from __future__ import annotations

import concurrent.futures
import hashlib
import itertools
import json
import os
import shutil
import subprocess
import sys
import tomllib
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import IO, Any

import pytest
from hypothesis import event, given
from hypothesis import strategies as st

from jsinfer import CONFIG_FILE, EXPLANATION_FILE, REPORT_FILE, SCHEMA_FILE
from jsinfer.config.registry import REGISTRY
from jsinfer.config.resources import RESOURCE_PARAMETERS, WORKERS
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.config.thresholds import ThresholdRecord, threshold_spec
from jsinfer.core.determinism import encode_canonical
from jsinfer.core.report import InferenceReport
from jsinfer.learn.bundle import BUNDLE_FORMAT, MANIFEST_FILE, THRESHOLDS_FILE
from jsinfer.structure.variants import DBSCAN_TECHNIQUE as DBSCAN, MINHASH_LSH
from tests.strategies import KEYS, TECHNIQUE_SETTINGS, json_scalars, setting_flags

OUTPUTS = (CONFIG_FILE, EXPLANATION_FILE, REPORT_FILE, SCHEMA_FILE)
MDL = "decide.mdl"

# Seconds one run (or a driver's start) may take; generous, so a slow or busy machine is not a failure.
TIMEOUT = 300

# ---------------------------------------------------------------------------
# Driver processes
# ---------------------------------------------------------------------------

# The string whose hash() each driver reports, to show the hash seeds differ.
HASH_PROBE = "jsinfer property 46"

# A driver: one JSON request per line on standard input (the CLI's argv), one JSON reply per line on
# standard output. The protocol runs over private copies of descriptors 0 and 1 (os.dup gives
# non-inheritable descriptors), so the CLI and the loky workers it launches see os.devnull as standard
# input and the log file (standard error) as standard output.
DRIVER = r"""
import contextlib, io, json, os, sys, traceback

requests = os.fdopen(os.dup(0), "r", encoding="utf-8")
replies = os.fdopen(os.dup(1), "w", encoding="utf-8")
null = os.open(os.devnull, os.O_RDONLY)
os.dup2(null, 0)
os.close(null)
os.dup2(2, 1)

import psutil
from jsinfer import cli

def loky_workers():
    count = 0
    for child in psutil.Process().children():
        try:
            if any("LokyProcess" in part for part in child.cmdline()):
                count += 1
        except psutil.Error:
            pass
    return count

def reply(message):
    replies.write(json.dumps(message) + "\n")
    replies.flush()

reply({"hash": hash(sys.argv[1]), "pid": os.getpid()})
for line in requests:
    argv = json.loads(line)
    out, err = io.StringIO(), io.StringIO()
    status, error = None, None
    try:
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = cli.main(argv)
    except BaseException:
        error = traceback.format_exc()
    reply({"status": status, "stdout": out.getvalue(), "stderr": err.getvalue(), "error": error,
           "workers": loky_workers()})
"""


@dataclass(frozen=True)
class DriverSpec:
    label: str
    hash_seed: str
    workers: int
    out: str  # the run's --out, relative to the example directory
    restart_every: int  # runs per driver process


DRIVERS = (
    # Restart periods are distinct primes, so a restarted driver's cold run mostly meets two warm ones.
    DriverSpec("h0", "0", 1, "out-0", 23),
    DriverSpec("h1", "1", 2, "nested/é 😀/out-1", 31),
    DriverSpec("hr", "random", 4, "deeper/a/b/c/out-random", 41),
)
RESTART_EVERY = {spec.label: spec.restart_every for spec in DRIVERS}
assert sorted(spec.workers for spec in DRIVERS) == [1, 2, 4]


@dataclass
class Reply:
    spec: DriverSpec
    cold: bool  # the first run of its driver process
    status: int | None
    stdout: str
    stderr: str
    error: str | None
    loky_workers: int


@dataclass
class Driver:
    spec: DriverSpec
    logs: Path
    process: subprocess.Popen[str] | None = None
    log: IO[str] | None = None
    probe: int | None = None
    runs: int = 0  # runs of the current process
    starts: int = 0

    def _start(self) -> None:
        self.log = open(self.logs / f"{self.spec.label}-{self.starts}.log", "w+", encoding="utf-8")
        self.starts += 1
        env = {**os.environ, "PYTHONHASHSEED": self.spec.hash_seed, "PYTHONIOENCODING": "utf-8"}
        self.process = subprocess.Popen(
            [sys.executable, "-c", DRIVER, HASH_PROBE],
            cwd=self.logs,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.log,
            encoding="utf-8",
        )
        self.probe = self._read()["hash"]
        self.runs = 0

    def _read(self) -> dict[str, Any]:
        assert self.process is not None and self.process.stdout is not None
        line = self.process.stdout.readline()
        if not line:
            self.process.wait(timeout=TIMEOUT)
            raise AssertionError(
                f"driver {self.spec.label} exited with status {self.process.returncode}:\n{self.log_tail()}"
            )
        return json.loads(line)

    def log_tail(self, size: int = 4000) -> str:
        if self.log is None:
            return ""
        self.log.flush()
        self.log.seek(0)
        return self.log.read()[-size:]

    def ensure_started(self) -> bool:
        """Start (or restart, after ``restart_every`` runs) the process; whether the next run is cold."""
        if self.process is not None and self.runs >= self.spec.restart_every:
            self.stop()
        if self.process is None:
            self._start()
            return True
        return False

    def run(self, argv: Sequence[str]) -> Reply:
        try:
            cold = self.ensure_started()
            assert self.process is not None and self.process.stdin is not None
            self.process.stdin.write(json.dumps(list(argv)) + "\n")
            self.process.stdin.flush()
            message = self._read()
        except BaseException:
            self.kill()  # a driver in an unknown state is replaced at the next run
            self.stop()
            raise
        self.runs += 1
        return Reply(
            self.spec,
            cold,
            message["status"],
            message["stdout"],
            message["stderr"],
            message["error"],
            message["workers"],
        )

    def stop(self) -> None:
        process, self.process = self.process, None
        if process is not None:
            if process.stdin is not None:
                process.stdin.close()  # the request loop ends, and loky shuts its workers down
            try:
                process.wait(timeout=60)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            if process.stdout is not None:
                process.stdout.close()
        if self.log is not None:
            self.log.close()
            self.log = None
        self.probe = None

    def kill(self) -> None:
        if self.process is not None:
            self.process.kill()


class DriverPool:
    """The three drivers; :meth:`run` runs one request per driver, in parallel."""

    def __init__(self, logs: Path) -> None:
        self.drivers = [Driver(spec, logs) for spec in DRIVERS]
        self.threads = concurrent.futures.ThreadPoolExecutor(max_workers=len(DRIVERS))

    def run(self, argv_of: Callable[[DriverSpec], Sequence[str]]) -> list[Reply]:
        futures = [self.threads.submit(driver.run, argv_of(driver.spec)) for driver in self.drivers]
        try:
            replies = [future.result(timeout=TIMEOUT) for future in futures]
        except concurrent.futures.TimeoutError:
            for driver in self.drivers:
                driver.kill()  # unblocks the reads; the next example starts new drivers
            concurrent.futures.wait(futures)
            for driver in self.drivers:
                driver.stop()
            pytest.fail(f"a run took more than {TIMEOUT} s")
        probes = {driver.spec.label: driver.probe for driver in self.drivers}
        assert len(set(probes.values())) == len(probes), f"hash seeds did not differ: {probes}"
        return replies

    def close(self) -> None:
        for driver in self.drivers:
            driver.stop()
        self.threads.shutdown()


@pytest.fixture(scope="module")
def pool(tmp_path_factory: pytest.TempPathFactory) -> Iterator[DriverPool]:
    drivers = DriverPool(tmp_path_factory.mktemp("p46-drivers"))
    try:
        yield drivers
    finally:
        drivers.close()


@pytest.fixture(scope="module")
def scratch(tmp_path_factory: pytest.TempPathFactory) -> Iterator[Callable[[], Path]]:
    """A new empty directory per example (removed after it)."""
    root = tmp_path_factory.mktemp("p46-examples")
    counter = itertools.count()
    yield lambda: Path(root, f"example-{next(counter)}")


# ---------------------------------------------------------------------------
# Model_Bundles with hand-set Learned_Thresholds
# ---------------------------------------------------------------------------

# Hand-set Learned_Thresholds inside their admissible ranges, with the extremes common.
THRESHOLD_VALUES: dict[str, st.SearchStrategy[int | float]] = {
    "lsh.keyset_jaccard": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
    "dbscan.eps": st.one_of(st.sampled_from([1e-9, 0.34, 0.5, 1.0]), st.floats(0.0, 1.0, exclude_min=True)),
    "dbscan.min_samples": st.integers(1, 6),
    "hdbscan.min_cluster_size": st.integers(2, 6),
    "cooccur.min_support": st.integers(1, 4),
    "cooccur.min_confidence": st.one_of(st.sampled_from([0.0, 0.5, 1.0]), st.floats(0.0, 1.0)),
}
THRESHOLD_TECHNIQUES = tuple(
    tid for tid in REGISTRY.ids() if any(name in THRESHOLD_VALUES for name in REGISTRY.technique(tid).thresholds)
)


class BundleDir:
    """Model_Bundle directories of hand-set Learned_Thresholds, one per distinct threshold set."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.written: dict[str, Path] = {}

    def path(self, thresholds: Mapping[str, int | float]) -> Path:
        entries = {
            name: ThresholdRecord.from_spec(
                threshold_spec(name), value=value, objective_value=0.5, n_examples=1, training_data="p46-stub"
            ).to_json()
            for name, value in sorted(thresholds.items())
        }
        text = json.dumps(entries, sort_keys=True)
        digest = hashlib.blake2b(text.encode(), digest_size=8).hexdigest()
        if digest not in self.written:
            bundle = self.root / digest
            bundle.mkdir()
            manifest = {"artifacts": {}, "bundle_id": f"p46-{digest}", "format": BUNDLE_FORMAT}
            (bundle / MANIFEST_FILE).write_text(json.dumps(manifest), encoding="utf-8")
            (bundle / THRESHOLDS_FILE).write_text(text, encoding="utf-8")
            self.written[digest] = bundle
        return self.written[digest]


@pytest.fixture(scope="module")
def bundles(tmp_path_factory: pytest.TempPathFactory) -> BundleDir:
    return BundleDir(tmp_path_factory.mktemp("p46-bundles"))


# ---------------------------------------------------------------------------
# Collections
# ---------------------------------------------------------------------------

FIELD_KEYS = (*KEYS, "x-provenance", "$ref", "type", "~1/", "%25", "Ω", "ß", "ﬁ", "𝔘")

_FORMAT_VALUES: Mapping[str, Callable[[int], str]] = {
    "date-time": lambda i: f"2024-{i % 12 + 1:02d}-{i % 28 + 1:02d}T{i % 24:02d}:{i % 60:02d}:00Z",
    "date": lambda i: f"20{i % 100:02d}-{i % 12 + 1:02d}-{i % 28 + 1:02d}",
    "email": lambda i: f"user{i}@example.com",
    "ipv4": lambda i: f"10.{i % 256}.{3 * i % 256}.1",
    "uri": lambda i: f"https://example.com/p/{i}",
    "uuid": lambda i: f"{i:08x}-1234-4abc-8def-{7919 * i:012x}",
}
_WORDS = ("new", "paid", "sent", "é", "😀", "a b", "", "\uffff")
_TEXT = st.text(alphabet='ab"\\\n\t😀é~/\u2028', max_size=6)
SCALAR_FAMILIES = (
    "int",
    "int",
    "float",
    "bigint",
    "bool",
    "null",
    "vocab",
    "vocab",
    "int-vocab",
    "text",
    "format",
    "format",
    "latent-int",
    "latent-dec",
    "free",
)
MAP_KEY_FAMILIES = ("data", "data", "uuid", "date", "email")
KINDS = ("scalar", "scalar", "record", "record", "map", "list", "tuple", "mix")
PRESENCE = ("always", "always", "sometimes", "group")


@dataclass(frozen=True)
class Template:
    """A generator of JSON values, as in Property 43.

    ``scalar``: ``family`` and its ``params``. ``record``: per Variant, its
    fields ``(key, presence, child)``, and ``tags`` the Discriminator
    ``(key, value)`` of each Variant, if any. ``map``: the key ``family`` and
    the value template. ``list``: the element template. ``tuple``: one
    template per position. ``mix``: the alternatives.
    """

    kind: str
    family: str = ""
    params: tuple[Any, ...] = ()
    variants: tuple[tuple[tuple[str, str, Template], ...], ...] = ()
    tags: tuple[tuple[str, str], ...] = ()
    children: tuple[Template, ...] = ()


@st.composite
def scalar_templates(draw: st.DrawFn) -> Template:
    family = draw(st.sampled_from(SCALAR_FAMILIES), label="scalar family")
    if family == "int":
        low = draw(st.sampled_from([0, 1, -5, 100, -1000]), label="low")
        return Template("scalar", family, (low, draw(st.sampled_from([0, 3, 50, 999]), label="span")))
    if family in ("vocab", "int-vocab"):
        members = st.sampled_from(_WORDS if family == "vocab" else (0, 1, 2, 7))
        return Template("scalar", family, tuple(draw(st.lists(members, min_size=1, max_size=3, unique=True))))
    if family == "format":
        return Template("scalar", family, (draw(st.sampled_from(sorted(_FORMAT_VALUES)), label="format"),))
    return Template("scalar", family)


# Free-form values at most one container deep, so documents stay within depth 4.
_FREE = st.one_of(
    json_scalars,
    st.lists(json_scalars, max_size=3),
    st.dictionaries(st.sampled_from(KEYS), json_scalars, max_size=3),
)


def scalar_values(template: Template) -> st.SearchStrategy[Any]:
    family, params = template.family, template.params
    if family == "int":
        low, span = params
        return st.integers(low, low + span)
    if family == "float":
        return st.integers(-40, 400).map(lambda k: k / 8)  # mostly non-integral decimals
    if family == "bigint":
        return st.integers(2**64 - 2, 2**64 + 2)
    if family == "bool":
        return st.booleans()
    if family == "null":
        return st.none()
    if family in ("vocab", "int-vocab"):
        return st.sampled_from(params)
    if family == "text":
        return _TEXT
    if family == "format":
        return st.integers(0, 500).map(_FORMAT_VALUES[params[0]])
    if family == "latent-int":
        return st.integers(-50, 5000).map(str)
    if family == "latent-dec":
        return st.tuples(st.integers(0, 999), st.integers(0, 99)).map(lambda p: f"{p[0]}.{p[1]:02d}")
    assert family == "free"
    return _FREE


def map_keys(family: str) -> st.SearchStrategy[str]:
    if family == "data":
        return st.integers(0, 999).map(lambda i: f"u{i}")
    return st.integers(0, 500).map(_FORMAT_VALUES[family])


def child_templates(depth: int) -> st.SearchStrategy[Template]:
    return templates(depth - 1) if depth > 1 else scalar_templates()


@st.composite
def record_templates(draw: st.DrawFn, depth: int) -> Template:
    """One to three Variants with disjoint key sets, maybe a key shared by all, maybe a Discriminator."""
    n_variants = draw(st.sampled_from([1, 2, 2, 3]), label="variants")
    order = draw(st.permutations(FIELD_KEYS), label="key order")
    tagged = n_variants > 1 and draw(st.booleans(), label="tagged")
    tag_key, order = (order[0], order[1:]) if tagged else ("", order)
    shared = order[:1] if n_variants > 1 and draw(st.booleans(), label="a shared key") else []
    rest = order[len(shared) :]
    variants: list[tuple[tuple[str, str, Template], ...]] = []
    start = 0
    for _ in range(n_variants):
        size = draw(st.integers(1 if n_variants > 1 else 0, 4), label="own keys")
        own = rest[start : start + size]
        start += size
        fields = []
        for key in (*shared, *own):
            presence = draw(st.sampled_from(PRESENCE), label="presence")
            fields.append((key, presence, draw(child_templates(depth), label="field values")))
        variants.append(tuple(fields))
    tags = tuple((tag_key, f"v{i}") for i in range(n_variants)) if tagged else ()
    return Template("record", variants=tuple(variants), tags=tags)


@st.composite
def templates(draw: st.DrawFn, depth: int, mix: bool = True) -> Template:
    """A template whose values are at most ``depth`` containers deep, plus one for a free-form leaf."""
    kinds = KINDS if mix else tuple(kind for kind in KINDS if kind != "mix")
    kind = draw(st.sampled_from(kinds), label="kind") if depth > 0 else "scalar"
    if kind == "scalar":
        return draw(scalar_templates())
    if kind == "record":
        return draw(record_templates(depth))
    if kind == "map":
        family = draw(st.sampled_from(MAP_KEY_FAMILIES), label="map key family")
        return Template("map", family=family, children=(draw(child_templates(depth)),))
    if kind == "list":
        return Template("list", children=(draw(child_templates(depth)),))
    if kind == "tuple":
        positions = draw(st.lists(child_templates(depth), min_size=1, max_size=3), label="positions")
        return Template("tuple", children=tuple(positions))
    assert kind == "mix"
    options = draw(st.lists(templates(depth, mix=False), min_size=2, max_size=3), label="alternatives")
    return Template("mix", children=tuple(options))


@st.composite
def root_templates(draw: st.DrawFn) -> Template:
    """Mostly Records; also arrays of Records, Records among other types, and any template."""
    shape = draw(st.sampled_from(["record", "record", "record", "array", "mixed", "any"]), label="root shape")
    if shape == "any":
        return draw(templates(3))
    if shape == "array":
        return Template("list", children=(draw(record_templates(2)),))
    record = draw(record_templates(3), label="root record")
    if shape == "record":
        return record
    return Template("mix", children=(record, draw(scalar_templates()), draw(templates(2, mix=False))))


@st.composite
def instances(draw: st.DrawFn, template: Template) -> Any:
    kind = template.kind
    if kind == "scalar":
        return draw(scalar_values(template))
    if kind == "record":
        index = draw(st.integers(0, len(template.variants) - 1), label="variant")
        group = draw(st.booleans(), label="group present")
        out: dict[str, Any] = {}
        if template.tags:
            key, value = template.tags[index]
            out[key] = value
        for key, presence, child in template.variants[index]:
            present = presence == "always" or (group if presence == "group" else draw(st.booleans(), label="present"))
            if present:
                out[key] = draw(instances(child))
        return out
    if kind == "map":
        keys = dict.fromkeys(draw(st.lists(map_keys(template.family), min_size=1, max_size=5), label="map keys"))
        return {key: draw(instances(template.children[0])) for key in keys}
    if kind == "list":
        return draw(st.lists(instances(template.children[0]), max_size=3), label="elements")
    if kind == "tuple":
        full = draw(st.integers(0, 3), label="full tuple") != 0  # mostly full, so Tuples beat Lists
        length = len(template.children) if full else draw(st.integers(0, len(template.children)), label="length")
        return [draw(instances(child)) for child in template.children[:length]]
    assert kind == "mix"
    return draw(instances(template.children[draw(st.integers(0, len(template.children) - 1))]))


BAD_LINES = ("{bad", "[1,]", '{"a": NaN}', '"\\ud800"')
FILE_NAMES = ("a.jsonl", "B.ndjson", "é-docs.jsonl", "sub/inner.jsonl")


@dataclass(frozen=True)
class Collection:
    files: tuple[tuple[str, bytes], ...]  # (relative path, content)
    directory: bool  # whether INPUT is the directory, else its only file
    documents: int  # valid documents
    bad_lines: int

    def write(self, root: Path) -> Path:
        for name, data in self.files:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        return root if self.directory else root / self.files[0][0]


@st.composite
def collections(draw: st.DrawFn, min_documents: int) -> Collection:
    """At least ``min_documents`` valid documents in one ``.jsonl`` file or a directory of files."""
    root = draw(root_templates(), label="root template")
    n = draw(st.integers(min_documents, max(min_documents, 40)), label="documents")
    docs = [draw(instances(root), label="document") for _ in range(n)]
    escaped = draw(st.booleans(), label="ascii escapes")
    lines = [json.dumps(doc, ensure_ascii=escaped) for doc in docs]
    bad = draw(st.lists(st.tuples(st.integers(0, n), st.sampled_from(BAD_LINES)), max_size=2), label="bad lines")
    for position, text in sorted(bad, reverse=True):
        lines.insert(position, text)

    def jsonl(chunk: Sequence[str]) -> bytes:
        return "".join(line + "\n" for line in chunk).encode("utf-8")

    if draw(st.sampled_from(["directory", "one file"]), label="layout") == "one file":
        return Collection((("docs.jsonl", jsonl(lines)),), False, n, len(bad))
    files: list[tuple[str, bytes]] = []
    if draw(st.booleans(), label="a .json file"):  # also the first line; maybe unwrapped (input.unwrap_arrays)
        files.append(("0-single.json", json.dumps(docs[0], ensure_ascii=escaped, indent=1).encode("utf-8")))
    cuts = sorted(draw(st.lists(st.integers(0, len(lines)), max_size=len(FILE_NAMES) - 1), label="file cuts"))
    bounds = [0, *cuts, len(lines)]
    for i, (low, high) in enumerate(itertools.pairwise(bounds)):
        files.append((FILE_NAMES[i], jsonl(lines[low:high])))
    files.append(("notes.txt", b"not a document"))  # other extensions are ignored
    single = 1 if files[0][0] == "0-single.json" else 0
    return Collection(tuple(files), True, n + single, len(bad))


# ---------------------------------------------------------------------------
# Configurations
# ---------------------------------------------------------------------------

SAMPLE_PARAMETERS = tuple(sorted(name for name in RESOURCE_PARAMETERS if name.startswith("sample.")))
SCENARIOS = ("drawn", "priority", "dbscan first")
VARIED_SETTINGS = (*TECHNIQUE_SETTINGS, "seed", "input.unwrap_arrays", "output.inline_provenance")


@st.composite
def resource_values(draw: st.DrawFn, chunk_size: int) -> dict[str, int]:
    """Small passes, MinHash, EM, and sample settings; ``workers`` is set per run by a flag."""
    perm = draw(st.sampled_from([1, 8, 32]), label="minhash.perm")
    values = {
        WORKERS: 1,
        "ingest.chunk_size": chunk_size,
        "soundness.max_passes": draw(st.sampled_from([1, 2, 3]), label="soundness.max_passes"),
        "minhash.perm": perm,
        "lsh.bands": draw(st.integers(1, perm), label="lsh.bands"),
        "variants.max_components": draw(st.integers(1, 6), label="variants.max_components"),
        "variants.em_max_iter": draw(st.sampled_from([1, 5, 100]), label="variants.em_max_iter"),
    }
    for name in SAMPLE_PARAMETERS:  # mostly 1–3, so samples leave signatures out; sometimes the default
        small = draw(st.integers(0, 3), label=f"small {name}") != 0
        values[name] = draw(st.integers(1, 3), label=name) if small else RESOURCE_PARAMETERS[name].default_value()
    return values


@dataclass(frozen=True)
class Case:
    scenario: str
    flags: Mapping[str, Any]
    thresholds: Mapping[str, int | float]
    collection: Collection


@st.composite
def cases(draw: st.DrawFn) -> Case:
    chunk_size = draw(st.sampled_from([1, 1, 2, 3, 5]), label="ingest.chunk_size")
    # At least four chunks per stage, so the four-worker run hands work to four processes.
    collection = draw(collections(min_documents=4 * chunk_size), label="collection")
    scenario = draw(st.sampled_from(SCENARIOS), label="scenario")
    base: dict[str, Any] = dict(draw(resource_values(chunk_size), label="resources"))
    if scenario != "drawn":
        base[technique_setting(MDL)] = False
    if scenario == "dbscan first":
        base[technique_setting(MINHASH_LSH)] = False
        base[technique_setting(DBSCAN)] = True
    flags = draw(setting_flags(vary=VARIED_SETTINGS, base=base), label="flags")
    present = {tid: draw(st.integers(0, 3), label=f"thresholds of {tid}") != 0 for tid in THRESHOLD_TECHNIQUES}
    if scenario == "dbscan first":
        present[DBSCAN] = True
    thresholds = {
        name: draw(values, label=name)
        for name, values in THRESHOLD_VALUES.items()
        if all(present[tid] for tid in THRESHOLD_TECHNIQUES if name in REGISTRY.technique(tid).thresholds)
    }
    if scenario == "dbscan first":
        thresholds["dbscan.eps"] = draw(st.sampled_from([1e-9, 0.34, 0.5]), label="small dbscan.eps")
        thresholds["dbscan.min_samples"] = 1
    return Case(scenario, flags, thresholds, collection)


# ---------------------------------------------------------------------------
# Comparison
# ---------------------------------------------------------------------------


@dataclass
class Run:
    reply: Reply
    out: Path
    files: dict[str, bytes] = field(default_factory=dict)

    @property
    def label(self) -> str:
        spec = self.reply.spec
        start = "cold" if self.reply.cold else "warm"
        return f"{spec.label} (PYTHONHASHSEED={spec.hash_seed}, workers={spec.workers}, {start}, --out {spec.out!r})"


def byte_difference(a: bytes, b: bytes) -> str:
    """Where ``a`` and ``b`` first differ, with some context."""
    at = next((i for i, (x, y) in enumerate(zip(a, b, strict=False)) if x != y), min(len(a), len(b)))
    low = max(at - 80, 0)
    return f"first difference at byte {at} (lengths {len(a)}, {len(b)}):\n  {a[low : at + 80]!r}\n  {b[low : at + 80]!r}"


def json_difference(a: Any, b: Any, where: str = "") -> str | None:
    """The JSON location of the first difference between ``a`` and ``b`` (types compared strictly)."""
    if type(a) is not type(b):
        return f"{where or '/'}: {a!r} != {b!r}"
    if isinstance(a, dict):
        for key in sorted(a.keys() | b.keys()):
            if key not in a or key not in b:
                return f"{where}/{key}: present in only one"
            found = json_difference(a[key], b[key], f"{where}/{key}")
            if found is not None:
                return found
        return None
    if isinstance(a, list):
        if len(a) != len(b):
            return f"{where or '/'}: lengths {len(a)} != {len(b)}"
        for index, (x, y) in enumerate(zip(a, b, strict=True)):
            found = json_difference(x, y, f"{where}/{index}")
            if found is not None:
                return found
        return None
    return None if a == b else f"{where or '/'}: {a!r} != {b!r}"


def comparable_report(run: Run) -> tuple[bytes, dict[str, Any]]:
    """The canonical bytes of the report without what 27.12 exempts, and that JSON."""
    raw = json.loads(run.files[REPORT_FILE])
    report = InferenceReport.from_json(raw)  # also checks the canonical form
    assert report.ok, f"{run.label}: {raw['status']}"
    assert raw["resources"][WORKERS]["value"] == run.reply.spec.workers, run.label
    comparable = report.comparable_json()
    return encode_canonical(comparable), comparable


def config_without_workers(run: Run) -> dict[str, Any]:
    data = tomllib.loads(run.files[CONFIG_FILE].decode("utf-8"))
    assert data["run"].pop(WORKERS) == run.reply.spec.workers, run.label
    return data


def note(case: Case, runs: Sequence[Run]) -> None:
    event(f"scenario {case.scenario}")
    event(f"{'directory' if case.collection.directory else 'one file'}, {case.collection.bad_lines} bad lines")
    event(f"chunk size {case.flags['ingest.chunk_size']}")
    event(f"inline provenance {bool(case.flags.get('output.inline_provenance', False))}")
    event(f"cold runs: {', '.join(run.reply.spec.label for run in runs if run.reply.cold) or 'none'}")
    schema = runs[0].files[SCHEMA_FILE]
    for keyword in (b'"anyOf"', b'"oneOf"', b'"dependentRequired"', b'"format"', b'"enum"', b'"prefixItems"'):
        if keyword in schema:
            event(f"emitted {keyword.decode()}")


@given(case=cases())
def test_runs_are_byte_identical_across_worker_counts_hash_seeds_and_output_directories(
    pool: DriverPool, bundles: BundleDir, scratch: Callable[[], Path], case: Case
) -> None:
    root = scratch()
    try:
        inputs = case.collection.write(root / "collection")
        config = RunConfiguration.resolve({**case.flags, "model_bundle": str(bundles.path(case.thresholds))})
        config_file = root / "run.toml"
        config_file.write_text(config.to_toml(), encoding="utf-8")

        def argv(spec: DriverSpec) -> list[str]:
            out = root / spec.out
            return ["infer", str(inputs), "-c", str(config_file), "--set", f"{WORKERS}={spec.workers}", "--out", str(out)]

        replies = pool.run(argv)
        runs = [Run(reply, root / reply.spec.out) for reply in replies]
        for run in runs:
            reply = run.reply
            assert reply.error is None, f"{run.label}: cli.main raised\n{reply.error}"
            assert reply.status == 0, f"{run.label}: exit status {reply.status}\n{reply.stderr}"
            # Non-vacuity: a run with W > 1 workers had W worker processes; the one-worker driver has none.
            if reply.spec.workers > 1:
                assert reply.loky_workers >= reply.spec.workers, f"{run.label}: {reply.loky_workers} workers"
            else:
                assert reply.loky_workers == 0, f"{run.label}: {reply.loky_workers} workers"
            names = sorted(os.listdir(run.out))
            assert set(OUTPUTS) <= set(names), f"{run.label}: wrote {names}"
            run.files = {name: (run.out / name).read_bytes() for name in names}

        first, *others = runs
        for other in others:
            assert sorted(other.files) == sorted(first.files), f"{first.label} vs {other.label}"
        # Schemas (and versions, diffs, and the combined schema) and explanation files: the same bytes.
        for name in sorted(first.files):
            if name in (REPORT_FILE, CONFIG_FILE):
                continue
            for other in others:
                if other.files[name] != first.files[name]:
                    detail = byte_difference(first.files[name], other.files[name])
                    pytest.fail(f"{name} differs between {first.label} and {other.label}: {detail}")
        # Reports: equal once timings, timestamps, and the worker count are removed (27.12).
        expected, expected_json = comparable_report(first)
        for other in others:
            actual, actual_json = comparable_report(other)
            if actual != expected:
                detail = json_difference(expected_json, actual_json)
                pytest.fail(f"{REPORT_FILE} differs between {first.label} and {other.label} at {detail}")
        # The configuration files differ in the worker count only.
        reference = config_without_workers(first)
        for other in others:
            assert config_without_workers(other) == reference, f"{first.label} vs {other.label}"
        note(case, runs)
    finally:
        shutil.rmtree(root, ignore_errors=True)
