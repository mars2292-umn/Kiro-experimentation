"""Shared pytest configuration.

Hypothesis profiles:

- ``ci`` (default): 100 examples per property, derandomized so CI runs are reproducible.
- ``nightly``: 500 examples per property, randomized to explore new inputs each run.

Select a profile with the ``HYPOTHESIS_PROFILE`` environment variable or with
Hypothesis's ``--hypothesis-profile`` pytest option (the option wins).
"""

from __future__ import annotations

import os

from hypothesis import settings

# Pipeline-level properties run whole inferences per example, so wall-clock
# deadlines would only produce timing flakes.
settings.register_profile("ci", max_examples=100, derandomize=True, deadline=None, print_blob=True)
settings.register_profile("nightly", max_examples=500, deadline=None, print_blob=True)
settings.load_profile(os.environ.get("HYPOTHESIS_PROFILE", "ci"))


# ---------------------------------------------------------------------------
# Model_Bundle fixtures (task 23.5)
# ---------------------------------------------------------------------------

import hashlib  # noqa: E402
from pathlib import Path  # noqa: E402

import pytest  # noqa: E402

FIXTURE_CORPUS = Path(__file__).parent / "fixtures" / "corpus"
FIXTURE_MANIFEST = FIXTURE_CORPUS / "manifest.json"
FIXTURE_SPLIT = FIXTURE_CORPUS / "split.json"
# The training settings of tiny_bundle: small collections, shallow recursion, two collections per schema.
TINY_TRAINING = {
    "seed": 0,
    "resources": {"synth.collections_per_schema": 2, "synth.max_depth": 3},
    "generator": {"size_min": 8, "size_max": 24},
}


def _tiny_bundle_key() -> str:
    """A content hash of the fixture corpus and the training code, so a cached bundle is rebuilt when either changes."""
    digest = hashlib.sha256()
    source = Path(__file__).parent.parent / "src" / "jsinfer"
    files = sorted(FIXTURE_CORPUS.rglob("*.json")) + sorted(source.rglob("*.py"))
    for file in files:
        digest.update(str(file.relative_to(file.parents[-1])).encode())
        digest.update(file.read_bytes())
    digest.update(repr(sorted(TINY_TRAINING.items())).encode())
    return digest.hexdigest()[:16]


@pytest.fixture(scope="session")
def tiny_bundle(request: pytest.FixtureRequest, tmp_path_factory: pytest.TempPathFactory) -> str:
    """A Model_Bundle trained once per session on the fixture corpus, cached by content hash.

    The cache lives in pytest's cache directory; without the cache plugin
    (``-p no:cacheprovider``) the bundle is trained into a session temp directory.
    """
    from jsinfer.learn.bundle import MANIFEST_FILE
    from jsinfer.learn.trainer import Trainer, TrainingConfig

    cache = getattr(request.config, "cache", None)
    name = f"jsinfer-tiny-bundle-{_tiny_bundle_key()}"
    cache_dir = cache.mkdir(name) if cache is not None else tmp_path_factory.mktemp(name)
    bundle = cache_dir / "bundle"
    if not (bundle / MANIFEST_FILE).is_file():
        config = TrainingConfig(str(FIXTURE_MANIFEST), str(FIXTURE_SPLIT), **TINY_TRAINING)  # type: ignore[arg-type]
        Trainer().run(config, bundle)
    return str(bundle)


@pytest.fixture(scope="session")
def empty_bundle() -> str:
    """The shipped bundle with no artifacts: every Technique that needs one falls back."""
    from jsinfer.config.run_config import DEFAULT_MODEL_BUNDLE

    return DEFAULT_MODEL_BUNDLE
